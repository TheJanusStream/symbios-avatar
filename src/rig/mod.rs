//! Turning a capsule graph into something that can be posed.
//!
//! A [`Skeleton`] is an undirected graph, which is the right shape for meshing
//! but not for animation: posing needs a hierarchy, so that rotating a shoulder
//! carries the whole arm with it. A [`Rig`] is that hierarchy — the same nodes,
//! rooted and ordered parent-before-child, which is also the order glTF and VRM
//! want joints written in.
//!
//! From there, [`skin::bind`] attaches a mesh to the rig and [`Landmarks`]
//! extracts the named anchors that hair, hats, and garments fit against.
//!
//! ```rust
//! use symbios_avatar::{AvatarRecord, Landmark, Limb, Rig, Zone};
//!
//! let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;
//!
//! // Semantic queries instead of bone names — the same call works on a quadruped.
//! let feet = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
//! assert_eq!(feet.len(), 2);
//!
//! // Named anchors for fitting hair, hats, and garments.
//! let marks = rig.landmarks();
//! let hat = marks.get(Landmark::Crown).expect("every body has a crown");
//! let shoulders = marks.span(
//!     Landmark::LimbRoot(Limb::ForeLeft),
//!     Landmark::LimbRoot(Limb::ForeRight),
//! );
//! assert!(shoulders.is_some());
//! # Ok::<(), symbios_avatar::RigError>(())
//! ```

pub mod landmark;
pub mod skin;
pub mod surface;

use glam::Vec3;
use std::collections::VecDeque;
use thiserror::Error;

use crate::plan::{Limb, Zone};
use crate::skeleton::{Skeleton, SkeletonError};

pub use landmark::{Anchor, Landmark, Landmarks};
pub use skin::{Influence, MAX_INFLUENCES, SkinConfig, SkinWeights};
pub use surface::Surface;

/// Where the body's skeleton lies beneath a point on its surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneHit {
    /// The nearest joint's index.
    pub joint: usize,
    /// Distance from the query point to that joint's bone.
    pub distance: f32,
    /// The body's radius where the bone was met.
    pub radius: f32,
    /// The nearest point on the bone itself.
    pub closest: Vec3,
}

impl BoneHit {
    /// How convex the surface is at `point`, given its outward `normal`.
    ///
    /// Returns `0` on a smooth tube and rises toward `1` in a crease. Comparing
    /// the normal against the direction away from the bone is what distinguishes
    /// a genuine cavity — an armpit, a crotch, the inside of an elbow, where the
    /// surface folds back on itself — from merely being past the end of a bone,
    /// which is what a fingertip is. Distance to the bone alone cannot tell those
    /// apart, and reads every limb tip as a deep crease.
    #[must_use]
    pub fn crease(&self, point: Vec3, normal: Vec3) -> f32 {
        let outward = (point - self.closest).normalize_or(normal);
        ((1.0 - normal.dot(outward)) * 1.4).clamp(0.0, 1.0)
    }
}

/// One posable joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    /// The skeleton node this joint came from.
    pub node: u32,
    /// Index of this joint's parent, or `None` for the root.
    pub parent: Option<usize>,
    /// Rest position in body space.
    pub position: Vec3,
    /// The node's radius, which sets how far this joint's influence reaches.
    pub radius: f32,
    /// Which part of the body this joint drives.
    pub zone: Zone,
}

/// Errors raised while rooting a skeleton.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum RigError {
    /// The skeleton itself is malformed.
    #[error("skeleton is not usable: {0}")]
    Skeleton(#[from] SkeletonError),
    /// The requested root does not exist.
    #[error("root node {root} does not exist; the skeleton has {count} nodes")]
    RootOutOfRange {
        /// The requested root.
        root: u32,
        /// How many nodes exist.
        count: usize,
    },
    /// Some nodes cannot be reached from the root.
    ///
    /// A body has to be one connected piece; a detached island could never be
    /// posed, and would have been meshed as a separate surface anyway.
    #[error("nodes {unreachable:?} cannot be reached from root {root}")]
    Disconnected {
        /// The root the walk started from.
        root: u32,
        /// Nodes the walk never reached.
        unreachable: Vec<u32>,
    },
}

