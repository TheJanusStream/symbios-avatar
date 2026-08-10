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
//! cargo run --example measure -- --face     # how fine the face's surface is
//! ```

use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Limb, Rig, Zone, build_cage,
    catmull_clark,
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
    let mesh = catmull_clark(&cage, BODY_SUBDIVISIONS);
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
    // Measured along the arm, not across the body. Arms rest angled down, so
    // twice their horizontal reach is not the span anybody means by arm span.
    let mut reach = rig
        .in_zone(Zone::Chest)
        .iter()
        .map(|&joint| rig.joints[joint].position.x.abs())
        .fold(0.0f32, f32::max);
    let mut joint = *rig
        .in_zone(Zone::Extremity(Limb::ForeRight))
        .first()
        .expect("a humanoid has hands");
    while let Some(parent) = rig.joints[joint].parent {
        reach += rig.joints[joint]
            .position
            .distance(rig.joints[parent].position);
        if rig.joints[parent].zone == Zone::Chest {
            break;
        }
        joint = parent;
    }
    println!(
        "{:<10} {:>8.3} {:>8.3}",
        "arm span",
        reach * 2.0 / height,
        1.000
    );

    if args.iter().any(|arg| arg == "--face") {
        cells(&skeleton);
    }
}

/// How fine the face's surface is, in each band a feature occupies.
///
/// The argument of #59 in one table. A brow ridge is 10 mm tall and a nose is
/// one eye-width across; neither can be shaped into a surface whose quads are
/// 24 mm on a side, and no amount of tuning the feature generators changes that.
/// The figure to watch is the median edge, because a mean is dragged down by the
/// slivers around a pole.
///
/// Prints every refinement level rather than the one that ships, so the cost of
/// the next one is on the same page as what it buys.
fn cells(skeleton: &symbios_avatar::Skeleton) {
    use symbios_avatar::{Zone, refine_face, shape_skull};

    /// Each band as a fraction of the way up the head, bottom to crown.
    const BANDS: [(&str, f32, f32); 4] = [
        ("brow", 0.55, 0.75),
        ("eye", 0.42, 0.58),
        ("nose", 0.25, 0.45),
        ("mouth", 0.12, 0.28),
    ];

    let Ok(cage) = build_cage(skeleton, &CageConfig::default()) else {
        return;
    };
    let Ok(rig) = Rig::from_skeleton(skeleton) else {
        return;
    };
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let centre = rig.joints[head].position;

    println!(
        "\n{:<8} {:>8}  median edge in each feature band",
        "passes", "tris"
    );
    for levels in 0..=3 {
        let mut mesh = refine_face(&catmull_clark(&cage, BODY_SUBDIVISIONS), &rig, levels);
        shape_skull(&mut mesh, &rig, &Default::default());

        let (lo, hi) = mesh
            .positions
            .iter()
            .filter(|&&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
            .fold((f32::MAX, f32::MIN), |(lo, hi), point| {
                (lo.min(point.y), hi.max(point.y))
            });

        let mut found: Vec<Vec<f32>> = vec![Vec::new(); BANDS.len()];
        for face in 0..mesh.face_count() {
            let at = mesh.face_centroid(face) - centre;
            // The front of the head only. A cheek behind the ear carries no
            // feature and averaging it in would flatter the figure.
            if at.z <= 0.0 {
                continue;
            }
            let up = (at.y + centre.y - lo) / (hi - lo);
            let corners = &mesh.faces[face];
            let mean = (0..corners.len())
                .map(|corner| {
                    mesh.positions[corners[corner] as usize]
                        .distance(mesh.positions[corners[(corner + 1) % corners.len()] as usize])
                })
                .sum::<f32>()
                / corners.len() as f32;
            for (band, (_, low, high)) in BANDS.iter().enumerate() {
                if up >= *low && up <= *high {
                    found[band].push(mean * 1000.0);
                }
            }
        }

        print!(
            "{levels:<8} {:>8}",
            mesh.faces.iter().map(|face| face.len() - 2).sum::<usize>()
        );
        for (band, (name, ..)) in BANDS.iter().enumerate() {
            found[band].sort_by(f32::total_cmp);
            let middle = found[band]
                .get(found[band].len() / 2)
                .copied()
                .unwrap_or(0.0);
            print!("   {name} {middle:>5.1} mm");
        }
        println!();
    }
}
