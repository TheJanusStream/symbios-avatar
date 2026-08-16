//! What a swim is doing, measured.
//!
//! There is no ground under a swimming body, so the readings a walk lives or
//! dies by — penetration, skid, clearance — have nothing to be taken against.
//! The temptation is to judge a swim by eye. That is right about the *last*
//! question and wrong about the ones before it: a cycle can be measured for
//! whether it closes, for whether its two
//! halves agree, for whether the hands actually push water backwards, and for
//! whether anything sweeps through the body. All four are defects a viewer
//! shows only once you already suspect them.
//!
//! # What is measured
//!
//! 1. **Does the cycle close?** The furthest any joint sits from where it was a
//!    whole cycle earlier. A procedural loop that does not close pops once per
//!    cycle for as long as the body swims, and it is the exact defect the
//!    imported reference clips carry. This is the reading the design
//!    is built around: every frequency in the motion is a whole multiple of the
//!    stroke because of it.
//!
//! 2. **Is the second half the mirror of the first?** A crawl is one arm's
//!    stroke done twice, half a cycle apart, so the body at `t + 0.5` should be
//!    the mirror image of the body at `t`. Measured across every joint, against
//!    its own mirror partner, which is a stronger statement than measuring the
//!    hands: it catches a trunk that rolls asymmetrically as readily as an arm
//!    that does.
//!
//! 3. **Do the hands push water backwards?** A hand is what a swimmer has
//!    instead of a foot, and the stroke is propulsive exactly while the hand
//!    travels backwards relative to the body. Reported as how far each hand
//!    travels backwards over its pull and what share of the cycle it spends
//!    doing it — a swim whose hands only ever paddle up and down is a body
//!    miming.
//!
//! 4. **Does anything sweep through the body?** The closest any hand comes to
//!    the trunk's own axis. An arm loop opened out by an effort axis is exactly
//!    the sort of thing that reaches its widest by passing through the chest.
//!
//! 5. **Is it continuous?** The furthest any joint moves between two adjacent
//!    samples, which catches a pop that the closure reading misses because it
//!    happens somewhere other than the seam.
//!
//! And the carriage, reported rather than judged: the pitch, the roll and the
//! surge at each pace.
//!
//! **What no number here settles is how it reads.** For that:
//!
//! ```text
//! cargo run --release -F builtin-clips --example viewer -- --swim 1.3
//! ```
//!
//! ```text
//! cargo run --example swimaudit
//! cargo run --example swimaudit -- --pace 1.3        # one pace, in detail
//! cargo run --example swimaudit -- --samples 720
//! cargo run --example swimaudit -- --bare            # the limbs, without the trunk
//! ```

use glam::Vec3;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Limb, Pose, Rig, Swim, Zone,
    anim::swim::{PRONE_AT, length_is},
};

/// How many points of the stroke every summary figure is taken from.
const SWEEP: usize = 240;

/// One pace's worth of readings.
struct Reading {
    pace: f32,
    effort: f32,
    /// How far out of family the step across the wrap is: the joint travel from
    /// the last sample of the cycle to the first, over the median travel
    /// between samples everywhere else.
    seam: f32,
    /// The furthest any joint sits from its mirror partner half a cycle away.
    mirror: f32,
    /// The furthest any joint moves between two adjacent samples, and which
    /// joint and moment it was — because a pop that cannot be attributed is a
    /// pop that has to be hunted for by eye.
    step: f32,
    step_by: (usize, f32),
    /// How far a hand travels backwards through its pull, and the share of the
    /// cycle it spends going backwards.
    pull: f32,
    pulling: f32,
    /// The closest a hand comes to the trunk's own axis.
    clearance: f32,
    pitch: f32,
    roll: f32,
    surge: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let samples = number("--samples")
        .map_or(SWEEP, |value| value as usize)
        .max(4);
    let bare = args.iter().any(|arg| arg == "--bare");

