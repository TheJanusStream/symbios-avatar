//! Whether a goal-space gesture reads the same on every body (#248).
//!
//! A baked clip is a set of joint angles and means whatever those angles mean on
//! the skeleton it was baked from. A gesture in [`symbios_avatar::anim::gesture`]
//! is a set of goals in normalised body space, and the claim that buys is that
//! it means the same thing on a body that did not exist when it was written.
//!
//! **That claim is a measurement, and this is it.** Every reading below is taken
//! on a sweep of bodies from 1.2 to 2.2 m — the plan's whole stature range — and
//! reported as a fraction of each body's own size. What matters is not any one
//! figure but the SPREAD of it across the sweep: a gesture that puts the hand
//! 0.31 body-heights above the shoulder on every body is body-agnostic, and one
//! whose figure drifts with stature is normalised in name only.
//!
//! # What is measured
//!
//! 1. **Strain.** Every limb whose goal was out of reach, at any moment, on any
//!    body. `Clip::apply` already reports it; a gesture that strains is asking
//!    for a place the arm cannot get to, and on the small bodies of the sweep
//!    that is easy to do by accident.
//!
//! 2. **Reach.** How far the gesture actually moves the hand, as a fraction of
//!    the body's height — up, forward and across. This is the gesture's own
//!    signature and the thing the spread is asked about.
//!
//! 3. **Clearance.** How close the hand comes to the head and to the trunk's
//!    axis. A raised hand belongs beside the head, and the arithmetic that puts
//!    it there is exactly the arithmetic that can put it through the ear.
//!
//! 4. **Return.** How far from rest the body is at the first and last frame. A
//!    gesture that ends somewhere else leaves the body in a pose it never chose.
//!
//! 5. **Step.** The furthest any joint moves between two adjacent samples, which
//!    catches a key that jumps.
//!
//! **What no number here settles is whether it reads as the gesture.** For that:
//!
//! ```text
//! cargo run --release -F builtin-clips --example viewer -- --gesture Greeting
//! ```
//!
//! ```text
//! cargo run --example gestureaudit
//! cargo run --example gestureaudit -- --gesture Reject
//! cargo run --example gestureaudit -- --samples 480
//! ```

use glam::Vec3;
use symbios_avatar::{
    Limb, Pose, Rig, Zone,
    anim::gesture::{self, ROSTER},
    plan::{BodyPlan, Composites, HumanoidParams},
};

/// How many points of a gesture every reading is taken from.
const SWEEP: usize = 120;

