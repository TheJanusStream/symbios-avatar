//! The column between the shoulders and the jaw, measured axis-free (#125).
//!
//! Every figure here is a bisection against `PolyMesh::contains`, the same
//! primitive `tests/parts.rs` judges a buried feature with.
//!
//! **The depth is the reading to trust and the reaches are the reading that
//! lied.** Back-over-forward measured from `z = 0` reported 2.50 against the
//! reference's 2.43 after the neck was leaned back, which looked like the whole
//! defect closed by one constant and was the probe reading the OFFSET rather
//! than the surface. So the reaches below are printed against the neck node's
//! OWN z, and the depth — front minus back, which no axis enters — is printed
//! beside them.
//!
//! What it is for, beyond looking: the neck's section is swept off its own joint
//! and the arithmetic says the THROAT should not move for it. This is what
//! checks that, and it earned its keep the first time it was run — the throat
//! had come 9 mm forward and the chest 20, because the socket ring a joint hull
//! opens for a limb blended that limb's depth without its offset.
//!
//! **And it says how long the neck reads, broken into who owns that length**
//! (#129). The table at the end runs
//! `tests/plan::the_neck_is_the_length_of_a_neck`'s own arithmetic over that
//! test's own seeds — so its ratio column can be checked against a shipped
//! assertion — and then splits the chin-to-shoulder span four ways. That split
//! is the answer to why three passes of tuning the neck moved it so little: the
//! neck bone owns two to five millimetres of it.
//!
//! ```text
//! cargo run --release --example neckaudit
//! cargo run --release --example neckaudit -- 7
//! ```

use symbios_avatar::face::Skull;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3, Zone};

/// The seeds `tests/plan::the_neck_is_the_length_of_a_neck` asserts over.
///
/// The table below reproduces that test's own arithmetic, so these have to be
/// its seeds and not a nicer set: an instrument that cannot be checked against
/// a shipped assertion is the one this project keeps being lied to by.
const GUARDED: [i64; 5] = [0, 3, 7, 13, 21];

fn main() {
    let seed: Option<i64> = std::env::args().nth(1).and_then(|arg| arg.parse().ok());
    let mut record = AvatarRecord::new("Necked", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let avatar = Avatar::build(&record).expect("a biped builds");
    let mesh = &avatar.parts.body;
    let rig = &avatar.rig;

    let head = *rig.in_zone(Zone::Head).first().expect("a head");
    let neck = rig.joints[head].parent.expect("a head sits on a neck");
    let joint = rig.joints[head].position;
    let axis = rig.joints[neck].position;

    // Outward from a point known to be inside, in metres.
    let reach = |from: Vec3, along: Vec3| -> Option<f32> {
        if !mesh.contains(from) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, 0.4f32);
        if mesh.contains(from + along * far) {
            return None;
        }
        for _ in 0..40 {
            let middle = (near + far) * 0.5;
            if mesh.contains(from + along * middle) {
                near = middle;
            } else {
                far = middle;
            }
        }
        Some(near)
    };

    println!(
        "seed {}   head joint y {:.1} mm   neck joint y {:.1} mm, z {:.1} mm",
        seed.map_or("default".to_string(), |seed| seed.to_string()),
        joint.y * 1000.0,
        axis.y * 1000.0,
        axis.z * 1000.0,
    );
    println!("  y below the head joint, and everything in millimetres");
    println!("      y    half-width     ahead    behind     DEPTH");

    let mut widths: Vec<f32> = Vec::new();
    let mut step = 0;
    while step <= 44 {
        let y = joint.y - 0.005 * step as f32;
        let from = Vec3::new(0.0, y, axis.z);
        let (Some(ahead), Some(behind)) = (reach(from, Vec3::Z), reach(from, -Vec3::Z)) else {
            step += 1;
            continue;
        };
        // **The width is a maximum over the slice, not a ray from the axis, and
        // the difference is the trap this issue has already paid for once.** A
        // section swept off-centre is still an ellipse; a ray fired sideways
        // from the joint crosses it on a CHORD, so it reads narrower the further
        // the mass moves, and reports a column pinching that is not.
        let across = (0..=20)
            .filter_map(|slice| {
                let z = axis.z - behind + (ahead + behind) * slice as f32 / 20.0;
                reach(Vec3::new(0.0, y, z), Vec3::X)
            })
            .fold(0.0f32, f32::max);
        widths.push(across);
        println!(
            "  {:6.1}   {:9.1}   {:7.1}   {:7.1}   {:7.1}",
            (y - joint.y) * 1000.0,
            across * 1000.0,
            ahead * 1000.0,
            behind * 1000.0,
            (ahead + behind) * 1000.0,
        );
        step += 1;
    }

    // Swell, pinch, swell is what the reference does not do: its half-width
    // grows all the way out into the shoulder, monotonically. A turn is a
    // reversal in the width, and it is counted with a DEADBAND because the
    // surface ripples a few tenths of a millimetre between rows and a turn under
    // half a millimetre over five of height is not a swell — it is #47.
    const DEADBAND: f32 = 0.0005;
    let mut turns = 0;
    let mut rising: Option<bool> = None;
    let mut mark = widths[0];
    for &width in &widths[1..] {
        if (width - mark).abs() < DEADBAND {
            continue;
        }
        let now = width > mark;
        if rising.is_some_and(|was| was != now) {
            turns += 1;
        }
        rising = Some(now);
        mark = width;
    }
    println!(
        "  turns down the column, past a {:.1} mm deadband: {turns}",
        DEADBAND * 1000.0
    );

    visible_neck();
}

