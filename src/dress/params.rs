//! What a record asks an outfit to be.
//!
//! Two garments, a top and a pair of trousers, each with the same three
//! things: an explicit colour, how far down the limbs it runs, and optionally
//! a texture to be made of. The colour and the length used to be something
//! else, and this module reads records written either way.
//!
//! ## Colour
//!
//! An sRGB triple, as hair and irises already are. It used to be a hue and a
//! shade fed through [`dye`], which never reaches black, white or a saturated
//! colour, so an outfit could not be the one thing somebody had in mind.
//!
//! ## Length
//!
//! A share of the limb, continuous. **`0` is the limb's root, `½` its middle
//! joint and `1` its end** — sleeveless, elbow, wrist for a top; hip, knee,
//! ankle for trousers — so a value names the same landmark on every body
//! whatever its proportions. Each half is a share of that segment's own bone,
//! and the hem is a ring square to the bone it crosses (see
//! [`super::garment::GarmentCut::limbs`]).
//!
//! Lengths used to be three named cuts each, on the argument that a slider
//! would spend most of its range on hems that look like mistakes. The owner
//! asked for any length (symbios-avatar #356). Trousers still stop short of
//! the crotch at [`SHORTS`]: shorter than that is what bared it before
//! (#314), so the old shortest cut is the floor.
//!
//! ## Records written before 0.10
//!
//! Carried `sleeve` and `leg` as names and `topHue`, `topShade`, `legHue` and
//! `legShade` as axes. They read here as the lengths those names stood for and
//! the colours [`dye`] made of those axes, so every published avatar keeps its
//! colours to the thousandth and its cuts to within a row of faces. A hem is
//! a ring at its length now, where it used to sit on whichever ring of faces
//! the cut stopped at: `shorts` and `calf` land where their shares always
//! said, a little below where their hems used to sit, and `forearm` — which
//! was the whole upper-arm zone, a boundary that wanders past the elbow — is a
//! ring at [`FOREARM`], its measured average. Nothing writes the old fields
//! any more.

use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::garment::dye;
use super::surface::GarmentTexture;

/// How far a top's sleeves may run: from none at all to the wrist.
pub const SLEEVE_RANGE: (f32, f32) = (0.0, 1.0);

/// How far trousers may run: from [`SHORTS`] to the ankle.
pub const TROUSER_RANGE: (f32, f32) = (SHORTS, 1.0);

/// The shortest trousers: a third of the way down the thigh.
///
/// The old `shorts` cut, which is where #314 put it: shorts that ended at the
/// pelvis zone's own boundary left the crotch bare between the hem and the
/// trunk, and a third of the thigh clears the crotch by a hand and sits well
/// above the knee on every body the envelope produces. In the length's own
/// units that is `0.35` of the upper half.
///
/// Provenance: **measured** at #314, as the thigh share it was.
pub const SHORTS: f32 = 0.35 * 0.5;

/// Mid-calf: the old `calf` cut, which took the thigh and 55% of the shin.
pub const CALF: f32 = 0.5 + 0.55 * 0.5;

/// Where the old `forearm` sleeve read to, and the top's default length.
///
/// That cut took the whole upper-arm zone, and the zone's edge is a
/// nearest-bone boundary that runs past the elbow and wanders. **Measured**
/// on seeds 1 and 9 as a position along the arm (#356): the old hem ran from
/// the elbow, at 0.50 and 0.50, to 0.75 on both, and sat at 0.62 and 0.60 on
/// average. A ring a fifth of the way down the forearm is that average, and
/// it is what a record naming the old cut reads as.
pub const FOREARM: f32 = 0.5 + 0.2 * 0.5;

/// The colour of a top when a record does not say: the old default's hue and
/// shade, dyed.
const TOP_DYE: (f32, f32) = (0.04, 0.62);

/// The colour of trousers when a record does not say, likewise.
const TROUSER_DYE: (f32, f32) = (0.61, 0.20);

/// One garment, as a record describes it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GarmentParams {
    /// The colour it is dyed, in sRGB.
    ///
    /// Free rather than a point on a ramp, as hair and irises are, and sRGB
    /// on both sides of every picker so nothing converts. A texture is
    /// multiplied by it, so white shows a texture as its generator paints it.
    #[serde(with = "crate::plan::scaled::triple")]
    pub colour: [f32; 3],
    /// How far down the limbs it runs: `0` the root, `½` the middle joint,
    /// `1` the end — see the module documentation.
    #[serde(with = "crate::plan::scaled")]
    pub length: f32,
    /// What it is made of, if not plain dyed cloth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<GarmentTexture>,
}

