//! A winged body's whole life through the driver (#424): on the ground, into
//! the air and back, and at rest, with every hand-off between the three - the
//! ground gait to a takeoff, a landing back to a walk, a walk to the family's
//! rest and up again, and an emote - held to the bounds the flight and the
//! rests are held to on their own.

use super::winged_bounds::{UNDER_GROUND, lowest_drawn, pops_over, turn_between};
use symbios_avatar::anim::driver::{Driver, DriverConfig, Inputs, Source, level_ground};
use symbios_avatar::anim::emote;
use symbios_avatar::anim::winged::flight::{Flight, FlightMode};
use symbios_avatar::anim::winged::rests::roster;
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Limb, Rig, Vec3, WingedFamily, WingedParams, Zone,
};

/// The three families, each pair crossed either side of and at the halfway
/// blend, and a few rolls.
fn bodies() -> Vec<(String, AvatarRecord)> {
    use WingedFamily::{Bat, Bird, Glider};
    let record = |lineage: Lineage<WingedFamily>| {
        let mut params = WingedParams::default();
        params.set_lineage(lineage);
        AvatarRecord::new("Winged", Archetype::Winged(params))
    };
    let mut bodies: Vec<(String, AvatarRecord)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), record(Lineage::pure(family))))
        .collect();
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.49, 0.5, 0.51] {
            bodies.push((
                format!("{a:?} x {b:?} {blend}"),
                record(Lineage::cross(a, b, blend)),
            ));
        }
    }
    for seed in 1..=3 {
        let mut rolled = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        rolled.reroll(seed);
        bodies.push((format!("roll {seed}"), rolled));
    }
    bodies
}

/// The most any joint may turn in a sixtieth of a second on the ground, in
/// radians: the flight's bound (`winged_flight`), which a joint flipping
/// about a toe or a paw - small enough to pass under a bound on positions -
/// breaks (a glider bounding at 3 m/s turned a forepaw 2.8 rad in a frame).
/// The ground gaits hold under 0.8 at 60 frames a second and 0.95 over the
/// uneven steps, a glider-led cross's forepaw breaking into its bound.
const GROUND_TURN: f32 = 1.25;

/// One stretch of a life: a flight or none, the speed the body is carried
/// at, how long it lasts, and a gesture asked on its first frame.
struct Stretch {
    flight: Option<FlightMode>,
    speed: f32,
    seconds: f32,
    asked: Option<&'static str>,
}

const fn ground(speed: f32, seconds: f32) -> Stretch {
    Stretch {
        flight: None,
        speed,
        seconds,
        asked: None,
    }
}

const fn air(mode: FlightMode, speed: f32, seconds: f32) -> Stretch {
    Stretch {
        flight: Some(mode),
        speed,
        seconds,
        asked: None,
    }
}

/// How high a body's hip joint stands at rest.
fn hip(rig: &Rig) -> f32 {
    let chain = rig.limb_chain(Limb::HindLeft).expect("a hind leg");
    rig.joints[chain[0]].position.y
}

/// The trunk's length, shoulder to hip.
fn trunk(rig: &Rig) -> f32 {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    rig.joints[pelvis]
        .position
        .distance(rig.joints[withers].position)
}

/// The life of `rig`: stand, walk, its fast gait, stop, take off, flap,
/// glide, flare, land, walk, stop, its family's rest and up, an emote, and
/// stand.
fn life(rig: &Rig) -> Vec<Stretch> {
    // A walk and the fast gait at their Froude numbers on its hip: a bird's
    // run or hop, a bat's and a glider's bound.
    let at = |froude: f32| (froude * 9.81 * hip(rig)).sqrt().max(0.4);
    let (walk, fast) = (at(0.15), at(1.0));
    let names = roster(rig);
    let (rest, up) = (names[0], emote::GET_UP);
    let own = names
        .iter()
        .copied()
        .find(|&name| name != up && emote::seconds(rig, name).is_none())
        .expect("an emote");
    let down = emote::seconds(rig, rest).expect("a rest is a state");
    let rise = emote::getting_up(rig, rest)
        .expect("a rest is got up from")
        .1;
    let emoting = DriverConfig::default().gesture_secs;
    let asking = |seconds: f32, name: &'static str| Stretch {
        asked: Some(name),
        ..ground(0.0, seconds)
    };
    vec![
        ground(0.0, 1.0),
        ground(walk, 2.0),
        ground(fast, 2.0),
        ground(0.0, 1.0),
        air(FlightMode::Takeoff, 2.5, 0.6),
        air(FlightMode::Flap, 5.0, 1.5),
        air(FlightMode::Glide, 6.0, 1.0),
        air(FlightMode::Flare, 4.0, 0.45),
        air(FlightMode::Land, 0.0, 0.3),
        ground(walk, 1.5),
        ground(0.0, 0.8),
        asking(down + 0.6, rest),
        asking(rise + 0.4, up),
        asking(emoting + 0.4, own),
        ground(0.0, 1.5),
    ]
}

/// Each frame's step, of its index.
type Steps = fn(usize) -> f32;

