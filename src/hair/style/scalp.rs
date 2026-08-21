//! The hair of the head, and its five styles.
//!
//! # Why a scalp lock is not a brow's
//!
//! A brow lies on the skin for its whole length. A scalp lock does something no
//! other region's does: it is **held up by the skull** for the first part of its
//! travel and hangs only once the head has fallen away beneath it. The old shell
//! system had this in one sentence — a lock "follows the skull down to the
//! hairline before falling free" — and every attempt since to approximate it with
//! a direction has failed the same way.
//!
//! [`Fall`](super::super::clump::Fall) leaves along the tangent plane at the
//! root and goes straight. On a head of 90 mm radius a straight 46 mm lock ends
//! 12 mm off the surface, so it covers scalp for a centimetre and then hangs in
//! the air. That is the whole of why a hundred and fifty of them read as strings
//! over a bare scalp: each one is a bristle standing off the head, not a lock
//! lying on it.
//!
//! So a lock here **walks the measured profile**. It descends the head at its own
//! azimuth, staying on the surface while the surface is still supporting it, and
//! hangs at the widest radius it has passed once the head starts coming back in.
//! That last rule is the physics in one line: hair is held out by whatever it has
//! draped over, which is why it stands off a neck and why a curtain clears a
//! shoulder.
//!
//! # Sheets, not strings, and it is CHEAPER
//!
//! The other half of the fix is width. A lock is a **card**: wide across the
//! scalp and thin off it, which at three sides is a flat sliver whose two long
//! faces turn outward. Coverage then comes from width rather than from count —
//! and width is free where count is fourteen triangles a time. Fifty wide cards
//! cover a head that a hundred and fifty strings left bare, for fewer triangles
//! than the strings cost.
//!
//! # What varies with azimuth, and why every style needs it
//!
//! **Fall length varies with azimuth**, carried over from the shell era, where it
//! was learned the hard way: uniform-length hair falls off the brow straight down
//! the face, which is a curtain and not a hairstyle. Real hair is long at the back
//! and a fringe at the front, and that difference is most of what makes a head of
//! hair look like one. It is the `fringe` of this file's own `Sheet` here, and the styles differ in it
//! more than in anything else.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Seating, Shape};
use super::super::follicle::{Follicle, Follicles};
use super::{Cut, Style, clumps_for};
use crate::plan::scaled;

/// The base styles of the hair of the head.
///
/// **Each variant carries the one axis that is its own**, and none carries an
/// axis that belongs to another: a tail height is not a thing a crop has, and a
/// shared field for it would be a number every reader has to know to ignore. The
/// four axes every style shares — length, coarseness, density, how far it hangs —
/// are [`Cut`]'s, so a variant's own field is the fifth and last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum ScalpStyle {
    /// Nothing is grown here: the scalp is painted, or bare.
    #[default]
    None,
    /// Short hair lying along the skull, from a buzz to a mop.
    ///
    /// The one style with no axis of its own: a crop is what the shared four
    /// already describe, and its whole character is that it stays on the head.
    Crop,
    /// A curtain to about the jaw, with a fringe over the brow.
    Bob {
        /// How much shorter the front is than the sides, `0` an even curtain and
        /// `1` a fringe well above the brow.
        #[serde(with = "crate::plan::scaled")]
        fringe: f32,
    },
    /// Hair past the shoulders, weighted to the back.
    Long {
        /// How much longer the back is than the sides, `0` even all round and
        /// `1` strongly back-weighted.
        #[serde(with = "crate::plan::scaled")]
        weight: f32,
    },
    /// Hair drawn back against the skull into a tail.
    TiedBack {
        /// Where the tail is gathered, `0` at the nape and `1` high on the crown.
        #[serde(with = "crate::plan::scaled")]
        tail: f32,
    },
    /// Coiled hair, volume forward.
    Curly {
        /// How tight the coil is, `0` a loose wave and `1` a tight curl.
        #[serde(with = "crate::plan::scaled")]
        curl: f32,
    },
}

/// How far a lock hangs PAST the hairline at full length, in metres.
///
/// One entry per style, in the order the enum declares them, and the whole
/// difference between a crop and a curtain.
///
/// **Past the hairline, not from the root**. A card covers the scalp
/// first — the crown to the hairline is 100 mm of travel on a default head and
/// the style has no say in it — and this is only what is left over to hang. The
/// numbers were the total when the cap was a share of them, and left where they
/// were they gave a crop a 54 mm fringe: dreadlocks over the eyes.
///
/// Provenance: **tuned by render**, against the anatomy each style is
/// named for — a crop stops on the head, a bob reaches the jaw, a long reaches
/// past the shoulder.
///
/// **The tied-back entry is the TAIL's length from the knot** (#316), since
/// that is the only part of the style that hangs; the hair that is not
/// gathered lies to the hairline and stops.
const REACH: [f32; 5] = [0.022, 0.130, 0.330, 0.160, 0.120];

/// How wide a lock is at its root at the coarsest cut, in metres.
///
/// **The lever that replaces count**, and the reason this file exists: a card
/// costs exactly what a string costs. Two centimetres is about a finger's width
/// of hair, which is what a lock of a real haircut is; below one it reads as a
/// bristle whatever else is right.
///
/// Curly is widest, because coiled hair genuinely clumps into fewer, fatter
/// locks — which is also what pays for its extra stations.
///
/// **The three HANGING styles were widened when [`CROWD`] was re-sized**,
/// and the split between which styles needed it and which did not is the
/// interesting half. Cutting the counts by measured card cost left a crop and a
/// tail unchanged on the render — a card that lies ON the skull tiles it, so
/// three times cover and eight times cover look the same — and left a bob, a
/// curtain and a coil visibly thin, with scalp showing through the crown. A
/// hanging card does not tile anything: past the hairline it is one lock in a
/// curtain, and the count IS the density of that curtain.
///
/// So the count came down for the budget and the width went up for the mass,
/// which is the flanks' own lesson in the region where it is least obvious
///: width is free and a card is four triangles a segment. Measured, the
/// whole catalogue still lands inside the scalp's share of the budget with these
/// at nearly twice their old widths.
///
/// Provenance: **tuned by render**, **widened by render against a
/// re-sized count**.
const WIDTH: [f32; 5] = [0.034, 0.046, 0.056, 0.036, 0.070];

/// What share of its width a lock keeps at its tip.
///
/// Real hair gathers: locks that fall exactly as they were rooted stay parallel
/// all the way down and read as a comb.
///
/// **Nearly to a point, because a card's END is a flat cap**. At a third of
/// the width the fringe was a row of square teeth over the forehead — the caps
/// themselves, 14 mm across and facing the camera. Only the hanging part tapers
/// (see `Sheet::width_at`), so this is what the fringe line is made of.
///
/// Provenance: **carried** in spirit from the shell's gather, **tuned by render**.
const TAPER: [f32; 5] = [0.10, 0.16, 0.14, 0.14, 0.30];

/// How many locks each style asks for at full density, as a share of the shared
/// count.
///
/// **Curly asks for fewer and that is not a saving, it is the style**: coiled
/// hair clumps into fewer, fatter locks than straight hair does. It is also what
/// makes a coil affordable, since a curve pays for its own stations.
///
/// **Sized against the measured cost of each style's own card**, because a
/// count is set in cards and paid for in triangles — the rule every catalogue
/// in this crate has had to learn. Sized against a crop, with a budget test
/// wearing a crop too, nothing costs the four styles that are not one.
/// Measured at the greediest cut a record may ask for, one card is
///
/// ```text
///   crop  15.3    bob  18.3    long  24.4    tied  42.0    curly  64.9
/// ```
///
/// triangles, because a tail's card walks the whole skull before it gathers and
/// a ringlet pays a station for every millimetre its coil departs from a chord.
/// At a flat count that is a scalp costing four times what its own budget was
/// set to — which is how a dearest legal record lands thousands of triangles
/// over the WebGL2 target with every budget test passing.
///
/// So each style is granted the count that spends what a crop spends. The crop
/// itself is the anchor and does not move; nothing about a card's width, reach,
/// taper or coil moves either, because those are tuned by render against the
/// shape of a card and not against its price.
///
/// **Coverage is checked before a count is cut, not after.** The scalp holds
/// 34.5% of a head's own surface — 485 cm² on the
/// default body — and these grant it between two and three times that in card
/// area at the default cut, against the one-and-a-half the flanks were judged
/// to need. Width is free and a card is four triangles a segment, so a style
/// that wants more mass takes it in [`WIDTH`], not here.
///
/// Provenance: **derived** from what each style is, **sized by the budget**
///, **re-sized by the measured cost of a card**.
const CROWD: [f32; 5] = [1.0, 0.83, 0.62, 0.36, 0.24];