impl GarmentParams {
    /// Clamps every field into range, onto the wire's grid. Idempotent.
    ///
    /// `range` is the garment's own — a top and a pair of trousers do not
    /// share one — and `fallback` is what a length with no position on the
    /// axis at all becomes.
    pub fn sanitize(&mut self, range: (f32, f32), fallback: &GarmentParams) {
        use crate::plan::scaled::quantize;
        for (channel, &default) in self.colour.iter_mut().zip(&fallback.colour) {
            // A channel with no position on the axis takes the garment's own
            // default rather than zero: a black shirt is a choice.
            *channel = quantize(if channel.is_finite() {
                channel.clamp(0.0, 1.0)
            } else {
                default
            });
        }
        self.length = crate::plan::sanitize_axis(self.length, fallback.length, range);
        if let Some(texture) = &mut self.texture {
            texture.sanitize();
        }
    }
}

/// What a body is wearing.
///
/// Not `Copy`: a texture carries a whole generator config.
#[derive(Clone, Debug, PartialEq)]
pub struct OutfitParams {
    /// The top: its sleeves are its length.
    pub top: GarmentParams,
    /// The trousers: their legs are their length.
    pub trousers: GarmentParams,
}

impl Default for OutfitParams {
    fn default() -> Self {
        Self {
            top: GarmentParams {
                colour: dyed(TOP_DYE),
                length: FOREARM,
                texture: None,
            },
            trousers: GarmentParams {
                colour: dyed(TROUSER_DYE),
                length: TROUSER_RANGE.1,
                texture: None,
            },
        }
    }
}

impl OutfitParams {
    /// Clamps every field into range. Idempotent.
    pub fn sanitize(&mut self) {
        let defaults = Self::default();
        self.top.sanitize(SLEEVE_RANGE, &defaults.top);
        self.trousers.sanitize(TROUSER_RANGE, &defaults.trousers);
    }

    /// Whether either garment asks for a texture this build can draw.
    ///
    /// What decides whether a build paints a cloth atlas at all: an outfit
    /// in plain colours costs nothing beyond its geometry.
    #[must_use]
    pub fn is_textured(&self) -> bool {
        [&self.top, &self.trousers].into_iter().any(|garment| {
            garment
                .texture
                .as_ref()
                .is_some_and(|texture| texture.surface.is_known())
        })
    }
}

/// A hue and a shade, dyed and put on the wire's grid.
fn dyed((hue, shade): (f32, f32)) -> [f32; 3] {
    dye(hue, shade).map(crate::plan::scaled::quantize)
}

impl Serialize for OutfitParams {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut outfit = serializer.serialize_struct("OutfitParams", 2)?;
        outfit.serialize_field("top", &self.top)?;
        outfit.serialize_field("trousers", &self.trousers)?;
        outfit.end()
    }
}

/// An outfit exactly as the wire may carry it: both shapes at once.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Wire {
    top: Option<GarmentWire>,
    trousers: Option<GarmentWire>,
    // Written before 0.10: see the module documentation.
    sleeve: Option<Value>,
    leg: Option<Value>,
    top_hue: Option<i64>,
    top_shade: Option<i64>,
    leg_hue: Option<i64>,
    leg_shade: Option<i64>,
}

/// One garment as the wire carries it, every field optional so that a
/// partial object keeps the defaults of what it leaves out — the rule the
/// record module states for every block.
#[derive(Default, Deserialize)]
#[serde(default)]
struct GarmentWire {
    colour: Option<[i64; 3]>,
    length: Option<i64>,
    texture: Option<Value>,
}

impl GarmentWire {
    /// This garment laid over its defaults.
    ///
    /// Read wide and clamped by `sanitize` later, as every axis is. A texture
    /// that is not one — no surface in it — is dropped; see
    /// [`GarmentTexture::from_wire`].
    fn onto(self, default: GarmentParams) -> GarmentParams {
        GarmentParams {
            colour: self
                .colour
                .map_or(default.colour, |raw| raw.map(thousandths)),
            length: self.length.map_or(default.length, thousandths),
            texture: self.texture.as_ref().and_then(GarmentTexture::from_wire),
        }
    }
}

