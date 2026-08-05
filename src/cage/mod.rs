//! Builds a quad-dominant control cage around a [`Skeleton`].
//!
//! The cage is the topological skeleton of the body: coarse, all-quad down the
//! limbs, and shaped so that [`crate::subdiv::catmull_clark`] turns it into a
//! smooth surface with edge loops running the way a deforming character needs
//! them. It follows the B-Mesh construction (Ji, Liu & Wang, 2010):
//!
//! 1. Decompose the graph into limbs and joints ([`Skeleton::limbs`]).
//! 2. Sweep a four-sided ring along each limb, parallel-transporting the frame
//!    so the tube never twists.
//! 3. At each joint, place one socket ring per incident limb, hull the sockets,
//!    and delete the socket facets to leave openings the tubes plug into.
//! 4. Cap leaf ends.
//!
//! Because socket rings are *shared* between the joint hull and the limb tube,
//! no stitching pass is needed and the result is a closed 2-manifold by
//! construction — which [`PolyMesh::manifold_report`] verifies.

mod joint;
pub(crate) mod limb;

use glam::{Vec2, Vec3};
use std::collections::BTreeMap;
use thiserror::Error;

use crate::hull::HullError;
use crate::mesh::PolyMesh;
use crate::skeleton::{Skeleton, SkeletonError};

/// Bones shorter than this are treated as degenerate.
pub(crate) const MIN_BONE_LENGTH: f32 = 1e-5;

/// Tuning for [`build_cage`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CageConfig {
    /// How far a socket ring sits from its joint centre, in joint radii.
    ///
    /// Around `1.0` places sockets on the joint's ball surface. Smaller values
    /// tuck limbs into the joint; larger ones stretch it into a star.
    pub socket_distance: f32,
    /// Hard ceiling on socket distance as a fraction of the bone it sits on,
    /// so a fat joint on a short bone cannot overshoot the next node.
    pub max_socket_fraction: f32,
    /// Extra clearance a socket keeps beyond its siblings' reach, as a fraction
    /// of its own radius.
    ///
    /// A socket only becomes an opening in the hull if its plane strictly
    /// supports the joint; a sibling corner landing exactly on that plane gets
    /// absorbed into the facet and the opening is lost. This is the daylight
    /// that prevents the tie.
    pub socket_margin: f32,
    /// How far past a leaf node the tip ring sits, in node radii.
    pub tip_extend: f32,
    /// Radius multiplier for the tip ring, rounding the cap after subdivision.
    pub tip_shrink: f32,
    /// Factor a socket is pushed outward by when it fails to surface on the
    /// joint hull.
    pub socket_push_step: f32,
    /// How many times a socket may be pushed before the joint is rejected.
    pub socket_push_attempts: usize,
    /// Fraction of the summed socket radii below which two sockets on one joint
    /// count as grossly overlapping.
    ///
    /// Deliberately permissive: limbs on a real body crowd their joints — human
    /// thighs touch at the hip — and the hull resolves most tight cases on its
    /// own. This only catches sockets that genuinely interpenetrate, where the
    /// hull's failure would be far harder to read.
    pub socket_clearance: f32,
}

impl Default for CageConfig {
    fn default() -> Self {
        Self {
            socket_distance: 0.9,
            max_socket_fraction: 0.82,
            socket_margin: 0.06,
            tip_extend: 0.22,
            tip_shrink: 0.72,
            socket_push_step: 1.15,
            socket_push_attempts: 6,
            socket_clearance: 0.5,
        }
    }
}

/// Errors raised while building a cage.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum CageError {
    /// The skeleton itself is malformed.
    #[error("skeleton is not usable: {0}")]
    Skeleton(#[from] SkeletonError),
    /// Two nodes sit on top of each other, leaving no direction to sweep along.
    #[error("bone between nodes {a} and {b} has no length")]
    ZeroLengthBone {
        /// First node of the bone.
        a: u32,
        /// Second node of the bone.
        b: u32,
    },
    /// Two limbs leave a joint so close together that their sockets intersect.
    ///
    /// The fix belongs in the skeleton: widen the joint, spread the limbs, or
    /// insert an intermediate node so each limb gets its own socket. A humanoid
    /// pelvis needs real width before both legs can attach to it.
    #[error(
        "at joint {joint}, the sockets toward nodes {first} and {second} overlap \
         (centres {separation:.4} apart, need {needed:.4}); widen the joint or spread the limbs"
    )]
    SocketsOverlap {
        /// The joint node.
        joint: u32,
        /// Neighbour node of the first limb.
        first: u32,
        /// Neighbour node of the second limb.
        second: u32,
        /// Distance between the two socket centres.
        separation: f32,
        /// Distance required for clearance.
        needed: f32,
    },
    /// The joint's socket points could not be hulled.
    #[error("joint {joint} could not be hulled: {source}")]
    JointHull {
        /// The joint node.
        joint: u32,
        /// Why the hull failed.
        source: HullError,
    },
    /// A socket stayed buried inside the joint hull even after being pushed out.
    ///
    /// The reported distances say exactly what to change: the socket has to
    /// clear its siblings' reach along its own axis, and its bone is too short
    /// to let it get there.
    #[error(
        "at joint {joint}, the socket toward node {toward} must sit {needed:.4} from the joint \
         centre to clear its siblings, but its bone only allows {available:.4}. \
         Lengthen that bone, slim the joint, or spread the limbs apart."
    )]
    SocketNotOnHull {
        /// The joint node.
        joint: u32,
        /// Neighbour node of the limb whose socket failed.
        toward: u32,
        /// Distance the socket needs to become an opening.
        needed: f32,
        /// Largest distance its bone permits.
        available: f32,
    },
}

