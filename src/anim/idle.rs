//! Standing still, which is not the same as not moving.
//!
//! An avatar is idle for most of the time it exists, so this is the state a
//! viewer sees longest — and it is the one place a procedural layer beats an
//! imported clip outright rather than merely matching it. **A clip is a loop,
//! and a loop repeats.** However good the performance, an eight-second idle
//! comes round every eight seconds, and once a viewer has noticed that they
//! cannot stop noticing it. Nothing here loops: the sway is noise over time and
//! the shifts and fidgets are drawn from a schedule, so the body never comes
//! back to a pose it held. `examples/idleaudit` measures exactly that, against
//! the shipped `Idle_A` as the thing to beat.
//!
//! # The layers, and what each of them is derived from
//!
//! * **Breath**, at a rate that is the body's own. A breath is a mechanical
//!   oscillation and this crate times mechanical oscillations the way it times
//!   everything else — against `sqrt(L/g)`, the pendulum scaling the Froude
//!   number encodes — so one dimensionless number sets the rate for a child and
//!   an adult alike. See [`BREATH_PERIODS`], which is checked against a
//!   toddler's resting rate as well as an adult's.
//! * **Sway**, as the inverted pendulum quiet standing actually is. The body
//!   tips about its feet at the frequency that pendulum has, `sqrt(g/h)` on its
//!   own centre of mass, so the sub-1 Hz band the posturography literature
//!   reports is a consequence rather than a filter setting. See [`Idle::sway_rate`].
//! * **Weight shift**, sized by the stance. A body settling onto one leg puts
//!   its mass over that foot, so how far the pelvis travels is half the stance
//!   width and not a number — and the sink that goes with it is whatever keeps
//!   the far leg in reach, which is the straight-leg slack the gait already
//!   knows about.
//! * **Fidgets**, as sparse scheduled events. A shoulder roll, and a glance —
//!   the glance through [`super::look_at`], which already spreads a turn down
//!   the chest, neck and head and already clamps it at a neck's limit.
//!
//! # Rotations, and one translation
//!
//! A [`Pose`] is a rotation per joint and a single root offset. There is no
//! per-joint translation and no scale, which rules out the obvious model of a
//! breath — **a body on this rig cannot lift its chest straight up.** What it
//! can do is extend the spine and raise the shoulders, which is most of what
//! the eye reads as breathing anyway, and that is what is here. It is worth
//! saying out loud because the first instrument written for this asked how far
//! the chest ROSE, and a rotation about a pivot below a joint can only lower
//! it: the reading came out negative and the breath looked inverted when it was
//! the ruler that was wrong.
//!
//! The sway is the one place the root offset earns its keep, and it is two
//! mechanisms rather than one. **Fore-and-aft** the body tips about the line
//! through its feet — a root rotation plus the translation that keeps that line
//! still — and the feet cannot move under it because they lie on the axis being
//! turned about. **Sideways** it does not tip at all: the feet are offset
//! ACROSS that axis, so a roll about it drives one sole into the floor and
//! lifts the other, which is exactly what the first version of this did and
//! exactly what `examples/idleaudit` caught, at 89 mm on a body that was
//! supposed to be standing still. Lateral sway in a standing body is a loading
//! strategy at the hips, so it is a translation and it shares the weight
//! shift's path and its footing solve.

use glam::{Quat, Vec3};
use noise::{NoiseFn, Simplex};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

use super::gaze::GazeConfig;
use super::ground::{FootingConfig, Ground, plant_feet_of, solve_contact};
use super::pose::Pose;
use super::speed::GRAVITY;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// How many `sqrt(height/g)` a breath lasts.
///
/// **Dimensionless, so it is the same number for a child and an adult**, which
/// is the whole reason to express a rate this way rather than in breaths per
/// minute. A breath is a mechanical oscillation of the chest wall and every
/// other timing in this crate — the gait's cadence, the leap's spring, the
/// walk–run transition — is set against the same pendulum scaling.
///
/// **8.9, and it is anchored at both ends of the range rather than fitted at
/// one.** On a 1.75 m adult, `sqrt(1.75/9.81)` is 0.422 s, so this gives a
/// 3.75 s breath — 16 a minute, the middle of the 12–20 the literature reports
/// at rest. The check that matters is the other end: on a 1.0 m toddler the
/// same number gives 2.83 s, which is 21 a minute, and a small child at rest
/// really does breathe half again as fast as an adult. A rate in breaths per
/// minute would have had to be re-picked for every body; this one predicts
/// them.
pub const BREATH_PERIODS: f32 = 8.9;

/// How far the shoulders travel over a breath, against how far the chest does.
///
/// **Half, and it is the ARM that moves, not the shoulder.** This rig has no
/// clavicle: a limb's first joint hangs off the chest, so rotating it swings
/// the arm and leaves the shoulder itself exactly where it was. There is
/// therefore no shoulder rise available here at all — the cue a real breath
/// carries most of its visibility in cannot be expressed — and what is left is
/// a slight opening of the arms against the ribcage as it fills.
///
/// Half of the chest's excursion, so an adult's elbows drift under three
/// millimetres over a breath. Larger reads as the body sighing, and the honest
/// note is that on a rig with a clavicle this constant would be replaced rather
/// than retuned.
const BREATH_SHOULDER: f32 = 0.5;

/// What share of the breath the abdomen carries, against the chest.
///
/// **Slightly more than half.** Quiet breathing at rest is predominantly
/// diaphragmatic: the abdomen moves at least as much as the ribcage, and a
/// chest that heaves while the belly stays still is what a body does when it is
/// out of breath. Standing shifts the balance toward the ribcage from the
/// supine two-thirds, so a little over half is the standing figure.
const BREATH_ABDOMEN: f32 = 0.55;

