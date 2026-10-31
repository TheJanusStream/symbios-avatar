//! Where a beast grows long hair, and what each card there does (#369).
//!
//! A zone is two things: a **mask** over the body - how much of a point is
//! the zone's, `0..=1`, read off the anatomy the coat painter reads (the
//! spine, the back's height, the tail, the declared head) and the bone a
//! point hangs off - and a **curve**, the [`Shape`] a card rooted there
//! follows. A card leaves the skin at a shallow angle toward its zone's own
//! way - a mane over the neck's side, a forelock down the face, a ruff back
//! along the body, a brush toward the tail's tip, feathering to the ground -
//! and bends toward the ground the longer it is, as [`super::super::clump::Fall`]
//! does. A mane's cards alone have a curve of their own, draped over the neck
//! and standing off its crest as stiffly as the record says (see
//! [`Lock::draped`] and [`Stand`]).

use std::f32::consts::PI;

use glam::Vec3;

use super::{Bed, Crest};
use crate::face::step;
use crate::hair::clump::{LIFT, Root, Shape};
use crate::plan::{Fur, Zone};

/// Where on a beast long hair grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FurZone {
    /// Down the neck's crest, from the poll to the withers.
    Mane,
    /// Between the ears, falling over the face.
    Forelock,
    /// Round the neck, over the chest's front and up behind the jaw.
    Ruff,
    /// Along the tail, past its dock.
    Tail,
    /// Down the back of the fetlocks.
    Feathering,
    /// At the ears' tips.
    Tufts,
}

/// How a zone's cards are cut, as shares of the back's height where they are
/// lengths, at a record's full length of `1`.
/// Provenance: **tuned by render** (#369).
#[derive(Clone, Copy, Debug)]
pub(super) struct Cut {
    /// How many cards the zone asks for.
    pub count: usize,
    /// How long a card is.
    pub reach: f32,
    /// Half a card's width at its root.
    pub width: f32,
    /// What share of that is left at its tip.
    pub taper: f32,
    /// How far it lies along its zone's way where it leaves the skin, `0`
    /// standing straight out and `1` flat.
    pub lie: f32,
    /// How far it bends to the ground over its length.
    pub droop: f32,
    /// What share of its width it has where it enters the skin, opening to
    /// the whole of it [`OPENS`] of the way along: the shorter that line, the
    /// less of it there is to see. `1` is a card that meets no skin and so
    /// is not sunk into any.
    pub pinch: f32,
}

impl FurZone {
    /// Every zone, in the order they are grown.
    pub const ALL: [Self; 6] = [
        Self::Mane,
        Self::Forelock,
        Self::Ruff,
        Self::Tail,
        Self::Feathering,
        Self::Tufts,
    ];

