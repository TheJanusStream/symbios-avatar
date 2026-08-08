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
//! **Three tables, and the first decides what the other two measure:** the two
//! skeletons side by side, ours by zone and ordinal — which is how a [`Slot`]
//! addresses a joint — and the reference's by its own hierarchy, both at rest.
//!
//! Three things were measured off that table before any retargeting was written,
//! and all three are on #139:
//!
//! * **Our arms rest forty degrees below horizontal and the reference's rest dead
//!   level.** An A-pose against a true T-pose, whatever the comment at the arm's
//!   construction still says. That ruled out transferring motion as a per-joint
//!   delta — cheap, and it carries roll for free — because an absolute pose would
//!   land forty degrees out.
//! * **The two rigs put their same-named side on opposite signs of `X`**, and
//!   both bodies face `+Z`. See #142; the correspondence maps by anatomy rather
//!   than by name.
//! * **A clip has to be baked against a built body's rig, not a plan's.** The
//!   plan gives 33 joints and `Avatar::build` gives 73; the difference is the
//!   hands and feet, which is where forty of the reference's sixty-six joints
//!   have to land.
//!
//! The second table is the transfer's contract: every bone of ours must point
//! where the bone it follows points. The third is the feet, reported as the
//! SLOWEST frame-to-frame step rather than as travel during a stance this would
//! have to guess the bounds of — a foot that is ever genuinely planted stops, and
//! one that skates never does.
//!
//! ```text
//! cargo run --release --example retargetaudit
//! cargo run --release --example retargetaudit -- --clip Idle_A
//! ```
//!
//! [`Slot`]: symbios_avatar::Slot

use symbios_avatar::gltf::Gltf;
use symbios_avatar::retarget::{self, Correspondence};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Limb, Pose, Rig, Vec3, Zone};

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
    // Through `Avatar::build` rather than the plan's skeleton, and the
    // difference is not cosmetic: the plan rigs 33 joints and a built body 73.
    // Everything between is the hands and the feet, which is where forty of the
    // reference's sixty-six joints have to land, and a bake run against the
    // plan's rig would come out fingerless with nothing to report it.
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the body could not be built");
        return;
    };
    ours(&avatar.rig);

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
    transferred(&library, &avatar, &wanted);
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

/// What the retarget did, against the motion it came from.
///
/// Two tables. The first is the contract the transfer is written to: every bone
/// of ours must point where the bone it follows points. The second is the defect
/// a retarget produces that nothing else here would catch — a foot that lands in
/// the right place and slides while it is there.
fn transferred(library: &Gltf, avatar: &Avatar, wanted: &str) {
    let rig = &avatar.rig;
    let Ok(skin) = library.skin(0) else {
        return;
    };
    let matched = match Correspondence::human(rig, library, &skin) {
        Ok(matched) => matched,
        Err(error) => {
            eprintln!("the reference does not match this body: {error}");
            return;
        }
    };
    let Some(clip) = library.clip(wanted) else {
        return;
    };
    let duration = library.duration(clip).unwrap_or(0.0);

    println!(
        "\nTHE RETARGET of {wanted}: {} joints matched",
        matched.len()
    );
    let mut worst = 0.0f32;
    let mut worst_at = String::new();
    let steps = 24;
    for step in 0..steps {
        let time = duration * step as f32 / steps as f32;
        let Ok(posed) = library.sample(clip, time) else {
            continue;
        };
        let mut pose = Pose::rest(rig);
        matched.pose_into(rig, &posed, &mut pose);
        let ours = pose.forward(rig);
        for (joint, off) in matched.bone_errors(&posed, &ours) {
            if off > worst {
                worst = off;
                worst_at = format!("{:?} at {time:.2} s", rig.joints[joint].zone);
            }
        }
    }
    println!("  worst bone pointing off by {worst:.3} degrees, {worst_at}");

    let baked = match retarget::clip(rig, library, &matched, clip, 30.0, true) {
        Ok(baked) => baked,
        Err(error) => {
            eprintln!("the clip did not bake: {error}");
            return;
        }
    };
    println!(
        "  baked: {} frames at {:.0} fps, {} of {} tracks move, {} bytes ({:.1} KiB), root {}",
        baked.frames,
        baked.rate,
        baked.moving(),
        baked.tracks.len(),
        baked.bytes(),
        baked.bytes() as f32 / 1024.0,
        if baked.root.is_empty() {
            "still"
        } else {
            "travels"
        },
    );

    // The feet. A contact is meant to hold still while it is down, and a
    // retarget onto legs of another length is exactly what makes it slide.
    println!("  the feet, frame to frame, in millimetres travelled by each toe:");
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let Some(toe) = symbios_avatar::Slot::new(Zone::Extremity(limb), 3).resolve(rig) else {
            continue;
        };
        let mut lowest = f32::MAX;
        let mut travel = Vec::new();
        let mut previous: Option<Vec3> = None;
        for frame in 0..baked.frames {
            let pose = baked.pose(rig, frame as f32 / baked.rate);
            let at = pose.forward(rig).positions[toe];
            lowest = lowest.min(at.y);
            if let Some(was) = previous {
                travel.push((at, (at - was).length()));
            }
            previous = Some(at);
        }
        // **Reported as the SLOWEST step rather than as travel during a stance
        // this has to guess the bounds of.** A foot that is ever genuinely
        // planted stops, so its slowest frame is near zero; one that skates
        // never stops, and the figure says so without anything having to decide
        // where the stance began. A threshold-based version of this read 4 of 49
        // frames as planted, which measured the threshold rather than the feet.
        let slowest = travel
            .iter()
            .fold(f32::MAX, |slowest, (_, moved)| slowest.min(*moved));
        let quickest = travel
            .iter()
            .fold(0.0f32, |worst, (_, moved)| worst.max(*moved));
        let highest = travel.iter().fold(f32::MIN, |high, (at, _)| high.max(at.y));
        println!(
            "    {limb:?}: slowest step {:.1} mm, quickest {:.1} mm, toe rises {:.0} mm over the cycle",
            slowest * 1000.0,
            quickest * 1000.0,
            (highest - lowest) * 1000.0,
        );
    }
}