/// How far the abdomen leads the chest, as a share of the breath.
///
/// The diaphragm moves first and the ribcage follows. A twentieth of a cycle
/// is under two tenths of a second on an adult, which is not something anyone
/// sees directly — what it removes is the single-piece look of a trunk whose
/// every joint turns on the same beat, the same thing [`super::gait`]'s arm lag
/// removes from a swing.
const BREATH_LEAD: f32 = 0.05;

/// What share of the body's weight the bearing leg takes in a relaxed stand.
///
/// **Two thirds, and the reason it is a share rather than a distance is that a
/// distance would be a different number on every body.** A person standing at
/// ease does not put all of their weight on one leg — that is a pose you hold
/// for a photograph, not one you stand in — they load one to roughly two thirds
/// and leave the other resting.
///
/// # The beam, and what it approximates away
///
/// How far the pelvis has to travel for that follows from where the weight line
/// falls between the two feet: treat the body as a beam on two supports and the
/// load share is linear in that position, so a share `s` puts the line at
/// `(2s − 1)` of the way from the middle to the bearing foot. On the default
/// body that is 29 mm, and the counter-lean that holds the shoulders over the
/// middle is then four degrees — a hip-shot stand rather than a lunge.
///
/// **The approximation is that the pelvis and the weight line are taken to move
/// together**, and they do not quite: the upper body stays behind, so the line
/// moves less than the pelvis does. Doing better needs a mass per joint and
/// this crate has none — `Zone` names body regions, not weights. What it would
/// change is the size of this one number, not the shape of anything, and the
/// error is in the conservative direction: the real load share at this pelvis
/// offset is a little under two thirds, which is a body standing a little more
/// evenly than it says. Recorded rather than smuggled.
const BEARING_SHARE: f32 = 2.0 / 3.0;

/// How far the body's centre of mass sits up its own height.
///
/// **Rounded from the anthropometric figure**, which puts a standing adult's
/// centre of mass at about 55% of stature, a little above the navel. Used for
/// the sway's pendulum length rather than for any mass calculation, so the
/// precision that matters is in the square root: a 10% error here is 5% in the
/// frequency.
const CENTRE_OF_MASS: f32 = 0.55;

/// Tuning for [`Idle`].
///
/// The defaults are a body standing and doing nothing. [`Self::talking`] and
/// [`Self::listening`] are the same layers at different settings rather than
/// different behaviours — which is the point of them being parameters: a
/// talking idle and a listening one used to be two more clips to author, keep
/// in step and blend between.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdleConfig {
    /// How far the chest travels over a breath, as a share of body height.
    ///
    /// A body-size fraction rather than a constant, so a child breathes at its
    /// own scale. The default puts an adult's chest excursion at a few
    /// millimetres, which is quiet breathing.
    pub breath: f32,
    /// Multiplier on the rate [`BREATH_PERIODS`] gives.
    ///
    /// One is at rest. Talking raises it, because speech is breathing.
    pub breath_rate: f32,
    /// How far the body sways, as a share of body height.
    ///
    /// The default puts an adult's head near a centimetre of drift, which is
    /// the top of the quiet-standing band — the head is the far end of the
    /// pendulum and moves furthest.
    pub sway: f32,
    /// Shortest wait between weight shifts, in seconds.
    pub min_shift: f32,
    /// Longest wait between weight shifts, in seconds.
    pub max_shift: f32,
    /// How long the body takes to settle onto the other leg, in seconds.
    pub shift_time: f32,
    /// Shortest wait between fidgets, in seconds.
    pub min_fidget: f32,
    /// Longest wait between fidgets, in seconds.
    pub max_fidget: f32,
    /// How long a fidget lasts, in seconds.
    pub fidget_time: f32,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            // Three thousandths of stature: about 5 mm on an adult.
            breath: 0.003,
            breath_rate: 1.0,
            // Six thousandths: about 10 mm at the head.
            sway: 0.006,
            min_shift: 10.0,
            max_shift: 40.0,
            shift_time: 1.2,
            min_fidget: 8.0,
            max_fidget: 30.0,
            fidget_time: 1.6,
        }
    }
}

impl IdleConfig {
    /// A body that is speaking.
    ///
    /// **Speech is breathing**, so the rate goes up and the depth with it — a
    /// talker takes quicker, shallower breaths between phrases. The gestures
    /// come with it: people move much more while they are talking, so the
    /// fidget schedule tightens by roughly a factor of three. The sway and the
    /// weight shifts are left alone; standing is standing.
    #[must_use]
    pub fn talking() -> Self {
        Self {
            breath_rate: 1.6,
            breath: 0.004,
            min_fidget: 2.5,
            max_fidget: 9.0,
            ..Self::default()
        }
    }

    /// A body that is listening to someone.
    ///
    /// **The opposite, and more so than it looks.** A listener goes still —
    /// stiller than a body alone in a room, because attention costs movement.
    /// The breath drops toward the bottom of the resting band, the fidgets
    /// become rare, and the weight shifts stretch out. What is deliberately
    /// *not* reduced is the sway: a body cannot stop swaying, and one that does
    /// reads as a photograph.
    #[must_use]
    pub fn listening() -> Self {
        Self {
            breath_rate: 0.85,
            breath: 0.0025,
            min_shift: 25.0,
            max_shift: 70.0,
            min_fidget: 25.0,
            max_fidget: 90.0,
            ..Self::default()
        }
    }
}

