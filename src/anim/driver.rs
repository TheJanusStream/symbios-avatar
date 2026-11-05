//! One body's motion, driven from what is carrying it.
//!
//! Every other module here answers one question about a body: how a leg swings
//! ([`super::gait`]), what a stopped body does with its arms ([`super::idle`]),
//! how a jump's three parts agree at their seams ([`super::leap`]). Something
//! has to decide **which of them is running this frame**, keep the clocks that
//! outlive a frame, and join two of them when the answer changes. That is this
//! module, and until now every consumer wrote it again.
//!
//! # What it is fed
//!
//! A [`Driver`] is fed a **chassis**, not a control surface: how fast the body
//! is travelling and which way, where it is in the world, and a handful of
//! facts only its owner knows — whether it is in deep water, whether a gesture
//! was just asked for, whether an editor is holding it still. Everything else
//! it works out. That is the whole shape of [`Inputs`], and it is why one
//! driver can serve an application whose bodies are pushed around by a physics
//! engine and a viewer whose body stands on a turntable.
//!
//! ```
//! use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
//! use symbios_avatar::{AvatarRecord, Rig, Vec3};
//!
//! let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;
//! let mut driver = Driver::seeded(7);
//! let driven = driver
//!     .drive(
//!         &rig,
//!         &Inputs {
//!             delta: 1.0 / 60.0,
//!             velocity: Vec3::Z * 1.4,
//!             ..Inputs::default()
//!         },
//!         level_ground,
//!     )
//!     .expect("a body that is not being held is posed");
//! assert_eq!(driven.source, symbios_avatar::anim::driver::Source::Gait);
//! # Ok::<(), symbios_avatar::RigError>(())
//! ```
//!
//! # What it decides, and the measured reasons
//!
//! Each of these was arrived at by measuring a consumer that had it wrong, and
//! the constant or the rule carries the reading on its own documentation:
//!
//! * **Speed picks the gait, and nothing else does** ([`Speed`]). A faster body
//!   takes a longer step rather than the same step more often, and changes gait
//!   on its own where walking stops working.
//! * **The pace the gait sees is eased and the source decision is not**
//!   ([`DriverConfig::pace_response`]). A chassis that assigns velocity rather than
//!   damping it steps between speeds in one frame, and no gait looks human fed
//!   a step function — but a body that holds its walk after the world has
//!   stopped is dragging a planted foot.
//! * **Airborne is a state** ([`Airborne`]). Every instantaneous test fails at
//!   the apex of a jump, which is the most airborne a body ever is.
//! * **A stop hands the idle the stance it arrived in** ([`Idle::arrive`]).
//!   Waiting for a better moment to stop is itself a skate.
//! * **A change of duty carries the cycle, not the number**
//!   ([`transition::carry_cycle`]).
//! * **A planted foot is held at the world point it went down at**
//!   ([`Footholds`]), whenever something else is carrying the body — turned
//!   round with the body when it turns, and yielding rather than holding once
//!   the stride has moved too far from it, because a hold kept through a turn
//!   splays the legs and one the body outruns drags the body to the floor.
//!
//! # The one thing a caller must still decide
//!
//! [`Carriage`] — whether the body's root is moved by something else or by the
//! motion itself. A jump is the case that makes it unavoidable: [`Leap::drive`]
//! carries the root through the parabola, which is right for a body that owns
//! its own root and doubles the flight of one hanging off a physics capsule.
//! There is no safe guess, so it is a field with no default that suits both.
//!
//! # What it deliberately does not own
//!
//! Where the velocity came from, what counts as deep water, which body a
//! request names, and what the face is doing. The first three are facts about
//! the world the caller lives in; the last is a layer laid over the body, and
//! [`Inputs::over_settled`] is where it goes.

use glam::{Vec2, Vec3};

use crate::det::{self, DetMath};
use crate::face::{Expression, Eyes};
use crate::plan::Limb;
use crate::rig::Rig;

use super::blend::Inertializer;
use super::foothold::Footholds;
use super::gait::{Gait, Steps, Stride, Walk};
use super::gaze::{GazeConfig, look_at};
use super::ground::{FootingConfig, Ground};
use super::heading::Heading;
use super::idle::{Idle, IdleConfig, Idled};
use super::leap::{Leap, Stage};
use super::pose::Pose;
use super::speed::{FROUDE_SPRINT, GRAVITY, Speed};
use super::spring::{SpringConfig, Springs};
use super::swim::Swim;
use super::transition::{self, Family};
use super::turn::{TURN_REACH, Turn};
use super::{Lead, Pattern, Target, gesture};

/// The yaw rate a body standing still must turn at to step round rather than
/// idle, in radians a second — about nine degrees a second (#361).
///
/// **Slow enough that a look round steps.** Below it a standing body idles,
/// and the idle poses its feet in the body, so they turn with it: at this rate
/// a foot a tenth of a metre out goes round by 15 mm a second, under the
/// idle's own weight shifts. Above it the body walks a pivot, a gait of no
/// length whose stances turn instead ([`Turn`]), and its planted feet stay
/// where they are. A body already stepping keeps stepping down to half of it,
/// so a turn easing off near the line does not flicker between the two.
pub const PIVOT_FROM: f32 = 0.15;

/// How close the eased turn comes to the turn asked for before it is that
/// turn, in radians a second (#361).
///
/// **A ten-thousandth, which no stance can show**: at a walk's cadence it
/// turns a stance by a few thousandths of a degree. What it buys is a lag that
/// lands — on exactly the turn asked for, and so on exactly zero when a turn
/// ends, which is the straight stride to the bit.
const TURN_SETTLED: f32 = 1e-4;

/// How much longer than [`DriverConfig::blend`] a four-legged body takes to
/// change its pattern, a walk breaking into a trot (#412). Provenance:
/// **measured** (#412): over a sweep of forty moments for the change, a dog's,
/// a cat's, a horse's and eight rolled bodies', level and up to .4, at 60 and
/// 144 frames a second, the worst joint went at one blend 1.55 times what a
/// frame allows, 1.31 on the level at 60; at 1.5 blends 1.16 and 1.00; at
/// 1.75, 1.12 and 0.91; at 2, 1.13 and 0.88.
const REPATTERN_BLEND: f32 = 1.75;

/// How much longer than [`DriverConfig::blend`] a four-legged body takes going
/// into or out of the water on a slope, at the steepest (#413). Provenance:
/// **chosen** (#413): three blends, which brings a 25 degree slope's whole turn
/// under two degrees a frame at sixty frames a second.
const SWIM_TURN: f32 = 2.0;

/// A layer a caller lays over the body part-way through a driven frame.
///
/// Everything a layer is given is already posed, and it writes only what it
/// addresses — which is what lets an authored clip ride a procedural walk, and
/// a face be written over both.
pub type Layer<'a> = &'a dyn Fn(&Rig, &mut Pose);

/// A level floor at `y = 0`, as a ground closure.
///
/// **Published rather than left to each caller to write**, because writing it
/// twice is how a body ends up with two floors: the stride seats its contacts
/// on one surface and the plant settles them onto another, which puts a swing
/// arc through a hill and was the shape of two separate defects. A caller with
/// real terrain answers from it instead; a caller standing a body on a plane
/// hands this over and never thinks about it again.
///
/// It is a plain function item, so it is [`Copy`] and can be given to
/// [`Driver::drive`] directly.
#[must_use]
pub fn level_ground(point: Vec3) -> Option<Ground> {
    Some(Ground::level(Vec3::new(point.x, 0.0, point.z)))
}

/// The velocity implied by a body having moved from `previous` to `now`.
///
/// **For a body whose motion arrives as positions rather than as a velocity** —
/// a remote peer played back from what the network said, an instrument marching
/// a transform. Defined here rather than at each call site so two consumers
/// cannot disagree about it, and so the one edge case is written down once: a
/// zero or negative `delta` has no velocity in it and answers zero rather than
/// an infinity that would read as a launch.
#[must_use]
pub fn velocity_of(previous: Vec3, now: Vec3, delta: f32) -> Vec3 {
    if delta <= 0.0 {
        return Vec3::ZERO;
    }
    (now - previous) / delta
}

/// What moves the body's root.
///
/// **The one decision a caller cannot avoid making**, because the two answers
/// differ by a whole flight arc rather than by a detail. It is not derivable:
/// nothing in a velocity says whether the thing that produced it is also going
/// to move the body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Carriage {
    /// Something else carries the body, and the motion must not carry it
    /// again.
    ///
    /// The default, because it is what a driver fed a chassis is for. A leap's
    /// flight height is given back after the drive rather than never asked for
    /// — the drive plants its contacts at the height it applied, so in flight
    /// there is nothing planted and subtracting afterward is exact, while on
    /// the ground there is nothing to subtract. The wind-up's and the landing's
    /// depths are **kept**: those are legs compressing, which a capsule does
    /// not do.
    ///
    /// A body given this while nothing external moves it never leaves the
    /// ground.
    #[default]
    Chassis,
    /// The body carries itself, as the engine's own motions assume.
    ///
    /// A leap flies, and a walk derives every stance offset afresh rather than
    /// holding world points — which is right for a body walking on the spot,
    /// where a foothold ledger would pin a treadmill's feet to the floor and
    /// tear the walk apart.
    ///
    /// A body given this while a chassis also moves it flies twice.
    Own,
}

/// An editor's grip on a body.
///
/// Both holds exist because a body that is being *edited* is being measured
/// against something, and a body that keeps moving cannot be. They differ in
/// what they are measured against, which is why one pose is not enough for
/// both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Hold {
    /// Nothing is holding the body.
    #[default]
    None,
    /// Pin the body to its bind pose.
    ///
    /// For editing something recorded in a joint's **rest** frame: the offset
    /// being dragged and the body it is measured against then agree. A hard
    /// snap rather than a blend, because a body still settling would let a
    /// release land against a pose that is already gone.
    Rest,
    /// Hold the body exactly where it stands.
    ///
    /// For editing something committed against the body's **current** pose,
    /// which may be any pose so long as it does not move. [`Driver::drive`]
    /// answers [`None`] and advances no clock, so the caller's last pose stays
    /// applied and the motion resumes from precisely where it paused. Re-posing
    /// to rest here moves a freshly detached part's parent out from under it,
    /// so selecting visibly shifts the thing being selected.
    Pose,
}

/// What is carrying the body this frame.
///
/// A change is what starts a blend — but only between families that share no
/// clock, which is [`Family`]'s question and not this enum's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// Pinned to the bind pose, or standing with the idle switched off.
    ///
    /// Kept distinct from [`Self::Idle`] so releasing a hold reads as a source
    /// change and blends back out instead of snapping.
    Rest,
    /// Standing: breath, sway, weight shift, fidgets.
    Idle,
    /// Travelling on the ground, on the speed axis.
    Gait,
    /// Off the ground, and landing again.
    Leap,
    /// In deep water: treading, or crawling if travelling.
    Swim,
    /// A gesture riding over whatever else the body is doing.
    Gesture,
}

impl Source {
    /// Which of the engine's motion families this is, or [`None`] for a hold.
    ///
    /// [`Self::Rest`] has no family and that is not an omission. It is a held
    /// pose rather than a generator, so a change into or out of it has no clock
    /// to be continuous with and always blends. Answering [`Family::Idle`]
    /// would silently stop a release from blending into an idle that is up to a
    /// sway's amplitude away from the pose it was pinned at.
    #[must_use]
    pub fn family(self) -> Option<Family> {
        match self {
            Source::Rest => None,
            Source::Idle => Some(Family::Idle),
            Source::Gait => Some(Family::Locomotion),
            Source::Leap => Some(Family::Jump),
            Source::Swim => Some(Family::Swim),
            Source::Gesture => Some(Family::Expressive),
        }
    }

    /// Whether moving from `self` to `into` needs a blend at all.
    ///
    /// Every speed is one family, which is the point of the speed axis: a walk
    /// becoming a run is a change of duty inside [`Family::Locomotion`], and
    /// the discontinuity that really is there at that moment belongs to the
    /// clock — [`transition::carry_cycle`] fixes it rather than a blend
    /// smearing it.
    #[must_use]
    pub fn needs_blend(self, into: Self) -> bool {
        match (self.family(), into.family()) {
            (Some(from), Some(into)) => from.needs_blend(into),
            // A hold at either end: always, for the reason `family` gives.
            _ => self != into,
        }
    }
}