/// A skeleton rooted into a posable hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    /// Joints ordered parent-before-child, root first.
    pub joints: Vec<Joint>,
}

impl Rig {
    /// Roots a skeleton at the node best suited to carry the body.
    ///
    /// The pelvis is preferred, then the rest of the torso, then any joint —
    /// the same choice a rigger makes, and the one that keeps a limb's hierarchy
    /// running outward from the body rather than inward from a fingertip.
    ///
    /// # Errors
    ///
    /// Returns [`RigError`] if the skeleton is malformed or disconnected.
    pub fn from_skeleton(skeleton: &Skeleton) -> Result<Self, RigError> {
        skeleton.validate()?;
        // Preference order, not body order: the pelvis carries the most of the
        // body, so rooting there keeps every limb's hierarchy running outward.
        const PREFERRED: [Zone; 3] = [Zone::Pelvis, Zone::Abdomen, Zone::Chest];
        let root = PREFERRED
            .into_iter()
            .find_map(|zone| skeleton.nodes.iter().position(|node| node.zone == zone))
            .unwrap_or(0);
        Self::rooted_at(skeleton, root as u32)
    }

    /// Roots a skeleton at a specific node.
    ///
    /// # Errors
    ///
    /// Returns [`RigError`] if the skeleton is malformed, the root does not
    /// exist, or some nodes cannot be reached from it.
    pub fn rooted_at(skeleton: &Skeleton, root: u32) -> Result<Self, RigError> {
        skeleton.validate()?;
        if root as usize >= skeleton.nodes.len() {
            return Err(RigError::RootOutOfRange {
                root,
                count: skeleton.nodes.len(),
            });
        }

        // Breadth-first, so joints come out parent-before-child: the order glTF
        // requires and the order a pose has to be evaluated in.
        let mut joint_of: Vec<Option<usize>> = vec![None; skeleton.nodes.len()];
        let mut joints = Vec::with_capacity(skeleton.nodes.len());
        let mut queue = VecDeque::from([(root, None)]);

        while let Some((node, parent)) = queue.pop_front() {
            if joint_of[node as usize].is_some() {
                continue;
            }
            let source = skeleton.nodes[node as usize];
            joint_of[node as usize] = Some(joints.len());
            joints.push(Joint {
                node,
                parent,
                position: source.position,
                radius: source.radius,
                zone: source.zone,
            });
            let here = joints.len() - 1;
            for neighbor in skeleton.neighbors(node) {
                if joint_of[neighbor as usize].is_none() {
                    queue.push_back((neighbor, Some(here)));
                }
            }
        }

        let unreachable: Vec<u32> = joint_of
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.is_none())
            .map(|(node, _)| node as u32)
            .collect();
        if !unreachable.is_empty() {
            return Err(RigError::Disconnected { root, unreachable });
        }

