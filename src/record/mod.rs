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
use crate::hair::{HairRecord, ScalpStyle};
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
/// **5** — the hair priors. Every axis a re-roll draws for hair changed
/// distribution at once, and none of them changed name:
///
/// - the scalp draws a STYLE, where it had been `Crop` on every body since the
///   catalogue landed. Deliberately uncoupled from the frame axis — the
///   couplings this crate makes are anatomy, and a haircut is culture.
/// - the natural shade is dark-dominant rather than uniform. A uniform draw put
///   as many blondes on the street as black-haired people.
/// - hair GREYS with age, which no draw could say before the record carried
///   free colours: the melanin ramp's light end is a warm blonde, and grey is
///   hair with the pigment gone.
/// - one rolled head in fifty is dyed instead.
/// - facial hair is a quarter at neutral where it was a twentieth, and
///   symmetric about the frame axis rather than flat below it; and
///   a bearded face draws a coherent CONFIGURATION rather than three
///   independent coins.
///
/// Only the distributions moved. Stream names are untouched again, so a record
/// carrying an older number rebuilds every non-hair axis bit-identically — and
/// `HAIR_GENERATION` (private, below) deliberately does NOT follow this bump,
/// because a generation-4 record's hair is valid in the current schema and
/// re-rolling it would silently restyle avatars that are fine.
///
/// **3** — the two-tier draw, and the one bump every rename and removal was
/// batched into. Three things moved at once and each
/// would have moved every seed on its own:
///
/// - the order. Composites are drawn first and everything else is drawn
///   against them, so a body can be coherent — stature follows the frame axis,
///   and `bodyFat`'s centre follows `mass`, `femininity` and age.
/// - the width. Every per-region axis is an OFFSET on what a composite derives
///   now, so it draws at a third of its old sigma. Drawn at the old width the
///   offset out-swung the composite it was correcting, which is the
///   tall-heavy-gaunt incoherence this generation exists to remove.
/// - the priors. Age was centred at 28 with sigma 10, which put six of the
///   first eight seeds under `plan::AGE_PIVOT` where the age axis does nothing
///   at all; it is 38 with sigma 15. The hairline's window follows age and the
///   frame axis.
///
/// `humanoid.build` and `humanoid.muscle` are gone for good with it, their
/// share-code slots removed in the version-6 payload, and
/// `face::HeadTraits` is what `face::Dimorphism` was called. Stream NAMES are
/// otherwise untouched, so axis independence survives the bump exactly as it
/// did the last one.
///
/// **2** — the exploration distributions. Every shape axis moved from
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
pub const GENERATOR_VERSION: u32 = 5;

