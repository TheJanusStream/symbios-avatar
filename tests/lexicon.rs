//! The published lexicons must describe the records this crate actually writes.
//!
//! Lexicons are a contract with every other application on the network, and one
//! that cannot be revised once anybody depends on it. A schema that drifts from
//! the implementation is worse than no schema, so the two are checked against
//! each other here rather than trusted to stay in step.

use serde_json::Value;
use symbios_avatar::{Archetype, AvatarRecord, HumanoidParams, ProfileRecord, QuadrupedParams};

/// Loads one published lexicon document.
fn lexicon(name: &str) -> Value {
    let path = format!(
        "{}/lexicons/network/symbios/avatar/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {path}: {e}"))
}

/// Property names a schema object declares.
fn property_names(schema: &Value) -> Vec<String> {
    schema["properties"]
        .as_object()
        .expect("schema has properties")
        .keys()
        .cloned()
        .collect()
}

/// Field names a serialised value carries.
fn field_names(value: &impl serde::Serialize) -> Vec<String> {
    let json = serde_json::to_value(value).expect("serialises");
    json.as_object()
        .expect("serialises to an object")
        .keys()
        .filter(|key| *key != "$type")
        .cloned()
        .collect()
}

#[test]
fn lexicon_ids_match_the_constants_the_crate_publishes() {
    assert_eq!(
        lexicon("avatar")["id"].as_str(),
        Some(symbios_avatar::record::AVATAR_NSID)
    );
    assert_eq!(
        lexicon("profile")["id"].as_str(),
        Some(symbios_avatar::record::PROFILE_NSID)
    );
    assert_eq!(
        lexicon("defs")["id"].as_str(),
        Some("network.symbios.avatar.defs")
    );
}

#[test]
fn the_avatar_record_matches_its_schema() {
    let schema = lexicon("avatar")["defs"]["main"]["record"].clone();
    let declared = property_names(&schema);

    let mut record = AvatarRecord::new("Schema", Archetype::default());
    record.created_at = Some("2026-08-01T00:00:00Z".into());
    for field in field_names(&record) {
        assert!(
            declared.contains(&field),
            "record writes `{field}`, which the lexicon does not declare"
        );
    }
}

#[test]
fn the_profile_record_matches_its_schema() {
    let schema = lexicon("profile")["defs"]["main"]["record"].clone();
    let declared = property_names(&schema);

    let profile = ProfileRecord::pointing_at("3lm2k4x", "2026-08-01T00:00:00Z");
    for field in field_names(&profile) {
        assert!(
            declared.contains(&field),
            "profile writes `{field}`, which the lexicon does not declare"
        );
    }
}

