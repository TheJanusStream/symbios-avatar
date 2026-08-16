//! The eyebrows' own catalogue.
//!
//! **A catalogue of its own rather than the shared
//! [`Fall`](super::super::clump::Fall), and it has to be.** A brow is where
//! the engine's defaults are most wrong:
//! `Fall` combs downhill, and downhill on a brow ridge is straight over the eye,
//! so a brow grown that way is a row of dark spikes hanging into the socket. The
//! render was unambiguous about it.
//!
//! Two things fix it, and both are about direction rather than about spending
//! more triangles:
//!
//! - **A brow's hairs run ALONG the brow**, out from the inner end toward the
//!   temple. Overlapping lateral streaks read as one stroke; nine hanging spines
//!   read as nine spines however finely they are drawn.
//! - **Their rise and fall is the ridge's own height** — up over the inner half,
//!   down over the tail — read off [`Ridge`] over each clump's own travel rather
//!   than invented here. The arch is one anatomical constant in
//!   [`super::super::follicle::brows`] and everything about a brow's direction
//!   falls out of it.
//!
//! # What the two styles differ by, and what they do not
//!
//! [`BrowStyle::Natural`] and [`BrowStyle::Thick`] cost the same: the same number
//! of clumps, each at the sampler's floor. The difference is section, streak
//! length, taper and how flat they lie — a fine tapered stroke against a coarse
//! blunt block. **A style that reads differently for the same triangles is the
//! whole bet of the hair system**, and the brow is the smallest region to test
//! it on, so it is the one where a wider ribbon has to do the work a longer
//! clump list would have done.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, brows::Ridge};
use super::{Cut, Style, clumps_for};

/// The base styles of the eyebrows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum BrowStyle {
    /// Nothing is grown here: the brow is painted, or bare.
    ///
    /// The painted layer's drawn brow is what a face wears at this setting, and
    /// most faces should have one — a bare brow ridge reads as a mannequin from
    /// across a room.
    #[default]
    None,
    /// A tapered stroke following the ridge, thinning to the tail.
    Natural,
    /// A fuller, blunter one: coarser clumps lying less flat, and a tail that
    /// keeps most of its width.
    Thick,
}

/// How long a streak is at full length, as a share of the ridge's own span.
///
/// **A share rather than millimetres**, for the reason every boundary in the
/// follicle module is: a 12 mm streak is a third of a brow on one head and half
/// of one on another, and the whole point of measuring the ridge is to stop
/// numbers meaning different anatomy on different faces.
///
/// A third of the span, so that nine streaks a brow overlap about three deep
/// along it and the row reads continuous. Longer and each streak starts to
/// disagree with the ridge's curve over its own length; shorter and the gaps
/// between them show.
///
/// Provenance: **tuned by render**, against the clump count `FULL` sets.
const REACH: [f32; 2] = [0.50, 0.44];

/// How wide one is at the root, in metres.
///
/// The lever that does the work a longer clump list cannot afford: a wider
/// ribbon costs exactly the same triangles as a narrow one, so a brow that has
/// to read at eighteen clumps reads by being wide rather than by being many.
///
/// Provenance: **tuned by render**.
const WIDTH: [f32; 2] = [0.0038, 0.0062];

/// What share of the section is left at the tip.
///
/// The natural brow comes to a point and the thick one stays blunt, which is
/// the difference between a stroke and a block at the same cost.
///
/// Provenance: **tuned by render**.
const TAPER: [f32; 2] = [0.22, 0.50];

/// How much of its length and width the tail loses.
///
/// A brow's tail is its thin end — the head of a brow is dense and the tail
/// tapers away — so this thins the clumps over the outer part of the ridge
/// rather than over the whole of it. See [`Brow::fullness`].
///
/// Provenance: **looked up** (the head/body/tail reading of a brow), amount
/// **tuned by render**.
const THIN: [f32; 2] = [0.55, 0.22];

/// How flat against the skin the clumps lie, `1` flat and `0` standing out.
///
/// Near flat for the natural brow, which is what a brow does. The thick one
/// stands off a little, and that lift over a 10 mm streak is the whole of what
/// makes it read as bushy rather than as painted.
///
/// Provenance: **tuned by render**.
const LIE: [f32; 2] = [0.97, 0.88];

/// Where along the ridge the tail's thinning begins.
///
/// Just past the arch's own peak, so the brow is full through the peak and
/// tapers only over the last third.
///
/// Provenance: **looked up**, same source as the ridge's `PEAK`.
const TAPER_FROM: f32 = 0.62;

