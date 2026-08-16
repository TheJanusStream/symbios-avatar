//! Sending a painted atlas across a worker boundary.
//!
//! [`TextureMap`] belongs to the published `symbios-texture` crate, so this
//! crate cannot derive `Serialize` on it — and a downstream that needs a
//! built [`crate::Avatar`] to cross a process or Web Worker boundary needs
//! the atlas most of all, because it is the bulk of the payload. This is the
//! `#[serde(with = ...)]` adapter that closes the gap over the type's own
//! public buffers.
//!
//! The buffers ride as **opaque bytes** (`serde_bytes`), not as sequences of
//! `u8`: a self-describing binary codec — msgpack, which is what the worker
//! boundary this exists for actually uses — writes a per-element tag for a
//! plain `Vec<u8>` and roughly doubles a multi-megabyte atlas. `bin` is one
//! header and a memcpy.
//!
//! Only compiled under `serde-avatar`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use symbios_texture::generator::TextureMap;

/// The atlas as it travels: the same public buffers, byte-tagged.
#[derive(Serialize, Deserialize)]
struct Wire {
    #[serde(with = "serde_bytes")]
    albedo: Vec<u8>,
    #[serde(with = "serde_bytes")]
    normal: Vec<u8>,
    #[serde(with = "serde_bytes")]
    roughness: Vec<u8>,
    #[serde(with = "serde_bytes")]
    emissive: Option<Vec<u8>>,
    width: u32,
    height: u32,
    mip_level_count: u32,
}

/// Serialize a painted atlas.
///
/// # Errors
///
/// Propagates the serializer's own failures; the conversion itself cannot
/// fail.
pub fn serialize<S: Serializer>(map: &TextureMap, serializer: S) -> Result<S::Ok, S::Error> {
    Wire {
        albedo: map.albedo.clone(),
        normal: map.normal.clone(),
        roughness: map.roughness.clone(),
        emissive: map.emissive.clone(),
        width: map.width,
        height: map.height,
        mip_level_count: map.mip_level_count,
    }
    .serialize(serializer)
}

/// Deserialize a painted atlas, checking that the buffers describe the
/// dimensions they claim.
///
/// The check is not defensive tidiness: an atlas whose buffer is short for
/// its stated size is uploaded to a GPU by whatever receives it, and the
/// failure there is a driver-level read past the end rather than a bad
/// picture. A truncated or mismatched payload is refused here, where it can
/// still be reported.
///
/// # Errors
///
/// Returns an error if a buffer is too short for `width × height × 4` (plus
/// its declared mip levels), or if the deserializer itself fails.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TextureMap, D::Error> {
    let wire = Wire::deserialize(deserializer)?;
    // The base level, plus every declared mip. Mip n is a quarter of n−1, so
    // the whole chain is bounded by 4/3 of the base — computed exactly here
    // rather than bounded, since a wrong count is the thing being caught.
    let mut expected = 0usize;
    let (mut w, mut h) = (wire.width as usize, wire.height as usize);
    for _ in 0..wire.mip_level_count.max(1) {
        expected += w.max(1) * h.max(1) * 4;
        w /= 2;
        h /= 2;
    }
    for (name, buffer) in [
        ("albedo", &wire.albedo),
        ("normal", &wire.normal),
        ("roughness", &wire.roughness),
    ] {
        if buffer.len() < expected {
            return Err(D::Error::custom(format!(
                "atlas {name} carries {} bytes for a {}×{} map of {} mip levels, which needs {expected}",
                buffer.len(),
                wire.width,
                wire.height,
                wire.mip_level_count
            )));
        }
    }
    Ok(TextureMap {
        albedo: wire.albedo,
        normal: wire.normal,
        roughness: wire.roughness,
        emissive: wire.emissive,
        width: wire.width,
        height: wire.height,
        mip_level_count: wire.mip_level_count,
    })
}
