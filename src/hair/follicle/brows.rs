//! The eyebrows.
//!
//! The smallest region here and the one that carries the most expression per
//! triangle: a face with no brows reads as a mannequin at any distance, and the
//! brow's height over the eye is most of what an expression is before anything
//! moves.

use serde::{Deserialize, Serialize};

use super::{At, Region, band};
use crate::face::Canon;

/// How the brow patch is shaped on one head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// How high the brow sits over the eye, `-1` low and heavy and `+1` high.
    #[serde(with = "crate::plan::scaled")]
    pub rise: f32,
    /// How far apart the two are, `-1` nearly meeting and `+1` well parted.
    #[serde(with = "crate::plan::scaled")]
    pub apart: f32,
    /// How far the tail runs past the eye's outer corner, `-1` short and `+1`
    /// long.
    #[serde(with = "crate::plan::scaled")]
    pub reach: f32,
}


impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        self.rise = self.rise.clamp(-1.0, 1.0);
        self.apart = self.apart.clamp(-1.0, 1.0);
        self.reach = self.reach.clamp(-1.0, 1.0);
    }
}

/// How far above the eye line the brow sits, in [`Canon::frame`]s.
///
/// Provenance: **looked up**, converted to this file's ruler. The brow sits
/// about 18 mm over the pupil on an adult face whose eye line to menton is
/// about 100 mm, which is this.
const RISE: f32 = 0.18;

/// How far that height moves over the whole of [`Params::rise`], in frames.
///
/// Provenance: **tuned by render** (#199).
const RISE_RANGE: f32 = 0.07;

/// Half the brow's thickness, in frames.
///
/// Provenance: **looked up**, same conversion: a brow is about 12 mm deep at
/// its fullest, so half of it is 6 mm on that 100 mm frame. The first cut of
/// this took the 10 mm a brow is usually quoted at, which left a band whose
/// two fades met in the middle — a brow that never came fully on anywhere
/// (best weight 0.920) and so had no core for a clump to root in.
const THICK: f32 = 0.06;

/// Where the inner end sits, in [`Canon::apart`]s from the midline.
///
/// The brow begins about above the inner corner of the eye, which sits well
/// inside the pupil's own offset — so a share of that offset rather than a
/// measurement of its own.
///
/// Provenance: **derived** from [`Canon::apart`].
const INNER: f32 = 0.35;

/// How far that end moves over the whole of [`Params::apart`], likewise.
///
/// Provenance: **tuned by render** (#199).
const INNER_RANGE: f32 = 0.25;

/// Where the outer end sits, in [`Canon::unit`]s past the pupil's own offset.
///
/// A brow runs past the eye's outer corner, and an eye is one unit wide, so
/// half a unit past the pupil is about the corner and this is a little beyond.
///
/// Provenance: **derived** from [`Canon::unit`].
const OUTER: f32 = 0.62;

/// How far that end moves over the whole of [`Params::reach`], likewise.
///
/// Provenance: **tuned by render** (#199).
const OUTER_RANGE: f32 = 0.22;

/// How softly the patch fades, in frames.
///
/// Tighter than the scalp's, because a brow has an edge a scalp does not: it is
/// read as a shape with a tail, and a soft-edged one reads as a smudge. Held
/// above 2 mm all the same, which is `no_boundary_is_a_cliff`'s floor and the
/// width of two of the finest cells on the face.
///
/// Provenance: **tuned by render** (#199).
const FADE: f32 = 0.032;

/// How far round the head a brow may reach before the face has turned away.
///
/// In [`At::forward`]'s own share, so it follows the head's curve rather than a
/// fixed angle. A brow's tail is on the front of the face by definition; past
/// this the surface belongs to the temple.
///
/// Provenance: **tuned by render** (#199).
const FRONT: f32 = 0.30;

/// The brows, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brows {
    /// The brow's own height, in head-local metres.
    level: f32,
    /// Half its thickness, likewise.
    thick: f32,
    /// The inner end's distance from the midline, likewise.
    inner: f32,
    /// The outer end's, likewise.
    outer: f32,
    /// The edge's width, likewise.
    fade: f32,
}

impl Brows {
    /// Cuts the region from a measured head.
    #[must_use]
    pub(super) fn of(canon: &Canon, params: &Params) -> Self {
        let frame = canon.frame;
        let inner = canon.apart * (INNER + params.apart * INNER_RANGE);
        let outer = canon.apart + canon.unit * (OUTER + params.reach * OUTER_RANGE);
        Self {
            level: canon.level + (RISE + params.rise * RISE_RANGE) * frame,
            thick: THICK * frame,
            // Ordered rather than assumed: the two ends are moved by different
            // axes and a record may put the inner one outside the outer, which
            // would otherwise be a brow of negative width — an empty region
            // that still passes every assertion about its edges.
            inner: inner.min(outer - canon.unit * 0.1),
            outer,
            fade: FADE * frame,
        }
    }
}

impl Region for Brows {
    fn weight(&self, at: &At) -> f32 {
        // On the front of the face, at the brow's own height, between the two
        // ends. The lateral span is taken in metres from the midline rather
        // than in [`At::lateral`]'s share, because a brow is placed against the
        // eye beneath it and the eye is placed in [`Canon`]'s own units — a
        // share of the skull's half-width would drift off the eye on any head
        // whose vault is wide for its face.
        let across = at.across.abs();
        band(at.height, self.level - self.thick, self.level + self.thick, self.fade)
            * band(across, self.inner, self.outer, self.fade)
            * crate::face::smooth((at.forward - FRONT) / 0.25)
    }
}
