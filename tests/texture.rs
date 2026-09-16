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
    // a ratchet. Raised 1950 -> 1960 when a tenth of re-rolls started wearing a
    // helmet (#351): the coin lands on seed 3, whose scalp was a crop, and a
    // helmet writes a longer name and an axis - measured, 1954.
    // Report the size, because "by how much" is the only useful thing to know
    // when a ratchet fires.
    let size = record.serialized_size().expect("serialises");
    assert!(
        size < 1960,
        "a whole avatar is {size} bytes, and should still be a couple of kilobytes"
    );
}

#[test]
fn a_beard_painted_at_full_density_is_its_own_colour() {
    // **The painted layer under a beard read as a brown shadow on the jaw**
    // (#344): a beard region's density was how much of the colour the paint
    // reached and its grain how much skin showed through, so at a density of
    // one the paint went on average 44% of the way from the skin to the hair's
    // colour. Density is coverage now. Read on the built atlas, texel by texel,
    // as how far a painted texel has gone from its own unpainted colour toward
    // the hair's, where the chin's or the flanks' mask is whole: at a density of
    // one that is all the way, and at a half it is half.
    use symbios_avatar::face::{Canon, Skull};
    use symbios_avatar::hair::{ChinStyle, FlankStyle, Follicle, Follicles, MoustacheStyle, Paint};
    use symbios_avatar::{Avatar, AvatarConfig, Vec3};
    let dressed = |density: f32, colour: [f32; 3]| {
        let mut record = AvatarRecord::new("Painted", Archetype::default());
        record.hair.chin.style = ChinStyle::None;
        record.hair.flanks.style = FlankStyle::None;
        record.hair.moustache.style = MoustacheStyle::None;
        record.hair.chin.skin = Paint { density, colour };
        record.hair.flanks.skin = Paint { density, colour };
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        (avatar, follicles)
    };
    for colour in [[0.13, 0.075, 0.043], [0.42, 0.30, 0.17]] {
        let (bare, follicles) = dressed(0.0, colour);
        let (full, _) = dressed(1.0, colour);
        let (half, _) = dressed(0.5, colour);
        let body = &bare.parts.body;
        let geometry = texture::bake(
            body,
            &bare.parts.unwrap,
            &[],
            &vec![0.0; body.vertex_count()],
            AvatarConfig::default().atlas,
        );
        assert_eq!(
            geometry.texels.len() * 4,
            bare.skin.albedo.len(),
            "the atlas baked here is not the shape of the one the body was painted on"
        );
        let hair = Vec3::from_array(colour) * 255.0;
        let rgb = |map: &symbios_texture::generator::TextureMap, at: usize| {
            Vec3::new(
                f32::from(map.albedo[at * 4]),
                f32::from(map.albedo[at * 4 + 1]),
                f32::from(map.albedo[at * 4 + 2]),
            )
        };
        for follicle in [Follicle::Chin, Follicle::Flanks] {
            let (mut shares, mut halves) = (Vec::new(), Vec::new());
            for (at, texel) in geometry.texels.iter().enumerate() {
                let Some(texel) = texel else { continue };
                if follicles.weight(follicle, texel.position - follicles.origin()) < 0.99 {
                    continue;
                }
                let skin = rgb(&bare.skin, at);
                let toward = hair - skin;
                if toward.length_squared() < 1.0 {
                    continue;
                }
                let gone = |map| (rgb(map, at) - skin).dot(toward) / toward.length_squared();
                shares.push(gone(&full.skin));
                halves.push(gone(&half.skin));
            }
            assert!(
                shares.len() > 500,
                "only {} {} texels are under a whole mask",
                shares.len(),
                follicle.name()
            );
            let mean = |all: &[f32]| all.iter().sum::<f32>() / all.len() as f32;
            shares.sort_by(f32::total_cmp);
            let lowest = shares[shares.len() / 10];
            println!(
                "{} in {colour:?}: density 1 goes {:.3} of the way (the lowest tenth {lowest:.3}), \
                 density 0.5 {:.3}",
                follicle.name(),
                mean(&shares),
                mean(&halves)
            );
            assert!(
                (mean(&shares) - 1.0).abs() <= PAINTED_SLACK && lowest >= 1.0 - 2.0 * PAINTED_SLACK,
                "the {} painted at a density of one goes {:.2} of the way to its colour, the lowest \
                 tenth {lowest:.2}",
                follicle.name(),
                mean(&shares)
            );
            assert!(
                (mean(&halves) - 0.5).abs() <= 2.0 * PAINTED_SLACK,
                "the {} painted at a density of a half goes {:.2} of the way to its colour",
                follicle.name(),
                mean(&halves)
            );
        }
    }
}

/// How far a beard region's painted texels may miss the share of the way to
/// their colour their density asks for; see
/// `a_beard_painted_at_full_density_is_its_own_colour`.
///
/// Measured at #344: 1.003 on average at a density of one, the lowest tenth at
/// 0.979 in a light brown and 0.998 in a dark one - the grain's swing in the
/// hair's own shade, and a texel's byte rounding.
const PAINTED_SLACK: f32 = 0.025;
