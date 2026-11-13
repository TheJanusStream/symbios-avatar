//! Sweeping a limb chain into a quad tube.
//!
//! A limb is meshed as a sequence of [`RING`]-sided rings. The ring frame is
//! parallel-transported from segment to segment — rotated by the minimal
//! rotation that carries the old direction onto the new one — so the tube
//! inherits no twist from the reference up-vector, and a socket ring at one end
//! lines up index-for-index with the socket at the other. On a beast's head chain
//! the rings of a tight bend are then drawn out of each other, column by column
//! (`straighten_columns`, #395).

use glam::{Quat, Vec2, Vec3};

use super::{CageConfig, CageError, MIN_BONE_LENGTH, Socket};
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::skeleton::{Chain, NodeKind, Skeleton};

/// How many vertices each ring carries.
///
/// **This is the single biggest lever on how fat the cage has to be.**
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
pub(crate) const RING: usize = 8;

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
/// has a reason to use, and because it has to follow [`RING`] rather than be
/// written out — at four points it is 45°, at eight 22.5°, and a literal would
/// silently stop meaning "half a segment" whenever [`RING`] moves.
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

/// Draws the rings of a beast's head chain out of each other at a tight bend
/// (#395), column by column.
///
/// **A bisector ring keeps a tube whole only while the bone is longer than the
/// bend eats.** A beast's head nods about the top of its neck, through up to a
/// hundred degrees in a bone shorter than the skull is wide, so on the throat
/// side the cranium's ring reaches back past the occiput's: the skin between
/// them runs backwards, in a hairpin the shaping cannot undo, because a map from
/// where a vertex is to where it goes sends both legs of a hairpin to the same
/// place. So a vertex that runs backwards to the next ring in its column, along
/// the bone between their centres, is drawn toward the middle of its two
/// neighbours, as far as it runs backwards, and the column straightens across
/// the bend. Nothing moves that has room: a ring on a straight run or a gentle
/// bend is where it was, and the far wall of the bend always is. The head and the
/// neck's upper run are projected from these vertices later, so what moves is
/// where the skin is sampled and not the surface it is drawn onto.
fn straighten_columns(
    skeleton: &Skeleton,
    nodes: &[u32],
    rings: &[[u32; RING]],
    mesh: &mut PolyMesh,
) {
    // The first ring is a socket the joint's hull still moves, and the last
    // interior one is the last with a ring beyond it on a bone.
    let interior = nodes.len() - 2;
    let ring_of = |node: usize| rings[node];
    let centre = |node: usize| skeleton.nodes[nodes[node] as usize].position;
    let radius = |node: usize| skeleton.nodes[nodes[node] as usize].radius;
    for _ in 0..COLUMN_PASSES {
        for column in 0..RING {
            for node in 1..=interior.saturating_sub(1) {
                let along = (centre(node + 1) - centre(node)).normalize_or_zero();
                let at = |node: usize| mesh.positions[ring_of(node)[column] as usize];
                let room = (at(node + 1) - at(node)).dot(along);
                let radius = radius(node);
                let short =
                    ((COLUMN_ROOM * radius - room) / (COLUMN_SPAN * radius)).clamp(0.0, 1.0);
                if short <= 0.0 {
                    continue;
                }
                let drawn = at(node).lerp(0.5 * (at(node - 1) + at(node + 1)), short);
                mesh.positions[ring_of(node)[column] as usize] = drawn;
            }
        }
    }
}

/// How many times a head chain's columns are straightened: each pass takes a
/// hairpin about half of the way to its chord.
/// Provenance: **tuned against the inward-face census** (#395).
const COLUMN_PASSES: usize = 4;

/// How far along the bone a column's next vertex has to stand before it is left
/// alone, in the ring's own radii: rings closer than this are crowding, and the
/// fold starts from there.
/// Provenance: **tuned against the inward-face census** (#395).
const COLUMN_ROOM: f32 = 0.1;

