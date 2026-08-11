//! Every measurable fact about a built head, in one dump (#73).
//!
//! Written to be handed to people (and agents) who cannot run the code: the
//! landmarks the crate believes in, the surface it actually built, the two
//! compared, and enough silhouette to see the shape rather than infer it.
//!
//! **Every surface query here is a bisection against [`PolyMesh::contains`],
//! never a bin over vertices.** Binning the furthest-forward vertex in each
//! 2 mm band reported six millimetres of ripple that is not in the mesh, off
//! the midline where the surface curves fast across a band (#71). The
//! containment test is the same primitive `tests/parts.rs` judges with.
//!
//! ```text
//! cargo run --release --example headaudit           # the default body
//! cargo run --release --example headaudit -- 99     # one seed
//! cargo run --release --example headaudit -- --sweep        # the proportions, by seed
//! cargo run --release --example headaudit -- --sweep 1 2 3  # over seeds of your own
//! cargo run --release --example headaudit -- --axis breadth  # one axis, end to end
//! ```
//!
//! `--axis` takes `breadth`, `length`, `nose`, `mouth` or `age` and walks it
//! from one end to the other on every sweep seed, which is what #61 needs before
//! it can choose where a default sits: an axis is not an axis until somebody has
//! looked at both of its ends on more than one body.
//!
//! **`age` is a composite and the columns read it unevenly.** Its lower-face
//! term is jowling, which lands squarely in `bigon/bizyg`; its lip term moves
//! the lip band's HEIGHT and this table's `mouth` column is the carve's width,
//! so that one does not appear here at all and wants the viewer (#167).

use symbios_avatar::face::{Canon, Eyes, Skull};
use symbios_avatar::{Archetype, Avatar, AvatarConfig, AvatarRecord, PolyMesh, Vec3, Zone, face};

/// Millimetres between samples, everywhere.
const STEP: f32 = 0.002;

