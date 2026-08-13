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

use glam::{Quat, Vec3};

use super::ground::solve_contact;
use super::pose::Pose;
use crate::plan::{Limb, Zone};
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
    /// walk. Duty is set so exactly one contact is airborne at a time on a body
    /// with several, which is the most stable way to move one.
    ///
    /// Two legs are the case that needs care: `1 − 1/2` is a duty of exactly a
    /// half, which means each foot leaves the instant the other lands and the
    /// body is never on both at once. Walking is defined by having that overlap;
    /// without it the result is a run performed at walking pace, and it reads as
    /// one. [`DOUBLE_SUPPORT`] is the floor that keeps it.
    #[must_use]
    pub fn wave(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        let count = limbs.len().max(1);
        Self {
            offsets: (0..limbs.len())
                .map(|index| index as f32 / count as f32)
                .collect(),
            limbs,
            duty: (1.0 - 1.0 / count as f32).max(0.5 + DOUBLE_SUPPORT),
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

/// Fraction of a walk cycle a two-legged body spends on both feet.
///
/// The overlap that makes a walk a walk rather than a slow run.
pub const DOUBLE_SUPPORT: f32 = 0.1;

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

/// How far one limb must sink for its chain to reach a goal `toward` its rest
/// contact, before any margin.
///
/// Measured to the joint the chain actually reaches, not to the contact hanging
/// off it — the same distinction the solve makes. Solved against the limb's
/// *actual* rest geometry rather than from stride length alone, because a
/// limb's contact is not generally beneath its hip: a quadruped's feet already
/// sit well forward of the joints that carry them, and assuming otherwise
/// under-crouches every four-legged body.
fn sink_needed(rig: &Rig, limb: Limb, toward: Vec3) -> Option<f32> {
    let reach = rig.limb_reach(limb)?;
    let chain = rig.limb_chain(limb)?;
    let offset = rig.joints[chain[2]].position - rig.joints[chain[0]].position;
    let reaching = offset + toward;
    let horizontal = reaching.length_squared() - reaching.y * reaching.y;
    // Sinking by `c` shortens the hip-to-goal distance by moving the hip down
    // toward the goal's height.
    let needed = -reaching.y - (reach * reach - horizontal).max(0.0).sqrt();
    Some(needed.max(0.0))
}

/// The deepest a body sinks anywhere in its stride — the envelope of
/// [`crouch_at`].
///
/// A leg standing straight has no slack: swinging its foot forward by half a
/// stride puts the goal further from the hip than the leg is long. Bodies solve
/// this by sinking as they stride, and so does this — without it every step is
/// out of reach and the legs merely stretch toward the ground.
///
/// This is the figure to plan around — camera heights, clearances — but not the
/// height to *hold*: a walk pinned at its own worst case rides flat, and that
/// flatness is exactly the pelvis bob the walk used to lack.
#[must_use]
pub fn crouch_for(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let half = stride.length * 0.5;

    gait.limbs
        .iter()
        .filter_map(|&limb| {
            // Both ends of the stride, since a contact may start forward of its
            // hip and only the further extreme matters.
            [half, -half]
                .into_iter()
                .filter_map(|swing| sink_needed(rig, limb, stride.direction * swing))
                .fold(f32::NEG_INFINITY, f32::max)
                .into()
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN
}

/// How far the body sinks at this point of the cycle — which is the pelvis bob.
///
/// Each limb asks for the sink its *current* goal needs — a foot at the far end
/// of its stride pulls the body down, one passing under its hip lets it rise,
/// and a swinging foot asks for less because its goal is lifted off the ground.
/// The body sinks by the deepest request. On a walking biped that request peaks
/// at every heel-strike and toe-off, where the legs are split at full stride
/// with both ends on the ground, and bottoms out as the stance foot passes
/// under the hip — so the pelvis vaults twice a cycle, highest at each
/// midstance, exactly the inverted pendulum a real walk rides. Nothing here is
/// tuned: the bob's depth, its timing and its pace-scaling all fall out of the
/// same reach geometry the stride was already solved against, and a standing
/// body still sinks exactly zero.
#[must_use]
pub fn crouch_at(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> f32 {
    gait.limbs
        .iter()
        .enumerate()
        .filter_map(|(index, &limb)| {
            sink_needed(rig, limb, contact_offset(stride, gait.phase(index, cycle)))
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN
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

    // Sink far enough that every foot's current goal is within its leg's reach
    // — no further. Holding the whole stride's worst case instead is what kept
    // the pelvis riding dead flat, 47 mm down, through the entire cycle.
    steps.crouch = crouch_at(rig, gait, stride, cycle);
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

/// How far an arm swings fore and aft, in radians.
const ARM_SWING: f32 = 0.46;

/// How far the arms drop from the pose the body is built in, in radians.
///
/// Bodies are modelled with their arms already angled down — an A-pose — so this
/// is only the rest of the way to a hanging arm, not the whole of it. Measured
/// from a T-pose the same drop needs roughly twice the rotation, and a shoulder
/// turned that far bulges however it is skinned.
const ARM_DROP: f32 = 0.66;

/// How far the shoulders twist against the hips, in radians.
///
/// Spread down the spine rather than spent at one joint — see [`swing_arms`].
const SHOULDER_TWIST: f32 = 0.17;

/// How far behind the legs the arms run, as a share of the cycle.
///
/// Not zero. Arms driven in lockstep with the legs read as clockwork; the lag is
/// most of what makes a swing look like it is being carried rather than driven.
const ARM_LAG: f32 = 0.07;

/// How far the elbow is bent even when the arm hangs at rest, in radians.
///
/// **An arm is never straight.** A body walking with locked elbows reads as a
/// mannequin being carried along, and the rest pose this plan builds is
/// perfectly straight — hip to knee to ankle and shoulder to elbow to wrist are
/// each three collinear points, so without this the arm has no bend at all.
const ELBOW_REST: f32 = 0.30;

/// How much further the elbow closes as the arm swings forward, in radians.
///
/// A walking arm folds on the way through and opens again behind, which is most
/// of what stops the swing reading as a pendulum.
const ELBOW_SWING: f32 = 0.26;

/// Swings the arms against the legs and counter-rotates the shoulders.
///
/// The arm on one side follows the leg on the *other*, which is the whole of why
/// a walk reads as a walk: a body swinging its arms in time with the legs on the
/// same side looks like it is marching, and one not swinging them at all — which
/// is what this gait did until now — does not look like it is walking.
///
/// Call after [`step`], which places the feet; this only touches the upper body.
///
/// **A limb the body stands on is left alone.** A quadruped's fore limbs are
/// legs, and they have just been placed by an IK solve; swinging them as though
/// they were arms moved each fore contact by 0.21 to 0.24 m every frame the
/// render tool drew. Asking which limbs carry the body — rather than which end
/// of it they are on — is also what makes this right for bodies nobody has
/// planned, a centaur's arms swinging while its four legs walk.
///
/// **Rotations are composed, not assigned.** This was the one pose producer
/// that overwrote whatever ran before it, which is why it could destroy an IK
/// solve rather than merely disagree with one. Each producer contributes to a
/// pose once; running this twice compounds its own drop, as any additive layer
/// would.
pub fn swing_arms(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32) {
    if !pose.fits(rig) {
        return;
    }

    let carries = rig.ground_contacts();
    let mut lead = 0.0;
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        if carries.contains(&limb) {
            continue;
        }
        let Some([shoulder, elbow, _]) = rig.limb_chain(limb) else {
            continue;
        };
        // The leg diagonally opposite drives this arm.
        let Some(driver) = gait
            .limbs
            .iter()
            .position(|&other| other == limb.mirrored().paired())
        else {
            continue;
        };
        let offset = gait.offsets.get(driver).copied().unwrap_or(0.0);
        let drive = ((cycle - offset + ARM_LAG) * std::f32::consts::TAU).sin();
        if limb == Limb::ForeLeft {
            lead = drive;
        }

        // Down first, then fore and aft about the body's own axis. Positive
        // rotation about X carries a hanging arm backward, so a forward swing is
        // the negative one.
        let side = rig.joints[shoulder].position.x.signum();
        pose.rotations[shoulder] *=
            Quat::from_rotation_x(-ARM_SWING * drive) * Quat::from_rotation_z(-ARM_DROP * side);

        // **The elbow folds forward, about X, and the same way on both arms.**
        // This used to turn about Y with a `side` factor, and both halves of
        // that were wrong. The arm hangs in the frontal plane, so Y is only 50
        // degrees off its own axis and most of the rotation was spent spinning
        // the forearm rather than bending it — measured on the walk, the elbow
        // reached 15 degrees of bend for 19 degrees of rotation asked. What
        // little bend there was went sideways, in and out from the body, rather
        // than forward where an elbow folds. And `side` mirrored the fold, so
        // one arm bent forward while the other bent back; an elbow is not
        // chiral, and folding it is the one thing about an arm that is the same
        // on both sides.
        //
        // **And it folds about the axis the drop has already carried away from
        // X.** The elbow's frame hangs off the shoulder rotation above, so a
        // plain local X sits `ARM_DROP` off the world's by the time it acts —
        // measured at the joint, 0.64 of every degree asked arrived as bend and
        // the rest rolled the forearm about its own length (#223). Undoing the
        // drop in the axis puts the fold back on world X, where a fold ahead of
        // a hanging arm is a bend and nothing else, and the constants deliver
        // the degrees they are written in.
        let fold = Quat::from_rotation_z(ARM_DROP * side) * Vec3::X;
        pose.rotations[elbow] *=
            Quat::from_axis_angle(fold, -(ELBOW_REST + ELBOW_SWING * drive.max(0.0)));
    }

    // Shoulders against hips, and then the neck against the shoulders so the
    // head keeps looking where it is going rather than being turned by the walk.
    //
    // **Spread down the whole spine, not spent at the top of it.** The twist
    // used to go entirely into the joint the arms hang from, which turned the
    // shoulders by the right angle and left the ribcage and waist beneath them
    // dead still — the shoulders read as a yoke swivelling on a post rather
    // than as a torso winding. Sharing it out costs nothing: local rotations
    // compound down a chain, so the shoulders still arrive at the same angle.
    //
    // Weighted toward the top, which is where a spine actually turns: the
    // shares run 1, 2, 3 up the chain, so the waist contributes a sixth and the
    // shoulder girdle a half. Derived from the chain the body has rather than
    // written out, so a plan with a longer spine winds along all of it.
    if let Some(&neck) = rig.in_zone(Zone::Neck).first()
        && let Some(girdle) = rig.joints[neck].parent
    {
        let spine = spine_to(rig, girdle);
        let total: f32 = (1..=spine.len()).map(|rank| rank as f32).sum();
        for (rank, &joint) in spine.iter().enumerate() {
            let share = (rank + 1) as f32 / total.max(1.0);
            pose.rotations[joint] *= Quat::from_rotation_y(SHOULDER_TWIST * lead * share);
        }
        pose.rotations[neck] *= Quat::from_rotation_y(-SHOULDER_TWIST * lead);
    }
}

/// The spine from the pelvis up to `top`, pelvis end first.
///
/// Walked up the parent chain and stopped at the root, so it is whatever spine
/// the body has rather than a list of names. The root itself is left out: it
/// carries the whole body, and turning it turns the legs too.
fn spine_to(rig: &Rig, top: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut at = Some(top);
    while let Some(joint) = at {
        if rig.joints[joint].parent.is_none() || !rig.joints[joint].zone.is_core() {
            break;
        }
        chain.push(joint);
        at = rig.joints[joint].parent;
    }
    chain.reverse();
    chain
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
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
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
    fn a_two_legged_walk_has_both_feet_down_some_of_the_time() {
        // Without an overlap each foot leaves as the other lands, which is a run
        // performed slowly rather than a walk.
        let gait = Gait::wave(&biped());
        let samples = 200;
        let both = (0..samples)
            .filter(|step| gait.grounded(*step as f32 / samples as f32) == 2)
            .count() as f32
            / samples as f32;
        assert!(
            (both - 2.0 * DOUBLE_SUPPORT).abs() < 0.05,
            "double support was {both:.2} of the cycle"
        );
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
                .skeleton(&crate::Composites::default()),
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
                .skeleton(&crate::Composites::default()),
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

    /// How far the elbow is bent, in degrees away from straight.
    ///
    /// Measured at the joint, never read off the rotation asked for. A
    /// quaternion about the arm's own axis is a perfectly good rotation that
    /// bends nothing, and that is exactly what this used to be: turning about Y
    /// spent most of itself spinning a forearm that hangs 50 degrees off that
    /// axis (#114).
    fn elbow_bend(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let Some([shoulder, elbow, wrist]) = rig.limb_chain(limb) else {
            return 0.0;
        };
        let posed = pose.forward(rig);
        let upper = (posed.positions[shoulder] - posed.positions[elbow]).normalize_or_zero();
        let fore = (posed.positions[wrist] - posed.positions[elbow]).normalize_or_zero();
        180.0 - upper.dot(fore).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_walking_arm_is_never_straight() {
        // A locked elbow reads as a mannequin being carried along, and the rest
        // pose is straight to the last decimal: shoulder, elbow and wrist are
        // three collinear points, so the bend has to come from here.
        let rig = biped();
        let gait = Gait::natural(&rig);
        for frame in 0..12 {
            let cycle = frame as f32 / 12.0;
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, cycle);
            for limb in [Limb::ForeLeft, Limb::ForeRight] {
                let bend = elbow_bend(&rig, &pose, limb);
                assert!(
                    bend > 8.0,
                    "{limb:?} was {bend:.1} degrees from straight at cycle {cycle:.2}"
                );
            }
        }
    }

    #[test]
    fn both_elbows_fold_the_same_way() {
        // An elbow is not chiral. Folding it is the one thing about an arm that
        // is the same on both sides, and a `side` factor here had one arm
        // bending forward while the other bent back — the same family of mistake
        // as building one hand by rotating the other (#113).
        //
        // Asked half a cycle apart, which is where the two arms are in the same
        // place in their own swings, so the comparison is like for like.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let bend_at = |limb: Limb, cycle: f32| {
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, cycle);
            elbow_bend(&rig, &pose, limb)
        };
        for frame in 0..6 {
            let cycle = frame as f32 / 12.0;
            let left = bend_at(Limb::ForeLeft, cycle);
            let right = bend_at(Limb::ForeRight, cycle + 0.5);
            assert!(
                (left - right).abs() < 1.0,
                "at cycle {cycle:.2} the left elbow bent {left:.1} and the right {right:.1}"
            );
        }
    }

    #[test]
    fn the_forearm_folds_forward_and_not_across_the_body() {
        // Which plane the fold is in, which no angle at the joint can tell you.
        // The old rotation bent the arm sideways, in and out from the hip; an
        // elbow folds the hand toward the front of the body.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let mut pose = Pose::rest(&rig);
        swing_arms(&rig, &mut pose, &gait, 0.0);
        let posed = pose.forward(&rig);

        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let [shoulder, elbow, wrist] = rig.limb_chain(limb).expect("an arm bends");
            // Where the wrist sits against the line the upper arm was heading
            // along: forward of it is a fold, to either side of it is not.
            let upper = (posed.positions[elbow] - posed.positions[shoulder]).normalize_or_zero();
            let fore = posed.positions[wrist] - posed.positions[elbow];
            let off_axis = fore - upper * fore.dot(upper);
            assert!(
                off_axis.z > off_axis.x.abs(),
                "{limb:?} put its forearm {:.3} forward and {:.3} across",
                off_axis.z,
                off_axis.x
            );
        }
    }

    #[test]
    fn the_whole_spine_turns_with_the_arms_not_just_the_top_of_it() {
        // The twist used to go entirely into the joint the arms hang off, so the
        // shoulders turned the right amount over a ribcage and waist that did
        // not move at all — a yoke swivelling on a post rather than a torso
        // winding (#114).
        let rig = biped();
        let gait = Gait::natural(&rig);
        let mut pose = Pose::rest(&rig);
        // A quarter cycle, where the lead is near its widest.
        swing_arms(&rig, &mut pose, &gait, 0.25);
        let posed = pose.forward(&rig);

        let neck = *rig.in_zone(Zone::Neck).first().expect("a neck");
        let girdle = rig.joints[neck].parent.expect("a girdle");
        let spine = spine_to(&rig, girdle);
        assert!(
            spine.len() >= 2,
            "a spine of {} to share a twist",
            spine.len()
        );

        // Every joint of it turns, and each one further round than the one below.
        let mut turned = 0.0f32;
        for &joint in &spine {
            let angle = (posed.rotations[joint] * Vec3::X)
                .z
                .atan2((posed.rotations[joint] * Vec3::X).x)
                .to_degrees();
            assert!(
                angle.abs() > turned,
                "joint {joint} of the spine turned {angle:.2}, no further than the {turned:.2} \
                 beneath it"
            );
            turned = angle.abs();
        }
        // And sharing the twist out must not ADD any: the shoulders still
        // arrive at the angle one joint used to carry alone, because local
        // rotations compound down a chain.
        assert!(
            turned <= SHOULDER_TWIST.to_degrees() + 0.5,
            "the shoulders turned {turned:.1} degrees, past the {:.1} asked for",
            SHOULDER_TWIST.to_degrees()
        );
    }

    #[test]
    fn the_pelvis_vaults_twice_a_cycle_between_its_envelope_and_its_midstances() {
        // The bob is not tuned, so it is pinned where the geometry pins it: the
        // sink returns to the whole-stride envelope at each heel-strike and
        // toe-off — where the legs are split at full stride with both goals on
        // the ground — and falls away as the stance foot passes under the hip.
        // With offsets 0 and 0.5 and duty 0.6, the handoffs sit at cycles 0.0,
        // 0.1, 0.5 and 0.6, and the midstances near 0.3 and 0.8.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let envelope = crouch_for(&rig, &gait, &stride);
        assert!(envelope > 0.01, "a stride this long must sink the body");

        for handoff in [0.0, 0.1, 0.5, 0.6] {
            let sink = crouch_at(&rig, &gait, &stride, handoff);
            assert!(
                (sink - envelope).abs() < envelope * 0.02,
                "at handoff {handoff} the sink was {sink:.4} against an envelope of \
                 {envelope:.4}"
            );
        }
        for midstance in [0.3, 0.8] {
            let sink = crouch_at(&rig, &gait, &stride, midstance);
            assert!(
                sink < envelope * 0.25,
                "at midstance {midstance} the body should rise: sink {sink:.4} against \
                 {envelope:.4}"
            );
        }
        // And nowhere does the phase ask for more than the envelope was built
        // from — the envelope is the maximum of the phases, not a separate law.
        for sample in 0..240 {
            let sink = crouch_at(&rig, &gait, &stride, sample as f32 / 240.0);
            assert!(
                sink <= envelope + 1e-5,
                "cycle {} sank {sink:.4} past the envelope {envelope:.4}",
                sample as f32 / 240.0
            );
        }
    }

    #[test]
    fn a_still_body_asks_for_no_sink_at_any_moment() {
        // `standing_still_leaves_the_body_standing` checks one cycle point;
        // the per-phase sink must hold the same zero at every one of them.
        let rig = biped();
        let gait = Gait::standing(&rig);
        for sample in 0..40 {
            let sink = crouch_at(&rig, &gait, &Stride::still(), sample as f32 / 40.0);
            assert_eq!(
                sink,
                0.0,
                "a still body sank at cycle {}",
                sample as f32 / 40.0
            );
        }
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