        Ok(Self { joints })
    }

    /// How many joints the rig has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.joints.len()
    }

    /// Whether the rig has no joints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// The bone leading into `joint`, as `(start, end)` positions.
    ///
    /// The root has no bone, so its segment is the single point it occupies.
    #[must_use]
    pub fn bone(&self, joint: usize) -> (Vec3, Vec3) {
        let here = self.joints[joint];
        match here.parent {
            Some(parent) => (self.joints[parent].position, here.position),
            None => (here.position, here.position),
        }
    }

    /// The radii at each end of the bone leading into `joint`.
    #[must_use]
    pub fn bone_radii(&self, joint: usize) -> (f32, f32) {
        let here = self.joints[joint];
        match here.parent {
            Some(parent) => (self.joints[parent].radius, here.radius),
            None => (here.radius, here.radius),
        }
    }

    /// The bone nearest `point`, and where on it the nearest spot lies.
    #[must_use]
    pub fn nearest_bone(&self, point: Vec3) -> BoneHit {
        let mut best = BoneHit {
            joint: 0,
            distance: f32::INFINITY,
            radius: 1.0,
            closest: point,
        };
        for joint in 0..self.len() {
            let (start, end) = self.bone(joint);
            let (start_radius, end_radius) = self.bone_radii(joint);
            let axis = end - start;
            let along = if axis.length_squared() <= f32::EPSILON {
                0.0
            } else {
                ((point - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
            };
            let closest = start + axis * along;
            let distance = point.distance(closest);
            if distance < best.distance {
                best = BoneHit {
                    joint,
                    distance,
                    radius: start_radius + (end_radius - start_radius) * along,
                    closest,
                };
            }
        }
        best
    }

    /// Joints belonging to `zone`, in hierarchy order.
    #[must_use]
    pub fn in_zone(&self, zone: Zone) -> Vec<usize> {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.zone == zone)
            .map(|(index, _)| index)
            .collect()
    }

    /// The three joints an [`crate::anim::ik::two_bone`] solve needs for `limb`.
    ///
    /// Returns `[root, mid, tip]` — hip, knee, ankle, or shoulder, elbow, wrist —
    /// or `None` if the body does not articulate that limb far enough to solve.
    ///
    /// The tip is the last joint *above* the extremity where a limb has enough
    /// of them, so a biped solves to its ankle and lets the foot hang off it. A
    /// quadruped's leg has one segment fewer, so its extremity is the tip. Both
    /// are handled by taking whatever the limb has and topping up from the end,
    /// rather than by assuming a particular anatomy.
    #[must_use]
    pub fn limb_chain(&self, limb: Limb) -> Option<[usize; 3]> {
        let mut chain = self.in_zone(Zone::UpperLimb(limb));
        chain.extend(self.in_zone(Zone::LowerLimb(limb)));
        if chain.len() < 3 {
            chain.extend(self.in_zone(Zone::Extremity(limb)));
        }
        if chain.len() < 3 {
            return None;
        }
        let tail = &chain[chain.len() - 3..];
        Some([tail[0], tail[1], tail[2]])
    }

    /// Limbs whose extremity rests near the ground.
    ///
    /// Which limbs carry a body is a property of the body, not of its plan: a
    /// biped stands on two of its four, a quadruped on all four. Reading it off
    /// the rest pose gets both right without either having to declare it, and
    /// gets a future six-legged plan right for free.
    #[must_use]
    pub fn ground_contacts(&self) -> Vec<Limb> {
        let extremities: Vec<(Limb, f32)> = Limb::ALL
            .into_iter()
            .filter_map(|limb| {
                let joint = *self.in_zone(Zone::Extremity(limb)).first()?;
                Some((limb, self.joints[joint].position.y))
            })
            .collect();

        let Some(lowest) = extremities
            .iter()
            .map(|(_, y)| *y)
            .fold(None, |acc: Option<f32>, y| {
                Some(acc.map_or(y, |a| a.min(y)))
            })
        else {
            return Vec::new();
        };
        // A quarter of the body's height of slack: enough that four legs of
        // slightly different length all count, far too little for a T-posed
        // biped's hands to.
        let slack = self.extent() * 0.25;

        extremities
            .into_iter()
            .filter(|(_, y)| *y - lowest <= slack)
            .map(|(limb, _)| limb)
            .collect()
    }

    /// How far a limb can reach: the sum of the bones an IK solve controls.
    ///
    /// The straight-line distance to the extremity is the wrong measure twice
    /// over — it stops short of the bones' true extent when the limb rests bent,
    /// and it includes the foot, which hangs off the chain rather than being part
    /// of it. Summing the chain's bones is what a solver can actually deliver.
    #[must_use]
    pub fn limb_reach(&self, limb: Limb) -> Option<f32> {
        let chain = self.limb_chain(limb)?;
        let joint = |index: usize| self.joints[chain[index]].position;
        Some(joint(0).distance(joint(1)) + joint(1).distance(joint(2)))
    }

    /// Vertical extent of the rig's rest pose, in metres.
    #[must_use]
    pub fn extent(&self) -> f32 {
        let (low, high) = self.joints.iter().fold((f32::MAX, f32::MIN), |acc, joint| {
            (acc.0.min(joint.position.y), acc.1.max(joint.position.y))
        });
        (high - low).max(f32::EPSILON)
    }

    /// Named anchors for fitting hair, hats, and garments.
    #[must_use]
    pub fn landmarks(&self) -> Landmarks {
        Landmarks::from_rig(self)
    }

    /// Joints whose zone satisfies `predicate`, in hierarchy order.
    ///
    /// The general form of a semantic query — "every ground contact", "every
    /// grasper" — which is how motion gets described without naming bones.
    #[must_use]
    pub fn query(&self, predicate: impl Fn(Zone) -> bool) -> Vec<usize> {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| predicate(joint.zone))
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Archetype, BodyPlan, HumanoidParams, Limb, QuadrupedParams};

    #[test]
    fn joints_come_out_parent_before_child() {
        let rig = Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs");
        for (index, joint) in rig.joints.iter().enumerate() {
            if let Some(parent) = joint.parent {
                assert!(parent < index, "joint {index} precedes its parent {parent}");
            }
        }
        assert_eq!(rig.joints[0].parent, None, "exactly one root, first");
        assert_eq!(rig.joints.iter().filter(|j| j.parent.is_none()).count(), 1);
    }

    #[test]
    fn a_body_roots_at_its_pelvis() {
        for archetype in [
            Archetype::Humanoid(HumanoidParams::default()),
            Archetype::Quadruped(QuadrupedParams::default()),
        ] {
            let rig = Rig::from_skeleton(&archetype.skeleton()).expect("rigs");
            assert_eq!(
                rig.joints[0].zone,
                Zone::Pelvis,
                "{} should root at the pelvis",
                archetype.name()
            );
        }
    }

    #[test]
    fn every_node_becomes_exactly_one_joint() {
        let skeleton = QuadrupedParams::default().skeleton();
        let rig = Rig::rooted_at(&skeleton, 0).expect("rigs");
        assert_eq!(rig.len(), skeleton.nodes.len());

        let mut nodes: Vec<u32> = rig.joints.iter().map(|j| j.node).collect();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.len(), skeleton.nodes.len());
    }

    #[test]
    fn semantic_queries_find_limbs_without_naming_bones() {
        let rig = Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs");

        let contacts = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
        assert_eq!(contacts.len(), 2, "a biped stands on two feet");

        let graspers = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if limb.is_fore()));
        assert_eq!(graspers.len(), 2, "and has two hands");

        // Skull and crown; a head takes two nodes to have a dome.
        assert_eq!(rig.in_zone(Zone::Head).len(), 2);
        assert_eq!(rig.in_zone(Zone::UpperLimb(Limb::ForeLeft)).len(), 2);
    }

    #[test]
    fn a_quadruped_stands_on_four_feet() {
        let rig = Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs");
        let contacts = rig.query(|zone| matches!(zone, Zone::Extremity(_)));
        assert_eq!(contacts.len(), 4);
        assert_eq!(rig.in_zone(Zone::Tail).len(), 2);
    }

    #[test]
    fn bones_run_from_parent_to_child() {
        let rig = Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs");
        let (start, end) = rig.bone(0);
        assert_eq!(start, end, "the root has no bone, only a position");

        let child = rig.joints.iter().position(|j| j.parent == Some(0)).unwrap();
        let (start, end) = rig.bone(child);
        assert_eq!(start, rig.joints[0].position);
        assert_eq!(end, rig.joints[child].position);
    }

    #[test]
    fn a_bad_root_is_reported() {
        let skeleton = HumanoidParams::default().skeleton();
        assert!(matches!(
            Rig::rooted_at(&skeleton, 999),
            Err(RigError::RootOutOfRange { root: 999, .. })
        ));
    }

    #[test]
    fn a_detached_island_is_reported() {
        use crate::skeleton::Node;

        let mut skeleton = HumanoidParams::default().skeleton();
        let a = skeleton.add_node(Node::new(Vec3::new(9.0, 9.0, 9.0), 0.1));
        let b = skeleton.add_node(Node::new(Vec3::new(9.0, 9.5, 9.0), 0.1));
        skeleton.connect(a, b);

        match Rig::rooted_at(&skeleton, 0) {
            Err(RigError::Disconnected { unreachable, .. }) => {
                assert_eq!(unreachable, vec![a, b]);
            }
            other => panic!("expected a disconnection report, got {other:?}"),
        }
    }
}
