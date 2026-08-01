//! Turning a body to look at something.
//!
//! A gaze is not one joint's job. Looking sharply to the side turns the eyes,
//! then the head, then the neck, and finally the shoulders — and a body that
//! swivels its skull alone to track something reads as a doll. So the rotation
//! is spread down the chain from the torso outward, each joint taking a share
//! and passing the remainder on.
//!
//! The share is applied to what *remains* after the joints before it have had
//! their turn, and the last joint takes everything left. That way the head ends
//! up genuinely pointing at the target — spreading fixed fractions of the total
//! instead leaves it slightly short, because each joint's rotation compounds
//! onto the ones before it.
//!
//! Clamping matters as much as distributing. Real necks stop; a gaze target
//! behind a body should turn it as far as it goes and leave it there, not wind
//! the head around backwards.

use glam::{Quat, Vec3};

use super::pose::Pose;
use crate::plan::Zone;
use crate::rig::{Rig, landmark};

/// Tuning for [`look_at`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GazeConfig {
    /// Furthest the whole chain may turn from facing forward, in radians.
    pub limit: f32,
    /// How much of the remaining turn each joint takes, from the torso outward.
    ///
    /// The last value should be `1.0` so the final joint completes the turn.
    pub shares: [f32; 3],
}

impl Default for GazeConfig {
    fn default() -> Self {
        Self {
            // About 100°: a body will turn its shoulders to look behind itself,
            // but not fold in half doing it.
            limit: 1.75,
            shares: [0.25, 0.45, 1.0],
        }
    }
}

/// What a gaze managed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaze {
    /// Angle the chain actually turned through, in radians.
    pub turned: f32,
    /// Whether the target ended up within the chain's limit.
    pub reached: bool,
}

