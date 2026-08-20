//! Measures a foot on a body whose foot is part of its leg.
//!
//! `examples/bodyaudit` measures proportions and skinning over the whole body;
//! this measures the one part of it that nothing else can reach. A foot meshed
//! into the leg is not a connected component and is not separable by a box, so
//! it is selected through the binding instead — see [`Patch`] for why, and for
//! the two ways of doing it that fail on this body.
//!
//! Every reference column comes from the Quaternius male and female mannequins
//! (CC0, via mesh2motion), measured off the GLB with the *same* selector and the
//! same ray-cast sole, so the two sides of each comparison are made the same
//! way.
//!
//! Four questions, in the order they have to be answered:
//!
//! 1. **Is the selection a foot?** Vertices, faces, pieces, and where it sits
//!    against the joints of the leg. A number from a selector nobody has checked
//!    is worth nothing, and this crate has produced several.
//! 2. **Is it the right shape?** Length against stature, width against length,
//!    and the profile of both down its own length.
//! 3. **Has it a heel?** Not the rearmost point, which a fat ankle supplies on
//!    its own, but how much of the sole's *contact patch* lies behind the ankle.
//! 4. **What did the mesher deliver?** The node radius the plan asked for beside
//!    the half-width that came out, so the plan can be back-solved instead of
//!    having another assumed factor stacked on it.
//!
//! ```text
//! cargo run --example footaudit
//! cargo run --example footaudit -- --seed 7
//! cargo run --example footaudit -- --grid 96
//! ```

use glam::Vec3;
use symbios_avatar::face::HeadTraits;
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Limb, Patch, Rig, SkinConfig, Zone,
    build_body, rig::skin,
};

/// How close to the lowest point of a sole counts as standing on the ground.
///
/// Ten millimetres, because that is the band the reference soles fall inside:
/// measured by the same ray cast, the male's contact cells run 0.2 to 10.0 mm
/// above its lowest point and the female's 0.0 to 9.9. A tighter band would
/// measure our foot against a stricter standard than the bodies it is being
/// compared with.
const CONTACT: f32 = 0.010;

/// The reference feet, measured off the GLBs.
struct Reference {
    /// What the body is called.
    name: &'static str,
    /// Foot length as a fraction of stature.
    length: f32,
    /// Foot width as a fraction of its own length.
    width: f32,
    /// How far behind the ankle the foot reaches, as a fraction of its length.
    behind: f32,
    /// Share of the sole's contact patch that lies behind the ankle.
    heel_share: f32,
    /// Median height of the contact patch above its lowest point, in mm.
    contact_median: f32,
}

/// Both reference mannequins. They share one skeleton and differ in flesh.
const REFERENCES: [Reference; 2] = [
    Reference {
        name: "male",
        length: 0.164,
        width: 0.37,
        behind: 0.156,
        heel_share: 0.112,
        contact_median: 3.1,
    },
    Reference {
        name: "female",
        length: 0.157,
        width: 0.38,
        behind: 0.189,
        heel_share: 0.137,
        contact_median: 2.6,
    },
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let seed = number("--seed").map(|value| value as i64);
    let grid = number("--grid").map_or(64, |value| value as usize);

    let mut record = AvatarRecord::new("Footed", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let skeleton = record.skeleton();
    // **Through `build_body`, not `build_cage` + `catmull_clark`** (#306).
    // This used to subdivide the cage itself and measure that, which is the
    // body BEFORE every carve — and the foot's wedge and medial bend are a
    // carve, so the instrument read the old slab unchanged while the render
    // showed the new foot. An instrument that reads a surface the body does
    // not ship is the staleness #304 exists to catch.
    let mesh = match build_body(
        &skeleton,
        &CageConfig::default(),
        BODY_SUBDIVISIONS,
        &HeadTraits::of(&record.composites),
    ) {
        Ok(mesh) => mesh,
        Err(error) => {
            // Printed rather than swallowed: every one of these errors names the
            // two nodes and the two distances involved, which is the whole of
            // what is needed to fix the skeleton that raised it.
            eprintln!("the body would not mesh: {error}");
            std::process::exit(1);
        }
    };
    let Ok(rig) = Rig::from_skeleton(&skeleton) else {
        eprintln!("the body would not rig");
        std::process::exit(1);
    };
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());

    let (low, high) = mesh.bounds();
    let stature = (high.y - low.y).max(1e-3);
    println!(
        "rendered height {stature:.3} m, standing on y = {:.4}",
        low.y
    );
    println!("reference: Quaternius male 1.830 m, female 1.806 m (CC0, via mesh2motion)");

    for limb in [Limb::HindLeft, Limb::HindRight] {
        let joints = rig.extremity_joints(limb);
        if joints.is_empty() {
            continue;
        }
        let patch = Patch::held_by(&mesh, &weights, &joints);
        selection(&rig, &mesh, &patch, &joints, limb);
        if limb == Limb::HindLeft {
            shape(&rig, &mesh, &patch, &joints, stature);
            sole(&rig, &mesh, &patch, &joints, grid);
            delivered(&rig, &mesh, &patch, &joints);
        }
    }
}

