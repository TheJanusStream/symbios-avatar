//! The imported walk against the procedural one, as a number.
//!
//! ```text
//! cargo run --release --example locomotion
//! ```
//!
//! # What this is for
//!
//! #141 asks whether the imported `Walk` replaces [`anim::gait`] or whether
//! procedural locomotion stays and clips cover only the expressive set. That is
//! a judgement made by watching — the viewer is where it gets made — but a
//! judgement made only by watching is one nobody can check, and this is the half
//! that can be.
//!
//! # What it measures, and why that and not something else
//!
//! **How far the footing solve has to move the feet.** A pose whose feet already
//! land where the ground is needs no correction; one whose do not is being held
//! together by [`plant_feet_of`], and the size of that correction is the
//! difference between the two sources stated without an opinion in it.
//!
//! The prediction it exists to test, made on #141 before any of it was written:
//! the retargeter matches bone DIRECTIONS and is deliberately scale-free (#139),
//! so an imported clip fixes joint ANGLES. The procedural gait derives its
//! stride from the body it is on, through [`Stride::for_body`]. So the imported
//! walk should need more correction as a body's legs move away from the
//! reference's, and the procedural one should not — and a slope should widen the
//! gap, because a clip's ankle angle is baked and a slope changes what it should
//! be.
//!
//! Both sources are driven from the same phase and solved by the same footing
//! configuration on the same ground, because a comparison in which anything else
//! differs is not one.

use glam::Vec3;
use symbios_avatar::anim::{Continuity, FootingConfig, Ground, contacts_during, plant_feet_of};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, ClipLibrary, Gait, Limb, Pose, Rig, Stride, Walk,
};

/// Where the baked artifact sits.
const ARTIFACT: &str = "assets/clips.bin";

/// What the imported set does to a body between its own frames.
///
/// **Printed first and by this instrument on purpose** (#249). Everything below
/// compares a clip against the procedural gait, and a comparison against a
/// reference nobody has stated the flaws of is a comparison that treats the
/// reference as a gold standard. It is not one: the owner reported at #237 that
/// the clips do not loop cleanly and that on some of them the body teleports
/// between frames, and epic #237 removes them from the runtime for exactly that
/// kind of reason. So the caveat is a number every future comparison inherits
/// rather than a warning somebody has to remember.
///
/// Two readings, one pass — see [`Continuity`]. Both are ratios to the clip's
/// own median frame-to-frame travel, because an absolute distance says nothing
/// across clips that move at wildly different speeds, and **one is what wants
/// watching in both directions**: a wrap far above the family jerks, and one
/// far below it pauses.
fn continuity(library: &ClipLibrary) {
    let reference = Avatar::build(&AvatarRecord::new("Reference", Archetype::default()))
        .expect("the default record builds");
    println!("What the imported set does between its own frames, on the default body.");
    println!("Both ratios are against each clip's OWN median step, so they compare across");
    println!("clips that move at very different speeds. One is in family; far from one,");
    println!("either way, is not.");
    println!();
    println!(
        "{:>16}{:>8}{:>9}{:>9}{:>8}{:>10}{:>9}{:>8}",
        "clip", "frames", "step mm", "jump mm", "jump x", "at frame", "seam mm", "seam x"
    );
    println!("{}", "-".repeat(77));
    for clip in &library.clips {
        let read: Continuity = clip.continuity(&reference.rig);
        println!(
            "{:>16}{:>8}{:>9.1}{:>9.1}{:>8.1}{:>10}{:>9}{:>8}",
            clip.name,
            clip.frames,
            read.step * 1000.0,
            read.jump * 1000.0,
            read.jump_ratio(),
            format!("{}/{}", read.jump_at, clip.frames),
            read.seam
                .map_or_else(|| "-".to_string(), |seam| format!("{:.1}", seam * 1000.0)),
            read.seam_ratio()
                .map_or_else(|| "-".to_string(), |ratio| format!("{ratio:.1}")),
        );
    }
    println!();
}

/// How many points around one cycle each source is sampled at.
const PHASES: usize = 24;

/// Bodies to try, as record seeds.
///
/// Zero is the default body. The rest are re-rolls, which is what a viewer does
/// with the space bar and is the case the prediction is about.
const SEEDS: [i64; 6] = [0, 3, 7, 19, 42, 101];

/// Grades to try, as a rise over run.
///
/// **Along Z, the way the body walks.** This instrument tilted the ground along
/// X for its whole life, and X is the body's lateral axis: the engine's forward
/// is `+Z` and [`Stride::for_body`] strides down it, so a stride never crossed
/// the tilt at all and every "grade" column was measuring a CAMBER — a hill the
/// body stood across and never climbed. It is why terrain-aware swing targets
/// (#221) changed the walk on `examples/walkaudit`, which tilts along Z, and
/// changed nothing whatever here.
const SLOPES: [f32; 3] = [0.0, 0.15, 0.3];

