//! The engine every hair style compiles to.
//!
//! One mechanism, five regions, any number of styles: a style says where a
//! clump goes and what curve it follows, and this turns that into triangles
//! with a root-to-tip gradient on them. Nothing here knows what a bob is.
//!
//! # The shape of the thing
//!
//! ```text
//!   Follicles  ──►  scatter  ──►  roots  ──►  Shape::at  ──►  loft  ──►  mesh
//!   (where hair          (on the built        (the style's        (one ribbon
//!    may grow)            surface)             own curve)          per clump)
//! ```
//!
//! [`Shape`] is the seam a style plugs into. It is asked three things per clump
//! — how long, what curve, how thick — and everything else is this module's
//! business: how finely to sample, which way the ribbon's width lies, where the
//! colours go, what it all cost.
//!
//! # Two decisions worth stating
//!
//! - **Roots are scattered over the surface's own faces**, so a clump is on the
//!   head by construction rather than by a clearance pass. See [`scatter`].
//! - **A clump is sampled by how much it bends**, so a straight lock costs two
//!   stations and only a curl pays for more. Measured on the reference head,
//!   that is 87,168 triangles against 25,998 for geometry within a millimetre
//!   of identical.
//!
//! # What it costs
//!
//! [`Growth::grown`] carries a per-region count of clumps and triangles, because
//! hair can quietly become most of a triangle budget, and the way that
//! happens is nobody being able to say which part of it is expensive.

pub mod loft;
pub mod scatter;

use glam::Vec3;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;

use super::follicle::{Follicle, Follicles};
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

pub use loft::LEAST;
pub use scatter::Root;

/// What one style has to be able to say about one clump.
///
/// **The whole surface between a catalogue of styles and the machinery that
/// draws them.** A style implements this and gets sampling, the ribbon's frame,
/// the gradient and the accounting for free; the engine never learns what the
/// style is called.
///
/// All three are asked per clump rather than per region, because a clump rooted
/// in the soft edge of a hairline should be shorter and finer than one rooted in
/// the middle of the scalp — that difference is what makes an edge read as hair
/// thinning rather than as hair stopping, and it can only be said here.
pub trait Shape {
    /// How long this clump is, in metres.
    ///
    /// Zero grows nothing, which is how a style declines a root it does not
    /// want without the scatterer having to know why.
    fn length(&self, root: &Root) -> f32;

    /// A point on this clump's spine, `0` at the root and `1` at the tip, in
    /// head-local metres.
    ///
    /// Asked at whatever fractions the curve's own bending earns, so this has
    /// to be a function of the fraction rather than a list of points: a style
    /// that returns a polyline has decided its own cost, and cost is not a
    /// style's to decide. Anything smooth is cheap here and only a curl is
    /// dear, which is the right way round.
    fn at(&self, root: &Root, along: f32) -> Vec3;

    /// Half the clump's width at its root and at its tip, in metres.
    ///
    /// **A width and not a section, because a clump is one flat card.**
    /// There is no thickness to give: a swept volume spends two
    /// thirds of its triangles closing a shape nobody sees the inside of, and a
    /// lock at this budget is a sheet.
    fn width(&self, root: &Root) -> (f32, f32);

    /// Half its width a share of the way along it.
    ///
    /// **Because a lock's width comes and goes, and only the tapered case can be
    /// written as two ends.** The default runs [`Self::width`]'s two ends
    /// into each other, which is a wedge: full at the root, a point at the tip,
    /// and a blunt squared-off end where it began. That is right for a hanging
    /// lock, whose root is hidden in the hair above it, and wrong for anything
    /// that has to overlap its neighbours — a row of wedges reads as a row of
    /// objects because every one of them visibly ends.
    ///
    /// A style that overrides this can be a leaf instead: thin, full, thin. It
    /// costs nothing — the same stations, the same triangles.
    ///
    /// Asked at the same fractions [`Self::at`] is, so a width follows the curve
    /// rather than the sampling.
    fn width_at(&self, root: &Root, along: f32) -> f32 {
        let (base, tip) = self.width(root);
        base + (tip - base) * along.clamp(0.0, 1.0)
    }

    /// Which way the clump's wide axis lies, in head-local space.
    ///
    /// **A lock is a ribbon, so which way round the ribbon faces is half of
    /// whether it reads as hair** — and only the style knows, because the answer
    /// is "across the way this clump runs" and the engine does not know which way
    /// that is. The default is across a clump that falls: tangent to the head and
    /// level, which is the sheet a hanging lock lies in.
    ///
    /// A brow is what proves it has to be asked. Its clumps run sideways
    /// along the ridge, and the falling default is parallel to that — a ribbon
    /// whose width lies along its own spine has no width at all, and
    /// [`crate::prim::sweep_outline`] quietly substitutes an arbitrary frame,
    /// which is the edge-on strand the ribbon note warns about.
    ///
    /// Need not be perpendicular to the spine: the loft squares it against the
    /// path's own direction at every station. It must not be parallel to it.
    fn across(&self, root: &Root) -> Vec3 {
        root.out
            .cross(Vec3::Y)
            .normalize_or(root.out.cross(Vec3::X).normalize_or(Vec3::X))
    }
}

