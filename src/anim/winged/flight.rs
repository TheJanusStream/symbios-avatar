//! Flight (W5, #424): a winged body held in the air, beating, gliding,
//! diving, flaring and coming down onto its feet.
//!
//! **A sustained airborne source, driven by its host.** A leap's flight is
//! ballistic and over in a second ([`crate::anim::leap`]); a flier stays up for as
//! long as its host flies it, so what it is doing in the air is the host's
//! to say, frame by frame: a [`FlightMode`], the air's velocity past it, and
//! where its wings are in their beat if the host keeps that clock - the
//! [`Flight`] on [`crate::anim::driver::Inputs::flight`]. The modes are the
//! arcade model's that the consuming application flies (research-11 8.1, 8.5):
//! a takeoff, flapping, gliding (and soaring, a glide on a thermal), a dive,
//! a flare and a landing. On the ground there is no flight, and the body
//! walks and stands as it always did.
//!
//! **Every number a pose reads is eased, so no change of mode pops.** A mode
//! names targets - how far the wings are spread, how wide they beat, how far
//! the body is levelled out of its ground posture, how far the legs are
//! tucked, the tail fanned, the flare's depth - and each value moves toward
//! its target through a first-order lag of its own, so a switch between any
//! two modes moves every joint by at most a lag's share of the distance in a
//! frame. The beat runs on a clock of its own ([`Flown::phase`]) that never
//! jumps: a host's phase is followed, not copied - the clock running between
//! half and twice its own rate to catch it, never backward - so a beat a host
//! starts early reads as a quicker stroke rather than a snap. Only two things are
//! scripted against time, and both start where the body already is: a
//! takeoff's push off the ground, and a landing's absorb.
//!
//! **What scales and what blends** (research-11 8.2, 8.3). The beat's rate
//! falls with the body's mass as `m^-0.28`, its trunk cubed, so as its length
//! to the `-0.84` - the same law the consuming application's own wingbeat
//! follows. The rate, the beat's width, its downstroke share, its upstroke's
//! fold (a bird's flexed wing, a bat's strong flex), the glide's dihedral and
//! the body's motion through a beat blend across a cross by its families'
//! shares ([`Kinship`]); whether it beats at all does not - a glider glides,
//! and so does a cross a glider leads ([`Kinship::flaps`]).
//!
//! **The host carries the attitude.** In the air a host writes the body's
//! heading, pitch and bank itself, so the pose here is the body as it rides
//! that attitude: its trunk levelled along its own `+Z` (a perched bird
//! stands nearly upright and flies level), its head held as it was, and only
//! the beat's small pitch and a flare's last lift of the nose added. A body
//! that carries itself ([`Carriage::Own`]) hovers on the spot instead, risen
//! by its takeoff to [`HOVER`] of its height - or as high as its wings' beat
//! needs to clear the ground - and brought down by its landing. Near the
//! ground no wing beats into it: its stroke is raised as far as clears it.
//!
//! # Handing on
//!
//! **A host need guess no durations**: [`Flown`] says when each scripted
//! part is done ([`crate::anim::driver::Driver::flight`]).
//!
//! - **A takeoff into flapping** two beats after [`Flown::lifted`]: once the
//!   feet are off, when [`Flown::phase`] has grown by two from where it stood
//!   that frame - `(phase - at_lift).rem_euclid(64.0) >= 2.0`, since the
//!   clock wraps every 64 beats - two of the takeoff's big beats (research-11
//!   3.1: two or three). Counted in beats, it reads alike at every size: a bird at size
//!   2.4 (a trunk of about 1.2 m) lifts off at about 0.45 s and beats a
//!   takeoff's beat about once a second, a songbird at 0.1 ten times in
//!   0.6 s, so no one duration suits both.
//!   A host may hand on sooner, to a glide or anything else; the values ease.
//! - **A landing back to the ground** - no flight - once [`Flown::settled`]:
//!   down, absorbed, folded and, carrying itself, at the ground. Handed back
//!   sooner, the ground posture blends in from wherever the landing was,
//!   mid-absorb and all.
//! - **A landing asked of a body on the ground** is a landing from no height:
//!   its feet put down and its wings folded, with no absorb.

use glam::{Quat, Vec3};

use crate::det::{self, DetMath, Rot};
use crate::plan::{Limb, Zone};
use crate::rig::Rig;
use crate::wing::{Kinship, Stroke};

use crate::anim::driver::Carriage;
use crate::anim::ground::{FootingConfig, Ground, plant_feet_of, solve_contact_toward};
use crate::anim::leap::LEG_STIFFNESS;
use crate::anim::pose::Pose;
use crate::anim::speed::GRAVITY;

/// What a flying body is doing, as its host flies it (#424).
///
/// The consuming application's own modes, which it sends to its peers
/// (its `FlightMode`): a body on its feet has no flight at all, so its
/// `Ground` is [`None`] on [`crate::anim::driver::Inputs::flight`]. [`Self::Soar`]
/// is a glide on rising air, which that application does not yet fly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlightMode {
    /// Off the ground: a crouch and a push from the legs, and two or three
    /// big beats - a bat's from its wrists as well, a glider's a spread
    /// leap.
    Takeoff,
    /// Beating its wings.
    Flap,
    /// Wings spread and still.
    Glide,
    /// A glide on rising air: the wings spread with a little more dihedral,
    /// the tail fanned to circle.
    Soar,
    /// Wings folded, falling nose-first.
    Dive,
    /// Nose up over what it is about to land on: wings forward and braking,
    /// the tail fanned and down, the legs reaching.
    Flare,
    /// Down, and settling onto its feet: the legs absorb and the wings fold.
    Land,
}

/// What a host knows about a body in the air, for one frame (#424): the
/// whole of what the flight is driven by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// What the body is doing.
    pub mode: FlightMode,
    /// The air's velocity past the body, in world metres a second: how fast
    /// it is flying, which a slow beat bobs at and a fast one does not.
    /// [`None`] reads [`crate::anim::driver::Inputs::velocity`], which is the same
    /// thing in still air.
    pub air: Option<Vec3>,
    /// Where the wings are in their beat, in beats - `0` the top of a
    /// downstroke, any whole number the same - when the host keeps the
    /// beat's clock, as a host that pushes the body at mid-downstroke does.
    /// Followed rather than copied ([`Flown::phase`]). [`None`] lets the
    /// engine keep its own clock at the body's own rate.
    pub phase: Option<f32>,
    /// How long until the body reaches what it is landing on, in seconds:
    /// the time to contact a pigeon flares by (research-11 6.1, 8.1).
    /// Under [`FLARE_TAU`] the body flares whatever the mode says, and
    /// deeper the nearer it is; [`None`] where there is nothing ahead.
    pub contact: Option<f32>,
}

impl Flight {
    /// A body doing `mode`, with nothing else said: its own beat, the
    /// driver's velocity for the air, and nothing ahead.
    #[must_use]
    pub fn new(mode: FlightMode) -> Self {
        Self {
            mode,
            air: None,
            phase: None,
            contact: None,
        }
    }
}

/// What a flying body was posed doing on the last frame (#424):
/// [`crate::anim::driver::Driver::flight`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Flown {
    /// The mode it was posed in: the host's, but a glider asked to flap
    /// glides, and anything but a landing flares under [`FLARE_TAU`].
    pub mode: FlightMode,
    /// The beat's clock, in beats: `0` the top of a downstroke, and the
    /// downstroke the first [`Flown::downstroke`] of each beat. It never
    /// runs backward, and catches up with a host's [`Flight::phase`] running
    /// between half and twice its own rate. **It wraps**: every 64 beats it
    /// takes 64 off, so a long flight keeps its precision - the beat drawn
    /// is the same either side, but a host counting beats reads the gap as
    /// `(now - then).rem_euclid(64.0)`, never `now - then`.
    pub phase: f32,
    /// How many beats a second it beats at, the mode's own: a takeoff's
    /// fifth faster than a cruise's.
    pub rate: f32,
    /// The share of a beat its downstroke takes.
    pub downstroke: f32,
    /// How far the wings were spread, `0` folded to `1`, as drawn: less than
    /// the mode's own on an upstroke, which folds them.
    pub extension: f32,
    /// How wide the beat was, in radians either side of its middle.
    pub amplitude: f32,
    /// How deep the flare, `0` none to `1` full.
    pub flare: f32,
    /// How far the trunk was levelled out of its ground posture, `0` as it
    /// stands to `1` flying level.
    pub level: f32,
    /// Seconds in this mode.
    pub seconds: f32,
    /// Whether the body has left the ground: a takeoff's push is over and
    /// its feet are off (always, once in any other mode). A host hands a
    /// takeoff on into flapping from here - see the module's "Handing on".
    pub lifted: bool,
    /// Whether a landing has settled: down, its absorb played out, its wings
    /// folded and, on a body that carries itself, its hover height at the
    /// ground. A host hands a landing back - no flight - from here.
    pub settled: bool,
    /// How high a body that carries itself hovers over where it stood, in
    /// metres; nothing on a carried body, whose host carries its height.
    pub height: f32,
}

