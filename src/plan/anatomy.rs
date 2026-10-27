//! What a body's parts are, as its plan declares them (#365).
//!
//! The build's rules about a part are stated against this - a human skull is
//! shaped and carved into a face, a limb that bears no weight ends in a hand,
//! a biped's foot is carved from the cage, a human trunk is given a chest - and
//! never against how many feet a body stands on. Counting feet told a person
//! from a hound while there were two plans, and it stops telling anything the
//! day a plan stands on two legs with a beast's head, or on four with hands.
//! How many limbs bear weight is a question about the gait, and the gait still
//! asks it ([`crate::Rig::ground_contacts`]) - as does the choice of a hand or a
//! foot at the end of a limb, which is a question of which limbs bear weight.
//!
//! A plan declares its anatomy ([`super::BodyPlan::anatomy`]) and stamps it on
//! the skeleton it builds, which hands it to the rig rooted from it - so a pass
//! holding only a rig asks the same question a record does.

use serde::{Deserialize, Serialize};

/// What kind of head a body carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// What kind of trunk a body carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrunkKind {
    /// A human trunk: given a chest and its taper, and dressed - the garment
    /// catalogue is cut for a human trunk and the limbs that hang from it.
    #[default]
    Human,
    /// A beast's barrel. Neither carved as a chest nor dressed.
    Beast,
}

/// What kind of foot ends a limb that bears weight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

impl Anatomy {
    /// A person's: a human head, trunk and feet, and hands on the limbs that
    /// bear no weight.
    pub const HUMAN: Self = Self {
        head: HeadKind::Human,
        trunk: TrunkKind::Human,
        hands: true,
        feet: FootKind::Human,
    };

    /// A four-legged beast's: nothing human, and no hands.
    pub const BEAST: Self = Self {
        head: HeadKind::Beast,
        trunk: TrunkKind::Beast,
        hands: false,
        feet: FootKind::Beast,
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
}
