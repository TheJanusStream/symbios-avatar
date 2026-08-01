//! Walking, for whatever number of legs a body has.
//!
//! A gait is two numbers per body: a **phase offset** for each ground contact,
//! saying when in the cycle it lifts, and a **duty factor**, saying what
//! fraction of the cycle it spends down. Everything else — a biped's walk, a
//! horse's trot, a wave gait rippling down a centipede — falls out of those.
//! That is what lets one implementation serve bodies whose leg counts differ,
//! which no library of authored walk cycles can do.
//!
//! Feet are placed by *goal*, not by joint angle: this contact belongs here now.
//! Where "here" is comes from the body's own rest stance and its stride, so a
//! short body takes short steps and a long-legged one takes long ones without
//! anything being retuned. Inverse kinematics turns the goals back into a pose.
//!
//! The cycle itself is the caller's to drive, because how fast a body walks is a
//! question about the world — speed, terrain, intent — and this crate does not
//! know about any of that.

use glam::Vec3;

use super::ground::solve_contact;
use super::pose::Pose;
use crate::plan::Limb;
use crate::rig::Rig;

/// Where one contact is in the cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// On the ground, holding position while the body travels over it.
    /// Carries progress through the stance, in `0..=1`.
    Stance(f32),
    /// In the air, swinging forward. Carries progress through the swing.
    Swing(f32),
}

impl Phase {
    /// Whether this contact is carrying the body.
    #[must_use]
    pub fn is_stance(self) -> bool {
        matches!(self, Phase::Stance(_))
    }

    /// How far through whichever phase this is, in `0..=1`.
    #[must_use]
    pub fn progress(self) -> f32 {
        match self {
            Phase::Stance(t) | Phase::Swing(t) => t,
        }
    }
}

/// When each of a body's contacts lifts, and for how long.
#[derive(Clone, Debug, PartialEq)]
pub struct Gait {
    /// The contacts this gait drives, in the rig's own order.
    pub limbs: Vec<Limb>,
    /// Each contact's offset into the cycle, in `0..1`.
    pub offsets: Vec<f32>,
    /// Fraction of the cycle a contact spends on the ground.
    ///
    /// Above `0.5` more than half the feet are always down, which is what makes
    /// a gait a walk; below it, the body has airborne moments and is running.
    pub duty: f32,
}

impl Gait {
    /// A body standing still: every contact down, always.
    #[must_use]
    pub fn standing(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        Self {
            offsets: vec![0.0; limbs.len()],
            limbs,
            duty: 1.0,
        }
    }

    /// Contacts lifting one after another, evenly spread around the cycle.
    ///
    /// The general answer for any number of legs, and for two it is simply a
    /// walk. Duty is set so exactly one contact is airborne at a time, which is
    /// the most stable way to move a body of any leg count.
    #[must_use]
    pub fn wave(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        let count = limbs.len().max(1);
        Self {
            offsets: (0..limbs.len())
                .map(|index| index as f32 / count as f32)
                .collect(),
            limbs,
            duty: (1.0 - 1.0 / count as f32).max(0.5),
        }
    }

    /// Diagonal pairs moving together — a horse's trot.
    ///
    /// Falls back to a wave gait on a body that is not four-legged, since
    /// "diagonal" means nothing without four corners.
    #[must_use]
    pub fn trot(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        if limbs.len() != 4 {
            return Self::wave(rig);
        }
        Self {
            offsets: limbs
                .iter()
                .map(|limb| {
                    // Front-left with hind-right, front-right with hind-left.
                    if limb.is_fore() == limb.is_left() {
                        0.0
                    } else {
                        0.5
                    }
                })
                .collect(),
            limbs,
            duty: 0.5,
        }
    }

    /// The gait a body of this shape moves with by default.
    #[must_use]
    pub fn natural(rig: &Rig) -> Self {
        if rig.ground_contacts().len() == 4 {
            Self::trot(rig)
        } else {
            Self::wave(rig)
        }
    }

    /// How many contacts this gait drives.
    #[must_use]
    pub fn len(&self) -> usize {
        self.limbs.len()
    }

    /// Whether the gait drives nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Where contact `index` is at this point in the cycle.
    #[must_use]
    pub fn phase(&self, index: usize, cycle: f32) -> Phase {
        let duty = self.duty.clamp(0.0, 1.0);
        let local = (cycle - self.offsets.get(index).copied().unwrap_or(0.0)).rem_euclid(1.0);

        if duty >= 1.0 {
            return Phase::Stance(local);
        }
        if local < duty {
            Phase::Stance(if duty > 0.0 { local / duty } else { 0.0 })
        } else {
            Phase::Swing((local - duty) / (1.0 - duty))
        }
    }

