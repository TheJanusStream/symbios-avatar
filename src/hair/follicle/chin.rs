//! The chin, and the plane under it.
//!
//! The mental region: below the lower lip, over the chin's own pad, and round
//! under it onto the submental plane the neck work shaped (#193). It reaches
//! under rather than stopping at the menton because a chin beard hangs — the
//! part of it a profile view reads is the part below the jaw, and a region that
//! stops at the front of the chin can only grow a painted-on one.

use serde::{Deserialize, Serialize};

use super::{At, Region, band};
use crate::face::{Canon, Skull};

/// How the chin patch is shaped on one head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// How wide the patch is, `-1` a narrow tuft and `+1` the whole chin.
    #[serde(with = "crate::plan::scaled")]
    pub width: f32,
    /// How far under the jaw it reaches, `-1` the chin's front only and `+1`
    /// well back along the submental plane.
    #[serde(with = "crate::plan::scaled")]
    pub under: f32,
    /// How far up toward the lower lip it grows, `-1` low on the chin and `+1`
    /// to the lip's own border.
    #[serde(with = "crate::plan::scaled")]
    pub rise: f32,
}


impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.width = quantize(self.width.clamp(-1.0, 1.0));
        self.under = quantize(self.under.clamp(-1.0, 1.0));
        self.rise = quantize(self.rise.clamp(-1.0, 1.0));
    }
}

/// Where the lower lip's vermilion ends, as a share of chin to mouth line.
///
/// Provenance: **derived**, the same halving as the upper lip's and from the
/// same convention: the lower lip band runs from half way down to the chin up
/// to the mouth line, so its foot is the ceiling of the skin below it.
const VERMILION: f32 = 0.5;

/// How far that ceiling moves over the whole of [`Params::rise`], likewise.
///
/// Provenance: **tuned by render** (#199).
const RISE_RANGE: f32 = 0.28;

/// How far under the menton the patch reaches, in [`Canon::frame`]s.
///
/// Provenance: **derived** from the head's own span rather than picked: the
/// surface runs about 28 mm past the chin before the neck owns it, on a frame
/// of about 100 mm, and a beard that reaches most of that way is on the
/// submental plane without being on the throat.
const UNDER: f32 = 0.20;

/// How far that reach moves over the whole of [`Params::under`], likewise.
///
/// Provenance: **tuned by render** (#199).
const UNDER_RANGE: f32 = 0.12;

/// Half the patch's width, in [`Canon::unit`]s.
///
/// A chin pad is a little wider than the mouth above it, which is where this
/// sits: the moustache's own half-width is 0.95 units.
///
/// Provenance: **derived** from [`Canon::unit`] and the mouth's width.
const HALF: f32 = 1.25;

/// How far that moves over the whole of [`Params::width`], likewise.
///
/// Provenance: **tuned by render** (#199).
const WIDTH_RANGE: f32 = 0.45;

/// How softly the patch fades, in frames.
///
/// Provenance: **tuned by render** (#199).
const FADE: f32 = 0.045;

/// How far round the head the patch may reach.
///
/// In [`At::forward`]'s share, and low, because most of this region is UNDER
/// the chin where the surface has turned to face the ground and the share falls
/// away. Its job is only to keep the region off the back of the neck.
///
/// Provenance: **tuned by render** (#199).
const FRONT: f32 = -0.35;

/// The patch of chin a beard grows on, as one object.
///
/// **Handed out to the styles, the same discipline as
/// [`Ridge`](super::brows::Ridge) and [`Lip`](super::moustache::Lip)** (#207).
/// A beard's shape is these numbers: how high it climbs is the lower lip, how
/// far back under the jaw it reaches is the submental edge, and where it hangs
/// FROM is the menton — which no mask needs and every chin style does, because
/// this is the one region whose hair leaves the head entirely.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pad {
    /// The lower lip's foot, in head-local metres: the ceiling.
    pub lip: f32,
    /// How far under the menton the patch reaches, likewise: the back edge.
    pub under: f32,
    /// The chin's own tip, likewise.
    ///
    /// **Not the middle of the patch and not either of its edges.** It is where
    /// the face stops and the throat begins, so it is what a hanging beard
    /// hangs from and what says which side of the neck it has to fall on.
    pub menton: f32,
    /// How far forward the chin reaches at that height, likewise.
    pub front: f32,
    /// Half the patch's width, likewise.
    pub half: f32,
    /// The edge's width, likewise.
    pub fade: f32,
}

impl Pad {
    /// How deep the patch is, from the submental edge to the lip.
    ///
    /// Floored for the reason every span in this module is: a record may put
    /// the two edges together and callers divide by it.
    #[must_use]
    pub fn span(&self) -> f32 {
        (self.lip - self.under).max(super::MINIMUM_SPAN)
    }

    /// How far out along the patch a point sits: `0` on the midline, `1` at the
    /// outer edge.
    #[must_use]
    pub fn along(&self, across: f32) -> f32 {
        across.abs() / self.half.max(super::MINIMUM_SPAN)
    }

    /// Where a beard grown on this hangs from, in head-local metres.
    ///
    /// Under the chin's own tip: hair falling off a jaw gathers to the front of
    /// it, which is why a beard comes to a point below the menton and not below
    /// the throat. **It is also the whole of the throat clearance** — a clump
    /// pulled toward this line as it falls is a clump moving away from the neck,
    /// whichever part of the submental plane it started on.
    #[must_use]
    pub fn hangs_from(&self) -> glam::Vec3 {
        glam::Vec3::new(0.0, self.menton, self.front)
    }
}

/// The chin, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chin {
    /// The patch, which the styles grow on and this masks.
    pad: Pad,
}

impl Chin {
    /// Cuts the region from a measured head.
    #[must_use]
    pub(super) fn of(skull: &Skull, canon: &Canon, params: &Params) -> Self {
        let chin = skull.chin();
        let mouth = canon.mouth_line();
        // Up toward the lip as `rise` rises.
        let lip = chin + (mouth - chin) * (VERMILION + params.rise * RISE_RANGE);
        Self {
            pad: Pad {
                lip,
                under: chin - (UNDER + params.under * UNDER_RANGE) * canon.frame,
                menton: chin,
                front: skull.surface_at(chin, 0.0).z,
                half: canon.unit * (HALF + params.width * WIDTH_RANGE),
                fade: FADE * canon.frame,
            },
        }
    }

    /// The patch this mask is cut around.
    #[must_use]
    pub(super) fn pad(&self) -> Pad {
        self.pad
    }
}

impl Region for Chin {
    fn weight(&self, at: &At) -> f32 {
        // The width is taken in metres from the midline rather than as a share
        // of the skull's half-width, because under the chin that half-width is
        // the JAW's and the patch would spread to the gonions with it.
        band(at.height, self.pad.under, self.pad.lip, self.pad.fade)
            * crate::face::smooth((self.pad.half - at.across.abs()) / self.pad.fade)
            * crate::face::smooth((at.forward - FRONT) / 0.30)
    }
}