/// The seeds `--sweep` reports when it is not given any.
///
/// The four #79 measured its baseline on, plus the two #107 found chinless
/// under the eight-point cage and two more for spread. A breadth default chosen
/// on one body is a breadth default chosen on one body.
const SWEEP_SEEDS: [i64; 8] = [7, 23, 29, 42, 1, 3, 6, 12];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let named = |flag: &str| -> Option<String> {
        let at = args.iter().position(|arg| arg == flag)?;
        args.get(at + 1).cloned()
    };
    if let Some(axis) = named("--axis") {
        walk(&axis);
        return;
    }
    if args.iter().any(|arg| arg == "--sweep") {
        let seeds: Vec<i64> = args.iter().filter_map(|arg| arg.parse().ok()).collect();
        sweep(
            if seeds.is_empty() {
                &SWEEP_SEEDS
            } else {
                &seeds
            },
            Axes::default(),
        );
        return;
    }
    let seed = args.iter().find_map(|arg| arg.parse::<i64>().ok());
    let mut record = AvatarRecord::new("Audited", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let avatar = Avatar::build(&record).expect("a biped builds");
    // `parts.body`, NOT the merged `MeshKind::Skin`. The merged skin has the
    // ears appended, and an ear sits within a couple of millimetres of the
    // head's own centre depth — so a half-width bisected along +X from the
    // midline runs straight through one and reports the head 18 mm wider than
    // it is, between about -23 and +9 mm. The first version of this tool did
    // exactly that and called the column "half-width", which is the sixth time
    // in this project a name has meant something other than what it measures.
    let body = &avatar.parts.body;
    let rig = &avatar.rig;
    let head = *rig.in_zone(Zone::Head).first().expect("a head");
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    // The eyes as the avatar carries them, not a fresh pair: they are seated by
    // bisecting the carved surface now (#76), so rebuilding them from a plan is
    // not a thing that can be done here any more — which is the point.
    let eyes: &Eyes = avatar.parts.eyes.as_ref().expect("a humanoid has eyes");
    let skull = Skull::measure(body, rig).expect("a skull");
    // The canon off the SHAPED head, which is the one the build read it from —
    // `Avatar::build_with` measures before it carves. Read off the carved body
    // instead it comes back with a frame 3.3 mm longer, because the carve's own
    // chin field moves the menton the frame ends at. A report that prints
    // landmarks the build did not use is the same defect as the hard-coded
    // fractions this file carried until #78.
    let skeleton = record.skeleton();
    let plain = symbios_avatar::build_body(
        &skeleton,
        &symbios_avatar::CageConfig::default(),
        symbios_avatar::BODY_SUBDIVISIONS,
        &Default::default(),
    )
    .expect("a body builds");
    let shaped = Skull::measure(&plain, rig).expect("a skull");
    let canon = Canon::measure(rig, &shaped, &record.eyes);

    println!(
        "# HEAD AUDIT — seed {}\n",
        seed.map_or("default".into(), |s| s.to_string())
    );
    println!(
        "Head joint radius {:.1} mm. All heights are millimetres above the head \
         joint; all depths are millimetres forward of it; widths are half-widths \
         from the midline.\n",
        radius * 1000.0
    );

    // ---- The landmarks the crate believes in ------------------------------
    //
    // The globe and the ruler are two numbers now, and were one until #77. Both
    // are printed, because "the eye is twice life size" and "every feature is
    // measured in the eye" were the same defect wearing two faces.
    let unit = canon.unit;
    let eyeball = eyes.left.radius;
    let level = canon.level;
    let chin = skull.chin();
    let (span_lo, span_hi) = skull.throat_and_crown();
    println!("## What the crate believes\n");
    println!("| landmark | mm | source |");
    println!("|---|---|---|");
    println!(
        "| eye globe radius | {:.1} | eyes.left.radius, the anatomical eyeball |",
        eyeball * 1000.0
    );
    println!(
        "| proportion unit | {:.1} | Canon::unit, one eye-width by fifths of the measured \
         half-width |",
        unit * 1000.0
    );
    println!(
        "| frame ruler | {:.1} | Canon::frame, the eye line to the chin |",
        canon.frame * 1000.0
    );
    println!(
        "| eye pivot, height | {:+.1} | eyes.left.pivot.y |",
        level * 1000.0
    );
    println!(
        "| eye pivot, depth | {:+.1} | eyes.left.pivot.z |",
        eyes.left.pivot.z * 1000.0
    );
    println!(
        "| eye pivot, from midline | {:.1} | eyes.left.pivot.x |",
        eyes.left.pivot.x.abs() * 1000.0
    );
    println!(
        "| Skull::span, high | {:+.1} | measured crown |",
        span_hi * 1000.0
    );
    println!(
        "| Skull::span, low | {:+.1} | measured floor, which is THROAT |",
        span_lo * 1000.0
    );
    println!(
        "| Skull::chin | {:+.1} | lowest crest of the measured midline profile |",
        chin * 1000.0
    );
    let frame = level - chin;
    println!(
        "| eye-to-chin frame | {:.1} | the span every feature fraction is OF |",
        frame * 1000.0
    );
    // Asked of the canon rather than recomputed from a copy of its fractions,
    // which is what this did until #78 moved them and it went on printing the
    // old ones.
    for (name, at) in [
        ("nose base", canon.nose_base()),
        ("mouth line", canon.mouth_line()),
        ("ear centre", canon.ear_centre()),
    ] {
        println!(
            "| {name} | {:+.1} | Canon, as a fraction of the frame |",
            at * 1000.0
        );
    }
    println!();

    // ---- The surface actually built ---------------------------------------
    let inside = |p: Vec3| body.contains(p);
    let bisect = |from: Vec3, along: Vec3, far: f32| -> Option<f32> {
        if !inside(from) {
            return None;
        }
        let (mut near, mut out) = (0.0f32, far);
        for _ in 0..40 {
            let mid = 0.5 * (near + out);
            if inside(from + along * mid) {
                near = mid;
            } else {
                out = mid;
            }
        }
        Some(near)
    };
    let at = |y: f32| Vec3::new(centre.x, centre.y + y, centre.z);

    // The crown and the floor of the whole skin, found by probing rather than
    // by zone, because what the eye sees is the surface and not the ownership.
    let (mut crown, mut floor) = (0.0f32, 0.0f32);
    let mut y = 0.0;
    while inside(at(y)) {
        crown = y;
        y += STEP;
    }
    y = 0.0;
    while inside(at(y)) {
        floor = y;
        y -= STEP;
    }

    println!("## Proportions\n");
    let shape = Proportions::measure(body, centre, radius, &skull, &canon, eyes);
    println!("| what | this head | life |");
    println!("|---|---|---|");
    println!(
        "| widest half-width | {:.1} mm at {:+.1} mm ({:+.2} R) | eurion, 25 to 45 mm above the pupil line |",
        shape.widest * 1000.0,
        shape.widest_at * 1000.0,
        shape.widest_at / radius
    );
    println!(
        "| height : depth : width | {:.2} : {:.2} : 1.00 | 1.48 : 1.28 : 1.00 |",
        shape.height / shape.width,
        shape.depth / shape.width
    );
    println!(
        "| vault depth : width | {:.2} : 1.00 | 1.28 : 1.00, glabella to opisthocranion |",
        shape.vault / shape.width
    );
    println!(
        "| crown to chin | {:.1} mm, {:.3} head radii | an eighth of stature |",
        shape.height * 1000.0,
        shape.height / radius
    );
    println!(
        "| too wide for its own height | {:+.1}% | — |",
        shape.too_wide() * 100.0
    );
    println!(
        "| bizygomatic | {:.1} mm | 137 |",
        shape.bizygomatic * 1000.0
    );
    println!(
        "| bigonial / bizygomatic | {:.3} | 0.73 to 0.76 |",
        shape.bigonial / shape.bizygomatic
    );
    println!(
        "| cranium : face | {:.2} | about 1.00 |",
        shape.cranium_to_face
    );
    println!(
        "| bare eye centre azimuth | {:+.0}° | 0, by construction |\n",
        shape.eye_azimuth
    );

    println!("## The surface built\n");
    println!(
        "Crown at {:+.1} mm, and the midline is still inside the body at {:+.1} mm — and it keeps going: head and neck are ONE continuous surface with no boundary anywhere.\n",
        crown * 1000.0,
        floor * 1000.0
    );
    println!("| mm up | forward | half-width (ears excluded) | width at half-depth | back |");
    println!("|---|---|---|---|---|");
    let mut rows: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut y = crown;
    while y > -0.75 * radius {
        let point = at(y);
        let ahead = bisect(point, Vec3::Z, 0.30);
        let behind = bisect(point, -Vec3::Z, 0.30);
        let wide = bisect(point, Vec3::X, 0.30);
        if let (Some(ahead), Some(behind), Some(wide)) = (ahead, behind, wide) {
            // And the width taken half-way forward, where a cheekbone is, not
            // through the head's centre where the ear canal is.
            let forward = Vec3::new(centre.x, centre.y + y, centre.z + ahead * 0.5);
            let cheek = bisect(forward, Vec3::X, 0.30).unwrap_or(f32::NAN);
            println!(
                "| {:+7.1} | {:7.1} | {:7.1} | {:7.1} | {:7.1} |",
                y * 1000.0,
                ahead * 1000.0,
                wide * 1000.0,
                cheek * 1000.0,
                behind * 1000.0
            );
            rows.push((y, ahead, wide, cheek));
        }
        y -= 0.004;
    }
    println!();

    // ---- Silhouettes ------------------------------------------------------
    println!("## Profile, midline — one column per 2 mm forward\n```text");
    for &(y, ahead, _, _) in &rows {
        println!(
            "{:+7.1} {}#",
            y * 1000.0,
            " ".repeat(((ahead / 0.002) as usize).min(80))
        );
    }
    println!("```\n");
    println!("## Front, half — one column per 2 mm across, at half depth\n```text");
    for &(y, _, _, cheek) in &rows {
        if cheek.is_nan() {
            continue;
        }
        println!(
            "{:+7.1} {}#",
            y * 1000.0,
            " ".repeat(((cheek / 0.002) as usize).min(80))
        );
    }
    println!("```\n");

    // ---- The eyes ---------------------------------------------------------
    println!("## The eyes\n");

    // The APERTURE, which is a different question from where the globe is and
    // the one the eye's shape turns on: not how much of the eye shows but WHERE,
    // and which of the two occluders decided it. Asked of [`Eye::aperture`]
    // rather than worked out here, so this report and
    // `the_eye_opens_on_the_gaze_rather_than_where_the_skin_falls_away` cannot
    // drift apart — which is the defect this whole file was rewritten for (#74).
    println!("| what covers the eye | bare | centred az/el | spans az | spans el |");
    println!("|---|---|---|---|---|");
    for (name, skin, lids) in [
        ("skin and lids", true, true),
        ("skin alone", true, false),
        ("lids alone", false, true),
    ] {
        let at = eyes.right.aperture(skin.then_some((body, centre)), lids);
        let deg = |radians: f32| radians.to_degrees();
        println!(
            "| {name} | {:.1}% | {:+.0} / {:+.0} | {:+.0}..{:+.0} | {:+.0}..{:+.0} |",
            at.share * 100.0,
            deg(at.centre.0),
            deg(at.centre.1),
            deg(at.span.0),
            deg(at.span.1),
            deg(at.span.2),
            deg(at.span.3),
        );
    }
    println!(
        "\nAzimuth is measured right of the GAZE, so positive is lateral for the \
         right eye. An aperture centred anywhere but zero is one the skin cut \
         rather than one the lids opened.\n"
    );

    // How far the skin sits in front of the globe, round the eye's own axis.
    // Positive means the skin is covering the globe there; negative means the
    // globe has already broken through. This is the profile an orbit has to
    // answer (#88), and reading it round the eye rather than across the face is
    // the point: the two are only the same on the midline.
    println!(
        "Round the eye's own axis: 0° is lateral, 90° up, 180° medial, 270° down. \
         How deep the globe's own surface lies UNDER the skin at that angle off \
         the gaze, measured along the head's outward radial — which is what a \
         carve displacing along vertex normals would have to remove to uncover \
         it. Negative means the globe is already bare there.\n\
         \n\
         Measured this way rather than along the ray from the eye's pivot. That \
         ray runs nearly tangential to the face on the nasal side, so it travels \
         28 mm inside the head at 60° medial and reports a clearance that is \
         mostly the length of the ray rather than the depth of the skin.\n"
    );
    print!("| off the gaze |");
    for step in 0..12 {
        print!(" {:.0}° |", step as f32 * 30.0);
    }
    println!();
    println!("|---|{}", "---|".repeat(12));
    for polar in [40.0f32, 50.0, 60.0, 70.0] {
        print!("| {polar:.0}° |");
        for step in 0..12 {
            let roll = (step as f32 * 30.0).to_radians();
            let (sin, cos) = polar.to_radians().sin_cos();
            let toward = Vec3::new(sin * roll.cos(), sin * roll.sin(), cos);
            let on_globe = centre + eyes.right.pivot + toward * eyes.right.radius;
            // Outward along the head's own radial from this point, which stands
            // in for the skin's normal on a surface as convex as a skull.
            let out = (on_globe - centre).normalize_or(Vec3::Z);
            let (mut near, mut far) = (0.0f32, eyes.right.radius * 4.0);
            if !inside(on_globe) {
                print!(" {:+.1} |", 0.0);
                continue;
            }
            for _ in 0..30 {
                let mid = 0.5 * (near + far);
                if inside(on_globe + out * mid) {
                    near = mid;
                } else {
                    far = mid;
                }
            }
            print!(" {:+.1} |", near * 1000.0);
        }
        println!();
    }
    println!();
    // `eyes.left.globe`, which is HEAD-LOCAL. Not `Avatar::eyes_at`, whose
    // meshes have already been through `to_body` — the first version of this
    // probe added the head joint to those and measured a globe a metre and a
    // half above the skull, which of course reported every vertex outside the
    // skin. The figure looked spectacular and meant nothing.
    let globe = &eyes.left.globe;
    let outside = globe
        .positions
        .iter()
        .filter(|p| !inside(**p + centre))
        .count();
    println!(
        "- The globe carries {} vertices; {} of them ({:.0}%) are outside the skin.",
        globe.positions.len(),
        outside,
        outside as f32 / globe.positions.len() as f32 * 100.0
    );

    // How far the skin reaches forward at the eye's own height and offset,
    // against where the globe's front pole is. This is the whole question:
    // a globe whose pole is in front of the skin is a ball on a face.
    let side = eyes.left.pivot.x.abs();
    let eye_line = centre.y + level;
    let socket = Vec3::new(centre.x + side, eye_line, centre.z);
    let skin_at_eye = bisect(socket, Vec3::Z, 0.30);
    let pole = eyes.left.pivot.z + eyeball;
    match skin_at_eye {
        Some(reach) => println!(
            "- At the eye's own height and {:.1} mm off the midline, the skin reaches \
             {:.1} mm forward. The globe's front pole is at {:.1} mm. The globe stands \
             **{:+.1} mm** proud of the face around it.",
            side * 1000.0,
            reach * 1000.0,
            pole * 1000.0,
            (pole - reach) * 1000.0
        ),
        None => println!("- The eye's own height is not inside the body on that column."),
    }
    // The socket, if there is one: how the skin's forward reach changes as you
    // walk up and down through the eye. A socket is a LOCAL MINIMUM here.
    println!("\n| mm up | skin reach at the eye's column |");
    println!("|---|---|");
    let mut dy = -unit * 2.0;
    while dy <= unit * 2.0 {
        let probe = Vec3::new(centre.x + side, eye_line + dy, centre.z);
        if let Some(reach) = bisect(probe, Vec3::Z, 0.30) {
            println!(
                "| {:+7.1} | {:7.1} |",
                (level + dy) * 1000.0,
                reach * 1000.0
            );
        }
        dy += 0.004;
    }
    println!();

    // ---- The carve's own contribution -------------------------------------
    println!("## What the relief carve moves, by height\n");
    let mut carved = plain.clone();
    face::carve_face(&mut carved, rig, &canon, &record.face);
    println!("| mm up | greatest outward | greatest inward |");
    println!("|---|---|---|");
    let mut band = 0.02f32;
    while band > -0.11 {
        let (mut out, mut back) = (0.0f32, 0.0f32);
        for (was, now) in plain.positions.iter().zip(&carved.positions) {
            let height = was.y - centre.y;
            if height < band - 0.005 || height >= band + 0.005 {
                continue;
            }
            let moved = (*now - *was).dot((*was - centre).normalize_or_zero());
            out = out.max(moved);
            back = back.min(moved);
        }
        println!(
            "| {:+7.1} | {:7.1} | {:7.1} |",
            band * 1000.0,
            out * 1000.0,
            back * 1000.0
        );
        band -= 0.01;
    }

    chin_blade(&plain, &carved, centre, chin, canon.nose_base());
    held_by(&avatar, centre, radius);
}

