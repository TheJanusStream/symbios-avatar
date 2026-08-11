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
use symbios_avatar::face::Skull;
use symbios_avatar::plan::{AGE_RANGE, DEFAULT_AGE, DEFAULT_BODY_FAT};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, BodyPlan, CageConfig, Composites, HumanoidParams,
    Limb, QuadrupedParams, Rig, Skeleton, Vec3, Zone, build_body, build_cage,
};

/// Builds a body and asserts it is watertight, reporting the parameters if not.
#[track_caller]
fn assert_meshable(archetype: &Archetype, composites: &Composites, what: &str) {
    let skeleton = archetype.skeleton(composites);
    skeleton
        .validate()
        .unwrap_or_else(|error| panic!("{what}: invalid skeleton: {error}"));

    let cage = build_cage(&skeleton, &CageConfig::default())
        .unwrap_or_else(|error| panic!("{what}: {error}"));
    let report = cage.manifold_report();
    assert!(report.is_clean(), "{what}: not watertight: {report:?}");
}

/// Composites that say nothing about the body, which is what a sweep of the
/// per-region axes wants: it is testing those axes, not this one.
const NEUTRAL: Composites = Composites {
    femininity: 0.0,
    mass: 0.0,
    body_fat: DEFAULT_BODY_FAT,
    age: DEFAULT_AGE,
};

/// The extreme and neutral value of each signed axis.
const EXTREMES: [f32; 3] = [-1.0, 0.0, 1.0];

