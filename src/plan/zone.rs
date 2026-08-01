//! Semantic regions of a body.
//!
//! Every skeleton node carries a [`Zone`] saying what part of the body it is.
//! That one tag does a surprising amount of work downstream:
//!
//! * **Garments** declare the zones they cover, and the body suppresses those
//!   zones underneath — poke-through is avoided by not emitting the geometry
//!   rather than by hiding it.
//! * **Landmarks** are found by zone rather than by node index, so hats and
//!   belts fit any body without the fitting code knowing which plan built it.
//! * **Animation** can pose semantic queries — every ground contact, every
//!   grasper — instead of naming bones, which is what lets one motion serve
//!   bodies with different limb counts.
//!
//! Limbs are named fore and hind rather than arm and leg, because a biped's arms
//! and a quadruped's forelegs occupy the same place in the body plan. One
//! vocabulary covers both.

use serde::{Deserialize, Serialize};

/// Which of a body's four limb positions a zone belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Limb {
    /// A biped's left arm, a quadruped's left foreleg.
    ForeLeft,
    /// A biped's right arm, a quadruped's right foreleg.
    ForeRight,
    /// A biped's left leg, a quadruped's left hind leg.
    HindLeft,
    /// A biped's right leg, a quadruped's right hind leg.
    HindRight,
}

impl Limb {
    /// All four limb positions, left before right, fore before hind.
    pub const ALL: [Limb; 4] = [
        Limb::ForeLeft,
        Limb::ForeRight,
        Limb::HindLeft,
        Limb::HindRight,
    ];

    /// Whether this limb is on the body's left.
    #[must_use]
    pub fn is_left(self) -> bool {
        matches!(self, Limb::ForeLeft | Limb::HindLeft)
    }

    /// Whether this limb is a front limb.
    #[must_use]
    pub fn is_fore(self) -> bool {
        matches!(self, Limb::ForeLeft | Limb::ForeRight)
    }

    /// This limb's mirror across the body's plane of symmetry.
    #[must_use]
    pub fn mirrored(self) -> Limb {
        match self {
            Limb::ForeLeft => Limb::ForeRight,
            Limb::ForeRight => Limb::ForeLeft,
            Limb::HindLeft => Limb::HindRight,
            Limb::HindRight => Limb::HindLeft,
        }
    }
}

/// A named region of the body surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Zone {
    /// The skull, including the face and scalp.
    Head,
    /// Between skull and chest.
    Neck,
    /// The upper torso, where the fore limbs attach.
    Chest,
    /// The middle torso. Also the neutral default for untagged nodes.
    Abdomen,
    /// The lower torso, where the hind limbs attach.
    Pelvis,
    /// A tail, if the body has one.
    Tail,
    /// The upper segment of a limb: upper arm, or thigh.
    UpperLimb(Limb),
    /// The lower segment of a limb: forearm, or shin.
    LowerLimb(Limb),
    /// The end of a limb: hand, or foot.
    Extremity(Limb),
}

impl Default for Zone {
    /// The neutral body mass, used by nodes a plan has not tagged.
    fn default() -> Self {
        Zone::Abdomen
    }
}

/// How many distinct zones exist.
pub const ZONE_COUNT: usize = 18;

impl Zone {
    /// Every zone, in head-to-tail order.
    #[must_use]
    pub fn all() -> Vec<Zone> {
        let mut zones = vec![
            Zone::Head,
            Zone::Neck,
            Zone::Chest,
            Zone::Abdomen,
            Zone::Pelvis,
            Zone::Tail,
        ];
        for limb in Limb::ALL {
            zones.push(Zone::UpperLimb(limb));
        }
        for limb in Limb::ALL {
            zones.push(Zone::LowerLimb(limb));
        }
        for limb in Limb::ALL {
            zones.push(Zone::Extremity(limb));
        }
        zones
    }

    /// This zone's position in a [`ZoneSet`] bitmask, below [`ZONE_COUNT`].
    #[must_use]
    pub fn index(self) -> u8 {
        match self {
            Zone::Head => 0,
            Zone::Neck => 1,
            Zone::Chest => 2,
            Zone::Abdomen => 3,
            Zone::Pelvis => 4,
            Zone::Tail => 5,
            Zone::UpperLimb(limb) => 6 + limb as u8,
            Zone::LowerLimb(limb) => 10 + limb as u8,
            Zone::Extremity(limb) => 14 + limb as u8,
        }
    }

    /// The bit this zone occupies in a [`ZoneSet`].
    #[must_use]
    pub fn bit(self) -> u32 {
        1 << self.index()
    }