/// Under this time to contact a body flares, in seconds (research-11 8.1:
/// 0.3 to 0.5, a bird's two or three beats and a bat's four or five), and
/// fully by [`FLARE_FULL`]. The consuming application flares at 0.45 too.
pub const FLARE_TAU: f32 = 0.45;

/// The time to contact a flare is full by, in seconds.
pub const FLARE_FULL: f32 = 0.1;

/// How much of a bird's lift-off speed its legs give (research-11 3.1: 80 to
/// 90% in a starling and a quail, 94 to 95% in a zebra finch and a dove):
/// the wings begin their first downstroke a little before the feet leave and
/// give the rest. Provenance: **the literature**, the middle of the range.
pub const LEG_SHARE: f32 = 0.88;

/// How far into its push a takeoff's first downstroke begins, as a share of
/// the push (research-11 3.1: "the first downstroke starts near lift-off").
/// Provenance: **chosen**, so the wings give the rest of [`LEG_SHARE`] over
/// the push's last fifth.
const DOWNSTROKE_FROM: f32 = 0.8;

/// A takeoff's lift-off speed at the reference body, in metres a second
/// (research-11 8.5: a bird's leap about 2 to 3 m/s; the vampire's 2.4),
/// scaled with the trunk as a cruise is, to the 0.4.
const LIFT_OFF: f32 = 2.0;

/// How deep a takeoff's crouch may go, as a share of the leg's reach: a leg
/// folded further is a body falling over rather than crouching, as a leap's
/// [`crate::anim::leap`] squash is held.
const CROUCH_MOST: f32 = 0.4;

/// The least a takeoff crouches, as a share of its leg, its push lasts and,
/// carrying itself, its crouch lasts, in seconds: so each reads (#424, the
/// look review), and the crouch outlasts the blend out of the ground.
const VISIBLE_CROUCH: f32 = 0.2;
/// See [`VISIBLE_CROUCH`].
const MIN_PUSH: f32 = 0.1;
/// See [`VISIBLE_CROUCH`].
const MIN_CROUCH: f32 = 0.18;

/// How far a tilted foot's sole is reckoned to reach past its joint, as a
/// share of how far under the joint it stands at rest, by the sine of the
/// tilt: its toes going down as it pitches.
const SOLE_TILT: f32 = 1.5;

/// How fast the root's hold on its feet comes and goes, in seconds.
const EASE_GROUNDING: f32 = 0.04;

/// How near the ground, as a share of the leg, a hind foot starts to flatten
/// to meet it.
const NEAR_GROUND: f32 = 0.5;

/// How much of a takeoff's push a bat or a glider waits before its wings
/// open, and how much of the belly's height over the ground its crouch may
/// take.
const LATE_OPEN: f32 = 0.6;
/// See [`LATE_OPEN`].
const CROUCH_BELLY: f32 = 1.2;

/// How far a raised body - a bird - levels into its push: it leans forward
/// over its feet as it springs, its half-open wings out of their droop.
const PUSH_LEAN: f32 = 0.35;

/// How far over its stroke's lowest reach a body that carries itself
/// hovers: a share of the reach, and of its height.
const CLEAR_REACH: f32 = 0.15;
/// See [`CLEAR_REACH`].
const CLEAR: f32 = 0.08;

/// How far over the ground a wing's joints are kept, as a share of the
/// body's height, and how many times a frame is posed again to raise them.
const WING_CLEAR: f32 = 0.06;
/// See [`WING_CLEAR`].
const RAISE_PASSES: usize = 8;
/// How far a wing is spread before it is raised off the ground in full:
/// folded, it is the ground posture's.
const RAISE_OPEN: f32 = 0.5;
/// How far past its joint a feather is reckoned to reach, in humeri.
const FEATHER_OVERHANG: f32 = 2.6;

/// How much of what a leg can still fold a takeoff's crouch may take: half,
/// so no knee comes near folding shut and turning over its pole.
const CROUCH_FOLD: f32 = 0.5;

/// How long a takeoff's crouch takes before its push, as a share of the
/// push, on a body that carries itself: a host that carries the body has
/// already launched it, so its takeoff starts crouched.
const ANTICIPATION: f32 = 1.5;

/// How high a body that carries itself hovers, as a share of its height, at
/// the least: higher where its beat needs it to clear the ground.
pub const HOVER: f32 = 0.8;

/// How far under the ground a body that carries itself aims its landing, as
/// a share of its height: so it arrives in a fraction of a second rather
/// than closing on the ground for ever, and is stopped there.
const LAND_UNDER: f32 = 0.05;

/// How near the ground, as a share of its height, a body that carries itself
/// has touched down.
const TOUCHDOWN: f32 = 0.002;

/// How far a landing's wings may still be spread for it to have settled.
const SETTLED_FOLD: f32 = 0.02;

/// How far a landing's legs give, as a share of their reach.
const ABSORB: f32 = 0.25;

/// How fast a takeoff beats against a cruise (research-11 3.1: about 20%
/// faster), and how much wider (4.2: 140 to 170 degrees against 90 to 130).
const TAKEOFF_RATE: f32 = 1.2;
/// See [`TAKEOFF_RATE`].
const TAKEOFF_WIDTH: f32 = 1.3;
/// How much wider a flare's braking beats are than a cruise's (4.2:
/// amplitude highest in takeoff and landing).
const FLARE_WIDTH: f32 = 1.25;

/// The cruise beat's rate at the reference body, beats a second: a bird's
/// and a bat's (research-11 4.1: birds beat faster than bats of a mass). The
/// bird's is the consuming application's stand-in's, 3 a second, about an
/// eagle owl's at a cat's size; a glider does not beat.
const RATE: (f32, f32) = (3.0, 2.6);

/// The trunk lengths the rates are for, in metres: each family's default
/// body, measured (shoulder to hip).
const REFERENCE_TRUNK: (f32, f32, f32) = (0.311, 0.269, 0.255);

/// How the beat's rate falls with the trunk's length: mass to the -0.28
/// (research-11 4.1, 8.2: -0.26 to -0.30 in both families), mass as the
/// trunk cubed.
pub const RATE_EXPONENT: f32 = -0.84;

/// A cruise speed at the reference body, metres a second, and how it
/// scales with the trunk (research-11 8.2: mass to the 0.13, so length to
/// about the 0.4): a slow beat bobs the body, one at cruise does not.
const CRUISE: f32 = 6.5;
/// See [`CRUISE`].
const CRUISE_EXPONENT: f32 = 0.4;

/// The cruise beat's half-width, in radians: a bird's and a bat's
/// (research-11 4.2: a bird's 90 to 130 degrees in all; a bat's 20 to 50
/// either side in the model sweep, wider read on an avatar).
const WIDTH: (f32, f32) = (0.80, 0.62);

/// How far an upstroke folds the wing at its middle, as extension taken off
/// (research-11 4.2: a bird's flexed wing at cruise, a bat's strong flex).
const FOLD: (f32, f32) = (0.25, 0.38);

/// The share of a beat its downstroke takes (research-11 4.2: a bird's 0.5
/// to 0.6, a bat's falling with its rate).
const DOWNSTROKE: (f32, f32) = (0.56, 0.5);

