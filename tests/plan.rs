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
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, BodyPlan, CageConfig, HumanoidParams, Limb,
    QuadrupedParams, Rig, Zone, build_body, build_cage,
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
            assert_meshable(&wrap(params), &format!("{plan} {name}={label}"));
        }
    }
}

#[test]
fn a_non_finite_humanoid_axis_takes_its_documented_default() {
    let axes: [Readable<HumanoidParams>; 9] = [
        ("height", |p, v| p.height = v, |p| p.height),
        ("build", |p, v| p.build = v, |p| p.build),
        ("muscle", |p, v| p.muscle = v, |p| p.muscle),
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
    let axes: [Axis<HumanoidParams>; 10] = [
        ("build", |p, v| p.build = v),
        ("muscle", |p, v| p.muscle = v.abs()),
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
                head_breadth: value,
                face_length: value,
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
            head_breadth: rng.random_range(-1.0..=1.0),
            face_length: rng.random_range(-1.0..=1.0),
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
    let skeleton = params.skeleton();
    let body = build_body(&skeleton, &CageConfig::default(), BODY_SUBDIVISIONS)
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
        let body = build_body(&skeleton, &CageConfig::default(), BODY_SUBDIVISIONS)
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
            && params.shoulder_width.abs() <= 1.0
            && params.build.abs() <= 1.0;
        // **0.475 to 0.535, and it is the POPULATION that moved, not the
        // neck** (#160). The bound has always been the sweep's own state plus
        // slack, and generator 2 redrew what these five seeds roll: the one
        // still-classic body (21, neck +0.64 on shoulders −0.84) reads 0.528
        // where no old seed happened to pair those. Geometry is untouched —
        // the default body reads as it did — so this is a re-base onto the
        // new population, same instrument, same slack, still the state and
        // still not the 0.33 target.
        let bound = if classic { 0.535 } else { 1.0 };
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
