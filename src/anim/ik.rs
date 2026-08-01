//! Inverse kinematics: posing a chain so its tip reaches a place.
//!
//! This is the machinery that lets motion be described by *goals* rather than by
//! joint angles — "this foot is on the ground here", "this hand is on that
//! handle" — which is the only way one description of a movement can serve
//! bodies whose proportions differ. Joint angles bake in the skeleton they were
//! authored on; goals do not.
//!
//! Two solvers, because two shapes of problem come up. A limb has exactly two
//! bones and an analytic answer, so [`two_bone`] solves it exactly and in one
//! step. A spine, a neck, or a tail has many bones and no closed form, so
//! [`fabrik`] iterates.
//!
//! Both work the same way at the end: solve for *positions*, then convert those
//! back into the joint rotations a pose is made of, composing each correction
//! onto the rotation already there so a limb's existing twist survives being
//! reached with.

use glam::{Quat, Vec3};

use super::pose::Pose;
use crate::rig::Rig;

/// Distances below this are treated as zero.
const EPSILON: f32 = 1e-5;

/// Tuning for [`fabrik`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FabrikConfig {
    /// Most passes to make before giving up.
    pub iterations: usize,
    /// How close to the target counts as solved, in metres.
    pub tolerance: f32,
}

impl Default for FabrikConfig {
    fn default() -> Self {
        Self {
            iterations: 10,
            tolerance: 1e-3,
        }
    }
}

/// Bends a two-bone chain so its tip reaches `target`.
///
/// `chain` is `[root, mid, tip]` — a shoulder, elbow, and wrist, or a hip, knee,
/// and ankle. `pole` is a point the middle joint bends toward, which is what
/// decides whether a knee faces forward or backward; without it the bend plane
/// is undetermined whenever the limb is straight.
///
/// Returns whether the target was actually reachable. An out-of-reach target
/// still poses the limb — stretched out toward it, which is what a body does
/// when it reaches for something too far away — but reports `false` so a caller
/// can decide to step, lean, or give up.
pub fn two_bone(rig: &Rig, pose: &mut Pose, chain: [usize; 3], target: Vec3, pole: Vec3) -> bool {
    if !pose.fits(rig) || chain.iter().any(|&joint| joint >= rig.len()) {
        return false;
    }

    let posed = pose.forward(rig);
    let [root, mid, tip] = chain.map(|joint| posed.positions[joint]);

    let upper = root.distance(mid);
    let lower = mid.distance(tip);
    if upper <= EPSILON || lower <= EPSILON {
        return false;
    }

    let to_target = target - root;
    let reach = to_target.length();
    if reach <= EPSILON {
        return false;
    }
    let direction = to_target / reach;

    // A chain can neither stretch past its length nor fold below the gap between
    // its bones; clamping keeps the triangle solvable either way.
    let longest = upper + lower - EPSILON;
    let shortest = (upper - lower).abs() + EPSILON;
    let solved = reach.clamp(shortest, longest);

    // The bend plane: toward the pole, falling back to the bend the limb already
    // has, and finally to any perpendicular if the limb is dead straight.
    let bend = perpendicular(pole - root, direction)
        .or_else(|| perpendicular(mid - root, direction))
        .unwrap_or_else(|| direction.any_orthonormal_vector());

    // Law of cosines for the angle between the upper bone and the line to the
    // target.
    let cosine = ((upper * upper + solved * solved - lower * lower) / (2.0 * upper * solved))
        .clamp(-1.0, 1.0);
    let angle = cosine.acos();

    let bent_mid = root + (direction * angle.cos() + bend * angle.sin()) * upper;
    let bent_tip = root + direction * solved;

    retarget(
        rig,
        pose,
        &chain,
        &[root, mid, tip],
        &[root, bent_mid, bent_tip],
        &posed.rotations,
    );

    // Reachability is judged against the true span, not the shortened one the
    // maths uses: a limb standing straight is at exactly full extension, and
    // reporting that as out of reach would call every rest pose a failure.
    reach <= upper + lower + EPSILON
}

