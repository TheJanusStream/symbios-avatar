//! Which corners of the slider axes refuse to build, and why (#391).
//!
//! An editor's sliders reach every axis's envelope end, which no roll does.
//! This draws `--samples` corners - every signed axis at -3, 0 or 3, the
//! composites at their ends, the height anywhere in its envelope - and
//! reports each one the cage does not hull or the rig does not root, with the
//! cage's own reason, and a tally of the reasons at the end.
//!
//! Before #391 about one beast corner in thirteen refused, nearly all of them
//! at a girdle (`QuadrupedParams::skeleton` names the families); a beast's
//! skeleton now eases such a body until it meshes, so this should report
//! none. `--people` runs the same census over people, whose plan does not
//! ease.
//!
//! ```text
//! cargo run --release --example cornercensus                  # 3,000 beast corners
//! cargo run --release --example cornercensus -- --samples 20000 --seed 7
//! cargo run --release --example cornercensus -- --people      # the same over people
//! cargo run --release --example cornercensus -- --show 13     # corner 13's nodes, and why
//! cargo run --release --example cornercensus -- --prints      # a fingerprint per skeleton
//! ```

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, HumanoidParams, QuadrupedParams, Rig, Skeleton,
    build_cage,
    plan::{AGE_RANGE, BODY_FAT_RANGE, Family},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|v| v.parse::<u64>().ok())
    };
    let samples = value("--samples").unwrap_or(3000);
    let seed = value("--seed").unwrap_or(0x391);
    let people = args.iter().any(|arg| arg == "--people");
    // `--show N`: corner N's skeleton, node by node.
    let show = value("--show");
    // `--prints`: a fingerprint of every corner's skeleton and of 2,000
    // rolled ones instead of the refusals, to diff what two builds make.
    let prints = args.iter().any(|arg| arg == "--prints");
    if prints {
        for seed in 0..2000 {
            let archetype = if people {
                Archetype::default()
            } else {
                Archetype::Quadruped(QuadrupedParams::default())
            };
            let record = AvatarRecord::rolled("Roll", archetype, seed);
            println!("roll {seed} {:016x}", fingerprint(&record.skeleton()));
        }
    }

    let mut rng = Pcg64Mcg::seed_from_u64(seed);
    let mut refused = 0;
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for sample in 0..samples {
        let record = corner(&mut rng, people);
        let skeleton = record.skeleton();
        if prints {
            println!("corner {sample} {:016x}", fingerprint(&skeleton));
            continue;
        }
        if show == Some(sample) {
            for (index, node) in skeleton.nodes.iter().enumerate() {
                println!("{index:3} {node:?}");
            }
        }
        let why = match build_cage(&skeleton, &CageConfig::default()) {
            Err(error) => Some(format!("cage: {error}")),
            Ok(_) => Rig::from_skeleton(&skeleton)
                .err()
                .map(|error| format!("rig: {error}")),
        };
        if let Some(why) = why {
            refused += 1;
            // The reason with its measures cut, so the joints group.
            let kind = why
                .split(" must sit")
                .next()
                .and_then(|kind| kind.split(" (centres").next())
                .and_then(|kind| kind.split(" left ").next())
                .unwrap_or(&why);
            *reasons.entry(kind.to_string()).or_default() += 1;
            println!("#{sample}: {why}");
            println!("    {:?}", record.archetype);
            println!("    {:?}", record.composites);
        }
    }
    if !prints {
        println!("{refused} of {samples} refused");
        for (kind, count) in reasons {
            println!("  {count:5}  {kind}");
        }
    }
}

/// One corner: every signed axis at an end or the middle, the composites at
/// theirs, the height anywhere, sanitised as a record from outside is.
fn corner(rng: &mut Pcg64Mcg, people: bool) -> AvatarRecord {
    let end = |rng: &mut Pcg64Mcg| [-3.0f32, 0.0, 3.0][rng.random_range(0..3)];
    let archetype = if people {
        Archetype::Humanoid(HumanoidParams {
            height: rng.random_range(0.0..=4.0),
            shoulder_width: end(rng),
            hip_width: end(rng),
            limb_length: end(rng),
            neck_length: end(rng),
            head_size: end(rng),
            head_breadth: end(rng),
            face_length: end(rng),
            extremity_size: end(rng),
            chest_volume: end(rng),
            chest_projection: end(rng),
            chest_lift: end(rng),
            chest_spacing: end(rng),
            chest_fullness: end(rng),
        })
    } else {
        let families = [Family::Dog, Family::Cat, Family::Horse];
        let family = families[rng.random_range(0..3)];
        let hybrid = rng
            .random_bool(0.3)
            .then(|| families[rng.random_range(0..3)]);
        Archetype::Quadruped(QuadrupedParams {
            height: rng.random_range(0.0..=4.3),
            body_length: end(rng),
            leg_length: end(rng),
            neck_length: end(rng),
            head_size: end(rng),
            tail_length: end(rng),
            family,
            hybrid,
            blend: rng.random_range(0.0..=1.0),
            muzzle_length: end(rng),
            stance: end(rng),
            neck_carriage: end(rng),
            tail_carriage: end(rng),
            chest_depth: end(rng),
            appeal: end(rng),
            muzzle_breadth: end(rng),
            stop: end(rng),
            eye_set: end(rng),
            ear_size: end(rng),
            ear_point: end(rng),
            ear_carriage: end(rng),
            whiskers: end(rng),
            ..Default::default()
        })
    };
    let mut record = AvatarRecord::new("Corner", archetype);
    record.composites.mass = end(rng);
    record.composites.femininity = end(rng);
    record.composites.body_fat = [BODY_FAT_RANGE.0, 0.2, BODY_FAT_RANGE.1][rng.random_range(0..3)];
    record.composites.age = [AGE_RANGE.0, AGE_RANGE.1][rng.random_range(0..2)];
    record.sanitize();
    record
}

/// A skeleton's every node, folded to one number.
fn fingerprint(skeleton: &Skeleton) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{skeleton:?}").hash(&mut hasher);
    hasher.finish()
}
