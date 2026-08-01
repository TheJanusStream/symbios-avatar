//! The capsule graph a body is grown from.
//!
//! A [`Skeleton`] is a graph of [`Node`]s (a position plus a cross-section
//! radius) joined by bones. Nodes are classified by degree — leaf, connector,
//! joint — and the graph is decomposed into [`Limb`]s: maximal chains that run
//! through connectors and terminate at leaves or joints. The cage builder meshes
//! limbs as swept tubes and joints as hulls, so this decomposition is the seam
//! the whole mesher is organised around.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One key ball of the skeleton: where the surface passes and how wide it is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Position in body space.
    pub position: Vec3,
    /// Cross-section radius before [`Node::scale`] is applied.
    pub radius: f32,
    /// Per-axis half-extent multipliers in the ring frame, letting a chest be
    /// wider than it is deep. `x` runs along the frame's first tangent, `y`
    /// along its second; the frame is parallel-transported down each limb from
    /// a world-up reference, so for an upright body `x` is broadly lateral.
    pub scale: Vec2,
}

impl Node {
    /// A node with a circular cross-section.
    #[must_use]
    pub fn new(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            radius,
            scale: Vec2::ONE,
        }
    }

    /// Sets the elliptical cross-section multipliers.
    #[must_use]
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Half-extents of this node's cross-section, `(x, y)` in the ring frame.
    #[must_use]
    pub fn half_extents(&self) -> Vec2 {
        self.scale * self.radius
    }
}

/// How a node participates in the graph, by degree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Degree 1 — a limb tip; the cage caps it.
    Leaf,
    /// Degree 2 — an interior ring of a limb.
    Connector,
    /// Degree 3 or more — the cage builds a hull with one opening per limb.
    Joint,
}

/// A maximal chain of nodes running between two terminals (leaf or joint).
///
/// The endpoints are terminals; every interior entry is a connector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limb {
    /// Node indices in traversal order; at least two entries.
    pub nodes: Vec<u32>,
}

impl Limb {
    /// The first node index.
    #[must_use]
    pub fn start(&self) -> u32 {
        self.nodes[0]
    }

    /// The last node index.
    #[must_use]
    pub fn end(&self) -> u32 {
        self.nodes[self.nodes.len() - 1]
    }
}

/// Errors raised while validating or decomposing a skeleton.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum SkeletonError {
    /// A bone referenced a node index that does not exist.
    #[error("bone {bone} references node {node}, but the skeleton has {count} nodes")]
    NodeOutOfRange {
        /// Index of the offending bone.
        bone: usize,
        /// The out-of-range node index.
        node: u32,
        /// How many nodes exist.
        count: usize,
    },
    /// A bone connected a node to itself.
    #[error("bone {bone} connects node {node} to itself")]
    SelfLoop {
        /// Index of the offending bone.
        bone: usize,
        /// The repeated node index.
        node: u32,
    },
    /// The same pair of nodes was connected twice.
    #[error("bone {bone} duplicates the connection between nodes {a} and {b}")]
    DuplicateBone {
        /// Index of the offending bone.
        bone: usize,
        /// First node of the pair.
        a: u32,
        /// Second node of the pair.
        b: u32,
    },
    /// A node has no bones at all, so no surface can be grown around it.
    #[error("node {node} is isolated; every node needs at least one bone")]
    IsolatedNode {
        /// The unreachable node index.
        node: u32,
    },
    /// A node carries a non-positive or non-finite radius.
    #[error("node {node} has radius {radius}, which must be finite and positive")]
    BadRadius {
        /// The offending node index.
        node: u32,
        /// The rejected radius.
        radius: f32,
    },
    /// The skeleton is empty.
    #[error("the skeleton has no bones")]
    Empty,
    /// A ring of connectors with no terminal to start from.
    #[error("nodes {nodes:?} form a closed ring of connectors with no leaf or joint to anchor it")]
    UnanchoredRing {
        /// The node indices making up the ring.
        nodes: Vec<u32>,
    },
}

/// A graph of key balls: the input to the cage builder.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    /// The key balls.
    pub nodes: Vec<Node>,
    /// Undirected bones, each a pair of indices into [`Skeleton::nodes`].
    pub bones: Vec<[u32; 2]>,
}

impl Skeleton {
    /// An empty skeleton.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a node and returns its index.
    pub fn add_node(&mut self, node: Node) -> u32 {
        let index = self.nodes.len() as u32;
        self.nodes.push(node);
        index
    }

    /// Appends a bone between two existing nodes.
    pub fn connect(&mut self, a: u32, b: u32) {
        self.bones.push([a, b]);
    }

    /// Adds a node and immediately connects it to `parent`, returning its index.
    pub fn extend_from(&mut self, parent: u32, node: Node) -> u32 {
        let index = self.add_node(node);
        self.connect(parent, index);
        index
    }

