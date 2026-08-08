//! The column from the crown to the shoulders, ours against the reference's.
//!
//! ```text
//! cargo run --release --example column
//! cargo run --release --example column -- 7
//! ```
//!
//! # Why this exists beside `neckaudit`
//!
//! [`neckaudit`](../neckaudit/index.html) answers *how long* the visible neck is
//! and who owns that length. This answers a different question — *what shape*
//! the column is — and it answers it against the CC0 reference body rather than
//! against a canon, because "the neck reads wrong" has three times been a
//! statement about the shoulder rather than about the neck (#125, #129, #143).
//!
//! # Exact plane cuts, not bins
//!
//! Every figure is the set of points where a face edge crosses a horizontal
//! plane. **Not vertices in a band**, which is the trap this project keeps
//! falling into: the reference body is 7,399 vertices and a five-millimetre band
//! of it catches a scatter of whatever rows happen to fall inside, so the first
//! version of this measurement produced a table that jumped between 22 mm and
//! 200 mm of half-width on neighbouring rows. A plane cut does not care how a
//! surface is tessellated.
//!
//! Heights are relative to each body's own **chin**, so two bodies of different
//! stature are compared at the same anatomy. `|x| < 0.20 m` keeps the T-posed
//! arms out of the section.
//!
//! # What it showed the first time it was run (#143)
//!
//! Two findings, and the second corrected an impression taken from a render:
//!
//! - **The shoulder arrives about seventy millimetres too low.** Ours holds a
//!   flat 51–53 mm half-width from the chin all the way to −80, then flares. The
//!   reference is at 71 mm at the chin and fully flared to 191 by −50. That is
//!   #129's finding — "the flare is right and it happens too low" — with a
//!   sharper number on it.
//! - **Our neck is not too wide. It is NARROWER than the reference's**, 50.5 mm
//!   at its narrowest against 66.7. What reads as a slab in a render is ninety
//!   millimetres of unchanging width, not excess width, and any fix aimed at
//!   slimming it would have been aimed at nothing.
//!
//! # What it showed the second time, and the reading to be careful with (#131)
//!
//! The second finding above was acted on and `neck_r` went from `0.030 · girth`
//! to `0.040 + 0.020 · (girth − 1)`. Two cautions for anyone reading the summary
//! lines at the bottom of a run.
//!
//! **The narrowest half-width is not always the neck.** `narrowest` scans from
//! 110 mm above the chin to 80 below, so once the neck is wide enough the
//! minimum migrates up to the JAWLINE — which is where the reference's own
//! minimum has always been, at +10. That is why widening the node stops moving
//! this figure: on seed 7 a 53% wider node bought 5.6 mm, because the last of it
//! was measuring a jaw.
//!
//! **Compare at equal stature or not at all.** The reference is one 1.829 m
//! body. Seed 21 reads 70.4 mm against its 66.7 and looks already-wide; it is a
//! 2.03 m body and scales to 63.4, narrow like every other seed. Every one of
//! seeds 0, 3, 7, 13 and 21 was narrow against the reference once its own
//! stature was divided out, which is what made a base raise defensible.
//!
//! And the throat differs in **rate** rather than in amount: the reference gives
//! up 45 mm of forward reach in the ten millimetres between −10 and −20 and is
//! then vertical, where ours spreads a slightly larger 63 mm over a fifty-
//! millimetre ramp. A corner against a slope.

use symbios_avatar::face::Skull;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Zone};