    /// Its name, for a ledger.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Mane => "mane",
            Self::Forelock => "forelock",
            Self::Ruff => "ruff",
            Self::Tail => "tail",
            Self::Feathering => "feathering",
            Self::Tufts => "tufts",
        }
    }

    /// How long a record says its fur is: a forelock is the mane's.
    #[must_use]
    pub fn length_in(self, fur: &Fur) -> f32 {
        match self {
            Self::Mane | Self::Forelock => fur.mane,
            Self::Ruff => fur.ruff,
            Self::Tail => fur.tail,
            Self::Feathering => fur.feathering,
            Self::Tufts => fur.tufts,
        }
    }

    /// Its cards' cut.
    pub(super) fn cut(self) -> Cut {
        let cut = |count, reach, width, taper, lie, droop, pinch| Cut {
            count,
            reach,
            width,
            taper,
            lie,
            droop,
            pinch,
        };
        match self {
            // Its lie goes unread: `Lock::draped` lies it on the neck.
            Self::Mane => cut(96, 0.30, 0.028, 0.5, 1.0, 0.2, 0.3),
            Self::Forelock => cut(20, 0.2, 0.028, 0.35, 0.45, 0.8, 0.3),
            Self::Ruff => cut(240, 0.13, 0.026, 0.45, 0.7, 0.3, 0.3),
            Self::Tail => cut(110, 0.38, 0.03, 0.45, 0.75, 0.4, 0.3),
            Self::Feathering => cut(56, 0.12, 0.03, 0.5, 0.75, 0.6, 0.3),
            // Grown off an ear's tip, into the air: it meets no skin to hide.
            Self::Tufts => cut(5, 0.09, 0.012, 0.15, 0.1, 0.0, 1.0),
        }
    }

    /// Whether its hair is a mane's or a tail's, which a coat that darkens
    /// its points - a bay's, a dun's - draws in its dark.
    pub(super) fn points(self) -> bool {
        matches!(self, Self::Mane | Self::Forelock | Self::Tail)
    }

    /// How much of a point of the body is this zone's, `0..=1`, at `at`
    /// facing along `normal` in the body zone `zone`.
    pub(super) fn weight(self, bed: &Bed, at: Vec3, normal: Vec3, zone: Zone) -> f32 {
        let anatomy = bed.anatomy;
        match self {
            // Rooted along the crest the rig draws, not by a mask: see
            // `crest`.
            Self::Mane => 0.0,
            // The poll, between the ears and just ahead of them.
            Self::Forelock => {
                let Some(head) = anatomy.head.as_ref().filter(|_| zone == Zone::Head) else {
                    return 0.0;
                };
                let local = head.local(at) / head.radius;
                step(0.55, 0.8, local.y)
                    * step(0.32, 0.18, local.x.abs())
                    * step(-0.55, -0.35, local.z)
                    * step(0.35, 0.15, local.z)
            }
            // All round the neck, and over the chest's front where the neck
            // meets it, thinning out down the top of each foreleg rather than
            // stopping on the line the skin's zones draw round it (#376).
            Self::Ruff => match zone {
                Zone::Neck => 1.0,
                Zone::Chest | Zone::UpperLimb(_) => {
                    step(0.78, 0.95, anatomy.along_spine(at))
                        * step(0.0, 0.4, normal.dot(bed.forward()))
                        * (1.0 - anatomy.junctions.share(at))
                }
                // And up behind the jaw onto the cheeks and the throat, where a
                // ruff frames a face: a big head hides a ruff on the neck alone
                // from the front.
                Zone::Head => anatomy.head.as_ref().map_or(0.0, |head| {
                    let local = head.local(at) / head.radius;
                    step(0.2, -0.15, local.z) * step(0.3, -0.05, local.y)
                }),
                _ => 0.0,
            },
            // The tail, past the dock it springs from.
            Self::Tail => {
                if zone == Zone::Tail {
                    step(0.05, 0.25, anatomy.along_tail(at))
                } else {
                    0.0
                }
            }
            // The back of the leg, from the hoof's or the paw's top to a
            // little above the fetlock.
            Self::Feathering => {
                if !matches!(zone, Zone::LowerLimb(_) | Zone::Extremity(_)) {
                    return 0.0;
                }
                let rise = anatomy.rise(at);
                step(0.02, 0.07, rise)
                    * step(0.34, 0.22, rise)
                    * step(0.1, 0.5, normal.dot(-bed.forward()))
            }
            // Grown on the ears, not on the body.
            Self::Tufts => 0.0,
        }
    }

    /// The way a card rooted at `root` combs toward, before it bends to the
    /// ground.
    pub(super) fn way(self, bed: &Bed, root: &Root) -> Vec3 {
        let forward = bed.forward();
        match self {
            // Drawn by `Lock::draped`, which has a crest to follow; there is no
            // way to comb a mane without one.
            Self::Mane => root.out,
            // Forward and down over the face.
            Self::Forelock => (bed.head_forward() * 0.5 + Vec3::NEG_Y * 0.5).normalize(),
            // Back along the body and a little down.
            Self::Ruff => (-forward * 0.7 + Vec3::NEG_Y * 0.3).normalize(),
            // Toward the tail's tip, along the bone it grows off.
            Self::Tail => bed.distal(root.at),
            // To the ground, and a little back.
            Self::Feathering => (Vec3::NEG_Y * 0.85 - forward * 0.15).normalize(),
            // On up the ear.
            Self::Tufts => root.out,
        }
    }
}

