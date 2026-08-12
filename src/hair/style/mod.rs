//! What a record says about hair, region by region.
//!
//! Five regions, two layers, one entry each. A [`Tress`] is everything about
//! what grows in one region: which base style, how it is cut, the two colours it
//! fades between, and the paint under it.
//!
//! # Why the styles are five types and not one
//!
//! A bob is not a thing a chin can have. One `Style` enum covering every region
//! would make that representable, and every reader would then carry a match arm
//! for a case that cannot happen — which is how a match arm ends up doing
//! something arbitrary instead of nothing. Each region gets its own enum, and
//! [`Tress`] is generic over it so there is still only one shape of entry.
//!
//! # Every style is real
//!
//! There is one base style per region here besides `None`, and each is
//! implemented and rendered. The catalogues of #204-#208 add their siblings —
//! bob, curly, handlebar, goatee, braided — and each arrives with its own curve
//! rather than being declared now and mapped to something else in the meantime.
//! A variant that exists and does not do what it says is worse than one that
//! does not exist.

use serde::{Deserialize, Serialize};

use super::clump::{Fall, Shape};
use super::follicle::{Follicle, FollicleParams, Follicles};
use super::painted::Paint;

pub mod brows;
pub mod moustache;
pub mod scalp;

pub use brows::BrowStyle;
pub use moustache::MoustacheStyle;
pub use scalp::ScalpStyle;

/// How the clumps of one region are cut.
///
/// Normalised rather than metric: `0` to `1` on every axis, turned into
/// millimetres by whichever style reads them. A length in metres would mean a
/// different haircut on a child and an adult, and a record that stores metres
/// has to be migrated every time a head changes size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Cut {
    /// How long the clumps are.
    #[serde(with = "crate::plan::scaled")]
    pub length: f32,
    /// How thick each clump is, `0` fine and `1` coarse.
    #[serde(with = "crate::plan::scaled")]
    pub thickness: f32,
    /// How many of them there are.
    #[serde(with = "crate::plan::scaled")]
    pub density: f32,
    /// How far they hang, `0` standing off the head and `1` falling with the
    /// ground.
    #[serde(with = "crate::plan::scaled")]
    pub droop: f32,
}

impl Default for Cut {
    fn default() -> Self {
        Self {
            length: 0.35,
            thickness: 0.5,
            density: 0.6,
            droop: 0.5,
        }
    }
}

impl Cut {
    /// Clamps and quantises every axis. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.length = quantize(self.length.clamp(0.0, 1.0));
        self.thickness = quantize(self.thickness.clamp(0.0, 1.0));
        self.density = quantize(self.density.clamp(0.0, 1.0));
        self.droop = quantize(self.droop.clamp(0.0, 1.0));
    }
}

/// What one region grows, both layers of it.
///
/// Generic over the region's own style enum, so the five entries are one type
/// with five parameters rather than five near-identical structs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Tress<S> {
    /// Which base style, if any geometry grows here at all.
    pub style: S,
    /// How its clumps are cut.
    pub cut: Cut,
    /// The colour at the roots, in sRGB.
    #[serde(with = "crate::plan::scaled::triple")]
    pub roots: [f32; 3],
    /// The colour at the tips, likewise.
    ///
    /// **Two colours with a fade between them is the owner's own model**, and
    /// it is what makes hair read as hair at this triangle count: a single
    /// colour on a low-poly clump is a plastic wig, and the fade costs nothing
    /// because it is vertex colour.
    #[serde(with = "crate::plan::scaled::triple")]
    pub tips: [f32; 3],
    /// The hair painted into the skin of this region.
    pub skin: Paint,
}

impl<S: Style> Tress<S> {
    /// Clamps and quantises everything. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.style.sanitize();
        self.cut.sanitize();
        for channel in self.roots.iter_mut().chain(self.tips.iter_mut()) {
            *channel = quantize(channel.clamp(0.0, 1.0));
        }
        self.skin.sanitize();
    }

    /// The clump shape this tress grows, if it grows any.
    #[must_use]
    pub fn shape(&self, follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        self.style.shape(&self.cut, follicle, head)
    }

    /// How many clumps to root, given how much surface the region has.
    #[must_use]
    pub fn clumps(&self, follicle: Follicle) -> usize {
        if self.style.grows() {
            self.style.clumps(&self.cut, follicle)
        } else {
            0
        }
    }
}