/// What the idle is doing at this moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Idled {
    /// Which leg is carrying the body's weight, or `None` while it stands
    /// evenly on both.
    pub bearing: Option<Limb>,
    /// Whether a fidget is running.
    pub fidgeting: bool,
    /// How far through the breath the body is, `0..1`, inhaling from zero.
    pub breath: f32,
    /// Where the body is looking, in body space, or `None` if the idle is not
    /// aiming the gaze this moment.
    ///
    /// **A point rather than a rotation**, so a caller hands it to
    /// [`super::look_at`] and the gaze layer spreads and clamps it as it does
    /// for every other target. Duplicating any of that here would give the body
    /// two opinions about where it is looking.
    pub glance: Option<Vec3>,
}

/// What a scheduled event is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Nothing happening, counting down.
    Waiting(f32),
    /// Running, counting down.
    Running(f32),
}

/// Drives a body standing still.
///
/// Seeded rather than drawing from the thread's generator, so a recording of a
/// body is reproducible and a test can assert on it — the same bargain
/// [`crate::Blink`] strikes, and for the same reason.
#[derive(Clone, Debug)]
pub struct Idle {
    config: IdleConfig,
    elapsed: f32,
    /// The noise field the sway is drawn from. Two axes rather than one so the
    /// two horizontal directions can be sampled from separate lanes of the same
    /// field, which is cheaper than two fields and keeps them uncorrelated.
    field: Simplex,
    shift: Stage,
    /// Which leg the body is settling onto, and which it was on before.
    bearing: Option<Limb>,
    was: Option<Limb>,
    fidget: Stage,
    /// Which way this fidget glances, and how far it rolls the shoulders.
    glance: Vec3,
    roll: f32,
    rng: Pcg64Mcg,
}

impl Idle {
    /// An idle with the given configuration and seed.
    #[must_use]
    pub fn new(config: IdleConfig, seed: u64) -> Self {
        let mut rng = Pcg64Mcg::seed_from_u64(seed);
        let shift = Stage::Waiting(draw(&mut rng, config.min_shift, config.max_shift));
        let fidget = Stage::Waiting(draw(&mut rng, config.min_fidget, config.max_fidget));
        Self {
            config,
            elapsed: 0.0,
            // Cast rather than hashed: the seed is already well mixed by the
            // generator above, and a second mixing here would only make the
            // relationship between the two harder to reason about.
            field: Simplex::new(seed as u32),
            shift,
            bearing: None,
            was: None,
            fidget,
            glance: Vec3::ZERO,
            roll: 0.0,
            rng,
        }
    }

