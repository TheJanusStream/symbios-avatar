//! Painting a body.
//!
//! Two steps, deliberately separate. [`bake_geometry`] rasterises the unwrapped
//! body into texture space, answering "where on the body is this texel?" once.
//! [`paint_skin`] then runs as a pure function of one texel's sample, which is
//! what keeps a procedural complexion tractable to write and to test.
//!
//! Output is a [`symbios_texture::generator::TextureMap`] — the same container
//! every other symbios generator produces — so an avatar's skin travels through
//! the ecosystem's existing image-conversion and upload path unchanged.
//!
//! ```rust
//! use symbios_avatar::{
//!     AvatarRecord, CageConfig, Rig, SkinConfig, UvConfig, build_cage, catmull_clark,
//!     rig::skin, texture, unwrap,
//! };
//!
//! let record = AvatarRecord::default();
//! let skeleton = record.skeleton();
//! let mesh = catmull_clark(&build_cage(&skeleton, &CageConfig::default())?, 2);
//! let rig = Rig::from_skeleton(&skeleton)?;
//!
//! let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
//! let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
//!
//! let geometry = texture::bake_geometry(&mesh, &uv, 512);
//! let map = texture::paint_skin(&geometry, &rig, &record.skin);
//! assert_eq!(map.width, 512);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod bake;
pub mod skin;

pub use bake::{AtlasGeometry, DILATION, Texel, bake, bake_geometry};
pub use skin::{SkinParams, paint_skin};
