//! Sweeping a limb chain into a quad tube.
//!
//! A limb is meshed as a sequence of four-sided rings. The ring frame is
//! parallel-transported from segment to segment — rotated by the minimal
//! rotation that carries the old direction onto the new one — so the tube
//! inherits no twist from the reference up-vector, and a socket ring at one end
//! lines up index-for-index with the socket at the other.

use glam::{Quat, Vec2, Vec3};

use super::{CageConfig, CageError, MIN_BONE_LENGTH, Socket};
use crate::mesh::PolyMesh;
use crate::skeleton::{Chain, NodeKind, Skeleton};

/// How many vertices each ring carries.
///
/// **This is the single biggest lever on how fat the cage has to be** (#107).
/// A ring is a control polygon, and Catmull-Clark converges to a curve well
/// inside it: a 4-point diamond delivers about 0.64 of its own half-extent as
/// rendered surface, so every node radius in a body plan has to be inflated by
/// half again to render at the size it means. That inflation is not free — the
/// mesher's socket clearances are computed on the *cage*, so a body ends up
/// clearing a phantom ribcage 1.5× wider than the one anybody sees, and the
/// shoulders are pushed out to match.
///
/// Eight points sit much closer to the curve they approximate, so the same
/// surface can be asked for with a smaller node and every clearance floor comes
/// down with it.
///
/// The value is a constant rather than a config field because it changes the
/// *shape* of `Socket::ring` and `LimbBuild::rings`. Anything reading those
/// should loop to `RING` rather than assume four.
pub(crate) const RING: usize = 4;

/// Ring vertex offsets from a centre, wound counter-clockwise about `u × v`.
///
/// Shared by ring creation and by [`super::Socket::write`], which rewrites the
/// same vertices after a socket slides. Two copies of this drifting apart would
/// tear the cage open along one limb.
pub(crate) fn ring_offsets(u: Vec3, v: Vec3, half: Vec2, roll: f32) -> [Vec3; RING] {
    std::array::from_fn(|corner| {
        let angle = std::f32::consts::TAU * corner as f32 / RING as f32 + roll;
        u * (half.x * angle.cos()) + v * (half.y * angle.sin())
    })
}

/// Half a segment of a ring, which is the roll that stands it on a flat edge
/// rather than on a vertex.
///
/// Named because it is the only value of [`crate::skeleton::Node::roll`] anything
/// has a reason to use, and because it has to follow [`RING`] rather than be written out — at
/// four points it is 45°, at eight 22.5°, and a literal would silently stop
/// meaning "half a segment" the moment #107 lands.
pub(crate) const HALF_SEGMENT: f32 = std::f32::consts::TAU / (2.0 * RING as f32);

/// Rings of one meshed limb, ordered from start to end.
pub(crate) struct LimbBuild {
    /// Ring vertex indices, [`RING`] per ring.
    pub rings: Vec<[u32; RING]>,
    /// Whether the start end is a leaf and needs a cap.
    pub cap_start: bool,
    /// Whether the end end is a leaf and needs a cap.
    pub cap_end: bool,
}

