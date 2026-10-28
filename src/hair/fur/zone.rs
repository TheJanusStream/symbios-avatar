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
//! does.

use glam::Vec3;

use super::Bed;
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
        let cut = |count, reach, width, taper, lie, droop| Cut {
            count,
            reach,
            width,
            taper,
            lie,
            droop,
        };
        match self {
            Self::Mane => cut(96, 0.30, 0.04, 0.5, 0.3, 0.2),
            Self::Forelock => cut(20, 0.2, 0.028, 0.35, 0.45, 0.8),
            Self::Ruff => cut(240, 0.13, 0.026, 0.45, 0.5, 0.3),
            Self::Tail => cut(110, 0.38, 0.03, 0.45, 0.75, 0.4),
            Self::Feathering => cut(56, 0.12, 0.03, 0.5, 0.75, 0.6),
            Self::Tufts => cut(5, 0.09, 0.012, 0.15, 0.1, 0.0),
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
            // Over one side of the neck or the other, and a little down the
            // neck toward the withers: down the neck's own run rather than to
            // the ground, which on a neck carried high is straight down its
            // back and piles every card at the withers. The side is the
            // root's own, by a hash of where it sits, so a mane on the
            // midline falls both ways and reads from either side.
            Self::Mane => {
                let side = if super::super::mask::lane_of(root.at).is_multiple_of(2) {
                    -1.0
                } else {
                    1.0
                };
                (Vec3::X * side * 0.75 - bed.distal(root.at) * 0.35).normalize_or(Vec3::X)
            }
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

/// One zone's cards, cut for one record's length on one body.
pub(super) struct Lock<'a> {
    zone: FurZone,
    bed: &'a Bed<'a>,
    /// How long a card is at full mask weight, in metres.
    length: f32,
    /// Half its width at the root, in metres.
    width: f32,
    taper: f32,
    lie: f32,
    droop: f32,
    /// How far a card's drawn spine may stray from its curve, in metres.
    flatness: f32,
}

impl<'a> Lock<'a> {
    /// The finest a card is drawn, however small the beast: a head's hair's
    /// own tolerance.
    const FINEST: f32 = 0.001;

    /// `zone`'s cards on `bed`, at the record's `length`.
    pub(super) fn new(zone: FurZone, bed: &'a Bed<'a>, length: f32) -> Self {
        let cut = zone.cut();
        let height = bed.anatomy.height;
        Self {
            zone,
            bed,
            length: cut.reach * height * length,
            // Narrower on shorter fur, so a short ruff is fine rather than a
            // fringe of paddles.
            width: cut.width * height * length.min(1.0).sqrt(),
            taper: cut.taper,
            // A pixel's worth at the framing a body is judged at, never finer
            // than a head's hair is drawn.
            flatness: (FLATNESS * height).max(Lock::FINEST),
            lie: cut.lie,
            // Long hair hangs and short fur stands, by how long it is in
            // metres: a horse's tail falls where a cat's brush fluffs.
            droop: cut.droop * (0.5 + cut.reach * height * length / DROOP_LENGTH).min(3.0),
        }
    }
}

impl Shape for Lock<'_> {
    fn length(&self, root: &Root) -> f32 {
        // Shorter where the mask is weaker, so a zone thins out at its edge.
        self.length * root.weight.clamp(0.0, 1.0).sqrt()
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let length = self.length(root);
        let way = self.zone.way(self.bed, root);
        let along_skin = (way - root.out * way.dot(root.out)).normalize_or(root.out);
        let leaves = root
            .out
            .lerp(along_skin, self.lie.clamp(0.0, 1.0))
            .normalize_or(root.out);
        let fall = Vec3::NEG_Y * (self.droop * along * along);
        let heading = (leaves + fall).normalize_or(leaves);
        root.at + root.out * LIFT + heading * (length * along)
    }

    fn flatness(&self) -> f32 {
        self.flatness
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let base = self.width * root.weight.clamp(0.0, 1.0).sqrt();
        (base, base * self.taper.clamp(0.0, 1.0))
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