/// One region's hair, grown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grown {
    /// Which region it belongs to.
    pub follicle: Follicle,
    /// How many clumps were rooted.
    pub clumps: usize,
    /// What they cost, in triangles.
    pub tris: usize,
}

/// Every region's hair, in one mesh.
///
/// **One mesh for all five, because a draw call is a draw call.** The regions
/// differ in colour, and colour is per vertex here, so nothing about them needs
/// a material of its own.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Growth {
    /// The hair, in head-local space, ready to be parented to the head joint.
    pub mesh: PolyMesh,
    /// What each region contributed.
    pub grown: Vec<Grown>,
    /// The joint whose space the mesh is in, and which it binds to.
    ///
    /// Carried here rather than beside it, because a mesh in one joint's space
    /// and a joint index passed separately are two things a caller can get out
    /// of step — and the symptom is hair growing out of a shoulder.
    pub head: usize,
}

/// The head one crop of hair is grown on.
///
/// The three things every region needs and none of them owns: the built
/// surface roots are scattered over, the rig that says which part of it is a
/// head, and the regions themselves. Bundled because they travel together —
/// every caller holds all three and passes all three.
#[derive(Clone, Copy, Debug)]
pub struct Bed<'a> {
    /// The built body, whose faces the roots are scattered over.
    pub body: &'a PolyMesh,
    /// The rig it was built against.
    pub rig: &'a Rig,
    /// How that rig holds each of the body's own vertices.
    ///
    /// **Carried so that hair can be bound like the skin it grows out of**:
    /// a root takes the binding of the seat it landed on, and every
    /// vertex of its clump takes the root's. See [`scatter::Root::skin`].
    pub weights: &'a crate::rig::SkinWeights,
    /// Where each kind of hair may grow on it.
    pub follicles: &'a Follicles,
}

/// One region's worth of hair to grow, and what it should look like.
///
/// The record carries one of these per region. Split from [`Bed`] because a
/// head is grown five times, once per region, and only this half changes
/// between them.
///
/// Not `Debug`: a style is a trait object here, and requiring every style in
/// every catalogue to be printable to make one struct derivable is the tail
/// wagging the dog.
#[derive(Clone, Copy)]
pub struct Sowing<'a> {
    /// Which region to grow.
    pub follicle: Follicle,
    /// How many clumps to root in it.
    pub count: usize,
    /// What each clump becomes.
    pub shape: &'a dyn Shape,
    /// The colour at the roots, in sRGB.
    pub roots: Vec3,
    /// The colour at the tips, likewise.
    pub tips: Vec3,
}