    /// Checks structural invariants the cage builder relies on.
    ///
    /// # Errors
    ///
    /// Returns the first violated invariant: out-of-range or duplicated bones,
    /// self-loops, isolated nodes, or non-positive radii.
    pub fn validate(&self) -> Result<(), SkeletonError> {
        if self.bones.is_empty() {
            return Err(SkeletonError::Empty);
        }
        let count = self.nodes.len();
        for (bone, &[a, b]) in self.bones.iter().enumerate() {
            for node in [a, b] {
                if node as usize >= count {
                    return Err(SkeletonError::NodeOutOfRange { bone, node, count });
                }
            }
            if a == b {
                return Err(SkeletonError::SelfLoop { bone, node: a });
            }
            let key = (a.min(b), a.max(b));
            if self.bones[..bone]
                .iter()
                .any(|&[c, d]| (c.min(d), c.max(d)) == key)
            {
                return Err(SkeletonError::DuplicateBone { bone, a, b });
            }
        }
        for (index, node) in self.nodes.iter().enumerate() {
            if !node.radius.is_finite() || node.radius <= 0.0 {
                return Err(SkeletonError::BadRadius {
                    node: index as u32,
                    radius: node.radius,
                });
            }
            if self.degree(index as u32) == 0 {
                return Err(SkeletonError::IsolatedNode { node: index as u32 });
            }
        }
        Ok(())
    }

    /// How many bones touch `node`.
    #[must_use]
    pub fn degree(&self, node: u32) -> usize {
        self.bones
            .iter()
            .filter(|&&[a, b]| a == node || b == node)
            .count()
    }

    /// Nodes sharing a bone with `node`, in bone order.
    #[must_use]
    pub fn neighbors(&self, node: u32) -> Vec<u32> {
        self.bones
            .iter()
            .filter_map(|&[a, b]| match (a == node, b == node) {
                (true, false) => Some(b),
                (false, true) => Some(a),
                _ => None,
            })
            .collect()
    }

    /// Classifies `node` by degree.
    #[must_use]
    pub fn kind(&self, node: u32) -> NodeKind {
        match self.degree(node) {
            0 | 1 => NodeKind::Leaf,
            2 => NodeKind::Connector,
            _ => NodeKind::Joint,
        }
    }

    /// Decomposes the graph into limbs: chains between leaves and joints.
    ///
    /// Every bone lands in exactly one limb. Order is deterministic — limbs are
    /// discovered by walking terminals in index order — so a given skeleton
    /// always meshes to the same vertex layout.
    ///
    /// # Errors
    ///
    /// Returns [`SkeletonError`] if [`Skeleton::validate`] fails, or
    /// [`SkeletonError::UnanchoredRing`] if bones remain that form a closed loop
    /// of connectors with no terminal on it.
    pub fn limbs(&self) -> Result<Vec<Limb>, SkeletonError> {
        self.validate()?;

        let mut used = vec![false; self.bones.len()];
        let mut limbs = Vec::new();

        for start in 0..self.nodes.len() as u32 {
            if self.kind(start) == NodeKind::Connector {
                continue;
            }
            for bone in self.incident_bones(start) {
                if used[bone] {
                    continue;
                }
                limbs.push(self.walk_limb(start, bone, &mut used));
            }
        }

        if let Some(bone) = used.iter().position(|&u| !u) {
            let mut ring = vec![self.bones[bone][0]];
            let mut cursor = self.bones[bone][1];
            let mut here = bone;
            used[bone] = true;
            while cursor != ring[0] {
                ring.push(cursor);
                let next = self
                    .incident_bones(cursor)
                    .into_iter()
                    .find(|&b| !used[b])
                    .expect("a connector ring always continues until it closes");
                used[next] = true;
                cursor = self.other_end(next, cursor);
                here = next;
            }
            let _ = here;
            return Err(SkeletonError::UnanchoredRing { nodes: ring });
        }

        Ok(limbs)
    }

    /// Indices of the bones touching `node`.
    fn incident_bones(&self, node: u32) -> Vec<usize> {
        self.bones
            .iter()
            .enumerate()
            .filter(|(_, bone)| bone[0] == node || bone[1] == node)
            .map(|(index, _)| index)
            .collect()
    }

    /// The end of `bone` that is not `node`.
    fn other_end(&self, bone: usize, node: u32) -> u32 {
        let [a, b] = self.bones[bone];
        if a == node { b } else { a }
    }

