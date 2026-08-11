//! Body plans: the parameters a record carries, and the skeletons they build.
//!
//! An [`Archetype`] is an open union of per-variant parameter structs, each
//! implementing [`BodyPlan`]. Adding a body plan means adding one variant and
//! one module, not editing a chain of `match` ladders scattered through the
//! crate.
//!
//! ## Semantic axes, not principal components
//!
//! Every axis here is hand-designed and named for what it means — `build`,
//! `shoulder_width`, `limb_length` — and each drives *several* skeleton
//! quantities at once. That is deliberate. Fitting a shape space to scan data
//! yields principal components nobody can steer by hand; authored axes with
//! their correlations written down are what make a slider feel like a slider.
//!
//! ## Meshability is an invariant, not a hope
//!
//! [`crate::cage::build_cage`] rejects joints whose sockets cannot clear each
//! other, and those limits are *geometric*: a chest of radius `r` needs its
//! clavicle to reach roughly `1.5 r` before an arm can attach. Rather than let a
//! slider wander into that wall, the derived quantities here are expressed as
//! multiples of the joint radius they must clear. The parameter sweep in
//! `tests/plan.rs` walks the whole space and asserts every combination meshes.

mod composites;
pub(crate) mod derive;
mod humanoid;
mod quadruped;
mod zone;

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use crate::skeleton::Skeleton;

pub use composites::{
    AGE_PIVOT, AGE_RANGE, BODY_FAT_RANGE, Composites, DEFAULT_AGE, DEFAULT_BODY_FAT,
};
pub use humanoid::HumanoidParams;
pub use quadruped::QuadrupedParams;
pub use zone::{Limb, ZONE_COUNT, Zone, ZoneSet};

/// Smallest and largest biped stature accepted, in metres.
#[must_use]
pub fn humanoid_height_range() -> (f32, f32) {
    humanoid::HEIGHT_RANGE
}

/// Smallest and largest quadruped back height accepted, in metres.
#[must_use]
pub fn quadruped_height_range() -> (f32, f32) {
    quadruped::HEIGHT_RANGE
}

/// A group of axes that can be locked or re-rolled together.
///
/// Categories are what a creator's lock toggles act on: re-rolling with
/// `Category::Build` locked keeps the body's mass while everything else
/// changes. Each category draws from its own seed stream, so locking one group
/// never reshuffles another.
///
/// ## What a category is for
///
/// **A lock answers "what do I want to keep while I roll again", and the
/// grouping is only right if it can answer that** (#53). The set below was four
/// body groups and one called `Features` that meant head size, extremity size,
/// skull breadth, face length, skin, eyes, face and hair all at once — eight
/// kinds of thing, including the two that most decide whether two seeds read as
/// two people. Somebody who had found a skull they liked and wanted a different
/// complexion could not say so.
///
/// So `Features` is split into [`Category::Head`], [`Category::Colouring`] and
/// [`Category::Hair`], extremity size joins the proportions it belongs with,
/// and the composite that fits nowhere else gets its own bit. The line drawn:
/// **a category is a thing a creator would keep on purpose**, which is why
/// colouring and hair are separate from the shape they sit on, and why one
/// axis on its own can still earn a bit if nothing else is like it.
///
/// ## This renumbered nothing, and narrowed one bit
///
/// Bits 0 to 3 mean exactly what they meant. Bit 4 was `Features` and is now
/// the narrower [`Category::Head`]; bits 5, 6 and 7 are new. A stored lock set
/// that set bit 4 meant to hold its complexion and hair as well, and against
/// this build it will not — **the one break here, taken while the lexicon is
/// unpublished and deliberately, which is what "before the bitmask has a
/// reader" meant.** Unknown bits still round-trip untouched, so the reverse
/// direction — a newer client's locks surviving an older one — holds as it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// Overall size — how tall or long the body is.
    Stature,
    /// Mass and how it is carried: the `mass` and `bodyFat` composites, and the
    /// `build` and `muscle` axes they are replacing.
    Build,
    /// Skeletal frame: the `femininity` composite, shoulder and hip width.
    Frame,
    /// Relative lengths of limbs, neck and tail, and the size of hands and feet.
    ///
    /// Extremity size joined this from `Features` (#53): a hand is a
    /// proportion of the arm it ends, and nobody locks a face to hold a hand.
    Proportions,
    /// The shape of the head: its size, breadth and face length, the eyes, and
    /// the nose, brow, mouth and ears carved into it.
    ///
    /// Shape only. What colour any of it is belongs to [`Category::Colouring`],
    /// and what grows on top belongs to [`Category::Hair`] — the split this
    /// category exists for.
    Head,
    /// Complexion: tone, undertone, blush, freckles and stubble.
    Colouring,
    /// Hair: its length, volume, hairline, part, wave, curl and shade.
    Hair,
    /// How old the body is.
    ///
    /// Its own bit because it belongs to no other group and reaches all of
    /// them — age moves the body, the skull and the skin together — so folding
    /// it into any one of them would make that lock a lie about the other two.
    Age,
}

