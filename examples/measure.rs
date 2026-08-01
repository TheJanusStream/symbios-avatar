//! Measures a body's proportions against the human canon.
//!
//! Written after several rounds of adjusting coefficients by eye and re-judging
//! the render, which is a slow way to be wrong. Canonical proportions are
//! published numbers; comparing against them turns "the head looks small" into
//! "the head joint sits at 0.98 of height, so its dome has collapsed" — which is
//! a different problem with a different fix.
//!
//! Figures are fractions of the *rendered* height rather than the nominal one,
//! since subdivision shrinks a body slightly and it is the rendered body that
//! gets looked at.
//!
//! ```text
//! cargo run --example measure
//! cargo run --example measure -- --seed 7
//! ```

use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, Limb, Rig, Zone, build_cage, catmull_clark,
};

/// Where each landmark sits in an eight-head figure, as a fraction of height.
const CANON: [(&str, Zone, f32); 6] = [
    ("head", Zone::Head, 0.935),
    ("neck", Zone::Neck, 0.865),
    ("chest", Zone::Chest, 0.720),
    ("waist", Zone::Abdomen, 0.640),
    ("pelvis", Zone::Pelvis, 0.545),
    ("knee", Zone::UpperLimb(Limb::HindLeft), 0.285),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed = args
        .iter()
        .position(|arg| arg == "--seed")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse::<i64>().ok());

    let mut record = AvatarRecord::new("Measured", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }

    let skeleton = record.skeleton();
    let Ok(cage) = build_cage(&skeleton, &CageConfig::default()) else {
        eprintln!("the body would not mesh");
        std::process::exit(1);
    };
    let mesh = catmull_clark(&cage, 2);
    let Ok(rig) = Rig::from_skeleton(&skeleton) else {
        eprintln!("the body would not rig");
        std::process::exit(1);
    };

    let (lo, hi) = mesh.bounds();
    let height = (hi.y - lo.y).max(1e-3);
    println!("rendered height {height:.3} m\n");
    println!(
        "{:<10} {:>8} {:>8} {:>8} {:>9}",
        "landmark", "y/H", "canon", "off", "radius/H"
    );

    for (name, zone, canon) in CANON {
        // Core zones hold several joints — a head has a crown above it, a chest
        // both clavicles — and it is the first that the canon figure names. A
        // limb's canon figure is its far joint, the knee.
        let joints = rig.in_zone(zone);
        let Some(&joint) = (if zone.is_core() {
            joints.first()
        } else {
            joints.last()
        }) else {
            continue;
        };
        let joint = rig.joints[joint];
        let at = (joint.position.y - lo.y) / height;
        println!(
            "{name:<10} {at:>8.3} {canon:>8.3} {:>+8.3} {:>9.3}",
            at - canon,
            joint.radius / height
        );
    }

    // Spans say as much about a silhouette as heights do.
    let span = |zone: Zone| -> f32 {
        rig.in_zone(zone)
            .iter()
            .map(|&joint| rig.joints[joint].position.x.abs())
            .fold(0.0, f32::max)
            * 2.0
            / height
    };
    println!("\n{:<10} {:>8} {:>8}", "span", "of H", "canon");
    println!(
        "{:<10} {:>8.3} {:>8.3}",
        "shoulders",
        span(Zone::Chest),
        0.245
    );
    println!(
        "{:<10} {:>8.3} {:>8.3}",
        "hips",
        span(Zone::UpperLimb(Limb::HindLeft)),
        0.190
    );
    println!(
        "{:<10} {:>8.3} {:>8.3}",
        "arm span",
        span(Zone::Extremity(Limb::ForeLeft)),
        1.000
    );
}
