//! What the eyes do between the places they are looking.
//!
//! A gaze that snaps from one target to the next and holds perfectly still is
//! the single clearest tell that a face is drawn rather than alive, and the
//! perceptual literature on virtual characters puts the fix in an unusual
//! place: a PROCEDURAL model carrying micro-motion is rated more natural than
//! data-driven eye motion WITHOUT it. The jitter is not a garnish on top of a
//! correct gaze; it is a large part of what reads as alive.
//!
//! Three things are modelled here, in the order they matter to a viewer.
//!
//! * **Saccades.** Eyes do not glide between targets, they jump — and the jump's
//!   duration is a function of how far it goes, which is the *main sequence*.
//!   See [`SaccadeConfig::onset`] and [`SaccadeConfig::per_radian`].
//! * **Undershoot.** A large saccade lands short and is followed by a small
//!   corrective one. It is a documented and separately-modelled effect rather
//!   than an error, and leaving it out is part of why a perfectly-aimed jump
//!   reads as mechanical. See [`SaccadeConfig::undershoot`].
//! * **Fixational jitter.** A fixating eye is never still: it drifts and is
//!   pulled back by microsaccades, and the result has a `1/f` character. See
//!   [`SaccadeConfig::jitter`].
//!
//! **What is NOT here, and why.** Pupil unrest — the pupil dilating and
//! contracting at rest — is in every list of these components and cannot be
//! drawn by this crate: the pupil is baked into the globe as vertex colour, so
//! #235's eye joints move it but cannot resize it. Doing it needs the iris on a
//! texture, which is the route #235 weighed and rejected for good reasons that
//! have not changed.
//!
//! Sources, gathered 2026-08-18 under #275:
//! *Eye Animation*, Springer Handbook of Digital Face and Body;
//! *Real-Time Conversational Gaze Synthesis for Avatars* (ACM);
//! *Saccadic undershooting in gaze generation for virtual characters*
//! (Frontiers in Virtual Reality).

use glam::Vec3;
use noise::{NoiseFn, Simplex};

use super::eye::Eyes;
use crate::anim::Pose;
use crate::rig::Rig;

/// Tuning for [`Saccades`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SaccadeConfig {
    /// The fixed cost of a saccade however short it is, in seconds.
    ///
    /// The intercept of the main sequence: even a tiny jump takes about this
    /// long, because most of it is the eye getting moving and stopping again.
    pub onset: f32,
    /// How much longer a saccade takes per radian of amplitude, in seconds.
    ///
    /// The slope of the main sequence. Together with [`Self::onset`] this makes
    /// a small glance nearly instant and a large one visibly a movement, which
    /// is the whole reason to model duration at all rather than pick one.
    pub per_radian: f32,
    /// How far short of its target a saccade lands, as a fraction of the
    /// amplitude.
    ///
    /// **A feature rather than an error.** Real saccades undershoot and then
    /// correct, and the correction is the second, small movement that makes a
    /// gaze shift read as two events rather than one slide. Zero disables it.
    pub undershoot: f32,
    /// Amplitude below which a saccade is not worth making, in radians.
    ///
    /// Under this the eyes simply follow, which is what smooth pursuit looks
    /// like at small angles and what stops a corrective saccade from provoking
    /// another one for ever.
    pub threshold: f32,
    /// How far a fixating eye wanders, in radians.
    pub jitter: f32,
    /// How fast the jitter wanders, in hertz.
    pub jitter_rate: f32,
}

impl Default for SaccadeConfig {
    fn default() -> Self {
        Self {
            // About 21 ms of onset and 2.2 ms per degree is the customary
            // linear fit to the main sequence; per radian that slope is 0.126.
            // A 20-degree gaze shift comes out at 65 ms, which is the right
            // order for a movement one cannot watch happen.
            onset: 0.021,
            per_radian: 0.126,
            // A tenth, the usual figure for a large saccade.
            undershoot: 0.10,
            // Half a degree. Below this a correction is not worth a second
            // movement and the eye just follows.
            threshold: 0.009,
            // A sixth of a degree of wander. Fixational drift runs from a few
            // arcminutes up to about half a degree; the low end of that band is
            // the conservative choice, because this is a thing a viewer should
            // feel rather than see.
            jitter: 0.003,
            // Slow enough to read as drift rather than as a shiver.
            jitter_rate: 1.7,
        }
    }
}