/// The chin's own section, and whether it is a point or a blade (#128).
///
/// The binding's territories, read from the weights that ship (#151/#152).
///
/// Each cell is the nearest vertex to a bisected surface point: its strongest
/// joint and that hold, with the jaw pivot's hold in brackets wherever it is
/// not the strongest. The tint renders drew the suspicion; this is the table
/// to argue from.
fn held_by(avatar: &Avatar, centre: Vec3, radius: f32) {
    let body = &avatar.parts.body;
    let rig = &avatar.rig;
    let head = *rig.in_zone(Zone::Head).first().expect("a head");
    let reach = |mesh: &PolyMesh, y: f32, degrees: f32| -> Option<f32> {
        let from = Vec3::new(centre.x, centre.y + y, centre.z);
        if !mesh.contains(from) {
            return None;
        }
        let (sin, cos) = degrees.to_radians().sin_cos();
        let along = Vec3::new(sin, 0.0, cos);
        let (mut near, mut out) = (0.0f32, 0.30f32);
        for _ in 0..40 {
            let mid = 0.5 * (near + out);
            if mesh.contains(from + along * mid) {
                near = mid;
            } else {
                out = mid;
            }
        }
        Some(near)
    };
    let _ = radius;
    // ---- Who holds the skin ------------------------------------------------
    //
    println!("\n## Who holds the skin, by height\n");
    println!("| mm | front | 45° | side | back |\n|---|---|---|---|---|");
    let weights = &avatar.parts.weights;
    let neck = rig.joints[head].parent.unwrap_or(head);
    let girdle = rig.joints[neck].parent.unwrap_or(neck);
    let pivot = (0..rig.len())
        .find(|&j| rig.joints[j].parent == Some(head) && rig.joints[j].position.y < centre.y);
    let tip = pivot.and_then(|p| (0..rig.len()).find(|&j| rig.joints[j].parent == Some(p)));
    let crown = (0..rig.len())
        .find(|&j| rig.joints[j].parent == Some(head) && rig.joints[j].position.y > centre.y);
    let label = |joint: usize| -> String {
        if joint == head {
            "head".into()
        } else if Some(joint) == pivot {
            "jawP".into()
        } else if Some(joint) == tip {
            "jawT".into()
        } else if Some(joint) == crown {
            "crown".into()
        } else if joint == neck {
            "neck".into()
        } else if joint == girdle {
            "girdle".into()
        } else if rig.joints[girdle].parent == Some(joint) {
            "chest".into()
        } else {
            format!("j{joint}")
        }
    };
    for step in 0..=15 {
        let y = 0.04 - 0.02 * step as f32;
        let mut cells: Vec<String> = Vec::with_capacity(4);
        for degrees in [0.0f32, 45.0, 90.0, 180.0] {
            let Some(out) = reach(body, y, degrees) else {
                cells.push("—".into());
                continue;
            };
            let (sin, cos) = degrees.to_radians().sin_cos();
            let at = Vec3::new(centre.x, centre.y + y, centre.z)
                + Vec3::new(sin, 0.0, cos) * (out - 0.001);
            let vertex = body
                .positions
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    a.1.distance_squared(at)
                        .total_cmp(&b.1.distance_squared(at))
                })
                .map(|(index, _)| index)
                .expect("a body has vertices");
            let row = &weights.vertices[vertex];
            let strongest = row[0];
            let jaw_hold: f32 = row
                .iter()
                .filter(|i| Some(i.joint as usize) == pivot)
                .map(|i| i.weight)
                .sum();
            let mut cell = format!(
                "{} {:.2}",
                label(strongest.joint as usize),
                strongest.weight
            );
            if jaw_hold > 0.005 && Some(strongest.joint as usize) != pivot {
                cell.push_str(&format!(" (jawP {jaw_hold:.2})"));
            }
            cells.push(cell);
        }
        println!(
            "| {:+.0} | {} | {} | {} | {} |",
            y * 1000.0,
            cells[0],
            cells[1],
            cells[2],
            cells[3]
        );
    }
}

