//! The published lexicons must describe the records this crate actually writes.
//!
//! Lexicons are a contract with every other application on the network, and one
//! that cannot be revised once anybody depends on it. A schema that drifts from
//! the implementation is worse than no schema, so the two are checked against
//! each other here rather than trusted to stay in step.

use serde_json::Value;
use symbios_avatar::hair;
use symbios_avatar::{
    Archetype, AvatarRecord, BrowStyle, ChinStyle, FlankStyle, HumanoidParams, MoustacheStyle,
    ProfileRecord, QuadrupedParams, ScalpStyle,
};

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

    // The composites (#162), whose two kinds of axis are bounded two different
    // ways and so are the easiest place in the schema for a bound to drift. The
    // shape pair carry the exploration envelope like every other shape axis;
    // `bodyFat` and `age` carry their own hard ranges, because a tripled body
    // fat fraction is negative and a tripled age is not a person.
    let composites = &defs["defs"]["composites"]["properties"];
    let signed = symbios_avatar::Composites::signed_envelope();
    for axis in ["femininity", "mass"] {
        assert_eq!(
            composites[axis]["minimum"].as_i64(),
            Some((signed.0 * 1000.0).round() as i64),
            "the declared floor on {axis} disagrees with the crate"
        );
        assert_eq!(
            composites[axis]["maximum"].as_i64(),
            Some((signed.1 * 1000.0).round() as i64),
            "the declared ceiling on {axis} disagrees with the crate"
        );
    }
    assert_eq!(
        composites["bodyFat"]["minimum"].as_i64(),
        Some((symbios_avatar::plan::BODY_FAT_RANGE.0 * 1000.0).round() as i64),
        "the declared floor on bodyFat disagrees with the crate"
    );
    assert_eq!(
        composites["bodyFat"]["maximum"].as_i64(),
        Some((symbios_avatar::plan::BODY_FAT_RANGE.1 * 1000.0).round() as i64),
        "the declared ceiling on bodyFat disagrees with the crate"
    );
    assert_eq!(
        composites["age"]["minimum"].as_u64(),
        Some(u64::from(symbios_avatar::plan::AGE_RANGE.0)),
        "the declared floor on age disagrees with the crate"
    );
    assert_eq!(
        composites["age"]["maximum"].as_u64(),
        Some(u64::from(symbios_avatar::plan::AGE_RANGE.1)),
        "the declared ceiling on age disagrees with the crate"
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
    let cases: [(&str, Value); 12] = [
        (
            // The identity anchor of the whole composite overhaul (#161): the
            // formulas are written so that THIS description reproduces the body
            // the plan built before composites existed. A default that drifts
            // here moves the anchor and silently invalidates every coefficient
            // tuned against it.
            "composites",
            serde_json::to_value(symbios_avatar::Composites::default()).expect("serialises"),
        ),
        (
            "skin",
            serde_json::to_value(symbios_avatar::SkinParams::default()).expect("serialises"),
        ),
        (
            "eyes",
            serde_json::to_value(symbios_avatar::EyeParams::default()).expect("serialises"),
        ),
        (
            "face",
            serde_json::to_value(symbios_avatar::FaceParams::default()).expect("serialises"),
        ),
        (
            "humanoid",
            serde_json::to_value(HumanoidParams::default()).expect("serialises"),
        ),
        // **The hair fragments, which is nineteen definitions arriving at once**
        // (#211). Only the FLAT ones are here: a tress and the hair block itself
        // are objects of refs and have no scalar default to promise. What they
        // owe instead is `every_declared_field_is_one_the_crate_actually_writes`
        // below, which does cover them.
        //
        // `hairPaint` and `hairCut` are declared once and referenced by all five
        // regions, so a default that drifts here drifts for the whole head.
        (
            "hairCut",
            serde_json::to_value(symbios_avatar::Cut::default()).expect("serialises"),
        ),
        (
            "hairPaint",
            serde_json::to_value(symbios_avatar::Paint::default()).expect("serialises"),
        ),
        (
            "scalpRegion",
            serde_json::to_value(hair::follicle::scalp::Params::default()).expect("serialises"),
        ),
        (
            "browRegion",
            serde_json::to_value(hair::follicle::brows::Params::default()).expect("serialises"),
        ),
        (
            "moustacheRegion",
            serde_json::to_value(hair::follicle::moustache::Params::default()).expect("serialises"),
        ),
        (
            "chinRegion",
            serde_json::to_value(hair::follicle::chin::Params::default()).expect("serialises"),
        ),
        (
            "flankRegion",
            serde_json::to_value(hair::follicle::flanks::Params::default()).expect("serialises"),
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

#[test]
fn every_hair_style_the_crate_can_write_is_declared_with_its_axis() {
    // **What `the_hair_block_is_the_one_field_the_lexicon_declines_to_declare`
    // turned into** (#211). That test held the gap open in a form that failed
    // the moment the representation became declarable, which is what happened:
    // the style enums are internally tagged now, so a style is always an object
    // with a `name` and whatever axis it carries under its own name, and the
    // lexicon says so.
    //
    // This is the check that keeps it true. A style catalogue grows an issue at
    // a time — five of them grew one each over milestone #6 — and the way that
    // goes wrong is silent: a variant lands in Rust, the record writes its name,
    // and every reader on the network that validates against the schema rejects
    // the record. So every variant the crate can construct is written out and
    // checked against what the schema declares, name and axis both.
    let defs = lexicon("defs");
    let known = |fragment: &str| -> Vec<String> {
        defs["defs"][fragment]["properties"]["name"]["knownValues"]
            .as_array()
            .unwrap_or_else(|| panic!("{fragment} declares no knownValues"))
            .iter()
            .map(|value| value.as_str().expect("names are strings").to_string())
            .collect()
    };
    let declared = |fragment: &str| property_names(&defs["defs"][fragment]);

    // Every variant of every catalogue, written out rather than iterated: the
    // enums carry no `ALL`, and a list that derived itself from the enum could
    // not catch a variant added without a thought for the wire.
    let styles: Vec<(&str, Vec<Value>)> = vec![
        (
            "scalpStyle",
            vec![
                json(ScalpStyle::None),
                json(ScalpStyle::Crop),
                json(ScalpStyle::Bob { fringe: 0.8 }),
                json(ScalpStyle::Long { weight: 0.8 }),
                json(ScalpStyle::TiedBack { tail: 0.8 }),
                json(ScalpStyle::Curly { curl: 0.8 }),
            ],
        ),
        (
            "browStyle",
            vec![
                json(BrowStyle::None),
                json(BrowStyle::Natural),
                json(BrowStyle::Thick),
            ],
        ),
        (
            "moustacheStyle",
            vec![
                json(MoustacheStyle::None),
                json(MoustacheStyle::Chevron),
                json(MoustacheStyle::Handlebar { sweep: 0.8 }),
                json(MoustacheStyle::Pencil { ride: 0.8 }),
            ],
        ),
        (
            "chinStyle",
            vec![
                json(ChinStyle::None),
                json(ChinStyle::Goatee { point: 0.8 }),
                json(ChinStyle::Full),
                json(ChinStyle::Braided { twist: 0.8 }),
            ],
        ),
        (
            "flankStyle",
            vec![
                json(FlankStyle::None),
                json(FlankStyle::Sideburns { drop: 0.8 }),
                json(FlankStyle::FullConnect { reach: 0.8 }),
            ],
        ),
    ];

    for (fragment, written) in styles {
        let names = known(fragment);
        let properties = declared(fragment);
        for style in written {
            let object = style.as_object().expect("a style writes an object");
            let name = object["name"].as_str().expect("a style writes its name");
            assert!(
                names.contains(&name.to_string()),
                "the crate writes `{fragment}` style `{name}`, which the lexicon's \
                 knownValues does not list: {names:?}"
            );
            for key in object.keys() {
                assert!(
                    properties.contains(key),
                    "`{fragment}` style `{name}` writes `{key}`, which the lexicon \
                     does not declare"
                );
            }
        }
        // And nothing is declared that no variant writes, which is the
        // direction #212 was filed for.
        assert_eq!(
            names.len(),
            defs["defs"][fragment]["properties"]["name"]["knownValues"]
                .as_array()
                .expect("knownValues")
                .len(),
            "{fragment} lists a name twice"
        );
    }
}

/// One style, as the record writes it.
fn json(style: impl serde::Serialize) -> Value {
    serde_json::to_value(style).expect("a style serialises")
}

#[test]
fn every_declared_field_is_one_the_crate_actually_writes() {
    // **The third direction, and the one that let a dead axis sit on the wire
    // for a day without a test noticing** (#212). Every other check in this
    // file runs written-to-declared or checks a required field is written; none
    // of them asks whether a DECLARED field is written at all. So
    // `defs#skin.stubble` went on being published after the painted hair layer
    // replaced what it drew (#200) and the record grew a density and a colour
    // per follicle region (#202), and the suite stayed green.
    //
    // A declared field nobody writes is not harmless. It is a promise to every
    // reader on the network that the field means something, and the cost lands
    // on whoever implements against it: they wire up a stubble slider, and it
    // does nothing, and there is no way to tell from the schema that it will.
    //
    // Checked per fragment against the struct that owns it, because a fragment
    // is exactly one Rust type's serialised shape and that is what makes the
    // comparison total rather than a spot check.
    let defs = lexicon("defs");
    let cases: [(&str, Value); 20] = [
        (
            "composites",
            serde_json::to_value(symbios_avatar::Composites::default()).expect("serialises"),
        ),
        (
            "skin",
            serde_json::to_value(symbios_avatar::SkinParams::default()).expect("serialises"),
        ),
        (
            "eyes",
            serde_json::to_value(symbios_avatar::EyeParams::default()).expect("serialises"),
        ),
        (
            "face",
            serde_json::to_value(symbios_avatar::FaceParams::default()).expect("serialises"),
        ),
        (
            "humanoid",
            serde_json::to_value(HumanoidParams::default()).expect("serialises"),
        ),
        (
            "quadruped",
            serde_json::to_value(QuadrupedParams::default()).expect("serialises"),
        ),
        // The hair fragments, nested ones included: `field_names` reads the keys
        // of whatever serialises to an object, so a tress and the hair block are
        // checked here even though they have no defaults to check above.
        (
            "hair",
            serde_json::to_value(symbios_avatar::HairRecord::default()).expect("serialises"),
        ),
        (
            "follicleRegions",
            serde_json::to_value(symbios_avatar::FollicleParams::default()).expect("serialises"),
        ),
        (
            "hairCut",
            serde_json::to_value(symbios_avatar::Cut::default()).expect("serialises"),
        ),
        (
            "hairPaint",
            serde_json::to_value(symbios_avatar::Paint::default()).expect("serialises"),
        ),
        (
            "scalpTress",
            serde_json::to_value(symbios_avatar::HairRecord::default().scalp).expect("serialises"),
        ),
        (
            "browTress",
            serde_json::to_value(symbios_avatar::HairRecord::default().brows).expect("serialises"),
        ),
        (
            "moustacheTress",
            serde_json::to_value(symbios_avatar::HairRecord::default().moustache)
                .expect("serialises"),
        ),
        (
            "chinTress",
            serde_json::to_value(symbios_avatar::HairRecord::default().chin).expect("serialises"),
        ),
        (
            "flankTress",
            serde_json::to_value(symbios_avatar::HairRecord::default().flanks).expect("serialises"),
        ),
        (
            "scalpRegion",
            serde_json::to_value(hair::follicle::scalp::Params::default()).expect("serialises"),
        ),
        (
            "browRegion",
            serde_json::to_value(hair::follicle::brows::Params::default()).expect("serialises"),
        ),
        (
            "moustacheRegion",
            serde_json::to_value(hair::follicle::moustache::Params::default()).expect("serialises"),
        ),
        (
            "chinRegion",
            serde_json::to_value(hair::follicle::chin::Params::default()).expect("serialises"),
        ),
        (
            "flankRegion",
            serde_json::to_value(hair::follicle::flanks::Params::default()).expect("serialises"),
        ),
    ];

    for (fragment, written) in cases {
        let written = field_names(&written);
        for field in property_names(&defs["defs"][fragment]) {
            assert!(
                written.contains(&field),
                "the lexicon declares `{fragment}#{field}` and the crate writes no such \
                 field: either it was renamed, or it is a dead axis that should come off \
                 the wire"
            );
        }
    }
}
