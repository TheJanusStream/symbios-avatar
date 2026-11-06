//! A bird's contour feathers (W4, #423): the small feathers that make its
//! body's outline, as cards on the body's own faces.
//!
//! **What they are** (research-10 5.1): the contour feathers lie over the
//! whole body in tracts, a few thousand on a songbird, each overlapping the
//! next one behind it, the down unseen beneath. At an avatar's budget a few
//! hundred cards stand for them: each a small feather - narrow where its quill
//! goes into the skin, broadest past the middle, rounded at the tip, as a
//! flight feather's card is (`wing::feather`) - laid along the body's flow,
//! from the beak back over the crown and down the nape, down the breast and
//! back along the back to the tail, down the legs.
//!
//! **Back over front**: a card goes in under the skin and lies along it,
//! its tip standing a little off it, so the card ahead of it - whose tip
//! reaches back over this one's quill - covers where it comes out, and the
//! body reads as overlapping feathers rather than an egg. Each tip is laid on
//! the body where it lands, and a card is bowed out over the skin where the
//! body bulges under it, so a feather lies round a breast as it does round a
//! nape.
//!
//! **Where they leave clear**: the owl's facial dish (`Disc`), whose stiff
//! feathers are relief and shading on the head (W3); round the beak and the
//! eyes, which a card would hide; the bare scaled ball and toes of the foot;
//! and the flank the folded wing lies over (`Clear`), whose own feathers
//! cover it. A card that still passes through a folded feather, or comes
//! near an eye's globe, is left out whole (`Avatar::grow_winged_cards`).
//!
//! **Coloured through one call** (`grow`'s `colour`): the bird's plumage at a
//! card's root (`PlumageSampler`), the colour of the skin it grows from. A card
//! is a shade deeper at its quill than at its tip, and each a little
//! lighter or darker than its neighbours, so feathers read one from the next.
//!
//! **Fitted to a ceiling** as fur is, the cards kept in an order that spreads
//! over the body at any share (`Thinned`): a winged body's cards share what
//! its other draws leave of its budget.

use glam::{Vec2, Vec3};
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

use super::clear::Grid;
use super::{Bed, FurGrowth, FurZone, GrownZone, Thinned, scatter_by};
use crate::face::step;
use crate::hair::mask::{StrandMask, lane_of};
use crate::mesh::PolyMesh;
use crate::plan::Zone;

/// How many cards a bird's body asks for, at most.
/// Provenance: **chosen by render** (#423): enough that every part of the
/// body is under a feather or two at the default owl, inside what a bird's
/// other draws leave of its budget.
pub(crate) const COUNT: usize = 760;

/// What one card costs: three quads and the triangle of its tip.
pub(crate) const CARD_TRIANGLES: usize = 7;

/// How long a card on the body is, as a share of the back's height, and how
/// much smaller it is on the head and down a leg, where feathers are finer.
/// Provenance: **chosen by render** (#423).
const LENGTH: f32 = 0.16;
/// See [`LENGTH`].
const HEAD_SMALLER: f32 = 0.55;
/// See [`LENGTH`].
const LEG_SMALLER: f32 = 0.45;

/// How much less often a card roots on the body than on the head, where
/// feathers are smaller and as many cover less.
/// Provenance: **chosen by render** (#423).
const BODY_SPARSER: f32 = 0.4;

/// How wide a card is at its broadest, as a share of its length: a contour
/// feather is broad and round-tipped, not a lock.
/// Provenance: **chosen by render** (#423).
const WIDTH: f32 = 0.56;

/// A card's vane across its length, quill to tip: where each row stands as a
/// share of the length, and the width there as a share of its broadest.
/// Narrow at the quill, broadest past the middle, rounded to a point at the
/// tip, which is one vertex: cut square, a card read as a plate.
const VANE: [(f32, f32); 5] = [
    (0.0, 0.40),
    (0.38, 1.0),
    (0.70, 0.90),
    (0.90, 0.55),
    (1.0, 0.0),
];

/// How many vertices a card has: two a row, and its tip.
pub(crate) const VERTICES: usize = VANE.len() * 2 - 1;

