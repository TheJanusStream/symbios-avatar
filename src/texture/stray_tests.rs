//! Texels the bake paints from somewhere else on the body (#472).
//!
//! Every texel the bake writes is either inside a triangle or within
//! [`COVER`] of one, and should say where on the body the nearest point of the
//! nearest such triangle is. The instrument here reads that back: for each
//! texel the bake drew (before the gutters are dilated), the triangles nearest
//! its centre in the atlas, the nearest point of each, and how far the baked
//! position lies from the closest of those points on the skin, against one
//! texel's own footprint there - the longer axis of its [`Footprint`], the
//! skin one step to the next texel spans the coarser way. Two triangles within
//! one of [`RANK_STEPS`]' steps of a texel's centre are both its nearest.
//!
//! **Nothing here asks the bake where a triangle's nearest point is.** The
//! nearest point comes from [`nearest_by_segments`], written apart from the
//! bake's own [`nearest_on_edges`], and a crossed texel from separating axes
//! ([`crosses`]). With both sides reading the bake's function, a bake whose
//! rim texels took each edge's weights swapped passed every test here.

use glam::{Vec2, Vec3};

use super::*;
use crate::{Avatar, AvatarConfig};

/// A texel the bake painted from somewhere other than its nearest triangle.
#[derive(Clone, Copy, Debug)]
pub(super) struct Stray {
    /// Its index in [`AtlasGeometry::texels`].
    pub(super) at: usize,
    /// Its zone, as baked.
    pub(super) zone: Zone,
    /// How far, in metres, the baked position lies from the nearest point of
    /// the nearest triangle; infinite where no triangle lies within [`COVER`].
    pub(super) off: f32,
    /// One texel's footprint there, in metres.
    pub(super) footprint: f32,
    /// `1 - cos` of the angle between the baked normal and the nearest
    /// point's.
    pub(super) bent: f32,
    /// The smallest corner, in degrees, of any triangle the texel's centre
    /// lies within [`COVER`] of, in the atlas.
    pub(super) sharpest: f32,
}

/// What [`read`] found over one bake.
#[derive(Debug, Default)]
pub(super) struct Reading {
    /// How many texels the bake drew.
    pub(super) drawn: usize,
    /// Those further than a footprint from the nearest point of every
    /// triangle nearest them, or within [`COVER`] of none.
    pub(super) strays: Vec<Stray>,
}

/// The point of the triangle `pixels` nearest `point`, all in texels: how far
/// it lies from `point`, `0` inside, and its barycentric weights.
fn closest(pixels: &[Vec2; 3], point: Vec2) -> (f32, Vec3) {
    let area = edge(pixels[0], pixels[1], pixels[2]);
    let raw = Vec3::new(
        edge(pixels[1], pixels[2], point),
        edge(pixels[2], pixels[0], point),
        edge(pixels[0], pixels[1], point),
    );
    if (raw * area.signum()).min_element() >= 0.0 {
        return (0.0, raw / area);
    }
    nearest_by_segments(pixels, point)
}

/// The point of the triangle `pixels` nearest a `point` outside it, as the
/// nearest of its three edges' nearest points: how far it lies, and its
/// barycentric weights.
///
/// **Written apart from the bake's [`nearest_on_edges`] on purpose**, so that a
/// mistake in either shows against the other: each edge is walked from its
/// start along its unit direction, and the weights are read off how far along
/// it the nearest point lies. The nearest point of a triangle to a point
/// outside it is unique, so the two agree to rounding wherever both are right.
/// Not the corner-and-edge regions of Ericson's construction (Real-Time
/// Collision Detection, 5.1.5), tried first: its region tests subtract
/// products that nearly cancel on a sliver, and it put one of seed 237's
/// texels out of reach of every triangle, where this and the bake both find
/// the triangle that drew it within reach.
fn nearest_by_segments(pixels: &[Vec2; 3], point: Vec2) -> (f32, Vec3) {
    let mut best = (f32::INFINITY, Vec3::ZERO);
    for (start, end) in [(0usize, 1usize), (1, 2), (2, 0)] {
        let run = pixels[end] - pixels[start];
        let length = run.length();
        let travelled = if length > 0.0 {
            (point - pixels[start]).dot(run / length).clamp(0.0, length)
        } else {
            0.0
        };
        let share = if length > 0.0 {
            travelled / length
        } else {
            0.0
        };
        let distance = point.distance(pixels[start].lerp(pixels[end], share));
        if distance < best.0 {
            let mut weights = Vec3::ZERO;
            weights[start] += 1.0 - share;
            weights[end] += share;
            best = (distance, weights);
        }
    }
    best
}

