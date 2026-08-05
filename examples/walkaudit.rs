//! Measures a walk cycle: foot clearance, elbow bend, and how the torso turns.
//!
//! `examples/dump -- --walk` writes frames to look at and prints what the gait
//! and the terrain each decided. This asks the three questions a walk is judged
//! on that no parameter can answer, because each of them is a property of the
//! *posed body* rather than of a constant:
//!
//! 1. **Does a swinging foot clear the ground?** Not the ankle — the foot. The
//!    leg IK places a joint, and everything below it rides along in whatever
//!    orientation the rest pose left it, so a foot can sit at a perfectly good
//!    ankle height with its toe through the floor. Measured on the real surface,
//!    selected by [`Patch`], which is the only thing that can find a foot on a
//!    body whose foot is part of its leg.
//! 2. **Is the elbow bent?** Asked as the angle at the joint between the upper
//!    arm and the forearm, so it cannot be satisfied by a rotation that turns
//!    out to be about the arm's own axis and bends nothing.
//! 3. **Does the torso turn with the arms?** Asked as the angle between the
//!    shoulder line and the hip line, measured off the posed skeleton. A body
//!    that swings its arms from rigid shoulders reads as a puppet.
//!
//! ```text
//! cargo run --example walkaudit
//! cargo run --example walkaudit -- --frames 24
//! cargo run --example walkaudit -- --pace 1.4 --grade 0.12
//! ```

use glam::Vec3;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, FootingConfig, Gait, Ground, Limb, Patch, Pose, Rig, Stride,
    anim::gait, anim::plant_feet_of,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let frames = number("--frames").map_or(16, |value| value as usize).max(2);
    let pace = number("--pace").unwrap_or(1.0);
    let grade = number("--grade").unwrap_or(0.0);

    let record = AvatarRecord::new("Walker", Archetype::default());
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the walking body would not build");
        std::process::exit(1);
    };
    let rig = &avatar.rig;
    let body = &avatar.parts.body;
    let weights = &avatar.parts.weights;
    let gait = Gait::natural(rig);
    let stride = Stride::for_body(rig, pace);

    // The feet, as surface rather than as joints. Selected once from the rest
    // body and then carried through every frame, because a patch is a set of
    // vertex indices and deforming a mesh does not renumber it.
    let feet: Vec<(Limb, Patch)> = [Limb::HindLeft, Limb::HindRight]
        .into_iter()
        .map(|limb| {
            (
                limb,
                Patch::held_by(body, weights, &rig.extremity_joints(limb)),
            )
        })
        .collect();

    println!(
        "walking at pace {pace:.2} on a {:.0}% grade, {frames} frames of one cycle",
        grade * 100.0
    );
    println!(
        "stride {:.3} m long, lifting {:.3} m",
        stride.length, stride.lift
    );
    println!(
        "\n{:>5} {:>6}  {:>18} {:>18}  {:>13}  {:>7}",
        "frame", "cycle", "HindLeft sole", "HindRight sole", "elbow L / R", "torso"
    );
    println!(
        "{:>5} {:>6}  {:>18} {:>18}  {:>13}  {:>7}",
        "", "", "phase  mm to floor", "phase  mm to floor", "degrees bent", "degrees"
    );

    let mut worst_dip = 0.0f32;
    let mut least_clearance = f32::MAX;
    let mut straightest = 0.0f32;
    let mut widest_twist = 0.0f32;

    for frame in 0..frames {
        let cycle = frame as f32 / frames as f32;
        let mut pose = Pose::rest(rig);
        let steps = gait::step(rig, &mut pose, &gait, &stride, cycle);
        gait::swing_arms(rig, &mut pose, &gait, cycle);
        let floor = |foot: Vec3| {
            Some(Ground {
                position: Vec3::new(foot.x, foot.z * grade, foot.z),
                normal: Vec3::new(0.0, 1.0, -grade).normalize(),
            })
        };
        plant_feet_of(
            rig,
            &mut pose,
            &steps.stance,
            floor,
            &FootingConfig::default(),
        );

        let posed = pose.forward(rig);
        let moved = posed.deform(rig, &body.positions, weights);

        let mut cells = String::new();
        for (limb, patch) in &feet {
            let sole = patch
                .vertices()
                .map(|vertex| {
                    // Height above the ground beneath that vertex, not above
                    // zero: on a grade the floor is not level and a foot that
                    // tracks it would otherwise read as sinking downhill.
                    let at = moved[vertex];
                    at.y - at.z * grade
                })
                .fold(f32::MAX, f32::min);
            let airborne = steps.swing.contains(limb);
            if airborne {
                least_clearance = least_clearance.min(sole);
            }
            worst_dip = worst_dip.min(sole).min(0.0);
            cells.push_str(&format!(
                " {:>6} {:>11.1}",
                if airborne { "swing" } else { "stance" },
                sole * 1000.0
            ));
        }

        let bend: Vec<f32> = [Limb::ForeLeft, Limb::ForeRight]
            .into_iter()
            .map(|limb| elbow_bend(rig, &posed, limb))
            .collect();
        straightest = straightest.max(bend.iter().copied().fold(f32::MAX, f32::min));

        let twist = torso_twist(rig, &posed);
        widest_twist = widest_twist.max(twist.abs());

        println!(
            "{frame:>5} {cycle:>6.3} {cells}  {:>6.1} {:>6.1}  {twist:>7.1}",
            bend[0], bend[1]
        );
    }

    println!(
        "\nswinging feet cleared the floor by at least {:.1} mm; \
         the lowest any foot reached was {:.1} mm below it",
        least_clearance * 1000.0,
        -worst_dip * 1000.0,
    );
    println!(
        "the straighter elbow was never less than {straightest:.1} degrees bent; \
         the torso turned up to {widest_twist:.1} degrees"
    );
}