/// How far a lock stands off the skull at no droop at all, in metres.
///
/// [`Cut::droop`] is what share of this is given up: at `1` the hair falls with
/// the ground and hugs the head, at `0` it stands off it. Eighteen millimetres of
/// lift over a lock's own travel is the difference between hair combed flat and
/// hair with body in it.
///
/// Provenance: **tuned by render**.
const VOLUME: f32 = 0.008;

/// How far round the head the fringe's shortening reaches.
///
/// In the cosine of the azimuth, so it follows the head's own curve: `1` is dead
/// ahead and this is about the temple. A fringe that reaches further is a bowl
/// cut, and one that reaches less is a tuft over the nose.
///
/// Provenance: **tuned by render**.
const FRONT: f32 = 0.35;

/// How far a coil swings either side of the lock's own fall, loose to tight, in
/// metres.
///
/// **A ringlet rather than a corkscrew, and the budget is what decided that.**
/// Cost here is path length and curvature, not extent: the sampler holds a drawn
/// spine within a millimetre of its curve, so a coil of amplitude `a` and
/// wavelength `λ` needs a station every `sqrt(8 × 0.001 × λ² / (4π²a))` metres. A
/// real tight curl — 8 mm through a 25 mm wavelength — works out at a station
/// every 4 mm, which is 37 stations and 218 triangles for ONE lock, and thirty of
/// those is more than the whole avatar's budget.
///
/// A big soft ringlet at 12 mm through 75 mm is 15 stations, reads as curly hair
/// at the framing a head is judged at, and is affordable. So this style is
/// ringlets; tight coils are not a thing this triangle count can draw, and
/// pretending otherwise would mean a variant that does not do what it says.
///
/// Provenance: **derived** from the sampler's own tolerance, **tuned by render**.
const SWING: [f32; 2] = [0.007, 0.017];

/// The wavelength of that coil over the lock's travel, loose to tight, likewise.
///
/// Provenance: **derived** with [`SWING`], from what a station costs.
const WAVE: [f32; 2] = [0.105, 0.062];

impl Style for ScalpStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        let slot = self.slot()?;
        let length = 0.25 + 0.75 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.65 + 0.7 * cut.thickness.clamp(0.0, 1.0);
        let hang = cut.droop.clamp(0.0, 1.0);
        // How the length varies round the head, which is the one thing every
        // style says differently. A share of the reach at the front, and a
        // multiple of it at the back.
        let (fringe, behind) = match self {
            Self::None => return None,
            // A crop is a crop all round: it is short enough that varying it
            // reads as a mistake rather than as a cut.
            Self::Crop => (0.85, 1.0),
            // A bob's fringe is its whole character, and its sides are its
            // length.
            Self::Bob { fringe } => (1.0 - 0.75 * fringe.clamp(0.0, 1.0), 1.05),
            // Long hair is back-weighted: the front is a face-framing length and
            // the back is the style.
            Self::Long { weight } => (0.45, 1.0 + 0.7 * weight.clamp(0.0, 1.0)),
            // Drawn back, so there is no fringe to speak of and the length is in
            // the tail.
            // Drawn back: what is not gathered lies to the hairline and stops
            // there, and what is gathered hangs one length from the knot.
            Self::TiedBack { .. } => (1.0, 1.0),
            // A curl frames the face: at nine tenths of its reach the
            // ringlets curtained the eyes (#316), and a coil does not get out
            // of the way on its own.
            Self::Curly { .. } => (0.5, 1.0),
        };
        let knot = self.knot(head);
        let curl = match self {
            Self::Curly { curl } => curl.clamp(0.0, 1.0),
            _ => 0.0,
        };
        Some(Box::new(Sheet {
            regions: head.clone(),
            reach: REACH[slot] * length,
            fringe,
            behind,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            volume: VOLUME * (1.0 - hang),
            knot,
            pull: if knot.is_some() { 1.0 } else { 0.0 },
            curl,
            part: match self {
                Self::Long { .. } => 1.0,
                _ => 0.0,
            },
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
            Self::None | Self::Crop => {}
            Self::Bob { fringe } => *fringe = scaled::quantize(fringe.clamp(0.0, 1.0)),
            Self::Long { weight } => *weight = scaled::quantize(weight.clamp(0.0, 1.0)),
            Self::TiedBack { tail } => *tail = scaled::quantize(tail.clamp(0.0, 1.0)),
            Self::Curly { curl } => *curl = scaled::quantize(curl.clamp(0.0, 1.0)),
        }
    }
}

impl ScalpStyle {
    /// Where a tail is knotted, if this style has one: behind the head at a
    /// height its own axis picks, head-local.
    fn knot(self, head: &Follicles) -> Option<Vec3> {
        match self {
            Self::TiedBack { tail } => {
                let skull = head.skull();
                let (throat, crown) = skull.throat_and_crown();
                let height = throat + (crown - throat) * (0.35 + 0.5 * tail.clamp(0.0, 1.0));
                Some(Vec3::new(
                    0.0,
                    height,
                    skull.depth_behind(height) * KNOT_STANDOFF,
                ))
            }
            _ => None,
        }
    }

    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    ///
    /// One place the order is written down, so a new style is a variant, an arm
    /// here, and one entry in each table rather than five chances to misalign.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Crop => Some(0),
            Self::Bob { .. } => Some(1),
            Self::Long { .. } => Some(2),
            Self::TiedBack { .. } => Some(3),
            Self::Curly { .. } => Some(4),
        }
    }
}

/// How far the knot sits off the back of the head, as a share of its reach there.
///
/// Just clear of the occiput, so the tail is behind the head rather than inside
/// it, and the locks arriving have somewhere to meet.
///
/// Provenance: **tuned by render**.
const KNOT_STANDOFF: f32 = 1.15;

/// The mask weight below which a card has left the scalp and is hanging.
///
/// A third: the mask's own fade is what thins the locks crossing a hairline, so
/// this only has to say which side of it a point is on, and the middle of the
/// fade is the honest place to put that.
///
/// Provenance: **derived** from the mask's own fade.
const EDGE: f32 = 0.35;

/// Over how much free hang a lock takes up its full volume and coil, in metres.
///
/// Hair lying on a scalp has neither: it is pressed against the head. Both come
/// on over the first few centimetres of hanging, which is what makes a crop hug
/// and a curtain swing.
///
/// Provenance: **tuned by render**.
const LOOSE: f32 = 0.045;

/// How wide a card is where it leaves the crown, as a share of its full width.
///
/// Not zero: cards converge at a whorl and a card that came to a point there
/// would leave the whorl itself bare, which is the one part of a head nothing
/// else covers. Not large either — at a third of full width, thirty-four cards
/// piled into a rosette of plates at the crown where the circumference they had
/// to cover was a tenth of what they were carrying.
///
/// Provenance: **tuned by render**.
const FAN: f32 = 0.10;

/// How far from the pole a card reaches its full width, in metres.
///
/// About the crown's own radius: the cards converge on the whorl and have to
/// share its circumference, and by this far out the circumference is wide
/// enough for all of them at full width.
///
/// Provenance: **tuned by render** (#316).
const FAN_OVER: f32 = 0.05;

/// How far the length of a lock's hang varies from card to card, as a share.
///
/// **Locks end where they end, not on a line** (#316). Cards that all hang
/// the same distance past the hairline end on one contour, and the contour
/// of tapered cards is a sawtooth: the fringe on the starting sheet. Real
/// locks stagger, and a crop's edge is feathered because its locks do.
///
/// Provenance: **tuned by render**.
const STAGGER: f32 = 0.30;