/// What a whole head of hair may cost, in triangles.
///
/// **A hard ceiling, because a budget test alone cannot police the whole
/// catalogue.** A budget test wears one style at a time, and the styles are
/// not one cost. A tied-back tail is 42 triangles
/// a card against a crop's 15 and a ringlet is 65, because both spend their cost
/// on path and curvature rather than on count — so without this the dearest
/// legal record is
/// 32,448 triangles against a 30,000 target while every budget test passes. Two
/// and a half thousand over, on a corner nothing visits.
///
/// A ceiling is the right answer rather than smaller counts, and the difference
/// matters: every scalp style's count and width are tuned by render,
/// and cutting them to fit the corner would pay for one unreachable record
/// with every reachable one. A record that asks for maximum length AND maximum
/// thickness AND maximum density AND a tail is asking for more than the budget
/// holds, and the crate's answer to that has always been that the count is a
/// REQUEST. Everything under the corner is untouched, bit for bit.
///
/// Provenance: **derived** from the budget. The dearest body the sweep in
/// `tests/budget.rs` reaches costs 25,994 triangles without any hair on it, and
/// the WebGL2 target is 30,000, so this is what is left over with a little kept
/// back — and `the_hair_ceiling_is_what_the_budget_actually_leaves` re-measures
/// the leftover rather than trusting this docstring: a leftover-defined
/// ceiling taken on faith goes stale the moment the body's own cost moves.
///
/// **Raised from 3,200 under #116** (2026-08-18), which is that test doing the
/// job it was written for. Angle-weighted vertex normals carve a marginally
/// different surface, which puts different vertices in each refinement band, so
/// the dearest bald body fell from 26,670 to 25,994 and the leftover rose to
/// 4,006.
///
/// **The rise was evidenced rather than taken**, because a body that got
/// cheaper by losing detail would be a regression wearing a saving's clothes
/// and this constant would lock it in. What the sweep actually shows is the
/// SPREAD collapsing — seed 42 was the outlier at 10,512 faces against seed 1's
/// 10,159, and afterwards they are 10,075 and 10,153 — so a body that had been
/// refining against a tessellation artefact came back to the pack. Angle
/// weighting is invariant to how a quad is split and area weighting is not,
/// which is the mechanism. Judged by render on that same seed 42: the
/// difference is a diffuse shading shift brightest at the eye rims, nostrils,
/// lips and jawline, about a hundred pixels in a million past a channel delta
/// of 40, and no feature moved.
/// **And DOWN to 2,938 when the trunk was refined** (#285, 2026-08-19), which
/// is the same test doing the same job in the other direction — and it is the
/// largest single move this constant has made. `torso::refine_chest` gives the
/// front of the trunk two passes, and the dearest bald body in the sweep went
/// from 26,026 to 27,062, and the leftover is 2,938.
///
/// **This constant has a FLOOR as well as a definition, and finding it is what
/// decided the size of that refinement.** The leftover says what hair MAY have;
/// #209's `the_tier_bites_only_where_a_record_asks_for_more_than_the_budget_holds`
/// says what it MUST have, because nothing a re-roll can produce may ever be
/// trimmed — and the dearest re-rollable head costs about 2,750. Bisected
/// 2026-08-19: the tier test passes at 2,750 and fails at 2,278, reporting a
/// crop at density 1 on seed 0 that costs 2,732. So there were never more than
/// about 550 triangles here to spend, whatever the leftover said.
///
/// That is what refused `refine_chest` its second, tighter pass: with it in,
/// the leftover demands at most 2,278 and the floor demands at least 2,750, and
/// no number is both. One pass fits, with 188 triangles between this and the
/// floor.
///
/// **And DOWN again to 2,830 when that pass's band reached its proper floor**
/// (#292, 2026-08-19). The pass is still one; what moved is where it stops,
/// 0.05 → −0.10 of the waist-to-girdle span, which is the value #285 measured
/// and could not afford. The dearest bald body went 27,032 → 27,140 and the
/// leftover with it, 2,968 → 2,860. **80 triangles between this and the floor
/// now, where there were 188**, and that is the whole of the room left: the
/// second pass was retaken at #292 with its render in hand and declined again,
/// and anything the trunk wants next comes out of `TRIANGLE_TARGET` rather than
/// out of here.
///
/// The premise that made the whole question necessary was wrong for a while and
/// is worth recording: #283's research put the dearest hair-bearing body at
/// 27,624 and concluded there were ~2,350 triangles spare, where the real
/// binding corner is the PRODUCT of the dearest head and the greediest hair at
/// 29,078 — 922 free.
///
/// What the 11% buys is that a chest is a shape rather than a shelf, and what
/// it costs is borne ONLY at the corner this constant describes, because a
/// clump count is a REQUEST and everything under the corner is untouched bit
/// for bit; a record has to ask for maximum length AND thickness AND density
/// AND a tail to feel it, and the test above proves no re-roll does.
///
/// **Then the body got cheaper** (#310): `refine_chest` stopped refining
/// skin within a limb's own radius of its bone — the inner armpit and the
/// groin, which nothing carves — and the dearest bald body went 29,508 →
/// 28,438, leaving 3,562 against `TRIANGLE_TARGET`. The budget test's upper
/// rail fired, as it is meant to when room opens. This took the smallest
/// step that satisfies it, 2,830 → 2,850; the other ~700 triangles are the
/// owner's to spend at #307/#308, on the face, on the hair, or on nothing.
pub const MAX_TRIANGLES: usize = 2_850;

/// How many times a head of hair is regrown to get under [`MAX_TRIANGLES`].
///
/// Scaling the counts by the ratio a measurement asks for lands close but not
/// exactly, because a card's cost is its own path and not the average — so one
/// pass can still come in over. Three is what the dearest corner in the
/// catalogue needs plus one; a fourth has never changed an answer.
const TIER_PASSES: usize = 3;

/// How far under the ceiling a tier aims.
///
/// A ratio computed from a measurement lands ON the ceiling if it lands
/// perfectly, and the pass after it has nothing to correct with. Aiming a couple
/// of percent low converges from below instead of oscillating on the line.
const TIER_AIM: f32 = 0.98;

/// Grows a whole head of hair, tiered to fit [`MAX_TRIANGLES`].
///
/// **The one place the five regions are grown**, shared by `Avatar::build` and
/// by `tests/budget.rs`, which costs a catalogue without building a body forty
/// times. Two copies of a loop whose
/// whole content is "one shared stream, in `Follicle::ALL` order" would be two
/// opinions about the one thing that has to match.
///
/// Every region is grown from one stream seeded from the record's own seed, so a
/// body grows the same hair every time it is built. If the result is over the
/// ceiling, every region's count is scaled by what the measurement asks for and
/// the lot is regrown from a fresh stream — regrown rather than trimmed, because
/// dropping the last clumps of a scatter takes the hair off whichever part of the
/// head the stream happened to visit last.
#[must_use]
pub fn grow_head(bed: &Bed, sowings: &[Sowing], seed: i64, ceiling: usize) -> Growth {
    let mut grown = sow(bed, sowings, seed, 1.0);
    let mut share = 1.0;
    for _ in 0..TIER_PASSES {
        let tris = grown.tris();
        if tris <= ceiling || tris == 0 {
            break;
        }
        share *= (ceiling as f32 / tris as f32) * TIER_AIM;
        grown = sow(bed, sowings, seed, share);
    }
    grown
}

