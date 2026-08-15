//! One axis for how fast a body is going, and everything that follows from it.
//!
//! Before this, a walk took a `pace` — a free multiplier on the stride with no
//! units and no relation to anything. Everything downstream inherited that: the
//! trunk lean scales with pace, the crouch scales with the stride, the gait was
//! chosen by hand, and the cadence was whatever the caller advanced the cycle
//! by. Overlands pinned the stride at `pace 1.0` and expressed speed by bending
//! the cadence alone, so a sprinting avatar took the same length of step more
//! often — which is not what a body does, and which left the trunk lean a
//! constant in the app because the lean is scaled by exactly the ratio that
//! never moved (#239, #240).
//!
//! What replaces it is one number with a physical meaning, and the rest derived:
//!
//! * **[`Speed`] is the Froude number** `v² / (g·L)`, the dimensionless speed
//!   from the gait literature. Dimensionless is the point: two bodies at the
//!   same Froude number are doing the same thing at their own scale, so a
//!   toddler and a giant get the same walk without either being special-cased,
//!   and a quadruped is covered by the same relation as a biped.
//! * **Walk or run is a threshold on it**, at `Fr ≈ 0.5`, which is where people
//!   spontaneously change over and where the inverted pendulum stops working:
//!   above it the centripetal demand of vaulting over a straight leg exceeds
//!   what gravity can supply.
//! * **Step length comes from Grieve's relation**, stated in the form that
//!   normalises per body — `step/L = 1.22 · (v/√(gL))^0.54` — so it is the
//!   *relative* step that is fitted and the metres fall out of the leg.
//! * **Cadence is then arithmetic**, not a second fit: a body covering a known
//!   distance per cycle at a known speed takes cycles per second at the rate
//!   that makes those agree.
//!
//! Nothing here is tuned against how a body looks. Every constant is a
//! published figure or a consequence of one, and the two that are neither —
//! the duty at each end of a walk — are named and anchored rather than dialled.

use glam::Vec3;

use super::gait::{Gait, RUN_DUTY, Stride};
use crate::rig::Rig;

/// Acceleration due to gravity, in metres per second squared.
///
/// Earth's. A body on another world would want this to be a parameter, and the
/// whole point of expressing speed as a Froude number is that changing it here
/// changes every relation below correctly and at once.
pub const GRAVITY: f32 = 9.81;

/// The Froude number at which a body stops walking and starts running.
///
/// **One half, and it is a prediction rather than a measurement.** Walking is
/// an inverted pendulum: the body vaults over a straight leg, which needs
/// centripetal acceleration `v²/L` pointing down at the hip, and gravity is all
/// there is to supply it. At `v² = gL` — Froude 1 — the foot would have to pull
/// the body down, and in practice people change over at about half that,
/// because the useful limit arrives before the impossible one. The literature
/// puts the preferred transition at `Fr` between 0.4 and 0.6 across a wide
/// range of leg lengths, which is the check that matters: it is the same number
/// for a child and an adult, and it would not be if it were a speed in metres.
pub const FROUDE_TRANSITION: f32 = 0.5;

/// Grieve's coefficient: relative step length at unit dimensionless speed.
pub const GRIEVE_ALPHA: f32 = 1.22;

/// Grieve's exponent, against `v/√(gL)` rather than against the Froude number
/// itself.
///
/// **Which of the two the exponent applies to is the whole difference between a
/// stride and a shuffle**, since `Fr` is the square of `v/√(gL)` and a 0.54
/// power of one is a 0.27 power of the other. Checked against the body rather
/// than taken on trust: at 1.4 m/s on a 0.9 m leg the form used here gives a
/// 0.73 m step, and an adult's step at a normal walking speed is about 0.72.
/// The other reading gives 0.49 m, which is nobody's walk.
pub const GRIEVE_BETA: f32 = 0.54;