/// The least that stagger may be, in metres.
///
/// A share of a crop's ten-millimetre hang is a millimetre and a half, which
/// is under what the render resolves; a crop's edge feathers over about a
/// centimetre.
///
/// Provenance: **tuned by render**.
const STAGGER_LEAST: f32 = 0.008;

/// Over how much of the end of a hang a card tapers to its tip, in metres.
///
/// **The taper belongs to the END of a lock, not to its whole hang.** Tapered
/// from the hairline to the tip, a bob's card was a 46 mm isosceles triangle
/// 70 mm tall, and a row of those is a row of teeth; a lock keeps its width
/// and comes to its point over its last few centimetres.
///
/// Provenance: **tuned by render** (#316).
const TIP: f32 = 0.045;

/// How wide the band of mask weight is over which cards leave the scalp.
///
/// **A hairline is where locks STOP LYING DOWN, and they do not all stop at
/// once** (#316). One threshold put every card's first free station on one
/// contour, and the crop's hairline read as the rim of a cap. Each card
/// leaves at its own point across the mask's fade, which is the fade doing
/// for the geometry what it already did for the paint.
///
/// Provenance: **derived** from the fade: the middle half of it.
const EDGE_SPREAD: f32 = 0.5;

/// Where a parted style's front locks are combed to, in radians from dead
/// ahead.
///
/// Just in front of the ear: a lock from the forehead swept to the temple
/// hangs beside the face, and one swept further hangs behind the ear where
/// it no longer frames anything.
///
/// Provenance: **tuned by render** (#316).
const PART_TO: f32 = 1.25;

/// How far down the head a parted lock has finished turning, as a share of the
/// head's height below the crown.
///
/// The front hairline is about a third of the way down the head, and the lock
/// has to have reached the temple by the time it gets there or it hangs off
/// the forehead after all. Scheduled on the descent, as the comb is, for the
/// reason [`Sheet::combed`] gives.
///
/// Provenance: **derived** from where the hairline sits.
const PART_BY: f32 = 0.30;

/// How much of the way to the hairline a lock at the parting descends before
/// it starts to turn, as a share of [`PART_BY`].
///
/// Provenance: **tuned by render** (#316).
const PART_LATE: f32 = 0.6;

/// The cosine of the azimuth behind which a tied-back lock is drawn to the
/// knot at all.
///
/// About 70° off dead ahead — the temple. Forward of it the hair lies to the
/// hairline as a crop's does.
///
/// Provenance: **tuned by render** (#316).
const PULL_FROM: f32 = 0.35;

/// Over how much of that cosine the pull comes on, behind [`PULL_FROM`].
///
/// Provenance: **tuned by render** (#316).
const PULL_FADE: f32 = 0.5;

/// The azimuths the walk is measured at, in radians from dead ahead.
///
/// Sixteen, round the whole head and on both sides: a head is not symmetric
/// fore-and-aft and its profile tables are coarsest where it is narrowest, so a
/// handful of azimuths is a handful of chances to miss.
#[cfg(test)]
const TURNS: [f32; 16] = [
    0.0, 0.4, 0.8, 1.2, 1.6, 2.0, 2.4, 2.8, 3.1, -0.4, -0.8, -1.2, -1.6, -2.0, -2.4, -2.8,
];

/// How finely a lock's walk down the skull is integrated.
///
/// The walk has no closed form — it is the measured profile, sampled — so travel
/// along it is a sum, and the drawn clump can be no smoother than this polyline.
///
/// **Set by the COMB, not by the descent**. A lock going straight down
/// needs very few steps: at two dozen the chord between them sagged 0.2 mm on a
/// 90 mm skull. A lock combed to the nape turns 180° over about 60 mm of
/// descent, which at two dozen steps is 40° between two of them and a chord
/// cutting 4.4 mm inside the skull — a tied-back style with hair sunk into the
/// head behind the ear. At sixty-four the turn is 15° a step and the chord sags
/// 0.7 mm, which is inside the millimetre the loft's own sampler works to.
///
/// Provenance: **derived** from the chord a combed turn leaves, measured against
/// the walk (`a_lock_stays_on_the_head_while_the_head_is_holding_it_up`).
const STEPS: usize = 64;

/// How many of those steps are spent crossing the crown's own cap.
///
/// Twelve of the sixty-four, which puts about a millimetre of height between
/// them over a band the radius crosses its whole range in. Fewer and the chord
/// cuts into the scalp at the whorl; more and the descent below the cap goes
/// coarse for a part of the head that is barely turning.
///
/// Provenance: **derived** from the cap's own curvature, measured against
/// `a_lock_stays_on_the_head_while_the_head_is_holding_it_up`.
const CROWN_STEPS: usize = 12;

/// How many steps the walk spends below the head, hanging.
///
/// Four. Past the hairline the card hangs from where it left at the radius it
/// left with — a straight vertical line, whatever its length — so this is not a
/// resolution but a handful of points to draw one with.
///
/// Provenance: **derived** from what a straight line needs.
const HANG: usize = 4;

/// A lock the skull holds up, and which hangs once it does not.
///
/// The one shape all five styles compile to, differing in its numbers. See the
/// module header for why it walks a profile rather than a direction.
#[derive(Clone, Debug, PartialEq)]
struct Sheet {
    /// The measured head this lock is draped over, and where hair may grow on it.
    ///
    /// Cloned rather than borrowed because a [`Shape`] outlives the call that
    /// built it. The pipeline had already measured all of it.
    ///
    /// **The mask is in here because the card has to know where the hairline is**
    ///: the scalp is where a card LIES, and the hairline is where it stops
    /// lying and starts hanging. A style that guessed that from a height would
    /// hang a fringe off the middle of a forehead.
    regions: Follicles,
    /// How far the lock hangs PAST the head at the back, in metres.
    reach: f32,
    /// What share of that it travels at the front.
    fringe: f32,
    /// What multiple of it at the back.
    behind: f32,
    /// How wide the card is at its root, in metres.
    width: f32,
    /// What share of the width is left at the tip.
    taper: f32,
    /// How far the lock stands off the skull by the end of its travel, likewise.
    volume: f32,
    /// Where a tail gathers this lock, if the style has one, head-local.
    ///
    /// Computed once from the style's own axis rather than per call: it is also
    /// the schedule the comb runs on, and two places deriving the same point is
    /// how they come to disagree.
    knot: Option<Vec3>,
    /// How much of the way round to the back of the head the lock is combed by
    /// the time it has descended to the knot, `0` straight down its own azimuth.
    pull: f32,
    /// How tightly it coils.
    curl: f32,
    /// How far its front locks are parted to either side of the face, `0` a
    /// fringe and `1` swept clear to the temple.
    ///
    /// **Long hair does not hang over the face**: every card rooted in the
    /// front third of the head fell straight down its own meridian through
    /// the eyes and the mouth to the chest, which is a curtain and not a
    /// haircut. A parting is the same comb a tail is — the azimuth turns as
    /// the lock descends — aimed at the temple instead of the nape.
    part: f32,
}

impl Sheet {
    /// Which way round the head a root sits, from dead ahead.
    fn azimuth(root: &Root) -> f32 {
        root.at.x.atan2(root.at.z)
    }

    /// How far this lock hangs past the hairline, before the mask's own thinning.
    ///
    /// **The curtain lesson**: a share of the reach at the front and a multiple
    /// of it at the back, so a fringe stops above the eyes while the same style's
    /// sides reach the jaw. Uniform length is what makes hair read as a hood.
    fn fall(&self, root: &Root) -> f32 {
        // A tied lock either reaches the knot and hangs the tail's length
        // from it, or lies to the hairline and stops; see [`Self::pulled`].
        if self.knot.is_some() {
            let from = Self::azimuth(root);
            let spread = STAGGER_LEAST * (Self::salt(root, 0) - 0.5);
            return if Self::pulled(from) >= 0.5 {
                self.reach + spread
            } else {
                (STAGGER_LEAST * 0.5 + spread).max(0.0)
            };
        }
        let facing = Self::azimuth(root).cos();
        let front = crate::face::smooth((facing - FRONT) / (1.0 - FRONT));
        let back = crate::face::smooth((-facing - 0.1) / 0.9);
        let share = self.fringe + (1.0 - self.fringe) * (1.0 - front);
        let hang = self.reach * share * (1.0 + (self.behind - 1.0) * back);
        // Staggered card by card, so the tips do not draw a contour.
        let spread = (hang * STAGGER).max(STAGGER_LEAST);
        (hang + spread * (Self::salt(root, 0) - 0.5)).max(0.0)
    }