impl Category {
    /// Every category, in creator-panel order.
    pub const ALL: [Category; 8] = [
        Category::Stature,
        Category::Build,
        Category::Frame,
        Category::Proportions,
        Category::Head,
        Category::Colouring,
        Category::Hair,
        Category::Age,
    ];

    /// The bit this category occupies in a lock set.
    #[must_use]
    pub fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

/// One re-roll's random draws, with an independent stream per named axis.
///
/// Drawing every axis in sequence from one stream is reproducible but not
/// *stable*: inserting an axis shifts every draw after it, so the same seed
/// yields a different person than it did before the insertion. A record keeps
/// its seed precisely so a look can be reproduced, and there is no way for a
/// reader to notice that the promise has been broken.
///
/// Keying each stream on the axis's own name removes the coupling entirely.
/// Adding, removing or reordering an axis changes that axis and nothing else,
/// which is what makes the seed worth storing. The cost is that axis names are
/// now part of the wire contract in the same way field names are: renaming one
/// re-rolls it.
///
/// Names are namespaced by the struct they belong to — `humanoid.height`,
/// `skin.melanin` — because two axes called `height` on different plans must not
/// share a stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rolls {
    seed: i64,
}

impl Rolls {
    /// The draws for one seed.
    #[must_use]
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    /// The stream one named axis draws from.
    ///
    /// FNV-1a over the name, mixed into the seed. Any decent hash would do; what
    /// matters is that it depends on the name and on nothing else about where
    /// the axis sits.
    #[must_use]
    pub fn stream(&self, axis: &str) -> Pcg64Mcg {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in axis.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Pcg64Mcg::seed_from_u64((self.seed as u64) ^ hash)
    }

    /// A value drawn uniformly from `low..=high`.
    #[must_use]
    pub fn range(&self, axis: &str, low: f32, high: f32) -> f32 {
        use rand::Rng;
        self.stream(axis).random_range(low..=high)
    }

    /// Whether a one-in-`probability` event happened.
    #[must_use]
    pub fn chance(&self, axis: &str, probability: f64) -> bool {
        use rand::Rng;
        self.stream(axis).random_bool(probability)
    }

