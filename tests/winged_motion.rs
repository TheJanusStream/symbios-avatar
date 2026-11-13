//! Winged bodies on the ground (#424): the bird's walk with its head-bob, its
//! hop and its run, the bat's crawl and bound, the glider's bound; the drawn
//! toes on the ground through a stride; a bird's tail as one chain; a bird's
//! folded wings left out of the idle's breath; and motion that goes the same
//! at 60 and at 144 frames a second (#415).
//!
//! Every body is driven through [`Driver::drive`], as an application drives
//! one, carried by a chassis that steps it at the speed asked.

use super::winged_bounds::{
    UNDER_GROUND, lowest_drawn, lowest_drawn_over, lowest_toe, pops_over, turn_between,
};
use symbios_avatar::anim::driver::{Driven, Driver, Inputs, level_ground, velocity_of};
use symbios_avatar::anim::winged::ground::bob_share;
use symbios_avatar::anim::{Clip, Key, Pattern, Target, Track};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Family, Ground, Limb, Pose, QuadrupedParams,
    Quat, Rig, Vec3, WingedFamily, WingedParams, Zone,
};

const STEP: f32 = 1.0 / 60.0;

/// A winged body of `lineage`, `shaped` as a test asks.
fn winged(lineage: Lineage<WingedFamily>, shaped: impl Fn(&mut WingedParams)) -> Avatar {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    shaped(&mut params);
    built(AvatarRecord::new("Winged", Archetype::Winged(params)))
}

fn built(record: AvatarRecord) -> Avatar {
    Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("builds")
}

/// A crow's head (`render --beak 1,-1,-1`): a straight beak, which bobs.
fn crow(params: &mut WingedParams) {
    (params.beak_length, params.beak_hook, params.facial_disc) = (1.0, -1.0, -1.0);
}

/// A songbird's head on a small body, which hops at every pace.
fn songbird(params: &mut WingedParams) {
    (params.beak_length, params.beak_hook, params.facial_disc) = (0.2, -1.3, -1.0);
    params.size = 0.4;
}

fn bird() -> Lineage<WingedFamily> {
    Lineage::pure(WingedFamily::Bird)
}

/// One driven frame, with where the chassis stood and faced.
struct Frame {
    driven: Driven,
    at: Vec3,
    facing: f32,
    time: f32,
    cycle: f32,
    pattern: Option<Pattern>,
}

impl Frame {
    /// Where a joint stands in the world.
    fn world(&self, rig: &Rig, joint: usize) -> Vec3 {
        let posed = self.driven.pose.forward(rig);
        self.at + Quat::from_rotation_y(self.facing) * posed.positions[joint]
    }

    /// How high a limb's pad stands over the level ground, against where it
    /// rests.
    fn pad(&self, rig: &Rig, limb: Limb) -> f32 {
        let pad = rig.in_zone(Zone::Extremity(limb))[0];
        self.driven.pose.forward(rig).positions[pad].y - rig.joints[pad].position.y
    }

    /// How high a limb's foot stands off the ground: the least any joint of
    /// it - its pad, its sole and its toes - stands over where it rests, so a
    /// foot whose heel has peeled up over toes still down is on the ground.
    fn foot(&self, rig: &Rig, limb: Limb) -> f32 {
        let posed = self.driven.pose.forward(rig);
        rig.in_zone(Zone::Extremity(limb))
            .into_iter()
            .filter(|&joint| !rig.wings.as_ref().is_some_and(|wings| wings.owns(joint)))
            .map(|joint| posed.positions[joint].y - rig.joints[joint].position.y.max(0.0))
            .fold(f32::MAX, f32::min)
    }
}

/// Drives `rig` for `seconds` at `hz`, `plan` saying at each moment how fast
/// it goes, in metres a second, and how fast it turns, in radians a second.
fn drive(rig: &Rig, seconds: f32, hz: f32, plan: impl Fn(f32) -> (f32, f32)) -> Vec<Frame> {
    drive_steps(rig, seconds, |_| 1.0 / hz, plan)
}

/// [`drive`], each frame's step `steps` of its index: a frame rate, or an
/// uneven one.
fn drive_steps(
    rig: &Rig,
    seconds: f32,
    steps: impl Fn(usize) -> f32,
    plan: impl Fn(f32) -> (f32, f32),
) -> Vec<Frame> {
    drive_over(rig, seconds, steps, plan, Vec3::ZERO)
}

/// [`drive_steps`] over ground rising by `rise` along each axis (the level
/// at none): a plane through the body's origin, in the frame it is posed in,
/// as a host on a slope hands the driver its ground.
fn drive_over(
    rig: &Rig,
    seconds: f32,
    steps: impl Fn(usize) -> f32,
    plan: impl Fn(f32) -> (f32, f32),
    rise: Vec3,
) -> Vec<Frame> {
    let up = Vec3::new(-rise.x, 1.0, -rise.z).normalize();
    let sloped = move |point: Vec3| {
        Some(Ground {
            position: Vec3::new(point.x, rise.x * point.x + rise.z * point.z, point.z),
            normal: up,
        })
    };
    let mut driver = Driver::seeded(7);
    let (mut at, mut facing, mut time) = (Vec3::ZERO, 0.0f32, 0.0f32);
    let mut last: Option<Vec3> = None;
    let mut out = Vec::new();
    let mut frame = 0usize;
    while time < seconds - 1.0e-4 {
        let delta = steps(frame);
        let (speed, turn) = plan(time);
        facing += turn * delta;
        at += Vec3::new(facing.sin(), 0.0, facing.cos()) * (speed * delta);
        let velocity = last.map_or(Vec3::ZERO, |last| velocity_of(last, at, delta));
        last = Some(at);
        let inputs = Inputs {
            delta,
            velocity,
            at,
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
        out.push(Frame {
            driven,
            at,
            facing,
            time,
            cycle: driver.cycle(),
            pattern: driver.speed().and_then(|speed| speed.pattern()),
        });
        time += delta;
        frame += 1;
    }
    out
}

/// Standing a second, then going at `speed` straight ahead.
fn walking(speed: f32) -> impl Fn(f32) -> (f32, f32) {
    move |t| (if t < 1.0 { 0.0 } else { speed }, 0.0)
}

/// **A bird's head holds still and then thrusts, once a step** (research-11
/// 2.2): a crow walking at a Froude number near a fifth holds its head within
/// a few millimetres a second for at least a third of every step and thrusts
/// it at more than twice the body's speed, and there are as many holds as
/// steps. Without the bob the head rides the body at its speed throughout.
#[test]
fn a_crows_head_holds_and_thrusts_once_a_step() {
    let avatar = winged(bird(), crow);
    let rig = &avatar.rig;
    let speed = 0.6;
    let frames = drive(rig, 4.0, 60.0, walking(speed));
    let head = rig.in_zone(Zone::Head)[0];
    // Two seconds walking, past the start.
    let window = &frames[120..240];
    let along: Vec<f32> = window
        .iter()
        .map(|frame| frame.world(rig, head).z)
        .collect();
    let speeds: Vec<f32> = along
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) / STEP)
        .collect();
    // A footfall every half cycle on two feet: the steps the window holds.
    let cycles: f32 = window
        .windows(2)
        .map(|pair| (pair[1].cycle - pair[0].cycle).rem_euclid(1.0))
        .sum();
    let steps = cycles * 2.0;
    let held: Vec<bool> = speeds.iter().map(|&v| v.abs() < 0.03).collect();
    let mut holds = Vec::new();
    let mut run = 0usize;
    for &still in &held {
        if still {
            run += 1;
        } else if run > 0 {
            holds.push(run);
            run = 0;
        }
    }
    let per_step = held.len() as f32 / steps;
    assert!(
        (holds.len() as f32 - steps).abs() <= 1.5,
        "{} holds over {steps:.1} steps",
        holds.len()
    );
    // The first and last runs may be cut by the window.
    for &length in &holds[1..holds.len() - 1] {
        assert!(
            length as f32 >= per_step * 0.3,
            "a hold of {length} frames in a step of {per_step:.1}"
        );
    }
    let fastest = speeds.iter().copied().fold(0.0f32, f32::max);
    assert!(
        fastest > 2.0 * speed,
        "the head thrust at {fastest:.2} m/s, the body going {speed}"
    );
}