    /// How many contacts are on the ground at this point in the cycle.
    ///
    /// Never reaching zero is what separates a walk from a run, and reaching it
    /// accidentally is what makes a gait look like a stumble.
    #[must_use]
    pub fn grounded(&self, cycle: f32) -> usize {
        (0..self.len())
            .filter(|&index| self.phase(index, cycle).is_stance())
            .count()
    }
}

/// How much further than strictly necessary a body sinks to take its stride.
///
/// The margin keeps a visible bend in the knee and the solver clear of the
/// singularity at full extension. It scales with the sinking rather than being
/// added to it, so a body that is standing still does not crouch at all.
pub const CROUCH_MARGIN: f32 = 1.15;

/// How far and how high a body steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stride {
    /// Direction of travel in body space. `+Z` is forward.
    pub direction: Vec3,
    /// Ground covered per cycle, in metres.
    pub length: f32,
    /// How high a contact lifts at the top of its swing, in metres.
    pub lift: f32,
}

impl Stride {
    /// A stride scaled to the body walking it.
    ///
    /// Scaled by how far the legs reach, not by how tall the body is: the same
    /// height can be mostly leg or mostly torso, and it is the leg that takes the
    /// step. Expressing it this way is what lets one stride description suit a
    /// child and a giant.
    #[must_use]
    pub fn for_body(rig: &Rig, pace: f32) -> Self {
        let reach = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_reach(limb))
            .fold(0.0f32, f32::max)
            .max(f32::EPSILON);
        Self {
            direction: Vec3::Z,
            length: reach * 0.7 * pace.max(0.0),
            lift: reach * 0.12 * pace.max(0.0),
        }
    }

    /// Standing still.
    #[must_use]
    pub fn still() -> Self {
        Self {
            direction: Vec3::Z,
            length: 0.0,
            lift: 0.0,
        }
    }
}

/// How far a body must sink for its stride to stay within its legs' reach.
///
/// A leg standing straight has no slack: swinging its foot forward by half a
/// stride puts the goal further from the hip than the leg is long. Bodies solve
/// this by sinking as they stride, and so does this — without it every step is
/// out of reach and the legs merely stretch toward the ground.
///
/// Solved per limb against its *actual* rest geometry rather than from stride
/// length alone, because a limb's contact is not generally beneath its hip: a
/// quadruped's feet already sit well forward of the joints that carry them, and
/// assuming otherwise under-crouches every four-legged body.
#[must_use]
pub fn crouch_for(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let half = stride.length * 0.5;

    gait.limbs
        .iter()
        .filter_map(|&limb| {
            let reach = rig.limb_reach(limb)?;
            let chain = rig.limb_chain(limb)?;
            // Measured to the joint the chain actually reaches, not to the
            // contact hanging off it — the same distinction the solve makes.
            let offset = rig.joints[chain[2]].position - rig.joints[chain[0]].position;

            // Both ends of the stride, since a contact may start forward of its
            // hip and only the further extreme matters.
            [half, -half]
                .into_iter()
                .map(|swing| {
                    let reaching = offset + stride.direction * swing;
                    let horizontal = reaching.length_squared() - reaching.y * reaching.y;
                    // Sinking by `c` shortens the hip-to-goal distance by moving
                    // the hip down toward the goal's height.
                    let needed = -reaching.y - (reach * reach - horizontal).max(0.0).sqrt();
                    needed.max(0.0) * CROUCH_MARGIN
                })
                .fold(f32::NEG_INFINITY, f32::max)
                .into()
        })
        .fold(0.0f32, f32::max)
}

/// Where a contact belongs, relative to where it rests.
///
/// During stance the foot holds still in the world while the body travels over
/// it, which in body space is a slide backwards. During swing it arcs forward
/// and up, and back down to meet the ground at the front of the step.
#[must_use]
pub fn contact_offset(stride: &Stride, phase: Phase) -> Vec3 {
    let half = stride.length * 0.5;
    match phase {
        Phase::Stance(t) => stride.direction * (half - stride.length * t),
        Phase::Swing(t) => {
            let along = stride.direction * (stride.length * t - half);
            along + Vec3::Y * (stride.lift * (t * std::f32::consts::PI).sin())
        }
    }
}

/// What one step of a gait did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Steps {
    /// Contacts currently carrying the body.
    pub stance: Vec<Limb>,
    /// Contacts currently in the air.
    pub swing: Vec<Limb>,
    /// Contacts whose goal was out of reach.
    pub straining: Vec<Limb>,
    /// How far the body sank to keep its stride within reach, in metres.
    pub crouch: f32,
}

impl Steps {
    /// Whether every contact reached where the gait wanted it.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.straining.is_empty()
    }
}