fn main() {
    let Ok(bytes) = std::fs::read(ARTIFACT) else {
        eprintln!("{ARTIFACT} is not there; run examples/bakeclips first");
        std::process::exit(1);
    };
    let library = ClipLibrary::read(&bytes).expect("the artifact parses");
    let walk = library.get("Walk").expect("Walk is in the shipped set");

    continuity(&library);

    println!("The imported Walk against the procedural gait, on the same bodies.");
    println!("Each figure is the worst distance the footing solve had to move a sole,");
    println!("over {PHASES} points of one cycle, in millimetres.");
    println!();
    println!(
        "{:>6}{:>8}{:>7}  {:>17}{:>17}{:>17}",
        "seed", "leg mm", "vs ref", "grade 0.00", "grade 0.15", "grade 0.30"
    );
    println!(
        "{:>21}  {:>8}{:>9}{:>8}{:>9}{:>8}{:>9}",
        "", "gait", "clip", "gait", "clip", "gait", "clip"
    );
    println!("{}", "-".repeat(74));

    // The reference the clips were authored on, for the "vs ref" column: its
    // hip-to-sole is 932.1 mm, measured at #139 off the CC0 rig itself.
    const REFERENCE_LEG: f32 = 932.1;

    let mut worst_gap = (0.0f32, 0i64, 0.0f32);
    for seed in SEEDS {
        let mut record = AvatarRecord::new("Compared", Archetype::default());
        if seed != 0 {
            record.reroll(seed);
        }
        let Some(avatar) = Avatar::build(&record) else {
            eprintln!("seed {seed} did not build");
            continue;
        };
        let rig = &avatar.rig;
        let leg = leg_length(rig) * 1000.0;

        let mut cells = Vec::new();
        for grade in SLOPES {
            let procedural = read(rig, grade, |rig, cycle| {
                let mut pose = Pose::rest(rig);
                let gait = Gait::natural(rig);
                let stride = Stride::for_body(rig, 1.0);
                // **The whole sequence through the engine's entry point**
                // (#253), with the footing switched OFF — because settling the
                // feet is the correction this instrument exists to measure, and
                // `read` applies it either side of its own reading. This is the
                // ablation the switch was put there for.
                //
                // Hand-rolled until now, and it is why the gait column was
                // unfair for as long as it was: `roll_feet` was simply missing
                // from it, so this measured a walk with no ankles while
                // `examples/walkaudit` measured one with them (#238). Two
                // instruments, two different gaits, one of them the gait.
                //
                // The stride is told about the slope, exactly as the solve is —
                // withholding it here and offering it there was the asymmetry
                // that put the swing arc through the hill (#221).
                let unsettled = Walk {
                    footing: None,
                    ..Walk::at(cycle)
                };
                let walked = unsettled.drive(rig, &mut pose, &gait, &stride, |foot| {
                    Some(Ground::level(Vec3::new(foot.x, foot.z * grade, foot.z)))
                });
                (pose, walked.steps.stance)
            });
            let imported = read(rig, grade, |rig, cycle| {
                let mut pose = walk.pose(rig, cycle * walk.duration());
                // In place, exactly as the viewer plays it: a clip that carries
                // its root forward and the gait that stays put are not otherwise
                // comparable. The vertical bob is kept.
                pose.translation.x = 0.0;
                pose.translation.z = 0.0;
                // Stance taken from the TRAVELLING clip, then applied to the
                // in-place pose. A foot planted on the ground is stationary in
                // the world; in place it slides backwards at walking pace, so
                // nothing about the in-place pose can tell a plant from a skate.
                let stance = contacts_during(rig, walk, cycle * walk.duration());
                (pose, stance)
            });
            let gap = imported.after - procedural.after;
            if gap > worst_gap.0 {
                worst_gap = (gap, seed, grade);
            }
            cells.push((procedural, imported));
        }

        print!("{seed:>6}{leg:>8.0}{:>6.0}%  ", 100.0 * leg / REFERENCE_LEG);
        for (procedural, imported) in &cells {
            print!(
                "{:>8.0}{:>9.0}",
                procedural.after * 1000.0,
                imported.after * 1000.0
            );
        }
        println!();
        for (label, pick) in [
            ("pre-solve", (|r: &Reading| r.before) as fn(&Reading) -> f32),
            ("lift", |r: &Reading| r.lift),
        ] {
            print!("{label:>21}  ");
            for (procedural, imported) in &cells {
                print!(
                    "{:>8.0}{:>9.0}",
                    pick(procedural) * 1000.0,
                    pick(imported) * 1000.0
                );
            }
            println!();
        }
    }

    println!("{}", "-".repeat(74));
    println!(
        "Top row: how far a foot is under the floor AFTER the footing solve — what a\n\
         viewer sees, and the only row that asks both sources the same question.\n\
         'pre-solve' is what the source handed the solve; a gait is two-staged on\n\
         purpose, so a large figure there is its design and not its defect. 'lift' is\n\
         how far the solve then had to move a joint. All in millimetres, measured at the\n\
         extremity joints — which sit inside the foot, so every column understates the\n\
         sole by the stand-off, equally for both. walkaudit reads the mesh."
    );
    println!(
        "worst the clip sank below the gait: {:.0} mm, on seed {} at grade {:.2}",
        worst_gap.0 * 1000.0,
        worst_gap.1,
        worst_gap.2
    );

    let reference = Avatar::build(&AvatarRecord::new("Reference", Archetype::default()))
        .expect("the default body builds");
    report_clip_defects(&library, &reference.rig);
}