/// Field names a schema object marks required.
fn required_names(schema: &Value) -> Vec<String> {
    schema["required"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .map(|name| {
                    name.as_str()
                        .expect("required names are strings")
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_required_field_is_actually_written() {
    // The direction that was missing, and the reason a whole class of invalid
    // record shipped: the checks above run written-to-declared, which passes
    // happily when a required field is never written at all. A PDS that
    // resolves the lexicon rejects such a record outright.
    let avatar =
        AvatarRecord::new("Complete", Archetype::default()).created("2026-08-01T00:00:00Z");
    assert_eq!(avatar.publishable(), Ok(()));
    let written = field_names(&avatar);
    for field in required_names(&lexicon("avatar")["defs"]["main"]["record"]) {
        assert!(
            written.contains(&field),
            "the lexicon requires `{field}`, which the avatar record does not write"
        );
    }

    let profile = ProfileRecord::pointing_at("3lm2k4x", "2026-08-01T00:00:00Z");
    assert_eq!(profile.publishable(), Ok(()));
    let written = field_names(&profile);
    for field in required_names(&lexicon("profile")["defs"]["main"]["record"]) {
        assert!(
            written.contains(&field),
            "the lexicon requires `{field}`, which the profile record does not write"
        );
    }
}

#[test]
fn a_record_missing_a_required_field_says_so_before_it_is_published() {
    // The counterpart: the crate can hold a partial record — that is what an
    // in-progress creator has — but it must not let one be written as if it
    // were complete.
    let unstamped = AvatarRecord::new("Draft", Archetype::default());
    assert!(unstamped.publishable().is_err());
    assert!(
        !field_names(&unstamped).contains(&"createdAt".to_string()),
        "an unstamped record must not invent a timestamp"
    );
}

#[test]
fn an_unknown_archetype_round_trips_through_the_union() {
    // The archetype union is declared open, and an open union is only open if a
    // reader survives a variant it has never seen. WS6 adds creature
    // archetypes, so this is a dated promise rather than a hypothetical.
    let json = r#"{"name":"Hexapod","archetype":{"$type":"network.symbios.avatar.defs#hexapod","legs":6}}"#;
    let record: AvatarRecord = serde_json::from_str(json).expect("an unknown body still loads");
    assert!(!record.archetype.is_understood());

    let back = serde_json::to_value(&record).expect("serialises");
    assert_eq!(
        back["archetype"]["$type"],
        "network.symbios.avatar.defs#hexapod"
    );
    assert_eq!(
        back["archetype"]["legs"], 6,
        "the unknown body was rewritten lossily"
    );
}

#[test]
fn every_archetype_variant_is_declared_and_named_by_its_ref() {
    let defs = lexicon("defs");
    let refs = lexicon("avatar")["defs"]["main"]["record"]["properties"]["archetype"]["refs"]
        .as_array()
        .expect("archetype is a union")
        .iter()
        .map(|r| r.as_str().expect("ref is a string").to_string())
        .collect::<Vec<_>>();

    for archetype in [
        Archetype::Humanoid(HumanoidParams::default()),
        Archetype::Quadruped(QuadrupedParams::default()),
    ] {
        let json = serde_json::to_value(&archetype).expect("serialises");
        let tag = json["$type"]
            .as_str()
            .expect("tagged with $type")
            .to_string();

        assert!(
            refs.contains(&tag),
            "`{tag}` is written but not listed in the archetype union"
        );

        let (_, fragment) = tag.split_once('#').expect("refs point at a definition");
        let schema = &defs["defs"][fragment];
        assert!(
            !schema.is_null(),
            "`{fragment}` is not defined in defs.json"
        );

        let declared = property_names(schema);
        for field in field_names(&archetype) {
            assert!(
                declared.contains(&field),
                "{fragment} writes `{field}`, which its schema does not declare"
            );
        }
    }
}

#[test]
fn axes_are_written_as_integers_because_atproto_has_no_float_type() {
    // The AT Protocol data model deliberately omits floats so records have one
    // canonical encoding. Writing one would produce records other software on
    // the network cannot represent.
    let record = AvatarRecord::new("Integers", Archetype::default());
    let json = serde_json::to_value(&record).expect("serialises");

    fn assert_no_floats(value: &Value, path: &str) {
        match value {
            Value::Number(number) => assert!(
                number.is_i64() || number.is_u64(),
                "{path} is a float: {number}"
            ),
            Value::Object(map) => {
                for (key, child) in map {
                    assert_no_floats(child, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    assert_no_floats(child, &format!("{path}[{index}]"));
                }
            }
            _ => {}
        }
    }

    assert_no_floats(&json, "record");
}

#[test]
fn declared_axis_bounds_match_the_ranges_the_crate_enforces() {
    let defs = lexicon("defs");
    // The range the crate ENFORCES is the exploration envelope (#160), which
    // is what `sanitize` clamps to — the conservative `*_height_range()`
    // constants still exist but are the envelope's inputs, not its bounds.
    for (fragment, range) in [
        (
            "humanoid",
            symbios_avatar::HumanoidParams::height_envelope(),
        ),
        (
            "quadruped",
            symbios_avatar::QuadrupedParams::height_envelope(),
        ),
    ] {
        let height = &defs["defs"][fragment]["properties"]["height"];
        assert_eq!(
            height["minimum"].as_i64(),
            Some((range.0 * 1000.0).round() as i64),
            "{fragment} minimum height disagrees with the crate"
        );
        assert_eq!(
            height["maximum"].as_i64(),
            Some((range.1 * 1000.0).round() as i64),
            "{fragment} maximum height disagrees with the crate"
        );
    }

    // How many locks the rim of the hair breaks into. This used to be `groups`,
    // the count of strand groups, and it was the one axis a record could spend
    // the whole avatar's triangle budget through. The mass is a shell now and
    // costs what the head's size dictates (#68), so the bounds here are about
    // what still reads as hair rather than about what is affordable.
    let locks = &defs["defs"]["hair"]["properties"]["locks"];
    assert_eq!(
        locks["minimum"].as_u64(),
        Some(u64::from(symbios_avatar::hair::MIN_LOCKS)),
        "the declared floor on hair locks disagrees with the crate"
    );
    assert_eq!(
        locks["maximum"].as_u64(),
        Some(u64::from(symbios_avatar::hair::MAX_LOCKS)),
        "the declared ceiling on hair locks disagrees with the crate"
    );
}

#[test]
fn declared_defaults_match_the_values_the_crate_writes() {
    // A schema's stated default is a promise to every reader that omits the
    // field. This drifted the moment a default changed in Rust — the hair group
    // count went to 128 while the lexicon still said 96 — and nothing caught it,
    // because every other check here is about names and bounds.
    let defs = lexicon("defs");
    let declared = |fragment: &str| -> Value { defs["defs"][fragment]["properties"].clone() };

    // **`face` and `humanoid` were missing from this list, and adding two axes
    // to each is what found it** (#61). Every check above runs over the names a
    // schema declares; this one is the only one that reads a schema's promised
    // VALUE, and the two definitions carrying the most axes were exempt from it.
    // A default that drifts is invisible to every reader who omits the field and
    // to nobody else, which is the hardest kind of schema defect to notice.
    let cases: [(&str, Value); 5] = [
        (
            "skin",
            serde_json::to_value(symbios_avatar::SkinParams::default()).expect("serialises"),
        ),
        (
            "eyes",
            serde_json::to_value(symbios_avatar::EyeParams::default()).expect("serialises"),
        ),
        (
            "hair",
            serde_json::to_value(symbios_avatar::HairParams::default()).expect("serialises"),
        ),
        (
            "face",
            serde_json::to_value(symbios_avatar::FaceParams::default()).expect("serialises"),
        ),
        (
            "humanoid",
            serde_json::to_value(HumanoidParams::default()).expect("serialises"),
        ),
    ];

    for (fragment, written) in cases {
        let schema = declared(fragment);
        let written = written.as_object().expect("serialises to an object");
        for (field, value) in written {
            let Some(declared) = schema[field].get("default") else {
                panic!("{fragment}#{field} declares no default");
            };
            assert_eq!(
                declared, value,
                "{fragment}#{field}: the lexicon promises {declared} but the crate writes {value}"
            );
        }
    }
}
