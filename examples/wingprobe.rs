//! Measures a winged body against docs/winged-anatomy.md (#420).
//!
//! ```text
//! cargo run --release --example wingprobe                # the three families at their anchors, and crosses
//! cargo run --release --example wingprobe -- --appeal 1  # at the roll's default appeal
//! cargo run --release --example wingprobe -- --sweep 400 # how many rolls build, and the first that does not
//! cargo run --release --example wingprobe -- --crosses   # every cross at blends a twentieth apart
//! cargo run --release --features serde-avatar --example wingprobe -- --write DIR  # records, codes and built bodies for an older build to read
//! ```
//!
//! Every length is printed in trunks, `T`, the shoulder joint to the hip
//! joint, as the derive builds them, and the bones against the humerus as
//! research-10 gives them.

use symbios_avatar::plan::Lineage;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, BodyPlan, CageConfig, Composites, Limb, Rig, Skeleton,
    WingedFamily, WingedParams, Zone, build_cage,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
    };
    let appeal = value("--appeal")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);
    if let Some(count) = value("--sweep").and_then(|v| v.parse::<i64>().ok()) {
        sweep(count);
        return;
    }
    if args.iter().any(|arg| arg == "--crosses") {
        crosses();
        return;
    }
    if let Some(dir) = value("--write") {
        write(std::path::Path::new(dir));
        return;
    }
    use WingedFamily::{Bat, Bird, Glider};
    let bodies = [
        ("bird", Lineage::pure(Bird)),
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bird x bat 0.3", Lineage::cross(Bird, Bat, 0.3)),
        ("bird x bat 0.7", Lineage::cross(Bird, Bat, 0.7)),
        ("bat x glider 0.5", Lineage::cross(Bat, Glider, 0.5)),
        ("glider x bird 0.5", Lineage::cross(Glider, Bird, 0.5)),
    ];
    for (name, lineage) in bodies {
        let mut params = WingedParams::default();
        symbios_avatar::plan::Blended::set_lineage(&mut params, lineage);
        params.appeal = appeal;
        params.sanitize();
        report(name, &params);
    }
}

/// Prints one body's measures.
fn report(name: &str, params: &WingedParams) {
    let composites = Composites::default();
    let skeleton = params.skeleton(&composites);
    let cage = build_cage(&skeleton, &CageConfig::default());
    let meshes = match &cage {
        Ok(cage) if cage.manifold_report().is_clean() => "meshes".to_string(),
        Ok(cage) => format!("NOT CLEAN {:?}", cage.manifold_report()),
        Err(error) => format!("REFUSED: {error}"),
    };
    println!("{name}: {meshes}");
    if std::env::args().any(|arg| arg == "--dump") {
        for (index, node) in skeleton.nodes.iter().enumerate() {
            println!(
                "    {index:2} {:?}{} at ({:+.3}, {:+.3}, {:+.3}) r {:.3} scale {:?}",
                node.zone,
                if node.marker { " marker" } else { "" },
                node.position.x,
                node.position.y,
                node.position.z,
                node.radius,
                node.scale
            );
        }
    }
    let Ok(rig) = Rig::from_skeleton(&skeleton) else {
        println!("  does not rig");
        return;
    };
    measure(&skeleton, &rig);
    let record = AvatarRecord::new("Probe", Archetype::Winged(*params));
    match Avatar::build(&record) {
        Some(avatar) => {
            let (low, high) = avatar
                .parts
                .body
                .positions
                .iter()
                .fold((glam_min(), glam_max()), |(low, high), &at| {
                    (low.min(at), high.max(at))
                });
            // How much of the skin the fore limb holds: on a bird, whose wing
            // arm is all markers inside its flank, none should be its.
            let fore: Vec<usize> = [Limb::ForeLeft, Limb::ForeRight]
                .into_iter()
                .flat_map(|limb| {
                    [
                        Zone::UpperLimb(limb),
                        Zone::LowerLimb(limb),
                        Zone::Extremity(limb),
                    ]
                })
                .flat_map(|zone| avatar.rig.in_zone(zone))
                .collect();
            let held = avatar
                .parts
                .weights
                .vertices
                .iter()
                .filter(|influences| {
                    influences
                        .iter()
                        .any(|i| i.weight > 0.05 && fore.contains(&(i.joint as usize)))
                })
                .count();
            println!("  fore limb holds {held} skin vertices");
            println!(
                "  built: {} body vertices, extent {:.3} x {:.3} x {:.3} m, lowest {:+.3}",
                avatar.parts.body.positions.len(),
                high.x - low.x,
                high.y - low.y,
                high.z - low.z,
                low.y
            );
        }
        None => println!("  DOES NOT BUILD"),
    }
}

fn glam_min() -> symbios_avatar::Vec3 {
    symbios_avatar::Vec3::splat(f32::MAX)
}

fn glam_max() -> symbios_avatar::Vec3 {
    symbios_avatar::Vec3::splat(f32::MIN)
}