/// The owner reported a second nose hanging off the chin in front view and a
/// flab from chin to mid-neck on the diagonal — one shape from two angles. What
/// it is, measured, is a midline that runs a long way ahead of the surface
/// either side of it, so the section at the chin's height is a blade rather than
/// an arc.
///
/// **How proud is defined, because the definition is the instrument.** Rays are
/// fired from the head's own vertical axis at 0, 15 and 30 degrees off the
/// midline; the two off-midline reaches give a straight run in azimuth, and
/// PROUD is how far past that run the midline reaches. It is a shape reading and
/// not a size one: a chin that projects a long way but carries its neighbours
/// with it reads as a jaw, and one that projects alone reads as a blade.
///
/// **Both surfaces, because the two answer different questions.** The SHAPED
/// head is where [`symbios_avatar::face`]'s own profile tables land, so it is
/// what a change to them moves. The CARVED one is what ships, and it is the only
/// place the lip can be compared with the chin — which is the check #72 was
/// bought with: cutting this amplitude once before cost the chin 7 mm and put
/// the lower lip in front of it, and a face whose lip swallows its chin has no
/// jaw at all.
fn chin_blade(shaped: &PolyMesh, carved: &PolyMesh, centre: Vec3, chin: f32, nose_base: f32) {
    println!("\n## The chin's section\n");
    println!(
        "Rays from the head's own axis, at the chin's height. PROUD is the midline against a straight run through the 15° and 30° readings.\n"
    );
    println!("| surface | dead ahead | 15° off | 30° off | straight run | proud | breadth |");
    println!("|---|---|---|---|---|---|---|");

    let reach = |mesh: &PolyMesh, y: f32, degrees: f32| -> Option<f32> {
        let from = Vec3::new(centre.x, centre.y + y, centre.z);
        if !mesh.contains(from) {
            return None;
        }
        let (sin, cos) = degrees.to_radians().sin_cos();
        let along = Vec3::new(sin, 0.0, cos);
        let (mut near, mut out) = (0.0f32, 0.30f32);
        for _ in 0..40 {
            let mid = 0.5 * (near + out);
            if mesh.contains(from + along * mid) {
                near = mid;
            } else {
                out = mid;
            }
        }
        Some(near)
    };

    for (name, mesh) in [("shaped", shaped), ("carved", carved)] {
        let (Some(ahead), Some(off15), Some(off30)) = (
            reach(mesh, chin, 0.0),
            reach(mesh, chin, 15.0),
            reach(mesh, chin, 30.0),
        ) else {
            println!("| {name} | the midline is outside the surface at the chin's height |");
            continue;
        };
        // **BREADTH, and not a ray fired sideways from the axis.** A blade is an
        // off-centre section, and a ray crossing one from the head's own centre
        // depth reads a CHORD — it reports the section narrower the further
        // forward its mass sits, which on this shape is the whole finding
        // running backwards. #125 paid for that reading once on the neck. So the
        // slice is swept front to back and asked for its widest point.
        let behind = reach(mesh, chin, 180.0).unwrap_or(0.0);
        let at = Vec3::new(centre.x, centre.y + chin, centre.z);
        let wide = (0..=40)
            .filter_map(|slice| {
                let z = at.z - behind + (ahead + behind) * slice as f32 / 40.0;
                let from = Vec3::new(at.x, at.y, z);
                if !mesh.contains(from) {
                    return None;
                }
                let (mut near, mut out) = (0.0f32, 0.30f32);
                for _ in 0..40 {
                    let mid = 0.5 * (near + out);
                    if mesh.contains(from + Vec3::X * mid) {
                        near = mid;
                    } else {
                        out = mid;
                    }
                }
                Some(near)
            })
            .fold(0.0f32, f32::max);
        // The straight run is in AZIMUTH, so the two neighbours are one step
        // apart and the midline is one step beyond the nearer of them.
        let run = 2.0 * off15 - off30;
        // Breadth is context and not the defect. The section's depth-to-breadth
        // is NOT printed beside it on purpose: at the chin's height that depth
        // runs from the chin to the nape, so the ratio would be the head's
        // aspect wearing the chin's name. What says blade is the fall from the
        // midline to 15° — 37 mm on the default body — and `proud` beside it.
        println!(
            "| {name} | {:.1} | {:.1} | {:.1} | {:.1} | **{:+.1}** | {:.1} |",
            ahead * 1000.0,
            off15 * 1000.0,
            off30 * 1000.0,
            run * 1000.0,
            (ahead - run) * 1000.0,
            wide * 2000.0,
        );
    }

    // The lip against the chin, on the surface that ships, and **by a definition
    // rather than by a window.** #108 measured the lip reaching 119.2 mm against
    // the chin's 114.0 and had to rewrite
    // `the_chin_landmark_lands_on_the_chin_of_the_shipped_face` for exactly this
    // reason: a ±25 mm scan answers with the chin only while the lip stays
    // outside it, and reads the edge of its own window everywhere else.
    //
    // So the lip is found the way the anatomy runs. Walking up the midline from
    // the chin the profile falls into the mentolabial sulcus — the crease under
    // the lower lip — and rises again into the lip itself. The first local
    // minimum above the chin is that crease, and the greatest reach between it
    // and the nose base is the lip. If there is no crease the profile has no
    // lower lip on it, and that is worth saying rather than papering over.
    let Some(at_chin) = reach(carved, chin, 0.0) else {
        return;
    };
    let profile: Vec<(f32, f32)> = (0..)
        .map(|step| chin + 0.001 * step as f32)
        .take_while(|y| *y < nose_base)
        .filter_map(|y| reach(carved, y, 0.0).map(|here| (y, here)))
        .collect();
    // Turning points with a deadband, for the reason `neckaudit` counts its
    // turns with one: the surface ripples a few tenths of a millimetre between
    // samples and a reversal under that is not a feature.
    const DEADBAND: f32 = 0.0004;
    let mut turns: Vec<(f32, f32, bool)> = Vec::new();
    let mut rising: Option<bool> = None;
    let mut mark = profile[0];
    for &(y, here) in &profile[1..] {
        if (here - mark.1).abs() < DEADBAND {
            continue;
        }
        let now = here > mark.1;
        if rising.is_some_and(|was| was != now) {
            turns.push((mark.0, mark.1, now));
        }
        rising = Some(now);
        mark = (y, here);
    }
    // Up from the chin the profile falls into the crease and rises into the
    // lip, so the two are the first turn that starts a rise and the first that
    // starts a fall after it.
    let crease = turns.iter().find(|(_, _, up)| *up).copied();
    let (Some((crease_at, crease, _)), Some(&(lip_at, lip, _))) = (
        crease,
        turns
            .iter()
            .skip_while(|(_, _, up)| !*up)
            .find(|(_, _, up)| !*up),
    ) else {
        println!(
            "\nOn the carved midline the chin reaches {:.1} mm and the profile above it has no \
             crease and crest on it before the nose base — there is no lower lip here to \
             measure the chin against.",
            at_chin * 1000.0,
        );
        return;
    };
    println!(
        "\nOn the carved midline: the chin reaches {:.1} mm, the crease under the lip falls to \
         {:.1} at {:+.1} mm, and the lower lip rises to {:.1} at {:+.1}. **The chin leads its \
         own lip by {:+.1} mm.** A chin is supposed to win this — #72 cut this amplitude once \
         before, lost 7 mm of projection, and put the lip in front.",
        at_chin * 1000.0,
        crease * 1000.0,
        crease_at * 1000.0,
        lip * 1000.0,
        lip_at * 1000.0,
        (at_chin - lip) * 1000.0,
    );
}

