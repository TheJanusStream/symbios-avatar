//! The records an avatar lives in.
//!
//! These are the Rust side of the `network.symbios.avatar.*` lexicons. An
//! avatar is a small parametric record in the owner's AT Protocol repository —
//! the source of truth — and geometry is derived from it on demand rather than
//! stored. Identity owns the avatar, not any one application.
//!
//! ## Wardrobe, not a single avatar
//!
//! Each avatar is its own record under its own record key, and a separate
//! [`ProfileRecord`] names which one is the default. Pinning an identity to
//! exactly one avatar is a limitation people notice quickly.
//!
//! ## Schema evolution
//!
//! Records only ever grow. New fields are optional with `#[serde(default)]`
//! **on the field**, never on the container — a container-level default silently
//! resets sibling fields when one is missing, which is the subtle way this goes
//! wrong. Unknown fields are ignored on read, so a record written by a newer
//! build still loads.
//!
//! ## Budget
//!
//! The protocol caps a record at one megabyte; this crate holds itself to
//! [`RECORD_BUDGET_BYTES`], well inside it. A small record is not just polite —
//! it forces the parameterisation to stay high-level, which is what keeps the
//! creator comprehensible.

mod code;
mod lock;

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use crate::plan::{Archetype, Category};
use crate::skeleton::Skeleton;

pub use code::{SHARE_CODE_VERSION, ShareCodeError};
pub use lock::LockSet;

/// The lexicon this crate's avatar records belong to.
pub const AVATAR_NSID: &str = "network.symbios.avatar.avatar";

/// The lexicon naming an identity's default avatar.
pub const PROFILE_NSID: &str = "network.symbios.avatar.profile";

/// Self-imposed ceiling on a serialised avatar record, in bytes.
///
/// The protocol's hard limit is 1 MB; staying far under it leaves room for the
/// record to grow through hair, outfits, and accessories without ever
/// approaching a wall.
pub const RECORD_BUDGET_BYTES: usize = 100 * 1024;

/// Longest accepted avatar name, in characters.
pub const MAX_NAME_CHARS: usize = 64;

/// One avatar: a parametric body plus the state its creator needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarRecord {
    /// Display name, as shown in a wardrobe.
    #[serde(default)]
    pub name: String,
    /// The body this avatar describes.
    #[serde(default)]
    pub archetype: Archetype,
    /// Seed of the last re-roll, kept so a look can be reproduced.
    ///
    /// Signed because AT Protocol integers are signed 64-bit; an unsigned seed
    /// would serialise to a number some readers cannot represent.
    #[serde(default)]
    pub seed: i64,
    /// Categories a re-roll must leave alone.
    #[serde(default)]
    pub locks: LockSet,
    /// When the record was created, as an ISO-8601 timestamp.
    ///
    /// Supplied by the application: this crate stays clock-free so it can build
    /// bodies anywhere, including on wasm where the system clock will panic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

impl Default for AvatarRecord {
    fn default() -> Self {
        Self {
            name: String::from("Unnamed"),
            archetype: Archetype::default(),
            seed: 0,
            locks: LockSet::NONE,
            created_at: None,
        }
    }
}

impl AvatarRecord {
    /// A named avatar with the given body.
    #[must_use]
    pub fn new(name: impl Into<String>, archetype: Archetype) -> Self {
        let mut record = Self {
            name: name.into(),
            archetype,
            ..Self::default()
        };
        record.sanitize();
        record
    }

    /// Clamps every field into range.
    ///
    /// Idempotent: sanitising a sanitised record changes nothing. Call it after
    /// reading a record from the network, where nothing about the contents can
    /// be assumed.
    pub fn sanitize(&mut self) {
        if self.name.chars().count() > MAX_NAME_CHARS {
            self.name = self.name.chars().take(MAX_NAME_CHARS).collect();
        }
        self.name = self.name.trim().to_string();
        self.archetype.sanitize();
    }

    /// Builds the capsule graph for this avatar's body.
    ///
    /// Sanitise first if the record came from outside; a sanitised record always
    /// produces a skeleton [`crate::cage::build_cage`] can mesh.
    #[must_use]
    pub fn skeleton(&self) -> Skeleton {
        self.archetype.skeleton()
    }