    /// How far it is from the crown to the hairline down this lock's meridian, in
    /// metres of travel over the surface.
    ///
    /// **The card covers the scalp, and the scalp is not a share of anything** —
    /// it is where the mask says hair grows, which on a face is 100 mm of forehead
    /// above the brow and on a nape is 20 mm below the occiput. So it is walked
    /// and measured rather than assumed.
    fn cap(&self, root: &Root) -> f32 {
        let walked = self.walked(root, f32::MAX);
        walked.cap.unwrap_or(walked.gone)
    }

    /// Whether hair grows anywhere down this lock's meridian.
    fn grows(&self, root: &Root) -> bool {
        self.walked(root, f32::MAX).grows
    }

    /// Which way round the head this lock is heading, having descended to
    /// `height` from `root`.
    ///
    /// **A comb turns the azimuth, it does not move the point**. The first
    /// cut of the tied-back style lerped a lock's POSITION toward the knot, which
    /// draws a chord straight through the skull — measured at 88.6 mm off the
    /// surface for a lock rooted over the brow. Hair combed back travels ROUND
    /// the head, so what a comb changes is which way the walk is going, and the
    /// walk stays on the surface for free.
    ///
    /// **Scheduled on the DESCENT, and it has to be**. Scheduled on the
    /// travel it fed back on itself: turning the azimuth adds azimuthal arc,
    /// which is travel, which turns it further — so the walk's steps grew from
    /// five millimetres to fifty, the polyline went coarse exactly where it was
    /// curving hardest, and the chord across one step cut 5 mm inside the skull.
    /// Height has no such loop, and combed hair does sweep down and round
    /// together: it reaches the nape when it reaches the nape's height.
    ///
    /// It turns toward the back on whichever side the lock started, so the two
    /// sides sweep round to the nape in opposite directions and part at the front
    /// rather than crossing over the face.
    fn combed(&self, from: f32, root: f32, height: f32) -> f32 {
        let side = if from < 0.0 { -1.0 } else { 1.0 };
        if let Some(knot) = self.knot.filter(|_| self.pull > 0.0) {
            let share = ((root - height) / (root - knot.y).max(f32::EPSILON)).clamp(0.0, 1.0);
            let back = std::f32::consts::PI * side;
            return from + (back - from) * (self.pull * Self::pulled(from) * share);
        }
        if self.part > 0.0 {
            // Only the locks over the face are parted, and the turn fades
            // in across the temple so the parted and the unparted meet.
            let facing = from.cos();
            let over = crate::face::smooth((facing - FRONT) / (1.0 - FRONT));
            if over <= 0.0 {
                return from;
            }
            let (throat, _) = self.regions.skull().throat_and_crown();
            let depth = (root - throat) * PART_BY;
            // **The midline turns LAST.** Turned together, every front lock
            // was half-way to the temple half-way down, and nothing was left
            // over the middle of the forehead: a bare V above the brow from
            // the parting to the hairline. A lock at the parting stays on its
            // own meridian until just above the hairline and then sweeps
            // along it, which is what combed-over hair does; a lock already
            // near the temple has little to turn and turns from the start.
            let start = depth * over * PART_LATE;
            let share =
                ((root - height - start) / (depth - start).max(f32::EPSILON)).clamp(0.0, 1.0);
            let to = PART_TO * side;
            return from + (to - from) * (self.part * over * crate::face::smooth(share));
        }
        from
    }

    /// How much of the way to the knot a lock rooted at `from` is drawn.
    ///
    /// **Only the back of the head feeds a tail** (#316). Combed from the
    /// crown toward the knot whatever its meridian, every front lock had
    /// turned away from the forehead before it got there, and the sheet
    /// showed a bare band of scalp above the brow on a head whose hair was
    /// supposedly drawn back over it. Hair drawn back lies flat to the front
    /// hairline; what is gathered is the hair behind the ears.
    fn pulled(from: f32) -> f32 {
        crate::face::smooth((PULL_FROM - from.cos()) / PULL_FADE)
    }

    /// A number in `0..1` that is this card's own, by lane.
    ///
    /// **Hashed from the root, not drawn from the stream**: a [`Shape`] is
    /// asked about a card many times over and has to answer the same each
    /// time, and the scatter's stream has already moved on. Two cards a
    /// millimetre apart get unrelated numbers, which is the point — stagger
    /// that correlated with position would be a wave, not a feathering.
    fn salt(root: &Root, lane: u32) -> f32 {
        let mut hash = lane.wrapping_mul(0x9E37_79B9);
        for part in [root.at.x, root.at.y, root.at.z] {
            hash ^= part.to_bits();
            hash = hash.wrapping_mul(0x85EB_CA6B);
            hash ^= hash >> 13;
        }
        hash = hash.wrapping_mul(0xC2B2_AE35);
        hash ^= hash >> 16;
        (hash >> 8) as f32 / (1u32 << 24) as f32
    }

    /// The mask weight below which THIS card has left the scalp.
    ///
    /// [`EDGE`] spread across the middle of the mask's fade, card by card.
    fn edge(root: &Root) -> f32 {
        EDGE + (Self::salt(root, 1) - 0.5) * EDGE_SPREAD
    }

    /// Which way the envelope faces at one height and azimuth, unit length.
    ///
    /// The surface is one of revolution locally — a radius that is a function
    /// of height — so its normal is the radial direction tilted by the
    /// radius's own slope: straight up at the pole, where the radius grows
    /// without bound per unit height, and radial at the head's widest.
    fn normal(&self, height: f32, azimuth: f32) -> Vec3 {
        let skull = self.regions.skull();
        let (throat, crown) = skull.throat_and_crown();
        let radius = |height: f32| {
            let at = skull.surface_at(height.clamp(throat, crown), azimuth);
            (at.x * at.x + at.z * at.z).sqrt()
        };
        let step = 0.001;
        let above = (height + step).min(crown);
        let below = (height - step).max(throat);
        let slope = (radius(above) - radius(below)) / (above - below).max(f32::EPSILON);
        let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
        (out - Vec3::Y * slope).normalize_or(Vec3::Y)
    }