/// The head's overall proportions, in head-local metres.
///
/// **Here rather than in a scratch script, which is where it was.** #79's
/// baseline — where the head is widest, how wide it is for its height, the jaw's
/// angle against the cheekbone — was measured by a harness that was never
/// committed, so every number on that issue went stale the moment #107 moved the
/// cage and there was nothing to re-run. A measurement that cannot be repeated
/// is a measurement that has to be argued about instead.
struct Proportions {
    /// The widest half-width anywhere on the head, and where.
    widest: f32,
    widest_at: f32,
    /// Crown to the chin's tip.
    height: f32,
    /// Twice [`Proportions::widest`].
    width: f32,
    /// The greatest fore-and-aft extent anywhere on the head.
    depth: f32,
    /// The same over the VAULT alone, above the brow.
    ///
    /// **The column life's 1.28 is actually a ratio of, and [`Proportions::depth`]
    /// above is not** (#79). Head length is glabella to opisthocranion: it stops
    /// at the brow and does not include a nose. Measured anywhere from the chin
    /// up, a face with a prominent nose reports its nose — on seed 42 the
    /// greatest extent sits at −0.08 R, which is the nose tip against the
    /// occiput, and reads 1.39 where the cranium reads 1.31. Both numbers are
    /// printed because the difference between them is a fact about the face; the
    /// one to compare with 1.28 is this one.
    vault: f32,
    /// Face width at the cheekbone and at the angle of the jaw.
    bizygomatic: f32,
    bigonial: f32,
    /// Crown-to-eye-line against eye-line-to-chin.
    cranium_to_face: f32,
    /// Where the eye's aperture is centred, in degrees off the gaze.
    ///
    /// **Skin AND lids, which is what #88 and #79 both quote and what the first
    /// version of this column did not measure.** Asked of the skin alone the
    /// same eye reads twenty degrees further lateral, because the lid's own
    /// margin owns that edge (#81) — so a column headed "eye azimuth" that
    /// omitted the lids was reporting a number nothing in the crate is tuned
    /// against, and would have said the aperture had regressed when it had not.
    /// The fifteenth instrument in this project to have measured something other
    /// than its own name.
    eye_azimuth: f32,
}