/// How far a card's drawn spine may stray from its curve, as a share of the
/// back's height: about a pixel where a whole body fills a 512-pixel view.
/// Provenance: **derived** from the gallery's framing, as the hair's own
/// millimetre is from a head's.
const FLATNESS: f32 = 0.002;

/// How long a card is, in metres, where it droops one and a half times as
/// far as its zone's cut says: the length long hair starts to hang at.
/// Provenance: **tuned by render** (#369).
const DROOP_LENGTH: f32 = 0.15;

/// How far along a card it opens from its pinch to its whole width.
/// Provenance: **tuned by render** (#386).
const OPENS: f32 = 0.15;

/// How much of a card's length may go to coming out of the skin, at most: a
/// card at a zone's thin edge is hardly longer than a body's sink.
/// Provenance: **tuned by render** (#386).
const COMES_OUT: f32 = 0.4;

/// How far off the neck's run toward its side a mane's card combs, in radians:
/// back along the crest, so that each root lies under the card ahead of it,
/// and over the side, so that it does not hang down the neck's back and pile
/// at the withers.
/// Provenance: **tuned by render** (#386).
const SWEEP: f32 = 0.9;

/// How far round the neck a mane's card follows it before it hangs free, in
/// radians: short of a right angle, where the card would face away from the
/// way its root does and the loft would turn it over.
/// Provenance: **tuned by render** (#386).
const WRAP: f32 = 1.2;

/// How far off the skin a mane's card has lifted by its tip, as a share of its
/// length: flat on the neck it reads as paint.
/// Provenance: **tuned by render** (#386).
const LOFT: f32 = 0.08;

/// How high a mane's card arches off the crest before it falls to the neck's
/// side, as a share of its length, and how far along it has come back down: a
/// mane flat on the neck has no outline, and on a coat of its own colour shows
/// nothing at all.
/// Provenance: **tuned by render** (#386).
const ARCH: f32 = 0.09;
/// See [`ARCH`].
const ARCH_ENDS: f32 = 0.5;

/// How far off the skin a mane's card lies once it is out of it, as a share of
/// the back's height: more than a chord of the skin's own faces can bulge by.
/// Provenance: **tuned by render** (#386).
const LOFT_AT_ROOT: f32 = 0.004;

/// How high a full-length mane's crest stands at full stiffness, as a share of
/// the back's height; it goes with the square root of the record's length, so
/// that length stays an axis of its own without a long mane making a sail.
/// Provenance: **tuned by render** (#389), about a hogged horse's 10 to 12 cm.
const STAND_REACH: f32 = 0.08;

/// How much of a card a rise may spend, at most: the rest is its foot, and a
/// tip.
/// Provenance: **tuned by render** (#389).
const STAND_SHARE: f32 = 0.7;

/// How far round a standing card's foot comes before it rises, in radians: over
/// the crest and little further, or an upright card's foot pokes through the
/// falling cards lying there.
/// Provenance: **tuned by render** (#389).
const FOOT_TURN: f32 = 0.12;

/// How much of a card's turn round the neck is left at the top of its rise: a
/// little, so that the two sides' cards lean together into a ridge and do not
/// lie in one plane.
/// Provenance: **tuned by render** (#389).
const RIDGE: f32 = 0.2;

/// How far back along the crest a card leans as it rises, as a share of the rise.
/// Provenance: **tuned by render** (#389).
const RAKE: f32 = 0.35;

/// How far along a card it comes down from the top of its rise, in rises.
/// Provenance: **tuned by render** (#389).
const FALL: f32 = 1.6;

/// How far under the skin a stiff card's root goes past a limp one's, at full
/// stiffness, as a share of it: a card that leaves the neck steeply needs its
/// straight root edge further below the skin.
/// Provenance: **tuned by render** (#389).
const BURY: f32 = 0.5;

