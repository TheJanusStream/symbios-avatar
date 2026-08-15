//! Every body must be able to move, not just the one that was tuned.
//!
//! The sweeps elsewhere prove a body can be built, dressed, and painted. This
//! one proves it can walk — across the parameter space, on both archetypes, over
//! ground that is not flat, and without the geometry tearing when the pose is
//! actually applied to it.

use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, FootingConfig, Gait, Ground,
    Inertializer, Pose, QuadrupedParams, Rig, SkinConfig, Stride, Vec3, anim::gait,
    anim::plant_feet_of, build_cage, catmull_clark, rig::skin,
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
        let steps = gait::step(&rig, &mut pose, &gait, &stride, cycle, |_| None);

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
        let steps = gait::step(&rig, &mut pose, &gait, &stride, frame as f32 / 24.0, |_| {
            None
        });

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
        gait::step(&rig, &mut pose, &walking, &stride, cycle, |_| None);
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
            |_| None,
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
        gait::step(&rig, &mut pose, &gait, &stride, frame as f32 / 16.0, |_| {
            None
        });
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

/// Twice the area of a triangle.
fn area(corners: [Vec3; 3]) -> f32 {
    (corners[1] - corners[0])
        .cross(corners[2] - corners[0])
        .length()
        * 0.5
}

#[test]
fn dual_quaternion_skinning_compresses_less_of_the_body_than_matrices_do() {
    // Averaging matrices does not average rotations, so where two bones turn
    // apart the surface between them collapses toward the axis. This is the
    // measurement that chose the default, and it is deliberately a measurement
    // over the WHOLE surface: judged on the single worst triangle instead, the
    // two methods trade places and the answer comes out backwards.
    let record = AvatarRecord::new("Skinned", Archetype::default());
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, BODY_SUBDIVISIONS);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
    let triangles = mesh.triangulated();

    let rest: Vec<f32> = triangles
        .iter()
        .map(|tri| area(tri.map(|corner| mesh.positions[corner as usize])))
        .collect();

    let gait = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);

    // **How much area the surface LOSES, not how many triangles cross a
    // threshold** (#107). This counted triangles under two thirds of their rest
    // area and compared the worst hundredth per phase, and both of those are
    // measures of the mesh as much as of the skinning. Eight-point cage rings at
    // one subdivision took the body from about twelve thousand triangles to
    // 3,140, and with it:
    //
    // - the counts collapsed to single digits — 50 crushed over a whole cycle
    //   against 54 for matrices — so a rule asking for a factor of two was
    //   comparing noise;
    // - the "worst hundredth" became the thirty-second triangle, which is deep
    //   in the handful at the crotch this test already knew about, and exactly
    //   the single-worst regime the note above says the answer inverts in. It
    //   duly inverted, by 0.0018, on two of eight phases.
    //
    // Neither was a skinning regression. Measured as area lost, dual
    // quaternions beat matrices on every phase of the cycle and by a margin that
    // does not depend on how finely the body is meshed:
    //
    // ```text
    //   cycle   0.000  0.125  0.250  0.375   (0.5 to 0.875 mirror these)
    //   ratio   0.922  0.908  0.849  0.848    whole cycle 0.869
    // ```
    //
    // Only losses count. A limb that stretches must not be allowed to cancel a
    // crotch that collapsed, which is the whole failure being guarded against.
    let lost = |deformed: &[Vec3]| -> f32 {
        triangles
            .iter()
            .enumerate()
            .map(|(index, tri)| {
                let now = area(tri.map(|corner| deformed[corner as usize]));
                (rest[index] - now).max(0.0)
            })
            .sum::<f32>()
    };

    let (mut linear_total, mut dual_total) = (0.0f32, 0.0f32);
    for step in 0..8 {
        let cycle = step as f32 / 8.0;
        let mut pose = Pose::rest(&rig);
        gait::step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
        let posed = pose.forward(&rig);

        let linear_lost = lost(&posed.deform_linear(&rig, &mesh.positions, &weights));
        let dual_lost = lost(&posed.deform(&rig, &mesh.positions, &weights));

        // Per phase as well as over the cycle, because a method that wins on
        // the average by winning hugely at mid-stride and losing at the
        // extremes is not the one to make the default.
        assert!(
            dual_lost < linear_lost * 0.95,
            "cycle {cycle}: dual quaternions lost {dual_lost} of area against {linear_lost} \
             for matrices, which is {:.3} of it",
            dual_lost / linear_lost
        );
        linear_total += linear_lost;
        dual_total += dual_lost;
    }

    assert!(
        linear_total > 0.0,
        "no phase of the walk compressed anything under matrix skinning, so nothing was compared"
    );
    // **0.93 rather than 0.90, and the number moved because the WALK did, not
    // because skinning did.** Until #254 the leg solve missed its goal by the
    // extremity's hang, so the legs sat at angles a little short of the ones
    // the gait asked for; solving accurately puts the joints where they were
    // always meant to be, and the crotch is a little more folded at the
    // extremes of the stride as a result. Dual quaternions still beat matrices
    // on every phase and over the cycle — 0.900 against the 0.869 recorded
    // above — which is the claim. The margin is a property of the pose being
    // compared and not of the two methods.
    assert!(
        dual_total < linear_total * 0.93,
        "over a whole cycle dual quaternions lost {dual_total} of area and matrices \
         {linear_total}, which is {:.3} of it",
        dual_total / linear_total
    );
}

