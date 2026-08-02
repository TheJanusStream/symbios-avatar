//! Every point of the parameter space must produce a body.
//!
//! This is the contract that makes a creator safe to ship. Procedural character
//! systems classically fail here: sampling parameters at random mostly produces
//! broken output, and the fix is a constraint layer, not better sliders. So the
//! constraints live in the body plans — derived lengths floored against the
//! joint radii they have to clear — and these sweeps are what hold them honest.
//!
//! A failure here is not a mesher bug. It means a slider can reach a shape whose
//! joints cannot be resolved, and the body plan has to be corrected.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use symbios_avatar::{
    Archetype, AvatarRecord, BodyPlan, CageConfig, HumanoidParams, QuadrupedParams, build_cage,
};

/// Builds a body and asserts it is watertight, reporting the parameters if not.
#[track_caller]
fn assert_meshable(archetype: &Archetype, what: &str) {
    let skeleton = archetype.skeleton();
    skeleton
        .validate()
        .unwrap_or_else(|error| panic!("{what}: invalid skeleton: {error}"));

    let cage = build_cage(&skeleton, &CageConfig::default())
        .unwrap_or_else(|error| panic!("{what}: {error}"));
    let report = cage.manifold_report();
    assert!(report.is_clean(), "{what}: not watertight: {report:?}");
}

/// The extreme and neutral value of each signed axis.
const EXTREMES: [f32; 3] = [-1.0, 0.0, 1.0];

