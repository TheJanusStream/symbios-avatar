//! Unwrapping must hold across the whole parameter space.
//!
//! A UV defect is invisible until something is painted on the body, and then it
//! is catastrophic and hard to trace: a chart overlapping its neighbour puts one
//! body part's texture on another, and a smeared face stretches a few texels
//! across a whole limb. Both are cheap to check here and expensive to find later.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, Chart, HumanoidParams, PolyMesh, QuadrupedParams, Rig,
    SkinConfig, UvConfig, UvUnwrap, Zone, build_cage, catmull_clark, rig::skin, unwrap,
};

/// Runs a record all the way to an unwrapped atlas.
fn atlas(record: &AvatarRecord, levels: usize) -> (PolyMesh, UvUnwrap) {
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, levels);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
    let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
    (mesh, uv)
}

/// Area a face covers in the atlas.
fn uv_area(uv: &UvUnwrap, face: &[u32]) -> f32 {
    let mut doubled = 0.0;
    for corner in 0..face.len() {
        let a = uv.uvs[face[corner] as usize];
        let b = uv.uvs[face[(corner + 1) % face.len()] as usize];
        doubled += a.x * b.y - b.x * a.y;
    }
    (doubled * 0.5).abs()
}

/// Area a face covers on the body.
fn world_area(mesh: &PolyMesh, face_index: usize) -> f32 {
    let face = &mesh.faces[face_index];
    if face.len() < 3 {
        return 0.0;
    }
    let anchor = mesh.positions[face[0] as usize];
    (1..face.len() - 1)
        .map(|corner| {
            let a = mesh.positions[face[corner] as usize] - anchor;
            let b = mesh.positions[face[corner + 1] as usize] - anchor;
            a.cross(b).length() * 0.5
        })
        .sum()
}

/// How far a texel reaches each way over the first triangle of `face`: the
/// determinant of the map from the body to the atlas, and its two singular
/// values, the most and the least. `None` for a triangle with no area.
fn texel_reach(uv: &UvUnwrap, body: &PolyMesh, face: &[u32]) -> Option<(f32, f32, f32)> {
    use symbios_avatar::Vec2;
    let corners = [face[0], face[1], face[2]];
    let at = corners.map(|v| body.positions[uv.source[v as usize] as usize]);
    let (edge, other) = (at[1] - at[0], at[2] - at[0]);
    let normal = edge.cross(other);
    if normal.length_squared() < 1e-20 {
        return None;
    }
    let (x, y) = (edge.normalize(), normal.cross(edge).normalize());
    let flat = glam::Mat2::from_cols(
        Vec2::new(edge.dot(x), edge.dot(y)),
        Vec2::new(other.dot(x), other.dot(y)),
    );
    let atlas = corners.map(|v| uv.uvs[v as usize]);
    let map = glam::Mat2::from_cols(atlas[1] - atlas[0], atlas[2] - atlas[0]) * flat.inverse();
    let sum = map.x_axis.length_squared() + map.y_axis.length_squared();
    let area = map.determinant().abs();
    let spread = (sum * sum - 4.0 * area * area).max(0.0).sqrt();
    Some((
        area,
        ((sum + spread) / 2.0).sqrt(),
        ((sum - spread) / 2.0).max(0.0).sqrt(),
    ))
}

/// Checks every invariant an atlas must satisfy, whatever the body.
#[track_caller]
fn assert_sound_atlas(mesh: &PolyMesh, uv: &UvUnwrap, what: &str) {
    assert_eq!(
        uv.faces.len(),
        mesh.faces.len(),
        "{what}: faces went missing"
    );
    assert!(!uv.charts.is_empty(), "{what}: nothing was charted");

    for (index, coordinate) in uv.uvs.iter().enumerate() {
        assert!(
            (0.0..=1.0).contains(&coordinate.x) && (0.0..=1.0).contains(&coordinate.y),
            "{what}: vertex {index} lands outside the atlas at {coordinate:?}"
        );
    }

    for (index, chart) in uv.charts.iter().enumerate() {
        for other in &uv.charts[index + 1..] {
            assert!(
                !chart.rect.overlaps(&other.rect),
                "{what}: {:?} and {:?} share texels",
                chart.zone,
                other.zone
            );
        }
    }

    for (index, face) in uv.faces.iter().enumerate() {
        let chart = uv.charts[uv.chart_of_face[index] as usize];
        let world = world_area(mesh, uv.source_face[index] as usize);
        if world <= 0.0 {
            continue;
        }
        let stretch = uv_area(uv, face) / world / chart.texel_density().max(1e-9);
        // **11.0, up from 8.0, and it is three faces on a tail** (#107). Eight-
        // point cage rings doubled the faces around every tube, and at the tip of
        // a quadruped's tapering tail the worst of them went from 5.5x to 11.0x
        // over thirty seeds. `Kind::Cylindrical` sets a chart's texel density
        // from its MEAN radius, so a face out where the tail has tapered to
        // almost nothing measures as stretched however it is unwrapped; halving
        // the world area of each face without changing the chart doubles the
        // reading. Nothing else on either body exceeds 10.6x, and nothing outside the tail
        // exceeds 5.3x.
        assert!(
            stretch < 11.5,
            "{what}: face {index} in {:?} is stretched {stretch:.1}x",
            chart.zone
        );
    }
}

