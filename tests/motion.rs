//! Every body must be able to move, not just the one that was tuned.
//!
//! The sweeps elsewhere prove a body can be built, dressed, and painted. This
//! one proves it can walk — across the parameter space, on both archetypes, over
//! ground that is not flat, and without the geometry tearing when the pose is
//! actually applied to it.

use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, FootingConfig, Gait, Ground, Inertializer, Pose,
    QuadrupedParams, Rig, SkinConfig, Stride, Vec3, anim::gait, anim::plant_feet_of, build_cage,
    catmull_clark, rig::skin,
};

/// Walks one body through a full cycle, asserting every step is within reach.
#[track_caller]
fn assert_walks(record: &AvatarRecord, what: &str) {
    let skeleton = record.skeleton();
    let rig = Rig::from_skeleton(&skeleton).unwrap_or_else(|e| panic!("{what}: {e}"));
    let gait = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);

    assert!(!gait.is_empty(), "{what}: nothing to walk on");

    for frame in 0..32 {
        let cycle = frame as f32 / 32.0;
        let mut pose = Pose::rest(&rig);
        let steps = gait::step(&rig, &mut pose, &gait, &stride, cycle);

        assert!(
            steps.is_clean(),
            "{what}: frame {frame} strained {:?}",
            steps.straining
        );
        assert!(
            !steps.stance.is_empty(),
            "{what}: frame {frame} had nothing on the ground"
        );
    }
}

#[test]
fn every_rerolled_body_can_walk() {
    for seed in 0..40i64 {
        let mut biped = AvatarRecord::new("Walker", Archetype::default());
        biped.reroll(seed);
        assert_walks(&biped, &format!("biped seed {seed}"));

        let mut beast =
            AvatarRecord::new("Runner", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        assert_walks(&beast, &format!("quadruped seed {seed}"));
    }
}

#[test]
fn a_gait_keeps_a_body_up_at_every_moment() {
    // Losing every contact at once is the difference between a walk and a
    // stumble, and it has to hold continuously rather than on average.
    for archetype in [
        Archetype::default(),
        Archetype::Quadruped(QuadrupedParams::default()),
    ] {
        let record = AvatarRecord::new("Steady", archetype);
        let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
        let gait = Gait::natural(&rig);

        for frame in 0..240 {
            let cycle = frame as f32 / 240.0;
            assert!(
                gait.grounded(cycle) >= 1,
                "nothing grounded at {cycle} with {} contacts",
                gait.len()
            );
        }
    }
}

#[test]
fn a_walking_body_settles_its_stance_feet_onto_a_slope() {
    // The composition that makes locomotion work on terrain: the gait decides
    // where feet go, then the stance feet — and only those — are settled onto
    // whatever is really there. Planting the swinging foot too would drag it
    // down and reduce the walk to a shuffle.
    let record = AvatarRecord::default();
    let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
    let gait = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);
    let grade = 0.15;

    let mut lifted_above_ground = 0;
    for frame in 0..24 {
        let mut pose = Pose::rest(&rig);
        let steps = gait::step(&rig, &mut pose, &gait, &stride, frame as f32 / 24.0);

        let footing = plant_feet_of(
            &rig,
            &mut pose,
            &steps.stance,
            |foot| {
                Some(Ground {
                    position: Vec3::new(foot.x, foot.z * grade, foot.z),
                    normal: Vec3::new(0.0, 1.0, -grade).normalize(),
                })
            },
            &FootingConfig::default(),
        );
        assert!(
            footing.straining.is_empty(),
            "frame {frame}: {:?} could not reach the slope",
            footing.straining
        );

        // A swinging foot should still be off the ground afterwards.
        let posed = pose.forward(&rig);
        for &limb in &steps.swing {
            let joint = rig.in_zone(symbios_avatar::Zone::Extremity(limb))[0];
            let foot = posed.positions[joint];
            if foot.y > foot.z * grade + 1e-3 {
                lifted_above_ground += 1;
            }
        }
    }
    assert!(
        lifted_above_ground > 0,
        "no swinging foot ever cleared the ground"
    );
}