/// The bodies the roster is judged on: a stature in metres, and a limb length
/// against the torso.
///
/// **Stature alone is not a test of this format.** The plan scales a body
/// uniformly with height, so every length in it stays in the same proportion
/// and a goal measured in reaches lands in the same place by arithmetic rather
/// than by design — the spread reads a flat zero and proves nothing. What can
/// move a normalised goal is a change of PROPORTION: `limb_length` raises the
/// pelvis and shortens the torso at a fixed stature, so it moves the shoulder,
/// the reach and the hand's rest position against each other. The sweep runs
/// both, and the extremes of the second are where a gesture actually breaks.
const BODIES: [(f32, f32); 7] = [
    (1.2, 0.0),
    (1.5, 0.0),
    (1.7, 0.0),
    (2.0, 0.0),
    (2.2, 0.0),
    (1.7, -1.0),
    (1.7, 1.0),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<usize>().ok())
    };
    let word = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let samples = number("--samples").unwrap_or(SWEEP).max(4);
    let wanted = word("--gesture");
    let roster: Vec<&str> = match &wanted {
        Some(name) => vec![name.as_str()],
        None => ROSTER.to_vec(),
    };

    for name in roster {
        let Some(clip) = gesture::by_name(name) else {
            eprintln!("no gesture called {name}; the roster is {ROSTER:?}");
            std::process::exit(1);
        };
        println!(
            "\n{name}, over {samples} samples and {} bodies:",
            BODIES.len()
        );
        println!(
            "  returns to rest at both ends: {}",
            gesture::returns_to_rest(&clip)
        );
        println!(
            "\n{:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>9} {:>8} {:>8}",
            "stature",
            "reach m",
            "strain",
            "up",
            "forward",
            "across",
            "head mm",
            "trunk",
            "step mm"
        );

        let mut spread: Vec<(f32, f32, f32)> = Vec::new();
        for (stature, limbs) in BODIES {
            let rig = body(stature, limbs);
            let Some(arm) = rig.limb_reach(Limb::ForeRight) else {
                eprintln!("the {stature} m body has no arm");
                std::process::exit(1);
            };
            let height = rig.extent();
            // **The joint the clip actually drives**, which is not the one
            // `extremity_joints` names first. That list carries the wrist at its
            // head; `Clip` places the contact, which `in_zone(Extremity)` gives
            // and which is the joint past it. Measured against the wrist, this
            // audit read a wave landing 117 mm from where it was sent and called
            // it no strain at all — the same trap the gait documented for the
            // ankle, in the same crate, from the same two lists.
            let hands: Vec<usize> = [Limb::ForeLeft, Limb::ForeRight]
                .into_iter()
                .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
                .collect();
            let head = *rig
                .in_zone(Zone::Head)
                .first()
                .expect("the humanoid plan builds a head");
            let trunk: Vec<usize> = [Zone::Chest, Zone::Abdomen, Zone::Pelvis]
                .into_iter()
                .flat_map(|zone| rig.in_zone(zone))
                .collect();
            let shoulder = rig.limb_chain(Limb::ForeLeft).expect("an arm")[0];

            let rest = Pose::rest(&rig).forward(&rig).positions;
            if std::env::var("ANATOMY").is_ok() {
                let hand = rest[hands[0]] - rest[shoulder];
                println!(
                    "    ANATOMY stature {stature} limbs {limbs}: extent {height:.3}, reach \
                     {:.3} of it, rest hand from shoulder ({:.3}, {:.3}, {:.3}) of it",
                    arm / height,
                    hand.x / height,
                    hand.y / height,
                    hand.z / height,
                );
            }
            let mut strained = 0usize;
            let mut most = (0.0f32, 0.0f32, 0.0f32);
            let mut nearest_head = f32::MAX;
            let mut nearest_trunk = f32::MAX;
            let mut step = 0.0f32;
            let mut before: Option<Vec<Vec3>> = None;

            for frame in 0..=samples {
                let time = frame as f32 / samples as f32;
                let mut pose = Pose::rest(&rig);
                strained += clip.apply(&rig, &mut pose, time).len();
                let places = pose.forward(&rig).positions;
                for &hand in &hands {
                    let moved = places[hand] - rest[hand];
                    most = (
                        most.0.max(moved.y / height),
                        most.1.max(moved.z / height),
                        most.2.max(moved.x.abs() / height),
                    );
                    nearest_head = nearest_head.min(places[hand].distance(places[head]));
                    nearest_trunk = nearest_trunk.min(axis_distance(&places, &trunk, places[hand]));
                }
                if let Some(last) = &before {
                    step = step.max(
                        places
                            .iter()
                            .zip(last)
                            .fold(0.0f32, |most, (a, b)| most.max(a.distance(*b))),
                    );
                }
                before = Some(places);
            }
            // Where the hand ends up against the shoulder, which is what makes a
            // raise a greeting rather than a stretch.
            let over_shoulder = {
                let mut pose = Pose::rest(&rig);
                clip.apply(&rig, &mut pose, 0.5);
                let places = pose.forward(&rig).positions;
                (places[hands[hands.len() - 1]].y - places[shoulder].y) / height
            };

            spread.push(most);
            println!(
                "{stature:>8.2} {limbs:>6.1} {arm:>8.3} {strained:>7} {:>8.3} {:>8.3} {:>8.3} {:>9.0} {:>8.0} {:>8.1}",
                most.0,
                most.1,
                most.2,
                nearest_head * 1000.0,
                nearest_trunk * 1000.0,
                step * 1000.0,
            );
            let _ = over_shoulder;
        }

        let widest = |pick: fn(&(f32, f32, f32)) -> f32| {
            let (low, high) = spread.iter().fold((f32::MAX, f32::MIN), |range, at| {
                (range.0.min(pick(at)), range.1.max(pick(at)))
            });
            high - low
        };
        println!(
            "\n  spread: across every body in the sweep the hand's rise varies by {:.3} of a body \
             height, its forward reach by {:.3} and its swing by {:.3}",
            widest(|at| at.0),
            widest(|at| at.1),
            widest(|at| at.2),
        );
        println!(
            "          (this is the reading that says whether the format works. A gesture stated \
             in reaches should land in the same place on every body as a fraction of that body; \
             one whose figures drift with stature is normalised in name only)"
        );
    }
    println!(
        "\nhow it READS is not in this table. `cargo run --release -F builtin-clips --example \
         viewer -- --gesture Greeting`, in the sibling `bevy_symbios_avatar`."
    );
}

/// A humanoid rig of the given stature and limb proportion.
fn body(height: f32, limbs: f32) -> Rig {
    let params = HumanoidParams {
        height,
        limb_length: limbs,
        ..HumanoidParams::default()
    };
    Rig::from_skeleton(&params.skeleton(&Composites::default())).expect("the plan builds a rig")
}

/// How far a point is from the line the trunk lies along.
fn axis_distance(posed: &[Vec3], trunk: &[usize], at: Vec3) -> f32 {
    let (Some(&first), Some(&last)) = (trunk.first(), trunk.last()) else {
        return f32::MAX;
    };
    let (from, to) = (posed[last], posed[first]);
    let axis = to - from;
    if axis.length_squared() <= f32::EPSILON {
        return at.distance(from);
    }
    let along = (at - from).dot(axis) / axis.length_squared();
    (at - (from + axis * along.clamp(0.0, 1.0))).length()
}