/// The CC0 reference body's own column, measured with this same plane cut.
///
/// Rows are `(height below the chin, half-width, back z, front z)` in
/// millimetres, taken every ten millimetres from 250 above the chin to 240
/// below.
///
/// **Extended from +110 to the crown** (#144). The old table stopped 10 mm short
/// of where the reference is WIDEST, so every reading taken from it under-read
/// the vault: its maximum is 86.7 mm at +120 to +140 and the top row said 85.7.
/// The crown sits at +260.0. Regenerated with the same plane cut against the
/// same mesh — 7,399 vertices, 13,757 triangles — and the overlapping rows
/// reproduce the originals to 0.4 mm, which is the calibration on the chin
/// height rather than a disagreement.
///
/// Provenance: **measured** (#143), off `model-human.glb` in the mesh2motion
/// checkout, with its chin taken at its own `head` joint — which #126 measured
/// sits at the chin on that rig, 4.4% of the head below it. Recorded here as
/// numbers rather than read at run time because [`crate::gltf`] deliberately
/// reads no meshes: it is an animation reader, and giving it a mesh path to
/// support one instrument would be the largest thing in this crate that only
/// one tool uses.
#[rustfmt::skip]
const REFERENCE: [(i32, f32, f32, f32); 50] = [
    ( 250,   54.8,   -45.2,   47.1),
    ( 240,   62.5,   -65.1,   68.9),
    ( 230,   70.2,   -79.8,   78.2),
    ( 220,   77.4,   -89.7,   87.5),
    ( 210,   80.0,   -97.5,   96.8),
    ( 200,   82.5,  -105.2,  102.7),
    ( 190,   84.0,  -108.2,  106.9),
    ( 180,   85.2,  -110.5,  111.0),
    ( 170,   86.4,  -111.1,  113.2),
    ( 160,   86.6,  -108.6,  115.5),
    ( 150,   86.6,  -106.2,  117.1),
    ( 140,   86.7,  -103.8,  117.3),
    ( 130,   86.7,  -101.3,  117.6),
    ( 120,   86.7,   -98.6,  117.8),
    ( 110,  85.7,  -95.1, 118.0),
    ( 100,  84.6,  -91.8, 118.3),
    (  90,  83.5,  -87.4, 117.9),
    (  80,  82.4,  -88.9, 116.8),
    (  70,  80.2,  -92.7, 115.7),
    (  60,  76.0,  -96.4, 114.6),
    (  50,  73.4,  -99.4, 113.5),
    (  40,  71.5, -102.5, 111.9),
    (  30,  69.6, -107.1, 110.0),
    (  20,  68.0, -111.6, 108.2),
    (  10,  66.7, -115.6, 106.3),
    (   0,  71.2, -119.5,  98.6),
    ( -10,  80.9, -124.4,  89.0),
    ( -20,  96.8, -129.4,  53.6),
    ( -30, 115.3, -133.9,  44.0),
    ( -40, 134.7, -138.5,  42.2),
    ( -50, 191.2, -142.6,  40.4),
    ( -60, 199.0, -146.7,  39.3),
    ( -70, 188.4, -150.8,  42.7),
    ( -80, 195.6, -153.9,  52.5),
    ( -90, 185.7, -157.0,  65.3),
    (-100, 182.1, -159.4,  78.4),
    (-110, 196.2, -160.9,  87.7),
    (-120, 186.1, -162.5,  94.2),
    (-130, 190.2, -163.2, 100.5),
    (-140, 192.9, -163.9, 106.0),
    (-150, 191.7, -163.6, 111.6),
    (-160, 192.3, -162.6, 116.1),
    (-170, 193.4, -161.6, 119.3),
    (-180, 175.1, -160.9, 122.5),
    (-190, 172.2, -160.2, 125.7),
    (-200, 169.4, -159.9, 128.7),
    (-210, 170.2, -159.6, 128.2),
    (-220, 171.4, -158.9, 127.7),
    (-230, 171.0, -154.3, 127.3),
    (-240, 169.8, -149.6, 127.1),
];

/// How far from the midline a section is taken, in metres.
///
/// Wide enough to hold a shoulder, narrow enough to leave a T-posed arm out.
const REACH: f32 = 0.20;