/// How far a card's tip stands off the skin it lands on, and its middle off
/// the skin under it, as shares of its length: enough that the card behind
/// shows its edge under it.
/// Provenance: **chosen by render** (#423).
const TIP_LIFT: f32 = 0.07;
/// See [`TIP_LIFT`].
const MIDDLE_LIFT: f32 = 0.03;

/// How far a card's tip may have to be carried to land on the skin, as a
/// share of its length, before the card is left out: one whose tip hangs over
/// a crease - under the chin, where the head sits on the breast - would bridge
/// it as a flap.
const LANDS: f32 = 0.35;

/// How far a card's quill goes in under the skin, as a share of its width
/// there, so its straight root is hidden.
const SINK: f32 = 0.3;

/// The share of the strand mask's length a card spans, from its root: inside
/// the lock's solid run, so a card is drawn whole (`wing::feather`'s).
const MASK_SPAN: (f32, f32) = (0.02, 0.56);

/// How deep a card's quill is shaded against its tip, and how far its tip
/// lightens. Provenance: **chosen by render** (#423).
const QUILL_SHADE: f32 = 0.86;
/// See [`QUILL_SHADE`].
const TIP_LIGHT: f32 = 0.08;

/// How far one card's shade strays from the next, either way.
const SHADE_SPREAD: f32 = 0.07;

/// How far round the dish of an owl's disc, the beak and each eye the body
/// grows no card, as shares of their own sizes: the dish's ball, the beak's
/// depth and width, the globe's radius.
/// Provenance: **chosen by render** (#423).
const DISH_CLEAR: f32 = 1.02;
/// See [`DISH_CLEAR`].
const BEAK_CLEAR: f32 = 1.5;
/// See [`DISH_CLEAR`].
const EYE_CLEAR: f32 = 1.25;

/// The salt a body's feather stream is drawn under, apart from every fur
/// zone's.
const SALT: u64 = 0x5eed_c0a7;

