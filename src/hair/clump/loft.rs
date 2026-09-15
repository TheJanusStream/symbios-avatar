//! One clump of hair, turned into triangles.
//!
//! A clump is a spine and a width: one flat **card**, two triangles a segment.
//! Everything expensive about hair is decided here, so three things are worth
//! stating up front:
//!
//! - **A clump is ONE low-poly construction, not a volume built out of
//!   parts.** A cross-section swept down the spine — a closed tube of rings,
//!   `sides x 2` triangles a segment plus two caps — is fourteen at the floor
//!   and fifty-nine for a card that walks a skull. A card
//!   is four at the floor and twenty for the same walk, so the avatar's 30,000
//!   stops being the thing every other decision is squeezed against.
//!
//!   It is also what hair IS at this budget: a lock is a sheet, and a swept tube
//!   spent two thirds of its triangles closing a volume nobody sees the inside
//!   of. Cards are single-sided, which the contact sheet's rasteriser handles
//!   because it is two-sided by construction, and which a consuming engine
//!   handles with a double-sided hair material — the standard for cards.
//!
//! - **A clump is sampled by how much it BENDS**, not by how far it travels and
//!   not by a fixed count. A fixed count is wrong because a fringe a
//!   tenth the length of the hair behind it does not need the same number of
//!   points, and travel alone still spends fourteen stations
//!   drawing a straight line. The tolerance that replaces both is this module's
//!   own `FLATNESS`.
//! - **The gradient is vertex colour**, so a root-to-tip fade costs nothing: no
//!   texture, no second material, no atlas space. It is the whole reason the
//!   two-colour model is affordable at this triangle count.
//! - **The outline is not the geometry's** (#340). A card is a rectangle whose
//!   texture coordinates run edge to edge across one lane of the strand mask
//!   and root to tip down it, and the mask's alpha is what cuts a lock with a
//!   frayed end out of the rectangle (see [`crate::hair::mask`]): the one
//!   texture hair spends, shared by every card of every avatar.

use glam::{Vec2, Vec3};

use super::Shape;
use super::scatter::Root;
use crate::hair::mask::{self, StrandMask};
use crate::mesh::{PolyMesh, VertexSkin};

/// How far a clump's drawn spine may stray from the curve it stands for, in
/// metres.
///
/// **The one number that decides what hair costs, and it is a tolerance rather
/// than a step.** Sampling by travel alone spends the same on a straight
/// lock as on a curled one: at four millimetres a station the reference head
/// costs 87,168 triangles, of which the scalp's 883 clumps are 70,202 — and
/// most of those stations are drawing a straight line with fourteen points.
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

/// How far a turning card's width may turn between two stations, in radians,
/// where the card asks the sampler to follow it (see [`Shape::turns`]).
///
/// A quad whose two edges are more than a right angle apart is a bow-tie, and
/// one well short of it is a visibly twisted strip; a third of that keeps the
/// strip reading as one surface.
///
/// Provenance: **derived** from the bow-tie's right angle (#344).
const TURN: f32 = 0.5;

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

/// How far each edge's normal is tilted outward about the spine, in radians.
///
/// The angle the side of a cylinder makes with its top: at zero the card is
/// flat, at a right angle its edges face straight sideways and the middle of
/// the strip carries all the light. Fifty degrees reads as a rounded lock
/// without the edges going dark.
///
/// Provenance: **tuned by render** (#316).
const BEVEL: f32 = 0.87;