/// How much narrower a card is at full stiffness: a crest's cards are short,
/// and a card as wide as it is long reads as a scale.
/// Provenance: **tuned by render** (#389).
const NARROWS: f32 = 0.5;

/// How much lower a falling card's arch stands than the crest's upright cards.
/// Provenance: **tuned by render** (#389).
const ARCHED: f32 = 0.4;

/// How many times longer than its half width an upright card is, at least.
/// Provenance: **tuned by render** (#389), from #369's length over width of 2.5.
const UPRIGHT_SLENDER: f32 = 4.0;

/// How long the falling cards are at full stiffness, as a share of the back's
/// height: a low skirt either side of the crest. Without it a crest's cards stand
/// out of bare neck, one by one, like the teeth of a comb. The seam sweep cannot
/// see that, and reads the skirt's own seams against it (0.051 without, 0.056
/// with): this is a judgement of the render.
/// Provenance: **tuned by render** (#389).
const SKIRT: f32 = 0.06;

/// What share of its width a skirt's card has where it enters the skin, and how
/// far along it opens to the whole of it: it has too little length to cover its
/// own seam with.
/// Provenance: **tuned by render** (#389), against the seam sweep.
const SKIRT_PINCH: f32 = 0.12;
/// See [`SKIRT_PINCH`].
const SKIRT_OPENS: f32 = 0.5;

/// The same of an upright card, which leaves the skin well along it, on the
/// crest, where a viewer sees the seam from above whatever lies beside it: so it
/// has hardly any width there, and a card reads as a spear of hair.
/// Provenance: **tuned by render** (#389), against the seam sweep.
const UPRIGHT_PINCH: f32 = 0.12;
/// See [`UPRIGHT_PINCH`].
const UPRIGHT_OPENS: f32 = 0.8;

/// How a stiff mane's cards stand, on one record: see [`Lock::standing`].
#[derive(Clone, Copy, Debug, Default)]
struct Stand {
    /// How stiff, `0..=1`.
    stiffness: f32,
    /// How far a full-weight card rises, in metres, before its share of a
    /// card's length is taken.
    rise: f32,
    /// How long a full-weight card is at full stiffness, in metres.
    least: f32,
}

/// One zone's cards, cut for one record's length on one body.
pub(super) struct Lock<'a> {
    zone: FurZone,
    bed: &'a Bed<'a>,
    /// The crest a mane's cards lie along.
    crest: Option<&'a Crest>,
    /// How long a card is at full mask weight, in metres.
    length: f32,
    /// Half its width at the root, in metres.
    width: f32,
    /// What that would be on a limp card.
    limp_width: f32,
    taper: f32,
    lie: f32,
    droop: f32,
    pinch: f32,
    /// How far along it opens to its whole width.
    opens: f32,
    /// How far a card's drawn spine may stray from its curve, in metres.
    flatness: f32,
    /// How a mane's cards stand.
    stand: Stand,
}

impl<'a> Lock<'a> {
    /// The finest a card is drawn, however small the beast: a head's hair's
    /// own tolerance.
    const FINEST: f32 = 0.001;

