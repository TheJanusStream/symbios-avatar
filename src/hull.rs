//! Convex hull of a small point set, returned as convex polygons.
//!
//! Joints are meshed by hulling the socket rings of their incident limbs, so
//! this hull has an unusual requirement: the socket squares are *exactly*
//! coplanar by construction, and each one must come back as a single quad face
//! rather than a pair of arbitrarily-split triangles. That rules out a plain
//! triangle-emitting quickhull.
//!
//! Instead the hull is found by enumerating supporting planes. Every triple of
//! points spans a candidate plane; the plane is a facet exactly when all other
//! points lie on one side of it. Collecting *every* point on such a plane and
//! ordering it around the face normal yields one polygon per facet, coplanar
//! groups included. That is `O(n⁴)` — irrelevant at the sizes involved (a joint
//! of degree `d` hulls `4d + 2` points) and far easier to make robust than
//! incremental hulling with coplanar merging bolted on.

use glam::Vec3;
use thiserror::Error;

/// Upper bound on hull input size, keeping the `O(n⁴)` search negligible.
///
/// A joint contributes four points per incident limb plus at most two apex
/// points, so this admits joints of degree 15.
pub const MAX_HULL_POINTS: usize = 64;

/// Errors raised while hulling a point set.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum HullError {
    /// Fewer than four points cannot bound a volume.
    #[error("a convex hull needs at least 4 points, got {0}")]
    TooFewPoints(usize),
    /// More points than [`MAX_HULL_POINTS`].
    #[error("convex hull input is capped at {MAX_HULL_POINTS} points, got {0}")]
    TooManyPoints(usize),
    /// Every point lies on one line.
    #[error("all {0} points are collinear or coincident")]
    Collinear(usize),
    /// Every point lies in one plane, so the hull has no volume.
    #[error("all {0} points lie in a single plane, so the hull has no volume")]
    Coplanar(usize),
}

/// A supporting plane oriented so every input point satisfies `n·p ≤ d`.
#[derive(Clone, Copy, Debug)]
struct Plane {
    normal: Vec3,
    offset: f32,
}

/// Computes the convex hull of `points`.
///
/// Returns one entry per facet: a ring of indices into `points`, wound
/// counter-clockwise seen from outside the hull. Coplanar points share a facet,
/// so a square face comes back as a single four-index ring.
///
/// # Errors
///
/// Returns [`HullError`] when the input is too small, too large, or degenerate
/// (collinear or coplanar). Callers that can repair degeneracy — the joint
/// builder adds apex points to a flat socket fan — should match on
/// [`HullError::Coplanar`].
pub fn convex_hull(points: &[Vec3]) -> Result<Vec<Vec<u32>>, HullError> {
    if points.len() < 4 {
        return Err(HullError::TooFewPoints(points.len()));
    }
    if points.len() > MAX_HULL_POINTS {
        return Err(HullError::TooManyPoints(points.len()));
    }

    let scale = extent(points).max(1e-6);
    let eps = 1e-4 * scale;

    reject_degenerate(points, eps)?;

    let planes = supporting_planes(points, eps);
    let mut faces = Vec::with_capacity(planes.len());
    for plane in planes {
        let on_plane: Vec<u32> = points
            .iter()
            .enumerate()
            .filter(|(_, p)| (plane.normal.dot(**p) - plane.offset).abs() <= eps)
            .map(|(i, _)| i as u32)
            .collect();
        if on_plane.len() >= 3 {
            faces.push(order_ccw(points, &on_plane, plane.normal));
        }
    }
    Ok(faces)
}

/// Largest side of the axis-aligned bounding box, used to scale epsilons.
fn extent(points: &[Vec3]) -> f32 {
    let mut lo = points[0];
    let mut hi = points[0];
    for &p in points {
        lo = lo.min(p);
        hi = hi.max(p);
    }
    (hi - lo).max_element()
}

/// Fails if the points are collinear or all lie in one plane.
fn reject_degenerate(points: &[Vec3], eps: f32) -> Result<(), HullError> {
    let base = points[0];

    let (spread, along) = points.iter().map(|&p| ((p - base).length(), p)).fold(
        (0.0f32, base),
        |(best, at), (d, p)| {
            if d > best { (d, p) } else { (best, at) }
        },
    );
    if spread <= eps {
        return Err(HullError::Collinear(points.len()));
    }

    let edge = along - base;
    let (area, normal) = points
        .iter()
        .map(|&p| {
            let n = edge.cross(p - base);
            (n.length(), n)
        })
        .fold((0.0f32, Vec3::ZERO), |(best, at), (a, n)| {
            if a > best { (a, n) } else { (best, at) }
        });
    if area <= eps * spread {
        return Err(HullError::Collinear(points.len()));
    }

    let normal = normal.normalize();
    let thickness = points
        .iter()
        .map(|&p| normal.dot(p - base).abs())
        .fold(0.0f32, f32::max);
    if thickness <= eps {
        return Err(HullError::Coplanar(points.len()));
    }

    Ok(())
}