    /// Where the lock is after walking `want` metres from its root.
    ///
    /// **Nothing here may depend on how far is being ASKED for**, only on the
    /// walk's own position — or `at` stops describing one curve. An earlier cut
    /// scheduled the comb on `want`, so `at(root, 0.25)` returned the quarter
    /// point of a lock that finished combing in a quarter of the length and
    /// `at(root, 1.0)` the tip of a different one: the sampler was drawing a
    /// polyline through points that were never on one curve, and it went 14 mm
    /// inside the skull.
    ///
    /// **The walk is the whole design, and where a lock leaves the head is
    /// computed rather than given.** An earlier cut let the cap have a fixed share
    /// of the length and then hang; that puts a crop's tips through the cheekbone
    /// on any head whose face is long for its vault, because a share of a length
    /// knows nothing about where the head ends.
    ///
    /// It descends the head at its own azimuth,
    /// staying on the measured surface while the surface is still holding the
    /// hair out, and keeping the widest radius it has passed once the head starts
    /// coming back in — which is hair draping, stated as an inequality rather
    /// than as a special case. Below the head entirely it simply hangs.
    fn walked(&self, root: &Root, want: f32) -> Walked {
        let from = Self::azimuth(root);
        let skull = self.regions.skull();
        let (throat, crown) = skull.throat_and_crown();
        let radius = |height: f32, azimuth: f32| {
            let at = skull.surface_at(height.clamp(throat, crown), azimuth);
            (at.x * at.x + at.z * at.z).sqrt()
        };
        // **Every card starts at the crown**, whatever its root, and that is the
        // fix the render argued for (#204). A card is a sheet of hair lying on the
        // scalp, not one hair: forty of them rooted at random and running only
        // from their own root left a bald disc at the crown and holes everywhere
        // else, because random radial strips do not tile a dome. Radiating from
        // the crown they overlap most where they converge, which is exactly where
        // hair does, and every one of them covers the crown.
        //
        // The root then says which MERIDIAN this card takes and how much of it
        // there is — its mask weight is the hairline's own thinning — which is
        // still the scatter following the mask, one dimension of it instead of two.
        // Every card starts at the crown, which is the top of the profile.
        let top = crown;
        // **The steps are not uniform in height, and the crown is why** (#210).
        // The profile's topmost band is closed with a quarter-circle to a point,
        // so the surface's radius goes from nothing to the head's full width
        // inside eleven millimetres there and then covers the other quarter of a
        // metre at a few millimetres a centimetre. Stepping uniformly spent two
        // and a half steps of sixty-four on the part that is actually turning,
        // and the chord across them cut 6.1 mm INSIDE the scalp — a card buried
        // in the head at the whorl, which is the one place every card converges.
        //
        // So the walk is in three parts, and each is sampled for what it is:
        // the cap, the descent down the head, and the free hang below it.
        //
        // **Splitting the hang off is what keeps the descent's pitch fixed.**
        // The walk runs a style's whole reach past the throat so that a fall has
        // somewhere to go, and dividing one count over that span made the step
        // depend on the haircut: a crop stepped 4.5 mm down the head and a bob
        // 7.3 over the same skull, so the same head strayed 5.4 mm under one
        // style and nothing under the other. Below the hairline the card hangs
        // at a frozen radius down a straight line, which needs no resolution at
        // all.
        let cap = skull.crown_band();
        let descent = (top - cap - throat) / (STEPS - CROWN_STEPS) as f32;
        let height_of = |index: usize| {
            if index < CROWN_STEPS {
                top - cap * (index + 1) as f32 / CROWN_STEPS as f32
            } else if index < STEPS {
                top - cap - descent * (index + 1 - CROWN_STEPS) as f32
            } else {
                throat - self.reach * (index + 1 - STEPS) as f32 / HANG as f32
            }
        };
        let mut at = skull.surface_at(top, from);
        let mut gone = 0.0;
        let mut free = 0.0;
        let mut hung: Option<f32> = None;
        let mut cap = None;
        // **The scalp mask fades out at the top of the head as well as at the
        // hairline, and only one of those is a hairline** (#204). Its own upper
        // bound is there so that a query from above the skull cannot answer `yes`;
        // the profile the walk rides is an outer envelope and sits a little above
        // the surface that bound was measured on, so at some azimuths a card's
        // first step is already outside it. Read as a hairline, that gave those
        // cards no cap at all — and a card with no cap is full width from its first
        // station, which the sheet showed as two blocky plates stuck to the top of
        // the head. Starting the cards lower instead left the whole crown bare,
        // which is worse and is what this flag is instead of.
        let mut begun = false;
        // The height whose radius has been the widest so far, which is what the
        // hair is hanging off.
        //
        // **A height rather than a radius, and that is the whole of it.** Holding
        // the widest RADIUS seen is right for a lock going straight down and wrong
        // for one combed round the head: the brow's forward reach is 15 mm more
        // than the side's half-width, so a lock combed from the forehead to the
        // ear kept the forehead's radius and stood 12 mm off the side of the head.
        // Holding the height instead means the support is re-read at whatever
        // azimuth the lock has reached, which is the same claim measured in the
        // right place.
        let mut crest = top;
        let edge = Self::edge(root);
        // How many stations of tail have hung from the knot, once the lock
        // has reached it.
        let mut tail: Option<usize> = None;
        for index in 0..STEPS + HANG {
            let height = height_of(index);
            // **A tied lock hangs from the KNOT, not from the nape** (#316).
            // The comb brings it round to the back at the knot's height, and
            // from there it is gathered: it leaves the surface for the knot
            // and hangs below it. It used to walk on down the back of the head
            // to the nape hairline and be lerped toward the knot over its last
            // half — a chord up through the occiput for a high tail, which on
            // the sheet was a clump floating off the back of the head.
            if let Some(knot) = self.knot
                && tail.is_none()
                && hung.is_none()
                && begun
                && height <= knot.y
                && Self::pulled(from) >= 0.5
            {
                tail = Some(0);
                hung = Some(0.0);
                cap = Some(gone);
            }
            if let (Some(knot), Some(hung_at)) = (self.knot, tail) {
                if hung_at >= HANG {
                    break;
                }
                let next = if hung_at == 0 {
                    knot
                } else {
                    knot + Vec3::NEG_Y * (self.reach * hung_at as f32 / HANG as f32)
                };
                tail = Some(hung_at + 1);
                let leg = (next - at).length();
                free += leg;
                if gone + leg >= want {
                    let left = if leg > f32::EPSILON {
                        (want - gone) / leg
                    } else {
                        0.0
                    };
                    return Walked {
                        at: at.lerp(next, left.clamp(0.0, 1.0)),
                        gone: want,
                        free: (free - leg * (1.0 - left)).max(0.0),
                        grows: begun,
                        cap,
                    };
                }
                gone += leg;
                at = next;
                continue;
            }
            // **Turned by how far the lock has TRAVELLED, not by which step this
            // is.** A step is a slice of the descent, and a lock that is combed
            // round the head spends most of its length going sideways — so a comb
            // measured in steps finishes turning long after the lock has ended, or
            // never starts. Measured in travel, the turn completes exactly when
            // the lock does.
            let azimuth = self.combed(from, top, height);
            // **Held out by whatever it has draped over.** Above the widest part
            // of the head the surface pushes the hair out; below it, the head
            // falls away and the hair does not follow it in.
            //
            // And once past the hairline the hair is not on the head at all: it
            // hangs from wherever it left, at the radius it left with. Frozen
            // rather than faded, because a hairline is a line — the fade belongs
            // to the mask, which has already thinned the locks that cross it.
            let held = match hung {
                Some(held) => held,
                None => {
                    let here = radius(height, azimuth);
                    let over = radius(crest, azimuth);
                    if here >= over {
                        crest = height;
                    }
                    here.max(over)
                }
            };
            let next = Vec3::new(held * azimuth.sin(), height, held * azimuth.cos());
            let leg = (next - at).length();
            if hung.is_some() {
                free += leg;
            }
            if gone + leg >= want {
                let left = if leg > f32::EPSILON {
                    (want - gone) / leg
                } else {
                    0.0
                };
                return Walked {
                    at: at.lerp(next, left.clamp(0.0, 1.0)),
                    gone: want,
                    free: (free - leg * (1.0 - left)).max(0.0),
                    // Whatever the walk has seen SO FAR, which is all a caller
                    // asking about a point can be told; `grows` and `cap` are asked
                    // of a walk to the end.
                    grows: begun,
                    cap,
                };
            }
            gone += leg;
            at = next;
            let grows = self.regions.weight(Follicle::Scalp, next) >= edge;
            begun |= grows;
            if begun && hung.is_none() && !grows {
                hung = Some(held);
                cap = Some(gone);
            }
        }
        Walked {
            at,
            gone,
            free,
            grows: begun,
            cap,
        }
    }
}

/// Where a walk got to, and how much of it was off the head.
///
/// The free share is what the coil and the volume are scaled by: hair coils and
/// stands off once it is hanging, and a card lying on a scalp does neither.
struct Walked {
    /// The point reached, head-local.
    at: Vec3,
    /// How far the walk travelled to get there, in metres.
    gone: f32,
    /// How much of that was past the hairline.
    free: f32,
    /// Whether the mask claimed this meridian anywhere along the walk.
    ///
    /// **A card whose meridian grows no hair grows nothing at all**. It
    /// used to grow a card with no cap, and a card with no cap is full width from
    /// its first station and only as long as its own hang — a short wide slab. The
    /// contact sheet showed two of them floating off the back of the head, one
    /// either side, and they had survived three other repairs because they are not
    /// a walk that strays: they are a walk that should never have been drawn.
    grows: bool,
    /// How far it had travelled when it crossed the hairline, if it did.
    ///
    /// **What says how long the cap is, and the first cut of this had no such
    /// field**. Without it the cap was the whole polyline — the walk runs
    /// on past the head so that a fall has somewhere to go — so every lock became
    /// a 300 mm dreadlock hanging past the chin and cost 84 triangles.
    cap: Option<f32>,
}

