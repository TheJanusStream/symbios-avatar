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
//! [`Lock::draped`] and [`Lock::section`]).

use std::f32::consts::{FRAC_PI_2, PI};

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

/// By what stiffness a mane's card has gone from draped over the neck to the
/// stiff card [`Lock::section`] draws: the one hands over to the other low on the
/// slider, where they lie close together.
/// Provenance: **tuned by render** (#389).
const STIFF_FROM: f32 = 0.2;

/// How much higher a stiffening card lies than a limp one, past its foot, as a
/// share of its length, by its stiffness: what lifts a short card's outline, which
/// has little left to stand on past its foot, before it can stand.
/// Provenance: **tuned by render** (#389), against the outline's sweep.
const STIFF_ARCH: f32 = 0.65;

/// How much longer than its cut a stiff card's curve is drawn: the limp curve,
/// laid over the skin that stands off its bone, comes out 1.14 to 1.44 times its
/// cut, and a mane is to keep its length as it stands. In the middle of that.
/// Provenance: **measured** (`stiffness_table`, #389).
const DRAWN: f32 = 1.25;

/// How far past its top a card comes round once it bends over, in radians from
/// straight up: short of straight down, so that what is left of a long card
/// falls clear of the one beside it.
/// Provenance: **tuned by render** (#389).
const OVER: f32 = 2.95;

/// How much wider a card's bend gets with stiffness, as a share of its length at
/// full stiffness: a limp card bends round the neck, and a stiff one over a bow.
/// Provenance: **tuned by render** (#389).
const BOW: f32 = 0.8;

/// How much wider a card's bend is from the start of the slider, as a share of its
/// length at full stiffness: past the neck's side a long card would fall the moment
/// it left the crest, and never rise above a limp one's arch.
/// Provenance: **tuned by render** (#389), against the outline's sweep.
const BOW_EARLY: f32 = 0.2;

/// How long a card's foot is, in its ways out of the skin, on a limp card.
/// Provenance: **tuned by render** (#389), against the seam sweep.
const FOOT: f32 = 3.5;

/// How much of a card a foot may spend, at most: on a short card its way out of
/// the skin is a good part of it.
/// Provenance: **tuned by render** (#389).
const FOOT_SHARE: f32 = 0.45;

/// How much of a card's way out of the skin the bend up off it takes.
/// Provenance: **tuned by render** (#389).
const LIFTS_OFF: f32 = 0.25;

/// How far under the skin a stiff card's root goes past a limp one's, at full
/// stiffness, as a share of it: a card that leaves the neck steeply needs its
/// straight root edge further below the skin.
/// Provenance: **tuned by render** (#389).
const BURY: f32 = 0.5;

/// How much narrower a card is at full stiffness: it stands on its edge along the
/// crest beside every other card, and at a limp card's width that is a board.
/// Provenance: **tuned by render** (#389).
const NARROWS: f32 = 0.5;

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
    /// How stiff a mane's cards are, `0..=1`: see [`Lock::section`].
    stiffness: f32,
    taper: f32,
    lie: f32,
    droop: f32,
    pinch: f32,
    /// How far a card's drawn spine may stray from its curve, in metres.
    flatness: f32,
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
        let cut = zone.cut();
        let height = bed.anatomy.height;
        // Narrower on shorter fur, so a short ruff is fine rather than a
        // fringe of paddles.
        let limp_width = cut.width * height * length.min(1.0).sqrt();
        Self {
            zone,
            bed,
            crest,
            length: cut.reach * height * length,
            width: limp_width * (1.0 - NARROWS * stiffness),
            limp_width,
            stiffness,
            taper: cut.taper,
            // A pixel's worth at the framing a body is judged at, never finer
            // than a head's hair is drawn.
            // Coarser the stiffer: a stiff card's bend is long and shallow, and
            // drawn as finely as a limp card's wrap it is the dearest fur there is.
            flatness: (FLATNESS * height).max(Lock::FINEST) * (1.0 + stiffness),
            lie: cut.lie,
            pinch: cut.pinch,
            // Long hair hangs and short fur stands, by how long it is in
            // metres: a horse's tail falls where a cat's brush fluffs.
            droop: cut.droop * (0.5 + cut.reach * height * length / DROOP_LENGTH).min(3.0),
        }
    }
}