/// Whether the triangle `pixels` crosses texel `(x, y)`'s square, by
/// separating axes: the two overlap unless the square's own two axes or one of
/// the triangle's three edge normals parts them. Exact, and owes nothing to
/// either nearest-point construction.
fn crosses(pixels: &[Vec2; 3], x: u32, y: u32) -> bool {
    let corner = Vec2::new(x as f32, y as f32);
    let square = [
        corner,
        corner + Vec2::X,
        corner + Vec2::ONE,
        corner + Vec2::Y,
    ];
    let span = |points: &[Vec2], axis: Vec2| {
        points
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), point| {
                let along = point.dot(axis);
                (lo.min(along), hi.max(along))
            })
    };
    [
        Vec2::X,
        Vec2::Y,
        (pixels[1] - pixels[0]).perp(),
        (pixels[2] - pixels[1]).perp(),
        (pixels[0] - pixels[2]).perp(),
    ]
    .into_iter()
    .all(|axis| {
        let (triangle_lo, triangle_hi) = span(pixels, axis);
        let (square_lo, square_hi) = span(&square, axis);
        triangle_lo <= square_hi && square_lo <= triangle_hi
    })
}

/// The smallest corner of a triangle, in degrees.
fn sharpest(pixels: &[Vec2; 3]) -> f32 {
    (0..3)
        .map(|corner| {
            let at = pixels[corner];
            let (a, b) = (pixels[(corner + 1) % 3] - at, pixels[(corner + 2) % 3] - at);
            a.angle_to(b).abs().to_degrees()
        })
        .fold(180.0, f32::min)
}

/// Reads every texel [`draw`] fills for these inputs against the nearest point
/// of the triangles nearest it.
/// How far past [`COVER`] a triangle still counts as reaching a texel here, in
/// texels: the bake and this read a texel's distance to a triangle two ways,
/// which agree to rounding, and on the reach's own line rounding decides
/// (#473). Under a nudge of libm one texel of beast 0's hind leg, once the
/// packer laid it elsewhere, stood 0.7100058 texels from the triangle the
/// bake drew it from, and so off every triangle in reach here.
const ROUNDING: f32 = 1e-4;

