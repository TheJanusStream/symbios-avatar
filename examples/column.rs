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
//! statement about the shoulder rather than about the neck.
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
//! # What it showed the first time it was run
//!
//! Two findings, and the second corrected an impression taken from a render:
//!
//! - **The shoulder arrives about seventy millimetres too low.** Ours holds a
//!   flat 51–53 mm half-width from the chin all the way to −80, then flares. The
//!   reference is at 71 mm at the chin and fully flared to 191 by −50. That is
//!   The flare itself is right; it happens too low.
//! - **Our neck is not too wide. It is NARROWER than the reference's**, 50.5 mm
//!   at its narrowest against 66.7. What reads as a slab in a render is ninety
//!   millimetres of unchanging width, not excess width, and any fix aimed at
//!   slimming it would have been aimed at nothing.
//!
//! # What it showed the second time, and the reading to be careful with
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

use symbios_avatar::face::{Canon, HeadTraits, Skull, carve_face};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, PolyMesh, Rig, Zone, build_body,
};

/// The CC0 reference body's own column, measured with this same plane cut.
///
/// Rows are `(height below the chin, half-width, back z, front z)` in
/// millimetres, taken every ten millimetres from 250 above the chin to 240
/// below.
///
/// **Extended from +110 to the crown**. The old table stopped 10 mm short
/// of where the reference is WIDEST, so every reading taken from it under-read
/// the vault: its maximum is 86.7 mm at +120 to +140 and the top row said 85.7.
/// The crown sits at +260.0. Regenerated with the same plane cut against the
/// same mesh — 7,399 vertices, 13,757 triangles — and the overlapping rows
/// reproduce the originals to 0.4 mm, which is the calibration on the chin
/// height rather than a disagreement.
///
/// Provenance: **measured**, off `model-human.glb` in the mesh2motion
/// checkout, with its chin taken at its own `head` joint — which sits at the
/// chin on that rig, 4.4% of the head below it. Recorded here as
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

    shelf(&record, mesh, chin, avatar.rig.joints[head].position);

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
/// **Why this is here and not in `headaudit`**. That tool reads the crown
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
/// second figure is here.** Comparing the neck's depth
/// at a fixed height under the chin is a trap: our column
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

/// How far the CC0 reference's crown stands over its own chin, in millimetres.
///
/// The top row of [`REFERENCE`] plus the ten it stops short of; `vault` prints
/// the same quantity for our bodies. It is here so the shelf below can be read
/// at the same ANATOMY on two heads of different size rather than at the same
/// millimetre.
const REFERENCE_FACE: f32 = 260.0;

/// The reference's forward reach at any height below its chin, in millimetres,
/// interpolated between the rows of [`REFERENCE`].
fn reference_front(at: f32) -> f32 {
    let rows = REFERENCE.windows(2).find(|pair| {
        let (high, low) = (pair[0].0 as f32, pair[1].0 as f32);
        at <= high && at >= low
    });
    let Some(pair) = rows else {
        return REFERENCE.last().map_or(0.0, |row| row.3);
    };
    let (high, low) = (pair[0].0 as f32, pair[1].0 as f32);
    let along = (high - at) / (high - low).max(f32::EPSILON);
    pair[0].3 * (1.0 - along) + pair[1].3 * along
}

