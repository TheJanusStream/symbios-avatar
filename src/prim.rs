//! Small closed shapes the body's features are built from.
//!
//! The cage builder grows a body from a skeleton, which is the right machinery
//! for a torso and quite the wrong one for an eyeball. Eyes, lids, and hair
//! strands are small, rigid, and known in advance, so they are built directly
//! from primitives instead.
//!
//! Everything here comes out **closed and outward-wound**, the same contract the
//! body meshes hold themselves to. A lid is a shell rather than a single
//! surface for that reason: it is never seen from inside, but a mesh with a
//! boundary edge is one that normals, subdivision, and glTF export all have to
//! make a special case for.

use glam::{Mat4, Vec2, Vec3};

use crate::mesh::PolyMesh;

/// A sphere of `radius`, centred on the origin.
///
/// `rings` counts the divisions between the poles and `segments` those around
/// the equator; both are clamped to what still makes a solid.
#[must_use]
pub fn sphere(radius: f32, rings: usize, segments: usize) -> PolyMesh {
    let rings = rings.max(2);
    let segments = segments.max(3);
    let mut mesh = PolyMesh::new();

    let north = mesh.push_vertex(Vec3::Y * radius);
    // Interior rings only; the poles are single vertices.
    for ring in 1..rings {
        let polar = std::f32::consts::PI * ring as f32 / rings as f32;
        for segment in 0..segments {
            mesh.push_vertex(on_sphere(radius, polar, turn(segment, segments)));
        }
    }
    let south = mesh.push_vertex(-Vec3::Y * radius);

    let at = |ring: usize, segment: usize| (1 + (ring - 1) * segments + segment % segments) as u32;

    for segment in 0..segments {
        mesh.push_face([north, at(1, segment + 1), at(1, segment)]);
    }
    for ring in 1..rings - 1 {
        for segment in 0..segments {
            mesh.push_face([
                at(ring, segment),
                at(ring, segment + 1),
                at(ring + 1, segment + 1),
                at(ring + 1, segment),
            ]);
        }
    }
    for segment in 0..segments {
        mesh.push_face([south, at(rings - 1, segment), at(rings - 1, segment + 1)]);
    }

    mesh
}

/// A closed shell cut from a sphere, capping the `+Y` pole.
///
/// `half_angle` is how far down from the pole the cap reaches, in radians;
/// `thickness` is how far its inner surface sits below the outer one. The result
/// is a solid: outer surface, inner surface, and a rim joining them.
#[must_use]
pub fn cap_shell(
    radius: f32,
    thickness: f32,
    half_angle: f32,
    rings: usize,
    segments: usize,
) -> PolyMesh {
    let rings = rings.max(1);
    let segments = segments.max(3);
    let half_angle = half_angle.clamp(0.05, std::f32::consts::PI - 0.05);
    let inner = (radius - thickness.abs()).max(radius * 0.2);

    let mut mesh = PolyMesh::new();
    let outer_pole = mesh.push_vertex(Vec3::Y * radius);
    for ring in 1..=rings {
        let polar = half_angle * ring as f32 / rings as f32;
        for segment in 0..segments {
            mesh.push_vertex(on_sphere(radius, polar, turn(segment, segments)));
        }
    }
    let inner_start = mesh.vertex_count();
    let inner_pole = mesh.push_vertex(Vec3::Y * inner);
    for ring in 1..=rings {
        let polar = half_angle * ring as f32 / rings as f32;
        for segment in 0..segments {
            mesh.push_vertex(on_sphere(inner, polar, turn(segment, segments)));
        }
    }

    let outer =
        |ring: usize, segment: usize| (1 + (ring - 1) * segments + segment % segments) as u32;
    let inner_at = |ring: usize, segment: usize| {
        (inner_start + 1 + (ring - 1) * segments + segment % segments) as u32
    };

    for segment in 0..segments {
        mesh.push_face([outer_pole, outer(1, segment + 1), outer(1, segment)]);
        // Reversed: the inner surface faces the other way.
        mesh.push_face([inner_pole, inner_at(1, segment), inner_at(1, segment + 1)]);
    }
    for ring in 1..rings {
        for segment in 0..segments {
            mesh.push_face([
                outer(ring, segment),
                outer(ring, segment + 1),
                outer(ring + 1, segment + 1),
                outer(ring + 1, segment),
            ]);
            mesh.push_face([
                inner_at(ring, segment),
                inner_at(ring + 1, segment),
                inner_at(ring + 1, segment + 1),
                inner_at(ring, segment + 1),
            ]);
        }
    }
    // The rim closes the shell along its open edge.
    for segment in 0..segments {
        mesh.push_face([
            outer(rings, segment),
            outer(rings, segment + 1),
            inner_at(rings, segment + 1),
            inner_at(rings, segment),
        ]);
    }

    mesh
}

