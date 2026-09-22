//! What a garment may be made of: the tileable surfaces of `symbios-texture`.
//!
//! A garment is dyed cloth by default, and [`GarmentTexture`] is how a record
//! asks for more: one of the ecosystem's procedural surfaces — a woven fabric,
//! a plate of metal, a tiled print — laid over the garment at a tile size and
//! an angle, and multiplied by the garment's own colour the way a material's
//! base colour multiplies its texture everywhere else in the ecosystem.
//!
//! ## Surfaces, not cards
//!
//! [`SurfaceConfig`] is generated from `symbios_texture::for_each_generator!`,
//! keeping the rows marked `Surface` and dropping the ones marked `Card`. A card
//! — a leaf, a window, a particle sprite — is an alpha cut-out that must span
//! its quad exactly once, and a garment is a tiled surface with no quad to
//! span. The registry already knows which is which, so a surface added upstream
//! appears here on the next dependency bump with nothing written in this file.
//!
//! ## The wire
//!
//! A config crosses the wire the way `symbios-overlands` writes the same
//! configs into its material records, so the two can hand each other a
//! texture: the variant's name under `$type`, the config's own field names,
//! and every float as an integer in ten-thousandths ([`WIRE_SCALE`]) because
//! the AT Protocol data model has no floating-point type. That is a finer grid
//! than the thousandths the rest of an avatar record uses, and deliberately:
//! a texture config carries grout widths and warp amplitudes a thousandth
//! cannot resolve, and one texture reading the same in two applications is
//! worth more than one unit across the record.
//!
//! Reading is driven by the config's own default rather than by a hand-kept
//! table of which field is a float: [`SurfaceConfig`] serialises the default,
//! and every place that default holds a float is a place the wire holds an
//! integer to scale back. A field the wire leaves out takes the default's
//! value, so a writer that elides defaults — which `symbios-overlands` does —
//! reads back exactly.
//!
//! ## Degrading
//!
//! A name this build does not know is kept verbatim as
//! [`SurfaceConfig::Unknown`] and written back unchanged, as every other open
//! union in these records is; a garment wearing one is drawn in its plain
//! colour. So is a known name whose fields will not read, rather than failing
//! the avatar over one texture.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use symbios_texture::generator::{TextureError, TextureGenerator, TextureMap};

/// One unit of a texture config's floats on the wire.
///
/// Ten-thousandths, the grid `symbios-overlands` writes the same configs in —
/// see the module documentation for why this is not the record's thousandths.
pub const WIRE_SCALE: f64 = 10_000.0;

/// How many tiles a garment's texture repeats per metre of cloth, when a
/// record does not say.
///
/// A 25 cm tile, so a woven fabric's default two dozen threads come out about
/// a centimetre apart — coarse for cloth, and about as fine as the cloth
/// atlas resolves: it carries one texel every five millimetres or so
/// (`dress::cloth::side`), two to a thread.
///
/// Provenance: **derived** from that density; not yet judged by render.
pub const DEFAULT_SCALE: f32 = 4.0;

/// The tile repeats a garment's texture may ask for, per metre.
///
/// From a 4 m tile, which lays one pattern repeat over the whole garment, to
/// a 2 cm one, past which the cloth atlas cannot resolve a tile at all and a
/// pattern turns to a flat average of itself.
pub const SCALE_RANGE: (f32, f32) = (0.25, 50.0);

/// How far a garment's pattern may be turned, in degrees either way.
pub const ROTATION_RANGE: (f32, f32) = (-180.0, 180.0);