    /// Draws new values for every unlocked category.
    ///
    /// Each category draws from its own stream derived from `seed`, so locking
    /// one category never reshuffles another — a lock is a promise about what
    /// stays, and it would be broken if unlocking changed unrelated axes.
    pub fn reroll(&mut self, seed: i64) {
        self.seed = seed;
        for category in Category::ALL {
            if self.locks.is_locked(category) {
                continue;
            }
            let mut rng = category_stream(seed, category);
            self.archetype.reroll(category, &mut rng);
        }
        self.sanitize();
    }

    /// Renders this avatar's look as a compact share code.
    #[must_use]
    pub fn share_code(&self) -> String {
        code::encode(&self.archetype)
    }

    /// Replaces this avatar's body with the look a share code describes,
    /// keeping its name, locks, and seed.
    ///
    /// # Errors
    ///
    /// Returns [`ShareCodeError`] if the code is malformed or unsupported.
    pub fn apply_share_code(&mut self, share_code: &str) -> Result<(), ShareCodeError> {
        self.archetype = code::decode(share_code)?;
        self.sanitize();
        Ok(())
    }

    /// Builds an avatar from a share code alone.
    ///
    /// # Errors
    ///
    /// Returns [`ShareCodeError`] if the code is malformed or unsupported.
    pub fn from_share_code(
        name: impl Into<String>,
        share_code: &str,
    ) -> Result<Self, ShareCodeError> {
        Ok(Self::new(name, code::decode(share_code)?))
    }

    /// Size of this record once serialised, in bytes.
    ///
    /// # Errors
    ///
    /// Returns the underlying `serde_json` error if the record cannot be
    /// serialised.
    pub fn serialized_size(&self) -> Result<usize, serde_json::Error> {
        serde_json::to_vec(self).map(|bytes| bytes.len())
    }

    /// Whether this record fits [`RECORD_BUDGET_BYTES`].
    #[must_use]
    pub fn fits_budget(&self) -> bool {
        self.serialized_size()
            .is_ok_and(|size| size <= RECORD_BUDGET_BYTES)
    }
}

/// An identity's avatar preferences.
///
/// Avatars live under their own record keys; this record says which one to wear
/// by default. Keeping the pointer separate means renaming or replacing the
/// default avatar does not disturb the avatars themselves.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileRecord {
    /// Record key of the wardrobe's default avatar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_avatar: Option<String>,
    /// When the record was created, as an ISO-8601 timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

impl ProfileRecord {
    /// A profile pointing at one avatar record key.
    #[must_use]
    pub fn pointing_at(record_key: impl Into<String>) -> Self {
        Self {
            default_avatar: Some(record_key.into()),
            created_at: None,
        }
    }
}

