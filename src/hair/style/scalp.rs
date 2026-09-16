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

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Lump, Root, Seating, Shape};
use super::super::follicle::{Follicle, Follicles};
use super::super::shell::Cap;
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
    /// A curtain to about the jaw, with a fringe over the brow or the front
    /// swept aside.
    Bob {
        /// What the front does: `0` sweeps it to the temples, parted as a long
        /// head is, so it hangs beside the face at the sides' length; a little
        /// more is a fringe ending just above the brow, and `1` a fringe well
        /// above it. Never a curtain: a fringe's length is a share of the
        /// forehead it hangs over, not of the cut (#341).
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
    /// A sculpted low-poly crop: a faceted bowl to the hairline, with a notch
    /// over the brow and wisps at the fringe and the nape.
    ///
    /// **The first of the helmet family** (#338, #346): the mass is a closed
    /// solid rather than a layer of cards, because flat cards cannot lie under
    /// flat cards - see [`hair::shell`](crate::hair::shell). A shell covers the
    /// whole scalp mask, so the painted layer under one is invisible.
    Cap {
        /// How deep the fringe notch is cut back over the brow, `0` at the
        /// hairline and `1` a notch 22 mm behind it.
        #[serde(with = "crate::plan::scaled")]
        fringe: f32,
    },
    /// Swept back off the brow as one smooth shell, with a pompadour at the
    /// front and no cards anywhere: the one style whose edge is the solid's own.
    SlickBack {
        /// How much the front rises over the forehead, `0` combed flat and `1` a
        /// full pompadour.
        #[serde(with = "crate::plan::scaled")]
        volume: f32,
    },
    /// A bob as one smooth bell, with a notch over the face and wisps
    /// everywhere the notch is not.
    Bell {
        /// How far the hem falls, `0` just past the hairline and `1` to the
        /// jawline - in the head's own radii, since three measured heads
        /// disagree by half about how many millimetres that is.
        #[serde(with = "crate::plan::scaled")]
        length: f32,
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
const WIDTH: [f32; 5] = [0.034, 0.046, 0.056, 0.050, 0.070];

/// What share of its width a lock keeps at its tip.
///
/// Real hair gathers: locks that fall exactly as they were rooted stay parallel
/// all the way down and read as a comb.
///
/// **Half, on every style, because the point is the strand mask's now**
/// (#340). The geometry used to make it: a card's END is a flat cap, and at a
/// third of the width the fringe was a row of square teeth over the forehead -
/// the caps themselves, 14 mm across and facing the camera - so every lock was
/// tapered nearly to a point, and every style's ends went thin getting there.
/// The mask cuts each card's end into strands that come to points of their
/// own, so there is no cap left to hide and the width goes back into the lock.
/// Rendered with the mask at the old taper, a third and a half, each step wider
/// read fuller with the ends still feathered, and a half hid the most of the
/// painted layer where the cut uncovers it. Only the hanging part tapers (see
/// `Sheet::width_at`).
///
/// Provenance: **carried** in spirit from the shell's gather, **tuned by render**
/// (#340: 0.10 to 0.30 by style before the mask, then a third and a half).
const TAPER: [f32; 5] = [0.5, 0.5, 0.5, 0.5, 0.5];

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

/// How far a ringlet's width is turned from lying across the head to lying
/// along its coil's binormal, as a share (#343).
///
/// **A flat card following a coil folds itself, twice a turn.** Across the
/// head's own normal, a ringlet's width lay in the coil's plane for part of
/// every turn, and at the tightest coil the curve's radius there is 23 mm
/// against a 35 mm half-width: the inner edge runs backwards, and the card
/// flips over between two stations. Turned with the coil, the width stays
/// along the coil's binormal, the edges are coils of their own, and the
/// ringlet is a spiral band with a body from any side.
///
/// Provenance: **derived** from the coil: one turn a turn.
const TWIST: f32 = 1.0;

/// Whether a ringlet is seamed where its face turns over; see
/// [`Shape::seamed`] (#343).
///
/// Provenance: **derived**: a card that turns with its coil turns over twice
/// a turn.
const SEAMED: bool = true;

/// What multiple of its reach a curl hangs at the back of the head (#343).
///
/// **The nape a curl used to cover with ringlet ends.** The strand mask cuts
/// a card's end into points (#340) and a ringlet held clear of its own bend
/// is narrower on its turns, so at the reach every side hangs, the neck under
/// the nape's hairline showed from behind. Longer ringlets at the back hang
/// over it, as a bob's and a long head's back do, and no card lies under them.
///
/// Provenance: **tuned by render**.
const CURL_BEHIND: f32 = 1.5;

/// What share of the radius a ringlet bends through, in the plane its width
/// lies in, its half-width may be (#343).
///
/// **An edge of a card runs backwards where the card is wider than the bend
/// it is going round**: at a half-width `h` over a bend of curvature `k` in
/// the width's own plane, the inner edge moves `1 - h k` as far as the spine.
/// On its binormal a ringlet's width is clear of the bend where the coil is a
/// helix, but the swing grows over [`LOOSE`] and there it is not one.
///
/// Provenance: **derived** from the edge's own speed, with a fifth kept.
const FOLD_CAP: f32 = 0.8;

/// How far either side of a station the bend is read, in metres.
///
/// Provenance: **derived** from the sampler: a few millimetres is under a
/// tenth of the tightest wave and over the loft's own tolerance.
const FOLD_STEP: f32 = 0.003;

impl Style for ScalpStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        // **A helmet style is a solid, so it never reaches the tables below**
        // (#346): its geometry is a `Shell` description and its cards are its
        // rim's, both sized in `hair::shell` against the measured head. One
        // place, because `tests/budget.rs` costs a style through the same call.
        if let Some(cap) = self.helmet(head) {
            return Some(cap.shape(cut, head));
        }
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
            Self::Curly { .. } => (0.5, CURL_BEHIND),
            // Unreachable: a helmet style returned its own shape above, and the
            // match is written out rather than caught by a wildcard so the next
            // style added has to answer here too.
            Self::Cap { .. } | Self::SlickBack { .. } | Self::Bell { .. } => return None,
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
            whorl: if knot.is_some() { WHORL } else { 0.0 },
            part: match self {
                Self::Long { .. } => 1.0,
                // A bob with no fringe is swept, not a curtain: see [`SWEPT`].
                Self::Bob { fringe } => 1.0 - crate::face::smooth(fringe.clamp(0.0, 1.0) / SWEPT),
                _ => 0.0,
            },
            room: match self {
                Self::Bob { fringe } => ROOM[0] + (ROOM[1] - ROOM[0]) * fringe.clamp(0.0, 1.0),
                _ => ROOM[0],
            },
        }))
    }

    fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize {
        // A helmet's count is its rim's, and the rim is a fixed ring of
        // meridians rather than a density: the solid is the mass, so a thinner
        // cut thins nothing and a denser one has nothing to fill.
        if let Some(cards) = self.rim_cards() {
            return cards;
        }
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
            Self::Cap { fringe } => *fringe = scaled::quantize(fringe.clamp(0.0, 1.0)),
            Self::SlickBack { volume } => *volume = scaled::quantize(volume.clamp(0.0, 1.0)),
            Self::Bell { length } => *length = scaled::quantize(length.clamp(0.0, 1.0)),
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
            // A helmet has no row in the tables above: they describe a card's
            // reach, width, taper and crowd, and a shell has none of those. See
            // [`Self::helmet`].
            Self::Cap { .. } | Self::SlickBack { .. } | Self::Bell { .. } => None,
        }
    }

    /// The solid this style wears, if it is one of the helmet family (#346).
    ///
    /// **One place the catalogue's three shells are described**, so a style is a
    /// variant here and a handful of numbers in `hair::shell` rather than a
    /// second opinion in every caller. `tests/budget.rs` costs a helmet through
    /// the same call the body draws it with.
    pub(crate) fn helmet(self, head: &Follicles) -> Option<Cap> {
        match self {
            Self::Cap { fringe } => Some(Cap::crop(fringe)),
            Self::SlickBack { volume } => Some(Cap::slicked(volume)),
            Self::Bell { length } => Some(Cap::bell(length, head)),
            _ => None,
        }
    }

    /// How many meridians a helmet style's rim is seated on, if it is one.
    ///
    /// Read without a measured head, which `Style::clumps` has none of: a rim's
    /// count is a ring and not a shape, so it does not need one.
    fn rim_cards(self) -> Option<usize> {
        match self {
            Self::Cap { .. } => Some(Cap::crop(0.0).rim_cards),
            Self::SlickBack { .. } => Some(Cap::slicked(0.0).rim_cards),
            Self::Bell { .. } => Some(Cap::default().rim_cards),
            _ => None,
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
/// **The shell generator's rim is this same weight** (#345), taken from here
/// rather than copied: where a card stops lying on the scalp is where the hair
/// ends, and a shell that ended anywhere else would end somewhere no card does.
///
/// Provenance: **derived** from the mask's own fade.
pub(crate) const EDGE: f32 = 0.35;

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

/// The same for a tied-back head, whose back cards rise from the nape and
/// leave fewer to cross the crown behind the pole (#342).
///
/// Provenance: **tuned by render**.
const FAN_OVER_TIED: f32 = 0.03;

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

/// Under what `fringe` a bob is swept rather than fringed.
///
/// **A bob's `fringe` of `0` used to be an even curtain** (#341): the front as
/// long as the sides, which at a full cut is 130 mm hanging straight over the
/// eyes, and the axis documented it as exactly that. Nobody asks for a curtain
/// over the face; the bob without a fringe is the one parted at the front and
/// swept to the temples, so that is what `0` is now. The sweep eases off over
/// the bottom of the axis, so a record just above it is a long fringe and not a
/// jump.
///
/// Provenance: **tuned by render**.
const SWEPT: f32 = 0.25;

/// How far a lock hanging in front of the face may fall, as a share of the room
/// between where it leaves the scalp and the brow: at a bob's `fringe` of `0`
/// and of `1`. The first is every other style's.
///
/// **A fringe is a height, not a length** (#341). It was a share of the cut's
/// reach, so a bob cut long grew a fringe to the nose whatever its own axis
/// said: rolled seed 42 is a bob at `fringe` 0.73 - "well above the brow" - and
/// it hung eleven front locks 40 to 60 mm under the brow. A share of the
/// forehead reaches the same anatomy on any cut and any head, and it only ever
/// shortens what the reach asked for.
///
/// The top end leaves the stagger its room: on the default head's 50 mm of
/// forehead, 0.85 grown by half of [`STAGGER_LEAST`] is 0.93 of the way down,
/// so a lock ends on the forehead. On a forehead short enough that it would
/// not, the walk's floor stops it at the clearance instead.
///
/// Provenance: **derived** (the top end, from the stagger), **tuned by render**
/// (the bottom end).
const ROOM: [f32; 2] = [0.85, 0.30];

/// Over how far inside the temple walls a lock goes from the reach's length to
/// the forehead's, in metres.
///
/// A lock a hair outside the face hangs its whole length and one a hair inside
/// it stops above the brow, and a step that sharp between two neighbours is a
/// notch at the temple. A centimetre is about a quarter of a card.
///
/// Provenance: **tuned by render**.
const OVER: f32 = 0.012;

/// How finely a leg of the walk is checked against the face, in metres.
///
/// A hanging leg is a straight line, but the lock drawn along it coils, and at
/// the tightest [`SWING`] and [`WAVE`] the coil turns through a whole wave
/// inside one leg. It is a helix of 17 mm radius and 10 mm pitch per radian,
/// curving at 0.044 per millimetre, so chords of two millimetres of travel
/// (eight of arc) sag 0.09 mm from it: inside [`CLEAR`].
///
/// Provenance: **derived** from the coil's own curvature.
const CHECK: f32 = 0.002;

/// How far outside the face a check keeps the chords of a drawn lock, in
/// metres.
///
/// Provenance: **derived**: over the sag [`CHECK`] leaves, with room for the
/// walk's own float arithmetic, which a lock stopped exactly on a wall would
/// otherwise cross.
const CLEAR: f32 = 0.0005;

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

/// How far round the pole a tied-back card turns as it leaves the crown, in
/// radians (#342).
///
/// Provenance: **tuned by render**.
const WHORL: f32 = 0.9;

/// Over how much descent below the crown that turn is made, in metres.
///
/// Provenance: **tuned by render**.
const WHORL_OVER: f32 = 0.045;

/// How far below the crown that turn starts, in metres: above it a card is
/// only turned, as a whole, by the whorl's full amount.
///
/// Provenance: **tuned by render**.
const WHORL_FROM: f32 = 0.008;

/// The power of the descent that turn is scheduled on: a half is even in the
/// distance from the pole, which on a dome is a spiral; one was gentler at the
/// pole and opened a rosette of scalp there on the sheet.
///
/// Provenance: **tuned by render**.
const WHORL_POW: f32 = 0.5;

/// What share of the back's gathered cards rise to the knot from the nape, as
/// a multiple of the share of their meridian's scalp that lies under the knot
/// (#342).
///
/// Provenance: **derived**: one, a card for the scalp it has to cover.
const RISE: f32 = 1.0;

/// How far round the head a card's meridian must be for it to rise, in
/// radians from dead ahead.
///
/// **Only the back of the head**: a card from just behind the ear climbed a
/// quarter of the way round the head to the knot, and a wide flat card on that
/// diagonal read as a plate standing off the nape on the sheet.
///
/// Provenance: **tuned by render**.
const RISE_FROM: f32 = 2.3;

/// How many steps a rising card takes from the nape's hairline to the knot.
///
/// Provenance: **derived** from the turn it makes on the way: up to a
/// quarter of the head round in sixteen steps is under six degrees a step.
const RISE_STEPS: usize = 16;

/// How finely the nape's hairline is searched for below a rising card's root,
/// in metres.
///
/// Provenance: **derived** from the hairline's own fade, which is wider.
const RISE_SEARCH: f32 = 0.002;

/// How far above the nape's hairline a rising card may start, the most, in
/// metres: its own salt picks where in that.
///
/// Provenance: **tuned by render**.
const RISE_STAGGER: f32 = 0.012;

/// What share of its width a rising card has where it starts.
///
/// Provenance: **tuned by render**.
const RISE_ROOT: f32 = 0.4;

/// Over how far a rising card reaches its full width, in metres.
///
/// Provenance: **tuned by render**.
const RISE_FEATHER: f32 = 0.015;

/// How far the tail's cards are turned about the tail, the furthest of them,
/// in radians (#342).
///
/// Provenance: **tuned by render**.
const CROSS: f32 = std::f32::consts::FRAC_PI_2;

/// What share of its width a tail card keeps where it is gathered (#342).
///
/// Provenance: **tuned by render**.
const GATHER: f32 = 0.35;

/// Over how much of its fall below the scalp a tail card opens to its width,
/// in metres.
///
/// Provenance: **tuned by render**.
const GATHER_OVER: f32 = 0.025;

/// What share of a tail card's hang its point is tapered over (#342).
///
/// Provenance: **tuned by render**.
const TAIL_TAPER: f32 = 1.0 / 3.0;

/// The knot lump's half-extents across, up and out, in metres (#342).
///
/// Provenance: **tuned by render**.
const LUMP: [f32; 3] = [0.014, 0.011, 0.009];

/// Where the lump's middle sits, as a share of the knot's standoff.
///
/// Provenance: **tuned by render**.
const LUMP_AT: f32 = 0.93;

/// How much of the roots' colour the lump keeps.
///
/// Provenance: **tuned by render**.
const LUMP_SHADE: f32 = 0.85;

/// What share of its width a tail card opens to below the knot (#342).
///
/// Provenance: **tuned by render**.
const TAIL_WIDTH: f32 = 0.55;

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

/// How far one card's shade sits from its neighbour's, either way, as a share
/// of its colour.
///
/// **Per card and correlated with nothing** (#339). Every card of a head used
/// to be drawn in exactly the record's two colours, so at any distance the mass
/// was one shade with a torn rim; real hair is never one tone, and the eye
/// separates locks by the step in shade between them before it resolves their
/// edges. Hashed from the root like the stagger, in a lane of its own, so the
/// step is a feathering and not a stripe.
///
/// Provenance: **tuned by render**, from the issue's six per cent. Sheeted on
/// its own, it separated a curtain into locks and moved nothing else.
const TONE: f32 = 0.06;

/// How much darker a card is where it lies on the scalp than where it hangs
/// free, as a share of its colour.
///
/// **The part of a card lying on the scalp lies under the cards that cross it,
/// and the part hanging past the hairline is the outside of the mass** (#339).
/// Read off the card's own walk - how far past the hairline a station is, eased
/// over the same [`LOOSE`] the volume and the coil are - rather than searched
/// for among the other cards, so it costs nothing a loft does not already know.
///
/// **Halved from 0.30 at the first look**: over every scalp-lying station of a
/// crop - which is nearly all of one - a record's blond rendered brown and a
/// light long head read as dark roots, and the colour somebody picked is not
/// this constant's to change.
///
/// Provenance: **tuned by render**.
const SHADOW: f32 = 0.15;

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
    /// What share of the room between where it leaves the scalp and the brow a
    /// lock hanging in front of the face may fall; see [`ROOM`].
    room: f32,
    /// How far round the pole the card turns leaving the crown; see [`WHORL`].
    whorl: f32,
}

impl Sheet {
    /// Which way round the head a root sits, from dead ahead.
    fn azimuth(root: &Root) -> f32 {
        root.at.x.atan2(root.at.z)
    }

    /// How far round the pole a card has turned by `height`, below `top`:
    /// the whole of [`Self::whorl`] back from its own meridian down to
    /// [`WHORL_FROM`], and none from [`WHORL_OVER`] down, where the card is
    /// on its meridian again.
    ///
    /// In the square root of the descent, because on a dome that is the
    /// distance from the pole: a turn even in the radius is a spiral.
    fn twist(&self, top: f32, height: f32) -> f32 {
        if self.whorl <= 0.0 {
            return 0.0;
        }
        let share = ((top - height - WHORL_FROM) / (WHORL_OVER - WHORL_FROM))
            .clamp(0.0, 1.0)
            .powf(WHORL_POW);
        self.whorl * (share - 1.0)
    }

    /// The most of `half` a ringlet keeps a share `along` of the way down it
    /// without an edge running backwards: [`FOLD_CAP`] of the radius the drawn
    /// lock bends through in the plane its width lies in (#343). Every other
    /// style, and a curl's very ends, keep `half` as it is.
    fn unfolded(&self, root: &Root, along: f32, half: f32) -> f32 {
        if FOLD_CAP <= 0.0 || self.curl <= 0.0 {
            return half;
        }
        let step = FOLD_STEP / self.length(root).max(f32::EPSILON);
        if along - step < 0.0 || along + step > 1.0 {
            return half;
        }
        let (behind, here, ahead) = (
            self.at(root, along - step),
            self.at(root, along),
            self.at(root, along + step),
        );
        let (one, two) = (here - behind, ahead - here);
        let travel = 0.5 * (one.length() + two.length());
        if travel <= f32::EPSILON {
            return half;
        }
        // How fast the heading turns, per metre of the lock: the curvature,
        // pointing into the bend.
        let bend = (two.normalize_or(Vec3::ZERO) - one.normalize_or(Vec3::ZERO)) / travel;
        let heading = (ahead - behind).normalize_or(Vec3::ZERO);
        let named = self.across_at(root, along);
        let side = (named - heading * named.dot(heading)).normalize_or(Vec3::ZERO);
        let inside = side.dot(bend).abs();
        if inside <= f32::EPSILON {
            return half;
        }
        half.min(FOLD_CAP / inside)
    }

    /// Whether this card's end is a tail's: gathered to a knot.
    fn tailed(&self, root: &Root) -> bool {
        self.knot.is_some() && Self::pulled(Self::azimuth(root)) >= 0.5
    }

    /// Whether this card rises to the knot from the nape rather than falling
    /// to it from the crown: see [`Self::risen`].
    ///
    /// **Chosen by the card's own salt, in proportion to how much of its
    /// meridian's scalp is under the knot**, and not by where its root sits:
    /// a root is seated anywhere down its sector, so under a knot at the
    /// middle of the head few roots were below it and the nape stayed bare.
    fn rises(&self, root: &Root) -> bool {
        let Some(knot) = self.knot else {
            return false;
        };
        let from = Self::azimuth(root);
        if from.abs() < RISE_FROM || !self.tailed(root) {
            return false;
        }
        let low = self.nape(from, Self::edge(root), knot);
        let (_, crown) = self.regions.skull().throat_and_crown();
        let under = ((knot.y - low) / (crown - low).max(f32::EPSILON)).clamp(0.0, 1.0);
        Self::salt(root, 3) < under * RISE
    }

    /// How high the nape's hairline is under the knot down one meridian: where
    /// the mask falls under `edge`, searched down from the knot's height.
    fn nape(&self, azimuth: f32, edge: f32, knot: Vec3) -> f32 {
        let skull = self.regions.skull();
        let (throat, _) = skull.throat_and_crown();
        let mut low = knot.y;
        while low - RISE_SEARCH > throat
            && self.regions.weight(
                Follicle::Scalp,
                skull.surface_at(low - RISE_SEARCH, azimuth),
            ) >= edge
        {
            low -= RISE_SEARCH;
        }
        low
    }

    /// The walk of a card that rises to the knot from the nape (#342).
    ///
    /// **Hair under a tail is brushed UP into it.** Every card starts at the
    /// crown and a gathered one leaves the scalp for the knot at the knot's
    /// height, so nothing at all lay on the scalp between the knot and the
    /// nape's hairline: measured on the default head at a tail of 0.6, most
    /// of the 76 cm2 of painted scalp behind the temples that no card covered.
    /// A rising card starts instead just above the hairline under the knot,
    /// climbs the skull turning toward the back of the head, meets the knot,
    /// and hangs as tail like every other gathered card.
    fn risen(&self, root: &Root, want: f32, knot: Vec3) -> Walked {
        let from = Self::azimuth(root);
        let back = std::f32::consts::PI * if from < 0.0 { -1.0 } else { 1.0 };
        let skull = self.regions.skull();
        // From where the mask gives out under the knot, each card a little
        // above it by its own salt: cards that all start on the hairline
        // draw it as a hem.
        let low = self.nape(from, Self::edge(root), knot);
        let low = (low + RISE_STAGGER * Self::salt(root, 0)).min(knot.y);
        let mut points: Vec<Vec3> = (0..=RISE_STEPS)
            .map(|step| {
                let share = step as f32 / RISE_STEPS as f32;
                let azimuth = from + (back - from) * crate::face::smooth(share);
                skull.surface_at(low + (knot.y - low) * share, azimuth)
            })
            .collect();
        // The first leg that leaves the scalp is the one to the knot.
        let scalp = points.len();
        points.push(knot);
        points.extend(
            (1..=HANG).map(|hung| knot + Vec3::NEG_Y * (self.reach * hung as f32 / HANG as f32)),
        );
        let (mut gone, mut free) = (0.0f32, 0.0f32);
        let (mut cap, mut left) = (None, None);
        for index in 1..points.len() {
            let (from, to) = (points[index - 1], points[index]);
            let leg = from.distance(to);
            let hanging = index >= scalp;
            if index == scalp {
                cap = Some(gone);
                left = Some(from);
            }
            if gone + leg >= want {
                let share = if leg > f32::EPSILON {
                    ((want - gone) / leg).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return Walked {
                    at: from.lerp(to, share),
                    gone: want,
                    free: if hanging { free + leg * share } else { 0.0 },
                    grows: true,
                    cap,
                    left,
                    floor: None,
                };
            }
            gone += leg;
            if hanging {
                free += leg;
            }
        }
        Walked {
            at: points[points.len() - 1],
            gone,
            free,
            grows: true,
            cap,
            left,
            floor: None,
        }
    }

    /// How far this lock hangs past the hairline, before the mask's own thinning.
    ///
    /// **The curtain lesson**: a share of the reach at the front and a multiple
    /// of it at the back, so a fringe stops above the eyes while the same style's
    /// sides reach the jaw. Uniform length is what makes hair read as a hood.
    ///
    /// **And over the face, a share of the forehead** (#341): a lock whose hang
    /// lands between the temples takes the lesser of the reach's length and
    /// [`Self::room`] of the way from where it left the scalp down to the brow,
    /// eased in over [`OVER`] inside the temple walls. `walked` is this lock's
    /// walk to the end, which is what says where it left.
    fn fall(&self, root: &Root, walked: &Walked) -> f32 {
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
        let reached = (hang + spread * (Self::salt(root, 0) - 0.5)).max(0.0);
        let Some(left) = walked.left else {
            return reached;
        };
        // Where this lock hangs once it is level with the brow: the radius it
        // left the scalp with, at the azimuth the comb has turned it to by then.
        // A parted lock has left the face by that height, and one going straight
        // down is still where it left.
        let face = self.regions.clearance();
        let (_, crown) = self.regions.skull().throat_and_crown();
        let azimuth = self.combed(Self::azimuth(root), crown, face.brow);
        let held = (left.x * left.x + left.z * left.z).sqrt();
        let inside =
            (face.side - (held * azimuth.sin()).abs()).min(held * azimuth.cos() - face.front);
        let over = crate::face::smooth(inside / OVER);
        if over <= 0.0 {
            return reached;
        }
        // Staggered exactly as the reach is - the same lane, so a fringe keeps
        // the order its locks had, and the same least spread, because a share
        // of a short fringe is under what the render resolves and its tips
        // went back to ending on one line without it.
        let forehead = (left.y - face.brow).max(0.0) * self.room;
        let spread = (forehead * STAGGER).max(STAGGER_LEAST);
        let forehead = (forehead + spread * (Self::salt(root, 0) - 0.5)).max(0.0);
        reached - (reached - reached.min(forehead)) * over
    }

    /// How far a drawn lock can stand off its own walk sideways, in metres: its
    /// volume, its lift, its coil, and the margin a check keeps.
    ///
    /// The coil swings round the fall in a circle, so either horizontal axis can
    /// carry up to the whole swing of both its terms: a root two of it.
    fn stray(&self) -> f32 {
        let swing = if self.curl > 0.0 {
            SWING[0] + (SWING[1] - SWING[0]) * self.curl
        } else {
            0.0
        };
        self.volume + LIFT + std::f32::consts::SQRT_2 * swing + CLEAR
    }

    /// Where a lock is drawn, given where its walk is: `walked` after `travel`
    /// metres, `free` of them past the hairline, lifted along `lift`.
    ///
    /// **One place, because two readers need it**: [`Shape::at`], which draws
    /// the lock, and the walk's check against the face, which has to keep the
    /// DRAWN lock clear and not merely its spine (#341). A coil swings a curly
    /// lock 17 mm off its walk, which is two thirds of the way across a temple
    /// wall's margin.
    fn dress(&self, walked: Vec3, out: Vec3, travel: f32, free: f32, lift: Vec3) -> Vec3 {
        let mut at = walked;
        let loose = crate::face::smooth(free / LOOSE);
        at += out * (self.volume * loose) + lift * LIFT;
        // And coiled, if it is a curl: a wave across its own fall rather than a
        // helix round it. See [`COIL`] for what the helix cost.
        if self.curl > 0.0 {
            let swing = (SWING[0] + (SWING[1] - SWING[0]) * self.curl) * loose;
            let wave = WAVE[0] + (WAVE[1] - WAVE[0]) * self.curl;
            // Around the fall rather than across it, so a ringlet reads as one
            // from any angle: a wave in one plane is a kink from the side and
            // nothing at all from the front.
            let phase = travel * std::f32::consts::TAU / wave;
            let sideways = out.cross(Vec3::Y).normalize_or(Vec3::X);
            at += sideways * (swing * phase.sin()) + out * (swing * phase.cos());
        }
        at
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
        if let Some(knot) = self.knot
            && self.rises(root)
        {
            return self.risen(root, want, knot);
        }
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
        // Where the walk's own first step would put it, twist and all: a
        // whorled card that started at its root's azimuth crossed the pole
        // sideways, and a card narrow at the pole lying across it leaves a
        // rosette of petals between its neighbours (#342).
        let mut at = skull.surface_at(top, from + self.twist(top, top));
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
        // Where the lock left the scalp, once it has.
        let mut left_scalp: Option<Vec3> = None;
        // **The face is kept clear by construction** (#341): the travel at
        // which this lock, as drawn, would first enter the space in front of
        // the face. A lock's length stops there (see [`Shape::length`]), so no
        // point of it is ever inside, however long the cut, the fringe or the
        // coil - where a test bounding a SHARE of the hair in front of the face
        // passed while a bob's fringe hung to its nose.
        let face = self.regions.clearance();
        let out = Vec3::new(from.sin(), 0.0, from.cos());
        let stray = self.stray();
        let mut floor: Option<f32> = None;
        // One leg, `gone` metres and `free` of them hanging at its start. A leg
        // nowhere near the face is passed over on its spine, grown by how far a
        // drawn lock can stray from it; a leg near it is checked as drawn, in
        // chords [`CHECK`] long, since a coil swings a whole wave inside one
        // hanging leg.
        let enters = |from: Vec3, to: Vec3, gone: f32, free: f32, hanging: bool| -> Option<f32> {
            face.entry(from, to, stray)?;
            let leg = from.distance(to);
            let pieces = (leg / CHECK).ceil().max(1.0) as usize;
            let drawn = |share: f32| {
                let free = if hanging { free + leg * share } else { free };
                let walked = from.lerp(to, share);
                let lift = if free > 0.0 {
                    out
                } else {
                    self.normal(walked.y, Self::azimuth(root))
                };
                self.dress(walked, out, gone + leg * share, free, lift)
            };
            let mut before = drawn(0.0);
            for piece in 1..=pieces {
                let after = drawn(piece as f32 / pieces as f32);
                if let Some(into) = face.entry(before, after, CLEAR) {
                    return Some(gone + leg * ((piece - 1) as f32 + into) / pieces as f32);
                }
                before = after;
            }
            None
        };
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
                left_scalp = Some(at);
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
                        left: left_scalp,
                        floor,
                    };
                }
                if floor.is_none() {
                    floor = enters(at, next, gone, free - leg, true);
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
            let azimuth = self.combed(from, top, height) + self.twist(top, height);
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
                    left: left_scalp,
                    floor,
                };
            }
            if floor.is_none() {
                let hanging = hung.is_some();
                let from_free = if hanging { free - leg } else { free };
                floor = enters(at, next, gone, from_free, hanging);
            }
            gone += leg;
            at = next;
            let grows = self.regions.weight(Follicle::Scalp, next) >= edge;
            begun |= grows;
            if begun && hung.is_none() && !grows {
                hung = Some(held);
                cap = Some(gone);
                left_scalp = Some(next);
            }
        }
        Walked {
            at,
            gone,
            free,
            grows: begun,
            cap,
            left: left_scalp,
            floor,
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
    /// Where it crossed the hairline, head-local, if it did: what a lock over
    /// the face measures the room above the brow from (#341).
    left: Option<Vec3>,
    /// How far it had travelled when the lock, as drawn, would first have
    /// entered the space in front of the face, if it would (#341).
    floor: Option<f32>,
}