/// Grows a bird's contour feathers on `bed`, at most `ceiling` triangles of
/// them, on a stream drawn from `seed`, each card coloured by `colour` at its
/// root - where it is, which way the skin faces there, and the body zone.
///
/// **`colour` is the one call a plumage is asked through.**
pub(crate) fn grow(
    bed: &Bed,
    colour: &dyn Fn(Vec3, Vec3, Zone) -> Vec3,
    seed: i64,
    ceiling: usize,
) -> FurGrowth {
    let mut growth = FurGrowth::default();
    let kept = (ceiling / CARD_TRIANGLES).min(COUNT);
    if kept == 0 || bed.body.face_count() == 0 {
        return growth;
    }
    let mut stream =
        Pcg64Mcg::seed_from_u64((seed as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ SALT);
    let thinned = Thinned::new(COUNT, kept);
    let roots = scatter_by(
        bed,
        &|at, normal, zone| weight(bed, at, normal, zone),
        &thinned,
        &mut stream,
    );
    let normals = bed.body.shading_normals();
    let near = Grid::of(&bed.body.positions, LENGTH * bed.anatomy.height);
    let height = bed.anatomy.height;
    let mut clumps = 0;
    for (index, (root, zone)) in roots.iter().enumerate() {
        let (on_head, on_leg) = (on_head(bed, root.at), bed.anatomy.junctions.share(root.at));
        let way = way(bed, on_head, on_leg);
        let out = root.out;
        let along = (way - out * way.dot(out)).normalize_or(out.any_orthonormal_vector());
        let across = out.cross(along).normalize_or(Vec3::X);
        let jitter = stream.random_range(0.85..1.15);
        let length = LENGTH
            * height
            * (1.0 - HEAD_SMALLER * on_head)
            * (1.0 - LEG_SMALLER * on_leg)
            * jitter
            * root.weight.clamp(0.0, 1.0).sqrt();
        let width = length * WIDTH;
        // Laid on the body where its tip lands, standing off it.
        let on_skin = |at: Vec3, lift: f32| {
            near.nearest(at).map_or(at, |vertex| {
                let (skin, facing) = (bed.body.positions[vertex], normals[vertex]);
                at + facing * (lift - (at - skin).dot(facing))
            })
        };
        let quill = root.at - out * (SINK * width * VANE[0].1);
        let reached = root.at + along * length;
        let tip = on_skin(reached, TIP_LIFT * length);
        if tip.distance(reached) > LANDS * length {
            continue;
        }
        // Nor one that reaches over what its root keeps clear of: a card
        // rooted behind a beak or an eye may lie across it.
        if face(bed, tip) > 0.5 || face(bed, quill.lerp(tip, 0.5)) > 0.5 {
            continue;
        }
        // Bowed out over the skin where it bulges under the chord.
        let middle = quill.lerp(tip, 0.5);
        let over = on_skin(middle, MIDDLE_LIFT * length);
        let bow = (over - middle).dot(out).max(0.0);
        let shade = 1.0 + SHADE_SPREAD * stream.random_range(-1.0..1.0);
        let base = colour(root.at, out, *zone) * shade;
        let card = Card {
            quill,
            tip,
            out,
            across,
            width,
            bow,
            colours: (
                base * QUILL_SHADE,
                base.lerp(Vec3::ONE, TIP_LIGHT).min(Vec3::ONE),
            ),
        };
        card.loft(&mut growth.mesh, root.skin, index);
        clumps += 1;
    }
    if clumps > 0 {
        growth.grown.push(GrownZone {
            zone: FurZone::Contour,
            clumps,
            tris: clumps * CARD_TRIANGLES,
        });
    }
    growth
}

/// How much of a bird's body at `at`, facing along `normal` in `zone`,
/// grows contour feathers, `0..=1`: all of it but the bare foot, its face's
/// dish and round its beak and eyes, and under its folded wing.
fn weight(bed: &Bed, at: Vec3, _normal: Vec3, zone: Zone) -> f32 {
    // The ball and the toes are bare scaled skin (#422).
    if matches!(zone, Zone::Extremity(_)) {
        return 0.0;
    }
    let kept = bed.clear.map_or(1.0, |clear| clear.kept(at));
    let denser = 1.0 - BODY_SPARSER * (1.0 - on_head(bed, at));
    kept * denser * (1.0 - face(bed, at))
}

/// How much of the head at `at` is bare of cards, `0..=1`: an owl's dish,
/// and round the beak and each eye, read in the declared head's frame off the
/// parts it declares - never off whether a point is inside the head, which
/// everything sunk into it answers.
fn face(bed: &Bed, at: Vec3) -> f32 {
    let Some(head) = bed.anatomy.head.as_ref() else {
        return 0.0;
    };
    let local = head.local(at);
    let mut bare = 0.0f32;
    if let Some(disc) = &head.disc {
        // On the ball whose inside the dish is - the carved face lies on its
        // surface, the uncarved one outside it - as strongly as the disc is
        // carved, fading out over a fiftieth of its radius past it.
        let reach = disc.radius * DISH_CLEAR;
        let dish = step(reach, disc.radius, local.distance(disc.centre));
        bare = bare.max(dish * step(0.1, 0.4, disc.weight));
    }
    if let Some(beak) = &head.beak {
        // Round where it leaves the face: a deep beak's whole length cleared
        // a crow's cheeks.
        let reach = (0.5 * beak.depth + beak.half_width) * BEAK_CLEAR;
        let run = beak.forward * (beak.length * 0.15);
        let t =
            ((local - beak.base).dot(run) / run.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
        let off = local.distance(beak.base + run * t);
        bare = bare.max(step(reach, reach * 0.75, off));
    }
    bare.max(round_eyes(bed, at))
}

/// How much of the head at `at` is bare round an eye, `0..=1`: none past
/// [`EYE_CLEAR`] of the declared globe's radius, all of it inside four fifths
/// of that. A bird's contour feathers' and a bat's or a glider's fur's
/// (`scatter`), so no card on a winged body roots on an eye.
pub(super) fn round_eyes(bed: &Bed, at: Vec3) -> f32 {
    let Some(head) = bed.anatomy.head.as_ref() else {
        return 0.0;
    };
    let local = head.local(at);
    let eye = head.eye.centre.with_x(head.eye.centre.x.abs());
    let reach = head.eye.radius * EYE_CLEAR;
    let off = local.with_x(local.x.abs()).distance(eye);
    step(reach, reach * 0.8, off)
}

/// How much of the head's reach `at` is in, `0..=1`: on it, fading out a head
/// radius off it. Only how a card turns and how big it is are read by it.
fn on_head(bed: &Bed, at: Vec3) -> f32 {
    bed.anatomy.head.as_ref().map_or(0.0, |head| {
        step(1.0, 0.3, head.distance(head.local(at)) / head.radius)
    })
}

/// Which way a card lies before it is laid in the skin, `on_head` of the way
/// onto the head and `on_leg` down a leg: back from the beak over the head,
/// down and back along the body, down a leg.
fn way(bed: &Bed, on_head: f32, on_leg: f32) -> Vec3 {
    let body = (-bed.forward() * 0.6 + Vec3::NEG_Y * 0.4).normalize_or(Vec3::NEG_Y);
    // Up off a forehead, which faces the way the head does: back from the
    // beak over the crown, and back down the cheeks and the throat.
    let head = (-bed.head_forward() + Vec3::Y * 0.35).normalize_or(body);
    body.lerp(head, on_head)
        .lerp(Vec3::NEG_Y, on_leg)
        .normalize_or(body)
}

/// One contour feather, laid.
struct Card {
    /// Where its quill is, under the skin.
    quill: Vec3,
    /// Where its tip is, off the skin.
    tip: Vec3,
    /// The skin's outward facing at its root.
    out: Vec3,
    /// Across it, in the skin.
    across: Vec3,
    /// How wide it is at its broadest.
    width: f32,
    /// How far its middle is bowed out over the skin.
    bow: f32,
    /// Its colour at the quill and at the tip.
    colours: (Vec3, Vec3),
}

impl Card {
    /// Lofts this card into `mesh`, bound as the skin it grows from is,
    /// cut from a lane of the strand mask's solid run.
    fn loft(&self, mesh: &mut PolyMesh, skin: crate::mesh::VertexSkin, salt: usize) {
        let (lo, hi) = StrandMask::lane_span(lane_of(self.quill + Vec3::splat(salt as f32 * 1e-3)));
        let (lo, hi) = (lo + (hi - lo) * 0.12, hi - (hi - lo) * 0.12);
        let mut card = PolyMesh::new();
        let mut uvs = Vec::with_capacity(VERTICES);
        let mut colours = Vec::with_capacity(VERTICES);
        for &(t, wide) in &VANE {
            let centre = self.quill.lerp(self.tip, t) + self.out * (self.bow * 4.0 * t * (1.0 - t));
            let half = self.across * (0.5 * self.width * wide);
            let tone = self.colours.0.lerp(self.colours.1, t);
            let v = MASK_SPAN.0 + (MASK_SPAN.1 - MASK_SPAN.0) * t;
            // The tip is one point, in the middle of the lane.
            let sides: &[(Vec3, f32)] = if wide > 0.0 {
                &[(-half, lo), (half, hi)]
            } else {
                &[(Vec3::ZERO, 0.0)]
            };
            for &(offset, u) in sides {
                let u = if wide > 0.0 { u } else { 0.5 * (lo + hi) };
                card.push_vertex(centre + offset);
                uvs.push(Vec2::new(u, v));
                colours.push(tone);
            }
        }
        // Wound to face out of the skin: a quad a row, and the tip's triangle.
        for row in 0..VANE.len() as u32 - 2 {
            let a = row * 2;
            card.push_face(vec![a, a + 2, a + 3, a + 1]);
        }
        let a = (VANE.len() as u32 - 2) * 2;
        card.push_face(vec![a, a + 2, a + 1]);
        card.set_uvs(uvs);
        card.set_colours(colours);
        card.set_skin(vec![skin; VERTICES]);
        mesh.append(&card);
    }
}
