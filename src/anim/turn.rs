//! One axis for how sharply a body is turning, and everything that follows
//! from it.
//!
//! [`super::speed`] is the model for this and the argument is the same one. A
//! turn could have been a set of dials — a lean angle, a per-limb stride
//! scale, a pivot threshold — and every one of them would have been a number
//! nothing could check, fitted to how one body looked on one day. What is here
//! instead is a yaw rate, and four things derived from it:
//!
//! * **The path** is a circular arc of radius `v/ω`, which is not a model of a
//!   turn but the definition of one. [`super::gait::contact_offset`] walks a
//!   contact round it, and the **differential stride falls out** — the inside
//!   foot is nearer the centre so it sweeps a shorter arc, by exactly the ratio
//!   of the radii and with nothing computing a ratio.
//! * **The bank is `atan(v·ω/g)`**, which has no free parameter at all. See
//!   [`Turn::bank`].
//! * **The gait answers to the working limb**, not to the body's centre — which
//!   is what makes a **pivot in place** the same expression as a turn rather
//!   than a branch beside it. See [`Turn::working`].
//! * **A pivot is a turn of zero radius.** Nothing tests for one.
//!
//! # What is not here
//!
//! A direction of travel. Reverse and strafe are [`super::gait::Stride`]'s
//! `direction`, which this leaves at forward, and they are #242 — the seam is
//! open and the arc construction already runs around it.

use glam::Vec3;

use super::gait::{Gait, Stride};
use super::speed::{GRAVITY, Speed};
use crate::plan::Zone;
use crate::rig::Rig;

/// How sharply a body is turning.
///
/// Held as a yaw rate because that is the quantity a caller has: a chassis
/// turning, a stick pushed over, a path being followed. Everything a gait needs
/// comes off it and the speed it is travelling at, which is why every method
/// here takes a [`Speed`] — a turn means nothing on its own, and the two
/// together are what a body is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    yaw_rate: f32,
}

impl Turn {
    /// Not turning.
    pub const STRAIGHT: Self = Self { yaw_rate: 0.0 };

    /// From a yaw rate in radians per second.
    ///
    /// Positive turns toward the body's **left**, which is `+X` — and which is
    /// also the direction a positive rotation about `+Y` carries `+Z`, so this
    /// is the sign of the yaw itself rather than a convention laid over one.
    #[must_use]
    pub fn new(radians_per_second: f32) -> Self {
        Self {
            yaw_rate: radians_per_second,
        }
    }

    /// The yaw rate, in radians per second.
    #[must_use]
    pub fn yaw_rate(self) -> f32 {
        self.yaw_rate
    }

    /// Whether this is a turn at all.
    #[must_use]
    pub fn is_straight(self) -> bool {
        self.yaw_rate == 0.0
    }

    /// The radius of the arc a body at this speed turns on, in metres.
    ///
    /// Infinite for a straight line and **zero for a pivot in place**, which is
    /// the reading that says why nothing below needs to test for one: a pivot
    /// is not a different motion, it is this number reaching its floor.
    #[must_use]
    pub fn radius(self, rig: &Rig, speed: Speed) -> f32 {
        if self.yaw_rate == 0.0 {
            return f32::INFINITY;
        }
        speed.metres_per_second(rig) / self.yaw_rate.abs()
    }

