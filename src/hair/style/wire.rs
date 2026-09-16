//! How a head of hair crosses the wire when it names a style this build does
//! not know (#351).
//!
//! The style enums are internally tagged, and a derived internally tagged enum
//! has exactly one answer to a tag it does not recognise: an error, which
//! fails the [`HairRecord`], which fails the whole avatar record. That is what
//! 0.8.1 does with every name 0.9.0 added (measured), and it is the opposite
//! of the rule the record module states for every other token - an unknown
//! value degrades rather than fails. A `#[serde(other)]` unit variant would
//! degrade, but would drop the name on the next write, which deletes
//! somebody's haircut the first time an older client touches their avatar.
//!
//! So the record goes through a [`serde_json::Value`] on the way in and out.
//! On the way in, each region's style object is checked by NAME against the
//! enum's own list, and an unknown one is set aside and replaced by `none`
//! before the derived decode runs; on the way out, a set-aside object is put
//! back while its region still wears `none`. Only an unknown NAME degrades: a
//! known name with a malformed axis is still an error, as it was.
//!
//! The detour costs nothing a record depends on. [`crate::AvatarRecord`]
//! already needs a self-describing format for its flattened `extra`, and a
//! record is a couple of kilobytes.

use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use super::{BrowStyle, ChinStyle, FlankStyle, HairRecord, MoustacheStyle, ScalpStyle, Tress};
use crate::hair::FollicleParams;

/// The style objects a record named that this build does not know, one slot a
/// region, each kept verbatim.
///
/// See [`HairRecord::unrecognised`]. `None` in a slot means the region's style
/// was one this build knows, which is every slot on every record this build
/// wrote itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Unrecognised {
    /// The scalp's style object, if its name was unknown.
    pub scalp: Option<Map<String, Value>>,
    /// The brows' style object, likewise.
    pub brows: Option<Map<String, Value>>,
    /// The moustache's style object, likewise.
    pub moustache: Option<Map<String, Value>>,
    /// The chin's style object, likewise.
    pub chin: Option<Map<String, Value>>,
    /// The flanks' style object, likewise.
    pub flanks: Option<Map<String, Value>>,
}

impl Unrecognised {
    /// Whether no region named a style this build does not know.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.scalp.is_none()
            && self.brows.is_none()
            && self.moustache.is_none()
            && self.chin.is_none()
            && self.flanks.is_none()
    }
}

/// The record's fields exactly as the wire carries them: what the derive on
/// [`HairRecord`] was before #351, so every name, default and axis encoding is
/// the one the lexicon already declares.
#[derive(Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Wire {
    regions: FollicleParams,
    scalp: Tress<ScalpStyle>,
    brows: Tress<BrowStyle>,
    moustache: Tress<MoustacheStyle>,
    chin: Tress<ChinStyle>,
    flanks: Tress<FlankStyle>,
}

impl Default for Wire {
    /// A missing block or region reads as [`HairRecord::default`]'s, as the
    /// container-level default on the old derive did.
    fn default() -> Self {
        let hair = HairRecord::default();
        Self {
            regions: hair.regions,
            scalp: hair.scalp,
            brows: hair.brows,
            moustache: hair.moustache,
            chin: hair.chin,
            flanks: hair.flanks,
        }
    }
}

/// Sets aside the style object at `region` if its name is not in `names`,
/// leaving `none` in its place.
fn set_aside(
    hair: &mut Map<String, Value>,
    region: &str,
    names: &[&str],
) -> Option<Map<String, Value>> {
    let style = hair.get_mut(region)?.as_object_mut()?.get_mut("style")?;
    let name = style.get("name")?.as_str()?;
    if names.contains(&name) {
        return None;
    }
    let mut none = Map::new();
    none.insert("name".into(), Value::String("none".into()));
    match std::mem::replace(style, Value::Object(none)) {
        Value::Object(kept) => Some(kept),
        // `style.get("name")` above only answers for an object.
        _ => None,
    }
}

impl<'de> Deserialize<'de> for HairRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut value = Value::deserialize(deserializer)?;
        let mut unrecognised = Unrecognised::default();
        if let Value::Object(hair) = &mut value {
            unrecognised.scalp = set_aside(hair, "scalp", ScalpStyle::NAMES);
            unrecognised.brows = set_aside(hair, "brows", BrowStyle::NAMES);
            unrecognised.moustache = set_aside(hair, "moustache", MoustacheStyle::NAMES);
            unrecognised.chin = set_aside(hair, "chin", ChinStyle::NAMES);
            unrecognised.flanks = set_aside(hair, "flanks", FlankStyle::NAMES);
        }
        let wire = Wire::deserialize(value).map_err(D::Error::custom)?;
        Ok(Self {
            regions: wire.regions,
            scalp: wire.scalp,
            brows: wire.brows,
            moustache: wire.moustache,
            chin: wire.chin,
            flanks: wire.flanks,
            unrecognised,
        })
    }
}

