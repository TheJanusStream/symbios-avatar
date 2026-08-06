//! Meshing a joint by hulling its socket rings.
//!
//! This is the hard case in skeleton-to-mesh conversion: three or more limbs
//! meet and the surface has to wrap them without seams, self-intersection, or
//! arbitrary topology choices. The construction here is B-Mesh's — hull the
//! socket rings, then delete the facets that *are* sockets so each limb gets an
//! opening to plug into.
//!
//! Two degenerate cases have to be handled rather than wished away:
//!
//! * **Coplanar socket fans.** A T-junction, or a pelvis whose spine and legs
//!   all lie in the sagittal plane, produces a flat point set with no hull. The
//!   joint's own ball supplies the missing thickness: two apex points at
//!   `centre ± radius · n` give the hull volume and read as the joint's depth.
//! * **Buried sockets.** A thin limb leaving a fat joint can sit inside the hull
//!   of its siblings, so it never surfaces as a facet. Such sockets are pushed
//!   outward along their own axis and the hull retried, bounded by
//!   [`CageConfig::socket_push_attempts`] and by how far the socket may travel
//!   before it overshoots the next node.

use glam::Vec3;
use std::collections::BTreeSet;

use super::limb::RING;
use super::{CageConfig, CageError, Socket};
use crate::hull::{HullError, convex_hull};
use crate::mesh::PolyMesh;
use crate::skeleton::Skeleton;

/// Hulls one joint's sockets and appends the connective faces to `mesh`.
pub(crate) fn build_joint(
    skeleton: &Skeleton,
    joint: u32,
    socket_indices: &[usize],
    sockets: &mut [Socket],
    config: &CageConfig,
    mesh: &mut PolyMesh,
) -> Result<(), CageError> {
    check_clearance(joint, socket_indices, sockets, config)?;
    space_sockets(socket_indices, sockets, config, mesh);

    let hub = skeleton.nodes[joint as usize];
    let mut buried = socket_indices[0];

    for _ in 0..=config.socket_push_attempts {
        let mut points = gather_points(socket_indices, sockets, mesh);
        let mut apex_local = Vec::new();

        let faces = match convex_hull(&points) {
            Ok(faces) => faces,
            Err(HullError::Coplanar(_)) => {
                let normal = plane_normal(&points).ok_or(CageError::JointHull {
                    joint,
                    source: HullError::Collinear(points.len()),
                })?;
                apex_local.push(points.len());
                points.push(hub.position + normal * hub.radius);
                apex_local.push(points.len());
                points.push(hub.position - normal * hub.radius);
                convex_hull(&points).map_err(|source| CageError::JointHull { joint, source })?
            }
            Err(source) => return Err(CageError::JointHull { joint, source }),
        };

        // A socket is resolved when some facet is exactly its four points.
        let openings: Vec<Option<usize>> = (0..socket_indices.len())
            .map(|slot| {
                let want: BTreeSet<u32> = (0..RING).map(|k| (slot * RING + k) as u32).collect();
                faces
                    .iter()
                    .position(|face| face.iter().copied().collect::<BTreeSet<u32>>() == want)
            })
            .collect();

        if openings.iter().all(Option::is_some) {
            commit(
                socket_indices,
                sockets,
                &points,
                &faces,
                &openings,
                &apex_local,
                mesh,
            );
            return Ok(());
        }

        let mut moved = false;
        for (slot, &index) in socket_indices.iter().enumerate() {
            if openings[slot].is_some() {
                continue;
            }
            buried = index;
            let socket = &mut sockets[index];
            let before = socket.dist;
            socket.set_dist(socket.dist * config.socket_push_step);
            if socket.dist > before + 1e-6 {
                socket.write(&mut mesh.positions);
                moved = true;
            }
        }
        if !moved {
            break;
        }
    }

    let needed = required_distance(buried, socket_indices, sockets, mesh, config);
    Err(CageError::SocketNotOnHull {
        joint,
        toward: sockets[buried].toward,
        needed,
        available: sockets[buried].max_dist,
    })
}

