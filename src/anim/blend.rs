//! Transitions that keep their momentum.
//!
//! A crossfade blends two poses together for the length of the transition, which
//! means evaluating both, and means a limb moving fast in the outgoing motion
//! visibly stalls as the incoming one takes over.
//!
//! Inertialization (Bollo, GDC 2018) inverts that. At the moment of transition it
//! records the *offset* between where the body was and where the new motion says
//! it should be, along with how fast that offset was changing, and then decays
//! the offset to nothing over the transition. Only the incoming pose is ever
//! evaluated, and because the offset's velocity is carried into the decay, the
//! body keeps moving the way it was moving.
//!
//! It also composes with anything: the offset is taken between two poses, and it
//! neither knows nor cares whether they came from a clip, a solver, or a gait.
//! For a system whose motion is mostly generated rather than played back, that
//! matters more than it would for a clip-driven one.

use glam::{Quat, Vec3};

use super::pose::Pose;

/// A scalar offset decaying smoothly to zero.
///
/// The quintic is chosen so the offset, its velocity, and its acceleration all
/// reach zero together at the end — anything less leaves a discontinuity that
/// reads as a flick.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Decay {
    x0: f32,
    v0: f32,
    a0: f32,
    a: f32,
    b: f32,
    c: f32,
    duration: f32,
    flipped: bool,
}

impl Decay {
    /// Builds a decay from an initial offset and how fast it was changing.
    fn new(offset: f32, velocity: f32, duration: f32) -> Self {
        // The derivation assumes the offset starts non-positive; mirroring keeps
        // one set of coefficients correct for both directions.
        let flipped = offset > 0.0;
        let (x0, v0) = if flipped {
            (-offset, -velocity)
        } else {
            (offset, velocity)
        };

        let mut duration = duration.max(0.0);
        // An offset still growing would swing wildly over a long transition;
        // shortening it bounds the overshoot.
        if v0 > 0.0 && x0 < 0.0 {
            duration = duration.min(-5.0 * x0 / v0);
        }

        if duration <= f32::EPSILON {
            return Self {
                x0,
                v0,
                a0: 0.0,
                a: 0.0,
                b: 0.0,
                c: 0.0,
                duration: 0.0,
                flipped,
            };
        }

        let t = duration;
        let a0 = (-8.0 * v0 * t - 20.0 * x0) / (t * t);
        Self {
            x0,
            v0,
            a0,
            a: -(a0 * t * t + 6.0 * v0 * t + 12.0 * x0) / (2.0 * t.powi(5)),
            b: (3.0 * a0 * t * t + 16.0 * v0 * t + 30.0 * x0) / (2.0 * t.powi(4)),
            c: -(3.0 * a0 * t * t + 12.0 * v0 * t + 20.0 * x0) / (2.0 * t.powi(3)),
            duration,
            flipped,
        }
    }

    /// The offset remaining at time `t`.
    fn at(&self, t: f32) -> f32 {
        if t >= self.duration {
            return 0.0;
        }
        let t = t.max(0.0);
        let value = self.a * t.powi(5)
            + self.b * t.powi(4)
            + self.c * t.powi(3)
            + 0.5 * self.a0 * t * t
            + self.v0 * t
            + self.x0;
        if self.flipped { -value } else { value }
    }
}

/// One joint's rotational offset, decaying about a fixed axis.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Turn {
    axis: Vec3,
    angle: Decay,
}

/// A transition in progress.
///
/// Build one with [`Inertializer::start`] at the moment of the transition, then
/// advance it each frame and let it correct whatever the new motion produces.
#[derive(Clone, Debug, PartialEq)]
pub struct Inertializer {
    joints: Vec<Turn>,
    translation: [Decay; 3],
    duration: f32,
    elapsed: f32,
}

