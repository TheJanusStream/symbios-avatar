//! The capsule graph a body is grown from.
//!
//! A [`Skeleton`] is a graph of [`Node`]s (a position plus a cross-section
//! radius) joined by bones. Nodes are classified by degree — leaf, connector,
//! joint — and the graph is decomposed into [`Chain`]s: maximal chains that run
//! through connectors and terminate at leaves or joints. The cage builder meshes
//! limbs as swept tubes and joints as hulls, so this decomposition is the seam
//! the whole mesher is organised around.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::plan::Zone;

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
    /// How far the cross-section is rolled about the bone, in radians.
    ///
    /// **A ring of an even number of points has a vertex pointing straight down,
    /// and a body part that has to rest on the ground cannot afford one.** With
    /// the cage's ring at four points the section is a diamond standing on
    /// its point, so a foot meshed from the graph contacts the floor along a line
    /// and rocks — measured on the swept foot this replaces, 0.0 mm at the centre
    /// line rising to 19 mm at the edges against a reference sole flat to a few
    /// millimetres (#111). Rolled by half a segment the same four points become an
    /// axis-aligned rectangle standing on a flat edge, and the sole is a face
    /// rather than a ridge.
    ///
    /// Zero everywhere else, and a rotation rather than a new section shape
    /// because it is the smallest thing that expresses the defect: the ring was
    /// never the wrong SIZE, only turned half a segment out of phase with the
    /// ground.
    ///
    /// Depends on the ring frame in exactly the way [`Node::scale`] already does —
    /// it is transported down the limb from a world-up reference, which puts the
    /// frame's second tangent on world up by the time a chain has turned to run
    /// forward along a foot.
    ///
    /// Defaulted **at the field**, not at the container: a `Skeleton` written
    /// before this existed still reads, and reads as unrolled, which is what it
    /// was. A container-level default would only apply to a wholly absent struct
    /// and would leave an older node failing to parse.
    #[serde(default)]
    pub roll: f32,
    /// How far the swept cross-section sits from the joint, in the ring frame.
    ///
    /// **The joint stays where it is and the SURFACE moves, which is the whole
    /// point of it being here rather than on [`Node::position`]** (#125). A
    /// node's position is a joint: bones meet there, the rig rotates about it,
    /// and `face::skull` and `hair::follicle` measure the head in radii about it.
    /// Moving a node to put mass somewhere therefore moves the axis everything
    /// else is measured from — which is exactly what happened when the neck was
    /// leaned back, and is why that lean is bounded at a third of a radius by
    /// what it does to the head's own floor rather than by anatomy.
    ///
    /// This moves only the ring that is swept. A section offset back by `d` with
    /// its depth raised by the same `d` reaches `d` further behind the joint and
    /// stops in the same place in front — a lobe on one side of the axis, which
    /// no centred ellipse can be however it is scaled.
    ///
    /// Same frame and same order as [`Node::scale`]: `x` is broadly lateral on
    /// an upright body and `y` is the other tangent, which is forward for a
    /// vertical bone.
    ///
    /// Defaulted **at the field** for the reason [`Node::roll`] gives: a
    /// `Skeleton` written before this existed still reads, and reads as centred.
    #[serde(default)]
    pub offset: Vec2,
    /// Which part of the body this node belongs to.
    ///
    /// This is what makes the skeleton a *semantic* body plan rather than a bag
    /// of capsules: garments, landmarks, and eventually animation all address
    /// the body through zones instead of node indices, so they work on any body
    /// without knowing which plan built it.
    pub zone: Zone,
    /// A rig-only node: the RIG keeps it — a joint, a bone, a pivot — and the
    /// CAGE skips it entirely.
    ///
    /// **This exists because the joint-hull mesher cannot carry an extra socket
    /// beside large sibling rings, and that has now been measured at three
    /// joints** (#134). A socket must sit past every sibling ring corner along
    /// its own axis (see `cage::joint`), and beside the head's rings that floor
    /// is ~0.12 m against an 0.82-of-the-bone ceiling — a mandible node meshed
    /// as a socket would have to hang outside the head's own surface. #125
    /// measured the same wall at the girdle and the neck for a trapezius.
    ///
    /// A marker is the honest answer for anatomy that is not a limb: a mandible
    /// is a mass fused to the skull with its own hinge. The mass stays the
    /// surface's business (`face::skull`'s submental construction); the HINGE is
    /// the marker — `anim` gets a real pivot and a real bone, and skinning binds
    /// surface to it by the ordinary falloff, without asking the hull for a
    /// socket it cannot give.
    ///
    /// Defaulted **at the field** for the reason [`Node::roll`] gives: a
    /// `Skeleton` written before this existed still reads, and reads as meshed.
    #[serde(default)]
    pub marker: bool,
}

