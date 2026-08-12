//! The hair of the head, and the line it stops at.
//!
//! The one region that is a whole surface rather than a patch of one, and the
//! only one whose boundary a person can name: a hairline is read at conversation
//! distance and is most of what says how old somebody is.

use serde::{Deserialize, Serialize};

use super::{At, Region, band};
use crate::face::{Canon, Skull};

/// How the hairline is shaped on one head.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// Where the hairline sits, `-1` receding and `+1` low on the brow.
    ///
    /// The whole curve moves together, which is what age does to it least, and
    /// what a low or high hairline is. The temples are the part that recedes on
    /// its own; see [`Self::temples`].
    #[serde(with = "crate::plan::scaled")]
    pub line: f32,
    /// How deeply the hairline notches back at the temples, `0` square across
    /// the brow and `1` two deep bays.
    ///
    /// **Separate from [`Self::line`] because recession is not a raised
    /// hairline.** A hairline that has moved up as a whole is a high forehead,
    /// which is a face somebody was born with; a hairline that has kept its
    /// midline peak while the corners went back is the pattern the word
    /// recession means. Drawing both with one axis makes every balding head
    /// look like a tall one.
    #[serde(with = "crate::plan::scaled")]
    pub temples: f32,
    /// Where the hair stops at the back, `-1` shaved high up the nape and `+1`
    /// low onto the neck.
    #[serde(with = "crate::plan::scaled")]
    pub nape: f32,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            line: 0.0,
            temples: 0.25,
            nape: 0.0,
        }
    }
}

impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        self.line = self.line.clamp(-1.0, 1.0);
        self.temples = self.temples.clamp(0.0, 1.0);
        self.nape = self.nape.clamp(-1.0, 1.0);
    }
}

/// Where the hairline sits dead ahead, in [`Canon::frame`]s above the eye line.
///
/// Provenance: **derived** from the canon of thirds. A face is divided
/// trichion to glabella, glabella to subnasale, subnasale to menton, in three
/// equal parts; [`Canon::frame`] runs from the eye line — a little under the
/// glabella — to the menton, so it spans about two of those thirds and one
/// third is half a frame. The hairline is one third above the brow, which is
/// this.
const FRONT: f32 = 0.5;

/// How far above the ear's centre the hairline runs at the side, in frames.
///
/// **Derived from the ear, and from a correct one rather than the built one.**
/// The temporal hairline passes about the top of the ear; an ear is 0.267 to
/// 0.30 of head height in life, so on a head about two frames tall its half-
/// span is a little under a third of a frame, which is this.
///
/// The ear this crate currently builds is bigger than that — `face::features`
/// records its shell at 0.418 of head height against life's 0.267 and names it
/// as a defect of its own — so until that is fixed the top of the built ear
/// pokes a centimetre into the scalp's region. Matching this to the oversized
/// ear would bury the correct hairline to hide another part's bug, and would
/// then be wrong twice when the ear is fixed.
///
/// Provenance: **derived** from the ear's own proportion in life.
const SIDE: f32 = 0.30;

/// How far below the ear's centre the hair reaches at the nape, in frames.
///
/// Provenance: **tuned by render** (#199).
const NAPE: f32 = 0.30;

/// How far the hairline moves over the whole of [`Params::line`], in frames.
///
/// A fifth of the eye-to-chin span either way, which takes the front hairline
/// from about the brow ridge to well up the vault — the range a person would
/// call low-to-high without either end reading as a different species.
///
/// Provenance: **tuned by render** (#199).
const LINE_RANGE: f32 = 0.20;

/// How far the temples pull back at [`Params::temples`] of one, in frames.
///
/// Provenance: **tuned by render** (#199).
const TEMPLE_DEPTH: f32 = 0.28;