/// The ankle: the joint the foot's own chain hangs from.
///
/// Named by what it is rather than by where it sits in the chain. Reading a
/// landmark off its position in a bone list is how an extremity node ends up
/// beside the reference's ankle joint, reporting an ankle at 2.6% of stature
/// that was never anywhere but 6.86%.
fn ankle(joints: &[usize]) -> usize {
    joints[0]
}

/// Whether the selection is a foot at all, said in numbers.
fn selection(
    rig: &Rig,
    mesh: &symbios_avatar::PolyMesh,
    patch: &Patch,
    joints: &[usize],
    limb: Limb,
) {
    let (lo, hi) = patch.bounds(mesh);
    println!("\n{:=<78}", "");
    println!("{limb:?}: the surface held by");
    for &joint in joints {
        let at = rig.joints[joint].position;
        println!(
            "    joint {joint:>3}  {:<22} at ({:+.3} {:+.3} {:+.3})  r {:.4}",
            format!("{:?}", rig.joints[joint].zone),
            at.x,
            at.y,
            at.z,
            rig.joints[joint].radius,
        );
    }
    println!(
        "  {} vertices, {} faces, {} piece(s)",
        patch.vertex_count(),
        patch.faces().len(),
        patch.components(mesh),
    );
    println!(
        "  spans x {:+.3}..{:+.3}   y {:+.3}..{:+.3}   z {:+.3}..{:+.3}",
        lo.x, hi.x, lo.y, hi.y, lo.z, hi.z
    );
    // The check a height crop failed. Whatever the selector says, a standing
    // body's lowest vertex is on a sole, and a foot patch that does not reach
    // the floor has selected something else.
    let floor = mesh
        .positions
        .iter()
        .map(|at| at.y)
        .fold(f32::INFINITY, f32::min);
    println!(
        "  reaches {:.1} mm above the body's lowest vertex; \
         top sits {:.1} mm below the knee",
        (lo.y - floor) * 1000.0,
        (rig.joints[rig.limb_chain(limb).map_or(0, |chain| chain[1])]
            .position
            .y
            - hi.y)
            * 1000.0,
    );
}