#[test]
fn a_change_of_gait_is_absorbed_rather_than_snapped() {
    let record = AvatarRecord::default();
    let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
    let walking = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);
    let dt = 1.0 / 60.0;

    let posed_at = |cycle: f32| {
        let mut pose = Pose::rest(&rig);
        gait::step(&rig, &mut pose, &walking, &stride, cycle);
        pose
    };

    // Mid-stride, switch to standing still.
    let previous = posed_at(0.28);
    let current = posed_at(0.30);
    let standing = {
        let mut pose = Pose::rest(&rig);
        gait::step(
            &rig,
            &mut pose,
            &Gait::standing(&rig),
            &Stride::still(),
            0.0,
        );
        pose
    };

    let mut transition = Inertializer::start(&previous, &current, &standing, dt, 0.25);
    let first = transition.apply(&standing);

    // It starts where the walk left off, not where standing begins.
    let drift = |a: &Pose, b: &Pose| {
        a.rotations
            .iter()
            .zip(&b.rotations)
            .map(|(x, y)| x.angle_between(*y))
            .fold(0.0f32, f32::max)
    };
    assert!(
        drift(&first, &current) < drift(&standing, &current) * 0.2,
        "the transition should begin at the outgoing pose"
    );

    // And it arrives, exactly.
    for _ in 0..20 {
        transition.advance(dt);
    }
    assert!(transition.finished());
    assert_eq!(transition.apply(&standing), standing);
}

#[test]
fn walking_does_not_tear_the_body_it_moves() {
    // The whole chain applied for real: mesh, weights, pose, deform. A limb
    // whose weights were wrong shows up here as geometry stretching far beyond
    // anything the skeleton did.
    let record = AvatarRecord::default();
    let skeleton = record.skeleton();
    let mesh = catmull_clark(
        &build_cage(&skeleton, &CageConfig::default()).expect("meshes"),
        1,
    );
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
    let gait = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);

    // How far the skeleton itself travels over the cycle bounds what the skin
    // is allowed to.
    let mut widest_joint = 0.0f32;
    let mut widest_vertex = 0.0f32;
    let rest = Pose::rest(&rig).forward(&rig);
    let rest_skin = rest.deform(&rig, &mesh.positions, &weights);

    for frame in 0..16 {
        let mut pose = Pose::rest(&rig);
        gait::step(&rig, &mut pose, &gait, &stride, frame as f32 / 16.0);
        let posed = pose.forward(&rig);

        for (index, position) in posed.positions.iter().enumerate() {
            widest_joint = widest_joint.max(position.distance(rest.positions[index]));
        }
        for (index, position) in posed
            .deform(&rig, &mesh.positions, &weights)
            .iter()
            .enumerate()
        {
            widest_vertex = widest_vertex.max(position.distance(rest_skin[index]));
        }
    }

    assert!(widest_joint > 0.01, "the body should actually have moved");
    assert!(
        widest_vertex < widest_joint * 2.0,
        "skin travelled {widest_vertex:.3} where bone travelled {widest_joint:.3}"
    );
}

#[test]
fn a_faster_pace_takes_longer_steps_and_sinks_further() {
    let record = AvatarRecord::default();
    let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
    let gait = Gait::natural(&rig);

    let amble = Stride::for_body(&rig, 0.4);
    let march = Stride::for_body(&rig, 1.0);
    assert!(march.length > amble.length);
    assert!(march.lift > amble.lift);
    assert!(
        gait::crouch_for(&rig, &gait, &march) > gait::crouch_for(&rig, &gait, &amble),
        "a longer stride needs more sinking to stay in reach"
    );
    assert_eq!(
        gait::crouch_for(&rig, &gait, &Stride::still()),
        0.0,
        "a body standing still has nothing to sink for"
    );
}
