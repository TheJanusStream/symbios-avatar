//! Small closed shapes the body's features are built from.
//!
//! The cage builder grows a body from a skeleton, which is the right machinery
//! for a torso and quite the wrong one for an eyeball. Eyes, lids, and the
//! swept parts of hands and feet are small, shaped in advance, and known
//! exactly, so they are built directly from primitives instead.
//!
//! Everything here comes out **closed and outward-wound**, the same contract the
//! body meshes hold themselves to. A lid is a shell rather than a single
//! surface for that reason: it is never seen from inside, but a mesh with a
//! boundary edge is one that normals, subdivision, and glTF export all have to
//! make a special case for.
//!
//! Everything here also comes out **mapped**: each primitive knows its own
//! parameterisation, so it fills in texture coordinates as it builds. A sphere
//! is mapped by latitude and longitude, a swept tube by angle around the ring
//! and distance along the path. Both wrap, and the vertices that straddle the
//! wrap are *not* duplicated here — that would cost the closed-manifold
//! property every caller relies on. [`crate::PolyMesh::split_uv_seams`] makes
//! the cut on a render-ready copy instead.
//!
//! The one place the mapping is knowingly poor is a sweep's end caps, which
//! reuse their ring's vertices and so reuse its coordinates. A cap is the tip
//! of a finger or a toe — a few square millimetres — and giving it an honest
//! chart would mean splitting it off the ring it closes.

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;

