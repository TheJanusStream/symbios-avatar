//! The upper lip's own catalogue.
//!
//! # Why a whisker is not a brow, though it looks like one
//!
//! Both run sideways along a line on the face, so this file starts where
//! [`brows`](super::brows) ended: streaks combed out from the midline, gathered
//! onto the line the mask was cut around, thin at both ends so a row of them
//! reads as one mass rather than as a row of objects.
//!
//! What is different is underneath. A brow's line sits in the middle of a ridge
//! with skin above and below it; a moustache's sits on a patch whose floor is a
//! MOUTH. Ten millimetres below the nostrils is lip, and a millimetre past that
//! is the parting the jaw opens along — so the one thing this catalogue may not
//! do, at any setting of any axis on any head, is put a hair below the
//! vermilion.
//!
//! **So the clearance is a construction and not a check.** A whisker sweeps down
//! as it runs — that is what a moustache does — but by a share of the room its
//! own root stands in above the floor, never by a fixed drop. A clump rooted
//! high sweeps a long way and a clump rooted on the vermilion sweeps not at all,
//! which is both the correct behaviour and a guarantee: see `SAG`'s ceiling of
//! four fifths, which leaves the last fifth of every clump's room unspent.
//! Nothing here clamps, and nothing needs to be checked afterwards — though
//! `a_moustache_stays_out_of_the_mouth_it_grows_over` checks it anyway, against
//! the mouth's own cut seam rather than against this file's arithmetic.
//!
//! # Talk: bound as the skin under it
//!
//! A moustache's whiskers lie along the skin, and since #475 each of their
//! vertices is bound as the skin nearest it (`clump::loft`'s
//! `Binding::Under`), blended across the nearest triangle so a card stretches
//! no more than the skin under it does. Over the upper lip that skin is the skull's, and the
//! corners' near them, so the moustache rides the upper lip and the lower lip
//! moves AWAY from it as the jaw opens. The ends that run down beside the
//! mouth toward the chin lie on skin the jaw carries, and open with it as the
//! chin's beard does: bound like the root and handed to the head, as the other
//! regions' hair is, a clump laid down that skin would stay where the closed
//! mouth was.
//!
//! # The outline (#475)
//!
//! Every style grows on the same `Outline` the paint is masked by: a floor
//! that follows the vermilion down to the mouth's corners, a ceiling under the
//! nose, and a width from a Chaplin to ends that run down beside the mouth.
//! The clearance is the same construction over that floor: a clump sweeps a
//! share of the room over the floor under its root, which only falls outward,
//! and a clump beside the mouth below its corners runs down rather than out,
//! held off the ends' own bottom by the same share. Past the lips, the chevron
//! and the pencil turn down as well - a moustache's ends turning down at the
//! corners - and the handlebar does not: its ends carry on past the outline
//! and curl up, from the top of the turned-down ends on a wide width. The
//! pencil's line is a share of the way from the floor under it to the ceiling,
//! so it curves down with the vermilion. The sculpted chevron walks its
//! columns from each one's own floor down to the corners' height, and keeps
//! width `0`'s outline at every width above it (the owner's decision, see
//! `MoustacheStyle::widest`): its ends turn down to the corners and stop,
//! one solid, rather than running on beside the mouth onto skin the jaw
//! carries. Its flare carries the top of the moustache past the outline as
//! before.
//!
//! **The count furnishes the lip, and the ends are paid for on top** (#475's
//! fixer): a region's clump count is fixed whatever its area, so the ends took
//! their roots from the lip until the count was read as the lip's own density
//! (the clumps a style asks for across 0.24.5's band, at the height the band
//! under the nose has) and the ends' as 0.24.5's density by area: see
//! `Outline::spread_on`.
//!
//! # The three styles, and what separates them
//!
//! One shape with different numbers, as the brows are: [`MoustacheStyle::Chevron`]
//! is the full patch, [`MoustacheStyle::Handlebar`] is the same with its outer
//! clumps carrying on past the corners and curling up, and
//! [`MoustacheStyle::Pencil`] is the same gathered hard onto a line just above
//! the vermilion, thin and few. The handlebar is this milestone's first style
//! whose guides leave the surface on purpose, and its own axis is how far.
//!
//! [`MoustacheStyle::Sculpted`] keeps the same one rule as a solid chevron
//! drawn by the helmet family's facial walk (`hair::shell::face`, #349): every
//! ray it is walked along leaves from its rim's own floor, so no vertex is
//! below the vermilion, and its `flare` carries its ends past the corners and
//! turns them up.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, moustache::Outline};
use super::super::shell::face::{Sculpt, Sculpted};
use super::{Cut, SCULPTED_PAINT, Style, clumps_for};
use crate::plan::scaled;

/// The base styles of the upper lip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum MoustacheStyle {
    /// Nothing is grown here: the lip is painted, or bare.
    #[default]
    None,
    /// A full moustache over the whole patch, sweeping down and out.
    ///
    /// The one style with no axis of its own, for the same reason a crop has
    /// none: a chevron is what the shared four already describe.
    Chevron,
    /// The same, with the ends carried past the mouth's corners and turned up.
    Handlebar {
        /// How far the ends sweep, `0` barely past the corners and `1` a full
        /// waxed curl.
        #[serde(with = "crate::plan::scaled")]
        sweep: f32,
    },
    /// A thin line along the top of the vermilion.
    Pencil {
        /// How high the line sits, `0` on the lip's own border and `1` halfway
        /// to the nostrils.
        #[serde(with = "crate::plan::scaled")]
        ride: f32,
    },
    /// A solid sculpted chevron riding the upper lip, never below the
    /// vermilion (#349).
    ///
    /// Keeps the classic outline of `width` `0` at every width above it, its
    /// ends turning down to the mouth's corners and no further (#475, the
    /// owner's decision): drawn down beside the mouth its ends read as two
    /// flat slabs beside the band, and one solid round the corner would tear
    /// there as the jaw opens, the skin below the corners being the jaw's.
    /// Widths from `-1` to `0` narrow it as they narrow every style.
    Sculpted {
        /// How far its ends flare, `0` a chevron and `1` a handlebar turned up.
        #[serde(with = "crate::plan::scaled")]
        flare: f32,
    },
}

impl MoustacheStyle {
    /// Every name this build writes, in declaration order.
    ///
    /// **What a reader checks a name against before it trusts one** (#351). A
    /// record from a newer build can name a style this one has never heard
    /// of, and [`crate::hair::HairRecord`] reads such a name as `none` and
    /// keeps it for the rewrite rather than refusing the whole avatar.
    /// `tests/lexicon.rs` holds this list to the lexicon's knownValues.
    pub const NAMES: &'static [&'static str] =
        &["none", "chevron", "handlebar", "pencil", "sculpted"];
}

/// How far out along the lip one clump runs at full length, as a share of the
/// patch's own half-width.
///
/// One entry per style, in the order the enum declares them.
///
/// **A share of the patch and not millimetres**, for the reason every length in
/// the follicle module is one: a 9 mm streak is half a moustache on one face and
/// a third of one on another.
///
/// A little over a third, so that the clumps a full patch can afford overlap
/// about three deep along it and the row reads continuous — the same reading
/// [`brows`](super::brows)'s own reach was tuned to. The pencil runs longer
/// because it has fewer clumps to cover the same width with.
///
/// Provenance: **tuned by render**.
const REACH: [f32; 3] = [0.38, 0.62, 0.55];

/// How wide one is at the root, in metres.
///
/// **The lever that replaces count**, as everywhere in this milestone: a wider
/// card costs exactly what a narrow one does. A moustache is coarser hair than a
/// brow and sits on a patch about twice as deep, so these are up on
/// [`brows`](super::brows)'s.
///
/// Provenance: **tuned by render**.
const WIDTH: [f32; 3] = [0.0038, 0.0042, 0.0026];

/// What share of that is left at the tip.
///
/// Provenance: **tuned by render**.
const TAPER: [f32; 3] = [0.30, 0.34, 0.20];

/// Where in the band each style's mass sits, `0` at the vermilion and `1` at the
/// nostrils.
///
/// A chevron and a handlebar fill the patch, so they gather onto its middle. A
/// pencil is a line above the lip's own border, which is the low end of this and
/// what its own axis moves.
///
/// Provenance: **tuned by render**, against the anatomy each is named for.
const LINE: [f32; 3] = [0.46, 0.46, 0.16];

/// How much of its offset from that line a clump closes over its own length.
///
/// **What turns a scatter of streaks into one moustache.** Roots land
/// anywhere in the band's ten millimetres, and a streak running straight out
/// from wherever it started stays there — which reads as three stacked bars
/// rather than as one moustache.
///
/// The pencil gathers hardest, because a pencil moustache IS a line: it is the
/// only one of the three whose whole character is that its hairs agree about
/// where they are.
///
/// Provenance: **tuned by render**.
const GATHER: [f32; 3] = [0.15, 0.18, 0.88];