/// A body that is off the ground, and the leap describing it.
///
/// **Built twice, and that is the shape of the problem rather than a
/// hesitation.** [`Leap`] wants to know at the start how far the body will
/// fall, because the landing's depth and the flight's duration both come off
/// it; a body carried by something else has that decided for it after the fact.
/// So this carries the leap the takeoff implied while the body is rising, and
/// is replaced at touchdown by one built from the impact that actually arrived
/// — which is the number the landing needs and the only one it needs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Airborne {
    /// The leap being driven.
    pub leap: Leap,
    /// Seconds into it, on [`Leap`]'s own clock.
    pub elapsed: f32,
    /// Whether the body has begun falling, which is what makes the landing edge
    /// findable — see [`DriverConfig::settle_speed`].
    pub falling: bool,
    /// The fastest the body has travelled downward, in m/s, which at touchdown
    /// is the impact the legs have to absorb.
    pub impact: f32,
    /// Whether the feet are back on the ground and this is playing out the
    /// landing.
    pub landed: bool,
}

/// A motion a caller is showing rather than one the chassis implies.
///
/// **For an instrument, not for an application.** A jump cannot be judged from
/// a table — its whole quality is whether the wind-up, the flight and the
/// landing read as one movement, and the numbers say only that they meet — so
/// something has to be able to put one on the screen and hold it at a chosen
/// instant. A body being shown this way has no chassis to infer it from.
///
/// It outranks everything the velocity implies, so a caller showing a leap gets
/// that leap and not the one a fall would have produced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Showing {
    /// This leap, its elapsed time taken from the cycle: `0` is the start of
    /// the wind-up and `1` is standing again, so scrubbing the cycle scrubs the
    /// jump exactly as it scrubs a gait.
    Leap(Leap),
    /// This swim, on the driver's own cycle.
    Swim(Swim),
}

/// The walk's ablation switches, as a caller sets them.
///
/// Separate from [`DriverConfig`] because these are a *frame's* question — an
/// instrument turns the posture off for one capture and back on for the next —
/// where a config is a property of the body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WalkFlags {
    /// Whether the postural layer runs: the arms swinging against the legs and
    /// the trunk leaning into the walk.
    pub posture: bool,
    /// Whether the neck takes the trunk's lean back off to hold the head level.
    pub head_level: bool,
    /// How to settle the stance contacts onto the ground, or [`None`] to leave
    /// the gait's own placement untouched — which is what an instrument
    /// measuring how much work the solve does wants.
    pub footing: Option<FootingConfig>,
    /// How to aim the head down the path the body is walking, or [`None`] to
    /// leave the gaze alone.
    ///
    /// [`None`] by default, deliberately: where a body looks is very often
    /// something its caller is already deciding, and a walk that quietly took
    /// the gaze over would be overwriting an intention rather than filling a
    /// gap.
    pub gaze: Option<GazeConfig>,
}

impl Default for WalkFlags {
    /// Exactly what [`Walk::at`] describes: posture on, head level, feet
    /// settled with the default footing, gaze left alone.
    fn default() -> Self {
        Self {
            posture: true,
            head_level: true,
            footing: Some(FootingConfig::default()),
            gaze: None,
        }
    }
}

/// The numbers that decide how one body behaves, as opposed to what it is doing
/// this frame.
///
/// Every one of these was measured on a real consumer, and each field's
/// documentation carries the reading rather than the taste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DriverConfig {
    /// What moves the body's root. See [`Carriage`] — there is no answer that
    /// suits both kinds of caller.
    pub carriage: Carriage,
    /// Whether a body with nothing else to do stands and breathes.
    ///
    /// **On, because a body doing nothing is the state most bodies are in most
    /// of the time**, and the failure mode of leaving it off is silent: a rest
    /// pose is a perfectly valid pose, written every frame, forever. Off is
    /// what an instrument wants when it is looking at the rest pose itself — a
    /// body that is breathing has no frame that *is* the rest pose.
    pub idle: bool,
    /// Below this horizontal speed the body idles, in m/s — unless it is
    /// turning faster than [`PIVOT_FROM`], when it steps round where it stands
    /// (#361).
    pub idle_below: f32,
    /// How long the gait takes to believe a change of speed, in seconds.
    ///
    /// The engine's postural terms — trunk lean, stride, duty, the neck's
    /// counter — are pure functions of the pace they are fed, so they are
    /// exactly as continuous as the thing feeding them. A chassis that assigns
    /// velocity rather than damping it reaches tens of metres per second
    /// squared, and fed raw, a speed step unloads the whole trunk pitch between
    /// two frames. The pace the speed axis reads is therefore eased through a
    /// first-order lag with this time constant, of the order of one step, so a
    /// change of speed reads as intent the body carries out rather than as a
    /// snap.
    ///
    /// **The pace only, never the source.** Stopping hands over to the idle on
    /// the frame the chassis stops, because holding a gait while the world
    /// stands still is a measured skid. Set it to zero for a caller whose own
    /// speed is already smooth, or for an instrument that wants the step.
    pub pace_response: f32,
    /// Upward speed that can only be a launch, in m/s.
    ///
    /// Nothing a body does on the ground pushes it up this fast, and a walk on
    /// rough ground never approaches it. Deliberately clear of both: the cost
    /// of missing a launch is one frame of walk cycle, and the cost of a false
    /// one is a body that tucks its legs while standing on a kerb.
    pub launch_speed: f32,
    /// Downward speed past which a body has left the ground rather than walked
    /// down something, in m/s.
    ///
    /// The other way into the air: a body that steps off a ledge never
    /// launches, and reaches this within about a fifth of a second of having
    /// nothing under it.
    pub fall_speed: f32,
    /// Vertical speed below which a body counts as no longer falling, in m/s.
    ///
    /// **The landing edge, and the reason airborne has to be a state.** No
    /// instantaneous test can find it: at the apex of a jump the vertical speed
    /// is zero, which is the most airborne a body ever is. What cannot be
    /// confused with the apex is a body that *has* been falling and has
    /// stopped, because at the apex it is still on its way down.
    pub settle_speed: f32,
    /// How long a source switch blends, in seconds. Zero snaps.
    pub blend: f32,
    /// How long a gesture takes, in seconds.
    ///
    /// The engine's gestures are written in normalised time — a goal-space clip
    /// runs `0..1` and says nothing about seconds - so this is where the real
    /// duration is decided. A second and a half is a greeting: long enough for
    /// three waves to read as waves, short enough that a body is not still
    /// doing it when the conversation has moved on.
    ///
    /// **A beast's states keep their own time** (#377): a sit or a lying down
    /// goes down at the pace [`super::emote::seconds`] gives it on the body
    /// and is held, for as long as nothing gets the beast up - its being
    /// moved, leaving the ground or swimming, which gets it up in a hurry
    /// ([`super::emote::HURRY`]), or another gesture asked for, or
    /// [`super::emote::GET_UP`], at the pace [`super::emote::getting_up`]
    /// gives. This is read for none of it.
    pub gesture_secs: f32,
    /// The shortest gap between two gestures on one body, in seconds.
    ///
    /// Measured from one gesture's **start** rather than its end, so the limit
    /// is a rate and not a gap: a sender pasting the same word four times
    /// waves once and then stands there. Longer than [`Self::gesture_secs`], so
    /// a well-behaved sender is never throttled by it - and never by a beast's
    /// state, which lasts as long as it is held: a request while it lasts is
    /// not rate-limited but played once the beast is up (#377).
    pub gesture_cooldown: f32,
    /// How far one stroke of a crawl carries the body, in its own lengths.
    ///
    /// **One, and both ends of the stroke's clock are derived from it.**
    /// [`Swim`] leaves the cadence to its caller on purpose — a tread and a
    /// crawl run the same loops and differ in how fast the caller advances them
    /// — so this is where the seconds are decided. A body covering its own
    /// length per cycle at the engine's own full effort is stroking about
    /// seven tenths of a time a second, which is inside the band competitive
    /// swimmers hold.
    pub lengths_per_stroke: f32,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            carriage: Carriage::default(),
            idle: true,
            idle_below: 0.3,
            pace_response: 0.3,
            launch_speed: 2.0,
            fall_speed: 2.0,
            settle_speed: 0.5,
            blend: 0.15,
            gesture_secs: 1.5,
            gesture_cooldown: 2.0,
            lengths_per_stroke: 1.0,
        }
    }
}

/// Everything the caller knows and the driver cannot work out, for one frame.
///
/// Built with struct-update from [`Default`], which is the shape every caller
/// wants: an application fills the first handful of fields and leaves the rest,
/// and an instrument reaches for the overrides underneath.
pub struct Inputs<'a> {
    /// Seconds since the last frame. Zero or less is a frame that did not
    /// happen, and [`Driver::drive`] answers [`None`] for it.
    pub delta: f32,
    /// How fast the body is travelling, in world metres per second.
    ///
    /// Horizontal magnitude picks the gait and its speed; the **signed**
    /// vertical is the whole of the airborne state machine, so an absolute
    /// value here throws away which way the body is going and lands it at every
    /// apex. [`velocity_of`] derives one from two positions for a caller whose
    /// motion arrives that way.
    pub velocity: Vec3,
    /// Where the body is in the world — the origin the pose will be rendered
    /// under. Only its horizontal is read; the vertical belongs to the ground
    /// closure.
    pub at: Vec3,
    /// The yaw about `+Y` carrying the body's own `+Z` onto its world heading.
    ///
    /// **Not the chassis' rotation.** A consumer whose body hangs off a root
    /// that corrects between its own forward convention and the engine's must
    /// carry that correction into this angle, or every held foothold is
    /// mirrored through the body and a walking body reads as skating.
    pub facing: f32,
    /// Which way the body **travels**, relative to the way it faces, or
    /// [`None`] for straight ahead.
    ///
    /// [`None`] is not merely the zero angle: it leaves the stride exactly as
    /// the speed axis built it, which is what a caller wants when its facing
    /// and its travel are kept in step by something else. A caller whose body
    /// can strafe or back up derives one and passes it. Either way the stride
    /// is stepped at the cadence that covers the ground the body covers
    /// (#362, #363).
    pub heading: Option<Heading>,
    /// Whether the body is in water deep enough to swim in.
    ///
    /// The caller's classification, because what counts as deep is a fact about
    /// the world and its water rather than about the body. A wading body has
    /// its feet on the bottom and is walking.
    pub swimming: bool,
    /// A gesture asked for **this frame**, by the name
    /// [`gesture::for_body`] knows it as on this body: a person's roster, and
    /// on four legs a beast's (#370).
    ///
    /// A request, not a state: hold it for one frame and let it go. A name the
    /// roster does not carry is ignored and does not spend the cooldown.
    pub gesture: Option<&'a str>,
    /// Whether an editor is holding the body, and how.
    pub hold: Hold,
    /// A motion to show instead of the one the chassis implies.
    pub showing: Option<Showing>,
    /// Hold the cycle at this point instead of running it.
    ///
    /// The instrument's control: a gait judged at whatever phase a frame
    /// happened to land on is how a walk gets called stiff when it has only
    /// ever been seen at mid-stance.
    pub cycle: Option<f32>,
    /// Run the cycle at this many cycles a second instead of the one the speed
    /// implies.
    ///
    /// For a body being shown rather than driven — one walking on the spot has
    /// no speed to derive a cadence from.
    pub cadence: Option<f32>,
    /// Walk in this pattern instead of the one the speed picks.
    ///
    /// A gait that looks right at the pattern a body chose can still be wrong
    /// at another, which is the only reason to override it.
    pub gait: Option<&'a Gait>,
    /// Move in this pattern instead of the one the speed picks, on four legs
    /// (#370): a walk, a trot, a canter or a gallop asked for by name. Its
    /// cycle and cadence are still the speed's own and its duty the asked
    /// pattern's - at this speed, or past either end of the pattern's own
    /// speed band, at that end - so the stride is the travel its stance
    /// covers and its feet keep to the ground. [`None`] picks by speed; a body
    /// that is not four-legged ignores it.
    pub pattern: Option<Pattern>,
    /// The walk's ablation switches.
    pub walk: WalkFlags,
    /// How fast the body is turning, in radians a second, positive toward its
    /// own left.
    ///
    /// A yaw **rate** is per second and a stride is per stance, so the cadence
    /// joins them — the body's own, recovered from the stride it is walking,
    /// rather than named beside it. A turn asserted independently of the legs
    /// is a turn the feet are not taking.
    ///
    /// **Read whether or not the body is travelling** (#361): eased through
    /// [`DriverConfig::pace_response`], it sets the gait, the cadence and the
    /// stride's yaw through [`Turn`], so no stance carries the body round
    /// further than a hip turns ([`super::turn::TURN_REACH`]), and a body under
    /// [`DriverConfig::idle_below`] turning past [`PIVOT_FROM`] steps round
    /// where it stands rather than idling. A caller holding planted feet to
    /// the world ([`Carriage::Chassis`]) keeps them still through the part of
    /// a turn this reports. Zero is a body going straight, exactly as before,
    /// and a turn that ends hands that body back to the bit once its ease has
    /// landed.
    pub turn: f32,
    /// Hold the lids at this point of a blink, `0` open and `1` shut, instead of
    /// blinking.
    ///
    /// A blink is stochastic, so a single captured frame almost never catches
    /// one and a still cannot show what the lids do without this.
    pub lids: Option<f32>,
    /// The face the body is resting in, which decides where the lids sit when
    /// they are not mid-blink.
    ///
    /// The blink phase runs **through** this rather than beside it, because
    /// adding a widened rest to a full blink leaves an eye that never shuts.
    pub face: Expression,
    /// The body's eyes, if it has any, so the closure can be applied to the
    /// pose here rather than by every caller in turn.
    ///
    /// A blink is a pose — the four lids are joints — and it rides **after**
    /// the blend: about a tenth of a second from open to shut and back,
    /// smoothed by a gait transition, arrives as a slow heavy-lidded droop that
    /// reads as a body falling asleep. Keeping that ordering here is the point
    /// of taking the eyes at all.
    pub eyes: Option<&'a Eyes>,
    /// A layer laid over the locomotion, before the contacts are settled.
    ///
    /// Where authored motion goes: a clip whose joints ride the procedural
    /// walk. It runs after the gesture for the same reason the gesture runs
    /// after the gait — each writes only what it addresses, over whatever the
    /// last one left.
    pub over_locomotion: Option<Layer<'a>>,
    /// A layer laid over the settled body, before the blend.
    ///
    /// Where a face and a deliberate gaze go. **After the settle**, so it
    /// cannot fight the footing solve, and **before the blend**, so a
    /// transition corrects what these produced rather than being overwritten by
    /// them.
    pub over_settled: Option<Layer<'a>>,
}