/// Generates [`SurfaceConfig`] from the generator registry's `Surface` rows.
///
/// The registry passes every row, cards included; the `@keep` arms walk them
/// and drop each `Card`, so the enum is the surfaces and nothing else.
macro_rules! surfaces {
    ($($row:tt),* $(,)?) => {
        surfaces!(@keep [] $($row),*);
    };
    (@keep [$($kept:tt)*] ($variant:ident, $module:ident, $config:ty, $generator:ty, Surface) $(, $rest:tt)*) => {
        surfaces!(@keep [$($kept)* ($variant, $config, $generator)] $($rest),*);
    };
    (@keep [$($kept:tt)*] ($variant:ident, $module:ident, $config:ty, $generator:ty, Card) $(, $rest:tt)*) => {
        surfaces!(@keep [$($kept)*] $($rest),*);
    };
    (@keep [$(($variant:ident, $config:ty, $generator:ty))*]) => {
        /// One tileable surface from `symbios-texture`, configured.
        ///
        /// A variant per `Surface` row of the generator registry, holding
        /// that generator's own config type — see the module documentation.
        /// Not `Copy` and not `Eq`: the configs are neither, and equality is
        /// taken over the wire form, which is what a record means by two
        /// textures being the same.
        #[derive(Clone, Debug)]
        #[allow(missing_docs, reason = "each variant is named for the generator it carries")]
        pub enum SurfaceConfig {
            $( $variant($config), )*
            /// A surface this build does not know, kept verbatim — its
            /// `$type` included — so that writing the record back loses
            /// nothing a newer build put there.
            Unknown(Map<String, Value>),
        }

        impl SurfaceConfig {
            /// Every surface this build knows, by the name it goes by on the
            /// wire, in the registry's order.
            pub const NAMES: &'static [&'static str] = &[$(stringify!($variant)),*];

            /// Every surface this build knows, each at its generator's own
            /// default, in the registry's order.
            ///
            /// What a picker offers. Drawn from the registry, so a surface
            /// added upstream is offered without anything here changing.
            #[must_use]
            pub fn all() -> Vec<SurfaceConfig> {
                vec![$(Self::$variant(<$config>::default())),*]
            }

            /// The surface called `name` on the wire, at its generator's
            /// default, if this build knows it.
            #[must_use]
            pub fn named(name: &str) -> Option<SurfaceConfig> {
                match name {
                    $(stringify!($variant) => Some(Self::$variant(<$config>::default())),)*
                    _ => None,
                }
            }

            /// The name this surface goes by on the wire.
            ///
            /// For [`Self::Unknown`], the name it came in with, or `unknown`
            /// if it came in with none.
            #[must_use]
            pub fn name(&self) -> &str {
                match self {
                    $(Self::$variant(_) => stringify!($variant),)*
                    Self::Unknown(kept) => kept
                        .get("$type")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                }
            }

            /// Generates one square tile of this surface, `size` texels a side.
            ///
            /// `None` for a surface this build cannot draw.
            ///
            /// # Errors
            ///
            /// Propagates the generator's own refusal, which is a dimension
            /// it cannot make.
            pub fn generate(&self, size: u32) -> Option<Result<TextureMap, TextureError>> {
                match self {
                    $(Self::$variant(config) => {
                        Some(<$generator>::new(config.clone()).generate(size, size))
                    })*
                    Self::Unknown(_) => None,
                }
            }

            /// Clamps every field into the envelope the registry gives it.
            fn clamp_to_envelope(&mut self) {
                use symbios_texture::ClampToEnvelope as _;
                match self {
                    $(Self::$variant(config) => config.clamp_to_envelope(),)*
                    Self::Unknown(_) => {}
                }
            }

            /// The config as its own serde writes it: floats as floats.
            ///
            /// `None` for [`Self::Unknown`], whose native form nobody here
            /// knows.
            #[must_use]
            pub fn to_native(&self) -> Option<Value> {
                match self {
                    $(Self::$variant(config) => serde_json::to_value(config).ok(),)*
                    Self::Unknown(_) => None,
                }
            }

            /// A config back from its native form, if `name` is a surface
            /// this build knows and `native` reads as one.
            #[must_use]
            pub fn from_native(name: &str, native: Value) -> Option<SurfaceConfig> {
                match name {
                    $(stringify!($variant) => serde_json::from_value::<$config>(native)
                        .ok()
                        .map(Self::$variant),)*
                    _ => None,
                }
            }

            /// What the wire is read against: the named generator's default,
            /// serialised, whose floats say which wire integers to scale.
            fn template(name: &str) -> Option<Value> {
                match name {
                    $(stringify!($variant) => serde_json::to_value(<$config>::default()).ok(),)*
                    _ => None,
                }
            }
        }
    };
}

symbios_texture::for_each_generator!(surfaces);