/// How far the MIDLINE reaches forward at a height, in millimetres, bisected
/// against the surface.
///
/// **Beside [`front`] because a section's forward-most point is not always on
/// the midline** — the jaw's flank beside the chin can stand ahead of the chin
/// itself — the blade a radial planing of the submental leaves behind.
/// The two rows agreeing says the run under the chin is the chin's;
/// the two rows parting says the next question is about the flank and not about
/// a profile. `REFERENCE` is a section maximum, so [`front`] is what may be held
/// against it and this may not.
fn midline(mesh: &PolyMesh, head: symbios_avatar::Vec3, height: f32) -> Option<f32> {
    use symbios_avatar::Vec3;
    let from = Vec3::new(head.x, height, head.z);
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, 0.40f32);
    for _ in 0..32 {
        let middle = 0.5 * (inside + outside);
        if mesh.contains(from + Vec3::Z * middle) {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    Some((head.z + inside) * 1000.0)
}

/// The forward-most point of a plane cut at `height`, in millimetres, or `None`
/// where the section is too thin to be a section.
fn front(mesh: &PolyMesh, height: f32) -> Option<f32> {
    let points: Vec<(f32, f32)> = cut(mesh, height)
        .into_iter()
        .filter(|(x, _)| x.abs() < REACH)
        .collect();
    (points.len() >= 3).then(|| points.iter().fold(f32::MIN, |z, p| z.max(p.1)) * 1000.0)
}

/// How far this body's crown stands over its own chin, in millimetres.
///
/// The same plane-cut scan `vault` runs, factored out because the shelf is read
/// per face length and not per millimetre.
fn face_length(mesh: &PolyMesh, chin: f32) -> f32 {
    (0..=320)
        .filter(|step| front(mesh, chin + *step as f32 / 1000.0).is_some())
        .max()
        .map_or(0.0, |step| step as f32)
}

/// The body this seed builds, optionally with `CHIN`'s push multiplied by
/// something other than what the record's own axis derives.
///
/// **Both bodies go through this**, rather than one through here and one
/// through [`Avatar::build`]: the shelf below is a difference of two numbers
/// and it is only worth reading if exactly one thing differs between the
/// surfaces they come off. `HeadTraits::chin` is the only multiplier on `CHIN`
/// anywhere in the crate, and `FaceParams::on` reads `lips` rather than `chin`,
/// so the params are the record's own on both.
///
/// The mouth cut, the eyes and everything attached are left off — they are
/// after the carve and none of them is on the midline under the chin.
fn built(record: &AvatarRecord, chin: Option<f32>) -> Option<(PolyMesh, Rig)> {
    let skeleton = record.skeleton();
    let mut traits = HeadTraits::of(&record.composites);
    let params = record.face.on(&traits);
    if let Some(chin) = chin {
        traits.chin = chin;
    }
    let config = AvatarConfig::default();
    let mut body = build_body(&skeleton, &config.cage, config.subdivisions, &traits).ok()?;
    let rig = Rig::from_skeleton(&skeleton).ok()?;
    if let Some(skull) = Skull::measure(&body, &rig) {
        let canon = Canon::measure(&rig, &skull, &record.eyes);
        carve_face(&mut body, &rig, &canon, &params);
    }
    Some((body, rig))
}

/// The run under the chin, ours against the reference's, and what `CHIN` owns
/// of it.
///
/// # What this was built to settle
///
/// The drop over the first centimetre below the chin: the
/// reference gives up 9.6 mm there and holds a shelf. Read off
/// `examples/jawprobe`, whose cage row is a constant 0.207–0.225 against a
/// reference "well under that", the cage looks like the culprit.
/// **That comparison is not legal.** jawprobe's shelf
/// is a share over a BONE-relative span and the reference's 0.165 is a share
/// over chin-to-throat; quoting one against the other is the error jawprobe's
/// own docstring exists to forbid. Nobody had measured the cage chin-relatively,
/// because the cage has no chin to measure from.
///
/// So the surface below is the shipped one with `HeadTraits::chin` at ZERO, and
/// the height it is read from is the shipped body's own measured chin. The span
/// is located by a real landmark on a real body and the term under test is not
/// in the surface. It is a decision rather than a computation and this is the
/// least dishonest version of it.
///
/// # Two things it found, and both re-point the issue
///
/// **The cage is innocent.** With `CHIN` zeroed the surface gives up 0.0250,
/// 0.0235, 0.0241 and 0.0272 of its own face length over the first centimetre
/// on seeds 0, 3, 6 and 12, against the reference's 0.0369. It is not steeper
/// than the reference under the jaw; it is barely two thirds as steep. And
/// below −20 it sits on the reference's own throat line to within a few
/// millimetres — 43.5 against 53.6, 41.2 against 44.0, 42.0 against 42.2 on
/// seed 0. The swept-capsule surface is not what is left.
///
/// **The first centimetre was never the defect either — the ruler was.** Read
/// as REACH rather than as a drop, at the same share of each face, ours lands
/// on the reference at −10 on every seed: 89.5, 89.3, 90.8 and 81.4 against
/// 89.0. The drop reads large only because our chin starts a few millimetres
/// prouder, and the absolute-millimetre version of it reads larger still
/// because it takes ten millimetres off a 181 mm face and calls that the same
/// question as ten off a 260 mm one. `column`'s standing caution — compare at
/// equal stature or not at all — was never applied to this row.
///
/// What was left was ONE row, and it was the second centimetre: at −20 we stood
/// 19.9, 18.7, 23.7 and 10.3 mm proud of the reference. The reference spends
/// 35.4 mm of forward reach in that one centimetre and we spent 16. It had a
/// cliff and we had a ramp.
///
/// # And the ramp was the submental CEILING, not `CHIN`'s tail
///
/// Which this instrument also settled, by running with `SKIP_SUBMENTAL` set:
/// seed 0 reads 82.7 mm at −20 unclamped and 73.5 shipped, so the construction
/// is removing 9.2 mm there and the surface is AGAINST its chord. Lowering
/// `CHIN` underneath a ceiling it is already touching moves nothing, which is
/// why the same tail knot moved seed 0's −20 row by 2.7 mm and seed 12's by
/// 22.9. `face::skull::SUBMENTAL_SPEND` gives that chord the reference's own shape, and
/// the row now reads +9.5, +6.9, +3.0 and −5.8.
///
/// # The calibration, said out loud
///
/// Our zero is a measured crest and the reference's is its `head` joint, which
/// sits at its chin. The reference's own front reach falls monotonically
/// from the nose to that row, so its chin cannot be ABOVE it — and every
/// recalibration downward makes the −20 excess larger rather than smaller. The
/// finding is robust to the one thing about this comparison that is not
/// measured.
fn shelf(record: &AvatarRecord, shipped: &PolyMesh, chin: f32, avatar_head: symbios_avatar::Vec3) {
    let Some((ours, _)) = built(record, None) else {
        return;
    };
    let Some((bare, _)) = built(record, Some(0.0)) else {
        return;
    };
    let face = face_length(shipped, chin);
    if face <= 0.0 {
        return;
    }
    // Heights are the reference's own millimetres scaled onto this face, so two
    // heads are cut at the same anatomy rather than at the same millimetre.
    let at = |depth: f32| chin + depth * face / REFERENCE_FACE / 1000.0;

    println!("\nthe run under the chin, at this face's own share of each height");
    println!(
        "  {face:.0} mm chin to crown against the reference's {REFERENCE_FACE:.0}, so ten of \
         its millimetres are {:.1} of ours",
        10.0 * face / REFERENCE_FACE
    );
    println!(
        "\n{:>22}{:>8}{:>8}{:>8}{:>8}{:>8}",
        "forward reach, mm", "0", "-10", "-20", "-30", "-40"
    );
    let depths = [0.0f32, -10.0, -20.0, -30.0, -40.0];
    let mut mine = [None; 5];
    for (name, mesh) in [("ours", &ours), ("ours, CHIN zeroed", &bare)] {
        print!("{name:>22}");
        for (slot, depth) in depths.iter().enumerate() {
            let reach = front(mesh, at(*depth));
            match reach {
                Some(z) => print!("{z:>8.1}"),
                None => print!("{:>8}", "-"),
            }
            if name == "ours" {
                mine[slot] = reach;
            }
        }
        println!();
    }
    print!("{:>22}", "ours, midline only");
    for depth in depths {
        match midline(&ours, avatar_head, at(depth)) {
            Some(z) => print!("{z:>8.1}"),
            None => print!("{:>8}", "-"),
        }
    }
    println!();
    print!("{:>22}", "the reference");
    for depth in depths {
        print!("{:>8.1}", reference_front(depth));
    }
    println!();
    print!("{:>22}", "ours, proud by");
    for (slot, depth) in depths.iter().enumerate() {
        match mine[slot] {
            Some(z) => print!("{:>+8.1}", z - reference_front(*depth)),
            None => print!("{:>8}", "-"),
        }
    }
    println!();

    // And the drop over the first centimetre, which is the figure #94 has been
    // quoting all week — kept so the two rulers can be read against each other,
    // and so the absolute column can be seen to be the one that misleads.
    println!(
        "\n{:>22}  {:>17}  {:>17}",
        "drop over the first cm", "10 mm", "the same share"
    );
    println!(
        "{:>22}  {:>8}{:>9}  {:>8}{:>9}",
        "", "mm", "/face", "mm", "/face"
    );
    let mut spent = Vec::new();
    for (name, mesh) in [("ours", &ours), ("ours, CHIN zeroed", &bare)] {
        let (Some(crest), Some(ten), Some(share)) = (
            front(mesh, chin),
            front(mesh, chin - 0.010),
            front(mesh, at(-10.0)),
        ) else {
            println!("{name:>22}: no section under the chin");
            continue;
        };
        spent.push((crest - share) / face);
        println!(
            "{name:>22}  {:>8.1}{:>9.4}  {:>8.1}{:>9.4}",
            crest - ten,
            (crest - ten) / face,
            crest - share,
            (crest - share) / face
        );
    }
    let theirs = 98.6 - reference_front(-10.0);
    println!(
        "{:>22}  {:>8.1}{:>9.4}  {:>8.1}{:>9.4}",
        "the reference",
        theirs,
        theirs / REFERENCE_FACE,
        theirs,
        theirs / REFERENCE_FACE
    );
    if let (Some(ours), Some(bare)) = (spent.first(), spent.get(1)) {
        println!(
            "  everything but `CHIN` spends {bare:.4} of the face here and `CHIN` adds \
             {:+.4}, against the reference's {:.4}",
            ours - bare,
            theirs / REFERENCE_FACE
        );
    }
}