    /// `zone`'s cards on `bed`, at the record's `length`, a mane's as stiff as
    /// `stiffness` says.
    pub(super) fn new(
        zone: FurZone,
        bed: &'a Bed<'a>,
        crest: Option<&'a Crest>,
        length: f32,
        stiffness: f32,
    ) -> Self {
        let stiffness = if zone == FurZone::Mane {
            stiffness.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let reach = bed.anatomy.height * length.max(0.0).sqrt();
        let stand = Stand {
            stiffness,
            // Gone at full stiffness, where all a falling card does is cover
            // the crest's seams.
            rise: STAND_REACH * reach * stiffness * ARCHED * (1.0 - stiffness.powi(3)),
            least: SKIRT * bed.anatomy.height,
        };
        Self::cut(zone, bed, crest, length, stand)
    }

    /// The cards of a mane's crest that do not fall over (see [`crest_split`]):
    /// as stiff as a card gets, and as tall as the record's `stiffness` says.
    pub(super) fn upright(
        bed: &'a Bed<'a>,
        crest: Option<&'a Crest>,
        length: f32,
        stiffness: f32,
    ) -> Self {
        // The tallest a mane's crest is at this stiffness.
        let rise = STAND_REACH * bed.anatomy.height * length.max(0.0).sqrt() * stiffness.powf(0.4);
        let stand = Stand {
            stiffness: 1.0,
            rise,
            least: rise / STAND_SHARE,
        };
        let mut lock = Self::cut(FurZone::Mane, bed, crest, length, stand);
        // Long over wide, however short: a card as wide as it is long reads as
        // a scale.
        lock.width = lock.width.min(stand.rise / STAND_SHARE / UPRIGHT_SLENDER);
        lock.pinch = UPRIGHT_PINCH;
        lock.opens = UPRIGHT_OPENS;
        lock
    }

    fn cut(
        zone: FurZone,
        bed: &'a Bed<'a>,
        crest: Option<&'a Crest>,
        length: f32,
        stand: Stand,
    ) -> Self {
        let cut = zone.cut();
        let height = bed.anatomy.height;
        // Narrower on shorter fur, so a short ruff is fine rather than a
        // fringe of paddles.
        let limp_width = cut.width * height * length.min(1.0).sqrt();
        let skirted = stand.stiffness.powi(3);
        Self {
            zone,
            bed,
            crest,
            length: cut.reach * height * length,
            width: limp_width * (1.0 - NARROWS * stand.stiffness),
            limp_width,
            stand,
            taper: cut.taper,
            // A pixel's worth at the framing a body is judged at, never finer
            // than a head's hair is drawn.
            flatness: (FLATNESS * height).max(Lock::FINEST),
            lie: cut.lie,
            // A falling card at full stiffness is a short skirt, which has no
            // length left to cover its own seam with: it pinches as a crest's.
            pinch: cut.pinch + (SKIRT_PINCH - cut.pinch) * skirted,
            opens: OPENS + (SKIRT_OPENS - OPENS) * skirted,
            // Long hair hangs and short fur stands, by how long it is in
            // metres: a horse's tail falls where a cat's brush fluffs.
            droop: cut.droop * (0.5 + cut.reach * height * length / DROOP_LENGTH).min(3.0),
        }
    }
}

/// How `count` cards of a mane are shared out at `stiffness`, as the ones that
/// fall over and the ones that stand as its crest: all fall on a limp mane, and
/// on a stiff one eight in ten stand and seven in ten lie under them as a skirt. The
/// upright cards' tips are the crest's frayed outline, where a falling card's
/// arch has only an edge.
/// Provenance: **tuned by render** (#389).
pub(super) fn crest_split(count: usize, stiffness: f32) -> (usize, usize) {
    let stiffness = stiffness.clamp(0.0, 1.0);
    let of = |share: f32| (count as f32 * share).round() as usize;
    (
        of(1.0 - 0.3 * stiffness.powi(3)),
        of(0.8 * stiffness.sqrt()),
    )
}

impl Lock<'_> {
    /// How long a card is cut, before a mane's stiffness shortens it. Shorter
    /// where the mask is weaker, so a zone thins out at its edge.
    fn uncut(&self, root: &Root) -> f32 {
        self.length * root.weight.clamp(0.0, 1.0).sqrt()
    }

    /// How much of a mane's card's way out of the skin it has still to go, a
    /// share of the way along it: `1` at its root, and none from
    /// [`Self::comes_out`] on, which it reaches at a shallow angle.
    fn buried(&self, root: &Root, along: f32) -> f32 {
        let run = self.comes_out(root).max(f32::EPSILON);
        let left = 1.0 - (self.length(root) * along / run).min(1.0);
        left * left
    }