impl Proportions {
    /// How far a life-proportioned head of this height is from this one's width.
    ///
    /// Human height-to-breadth is 1.48. A head broader than that for its own
    /// length is too wide by this much, and it is the figure #79 raised and
    /// #61 has to choose a breadth default against.
    fn too_wide(&self) -> f32 {
        LIFE_HEIGHT_TO_WIDTH / (self.height / self.width) - 1.0
    }

    /// Measures a built head.
    ///
    /// **Every reading here is a bisection against the surface and every height
    /// comes off a landmark, not off a constant.** The chin is [`Skull::chin`]
    /// and the angle of the jaw is [`Skull::gonion`], so this report and
    /// `the_face_narrows_from_cheekbone_to_chin` measure the same two places by
    /// construction. The first version of #79's harness probed DOWN the midline
    /// for a chin, which on a body whose head and neck are one surface walks
    /// through the throat and the chest and reports a head a metre tall.
    fn measure(
        body: &symbios_avatar::PolyMesh,
        centre: Vec3,
        radius: f32,
        skull: &Skull,
        canon: &Canon,
        eyes: &Eyes,
    ) -> Self {
        let inside = |p: Vec3| body.contains(p);
        let bisect = |from: Vec3, along: Vec3| -> Option<f32> {
            if !inside(from) {
                return None;
            }
            let (mut near, mut out) = (0.0f32, 0.30f32);
            for _ in 0..40 {
                let mid = 0.5 * (near + out);
                if inside(from + along * mid) {
                    near = mid;
                } else {
                    out = mid;
                }
            }
            Some(near)
        };
        let at = |y: f32| Vec3::new(centre.x, centre.y + y, centre.z);

        let mut crown = 0.0f32;
        let mut y = 0.0;
        while inside(at(y)) {
            crown = y;
            y += STEP;
        }

        // Widest and deepest, swept from the chin's tip to the crown. Started at
        // the chin rather than at the head's own floor on purpose: the surface
        // runs on past the chin into the throat, which is broader than a jaw, so
        // a sweep that reaches the floor reports the neck as the head's widest
        // point on any body with a slender face.
        let chin = skull.chin();
        let (mut widest, mut widest_at, mut depth, mut vault) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let mut y = chin;
        while y <= crown {
            if let Some(wide) = bisect(at(y), Vec3::X)
                && wide > widest
            {
                widest = wide;
                widest_at = y;
            }
            if let (Some(ahead), Some(behind)) = (bisect(at(y), Vec3::Z), bisect(at(y), -Vec3::Z)) {
                depth = depth.max(ahead + behind);
                // Above the brow, which `BROW` runs out at by 0.58 R, so
                // nothing on the face reaches into this.
                if y > 0.30 * radius {
                    vault = vault.max(ahead + behind);
                }
            }
            y += STEP;
        }

        // The face's own widths, taken half-way forward on the midline's reach
        // rather than through the head's centre. Through the centre the sample
        // at the chin's height sits ninety millimetres behind the chin and is
        // the upper neck seen from the side, which is the reading that let a
        // cone pass this check for three rounds (#80).
        let face_width = |height: f32| -> Option<f32> {
            let reach = bisect(at(height), Vec3::Z)?;
            bisect(at(height) + Vec3::Z * reach * 0.5, Vec3::X)
        };
        let bizygomatic = face_width(-0.05 * radius).unwrap_or(f32::NAN) * 2.0;
        let bigonial = face_width(skull.gonion()).unwrap_or(f32::NAN) * 2.0;

        Self {
            widest,
            widest_at,
            height: crown - chin,
            width: widest * 2.0,
            depth,
            vault,
            bizygomatic,
            bigonial,
            cranium_to_face: (crown - canon.level) / canon.frame,
            eye_azimuth: eyes
                .right
                .aperture(Some((body, centre)), true)
                .centre
                .0
                .to_degrees(),
        }
    }
}