impl Default for Inputs<'_> {
    /// A body standing still, unheld, with nothing overridden.
    fn default() -> Self {
        Self {
            delta: 0.0,
            velocity: Vec3::ZERO,
            at: Vec3::ZERO,
            facing: 0.0,
            heading: None,
            swimming: false,
            gesture: None,
            hold: Hold::None,
            showing: None,
            cycle: None,
            cadence: None,
            gait: None,
            pattern: None,
            walk: WalkFlags::default(),
            turn: 0.0,
            lids: None,
            face: Expression::NEUTRAL,
            eyes: None,
            over_locomotion: None,
            over_settled: None,
        }
    }
}

impl std::fmt::Debug for Inputs<'_> {
    /// Everything but the two overlay closures, which have nothing to print.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inputs")
            .field("delta", &self.delta)
            .field("velocity", &self.velocity)
            .field("at", &self.at)
            .field("facing", &self.facing)
            .field("heading", &self.heading)
            .field("swimming", &self.swimming)
            .field("gesture", &self.gesture)
            .field("hold", &self.hold)
            .field("showing", &self.showing)
            .field("cycle", &self.cycle)
            .field("cadence", &self.cadence)
            .field("gait", &self.gait)
            .field("pattern", &self.pattern)
            .field("walk", &self.walk)
            .field("turn", &self.turn)
            .field("lids", &self.lids)
            .field("face", &self.face)
            .field("eyes", &self.eyes.is_some())
            .field("over_locomotion", &self.over_locomotion.is_some())
            .field("over_settled", &self.over_settled.is_some())
            .finish()
    }
}

/// What one frame of driving produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Driven {
    /// The pose to draw the body in, blend and blink included.
    pub pose: Pose,
    /// How shut the lids are, `0` open and `1` shut — already applied to
    /// [`Self::pose`], and handed back for anything that wants to record it.
    pub closure: f32,
    /// What carried the body this frame.
    pub source: Source,
    /// Whether any contact's goal was out of the solver's reach.
    ///
    /// One flag rather than a list, because what a caller does with it is count
    /// frames: a body that strains occasionally is a body on hard ground, and
    /// one that strains constantly is a body whose goals are wrong. Exactly
    /// [`Self::straining`] above zero.
    pub strained: bool,
    /// How many contacts could not reach their goal this frame, each counted
    /// once whichever stage found it — the stride's own solve, the footing
    /// solve that settles it, or a leap's.
    ///
    /// **The count beside the flag** (#334): a body with one leg reaching for a
    /// hole and a body with both legs out of reach are the same `true`, and
    /// they are not the same body.
    pub straining: usize,
    /// How far the footing solve had to move any one joint of a contact this
    /// frame, in metres — [`super::gait::Walked::lift`], as the walk's own
    /// settle reported it.
    ///
    /// The readout a locomotion question is settled on rather than on taste: a
    /// pose whose feet already land where the ground is needs no correction,
    /// and one whose do not is being held together by the solve. Zero on a
    /// frame the walk did not carry — the idle plants its own feet, a leap
    /// places its own contacts and a swimming body has none — because no
    /// footing solve ran there to cost anything.
    pub lift: f32,
    /// Whether a gesture aimed the head this frame.
    ///
    /// A caller with a gaze layer of its own stands aside when this is set: a
    /// nod is a gesture the body is making on purpose, and a target written
    /// over it arrives correct and is put back level a moment later.
    pub aimed: bool,
    /// What the idle reported, if the idle ran.
    pub idled: Option<Idled>,
}

/// Adds each of `limbs` to `into` that is not already there — one frame's
/// straining contacts, counted once however many stages found them.
fn count_straining(into: &mut Vec<Limb>, limbs: &[Limb]) {
    for &limb in limbs {
        if !into.contains(&limb) {
            into.push(limb);
        }
    }
}

/// A gesture in progress on one body.
#[derive(Clone, Debug, PartialEq)]
struct ActiveGesture {
    /// Which gesture is playing, by roster name. The clip itself is rebuilt
    /// each frame — a goal-space clip is a few keyed goals, so rebuilding costs
    /// less than the solve that follows it and keeps this a value rather than a
    /// cache.
    name: String,
    /// How long it has played, in seconds. A gesture plays **once**: this
    /// counts up to [`DriverConfig::gesture_secs`] and then clears, where a
    /// locomotion cycle wraps - but a beast's state, which counts for as long
    /// as the state lasts, going down, held and getting up (#377).
    elapsed: f32,
    /// Where a beast's state has got to, if this is one (#377).
    resting: Option<Resting>,
}

/// Where a beast's state - a sit or a lying down - has got to (#377).
///
/// It goes down at its own pace ([`super::emote::seconds`]) and is held until
/// something gets the beast up: moving, leaving the ground, swimming, another
/// gesture, or [`super::emote::GET_UP`]. Getting up from the held state is the
/// animal's own order ([`super::emote::getting_up`]); a beast interrupted on
/// the way down comes back up the way it was going. Either way from wherever
/// the body is, so nothing snaps.
#[derive(Clone, Debug, PartialEq)]
struct Resting {
    phase: Phase,
    /// How far through the phase's clip, `0..1`: the way down's for
    /// [`Phase::Down`], [`Phase::Held`] and [`Phase::Back`], the getting up's
    /// for [`Phase::Up`].
    through: f32,
    /// Getting up in a hurry, at [`super::emote::HURRY`] of the pace: the body
    /// is moving, off the ground or in the water.
    hurried: bool,
    /// The gesture asked for while the beast was down, played once it is up:
    /// the last one asked for.
    then: Option<String>,
}

/// What a beast's state is doing (#377).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Going down into it.
    Down,
    /// Down, until something gets it up.
    Held,
    /// Getting up out of it, as the animal does.
    Up,
    /// Coming back up the way it was going down, stopped part way.
    Back,
}

impl Resting {
    /// Gets the beast up from wherever it has got to: from the held state as
    /// the animal rises, from part way down the way it came.
    fn get_up(&mut self, hurried: bool) {
        self.hurried |= hurried;
        match self.phase {
            Phase::Down => self.phase = Phase::Back,
            Phase::Held => {
                self.phase = Phase::Up;
                self.through = 0.0;
            }
            Phase::Up | Phase::Back => {}
        }
    }

    /// Moves it on by `delta` seconds of the state `name` on `rig`, and
    /// answers whether the beast is up.
    fn advance(&mut self, rig: &Rig, name: &str, delta: f32) -> bool {
        let (Some(down), Some((_, up))) = (
            super::emote::seconds(rig, name),
            super::emote::getting_up(rig, name),
        ) else {
            return true;
        };
        let pace = if self.hurried {
            super::emote::HURRY
        } else {
            1.0
        };
        let share = |seconds: f32| delta / (seconds * pace).max(f32::EPSILON);
        match self.phase {
            Phase::Down => {
                self.through += share(down);
                if self.through >= 1.0 {
                    self.through = 1.0;
                    self.phase = Phase::Held;
                }
                false
            }
            Phase::Held => false,
            Phase::Up => {
                self.through += share(up);
                self.through >= 1.0
            }
            Phase::Back => {
                self.through -= share(down);
                self.through <= 0.0
            }
        }
    }
}

/// How fast a body may be carried and still be resting, in metres a second
/// (#377): a smoothed transform settles slower than this, and a body carried
/// any faster is being moved, which gets a beast up out of a sit.
/// Provenance: **chosen** (#377), a tenth of a metre a second.
const STIR: f32 = 0.1;