impl Node {
    /// A node with a circular cross-section, in the neutral zone.
    #[must_use]
    pub fn new(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            radius,
            scale: Vec2::ONE,
            roll: 0.0,
            offset: Vec2::ZERO,
            zone: Zone::default(),
            marker: false,
        }
    }

    /// Sets the elliptical cross-section multipliers.
    #[must_use]
    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// Offsets the cross-section from the joint, in ring-frame units of length.
    ///
    /// See [`Node::offset`]. This is how a node says "mass on one side of me"
    /// without moving the joint that everything else is measured from.
    #[must_use]
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Rolls the cross-section about the bone, in radians.
    ///
    /// See [`Node::roll`]. Half a segment — `TAU / (2 * RING)` — is what stands a
    /// ring on a flat edge instead of on a vertex.
    #[must_use]
    pub fn with_roll(mut self, roll: f32) -> Self {
        self.roll = roll;
        self
    }

    /// Marks this node rig-only: a joint the cage does not mesh. See
    /// [`Node::marker`] for why such a node exists at all.
    #[must_use]
    pub fn as_marker(mut self) -> Self {
        self.marker = true;
        self
    }

    /// Tags which part of the body this node belongs to.
    #[must_use]
    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = zone;
        self
    }

    /// Half-extents of this node's cross-section, `(x, y)` in the ring frame.
    #[must_use]
    pub fn half_extents(&self) -> Vec2 {
        self.scale * self.radius
    }

    /// Where this node's cross-section is centred, given its ring frame.
    ///
    /// [`Node::position`] for every node that does not carry an
    /// [`offset`](Node::offset), which is all of them but one.
    #[must_use]
    pub fn centre(&self, u: Vec3, v: Vec3) -> Vec3 {
        self.position + u * self.offset.x + v * self.offset.y
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
/// Named a chain rather than a limb because it is a *meshing* unit: a spine
/// segment and a tail are chains too. [`crate::plan::Limb`] is the different,
/// semantic notion of which of four limb positions a part occupies.
///
/// The endpoints are terminals; every interior entry is a connector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    /// Node indices in traversal order; at least two entries.
    pub nodes: Vec<u32>,
}

impl Chain {
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
        // Meshed degree, not raw degree: a marker bone must not turn its parent
        // into a hull. The head is a CONNECTOR — the chain neck→head→crown
        // sweeps straight through it — and counting the jaw markers would make
        // it a three-socket joint, which is the construction #134 measured as
        // impossible there.
        match self
            .incident_bones(node)
            .into_iter()
            .filter(|&bone| !self.is_marker_bone(bone))
            .count()
        {
            0 | 1 => NodeKind::Leaf,
            2 => NodeKind::Connector,
            _ => NodeKind::Joint,
        }
    }

    /// Whether `bone` touches a marker node, making it rig-only too.
    fn is_marker_bone(&self, bone: usize) -> bool {
        let [a, b] = self.bones[bone];
        self.nodes[a as usize].marker || self.nodes[b as usize].marker
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
    pub fn limbs(&self) -> Result<Vec<Chain>, SkeletonError> {
        self.validate()?;

        // Marker bones are the rig's and never mesh, so they are spent before
        // the walk begins — both so no chain crosses one and so the unanchored-
        // ring check below does not mistake them for an unreached loop.
        let mut used: Vec<bool> = (0..self.bones.len())
            .map(|bone| self.is_marker_bone(bone))
            .collect();
        let mut limbs = Vec::new();

        for start in 0..self.nodes.len() as u32 {
            if self.nodes[start as usize].marker || self.kind(start) == NodeKind::Connector {
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
    fn walk_limb(&self, start: u32, bone: usize, used: &mut [bool]) -> Chain {
        let mut nodes = vec![start];
        let mut current = bone;
        let mut cursor = self.other_end(bone, start);
        used[bone] = true;
        nodes.push(cursor);

        while self.kind(cursor) == NodeKind::Connector {
            // Marker bones are filtered here too: a connector's kind already
            // ignores them, so following one would walk the chain off the
            // meshed graph and onto a rig-only bone.
            let Some(next) = self
                .incident_bones(cursor)
                .into_iter()
                .find(|&b| b != current && !self.is_marker_bone(b))
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

        Chain { nodes }
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
    fn a_marker_is_a_joint_and_not_geometry() {
        // #134. A marker exists because the joint-hull mesher cannot carry an
        // extra socket beside large sibling rings — measured at the girdle and
        // the neck (#125) and at the head (#134) — so anatomy that is not a
        // limb enters the rig without asking the cage for a socket. The
        // contract, both halves: hanging markers anywhere changes NOTHING the
        // mesher sees, and the rig still reaches them.
        let plain = chain(4);
        let mut marked = chain(4);
        // A marker chain off a mid-chain connector: the exact shape that would
        // otherwise turn node 1 into a three-socket hull.
        let pivot = marked.extend_from(1, Node::new(Vec3::new(0.3, 1.0, 0.0), 0.1).as_marker());
        marked.extend_from(pivot, Node::new(Vec3::new(0.6, 1.0, 0.0), 0.1).as_marker());

        assert_eq!(marked.kind(1), NodeKind::Connector, "no hull grows at 1");
        let limbs = marked.limbs().expect("markers do not break decomposition");
        assert_eq!(limbs.len(), 1, "still one limb");
        assert_eq!(limbs[0].nodes, vec![0, 1, 2, 3], "and the same limb");
        assert_eq!(
            plain.limbs().expect("plain chain decomposes")[0].nodes,
            limbs[0].nodes,
        );

        // The rig keeps what the cage skips: both markers are joints, parented
        // through the chain they hang off.
        let rig = crate::Rig::rooted_at(&marked, 0).expect("markers rig");
        assert_eq!(rig.len(), 6);
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