pub(super) fn read(
    mesh: &PolyMesh,
    uv: &UvUnwrap,
    parts: &[(&PolyMesh, Zone, &[f32])],
    mouth: &[f32],
    size: u32,
) -> Reading {
    let (geometry, _) = draw(mesh, uv, parts, mouth, size, false);
    let count = (size * size) as usize;
    let scale = Vec2::splat(size as f32);
    let each = |corners: &Corners, visit: &mut dyn FnMut(usize, Vec2, &[Vec2; 3])| {
        let pixels = corners.uvs.map(|uv| uv * scale);
        if edge(pixels[0], pixels[1], pixels[2]).abs() < 1e-9 {
            return;
        }
        let lo = pixels[0].min(pixels[1]).min(pixels[2]) - Vec2::splat(COVER);
        let hi = pixels[0].max(pixels[1]).max(pixels[2]) + Vec2::splat(COVER);
        let (x0, y0) = (lo.x.floor().max(0.0) as u32, lo.y.floor().max(0.0) as u32);
        let x1 = (hi.x.ceil() as i64).clamp(0, i64::from(size)) as u32;
        let y1 = (hi.y.ceil() as i64).clamp(0, i64::from(size)) as u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                visit((y * size + x) as usize, point, &pixels);
            }
        }
    };

    // The distance to the nearest triangle, and the sharpest corner of any
    // within reach.
    let mut nearest = vec![f32::INFINITY; count];
    let mut sharp = vec![180.0f32; count];
    triangles(mesh, uv, parts, mouth, size, true, |_, corners| {
        each(corners, &mut |at, point, pixels| {
            let (distance, _) = closest(pixels, point);
            if distance <= COVER + ROUNDING {
                nearest[at] = nearest[at].min(distance);
                sharp[at] = sharp[at].min(sharpest(pixels));
            }
        });
    });

    // The best fit among the nearest triangles: (off / footprint, off,
    // footprint, bent).
    let mut best = vec![(f32::INFINITY, f32::INFINITY, 0.0f32, 0.0f32); count];
    triangles(mesh, uv, parts, mouth, size, true, |_, corners| {
        each(corners, &mut |at, point, pixels| {
            let Some(baked) = geometry.texels[at] else {
                return;
            };
            let (distance, w) = closest(pixels, point);
            if distance > COVER + ROUNDING || distance > nearest[at] + COVER / RANK_STEPS {
                return;
            }
            let position = corners.positions[0] * w.x
                + corners.positions[1] * w.y
                + corners.positions[2] * w.z;
            let normal =
                (corners.normals[0] * w.x + corners.normals[1] * w.y + corners.normals[2] * w.z)
                    .normalize_or(Vec3::Y);
            let spread =
                corners.spreads[0] * w.x + corners.spreads[1] * w.y + corners.spreads[2] * w.z;
            let footprint = Footprint::of(spread, normal).axes[0].length().max(1e-4);
            let off = baked.position.distance(position);
            let ratio = off / footprint;
            if ratio < best[at].0 {
                best[at] = (ratio, off, footprint, 1.0 - baked.normal.dot(normal));
            }
        });
    });

    let mut reading = Reading::default();
    for (at, texel) in geometry.texels.iter().enumerate() {
        let Some(texel) = texel else { continue };
        reading.drawn += 1;
        let (ratio, off, footprint, bent) = best[at];
        if ratio > 1.0 {
            reading.strays.push(Stray {
                at,
                zone: texel.zone,
                off,
                footprint,
                bent,
                sharpest: sharp[at],
            });
        }
    }
    reading
}

/// [`read`] over an avatar's own atlas: its body, its attached parts and its
/// mouth, as [`Avatar::build`] baked them.
pub(super) fn read_avatar(avatar: &Avatar) -> Reading {
    let parts = &avatar.parts;
    let placed = crate::avatar::attached_in_body(
        &avatar.rig,
        &parts.features,
        &parts.extremities,
        &parts.eyes,
        &parts.beast,
        &parts.wings,
    );
    let borrowed: Vec<(&PolyMesh, Zone, &[f32])> = placed
        .iter()
        .map(|(mesh, zone, mouth)| (mesh, *zone, mouth.as_slice()))
        .collect();
    let mut inside = vec![0.0f32; parts.body.vertex_count()];
    if let Some(mouth) = &parts.mouth {
        for &vertex in mouth.roof.iter().chain(&mouth.floor) {
            inside[vertex as usize] = 1.0;
        }
        for &vertex in &mouth.teeth {
            inside[vertex as usize] = 0.5;
        }
    }
    read(
        &parts.body,
        &parts.unwrap,
        &borrowed,
        &inside,
        AvatarConfig::default().atlas,
    )
}

/// One triangle's [`Corners`], its corners `pixels` texels into an atlas of
/// `size`, each at its own pixel on the body: so a texel's baked position
/// says which point of the triangle it was painted from, in texels.
fn flat(pixels: [Vec2; 3], size: u32) -> Corners {
    Corners {
        uvs: pixels.map(|pixel| pixel / size as f32),
        positions: pixels.map(|pixel| pixel.extend(0.0)),
        normals: [Vec3::Z; 3],
        creases: [0.0; 3],
        mouths: [0.0, 0.0, 1.0],
        spreads: [Mat3::ZERO; 3],
    }
}