/// How far a clump may run past the patch's outer edge, as a share of it.
///
/// A chevron stops about where the paint does. A handlebar is the style whose
/// whole point is that it does not — its ends carry past the mouth's corners,
/// which is where its own curl happens — and a pencil ends at the corner because
/// a pencil that overshot would be a handlebar drawn thin.
///
/// Provenance: **tuned by render**.
const OVERSHOOT: [f32; 3] = [0.06, 0.55, 0.02];

/// How flat against the skin the clumps lie, `1` flat and `0` standing out.
///
/// A moustache stands off the lip rather more than a brow stands off its ridge —
/// that is most of what makes it read as bristle rather than as paint — and the
/// pencil is the one that lies almost flat, being nearly the painted layer with
/// a few hairs on it.
///
/// Provenance: **tuned by render**.
const LIE: [f32; 3] = [0.86, 0.84, 0.96];

/// How many clumps each style asks for at full density, as a share of the shared
/// count.
///
/// **The pencil asks for a third and that is the style, not a saving**: a pencil
/// moustache is a few hairs on a painted line, and drawing it with a full
/// patch's worth of clumps would be drawing a thin chevron.
///
/// **A half at #475, and back to a third**: the outline's ends turn down
/// past the mouth's corners and take about a third of the patch's roots, so a
/// third of the count left the line over the lip with about a fifth of a full
/// patch's clumps - 34 of a pencil's vertices over the mouth on the default
/// head, where the guard asks for more than 40. A half kept the line as
/// furnished as it was, and was a symptom's fix: the region's count is now
/// read as the lip's own density whatever the ends take
/// (`Outline::spread_on`), so the pencil asks for its third again.
///
/// Provenance: **derived** from what each style is, **sized by the budget**
///.
const CROWD: [f32; 3] = [1.0, 1.0, 0.34];

/// The most of a clump's clearance above the vermilion its own half-width may
/// take.
///
/// A little over half, so that a card is never so deep that there is no sweep
/// left to spend — the two together are what [`Whisker::room`] divides, and a
/// width that ate the whole clearance would leave a moustache that lay
/// perfectly level.
///
/// Provenance: **derived** from the split between the two.
const WIDTH_SHARE: f32 = 0.55;

/// The most of its own room above the vermilion a clump may sweep down through.
///
/// **The clearance, written as an inequality rather than as a clamp**.
/// The floor a whisker may not reach is the vermilion, so the drop is a share of
/// how far the clump's own seat stands above it — and at four fifths the last
/// fifth of that room is never spent, on any head, at any cut, for any root.
/// A clump rooted on the border sweeps nothing, which is also what a hair
/// growing out of the edge of a lip does.
///
/// Clamping to the floor instead would put a kink in every clump that reached
/// it, and the sampler spends stations on kinks: the guarantee would cost
/// triangles as well as reading as a hair with a corner in it.
///
/// Provenance: **derived** from the clearance it has to leave.
const SAG: f32 = 0.8;

/// The steepest a whisker sweeps down over its own length, in metres a metre.
///
/// Only binds where the floor is far below, which since #475 is beside the
/// mouth: a moustache's ends turning down there sweep at about fifty degrees,
/// and on the lip itself the room binds long before this does.
///
/// Provenance: **tuned by render**.
const STEEPEST: f32 = 1.2;

/// How much of that a moustache actually asks for at [`Cut::droop`] of one.
///
/// Droop on this region is how heavily the moustache sweeps toward the mouth
/// rather than how far it hangs, so the axis runs from a whisker held out level
/// to one combed down over the lip. Its top end is [`SAG`] itself.
///
/// Provenance: **tuned by render**.
const SWEEP_FROM: f32 = 0.25;

/// How high the handlebar's ends rise past the corner at a sweep of one, as a
/// share of the patch's half-width.
///
/// **Measured against the patch rather than in millimetres**, so a waxed end is
/// the same gesture on a small face and a large one.
///
/// Provenance: **tuned by render**.
const CURL: f32 = 0.62;

/// How far out the pencil's own axis may raise its line, in the band's share.
///
/// Half the band: at the top of the axis the pencil sits midway to the nostrils,
/// which is a pencil worn high, and past that it would not be a pencil.
///
/// Provenance: **derived** from what the axis is named for.
const RIDE: f32 = 0.34;

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Only a floor against the degenerate, and the same one the brows settled on
/// after learning that shortness is not what makes a stub read badly — a section
/// that follows the clump's own length is.
///
/// Provenance: **derived** from what the render can resolve.
const LEAST_WORTH: f32 = 0.08;

/// How thin a clump is at each of its ends, as a share of its middle.
///
/// A leaf rather than a wedge: the overlaps have no ends in them, so the union
/// is one mass with a ragged edge. See [`Shape::width_at`], whose default is the
/// wedge.
///
/// Provenance: **carried** from [`brows`](super::brows), whose render settled it
///.
const ENDS: f32 = 0.3;

impl Style for MoustacheStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        if let Self::Sculpted { flare } = self {
            return Some(Box::new(Sculpted(Sculpt::Moustache { flare: *flare })));
        }
        let slot = self.slot()?;
        let outline = head.outline();
        let lip = outline.lip;
        // **A narrow length axis, as the brows have** (#205): a moustache hair
        // is a moustache hair, and the shared floor of a quarter would leave a
        // three-millimetre streak — a row of arrowheads on a lip. A record still
        // asks for a sparse or a heavy moustache through `density` and
        // `thickness`.
        let length = 0.75 + 0.5 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        let line = match self {
            Self::Pencil { ride } => LINE[slot] + RIDE * ride.clamp(0.0, 1.0),
            _ => LINE[slot],
        };
        let curl = match self {
            Self::Handlebar { sweep } => CURL * sweep.clamp(0.0, 1.0),
            _ => 0.0,
        };
        Some(Box::new(Whisker {
            outline,
            // A share of the lip a whisker grows along, not of the outline's
            // whole extent (#475): the ends that turn down past the corners
            // widen the outline and lengthen no hair.
            reach: lip.half.min(outline.reach) * REACH[slot] * length,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            line,
            gather: GATHER[slot],
            overshoot: OVERSHOOT[slot],
            lie: LIE[slot],
            sag: SAG * (SWEEP_FROM + (1.0 - SWEEP_FROM) * cut.droop.clamp(0.0, 1.0)),
            curl,
            turns: !matches!(self, Self::Handlebar { .. }),
        }))
    }

    fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize {
        let Some(slot) = self.slot() else {
            return 0;
        };
        ((clumps_for(cut, follicle) as f32) * CROWD[slot]).round() as usize
    }

    fn sanitize(&mut self) {
        match self {
            Self::None | Self::Chevron => {}
            Self::Handlebar { sweep } => *sweep = scaled::quantize(sweep.clamp(0.0, 1.0)),
            Self::Pencil { ride } => *ride = scaled::quantize(ride.clamp(0.0, 1.0)),
            Self::Sculpted { flare } => *flare = scaled::quantize(flare.clamp(0.0, 1.0)),
        }
    }

    fn paint_floor(&self) -> Option<f32> {
        matches!(self, Self::Sculpted { .. }).then_some(SCULPTED_PAINT)
    }
}

impl MoustacheStyle {
    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    ///
    /// One place the order is written down, so a new style is a variant, an arm
    /// here, and one entry in each table rather than seven chances to misalign.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None | Self::Sculpted { .. } => None,
            Self::Chevron => Some(0),
            Self::Handlebar { .. } => Some(1),
            Self::Pencil { .. } => Some(2),
        }
    }

    /// The widest the region's `width` is read at under this style: `+1`,
    /// the whole slider, for every style but [`Self::Sculpted`], which keeps
    /// width `0`'s shape at every width above it.
    ///
    /// **The owner's decision** (#475, 2026-11-15): a wide sculpted moustache
    /// was the band and two flat solids down the sides of the mouth, and read
    /// as three planks; one solid round the corner would tear there as the jaw
    /// opens, the skin beside the mouth below the corners being the jaw's and
    /// the band's the skull's. So a sculpted moustache is the classic outline
    /// whose ends turn down to the corners' height, one solid, while the
    /// painted moustache and the grown styles still run down beside the mouth.
    /// A one-piece wide solid is a follow-up of its own.
    pub(crate) fn widest(self) -> f32 {
        match self {
            Self::Sculpted { .. } => 0.0,
            Self::None | Self::Chevron | Self::Handlebar { .. } | Self::Pencil { .. } => 1.0,
        }
    }
}

