//! Where a coat's marks go: the beast's body as the painter reads it (#368).
//!
//! A mark is placed by anatomy - a saddle on the back, socks on the feet, a
//! blaze down the face, stripes round the barrel - and the painter has only
//! a texel's body-space position, its normal and its zone. This is the rest:
//! the spine the stripes run across, the back's height the legs are measured
//! against, the tail's tip, the head and the feet the plan declared, whose
//! frames place a blaze and tell a claw from a pad, and where each leg leaves
//! the trunk.
//!
//! **A mark that differs between the barrel and a leg asks the junction, not
//! the zone** (#376). The zone map's line between the two is where the skin
//! weights cross, which is nowhere an animal's coat changes: a stripe turned
//! from a band to a ring there, and a sock, a point or a roan's thinning cut
//! off along it, on a hard line round the top of the leg. The junction's
//! share runs from the barrel to the leg through the crease at the elbow and
//! the stifle, so every such mark blends over that band instead - and below
//! it, down the leg, a sock, a point or a bay's black lies where the leg's
//! zone used to put it.

use glam::Vec3;

use crate::plan::beast_foot::FootShape;
use crate::plan::{BeastFeet, BeastHead, Limb, Zone};
use crate::rig::Rig;
use crate::rig::junction::Junctions;

use super::marks::step;

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
    /// Which way the tail's first bone leaves its root.
    pub dock: Option<Vec3>,
    /// The head its plan declared.
    pub head: Option<BeastHead>,
    /// Each foot's shape, by the limb it ends.
    pub feet: Vec<(Limb, FootShape)>,
    /// Where each leg leaves the trunk (#376): what a mark asks instead of a
    /// texel's zone wherever it changes from the barrel to a leg.
    pub junctions: Junctions,
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
        // Along its first bone, or its whole run on a tail of one.
        let dock = rig
            .in_zone(Zone::Tail)
            .get(1)
            .map(|&joint| rig.joints[joint].position);
        let dock = tail.map(|(root, tip)| (dock.unwrap_or(tip) - root).normalize_or(Vec3::NEG_Z));
        Self {
            hips,
            withers,
            height: withers.y.max(f32::EPSILON),
            tail,
            dock,
            head: head.copied(),
            feet: feet.map_or_else(Vec::new, |feet| crate::extremity::beast::shapes(rig, feet)),
            junctions: Junctions::of(rig),
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

    /// How far out along the tail's first bone `at` lies, in back heights.
    pub(crate) fn along_dock(&self, at: Vec3) -> f32 {
        self.tail
            .zip(self.dock)
            .map_or(0.0, |((root, _), dock)| (at - root).dot(dock) / self.height)
    }

    /// How much of a colour picked for the tail a point on it has taken, `0..=1`
    /// (#401): none where the tail leaves the rump, all of it soon after.
    pub(crate) fn tail_share(&self, at: Vec3) -> f32 {
        let (out, down) = (super::DOCK_FADE, super::TAIL_FADE);
        step(out.0, out.1, self.along_dock(at)).max(step(down.0, down.1, self.along_tail(at)))
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