impl SurfaceConfig {
    /// Whether this build can draw it.
    #[must_use]
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    /// Clamps every field into its envelope and onto the wire's grid.
    /// Idempotent.
    ///
    /// The round trip through the wire is the quantisation: an in-memory
    /// config that differs from what the wire can carry is one nothing
    /// downstream can compare for equality.
    pub fn sanitize(&mut self) {
        if !self.is_known() {
            return;
        }
        self.clamp_to_envelope();
        *self = Self::from_wire(&self.to_wire());
    }

    /// The config as the wire carries it: `$type` and its fields, floats as
    /// integers in [`WIRE_SCALE`].
    #[must_use]
    pub fn to_wire(&self) -> Map<String, Value> {
        let Self::Unknown(kept) = self else {
            let mut wire = match self.to_native().map(|native| encode(&native)) {
                Some(Value::Object(fields)) => fields,
                // Every config is a struct, so its native form is an object.
                _ => Map::new(),
            };
            wire.insert("$type".into(), Value::String(self.name().to_owned()));
            return wire;
        };
        kept.clone()
    }

    /// A config back from the wire.
    ///
    /// Never fails: a name this build does not know, and a known name whose
    /// fields will not read, both come back as [`Self::Unknown`] holding the
    /// object exactly as it arrived.
    #[must_use]
    pub fn from_wire(wire: &Map<String, Value>) -> Self {
        let decoded = wire.get("$type").and_then(Value::as_str).and_then(|name| {
            let template = Self::template(name)?;
            let native = decode(&Value::Object(wire.clone()), &template);
            Self::from_native(name, native)
        });
        decoded.unwrap_or_else(|| Self::Unknown(wire.clone()))
    }
}

impl PartialEq for SurfaceConfig {
    /// Equal when they write the same wire object — which is what a record
    /// means by two garments wearing the same texture.
    fn eq(&self, other: &Self) -> bool {
        self.to_wire() == other.to_wire()
    }
}

/// A native config's JSON with every float scaled to its wire integer.
fn encode(native: &Value) -> Value {
    match native {
        Value::Number(number) if number.is_f64() => {
            Value::from(to_wire_integer(number.as_f64().unwrap_or(0.0)))
        }
        Value::Array(items) => Value::Array(items.iter().map(encode).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), encode(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// One float as the integer the wire carries.
///
/// A non-finite value has no position on any axis and goes out as zero, the
/// same answer the record's own axes give; `as` saturates the rest.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a saturating cast is the intended behaviour for an absurd value"
)]
fn to_wire_integer(value: f64) -> i64 {
    if value.is_finite() {
        (value * WIRE_SCALE).round() as i64
    } else {
        0
    }
}

/// A wire value read back against the template it must take the shape of.
///
/// The template decides every type: where it holds a float the wire's integer
/// is scaled back, and where the wire is silent — or says something of the
/// wrong shape — the template's own value stands. Keys the template does not
/// have are left out, which is how a field a newer generator added is dropped
/// rather than handed to a config that would refuse it.
#[allow(
    clippy::cast_precision_loss,
    reason = "a wire integer is a ten-thousandth of a config value, far inside f64's exact range"
)]
fn decode(wire: &Value, template: &Value) -> Value {
    match (template, wire) {
        (Value::Number(shape), Value::Number(number)) if shape.is_f64() => {
            if number.is_f64() {
                // A writer that ignored the convention and wrote the float
                // itself: taken at its word rather than divided again.
                Value::from(number.as_f64().unwrap_or(0.0))
            } else {
                let raw = number
                    .as_i64()
                    .map_or_else(|| number.as_u64().map_or(0.0, |v| v as f64), |v| v as f64);
                Value::from(raw / WIRE_SCALE)
            }
        }
        (Value::Number(shape), Value::Number(number)) => {
            // An integer field: a seed, a count. Unsigned in every config
            // the registry holds, and never wider than 32 bits, so a value
            // outside that is bad data to clamp rather than a reason for the
            // config to refuse to read.
            if shape.is_u64() {
                let wide = number
                    .as_i64()
                    .or_else(|| number.as_f64().map(|v| v.round() as i64))
                    .unwrap_or(0);
                Value::from(wide.clamp(0, i64::from(u32::MAX)))
            } else {
                Value::from(
                    number
                        .as_i64()
                        .unwrap_or_else(|| number.as_f64().map_or(0, |v| v.round() as i64)),
                )
            }
        }
        (Value::Bool(_), Value::Bool(_)) | (Value::String(_), Value::String(_)) => wire.clone(),
        (Value::Array(shape), Value::Array(items)) if shape.len() == items.len() => Value::Array(
            shape
                .iter()
                .zip(items)
                .map(|(shape, item)| decode(item, shape))
                .collect(),
        ),
        (Value::Object(shape), Value::Object(fields)) => Value::Object(
            shape
                .iter()
                .map(|(name, shape)| {
                    let value = fields
                        .get(name)
                        .map_or_else(|| shape.clone(), |field| decode(field, shape));
                    (name.clone(), value)
                })
                .collect(),
        ),
        // The wire said something of the wrong shape: the default stands.
        _ => template.clone(),
    }
}