/// What the reference clips are, as numbers, so a walk that loses to a
/// defective reference is not billed as losing (#238).
///
/// The owner reported two faults by eye on 2026-08-14: the clips do not loop
/// cleanly, and some teleport the body as if a reference frame differed between
/// frames. Both are measurable, and both are measured **relative to each clip's
/// own typical frame step** rather than as an absolute distance — a fast clip
/// moves a long way every frame, so an absolute threshold would call `Sprint`
/// broken and a `Head Nod` perfect no matter what either did.
///
/// * **seam** is the size of the wrap from last frame back to first, as a
///   multiple of the median step. A clean loop wraps like any other frame and
///   scores about 1. Reported only for clips that claim to loop: a one-shot is
///   not supposed to return to its start and it is no defect that it does not.
/// * **jump** is the largest single-frame root move as a multiple of the median
///   root move. Steady travel scores about 1; a reference frame changing under
///   the clip shows up as one enormous step among ordinary ones.
/// * **root seam** is how far the root has to travel to get back to where it
///   started, in the same units. A clip that stays put scores 0; one that
///   carries the body forward scores its whole journey, and pays for it in one
///   frame every loop.
fn report_clip_defects(library: &ClipLibrary, rig: &Rig) {
    println!();
    println!("The reference clips themselves, measured — they are NOT ground truth.");
    println!("Both figures are multiples of that clip's own median frame step, so a");
    println!("clean clip scores about 1 whatever its speed. '-' is not applicable.");
    println!();
    println!(
        "{:>18}{:>8}{:>9}{:>9}{:>9}{:>9}{:>11}",
        "clip", "frames", "loops", "seam", "jump", "jump mm", "root seam"
    );
    println!("{}", "-".repeat(73));

    let median = |mut values: Vec<f32>| -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        values.sort_by(f32::total_cmp);
        values[values.len() / 2]
    };

    for clip in &library.clips {
        if clip.frames < 3 || clip.rate <= 0.0 {
            continue;
        }
        // One pose per authored frame, sampled at the frame times themselves so
        // nothing here is reading the interpolator instead of the data.
        let poses: Vec<Pose> = (0..clip.frames)
            .map(|frame| clip.pose(rig, frame as f32 / clip.rate))
            .collect();
        let apart = |a: &Pose, b: &Pose| {
            a.rotations
                .iter()
                .zip(&b.rotations)
                .map(|(a, b)| 2.0 * a.dot(*b).abs().clamp(-1.0, 1.0).acos())
                .fold(0.0f32, f32::max)
        };
        let steps: Vec<f32> = poses
            .windows(2)
            .map(|pair| apart(&pair[0], &pair[1]))
            .collect();
        let typical = median(steps.clone());
        let seam = if clip.looping && typical > 1e-6 {
            format!(
                "{:.1}x",
                apart(&poses[clip.frames - 1], &poses[0]) / typical
            )
        } else {
            "-".to_owned()
        };

        let root_steps: Vec<f32> = clip
            .root
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .collect();
        let root_typical = median(root_steps.clone());
        // **A ratio alone can raise a false alarm and this one nearly did.**
        // Thirty-five times a median of nothing is still nothing, so the
        // absolute travel is printed beside it and the two are read together: a
        // defect is a large multiple that is ALSO a distance a viewer would
        // see.
        let worst_root = root_steps.iter().copied().fold(0.0f32, f32::max);
        let jump = if root_typical > 1e-6 {
            format!("{:.1}x", worst_root / root_typical)
        } else {
            "-".to_owned()
        };
        let jump_mm = if root_steps.is_empty() {
            "-".to_owned()
        } else {
            format!("{:.0}", worst_root * 1000.0)
        };

        // The root's own wrap, which is a different question from the pose's.
        // Every looping clip above seams cleanly in its ROTATIONS, and the
        // owner still saw loops break — because a clip that travels has to get
        // back to where it started, and doing that in one frame is a stride's
        // worth of snap. #141 predicted exactly this before any of it was
        // built; this is the column that says how big it is.
        let root_seam = match (clip.looping, clip.root.first(), clip.root.last()) {
            (true, Some(first), Some(last)) if root_typical > 1e-6 => {
                format!("{:.1}x", first.distance(*last) / root_typical)
            }
            _ => "-".to_owned(),
        };

        println!(
            "{:>18}{:>8}{:>9}{:>9}{:>9}{:>9}{:>11}",
            clip.name,
            clip.frames,
            if clip.looping { "yes" } else { "no" },
            seam,
            jump,
            jump_mm,
            root_seam
        );
    }
}