    /// An idle with the default configuration.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self::new(IdleConfig::default(), seed)
    }

    /// The configuration in force.
    #[must_use]
    pub fn config(&self) -> IdleConfig {
        self.config
    }

    /// Changes the configuration without disturbing what is already running.
    ///
    /// **The schedule in flight is left alone**, so a body that starts talking
    /// does not snap out of the weight shift it is halfway through — the next
    /// wait is drawn from the new settings and everything already begun
    /// finishes under the old ones. That is the same courtesy
    /// [`super::Gait::until_handoff`] extends to a gait changing speed, and for
    /// the same reason: a change made where the body is committed reads as a
    /// glitch rather than as a change of mind.
    pub fn set_config(&mut self, config: IdleConfig) {
        self.config = config;
    }

    /// How far through a breath the body is, `0..1`, inhaling from zero.
    #[must_use]
    pub fn breath(&self, rig: &Rig) -> f32 {
        let period = self.breath_period(rig);
        if period <= f32::EPSILON {
            return 0.0;
        }
        (self.elapsed / period).rem_euclid(1.0)
    }

    /// How long one breath lasts on this body, in seconds.
    ///
    /// [`BREATH_PERIODS`] against the body's own `sqrt(height/g)`. A body with
    /// no height — which is not a body — breathes not at all rather than
    /// infinitely fast.
    #[must_use]
    pub fn breath_period(&self, rig: &Rig) -> f32 {
        let height = stature(rig);
        if height <= f32::EPSILON || self.config.breath_rate <= f32::EPSILON {
            return 0.0;
        }
        BREATH_PERIODS * (height / GRAVITY).sqrt() / self.config.breath_rate
    }

    /// How fast this body sways, in cycles per second.
    ///
    /// **The inverted pendulum's own frequency, and so not a choice.** Quiet
    /// standing is a body balancing over its ankles, which is a pendulum upside
    /// down with its mass at `CENTRE_OF_MASS` of stature; its frequency is
    /// `sqrt(g/h)/2π`, which on an adult comes to about half a hertz. That the
    /// posturography literature reports quiet-standing sway as almost entirely
    /// sub-1 Hz is then a consequence of the body's proportions rather than a
    /// band anything here was filtered to.
    #[must_use]
    pub fn sway_rate(&self, rig: &Rig) -> f32 {
        let height = stature(rig) * CENTRE_OF_MASS;
        if height <= f32::EPSILON {
            return 0.0;
        }
        (GRAVITY / height).sqrt() / std::f32::consts::TAU
    }

    /// Advances the schedule and poses the body, and reports what it did.
    ///
    /// **One entry point, and every layer inside it.** The gait learned this
    /// the expensive way — [`super::Walk::drive`] exists because four of five
    /// consumers had forgotten a stage — and an idle has the same shape and the
    /// same trap: four layers, an order between two of them that matters, and a
    /// footing solve in the middle.
    ///
    /// The order is breath, sway, shift, fidget. The shift comes after the sway
    /// because the sway is a rotation about the feet and the shift is a
    /// translation across them, and settling the feet after both is what keeps
    /// the soles where the ground is. The fidget is last because it touches only
    /// the shoulders and cannot disturb a footing solve, exactly as the walk's
    /// arm swing cannot.
    ///
    /// `ground` answers what is beneath a point in the frame the body is posed
    /// in, as [`plant_feet_of`] requires. A body standing on the flat can hand
    /// back a level plane and never think about it again.
    pub fn drive(&mut self, rig: &Rig, pose: &mut Pose, dt: f32) -> Idled {
        self.drive_on(rig, pose, dt, |at| {
            Some(Ground {
                position: Vec3::new(at.x, 0.0, at.z),
                normal: Vec3::Y,
            })
        })
    }

    /// As [`Self::drive`], on ground that is not level.
    pub fn drive_on<F>(&mut self, rig: &Rig, pose: &mut Pose, dt: f32, ground: F) -> Idled
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        if !pose.fits(rig) {
            return Idled::default();
        }
        self.elapsed += dt.max(0.0);
        self.advance(dt.max(0.0));

        let breath = self.breath(rig);
        self.breathe(rig, pose, breath);
        // The two horizontal axes are gathered before either is applied,
        // because they end in the same place — one root offset and one plant —
        // and settling the feet twice would be a solve chasing itself.
        let (fore, side) = self.drift(rig);
        let bearing = self.settle(rig, pose, side, &ground);
        self.tip(rig, pose, fore);
        // **The feet are put back by a SOLVE, not by the plant.**
        // `plant_feet_of` settles a sole onto the ground it is over — it is a
        // vertical correction and a levelling, which is all the gait ever needs
        // because `step` has already placed each contact horizontally. Nothing
        // has placed these. Measured, a body whose pelvis had moved 88 mm
        // sideways came out of the plant with its feet 98 mm from where they
        // started, because the plant was never asked to hold them there.
        //
        // So each contact is solved back to where it rests, and only then
        // levelled. That is also what softens the free knee: the leg is now
        // nearer its own foot than a straight leg would be, and a solve that
        // reaches a closer goal bends.
        let contacts = rig.ground_contacts();
        for &limb in &contacts {
            let Some(&joint) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            solve_contact(rig, pose, limb, rig.joints[joint].position);
        }
        if !contacts.is_empty() {
            plant_feet_of(rig, pose, &contacts, &ground, &FootingConfig::default());
        }
        let fidgeting = self.twitch(rig, pose);

        Idled {
            bearing,
            fidgeting,
            breath,
            // **A point in body space, not a direction**, so a caller hands it
            // to `super::look_at` without having to know how far away a glance
            // looks or how tall this body is. Thrown two statures out, which is
            // far enough that the gaze reads as a look across a room rather
            // than at something held in front of the face.
            glance: fidgeting.then(|| {
                let height = stature(rig);
                let eye = rig
                    .in_zone(Zone::Head)
                    .first()
                    .map_or(height, |&joint| rig.joints[joint].position.y);
                self.glance * height * 2.0 + Vec3::Y * eye
            }),
        }
    }

    /// Counts the schedules down and draws the next wait when one expires.
    fn advance(&mut self, dt: f32) {
        let config = self.config;
        let mut rng = std::mem::replace(&mut self.rng, Pcg64Mcg::seed_from_u64(0));

        self.shift = match self.shift {
            Stage::Waiting(left) if left <= dt => {
                // Onto whichever leg is not already carrying. A body that
                // shifted onto the leg it was already standing on would have
                // nothing to do, and one that chose at random would sometimes
                // do it twice in a row.
                self.was = self.bearing;
                self.bearing = Some(match self.bearing {
                    Some(Limb::HindLeft) => Limb::HindRight,
                    Some(_) => Limb::HindLeft,
                    None if rng.random::<bool>() => Limb::HindLeft,
                    None => Limb::HindRight,
                });
                Stage::Running(config.shift_time - (dt - left))
            }
            Stage::Waiting(left) => Stage::Waiting(left - dt),
            Stage::Running(left) if left <= dt => {
                self.was = self.bearing;
                Stage::Waiting(draw(&mut rng, config.min_shift, config.max_shift) - (dt - left))
            }
            Stage::Running(left) => Stage::Running(left - dt),
        };

        self.fidget = match self.fidget {
            Stage::Waiting(left) if left <= dt => {
                // Where this one glances and how far it rolls, drawn once at
                // the start so the fidget holds one intention rather than
                // wandering while it runs.
                // A direction, normalised; `drive_on` puts it at a distance
                // and a height the body has, so what a caller receives is a
                // point it can hand straight to `look_at`.
                self.glance = Vec3::new(
                    rng.random_range(-0.6f32..0.6),
                    rng.random_range(-0.15f32..0.15),
                    1.0,
                )
                .normalize();
                self.roll = rng.random_range(-1.0f32..1.0);
                Stage::Running(config.fidget_time - (dt - left))
            }
            Stage::Waiting(left) => Stage::Waiting(left - dt),
            Stage::Running(left) if left <= dt => {
                Stage::Waiting(draw(&mut rng, config.min_fidget, config.max_fidget) - (dt - left))
            }
            Stage::Running(left) => Stage::Running(left - dt),
        };

        self.rng = rng;
    }

    /// Extends the spine and lifts the shoulders over the breath.
    ///
    /// Composed rather than assigned, like every other additive layer in this
    /// crate — running it twice compounds its own breath.
    fn breathe(&self, rig: &Rig, pose: &mut Pose, breath: f32) {
        let wanted = stature(rig) * self.config.breath;
        if wanted <= 0.0 {
            return;
        }
        let wave = |phase: f32| (phase * std::f32::consts::TAU).sin();
        let chest_wave = wave(breath);
        // The diaphragm moves first and the ribcage follows.
        let belly_wave = wave(breath + BREATH_LEAD);

        let at = |zone: Zone| {
            rig.in_zone(zone)
                .first()
                .map(|&joint| (joint, rig.joints[joint].position))
        };
        let (Some((hinge, hinge_at)), Some((chest, chest_at))) =
            (at(Zone::Abdomen), at(Zone::Chest))
        else {
            return;
        };
        // **The angle is solved, not written down.** `config.breath` says how
        // far the chest travels as a share of stature, and an angle at the
        // abdomen delivers that only through the lever between the two joints —
        // so on a body with a short trunk the same angle would deliver less and
        // the field would be describing something other than what happened.
        // The crate has been here before: a constant nothing checks against the
        // body it moves is `super::gait::TRUNK_LEAN` spread down the spine,
        // delivering 2.1 degrees of the 5.5 it asked for.
        let lever = hinge_at.distance(chest_at);
        if lever <= f32::EPSILON {
            return;
        }
        let extension = wanted / lever;
        // Extension at the abdomen and a counter-flexion at the chest, so the
        // trunk lengthens through its middle without the head nodding — the
        // same bargain `super::gait::lean` strikes with the neck.
        pose.rotations[hinge] *= Quat::from_rotation_x(-extension * belly_wave * BREATH_ABDOMEN);
        pose.rotations[chest] *=
            Quat::from_rotation_x(extension * chest_wave * (1.0 - BREATH_ABDOMEN));

        // The shoulders, which is the cue that actually carries a breath on a
        // rig with no per-joint translation. Mirrored, so the two rise together
        // rather than the body shrugging one-sided, and solved against their own
        // lever for the same reason the spine is.
        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let Some(chain) = rig.limb_chain(limb) else {
                continue;
            };
            let arm = rig.joints[chain[0]]
                .position
                .distance(rig.joints[chain[1]].position);
            if arm <= f32::EPSILON {
                continue;
            }
            let side = if limb == Limb::ForeLeft { 1.0 } else { -1.0 };
            let lift = wanted * BREATH_SHOULDER / arm;
            pose.rotations[chain[0]] *= Quat::from_rotation_z(-lift * chest_wave * side);
        }
    }

    /// How far the body has drifted from centre this moment: fore-and-aft, and
    /// side to side, both in metres.
    ///
    /// Drawn from a noise field rather than a sine, because two sines at the
    /// body's own frequency trace a Lissajous figure and a body drawing one
    /// reads as a machine. Sampled against elapsed TIME rather than per frame,
    /// so the motion is smooth at any frame rate and identical at every one.
    fn drift(&self, rig: &Rig) -> (f32, f32) {
        if self.config.sway <= 0.0 {
            return (0.0, 0.0);
        }
        let rate = self.sway_rate(rig);
        if rate <= f32::EPSILON {
            return (0.0, 0.0);
        }
        let at = (self.elapsed * rate) as f64;
        // Two lanes of one field, far enough apart that neither sees the other.
        let wanted = stature(rig) * self.config.sway;
        (
            self.field.get([at, 0.0]) as f32 * wanted,
            self.field.get([at, 37.0]) as f32 * wanted,
        )
    }

    /// Tips the whole body fore-and-aft about the line through its feet.
    ///
    /// **The ankle strategy, and it is only the sagittal half of a sway.** A
    /// body standing quietly balances fore-and-aft by turning about its ankles
    /// — an inverted pendulum, which is where [`Self::sway_rate`] comes from —
    /// and the feet stay exactly put under it because they lie ON the axis
    /// being turned about. Nothing is solved and nothing can drift.
    ///
    /// **Sideways is a different mechanism and gets different treatment.** Tip
    /// a standing body about the same point laterally and one foot goes into
    /// the floor while the other leaves it, because the feet are offset ACROSS
    /// that axis. Real lateral sway is a loading strategy at the hips rather
    /// than a tilt at the ankles, so it is a translation, and it goes through
    /// [`Self::settle`] with the weight shift because the two are the same
    /// motion at two scales.
    ///
    /// **What that argument is no longer worth**, said plainly because the
    /// comment here used to claim more. A lateral roll was measured moving the
    /// feet 89 mm — but that was before [`Self::drive_on`] solved each contact
    /// back to where it rests, and with the solve in place a roll added here
    /// moves the feet by nothing at all: reintroduced as an experiment, the
    /// feet guard passed. So the split is not what keeps the soles down; the
    /// solve is. What the split still buys is a lateral sway the legs are not
    /// fighting — a roll about the ankles asks one leg to lengthen and the
    /// other to shorten, and the solve would spend every frame undoing it — and
    /// the mechanism the posturography literature actually describes.
    fn tip(&self, rig: &Rig, pose: &mut Pose, fore: f32) {
        if fore == 0.0 {
            return;
        }
        let height = stature(rig);
        if height <= f32::EPSILON {
            return;
        }
        let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
            return;
        };
        // The head is at the top of the pendulum, so the angle that moves it by
        // `fore` is `fore/height` — not `fore` over the centre of mass, which
        // is where the pendulum's PERIOD comes from but not its geometry.
        let angle = (fore / height).clamp(-0.2, 0.2);
        let tip = Quat::from_rotation_x(angle);
        // Rotating about the root and then moving the root by the amount the
        // pivot would otherwise have travelled is a rotation about the pivot,
        // exactly.
        let pivot = ground_centre(rig);
        let offset = rig.joints[root].position - pivot;
        pose.rotations[root] *= tip;
        pose.translation += tip * offset - offset;
    }

    /// Settles the body over one leg, and puts the feet back where they were.
    ///
    /// The pelvis travels **half the stance width**, which is the distance that
    /// puts its mass over the bearing foot rather than a number — a wide stance
    /// shifts further because it has further to go. The body sinks as it goes,
    /// and the sink is not chosen either: a resting leg stands at very nearly
    /// full extension (which is the straight-leg slack the gait had to learn
    /// about), so a pelvis that moved sideways without dropping would ask the
    /// far leg for reach it has not got. Dropping by the sagitta of that
    /// sideways move keeps the far hip exactly as far from its foot as it was.
    ///
    /// Then [`plant_feet_of`] puts the soles back. The free knee softens
    /// without being told to: it is now nearer its own foot than a straight leg
    /// would be, and a solve that reaches a closer goal bends.
    fn settle<F>(&self, rig: &Rig, pose: &mut Pose, side: f32, ground: &F) -> Option<Limb>
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let _ = ground;
        let Some(bearing) = self.bearing else {
            // No shift running, but the lateral sway still has to land — it is
            // the same motion at a smaller scale, and dropping it here is what
            // would leave a body swaying in one axis only.
            pose.translation.x += side;
            return None;
        };
        let share = match self.shift {
            // Eased, because a body settling onto a hip does not arrive at
            // constant speed. A smoothstep on the fraction remaining.
            Stage::Running(left) => {
                let done = 1.0 - (left / self.config.shift_time.max(f32::EPSILON)).clamp(0.0, 1.0);
                done * done * (3.0 - 2.0 * done)
            }
            Stage::Waiting(_) => 1.0,
        };
        // Coming FROM the other leg rather than from centre, when there was
        // one: a body that returned to the middle between every shift would
        // bob through neutral each time, which is not what settling looks like.
        let from = self.was.map_or(0.0, |limb| side_of(rig, limb));
        let toward = side_of(rig, bearing);
        let over = from + (toward - from) * share;

        let width = stance_width(rig);
        if width <= f32::EPSILON {
            return Some(bearing);
        }
        // See `BEARING_SHARE`: the beam's weight line, not the foot. Moving the
        // pelvis right over the bearing foot is a full transfer — measured, it
        // walked the head 10 mm outside the support polygon and moved some
        // joints at 471 mm/s, which is a body lunging rather than one settling.
        let across = over * width * 0.5 * (2.0 * BEARING_SHARE - 1.0);
        // The sagitta: how far a hip must drop for its distance to its own foot
        // to be unchanged after moving `across` sideways. Exact for a leg of
        // length `reach`, and it degenerates safely on a body with none.
        let reach = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_reach(limb))
            .fold(0.0f32, f32::max);
        let sink = if reach > across.abs() {
            reach - (reach * reach - across * across).sqrt()
        } else {
            0.0
        };
        pose.translation += Vec3::new(across + side, -sink, 0.0);

        // **And the trunk comes back the other way**, which is the whole shape
        // of the pose: a hip pushed out to one side and the shoulders held over
        // the middle. Measured over ninety seconds, the pelvis reaches 39 mm
        // across and this brings the shoulders back to 23 and the head to 16 —
        // without it all three sit at 39 and the body slides sideways in one
        // piece, which is a body stepping rather than one standing at ease.
        //
        // It is NOT what keeps the head inside the support polygon; at this
        // shift's size the head stays inside either way, and the claim that it
        // rescued containment was left over from a shift three times larger.
        // It is the same bargain `super::gait::lean` strikes with the neck, and
        // the angle is not a choice: it is whatever puts the shoulders back
        // over the point the pelvis left.
        let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
            return Some(bearing);
        };
        let Some(girdle) = rig.joints[neck].parent else {
            return Some(bearing);
        };
        let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
            return Some(bearing);
        };
        let trunk = rig.joints[girdle].position.y - rig.joints[root].position.y;
        if trunk <= f32::EPSILON {
            return Some(bearing);
        }
        let back = (across / trunk).clamp(-0.5, 0.5).asin();
        // At the joint above the pelvis, for the reason `lean` gives: the
        // pelvis carries the legs, and turning it turns them out from under the
        // solve that is about to hold the feet.
        if let Some(&hinge) = rig.in_zone(Zone::Abdomen).first() {
            pose.rotations[hinge] *= Quat::from_rotation_z(back);
            pose.rotations[neck] *= Quat::from_rotation_z(-back);
        }
        Some(bearing)
    }

    /// Rolls the shoulders, if a fidget is running.
    ///
    /// The glance is **not** applied here: it is reported on [`Idled::glance`]
    /// for a caller to hand to [`super::look_at`], because the gaze layer
    /// already spreads a turn down the chest, neck and head and already stops
    /// where a neck stops. A head turn written here would be a second answer to
    /// a question that has one.
    fn twitch(&self, rig: &Rig, pose: &mut Pose) -> bool {
        let Stage::Running(left) = self.fidget else {
            return false;
        };
        let time = self.config.fidget_time.max(f32::EPSILON);
        let done = (1.0 - left / time).clamp(0.0, 1.0);
        // One rise and fall over the fidget, so it begins and ends at nothing
        // and cannot step when it starts or stops.
        let amount = (done * std::f32::consts::PI).sin() * self.roll * SHOULDER_ROLL;
        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let Some(chain) = rig.limb_chain(limb) else {
                continue;
            };
            let side = if limb == Limb::ForeLeft { 1.0 } else { -1.0 };
            pose.rotations[chain[0]] *= Quat::from_rotation_z(-amount * side);
        }
        true
    }
}