/// Every distinct plane that supports the point set, oriented outward.
fn supporting_planes(points: &[Vec3], eps: f32) -> Vec<Plane> {
    let mut planes: Vec<Plane> = Vec::new();
    let count = points.len();

    for i in 0..count {
        for j in (i + 1)..count {
            for k in (j + 1)..count {
                let normal = (points[j] - points[i]).cross(points[k] - points[i]);
                let length = normal.length();
                if length <= eps * eps {
                    continue;
                }
                let normal = normal / length;
                let offset = normal.dot(points[i]);

                let mut above = false;
                let mut below = false;
                for &p in points {
                    let signed = normal.dot(p) - offset;
                    above |= signed > eps;
                    below |= signed < -eps;
                }
                let plane = match (above, below) {
                    // Points sit only on the negative side: `normal` faces out.
                    (false, _) => Plane { normal, offset },
                    // Only on the positive side: the opposite face is the hull's.
                    (_, false) => Plane {
                        normal: -normal,
                        offset: -offset,
                    },
                    // Points straddle the plane, so it cannot support the hull.
                    _ => continue,
                };
                if !planes.iter().any(|seen| same_plane(seen, &plane, eps)) {
                    planes.push(plane);
                }
            }
        }
    }

    planes
}

/// Whether two oriented planes describe the same facet.
fn same_plane(a: &Plane, b: &Plane, eps: f32) -> bool {
    a.normal.dot(b.normal) > 0.9999 && (a.offset - b.offset).abs() <= eps
}

/// Orders a facet's points counter-clockwise about `normal`.
fn order_ccw(points: &[Vec3], face: &[u32], normal: Vec3) -> Vec<u32> {
    let centroid: Vec3 = face.iter().map(|&i| points[i as usize]).sum::<Vec3>() / face.len() as f32;

    // Any in-plane direction works as the angular origin; pick the one that is
    // furthest from the centroid so the basis is numerically well conditioned.
    let axis = face
        .iter()
        .map(|&i| points[i as usize] - centroid)
        .fold(Vec3::ZERO, |best, d| {
            if d.length_squared() > best.length_squared() {
                d
            } else {
                best
            }
        });
    let u = (axis - normal * normal.dot(axis)).normalize_or_zero();
    let u = if u == Vec3::ZERO {
        normal.any_orthonormal_vector()
    } else {
        u
    };
    let v = normal.cross(u);

    let mut ordered: Vec<(f32, u32)> = face
        .iter()
        .map(|&i| {
            let d = points[i as usize] - centroid;
            (v.dot(d).atan2(u.dot(d)), i)
        })
        .collect();
    ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
    ordered.into_iter().map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::PolyMesh;

    fn cube_points() -> Vec<Vec3> {
        let mut points = Vec::new();
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    points.push(Vec3::new(x, y, z));
                }
            }
        }
        points
    }

    /// Wraps hull output in a mesh so the manifold audit can vet winding.
    fn hull_mesh(points: &[Vec3]) -> PolyMesh {
        let faces = convex_hull(points).expect("hull succeeds");
        PolyMesh {
            positions: points.to_vec(),
            faces,
        }
    }

    #[test]
    fn a_cube_hulls_to_six_quads() {
        let points = cube_points();
        let faces = convex_hull(&points).expect("hull succeeds");
        assert_eq!(faces.len(), 6);
        assert!(faces.iter().all(|f| f.len() == 4), "coplanar corners merge");
    }

    #[test]
    fn hull_faces_are_wound_outward_and_closed() {
        let mesh = hull_mesh(&cube_points());
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());

        // Outward winding: each face normal points away from the centre.
        for index in 0..mesh.face_count() {
            let face = &mesh.faces[index];
            let a = mesh.positions[face[0] as usize];
            let b = mesh.positions[face[1] as usize];
            let c = mesh.positions[face[2] as usize];
            let normal = (b - a).cross(c - a);
            assert!(
                normal.dot(mesh.face_centroid(index)) > 0.0,
                "face {index} winds inward"
            );
        }
    }

    #[test]
    fn a_tetrahedron_hulls_to_four_triangles() {
        let points = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let faces = convex_hull(&points).expect("hull succeeds");
        assert_eq!(faces.len(), 4);
        assert!(faces.iter().all(|f| f.len() == 3));
    }

    #[test]
    fn interior_points_are_excluded_from_every_face() {
        let mut points = cube_points();
        points.push(Vec3::new(0.0, 0.0, 0.0));
        let interior = (points.len() - 1) as u32;
        let faces = convex_hull(&points).expect("hull succeeds");
        assert_eq!(faces.len(), 6);
        assert!(faces.iter().all(|f| !f.contains(&interior)));
    }

    #[test]
    fn degenerate_inputs_are_reported_precisely() {
        let collinear: Vec<Vec3> = (0..5).map(|i| Vec3::X * i as f32).collect();
        assert_eq!(
            convex_hull(&collinear),
            Err(HullError::Collinear(collinear.len()))
        );

        let flat = vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y];
        assert_eq!(convex_hull(&flat), Err(HullError::Coplanar(flat.len())));

        assert_eq!(
            convex_hull(&[Vec3::ZERO, Vec3::X, Vec3::Y]),
            Err(HullError::TooFewPoints(3))
        );

        let many = vec![Vec3::ZERO; MAX_HULL_POINTS + 1];
        assert_eq!(
            convex_hull(&many),
            Err(HullError::TooManyPoints(MAX_HULL_POINTS + 1))
        );
    }

    #[test]
    fn a_square_socket_survives_as_one_quad() {
        // Two parallel squares plus an apex: the squares must not be split.
        let mut points = vec![
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        points.push(Vec3::new(0.0, 0.0, -1.5));
        let faces = convex_hull(&points).expect("hull succeeds");
        let quads: Vec<_> = faces.iter().filter(|f| f.len() == 4).collect();
        assert_eq!(quads.len(), 1, "the coplanar square stays one face");
        assert_eq!(quads[0].len(), 4);
    }
}