    let record = AvatarRecord::new("Swimmer", Archetype::default());
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the swimming body would not build");
        std::process::exit(1);
    };
    let rig = &avatar.rig;
    let length = length_is(rig);

    // Every joint's mirror partner, matched by rest position: the joint at the
    // same height and depth on the other side. Taken once off the rest
    // skeleton, because a mirror is a property of the body and not of the pose.
    let partners = mirrors(rig);

    // The hands, and the trunk axis they must not sweep through.
    let hands: Vec<(Limb, usize)> = [Limb::ForeLeft, Limb::ForeRight]
        .into_iter()
        .filter_map(|limb| rig.extremity_joints(limb).first().map(|&at| (limb, at)))
        .collect();
    let trunk: Vec<usize> = [Zone::Chest, Zone::Abdomen, Zone::Pelvis]
        .into_iter()
        .flat_map(|zone| rig.in_zone(zone))
        .collect();

    let paces: Vec<f32> = match number("--pace") {
        Some(pace) => vec![pace],
        None => vec![0.0, 0.3, 0.7, 1.3, 2.0],
    };

    println!(
        "a body {:.3} m long; full effort is {:.2} lengths a second, which is {:.2} m/s",
        length,
        PRONE_AT,
        length * PRONE_AT,
    );
    if bare {
        println!("the limbs alone: no pitch, no roll, no surge");
    }

    let readings: Vec<Reading> = paces
        .iter()
        .map(|&pace| {
            let at = |cycle: f32| {
                let mut pose = Pose::rest(rig);
                let swum = Swim {
                    cycle,
                    pace,
                    carriage: !bare,
                }
                .drive(rig, &mut pose);
                (pose.forward(rig).positions, swum)
            };

            let sweep: Vec<Vec<Vec3>> = (0..samples)
                .map(|frame| at(frame as f32 / samples as f32).0)
                .collect();
            let effort = at(0.0).1.effort;

            // 1. The seam, as a RATIO and not a distance. Asking for the pose a
            //    whole cycle on and comparing it with the pose here answers
            //    nothing: `Swim::drive` wraps its own cycle, so cycle 1.0 IS
            //    cycle 0.0 and the two are equal by construction however badly
            //    the motion pops. What a loop failing to close actually looks
            //    like is a step across the wrap that is out of family with
            //    every other step, so that is what is measured — against the
            //    median rather than the mean, which one bad step would drag.
            let mut steps: Vec<f32> = (0..samples)
                .map(|frame| worst(&sweep[frame], &sweep[(frame + 1) % samples]))
                .collect();
            let across = steps[samples - 1];
            steps.sort_by(f32::total_cmp);
            let median = steps[samples / 2];
            let seam = if median > f32::EPSILON {
                across / median
            } else {
                0.0
            };

            // 2. The mirror. Half a cycle on, every joint should sit where its
            //    partner sits now, reflected across the body's midline.
            let mirror = (0..samples).fold(0.0f32, |worst, frame| {
                let now = &sweep[frame];
                let half = &sweep[(frame + samples / 2) % samples];
                partners
                    .iter()
                    .enumerate()
                    .fold(worst, |worst, (joint, &partner)| {
                        let reflected = Vec3::new(-now[joint].x, now[joint].y, now[joint].z);
                        worst.max(half[partner].distance(reflected))
                    })
            });

            // 5. Continuity, between adjacent samples and across the wrap.
            let mut step = 0.0f32;
            let mut step_by = (0usize, 0.0f32);
            for frame in 0..samples {
                let next = &sweep[(frame + 1) % samples];
                for (joint, at) in sweep[frame].iter().enumerate() {
                    if at.distance(next[joint]) > step {
                        step = at.distance(next[joint]);
                        step_by = (joint, frame as f32 / samples as f32);
                    }
                }
            }

            // 3. Propulsion. The hand's travel relative to the BODY, taken
            //    along the body's own long axis — which is the way it is
            //    travelling, and which the pitch has laid down, so it is read
            //    off the trunk rather than assumed to be `+Z`.
            let mut pull = 0.0f32;
            let mut pulling = 0.0f32;
            for &(_, hand) in &hands {
                let (mut back, mut most, mut share) = (0.0f32, 0.0f32, 0usize);
                for frame in 0..samples {
                    let ahead = travel_axis(&sweep[frame], &trunk);
                    let moved = sweep[(frame + 1) % samples][hand] - sweep[frame][hand];
                    let along = moved.dot(ahead);
                    if along < 0.0 {
                        back -= along;
                        share += 1;
                    } else {
                        most = most.max(back);
                        back = 0.0;
                    }
                }
                pull = pull.max(most.max(back));
                pulling = pulling.max(share as f32 / samples as f32);
            }

            // 4. Clearance from the trunk's own axis.
            let clearance = (0..samples).fold(f32::MAX, |closest, frame| {
                hands.iter().fold(closest, |closest, &(_, hand)| {
                    closest.min(axis_distance(&sweep[frame], &trunk, sweep[frame][hand]))
                })
            });

            let (pitch, roll, surge) =
                (0..samples).fold((0.0f32, 0.0f32, 0.0f32), |most, frame| {
                    let swum = at(frame as f32 / samples as f32).1;
                    (
                        most.0.max(swum.pitch),
                        most.1.max(swum.roll.abs()),
                        most.2.max(swum.surge.abs()),
                    )
                });

            Reading {
                pace,
                effort,
                seam,
                mirror,
                step,
                step_by,
                pull,
                pulling,
                clearance,
                pitch,
                roll,
                surge,
            }
        })
        .collect();

    println!(
        "\n{:>6} {:>7} {:>9} {:>9} {:>9} {:>9} {:>8} {:>10} {:>7} {:>7} {:>7}",
        "m/s",
        "effort",
        "seam mm",
        "mirror mm",
        "step mm",
        "pull mm",
        "pulling",
        "clear mm",
        "pitch",
        "roll",
        "surge mm"
    );
    for reading in &readings {
        println!(
            "{:>6.2} {:>7.2} {:>9.2} {:>9.3} {:>9.1} {:>9.0} {:>8.2} {:>10.0} {:>7.1} {:>7.1} {:>7.1}",
            reading.pace,
            reading.effort,
            reading.seam,
            reading.mirror * 1000.0,
            reading.step * 1000.0,
            reading.pull * 1000.0,
            reading.pulling,
            reading.clearance * 1000.0,
            reading.pitch.to_degrees(),
            reading.roll.to_degrees(),
            reading.surge * 1000.0,
        );
    }

    let worst_seam = readings.iter().fold(0.0f32, |most, r| most.max(r.seam));
    let worst_mirror = readings.iter().fold(0.0f32, |most, r| most.max(r.mirror));
    let worst_step = readings.iter().fold(0.0f32, |most, r| most.max(r.step));
    let loudest = readings
        .iter()
        .max_by(|a, b| a.step.total_cmp(&b.step))
        .expect("a pace was measured");
    let least_pull = readings
        .iter()
        .filter(|r| r.effort > 0.0)
        .fold(f32::MAX, |least, r| least.min(r.pull));
    let least_clear = readings
        .iter()
        .fold(f32::MAX, |least, r| least.min(r.clearance));

    println!("\nover {samples} samples of the stroke:");
    println!(
        "  seam:    the step across the wrap is at worst {:.2} times the median step (a loop that \
         does not close pops once a stroke, forever, and this is the reading every frequency in \
         the motion is a whole multiple of the stroke to satisfy. A RATIO, because the motion \
         wraps its own cycle and so is equal to itself a cycle on however badly it pops)",
        worst_seam
    );
    println!(
        "  mirror:  the two halves agree to {:.3} mm (a crawl is one stroke done twice, half a \
         cycle apart, so the body at t+0.5 is the body at t reflected — measured across every \
         joint against its own partner, not just the hands)",
        worst_mirror * 1000.0
    );
    println!(
        "  step:    no joint moved more than {:.1} mm between adjacent samples — joint {} at \
         cycle {:.3}, at {:.2} m/s (a pop the seam reading cannot see, because it happens \
         somewhere other than the seam)",
        worst_step * 1000.0,
        loudest.step_by.0,
        loudest.step_by.1,
        loudest.pace,
    );
    println!(
        "  pull:    every swimming pace drove a hand at least {:.0} mm backwards along the body's \
         own axis in one sweep, and a hand spends up to {:.0}% of the stroke going backwards \
         (a swim whose hands only paddle up and down is a body miming)",
        least_pull * 1000.0,
        readings.iter().fold(0.0f32, |most, r| most.max(r.pulling)) * 100.0,
    );
    println!(
        "  clear:   a hand came within {:.0} mm of the trunk's own axis (the arm's loop opens \
         with effort, and the widest thing it could open into is the chest)",
        least_clear * 1000.0
    );
    println!(
        "\nhow it READS is not in this table. `cargo run --release -F builtin-clips --example viewer -- --swim 1.3`."
    );
}