/// Reaches a chain of any length toward `target`, iteratively.
///
/// `chain` runs from the anchored root outward to the tip. The root stays put;
/// everything beyond it bends.
///
/// Returns whether the tip ended within [`FabrikConfig::tolerance`] of the
/// target.
pub fn fabrik(
    rig: &Rig,
    pose: &mut Pose,
    chain: &[usize],
    target: Vec3,
    config: &FabrikConfig,
) -> bool {
    if chain.len() < 2 || !pose.fits(rig) || chain.iter().any(|&joint| joint >= rig.len()) {
        return false;
    }

    let posed = pose.forward(rig);
    let original: Vec<Vec3> = chain.iter().map(|&joint| posed.positions[joint]).collect();
    let lengths: Vec<f32> = original
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .collect();
    if lengths.iter().any(|length| *length <= EPSILON) {
        return false;
    }

    let anchor = original[0];
    let span: f32 = lengths.iter().sum();
    let mut solved = original.clone();

    if anchor.distance(target) > span {
        // Out of reach: the honest answer is a straight line toward the target.
        let direction = (target - anchor).normalize_or_zero();
        if direction == Vec3::ZERO {
            return false;
        }
        for index in 1..solved.len() {
            solved[index] = solved[index - 1] + direction * lengths[index - 1];
        }
    } else {
        for _ in 0..config.iterations {
            // Backward: put the tip on the target and walk back to the root.
            let last = solved.len() - 1;
            solved[last] = target;
            for index in (0..last).rev() {
                solved[index] = step(solved[index + 1], solved[index], lengths[index]);
            }
            // Forward: put the root back and walk out to the tip.
            solved[0] = anchor;
            for index in 1..solved.len() {
                solved[index] = step(solved[index - 1], solved[index], lengths[index - 1]);
            }
            if solved[last].distance(target) <= config.tolerance {
                break;
            }
        }
    }

    retarget(rig, pose, chain, &original, &solved, &posed.rotations);
    solved[solved.len() - 1].distance(target) <= config.tolerance
}

/// Moves `toward` to sit exactly `length` from `from`, along the same direction.
fn step(from: Vec3, toward: Vec3, length: f32) -> Vec3 {
    let direction = (toward - from).normalize_or_zero();
    if direction == Vec3::ZERO {
        return from + Vec3::Y * length;
    }
    from + direction * length
}

/// The part of `vector` perpendicular to `axis`, if there is one.
fn perpendicular(vector: Vec3, axis: Vec3) -> Option<Vec3> {
    let flattened = vector - axis * vector.dot(axis);
    (flattened.length_squared() > EPSILON * EPSILON).then(|| flattened.normalize())
}