impl Shape for Sheet {
    fn length(&self, root: &Root) -> f32 {
        // One walk to the end answers all of it: whether hair grows here, where
        // the cap ends, where the lock left the scalp and where it would reach
        // the face.
        let walked = self.walked(root, f32::MAX);
        // Nothing at all where no hair grows down this meridian: see
        // [`Walked::grows`]. This is what [`Shape::length`]'s zero is for, and it
        // hands the triangles back rather than spending them on a slab.
        if !walked.grows {
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
        let reached = walked.cap.unwrap_or(walked.gone) + self.fall(root, &walked) * thinned;
        // And never into the face: see [`Walked::floor`].
        walked.floor.map_or(reached, |floor| reached.min(floor))
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let length = self.length(root);
        let walked = self.walked(root, length * along);
        // Standing off the skull, but only once it is off the head: this is what
        // `droop` gives up. Radially, because that is the direction a head pushes
        // hair. A card lying on a scalp has no volume to give.
        let azimuth = Self::azimuth(root);
        let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
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
        self.dress(walked.at, out, length * along, walked.free, lift)
    }

    fn seating(&self) -> Seating {
        // A root is a meridian here: see [`Seating`], and the bald side of a
        // tied-back head that found it.
        Seating::Meridians
    }

    fn shade_at(&self, root: &Root, along: f32) -> f32 {
        // A lane of its own, so the tone is correlated with neither the length
        // stagger (lane 0) nor where the card leaves the hairline (lane 1).
        let tone = 1.0 + TONE * (2.0 * Self::salt(root, 2) - 1.0);
        // How far past the hairline this station hangs, eased the way the
        // volume and the coil are: lying on the scalp is lying under the cards
        // that cross it, and hanging free is the outside of the mass.
        let walked = self.walked(root, self.length(root) * along.clamp(0.0, 1.0));
        let lying = 1.0 - crate::face::smooth(walked.free / LOOSE);
        tone * (1.0 - SHADOW * lying)
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
        let across = normal.cross(heading).normalize_or(self.across(root));
        // **A ringlet's width lies along its coil's binormal** (#343): across
        // the coil's own radial and the way the drawn lock is heading, so the
        // width never lies in the plane the coil turns in and neither edge runs
        // backwards (see [`TWIST`]). The coil's frame is [`Self::dress`]'s.
        //
        // **As an angle about the spine that never jumps.** Measured from the
        // card's lying axis, the binormal's angle IS the coil's phase, give or
        // take how far the drawn lock is from an ideal helix. So the turn is
        // the phase wound in as the swing comes on (the smooth ramp over
        // [`LOOSE`], integrated), plus that small departure and a constant for
        // where on the coil this card left the scalp, both brought in with the
        // swing and each taken as an axis. Once the swing is full the width is
        // on the binormal; nowhere does it flip between two stations, which a
        // blend of two axes did wherever they were square.
        if TWIST > 0.0 && self.curl > 0.0 && walked.free > 0.0 && heading != Vec3::ZERO {
            use std::f32::consts::{FRAC_PI_2, PI, TAU};
            // Into (-a quarter turn, a quarter turn]: a width is an axis.
            let axis = |angle: f32| angle - PI * (angle / PI).round();
            let wave = WAVE[0] + (WAVE[1] - WAVE[0]) * self.curl;
            let travel = self.length(root) * along;
            let phase = travel * TAU / wave;
            let from = Self::azimuth(root);
            let out = Vec3::new(from.sin(), 0.0, from.cos());
            let sideways = out.cross(Vec3::Y).normalize_or(Vec3::X);
            let coil = sideways * phase.sin() + out * phase.cos();
            let wound = coil.cross(heading);
            let beside = heading.cross(across);
            let departure = axis(wound.dot(beside).atan2(wound.dot(across)) - phase);
            // The phase wound in over the ramp: the integral of the smooth
            // step, x^3 - x^4 / 2 over it and a half short of x past it.
            let loose = walked.free / LOOSE;
            let ramped = if loose <= 1.0 {
                loose.powi(3) - 0.5 * loose.powi(4)
            } else {
                loose - 0.5
            };
            let per = TAU / wave;
            let left = axis((travel - walked.free) * per + 0.5 * LOOSE * per);
            let share = crate::face::smooth(loose);
            let turn = TWIST.min(1.0) * (LOOSE * per * ramped + share * (departure + left));
            debug_assert!(departure.abs() <= FRAC_PI_2 + 1e-3);
            return Quat::from_axis_angle(heading, turn) * across;
        }
        // **A tail is a bundle, turned about itself card by card** (#342).
        // Every gathered card hangs from one knot facing straight back, so
        // from the side the tail was every card edge-on: a rope. Turned by
        // how far round the head its card started, the back's cards face
        // sideways and the sides' face back, and the bundle has a body from
        // any angle.
        if self.tailed(root) && walked.free > 0.0 && heading != Vec3::ZERO {
            let from = Self::azimuth(root);
            let first = (PULL_FROM - PULL_FADE * 0.5).clamp(-1.0, 1.0).acos();
            let round = ((from.abs() - first) / (std::f32::consts::PI - first)).clamp(0.0, 1.0);
            let turn = CROSS * round * if from < 0.0 { -1.0 } else { 1.0 };
            return Quat::from_axis_angle(heading, turn) * across;
        }
        across
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
            // A card rising from the nape has no pole to fan out of: it is a
            // lock lying on the scalp, feathered in over its first
            // centimetre and a half, because rising cards that all started
            // at full width drew the nape's hairline as a hem (#342).
            if self.rises(root) {
                let travel = along * self.length(root);
                return base
                    * (RISE_ROOT + (1.0 - RISE_ROOT) * crate::face::smooth(travel / RISE_FEATHER));
            }
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
            // A tail's back cards rise from the nape, and the ones left
            // to cross the crown behind the pole have more of it to share:
            // they reach their width sooner (#342).
            let over = if self.knot.is_some() {
                FAN_OVER_TIED
            } else {
                FAN_OVER
            };
            let fan = FAN + (1.0 - FAN) * (travel / over).clamp(0.0, 1.0);
            return base * fan;
        }
        // Full width down the hang, then to the tip over the last [`TIP`]
        // of it: a lock's point is at its end, not spread over its length.
        let length = self.length(root);
        let hang = length * (1.0 - cap);
        let left = (1.0 - along) * length;
        // **A tail tapers over its last third and is gathered at its top**
        // (#342): narrow where the knot holds it, full below, and to a point
        // over the end rather than over the last few centimetres.
        let tailed = self.tailed(root);
        let span = if tailed {
            TIP.max(hang * TAIL_TAPER)
        } else {
            TIP
        };
        let past = 1.0 - (left / span.min(hang).max(f32::EPSILON)).clamp(0.0, 1.0);
        let width = base + (tip - base) * past;
        if !tailed {
            return self.unfolded(root, along, width);
        }
        let below = (along - cap) * length;
        width * (GATHER + (TAIL_WIDTH - GATHER) * crate::face::smooth(below / GATHER_OVER))
    }

    fn lump(&self) -> Option<Lump> {
        // The knot, where every gathered card meets: see [`Shape::lump`].
        let knot = self.knot?;
        Some(Lump {
            centre: Vec3::new(0.0, knot.y, knot.z * LUMP_AT),
            radii: Vec3::from_array(LUMP),
            shade: LUMP_SHADE,
        })
    }

    fn seamed(&self) -> bool {
        // A ringlet turns over with its coil: see [`Shape::seamed`].
        SEAMED && self.curl > 0.0
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
