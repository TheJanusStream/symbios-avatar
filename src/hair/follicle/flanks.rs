//! The flanks of the jaw: sideburn, cheek and the jawline itself.
//!
//! The largest of the four facial regions and the only one whose lower edge is
//! shared with another file. A beard ends where the mandible does, and the
//! mandible's lower border is [`crate::face::skull`]'s own carved line — so this
//! region reads that line rather than drawing a second one beside it. #196 is
//! what the second copy cost.

use serde::{Deserialize, Serialize};

use super::{At, Follicles, Region, band};
use crate::face::{Canon, Skull};

/// How the flank patch is shaped on one head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// How high up the cheek it grows, `-1` a low beard line and `+1` high on
    /// the cheekbone.
    #[serde(with = "crate::plan::scaled")]
    pub cheek: f32,
    /// How far below the jawline it rides, `-1` stopping on the border and `+1`
    /// well down the neck.
    #[serde(with = "crate::plan::scaled")]
    pub under: f32,
    /// How far forward of the ear the sideburn begins, `-1` well back and `+1`
    /// onto the cheek.
    #[serde(with = "crate::plan::scaled")]
    pub sideburn: f32,
}


impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.cheek = quantize(self.cheek.clamp(-1.0, 1.0));
        self.under = quantize(self.under.clamp(-1.0, 1.0));
        self.sideburn = quantize(self.sideburn.clamp(-1.0, 1.0));
    }
}

/// Where the beard line sits in front of the ear, in [`Canon::frame`]s above
/// the ear's centre.
///
/// The sideburn's top, which is the one part of this boundary a person picks
/// deliberately. Level with the ear's upper third on a default face.
///
/// Provenance: **tuned by render** (#199).
const SIDEBURN: f32 = 0.08;

/// How far the whole cheek line moves over [`Params::cheek`], in frames.
///
/// Provenance: **tuned by render** (#199).
const CHEEK_RANGE: f32 = 0.16;

/// How far below the mandible's border the beard reaches, in frames.
///
/// **Below it, not to it, and that is the anatomy rather than a soft edge.**
/// Beard growth crosses the jawline and stops on the upper neck; a region that
/// ended at the border would leave the crease itself bald and draw a bright
/// line down the one edge #195 spent a day getting smooth.
///
/// Provenance: **tuned by render** (#199).
const UNDER: f32 = 0.16;

/// How far that reach moves over the whole of [`Params::under`], likewise.
///
/// Provenance: **tuned by render** (#199).
const UNDER_RANGE: f32 = 0.12;

/// How softly the patch fades, in frames.
///
/// The widest fade of the four facial regions, and it has to be: this one's
/// upper edge is a DIAGONAL — it drops from the sideburn to the cheek across
/// the flank — and a boundary that moves as it is crossed is as steep as the
/// sum of its own fade and its movement. At the 0.05 the other regions use,
/// `follicleaudit` read the beard line as a 1.7 mm edge against a 5.2 mm fade.
///
/// Provenance: **derived** from that measurement, and judged by render (#199).
const FADE: f32 = 0.07;

/// Where the region gives way to the chin's, as a share of the skull's own
/// half-width.
///
/// **The seam between the two beard regions, and it is a share rather than a
/// distance because that is what makes it hold.** The chin's patch is placed in
/// eye-widths from the midline and this one in shares of the head's breadth; on
/// a broad head the two would part and leave a bald stripe down the jaw if this
/// were a metre figure.
///
/// **It comes on faster than it used to, and the render is what said so**
/// (#199). At a ramp reaching full only past 0.72 of the half-width, the
/// underside of the jaw was left at about a quarter weight: a point below the
/// border sits at a height where the skull's half-width is already the neck's,
/// so it never reaches a high lateral share however far out it is. The overlay
/// drew that as a pale wedge between this region and the chin's, and it was
/// the mask rather than the instrument — the first suspicion, that the two
/// regions were each half-on and the overlay was hiding their sum, was checked
/// by changing the overlay to show coverage and the wedge did not move.
///
/// Provenance: **tuned by render** (#199), against the seam it has to close.
const INNER: f32 = 0.35;