/// A named axis of a body plan, so each can be driven to its extremes in turn.
type Axis<P> = (&'static str, fn(&mut P, f32));

/// A named axis that can also be read back, to check what sanitising did to it.
type Readable<P> = (&'static str, fn(&mut P, f32), fn(&P) -> f32);

/// Every non-finite value the public API can be handed.
const NON_FINITE: [(&str, f32); 3] = [
    ("NaN", f32::NAN),
    ("+inf", f32::INFINITY),
    ("-inf", f32::NEG_INFINITY),
];

/// Poisons each axis in turn and checks both halves of the contract.
///
/// **Both assertions are load-bearing and the weaker one is the meshability
/// check.** Poisoning any of the `-1..=1` axes and asserting only that the body
/// meshes passes even when the guard is missing entirely, because a lost axis
/// lands on `0.0` and `0.0` is that axis's neutral — fifteen of seventeen axes
/// across the two plans were correct by that coincidence rather than by any
/// guard (#55). Only `height`, whose range excludes zero, failed visibly. So
/// the value is checked against `Default` as well, and it is read out of
/// `Default` rather than written down here so the two cannot drift apart.
fn assert_non_finite_axes_fall_back<P>(plan: &str, axes: &[Readable<P>], wrap: fn(P) -> Archetype)
where
    P: BodyPlan + Default + Copy,
{
    let default = P::default();
    for &(name, poison, read) in axes {
        for (label, value) in NON_FINITE {
            let mut params = default;
            poison(&mut params, value);
            params.sanitize();
            assert_eq!(
                read(&params),
                read(&default),
                "{plan} {name}={label} should sanitize to its documented default"
            );
            assert_meshable(&wrap(params), &NEUTRAL, &format!("{plan} {name}={label}"));
        }
    }
}

#[test]
fn a_non_finite_humanoid_axis_takes_its_documented_default() {
    let axes: [Readable<HumanoidParams>; 7] = [
        ("height", |p, v| p.height = v, |p| p.height),
        (
            "shoulder_width",
            |p, v| p.shoulder_width = v,
            |p| p.shoulder_width,
        ),
        ("hip_width", |p, v| p.hip_width = v, |p| p.hip_width),
        ("limb_length", |p, v| p.limb_length = v, |p| p.limb_length),
        ("neck_length", |p, v| p.neck_length = v, |p| p.neck_length),
        ("head_size", |p, v| p.head_size = v, |p| p.head_size),
        (
            "extremity_size",
            |p, v| p.extremity_size = v,
            |p| p.extremity_size,
        ),
    ];
    assert_non_finite_axes_fall_back("humanoid", &axes, Archetype::Humanoid);
}

#[test]
fn a_non_finite_quadruped_axis_takes_its_documented_default() {
    let axes: [Readable<QuadrupedParams>; 8] = [
        ("height", |p, v| p.height = v, |p| p.height),
        ("body_length", |p, v| p.body_length = v, |p| p.body_length),
        ("build", |p, v| p.build = v, |p| p.build),
        ("muscle", |p, v| p.muscle = v, |p| p.muscle),
        ("leg_length", |p, v| p.leg_length = v, |p| p.leg_length),
        ("neck_length", |p, v| p.neck_length = v, |p| p.neck_length),
        ("head_size", |p, v| p.head_size = v, |p| p.head_size),
        ("tail_length", |p, v| p.tail_length = v, |p| p.tail_length),
    ];
    assert_non_finite_axes_fall_back("quadruped", &axes, Archetype::Quadruped);
}

#[test]
fn every_humanoid_axis_meshes_at_its_extremes() {
    let heights = [1.2f32, 1.75, 2.2];
    let axes: [Axis<HumanoidParams>; 8] = [
        ("shoulder_width", |p, v| p.shoulder_width = v),
        ("hip_width", |p, v| p.hip_width = v),
        ("limb_length", |p, v| p.limb_length = v),
        ("neck_length", |p, v| p.neck_length = v),
        ("head_size", |p, v| p.head_size = v),
        // The head's own two (#61). `head_breadth` is the one that can fail
        // here: it widens the skull's lateral half-extent, and a socket
        // surfaces as a hull facet only when its plane clears every sibling
        // ring point, so the broad end has to clear the neck below it.
        // `face_length` moves the head joint up its neck and has no socket
        // beneath it to overlap, which is why its bound came from the
        // refinement bands instead.
        ("head_breadth", |p, v| p.head_breadth = v),
        ("face_length", |p, v| p.face_length = v),
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
                    &NEUTRAL,
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
                shoulder_width: value,
                hip_width: value,
                limb_length: value,
                neck_length: value,
                head_size: value,
                head_breadth: value,
                face_length: value,
                extremity_size: value,
            };
            params.sanitize();
            assert_meshable(
                &Archetype::Humanoid(params),
                &NEUTRAL,
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
            shoulder_width: rng.random_range(-1.0..=1.0),
            hip_width: rng.random_range(-1.0..=1.0),
            limb_length: rng.random_range(-1.0..=1.0),
            neck_length: rng.random_range(-1.0..=1.0),
            head_size: rng.random_range(-1.0..=1.0),
            head_breadth: rng.random_range(-1.0..=1.0),
            face_length: rng.random_range(-1.0..=1.0),
            extremity_size: rng.random_range(-1.0..=1.0),
        };
        params.sanitize();
        // **The composites are rolled here too, and that is the epic's rule
        // rather than an extra** (#161): a quantity is `formula(composites)`
        // then an offset, so a sweep that holds the composites neutral tests
        // half of every formula. `femininity` reaches the trunk, the hips and
        // both limb ladders (#100), and it met a wall the first time it was
        // swept — the pelvis's spine socket, at −1.30.
        // Age joins it in #167: it is the second composite to reach the cage,
        // and the one that spends a length the mesher's clearances are also
        // spending — the trunk's free spine run, which the settle takes out of
        // and two socket gaps sit either side of.
        let mut composites = Composites {
            femininity: rng.random_range(-3.0..=3.0),
            age: rng.random_range(AGE_RANGE.0..=AGE_RANGE.1),
            ..NEUTRAL
        };
        composites.sanitize();
        assert_meshable(
            &Archetype::Humanoid(params),
            &composites,
            &format!("random humanoid #{sample}: {params:?} {composites:?}"),
        );
    }
}

#[test]
fn the_frame_axis_meshes_against_every_corner_of_the_body() {
    // The composite corner product (#161's rule, #100's axis). Each end of the
    // frame axis against each corner of the per-region space, at both ends of
    // stature: the axis narrows a chest while `shoulder_width` widens it, and
    // widens a pelvis while `hip_width` narrows it, so the interesting bodies
    // are the ones where the two tiers disagree.
    //
    // The envelope ends are swept, not just ±1, because a rolled record reaches
    // them: `femininity` is a shape axis and carries the #160 stretch.
    for femininity in [-3.0f32, -1.0, 0.0, 1.0, 3.0] {
        let composites = Composites {
            femininity,
            ..NEUTRAL
        };
        for value in EXTREMES {
            for height in [1.2f32, 2.2] {
                let mut params = HumanoidParams {
                    height,
                    shoulder_width: value,
                    hip_width: -value,
                    limb_length: value,
                    neck_length: value,
                    head_size: value,
                    head_breadth: value,
                    face_length: value,
                    extremity_size: value,
                };
                params.sanitize();
                assert_meshable(
                    &Archetype::Humanoid(params),
                    &composites,
                    &format!("humanoid h={height} all={value} femininity={femininity}"),
                );
            }
        }
    }
}

#[test]
fn the_age_axis_meshes_against_every_corner_of_the_body() {
    // The same composite corner product as the frame axis above, for the axis
    // #167 added. Age is the one composite that spends a LENGTH rather than a
    // girth — the settle comes out of the trunk's free spine run, with a socket
    // clearance either side of it — so the bodies to worry about are the ones
    // whose trunk is already short: a small body at a long limb length, where
    // the girdle's floor and the pelvis's are closest together.
    //
    // Both ends of the frame axis are swept with it, because the settle is
    // interpolated between the two references' own height losses and so is
    // largest exactly where the frame is most feminine.
    for age in [AGE_RANGE.0, 50, AGE_RANGE.1] {
        for femininity in [-1.25f32, 0.0, 2.85] {
            let composites = Composites {
                age,
                femininity,
                ..NEUTRAL
            };
            for value in EXTREMES {
                for height in [1.2f32, 2.2] {
                    let mut params = HumanoidParams {
                        height,
                        shoulder_width: value,
                        hip_width: -value,
                        limb_length: value,
                        neck_length: value,
                        head_size: value,
                        head_breadth: value,
                        face_length: value,
                        extremity_size: value,
                    };
                    params.sanitize();
                    assert_meshable(
                        &Archetype::Humanoid(params),
                        &composites,
                        &format!(
                            "humanoid h={height} all={value} age={age} femininity={femininity}"
                        ),
                    );
                }
            }
        }
    }
}

#[test]
fn the_default_body_stands_near_the_proportion_canon() {
    // #66, re-based on measured bodies in #98. Shoulder and hip breadth are set
    // by coefficients that are ALSO meshability floors, so widening them back
    // would make every sweep above pass more easily and nothing else would
    // complain. Measured against the rendered height rather than the nominal
    // one, because subdivision shrinks a body by about six percent and it is
    // the rendered body that is looked at.
    //
    // **Both figures this used to assert were wrong, in different ways.**
    //
    // The shoulder bound compared `Zone::Chest`'s widest joint against 0.245,
    // the canon's *bideltoid* breadth — a measurement across the shoulder
    // muscle, which is not what any joint in this plan sits at. Two different
    // quantities were being held equal by a tolerance. It now measures where
    // the arm's chain actually starts, against the reference pair's 0.190 and
    // 0.156.
    //
    // The hip bound asserted 0.24 and explained that the canonical 0.190 was
    // unreachable — "0.190 needs a hip coefficient of about 1.13, and below
    // 1.35 the pelvis stops being able to separate two leg sockets from the
    // spine's". That was measuring the pelvis's own width, not the hips: the
    // floor is `PELVIS_SECTION.x · pelvis_r / 0.82`, so narrowing the pelvis
    // node moved it. The hips now sit at 0.098, inside the reference pair, and
    // the coefficient is 0.60.
    let params = HumanoidParams::default();
    let skeleton = params.skeleton(&symbios_avatar::Composites::default());
    let body = build_body(
        &skeleton,
        &CageConfig::default(),
        BODY_SUBDIVISIONS,
        &Default::default(),
    )
    .expect("the default body meshes");
    let (lo, hi) = body.bounds();
    let height = hi.y - lo.y;
    let rig = Rig::from_skeleton(&skeleton).expect("the default body rigs");

    // The root of each limb chain — the shoulder and the hip — rather than the
    // widest joint of a zone, which for an A-posed arm is the elbow.
    let root_span = |limb: Limb| {
        rig.limb_chain(limb)
            .map(|chain| rig.joints[chain[0]].position.x.abs() * 2.0 / height)
            .expect("a humanoid articulates its limbs")
    };

    // Above the reference pair, and knowingly: `clavicle_x` sits just over a
    // floor set by the CHEST's width, which is correct anatomy and must not be
    // narrowed to buy shoulder span. The bound is here to stop the old 0.337
    // coming back, not to claim the reference was reached.
    let shoulders = root_span(Limb::ForeLeft);
    assert!(
        (0.15..0.23).contains(&shoulders),
        "shoulders span {shoulders:.3} of height, against a reference 0.190 (male) \
         and 0.156 (female)"
    );

    let hips = root_span(Limb::HindLeft);
    assert!(
        (0.085..0.115).contains(&hips),
        "hips span {hips:.3} of height, against a reference 0.097 (male) and 0.099 (female)"
    );
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
                    build: value,
                    muscle: value.max(0.0),
                    height,
                    ..Default::default()
                };
                apply(&mut params, value);
                params.sanitize();
                assert_meshable(
                    &Archetype::Quadruped(params),
                    &NEUTRAL,
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
            build: rng.random_range(-1.0..=1.0),
            muscle: rng.random_range(0.0..=1.0),
            height: rng.random_range(0.25..=1.8),
            body_length: rng.random_range(-1.0..=1.0),
            leg_length: rng.random_range(-1.0..=1.0),
            neck_length: rng.random_range(-1.0..=1.0),
            head_size: rng.random_range(-1.0..=1.0),
            tail_length: rng.random_range(-1.0..=1.0),
        };
        params.sanitize();
        assert_meshable(
            &Archetype::Quadruped(params),
            &NEUTRAL,
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
        assert_meshable(
            &record.archetype,
            &record.composites,
            &format!("rerolled humanoid seed {seed}"),
        );

        let mut beast =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        assert_meshable(
            &beast.archetype,
            &beast.composites,
            &format!("rerolled quadruped seed {seed}"),
        );
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
        assert_meshable(
            &copy.archetype,
            &copy.composites,
            &format!("share-code copy of seed {seed}"),
        );
    }
}