/// The generation whose hair a record has to be re-rolled to reach.
///
/// **The hair rewrite is the only migration this crate has needed, and it
/// needed one because the old axes cannot be mapped.** The superseded schema
/// described one object — a sculpted mass with locks cut into its rim, at one
/// length, one volume and one colour — and the record describes five
/// regions in two layers with a style and two colours each. There is no
/// function from the first to the second: a body with `length` 0.4 has no
/// answer for whether it has a beard.
///
/// So a record older than this has its hair **re-drawn from its own seed**,
/// deliberately, rather than mapped to a nearest look. It is
/// deterministic, it is what the same seed would produce today, and it is
/// honest about the break rather than inventing a beard nobody chose. Every
/// other axis is untouched: hair draws from streams named for itself, so the
/// rest of the body a seed describes is bit-identical across the bump.
///
/// **It stays at 4 while [`GENERATOR_VERSION`] is 5**, and the distinction is
/// the whole reason this constant is separate. That
/// bump changed what a FRESH roll produces; it did not make any stored record
/// unreadable. A generation-4 record describes five regions with styles and
/// colours the current build understands perfectly, and re-drawing its hair
/// because the priors behind it have improved would restyle somebody's avatar
/// to fix nothing. This moves only when a record's hair cannot be read at all.
const HAIR_GENERATION: u32 = 4;

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
    /// The high tier of the two-tier parameterisation: these fan out
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
    pub hair: HairRecord,
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
            hair: HairRecord::default(),
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

    /// The avatar a seed names: [`Self::new`] plus a full [`Self::reroll`],
    /// in one call.
    ///
    /// This is the entry point for a host that has no stored record and must
    /// invent one — hash something stable about an identity into a seed, and
    /// every reader that does the same derives the same person. What makes
    /// that a promise rather than a hope is the [`crate::plan::Rolls`]
    /// contract: each axis draws from its own named stream, so the result
    /// depends on `seed`, on this build's generation (stamped into
    /// [`Self::generator`]), and on nothing else. The
    /// `a_stored_seed_reproduces_its_person` test pins the draws for the
    /// current generation; a change that moves them must bump
    /// [`GENERATOR_VERSION`].
    ///
    /// The archetype is a parameter, not a draw. Which body plan an identity
    /// gets is the host's decision — a re-roll varies a body, it does not
    /// pick one — and drawing it here would burn that choice into every
    /// derived default.
    #[must_use]
    pub fn rolled(name: impl Into<String>, archetype: Archetype, seed: i64) -> Self {
        let mut record = Self::new(name, archetype);
        record.reroll(seed);
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
        self.migrate();
    }

    /// Brings a record forward from an older generation of the generator.
    ///
    /// **Run from [`Self::sanitize`], which is the one thing every path that
    /// takes a record from outside already calls.** A migration a caller has to
    /// remember is a migration that gets forgotten, and the failure would be
    /// silent: a record whose hair fields did not parse deserialises to a
    /// default head of hair rather than to an error.
    ///
    /// Idempotent, because it stamps the generation it brought the record to
    /// and does nothing when there is nothing to do.
    ///
    /// Stamping [`GENERATOR_VERSION`] afterwards is exact rather than
    /// approximate: generation 4 changed how hair is drawn and nothing else, so
    /// a generation-3 record with its hair re-drawn by 4 is what 4 would have
    /// drawn from that seed.
    fn migrate(&mut self) {
        if self.generator >= HAIR_GENERATION {
            return;
        }
        // Deliberately ignores the hair lock. A lock is a promise that a
        // category will not change, and this crate cannot keep it here: the
        // hair the lock was protecting is not representable any more. Breaking
        // it loudly in one place beats keeping a default head of hair that
        // nobody chose and calling it the locked one.
        reroll_hair(&mut self.hair, &Rolls::new(self.seed), &self.composites);
        self.hair.sanitize();
        self.generator = GENERATOR_VERSION;
    }

    /// Builds the capsule graph for this avatar's body.
    ///
    /// Sanitise first if the record came from outside; a sanitised record always
    /// produces a skeleton [`crate::cage::build_cage`] can mesh.
    #[must_use]
    pub fn skeleton(&self) -> Skeleton {
        self.archetype.skeleton(&self.composites)
    }

    /// Draws new values for every unlocked category.
    ///
    /// Each category draws from its own stream derived from `seed`, so locking
    /// one category never reshuffles another — a lock is a promise about what
    /// stays, and it would be broken if unlocking changed unrelated axes.
    ///
    /// **Two passes, and the order is the design.**
    /// The composites are drawn first, in full, and everything else is drawn
    /// afterwards against the result — which is what lets a body be coherent:
    /// stature can follow the frame axis, the offsets can be small because the
    /// composites carry the shape, and no draw has to guess at a value that
    /// another draw is about to make.
    ///
    /// **A locked composite still shapes what the second pass draws**, which
    /// falls out of reading `self.composites` rather than the pass-one result:
    /// a creator who locks a feminine frame and re-rolls gets bodies drawn
    /// around that frame rather than around a neutral one. That is what a lock
    /// should mean and it is the reason the two passes read the record instead
    /// of passing values between themselves.
    pub fn reroll(&mut self, seed: i64) {
        self.seed = seed;
        self.generator = GENERATOR_VERSION;
        let rolls = Rolls::new(seed);

        // Pass one: the composites, in an order of their own. `Category::ALL`
        // puts `Build` before `Frame`, and `body_fat`'s draw reads the frame
        // axis — the same fraction is a different body on a masculine and a
        // feminine frame — so the dependency is written out here rather than
        // left to the order a bitmask happens to declare its variants in.
        for category in [Category::Frame, Category::Build, Category::Age] {
            if self.locks.is_locked(category) {
                continue;
            }
            self.composites.reroll(category, &rolls);
        }

        // Pass two: everything that reads them.
        for category in Category::ALL {
            if self.locks.is_locked(category) {
                continue;
            }
            self.archetype.reroll(category, &rolls, &self.composites);
            // The three groups #53 split out of the old `Features` bit. Each
            // draws from the streams it always drew from, so no seed names a
            // different person for this; what changed is that a creator who
            // has found a face can now roll a complexion without losing it.
            match category {
                Category::Head => reroll_face(&mut self.eyes, &mut self.face, &rolls),
                Category::Colouring => reroll_skin(&mut self.skin, &rolls),
                Category::Hair => reroll_hair(&mut self.hair, &rolls, &self.composites),
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

    /// Clamps every field into what the lexicon allows.
    ///
    /// Idempotent, like every `sanitize` in this crate: call it after reading
    /// a record from the network, where nothing about the contents can be
    /// assumed.
    ///
    /// **An invalid pointer is dropped, never repaired.** This is the
    /// one field consumers dereference into an AT-URI. Repairing is the
    /// wrong shape for a POINTER: stripping the offending characters from a
    /// record key yields a syntactically valid key that names some *other*
    /// record, which turns a malformed profile into a working reference to a
    /// document nobody chose. A profile whose pointer was invalid simply has
    /// no default avatar, exactly as if the optional field were absent.
    pub fn sanitize(&mut self) {
        if self
            .default_avatar
            .as_deref()
            .is_some_and(|key| !is_record_key(key))
        {
            self.default_avatar = None;
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

/// Whether `key` is a syntactically valid atproto record key.
///
/// The specification's rule, verbatim: one to 512 characters drawn from
/// `A-Z a-z 0-9 . _ : ~ -`, and not the two path-traversal spellings `.` and
/// `..`, which are reserved. `self` — the key this crate's own profile lives
/// under — is an ordinary word under this rule and needs no case.
///
/// The charset is the whole of the safety argument: every character in it is
/// unreserved in a URI path segment, so a key that passes cannot terminate,
/// escape or re-route the AT-URI a consumer builds around it.
fn is_record_key(key: &str) -> bool {
    (1..=512).contains(&key.len())
        && key != "."
        && key != ".."
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'~' | b'-'))
}

/// Draws a new face: the eyes, and what is carved around them.
///
/// **Shape only.** The complexion and the hair have locks of their own. The
/// argument for grouping all three — "a creator with one lock per slider ends
/// up with more locks than anyone reads" — answers the wrong question. The
/// right one is what somebody would keep on purpose, and a face is kept while
/// its colouring is rolled all the time.
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

    // Colour, on named streams like hair's, drawn on every roll whatever the
    // branches do (#203's rule) so adding it moved no other axis on any seed.
    // The square bias makes brown the population's home the way DARK_BIAS
    // does for hair; the periphery is the same pigment darkened — life's
    // irises run darker at the rim — and the ring is the periphery well down
    // in value, which is exactly what the old constant ring was to the old
    // constant iris (#229).
    let shade = rolls.range("eyes.shade", 0.0, 1.0).powi(2);
    let fade = rolls.range("eyes.fade", 0.10, 0.35);
    let centre = crate::face::eye::iris_pigment(shade);
    eyes.inner = centre;
    eyes.outer = centre.map(|channel| channel * (1.0 - fade));
    eyes.ring = eyes.outer.map(|channel| channel * 0.41);
}

/// Draws a fresh head of hair.
///
/// Its own category. Hair is the loudest thing about a head and the
/// one most often kept while everything under it changes, which is the whole
/// argument for a lock of its own.
///
/// **Five regions**, drawn from the streams their own names give
/// them. The composites keep their say — a beard is strongly sexed and a
/// hairline recedes with age.
fn reroll_hair(hair: &mut HairRecord, rolls: &Rolls, composites: &crate::Composites) {
    // Hair is a style, not a shape: it keeps its uniform draws and its
    // conservative ranges, for the same reason complexion does.
    let masculinity = -composites.femininity.clamp(-1.0, 1.0);
    let ageing = composites.ageing();

    // **The hairline is the one thing on a head that age and the frame axis
    // both have a claim on** (#169). Recession is age-related and strongly
    // masculine, so the composites shift where the uniform window SITS rather
    // than adding a term anywhere. At the top of the age ramp on a fully
    // masculine frame the window is 0.55 lower than at neutral; on a fully
    // feminine one age moves it a third as far, which is the direction and the
    // ratio the pattern-baldness literature reports rather than a measured
    // pair.
    //
    // Provenance: **looked up for the direction and the sex ratio, sized to
    // the axis** (#169).
    let recession = ageing * (0.4 + 0.15 * masculinity);
    hair.regions.scalp.line = rolls.range("hair.line", -0.8 - recession, 0.8 - recession);
    // The temples go back on their own, which is what recession IS — see the
    // axis's own docstring for why one number cannot draw both.
    hair.regions.scalp.temples =
        (rolls.range("hair.temples", 0.0, 0.5) + recession).clamp(0.0, 1.0);
    hair.regions.scalp.nape = rolls.range("hair.nape", -1.0, 1.0);

    let (roots, tips) = rolled_colour(rolls, ageing);

    // **The scalp draws its style from the catalogue, and draws it UNCOUPLED**
    // (#203, owner call). The two couplings this crate does make — a hairline
    // that recedes with age and masculinity, and facial hair gated the same way
    // — are both anatomy. A haircut is not: which style a person wears is
    // culture, and an engine whose deliverable is a BODY has no business
    // encoding an expectation about who wears their hair long. The editor is
    // where a deliberate look is set, and a re-roll's job here is variety.
    //
    // Weighted toward the short end anyway, because a crop is what most heads
    // in most references are and a population of curtains reads as a costume
    // department.
    //
    // Provenance: **owner call** for the absence of coupling, **sized by eye**
    // against the render sheet of a rolled population.
    hair.scalp.style = match rolls.pick("hair.style", &[35.0, 20.0, 20.0, 15.0, 10.0]) {
        1 => ScalpStyle::Bob {
            fringe: rolls.range("hair.fringe", 0.0, 1.0),
        },
        2 => ScalpStyle::Long {
            weight: rolls.range("hair.weight", 0.0, 1.0),
        },
        3 => ScalpStyle::TiedBack {
            tail: rolls.range("hair.tail", 0.0, 1.0),
        },
        4 => ScalpStyle::Curly {
            curl: rolls.range("hair.curl", 0.2, 1.0),
        },
        _ => ScalpStyle::Crop,
    };
    hair.scalp.cut.length = rolls.range("hair.length", 0.0, 1.0);
    hair.scalp.cut.density = rolls.range("hair.density", 0.35, 1.0);
    hair.scalp.cut.thickness = rolls.range("hair.thickness", 0.2, 0.9);
    hair.scalp.cut.droop = rolls.range("hair.droop", 0.3, 0.9);
    hair.scalp.roots = roots;
    hair.scalp.tips = tips;
    // Painted under the grown layer, always: it is what stops a thin crop
    // reading as a bald head, and it costs nothing.
    hair.scalp.skin = crate::hair::Paint {
        density: 0.85,
        colour: roots,
    };

    // Brows are near-universal and nearly always grown, so they are drawn as a
    // shape rather than as a coin. Which of the two, though, is a coin — and it
    // is the one place besides the beard where the frame axis has a say on hair
    // that is not on the scalp, for the same reason: brow density answers to
    // androgens as facial hair does. It is a mild lean rather than a rule,
    // because plenty of masculine faces have fine brows and the axis is a
    // description of a body rather than a promise about one.
    hair.brows.style = if rolls.chance("brow.thick", f64::from(0.22 + 0.24 * masculinity.max(0.0)))
    {
        crate::hair::BrowStyle::Thick
    } else {
        crate::hair::BrowStyle::Natural
    };
    hair.brows.cut.density = rolls.range("brow.density", 0.5, 1.0);
    hair.brows.cut.thickness = rolls.range("brow.thickness", 0.3, 1.0);
    hair.brows.cut.length = rolls.range("brow.length", 0.4, 0.9);
    hair.brows.roots = roots;
    hair.brows.tips = roots;
    hair.brows.skin = crate::hair::Paint {
        density: 0.9,
        colour: roots,
    };

    // **Facial hair is gated on the composites, as the stubble coin was**
    // (#169's reading of it, carried to three regions). The coin and the amount
    // draw from separate streams, so changing how full a beard is cannot change
    // who has one.
    //
    // **A quarter at neutral, and it was a twentieth** (#203, owner call). #202
    // wrote `0.05 + 0.45 * masculinity.max(0)`, which is the coupling this
    // issue's brief asked for at a fifth of the rate it asked for — the brief
    // says "as the stubble coin was", and that coin was a flat 25%. Re-centred
    // rather than replaced: the coupling is what makes it a body axis, and the
    // level is what decides whether a rolled population has any facial hair in
    // it at all. At a twentieth, and with the fullness guard below taking
    // another quarter off, a beard turned up on one neutral body in
    // twenty-seven.
    //
    // Symmetric now, so a feminine frame is near zero rather than flat at the
    // neutral rate — which is what the old `max(0.0)` was silently doing to the
    // whole lower half of the axis.
    let bearded = (0.25 + 0.35 * masculinity).clamp(0.0, 1.0);
    let grows = rolls.chance("beard.grows", f64::from(bearded));
    // **Age's claim on facial hair is the FULLNESS, not the presence** — a
    // beard fills out through the twenties and a nineteen-year-old's is patchy
    // whether or not he has one. This is the brief's "gated on femininity/age"
    // read the way the anatomy actually runs, and it is drawn on the amount so
    // that the coin above stays a coin.
    //
    // It reads the raw age rather than `Composites::ageing`, which is the only
    // term in this file that does. That ramp is deliberately zero below
    // `plan::AGE_PIVOT` because it exists for the changes that come after
    // thirty; every year this term cares about is under it, so through the ramp
    // an eighteen-year-old and a thirty-year-old grow the same beard.
    let grown_in = ((f32::from(u16::try_from(composites.age).unwrap_or(u16::MAX)) - 18.0) / 12.0)
        .clamp(0.0, 1.0);
    let full = rolls.range("beard.full", 0.0, 1.0) * (0.55 + 0.45 * grown_in);
    let coarse = rolls.range("beard.thickness", 0.3, 0.9);
    for cut in [
        &mut hair.moustache.cut,
        &mut hair.chin.cut,
        &mut hair.flanks.cut,
    ] {
        cut.density = full;
        cut.length = full;
        cut.thickness = coarse;
    }
    let beard = crate::hair::Paint {
        // Painted even where nothing is grown: that is what a shaved jaw is.
        density: (0.25 + 0.75 * full) * f32::from(u8::from(masculinity > -0.4)),
        colour: roots,
    };
    for tress in [
        &mut hair.moustache.skin,
        &mut hair.chin.skin,
        &mut hair.flanks.skin,
    ] {
        *tress = beard;
    }
    for (start, end) in [
        (&mut hair.moustache.roots, &mut hair.moustache.tips),
        (&mut hair.chin.roots, &mut hair.chin.tips),
        (&mut hair.flanks.roots, &mut hair.flanks.tips),
    ] {
        // A beard is short enough that its tips are the same age as its roots,
        // so it takes one colour rather than a fade.
        *start = roots;
        *end = roots;
    }
    let (moustache, chin, flanks) = if grows && full > 0.25 {
        rolled_beard(rolls)
    } else {
        (
            crate::hair::MoustacheStyle::None,
            crate::hair::ChinStyle::None,
            crate::hair::FlankStyle::None,
        )
    };
    hair.moustache.style = moustache;
    hair.chin.style = chin;
    hair.flanks.style = flanks;
}

/// How far toward the dark end the natural shade draw is pulled.
///
/// **Real hair is overwhelmingly dark and a uniform draw is not a population.**
/// A uniform shade puts as many blondes on the street as black-haired people,
/// which nowhere on earth has ever looked like. Raising a unit draw to this
/// power lands about half the population in the darkest fifth of the ramp and
/// about a sixth past its middle.
///
/// Deliberately kinder to the light end than any real population is: globally,
/// blonde and red together are a couple of percent, and a character generator
/// that produced one blonde in fifty rolls would read as broken rather than as
/// accurate. The ramp reddens through its middle on the way, so red is drawn by
/// the same number.
///
/// Provenance: **looked up** for the direction and the ordering, **sized by
/// eye** against a sheet of rolled heads.
const DARK_BIAS: f32 = 2.2;

/// How often a rolled head is dyed rather than grown.
///
/// One in fifty. Grey and fantasy come free from the record's sRGB pairs, and
/// DELIBERATE fantasy belongs in a creator's colour pickers; what a roll owes
/// is the occasional surprise, not a parade. At one in fifty a creator flipping through re-rolls meets one every
/// page or two, which is what "rare" has to mean to be worth having at all.
///
/// Provenance: **owner brief** ("rare outlier chance for the fantasy end"),
/// **sized by eye**.
const DYED: f64 = 0.02;

/// The two colours one rolled head of hair fades between.
///
/// Every draw here happens on every roll whatever the branches do, which is not
/// an accident and is not free-standing tidiness: [`Rolls`] keys a stream on the
/// axis's own NAME, so a draw that is skipped costs nothing and a draw that is
/// added moves no other axis on any seed. What that buys is that greying and
/// dyeing could be added at all without re-rolling every stored record's
/// haircut.
fn rolled_colour(rolls: &Rolls, ageing: f32) -> ([f32; 3], [f32; 3]) {
    let shade = rolls.range("hair.shade", 0.0, 1.0).powf(DARK_BIAS);
    // Tips are the same hair further from the scalp: lighter, never a different
    // colour. A random second colour reads as costume. The fade is drawn rather
    // than fixed because how far a head lightens toward its ends is a thing
    // heads differ in, and at a fixed step every rolled head had the same one.
    let fade = rolls.range("hair.fade", 0.05, 0.26);
    let (roots, tips) = (
        crate::hair::style::melanin(shade),
        crate::hair::style::melanin((shade + fade).min(1.0)),
    );

    // **Greying is age-coupled, and it is the one hair axis that could not
    // exist before the free colours did** (#169 left it open; #203 closes it).
    // The melanin ramp's light end is a warm blonde — pale hair WITH pigment —
    // and grey is hair with the pigment gone, so no point on that line could
    // ever have said it.
    //
    // How fast a head greys is its own draw rather than a function of age
    // alone: people at fifty run from black to white, and a rolled population
    // where everybody the same age has the same amount of grey reads as a
    // uniform. The multiplier reaches past one so that the top of the age ramp
    // can be fully white for some bodies and merely salted for others.
    //
    // **And it takes the SQUARE ROOT of the shared age ramp, which nothing else
    // here does.** `Composites::ageing` is quadratic on purpose: it is built for
    // the changes that accelerate late, and the recession above it is one of
    // them. Greying is the opposite kind of age marker — it is among the first
    // to show and it shows gradually, typically from the thirties. Read through
    // the raw ramp, a rolled forty-five-year-old was at most a tenth grey and a
    // population of them had no grey hair in it at all; through the root, that
    // body reaches a third and a sixty-year-old is mostly there, which is what
    // a room full of them looks like.
    //
    // It is still ZERO below `plan::AGE_PIVOT`, because the root of nothing is
    // nothing — the coupling is real rather than merely correlated, and nobody
    // rolls grey at twenty-two.
    let grey = (ageing.sqrt() * rolls.range("hair.greying", 0.15, 1.6)).clamp(0.0, 1.0);
    let (roots, tips) = (
        crate::hair::style::greyed(roots, grey),
        crate::hair::style::greyed(tips, grey),
    );

    if rolls.chance("hair.dyed", DYED) {
        // A dye covers what is under it, grey included — which is most of why
        // people dye their hair, and why this replaces the pair rather than
        // mixing into it.
        let hue = rolls.range("hair.hue", 0.0, 1.0);
        let vivid = rolls.range("hair.vivid", 0.30, 0.85);
        return (
            crate::hair::style::dyed(hue, vivid),
            crate::hair::style::dyed(hue, (vivid + 0.12).min(1.0)),
        );
    }
    (roots, tips)
}

/// What a bearded face grows, as a coherent set rather than three coins.
///
/// **Three independent draws are not three regions of a beard.** The moustache,
/// the chin and the flanks are one thing on a face, and rolling each on its own
/// produces the combinations nobody wears far more often than the ones everybody
/// does — a chin and flanks with a bare lip, most of the time. So a named
/// configuration is drawn first and the regions follow from it, which is the
/// same reason the scalp draws a style rather than five separate parameters.
///
/// The variant axes inside a configuration are still drawn independently, since
/// how far a handlebar sweeps says nothing about how pointed a chin is.
///
/// Provenance: **derived** from what a beard is, **weighted by eye**.
fn rolled_beard(
    rolls: &Rolls,
) -> (
    crate::hair::MoustacheStyle,
    crate::hair::ChinStyle,
    crate::hair::FlankStyle,
) {
    use crate::hair::{ChinStyle, FlankStyle, MoustacheStyle};
    let moustache = match rolls.pick("beard.lip", &[60.0, 25.0, 15.0]) {
        1 => MoustacheStyle::Handlebar {
            sweep: rolls.range("beard.sweep", 0.0, 1.0),
        },
        2 => MoustacheStyle::Pencil {
            ride: rolls.range("beard.ride", 0.0, 1.0),
        },
        _ => MoustacheStyle::Chevron,
    };
    let goatee = ChinStyle::Goatee {
        point: rolls.range("beard.point", 0.0, 1.0),
    };
    // A braid is the show-off style and it belongs to a full beard: there is
    // nothing to gather on a chin that only grows a tuft.
    let heavy = if rolls.chance("beard.braided", 0.12) {
        ChinStyle::Braided {
            twist: rolls.range("beard.twist", 0.0, 1.0),
        }
    } else {
        ChinStyle::Full
    };
    let connected = FlankStyle::FullConnect {
        reach: rolls.range("beard.reach", 0.0, 1.0),
    };
    let sideburns = FlankStyle::Sideburns {
        drop: rolls.range("beard.drop", 0.0, 1.0),
    };
    match rolls.pick("beard.shape", &[34.0, 18.0, 16.0, 12.0, 12.0, 8.0]) {
        // A goatee: the lip and the chin, and a clean cheek.
        1 => (moustache, goatee, FlankStyle::None),
        // The lip on its own.
        2 => (moustache, ChinStyle::None, FlankStyle::None),
        // The chin on its own, which is the goatee without its moustache.
        3 => (MoustacheStyle::None, goatee, FlankStyle::None),
        // Sideburns and nothing else.
        4 => (MoustacheStyle::None, ChinStyle::None, sideburns),
        // Chops: the flanks carried forward with a bare chin under them.
        5 => (moustache, ChinStyle::None, connected),
        // The whole face, which is the commonest beard there is.
        _ => (moustache, heavy, connected),
    }
}

fn reroll_skin(skin: &mut SkinParams, rolls: &Rolls) {
    // Complexion stays UNIFORM while the shape axes went Gaussian (#160, an
    // owner call worth restating): a Gaussian centred on the default would
    // make every complexion far from it a "rare extreme", and a skin tone is
    // not an extreme of anything.
    skin.melanin = rolls.range("skin.melanin", 0.0, 1.0);
    skin.undertone = rolls.range("skin.undertone", -1.0, 1.0);
    skin.blush = rolls.range("skin.blush", 0.15, 0.8);
    // Most people have none, so it stays off more often than not. The coin and
    // the amount draw from separate streams, so changing how many freckles a
    // freckled face has cannot change which faces have any.
    //
    // **Stubble used to be drawn here too and is not a complexion axis any
    // more** (#212). The painted hair layer replaced what it drew at #200 and
    // the record grew a density and a colour per region at #202; the axis was
    // left behind, still written, still rolled, and read by nothing.
    skin.freckles = if rolls.chance("skin.freckled", 0.3) {
        rolls.range("skin.freckles", 0.2, 1.0)
    } else {
        0.0
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hair::Style as _;
    use crate::plan::HumanoidParams;

    #[test]
    fn a_new_record_is_sane_and_tiny() {
        let record = AvatarRecord::new("Ari", Archetype::default());
        assert_eq!(record.name, "Ari");
        assert!(record.fits_budget());
        // The bound is a ratchet on a record that should stay small, not a
        // budget — [`RECORD_BUDGET_BYTES`] is that, and it is still sixty times
        // further away. Raised 700 -> 800 when the composites block landed
        // (#162), and 800 -> 1900 when hair became five regions in two layers
        // (#202): measured, a fresh record went 745 -> 1720 bytes.
        //
        // **That is the largest single jump this record has taken, and it is
        // what the owner's model costs.** Five regions each carry a style, four
        // cut axes, two sRGB colours and a painted colour and density — about
        // 190 bytes apiece — against eight scalars and one colour for the whole
        // head before. The alternative was fewer colours, which is the one
        // thing the model is for.
        //
        // Report the size when it fires, because the question a reader has is
        // "by how much".
        let size = record.serialized_size().expect("serialises");
        assert!(
            size < 1900,
            "a fresh record is {size} bytes; a body is a couple of kilobytes, not tens"
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
        assert_ne!(
            second.shoulder_width, first.shoulder_width,
            "an unlocked shape axis changed"
        );
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
        assert_eq!(free.shoulder_width, locked.shoulder_width);
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
    fn a_stored_seed_reproduces_its_person() {
        // The promise behind every stored seed, pinned to concrete draws. The
        // determinism test above proves two rolls in one process agree; this
        // one catches the drift that test cannot see: a `rand`/`rand_pcg`
        // upgrade or an edited distribution moves these values on every seed
        // at once, silently, and every stored record's look with them. If
        // this fails and the change was deliberate, bump `GENERATOR_VERSION`
        // and re-pin; if it was not, the dependency or edit that moved it is
        // the bug.
        //
        // Pinned axes only, never the whole record: the independence contract
        // means adding an axis moves none of these, so growth does not pay a
        // re-pinning tax here.
        let record = AvatarRecord::rolled("Pinned", Archetype::default(), 42);
        assert_eq!(record.generator, GENERATOR_VERSION);
        assert_eq!(record.seed, 42);
        let Archetype::Humanoid(params) = &record.archetype else {
            panic!("archetype changed");
        };
        let drawn = format!(
            "femininity {:.6} mass {:.6} fat {:.6} age {} height {:.6} melanin {:.6}",
            record.composites.femininity,
            record.composites.mass,
            record.composites.body_fat,
            record.composites.age,
            params.height,
            record.skin.melanin,
        );
        assert_eq!(
            drawn,
            "femininity 1.340000 mass -0.298000 fat 0.212000 age 30 height 1.482000 melanin 0.933000"
        );
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
        assert_eq!(
            params.shoulder_width, 0.0,
            "unspecified axes take their defaults"
        );
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
        // The hair record's own default survives a partial read the same way,
        // now that it is five regions rather than eight scalars.
        assert_eq!(
            record.hair.scalp.cut.density,
            HairRecord::default().scalp.cut.density
        );
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

    /// One rolled population, at a frame and an age the caller names.
    ///
    /// **A prior is a statement about a POPULATION and cannot be tested on a
    /// body.** Every claim the priors make — dark-dominant, greying with
    /// age, a quarter bearded at neutral — is false of some individual seed and
    /// has to be, or it would not be a distribution. So the tests below roll
    /// several hundred and count.
    ///
    /// Six hundred is enough that a rate of a quarter lands within about three
    /// points of itself at two sigma, which is what the bounds are set from; the
    /// bounds are still written wide, because a test that fails when a
    /// well-behaved change moves a rate by two points is a test nobody will keep.
    fn population(count: i64, femininity: f32, age: u32) -> Vec<AvatarRecord> {
        (0..count)
            .map(|seed| {
                let mut record = AvatarRecord::new("Rolled", Archetype::default());
                record.reroll(seed);
                record.composites.femininity = femininity;
                record.composites.age = age;
                // Re-rolled AFTER the composites are set, because every hair
                // prior that couples to a body reads them at draw time — the
                // first cut of these tests set them afterwards and measured a
                // population of neutral thirty-year-olds five times over.
                record.locks = LockSet::NONE;
                reroll_hair(&mut record.hair, &Rolls::new(seed), &record.composites);
                record
            })
            .collect()
    }

    #[test]
    fn a_rolled_population_wears_the_whole_scalp_catalogue() {
        // The scalp was `Crop` on every rolled body from the catalogue landing
        // at #204 until this issue: five styles shipped and a re-roll could
        // reach one of them.
        let mut worn = [0usize; 5];
        for record in population(600, 0.0, 30) {
            let slot = match record.hair.scalp.style {
                ScalpStyle::Crop => 0,
                ScalpStyle::Bob { .. } => 1,
                ScalpStyle::Long { .. } => 2,
                ScalpStyle::TiedBack { .. } => 3,
                ScalpStyle::Curly { .. } => 4,
                ScalpStyle::None => panic!("a re-roll shaved a head"),
            };
            worn[slot] += 1;
        }
        for (slot, count) in worn.iter().enumerate() {
            assert!(
                *count > 20,
                "style {slot} turned up {count} times in 600 rolls: {worn:?}"
            );
        }
        // And a crop is the commonest, which is the one thing the weighting
        // says out loud.
        assert!(
            worn[0] == *worn.iter().max().expect("five styles"),
            "a crop is no longer the commonest rolled style: {worn:?}"
        );
    }

    #[test]
    fn a_rolled_style_carries_a_rolled_axis() {
        // Every parametric variant in every catalogue has an axis, all of them
        // were sanitized and quantised from the day they shipped, and until this
        // issue not one was ever drawn. A style picked from a catalogue with its
        // axis left at a default is five silhouettes rather than a catalogue.
        let mut seen: Vec<i32> = Vec::new();
        for record in population(400, 0.0, 30) {
            let axis = match record.hair.scalp.style {
                ScalpStyle::Bob { fringe } => fringe,
                ScalpStyle::Long { weight } => weight,
                ScalpStyle::TiedBack { tail } => tail,
                ScalpStyle::Curly { curl } => curl,
                ScalpStyle::Crop | ScalpStyle::None => continue,
            };
            seen.push((axis * 1000.0).round() as i32);
        }
        seen.sort_unstable();
        seen.dedup();
        assert!(
            seen.len() > 100,
            "the rolled style axes took only {} distinct values",
            seen.len()
        );
    }

    #[test]
    fn rolled_hair_is_mostly_dark() {
        // A uniform shade puts as many blondes on the street as black-haired
        // people, which is what the draw did until this issue and what nowhere
        // on earth has ever looked like. Measured on the ROOT colour's own
        // luminance rather than on the shade that drew it, because the ramp is
        // not linear in shade and the claim is about what a head looks like.
        let mut dark = 0;
        let mut pale = 0;
        for record in population(600, 0.0, 30) {
            let [r, g, b] = record.hair.scalp.roots;
            let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            if luminance < 0.10 {
                dark += 1;
            }
            if luminance > 0.45 {
                pale += 1;
            }
        }
        assert!(
            dark > 300,
            "only {dark} of 600 rolled heads are dark-haired"
        );
        // And not so dark that the light end never turns up: a generator that
        // produced one blonde in fifty rolls would read as broken rather than
        // as accurate, which is the deliberate departure from a real
        // population `DARK_BIAS` documents.
        assert!(
            pale > 15,
            "only {pale} of 600 rolled heads are fair-haired, which is fewer \
             than DARK_BIAS claims"
        );
    }

    #[test]
    fn hair_greys_with_age_and_bodies_grey_at_their_own_rates() {
        // Greying is the one hair axis that could not exist before the record
        // carried free colours (#169 left it open): the melanin ramp's light end
        // is a warm blonde, which is pale hair WITH pigment in it.
        //
        // Measured as how far a head's own colour has moved toward the white
        // `greyed` mixes in, rather than as lightness — a natural blonde is
        // light and is not grey, and the first cut of this test could not tell
        // the two apart.
        let greyness = |record: &AvatarRecord| {
            let [r, g, b] = record.hair.scalp.roots;
            // Grey is the neutral axis: a head that has gone white is bright
            // AND unsaturated, and a blonde is bright and warm.
            let high = r.max(g).max(b);
            let low = r.min(g).min(b);
            let saturation = if high > 0.0 { (high - low) / high } else { 0.0 };
            high * (1.0 - saturation)
        };
        let young: Vec<f32> = population(200, 0.0, 22).iter().map(greyness).collect();
        let old: Vec<f32> = population(200, 0.0, 78).iter().map(greyness).collect();
        let mean = |all: &[f32]| all.iter().sum::<f32>() / all.len() as f32;
        assert!(
            mean(&old) > mean(&young) * 2.0,
            "an old population is {:.3} grey against a young one's {:.3}",
            mean(&old),
            mean(&young)
        );
        // Nobody is grey at twenty-two, which is the half of the claim a mean
        // alone cannot make: `ageing` is zero below the pivot, so this is the
        // coupling being real rather than merely correlated.
        assert!(
            young.iter().all(|grey| *grey < 0.62),
            "somebody went grey at twenty-two"
        );
        // And an old population is not uniformly grey, which is what a draw
        // that used age alone would give: people at seventy-eight run from
        // salted to white.
        let spread =
            old.iter().cloned().fold(0.0f32, f32::max) - old.iter().cloned().fold(1.0f32, f32::min);
        assert!(
            spread > 0.3,
            "every seventy-eight-year-old greyed by the same amount ({spread:.3} spread)"
        );
    }

    #[test]
    fn one_rolled_head_in_fifty_is_dyed() {
        // Rare, and not never. The epic's decision is that DELIBERATE fantasy
        // lives in the editor's colour pickers and that a roll owes the
        // occasional surprise; a rate that rounds to zero would mean the
        // surprise never arrives, and this test would pass on a branch that had
        // been deleted.
        //
        // A dye is recognised by being a colour the melanin ramp cannot make:
        // the ramp is warm all the way along, so red is never below green.
        let dyed = population(600, 0.0, 30)
            .iter()
            .filter(|record| {
                let [r, g, b] = record.hair.scalp.roots;
                r < g || b > g
            })
            .count();
        assert!(
            (4..40).contains(&dyed),
            "{dyed} of 600 rolled heads are dyed, against a one-in-fifty rate"
        );
    }

    #[test]
    fn facial_hair_follows_the_frame_axis_and_is_symmetric_about_it() {
        // **Both halves of #203's owner call.** The rate moved from a twentieth
        // at neutral to a quarter, and the coupling became symmetric — #202's
        // `masculinity.max(0.0)` gave the whole feminine half of the axis the
        // same rate as a neutral frame, so a fully feminine body was as likely
        // to grow a beard as an androgynous one.
        let bearded = |femininity: f32| {
            population(400, femininity, 35)
                .iter()
                .filter(|record| {
                    record.hair.chin.style.grows()
                        || record.hair.flanks.style.grows()
                        || record.hair.moustache.style.grows()
                })
                .count()
        };
        let (feminine, neutral, masculine) = (bearded(1.0), bearded(0.0), bearded(-1.0));
        assert!(
            feminine < neutral / 3,
            "a feminine frame grew {feminine} beards in 400 against a neutral \
             frame's {neutral}"
        );
        assert!(
            (50..110).contains(&neutral),
            "{neutral} of 400 neutral bodies grew facial hair; the gate is a \
             quarter and the fullness guard takes about another quarter off"
        );
        assert!(
            masculine > neutral * 2,
            "a masculine frame grew {masculine} beards against a neutral \
             frame's {neutral}"
        );
    }

    #[test]
    fn a_young_beard_is_thinner_than_a_grown_one() {
        // The age half of this issue's gate, and it is on the FULLNESS rather
        // than on the coin: a beard fills out through the twenties, and a
        // nineteen-year-old's is patchy whether or not he has one.
        //
        // It is the one term in `reroll_hair` that reads the raw age instead of
        // `Composites::ageing`, because that ramp is zero below `AGE_PIVOT` and
        // every year this cares about is under it — so this test would pass on a
        // term that did nothing if it compared thirty against fifty.
        let fullness = |age: u32| {
            let all = population(300, -1.0, age);
            all.iter()
                .map(|record| record.hair.chin.cut.density)
                .sum::<f32>()
                / all.len() as f32
        };
        let (young, grown) = (fullness(18), fullness(35));
        assert!(
            grown > young * 1.5,
            "a beard at eighteen is {young:.3} full against thirty-five's {grown:.3}"
        );
        // And fewer of them clear the threshold at which anything is grown at
        // all, which is what the thinner draw does to the population.
        let bearded = |age: u32| {
            population(300, -1.0, age)
                .iter()
                .filter(|record| record.hair.chin.style.grows() || record.hair.flanks.style.grows())
                .count()
        };
        assert!(
            bearded(18) < bearded(35),
            "as many eighteen-year-olds grew a beard as thirty-five-year-olds: \
             {} against {}",
            bearded(18),
            bearded(35)
        );
    }

    #[test]
    fn a_bearded_face_grows_a_beard_somebody_would_wear() {
        // Three independent coins produce the combinations nobody wears far
        // more often than the ones everybody does. A configuration is drawn
        // instead, so what has to hold is that every bearded face is one of
        // them — and in particular that nothing grows a chin and a pair of
        // flanks with a bare lip between them, which is the one combination
        // that reads as a mistake rather than as a choice.
        let mut bearded = 0;
        for record in population(600, -1.0, 35) {
            let (lip, chin, flanks) = (
                record.hair.moustache.style.grows(),
                record.hair.chin.style.grows(),
                record.hair.flanks.style.grows(),
            );
            if !(lip || chin || flanks) {
                continue;
            }
            bearded += 1;
            assert!(
                !(chin && flanks && !lip),
                "a face grew a full beard with a shaved upper lip"
            );
        }
        assert!(
            bearded > 100,
            "only {bearded} of 600 masculine bodies are bearded"
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
        // **Carries a current `generator`, and has to.** A record without one
        // predates every generation, so `sanitize` re-draws its hair from its
        // own seed (see `AvatarRecord::migrate`) — which would replace the wild
        // value this test exists to watch being clamped.
        let json = r#"{"name":"Wide","generator":4,"skin":{"melanin":3000000000},
            "hair":{"scalp":{"cut":{"length":4000000000}}}}"#;
        let mut record: AvatarRecord = serde_json::from_str(json).expect("loads a wild value");
        record.sanitize();
        assert_eq!(record.skin.melanin, 1.0);
        assert_eq!(record.hair.scalp.cut.length, 1.0);
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
            GENERATOR_VERSION, 5,
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
                (params.shoulder_width * 1000.0).round() as i32,
                (record.skin.melanin * 1000.0).round() as i32,
                (record.hair.scalp.cut.length * 1000.0).round() as i32,
            )
        };
        // Generation 3 (#169): the two-tier draw. Both body columns moved and
        // both moved for a reason this table can show.
        //
        // `height` came in on all three seeds — 2353 → 1799, 1012 → 1482,
        // 2186 → 1902 — and both halves of that are generation 3: the centre
        // follows the frame axis now rather than sitting on a bare 1.75, and
        // `STATURE_SIGMA` narrowed the draw from half a metre to 0.12, which
        // is why all three land inside a human range where two of them were
        // 2.35 m and 1.01 m. `shoulder_width` came in much harder,
        // −40 → −13 and 147 → 49 and 258 → 86, which is very nearly the third
        // `OFFSET_SIGMA` names: the axis is an offset on what `femininity` and
        // `mass` derive, and it is drawn as one.
        //
        // **Melanin and hair length are IDENTICAL to generation 2's table on
        // all three seeds, which is the other half of the contract** and is
        // the third generation running that it has held: a stream is keyed by
        // its axis's name, so re-ordering the draw, re-centring it, and
        // retiring `humanoid.build` and `humanoid.muscle` for good cannot
        // move what an unrelated axis rolls. Colouring and hair keep their
        // uniform draws and reproduce across the bump.
        //
        // Generation 5 (#203) is the hair priors, and this table is the
        // cheapest possible statement of what that did and did not touch: the
        // hair-length column is `hair.length`, whose stream name and range are
        // both unchanged, so all three seeds hold it to the digit while the
        // scalp STYLE above it went from `Crop` on every body to a draw from
        // the catalogue, the shade went dark-dominant, and greying arrived.
        // Four new axes and a changed distribution on two old ones, and the one
        // axis that did not move did not move.
        //
        // If a future change to hair moves this column, that is not a bug in
        // this table — it is the table saying that whatever moved was not
        // confined to the axes it was supposed to be.
        //
        // Generation 2 (#160) was the exploration distributions; generation 1
        // the first numbered draw.
        assert_eq!(quantised(1), (1799, -13, 594, 238));
        assert_eq!(quantised(42), (1482, 49, 933, 973));
        assert_eq!(quantised(-7), (1902, 86, 513, 903));
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
    fn a_profile_pointer_that_is_not_a_record_key_is_dropped_not_repaired() {
        // **#51.** `defaultAvatar` is the one field consumers dereference into
        // an AT-URI and was the one field with no sanitization. The rule is
        // the record-key spec's own; the design decision under test is that an
        // invalid pointer is DROPPED — repairing one by stripping characters
        // yields a valid key that names some other record, which is worse than
        // no default at all.
        let profile = |key: &str| ProfileRecord {
            default_avatar: Some(key.into()),
            created_at: Some("2026-08-16T00:00:00Z".into()),
        };

        // Keys the spec allows, including the odd-looking ones: every
        // character class, `self` (this record's own key), and a 512-char key
        // at the length limit exactly.
        let longest = "a".repeat(512);
        for key in [
            "3lm2k4x",
            "self",
            "a.b:c~d_e-f",
            "A-Za-z0-9",
            longest.as_str(),
        ] {
            let mut kept = profile(key);
            kept.sanitize();
            assert_eq!(
                kept.default_avatar.as_deref(),
                Some(key),
                "{key:?} is a valid record key and must survive"
            );
        }

        // The hostile and the malformed: URI metacharacters that would
        // terminate or re-route the AT-URI built around the key, the two
        // reserved traversal spellings, the empty string, and one past the
        // length limit.
        let overlong = "a".repeat(513);
        for key in [
            "",
            ".",
            "..",
            "a/b",
            "a?b=c",
            "a#b",
            "a b",
            "a%2Fb",
            "key\u{9}tab",
            "\u{fc}mlaut",
            overlong.as_str(),
        ] {
            let mut dropped = profile(key);
            dropped.sanitize();
            assert_eq!(
                dropped.default_avatar, None,
                "{key:?} is not a record key and the pointer must be dropped"
            );
        }

        // Idempotent, like every sanitize in the crate: a second pass changes
        // nothing, on a kept pointer and on a dropped one.
        let mut twice = profile("3lm2k4x");
        twice.sanitize();
        let once = twice.clone();
        twice.sanitize();
        assert_eq!(twice, once);

        // And the lexicon says the same thing the code does, in the spec's own
        // vocabulary: the field is format record-key. A rule enforced in code
        // and absent from the schema is one every other consumer of the
        // lexicon re-discovers by incident (#211's three-directions lesson,
        // pointed the other way).
        let lexicon: serde_json::Value = serde_json::from_str(include_str!(
            "../../lexicons/network/symbios/avatar/profile.json"
        ))
        .expect("the profile lexicon parses");
        assert_eq!(
            lexicon["defs"]["main"]["record"]["properties"]["defaultAvatar"]["format"],
            "record-key",
            "the lexicon must name the constraint the code enforces"
        );
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