/// A closed tube swept along `path`, tapering from `base` to `tip` half-extents.
///
/// The cross-section is a ring of `sides` vertices, parallel-transported down the
/// path so the tube does not twist. Used for hair strands, where a ribbon rather
/// than a circle is wanted — hence half-extents rather than a radius.
#[must_use]
pub fn tube(path: &[Vec3], base: Vec2, tip: Vec2, sides: usize) -> PolyMesh {
    let across = path.windows(2).next().map_or(Vec3::X, |step| {
        frame((step[1] - step[0]).normalize_or(Vec3::Y)).0
    });
    ribbon(path, base, tip, sides, across)
}

/// A tube swept with its cross-section's wide axis held along `across`.
///
/// Deriving that axis from the path — which is what [`tube`] does — is fine
/// until the path starts out near-vertical, at which point the derivation snaps
/// to a world axis and the ribbon turns edge-on. Hair falling down the side of a
/// head does exactly that, and the strands that should have overlapped into a
/// sheet showed as separate strings instead.
#[must_use]
pub fn ribbon(path: &[Vec3], base: Vec2, tip: Vec2, sides: usize, across: Vec3) -> PolyMesh {
    let sections: Vec<Vec2> = (0..path.len())
        .map(|at| base.lerp(tip, at as f32 / (path.len().max(2) - 1) as f32))
        .collect();
    sweep(path, &sections, sides, across)
}

/// A tube swept with a half-extent given at every point on the path.
///
/// [`ribbon`] can only run one cross-section into another, which is enough for a
/// lock of hair and not enough for anything whose width comes and goes: a foot
/// is narrow at the heel, widest at the ball, and narrows again at the toe, and
/// no interpolation between two ends will say that.
#[must_use]
pub fn sweep(path: &[Vec3], sections: &[Vec2], sides: usize, across: Vec3) -> PolyMesh {
    let sides = sides.max(3);
    let mut mesh = PolyMesh::new();
    if path.len() < 2 || sections.len() < path.len() {
        return mesh;
    }

    let mut direction = (path[1] - path[0]).normalize_or(Vec3::Y);
    let (mut u, mut v) = {
        let square = across - direction * across.dot(direction);
        let u = square.normalize_or(frame(direction).0);
        (u, direction.cross(u))
    };
    let mut rings: Vec<Vec<u32>> = Vec::with_capacity(path.len());

    for (index, &point) in path.iter().enumerate() {
        let next = if index + 1 < path.len() {
            (path[index + 1] - point).normalize_or(direction)
        } else {
            direction
        };
        let bend = if index == 0 {
            direction
        } else {
            (direction + next).normalize_or(next)
        };
        (u, v) = transport(u, v, direction, bend);
        direction = bend;

        let half = sections[index];
        let ring: Vec<u32> = (0..sides)
            .map(|side| {
                let angle = turn(side, sides);
                mesh.push_vertex(point + u * (angle.cos() * half.x) + v * (angle.sin() * half.y))
            })
            .collect();
        rings.push(ring);
    }

    for pair in rings.windows(2) {
        for side in 0..sides {
            let next = (side + 1) % sides;
            mesh.push_face([pair[0][side], pair[0][next], pair[1][next], pair[1][side]]);
        }
    }

    // Cap both ends so the strand is a solid.
    let first: Vec<u32> = rings[0].iter().rev().copied().collect();
    mesh.push_face(first);
    mesh.push_face(rings[rings.len() - 1].clone());

    mesh
}

/// A point on a sphere at the given polar and azimuthal angles.
fn on_sphere(radius: f32, polar: f32, azimuth: f32) -> Vec3 {
    Vec3::new(
        radius * polar.sin() * azimuth.cos(),
        radius * polar.cos(),
        radius * polar.sin() * azimuth.sin(),
    )
}

/// The azimuth of one segment of a full turn.
fn turn(segment: usize, segments: usize) -> f32 {
    std::f32::consts::TAU * segment as f32 / segments as f32
}

/// An orthonormal pair perpendicular to `direction`.
fn frame(direction: Vec3) -> (Vec3, Vec3) {
    let reference = if direction.dot(Vec3::Y).abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let u = reference.cross(direction).normalize_or(Vec3::X);
    (u, direction.cross(u))
}

/// Rotates a frame by the minimal rotation carrying `from` onto `to`.
fn transport(u: Vec3, v: Vec3, from: Vec3, to: Vec3) -> (Vec3, Vec3) {
    let turn = glam::Quat::from_rotation_arc(from, to);
    (turn * u, turn * v)
}