/// How far a beat sweeps the wing forward on its downstroke and back on its
/// upstroke, in radians: the tip's forward-tilted loop.
const SWEEP: (f32, f32) = (0.22, 0.28);

/// A glide's dihedral, in radians: a bird's slight one, a bat's flatter, a
/// glider's flat X (research-11 5, 8.1).
const DIHEDRAL: (f32, f32, f32) = (0.10, 0.04, 0.0);

/// How far a slow beat heaves the body, as a share of its trunk, and pitches
/// it, in radians (research-11 4.3: up on the downstroke and nose up; a
/// bat's heavy wings rock it more).
const HEAVE: (f32, f32) = (0.06, 0.07);
/// See [`HEAVE`].
const BEAT_PITCH: (f32, f32) = (0.05, 0.10);

/// How far a dive spreads the wings: half folded and swept back, a stoop -
/// a bird's, a bat's and a glider's. A bat's and a glider's halfway pose is
/// partway to the crawl's fold, which reads crumpled further in.
const DIVE_EXTENSION: (f32, f32, f32) = (0.3, 0.5, 0.4);

/// How far the trunk is levelled back toward its ground posture in a full
/// flare - a bird's nose comes up to near vertical (research-11 6.1) over
/// the host's own pitch - and how far a body with no raised posture lifts its
/// nose instead, in radians.
const FLARE_LEVEL: f32 = 0.45;
/// See [`FLARE_LEVEL`].
const FLARE_NOSE: f32 = 0.25;

/// A full flare's wings swung forward and raised, in radians, and its tail
/// depressed.
const FLARE_REACH: f32 = 0.35;
/// See [`FLARE_REACH`].
const FLARE_RAISE: f32 = 0.15;
/// See [`FLARE_REACH`].
const FLARE_TAIL: f32 = 0.35;

/// How far a full flare swings a bird's legs forward, in radians.
const FLARE_LEGS: f32 = 0.7;

/// How far a tucked leg's foot is drawn toward its hip, as a share of the
/// leg.
const TUCK: f32 = 0.45;

/// The tail as a rudder: radians of yaw per radian a second of turn, and the
/// most.
const RUDDER: (f32, f32) = (0.15, 0.3);

/// How far the wings differ in a turn: radians of elevation per radian a
/// second of turn (the outer wing raised), and the most.
const BANKED: (f32, f32) = (0.04, 0.1);

/// The time constants each eased value moves through, in seconds.
const EASE_EXTENSION: f32 = 0.08;
const EASE_WIDTH: f32 = 0.08;
const EASE_LEVEL: f32 = 0.15;
const EASE_LIMBS: f32 = 0.1;
const EASE_TAIL: f32 = 0.12;
const EASE_FLARE: f32 = 0.04;
/// The most a wing's extension changes in a second: opened or folded whole
/// in no less than a sixth of a second, ten frames at 60 a second.
const EXTENSION_RATE: f32 = 6.0;
/// The most a landing body folds its wings in a second: the whole range in
/// no less than half a second, the legs of a body standing on its wings - a
/// bat, a glider - coming down onto the ground over the fifth of a second
/// the middle of it takes.
const LAND_FOLD_RATE: f32 = 2.0;
/// How much faster the wings open and widen through a takeoff's push.
const EASE_PUSH: f32 = 0.35;
/// How much slower and faster than its own rate the beat's clock may run
/// while it follows a host's phase: never backward, and at most twice.
const FOLLOW_SLOWER: f32 = 0.5;
/// See [`FOLLOW_SLOWER`].
const FOLLOW_FASTER: f32 = 1.0;
/// How fast a host's phase is followed, in seconds.
const FOLLOW: f32 = 0.1;
/// How long feet take to leave the ground as a push ends, and to come onto
/// it in a landing, in seconds.
const PLANT: f32 = 0.08;
/// See [`PLANT`].
const PLANT_DOWN: f32 = 0.2;
/// A hovering body's height follows its target as a critically damped spring
/// of this frequency, in Hertz.
const HOVER_HZ: f32 = 0.8;

/// What a flier's body reads, from its rig: its families, its size, and the
/// numbers they make.
#[derive(Clone, Copy, Debug)]
struct Build {
    flaps: bool,
    /// Whether a bird leads the body, whose wings open as its legs push; a
    /// bat's and a glider's at the push's end.
    early: bool,
    rate: f32,
    width: f32,
    fold: f32,
    downstroke: f32,
    sweep: f32,
    dihedral: f32,
    dive: f32,
    heave: f32,
    pitch: f32,
    cruise: f32,
    /// How far the trunk is raised at rest, in radians: what flying levels.
    raised: f32,
    pivot: Vec3,
    leg: f32,
    push: f32,
    crouch: f32,
    /// How far the legs it stands on may fold, as a crouch or a landing
    /// takes them: see [`CROUCH_FOLD`].
    folds: f32,
    height: f32,
    /// How high its belly stands over the ground.
    belly: f32,
    /// How high a body that carries itself hovers over the ground.
    hover: f32,
    /// The speed its legs give it as its feet leave, from its crouch over
    /// its push.
    legs: f32,
}