impl Shape for Sheet {
    fn length(&self, root: &Root) -> f32 {
        // Nothing at all where no hair grows down this meridian: see
        // [`Walked::grows`]. This is what [`Shape::length`]'s zero is for, and it
        // hands the triangles back rather than spending them on a slab.
        if !self.grows(root) {
            return 0.0;
        }
        // The scalp it covers plus what hangs past the hairline. Only the fall is
        // thinned by the mask — a card lies on the scalp whether or not the
        // hairline there is receding, and it is what HANGS that thins out. Square-
        // rooted so a thinning edge keeps some length rather than collapsing over
        // the last tenth.
        //
        // Except into a knot: a gathered lock hangs the tail's length from
        // the knot whatever the mask said where its root happened to sit,
        // or the tail's tips spread over the thinning of a hairline they
        // never crossed (#316).
        let thinned = if self.knot.is_some() && Self::pulled(Self::azimuth(root)) >= 0.5 {
            1.0
        } else {
            root.weight.clamp(0.0, 1.0).sqrt()
        };
        self.cap(root) + self.fall(root) * thinned
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let length = self.length(root);
        let walked = self.walked(root, length * along);
        let mut at = walked.at;
        // Standing off the skull, but only once it is off the head: this is what
        // `droop` gives up. Radially, because that is the direction a head pushes
        // hair. A card lying on a scalp has no volume to give.
        let azimuth = Self::azimuth(root);
        let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
        let loose = crate::face::smooth(walked.free / LOOSE);
        // **Lifted along the surface the card is LYING ON, not along its
        // root's normal** (#316). Every card starts at the crown, and a root
        // at the nape has a normal pointing back and down — so its card
        // crossed the whorl with its clearance pointing sideways, and the
        // chords between its stations cut a millimetre into the back of the
        // dome. Measured on the default crop: 15% of the stations in the
        // first third of every card under the mesh, worst 1.1 mm, all at the
        // back of the crown; on the sheet, slivers of scalp showing through
        // the cards behind the whorl. The normal of the envelope at the point
        // reached is what a card needs to clear, and it is the crown's up
        // at the crown.
        let lift = if walked.free > 0.0 {
            out
        } else {
            self.normal(walked.at.y, azimuth)
        };
        at += out * (self.volume * loose) + lift * LIFT;
        // And coiled, if it is a curl: a wave across its own fall rather than a
        // helix round it. See [`COIL`] for what the helix cost.
        if self.curl > 0.0 {
            let swing = (SWING[0] + (SWING[1] - SWING[0]) * self.curl) * loose;
            let wave = WAVE[0] + (WAVE[1] - WAVE[0]) * self.curl;
            // Around the fall rather than across it, so a ringlet reads as one
            // from any angle: a wave in one plane is a kink from the side and
            // nothing at all from the front.
            let phase = length * along * std::f32::consts::TAU / wave;
            let sideways = out.cross(Vec3::Y).normalize_or(Vec3::X);
            at += sideways * (swing * phase.sin()) + out * (swing * phase.cos());
        }
        at
    }

    fn seating(&self) -> Seating {
        // A root is a meridian here: see [`Seating`], and the bald side of a
        // tied-back head that found it.
        Seating::Meridians
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        // **A card, and coverage comes from how wide it is.** Count is triangles
        // and width is free, which is the whole argument of this file: forty wide
        // cards cover a head that a hundred and fifty strings left bare.
        let thin = root.weight.clamp(0.0, 1.0).sqrt();
        let base = self.width * 0.5 * thin;
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // **Round the head, not level with the ground** (#204). A lock walks DOWN
        // a meridian, so its card lies along the parallel — and at the crown that
        // is the only one of the two that exists at all: the engine's default is
        // the level tangent, which at the top of a head is the normal's own cross
        // product with itself and collapses. The sweep then substitutes an
        // arbitrary frame, the card turns edge-on, and the crown renders as a bald
        // disc with hair radiating from its rim. It is #205's lesson again, in the
        // one place on a body where a level tangent has no direction.
        let azimuth = Self::azimuth(root);
        Vec3::new(azimuth.cos(), 0.0, -azimuth.sin())
    }

    fn across_at(&self, root: &Root, along: f32) -> Vec3 {
        // **Across the lock where the lock IS, lying in the surface it is
        // lying on** (#316). A combed lock is not where it started, and the
        // one axis that keeps a card flat on a skull wherever its spine is
        // heading is the surface's normal crossed with that heading. The
        // heading is read off the curve itself, either side of the station;
        // the normal is the envelope's where the lock is still on the head
        // and radial once it hangs. Anything degenerate falls back to the
        // root's own parallel, which is what every other station has.
        let along = along.clamp(0.0, 1.0);
        let step = 0.01;
        let ahead = self.at(root, (along + step).min(1.0));
        let behind = self.at(root, (along - step).max(0.0));
        let heading = (ahead - behind).normalize_or(Vec3::ZERO);
        let here = self.at(root, along);
        let azimuth = here.x.atan2(here.z);
        let walked = self.walked(root, self.length(root) * along);
        let normal = if walked.free > 0.0 {
            Vec3::new(azimuth.sin(), 0.0, azimuth.cos())
        } else {
            self.normal(here.y, azimuth)
        };
        normal.cross(heading).normalize_or(self.across(root))
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // **A fan on the scalp, a taper off it, and the two are different jobs.**
        //
        // The fan is because the cards all start at one point: the circumference a
        // card has to cover grows with its distance from the crown — at 17 mm out
        // forty cards need two millimetres each and at the rim they need thirteen —
        // so a card of one width either piles into a solid mass at the crown or
        // leaves gaps at the hairline. Widening with the travel is what a whorl
        // does and what tiles a dome.
        //
        // The taper is what a lock of hair does as it hangs, and it belongs to the
        // hanging part ONLY. Tapering across the whole card narrowed it to half its
        // width by the time it reached the hairline — which is exactly where a card
        // has the most scalp to cover — and the sheet read as a rosette of tubes
        // with bare scalp between them.
        let (base, tip) = self.width(root);
        let along = along.clamp(0.0, 1.0);
        let cap = (self.cap(root) / self.length(root).max(f32::EPSILON)).clamp(0.0, 1.0);
        if along <= cap {
            // **Linear in the travel, not eased** (#316). The circumference a
            // card has to share grows linearly with its distance from the
            // pole, and an eased ramp is quadratic at its start: at fifteen
            // millimetres out it had reached six per cent of the way to full
            // width, which on a tied-back head of twenty-eight cards is a
            // five-millimetre card covering a thirteen-degree sector, and the
            // star of bare wedges between them at the whorl.
            //
            // **And in metres from the pole, not in a share of the cap** — a
            // tied-back card's cap walks round the head and is three times a
            // crop's, so a share of it reached full width three times further
            // out, and the tied-back crown kept its wedges after the crop had
            // lost them. The whorl is the same size whatever the style.
            let travel = along * self.length(root);
            let fan = FAN + (1.0 - FAN) * (travel / FAN_OVER).clamp(0.0, 1.0);
            return base * fan;
        }
        // Full width down the hang, then to the tip over the last [`TIP`]
        // of it: a lock's point is at its end, not spread over its length.
        let length = self.length(root);
        let hang = length * (1.0 - cap);
        let left = (1.0 - along) * length;
        let past = 1.0 - (left / TIP.min(hang).max(f32::EPSILON)).clamp(0.0, 1.0);
        base + (tip - base) * past
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull as MeasuredSkull};
    use crate::hair::follicle::FollicleParams;
    use crate::{Archetype, Avatar, AvatarRecord};