/// Poses a body's legs for one moment of a gait.
///
/// `cycle` runs `0..1` and wraps; the caller advances it, because how fast a
/// body walks depends on the world rather than on the body.
///
/// The result reports which contacts are down, which a caller passes to
/// [`super::plant_feet_of`] to settle them onto real terrain — a swinging foot
/// must not be dragged to the ground it is travelling over.
pub fn step(rig: &Rig, pose: &mut Pose, gait: &Gait, stride: &Stride, cycle: f32) -> Steps {
    let mut steps = Steps::default();
    if !pose.fits(rig) {
        return steps;
    }

    // Sink far enough that a foot at the far end of its stride is still within
    // the leg's reach.
    steps.crouch = crouch_for(rig, gait, stride);
    pose.translation.y -= steps.crouch;

    for (index, &limb) in gait.limbs.iter().enumerate() {
        let Some(home) = home_of(rig, limb) else {
            continue;
        };
        let phase = gait.phase(index, cycle);
        let target = home + contact_offset(stride, phase);

        if phase.is_stance() {
            steps.stance.push(limb);
        } else {
            steps.swing.push(limb);
        }
        if !solve_contact(rig, pose, limb, target) {
            steps.straining.push(limb);
        }
    }

    steps
}

/// Where a limb's contact rests when the body is standing.
fn home_of(rig: &Rig, limb: Limb) -> Option<Vec3> {
    let joint = *rig.in_zone(crate::plan::Zone::Extremity(limb)).first()?;
    Some(rig.joints[joint].position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams, Zone};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs")
    }

    /// The world height of one contact in the given pose.
    fn contact_height(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let joint = rig.in_zone(Zone::Extremity(limb))[0];
        pose.forward(rig).positions[joint].y
    }

    #[test]
    fn a_gait_covers_exactly_the_body_that_carries_it() {
        assert_eq!(Gait::natural(&biped()).len(), 2);
        assert_eq!(Gait::natural(&quadruped()).len(), 4);
        assert_eq!(Gait::standing(&biped()).limbs, biped().ground_contacts());
    }

    #[test]
    fn standing_never_lifts_a_foot() {
        let gait = Gait::standing(&biped());
        for step in 0..20 {
            let cycle = step as f32 / 20.0;
            assert_eq!(gait.grounded(cycle), gait.len(), "a foot left the ground");
        }
    }

    #[test]
    fn a_walking_body_always_has_a_foot_down() {
        // The property that separates a walk from a stumble. It has to hold at
        // every point of the cycle, not merely on average.
        for rig in [biped(), quadruped()] {
            let gait = Gait::wave(&rig);
            for step in 0..120 {
                let cycle = step as f32 / 120.0;
                assert!(
                    gait.grounded(cycle) >= 1,
                    "{} contacts: nothing on the ground at {cycle}",
                    gait.len()
                );
            }
        }
    }

    #[test]
    fn a_wave_gait_lifts_one_contact_at_a_time() {
        let rig = quadruped();
        let gait = Gait::wave(&rig);
        for step in 0..120 {
            let cycle = step as f32 / 120.0;
            let airborne = gait.len() - gait.grounded(cycle);
            assert!(airborne <= 1, "{airborne} feet airborne at {cycle}");
        }
    }

    #[test]
    fn a_trot_moves_diagonal_pairs_together() {
        let rig = quadruped();
        let gait = Gait::trot(&rig);
        let offset_of = |limb: Limb| {
            let index = gait.limbs.iter().position(|&l| l == limb).expect("limb");
            gait.offsets[index]
        };

        assert_eq!(offset_of(Limb::ForeLeft), offset_of(Limb::HindRight));
        assert_eq!(offset_of(Limb::ForeRight), offset_of(Limb::HindLeft));
        assert_ne!(offset_of(Limb::ForeLeft), offset_of(Limb::ForeRight));
    }

    #[test]
    fn a_trot_falls_back_to_a_wave_on_a_body_without_four_corners() {
        let rig = biped();
        assert_eq!(Gait::trot(&rig), Gait::wave(&rig));
    }

    #[test]
    fn phases_run_stance_then_swing_and_wrap() {
        let gait = Gait {
            limbs: vec![Limb::HindLeft],
            offsets: vec![0.0],
            duty: 0.6,
        };
        let close = |a: Phase, b: Phase| match (a, b) {
            (Phase::Stance(x), Phase::Stance(y)) | (Phase::Swing(x), Phase::Swing(y)) => {
                (x - y).abs() < 1e-4
            }
            _ => false,
        };
        assert!(close(gait.phase(0, 0.0), Phase::Stance(0.0)));
        assert!(close(gait.phase(0, 0.3), Phase::Stance(0.5)));
        assert!(close(gait.phase(0, 0.6), Phase::Swing(0.0)));
        assert!(close(gait.phase(0, 0.8), Phase::Swing(0.5)));
        // A cycle later is the same place.
        assert!(close(gait.phase(0, 1.3), gait.phase(0, 0.3)));
        assert!(close(gait.phase(0, -0.7), gait.phase(0, 0.3)));
    }

    #[test]
    fn a_stance_foot_travels_backwards_and_a_swing_foot_lifts() {
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.8,
            lift: 0.1,
        };

        let early = contact_offset(&stride, Phase::Stance(0.0));
        let late = contact_offset(&stride, Phase::Stance(1.0));
        assert!(early.z > late.z, "the body travels over a planted foot");
        assert_eq!(early.y, 0.0, "a planted foot stays down");

        let peak = contact_offset(&stride, Phase::Swing(0.5));
        assert!(
            (peak.y - 0.1).abs() < 1e-5,
            "the swing should reach its lift"
        );
        assert!(contact_offset(&stride, Phase::Swing(0.0)).y.abs() < 1e-5);
        assert!(contact_offset(&stride, Phase::Swing(1.0)).y.abs() < 1e-5);
    }

    #[test]
    fn a_step_ends_where_the_next_one_starts() {
        // Stance must hand off to swing without a jump, or the foot teleports
        // once per cycle.
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.8,
            lift: 0.1,
        };
        let handoff = contact_offset(&stride, Phase::Stance(1.0));
        let pickup = contact_offset(&stride, Phase::Swing(0.0));
        assert!(handoff.distance(pickup) < 1e-5, "{handoff:?} vs {pickup:?}");

        let landing = contact_offset(&stride, Phase::Swing(1.0));
        let plant = contact_offset(&stride, Phase::Stance(0.0));
        assert!(landing.distance(plant) < 1e-5, "{landing:?} vs {plant:?}");
    }

    #[test]
    fn stride_scales_with_the_body_walking_it() {
        let short = Stride::for_body(
            &Rig::from_skeleton(
                &HumanoidParams {
                    height: 1.3,
                    ..Default::default()
                }
                .skeleton(),
            )
            .expect("rigs"),
            1.0,
        );
        let tall = Stride::for_body(
            &Rig::from_skeleton(
                &HumanoidParams {
                    height: 2.1,
                    ..Default::default()
                }
                .skeleton(),
            )
            .expect("rigs"),
            1.0,
        );
        assert!(
            tall.length > short.length * 1.3,
            "a taller body steps further"
        );
        assert!(tall.lift > short.lift);
    }

    #[test]
    fn walking_lifts_the_swinging_foot_above_the_planted_one() {
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        // A quarter through the cycle the first contact is mid-stance and the
        // second is mid-swing.
        let mut pose = Pose::rest(&rig);
        let steps = step(&rig, &mut pose, &gait, &stride, 0.75);
        assert!(steps.is_clean(), "{steps:?}");
        assert_eq!(steps.stance.len() + steps.swing.len(), 2);

        let lifted = steps.swing[0];
        let planted = steps.stance[0];
        assert!(
            contact_height(&rig, &pose, lifted) > contact_height(&rig, &pose, planted),
            "the swinging foot should be the higher one"
        );
    }

    #[test]
    fn a_whole_cycle_returns_the_body_to_where_it_began() {
        for rig in [biped(), quadruped()] {
            let gait = Gait::natural(&rig);
            let stride = Stride::for_body(&rig, 1.0);

            let at = |cycle: f32| {
                let mut pose = Pose::rest(&rig);
                step(&rig, &mut pose, &gait, &stride, cycle);
                pose
            };
            let start = at(0.0);
            let round = at(1.0);
            for (a, b) in start.rotations.iter().zip(&round.rotations) {
                assert!(a.abs_diff_eq(*b, 1e-4), "the cycle did not close");
            }
        }
    }

    #[test]
    fn every_body_can_walk_its_own_gait() {
        for rig in [biped(), quadruped()] {
            let gait = Gait::natural(&rig);
            let stride = Stride::for_body(&rig, 1.0);
            for frame in 0..24 {
                let mut pose = Pose::rest(&rig);
                let steps = step(&rig, &mut pose, &gait, &stride, frame as f32 / 24.0);
                assert!(
                    steps.is_clean(),
                    "{} contacts strained at frame {frame}: {steps:?}",
                    gait.len()
                );
            }
        }
    }

    #[test]
    fn standing_still_leaves_the_body_standing() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let steps = step(
            &rig,
            &mut pose,
            &Gait::standing(&rig),
            &Stride::still(),
            0.4,
        );
        assert_eq!(steps.swing.len(), 0);
        assert!(steps.is_clean());
        assert_eq!(steps.crouch, 0.0, "a still body has nothing to sink for");
        for rotation in &pose.rotations {
            // Not exactly nothing: a rest leg stands at full extension, and the
            // solver holds a fraction of a degree back from that singularity on
            // purpose. Well under a degree is standing still.
            assert!(
                rotation.to_axis_angle().1 < 0.02,
                "a still body should not move: {rotation:?}"
            );
        }
    }
}