/// How thin a clump is at each of its ends, as a share of its middle.
///
/// **A leaf rather than a wedge, and it is what makes eighteen clumps read as
/// two brows.** Swept from full width to a point, every clump ends in a
/// blunt face at the root, and a row of them reads as a row of separate objects
/// however much they overlap. Thin at both ends, the overlaps have no ends in
/// them: the union is one arched mass with a ragged edge, which is what a brow is
/// at this triangle count. See [`Shape::width_at`], whose default is the wedge.
///
/// Provenance: **tuned by render**.
const ENDS: f32 = 0.3;

/// How much of its offset from the ridge a clump closes over its own length.
///
/// **What turns a scatter of streaks into one brow.** Roots land anywhere
/// in the band's twelve millimetres, and a streak that runs straight out from
/// wherever it started stays there — the row reads as two or three stacked
/// bars, and the ones rooted in the band's lower edge run out of the brow
/// altogether and onto the eyelid. Real brow hairs converge toward the body of
/// the brow, and a clump that does the same is both more correct and the reason
/// eighteen of them read as a stroke.
///
/// Half is enough to gather them without making the brow read as a single line.
///
/// Provenance: **tuned by render**.
const GATHER: f32 = 0.7;

/// How far past the tail a clump may reach, in metres.
///
/// A streak rooted near the tail would otherwise run its whole length past the
/// end of the brow and hang in the air beside the temple, which reads
/// as a detached dash. Inside the mask's own lateral fade — 3.3 mm on this
/// frame — so a hair that passes the nominal end is still somewhere the paint
/// reaches.
///
/// Provenance: **derived** from the brow mask's fade.
const OVERSHOOT: f32 = 0.002;

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Only a floor against the degenerate: a clump under a millimetre and a half is
/// invisible and its triangles are spent on nothing, which is what
/// [`Shape::length`]'s zero is for.
///
/// **Low on purpose, because a higher floor is the wrong instrument.** A root
/// near the tail has little ridge left to run along, and declining every
/// short clump throws away a fifth of the brow's clumps and leaves the outer
/// five millimetres bare — measured at five times this floor: thirteen clumps
/// asked for, ten grown, and the audit's grown band stopping short of the
/// mask's. A fat lozenge at the tail is not the shortness, it is a short
/// clump keeping its full width; the section follows the length, so a short
/// clump is what it should be — the fine hair at the end of a brow.
///
/// Provenance: **derived** from what the render can resolve.
const LEAST_WORTH: f32 = 0.08;

/// How steeply [`Cut::droop`] tilts a brow away from the ridge's own slope.
///
/// **The one axis a brow reads differently from every other region**: hair
/// elsewhere hangs, and a brow does not, so droop here is the difference between
/// a raised brow and a heavy one rather than between short hair and long. In
/// metres of drop per metre run, added to the ridge's slope, so half of the axis
/// is the ridge as measured.
///
/// Provenance: **tuned by render**.
const SAG: f32 = 0.45;

impl Style for BrowStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        let slot = match self {
            Self::None => return None,
            Self::Natural => 0,
            Self::Thick => 1,
        };
        let ridge = head.brow_ridge();
        // **A narrow axis, unlike every other region's** (#205): brow hairs are
        // brow hairs, and the first cut of this took the shared floor of 0.35 —
        // which at the default cut left a streak five millimetres long and four
        // wide, a row of arrowheads rather than a brow. A record may still ask
        // for a sparse or a heavy brow through `thickness` and `density`; what it
        // may not do is ask for one a quarter of the length of a brow.
        let length = 0.75 + 0.5 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        Some(Box::new(Brow {
            ridge,
            reach: ridge.span() * REACH[slot] * length,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            thin: THIN[slot],
            lie: LIE[slot],
            sag: (cut.droop.clamp(0.0, 1.0) - 0.5) * 2.0 * SAG,
        }))
    }

    fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize {
        match self {
            Self::None => 0,
            // **Both styles the same count, deliberately** (#205). Thick is
            // fuller by section rather than by number, so choosing it costs the
            // budget nothing and the greediest legal record has no dearer brow
            // to pick.
            Self::Natural | Self::Thick => clumps_for(cut, follicle),
        }
    }
}

/// A clump combed along the brow ridge.
///
/// Everything it needs to know about the head is the [`Ridge`] it grew on, which
/// is the same object the mask centred its band on — so the geometry cannot arch
/// differently from the paint under it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Brow {
    /// The line this brow follows.
    ridge: Ridge,
    /// How far out along it one clump runs at full length, in metres.
    reach: f32,
    /// How wide one is at the root, in metres.
    width: f32,
    /// What share of the section is left at the tip.
    taper: f32,
    /// How much of its length and width the tail loses.
    thin: f32,
    /// How flat against the skin it lies.
    lie: f32,
    /// An extra drop per metre run, over and above the ridge's own slope.
    sag: f32,
}

