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
//! ```text
//! cargo run --release --example neckaudit
//! cargo run --release --example neckaudit -- 7
//! ```

use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3, Zone};

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
}