/// One whisker: a streak running out along the lip, sweeping toward the mouth
/// without ever reaching it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Whisker {
    /// The outline this grew on, which is also where its floor is (#475).
    outline: Outline,
    /// How far out along the lip one clump runs at full length, in metres.
    reach: f32,
    /// How wide one is at the root, in metres.
    width: f32,
    /// What share of the section is left at the tip.
    taper: f32,
    /// Where in the band the style's mass sits, in the band's own share.
    line: f32,
    /// How much of its offset from that a clump closes over its length.
    gather: f32,
    /// How far a clump may run past the patch's outer edge, as a share of it.
    overshoot: f32,
    /// How flat against the skin it lies.
    lie: f32,
    /// What share of its room above the vermilion it sweeps down through.
    sag: f32,
    /// How high its end rises past the corner, as a share of the half-width.
    curl: f32,
    /// Whether the clumps rooted beside the mouth, past the lips, run down
    /// rather than out: every style's but the handlebar's, whose ends curl up
    /// past the corners instead (#475).
    turns: bool,
}

impl Whisker {
    /// Which way this clump runs along the skin: outward, toward the corner.
    ///
    /// **Sideways only, with the rise and the fall left out on purpose** — the
    /// same separation [`brows`](super::brows) had to make. How much a clump
    /// climbs or sweeps is stated once in [`Shape::at`], and a direction that
    /// also carried it would mean the two were added and one of them subtracted
    /// again, which is how a droop axis comes to do nothing at all.
    fn run(&self, root: &Root) -> Vec3 {
        let out = root.out;
        let side = if root.at.x < 0.0 { -1.0 } else { 1.0 };
        // **Round the face, not along world X** (#206). A lip is not flat: by
        // the corner of a mouth the skin's own normal has turned to face
        // sideways, and world X minus its normal component is nothing at all
        // there — so the direction fell back to the NORMAL and the clump ran
        // straight out of the cheek. The sheet showed it as black blades
        // standing in the air beside the face, which is the same shape of defect
        // the brows' own `across` had (#205) and for the same reason: a
        // direction that degenerates does not fail, it substitutes.
        //
        // The azimuthal tangent cannot degenerate against a normal that is
        // roughly radial, which on a face it is, and it is also what a moustache
        // actually does: the hairs follow the lip round toward the cheek rather
        // than pointing at a wall.
        let azimuth = root.at.x.atan2(root.at.z);
        let round = Vec3::new(azimuth.cos(), 0.0, -azimuth.sin()) * side;
        let round = (round - out * round.dot(out)).normalize_or(round);
        // **And down, beside the mouth below its corners** (#475): an end that
        // runs down to the chin is hair combed down, not out into the cheek.
        // Down along the skin, which beside a mouth faces forward and out, so
        // the tangent cannot degenerate there.
        let down = self.down(root);
        if down <= 0.0 {
            return round;
        }
        let fall = (Vec3::NEG_Y - out * Vec3::NEG_Y.dot(out)).normalize_or(Vec3::NEG_Y);
        (round * (1.0 - down) + fall * down).normalize_or(fall)
    }

    /// Where this clump's body sits, in head-local metres.
    ///
    /// The gathered height: where the clump has arrived by its tip, rather than
    /// where its root happened to land. **The floor's clearance is measured from
    /// the LOWER of this and the root**, because a clump rooted below the line
    /// climbs toward it and one rooted above it descends — and the room a sweep
    /// may spend is the room the clump has at its lowest, not at either end
    /// alone.
    ///
    /// **The line follows the outline** (#475): a share of the way from the
    /// floor under the root to the ceiling over it, so a pencil's line curves
    /// down with the vermilion to the corners rather than running level.
    fn seat(&self, root: &Root) -> f32 {
        let floor = self.outline.floor(root.at.x);
        let ceiling = self.outline.ceiling(root.at.x).max(floor);
        let line = floor + (ceiling - floor) * self.line;
        root.at.y + (line - root.at.y) * self.gather.clamp(0.0, 1.0)
    }

    /// How much this clump runs down rather than out, `0` to `1` (#475).
    ///
    /// Down beside the mouth below its corners, where the ends of a wide
    /// moustache run to the chin; and, for every style but the handlebar, down
    /// past the lips at any height, which is a moustache's ends turning down
    /// at the corners rather than sweeping out into the cheek.
    fn down(&self, root: &Root) -> f32 {
        let below = self.outline.down(root.at);
        if self.turns {
            below.max(self.outline.beside(root.at.x))
        } else {
            below
        }
    }

    /// How far this clump's SPINE stands above the floor under its root at its
    /// lowest, in metres.
    ///
    /// The floor is the outline's: the vermilion's top over the lips, falling
    /// toward the corners, and the ends' bottom beside the mouth (#475). It
    /// only falls outward, and no clump runs inward, so the floor under the
    /// root is the highest the clump passes over.
    fn clear(&self, root: &Root) -> f32 {
        (root.at.y.min(self.seat(root)) - self.outline.floor(root.at.x)).max(0.0)
    }

    /// How much of that is left for the sweep, once the card's own half-width is
    /// paid for.
    ///
    /// **A card is a ribbon and its width here is VERTICAL**, so a spine that
    /// clears the vermilion is not a whisker that does: the card hangs half its
    /// width below its own spine. Measured on the built head, a full chevron's
    /// spine stayed above the line and its lower edge was 2.1 mm under it —
    /// hair drawn on the red of a lip, with no paint beneath it, which is the
    /// one thing this epic's two layers may not do.
    ///
    /// So the width comes out of the clearance first (see [`WIDTH_SHARE`]) and
    /// the sweep spends a share of what is left. Both together are still less
    /// than the room, which is the guarantee.
    fn room(&self, root: &Root) -> f32 {
        (self.clear(root) - self.width(root).0).max(0.0)
    }

    /// How much of the full section and length this root gets.
    ///
    /// The mask's own weight, square-rooted for the reason every region does it:
    /// the last tenth of an edge should keep some substance rather than
    /// collapsing, so the patch's rim reads as hair thinning out and not as hair
    /// stopping.
    fn share(&self, root: &Root) -> f32 {
        root.weight.clamp(0.0, 1.0).sqrt()
    }

    /// How much of the full section this clump gets.
    ///
    /// Its own length as a share of the full reach, so a clump is as thick as it
    /// is long: a short one cut off by the corner is the fine hair a moustache
    /// ends in rather than a lozenge sitting off it.
    fn stoutness(&self, root: &Root) -> f32 {
        self.length(root) / self.reach.max(f32::EPSILON)
    }
}

