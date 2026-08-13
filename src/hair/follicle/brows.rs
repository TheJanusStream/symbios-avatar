//! The eyebrows.
//!
//! The smallest region here and the one that carries the most expression per
//! triangle: a face with no brows reads as a mannequin at any distance, and the
//! brow's height over the eye is most of what an expression is before anything
//! moves.
//!
//! # A brow is a line before it is a patch
//!
//! Everything about a brow follows one curve — [`Ridge`] — and both layers are
//! written against that one object rather than against two copies of it. The
//! mask centres its band on the ridge; the style of #205 combs its clumps along
//! it and takes its own rise and fall from the ridge's slope. So a painted brow
//! and a grown brow arch together by construction, which is the discipline #199
//! spent the jawline learning: a boundary that is a copy of another boundary
//! drifts from it, and a boundary that IS it cannot.

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
    /// How much the brow arches, `-1` a straight bar and `+1` a high curve.
    ///
    /// **Zero is the arch an ordinary brow has, not a flat one**, which is the
    /// convention every axis in this file keeps: the constant is the looked-up
    /// figure and the axis moves it. A brow with no arch at all is a look a
    /// record may ask for and not a sensible default for every face.
    ///
    /// It lives here rather than on the style because it says where the brow
    /// IS — which both layers obey — rather than what is grown on it. A record
    /// changing its haircut does not move its brow ridge.
    #[serde(with = "crate::plan::scaled")]
    pub arch: f32,
}

impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.rise = quantize(self.rise.clamp(-1.0, 1.0));
        self.apart = quantize(self.apart.clamp(-1.0, 1.0));
        self.reach = quantize(self.reach.clamp(-1.0, 1.0));
        self.arch = quantize(self.arch.clamp(-1.0, 1.0));
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

/// How far the arch's peak stands above the line's mean, in frames.
///
/// Provenance: **looked up**, converted to this file's ruler: the peak of an
/// ordinary brow sits about 5 mm above its inner end on that 100 mm frame, and
/// [`ARCH_MEAN`] is what turns a height above the inner end into one above the
/// mean.
const ARCH: f32 = 0.048;

/// How far that moves over the whole of [`Params::arch`], likewise.
///
/// Set so that `-1` is very nearly a straight bar — 2 mm of arch on a 104 mm
/// frame — without going negative, a brow that peaks at its inner end being an
/// anatomy nothing has.
///
/// Provenance: **derived** from the flattest brow the axis should reach.
const ARCH_RANGE: f32 = 0.030;

/// Where along the brow the arch peaks, as a share of its length.
///
/// Provenance: **looked up**. The peak of a brow sits over the outer third of
/// the iris, which is about two thirds of the way from the inner end to the
/// tail.
const PEAK: f32 = 0.68;

/// How high the tail ends, as a share of the peak's own height.
///
/// The tail comes down from the peak without returning to the inner end's
/// height, which is what makes the two ends of a brow different and what makes
/// the whole read as a curve rather than as a hoop.
///
/// Provenance: **looked up**, same source as [`PEAK`].
const TAIL: f32 = 0.45;

/// The mean of [`hump`] over the brow's whole length.
///
/// **The arch tilts the line without moving it**, which is what keeps
/// [`Params::arch`] from fighting [`Params::rise`]: subtracting the profile's
/// own mean leaves the brow's average height exactly where #199 tuned it, so
/// turning the arch up raises the peak and drops the ends rather than raising
/// the whole brow.
///
/// Provenance: **derived**, exactly. [`crate::face::smooth`] is symmetric about
/// its middle, so its mean over a full ramp is one half, and the profile is two
/// ramps.
const ARCH_MEAN: f32 = 0.5 * PEAK + (1.0 - PEAK) * (1.0 - 0.5 * (1.0 - TAIL));

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

/// The arch's profile: `0` at the inner end, `1` at the peak, [`TAIL`] at the
/// tail.
///
/// Two smoothsteps meeting at [`PEAK`], so the line arrives and leaves with zero
/// slope at both ends and has no corner at the top. A brow drawn with a corner
/// in it reads as a drawn-on brow, which is the one thing the geometry layer is
/// there to avoid.
fn hump(along: f32) -> f32 {
    if along <= PEAK {
        crate::face::smooth(along / PEAK)
    } else {
        1.0 - (1.0 - TAIL) * crate::face::smooth((along - PEAK) / (1.0 - PEAK))
    }
}

/// The line one brow follows, in head-local metres.
///
/// **One curve, both layers.** The mask centres its band on this and the style
/// combs along it, so nothing has to keep two arcs in step. Everything is on the
/// right-hand side of the face, distances out from the midline being unsigned —
/// a head is symmetric here, and a style that needs to tell one brow from the
/// other has the sign of the point it was handed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ridge {
    /// Where the inner end sits, as a distance from the midline.
    pub inner: f32,
    /// Where the tail ends, likewise.
    pub outer: f32,
    /// The line's mean height above the head joint.
    pub level: f32,
    /// How far the peak stands above that mean.
    pub arch: f32,
    /// Half the thickness of the band centred on the line.
    ///
    /// Part of the line's own description rather than of the mask that uses it,
    /// because it is what says how far off the line hair may be — which the mask
    /// needs to draw its band and an instrument needs to say whether the grown
    /// layer stayed inside it (#205).
    pub thick: f32,
}