/// How far the shoulders roll in a fidget, in radians.
///
/// Two degrees at full amplitude, and the draw that scales it runs to either
/// side of zero — so most fidgets are smaller than this and some are the other
/// way. A shoulder roll that always went the same way and always went the whole
/// distance would be the clip this replaces.
const SHOULDER_ROLL: f32 = 0.035;

/// A wait drawn between two bounds, in seconds.
fn draw(rng: &mut Pcg64Mcg, min: f32, max: f32) -> f32 {
    let (low, high) = (min.min(max).max(0.0), max.max(min).max(0.0));
    if high <= low {
        return low;
    }
    rng.random_range(low..high)
}

/// How tall the body is, in metres.
///
/// The highest joint above the ground the body was built standing on, which is
/// the same ground `crate::extremity::Extremities::build` puts the soles at.
fn stature(rig: &Rig) -> f32 {
    rig.joints
        .iter()
        .map(|joint| joint.position.y)
        .fold(0.0f32, f32::max)
}

/// How far apart the body stands, in metres.
fn stance_width(rig: &Rig) -> f32 {
    let sides: Vec<f32> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .map(|joint| rig.joints[joint].position.x)
        .collect();
    let low = sides.iter().copied().fold(f32::MAX, f32::min);
    let high = sides.iter().copied().fold(f32::MIN, f32::max);
    if sides.is_empty() { 0.0 } else { high - low }
}

