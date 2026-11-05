//! What a body's parts are, as its plan declares them (#365).
//!
//! The build's rules about a part are stated against this - a human skull is
//! shaped and carved into a face, a limb that bears no weight ends in a hand,
//! a biped's foot is carved from the cage, a human trunk is given a chest - and
//! never against how many feet a body stands on. Counting feet told a person
//! from a hound while there were two plans, and it stops telling anything the
//! day a plan stands on two legs with a beast's head, or on four with hands.
//!
//! **Which limbs bear the weight is declared here too** (#419), as [`Legs`]:
//! the gait, every rest and the choice of a hand or a foot at the end of a
//! limb ask it ([`crate::Rig::ground_contacts`], [`crate::Rig::four_legged`]).
//! It used to be read off the rest pose - any extremity within a quarter of
//! the body's height of the lowest - which a wing's drooping tip would pass.
//!
//! A plan declares its anatomy ([`super::BodyPlan::anatomy`]) and stamps it on
//! the skeleton it builds, which hands it to the rig rooted from it - so a pass
//! holding only a rig asks the same question a record does.

use serde::{Deserialize, Serialize};

use super::Limb;
use crate::tolerant::tolerant_unit;

/// What kind of head a body carries.
///
/// **A kind this build does not know reads as [`Self::Beast`]** (#419), from a
/// body a newer build sent: a creature's head, on which nothing human is built
/// or driven - no face, mouth, brows or scalp hair, each of which asks for
/// joints a head that is not a person's does not have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadKind {
    /// A human skull: shaped and refined as one, on a human neck column, and
    /// carved into a face with its mouth, its features and a scalp's hair.
    #[default]
    Human,
    /// A beast's head. Nothing human is built on it; its own shape is the
    /// creature work's (#364).
    Beast,
}

tolerant_unit!(HeadKind, "a kind of head", fallback Beast, {
    Human => "human",
    Beast => "beast",
});

/// What kind of trunk a body carries.
///
/// **A kind this build does not know reads as [`Self::Beast`]** (#419): a
/// trunk that is neither carved as a chest nor dressed, nor fitted with what
/// rides a person's chest - the garment catalogue is cut for a human trunk
/// and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TrunkKind {
    /// A human trunk: given a chest and its taper, and dressed - the garment
    /// catalogue is cut for a human trunk and the limbs that hang from it.
    #[default]
    Human,
    /// A beast's barrel. Neither carved as a chest nor dressed.
    Beast,
}

tolerant_unit!(TrunkKind, "a kind of trunk", fallback Beast, {
    Human => "human",
    Beast => "beast",
});

/// What kind of foot ends a limb that bears weight.
///
/// **A kind this build does not know reads as [`Self::Beast`]** (#419): a
/// paw, which stands on its toes. A person's foot is planted and rolled
/// through a heel, a ball and a toe that a foot of another kind does not
/// have, and a hoof gallops transverse; a paw asks the least of the foot it
/// is put on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FootKind {
    /// A biped's foot, carved from the cage's own ankle, ball and toe nodes
    /// (#111).
    #[default]
    Human,
    /// A beast's paw, its claws showing or sheathed (#367).
    Beast,
    /// A beast's hoof (#367): a horse's, which gallops transverse where a
    /// paw's body gallops rotary (#370).
    Hooves,
}

tolerant_unit!(FootKind, "a kind of foot", fallback Beast, {
    Human => "human",
    Beast => "beast",
    Hooves => "hooves",
});

/// The limbs a body stands on, as its plan declares them (#419).
///
/// **Declared, never counted.** A biped stands on its two hind limbs and a
/// quadruped on all four, and a plan says so; reading it off the rest pose
/// instead - an extremity within a quarter of the body's height of the lowest -
/// counts any limb that happens to hang low, a wing's drooping tip among them.
///
/// Written as a number, one bit per limb in [`Limb::ALL`]'s order. A bit this
/// build has no limb for is kept and never read.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Legs {
    bits: u8,
}