impl Build {
    fn of(rig: &Rig) -> Option<Self> {
        let wings = rig.wings.as_ref()?;
        let flier: Kinship = wings.kinship(rig);
        let pelvis = *rig.in_zone(Zone::Pelvis).first()?;
        let withers = *rig.in_zone(Zone::Chest).last()?;
        let (hip, shoulder) = (rig.joints[pelvis].position, rig.joints[withers].position);
        let run = shoulder - hip;
        let trunk = run.length().max(1e-3);
        let raised = run.y.det_atan2(run.z.abs().max(1e-6));
        let reference = flier.mix(REFERENCE_TRUNK.0, REFERENCE_TRUNK.1, REFERENCE_TRUNK.2);
        let size = trunk / reference;
        let flaps = flier.flaps();
        let beats = |bird: f32, bat: f32| {
            // A glider's share beats as a bat's: it only ever weighs a cross
            // a bird or a bat leads.
            flier.mix(bird, bat, bat)
        };
        let leg = crate::anim::speed::leg_of(rig).max(1e-3);
        let lift_off = LIFT_OFF * size.det_powf(CRUISE_EXPONENT);
        // The legs' share of the lift-off speed, from a spring released
        // over a quarter turn: the push is the leg's half period as a
        // leap's is, and its depth what that speed asks, held short of
        // folding the leg.
        let legs = LEG_SHARE * lift_off;
        let half_period = std::f32::consts::PI * (leg / (LEG_STIFFNESS * GRAVITY)).sqrt();
        // And held to what every leg it stands on folds without its knee
        // passing its pole: a glider stands crouched already, and pushed
        // down a leg's two-fifths its hind knees flipped over its hips.
        let folds = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_chain(limb))
            .map(|[root, mid, tip]| {
                let [root, mid, tip] = [root, mid, tip].map(|joint| rig.joints[joint].position);
                let (upper, lower) = (root.distance(mid), mid.distance(tip));
                CROUCH_FOLD * (root.distance(tip) - (upper - lower).abs()).max(0.0)
            })
            .fold(f32::MAX, f32::min);
        // **And deep and long enough to be seen** (#424, the look review):
        // at the leg's own spring a glider's crouch was a centimetre and its
        // push a twentieth of a second, under the blend out of the ground,
        // and it read as standing until it sprang.
        // Nor so deep the belly meets the ground: a bat crawls with its
        // belly a tenth of its trunk off it.
        // Nor so deep a folded wing that is no leg meets it, a cross's whose
        // feathers hang to the ground.
        let legs_of = rig.ground_contacts();
        let belly = (0..rig.len())
            .filter(|&joint| match rig.joints[joint].zone {
                Zone::Pelvis | Zone::Abdomen | Zone::Chest => true,
                Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) => {
                    limb.is_fore() && !legs_of.contains(&limb)
                }
                _ => false,
            })
            .map(|joint| rig.joints[joint].position.y - rig.joints[joint].radius)
            .fold(f32::MAX, f32::min)
            // And its folded wing's drawn feathers, which on a bird-led cross
            // with a bat's long arm hang to the ground already.
            .min(match wings.low {
                Some(low) if !legs_of.iter().any(|limb| limb.is_fore()) => low,
                _ => f32::MAX,
            });
        let crouch = (2.0 * legs * half_period / std::f32::consts::PI)
            .max(VISIBLE_CROUCH * leg)
            .min(CROUCH_MOST * leg)
            .min(folds.max(1e-3))
            .min(CROUCH_BELLY * belly.max(0.0))
            .max(1e-3);
        let push = (std::f32::consts::FRAC_PI_2 * crouch / legs.max(1e-3)).max(MIN_PUSH);
        // How far a wing reaches from its shoulder spread, which its stroke
        // must clear the ground by.
        let reach = {
            let mut pose = Pose::rest(rig);
            rig.spread_wings(&mut pose, 1.0, 1.0);
            let posed = pose.forward(rig);
            wings
                .sides
                .iter()
                .filter_map(|side| {
                    let upper = rig.in_zone(Zone::UpperLimb(side.limb));
                    let shoulder = *upper.get(upper.len().checked_sub(2)?)?;
                    let at = posed.positions[shoulder];
                    side.drives
                        .iter()
                        .filter(|drive| !matches!(
                            rig.joints[drive.joint].zone,
                            Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb)
                                if !limb.is_fore()
                        ))
                        .map(|drive| posed.positions[drive.joint].distance(at))
                        .reduce(f32::max)
                })
                .fold(0.0f32, f32::max)
        };
        let width = if flaps { beats(WIDTH.0, WIDTH.1) } else { 0.0 };
        // **A hover high enough that the beat's lowest reach clears the
        // ground** (#424, the look review: a bat hovering at 0.29 m beat its
        // wings 0.2 m into it, and 0.45 m in its takeoff's first beats).
        let deepest = (width * TAKEOFF_WIDTH - 0.08).clamp(0.0, std::f32::consts::FRAC_PI_2);
        let height = rig.extent();
        let hover =
            (HOVER * height).max(reach * deepest.det_sin() * (1.0 + CLEAR_REACH) + CLEAR * height);
        Some(Self {
            flaps,
            early: flier.leading == crate::plan::WingedFamily::Bird,
            rate: beats(RATE.0, RATE.1) * size.det_powf(RATE_EXPONENT),
            width,
            fold: beats(FOLD.0, FOLD.1),
            downstroke: beats(DOWNSTROKE.0, DOWNSTROKE.1),
            sweep: beats(SWEEP.0, SWEEP.1),
            dihedral: flier.mix(DIHEDRAL.0, DIHEDRAL.1, DIHEDRAL.2),
            dive: flier.mix(DIVE_EXTENSION.0, DIVE_EXTENSION.1, DIVE_EXTENSION.2),
            heave: beats(HEAVE.0, HEAVE.1) * trunk,
            pitch: beats(BEAT_PITCH.0, BEAT_PITCH.1),
            cruise: CRUISE * size.det_powf(CRUISE_EXPONENT),
            raised,
            pivot: (hip + shoulder) * 0.5,
            leg,
            push,
            crouch,
            folds,
            height,
            hover,
            belly: belly.max(0.0),
            legs: std::f32::consts::FRAC_PI_2 * crouch / push,
        })
    }

    /// How long a landing's absorb takes, in seconds: the leg's half period,
    /// as a leap's landing.
    fn absorb(&self) -> f32 {
        std::f32::consts::PI * (self.leg / (LEG_STIFFNESS * GRAVITY)).sqrt()
    }

    /// How long a takeoff crouches before it pushes: on a body that carries
    /// itself; a host that carries one has already launched it.
    fn anticipation(&self, carriage: Carriage) -> f32 {
        match carriage {
            Carriage::Own => (ANTICIPATION * self.push).max(MIN_CROUCH),
            Carriage::Chassis => 0.0,
        }
    }

    /// When a takeoff's first downstroke begins, seconds into it.
    fn first(&self, anticipation: f32) -> f32 {
        anticipation + DOWNSTROKE_FROM * self.push
    }

    /// When the upstroke before a takeoff's first downstroke begins, seconds
    /// into it, if the takeoff holds its beat until then: the wings held,
    /// and rising only on that upstroke.
    fn rising(&self, anticipation: f32) -> Option<f32> {
        let up = (1.0 - self.downstroke.clamp(0.2, 0.8)) / (self.rate * TAKEOFF_RATE).max(1e-3);
        let from = self.first(anticipation) - up;
        (from > 0.0).then_some(from)
    }

    /// Where a beat at `phase` stands: `0` the top of the downstroke, `1`
    /// its bottom, `2` the top again.
    fn stroke_at(&self, phase: f32) -> f32 {
        let at = phase.rem_euclid(1.0);
        let down = self.downstroke.clamp(0.2, 0.8);
        if at < down {
            at / down
        } else {
            1.0 + (at - down) / (1.0 - down)
        }
    }
}

/// What a mode asks of each eased value.
#[derive(Clone, Copy, Debug)]
struct Targets {
    extension: f32,
    width: f32,
    raise: f32,
    sweep: f32,
    tail: f32,
    tuck: f32,
    level: f32,
}

/// A flying body's clocks and eased values (#424), kept by the driver for as
/// long as the body flies.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Aloft {
    /// The mode it is posed in.
    mode: FlightMode,
    /// Seconds in it.
    in_mode: f32,
    /// Seconds since a takeoff from the ground began, while its push lasts.
    pushing: Option<f32>,
    /// Seconds since the landing began.
    landing: Option<f32>,
    /// The beat's clock, in beats.
    phase: f32,
    /// How fast the clock runs against its own rate, following a host's.
    pace: f32,
    /// Whether the clock was held at the bottom of a takeoff's first
    /// upstroke, which then rises from the wing's rest rather than from a
    /// stroke's bottom: until the clock passes nought, the first top.
    lifting: bool,
    /// The rate it ran at last frame.
    rate: f32,
    extension: f32,
    width: f32,
    /// How fast the beat is widening, a second.
    widening: f32,
    raise: f32,
    sweep: f32,
    tail: f32,
    tuck: f32,
    level: f32,
    /// How fast the trunk is levelling, a second.
    levelling: f32,
    flare: f32,
    /// The rudder's yaw and the wings' difference in a turn.
    rudder: f32,
    banked: f32,
    /// How slow the body flies against its cruise, `0..1`: what the beat
    /// bobs it by.
    slow: f32,
    /// How far its root is held up by feet that would sink, `0..1`: through
    /// a takeoff and a landing, eased in and out, and not in the air between,
    /// where a host may carry a body as low as it likes.
    grounding: f32,
    /// How far its feet are held to the ground, `0..1`.
    planted: f32,
    /// A body that carries itself: its height over where it stood, and how
    /// fast it is climbing.
    height: f32,
    climb: f32,
    /// What the takeoff carries the root by, over and above the hover:
    /// the crouch and the push.
    offset: f32,
    /// Where the root was carried when the landing began, which its absorb
    /// eases out from.
    landed_from: f32,
    /// How deep this landing's absorb goes: none for one asked on the
    /// ground.
    depth: f32,
    /// Whether this landing's absorb has played out.
    absorbed: bool,
    /// Whether the flight began on the ground in its landing, with nothing
    /// to come down from.
    grounded: bool,
    /// Whether the body carries itself, as the last frame was stepped.
    carried: bool,
}

/// Moves `value` toward `target` through a first-order lag of `tau`
/// seconds.
fn ease(value: f32, target: f32, tau: f32, delta: f32) -> f32 {
    if tau <= 0.0 {
        return target;
    }
    value + (target - value) * (1.0 - (-delta / tau).det_exp())
}