    /// A shape axis drawn as a person first and an extreme rarely (#160).
    ///
    /// Replaces the uniform-inside-a-fence draw the shape axes used to make.
    /// That fence was a judgement — "a re-roll that reaches the bounds makes
    /// every third seed a caricature" — and this keeps the judgement while
    /// removing the wall: the bulk is a Gaussian centred on the axis's own
    /// **default** with `sigma` half the old fence's width, so a typical draw
    /// lands where it always did, and the tail is a `WILDCARD` chance of a
    /// uniform draw over the whole exploration `range`, because a clamped
    /// Gaussian at four sigma never lands and "rare" must not mean "never".
    ///
    /// Out-of-range bulk draws are **reflected** at the bounds rather than
    /// clamped. An axis whose default sits on its own range edge — `muscle`
    /// defaults to `0.0` at the bottom of `0..` — would otherwise put half its
    /// mass in an atom exactly on the edge; folding the distribution keeps the
    /// mass near the default without the spike.
    ///
    /// All draws come from the axis's own named stream, in a fixed order, so
    /// the independence contract [`Rolls`] documents survives: adding an axis
    /// moves no other axis on any seed. The *distribution* change is what
    /// `GENERATOR_VERSION` moves for.
    #[must_use]
    pub fn shape(&self, axis: &str, default: f32, sigma: f32, range: (f32, f32)) -> f32 {
        use rand::Rng;
        let mut stream = self.stream(axis);
        if stream.random_bool(WILDCARD) {
            return stream.random_range(range.0..=range.1);
        }
        // Box–Muller from two uniforms: exact, allocation-free, and stable —
        // this pair of draws is the wire contract for every stored seed.
        let (a, b): (f32, f32) = (
            stream.random_range(f32::EPSILON..=1.0),
            stream.random_range(0.0..1.0),
        );
        let normal = (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos();
        let mut value = default + sigma * normal;
        if value < range.0 {
            value = range.0 + (range.0 - value);
        }
        if value > range.1 {
            value = range.1 - (value - range.1);
        }
        value.clamp(range.0, range.1)
    }
}

/// How often [`Rolls::shape`] ignores the Gaussian and draws anywhere.
///
/// With about twenty shape axes on a body, one in thirty per axis means most
/// bodies carry no wild axis and every third or so carries one — exploration
/// without every seed being a caricature.
/// Provenance: **tuned by render** (#160), over reroll contact sheets.
const WILDCARD: f64 = 1.0 / 30.0;

/// How far past its conservative range an axis may be pushed, as a factor on
/// each bound's distance from the axis's own default.
///
/// The conservative ranges were chosen so no slider could distort a body; the
/// owner's call (#160) is that *exploration* should not be so limited, and the
/// records already carry the values (scaled i64, no format ceiling at ±1).
/// The stored unit's MEANING does not change — a stored `0.7` nose is the nose
/// it always was — the envelope just continues past the old ends. Extremes are
/// bounded by the one contract that cannot bend: a sanitised record must
/// always mesh, so any axis whose tripled end breaks the cage is pulled in at
/// its own `sanitize`, with the wall recorded beside it.
pub const EXPLORE: f32 = 3.0;

/// The exploration envelope of an axis: its conservative range, stretched
/// [`EXPLORE`]× about the axis's own default.
#[must_use]
pub fn explore_range(default: f32, conservative: (f32, f32)) -> (f32, f32) {
    (
        default + EXPLORE * (conservative.0 - default),
        default + EXPLORE * (conservative.1 - default),
    )
}

/// Behaviour every body plan provides.
pub trait BodyPlan: Sized {
    /// Clamps every axis into its valid range.
    ///
    /// Must be idempotent: sanitising twice equals sanitising once, and a
    /// sanitised record must always mesh. A non-finite axis — which the public
    /// Rust API can be handed, and which is what a creator UI produces when a
    /// slider is fed a division that blew up — takes that axis's documented
    /// default. Build each axis with `plan::sanitize_axis` and both hold by
    /// construction; do the clamping by hand and see #55.
    fn sanitize(&mut self);

    /// Builds the capsule graph this plan describes.
    ///
    /// The result is always meshable by [`crate::cage::build_cage`] for any
    /// sanitised parameters.
    fn skeleton(&self, composites: &Composites) -> Skeleton;

    /// Draws fresh values for the axes belonging to `category`.
    ///
    /// Each axis draws from its own named stream — see [`Rolls`] — so adding one
    /// never changes what the others produce for a given seed.
    ///
    /// **`composites` are already drawn when this is called, and that ordering
    /// is the whole of reroll v3** (#169). A plan's axes are OFFSETS on what
    /// the composites derive, so they are drawn second and drawn small; a plan
    /// that wants to correlate — stature with the frame axis, say — reads them
    /// here rather than rolling a quantity the composites have already decided.
    /// What may NOT move is which stream an axis draws from: correlation shifts
    /// a draw's mean and width, never its stream, so adding an axis still
    /// disturbs nothing.
    fn reroll(&mut self, category: Category, rolls: &Rolls, composites: &Composites);

    /// Appends this plan's quantised parameters to a share code.
    fn encode(&self, out: &mut Vec<u8>);

    /// Reads parameters back from a share code.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError::Truncated`] if the payload ends early.
    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError>;

    /// Reads parameters from a **version-3** share code (#160).
    ///
    /// Version 4 widened each byte's span to the exploration envelope; the
    /// old codes map ±1 and `0..1` and must go on meaning the body they named
    /// when they were written down.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError::Truncated`] if the payload ends early.
    fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError>;