/// A wire integer in thousandths, as the axis it encodes.
#[allow(
    clippy::cast_precision_loss,
    reason = "a thousandth of any value sanitize keeps is exact"
)]
fn thousandths(raw: i64) -> f32 {
    raw as f32 / 1000.0
}

/// A pre-0.10 garment, from the names and axes it was written with.
///
/// `lengths` is the old cut vocabulary for this garment; a name outside it —
/// one a newer build of the old scheme might have added — reads as `default`'s
/// length, which is what the old reader wore it as.
fn legacy(
    cut: Option<&Value>,
    lengths: &[(&str, f32)],
    hue: Option<i64>,
    shade: Option<i64>,
    (default_hue, default_shade): (f32, f32),
    default: GarmentParams,
) -> GarmentParams {
    let length = cut
        .and_then(Value::as_str)
        .and_then(|name| lengths.iter().find(|(known, _)| *known == name))
        .map_or(default.length, |&(_, length)| length);
    let hue = hue.map_or(default_hue, thousandths);
    let shade = shade.map_or(default_shade, thousandths);
    GarmentParams {
        colour: dyed((hue, shade)),
        length,
        texture: None,
    }
}

/// The old sleeve cuts, as lengths.
const SLEEVES: [(&str, f32); 3] = [("bare", 0.0), ("forearm", FOREARM), ("wrist", 1.0)];

/// The old trouser cuts, as lengths.
const LEGS: [(&str, f32); 3] = [("shorts", SHORTS), ("calf", CALF), ("ankle", 1.0)];

impl<'de> Deserialize<'de> for OutfitParams {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;
        let defaults = OutfitParams::default();
        // A garment block wins over the old fields: a record carrying both
        // was written by this build, which never writes the old ones, so the
        // block is the newer of the two.
        let top = match wire.top {
            Some(top) => top.onto(defaults.top),
            None if wire.sleeve.is_some() || wire.top_hue.is_some() || wire.top_shade.is_some() => {
                legacy(
                    wire.sleeve.as_ref(),
                    &SLEEVES,
                    wire.top_hue,
                    wire.top_shade,
                    TOP_DYE,
                    defaults.top,
                )
            }
            None => defaults.top,
        };
        let trousers = match wire.trousers {
            Some(trousers) => trousers.onto(defaults.trousers),
            None if wire.leg.is_some() || wire.leg_hue.is_some() || wire.leg_shade.is_some() => {
                legacy(
                    wire.leg.as_ref(),
                    &LEGS,
                    wire.leg_hue,
                    wire.leg_shade,
                    TROUSER_DYE,
                    defaults.trousers,
                )
            }
            None => defaults.trousers,
        };
        Ok(Self { top, trousers })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dress::SurfaceConfig;

    #[test]
    fn the_default_outfit_wears_the_old_defaults_colours() {
        // A record written before 0.10 with no outfit block at all read as
        // hue 0.04/0.62 over 0.61/0.20; one written after reads as these
        // colours. They must be the same colours or every default body
        // changed its clothes at the upgrade.
        let outfit = OutfitParams::default();
        let old = |hue: f32, shade: f32| dye(hue, shade).map(crate::plan::scaled::quantize);
        assert_eq!(outfit.top.colour, old(0.04, 0.62));
        assert_eq!(outfit.trousers.colour, old(0.61, 0.20));
        assert_eq!(outfit.top.length, FOREARM);
        assert_eq!(outfit.trousers.length, 1.0);
        assert!(!outfit.is_textured());
    }

    #[test]
    fn an_outfit_survives_a_round_trip_through_json() {
        let mut outfit = OutfitParams::default();
        outfit.top.colour = [0.9, 0.1, 0.25];
        outfit.top.length = 0.3;
        outfit.trousers.texture = Some(GarmentTexture::new(
            SurfaceConfig::named("Fabric").expect("a surface"),
        ));
        outfit.sanitize();
        let text = serde_json::to_string(&outfit).expect("serialises");
        let back: OutfitParams = serde_json::from_str(&text).expect("deserialises");
        assert_eq!(back, outfit);
        // The new shape only: nothing writes the old fields.
        let json: Value = serde_json::from_str(&text).expect("json");
        assert!(json.get("sleeve").is_none() && json.get("topHue").is_none());
        assert_eq!(json["top"]["length"], 300);
        assert_eq!(json["top"]["colour"], serde_json::json!([900, 100, 250]));
        assert!(json["top"].get("texture").is_none(), "no texture, no key");
        assert_eq!(json["trousers"]["texture"]["surface"]["$type"], "Fabric");
    }

