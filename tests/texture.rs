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
    // helmet writes a longer name and an axis - measured, 1954. Raised
    // 1960 -> 1965 when the outfit became two garments of an sRGB triple and a
    // length each (#355, #356) and a re-roll started dressing the body
    // (#358): measured, 1961 — the outfit block is 96 bytes where hue, shade
    // and two cut names wrote 89. Seed 3's roll wears no texture; one that
    // does carries its whole generator config, a few hundred bytes, which is
    // the record's budget's business and not this ratchet's.
    // Report the size, because "by how much" is the only useful thing to know
    // when a ratchet fires.
    let size = record.serialized_size().expect("serialises");
    assert!(
        size < 1965,
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
/// hair's own shade, and a texel's byte rounding. Re-measured at #432, whose
/// feather reads a mask over a little ellipse of skin round each texel, so that
/// a texel whole at its centre but within that reach of the flanks' edge is
/// not quite whole: the chin 1.000 and 1.001, the lowest tenth 0.997 and 0.976;
/// the flanks 0.986 and 0.987, the lowest tenth 0.968 and 0.964, and no lower
/// under any nudge of libm the engine's gate tries. At a density of a half the
/// regions read 0.492 to 0.501 where they read 0.528 to 0.541 before, the
/// overlap under the jaw no longer laying the chin and the flanks twice.
/// Re-measured on #469's finer chart, with the feather a tenth wider: the chin
/// 0.997 in both, the lowest tenth 0.995 and 0.975; the flanks 0.994 and
/// 0.995, the lowest tenth 0.984 and 0.974, and no lower than 0.972 under a
/// nudge; at a half 0.495 to 0.499.
const PAINTED_SLACK: f32 = 0.025;

/// A person with painted hair in a colour far from their skin, everywhere it
/// may be painted, and the same person with none.
fn painted_and_bare(seed: i64) -> (symbios_avatar::Avatar, symbios_avatar::Avatar, [f32; 3]) {
    let dark = [0.06, 0.045, 0.035];
    let mut record = AvatarRecord::new("Hair", Archetype::default());
    record.reroll(seed);
    record.skin.melanin = 0.12;
    let hair = &mut record.hair;
    for paint in [
        &mut hair.scalp.skin,
        &mut hair.brows.skin,
        &mut hair.moustache.skin,
        &mut hair.chin.skin,
        &mut hair.flanks.skin,
    ] {
        paint.colour = dark;
        paint.density = 1.0;
    }
    record.sanitize();
    let mut bare = record.clone();
    let hair = &mut bare.hair;
    for paint in [
        &mut hair.scalp.skin,
        &mut hair.brows.skin,
        &mut hair.moustache.skin,
        &mut hair.chin.skin,
        &mut hair.flanks.skin,
    ] {
        paint.density = 0.0;
    }
    (
        symbios_avatar::Avatar::build(&record).expect("a person builds"),
        symbios_avatar::Avatar::build(&bare).expect("a person builds"),
        dark,
    )
}

/// An sRGB channel in linear light.
fn linear(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// An atlas's albedo in linear light, and the levels a GPU's mip chain halves
/// it into: each texel the mean of the four under it.
fn mip_levels(
    map: &symbios_texture::generator::TextureMap,
    count: usize,
) -> Vec<(usize, Vec<glam::Vec3>)> {
    let side = map.width as usize;
    let base = map.albedo[..side * side * 4]
        .chunks(4)
        .map(|texel| {
            glam::Vec3::new(
                f32::from(texel[0]),
                f32::from(texel[1]),
                f32::from(texel[2]),
            )
            .map(|channel| linear(channel / 255.0))
        })
        .collect();
    let mut levels: Vec<(usize, Vec<glam::Vec3>)> = vec![(side, base)];
    while levels.len() < count {
        let (side, texels) = &levels[levels.len() - 1];
        let half = side / 2;
        let at = |x: usize, y: usize| texels[y * side + x];
        let next = (0..half * half)
            .map(|index| {
                let (x, y) = (index % half * 2, index / half * 2);
                (at(x, y) + at(x + 1, y) + at(x, y + 1) + at(x + 1, y + 1)) * 0.25
            })
            .collect();
        levels.push((half, next));
    }
    levels
}

/// The texels of an atlas `side` texels square whose centres a part's
/// triangles cover.
fn texels_of(part: &symbios_avatar::PolyMesh, side: usize) -> Vec<usize> {
    let scale = side as f32;
    let mut out = std::collections::BTreeSet::new();
    for face in &part.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let t = [face[0], face[corner], face[corner + 1]].map(|v| part.uvs[v as usize] * scale);
            let area = (t[1] - t[0]).perp_dot(t[2] - t[0]);
            if area.abs() < 1e-9 {
                continue;
            }
            let (lo, hi) = (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]));
            for y in (lo.y.floor().max(0.0) as usize)..=(hi.y.ceil() as usize).min(side - 1) {
                for x in (lo.x.floor().max(0.0) as usize)..=(hi.x.ceil() as usize).min(side - 1) {
                    let centre = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let one = (t[1] - centre).perp_dot(t[2] - centre) / area;
                    let two = (t[2] - centre).perp_dot(t[0] - centre) / area;
                    if one >= 0.0 && two >= 0.0 && one + two <= 1.0 {
                        out.insert(y * side + x);
                    }
                }
            }
        }
    }
    out.into_iter().collect()
}