/// An atlas `size` texels square with these triangles rasterised into it,
/// in this order, before dilation.
fn drawn(size: u32, all: &[Corners]) -> AtlasGeometry {
    let mut geometry = AtlasGeometry {
        width: size,
        height: size,
        texels: vec![None; (size * size) as usize],
    };
    let mut measured = Footprints::over(size, size);
    let mut ranks = vec![u8::MAX; (size * size) as usize];
    for corners in all {
        rasterize(
            &mut geometry,
            Some(&mut measured),
            &mut ranks,
            Zone::Chest,
            false,
            corners,
        );
    }
    geometry
}

#[test]
fn a_texel_beside_a_needle_paints_from_the_needle_s_nearest_point() {
    // #472: a needle thirty texels long and a fifth of a texel thick at its
    // widest, its third corner almost on its long edge near one end, as
    // fanning a face whose extra corner lies on an edge makes one. Every
    // texel it touches is filled (#309: a sliver lands on every row and
    // column it crosses), and each says the needle's point nearest its
    // centre. Read before the fix: the texel half a texel under the long
    // edge at x 12.5 painted from x 31.4, and every texel along that side
    // from between x 30.6 and the third corner's end; and texels beyond the
    // sharp corner were filled out to 9.5 texels past it, where the atlas
    // ends.
    //
    // The second runs across the atlas's diagonal with its sharp corner off
    // the texel grid, at (10.4, 49.6): the texel past that corner along the
    // needle's line, (9, 50), lies within the margin of all three edges'
    // lines and 1.27 texels from the needle, out of reach. With its corners on
    // the grid every texel a bake could reach past a corner lies within 0.71,
    // and the first needle alone passed a bake whose reach was doubled or
    // measured to the edges' lines (Group 25's critic).
    let size = 64;
    for needle in [
        [
            Vec2::new(10.0, 10.0),
            Vec2::new(40.0, 10.0),
            Vec2::new(39.0, 10.2),
        ],
        [
            Vec2::new(10.4, 49.6),
            Vec2::new(40.4, 19.6),
            Vec2::new(39.4, 20.8),
        ],
    ] {
        let geometry = drawn(size, &[flat(needle, size)]);
        let mut filled = 0;
        for y in 0..size {
            for x in 0..size {
                let centre = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let (distance, weights) = closest(&needle, centre);
                let texel = geometry.get(x, y);
                if crosses(&needle, x, y) {
                    assert!(
                        texel.is_some(),
                        "{needle:?}: ({x}, {y}) is crossed and empty"
                    );
                }
                let Some(texel) = texel else { continue };
                filled += 1;
                assert!(
                    distance <= COVER,
                    "{needle:?}: ({x}, {y}) is filled {distance:.2} texels from the needle"
                );
                let nearest = needle[0] * weights.x + needle[1] * weights.y + needle[2] * weights.z;
                assert!(
                    texel.position.truncate().distance(nearest) < 1e-3,
                    "{needle:?}: ({x}, {y}) painted from {:?}, where the needle's nearest point \
                     is {nearest:?}",
                    texel.position.truncate()
                );
                assert!(
                    (texel.mouth - weights.z).abs() < 1e-4,
                    "{needle:?}: ({x}, {y}) reads {} of the third corner's channel, where its \
                     nearest point reads {}",
                    texel.mouth,
                    weights.z
                );
            }
        }
        assert!(
            filled >= 60,
            "{needle:?}: the needle filled only {filled} texels"
        );
    }
}