impl Ridge {
    /// How far along the brow a point sits: `0` at the inner end, `1` at the
    /// tail.
    ///
    /// Takes a signed offset and answers for whichever brow it belongs to,
    /// since the two are mirror images and every caller has the signed number
    /// to hand.
    #[must_use]
    pub fn along(&self, across: f32) -> f32 {
        ((across.abs() - self.inner) / self.span()).clamp(0.0, 1.0)
    }

    /// How long the brow is, from the inner end to the tail.
    ///
    /// Floored, because [`Ridge::along`] divides by it and a record that puts
    /// the two ends together would otherwise turn the whole brow into one
    /// point. The ends are ordered when the ridge is cut, so this is a guard
    /// rather than a correction.
    #[must_use]
    pub fn span(&self) -> f32 {
        (self.outer - self.inner).max(super::MINIMUM_SPAN)
    }

    /// The line's height a share of the way along it.
    #[must_use]
    pub fn height(&self, along: f32) -> f32 {
        self.level + self.arch * (hump(along.clamp(0.0, 1.0)) - ARCH_MEAN)
    }
}

/// The brows, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brows {
    /// The line the patch is centred on, and how thick a band it carries.
    ridge: Ridge,
    /// The edge's width, in head-local metres.
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
            ridge: Ridge {
                thick: THICK * frame,
                // Ordered rather than assumed: the two ends are moved by
                // different axes and a record may put the inner one outside the
                // outer, which would otherwise be a brow of negative width — an
                // empty region that still passes every assertion about its edges.
                inner: inner.min(outer - canon.unit * 0.1),
                outer,
                level: canon.level + (RISE + params.rise * RISE_RANGE) * frame,
                arch: (ARCH + params.arch * ARCH_RANGE) * frame,
            },
            fade: FADE * frame,
        }
    }

    /// The line this patch is centred on, for the style that combs along it.
    #[must_use]
    pub(super) fn ridge(&self) -> Ridge {
        self.ridge
    }
}