#[test]
fn every_rerolled_body_unwraps_soundly() {
    for seed in 0..30i64 {
        let mut biped = AvatarRecord::new("Biped", Archetype::default());
        biped.reroll(seed);
        let (mesh, uv) = atlas(&biped, 1);
        assert_sound_atlas(&mesh, &uv, &format!("biped seed {seed}"));

        let mut beast =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        let (mesh, uv) = atlas(&beast, 1);
        assert_sound_atlas(&mesh, &uv, &format!("quadruped seed {seed}"));
    }
}

#[test]
fn extreme_bodies_unwrap_soundly() {
    let mut rng = Pcg64Mcg::seed_from_u64(0x0A71A5);
    for sample in 0..25 {
        let mut params = HumanoidParams {
            height: rng.random_range(1.2..=2.2),
            shoulder_width: rng.random_range(-1.0..=1.0),
            hip_width: rng.random_range(-1.0..=1.0),
            limb_length: rng.random_range(-1.0..=1.0),
            neck_length: rng.random_range(-1.0..=1.0),
            head_size: rng.random_range(-1.0..=1.0),
            head_breadth: rng.random_range(-1.0..=1.0),
            face_length: rng.random_range(-1.0..=1.0),
            extremity_size: rng.random_range(-1.0..=1.0),
            chest_volume: rng.random_range(-1.0..=1.0),
            chest_projection: rng.random_range(-1.0..=1.0),
            chest_lift: rng.random_range(-1.0..=1.0),
            chest_spacing: rng.random_range(-1.0..=1.0),
            chest_fullness: rng.random_range(-1.0..=1.0),
        };
        use symbios_avatar::BodyPlan;
        params.sanitize();

        let record = AvatarRecord::new("Extreme", Archetype::Humanoid(params));
        let (mesh, uv) = atlas(&record, 1);
        assert_sound_atlas(
            &mesh,
            &uv,
            &format!("random humanoid #{sample}: {params:?}"),
        );
    }
}

#[test]
fn every_body_of_a_plan_charts_the_same_zones() {
    // The contract a procedural painter relies on: it addresses charts by the
    // body part they cover, and every part it knows about is present. The chart
    // *count* is deliberately not part of that contract — a zone can be
    // genuinely disconnected, and how many pieces it falls into depends on the
    // body's proportions.
    let mut short = AvatarRecord::default();
    short.reroll(4);
    let mut tall = AvatarRecord::default();
    tall.reroll(9);

    let charted = |record: &AvatarRecord| -> Vec<Zone> {
        let (_, uv) = atlas(record, 1);
        let mut zones: Vec<Zone> = uv.charts.iter().map(|chart| chart.zone).collect();
        zones.sort();
        zones.dedup();
        zones
    };
    assert_eq!(charted(&short), charted(&tall));

    // And nothing the body declares goes unpainted.
    let skeleton = short.skeleton();
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let zones = charted(&short);
    for joint in &rig.joints {
        assert!(
            zones.contains(&joint.zone),
            "{:?} has no chart, so it could never be painted",
            joint.zone
        );
    }
}

#[test]
fn the_face_and_hands_are_given_the_most_texels() {
    let record = AvatarRecord::default();
    let (_, uv) = atlas(&record, 2);

    let density = |zone: Zone| -> f32 {
        uv.charts
            .iter()
            .filter(|chart| chart.zone == zone)
            .map(Chart::texel_density)
            .fold(0.0, f32::max)
    };

    let body = density(Zone::Abdomen);
    assert!(density(Zone::Head) > body * 1.5, "the face needs detail");
    assert!(
        density(Zone::Extremity(symbios_avatar::Limb::ForeLeft)) > body,
        "so do hands"
    );
}

#[test]
fn the_atlas_is_not_mostly_empty() {
    let record = AvatarRecord::default();
    let (_, uv) = atlas(&record, 2);
    let used: f32 = uv.charts.iter().map(|chart| chart.rect.area()).sum();
    assert!(used > 0.45, "packing wasted the atlas: only {used:.2} used");
}

#[test]
fn duplication_stays_proportionate() {
    // Every chart boundary and seam costs duplicated vertices. Some is the price
    // of a zoned atlas; a lot would mean charts are fragmenting.
    let record = AvatarRecord::default();
    let (mesh, uv) = atlas(&record, 2);
    let extra = uv.vertex_count() - mesh.vertex_count();
    assert!(
        extra < mesh.vertex_count() / 2,
        "unwrapping duplicated {extra} of {} vertices",
        mesh.vertex_count()
    );
}