impl Legs {
    /// No limb: a body that stands on none.
    pub const NONE: Self = Self { bits: 0 };

    /// The two hind limbs: a biped's legs.
    pub const HIND: Self = Self { bits: 0b1100 };

    /// All four limbs: a quadruped's.
    pub const ALL: Self = Self { bits: 0b1111 };

    /// The bit `limb` takes.
    const fn bit(limb: Limb) -> u8 {
        1 << limb as u8
    }

    /// These legs, and `limb` as one more.
    #[must_use]
    pub const fn with(self, limb: Limb) -> Self {
        Self {
            bits: self.bits | Self::bit(limb),
        }
    }

    /// Whether `limb` is one of them.
    #[must_use]
    pub const fn contains(self, limb: Limb) -> bool {
        self.bits & Self::bit(limb) != 0
    }

    /// The legs, in [`Limb::ALL`]'s order.
    pub fn iter(self) -> impl Iterator<Item = Limb> {
        Limb::ALL
            .into_iter()
            .filter(move |&limb| self.contains(limb))
    }

    /// How many there are.
    #[must_use]
    pub fn len(self) -> usize {
        self.iter().count()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

impl FromIterator<Limb> for Legs {
    fn from_iter<I: IntoIterator<Item = Limb>>(limbs: I) -> Self {
        limbs.into_iter().fold(Self::NONE, Self::with)
    }
}

/// As the list of limbs it holds, which is what a reader wants of it.
impl std::fmt::Debug for Legs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

/// What a body's parts are.
///
/// Read through the accessors rather than the fields wherever the question is
/// a yes-or-no one, so a rule says what it asks - [`Self::human_head`] - and a
/// third kind of head does not silently answer it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Anatomy {
    /// The head.
    pub head: HeadKind,
    /// The trunk.
    pub trunk: TrunkKind,
    /// Whether a limb that bears no weight ends in a hand.
    pub hands: bool,
    /// The foot on a limb that bears weight.
    pub feet: FootKind,
    /// The limbs that bear the body's weight (#419): see [`Legs`].
    ///
    /// **`None` is a body from before they were declared**, an older build's
    /// rig read across a worker boundary, and only that: every plan declares
    /// its own. A rig with none goes on counting them off its rest pose, as
    /// every rig did ([`crate::Rig::legs`]), which is what the older build that
    /// sent it did with it. So it is read as missing, and never as a person's
    /// two the way a missing part of the anatomy is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legs: Option<Legs>,
}

impl Anatomy {
    /// A person's: a human head, trunk and feet, hands on the limbs that bear
    /// no weight, and the two hind limbs bearing it.
    pub const HUMAN: Self = Self {
        head: HeadKind::Human,
        trunk: TrunkKind::Human,
        hands: true,
        feet: FootKind::Human,
        legs: Some(Legs::HIND),
    };

    /// A four-legged beast's: nothing human, no hands, and all four limbs
    /// bearing its weight.
    pub const BEAST: Self = Self {
        head: HeadKind::Beast,
        trunk: TrunkKind::Beast,
        hands: false,
        feet: FootKind::Beast,
        legs: Some(Legs::ALL),
    };

    /// Whether the head is a human skull, which everything facial and the
    /// scalp's hair are built on.
    #[must_use]
    pub fn human_head(self) -> bool {
        self.head == HeadKind::Human
    }

    /// Whether the trunk is a human one: carved as a chest, and dressed.
    #[must_use]
    pub fn human_trunk(self) -> bool {
        self.trunk == TrunkKind::Human
    }

