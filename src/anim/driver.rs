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
//!   ([`Footholds`]), whenever something else is carrying the body.
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

use crate::det::DetMath;
use crate::face::{Expression, Eyes};
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
use super::speed::{GRAVITY, Speed};
use super::swim::Swim;
use super::transition::{self, Family};
use super::{Target, gesture};

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
    /// Below this horizontal speed the body idles, in m/s.
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
    /// runs `0..1` and says nothing about seconds — so this is the only place
    /// the real duration is decided. A second and a half is a greeting: long
    /// enough for three waves to read as waves, short enough that a body is not
    /// still doing it when the conversation has moved on.
    pub gesture_secs: f32,
    /// The shortest gap between two gestures on one body, in seconds.
    ///
    /// Measured from one gesture's **start** rather than its end, so the limit
    /// is a rate and not a gap: a sender pasting the same word four times
    /// waves once and then stands there. Longer than [`Self::gesture_secs`], so
    /// a well-behaved sender is never throttled by it.
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
    /// can strafe or back up derives one and passes it.
    pub heading: Option<Heading>,
    /// Whether the body is in water deep enough to swim in.
    ///
    /// The caller's classification, because what counts as deep is a fact about
    /// the world and its water rather than about the body. A wading body has
    /// its feet on the bottom and is walking.
    pub swimming: bool,
    /// A gesture asked for **this frame**, by the name
    /// [`gesture::by_name`] knows it as.
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
    /// The walk's ablation switches.
    pub walk: WalkFlags,
    /// How fast the body is turning, in radians a second, positive toward its
    /// own left.
    ///
    /// A yaw **rate** is per second and a stride is per stance, so the cadence
    /// joins them — the body's own, recovered from the stride it is walking,
    /// rather than named beside it. A turn asserted independently of the legs
    /// is a turn the feet are not taking.
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
    /// one that strains constantly is a body whose goals are wrong.
    pub strained: bool,
    /// Whether a gesture aimed the head this frame.
    ///
    /// A caller with a gaze layer of its own stands aside when this is set: a
    /// nod is a gesture the body is making on purpose, and a target written
    /// over it arrives correct and is put back level a moment later.
    pub aimed: bool,
    /// What the idle reported, if the idle ran.
    pub idled: Option<Idled>,
}