#[test]
fn a_beast_s_legs_are_charted_as_finely_as_its_barrel() {
    // A beast's upper arm and thigh lie inside its trunk, so the skin the
    // zone map gave each leg ran up over the shoulder and the flank, and
    // wrapped round a bone inside the trunk - a wall beside the bone, not a
    // tube round it - that skin came out at a third to a half of the barrel's
    // density, stretched two to three to one, and painted as streaked fur in
    // a crescent over the elbow (#376). Charted with the trunk it wraps, and
    // the leg below round the bone leaving the elbow and the stifle, every
    // leg is as fine as the barrel and nearly square.
    //
    // Fore and hind apart, and the worst tenth as well as the thinnest: a
    // horse's thigh is half again as deep at the stifle as it is across, and
    // the two pairs averaged hid a hind leg whose chart had taken in the fold
    // over the stifle and the groin past the midline, stretched up to twenty
    // to one, while the forelegs came out better than ever. And over sixty
    // rolls, every family and their hybrids: six seeds held it on average.
    use symbios_avatar::plan::Limb;
    use symbios_avatar::{Avatar, AvatarConfig};
    // The charts do not depend on the atlas's size; the painting does.
    let config = AvatarConfig {
        atlas: 64,
        ..AvatarConfig::default()
    };
    let mut worst = [(f32::INFINITY, 0i64, "fore"), (f32::INFINITY, 0, "hind")];
    let mut stretched = [(0.0f32, 0i64, "fore"), (0.0, 0, "hind")];
    for seed in 0..60i64 {
        let mut record =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        record.reroll(seed);
        let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
        let (uv, body) = (&avatar.parts.unwrap, &avatar.parts.body);
        let mut barrel = Vec::new();
        let mut legs: [(Vec<f32>, Vec<f32>); 2] = Default::default();
        for (index, face) in uv.faces.iter().enumerate() {
            let Some((area, most, least)) = texel_reach(uv, body, face) else {
                continue;
            };
            match uv.charts[uv.chart_of_face[index] as usize].zone {
                Zone::Chest | Zone::Abdomen | Zone::Pelvis => barrel.push(area.sqrt()),
                Zone::UpperLimb(limb) => {
                    let pair = usize::from(matches!(limb, Limb::HindLeft | Limb::HindRight));
                    legs[pair].0.push(area.sqrt());
                    legs[pair].1.push(most / least.max(1e-9));
                }
                _ => {}
            }
        }
        barrel.sort_by(f32::total_cmp);
        let share = |list: &[f32], at: f32| list[((list.len() - 1) as f32 * at) as usize];
        let middle = share(&barrel, 0.5);
        for (pair, (mut density, mut stretch)) in legs.into_iter().enumerate() {
            density.sort_by(f32::total_cmp);
            stretch.sort_by(f32::total_cmp);
            let thinnest = share(&density, 0.1) / middle;
            if thinnest < worst[pair].0 {
                worst[pair] = (thinnest, seed, worst[pair].2);
            }
            let reach = share(&stretch, 0.9);
            if reach > stretched[pair].0 {
                stretched[pair] = (reach, seed, stretched[pair].2);
            }
        }
    }
    // At the slice: the thinnest tenth 0.69 fore and 0.67 hind, the worst
    // tenth stretched 3.0 and 3.6 to one. Before it, 0.25 and 0.25, and 22
    // and 32; with the hind thigh measured in its deepest radius 0.33 and 21
    // to one, and with the fold over the stifle charted with the leg 0.39
    // and 12. With the junctions wrapped under the trunk (#387), 0.62 and 0.68,
    // and 4.15 and 2.92: the hind legs better, a cat's fore legs worse on its
    // worst rolls, as its elbow stands above the chest's floor and the ring
    // reaches further to it. The two worst, seeds 19 and 40, show nothing of it
    // in a render, and the wrap alone holds it at 4.0, so these bounds moved
    // with it rather than the wrap: from 0.62 and 4.0. Broadening the legs' outer
    // walls (#394) leaves 0.66 and 0.70, and 4.33 and 2.90, within the same bounds.
    for (thinnest, seed, pair) in worst {
        assert!(
            thinnest > 0.6,
            "seed {seed}: a tenth of the {pair} legs is charted at {thinnest:.2} of the barrel"
        );
    }
    for (reach, seed, pair) in stretched {
        assert!(
            reach < 4.5,
            "seed {seed}: a tenth of the {pair} legs' texels is stretched {reach:.2} to one"
        );
    }
}