    #[test]
    fn a_record_written_before_0_10_keeps_its_colours_and_its_cuts() {
        let json = r#"{"sleeve":"wrist","leg":"calf","topHue":300,"topShade":800,"legHue":0,"legShade":100}"#;
        let outfit: OutfitParams = serde_json::from_str(json).expect("the old shape reads");
        let old = |hue: f32, shade: f32| dye(hue, shade).map(crate::plan::scaled::quantize);
        assert_eq!(outfit.top.colour, old(0.3, 0.8));
        assert_eq!(outfit.trousers.colour, old(0.0, 0.1));
        assert_eq!(outfit.top.length, 1.0);
        assert_eq!(outfit.trousers.length, CALF);

        for (name, length) in SLEEVES {
            let outfit: OutfitParams =
                serde_json::from_str(&format!(r#"{{"sleeve":"{name}"}}"#)).expect("reads");
            assert_eq!(outfit.top.length, length, "{name}");
            // A garment named without its colours keeps the old default dye.
            assert_eq!(outfit.top.colour, OutfitParams::default().top.colour);
        }
        for (name, length) in LEGS {
            let outfit: OutfitParams =
                serde_json::from_str(&format!(r#"{{"leg":"{name}"}}"#)).expect("reads");
            assert_eq!(outfit.trousers.length, length, "{name}");
        }
    }

    #[test]
    fn an_old_cut_this_build_never_knew_reads_as_the_default_length() {
        let json = r#"{"sleeve":"cape","leg":"culottes"}"#;
        let outfit: OutfitParams = serde_json::from_str(json).expect("unknown names load");
        let defaults = OutfitParams::default();
        assert_eq!(outfit.top.length, defaults.top.length);
        assert_eq!(outfit.trousers.length, defaults.trousers.length);
    }

    #[test]
    fn a_partial_garment_keeps_the_defaults_of_the_fields_it_omits() {
        // The record module's rule, at the garment's level: a top that names
        // only its length must not take a black shirt with it.
        let json = r#"{"top":{"length":250},"trousers":{"colour":[10,20,30]}}"#;
        let outfit: OutfitParams = serde_json::from_str(json).expect("reads");
        let defaults = OutfitParams::default();
        assert_eq!(outfit.top.length, 0.25);
        assert_eq!(outfit.top.colour, defaults.top.colour);
        assert_eq!(outfit.trousers.colour, [0.01, 0.02, 0.03]);
        assert_eq!(outfit.trousers.length, defaults.trousers.length);
    }

    #[test]
    fn sanitize_clamps_each_garment_to_its_own_range_and_is_idempotent() {
        let mut outfit = OutfitParams::default();
        outfit.top.length = -3.0;
        outfit.top.colour = [9.0, -1.0, f32::NAN];
        outfit.trousers.length = 0.0;
        outfit.sanitize();
        assert_eq!(outfit.top.length, SLEEVE_RANGE.0);
        assert_eq!(outfit.top.colour[0], 1.0);
        assert_eq!(outfit.top.colour[1], 0.0);
        assert_eq!(
            outfit.top.colour[2],
            OutfitParams::default().top.colour[2],
            "a channel with no value takes the garment's default"
        );
        assert_eq!(
            outfit.trousers.length, SHORTS,
            "trousers stop short of the crotch"
        );

        let once = outfit.clone();
        outfit.sanitize();
        assert_eq!(once, outfit, "sanitize must reach a fixpoint");
    }

    #[test]
    fn a_texture_named_by_a_newer_build_survives_a_round_trip() {
        let json = r#"{"top":{"texture":{"surface":{"$type":"Tartan","sett":4},"scale":2000,"rotation":0}}}"#;
        let mut outfit: OutfitParams = serde_json::from_str(json).expect("reads");
        outfit.sanitize();
        let texture = outfit.top.texture.as_ref().expect("kept");
        assert!(!texture.surface.is_known());
        assert!(!outfit.is_textured(), "nothing this build can draw");
        let back = serde_json::to_value(&outfit).expect("writes");
        assert_eq!(back["top"]["texture"]["surface"]["$type"], "Tartan");
        assert_eq!(back["top"]["texture"]["surface"]["sett"], 4);
    }
}