    /// How far under the skin a card's root sits: below the pinched root's own
    /// half width, or a straight line across a curved skin stands out of it
    /// at both ends; and over no more of a short card than [`COMES_OUT`].
    fn sunk(&self, root: &Root) -> f32 {
        if self.pinch >= 1.0 {
            return 0.0;
        }
        // Measured off the width a limp card has: a stiff card is narrower for
        // its short length's sake, and no less steep where it leaves the skin.
        let base = self.limp_width * root.weight.clamp(0.0, 1.0).sqrt();
        (base * self.zone.cut().pinch * (1.0 + BURY * self.stand.stiffness.powi(3)))
            .min(self.length(root) * COMES_OUT * 0.25)
    }

    /// How far along a card it takes to come out of the skin, in metres: four
    /// times as far as it was sunk, at about twenty-five degrees.
    fn comes_out(&self, root: &Root) -> f32 {
        self.sunk(root) * 4.0
    }

    /// How far this card rises off the crest, in metres, and how far along it,
    /// in metres, it starts to: its foot.
    ///
    /// A card is cut for its record's length and the rise is taken out of that
    /// (see [`Shape::length`]), so a stiff mane is short for being stiff and a
    /// long mane still longer than a short one at any stiffness.
    fn standing(&self, crest: &Crest, root: &Root) -> (f32, f32) {
        if self.stand.stiffness <= 0.0 {
            return (0.0, 0.0);
        }
        let length = self.length(root);
        let radius = crest.near(root.at).skin_at(WRAP * 0.5);
        let foot = (self.comes_out(root) * 0.5 + FOOT_TURN * radius) / SWEEP.sin();
        let foot = foot.min(length * (1.0 - STAND_SHARE));
        // By its weight, where its length goes by the root of that: a thin
        // card at a mane's end hardly rises, or its few stations turn it over.
        let rise = self.stand.rise * root.weight.clamp(0.0, 1.0);
        (rise.min(self.uncut(root) * STAND_SHARE), foot)
    }

    /// Where a mane's card has got to, a share of the way along it: back
    /// down the crest and round the neck's side at once, as far off the skin
    /// as [`Crest`] measured it plus its loft, until it has come round far
    /// enough to hang free.
    ///
    /// **A stiff card rises on the way** (#389). Over its rise the card stops
    /// going round the neck, comes back to the crest's line and lifts off it;
    /// past its top it comes back down as it goes on round. At no stiffness
    /// there is no rise and this is the draped mane of #386, byte for byte.
    fn draped(&self, crest: &Crest, root: &Root, along: f32) -> Draped {
        let length = self.length(root);
        let buried = self.buried(root, along);
        let side = Vec3::X * if root.out.x < 0.0 { -1.0 } else { 1.0 };
        let gone = length * along;
        let (rise, foot) = self.standing(crest, root);
        let (risen, stood) = if rise > 0.0 {
            let risen = step(0.0, 1.0, (gone - foot) / rise);
            let fallen = step(0.0, 1.0, (gone - foot - rise) / (rise * FALL));
            (risen, risen * (1.0 - fallen))
        } else {
            (0.0, 0.0)
        };
        // What the rise spends goes neither back along the crest nor round the
        // neck, past its own rake.
        let spent = gone - rise * risen;
        let (here, past) = crest.walked(root.at, spent * SWEEP.cos() + rise * risen * RAKE);
        // From under the far side of the crest, so that the two sides' cards
        // cross on it and each covers where the other comes out.
        let travel = spent * SWEEP.sin() - self.comes_out(root) * 0.5;
        let radius = here.skin_at(WRAP * 0.5);
        let turned = (travel / radius).min(WRAP) * (1.0 - (1.0 - RIDGE) * stood);
        Draped {
            up: here.up,
            side,
            run: here.run,
            turned,
            stood,
            stiffness: self.stand.stiffness,
            // The bone under the crest it has come down to, and on past the
            // crest's end.
            bone: here.at - here.up * here.skin[0] + here.run * past,
            off: here.skin_at(turned.abs())
                + (LOFT_AT_ROOT * self.bed.anatomy.height
                    + length * (ARCH * (PI * (along / ARCH_ENDS).min(1.0)).sin() + LOFT * along)
                    + rise * stood)
                    * (1.0 - buried)
                - self.sunk(root) * buried,
            hangs: travel - (travel / radius).min(WRAP) * radius,
        }
    }
}