/// Height over breadth on a human head.
///
/// Vertex-to-menton against maximum breadth. The one ratio #61's breadth default
/// has to be chosen against, and the reason #79 could say the head was 11% wide
/// without anybody having tuned it wide on purpose.
const LIFE_HEIGHT_TO_WIDTH: f32 = 1.48;

/// The four axes this report can hold at a chosen value.
///
/// `None` means "whatever the seed rolled", which is what the plain sweep wants;
/// `Some` is what `--axis` walks.
#[derive(Clone, Copy, Default)]
struct Axes {
    breadth: Option<f32>,
    length: Option<f32>,
    nose: Option<f32>,
    mouth: Option<f32>,
    /// In whole years, and a composite rather than a per-region axis — the
    /// first one this instrument can walk (#167).
    age: Option<u32>,
}

impl Axes {
    /// Puts them on a record.
    fn onto(self, record: &mut AvatarRecord) {
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.head_breadth = self.breadth.unwrap_or(params.head_breadth);
            params.face_length = self.length.unwrap_or(params.face_length);
        }
        record.face.nose_width = self.nose.unwrap_or(record.face.nose_width);
        record.face.mouth_width = self.mouth.unwrap_or(record.face.mouth_width);
        record.composites.age = self.age.unwrap_or(record.composites.age);
        record.sanitize();
    }
}

/// Walks one axis from end to end on every sweep seed.
///
/// **The step #61 exists to make possible, and the one the complexion half of
/// #39 skipped**: the undertone axis swung 47 to 59 degrees of hue and was still
/// wrong, because nobody had looked at what it did at BOTH ends on more than one
/// body. An axis that moves something is not the same as an axis that means
/// something.
fn walk(axis: &str) {
    let values: [f32; 5] = match axis {
        "nose" | "mouth" => [0.0, 0.25, 0.5, 0.75, 1.0],
        // Years, and the first two are both under `plan::AGE_PIVOT` on purpose:
        // the age axis is the identity below its pivot, so two rows that have
        // to come out bit-identical are the cheapest check that this
        // instrument is reading the axis at all (#167).
        "age" => [18.0, 30.0, 55.0, 70.0, 80.0],
        _ => [-1.0, -0.5, 0.0, 0.5, 1.0],
    };
    for value in values {
        let axes = match axis {
            "breadth" => Axes {
                breadth: Some(value),
                ..Axes::default()
            },
            "length" => Axes {
                length: Some(value),
                ..Axes::default()
            },
            "nose" => Axes {
                nose: Some(value),
                ..Axes::default()
            },
            "mouth" => Axes {
                mouth: Some(value),
                ..Axes::default()
            },
            "age" => Axes {
                age: Some(value as u32),
                ..Axes::default()
            },
            other => {
                println!("no axis named {other}; try breadth, length, nose or mouth");
                return;
            }
        };
        println!("## {axis} = {value:+.2}\n");
        sweep(&SWEEP_SEEDS, axes);
        println!();
    }
}