/// Slides each socket out until its own plane supports the whole joint.
///
/// This is the exact membership condition, and getting it right is the crux of
/// the whole joint construction. A socket appears as a hull facet if and only if
/// every other point of the joint lies behind the socket's plane — the plane
/// through the ring, normal to its limb. So the distance a socket needs is
/// simply how far its siblings' corners reach along its own axis:
///
/// ```text
/// dist ≥ max over sibling points p of  dir · (p − joint centre)
/// ```
///
/// Angular reasoning is tempting here and *wrong*: a socket can subtend a
/// comfortable 34° inside an 86° gap and still be buried, because a fat sibling
/// ring's corner pokes past its plane sideways. Only the plane test catches
/// that.
///
/// Radius and distance are mutually dependent along a tapering limb — sliding a
/// socket out makes it thinner, which lets it come back in — so this is iterated
/// Gauss-Seidel style, writing each result before the next socket reads it.
/// Apex points need no consideration: they sit on the joint's own plane normal,
/// perpendicular to every socket direction, so they project to zero.
fn space_sockets(
    socket_indices: &[usize],
    sockets: &mut [Socket],
    config: &CageConfig,
    mesh: &mut PolyMesh,
) {
    const ITERATIONS: usize = 4;

    if socket_indices.len() < 2 {
        return;
    }

    for _ in 0..ITERATIONS {
        for &index in socket_indices {
            let needed = required_distance(index, socket_indices, sockets, mesh, config);
            sockets[index].set_dist(needed);
            sockets[index].write(&mut mesh.positions);
        }
    }
}

/// How far socket `index` must sit for its plane to clear every sibling point.
fn required_distance(
    index: usize,
    socket_indices: &[usize],
    sockets: &[Socket],
    mesh: &PolyMesh,
    config: &CageConfig,
) -> f32 {
    let socket = &sockets[index];
    let reach = socket_indices
        .iter()
        .filter(|&&other| other != index)
        .flat_map(|&other| sockets[other].ring)
        .map(|vertex| {
            socket
                .dir
                .dot(mesh.positions[vertex as usize] - socket.base)
        })
        .fold(f32::NEG_INFINITY, f32::max);

    if reach.is_finite() {
        // The margin keeps the plane strictly supporting: a sibling point landing
        // exactly on it would be swept into the socket's facet and break the
        // four-point match the opening is identified by.
        reach + config.socket_margin * socket.clearance_radius()
    } else {
        socket.dist
    }
}

/// Rejects joints whose sockets are close enough to intersect.
///
/// The hull would fail on such a joint anyway, with a far less useful message.
/// The remedy is always in the skeleton — widen the joint, spread the limbs, or
/// give each limb its own attachment node.
fn check_clearance(
    joint: u32,
    socket_indices: &[usize],
    sockets: &[Socket],
    config: &CageConfig,
) -> Result<(), CageError> {
    for (offset, &first) in socket_indices.iter().enumerate() {
        for &second in &socket_indices[offset + 1..] {
            let (a, b) = (&sockets[first], &sockets[second]);
            let separation = a.center().distance(b.center());
            let needed = config.socket_clearance * (a.clearance_radius() + b.clearance_radius());
            if separation < needed {
                return Err(CageError::SocketsOverlap {
                    joint,
                    first: a.toward,
                    second: b.toward,
                    separation,
                    needed,
                });
            }
        }
    }
    Ok(())
}

/// Socket ring positions, four per socket in socket order.
fn gather_points(socket_indices: &[usize], sockets: &[Socket], mesh: &PolyMesh) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(socket_indices.len() * RING + 2);
    for &index in socket_indices {
        for &vertex in &sockets[index].ring {
            points.push(mesh.positions[vertex as usize]);
        }
    }
    points
}