/// One body's motion, and every clock that outlives a frame.
///
/// One per body, and seeded per body: the idle's seed decides when its settling
/// weight shift fires and which leg it moves first, so a room full of bodies
/// sharing a seed breathes, shifts and blinks in unison and reads as a drill
/// team rather than a crowd. There is deliberately no [`Default`] — a seed
/// drawn from somewhere the driver cannot see is how a measurement becomes a
/// function of how many bodies were made before it.
#[derive(Clone, Debug)]
pub struct Driver {
    config: DriverConfig,
    /// Where in the locomotion cycle the body is, `0..1`.
    cycle: f32,
    /// What carried the body last frame. A change starts a blend.
    source: Source,
    /// The speed the gait was built from last frame, or [`None`] when the body
    /// was not travelling.
    ///
    /// Kept so [`transition::carry_cycle`] has a gait to map out of. A
    /// [`Speed`] is one dimensionless number, so remembering the speed rather
    /// than the gait keeps this a value and rebuilds the gait only on the
    /// frames a change of duty actually needs one.
    gaiting: Option<Speed>,
    /// The eased pace the gait was last fed, in m/s. [`None`] whenever the gait
    /// is not carrying the body — the next walk eases from its own first frame
    /// rather than from a stale speed.
    paced: Option<f32>,
    /// The eased yaw rate, in radians a second, positive toward the body's
    /// left — [`Inputs::turn`] through the same lag as the pace (#361), so a
    /// turn arriving all at once reaches the stride, the cadence and the bank
    /// as a body would carry it out.
    turning: f32,
    /// Whether the gait last frame was a pivot (#361) — kept so a change
    /// between stepping round and walking off blends.
    pivoted: bool,
    /// The leap in progress, or [`None`] while the body is on the ground.
    airborne: Option<Airborne>,
    /// The frame before last, as drawn — the velocity a transition carries
    /// through.
    previous: Option<Pose>,
    /// Last frame, as drawn.
    current: Option<Pose>,
    /// The transition in progress, if any.
    transition: Option<Inertializer>,
    /// The blink timer.
    blink: crate::face::Blink,
    /// The idle driver: breath, sway, weight shift and fidget scheduling.
    /// Stateful, and paused rather than advanced while something else carries
    /// the body, so a body that stops walking resumes its own idle rather than
    /// a reset one.
    idler: Idle,
    /// The world plant-point ledger.
    footholds: Footholds,
    /// The gesture riding over the locomotion, if any.
    gesture: Option<ActiveGesture>,
    /// When the last gesture **started**, on [`Self::elapsed`]'s clock. Kept
    /// across the gesture ending, which is the whole point: the cooldown
    /// outlives what it is limiting.
    gestured_at: Option<f32>,
    /// How long this driver has been running, in seconds. Its own clock rather
    /// than the caller's, so a driver is comparable between an application with
    /// a wall clock and an instrument stepping a fixed delta.
    elapsed: f32,
    /// Whether the caller has asked the next driven frame to blend in, through
    /// [`Self::blend_now`].
    blend_asked: bool,
    /// Whether this frame's airborne step came down onto the ground (#414),
    /// where what the trunk rides takes the height it comes down on at once.
    touched_down: bool,
    /// The pitch the flight came down in, which the landing sets off from
    /// (#414): see [`Leap::drive`]. Kept for the landing it was taken for, and
    /// forgotten once the body is no longer in the air or landing.
    landing_from: Option<f32>,
    /// The body's spring chains - a beast's ears (#370) - and where each
    /// joint of them was. [`None`] until the first frame finds them on the
    /// rig it is given.
    springs: Option<Springs>,
    /// What a beast's rests solve from its rig alone, kept while the rig is
    /// the same (#377): a held sit costs what an idle frame does.
    rests: super::rest::Rests,
    /// The ground a four-legged body's trunk rode on the last frame it stood
    /// on any (#413): what a leap's flight rides, and what the water is gone
    /// into from. `None` on the level and for any other body.
    grounded: Option<super::quadruped::Girdles>,
    /// The ground under a four-legged body's girdles its trunk rode the last
    /// frame (#413), as it follows what is read there: `None` before the first,
    /// after a warp on the ground, a hold at rest ([`Hold::Rest`]; a held pose
    /// keeps it) or a change of body, and in the water, where it takes what it
    /// reads.
    ridden: Option<Option<super::quadruped::Girdles>>,
    /// Where the girdles of the body last driven are read (#413): see
    /// `quadruped::girdle_spacing`. What was ridden and stood on is forgotten
    /// when this changes.
    girdled: Option<[f32; 4]>,
}

/// How far one paddle carries a swimming beast, in its leg's reach (#370).
/// Provenance: **the literature, rounded**: a dog swimming at about a metre a
/// second paddles about twice a second on a leg of a third of a metre.
const PADDLE_STRIDE: f32 = 1.5;

/// How a beast's ears follow its head (#370): far stiffer than the hair
/// [`SpringConfig::default`] is tuned for, and damped to settle in a few
/// tenths of a second, so an ear moves off its pose and back with the gait
/// rather than streaming behind it. The head it hangs from is held steady, so
/// there is little to follow: the tip of the default dog's 69 mm ear moves
/// 2 mm at a walk and a trot and 4 to 5 mm at a canter and a gallop, two to
/// four degrees. No gravity, because an ear is built in its own carriage:
/// pricked, folded or hanging.
/// Provenance: **tuned by test** (#370), and only just: the ear test asks a
/// trotting ear to move more than 2 mm off its pose and a standing one to
/// settle home, and a trotting ear moves 2.4.
const EARS: SpringConfig = SpringConfig {
    stiffness: 1000.0,
    drag: 20.0,
    gravity: Vec3::ZERO,
    clearance: 0.01,
};