/// Where the temple bay is centred, as a cosine of the azimuth.
///
/// About 50° off dead ahead, which is where a receding corner sits on a head:
/// far enough round that the midline peak survives between the two bays, and
/// not so far that the bay is behind the eye.
///
/// Provenance: **derived** from the anatomy the bays are named for, quoted as
/// the cosine this file works in.
const TEMPLE_AT: f32 = 0.64;

/// How wide that bay is, in the same cosine.
///
/// Provenance: **tuned by render** (#199).
const TEMPLE_WIDE: f32 = 0.26;

/// How far the nape moves over the whole of [`Params::nape`], in frames.
///
/// Provenance: **tuned by render** (#199).
const NAPE_RANGE: f32 = 0.18;

/// How softly the hairline fades, in frames.
///
/// **A hairline is the softest edge on a head and the render says so.** The
/// first cut of this faded over a tenth of what it does now and read as a wig's
/// rim in the contact sheet — a line hair stopped at rather than thinned
/// through. Real hair thins over a centimetre or more, and both layers want the
/// same gradient: the painted one to fade its density, the geometry one to
/// thin its clumps.
///
/// Provenance: **tuned by render** (#199).
const FADE: f32 = 0.09;

/// The scalp, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scalp {
    /// The hairline dead ahead, in head-local metres.
    front: f32,
    /// The hairline at the side, likewise.
    side: f32,
    /// The hairline at the nape, likewise.
    back: f32,
    /// How far the temple bays cut, in metres.
    temples: f32,
    /// The edge's width, in metres.
    fade: f32,
    /// The crown, so a query above the head cannot be answered as scalp.
    crown: f32,
}

impl Scalp {
    /// Cuts the region from a measured head.
    #[must_use]
    pub(super) fn of(skull: &Skull, canon: &Canon, params: &Params) -> Self {
        let frame = canon.frame;
        let shift = params.line * LINE_RANGE * frame;
        let (_, crown) = skull.throat_and_crown();
        Self {
            // Down with a rising `line`: `+1` is low on the brow.
            front: canon.level + FRONT * frame - shift,
            side: canon.ear_centre() + SIDE * frame - shift,
            back: canon.ear_centre() - NAPE * frame - params.nape * NAPE_RANGE * frame,
            temples: params.temples * TEMPLE_DEPTH * frame,
            fade: FADE * frame,
            crown,
        }
    }

    /// Where the hairline sits at one azimuth, in head-local metres.
    ///
    /// Three anchors — dead ahead, at the side and at the nape — carried
    /// between by the azimuth's own cosine, with the temple bays added on top.
    /// Interpolating in the cosine rather than in the angle is what puts the
    /// change where the head changes: the hairline barely moves across the
    /// forehead and drops fast round the temple, which is the cosine's own
    /// shape and not a curve anybody had to fit.
    fn line(&self, facing: f32) -> f32 {
        let ahead = facing.max(0.0);
        let behind = (-facing).max(0.0);
        let level = self.side + (self.front - self.side) * ahead + (self.back - self.side) * behind;
        // A bay either side, raised where a receding corner sits. Gaussian
        // rather than a band because a bay has no edges — it is the smooth part
        // of a hairline, and any join here would read as a step in the one
        // boundary a person looks straight at.
        let from = (facing - TEMPLE_AT) / TEMPLE_WIDE;
        level + self.temples * (-from * from).exp()
    }
}

impl Region for Scalp {
    fn weight(&self, at: &At) -> f32 {
        // Above the line and below the crown. The upper bound is not margin: a
        // query can arrive from anywhere — a scattered root above the head, a
        // texel on a hat — and a region that answers `1` above its own skull is
        // one the geometry layer will grow hair off the top of.
        //
        // The line is the MIDDLE of the fade rather than its foot, so that the
        // landmark [`Scalp::line`] reports and the place a person would point
        // at are the same height. Half the fade sits above it and half below.
        band(
            at.height,
            self.line(at.facing) - self.fade * 0.5,
            self.crown + self.fade,
            self.fade,
        )
    }
}