    /// Walks from `start` across `bone` until the next terminal.
    fn walk_limb(&self, start: u32, bone: usize, used: &mut [bool]) -> Limb {
        let mut nodes = vec![start];
        let mut current = bone;
        let mut cursor = self.other_end(bone, start);
        used[bone] = true;
        nodes.push(cursor);

        while self.kind(cursor) == NodeKind::Connector {
            let Some(next) = self
                .incident_bones(cursor)
                .into_iter()
                .find(|&b| b != current)
            else {
                break;
            };
            if used[next] {
                break;
            }
            used[next] = true;
            current = next;
            cursor = self.other_end(next, cursor);
            nodes.push(cursor);
        }

        Limb { nodes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(count: usize) -> Skeleton {
        let mut skel = Skeleton::new();
        let mut previous = skel.add_node(Node::new(Vec3::ZERO, 0.2));
        for i in 1..count {
            previous = skel.extend_from(previous, Node::new(Vec3::new(0.0, i as f32, 0.0), 0.2));
        }
        skel
    }

    #[test]
    fn a_chain_is_one_limb() {
        let skel = chain(4);
        let limbs = skel.limbs().expect("chain decomposes");
        assert_eq!(limbs.len(), 1);
        assert_eq!(limbs[0].nodes, vec![0, 1, 2, 3]);
        assert_eq!(skel.kind(0), NodeKind::Leaf);
        assert_eq!(skel.kind(1), NodeKind::Connector);
    }

    #[test]
    fn a_joint_splits_limbs_and_uses_every_bone() {
        let mut skel = chain(3);
        let hub = 1;
        skel.extend_from(hub, Node::new(Vec3::new(1.0, 1.0, 0.0), 0.2));
        assert_eq!(skel.kind(hub), NodeKind::Joint);

        let limbs = skel.limbs().expect("tripod decomposes");
        assert_eq!(limbs.len(), 3);
        assert!(limbs.iter().all(|l| l.nodes.len() == 2));
        assert!(limbs.iter().all(|l| l.start() == hub || l.end() == hub));
    }

    #[test]
    fn every_bone_lands_in_exactly_one_limb() {
        let mut skel = chain(5);
        skel.extend_from(2, Node::new(Vec3::new(1.0, 2.0, 0.0), 0.2));
        skel.extend_from(2, Node::new(Vec3::new(-1.0, 2.0, 0.0), 0.2));
        let limbs = skel.limbs().expect("decomposes");
        let bones: usize = limbs.iter().map(|l| l.nodes.len() - 1).sum();
        assert_eq!(bones, skel.bones.len());
    }

    #[test]
    fn decomposition_is_deterministic() {
        let mut skel = chain(4);
        skel.extend_from(1, Node::new(Vec3::new(1.0, 1.0, 0.0), 0.2));
        assert_eq!(skel.limbs().unwrap(), skel.limbs().unwrap());
    }

    #[test]
    fn a_closed_ring_of_connectors_is_rejected() {
        let mut skel = Skeleton::new();
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::FRAC_PI_2;
            skel.add_node(Node::new(Vec3::new(angle.cos(), 0.0, angle.sin()), 0.2));
        }
        for i in 0..4u32 {
            skel.connect(i, (i + 1) % 4);
        }
        assert!(matches!(
            skel.limbs(),
            Err(SkeletonError::UnanchoredRing { .. })
        ));
    }

    #[test]
    fn validation_catches_structural_faults() {
        let mut empty = Skeleton::new();
        empty.add_node(Node::new(Vec3::ZERO, 0.2));
        assert_eq!(empty.validate(), Err(SkeletonError::Empty));

        let mut loops = chain(2);
        loops.connect(0, 0);
        assert!(matches!(
            loops.validate(),
            Err(SkeletonError::SelfLoop { .. })
        ));

        let mut duplicate = chain(2);
        duplicate.connect(1, 0);
        assert!(matches!(
            duplicate.validate(),
            Err(SkeletonError::DuplicateBone { .. })
        ));

        let mut orphan = chain(2);
        orphan.add_node(Node::new(Vec3::new(9.0, 9.0, 9.0), 0.2));
        assert!(matches!(
            orphan.validate(),
            Err(SkeletonError::IsolatedNode { node: 2 })
        ));

        let mut out_of_range = chain(2);
        out_of_range.connect(0, 7);
        assert!(matches!(
            out_of_range.validate(),
            Err(SkeletonError::NodeOutOfRange { node: 7, .. })
        ));

        let mut bad_radius = chain(2);
        bad_radius.nodes[1].radius = 0.0;
        assert!(matches!(
            bad_radius.validate(),
            Err(SkeletonError::BadRadius { node: 1, .. })
        ));
    }

    #[test]
    fn skeletons_round_trip_through_serde() {
        let skel = chain(3);
        let json = serde_json::to_string(&skel).expect("serialises");
        let back: Skeleton = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(skel, back);
    }
}
