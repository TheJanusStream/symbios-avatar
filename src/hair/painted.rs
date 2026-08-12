//! Hair that is painted on rather than grown.
//!
//! The first of the two layers: hair drawn into the skin's own albedo, at the
//! density and colour each region asks for. It is what stubble was, generalised
//! from one scalar over a hand-drawn window to five regions over the masks both
//! layers share.
//!
//! **It is not a cheaper substitute for the grown layer, it is the other half
//! of it.** Geometry gives hair a silhouette and catches light; paint gives it
//! coverage. A scalp of clumps with bare skin between them reads as balding
//! however many clumps there are, and a shaved jaw is a colour rather than a
//! shape — so most heads want both, and the two agree about where because they
//! ask the same [`super::Follicles`].
//!
//! One colour per region and no gradient, which is the owner's own line: a
//! painted hair is a hair seen end-on, and the end of a hair is one colour.

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
        assert_eq!(once, paint, "sanitize moved a record it had already cleaned");
        assert_eq!(paint.scalp.density, 1.0);
        assert_eq!(paint.scalp.colour[0], 0.0);
        assert_eq!(paint.scalp.colour[2], 1.0);
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