/// FNV-1a over every bit a skeleton carries.
///
/// Raw bits rather than a tolerance, because the question this answers is
/// whether the arithmetic produced *the same* floats, not similar ones. Zones go
/// in through `Debug`, which is the name they carry in the source, so retagging
/// a node moves the fingerprint even when no coordinate does.
fn digest(skeleton: &Skeleton) -> u64 {
    fn eat(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for node in &skeleton.nodes {
        for value in [
            node.position.x,
            node.position.y,
            node.position.z,
            node.radius,
            node.scale.x,
            node.scale.y,
            node.roll,
            node.offset.x,
            node.offset.y,
        ] {
            eat(&mut hash, &value.to_bits().to_le_bytes());
        }
        eat(&mut hash, &[u8::from(node.marker)]);
        eat(&mut hash, format!("{:?}", node.zone).as_bytes());
    }
    for [a, b] in &skeleton.bones {
        eat(&mut hash, &a.to_le_bytes());
        eat(&mut hash, &b.to_le_bytes());
    }
    hash
}

/// The bodies the fingerprint below is taken over: both plans at their defaults,
/// the corners of the humanoid space, and the first rolls of each plan.
///
/// Corners and rolls both, because they fail differently: a corner catches a
/// coefficient that only matters at an extreme, and a roll catches one that only
/// matters in combination with another axis.
fn fingerprinted_bodies() -> Vec<(String, Skeleton)> {
    let mut bodies = vec![
        (
            "humanoid default".to_string(),
            Archetype::default(),
            NEUTRAL,
        ),
        (
            "quadruped default".to_string(),
            Archetype::Quadruped(QuadrupedParams::default()),
            NEUTRAL,
        ),
    ];

    // **The composites are part of a body now** (#100), so they are part of
    // what this ratchet holds. Without these rows the table would have gone on
    // passing while the frame axis moved every quantity it touches, which is
    // the one thing it exists to notice.
    for femininity in [-1.0f32, 1.0] {
        bodies.push((
            format!("humanoid femininity {femininity:+}"),
            Archetype::default(),
            Composites {
                femininity,
                ..NEUTRAL
            },
        ));
    }

    // **And the composites include age now** (#167), for the same reason: the
    // axis reaches the trunk's own length, both limb ladders and the waist, and
    // the rolled bodies below cannot be trusted to carry it — six of those eight
    // seeds roll an age under the pivot, where the axis is deliberately the
    // identity. Without these two rows a change to the settle would move nothing
    // this table can see.
    for age in [55u32, AGE_RANGE.1] {
        bodies.push((
            format!("humanoid age {age}"),
            Archetype::default(),
            Composites { age, ..NEUTRAL },
        ));
    }

    for value in EXTREMES {
        for height in [1.2f32, 2.2] {
            let mut params = HumanoidParams {
                height,
                shoulder_width: value,
                hip_width: value,
                limb_length: value,
                neck_length: value,
                head_size: value,
                head_breadth: value,
                face_length: value,
                extremity_size: value,
            };
            params.sanitize();
            bodies.push((
                format!("humanoid corner h={height} all={value}"),
                Archetype::Humanoid(params),
                NEUTRAL,
            ));
        }
    }

    for seed in 0..8i64 {
        let mut biped = AvatarRecord::new("Roll", Archetype::default());
        biped.reroll(seed);
        bodies.push((
            format!("humanoid seed {seed}"),
            biped.archetype,
            biped.composites,
        ));

        let mut beast = AvatarRecord::new("Roll", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        bodies.push((
            format!("quadruped seed {seed}"),
            beast.archetype,
            beast.composites,
        ));
    }

    bodies
        .into_iter()
        .map(|(name, archetype, composites)| (name, archetype.skeleton(&composites)))
        .collect()
}

/// What each body in [`fingerprinted_bodies`] hashed to when it was last judged.
///
/// See [`the_plan_builds_the_bodies_it_was_last_judged_on`] for what to do when
/// one of these moves.
///
/// **Re-based 2026-08-10 for #106**, the shoulder girdle's widening: 15 of the
/// 24 moved and all nine quadrupeds held, which is what a humanoid-only change
/// should do. Judged on `--bare` renders of the default body and seeds 3 and 21
/// before the paste, per the note on the test below.
///
/// **Re-based again the same day for #100**, the frame axis, and this time the
/// table GREW: the two `femininity` rows are new, because a body is its
/// composites as well as its archetype now and a ratchet that fingerprinted
/// only the archetype would have gone on passing while the axis moved every
/// quantity it touches. Ten of the 26 moved — those two, and the eight rolled
/// humanoids, which carry a rolled `femininity` that used to reach nothing. The
/// default body and all six corners are unmoved, which is the identity the
/// epic requires of a neutral composite. Judged on `--bare` renders at
/// `--femininity` −1, 0 and +1 before the paste.
///
/// **Re-based a third time for #164**, the allometric girth, and this one moved
/// everything: `build` and `muscle` retired, so every body here is built by a
/// different formula. Judged on `--bare` renders across the grid that issue
/// asks for — slight, neutral, muscular (`--mass 1 --fat 0.08`) and heavy
/// (`--mass 1 --fat 0.45`) — where what to look for is that the last two are
/// different bodies rather than one body at two sizes.
///
/// **Re-based a fourth time for #174**, the neck floor, and this one moved
/// exactly eight: the default body, `femininity` −1, four of the six corners,
/// and rolled seeds 3 and 5. Those eight are the bodies whose neck bone came
/// from the floor rather than from its own length term, which is the whole
/// population the change can reach — the other eighteen are bit-identical, and
/// that is the check that the fix went where it was aimed. Nothing but
/// `neck_y` and what hangs above it moved: the girdle node reads 1.32125 on the
/// default body before and after, so the trunk under it is untouched and the
/// rendered stature goes 1.7330 m back to 1.7235, which is where #164 found it.
/// Judged on `--bare` renders of the same grid, plus `--head --bare` on the
/// heavy body, where the neck sits deeper into the shoulders with the nape and
/// throat clean.
///
/// **Re-based a sixth time for #167**, the age composite, and this one names
/// its own population twice over. The table GREW by two rows — `age 55` and
/// `age 80`, added for the reason the `femininity` pair were: six of the eight
/// rolled seeds draw an age under `plan::AGE_PIVOT`, where this axis is
/// deliberately the identity, so a ratchet without them would have gone on
/// passing while the settle moved every body over thirty.
///
/// Of the 26 rows that already existed, exactly TWO moved: rolled humanoid
/// seeds 0 and 4. Those are the only two of the eight that roll an age past the
/// pivot — 43 and 37 years, for an `ageing` of 0.068 and 0.020 — and every
/// other body here holds age at its default, which is under the pivot. So the
/// default body, both `femininity` rows, all six corners and all eight
/// quadrupeds are bit-identical, and that is the acceptance #167 asks for
/// stated as a hash: neutral age is the body that was already built.
///
/// Judged on `--bare` renders at `--age` 18, 50 and 80, where the arms visibly
/// thin and the abdomen fills while the trunk holds, and on the numbers behind
/// them, because the sheet cannot see the rest: stature 1.7235 → 1.6826 m
/// (−4.1 cm, and the camera frames to the body's own height so a render can
/// never show it), the deltoid −9.4%, the wrist −3.0%, the waist +6% across and
/// +14% deep. The head terms were judged on `headaudit --axis age` rather than
/// on the head sheet, at 0.86 mm per pixel against a 3 mm change.
///
/// **Re-based a fifth time for #166's neck**, and this one names its own
/// population: the two `femininity` bodies and the eight rolled humanoids, which
/// are every body here that carries a non-neutral frame. The default body, all
/// six corners and every quadruped are bit-identical, because the corners hold
/// the composites neutral and `frame` is one at neutral. That split is the check
/// — a frame term that moved a neutral body would be a bug in the anchor pair,
/// not a new shape. Judged on `--bare` renders at `--femininity` −1 and +1,
/// where the masculine neck reads as a column into broad shoulders and the
/// feminine one as slimmer and longer without reading as a stalk.
const FINGERPRINTS: [(&str, u64); 28] = [
    ("humanoid default", 0xa4409953adde3c58),
    ("quadruped default", 0x2aabd8cffd3320f0),
    ("humanoid femininity -1", 0x4be35eca959f61bf),
    ("humanoid femininity +1", 0x53c9b142d969f642),
    ("humanoid age 55", 0x3e59557f23336ea1),
    ("humanoid age 80", 0x0b9f5241f4d9cdf8),
    ("humanoid corner h=1.2 all=-1", 0x6a9e05246c9e7acb),
    ("humanoid corner h=2.2 all=-1", 0xf2c916e94ca043f4),
    ("humanoid corner h=1.2 all=0", 0x3ac7e61821ffd7a4),
    ("humanoid corner h=2.2 all=0", 0x8f9291d4a7a5f24e),
    ("humanoid corner h=1.2 all=1", 0xef1d9b0df29fdba4),
    ("humanoid corner h=2.2 all=1", 0xc9766775e866de06),
    ("humanoid seed 0", 0xd5ea3b435fbdc851),
    ("quadruped seed 0", 0x181d22a61a29e06b),
    ("humanoid seed 1", 0xfa75b04629f8d056),
    ("quadruped seed 1", 0x66b32cdababaf760),
    ("humanoid seed 2", 0x4caf440468491466),
    ("quadruped seed 2", 0x0ca673b8f4eb9dd5),
    ("humanoid seed 3", 0xc8255cf85161ab16),
    ("quadruped seed 3", 0x5fe315cd16d9b52b),
    ("humanoid seed 4", 0x0708e31ccd146660),
    ("quadruped seed 4", 0x050364ec7b8118ea),
    ("humanoid seed 5", 0xd9a805ac31be0f9b),
    ("quadruped seed 5", 0x2d22051939b73f20),
    ("humanoid seed 6", 0x271650f71ca43f6a),
    ("quadruped seed 6", 0x94b5b20cbe08a43a),
    ("humanoid seed 7", 0x190d42dfb03ea4a7),
    ("quadruped seed 7", 0xc6b4259ae378e3bb),
];

/// Every number the two body plans produce, pinned.
///
/// **This test cannot tell a good change from a bad one and is not trying to.**
/// Every other test in this file asks whether a body meshes or whether one
/// measurement lands in a range, and a plan can be rewritten under all of them
/// while quietly moving every coordinate it produces. This one asks the
/// remaining question — *did anything move at all* — which is the only question
/// a refactor has to answer, and the one the file's own docstrings say keeps
/// being answered wrong: `clavicle_x` records a comment that was made false by a
/// change in another file that never touched this one, and `HEAD_BELOW_JOINT`
/// records that the head, the neck and the shoulders are one measurement chain
/// where any change re-tunes the face in silence.
///
/// So a failure here means the geometry moved. That is fine when it was meant
/// to: **judge the new bodies by render** — `cargo run --release --example
/// render` — and paste the table this prints into [`FINGERPRINTS`], noting the
/// issue that moved them. It is not fine when the change was supposed to be a
/// refactor, and that is what this was added for (#163).
#[test]
fn the_plan_builds_the_bodies_it_was_last_judged_on() {
    let bodies = fingerprinted_bodies();
    assert_eq!(
        bodies.len(),
        FINGERPRINTS.len(),
        "the fingerprint table has to name every body the sweep builds"
    );

    let mut moved = Vec::new();
    for ((name, skeleton), (expected_name, expected)) in bodies.iter().zip(FINGERPRINTS) {
        assert_eq!(name, expected_name, "the table is out of order");
        let actual = digest(skeleton);
        if actual != expected {
            moved.push(format!("{name}: {expected:#018x} -> {actual:#018x}"));
        }
    }

    assert!(
        moved.is_empty(),
        "{} of {} bodies changed shape:\n  {}\n\nIf that was intended, judge the \
         new bodies by render and paste this table into FINGERPRINTS:\n{}",
        moved.len(),
        bodies.len(),
        moved.join("\n  "),
        bodies
            .iter()
            .map(|(name, skeleton)| format!("    (\"{name}\", {:#018x}),", digest(skeleton)))
            .collect::<Vec<_>>()
            .join("\n")
    );
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

#[test]
fn the_neck_is_the_width_of_a_neck_on_every_head_it_carries() {
    // **The guard that makes #175 hold, and the reason it is a RATIO.** A neck
    // and the skull on it used to be sized by expressions sharing only stature:
    // `neck_r` is a fraction of it times girth times the frame axis, `head_r` a
    // fraction of it times `head_size`. So the two could disagree by a whole
    // axis, and measured over this grid before `face::neck` existed the built
    // column ran from 0.65 of the skull's own width to 1.21 of it — at
    // `head_size` −1 the neck was WIDER than the head it carried, which is what
    // the owner was looking at when they asked for the throat stump to go.
    //
    // Asserting the absolute width instead would pass on every body in that
    // sweep, because the absolute width was never the defect: it barely moved
    // at all (54–71 mm) while the skull ran 54–90. Only the ratio can see it.
    //
    // The band is one-sided on purpose. `face::neck` narrows and never widens —
    // inflating a slender column to meet a target would be that module causing
    // the defect it exists to remove — so a body whose cage already gave it a
    // thin neck stays thin, and the floor here is what that leaves: 0.644 at
    // `head_size` +1 with `femininity` +1, the leanest corner of the grid.
    //
    // **0.82 → 0.83, and it is a cost rather than a drift** (#176). The carve
    // used to set the width from a twenty-four band envelope of the column,
    // which hit the target closely and was NOISE — a maximum over a band of a
    // coarse quad tube reports where the rings fall, so adjacent bands read 52,
    // 60, 43, 57 mm and the surface stepped wherever they did. It sets the
    // width from one windowed reading at the waist now, which is smooth in
    // height by construction, and the ceiling of the spread went 0.777 to
    // 0.815 with it. That is the whole of the trade: a tenth of a point of
    // width accuracy for a column that is not stepped, on a ruler whose spread
    // was 1.86x before any of this existed and is 1.27x now.
    for &head_size in &[-1.0f32, 0.0, 1.0] {
        for &(mass, femininity) in &[(0.0f32, 0.0f32), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0)] {
            let mut record = AvatarRecord::new("Column", Archetype::default());
            if let Archetype::Humanoid(ref mut params) = record.archetype {
                params.head_size = head_size;
            }
            record.composites.mass = mass;
            record.composites.femininity = femininity;
            record.composites.sanitize();
            record.sanitize();
            let skeleton = record.skeleton();
            let body = build_body(
                &skeleton,
                &CageConfig::default(),
                BODY_SUBDIVISIONS,
                &Default::default(),
            )
            .expect("the body meshes");
            let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
            let Some(skull) = Skull::measure(&body, &rig) else {
                continue;
            };
            let head = *rig.in_zone(Zone::Head).first().expect("a head");
            let at = rig.joints[head].position;
            let (_, crown) = skull.throat_and_crown();

            // The widest chord of the section, swept over its own depth. A ray
            // fired sideways from the axis crosses an off-centre section on a
            // CHORD and reads it narrower the further the mass leans back,
            // which is the trap `examples/neckaudit` already records paying
            // for once.
            let widest = |y: f32| -> f32 {
                // Eighteen halvings of 0.40 m is 1.5 microns and this is
                // asserted to three decimal places of a ratio, so the cost of
                // the extra fourteen is a slower debug suite and nothing else.
                let bisect = |from: Vec3, along: Vec3| -> f32 {
                    let (mut inside, mut outside) = (0.0f32, 0.40f32);
                    for _ in 0..18 {
                        let middle = 0.5 * (inside + outside);
                        if body.contains(from + along * middle) {
                            inside = middle;
                        } else {
                            outside = middle;
                        }
                    }
                    inside
                };
                let axis = Vec3::new(at.x, y, at.z);
                let (ahead, behind) = (bisect(axis, Vec3::Z), bisect(axis, -Vec3::Z));
                (0..=8)
                    .map(|slice| {
                        let z = at.z - behind + (ahead + behind) * slice as f32 / 8.0;
                        bisect(Vec3::new(at.x, y, z), Vec3::X)
                    })
                    .fold(0.0f32, f32::max)
            };

            // The column is hunted from the CHIN rather than from the head's
            // floor: the waist sits well above where the head's surface stops —
            // 80 mm under the joint against a floor at 135 on the default body
            // — so a search starting at the floor starts below the neck and
            // measures the shoulders.
            let neck = *rig.in_zone(Zone::Neck).first().expect("a neck");
            let girdle = rig.joints[neck].parent.expect("a neck sits on a girdle");
            let chin = at.y + skull.chin();
            let bottom = rig.joints[girdle].position.y + rig.joints[girdle].radius;

            // A 5 mm ladder, which is finer than the rings the cage puts in
            // this region and coarse enough that the whole grid costs seconds
            // rather than minutes in a debug build.
            let mut skull_wide = 0.0f32;
            let mut y = chin;
            while y < at.y + crown {
                skull_wide = skull_wide.max(widest(y));
                y += 0.005;
            }
            let mut column = f32::MAX;
            let mut y = chin;
            while y > bottom {
                column = column.min(widest(y));
                y -= 0.005;
            }
            let ratio = column / skull_wide.max(f32::EPSILON);
            assert!(
                (0.60..0.83).contains(&ratio),
                "head_size {head_size:+.1}, mass {mass:+.1}, femininity {femininity:+.1}: \
                 the column is {:.1} mm against a skull of {:.1}, a ratio of {ratio:.3}",
                column * 1000.0,
                skull_wide * 1000.0
            );
        }
    }
}

#[test]
fn the_neck_is_the_length_of_a_neck() {
    // #93. The neck's length is not an anatomical choice anywhere in this plan:
    // it falls out of the socket-clearance floor under `neck_y`, which BINDS, so
    // the coefficient that looks like it sets the neck does not. Nothing
    // measured the result until the owner said the head-to-body transition read
    // as mangled, and it came out at 0.480 of a head height against an eight-head
    // figure that puts the shoulder line about a third of a head below the chin.
    //
    // **Measured against the shoulder SURFACE, not the clavicle joint.** The
    // joint sits about 95 mm lower, under the trapezius, so reading the span off
    // the rig predicts 190 mm where the surface gives 103 — an 85% error, and
    // the reason a body checked against canon repeatedly still had this in it.
    for seed in [0i64, 3, 7, 13, 21] {
        let mut record = AvatarRecord::new("Neck", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let body = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &Default::default(),
        )
        .expect("the body meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        let Some(skull) = Skull::measure(&body, &rig) else {
            continue;
        };
        let at = rig.joints[*rig.in_zone(Zone::Head).first().expect("a head")].position;
        let (throat, crown) = skull.throat_and_crown();
        let (chin, crown, throat) = (at.y + skull.chin(), at.y + crown, at.y + throat);

        // Bisected, like every other measurement in this crate: binning vertices
        // into height bands reports ripple that is not in the mesh.
        //
        // **The width of a slice, not a chord across it** (#148). This used to
        // fire sideways at the head joint's own `z` alone, which crosses an
        // off-centre section on a chord — the same trap `NECK_LOBE`'s docstring
        // records for an earlier probe. It went unnoticed while the neck's rear
        // lobe stood still; #148 pulled the lobe in, the chord at the joint's
        // `z` moved with it, and this ruler drifted 0.005 for a change the
        // lateral silhouette barely sees. Taking the widest reading across the
        // body's depth measures the silhouette the eye actually reads.
        let half_width = |y: f32| {
            let mut widest = 0.0f32;
            let mut probe = at.z - 0.18;
            while probe <= at.z + 0.12 {
                let (mut inside, mut outside) = (0.0f32, 0.40f32);
                for _ in 0..32 {
                    let mid = 0.5 * (inside + outside);
                    if body.contains(glam::Vec3::new(at.x + mid, y, probe)) {
                        inside = mid;
                    } else {
                        outside = mid;
                    }
                }
                widest = widest.max(inside);
                probe += 0.02;
            }
            widest
        };

        // Down from the throat to where the body is half again as wide as the
        // narrowest point of the neck: the shoulder line as an eye reads it.
        let (mut narrowest, mut y) = (f32::MAX, throat);
        while y > throat - 0.30 {
            narrowest = narrowest.min(half_width(y));
            if half_width(y) > narrowest * 1.5 {
                break;
            }
            y -= 0.001;
        }

        // **0.52 to 0.46, a ratchet rather than a fix** (#125). The bound has
        // always been the state and the eight-head figure it quotes is 0.33.
        // Giving the neck the forward lean the Quaternius reference has — its
        // neck node sits behind BOTH its parent and its child — seats the neck
        // further inside the shoulder mass, so the flare reaches higher and the
        // visible neck shortens on every one of these seeds: 0.423–0.472 to
        // 0.351–0.443.
        //
        // Not to 0.33, and what is still missing is not length. The reference's
        // surface reaches 2.0 to 2.4 times as far behind its neck axis as its
        // throat reaches in front, and ours reads 0.91 — measured from the
        // column's own axis, after a first attempt read 2.50 by measuring from
        // the midline while the axis had moved. Axis-free: the section at
        // mid-neck is 103 mm deep against the reference's 167. That is mass
        // this cage cannot express and it is #125's remaining half.
        //
        // **0.46 to 0.44, a ratchet on a state that did not move** (#129). No
        // geometry changed for this: the seeds read 0.385 to 0.429 today and
        // read the same before, and the bound is simply brought down onto them
        // so a regression cannot hide in the slack. It is still the state and
        // still not the target.
        //
        // **0.44 to 0.475, and it is the RULER that moved, not the neck**
        // (#148). Every figure above was read by the chord probe; the
        // silhouette probe reads the same bodies at 0.423–0.472, because a
        // wider `narrowest` raises the stop threshold and the walk finds the
        // shoulder line lower. Measured under this probe, #148's nape tuck
        // moved seeds 0, 3 and 7 by nothing at four decimals and brought 13
        // and 21 DOWN by 0.005 and 0.004 — the invariance the chord probe
        // lacked, and the reason it was replaced rather than its bound eased.
        // The bound is re-based onto the new instrument's state with the same
        // few-thousandths of slack the old one carried. Numbers on the two
        // rulers are not comparable.
        //
        // **And what the remaining 0.10 is made of, which is why no coefficient
        // in the neck reaches it.** `examples/neckaudit` prints this same span
        // broken into its four owners. Across these seeds:
        //
        // ```text
        //   chin to the head's own floor      42.6 – 57.8 mm   57–60%
        //   that floor to the neck joint      12.7 – 17.8       17–18%
        //   the neck joint to girdle's crown   2.1 –  5.3        2– 5%
        //   the crown to the shoulder line    15.7 – 28.7       20–27%
        // ```
        //
        // The neck BONE owns two to five millimetres of it, because the girdle's
        // crown sits directly under the neck joint by construction — the joint
        // is `girdle_y` plus a floor of `1.02 · girdle_r` and the crown is
        // `girdle_y + girdle_r`. Most of what an eye reads as neck is head-owned
        // surface below the chin, and the rest is the girdle's shoulder. #93,
        // #107 and #125 all tuned the neck; this is why each of them moved the
        // number by so much less than its own arithmetic predicted.
        // The ratchet judges bodies inside the classic range it was measured
        // over. Generator 2 (#160) deliberately rolls rare extremes past ±1,
        // and a `neck_length` of +2 is MEANT to read as an unusually long
        // neck — the wild band only refuses a neck longer than the head that
        // tops it, which is where unusual ends and detached begins.
        let Archetype::Humanoid(params) = &record.archetype else {
            panic!("archetype changed")
        };
        // Classic means every axis the RULER reads is inside ±1: the neck and
        // head own the span's top, and the shoulder line at its bottom is
        // where the flare is, which `shoulder_width` and `build` place. Under
        // generator 2 that thins this sweep's classic sample; the default
        // body and any classic roll still hold the ratchet.
        let classic = params.neck_length.abs() <= 1.0
            && params.head_size.abs() <= 1.0
            && params.shoulder_width.abs() <= 1.0;
        // **0.535 to 0.66, and this time the NECK moved** (#164). Measured over
        // the first twenty-four seeds, the six classic bodies now read 0.462,
        // 0.480, 0.503, 0.564, 0.623 and 0.637 where the bound was 0.535, and
        // the default body's rendered height went 1.724 m to 1.733 for the same
        // reason. The cause is one swept constant: `neck_y`'s floor buys the
        // girdle's neck-socket clearance in neck BONE at 1.02 girdle radii, and
        // the allometric girth grows the girdle enough that 1.02 stopped
        // clearing — 1.12 is where the 400-roll gate goes green again, and a
        // floor of 1.12 exceeds the neck's own length term at neutral, so it
        // binds on every body rather than on heavy ones.
        //
        // **This is a re-base onto a body nobody has judged yet, which is not
        // what this ratchet is for**, so it is recorded as a debt rather than a
        // decision: #174 owns the neck floor, and the number here comes down
        // when it lands. What makes it a cage question rather than a coefficient
        // one is that every bone in the trunk is a multiple of `girdle_r` — the
        // chest gap, the neck floor — so the trunk cannot thicken without
        // lengthening, and no coefficient in this file can separate them.
        //
        // **0.66 to 0.645, which is a tightening onto the state and NOT the
        // repayment the paragraph above promised** (#174). The neck floor is
        // fixed: it is a computed clearance now instead of a blanket 1.12
        // girdle radii, it no longer binds on the default body, and over the
        // 400 rolled records it binds on 172 rather than 234. **It moves this
        // ruler by nothing, because it never bound on the body this ruler
        // reads.** Seed 21 is the only classic body among the five swept here,
        // and its neck bone is its own length term — 0.1391 m against a floor
        // of 0.1201 under #164 and 0.1098 under #174 — so `max` picked the
        // length term before and picks it now. The ratio reads 0.637 either
        // way.
        //
        // So the paragraph above misattributed its own regression, and the
        // arithmetic to catch it was one subtraction: a floor cannot move a
        // body it does not bind on. What is left to own it is the rest of
        // #164 — the girth that grew `girdle_r` itself, which lifts
        // `girdle_y` through `torso_min` and the chest gap, and the retired
        // `build`/`muscle` axes, which changed which body seed 21 even is.
        // Neither is a debt this issue can pay, and 0.645 is the state plus
        // the same few thousandths of slack every re-base above carries.
        //
        // The previous re-base, kept because its lesson is the opposite one:
        //
        // **0.475 to 0.535, and it is the POPULATION that moved, not the
        // neck** (#160). The bound has always been the sweep's own state plus
        // slack, and generator 2 redrew what these five seeds roll: the one
        // still-classic body (21, neck +0.64 on shoulders −0.84) reads 0.528
        // where no old seed happened to pair those. Geometry is untouched —
        // the default body reads as it did — so this is a re-base onto the
        // new population, same instrument, same slack, still the state and
        // still not the 0.33 target.
        let bound = if classic { 0.645 } else { 1.0 };
        let ratio = (chin - y) / (crown - chin);
        assert!(
            ratio < bound,
            "seed {seed}: the chin sits {:.1} mm above the shoulder line on a \
             {:.1} mm head, a ratio of {ratio:.3}. The eight-head figure puts it \
             near 0.33; this shipped at 0.480 before #93 shortened the girdle's \
             neck floor.",
            (chin - y) * 1000.0,
            (crown - chin) * 1000.0
        );
    }
}
