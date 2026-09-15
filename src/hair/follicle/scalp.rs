//! The hair of the head, and the line it stops at.
//!
//! The one region that is a whole surface rather than a patch of one, and the
//! only one whose boundary a person can name: a hairline is read at conversation
//! distance and is most of what says how old somebody is.

use glam::Vec3;
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
        use crate::plan::scaled::quantize;
        self.line = quantize(self.line.clamp(-1.0, 1.0));
        self.temples = quantize(self.temples.clamp(0.0, 1.0));
        self.nape = quantize(self.nape.clamp(-1.0, 1.0));
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
/// Provenance: **tuned by render**.
const NAPE: f32 = 0.30;

/// How far the hairline moves over the whole of [`Params::line`], in frames.
///
/// A fifth of the eye-to-chin span either way, which takes the front hairline
/// from about the brow ridge to well up the vault — the range a person would
/// call low-to-high without either end reading as a different species.
///
/// Provenance: **tuned by render**.
const LINE_RANGE: f32 = 0.20;

/// How far the temples pull back at [`Params::temples`] of one, in frames.
///
/// Provenance: **tuned by render**.
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
/// Provenance: **tuned by render**.
const TEMPLE_WIDE: f32 = 0.26;

/// How far the nape moves over the whole of [`Params::nape`], in frames.
///
/// Provenance: **tuned by render**.
const NAPE_RANGE: f32 = 0.18;

/// How softly the hairline fades, in frames.
///
/// **A hairline is the softest edge on a head and the render says so.** Faded
/// over a tenth of this it reads as a wig's rim in the contact sheet — a line
/// hair stopped at rather than thinned through. Real hair thins over a
/// centimetre or more, and both layers want the
/// same gradient: the painted one to fade its density, the geometry one to
/// thin its clumps.
///
/// Provenance: **tuned by render**.
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

/// The space in front of a face that no scalp hair may hang in (#341).
///
/// A box: from the brow line down to the chin's tip, forward of the coronal
/// plane through the two temples, and between them. A temple here is the
/// skull's surface at the brow's height at `TEMPLE_AT`, the azimuth the
/// hairline's own bays are centred on - so the box is the front of the face
/// between the two corners a receding hairline goes back at, which is the part
/// of a head a person looks at when they look somebody in the eye.
///
/// **A construction and not a tolerance.** A curtain over the face used to be
/// read by a test as a share of the hair in a narrower box, and a bob with a
/// long cut hung its fringe to the nose while that share stayed under its bound.
/// The scalp styles walk this box instead: a lock stops before it would enter,
/// so the count of hair inside is zero by construction rather than small by
/// measurement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clearance {
    /// The brow line, in head-local metres: the brow ridge's mean level.
    pub brow: f32,
    /// The chin's tip, likewise.
    pub chin: f32,
    /// How far in front of the head joint the temple plane stands.
    pub front: f32,
    /// How far either temple stands from the midline.
    pub side: f32,
}

impl Clearance {
    /// Cuts the box from a measured head and the height its brows run at.
    #[must_use]
    pub(super) fn of(skull: &Skull, canon: &Canon, brow: f32) -> Self {
        let temple = skull.surface_at(brow, TEMPLE_AT.acos());
        Self {
            brow,
            chin: canon.chin(),
            front: temple.z,
            side: temple.x.abs(),
        }
    }

    /// Whether a head-local point is inside the box.
    ///
    /// Open on every side: a point on a wall is outside, so a lock stopped at
    /// the wall is a lock that did not enter.
    #[must_use]
    pub fn contains(&self, at: Vec3) -> bool {
        at.y < self.brow && at.y > self.chin && at.z > self.front && at.x.abs() < self.side
    }

    /// Where a straight run from `from` to `to` first enters the box grown by
    /// `margin` on every side, as a share of the run: `Some(0.0)` if it starts
    /// inside, `None` if it never enters.
    ///
    /// Clipped against the box's five walls one half-space at a time (the
    /// temple plane has no back wall: behind it is the head). A run can enter
    /// and leave between its two ends, so testing the ends alone would miss a
    /// lock cutting the corner of the box.
    #[must_use]
    pub(crate) fn entry(&self, from: Vec3, to: Vec3, margin: f32) -> Option<f32> {
        let run = to - from;
        let mut enter = 0.0f32;
        let mut exit = 1.0f32;
        // Each wall as `along * t < room`: how fast the run closes on it, and
        // how far it has to go.
        for (along, room) in [
            (run.y, self.brow + margin - from.y),
            (-run.y, from.y - self.chin + margin),
            (-run.z, from.z - self.front + margin),
            (run.x, self.side + margin - from.x),
            (-run.x, from.x + self.side + margin),
        ] {
            if along.abs() <= f32::EPSILON {
                if room <= 0.0 {
                    return None;
                }
                continue;
            }
            let at = room / along;
            if along < 0.0 {
                enter = enter.max(at);
            } else {
                exit = exit.min(at);
            }
            if enter >= exit {
                return None;
            }
        }
        Some(enter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clearance() -> Clearance {
        Clearance {
            brow: 0.02,
            chin: -0.10,
            front: 0.05,
            side: 0.06,
        }
    }

    #[test]
    fn a_run_enters_the_face_where_it_crosses_the_first_wall() {
        let face = clearance();
        // Straight down the midline in front of the face: enters at the brow.
        let down = face.entry(Vec3::new(0.0, 0.06, 0.08), Vec3::new(0.0, -0.02, 0.08), 0.0);
        assert!((down.expect("enters") - 0.5).abs() < 1e-5, "{down:?}");
        // Already inside.
        assert_eq!(
            face.entry(Vec3::new(0.0, 0.0, 0.08), Vec3::new(0.0, -0.01, 0.08), 0.0),
            Some(0.0)
        );
        // Beside the face at the temple, and behind the temple plane: never.
        assert_eq!(
            face.entry(
                Vec3::new(0.07, 0.06, 0.08),
                Vec3::new(0.07, -0.2, 0.08),
                0.0
            ),
            None
        );
        assert_eq!(
            face.entry(Vec3::new(0.0, 0.06, 0.04), Vec3::new(0.0, -0.2, 0.04), 0.0),
            None
        );
        // The same run beside the face, grown by a margin that reaches it.
        assert!(
            face.entry(
                Vec3::new(0.07, 0.06, 0.08),
                Vec3::new(0.07, -0.2, 0.08),
                0.02
            )
            .is_some()
        );
        // Cutting the corner: both ends outside, the middle inside.
        let corner = face.entry(Vec3::new(0.10, 0.0, 0.08), Vec3::new(0.0, 0.04, 0.08), 0.0);
        let at = corner.expect("cuts the corner");
        let point = Vec3::new(0.10, 0.0, 0.08).lerp(Vec3::new(0.0, 0.04, 0.08), at + 1e-3);
        assert!(face.contains(point), "{at} lands at {point}");
    }
}
