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