#[test]
fn skinning_leaves_the_rest_pose_exactly_alone() {
    // Whatever the method, a body that is not posed must not move. Dual
    // quaternion blending normalises, and a normalisation that is even slightly
    // wrong shows up here first.
    let record = AvatarRecord::new("Still", Archetype::default());
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
    let mesh = catmull_clark(&cage, BODY_SUBDIVISIONS);
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());

    let posed = Pose::rest(&rig).forward(&rig);
    for (original, moved) in
        mesh.positions
            .iter()
            .zip(posed.deform(&rig, &mesh.positions, &weights))
    {
        assert!(
            original.distance(moved) < 1e-4,
            "the rest pose moved {original:?} to {moved:?}"
        );
    }
}

#[test]
fn swinging_arms_leaves_the_limbs_the_body_stands_on_alone() {
    // Measured defect: on a quadruped every fore contact moved 0.21-0.24 m the
    // moment the arms swung, because the fore limbs are legs and had just been
    // placed by IK. swing_arms was the one pose producer that assigned rather
    // than composed, so it did not merely disagree with the solve — it erased it.
    use symbios_avatar::{Limb, Zone, anim::gait};

    for (name, archetype) in [
        ("biped", Archetype::default()),
        (
            "quadruped",
            Archetype::Quadruped(QuadrupedParams::default()),
        ),
    ] {
        let rig = Rig::from_skeleton(&archetype.skeleton(&symbios_avatar::Composites::default()))
            .expect("rigs");
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let carries = rig.ground_contacts();

        for frame in 0..8 {
            let cycle = frame as f32 / 8.0;
            let mut pose = Pose::rest(&rig);
            let steps = gait::step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
            plant_feet_of(
                &rig,
                &mut pose,
                &steps.stance,
                |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
                &FootingConfig::default(),
            );

            let planted = pose.forward(&rig);
            gait::swing_arms(&rig, &mut pose, &gait, &stride, cycle);
            let swung = pose.forward(&rig);

            for &limb in &carries {
                let contact = *rig
                    .in_zone(Zone::Extremity(limb))
                    .first()
                    .expect("a contact limb has an extremity");
                let moved = planted.positions[contact].distance(swung.positions[contact]);
                assert!(
                    moved < 1e-4,
                    "{name} {limb:?} contact moved {moved:.4} m at cycle {cycle:.2}"
                );
            }
        }

        // And on a body that has arms, the swing must still do something.
        if carries.len() < Limb::ALL.len() {
            let mut pose = Pose::rest(&rig);
            let before = pose.clone();
            gait::swing_arms(&rig, &mut pose, &gait, &stride, 0.25);
            assert_ne!(pose, before, "{name} did not swing its arms at all");
        }
    }
}

#[test]
fn a_limb_folds_the_way_its_own_plan_says_it_folds() {
    // The pole was hardcoded forward at the one call site, which is right for a
    // knee and backwards for every other joint that can be solved.
    //
    // The expectations here were *measured*, not assumed. A first attempt keyed
    // the direction on fore-versus-hind and the render disagreed: a quadruped's
    // hock folds backward like its carpus, so the rule got half a creature
    // wrong. The rest pose already knew — a quadruped's limbs are built with the
    // bend in them — and reading it beats any rule about limb names.
    use symbios_avatar::{Limb, anim::ik};

    // Backward for everything a quadruped has; on a biped, whose limbs are all
    // dead straight at rest, the anatomical fallback decides.
    let expected = |name: &str, limb: Limb| -> f32 {
        match (name, limb.is_fore()) {
            ("quadruped", _) => -1.0,
            (_, true) => -1.0,
            (_, false) => 1.0,
        }
    };

    for (name, archetype) in [
        ("biped", Archetype::default()),
        (
            "quadruped",
            Archetype::Quadruped(QuadrupedParams::default()),
        ),
    ] {
        let rig = Rig::from_skeleton(&archetype.skeleton(&symbios_avatar::Composites::default()))
            .expect("rigs");
        for limb in Limb::ALL {
            let Some(chain) = rig.limb_chain(limb) else {
                continue;
            };
            let pole = rig.bend_pole(limb).expect("a solvable limb has a pole");

            // Reach for a point well inside the limb's span, so it must bend.
            let root = rig.joints[chain[0]].position;
            let tip = rig.joints[chain[2]].position;
            let target = root + (tip - root) * 0.55;

            let mut pose = Pose::rest(&rig);
            ik::two_bone(&rig, &mut pose, chain, target, pole);
            let posed = pose.forward(&rig);

            // The apex is how far the middle joint left the line from root to
            // tip, measured fore-and-aft.
            let line =
                (posed.positions[chain[2]] - posed.positions[chain[0]]).normalize_or(Vec3::Y);
            let out = posed.positions[chain[1]] - posed.positions[chain[0]];
            let apex = (out - line * out.dot(line)).z;
            let want = expected(name, limb);
            assert!(
                apex * want > 1e-3,
                "{name} {limb:?} folded the wrong way: apex {apex:+.4}, wanted sign {want:+}"
            );
        }
    }
}