/// How much neck an eye actually sees: the chin down to the shoulder line.
///
/// This is `tests/plan::the_neck_is_the_length_of_a_neck`'s arithmetic, run for
/// reading rather than for asserting — the same bisected half-width, the same
/// "half again as wide as the narrowest point" rule, the same five seeds. It is
/// duplicated rather than shared because the test is the contract and this is
/// an instrument: the day they disagree, the instrument is the one that is
/// wrong, and that is only checkable if the numbers can be held side by side.
///
/// The reference figures it is read against, both measured in #125: the
/// Quaternius pair land on 0.13 and 0.14, and the eight-head canon this crate
/// quotes elsewhere puts the shoulder line about a third of a head under the
/// chin, so 0.33.
///
/// **The span is broken into its four owners, and that is the reading that
/// matters** (#129). Three passes have tried to shorten this by tuning the neck
/// — #93 the girdle's socket floor, #107 a re-sweep for the eight-point cage,
/// #125 the neck's backward lean — and the columns below say why so little
/// moved: the neck BONE's own contribution is the few millimetres by which its
/// length exceeds the girdle's radius, because the girdle's crown sits directly
/// under the neck joint by construction. Most of what an eye reads as neck is
/// head-owned surface hanging below the chin, and the rest is the girdle.
fn visible_neck() {
    println!();
    println!("visible neck, chin to the shoulder line, by the guard's own rule");
    println!("  seed      head       neck     ratio   (canon 0.33, reference 0.13)");
    println!("           and who owns the span: chin to the head's own floor,");
    println!("           that floor to the neck joint, the neck joint to the");
    println!("           girdle's crown, and the crown down to the shoulder line");

    for seed in GUARDED {
        let mut record = AvatarRecord::new("Necked", Archetype::default());
        record.reroll(seed);
        let avatar = Avatar::build(&record).expect("a biped builds");
        let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
        let Some(skull) = Skull::measure(mesh, rig) else {
            println!("  {seed:5}   no skull measured");
            continue;
        };
        let at = rig.joints[*rig.in_zone(Zone::Head).first().expect("a head")].position;
        let (throat, crown) = skull.throat_and_crown();
        let (chin, crown, throat) = (at.y + skull.chin(), at.y + crown, at.y + throat);

        // Bisected against the surface, never binned: binning vertices into
        // height bands reports ripple that is not in the mesh.
        let half_width = |y: f32| {
            let (mut inside, mut outside) = (0.0f32, 0.40f32);
            for _ in 0..32 {
                let middle = 0.5 * (inside + outside);
                if mesh.contains(Vec3::new(at.x + middle, y, at.z)) {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            inside
        };

        let (mut narrowest, mut y) = (f32::MAX, throat);
        while y > throat - 0.30 {
            narrowest = narrowest.min(half_width(y));
            if half_width(y) > narrowest * 1.5 {
                break;
            }
            y -= 0.001;
        }

        // The four owners of the span, top to bottom. `throat` is where the
        // head's own SURFACE stops, so the first is head-owned and the rest is
        // the body's; the girdle's crown is its joint plus its radius, since a
        // node's section scales its width and its depth and never its height.
        let neck_joint = rig.joints[rig.joints[*rig.in_zone(Zone::Head).first().expect("a head")]
            .parent
            .expect("a head sits on a neck")];
        let girdle = rig.joints[neck_joint.parent.expect("a neck sits on a girdle")];
        let crown_of_girdle = girdle.position.y + girdle.radius;

        println!(
            "  {seed:5}   {:6.1} mm  {:6.1} mm   {:.3}    {:5.1} + {:5.1} + {:5.1} + {:5.1}",
            (crown - chin) * 1000.0,
            (chin - y) * 1000.0,
            (chin - y) / (crown - chin),
            (chin - throat) * 1000.0,
            (throat - neck_joint.position.y) * 1000.0,
            (neck_joint.position.y - crown_of_girdle) * 1000.0,
            (crown_of_girdle - y) * 1000.0,
        );
    }
}