    /// Reads parameters from a **version-4 or version-5** share code (#169).
    ///
    /// Between them, those two versions carried a humanoid payload with two
    /// dead bytes in the middle of it: `build` and `muscle` retired into `mass`
    /// and `bodyFat` in #164, and their slots were held rather than removed so
    /// that codes minted before the retirement went on decoding at the right
    /// offsets. Version 6 removes the slots, so the older layout needs a path
    /// of its own — this one.
    ///
    /// **Identical to [`Self::decode`] on every plan whose axes did not
    /// retire**, which is why it is a plain method rather than a flag: the
    /// quadruped's `build` and `muscle` are live axes and its two layouts are
    /// the same bytes.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError::Truncated`] if the payload ends early.
    fn decode_reserved(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError>;
}

/// Why a body plan could not be read from a share code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PlanDecodeError {
    /// The payload ran out before every axis was read.
    #[error("share code payload ended early")]
    Truncated,
    /// The archetype tag is not one this build knows.
    #[error("unknown archetype tag {0}")]
    UnknownArchetype(u8),
}

/// Type name of the humanoid variant, in the shared definitions lexicon.
pub const HUMANOID_TYPE: &str = "network.symbios.avatar.defs#humanoid";
/// Type name of the quadruped variant.
pub const QUADRUPED_TYPE: &str = "network.symbios.avatar.defs#quadruped";

/// Which kind of body a record describes.
///
/// An open union discriminated by `$type`, as AT Protocol lexicons require:
/// each variant names a definition in `network.symbios.avatar.defs`, so a reader
/// that does not know a variant can recognise that fact rather than mis-render
/// the body. New archetypes are added as variants without disturbing old ones.
///
/// **Open means a reader must survive a `$type` it has never heard of.** WS6
/// adds creature archetypes by design, so the day the first one exists every
/// deployed client would otherwise lose the ability to render even a
/// placeholder — a derived `Deserialize` fails the whole record on an unknown
/// variant, taking the name, the seed and the locks down with the body. The
/// [`Archetype::Unknown`] variant keeps the type name and every field verbatim
/// so a read-modify-write cannot destroy a body this build does not understand,
/// and [`Archetype::is_understood`] lets a client say so rather than pretend.
#[derive(Clone, Debug, PartialEq)]
pub enum Archetype {
    /// An upright biped.
    Humanoid(HumanoidParams),
    /// A four-legged creature.
    Quadruped(QuadrupedParams),
    /// A body this build does not know about, preserved as written.
    Unknown {
        /// The `$type` that was read.
        type_name: String,
        /// Every other field, kept so writing the record back loses nothing.
        fields: serde_json::Map<String, serde_json::Value>,
    },
}

impl Serialize for Archetype {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let (type_name, body) = match self {
            Archetype::Humanoid(params) => (HUMANOID_TYPE, serde_json::to_value(params)),
            Archetype::Quadruped(params) => (QUADRUPED_TYPE, serde_json::to_value(params)),
            Archetype::Unknown { type_name, fields } => {
                let mut map = serializer.serialize_map(Some(fields.len() + 1))?;
                map.serialize_entry("$type", type_name)?;
                for (key, value) in fields {
                    map.serialize_entry(key, value)?;
                }
                return map.end();
            }
        };
        let body = body.map_err(serde::ser::Error::custom)?;
        let body = body
            .as_object()
            .ok_or_else(|| serde::ser::Error::custom("a body plan must serialise to an object"))?;
        let mut map = serializer.serialize_map(Some(body.len() + 1))?;
        map.serialize_entry("$type", type_name)?;
        for (key, value) in body {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Archetype {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        // Read the object first, then dispatch on its `$type`. A derived
        // internally-tagged enum cannot express "and otherwise keep what you
        // were given", which is the whole requirement here.
        let mut value = serde_json::Map::deserialize(deserializer)?;
        let type_name = match value.remove("$type") {
            Some(serde_json::Value::String(name)) => name,
            // No discriminator at all: an older writer, or a hand-written
            // record. Treat it as the default body rather than as a loss.
            None => HUMANOID_TYPE.to_string(),
            Some(other) => {
                return Err(D::Error::custom(format!(
                    "$type must be a string, got {other}"
                )));
            }
        };
        let body = serde_json::Value::Object(value);
        match type_name.as_str() {
            HUMANOID_TYPE => serde_json::from_value(body)
                .map(Archetype::Humanoid)
                .map_err(D::Error::custom),
            QUADRUPED_TYPE => serde_json::from_value(body)
                .map(Archetype::Quadruped)
                .map_err(D::Error::custom),
            _ => Ok(Archetype::Unknown {
                type_name,
                fields: match body {
                    serde_json::Value::Object(fields) => fields,
                    _ => serde_json::Map::new(),
                },
            }),
        }
    }
}

impl Default for Archetype {
    fn default() -> Self {
        Archetype::Humanoid(HumanoidParams::default())
    }
}

impl Archetype {
    /// Tag byte identifying the variant inside a share code.
    ///
    /// Zero for a body this build does not understand: a share code carries
    /// quantised axes, and there are none to quantise for a plan whose axes are
    /// unknown.
    #[must_use]
    pub fn tag(&self) -> u8 {
        match self {
            Archetype::Humanoid(_) => 1,
            Archetype::Quadruped(_) => 2,
            Archetype::Unknown { .. } => 0,
        }
    }