/// Which side of the body a limb stands on: `+1` for the left, `-1` for the
/// right, and zero for one on the centreline.
fn side_of(rig: &Rig, limb: Limb) -> f32 {
    rig.in_zone(Zone::Extremity(limb))
        .first()
        .map_or(0.0, |&joint| rig.joints[joint].position.x.signum())
}

/// The point on the ground between the body's contacts, in the rest frame.
fn ground_centre(rig: &Rig) -> Vec3 {
    let feet: Vec<Vec3> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .map(|joint| rig.joints[joint].position)
        .collect();
    if feet.is_empty() {
        return Vec3::ZERO;
    }
    let middle = feet.iter().copied().sum::<Vec3>() / feet.len() as f32;
    Vec3::new(middle.x, 0.0, middle.z)
}

/// The gaze settings an idle's glance is meant to be spread with.
///
/// A glance is not a stare: it is narrower and softer than the default a caller
/// would use to look at something on purpose, so the chest barely joins in.
#[must_use]
pub fn glance_config() -> GazeConfig {
    GazeConfig {
        limit: 0.9,
        shares: [0.05, 0.35, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams};

    fn body(height: f32) -> Rig {
        Rig::from_skeleton(
            &HumanoidParams {
                height,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
    }

    /// Poses a body for `seconds` and hands back every frame's joint positions.
    fn run(rig: &Rig, config: IdleConfig, seconds: f32) -> Vec<Vec<Vec3>> {
        let step = 1.0 / 60.0;
        let mut idle = Idle::new(config, 0x1de);
        (0..(seconds / step) as usize)
            .map(|_| {
                let mut pose = Pose::rest(rig);
                idle.drive(rig, &mut pose, step);
                pose.forward(rig).positions
            })
            .collect()
    }

    #[test]
    fn a_standing_body_does_not_move_its_feet() {
        // **The invariant the whole layer is built around.** What it guards is
        // the contact solve in `drive_on`: without that, a body whose pelvis
        // has moved 39 mm sideways drags its feet 39.5 mm with it, because
        // `plant_feet_of` is a vertical correction and a levelling and was
        // never asked to hold a foot in place.
        //
        // **It does not guard the sway's fore-aft/lateral split**, which is
        // what the comment here used to say. Reintroducing a lateral roll
        // passes this test unchanged, because the solve takes the feet back
        // whatever the roll did. Checked, rather than assumed from the 89 mm
        // the roll cost before the solve existed.
        let rig = body(1.75);
        let rest = Pose::rest(&rig).forward(&rig).positions;
        let contacts: Vec<usize> = rig
            .ground_contacts()
            .into_iter()
            .flat_map(|limb| rig.extremity_joints(limb))
            .collect();
        let mut worst = 0.0f32;
        for frame in run(&rig, IdleConfig::default(), 90.0) {
            for &joint in &contacts {
                worst = worst.max(frame[joint].distance(rest[joint]));
            }
        }
        assert!(
            worst < 0.003,
            "a standing body's feet moved {:.1} mm",
            worst * 1000.0
        );
    }

    #[test]
    fn a_still_body_is_never_still() {
        // The other half of the same idea: feet that do not move on a body that
        // does. A pose held for even a quarter of a second reads as the
        // animation having stopped, which is the single worst thing an idle can
        // do.
        let rig = body(1.75);
        let frames = run(&rig, IdleConfig::default(), 60.0);
        let mut stalled = 0;
        let mut worst_stall = 0;
        for pair in frames.windows(2) {
            let moved = pair[0]
                .iter()
                .zip(&pair[1])
                .map(|(a, b)| a.distance(*b))
                .fold(0.0f32, f32::max);
            // A micrometre a frame is a body that is doing something.
            if moved < 1e-6 {
                stalled += 1;
                worst_stall = worst_stall.max(stalled);
            } else {
                stalled = 0;
            }
        }
        assert!(
            worst_stall < 6,
            "the body held one pose for {worst_stall} frames"
        );
    }

    #[test]
    fn a_smaller_body_breathes_faster_and_both_are_people() {
        // The check `BREATH_PERIODS` is anchored on. A rate in breaths per
        // minute would satisfy the adult and fail the child; a dimensionless
        // period predicts both. Reintroducing the defect means writing a
        // constant period here, which puts the toddler at the adult's 16.
        let idle = Idle::seeded(1);
        let adult = 60.0 / idle.breath_period(&body(1.75));
        let child = 60.0 / idle.breath_period(&body(1.0));
        assert!(
            (12.0..=20.0).contains(&adult),
            "an adult breathed {adult:.1} times a minute"
        );
        assert!(
            (18.0..=30.0).contains(&child),
            "a small child breathed {child:.1} times a minute"
        );
        assert!(child > adult * 1.2, "{child:.1} against {adult:.1}");
    }

    #[test]
    fn the_sway_is_the_pendulum_the_body_actually_is() {
        // Quiet standing is an inverted pendulum about the ankles, so its rate
        // is `sqrt(g/h)/2pi` on the body's own centre of mass — which is why
        // the sub-1 Hz band the literature reports is a consequence here rather
        // than a filter setting. A taller body sways more slowly, exactly as a
        // longer pendulum swings more slowly.
        let idle = Idle::seeded(1);
        let tall = idle.sway_rate(&body(2.1));
        let small = idle.sway_rate(&body(1.0));
        assert!(tall < small, "{tall} against {small}");
        for rate in [tall, idle.sway_rate(&body(1.75)), small] {
            assert!((0.2..1.0).contains(&rate), "swayed at {rate} Hz");
        }
    }

    #[test]
    fn the_body_stays_over_its_own_feet() {
        // A sway that leaves the support polygon is a body falling over, and a
        // weight shift that puts the head outside the outer foot is a lunge.
        //
        // **What holds this is `BEARING_SHARE`, not the counter-lean.** The
        // counter-lean was reintroduced as an experiment and this passed
        // unchanged: at a 39 mm shift the head stays inside either way, and it
        // was a shift three times that size which needed rescuing. The
        // counter-lean earns its place on the shape it makes — shoulders 23 mm
        // where the pelvis is 39 — and that is asserted below rather than here.
        let rig = body(1.75);
        let sole: Vec<f32> = rig
            .ground_contacts()
            .into_iter()
            .flat_map(|limb| rig.extremity_joints(limb))
            .map(|joint| rig.joints[joint].position.x)
            .collect();
        let edge = sole.iter().copied().fold(0.0f32, |a, b| a.max(b.abs()));
        let head = rig.in_zone(Zone::Head)[0];
        let mut furthest = 0.0f32;
        for frame in run(&rig, IdleConfig::default(), 90.0) {
            furthest = furthest.max(frame[head].x.abs());
        }
        assert!(
            furthest < edge,
            "the head reached {:.0} mm across where the feet end at {:.0}",
            furthest * 1000.0,
            edge * 1000.0
        );
    }

    #[test]
    fn the_hip_goes_out_and_the_shoulders_stay_over_the_middle() {
        // The counter-lean's own guard, written after the containment test
        // turned out not to be one: settling onto a leg is a hip pushed out
        // with the trunk brought back over it, not a body sliding sideways in
        // one piece. Reintroducing the defect — dropping the counter-lean —
        // puts the shoulders exactly where the pelvis is and fails here.
        let rig = body(1.75);
        let neck = rig.in_zone(Zone::Neck)[0];
        let girdle = rig.joints[neck].parent.expect("a girdle");
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a root");
        let rest = Pose::rest(&rig).forward(&rig).positions;
        let (mut hip, mut shoulders) = (0.0f32, 0.0f32);
        for frame in run(&rig, IdleConfig::default(), 90.0) {
            hip = hip.max((frame[root].x - rest[root].x).abs());
            shoulders = shoulders.max((frame[girdle].x - rest[girdle].x).abs());
        }
        assert!(
            hip > 0.02,
            "the pelvis barely moved: {:.1} mm",
            hip * 1000.0
        );
        assert!(
            shoulders < hip * 0.8,
            "the pelvis reached {:.1} mm across and the shoulders {:.1} — the body slid \
             sideways in one piece",
            hip * 1000.0,
            shoulders * 1000.0
        );
    }

    #[test]
    fn talking_moves_more_than_listening_and_both_still_breathe() {
        // The two variants are the same layers at different settings, which is
        // the whole reason they are parameters: they used to be two more clips
        // to author and blend between. Asserted as an ordering rather than on
        // the numbers, because the numbers are the thing that may be tuned.
        let (talk, listen) = (IdleConfig::talking(), IdleConfig::listening());
        assert!(talk.breath_rate > listen.breath_rate);
        assert!(talk.max_fidget < listen.max_fidget);
        assert!(talk.min_fidget < listen.min_fidget);
        assert!(talk.max_shift <= listen.max_shift);
        // **And neither stops swaying.** A listener goes still, but a body that
        // stops swaying entirely reads as a photograph.
        assert_eq!(talk.sway, listen.sway);
        assert!(listen.sway > 0.0);
    }

    #[test]
    fn the_same_seed_is_the_same_idle_and_a_different_one_is_not() {
        let rig = body(1.75);
        let step = 1.0 / 60.0;
        let sample = |seed: u64| {
            let mut idle = Idle::seeded(seed);
            let mut last = Pose::rest(&rig);
            for _ in 0..3000 {
                last = Pose::rest(&rig);
                idle.drive(&rig, &mut last, step);
            }
            last.forward(&rig).positions
        };
        let once = sample(7);
        assert_eq!(once, sample(7), "a seeded idle must reproduce");
        let apart = once
            .iter()
            .zip(&sample(8))
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        assert!(apart > 1e-4, "two seeds gave the same body: {apart}");
    }
}
