//! Painting must hold across the whole parameter space.
//!
//! A texture defect is the most expensive kind to find late: it shows up only
//! once something is rendered, and by then it could be the mesher, the unwrap,
//! the bake, or the paint. These checks pin the properties that must hold for
//! any body and any complexion, so a failure names its own layer.

use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, QuadrupedParams, Rig, SkinConfig, SkinParams, UvConfig,
    build_cage, catmull_clark, rig::skin, texture, unwrap,
};

/// Runs a record all the way to a painted atlas.
fn painted(
    record: &AvatarRecord,
    size: u32,
) -> (
    texture::AtlasGeometry,
    symbios_texture::generator::TextureMap,
) {
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, 1);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
    let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
    let geometry = texture::bake_geometry(&mesh, &uv, size);
    // The record's composites travel with its complexion, so this sweep
    // covers the body-composition half of the painter too (#165).
    let map = texture::paint_skin(
        &geometry,
        &rig,
        &record.skin,
        &texture::Condition::of(&record.composites),
        None,
    );
    (geometry, map)
}

#[test]
fn every_rerolled_body_paints_soundly() {
    for seed in 0..12i64 {
        for archetype in [
            Archetype::default(),
            Archetype::Quadruped(QuadrupedParams::default()),
        ] {
            let mut record = AvatarRecord::new("Painted", archetype);
            record.reroll(seed);
            let (geometry, map) = painted(&record, 96);

            assert_eq!(map.albedo.len(), map.base_len(), "seed {seed}");
            assert_eq!(map.roughness.len(), map.base_len(), "seed {seed}");
            assert_eq!(map.normal.len(), map.base_len(), "seed {seed}");
            assert!(geometry.coverage() > 0.3, "seed {seed}: atlas mostly empty");

            // Every covered texel must be painted; an unpainted one is left
            // black, which reads as a hole in the body.
            for (index, sample) in geometry.texels.iter().enumerate() {
                if sample.is_none() {
                    continue;
                }
                let at = index * 4;
                assert!(
                    map.albedo[at] > 0 || map.albedo[at + 1] > 0 || map.albedo[at + 2] > 0,
                    "seed {seed}: texel {index} was left unpainted"
                );
                assert_eq!(
                    map.albedo[at + 3],
                    255,
                    "seed {seed}: albedo must be opaque"
                );
            }
        }
    }
}

#[test]
fn skin_tone_stays_coherent_across_the_body() {
    // The failure this guards against: a per-zone term stepping abruptly where
    // two zones meet, which draws a visible line across a jaw or a wrist. Every
    // shading term must vary smoothly, so the body's overall spread of tone
    // stays narrow even though the detail varies.
    let record = AvatarRecord::default();
    let (geometry, map) = painted(&record, 192);

    let mut reds: Vec<f32> = Vec::new();
    for (index, sample) in geometry.texels.iter().enumerate() {
        if sample.is_none() {
            continue;
        }
        let at = index * 4;
        // Red minus green tracks how much blood is showing.
        reds.push(f32::from(map.albedo[at]) - f32::from(map.albedo[at + 1]));
    }
    reds.sort_by(f32::total_cmp);

    let low = reds[reds.len() / 20];
    let high = reds[reds.len() * 19 / 20];
    assert!(
        high - low < 45.0,
        "skin tone varies by {:.0} across the body, which reads as banding",
        high - low
    );
}

#[test]
fn a_complexion_survives_a_share_code() {
    let mut source = AvatarRecord {
        skin: SkinParams {
            melanin: 0.8,
            undertone: -0.6,
            blush: 0.7,
            freckles: 0.4,
        },
        ..Default::default()
    };
    source.sanitize();

    let copy = AvatarRecord::from_share_code("Copy", &source.share_code()).expect("decodes");
    let delta = |a: f32, b: f32| (a - b).abs();
    assert!(delta(copy.skin.melanin, source.skin.melanin) < 0.01);
    assert!(delta(copy.skin.undertone, source.skin.undertone) < 0.01);
    assert!(delta(copy.skin.blush, source.skin.blush) < 0.01);
    assert!(delta(copy.skin.freckles, source.skin.freckles) < 0.01);
}

#[test]
fn a_record_carries_its_complexion_through_json() {
    let mut record = AvatarRecord::default();
    record.reroll(17);
    let json = serde_json::to_string(&record).expect("serialises");
    let back: AvatarRecord = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(record.skin, back.skin);

    // And a record written before skin existed still loads, taking the default.
    let older = r#"{"name":"Older","archetype":{"$type":"network.symbios.avatar.defs#humanoid","height":1750}}"#;
    let loaded: AvatarRecord = serde_json::from_str(older).expect("older records still load");
    assert_eq!(loaded.skin, SkinParams::default());
}

#[test]
fn painting_does_not_bloat_the_record() {
    let mut record = AvatarRecord::default();
    record.reroll(3);
    assert!(record.fits_budget());
    // A ratchet on a record that should stay small rather than a budget —
    // `RECORD_BUDGET_BYTES` is that, and it is still sixty times further off.
    // Raised 800 -> 900 for the composites block (#162), and 900 -> 1900 when
    // hair became five regions in two layers (#202): measured, a rolled avatar
    // went 800 -> 1760 bytes. Five regions carrying a style, four cut axes and
    // three sRGB colours apiece is what the owner's two-colour model costs, and
    // the alternative was fewer colours. Raised 1900 -> 1950 for the three
    // chest axes (#273): measured, 1898 -> 1902, which is two bytes and three
    // field names — the cheapest thing this ratchet has ever been moved for,
    // and moved rather than widened because a ratchet that is not tight is not
    // a ratchet.
    // Report the size, because "by how much" is the only useful thing to know
    // when a ratchet fires.
    let size = record.serialized_size().expect("serialises");
    assert!(
        size < 1950,
        "a whole avatar is {size} bytes, and should still be a couple of kilobytes"
    );
}