    /// Human-readable name, for creator panels and diagnostics.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Archetype::Humanoid(_) => "humanoid",
            Archetype::Quadruped(_) => "quadruped",
            Archetype::Unknown { type_name, .. } => type_name,
        }
    }

    /// Whether this build knows what kind of body this is.
    ///
    /// A client showing an avatar it cannot render should say so — "this body
    /// needs a newer version" — rather than silently show the stand-in
    /// [`Archetype::skeleton`] hands back.
    #[must_use]
    pub fn is_understood(&self) -> bool {
        !matches!(self, Archetype::Unknown { .. })
    }

    /// Clamps the wrapped parameters into range.
    ///
    /// An unknown body is left exactly as it was read. Nothing here knows what
    /// its axes mean, so clamping them would be guessing, and guessing wrong
    /// destroys the record on the next write.
    pub fn sanitize(&mut self) {
        match self {
            Archetype::Humanoid(params) => params.sanitize(),
            Archetype::Quadruped(params) => params.sanitize(),
            Archetype::Unknown { .. } => {}
        }
    }

    /// Builds the capsule graph for these parameters.
    ///
    /// An unknown body gets the default humanoid, so a client renders a
    /// stand-in rather than nothing. Pair it with [`Archetype::is_understood`]:
    /// showing the stand-in *and* saying it is one is the honest behaviour;
    /// showing it silently is the substitution the lexicon warns against.
    #[must_use]
    pub fn skeleton(&self, composites: &Composites) -> Skeleton {
        match self {
            Archetype::Humanoid(params) => params.skeleton(composites),
            Archetype::Quadruped(params) => params.skeleton(composites),
            Archetype::Unknown { .. } => HumanoidParams::default().skeleton(composites),
        }
    }

    /// Re-rolls one category of axes.
    ///
    /// An unknown body is left alone: re-rolling axes whose meaning is unknown
    /// would be inventing a body, not re-rolling one.
    pub fn reroll(&mut self, category: Category, rolls: &Rolls, composites: &Composites) {
        match self {
            Archetype::Humanoid(params) => params.reroll(category, rolls, composites),
            Archetype::Quadruped(params) => params.reroll(category, rolls, composites),
            Archetype::Unknown { .. } => {}
        }
    }

    /// Appends the tag and quantised parameters to a share code.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.tag());
        match self {
            Archetype::Humanoid(params) => params.encode(out),
            Archetype::Quadruped(params) => params.encode(out),
            Archetype::Unknown { .. } => {}
        }
    }

    /// Reads an archetype back from a share code payload.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError`] if the tag is unknown or the payload is short.
    pub fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let (&tag, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
        *bytes = rest;
        match tag {
            1 => Ok(Archetype::Humanoid(HumanoidParams::decode(bytes)?)),
            2 => Ok(Archetype::Quadruped(QuadrupedParams::decode(bytes)?)),
            other => Err(PlanDecodeError::UnknownArchetype(other)),
        }
    }

    /// Reads an archetype from a **version-3** share code payload (#160).
    ///
    /// See [`BodyPlan::decode_legacy`]: same layout, narrower byte spans.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError`] if the tag is unknown or the payload is short.
    pub fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let (&tag, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
        *bytes = rest;
        match tag {
            1 => Ok(Archetype::Humanoid(HumanoidParams::decode_legacy(bytes)?)),
            2 => Ok(Archetype::Quadruped(QuadrupedParams::decode_legacy(bytes)?)),
            other => Err(PlanDecodeError::UnknownArchetype(other)),
        }
    }

    /// Reads an archetype from a **version-4 or version-5** payload (#169).
    ///
    /// See [`BodyPlan::decode_reserved`]: the same spans, with the two slots
    /// `build` and `muscle` left behind them still on the wire.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError`] if the tag is unknown or the payload is short.
    pub fn decode_reserved(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let (&tag, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
        *bytes = rest;
        match tag {
            1 => Ok(Archetype::Humanoid(HumanoidParams::decode_reserved(bytes)?)),
            2 => Ok(Archetype::Quadruped(QuadrupedParams::decode_reserved(
                bytes,
            )?)),
            other => Err(PlanDecodeError::UnknownArchetype(other)),
        }
    }
}

/// Serialises axes as scaled integers.
///
/// The AT Protocol data model has **no floating-point type** — it was left out
/// so that records have one canonical binary encoding. Axes are therefore stored
/// as thousandths and lengths as millimetres, and the conversion lives here so
/// every parameter struct gets it right by construction.
pub(crate) mod scaled {
    use serde::{Deserialize, Deserializer, Serializer};

