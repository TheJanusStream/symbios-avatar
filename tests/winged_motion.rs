//! Winged bodies on the ground (#424): the bird's walk with its head-bob, its
//! hop and its run, the bat's crawl and bound, the glider's bound; the drawn
//! toes on the ground through a stride; a bird's tail as one chain; a bird's
//! folded wings left out of the idle's breath; and motion that goes the same
//! at 60 and at 144 frames a second (#415).
//!
//! Every body is driven through [`Driver::drive`], as an application drives
//! one, carried by a chassis that steps it at the speed asked.

use super::winged_bounds::{lowest_drawn, lowest_toe, pops_over};
use symbios_avatar::anim::driver::{Driven, Driver, Inputs, level_ground, velocity_of};
use symbios_avatar::anim::winged::ground::bob_share;
use symbios_avatar::anim::{Clip, Key, Pattern, Target, Track};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Family, Limb, Pose, QuadrupedParams, Quat, Rig,
    Vec3, WingedFamily, WingedParams, Zone,
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
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta,
                    velocity,
                    at,
                    facing,
                    turn,
                    ..Inputs::default()
                },
                level_ground,
            )
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
                .map(|(a, b)| a.angle_between(*b))
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
/// Bird x Bat cross at 0.49 stands with its folded primaries 83 mm under the
/// ground - its build's (#433) - and running at 3 m/s takes no drawn point
/// more than 6 mm under where it stood. Before, its crouch and lean took
/// them 14 mm further.
#[test]
fn a_birds_folded_wing_rides_no_lower_running_than_it_stands() {
    use super::winged_bounds::UNDER_GROUND;
    let avatar = winged(
        Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.49),
        |_| {},
    );
    let frames = drive(&avatar.rig, 5.0, 60.0, walking(3.0));
    let standing = lowest_drawn(&avatar, &frames[30].driven.pose).min(0.0);
    for (index, frame) in frames.iter().enumerate().skip(60).step_by(2) {
        let low = lowest_drawn(&avatar, &frame.driven.pose);
        assert!(
            low > standing - UNDER_GROUND,
            "frame {index}: {:.1} mm under, standing {:.1}",
            -low * 1000.0,
            -standing * 1000.0
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