impl Serialize for HairRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = Wire {
            regions: self.regions,
            scalp: self.scalp,
            brows: self.brows,
            moustache: self.moustache,
            chin: self.chin,
            flanks: self.flanks,
        };
        if self.unrecognised.is_empty() {
            return wire.serialize(serializer);
        }
        let mut value = serde_json::to_value(&wire).map_err(S::Error::custom)?;
        let kept = &self.unrecognised;
        for (region, style, still_none) in [
            ("scalp", &kept.scalp, self.scalp.style == ScalpStyle::None),
            ("brows", &kept.brows, self.brows.style == BrowStyle::None),
            (
                "moustache",
                &kept.moustache,
                self.moustache.style == MoustacheStyle::None,
            ),
            ("chin", &kept.chin, self.chin.style == ChinStyle::None),
            (
                "flanks",
                &kept.flanks,
                self.flanks.style == FlankStyle::None,
            ),
        ] {
            if let (Some(style), true) = (style, still_none) {
                value[region]["style"] = Value::Object(style.clone());
            }
        }
        value.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_with(region: &str, style: Value) -> Value {
        let mut value = serde_json::to_value(HairRecord::default()).expect("serialises");
        value[region]["style"] = style;
        value
    }

    #[test]
    fn an_unknown_style_name_draws_nothing_and_survives_a_rewrite() {
        for region in ["scalp", "brows", "moustache", "chin", "flanks"] {
            let style = serde_json::json!({ "name": "from_a_newer_build", "width": 700 });
            let mut value = record_with(region, style.clone());
            value[region]["cut"]["length"] = serde_json::json!(900);
            let hair: HairRecord = serde_json::from_value(value).expect("an unknown name loads");

            // Drawn as nothing, with the rest of the region kept.
            let (grows, length) = match region {
                "scalp" => (hair.scalp.style != ScalpStyle::None, hair.scalp.cut.length),
                "brows" => (hair.brows.style != BrowStyle::None, hair.brows.cut.length),
                "moustache" => (
                    hair.moustache.style != MoustacheStyle::None,
                    hair.moustache.cut.length,
                ),
                "chin" => (hair.chin.style != ChinStyle::None, hair.chin.cut.length),
                _ => (
                    hair.flanks.style != FlankStyle::None,
                    hair.flanks.cut.length,
                ),
            };
            assert!(!grows, "{region}: an unknown name was drawn as a style");
            assert!(
                (length - 0.9).abs() < 1e-6,
                "{region}: the cut was not kept ({length})"
            );
            assert!(
                !hair.unrecognised.is_empty(),
                "{region}: the name was not kept"
            );

            // And written back exactly, name and axis.
            let back = serde_json::to_value(&hair).expect("serialises");
            assert_eq!(
                back[region]["style"], style,
                "{region}: the rewrite lost the style"
            );
            let again: HairRecord = serde_json::from_value(back).expect("reloads");
            assert_eq!(
                again, hair,
                "{region}: a second round trip moved the record"
            );
        }
    }

    #[test]
    fn a_style_chosen_over_an_unknown_one_replaces_it() {
        let value = record_with("scalp", serde_json::json!({ "name": "from_a_newer_build" }));
        let mut hair: HairRecord = serde_json::from_value(value).expect("loads");
        hair.scalp.style = ScalpStyle::Crop;
        let back = serde_json::to_value(&hair).expect("serialises");
        assert_eq!(
            back["scalp"]["style"],
            serde_json::json!({ "name": "crop" })
        );
    }

    #[test]
    fn a_known_name_is_not_set_aside_and_a_malformed_one_still_fails() {
        // Control: every name the crate writes is decoded as itself, and a
        // record this build wrote keeps nothing aside.
        let hair = HairRecord {
            scalp: Tress {
                style: ScalpStyle::Bun { height: 0.5 },
                ..Tress::default()
            },
            brows: Tress {
                style: BrowStyle::Sculpted,
                ..Tress::default()
            },
            moustache: Tress {
                style: MoustacheStyle::Sculpted { flare: 0.5 },
                ..Tress::default()
            },
            chin: Tress {
                style: ChinStyle::Sculpted { length: 0.5 },
                ..Tress::default()
            },
            flanks: Tress {
                style: FlankStyle::Sculpted,
                ..Tress::default()
            },
            ..HairRecord::default()
        };
        let back: HairRecord =
            serde_json::from_value(serde_json::to_value(&hair).expect("serialises"))
                .expect("loads");
        assert_eq!(back, hair);
        assert!(back.unrecognised.is_empty());

        // Only an unknown NAME degrades: a known one with a broken axis is the
        // error it always was.
        let broken = record_with(
            "scalp",
            serde_json::json!({ "name": "bun", "height": "tall" }),
        );
        assert!(serde_json::from_value::<HairRecord>(broken).is_err());
    }

    #[test]
    fn a_missing_block_still_reads_as_the_default_head_of_hair() {
        let hair: HairRecord = serde_json::from_str("{}").expect("loads");
        assert_eq!(hair, HairRecord::default());
        let scalp_only: HairRecord =
            serde_json::from_str(r#"{"scalp":{"style":{"name":"bob","fringe":400}}}"#)
                .expect("loads");
        assert_eq!(scalp_only.brows, HairRecord::default().brows);
    }
}