#[test]
fn a_person_s_ears_and_lids_take_no_painted_hair() {
    // **Hair colour bled onto a person's ears** (#471, the owner's report). An
    // ear is charted with the head and stands beside the scalp and the
    // sideburn, and the painted layer asked its masks by position alone: the
    // scalp's line runs over the ear's middle at the side and drops to the nape
    // behind it, so nine tenths of every ear went the hair's colour. A lid took
    // a brow's edge the same way. Read on the built atlas against the same body
    // with no painted hair, each texel of a part as its share of the way from
    // the bare skin toward the hair's colour, at the atlas's own level and the
    // two mip levels under it.
    //
    // Measured at #471, on the first level: an ear on seed 42 the whole way,
    // nine in ten of its texels past a quarter of it; a lid up to 0.40 of it on
    // seed 237. Since, nothing, at all three levels. (Further down the chain a
    // neighbouring chart's hair reaches a part's rim through the gutter between
    // them, as it reaches every chart's; the skin atlas is drawn without mips.)
    for seed in [42, 237, 175] {
        let (painted, bare, hair) = painted_and_bare(seed);
        let hair = glam::Vec3::from_array(hair).map(linear);
        let (with, without) = (mip_levels(&painted.skin, 3), mip_levels(&bare.skin, 3));
        let mut parts: Vec<(&str, &symbios_avatar::PolyMesh)> = Vec::new();
        for ear in &painted
            .parts
            .features
            .as_ref()
            .expect("a person has ears")
            .ears
        {
            parts.push(("an ear", ear));
        }
        for (lid, _) in painted
            .parts
            .eyes
            .as_ref()
            .expect("a person has eyes")
            .lids()
        {
            parts.push(("a lid", lid));
        }
        for (level, ((side, ours), (_, theirs))) in with.iter().zip(&without).enumerate() {
            for (name, part) in &parts {
                let texels = texels_of(part, *side);
                assert!(
                    !texels.is_empty(),
                    "seed {seed}: {name} covers no texel at level {level}"
                );
                let worst = texels
                    .iter()
                    .map(|&at| {
                        let toward = hair - theirs[at];
                        (ours[at] - theirs[at]).dot(toward) / toward.length_squared().max(1e-6)
                    })
                    .fold(0.0f32, f32::max);
                assert!(
                    worst < 0.02,
                    "seed {seed}: {name} goes {worst:.2} of the way to the hair's colour at mip \
                     level {level}"
                );
            }
        }
    }
}

#[test]
fn a_painted_scalp_is_as_grained_on_the_crown_as_behind_it() {
    // **A disc of crisp stubble on the crown** (#469's fixer). The painted
    // scalp's grain fades by how finely its texel is sampled, and a person's
    // head chart closes on its crown, whose last rings come out two to three
    // times finer than the scalp below; so the grain drew at full strength
    // there and at half or less elsewhere, and from above a bald or shaved head
    // wore a disc of speckle. Read on the built atlas as each texel's step from
    // the mean of its four neighbours, in sRGB levels of luminance, over the
    // crown (within 15 degrees of straight up from the head joint) against the
    // back of the scalp (50 to 70 degrees down, behind the joint): the crown's
    // over the back's, measured before the fix at 1.21, 1.71 and 1.42 on these
    // rolls and after it at 0.98, 1.08 and 0.99.
    let mut read = Vec::new();
    for seed in [42, 175, 237] {
        let (painted, _, _) = painted_and_bare(seed);
        let map = &painted.skin;
        let geometry = symbios_avatar::texture::bake_geometry(
            &painted.parts.body,
            &painted.parts.unwrap,
            map.width,
        );
        let joint = painted.rig.in_zone(symbios_avatar::Zone::Head)[0];
        let origin = painted.rig.joints[joint].position;
        let width = map.width as usize;
        let luminance = |at: usize| {
            let rgb = &map.albedo[at * 4..at * 4 + 3];
            0.3 * f32::from(rgb[0]) + 0.59 * f32::from(rgb[1]) + 0.11 * f32::from(rgb[2])
        };
        let on_head = |at: usize| {
            geometry.texels[at].is_some_and(|texel| texel.zone == symbios_avatar::Zone::Head)
        };
        let (mut crown, mut back) = ((0.0f32, 0u32), (0.0f32, 0u32));
        for (at, texel) in geometry.texels.iter().enumerate() {
            let Some(texel) = texel else { continue };
            let (x, y) = (at % width, at / width);
            if texel.zone != symbios_avatar::Zone::Head || x == 0 || y == 0 || x + 1 >= width {
                continue;
            }
            let around = [at - 1, at + 1, at - width, at + width];
            if around
                .iter()
                .any(|&near| near >= geometry.texels.len() || !on_head(near))
            {
                continue;
            }
            let local = texel.position - origin;
            let down = local
                .normalize_or_zero()
                .y
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            let step = (luminance(at)
                - around.iter().map(|&near| luminance(near)).sum::<f32>() / 4.0)
                .abs();
            if down < 15.0 {
                crown = (crown.0 + step, crown.1 + 1);
            } else if (50.0..70.0).contains(&down) && local.z < 0.0 {
                back = (back.0 + step, back.1 + 1);
            }
        }
        assert!(
            crown.1 > 0 && back.1 > 0,
            "seed {seed}: no crown or no back read"
        );
        let ratio = (crown.0 / crown.1 as f32) / (back.0 / back.1 as f32);
        println!("seed {seed}: the crown's grain {ratio:.2} of the back's");
        read.push((seed, ratio));
    }
    for (seed, ratio) in read {
        assert!(
            ratio <= 1.15,
            "seed {seed}: the crown's painted grain is {ratio:.2} of the back of the scalp's"
        );
    }
}