/// A texture laid over one garment.
///
/// The surface, how many times it repeats per metre of cloth, and how far it
/// is turned. It is multiplied by the garment's own colour — white shows the
/// surface as its generator paints it — and baked with the rest of the
/// outfit into the cloth atlas (`crate::dress::cloth`), so a textured outfit
/// still costs one draw.
#[derive(Clone, Debug, PartialEq)]
pub struct GarmentTexture {
    /// Which surface, configured.
    pub surface: SurfaceConfig,
    /// Tile repeats per metre of cloth, like a material's UV scale: `4` lays
    /// a 25 cm tile. Clamped to [`SCALE_RANGE`].
    pub scale: f32,
    /// How far the pattern is turned on the cloth, in degrees
    /// counter-clockwise as the garment is seen from outside. Clamped to
    /// [`ROTATION_RANGE`].
    pub rotation: f32,
}

impl GarmentTexture {
    /// A surface at the default tile size, unturned.
    #[must_use]
    pub fn new(surface: SurfaceConfig) -> Self {
        Self {
            surface,
            scale: DEFAULT_SCALE,
            rotation: 0.0,
        }
    }

    /// Clamps every field into range and onto the wire's grid. Idempotent.
    pub fn sanitize(&mut self) {
        self.surface.sanitize();
        self.scale = crate::plan::sanitize_axis(self.scale, DEFAULT_SCALE, SCALE_RANGE);
        self.rotation = crate::plan::sanitize_axis(self.rotation, 0.0, ROTATION_RANGE);
    }

    /// The texture as the wire carries it.
    #[must_use]
    pub fn to_wire(&self) -> Value {
        let mut wire = Map::new();
        wire.insert("surface".into(), Value::Object(self.surface.to_wire()));
        wire.insert("scale".into(), Value::from(thousandths(self.scale)));
        wire.insert("rotation".into(), Value::from(thousandths(self.rotation)));
        Value::Object(wire)
    }

    /// A texture back from the wire, if the value is one.
    ///
    /// `None` for anything but an object carrying a `surface` object: a
    /// texture without a surface is not a texture, and a garment asked to
    /// wear one wears its plain colour. A surface this build does not know is
    /// NOT that case — it comes back, as [`SurfaceConfig::Unknown`], and is
    /// written out again unchanged.
    #[must_use]
    pub fn from_wire(wire: &Value) -> Option<Self> {
        let fields = wire.as_object()?;
        let surface = SurfaceConfig::from_wire(fields.get("surface")?.as_object()?);
        let axis = |name: &str, default: f32| {
            fields
                .get(name)
                .and_then(Value::as_i64)
                .map_or(default, from_thousandths)
        };
        Some(Self {
            surface,
            scale: axis("scale", DEFAULT_SCALE),
            rotation: axis("rotation", 0.0),
        })
    }
}

impl Serialize for GarmentTexture {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_wire().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GarmentTexture {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = Value::deserialize(deserializer)?;
        Self::from_wire(&wire)
            .ok_or_else(|| serde::de::Error::custom("a garment texture needs a surface object"))
    }
}

