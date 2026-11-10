//! A winged body that stands on four steps without a joint jumping in a frame
//! (#445, #446): a bat, a glider and their crosses walking and bounding over
//! a 20 degree slope, carried and held, and turning on the spot on the level
//! and on the slope, left and right, at 30, 60 and 144 frames a second - positions and angles bounded
//! against each frame's neighbours, and against a joint going there and back.
//!
//! Every body is driven through [`Driver::drive`], as an application drives
//! one. A bird, and a cross a bird leads, stands on a level floor in
//! Overlands as a person does and is not here: `winged_motion` and
//! `winged_life` hold its strides.

use super::winged_bounds::pops_over;
use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Ground, Limb, Quat, Rig, Vec3, WingedFamily, WingedParams,
    Zone,
};

/// The rates each case is driven at, frames a second.
const RATES: [f32; 3] = [30.0, 60.0, 144.0];

/// One driven frame: its joints and their turns, in the body.
type Posed = (Vec<Vec3>, Vec<Quat>);

/// The bodies that stand on four: a bat, a glider, the two at the halfway
/// blend both ways round - a glider leads the first and a bat the second - a
/// bat and a glider each crossed with a bird, and two rolled bats (seeds 4
/// and 13, a large long-legged one and a small short-legged one).
fn bodies() -> Vec<(String, Avatar)> {
    use WingedFamily::{Bat, Bird, Glider};
    let named = [
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bat x glider 0.5", Lineage::cross(Bat, Glider, 0.5)),
        ("glider x bat 0.5", Lineage::cross(Glider, Bat, 0.5)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
        ("glider x bird 0.3", Lineage::cross(Glider, Bird, 0.3)),
    ]
    .into_iter()
    .map(|(name, lineage)| {
        let mut params = WingedParams::default();
        params.set_lineage(lineage);
        let record = AvatarRecord::new("Winged", Archetype::Winged(params));
        (name.to_string(), record)
    });
    let rolled = [4i64, 13].into_iter().map(|seed| {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        (format!("roll {seed}"), record)
    });
    named
        .chain(rolled)
        .map(|(name, record)| {
            let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
            assert!(
                avatar.rig.four_legged(),
                "the premise: {name} stands on four"
            );
            (name, avatar)
        })
        .collect()
}

/// How high a body's hip joint stands at rest.
fn hip(rig: &Rig) -> f32 {
    let chain = rig.limb_chain(Limb::HindLeft).expect("a hind leg");
    rig.joints[chain[0]].position.y
}

/// A hundredth of the trunk, shoulder to hip: what a frame may move a joint
/// over any bound here, `winged_life`'s slack.
fn slack(rig: &Rig) -> f32 {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    0.01 * rig.joints[pelvis]
        .position
        .distance(rig.joints[withers].position)
}

/// Two seconds going straight ahead at `speed` at `hz`, the chassis carried
/// along (`carried`) or held where it stands while its velocity still asks for
/// the pace, on ground rising by `rise` along each axis (the level at none):
/// `winged_motion`'s `bounding`, at any pace.
fn going(rig: &Rig, hz: f32, carried: bool, rise: Vec3, speed: f32) -> Vec<Posed> {
    let delta = 1.0 / hz;
    let mut driver = Driver::seeded(7);
    let mut at = Vec3::ZERO;
    let frames = (2.0 * hz).round() as usize;
    let up = Vec3::new(-rise.x, 1.0, -rise.z).normalize();
    let sloped = move |point: Vec3| {
        Some(Ground {
            position: Vec3::new(point.x, rise.x * point.x + rise.z * point.z, point.z),
            normal: up,
        })
    };
    (0..frames)
        .map(|_| {
            if carried {
                at += Vec3::Z * (speed * delta);
            }
            let inputs = Inputs {
                delta,
                velocity: Vec3::Z * speed,
                at,
                ..Inputs::default()
            };
            let driven = if rise == Vec3::ZERO {
                driver.drive(rig, &inputs, level_ground)
            } else {
                driver.drive(rig, &inputs, sloped)
            }
            .expect("an unheld body is posed");
            let posed = driven.pose.forward(rig);
            (posed.positions, posed.rotations)
        })
        .collect()
}

/// `seconds` standing on ground rising by `rise` along each axis (the level
/// at none) and turning as `plan` says at each moment, in radians a second,
/// positive toward the body's left, at `hz`: the chassis turned by its host,
/// which tells the driver how fast.
fn turning(rig: &Rig, hz: f32, seconds: f32, rise: Vec3, plan: impl Fn(f32) -> f32) -> Vec<Posed> {
    let delta = 1.0 / hz;
    let mut driver = Driver::seeded(7);
    let mut facing = 0.0f32;
    let frames = (seconds * hz).round() as usize;
    let up = Vec3::new(-rise.x, 1.0, -rise.z).normalize();
    let sloped = move |point: Vec3| {
        Some(Ground {
            position: Vec3::new(point.x, rise.x * point.x + rise.z * point.z, point.z),
            normal: up,
        })
    };
    (0..frames)
        .map(|frame| {
            let turn = plan(frame as f32 * delta);
            facing += turn * delta;
            let inputs = Inputs {
                delta,
                facing,
                turn,
                ..Inputs::default()
            };
            let driven = if rise == Vec3::ZERO {
                driver.drive(rig, &inputs, level_ground)
            } else {
                driver.drive(rig, &inputs, sloped)
            }
            .expect("an unheld body is posed");
            let posed = driven.pose.forward(rig);
            (posed.positions, posed.rotations)
        })
        .collect()
}

/// Each frame's step after the first `from`: how far the joints moved and
/// turned into it, the most of them, read as a sixtieth's at `hz`.
fn steps(frames: &[Posed], from: usize, hz: f32) -> Vec<(f32, f32)> {
    let scale = hz / 60.0;
    (from.max(1)..frames.len())
        .map(|at| {
            let ((was, was_turned), (is, is_turned)) = (&frames[at - 1], &frames[at]);
            let moved = is
                .iter()
                .zip(was)
                .map(|(a, b)| a.distance(*b))
                .fold(0.0f32, f32::max);
            let turned = is_turned
                .iter()
                .zip(was_turned)
                .map(|(a, b)| a.angle_between(*b))
                .fold(0.0f32, f32::max);
            (moved * scale, turned * scale)
        })
        .collect()
}

/// The furthest any joint went back along the way it had just come, between
/// one frame's step and the next, from the frame `from` on, read as a
/// sixtieth's at `hz`: with the frame it turned back in and the joint.
///
/// **What a zig-zag is**: a joint thrown one way in a frame and back the next.
/// A step into a frame and the step out of it that point against each other
/// went back by the shorter of the two, as far as they are opposed. A foot
/// landing turns back too - forward through its swing, back under the body in
/// its stance - by what a frame of its stance moves it, which is the body's
/// own pace; a pop there and back turns back by the pop, whatever the pace.
fn turned_back(frames: &[Posed], from: usize, hz: f32) -> (f32, usize, usize) {
    let scale = hz / 60.0;
    let mut worst = (0.0f32, 0usize, 0usize);
    for at in from.max(1)..frames.len().saturating_sub(1) {
        let (before, here, after) = (&frames[at - 1].0, &frames[at].0, &frames[at + 1].0);
        for joint in 0..here.len() {
            let (into, out) = (here[joint] - before[joint], after[joint] - here[joint]);
            let against = -into.dot(out);
            if against <= 0.0 {
                continue;
            }
            let back = against / into.length().max(out.length()) * scale;
            if back > worst.0 {
                worst = (back, at + 1, joint);
            }
        }
    }
    worst
}

/// The median of `values`.
fn median_of(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// The most any joint may turn in a sixtieth of a second on the ground, in
/// radians: `winged_life`'s bound.
const GROUND_TURN: f32 = 1.25;

/// How much more than twice the most it turned in the frames around it a
/// joint may turn in a sixtieth: `winged_motion`'s.
const TURN_SLACK: f32 = 0.05;

/// Every frame of `frames` from `from` on that pops against its neighbours,
/// in place or in turn: a joint moved more than twice the most it moved in
/// the four frames either side and `slack`, or turned more than twice the
/// most it turned in them and [`TURN_SLACK`], or more than [`GROUND_TURN`],
/// each read as a sixtieth's at `hz`.
fn pops(
    label: &str,
    frames: &[Posed],
    from: usize,
    hz: f32,
    (slack, turn_slack): (f32, f32),
) -> Vec<String> {
    let mut failures = Vec::new();
    let read = steps(frames, from, hz);
    for at in 4..read.len().saturating_sub(4) {
        let turn = read[at].1;
        let around = (at - 4..at)
            .chain(at + 1..=at + 4)
            .map(|frame| read[frame].1)
            .fold(0.0f32, f32::max);
        if turn > (2.0 * around + turn_slack).min(GROUND_TURN) {
            failures.push(format!(
                "{label}: frame {} turned a joint {turn:.3} rad a sixtieth against {around:.3}",
                from + at
            ));
        }
    }
    let joints: Vec<Vec<Vec3>> = frames[from..].iter().map(|frame| frame.0.clone()).collect();
    let deltas = vec![1.0 / hz; joints.len()];
    for (at, step, around) in pops_over(&joints, &deltas, 1.0 / 60.0, slack) {
        failures.push(format!(
            "{label}: frame {} moved {:.1} mm a sixtieth against {:.1}",
            from + at,
            step * 1000.0,
            around * 1000.0
        ));
    }
    failures
}

/// How many times what a median frame of a carried stride moves a joint a
/// frame may move one, on the level and on the slope: `winged_motion`'s, for
/// a bound.
const PEAK: (f32, f32) = (2.0, 2.5);

/// How much of what a median frame of a carried stride moves a joint any
/// joint may go back by between one frame and the next, see [`turned_back`]:
/// a foot landing goes back 0.67 of it at the most (a rolled bat's, bounding
/// down the slope at 30 frames a second), a Bat x Glider cross's elbow went
/// 2.5 and 1.6 at 144 and 0.95 at 30 before #445.
const BACK: f32 = 0.8;

/// **A body on four steps over a slope without a zig-zag** (#445): a bat, a
/// glider, the two crossed at the halfway blend both ways round, each crossed
/// with a bird and two rolled bats, walking at a Froude number of 0.15 on the
/// hip and bounding at one, at 30, 60 and 144 frames a second, on the level
/// and facing up, down and across a 20 degree slope both ways, their chassis
/// carried along and held where it stands, every frame from the half second
/// in read as a sixtieth at the speed it moved:
///
/// - **no joint moves in a frame more than [`PEAK`] times what a median frame
///   of the carried stride moves it on the same ground at the same rate, and
///   a hundredth of the trunk**;
/// - **no joint goes back along the way it had just come by more than [`BACK`]
///   of that median and the hundredth** ([`turned_back`]: a zig-zag);
/// - **nor pops against the four frames either side, in place or in turn**
///   ([`pops`]).
///
/// Before, a Bat x Glider cross at the halfway blend - a glider's stance on
/// a bat's long arm bones, folded shut but for 52 mm - ran its wrist into its
/// arm's fold as its foot lifted over the slope, and its elbow went there and
/// back: walking down the slope carried, a joint moved 84.4 mm a sixtieth at
/// 144 frames a second against a median of 27.1 (71.3 at 60) and went 47.2
/// back in the next frame; held walking up it, 82.6 and 72.4 back. Bounding,
/// and every other body here, kept every bound then as now.
#[test]
fn a_body_on_four_steps_over_a_slope_without_a_zig_zag() {
    let grade = 20.0f32.to_radians().tan();
    let grounds = [
        ("on the level", Vec3::ZERO),
        ("up the slope", Vec3::Z * grade),
        ("down the slope", -Vec3::Z * grade),
        ("across the slope, rising left", Vec3::X * grade),
        ("across the slope, rising right", -Vec3::X * grade),
    ];
    let bodies = bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let (failures, grounds) = (&failures, &grounds);
            scope.spawn(move || {
                let found = stride_failures(name, &avatar.rig, grounds);
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// [`a_body_on_four_steps_over_a_slope_without_a_zig_zag`] for one body:
/// every bound it breaks, on each of `grounds`.
fn stride_failures(name: &str, rig: &Rig, grounds: &[(&str, Vec3)]) -> Vec<String> {
    let mut failures = Vec::new();
    let slack = slack(rig);
    let paces = [
        ("walking", (0.15 * 9.81 * hip(rig)).sqrt().max(0.4)),
        ("bounding", (9.81 * hip(rig)).sqrt().max(0.4)),
    ];
    for (pace, speed) in paces {
        for &(on, rise) in grounds {
            for hz in RATES {
                let from = (0.5 * hz).round() as usize;
                let carried = going(rig, hz, true, rise, speed);
                let moved = median_of(
                    steps(&carried, from, hz)
                        .iter()
                        .map(|step| step.0)
                        .collect(),
                );
                let peak = if rise == Vec3::ZERO { PEAK.0 } else { PEAK.1 };
                for how in ["carried", "held"] {
                    let held;
                    let frames = if how == "carried" {
                        &carried
                    } else {
                        held = going(rig, hz, false, rise, speed);
                        &held
                    };
                    let label = format!("{name} {pace} {how} {on} at {hz}");
                    let most = steps(frames, from, hz)
                        .iter()
                        .map(|step| step.0)
                        .fold(0.0f32, f32::max);
                    let (back, frame, joint) = turned_back(frames, from, hz);
                    if std::env::var("W20_DIAG").is_ok() {
                        println!(
                            "{label}: most {:.1} median {:.1} ratio {:.2} back {:.1} share {:.2} \
                             (joint {joint} frame {frame}) pops {}",
                            most * 1000.0,
                            moved * 1000.0,
                            (most - slack) / moved,
                            back * 1000.0,
                            (back - slack) / moved,
                            pops(&label, frames, from, hz, (slack, TURN_SLACK)).len()
                        );
                    }
                    if most > peak * moved + slack {
                        failures.push(format!(
                            "{label}: a frame moved {:.1} mm a sixtieth, a median carried frame \
                             {:.1}",
                            most * 1000.0,
                            moved * 1000.0
                        ));
                    }
                    if back > BACK * moved + slack {
                        failures.push(format!(
                            "{label}: joint {joint} went {:.1} mm a sixtieth back in frame \
                             {frame}, a median carried frame {:.1}",
                            back * 1000.0,
                            moved * 1000.0
                        ));
                    }
                    failures.extend(pops(&label, frames, from, hz, (slack, TURN_SLACK)));
                }
            }
        }
    }
    failures
}

/// **A grade of a ten-thousandth is the level** (#445, #446): the same bodies
/// walking and bounding, carried, at 60 and 144 frames a second, up and
/// across ground rising a ten-thousandth, stand no joint more than
/// [`NEAR_LEVEL`] from where the level floor stands it, at any frame. Real
/// terrain is never exactly level, so anything keyed on a slope being nought
/// would pop at every road's edge: nothing #445 or #446 changed is, and this
/// holds them to it. A guard, not a fix's test: it reads 0.37 mm at the
/// most, as it did before them.
#[test]
fn a_grade_of_a_ten_thousandth_is_the_level() {
    let bodies = bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let failures = &failures;
            scope.spawn(move || {
                let rig = &avatar.rig;
                let paces = [
                    ("walking", (0.15 * 9.81 * hip(rig)).sqrt().max(0.4)),
                    ("bounding", (9.81 * hip(rig)).sqrt().max(0.4)),
                ];
                for (pace, speed) in paces {
                    for hz in [60.0f32, 144.0] {
                        let level = going(rig, hz, true, Vec3::ZERO, speed);
                        for (on, rise) in [("up", Vec3::Z), ("across", Vec3::X)] {
                            let near = going(rig, hz, true, rise * 1.0e-4, speed);
                            let apart = level
                                .iter()
                                .zip(&near)
                                .flat_map(|(a, b)| a.0.iter().zip(&b.0))
                                .map(|(a, b)| a.distance(*b))
                                .fold(0.0f32, f32::max);
                            if std::env::var("W20_DIAG").is_ok() {
                                println!("{name} {pace} {on} at {hz}: {:.3} mm", apart * 1000.0);
                            }
                            if apart > NEAR_LEVEL {
                                failures.lock().expect("a lock").push(format!(
                                    "{name} {pace} {on} a grade of a ten-thousandth at {hz}: a \
                                     joint {:.2} mm from where the level stands it",
                                    apart * 1000.0
                                ));
                            }
                        }
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far from where the level floor stands it a grade of a ten-thousandth
/// may stand a joint, in metres.
const NEAR_LEVEL: f32 = 0.001;

/// **A body on four turns on the spot without a pop** (#446): the same
/// bodies, standing half a second and then turned where they stand, to the
/// left and to the right, at 30, 60 and 144 frames a second - a pivot at a
/// radian a second for two and a half seconds and then standing again, and a
/// spin at two and a half radians a second for two seconds, back the other
/// way at 0.6 for two more and then standing - on the level and on a 20
/// degree slope, first facing up it, down it and across it both ways, every
/// frame from a third of a second in read as a sixtieth at the speed it
/// moved:
///
/// - **no joint pops against the four frames either side, in place or in
///   turn** ([`pops`], its turns' slack [`SPIN_TURN_SLACK`]), **nor turns more
///   than [`GROUND_TURN`]**;
/// - **nor goes back along the way it had just come by more than
///   [`SPIN_BACK`] of the trunk** ([`turned_back`]).
///
/// Before, every one of the eight broke it on the level, 582 times between
/// them:
///
/// - **a bat, a Glider x Bat and a Bat x Bird cross and both rolled bats
///   spinning**: a standstill turned knows no pattern, so their pivot changed
///   from a walk to a bound, unblended, in the frame the working foot's speed
///   passed where a bound begins, and back as the turn eased down - a bat
///   moved a joint 106.8 mm a sixtieth against 34.6 and then 227.1 against
///   11.7 at 144 frames a second (44 and 95 mm in the one frame), 95.5 at 60;
/// - **a glider, a Bat x Glider and a Glider x Bird cross pivoting and
///   spinning**: a stride that only turns has no length to read its swing's
///   time off, so their paws folded a beast's whole fold over a swing of a
///   twelfth of a second - a joint turned 1.76 to 1.98 rad a sixtieth at 144
///   frames a second and 1.41 to 1.66 at 60, over [`GROUND_TURN`];
/// - **a rolled bat pivoting** (seed 4, and seed 13 spinning): a hind foot
///   lifting a millimetre as its turn died away had its toes thrown out of
///   the ground in one frame, 28.3 mm a sixtieth against 10.7 at 144.
///
/// And across the slope, after those, 79 times: **a turning swing seated its
/// foot for ground it was only passing beside** (`gait::seated_offset`'s roof
/// read along the stride's direction, which a pivot's feet swing across), so
/// a downhill forepaw sweeping in under the chest uphill was held up at the
/// landing's height from its take-off. A Glider x Bird cross spinning brought
/// its wrist within 6.5 mm of its shoulder and threw its elbow 116 mm out in
/// one frame - 337.8 mm a sixtieth at 144 frames a second, a joint turned 4.12
/// rad and went back 0.43 of the trunk, 108.6 mm and 1.48 rad at 60; a Bat x
/// Glider cross pivoting at 60 moved its elbow 49.3 mm a sixtieth, a pop
/// against the frames around it; a bat spinning went back 0.13 to 0.14 of
/// the trunk at 30 and 60.
///
/// **On the slope the frames are read until a second after the turn has
/// stopped** ([`SLOPE_STANDING`]), where the level reads the whole of its
/// standing: further on, the idle steps its feet home, and standing on a
/// slope a Glider x Bat cross's hind leg is left in its rest bend for one
/// frame where its stance solve finds the rest contact within its tolerance
/// of the goal (`ground::solve_leg`'s early return, #262), 63.8 mm a sixtieth
/// at 144 frames a second, 4.3 s into a pivot. That is the idle's, not a
/// turn's, and is left to its own issue.
#[test]
fn a_body_on_four_turns_on_the_spot_without_a_pop() {
    let bodies = bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let failures = &failures;
            scope.spawn(move || {
                let found = turn_failures(name, &avatar.rig);
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The furthest a joint of a body turning on the spot may go back along the
/// way it had just come between one frame and the next, a sixtieth's, as a
/// share of the trunk: see [`turned_back`]. A foot landing in a spin of two
/// and a half radians a second goes back 0.09; a Bat x Glider cross's elbow
/// went 0.15 (38.4 mm) before #445. Spinning across the slope that cross's
/// elbow goes 0.117 at 60 frames a second, its forepaw's swing passing
/// through its fold over five frames with no pop (0.07 on the level).
const SPIN_BACK: f32 = 0.12;

/// How much more than twice the most it turned in the frames around it a
/// joint of a body turning on the spot may turn in a sixtieth, in radians: a
/// tenth, twice a stride's [`TURN_SLACK`]. A body stepping round turns its
/// joints a tenth of a radian a sixtieth, and a paw lifting off folds a
/// quarter of a radian in the frame it lifts at 60 frames a second (a Bat x
/// Glider cross's wrist 0.255 against 0.100), which is its gait's and no pop.
const SPIN_TURN_SLACK: f32 = 0.1;

/// How long after a turn has stopped a turn on a slope is read for, in
/// seconds: see [`a_body_on_four_turns_on_the_spot_without_a_pop`].
const SLOPE_STANDING: f32 = 1.0;

/// A named turn on the spot: how long it is driven on the level, in seconds,
/// when it stops turning, and how fast it turns at each moment, in radians a
/// second.
type Turned<'a> = (&'a str, f32, f32, &'a dyn Fn(f32) -> f32);

/// [`a_body_on_four_turns_on_the_spot_without_a_pop`] for one body: every
/// bound it breaks.
fn turn_failures(name: &str, rig: &Rig) -> Vec<String> {
    let mut failures = Vec::new();
    let slack = slack(rig);
    let pivot = |t: f32| if (0.5..3.0).contains(&t) { 1.0 } else { 0.0 };
    let spin = |t: f32| match t {
        t if t < 0.5 => 0.0,
        t if t < 2.5 => 2.5,
        t if t < 4.5 => -0.6,
        _ => 0.0,
    };
    let turns: [Turned; 2] = [
        ("pivoting", 4.5, 3.0, &pivot),
        ("spinning", 6.0, 4.5, &spin),
    ];
    let grade = 20.0f32.to_radians().tan();
    let grounds = [
        ("on the level", Vec3::ZERO),
        ("facing up the slope", Vec3::Z * grade),
        ("facing down the slope", -Vec3::Z * grade),
        ("across the slope, rising left", Vec3::X * grade),
        ("across the slope, rising right", -Vec3::X * grade),
    ];
    for (turn, seconds, stops, plan) in turns {
        for (on, rise) in grounds {
            let seconds = if rise == Vec3::ZERO {
                seconds
            } else {
                stops + SLOPE_STANDING
            };
            for (side, way) in [("left", 1.0f32), ("right", -1.0)] {
                for hz in RATES {
                    let from = (hz / 3.0).round() as usize;
                    let frames = turning(rig, hz, seconds, rise, |t| way * plan(t));
                    let label = format!("{name} {turn} {side} {on} at {hz}");
                    let (back, frame, joint) = turned_back(&frames, from, hz);
                    // In trunks: the slack is a hundredth of one.
                    let share = back / slack * 0.01;
                    if std::env::var("W20_DIAG").is_ok() {
                        let read = steps(&frames, from, hz);
                        println!(
                            "{label}: most {:.1} turn {:.3} back {:.1} trunks {share:.3} (joint \
                         {joint} frame {frame}) pops {}",
                            read.iter().map(|step| step.0).fold(0.0f32, f32::max) * 1000.0,
                            read.iter().map(|step| step.1).fold(0.0f32, f32::max),
                            back * 1000.0,
                            pops(&label, &frames, from, hz, (slack, SPIN_TURN_SLACK)).len()
                        );
                    }
                    if share > SPIN_BACK {
                        failures.push(format!(
                        "{label}: joint {joint} went {:.1} mm a sixtieth back in frame {frame}, \
                         {share:.3} of the trunk",
                        back * 1000.0
                    ));
                    }
                    failures.extend(pops(&label, &frames, from, hz, (slack, SPIN_TURN_SLACK)));
                }
            }
        }
    }
    failures
}