/// What a region's style enum has to be able to say.
///
/// The dispatch seam between a catalogue and the engine, and the reason a new
/// style is one file and one match arm: everything else here reads a style
/// through this and never learns its name.
pub trait Style: Copy + Default {
    /// Whether this style grows any geometry at all.
    ///
    /// `None` variants answer `false`, which is how a record asks for a region
    /// that is painted and not grown — a shaved jaw, a drawn-on brow.
    fn grows(&self) -> bool;

    /// The clump shape it grows, or `None` if it grows none.
    fn shape(&self, cut: &Cut, follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>>;

    /// How many clumps it wants at this cut.
    fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize;

    /// Clamps and quantises whatever axes the style itself carries. Idempotent.
    ///
    /// **Because a variant may carry its own axis**, which is how a tail height
    /// exists without every other style having one (#204). Most styles carry
    /// none, so this does nothing by default — but a record off the network can
    /// put anything in the ones that do, and every other number in a record is
    /// clamped and snapped to the wire's precision before it is used.
    fn sanitize(&mut self) {}
}

/// How many clumps a region gets at full density.
///
/// **The single most expensive number in the hair system, and it is a budget
/// rather than a taste.** A clump costs about 14 triangles straight and up to
/// 45 curved (#201), and the whole avatar's target is 30,000 of which the body
/// already spends about 28,000 — so a full head of hair has a few thousand to
/// live in, not tens of thousands. The reference sheet at #201 used 883 scalp
/// clumps and cost 15,932 triangles for that region alone.
///
/// Ordered as [`Follicle::ALL`].
///
/// **Set by the budget test rather than by eye, and re-set three times** (#202,
/// #204). The history is the argument for what an element should be:
///
/// - 220 when a clump was a swept tube, which put the greediest record at 31,892
///   against the 30,000 target.
/// - 150 when the budget cut it, which is where the scalp read STRINGY: a hundred
///   and fifty bristles standing off a bare scalp.
/// - 44 when a clump became a wide card that walks the skull — better coverage for
///   fewer elements, and still 59 triangles each because a swept volume pays
///   `sides x 2` a segment plus two caps.
/// - These, once an element became ONE FLAT CARD (owner call): 19 triangles for the
///   same walk and 4 for a brow. The counts that the budget had been starving
///   could simply be paid for, and coverage stopped being a fight.
///
/// The sparseness these buy is still what the painted layer is for: skin the
/// colour of hair between the cards reads as hair, and bare skin between them
/// reads as balding.
///
/// Provenance: **derived** from the triangle budget and the measured cost of a
/// card.
const FULL: [usize; 5] = [104, 40, 78, 56, 80];

/// How many clumps one region grows at a given density.
///
/// Shared by every style, because how many clumps a region can afford is a
/// property of the budget and the region's own size rather than of the haircut.
fn clumps_for(cut: &Cut, follicle: Follicle) -> usize {
    let full = FULL[Follicle::ALL
        .iter()
        .position(|other| *other == follicle)
        .unwrap_or(0)];
    // A third of the count at zero density rather than none: a record asking
    // for thin hair wants thin hair, and a region that empties at one end of an
    // axis is a second way of saying `None` that nothing else reads as one.
    let share = 0.34 + 0.66 * cut.density.clamp(0.0, 1.0);
    ((full as f32) * share).round() as usize
}

/// How long a region's clumps are at full length, in metres.
///
/// Ordered as [`Follicle::ALL`]. A scalp's is what makes the difference between
/// a crop and a curtain; the rest are near enough fixed by anatomy.
///
/// Three of the five entries are now what that region would be if it fell, and
/// nothing shipped reads them: the scalp walks a measured profile (#204), the
/// brows comb along a measured ridge (#205), and the moustache runs along a
/// measured lip (#206), each taking its length from the thing it was fitted to.
/// They stay because this array is indexed by [`Follicle::ALL`]'s own order and
/// a hole in it would be a worse thing to maintain than an entry the catalogue
/// has outgrown — and because the chin and the flanks still read theirs until
/// #207 and #208.
///
/// Provenance: **tuned by render** (#202).
const REACH: [f32; 5] = [0.090, 0.012, 0.014, 0.030, 0.022];

/// The shape one region grows at a given cut.
///
/// Every style in this file is a [`Fall`] with its own numbers, which is not a
/// simplification: a crop, a brow, a moustache and a beard genuinely are the
/// same curve at different lengths. The catalogues add the ones that are not —
/// a bob's curtain, a curl's helix, a handlebar's sweep — and those bring their
/// own [`Shape`] implementations with them.
fn fall_for(cut: &Cut, follicle: Follicle) -> Fall {
    let slot = Follicle::ALL
        .iter()
        .position(|other| *other == follicle)
        .unwrap_or(0);
    // A short floor under the length so the thinnest cut still grows something
    // that reads as hair rather than as fuzz.
    let length = REACH[slot] * (0.25 + 0.75 * cut.length.clamp(0.0, 1.0));
    let width = 0.004 + 0.005 * cut.thickness.clamp(0.0, 1.0);
    Fall {
        length,
        width,
        taper: 0.35,
        droop: cut.droop.clamp(0.0, 1.0) * 1.2,
        // Hair leaves skin at a shallow angle; see [`Fall::lie`] for the render
        // that settled it.
        lie: 0.85,
    }
}

/// A natural hair colour, along a melanin ramp.
///
/// **Kept from the shell era, whose own axis it was, because the ramp itself was
/// never the problem** (#202). Dark hair is very dark — the common mistake is a
/// mid-brown that reads as dusty — and the ramp reddens through the middle
/// before it lightens, because that is the order melanin actually gives up.
///
/// What retired with the shell is the ramp being the ONLY colour a record could
/// ask for. A record now stores two sRGB colours per region and this is one
/// convenient way to pick a plausible pair, used by `reroll` and by any editor
/// preset that wants a natural head; nothing is confined to it. Its light end is
/// a warm blonde, so it cannot say grey — which is exactly why grey needed the
/// free colours rather than another point on this line (#169 left that open).
///
/// Provenance: **carried** from `HairParams::colour`, unchanged.
#[must_use]
pub fn melanin(shade: f32) -> [f32; 3] {
    const RAMP: [[f32; 3]; 5] = [
        [0.021, 0.017, 0.014],
        [0.098, 0.055, 0.036],
        [0.275, 0.148, 0.070],
        [0.545, 0.355, 0.140],
        [0.820, 0.660, 0.350],
    ];
    let along = shade.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32;
    let stop = (along.floor() as usize).min(RAMP.len() - 2);
    let blend = along - stop as f32;
    let (low, high) = (RAMP[stop], RAMP[stop + 1]);
    [
        low[0] + (high[0] - low[0]) * blend,
        low[1] + (high[1] - low[1]) * blend,
        low[2] + (high[2] - low[2]) * blend,
    ]
}

/// Declares a region's style enum, its `None` variant and its one base style.
///
/// The regions that still have exactly one style are identical in shape and
/// differ only in what they are called, which is the whole argument for writing
/// them once: a macro that expands to several enums cannot let one of them drift
/// into implementing [`Style`] differently by accident.
///
/// **It shrinks by one with each catalogue issue and is meant to.** The brows
/// left at #205, the scalp at #204 and the moustache at #206 — three regions
/// whose styles comb along something measured rather than falling downhill —
/// each into its own file beside [`brows`]. The chin and the flanks follow at
/// #207 and #208, and this macro goes with the last of them. What is left here
/// is the regions whose base style genuinely is the shared fall.
macro_rules! styles {
    ($($(#[$doc:meta])* $name:ident { $(#[$grown_doc:meta])* $grown:ident }),* $(,)?) => {
        $(
            $(#[$doc])*
            #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
            #[serde(rename_all = "snake_case")]
            pub enum $name {
                /// Nothing is grown here: the region is painted, or bare.
                #[default]
                None,
                $(#[$grown_doc])*
                $grown,
            }

            impl Style for $name {
                fn grows(&self) -> bool {
                    !matches!(self, Self::None)
                }

                fn shape(
                    &self,
                    cut: &Cut,
                    follicle: Follicle,
                    _head: &Follicles,
                ) -> Option<Box<dyn Shape>> {
                    match self {
                        Self::None => None,
                        Self::$grown => Some(Box::new(fall_for(cut, follicle))),
                    }
                }

                fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize {
                    match self {
                        Self::None => 0,
                        Self::$grown => clumps_for(cut, follicle),
                    }
                }
            }
        )*
    };
}

styles! {
    /// The base styles of the chin.
    ChinStyle {
        /// A beard over the chin and under it. #207 adds the goatee and the
        /// braided one.
        Full
    },
    /// The base styles of the jaw's flanks.
    FlankStyle {
        /// Cheek and jaw grown together. #208 adds the sideburns-only cut and
        /// the full connection to the chin.
        Full
    },
}

/// Everything a record says about one head of hair.
///
/// **Replaces the eight scalars of the shell era** (#202). Those described one
/// object — a sculpted mass with locks cut into its rim — and could not say
/// that a face has eyebrows.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HairRecord {
    /// Where each kind of hair may grow, which both layers obey.
    ///
    /// Kept beside the five rather than inside them because it is not a style:
    /// a hairline is a property of the head, and it stays where it is when
    /// somebody changes their haircut.
    pub regions: FollicleParams,
    /// The hair of the head.
    pub scalp: Tress<ScalpStyle>,
    /// The eyebrows.
    pub brows: Tress<BrowStyle>,
    /// The upper lip.
    pub moustache: Tress<MoustacheStyle>,
    /// The chin.
    pub chin: Tress<ChinStyle>,
    /// The jaw's flanks.
    pub flanks: Tress<FlankStyle>,
}

impl Default for HairRecord {
    /// A person with hair on their head and eyebrows, and no facial hair.
    ///
    /// **Not the all-`None` record the derive would give**, which is a bald
    /// mannequin — and a default that has to be edited before it looks like
    /// anybody is a default in name only. This is also what a record with no
    /// `hair` block at all reads as, so an old or partial record arrives as a
    /// person rather than as a cue ball.
    ///
    /// Facial hair stays off, because it is the one part of this a body should
    /// not be given without asking: `reroll` gates it on the composites and a
    /// creator picks it deliberately.
    fn default() -> Self {
        // Generic over the style so the two grown regions are written once; a
        // closure cannot be, since each region's style is its own type.
        fn hair<S>(style: S) -> Tress<S> {
            Tress {
                style,
                cut: Cut::default(),
                roots: melanin(0.3),
                tips: melanin(0.3),
                skin: Paint {
                    density: 0.85,
                    colour: melanin(0.3),
                },
            }
        }
        Self {
            regions: FollicleParams::default(),
            scalp: hair(ScalpStyle::Crop),
            // **A full brow rather than a shared default one** (#205). `Cut`'s own
            // default density is six tenths, which is a reasonable haircut and a
            // thin brow: measured, thirteen clumps over BOTH brows where the
            // budget allows eighteen. Nobody's brows are at 60% and the region is
            // the cheapest on the head — five clumps is seventy triangles — so the
            // default record wears the full one it can afford.
            brows: Tress {
                cut: Cut {
                    density: 1.0,
                    ..Cut::default()
                },
                ..hair(BrowStyle::Natural)
            },
            moustache: Tress::default(),
            chin: Tress::default(),
            flanks: Tress::default(),
        }
    }
}

impl HairRecord {
    /// A head with nothing grown or painted anywhere: the record's way of
    /// saying bald.
    ///
    /// Every style `None` and every paint at zero. What
    /// [`ScalpStyle::None`] and its siblings are for, in one call, because
    /// "bald" is asked for often enough — a judgement render, a helmet, a
    /// record that means it — that five assignments at each call site is how
    /// one of them gets forgotten.
    #[must_use]
    pub fn bald() -> Self {
        Self {
            regions: FollicleParams::default(),
            scalp: Tress::default(),
            brows: Tress::default(),
            moustache: Tress::default(),
            chin: Tress::default(),
            flanks: Tress::default(),
        }
    }

    /// Clamps and quantises everything. Idempotent.
    pub fn sanitize(&mut self) {
        self.regions.sanitize();
        self.scalp.sanitize();
        self.brows.sanitize();
        self.moustache.sanitize();
        self.chin.sanitize();
        self.flanks.sanitize();
    }

    /// The painted layer this record describes.
    #[must_use]
    pub fn painted(&self) -> super::painted::PaintedHair {
        super::painted::PaintedHair {
            scalp: self.scalp.skin,
            brows: self.brows.skin,
            moustache: self.moustache.skin,
            chin: self.chin.skin,
            flanks: self.flanks.skin,
        }
    }

    /// What one region grows: its shape, how many clumps, and its two colours.
    ///
    /// `None` where the region grows no geometry, which is what a painted-only
    /// region answers and what the builder skips on.
    ///
    /// Takes the head's own regions, because a style may need to be fitted to
    /// the face it grows on rather than only to the cut it was asked for: a brow
    /// combs along the ridge the mask was centred on, and #204's curtain will
    /// want the same measured head. The one every caller already holds — the
    /// masks the roots are about to be scattered through — rather than a second
    /// measurement of it.
    #[must_use]
    pub fn sowing(&self, follicle: Follicle, head: &Follicles) -> Option<Sown> {
        let (shape, clumps, roots, tips) = match follicle {
            Follicle::Scalp => (
                self.scalp.shape(follicle, head),
                self.scalp.clumps(follicle),
                self.scalp.roots,
                self.scalp.tips,
            ),
            Follicle::Brows => (
                self.brows.shape(follicle, head),
                self.brows.clumps(follicle),
                self.brows.roots,
                self.brows.tips,
            ),
            Follicle::Moustache => (
                self.moustache.shape(follicle, head),
                self.moustache.clumps(follicle),
                self.moustache.roots,
                self.moustache.tips,
            ),
            Follicle::Chin => (
                self.chin.shape(follicle, head),
                self.chin.clumps(follicle),
                self.chin.roots,
                self.chin.tips,
            ),
            Follicle::Flanks => (
                self.flanks.shape(follicle, head),
                self.flanks.clumps(follicle),
                self.flanks.roots,
                self.flanks.tips,
            ),
        };
        let shape = shape?;
        (clumps > 0).then_some(Sown {
            shape,
            clumps,
            roots,
            tips,
        })
    }
}

/// What one region of a record turns into, ready for the clump engine.
///
/// A named type rather than a tuple: four fields of which two are the same
/// `[f32; 3]` is exactly the shape a caller swaps by accident, and the symptom
/// would be a head of hair with its gradient upside down.
pub struct Sown {
    /// The curve every clump of this region follows.
    pub shape: Box<dyn Shape>,
    /// How many to root.
    pub clumps: usize,
    /// The colour at the roots, in sRGB.
    pub roots: [f32; 3],
    /// The colour at the tips, likewise.
    pub tips: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull};
    use crate::{Archetype, Avatar, AvatarRecord};

    /// The regions of one built head, which a style is fitted against.
    ///
    /// Built rather than faked, because a brow style takes its length from the
    /// ridge the mask measured and a hand-made [`Follicles`] would be a second
    /// opinion about where a brow is. The catalogue's own numbers are unit-tested
    /// in each style's file; this is what a record asks for on a real face.
    fn head() -> Follicles {
        let record = AvatarRecord::new("Styles", Archetype::default());
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default())
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut record = HairRecord {
            scalp: Tress {
                style: ScalpStyle::Crop,
                cut: Cut {
                    length: 9.0,
                    thickness: -3.0,
                    density: 0.5,
                    droop: 0.5,
                },
                roots: [2.0, -1.0, 0.5],
                tips: [0.5, 0.5, 0.5],
                skin: Paint {
                    density: 4.0,
                    colour: [0.1, 0.1, 0.1],
                },
            },
            ..HairRecord::default()
        };
        record.sanitize();
        let once = record;
        record.sanitize();
        assert_eq!(once, record, "sanitize moved a record it had already cleaned");
        assert_eq!(record.scalp.cut.length, 1.0);
        assert_eq!(record.scalp.cut.thickness, 0.0);
        assert_eq!(record.scalp.roots[0], 1.0);
        assert_eq!(record.scalp.roots[1], 0.0);
        assert_eq!(record.scalp.skin.density, 1.0);
    }

    #[test]
    fn a_region_that_grows_nothing_says_so() {
        // The `None` variants are how a record asks for a painted region and no
        // geometry — a shaved jaw, a drawn-on brow — and every reader downstream
        // keys off this rather than off a zero count, which a thin cut also has.
        let head = head();
        let bare = HairRecord::bald();
        for follicle in Follicle::ALL {
            assert!(
                bare.sowing(follicle, &head).is_none(),
                "a bald record grows {} it was never asked for",
                follicle.name()
            );
        }
        // And the default is a person rather than a mannequin: hair on the head
        // and brows, no beard.
        let usual = HairRecord::default();
        assert!(usual.sowing(Follicle::Scalp, &head).is_some());
        assert!(usual.sowing(Follicle::Brows, &head).is_some());
        assert!(usual.sowing(Follicle::Chin, &head).is_none());
    }

    #[test]
    fn a_thin_cut_still_grows_hair() {
        // A density of zero is thin hair, not no hair: `None` is how a record
        // says none, and two ways of saying it is how a reader ends up handling
        // only one of them.
        let mut record = HairRecord::default();
        record.scalp.cut.density = 0.0;
        let sown = record
            .sowing(Follicle::Scalp, &head())
            .expect("a crop at zero density is still a crop");
        assert!(sown.clumps > 0, "a thin crop grew no clumps at all");
    }

    #[test]
    fn every_region_can_be_grown_and_costs_what_it_should() {
        // The counts are a budget (see `FULL`), so they are asserted as one: a
        // whole head at full density has to stay in the low hundreds of clumps,
        // because a clump is 14 to 45 triangles and the body already spends
        // most of the avatar's own 30,000.
        let mut record = HairRecord::default();
        record.scalp.style = ScalpStyle::Crop;
        record.brows.style = BrowStyle::Natural;
        record.moustache.style = MoustacheStyle::Chevron;
        record.chin.style = ChinStyle::Full;
        record.flanks.style = FlankStyle::Full;
        for tress in [
            &mut record.scalp.cut,
            &mut record.brows.cut,
            &mut record.moustache.cut,
            &mut record.chin.cut,
            &mut record.flanks.cut,
        ] {
            tress.density = 1.0;
        }
        let head = head();
        let total: usize = Follicle::ALL
            .into_iter()
            .filter_map(|follicle| record.sowing(follicle, &head))
            .map(|sown| sown.clumps)
            .sum();
        // **The band moved twice, and the second time the CURRENCY changed.**
        // It came down at #204 because a scalp of sheets is 60 cards where a
        // scalp of strings was 150 and the sheets covered a head the strings left
        // bare: coverage came from width, which is free, rather than from count,
        // which was then 14 to 45 triangles a time.
        //
        // It goes back up at #206, and the explanation the comment above asked
        // for is that a clump is no longer 14 triangles — it is FOUR, being one
        // flat card at the sampler's floor, and up to about twenty where it
        // curves. A moustache drawn with the count a swept tube could afford is
        // twenty-five tiles as tall as the lip they sit on, which is what the
        // sheet showed; drawn with three times as many it is hair, for 228
        // triangles instead of 96. So this is stated in the triangles it is
        // really about rather than in a count whose price has changed under it,
        // and `tests/budget.rs` remains the gate that actually holds the line.
        let floor = total * crate::hair::clump::LEAST.saturating_sub(1) * 2;
        assert!(
            (120..=520).contains(&total) && floor < 2_500,
            "a full head is {total} clumps and {floor} triangles at the card's own floor, which \
             is not a budget the body leaves room for"
        );
    }
}
