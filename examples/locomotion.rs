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
use symbios_avatar::anim::{FootingConfig, Ground, contacts_during, gait, plant_feet_of};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, ClipLibrary, Gait, Limb, Pose, Rig, Stride};

/// Where the baked artifact sits.
const ARTIFACT: &str = "assets/clips.bin";

/// How many points around one cycle each source is sampled at.
const PHASES: usize = 24;

/// Bodies to try, as record seeds.
///
/// Zero is the default body. The rest are re-rolls, which is what a viewer does
/// with the space bar and is the case the prediction is about.
const SEEDS: [i64; 6] = [0, 3, 7, 19, 42, 101];

/// Grades to try, as a rise over run.
const SLOPES: [f32; 3] = [0.0, 0.15, 0.3];

fn main() {
    let Ok(bytes) = std::fs::read(ARTIFACT) else {
        eprintln!("{ARTIFACT} is not there; run examples/bakeclips first");
        std::process::exit(1);
    };
    let library = ClipLibrary::read(&bytes).expect("the artifact parses");
    let walk = library.get("Walk").expect("Walk is in the shipped set");

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
                let steps = gait::step(rig, &mut pose, &gait, &stride, cycle);
                gait::swing_arms(rig, &mut pose, &gait, cycle);
                (pose, steps.stance)
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
            let gap = imported.sink - procedural.sink;
            if gap > worst_gap.0 {
                worst_gap = (gap, seed, grade);
            }
            cells.push((procedural, imported));
        }

        print!("{seed:>6}{leg:>8.0}{:>6.0}%  ", 100.0 * leg / REFERENCE_LEG);
        for (procedural, imported) in &cells {
            print!(
                "{:>8.0}{:>9.0}",
                procedural.sink * 1000.0,
                imported.sink * 1000.0
            );
        }
        println!();
        print!("{:>21}  ", "lift");
        for (procedural, imported) in &cells {
            print!(
                "{:>8.0}{:>9.0}",
                procedural.lift * 1000.0,
                imported.lift * 1000.0
            );
        }
        println!();
    }

    println!("{}", "-".repeat(74));
    println!(
        "The upper row of each pair is how far a foot went UNDER the floor before any\n\
         solve, which is the defect. The lower is how far the footing solve then had to\n\
         move a joint to fix it. Both in millimetres."
    );
    println!(
        "worst the clip sank below the gait: {:.0} mm, on seed {} at grade {:.2}",
        worst_gap.0 * 1000.0,
        worst_gap.1,
        worst_gap.2
    );
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
    /// Worst distance any sole ended up BELOW the ground, before the solve.
    ///
    /// Unambiguous and source-neutral: no foot should ever be under the floor,
    /// whatever it is doing. A swinging foot may legitimately be high, so the
    /// other direction is not a defect and is not measured.
    sink: f32,
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
    let ground = |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, foot.x * grade, foot.z)));
    let mut reading = Reading {
        sink: 0.0,
        lift: 0.0,
    };
    for step in 0..PHASES {
        let cycle = step as f32 / PHASES as f32;
        let (mut pose, stance) = source(rig, cycle);

        // Penetration, measured on every foot the body has rather than only on
        // the ones it is standing on: a swinging foot ploughing through the
        // floor is the defect this is looking for.
        let posed = pose.forward(rig);
        for limb in rig.ground_contacts() {
            for &joint in &rig.extremity_joints(limb) {
                let at = posed.positions[joint];
                reading.sink = reading.sink.max(at.x * grade - at.y);
            }
        }

        if stance.is_empty() {
            continue;
        }
        let before = pose.forward(rig).positions;
        plant_feet_of(rig, &mut pose, &stance, ground, &FootingConfig::default());
        let after = pose.forward(rig).positions;
        for &limb in &stance {
            for &joint in &rig.extremity_joints(limb) {
                reading.lift = reading.lift.max(before[joint].distance(after[joint]));
            }
        }
    }
    reading
}
