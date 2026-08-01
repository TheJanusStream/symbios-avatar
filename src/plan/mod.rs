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

use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use crate::skeleton::Skeleton;

pub use humanoid::HumanoidParams;
pub use quadruped::QuadrupedParams;

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
    fn reroll(&mut self, category: Category, rng: &mut Pcg64Mcg);

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

/// Which kind of body a record describes.
///
/// An open union discriminated by `$type`, as AT Protocol lexicons require:
/// each variant names a definition in `network.symbios.avatar.defs`, so a reader
/// that does not know a variant can recognise that fact rather than mis-render
/// the body. New archetypes are added as variants without disturbing old ones.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum Archetype {
    /// An upright biped.
    #[serde(rename = "network.symbios.avatar.defs#humanoid")]
    Humanoid(HumanoidParams),
    /// A four-legged creature.
    #[serde(rename = "network.symbios.avatar.defs#quadruped")]
    Quadruped(QuadrupedParams),
}

impl Default for Archetype {
    fn default() -> Self {
        Archetype::Humanoid(HumanoidParams::default())
    }
}

impl Archetype {
    /// Tag byte identifying the variant inside a share code.
    #[must_use]
    pub fn tag(&self) -> u8 {
        match self {
            Archetype::Humanoid(_) => 1,
            Archetype::Quadruped(_) => 2,
        }
    }

    /// Human-readable name, for creator panels and diagnostics.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Archetype::Humanoid(_) => "humanoid",
            Archetype::Quadruped(_) => "quadruped",
        }
    }

    /// Clamps the wrapped parameters into range.
    pub fn sanitize(&mut self) {
        match self {
            Archetype::Humanoid(params) => params.sanitize(),
            Archetype::Quadruped(params) => params.sanitize(),
        }
    }

    /// Builds the capsule graph for these parameters.
    #[must_use]
    pub fn skeleton(&self) -> Skeleton {
        match self {
            Archetype::Humanoid(params) => params.skeleton(),
            Archetype::Quadruped(params) => params.skeleton(),
        }
    }

    /// Re-rolls one category of axes.
    pub fn reroll(&mut self, category: Category, rng: &mut Pcg64Mcg) {
        match self {
            Archetype::Humanoid(params) => params.reroll(category, rng),
            Archetype::Quadruped(params) => params.reroll(category, rng),
        }
    }

    /// Appends the tag and quantised parameters to a share code.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.tag());
        match self {
            Archetype::Humanoid(params) => params.encode(out),
            Archetype::Quadruped(params) => params.encode(out),
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
    /// # Errors
    ///
    /// Propagates the deserialiser's own failure.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
        Ok(i32::deserialize(deserializer)? as f32 / SCALE)
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