    /// One unit of the wire representation, in axis units.
    const SCALE: f32 = 1000.0;

    /// Writes a float as its scaled integer.
    ///
    /// **This used to narrow to `i16` first, and that was a silent ceiling of
    /// 32.767 IN AXIS UNITS** (#170). Nothing about the wire format asked for
    /// it: the value is emitted as an `i32` and read back as an `i64`, and
    /// [`deserialize`] says in as many words why it reads wide — an
    /// out-of-range value is bad data for `sanitize` to clamp, never a reason
    /// to lose a record. A writer that quietly truncates is the same defect
    /// from the other end, and it is worse, because the truncation is what
    /// gets stored.
    ///
    /// No shipped axis met it — every shape axis is inside the ±3 exploration
    /// envelope and every length is a stature in metres — but #162 walked
    /// straight into it: the composites wanted an age in whole years, and forty
    /// years through this encoder is 40000 thousandths, which would have stored
    /// a forty-year-old as 32.767. That axis is a count instead, which is the
    /// honest representation anyway, so the trap was avoided rather than met
    /// and the next physical quantity would have met it silently.
    ///
    /// What bounds it now is the `as` cast, which saturates: an axis would have
    /// to reach 2_147_483 in its own units to lose anything here, and a value
    /// that large is a caller that never sanitized. The narrowing is gone; the
    /// principle — read wide, write wide, clamp in `sanitize` — is the module's
    /// throughout.
    ///
    /// # Errors
    ///
    /// Propagates the serialiser's own failure.
    pub fn serialize<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
        let scaled = if value.is_finite() {
            (value * SCALE).round()
        } else {
            0.0
        };
        serializer.serialize_i32(scaled as i32)
    }

    /// Reads a scaled integer back into a float.
    ///
    /// Read as `i64` — the widest AT Protocol integer — rather than as the `i32`
    /// the writer produces. A value outside the axis's range is bad *data* and
    /// belongs to `sanitize`, which clamps it; reading it narrowly makes it a
    /// bad *parse* instead, and the whole record is lost over one field being
    /// out of bounds. Sanitising cannot run on a record that would not load.
    ///
    /// # Errors
    ///
    /// Propagates the deserialiser's own failure.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
        Ok(i64::deserialize(deserializer)? as f32 / SCALE)
    }

    /// Reads a count that must survive being out of range.
    ///
    /// The same argument as [`deserialize`]: a hair group count of four billion
    /// is a value for `sanitize` to clamp, not a reason to lose the avatar.
    ///
    /// # Errors
    ///
    /// Propagates the deserialiser's own failure.
    pub fn deserialize_count<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        Ok(i64::deserialize(deserializer)?.clamp(0, i64::from(u32::MAX)) as u32)
    }

    /// Writes a count.
    ///
    /// # Errors
    ///
    /// Propagates the serialiser's own failure.
    pub fn serialize_count<S: Serializer>(value: &u32, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i64(i64::from(*value))
    }

    /// Snaps a value to the precision the wire format can carry.
    ///
    /// Sanitising applies this so a record in memory is exactly the record that
    /// will be written: without it, a value round-trips to something slightly
    /// different and nothing downstream can be compared for equality.
    #[must_use]
    pub fn quantize(value: f32) -> f32 {
        if value.is_finite() {
            (value * SCALE).round() / SCALE
        } else {
            0.0
        }
    }
}

/// Brings one axis into range: substitute, then clamp, then quantise.
///
/// **The order is the whole point, and getting it wrong is silent.**
/// `f32::clamp` propagates `NaN` rather than choosing a bound, and
/// [`scaled::quantize`] maps every non-finite value to `0.0` — so a guard
/// placed *after* those two can never fire, and the axis lands on zero
/// whatever its range says. Both body plans had exactly that shape, and it was
/// only visible on `height`, whose range excludes zero (#55). Every other axis
/// agreed with zero because zero is its neutral, which made fifteen of
/// seventeen axes correct by coincidence rather than by construction.
///
/// So the fallback is passed in rather than assumed: callers hand over the
/// axis's own value from `Default::default()`, which cannot drift from the
/// documented default because it *is* the documented default. An axis added
/// later whose neutral is not zero is then correct for free — which is the
/// case this helper exists to serve, since nothing would have caught it.
///
/// Infinities take the fallback too, not the near bound. A slider cannot
/// produce one; an arithmetic accident upstream can, and answering it with a
/// 2.2 m body is a worse guess than answering with the default. This matches
/// `EyeParams::sanitize` and the hair and skin params, which already
/// substitute before clamping.
#[must_use]
pub(crate) fn sanitize_axis(value: f32, fallback: f32, range: (f32, f32)) -> f32 {
    let value = if value.is_finite() { value } else { fallback };
    scaled::quantize(value.clamp(range.0, range.1))
}

