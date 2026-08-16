//! Hair that is painted on rather than grown.
//!
//! The first of the two layers: hair drawn into the skin's own albedo, at the
//! density and colour each region asks for — stubble generalised, five regions
//! over the masks both layers share rather than one scalar over a hand-drawn
//! window.
//!
//! **It is not a cheaper substitute for the grown layer, it is the other half
//! of it.** Geometry gives hair a silhouette and catches light; paint gives it
//! coverage. A scalp of clumps with bare skin between them reads as balding
//! however many clumps there are, and a shaved jaw is a colour rather than a
//! shape — so most heads want both, and the two agree about where because they
//! ask the same [`super::Follicles`].
//!
//! One colour per region and no gradient: a painted hair is a hair seen
//! end-on, and the end of a hair is one colour.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::follicle::Follicle;

/// How one region's painted hair looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Paint {
    /// How much of the skin it covers, `0` bare and `1` solid.
    ///
    /// Density rather than opacity: at `1` the skin under a full mask is that
    /// colour, and between the two the grain shows through, which is what
    /// stubble is.
    #[serde(with = "crate::plan::scaled")]
    pub density: f32,
    /// What colour, in sRGB.
    ///
    /// Stored as three scaled ints like every other colour a record carries, so
    /// the wire format has no floats in it.
    #[serde(with = "crate::plan::scaled::triple")]
    pub colour: [f32; 3],
}

impl Paint {
    /// Clamps the density and the colour into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.density = quantize(self.density.clamp(0.0, 1.0));
        for channel in &mut self.colour {
            *channel = quantize(channel.clamp(0.0, 1.0));
        }
    }

    /// Whether it would paint anything at all.
    #[must_use]
    pub fn shows(&self) -> bool {
        self.density > 0.0
    }

    /// Its colour as a vector.
    #[must_use]
    pub fn tone(&self) -> Vec3 {
        Vec3::from_array(self.colour)
    }
}

/// How one region's painted hair breaks up into hairs.
///
/// **One grain cannot serve five regions, because the regions are not one kind
/// of hair.** Painted stubble is mostly skin with hairs through it, so its
/// grain has to cut nearly to bare skin at its darkest; a brow is the opposite
/// — dense hair with almost no skin between — and drawn at the stubble's grain
/// it reads as a checkerboard rather than as a brow, being three cells wide in
/// total.
///
/// So both numbers are per region: how much skin shows between the hairs, and
/// how big a hair's own cell is. Neither is a taste, and the second one has a
/// ceiling the atlas sets — see [`Grain::cells`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grain {
    /// How much skin shows through at the grain's thinnest, `0` a solid colour
    /// and `1` bare skin.
    ///
    /// Its mean is half of this, so a region at `1` reaches half its own colour
    /// on average — the strength a shave paints at.
    pub shows: f32,
    /// How close to its own colour the paint gets where the mask is full and the
    /// density is `1`.
    ///
    /// **Not all the way, and how far is the region's business.** Eight tenths
    /// is right for stubble: a shaved jaw is mostly skin with hair through it,
    /// and paint that reaches its colour exactly reads as a decal. A SCALP
    /// under a head of hair is the opposite — the skin between the locks is in
    /// shadow under hair, not showing between bristles — and at eight tenths of
    /// a dark brown from a pale complexion it renders as a tan disc at the
    /// crown that reads as a bald spot.
    pub reach: f32,
    /// How many cells of it there are per metre.
    ///
    /// **A wish rather than a promise**, because most charts carry three to five
    /// millimetres of body per texel and a cell finer than twice that cannot be
    /// drawn — the painter fades the variation by what the local spacing
    /// actually resolves, exactly as it does for pores, striations and
    /// wrinkles. Where a
    /// grain cannot be resolved the region paints its mean, which is the right
    /// answer: at four millimetres a texel you cannot draw a hair, so you draw
    /// what a patch of them averages to.
    pub cells: f32,
}

