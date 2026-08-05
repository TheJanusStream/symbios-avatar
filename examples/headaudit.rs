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
//! ```

use symbios_avatar::face::{Canon, Eyes, Skull};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3, Zone, face};

/// Millimetres between samples, everywhere.
const STEP: f32 = 0.002;

fn main() {
    let seed = std::env::args()
        .skip(1)
        .find_map(|arg| arg.parse::<i64>().ok());
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
        "| Skull::chin | {:+.1} | CHIN profile peak through the floor scaling |",
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
}