impl Lock<'_> {
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
        // Measured off the width a limp card has: a stiff card is narrower, and
        // no less steep where it leaves the skin.
        let base = self.limp_width * root.weight.clamp(0.0, 1.0).sqrt();
        (base * self.zone.cut().pinch * (1.0 + BURY * self.stiffness.powi(3)))
            .min(self.length(root) * COMES_OUT * 0.25)
    }

    /// How far along a card it takes to come out of the skin, in metres: four
    /// times as far as it was sunk, at about twenty-five degrees.
    fn comes_out(&self, root: &Root) -> f32 {
        self.sunk(root) * 4.0
    }

    /// How far a card of `total` metres lies low before it lifts: over the crest
    /// and on, over where the other side's cards come out, until there is nothing
    /// left to stand on it.
    fn foot(&self, root: &Root, total: f32) -> f32 {
        (self.comes_out(root) * FOOT).min(total * FOOT_SHARE) * (1.0 - self.stiffness)
    }

    /// How far from straight up a stiff card's run leans.
    fn leans(&self) -> f32 {
        FRAC_PI_2 * (1.0 - self.stiffness.powf(0.4))
    }

    /// The radius of the bend a card takes up off the skin.
    fn lifts(&self, root: &Root) -> f32 {
        (self.comes_out(root) * LIFTS_OFF).max(f32::EPSILON)
    }

    /// How far along a card of `total` metres it has lifted off its foot and is
    /// on its run.
    fn lifted(&self, root: &Root, total: f32) -> f32 {
        self.foot(root, total) + self.lifts(root) * (FRAC_PI_2 - self.leans())
    }

    /// A stiff card a way `travel` along it, in the plane across the neck: how
    /// far it has gone to its side, and how far up off the crest.
    ///
    /// **One curve at every stiffness, its length its own** (#389). It comes off
    /// the skin on a bend, runs straight at the angle its stiffness holds it at,
    /// and past as much of its length as its stiffness stands it, bends over and
    /// falls: straight lines and arcs of circles, so a way along the curve is a
    /// way along the card, whatever shape the stiffness gives it. At full
    /// stiffness the straight run is all of it, and square to the crest; at none
    /// the run is gone, the bend is the neck's own, and [`Lock::draped`]'s limp
    /// curve has taken over.
    fn section(&self, root: &Root, travel: f32, total: f32, beside: f32) -> (f32, f32) {
        let stiff = self.stiffness;
        let leans = self.leans();
        let lifts = self.lifts(root);
        let foot = self.foot(root, total);
        if travel <= foot {
            return (travel, 0.0);
        }
        let lifted = self.lifted(root, total);
        if travel <= lifted {
            let turn = (travel - foot) / lifts;
            return (foot + lifts * turn.sin(), lifts * (1.0 - turn.cos()));
        }
        let turn = FRAC_PI_2 - leans;
        let mut x = foot + lifts * turn.sin();
        let mut y = lifts * (1.0 - turn.cos());
        let stands = lifted + (total - lifted).max(0.0) * stiff.powf(1.35);
        let run = travel.min(stands) - lifted;
        x += run * leans.sin();
        y += run * leans.cos();
        let past = travel - stands;
        if past <= 0.0 {
            return (x, y);
        }
        // Round to the neck's side, and no wider, until it is stiff enough to
        // bow: a limp card sent wider than the neck stands out as a wing.
        let fits = (beside - x).max(0.0) / (leans.cos() - OVER.cos());
        let bow = fits.max(f32::EPSILON) + total * (BOW_EARLY * stiff + BOW * stiff.powi(4));
        let bend = (past / bow).min(OVER - leans).max(0.0);
        let heading = leans + bend;
        x += bow * (leans.cos() - heading.cos());
        y += bow * (heading.sin() - leans.sin());
        let falling = past - bend * bow;
        (x + falling * heading.sin(), y + falling * heading.cos())
    }

    /// Where a mane's card has got to, a share of the way along it: back
    /// down the crest and round the neck's side at once, as far off the skin
    /// as [`Crest`] measured it plus its loft, until it has come round far
    /// enough to hang free.
    ///
    /// **A stiff card stands on the way** (#389): it sweeps back along the crest
    /// less, and [`Self::section`] lifts it off the neck. At no stiffness this is
    /// the draped mane of #386, byte for byte.
    fn draped(&self, crest: &Crest, root: &Root, along: f32) -> Draped {
        let length = self.length(root);
        let buried = self.buried(root, along);
        let side = Vec3::X * if root.out.x < 0.0 { -1.0 } else { 1.0 };
        let stiff = self.stiffness;
        let stands = if stiff > 0.0 {
            step(0.0, STIFF_FROM, stiff)
        } else {
            0.0
        };
        let drawn = length * (1.0 + (DRAWN - 1.0) * stands);
        let gone = drawn * along;
        let sweep = SWEEP + (FRAC_PI_2 - SWEEP) * stiff * stiff * stiff;
        let (here, past) = crest.walked(root.at, gone * sweep.cos());
        // From under the far side of the crest, so that the two sides' cards
        // cross on it and each covers where the other comes out.
        let out = self.comes_out(root) * 0.5;
        let travel = gone * sweep.sin() - out;
        let radius = here.skin_at(WRAP * 0.5);
        let wrapped = (travel / radius).min(WRAP);
        // Off the skin at a share of the way along, on top of the skin itself: a limp
        // card's whole lift, and all a stiff card's foot has while it lies low.
        let lift = |along: f32| {
            (LOFT_AT_ROOT * self.bed.anatomy.height
                + length * (ARCH * (PI * (along / ARCH_ENDS).min(1.0)).sin() + LOFT * along))
                * (1.0 - self.buried(root, along))
        };
        let limp_off = here.skin_at(wrapped.abs()) + lift(along);
        let hangs = travel - wrapped * radius;
        let (turned, off) = if stands > 0.0 {
            let across = drawn * sweep.sin();
            let total = across - out;
            // At a share of the way along: how far round the neck it is, how far off
            // the bone its curve puts it, and how far the skin and its lift do.
            let placed = |along: f32| {
                let travel = across * along - out;
                let (x, y) = if travel > 0.0 {
                    self.section(root, travel, total, here.skin_at(FRAC_PI_2))
                } else {
                    (travel, 0.0)
                };
                // Higher, once it is clear of where it came out: until it stands, this
                // is all that lifts its outline.
                let arch =
                    length * STIFF_ARCH * stiff * (1.0 - stiff).powi(2) * step(0.15, 0.4, along);
                let high = here.skin[0] + y + arch;
                let round = x.atan2(high);
                (
                    round,
                    x.hypot(high),
                    here.skin_at(round.abs()) + lift(along),
                )
            };
            let (round, curve, clear) = placed(along);
            // Its foot lies on its lift, over the other side's seams. Past the foot
            // what that lift stood above the curve goes with it: a floor the curve
            // had to climb back up to would hold it still across the neck while it
            // went on along the crest, which swings a width squared to its way round
            // to face that way, and turns it over.
            let left = ((self.lifted(root, total) + out) / across.max(f32::EPSILON)).min(1.0);
            let stood = if along <= left {
                curve.max(clear)
            } else {
                let (_, curve_then, clear_then) = placed(left);
                curve + (clear_then - curve_then).max(0.0)
            };
            (
                wrapped + (round - wrapped) * stands,
                limp_off + (stood - limp_off) * stands,
            )
        } else {
            (wrapped, limp_off)
        };
        Draped {
            up: here.up,
            side,
            run: here.run,
            sweep,
            turned,
            stands,
            // The bone under the crest it has come down to, and on past the
            // crest's end.
            bone: here.at - here.up * here.skin[0] + here.run * past,
            off: off - self.sunk(root) * buried,
            hangs: hangs * (1.0 - stands),
        }
    }
}