/// Duty factor of a walk as slow as walking gets.
///
/// A body barely moving spends nearly all of the cycle on both feet. Anchored
/// at 0.65, which is about a 0.30 double-support share on two legs — the figure
/// the literature reports for a slow walk.
pub const WALK_DUTY_SLOW: f32 = 0.65;

/// Duty factor of a walk on the point of becoming a run.
///
/// 0.55, a 0.10 double-support share: the overlap has nearly closed, which is
/// what makes the change to a run available rather than violent.
pub const WALK_DUTY_FAST: f32 = 0.55;

/// Duty factor of a run at the top of the range this models.
///
/// 0.22 at `Fr = 4`, the sprint end of the literature's band. [`RUN_DUTY`] is
/// the other end, at the transition.
pub const SPRINT_DUTY: f32 = 0.22;

/// The Froude number [`SPRINT_DUTY`] is anchored at.
pub const FROUDE_SPRINT: f32 = 4.0;

/// How fast a body is travelling, as a number that means the same thing on any
/// body.
///
/// Constructed from metres per second and a body, or directly from a Froude
/// number when the caller is already thinking dimensionlessly. Everything a
/// gait needs comes off it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Speed {
    froude: f32,
}

impl Speed {
    /// Standing still.
    pub const STILL: Self = Self { froude: 0.0 };

    /// From a ground speed in metres per second, on this body.
    ///
    /// Negative speeds are travel, not a different gait: the sign belongs to
    /// the stride's direction, and how fast a body is going is how fast it is
    /// going either way.
    #[must_use]
    pub fn new(rig: &Rig, metres_per_second: f32) -> Self {
        let leg = leg_of(rig);
        if leg <= f32::EPSILON {
            return Self::STILL;
        }
        let speed = metres_per_second.abs();
        Self {
            froude: speed * speed / (GRAVITY * leg),
        }
    }

    /// From the dimensionless number itself.
    #[must_use]
    pub fn from_froude(froude: f32) -> Self {
        Self {
            froude: froude.max(0.0),
        }
    }

    /// The Froude number.
    #[must_use]
    pub fn froude(self) -> f32 {
        self.froude
    }

    /// What this is in metres per second, on this body.
    #[must_use]
    pub fn metres_per_second(self, rig: &Rig) -> f32 {
        (self.froude * GRAVITY * leg_of(rig)).sqrt()
    }

    /// Whether a body at this speed runs rather than walks.
    ///
    /// See [`FROUDE_TRANSITION`]. The threshold is on the dimensionless number
    /// and so is the same question for every body, which is the point of asking
    /// it here rather than against a speed in metres.
    #[must_use]
    pub fn is_running(self) -> bool {
        self.froude > FROUDE_TRANSITION
    }

    /// How long one step is on this body, in metres — one footfall to the next.
    ///
    /// Grieve's relation in its dimensionless form: the *relative* step length
    /// is what is fitted, and the metres come from the leg taking it. See
    /// [`GRIEVE_BETA`] for the reading of the exponent and the check on it.
    #[must_use]
    pub fn step_length(self, rig: &Rig) -> f32 {
        let leg = leg_of(rig);
        if self.froude <= 0.0 || leg <= f32::EPSILON {
            return 0.0;
        }
        leg * GRIEVE_ALPHA * self.froude.sqrt().powf(GRIEVE_BETA)
    }

    /// What share of the cycle a contact spends on the ground at this speed.
    ///
    /// Falls with speed throughout, and **steps down at the transition**, which
    /// is not a discontinuity to be smoothed away: changing from a walk to a
    /// run is a change of mechanism, and the duty factor really does drop
    /// abruptly when it happens. Interpolated linearly between the anchors
    /// named on [`WALK_DUTY_SLOW`], [`WALK_DUTY_FAST`], [`RUN_DUTY`] and
    /// [`SPRINT_DUTY`] — the anchors are the literature and the straight lines
    /// between them are this crate's, said out loud rather than dialled in.
    #[must_use]
    pub fn duty(self) -> f32 {
        if self.froude <= 0.0 {
            return 1.0;
        }
        if !self.is_running() {
            let along = (self.froude / FROUDE_TRANSITION).clamp(0.0, 1.0);
            return WALK_DUTY_SLOW + (WALK_DUTY_FAST - WALK_DUTY_SLOW) * along;
        }
        let span = (FROUDE_SPRINT - FROUDE_TRANSITION).max(f32::EPSILON);
        let along = ((self.froude - FROUDE_TRANSITION) / span).clamp(0.0, 1.0);
        RUN_DUTY + (SPRINT_DUTY - RUN_DUTY) * along
    }

