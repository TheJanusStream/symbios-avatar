//! Measures a walk cycle against the body walking it, and against the gait
//! literature.
//!
//! `examples/dump -- --walk` writes frames to look at and prints what the gait
//! and the terrain each decided. This asks the questions a walk is judged on
//! that no parameter can answer, because each of them is a property of the
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
//! 4. **Does the pelvis bob?** A real walk vaults over its stance leg twice a
//!    cycle; a pelvis that rides at one height reads as a dolly shot.
//! 5. **Does the foot articulate?** Heel-strike lands toe-up, push-off leaves
//!    toe-down. A sole held flat through the whole cycle is a shuffle.
//!
//! **Clearances are measured against the body's own standing depth, not against
//! `y = 0`.** The build delivers a sole that bulges below its own ground plane
//! (#220), and an instrument that measures from the floor reads that geometry as
//! a gait defect. The baseline is printed so the build defect stays visible; it
//! is just not billed to the walk.
//!
//! The table samples at display resolution (`--frames`); every summary figure is
//! computed from its own fine sweep of the cycle, because a coarse table hides
//! whatever falls between its rows — at 16 frames the swing's lowest pass never
//! appeared at all.
//!
//! Reference bands are approximate normal-adult figures from the gait-analysis
//! literature (Murray's walking-pattern studies, Winter's and Perry's gait
//! texts): minimum toe clearance ~10–30 mm; elbow never straight, ~10–25° at
//! its straightest and ~25–45° at its peak; shoulder-against-hip counter-rotation
//! peaking ~5–15°; pelvis vertical excursion ~25–50 mm peak-to-peak at a natural
//! pace; duty factor ~0.6 with ~0.2 of the cycle on both feet; heel-strike
//! ~15–25° toe-up and push-off ~15–20° toe-down. They are bands to stand beside
//! a number, not targets to fit — this body is stylised, and where it leaves a
//! band on purpose the summary should say so, out loud, every run.
//!
//! ```text
//! cargo run --example walkaudit
//! cargo run --example walkaudit -- --frames 24
//! cargo run --example walkaudit -- --pace 1.4 --grade 0.12
//! cargo run --example walkaudit -- --camber 0.20
//! ```

use glam::Vec3;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Gait, Ground, Limb, Patch, Pose, Rig, Stride, Walk, anim::gait,
};

/// How many points of the cycle the summary statistics are taken from.
///
/// Deliberately not `--frames`: the table is for reading and the statistics are
/// for judging, and a statistic taken at display resolution inherits the
/// display's blind spots.
const SWEEP: usize = 240;