/// A mane's card a share of the way along it, from [`Lock::draped`].
struct Draped {
    up: Vec3,
    side: Vec3,
    run: Vec3,
    /// How far off the neck's run toward its side it combs, in radians.
    sweep: f32,
    /// How far round the neck, in radians.
    turned: f32,
    /// How far from draped to stiff it is, `0..=1`: see [`Lock::section`].
    stands: f32,
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
            .cross(self.run * self.sweep.cos() + turning * self.sweep.sin());
        // A stiff card's width lies along the crest however it is bent, so that
        // it goes on lying the way it did: the loft squares this to the card.
        let along_crest = self.run * lying.dot(self.run).signum();
        lying.lerp(along_crest, self.stands)
    }
}

impl Shape for Lock<'_> {
    fn length(&self, root: &Root) -> f32 {
        // Shorter where the mask is weaker, so a zone thins out at its edge.
        self.length * root.weight.clamp(0.0, 1.0).sqrt()
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        if let Some(crest) = self.crest {
            let limp = 1.0 - self.stiffness;
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
        self.stiffness > 0.0
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let base = self.width * root.weight.clamp(0.0, 1.0).sqrt();
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        let (base, tip) = self.width(root);
        let along = along.clamp(0.0, 1.0);
        let opened = self.pinch + (1.0 - self.pinch) * (along / OPENS).min(1.0);
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