#[test]
fn a_beast_s_tail_is_charted_along_its_bones() {
    // A tail wrapped round the bone leaving the hips runs across that axis
    // wherever it curls away from it, so the whole of its upper length came out
    // at a handful of the chart's heights: a few texels along a tail whose
    // coat is painted in rings along it, and a striped cat's tail showed a dark
    // line down its top instead of its rings (#397). Charted along its bones,
    // its texels are as nearly square as the barrel's, and as fine.
    use symbios_avatar::{Avatar, AvatarConfig};
    let config = AvatarConfig {
        atlas: 64,
        ..AvatarConfig::default()
    };
    let (mut stretched, mut thinnest) = ((0.0f32, 0i64), (f32::INFINITY, 0i64));
    for seed in 0..40i64 {
        let mut record =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        record.reroll(seed);
        let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
        let (uv, body) = (&avatar.parts.unwrap, &avatar.parts.body);
        let (mut barrel, mut density, mut stretch) = (Vec::new(), Vec::new(), Vec::new());
        for (index, face) in uv.faces.iter().enumerate() {
            let Some((area, most, least)) = texel_reach(uv, body, face) else {
                continue;
            };
            match uv.charts[uv.chart_of_face[index] as usize].zone {
                Zone::Chest | Zone::Abdomen => barrel.push(area.sqrt()),
                Zone::Tail => {
                    density.push(least);
                    stretch.push(most / least.max(1e-9));
                }
                _ => {}
            }
        }
        for list in [&mut barrel, &mut density, &mut stretch] {
            list.sort_by(f32::total_cmp);
        }
        let share = |list: &[f32], at: f32| list[((list.len() - 1) as f32 * at) as usize];
        let reach = share(&stretch, 0.9);
        if reach > stretched.0 {
            stretched = (reach, seed);
        }
        let thin = share(&density, 0.1) / share(&barrel, 0.5);
        if thin < thinnest.0 {
            thinnest = (thin, seed);
        }
    }
    // At the slice: the worst tenth stretched 2.11 to one, the thinnest tenth
    // 0.47 of the barrel, which is the last centimetre, where the tail rounds
    // over its tip. Wrapped round the first bone, 28 to one and 0.05; unrolled
    // by angle alone, so a tapering tail's tip spread across the whole chart,
    // 5.3 to one and 0.31.
    let (reach, seed) = stretched;
    assert!(
        reach < 2.5,
        "seed {seed}: a tenth of a tail's texels is stretched {reach:.2} to one"
    );
    let (thin, seed) = thinnest;
    assert!(
        thin > 0.4,
        "seed {seed}: a tenth of a tail is charted at {thin:.2} of the barrel"
    );
}

/// Where on a person's head a point lies, for
/// `a_person_s_head_is_charted_as_finely_behind_and_under_it_as_its_face`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Face,
    Mouth,
    Eyes,
    UnderTheJaw,
    BehindTheCheek,
    Sideburn,
    Scalp,
    Nape,
    /// The frontmost skin of the face: the nose and the lips, which stand out
    /// of it and face the side as much as forward.
    Nose,
    /// The flanks of the nose and the cheek beside them (#473): from the base
    /// of the nose to just under the eyes, more than a centimetre off the
    /// midline and within most of an eye-width of it, in the front of the face.
    NoseFlanks,
}

/// Which [`Part`]s a head-local point is in: by the head's own landmarks, the
/// jawline the face was carved to, the scalp's hairline, the ear's centre and
/// the eye line, as the follicle regions are cut.
fn parts_at(
    follicles: &symbios_avatar::Follicles,
    canon: &symbios_avatar::Canon,
    local: symbios_avatar::Vec3,
) -> Vec<Part> {
    let reach = (local.x * local.x + local.z * local.z).sqrt();
    let facing = if reach > 1e-6 { local.z / reach } else { 1.0 };
    let (ear, frame) = (canon.ear_centre(), canon.frame);
    if local.y < follicles.jawline(facing) && facing > -0.35 {
        return vec![Part::UnderTheJaw];
    }
    if follicles.weight(symbios_avatar::Follicle::Scalp, local) >= 0.5 {
        return vec![Part::Scalp];
    }
    if facing < -0.35 {
        return vec![Part::Nape];
    }
    if facing <= 0.45 && local.y < ear - 0.1 * frame {
        return vec![Part::BehindTheCheek];
    }
    if facing <= 0.55 && local.y >= ear - 0.1 * frame {
        return vec![Part::Sideburn];
    }
    let mut parts = vec![Part::Face];
    // The mouth's height, as `face::features` places it.
    if facing > 0.7 && (local.y - canon.down(0.596)).abs() < 0.08 * frame {
        parts.push(Part::Mouth);
    }
    if facing > 0.5 && (local.y - canon.level).abs() < 0.07 * frame {
        parts.push(Part::Eyes);
    }
    if facing > 0.85
        && local.y >= canon.nose_base()
        && local.y <= canon.level - 0.1 * frame
        && local.x.abs() > 0.010
        && local.x.abs() < 0.8 * canon.unit
    {
        parts.push(Part::NoseFlanks);
    }
    parts
}