    /// The regions of one built head, which a style is fitted against.
    fn head() -> Follicles {
        let record = AvatarRecord::new("Scalp", Archetype::default());
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull =
            MeasuredSkull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default())
    }

    /// A root on the crown of that head, turned to one azimuth.
    fn root(head: &Follicles, azimuth: f32, height: f32) -> Root {
        let at = head.skull().surface_at(height, azimuth);
        Root {
            at,
            out: Vec3::new(at.x, 0.0, at.z).normalize_or(Vec3::Y),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    /// Every style, at a middling cut.
    fn every_style() -> [ScalpStyle; 5] {
        [
            ScalpStyle::Crop,
            ScalpStyle::Bob { fringe: 0.6 },
            ScalpStyle::Long { weight: 0.6 },
            ScalpStyle::TiedBack { tail: 0.6 },
            ScalpStyle::Curly { curl: 0.6 },
        ]
    }

    #[test]
    fn a_lock_stays_on_the_head_while_the_head_is_holding_it_up() {
        // **The defect this file exists for** (#204). A lock that leaves along
        // its tangent plane is 12 mm off a 90 mm skull after 46 mm of travel, so
        // it covers scalp for a centimetre and then hangs in the air — which is
        // why a hundred and fifty of them read as strings over a bare scalp.
        //
        // Asserted as the distance from the profile over the part of the walk
        // that is still on the head: a lock following the surface stays within
        // its own thickness of it.
        let head = head();
        let (throat, crown) = head.skull().throat_and_crown();
        // **At full droop, which is hair hugging the head with no volume asked
        // for.** `Cut::droop` buys a deliberate standoff of up to eighteen
        // millimetres, and a bound that allowed for it would be measuring the
        // volume rather than the walk — the first cut of this failed at 12.3 mm
        // against a 12 mm bound, which was almost exactly the 9 mm of volume the
        // default cut asks for plus the drape.
        let flat = Cut {
            droop: 1.0,
            ..Cut::default()
        };
        for style in every_style() {
            // A tail is hair deliberately drawn off the surface into a knot, so
            // the claim does not apply to it past the gather; `a_tail_gathers_\
            // every_lock_to_one_knot` is what holds that style. Up to the gather
            // it walks the skull like everything else, and that is what is
            // measured here.
            let knot = style.knot(&head);
            // And a coil swings off the surface on purpose, so its own swing is
            // added to what it is allowed. Named from the style's own constants
            // rather than picked, so a wider coil cannot quietly widen the bound.
            let swung = match style {
                ScalpStyle::Curly { curl } => SWING[0] + (SWING[1] - SWING[0]) * curl,
                _ => 0.0,
            };
            let shape = style
                .shape(&flat, Follicle::Scalp, &head)
                .expect("a grown style has a shape");
            // **Swept round the head and up to the crown itself** (#204). Four
            // azimuths starting an eighth of the way down missed the two cards the
            // contact sheet showed sticking off the top of the head as blocky
            // plates: whatever a walk does at the crown, where the head's own
            // radius is smallest and its profile tables are coarsest, nothing was
            // measuring it.
            let mut probed = 0usize;
            for turn in TURNS {
                for high in [0.02f32, 0.12, 0.30] {
                    let root = root(&head, turn, crown - (crown - throat) * high);
                    let mut worst = 0.0f32;
                    let radius = |point: Vec3| (point.x * point.x + point.z * point.z).sqrt();
                    for step in 0..=20 {
                        let along = step as f32 / 20.0;
                        let at = shape.at(&root, along);
                        // A tail has left the skull once it is at the knot's
                        // height: from there it hangs from the knot.
                        if knot.is_some_and(|knot| at.y <= knot.y + 0.002) {
                            break;
                        }
                        // **Only while the head is still holding this lock UP**, which
                        // is what the claim says and is not a fixed share of the head's
                        // height: below the widest band it passes, hair drapes and is
                        // supposed to stand off a narrowing skull —
                        // `hair_does_not_follow_the_head_back_in_under_the_widest_part`
                        // is the test for that half. The first cut of this drew the
                        // line at 45% of the head and then read the occiput's own
                        // drape as 5.4 mm of stray.
                        //
                        // Measured at the lock's own azimuth, since a combed lock is
                        // not where it started: still supported means the profile is
                        // no narrower here than anywhere it has come from.
                        // **And only while it is still ON the scalp**, which is the
                        // other half of what holds a card out and is not the same
                        // claim (#210). A fringe leaves the mask at the hairline and
                        // hangs from the radius it left with, while the head under
                        // it goes on WIDENING all the way down the brow — so a bob
                        // read as 5.2 mm of stray for doing exactly what a fringe
                        // does. The head being wider above is drape; the mask ending
                        // is a hairline.
                        if head.weight(Follicle::Scalp, at) < EDGE {
                            break;
                        }
                        let azimuth = at.x.atan2(at.z);
                        let here = radius(head.skull().surface_at(at.y, azimuth));
                        // **From the CROWN, which is where the card started, and not
                        // from its root** (#210). Every card starts at the crown
                        // whatever its root, and the crown is a point: the cap over
                        // the topmost band closes the radius to nothing there. So a
                        // scan from the root read the head as having been wider
                        // above the very first station of every card, broke out of
                        // the loop before measuring anything, and this test asserted
                        // NOTHING AT ALL for five styles across sixteen azimuths —
                        // measured, zero stations of 1,008. It is the seventh
                        // instrument this milestone to have been measuring its own
                        // parameterisation rather than the hair, and the only one so
                        // far to have been measuring none of it.
                        let widest = (0..40)
                            .map(|band| {
                                let height = at.y + (crown - at.y) * band as f32 / 39.0;
                                radius(head.skull().surface_at(height, azimuth))
                            })
                            .fold(0.0f32, f32::max);
                        if widest > here + 0.0005 {
                            break;
                        }
                        // **Against the surface where the lock IS, not where it
                        // started.** A combed lock travels round the head, and the
                        // head is a different width behind the ear than over the
                        // brow — so measuring it against its root's azimuth reads a
                        // correct walk as 12.8 mm of stray. That is the third time in
                        // this milestone that an instrument has measured its own
                        // parameterisation rather than the hair.
                        // **As the distance to the meridian, not the radius at
                        // the height** (#316). Over the crown the dome is flat,
                        // so a card lifted its clearance off the surface there
                        // is a fifth of a millimetre higher and eleven
                        // millimetres further out at the same height — and a
                        // station lifted ABOVE the crown has no surface at its
                        // height at all and was measured against the pole:
                        // 17.7 mm of "stray" on a crop doing nothing wrong.
                        // The nearest point of the meridian within a few
                        // millimetres of height is the claim the test makes.
                        let nearest = (0..=20)
                            .map(|step| {
                                let height = (at.y - 0.006 + 0.012 * step as f32 / 20.0)
                                    .clamp(throat, crown);
                                at.distance(head.skull().surface_at(height, azimuth))
                            })
                            .fold(f32::MAX, f32::min);
                        worst = worst.max(nearest);
                        probed += 1;
                    }
                    assert!(
                        worst < 0.004 + swung,
                        "a {style:?} lock at azimuth {turn} rooted {high} of the way down strays \
                     {:.1} mm from the skull while the skull is still under it, with no volume \
                     asked for and {:.1} mm of coil allowed",
                        worst * 1000.0,
                        swung * 1000.0
                    );
                }
            }
            // **And that it measured anything at all** (#210). Every card starts
            // at the crown and the crown is a point, so the supported test above
            // read the head as having been wider above the very first station of
            // every card and broke out before measuring one of them: this
            // assertion held for five styles over sixteen azimuths and three root
            // heights by measuring NOTHING, for as long as #204 was open. An
            // instrument that has stopped looking passes every bound there is.
            //
            // Measured now: 420 stations of 1,008 on the style that leaves the
            // mask soonest, and all 1,008 on the tied-back, which never leaves it
            // before its gather.
            assert!(
                probed > 250,
                "a {style:?} was checked against the skull at only {probed} stations of {} — the \
                 claim above is not being made about anything",
                TURNS.len() * 3 * 21
            );
        }
    }

    #[test]
    fn hair_does_not_follow_the_head_back_in_under_the_widest_part() {
        // The other half of the walk, and the one that is physics rather than
        // anatomy: hair is held out by whatever it has draped over. A lock that
        // tracked the profile all the way down would go INTO the neck, which is
        // what a surface-following drape does if nobody states the inequality.
        let head = head();
        let (throat, crown) = head.skull().throat_and_crown();
        let shape = ScalpStyle::Long { weight: 0.5 }
            .shape(&Cut::default(), Follicle::Scalp, &head)
            .expect("a long style has a shape");
        let root = root(&head, 3.1, crown - (crown - throat) * 0.1);
        let radius = |point: Vec3| (point.x * point.x + point.z * point.z).sqrt();
        let widest = (0..40)
            .map(|step| {
                let height = throat + (crown - throat) * step as f32 / 39.0;
                radius(head.skull().surface_at(height, 3.1))
            })
            .fold(0.0f32, f32::max);
        let mut narrowest = f32::MAX;
        let mut below = false;
        for step in 0..=30 {
            let at = shape.at(&root, step as f32 / 30.0);
            if at.y < throat {
                below = true;
                narrowest = narrowest.min(radius(at));
            }
        }
        assert!(below, "a long lock never reached past the head at all");
        assert!(
            narrowest > widest * 0.9,
            "hair came back in to {:.1} mm under a head {:.1} mm wide, so it is following the \
             neck rather than hanging off the skull",
            narrowest * 1000.0,
            widest * 1000.0
        );
    }

    #[test]
    fn the_fringe_is_shorter_than_the_back_on_every_style_that_says_so() {
        // The curtain lesson, as an assertion. Uniform length falls off the brow
        // straight down the face; what makes a haircut is that the front and the
        // back are different, and each style says how different in its own way.
        let head = head();
        let (throat, crown) = head.skull().throat_and_crown();
        let high = crown - (crown - throat) * 0.1;
        for style in every_style() {
            // A tail is combed round to one knot, so its front and its back END IN
            // THE SAME PLACE by construction — the curtain claim is not a claim
            // about it. `a_tail_gathers_every_lock_to_one_knot` is what holds that
            // style.
            if matches!(style, ScalpStyle::TiedBack { .. }) {
                continue;
            }
            let shape = style
                .shape(&Cut::default(), Follicle::Scalp, &head)
                .expect("a grown style has a shape");
            // **Where the tip LANDS, not how far the lock travelled** (#204). A
            // card's travel is the scalp it covers plus its own hang, and the
            // scalp at the front of a head is a forehead — so a front lock travels
            // further than a back one while hanging much less. Measured as total
            // travel, the tied-back style read as a 392 mm fringe. The claim was
            // always about the fringe stopping above the eyes, which is a height.
            let tip = |turn: f32| shape.at(&root(&head, turn, high), 1.0).y;
            let (front, back) = (tip(0.0), tip(std::f32::consts::PI));
            assert!(
                front > back,
                "{style:?} hangs its fringe to {:+.0} mm and its back to {:+.0}, which is a hood",
                front * 1000.0,
                back * 1000.0
            );
        }
        // And a bob's own axis is what does it, over its whole range.
        let fringe_of = |fringe: f32| {
            let shape = ScalpStyle::Bob { fringe }
                .shape(&Cut::default(), Follicle::Scalp, &head)
                .expect("a bob has a shape");
            shape.at(&root(&head, 0.0, high), 1.0).y
        };
        assert!(
            fringe_of(1.0) > fringe_of(0.5) && fringe_of(0.5) > fringe_of(0.0),
            "the fringe axis does not order a bob: {:+.0}, {:+.0}, {:+.0} mm",
            fringe_of(1.0) * 1000.0,
            fringe_of(0.5) * 1000.0,
            fringe_of(0.0) * 1000.0
        );
    }

    #[test]
    fn a_tail_gathers_every_lock_to_one_knot() {
        // What makes a tied-back style that rather than a short crop: the locks
        // lie on the skull and their TIPS meet. Asserted as the spread of the
        // tips against the spread of the roots, which is the only way to say
        // "gathered" that does not depend on where the knot happens to be.
        let head = head();
        let (throat, crown) = head.skull().throat_and_crown();
        let high = crown - (crown - throat) * 0.15;
        // **Behind the temple**, which is the hair a tail gathers (#316): a
        // lock over the brow lies to the hairline and stops, and asking it
        // to meet the knot is asking a tied-back head to have no front.
        let turns = [1.6f32, 2.2, 3.0, -1.6, -2.2, -2.8];
        let spread = |style: ScalpStyle| {
            let shape = style
                .shape(&Cut::default(), Follicle::Scalp, &head)
                .expect("a grown style has a shape");
            let tips: Vec<Vec3> = turns
                .iter()
                .map(|turn| {
                    let root = root(&head, *turn, high);
                    shape.at(&root, 1.0)
                })
                .collect();
            let middle = tips.iter().fold(Vec3::ZERO, |sum, at| sum + *at) / tips.len() as f32;
            tips.iter()
                .map(|at| at.distance(middle))
                .fold(0.0f32, f32::max)
        };
        let tied = spread(ScalpStyle::TiedBack { tail: 0.6 });
        let loose = spread(ScalpStyle::Crop);
        assert!(
            tied < loose * 0.6,
            "a tail's tips spread {:.0} mm against a crop's {:.0}, so nothing is being gathered",
            tied * 1000.0,
            loose * 1000.0
        );
    }

    #[test]
    fn a_curl_coils_and_a_crop_does_not() {
        // Measured as path length against straight-line distance: a coil travels
        // further than it reaches, and that ratio is what a curl IS. Asserted
        // rather than eyeballed because the wave is drawn across the fall, and a
        // wave of the wrong amplitude reads as a kink from one angle and as
        // nothing at all from another.
        let head = head();
        let (throat, crown) = head.skull().throat_and_crown();
        let wander = |style: ScalpStyle| {
            let shape = style
                .shape(&Cut::default(), Follicle::Scalp, &head)
                .expect("a grown style has a shape");
            let root = root(&head, 2.0, crown - (crown - throat) * 0.12);
            let path: Vec<Vec3> = (0..=60)
                .map(|step| shape.at(&root, step as f32 / 60.0))
                .collect();
            let walked: f32 = path.windows(2).map(|step| step[0].distance(step[1])).sum();
            walked
                / path[0]
                    .distance(*path.last().expect("a path"))
                    .max(f32::EPSILON)
        };
        // **Against its own loose setting, not against a crop** (#204). Every
        // scalp card now walks the skull, and a curved path wanders further than
        // its own chord whether or not it coils — measured, a crop wanders 1.19
        // times its reach on its own. Comparing the two styles was measuring the
        // skull. The axis against itself is the claim: turning curl up has to add
        // path.
        let tight = wander(ScalpStyle::Curly { curl: 1.0 });
        let loose = wander(ScalpStyle::Curly { curl: 0.0 });
        // A ringlet rather than a corkscrew: see [`SWING`] for why a tight coil
        // is not something this triangle count can draw.
        assert!(
            tight > loose * 1.15,
            "a tight curl wanders {tight:.2} times its own reach against a loose one's \
             {loose:.2}, so the axis is not coiling"
        );
    }

    #[test]
    fn every_variants_own_axis_is_clamped_and_quantised() {
        // A variant carries its own axis, so `sanitize` has to reach inside it —
        // and a record off the network is the reason. Nothing else in the record
        // has this shape, so the wiring is worth a test of its own.
        let mut styles = [
            ScalpStyle::Bob { fringe: 9.0 },
            ScalpStyle::Long { weight: -3.0 },
            ScalpStyle::TiedBack { tail: 0.123_456_7 },
            ScalpStyle::Curly { curl: 0.987_654_3 },
        ];
        for style in &mut styles {
            style.sanitize();
            let once = *style;
            style.sanitize();
            assert_eq!(
                once, *style,
                "sanitize moved a style it had already cleaned"
            );
        }
        assert_eq!(styles[0], ScalpStyle::Bob { fringe: 1.0 });
        assert_eq!(styles[1], ScalpStyle::Long { weight: 0.0 });
        assert_eq!(
            styles[2],
            ScalpStyle::TiedBack {
                tail: scaled::quantize(0.123_456_7)
            }
        );
        // And the quantisation is the wire's own, so a record round-trips.
        let ScalpStyle::Curly { curl } = styles[3] else {
            panic!("a curl stopped being a curl");
        };
        assert_eq!(curl, scaled::quantize(curl), "a curl does not round-trip");
    }
}