/// Drives `avatar` through its life, each frame's step `steps` of its
/// index, and answers every frame that breaks a bound.
fn live(label: &str, avatar: &Avatar, steps: Steps) -> Vec<String> {
    let rig = &avatar.rig;
    let mut driver = Driver::seeded(7);
    let mut failures = Vec::new();
    let (mut joints, mut deltas, mut sources) = (Vec::new(), Vec::new(), Vec::new());
    let mut turns: Vec<Vec<symbios_avatar::Quat>> = Vec::new();
    let mut at = Vec3::ZERO;
    let mut first: Option<f32> = None;
    let mut lowest = (f32::MAX, 0usize);
    let mut frame = 0usize;
    for stretch in life(rig) {
        let mut elapsed = 0.0f32;
        while elapsed < stretch.seconds - 1.0e-4 {
            let delta = steps(frame);
            at += Vec3::Z * (stretch.speed * delta);
            let inputs = Inputs {
                delta,
                velocity: Vec3::Z * stretch.speed,
                at,
                flight: stretch.flight.map(Flight::new),
                gesture: if elapsed == 0.0 { stretch.asked } else { None },
                ..Inputs::default()
            };
            let driven = driver
                .drive(rig, &inputs, level_ground)
                .expect("an unheld body is posed");
            let posed = driven.pose.forward(rig);
            joints.push(posed.positions);
            turns.push(posed.rotations);
            deltas.push(delta);
            sources.push(driven.source);
            // On the ground, the whole drawn body.
            if driven.source != Source::Fly && frame.is_multiple_of(2) {
                let low = lowest_drawn(avatar, &driven.pose);
                match first {
                    None => first = Some(low.min(0.0)),
                    Some(_) if low < lowest.0 => lowest = (low, frame),
                    Some(_) => {}
                }
            }
            elapsed += delta;
            frame += 1;
        }
    }
    for wanted in [Source::Gait, Source::Fly, Source::Gesture] {
        if !sources.contains(&wanted) {
            failures.push(format!("{label}: never {wanted:?}"));
        }
    }
    let first = first.unwrap_or(0.0);
    if lowest.0 < first - UNDER_GROUND {
        failures.push(format!(
            "{label}: {:.1} mm below the ground at frame {} ({:?}), standing {:.1}",
            -lowest.0 * 1000.0,
            lowest.1,
            sources[lowest.1],
            -first * 1000.0
        ));
    }
    let reference = 1.0 / 60.0;
    // On the ground, no joint turns more than [`GROUND_TURN`] in a sixtieth,
    // each frame's turn read at the speed it turned.
    for at in 1..turns.len() {
        if sources[at] == Source::Fly || sources[at - 1] == Source::Fly {
            continue;
        }
        let (turn, joint) = turns[at]
            .iter()
            .zip(&turns[at - 1])
            .enumerate()
            .map(|(joint, (a, b))| (turn_between(*b, *a) * reference / deltas[at], joint))
            .fold(
                (0.0f32, 0usize),
                |best, this| if this.0 > best.0 { this } else { best },
            );
        if turn > GROUND_TURN {
            failures.push(format!(
                "{label}: frame {at} ({:?}) turned joint {joint} {turn:.2} rad a sixtieth",
                sources[at]
            ));
        }
    }
    for (at, moved, around) in pops_over(&joints, &deltas, reference, 0.01 * trunk(rig)) {
        failures.push(format!(
            "{label}: frame {at} ({:?} after {:?}) moved {moved:.3} m a sixtieth against {around:.3}",
            sources[at],
            sources[at - 1]
        ));
    }
    failures
}

/// **A winged body's whole life hands off without a pop or a sink** (#424):
/// the three families, each pair crossed at 0.49, 0.5 and 0.51, and three
/// rolls, driven through standing, a walk, the fast gait (a run, a hop or a
/// bound), a stop, a takeoff, flapping, a glide, a flare and a landing, a
/// walk, a stop, the family's rest and getting up, an emote and standing
/// again, at 60 frames a second and at steps of a fortieth and a hundred and
/// twentieth in turn:
///
/// - **no joint moves in a frame more than twice the most it moved in the
///   four frames either side and a hundredth of the trunk** - the flight's
///   bound (`winged_flight`) - each frame's travel read as a sixtieth's at
///   the speed it moved, so a long frame among short ones is held to what
///   they move in as long;
/// - **on the ground no drawn point goes more than [`UNDER_GROUND`] below
///   where the body stood at the start** - the rests' bound
///   (`winged_rests`).
///
/// **Nothing is let off.** Two faults this test found are fixed: over the
/// uneven steps a bird-led body's thigh jumped 50 to 80 mm in one frame of
/// each stride - its foot's peel, solved in a beast's pantograph while the
/// rest of its stride stood the foot - and a stop's blend read the velocity
/// of a long frame over a short one; and a glider's forepaw, rolled by the
/// blend as its pace changed, put its sole 6 to 13 mm under the ground (a
/// dog's paw does too, 44 mm, which is left to it).
#[test]
fn a_winged_bodys_whole_life_hands_off_without_a_pop_or_a_sink() {
    let bodies = bodies();
    let rates: [(&str, Steps); 2] = [
        ("60", |_| 1.0 / 60.0),
        ("40 and 120", |frame| {
            if frame.is_multiple_of(2) {
                1.0 / 40.0
            } else {
                1.0 / 120.0
            }
        }),
    ];
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for chunk in bodies.chunks(bodies.len().div_ceil(4)) {
            let failures = &failures;
            scope.spawn(move || {
                for (name, record) in chunk {
                    let avatar = Avatar::build_for_shape(record).expect("a winged body builds");
                    for (rate, steps) in rates {
                        let label = format!("{name} at {rate}");
                        for line in live(&label, &avatar, steps) {
                            failures.lock().expect("a lock").push(line);
                        }
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