#[test]
fn a_plan_that_states_its_own_bend_is_believed_over_the_fallback() {
    // The property that makes the fallback safe to keep: where a plan has an
    // opinion, the opinion wins. A quadruped's hind limb folds backward, which
    // is the opposite of what the fore-versus-hind fallback would choose, so
    // this fails the moment the measurement stops being consulted.
    use symbios_avatar::Limb;

    let rig = Rig::from_skeleton(
        &Archetype::Quadruped(QuadrupedParams::default())
            .skeleton(&symbios_avatar::Composites::default()),
    )
    .expect("rigs");
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let chain = rig.limb_chain(limb).expect("a hind limb solves");
        let root = rig.joints[chain[0]].position;
        let pole = rig.bend_pole(limb).expect("a solvable limb has a pole");
        assert!(
            (pole - root).z < 0.0,
            "{limb:?} was given a forward pole despite resting bent backward"
        );
    }
}

#[test]
fn turning_one_joint_leaves_the_bone_before_it_alone() {
    // The noodle, as a property (#97). A limb is two rigid bones and a hinge:
    // flexing a knee carries the shank and must not touch the thigh. It did,
    // for as long as the binding credited a bone to the joint at its far end
    // while `Pose::forward` turned it from the near one — the mid-thigh
    // travelled 39.8 mm against the mid-shank's 73.1 mm, and the whole leg
    // curved like rope.
    //
    // Written against travel rather than against weights on purpose. The
    // convention is an implementation detail and could be expressed a dozen
    // ways; what must never come back is a limb that bends everywhere.
    use symbios_avatar::{Limb, Quat};

    let record = AvatarRecord::default();
    let skeleton = record.skeleton();
    let mesh = catmull_clark(
        &build_cage(&skeleton, &CageConfig::default()).expect("meshes"),
        1,
    );
    let rig = Rig::from_skeleton(&skeleton).expect("rigs");
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());

    for limb in [Limb::HindLeft, Limb::ForeLeft] {
        let [root, mid, tip] = rig.limb_chain(limb).expect("a solvable limb");

        let mut pose = Pose::rest(&rig);
        pose.rotations[mid] = Quat::from_rotation_x(0.5);
        let moved = pose.forward(&rig).deform(&rig, &mesh.positions, &weights);

        // The middle half of each bone. The bands beside the joint are meant to
        // move — that is the blend that keeps the surface from creasing — so
        // including them would be asking a different question.
        let travel = |from: usize, to: usize| {
            let (start, end) = (rig.joints[from].position, rig.joints[to].position);
            let axis = end - start;
            let span = axis.length_squared().max(f32::EPSILON);
            let girth = rig.joints[from].radius.max(rig.joints[to].radius) * 2.0;
            let mut total = 0.0;
            let mut count = 0usize;
            for (vertex, &at) in mesh.positions.iter().enumerate() {
                let along = (at - start).dot(axis) / span;
                if !(0.25..0.75).contains(&along) {
                    continue;
                }
                if (at - (start + axis * along)).length() > girth {
                    continue;
                }
                total += at.distance(moved[vertex]);
                count += 1;
            }
            (total / count.max(1) as f32, count)
        };

        let (before, seen) = travel(root, mid);
        let (after, _) = travel(mid, tip);
        assert!(seen > 0, "{limb:?} has no surface on its upper bone");
        assert!(after > 1e-3, "{limb:?} did not bend at all");
        assert!(
            before < after * 0.10,
            "{limb:?} leaked {:.0}% of the bend into the bone before the joint \
             ({before:.4} m against {after:.4} m)",
            before / after * 100.0
        );
    }
}