#[test]
fn a_person_s_head_is_charted_as_finely_behind_and_under_it_as_its_face() {
    // **The head's chart spent four to eight millimetres of skin a texel under
    // the jaw and behind the cheek** (#469). It was wrapped round its vertices'
    // centroid, which the refined face pulls to just behind the lips, so the
    // whole back of the head had a sliver of the way round; and the plane under
    // the chin faces down the neck, so by height alone it had a few rows. The
    // hairline behind the ear stepped, and a beard's edge.
    //
    // Read on the 1024-texel atlas as the skin the coarser way of each texel
    // spans - the larger of the two the triangle's map from the atlas onto the
    // skin stretches a texel to - by the skin each triangle covers, pooled over
    // three rolls and two corners: the median, and the ninetieth percentile
    // under the jaw. Measured before #469, at it, and after its fixer, in
    // millimetres: under the jaw 5.2, 2.6 and 2.7, at the ninetieth 29.4, 4.4
    // and 4.7; behind the cheek 6.4, 3.5 and 3.3; at the sideburn 5.9, 3.6 and
    // 3.5; over the scalp 8.3, 3.9 and 3.7; at the nape 7.9, 3.5 and 3.3. And
    // the face's own, which its importance holds: the face 2.12, 1.80 and 1.72,
    // its ninetieth 5.33, 3.43 and 3.36; the eyes 2.19, 1.96 and 1.94; the mouth
    // 1.37, 1.55 and 1.46, the rows its detail drew from every column of the
    // old chart.
    //
    // **And the nose and the lips** (#469's fixer): round the head joint the
    // skin that stands out of the face and faces the side, the nose's flanks
    // and the philtrum's edges, runs nearly along the line from the axis, and
    // whole triangles of the nose's side came out one texel. Read as the worst
    // hundredth of the frontmost sixth of the head facing forward, 3.49 before
    // #469, 8.61 at it and 4.66 after its fixer; the old chart's was its
    // centroid's luck, standing just behind the lips, and the back of the head
    // paid for it. Under a nudge of libm the bodies move and these with them,
    // at most: the face 1.81, the eyes 2.03, the mouth 1.46, under the jaw 4.71
    // at the ninetieth, the scalp 3.81, the nose 4.72.
    //
    // **And the face a chart of its own** (#473), flattened the way it runs,
    // with #474's packer: the face 1.32 at the median and 2.08 at the
    // ninetieth, the eyes 1.31, the mouth 1.39, under the jaw 1.72 and 3.27,
    // behind the cheek 2.81, at the sideburn 2.96, over the scalp 3.39, at the
    // nape 2.87, the nose's worst hundredth 2.86. And the nose's flanks, read
    // by every triangle: 2.17 at their worst hundredth, against 3.58 at 0.24.4
    // and 6.65 at 0.24.5 read the same way. Under a nudge, at most: the face
    // 1.34, the eyes 1.33, the mouth 1.41, under the jaw 3.31 at the
    // ninetieth, the scalp 3.44, the nose 2.90, its flanks 2.21. On its
    // fixer's charts, the face reaching up over the forehead, the crown its own
    // and the band round the back with no hole, the face 1.37 and 1.97, the
    // eyes 1.36, the mouth 1.37, under the jaw 1.71 and 2.88, behind the cheek
    // 2.84, at the sideburn 2.76, over the scalp 2.86, at the nape 2.88, the
    // nose's worst hundredth 2.81 and its flanks' 2.22; under a nudge at most
    // the face 1.38, under the jaw 2.91 at the ninetieth, the flanks 2.22.
    // With the mouth's band magnified, the skin under the jaw asking less, the
    // band round the back weighted on narrow heads and the head asking more
    // of the atlas (#473's second fixer), the face 1.46 and 2.08, the eyes
    // 1.45, the mouth 1.23, under the jaw 1.96 and 2.88, behind the cheek 2.66,
    // at the sideburn 2.58, over the scalp 2.62, at the nape 2.76, the nose's
    // worst hundredth 3.17 and its flanks' 2.25; under a nudge the flanks' at
    // most 2.32.
    let corner = |edit: fn(&mut symbios_avatar::HumanoidParams)| {
        let mut record = AvatarRecord::default();
        if let Archetype::Humanoid(params) = &mut record.archetype {
            edit(params);
        }
        record
    };
    let mut records: Vec<AvatarRecord> = [175, 237, 9]
        .into_iter()
        .map(|seed| {
            let mut record = AvatarRecord::default();
            record.reroll(seed);
            record
        })
        .collect();
    records.push(corner(|params| params.head_size = -1.0));
    records.push(corner(|params| params.face_length = 1.0));

    let size = 1024.0;
    let mut read: Vec<(Part, f32, f32)> = Vec::new();
    let mut ahead: Vec<(symbios_avatar::Vec3, f32, f32)> = Vec::new();
    for record in &records {
        let avatar = symbios_avatar::Avatar::build(record).expect("a person builds");
        let body = &avatar.parts.body;
        let skull = symbios_avatar::face::Skull::measure(body, &avatar.rig).expect("a head");
        let canon = symbios_avatar::Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles =
            symbios_avatar::Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        let uv = &avatar.parts.unwrap;
        for (index, face) in uv.faces.iter().enumerate() {
            if uv.charts[uv.chart_of_face[index] as usize].zone != Zone::Head {
                continue;
            }
            // Each face is read by its first triangle, but the nose's flanks by
            // every triangle of every face: their polygons are fanned round
            // the face's finer faces, and which triangle comes first there is a
            // lottery of needles - read by the first alone, 0.24.4's flanks,
            // which a render shows fine, came out at 14.9 mm a texel at their
            // worst hundredth.
            for fan in 1..face.len() - 1 {
                let triangle = [face[0], face[fan], face[fan + 1]];
                let Some((_, _, finest)) = texel_reach(uv, body, &triangle) else {
                    continue;
                };
                if finest <= 0.0 {
                    continue;
                }
                let corners = triangle.map(|v| body.positions[uv.source[v as usize] as usize]);
                let skin = (corners[1] - corners[0])
                    .cross(corners[2] - corners[0])
                    .length()
                    * 0.5;
                let local = (corners[0] + corners[1] + corners[2]) / 3.0 - follicles.origin();
                // The coarser step: one texel over the least the map stretches.
                let step = 1000.0 / (finest * size);
                for part in parts_at(&follicles, &canon, local) {
                    if part == Part::NoseFlanks || fan == 1 {
                        read.push((part, step, skin));
                    }
                }
                if fan == 1 {
                    ahead.push((local, step, skin));
                }
            }
        }
        // The nose: skin facing within a third of the way round from straight
        // ahead, in the frontmost sixth of the head's depth - the nose and the
        // lips.
        let front = ahead.iter().map(|row| row.0.z).fold(f32::MIN, f32::max);
        for (local, step, skin) in ahead.drain(..) {
            let reach = (local.x * local.x + local.z * local.z).sqrt();
            if reach > 1e-6 && local.z / reach > 0.85 && local.z > 0.85 * front {
                read.push((Part::Nose, step, skin));
            }
        }
    }
    let at = |part: Part, share: f32| {
        let mut rows: Vec<(f32, f32)> = read
            .iter()
            .filter(|row| row.0 == part)
            .map(|row| (row.1, row.2))
            .collect();
        rows.sort_by(|one, two| one.0.total_cmp(&two.0));
        let total: f32 = rows.iter().map(|row| row.1).sum();
        let mut run = 0.0;
        rows.iter()
            .find(|row| {
                run += row.1;
                run >= share * total
            })
            .map_or(f32::INFINITY, |row| row.0)
    };
    let reading = [
        Part::Face,
        Part::Mouth,
        Part::Eyes,
        Part::UnderTheJaw,
        Part::BehindTheCheek,
        Part::Sideburn,
        Part::Scalp,
        Part::Nape,
    ]
    .map(|part| (part, at(part, 0.5), at(part, 0.9)));
    for (part, median, ninetieth) in reading {
        println!("{part:?}: {median:.2} mm a texel at the median, {ninetieth:.2} at the ninetieth");
    }
    let nose = (at(Part::Nose, 0.5), at(Part::Nose, 0.99));
    println!(
        "Nose: {:.2} mm a texel at the median, {:.2} at the ninety-ninth",
        nose.0, nose.1
    );
    let flanks = (at(Part::NoseFlanks, 0.5), at(Part::NoseFlanks, 0.99));
    println!(
        "NoseFlanks: {:.2} mm a texel at the median, {:.2} at the ninety-ninth",
        flanks.0, flanks.1
    );
    let [face, mouth, eyes, under, behind, sideburn, scalp, nape] = reading;
    assert!(
        under.2 <= 6.0,
        "a tenth of the skin under the jaw spans {:.1} mm a texel",
        under.2
    );
    for (part, median, _) in [under, behind, sideburn, scalp, nape] {
        assert!(
            median <= 4.5,
            "{part:?} spans {median:.2} mm a texel at the median"
        );
    }
    assert!(
        face.1 <= 2.0 && face.2 <= 3.8 && eyes.1 <= 2.15 && mouth.1 <= 1.75,
        "the face spans {:.2} mm a texel at the median and {:.2} at the ninetieth, the eyes {:.2} \
         and the mouth {:.2}",
        face.1,
        face.2,
        eyes.1,
        mouth.1
    );
    assert!(
        nose.1 <= 5.0,
        "a hundredth of the nose and the lips spans {:.2} mm a texel",
        nose.1
    );
    assert!(
        flanks.1 <= 2.35,
        "a hundredth of the nose's flanks spans {:.2} mm a texel",
        flanks.1
    );
}