/// A gesture in progress on one body.
#[derive(Clone, Debug, PartialEq)]
struct ActiveGesture {
    /// Which gesture is playing, by roster name. The clip itself is rebuilt
    /// each frame — a goal-space clip is a few keyed goals, so rebuilding costs
    /// less than the solve that follows it and keeps this a value rather than a
    /// cache.
    name: String,
    /// How far into it, in seconds. A gesture plays **once**: this counts up to
    /// [`DriverConfig::gesture_secs`] and then clears, where a locomotion cycle
    /// wraps.
    elapsed: f32,
}

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
}

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

    /// The leap in progress, or [`None`] while the body is on the ground.
    #[must_use]
    pub fn airborne(&self) -> Option<Airborne> {
        self.airborne
    }

    /// The gesture playing and how far into it, in seconds.
    #[must_use]
    pub fn gesture(&self) -> Option<(&str, f32)> {
        self.gesture
            .as_ref()
            .map(|active| (active.name.as_str(), active.elapsed))
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
    pub fn warped(&mut self) {
        self.footholds.reset();
        self.paced = None;
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
        let travelling = (planar >= config.idle_below).then(|| Speed::new(rig, paced));

        // **The stroke's clock, which the swim leaves to its caller.** Both
        // ends are derived: at speed one cycle carries the body a fixed number
        // of its own lengths, and at rest a tread sculls at the body's own
        // pendulum frequency — the one rate a body of this size has without
        // anybody choosing it. A giant sculls slower for the same reason it
        // walks slower.
        let stroke_rate = || {
            let length = rig.extent().max(f32::EPSILON);
            let tread = (GRAVITY / length).sqrt() / std::f32::consts::TAU;
            (planar / (length * config.lengths_per_stroke)).max(tread)
        };

        // A leap owns the legs for as long as it lasts: a body cannot be
        // mid-stride and mid-air at once, and pretending otherwise is how a
        // jump ends up with a walk cycle running underneath it.
        let showing_leap = matches!(inputs.showing, Some(Showing::Leap(_)));
        let (locomotion, derived_cadence) = if swimming {
            (Source::Swim, stroke_rate())
        } else if showing_leap || airborne {
            (Source::Leap, 0.0)
        } else if let Some(speed) = travelling {
            (Source::Gait, speed.cadence(rig))
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
        if let (Some(speed), Some(before)) = (travelling, self.gaiting)
            && before != speed
        {
            let into = speed.gait(rig);
            let from = Gait {
                duty: before.duty(),
                ..into.clone()
            };
            self.cycle = transition::carry_cycle(&from, &into, self.cycle);
        }
        self.gaiting = travelling;

        if let Some(held) = inputs.cycle {
            self.cycle = held;
        } else if !airborne || swimming {
            self.cycle = (self.cycle + delta * cadence).fract();
        }

        self.start_gesture(inputs.gesture);
        // Advanced and retired before the pose is built, so a gesture that
        // finished this frame does not get one last frame of overlay.
        if let Some(active) = self.gesture.as_mut() {
            active.elapsed += delta;
            if active.elapsed >= config.gesture_secs {
                self.gesture = None;
            }
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
        let mut strained = false;
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
        if locomotion != Source::Gait {
            self.footholds.reset();
        }
        match locomotion {
            Source::Rest | Source::Gesture => {}
            Source::Idle => {
                // The whole layer through one call, which advances the schedule
                // and poses every layer together: a stage a caller has to
                // remember is a stage a caller forgets.
                idled = Some(self.idler.drive_on(rig, &mut pose, delta, ground));
            }
            Source::Gait => {
                let (gait, stride, walked) =
                    self.walk_frame(rig, inputs, travelling, &mut pose, ground);
                strained |= !walked.steps.straining.is_empty();
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
                let leaping = match inputs.showing {
                    Some(Showing::Leap(leap)) => Some((leap, self.cycle * leap.duration(rig))),
                    _ => self.airborne.map(|air| (air.leap, air.elapsed)),
                };
                if let Some((leap, elapsed)) = leaping {
                    let leapt = leap.drive(rig, &mut pose, elapsed, ground);
                    strained |= !leapt.straining.is_empty();
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
        if let Some(active) = &self.gesture
            && let Some(clip) = gesture::by_name(&active.name)
        {
            clip.apply(rig, &mut pose, active.elapsed / config.gesture_secs);
            aimed = clip.tracks.iter().any(|track| track.target == Target::Gaze);
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
        // has nothing down, and a leap places its own contacts.
        if let Some((gait, stride)) = &walking {
            let walked = self
                .walk_at(inputs, inputs.walk.footing)
                .settle(rig, &mut pose, gait, stride, &steps, ground);
            strained |= walked.straining() > 0;
        }

        // **The idle's glance, and only where nothing is aiming the head on
        // purpose.** A gesture that aims the head owns it for as long as it
        // runs, and asking the clip rather than keeping a list of which
        // gestures involve the head is what keeps that true as the roster
        // grows.
        if !aimed && let Some(target) = idled.and_then(|idled: Idled| idled.glance) {
            look_at(rig, &mut pose, target, &super::idle::glance_config());
        }
        if let Some(over) = inputs.over_settled {
            over(rig, &mut pose);
        }

        // Inertialize source switches so a walk does not snap into a stand —
        // but only where the two are different activities. Within locomotion
        // the answer is no: see `Source::family`.
        if config.blend > 0.0
            && self.source.needs_blend(source)
            && let (Some(previous), Some(current)) = (&self.previous, &self.current)
        {
            self.transition = Some(Inertializer::start(
                previous,
                current,
                &pose,
                delta,
                config.blend,
            ));
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

        // The blink rides AFTER the blend — see `Inputs::eyes`.
        let closure = self.closure(inputs, delta);
        if let Some(eyes) = inputs.eyes {
            eyes.blink(&mut posed, closure);
        }
        Some(Driven {
            pose: posed,
            closure,
            source,
            strained,
            aimed,
            idled,
        })
    }

    /// One frame of the walk: the head of the engine's drive sequence, with the
    /// footing left off because something may be laid over it before the
    /// contacts are settled.
    fn walk_frame<F>(
        &mut self,
        rig: &Rig,
        inputs: &Inputs<'_>,
        travelling: Option<Speed>,
        pose: &mut Pose,
        ground: F,
    ) -> (Gait, Stride, super::gait::Walked)
    where
        F: Fn(Vec3) -> Option<Ground> + Copy,
    {
        // **One number in, everything out.** The gait pattern, its duty, the
        // stride length, the foot lift and the cadence all come from how fast
        // the body is travelling, expressed as a Froude number so the same
        // relation fits a child, a giant and a quadruped.
        let speed = travelling.unwrap_or(Speed::STILL);
        let gait = inputs.gait.cloned().unwrap_or_else(|| speed.gait(rig));
        let mut stride = match inputs.heading {
            Some(heading) => speed.stride(rig).toward(rig, heading),
            None => speed.stride(rig),
        };
        if inputs.turn != 0.0 {
            let cadence = Speed::of(rig, &gait, &stride).cadence(rig);
            if cadence > f32::EPSILON {
                stride.yaw = inputs.turn / cadence * gait.duty;
            }
        }
        let walk = self.walk_at(inputs, None);
        let walked = match self.config.carriage {
            // Through the ledger: the same sequence with each planted contact
            // held to the world point it went down at, which is what keeps a
            // stance still while the body's speed changes underneath it.
            Carriage::Chassis => self.footholds.drive(
                walk,
                rig,
                pose,
                &gait,
                &stride,
                inputs.at,
                inputs.facing,
                ground,
            ),
            Carriage::Own => walk.drive(rig, pose, &gait, &stride, ground),
        };
        (gait, stride, walked)
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
                    let leap = Leap::new(air.impact.abs());
                    air.leap = leap;
                    air.elapsed = leap.wind_up(rig) + leap.flight();
                    air.landed = true;
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
    fn start_gesture(&mut self, request: Option<&str>) {
        let Some(name) = request else {
            return;
        };
        let ready = self
            .gestured_at
            .is_none_or(|last| self.elapsed - last >= self.config.gesture_cooldown);
        if !ready || gesture::by_name(name).is_none() {
            return;
        }
        self.gesture = Some(ActiveGesture {
            name: name.to_string(),
            elapsed: 0.0,
        });
        self.gestured_at = Some(self.elapsed);
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
        self.previous = self.current.take();
        self.current = Some(posed.clone());
        Driven {
            pose: posed,
            closure,
            source: Source::Rest,
            strained: false,
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
}