/// Length, width and the profile of both, against the reference.
fn shape(
    rig: &Rig,
    mesh: &symbios_avatar::PolyMesh,
    patch: &Patch,
    joints: &[usize],
    stature: f32,
) {
    let (lo, hi) = patch.bounds(mesh);
    let length = hi.z - lo.z;
    let width = hi.x - lo.x;
    let at = rig.joints[ankle(joints)].position;

    println!(
        "\n  {:<22} {:>9} {:>9} {:>9}",
        "", "ours", REFERENCES[0].name, REFERENCES[1].name
    );
    let row = |name: &str, ours: f32, pick: fn(&Reference) -> f32| {
        println!(
            "  {name:<22} {ours:>9.4} {:>9.4} {:>9.4}",
            pick(&REFERENCES[0]),
            pick(&REFERENCES[1])
        );
    };
    row("length / stature", length / stature, |r| r.length);
    row("width / length", width / length, |r| r.width);
    row("behind ankle / length", (at.z - lo.z) / length, |r| {
        r.behind
    });
    println!(
        "  foot is {:.2} cm long, {:.2} cm wide, {:.2} cm thick; \
         ankle {:.2} cm up ({:.2}% of stature)",
        length * 100.0,
        width * 100.0,
        (hi.y - lo.y) * 100.0,
        at.y * 100.0,
        at.y / stature * 100.0,
    );

    // The profile the reference's own table is printed as, so the two can be
    // read side by side: ten bands from heel to toe, each reporting how high the
    // sole and the top of the foot sit above its lowest point.
    println!(
        "\n  {:>7} {:>9} {:>9} {:>9} {:>9}",
        "along", "sole mm", "top mm", "thick mm", "width mm"
    );
    for band in 0..10 {
        let (z0, z1) = (
            lo.z + length * band as f32 / 10.0,
            lo.z + length * (band as f32 + 1.0) / 10.0,
        );
        let inside: Vec<Vec3> = patch
            .vertices()
            .map(|vertex| mesh.positions[vertex])
            .filter(|at| at.z >= z0 && at.z <= z1)
            .collect();
        if inside.len() < 3 {
            continue;
        }
        let sole = inside.iter().map(|at| at.y).fold(f32::INFINITY, f32::min);
        let top = inside
            .iter()
            .map(|at| at.y)
            .fold(f32::NEG_INFINITY, f32::max);
        let wide = inside
            .iter()
            .map(|at| at.x)
            .fold(f32::NEG_INFINITY, f32::max)
            - inside.iter().map(|at| at.x).fold(f32::INFINITY, f32::min);
        println!(
            "  {:>6}% {:>9.1} {:>9.1} {:>9.1} {:>9.1}   {}",
            band * 10,
            (sole - lo.y) * 1000.0,
            (top - lo.y) * 1000.0,
            (top - sole) * 1000.0,
            wide * 1000.0,
            "#".repeat((wide / width * 20.0).round() as usize),
        );
    }
    println!("  (0% is the heel end, 100% the toe; heights above the foot's lowest point)");
}

/// The sole: is it flat, and how much of it is behind the ankle.
fn sole(rig: &Rig, mesh: &symbios_avatar::PolyMesh, patch: &Patch, joints: &[usize], grid: usize) {
    let print = patch.footprint(mesh, grid);
    let Some(floor) = print.ground() else {
        println!("\n  the foot has no underside to measure");
        return;
    };
    let at = rig.joints[ankle(joints)].position;
    let cell = print.cell_area();

    let shadow: Vec<(f32, f32)> = print
        .hits()
        .map(|(ground, height)| (ground.y, (height - floor) * 1000.0))
        .collect();
    let contact: Vec<(f32, f32)> = print
        .contact(CONTACT)
        .map(|(ground, height)| (ground.y, (height - floor) * 1000.0))
        .collect();

    println!("\n  sole, by {grid}x{grid} vertical ray cast");
    println!(
        "  shadow  {:>5} cells {:>7.1} cm2      contact {:>5} cells {:>7.1} cm2",
        shadow.len(),
        shadow.len() as f32 * cell * 1e4,
        contact.len(),
        contact.len() as f32 * cell * 1e4,
    );

    // **The heel test.** The rearmost point of a foot is a weak thing to ask —
    // a thick ankle has one, at ankle height, and nothing rests on it. What says
    // heel is contact area on the ground behind the ankle.
    let behind = contact.iter().filter(|(z, _)| *z < at.z).count();
    let share = behind as f32 / contact.len().max(1) as f32;
    println!(
        "  contact behind the ankle {:>5} cells {:>7.2} cm2 = {:>5.1}% \
         ({} {:.1}%, {} {:.1}%)",
        behind,
        behind as f32 * cell * 1e4,
        share * 100.0,
        REFERENCES[0].name,
        REFERENCES[0].heel_share * 100.0,
        REFERENCES[1].name,
        REFERENCES[1].heel_share * 100.0,
    );

    let mut heights: Vec<f32> = contact.iter().map(|(_, height)| *height).collect();
    heights.sort_by(f32::total_cmp);
    if let (Some(&least), Some(&most)) = (heights.first(), heights.last()) {
        println!(
            "  contact heights  min {least:.1} mm  median {:.1}  max {most:.1}   \
             (median {} {:.1}, {} {:.1})",
            heights[heights.len() / 2],
            REFERENCES[0].name,
            REFERENCES[0].contact_median,
            REFERENCES[1].name,
            REFERENCES[1].contact_median,
        );
    }

    // Where the sole actually is, as a picture. Five rows across the foot by
    // nine bands along it, in millimetres above the lowest point: a keel reads
    // as a valley down the middle, a rocker as a ridge along the length.
    let (lo, hi) = patch.bounds(mesh);
    println!(
        "\n  {:<9}{}   <- heel to toe",
        "",
        (0..9)
            .map(|band| format!("{:>7}%", band * 100 / 8))
            .collect::<String>()
    );
    for row in 0..5 {
        let x = lo.x + (hi.x - lo.x) * (row as f32 + 0.5) / 5.0;
        let cells: String = (0..9)
            .map(|band| {
                let z = lo.z + (hi.z - lo.z) * (band as f32 + 0.5) / 9.0;
                match lowest_near(&print, x, z) {
                    Some(height) => format!("{:>8.1}", (height - floor) * 1000.0),
                    None => "       .".to_string(),
                }
            })
            .collect();
        let edge = match row {
            0 => "medial",
            4 => "lateral",
            _ => "",
        };
        println!("  {edge:<9}{cells}");
    }
}