impl Region for Brows {
    fn weight(&self, at: &At) -> f32 {
        // On the front of the face, within the band's thickness of the ridge's
        // own height there, between the two ends. The lateral span is taken in
        // metres from the midline rather than in [`At::lateral`]'s share,
        // because a brow is placed against the eye beneath it and the eye is
        // placed in [`Canon`]'s own units — a share of the skull's half-width
        // would drift off the eye on any head whose vault is wide for its face.
        let across = at.across.abs();
        // **Centred on the arch rather than on a level, which is what makes the
        // painted brow curve** (#205). The band's own thickness is unchanged, so
        // this tilts the patch and does not fatten it.
        let level = self.ridge.height(self.ridge.along(across));
        let thick = self.ridge.thick;
        band(at.height, level - thick, level + thick, self.fade)
            * band(across, self.ridge.inner, self.ridge.outer, self.fade)
            * crate::face::smooth((at.forward - FRONT) / 0.25)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ridge on the scale of a real head, in metres.
    fn ridge() -> Ridge {
        Ridge {
            inner: 0.011,
            outer: 0.050,
            level: 0.024,
            arch: 0.005,
            thick: 0.0063,
        }
    }

    #[test]
    fn the_arch_tilts_the_line_without_raising_it() {
        // The reason [`ARCH_MEAN`] exists: `rise` places the brow and `arch`
        // curves it, and if the arch moved the mean then turning it up would
        // also raise the brow — two axes doing one thing, which is how a panel
        // becomes impossible to use. Asserted as the mean over the span being
        // the level, whatever the arch.
        for arch in [0.0, 0.002, 0.005, 0.010] {
            let ridge = Ridge { arch, ..ridge() };
            const STEPS: usize = 400;
            let mean = (0..STEPS)
                .map(|step| ridge.height((step as f32 + 0.5) / STEPS as f32))
                .sum::<f32>()
                / STEPS as f32;
            assert!(
                (mean - ridge.level).abs() < 1e-4,
                "an arch of {arch} m moved the brow's mean height to {mean} from {}",
                ridge.level
            );
        }
    }

    #[test]
    fn the_line_peaks_over_the_outer_third_and_ends_high() {
        // The anatomy the profile is for: a brow rises from its inner end to a
        // peak two thirds out and comes down to a tail that stays above where it
        // started. All three, because a hump that merely goes up and down would
        // pass an assertion about its peak and read as a hoop.
        let ridge = ridge();
        let peak = (0..=100)
            .max_by(|one, two| {
                ridge
                    .height(*one as f32 / 100.0)
                    .total_cmp(&ridge.height(*two as f32 / 100.0))
            })
            .expect("a hundred samples") as f32
            / 100.0;
        assert!(
            (peak - PEAK).abs() < 0.02,
            "the arch peaks at {peak} of the way along rather than at {PEAK}"
        );
        assert!(
            ridge.height(1.0) > ridge.height(0.0),
            "the tail ended at or below the inner end, which is a hoop and not a brow"
        );
        assert!(
            ridge.height(1.0) < ridge.height(PEAK),
            "the tail is the highest point of the brow"
        );
    }

    #[test]
    fn the_line_rises_to_its_peak_and_falls_from_it() {
        // The style's comb direction is the height this reports over a clump's
        // own travel, so a line that did not rise and fall would comb every hair
        // of a brow the same way — which is what the whole arch is here to stop.
        let ridge = ridge();
        let over = |from: f32, to: f32| ridge.height(to) - ridge.height(from);
        assert!(
            over(0.15, 0.45) > 0.0,
            "the inner brow does not climb toward the peak"
        );
        assert!(
            over(0.75, 0.98) < 0.0,
            "the tail does not come down from the peak"
        );
    }

    #[test]
    fn the_mask_arches_with_the_line_and_keeps_its_thickness() {
        // The paint has to arch too, or the grown brow curves over a straight
        // painted one and the two layers disagree about where the brow is — the
        // one thing the epic says must not happen. Asserted through the mask
        // itself: the height at which the band is fully on has to follow the
        // ridge across the brow, and the band has to be no thicker for it.
        let brows = |arch: f32| {
            let params = Params {
                arch,
                ..Params::default()
            };
            // A canon on the scale of a real head, so the constants convert.
            Brows::of(
                &Canon {
                    head: 0,
                    frame: 0.1044,
                    level: 0.0052,
                    apart: 0.0315,
                    unit: 0.0287,
                },
                &params,
            )
        };
        // Where the band is centred, measured rather than read off the ridge: the
        // height at which the weight peaks, at one point along the brow.
        let centre = |brows: &Brows, across: f32| {
            let at = |height: f32| At {
                height,
                forward: 1.0,
                lateral: 0.5,
                facing: 1.0,
                across,
            };
            let best = (0..2000)
                .max_by(|one, two| {
                    let height = |step: &i32| 0.0 + *step as f32 * 0.00002;
                    brows
                        .weight(&at(height(one)))
                        .total_cmp(&brows.weight(&at(height(two))))
                })
                .expect("two thousand samples");
            best as f32 * 0.00002
        };
        // How thick it is there, as the span of heights fully inside it.
        let thickness = |brows: &Brows, across: f32| {
            let inside = (0..2000).filter(|step| {
                let height = *step as f32 * 0.00002;
                brows.weight(&At {
                    height,
                    forward: 1.0,
                    lateral: 0.5,
                    facing: 1.0,
                    across,
                }) > 0.5
            });
            inside.count() as f32 * 0.00002
        };
        let arched = brows(0.0);
        let ridge = arched.ridge();
        let (inner, peak) = (
            ridge.inner + ridge.span() * 0.1,
            ridge.inner + ridge.span() * PEAK,
        );
        assert!(
            centre(&arched, peak) > centre(&arched, inner) + 0.001,
            "the mask's band does not rise from the inner end to the peak: {:.1} mm against              {:.1} mm",
            centre(&arched, peak) * 1000.0,
            centre(&arched, inner) * 1000.0
        );
        assert!(
            (thickness(&arched, peak) - thickness(&arched, inner)).abs() < 0.001,
            "the band is {:.1} mm deep at the peak and {:.1} mm at the inner end, so arching it              fattened it",
            thickness(&arched, peak) * 1000.0,
            thickness(&arched, inner) * 1000.0
        );
        // And the axis does what it says: flatter at -1 than at +1.
        let rise = |brows: &Brows| {
            let ridge = brows.ridge();
            centre(brows, ridge.inner + ridge.span() * PEAK)
                - centre(brows, ridge.inner + ridge.span() * 0.1)
        };
        assert!(
            rise(&brows(-1.0)) < rise(&arched) && rise(&arched) < rise(&brows(1.0)),
            "the arch axis does not order a brow from flat to curved: {:.1}, {:.1}, {:.1} mm",
            rise(&brows(-1.0)) * 1000.0,
            rise(&arched) * 1000.0,
            rise(&brows(1.0)) * 1000.0
        );
    }

    #[test]
    fn a_point_outside_the_ends_still_answers_somewhere_along() {
        // The mask asks about points beyond both ends — its own lateral fade
        // reaches past them — and a share of the way along has to stay in range
        // there, or the band's centre jumps to a height off the head and the
        // fade the region exists for becomes a cliff.
        let ridge = ridge();
        for across in [-0.2f32, -0.02, 0.0, 0.005, 0.2] {
            let along = ridge.along(across);
            assert!(
                (0.0..=1.0).contains(&along),
                "a point {across} m across the face sits {along} along the brow"
            );
        }
        assert_eq!(
            ridge.along(-0.030),
            ridge.along(0.030),
            "the two brows are not mirror images"
        );
    }
}