/// A named axis of a body plan, so each can be driven to its extremes in turn.
type Axis<P> = (&'static str, fn(&mut P, f32));

#[test]
fn every_humanoid_axis_meshes_at_its_extremes() {
    let heights = [1.2f32, 1.75, 2.2];
    let axes: [Axis<HumanoidParams>; 8] = [
        ("build", |p, v| p.build = v),
        ("muscle", |p, v| p.muscle = v.abs()),
        ("shoulder_width", |p, v| p.shoulder_width = v),
        ("hip_width", |p, v| p.hip_width = v),
        ("limb_length", |p, v| p.limb_length = v),
        ("neck_length", |p, v| p.neck_length = v),
        ("head_size", |p, v| p.head_size = v),
        ("extremity_size", |p, v| p.extremity_size = v),
    ];

    for height in heights {
        for (name, apply) in axes {
            for value in EXTREMES {
                let mut params = HumanoidParams {
                    height,
                    ..Default::default()
                };
                apply(&mut params, value);
                params.sanitize();
                assert_meshable(
                    &Archetype::Humanoid(params),
                    &format!("humanoid h={height} {name}={value}"),
                );
            }
        }
    }
}

#[test]
fn the_humanoid_corners_of_the_space_mesh() {
    // Every axis pinned to the same extreme at once: the heaviest, widest,
    // longest-limbed body, and its opposite.
    for value in EXTREMES {
        for height in [1.2f32, 2.2] {
            let mut params = HumanoidParams {
                height,
                build: value,
                muscle: value.max(0.0),
                shoulder_width: value,
                hip_width: value,
                limb_length: value,
                neck_length: value,
                head_size: value,
                extremity_size: value,
            };
            params.sanitize();
            assert_meshable(
                &Archetype::Humanoid(params),
                &format!("humanoid corner h={height} all={value}"),
            );
        }
    }
}

#[test]
fn random_humanoids_always_mesh() {
    let mut rng = Pcg64Mcg::seed_from_u64(0xA1FA);
    for sample in 0..1500 {
        let mut params = HumanoidParams {
            height: rng.random_range(1.2..=2.2),
            build: rng.random_range(-1.0..=1.0),
            muscle: rng.random_range(0.0..=1.0),
            shoulder_width: rng.random_range(-1.0..=1.0),
            hip_width: rng.random_range(-1.0..=1.0),
            limb_length: rng.random_range(-1.0..=1.0),
            neck_length: rng.random_range(-1.0..=1.0),
            head_size: rng.random_range(-1.0..=1.0),
            extremity_size: rng.random_range(-1.0..=1.0),
        };
        params.sanitize();
        assert_meshable(
            &Archetype::Humanoid(params),
            &format!("random humanoid #{sample}: {params:?}"),
        );
    }
}

#[test]
fn every_quadruped_axis_meshes_at_its_extremes() {
    let heights = [0.25f32, 0.58, 1.8];
    let axes: [Axis<QuadrupedParams>; 7] = [
        ("body_length", |p, v| p.body_length = v),
        ("build", |p, v| p.build = v),
        ("muscle", |p, v| p.muscle = v.abs()),
        ("leg_length", |p, v| p.leg_length = v),
        ("neck_length", |p, v| p.neck_length = v),
        ("head_size", |p, v| p.head_size = v),
        ("tail_length", |p, v| p.tail_length = v),
    ];

    for height in heights {
        for (name, apply) in axes {
            for value in EXTREMES {
                let mut params = QuadrupedParams {
                    height,
                    ..Default::default()
                };
                apply(&mut params, value);
                params.sanitize();
                assert_meshable(
                    &Archetype::Quadruped(params),
                    &format!("quadruped h={height} {name}={value}"),
                );
            }
        }
    }
}

#[test]
fn random_quadrupeds_always_mesh() {
    let mut rng = Pcg64Mcg::seed_from_u64(0x4BEA_5700);
    for sample in 0..1500 {
        let mut params = QuadrupedParams {
            height: rng.random_range(0.25..=1.8),
            body_length: rng.random_range(-1.0..=1.0),
            build: rng.random_range(-1.0..=1.0),
            muscle: rng.random_range(0.0..=1.0),
            leg_length: rng.random_range(-1.0..=1.0),
            neck_length: rng.random_range(-1.0..=1.0),
            head_size: rng.random_range(-1.0..=1.0),
            tail_length: rng.random_range(-1.0..=1.0),
        };
        params.sanitize();
        assert_meshable(
            &Archetype::Quadruped(params),
            &format!("random quadruped #{sample}: {params:?}"),
        );
    }
}

#[test]
fn every_rerolled_record_meshes() {
    // The path a creator's randomise button actually takes.
    for seed in 0..400i64 {
        let mut record = AvatarRecord::new("Roll", Archetype::default());
        record.reroll(seed);
        assert_meshable(&record.archetype, &format!("rerolled humanoid seed {seed}"));

        let mut beast =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        assert_meshable(&beast.archetype, &format!("rerolled quadruped seed {seed}"));
    }
}

#[test]
fn a_look_survives_a_share_code_and_still_meshes() {
    for seed in 0..200i64 {
        let mut record = AvatarRecord::new("Shared", Archetype::default());
        record.reroll(seed);

        let code = record.share_code();
        let copy = AvatarRecord::from_share_code("Copy", &code)
            .unwrap_or_else(|error| panic!("seed {seed}: {code} did not decode: {error}"));

        // Quantisation moves the axes slightly; the result must still be a body.
        assert_meshable(&copy.archetype, &format!("share-code copy of seed {seed}"));
    }
}

/// Extent of one zone's surface along each axis, in metres.
///
/// Measured off the built body rather than off the plan, because the plan's
/// numbers are what a node was asked for and this is about what the mesh did.
fn zone_extent(record: &AvatarRecord, zone: symbios_avatar::Zone) -> [f32; 3] {
    let avatar = symbios_avatar::Avatar::build(record).expect("the body builds");
    let parts = &avatar.parts;
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for (vertex, at) in parts.zones.iter().enumerate() {
        if *at != zone {
            continue;
        }
        let point = parts.body.positions[vertex];
        for axis in 0..3 {
            lo[axis] = lo[axis].min(point[axis]);
            hi[axis] = hi[axis].max(point[axis]);
        }
    }
    assert!(lo[0] <= hi[0], "{zone:?} has no surface");
    [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]
}

#[test]
fn a_trunk_is_not_a_surface_of_revolution() {
    // Every node used to be a circle, so a chest measured perfectly round and
    // the body read as a stack of drums. `Node::scale` was plumbed through the
    // mesher the whole time and simply never set.
    //
    // Measured on the ABDOMEN of each plan, which is the one trunk zone that is
    // a plain connector: a chest or a pelvis carries limb sockets, and their
    // spread dominates the lateral extent, so a ratio taken there would report
    // the arms rather than the ribcage.
    let biped = AvatarRecord::new("Trunk", Archetype::default());
    let [wide, _, deep] = zone_extent(&biped, symbios_avatar::Zone::Abdomen);
    assert!(
        wide / deep > 1.2,
        "an upright trunk measured {wide:.4} across and {deep:.4} through — ratio {:.2}",
        wide / deep
    );

    // And the same anatomy comes out with the axes swapped, because a
    // quadruped's spine runs fore-and-aft: its ribcage is narrow and deep, which
    // is the opposite proportion in the same two numbers. If this ever reports a
    // ratio near one, the scale is being dropped somewhere between plan and cage.
    let beast = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
    let [across, tall, _] = zone_extent(&beast, symbios_avatar::Zone::Abdomen);
    assert!(
        tall / across > 1.15,
        "a four-legged trunk measured {across:.4} across and {tall:.4} deep — ratio {:.2}",
        tall / across
    );
}