/// Writes the hull's non-socket facets into `mesh`, welded to the ring vertices.
fn commit(
    socket_indices: &[usize],
    sockets: &[Socket],
    points: &[Vec3],
    faces: &[Vec<u32>],
    openings: &[Option<usize>],
    apex_local: &[usize],
    mesh: &mut PolyMesh,
) {
    // Socket points already exist in the mesh — reusing their indices is what
    // welds the joint to its limbs. Apex points are new.
    let mut globals: Vec<u32> = Vec::with_capacity(points.len());
    for &index in socket_indices {
        globals.extend_from_slice(&sockets[index].ring);
    }
    for &local in apex_local {
        debug_assert_eq!(globals.len(), local, "apexes follow the socket points");
        globals.push(mesh.push_vertex(points[local]));
    }

    let skip: BTreeSet<usize> = openings.iter().filter_map(|slot| *slot).collect();
    for (index, face) in faces.iter().enumerate() {
        if skip.contains(&index) {
            continue;
        }
        let mapped: Vec<u32> = face.iter().map(|&local| globals[local as usize]).collect();
        mesh.push_face(mapped);
    }
}

/// Unit normal of a coplanar point set, or `None` if the points are collinear.
fn plane_normal(points: &[Vec3]) -> Option<Vec3> {
    let base = *points.first()?;
    let edge = points
        .iter()
        .map(|&p| p - base)
        .fold(Vec3::ZERO, |best, d| {
            if d.length_squared() > best.length_squared() {
                d
            } else {
                best
            }
        });
    let normal = points
        .iter()
        .map(|&p| edge.cross(p - base))
        .fold(Vec3::ZERO, |best, n| {
            if n.length_squared() > best.length_squared() {
                n
            } else {
                best
            }
        });
    (normal.length_squared() > 0.0).then(|| normal.normalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn socket(dir: Vec3, half: f32, first: u32, toward: u32) -> Socket {
        let (u, v) = crate::cage::limb::initial_frame(dir);
        Socket {
            joint: 0,
            base: Vec3::ZERO,
            toward,
            dir,
            u,
            v,
            roll: 0.0,
            offset: Vec2::ZERO,
            half: Vec2::splat(half),
            joint_half: Vec2::splat(half),
            neighbor_half: Vec2::splat(half),
            joint_offset: Vec2::ZERO,
            neighbor_offset: Vec2::ZERO,
            bone_length: 4.0,
            dist: 1.0,
            base_dist: 1.0,
            max_dist: 2.0,
            ring: std::array::from_fn(|k| first + k as u32),
        }
    }

    #[test]
    fn overlapping_sockets_are_reported_with_both_limbs() {
        let config = CageConfig::default();
        let sockets = vec![
            socket(Vec3::Y, 0.5, 0, 7),
            socket(Vec3::new(0.05, 1.0, 0.0).normalize(), 0.5, RING as u32, 9),
        ];
        let error = check_clearance(0, &[0, 1], &sockets, &config).expect_err("sockets collide");
        assert!(matches!(
            error,
            CageError::SocketsOverlap {
                joint: 0,
                first: 7,
                second: 9,
                ..
            }
        ));
    }

    #[test]
    fn well_spread_sockets_pass_clearance() {
        let config = CageConfig::default();
        let sockets = vec![
            socket(Vec3::Y, 0.2, 0, 1),
            socket(Vec3::X, 0.2, RING as u32, 2),
            socket(-Vec3::Z, 0.2, 2 * RING as u32, 3),
        ];
        assert!(check_clearance(0, &[0, 1, 2], &sockets, &config).is_ok());
    }

    #[test]
    fn plane_normal_recovers_a_flat_fan() {
        let points = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
        ];
        let normal = plane_normal(&points).expect("a plane exists");
        assert!(normal.dot(Vec3::Z).abs() > 0.999, "normal is ±Z");
    }

    #[test]
    fn plane_normal_rejects_collinear_points() {
        let points: Vec<Vec3> = (0..4).map(|i| Vec3::X * i as f32).collect();
        assert_eq!(plane_normal(&points), None);
    }
}