    /// The limb this zone belongs to, if it is part of one.
    #[must_use]
    pub fn limb(self) -> Option<Limb> {
        match self {
            Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) => Some(limb),
            _ => None,
        }
    }

    /// Whether this zone is part of the torso, neck, or head.
    #[must_use]
    pub fn is_core(self) -> bool {
        self.limb().is_none()
    }
}

/// A set of zones, as a bitmask.
///
/// Garments declare their coverage this way, and the body consults it to decide
/// which surface to emit — so a shirt and the skin beneath it can never occupy
/// the same space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ZoneSet {
    /// One bit per zone, as given by [`Zone::bit`].
    pub bits: u32,
}

impl ZoneSet {
    /// The empty set.
    pub const NONE: ZoneSet = ZoneSet { bits: 0 };

    /// Every zone.
    #[must_use]
    pub fn all() -> ZoneSet {
        Zone::all().into_iter().fold(ZoneSet::NONE, ZoneSet::with)
    }

    /// Whether `zone` is in the set.
    #[must_use]
    pub fn contains(self, zone: Zone) -> bool {
        self.bits & zone.bit() != 0
    }

    /// The set with `zone` added.
    #[must_use]
    pub fn with(mut self, zone: Zone) -> Self {
        self.bits |= zone.bit();
        self
    }

    /// The set with `zone` removed.
    #[must_use]
    pub fn without(mut self, zone: Zone) -> Self {
        self.bits &= !zone.bit();
        self
    }

    /// Every zone in either set.
    #[must_use]
    pub fn union(self, other: ZoneSet) -> Self {
        ZoneSet {
            bits: self.bits | other.bits,
        }
    }

    /// Every zone in both sets.
    #[must_use]
    pub fn intersection(self, other: ZoneSet) -> Self {
        ZoneSet {
            bits: self.bits & other.bits,
        }
    }

    /// Whether the set holds no zones.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.bits == 0
    }

    /// The zones in the set, in head-to-tail order.
    #[must_use]
    pub fn zones(self) -> Vec<Zone> {
        Zone::all()
            .into_iter()
            .filter(|&zone| self.contains(zone))
            .collect()
    }
}

impl FromIterator<Zone> for ZoneSet {
    fn from_iter<I: IntoIterator<Item = Zone>>(iter: I) -> Self {
        iter.into_iter().fold(ZoneSet::NONE, ZoneSet::with)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_has_a_distinct_bit_within_a_u32() {
        let zones = Zone::all();
        assert_eq!(zones.len(), ZONE_COUNT);

        let mut seen = 0u32;
        for zone in zones {
            assert!(
                (zone.index() as usize) < ZONE_COUNT,
                "{zone:?} indexes past the zone count"
            );
            assert_eq!(seen & zone.bit(), 0, "{zone:?} collides with another zone");
            seen |= zone.bit();
        }
    }

    #[test]
    fn a_garment_covers_the_zones_it_declares() {
        let shirt: ZoneSet = [
            Zone::Chest,
            Zone::Abdomen,
            Zone::UpperLimb(Limb::ForeLeft),
            Zone::UpperLimb(Limb::ForeRight),
        ]
        .into_iter()
        .collect();

        assert!(shirt.contains(Zone::Chest));
        assert!(!shirt.contains(Zone::Pelvis));
        assert_eq!(shirt.zones().len(), 4);

        let trousers: ZoneSet = [Zone::Pelvis, Zone::UpperLimb(Limb::HindLeft)]
            .into_iter()
            .collect();
        assert!(
            shirt.intersection(trousers).is_empty(),
            "layers do not fight"
        );
        assert_eq!(shirt.union(trousers).zones().len(), 6);
    }

    #[test]
    fn limbs_mirror_and_classify() {
        assert_eq!(Limb::ForeLeft.mirrored(), Limb::ForeRight);
        assert_eq!(Limb::HindRight.mirrored().mirrored(), Limb::HindRight);
        assert!(Limb::ForeLeft.is_fore() && Limb::ForeLeft.is_left());
        assert!(!Limb::HindRight.is_fore() && !Limb::HindRight.is_left());
    }

    #[test]
    fn zones_know_their_limb_and_whether_they_are_core() {
        assert_eq!(Zone::UpperLimb(Limb::HindLeft).limb(), Some(Limb::HindLeft));
        assert_eq!(Zone::Chest.limb(), None);
        assert!(Zone::Head.is_core());
        assert!(!Zone::Extremity(Limb::ForeRight).is_core());
    }

    #[test]
    fn the_default_zone_is_the_neutral_body_mass() {
        assert_eq!(Zone::default(), Zone::Abdomen);
    }
}