/// What one frame of eye motion did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Saccaded {
    /// Whether a saccade is in flight this frame.
    pub moving: bool,
    /// How far the eyes are from the target they were given, in radians.
    ///
    /// Nonzero during a saccade and after an undershoot, which is what the
    /// corrective saccade is then made of.
    pub error: f32,
}

/// What the eyes are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Holding, with jitter.
    Fixating,
    /// In flight: how far through, and how long it takes.
    Moving(f32, f32),
}

/// Drives the eyes between and during fixations.
///
/// **Seeded, but it draws nothing.** The seed picks the noise field the jitter
/// is sampled from and that is all: given the same targets, the same seed and
/// the same clock, this is a pure function of its inputs. That is a stronger
/// promise than [`crate::Blink`]'s — a blink has to decide WHEN on its own, and
/// this never does, because the thing it is reacting to is handed to it every
/// frame. There is deliberately no generator to reach for later: an eye that
/// randomly decided where to look would be fighting whatever is aiming it.
///
/// **It works in DIRECTIONS and aims through [`Eyes::look`]**, so the vergence
/// that falls out of aiming each eye from where it is survives untouched: the
/// point handed to the pair is put back at the target's own distance.
#[derive(Clone, Debug)]
pub struct Saccades {
    config: SaccadeConfig,
    elapsed: f32,
    field: Simplex,
    /// Where the eyes point now, as a unit direction in body space.
    aim: Vec3,
    /// The ends of the saccade in flight.
    from: Vec3,
    toward: Vec3,
    stage: Stage,
}

impl Saccades {
    /// Eyes with the given configuration and seed.
    #[must_use]
    pub fn new(config: SaccadeConfig, seed: u64) -> Self {
        Self {
            config,
            elapsed: 0.0,
            field: Simplex::new(seed as u32),
            aim: Vec3::Z,
            from: Vec3::Z,
            toward: Vec3::Z,
            stage: Stage::Fixating,
        }
    }

    /// Eyes with the default configuration.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self::new(SaccadeConfig::default(), seed)
    }

    /// The configuration in force.
    #[must_use]
    pub fn config(&self) -> SaccadeConfig {
        self.config
    }

    /// Aims `eyes` at `target`, the way eyes actually get there.
    ///
    /// `target` is a point in body space, as [`Eyes::look`] takes. Call it once
    /// per frame with wherever the body is trying to look; it is this that
    /// decides when to jump, how long the jump takes and what the eyes do in
    /// between.
    ///
    /// Does nothing before [`Eyes::rig`], and nothing to a pose that is not
    /// this rig's.
    ///
    /// [`Eyes::rig`]: super::eye::Eyes::rig
    pub fn drive(
        &mut self,
        rig: &Rig,
        pose: &mut Pose,
        eyes: &Eyes,
        target: Vec3,
        dt: f32,
    ) -> Saccaded {
        let idle = Saccaded {
            moving: false,
            error: 0.0,
        };
        if !pose.fits(rig) {
            return idle;
        }
        let dt = dt.max(0.0);
        self.elapsed += dt;

        // Where the pair sits, and therefore what direction the target is in.
        // The midpoint, because this layer decides ONE aim for both eyes; the
        // per-eye difference is vergence and belongs to `Eyes::look`.
        let head = rig.joints[eyes.head].position;
        let between = head + (eyes.left.pivot + eyes.right.pivot) * 0.5;
        let Some(wanted) = (target - between).try_normalize() else {
            return idle;
        };
        if self.aim.length_squared() < 0.5 {
            self.aim = wanted;
            self.from = wanted;
            self.toward = wanted;
        }

        match self.stage {
            Stage::Fixating => {
                // A target far enough from where the eyes rest is a jump. Under
                // the threshold they simply follow, which is smooth pursuit at
                // small angles and is also what stops a corrective saccade from
                // provoking another one for ever.
                let error = self.aim.angle_between(wanted);
                if error > self.config.threshold {
                    let overshoot_free = 1.0 - self.config.undershoot;
                    self.from = self.aim;
                    // Landing SHORT, deliberately: the correction that follows
                    // is the second movement that makes a gaze shift read as two
                    // events rather than one slide.
                    self.toward = self.aim.lerp(wanted, overshoot_free).normalize_or(wanted);
                    let duration = self.config.onset + self.config.per_radian * error;
                    self.stage = Stage::Moving(0.0, duration.max(f32::EPSILON));
                } else {
                    self.aim = wanted;
                }
            }
            Stage::Moving(done, duration) => {
                let done = done + dt;
                if done >= duration {
                    self.aim = self.toward;
                    self.stage = Stage::Fixating;
                } else {
                    // Eased at both ends: an eye accelerates and decelerates
                    // rather than travelling at a constant rate, which is the
                    // shape the main sequence's peak-velocity relation implies.
                    let share = done / duration;
                    let eased = share * share * (3.0 - 2.0 * share);
                    self.aim = self.from.lerp(self.toward, eased).normalize_or(self.toward);
                    self.stage = Stage::Moving(done, duration);
                }
            }
        }

        // The jitter, which runs whether or not a saccade does — an eye in
        // flight is not still either, and gating it on fixation would make the
        // motion stop dead for the duration of every jump.
        //
        // Two octaves, the second at half the amplitude and twice the rate, so
        // the sum falls off with frequency the way fixational drift does. Two
        // lanes of one field for the two axes, far enough apart that neither
        // sees the other — the same construction the idle's sway uses.
        let at = f64::from(self.elapsed * self.config.jitter_rate);
        let lane = |offset: f64| {
            (self.field.get([at, offset]) + 0.5 * self.field.get([at * 2.0, offset + 11.0])) as f32
                / 1.5
        };
        let side = self.aim.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
        let up = side.cross(self.aim).normalize_or(Vec3::Y);
        let shaken = (self.aim
            + side * (lane(0.0) * self.config.jitter)
            + up * (lane(53.0) * self.config.jitter))
            .normalize_or(self.aim);

        let distance = (target - between).length();
        eyes.look(rig, pose, between + shaken * distance);
        Saccaded {
            moving: matches!(self.stage, Stage::Moving(_, _)),
            error: self.aim.angle_between(wanted),
        }
    }

    /// Where the eyes point now, as a unit direction in body space.
    ///
    /// Without the jitter, which is a property of the frame rather than of the
    /// gaze — an instrument asking where a body is looking wants the fixation,
    /// not the shiver on top of it.
    #[must_use]
    pub fn aim(&self) -> Vec3 {
        self.aim
    }

    /// Whether a saccade is in flight.
    #[must_use]
    pub fn moving(&self) -> bool {
        matches!(self.stage, Stage::Moving(_, _))
    }
}