    /// How far the body leans into the turn, in radians, positive toward the
    /// centre.
    ///
    /// **`atan(v·ω/g)`, and there is no constant in it.** A body going round a
    /// bend is accelerating toward the centre at `v·ω`, and the ground can only
    /// push along the line from the support to the mass — so the body has to
    /// put its mass on that line, which means inclining until the resultant of
    /// gravity and the centripetal demand runs down it. The angle that does
    /// that is the arctangent of their ratio. Nothing is fitted and nothing is
    /// preferred; a body that leaned by anything else would fall over.
    ///
    /// **`v·ω/g` is dimensionless, and that is deliberate**: it is the same
    /// kind of quantity as [`Speed`]'s Froude number, so two bodies at the same
    /// ratio are doing the same thing at their own scale and a child and a
    /// giant round the same corner get the same carriage.
    ///
    /// # Why this is not `super::gait::TRUNK_LEAN`'s shape
    ///
    /// The forward lean **had** to be a fitted constant times a pace, because a
    /// body walking at a steady speed is not accelerating forwards and there is
    /// no statics problem to solve — what a trunk does going forward is a
    /// posture, measured off people, and the literature's 2–7 degrees is the
    /// only thing that could set it. A turn is the opposite case: there *is* an
    /// acceleration, it has a size, and the lean answering it is determined. So
    /// this is not TRUNK_LEAN with a different number in it. It is the thing
    /// TRUNK_LEAN would have been if a walk had had an answer.
    ///
    /// Zero at a pivot in place, because a body turning on the spot has no
    /// centripetal demand to answer — it is not going round anything.
    #[must_use]
    pub fn bank(self, rig: &Rig, speed: Speed) -> f32 {
        (speed.metres_per_second(rig) * self.yaw_rate / GRAVITY).atan()
    }

    /// How fast the hardest-working contact is travelling over the ground.
    ///
    /// **This is what the gait's timing answers to, and it is the whole of why
    /// a pivot in place needs no special case.** A body's centre can be
    /// stationary while its feet are not: turn on the spot and the legs are
    /// still stepping, still transferring support, still doing everything a
    /// gait describes. Asking the body's own speed there gives zero, which
    /// picks [`Gait::standing`] and freezes every foot to the floor — the
    /// duty-1.0 rule that #230 put into [`super::gait::step`] and
    /// [`super::gait::crouch_at`] doing exactly its job on a body that really
    /// is moving.
    ///
    /// Asking the *foot* instead is right in both cases and is the same
    /// question. A contact at body-space `(x, ·, z)` under a body translating
    /// at `v` and yawing at `ω` moves at `(ω·z, ·, v − ω·x)`, so the outermost
    /// contact of a turn is going faster than the body and the innermost
    /// slower. On a straight line every contact returns `v` exactly and this is
    /// the speed it was given, unchanged — which is what keeps every existing
    /// walk where it was.
    #[must_use]
    pub fn working(self, rig: &Rig, speed: Speed) -> Speed {
        let travel = speed.metres_per_second(rig);
        if self.yaw_rate == 0.0 {
            return speed;
        }
        let fastest = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
            .map(|joint| {
                let at = rig.joints[joint].position;
                let across = self.yaw_rate * at.z;
                let along = travel - self.yaw_rate * at.x;
                (across * across + along * along).sqrt()
            })
            .fold(travel, f32::max);
        Speed::new(rig, fastest)
    }

    /// The gait a body of this shape turns at this speed with.
    ///
    /// [`Speed::gait`] asked of [`Self::working`], for the reason given there:
    /// the pattern and the duty are properties of how hard the legs are having
    /// to work, and on a pivot the body's own speed says they are not working
    /// at all.
    #[must_use]
    pub fn gait(self, rig: &Rig, speed: Speed) -> Gait {
        self.working(rig, speed).gait(rig)
    }

    /// How many cycles of that gait pass per second.
    ///
    /// The working contact's, so that the foot doing the most takes a step of
    /// the length a foot going that fast takes. The body's centre then covers
    /// whatever the geometry says it does, which on a turn is less — and that
    /// difference, spread between the two feet, *is* the differential stride.
    #[must_use]
    pub fn cadence(self, rig: &Rig, speed: Speed) -> f32 {
        self.working(rig, speed).cadence(rig)
    }

