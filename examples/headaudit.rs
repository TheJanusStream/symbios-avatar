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

use symbios_avatar::face::{Eyes, Skull};
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
    let eyes = Eyes::build(rig, &record.eyes).expect("a humanoid has eyes");
    let skull = Skull::measure(body, rig).expect("a skull");

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
    let unit = eyes.left.radius;
    let level = eyes.left.pivot.y;
    let chin = skull.chin();
    let (span_lo, span_hi) = skull.throat_and_crown();
    println!("## What the crate believes\n");
    println!("| landmark | mm | source |");
    println!("|---|---|---|");
    println!(
        "| eye globe radius | {:.1} | eyes.left.radius, the unit every feature is sized in |",
        unit * 1000.0
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
    for (name, fraction) in [
        ("nose base", 0.51f32),
        ("mouth line", 0.69),
        ("ear centre", 0.19),
    ] {
        println!(
            "| {name} | {:+.1} | eye + {fraction} x frame |",
            (level + (chin - level) * fraction) * 1000.0
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
    let pole = eyes.left.pivot.z + unit;
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
    let skeleton = record.skeleton();
    let plain = symbios_avatar::build_body(&skeleton, &symbios_avatar::CageConfig::default(), 2)
        .expect("a body builds");
    let mut carved = plain.clone();
    face::carve_face(&mut carved, rig, &eyes, &record.face);
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