/// What one moment of the walk measures.
struct Moment {
    /// Where in the cycle this is, `0..1`.
    cycle: f32,
    /// Per foot: whether it is swinging, sole height above the body's own
    /// standing depth, its progress through its phase, and its pitch against
    /// the ground plane in degrees (positive toe-up).
    feet: Vec<(bool, f32, f32, f32)>,
    /// Elbow bend per arm, degrees away from straight.
    elbows: Vec<f32>,
    /// Shoulder line against hip line, degrees.
    twist: f32,
    /// Pelvis height about its standing height, in metres.
    pelvis: f32,
    /// Trunk pitch away from its own rest carriage, in degrees, positive
    /// forward.
    lean: f32,
}

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
    // The lateral axis, which is a different question from the grade and was
    // for a long time the only one being asked by accident: `examples/locomotion`
    // tilted its ground along X and called the result a grade for its whole
    // life (fixed under #221). A foot on a side-slope has to ROLL to meet it,
    // where a foot on a grade has to PITCH, and nothing here measured the first
    // one until #250.
    let camber = number("--camber").unwrap_or(0.0);

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

    // The body's own standing depth, per foot: where the sole rests before
    // anything is posed. Every clearance is measured from here — see the module
    // docs and #220 for why the floor itself is the wrong ruler.
    let standing: Vec<f32> = feet
        .iter()
        .map(|(_, patch)| {
            patch
                .vertices()
                .map(|vertex| body.positions[vertex].y)
                .fold(f32::MAX, f32::min)
        })
        .collect();

    // Heel and toe, for pitch: of the joints past the ankle, the rearmost and
    // foremost at rest. A foot with fewer than two has no pitch to measure.
    // Pitch is reported about the line's own REST attitude — the nodes need not
    // lie level in a foot that stands flat, and on this body they run 3 degrees
    // uphill — so 0 means "carried as it stands", exactly like the soles.
    let spans: Vec<Option<(usize, usize, f32)>> = feet
        .iter()
        .map(|(limb, _)| {
            let joints = rig.extremity_joints(*limb);
            let past_ankle = joints.get(1..).unwrap_or(&[]);
            let heel = past_ankle.iter().copied().min_by(|&a, &b| {
                rig.joints[a]
                    .position
                    .z
                    .total_cmp(&rig.joints[b].position.z)
            });
            let toe = past_ankle.iter().copied().max_by(|&a, &b| {
                rig.joints[a]
                    .position
                    .z
                    .total_cmp(&rig.joints[b].position.z)
            });
            match (heel, toe) {
                (Some(heel), Some(toe)) if heel != toe => {
                    let run = rig.joints[toe].position - rig.joints[heel].position;
                    let rest = run
                        .y
                        .atan2((run.x * run.x + run.z * run.z).sqrt())
                        .to_degrees();
                    Some((heel, toe, rest))
                }
                _ => None,
            }
        })
        .collect();

    let pelvis_rest = rig
        .joints
        .iter()
        .position(|joint| joint.parent.is_none())
        .map(|root| (root, rig.joints[root].position.y));

    // The instrument's model of terrain, and now the gait's too: `step` seats
    // its contacts on the same surface the plant settles them onto (#221),
    // which is what stops a swing arc built at the rest ground height from
    // ploughing through a slope it is climbing.
    let height = |at: Vec3| at.z * grade + at.x * camber;
    let floor = |foot: Vec3| {
        Some(Ground {
            position: Vec3::new(foot.x, height(foot), foot.z),
            normal: Vec3::new(-camber, 1.0, -grade).normalize(),
        })
    };
    let measure = |cycle: f32| -> Moment {
        let mut pose = Pose::rest(rig);
        // The whole sequence, through the engine's own entry point (#253) —
        // step, arms, lean, plant, roll, in that order. Hand-rolled here until
        // the fourth stage arrived and the order stopped being something an
        // instrument should be trusted to remember.
        Walk::at(cycle).drive(rig, &mut pose, &gait, &stride, floor);

        let posed = pose.forward(rig);
        let moved = posed.deform(rig, &body.positions, weights);

        let feet = feet
            .iter()
            .zip(&standing)
            .zip(&spans)
            .map(|(((limb, patch), base), span)| {
                let sole = patch
                    .vertices()
                    .map(|vertex| {
                        // Height above the ground beneath that vertex, not above
                        // zero: on a grade the floor is not level and a foot that
                        // tracks it would otherwise read as sinking downhill.
                        let at = moved[vertex];
                        at.y - height(at)
                    })
                    .fold(f32::MAX, f32::min);
                let index = gait.limbs.iter().position(|other| other == limb);
                let phase = index.map_or(gait::Phase::Stance(0.0), |at| gait.phase(at, cycle));
                let pitch = span.map_or(0.0, |(heel, toe, rest)| {
                    let run = posed.positions[toe] - posed.positions[heel];
                    let flat = (run.x * run.x + run.z * run.z).sqrt();
                    // Against the slope the foot walks on, not against level:
                    // a sole lying along a ramp is flat for this question.
                    run.y.atan2(flat).to_degrees() - grade.atan().to_degrees() - rest
                });
                (!phase.is_stance(), sole - base, phase.progress(), pitch)
            })
            .collect();

        let elbows = [Limb::ForeLeft, Limb::ForeRight]
            .into_iter()
            .map(|limb| elbow_bend(rig, &posed, limb))
            .collect();

        Moment {
            cycle,
            feet,
            elbows,
            twist: torso_twist(rig, &posed),
            pelvis: pelvis_rest.map_or(0.0, |(root, rest)| posed.positions[root].y - rest),
            lean: trunk_lean(rig, &posed),
        }
    };

    println!(
        "walking at pace {pace:.2} on a {:.0}% grade and {:.0}% camber, {frames} frames of \
         one cycle",
        grade * 100.0,
        camber * 100.0
    );
    println!(
        "stride {:.3} m long, lifting {:.3} m; sole clearances measured above the body's \
         own standing depth ({:.1} mm under the build's floor — the mesh's, not the walk's, #220)",
        stride.length,
        stride.lift,
        standing
            .iter()
            .map(|base| base * -1000.0)
            .fold(0.0f32, f32::max),
    );
    println!(
        "\n{:>5} {:>6}  {:>25} {:>25}  {:>13}  {:>7} {:>7}",
        "frame", "cycle", "HindLeft sole", "HindRight sole", "elbow L / R", "torso", "pelvis"
    );
    println!(
        "{:>5} {:>6}  {:>25} {:>25}  {:>13}  {:>7} {:>7}",
        "",
        "",
        "phase     mm  pitch deg",
        "phase     mm  pitch deg",
        "degrees bent",
        "degrees",
        "mm"
    );

    for frame in 0..frames {
        let moment = measure(frame as f32 / frames as f32);
        let mut cells = String::new();
        for &(airborne, sole, _, pitch) in &moment.feet {
            cells.push_str(&format!(
                " {:>6} {:>10.1} {:>7.1}",
                if airborne { "swing" } else { "stance" },
                sole * 1000.0,
                pitch,
            ));
        }
        println!(
            "{frame:>5} {:>6.3} {cells}  {:>6.1} {:>6.1}  {:>7.1} {:>7.1}",
            moment.cycle,
            moment.elbows[0],
            moment.elbows[1],
            moment.twist,
            moment.pelvis * 1000.0,
        );
    }

    // The judging pass, at its own resolution.
    let sweep: Vec<Moment> = (0..SWEEP)
        .map(|at| measure(at as f32 / SWEEP as f32))
        .collect();

    let mut lowest_swing = f32::MAX;
    let mut lowest_at = 0.0f32;
    let mut mid_swing = f32::MAX;
    let mut stance_err = 0.0f32;
    let mut straightest = f32::MAX;
    let mut deepest_bend = 0.0f32;
    let mut widest_twist = 0.0f32;
    let (mut lean_low, mut lean_high) = (f32::MAX, f32::MIN);
    let (mut pelvis_low, mut pelvis_high) = (f32::MAX, f32::MIN);
    let (mut stance_pitch, mut swing_pitch) = ((f32::MAX, f32::MIN), (f32::MAX, f32::MIN));
    for moment in &sweep {
        for &(airborne, sole, progress, pitch) in &moment.feet {
            if airborne {
                if sole < lowest_swing {
                    lowest_swing = sole;
                    lowest_at = progress;
                }
                if (0.4..=0.6).contains(&progress) {
                    mid_swing = mid_swing.min(sole);
                }
                swing_pitch = (swing_pitch.0.min(pitch), swing_pitch.1.max(pitch));
            } else {
                stance_err = stance_err.max(sole.abs());
                stance_pitch = (stance_pitch.0.min(pitch), stance_pitch.1.max(pitch));
            }
        }
        let bend = moment.elbows.iter().copied().fold(f32::MAX, f32::min);
        straightest = straightest.min(bend);
        deepest_bend = deepest_bend.max(moment.elbows.iter().copied().fold(0.0, f32::max));
        widest_twist = widest_twist.max(moment.twist.abs());
        lean_low = lean_low.min(moment.lean);
        lean_high = lean_high.max(moment.lean);
        pelvis_low = pelvis_low.min(moment.pelvis);
        pelvis_high = pelvis_high.max(moment.pelvis);
    }
    let both_down = (0..SWEEP)
        .filter(|&at| gait.grounded(at as f32 / SWEEP as f32) >= 2)
        .count() as f32
        / SWEEP as f32;

    println!("\nagainst the reference, over {SWEEP} samples of the cycle:");
    println!(
        "  swing:  lowest pass {:.1} mm above standing depth, at swing phase {:.2}; \
         mid-swing bottoms at {:.1} mm",
        lowest_swing * 1000.0,
        lowest_at,
        mid_swing * 1000.0,
    );
    println!(
        "          (this lift's arc starts and ends AT the ground, so only a negative reading \
         is a scuff; what clears it at a phase end is the roll — a foot pitched onto its heel \
         or its toe carries the rest of its sole up. A real swing's tightest pass is 10-30 mm \
         up, mid-swing, because the toe hangs)"
    );
    println!(
        "  stance: soles held within {:.1} mm of standing depth (a plant holds the sole flat \
         at 0; the roll lifts whatever is not bearing weight, so a foot pitched onto its heel \
         or its toe rests on its rim and reads here as a positive. How flat the sole is to \
         begin with is `examples/footaudit`'s measurement, not this one's — quoting a figure \
         for it here is what left this line citing an 11.7 mm convexity that #220 had already \
         fixed)",
        stance_err * 1000.0
    );
    println!(
        "  elbows: never under {straightest:.1} deg bent, peaking {deepest_bend:.1} \
         (reference: ~10-25 at straightest, ~25-45 at peak)"
    );
    println!(
        "  torso:  shoulders led the hips by up to {widest_twist:.1} deg \
         (reference peak: ~5-15)"
    );
    println!(
        "  lean:   trunk pitched {lean_low:.1} to {lean_high:.1} deg forward of its rest \
         carriage (reference: ~2-7 at a normal pace, more as it rises; a body that walks \
         bolt upright reads as a mannequin being carried, #239)"
    );
    println!(
        "  pelvis: rode {:.1} to {:.1} mm about standing height, {:.1} mm peak-to-peak \
         (reference: ~25-50 mm at a natural pace)",
        pelvis_low * 1000.0,
        pelvis_high * 1000.0,
        (pelvis_high - pelvis_low) * 1000.0,
    );
    println!(
        "  feet:   pitched {:.1} to {:.1} deg in stance, {:.1} to {:.1} in swing \
         (reference: ~15-25 toe-up at heel-strike, ~15-20 toe-down at push-off)",
        stance_pitch.0, stance_pitch.1, swing_pitch.0, swing_pitch.1,
    );
    println!(
        "  gait:   duty {:.2}, both feet down {both_down:.2} of the cycle \
         (reference: ~0.60 / ~0.20)",
        gait.duty,
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
/// How far the trunk is pitched forward of its own rest carriage, in degrees.
///
/// Measured pelvis-to-shoulders on the POSED body and referenced to the same
/// line at rest, for the reason every other column here is: this body is not
/// built standing perfectly upright, so an angle from vertical would report a
/// lean that was never applied. Zero means "carried as it stands".
///
/// Taken from the joint the arms hang off rather than from the neck, because
/// the neck deliberately takes the lean back off again to hold the head level —
/// measuring there would read the head's correction and call it the trunk's
/// posture.
fn trunk_lean(rig: &Rig, posed: &symbios_avatar::anim::Posed) -> f32 {
    let Some(&neck) = rig.in_zone(symbios_avatar::Zone::Neck).first() else {
        return 0.0;
    };
    let Some(girdle) = rig.joints[neck].parent else {
        return 0.0;
    };
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return 0.0;
    };
    let pitch = |run: Vec3| run.z.atan2(run.y).to_degrees();
    pitch(posed.positions[girdle] - posed.positions[root])
        - pitch(rig.joints[girdle].position - rig.joints[root].position)
}

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