fn main() {
    let seed: i64 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(0);
    let mut record = AvatarRecord::new("Column", Archetype::default());
    if seed != 0 {
        record.reroll(seed);
    }
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("seed {seed} does not build a body");
        std::process::exit(1);
    };
    let mesh = &avatar.parts.body;
    let Some(skull) = Skull::measure(mesh, &avatar.rig) else {
        eprintln!("this body has no head to measure a column against");
        std::process::exit(1);
    };
    let head = avatar.rig.in_zone(Zone::Head)[0];
    let chin = avatar.rig.joints[head].position.y + skull.chin();

    println!("THE COLUMN, crown to shoulders, seed {seed} against the CC0 reference.");
    println!("Millimetres. Heights are each body's own chin, so two statures compare");
    println!("at the same anatomy. Sections are cut with |x| < {REACH:.2} m, arms excluded.\n");
    println!(
        "{:>8}  {:>17}  {:>17}  {:>17}",
        "vs chin", "half-width", "back z", "front z"
    );
    println!(
        "{:>8}  {:>8}{:>9}  {:>8}{:>9}  {:>8}{:>9}",
        "", "ours", "ref", "ours", "ref", "ours", "ref"
    );
    println!("{}", "-".repeat(68));

    let mut widest_gap = (0.0f32, 0i32);
    for &(at, their_width, their_back, their_front) in &REFERENCE {
        let height = chin + at as f32 / 1000.0;
        let points: Vec<(f32, f32)> = cut(mesh, height)
            .into_iter()
            .filter(|(x, _)| x.abs() < REACH)
            .collect();
        if points.len() < 3 {
            continue;
        }
        let width = points.iter().fold(0.0f32, |w, p| w.max(p.0.abs())) * 1000.0;
        let back = points.iter().fold(f32::MAX, |z, p| z.min(p.1)) * 1000.0;
        let front = points.iter().fold(f32::MIN, |z, p| z.max(p.1)) * 1000.0;
        if their_width - width > widest_gap.0 {
            widest_gap = (their_width - width, at);
        }
        println!(
            "{at:>8}  {width:>8.1}{their_width:>9.1}  {back:>8.1}{their_back:>9.1}  \
             {front:>8.1}{their_front:>9.1}"
        );
    }

    println!("{}", "-".repeat(68));
    println!(
        "widest the reference's shoulder mass leads ours: {:.0} mm, at {} below the chin",
        widest_gap.0, widest_gap.1
    );
    vault(mesh, chin);

    let least = narrowest(mesh, chin);
    println!("our narrowest half-width: {least:.1} mm; the reference's: 66.7");

    // The neck node's own lateral half-extent beside what the surface delivers
    // of it. Printed because `neck_r` is the only coefficient that can move the
    // flat run of this column, and a factor applied to it is only worth what
    // the surface hands back — see #131.
    let neck = avatar.rig.in_zone(Zone::Neck)[0];
    let joint = &avatar.rig.joints[neck];
    let reach = joint.radius * joint.scale.x * 1000.0;
    println!(
        "the neck node reaches {reach:.1} mm sideways; the surface delivers {:.3} of it",
        least / reach
    );
}