/// Grows every region once, with each region's count scaled by `share`.
fn sow(bed: &Bed, sowings: &[Sowing], seed: i64, share: f32) -> Growth {
    let mut stream = Pcg64Mcg::seed_from_u64(seed as u64);
    let mut growth = Growth::on(bed.follicles.head);
    for sowing in sowings {
        // A region that grows keeps growing, however hard the tier bites: a
        // ceiling that can shave a region out of existence is a second way of
        // saying `None` that no reader of the record would see coming.
        let count = if sowing.count == 0 {
            0
        } else {
            ((sowing.count as f32) * share).round().max(1.0) as usize
        };
        growth.grow(bed, &Sowing { count, ..*sowing }, &mut stream);
    }
    growth
}

impl Growth {
    /// An empty head of hair, to be grown in one joint's space.
    #[must_use]
    pub fn on(head: usize) -> Self {
        Self {
            head,
            ..Self::default()
        }
    }

    /// What the whole head of hair costs, in triangles.
    #[must_use]
    pub fn tris(&self) -> usize {
        self.grown.iter().map(|grown| grown.tris).sum()
    }

    /// How many clumps it is made of.
    #[must_use]
    pub fn clumps(&self) -> usize {
        self.grown.iter().map(|grown| grown.clumps).sum()
    }

    /// Grows one region into this mesh.
    ///
    /// The mask decides where the clumps land and how densely, and the sowing's
    /// own shape decides what each becomes. `stream` is drawn from the record's
    /// own seed, so a body grows the same hair every time it is built.
    pub fn grow(&mut self, bed: &Bed, sowing: &Sowing, stream: &mut Pcg64Mcg) {
        let roots = scatter::scatter(
            bed.body,
            bed.rig,
            bed.weights,
            bed.follicles,
            sowing.follicle,
            sowing.count,
            stream,
        );
        let before = self.mesh.face_count();
        let mut clumps = 0;
        for root in &roots {
            if loft::loft(
                &mut self.mesh,
                root,
                sowing.shape,
                self.head as u16,
                sowing.roots,
                sowing.tips,
            ) > 0
            {
                clumps += 1;
            }
        }
        // Counted from the mesh rather than predicted from the stations,
        // because the two have disagreed before: a sweep drops a degenerate
        // ring silently, and an accounting that trusts its own arithmetic
        // reports hair that is not there.
        let tris: usize = (before..self.mesh.face_count())
            .map(|face| self.mesh.faces[face].len().saturating_sub(2))
            .sum();
        if clumps > 0 {
            self.grown.push(Grown {
                follicle: sowing.follicle,
                clumps,
                tris,
            });
        }
    }
}

/// How far a clump stands its root off the skin, in metres.
///
/// Hair grows out of skin, so a clump lying exactly on the surface is partly
/// buried in it — which reads as hair sunk into the scalp wherever the surface
/// curves away.
///
/// **Sized by the SAMPLER, not by the geometry.** A card
/// is a polyline of chords, and a chord may sag [`loft`]'s own flatness tolerance
/// below the curve it stands for: a whole millimetre. At half of that the cards
/// crossing the crown dip under the scalp between their stations and the
/// contact sheet shows a bare star-shaped hole at the whorl — the one part of a
/// head no other card covers.
///
/// Provenance: **derived** from the loft's flatness tolerance, which is what a
/// chord can sag.
pub const LIFT: f32 = 0.0015;

/// A clump that lies along the surface and falls away from it.
///
/// **The reference implementation of [`Shape`], and a real one rather than a
/// stub**: it is what a short crop, a brow and a stubbled chin all are, and the
/// region catalogues are this with their own curves.
///
/// **It leaves at a shallow angle to the skin, and it has to.**
/// Straight out along the normal is what a hair follicle looks
/// like in a diagram and what a hedgehog looks like in a render: on a head,
/// every normal is radial, so a thousand clumps drawn that way are a thousand
/// spines. A lock follows the skull down to the hairline before falling free,
/// and [`Self::lie`] is that sentence as a parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    /// How long a clump is at full mask weight, in metres.
    pub length: f32,
    /// How wide one is at the root, in metres.
    pub width: f32,
    /// What share of that is left at the tip.
    pub taper: f32,
    /// How far the clump bends toward the ground over its length, `0` straight
    /// out and `1` hanging.
    pub droop: f32,
    /// How much the clump lies along the skin where it leaves it, `0` standing
    /// straight out and `1` flat against the surface.
    ///
    /// The direction it lies in is downhill: the ground, projected onto the
    /// skin's own tangent plane. That is what combs a scalp from the crown
    /// outward, a brow from its root along the ridge, and a beard down the jaw,
    /// without any of the three needing to know where it is.
    pub lie: f32,
}