/// **The head-bob fades out by a walk's end** (research-11 2.2): all of it to
/// a Froude number of 0.3, none from a half, easing between; and a bird whose
/// kind does not bob - the default owl - never holds its head.
#[test]
fn the_head_bob_fades_by_a_walks_end_and_an_owl_does_not_bob() {
    assert_eq!(bob_share(0.2), 1.0);
    assert_eq!(bob_share(0.3), 1.0);
    assert_eq!(bob_share(0.5), 0.0);
    assert_eq!(bob_share(0.8), 0.0);
    let mut last = 1.0;
    for step in 0..=20 {
        let share = bob_share(0.3 + 0.01 * step as f32);
        assert!(share <= last, "the fade rises at {step}");
        last = share;
    }
    let avatar = winged(bird(), |_| {});
    let rig = &avatar.rig;
    let speed = 0.6;
    let frames = drive(rig, 3.0, 60.0, walking(speed));
    let head = rig.in_zone(Zone::Head)[0];
    let along: Vec<f32> = frames[90..]
        .iter()
        .map(|frame| frame.world(rig, head).z)
        .collect();
    let slowest = along
        .windows(2)
        .map(|pair| (pair[1] - pair[0]) / STEP)
        .fold(f32::MAX, f32::min);
    assert!(
        slowest > 0.5 * speed,
        "an owl's head slowed to {slowest:.2} m/s"
    );
}

/// Each frame's two feet's heights off the ground, the left's and the
/// right's.
fn feet(rig: &Rig, frames: &[Frame]) -> Vec<(f32, f32)> {
    frames
        .iter()
        .map(|frame| {
            (
                frame.foot(rig, Limb::HindLeft),
                frame.foot(rig, Limb::HindRight),
            )
        })
        .collect()
}

/// **A small bird hops at every pace, and a crow walks and then hops**
/// (research-11 2.1): a hop's two feet go together, leaving the ground at
/// once for part of every cycle; a walk's alternate, one down while the other
/// swings and never both up. Without the hop both birds walk.
#[test]
fn a_small_bird_hops_and_a_crow_walks_and_then_hops() {
    let hopping = |pads: &[(f32, f32)]| {
        let together = pads
            .iter()
            .all(|(left, right)| (left - right).abs() < 0.003);
        let airborne = pads
            .iter()
            .filter(|(left, right)| *left > 0.005 && *right > 0.005)
            .count();
        (together, airborne)
    };
    let small = winged(bird(), songbird);
    let frames = drive(&small.rig, 4.0, 60.0, walking(0.4));
    let (together, airborne) = hopping(&feet(&small.rig, &frames[120..]));
    assert!(together, "a small bird's feet part");
    assert!(
        airborne > 30,
        "a small bird is off the ground {airborne} frames of 120"
    );

    let crow = winged(bird(), crow);
    let frames = drive(&crow.rig, 4.0, 60.0, walking(0.6));
    let pads = feet(&crow.rig, &frames[120..]);
    let (_, airborne) = hopping(&pads);
    assert_eq!(airborne, 0, "a walking crow left the ground");
    assert!(
        pads.iter()
            .any(|(left, right)| left.min(*right) < 0.001 && left.max(*right) > 0.01),
        "a walking crow's feet never alternate"
    );
    let frames = drive(&crow.rig, 4.0, 60.0, walking(1.6));
    let (together, airborne) = hopping(&feet(&crow.rig, &frames[150..]));
    assert!(together, "a crow hopping parts its feet");
    assert!(
        airborne > 30,
        "a crow hopping is off the ground {airborne} frames of 90"
    );
}

/// When `limb` comes down over `frames`: the frames its pad goes from over a
/// millimetre and a half to on the ground.
fn landings(rig: &Rig, frames: &[Frame], limb: Limb) -> Vec<usize> {
    let heights: Vec<f32> = frames.iter().map(|frame| frame.pad(rig, limb)).collect();
    heights
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] > 0.0015 && pair[1] <= 0.0015)
        .map(|(index, _)| index + 1)
        .collect()
}

/// **A bat crawls on its wrists and its feet and bounds** (research-11 2.3): a
/// lateral-sequence walk at a crawl, each hind then the fore on its side, its
/// fore contact the wrist's pad and two feet always down; and where a beast
/// would trot, a bound, the hinds landing together and the fores after them.
#[test]
fn a_bat_crawls_on_its_wrists_and_feet_and_bounds() {
    let avatar = winged(Lineage::pure(WingedFamily::Bat), |_| {});
    let rig = &avatar.rig;
    // The fore contact is the wrist's pad: the pad hangs off the wrist.
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        let pad = rig.in_zone(Zone::Extremity(limb))[0];
        let wrist = rig.joints[pad].parent.expect("a pad hangs off its wrist");
        assert_eq!(rig.joints[wrist].zone, Zone::LowerLimb(limb));
    }
    // A walk's pace at its Froude number, over the idle's floor of 0.3 m/s:
    // a vampire bat walks at 0.12 to 0.28 m/s on a hip a third as high.
    let frames = drive(rig, 5.0, 60.0, walking(0.4));
    let window = &frames[120..];
    assert!(
        window
            .iter()
            .all(|frame| frame.pattern == Some(Pattern::Walk)),
        "a crawling bat walks"
    );
    for frame in window {
        let down = Limb::ALL
            .into_iter()
            .filter(|&limb| frame.foot(rig, limb) < 0.002)
            .count();
        assert!(down >= 2, "a crawling bat stood on {down} feet");
    }
    // Each hind, then the fore on its own side: a lateral sequence.
    let mut order: Vec<(usize, Limb)> = Limb::ALL
        .into_iter()
        .flat_map(|limb| {
            landings(rig, window, limb)
                .into_iter()
                .map(move |at| (at, limb))
        })
        .collect();
    order.sort_by_key(|&(at, _)| at);
    let sequence: Vec<Limb> = order.iter().map(|&(_, limb)| limb).collect();
    let next = |limb: Limb| match limb {
        Limb::HindLeft => Limb::ForeLeft,
        Limb::ForeLeft => Limb::HindRight,
        Limb::HindRight => Limb::ForeRight,
        Limb::ForeRight => Limb::HindLeft,
    };
    assert!(sequence.len() >= 8, "only {} landings", sequence.len());
    for pair in sequence.windows(2) {
        assert_eq!(pair[1], next(pair[0]), "a crawl landed {sequence:?}");
    }

    let frames = drive(rig, 4.0, 60.0, walking(1.0));
    let window = &frames[150..];
    assert!(
        window
            .iter()
            .all(|frame| matches!(frame.pattern, Some(Pattern::Bound(_)))),
        "a bat going a metre a second bounds"
    );
    bounds(rig, window, "a bat");
}

/// The hinds land together and the fores after them, a little apart.
fn bounds(rig: &Rig, window: &[Frame], name: &str) {
    let [left, right] = [Limb::HindLeft, Limb::HindRight].map(|limb| landings(rig, window, limb));
    assert!(
        left.len() >= 3,
        "{name}'s hinds landed {} times",
        left.len()
    );
    for (a, b) in left.iter().zip(&right) {
        assert!(a.abs_diff(*b) <= 1, "{name}'s hinds landed at {a} and {b}");
    }
    let fores = landings(rig, window, Limb::ForeLeft);
    for &hind in &left {
        let after = fores.iter().find(|&&fore| fore > hind);
        if let Some(&fore) = after {
            assert!(fore > hind + 1, "{name}'s fore landed with its hinds");
        }
    }
}