    /// The gait a body of this shape moves with at this speed.
    ///
    /// The pattern is the body's — a trot on four legs, a wave on anything
    /// else, both from [`Gait::natural`] — and the duty is this speed's. What
    /// the caller no longer does is choose between walking and running, which
    /// was never a preference: it is a threshold, and a body asked to walk
    /// above it either flails or slides.
    #[must_use]
    pub fn gait(self, rig: &Rig) -> Gait {
        if self.froude <= 0.0 {
            return Gait::standing(rig);
        }
        Gait {
            duty: self.duty(),
            ..Gait::natural(rig)
        }
    }

    /// How far the body travels in one full cycle of that gait, in metres.
    ///
    /// One cycle is one step per **support transfer**, which is not the same as
    /// one per contact: a trot puts down diagonal pairs, so a four-legged body
    /// lands four feet in two events and covers two steps a cycle where a
    /// four-legged wave covers four. See [`Gait::footfalls`], which is where
    /// that is counted — taking the contact count instead gives a trotting body
    /// twice the cadence it needs.
    #[must_use]
    pub fn cycle_length(self, rig: &Rig) -> f32 {
        self.step_length(rig) * self.gait(rig).footfalls() as f32
    }

    /// How many cycles of the gait pass per second at this speed.
    ///
    /// **Arithmetic, not a second fit.** A body covering [`Self::cycle_length`]
    /// per cycle at this many metres per second takes cycles per second at the
    /// only rate that makes those two agree — which is what stops the stride
    /// and the cadence from being able to disagree at all. Overlands used to
    /// hold the stride still and bend this alone; the result was a body taking
    /// a stroller's step at a sprinter's rate.
    #[must_use]
    pub fn cadence(self, rig: &Rig) -> f32 {
        let per_cycle = self.cycle_length(rig);
        if per_cycle <= f32::EPSILON {
            return 0.0;
        }
        self.metres_per_second(rig) / per_cycle
    }

    /// The stride a body of this shape takes at this speed.
    ///
    /// [`Stride::length`] is a contact's **excursion** — how far it travels
    /// backwards under the body across one stance — which is the body's travel
    /// per cycle times the share of the cycle the foot is down. That is the
    /// quantity [`super::gait::contact_offset`] slides the foot along, and
    /// stating it here from the speed is what ties it to the cadence.
    #[must_use]
    pub fn stride(self, rig: &Rig) -> Stride {
        let length = self.cycle_length(rig) * self.duty();
        Stride {
            direction: Vec3::Z,
            length,
            lift: length * LIFT_OF_STRIDE,
        }
    }
}

/// How high a contact lifts at the top of its swing, as a share of its
/// excursion.
///
/// Taken from the ratio [`Stride::for_body`] already used — a lift of 0.12 of
/// reach against a length of 0.70 of it — so a stride derived from speed lifts
/// its feet exactly as the crate's own walk always did, rather than acquiring a
/// second opinion about toe clearance along the way.
///
/// [`Stride::for_body`]: super::gait::Stride::for_body
const LIFT_OF_STRIDE: f32 = 0.12 / 0.70;