    /// Whether a weight-bearing limb's foot is carved from the cage as a
    /// biped's is.
    #[must_use]
    pub fn human_feet(self) -> bool {
        self.feet == FootKind::Human
    }
}

/// A person's, because every body this crate built before #365 that did not
/// stand on four legs was treated as one - and a skeleton assembled by hand, in
/// a test or a probe, is almost always a biped standing in for a person.
impl Default for Anatomy {
    fn default() -> Self {
        Self::HUMAN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_anatomies_disagree_about_every_part() {
        let (human, beast) = (Anatomy::HUMAN, Anatomy::BEAST);
        assert!(human.human_head() && !beast.human_head());
        assert!(human.human_trunk() && !beast.human_trunk());
        assert!(human.human_feet() && !beast.human_feet());
        assert!(human.hands && !beast.hands);
        assert_eq!(Anatomy::default(), human);
    }

    #[test]
    fn an_anatomy_round_trips_and_a_missing_one_reads_as_a_person() {
        let json = serde_json::to_string(&Anatomy::BEAST).expect("serialises");
        let back: Anatomy = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, Anatomy::BEAST);
        let partial: Anatomy = serde_json::from_str(r#"{"head":"beast"}"#).expect("parses");
        assert_eq!(partial.head, HeadKind::Beast);
        assert_eq!(partial.trunk, TrunkKind::Human);
    }

    #[test]
    fn an_anatomy_from_before_legs_were_declared_leaves_them_to_be_counted() {
        // #419: an older worker's rig carries no `legs`. It is missing, not a
        // person's two, or every beast an older worker sent would stand on its
        // hind legs; and a declared one is written and read back.
        let older = r#"{"head":"beast","trunk":"beast","hands":false,"feet":"hooves"}"#;
        let read: Anatomy = serde_json::from_str(older).expect("parses");
        assert_eq!(read.legs, None);
        assert_eq!(read.feet, FootKind::Hooves);
        let json = serde_json::to_value(Anatomy::HUMAN).expect("serialises");
        assert_eq!(json["legs"], serde_json::json!(0b1100));
        for anatomy in [Anatomy::HUMAN, Anatomy::BEAST] {
            assert_eq!(
                crate::tolerant::through_both::<Anatomy>(&serde_json::to_value(anatomy).unwrap()),
                anatomy
            );
        }
    }

    #[test]
    fn the_declared_legs_are_a_person_s_two_and_a_beast_s_four() {
        assert_eq!(
            Anatomy::HUMAN
                .legs
                .map(|legs| legs.iter().collect::<Vec<_>>()),
            Some(vec![Limb::HindLeft, Limb::HindRight])
        );
        assert_eq!(Anatomy::BEAST.legs.map(Legs::len), Some(4));
        assert_eq!(Limb::ALL.into_iter().collect::<Legs>(), Legs::ALL);
        assert!(Legs::NONE.is_empty());
        assert_eq!(format!("{:?}", Legs::HIND), "[HindLeft, HindRight]");
    }

    #[test]
    fn every_kind_reads_back_and_one_this_build_does_not_know_reads_as_a_beast_s() {
        // #419: a newer build's head, trunk or foot, bare or carrying content,
        // through the two codecs a built body crosses by.
        use crate::tolerant::{round_trips, through_both, unknowns};
        round_trips(&[HeadKind::Human, HeadKind::Beast]);
        round_trips(&[TrunkKind::Human, TrunkKind::Beast]);
        round_trips(&[FootKind::Human, FootKind::Beast, FootKind::Hooves]);
        for unknown in unknowns() {
            assert_eq!(through_both::<HeadKind>(&unknown), HeadKind::Beast);
            assert_eq!(through_both::<TrunkKind>(&unknown), TrunkKind::Beast);
            assert_eq!(through_both::<FootKind>(&unknown), FootKind::Beast);
        }
        let newer = serde_json::json!({
            "head": "beak", "trunk": "keel", "hands": false, "feet": "talons",
            "legs": 0b1100, "wings": 0b0011,
        });
        let read = through_both::<Anatomy>(&newer);
        assert_eq!(
            read,
            Anatomy {
                legs: Some(Legs::HIND),
                ..Anatomy::BEAST
            }
        );
    }
}