/// The texels of an atlas `size` texels square that two of `triangles` cover,
/// and the atlas area, in texels, of those laid the other way round from
/// most of them.
fn laid_twice(triangles: &[[symbios_avatar::Vec2; 3]], size: f32) -> (usize, f32) {
    use symbios_avatar::Vec2;
    let signed = |a: Vec2, b: Vec2, c: Vec2| 0.5 * (b - a).perp_dot(c - a);
    let way: f32 = triangles.iter().map(|t| signed(t[0], t[1], t[2])).sum();
    let flipped: f32 = triangles
        .iter()
        .map(|t| signed(t[0] * size, t[1] * size, t[2] * size))
        .filter(|area| area * way < 0.0)
        .map(f32::abs)
        .sum();
    let low = triangles
        .iter()
        .flatten()
        .fold(Vec2::splat(f32::MAX), |low, at| low.min(*at * size))
        .floor();
    let high = triangles
        .iter()
        .flatten()
        .fold(Vec2::splat(f32::MIN), |high, at| high.max(*at * size))
        .ceil();
    let (width, height) = ((high.x - low.x) as usize + 1, (high.y - low.y) as usize + 1);
    let mut cover = vec![0u8; width * height];
    for t in triangles {
        let p = t.map(|at| at * size - low);
        let area = signed(p[0], p[1], p[2]);
        if area.abs() < 1e-12 {
            continue;
        }
        let (from, to) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
        for y in from.y.floor() as usize..=(to.y.ceil() as usize).min(height - 1) {
            for x in from.x.floor() as usize..=(to.x.ceil() as usize).min(width - 1) {
                let centre = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let one = signed(p[1], p[2], centre) / area;
                let two = signed(p[2], p[0], centre) / area;
                if one > 1e-4 && two > 1e-4 && 1.0 - one - two > 1e-4 {
                    cover[y * width + x] = cover[y * width + x].saturating_add(1);
                }
            }
        }
    }
    (cover.iter().filter(|&&count| count >= 2).count(), flipped)
}

