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
//! Records only ever grow, and every rule here exists because the alternative
//! was reproduced and found to corrupt bodies.
//!
//! * **`#[serde(default)]` goes on the container, not on the field.** This is
//!   the opposite of what this module used to say, and the old advice was wrong.
//!   A field-level default on a scalar yields the *type's* zero, not the
//!   struct's default: `{"skin":{"melanin":600}}` parsed to `blush: 0.0` where
//!   the default is `0.45`, giving pale, shut-eyed, black-haired avatars from
//!   any partial object — which is exactly what a field-eliding encoder writes.
//!   A container-level default fills only the fields that are *absent*; it does
//!   not touch siblings that are present.
//! * **Integers are read wide, then clamped.** An axis that arrives out of
//!   `i32` range must be clamped by [`AvatarRecord::sanitize`], not rejected by
//!   the parser before sanitising can run.
//! * **Unknown fields are kept, not ignored.** [`AvatarRecord::extra`] holds
//!   them, so an older client editing a newer client's record writes back what
//!   it did not understand instead of deleting it.
//! * **Unknown `$type`s and unknown tokens degrade rather than fail.** See
//!   [`crate::plan::Archetype`] and [`crate::dress::Sleeve`].
//!
//! ## Budget
//!
//! The protocol caps a record at one megabyte; this crate holds itself to
//! [`RECORD_BUDGET_BYTES`], well inside it. A small record is not just polite —
//! it forces the parameterisation to stay high-level, which is what keeps the
//! creator comprehensible.

mod code;
mod lock;

use serde::{Deserialize, Serialize};

use crate::dress::OutfitParams;
use crate::face::{EyeParams, FaceParams};
use crate::hair::HairParams;
use crate::plan::{Archetype, Category, Composites, Rolls};
use crate::skeleton::Skeleton;
use crate::texture::SkinParams;

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

/// Which generation of the re-roll this build implements.
///
/// A seed is stored so a look can be reproduced, and that promise only holds
/// against the generator that drew it. Per-axis streams (see
/// [`crate::plan::Rolls`]) mean adding or removing an axis no longer disturbs
/// the others, so this should move rarely — but when it does, a reader carrying
/// an older number knows the body it rebuilds is not the body that was rolled.
///
/// **2** — the exploration distributions (#160). Every shape axis moved from
/// a uniform draw inside a conservative fence to [`crate::plan::Rolls::shape`]:
/// a Gaussian on the axis's own default with the old fence as its width, plus
/// a rare wildcard over the whole widened envelope. Stream names are
/// untouched, so axis independence survives — but the same seed maps its
/// stream to a different value now, which is exactly what this number exists
/// to say. Complexion and hair kept their uniform draws and reproduce.
///
/// **1** — the first numbered generation. Everything before it drew whole
/// categories in sequence from one stream, so any seed rolled by an earlier
/// build reproduces a different person here. That break is taken deliberately
/// and once, while the lexicon is unpublished and nothing depends on it.
pub const GENERATOR_VERSION: u32 = 2;

