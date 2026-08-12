//! One clump of hair, turned into triangles.
//!
//! A clump is a spine and a cross-section swept down it. Everything expensive
//! about hair is decided here, so two things are worth stating up front:
//!
//! - **A clump is sampled by how much it BENDS**, not by how far it travels and
//!   not by a fixed count. #40's lesson was the second of those — a fringe a
//!   tenth the length of the hair behind it does not need the same number of
//!   points — and following it by travel alone still spends fourteen stations
//!   drawing a straight line. The tolerance that replaces it is this module's
//!   own `FLATNESS`.
//! - **The gradient is vertex colour**, so a root-to-tip fade costs nothing: no
//!   texture, no second material, no atlas space. It is the whole reason the
//!   owner's two-colour model is affordable at this triangle count.

use glam::{Vec2, Vec3};

use super::Shape;
use super::scatter::Root;
use crate::mesh::PolyMesh;
use crate::prim;

/// How far a clump's drawn spine may stray from the curve it stands for, in
/// metres.
///
/// **The one number that decides what hair costs, and it is a tolerance rather
/// than a step** (#201). Sampling by travel alone spends the same on a straight
/// lock as on a curled one: at four millimetres a station the reference head
/// cost 87,168 triangles, of which the scalp's 883 clumps were 70,202 — and
/// most of those stations were drawing a straight line with fourteen points.
/// Splitting only where the polyline actually departs from the curve gives a
/// straight lock two stations and a curl as many as it needs, with no knob for
/// a style to get wrong.
///
/// A millimetre is under what the render can resolve at the framing a head is
/// judged at (0.86 mm per pixel), so a finer tolerance buys nothing anyone can
/// see.
///
/// Provenance: **derived** from the render's own resolution.
const FLATNESS: f32 = 0.001;

/// The fewest stations any clump gets, however short it is.
///
/// Two is a straight line with no bend at all, so three is the floor for
/// anything that has to look grown rather than extruded.
///
/// Provenance: **derived** from what a curve needs.
pub const LEAST: usize = 3;

/// The most any one clump gets, however long it is.
///
/// A backstop rather than a budget: a curve that keeps failing [`FLATNESS`] —
/// a style with a kink in it, or one whose arithmetic has gone wrong — would
/// otherwise subdivide until it ran out of memory, and something is wrong
/// upstream long before a hundred stations is the right answer for one lock.
///
/// Provenance: **derived** from the longest hair a record can ask for at the
/// tolerance above.
const MOST: usize = 96;

/// Sweeps one clump and appends it to `into`, gradient and all.
///
/// Returns how many stations it spent, which is what the caller's triangle
/// accounting is built from.
pub(super) fn loft(
    into: &mut PolyMesh,
    root: &Root,
    shape: &dyn Shape,
    roots_colour: Vec3,
    tips_colour: Vec3,
) -> usize {
    let length = shape.length(root);
    if length <= f32::EPSILON {
        return 0;
    }
    let (fractions, path) = sample(root, shape);
    let stations = path.len();
    // A path that doubled back on itself or collapsed would sweep an inside-out
    // tube; a style that does that is a bug in the style, and the sweep is not
    // the place to discover it.
    if path.windows(2).all(|step| step[0] == step[1]) {
        return 0;
    }

    // **A clump's width comes and goes, and a lock is the case that proves it**
    // (#205). `prim::ribbon` can only run one section into another, which draws a
    // wedge: full width at the root, a point at the tip. Overlapping wedges read
    // as a row of separate objects, because every one of them ends in a blunt
    // squared-off face where it began. A section asked for per station lets a
    // style be a leaf instead — thin, full, thin — so that where two clumps
    // overlap neither has an end.
    //
    // It costs nothing: the same stations, the same sides, the same triangles.
    // `prim::sweep` has taken a section per station since a foot needed one.
    let sections: Vec<Vec2> = fractions
        .iter()
        .map(|along| shape.section_at(root, *along))
        .collect();
    // **The wide axis is handed in rather than derived, and hair is the case
    // that proves it** (`prim::ribbon`'s own note). A frame derived from a
    // near-vertical path snaps to a world axis and the ribbon turns edge-on,
    // which is what turned a sheet of hair into separate strings.
    //
    // **And it is the STYLE that hands it in, not this file** (#205). Across the
    // fall is what a hanging lock's width lies along, and it is
    // [`Shape::across`]'s default; a brow's clumps run sideways and want their
    // width across that instead. The one thing a style may not do is name an
    // axis parallel to its own spine, so the parallel case is caught here rather
    // than left to the sweep's silent fallback.
    let tangent = (path[1] - path[0]).normalize_or(root.out);
    let named = shape.across(root).normalize_or(Vec3::ZERO);
    let squared = named - tangent * named.dot(tangent);
    let across = squared.normalize_or(
        root.out
            .cross(tangent)
            .normalize_or(tangent.cross(Vec3::Y).normalize_or(Vec3::X)),
    );
    let first = into.positions.len();
    let sides = shape.sides();
    let clump = prim::sweep(&path, &sections, sides, across);
    into.append(&clump);

    // The gradient, by travel rather than by height: hair that falls and then
    // curls back up is still older at its tip, and a colour taken from height
    // would run backwards over the curl.
    //
    // **By how far along the curve a station is, not by its index** (#205).
    // Adaptive sampling puts stations where a clump bends, so an index is not a
    // share of the way down the hair: on a curl the colour ran fast through the
    // bend and slowly over the straight, which is not what "root to tip" means
    // and is not what this comment used to claim it did. Identical on a straight
    // clump, where the two agree.
    let added = into.positions.len() - first;
    into.colours.resize(first, roots_colour);
    for vertex in 0..added {
        let station = (vertex / sides.max(1)).min(fractions.len().saturating_sub(1));
        into.colours
            .push(shade(roots_colour, tips_colour, fractions[station]));
    }
    stations
}

