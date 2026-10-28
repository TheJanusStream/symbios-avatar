//! Where a coat's marks go: the beast's body as the painter reads it (#368).
//!
//! A mark is placed by anatomy - a saddle on the back, socks on the feet, a
//! blaze down the face, stripes round the barrel - and the painter has only
//! a texel's body-space position, its normal and its zone. This is the rest:
//! the spine the stripes run across, the back's height the legs are measured
//! against, the tail's tip, and the head and the feet the plan declared,
//! whose frames place a blaze and tell a claw from a pad.

use glam::Vec3;

use crate::plan::beast_foot::FootShape;
use crate::plan::{BeastFeet, BeastHead, Limb, Zone};
use crate::rig::Rig;

/// A beast's body, as the painter reads it.
#[derive(Clone, Debug)]
pub(crate) struct CoatBody {
    /// The rear girdle: where the spine starts.
    pub hips: Vec3,
    /// The front girdle: where it ends.
    pub withers: Vec3,
    /// The back's height over the ground.
    pub height: f32,
    /// The tail's first joint, at the croup, and its last.
    pub tail: Option<(Vec3, Vec3)>,
    /// The head its plan declared.
    pub head: Option<BeastHead>,
    /// Each foot's shape, by the limb it ends.
    pub feet: Vec<(Limb, FootShape)>,
}

impl CoatBody {
    /// Reads a beast's body off its rig and its plan's declarations.
    pub(crate) fn of(rig: &Rig, head: Option<&BeastHead>, feet: Option<&BeastFeet>) -> Self {
        let first = |zone: Zone| {
            rig.in_zone(zone)
                .first()
                .map(|&joint| rig.joints[joint].position)
        };
        let hips = first(Zone::Pelvis).unwrap_or(Vec3::ZERO);
        // The front girdle is the joint the neck hangs from: since the trunk
        // became six rings (#375) the chest's first is the middle of the
        // ribcage, and read as the withers it ran every mark measured along
        // the spine (a tan chest, a ruff) back over half the barrel.
        let girdle = rig
            .in_zone(Zone::Neck)
            .first()
            .and_then(|&neck| rig.joints[neck].parent)
            .map(|joint| rig.joints[joint].position);
        let withers = girdle
            .or_else(|| first(Zone::Chest))
            .unwrap_or(hips + Vec3::Z);
        let tail = rig.in_zone(Zone::Tail);
        let tail = tail
            .first()
            .zip(tail.last())
            .map(|(&root, &tip)| (rig.joints[root].position, rig.joints[tip].position));
        Self {
            hips,
            withers,
            height: withers.y.max(f32::EPSILON),
            tail,
            head: head.copied(),
            feet: feet.map_or_else(Vec::new, |feet| crate::extremity::beast::shapes(rig, feet)),
        }
    }

    /// How far along the spine `at` lies: zero over the croup, one over the
    /// withers, past either end beyond them.
    pub(crate) fn along_spine(&self, at: Vec3) -> f32 {
        let spine = self.withers - self.hips;
        (at - self.hips).dot(spine) / spine.length_squared().max(f32::EPSILON)
    }

    /// How high `at` stands, as a share of the back's height.
    pub(crate) fn rise(&self, at: Vec3) -> f32 {
        at.y / self.height
    }

    /// How close `at` is to the tail's tip, as a share of the back's height:
    /// zero at the tip.
    pub(crate) fn tail_tip_distance(&self, at: Vec3) -> f32 {
        self.tail
            .map_or(f32::INFINITY, |(_, tip)| at.distance(tip) / self.height)
    }

    /// How close `at` is to the tail's root, as [`Self::tail_tip_distance`].
    pub(crate) fn tail_root_distance(&self, at: Vec3) -> f32 {
        self.tail
            .map_or(f32::INFINITY, |(root, _)| at.distance(root) / self.height)
    }

    /// How far down the tail `at` lies: zero at the root, one at the tip,
    /// whichever way the tail curls.
    pub(crate) fn along_tail(&self, at: Vec3) -> f32 {
        let (root, tip) = (self.tail_root_distance(at), self.tail_tip_distance(at));
        if root.is_finite() {
            root / (root + tip).max(f32::EPSILON)
        } else {
            0.0
        }
    }

    /// The foot `at` belongs to, if its zone is a foot's, with `at` in that
    /// foot's frame.
    pub(crate) fn foot(&self, zone: Zone, at: Vec3) -> Option<(&FootShape, Vec3)> {
        let Zone::Extremity(limb) = zone else {
            return None;
        };
        self.feet
            .iter()
            .find(|(foot, _)| *foot == limb)
            .map(|(_, shape)| (shape, shape.local(at)))
    }
}

/// How much a surface facing along `normal` faces the ground: none on a
/// side, all on a belly.
pub(crate) fn underside(normal: Vec3) -> f32 {
    (-normal.y).clamp(0.0, 1.0)
}

/// How much a surface facing along `normal` faces the sky.
pub(crate) fn topside(normal: Vec3) -> f32 {
    normal.y.clamp(0.0, 1.0)
}

/// Whether a zone is the trunk's: the barrel, the croup, the chest.
pub(crate) fn trunk(zone: Zone) -> bool {
    matches!(zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest)
}

/// Whether a zone is a leg's, any part of it.
pub(crate) fn leg(zone: Zone) -> bool {
    matches!(
        zone,
        Zone::UpperLimb(_) | Zone::LowerLimb(_) | Zone::Extremity(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, QuadrupedParams};

    #[test]
    fn the_withers_are_the_girdle_the_neck_hangs_from() {
        // #375: the trunk is six rings, and the chest's first is the middle
        // of the ribcage. Read as the withers, every mark measured along the
        // spine - a tan chest, a ruff - ran back over half the barrel.
        let rig = Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
            .expect("rigs");
        let body = CoatBody::of(&rig, None, None);
        let neck = rig.in_zone(Zone::Neck)[0];
        let girdle = rig.joints[neck].parent.expect("a girdle");
        assert_eq!(body.withers, rig.joints[girdle].position);
        // Every ring of the trunk lies between the croup and the withers.
        for joint in [Zone::Pelvis, Zone::Abdomen, Zone::Chest]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
        {
            let along = body.along_spine(rig.joints[joint].position);
            assert!(
                (-1e-4..=1.0 + 1e-4).contains(&along),
                "a trunk ring lies {along} along the spine"
            );
        }
    }
}