/// Quantises an axis to one byte over an explicit span (#160).
///
/// The share code's byte has always mapped a fixed range; with the
/// exploration envelope the range is per-axis, so the span is a parameter and
/// the same constants `sanitize` clamps with are the ones the code writes
/// with — one authority, two consumers. Codes are deliberately lossy and a
/// wider span is coarser per step: a ±3 axis moves in ~0.024 steps against
/// the old ±1's 0.008, which is still below what a slider shows.
pub(crate) fn put_span(out: &mut Vec<u8>, value: f32, span: (f32, f32)) {
    let width = (span.1 - span.0).max(f32::EPSILON);
    let unit = ((value - span.0) / width).clamp(0.0, 1.0);
    out.push((unit * 255.0).round() as u8);
}

/// Reads a one-byte axis back over the same span it was written with.
pub(crate) fn take_span(bytes: &mut &[u8], span: (f32, f32)) -> Result<f32, PlanDecodeError> {
    let unit = f32::from(take_byte(bytes)?) / 255.0;
    Ok(span.0 + unit * (span.1 - span.0))
}

/// Quantises a `-1..=1` axis to one byte.
pub(crate) fn put_signed(out: &mut Vec<u8>, value: f32) {
    let unit = (value.clamp(-1.0, 1.0) + 1.0) * 0.5;
    out.push((unit * 255.0).round() as u8);
}

/// Reads a `-1..=1` axis from one byte.
pub(crate) fn take_signed(bytes: &mut &[u8]) -> Result<f32, PlanDecodeError> {
    Ok(f32::from(take_byte(bytes)?) / 255.0 * 2.0 - 1.0)
}

/// Quantises a `0..=1` axis to one byte.
pub(crate) fn put_unit(out: &mut Vec<u8>, value: f32) {
    out.push((value.clamp(0.0, 1.0) * 255.0).round() as u8);
}

/// Reads a `0..=1` axis from one byte.
pub(crate) fn take_unit(bytes: &mut &[u8]) -> Result<f32, PlanDecodeError> {
    Ok(f32::from(take_byte(bytes)?) / 255.0)
}

/// Quantises a length in metres to two bytes of millimetres.
pub(crate) fn put_length(out: &mut Vec<u8>, metres: f32) {
    let millimetres = (metres.max(0.0) * 1000.0).round().min(f32::from(u16::MAX)) as u16;
    out.extend_from_slice(&millimetres.to_le_bytes());
}

/// Reads a length in metres from two bytes of millimetres.
pub(crate) fn take_length(bytes: &mut &[u8]) -> Result<f32, PlanDecodeError> {
    let low = take_byte(bytes)?;
    let high = take_byte(bytes)?;
    Ok(f32::from(u16::from_le_bytes([low, high])) / 1000.0)
}