/// The random stream one category draws from for a given seed.
///
/// Mixing the category into the seed — rather than drawing categories in
/// sequence from one stream — is what makes locks independent.
fn category_stream(seed: i64, category: Category) -> Pcg64Mcg {
    let salt = (category as u64)
        .wrapping_add(1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    Pcg64Mcg::seed_from_u64(seed as u64 ^ salt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::HumanoidParams;

    #[test]
    fn a_new_record_is_sane_and_tiny() {
        let record = AvatarRecord::new("Ari", Archetype::default());
        assert_eq!(record.name, "Ari");
        assert!(record.fits_budget());
        assert!(
            record.serialized_size().expect("serialises") < 400,
            "a body is a few hundred bytes, not kilobytes"
        );
    }

    #[test]
    fn rerolling_respects_locks() {
        let mut record = AvatarRecord::new("Test", Archetype::default());
        record.reroll(1);
        let Archetype::Humanoid(first) = record.archetype else {
            panic!("archetype changed");
        };

        record.locks = LockSet::NONE.with(Category::Stature);
        record.reroll(2);
        let Archetype::Humanoid(second) = record.archetype else {
            panic!("archetype changed");
        };

        assert_eq!(second.height, first.height, "locked stature is preserved");
        assert_ne!(second.build, first.build, "unlocked build changed");
    }

    #[test]
    fn locking_one_category_does_not_reshuffle_another() {
        // The point of per-category streams: what a lock protects is exactly
        // what it says, and unlocking it leaves everything else alone.
        let mut free = AvatarRecord::new("A", Archetype::default());
        free.reroll(7);

        let mut locked = AvatarRecord::new("B", Archetype::default());
        locked.locks = LockSet::NONE.with(Category::Stature);
        locked.reroll(7);

        let (Archetype::Humanoid(free), Archetype::Humanoid(locked)) =
            (free.archetype, locked.archetype)
        else {
            panic!("archetype changed");
        };
        assert_eq!(free.build, locked.build);
        assert_eq!(free.head_size, locked.head_size);
        assert_ne!(free.height, locked.height, "only stature differed");
    }

    #[test]
    fn rerolling_is_deterministic() {
        let mut first = AvatarRecord::new("A", Archetype::default());
        let mut second = AvatarRecord::new("B", Archetype::default());
        first.reroll(42);
        second.reroll(42);
        assert_eq!(first.archetype, second.archetype);
    }

    #[test]
    fn a_fully_locked_record_does_not_move() {
        let mut record = AvatarRecord::new("Fixed", Archetype::default());
        record.reroll(3);
        let before = record.archetype.clone();

        for category in Category::ALL {
            record.locks = record.locks.with(category);
        }
        assert!(record.locks.is_everything());
        record.reroll(999);
        assert_eq!(record.archetype, before);
    }

    #[test]
    fn sanitize_trims_names_and_is_idempotent() {
        let mut record = AvatarRecord {
            name: format!("  {}  ", "x".repeat(200)),
            ..Default::default()
        };
        record.sanitize();
        assert!(record.name.chars().count() <= MAX_NAME_CHARS);

        let once = record.clone();
        record.sanitize();
        assert_eq!(once, record, "sanitize must reach a fixpoint");
    }

    #[test]
    fn missing_fields_fall_back_without_disturbing_siblings() {
        // The field-level-default rule: a record that omits `locks` and
        // `createdAt` must keep the archetype it does specify.
        let json = r#"{"name":"Partial","archetype":{"$type":"network.symbios.avatar.defs#humanoid","height":2000}}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("deserialises");
        assert_eq!(record.name, "Partial");
        assert_eq!(record.locks, LockSet::NONE);
        assert_eq!(record.created_at, None);
        let Archetype::Humanoid(params) = record.archetype else {
            panic!("archetype changed");
        };
        assert_eq!(params.height, 2.0);
        assert_eq!(params.build, 0.0, "unspecified axes take their defaults");
    }

    #[test]
    fn unknown_fields_are_ignored_on_read() {
        let json = r#"{"name":"Future","hairstyle":{"kind":"bob"},"$type":"network.symbios.avatar.avatar"}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("tolerates new fields");
        assert_eq!(record.name, "Future");
    }

    #[test]
    fn records_round_trip_through_json() {
        let mut record = AvatarRecord::new("Round", Archetype::default());
        record.reroll(11);
        record.locks = LockSet::NONE.with(Category::Frame);
        record.created_at = Some("2026-08-01T10:00:00Z".into());

        let json = serde_json::to_string(&record).expect("serialises");
        let back: AvatarRecord = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(record, back);
    }

    #[test]
    fn share_codes_move_a_look_between_records() {
        let mut source = AvatarRecord::new("Source", Archetype::default());
        source.reroll(5);
        let code = source.share_code();

        let mut target = AvatarRecord::new("Target", Archetype::default());
        target.locks = LockSet::NONE.with(Category::Build);
        target.apply_share_code(&code).expect("applies");

        assert_eq!(
            target.name, "Target",
            "the look moves, the identity does not"
        );
        assert_eq!(target.locks, LockSet::NONE.with(Category::Build));

        let (Archetype::Humanoid(from), Archetype::Humanoid(to)) =
            (source.archetype, target.archetype)
        else {
            panic!("archetype changed");
        };
        assert!((from.height - to.height).abs() < 0.002);
    }

    #[test]
    fn a_profile_names_the_default_avatar() {
        let profile = ProfileRecord::pointing_at("3lm2k4x");
        let json = serde_json::to_string(&profile).expect("serialises");
        assert_eq!(json, r#"{"defaultAvatar":"3lm2k4x"}"#);
        let back: ProfileRecord = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(back, profile);
    }

    #[test]
    fn quadruped_records_work_the_same_way() {
        use crate::plan::QuadrupedParams;
        let mut record =
            AvatarRecord::new("Hound", Archetype::Quadruped(QuadrupedParams::default()));
        record.reroll(21);
        assert!(record.fits_budget());
        let back = AvatarRecord::from_share_code("Copy", &record.share_code()).expect("decodes");
        assert!(matches!(back.archetype, Archetype::Quadruped(_)));
    }

    #[test]
    fn a_record_with_a_humanoid_body_names_its_archetype() {
        let record = AvatarRecord::new("Named", Archetype::Humanoid(HumanoidParams::default()));
        assert_eq!(record.archetype.name(), "humanoid");
    }
}