#[test]
fn a_person_s_face_is_one_island_laid_once() {
    // **A person's face is a chart of its own, flattened the way it runs**
    // (#473). Wrapped round the head joint, the head's one chart laid the skin
    // that faces along the line from the axis - the flanks of the nose, the
    // walls of the eye sockets - edge-on, and folded some of it over itself:
    // 14 to 48 of the face's texels on these heads were two places of the skin
    // at once, so a bilinear tap there read the other. The face's own chart
    // lays none twice on most heads and a texel on a few - roll 4 here, where a
    // needle at the corner of the mouth turns over - and still holds the whole
    // face, the brows, the eyes, the nose, the mouth and the chin, in one
    // island. Reaching up over the forehead and asking more of the mouth's
    // band (#473's fixer), at most 2 texels laid twice and 0.79 flipped, on
    // roll 4, and 0.83 under a nudge of libm. But off these seven bodies that
    // band, asked of the flattening, folded the face at the mouth's corners:
    // on the thirteen added below, up to 13 texels laid twice and 9.69 turned
    // over (slider corner rc10), and 9 and 2.79 on roll 0, where the border
    // by the neck crossed itself too. With the band magnified once the face is
    // flat (`uv::MOUTH_DETAIL`) and what is still turned over relaxed out of
    // it (`uv::unfold`'s `untangle`, #473's second fixer), none of the twenty
    // lays a texel twice or turns one over; under a nudge of libm, slider
    // corner rc8 - whose neck's zone takes in half its lower lip, so that the
    // face's border runs through its mouth - lays 1 or 2 twice. With each
    // triangle turned over laid back by its own shape before anything is
    // relaxed (#473's third fixer), rc8, whose border crossing itself is still
    // relaxed out, lays 3 twice, and 2 or 3 under a nudge of libm; roll 0 turns
    // a fifth of a texel over under one; the other eighteen none.
    let corner = |edit: fn(&mut AvatarRecord)| {
        let mut record = AvatarRecord::default();
        edit(&mut record);
        record.sanitize();
        record
    };
    let mut records: Vec<AvatarRecord> = [175, 237, 9, 4]
        .into_iter()
        .map(|seed| {
            let mut record = AvatarRecord::default();
            record.reroll(seed);
            record
        })
        .collect();
    records.push(corner(|record| {
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.head_size = -1.0;
        }
    }));
    records.push(corner(|record| record.face.nose = 2.0));
    records.push(corner(|record| record.face.mouth_width = 2.0));
    // And the bodies #473's critics found folding the face chart at the
    // mouth's corners, at the border by the neck, and where the neck's zone
    // reaches the lips (#473's second fixer): rolls 0, 2, 30 and 33, the
    // widest head, five of `slider_draws`'s sixteen slider corners and three of
    // its twenty-four uniform draws.
    for seed in [0, 2, 30, 33] {
        let mut record = AvatarRecord::default();
        record.reroll(seed);
        records.push(record);
    }
    records.push(corner(|record| {
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.head_breadth = 3.0;
        }
    }));
    let (corners, draws) = slider_draws();
    for at in [2, 5, 8, 10, 14] {
        records.push(corners[at].clone());
    }
    for at in [11, 13, 18] {
        records.push(draws[at].clone());
    }

    let size = 1024.0;
    let mut read = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let avatar = symbios_avatar::Avatar::build(record).expect("a person builds");
        let body = &avatar.parts.body;
        let uv = &avatar.parts.unwrap;
        let skull = symbios_avatar::face::Skull::measure(body, &avatar.rig).expect("a head");
        let canon = symbios_avatar::Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles =
            symbios_avatar::Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        // The mouth's pocket is charted apart, out of the face's rectangle.
        let charted = |face: usize| {
            let rect = uv.charts[uv.chart_of_face[face] as usize].rect;
            uv.faces[face].iter().all(|&corner| {
                let at = uv.uvs[corner as usize];
                at.cmpge(rect.min - 1e-5).all() && at.cmple(rect.max + 1e-5).all()
            })
        };
        let head = |face: usize| uv.charts[uv.chart_of_face[face] as usize].zone == Zone::Head;
        let local = |face: usize| {
            let corners = &uv.faces[face];
            corners
                .iter()
                .map(|&corner| body.positions[uv.source[corner as usize] as usize])
                .sum::<symbios_avatar::Vec3>()
                / corners.len() as f32
                - follicles.origin()
        };
        // The face's chart: the one the tip of the nose is in.
        let tip = (0..uv.faces.len())
            .filter(|&face| head(face) && charted(face))
            .max_by(|&one, &two| local(one).z.total_cmp(&local(two).z))
            .expect("a head");
        let chart = uv.chart_of_face[tip];
        // Every face between the cheeks from the chin to the brows is in it.
        for face in (0..uv.faces.len()).filter(|&face| head(face) && charted(face)) {
            let at = local(face);
            let reach = (at.x * at.x + at.z * at.z).sqrt();
            if at.x.abs() < canon.unit
                && at.z > 0.7 * reach
                && at.y > canon.chin() + 0.1 * canon.frame
                && at.y < canon.level + 0.3 * canon.frame
            {
                assert_eq!(
                    uv.chart_of_face[face], chart,
                    "body {index}: the face at {at:?} is charted apart from the nose"
                );
            }
        }
        let triangles: Vec<[symbios_avatar::Vec2; 3]> = (0..uv.faces.len())
            .filter(|&face| uv.chart_of_face[face] == chart && charted(face))
            .flat_map(|face| {
                let corners = &uv.faces[face];
                (1..corners.len() - 1).map(move |fan| {
                    [corners[0], corners[fan], corners[fan + 1]].map(|at| uv.uvs[at as usize])
                })
            })
            .collect();
        let (twice, flipped) = laid_twice(&triangles, size);
        println!("body {index}: {twice} texels laid twice, {flipped:.2} flipped");
        read.push((index, twice, flipped));
    }
    for (index, twice, flipped) in read {
        assert!(
            twice <= 4,
            "body {index}: {twice} of the face's texels are laid twice"
        );
        assert!(
            flipped < 1.0,
            "body {index}: {flipped:.2} texels of the face are laid the other way round"
        );
    }
}