impl Brow {
    /// How full the brow is a share of the way along it, `1` through the body
    /// and less over the tail.
    ///
    /// One profile for length and section together, because a brow's tail is
    /// thin in both senses at once and two knobs would let a record ask for a
    /// tail that was short and fat.
    fn fullness(&self, along: f32) -> f32 {
        let over = (along - TAPER_FROM) / (1.0 - TAPER_FROM);
        1.0 - self.thin * crate::face::smooth(over)
    }

    /// Where along the ridge one root sits.
    fn along(&self, root: &Root) -> f32 {
        self.ridge.along(root.at.x)
    }

    /// How far above the ridge's own line a root sits, in metres.
    ///
    /// Signed, and the thing [`GATHER`] closes: a clump rooted below the line
    /// climbs toward it as it runs, which is how a band twelve millimetres deep
    /// grows a brow rather than three rows of one.
    fn offset(&self, root: &Root) -> f32 {
        root.at.y - self.ridge.height(self.along(root))
    }

    /// How much of the full section and length this root gets.
    ///
    /// The mask's own weight is in here as well as the tail's taper: a clump
    /// rooted in the soft top or bottom edge of the band is shorter and finer
    /// than one in the middle, which is what makes the brow's edge read as hair
    /// thinning rather than as hair stopping. Square-rooted for the same reason
    /// [`Fall`](super::super::clump::Fall) does it — the last tenth of an edge
    /// should keep some substance.
    fn share(&self, root: &Root) -> f32 {
        self.fullness(self.along(root)) * root.weight.clamp(0.0, 1.0).sqrt()
    }

    /// How much of the full section this clump gets.
    ///
    /// **Its own length as a share of the full reach, so a clump is as thick as
    /// it is long.** The alternative is [`Self::share`] alone, and the
    /// difference is every clump the end of the brow cuts short: at full width a
    /// four-millimetre one is a lozenge sitting off the tail, and at a
    /// proportional width it is the fine hair a brow actually ends in.
    fn stoutness(&self, root: &Root) -> f32 {
        self.length(root) / self.reach.max(f32::EPSILON)
    }

    /// Which way this clump runs along the skin: outward, toward the temple.
    ///
    /// **Sideways only, with the rise left out of it on purpose.** How much a
    /// clump climbs is the ridge's own business and [`Shape::at`] takes it from
    /// the ridge's height directly; a direction that also carried the climb
    /// would add the two and then subtract one again, leaving `droop`
    /// doing nothing at all — the axis cancelled by the very
    /// correction that keeps a clump on its line (guarded by
    /// `droop_raises_and_lowers_a_brow_rather_than_hanging_it`).
    ///
    /// The surface's own component is taken out so the streak stays on the brow it
    /// grew on rather than leaving the skin.
    fn run(&self, root: &Root) -> Vec3 {
        let side = if root.at.x < 0.0 { -1.0 } else { 1.0 };
        let out = root.out;
        (Vec3::X * side - out * (out.x * side)).normalize_or(out)
    }
}