/// **A glider bounds where a dog trots** (research-11 2.4): the hinds together
/// and the fores after them, with a moment in the air every cycle; a dog at
/// the same Froude number trots, as it always has.
#[test]
fn a_glider_bounds_and_a_dog_still_trots() {
    let avatar = winged(Lineage::pure(WingedFamily::Glider), |_| {});
    let rig = &avatar.rig;
    let frames = drive(rig, 4.0, 60.0, walking(1.2));
    let window = &frames[150..];
    let froude = 1.2f32 * 1.2 / (9.81 * rig_hip(rig));
    assert!(
        froude > 0.6,
        "the premise: a glider past a walk, at {froude}"
    );
    assert!(
        window
            .iter()
            .all(|frame| matches!(frame.pattern, Some(Pattern::Bound(_)))),
        "a glider at a metre and a fifth a second bounds"
    );
    let gait = symbios_avatar::Speed::new(rig, 1.2).gait(rig);
    assert!(gait.airborne() > 0.0, "a bound has a moment in the air");
    bounds(rig, window, "a glider");

    let mut params = QuadrupedParams::default();
    params.set_lineage(Lineage::pure(Family::Dog));
    let dog = built(AvatarRecord::new("Dog", Archetype::Quadruped(params)));
    let speed = (froude * 9.81 * rig_hip(&dog.rig)).sqrt();
    let frames = drive(&dog.rig, 3.0, 60.0, walking(speed));
    assert_eq!(
        frames.last().and_then(|frame| frame.pattern),
        Some(Pattern::Trot)
    );
}

/// How high a body's hip joint stands at rest.
fn rig_hip(rig: &Rig) -> f32 {
    let chain = rig.limb_chain(Limb::HindLeft).expect("a hind leg");
    rig.joints[chain[0]].position.y
}

/// **No drawn toe passes into the ground through a stride** (#424, from W3):
/// a bird, a bat, a crow, a small bird and two crosses, standing, setting
/// off, walking and running and stopping, never put a toe more than a
/// millimetre and a half under the ground. Before, the gait peeled a foot
/// about its toes' tips and carried them rigidly - a bat's went 14 mm under,
/// a bat-glider cross's 16 and a bird-led cross's 42 the frame it set off.
#[test]
fn no_drawn_toe_passes_into_the_ground() {
    let bodies = [
        ("bird", winged(bird(), |_| {})),
        ("crow", winged(bird(), crow)),
        ("songbird", winged(bird(), songbird)),
        ("bat", winged(Lineage::pure(WingedFamily::Bat), |_| {})),
        (
            "bat x glider",
            winged(
                Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.4),
                |_| {},
            ),
        ),
        (
            "bird x bat",
            winged(
                Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.3),
                |_| {},
            ),
        ),
    ];
    for (name, avatar) in &bodies {
        for speed in [0.5, 1.2] {
            let frames = drive(&avatar.rig, 6.0, 60.0, |t| {
                (if (2.0..5.0).contains(&t) { speed } else { 0.0 }, 0.0)
            });
            for (index, frame) in frames.iter().enumerate() {
                let low = lowest_toe(avatar, &frame.driven.pose);
                assert!(
                    low > -0.0015,
                    "{name} at {speed} m/s, frame {index}: a toe {:.1} mm under the ground",
                    -low * 1000.0
                );
            }
        }
    }
}