impl Default for Fall {
    fn default() -> Self {
        Self {
            length: 0.02,
            width: 0.006,
            taper: 0.35,
            droop: 0.5,
            // Mostly lying down: hair leaves skin at a shallow angle, and the
            // render is unambiguous about what the alternative looks like.
            lie: 0.85,
        }
    }
}

impl Shape for Fall {
    fn length(&self, root: &Root) -> f32 {
        // Shorter where the mask is weaker, which is what makes a hairline
        // thin out rather than stop. Square-rooted so the edge keeps some
        // length rather than collapsing to nothing over the last tenth.
        self.length * root.weight.clamp(0.0, 1.0).sqrt()
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let length = self.length(root);
        // Downhill along the skin: the ground with the surface's own component
        // taken out of it. On the crown, where the skin faces the sky and there
        // is no downhill, it falls back to the normal — which is the one place
        // standing straight out is right.
        let down = Vec3::NEG_Y;
        let flow = (down - root.out * down.dot(root.out)).normalize_or(root.out);
        let leaves = root
            .out
            .lerp(flow, self.lie.clamp(0.0, 1.0))
            .normalize_or(root.out);
        // Then increasingly toward the ground: the bend is quadratic in the
        // distance travelled, which is what a hanging thing does and what a
        // linear blend of two directions does not.
        let fall = down * (self.droop * along * along);
        let heading = (leaves + fall).normalize_or(leaves);
        root.at + root.out * LIFT + heading * (length * along)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let thin = root.weight.clamp(0.0, 1.0).sqrt();
        let base = self.width * 0.5 * thin;
        (base, base * self.taper.clamp(0.0, 1.0))
    }
}