/// Over how much shortfall, in the ring's own radii, a vertex goes from staying
/// where it is to the middle of its neighbours, so that a ring's slide with a
/// slider is smooth. Provenance: **tuned against the inward-face census** (#395).
const COLUMN_SPAN: f32 = 0.3;

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
    // A bridged leg whose rings are drawn out of round starts its frame with
    // its first axis across the body (#374), so a section says across and
    // fore-and-aft wherever the leg leans. Transported from world up, a leg
    // dropping from the midline at a slight lean out and forward turned its
    // first axis forty-five degrees round, and a thigh drawn deep from the
    // side came out deep on the diagonal. A round leg keeps the frame it
    // always had, and so its socket's fit on the hull: turned, the fore
    // legs' sockets crowded the spine's off the forechest at the envelope's
    // low corner. A person has no bridge.
    let shaped = skeleton.nodes[nodes[1] as usize].scale != Vec2::ONE;
    let (mut u, mut v) = if shaped && skeleton.bridge_aim(nodes[0], nodes[1]).is_some() {
        lateral_frame(dir)
    } else {
        initial_frame(dir)
    };

    if start_is_joint {
        // A bridged bone's socket faces the joint the limb hangs from rather
        // than the node it sweeps to (`Skeleton::bridge_aim`), its ring turned
        // by the least rotation onto that face.
        let away = aim(skeleton, nodes[0], nodes[1]).unwrap_or(dirs[0]);
        let (su, sv) = transport(u, v, dirs[0], away);
        let socket = place_socket(
            skeleton, nodes[0], nodes[1], away, lengths[0], su, sv, config, mesh,
        );
        rings.push(socket.ring);
        sockets.push(socket);
    } else {
        let node = skeleton.nodes[nodes[0] as usize];
        let tip = node.centre(u, v) - dir * (config.tip_extend * node.radius);
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
            node.centre(u, v),
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
    }

    let carries_head = skeleton.beast_head.as_ref().is_some_and(|head| {
        nodes
            .iter()
            .any(|&node| skeleton.nodes[node as usize].position == head.origin)
    });
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
            node.centre(u, v),
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
    }
    if carries_head {
        straighten_columns(skeleton, nodes, &rings, mesh);
    }

    let final_dir = dirs[count - 2];
    (u, v) = transport(u, v, dir, final_dir);
    dir = final_dir;

    if end_is_joint {
        let away = aim(skeleton, nodes[count - 1], nodes[count - 2]).unwrap_or(-final_dir);
        let (su, sv) = transport(u, v, -final_dir, away);
        let socket = place_socket(
            skeleton,
            nodes[count - 1],
            nodes[count - 2],
            away,
            lengths[count - 2],
            su,
            sv,
            config,
            mesh,
        );
        rings.push(socket.ring);
        sockets.push(socket);
    } else {
        let node = skeleton.nodes[nodes[count - 1] as usize];
        rings.push(push_ring(
            mesh,
            node.centre(u, v),
            u,
            v,
            node.half_extents(),
            node.roll,
        ));
        let tip = node.centre(u, v) + dir * (config.tip_extend * node.radius);
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

/// Holds each ring beside a person's hip socket in front of that socket's
/// plane (#281), and says how far it carried each vertex.
///
/// **A tube that runs back into the joint it leaves crosses that joint's
/// hull, and no subdivision undoes it.** A person's hip socket faces almost
/// straight out of the pelvis, and the thigh turns down at the hip node just
/// past it - 65 degrees, with the socket under 20 mm short of a hip ring
/// 190 mm across, on the default body. The ring is laid on the bisector of
/// that turn, so its inner, lower side swings back toward the midline, past
/// the socket and above the floor of the pelvis's hull between the two
/// sockets: measured on the default body, the ring's inner vertex stood 33 mm
/// behind the socket's plane, 9 mm off the midline and 29 mm above the hull's
/// floor. The thigh's inner wall then ran up and in from the floor, turned,
/// and ran down to the knee, through the floor facet: 21 crossing triangle
/// pairs on the default body's cage and ten after the shipped subdivision,
/// and on seed 9, 22 after it and 38 after a second, every one of them at the
/// crotch.
///
/// So a vertex of the ring after a hip socket that stands behind that
/// socket's plane is carried along the socket's own axis until it stands in
/// front of it by the daylight the socket keeps from its siblings,
/// [`CageConfig::socket_margin`] of its ring - the same plane and the same
/// margin `joint::required_distance` holds the hull to. Along the axis
/// because that is the shortest way out, and the one that keeps the groin:
/// carried down the thigh or along its column instead, the crease from the
/// hip to the crotch flattened into a level ledge.
///
/// **What it moves, measured on 401 rolls and ten composite corners:** the
/// three inner columns of each hip ring (four on 21 of those 411 bodies), by
/// a median 38 mm on the cage, 59 mm at the ninetieth percentile and up to
/// 91 mm on the heaviest corners - about 0.36 to 0.41 of the ring's own
/// clearance radius on every body, so a big body's push is big because its
/// thigh is. Subdivision carries about two thirds of that onto the surface.
/// A vertex in front of the plane is not touched, so a ring with room is
/// where it was.
///
/// **Only the hips**, because that is the fold measured, and the same rule
/// run over every socket also nudged rings that were never crossing anything:
/// a foot's ring 0.18 mm on ten of 401 rolls, lowering the sole 0.12 mm, and
/// a neck ring by up to 4.2 mm, each outside the fold this was measured on
/// and left to its own issue.
///
/// The rings are moved after every hull, which is what makes the socket's
/// plane final, and before the faces are emitted, which simply follow. The
/// push comes back per cage vertex, zero where nothing moved, so the skin can
/// be bound where the cage first laid the groin (see `Avatar::build`): bound
/// where the push leaves it, the inner wall of the groin lay in the thigh's
/// reach and swung with it, and a raised knee drove it through the belly.
pub(crate) fn clear_sockets(
    skeleton: &Skeleton,
    sockets: &[Socket],
    builds: &[LimbBuild],
    config: &CageConfig,
    mesh: &mut PolyMesh,
) -> Vec<Vec3> {
    let mut pushed = vec![Vec3::ZERO; mesh.positions.len()];
    for build in builds {
        let count = build.rings.len();
        if count < 3 {
            continue;
        }
        for (socket_ring, beside) in [(0, 1), (count - 1, count - 2)] {
            let Some(socket) = sockets.iter().find(|socket| {
                socket.ring == build.rings[socket_ring]
                    && skeleton.nodes[socket.joint as usize].zone == Zone::Pelvis
                    && skeleton.nodes[socket.toward as usize].zone.limb().is_some()
            }) else {
                continue;
            };
            let centre = socket.center();
            let daylight = config.socket_margin * socket.clearance_radius();
            for &vertex in &build.rings[beside] {
                let at = mesh.positions[vertex as usize];
                let room = socket.dir.dot(at - centre);
                if room < daylight {
                    let push = socket.dir * (daylight - room);
                    mesh.positions[vertex as usize] = at + push;
                    pushed[vertex as usize] = push;
                }
            }
        }
    }
    pushed
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
    // A bridged bone's socket is sized by the joint it opens toward, where
    // the limb leaves the body, not by the node it sweeps to (#374): a
    // thigh's muscle rides the stifle's ring, and blended into the socket
    // on the pelvis it crowded the spine's socket off its bone.
    let sized_by = skeleton.bridge_aim(joint, neighbor).unwrap_or(neighbor);
    let next = skeleton.nodes[sized_by as usize];

    let max_dist = config.max_socket_fraction * bone_length;
    let dist = (config.socket_distance * hub.radius).min(max_dist);
    let blend = (dist / bone_length).clamp(0.0, 1.0);
    let half = hub.half_extents().lerp(next.half_extents(), blend);
    // The roll travels with the half-extents, for the same reason they blend at
    // all: a socket that stood on a flat edge while the node beside it stood on a
    // vertex would tear the section as it slid.
    let roll = hub.roll + (next.roll - hub.roll) * blend;
    // And so does the offset, for a sharper version of the same reason: the
    // depth and the displacement of a lobed section are one shape, and blending
    // one without the other hands the joint the extra depth with nothing to
    // cancel it. See [`Socket::offset`].
    let offset = hub.offset.lerp(next.offset, blend);

    let center = hub.position + away * dist + u * offset.x + v * offset.y;
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
        offset,
        joint_half: hub.half_extents(),
        neighbor_half: next.half_extents(),
        joint_offset: hub.offset,
        neighbor_offset: next.offset,
        bone_length,
        dist,
        base_dist: dist,
        max_dist,
        ring,
    }
}

