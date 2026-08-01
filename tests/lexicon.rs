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

    let mut profile = ProfileRecord::pointing_at("3lm2k4x");
    profile.created_at = Some("2026-08-01T00:00:00Z".into());
    for field in field_names(&profile) {
        assert!(
            declared.contains(&field),
            "profile writes `{field}`, which the lexicon does not declare"
        );
    }
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
    for (fragment, range) in [
        ("humanoid", symbios_avatar::plan::humanoid_height_range()),
        ("quadruped", symbios_avatar::plan::quadruped_height_range()),
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
}