/// An axis in the record's own thousandths — the garment's half of a
/// texture, as opposed to the surface's ten-thousandths.
#[allow(
    clippy::cast_possible_truncation,
    reason = "a sanitised axis is far inside i64, and `as` saturates the rest"
)]
fn thousandths(value: f32) -> i64 {
    if value.is_finite() {
        (f64::from(value) * 1000.0).round() as i64
    } else {
        0
    }
}

/// And back.
#[allow(
    clippy::cast_precision_loss,
    reason = "a thousandth of any sane axis is exact in f64 and within f32's resolution of it"
)]
fn from_thousandths(raw: i64) -> f32 {
    (raw as f64 / 1000.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a JSON value holds a float anywhere inside it.
    fn has_float(value: &Value) -> bool {
        match value {
            Value::Number(number) => number.is_f64(),
            Value::Array(items) => items.iter().any(has_float),
            Value::Object(fields) => fields.values().any(has_float),
            _ => false,
        }
    }

    /// Whether a JSON value holds a null anywhere inside it.
    fn has_null(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::Array(items) => items.iter().any(has_null),
            Value::Object(fields) => fields.values().any(has_null),
            _ => false,
        }
    }

    #[test]
    fn the_union_is_the_registry_surfaces_and_no_cards() {
        // Thirty-five surfaces on symbios-texture 0.7; the exact count is not
        // the claim, which is that every one is a Surface row and none a
        // card. A card named here would be offered as a garment texture.
        let all = SurfaceConfig::all();
        assert_eq!(all.len(), SurfaceConfig::NAMES.len());
        assert!(all.len() >= 30, "only {} surfaces", all.len());
        for name in ["Fabric", "Metal", "Chitin", "Truchet", "Encaustic"] {
            assert!(SurfaceConfig::NAMES.contains(&name), "{name} is a surface");
        }
        for card in ["Leaf", "Window", "Spark", "Flame", "GrassTuft", "ChainLink"] {
            assert!(!SurfaceConfig::NAMES.contains(&card), "{card} is a card");
        }
        for (surface, name) in all.iter().zip(SurfaceConfig::NAMES) {
            assert_eq!(surface.name(), *name);
            assert_eq!(SurfaceConfig::named(name).as_ref(), Some(surface));
        }
    }

    #[test]
    fn every_template_knows_the_type_of_every_field() {
        // The reader takes each field's type from the generator's default, so
        // a default holding a null — an `Option` left empty — would leave that
        // field's type unknown and its wire value unread. None does, and this
        // is what says so rather than an assumption.
        for name in SurfaceConfig::NAMES {
            let template = SurfaceConfig::template(name).expect("a known surface has a template");
            assert!(!has_null(&template), "{name}'s default holds a null");
        }
    }

    #[test]
    fn every_surface_crosses_the_wire_as_integers_and_comes_back_the_same() {
        for surface in SurfaceConfig::all() {
            let mut sanitized = surface.clone();
            sanitized.sanitize();
            let wire = Value::Object(sanitized.to_wire());
            assert!(
                !has_float(&wire),
                "{} wrote a float: {wire}",
                surface.name()
            );
            assert_eq!(wire["$type"], surface.name());
            let back = SurfaceConfig::from_wire(wire.as_object().expect("an object"));
            assert!(back.is_known(), "{} did not read back", surface.name());
            assert_eq!(
                back,
                sanitized,
                "{} changed across the wire",
                surface.name()
            );
            // And the sanitised form is a fixpoint.
            let mut twice = back.clone();
            twice.sanitize();
            assert_eq!(twice, back, "{} is not a sanitize fixpoint", surface.name());
        }
    }

    #[test]
    fn a_field_the_wire_leaves_out_takes_the_generators_default() {
        // What an eliding writer — symbios-overlands — sends for a config at
        // its defaults but one field.
        let mut wire = Map::new();
        wire.insert("$type".into(), Value::from("Fabric"));
        wire.insert("thread_count".into(), Value::from(400_000));
        let SurfaceConfig::Fabric(fabric) = SurfaceConfig::from_wire(&wire) else {
            panic!("a fabric reads as a fabric");
        };
        let default = symbios_texture::fabric::FabricConfig::default();
        assert!(
            (fabric.thread_count - 40.0).abs() < 1e-9,
            "{}",
            fabric.thread_count
        );
        assert!((fabric.thread_width - default.thread_width).abs() < 1e-12);
        assert_eq!(fabric.seed, default.seed);
        assert_eq!(fabric.color_warp, default.color_warp);
    }

    #[test]
    fn an_unknown_surface_is_kept_verbatim_and_written_back() {
        let mut wire = Map::new();
        wire.insert("$type".into(), Value::from("Tartan"));
        wire.insert("sett".into(), Value::from(12));
        let surface = SurfaceConfig::from_wire(&wire);
        assert!(!surface.is_known());
        assert_eq!(surface.name(), "Tartan");
        assert!(surface.generate(16).is_none(), "nothing to draw");
        let mut sanitized = surface.clone();
        sanitized.sanitize();
        assert_eq!(sanitized.to_wire(), wire, "kept exactly as it arrived");
    }

    #[test]
    fn a_known_surface_whose_fields_will_not_read_degrades_instead_of_failing() {
        // A weave this build has never heard of: the enum refuses it, and the
        // texture is kept rather than the avatar lost.
        let mut wire = Map::new();
        wire.insert("$type".into(), Value::from("Fabric"));
        wire.insert("weave".into(), Value::from("Herringbone"));
        let surface = SurfaceConfig::from_wire(&wire);
        assert!(!surface.is_known());
        assert_eq!(surface.to_wire(), wire);
    }

    #[test]
    fn an_out_of_range_integer_is_clamped_not_refused() {
        let mut wire = Map::new();
        wire.insert("$type".into(), Value::from("Fabric"));
        wire.insert("seed".into(), Value::from(-5));
        let SurfaceConfig::Fabric(fabric) = SurfaceConfig::from_wire(&wire) else {
            panic!("a negative seed is bad data, not a reason to lose the fabric");
        };
        assert_eq!(fabric.seed, 0);
    }

    #[test]
    fn sanitize_clamps_a_surface_into_its_envelope() {
        let mut surface = SurfaceConfig::Fabric(symbios_texture::fabric::FabricConfig {
            thread_width: 50.0,
            ..Default::default()
        });
        surface.sanitize();
        let SurfaceConfig::Fabric(fabric) = surface else {
            panic!("still a fabric");
        };
        assert!(fabric.thread_width <= 1.0, "{}", fabric.thread_width);
    }

    #[test]
    fn a_garment_texture_round_trips_and_sanitizes_to_a_fixpoint() {
        let mut texture = GarmentTexture {
            surface: SurfaceConfig::named("Truchet").expect("a surface"),
            scale: 900.0,
            rotation: f32::NAN,
        };
        texture.sanitize();
        assert_eq!(texture.scale, SCALE_RANGE.1);
        assert_eq!(texture.rotation, 0.0);

        let text = serde_json::to_string(&texture).expect("serialises");
        let back: GarmentTexture = serde_json::from_str(&text).expect("deserialises");
        assert_eq!(back, texture);
        let mut twice = back.clone();
        twice.sanitize();
        assert_eq!(twice, back);
        assert!(!has_float(
            &serde_json::from_str::<Value>(&text).expect("json")
        ));
    }

    #[test]
    fn a_texture_without_a_surface_is_no_texture() {
        assert!(GarmentTexture::from_wire(&serde_json::json!({"scale": 4000})).is_none());
        assert!(GarmentTexture::from_wire(&serde_json::json!({"surface": 3})).is_none());
        assert!(GarmentTexture::from_wire(&serde_json::json!("fabric")).is_none());
    }

    #[test]
    fn every_surface_generates_a_tile() {
        // The bake's premise: any surface a record can name, this build can
        // draw. Tiny tiles, because this is about the dispatch, not the art.
        for surface in SurfaceConfig::all() {
            let tile = surface
                .generate(16)
                .expect("a known surface generates")
                .unwrap_or_else(|error| panic!("{} refused 16 px: {error}", surface.name()));
            assert_eq!((tile.width, tile.height), (16, 16));
            assert_eq!(tile.albedo.len(), tile.base_len());
        }
    }
}