/// A sphere of `radius`, centred on the origin.
///
/// `rings` counts the divisions between the poles and `segments` those around
/// the equator; both are clamped to what still makes a solid.
///
/// Mapped by longitude across and latitude up, with each pole placed at the
/// middle of its edge of the chart — the best a single shared vertex can do
/// where a whole row of texels meets.
#[must_use]
pub fn sphere(radius: f32, rings: usize, segments: usize) -> PolyMesh {
    let rings = rings.max(2);
    let segments = segments.max(3);
    let mut mesh = PolyMesh::new();

    let north = mesh.push_uv_vertex(Vec3::Y * radius, Vec2::new(0.5, 1.0));
    // Interior rings only; the poles are single vertices.
    for ring in 1..rings {
        let along = ring as f32 / rings as f32;
        let polar = std::f32::consts::PI * along;
        for segment in 0..segments {
            mesh.push_uv_vertex(
                on_sphere(radius, polar, turn(segment, segments)),
                Vec2::new(segment as f32 / segments as f32, 1.0 - along),
            );
        }
    }
    let south = mesh.push_uv_vertex(-Vec3::Y * radius, Vec2::new(0.5, 0.0));

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

/// A sphere about `+Z` whose latitude rings are placed where the caller says.
///
/// [`sphere`] spaces its rings evenly about `+Y`, which is the right answer for
/// a ball and the wrong one for an eyeball. An iris is a set of CONCENTRIC
/// circles about the gaze axis, and the two things that decide whether one reads
/// as an eye are both angular: an 11.7 mm iris is 28.9° of half-angle on a life
/// globe and a 3.5 mm pupil is 8.2°. Evenly spaced rings about `+Y` cross that
/// pattern diagonally and land nowhere near either boundary, so a per-vertex
/// colour draws a limbus as an 18° Gouraud smear and cannot draw a pupil at all
/// (#81).
///
/// So `polars` is the list of interior latitudes, in radians from the `+Z` pole,
/// and a caller that wants a crisp colour boundary puts a PAIR of rings a degree
/// or two either side of it. Sorted and clamped here rather than trusted: a ring
/// list out of order would wind faces backwards.
///
/// Mapped as an azimuthal projection about the same pole — the `+Z` pole at the
/// middle of the chart and latitude running out to its rim. Unlike a longitude
/// map this has **no wrap seam**, because azimuth enters only through a sine and
/// a cosine, which is what an eye wants: the seam of a lat-long map would run
/// straight through the iris.
#[must_use]
pub fn sphere_rings(radius: f32, polars: &[f32], segments: usize) -> PolyMesh {
    let segments = segments.max(3);
    let mut rings: Vec<f32> = polars
        .iter()
        .copied()
        .filter(|polar| polar.is_finite())
        .map(|polar| polar.clamp(1e-4, std::f32::consts::PI - 1e-4))
        .collect();
    rings.sort_by(f32::total_cmp);
    if rings.is_empty() {
        rings.push(std::f32::consts::FRAC_PI_2);
    }

    let mut mesh = PolyMesh::new();
    let front = mesh.push_uv_vertex(Vec3::Z * radius, Vec2::splat(0.5));
    for &polar in &rings {
        // The chart's radius, running 0 at the near pole to 0.5 at the far one.
        let out = polar / std::f32::consts::PI * 0.5;
        for segment in 0..segments {
            let azimuth = turn(segment, segments);
            let (sin, cos) = azimuth.sin_cos();
            mesh.push_uv_vertex(
                Vec3::new(
                    radius * polar.sin() * cos,
                    radius * polar.sin() * sin,
                    radius * polar.cos(),
                ),
                Vec2::new(0.5 + out * cos, 0.5 + out * sin),
            );
        }
    }
    let back = mesh.push_uv_vertex(-Vec3::Z * radius, Vec2::new(0.5, 1.0));

    let at = |ring: usize, segment: usize| (1 + ring * segments + segment % segments) as u32;
    for segment in 0..segments {
        mesh.push_face([front, at(0, segment), at(0, segment + 1)]);
    }
    for ring in 0..rings.len() - 1 {
        for segment in 0..segments {
            mesh.push_face([
                at(ring, segment),
                at(ring + 1, segment),
                at(ring + 1, segment + 1),
                at(ring, segment + 1),
            ]);
        }
    }
    let last = rings.len() - 1;
    for segment in 0..segments {
        mesh.push_face([back, at(last, segment + 1), at(last, segment)]);
    }

    mesh
}

/// A closed shell cut from a sphere, capping the `+Y` pole.
///
/// `half_angle` is how far down from the pole the cap reaches, in radians;
/// `thickness` is how far its inner surface sits below the outer one. The result
/// is a solid: outer surface, inner surface, and a rim joining them.
///
/// A circular rim, which is the one thing a cap cannot help being — see
/// [`margin_shell`], which is this with the rim allowed to move.
#[must_use]
pub fn cap_shell(
    radius: f32,
    thickness: f32,
    half_angle: f32,
    rings: usize,
    segments: usize,
) -> PolyMesh {
    let segments = segments.max(3);
    margin_shell(radius, thickness, &vec![half_angle; segments], rings)
}

/// The same shell with its rim free to move: one polar angle per segment.
///
/// **Because two circles cannot meet at two points.** An eyelid is a cap whose
/// rim has to run from one canthus round to the other and back, meeting its
/// opposite lid at both — and two spherical caps with circular rims either miss
/// each other everywhere (leaving an annulus of bare eye, which is what this
/// crate shipped) or overlap along a whole arc. Letting the rim's polar angle
/// vary with azimuth is the smallest change that lets a boundary be authored
/// rather than inherited from the shape of a cap (#81).
///
/// `rim` gives the polar angle at each segment, measured from the `+Y` pole, and
/// its length is the segment count. Angles past 90° are meaningful and used: a
/// lid reaches well below the eye's equator away from the fissure.
///
/// **A rim that varies faster than the rings can follow will sag into whatever
/// the shell is meant to clear.** The surface between two rings is a chord, and
/// a chord across `d` radians sits at `cos(d/2)` of the radius: three rings over
/// a 112° rim leaves 0.947, so a shell at 1.06 of a globe passes 0.9996 of it
/// and z-fights. Either add rings or stand the shell further off.
#[must_use]
pub fn margin_shell(radius: f32, thickness: f32, rim: &[f32], rings: usize) -> PolyMesh {
    let rings = rings.max(1);
    let segments = rim.len().max(3);
    let at = |segment: usize| {
        rim.get(segment % rim.len())
            .copied()
            .unwrap_or(1.0)
            .clamp(0.05, std::f32::consts::PI - 0.05)
    };
    let inner = (radius - thickness.abs()).max(radius * 0.2);

    // Outer surface over the top half of the chart, inner over the bottom, both
    // running from their pole out to the rim they share at the middle.
    let mut mesh = PolyMesh::new();
    let outer_pole = mesh.push_uv_vertex(Vec3::Y * radius, Vec2::new(0.5, 1.0));
    for ring in 1..=rings {
        let along = ring as f32 / rings as f32;
        for segment in 0..segments {
            mesh.push_uv_vertex(
                on_sphere(radius, at(segment) * along, turn(segment, segments)),
                Vec2::new(segment as f32 / segments as f32, 1.0 - 0.5 * along),
            );
        }
    }
    let inner_start = mesh.vertex_count();
    let inner_pole = mesh.push_uv_vertex(Vec3::Y * inner, Vec2::new(0.5, 0.0));
    for ring in 1..=rings {
        let along = ring as f32 / rings as f32;
        for segment in 0..segments {
            mesh.push_uv_vertex(
                on_sphere(inner, at(segment) * along, turn(segment, segments)),
                Vec2::new(segment as f32 / segments as f32, 0.5 * along),
            );
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
/// The cross-section is a ring of `sides` vertices, parallel-transported down
/// the path so the tube does not twist. Half-extents rather than a radius so
/// the section can be a ribbon as well as a circle.
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
/// [`ribbon`] can only run one cross-section into another, and that is not
/// enough for anything whose width comes and goes: a foot is narrow at the
/// heel, widest at the ball, and narrows again at the toe, and no interpolation
/// between two ends will say that.
#[must_use]
pub fn sweep(path: &[Vec3], sections: &[Vec2], sides: usize, across: Vec3) -> PolyMesh {
    let sides = sides.max(3);
    let outlines: Vec<Vec<Vec2>> = sections
        .iter()
        .map(|half| {
            (0..sides)
                .map(|side| {
                    let angle = turn(side, sides);
                    Vec2::new(angle.cos() * half.x, angle.sin() * half.y)
                })
                .collect()
        })
        .collect();
    sweep_outline(path, &outlines, across)
}

/// A tube swept along `path` with an explicit closed outline at every station.
///
/// **The cross-section a body part wants is not always an ellipse, and a foot is
/// the case that proves it.** [`sweep`]'s ring puts a vertex at every `turn`,
/// which on an even count means one pointing straight down — so a foot swept that
/// way rests on a keel rather than on a sole, and measured by ray-cast against the
/// built mesh it touched the ground along its centre line and rose 6 mm at the
/// quarter width and up to 19 mm at its edges. Both Quaternius reference bodies
/// are flat across the whole sole to within a few millimetres (#110).
///
/// Flattening an ellipse afterwards would be the wrong repair: the outline is what
/// is wrong, so the outline is what this takes. [`sweep`] is now an ellipse
/// outline through here, which keeps one stitcher, one parallel-transport frame
/// and one UV convention for every swept part in the crate.
///
/// Each outline is in the section's own plane — `x` along `across`, `y` along the
/// path's up — and must have the same number of points as every other, since the
/// stations are stitched to each other in order.
#[must_use]
pub fn sweep_outline(path: &[Vec3], outlines: &[Vec<Vec2>], across: Vec3) -> PolyMesh {
    let mut mesh = PolyMesh::new();
    if path.len() < 2 || outlines.len() < path.len() {
        return mesh;
    }
    let sides = outlines[0].len();
    if sides < 3 || outlines.iter().any(|ring| ring.len() != sides) {
        return mesh;
    }

    let mut direction = (path[1] - path[0]).normalize_or(Vec3::Y);
    let (mut u, mut v) = {
        let square = across - direction * across.dot(direction);
        let u = square.normalize_or(frame(direction).0);
        (u, direction.cross(u))
    };
    let mut rings: Vec<Vec<u32>> = Vec::with_capacity(path.len());
    // `v` follows arc length rather than the point index, so a path whose
    // samples bunch up — a hair strand's densely sampled cap, a foot's ball —
    // does not stretch its texture where it was sampled finely.
    let along_path = arc_lengths(path);

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

        let outline = &outlines[index];
        let ring: Vec<u32> = (0..sides)
            .map(|side| {
                let at = outline[side];
                mesh.push_uv_vertex(
                    point + u * at.x + v * at.y,
                    Vec2::new(side as f32 / sides as f32, along_path[index]),
                )
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

/// How far along `path` each point sits, as a fraction of its whole length.
///
/// A path of zero length — every sample in one spot — spreads its points evenly
/// rather than dividing by nothing.
fn arc_lengths(path: &[Vec3]) -> Vec<f32> {
    let mut run = 0.0f32;
    let mut lengths = Vec::with_capacity(path.len());
    lengths.push(0.0);
    for step in path.windows(2) {
        run += step[0].distance(step[1]);
        lengths.push(run);
    }
    if run <= f32::EPSILON {
        let last = (path.len().max(2) - 1) as f32;
        return (0..path.len()).map(|at| at as f32 / last).collect();
    }
    for length in &mut lengths {
        *length /= run;
    }
    lengths
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

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;

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
    fn a_margin_shell_takes_its_rim_from_the_angles_it_is_given() {
        // The property [`cap_shell`] cannot have: a rim that is somewhere else
        // at every azimuth. Two shells like this can meet at two points, which
        // is what an eyelid needs and what two circles cannot do (#81).
        let segments = 16;
        let rim: Vec<f32> = (0..segments)
            .map(|segment| {
                // Shallow at +X, reaching past the equator a quarter turn on.
                let turn = std::f32::consts::TAU * segment as f32 / segments as f32;
                1.0 - 0.8 * turn.cos()
            })
            .collect();
        let mesh = margin_shell(0.5, 0.03, &rim, 4);
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());

        // Every rim vertex sits at the polar angle it was asked for, on the
        // outer surface. Read off the mesh rather than trusted: the rim is the
        // last ring of the outer run, which is where the two surfaces join.
        for (segment, &want) in rim.iter().enumerate() {
            let at = mesh.positions[1 + (4 - 1) * segments + segment];
            let polar = (at.y / at.length()).clamp(-1.0, 1.0).acos();
            assert!(
                (polar - want).abs() < 1e-3,
                "segment {segment} sits at {polar:.4} rad against the {want:.4} it was given"
            );
        }

        // And a constant rim is exactly a cap, so the two cannot drift apart.
        let capped = cap_shell(0.5, 0.03, 1.2, 4, 16);
        let flat = margin_shell(0.5, 0.03, &[1.2; 16], 4);
        assert_eq!(capped.positions, flat.positions);
        assert_eq!(capped.faces, flat.faces);
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
    fn every_primitive_comes_out_mapped() {
        let shapes = [
            ("sphere", sphere(0.5, 8, 12)),
            ("cap_shell", cap_shell(0.5, 0.03, 1.2, 4, 12)),
            (
                "sweep",
                tube(
                    &[Vec3::ZERO, Vec3::new(0.0, -0.2, 0.0), Vec3::NEG_Y],
                    Vec2::splat(0.03),
                    Vec2::splat(0.01),
                    6,
                ),
            ),
        ];
        for (name, mesh) in shapes {
            assert!(mesh.channels_are_consistent(), "{name}");
            assert_eq!(mesh.uvs.len(), mesh.vertex_count(), "{name} is unmapped");
            for uv in &mesh.uvs {
                assert!(
                    (0.0..=1.0).contains(&uv.x) && (0.0..=1.0).contains(&uv.y),
                    "{name} put a vertex at {uv:?}"
                );
            }
        }
    }

    #[test]
    fn a_sweep_measures_v_by_arc_length_not_by_sample() {
        // Half the samples cover the first tenth of the path. Indexed v would
        // give that tenth half the texture; arc length gives it a tenth.
        let mut path = vec![Vec3::ZERO];
        for step in 1..=5 {
            path.push(Vec3::new(0.0, -0.02 * step as f32, 0.0));
        }
        path.push(Vec3::new(0.0, -1.0, 0.0));
        let mesh = tube(&path, Vec2::splat(0.01), Vec2::splat(0.01), 4);
        let at = |ring: usize| mesh.uvs[ring * 4].y;
        assert!(
            (at(5) - 0.1).abs() < 0.01,
            "the dense end took {} of the chart",
            at(5)
        );
        assert!((at(6) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_wrapping_chart_splits_into_a_seam_only_where_it_wraps() {
        // The last column of a tube runs from high u back to zero. Split, the
        // face becomes continuous; unsplit, it covers the whole chart backwards.
        let mesh = tube(
            &[Vec3::ZERO, Vec3::NEG_Y],
            Vec2::splat(0.1),
            Vec2::splat(0.1),
            8,
        );
        // Measured over the walls only. The two end caps span the whole chart
        // by construction and are documented as doing so — they are poles, and
        // splitting one would only move the smear.
        let widest = |mesh: &PolyMesh| {
            mesh.faces
                .iter()
                .filter(|face| face.len() == 4)
                .map(|face| {
                    let (lo, hi) = face.iter().fold((f32::MAX, f32::MIN), |acc, &c| {
                        (
                            acc.0.min(mesh.uvs[c as usize].x),
                            acc.1.max(mesh.uvs[c as usize].x),
                        )
                    });
                    hi - lo
                })
                .fold(0.0f32, f32::max)
        };
        assert!(
            widest(&mesh) > 0.5,
            "the wrap should be there to begin with"
        );

        let split = mesh.split_uv_seams();
        assert!(split.channels_are_consistent());
        assert_eq!(split.face_count(), mesh.face_count(), "no face was lost");
        // Only the seam column duplicates: one vertex per ring, two rings.
        assert_eq!(split.vertex_count(), mesh.vertex_count() + 2);
        assert!(
            widest(&split) <= 0.5,
            "a wall still spans {} of the chart",
            widest(&split)
        );
        // Every duplicate sits exactly on top of the vertex it came from.
        for extra in mesh.vertex_count()..split.vertex_count() {
            assert!(
                mesh.positions.contains(&split.positions[extra]),
                "a split vertex moved"
            );
        }
        // And a cap is left whole rather than being cut into a worse smear.
        let caps: Vec<&Vec<u32>> = split.faces.iter().filter(|f| f.len() != 4).collect();
        assert_eq!(caps.len(), 2);
        for cap in caps {
            assert!(cap.iter().all(|&c| (c as usize) < mesh.vertex_count()));
        }
    }

    #[test]
    fn splitting_an_unmapped_mesh_changes_nothing() {
        let mut mesh = sphere(0.3, 5, 8);
        mesh.set_uvs(Vec::new());
        assert_eq!(mesh.split_uv_seams(), mesh);
    }

    #[test]
    fn transforming_carries_the_channels_and_reorients_the_normals() {
        let mut mesh = sphere(0.4, 6, 10);
        mesh.set_normals(mesh.vertex_normals());
        mesh.bind_rigidly(3);
        mesh.paint(Vec3::new(0.2, 0.4, 0.6));

        let turned = mesh.transformed(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2));
        assert!(turned.channels_are_consistent());
        assert_eq!(turned.uvs, mesh.uvs, "coordinates are not positions");
        assert_eq!(turned.skin, mesh.skin);
        assert_eq!(turned.colours, mesh.colours);
        for (before, after) in mesh.normals.iter().zip(&turned.normals) {
            assert!(
                (after.length() - 1.0).abs() < 1e-4,
                "a normal lost its unit"
            );
            assert!(
                (after.y - before.x).abs() < 1e-4 && (after.x + before.y).abs() < 1e-4,
                "{before:?} turned into {after:?}"
            );
        }

        // A mirror flips winding, and the inverse transpose flips the normals
        // with it, so the copy still faces out of its own surface.
        let flipped = mesh.transformed(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
        for (index, face) in flipped.faces.iter().enumerate() {
            let a = flipped.positions[face[0] as usize];
            let b = flipped.positions[face[1] as usize];
            let c = flipped.positions[face[2] as usize];
            assert!(
                (b - a).cross(c - a).dot(flipped.normals[face[0] as usize]) > 0.0,
                "face {index} disagrees with its normal"
            );
        }
    }

    #[test]
    fn appending_unions_the_channels_rather_than_dropping_them() {
        // The failure this exists to prevent: merge a mapped part into an
        // unmapped one and every coordinate after the join belongs to the wrong
        // vertex — or the channel vanishes and the part draws untextured.
        let mapped = sphere(0.3, 5, 8);
        let mut plain = sphere(0.3, 5, 8);
        plain.set_uvs(Vec::new());

        let mut onto_plain = plain.clone();
        onto_plain.append(&mapped);
        assert!(onto_plain.channels_are_consistent());
        assert_eq!(
            &onto_plain.uvs[plain.vertex_count()..],
            mapped.uvs.as_slice(),
            "the mapped half lost its coordinates"
        );

        let mut onto_mapped = mapped.clone();
        onto_mapped.append(&plain);
        assert!(onto_mapped.channels_are_consistent());
        assert_eq!(&onto_mapped.uvs[..mapped.vertex_count()], mapped.uvs);

        // And a channel neither side carries stays absent.
        assert!(onto_mapped.skin.is_empty() && onto_mapped.colours.is_empty());
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