/// Hip to sole at rest, in metres — how long this body's leg is.
fn leg_length(rig: &Rig) -> f32 {
    let hip = rig
        .in_zone(symbios_avatar::Zone::UpperLimb(Limb::HindLeft))
        .first()
        .map_or(0.0, |&joint| rig.joints[joint].position.y);
    let sole = rig
        .extremity_joints(Limb::HindLeft)
        .iter()
        .fold(f32::MAX, |low, &joint| {
            low.min(rig.joints[joint].position.y)
        });
    hip - sole
}

/// What one source does to one body on one grade, in metres.
struct Reading {
    /// Worst distance any foot ended up below the ground **after** the footing
    /// solve — what a viewer actually sees.
    ///
    /// **This is the headline, and it used to be [`Self::before`], which asked
    /// the two sources different questions** (#238). A clip is a finished pose:
    /// what it delivers is what is drawn. A procedural gait is deliberately two
    /// staged — [`gait::step`] places contacts in BODY space and
    /// [`plant_feet_of`] settles them onto the real ground — so its pre-solve
    /// pose is an intermediate that was never promised to clear anything, and
    /// scoring it as though it were final is scoring the gait on a contract it
    /// does not have. The stage both sources genuinely share is the one after
    /// the solve.
    ///
    /// A swinging foot is still measured here, and deliberately: the solve
    /// plants only stance feet, so a swing arc ploughing through a slope
    /// survives to this reading, which is exactly the defect #221 names.
    after: f32,
    /// The same distance measured before the solve.
    ///
    /// Kept as a diagnostic rather than deleted: it is what the gait hands the
    /// solve, and a number that grows here while [`Self::after`] holds is the
    /// solve working harder for the same result.
    before: f32,
    /// Worst distance the footing solve had to move any one joint of a contact.
    ///
    /// **Per joint, each compared against itself.** The first version of this
    /// took the LOWEST joint of each foot before and after and measured between
    /// them — but which joint is lowest changes when an ankle turns, so it was
    /// comparing a heel against a toe and reporting a quarter of a metre of
    /// correction on flat ground where #139 had measured four millimetres. An
    /// argmin whose identity moves is not a measurement.
    lift: f32,
}

/// Runs one source over a cycle and reports what it did.
///
/// `source` is asked for a pose and for the feet carrying the body at that
/// phase. A gait knows its own stance; a clip does not and is asked of the pose
/// — a difference in what the sources *can say*, not in how they are treated.
fn read(rig: &Rig, grade: f32, source: impl Fn(&Rig, f32) -> (Pose, Vec<Limb>)) -> Reading {
    let ground = |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, foot.z * grade, foot.z)));
    let mut reading = Reading {
        after: 0.0,
        before: 0.0,
        lift: 0.0,
    };
    // Penetration of every foot the body has, not only the planted ones: a
    // swinging foot through the floor is the defect this is looking for.
    //
    // Measured at the extremity JOINTS, which sit inside the foot rather than
    // on its sole, so both columns understate the true sole clearance by the
    // stand-off — equally for both sources, which is what keeps the comparison
    // fair. `examples/walkaudit` is the instrument that reads the mesh.
    let deepest = |pose: &Pose| {
        let posed = pose.forward(rig);
        let mut worst = 0.0f32;
        for limb in rig.ground_contacts() {
            for &joint in &rig.extremity_joints(limb) {
                let at = posed.positions[joint];
                worst = worst.max(at.z * grade - at.y);
            }
        }
        worst
    };
    for step in 0..PHASES {
        let cycle = step as f32 / PHASES as f32;
        let (mut pose, stance) = source(rig, cycle);
        reading.before = reading.before.max(deepest(&pose));

        if stance.is_empty() {
            // Nothing to settle, so the pose as delivered IS the final one.
            reading.after = reading.after.max(deepest(&pose));
            continue;
        }
        let before = pose.forward(rig).positions;
        plant_feet_of(rig, &mut pose, &stance, ground, &FootingConfig::default());
        let after = pose.forward(rig).positions;
        reading.after = reading.after.max(deepest(&pose));
        for &limb in &stance {
            for &joint in &rig.extremity_joints(limb) {
                reading.lift = reading.lift.max(before[joint].distance(after[joint]));
            }
        }
    }
    reading
}