/// The reach of the longest leg a body stands on, in metres.
///
/// The same measure [`Stride::for_body`] and [`super::gait::crouch_for`] scale
/// by: the leg that takes the step, not the height of the body it is under,
/// since the same height can be mostly leg or mostly torso.
///
/// [`Stride::for_body`]: super::gait::Stride::for_body
fn leg_of(rig: &Rig) -> f32 {
    rig.ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::gait;
    use crate::anim::ground::Ground;
    use crate::anim::pose::Pose;
    use crate::plan::{BodyPlan, HumanoidParams, Zone};

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

    fn biped() -> Rig {
        body(1.75)
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(
            &crate::plan::QuadrupedParams::default().skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
    }

    #[test]
    fn a_step_is_the_length_the_body_taking_it_would_take() {
        // **The check that catches the exponent being read against the wrong
        // quantity.** `Fr` is the square of `v/sqrt(gL)`, so a 0.54 power of one
        // is a 0.27 power of the other, and both readings look equally
        // plausible written down. On a 0.9 m leg at 1.4 m/s — an adult at a
        // normal walking speed — the form used here gives 0.73 m and an adult's
        // step is about 0.72. The other reading gives 0.49 m.
        let leg = 0.9f32;
        let froude = 1.4 * 1.4 / (GRAVITY * leg);
        let step = leg * GRIEVE_ALPHA * froude.sqrt().powf(GRIEVE_BETA);
        assert!(
            (step - 0.72).abs() < 0.05,
            "a 0.9 m leg at 1.4 m/s took a {step:.3} m step; an adult takes about 0.72"
        );
    }

    #[test]
    fn the_same_froude_number_is_the_same_walk_on_any_body() {
        // The whole reason speed is expressed dimensionlessly: two bodies at the
        // same Froude number are doing the same thing at their own scale. What
        // must match is every *relative* quantity — the step against the leg,
        // the excursion against the leg — and what must not is the metres.
        let (small, large) = (body(1.2), body(2.1));
        let speed = Speed::from_froude(0.25);
        let relative = |rig: &Rig| {
            let leg = leg_of(rig);
            (
                speed.step_length(rig) / leg,
                speed.stride(rig).length / leg,
                // Cadence has units of 1/time, so its dimensionless form carries
                // the body's own time scale, sqrt(L/g).
                speed.cadence(rig) * (leg / GRAVITY).sqrt(),
            )
        };
        let (a_step, a_stride, a_cadence) = relative(&small);
        let (b_step, b_stride, b_cadence) = relative(&large);
        assert!((a_step - b_step).abs() < 1e-4, "{a_step} against {b_step}");
        assert!((a_stride - b_stride).abs() < 1e-4);
        assert!((a_cadence - b_cadence).abs() < 1e-4);
        // And the metres differ, or nothing has been scaled at all.
        assert!(speed.step_length(&large) > speed.step_length(&small) * 1.3);
        assert!(leg_of(&small) < leg_of(&large));
    }

    #[test]
    fn the_transition_is_a_froude_number_and_not_a_speed() {
        // A child does not run at the same metres per second an adult does, and
        // a threshold written in metres would say otherwise. Written on the
        // dimensionless number, the changeover speed scales as the square root
        // of the leg for free — which is the check that the number is doing
        // physical work rather than being a magic constant.
        let (small, large) = (body(1.2), body(2.1));
        let changeover = |rig: &Rig| (FROUDE_TRANSITION * GRAVITY * leg_of(rig)).sqrt();
        let ratio = changeover(&large) / changeover(&small);
        let legs = (leg_of(&large) / leg_of(&small)).sqrt();
        assert!(
            (ratio - legs).abs() < 1e-4,
            "the transition speed should scale as sqrt(leg): {ratio:.4} against {legs:.4}"
        );

        for rig in [&small, &large] {
            let at = changeover(rig);
            assert!(!Speed::new(rig, at * 0.99).is_running());
            assert!(Speed::new(rig, at * 1.01).is_running());
        }
        // And on an adult-sized leg it lands where people actually change over.
        let human = (FROUDE_TRANSITION * GRAVITY * 0.9f32).sqrt();
        assert!(
            (human - 2.0).abs() < 0.2,
            "an adult should change to a run near 2 m/s, got {human:.2}"
        );
    }

    #[test]
    fn the_cadence_and_the_stride_cannot_disagree_about_the_speed() {
        // The defect this axis exists to remove: overlands pinned the stride and
        // bent the cadence, so the body took a stroller's step at a sprinter's
        // rate. Deriving one from the other makes that unrepresentable.
        let rig = biped();
        for metres in [0.5f32, 1.0, 1.4, 2.0, 3.0, 6.0] {
            let speed = Speed::new(&rig, metres);
            let travelled = speed.cadence(&rig) * speed.cycle_length(&rig);
            assert!(
                (travelled - metres).abs() < 1e-3,
                "at {metres} m/s the gait covers {travelled} m/s"
            );
            assert!((speed.metres_per_second(&rig) - metres).abs() < 1e-3);
        }
    }

    #[test]
    fn a_body_at_speed_is_never_asked_which_gait_it_is_in() {
        // Walking above the transition is not a preference a caller should be
        // able to express: the pendulum has stopped working and the body either
        // flails or slides. The gait comes off the speed.
        let rig = biped();
        let at = |froude: f32| Speed::from_froude(froude).gait(&rig);
        assert!(!at(0.2).has_flight(), "a stroll must not leave the ground");
        assert!(!at(0.49).has_flight());
        assert!(at(0.51).has_flight(), "past the transition a body runs");
        assert!(at(3.0).has_flight());
        assert_eq!(Speed::STILL.gait(&rig).duty, 1.0, "a still body stands");
    }

    #[test]
    fn the_duty_falls_with_speed_and_steps_down_where_the_gait_changes() {
        // Falling throughout, because a faster body spends less of each cycle
        // on the ground — and stepping down at the transition, which is not a
        // discontinuity to be smoothed away but a change of mechanism.
        let mut last = f32::MAX;
        for sample in 1..400 {
            let froude = sample as f32 / 100.0;
            let duty = Speed::from_froude(froude).duty();
            assert!(duty <= last + 1e-6, "duty rose at Fr {froude}");
            last = duty;
        }
        let below = Speed::from_froude(FROUDE_TRANSITION - 1e-3).duty();
        let above = Speed::from_froude(FROUDE_TRANSITION + 1e-3).duty();
        assert!(
            below - above > 0.15,
            "the walk-run change should drop the duty sharply: {below:.3} to {above:.3}"
        );
        assert!((below - WALK_DUTY_FAST).abs() < 1e-2);
        assert!((above - RUN_DUTY).abs() < 1e-2);
    }

    #[test]
    fn the_body_the_gait_poses_travels_at_the_speed_it_was_given() {
        // **End to end, through the gait rather than around it.** Everything
        // above is arithmetic about the numbers; this checks that the pose the
        // numbers produce actually moves the body that far. A planted contact is
        // stationary in the world while the body travels over it, so how far it
        // slides back under the hip across one stance, divided by how long that
        // stance lasts, IS the ground speed.
        let rig = biped();
        let ground = |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)));
        // How far the extremity hangs below the joint the solve actually aims,
        // against the leg doing the aiming. It bounds the overshoot below: the
        // solve corrects for that hang with an offset it read before turning
        // the joint it read it from, so the foot travels further than the goal
        // by that ratio (#254). Measured at 1.123 on this body, constant across
        // the whole speed range — which is what says it is one systematic
        // factor and not an error in the axis.
        let hang = {
            let limb = rig.ground_contacts()[0];
            let chain = rig.limb_chain(limb).expect("a leg");
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            rig.joints[chain[2]]
                .position
                .distance(rig.joints[foot].position)
                / rig.limb_reach(limb).expect("reach")
        };
        let mut ratios = Vec::new();
        for metres in [0.8f32, 1.4, 2.5] {
            let speed = Speed::new(&rig, metres);
            let gait = speed.gait(&rig);
            let stride = speed.stride(&rig);
            let limb = gait.limbs[0];
            let foot = rig.in_zone(Zone::Extremity(limb))[0];

            // Just inside each end of that contact's stance, so neither sample
            // lands on the instant it changes state.
            let at = |cycle: f32| {
                let mut pose = Pose::rest(&rig);
                gait::step(&rig, &mut pose, &gait, &stride, cycle, ground);
                pose.forward(&rig).positions[foot].z
            };
            let (from, to) = (gait.duty * 0.25, gait.duty * 0.75);
            let slid = at(from) - at(to);
            let _ = stride;
            let seconds = (to - from) / speed.cadence(&rig);
            let travelled = slid / seconds;
            let ratio = travelled / metres;
            assert!(
                ratio > 0.98 && ratio < 1.0 + hang + 0.02,
                "asked {metres:.2} m/s and the body travelled {travelled:.2}; the solve's \
                 own overshoot accounts for at most {:.1}%",
                hang * 100.0
            );
            ratios.push(ratio);
        }
        // **And it is the same factor at every speed**, across the walk and the
        // run alike. That is the axis's own claim: a body asked for twice the
        // speed goes twice as fast, whatever constant sits in front. An error
        // in the relation between stride, duty and cadence would show up here
        // as a ratio that moved with speed.
        let (low, high) = ratios
            .iter()
            .fold((f32::MAX, f32::MIN), |(low, high), &at| {
                (low.min(at), high.max(at))
            });
        assert!(
            high - low < 0.01,
            "the travelled-to-asked ratio moved from {low:.4} to {high:.4} across the range"
        );
    }

    #[test]
    fn a_cycle_is_one_footfall_per_contact_and_the_distance_says_so() {
        // Why [`Speed::cycle_length`] multiplies by the number of contacts. A
        // step is one footfall to the next; a cycle is every foot having taken
        // one. Counting the footfalls off the gait rather than asserting the
        // arithmetic against itself is what makes this a check — a four-legged
        // body covers four steps a cycle and a two-legged one covers two, and a
        // relation that ignored that would give a quadruped half the cadence it
        // needs at the same speed.
        for rig in [biped(), quadruped()] {
            let speed = Speed::from_froude(0.25);
            let gait = speed.gait(&rig);
            const STEPS: usize = 2000;
            let footfalls = (0..STEPS)
                .filter(|&sample| {
                    let now = sample as f32 / STEPS as f32;
                    let then = (sample + STEPS - 1) as f32 / STEPS as f32;
                    (0..gait.len()).any(|index| {
                        gait.phase(index, now).is_stance() && !gait.phase(index, then).is_stance()
                    })
                })
                .count();
            assert_eq!(
                footfalls,
                gait.footfalls(),
                "the gait disagrees with itself about how often support transfers"
            );
            // Two on a biped's wave AND two on a quadruped's trot, which is the
            // case that separates support transfers from feet.
            assert_eq!(footfalls, 2, "both default gaits transfer support twice");
            assert!(
                (speed.cycle_length(&rig) - speed.step_length(&rig) * footfalls as f32).abs()
                    < 1e-5,
                "the distance per cycle is not the steps taken in it"
            );
        }
    }

    #[test]
    fn standing_still_is_standing_still_and_not_a_very_slow_walk() {
        let rig = biped();
        let still = Speed::new(&rig, 0.0);
        assert_eq!(still.froude(), 0.0);
        assert_eq!(still.step_length(&rig), 0.0);
        assert_eq!(still.cadence(&rig), 0.0);
        assert_eq!(still.stride(&rig).length, 0.0);
        assert_eq!(still.duty(), 1.0);
        assert!(!still.gait(&rig).has_flight());
        // Backwards is a direction, not a different gait.
        assert_eq!(Speed::new(&rig, -1.4), Speed::new(&rig, 1.4));
    }
}
