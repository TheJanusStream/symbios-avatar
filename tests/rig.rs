//! Rigging and skinning must hold across the whole parameter space.
//!
//! The mesher's sweep proves every body can be *built*; this one proves every
//! body can be *posed and dressed*. The two failure modes that matter are silent
//! ones: a vertex bound to nothing deforms wrongly under animation, and a zone
//! that never appears means a garment covering it would leave skin showing.

use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, Landmark, Limb, QuadrupedParams, Rig, SkinConfig, Zone,
    build_cage, catmull_clark, rig::skin,
};

/// Builds the full chain for one record and checks the invariants that must
/// hold for any body.
#[track_caller]
fn assert_riggable(record: &AvatarRecord, what: &str) {
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default())
        .unwrap_or_else(|error| panic!("{what}: {error}"));
    let mesh = catmull_clark(&cage, 1);

    let rig = Rig::from_skeleton(&skeleton).unwrap_or_else(|error| panic!("{what}: {error}"));
    assert_eq!(rig.len(), skeleton.nodes.len(), "{what}: joints lost");

    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
    assert!(
        weights.is_normalized(1e-3),
        "{what}: weights do not sum to one"
    );
    assert_eq!(weights.vertices.len(), mesh.vertex_count());

    for (vertex, influences) in weights.vertices.iter().enumerate() {
        assert!(
            influences[0].weight > 0.0,
            "{what}: vertex {vertex} is bound to nothing"
        );
        for influence in influences {
            assert!(
                (influence.joint as usize) < rig.len(),
                "{what}: vertex {vertex} references a missing joint"
            );
            assert!(
                influence.weight.is_finite() && influence.weight >= 0.0,
                "{what}: vertex {vertex} has a bad weight"
            );
        }
    }
}

#[test]
fn every_rerolled_body_rigs_and_binds() {
    for seed in 0..60i64 {
        let mut biped = AvatarRecord::new("Biped", Archetype::default());
        biped.reroll(seed);
        assert_riggable(&biped, &format!("biped seed {seed}"));

        let mut beast =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        assert_riggable(&beast, &format!("quadruped seed {seed}"));
    }
}

#[test]
fn every_zone_a_body_declares_reaches_the_surface() {
    // A zone with no vertices would be a garment slot that covers nothing, and
    // the skin beneath it would show through.
    for (label, archetype) in [
        ("biped", Archetype::default()),
        ("beast", Archetype::Quadruped(QuadrupedParams::default())),
    ] {
        let record = AvatarRecord::new(label, archetype);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);

        for joint in &rig.joints {
            assert!(
                zones.contains(&joint.zone),
                "{label}: no surface belongs to {:?}",
                joint.zone
            );
        }
    }
}

#[test]
fn limb_zones_land_on_the_correct_side_of_the_body() {
    let record = AvatarRecord::default();
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, 1);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);

    for (vertex, zone) in zones.iter().enumerate() {
        let Some(limb) = zone.limb() else { continue };
        // Extremities and lower limbs are unambiguous; upper limbs meet at the
        // torso, so only check the parts that are clear of the midline.
        if matches!(zone, Zone::Extremity(_) | Zone::LowerLimb(_)) {
            // Left is `+X` on a body facing `+Z`, which is glTF's convention —
            // see `plan::Limb`, and #142 for the pass that moved our names onto
            // the sides they name.
            let x = mesh.positions[vertex].x;
            if limb.is_left() {
                assert!(x > 0.0, "vertex {vertex} in {zone:?} sits at x={x}");
            } else {
                assert!(x < 0.0, "vertex {vertex} in {zone:?} sits at x={x}");
            }
        }
    }
}

#[test]
fn landmarks_track_the_body_they_came_from() {
    let mut wide = AvatarRecord::default();
    let Archetype::Humanoid(ref mut params) = wide.archetype else {
        panic!("default is a biped");
    };
    params.shoulder_width = 1.0;

    let mut narrow = AvatarRecord::default();
    let Archetype::Humanoid(ref mut params) = narrow.archetype else {
        panic!("default is a biped");
    };
    params.shoulder_width = -1.0;

    let span = |record: &AvatarRecord| {
        Rig::from_skeleton(&record.skeleton())
            .expect("rigs")
            .landmarks()
            .span(
                Landmark::LimbRoot(Limb::ForeLeft),
                Landmark::LimbRoot(Limb::ForeRight),
            )
            .expect("both shoulders")
    };

    assert!(
        span(&wide) > span(&narrow),
        "broad shoulders should measure wider"
    );
}

#[test]
fn every_rerolled_body_offers_the_anchors_a_wardrobe_needs() {
    for seed in 0..40i64 {
        let mut record = AvatarRecord::new("Dressed", Archetype::default());
        record.reroll(seed);
        let marks = Rig::from_skeleton(&record.skeleton())
            .expect("rigs")
            .landmarks();

        for landmark in [
            Landmark::Crown,
            Landmark::NeckBase,
            Landmark::ChestRing,
            Landmark::WaistRing,
            Landmark::HipRing,
        ] {
            assert!(marks.has(landmark), "seed {seed} is missing {landmark:?}");
        }
        for limb in Limb::ALL {
            assert!(marks.has(Landmark::LimbTip(limb)), "seed {seed}: {limb:?}");
        }

        // Anchors have to stay in body order however the sliders moved.
        let y = |l: Landmark| marks.get(l).expect("present").position.y;
        assert!(
            y(Landmark::Crown) > y(Landmark::ChestRing),
            "seed {seed}: crown fell below the chest"
        );
        assert!(
            y(Landmark::ChestRing) > y(Landmark::HipRing),
            "seed {seed}: chest fell below the hips"
        );
    }
}

#[test]
fn a_garment_and_the_skin_beneath_it_never_share_a_zone() {
    use symbios_avatar::ZoneSet;

    let shirt: ZoneSet = [
        Zone::Chest,
        Zone::Abdomen,
        Zone::UpperLimb(Limb::ForeLeft),
        Zone::UpperLimb(Limb::ForeRight),
    ]
    .into_iter()
    .collect();

    let record = AvatarRecord::default();
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, 1);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);

    let covered = zones.iter().filter(|zone| shirt.contains(**zone)).count();
    let bare = zones.len() - covered;
    assert!(covered > 0, "the shirt must cover something");
    assert!(bare > 0, "and must not cover everything");
    assert_eq!(
        covered + bare,
        mesh.vertex_count(),
        "every vertex is decided"
    );
}