impl Grain {
    /// The grain one region's painted hair has.
    ///
    /// Provenance: the stubble regions are **carried** from the complexion's
    /// retired stubble term — bare skin at the thinnest, its 0.8 reach and its
    /// 260 cells per metre — so a beard's painted colour matches it to the bit.
    /// The brows and the scalp are **derived** from what they are (dense hair
    /// rather than a shave) and **tuned by render**.
    #[must_use]
    pub fn of(follicle: Follicle) -> Self {
        match follicle {
            // Under a crop, and what stops a thin style reading as balding. Not
            // as solid as a brow: a scalp seen between clumps genuinely does
            // show skin, and the whole reason this layer exists is that it reads
            // as hair anyway.
            Follicle::Scalp => Self {
                shows: 0.40,
                reach: 0.95,
                cells: 420.0,
            },
            // Dense, small, and the fallback a face wears when no brow is grown
            // at all — which is most of what stops a bald head reading as a
            // mannequin.
            Follicle::Brows => Self {
                shows: 0.35,
                reach: 0.90,
                cells: 700.0,
            },
            // Stubble, all three of them, at the term this replaced.
            Follicle::Moustache | Follicle::Chin | Follicle::Flanks => Self {
                shows: 1.0,
                reach: 0.8,
                cells: 260.0,
            },
        }
    }

    /// The share of its colour the paint reaches at one sample of the grain.
    ///
    /// `speckle` is the noise field at this point, `0` to `1`, and `resolved` is
    /// how much of the grain's own frequency the atlas can carry there. The mean
    /// is held whatever that is, so a region that cannot show its hairs still
    /// paints the right amount of hair.
    #[must_use]
    pub fn at(&self, speckle: f32, resolved: f32) -> f32 {
        let broken = 1.0 - self.shows * (1.0 - speckle.clamp(0.0, 1.0));
        let mean = 1.0 - self.shows * 0.5;
        mean + (broken - mean) * resolved.clamp(0.0, 1.0)
    }
}

/// The painted layer, region by region.
///
/// One entry per [`Follicle`], in the same order, so that a panel, a record and
/// this cannot disagree about which is which.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintedHair {
    /// Under the hair of the head, which is what stops a thin style reading as
    /// a bald one.
    pub scalp: Paint,
    /// The brows, which most faces want painted whether or not they are grown.
    pub brows: Paint,
    /// The upper lip.
    pub moustache: Paint,
    /// The chin.
    pub chin: Paint,
    /// The jaw's flanks.
    pub flanks: Paint,
}

impl PaintedHair {
    /// The paint for one region.
    #[must_use]
    pub fn of(&self, follicle: Follicle) -> &Paint {
        match follicle {
            Follicle::Scalp => &self.scalp,
            Follicle::Brows => &self.brows,
            Follicle::Moustache => &self.moustache,
            Follicle::Chin => &self.chin,
            Follicle::Flanks => &self.flanks,
        }
    }

    /// The paint for one region, to be changed.
    pub fn of_mut(&mut self, follicle: Follicle) -> &mut Paint {
        match follicle {
            Follicle::Scalp => &mut self.scalp,
            Follicle::Brows => &mut self.brows,
            Follicle::Moustache => &mut self.moustache,
            Follicle::Chin => &mut self.chin,
            Follicle::Flanks => &mut self.flanks,
        }
    }

    /// Clamps every region into range. Idempotent.
    pub fn sanitize(&mut self) {
        for follicle in Follicle::ALL {
            self.of_mut(follicle).sanitize();
        }
    }

    /// Whether any region would paint anything.
    ///
    /// The painter skips the whole layer on this, which matters: the masks cost
    /// a table walk per texel and an atlas is a quarter of a million of them.
    #[must_use]
    pub fn shows(&self) -> bool {
        Follicle::ALL
            .into_iter()
            .any(|follicle| self.of(follicle).shows())
    }