    /// The stride a body of this shape takes turning this sharply at this
    /// speed.
    ///
    /// `length` is the **centreline** travel per stance and `yaw` is the
    /// heading turned across the same span, which together are the screw
    /// [`super::gait::contact_offset`] walks each contact around. The lift is
    /// the working contact's, because a lift is a clearance and it is the foot
    /// that is travelling furthest that needs one — on a pivot in place the
    /// centre covers nothing at all and a lift taken from it would leave both
    /// feet grinding round on the floor.
    ///
    /// **Identical to [`Speed::stride`] at [`Self::STRAIGHT`]**, term for term
    /// rather than nearly: the working contact is then the body itself.
    #[must_use]
    pub fn stride(self, rig: &Rig, speed: Speed) -> Stride {
        let working = self.working(rig, speed);
        let cadence = working.cadence(rig);
        if cadence <= f32::EPSILON {
            return Stride::still();
        }
        let duty = working.duty();
        Stride {
            direction: Vec3::Z,
            length: speed.metres_per_second(rig) / cadence * duty,
            lift: working.stride(rig).lift,
            yaw: self.yaw_rate / cadence * duty,
        }
    }

    /// Where on its own path the body will be `cycles` from now, in body space.
    ///
    /// **For the gaze, and it is a point rather than an angle** so that
    /// [`super::look_at`] can be handed it directly — the gaze layer already
    /// spreads a turn down the chest, neck and head and already clamps it, and
    /// duplicating any of that here would give a body two opinions about where
    /// it is looking.
    ///
    /// The lead is a *distance* ahead rather than an angle, which is what makes
    /// it derived: a body walking a bend looks at where it is going, so the
    /// further round the bend that is, the further the head has turned. On a
    /// straight line it is a point straight ahead and the gaze does nothing.
    ///
    /// Raised to the height of the head the body has, so the aim is level
    /// rather than at the floor.
    #[must_use]
    pub fn aim(self, rig: &Rig, speed: Speed, cycles: f32) -> Vec3 {
        super::gait::path_ahead(
            rig,
            &self.gait(rig, speed),
            &self.stride(rig, speed),
            cycles,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::gait;
    use crate::plan::{BodyPlan, HumanoidParams};

    fn biped() -> Rig {
        Rig::from_skeleton(
            &HumanoidParams {
                height: 1.75,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
    }

    #[test]
    fn a_straight_turn_is_the_walk_that_was_already_there() {
        // Term for term, because everything this crate has ever measured was
        // measured on that walk. Reintroducing the defect means taking the
        // cadence from the body rather than from the working contact and
        // letting the two disagree by a rounding — this is the check that says
        // they do not.
        let rig = biped();
        for froude in [0.05f32, 0.2, 0.49, 1.0, 2.5] {
            let speed = Speed::from_froude(froude);
            assert_eq!(Turn::STRAIGHT.stride(&rig, speed), speed.stride(&rig));
            assert_eq!(Turn::STRAIGHT.working(&rig, speed), speed);
            assert_eq!(Turn::STRAIGHT.bank(&rig, speed), 0.0);
            assert!(Turn::STRAIGHT.radius(&rig, speed).is_infinite());
        }
    }

    #[test]
    fn the_bank_is_the_angle_that_stands_the_body_up_in_its_own_turn() {
        // Asserted against the statics rather than against a table of angles:
        // the resultant of gravity and the centripetal demand must run down the
        // body's own long axis, so the tangent of the lean is the ratio of the
        // two accelerations. A constant smuggled in anywhere would break this
        // at every speed but the one it was fitted at.
        let rig = biped();
        for metres in [0.6f32, 1.4, 3.0] {
            for degrees in [10.0f32, 45.0, 120.0] {
                let speed = Speed::new(&rig, metres);
                let turn = Turn::new(degrees.to_radians());
                let bank = turn.bank(&rig, speed);
                let centripetal = metres * degrees.to_radians();
                assert!(
                    (bank.tan() - centripetal / GRAVITY).abs() < 1e-5,
                    "at {metres} m/s and {degrees} deg/s the body leaned {} rad",
                    bank
                );
                // And it leans INTO the turn, not out of it.
                assert!(bank > 0.0);
                assert!(Turn::new(-degrees.to_radians()).bank(&rig, speed) < 0.0);
            }
        }
    }

    #[test]
    fn a_pivot_in_place_still_walks() {
        // The case that would be a branch in any other shape of this. A body
        // turning on the spot travels nowhere, so its own speed is zero and
        // `Speed::gait` hands back a standing gait whose duty-1.0 rule freezes
        // every foot to the floor. Asking the FOOT instead is what makes it a
        // walk — and reintroducing the defect means asking `speed.gait(rig)`
        // here, which fails on the first assertion.
        let rig = biped();
        let pivot = Turn::new(90.0f32.to_radians());
        let gait = pivot.gait(&rig, Speed::STILL);
        assert!(gait.duty < 1.0, "a pivoting body must lift its feet");
        assert!(!gait.has_flight(), "and must not have to run to do it");

        let stride = pivot.stride(&rig, Speed::STILL);
        assert_eq!(stride.length, 0.0, "the body itself goes nowhere");
        assert!(stride.yaw > 0.0, "but its heading turns");
        assert!(stride.lift > 0.0, "and its feet come off the floor");
        assert_eq!(pivot.radius(&rig, Speed::STILL), 0.0);
        assert_eq!(pivot.bank(&rig, Speed::STILL), 0.0, "nothing to lean into");

        // And the feet really do go opposite ways round.
        let left = rig.in_zone(Zone::Extremity(crate::Limb::HindLeft))[0];
        let right = rig.in_zone(Zone::Extremity(crate::Limb::HindRight))[0];
        let swept = |joint: usize| {
            let home = rig.joints[joint].position;
            (home + gait::contact_offset(home, &stride, gait::Phase::Stance(1.0))).z - home.z
        };
        assert!(swept(left) * swept(right) < 0.0);
    }

    #[test]
    fn the_working_contact_is_the_one_on_the_outside() {
        // And by the ratio of the radii, which is the only answer the geometry
        // allows. Taking the body's own speed instead — the defect — makes this
        // an equality and the assertion fails.
        let rig = biped();
        let speed = Speed::new(&rig, 1.2);
        let turn = Turn::new(60.0f32.to_radians());
        let working = turn.working(&rig, speed).metres_per_second(&rig);
        assert!(
            working > 1.2,
            "the outside foot must outrun the body: {working}"
        );
        let stance = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
            .map(|joint| rig.joints[joint].position.x.abs())
            .fold(0.0f32, f32::max);
        let radius = turn.radius(&rig, speed);
        let wanted = 1.2 * (radius + stance) / radius;
        assert!(
            (working - wanted).abs() < 0.02,
            "the outside foot went {working} m/s where the radii say {wanted}"
        );
    }

    #[test]
    fn a_sharper_turn_shortens_the_inside_step_and_lengthens_the_outside() {
        // The differential stride, end to end through the real constructors.
        let rig = biped();
        let speed = Speed::new(&rig, 1.0);
        let mut ratios = Vec::new();
        for degrees in [0.0f32, 20.0, 45.0, 90.0] {
            let turn = Turn::new(degrees.to_radians());
            let stride = turn.stride(&rig, speed);
            let covered = |limb| {
                let joint = rig.in_zone(Zone::Extremity(limb))[0];
                let home = rig.joints[joint].position;
                let from = home + gait::contact_offset(home, &stride, gait::Phase::Stance(0.0));
                let to = home + gait::contact_offset(home, &stride, gait::Phase::Stance(1.0));
                from.distance(to)
            };
            let inside = covered(crate::Limb::HindLeft);
            let outside = covered(crate::Limb::HindRight);
            ratios.push(inside / outside);
        }
        assert!(
            (ratios[0] - 1.0).abs() < 1e-4,
            "a straight walk must not have an inside foot: {}",
            ratios[0]
        );
        for pair in ratios.windows(2) {
            assert!(
                pair[1] < pair[0],
                "a sharper turn must widen the difference: {pair:?}"
            );
        }
    }
}