/// Every joint's mirror partner: the joint at the same height and depth on the
/// other side of the body, or the joint itself where there is none.
fn mirrors(rig: &Rig) -> Vec<usize> {
    (0..rig.len())
        .map(|joint| {
            let at = rig.joints[joint].position;
            let want = Vec3::new(-at.x, at.y, at.z);
            (0..rig.len())
                .min_by(|&a, &b| {
                    rig.joints[a]
                        .position
                        .distance(want)
                        .total_cmp(&rig.joints[b].position.distance(want))
                })
                .unwrap_or(joint)
        })
        .collect()
}

/// The furthest apart any one joint is between two posed skeletons.
fn worst(one: &[Vec3], other: &[Vec3]) -> f32 {
    one.iter()
        .zip(other)
        .fold(0.0f32, |most, (a, b)| most.max(a.distance(*b)))
}

/// Which way the body is pointing, taken off its own trunk.
///
/// The head end minus the tail end of the trunk joints, which on an upright
/// body is `+Y` and on a prone one is `+Z` — so the propulsion reading follows
/// the body down as it lies over, rather than measuring a fixed axis the body
/// has left.
fn travel_axis(posed: &[Vec3], trunk: &[usize]) -> Vec3 {
    let (Some(&first), Some(&last)) = (trunk.first(), trunk.last()) else {
        return Vec3::Z;
    };
    (posed[first] - posed[last]).normalize_or(Vec3::Z)
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