/// A mane's card a share of the way along it, from [`Lock::draped`].
struct Draped {
    up: Vec3,
    side: Vec3,
    run: Vec3,
    /// How far round the neck, in radians.
    turned: f32,
    /// How far up its rise it stands, `0..=1`: see [`Lock::standing`].
    stood: f32,
    /// How stiff the card is.
    stiffness: f32,
    /// The bone it lies round.
    bone: Vec3,
    /// How far it lies off that bone.
    off: f32,
    /// How far it hangs free past where it left the neck.
    hangs: f32,
}

impl Draped {
    /// Which way off the bone, this far round.
    fn round(&self) -> Vec3 {
        self.up * self.turned.cos() + self.side * self.turned.sin()
    }

    /// Where it is.
    fn at(&self) -> Vec3 {
        let leaving =
            (self.side * WRAP.cos() - self.up * WRAP.sin() + Vec3::NEG_Y).normalize_or(self.side);
        self.bone + self.round() * self.off + leaving * self.hangs
    }

    /// Across it, in the neck's skin.
    fn across(&self) -> Vec3 {
        let turning = self.side * self.turned.cos() - self.up * self.turned.sin();
        let lying = self
            .round()
            .cross(self.run * SWEEP.cos() + turning * SWEEP.sin());
        // Going up, a card's width lies along the crest, its face to the side.
        let along_crest = self.run * lying.dot(self.run).signum();
        lying.lerp(along_crest, self.stood * self.stiffness * self.stiffness)
    }
}

impl Shape for Lock<'_> {
    fn length(&self, root: &Root) -> f32 {
        let uncut = self.uncut(root);
        let stiff = self.stand.stiffness;
        if stiff <= 0.0 {
            return uncut;
        }
        // A stiff mane is a short one: what is past a card's least goes as it
        // stiffens, and is gone at full stiffness.
        let least = (self.stand.least * root.weight.clamp(0.0, 1.0).sqrt()).min(uncut);
        least + (uncut - least) * (1.0 - stiff.powi(3))
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        if let Some(crest) = self.crest {
            let limp = 1.0 - self.stand.stiffness;
            return self.draped(crest, root, along).at()
                + Vec3::NEG_Y * (limp * self.droop * self.length(root) * along.powi(3) * 0.25);
        }
        let length = self.length(root);
        let way = self.zone.way(self.bed, root);
        let along_skin = (way - root.out * way.dot(root.out)).normalize_or(root.out);
        let leaves = root
            .out
            .lerp(along_skin, self.lie.clamp(0.0, 1.0))
            .normalize_or(root.out);
        let fall = Vec3::NEG_Y * (self.droop * along * along);
        let heading = (leaves + fall).normalize_or(leaves);
        // Straight up out of the skin over its whole length, which spends no
        // station on a bend: this is the zone with the most cards.
        root.at + root.out * (LIFT - self.sunk(root) * (1.0 - along)) + heading * (length * along)
    }

    fn flatness(&self) -> f32 {
        self.flatness
    }

    fn keeps_its_way_round(&self) -> bool {
        self.stand.stiffness > 0.0
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let base = self.width * root.weight.clamp(0.0, 1.0).sqrt();
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        let (base, tip) = self.width(root);
        let along = along.clamp(0.0, 1.0);
        let opened = self.pinch + (1.0 - self.pinch) * (along / self.opens).min(1.0);
        (base + (tip - base) * along) * opened
    }

    fn across_at(&self, root: &Root, along: f32) -> Vec3 {
        match self.crest {
            Some(crest) => self.draped(crest, root, along).across(),
            None => self.across(root),
        }
    }

    fn across(&self, root: &Root) -> Vec3 {
        // Across the way it combs, in the skin's own plane: a card lies on
        // the coat it grows from, as a feather does.
        let way = self.zone.way(self.bed, root);
        root.out
            .cross(way)
            .normalize_or(root.out.cross(Vec3::Y).normalize_or(Vec3::X))
    }
}