/// Samples a clump's spine only as finely as its own curvature needs.
///
/// Bisects the fractions where the drawn line strays furthest from the curve,
/// which converges on a polyline within [`FLATNESS`] of it everywhere. A
/// straight clump keeps the two ends it started with; a curl earns its stations
/// where it actually turns rather than spreading them evenly over a shape that
/// is flat for half its length.
///
/// Answers the fractions as well as the points, because the sections and the
/// gradient are both functions of how far along the curve a station is and the
/// index is not that (see the two call sites).
fn sample(root: &Root, shape: &dyn Shape) -> (Vec<f32>, Vec<Vec3>) {
    let mut fractions: Vec<f32> = (0..LEAST)
        .map(|station| station as f32 / (LEAST - 1) as f32)
        .collect();
    let mut points: Vec<Vec3> = fractions.iter().map(|at| shape.at(root, *at)).collect();
    while points.len() < MOST {
        // The gap whose midpoint is furthest off the chord that spans it.
        let mut worst = (0.0f32, 0usize);
        for gap in 0..points.len() - 1 {
            let middle = (fractions[gap] + fractions[gap + 1]) * 0.5;
            let strays = shape
                .at(root, middle)
                .distance(points[gap].lerp(points[gap + 1], 0.5));
            if strays > worst.0 {
                worst = (strays, gap);
            }
        }
        if worst.0 <= FLATNESS {
            break;
        }
        let gap = worst.1;
        let middle = (fractions[gap] + fractions[gap + 1]) * 0.5;
        fractions.insert(gap + 1, middle);
        points.insert(gap + 1, shape.at(root, middle));
    }
    (fractions, points)
}

/// The colour a share of the way from root to tip.
///
/// **Interpolated in the space the colours are stored in, which is sRGB**
/// (`PolyMesh::colours` is, whatever its own docstring once said), because
/// these two came off a record where a person picked them and a renderer draws
/// them without converting. Blending in linear light instead would be more
/// correct about photons and would put a colour on the middle of the lock that
/// neither end of the record asked for.
fn shade(roots: Vec3, tips: Vec3, along: f32) -> Vec3 {
    roots.lerp(tips, along.clamp(0.0, 1.0))
}

/// The cross-section of a clump at its root and its tip, as half-extents.
///
/// A helper for styles: a lock is a ribbon rather than a rope — wider across
/// than it is thick — and this is that shape from one width and one thickness.
#[must_use]
pub fn ribbon_section(width: f32, thickness: f32, taper: f32) -> (Vec2, Vec2) {
    let base = Vec2::new(width * 0.5, thickness * 0.5);
    (base, base * taper.clamp(0.0, 1.0))
}
