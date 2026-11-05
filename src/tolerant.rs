//! Reading a variant this build does not know (#419).
//!
//! **A built body crosses between two builds of this crate.** A browser's main
//! thread and the Web Worker that builds its bodies are fetched apart, and a
//! cached worker can be a release older or newer than the page that runs it;
//! every client also builds the bodies of everyone it sees with its own engine
//! and draws what the others send. So the day a later release adds a kind of
//! head, a zone, a role or a draw, a build that predates it must still read a
//! body that carries one - and a derived `Deserialize` refuses the whole body
//! over one name it has not heard of.
//!
//! Every enum a sent [`crate::Avatar`] carries is read through here instead,
//! and a name it does not know reads as a **fallback its own type documents**:
//! the variant an older build draws most sensibly in its place. Nothing here
//! adds a public variant, so a downstream `match` stays exhaustive, and the
//! WRITING side is still the derived `Serialize`, so the bytes on the wire are
//! exactly what they were.
//!
//! **The decode asks the format what it holds** (`deserialize_any`), because
//! a variant this build does not know may carry content - a newtype variant
//! is written as a one-entry map, `{"name": content}`, by both the codecs a
//! built body travels by, `serde_json` and `rmp-serde` - and only a format
//! that describes itself can say how much to skip. Both of them do. A format
//! that does not, `bincode`'s, cannot carry a sent avatar across two builds
//! anyway, as it could not skip a field it does not know either.
//!
//! The record's own open unions are not here and do not change: an
//! [`crate::Archetype`] whose `$type` this build does not know reads as
//! `Unknown`, and a share code's unknown tag is an error.

use std::fmt;

use serde::Deserializer;
use serde::de::{Error, IgnoredAny, MapAccess, Visitor};

/// Reads an enum of unit variants, with `fallback` for a name not in `names`.
///
/// `names` holds every variant under the name its derived `Serialize` writes;
/// a test beside each enum holds the two to agree. A known name written with
/// content, `{"name": null}`, reads as that variant, as a derived decode reads
/// it from JSON; an unknown one's content is skipped whatever it is.
///
/// # Errors
///
/// Whatever the format raises on malformed input, and an error for a value
/// that is neither a name nor a one-entry map.
pub(crate) fn unit<'de, D, T>(
    deserializer: D,
    expecting: &'static str,
    names: &'static [(&'static str, T)],
    fallback: T,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Copy,
{
    deserializer.deserialize_any(Unit {
        expecting,
        names,
        fallback,
    })
}

/// The variant written as `name`, if `names` has it.
pub(crate) fn lookup<T: Copy>(names: &[(&'static str, T)], name: &str) -> Option<T> {
    names
        .iter()
        .find(|(known, _)| *known == name)
        .map(|&(_, value)| value)
}

/// Reads the one key of a variant written with content, or says why not.
///
/// # Errors
///
/// The format's, and an error for an empty map.
pub(crate) fn only_key<'de, A: MapAccess<'de>>(
    map: &mut A,
    expecting: &dyn serde::de::Expected,
) -> Result<String, A::Error> {
    map.next_key::<String>()?
        .ok_or_else(|| A::Error::invalid_length(0, expecting))
}

/// Refuses a second entry in a map that should hold one variant.
///
/// # Errors
///
/// The format's, and an error for a second key.
pub(crate) fn no_more<'de, A: MapAccess<'de>>(
    map: &mut A,
    expecting: &dyn serde::de::Expected,
) -> Result<(), A::Error> {
    if map.next_key::<IgnoredAny>()?.is_some() {
        return Err(A::Error::invalid_length(2, expecting));
    }
    Ok(())
}

/// [`unit()`]'s visitor.
struct Unit<T: 'static> {
    expecting: &'static str,
    names: &'static [(&'static str, T)],
    fallback: T,
}

impl<'de, T: Copy> Visitor<'de> for Unit<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_str<E: Error>(self, name: &str) -> Result<T, E> {
        Ok(lookup(self.names, name).unwrap_or(self.fallback))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<T, A::Error> {
        let name = only_key(&mut map, &self)?;
        map.next_value::<IgnoredAny>()?;
        no_more(&mut map, &self)?;
        Ok(lookup(self.names, &name).unwrap_or(self.fallback))
    }
}

