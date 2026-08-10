//! Where each record axis stops building, measured (#160).
//!
//! The exploration envelope stretches every shape axis [`EXPLORE`]× past its
//! conservative range, and the one contract that cannot bend is that a
//! sanitised record always builds. This walks each axis from its default
//! toward each envelope end, one at a time with everything else at default,
//! and bisects the last value that [`Avatar::build`] accepts — the measured
//! wall the envelope must stay inside. It then rolls a run of seeds and
//! reports every one that fails to build, which is what catches the walls
//! axes only hit in pairs.
//!
//! [`EXPLORE`]: symbios_avatar::plan::EXPLORE

use symbios_avatar::{Archetype, Avatar, AvatarRecord, HumanoidParams, plan::explore_range};

/// How finely the boundary is bisected, in halvings of the span.
const BISECTIONS: usize = 12;

/// How a sweep writes one axis into a record.
type Setter = Box<dyn Fn(&mut AvatarRecord, f32)>;

/// One axis to sweep: name, default, envelope, and how to set it.
type Axis = (&'static str, f32, (f32, f32), Setter);

/// Wraps a humanoid-plan field write.
fn body(set: impl Fn(&mut HumanoidParams, f32) + 'static) -> Setter {
    Box::new(move |record, value| {
        if let Archetype::Humanoid(params) = &mut record.archetype {
            set(params, value);
        }
    })
}

fn main() {
    let unit = explore_range(0.5, (0.0, 1.0));
    let signed = explore_range(0.0, (-1.0, 1.0));
    let axes: Vec<Axis> = vec![
        (
            "height",
            1.75,
            explore_range(1.75, (1.2, 2.2)),
            body(|p, v| p.height = v),
        ),
        (
            "shoulderWidth",
            0.0,
            signed,
            body(|p, v| p.shoulder_width = v),
        ),
        ("hipWidth", 0.0, signed, body(|p, v| p.hip_width = v)),
        ("limbLength", 0.0, signed, body(|p, v| p.limb_length = v)),
        ("neckLength", 0.0, signed, body(|p, v| p.neck_length = v)),
        ("headSize", 0.0, signed, body(|p, v| p.head_size = v)),
        ("headBreadth", 0.0, signed, body(|p, v| p.head_breadth = v)),
        ("faceLength", 0.0, signed, body(|p, v| p.face_length = v)),
        (
            "extremitySize",
            0.0,
            signed,
            body(|p, v| p.extremity_size = v),
        ),
        ("face.nose", 0.5, unit, Box::new(|r, v| r.face.nose = v)),
        (
            "face.noseWidth",
            0.5,
            unit,
            Box::new(|r, v| r.face.nose_width = v),
        ),
        ("face.brow", 0.5, unit, Box::new(|r, v| r.face.brow = v)),
        ("face.mouth", 0.5, unit, Box::new(|r, v| r.face.mouth = v)),
        (
            "face.mouthWidth",
            0.5,
            unit,
            Box::new(|r, v| r.face.mouth_width = v),
        ),
        ("face.ears", 0.5, unit, Box::new(|r, v| r.face.ears = v)),
        ("eyes.size", 0.5, unit, Box::new(|r, v| r.eyes.size = v)),
        (
            "eyes.spacing",
            0.0,
            signed,
            Box::new(|r, v| r.eyes.spacing = v),
        ),
        ("eyes.depth", 0.0, signed, Box::new(|r, v| r.eyes.depth = v)),
        (
            "eyes.aperture",
            0.8,
            explore_range(0.8, (0.0, 1.0)),
            Box::new(|r, v| r.eyes.aperture = v),
        ),
    ];

    let probe = |set: &dyn Fn(&mut AvatarRecord, f32), value: f32| -> bool {
        let mut record = AvatarRecord::new("Sweep", Archetype::default());
        set(&mut record, value);
        record.sanitize();
        Avatar::build(&record).is_some()
    };

    println!("axis walls: everything else at default");
    let mut walled = 0;
    for (name, default, envelope, set) in &axes {
        for (end, label) in [(envelope.0, "low"), (envelope.1, "high")] {
            if probe(&**set, end) {
                println!("  {name:>16} {label:<4} end {end:+.3} builds");
                continue;
            }
            let (mut good, mut bad) = (*default, end);
            assert!(
                probe(&**set, good),
                "{name}: the default itself does not build"
            );
            for _ in 0..BISECTIONS {
                let middle = 0.5 * (good + bad);
                if probe(&**set, middle) {
                    good = middle;
                } else {
                    bad = middle;
                }
            }
            walled += 1;
            println!("  {name:>16} {label:<4} end {end:+.3} WALL at {good:+.3}");
        }
    }

    // Two axes at their far corners together, which is where walls hide from
    // single-axis sweeps: the seed sweep only reaches a corner when two
    // wildcards land in one body.
    if std::env::args().any(|arg| arg == "--pairs") {
        println!("\npairwise corners that fail to build:");
        let mut failed = 0;
        for first in 0..axes.len() {
            for second in (first + 1)..axes.len() {
                let (a_name, _, a_env, a_set) = &axes[first];
                let (b_name, _, b_env, b_set) = &axes[second];
                for a_end in [a_env.0, a_env.1] {
                    for b_end in [b_env.0, b_env.1] {
                        let mut record = AvatarRecord::new("Sweep", Archetype::default());
                        a_set(&mut record, a_end);
                        b_set(&mut record, b_end);
                        record.sanitize();
                        if Avatar::build(&record).is_none() {
                            failed += 1;
                            println!("  {a_name} {a_end:+.2} with {b_name} {b_end:+.2}");
                        }
                    }
                }
            }
        }
        println!("{failed} failing pair corners");
    }

    println!("\nquadruped axis walls:");
    use symbios_avatar::QuadrupedParams;
    let hoof = |set: fn(&mut QuadrupedParams, f32)| -> Setter {
        Box::new(move |record, value| {
            if let Archetype::Quadruped(params) = &mut record.archetype {
                set(params, value);
            }
        })
    };
    let beast: Vec<Axis> = vec![
        (
            "height",
            0.58,
            explore_range(0.58, (0.25, 1.8)),
            hoof(|p, v| p.height = v),
        ),
        ("bodyLength", 0.0, signed, hoof(|p, v| p.body_length = v)),
        ("build", 0.0, signed, hoof(|p, v| p.build = v)),
        (
            "muscle",
            0.0,
            explore_range(0.0, (0.0, 1.0)),
            hoof(|p, v| p.muscle = v),
        ),
        ("legLength", 0.0, signed, hoof(|p, v| p.leg_length = v)),
        ("neckLength", 0.0, signed, hoof(|p, v| p.neck_length = v)),
        ("headSize", 0.0, signed, hoof(|p, v| p.head_size = v)),
        ("tailLength", 0.0, signed, hoof(|p, v| p.tail_length = v)),
    ];
    let probe_beast = |set: &dyn Fn(&mut AvatarRecord, f32), value: f32| -> bool {
        let mut record =
            AvatarRecord::new("Sweep", Archetype::Quadruped(QuadrupedParams::default()));
        set(&mut record, value);
        record.sanitize();
        Avatar::build(&record).is_some()
    };
    for (name, default, envelope, set) in &beast {
        for (end, label) in [(envelope.0, "low"), (envelope.1, "high")] {
            if probe_beast(&**set, end) {
                println!("  {name:>16} {label:<4} end {end:+.3} builds");
                continue;
            }
            let (mut good, mut bad) = (*default, end);
            assert!(
                probe_beast(&**set, good),
                "{name}: the quadruped default does not build"
            );
            for _ in 0..BISECTIONS {
                let middle = 0.5 * (good + bad);
                if probe_beast(&**set, middle) {
                    good = middle;
                } else {
                    bad = middle;
                }
            }
            walled += 1;
            println!("  {name:>16} {label:<4} end {end:+.3} WALL at {good:+.3}");
        }
    }

    if std::env::args().any(|arg| arg == "--beast-pairs") {
        println!("\nquadruped pairwise corners that fail to build:");
        let mut failed = 0;
        for first in 0..beast.len() {
            for second in (first + 1)..beast.len() {
                let (a_name, _, a_env, a_set) = &beast[first];
                let (b_name, _, b_env, b_set) = &beast[second];
                for a_end in [a_env.0, a_env.1] {
                    for b_end in [b_env.0, b_env.1] {
                        let mut record = AvatarRecord::new(
                            "Sweep",
                            Archetype::Quadruped(QuadrupedParams::default()),
                        );
                        a_set(&mut record, a_end);
                        b_set(&mut record, b_end);
                        record.sanitize();
                        if Avatar::build(&record).is_none() {
                            failed += 1;
                            println!("  {a_name} {a_end:+.2} with {b_name} {b_end:+.2}");
                        }
                    }
                }
            }
        }
        println!("{failed} failing quadruped pair corners");
    }

    println!("\nrolled seeds that fail to build:");
    let mut broken = 0;
    for seed in 0..200i64 {
        let mut record = AvatarRecord::new("Sweep", Archetype::default());
        record.reroll(seed);
        if Avatar::build(&record).is_none() {
            broken += 1;
            let stage = match symbios_avatar::build_cage(
                &record.skeleton(),
                &symbios_avatar::CageConfig::default(),
            ) {
                Err(error) => format!("cage: {error}"),
                Ok(_) => "past the cage".to_string(),
            };
            let Archetype::Humanoid(p) = &record.archetype else {
                continue;
            };
            println!(
                "  seed {seed}: {stage}\n    h {:+.2} sh {:+.2} hip {:+.2} limb {:+.2} neck {:+.2} head {:+.2} breadth {:+.2} face {:+.2} ext {:+.2}",
                p.height,
                p.shoulder_width,
                p.hip_width,
                p.limb_length,
                p.neck_length,
                p.head_size,
                p.head_breadth,
                p.face_length,
                p.extremity_size
            );
        }
    }
    println!("{broken} of 200 seeds fail; {walled} axis ends walled");
}