/// Appends a [`RING`]-vertex ring, wound counter-clockwise about `u × v`.
/// Which way the socket of a bridged bone faces out of its joint, if the bone
/// from `joint` to `neighbor` is a bridge: toward the last marker of the run,
/// the joint inside the body the limb hangs from - a shoulder, a hip (#374).
///
/// **Aimed there because two legs aimed at their own feet cannot both open on
/// one hull.** Dropped near-parallel from a girdle, their rings lie side by
/// side on the hull's floor and one facet takes both; the separation the hull
/// needs is what spread the P1 legs into an A. Aimed at the shoulder, the
/// socket opens on the front of the chest where the upper arm leaves it; at
/// the hip, on the side of the croup where the thigh does, and the leg runs
/// from there to the elbow or the stifle under the body.
fn aim(skeleton: &Skeleton, joint: u32, neighbor: u32) -> Option<Vec3> {
    let marker = skeleton.bridge_aim(joint, neighbor)?;
    let toward = skeleton.nodes[marker as usize].position - skeleton.nodes[joint as usize].position;
    (toward.length() > MIN_BONE_LENGTH).then(|| toward.normalize())
}

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

/// A ring frame for a bone along `dir` whose first axis is as near across
/// the body, `+X`, as the bone allows: see [`build_limb`]. Falls back to
/// [`initial_frame`] for a bone that runs across the body itself.
fn lateral_frame(dir: Vec3) -> (Vec3, Vec3) {
    let across = Vec3::X - dir * dir.dot(Vec3::X);
    if across.length_squared() < 1e-6 {
        return initial_frame(dir);
    }
    let u = across.normalize();
    (u, dir.cross(u))
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
    use crate::Node;

    /// Whether `socket` is a hip's: on the pelvis, opening onto a leg.
    fn hip(skeleton: &Skeleton, socket: &Socket) -> bool {
        skeleton.nodes[socket.joint as usize].zone == Zone::Pelvis
            && skeleton.nodes[socket.toward as usize].zone.limb().is_some()
    }

    #[test]
    fn only_the_hips_move_and_by_what_the_docs_say() {
        // **The pass is for the crotch, and moves nothing else** (#281). Run
        // over every socket it also nudged rings that were crossing nothing:
        // seed 40's foot rings 0.18 mm (its sole 0.12 mm lower), and seeds
        // 245's and 247's neck rings 4.2 and 0.8 mm. Asked of the pushes the
        // pass hands back: every vertex it moved lies on the ring beside a
        // hip socket, at most four columns of each ring move, and no push is
        // longer than the figure `clear_sockets` states (91 mm on the
        // heaviest corner of 411 bodies; these reach 67 mm). Without the
        // hips-only rule: a moved vertex on seed 40's foot ring.
        let mut records = Vec::new();
        for seed in [40i64, 245, 247, 9] {
            let mut record = crate::AvatarRecord::new("Rolled", crate::Archetype::default());
            record.reroll(seed);
            records.push((format!("seed {seed}"), record));
        }
        let config = CageConfig::default();
        let mut longest = 0.0f32;
        for (name, record) in &records {
            let skeleton = record.skeleton();
            let (_, sockets, builds, pushed) =
                super::super::build_all(&skeleton, &config).expect("a person meshes");
            let mut hip_rings: Vec<[u32; RING]> = Vec::new();
            for build in &builds {
                let count = build.rings.len();
                if count < 3 {
                    continue;
                }
                for (end, beside) in [(0, 1), (count - 1, count - 2)] {
                    if sockets
                        .iter()
                        .any(|s| s.ring == build.rings[end] && hip(&skeleton, s))
                    {
                        hip_rings.push(build.rings[beside]);
                    }
                }
            }
            assert_eq!(hip_rings.len(), 2, "{name}: two hips");
            for (vertex, push) in pushed.iter().enumerate() {
                if *push == Vec3::ZERO {
                    continue;
                }
                longest = longest.max(push.length());
                assert!(
                    hip_rings.iter().any(|ring| ring.contains(&(vertex as u32))),
                    "{name}: cage vertex {vertex} moved {:.2} mm, and it is on no hip ring",
                    push.length() * 1000.0
                );
            }
            for ring in &hip_rings {
                let moved = ring
                    .iter()
                    .filter(|&&vertex| pushed[vertex as usize] != Vec3::ZERO)
                    .count();
                assert!(moved <= 4, "{name}: {moved} columns of a hip ring moved");
            }
        }
        println!("the longest push: {:.1} mm", longest * 1000.0);
        assert!(longest < 0.091, "a push of {:.1} mm", longest * 1000.0);
    }

    #[test]
    fn no_ring_of_a_person_stands_behind_its_socket() {
        // **The crotch's self-crossing, at its source** (#281). A ring beside a
        // socket that stands behind the socket's plane is a tube running back
        // into its joint, through the hull's facets there, and no subdivision
        // takes that out. Without `clear_sockets` the worst ring of these four
        // bodies stood 67.2 mm short of its socket's daylight, a hip ring, the
        // inner wall of its thigh rising through the pelvis's floor. Asked of
        // the plane and its daylight, the condition the pass keeps. Of the
        // hips, which are all the pass holds (see
        // `only_the_hips_move_and_by_what_the_docs_say`); of the whole cage
        // these four bodies read the same, since no other ring of theirs
        // stands behind its socket.
        let mut records = vec![crate::AvatarRecord::default()];
        for seed in [9i64, 16] {
            let mut record = crate::AvatarRecord::new("Rolled", crate::Archetype::default());
            record.reroll(seed);
            records.push(record);
        }
        let mut narrow = crate::AvatarRecord::default();
        if let crate::Archetype::Humanoid(params) = &mut narrow.archetype {
            params.hip_width = -3.0;
        }
        narrow.sanitize();
        records.push(narrow);

        let config = CageConfig::default();
        let mut worst = f32::INFINITY;
        for record in &records {
            let skeleton = record.skeleton();
            let (mesh, sockets, builds) =
                super::super::build_with_limbs(&skeleton, &config).expect("a person meshes");
            for build in &builds {
                let count = build.rings.len();
                for (end, beside) in [(0, 1), (count - 1, count.saturating_sub(2))] {
                    let Some(socket) = sockets.iter().find(|s| s.ring == build.rings[end]) else {
                        continue;
                    };
                    if count < 3 || !hip(&skeleton, socket) {
                        continue;
                    }
                    let daylight = config.socket_margin * socket.clearance_radius();
                    for &vertex in &build.rings[beside] {
                        let room = socket
                            .dir
                            .dot(mesh.positions[vertex as usize] - socket.center());
                        worst = worst.min(room - daylight);
                    }
                }
            }
        }
        println!(
            "the nearest any ring comes to its socket's daylight: {:.2} mm",
            worst * 1000.0
        );
        assert!(
            worst > -1e-5,
            "a ring stands {:.1} mm short of its socket's daylight, behind or too near the \
             socket's plane: that tube runs back into its joint",
            -worst * 1000.0
        );
    }

    #[test]
    fn a_section_offset_moves_the_surface_and_leaves_the_joint() {
        // **The whole reason the offset is on the section and not on the
        // position** (#125). A node's position is a joint: bones meet there,
        // the rig rotates about it, and `face::skull` and `hair::follicle`
        // measure the head in radii about it. Moving a node to put mass somewhere moves
        // the axis everything else is measured from, which is why the neck's
        // lean is bounded at a third of a radius by what it does to the head's
        // own floor rather than by anatomy.
        //
        // So: same joint, same ring frame, and a ring swept a measured distance
        // off it. Asserted in the ring frame rather than in world axes, because
        // `u` and `v` are transported down a chain and a test keyed to `Z`
        // would pass on an upright bone and say nothing about any other.
        let node = Node::new(Vec3::new(0.0, 1.0, 0.0), 0.05);
        let (u, v) = initial_frame(Vec3::Y);
        assert_eq!(node.centre(u, v), node.position, "a plain node is centred");

        let lobed = node.with_offset(Vec2::new(0.0, -0.03));
        assert_eq!(lobed.position, node.position, "the joint moved");
        let moved = lobed.centre(u, v) - lobed.position;
        assert!(
            (moved.dot(v) + 0.03).abs() < 1e-6 && moved.dot(u).abs() < 1e-6,
            "the section went {moved:?} for an offset of 0.03 along -v"
        );

        // And what it is FOR: reach past the joint on one side without giving
        // any of it back on the other. A centred ellipse cannot, at any scale.
        let deep = lobed.with_scale(Vec2::new(1.0, 1.6));
        let half = deep.half_extents().y;
        let behind = half - deep.centre(u, v).dot(v) + deep.position.dot(v);
        let ahead = half + deep.centre(u, v).dot(v) - deep.position.dot(v);
        assert!(
            behind > ahead * 1.9,
            "reaches {behind:.4} behind the joint and {ahead:.4} in front, which \
             is not a lobe"
        );
    }

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