/// A smooth step from `0` at `0` to `1` at `1`.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Aloft {
    /// A body leaving the ground, or opening its wings off it, in `flight`:
    /// everything where the ground posture has it - the wings folded, the
    /// trunk as it stands - so the first frame is the body as it was.
    pub(crate) fn enter(rig: &Rig, flight: &Flight, carriage: Carriage) -> Option<Self> {
        let build = Build::of(rig)?;
        let takeoff = flight.mode == FlightMode::Takeoff;
        let anticipation = build.anticipation(carriage);
        // The beat held at the bottom of an upstroke until the upstroke
        // that tops at the first downstroke: the wings rise while the legs
        // extend, and that downstroke begins a little before the feet leave.
        let phase = match build.rising(anticipation) {
            _ if !takeoff => 0.0,
            Some(_) => build.downstroke.clamp(0.2, 0.8) - 1.0,
            None => -build.first(anticipation) * build.rate * TAKEOFF_RATE,
        };
        Some(Self {
            mode: flight.mode,
            in_mode: 0.0,
            pushing: takeoff.then_some(0.0),
            // A landing asked on the ground is down already: no absorb.
            landing: (flight.mode == FlightMode::Land).then_some(0.0),
            phase,
            pace: 1.0,
            lifting: takeoff && build.rising(anticipation).is_some(),
            rate: build.rate,
            extension: 0.0,
            width: 0.0,
            widening: 0.0,
            raise: 0.0,
            sweep: 0.0,
            tail: 0.0,
            tuck: 0.0,
            level: 0.0,
            levelling: 0.0,
            flare: 0.0,
            rudder: 0.0,
            banked: 0.0,
            slow: 1.0,
            planted: if takeoff { 1.0 } else { 0.0 },
            grounding: if matches!(flight.mode, FlightMode::Takeoff | FlightMode::Land) {
                1.0
            } else {
                0.0
            },
            height: 0.0,
            climb: 0.0,
            offset: if takeoff && anticipation == 0.0 {
                -build.crouch
            } else {
                0.0
            },
            landed_from: 0.0,
            depth: 0.0,
            absorbed: false,
            grounded: flight.mode == FlightMode::Land,
            carried: carriage == Carriage::Own,
        })
    }

    /// What it was posed doing.
    pub(crate) fn flown(&self, rig: &Rig) -> Flown {
        let downstroke = Build::of(rig).map_or(0.5, |build| build.downstroke);
        Flown {
            mode: self.mode,
            phase: self.phase,
            rate: self.rate,
            downstroke,
            extension: self.drawn_extension(rig),
            amplitude: self.width,
            flare: self.flare,
            level: self.level,
            seconds: self.in_mode,
            lifted: self.pushing.is_none(),
            settled: self.mode == FlightMode::Land
                && self.absorbed
                && self.drawn_extension(rig) <= SETTLED_FOLD
                && (!self.carried || self.height <= TOUCHDOWN * self.reach(rig)),
            height: if self.carried { self.height } else { 0.0 },
        }
    }

    /// The body's height, which its hover is reckoned in.
    fn reach(&self, rig: &Rig) -> f32 {
        Build::of(rig).map_or(1.0, |build| build.height)
    }

    fn drawn_extension(&self, rig: &Rig) -> f32 {
        Build::of(rig).map_or(self.extension, |build| {
            (self.extension - self.folded(&build)).clamp(0.0, 1.0)
        })
    }

    /// How far an upstroke folds the wing this frame.
    fn folded(&self, build: &Build) -> f32 {
        if build.width <= f32::EPSILON {
            return 0.0;
        }
        let u = build.stroke_at(self.phase);
        let up = if u > 1.0 {
            (std::f32::consts::PI * (u - 1.0)).det_sin().powi(2)
        } else {
            0.0
        };
        build.fold * (self.width / build.width).min(TAKEOFF_WIDTH) * up
    }

    /// The mode a frame of `flight` is posed in.
    fn mode_of(build: &Build, flight: &Flight) -> FlightMode {
        let asked = match flight.mode {
            FlightMode::Flap if !build.flaps => FlightMode::Glide,
            mode => mode,
        };
        let near = flight
            .contact
            .is_some_and(|tau| tau.is_finite() && tau <= FLARE_TAU);
        match asked {
            FlightMode::Glide | FlightMode::Soar | FlightMode::Flap if near => FlightMode::Flare,
            mode => mode,
        }
    }

    /// One frame: the mode, the clocks and every eased value moved on.
    /// `velocity` is the driver's, `turn` its eased yaw rate.
    pub(crate) fn step(
        &mut self,
        rig: &Rig,
        flight: &Flight,
        velocity: Vec3,
        turn: f32,
        delta: f32,
        carriage: Carriage,
    ) {
        let Some(build) = Build::of(rig) else {
            return;
        };
        self.carried = carriage == Carriage::Own;
        let mode = Self::mode_of(&build, flight);
        if mode != self.mode && mode == FlightMode::Land {
            self.touch_down(&build, carriage);
        }
        if mode != self.mode {
            // A takeoff asked again in the air has no ground to push off;
            // a landing starts its absorb at touchdown, below.
            if self.mode == FlightMode::Land {
                self.landing = None;
                self.absorbed = false;
            }
            if mode != FlightMode::Takeoff {
                self.pushing = None;
            }
            self.mode = mode;
            self.in_mode = 0.0;
        } else {
            self.in_mode += delta;
        }
        let air = flight.air.unwrap_or(velocity);
        let speed = if air.is_finite() { air.length() } else { 0.0 };
        self.slow = ease(
            self.slow,
            (1.0 - speed / build.cruise.max(1e-3)).clamp(0.2, 1.0),
            EASE_LEVEL,
            delta,
        );

        // The push, scripted from where the body was when it began.
        let anticipation = build.anticipation(carriage);
        let mut pushed = false;
        let mut opening = true;
        if let Some(since) = self.pushing.as_mut() {
            *since += delta;
            let t = *since;
            let lift = anticipation + build.push;
            if t < lift {
                pushed = true;
                // A bird's wings open as its legs push; a bat's and a
                // glider's, whose wings are the legs it pushes with, at its
                // end, as the feet leave.
                opening = t >= anticipation
                    && (build.early || t >= anticipation + LATE_OPEN * build.push);
                self.offset = if t < anticipation {
                    -build.crouch
                        * 0.5
                        * (1.0 - (std::f32::consts::PI * t / anticipation).det_cos())
                } else {
                    -build.crouch
                        * (std::f32::consts::FRAC_PI_2 * (t - anticipation) / build.push).det_cos()
                };
                // The wings' share, from their first downstroke to the feet
                // leaving, on a body that carries itself.
                if carriage == Carriage::Own {
                    let from = anticipation + DOWNSTROKE_FROM * build.push;
                    let span = (lift - from).max(1e-4);
                    let wings = build.legs * (1.0 - LEG_SHARE) / LEG_SHARE;
                    let gain = wings / span;
                    let into = (t - from).max(0.0);
                    self.offset += 0.5 * gain * into * into;
                }
            } else {
                // Off the ground: what the push gave is the climb's start.
                // At the speed the push left it with, legs and wings.
                if carriage == Carriage::Own {
                    self.height = self.offset;
                    self.climb = build.legs / LEG_SHARE;
                }
                self.offset = 0.0;
                self.pushing = None;
            }
        }

        // The beat's clock: its own rate, following a host's phase.
        let rate = build.rate
            * match mode {
                FlightMode::Takeoff => TAKEOFF_RATE,
                _ => 1.0,
            };
        self.rate = rate;
        // Held at the bottom through a takeoff's crouch, until the upstroke
        // that tops at its first downstroke.
        let held = self
            .pushing
            .zip(build.rising(anticipation))
            .is_some_and(|(since, from)| since < from);
        // A takeoff's first upstroke runs on its own clock to its top, which
        // is where it meets the beat: followed before then, a host's phase
        // could carry the clock back across a whole beat, where the first
        // upstroke's curve and the beat's do not meet.
        if self.lifting && self.phase >= 0.0 {
            self.lifting = false;
        }
        // Caught up on, never run backward or raced: the clock keeps
        // between half and twice its own rate, so a host whose clock runs
        // the other way, or sits a half beat off, never shakes the beat; and
        // its pace eases between them, since a clock that jumped from half
        // to twice its rate where a host's sat across the half beat moved a
        // bat's wingtip four times as far in a frame (#424, the look
        // review).
        let pace = match flight.phase.filter(|phase| phase.is_finite()) {
            Some(asked) if !pushed && !self.lifting => {
                let gap = (asked - self.phase + 0.5).rem_euclid(1.0) - 0.5;
                (1.0 + gap / (FOLLOW * rate.max(1e-3))).clamp(FOLLOW_SLOWER, 1.0 + FOLLOW_FASTER)
            }
            _ => 1.0,
        };
        self.pace = ease(self.pace, pace, FOLLOW, delta);
        if !held {
            self.phase += delta * rate * self.pace;
        }
        // Kept small, so a long flight keeps its precision.
        if self.phase > 64.0 {
            self.phase -= 64.0;
        }

        // The flare, by time to contact where there is one.
        let flare = match mode {
            FlightMode::Flare => match flight.contact.filter(|tau| tau.is_finite()) {
                Some(tau) => smooth((FLARE_TAU - tau) / (FLARE_TAU - FLARE_FULL)),
                None => smooth(self.in_mode / (FLARE_TAU - FLARE_FULL)),
            },
            _ => 0.0,
        };
        self.flare = ease(self.flare, flare, EASE_FLARE, delta);

        let targets = self.targets(&build, mode, pushed, opening);
        // A push opens a bird's wings in its tenth of a second, so its first
        // downstroke is a downstroke and not the wing still unfolding.
        let opening = if pushed { EASE_PUSH } else { 1.0 };
        // And no faster than [`EXTENSION_RATE`]: a lag moves fastest on its
        // first frame, and a wing's joints turn up to a half turn over its
        // range, so a push's opening turned a bird's feather 1.5 radians in a
        // frame at 60 a second.
        let eased = ease(
            self.extension,
            targets.extension,
            EASE_EXTENSION * opening,
            delta,
        );
        // **A landing folds its wings over a span that reads** (#424, the
        // look review): a glider's spread legs swung down and planted in two
        // frames at 60 a second, its patagium flipping from a sheet to a
        // hanging panel in the same moment; a body standing on its wings
        // puts its legs down as far as they are folded. The fold is held to
        // [`LAND_FOLD_RATE`] through a landing.
        let rate = if mode == FlightMode::Land {
            LAND_FOLD_RATE
        } else {
            EXTENSION_RATE
        };
        let most = rate * delta;
        self.extension = self.extension + (eased - self.extension).clamp(-most, most);
        // The beat's width critically damped, so it sets off from rest: a
        // lag widened a gliding bat's beat into its flare by a fifth of the
        // whole in its first frame, a pop at mid-stroke (#424, the look
        // review).
        {
            let omega = 2.0 / (EASE_WIDTH * opening);
            let off = self.width - targets.width;
            let carry = self.widening + omega * off;
            let decay = (-omega * delta).det_exp();
            self.width = (targets.width + (off + carry * delta) * decay).max(0.0);
            self.widening = (self.widening - omega * carry * delta) * decay;
        }
        self.raise = ease(self.raise, targets.raise, EASE_LIMBS, delta);
        self.sweep = ease(self.sweep, targets.sweep, EASE_LIMBS, delta);
        self.tail = ease(self.tail, targets.tail, EASE_TAIL, delta);
        self.tuck = ease(self.tuck, targets.tuck, EASE_LIMBS, delta);
        // Critically damped, so it sets off from rest: a first-order lag
        // levelled a bird out of its upright push at its fastest in the
        // first frame after lift-off, a snap.
        {
            let omega = 2.0 / EASE_LEVEL;
            let off = self.level - targets.level;
            let carry = self.levelling + omega * off;
            let decay = (-omega * delta).det_exp();
            self.level = targets.level + (off + carry * delta) * decay;
            self.levelling = (self.levelling - omega * carry * delta) * decay;
        }
        let turn = if turn.is_finite() { turn } else { 0.0 };
        self.rudder = ease(
            self.rudder,
            (RUDDER.0 * turn).clamp(-RUDDER.1, RUDDER.1),
            EASE_TAIL,
            delta,
        );
        self.banked = ease(
            self.banked,
            (BANKED.0 * turn).clamp(-BANKED.1, BANKED.1),
            EASE_LIMBS,
            delta,
        );

        // A body that carries itself hovers, and comes down to land.
        if carriage == Carriage::Own && !pushed {
            // A landing aims a little under the ground, so it arrives rather
            // than closing on it for ever, and stops there.
            let target = if mode == FlightMode::Land {
                -LAND_UNDER * build.height
            } else {
                build.hover
            };
            // Critically damped, stepped in closed form: exact at any frame
            // length, where an explicit step at this frequency diverged on a
            // frame of a quarter of a second.
            let omega = std::f32::consts::TAU * HOVER_HZ;
            let off = self.height - target;
            let carry = self.climb + omega * off;
            let decay = (-omega * delta).det_exp();
            self.height = target + (off + carry * delta) * decay;
            self.climb = (self.climb - omega * carry * delta) * decay;
            if mode == FlightMode::Land && self.height < 0.0 {
                self.height = 0.0;
                self.climb = 0.0;
            }
        } else if carriage == Carriage::Chassis {
            self.height = 0.0;
            self.climb = 0.0;
        }

        // Touchdown, for a body that carries itself and has come down.
        if mode == FlightMode::Land {
            self.touch_down(&build, carriage);
        }

        let grounded = if pushed || matches!(mode, FlightMode::Takeoff | FlightMode::Land) {
            1.0
        } else {
            0.0
        };
        self.grounding = ease(self.grounding, grounded, EASE_GROUNDING, delta);

        // The feet: held to the ground through a push, let go as it ends,
        // and put down over a landing's first moment.
        let planted = if pushed || self.landing.is_some() {
            1.0
        } else {
            0.0
        };
        // Put down over a fifth of a second, which the planted legs, the
        // levelled paws and the absorb all follow: in a twelfth, a glider's
        // spread legs came down in two frames.
        self.planted = if planted > self.planted {
            (self.planted + delta / PLANT_DOWN).min(planted)
        } else {
            (self.planted - delta / PLANT).max(planted)
        };
        // A body standing on its wings - a bat's, a glider's - absorbs only as
        // far as they are folded under it: half spread, the legs the absorb
        // bends are still wings, and sinking it put a glider's paws and a
        // cross's arms into the ground (#424, the look review).
        let on_wings = rig.ground_contacts().iter().any(|limb| limb.is_fore());
        let folded = (1.0 - self.drawn_extension(rig)).powi(2);
        if let Some(landing) = self.landing.as_mut() {
            *landing += delta;
            let absorb = build.absorb();
            let t = (*landing / absorb.max(1e-3)).min(1.0);
            self.absorbed = t >= 1.0;
            let depth = if on_wings {
                self.depth * folded
            } else {
                self.depth
            };
            // From wherever the root was carried - a push cut short by the
            // landing is still crouched - eased out over the absorb.
            self.offset =
                self.landed_from * (1.0 - smooth(t)) - depth * (std::f32::consts::PI * t).det_sin();
        } else if !pushed {
            self.offset = ease(self.offset, 0.0, EASE_LIMBS, delta);
        }
    }

    /// **Touchdown**: a landing absorbs once the feet are down - at once
    /// under a carried body, whose host says it is down; when a body that
    /// carries itself has come down to the ground, and not before, or it
    /// absorbed in mid-air and climbed back (#424, the critic). A landing
    /// asked of a body standing on the ground absorbs nothing.
    fn touch_down(&mut self, build: &Build, carriage: Carriage) {
        if self.landing.is_some()
            || (carriage == Carriage::Own && self.height > TOUCHDOWN * build.height)
        {
            return;
        }
        self.landing = Some(0.0);
        self.landed_from = self.offset;
        // Held to what the legs fold, as a takeoff's crouch is: a glider's
        // legs, crouched already, flipped their knees under a quarter of
        // their reach.
        self.depth = if self.grounded {
            0.0
        } else {
            (ABSORB * build.leg)
                .min(build.folds)
                .min(CROUCH_BELLY * build.belly)
        };
    }

    /// What `mode` asks of each eased value.
    fn targets(&self, build: &Build, mode: FlightMode, pushed: bool, opening: bool) -> Targets {
        let flare = self.flare;
        let fanned = 0.15 + 0.6 * self.slow;
        match mode {
            FlightMode::Takeoff => Targets {
                // Folded through the crouch, opening as the legs push
                // (research-11 3: the wings rise while the legs extend).
                extension: if opening { 1.0 } else { 0.0 },
                width: build.width * TAKEOFF_WIDTH,
                raise: 0.08,
                sweep: 0.0,
                tail: 0.8,
                tuck: if pushed { 0.0 } else { 1.0 },
                // A raised body leans into its push.
                level: if pushed { PUSH_LEAN } else { 1.0 },
            },
            FlightMode::Flap => Targets {
                extension: 1.0,
                width: build.width,
                raise: 0.08,
                sweep: 0.0,
                tail: fanned,
                tuck: 1.0,
                level: 1.0,
            },
            FlightMode::Glide => Targets {
                extension: 1.0,
                width: 0.0,
                raise: build.dihedral,
                sweep: 0.0,
                tail: 0.35,
                tuck: 1.0,
                level: 1.0,
            },
            FlightMode::Soar => Targets {
                extension: 1.0,
                width: 0.0,
                raise: build.dihedral + 0.06,
                sweep: 0.05,
                tail: 0.7,
                tuck: 1.0,
                level: 1.0,
            },
            FlightMode::Dive => Targets {
                extension: build.dive,
                width: 0.0,
                raise: 0.12,
                sweep: -0.15,
                tail: 0.0,
                tuck: 1.0,
                level: 1.0,
            },
            FlightMode::Flare => Targets {
                extension: 1.0,
                width: build.width * FLARE_WIDTH,
                raise: 0.08 + FLARE_RAISE * flare,
                sweep: 0.0,
                tail: 1.0,
                tuck: 1.0 - flare,
                level: 1.0 - FLARE_LEVEL * flare,
            },
            // Coming down to the ground, not yet on it: spread, the legs
            // reaching and the nose up, as a flare's end.
            FlightMode::Land if self.landing.is_none() => Targets {
                extension: 1.0,
                width: 0.0,
                raise: 0.08 + FLARE_RAISE,
                sweep: 0.0,
                tail: 1.0,
                tuck: 0.0,
                level: 1.0 - FLARE_LEVEL,
            },
            FlightMode::Land => Targets {
                extension: 0.0,
                width: 0.0,
                raise: 0.0,
                sweep: 0.0,
                tail: 0.0,
                tuck: 0.0,
                level: 0.0,
            },
        }
    }

    /// Poses this frame onto `pose`, which starts at rest: the legs, the
    /// wings and tail, the trunk levelled about its middle with the head held,
    /// the beat's heave, and the feet put on `ground` as far as they are held
    /// to it.
    ///
    /// **And no wing into the ground** (#424, the look review): wherever a
    /// beat would carry a wing's joint under the ground - the takeoff's
    /// first beats, the landing's last, a host that keeps the body low - its
    /// stroke is raised by as much as brings it clear, and posed again: a
    /// partial stroke, raised, as a bird or a bat beats close to the ground.
    /// The raise is measured on the joints, so it is held short of a jump: a
    /// pass that would leave the wing further under is undone.
    pub(crate) fn pose<F>(&self, rig: &Rig, pose: &mut Pose, ground: F, carriage: Carriage)
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let fresh = pose.clone();
        let extension = self.drawn_extension(rig);
        // Not a folded wing, which is the ground posture's own; nor a wing
        // being put down as a leg, which the ground holds and a raise would
        // turn against: a bat's or a glider's, as far as it is.
        // (Its wings the legs it lands on, a bat or a glider gives up the
        // raise as its feet are put down: raised, its tilting paws held the
        // root up and let it fall when the raise let go.)
        let held = if rig.ground_contacts().iter().any(|limb| limb.is_fore()) {
            self.planted
        } else {
            0.0
        };
        let scale = (1.0 - held) * smooth(extension / RAISE_OPEN);
        self.compose(rig, pose, &ground, carriage, [0.0; 2]);
        if scale <= 0.0 {
            return;
        }
        let mut short = self.under_ground(rig, pose, &ground);
        let mut lift = [0.0f32; 2];
        // Raised until clear, and never further than helps: a pass that
        // leaves a wing further under than the last is undone.
        for _ in 0..RAISE_PASSES {
            let worst = short[0].max(short[1]);
            if worst <= 1e-4 {
                return;
            }
            let next =
                [0, 1].map(|side| (lift[side] + short[side]).min(std::f32::consts::FRAC_PI_2));
            let mut trial = fresh.clone();
            self.compose(
                rig,
                &mut trial,
                &ground,
                carriage,
                next.map(|lift| lift * scale),
            );
            let after = self.under_ground(rig, &trial, &ground);
            if after[0].max(after[1]) >= worst {
                return;
            }
            *pose = trial;
            lift = next;
            short = after;
        }
    }

    /// How far each wing - left, right - must be raised for no joint of it
    /// to stand under the ground in `pose`, in radians: a joint the fold
    /// rests on the ground (a bat's wrist) may stand on it, and every other
    /// is kept [`WING_CLEAR`] of the body's height over it.
    fn under_ground<F>(&self, rig: &Rig, pose: &Pose, ground: &F) -> [f32; 2]
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let Some(wings) = rig.wings.as_ref() else {
            return [0.0; 2];
        };
        let clear = WING_CLEAR * rig.extent();
        let posed = pose.forward(rig);
        let mut short = [0.0f32; 2];
        for side in &wings.sides {
            let upper = rig.in_zone(Zone::UpperLimb(side.limb));
            let Some(&shoulder) = upper.len().checked_sub(2).and_then(|at| upper.get(at)) else {
                continue;
            };
            let from = posed.positions[shoulder];
            // A feather reaches past the joint it rides by up to its own
            // length, outward along the wing: a crow's primaries two humeri.
            let overhang = if wings.tail.is_empty() {
                0.0
            } else {
                upper.last().map_or(0.0, |&elbow| {
                    rig.joints[elbow]
                        .position
                        .distance(rig.joints[shoulder].position)
                }) * FEATHER_OVERHANG
            };
            let mut most = 0.0f32;
            // Every joint of the wing, to the digits' tips the drives end
            // short of.
            for joint in (0..rig.len()).filter(|&joint| {
                matches!(
                    rig.joints[joint].zone,
                    Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb)
                        if limb == side.limb
                )
            }) {
                let root = posed.positions[joint];
                let at = if wings.owns(joint) {
                    root + (root - from).normalize_or_zero() * overhang
                } else {
                    root
                };
                let Some(below) = ground(at) else {
                    continue;
                };
                // Its skin under it by its radius, as at rest.
                let wanted = below.position.y
                    + (clear + rig.joints[joint].radius).min(rig.joints[joint].position.y);
                let deficit = wanted - at.y;
                if deficit > 0.0 {
                    let arm = at.distance(from).max(1e-3);
                    most = most.max((deficit / arm).min(1.0).det_asin());
                }
            }
            short[usize::from(!side.limb.is_left())] = most;
        }
        short
    }

    /// [`Self::pose`]'s one pass, each wing's stroke raised by `lift`
    /// radians, left and right.
    fn compose<F>(&self, rig: &Rig, pose: &mut Pose, ground: &F, carriage: Carriage, lift: [f32; 2])
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let Some(build) = Build::of(rig) else {
            return;
        };
        let Some(wings) = rig.wings.as_ref() else {
            return;
        };
        let pi = std::f32::consts::PI;
        let u = build.stroke_at(self.phase);
        let share = if build.width > f32::EPSILON {
            (self.width / build.width).min(TAKEOFF_WIDTH * 1.2)
        } else {
            0.0
        };
        // A takeoff's first upstroke rises from the wing's rest to the top,
        // meeting the beat there at its top and still.
        let beat = if self.lifting && self.phase < 0.0 {
            0.5 * (1.0 - (pi * (u - 1.0).max(0.0)).det_cos())
        } else {
            (pi * u).det_cos()
        };
        let swing = (pi * u).det_sin();
        let flare = self.flare;

        // The legs a wing does not drive - a bird's - tucked toward the hip
        // and swung forward into a flare, in the body as it rests.
        let driven = |joint: usize| {
            wings
                .sides
                .iter()
                .any(|side| side.drives.iter().any(|drive| drive.joint == joint))
        };
        let contacts = rig.ground_contacts();
        for &limb in &contacts {
            let (Some(chain), Some(&foot)) = (
                rig.limb_chain(limb),
                rig.in_zone(Zone::Extremity(limb)).first(),
            ) else {
                continue;
            };
            if chain.iter().any(|&joint| driven(joint)) {
                continue;
            }
            let Some(pole) = rig.bend_pole(limb) else {
                continue;
            };
            let hip = rig.joints[chain[0]].position;
            let home = rig.joints[foot].position;
            let reach = Rot::x(-FLARE_LEGS * flare) * (home - hip);
            let target = hip + reach * (1.0 - TUCK * self.tuck);
            if target.distance(home) > 1e-5 {
                solve_contact_toward(rig, pose, limb, target, pole);
            }
        }

        // The wings at their stroke, and the tail.
        let elevation = self.raise + self.width * beat;
        let sweep = self.sweep + build.sweep * share * swing;
        let extension = (self.extension - self.folded(&build)).clamp(0.0, 1.0);
        let reach = FLARE_REACH * flare;
        let side = |banked: f32, steer: f32, raised: f32| Stroke {
            extension: (extension - steer).clamp(0.0, 1.0),
            elevation: elevation + banked + raised,
            sweep,
            reach,
        };
        // A glider steers by its limbs, one side's membrane eased (research-11
        // 5), a flapper by its wings' difference: the outer wing up.
        let steer = if build.flaps { 0.0 } else { self.banked };
        rig.stroke_wings(
            pose,
            side(-self.banked, steer.max(0.0), lift[0]),
            side(self.banked, (-steer).max(0.0), lift[1]),
        );
        rig.steer_tail(pose, self.tail, self.rudder, FLARE_TAIL * flare);

        // The trunk levelled about its middle, the beat's pitch and a flare's
        // nose over it, and the head held as it was.
        let nose_up = build.pitch * self.slow * share * swing * (1.0 - self.planted)
            + if build.raised > 0.3 {
                0.0
            } else {
                FLARE_NOSE * flare
            };
        let pitch = build.raised * self.level - nose_up;
        if pitch != 0.0
            && let Some(root) = (0..rig.len()).find(|&joint| rig.joints[joint].parent.is_none())
        {
            let turn = Rot::x(pitch);
            pose.rotations[root] = (turn * pose.rotations[root]).normalize();
            let at = rig.joints[root].position + pose.translation;
            pose.translation += (build.pivot - at) - turn * (build.pivot - at);
            let neck = rig.in_zone(Zone::Neck);
            let holding = if neck.is_empty() {
                rig.in_zone(Zone::Head)
                    .first()
                    .copied()
                    .into_iter()
                    .collect()
            } else {
                neck
            };
            if !holding.is_empty() {
                let each = Rot::x(-pitch / holding.len() as f32);
                for joint in holding {
                    pose.rotations[joint] = (each * pose.rotations[joint]).normalize();
                }
            }
        }

        // The beat's heave, a slow beat's, and what carries the root.
        let heave = -build.heave * self.slow * share * beat * (1.0 - self.planted);
        let carried = match carriage {
            Carriage::Own => self.height,
            Carriage::Chassis => 0.0,
        };
        pose.translation.y += heave + self.offset + carried;

        // **The hind feet flatten to meet the ground as it nears** (#424,
        // the look review): reaching into a landing, a bird's claws pointed
        // down and dug a few millimetres in for the frames its feet took to
        // be put down. Levelled against the ground under them, the nearer
        // the more.
        if self.pushing.is_none() {
            let posed = pose.forward(rig);
            let feet: Vec<(Limb, usize)> = contacts
                .iter()
                .filter(|limb| !limb.is_fore())
                .filter_map(|&limb| Some((limb, *rig.in_zone(Zone::Extremity(limb)).first()?)))
                .collect();
            let gap = feet
                .iter()
                .filter_map(|&(_, foot)| {
                    let at = posed.positions[foot];
                    ground(at).map(|below| at.y - below.position.y - rig.joints[foot].position.y)
                })
                .fold(f32::MAX, f32::min);
            // Less as the feet are put down, which levels them anyway.
            let near = if gap < f32::MAX {
                (1.0 - (gap / (NEAR_GROUND * build.leg).max(1e-4)).clamp(0.0, 1.0))
                    * (1.0 - self.planted)
            } else {
                0.0
            };
            // And a wing's own paw put down in a landing - a glider's - is
            // levelled as far as the plant does not yet hold it, its arm
            // still half spread: tilted with the spread arm, its toes went
            // in.
            let paws = if self.landing.is_some() {
                self.planted * (1.0 - (1.0 - extension).powi(2))
            } else {
                0.0
            };
            if near > 0.0 || paws > 0.0 {
                let mut level = pose.clone();
                crate::anim::ground::level_feet(rig, &mut level, ground, &FootingConfig::default());
                for joint in 0..rig.len() {
                    if wings.owns(joint) {
                        continue;
                    }
                    let Zone::Extremity(limb) = rig.joints[joint].zone else {
                        continue;
                    };
                    let weight = if feet.iter().any(|&(foot, _)| foot == limb) {
                        near
                    } else if limb.is_fore() && contacts.contains(&limb) {
                        paws
                    } else {
                        0.0
                    };
                    if weight > 0.0 {
                        pose.rotations[joint] =
                            det::slerp(pose.rotations[joint], level.rotations[joint], weight);
                    }
                }
            }
        }

        // The feet on the ground, as far as they are held to it: a wing's
        // own limb only as far as it is folded, since a spread wing is no
        // leg.
        if self.planted > 0.0 {
            // **A wing's own limb is put down from its fold** (#424, the
            // critic): solved where it stood half spread, a glider-led
            // cross's fore limb found the other bend between two frames of a
            // landing and turned two radians in one. Its chain is laid back
            // at rest - the fold, which stands on the ground - and solved
            // from there, and the wing comes down onto that as it folds.
            let wing_limb = |limb: Limb| {
                rig.limb_chain(limb)
                    .is_some_and(|chain| chain.iter().any(|&joint| driven(joint)))
            };
            let mut planted = pose.clone();
            for joint in 0..rig.len() {
                if wings.owns(joint) {
                    continue;
                }
                if let Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) =
                    rig.joints[joint].zone
                    && contacts.contains(&limb)
                    && wing_limb(limb)
                {
                    planted.rotations[joint] = Quat::IDENTITY;
                }
            }
            let config = FootingConfig {
                max_pelvis_drop: 0.0,
                ..FootingConfig::default()
            };
            plant_feet_of(rig, &mut planted, &contacts, ground, &config);
            let folded = (1.0 - extension).powi(2);
            for (joint, (rotation, held)) in pose
                .rotations
                .iter_mut()
                .zip(&planted.rotations)
                .enumerate()
            {
                let weight = match rig.joints[joint].zone {
                    Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb)
                        if wing_limb(limb) =>
                    {
                        self.planted * folded
                    }
                    _ => self.planted,
                };
                if weight > 0.0 && *rotation != *held {
                    *rotation = det::slerp(*rotation, *held, weight);
                }
            }
        }
        // **And no foot under the ground** (#424, the look review): a
        // landing's absorb sank a glider's paws 1-2 cm while its folding
        // wings let them go, and a bird reaching into its landing put its
        // toes in before its hover had brought it down; so the root is held
        // up by whatever a foot would sink - its sole reckoned under each of
        // its joints as far as it stands at rest, and further as it tilts,
        // its toes going down with it.
        let posed = pose.forward(rig);
        let sunk = contacts
            .iter()
            .flat_map(|&limb| rig.in_zone(Zone::Extremity(limb)))
            .filter(|&joint| !wings.owns(joint))
            .filter_map(|foot| {
                let at = posed.positions[foot];
                let down = posed.rotations[foot] * Vec3::NEG_Y;
                let cos = (-down.y).clamp(0.0, 1.0);
                let sin = (1.0 - cos * cos).max(0.0).sqrt();
                let sole = rig.joints[foot].position.y.max(0.0) * (cos + SOLE_TILT * sin);
                ground(at).map(|below| below.position.y + sole - at.y)
            })
            .fold(0.0f32, f32::max);
        pose.translation.y += sunk * self.grounding;
    }
}