/// Sweeps one clump and appends it to `into`, gradient and all.
///
/// Returns how many stations it spent, which is what the caller's triangle
/// accounting is built from.
pub(super) fn loft(
    into: &mut PolyMesh,
    root: &Root,
    shape: &dyn Shape,
    head: u16,
    roots_colour: Vec3,
    tips_colour: Vec3,
) -> usize {
    let length = shape.length(root);
    if length <= f32::EPSILON {
        return 0;
    }
    let (fractions, path) = sample(root, shape);
    // A path that doubled back on itself or collapsed would sweep an inside-out
    // tube; a style that does that is a bug in the style, and the sweep is not
    // the place to discover it.
    if path.windows(2).all(|step| step[0] == step[1]) {
        return 0;
    }

    // **The wide axis is handed in rather than derived, and hair is the case
    // that proves it.** A frame derived from a near-vertical path snaps to a
    // world axis and the card turns edge-on, which is what turned a sheet of hair
    // into separate strings.
    //
    // **And it is the STYLE that hands it in, not this file** (#205). Across the
    // fall is what a hanging lock's width lies along, and it is
    // [`Shape::across`]'s default; a brow's clumps run sideways and want their
    // width across that instead. The one thing a style may not do is name an axis
    // parallel to its own spine, so the parallel case is caught here rather than
    // silently substituted.
    //
    // Squared against the LOCAL tangent at every station rather than transported
    // from the first: a card whose width stayed in one plane would twist wherever
    // its spine turned, and re-squaring costs a dot product. It also cannot drift,
    // which a transported frame can over a curl.
    let first = into.positions.len() as u32;
    // **One lane of the strand mask for the whole card, picked by a hash of its
    // root** (#340): cards side by side end in different locks, and a card that
    // changed lane along its length would tear its own silhouette in two.
    let (lane_from, lane_to) = StrandMask::lane_span(mask::lane_of(root.at));
    // Whether this card is seamed where it turns over, which way round the
    // last station faced, and whether the card has turned over an odd number
    // of times (see [`Shape::seamed`]).
    let seamed = shape.seamed();
    let mut was: Option<bool> = None;
    let mut over = false;
    for (station, (at, along)) in path.iter().zip(&fractions).enumerate() {
        let tangent = if station + 1 < path.len() {
            path[station + 1] - *at
        } else {
            *at - path[station - 1]
        }
        .normalize_or(root.out);
        // Asked per station, because a clump that turns has a different
        // across at its end than at its root; see [`Shape::across_at`].
        let named = shape.across_at(root, *along).normalize_or(Vec3::ZERO);
        let squared = named - tangent * named.dot(tangent);
        let mut side = squared.normalize_or(
            root.out
                .cross(tangent)
                .normalize_or(tangent.cross(Vec3::Y).normalize_or(Vec3::X)),
        );
        let half = shape.width_at(root, *along).max(0.0);
        // The card's own face, which is what catches the light: perpendicular to
        // both the spine and the width.
        let mut out = side.cross(tangent).normalize_or(root.out);
        // **And turned to face the way the skin does, here rather than in the
        // style** (#206). [`Shape::across`] names an AXIS — which way the width
        // lies — and an axis has two directions; which of them a style happens
        // to write decides which way the card's face ends up pointing, and every
        // one of the three catalogues that overrides it wrote the one that
        // points INTO the head. Measured on a built body: 100% of the scalp's
        // cards, 100% of the brows' and 100% of the moustache's, lit from behind
        // on a two-sided rasteriser, which is why hair that should have been
        // brown rendered as black slabs and why a brow read as a dark dash
        // however it was tuned.
        //
        // A style cannot reasonably be asked to get this right — it is a
        // cross-product's handedness, not a fact about hair — so it is not asked.
        // The width axis is flipped with the face so the quad's winding still
        // agrees with its normal.
        let flipped = out.dot(root.out) < 0.0;
        if flipped {
            out = -out;
            side = -side;
        }
        // The gradient, by travel rather than by height: hair that falls and then
        // curls back up is still older at its tip, and a colour taken from height
        // would run backwards over the curl. And by how far along the curve a
        // station is, not by its index (#205) — adaptive sampling puts stations
        // where a clump bends, so an index is not a share of the way down a hair.
        // And the style's own say about this station's light (#339): a card a
        // shade apart from its neighbour, a stretch of it lying in the others'
        // shadow. Clamped, because a factor over one on a pale colour is a
        // channel past white.
        let shade = (shade(roots_colour, tips_colour, *along) * shape.shade_at(root, *along))
            .clamp(Vec3::ZERO, Vec3::ONE);
        // **Seamed where a turning card turns over** (#343): the station is
        // drawn first the way the last one faced, so the quad from it has no
        // crossed edges, and then this way, which leaves a quad of no area
        // between the two. The mask's lane runs across the card by where an
        // edge IS rather than by which way round it was drawn, so a strand
        // does not jump sides at the seam.
        let turns = seamed && was.is_some_and(|facing| facing != flipped);
        was = Some(flipped);
        let drawn: &[(Vec3, Vec3, bool)] = if turns {
            &[(-side, -out, over), (side, out, !over)]
        } else {
            &[(side, out, over)]
        };
        over ^= turns;
        for &(side, out, over) in drawn {
            let lane = if over { -1.0f32 } else { 1.0 };
            for edge in [-1.0f32, 1.0] {
                into.positions.push(*at + side * (half * edge));
                // **The normal is rounded across the card though the card is
                // flat** (#316). A flat card lit flat is a ribbon: one shade
                // edge to edge, and a head of them reads as a bundle of dark
                // straps with no body in it. Tilting each edge's normal outward
                // about the spine shades the strip as the half-cylinder a lock
                // is, and costs nothing — the rasteriser interpolates it.
                into.normals
                    .push((out * BEVEL.cos() + side * (BEVEL.sin() * edge)).normalize_or(out));
                // Across the card's lane of the strand mask edge to edge, and down
                // it by the same share of the way the width and the shade are
                // asked at, so the lock the mask cuts tapers where the card does.
                into.uvs.push(Vec2::new(
                    lane_from + (lane_to - lane_from) * (edge * lane + 1.0) * 0.5,
                    along.clamp(0.0, 1.0),
                ));
                into.colours.push(shade);
                // **Bound like the skin it grew out of at the root, and like the
                // head at the tip** (#207). The whole crop used to bind rigidly to
                // the head joint, which is right for a scalp and wrong for a face:
                // with the jaw open, the chin's own skin moves 44.7 mm and hair on
                // the head moves nothing at all, so a beard stays where the closed
                // mouth was.
                //
                // And the other way round is wrong too. Bound entirely by its root,
                // a beard is rigid to the mandible — which swings DOWN AND BACK
                // about the condyle, so the hanging part of it goes into the neck as
                // soon as somebody speaks. What a beard does instead is hang: the
                // hair leaves the chin and then belongs to nothing in particular,
                // which is what this says. The patch owns the root and lets go over
                // the free length.
                //
                // It is a no-op wherever the skin is already the head's, which is
                // the scalp, the brows and the upper lip — three of the five regions
                // move not at all.
                into.skin.push(handed_over(root.skin, head, *along));
            }
        }
    }
    // One quad a segment, which the mesh counts as the two triangles it is.
    let stations = (into.positions.len() - first as usize) / 2;
    for segment in 0..stations.saturating_sub(1) {
        let step = first + segment as u32 * 2;
        into.faces.push(vec![step, step + 1, step + 3, step + 2]);
    }
    stations
}