impl Default for Saccades {
    fn default() -> Self {
        Self::seeded(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::eye::{EyeParams, OCULAR_LIMIT};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::landmark;

    /// A rigged pair on a body, with the rig they were hung on.
    fn rigged() -> (Rig, Eyes) {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let mut rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let mut mesh = crate::build_body(
            &skeleton,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &Default::default(),
        )
        .expect("mesh");
        let params = EyeParams::default();
        let skull = crate::face::skull::Skull::measure(&mesh, &rig).expect("a skull");
        let canon = crate::face::Canon::measure(&rig, &skull, &params);
        crate::face::carve_face(&mut mesh, &rig, &canon, &Default::default());
        let mut pair = Eyes::build(&rig, &mesh, &canon, &params);
        pair.rig(&mut rig);
        (rig, pair)
    }

    /// Where the left globe points, in head-local space.
    fn aim_of(rig: &Rig, eyes: &Eyes, pose: &Pose) -> Vec3 {
        let posed = pose.forward(rig);
        let joint = eyes.left.globe_joint.expect("a rigged pair");
        (posed.rotations[eyes.head].inverse() * (posed.rotations[joint] * landmark::FORWARD))
            .normalize()
    }

    #[test]
    fn the_eyes_move_while_the_gaze_target_is_held_perfectly_still() {
        // **The guard #275 asked for, and the thing that cannot happen without
        // it.** A fixating eye is never still; an eye that is exactly still is
        // the clearest tell that a face is drawn. The target is held constant
        // for the whole run, so every millimetre of motion here is fixational
        // jitter and nothing else.
        let (rig, eyes) = rigged();
        let head = rig.joints[eyes.head].position;
        let target = head + landmark::FORWARD * 4.0;
        let mut saccades = Saccades::seeded(3);

        let mut aims = Vec::new();
        for _ in 0..600 {
            let mut pose = Pose::rest(&rig);
            saccades.drive(&rig, &mut pose, &eyes, target, 1.0 / 60.0);
            aims.push(aim_of(&rig, &eyes, &pose));
        }
        let spread = aims
            .iter()
            .map(|a| a.angle_between(aims[0]))
            .fold(0.0f32, f32::max);
        println!("fixational jitter spans {:.3} degrees", spread.to_degrees());
        assert!(
            spread > 0.0005,
            "the eyes did not move at all over ten seconds of fixation"
        );
        // And it is a drift rather than a shiver: nowhere near a saccade.
        assert!(
            spread < 0.02,
            "fixational jitter spans {:.2} degrees, which is a twitch rather than a drift",
            spread.to_degrees()
        );
    }

    #[test]
    fn a_gaze_shift_is_a_jump_of_a_length_the_distance_decides() {
        // The main sequence: a saccade's duration is a function of how far it
        // goes. Asserted as the ORDERING rather than to the millisecond,
        // because the fit is a fit — what must be true is that a big jump takes
        // measurably longer than a small one and that neither is instant.
        let (rig, eyes) = rigged();
        let head = rig.joints[eyes.head].position;
        let frames_for = |sideways: f32| {
            let mut saccades = Saccades::seeded(5);
            let mut pose = Pose::rest(&rig);
            // Settle on straight ahead first, so the jump starts from rest.
            for _ in 0..10 {
                saccades.drive(
                    &rig,
                    &mut pose,
                    &eyes,
                    head + landmark::FORWARD * 4.0,
                    1.0 / 60.0,
                );
            }
            let target = head + landmark::FORWARD * 4.0 + Vec3::X * sideways;
            let mut frames = 0;
            for _ in 0..120 {
                saccades.drive(&rig, &mut pose, &eyes, target, 1.0 / 600.0);
                if saccades.moving() {
                    frames += 1;
                }
            }
            frames
        };
        let small = frames_for(0.10);
        let large = frames_for(1.60);
        println!("saccade lasted {small} ticks for a small jump, {large} for a large one");
        assert!(small > 0, "a gaze shift happened instantly");
        assert!(
            large > small,
            "a large saccade took {large} ticks against {small} for a small one — the duration \
             is not following the amplitude"
        );
    }

    #[test]
    fn a_saccade_lands_short_and_the_correction_finishes_it() {
        // Undershoot is a feature and it has to be VISIBLE as one: the eyes
        // must be measurably off target when the first movement ends, and on
        // target after the correction. Both halves, because either alone is
        // satisfied by doing nothing.
        let (rig, eyes) = rigged();
        let head = rig.joints[eyes.head].position;
        let mut saccades = Saccades::seeded(9);
        let mut pose = Pose::rest(&rig);
        for _ in 0..10 {
            saccades.drive(
                &rig,
                &mut pose,
                &eyes,
                head + landmark::FORWARD * 4.0,
                1.0 / 60.0,
            );
        }
        let target = head + landmark::FORWARD * 4.0 + Vec3::X * 1.40;
        let mut first_landing = None;
        let mut last = Saccaded {
            moving: false,
            error: 0.0,
        };
        for _ in 0..400 {
            let now = saccades.drive(&rig, &mut pose, &eyes, target, 1.0 / 600.0);
            if last.moving && !now.moving && first_landing.is_none() {
                first_landing = Some(now.error);
            }
            last = now;
        }
        let landed = first_landing.expect("the eyes made a saccade at all");
        println!(
            "first landing was {:.2} degrees short; settled at {:.3}",
            landed.to_degrees(),
            last.error.to_degrees()
        );
        assert!(
            landed > 0.01,
            "the first saccade landed {:.3} degrees off — it is not undershooting",
            landed.to_degrees()
        );
        assert!(
            last.error < landed * 0.5,
            "the correction left {:.2} degrees against a first landing of {:.2}",
            last.error.to_degrees(),
            landed.to_degrees()
        );
    }

    #[test]
    fn micro_motion_never_pushes_an_eye_past_what_an_eye_does() {
        // The jitter is added to the aim and the aim is clamped downstream by
        // `Eyes::look` — but only if the jitter goes THROUGH it. A layer that
        // wrote the joint itself could quietly exceed the ocular limit, and at
        // a target already at the edge that is exactly where it would show.
        let (rig, eyes) = rigged();
        let head = rig.joints[eyes.head].position;
        let mut saccades = Saccades::seeded(2);
        for frame in 0..600 {
            let mut pose = Pose::rest(&rig);
            saccades.drive(
                &rig,
                &mut pose,
                &eyes,
                head + Vec3::new(3.0, 0.0, 0.6),
                1.0 / 60.0,
            );
            let off = aim_of(&rig, &eyes, &pose).angle_between(landmark::FORWARD);
            assert!(
                off <= OCULAR_LIMIT + 1e-3,
                "frame {frame}: an eye sat {:.1} degrees off centre against a limit of {:.1}",
                off.to_degrees(),
                OCULAR_LIMIT.to_degrees()
            );
        }
    }
}