/// How quickly it comes on over that share.
///
/// Provenance: **tuned by render** (#199).
const INNER_RAMP: f32 = 0.20;

/// How far behind the ear the region reaches, as a cosine of the azimuth.
///
/// Negative because the sideburn's own root is already behind the widest part
/// of the head. Past this is the nape, which is the scalp's business.
///
/// Provenance: **tuned by render** (#199).
const BEHIND: f32 = -0.35;

/// How far forward of the ear the sideburn's back edge moves over the whole of
/// [`Params::sideburn`], in the same cosine.
///
/// Provenance: **tuned by render** (#199).
const SIDEBURN_RANGE: f32 = 0.25;

/// How far round from the side the beard line has finished dropping to the
/// cheek, as a cosine of the azimuth.
///
/// About 45°, so the line runs diagonally down the flank and is level again by
/// the time it reaches the chin's own patch.
///
/// Provenance: **tuned by render** (#199).
const DIAGONAL: f32 = 0.70;

/// The jaw flanks, cut from one head's landmarks.
#[derive(Clone, Debug, PartialEq)]
pub struct Flanks {
    /// The beard line in front of the ear, in head-local metres.
    sideburn: f32,
    /// The beard line at the front of the cheek, likewise.
    cheek: f32,
    /// How far below the mandible's border the patch reaches, likewise.
    under: f32,
    /// The edge's width, likewise.
    fade: f32,
    /// How far back the patch reaches, as a cosine of the azimuth.
    behind: f32,
    /// The skull the border is read from, which is a function of azimuth and so
    /// cannot be resolved until a point arrives.
    skull: Skull,
}

impl Flanks {
    /// Cuts the region from a measured head.
    #[must_use]
    pub(super) fn of(skull: &Skull, canon: &Canon, params: &Params) -> Self {
        let frame = canon.frame;
        let lift = params.cheek * CHEEK_RANGE * frame;
        Self {
            sideburn: canon.ear_centre() + SIDEBURN * frame + lift,
            // Forward of the ear the beard line drops to about the mouth: that
            // is where a shaved cheek line sits on a face, and it is a landmark
            // rather than a fraction so the two ends of this line cannot drift
            // apart on a long face.
            cheek: canon.mouth_line() + lift,
            under: (UNDER + params.under * UNDER_RANGE) * frame,
            fade: FADE * frame,
            behind: BEHIND + params.sideburn * SIDEBURN_RANGE,
            skull: skull.clone(),
        }
    }

    /// The beard line at one point's own distance round the head, in metres.
    ///
    /// Carried between the sideburn and the cheek by how far round the point
    /// is, so the line runs diagonally down the face the way a shaved one does
    /// rather than sitting level and reading as a chinstrap.
    ///
    /// **In the azimuth's cosine and not in [`At::forward`], and that is the
    /// difference between a soft edge and a cliff** (#199). A moving boundary
    /// is as steep as whatever moves it: `forward` is a share of the skull's
    /// reach at the point's own height, and under the jaw that reach is small,
    /// so a millimetre of travel there moved this line three — which made a
    /// 5 mm fade read as a 0.8 mm one and put the only cliff in the file on the
    /// largest facial region. The cosine changes smoothly everywhere the region
    /// reaches, and it is what the border below is written in anyway.
    fn top(&self, facing: f32) -> f32 {
        let ahead = crate::face::smooth(facing / DIAGONAL);
        self.sideburn + (self.cheek - self.sideburn) * ahead
    }
}

impl Region for Flanks {
    fn weight(&self, at: &At) -> f32 {
        // Between the mandible's own border — reached down past by `under` —
        // and the beard line above it.
        let border = Follicles::border(&self.skull, at.facing);
        band(
            at.height,
            border - self.under,
            self.top(at.facing),
            self.fade,
        )
        // Out on the flank, handing the midline over to the chin's patch.
        * crate::face::smooth((at.lateral - INNER) / INNER_RAMP)
        // And in front of the nape.
        * crate::face::smooth((at.facing - self.behind) / 0.30)
    }
}
