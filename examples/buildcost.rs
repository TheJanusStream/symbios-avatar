//! What a build costs, and what measuring its head costs inside it (#89).
//!
//! A person's build measures its skull twice - once on the shaped head, for
//! the canon the carve reads, and once on the carved head with its mouth cut,
//! for the ears and the hair - and on the old mesh those two calls were 33 of
//! the 54 ms a geometry build took. This prices both calls on bodies staged
//! through the public API exactly as `Avatar::build_with` stages them, and the
//! whole build beside them - at the issue's geometry atlas, at the live
//! slider's draft and at the default - so a change to the measurement can be
//! costed against the build it lives in rather than against itself.
//!
//! ```text
//! cargo run --release --example buildcost              # the bodies below, 3 samples
//! cargo run --release --example buildcost -- 5         # 5 samples each
//! ```
//!
//! **Time it on a quiet machine, and compare two binaries, never two runs of
//! one.** Each figure is the fastest of its samples, which is the one least
//! disturbed by whatever else the desktop was doing; the median is printed
//! beside it so a noisy run shows itself.
//!
//! The bodies: the default record, three rolled persons, and the budget sweep's
//! dear corner (`tests/budget.rs`: seed 2, lean, thin and tall at frame 0, the
//! head long and broad), which is the dearest body the sweep builds.

use std::hint::black_box;
use std::time::Instant;

use symbios_avatar::face::{Canon, HeadTraits, Skull, carve_face, mouth};
use symbios_avatar::torso::{ChestAxes, ChestTraits, carve_chest, carve_taper, refine_lobe};
use symbios_avatar::{Archetype, Avatar, AvatarConfig, AvatarRecord, PolyMesh, Rig, build_body};

/// The bodies every row is taken over: a label and its record.
fn bodies() -> Vec<(String, AvatarRecord)> {
    let mut out = vec![(
        "default".to_owned(),
        AvatarRecord::new("Cost", Archetype::default()),
    )];
    for seed in [3, 7, 23] {
        let mut record = AvatarRecord::new("Cost", Archetype::default());
        record.reroll(seed);
        out.push((format!("seed {seed}"), record));
    }
    let mut record = AvatarRecord::new("Cost", Archetype::default());
    record.reroll(2);
    record.composites.femininity = 0.0;
    record.composites.mass = -2.0;
    record.composites.body_fat = symbios_avatar::plan::BODY_FAT_RANGE.0;
    if let Archetype::Humanoid(params) = &mut record.archetype {
        params.head_breadth = 1.0;
        params.face_length = 1.0;
        params.height = symbios_avatar::plan::humanoid_height_range().1;
    }
    record.sanitize();
    out.push(("seed 2 dear corner".to_owned(), record));
    out
}

/// The two bodies a build measures its skull on, staged as
/// `Avatar::build_with` stages them: the shaped head, and the carved head
/// with its chest carved and its mouth cut. `None` for a body that does not
/// build or has no person's head.
fn staged(record: &AvatarRecord) -> Option<(PolyMesh, PolyMesh, Rig)> {
    let config = AvatarConfig::default();
    let skeleton = record.skeleton();
    let traits = HeadTraits::of(&record.composites);
    let chest = ChestTraits::of(&record.composites).on(match &record.archetype {
        Archetype::Humanoid(params) => ChestAxes {
            volume: params.chest_volume,
            projection: params.chest_projection,
            lift: params.chest_lift,
            spacing: params.chest_spacing,
            fullness: params.chest_fullness,
        },
        _ => ChestAxes::default(),
    });
    let face_params = record.face.on(&traits);
    let body = build_body(&skeleton, &config.cage, config.subdivisions, &traits).ok()?;
    let rig = Rig::from_skeleton(&skeleton).ok()?;
    let shaped = refine_lobe(&body, &rig, &chest);
    let skull = Skull::measure(&shaped, &rig)?;
    let canon = Canon::measure(&rig, &skull, &record.eyes);
    let mut carved = shaped.clone();
    carve_face(&mut carved, &rig, &canon, &face_params);
    carve_chest(&mut carved, &rig, &chest);
    carve_taper(&mut carved, &rig, &chest);
    let _ = mouth::open(&mut carved, &rig, &canon, &face_params);
    Some((shaped, carved, rig))
}

/// The fastest and the median of `samples` timings of `run`, in milliseconds.
fn clock(samples: usize, mut run: impl FnMut()) -> (f64, f64) {
    run();
    let mut taken: Vec<f64> = (0..samples)
        .map(|_| {
            let started = Instant::now();
            run();
            started.elapsed().as_secs_f64() * 1e3
        })
        .collect();
    taken.sort_by(f64::total_cmp);
    (taken[0], taken[taken.len() / 2])
}

fn main() {
    let samples = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(3usize)
        .max(1);
    // The issue's "geometry only" (#89), and the live slider's draft as
    // `bevy_symbios_avatar`'s editor rebuilds it while an axis moves (#87).
    let geometry = AvatarConfig {
        atlas: 32,
        ..AvatarConfig::default()
    };
    let draft = AvatarConfig {
        atlas: 256,
        far_hair: true,
        ..AvatarConfig::default()
    };
    println!("fastest (median) of {samples}, ms");
    println!(
        "{:20} {:>17} {:>17} {:>17} {:>17} {:>17}",
        "body", "measure shaped", "measure carved", "build atlas 32", "draft 256", "build default"
    );
    for (label, record) in bodies() {
        let Some((shaped, carved, rig)) = staged(&record) else {
            println!("{label:20} does not stage");
            continue;
        };
        let first = clock(samples, || {
            black_box(Skull::measure(black_box(&shaped), &rig));
        });
        let second = clock(samples, || {
            black_box(Skull::measure(black_box(&carved), &rig));
        });
        let small = clock(samples, || {
            black_box(Avatar::build_with(&record, &geometry));
        });
        let drafted = clock(samples, || {
            black_box(Avatar::build_with(&record, &draft));
        });
        let full = clock(samples, || {
            black_box(Avatar::build(&record));
        });
        let cell = |(fast, middle): (f64, f64)| format!("{fast:8.2} ({middle:6.2})");
        println!(
            "{label:20} {:>17} {:>17} {:>17} {:>17} {:>17}",
            cell(first),
            cell(second),
            cell(small),
            cell(drafted),
            cell(full)
        );
    }
}