/// **Nothing drawn passes into the ground, standing or going** (#433, #440):
/// the three families, a crow and a songbird, each pair crossed just short
/// of the halfway blend, at it and just past it (0.49, 0.5 and 0.51), the
/// two crosses that failed at 0.49 at their neighbours as well and named the
/// other way round (a Bird x Bat at 0.4 and 0.45 and a Bat x Bird at 0.51; a
/// Bat x Glider at 0.45 and a Glider x Bat at 0.51), and three rolls,
/// standing a second and then walking, going at their fast gait (a Froude
/// number of 0.15 and of one on the hip) and stopping on the level at 60
/// frames a second: **no drawn point - skin, membrane, digits and flight
/// feathers, not only the toes - lies more than [`UNDER_GROUND`] under the
/// ground, standing or going** (the rests' and the whole life's bound),
/// every other frame, measured from the ground itself and not from where
/// the body stood.
///
/// Before: a bat's thumb, aimed from its wrist at the hand's node on the
/// ground and longer than that reach, ran on into it, 14.5 mm under standing
/// and going, and two bat-led rolls' 13.1 and 9.6 (#433). And until #440 two
/// crosses were left out of this: a Bird x Bat cross at 0.49 stood with its
/// folded secondaries 82.9 mm under the ground (64.0 at 0.45 and 36.4 at
/// 0.4), carried there as they were laid on its body (`wing::feather`); and
/// a Bat x Glider cross at 0.49 put its tail 21.6 mm into the ground as its
/// back first gathered into a bound, which nothing held up
/// (`winged::ground::keep_clear`).
#[test]
fn nothing_drawn_passes_into_the_ground_standing_or_going() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies: Vec<(String, Avatar)> = vec![
        ("bird".to_string(), winged(bird(), |_| {})),
        ("crow".to_string(), winged(bird(), crow)),
        ("songbird".to_string(), winged(bird(), songbird)),
        ("bat".to_string(), winged(Lineage::pure(Bat), |_| {})),
        ("glider".to_string(), winged(Lineage::pure(Glider), |_| {})),
    ];
    let crosses = [(Bird, Bat), (Bat, Glider), (Glider, Bird)]
        .into_iter()
        .flat_map(|(a, b)| [0.49, 0.5, 0.51].map(|blend| (a, b, blend)))
        .chain([
            (Bird, Bat, 0.4),
            (Bird, Bat, 0.45),
            (Bat, Bird, 0.51),
            (Bat, Glider, 0.45),
            (Glider, Bat, 0.51),
        ]);
    for (a, b, blend) in crosses {
        bodies.push((
            format!("{a:?} x {b:?} {blend}"),
            winged(Lineage::cross(a, b, blend), |_| {}),
        ));
    }
    for seed in 1..=3 {
        let mut rolled = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        rolled.reroll(seed);
        bodies.push((format!("roll {seed}"), built(rolled)));
    }
    let mut failures = Vec::new();
    for (name, avatar) in &bodies {
        let rig = &avatar.rig;
        let frames = drive(rig, 5.0, 60.0, paces(rig));
        failures.extend(under_ground(name, avatar, &frames, Vec3::ZERO));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Standing a second, walking a second and a half, going at the fast gait
/// as long and stopping: a Froude number of 0.15 and of one on `rig`'s hip.
fn paces(rig: &Rig) -> impl Fn(f32) -> (f32, f32) {
    let at = |froude: f32| (froude * 9.81 * rig_hip(rig)).sqrt().max(0.4);
    let (walk, fast) = (at(0.15), at(1.0));
    move |t| {
        let speed = if t < 1.0 {
            0.0
        } else if t < 2.5 {
            walk
        } else if t < 4.0 {
            fast
        } else {
            0.0
        };
        (speed, 0.0)
    }
}

/// Where every other frame of `frames` draws `avatar` more than
/// [`UNDER_GROUND`] under ground rising by `rise` along each axis, standing
/// (its first second) and going: the deepest of each, as a failure's line.
fn under_ground(name: &str, avatar: &Avatar, frames: &[Frame], rise: Vec3) -> Vec<String> {
    let (mut standing, mut going) = ((0.0f32, 0usize), (0.0f32, 0usize));
    for (index, frame) in frames.iter().enumerate().step_by(2) {
        let low = lowest_drawn_over(avatar, &frame.driven.pose, rise);
        let worst = if frame.time < 1.0 {
            &mut standing
        } else {
            &mut going
        };
        if low < worst.0 {
            *worst = (low, index);
        }
    }
    [("standing", standing), ("going", going)]
        .into_iter()
        .filter(|(_, worst)| worst.0 < -UNDER_GROUND)
        .map(|(when, worst)| {
            format!(
                "{name}: {:.1} mm under the ground {when}, frame {}",
                -worst.0 * 1000.0,
                worst.1
            )
        })
        .collect()
}

/// A 20 degree slope's rise a metre, facing up it, down it and across it
/// both ways.
fn slopes() -> [(&'static str, Vec3); 4] {
    let grade = 20.0f32.to_radians().tan();
    [
        ("up the slope", Vec3::Z * grade),
        ("down the slope", -Vec3::Z * grade),
        ("across the slope, its left the higher", Vec3::X * grade),
        ("across the slope, its right the higher", -Vec3::X * grade),
    ]
}

/// The bodies that stand on four legs, which Overlands hands the real
/// terrain: a bat, a glider, their crosses about the halfway blend, and the
/// crosses with a bird that a bat or a glider leads, each named both ways
/// round.
fn bodies_on_four() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies = vec![
        ("bat".to_string(), Lineage::pure(Bat)),
        ("glider".to_string(), Lineage::pure(Glider)),
    ];
    for (a, b, blend) in [
        (Bat, Glider, 0.49),
        (Bat, Glider, 0.5),
        (Glider, Bat, 0.5),
        (Bird, Bat, 0.5),
        (Bat, Bird, 0.49),
        (Bat, Bird, 0.3),
        (Glider, Bird, 0.49),
        (Bird, Glider, 0.5),
    ] {
        bodies.push((
            format!("{a:?} x {b:?} {blend}"),
            Lineage::cross(a, b, blend),
        ));
    }
    bodies
}

/// **Nothing drawn passes into a slope under a body on four legs** (#441): a
/// bat, a glider and every cross one of them leads ([`bodies_on_four`]),
/// standing a second and then walking, going at its fast gait and stopping
/// ([`paces`]) on a 20 degree slope - facing up it, down it and across it
/// both ways - at 30, 60 and 144 frames a second: **no drawn point lies more
/// than [`UNDER_GROUND`] under the slope, standing or going**, every other
/// frame, measured straight down to the slope itself.
///
/// Before, at 60 frames a second: a bat's thumb, built along a level floor
/// ahead of a wrist that stands upright whatever it stands on, 37.3 mm into
/// a slope it faced up, and the membrane's edge beside its wrist 17.0 mm
/// into one rising to either side, 14.3 standing there; a Bat x Bird cross
/// at 0.3's thumb 27.6 mm; a Bat x Glider cross at 0.49's tail 52.7 mm
/// facing up the slope and 27.7 across it, its wrist's membrane 19.1
/// standing across it; and a Bird x Bat cross at 0.5's belly 20.8 mm
/// bounding up it, 11.6 as it stopped. A glider and the crosses it leads
/// stood within the bound but one, a Bird x Glider at 0.5 bounding up the
/// slope 6.3 mm. And with a bat's thumb and wrist laid on the slope, its
/// belly was next: 19.4 mm bounding up it.
#[test]
fn nothing_drawn_passes_into_a_slope_under_a_body_on_four() {
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies_on_four() {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = winged(lineage, |_| {});
                let rig = &avatar.rig;
                assert!(rig.four_legged(), "the premise: {name} stands on four");
                let mut found = Vec::new();
                for (on, rise) in slopes() {
                    for hz in [30.0f32, 60.0, 144.0] {
                        let frames = drive_over(rig, 5.0, |_| 1.0 / hz, paces(rig), rise);
                        let label = format!("{name} {on} at {hz}");
                        found.extend(under_ground(&label, &avatar, &frames, rise));
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far a joint on ground rising one part in ten thousand may stand from
/// where the level floor puts it, in metres: see
/// [`a_slopes_hold_comes_in_from_nothing_and_holds_a_body_still`]. A
/// millimetre, twice what the gaits' own reading of that ground moves one
/// (0.53 mm at the most, before #441 and after: a foot planted a tenth of a
/// millimetre higher is a leg solved a little otherwise).
const NEAR_LEVEL: f32 = 1.0e-3;

/// How much further than standing on the level a joint may move in a frame
/// standing on a slope, in metres a sixtieth of a second.
const STILL_ON_A_SLOPE: f32 = 5.0e-4;

/// **A slope's hold on a body comes in from nothing, and holds it still**
/// (#441): what keeps a body on four legs out of a slope
/// (`winged::ground::keep_clear`: its hands laid along the ground, its
/// belly and its tail lifted off it) answers the ground's own tilt, so on
/// ground that is all but level it does all but nothing, and on a slope it
/// does the same every frame. A bat, a Bat x Glider cross at 0.49 and a
/// Bird x Bat cross at 0.5:
///
/// - **on ground rising one part in ten thousand** - ahead, behind and to
///   either side - standing, walking and bounding at 60 frames a second,
///   **stand every joint within [`NEAR_LEVEL`] of where the level floor
///   puts it**, every frame: real terrain is never exactly level, and a
///   hold keyed on a slope being nought pops at every road's edge (to the
///   stop, after which the idle's own glances fall on other frames on the
///   two grounds, a joint of a bat's 39 mm apart for one of them, as before
///   #441);
/// - **standing on a 20 degree slope**, each way, at 30, 60 and 144 frames
///   a second, **move no joint further in a frame than standing on the
///   level does and [`STILL_ON_A_SLOPE`]**, from half a second in: a body
///   that stands still and flickers moves the same distance every frame;
/// - and **through the whole of it on each slope turn no joint more than
///   [`GROUND_TURN`] in a sixtieth of a second**.
#[test]
fn a_slopes_hold_comes_in_from_nothing_and_holds_a_body_still() {
    use WingedFamily::{Bat, Bird, Glider};
    let bodies = [
        ("bat", Lineage::pure(Bat)),
        ("bat x glider 0.49", Lineage::cross(Bat, Glider, 0.49)),
        ("bird x bat 0.5", Lineage::cross(Bird, Bat, 0.5)),
    ];
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies {
            let failures = &failures;
            scope.spawn(move || {
                let found = slope_hold_failures(name, lineage);
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// [`a_slopes_hold_comes_in_from_nothing_and_holds_a_body_still`] for one
/// body: every bound it breaks.
fn slope_hold_failures(name: &str, lineage: Lineage<WingedFamily>) -> Vec<String> {
    let mut failures = Vec::new();
    let avatar = winged(lineage, |_| {});
    let rig = &avatar.rig;
    let joints = |frames: &[Frame]| -> Vec<(Vec<Vec3>, Vec<Quat>)> {
        frames
            .iter()
            .map(|frame| {
                let posed = frame.driven.pose.forward(rig);
                (posed.positions, posed.rotations)
            })
            .collect()
    };
    // The furthest a joint moves in a frame standing, from half a second in
    // to the second it sets off at, read as a sixtieth's.
    let standing = |frames: &[(Vec<Vec3>, Vec<Quat>)], hz: f32| {
        let (from, to) = ((0.5 * hz) as usize, (1.0 * hz) as usize - 1);
        bound_steps(&frames[..to], from, hz)
            .iter()
            .map(|step| step.0)
            .fold(0.0f32, f32::max)
    };
    let level = joints(&drive(rig, 5.0, 60.0, paces(rig)));
    for (on, rise) in slopes() {
        let near = joints(&drive_over(
            rig,
            5.0,
            |_| 1.0 / 60.0,
            paces(rig),
            rise.normalize() * 1.0e-4,
        ));
        let (apart, frame) = level
            .iter()
            .zip(&near)
            .take(4 * 60)
            .map(|(a, b)| super::winged_bounds::furthest(&a.0, &b.0))
            .enumerate()
            .fold((0.0f32, 0), |most, (frame, apart)| {
                if apart > most.0 { (apart, frame) } else { most }
            });
        if apart > NEAR_LEVEL {
            failures.push(format!(
                "{name} all but level, {on}: a joint {:.2} mm from where the level puts it, \
                 frame {frame}",
                apart * 1000.0
            ));
        }
        for hz in [30.0f32, 60.0, 144.0] {
            let flat = joints(&drive(rig, 1.0, hz, |_| (0.0, 0.0)));
            let sloped = joints(&drive_over(rig, 5.0, |_| 1.0 / hz, paces(rig), rise));
            let (still, moved) = (standing(&flat, hz), standing(&sloped, hz));
            if moved > still + STILL_ON_A_SLOPE {
                failures.push(format!(
                    "{name} standing {on} at {hz}: a joint moved {:.2} mm a sixtieth, {:.2} on \
                     the level",
                    moved * 1000.0,
                    still * 1000.0
                ));
            }
            let turned = bound_steps(&sloped, 1, hz)
                .iter()
                .map(|step| step.1)
                .fold(0.0f32, f32::max);
            if turned > GROUND_TURN {
                failures.push(format!(
                    "{name} {on} at {hz}: a joint turned {turned:.2} rad a sixtieth"
                ));
            }
        }
    }
    failures
}

/// **A bird's tail is one chain** (#425): its tail zone holds the bony stub's
/// five joints and then the fan's two edges at the rump, and [`Rig::tail`]
/// is the stub alone, each joint the next one's parent, its last the tip. A
/// tail swung by a clip turns the stub and leaves the fan's edges to the
/// wings; a beast's tail is its zone.
#[test]
fn a_birds_tail_is_one_chain() {
    let avatar = winged(bird(), |_| {});
    let rig = &avatar.rig;
    let zone = rig.in_zone(Zone::Tail);
    let tail = rig.tail();
    assert_eq!(
        zone.len(),
        7,
        "the premise: a stub of five and two fan edges"
    );
    assert_eq!(tail.len(), 5);
    let wings = rig.wings.as_ref().expect("a bird has wings");
    for pair in tail.windows(2) {
        assert_eq!(rig.joints[pair[1]].parent, Some(pair[0]), "{tail:?}");
    }
    assert!(tail.iter().all(|&joint| !wings.owns(joint)));
    let edges: Vec<usize> = zone.iter().copied().filter(|&j| wings.owns(j)).collect();
    assert_eq!(edges.len(), 2);

    let clip = Clip::new([Track::new(
        Target::Tail,
        [
            Key::new(0.0, Vec3::new(1.0, 0.5, 0.0)),
            Key::new(1.0, Vec3::new(1.0, 0.5, 0.0)),
        ],
    )]);
    let mut pose = Pose::rest(rig);
    clip.apply(rig, &mut pose, 0.5);
    for &edge in &edges {
        assert_eq!(pose.rotations[edge], Quat::IDENTITY, "a fan edge was swung");
    }
    let tip = *tail.last().expect("a tail");
    let moved = pose.forward(rig).positions[tip].distance(rig.joints[tip].position);
    assert!(moved > 0.005, "the stub's tip moved {moved}");

    let mut params = QuadrupedParams::default();
    params.set_lineage(Lineage::pure(Family::Dog));
    let dog = built(AvatarRecord::new("Dog", Archetype::Quadruped(params)));
    assert_eq!(dog.rig.tail(), dog.rig.in_zone(Zone::Tail));
}

/// **A bird's folded wing does not breathe** (#424): a limb that bears no
/// weight is an arm only on a body with hands, and the idle's breath and
/// fidget turned a bird's wing's marker shoulders. Ten seconds standing leave
/// every joint of its wing arms as they rest.
#[test]
fn a_birds_folded_wings_do_not_breathe() {
    let avatar = winged(bird(), |_| {});
    let rig = &avatar.rig;
    let arms: Vec<usize> = [Limb::ForeLeft, Limb::ForeRight]
        .into_iter()
        .flat_map(|limb| rig.in_zone(Zone::UpperLimb(limb)))
        .collect();
    assert!(!arms.is_empty());
    for (index, frame) in drive(rig, 10.0, 60.0, |_| (0.0, 0.0)).iter().enumerate() {
        for &joint in &arms {
            let turned = frame.driven.pose.rotations[joint].angle_between(Quat::IDENTITY);
            assert!(
                turned < 1.0e-5,
                "frame {index}: joint {joint} turned {turned} rad"
            );
        }
    }
}

/// The frame of `frames` at `moment` seconds, if one falls on it.
fn at_moment(frames: &[Frame], moment: f32) -> Option<&Frame> {
    frames
        .iter()
        .find(|frame| (frame.time - moment).abs() < 1.0e-3)
}

/// Each frame's step, of its index.
type Steps = fn(usize) -> f32;

/// How fast a body goes and turns at each moment.
type Plan = dyn Fn(f32) -> (f32, f32);

/// A named plan, and the stretches of it, in seconds, a test reads.
type Scenario<'a> = (&'a str, &'a Plan, &'a [(f32, f32)]);

/// The cycles `frames` stepped through from `from` to `to` seconds, counted
/// on: each frame's clock against the last's, forward.
fn stepped(frames: &[Frame], from: f32, to: f32) -> f32 {
    frames
        .windows(2)
        .filter(|pair| pair[1].time > from + 1.0e-4 && pair[1].time <= to + 1.0e-4)
        .map(|pair| (pair[1].cycle - pair[0].cycle).rem_euclid(1.0))
        .sum()
}

/// **A turning body's clock goes the same at every frame rate** (#415): a
/// pivot from a standstill and a spin - 2.5 rad/s, then back at 0.6 - at 90,
/// 144 and 240 frames a second and at steps of a fortieth and a hundred and
/// twentieth of a second in turn steps a dog, a horse, a person, a bird and a
/// bat through within a twentieth of a cycle of what it steps at 60 while the
/// turn eases, counted from after each turn has begun - a moment that falls
/// between frames - so what is read is the clock's rate and not where a
/// frame happened to start it. Before, a four-legged body's pivot clock
/// gained a twentieth of its stance every frame its turn eased, two and a
/// half times as much a second at 144 frames as at 60, and a horse's feet
/// stood 605 mm apart; held to the reference rate's gain a second while the
/// leading foot was down but not while it swung, a bat's clock still stepped
/// 0.18 of a cycle more in a spin's second and a half at 240 frames a second.
#[test]
fn a_turning_bodys_clock_goes_the_same_at_every_frame_rate() {
    let beast = |family: Family| {
        let mut params = QuadrupedParams::default();
        params.set_lineage(Lineage::pure(family));
        built(AvatarRecord::new("Beast", Archetype::Quadruped(params)))
    };
    let bodies = [
        ("dog", beast(Family::Dog)),
        ("horse", beast(Family::Horse)),
        (
            "person",
            built(AvatarRecord::new("Person", Archetype::default())),
        ),
        ("bird", winged(bird(), |_| {})),
        ("bat", winged(Lineage::pure(WingedFamily::Bat), |_| {})),
    ];
    let pivot = |t: f32| (0.0f32, if (1.0..4.0).contains(&t) { 1.0f32 } else { 0.0 });
    let spin = |t: f32| {
        let turn = match t {
            t if t < 1.0 => 0.0,
            t if t < 3.0 => 2.5,
            t if t < 6.0 => -0.6,
            _ => 0.0f32,
        };
        (0.0f32, turn)
    };
    let rates: [(&str, Steps); 4] = [
        ("90", |_| 1.0 / 90.0),
        ("144", |_| 1.0 / 144.0),
        ("240", |_| 1.0 / 240.0),
        ("40 and 120", |frame| {
            if frame % 2 == 0 {
                1.0 / 40.0
            } else {
                1.0 / 120.0
            }
        }),
    ];
    // While each turn eases, from a sixth of a second after it is asked.
    let windows: [Scenario; 2] = [
        ("pivot", &pivot, &[(1.5, 4.0)]),
        ("spin", &spin, &[(1.5, 3.0), (4.0, 6.0)]),
    ];
    for (name, avatar) in &bodies {
        let rig = &avatar.rig;
        for &(scenario, plan, spans) in &windows {
            let reference = drive_steps(rig, 7.0, |_| 1.0 / 60.0, plan);
            for (rate, steps) in rates {
                let frames = drive_steps(rig, 7.0, steps, plan);
                for &(from, to) in spans {
                    let (a, b) = (stepped(&reference, from, to), stepped(&frames, from, to));
                    if std::env::var("W5_DIAG").is_ok() {
                        println!(
                            "{name} {scenario} {rate} {from}: {a:.3} {b:.3} {:.3}",
                            b - a
                        );
                    }
                    assert!(
                        (a - b).abs() < CLOCK_APART,
                        "{name} {scenario} at {rate} frames a second, {from} to {to} s: \
                         {b:.3} cycles against {a:.3} at 60"
                    );
                }
            }
        }
    }
}

/// How far apart two frame rates' clocks may step through the same stretch
/// of a turn, in cycles: see
/// [`a_turning_bodys_clock_goes_the_same_at_every_frame_rate`].
const CLOCK_APART: f32 = 0.05;

/// **Motion goes the same at every frame rate** (#415): the same pivot and
/// spin, read at every sixth of a second the rates share with 60, put every
/// joint of each body within 20 cm of where it stands at 60 - the residue of
/// a turn starting and stopping between frames, which puts a person's hand
/// 140 mm apart where its spin reverses. Before #415, a horse's feet stood
/// 605 mm apart.
#[test]
fn motion_goes_the_same_at_every_frame_rate() {
    let mut params = QuadrupedParams::default();
    params.set_lineage(Lineage::pure(Family::Horse));
    let horse = built(AvatarRecord::new("Horse", Archetype::Quadruped(params)));
    let rig = &horse.rig;
    let pivot = |t: f32| (0.0f32, if (1.0..4.0).contains(&t) { 1.0f32 } else { 0.0 });
    let reference = drive_steps(rig, 7.0, |_| 1.0 / 60.0, pivot);
    for steps in [(|_| 1.0 / 144.0) as fn(usize) -> f32, |frame| {
        if frame % 2 == 0 {
            1.0 / 40.0
        } else {
            1.0 / 120.0
        }
    }] {
        let frames = drive_steps(rig, 7.0, steps, pivot);
        for sixth in 0..42 {
            let moment = sixth as f32 / 6.0;
            let (Some(a), Some(b)) = (at_moment(&reference, moment), at_moment(&frames, moment))
            else {
                continue;
            };
            let (pa, pb) = (a.driven.pose.forward(rig), b.driven.pose.forward(rig));
            let apart = pa
                .positions
                .iter()
                .zip(&pb.positions)
                .map(|(p, q)| p.distance(*q))
                .fold(0.0f32, f32::max);
            assert!(
                apart < 0.2,
                "{moment:.2} s: a joint {:.0} mm apart",
                apart * 1000.0
            );
        }
    }
}

/// **A bat sets off into its crawl and its bound without a snap** (#424): from
/// standing into 0.32 and 0.5 m/s and into a bound at 1.0, no joint of its
/// body moves more than 45 mm in a frame in the body, and its elbows stay up
/// and out over its wrists as the crawl carries them. Before, its stride was
/// centred under its blades as a beast's is: its wrists were drawn back 23 cm
/// and its elbows folded in over its spine, 109 mm in a frame; and its
/// trailing foot, folded in the swing as a paw's toes are, flicked its toes
/// up through 55 mm a frame.
#[test]
fn a_bat_sets_off_without_a_snap() {
    let avatar = winged(Lineage::pure(WingedFamily::Bat), |_| {});
    let rig = &avatar.rig;
    let wings = |joint: usize| rig.wings.as_ref().is_some_and(|wings| wings.owns(joint));
    let elbows =
        [Limb::ForeLeft, Limb::ForeRight].map(|limb| rig.limb_chain(limb).expect("a fore leg")[1]);
    for speed in [0.32, 0.5, 1.0] {
        let frames = drive(rig, 4.0, 60.0, walking(speed));
        let mut before: Option<Vec<Vec3>> = None;
        for (index, frame) in frames.iter().enumerate() {
            let now = frame.driven.pose.forward(rig).positions;
            if let Some(before) = &before {
                for joint in (0..now.len()).filter(|&joint| !wings(joint)) {
                    let moved = now[joint].distance(before[joint]);
                    assert!(
                        moved < 0.045,
                        "at {speed} m/s, frame {index}: joint {joint} moved {:.0} mm",
                        moved * 1000.0
                    );
                }
            }
            for &elbow in &elbows {
                assert!(
                    now[elbow].x.abs() > 0.15,
                    "at {speed} m/s, frame {index}: an elbow {:.0} mm off the spine",
                    now[elbow].x.abs() * 1000.0
                );
            }
            before = Some(now);
        }
    }
}

/// **A bird steps as evenly over uneven frames as over even ones** (#424):
/// walking and running from a stand and stopping, at steps of a fortieth and
/// a hundred and twentieth of a second in turn, no joint of a bird or a
/// bird-led cross moves in a frame more than twice the most it moved in the
/// four frames either side and a hundredth of its trunk, each frame read at
/// the speed it moved. Before, its foot's peel was solved in a beast's
/// pantograph while the rest of its stride stood the foot, and its knee
/// swung 52 mm out of its own bend and back in one frame as each heel lifted;
/// and a stop's blend read a long frame's velocity over a short one.
#[test]
fn a_bird_steps_evenly_over_uneven_frames() {
    let uneven = |frame: usize| {
        if frame.is_multiple_of(2) {
            1.0 / 40.0
        } else {
            1.0 / 120.0
        }
    };
    for (name, lineage) in [
        ("bird", bird()),
        (
            "glider x bird 0.5",
            Lineage::cross(WingedFamily::Glider, WingedFamily::Bird, 0.5),
        ),
    ] {
        let avatar = winged(lineage, |_| {});
        let rig = &avatar.rig;
        let pelvis = rig.joints[rig.in_zone(Zone::Pelvis)[0]].position;
        let withers = rig.joints[*rig.in_zone(Zone::Chest).last().expect("a chest")].position;
        let slack = 0.01 * pelvis.distance(withers);
        for speed in [0.6, 1.3] {
            let frames = drive_steps(rig, 5.0, uneven, |t| {
                (if (1.0..4.0).contains(&t) { speed } else { 0.0 }, 0.0)
            });
            let joints: Vec<Vec<Vec3>> = frames
                .iter()
                .map(|frame| frame.driven.pose.forward(rig).positions)
                .collect();
            let deltas: Vec<f32> = (0..frames.len()).map(uneven).collect();
            let pops = pops_over(&joints, &deltas, 1.0 / 60.0, slack);
            assert!(pops.is_empty(), "{name} at {speed} m/s: {pops:?}");
        }
    }
}

/// **A glider's paw stays on the ground as its pace changes** (#424): setting
/// off, breaking into a bound and stopping out of it, at 60 frames a second
/// and over uneven ones, no drawn point of a glider or a glider-led cross
/// goes more than 6 mm under the ground. Before, the blend into each change
/// rolled its forepaw and put its sole 6 to 13 mm under with its pad on the
/// ground, as a beast's paw is (a dog's 44 mm, which is left to it).
#[test]
fn a_gliders_paw_stays_on_the_ground_as_its_pace_changes() {
    use super::winged_bounds::UNDER_GROUND;
    for (name, lineage) in [
        ("glider", Lineage::pure(WingedFamily::Glider)),
        (
            "bat x glider 0.5",
            Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.5),
        ),
    ] {
        let avatar = winged(lineage, |_| {});
        let rig = &avatar.rig;
        let chain = rig.limb_chain(Limb::HindLeft).expect("a hind leg");
        let hip = rig.joints[chain[0]].position.y;
        let (walk, bound) = ((0.15 * 9.81 * hip).sqrt().max(0.4), (9.81 * hip).sqrt());
        let plan = move |t: f32| {
            let speed = match t {
                t if t < 1.0 => 0.0,
                t if t < 3.0 => walk,
                t if t < 5.0 => bound,
                _ => 0.0,
            };
            (speed, 0.0)
        };
        for (rate, steps) in [
            ("60", (|_| 1.0 / 60.0) as fn(usize) -> f32),
            ("40 and 120", |frame| {
                if frame.is_multiple_of(2) {
                    1.0 / 40.0
                } else {
                    1.0 / 120.0
                }
            }),
        ] {
            for (index, frame) in drive_steps(rig, 6.5, steps, plan).iter().enumerate() {
                let low = lowest_drawn(&avatar, &frame.driven.pose);
                assert!(
                    low > -UNDER_GROUND,
                    "{name} at {rate}, frame {index}: {:.1} mm under the ground",
                    -low * 1000.0
                );
            }
        }
    }
}

/// **A small four-legged body's paws turn no faster than a beast's** (#424):
/// a glider walking, bounding at 1.5 and at 3 m/s from a stand, turns no
/// joint more than 1.25 rad in a frame at 60 frames a second - the flight's
/// bound. Before, folding a forepaw as a dog's over a glider's swing of a
/// tenth of a second turned its toes 2.8 rad in a frame, where a dog at the
/// same Froude number turns 1.3.
#[test]
fn a_gliders_paws_turn_no_faster_than_a_beasts() {
    let avatar = winged(Lineage::pure(WingedFamily::Glider), |_| {});
    let rig = &avatar.rig;
    for speed in [0.6, 1.5, 3.0] {
        let frames = drive(rig, 4.0, 60.0, walking(speed));
        for (index, pair) in frames.windows(2).enumerate() {
            let (a, b) = (
                pair[0].driven.pose.forward(rig).rotations,
                pair[1].driven.pose.forward(rig).rotations,
            );
            let turn = a
                .iter()
                .zip(&b)
                .map(|(a, b)| turn_between(*a, *b))
                .fold(0.0f32, f32::max);
            assert!(
                turn < 1.25,
                "at {speed} m/s, frame {}: a joint turned {turn:.2} rad",
                index + 1
            );
        }
    }
}

/// **A bird's folded wing rides no lower running than it stands** (#424): a
/// Bird x Bat cross at 0.3, whose long folded feathers the hug lays down its
/// flank to 6.6 mm over the ground, running at 3 m/s takes no point of them
/// more than 6 mm under where they stood, and nothing drawn that far under
/// the ground. Before, its crouch and lean took them 10.4 mm lower, 3.8 mm
/// into the ground (`winged::ground::hold_wings` left out; 14 mm at 0.49
/// until #440, when that cross's feathers stood 83 mm under the ground).
///
/// At 0.3 because a cross with more of a bat's forearm folds its wing back
/// along the body and over the rump (#440, `wing::feather`), where running
/// does not lower it: at 0.49 it stood at 58 mm and ran no lower without
/// the hold, and laid by #440's first fold 5.4 mm lower, under the bound
/// either way.
#[test]
fn a_birds_folded_wing_rides_no_lower_running_than_it_stands() {
    use super::winged_bounds::UNDER_GROUND;
    let avatar = winged(
        Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.3),
        |_| {},
    );
    let rig = &avatar.rig;
    let feathers = &avatar.parts.wings.as_ref().expect("wings").feathers;
    let wing = |pose: &Pose| {
        let moved = pose.forward(rig).deform_mesh(rig, feathers);
        moved
            .positions
            .iter()
            .map(|at| at.y)
            .fold(f32::MAX, f32::min)
    };
    let frames = drive(rig, 5.0, 60.0, walking(3.0));
    let standing = wing(&frames[30].driven.pose);
    assert!(
        standing > 0.0,
        "the premise: its folded feathers stand {:.1} mm over the ground",
        standing * 1000.0
    );
    for (index, frame) in frames.iter().enumerate().skip(60).step_by(2) {
        let low = wing(&frame.driven.pose);
        assert!(
            low > standing - UNDER_GROUND,
            "frame {index}: its feathers {:.1} mm over the ground, standing {:.1}",
            low * 1000.0,
            standing * 1000.0
        );
        let low = lowest_drawn(&avatar, &frame.driven.pose);
        assert!(
            low > -UNDER_GROUND,
            "frame {index}: {:.1} mm under the ground",
            -low * 1000.0
        );
    }
}

/// The least any joint of each foot stands over where it rests, the most of
/// those over the four feet or two - how high the last foot down is - and
/// the spine's flex, the chest's pitch against the pelvis about the body's
/// side, in radians: each frame from `from` on.
fn airborne_and_flex(rig: &Rig, frames: &[Frame], from: usize) -> Vec<(f32, f32)> {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let girdle = rig.joints[rig.in_zone(Zone::Neck)[0]]
        .parent
        .expect("a girdle");
    frames[from..]
        .iter()
        .map(|frame| {
            let lowest = rig
                .ground_contacts()
                .into_iter()
                .map(|limb| frame.foot(rig, limb))
                .fold(f32::MAX, f32::min);
            let posed = frame.driven.pose.forward(rig);
            let (axis, angle) =
                (posed.rotations[pelvis].inverse() * posed.rotations[girdle]).to_axis_angle();
            (lowest, angle * axis.x)
        })
        .collect()
}

/// **A bat's and a glider's bound reads as a bound** (#424, the look
/// review): bounding at 1.0 and 1.2 m/s, each body has every foot more than
/// a tenth of its hip's height off the ground for part of every cycle, and
/// its back folds and stretches through more than half a radian. Before, at
/// a beast's stride and duty, its feet never all left the ground by more
/// than 3 mm and its back moved through 0.4 rad: a fast walk.
#[test]
fn a_bats_and_a_gliders_bound_leaves_the_ground_and_flexes_its_back() {
    for (name, family, speed) in [
        ("bat", WingedFamily::Bat, 1.0),
        ("glider", WingedFamily::Glider, 1.2),
    ] {
        let avatar = winged(Lineage::pure(family), |_| {});
        let rig = &avatar.rig;
        let frames = drive(rig, 4.0, 60.0, walking(speed));
        let read = airborne_and_flex(rig, &frames, 180);
        let up = read.iter().map(|&(low, _)| low).fold(0.0f32, f32::max);
        let off = read
            .iter()
            .filter(|&&(low, _)| low > 0.1 * rig_hip(rig))
            .count();
        let (least, most) = read
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &(_, flex)| {
                (lo.min(flex), hi.max(flex))
            });
        assert!(
            off >= 10,
            "{name}: every foot off the ground by a tenth of its hip {off} frames of 60, \
             {:.1} mm at most",
            up * 1000.0
        );
        assert!(
            most - least > 0.55,
            "{name}: its back flexed through {:.2} rad",
            most - least
        );
    }
}

/// **A small bird's hop is one a viewer can see** (#424, the look review):
/// hopping at 0.4 m/s a songbird-sized bird clears the ground with both feet
/// by more than two fifths of its hip's height at the top of every hop, and
/// its head swings through more than a third of its leg along its walk,
/// back as it lands and out as it springs. Before, at a run's step, it
/// cleared a quarter and its head swung a sixth: bouncing on the spot.
#[test]
fn a_small_birds_hop_clears_the_ground_and_swings_its_head() {
    let avatar = winged(bird(), songbird);
    let rig = &avatar.rig;
    let frames = drive(rig, 4.0, 60.0, walking(0.4));
    let read = airborne_and_flex(rig, &frames, 180);
    let up = read.iter().map(|&(low, _)| low).fold(0.0f32, f32::max);
    let hip = rig_hip(rig);
    assert!(
        up > 0.4 * hip,
        "both feet up {:.1} mm at most, {:.0}% of the hip",
        up * 1000.0,
        up / hip * 100.0
    );
    let head = rig.in_zone(Zone::Head)[0];
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let reach: Vec<f32> = frames[180..]
        .iter()
        .map(|frame| {
            let posed = frame.driven.pose.forward(rig);
            posed.positions[head].z - posed.positions[pelvis].z
        })
        .collect();
    let swing = reach.iter().copied().fold(f32::MIN, f32::max)
        - reach.iter().copied().fold(f32::MAX, f32::min);
    let leg = rig.limb_reach(Limb::HindLeft).expect("a leg");
    assert!(
        swing > leg / 3.0,
        "the head swung {:.1} mm, a leg {:.1}",
        swing * 1000.0,
        leg * 1000.0
    );
}

/// The most any joint may turn in a sixtieth of a second on the ground, in
/// radians: `winged_life`'s bound.
const GROUND_TURN: f32 = 1.25;

/// How much more than twice the most it turned in the frames around it a
/// joint may turn in a sixtieth: a fiftieth of a radian, the turns' slack for
/// what the pop bound's hundredth of the trunk is to positions. Read on
/// normalised rotations (#458), no bound's frame turns past twice the most
/// around it at all; read raw, the noise a twentieth of a radian was set to
/// take came to 0.07.
const TURN_SLACK: f32 = 0.02;

/// Two seconds of a bound at `hz`, at its Froude number of one on the
/// hip (as `winged_life` drives it), its chassis carried along (`carried`)
/// or held where it stands while its velocity still asks for the bound - a
/// chassis a wall holds, or a host that hands the driver no position, as
/// `winged_landing`'s does - on ground rising by `rise` along each axis (the
/// level at none): each frame's joints and their turns.
fn bounding(rig: &Rig, hz: f32, carried: bool, rise: Vec3) -> Vec<(Vec<Vec3>, Vec<Quat>)> {
    let delta = 1.0 / hz;
    let speed = (9.81 * rig_hip(rig)).sqrt().max(0.4);
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

/// Each frame's step after the first `from`: how far the joints moved and
/// turned into it, the most of them, read as a sixtieth's at `hz`.
fn bound_steps(frames: &[(Vec<Vec3>, Vec<Quat>)], from: usize, hz: f32) -> Vec<(f32, f32)> {
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
                .map(|(a, b)| turn_between(*a, *b))
                .fold(0.0f32, f32::max);
            (moved * scale, turned * scale)
        })
        .collect()
}

/// The median of `values`.
fn median_of(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

/// **A bound steps without a pop, carried or held** (#437): a bat, a glider
/// and two crosses a bat leads, bounding at a Froude number of one on the hip
/// at 30, 60 and 144 frames a second, on the level and facing up, down and
/// across a 20 degree slope, their chassis carried along at the bound's
/// speed and held where it stands while the velocity still asks for it,
/// every frame from the half second in read as a sixtieth at the speed it
/// moved:
///
/// - **no joint moves in a frame more than twice what a median frame of the
///   carried bound moves it on the same ground at the same rate, and a
///   hundredth of the trunk** - [`SLOPE_PEAK`] times it on the slope, where
///   a carried bound's own smooth peak stands up to 2.2 times its median (a
///   bat x bird's down it at 144 frames a second, 43.9 mm a sixtieth against
///   20.2), its gait's and not a hold's;
/// - **nor more than twice the most it moved in the four frames either side
///   and a hundredth of the trunk** (the pop bound, `winged_life`'s);
/// - **no joint turns in a frame more than twice the most it turned in the
///   four frames either side and [`TURN_SLACK`], nor more than
///   [`GROUND_TURN`]**.
///
/// Before: the foothold ledger let each held foot go at once as it lifted,
/// from the yield's bound ahead of the stride's answer, so a held bat's
/// wrist jumped 98 mm in one frame and its wing arm 115 at every rate (top
/// raw steps 116.9, 117.9 and 114.5 mm against a carried median of 40, 20 and
/// 8), and its hind feet 65. And with the gap carried into the swing along
/// the level alone, its height read off the ground under the path the
/// stride would have swung, a held foot on a slope dropped or rose by the
/// gap times the grade as it lifted: a held bat facing down the slope moved
/// a joint 112.7 mm a sixtieth at 144 frames a second, eight frames popping,
/// a bat x glider 266.8.
#[test]
fn a_bound_steps_without_a_pop_carried_or_held() {
    use WingedFamily::{Bat, Bird, Glider};
    let grade = 20.0f32.to_radians().tan();
    let grounds = [
        ("on the level", Vec3::ZERO),
        ("up the slope", Vec3::Z * grade),
        ("down the slope", -Vec3::Z * grade),
        ("across the slope", Vec3::X * grade),
    ];
    let bodies = [
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
        ("bat x glider 0.5", Lineage::cross(Bat, Glider, 0.5)),
    ];
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies {
            let failures = &failures;
            scope.spawn(move || {
                let found = bound_failures(name, lineage, &grounds);
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How many times what a median frame of a carried bound moves a joint
/// [`a_bound_steps_without_a_pop_carried_or_held`] lets a frame on the slope
/// move it: over the carried bound's own smooth peak there (2.2 times its
/// median at the most), and under the pops a hold put there (a held bat
/// facing down the slope 5.8 times, a bat x glider 7.5).
const SLOPE_PEAK: f32 = 2.5;

/// [`a_bound_steps_without_a_pop_carried_or_held`] for one body: every bound
/// it breaks, on each of `grounds`.
fn bound_failures(
    name: &str,
    lineage: Lineage<WingedFamily>,
    grounds: &[(&str, Vec3)],
) -> Vec<String> {
    let mut failures = Vec::new();
    let avatar = winged(lineage, |_| {});
    let rig = &avatar.rig;
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    let slack = 0.01
        * rig.joints[pelvis]
            .position
            .distance(rig.joints[withers].position);
    for &(on, rise) in grounds {
        for hz in [30.0f32, 60.0, 144.0] {
            let from = (0.5 * hz).round() as usize;
            let carried = bound_steps(&bounding(rig, hz, true, rise), from, hz);
            let moved = median_of(carried.iter().map(|step| step.0).collect());
            for (how, held) in [("carried", false), ("held", true)] {
                let frames = bounding(rig, hz, !held, rise);
                let steps = bound_steps(&frames, from, hz);
                let joints: Vec<Vec<Vec3>> =
                    frames[from..].iter().map(|frame| frame.0.clone()).collect();
                let label = format!("{name} {how} {on} at {hz}");
                let most = steps.iter().map(|step| step.0).fold(0.0f32, f32::max);
                let peak = if rise == Vec3::ZERO { 2.0 } else { SLOPE_PEAK };
                if most > peak * moved + slack {
                    failures.push(format!(
                        "{label}: a frame moved {:.1} mm a sixtieth, a median carried frame {:.1}",
                        most * 1000.0,
                        moved * 1000.0
                    ));
                }
                for at in 4..steps.len().saturating_sub(4) {
                    let turn = steps[at].1;
                    let around = (at - 4..at)
                        .chain(at + 1..=at + 4)
                        .map(|frame| steps[frame].1)
                        .fold(0.0f32, f32::max);
                    if turn > (2.0 * around + TURN_SLACK).min(GROUND_TURN) {
                        failures.push(format!(
                            "{label}: frame {at} turned a joint {turn:.3} rad a sixtieth against \
                             {around:.3}"
                        ));
                    }
                }
                let deltas = vec![1.0 / hz; joints.len()];
                for (at, step, around) in pops_over(&joints, &deltas, 1.0 / 60.0, slack) {
                    failures.push(format!(
                        "{label}: frame {at} moved {:.1} mm a sixtieth against {:.1}",
                        step * 1000.0,
                        around * 1000.0
                    ));
                }
            }
        }
    }
    failures
}