/// How far the elbow is bent, in degrees away from straight.
///
/// Asked at the joint rather than of the rotation applied to it. A quaternion
/// about the arm's own axis is a perfectly good rotation that bends nothing, and
/// reading the parameter instead of the geometry cannot tell the two apart.
fn elbow_bend(rig: &Rig, posed: &symbios_avatar::anim::Posed, limb: Limb) -> f32 {
    let Some([shoulder, elbow, wrist]) = rig.limb_chain(limb) else {
        return 0.0;
    };
    let upper = posed.positions[shoulder] - posed.positions[elbow];
    let fore = posed.positions[wrist] - posed.positions[elbow];
    let (upper, fore) = (upper.normalize_or_zero(), fore.normalize_or_zero());
    180.0 - upper.dot(fore).clamp(-1.0, 1.0).acos().to_degrees()
}

/// How far the shoulders are turned against the hips, in degrees.
///
/// Positive is the left shoulder forward. Measured between the two lines rather
/// than from either alone, so a body that turns as one — which is a turn and not
/// a swing — reads as zero.
fn torso_twist(rig: &Rig, posed: &symbios_avatar::anim::Posed) -> f32 {
    let line = |a: Limb, b: Limb| -> Option<Vec3> {
        let at = |limb: Limb| -> Option<Vec3> {
            let chain = rig.limb_chain(limb)?;
            Some(posed.positions[chain[0]])
        };
        let across = at(b)? - at(a)?;
        Some(Vec3::new(across.x, 0.0, across.z).normalize_or_zero())
    };
    let (Some(shoulders), Some(hips)) = (
        line(Limb::ForeLeft, Limb::ForeRight),
        line(Limb::HindLeft, Limb::HindRight),
    ) else {
        return 0.0;
    };
    // Signed about the body's up axis, so a lead reads the same way through the
    // whole cycle instead of folding at the halfway point.
    let angle = shoulders.cross(hips).dot(Vec3::Y);
    angle.atan2(shoulders.dot(hips)).to_degrees()
}