/// Builds the ring chain for `limb`, registering a [`Socket`] at each joint end.
pub(crate) fn build_limb(
    skeleton: &Skeleton,
    limb: &Chain,
    config: &CageConfig,
    mesh: &mut PolyMesh,
    sockets: &mut Vec<Socket>,
) -> Result<LimbBuild, CageError> {
    let nodes = &limb.nodes;
    let count = nodes.len();

    let mut dirs = Vec::with_capacity(count - 1);
    let mut lengths = Vec::with_capacity(count - 1);
    for pair in nodes.windows(2) {
        let a = skeleton.nodes[pair[0] as usize].position;
        let b = skeleton.nodes[pair[1] as usize].position;
        let delta = b - a;
        let length = delta.length();
        if length <= MIN_BONE_LENGTH {
            return Err(CageError::ZeroLengthBone {
                a: pair[0],
                b: pair[1],
            });
        }
        dirs.push(delta / length);
        lengths.push(length);
    }

    let start_is_joint = skeleton.kind(nodes[0]) == NodeKind::Joint;
    let end_is_joint = skeleton.kind(nodes[count - 1]) == NodeKind::Joint;

    let mut rings: Vec<[u32; RING]> = Vec::with_capacity(count + 2);
    let mut dir = dirs[0];
    let (mut u, mut v) = initial_frame(dir);

    if start_is_joint {
        let socket = place_socket(
            skeleton, nodes[0], nodes[1], dirs[0], lengths[0], u, v, config, mesh,
        );
        rings.push(socket.ring);
        sockets.push(socket);
    } else {
        let node = skeleton.nodes[nodes[0] as usize];
        let tip = node.position - dir * (config.tip_extend * node.radius);
        rings.push(push_ring(
            mesh,
            tip,
            u,
            v,
            node.half_extents() * config.tip_shrink,
            node.roll,
        ));
        rings.push(push_ring(
            mesh,
            node.position,
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
    }

    for index in 1..count - 1 {
        let node = skeleton.nodes[nodes[index] as usize];
        // A bisector keeps the ring from pinching where the chain bends.
        let bisector = (dirs[index - 1] + dirs[index]).normalize_or_zero();
        let next = if bisector == Vec3::ZERO {
            dirs[index]
        } else {
            bisector
        };
        (u, v) = transport(u, v, dir, next);
        dir = next;
        rings.push(push_ring(
            mesh,
            node.position,
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
    }

    let final_dir = dirs[count - 2];
    (u, v) = transport(u, v, dir, final_dir);
    dir = final_dir;

    if end_is_joint {
        let socket = place_socket(
            skeleton,
            nodes[count - 1],
            nodes[count - 2],
            -final_dir,
            lengths[count - 2],
            u,
            v,
            config,
            mesh,
        );
        rings.push(socket.ring);
        sockets.push(socket);
    } else {
        let node = skeleton.nodes[nodes[count - 1] as usize];
        rings.push(push_ring(
            mesh,
            node.position,
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
        let tip = node.position + dir * (config.tip_extend * node.radius);
        rings.push(push_ring(
            mesh,
            tip,
            u,
            v,
            node.half_extents() * config.tip_shrink,
            node.roll,
        ));
    }

    Ok(LimbBuild {
        rings,
        cap_start: !start_is_joint,
        cap_end: !end_is_joint,
    })
}

/// Emits the tube walls between consecutive rings, plus any leaf caps.
pub(crate) fn emit_limb_faces(build: &LimbBuild, mesh: &mut PolyMesh) {
    for pair in build.rings.windows(2) {
        let (back, front) = (pair[0], pair[1]);
        for corner in 0..RING {
            let next = (corner + 1) % RING;
            mesh.push_face([back[corner], back[next], front[next], front[corner]]);
        }
    }

    if build.cap_start {
        let ring = build.rings[0];
        // Reversed: the start cap faces against the direction of travel.
        mesh.push_face(ring.iter().rev().copied().collect::<Vec<u32>>());
    }
    if build.cap_end {
        let ring = build.rings[build.rings.len() - 1];
        mesh.push_face(ring.to_vec());
    }
}

/// Places a socket ring on `joint`, heading toward `neighbor`.
#[allow(clippy::too_many_arguments)]
fn place_socket(
    skeleton: &Skeleton,
    joint: u32,
    neighbor: u32,
    away: Vec3,
    bone_length: f32,
    u: Vec3,
    v: Vec3,
    config: &CageConfig,
    mesh: &mut PolyMesh,
) -> Socket {
    let hub = skeleton.nodes[joint as usize];
    let next = skeleton.nodes[neighbor as usize];

    let max_dist = config.max_socket_fraction * bone_length;
    let dist = (config.socket_distance * hub.radius).min(max_dist);
    let blend = (dist / bone_length).clamp(0.0, 1.0);
    let half = hub.half_extents().lerp(next.half_extents(), blend);
    // The roll travels with the half-extents, for the same reason they blend at
    // all: a socket that stood on a flat edge while the node beside it stood on a
    // vertex would tear the section as it slid.
    let roll = hub.roll + (next.roll - hub.roll) * blend;

    let center = hub.position + away * dist;
    let ring = push_ring(mesh, center, u, v, half, roll);

    Socket {
        joint,
        base: hub.position,
        toward: neighbor,
        dir: away,
        u,
        v,
        half,
        roll,
        joint_half: hub.half_extents(),
        neighbor_half: next.half_extents(),
        bone_length,
        dist,
        base_dist: dist,
        max_dist,
        ring,
    }
}

/// Appends a [`RING`]-vertex ring, wound counter-clockwise about `u × v`.
pub(crate) fn push_ring(
    mesh: &mut PolyMesh,
    center: Vec3,
    u: Vec3,
    v: Vec3,
    half: Vec2,
    roll: f32,
) -> [u32; RING] {
    ring_offsets(u, v, half, roll).map(|offset| mesh.push_vertex(center + offset))
}

/// An orthonormal frame `(u, v)` with `u × v == dir`.
///
/// The world-up reference makes `u` broadly lateral for an upright body, which
/// is what [`crate::skeleton::Node::scale`] is documented against; transport
/// then carries that choice down the whole limb.
pub(crate) fn initial_frame(dir: Vec3) -> (Vec3, Vec3) {
    let reference = if dir.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let u = reference.cross(dir).normalize();
    let v = dir.cross(u);
    (u, v)
}

/// Rotates a frame by the minimal rotation carrying `from` onto `to`.
pub(crate) fn transport(u: Vec3, v: Vec3, from: Vec3, to: Vec3) -> (Vec3, Vec3) {
    let axis = from.cross(to);
    let sine = axis.length();
    if sine <= 1e-6 {
        return (u, v);
    }
    let angle = from.dot(to).clamp(-1.0, 1.0).acos();
    let rotation = Quat::from_axis_angle(axis / sine, angle);
    (rotation * u, rotation * v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_initial_frame_is_right_handed() {
        for dir in [
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            Vec3::new(1.0, 2.0, -3.0).normalize(),
        ] {
            let (u, v) = initial_frame(dir);
            assert!((u.length() - 1.0).abs() < 1e-5);
            assert!((v.length() - 1.0).abs() < 1e-5);
            assert!(u.dot(v).abs() < 1e-5);
            assert!(
                (u.cross(v) - dir).length() < 1e-5,
                "u x v must reproduce the travel direction"
            );
        }
    }

    #[test]
    fn transport_tracks_the_direction_without_spinning() {
        let from = Vec3::Y;
        let to = Vec3::new(1.0, 1.0, 0.0).normalize();
        let (u, v) = initial_frame(from);
        let (tu, tv) = transport(u, v, from, to);

        assert!(
            (tu.cross(tv) - to).length() < 1e-5,
            "frame follows the bend"
        );
        // Minimal rotation: the component of `u` about the rotation axis is kept.
        let axis = from.cross(to).normalize();
        assert!((tu.dot(axis) - u.dot(axis)).abs() < 1e-5);
    }

    #[test]
    fn transport_is_stable_for_parallel_directions() {
        let (u, v) = initial_frame(Vec3::Z);
        let (tu, tv) = transport(u, v, Vec3::Z, Vec3::Z);
        assert_eq!((tu, tv), (u, v));
    }
}