/// The bones and the trunk, in trunks and against the humerus.
fn measure(skeleton: &Skeleton, rig: &Rig) {
    let at = |zone: Zone| rig.in_zone(zone);
    let position = |joint: usize| rig.joints[joint].position;
    let pelvis = at(Zone::Pelvis)[0];
    let fore_upper = at(Zone::UpperLimb(Limb::ForeLeft));
    let fore_lower = at(Zone::LowerLimb(Limb::ForeLeft));
    let fore_hand = at(Zone::Extremity(Limb::ForeLeft));
    let hind_upper = at(Zone::UpperLimb(Limb::HindLeft));
    let hind_lower = at(Zone::LowerLimb(Limb::HindLeft));
    let hind_foot = at(Zone::Extremity(Limb::HindLeft));
    let (shoulder, elbow, wrist) = (fore_upper[1], fore_upper[2], fore_lower[0]);
    let (hip, knee, ankle) = (hind_upper[0], hind_upper[1], hind_lower[0]);
    // The trunk as the plan names it: the shoulder joint to the hip, in the
    // side plane (a stretched body's is longer than its frame's).
    let side = |joint: usize| {
        let at = position(joint);
        symbios_avatar::Vec3::new(0.0, at.y, at.z)
    };
    let trunk = side(shoulder).distance(side(hip));
    let bone = |a: usize, b: usize| position(a).distance(position(b));
    let humerus = bone(shoulder, elbow);
    println!(
        "  trunk {trunk:.3} m; humerus {:.2} T; forearm {:.2} HUM, hand {:.2} HUM, digit {:.2} HUM",
        humerus / trunk,
        bone(elbow, wrist) / humerus,
        bone(wrist, fore_hand[0]) / humerus,
        bone(fore_hand[0], fore_hand[1]) / humerus,
    );
    println!(
        "  femur {:.2} HUM, tibia {:.2} HUM, metatarsus {:.2} HUM; hip {:.2} T up, shoulder {:.2} T up",
        bone(hip, knee) / humerus,
        bone(knee, ankle) / humerus,
        bone(ankle, hind_foot[0]) / humerus,
        position(hip).y / trunk,
        position(shoulder).y / trunk,
    );
    let head = at(Zone::Head);
    let nose = *head.last().expect("a head");
    let tail = at(Zone::Tail)[0];
    println!(
        "  head-and-body (nose to tail root) {:.3} m = {:.2} T; tail root to tip {:.2} T; pelvis at {:.2} T up; legs {:?}",
        position(nose).distance(position(tail)),
        position(nose).distance(position(tail)) / trunk,
        bone(tail, *at(Zone::Tail).last().expect("a tail")) / trunk,
        position(pelvis).y / trunk,
        rig.legs(),
    );
    let _ = skeleton;
}

/// Rolls `count` winged records and reports which mesh.
fn sweep(count: i64) {
    let mut failed = 0;
    for seed in 0..count {
        let mut record = AvatarRecord::new("Sweep", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        let skeleton = record.skeleton();
        match build_cage(&skeleton, &CageConfig::default()) {
            Ok(cage) if cage.manifold_report().is_clean() => {}
            Ok(_) => {
                failed += 1;
                println!("seed {seed}: not clean");
            }
            Err(error) => {
                failed += 1;
                if failed <= 10 {
                    println!("seed {seed}: {error}; {:?}", record.archetype);
                }
            }
        }
    }
    println!("{failed} of {count} rolls refused");
}

/// Every cross of two families at blends a twentieth apart, at the axes'
/// centres and at both ends of every axis together.
fn crosses() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut refused = 0;
    let mut total = 0;
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for step in 0..=20 {
            for value in [-3.0f32, -1.0, 0.0, 1.0, 3.0] {
                for size in [0.05f32, 0.6, 2.4] {
                    let mut params = WingedParams {
                        family: a,
                        hybrid: Some(b),
                        blend: step as f32 / 20.0,
                        size,
                        wing_length: value,
                        neck_length: value,
                        leg_length: value,
                        tail_length: value,
                        head_size: value,
                        appeal: value,
                    };
                    params.sanitize();
                    total += 1;
                    let skeleton = params.skeleton(&Composites::default());
                    if let Err(error) = build_cage(&skeleton, &CageConfig::default()) {
                        refused += 1;
                        if refused <= 12 {
                            println!(
                                "{a:?} x {b:?} at {:.2}, all {value}, size {size}: {error}",
                                params.blend
                            );
                        }
                    }
                }
            }
        }
    }
    println!("{refused} of {total} refused");
}

/// Writes, for a build that has no winged plan to read (#420): each family's
/// record as JSON, its share code, and under `serde-avatar` its built body as
/// the worker sends one (named msgpack).
fn write(dir: &std::path::Path) {
    use WingedFamily::{Bat, Bird, Glider};
    std::fs::create_dir_all(dir).expect("the directory");
    for (name, lineage) in [
        ("bird", Lineage::pure(Bird)),
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("cross", Lineage::cross(Bird, Bat, 0.4)),
    ] {
        let mut params = WingedParams::default();
        symbios_avatar::plan::Blended::set_lineage(&mut params, lineage);
        let mut record = AvatarRecord::new(name, Archetype::Winged(params));
        record.seed = 7;
        let json = serde_json::to_string(&record).expect("a record writes");
        std::fs::write(dir.join(format!("{name}.json")), json).expect("writes");
        std::fs::write(dir.join(format!("{name}.code")), record.share_code()).expect("writes");
        #[cfg(feature = "serde-avatar")]
        {
            let avatar = Avatar::build(&record).expect("builds");
            let packed = rmp_serde::to_vec_named(&avatar).expect("packs");
            std::fs::write(dir.join(format!("{name}.avatar")), packed).expect("writes");
        }
        println!("wrote {name}");
    }
}
