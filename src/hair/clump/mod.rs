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
//!   of identical (#40, #201).
//!
//! # What it costs
//!
//! [`Growth::grown`] carries a per-region count of clumps and triangles, because
//! hair has been 70% of this crate's triangle budget before and the way that
//! happened was nobody being able to say which part of it was expensive.

pub mod loft;
pub mod scatter;

use glam::{Vec2, Vec3};
use rand_pcg::Pcg64Mcg;

use super::follicle::{Follicle, Follicles};
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

pub use loft::{LEAST, ribbon_section};
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

    /// The clump's half-extents at root and tip, wide by thick, in metres.
    fn section(&self, root: &Root) -> (Vec2, Vec2);

    /// How many corners the cross-section has.
    ///
    /// Three is a prism and reads as a lock of hair at the distances a game is
    /// played from; more is rarely worth what it costs, this being the number
    /// every triangle in the mesh is multiplied by.
    fn sides(&self) -> usize {
        3
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
}

/// The head one crop of hair is grown on.
///
/// The three things every region needs and none of them owns: the built
/// surface roots are scattered over, the rig that says which part of it is a
/// head, and the regions themselves. Bundled because they travel together —
/// every caller from #202 on holds all three and passes all three.
#[derive(Clone, Copy, Debug)]
pub struct Bed<'a> {
    /// The built body, whose faces the roots are scattered over.
    pub body: &'a PolyMesh,
    /// The rig it was built against.
    pub rig: &'a Rig,
    /// Where each kind of hair may grow on it.
    pub follicles: &'a Follicles,
}

/// One region's worth of hair to grow, and what it should look like.
///
/// The record carries one of these per region once #202 lands; until then a
/// caller writes them out. Split from [`Bed`] because a head is grown five
/// times, once per region, and only this half changes between them.
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

impl Growth {
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
/// Hair grows out of skin, so a clump whose spine starts exactly on the surface
/// has half its cross-section buried — which reads as clumps sunk into the
/// scalp wherever the surface curves away. Half a millimetre is under the
/// thinnest section any style asks for.
///
/// Provenance: **derived** from the cross-sections the styles use.
pub const LIFT: f32 = 0.0005;

/// A clump that lies along the surface and falls away from it.
///
/// **The reference implementation of [`Shape`], and a real one rather than a
/// stub**: it is what a short crop, a brow and a stubbled chin all are, and the
/// styles of #204-#208 are this with their own curves.
///
/// **It leaves at a shallow angle to the skin, and the first cut of it did
/// not** (#201). Straight out along the normal is what a hair follicle looks
/// like in a diagram and what a hedgehog looks like in a render: on a head,
/// every normal is radial, so a thousand clumps drawn that way are a thousand
/// spines. The old shell system had already learned this and said so — a lock
/// "follows the skull down to the hairline before falling free" — and
/// [`Self::lie`] is that sentence as a parameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fall {
    /// How long a clump is at full mask weight, in metres.
    pub length: f32,
    /// How wide one is at the root, in metres.
    pub width: f32,
    /// How thick one is at the root, in metres — less than its width, a lock
    /// being a ribbon rather than a rope.
    pub thickness: f32,
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
            thickness: 0.003,
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
        let leaves = root.out.lerp(flow, self.lie.clamp(0.0, 1.0)).normalize_or(root.out);
        // Then increasingly toward the ground: the bend is quadratic in the
        // distance travelled, which is what a hanging thing does and what a
        // linear blend of two directions does not.
        let fall = down * (self.droop * along * along);
        let heading = (leaves + fall).normalize_or(leaves);
        root.at + root.out * LIFT + heading * (length * along)
    }

    fn section(&self, root: &Root) -> (Vec2, Vec2) {
        let thin = root.weight.clamp(0.0, 1.0).sqrt();
        let (base, tip) = ribbon_section(self.width * thin, self.thickness * thin, self.taper);
        (base, tip)
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
        follicles: Follicles,
    }

    impl Grounds {
        fn bed(&self) -> Bed<'_> {
            Bed {
                body: &self.body,
                rig: &self.rig,
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
            let roots =
                scatter::scatter(&bed.body, &bed.rig, &bed.follicles, follicle, 40, &mut stream);
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
                    if bed.body.contains(out) { "inside" } else { "outside" },
                    if bed.body.contains(inn) { "inside" } else { "outside" },
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
        // **The floor counts the CAPS, and the first cut of it did not**: a
        // swept tube is closed at both ends, so a three-station three-sided
        // clump is fourteen triangles rather than twelve. Measured, not
        // assumed — the straight run came to 280 against a predicted 240 and
        // the missing 40 was two triangles a clump.
        let sides = Fall::default().sides();
        let floor = clumps * ((LEAST - 1) * sides * 2 + 2 * (sides - 2));
        assert!(
            straight <= floor,
            "{clumps} straight clumps cost {straight} triangles against a floor of {floor}, so \
             they are being subdivided for a curve they do not have"
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