impl Inertializer {
    /// Captures the offset between where the body is and where it is going.
    ///
    /// `previous` and `current` are the last two frames of the outgoing motion —
    /// their difference is what supplies the velocity that gets carried through.
    /// `target` is the incoming motion's pose for this same frame.
    ///
    /// Returns an already-finished transition if the poses disagree in length or
    /// `duration` is not positive, so a caller never has to special-case it.
    #[must_use]
    pub fn start(previous: &Pose, current: &Pose, target: &Pose, dt: f32, duration: f32) -> Self {
        let count = current.len().min(target.len()).min(previous.len());
        if count == 0 || duration <= 0.0 {
            return Self::finished_now();
        }

        let rate = if dt > f32::EPSILON { 1.0 / dt } else { 0.0 };
        let mut joints = Vec::with_capacity(count);

        for index in 0..count {
            let inverse_target = target.rotations[index].inverse();
            let offset = current.rotations[index] * inverse_target;
            let before = previous.rotations[index] * inverse_target;

            let (axis, angle) = signed_axis_angle(offset);
            // Measure the earlier offset about the *same* axis, so the two
            // angles are comparable and their difference is a real rate.
            let previous_angle = signed_axis_angle(before).1 * {
                let (before_axis, _) = signed_axis_angle(before);
                if before_axis.dot(axis) < 0.0 {
                    -1.0
                } else {
                    1.0
                }
            };

            joints.push(Turn {
                axis,
                angle: Decay::new(angle, (angle - previous_angle) * rate, duration),
            });
        }

        let offset = current.translation - target.translation;
        let velocity = (current.translation - previous.translation) * rate;
        Self {
            joints,
            translation: [
                Decay::new(offset.x, velocity.x, duration),
                Decay::new(offset.y, velocity.y, duration),
                Decay::new(offset.z, velocity.z, duration),
            ],
            duration,
            elapsed: 0.0,
        }
    }

    /// A transition with nothing left to do.
    #[must_use]
    pub fn finished_now() -> Self {
        Self {
            joints: Vec::new(),
            translation: [Decay::new(0.0, 0.0, 0.0); 3],
            duration: 0.0,
            elapsed: 0.0,
        }
    }

    /// Moves the transition forward by `dt` seconds.
    pub fn advance(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt.max(0.0)).min(self.duration);
    }

    /// Whether the offset has fully decayed.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.elapsed >= self.duration || self.joints.is_empty()
    }

    /// How far through the transition this is, in `0..=1`.
    #[must_use]
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).clamp(0.0, 1.0)
        }
    }

    /// Adds what remains of the offset back onto the incoming pose.
    #[must_use]
    pub fn apply(&self, target: &Pose) -> Pose {
        if self.finished() {
            return target.clone();
        }

        let mut out = target.clone();
        for (index, turn) in self.joints.iter().enumerate() {
            if index >= out.rotations.len() {
                break;
            }
            let angle = turn.angle.at(self.elapsed);
            if angle.abs() > 1e-6 {
                out.rotations[index] =
                    Quat::from_axis_angle(turn.axis, angle) * out.rotations[index];
            }
        }
        out.translation += Vec3::new(
            self.translation[0].at(self.elapsed),
            self.translation[1].at(self.elapsed),
            self.translation[2].at(self.elapsed),
        );
        out
    }
}