#[test]
fn the_nearest_triangle_paints_a_texel_whichever_is_drawn_first() {
    // #472: a texel inside one triangle and within reach of another's rim,
    // and a texel within reach of two rims, take the nearest triangle's
    // point drawn in either order: two triangles far apart on the body (each
    // at its own pixels plus an offset that tells them apart) whose charts
    // lie within a texel of each other in the atlas. Read before the fix:
    // the texel at (5, 5), inside the first, painted from the second when
    // the second was drawn after it, and the texel at (5, 6) took whichever
    // rim was drawn last.
    let size = 16;
    let one = || {
        flat(
            [
                Vec2::new(0.0, 6.1),
                Vec2::new(16.0, 6.1),
                Vec2::new(8.0, -20.0),
            ],
            size,
        )
    };
    // The first's top edge runs along y = 6.1 and the second's bottom edge
    // along y = 6.8: (5, 6)'s centre lies 0.4 over the one and 0.3 under the
    // other.
    let two = || {
        let mut two = flat(
            [
                Vec2::new(16.0, 6.8),
                Vec2::new(0.0, 6.8),
                Vec2::new(8.0, 30.0),
            ],
            size,
        );
        two.positions = two.positions.map(|p| p + Vec3::new(0.0, 0.0, 100.0));
        two
    };
    // Its bottom edge along y = 5.7 reaches (5, 5)'s centre 0.2 under it,
    // which lies inside the first.
    let three = || {
        let mut three = flat(
            [
                Vec2::new(0.0, 5.7),
                Vec2::new(16.0, 5.7),
                Vec2::new(8.0, 30.0),
            ],
            size,
        );
        three.positions = three.positions.map(|p| p - Vec3::new(0.0, 0.0, 100.0));
        three
    };
    for both in [drawn(size, &[one(), two()]), drawn(size, &[two(), one()])] {
        let texel = both.get(5, 6).expect("(5, 6) is in reach of both");
        assert!(
            texel.position.z > 50.0,
            "(5, 6) lies 0.3 from the second and 0.4 from the first, and painted from {:?}",
            texel.position
        );
    }
    for both in [
        drawn(size, &[one(), three()]),
        drawn(size, &[three(), one()]),
    ] {
        let texel = both.get(5, 5).expect("(5, 5) is inside the first");
        assert!(
            texel.position.z.abs() < 1e-3,
            "(5, 5) lies inside the first and painted from {:?}",
            texel.position
        );
    }
}

#[test]
fn no_body_paints_a_texel_from_off_its_nearest_triangle() {
    // #472 over whole bodies: every texel the bake draws says where the
    // nearest point of the nearest triangle is, to within one texel's
    // footprint there. Seed 237's person is Group 24's, whose temple
    // hairline showed a one-texel scalp tick. Read before the fix: 18,826
    // of that person's texels further off than a footprint. Over the sixteen
    // published persons, beasts and winged bodies, rolls and slider corners,
    // 4,700 to 60,000 a body; 600 to 11,400 more than five footprints off;
    // 360 to 780 filled with no triangle within reach; none since.
    use crate::plan::Archetype;
    use crate::{AvatarRecord, QuadrupedParams, WingedParams};
    let size = AvatarConfig::default().atlas as usize;
    for (label, archetype, seed) in [
        ("person", Archetype::default(), 237),
        ("beast", Archetype::Quadruped(QuadrupedParams::default()), 0),
        ("winged", Archetype::Winged(WingedParams::default()), 0),
    ] {
        let mut record = AvatarRecord::new("Stray", archetype);
        record.reroll(seed);
        let avatar = Avatar::build(&record).expect("builds");
        let reading = read_avatar(&avatar);
        assert!(
            reading.drawn > 100_000,
            "the {label} drew {}",
            reading.drawn
        );
        let Some(worst) = reading
            .strays
            .iter()
            .max_by(|a, b| (a.off / a.footprint).total_cmp(&(b.off / b.footprint)))
        else {
            continue;
        };
        panic!(
            "{} of seed {seed}'s {label}'s texels lie off their nearest triangle; the worst at \
             ({}, {}) in {:?} {:.1} mm off against a footprint of {:.1} mm, its normal {:.3} off, \
             beside a corner of {:.2} degrees",
            reading.strays.len(),
            worst.at % size,
            worst.at / size,
            worst.zone,
            worst.off * 1000.0,
            worst.footprint * 1000.0,
            worst.bent,
            worst.sharpest,
        );
    }
}
