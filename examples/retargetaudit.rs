//! Measures the two skeletons against each other, and a retarget against the
//! motion it came from.
//!
//! **The instrument #139 is built before, not after.** Retargeting is a place
//! where a wrong result looks exactly like a right one at a glance: a limb that
//! points the right way with the wrong roll, a foot that lands in the right
//! place and slides while it is there, a clip that reads fine on the default
//! body and folds on a short one. None of those is visible in a contact sheet
//! and all of them are numbers.
//!
//! **What it prints today is the first of three tables**, and it is the one that
//! had to come first because it decides the strategy: the two skeletons side by
//! side, ours by zone and ordinal — which is how a [`Slot`] addresses a joint —
//! and the reference's by its own hierarchy, both at rest.
//!
//! Three things were measured off it before any retargeting was written, and all
//! three are on #139:
//!
//! * **Our arms rest forty degrees below horizontal and the reference's rest dead
//!   level.** A T-pose against an A-pose, whatever the comment at the arm's
//!   construction still says. That rules out transferring motion as a per-joint
//!   delta — cheap, and it carries twist for free — because an absolute pose
//!   would land forty degrees out. Directions have to be matched instead, and
//!   twist recovered explicitly.
//! * **The two rigs put their same-named side on opposite signs of `X`**, and
//!   both bodies face `+Z`. See #142; this maps by anatomy rather than by name.
//! * **A clip has to be baked against a built body's rig, not a plan's.** The
//!   plan gives 33 joints and `Avatar::build` gives 73; the difference is the
//!   hands and feet, which is where forty of the reference's sixty-six joints
//!   have to land.
//!
//! The other two tables arrive with the retargeter they measure: per-joint
//! angular error against the reference pose, and how far a planted foot travels
//! while it is meant to be planted — which is the defect a retarget produces
//! that nothing else here would catch.
//!
//! ```text
//! cargo run --release --example retargetaudit
//! cargo run --release --example retargetaudit -- --clip Idle_A
//! ```
//!
//! [`Slot`]: symbios_avatar::Slot

use symbios_avatar::gltf::Gltf;
use symbios_avatar::{Archetype, AvatarRecord, Limb, Rig, Vec3, Zone};

/// Where the CC0 reference animations sit, relative to this checkout.
const LIBRARY: &str = "../mesh2motion-app/static/animations/human-base-animations.glb";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let wanted = value("--clip").unwrap_or_else(|| "Walk".into());

    let record = AvatarRecord::new("Retargeted", Archetype::default());
    let Ok(rig) = Rig::from_skeleton(&record.skeleton()) else {
        eprintln!("the body could not be rigged");
        return;
    };
    ours(&rig);

    let Ok(bytes) = std::fs::read(LIBRARY) else {
        println!("\n{LIBRARY} is not checked out beside this repository, so the reference half");
        println!("of this audit cannot run. The CC0 animations live in the mesh2motion repo.");
        return;
    };
    let library = match Gltf::read(&bytes) {
        Ok(library) => library,
        Err(error) => {
            eprintln!("the reference library could not be read: {error}");
            return;
        }
    };
    theirs(&library, &wanted);
}

/// Our rig, by zone, in the order a [`Slot`] addresses it.
///
/// [`Slot`]: symbios_avatar::Slot
fn ours(rig: &Rig) {
    println!("OUR RIG: {} joints, by zone and ordinal", rig.len());
    println!("  a Slot is (zone, ordinal), and the ordinal is this column");
    let mut zones = vec![
        Zone::Pelvis,
        Zone::Abdomen,
        Zone::Chest,
        Zone::Neck,
        Zone::Head,
    ];
    for limb in [
        Limb::ForeLeft,
        Limb::ForeRight,
        Limb::HindLeft,
        Limb::HindRight,
    ] {
        zones.push(Zone::UpperLimb(limb));
        zones.push(Zone::LowerLimb(limb));
        zones.push(Zone::Extremity(limb));
    }
    for zone in zones {
        let joints = rig.in_zone(zone);
        if joints.is_empty() {
            continue;
        }
        println!("  {zone:?}: {} joints", joints.len());
        for (ordinal, &joint) in joints.iter().enumerate() {
            let at = rig.joints[joint].position;
            let parent = rig.joints[joint]
                .parent
                .map_or("root".to_string(), |parent| format!("{parent}"));
            println!(
                "    [{ordinal:2}] joint {joint:2}, parent {parent:>4}, at {:+7.1},{:+7.1},{:+7.1} mm",
                at.x * 1000.0,
                at.y * 1000.0,
                at.z * 1000.0,
            );
        }
    }
}

/// The reference skeleton, its rest shape and one clip's extent.
fn theirs(library: &Gltf, wanted: &str) {
    let Ok(skin) = library.skin(0) else {
        eprintln!("the reference has no skin");
        return;
    };
    let Ok(rest) = library.rest() else {
        eprintln!("the reference's rest pose could not be read");
        return;
    };
    let root = rest[skin.nodes[0]].transform_point3(Vec3::ZERO);

    println!(
        "\nTHE REFERENCE: {} joints, at rest, in millimetres about its own root",
        skin.len()
    );
    for (ordinal, &node) in skin.nodes.iter().enumerate() {
        let at = rest[node].transform_point3(Vec3::ZERO) - root;
        let parent =
            skin.parents[ordinal].map_or("root".to_string(), |parent| skin.names[parent].clone());
        println!(
            "  {:>20}  parent {:>20}  at {:+7.1},{:+7.1},{:+7.1} mm",
            skin.names[ordinal],
            parent,
            at.x * 1000.0,
            at.y * 1000.0,
            at.z * 1000.0,
        );
    }

    let Some(clip) = library.clip(wanted) else {
        println!("\nthe library has no clip called {wanted}");
        return;
    };
    let duration = library.duration(clip).unwrap_or(0.0);
    println!("\n{wanted}: {duration:.3} s");
}