/// Writes a tolerant `Deserialize` for an enum of unit variants.
///
/// `fallback` names the variant an unknown name reads as. Every variant must be
/// listed with the name its `Serialize` writes: the generated match is
/// exhaustive, so a variant added to the enum and not here does not compile.
macro_rules! tolerant_unit {
    ($(#[$attr:meta])* $kind:ident, $expecting:literal, fallback $fallback:ident,
     { $($variant:ident => $name:literal),+ $(,)? }) => {
        $(#[$attr])*
        impl<'de> serde::Deserialize<'de> for $kind {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                // Exhaustive, so a variant added to the enum must be added here too.
                #[allow(dead_code)]
                fn listed(kind: $kind) {
                    match kind {
                        $($kind::$variant => {}),+
                    }
                }
                const NAMES: &[(&str, $kind)] = &[$(($name, $kind::$variant)),+];
                $crate::tolerant::unit(deserializer, $expecting, NAMES, $kind::$fallback)
            }
        }
    };
}

pub(crate) use tolerant_unit;

/// A test's unknown variant, in the two forms a newer build can write one:
/// a bare name, and a name carrying content.
#[cfg(test)]
pub(crate) fn unknowns() -> [serde_json::Value; 3] {
    [
        serde_json::json!("aKindNoBuildHasYet"),
        serde_json::json!({ "aKindNoBuildHasYet": "foreLeft" }),
        serde_json::json!({ "aKindNoBuildHasYet": { "span": 3, "of": [1, 2] } }),
    ]
}

/// Decodes `value` as `T` through JSON and through msgpack as the worker
/// writes it (`rmp_serde::to_vec_named`), and says both read the same.
///
/// # Panics
///
/// If either refuses it, or they disagree.
#[cfg(test)]
pub(crate) fn through_both<T>(value: &serde_json::Value) -> T
where
    T: serde::de::DeserializeOwned + PartialEq + fmt::Debug,
{
    let json: T = serde_json::from_str(&value.to_string()).expect("JSON reads it");
    let packed = rmp_serde::to_vec_named(value).expect("msgpack writes it");
    let msgpack: T = rmp_serde::from_slice(&packed).expect("msgpack reads it");
    assert_eq!(json, msgpack, "the two codecs read {value} alike");
    json
}

/// Every variant in `all` writes as a derived `Serialize` does and reads back
/// as itself through both codecs: the table a tolerant decode reads agrees
/// with the names the enum writes.
///
/// # Panics
///
/// If any does not.
#[cfg(test)]
pub(crate) fn round_trips<T>(all: &[T])
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + fmt::Debug + Copy,
{
    for &kind in all {
        let value = serde_json::to_value(kind).expect("writes");
        assert_eq!(through_both::<T>(&value), kind, "{value} reads back");
        let packed = rmp_serde::to_vec_named(&kind).expect("msgpack writes it");
        let back: T = rmp_serde::from_slice(&packed).expect("msgpack reads its own");
        assert_eq!(back, kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
    enum Probe {
        One,
        Two,
    }

    tolerant_unit!(Probe, "a probe", fallback Two, { One => "One", Two => "Two" });

    #[test]
    fn a_name_this_build_does_not_know_reads_as_the_fallback_in_either_form() {
        round_trips(&[Probe::One, Probe::Two]);
        for unknown in unknowns() {
            assert_eq!(through_both::<Probe>(&unknown), Probe::Two, "{unknown}");
        }
        // A known name written with content reads as itself.
        assert_eq!(
            through_both::<Probe>(&serde_json::json!({ "One": null })),
            Probe::One
        );
    }

    #[test]
    fn what_is_not_a_variant_at_all_is_still_refused() {
        for wrong in [
            serde_json::json!(3),
            serde_json::json!({}),
            serde_json::json!({ "One": null, "Two": null }),
            serde_json::json!([1]),
        ] {
            assert!(
                serde_json::from_value::<Probe>(wrong.clone()).is_err(),
                "{wrong}"
            );
        }
    }

    #[test]
    fn a_value_after_the_variant_is_read_where_it_should_be() {
        // The content of an unknown variant is skipped, not left for the next
        // field to trip on.
        #[derive(Debug, PartialEq, serde::Deserialize)]
        struct Holder {
            kind: Probe,
            after: u32,
        }
        let value = serde_json::json!({ "kind": { "Three": [1, { "a": 2 }] }, "after": 7 });
        assert_eq!(
            through_both::<Holder>(&value),
            Holder {
                kind: Probe::Two,
                after: 7
            }
        );
    }
}