    /// A beard at one density in one colour, and nothing else.
    ///
    /// The three regions a beard is: the chin, the upper lip and the flanks of
    /// the jaw. A convenience because it is the commonest thing to ask for and
    /// because saying it in one place stops three call sites disagreeing about
    /// whether a moustache is part of a beard.
    #[must_use]
    pub fn beard(density: f32, colour: Vec3) -> Self {
        let paint = Paint {
            density,
            colour: colour.to_array(),
        };
        Self {
            moustache: paint,
            chin: paint,
            flanks: paint,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_is_idempotent_and_clamps() {
        let mut paint = PaintedHair {
            scalp: Paint {
                density: 4.0,
                colour: [-1.0, 0.5, 2.0],
            },
            ..PaintedHair::default()
        };
        paint.sanitize();
        let once = paint;
        paint.sanitize();
        assert_eq!(
            once, paint,
            "sanitize moved a record it had already cleaned"
        );
        assert_eq!(paint.scalp.density, 1.0);
        assert_eq!(paint.scalp.colour[0], 0.0);
        assert_eq!(paint.scalp.colour[2], 1.0);
    }

    #[test]
    fn a_grain_the_atlas_cannot_draw_paints_its_own_mean() {
        // **The bug this fixes was three cells of checkerboard on a brow**
        // (#205). The painted grain was the one field in the skin painter with no
        // resolvability fade, so where the atlas carries 3 to 5 mm of body per
        // texel it sampled a 3.8 mm noise per texel at random — and on a region
        // twelve millimetres deep the result was a chequerboard rather than hair.
        //
        // The contract is: hold the mean whatever the atlas can do, and spend the
        // variation only where it can be seen. Asserted at both ends of
        // `resolved`, over the whole range of the noise, for every region.
        for follicle in Follicle::ALL {
            let grain = Grain::of(follicle);
            let mean = 1.0 - grain.shows * 0.5;
            for speckle in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let flat = grain.at(speckle, 0.0);
                assert!(
                    (flat - mean).abs() < 1e-6,
                    "the {} grain paints {flat} where the atlas can resolve nothing, against a \
                     mean of {mean}",
                    follicle.name()
                );
            }
            // And where it can be drawn it varies, between bare skin at the
            // region's own `shows` and the full colour.
            let (dark, light) = (grain.at(0.0, 1.0), grain.at(1.0, 1.0));
            assert!(
                (dark - (1.0 - grain.shows)).abs() < 1e-6 && (light - 1.0).abs() < 1e-6,
                "the {} grain runs {dark} to {light} rather than {} to 1",
                follicle.name(),
                1.0 - grain.shows
            );
            // Anything asked outside the noise's own range is clamped rather than
            // extrapolated: a paint stronger than its own colour is not a thing.
            assert!(grain.at(2.0, 1.0) <= 1.0 && grain.at(-1.0, 1.0) >= 1.0 - grain.shows);
        }
        // The three stubble regions are the term this replaced, unchanged: bare
        // skin at the thinnest, so a mean of one half. That is what keeps a
        // beard's painted strength exactly what #200 judged.
        for follicle in [Follicle::Moustache, Follicle::Chin, Follicle::Flanks] {
            let grain = Grain::of(follicle);
            assert_eq!(
                grain.shows,
                1.0,
                "the {} is no longer a shave",
                follicle.name()
            );
            assert!((grain.at(0.5, 0.0) - 0.5).abs() < 1e-6);
        }
        // And a brow is dense hair rather than a shave, which is the whole point.
        assert!(Grain::of(Follicle::Brows).shows < Grain::of(Follicle::Chin).shows);
        assert!(Grain::of(Follicle::Brows).cells > Grain::of(Follicle::Chin).cells);
    }

    #[test]
    fn a_beard_is_three_regions_and_not_the_scalp() {
        // The one thing a helper like this can get wrong, and the reason it
        // exists rather than being written out at each call site.
        let beard = PaintedHair::beard(0.7, Vec3::new(0.2, 0.1, 0.05));
        assert!(beard.chin.shows() && beard.moustache.shows() && beard.flanks.shows());
        assert!(!beard.scalp.shows() && !beard.brows.shows());
        assert!(beard.shows());
        assert!(!PaintedHair::default().shows());
    }
}