/// Pops one byte from the payload.
fn take_byte(bytes: &mut &[u8]) -> Result<u8, PlanDecodeError> {
    let (&first, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
    *bytes = rest;
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_axis_with_a_large_natural_unit_survives_the_thousandths_encoder() {
        // #170, and the reason it is a test rather than a comment: `scaled`
        // used to narrow to `i16` before writing, which is a ceiling of 32.767
        // in AXIS UNITS and fires on nothing this crate ships. An axis whose
        // natural unit is large — a duration in seconds, a mass in kilograms,
        // the age in years #162 nearly stored — walked into it and was
        // truncated in silence rather than refused.
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct Big {
            #[serde(with = "super::scaled")]
            axis: f32,
        }

        for value in [40.0f32, 1_000.0, -1_000.0] {
            let json = serde_json::to_string(&Big { axis: value }).expect("serialises");
            let thousandths = (value * 1000.0) as i64;
            assert!(
                json.contains(&thousandths.to_string()),
                "{value} was written as {json}, which is not {thousandths} thousandths"
            );
            let back: Big = serde_json::from_str(&json).expect("reads back");
            assert_eq!(back.axis, value);
        }
    }

    /// How far a zone's nodes reach along an axis, least and greatest.
    fn span(skeleton: &Skeleton, zone: Zone, axis: fn(glam::Vec3) -> f32) -> (f32, f32) {
        skeleton.nodes.iter().filter(|node| node.zone == zone).fold(
            (f32::MAX, f32::MIN),
            |(lo, hi), node| {
                let at = axis(node.position);
                (lo.min(at), hi.max(at))
            },
        )
    }

    /// **A body's left limbs are the ones at `+X`.** The guard #142 was missing.
    ///
    /// Stated as a coordinate convention rather than as a comparison between the
    /// two sides, because that is the difference between this and the test that
    /// let the body wear two right hands for as long as that code existed (#113):
    /// a comparison of bounds is satisfied by a HALF TURN just as well as by a
    /// reflection, so it cannot tell a mirrored body from a correct one. Only an
    /// absolute statement can, and the absolute statement is glTF's: right-handed,
    /// `+Y` up, front at `+Z`, so right is `Z × Y` which is `−X`.
    ///
    /// The caller has to have shown its body faces `+Z` first, off the body's own
    /// geometry — that is the other half of the convention and it is measured
    /// per plan below, not assumed here.
    fn assert_left_limbs_are_at_positive_x(skeleton: &Skeleton, what: &str) {
        for limb in Limb::ALL {
            let mut nodes = 0;
            for node in &skeleton.nodes {
                if node.zone.limb() != Some(limb) {
                    continue;
                }
                nodes += 1;
                assert!(
                    (node.position.x > 0.0) == limb.is_left(),
                    "{what}: a node of {limb:?} sits at x {:+.1} mm, and a limb \
                     named Left belongs at +X on a body facing +Z",
                    node.position.x * 1000.0
                );
            }
            assert!(nodes > 0, "{what}: {limb:?} has no nodes to place");
        }
    }

    #[test]
    fn a_humanoid_faces_forward_and_puts_its_left_at_positive_x() {
        let skeleton =
            Archetype::Humanoid(HumanoidParams::default()).skeleton(&crate::Composites::default());

        // Facing, measured off the feet exactly as #139 measured it: a foot
        // reaches much further ahead of its ankle than behind it, so the sign of
        // the longer reach is the sign of forward.
        let (heel, toe) = span(&skeleton, Zone::Extremity(Limb::HindLeft), |at| at.z);
        assert!(
            toe > -heel,
            "the foot reaches {:.1} mm forward and {:.1} mm back, so this body \
             does not face +Z and the whole convention below is void",
            toe * 1000.0,
            -heel * 1000.0
        );

        assert_left_limbs_are_at_positive_x(&skeleton, "the humanoid");
    }

    #[test]
    fn a_quadruped_faces_forward_and_puts_its_left_at_positive_x() {
        let skeleton = Archetype::Quadruped(QuadrupedParams::default())
            .skeleton(&crate::Composites::default());

        // No foot to measure on this plan — its extremities are single nodes —
        // so facing is read off the two ends of the body instead: a head is
        // ahead of a tail.
        let (_, head) = span(&skeleton, Zone::Head, |at| at.z);
        let (tail, _) = span(&skeleton, Zone::Tail, |at| at.z);
        assert!(
            head > tail,
            "the head is at {:.1} mm and the tail at {:.1} mm, so this body does \
             not face +Z",
            head * 1000.0,
            tail * 1000.0
        );

        assert_left_limbs_are_at_positive_x(&skeleton, "the quadruped");
    }

    #[test]
    fn categories_occupy_distinct_bits() {
        let mut seen = 0u32;
        for category in Category::ALL {
            assert_eq!(seen & category.bit(), 0, "{category:?} collides");
            seen |= category.bit();
        }
    }

    #[test]
    fn quantisation_round_trips_within_tolerance() {
        for value in [-1.0f32, -0.5, 0.0, 0.25, 1.0] {
            let mut buffer = Vec::new();
            put_signed(&mut buffer, value);
            let mut slice = buffer.as_slice();
            let back = take_signed(&mut slice).expect("reads back");
            assert!((back - value).abs() < 0.01, "{value} -> {back}");
        }

        let mut buffer = Vec::new();
        put_length(&mut buffer, 1.752);
        let mut slice = buffer.as_slice();
        assert!((take_length(&mut slice).expect("reads back") - 1.752).abs() < 0.001);
    }

    #[test]
    fn truncated_payloads_are_rejected() {
        let mut empty: &[u8] = &[];
        assert_eq!(take_signed(&mut empty), Err(PlanDecodeError::Truncated));
        assert_eq!(
            Archetype::decode(&mut empty),
            Err(PlanDecodeError::Truncated)
        );

        let mut unknown: &[u8] = &[99];
        assert_eq!(
            Archetype::decode(&mut unknown),
            Err(PlanDecodeError::UnknownArchetype(99))
        );
    }
}