/// A rotation's axis and angle, with the angle signed into `-π..=π`.
///
/// A quaternion and its negation describe the same rotation, so the raw angle
/// can come back as either `θ` or `2π − θ`. Taking the short way round keeps
/// successive frames' angles comparable, which is what makes their difference a
/// meaningful velocity rather than an occasional full-turn spike.
fn signed_axis_angle(rotation: Quat) -> (Vec3, f32) {
    let rotation = if rotation.w < 0.0 {
        -rotation
    } else {
        rotation
    };
    let (axis, angle) = rotation.to_axis_angle();
    if angle > std::f32::consts::PI {
        (axis, angle - std::f32::consts::TAU)
    } else {
        (axis, angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::Rig;

    fn rig() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    /// A pose with one joint turned by `angle` about Z.
    fn turned(rig: &Rig, joint: usize, angle: f32) -> Pose {
        let mut pose = Pose::rest(rig);
        pose.rotations[joint] = Quat::from_rotation_z(angle);
        pose
    }

    #[test]
    fn a_transition_starts_where_the_body_was() {
        let rig = rig();
        let previous = turned(&rig, 3, 0.40);
        let current = turned(&rig, 3, 0.50);
        let target = turned(&rig, 3, 0.00);

        let transition = Inertializer::start(&previous, &current, &target, 1.0 / 60.0, 0.25);
        let applied = transition.apply(&target);

        let angle = applied.rotations[3].to_axis_angle().1;
        assert!(
            (angle - 0.5).abs() < 1e-3,
            "should start at the outgoing pose, got {angle}"
        );
    }

    #[test]
    fn a_transition_ends_exactly_on_the_target() {
        let rig = rig();
        let previous = turned(&rig, 3, 0.40);
        let current = turned(&rig, 3, 0.50);
        let target = turned(&rig, 3, 0.00);

        let mut transition = Inertializer::start(&previous, &current, &target, 1.0 / 60.0, 0.25);
        transition.advance(0.30);
        assert!(transition.finished());
        assert_eq!(transition.apply(&target), target);
    }

    #[test]
    fn a_still_offset_decays_without_ever_growing() {
        let rig = rig();
        let pose = turned(&rig, 3, 0.50);
        let target = Pose::rest(&rig);

        // No velocity to carry, so there is nothing to overshoot with.
        let mut transition = Inertializer::start(&pose, &pose, &target, 1.0 / 60.0, 0.3);
        let mut last = f32::INFINITY;
        for _ in 0..35 {
            let angle = transition.apply(&target).rotations[3].to_axis_angle().1;
            assert!(
                angle <= last + 1e-4,
                "the offset grew: {angle} after {last}"
            );
            last = angle;
            transition.advance(0.01);
        }
        assert!(last < 1e-3, "the offset never closed, {last} left");
    }

    #[test]
    fn an_offset_arriving_with_speed_overshoots_but_stays_bounded() {
        // Overshoot is the point, not a defect: a joint still moving carries
        // through the target rather than stopping dead on it. What must hold is
        // that the excursion is bounded and everything still settles.
        let rig = rig();
        let target = Pose::rest(&rig);
        let mut transition = Inertializer::start(
            &turned(&rig, 3, 0.40),
            &turned(&rig, 3, 0.50),
            &target,
            1.0 / 60.0,
            0.3,
        );

        let mut peak: f32 = 0.0;
        let mut last = f32::INFINITY;
        for _ in 0..40 {
            last = transition.apply(&target).rotations[3].to_axis_angle().1;
            peak = peak.max(last);
            transition.advance(0.01);
        }
        assert!(peak > 0.5, "momentum should carry past the starting offset");
        assert!(peak < 0.75, "but not run away: peaked at {peak}");
        assert!(last < 1e-3, "and it must still settle, {last} left");
    }

    #[test]
    fn momentum_carries_through_the_transition() {
        // The reason to inertialize rather than crossfade: a limb that was
        // moving keeps moving into the new pose instead of stalling at it.
        let rig = rig();
        let target = Pose::rest(&rig);

        let moving = Inertializer::start(
            &turned(&rig, 3, 0.30),
            &turned(&rig, 3, 0.50),
            &target,
            1.0 / 60.0,
            0.4,
        );
        let still = Inertializer::start(
            &turned(&rig, 3, 0.50),
            &turned(&rig, 3, 0.50),
            &target,
            1.0 / 60.0,
            0.4,
        );

        let sample = |mut transition: Inertializer| {
            transition.advance(1.0 / 60.0);
            transition.apply(&target).rotations[3].to_axis_angle().1
        };
        assert!(
            sample(moving) > sample(still),
            "a joint arriving with speed should overshoot further than one at rest"
        );
    }

    #[test]
    fn the_root_translation_settles_too() {
        let rig = rig();
        let mut current = Pose::rest(&rig);
        current.translation = Vec3::new(0.0, 0.3, 0.0);
        let previous = Pose::rest(&rig);
        let target = Pose::rest(&rig);

        let mut transition = Inertializer::start(&previous, &current, &target, 1.0 / 60.0, 0.2);
        assert!(transition.apply(&target).translation.y > 0.25);
        transition.advance(0.25);
        assert_eq!(transition.apply(&target).translation, Vec3::ZERO);
    }

    #[test]
    fn progress_runs_from_nothing_to_everything() {
        let rig = rig();
        let pose = Pose::rest(&rig);
        let mut transition = Inertializer::start(&pose, &pose, &pose, 1.0 / 60.0, 0.2);
        assert_eq!(transition.progress(), 0.0);
        transition.advance(0.1);
        assert!((transition.progress() - 0.5).abs() < 1e-5);
        transition.advance(0.2);
        assert_eq!(transition.progress(), 1.0);
    }

    #[test]
    fn a_degenerate_transition_is_simply_finished() {
        let rig = rig();
        let pose = Pose::rest(&rig);
        assert!(Inertializer::start(&pose, &pose, &pose, 1.0 / 60.0, 0.0).finished());
        assert!(Inertializer::finished_now().finished());
        assert_eq!(
            Inertializer::finished_now().apply(&pose),
            pose,
            "a finished transition passes the target through untouched"
        );
    }

    #[test]
    fn a_decay_reaches_zero_with_no_residual_motion() {
        let decay = Decay::new(-0.4, -0.9, 0.3);
        assert!((decay.at(0.0) + 0.4).abs() < 1e-5);
        assert_eq!(decay.at(0.3), 0.0);
        assert_eq!(decay.at(0.5), 0.0);

        // Velocity and acceleration land at zero together, so nothing flicks.
        let step = 1e-4;
        let velocity = (decay.at(0.3 - step) - decay.at(0.3 - 2.0 * step)) / step;
        assert!(velocity.abs() < 1e-2, "still moving at {velocity}");
    }
}