/// Everything a head needs before any of this can be asked of it.
///
/// A convenience for callers that hold an [`crate::Avatar`]'s parts: the body
/// mesh and rig are what [`Growth::grow`] wants, and the head's own zone is what
/// says whether there is anything to grow on.
#[must_use]
pub fn has_head(rig: &Rig) -> bool {
    !rig.in_zone(Zone::Head).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull};
    use crate::hair::follicle::FollicleParams;
    use crate::{Archetype, Avatar, AvatarRecord};
    use rand::SeedableRng;

    /// A built body and its regions, owned so a [`Bed`] can borrow them.
    struct Grounds {
        body: PolyMesh,
        rig: Rig,
        weights: crate::rig::SkinWeights,
        follicles: Follicles,
    }

    impl Grounds {
        fn bed(&self) -> Bed<'_> {
            Bed {
                body: &self.body,
                rig: &self.rig,
                weights: &self.weights,
                follicles: &self.follicles,
            }
        }
    }

    fn bed(seed: i64) -> Grounds {
        let mut record = AvatarRecord::new("Clumps", Archetype::default());
        record.reroll(seed);
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default());
        Grounds {
            body: avatar.parts.body.clone(),
            rig: avatar.rig.clone(),
            weights: avatar.parts.weights.clone(),
            follicles,
        }
    }

    fn stream() -> Pcg64Mcg {
        Pcg64Mcg::seed_from_u64(7)
    }

    /// How far either side of a root the surface is probed, in metres.
    ///
    /// Comfortably outside the sliver where a containment test on a flat face
    /// is ambiguous, and well inside the thinnest part of a head.
    const PROBE: f32 = 0.001;

    #[test]
    fn every_root_lands_on_the_surface_it_was_scattered_over() {
        // The whole reason roots are scattered over faces rather than over a
        // profile: a point inside a face is ON the head, so the floating hair
        // and sunken hair this crate has shipped twice cannot happen by
        // construction. Measured as the distance from each root to the nearest
        // point of the body, which for a point in a face is zero up to the
        // tolerance of the search.
        let bed = bed(0);
        let mut stream = stream();
        for follicle in Follicle::ALL {
            let roots = scatter::scatter(
                &bed.body,
                &bed.rig,
                &bed.weights,
                &bed.follicles,
                follicle,
                40,
                &mut stream,
            );
            assert!(
                !roots.is_empty(),
                "the {} region scattered no roots at all",
                follicle.name()
            );
            for root in &roots {
                let at = root.at + bed.follicles.origin();
                // **Asserted as sitting ON the boundary with its normal facing
                // out**, which is the property that matters and the one a
                // distance cannot state. The first cut of this measured how far
                // a root was from the nearest VERTEX and bounded it at 25 mm —
                // which is a fact about the vault's face size (they run past
                // 28 mm at the crown) and not about the root, and it failed on
                // a root that was exactly where it should be. Straddling the
                // surface catches sunken hair, floating hair and an inverted
                // normal all at once, and needs no tolerance picked by hand.
                let out = at + root.out * PROBE;
                let inn = at - root.out * PROBE;
                assert!(
                    !bed.body.contains(out) && bed.body.contains(inn),
                    "a {} root at {at:?} does not straddle the surface: a millimetre out is \
                     {} the body and a millimetre in is {} it",
                    follicle.name(),
                    if bed.body.contains(out) {
                        "inside"
                    } else {
                        "outside"
                    },
                    if bed.body.contains(inn) {
                        "inside"
                    } else {
                        "outside"
                    },
                );
                assert!(
                    (root.out.length() - 1.0).abs() < 1e-3,
                    "a {} root's normal is not unit length",
                    follicle.name()
                );
                assert!(
                    root.weight > 0.0,
                    "a {} root landed where its own mask is zero",
                    follicle.name()
                );
            }
        }
    }

    #[test]
    fn every_card_faces_out_of_the_skin_it_grew_on() {
        // **A card lit from behind is a black card, and three of the five
        // regions were** (#206). [`Shape::across`] names an AXIS, and an axis
        // has two directions; the one a style happens to write decides which way
        // the card's face points, and the scalp, the brows and the moustache
        // each wrote the one that points into the head. Measured before it was
        // fixed: 100% of every one of those three regions' vertices, on a
        // two-sided rasteriser that drew them anyway — which is why hair the
        // record said was brown rendered as black slabs, and why a brow read as
        // a dark dash however it was tuned.
        //
        // Asserted per REGION rather than over the whole head, because the
        // failure is per style and an average over five regions hides three of
        // them. And per clump against its own root's normal rather than against
        // any outward proxy, so a lock hanging past the head — where "outward"
        // means nothing — is still measured against the skin it grew from.
        let grounds = bed(0);
        let mut record = AvatarRecord::new("Facing", Archetype::default());
        record.hair.moustache.style = crate::hair::MoustacheStyle::Chevron;
        record.hair.chin.style = crate::hair::ChinStyle::Full;
        record.hair.flanks.style = crate::hair::FlankStyle::FullConnect { reach: 0.5 };
        for follicle in Follicle::ALL {
            let Some(sown) = record.hair.sowing(follicle, &grounds.follicles) else {
                panic!("{} grew nothing to face anywhere", follicle.name());
            };
            let roots = scatter::scatter(
                &grounds.body,
                &grounds.rig,
                &grounds.weights,
                &grounds.follicles,
                follicle,
                sown.clumps,
                &mut stream(),
            );
            let (mut total, mut away, mut vertices) = (0.0f32, 0usize, 0usize);
            for root in &roots {
                let mut one = PolyMesh::new();
                if loft::loft(
                    &mut one,
                    root,
                    sown.shape.as_ref(),
                    grounds.follicles.head as u16,
                    Vec3::ONE,
                    Vec3::ONE,
                ) == 0
                {
                    continue;
                }
                for normal in &one.normals {
                    let facing = normal.dot(root.out);
                    total += facing;
                    away += usize::from(facing < 0.0);
                    vertices += 1;
                }
            }
            assert!(vertices > 0, "{} lofted no cards at all", follicle.name());
            let mean = total / vertices as f32;
            let share = away as f32 / vertices as f32;
            assert!(
                mean > 0.0 && share < 0.05,
                "{}'s cards face {mean:+.2} against the skin they grew on, {:.0}% of them turned \
                 away from it",
                follicle.name(),
                share * 100.0
            );
        }
    }

    #[test]
    fn density_follows_the_mask_rather_than_the_mesh() {
        // `refine_face` splits the front of the face ten times and leaves the
        // vault at base subdivision. Scattering per vertex — the obvious way —
        // would put hundreds of scalp roots around the hairline where the face
        // begins and a handful on the whole crown. Asserted as the spread of
        // roots over the region's own height band: the top half of the scalp
        // must carry a real share of them.
        let bed = bed(0);
        let mut stream = stream();
        let roots = scatter::scatter(
            &bed.body,
            &bed.rig,
            &bed.weights,
            &bed.follicles,
            Follicle::Scalp,
            400,
            &mut stream,
        );
        let (low, high) = roots.iter().fold((f32::MAX, f32::MIN), |span, root| {
            (span.0.min(root.at.y), span.1.max(root.at.y))
        });
        let middle = (low + high) * 0.5;
        let above = roots.iter().filter(|root| root.at.y > middle).count();
        let share = above as f32 / roots.len() as f32;
        assert!(
            share > 0.20,
            "only {:.0}% of scalp roots landed in the top half of the scalp's own band, which \
             is a scatter following the refinement schedule rather than the surface",
            share * 100.0
        );
    }

    #[test]
    fn the_same_seed_grows_the_same_hair() {
        // A record has to reproduce its body exactly, and hair is the loudest
        // part of one. Two scatters from equal streams must agree vertex for
        // vertex — not approximately, since the record's whole promise is that
        // a body is a function of its seed.
        let bed = bed(3);
        let grow = || {
            let mut growth = Growth::default();
            let mut stream = stream();
            growth.grow(
                &bed.bed(),
                &Sowing {
                    follicle: Follicle::Scalp,
                    count: 50,
                    shape: &Fall::default(),
                    roots: Vec3::new(0.2, 0.1, 0.05),
                    tips: Vec3::new(0.6, 0.4, 0.2),
                },
                &mut stream,
            );
            growth
        };
        let (one, two) = (grow(), grow());
        assert_eq!(
            one.mesh.positions, two.mesh.positions,
            "the same seed grew two different heads of hair"
        );
        assert_eq!(one.tris(), two.tris(), "and they cost different amounts");
    }

    #[test]
    fn the_gradient_runs_from_the_roots_colour_to_the_tips() {
        // The owner's two-colour model, asserted where it is cheapest to break:
        // one vertex colour per station, lerped by travel. A gradient that ran
        // by height instead would reverse on any style that curls back up, and
        // nothing else in the pipeline would notice.
        let bed = bed(0);
        let mut growth = Growth::default();
        let mut stream = stream();
        let (roots_colour, tips_colour) = (Vec3::new(0.1, 0.05, 0.02), Vec3::new(0.9, 0.8, 0.5));
        growth.grow(
            &bed.bed(),
            &Sowing {
                follicle: Follicle::Scalp,
                count: 30,
                shape: &Fall::default(),
                roots: roots_colour,
                tips: tips_colour,
            },
            &mut stream,
        );
        assert_eq!(
            growth.mesh.colours.len(),
            growth.mesh.positions.len(),
            "every vertex of the hair needs a colour, or the renderer reads past the end"
        );
        let near = growth
            .mesh
            .colours
            .iter()
            .filter(|colour| colour.distance(roots_colour) < 1e-3)
            .count();
        let far = growth
            .mesh
            .colours
            .iter()
            .filter(|colour| colour.distance(tips_colour) < 1e-3)
            .count();
        assert!(
            near > 0 && far > 0,
            "the gradient reaches neither end: {near} vertices at the roots' colour and {far} \
             at the tips'"
        );
        // And nothing outside the two, which is what a lerp promises.
        for colour in &growth.mesh.colours {
            let along = (*colour - roots_colour).dot(tips_colour - roots_colour)
                / (tips_colour - roots_colour).length_squared();
            assert!(
                (-0.01..=1.01).contains(&along),
                "a hair vertex is coloured outside the two the record asked for"
            );
        }
    }

    #[test]
    fn a_clump_is_sampled_by_how_much_it_bends() {
        // The cost model, as a guard rather than a docstring. #40's lesson was
        // that a fixed station count is wrong; #201's is that sampling by
        // TRAVEL is still wrong, because it spends the same on a straight lock
        // as on a curled one — measured, 87,168 triangles against 25,998 for
        // geometry within a millimetre of identical.
        //
        // So the contract is: at one length, a straight clump is cheap and a
        // bent one pays for its bend. Asserted the way round that matters,
        // since the failure this replaces was cost scaling with the wrong
        // thing entirely.
        let bed = bed(0);
        let cost = |droop: f32| -> (usize, usize) {
            let mut growth = Growth::default();
            let mut stream = stream();
            growth.grow(
                &bed.bed(),
                &Sowing {
                    follicle: Follicle::Scalp,
                    count: 20,
                    shape: &Fall {
                        length: 0.08,
                        droop,
                        ..Fall::default()
                    },
                    roots: Vec3::ZERO,
                    tips: Vec3::ONE,
                },
                &mut stream,
            );
            (growth.tris(), growth.clumps())
        };
        let ((straight, clumps), (bent, _)) = (cost(0.0), cost(1.4));
        assert!(
            bent > straight,
            "a clump that bends cost {bent} triangles against a straight one\'s {straight}, so \
             curvature is not what is being paid for"
        );
        // And a straight one is at the floor, whatever its length: the fewest
        // stations allowed, and nothing spent subdividing a line.
        //
        // **The floor is a CARD, which is two triangles a segment and no caps**
        // (owner call, #204). It was a swept tube closed at both ends — a
        // three-station three-sided clump being fourteen triangles rather than
        // twelve, measured when a predicted 240 came to 280 — and a card of the
        // same three stations is four. That is the whole reason a scalp can afford
        // to walk a skull.
        let floor = clumps * (LEAST - 1) * 2;
        assert!(
            straight <= floor,
            "{clumps} straight clumps cost {straight} triangles against a floor of {floor}, so \
             they are being subdivided for a curve they do not have"
        );
    }

    #[test]
    fn a_section_asked_for_per_station_costs_what_a_tapered_one_does() {
        // The contract of [`Shape::section_at`] (#205): a style may be a leaf
        // rather than a wedge, and it pays nothing for it. Asserted as the
        // triangle count against the same style with the default two-ended
        // taper — the same clumps, the same stations, the same cost — and as the
        // meshes differing, since a profile that changed nothing would pass the
        // first half on its own.
        //
        // A leaf is what makes a row of overlapping clumps read as one mass: a
        // wedge ends in a blunt face at the root and the eye finds every one of
        // them.
        struct Leaf(Fall);

        impl Shape for Leaf {
            fn length(&self, root: &Root) -> f32 {
                self.0.length(root)
            }
            fn at(&self, root: &Root, along: f32) -> Vec3 {
                self.0.at(root, along)
            }
            fn width(&self, root: &Root) -> (f32, f32) {
                self.0.width(root)
            }
            fn width_at(&self, root: &Root, along: f32) -> f32 {
                let (base, _) = self.width(root);
                // Thin at both ends, fullest in the middle.
                base * (0.3 + 0.7 * (1.0 - (along * 2.0 - 1.0).abs()))
            }
        }

        let bed = bed(0);
        let grow = |shape: &dyn Shape| {
            let mut growth = Growth::default();
            let mut stream = stream();
            growth.grow(
                &bed.bed(),
                &Sowing {
                    follicle: Follicle::Brows,
                    count: 30,
                    shape,
                    roots: Vec3::ZERO,
                    tips: Vec3::ONE,
                },
                &mut stream,
            );
            growth
        };
        let (wedge, leaf) = (grow(&Fall::default()), grow(&Leaf(Fall::default())));
        assert_eq!(
            wedge.tris(),
            leaf.tris(),
            "a section asked for per station changed what the clump cost"
        );
        assert_eq!(
            wedge.mesh.positions.len(),
            leaf.mesh.positions.len(),
            "and it changed how many vertices there are"
        );
        assert_ne!(
            wedge.mesh.positions, leaf.mesh.positions,
            "the leaf profile drew the same geometry as the wedge, so nothing was asked of it"
        );
        // And the default really is the wedge: widest at the root, narrowest at
        // the tip, which is what every style that does not override this gets.
        let root = Root {
            at: Vec3::new(0.0, 0.0, 0.1),
            out: Vec3::Z,
            weight: 1.0,
            skin: Default::default(),
        };
        let fall = Fall::default();
        let (base, tip) = fall.width(&root);
        assert!((fall.width_at(&root, 0.0) - base).abs() < 1e-6);
        assert!((fall.width_at(&root, 1.0) - tip).abs() < 1e-6);
        assert!(fall.width_at(&root, 0.5) < base && fall.width_at(&root, 0.5) > tip);
    }

    #[test]
    fn a_style_may_not_lay_its_width_along_its_own_spine() {
        // What [`Shape::across`] is for, guarded at the engine rather than in one
        // style: a ribbon whose wide axis is parallel to its spine has no width,
        // and `prim::sweep_outline` does not complain — it substitutes an
        // arbitrary frame, and the clump turns edge-on. A style that names such
        // an axis must still get a ribbon.
        struct Parallel;

        impl Shape for Parallel {
            fn length(&self, _root: &Root) -> f32 {
                0.02
            }
            fn at(&self, root: &Root, along: f32) -> Vec3 {
                root.at + Vec3::X * (0.02 * along)
            }
            fn width(&self, _root: &Root) -> (f32, f32) {
                (0.002, 0.001)
            }
            fn across(&self, _root: &Root) -> Vec3 {
                // Straight along the spine, which is the one thing forbidden.
                Vec3::X
            }
        }

        let bed = bed(0);
        let mut growth = Growth::default();
        let mut stream = stream();
        growth.grow(
            &bed.bed(),
            &Sowing {
                follicle: Follicle::Brows,
                count: 12,
                shape: &Parallel,
                roots: Vec3::ZERO,
                tips: Vec3::ONE,
            },
            &mut stream,
        );
        assert!(
            growth.clumps() > 0,
            "no clump survived a degenerate wide axis"
        );
        // Measured as the clumps having real volume: a collapsed frame draws a
        // sliver, and the widest span across the spine is what says so.
        let (lo, hi) = growth
            .mesh
            .positions
            .iter()
            .fold((f32::MAX, f32::MIN), |span, at| {
                (span.0.min(at.y), span.1.max(at.y))
            });
        assert!(
            hi - lo > 0.001,
            "the clumps are {:.3} mm tall, so the ribbon collapsed edge-on",
            (hi - lo) * 1000.0
        );
    }

    #[test]
    fn what_it_costs_is_counted_from_the_mesh() {
        // An accounting that predicts its own answer reports hair that is not
        // there. Asserted against the mesh's own faces, which is the only thing
        // a renderer will actually draw.
        let bed = bed(0);
        let mut growth = Growth::default();
        let mut stream = stream();
        for follicle in Follicle::ALL {
            growth.grow(
                &bed.bed(),
                &Sowing {
                    follicle,
                    count: 25,
                    shape: &Fall::default(),
                    roots: Vec3::ZERO,
                    tips: Vec3::ONE,
                },
                &mut stream,
            );
        }
        let drawn: usize = (0..growth.mesh.face_count())
            .map(|face| growth.mesh.faces[face].len().saturating_sub(2))
            .sum();
        assert_eq!(
            growth.tris(),
            drawn,
            "the accounting and the mesh disagree about what was grown"
        );
        assert_eq!(
            growth.grown.len(),
            Follicle::ALL.len(),
            "not every region grew: {:?}",
            growth
                .grown
                .iter()
                .map(|grown| grown.follicle.name())
                .collect::<Vec<_>>()
        );
    }
}