/// Turns a body's head toward `target`.
///
/// Uses whichever of chest, neck, and head the body has, so a creature with no
/// separate neck still looks where it is told.
pub fn look_at(rig: &Rig, pose: &mut Pose, target: Vec3, config: &GazeConfig) -> Gaze {
    let idle = Gaze {
        turned: 0.0,
        reached: false,
    };
    if !pose.fits(rig) {
        return idle;
    }

    // Walked up the hierarchy from the head rather than looked up by zone: a
    // zone can hold several joints — a head has a crown above it, a chest has
    // both clavicles — and picking one by position gets a different joint the
    // moment a body plan gains a node.
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return idle;
    };
    let mut chain = vec![head];
    let mut cursor = head;
    while let Some(parent) = rig.joints[cursor].parent {
        if !matches!(rig.joints[parent].zone, Zone::Neck | Zone::Chest) {
            break;
        }
        chain.push(parent);
        cursor = parent;
    }
    chain.reverse();
    // The joints nearest the head do the looking; anything further down the
    // spine belongs to posture rather than gaze.
    if chain.len() > config.shares.len() {
        chain.drain(..chain.len() - config.shares.len());
    }

    // Where the head is looking before anything turns. Every joint's share is
    // measured against this, so the clamp applies to the whole gesture rather
    // than to each joint separately.
    let start = pose.forward(rig);
    let rest_facing = start.rotations[head] * landmark::FORWARD;
    let mut reached = true;

    for (index, &joint) in chain.iter().enumerate() {
        // Recomputed per joint, because turning the chest *moves the head*: the
        // direction to the target is not the same once the body beneath it has
        // shifted. Solving once from the starting geometry leaves the gaze
        // several degrees short.
        let posed = pose.forward(rig);
        let facing = posed.rotations[head] * landmark::FORWARD;
        let toward = (target - posed.positions[head]).normalize_or_zero();
        if toward == Vec3::ZERO {
            break;
        }

        // Clamp against the turn from rest, not from wherever this joint starts.
        let (axis, angle) = Quat::from_rotation_arc(rest_facing, toward).to_axis_angle();
        reached = angle <= config.limit + 1e-4;
        let goal = if reached {
            toward
        } else {
            Quat::from_axis_angle(axis, config.limit) * rest_facing
        };

        let needed = Quat::from_rotation_arc(facing, goal);
        let share = config
            .shares
            .get(index + config.shares.len() - chain.len())
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        // The last joint completes the turn; the others take a share and pass
        // the remainder outward.
        let take = if index + 1 == chain.len() {
            needed
        } else {
            Quat::IDENTITY.slerp(needed, share)
        };
        if take.is_near_identity() {
            continue;
        }

        let world = take * posed.rotations[joint];
        let parent = rig.joints[joint]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        pose.rotations[joint] = parent.inverse() * world;
    }

    let ended = pose.forward(rig).rotations[head] * landmark::FORWARD;
    let allowed = rest_facing.angle_between(ended);

    Gaze {
        turned: allowed,
        reached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    /// Where the head is pointing in the given pose.
    fn facing(rig: &Rig, pose: &Pose) -> Vec3 {
        let head = rig.in_zone(Zone::Head)[0];
        pose.forward(rig).rotations[head] * landmark::FORWARD
    }

    /// Where the head is in the given pose.
    fn head_at(rig: &Rig, pose: &Pose) -> Vec3 {
        let head = rig.in_zone(Zone::Head)[0];
        pose.forward(rig).positions[head]
    }

    #[test]
    fn a_body_ends_up_looking_where_it_was_told() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let target = head_at(&rig, &pose) + Vec3::new(1.0, 0.3, 2.0);

        let gaze = look_at(&rig, &mut pose, target, &GazeConfig::default());
        assert!(gaze.reached, "the target was within reach");

        let toward = (target - head_at(&rig, &pose)).normalize();
        assert!(
            facing(&rig, &pose).dot(toward) > 0.995,
            "the head should point at the target, off by {:.3} rad",
            facing(&rig, &pose).angle_between(toward)
        );
    }

    #[test]
    fn the_turn_is_shared_down_the_chain_rather_than_taken_by_the_head() {
        // A body that swivels only its skull reads as a doll.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let target = head_at(&rig, &pose) + Vec3::new(2.0, 0.0, 1.0);
        look_at(&rig, &mut pose, target, &GazeConfig::default());

        // Walk the same joints the gaze does, rather than naming them: the head,
        // and the two spine joints beneath it.
        let head = rig.in_zone(Zone::Head)[0];
        let neck = rig.joints[head].parent.expect("a neck");
        let girdle = rig.joints[neck].parent.expect("a girdle");
        for (name, joint) in [("head", head), ("neck", neck), ("girdle", girdle)] {
            let angle = pose.rotations[joint].to_axis_angle().1;
            assert!(
                angle > 0.02,
                "the {name} should carry part of the turn, has {angle:.3}"
            );
        }
    }

    #[test]
    fn a_neck_stops_rather_than_winding_round() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        // Directly behind the body.
        let target = head_at(&rig, &pose) - Vec3::Z * 3.0;

        let gaze = look_at(&rig, &mut pose, target, &GazeConfig::default());
        assert!(!gaze.reached, "looking straight backwards is beyond a neck");
        assert!(
            gaze.turned <= GazeConfig::default().limit + 1e-4,
            "turned {:.3} past the limit",
            gaze.turned
        );
        // It still turned as far as it could, toward the target.
        assert!(
            facing(&rig, &pose).z < 0.5,
            "should have turned some of the way"
        );
    }

    #[test]
    fn looking_where_it_already_looks_changes_nothing() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let ahead = head_at(&rig, &pose) + landmark::FORWARD * 5.0;

        let gaze = look_at(&rig, &mut pose, ahead, &GazeConfig::default());
        assert!(gaze.reached);
        assert!(gaze.turned < 1e-3, "turned {:.4} for nothing", gaze.turned);
        for rotation in &pose.rotations {
            assert!(rotation.is_near_identity());
        }
    }

    #[test]
    fn a_gaze_leaves_the_legs_alone() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let target = head_at(&rig, &pose) + Vec3::new(2.0, 1.0, 1.0);
        look_at(&rig, &mut pose, target, &GazeConfig::default());

        for joint in rig.limb_chain(crate::plan::Limb::HindLeft).expect("a leg") {
            assert!(
                pose.rotations[joint].is_near_identity(),
                "the legs should not follow a gaze"
            );
        }
    }

    #[test]
    fn a_creature_without_a_full_chain_still_looks() {
        let rig = Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs");
        let mut pose = Pose::rest(&rig);
        let target = head_at(&rig, &pose) + Vec3::new(1.0, 0.5, 1.0);

        let gaze = look_at(&rig, &mut pose, target, &GazeConfig::default());
        assert!(gaze.turned > 0.0, "a quadruped should look too");

        let toward = (target - head_at(&rig, &pose)).normalize();
        assert!(facing(&rig, &pose).dot(toward) > 0.99);
    }

    #[test]
    fn a_gaze_is_deterministic() {
        let rig = biped();
        let target = Vec3::new(1.0, 1.5, 2.0);
        let once = {
            let mut pose = Pose::rest(&rig);
            look_at(&rig, &mut pose, target, &GazeConfig::default());
            pose
        };
        let twice = {
            let mut pose = Pose::rest(&rig);
            look_at(&rig, &mut pose, target, &GazeConfig::default());
            pose
        };
        assert_eq!(once, twice);
    }
}