/// The sampled height nearest a point of the ground plane.
///
/// The picture above is drawn on a coarser grid than the rays were cast on, so
/// it reads the nearest cell rather than casting again.
fn lowest_near(print: &symbios_avatar::Footprint, x: f32, z: f32) -> Option<f32> {
    let row = ((x - print.origin.x) / print.step.x - 0.5).round();
    let column = ((z - print.origin.y) / print.step.y - 0.5).round();
    if row < 0.0 || column < 0.0 {
        return None;
    }
    print.height(row as usize, column as usize)
}

/// What the mesher delivered against what the plan asked for.
///
/// The trap this exists to close: `humanoid.rs` derives its foot radius as the
/// wanted half-width divided by an assumed delivered fraction times an assumed
/// roll factor, which is two guesses multiplied together and no measurement
/// anywhere. A node radius is a request. This is the reply.
fn delivered(rig: &Rig, mesh: &symbios_avatar::PolyMesh, patch: &Patch, joints: &[usize]) {
    println!(
        "\n  {:<22} {:>9} {:>9} {:>7}",
        "node", "asked r", "half-width", "kept"
    );
    for &joint in joints {
        if !matches!(rig.joints[joint].zone, Zone::Extremity(_)) {
            continue;
        }
        let at = rig.joints[joint].position;
        // The surface abreast of the node: everything within a quarter of the
        // node's own radius of its plane, which is narrow enough to be about
        // this node and wide enough to hold a ring of the cage.
        let band = rig.joints[joint].radius * 0.25;
        let half = patch
            .vertices()
            .map(|vertex| mesh.positions[vertex])
            .filter(|point| (point.z - at.z).abs() <= band)
            .map(|point| (point.x - at.x).abs())
            .fold(f32::NEG_INFINITY, f32::max);
        if !half.is_finite() {
            continue;
        }
        println!(
            "  {:<22} {:>9.4} {:>9.4} {:>6.0}%",
            format!("{:?} z{:+.3}", rig.joints[joint].zone, at.z),
            rig.joints[joint].radius,
            half,
            half / rig.joints[joint].radius * 100.0,
        );
    }
    println!(
        "  (kept is what a ring of {} points delivers as surface, roll included — \
         the number humanoid.rs must divide by, measured rather than assumed)",
        4,
    );
}