/// Sixteen slider corners, every slider at an end or the middle, and
/// twenty-four uniform draws over the sliders' whole range, each from a fixed
/// stream: the bodies #473's builders and critics measured the head's charts
/// over, beside the rolls.
fn slider_draws() -> (Vec<AvatarRecord>, Vec<AvatarRecord>) {
    struct Stream(u64);
    impl Stream {
        fn next(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 40) as f32) / (1u64 << 24) as f32
        }
        fn range(&mut self, low: f32, high: f32) -> f32 {
            low + (high - low) * self.next()
        }
        fn end(&mut self, low: f32, middle: f32, high: f32) -> f32 {
            let x = self.next();
            if x < 1.0 / 3.0 {
                low
            } else if x < 2.0 / 3.0 {
                middle
            } else {
                high
            }
        }
    }
    let mut stream = Stream(0x5eed_c0de);
    let mut corners = Vec::new();
    for _ in 0..16 {
        let mut record = AvatarRecord::default();
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.height = stream.range(0.0, 4.0);
            params.shoulder_width = stream.end(-3.0, 0.0, 3.0);
            params.hip_width = stream.end(-3.0, 0.0, 3.0);
            params.limb_length = stream.end(-3.0, 0.0, 3.0);
            params.neck_length = stream.end(-3.0, 0.0, 3.0);
            params.head_size = stream.end(-3.0, 0.0, 3.0);
            params.head_breadth = stream.end(-3.0, 0.0, 3.0);
            params.face_length = stream.end(-3.0, 0.0, 3.0);
            params.extremity_size = stream.end(-3.0, 0.0, 3.0);
            params.chest_volume = stream.end(-3.0, 0.0, 3.0);
        }
        record.face.nose = stream.end(-1.0, 0.5, 2.0);
        record.face.nose_width = stream.end(-1.0, 0.5, 2.0);
        record.face.brow = stream.end(-1.0, 0.5, 2.0);
        record.face.mouth = stream.end(-1.0, 0.5, 2.0);
        record.face.mouth_width = stream.end(-1.0, 0.5, 2.0);
        record.composites.mass = stream.end(-3.0, 0.0, 3.0);
        record.composites.femininity = stream.end(-3.0, 0.0, 3.0);
        record.sanitize();
        corners.push(record);
    }
    let mut stream = Stream(0xa11_d4a5);
    let mut draws = Vec::new();
    for _ in 0..24 {
        let mut record = AvatarRecord::default();
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.height = stream.range(0.0, 4.0);
            params.shoulder_width = stream.range(-3.0, 3.0);
            params.hip_width = stream.range(-3.0, 3.0);
            params.limb_length = stream.range(-3.0, 3.0);
            params.neck_length = stream.range(-3.0, 3.0);
            params.head_size = stream.range(-3.0, 3.0);
            params.head_breadth = stream.range(-3.0, 3.0);
            params.face_length = stream.range(-3.0, 3.0);
            params.extremity_size = stream.range(-3.0, 3.0);
            params.chest_volume = stream.range(-3.0, 3.0);
            params.chest_projection = stream.range(-3.0, 3.0);
        }
        record.face.nose = stream.range(-1.0, 2.0);
        record.face.nose_width = stream.range(-1.0, 2.0);
        record.face.brow = stream.range(-1.0, 2.0);
        record.face.mouth = stream.range(-1.0, 2.0);
        record.face.mouth_width = stream.range(-1.0, 2.0);
        record.face.ears = stream.range(-1.0, 2.0);
        record.composites.mass = stream.range(-3.0, 3.0);
        record.composites.femininity = stream.range(-3.0, 3.0);
        record.sanitize();
        draws.push(record);
    }
    (corners, draws)
}