/// One row of [`Proportions`] per seed.
///
/// The whole point of the mode: a breadth axis has a default, and a default
/// chosen against one body is how the last three head passes each found their
/// number moved on the next seed.
fn sweep(seeds: &[i64], axes: Axes) {
    println!(
        "| seed | head R | H | H/R | widest | at R | H:W | D:W | vault:W | bizyg | \
         bigon/bizyg | cran:face | frame | nose | mouth | eye az |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for &seed in seeds {
        let mut record = AvatarRecord::new("Swept", Archetype::default());
        record.reroll(seed);
        axes.onto(&mut record);
        // A 32-texel atlas: this measures geometry and a full bake is most of
        // the build time on a sweep this size.
        let config = AvatarConfig {
            atlas: 32,
            ..AvatarConfig::default()
        };
        let Some(avatar) = Avatar::build_with(&record, &config) else {
            println!("| {seed} | — | this record does not describe a body that meshes |");
            continue;
        };
        let body = &avatar.parts.body;
        let rig = &avatar.rig;
        let Some(head) = rig.in_zone(Zone::Head).first().copied() else {
            continue;
        };
        let centre = rig.joints[head].position;
        let radius = rig.joints[head].radius;
        let Some(skull) = Skull::measure(body, rig) else {
            continue;
        };
        let canon = Canon::measure(rig, &skull, &record.eyes);
        let Some(eyes) = avatar.parts.eyes.as_ref() else {
            continue;
        };
        let shape = Proportions::measure(body, centre, radius, &skull, &canon, eyes);
        let features = Features::measure(&record, rig, &canon, centre, radius);
        println!(
            "| {seed} | {:.1} | {:.1} | {:.3} | {:.1} | {:+.2} | {:.3} | {:.2} | {:.2} | \
             {:.1} | {:.3} | {:.2} | {:.1} | {:.1} | {:.1} | {:+.0}° |",
            radius * 1000.0,
            shape.height * 1000.0,
            shape.height / radius,
            shape.widest * 1000.0,
            shape.widest_at / radius,
            shape.height / shape.width,
            shape.depth / shape.width,
            shape.vault / shape.width,
            shape.bizygomatic * 1000.0,
            shape.bigonial / shape.bizygomatic,
            shape.cranium_to_face,
            canon.frame * 1000.0,
            features.nose * 1000.0,
            features.mouth * 1000.0,
            shape.eye_azimuth,
        );
    }
}

/// How wide the carve actually draws the nose and the mouth, in metres.
///
/// **Measured off the displacement, not computed from the constants that drew
/// it.** A report that recomputes a feature's width from its own copy of the
/// ramp is not measuring the body, it is restating the source — which is how
/// `examples/headaudit` went on printing the canon's old thirds for a whole
/// milestone after #78 moved them. This carves a copy of the head and asks where
/// the carve stops pushing.
struct Features {
    nose: f32,
    mouth: f32,
}

impl Features {
    /// How far from the midline the carve still moves the surface at a height.
    ///
    /// Taken as the widest sample whose outward movement is at least a fifth of
    /// the movement on the midline at that same height, so it reports the
    /// feature's own shoulder rather than the tail of its Gaussian. Doubled on
    /// return, because a nose has two sides.
    fn measure(
        record: &AvatarRecord,
        rig: &symbios_avatar::Rig,
        canon: &Canon,
        centre: Vec3,
        radius: f32,
    ) -> Self {
        let skeleton = record.skeleton();
        let Ok(plain) = symbios_avatar::build_body(
            &skeleton,
            &symbios_avatar::CageConfig::default(),
            symbios_avatar::BODY_SUBDIVISIONS,
            &Default::default(),
        ) else {
            return Self {
                nose: f32::NAN,
                mouth: f32::NAN,
            };
        };
        let mut carved = plain.clone();
        face::carve_face(&mut carved, rig, canon, &record.face);

        let width = |height: f32, outward: bool| -> f32 {
            // Every vertex within half a millimetre of the height asked for,
            // paired with how far the carve moved it along the head's radial.
            // The window is a fraction of the head rather than a fixed
            // millimetre figure: the rows under the face are 0.015 radii apart,
            // so a 1.5 mm window catches three rows on a small head and none at
            // all on a large one — which is how the first version of this
            // reported a nose of NaN on the two biggest seeds.
            let mut moved: Vec<(f32, f32)> = Vec::new();
            for (was, now) in plain.positions.iter().zip(&carved.positions) {
                if (was.y - centre.y - height).abs() > radius * 0.02 {
                    continue;
                }
                let along = (*now - *was).dot((*was - centre).normalize_or_zero());
                moved.push((
                    (was.x - centre.x).abs(),
                    if outward { along } else { -along },
                ));
            }
            let peak = moved
                .iter()
                .filter(|&&(across, _)| across < radius * 0.04)
                .fold(0.0f32, |most, &(_, along)| most.max(along));
            if peak <= 0.0 {
                return f32::NAN;
            }
            moved
                .iter()
                .filter(|&&(_, along)| along >= peak * 0.20)
                .fold(0.0f32, |wide, &(across, _)| wide.max(across))
                * 2.0
        };

        Self {
            // The nose at the base, where the wings are; the mouth at the lip
            // line, where a mouth is measured. The mouth's own line is a GROOVE,
            // so it is read as inward movement.
            nose: width(canon.nose_base(), true),
            mouth: width(canon.mouth_line(), false),
        }
    }
}