impl PolyMesh {
    /// A copy of this mesh with every vertex transformed.
    ///
    /// A mirroring transform reverses each face's winding to match, so a
    /// left-handed copy of a right-handed part still faces outward.
    #[must_use]
    pub fn transformed(&self, transform: Mat4) -> PolyMesh {
        let mirrored = transform.determinant() < 0.0;
        PolyMesh {
            positions: self
                .positions
                .iter()
                .map(|&point| transform.transform_point3(point))
                .collect(),
            faces: self
                .faces
                .iter()
                .map(|face| {
                    if mirrored {
                        face.iter().rev().copied().collect()
                    } else {
                        face.clone()
                    }
                })
                .collect(),
        }
    }

    /// Appends another mesh, re-indexing its faces.
    pub fn append(&mut self, other: &PolyMesh) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.faces.extend(
            other
                .faces
                .iter()
                .map(|face| face.iter().map(|index| index + base).collect::<Vec<u32>>()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sphere_is_closed_and_the_right_size() {
        let mesh = sphere(0.5, 8, 12);
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());
        for point in &mesh.positions {
            assert!(
                (point.length() - 0.5).abs() < 1e-5,
                "{point:?} is off-radius"
            );
        }
        let (lo, hi) = mesh.bounds();
        assert!((hi - lo).abs_diff_eq(Vec3::splat(1.0), 0.05));
    }

    #[test]
    fn a_sphere_faces_outward() {
        let mesh = sphere(1.0, 6, 10);
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
    fn a_cap_shell_is_a_solid() {
        let mesh = cap_shell(0.5, 0.03, 1.2, 4, 12);
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());
        // It occupies the top of the sphere only.
        let (lo, hi) = mesh.bounds();
        assert!(hi.y > 0.4, "the cap should reach the pole");
        assert!(lo.y > -0.2, "and should not wrap round the bottom");
    }

    #[test]
    fn a_tube_is_closed_and_follows_its_path() {
        let path = [
            Vec3::ZERO,
            Vec3::new(0.0, -0.2, 0.05),
            Vec3::new(0.0, -0.4, 0.15),
        ];
        let mesh = tube(&path, Vec2::splat(0.03), Vec2::splat(0.01), 6);
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());

        let (lo, hi) = mesh.bounds();
        assert!(lo.y < -0.4 + 0.02, "the tube should reach the path's end");
        assert!(hi.y > -0.05, "and start at its beginning");
    }

    #[test]
    fn a_tube_tapers() {
        let path = [Vec3::ZERO, Vec3::NEG_Y];
        let mesh = tube(&path, Vec2::splat(0.1), Vec2::splat(0.01), 8);
        let spread = |near: f32| {
            mesh.positions
                .iter()
                .filter(|point| (point.y - near).abs() < 0.01)
                .map(|point| Vec2::new(point.x, point.z).length())
                .fold(0.0f32, f32::max)
        };
        assert!(
            spread(0.0) > spread(-1.0) * 5.0,
            "the tip should be thinner"
        );
    }

    #[test]
    fn a_degenerate_path_makes_nothing_rather_than_panicking() {
        assert_eq!(tube(&[], Vec2::ONE, Vec2::ONE, 6).face_count(), 0);
        assert_eq!(tube(&[Vec3::ZERO], Vec2::ONE, Vec2::ONE, 6).face_count(), 0);
    }

    #[test]
    fn mirroring_keeps_a_mesh_facing_outward() {
        let mesh = sphere(0.4, 6, 10);
        let mirrored = mesh.transformed(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
        assert!(
            mirrored.is_closed_manifold(),
            "{:?}",
            mirrored.manifold_report()
        );
        for index in 0..mirrored.face_count() {
            let face = &mirrored.faces[index];
            let a = mirrored.positions[face[0] as usize];
            let b = mirrored.positions[face[1] as usize];
            let c = mirrored.positions[face[2] as usize];
            assert!(
                (b - a).cross(c - a).dot(mirrored.face_centroid(index)) > 0.0,
                "mirrored face {index} turned inside out"
            );
        }
    }

    #[test]
    fn appending_joins_two_meshes_without_losing_either() {
        let mut mesh = sphere(0.3, 5, 8);
        let faces = mesh.face_count();
        let other = sphere(0.3, 5, 8).transformed(Mat4::from_translation(Vec3::X * 2.0));
        mesh.append(&other);

        assert_eq!(mesh.face_count(), faces * 2);
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());
        let (lo, hi) = mesh.bounds();
        assert!(lo.x < -0.2 && hi.x > 2.2, "both spheres should be present");
    }
}