/// A field the lexicon requires that this record does not carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NotPublishable {
    /// The record has no creation timestamp.
    ///
    /// This crate is deliberately clock-free — `std::time` panics on wasm — so
    /// the application supplies it. Nothing else can.
    #[error("createdAt is required by the lexicon and this record has none")]
    MissingCreatedAt,
    /// The avatar has no name.
    #[error("name is required by the lexicon and this record's is empty")]
    MissingName,
}

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
    /// How the body is described at the level a person is described at.
    ///
    /// The high tier of the two-tier parameterisation (#161): these fan out
    /// through formulas to many quantities at once, and the per-region axes on
    /// [`Self::archetype`] and the face blocks apply as offsets on top. Kept
    /// here rather than inside the archetype because the cage, the skull and
    /// the skin all read them, and none of those should have to know which body
    /// plan is in use to do it.
    #[serde(default)]
    pub composites: Composites,
    /// The complexion painted onto it.
    #[serde(default)]
    pub skin: SkinParams,
    /// How its eyes are shaped and set.
    #[serde(default)]
    pub eyes: EyeParams,
    /// How prominent its nose, brow, mouth and ears are.
    #[serde(default)]
    pub face: FaceParams,
    /// The hair grown on its head.
    #[serde(default)]
    pub hair: HairParams,
    /// What it is wearing.
    #[serde(default)]
    pub outfit: OutfitParams,
    /// Seed of the last re-roll, kept so a look can be reproduced.
    ///
    /// Signed because AT Protocol integers are signed 64-bit; an unsigned seed
    /// would serialise to a number some readers cannot represent.
    #[serde(default)]
    pub seed: i64,
    /// Categories a re-roll must leave alone.
    #[serde(default)]
    pub locks: LockSet,
    /// Which build of the generator last re-rolled this record.
    ///
    /// A seed only reproduces a look against the generator that drew it. Every
    /// care is taken that it keeps doing so — each axis draws from its own named
    /// stream, so adding an axis cannot shift another — but a deliberate change
    /// to how an axis is drawn is still possible, and a reader has to be able to
    /// tell. See [`GENERATOR_VERSION`].
    #[serde(default)]
    pub generator: u32,
    /// When the record was created, as an ISO-8601 timestamp.
    ///
    /// Supplied by the application: this crate stays clock-free so it can build
    /// bodies anywhere, including on wasm where the system clock will panic.
    /// The lexicon marks it required, so a record without one is not
    /// publishable — see [`AvatarRecord::publishable`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Fields this build does not know about, kept verbatim.
    ///
    /// Without this, an older client reading and re-writing a record silently
    /// deletes every field a newer client added — the read-modify-write cycle
    /// that quietly destroys data across a whole network of readers.
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Default for AvatarRecord {
    fn default() -> Self {
        Self {
            name: String::from("Unnamed"),
            archetype: Archetype::default(),
            composites: Composites::default(),
            skin: SkinParams::default(),
            eyes: EyeParams::default(),
            face: FaceParams::default(),
            hair: HairParams::default(),
            outfit: OutfitParams::default(),
            seed: 0,
            locks: LockSet::NONE,
            generator: GENERATOR_VERSION,
            created_at: None,
            extra: std::collections::BTreeMap::new(),
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
        self.composites.sanitize();
        self.skin.sanitize();
        self.eyes.sanitize();
        self.face.sanitize();
        self.hair.sanitize();
        self.outfit.sanitize();
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
        self.generator = GENERATOR_VERSION;
        let rolls = Rolls::new(seed);
        for category in Category::ALL {
            if self.locks.is_locked(category) {
                continue;
            }
            self.archetype.reroll(category, &rolls);
            self.composites.reroll(category, &rolls);
            // The three groups #53 split out of the old `Features` bit. Each
            // draws from the streams it always drew from, so no seed names a
            // different person for this; what changed is that a creator who
            // has found a face can now roll a complexion without losing it.
            match category {
                Category::Head => reroll_face(&mut self.eyes, &mut self.face, &rolls),
                Category::Colouring => reroll_skin(&mut self.skin, &rolls),
                Category::Hair => reroll_hair(&mut self.hair, &rolls),
                _ => {}
            }
        }
        self.sanitize();
    }

    /// Whether this record carries everything the lexicon marks required.
    ///
    /// The crate can write a partial record — that is what an in-progress
    /// creator holds — but a PDS that resolves the lexicon rejects one, and
    /// finding that out at the point of publication is too late. Call this
    /// before writing.
    ///
    /// # Errors
    ///
    /// Returns the first required field that is missing.
    pub fn publishable(&self) -> Result<(), NotPublishable> {
        if self.name.trim().is_empty() {
            return Err(NotPublishable::MissingName);
        }
        if self
            .created_at
            .as_ref()
            .is_none_or(|at| at.trim().is_empty())
        {
            return Err(NotPublishable::MissingCreatedAt);
        }
        Ok(())
    }

    /// Stamps the record with the time its owner created it.
    ///
    /// The one required field this crate cannot supply for itself.
    #[must_use]
    pub fn created(mut self, timestamp: impl Into<String>) -> Self {
        self.created_at = Some(timestamp.into());
        self
    }

    /// Renames the avatar.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Renders this avatar's look as a compact share code.
    #[must_use]
    pub fn share_code(&self) -> String {
        code::encode(&self.archetype, &self.composites, &self.skin)
    }

    /// Replaces this avatar's body with the look a share code describes,
    /// keeping its name, locks, and seed.
    ///
    /// # Errors
    ///
    /// Returns [`ShareCodeError`] if the code is malformed or unsupported.
    pub fn apply_share_code(&mut self, share_code: &str) -> Result<(), ShareCodeError> {
        let (archetype, composites, skin) = code::decode(share_code)?;
        self.archetype = archetype;
        self.composites = composites;
        self.skin = skin;
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
        let (archetype, composites, skin) = code::decode(share_code)?;
        let mut record = Self::new(name, archetype);
        record.composites = composites;
        record.skin = skin;
        record.sanitize();
        Ok(record)
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
    ///
    /// `created_at` is taken here rather than left to be filled in later,
    /// because the lexicon marks it required and the version of this
    /// constructor that did not take one produced a record every conformant PDS
    /// rejects — with nothing in the type system, and nothing in the tests, to
    /// say so.
    #[must_use]
    pub fn pointing_at(record_key: impl Into<String>, created_at: impl Into<String>) -> Self {
        Self {
            default_avatar: Some(record_key.into()),
            created_at: Some(created_at.into()),
        }
    }

    /// Whether this record carries everything the lexicon marks required.
    ///
    /// # Errors
    ///
    /// Returns the first required field that is missing.
    pub fn publishable(&self) -> Result<(), NotPublishable> {
        if self
            .created_at
            .as_ref()
            .is_none_or(|at| at.trim().is_empty())
        {
            return Err(NotPublishable::MissingCreatedAt);
        }
        Ok(())
    }
}

/// Draws a new face: the eyes, and what is carved around them.
///
/// **Shape only.** This used to draw the complexion and the hair as well,
/// because all three rode the one `Features` lock; #53 gave each its own, and
/// the argument that grouped them — "a creator with one lock per slider ends up
/// with more locks than anyone reads" — turned out to be answering the wrong
/// question. The right one is what somebody would keep on purpose, and a face
/// is kept while its colouring is rolled all the time.
fn reroll_face(eyes: &mut EyeParams, face: &mut FaceParams, rolls: &Rolls) {
    // Shape axes draw [`Rolls::shape`] (#160): a Gaussian on each axis's own
    // default with sigma half the old uniform fence — so a typical seed still
    // lands where it always did — plus the wildcard tail over the whole
    // exploration envelope. Stream names are unchanged; the distribution
    // change is `GENERATOR_VERSION` 2's.
    use crate::plan::explore_range;
    let unit = |d: f32| explore_range(d, (0.0, 1.0));
    face.nose = rolls.shape("face.nose", 0.5, 0.375, unit(0.5));
    face.brow = rolls.shape("face.brow", 0.5, 0.4, unit(0.5));
    face.mouth = rolls.shape("face.mouth", 0.5, 0.4, unit(0.5));
    face.ears = rolls.shape("face.ears", 0.5, 0.375, unit(0.5));
    face.nose_width = rolls.shape("face.noseWidth", 0.5, 0.35, unit(0.5));
    face.mouth_width = rolls.shape("face.mouthWidth", 0.5, 0.375, unit(0.5));

    // The eyes draw Gaussian like every shape axis but stay inside the CLASSIC
    // range rather than the envelope: an eye is the one feature whose extremes
    // read as broken rather than stylised — a spacing past ±1 runs the iris
    // into the skin of the nose and `an_eye_shows_white_on_both_sides_of_its_iris`
    // fails on the population, which is the guard saying a rolled body stopped
    // reading as inhabited. The editor's sliders still reach the envelope;
    // a broken-looking eye should be a choice somebody made, never a roll.
    eyes.size = rolls.shape("eyes.size", 0.5, 0.3, (0.0, 1.0));
    eyes.spacing = rolls.shape("eyes.spacing", 0.0, 0.6, (-1.0, 1.0));
    eyes.depth = rolls.shape("eyes.depth", 0.0, 0.6, (-1.0, 1.0));
    eyes.aperture = rolls.shape("eyes.aperture", 0.8, 0.225, (0.0, 1.0));
}

/// Draws a fresh head of hair.
///
/// Its own category since #53. Hair is the loudest thing about a head and the
/// one most often kept while everything under it changes, which is the whole
/// argument for a lock of its own.
fn reroll_hair(hair: &mut HairParams, rolls: &Rolls) {
    // Hair is a style, not a shape: it keeps its uniform draws and its
    // conservative ranges (#160, owner call — same reason complexion does).
    hair.length = rolls.range("hair.length", 0.0, 1.0);
    hair.volume = rolls.range("hair.volume", -0.7, 0.9);
    hair.coverage = rolls.range("hair.coverage", -0.8, 0.8);
    hair.part = rolls.range("hair.part", -1.0, 1.0);
    hair.wave = rolls.range("hair.wave", 0.0, 1.0);
    hair.shade = rolls.range("hair.shade", 0.0, 1.0);
}

fn reroll_skin(skin: &mut SkinParams, rolls: &Rolls) {
    // Complexion stays UNIFORM while the shape axes went Gaussian (#160, an
    // owner call worth restating): a Gaussian centred on the default would
    // make every complexion far from it a "rare extreme", and a skin tone is
    // not an extreme of anything.
    skin.melanin = rolls.range("skin.melanin", 0.0, 1.0);
    skin.undertone = rolls.range("skin.undertone", -1.0, 1.0);
    skin.blush = rolls.range("skin.blush", 0.15, 0.8);
    // Most people have neither, so both stay off more often than not. The
    // coin and the amount draw from separate streams, so changing how much
    // stubble a stubbled face has cannot change which faces have any.
    skin.freckles = if rolls.chance("skin.freckled", 0.3) {
        rolls.range("skin.freckles", 0.2, 1.0)
    } else {
        0.0
    };
    skin.stubble = if rolls.chance("skin.stubbled", 0.25) {
        rolls.range("skin.stubble", 0.3, 1.0)
    } else {
        0.0
    };
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
        // The bound is a ratchet on a record that should stay small, not a
        // budget — [`RECORD_BUDGET_BYTES`] is that, and it is a hundred and
        // thirty times further away. Raised 700 -> 800 when the composites
        // block landed (#162), which is the first time it has moved: measured,
        // a fresh record went 683 -> 745 bytes, so the four axes cost 62.
        // Report the size when it fires, because the question a reader has is
        // "by how much".
        let size = record.serialized_size().expect("serialises");
        assert!(
            size < 800,
            "a fresh record is {size} bytes; a body is a few hundred, not kilobytes"
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
    fn a_partial_object_keeps_the_defaults_of_the_fields_it_omits() {
        // Reproduced defect: a field-level serde(default) on a scalar yields the
        // TYPE's zero, not the struct's default, so one specified axis dragged
        // every sibling to zero — pale, shut-eyed, black-haired avatars out of
        // any field-eliding encoder. Omitting the object entirely was always
        // fine; the partial object was the bug, and it is what a JS client
        // writes.
        let json = r#"{"name":"Partial","skin":{"melanin":600},"eyes":{"size":700}}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("deserialises");

        assert_eq!(record.skin.melanin, 0.6, "the specified axis is honoured");
        assert_eq!(
            record.skin.blush,
            SkinParams::default().blush,
            "an omitted sibling keeps the default, not zero"
        );
        assert_eq!(record.eyes.size, 0.7);
        assert_eq!(record.eyes.aperture, EyeParams::default().aperture);
        assert_eq!(record.hair.locks, HairParams::default().locks);
    }

    #[test]
    fn a_record_written_before_composites_existed_reads_as_the_neutral_body() {
        // Every avatar already stored was written without this block, and the
        // whole two-tier design rests on those records still meaning what they
        // meant: the composites default to the description whose formulas
        // reproduce the body the archetype builds on its own (#161, #162).
        //
        // This is also the axis pair that would have caught the #19-25 defect
        // on its own. `bodyFat` and `age` have NON-ZERO defaults, so a
        // field-level default here would give a record a bodyless 0.0 fraction
        // and a newborn's age rather than the neutral adult.
        let json = r#"{"name":"Older","composites":{"femininity":250}}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("deserialises");
        assert_eq!(record.composites.femininity, 0.25, "the stated axis holds");
        assert_eq!(
            record.composites.body_fat,
            Composites::default().body_fat,
            "an omitted composite keeps its default, not zero"
        );
        assert_eq!(record.composites.age, Composites::default().age);

        let absent: AvatarRecord = serde_json::from_str(r#"{"name":"Old"}"#).expect("deserialises");
        assert_eq!(absent.composites, Composites::default());
    }

    #[test]
    fn a_face_can_be_kept_while_its_colouring_is_rolled() {
        // The whole of #53 in one assertion. Under the old single `Features`
        // bit this was impossible: keeping a skull meant keeping the
        // complexion, the hair, the hands and the eyes with it, and the axes
        // people most want to hold separately were the ones fused together.
        let mut record = AvatarRecord::new("Kept", Archetype::default());
        record.reroll(31);
        let face = record.face;
        let eyes = record.eyes;
        let Archetype::Humanoid(before) = record.archetype else {
            panic!("archetype changed");
        };

        record.locks = LockSet::NONE.with(Category::Head);
        record.reroll(32);
        let Archetype::Humanoid(after) = record.archetype else {
            panic!("archetype changed");
        };

        assert_eq!(
            record.face, face,
            "a locked head keeps what is carved on it"
        );
        assert_eq!(record.eyes, eyes, "and keeps its eyes");
        assert_eq!(after.head_size, before.head_size);
        assert_eq!(after.head_breadth, before.head_breadth);
        assert_eq!(after.face_length, before.face_length);
        assert_ne!(
            record.skin.melanin, 0.0,
            "the complexion rolled, which is the point"
        );
    }

    #[test]
    fn colouring_and_hair_lock_apart_from_each_other() {
        let mut record = AvatarRecord::new("Apart", Archetype::default());
        record.reroll(41);
        let hair = record.hair;
        let skin = record.skin;

        record.locks = LockSet::NONE.with(Category::Hair);
        record.reroll(42);
        assert_eq!(record.hair, hair, "locked hair survives");
        assert_ne!(record.skin, skin, "unlocked colouring does not");

        record.locks = LockSet::NONE.with(Category::Colouring);
        let held = record.skin;
        record.reroll(43);
        assert_eq!(record.skin, held, "and the reverse holds too");
    }

    #[test]
    fn extremity_size_is_held_by_the_proportions_it_belongs_with() {
        // Moved out of `Features` in #53: a hand is a proportion of the arm it
        // ends, and nobody locks a face to hold a hand.
        let mut record = AvatarRecord::new("Hands", Archetype::default());
        record.reroll(51);
        let Archetype::Humanoid(before) = record.archetype else {
            panic!("archetype changed");
        };

        record.locks = LockSet::NONE.with(Category::Proportions);
        record.reroll(52);
        let Archetype::Humanoid(after) = record.archetype else {
            panic!("archetype changed");
        };
        assert_eq!(after.extremity_size, before.extremity_size);
        assert_eq!(after.limb_length, before.limb_length);
    }

    #[test]
    fn age_locks_on_its_own() {
        let mut record = AvatarRecord::new("Aged", Archetype::default());
        record.reroll(61);
        let age = record.composites.age;

        record.locks = LockSet::NONE.with(Category::Age);
        record.reroll(62);
        assert_eq!(record.composites.age, age, "a locked age is kept");
        assert_ne!(
            record.composites.mass, 0.0,
            "and holding it does not hold the body"
        );
    }

    #[test]
    fn locks_reach_the_composites_they_own() {
        // A lock is a promise about what stays, and it would be a lie if it
        // held the archetype's `build` while re-rolling the `mass` that is
        // going to replace it (#164). Mass and body fat belong to Build, the
        // frame axis to Frame.
        let mut record = AvatarRecord::new("Locked", Archetype::default());
        record.reroll(11);
        let first = record.composites;

        record.locks = LockSet::NONE.with(Category::Build);
        record.reroll(12);
        assert_eq!(record.composites.mass, first.mass, "locked mass is kept");
        assert_eq!(
            record.composites.body_fat, first.body_fat,
            "locked body fat is kept"
        );
        assert_ne!(
            record.composites.femininity, first.femininity,
            "an unlocked composite still moves"
        );
    }

    #[test]
    fn an_out_of_range_number_is_clamped_rather_than_failing_the_parse() {
        // Sanitising cannot run on a record that would not load, so the reader
        // has to be wide enough to accept the value first.
        let json = r#"{"name":"Wide","skin":{"melanin":3000000000},"hair":{"locks":4000000000}}"#;
        let mut record: AvatarRecord = serde_json::from_str(json).expect("loads a wild value");
        record.sanitize();
        assert_eq!(record.skin.melanin, 1.0);
        assert_eq!(record.hair.locks, crate::hair::MAX_LOCKS);
    }

    #[test]
    fn unknown_fields_are_kept_so_an_old_client_cannot_delete_them() {
        // The read-modify-write that quietly destroys data: an older build reads
        // a newer build's record, writes it back, and every field it did not
        // understand is gone.
        let json = r#"{"name":"Future","hairstyle":{"kind":"bob"},"$type":"network.symbios.avatar.avatar"}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("tolerates new fields");
        assert_eq!(record.name, "Future");

        let back = serde_json::to_value(&record).expect("serialises");
        assert_eq!(back["hairstyle"]["kind"], "bob", "a new field was dropped");
        assert_eq!(back["$type"], AVATAR_NSID, "the record type was dropped");
    }

    #[test]
    fn an_unknown_archetype_still_loads_and_survives_a_rewrite() {
        // WS6 adds creature archetypes by design. On the day the first one
        // exists, every deployed client must still render a placeholder rather
        // than lose the name, the seed and the locks along with the body.
        let json = r#"{"name":"Hexapod","seed":42,"archetype":{"$type":"network.symbios.avatar.defs#hexapod","height":900,"legs":6}}"#;
        let mut record: AvatarRecord = serde_json::from_str(json).expect("loads an unknown body");
        assert_eq!(record.name, "Hexapod");
        assert_eq!(record.seed, 42);
        assert!(!record.archetype.is_understood());
        assert_eq!(
            record.archetype.name(),
            "network.symbios.avatar.defs#hexapod"
        );

        // Something renders.
        assert!(!record.skeleton().nodes.is_empty());
        // And nothing this build does to it loses what it could not read.
        record.sanitize();
        record.reroll(7);
        let back = serde_json::to_value(&record).expect("serialises");
        assert_eq!(back["archetype"]["legs"], 6);
        assert_eq!(back["archetype"]["height"], 900);
        assert_eq!(
            back["archetype"]["$type"],
            "network.symbios.avatar.defs#hexapod"
        );
    }

    #[test]
    fn an_unknown_garment_cut_is_worn_as_the_default() {
        use crate::dress::{Leg, Sleeve};
        let json = r#"{"name":"Dressed","outfit":{"sleeve":"cape","leg":"culottes"}}"#;
        let record: AvatarRecord = serde_json::from_str(json).expect("loads unknown tokens");
        assert_eq!(record.outfit.sleeve, Sleeve::Other("cape".into()));
        assert_eq!(record.outfit.leg.cut(), Leg::default());

        // And the token survives a rewrite, so a newer client's cape is intact.
        let back = serde_json::to_value(&record).expect("serialises");
        assert_eq!(back["outfit"]["sleeve"], "cape");
    }

    #[test]
    fn a_record_says_when_it_cannot_be_published() {
        // Both lexicons mark createdAt required, and this crate is clock-free by
        // design, so the one thing it can do is refuse to pretend.
        let record = AvatarRecord::new("Unstamped", Archetype::default());
        assert_eq!(record.publishable(), Err(NotPublishable::MissingCreatedAt));

        let stamped = record.created("2026-08-02T09:00:00Z");
        assert_eq!(stamped.publishable(), Ok(()));

        let nameless = AvatarRecord::default()
            .created("2026-08-02T09:00:00Z")
            .named("");
        assert_eq!(nameless.publishable(), Err(NotPublishable::MissingName));

        assert_eq!(
            ProfileRecord::default().publishable(),
            Err(NotPublishable::MissingCreatedAt)
        );
        assert_eq!(
            ProfileRecord::pointing_at("3lm2k4x", "2026-08-02T09:00:00Z").publishable(),
            Ok(())
        );
    }

    #[test]
    fn adding_an_axis_does_not_move_the_axes_beside_it() {
        // The property the whole per-axis-stream design exists for. Drawing in
        // sequence, inserting one axis shifts every later draw and seed 42
        // becomes a different person; keyed by name, only the new axis appears.
        let rolls = Rolls::new(42);
        let before: Vec<f32> = ["skin.melanin", "hair.length", "eyes.size"]
            .iter()
            .map(|axis| rolls.range(axis, 0.0, 1.0))
            .collect();

        // Whatever a future build inserts, it draws from its own stream.
        let _inserted = rolls.range("skin.freckleSize", 0.0, 1.0);

        let after: Vec<f32> = ["skin.melanin", "hair.length", "eyes.size"]
            .iter()
            .map(|axis| rolls.range(axis, 0.0, 1.0))
            .collect();
        assert_eq!(before, after);
        // And two axes do not secretly share a stream.
        assert_ne!(before[0], before[1]);
    }

    #[test]
    fn a_seed_reproduces_the_same_person() {
        // A golden table. It exists to make a change to the draw LOUD: if this
        // fails, every stored seed now names a different avatar, and
        // GENERATOR_VERSION has to move with it.
        assert_eq!(
            GENERATOR_VERSION, 2,
            "bump the table below with the version"
        );
        let quantised = |seed: i64| {
            let mut record = AvatarRecord::new("Golden", Archetype::default());
            record.reroll(seed);
            let Archetype::Humanoid(params) = record.archetype else {
                panic!("archetype changed");
            };
            (
                (params.height * 1000.0).round() as i32,
                (params.build * 1000.0).round() as i32,
                (record.skin.melanin * 1000.0).round() as i32,
                (record.hair.length * 1000.0).round() as i32,
            )
        };
        // Generation 2 (#160): height and build moved — the shape axes now
        // draw `Rolls::shape` — while melanin and hair length are IDENTICAL
        // to generation 1's table on all three seeds, which is the other half
        // of the contract: complexion and hair kept their uniform draws, so a
        // stored seed's colouring survives the generator bump.
        assert_eq!(quantised(1), (2353, 101, 594, 238));
        assert_eq!(quantised(42), (1012, -33, 933, 973));
        assert_eq!(quantised(-7), (2186, 781, 513, 903));
    }

    #[test]
    fn a_reroll_stamps_the_generation_that_drew_it() {
        let mut record = AvatarRecord::new("Stamped", Archetype::default());
        record.generator = 0;
        record.reroll(3);
        assert_eq!(record.generator, GENERATOR_VERSION);
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

        // The composites travel with the look, and this is the assertion that
        // says why the code went to version 5 the moment they existed (#162):
        // a description that stays behind is a look that changes when it is
        // passed between people, which is the one thing a code is for.
        assert!((target.composites.femininity - source.composites.femininity).abs() < 0.03);
        assert!((target.composites.body_fat - source.composites.body_fat).abs() < 0.01);
        assert!(target.composites.age.abs_diff(source.composites.age) <= 1);

        let (Archetype::Humanoid(from), Archetype::Humanoid(to)) =
            (source.archetype, target.archetype)
        else {
            panic!("archetype changed");
        };
        assert!((from.height - to.height).abs() < 0.002);
    }

    #[test]
    fn a_profile_names_the_default_avatar() {
        // Written WITH its timestamp: the lexicon marks createdAt required, and
        // the version of this test that asserted `{"defaultAvatar":"3lm2k4x"}`
        // was enshrining a record every conformant PDS rejects.
        let profile = ProfileRecord::pointing_at("3lm2k4x", "2026-08-01T00:00:00Z");
        let json = serde_json::to_string(&profile).expect("serialises");
        assert_eq!(
            json,
            r#"{"defaultAvatar":"3lm2k4x","createdAt":"2026-08-01T00:00:00Z"}"#
        );
        assert_eq!(profile.publishable(), Ok(()));
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
