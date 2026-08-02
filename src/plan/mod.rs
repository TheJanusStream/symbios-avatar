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

mod humanoid;
mod quadruped;
mod zone;

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use crate::skeleton::Skeleton;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    /// Overall size — how tall or long the body is.
    Stature,
    /// Mass and musculature.
    Build,
    /// Skeletal frame: shoulder and hip width.
    Frame,
    /// Relative lengths of limbs, neck, and tail.
    Proportions,
    /// Head and extremity sizes.
    Features,
}

impl Category {
    /// Every category, in creator-panel order.
    pub const ALL: [Category; 5] = [
        Category::Stature,
        Category::Build,
        Category::Frame,
        Category::Proportions,
        Category::Features,
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
}

/// Behaviour every body plan provides.
pub trait BodyPlan: Sized {
    /// Clamps every axis into its valid range.
    ///
    /// Must be idempotent: sanitising twice equals sanitising once.
    fn sanitize(&mut self);

    /// Builds the capsule graph this plan describes.
    ///
    /// The result is always meshable by [`crate::cage::build_cage`] for any
    /// sanitised parameters.
    fn skeleton(&self) -> Skeleton;

    /// Draws fresh values for the axes belonging to `category`.
    ///
    /// Each axis draws from its own named stream — see [`Rolls`] — so adding one
    /// never changes what the others produce for a given seed.
    fn reroll(&mut self, category: Category, rolls: &Rolls);

    /// Appends this plan's quantised parameters to a share code.
    fn encode(&self, out: &mut Vec<u8>);

    /// Reads parameters back from a share code.
    ///
    /// # Errors
    ///
    /// Returns [`PlanDecodeError::Truncated`] if the payload ends early.
    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError>;
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
    pub fn skeleton(&self) -> Skeleton {
        match self {
            Archetype::Humanoid(params) => params.skeleton(),
            Archetype::Quadruped(params) => params.skeleton(),
            Archetype::Unknown { .. } => HumanoidParams::default().skeleton(),
        }
    }

    /// Re-rolls one category of axes.
    ///
    /// An unknown body is left alone: re-rolling axes whose meaning is unknown
    /// would be inventing a body, not re-rolling one.
    pub fn reroll(&mut self, category: Category, rolls: &Rolls) {
        match self {
            Archetype::Humanoid(params) => params.reroll(category, rolls),
            Archetype::Quadruped(params) => params.reroll(category, rolls),
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
    /// # Errors
    ///
    /// Propagates the serialiser's own failure.
    pub fn serialize<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
        let scaled = if value.is_finite() {
            (value * SCALE).round()
        } else {
            0.0
        };
        serializer.serialize_i32(scaled.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i32)
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