/// Rewrites a chain's local rotations so its joints land on `solved`.
///
/// Each joint is turned by the rotation carrying the direction its child sits in
/// onto the direction the solution wants — *after* the corrections its ancestors
/// already received, which is what `carried` accumulates. Composing onto the
/// existing world rotation rather than replacing it preserves the twist the limb
/// was already holding.
fn retarget(
    rig: &Rig,
    pose: &mut Pose,
    chain: &[usize],
    original: &[Vec3],
    solved: &[Vec3],
    world_rotations: &[Quat],
) {
    let mut carried = Quat::IDENTITY;
    let mut parent_world = rig.joints[chain[0]]
        .parent
        .map_or(Quat::IDENTITY, |parent| world_rotations[parent]);

    for index in 0..chain.len() - 1 {
        let was = (original[index + 1] - original[index]).normalize_or_zero();
        let wants = (solved[index + 1] - solved[index]).normalize_or_zero();
        if was == Vec3::ZERO || wants == Vec3::ZERO {
            continue;
        }

        let turn = Quat::from_rotation_arc(carried * was, wants);
        let world = turn * carried * world_rotations[chain[index]];
        pose.rotations[chain[index]] = parent_world.inverse() * world;

        carried = turn * carried;
        parent_world = world;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, Limb, Zone};
    use crate::rig::Rig;

    fn rig() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    /// The shoulder, elbow, and wrist of one arm.
    fn arm(rig: &Rig, limb: Limb) -> [usize; 3] {
        let upper = rig.in_zone(Zone::UpperLimb(limb));
        let lower = rig.in_zone(Zone::LowerLimb(limb));
        [upper[0], upper[1], lower[0]]
    }

    #[test]
    fn a_limb_reaches_a_target_within_its_span() {
        let rig = rig();
        let chain = arm(&rig, Limb::ForeLeft);
        let mut pose = Pose::rest(&rig);

        let start = pose.forward(&rig).positions;
        // Somewhere clearly inside the arm's reach.
        let target = start[chain[0]] + Vec3::new(-0.2, -0.15, 0.15);

        assert!(
            two_bone(&rig, &mut pose, chain, target, target + Vec3::Z),
            "target should be reachable"
        );
        let solved = pose.forward(&rig);
        assert!(
            solved.positions[chain[2]].distance(target) < 1e-3,
            "tip landed {:.4} away",
            solved.positions[chain[2]].distance(target)
        );
    }

    #[test]
    fn bones_keep_their_length_while_reaching() {
        let rig = rig();
        let chain = arm(&rig, Limb::ForeLeft);
        let mut pose = Pose::rest(&rig);
        let before = pose.forward(&rig).positions;
        let upper = before[chain[0]].distance(before[chain[1]]);
        let lower = before[chain[1]].distance(before[chain[2]]);

        let target = before[chain[0]] + Vec3::new(-0.25, -0.2, 0.1);
        two_bone(&rig, &mut pose, chain, target, target + Vec3::Z);

        let after = pose.forward(&rig).positions;
        assert!((after[chain[0]].distance(after[chain[1]]) - upper).abs() < 1e-4);
        assert!((after[chain[1]].distance(after[chain[2]]) - lower).abs() < 1e-4);
    }

    #[test]
    fn an_unreachable_target_stretches_the_limb_and_says_so() {
        let rig = rig();
        let chain = arm(&rig, Limb::ForeLeft);
        let mut pose = Pose::rest(&rig);
        let root = pose.forward(&rig).positions[chain[0]];
        let far = root + Vec3::new(-9.0, 0.0, 0.0);

        assert!(
            !two_bone(&rig, &mut pose, chain, far, far + Vec3::Z),
            "an out-of-reach target must be reported"
        );

        // Still posed, straight out toward it.
        let solved = pose.forward(&rig);
        let direction = (solved.positions[chain[2]] - root).normalize();
        assert!(
            direction.dot(Vec3::NEG_X) > 0.99,
            "the limb should stretch toward the target"
        );
    }

    #[test]
    fn the_pole_decides_which_way_the_joint_bends() {
        let rig = rig();
        let chain = arm(&rig, Limb::ForeLeft);
        let root = Pose::rest(&rig).forward(&rig).positions[chain[0]];
        let target = root + Vec3::new(-0.25, -0.15, 0.0);

        let elbow_toward = |pole: Vec3| {
            let mut pose = Pose::rest(&rig);
            two_bone(&rig, &mut pose, chain, target, pole);
            pose.forward(&rig).positions[chain[1]]
        };

        let forward = elbow_toward(root + Vec3::Z);
        let backward = elbow_toward(root - Vec3::Z);
        assert!(
            forward.z > backward.z,
            "the elbow should follow the pole: {forward:?} vs {backward:?}"
        );
    }

    #[test]
    fn reaching_leaves_the_rest_of_the_body_alone() {
        let rig = rig();
        let chain = arm(&rig, Limb::ForeLeft);
        let mut pose = Pose::rest(&rig);
        let before = pose.forward(&rig).positions.clone();

        let target = before[chain[0]] + Vec3::new(-0.2, -0.2, 0.1);
        two_bone(&rig, &mut pose, chain, target, target + Vec3::Z);
        let after = pose.forward(&rig).positions;

        let other = arm(&rig, Limb::ForeRight);
        for &joint in &other {
            assert!(
                before[joint].distance(after[joint]) < 1e-5,
                "the other arm moved"
            );
        }
        assert!(before[0].distance(after[0]) < 1e-5, "the root moved");
    }

    #[test]
    fn fabrik_reaches_along_a_long_chain() {
        let rig = rig();
        // Pelvis up through the spine to the head: five joints, no closed form.
        let chain: Vec<usize> = {
            let mut walk = vec![rig.in_zone(Zone::Head)[0]];
            while let Some(parent) = rig.joints[*walk.last().unwrap()].parent {
                walk.push(parent);
                if rig.joints[parent].zone == Zone::Pelvis {
                    break;
                }
            }
            walk.reverse();
            walk
        };
        assert!(chain.len() >= 4, "expected a real spine, got {chain:?}");

        let mut pose = Pose::rest(&rig);
        let start = pose.forward(&rig).positions;
        let target = start[*chain.last().unwrap()] + Vec3::new(0.15, -0.1, 0.2);

        assert!(fabrik(
            &rig,
            &mut pose,
            &chain,
            target,
            &FabrikConfig::default()
        ));
        let solved = pose.forward(&rig);
        assert!(
            solved.positions[*chain.last().unwrap()].distance(target) < 1e-2,
            "head landed {:.4} away",
            solved.positions[*chain.last().unwrap()].distance(target)
        );
    }

    #[test]
    fn fabrik_keeps_every_bone_the_length_it_was() {
        let rig = rig();
        let chain: Vec<usize> = vec![
            rig.in_zone(Zone::Pelvis)[0],
            rig.in_zone(Zone::Abdomen)[0],
            rig.in_zone(Zone::Chest)[0],
            rig.in_zone(Zone::Neck)[0],
        ];
        let mut pose = Pose::rest(&rig);
        let before = pose.forward(&rig).positions.clone();
        let lengths: Vec<f32> = chain
            .windows(2)
            .map(|pair| before[pair[0]].distance(before[pair[1]]))
            .collect();

        let target = before[chain[3]] + Vec3::new(0.2, -0.05, 0.1);
        fabrik(&rig, &mut pose, &chain, target, &FabrikConfig::default());

        let after = pose.forward(&rig).positions;
        for (index, pair) in chain.windows(2).enumerate() {
            let now = after[pair[0]].distance(after[pair[1]]);
            assert!(
                (now - lengths[index]).abs() < 1e-3,
                "bone {index} changed from {:.4} to {now:.4}",
                lengths[index]
            );
        }
    }

    #[test]
    fn the_same_goal_suits_bodies_of_different_proportions() {
        // The property the whole approach rests on: a goal expressed in space is
        // reachable by any body that can span it, without knowing its bones.
        for height in [1.3f32, 1.75, 2.1] {
            let rig = Rig::from_skeleton(
                &HumanoidParams {
                    height,
                    ..Default::default()
                }
                .skeleton(),
            )
            .expect("rigs");
            let chain = arm(&rig, Limb::ForeLeft);
            let mut pose = Pose::rest(&rig);

            // The goal expressed the way a body-independent motion would
            // phrase it: a fraction of the limb's own reach, in a direction.
            // That is what lets one description serve every body.
            let joints = pose.forward(&rig).positions;
            let root = joints[chain[0]];
            let span =
                root.distance(joints[chain[1]]) + joints[chain[1]].distance(joints[chain[2]]);
            let target = root + Vec3::new(-0.6, -0.4, 0.4).normalize() * (span * 0.75);

            assert!(
                two_bone(&rig, &mut pose, chain, target, target + Vec3::Z),
                "height {height} could not reach"
            );
            let reached = pose.forward(&rig).positions[chain[2]];
            assert!(reached.distance(target) < 1e-3, "height {height} missed");
        }
    }

    #[test]
    fn malformed_requests_are_refused_rather_than_panicking() {
        let rig = rig();
        let mut pose = Pose::rest(&rig);
        assert!(!two_bone(
            &rig,
            &mut pose,
            [0, 1, 9999],
            Vec3::ZERO,
            Vec3::Z
        ));
        assert!(!fabrik(
            &rig,
            &mut pose,
            &[0],
            Vec3::ZERO,
            &FabrikConfig::default()
        ));

        let mut short = Pose::rest(&rig);
        short.rotations.truncate(2);
        assert!(!two_bone(&rig, &mut short, [0, 1, 2], Vec3::X, Vec3::Z));
    }
}