impl Shape for Brow {
    fn length(&self, root: &Root) -> f32 {
        // Never further than the brow itself goes: a clump rooted near the tail
        // has only the room that is left, or it hangs in the air past the temple.
        let room = (self.ridge.outer - root.at.x.abs()).max(0.0) + OVERSHOOT;
        let length = (self.reach * self.share(root)).min(room);
        // And nothing at all if there is not enough room to be worth growing:
        // see [`LEAST_WORTH`].
        if length < self.reach * LEAST_WORTH {
            return 0.0;
        }
        length
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let length = self.length(root);
        let travel = length * along;
        let run = self.run(root);
        // **The clump follows the ridge's own curve rather than a straight tangent
        // to it**, which is what makes the inner hairs sweep up and the tail sweep
        // down by the right amount without either being a constant of its own: the
        // height the line has climbed over this much travel IS the curve.
        //
        // Then whatever droop asked for, which is a deliberate tilt and so is NOT
        // part of the line. And less the little height `run` carries of its own,
        // being a direction in a tilted tangent plane — taking that out is what
        // makes these two terms the whole of the vertical motion rather than most
        // of it.
        let from = self.ridge.height(self.ridge.along(root.at.x));
        let to = self
            .ridge
            .height(self.ridge.along(root.at.x.abs() + travel));
        let climb = Vec3::Y * (to - from - self.sag * travel - run.y * travel);
        // And standing off the skin as it goes, which is what `lie` is: a brow
        // that lies flat hugs the ridge, and one that does not is bushy.
        let stand = root.out * ((1.0 - self.lie.clamp(0.0, 1.0)) * travel);
        // And gathering toward the line as it goes. Linear in the travel, so it
        // adds no curvature and no stations: a straight clump aimed slightly
        // differently, which is all this is.
        let gather = Vec3::Y * (-self.offset(root) * GATHER * along.clamp(0.0, 1.0));
        root.at + root.out * LIFT + run * travel + climb + stand + gather
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`]. The fullest point is a little past the
        // middle, which is where a brow hair is fullest and which keeps the
        // ragged end of the row on the outer side where a tail belongs.
        let (base, _) = self.width(root);
        let from_middle = ((along.clamp(0.0, 1.0) - 0.55) / 0.55).abs().min(1.0);
        base * (1.0 - (1.0 - ENDS) * from_middle * from_middle)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        // Sized by how long the clump ended up, not by how long it asked to be:
        // see [`Self::stoutness`].
        let base = self.width * 0.5 * self.stoutness(root);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // **The card lies IN the plane of the skin, so its width runs across the
        // streak and its face turns outward** — which is where the light is and
        // where the camera is for a brow. The engine's default is across a FALL,
        // which on a brow is parallel to the spine: the case that put this on the
        // trait at all.
        //
        // A card has no cross-section, so #205's finding about which way round a
        // three-sided one sits retires with the sweep it was about (#204).
        root.out.cross(self.run(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ridge on the scale of a real head, and a root on it.
    fn ridge() -> Ridge {
        Ridge {
            inner: 0.011,
            outer: 0.050,
            level: 0.024,
            arch: 0.005,
            thick: 0.0063,
        }
    }

    /// A root a share of the way along the right brow, on a forward-facing
    /// surface.
    fn root(along: f32) -> Root {
        let ridge = ridge();
        let across = ridge.inner + ridge.span() * along;
        Root {
            at: Vec3::new(across, ridge.height(along), 0.09),
            out: Vec3::new(0.15, 0.25, 0.96).normalize(),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    fn brow(style: BrowStyle) -> Box<dyn Shape> {
        // The shape is built from a ridge rather than from a head here, so the
        // numbers under test are this file's own and not a body's.
        let slot = match style {
            BrowStyle::Natural => 0,
            BrowStyle::Thick => 1,
            BrowStyle::None => unreachable!("a bare brow has no shape to test"),
        };
        let ridge = ridge();
        Box::new(Brow {
            ridge,
            reach: ridge.span() * REACH[slot],
            width: WIDTH[slot],
            taper: TAPER[slot],
            thin: THIN[slot],
            lie: LIE[slot],
            sag: 0.0,
        })
    }

    #[test]
    fn a_brow_runs_along_the_ridge_and_not_down_the_face() {
        // **The defect this file exists for** (#205): `Fall` combs downhill, and
        // downhill on a brow ridge is over the eye, so the shipped brow was a row
        // of spikes hanging into the socket. Asserted as the direction a clump
        // actually travels — mostly sideways, and outward rather than toward the
        // nose, on both brows.
        let shape = brow(BrowStyle::Natural);
        for along in [0.15, 0.4, 0.7, 0.9] {
            for side in [1.0, -1.0] {
                let mut root = root(along);
                root.at.x *= side;
                let travelled = shape.at(&root, 1.0) - shape.at(&root, 0.0);
                let sideways = travelled.x.abs();
                assert!(
                    sideways > travelled.y.abs(),
                    "at {along} along, a brow clump travels {:.1} mm across and {:.1} mm up: it \
                     is hanging rather than combing",
                    sideways * 1000.0,
                    travelled.y * 1000.0
                );
                assert!(
                    travelled.x * side > 0.0,
                    "a brow clump on the {} side combs toward the nose",
                    if side > 0.0 { "right" } else { "left" }
                );
            }
        }
    }

    #[test]
    fn the_inner_half_sweeps_up_and_the_tail_sweeps_down() {
        // The arch doing the work of two constants: the hairs of a brow follow
        // the line, so they rise before the peak and fall after it. If this ever
        // reverses, the ridge's slope and the run direction have gone out of
        // sign with each other and the brow will read as combed the wrong way —
        // which is subtle enough in a render to survive a look.
        let shape = brow(BrowStyle::Natural);
        let rise = |along: f32| {
            let root = root(along);
            (shape.at(&root, 1.0) - shape.at(&root, 0.0)).y
        };
        assert!(
            rise(0.2) > 0.0,
            "the inner brow sweeps down: {:.2} mm",
            rise(0.2) * 1000.0
        );
        assert!(
            rise(0.9) < 0.0,
            "the tail sweeps up: {:.2} mm",
            rise(0.9) * 1000.0
        );
    }

    #[test]
    fn the_tail_is_the_thin_end() {
        // A brow tapers from its body to its tail and not the other way round.
        // Both length and section, since one profile drives them and the point
        // of that is they cannot disagree.
        let shape = brow(BrowStyle::Natural);
        let (body, tail) = (root(0.4), root(0.98));
        assert!(
            shape.length(&body) > shape.length(&tail) * 1.3,
            "the tail is {:.1} mm against the body's {:.1}",
            shape.length(&tail) * 1000.0,
            shape.length(&body) * 1000.0
        );
        assert!(
            shape.width(&body).0 > shape.width(&tail).0,
            "the tail is as wide as the body"
        );
    }

    #[test]
    fn the_wide_axis_is_never_along_the_spine() {
        // What put `across` on the trait. A ribbon whose width lies along its own
        // spine has no width, and the sweep does not complain — it substitutes an
        // arbitrary frame and the clump turns edge-on. Asserted as the angle
        // between the two, on both styles and all along the brow.
        for style in [BrowStyle::Natural, BrowStyle::Thick] {
            let shape = brow(style);
            for along in [0.05, 0.3, 0.68, 0.95] {
                for side in [1.0, -1.0] {
                    let mut root = root(along);
                    root.at.x *= side;
                    let spine = (shape.at(&root, 1.0) - shape.at(&root, 0.0)).normalize();
                    let across = shape.across(&root).normalize();
                    // The trait's contract is that it may not be PARALLEL, not
                    // that it must be perpendicular: the loft squares it against
                    // the local tangent at every station, so a card is never
                    // skewed by whatever tilt the gather and the arch put into the
                    // spine. What must not happen is the two being the same line,
                    // which leaves a card with no width at all.
                    assert!(
                        across.dot(spine).abs() < 0.5,
                        "at {along} along, the wide axis is {:.2} parallel to the spine",
                        across.dot(spine).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn thick_is_fuller_than_natural_and_costs_the_same() {
        // The bet of this issue, as an assertion: the two styles differ in what
        // they look like and not in what they cost. If a future edit buys Thick
        // its fullness with clumps instead of with section, the budget's dearest
        // corner moves and this is where that shows.
        let (natural, thick) = (brow(BrowStyle::Natural), brow(BrowStyle::Thick));
        let at = root(0.4);
        assert!(
            thick.width(&at).0 > natural.width(&at).0 * 1.4,
            "thick is {:.1} mm wide against natural's {:.1}",
            thick.width(&at).0 * 2000.0,
            natural.width(&at).0 * 2000.0
        );
        assert!(
            thick.width(&at).1 > natural.width(&at).1,
            "thick's tip is no blunter than natural's"
        );
        let cut = Cut::default();
        assert_eq!(
            BrowStyle::Thick.clumps(&cut, Follicle::Brows),
            BrowStyle::Natural.clumps(&cut, Follicle::Brows),
            "the two brow styles no longer cost the same number of clumps"
        );
    }

    #[test]
    fn droop_raises_and_lowers_a_brow_rather_than_hanging_it() {
        // Every region reads `Cut::droop` as how far hair hangs; a brow does not
        // hang, so here it is the difference between a raised brow and a heavy
        // one. Asserted through the axis rather than the field, because the
        // mapping — half the axis being the ridge as measured — is the part a
        // reader would get wrong.
        let head = |droop: f32| Brow {
            sag: (droop - 0.5) * 2.0 * SAG,
            ..Brow {
                ridge: ridge(),
                reach: ridge().span() * REACH[0],
                width: WIDTH[0],
                taper: TAPER[0],
                thin: THIN[0],
                lie: LIE[0],
                sag: 0.0,
            }
        };
        let tilt = |droop: f32| {
            let (shape, at) = (head(droop), root(0.4));
            (shape.at(&at, 1.0) - shape.at(&at, 0.0)).y
        };
        assert!(
            tilt(1.0) < tilt(0.5) && tilt(0.5) < tilt(0.0),
            "droop does not order a brow from heavy to raised: {:.2}, {:.2}, {:.2} mm",
            tilt(1.0) * 1000.0,
            tilt(0.5) * 1000.0,
            tilt(0.0) * 1000.0
        );
    }
}