/// How many segments a lump has round its middle.
///
/// Provenance: **derived** from the triangle count #342 allows a knot: eight
/// round and three rings between the poles is forty-eight triangles.
const LUMP_SEGMENTS: usize = 8;

/// How many rings of vertices a lump has between its two poles.
///
/// Provenance: **derived**, with [`LUMP_SEGMENTS`].
const LUMP_RINGS: usize = 3;

/// Which row of the strand mask a lump's texture coordinates sit on.
///
/// Inside the part of every lane the mask keeps whole, so a cut-out material
/// draws the lump solid (see [`crate::hair::mask`]).
///
/// Provenance: **derived** from the mask, whose lanes are whole to 60%.
const LUMP_ROW: f32 = 0.25;

/// Draws one lump and appends it to `into`: a closed ellipsoid, bound to the
/// head, in `colour` darkened by the lump's own shade.
///
/// Returns how many triangles it spent.
pub(super) fn lump(into: &mut PolyMesh, lump: &super::Lump, head: u16, colour: Vec3) -> usize {
    use std::f32::consts::{PI, TAU};
    let first = into.positions.len() as u32;
    let (lane_from, lane_to) = StrandMask::lane_span(0);
    let uv = Vec2::new((lane_from + lane_to) * 0.5, LUMP_ROW);
    let mut skin = VertexSkin::default();
    skin[0] = crate::rig::Influence {
        joint: head,
        weight: 1.0,
    };
    let shade = (colour * lump.shade).clamp(Vec3::ZERO, Vec3::ONE);
    let radii = lump.radii.max(Vec3::splat(f32::EPSILON));
    let mut push = |unit: Vec3| {
        into.positions.push(lump.centre + unit * radii);
        // The ellipsoid's own gradient, which is the unit sphere's normal
        // squashed the other way.
        into.normals.push((unit / radii).normalize_or(unit));
        into.uvs.push(uv);
        into.colours.push(shade);
        into.skin.push(skin);
    };
    push(Vec3::Y);
    for ring in 1..=LUMP_RINGS {
        let polar = PI * ring as f32 / (LUMP_RINGS + 1) as f32;
        for segment in 0..LUMP_SEGMENTS {
            let turn = TAU * segment as f32 / LUMP_SEGMENTS as f32;
            push(Vec3::new(
                polar.sin() * turn.sin(),
                polar.cos(),
                polar.sin() * turn.cos(),
            ));
        }
    }
    push(Vec3::NEG_Y);
    let at = |ring: usize, segment: usize| {
        first + 1 + (ring * LUMP_SEGMENTS + segment % LUMP_SEGMENTS) as u32
    };
    let bottom = first + 1 + (LUMP_RINGS * LUMP_SEGMENTS) as u32;
    let before = into.faces.len();
    // Wound so each face turns out of the lump: down the meridian, then round.
    for segment in 0..LUMP_SEGMENTS {
        into.faces
            .push(vec![first, at(0, segment), at(0, segment + 1)]);
        for ring in 0..LUMP_RINGS - 1 {
            into.faces.push(vec![
                at(ring, segment),
                at(ring + 1, segment),
                at(ring + 1, segment + 1),
                at(ring, segment + 1),
            ]);
        }
        into.faces.push(vec![
            at(LUMP_RINGS - 1, segment),
            bottom,
            at(LUMP_RINGS - 1, segment + 1),
        ]);
    }
    into.faces[before..].iter().map(|face| face.len() - 2).sum()
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
    let turns = shape.turns();
    // How far the width turns across a gap, as a stray the loop below weighs
    // like any other: nothing under [`TURN`], and past it more than
    // [`FLATNESS`] by as much as the turn is past it (see [`Shape::turns`]).
    let turned = |from: f32, to: f32| {
        let one = shape.across_at(root, from).normalize_or(Vec3::ZERO);
        let two = shape.across_at(root, to).normalize_or(Vec3::ZERO);
        let angle = one.dot(two).clamp(-1.0, 1.0).acos();
        if angle > TURN {
            FLATNESS * angle / TURN
        } else {
            0.0
        }
    };
    while points.len() < MOST {
        // The gap whose midpoint is furthest off the chord that spans it.
        let mut worst = (0.0f32, 0usize);
        for gap in 0..points.len() - 1 {
            let middle = (fractions[gap] + fractions[gap + 1]) * 0.5;
            let mut strays = shape
                .at(root, middle)
                .distance(points[gap].lerp(points[gap + 1], 0.5));
            if turns {
                strays = strays.max(turned(fractions[gap], fractions[gap + 1]));
            }
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

/// One clump's binding a share of the way along it.
///
/// The root's own binding, handed over to `head` as the clump leaves the skin.
/// See the call site for why a hanging clump stops belonging to the patch it
/// grew on.
///
/// **The hand-over is a REPLACEMENT and not an average of two influence lists.**
/// Those are sorted by strength and index different joints, so entry `k` of one
/// is a different bone from entry `k` of the other and averaging them entry by
/// entry produces a binding nobody wrote. This scales what is there and
/// adds the head's share to the head's own entry, which is the same arithmetic
/// a skinning weight has everywhere else.
fn handed_over(mut skin: VertexSkin, head: u16, along: f32) -> VertexSkin {
    let over = crate::face::smooth(along.clamp(0.0, 1.0));
    if over <= 0.0 {
        return skin;
    }
    if let Some(mine) = skin.iter_mut().find(|influence| influence.joint == head) {
        // Already there: the hand-over is a move along the one axis that
        // separates the patch's hold from the head's.
        let held = mine.weight;
        for influence in &mut skin {
            influence.weight *= 1.0 - over;
        }
        if let Some(mine) = skin.iter_mut().find(|influence| influence.joint == head) {
            mine.weight = held * (1.0 - over) + over;
        }
        return normalised(skin);
    }
    // The head is not in the list, so it takes the weakest entry's place: at
    // most four joints may hold a vertex, and the one being handed over to is by
    // construction stronger than whatever is being dropped.
    let weakest = (0..skin.len())
        .min_by(|one, two| skin[*one].weight.total_cmp(&skin[*two].weight))
        .unwrap_or(0);
    for influence in &mut skin {
        influence.weight *= 1.0 - over;
    }
    skin[weakest] = crate::rig::Influence {
        joint: head,
        weight: over,
    };
    normalised(skin)
}

/// The same binding, summing to one.
fn normalised(mut skin: VertexSkin) -> VertexSkin {
    let total: f32 = skin.iter().map(|influence| influence.weight).sum();
    if total > f32::EPSILON {
        for influence in &mut skin {
            influence.weight /= total;
        }
    }
    skin
}

/// The colour a share of the way from root to tip.
///
/// **Interpolated in the space the colours are stored in, which is sRGB**
/// (the convention `PolyMesh::colours` documents), because
/// these two came off a record where a person picked them and a renderer draws
/// them without converting. Blending in linear light instead would be more
/// correct about photons and would put a colour on the middle of the lock that
/// neither end of the record asked for.
fn shade(roots: Vec3, tips: Vec3, along: f32) -> Vec3 {
    roots.lerp(tips, along.clamp(0.0, 1.0))
}