/// One limb's opening in a joint hull.
///
/// The four ring vertices are shared with the limb tube, so moving a socket
/// moves both surfaces together and they stay welded.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Socket {
    /// The joint this socket belongs to.
    pub joint: u32,
    /// The joint's position, the origin the socket is placed from.
    pub base: Vec3,
    /// Neighbour node the limb heads toward; used for diagnostics.
    pub toward: u32,
    /// Unit direction away from the joint.
    pub dir: Vec3,
    /// First tangent of the ring frame.
    pub u: Vec3,
    /// Second tangent of the ring frame; `u × v` is the limb's travel direction.
    pub v: Vec3,
    /// Ring half-extents along `u` and `v`, derived from [`Socket::dist`].
    pub half: Vec2,
    /// How far the section is rolled about the bone, blended between the joint
    /// and its neighbour the same way `half` is. See
    /// [`crate::skeleton::Node::roll`].
    pub roll: f32,
    /// The joint's own half-extents, the ring's size at distance zero.
    pub joint_half: Vec2,
    /// The neighbour node's half-extents, the ring's size at the far end.
    pub neighbor_half: Vec2,
    /// Length of the bone the socket slides along.
    pub bone_length: f32,
    /// Current distance from the joint centre.
    pub dist: f32,
    /// The distance the socket was first placed at; spacing never pulls it in
    /// closer than this, so a joint cannot collapse into its own limbs.
    pub base_dist: f32,
    /// Largest distance the socket may be moved to.
    pub max_dist: f32,
    /// The shared ring vertices, [`limb::RING`] of them.
    pub ring: [u32; limb::RING],
}

impl Socket {
    /// Centre of the socket ring.
    pub(crate) fn center(&self) -> Vec3 {
        self.base + self.dir * self.dist
    }

    /// Moves the socket along its limb, re-deriving the ring's size.
    ///
    /// The radius has to follow the distance: a socket slid down a tapering limb
    /// keeps the joint's girth otherwise, which both looks wrong and defeats the
    /// angular-spread fix it was moved for.
    pub(crate) fn set_dist(&mut self, dist: f32) {
        self.dist = dist.clamp(self.base_dist, self.max_dist.max(self.base_dist));
        let blend = (self.dist / self.bone_length).clamp(0.0, 1.0);
        self.half = self.joint_half.lerp(self.neighbor_half, blend);
    }

    /// Effective radius of the ring for clearance tests.
    ///
    /// The larger half-extent sits between the ring's inradius and circumradius:
    /// permissive enough not to reject a tight but valid joint, strict enough to
    /// catch squares that truly interpenetrate.
    pub(crate) fn clearance_radius(&self) -> f32 {
        self.half.max_element()
    }

    /// Rewrites the shared ring vertices after `dist` changes.
    pub(crate) fn write(&self, positions: &mut [Vec3]) {
        let center = self.center();
        for (index, offset) in self
            .ring
            .iter()
            .zip(limb::ring_offsets(self.u, self.v, self.half, self.roll))
        {
            positions[*index as usize] = center + offset;
        }
    }
}

/// Grows a control cage around `skeleton`.
///
/// The result is a closed, consistently wound polygon mesh: quads along limbs
/// and caps, convex polygons over joints. Feed it to
/// [`crate::subdiv::catmull_clark`] to smooth it and make it all-quad.
///
/// # Errors
///
/// Returns [`CageError`] if the skeleton is malformed, a bone is degenerate, or
/// a joint cannot be resolved — see [`CageError::SocketsOverlap`] and
/// [`CageError::SocketNotOnHull`], both of which point at a skeleton that needs
/// adjusting rather than an internal failure.
pub fn build_cage(skeleton: &Skeleton, config: &CageConfig) -> Result<PolyMesh, CageError> {
    let limbs = skeleton.limbs()?;
    let mut mesh = PolyMesh::new();
    let mut sockets: Vec<Socket> = Vec::new();
    let mut builds = Vec::with_capacity(limbs.len());

    for limb in &limbs {
        builds.push(limb::build_limb(
            skeleton,
            limb,
            config,
            &mut mesh,
            &mut sockets,
        )?);
    }

    // Joints are resolved before limb faces are emitted only for readability;
    // faces reference indices, so socket pushes are picked up either way.
    let mut by_joint: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (index, socket) in sockets.iter().enumerate() {
        by_joint.entry(socket.joint).or_default().push(index);
    }
    for (joint, indices) in by_joint {
        joint::build_joint(skeleton, joint, &indices, &mut sockets, config, &mut mesh)?;
    }

    for build in &builds {
        limb::emit_limb_faces(build, &mut mesh);
    }

    Ok(mesh)
}