impl Driver {
    /// A driver whose idle and blink are seeded from `seed`.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self {
            config: DriverConfig::default(),
            cycle: 0.0,
            source: Source::Rest,
            gaiting: None,
            paced: None,
            turning: 0.0,
            pivoted: false,
            airborne: None,
            previous: None,
            current: None,
            transition: None,
            blink: crate::face::Blink::seeded(seed),
            idler: Idle::seeded(seed),
            footholds: Footholds::new(),
            gesture: None,
            gestured_at: None,
            elapsed: 0.0,
            blend_asked: false,
            touched_down: false,
            landing_from: None,
            springs: None,
            rests: super::rest::Rests::default(),
            grounded: None,
            ridden: None,
            girdled: None,
        }
    }

    /// The same, with a configuration that is not the default.
    #[must_use]
    pub fn new(config: DriverConfig, seed: u64) -> Self {
        Self {
            config,
            ..Self::seeded(seed)
        }
    }

    /// What this driver is tuned to.
    #[must_use]
    pub fn config(&self) -> DriverConfig {
        self.config
    }

    /// Re-tune it. Every field takes effect on the next frame.
    pub fn set_config(&mut self, config: DriverConfig) {
        self.config = config;
    }

    /// Which idle a standing body holds — the ordinary one, or the stiller one
    /// a body keeps while somebody else is talking.
    pub fn set_idle_config(&mut self, config: IdleConfig) {
        self.idler.set_config(config);
    }

    /// What carried the body on the last frame that was driven.
    #[must_use]
    pub fn source(&self) -> Source {
        self.source
    }

    /// Where in the locomotion cycle the body is, `0..1`.
    #[must_use]
    pub fn cycle(&self) -> f32 {
        self.cycle
    }

    /// The speed the gait was last built from, or [`None`] if the body was not
    /// travelling.
    ///
    /// **The speed the driver actually used**, which is the eased one — an
    /// instrument deriving the expected gait state from the caller's raw speed
    /// crosses the walk-run duty step on a different frame than the body did.
    #[must_use]
    pub fn speed(&self) -> Option<Speed> {
        self.gaiting
    }

    /// The turn the stride was last built from, in radians a second positive
    /// toward the body's left: [`Inputs::turn`] eased (#361), and exactly zero
    /// once a turn has ended.
    #[must_use]
    pub fn turning(&self) -> f32 {
        self.turning
    }

    /// The leap in progress, or [`None`] while the body is on the ground.
    #[must_use]
    pub fn airborne(&self) -> Option<Airborne> {
        self.airborne
    }

    /// The gesture playing and how long it has played, in seconds: up to
    /// [`DriverConfig::gesture_secs`], or for a beast's state as long as the
    /// state lasts, going down, held and getting up (#377).
    #[must_use]
    pub fn gesture(&self) -> Option<(&str, f32)> {
        self.gesture
            .as_ref()
            .map(|active| (active.name.as_str(), active.elapsed))
    }

    /// Whether a frame of these `inputs` is moving the body as a beast's state
    /// counts it (#377): carried faster than a tenth of a metre a second, shown
    /// swimming or leaping, or off the ground. Any of those gets a beast up out
    /// of a sit or a lying down and keeps it from going into one, and this is
    /// what [`Self::drive`] asks, so a caller that has to know beforehand reads
    /// the same answer. Only [`Inputs::velocity`], [`Inputs::showing`] and
    /// [`Inputs::swimming`] are read. Asked before a frame, it does not yet see
    /// a leap that frame's own vertical speed will start.
    #[must_use]
    pub fn stirred(&self, inputs: &Inputs<'_>) -> bool {
        Vec2::new(inputs.velocity.x, inputs.velocity.z).length() >= STIR
            || self.airborne.is_some()
            || inputs.swimming
            || matches!(inputs.showing, Some(Showing::Leap(_) | Showing::Swim(_)))
    }

    /// Puts the body at a point of the state `name` on `rig`, as one step (#383):
    /// `through` of the way down, the share [`super::emote::seconds`] runs it in;
    /// held from one up; and with nothing or less, out of it and standing.
    ///
    /// **As if it had gone down that far**, so what comes next is what would
    /// have: a body seated part way goes on down and comes back up the way it
    /// came, and one held gets up as the animal does. Whatever gesture the body
    /// was playing, and whatever it had been asked for after, is gone.
    ///
    /// **It draws there on the next frame**, with no blend from what it last
    /// drew: a driver that has drawn nothing has nothing to blend from, and one
    /// that has can be told to with [`Self::blend_now`].
    ///
    /// Answers whether `name` is a state of this body; a name that is not
    /// changes nothing. No clock runs, so a fresh body brought down in a rebuild
    /// costs nothing, and it spends no [`DriverConfig::gesture_cooldown`].
    pub fn seat(&mut self, rig: &Rig, name: &str, through: f32) -> bool {
        let Some(down) = super::emote::seconds(rig, name) else {
            return false;
        };
        self.transition = None;
        self.blend_asked = false;
        // Put where it is with no way down: no plane to go on from (#400).
        self.rests.unlay();
        if through <= 0.0 {
            self.gesture = None;
            return true;
        }
        let through = through.min(1.0);
        self.gesture = Some(ActiveGesture {
            name: name.to_string(),
            elapsed: through * down,
            resting: Some(Resting {
                phase: if through >= 1.0 {
                    Phase::Held
                } else {
                    Phase::Down
                },
                through,
                hurried: false,
                then: None,
            }),
        });
        self.source = Source::Gesture;
        true
    }

    /// The pose the body was last drawn in, if it has been driven.
    #[must_use]
    pub fn posed(&self) -> Option<&Pose> {
        self.current.as_ref()
    }

    /// Forget where the feet were planted and what speed the body was carrying.
    ///
    /// For a caller that knows the body was just moved somewhere else. An
    /// unnoticed warp self-heals — a hold further from the stride's own answer
    /// than the leg could reach is re-planted rather than lunged for — but
    /// healing costs a frame of a foot answering for a point that no longer
    /// makes sense. Most warps need no call at all: anything that stops the
    /// body passes through a frame the gait is not carrying it, and the ledger
    /// clears there.
    ///
    /// The ears' springs start again from where the pose puts them, rather
    /// than swinging in from where the body was (#370), and a four-legged
    /// trunk rides the ground it is put on at once rather than turning onto it
    /// from where it was (#413). In a leap's flight it lands as it would have,
    /// turning onto the ground it comes down on from the ground it left: taken
    /// at once there, a dog warped in the air turned 25 degrees in a frame as
    /// it landed.
    pub fn warped(&mut self) {
        self.footholds.reset();
        // Not in a leap's flight, which rides what it left and lands from it.
        if self.airborne.is_none_or(|air| air.landed) {
            self.ridden = None;
        }
        self.paced = None;
        self.turning = 0.0;
        if let Some(springs) = &mut self.springs {
            springs.settle();
        }
    }

    /// Blend the next driven frame in from the last one drawn, whatever
    /// carries it (#334).
    ///
    /// The driver starts a transition on its own when what carries the body
    /// changes. A caller whose own layer is what changed — an authored clip
    /// laid over the walk through [`Inputs::over_locomotion`], swapped for
    /// another — has no source to change, so without this the swap snaps. This
    /// starts the same inertialized transition on the caller's say-so, from the
    /// two poses the driver already keeps, over [`DriverConfig::blend`].
    ///
    /// A request, not a state: one call is one transition, taken by the next
    /// frame that is actually driven. A driver that has drawn fewer than two
    /// frames has nothing to blend from and lets the request go.
    pub fn blend_now(&mut self) {
        self.blend_asked = true;
    }

    /// One frame: decide what is carrying the body, run it, and hand back the
    /// pose to draw.
    ///
    /// `ground` answers what is beneath a point **in the frame the body is
    /// posed in**, exactly as [`super::plant_feet_of`] requires — and it is
    /// given to every stage that needs a floor, so the stride cannot seat a
    /// contact on one surface while the plant settles it onto another.
    /// [`level_ground`] is the flat answer.
    ///
    /// [`None`] means nothing was written and no clock advanced: the frame had
    /// no time in it, or the body is held exactly where it stands. The caller's
    /// last pose stays correct in both cases.
    #[allow(clippy::too_many_lines)]
    pub fn drive<F>(&mut self, rig: &Rig, inputs: &Inputs<'_>, ground: F) -> Option<Driven>
    where
        F: Fn(Vec3) -> Option<Ground> + Copy,
    {
        let delta = inputs.delta;
        if delta <= 0.0 {
            return None;
        }
        self.elapsed += delta;
        match inputs.hold {
            Hold::Pose => return None,
            Hold::Rest => return Some(self.hold_at_rest(rig, inputs, delta)),
            Hold::None => {}
        }
        let config = self.config;

        // Horizontal magnitude and the SIGNED vertical: which way a body is
        // going vertically is the whole of the airborne state machine below.
        let planar = Vec2::new(inputs.velocity.x, inputs.velocity.z).length();
        let vertical = inputs.velocity.y;

        // **Water before anything else.** A body in the water is not falling,
        // whatever its vertical speed says — swimming upward can outrun a
        // launch threshold, and a peer's smoothed transform would otherwise
        // trip the leap.
        let swimming = inputs.swimming || matches!(inputs.showing, Some(Showing::Swim(_)));
        if swimming {
            self.airborne = None;
        }
        self.advance_airborne(rig, vertical, delta, swimming);
        let airborne = self.airborne.is_some();
        if !airborne {
            self.landing_from = None;
        }
        // **The ground the trunk rides follows what is read under it** (#413),
        // so a consumer's ground switching source in a frame turns a beast's
        // trunk over a few: see `quadruped::follow`. Every state is handed the
        // ground laid on what is ridden, its feet with its trunk, and where the
        // two are the same, as on the level and over any ground walked on, the
        // ground as it came. Nothing is followed in the water, which floats the
        // trunk level, nor in a leap's flight, which reads nothing below.
        // What was ridden is another body's where the girdles stand elsewhere:
        // a horse's heights carried onto a dog mid-flight flew it 20 cm low,
        // and a beast's onto a person shifted the person's ground 138 mm.
        let spacing = super::quadruped::girdle_spacing(rig);
        if spacing != self.girdled {
            self.girdled = spacing;
            self.grounded = None;
            self.ridden = None;
        }
        let read = super::quadruped::girdle_ground(rig, &ground);
        let flying = self.airborne.is_some_and(|air| !air.landed);
        let shift = if swimming {
            self.ridden = None;
            None
        } else if flying {
            None
        } else {
            let ridden = match self.ridden {
                Some(was) => super::quadruped::follow(rig, was, read, delta, self.touched_down),
                None => read,
            };
            self.ridden = Some(ridden);
            super::quadruped::Shift::between(rig, read, ridden)
        };
        if !airborne && !swimming {
            self.grounded = self.ridden.flatten();
        }
        // The ground as it came, which a landing plants on (#413).
        let under = ground;
        let ground = move |at: Vec3| match shift {
            None => ground(at),
            Some(shift) => ground(at).map(|below| shift.apply(below)),
        };

        // The eased pace: a first-order lag toward the caller's raw speed,
        // alive only while the gait is. The gate below stays on the RAW speed
        // on purpose, so a stop hands over to the idle on the frame the body
        // stops and a gait starts from the crossing speed rather than zero.
        let paced = if planar >= config.idle_below {
            let from = self.paced.unwrap_or(planar);
            let eased = if config.pace_response > 0.0 {
                from + (planar - from) * (1.0 - (-delta / config.pace_response).det_exp())
            } else {
                planar
            };
            self.paced = Some(eased);
            eased
        } else {
            self.paced = None;
            planar
        };
        // On four legs, in the pattern the body was already moving in where
        // the speed still allows it: a body changes down later than it
        // changed up. One breaking into a canter on a curve leads with its
        // inside legs, as a horse is asked to (#370).
        let lead = if self.turning > 0.0 {
            Lead::Left
        } else {
            Lead::Right
        };
        let travelling = (planar >= config.idle_below).then(|| {
            let speed = Speed::new(rig, paced);
            match inputs.pattern.filter(|_| speed.pattern().is_some()) {
                Some(asked) => speed.in_pattern(asked),
                None => speed.held(rig, self.gaiting.and_then(Speed::pattern), lead),
            }
        });
        // How far the body actually travels for the pace its stride is built
        // from — one at a steady pace, and more or less while the pace eases
        // after a change of speed (#362, `stepping`).
        let travel = if travelling.is_some() && paced > f32::EPSILON {
            planar / paced
        } else {
            1.0
        };

        // **The turn, eased as the pace is** (#361): the stride's yaw, the
        // cadence a turn needs and the bank are all functions of it, so it
        // reaches them through the same lag rather than as a step.
        let asked = if inputs.turn.is_finite() {
            inputs.turn
        } else {
            0.0
        };
        let eased = if config.pace_response > 0.0 {
            self.turning
                + (asked - self.turning) * (1.0 - (-delta / config.pace_response).det_exp())
        } else {
            asked
        };
        // **And landed on, not approached for ever.** A lag closes a share of
        // its gap each frame, so on its own it never closes it: the gap falls
        // through the subnormals and stalls a few of them short of zero, and a
        // straight stride is a turn of exactly zero — a body that had ever
        // turned would never again walk the stride everything here was
        // measured on.
        self.turning = if (asked - eased).abs() < TURN_SETTLED {
            asked
        } else {
            eased
        };
        let turn = Turn::new(self.turning);
        // **A body turning where it stands steps round** (#361). The idle poses
        // its feet in the body, so a body yawing over them would carry them
        // round on the floor; past `PIVOT_FROM` it walks a pivot instead — a
        // stride of no length whose stances turn — and keeps walking it until
        // the turn falls to half of that, or the body sets off.
        let pivoting = travelling.is_none()
            && self.turning.abs()
                >= if self.gaiting.is_some() {
                    PIVOT_FROM * 0.5
                } else {
                    PIVOT_FROM
                };
        // **The speed the gait is built from**: the body's own, or none at all
        // for a body stepping round where it stands. `effort` is the speed
        // whose gait the legs walk, which a turn makes the outer foot's
        // (`Turn::working`, #361) — kept to carry a change of gait across and
        // to hand the idle its arrival. A heading changes the stride, not the
        // gait: walking and running is the leg vaulting over its foot, at the
        // speed the body goes whichever way it goes (#362, `stepping`).
        let pacing = travelling.or_else(|| pivoting.then_some(Speed::STILL));
        let effort = pacing.map(|speed| turn.working(rig, speed));

        // **The stroke's clock, which the swim leaves to its caller.** Both
        // ends are derived: at speed one cycle carries the body a fixed number
        // of its own lengths, and at rest a tread sculls at the body's own
        // pendulum frequency — the one rate a body of this size has without
        // anybody choosing it. A giant sculls slower for the same reason it
        // walks slower.
        //
        // **A beast paddles at its legs' pace, not its body's** (#370): a
        // paddle is its legs going round, so the pendulum is the leg and a
        // cycle carries the body [`PADDLE_STRIDE`] of them. Timed by the whole
        // body, as a crawl is, a dog half a metre long paddled at half a beat
        // a second.
        let stroke_rate = || {
            let (length, per_stroke) = if rig.four_legged() {
                (super::speed::leg_of(rig).max(f32::EPSILON), PADDLE_STRIDE)
            } else {
                (rig.extent().max(f32::EPSILON), config.lengths_per_stroke)
            };
            let tread = (GRAVITY / length).sqrt() / std::f32::consts::TAU;
            (planar / (length * per_stroke)).max(tread)
        };

        // A leap owns the legs for as long as it lasts: a body cannot be
        // mid-stride and mid-air at once, and pretending otherwise is how a
        // jump ends up with a walk cycle running underneath it.
        let showing_leap = matches!(inputs.showing, Some(Showing::Leap(_)));
        // The gait, the stride and the cadence the legs would walk this frame,
        // built once: the clock below runs at the cadence, and the walk walks
        // the rest.
        let legs = pacing.map(|speed| Self::walking(rig, inputs, speed, turn, travel));
        let (locomotion, derived_cadence) = if swimming {
            (Source::Swim, stroke_rate())
        } else if showing_leap || airborne {
            (Source::Leap, 0.0)
        } else if let Some((_, _, cadence)) = legs.as_ref() {
            (Source::Gait, *cadence)
        } else if config.idle {
            (Source::Idle, 0.0)
        } else {
            (Source::Rest, 0.0)
        };
        let cadence = inputs.cadence.unwrap_or(derived_cadence);

        // **Hand the idle the stance the body arrived in**, before anything
        // below moves the clock or forgets the speed. Left to itself an idle
        // solves every contact back to its rest position each frame, so a body
        // that stops mid-stride drags the foot that was bearing its weight up
        // to a third of a metre across the ground to close its stance. That is
        // not a transition-timing problem: waiting for a handoff and waiting
        // for a midstance were both measured WORSE than stopping immediately,
        // because a body that holds keeps striding while the world has already
        // stopped.
        //
        // Read before the assignments below: `gaiting` still holds the speed
        // the body was travelling at, `cycle` still reads the moment the last
        // pose was DRAWN at, and `source` still says what the body was doing.
        if self.source == Source::Gait
            && locomotion == Source::Idle
            && let Some(before) = self.gaiting
            && let Some(arrival) = self.current.clone()
        {
            let gait = before.gait(rig);
            let bearing: Vec<_> = gait
                .limbs
                .iter()
                .enumerate()
                .filter(|(index, _)| gait.phase(*index, self.cycle).is_stance())
                .map(|(_, &limb)| limb)
                .collect();
            self.idler.arrive(rig, &arrival, &bearing);
        }

        // **Carry the CYCLE across a change of gait, not the number.** A cycle
        // fraction means a different part of the step at a different duty: the
        // duty falls all the way along the speed axis and STEPS at the walk-run
        // transition, so a body that crosses it mid-stride hands `0.5` to a
        // gait where `0.5` is a different moment, and a foot in mid-swing
        // arrives planted.
        //
        // Asked every frame rather than at a detected boundary, because there
        // is no boundary to detect: the duty moves continuously as well as
        // stepping, and where it has not moved the map is the identity to the
        // bit.
        let last = self.cycle;
        // A four-legged body changing its pattern - a walk breaking into a
        // trot, a canter into a gallop - moves its feet to where the new
        // pattern has them, and the blend below takes them there (#370).
        let repatterned = matches!(
            (self.gaiting.and_then(Speed::pattern), effort.and_then(Speed::pattern)),
            (Some(before), Some(now)) if before != now
        );
        if let (Some(speed), Some(before)) = (effort, self.gaiting)
            && before != speed
        {
            let into = speed.gait(rig);
            let from = Gait {
                duty: before.duty(),
                ..into.clone()
            };
            self.cycle = transition::carry_cycle(&from, &into, self.cycle);
        }
        self.gaiting = effort;

        // How far the clock steps, for the footholds' account of a stride's
        // turn (#361): the step itself, which a carried cycle is not — the
        // carry keeps the moment of the step and turns no stance. A clock the
        // caller holds is read off its two values, forward and the short way
        // round.
        let advanced = if let Some(held) = inputs.cycle {
            self.cycle = held;
            ((held - last + 0.5).rem_euclid(1.0) - 0.5).max(0.0)
        } else if !airborne || swimming {
            self.cycle = (self.cycle + delta * cadence).fract();
            delta * cadence
        } else {
            0.0
        };

        // **A beast's state is held until something gets it up** (#377):
        // the body being moved, leaving the ground or swimming gets it up in a
        // hurry, from wherever it is.
        let stirred = self.stirred(inputs);
        self.start_gesture(rig, inputs.gesture, stirred);
        // Advanced and retired before the pose is built, so a gesture that
        // finished this frame does not get one last frame of overlay.
        let mut up = None;
        if let Some(active) = self.gesture.as_mut() {
            active.elapsed += delta;
            let done = match active.resting.as_mut() {
                None => active.elapsed >= config.gesture_secs,
                Some(resting) => {
                    if stirred {
                        resting.get_up(true);
                    }
                    let done = resting.advance(rig, &active.name, delta);
                    if done {
                        up = Some(resting.then.take());
                    }
                    done
                }
            };
            if done {
                self.gesture = None;
            }
        }
        // Up, and what it was asked for while down: the rise's last pose and
        // the next gesture's first are two poses of one source, so the switch
        // is blended as a change of source is.
        if let Some(Some(name)) = up
            && self.begin_gesture(rig, &name, stirred, false)
        {
            self.blend_asked = true;
        }
        // A gesture is its own source, so ending one blends back into the walk
        // it was riding rather than snapping.
        let source = if self.gesture.is_some() {
            Source::Gesture
        } else {
            locomotion
        };

        let mut pose = Pose::rest(rig);
        let mut steps = Steps::default();
        // Every contact some stage could not get to its goal, each once: the
        // stride's own solve, a leap's, and the footing solve that settles the
        // walk all report the same kind of failure, and a leg found out of
        // reach twice in one frame is still one leg (#334).
        let mut straining: Vec<Limb> = Vec::new();
        let mut lift = 0.0f32;
        // Kept past the match so the contacts can be settled and the ankles
        // rolled AFTER whatever is laid over the locomotion: the plant lays
        // every sole flat and a roll applied before it is simply levelled away.
        let mut walking = None;
        let mut idled = None;

        // The ledger lives only while the gait does: the idle shifts weight and
        // fidgets feet on its own authority, so a hold surviving an idle would
        // serve a point the body has since walked away from. A held stance
        // re-plants on stance entry, so clearing here costs a resumed walk
        // nothing — and it is the teleport reset as well, since anything that
        // stops a body passes through a frame like this one.
        // **And at a change of pattern** (#412): held through it, a foot
        // planted in the walk was let go the frame after, once the trot had
        // it elsewhere, a dog's hind paw 0.98 times what a frame allows in that
        // one frame at 144 a second, past the blend.
        if locomotion != Source::Gait || repatterned {
            self.footholds.reset();
        }
        match locomotion {
            // A beast with no idle stands in its rest pose, and rides its
            // girdles there as every other state does (#413): stood level, a
            // rest set off from the ridden trunk turned it 5.4 degrees a frame.
            // And its feet are stood on the ground under them, as the idle
            // stands them: turned with the trunk alone, a dog's paws sank 120
            // mm into a 25 degree slope and a horse's 242.
            Source::Rest => {
                if let Some(girdles) = super::quadruped::girdle_ground(rig, &ground) {
                    super::quadruped::ride_ground(rig, &mut pose, girdles);
                    super::quadruped::stand_on(rig, &mut pose, girdles, ground);
                }
            }
            Source::Gesture => {}
            Source::Idle => {
                // The whole layer through one call, which advances the schedule
                // and poses every layer together: a stage a caller has to
                // remember is a stage a caller forgets.
                idled = Some(self.idler.drive_on(rig, &mut pose, delta, ground));
            }
            Source::Gait => {
                let (gait, stride, _) = legs.unwrap_or_else(|| {
                    Self::walking(rig, inputs, pacing.unwrap_or(Speed::STILL), turn, travel)
                });
                let walked =
                    self.walk_frame(rig, inputs, &gait, &stride, advanced, &mut pose, ground);
                count_straining(&mut straining, &walked.steps.straining);
                steps = walked.steps;
                walking = Some((gait, stride));
            }
            Source::Swim => {
                // Nothing is added to the stance list: a swimming body has
                // nothing on the ground, and handing the footing tail a contact
                // is what would drag its feet back down to a floor it is
                // nowhere near.
                let swim = match inputs.showing {
                    Some(Showing::Swim(swim)) => Swim {
                        cycle: self.cycle,
                        ..swim
                    },
                    _ => Swim::at(self.cycle).toward(planar),
                };
                swim.drive(rig, &mut pose);
            }
            Source::Leap => {
                // A leap shown sets off from nothing the body really did: the
                // pitch a real flight came down in is that landing's alone.
                let leaping = match inputs.showing {
                    Some(Showing::Leap(leap)) => {
                        Some((leap, self.cycle * leap.duration(rig), None))
                    }
                    _ => self.airborne.map(|air| {
                        (
                            air.leap,
                            air.elapsed,
                            self.landing_from.filter(|_| air.landed),
                        )
                    }),
                };
                if let Some((leap, elapsed, landing_from)) = leaping {
                    let leapt = leap.drive_riding(
                        rig,
                        &mut pose,
                        elapsed,
                        ground,
                        under,
                        Some(super::leap::Riding {
                            took_off: self.grounded,
                            landing_from,
                        }),
                    );
                    count_straining(&mut straining, &leapt.straining);
                    if leapt.stage.is_grounded() {
                        steps.stance = rig.ground_contacts();
                    } else if config.carriage == Carriage::Chassis {
                        // The flight's height is given back: something else is
                        // already carrying the body along the parabola, and
                        // applying both flies it twice. Taken off after the
                        // drive rather than by not asking for it, because the
                        // drive plants its contacts at the height it applied.
                        pose.translation.y -= leapt.height;
                    }
                }
            }
        }

        // The gesture rides over whatever the body is already doing. A
        // goal-space clip writes only the parts its own tracks address — a wave
        // is a hand, a nod is a gaze — so the legs and the pelvis keep the walk
        // or the idle that carries them, and there is no joint mask to
        // maintain: the clip's own vocabulary is the mask.
        let mut aimed = false;
        let playing = self
            .gesture
            .as_ref()
            .and_then(|active| match &active.resting {
                None => gesture::for_body(rig, &active.name)
                    .map(|clip| (clip, active.elapsed / config.gesture_secs)),
                Some(resting) if resting.phase == Phase::Up => {
                    super::emote::getting_up(rig, &active.name)
                        .map(|(clip, _)| (clip, resting.through))
                }
                Some(resting) => {
                    gesture::for_body(rig, &active.name).map(|clip| (clip, resting.through))
                }
            });
        if let Some((clip, time)) = playing {
            clip.apply_resting(rig, &mut pose, time, &mut self.rests);
            self.rests.on_ground(rig, &mut pose, ground, delta);
            aimed = clip.tracks.iter().any(|track| track.target == Target::Gaze);
        } else {
            self.rests.unlay();
        }
        if let Some(over) = inputs.over_locomotion {
            over(rig, &mut pose);
        }

        // Defensive, and the invariant is the one the airborne state exists
        // for: nothing is planted while the body is off the ground. The source
        // pick above cannot reach it today — a swimming body has already
        // cleared its leap — but it costs a branch and states the rule.
        if airborne && locomotion != Source::Leap {
            steps.stance.clear();
        }
        // The tail of the drive: settle the contacts, then roll the ankles, in
        // that order. Gait only — the idle plants its own feet, a resting body
        // has nothing down, and a leap places its own contacts. **Nor under a
        // beast in a state** (#377), which rests its whole body on the ground
        // itself: settled after it, a dog walked off while seated dragged its
        // pads 115 mm through the ground.
        let resting = self
            .gesture
            .as_ref()
            .is_some_and(|active| active.resting.is_some());
        if let Some((gait, stride)) = walking.as_ref().filter(|_| !resting) {
            let walked = self
                .walk_at(inputs, inputs.walk.footing)
                .settle(rig, &mut pose, gait, stride, &steps, ground);
            if let Some(footing) = &walked.footing {
                count_straining(&mut straining, &footing.straining);
            }
            lift = walked.lift;
        }

        // **The idle's glance, and only where nothing is aiming the head on
        // purpose.** A gesture that aims the head owns it for as long as it
        // runs, and asking the clip rather than keeping a list of which
        // gestures involve the head is what keeps that true as the roster
        // grows.
        if !aimed
            && let Some(idled) = idled
            && let Some(target) = idled.glance
        {
            // The pose either side of the turn, rather than a share of the turn:
            // `look_at`'s last joint always completes what is asked of it.
            let before = (idled.glance_share < 1.0).then(|| pose.rotations.clone());
            look_at(rig, &mut pose, target, &super::idle::glance_config());
            if let Some(before) = before {
                for (turned, was) in pose.rotations.iter_mut().zip(before) {
                    *turned = det::slerp(was, *turned, idled.glance_share);
                }
            }
        }
        if let Some(over) = inputs.over_settled {
            over(rig, &mut pose);
        }

        // Inertialize source switches so a walk does not snap into a stand —
        // but only where the two are different activities. Within locomotion
        // the answer is no: see `Source::family`. Or wherever the caller asked
        // for one, because what changed was theirs (#334); the request is taken
        // here whether or not there is anything to blend from.
        //
        // **And between a pivot and a walk** (#361), the one change inside
        // locomotion where the stride itself jumps: a pivot's has no length
        // and a walk's starts at the crossing speed's, a third of a metre, so
        // setting off from a turn moves every contact at once, as setting off
        // from the idle does — measured, a planted foot 70 mm in one frame
        // walking off ahead and 155 mm into a strafe, where the idle's blend
        // holds the same start to 41.
        let asked = std::mem::take(&mut self.blend_asked);
        let pivot_edge =
            self.source == Source::Gait && source == Source::Gait && self.pivoted != pivoting;
        self.pivoted = pivoting;
        if config.blend > 0.0
            && (asked || pivot_edge || repatterned || self.source.needs_blend(source))
            && let (Some(previous), Some(current)) = (&self.previous, &self.current)
        {
            // **Into and out of the water slower, on a slope** (#413): a
            // swimming body floats level and one on the ground rides its
            // girdles, so the change turns the whole trunk by the slope; over
            // the one blend a dog on a 25 degree slope turned it 6 degrees a
            // frame. Longer by as much as the ground under it is steep; on the
            // level, the blend it always was. The steeper of the ground now
            // and the last stood on: a consumer may answer nothing under a
            // body the frame it begins to swim, as Overlands answers a swimmer.
            let swum = (source == Source::Swim) != (self.source == Source::Swim);
            let steep = if swum {
                let read = |girdles: Option<super::quadruped::Girdles>| {
                    girdles.map_or(0.0, super::quadruped::Girdles::steep)
                };
                read(super::quadruped::girdle_ground(rig, &ground)).max(read(self.grounded))
            } else {
                0.0
            };
            let blend = if steep > 0.0 {
                config.blend * (1.0 + SWIM_TURN * steep)
            } else {
                config.blend
            };
            // **And a change of pattern over a longer one** (#412): a walk's
            // four footfalls stand a quarter of a cycle off a trot's two, so
            // the change carries a foot planted in the walk into the trot's
            // swing, and over the one blend a horse's hind paw went 1.31
            // times what a frame allows on the level.
            let blend = if repatterned {
                blend * REPATTERN_BLEND
            } else {
                blend
            };
            self.transition = Some(Inertializer::start(previous, current, &pose, delta, blend));
        }
        self.source = source;
        let mut posed = match &mut self.transition {
            Some(transition) if !transition.finished() => {
                transition.advance(delta);
                transition.apply(&pose)
            }
            _ => {
                self.transition = None;
                pose.clone()
            }
        };
        self.previous = self.current.take();
        self.current = Some(posed.clone());

        // The ears follow late (#370): their spring chains are stepped over
        // the pose as drawn, so they bob with the gait's own motion. In the
        // pose's space rather than the world's, because a chassis that changes
        // speed or heading in a few frames would flop any ear soft enough to
        // bob at a trot. **A beast's ears and nothing else**: the stiffness is
        // an ear's, and a chain a consumer hangs off a body is the consumer's
        // to step, so a body without a beast's ears - a person, whatever hangs
        // off them - is left exactly as it was.
        let mut springs = match self.springs.take() {
            Some(springs) if springs.fits(rig) => springs,
            _ => {
                let ears: Vec<usize> = super::quadruped::ear_roots(rig).collect();
                Springs::of(rig).only(|anchor| ears.contains(&anchor))
            }
        };
        springs.advance(rig, &mut posed, None, delta, &EARS);
        self.springs = Some(springs);

        // The blink rides AFTER the blend — see `Inputs::eyes`.
        let closure = self.closure(inputs, delta);
        if let Some(eyes) = inputs.eyes {
            eyes.blink(&mut posed, closure);
        }
        Some(Driven {
            pose: posed,
            closure,
            source,
            strained: !straining.is_empty(),
            straining: straining.len(),
            lift,
            aimed,
            idled,
        })
    }

    /// The gait and the stride a body at `pace` walks, toward the heading and
    /// turning as the inputs say, and the cadence it walks them at.
    ///
    /// **One number in, everything out.** The gait pattern, its duty, the
    /// stride length, the foot lift and the cadence all come from how fast the
    /// body is travelling, expressed as a Froude number so the same relation
    /// fits a child, a giant and a quadruped — with the turn's working contact
    /// over it where the body turns (#361). The stride is then taken toward
    /// the heading, which shortens it by the reach that way; [`Self::stepping`]
    /// is what makes the shorter stride cover the same ground — `travel` of
    /// it, the body's own travel over the pace's. A body handed no heading
    /// walks the stride as built, stepped the same way (#363).
    fn walking(
        rig: &Rig,
        inputs: &Inputs<'_>,
        pace: Speed,
        turn: Turn,
        travel: f32,
    ) -> (Gait, Stride, f32) {
        let gait = inputs.gait.cloned().unwrap_or_else(|| turn.gait(rig, pace));
        // A straight stride is the speed axis's own, to the bit: everything
        // this crate measured was measured on it.
        let built = if turn.is_straight() {
            pace.stride(rig)
        } else {
            turn.stride(rig, pace)
        };
        let cadence = turn.cadence(rig, pace);
        // No heading is straight ahead on the stride exactly as built, and it
        // steps the ground the body covers all the same (#363).
        let (heading, mut stride) = match inputs.heading {
            Some(heading) => (heading, built.toward(rig, heading)),
            None => (Heading::FORWARD, built),
        };
        let cadence = Self::stepping(
            rig,
            heading,
            turn,
            &gait,
            (&built, &mut stride),
            cadence,
            travel,
        );
        (gait, stride, cadence)
    }

    /// How many cycles a second a body steps `stride` — `built` taken toward
    /// `heading` — when `built` is stepped at `cadence` and the body travels
    /// `travel` of the pace `built` was built from, and `stride`'s turn
    /// re-stated for that cadence.
    ///
    /// `cadence` is [`Turn::cadence`], which is the speed axis's own for a
    /// body going straight — **and this is faster wherever a heading's stride
    /// is shorter than the travel** (#362, playbook §2.7). A sideways or
    /// backward step is shorter, so the same ground is more steps, at the
    /// cadence that makes a stance cover exactly the ground the body covers:
    /// `planar · duty / length`, which is the two lengths' ratio over
    /// `cadence`, by `travel`. The gait itself stays the speed's, because
    /// walking and running is the leg vaulting over its foot at the speed the
    /// body goes, whichever way it goes. Against the CC0 references this is
    /// the reading that fits: the back-up steps at 1.0 Hz where this gives
    /// 1.03, and the strafe's 0.36 m stance is this to the centimetre — where
    /// building the gait from a faster "working" speed instead gave the
    /// back-up 0.87 Hz and the strafe 0.46 m, and turned a strafe at walking
    /// pace into a run.
    ///
    /// **The ground the body covers, not the pace's.** The pace eases behind
    /// a change of speed so the posture does not snap, and a stance stepped at
    /// the pace's cadence covers the pace's ground while the body covers its
    /// own. Measured on the consuming application's chassis looking out of
    /// its eyes, at every moment of the stride a change can land at: stepped
    /// at the pace's cadence, setting off ahead from a turn slid the planted
    /// soles 213 mm on average walking and 435 running, and a start from
    /// standing 365 and 529; stepped at the ground covered, 45 and 44, and 163
    /// either way — and no change of keys slides more for it. **A body handed
    /// no heading is stepped so too** (#363): the orbit's, whose stance the
    /// consuming application agreed by eye at its pace's cadence (its #1323).
    /// On that chassis, swept the same way, a start from standing slid 316 mm
    /// on average over the walking speeds and 506 over the running ones, and
    /// slides 164 at any speed; a change of pace slid 160 into a run and 114
    /// out of one, and slides 3; and the quarter turn agreed by eye slid 313
    /// on average and slides 317. A body that has only ever been handed one
    /// speed has `travel` one to the bit, so a heading straight ahead is
    /// exactly no heading at all and a walk measured that way is the walk it
    /// was. Anywhere else `travel` is one to within float rounding, not to
    /// the bit: the eased pace settles a few ULPs short of a new speed, and a
    /// speed differenced from positions, or one whose direction turns,
    /// carries their rounding into the clock — a few millionths of a cycle
    /// over a few seconds' walk. And a caller that hands its own gait
    /// ([`Inputs::gait`]) and turns hard enough for the turn's floor to bind
    /// is held to that gait's duty, as a headed stride always was.
    ///
    /// Held **up to a ceiling**: the faster of the legs' sprint cadence and
    /// the heading's own for whichever is more, the ground the body covers or
    /// its pace's. A stance bound can shorten a stride to a few centimetres
    /// where a split stance is easing through the feet's line (`split_bound`),
    /// and covering the travel in steps that short would cycle the legs at a
    /// dozen strides a second; past the ceiling the stance covers what it can
    /// and the planted foot gives the rest, as a turn too fast to step spins
    /// on its feet. The heading's cadence alone would clip a strafe, which
    /// steps at exactly it, every frame its body ran ahead of its pace: 4.6%
    /// slow under a fifth of velocity noise at 3 m/s. And **down to the turn's
    /// own need**, the yaw rate times the duty over [`TURN_REACH`] that
    /// [`Turn::cadence`] raises its cadence to: stepped slower, a stance would
    /// turn further than a hip does. The pace's cadence is no floor — a body slowing steps
    /// slower, and a jittering velocity is stepped as often above its mean as
    /// below it, where a floor at the pace's cadence stepped a straight-ahead
    /// walk 7% fast under a fifth of noise.
    ///
    /// A turn's stance is its yaw over the stance, so a stride stepped at
    /// another cadence than it was built for turns another amount in each:
    /// re-stated here, still inside [`TURN_REACH`], so a planted foot's
    /// bearing and the footholds' account of the turn agree with the cadence
    /// the body actually walks.
    fn stepping(
        rig: &Rig,
        heading: Heading,
        turn: Turn,
        gait: &Gait,
        (built, stride): (&Stride, &mut Stride),
        cadence: f32,
        travel: f32,
    ) -> f32 {
        if stride.length <= f32::EPSILON || built.length <= f32::EPSILON {
            return cadence;
        }
        let sprint = Speed::from_froude(FROUDE_SPRINT).cadence(rig);
        let ceiling = (cadence * travel.max(1.0) / heading.reach().max(f32::EPSILON)).max(sprint);
        let floor = (turn.yaw_rate().abs() * gait.duty / TURN_REACH).min(sprint);
        let stepped =
            (cadence * (built.length / stride.length) * travel).clamp(floor, ceiling.max(floor));
        if stepped != cadence && !turn.is_straight() {
            stride.yaw = (turn.yaw_rate() / stepped * gait.duty).clamp(-TURN_REACH, TURN_REACH);
        }
        stepped
    }

    /// One frame of the walk: the head of the engine's drive sequence, with the
    /// footing left off because something may be laid over it before the
    /// contacts are settled. `advanced` is how far the clock stepped this
    /// frame.
    #[allow(clippy::too_many_arguments)]
    fn walk_frame<F>(
        &mut self,
        rig: &Rig,
        inputs: &Inputs<'_>,
        gait: &Gait,
        stride: &Stride,
        advanced: f32,
        pose: &mut Pose,
        ground: F,
    ) -> super::gait::Walked
    where
        F: Fn(Vec3) -> Option<Ground> + Copy,
    {
        let walk = self.walk_at(inputs, None);
        match self.config.carriage {
            // Through the ledger: the same sequence with each planted contact
            // held to the world point it went down at, which is what keeps a
            // stance still while the body's speed changes underneath it.
            Carriage::Chassis => self.footholds.drive(
                walk,
                rig,
                pose,
                gait,
                stride,
                advanced,
                inputs.at,
                inputs.facing,
                ground,
            ),
            Carriage::Own => walk.drive(rig, pose, gait, stride, ground),
        }
    }

    /// This frame's [`Walk`], with the footing the caller asked for.
    fn walk_at(&self, inputs: &Inputs<'_>, footing: Option<FootingConfig>) -> Walk {
        Walk {
            cycle: self.cycle,
            posture: inputs.walk.posture,
            head_level: inputs.walk.head_level,
            gaze: inputs.walk.gaze,
            footing,
        }
    }

    /// Moves the airborne state on by one frame.
    ///
    /// **Two ways in and one way out.** A body launches when something pushes
    /// it up faster than the ground can, or falls when nothing is holding it
    /// up; it lands when a body that HAS been falling stops falling, which the
    /// apex cannot imitate because at the apex it is still on its way down.
    fn advance_airborne(&mut self, rig: &Rig, vertical: f32, delta: f32, swimming: bool) {
        let config = self.config;
        self.touched_down = false;
        let launched = vertical > config.launch_speed;
        let dropped = vertical < -config.fall_speed;
        let mut relaunched = false;
        match self.airborne.as_mut() {
            Some(air) if !air.landed => {
                // Held inside the flight, because whatever carries the body
                // decides how long it is in the air and the leap only predicted
                // it. A fall that outlasts its prediction holds at the end of
                // the arc, where the tuck has returned to nothing and the legs
                // are straight — which is what a body reaching for the ground
                // does anyway.
                let flight_ends = air.leap.wind_up(rig) + air.leap.flight();
                air.elapsed = (air.elapsed + delta).min(flight_ends - f32::EPSILON);
                air.falling |= vertical < -config.settle_speed;
                air.impact = air.impact.min(vertical);
                if air.falling && vertical >= -config.settle_speed {
                    // Touchdown. The leap is rebuilt from the arrival that
                    // actually happened rather than the one predicted at
                    // takeoff, and its clock is set to the head of the landing
                    // — the only stage left to play.
                    // **The landing sets off from the pitch the flight came
                    // down in** (#414): the rebuilt landing's own first pose
                    // is nose down, and a fall's flight, cut short wherever
                    // the ground arrived, was nose up, so a beast dropped
                    // 0.6 m pitched its trunk 24 degrees in the frame it came
                    // down. A person's leap pitches nothing.
                    self.landing_from = Some(air.leap.pitch_at(rig, air.elapsed));
                    let leap = Leap::new(air.impact.abs());
                    air.leap = leap;
                    air.elapsed = leap.wind_up(rig) + leap.flight();
                    air.landed = true;
                    self.touched_down = true;
                }
            }
            Some(air) => {
                air.elapsed += delta;
                // Leaving the ground again mid-landing is a new leap, not a
                // continuation of this one.
                relaunched = launched;
            }
            None => {}
        }
        if relaunched {
            self.airborne = None;
        } else if self.airborne.is_none() && !swimming && (launched || dropped) {
            // **Always symmetric, and never a drop.** A leap's `drop` says the
            // floor arrived at is lower than the one left, and carries that
            // difference in the pose for the whole landing — which is right for
            // a body that owns its own root and catastrophic where a chassis
            // has already carried it down: the two add and the root goes
            // underground by the entire fall height.
            //
            // Built from the SPEED rather than the direction, so a body that
            // walks off a ledge gets a real flight instead of the zero-length
            // one a leap of no speed has — whose stage machine reports a
            // landing from the first airborne frame and plants the feet in
            // mid-air all the way down.
            let leap = Leap::new(vertical.abs());
            self.airborne = Some(Airborne {
                leap,
                // Past the wind-up: a caller whose body is already off the
                // ground has no anticipation left to spend, so the leap is
                // joined at the instant its feet leave.
                elapsed: leap.wind_up(rig),
                falling: vertical < -config.settle_speed,
                impact: vertical.min(0.0),
                landed: false,
            });
        }
        // A landing that has played out is over; the body stands up out of it.
        if self
            .airborne
            .is_some_and(|air| air.leap.stage_at(rig, air.elapsed) == Stage::Standing)
        {
            self.airborne = None;
        }
    }

    /// Starts the gesture a request names, if the cooldown allows it.
    ///
    /// A name the roster does not carry starts nothing **and does not spend the
    /// cooldown**: a caller whose vocabulary has drifted from the engine's
    /// should get no gesture rather than a source change with no clip under it,
    /// and should not be rate-limited out of the gestures that do exist.
    ///
    /// **A beast in a state takes every request its own way** (#377), and none
    /// is rate-limited, since a state lasts as long as it is held:
    /// [`super::emote::GET_UP`] gets it up; any other gesture the body has
    /// gets it up and then plays, the last one asked for; the state it is
    /// already going into or holding changes nothing.
    fn start_gesture(&mut self, rig: &Rig, request: Option<&str>, stirred: bool) {
        let Some(name) = request else {
            return;
        };
        if let Some(active) = self.gesture.as_mut()
            && let Some(resting) = active.resting.as_mut()
        {
            if name == super::emote::GET_UP {
                resting.get_up(false);
                resting.then = None;
            } else if gesture::for_body(rig, name).is_some()
                && !(name == active.name && matches!(resting.phase, Phase::Down | Phase::Held))
            {
                resting.get_up(false);
                resting.then = Some(name.to_string());
            }
            return;
        }
        self.begin_gesture(rig, name, stirred, true);
    }

    /// Starts the gesture `name`, if the cooldown allows it when `throttled`,
    /// and answers whether it did.
    ///
    /// A beast's state does not start under a body that is being moved, is
    /// off the ground or is swimming (#377): any of those gets the beast
    /// straight back up. Nor does [`super::emote::GET_UP`] on a body already
    /// up, which has nothing to get up from.
    fn begin_gesture(&mut self, rig: &Rig, name: &str, stirred: bool, throttled: bool) -> bool {
        let ready = !throttled
            || self
                .gestured_at
                .is_none_or(|last| self.elapsed - last >= self.config.gesture_cooldown);
        if !ready || gesture::for_body(rig, name).is_none() {
            return false;
        }
        let state = super::emote::getting_up(rig, name).is_some();
        if (state && stirred) || name == super::emote::GET_UP {
            return false;
        }
        self.gesture = Some(ActiveGesture {
            name: name.to_string(),
            elapsed: 0.0,
            resting: state.then_some(Resting {
                phase: Phase::Down,
                through: 0.0,
                hurried: false,
                then: None,
            }),
        });
        self.gestured_at = Some(self.elapsed);
        true
    }

    /// How shut the lids are this frame: the blink's own phase, or the one the
    /// caller is holding them at, either way read through the resting face.
    fn closure(&mut self, inputs: &Inputs<'_>, delta: f32) -> f32 {
        let phase = match inputs.lids {
            Some(held) => held,
            None => self.blink.advance(delta),
        };
        inputs.face.closure_at(phase)
    }

    /// Pins the body to its bind pose for this frame.
    ///
    /// Deliberately bypasses the blend: the point is that the joints sit
    /// *exactly* where [`Pose::rest`] puts them, which is the frame an editor
    /// reconstructs a released offset against. `previous` and `current` are
    /// still kept up to date, so releasing the hold blends back out of rest
    /// normally. The blink rides along — an eyelid is not a socket, and a body
    /// that stops blinking reads as broken rather than as held.
    fn hold_at_rest(&mut self, rig: &Rig, inputs: &Inputs<'_>, delta: f32) -> Driven {
        let mut posed = Pose::rest(rig);
        let closure = self.closure(inputs, delta);
        if let Some(eyes) = inputs.eyes {
            eyes.blink(&mut posed, closure);
        }
        self.source = Source::Rest;
        self.transition = None;
        self.ridden = None;
        self.previous = self.current.take();
        self.current = Some(posed.clone());
        Driven {
            pose: posed,
            closure,
            source: Source::Rest,
            strained: false,
            straining: 0,
            lift: 0.0,
            aimed: false,
            idled: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_never_blends_and_locomotion_never_does_within_itself() {
        // The two ends of the rule `Source::family` states: a walk becoming a
        // run is one generator moving along one parameter, and a held pose has
        // no clock to be continuous with at all.
        assert!(!Source::Gait.needs_blend(Source::Gait));
        assert!(Source::Gait.needs_blend(Source::Idle));
        assert!(Source::Rest.needs_blend(Source::Idle));
        assert!(Source::Idle.needs_blend(Source::Rest));
        assert!(!Source::Rest.needs_blend(Source::Rest));
        assert_eq!(Source::Rest.family(), None);
    }

    #[test]
    fn a_frame_with_no_time_in_it_has_no_velocity() {
        // An infinity here reads as a launch, which is a body tucking its legs
        // because two positions arrived in the same frame.
        assert_eq!(velocity_of(Vec3::ZERO, Vec3::Z, 0.0), Vec3::ZERO);
        assert_eq!(velocity_of(Vec3::ZERO, Vec3::Z, 0.5), Vec3::Z * 2.0);
    }

    #[test]
    fn the_level_floor_is_flat_wherever_it_is_asked() {
        let ground = level_ground(Vec3::new(3.0, 9.0, -4.0)).expect("a level floor answers");
        assert_eq!(ground.position, Vec3::new(3.0, 0.0, -4.0));
        assert_eq!(ground.normal, Vec3::Y);
    }

    /// One frame at sixty a second.
    const STEP: f32 = 1.0 / 60.0;

    fn rig() -> Rig {
        Rig::from_skeleton(&crate::AvatarRecord::default().skeleton())
            .expect("the default body rigs")
    }

    /// A floor that falls away under the body's left side, further than any
    /// leg reaches: a left contact planted there cannot get down to it.
    fn a_hole_to_the_left(point: Vec3) -> Option<Ground> {
        let depth = if point.x > 0.02 { -1.5 } else { 0.0 };
        Some(Ground::level(Vec3::new(point.x, depth, point.z)))
    }

    /// Walks a body straight ahead at 1.4 m/s from its first frame for two
    /// seconds, over `ground`, and hands back every frame it drove.
    fn walked_over<F>(ground: F) -> Vec<Driven>
    where
        F: Fn(Vec3) -> Option<Ground> + Copy,
    {
        let rig = rig();
        let mut driver = Driver::seeded(7);
        (0..120)
            .map(|frame| {
                driver
                    .drive(
                        &rig,
                        &Inputs {
                            delta: STEP,
                            velocity: Vec3::Z * 1.4,
                            at: Vec3::Z * (1.4 * STEP * frame as f32),
                            ..Inputs::default()
                        },
                        ground,
                    )
                    .expect("an unheld body is posed")
            })
            .collect()
    }

    #[test]
    fn a_driven_frame_reports_what_its_settle_cost() {
        // **#334.** The walk's settle reports how far its footing solve had to
        // move a contact, and the driver used to swallow that into one bool.
        // Even on the flat the solve has work every step — it lays a sole that
        // lands toe-up flat before the roll pitches it again — so the reading
        // follows the cycle. Measured on this rig at 1.4 m/s: about 130 mm at
        // each touchdown, down to a millimetre at midstance.
        let walked = walked_over(level_ground);
        let lifts: Vec<f32> = walked
            .iter()
            .filter(|driven| driven.source == Source::Gait)
            .map(|driven| driven.lift)
            .collect();
        let most = lifts.iter().copied().fold(0.0f32, f32::max);
        let least = lifts.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            most > 0.0,
            "a body walked two seconds and no frame reported its footing solve moving a contact"
        );
        // A correction longer than the leg making it is not a correction: it
        // is a world coordinate or a unit leaking into the figure.
        let reach = rig()
            .limb_reach(crate::plan::Limb::HindLeft)
            .expect("a leg");
        assert!(
            most < reach,
            "the flat cost a {:.1} mm correction on a {:.1} mm leg",
            most * 1000.0,
            reach * 1000.0
        );
        // And it is THIS frame's figure: a value carried over from an earlier
        // frame, or set once, does not fall to a sliver at midstance.
        assert!(
            least < most * 0.1,
            "the reading barely moves through the cycle ({:.1} to {:.1} mm) — a stale figure",
            least * 1000.0,
            most * 1000.0
        );

        // A standing body is planted by its idle, not by a footing solve, so it
        // has nothing to report — zero rather than whatever the last walk cost.
        let rig = rig();
        let mut standing = Driver::seeded(7);
        for _ in 0..60 {
            let driven = standing
                .drive(
                    &rig,
                    &Inputs {
                        delta: STEP,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            assert_eq!(driven.source, Source::Idle);
            assert_eq!(
                driven.lift, 0.0,
                "an idle frame reported a settle it never ran"
            );
        }
    }

    #[test]
    fn the_straining_count_is_the_flag_counted() {
        // **#334.** `strained` is one bit and `straining` says how many — so
        // the two must agree on every frame, and on ground a leg cannot reach
        // the count must actually rise, or the invariant holds vacuously.
        let mut counted = 0usize;
        for driven in walked_over(a_hole_to_the_left)
            .iter()
            .chain(&walked_over(level_ground))
        {
            assert_eq!(
                driven.strained,
                driven.straining > 0,
                "the flag and the count disagree: strained {} with {} straining",
                driven.strained,
                driven.straining
            );
            assert!(
                driven.straining <= 2,
                "a biped cannot strain {} contacts",
                driven.straining
            );
            counted = counted.max(driven.straining);
        }
        assert!(
            counted >= 1,
            "a body walking beside a hole deeper than its leg never counted a contact straining"
        );
    }

    #[test]
    fn a_blend_asked_for_joins_a_layer_that_changed() {
        // **#334.** A caller swapping the layer it lays over the motion has no
        // source to change, so the driver never blends it: the swap lands in
        // one frame. Asked for, the same swap arrives through the transition
        // the driver already keeps for its own switches — and it is a
        // transition, not a different pose: once it has run, the two bodies
        // stand identically.
        let rig = rig();
        let arm = rig
            .limb_chain(crate::plan::Limb::ForeLeft)
            .expect("a humanoid has an arm");
        let (shoulder, wrist) = (arm[0], arm[2]);
        let raised = move |_: &Rig, pose: &mut Pose| {
            pose.rotations[shoulder] = glam::Quat::from_rotation_z(1.0);
        };
        let lowered = move |_: &Rig, pose: &mut Pose| {
            pose.rotations[shoulder] = glam::Quat::from_rotation_z(-1.0);
        };
        let frame = |driver: &mut Driver, layer: Layer<'_>| {
            driver
                .drive(
                    &rig,
                    &Inputs {
                        delta: STEP,
                        over_locomotion: Some(layer),
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed")
                .pose
                .forward(&rig)
                .positions[wrist]
        };

        let (mut snapped, mut blended) = (Driver::seeded(7), Driver::seeded(7));
        let mut before = Vec3::ZERO;
        for _ in 0..30 {
            before = frame(&mut snapped, &raised);
            frame(&mut blended, &raised);
        }
        blended.blend_now();
        let snap = frame(&mut snapped, &lowered).distance(before);
        let blend = frame(&mut blended, &lowered).distance(before);
        assert!(
            blend < snap * 0.5,
            "asked to blend, the wrist still jumped {:.1} mm in one frame against {:.1} unasked",
            blend * 1000.0,
            snap * 1000.0
        );

        let blend_frames = (DriverConfig::default().blend / STEP).ceil() as usize + 2;
        let (mut settled_snap, mut settled_blend) = (Vec3::ZERO, Vec3::ONE);
        for _ in 0..blend_frames {
            settled_snap = frame(&mut snapped, &lowered);
            settled_blend = frame(&mut blended, &lowered);
        }
        assert!(
            settled_snap.distance(settled_blend) < 1e-4,
            "the blend left a lasting difference of {:.3} mm — a request must be a transition",
            settled_snap.distance(settled_blend) * 1000.0
        );
    }
}