impl Shape for Whisker {
    fn length(&self, root: &Root) -> f32 {
        // Never further out than the style lets it go: a clump rooted near the
        // corner has only the room that is left, or it hangs in the air beside
        // the cheek. The handlebar's own room is most of a half-width past the
        // patch, which is what its ends are.
        let lateral = (self.outline.lip.half * (1.0 + self.overshoot) - root.at.x.abs()).max(0.0);
        // A clump running down beside the mouth has the ends' own bottom as its
        // room instead (#475), and spends no more of it than `SAG` lets a sweep:
        // its fall is `down` of its run's own drop, so its length is held to
        // the room over that. **And no more of the lateral room than its run
        // still spends sideways** (#475's fixer): a clump half turned down
        // runs half out, and held only to the vertical room it ran past the
        // patch's edge by as much again.
        let down = self.down(root);
        let room = if down > 0.0 {
            let drop = self.run(root).y.abs().max(f32::EPSILON) * down;
            let vertical = SAG * (root.at.y - self.outline.bottom).max(0.0) / drop;
            let sideways = if down < 1.0 {
                lateral / (1.0 - down)
            } else {
                f32::MAX
            };
            vertical.min(sideways)
        } else {
            lateral
        };
        let length = (self.reach * self.share(root)).min(room);
        if length < self.reach * LEAST_WORTH {
            return 0.0;
        }
        length
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let length = self.length(root);
        let travel = length * along;
        let run = self.run(root);
        // Gathering onto the style's own line as it runs, which is what makes a
        // scatter of streaks read as one moustache. Linear in the travel, so it
        // adds no curvature and no stations: a straight clump aimed slightly
        // differently, which is all this is.
        let gather = (self.seat(root) - root.at.y) * along;
        // And sweeping down toward the mouth — by a share of the room this clump
        // has above the vermilion, never by a distance. See [`SAG`]: this is the
        // whole of the clearance, and it is why nothing below clamps.
        //
        // Quadratic in the travel, because a whisker sweeps hardest at its end,
        // which is also what keeps the row's inner half lying along the lip.
        //
        // No steeper than [`STEEPEST`] over its own length, for a clump whose
        // room is the ends' bottom beside the mouth rather than a lip a few
        // millimetres under it (#475): a sweep of the whole of that would be a
        // clump stood on its end. A share of the room still, so still a
        // construction.
        let sink = (self.room(root) * self.sag).min(length * STEEPEST) * along * along;
        // Standing off the skin as it goes, which is what `lie` is: a moustache
        // lying flat is a painted one, and one that does not is bristle. The
        // engine's own root lift rides in the same direction and so is written
        // here with it rather than beside it.
        let stand = root.out * (LIFT + (1.0 - self.lie.clamp(0.0, 1.0)) * travel);
        // **And every one of those terms gives up its own rise, so that `gather`
        // and `sink` are the WHOLE of the vertical motion rather than most of
        // it** — which is what makes the clearance a construction. It is
        // [`brows`](super::brows)'s discipline about `run` extended to the lift
        // and the standoff, and it had to be: the skin of an upper lip faces
        // forward and DOWN, so a millimetre and a half of lift plus a
        // millimetre of standoff along that normal is two and a half
        // millimetres of drop nobody budgeted for. Measured on the built head,
        // that put a full chevron 2.9 mm below the vermilion — under its own
        // paint — while the synthetic roots this file's other test uses, whose
        // normals point straight out, saw nothing at all.
        //
        // **Except down a mouth's side** (#475), where the run's own fall IS
        // the clump's motion: there `down` of the run's drop is kept, and the
        // gather and the sweep give way to it. See [`Shape::length`] for how
        // that fall is held off the ends' bottom.
        let down = self.down(root);
        let rise = Vec3::Y * ((1.0 - down) * (gather - sink - run.y * travel) - stand.y);
        // And the handlebar's own end: past the patch's outer edge the clump
        // turns up and away, and nowhere else does it do anything at all.
        //
        // **Smoothstepped in how far past the corner it has come**, so the curl
        // starts at nothing exactly where the patch ends — a linear rise would
        // put a corner in every clump that crossed the edge, and the whole row
        // would crease along one line.
        //
        // Smoothed rather than SQUARED, which was the first cut and did nothing
        // visible: the furthest any clump gets is about a quarter of a
        // half-width past the edge, and a quarter squared is a sixteenth, so a
        // handlebar's ends rose 0.8 mm where the axis had asked for five. A
        // smoothstep has the same flat start and reaches its full value inside
        // the overshoot the style actually uses.
        // How far past the patch's outer edge the clump has actually come,
        // which with a run that curves round the face is not the same as how
        // far it has travelled.
        let past = (self.outline.lip.along(root.at.x + run.x * travel) - 1.0).max(0.0);
        let curl = Vec3::Y
            * (self.curl
                * self.outline.lip.half
                * crate::face::smooth(past / self.overshoot.max(f32::EPSILON)));
        root.at + run * travel + rise + stand + curl
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`]. Fullest a little past the middle,
        // which keeps the ragged end of the row on the outer side where a
        // moustache's own ends belong.
        let (base, _) = self.width(root);
        let from_middle = ((along.clamp(0.0, 1.0) - 0.55) / 0.55).abs().min(1.0);
        base * (1.0 - (1.0 - ENDS) * from_middle * from_middle)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        // Never wider than the room its own spine has above the vermilion: see
        // [`Whisker::room`]. A clump rooted close to the lip's border is the
        // fine hair a moustache ends in downward as well as outward, which is
        // both what the anatomy does and what keeps the card off the lip.
        let base = (self.width * 0.5 * self.share(root) * self.stoutness(root))
            .min(self.clear(root) * WIDTH_SHARE);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // The card lies IN the plane of the skin, so its width runs across the
        // streak and its face turns outward — which is where the light is and
        // where the camera is for a lip. The engine's default is across a fall,
        // which here is parallel to the spine (#205).
        root.out.cross(self.run(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull};
    use crate::hair::follicle::FollicleParams;
    use crate::hair::follicle::moustache::Lip;
    use crate::{Archetype, Avatar, AvatarRecord};

    /// The regions of one built head, which a style is fitted against.
    fn head() -> Follicles {
        head_of_width(0.0)
    }

    /// The same, with the moustache's width axis at `width` (#475).
    fn head_of_width(width: f32) -> Follicles {
        let record = AvatarRecord::new("Moustache", Archetype::default());
        let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let mut params = FollicleParams::default();
        params.moustache.width = width;
        Follicles::of(&avatar.rig, &skull, &canon, &params)
    }

    /// A root in the lip's band, a share of the way up it and out along it.
    ///
    /// **Its normal points forward and DOWN, which is what an upper lip's skin
    /// does**. The first cut of this used a level `+Z` and could
    /// therefore not see the defect the built head had: every term that rides
    /// the normal — the root lift, the standoff — carries a drop with it there,
    /// and on a lip that drop is millimetres. A synthetic root with a level
    /// normal is a synthetic lip, and a test on one measures the fixture.
    fn root(lip: &Lip, up: f32, out: f32) -> Root {
        root_facing(lip, up, out, -0.55)
    }

    /// The same, with the normal's downward tilt named.
    fn root_facing(lip: &Lip, up: f32, out: f32, tilt: f32) -> Root {
        Root {
            at: Vec3::new(lip.half * out, lip.height(up), 0.09),
            out: Vec3::new(0.0, tilt, 1.0).normalize(),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    /// Every style, at a middling axis.
    fn every_style() -> [MoustacheStyle; 3] {
        [
            MoustacheStyle::Chevron,
            MoustacheStyle::Handlebar { sweep: 0.6 },
            MoustacheStyle::Pencil { ride: 0.4 },
        ]
    }

    /// How far apart two readings of one edge may be by rounding, in metres.
    const ROUNDING: f32 = 1.0e-6;

    #[test]
    fn no_whisker_reaches_the_vermilion_at_any_setting() {
        // **The claim this file is built around, and it is asserted at the
        // extremes rather than at a default** (#206): the floor is a MOUTH, and
        // a moustache that reaches it at one cut of one style on one head is a
        // moustache in somebody's lip. The construction that guarantees it is
        // [`SAG`]'s unspent fifth, so what this measures is that the
        // construction holds — including for roots rooted ON the border, where
        // the room is nothing and the sweep must be nothing too.
        //
        // **The floor is the outline's since #475**: the vermilion's top as it
        // curves down to the mouth's corners, and the ends' bottom beside the
        // mouth, so a card's lower edge is asked against the floor under its
        // own root - the highest it passes over, no clump running inward - and
        // against the lips themselves, the lens the outline keeps them out
        // with. At every width from a Chaplin to a horseshoe, with roots down
        // the ends beside the mouth as well as across the lip.
        let mut grown = 0usize;
        for width in [-1.0f32, 0.0, 1.0] {
            let head = head_of_width(width);
            let outline = head.outline();
            let lip = outline.lip;
            let mut roots = Vec::new();
            for up in [0.0f32, 0.15, 0.5, 0.85, 1.0] {
                for out in [0.0f32, 0.3, 0.6, 0.9] {
                    // **Over a range of surface tilts**, because the drop the
                    // normal carries is what the built head caught and the
                    // fixture did not. A lip faces forward and down by about
                    // this much; the ends of the sweep are a flat one and a
                    // steep one.
                    for tilt in [0.0f32, -0.3, -0.55, -0.9] {
                        let mut root = root_facing(&lip, up, out, tilt);
                        // Seated on the floor under it where the floor falls
                        // below the midline's band, so the roots at `up` 0 are
                        // ON the border wherever they are.
                        let floor = outline.floor(root.at.x);
                        root.at.y = floor + (lip.nostrils.max(floor) - floor) * up;
                        roots.push(root);
                    }
                }
            }
            // And down the ends beside the mouth, from the corners' height to
            // the ends' bottom, across the end.
            for down in [0.0f32, 0.3, 0.6, 0.95, 1.0] {
                for across in [0.0f32, 0.5, 1.0] {
                    let x = outline.reach + (lip.half - outline.reach).max(0.0) * across;
                    // Never under the floor there, which a Chaplin holds level
                    // at the vermilion's height (#475's fixer): its patch has
                    // no ends, and a root below its floor is one the mask
                    // grows none of.
                    let y = (outline.side() + (outline.bottom - outline.side()) * down)
                        .max(outline.floor(x));
                    roots.push(Root {
                        at: Vec3::new(x, y, 0.07),
                        out: Vec3::new(0.6, -0.1, 0.8).normalize(),
                        weight: 1.0,
                        skin: Default::default(),
                    });
                }
            }
            for style in every_style() {
                for droop in [0.0f32, 0.5, 1.0] {
                    let cut = Cut {
                        droop,
                        length: 1.0,
                        thickness: 1.0,
                        density: 1.0,
                    };
                    let shape = style
                        .shape(&cut, Follicle::Moustache, &head)
                        .expect("a grown style has a shape");
                    for root in &roots {
                        if shape.length(root) <= 0.0 {
                            continue;
                        }
                        grown += 1;
                        let floor = outline.floor(root.at.x);
                        for step in 0..=24 {
                            let along = step as f32 / 24.0;
                            // **The card's lower EDGE, not its spine.** A
                            // whisker's card is a ribbon whose width lies
                            // vertically, so half of it hangs below the curve
                            // `at` describes - and a test on the spine passes
                            // while the moustache is on the lip (#206).
                            let at = shape.at(root, along);
                            let edge = at.y - shape.width_at(root, along);
                            assert!(
                                // A micron for rounding: a root ON the floor
                                // sits on the lens's own edge, which the two
                                // readings put a few nanometres apart.
                                edge >= floor - ROUNDING && outline.clear(at.x, edge) >= -ROUNDING,
                                "a {style:?} whisker at width {width} rooted at ({:+.1}, \
                                 {:+.1}) mm at droop {droop} reaches {:.2} mm below the floor \
                                 under it, {:.2} mm outside the lips",
                                root.at.x * 1000.0,
                                root.at.y * 1000.0,
                                (floor - edge) * 1000.0,
                                outline.clear(at.x, edge) * 1000.0
                            );
                        }
                    }
                }
            }
        }
        // And that it looked at anything: a sweep whose every root is declined
        // asserts nothing at all (#210).
        assert!(
            grown > 1500,
            "only {grown} of the sweep's roots grew a whisker to measure"
        );
    }

    #[test]
    fn a_moustache_stays_out_of_the_mouth_it_grows_over() {
        // **The same claim as the one above, made against the mouth itself
        // rather than against this file's own arithmetic** (#206). The test
        // above measures the construction on synthetic roots at the extremes;
        // this one grows the record's own moustache on a built head and
        // compares it with the seam `face::mouth` actually CUT — which is a
        // curve dipping toward the corners, and is not a number this file knows.
        //
        // **And it is the whole Talk claim too.** The parting's upper seam is
        // held by the skull and its lower seam by the jaw, coincident at rest;
        // hair binds rigidly to the head joint. So a moustache rides the upper
        // lip, the lower lip moves AWAY from it when the jaw opens, and rest is
        // the worst case. Clearing the seam here clears it at every jaw angle,
        // which is why there is no posed variant of this test.
        for style in every_style() {
            let mut record = AvatarRecord::new("Talking", Archetype::default());
            record.hair = crate::hair::HairRecord {
                moustache: crate::hair::Tress {
                    style,
                    cut: Cut {
                        length: 1.0,
                        droop: 1.0,
                        density: 1.0,
                        thickness: 1.0,
                    },
                    ..Default::default()
                },
                ..crate::hair::HairRecord::default()
            };
            let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
            let Some(mouth) = &avatar.parts.mouth else {
                panic!("the default body grew no openable mouth to clear");
            };
            let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
            let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
            let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
            let hair = grown(&record, &avatar, &follicles);
            assert!(
                !hair.is_empty(),
                "a {style:?} at a full cut grew nothing at all"
            );
            let origin = follicles.origin();
            let seam: Vec<Vec3> = mouth
                .upper
                .iter()
                .map(|at| avatar.parts.body.positions[*at as usize] - origin)
                .collect();
            let mut checked = 0usize;
            let mut closest = f32::MAX;
            for at in &hair {
                // The seam under this hair, if there is any: past the mouth's
                // own corners there is none, which is exactly where a
                // handlebar's ends are.
                let Some(under) = seam
                    .iter()
                    .filter(|point| (point.x - at.x).abs() < 0.002)
                    .map(|point| point.y)
                    .fold(None, |worst: Option<f32>, y| {
                        Some(worst.map_or(y, |worst: f32| worst.max(y)))
                    })
                else {
                    continue;
                };
                checked += 1;
                closest = closest.min(at.y - under);
                assert!(
                    at.y > under,
                    "a {style:?} hangs {:.2} mm below the mouth's own cut at x {:+.1} mm",
                    (under - at.y) * 1000.0,
                    at.x * 1000.0
                );
            }
            // And that it looked at the moustache rather than past it: a filter
            // that matches nothing passes every assertion inside it (#210).
            //
            // A count rather than a share, because the share is a property of
            // the STYLE: a handlebar's ends are past the mouth's own corners by
            // design, where there is no seam to clear, so more than half of its
            // vertices legitimately have nothing under them. Asking for a share
            // would be asking the handlebar to stop being one.
            assert!(
                checked > 40,
                "only {checked} of {} {style:?} vertices had any mouth under them",
                hair.len()
            );
            let lip = follicles.lip();
            let (lo, hi) = hair
                .iter()
                .fold((f32::MAX, f32::MIN), |s, at| (s.0.min(at.y), s.1.max(at.y)));
            println!(
                "{style:?}: {checked} vertices, closest {:.2} mm above the cut; band {:+.1}..{:+.1}, hair {:+.1}..{:+.1}",
                closest * 1000.0,
                lip.vermilion * 1000.0,
                lip.nostrils * 1000.0,
                lo * 1000.0,
                hi * 1000.0
            );
        }
    }

    /// How far over the vermilion a sculpted moustache's lowest point may sit and
    /// still be riding the lip, in metres: 1.1 to 1.5 mm measured on three heads
    /// (#349), and a floor further off than this would guard nothing.
    const RIDES_THE_LIP: f32 = 0.003;

    #[test]
    fn a_sculpted_moustache_stays_out_of_the_mouth_it_grows_over() {
        // **The module's one rule, asked of the SOLID** (#349): nothing below
        // the vermilion. A card whisker keeps it by sweeping a share of its own
        // room; the sculpted chevron keeps it by construction - every ray it is
        // walked along leaves from the rim's own floor, raised by the most its
        // bottom bevel can carry, so no vertex is lower than the vermilion plus
        // half the mask's fade. Asked of the built mesh on three heads at every
        // flare, against the vermilion (stricter than the seam, which sits 8 to
        // 11 mm below it) AND against the mouth's own cut seam.
        //
        // **And with the jaw open, because this is the region the acceptance
        // says must NOT open** (the beard does, the moustache does not). A card
        // moustache binds to the head; the solid binds as the skin under it
        // does, which measured is the head's on every vertex - so the posed
        // solid is asserted to be the rest one.
        //
        // **And at `width` `+1`, where it is width `0`'s solid** (#475, the
        // owner's decision; `MoustacheStyle::widest`): its ends turn down to
        // the corners' height and stop, so the solid at `+1` is asserted to be
        // the one at `0`, vertex for vertex, and to keep every rule above
        // there, the jaw's included. The wide solid #475 first drew walked
        // two more solids down beside the mouth onto skin the jaw carries, and
        // fails here: on the default head at `+1` 944 vertices against 448,
        // 408 of them below the corners.
        use crate::anim::Pose;
        use glam::Quat;
        for seed in [None, Some(42), Some(7)] {
            for flare in [0.0f32, 0.5, 1.0] {
                // The solid at width `0`, which the one at `+1` must be.
                let mut classic: Option<Vec<Vec3>> = None;
                for width in [0.0f32, 1.0] {
                    let mut record = AvatarRecord::new("Sculpted", Archetype::default());
                    if let Some(seed) = seed {
                        record.reroll(seed);
                    }
                    let mut regions = record.hair.regions;
                    regions.moustache.width = width;
                    record.hair = crate::hair::HairRecord {
                        regions,
                        moustache: crate::hair::Tress {
                            style: MoustacheStyle::Sculpted { flare },
                            ..Default::default()
                        },
                        ..crate::hair::HairRecord::bald()
                    };
                    record.sanitize();
                    let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
                    let mouth = avatar.parts.mouth.as_ref().expect("an openable mouth");
                    let skull =
                        Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
                    let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
                    // The regions as the build cuts them, which is width `0`'s
                    // moustache at `+1`.
                    let follicles =
                        Follicles::of(&avatar.rig, &skull, &canon, &record.hair.grown_regions());
                    let outline = follicles.outline();
                    let origin = follicles.origin();
                    let hair = avatar
                        .parts
                        .hair
                        .as_ref()
                        .expect("a sculpted moustache grows");
                    let positions = &hair.mesh.positions;
                    assert!(
                        positions.len() > 100,
                        "seed {seed:?} flare {flare}: the solid has {} vertices",
                        positions.len()
                    );
                    match &classic {
                        None => classic = Some(positions.clone()),
                        Some(classic) => assert!(
                            classic == positions,
                            "seed {seed:?} width {width} flare {flare}: the solid is not width 0's: \
                         {} vertices against {}, {} of them below the mouth's corners",
                            positions.len(),
                            classic.len(),
                            positions.iter().filter(|at| at.y < outline.side()).count()
                        ),
                    }
                    // **Against the floor under each vertex since #475**: the
                    // vermilion's top as it curves down to the corners, and the
                    // lips' own lens, rather than one level line.
                    let lowest = positions
                        .iter()
                        .map(|at| at.y - outline.floor(at.x))
                        .fold(f32::MAX, f32::min);
                    let inside = positions
                        .iter()
                        .map(|at| outline.clear(at.x, at.y))
                        .fold(f32::MAX, f32::min);
                    assert!(
                        lowest >= 0.0 && inside >= 0.0,
                        "seed {seed:?} width {width} flare {flare}: the solid reaches {:.2} mm below \
                     the floor under it and {:.2} mm into the lips",
                        -lowest * 1000.0,
                        -inside * 1000.0
                    );
                    let seam: Vec<Vec3> = mouth
                        .upper
                        .iter()
                        .map(|at| avatar.parts.body.positions[*at as usize] - origin)
                        .collect();
                    let mut checked = 0usize;
                    for at in positions {
                        let Some(under) = seam
                            .iter()
                            .filter(|point| (point.x - at.x).abs() < 0.002)
                            .map(|point| point.y)
                            .reduce(f32::max)
                        else {
                            continue;
                        };
                        checked += 1;
                        assert!(
                            at.y > under,
                            "seed {seed:?} width {width} flare {flare}: the solid hangs {:.2} mm below \
                         the mouth's cut at x {:+.1} mm",
                            (under - at.y) * 1000.0,
                            at.x * 1000.0
                        );
                    }
                    assert!(
                        checked > 40,
                        "seed {seed:?} flare {flare}: only {checked} vertices had any mouth under them"
                    );
                    // And that the rule BINDS: the solid rides the lip down to within
                    // a few millimetres of the vermilion (1.1 to 1.5 mm measured),
                    // so a floor asserted over it is not a floor over thin air.
                    assert!(
                        lowest < RIDES_THE_LIP,
                        "seed {seed:?} flare {flare}: the solid's lowest point is {:.2} mm over the \
                     floor under it, which is not riding the lip",
                        lowest * 1000.0
                    );
                    // The jaw opens and the moustache does not move.
                    let tip = (0..avatar.rig.len())
                        .find(|&tip| {
                            avatar.rig.joints[tip].marker
                                && avatar.rig.joints[tip]
                                    .parent
                                    .is_some_and(|at| avatar.rig.joints[at].marker)
                        })
                        .expect("a humanoid has a jaw");
                    let pivot = avatar.rig.joints[tip]
                        .parent
                        .expect("the tip hangs off the pivot");
                    let mut open = Pose::rest(&avatar.rig);
                    open.rotations[pivot] = Quat::from_rotation_x(20f32.to_radians());
                    let posed = |pose: &Pose| {
                        avatar
                            .posed(pose, 0.0)
                            .into_iter()
                            .find(|mesh| mesh.kind == crate::MeshKind::Hair)
                            .expect("the moustache is drawn")
                            .mesh
                            .positions
                    };
                    let (rest, opened) = (posed(&Pose::rest(&avatar.rig)), posed(&open));
                    let moved = rest
                        .iter()
                        .zip(&opened)
                        .map(|(one, two)| one.distance(*two))
                        .fold(0.0f32, f32::max);
                    assert!(
                        moved < 0.0001,
                        "seed {seed:?} width {width} flare {flare}: the jaw at 20 degrees moved the \
                     moustache {:.2} mm",
                        moved * 1000.0
                    );
                }
            }
        }
    }

    #[test]
    fn a_moustache_lies_on_its_skin_and_out_of_the_mouth_with_the_jaw_open() {
        // **Talk, at every width** (#475). The ends of a wide moustache run
        // down beside the mouth onto skin the jaw carries, so a moustache is
        // no longer the region that rides the skull alone: it has to open with
        // the mouth where its skin does, and stay out of the lips and the
        // parting while it does. Grown on the default head at a Chaplin, a
        // classic and a horseshoe, every style and the sculpted solid at both
        // ends of its flare, posed shut and with the jaw open 25 degrees (the
        // chin's own skin moves 44.7 mm there):
        //
        // - over the mouth, every vertex is above the mouth's own upper seam,
        //   posed (the existing guard's reading, now with the jaw open too);
        // - below the mouth's corners, every vertex is beside the mouth, out
        //   past the corners, posed: none in the parting;
        // - and no vertex moves off or into the skin under it as the jaw opens
        //   by more than [`OFF_THE_SKIN`]: none floating where the closed mouth
        //   was.
        //
        // - and at `+1` a grown style's ends ARE beside the mouth below the
        //   corners, and move with the jaw by more than [`ENDS_OPEN`];
        // - while the sculpted solid has NO vertex below the corners at any
        //   width (#475, the owner's decision: it keeps width `0`'s outline,
        //   `MoustacheStyle::widest`). Its wide ends walked as solids of
        //   their own read 680 vertex readings below the corners at `0.5`,
        //   the farthest moving 10.1 mm with the jaw.
        //
        // Measured: worst 0.43 mm of change off the skin, the closest 4.30 mm
        // over the seam, 4.10 mm out past a corner; at `+1` 523 to 1604 vertex
        // readings below the corners, the farthest moving 32.4 to 34.0 mm; no
        // edge grown past the skin under it by more than 1.20 times. Before
        // #475 no vertex reached below the corners at any width. Bound like
        // their roots and handed to the head, as every other region's hair is,
        // a horseshoe chevron's ends read 2.83 mm off the skin with the jaw
        // open; bound station by station to the nearest skin vertex, as #475
        // first bound them, an edge grew 2.62 times past its skin.
        //
        // **At `0.5` as well** since #475's second fixer, where the critic of
        // #475's fixer saw cards tear: now worst 0.41 mm off the skin, 4.30 mm
        // over the seam, 3.87 mm past a corner, no edge past 1.13 times its
        // skin over the four widths. The sculpted solid bound as the skin its
        // columns' rays left read 1.30 there (5.24 mm grown to 7.18 over skin
        // that grew 1.05 times); its stations are now bound as the skin under
        // them (`shell::face`'s `loft`).
        //
        // **Since the owner's decision** the sculpted solid is width `0`'s at
        // `0.5` and `+1`: worst 0.41 mm off the skin, 5.16 mm over the seam,
        // 3.87 mm past a corner, no edge past 1.09 times its skin; the grown
        // styles at `+1` 378 to 1114 vertex readings below the corners, the
        // farthest moving 33.4 to 34.0 mm.
        use crate::anim::Pose;
        use glam::Quat;
        let styles = [
            MoustacheStyle::Chevron,
            MoustacheStyle::Handlebar { sweep: 0.6 },
            MoustacheStyle::Pencil { ride: 0.4 },
            MoustacheStyle::Sculpted { flare: 0.0 },
            MoustacheStyle::Sculpted { flare: 1.0 },
        ];
        let (mut drift, mut over, mut beside) = (0.0f32, f32::MAX, f32::MAX);
        let mut stretch = 1.0f32;
        for width in [-1.0f32, 0.0, 0.5, 1.0] {
            for style in styles {
                // How many vertices lie beside the mouth below its corners, and
                // how far the farthest of them moves as the jaw opens.
                let (mut down, mut opened) = (0usize, 0.0f32);
                let mut record = AvatarRecord::new("Talking", Archetype::default());
                let mut hair = crate::hair::HairRecord::bald();
                hair.regions.moustache.width = width;
                hair.moustache = crate::hair::Tress {
                    style,
                    cut: Cut {
                        length: 1.0,
                        droop: 1.0,
                        density: 1.0,
                        thickness: 1.0,
                    },
                    ..Default::default()
                };
                record.hair = hair;
                record.sanitize();
                let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
                let mouth = avatar.parts.mouth.as_ref().expect("an openable mouth");
                let rig = &avatar.rig;
                let tip = (0..rig.len())
                    .find(|&tip| {
                        rig.joints[tip].marker
                            && rig.joints[tip]
                                .parent
                                .is_some_and(|at| rig.joints[at].marker)
                    })
                    .expect("a humanoid has a jaw");
                let pivot = rig.joints[tip].parent.expect("the tip hangs off the pivot");
                // The body's faces near the mouth, by index, so the posed and
                // the rest surfaces are read over the same faces.
                let body = &avatar.parts.body;
                let centre = body
                    .positions
                    .iter()
                    .enumerate()
                    .filter(|(at, _)| mouth.welds.contains(&(*at as u32)))
                    .map(|(_, at)| *at)
                    .sum::<Vec3>()
                    * 0.5;
                let near: Vec<usize> = (0..body.face_count())
                    .filter(|&face| {
                        body.positions[body.faces[face][0] as usize].distance(centre) < 0.07
                    })
                    .collect();
                let faces = avatar
                    .posed(&Pose::rest(rig), 0.0)
                    .into_iter()
                    .find(|mesh| mesh.kind == crate::MeshKind::Hair)
                    .expect("the moustache is drawn")
                    .mesh
                    .faces;
                let pose = |degrees: f32| {
                    let mut pose = Pose::rest(rig);
                    pose.rotations[pivot] = Quat::from_rotation_x(degrees.to_radians());
                    let mut skin = body.clone();
                    skin.skin = avatar.parts.weights.vertices.clone();
                    let skin = pose.forward(rig).deform_mesh(rig, &skin);
                    let hair = avatar
                        .posed(&pose, 0.0)
                        .into_iter()
                        .find(|mesh| mesh.kind == crate::MeshKind::Hair)
                        .expect("the moustache is drawn")
                        .mesh
                        .positions;
                    (skin.positions, hair)
                };
                let off = |skin: &[Vec3], at: Vec3| {
                    near.iter()
                        .flat_map(|&face| {
                            let corners = &body.faces[face];
                            (1..corners.len() - 1).map(move |fan| {
                                [corners[0], corners[fan], corners[fan + 1]]
                                    .map(|corner| skin[corner as usize])
                            })
                        })
                        .map(|[a, b, c]| {
                            crate::dress::garment::nearest_on_triangle(at, a, b, c).distance(at)
                        })
                        .fold(f32::MAX, f32::min)
                };
                let (rest_skin, rest_hair) = pose(0.0);
                let (open_skin, open_hair) = pose(25.0);
                // **And no card stretches more than the skin under it does**
                // (#475's fixer): every edge of every card, its length with
                // the jaw open against its length shut, set against the same
                // ratio read on the skin under its two ends (each end's
                // nearest point on the skin shut, carried by its triangle as
                // the jaw opens). The skin beside the mouth below its corners
                // does stretch, three and four times down its length as the
                // jaw drops away from the skull, and hair lying on it with it;
                // bound station by station to the one nearest skin VERTEX, two
                // stations a millimetre apart took two vertices the jaw held
                // differently, and a card tore across the skin as the mouth
                // opened.
                let under = |at: Vec3| {
                    near.iter()
                        .flat_map(|&face| {
                            let corners = &body.faces[face];
                            (1..corners.len() - 1)
                                .map(move |fan| [corners[0], corners[fan], corners[fan + 1]])
                        })
                        .map(|corners| {
                            let points = corners.map(|corner| rest_skin[corner as usize]);
                            let on = crate::dress::garment::nearest_on_triangle(
                                at, points[0], points[1], points[2],
                            );
                            (on.distance_squared(at), corners, on, points)
                        })
                        .min_by(|one, two| one.0.total_cmp(&two.0))
                        .map(|(_, corners, on, points)| {
                            let shares = crate::hair::clump::barycentric(on, &points);
                            let carried = corners
                                .iter()
                                .zip(shares)
                                .map(|(corner, share)| open_skin[*corner as usize] * share)
                                .sum::<Vec3>();
                            (on, carried)
                        })
                        .expect("there is skin round the mouth")
                };
                let skin_of: Vec<(Vec3, Vec3)> = rest_hair.iter().map(|at| under(*at)).collect();
                for face in &faces {
                    for (at, &one) in face.iter().enumerate() {
                        let two = face[(at + 1) % face.len()];
                        let (one, two) = (one as usize, two as usize);
                        let shut = rest_hair[one].distance(rest_hair[two]);
                        let skin_shut = skin_of[one].0.distance(skin_of[two].0);
                        if shut < SHORTEST_EDGE || skin_shut < SHORTEST_EDGE {
                            continue;
                        }
                        let open = open_hair[one].distance(open_hair[two]);
                        let skin_open = skin_of[one].1.distance(skin_of[two].1);
                        let hair = open / shut;
                        let skin = skin_open / skin_shut;
                        // Grown past what the skin under it grows to, which is
                        // a tear; a card's width is one binding a station and
                        // keeps it where the skin round a mouth's corner
                        // spreads, and that is not one.
                        let ratio = hair / skin.max(1.0);
                        stretch = stretch.max(ratio);
                        assert!(
                            ratio <= STRETCH,
                            "{style:?} at width {width}: an edge {:.2} mm long shut is {:.2} mm \
                             with the jaw open, where the skin under it goes {skin:.2} times",
                            shut * 1000.0,
                            open * 1000.0
                        );
                    }
                }
                for (skin, hair) in [(&rest_skin, &rest_hair), (&open_skin, &open_hair)] {
                    let seam: Vec<Vec3> = mouth.upper.iter().map(|at| skin[*at as usize]).collect();
                    let corners = mouth.welds.map(|at| skin[at as usize]);
                    let reach = (corners[0].x - centre.x)
                        .abs()
                        .max((corners[1].x - centre.x).abs());
                    let level = corners[0].y.max(corners[1].y);
                    for at in hair.iter() {
                        let under = seam
                            .iter()
                            .filter(|point| (point.x - at.x).abs() < 0.002)
                            .map(|point| point.y)
                            .reduce(f32::max);
                        if let Some(under) = under {
                            over = over.min(at.y - under);
                            assert!(
                                at.y > under,
                                "{style:?} at width {width}: a vertex hangs {:.2} mm below the \
                                 mouth's seam at x {:+.1} mm",
                                (under - at.y) * 1000.0,
                                (at.x - centre.x) * 1000.0
                            );
                        } else if at.y < level {
                            down += 1;
                            beside = beside.min((at.x - centre.x).abs() - reach);
                            assert!(
                                (at.x - centre.x).abs() > reach,
                                "{style:?} at width {width}: a vertex below the mouth's corners \
                                 is in the mouth at x {:+.1} mm",
                                (at.x - centre.x) * 1000.0
                            );
                        }
                    }
                }
                for (rest, open) in rest_hair.iter().zip(&open_hair) {
                    opened = opened.max(rest.distance(*open));
                    let moved = (off(&open_skin, *open) - off(&rest_skin, *rest)).abs();
                    drift = drift.max(moved);
                    assert!(
                        moved <= OFF_THE_SKIN,
                        "{style:?} at width {width}: a vertex moves {:.2} mm off the skin under \
                         it as the jaw opens",
                        moved * 1000.0
                    );
                }
                // And that a sculpted moustache keeps width `0`'s outline
                // above it (#475, the owner's decision): no vertex below the
                // mouth's corners, posed shut or open, at any width.
                if matches!(style, MoustacheStyle::Sculpted { .. }) {
                    assert!(
                        down == 0,
                        "{style:?} at width {width}: {down} vertex readings below the mouth's \
                         corners, the farthest moving {:.1} mm as the jaw opens; a sculpted \
                         moustache's ends stop at the corners",
                        opened * 1000.0
                    );
                } else if width >= 1.0 {
                    // And that a horseshoe IS one: its ends are beside the
                    // mouth, on the skin the jaw carries, and open with it.
                    println!(
                        "{style:?} at width {width}: {down} vertex readings beside the mouth, the \
                         farthest moving {:.1} mm with the jaw",
                        opened * 1000.0
                    );
                    assert!(
                        down > 40 && opened > ENDS_OPEN,
                        "{style:?} at width {width}: {down} vertex readings below the mouth's \
                         corners, the farthest moving {:.1} mm as the jaw opens",
                        opened * 1000.0
                    );
                }
            }
        }
        println!(
            "worst {:.2} mm off the skin with the jaw open, closest {:.2} mm over the seam, {:.2} \
             mm out past a corner, an edge stretched {stretch:.2} times",
            drift * 1000.0,
            over * 1000.0,
            beside * 1000.0
        );
    }

    /// The most any edge of a moustache's cards may stretch or shrink as the
    /// jaw opens 25 degrees past what the skin under it does, either way
    /// round; see
    /// `a_moustache_lies_on_its_skin_and_out_of_the_mouth_with_the_jaw_open`.
    const STRETCH: f32 = 1.25;

    /// The shortest edge that stretch is read on, in metres: a card's ends
    /// meet in edges of nothing, whose ratio is noise.
    const SHORTEST_EDGE: f32 = 1.0e-5;

    /// How many hair vertices each grown style kept over the middle half of
    /// the mouth before #475's outline, on the default head and seed 14, at
    /// `width` `0` and `+1`: (seed, width, chevron, handlebar, pencil). See
    /// `a_grown_moustache_keeps_its_lip_furnished_at_every_width`.
    const FURNISHED: [(Option<i64>, f32, [usize; 3]); 4] = [
        (None, 0.0, [91, 64, 24]),
        (None, 1.0, [63, 52, 16]),
        (Some(14), 0.0, [94, 71, 28]),
        (Some(14), 1.0, [61, 46, 21]),
    ];

    #[test]
    fn a_grown_moustache_keeps_its_lip_furnished_at_every_width() {
        // **The upper lip is what a moustache IS**, and #475's outline gave
        // the ends that turn down past the corners and run down beside the
        // mouth their roots out of the lip's: a region's clump count was fixed
        // whatever its area, so a wide chevron's lip was left with a sixth of
        // its hair - wisps between two side tufts. So the density is guarded
        // directly: the vertices a full cut of each style grows over the
        // middle half of the mouth (between the corners, within half their
        // reach of the middle, above their height), at rest, against what the
        // same record grew before the outline ([`FURNISHED`]).
        //
        // Measured with the count read as the lip's own density
        // (`Outline::spread_on`): 138, 113 and 39 at `0` and 132, 111 and 38
        // at `+1` on the default head (chevron, handlebar, pencil), 139, 110
        // and 36, and 160, 122 and 49 on seed 14 - at least 1.29 of what each
        // kept before the outline. With the outline and the count as #475 left
        // them, seed 14's chevron kept 36 of 94 at `0` and 10 of 61 at `+1`,
        // its pencil 15 of 28 and 4 of 21; with this outline and the count
        // fixed, 71 of 94 and 37 of 61, 16 of 28 and 10 of 21. With #475's
        // second fixer's fuller band under the nose, its count read against
        // the lip under the foot and its density falling to the ends' by the
        // neutral nose's wings (`Outline::band`, `Outline::crowd`): 130, 106
        // and 40 at `0` and 125, 98 and 36 at `+1` on the default head, 150,
        // 115 and 42, and 199, 154 and 61 on seed 14 - at least 1.43; read
        // against the fuller band itself, seed 14's chevron kept 92 of 94 at
        // `0` and its pencil 23 of 28.
        use crate::anim::Pose;
        let styles = [
            MoustacheStyle::Chevron,
            MoustacheStyle::Handlebar { sweep: 0.6 },
            MoustacheStyle::Pencil { ride: 0.4 },
        ];
        let mut short = Vec::new();
        for (seed, width, before) in FURNISHED {
            for (style, before) in styles.into_iter().zip(before) {
                let mut record = AvatarRecord::new("Talking", Archetype::default());
                if let Some(seed) = seed {
                    record.reroll(seed);
                }
                let mut hair = crate::hair::HairRecord::bald();
                hair.regions.moustache.width = width;
                hair.moustache = crate::hair::Tress {
                    style,
                    cut: Cut {
                        length: 1.0,
                        droop: 1.0,
                        density: 1.0,
                        thickness: 1.0,
                    },
                    ..Default::default()
                };
                record.hair = hair;
                record.sanitize();
                let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
                let mouth = avatar.parts.mouth.as_ref().expect("an openable mouth");
                let body = &avatar.parts.body;
                let corners = mouth.welds.map(|at| body.positions[at as usize]);
                let centre = (corners[0] + corners[1]) * 0.5;
                let reach = (corners[0].x - centre.x)
                    .abs()
                    .max((corners[1].x - centre.x).abs());
                let level = corners[0].y.max(corners[1].y);
                let hair = avatar
                    .posed(&Pose::rest(&avatar.rig), 0.0)
                    .into_iter()
                    .find(|mesh| mesh.kind == crate::MeshKind::Hair)
                    .expect("the moustache is drawn")
                    .mesh
                    .positions;
                let middle = hair
                    .iter()
                    .filter(|at| (at.x - centre.x).abs() < reach * 0.5 && at.y > level)
                    .count();
                println!(
                    "seed {seed:?} width {width:+}: {style:?} {middle} over the middle of the \
                     mouth, {before} before the outline, {} in all",
                    hair.len()
                );
                if middle < before {
                    short.push(format!(
                        "seed {seed:?} width {width:+} {style:?}: {middle} of {before}"
                    ));
                }
            }
        }
        assert!(
            short.is_empty(),
            "a grown moustache keeps less hair over the middle of the mouth than it did before \
             the outline: {short:?}"
        );
    }

    /// How far a horseshoe's ends must move as the jaw opens 25 degrees, in
    /// metres; see
    /// `a_moustache_lies_on_its_skin_and_out_of_the_mouth_with_the_jaw_open`.
    const ENDS_OPEN: f32 = 0.010;

    /// How far a moustache's vertex may move off or into the skin under it as
    /// the jaw opens 25 degrees, in metres; see
    /// `a_moustache_lies_on_its_skin_and_out_of_the_mouth_with_the_jaw_open`.
    const OFF_THE_SKIN: f32 = 0.0015;

    /// The moustache one record grows on one built head, head-local.
    ///
    /// **From one stream in [`Follicle::ALL`]'s order**, which is what
    /// `Avatar::build` does — so these are the roots that shipped rather than a
    /// second sample from the same distribution, which is the form
    /// `examples/follicleaudit` takes it in.
    fn grown(record: &AvatarRecord, avatar: &Avatar, follicles: &Follicles) -> Vec<Vec3> {
        use crate::hair::Growth;
        use crate::hair::clump::{Bed, Sowing};
        use rand::SeedableRng;
        let bed = Bed {
            body: &avatar.parts.body,
            rig: &avatar.rig,
            weights: &avatar.parts.weights,
            follicles,
        };
        let mut stream = rand_pcg::Pcg64Mcg::seed_from_u64(record.seed as u64);
        let mut mine = Vec::new();
        for follicle in Follicle::ALL {
            let Some(sown) = record.hair.sowing(follicle, follicles) else {
                continue;
            };
            let mut growth = Growth::on(follicles.head);
            growth.grow(
                &bed,
                &Sowing {
                    follicle,
                    count: sown.clumps,
                    shape: sown.shape.as_ref(),
                    roots: Vec3::from_array(sown.roots),
                    tips: Vec3::from_array(sown.tips),
                },
                &mut stream,
            );
            if follicle == Follicle::Moustache {
                mine = growth.mesh.positions.clone();
            }
        }
        mine
    }

    #[test]
    fn only_the_handlebar_leaves_the_patch() {
        // The three styles are one shape with different numbers, so what makes
        // them different has to be measured rather than asserted by name. This
        // is the difference a person would point at: a handlebar has ENDS, past
        // the corners of the mouth, and the other two stop on the lip.
        let head = head();
        let lip = head.lip();
        let cut = Cut::default();
        let furthest = |style: MoustacheStyle| {
            let shape = style
                .shape(&cut, Follicle::Moustache, &head)
                .expect("a grown style has a shape");
            let root = root(&lip, 0.5, 0.85);
            (0..=16)
                .map(|step| lip.along(shape.at(&root, step as f32 / 16.0).x))
                .fold(0.0f32, f32::max)
        };
        let (chevron, handlebar, pencil) = (
            furthest(MoustacheStyle::Chevron),
            furthest(MoustacheStyle::Handlebar { sweep: 1.0 }),
            furthest(MoustacheStyle::Pencil { ride: 0.0 }),
        );
        assert!(
            handlebar > 1.1,
            "a handlebar reaches {handlebar:.2} of the patch's own half-width, which is not past \
             the corners"
        );
        assert!(
            chevron <= 1.1 && pencil <= 1.1,
            "a chevron reaches {chevron:.2} and a pencil {pencil:.2} of the half-width, and \
             neither is meant to leave the patch"
        );
    }

    #[test]
    fn the_handlebars_sweep_axis_turns_its_ends_up() {
        // A variant carrying an axis owes a render that the axis does something,
        // and a test that it does it in order (#204's convention for the scalp's
        // five). Measured at the tip of a clump rooted near the corner, which is
        // the only place a handlebar differs from a chevron at all.
        let head = head();
        let lip = head.lip();
        let root = root(&lip, 0.5, 0.85);
        let tip = |sweep: f32| {
            MoustacheStyle::Handlebar { sweep }
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a handlebar has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            tip(1.0) > tip(0.5) && tip(0.5) > tip(0.0),
            "the sweep axis does not order a handlebar: {:+.1}, {:+.1}, {:+.1} mm",
            tip(1.0) * 1000.0,
            tip(0.5) * 1000.0,
            tip(0.0) * 1000.0
        );
    }

    #[test]
    fn the_pencils_ride_axis_lifts_its_line_off_the_lip() {
        let head = head();
        let lip = head.lip();
        // Rooted at the top of the band, so what is measured is where the clump
        // GATHERS to rather than where it started: a pencil's line is the whole
        // of what it is.
        let root = root(&lip, 1.0, 0.4);
        let seat = |ride: f32| {
            MoustacheStyle::Pencil { ride }
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a pencil has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            seat(1.0) > seat(0.5) && seat(0.5) > seat(0.0),
            "the ride axis does not order a pencil: {:+.1}, {:+.1}, {:+.1} mm",
            seat(1.0) * 1000.0,
            seat(0.5) * 1000.0,
            seat(0.0) * 1000.0
        );
        // And that it is still a line low on the lip at the top of its axis,
        // rather than a chevron: half the band is what the axis is worth.
        assert!(
            seat(1.0) < lip.height(0.6),
            "a pencil worn as high as it goes seats at {:.2} of the band, which is a chevron",
            (seat(1.0) - lip.vermilion) / lip.span()
        );
    }

    #[test]
    fn a_whisker_gathers_toward_its_line_rather_than_staying_where_it_landed() {
        // #205's lesson, which this file starts from: roots land anywhere in the
        // band and streaks that run straight out from where they landed read as
        // three stacked bars. Measured as the spread of the tips against the
        // spread of the roots.
        let head = head();
        let lip = head.lip();
        for style in every_style() {
            let shape = style
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a grown style has a shape");
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for up in [0.05f32, 0.3, 0.55, 0.8, 1.0] {
                let tip = shape.at(&root(&lip, up, 0.35), 1.0).y;
                lo = lo.min(tip);
                hi = hi.max(tip);
            }
            let spread = (hi - lo) / lip.span();
            assert!(
                spread < 0.8,
                "a {style:?}'s tips are spread over {:.0}% of the band from roots spread over \
                 95% of it, which is a row of bars rather than a moustache",
                spread * 100.0
            );
        }
    }
}
