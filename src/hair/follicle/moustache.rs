//! The upper lip, between the mouth and the nose.
//!
//! The cutaneous lip: the skin above the vermilion and below the nostrils,
//! philtrum included. Its two horizontal edges are the tightest boundaries on
//! the head — a moustache that reaches over the vermilion is a moustache in the
//! mouth, and one that reaches the nostrils is one growing out of the nose — so
//! both are read from landmarks rather than offset from a middle.

use serde::{Deserialize, Serialize};

use super::{At, Region, band};
use crate::face::Canon;

/// How the upper-lip patch is shaped on one head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// How far past the mouth's corners it grows, `-1` narrow and `+1` wide.
    #[serde(with = "crate::plan::scaled")]
    pub width: f32,
    /// How far down toward the vermilion it grows, `-1` shy of it and `+1` onto
    /// its border.
    #[serde(with = "crate::plan::scaled")]
    pub drop: f32,
}


impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.width = quantize(self.width.clamp(-1.0, 1.0));
        self.drop = quantize(self.drop.clamp(-1.0, 1.0));
    }
}

/// Where the upper lip's vermilion ends, as a share of mouth line to nose base.
///
/// Provenance: **looked up**, against the halving `examples/render`'s
/// `face_bands` uses for the same span. That convention — a band needing half
/// of a span takes the half between two landmarks — is a reasonable default
/// and slightly wrong here: an upper lip's vermilion is about 8 mm of a 19 mm
/// mouth-to-nose span, which is this rather than a half. Above it is skin that
/// grows hair; below it is lip that does not.
const VERMILION: f32 = 0.45;

/// How far that edge moves over the whole of [`Params::drop`], in the same
/// share.
///
/// Provenance: **tuned by render** (#199).
const DROP_RANGE: f32 = 0.25;

/// Half the patch's width, in [`Canon::unit`]s.
///
/// A mouth is about 50 mm across on a face of 138 mm, where one unit is 27.4 —
/// so a half-mouth is 0.91 units and a moustache stops about there.
///
/// Provenance: **derived** from [`Canon::unit`] and the mouth's own width.
const HALF: f32 = 0.95;

/// How far that moves over the whole of [`Params::width`], likewise.
///
/// Provenance: **tuned by render** (#199).
const WIDTH_RANGE: f32 = 0.30;

/// How softly the patch fades, in [`Canon::frame`]s.
///
/// Provenance: **tuned by render** (#199).
const FADE: f32 = 0.025;

/// How much of the gap to the nose the patch stops short of, in the same share.
///
/// **The ceiling is the nostril line, not [`Canon::nose_foot`], and the
/// difference is 6 mm of moustache** (#199). That landmark is where the nose's
/// RELIEF has finished — the foot of a ramp, which is what anything measuring
/// the lip's own surface wants — and it sits 6.4 mm below the base on a
/// default head. Hair does not stop there: a moustache grows to the nostrils.
/// Read against the base, the region is 10 mm tall and reaches full weight in
/// the middle; read against the foot it was 3.2 mm tall between two fades that
/// were each 2.9, so it never reached full weight anywhere and rendered as a
/// smudge under the nose.
///
/// Provenance: **derived** from the fade this region needs to clear.
const UNDER_NOSE: f32 = 0.05;

/// How far round the head the patch may reach.
///
/// In [`At::forward`]'s share. An upper lip is as far forward as a face gets,
/// so this is a high bar and exists to stop the region wrapping onto the cheek
/// on a narrow head.
///
/// Provenance: **tuned by render** (#199).
const FRONT: f32 = 0.45;

/// The upper lip, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moustache {
    /// The vermilion's top edge, in head-local metres.
    lo: f32,
    /// The nostril line, likewise.
    hi: f32,
    /// Half the patch's width, likewise.
    half: f32,
    /// The edge's width, likewise.
    fade: f32,
}

impl Moustache {
    /// Cuts the region from a measured head.
    #[must_use]
    pub(super) fn of(canon: &Canon, params: &Params) -> Self {
        let mouth = canon.mouth_line();
        let nose = canon.nose_base();
        // Down toward the mouth as `drop` rises, so `+1` reaches the vermilion.
        let vermilion = mouth + (nose - mouth) * (VERMILION - params.drop * DROP_RANGE);
        let nostrils = nose - (nose - mouth) * UNDER_NOSE;
        Self {
            lo: vermilion,
            // Ordered for the same reason the brow's ends are: the two edges
            // move on different axes and a record may cross them.
            hi: nostrils.max(vermilion + canon.frame * 0.02),
            half: canon.unit * (HALF + params.width * WIDTH_RANGE),
            fade: FADE * canon.frame,
        }
    }
}

impl Region for Moustache {
    fn weight(&self, at: &At) -> f32 {
        band(at.height, self.lo, self.hi, self.fade)
            * crate::face::smooth((self.half - at.across.abs()) / self.fade)
            * crate::face::smooth((at.forward - FRONT) / 0.30)
    }
}