/// The head's own proportions, cut with the same planes as the column above.
///
/// **Why this is here and not in `headaudit`** (#144). That tool reads the crown
/// off `Skull::span`, which is a *measured* landmark that stops about 20 mm
/// under the mesh's actual top, and its widths off `Skull`'s azimuthal profile.
/// Both are the right answers to their own questions and neither can be held
/// against a foreign mesh, which has no `Skull`. Everything below is a plane cut
/// and nothing else, so the same three lines describe our body and the CC0
/// reference and the comparison means something.
///
/// The reference's own figures, from the table above: a crown 260.0 mm over its
/// chin, a maximum breadth of 173.4 mm at +120 to +140, and about 218 mm of
/// depth there — so **H:W 1.499 and D:W 1.257**. Life is 1.53 and 1.28.
fn vault(mesh: &symbios_avatar::PolyMesh, chin: f32) {
    let (mut crown, mut widest, mut at, mut depth) = (0.0f32, 0.0f32, 0, 0.0f32);
    for step in 0..=320 {
        let points: Vec<(f32, f32)> = cut(mesh, chin + step as f32 / 1000.0)
            .into_iter()
            .filter(|(x, _)| x.abs() < REACH)
            .collect();
        if points.len() < 3 {
            continue;
        }
        crown = step as f32;
        let half = points.iter().fold(0.0f32, |w, p| w.max(p.0.abs())) * 1000.0;
        if half > widest {
            let back = points.iter().fold(f32::MAX, |z, p| z.min(p.1)) * 1000.0;
            let front = points.iter().fold(f32::MIN, |z, p| z.max(p.1)) * 1000.0;
            (widest, at, depth) = (half, step, front - back);
        }
    }
    let breadth = widest * 2.0;
    let top = mesh.positions.iter().fold(f32::MIN, |a, p| a.max(p.y));
    let floor = mesh.positions.iter().fold(f32::MAX, |a, p| a.min(p.y));
    let stature = (top - floor) * 1000.0;
    println!(
        "\nthe vault, plane-cut: crown {crown:.0} mm over the chin, widest {breadth:.1} mm \
         across at +{at}, {depth:.1} deep there"
    );
    println!(
        "  head over stature {:.4} and breadth over stature {:.4} on a {stature:.0} mm body \
         (reference 0.1421 and 0.0948)",
        crown / stature,
        breadth / stature
    );
    println!(
        "  height : width {:.3}   (reference 1.499, life 1.53)",
        crown / breadth
    );
    println!(
        "  depth  : width {:.3}   (reference 1.257, life 1.28)",
        depth / breadth
    );
}

/// Every point where a face edge crosses the plane at `height`.
///
/// **Not vertices in a band.** A cut needs no tolerance and does not care how a
/// surface is tessellated, which is the whole reason this is not the obvious
/// thing — see the module docs for what the obvious thing produced.
fn cut(mesh: &symbios_avatar::PolyMesh, height: f32) -> Vec<(f32, f32)> {
    let mut points = Vec::new();
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let a = mesh.positions[face[corner] as usize];
            let b = mesh.positions[face[(corner + 1) % face.len()] as usize];
            if (a.y - height) * (b.y - height) < 0.0 {
                let along = (height - a.y) / (b.y - a.y);
                points.push((a.x + along * (b.x - a.x), a.z + along * (b.z - a.z)));
            }
        }
    }
    points
}

/// The narrowest half-width anywhere in the column, in millimetres, and the
/// column's depth-to-width where it is narrowest.
///
/// **The waist is the only landmark the two columns share, which is why the
/// second figure is here** (#131, settled in #144). Comparing the neck's depth
/// at a fixed height under the chin is the trap #144 was closed on: our column
/// and the reference's have different lengths, so the same millimetre offset is
/// different anatomy on each. Both bodies have exactly one narrowest point —
/// theirs at +10 above the chin, ours below it — and the ratio there is
/// stature-free, anchor-free and the same question on both.
///
/// The reference's own waist: 66.7 mm of half-width and 221.9 mm of depth, so
/// **D:W 1.663**.
fn narrowest(mesh: &symbios_avatar::PolyMesh, chin: f32) -> f32 {
    let (mut least, mut depth, mut at) = (f32::MAX, 0.0f32, 0);
    for step in -80..=110 {
        let points: Vec<(f32, f32)> = cut(mesh, chin + step as f32 / 1000.0)
            .into_iter()
            .filter(|(x, _)| x.abs() < REACH)
            .collect();
        if points.len() < 3 {
            continue;
        }
        let half = points.iter().fold(0.0f32, |w, p| w.max(p.0.abs()));
        if half < least {
            let back = points.iter().fold(f32::MAX, |z, p| z.min(p.1));
            let front = points.iter().fold(f32::MIN, |z, p| z.max(p.1));
            (least, depth, at) = (half, front - back, step);
        }
    }
    println!(
        "at the column's own waist, +{at} over the chin: {:.1} deep on {:.1} across, \
         D:W {:.3}   (reference 1.663 at +10)",
        depth * 1000.0,
        least * 2000.0,
        depth / (least * 2.0)
    );
    least * 1000.0
}
