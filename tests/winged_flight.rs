//! Winged bodies in the air (#424): every flight mode is reached and no
//! change between them pops; the beat slows with size as mass to the -0.28;
//! a takeoff's legs give most of its lift-off speed before its wings beat; a
//! flare begins by time to contact; a glider never flaps; a bat's hand folds
//! flatter than 149 degrees and still never flips; a spread bird has cards
//! under its wing; and a body with no wings, or with no flight asked, moves
//! exactly as it did.

use super::winged_bounds::{UNDER_GROUND, furthest, lowest_drawn, pops, turn_between};
use symbios_avatar::anim::driver::{Carriage, Driver, DriverConfig, Inputs, Source, level_ground};
use symbios_avatar::anim::winged::flight::{
    FLARE_TAU, Flight, FlightMode, Flown, LEG_SHARE, RATE_EXPONENT,
};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Limb, MeshKind, Pose, QuadrupedParams, Rig, Stroke, Vec3,
    WingedFamily, WingedParams, Zone,
};

/// Frames at a hundred and twenty a second.
const STEP: f32 = 1.0 / 120.0;

fn winged(lineage: Lineage<WingedFamily>) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    AvatarRecord::new("Winged", Archetype::Winged(params))
}

fn built(record: &AvatarRecord) -> Avatar {
    Avatar::build_for_shape(record).expect("a winged body builds")
}

/// The families, and a cross of each pair either side of the halfway blend.
fn lineages() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut all: Vec<(String, Lineage<WingedFamily>)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), Lineage::pure(family)))
        .collect();
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.3, 0.7] {
            all.push((
                format!("{a:?} x {b:?} {blend}"),
                Lineage::cross(a, b, blend),
            ));
        }
    }
    all
}

/// The trunk's length, shoulder to hip, as the flight reads it.
fn trunk(rig: &Rig) -> f32 {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    rig.joints[pelvis]
        .position
        .distance(rig.joints[withers].position)
}

/// A frame of flight, or of standing where `mode` is `None`.
fn inputs(mode: Option<Flight>, speed: f32) -> Inputs<'static> {
    Inputs {
        delta: STEP,
        velocity: Vec3::Z * speed,
        flight: mode,
        ..Inputs::default()
    }
}

/// A script: each segment's flight and how long it lasts.
type Script = Vec<(Option<Flight>, f32, f32)>;

/// Every mode, one after another, from standing to standing again.
fn every_mode() -> Script {
    let mode = |mode| Some(Flight::new(mode));
    vec![
        (None, 0.4, 0.0),
        (mode(FlightMode::Takeoff), 0.6, 2.5),
        (mode(FlightMode::Flap), 0.83, 4.0),
        (mode(FlightMode::Glide), 0.51, 6.0),
        (mode(FlightMode::Soar), 0.47, 6.0),
        (mode(FlightMode::Flap), 0.38, 5.0),
        (mode(FlightMode::Dive), 0.55, 9.0),
        (mode(FlightMode::Flap), 0.29, 6.0),
        (mode(FlightMode::Glide), 0.31, 6.0),
        (mode(FlightMode::Flare), 0.45, 4.0),
        (mode(FlightMode::Land), 0.3, 0.0),
        (None, 0.5, 0.0),
    ]
}

/// What a script drives: each frame's joint positions, the drawn vertices of
/// its wing parts and body, and the mode it was posed in.
struct Run {
    joints: Vec<Vec<Vec3>>,
    modes: Vec<Option<FlightMode>>,
    sources: Vec<Source>,
    switches: Vec<usize>,
}

fn run(avatar: &Avatar, script: &Script, carriage: Carriage) -> Run {
    let rig = &avatar.rig;
    let mut driver = Driver::new(
        DriverConfig {
            carriage,
            ..DriverConfig::default()
        },
        7,
    );
    let mut out = Run {
        joints: Vec::new(),
        modes: Vec::new(),
        sources: Vec::new(),
        switches: Vec::new(),
    };
    for &(flight, seconds, speed) in script {
        out.switches.push(out.joints.len());
        for _ in 0..(seconds / STEP).round() as usize {
            let driven = driver
                .drive(rig, &inputs(flight, speed), level_ground)
                .expect("a driven frame");
            out.joints.push(driven.pose.forward(rig).positions);
            out.modes.push(driver.flight(rig).map(|flown| flown.mode));
            out.sources.push(driven.source);
        }
    }
    out
}

/// How far each joint moved into frame `at`, the most of them.
fn step_at(joints: &[Vec<Vec3>], at: usize) -> f32 {
    furthest(&joints[at - 1], &joints[at])
}

/// **Every mode is reached, and no change between them pops** (#424): over
/// the whole script, from standing through a takeoff, flapping, a glide, a
/// soar, a dive, a flare and a landing back onto the feet, every family and
/// cross is posed in each mode the script asks of it, and at every change
/// of mode no joint moves further in the frame than twice the most it moved
/// in the five frames either side, and a few millimetres.
/// Snapped to each mode's targets instead of eased, a bird's joints moved
/// 0.19 m in the frame its flapping became a glide, against 0.05 around it.
#[test]
fn every_mode_is_reached_and_no_change_of_mode_pops() {
    let script = every_mode();
    let mut failures = Vec::new();
    for (name, lineage) in lineages() {
        let avatar = built(&winged(lineage));
        let flaps = avatar
            .rig
            .wings
            .as_ref()
            .unwrap()
            .kinship(&avatar.rig)
            .flaps();
        for carriage in [Carriage::Chassis, Carriage::Own] {
            let run = run(&avatar, &script, carriage);
            for &(flight, ..) in &script {
                let Some(flight) = flight else { continue };
                let wanted = match flight.mode {
                    FlightMode::Flap if !flaps => FlightMode::Glide,
                    mode => mode,
                };
                assert!(
                    run.modes.contains(&Some(wanted)),
                    "{name} {carriage:?}: never posed in {wanted:?}"
                );
            }
            assert!(run.sources.contains(&Source::Fly), "{name}: never flew");
            let slack = 0.02 * trunk(&avatar.rig);
            for &at in &run.switches[1..] {
                let jump = step_at(&run.joints, at);
                let around = (at - 5..at)
                    .chain(at + 1..=at + 5)
                    .filter(|&frame| frame >= 1 && frame < run.joints.len())
                    .map(|frame| step_at(&run.joints, frame))
                    .fold(0.0f32, f32::max);
                if jump > around * 2.0 + slack {
                    failures.push(format!(
                        "{name} {carriage:?} at frame {at}: {jump:.4} m against {around:.4} m around it"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **Every change between two modes of the air is continuous**, at any
/// point of a beat: each ordered pair of flapping, gliding, soaring, diving
/// and flaring, switched at five points of the beat, moves no joint further
/// in the switching frame than the bound above.
#[test]
fn every_pair_of_air_modes_changes_without_a_pop() {
    use FlightMode::{Dive, Flap, Flare, Glide, Soar};
    let modes = [Flap, Glide, Soar, Dive, Flare];
    let mut failures = Vec::new();
    for lineage in [
        Lineage::pure(WingedFamily::Bird),
        Lineage::pure(WingedFamily::Bat),
    ] {
        let avatar = built(&winged(lineage));
        let slack = 0.02 * trunk(&avatar.rig);
        for from in modes {
            for into in modes {
                if from == into {
                    continue;
                }
                for offset in 0..5 {
                    let script: Script = vec![
                        (Some(Flight::new(Flap)), 0.6, 5.0),
                        (Some(Flight::new(from)), 0.5 + 0.067 * offset as f32, 5.0),
                        (Some(Flight::new(into)), 0.3, 5.0),
                    ];
                    let run = run(&avatar, &script, Carriage::Chassis);
                    let at = run.switches[2];
                    let jump = step_at(&run.joints, at);
                    let around = (at - 5..at)
                        .chain(at + 1..=at + 5)
                        .map(|frame| step_at(&run.joints, frame))
                        .fold(0.0f32, f32::max);
                    if jump > around * 2.0 + slack {
                        failures.push(format!(
                            "{lineage:?} {from:?} to {into:?} ({offset}): {jump:.4} m against {around:.4}"
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **No drawn vertex jumps in flight**: over the whole script, at every
/// frame, no vertex of a body or its wing parts moves more than three times
/// the furthest any joint moved and a few centimetres. A membrane blending
/// two joints come more than a half turn apart - an arm beaten through its
/// stroke against a spar that takes less of it - flips by most of a wing.
#[test]
fn no_drawn_vertex_jumps_in_flight() {
    let script = every_mode();
    let mut failures = Vec::new();
    for (name, lineage) in lineages() {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        let mut driver = Driver::seeded(7);
        let mut previous: Option<(Vec<Vec3>, Vec<Vec3>)> = None;
        for &(flight, seconds, speed) in &script {
            for _ in 0..(seconds / STEP).round() as usize {
                let driven = driver
                    .drive(rig, &inputs(flight, speed), level_ground)
                    .expect("a driven frame");
                let joints = driven.pose.forward(rig).positions;
                let drawn: Vec<Vec3> = avatar
                    .posed(&driven.pose, 0.0)
                    .into_iter()
                    .filter(|mesh| mesh.kind != MeshKind::Eye)
                    .flat_map(|mesh| mesh.mesh.positions)
                    .collect();
                if let Some((was_joints, was_drawn)) = &previous {
                    let joint = joints
                        .iter()
                        .zip(was_joints)
                        .map(|(a, b)| a.distance(*b))
                        .fold(0.0f32, f32::max);
                    let vertex = drawn
                        .iter()
                        .zip(was_drawn)
                        .map(|(a, b)| a.distance(*b))
                        .fold(0.0f32, f32::max);
                    if vertex > joint * 3.0 + 0.03 {
                        failures.push(format!(
                            "{name}: a vertex moved {vertex:.3} m when no joint moved more than {joint:.3}"
                        ));
                    }
                }
                previous = Some((joints, drawn));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far the left wrist stands raised over the left shoulder, in radians
/// about the trunk's long axis, the trunk as posed: the beat, read off the
/// pose, whatever the body's own pitch.
fn wing_elevation(rig: &Rig, pose: &Pose) -> f32 {
    let posed = pose.forward(rig);
    let upper = rig.in_zone(Zone::UpperLimb(Limb::ForeLeft));
    let shoulder = upper[upper.len() - 2];
    let wrist = rig.in_zone(Zone::LowerLimb(Limb::ForeLeft))[0];
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().unwrap();
    let run = rig.joints[withers].position - rig.joints[pelvis].position;
    let along = Vec3::new(0.0, run.y, run.z).normalize();
    let up = (Vec3::Y - along * along.y).normalize();
    let turn = posed.rotations[pelvis];
    let (up, out) = (turn * up, turn * Vec3::X);
    let arm = posed.positions[wrist] - posed.positions[shoulder];
    arm.dot(up).atan2(arm.dot(out))
}

/// Each frame at which the wing tops its beat, raised past `over`: a
/// downstroke begins.
fn tops(heights: &[f32], over: f32) -> Vec<usize> {
    (1..heights.len().saturating_sub(1))
        .filter(|&at| {
            heights[at] > over && heights[at] > heights[at - 1] && heights[at] >= heights[at + 1]
        })
        .collect()
}

/// **The beat slows with size as mass to the -0.28** (research-11 4.1,
/// 8.2): a small bird and a large one, and a small bat and a large one,
/// beat at rates whose ratio is their trunks' to the -0.84 - mass as the
/// trunk cubed - within a percent; and the pose beats at the rate the driver
/// reports, read off the wrist's tops over four seconds of flapping. A bird
/// beats faster than a bat of its trunk.
#[test]
fn the_beat_slows_with_size_as_mass_to_the_minus_0_28() {
    for family in [WingedFamily::Bird, WingedFamily::Bat] {
        let mut rates = Vec::new();
        for size in [0.4, 1.1] {
            let mut params = WingedParams::default();
            params.set_lineage(Lineage::pure(family));
            params.size = size;
            let mut record = AvatarRecord::new("Winged", Archetype::Winged(params));
            record.sanitize();
            let avatar = built(&record);
            let rig = &avatar.rig;
            let mut driver = Driver::seeded(7);
            let mut heights = Vec::new();
            let seconds = 4.0;
            for _ in 0..(seconds / STEP) as usize {
                let driven = driver
                    .drive(
                        rig,
                        &inputs(Some(Flight::new(FlightMode::Flap)), 6.0),
                        level_ground,
                    )
                    .expect("a driven frame");
                heights.push(wing_elevation(rig, &driven.pose));
            }
            let reported = driver.flight(rig).expect("flying").rate;
            // Past the first second, which eases into the beat.
            let settled = &heights[(1.0 / STEP) as usize..];
            let counted = tops(settled, 0.2).len() as f32 / (seconds - 1.0);
            assert!(
                (counted - reported).abs() <= 0.5,
                "{family:?} size {size}: the pose beats {counted:.2} a second, the driver says {reported:.2}"
            );
            rates.push((trunk(rig), reported));
        }
        let [(small_trunk, small), (large_trunk, large)] = rates[..] else {
            unreachable!()
        };
        assert!(
            large_trunk > small_trunk * 1.5,
            "{family:?}: the sizes differ"
        );
        let expected = (large_trunk / small_trunk).powf(RATE_EXPONENT);
        let mass = ((large_trunk / small_trunk).powi(3)).powf(-0.28);
        assert!(
            ((large / small) / expected - 1.0).abs() < 0.01,
            "{family:?}: rates {small:.2} and {large:.2} for trunks {small_trunk:.3} and {large_trunk:.3}, against {expected:.3}"
        );
        assert!((expected / mass - 1.0).abs() < 0.01);
    }
    let rate = |family| {
        let avatar = built(&winged(Lineage::pure(family)));
        let mut driver = Driver::seeded(7);
        driver
            .drive(
                &avatar.rig,
                &inputs(Some(Flight::new(FlightMode::Flap)), 6.0),
                level_ground,
            )
            .expect("a driven frame");
        let flown = driver.flight(&avatar.rig).expect("flying");
        flown.rate * trunk(&avatar.rig).powf(-RATE_EXPONENT)
    };
    assert!(rate(WingedFamily::Bird) > rate(WingedFamily::Bat));
}

/// **A takeoff's legs give most of its lift-off speed before its wings
/// beat, then it beats two or three big beats** (research-11 3.1: the legs
/// 80 to 95% of the lift-off speed): a bird carrying itself, on level
/// ground, keeps its feet on the ground through its push; its first
/// downstroke begins before its feet leave; by then its root is rising at
/// 75 to 95% of the speed it leaves the ground at; and the takeoff's beats
/// are wider than a cruise's, two of them at least in its 0.6 seconds.
#[test]
fn a_takeoff_pushes_with_the_legs_then_beats_big() {
    let avatar = built(&winged(Lineage::pure(WingedFamily::Bird)));
    let rig = &avatar.rig;
    // No blend into the flight: the push is a tenth of a second and the
    // blend would carry the standing pose's speed through all of it.
    let mut driver = Driver::new(
        DriverConfig {
            carriage: Carriage::Own,
            blend: 0.0,
            ..DriverConfig::default()
        },
        7,
    );
    for _ in 0..(0.3 / STEP) as usize {
        driver.drive(rig, &inputs(None, 0.0), level_ground);
    }
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let feet: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .map(|limb| rig.in_zone(Zone::Extremity(limb))[0])
        .collect();
    let rest_feet: Vec<f32> = feet
        .iter()
        .map(|&foot| rig.joints[foot].position.y)
        .collect();
    let (mut heights, mut wrist, mut lowest) = (Vec::new(), Vec::new(), Vec::new());
    let mut takeoff_width = 0.0f32;
    let takeoff = (0.6 / STEP).round() as usize;
    for frame in 0..takeoff {
        let driven = driver
            .drive(
                rig,
                &inputs(Some(Flight::new(FlightMode::Takeoff)), 2.5),
                level_ground,
            )
            .expect("a driven frame");
        let posed = driven.pose.forward(rig);
        heights.push(posed.positions[pelvis].y);
        wrist.push(wing_elevation(rig, &driven.pose));
        lowest.push(
            feet.iter()
                .zip(&rest_feet)
                .map(|(&foot, rest)| posed.positions[foot].y - rest)
                .fold(f32::MAX, f32::min),
        );
        if frame > takeoff / 2 {
            takeoff_width = takeoff_width.max(driver.flight(rig).unwrap().amplitude);
        }
    }
    // Lift-off: the last frame the feet are on the ground, before they are
    // off it for good.
    let lift = (1..takeoff)
        .find(|&at| lowest[at..].iter().take(12).all(|&up| up > 0.0005))
        .expect("the feet leave the ground")
        - 1;
    assert!(
        lowest[..lift].iter().all(|&up| up.abs() < 0.01),
        "the feet stay on the ground through the push: {:?}",
        &lowest[..lift]
    );
    let speed = |at: usize| (heights[at + 1] - heights[at - 1]) / (2.0 * STEP);
    let first = tops(&wrist, 0.4)[0];
    assert!(
        first <= lift,
        "the first downstroke (frame {first}) begins before lift-off (frame {lift})"
    );
    let share = speed(first) / speed(lift);
    assert!(
        (0.75..=0.95).contains(&share),
        "the legs gave {share:.2} of the lift-off speed before the wings beat (LEG_SHARE {LEG_SHARE})"
    );
    let beats = tops(&wrist, 0.4).len();
    assert!(beats >= 2, "{beats} beats in the takeoff");
    for _ in 0..(1.2 / STEP) as usize {
        driver.drive(
            rig,
            &inputs(Some(Flight::new(FlightMode::Flap)), 5.0),
            level_ground,
        );
    }
    let cruise = driver.flight(rig).unwrap().amplitude;
    assert!(
        takeoff_width > cruise * 1.15,
        "a takeoff's beat ({takeoff_width:.2}) is wider than a cruise's ({cruise:.2})"
    );
}

/// **A flare begins by time to contact, not by mode or distance**
/// (research-11 6.1, 8.1): a bird gliding with a top 1.2 seconds ahead
/// does not flare, at any speed; as the contact closes it flares from
/// [`FLARE_TAU`] and no earlier, deeper the nearer it is, its legs swinging
/// forward of where they glided and its tail fanning - at the same time to
/// contact whether it flies at 4 or 9 m/s, so at different distances.
#[test]
fn a_flare_begins_by_time_to_contact() {
    let avatar = built(&winged(Lineage::pure(WingedFamily::Bird)));
    let rig = &avatar.rig;
    let foot = rig.in_zone(Zone::Extremity(Limb::HindLeft))[0];
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let mut began = Vec::new();
    for speed in [4.0, 9.0] {
        let mut driver = Driver::seeded(7);
        let glide = |contact: Option<f32>| Flight {
            contact,
            ..Flight::new(FlightMode::Glide)
        };
        let mut gliding_foot = 0.0;
        for _ in 0..(1.0 / STEP) as usize {
            let driven = driver
                .drive(rig, &inputs(Some(glide(Some(1.2))), speed), level_ground)
                .unwrap();
            let posed = driven.pose.forward(rig);
            gliding_foot = posed.positions[foot].z - posed.positions[pelvis].z;
        }
        let flown = driver.flight(rig).unwrap();
        assert_eq!(flown.mode, FlightMode::Glide, "{speed}: no flare 1.2 s out");
        assert!(flown.flare < 1e-3);
        // Closing at one second a second, from 0.8 s out to contact.
        let mut start = None;
        let mut last = 0.0f32;
        let frames = (0.8 / STEP) as usize;
        for frame in 0..frames {
            let contact = 0.8 - frame as f32 * STEP;
            let driven = driver
                .drive(
                    rig,
                    &inputs(Some(glide(Some(contact))), speed),
                    level_ground,
                )
                .unwrap();
            let flown = driver.flight(rig).unwrap();
            if flown.mode == FlightMode::Flare && start.is_none() {
                start = Some(contact);
            }
            if contact > FLARE_TAU + 1e-3 {
                assert_eq!(
                    flown.mode,
                    FlightMode::Glide,
                    "{speed}: flared at {contact:.3} s"
                );
            }
            assert!(flown.flare + 1e-4 >= last, "{speed}: the flare eased off");
            last = flown.flare;
            if frame + 1 == frames {
                let posed = driven.pose.forward(rig);
                let reaching = posed.positions[foot].z - posed.positions[pelvis].z;
                assert!(
                    reaching > gliding_foot + 0.02,
                    "{speed}: the legs reach forward into the flare ({reaching:.3} against {gliding_foot:.3})"
                );
            }
        }
        assert!(
            last > 0.9,
            "{speed}: the flare is full at contact ({last:.2})"
        );
        began.push(start.expect("the flare began"));
    }
    assert!(
        (began[0] - began[1]).abs() < 2.0 * STEP,
        "flared at {began:?}"
    );
    assert!(began.iter().all(|&tau| tau <= FLARE_TAU + 1e-3));
}

/// **A glider never flaps** (the owner's decision on #418), nor a cross a
/// glider leads: asked to flap with a host's beat running, it glides, its
/// wings spread and still - no joint moving a tenth of a millimetre a frame
/// once settled - where a bird asked the same beats its wrist through
/// centimetres.
#[test]
fn a_glider_never_flaps() {
    use WingedFamily::{Bat, Bird, Glider};
    for (lineage, flaps) in [
        (Lineage::pure(Glider), false),
        (Lineage::cross(Glider, Bird, 0.3), false),
        (Lineage::cross(Glider, Bat, 0.45), false),
        (Lineage::pure(Bird), true),
    ] {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        let mut driver = Driver::seeded(7);
        let mut moved = 0.0f32;
        let mut previous: Option<Vec<Vec3>> = None;
        let frames = (2.0 / STEP) as usize;
        for frame in 0..frames {
            let flight = Flight {
                phase: Some(frame as f32 * STEP * 3.0),
                ..Flight::new(FlightMode::Flap)
            };
            let driven = driver
                .drive(rig, &inputs(Some(flight), 5.0), level_ground)
                .unwrap();
            let joints = driven.pose.forward(rig).positions;
            if frame > frames / 2
                && let Some(was) = &previous
            {
                moved = moved.max(
                    joints
                        .iter()
                        .zip(was)
                        .map(|(a, b)| a.distance(*b))
                        .fold(0.0, f32::max),
                );
            }
            previous = Some(joints);
        }
        let flown = driver.flight(rig).unwrap();
        if flaps {
            assert_eq!(flown.mode, FlightMode::Flap);
            assert!(
                moved > 0.005,
                "{lineage:?}: a bird beats ({moved:.4} m a frame)"
            );
        } else {
            assert_eq!(flown.mode, FlightMode::Glide, "{lineage:?}: glides");
            assert_eq!(flown.amplitude, 0.0, "{lineage:?}: no beat");
            assert!(
                moved < 1e-4,
                "{lineage:?}: its wings moved {moved:.5} m a frame"
            );
        }
    }
}

/// **A bat's hand folds flatter than 149 degrees and still never flips**
/// (#424; W2 stopped at 149 and 143): each digit's first phalanx lies back
/// along its metacarpal at more than 150 degrees and its second along the
/// first at more than 150; spread, every joint of the hand opens to within 16
/// degrees of straight, so the guard against a flip (`wing::fit`) has
/// trimmed none of it; and over two hundred steps of the extension, and
/// through a beat at every extension a beat folds to, no drawn vertex steps
/// more than six times the mean step. At this fold with no helpers at the
/// knuckles, the guard held digits IV and V 26 to 28 degrees shut.
#[test]
fn a_bats_hand_folds_flat_and_never_flips() {
    use WingedFamily::{Bat, Bird, Glider};
    for lineage in [
        Lineage::pure(Bat),
        Lineage::cross(Bat, Bird, 0.3),
        Lineage::cross(Bat, Glider, 0.3),
    ] {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        let wings = rig.wings.as_ref().unwrap();
        // The digits: four-joint chains of wing joints off the wrist, each
        // the parent of the next.
        let wrist = rig.in_zone(Zone::LowerLimb(Limb::ForeLeft))[0];
        let mut folds = Vec::new();
        let mut opens = Vec::new();
        let spread = {
            let mut pose = Pose::rest(rig);
            rig.spread_wings(&mut pose, 1.0, 1.0);
            pose.forward(rig).positions
        };
        for root in (0..rig.len())
            .filter(|&joint| wings.owns(joint) && rig.joints[joint].parent == Some(wrist))
        {
            let mut chain = vec![root];
            while let Some(next) = (0..rig.len()).find(|&joint| {
                rig.joints[joint].parent == chain.last().copied()
                    && rig.joints[joint]
                        .position
                        .distance(rig.joints[*chain.last().unwrap()].position)
                        > 1e-4
            }) {
                chain.push(next);
            }
            if chain.len() < 4 {
                continue;
            }
            let at = |index: usize| rig.joints[chain[index]].position;
            let bones: Vec<Vec3> = (0..3)
                .map(|bone| (at(bone + 1) - at(bone)).normalize())
                .collect();
            folds.push(bones[0].angle_between(bones[1]).to_degrees());
            folds.push(bones[1].angle_between(bones[2]).to_degrees());
            let open = |index: usize| spread[chain[index]];
            let spread: Vec<Vec3> = (0..3)
                .map(|bone| (open(bone + 1) - open(bone)).normalize())
                .collect();
            opens.push(spread[0].angle_between(spread[1]).to_degrees());
            opens.push(spread[1].angle_between(spread[2]).to_degrees());
        }
        assert_eq!(folds.len(), 8, "{lineage:?}: four digits of three bones");
        assert!(
            folds.iter().all(|&fold| fold > 150.0),
            "{lineage:?}: the hand folds at {folds:?} degrees"
        );
        assert!(
            opens.iter().all(|&open| open < 16.0),
            "{lineage:?}: the hand opens only to {opens:?} degrees spread"
        );

        let drawn = |pose: &Pose| -> Vec<Vec3> {
            avatar
                .posed(pose, 0.0)
                .into_iter()
                .filter(|mesh| mesh.kind != MeshKind::Eye)
                .flat_map(|mesh| mesh.mesh.positions)
                .collect()
        };
        let sweep = |stroke: &dyn Fn(f32) -> Stroke| {
            let mut previous: Option<Vec<Vec3>> = None;
            let (mut worst, mut sum) = (0.0f32, 0.0f32);
            for step in 0..=200 {
                let mut pose = Pose::rest(rig);
                let at = stroke(step as f32 / 200.0);
                rig.stroke_wings(&mut pose, at, at);
                let now = drawn(&pose);
                if let Some(was) = &previous {
                    let moved = now
                        .iter()
                        .zip(was)
                        .map(|(a, b)| a.distance(*b))
                        .fold(0.0, f32::max);
                    worst = worst.max(moved);
                    sum += moved;
                }
                previous = Some(now);
            }
            (worst, sum / 200.0)
        };
        let (worst, mean) = sweep(&|t| Stroke::at(t));
        assert!(
            worst <= mean * 6.0,
            "{lineage:?}: the extension jumped {worst:.3} against {mean:.4}"
        );
        for extension in [0.6, 0.8, 1.0] {
            let (worst, mean) = sweep(&|t| Stroke {
                extension,
                elevation: 0.9 * (std::f32::consts::TAU * t).cos(),
                sweep: 0.3 * (std::f32::consts::TAU * t).sin(),
                reach: 0.0,
            });
            assert!(
                worst <= mean * 6.0,
                "{lineage:?}: a beat at {extension} jumped {worst:.3} against {mean:.4}"
            );
        }
    }
}

/// How many of a bird's trunk vertices have no card rooted within 0.06 of
/// its back's height, and how many trunk vertices it has.
fn bare_trunk(avatar: &Avatar) -> (usize, usize) {
    let rig = &avatar.rig;
    let neck = rig.in_zone(Zone::Neck)[0];
    let withers = rig.joints[rig.joints[neck].parent.unwrap()].position;
    let reach = 0.06 * withers.y;
    let fur = avatar.parts.fur.as_ref().expect("a bird is feathered");
    let roots: Vec<Vec3> = fur
        .mesh
        .positions
        .chunks(9)
        .map(|card| (card[0] + card[1]) * 0.5)
        .collect();
    let trunk: Vec<Vec3> = avatar
        .parts
        .body
        .positions
        .iter()
        .zip(&avatar.parts.zones)
        .filter(|(_, zone)| matches!(zone, Zone::Chest | Zone::Abdomen | Zone::Pelvis))
        .map(|(at, _)| *at)
        .collect();
    let bare = trunk
        .iter()
        .filter(|at| roots.iter().all(|root| root.distance(**at) > reach))
        .count();
    (bare, trunk.len())
}

/// **A spread bird has cards under its wing** (#424; W4 cleared them): the
/// flank its folded wing covers keeps its cards, shortened, rather than
/// clearing to none - so some root within the reach the cards used to clear,
/// and with the wings spread no more than two thirds as many trunk vertices
/// as W4 left have no card rooted within 0.06 of the back's height, on the
/// default bird (W4: 25 of 575) and on seed 113 (77 of 876).
#[test]
fn a_spread_bird_has_cards_under_its_wing() {
    let mut seeded = winged(Lineage::pure(WingedFamily::Bird));
    seeded.reroll(113);
    for (name, record, was) in [
        ("default", winged(Lineage::pure(WingedFamily::Bird)), 25),
        ("seed 113", seeded, 77),
    ] {
        let avatar = Avatar::build(&record).expect("a bird builds");
        let (bare, of) = bare_trunk(&avatar);
        assert!(
            bare * 3 <= was * 2,
            "{name}: {bare} of {of} trunk vertices have no card near (W4: {was})"
        );
        // Under the fold: within the reach W4 kept clear (0.03 of the back's
        // height) of a folded feather.
        let rig = &avatar.rig;
        let neck = rig.in_zone(Zone::Neck)[0];
        let height = rig.joints[rig.joints[neck].parent.unwrap()].position.y;
        let feathers = &avatar.parts.wings.as_ref().unwrap().feathers.positions;
        let fur = avatar.parts.fur.as_ref().unwrap();
        let under = fur
            .mesh
            .positions
            .chunks(9)
            .map(|card| (card[0] + card[1]) * 0.5)
            .filter(|root| feathers.iter().any(|at| at.distance(*root) < 0.03 * height))
            .count();
        assert!(under > 0, "{name}: no card roots under the folded wing");
    }
}

/// A person, a dog and a horse.
fn wingless() -> Vec<(String, AvatarRecord)> {
    let mut all = vec![("person".to_string(), AvatarRecord::default())];
    for family in [
        symbios_avatar::plan::Family::Dog,
        symbios_avatar::plan::Family::Horse,
    ] {
        let mut params = QuadrupedParams::default();
        params.set_lineage(Lineage::pure(family));
        all.push((
            format!("{family:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    all
}

/// **A body with no wings moves exactly as it did, flight or none**: a
/// person, a dog and a horse driven through standing, a walk, a run, a leap
/// and its landing, each frame given a flight, are posed to the bit as
/// without one; and a winged body given none walks as it would, never
/// [`Source::Fly`]. `Inputs::default()` asks no flight.
#[test]
fn a_body_given_no_flight_or_no_wings_moves_as_it_did() {
    assert!(Inputs::default().flight.is_none());
    let mut bodies = wingless();
    bodies.push((
        "bird".to_string(),
        winged(Lineage::pure(WingedFamily::Bird)),
    ));
    for (name, record) in bodies {
        let avatar = Avatar::build_for_shape(&record).expect("a body builds");
        let rig = &avatar.rig;
        let winged = rig.wings.is_some();
        let script: Vec<(Vec3, f32)> = vec![
            (Vec3::ZERO, 0.5),
            (Vec3::Z * 1.4, 1.0),
            (Vec3::Z * 4.0, 1.0),
            (Vec3::new(0.0, 3.5, 3.0), 0.05),
            (Vec3::new(0.0, 0.0, 3.0), 0.3),
            (Vec3::new(0.0, -3.0, 3.0), 0.3),
            (Vec3::Z * 1.0, 0.6),
            (Vec3::ZERO, 0.6),
        ];
        let mut plain = Driver::seeded(7);
        let mut asked = Driver::seeded(7);
        for &(velocity, seconds) in &script {
            for _ in 0..(seconds / STEP).round() as usize {
                let without = Inputs {
                    delta: STEP,
                    velocity,
                    ..Inputs::default()
                };
                let a = plain.drive(rig, &without, level_ground).unwrap();
                assert_ne!(a.source, Source::Fly, "{name}: flew with no flight asked");
                if winged {
                    continue;
                }
                let with = Inputs {
                    flight: Some(Flight::new(FlightMode::Flap)),
                    ..without
                };
                let b = asked.drive(rig, &with, level_ground).unwrap();
                assert_eq!(
                    a.pose, b.pose,
                    "{name}: a flight moved a body with no wings"
                );
                assert_eq!(a.source, b.source);
            }
        }
    }
}

/// A frame's flight in a script whose host keeps the beat's clock: each
/// segment's mode, how long, its speed, and its phase as a function of the
/// seconds since the script began.
type Phased = Vec<(Option<FlightMode>, f32, f32)>;

/// A host's beat phase as a function of the seconds since a script began.
type Clock = dyn Fn(f32) -> Option<f32>;

/// Drives `script` with the host's phase given by `phase` of the seconds
/// since it began, `None` for the engine's own clock.
fn run_phased(
    avatar: &Avatar,
    script: &Phased,
    carriage: Carriage,
    phase: &Clock,
) -> (Vec<Vec<Vec3>>, Vec<Vec<symbios_avatar::Quat>>) {
    let rig = &avatar.rig;
    let mut driver = Driver::new(
        DriverConfig {
            carriage,
            ..DriverConfig::default()
        },
        7,
    );
    let mut joints = Vec::new();
    let mut rotations = Vec::new();
    let mut t = 0.0f32;
    for &(mode, seconds, speed) in script {
        for _ in 0..(seconds / STEP).round() as usize {
            let flight = mode.map(|mode| Flight {
                phase: phase(t),
                ..Flight::new(mode)
            });
            let driven = driver
                .drive(rig, &inputs(flight, speed), level_ground)
                .expect("a driven frame");
            let posed = driven.pose.forward(rig);
            joints.push(posed.positions);
            rotations.push(posed.rotations);
            t += STEP;
        }
    }
    (joints, rotations)
}

/// **No joint pops anywhere in a flight, whatever the host's clock does**
/// (#424, the critic): through a takeoff from standing into flapping, a
/// glide and a landing, with the host's phase running forward, running
/// backward through negative beats, and jumping half a beat every 0.37
/// seconds, and drifting slowly negative over a long flap; and through takeoffs cut short by a landing or a glide at every
/// point of their push - on a bird, a bat, a glider, five glider rolls and
/// four pairs crossed at 0.49, 0.5 and 0.51 (on the engine's own clock),
/// under both carriages - no frame moves a joint more than twice the most it
/// moved in the four frames either side, nor turns one more than twice the
/// most any turned there or 0.75 radians (1.5 at 60 frames a second). A takeoff's first upstroke left on
/// for good popped a bat's wing 0.8 m at the first negative whole beat; a
/// landing that took the root from a push's crouch straight to its absorb,
/// 0.28 m; a glider crouched past its fold flipped its hind knees.
#[test]
fn no_joint_pops_through_whole_flights() {
    use FlightMode::{Flap, Glide, Land, Takeoff};
    let mut failures = Vec::new();
    let mut bodies: Vec<(String, AvatarRecord)> =
        [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider]
            .into_iter()
            .map(|family| (format!("{family:?}"), winged(Lineage::pure(family))))
            .collect();
    for seed in [9, 14, 17, 24, 26] {
        let mut record = winged(Lineage::pure(WingedFamily::Glider));
        record.reroll(seed);
        bodies.push((format!("glider seed {seed}"), record));
    }
    {
        use WingedFamily::{Bat, Bird, Glider};
        for (a, b) in [(Bird, Glider), (Glider, Bird), (Bat, Glider), (Bird, Bat)] {
            for blend in [0.49, 0.5, 0.51] {
                bodies.push((
                    format!("cross {a:?} x {b:?} {blend}"),
                    winged(Lineage::cross(a, b, blend)),
                ));
            }
        }
    }
    let clocks: [(&str, &Clock); 5] = [
        ("own", &|_| None),
        ("forward", &|t| Some(3.0 * t)),
        ("backward", &|t| Some(-3.0 * t)),
        ("negative", &|t| Some(-0.3 * t)),
        ("jumping", &|t| Some(3.0 * t + 0.5 * (t / 0.37).floor())),
    ];
    for (name, record) in &bodies {
        let avatar = built(record);
        let slack = 0.01 * trunk(&avatar.rig);
        let mut scripts: Vec<(String, Phased)> = vec![(
            "whole".to_string(),
            vec![
                (None, 0.4, 0.0),
                (Some(Takeoff), 0.6, 2.5),
                (Some(Flap), 1.5, 5.0),
                (Some(Glide), 0.5, 6.0),
                (Some(Flap), 0.8, 5.0),
                (Some(Land), 0.3, 0.0),
                (None, 0.4, 0.0),
            ],
        )];
        // Long enough for a clock pulled back by a host running backward to
        // carry the beat's own below nought.
        if !name.starts_with("glider") && !name.starts_with("cross") {
            scripts.push((
                "long".to_string(),
                vec![
                    (None, 0.4, 0.0),
                    (Some(Takeoff), 0.6, 2.5),
                    (Some(Flap), 11.0, 5.0),
                ],
            ));
        }
        for cut in [0.03, 0.06, 0.09, 0.12, 0.15] {
            for into in [Land, Glide] {
                scripts.push((
                    format!("takeoff into {into:?} at {cut}"),
                    vec![
                        (None, 0.4, 0.0),
                        (Some(Takeoff), cut, 2.5),
                        (Some(into), 0.5, 1.0),
                        (None, 0.3, 0.0),
                    ],
                ));
            }
        }
        for (script_name, script) in &scripts {
            for carriage in [Carriage::Chassis, Carriage::Own] {
                for (clock, phase) in &clocks {
                    let wanted = match script_name.as_str() {
                        "whole" => !name.starts_with("cross") || *clock == "own",
                        "long" => *clock == "negative",
                        _ => *clock == "own",
                    };
                    if !wanted {
                        continue;
                    }
                    let (joints, rotations) = run_phased(&avatar, script, carriage, *phase);
                    for (frame, moved, around) in pops(&joints, slack) {
                        failures.push(format!(
                            "{name} {script_name} {carriage:?} {clock}: frame {frame} moved {moved:.3} m against {around:.3}"
                        ));
                    }
                    for (frame, turn, around) in snaps(&turned(&rotations), 0.05, 0.75) {
                        failures.push(format!(
                            "{name} {script_name} {carriage:?} {clock}: frame {frame} turned {turn:.3} rad against {around:.3}"
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **A long frame neither diverges nor strays** (#424, the critic): a body
/// carrying itself, through a takeoff, flapping and a landing in frames of
/// half a second, a quarter, and uneven ones from a hundredth to three
/// tenths, keeps every joint finite and its root within its own height of
/// where it hovers. An explicit step of the hover's spring flew a bird 10^8 m
/// on eleven frames of half a second, and 36 m on Bevy's quarter-second cap.
#[test]
fn a_long_frame_neither_diverges_nor_strays() {
    for family in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        let avatar = built(&winged(Lineage::pure(family)));
        let rig = &avatar.rig;
        let height = rig.extent();
        for frames in [vec![0.5; 11], vec![0.25; 20], {
            (0..40)
                .map(|at| if at % 3 == 0 { 0.3 } else { 0.01 })
                .collect::<Vec<f32>>()
        }] {
            let mut driver = Driver::new(
                DriverConfig {
                    carriage: Carriage::Own,
                    ..DriverConfig::default()
                },
                7,
            );
            for (at, &delta) in frames.iter().enumerate() {
                let mode = match at {
                    0..=1 => FlightMode::Takeoff,
                    _ if at + 2 >= frames.len() => FlightMode::Land,
                    _ => FlightMode::Flap,
                };
                let driven = driver
                    .drive(
                        rig,
                        &Inputs {
                            delta,
                            velocity: Vec3::Z * 4.0,
                            flight: Some(Flight::new(mode)),
                            ..Inputs::default()
                        },
                        level_ground,
                    )
                    .expect("a driven frame");
                let posed = driven.pose.forward(rig);
                assert!(
                    posed.positions.iter().all(|at| at.is_finite()),
                    "{family:?}: a joint went non-finite"
                );
                // Within its height of where it hovers, which is as high as
                // its wings' beat needs.
                let y = driven.pose.translation.y;
                let hover = driver.flight(rig).map_or(0.0, |flown| flown.height);
                assert!(
                    (y - hover).abs() < height,
                    "{family:?} frames of {delta}: the root at {y:.2} m hovering at {hover:.2} on a body {height:.2} tall"
                );
            }
        }
    }
}

/// **A body with no wings keeps every state of its own, flight or none**
/// (#424, the critic): a person and a dog, a cat and a horse, asked each
/// gesture and state of the roster their body has - a greeting, a bow, a sit,
/// lying down - and held there, are posed to the bit as without a flight. A
/// beast once counted a flight as being moved, and never sat.
#[test]
fn a_wingless_body_keeps_its_states_with_a_flight_given() {
    let mut bodies = wingless();
    let mut cat = QuadrupedParams::default();
    cat.set_lineage(Lineage::pure(symbios_avatar::plan::Family::Cat));
    bodies.push((
        "Cat".to_string(),
        AvatarRecord::new("Beast", Archetype::Quadruped(cat)),
    ));
    for (name, record) in bodies {
        let avatar = Avatar::build_for_shape(&record).expect("a body builds");
        let rig = &avatar.rig;
        let names: Vec<&str> = symbios_avatar::anim::gesture::ROSTER
            .iter()
            .chain(symbios_avatar::anim::emote::ROSTER)
            .copied()
            .filter(|gesture| symbios_avatar::anim::gesture::for_body(rig, gesture).is_some())
            .collect();
        assert!(!names.is_empty(), "{name}: a roster");
        for gesture in names {
            let mut plain = Driver::seeded(7);
            let mut asked = Driver::seeded(7);
            for frame in 0..(3.0 / STEP) as usize {
                let without = Inputs {
                    delta: STEP,
                    gesture: (frame == 2).then_some(gesture),
                    ..Inputs::default()
                };
                let with = Inputs {
                    flight: Some(Flight::new(FlightMode::Flap)),
                    ..without
                };
                assert_eq!(
                    plain.stirred(&without),
                    asked.stirred(&with),
                    "{name} {gesture}"
                );
                let a = plain.drive(rig, &without, level_ground).unwrap();
                let b = asked.drive(rig, &with, level_ground).unwrap();
                assert_eq!(a.pose, b.pose, "{name} {gesture}: frame {frame} moved");
                assert_eq!(a.source, b.source);
            }
        }
    }
}

/// **A cross at an even blend beats exactly when it wears a beater's
/// surface** (#424, the critic): its lead, its surface and whether it flaps
/// are one decision, the second family's at 0.5 - a glider crossed half way
/// to a bird is feathered and beats, half way to a glider a bird is a
/// patagium and glides.
#[test]
fn a_cross_at_an_even_blend_flaps_as_its_surface_says() {
    use WingedFamily::{Bat, Bird, Glider};
    for (lineage, flaps) in [
        (Lineage::cross(Glider, Bird, 0.5), true),
        (Lineage::cross(Glider, Bat, 0.5), true),
        (Lineage::cross(Bird, Glider, 0.5), false),
        (Lineage::cross(Bat, Glider, 0.5), false),
    ] {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        let feathered = !avatar.parts.wings.as_ref().unwrap().feathers.is_empty();
        assert_eq!(
            feathered,
            lineage.leading() == Bird,
            "{lineage:?}: its surface is its lead's"
        );
        assert_eq!(
            rig.wings.as_ref().unwrap().kinship(rig).flaps(),
            flaps,
            "{lineage:?}"
        );
        let mut driver = Driver::seeded(7);
        for _ in 0..(1.0 / STEP) as usize {
            driver.drive(
                rig,
                &inputs(Some(Flight::new(FlightMode::Flap)), 5.0),
                level_ground,
            );
        }
        let flown = driver.flight(rig).unwrap();
        assert_eq!(flown.mode == FlightMode::Flap, flaps, "{lineage:?}");
    }
}

/// The most any joint turned into each frame, in radians, against the frame
/// before: a joint's whole orientation, as the critic read it.
fn turned(rotations: &[Vec<symbios_avatar::Quat>]) -> Vec<f32> {
    (1..rotations.len())
        .map(|at| {
            rotations[at]
                .iter()
                .zip(&rotations[at - 1])
                .map(|(a, b)| turn_between(*b, *a))
                .fold(0.0, f32::max)
        })
        .collect()
}

/// Every frame some joint turned more than twice the most any turned in the
/// four frames either side and `slack`, or more than `most`.
fn snaps(turns: &[f32], slack: f32, most: f32) -> Vec<(usize, f32, f32)> {
    (0..turns.len())
        .filter_map(|at| {
            let around = (at.saturating_sub(4)..at)
                .chain(at + 1..(at + 5).min(turns.len()))
                .map(|frame| turns[frame])
                .fold(0.0f32, f32::max);
            let snapped = turns[at] > most || (at >= 4 && turns[at] > around * 2.0 + slack);
            snapped.then_some((at + 1, turns[at], around))
        })
        .collect()
}

/// A winged body of `lineage` at `size`, or as rolled.
fn sized(lineage: Lineage<WingedFamily>, size: Option<f32>) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    let resized = size.is_some();
    if let Some(size) = size {
        params.size = size;
    }
    let mut record = AvatarRecord::new("Winged", Archetype::Winged(params));
    if resized {
        record.sanitize();
    }
    record
}

/// **A host hands on without guessing** (#424, the viewer's critic): driven
/// at 60 frames a second as a host reads [`Flown`] - a takeoff handed on to
/// flapping two beats after the body has `lifted`, a glide, a landing handed
/// back to the ground once it has `settled` - every family, small and large,
/// and every pair crossed at 0.49, 0.5 and 0.51, under both carriages, still
/// and carried along at 3 m/s from a walk through a flare:
/// lifts and settles, each well inside a few seconds; is settled only folded
/// and down, with a body that carries itself at the ground; once a landing
/// carrying itself has begun coming down it never climbs again, and once
/// down nothing it draws rises off the ground (it absorbed in mid air and
/// climbed back 0.27 m on a bat at size 2.4); nothing drawn goes
/// under the ground at the hand back; and no joint pops, nor - but on the
/// smallest, which beats near 20 times a second - turns more than 1.25
/// radians in a frame (a Bird x Glider cross at 0.51 turned its fore limb
/// 2.02 radians in one frame of a landing).
#[test]
fn a_host_hands_on_by_lifted_and_settled() {
    use WingedFamily::{Bat, Bird, Glider};
    let step = 1.0 / 60.0;
    let mut bodies: Vec<(String, AvatarRecord)> = Vec::new();
    for family in [Bird, Bat, Glider] {
        for size in [Some(0.1), None, Some(2.4)] {
            bodies.push((
                format!("{family:?} size {size:?}"),
                sized(Lineage::pure(family), size),
            ));
        }
    }
    for (a, b) in [(Bird, Glider), (Glider, Bird), (Bat, Glider), (Bird, Bat)] {
        for blend in [0.49, 0.5, 0.51] {
            bodies.push((
                format!("{a:?} x {b:?} {blend}"),
                sized(Lineage::cross(a, b, blend), None),
            ));
        }
    }
    let mut failures = Vec::new();
    for (name, record) in &bodies {
        let avatar = built(record);
        let rig = &avatar.rig;
        let slack = 0.01 * trunk(rig);
        for (carriage, moving) in [
            (Carriage::Chassis, false),
            (Carriage::Own, false),
            (Carriage::Chassis, true),
            (Carriage::Own, true),
        ] {
            let tag = format!("{carriage:?}{}", if moving { " moving" } else { "" });
            // Still, or carried along at 3 m/s from a walk to a walk, as the
            // critic flew it.
            let (ground, up, cruise, gliding) = if moving {
                (3.0, 3.0, 3.0, 3.0)
            } else {
                (0.0, 2.5, 5.0, 6.0)
            };
            let mut driver = Driver::new(
                DriverConfig {
                    carriage,
                    ..DriverConfig::default()
                },
                7,
            );
            let (mut joints, mut rotations, mut flying) = (Vec::new(), Vec::new(), Vec::new());
            let mut frame = |driver: &mut Driver, flight: Option<Flight>, speed: f32| {
                let driven = driver
                    .drive(rig, &inputs_at(flight, speed, step), level_ground)
                    .expect("a driven frame");
                let posed = driven.pose.forward(rig);
                joints.push(posed.positions);
                rotations.push(posed.rotations);
                let flown = driver.flight(rig);
                flying.push(flown.is_some());
                (driven.pose, flown)
            };
            // The lowest it drew on the ground before it flew: standing, or
            // through a walk's every frame.
            let mut standing = f32::MAX;
            for _ in 0..((if moving { 0.4 } else { 0.3 }) / step) as usize {
                let (pose, _) = frame(&mut driver, None, ground);
                standing = standing.min(lowest_drawn(&avatar, &pose));
            }
            // The takeoff, until two beats after lifting off.
            let mut lifted_at = None;
            let mut seconds = 0.0f32;
            let mut taken = 0usize;
            loop {
                let (_, flown) = frame(&mut driver, Some(Flight::new(FlightMode::Takeoff)), up);
                let flown = flown.expect("flying");
                seconds += step;
                if flown.lifted && lifted_at.is_none() {
                    lifted_at = Some(flown.phase);
                }
                // Handed on two beats after lifting off - or, carried along,
                // at 0.6 s, as a host guessing did.
                taken += 1;
                let handed = if moving {
                    taken >= (0.6 / step) as usize
                } else {
                    lifted_at.is_some_and(|at| flown.phase >= at + 2.0)
                };
                if handed {
                    break;
                }
                if seconds > 6.0 {
                    failures.push(format!("{name} {tag}: never lifted and beat twice"));
                    break;
                }
            }
            for _ in 0..((if moving { 1.0 } else { 0.8 }) / step) as usize {
                frame(&mut driver, Some(Flight::new(FlightMode::Flap)), cruise);
            }
            for _ in 0..(0.5 / step) as usize {
                frame(&mut driver, Some(Flight::new(FlightMode::Glide)), gliding);
            }
            if moving {
                for _ in 0..(0.45 / step) as usize {
                    frame(&mut driver, Some(Flight::new(FlightMode::Flare)), gliding);
                }
            }
            // The landing, until it has settled.
            let mut seconds = 0.0f32;
            let mut lowest_height = f32::MAX;
            let mut down = false;
            loop {
                let (pose, flown) = frame(&mut driver, Some(Flight::new(FlightMode::Land)), ground);
                let flown = flown.expect("flying");
                seconds += step;
                if flown.height > lowest_height + 1e-4 {
                    failures.push(format!(
                        "{name} {tag}: climbed back to {:.3} m from {lowest_height:.3} in its landing",
                        flown.height
                    ));
                    break;
                }
                lowest_height = lowest_height.min(flown.height);
                // No absorb in the air: while a body carrying itself is still
                // coming down, its root is carried by its hover alone, not
                // dipped by legs it has not yet put down.
                // (Once the beat's heave has eased out with its width.)
                if carriage == Carriage::Own
                    && flown.height > 0.02 * rig.extent()
                    && flown.amplitude < 0.01
                {
                    let offset = pose.translation.y - flown.height - levelled(rig, &pose);
                    if offset < -0.01 * trunk(rig) {
                        failures.push(format!(
                            "{name} {tag}: absorbed {offset:.3} m with {:.3} m still to come down",
                            flown.height
                        ));
                        break;
                    }
                }
                // Once down, it stays down: nothing it draws rises off the
                // ground again until it is handed back.
                let lowest = lowest_drawn(&avatar, &pose);
                if down && lowest > standing.max(0.0) + 0.08 * trunk(rig) + 0.005 {
                    failures.push(format!(
                        "{name} {tag}: came down and rose again to {lowest:.3} m in its landing"
                    ));
                    break;
                }
                down |= lowest <= standing.max(0.0) + 0.01 * trunk(rig);
                if flown.settled {
                    if flown.extension > 0.02 || flown.height > 0.01 * rig.extent() {
                        failures.push(format!(
                            "{name} {tag}: settled spread {:.3} or up {:.3}",
                            flown.extension, flown.height
                        ));
                    }
                    break;
                }
                if seconds > 4.0 {
                    failures.push(format!("{name} {tag}: never settled"));
                    break;
                }
            }
            for at in 0..(0.5 / step) as usize {
                let (pose, flown) = frame(&mut driver, None, ground);
                assert!(flown.is_none());
                if at == 0 {
                    let lowest = lowest_drawn(&avatar, &pose);
                    if lowest < standing.min(0.0) - UNDER_GROUND - 0.01 * trunk(rig) {
                        failures.push(format!(
                            "{name} {tag}: handed back {lowest:.3} m under the ground"
                        ));
                    }
                }
            }
            // The flight's frames, and the first the ground takes back: the
            // ground gait's own are the gait's tests'.
            // Past a flight's first four frames, which the blend out of the
            // ground carries the gait's own motion through.
            let judged = |at: usize| {
                at < flying.len()
                    && at >= 4
                    && ((flying[at] && flying[at - 4]) || (flying[at - 1] && !flying[at]))
            };
            for (at, moved, around) in pops(&joints, slack).into_iter().filter(|p| judged(p.0)) {
                failures.push(format!(
                    "{name} {tag}: frame {at} moved {moved:.3} m against {around:.3}"
                ));
            }
            // A small bird beats faster than a host draws it - nineteen times a
            // second at size 0.1, three frames a beat at 60 - so its feathers
            // turn further in a frame than any bound on a pop.
            if name.contains("Some(0.1)") {
                continue;
            }
            for (at, turn, around) in snaps(&turned(&rotations), 0.05, 1.25)
                .into_iter()
                .filter(|p| judged(p.0))
            {
                failures.push(format!(
                    "{name} {tag}: frame {at} turned {turn:.3} rad against {around:.3}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far levelling the trunk about its middle moved the root up, as the
/// flight turns the root of `pose` about the trunk's middle.
fn levelled(rig: &Rig, pose: &Pose) -> f32 {
    let Some(root) = (0..rig.len()).find(|&joint| rig.joints[joint].parent.is_none()) else {
        return 0.0;
    };
    let pelvis = rig.joints[rig.in_zone(Zone::Pelvis)[0]].position;
    let withers = rig.joints[*rig.in_zone(Zone::Chest).last().unwrap()].position;
    let pivot = (pelvis + withers) * 0.5 - rig.joints[root].position;
    (pivot - pose.rotations[root] * pivot).y
}

/// A frame of `flight` at `speed`, `delta` long.
fn inputs_at(flight: Option<Flight>, speed: f32, delta: f32) -> Inputs<'static> {
    Inputs {
        delta,
        ..inputs(flight, speed)
    }
}

/// **A landing asked of a body on the ground is a landing from no height**
/// (#424): a bird, a bat and a glider standing, then asked to land, carried
/// or carrying itself, put their feet on the ground and keep them there -
/// nothing drawn lower than standing put it, nor a centimetre higher, at the
/// end - settle within
/// half a second, and do not dip: no absorb from no fall. Asked so before,
/// it held no feet and never absorbed.
#[test]
fn a_landing_asked_on_the_ground_plants_and_settles() {
    for family in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        let avatar = built(&winged(Lineage::pure(family)));
        let rig = &avatar.rig;
        for carriage in [Carriage::Chassis, Carriage::Own] {
            let mut driver = Driver::new(
                DriverConfig {
                    carriage,
                    ..DriverConfig::default()
                },
                7,
            );
            let mut standing = 0.0;
            for _ in 0..(0.3 / STEP) as usize {
                let driven = driver.drive(rig, &inputs(None, 0.0), level_ground).unwrap();
                standing = lowest_drawn(&avatar, &driven.pose);
            }
            let mut settled_at = None;
            let mut last = None;
            for at in 0..(0.6 / STEP) as usize {
                let driven = driver
                    .drive(
                        rig,
                        &inputs(Some(Flight::new(FlightMode::Land)), 0.0),
                        level_ground,
                    )
                    .unwrap();
                let flown = driver.flight(rig).unwrap();
                assert!(
                    driven.pose.translation.y > -0.01 * trunk(rig),
                    "{family:?} {carriage:?}: dipped {:.3} m landing from no height",
                    driven.pose.translation.y
                );
                if flown.settled && settled_at.is_none() {
                    settled_at = Some(at);
                }
                last = Some(driven.pose);
            }
            assert!(
                settled_at.is_some(),
                "{family:?} {carriage:?}: never settled"
            );
            let lowest = lowest_drawn(&avatar, &last.unwrap());
            assert!(
                (standing.min(0.0) - UNDER_GROUND..standing.max(0.0) + 0.01).contains(&lowest),
                "{family:?} {carriage:?}: its lowest point at {lowest:.3} m, standing {standing:.3}"
            );
        }
    }
}

/// **No joint turns over in a landing** (#424, the viewer's critic): every
/// family and every pair crossed at 0.49, 0.5 and 0.51, carried along at
/// 3 m/s from a walk through a takeoff, flapping, a glide and a flare into a
/// landing at 60 frames a second, under both carriages, turns no joint more
/// than 0.8 radians in a frame of its landing. A Bird x Glider cross at 0.51
/// turned its fore limb 1.6 radians in one frame there, solved onto the
/// ground half spread, where the other bend of its arm was nearer.
#[test]
fn no_joint_turns_over_in_a_landing() {
    use WingedFamily::{Bat, Bird, Glider};
    let step = 1.0 / 60.0;
    let mut lineages = vec![
        Lineage::pure(Bird),
        Lineage::pure(Bat),
        Lineage::pure(Glider),
    ];
    for (a, b) in [
        (Bird, Glider),
        (Glider, Bird),
        (Bird, Bat),
        (Bat, Bird),
        (Bat, Glider),
        (Glider, Bat),
    ] {
        for blend in [0.49, 0.5, 0.51] {
            lineages.push(Lineage::cross(a, b, blend));
        }
    }
    let script = [
        (None, 0.4),
        (Some(FlightMode::Takeoff), 0.6),
        (Some(FlightMode::Flap), 1.0),
        (Some(FlightMode::Glide), 0.5),
        (Some(FlightMode::Flare), 0.45),
        (Some(FlightMode::Land), 0.5),
    ];
    let mut failures = Vec::new();
    for lineage in lineages {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        for carriage in [Carriage::Chassis, Carriage::Own] {
            let mut driver = Driver::new(
                DriverConfig {
                    carriage,
                    ..DriverConfig::default()
                },
                7,
            );
            let mut previous: Option<Vec<symbios_avatar::Quat>> = None;
            for (mode, seconds) in script {
                for _ in 0..(seconds / step) as usize {
                    let driven = driver
                        .drive(
                            rig,
                            &Inputs {
                                delta: step,
                                velocity: Vec3::Z * 3.0,
                                flight: mode.map(Flight::new),
                                ..Inputs::default()
                            },
                            level_ground,
                        )
                        .expect("a driven frame");
                    let rotations = driven.pose.forward(rig).rotations;
                    if let Some(was) = previous.as_ref().filter(|_| mode == Some(FlightMode::Land))
                    {
                        let turn = rotations
                            .iter()
                            .zip(was)
                            .map(|(a, b)| turn_between(*b, *a))
                            .fold(0.0, f32::max);
                        if turn > 0.8 {
                            failures.push(format!(
                                "{lineage:?} {carriage:?}: a joint turned {turn:.3} rad in a frame of its landing"
                            ));
                        }
                    }
                    previous = Some(rotations);
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// One flight a host hands on, at 60 frames a second: standing, a takeoff
/// into flapping two beats after `lifted`, half a second of flapping, and a
/// landing back once `settled`. Each frame's pose, stage and flight report.
fn handed_on(avatar: &Avatar, carriage: Carriage) -> Vec<(Pose, &'static str, Option<Flown>)> {
    let rig = &avatar.rig;
    let step = 1.0 / 60.0;
    let mut driver = Driver::new(
        DriverConfig {
            carriage,
            ..DriverConfig::default()
        },
        7,
    );
    let mut frames = Vec::new();
    let mut frame = |driver: &mut Driver, flight: Option<Flight>, speed: f32, stage| {
        let driven = driver
            .drive(rig, &inputs_at(flight, speed, step), level_ground)
            .expect("a driven frame");
        let flown = driver.flight(rig);
        frames.push((driven.pose, stage, flown));
        flown
    };
    for _ in 0..12 {
        frame(&mut driver, None, 0.0, "stand");
    }
    let mut lifted_at = None;
    for _ in 0..300 {
        let flown = frame(
            &mut driver,
            Some(Flight::new(FlightMode::Takeoff)),
            2.0,
            "takeoff",
        )
        .expect("flying");
        if flown.lifted && lifted_at.is_none() {
            lifted_at = Some(flown.phase);
        }
        if lifted_at.is_some_and(|at| flown.phase >= at + 2.0) {
            break;
        }
    }
    for _ in 0..30 {
        frame(
            &mut driver,
            Some(Flight::new(FlightMode::Flap)),
            4.0,
            "flap",
        );
    }
    for _ in 0..300 {
        let flown = frame(
            &mut driver,
            Some(Flight::new(FlightMode::Land)),
            0.0,
            "land",
        )
        .expect("flying");
        if flown.settled {
            break;
        }
    }
    for _ in 0..12 {
        frame(&mut driver, None, 0.0, "after");
    }
    frames
}

/// How much of its trunk, past [`UNDER_GROUND`], a body may draw under where
/// it stood in a takeoff or a landing: a folded bat's hand dipped with its
/// absorb, a glider-led cross's paw turned with its half-spread arm.
const GROUND_SLACK: f32 = 0.015;

/// **Nothing drawn goes under the ground through a takeoff and a landing**
/// (#424, the look review): every family at sizes 0.1, as rolled and 2.4,
/// and every pair crossed at 0.49 and 0.51, under both carriages - a carried
/// body handed no lift by its host, so beating on the ground - flown as a
/// host hands on, at no frame draws anything - skin, toes, membrane or
/// feathers - lower than it stood, past [`UNDER_GROUND`] and
/// [`GROUND_SLACK`] of its trunk. Before: a bat
/// hovering at 0.29 m beat its wings 0.22 m into the ground and 0.45 m in its
/// takeoff's first beats; a bird's crouch drooped its primaries 0.027 m in;
/// a glider's landing sank its paws 0.052 m, a bird's toes 0.042.
///
/// **And a Bird x Bat cross at 0.4 with them** (#440): until then it and the
/// cross at 0.49 stood their folded feathers 36 and 83 mm under the ground,
/// and lower than that nothing went; stood off it, a carried takeoff's first
/// beats put their primaries 11.8 and 25.0 mm into the ground, their joints
/// all reckoned clear of it (`winged::flight`'s raise now reads the cards).
#[test]
fn nothing_drawn_goes_under_the_ground_in_takeoff_or_landing() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies: Vec<(String, AvatarRecord)> = Vec::new();
    for family in [Bird, Bat, Glider] {
        for size in [Some(0.1), None, Some(2.4)] {
            bodies.push((
                format!("{family:?} size {size:?}"),
                sized(Lineage::pure(family), size),
            ));
        }
    }
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.49, 0.51] {
            bodies.push((
                format!("{a:?} x {b:?} {blend}"),
                winged(Lineage::cross(a, b, blend)),
            ));
        }
    }
    bodies.push((
        "Bird x Bat 0.4".to_string(),
        winged(Lineage::cross(Bird, Bat, 0.4)),
    ));
    let mut failures = Vec::new();
    for (name, record) in &bodies {
        let avatar = Avatar::build(record).expect("a winged body builds");
        for carriage in [Carriage::Chassis, Carriage::Own] {
            let frames = handed_on(&avatar, carriage);
            let standing = frames
                .iter()
                .filter(|frame| frame.1 == "stand")
                .map(|frame| lowest_drawn(&avatar, &frame.0))
                .fold(f32::MAX, f32::min)
                .min(0.0);
            let (worst, at) = frames
                .iter()
                .map(|frame| (lowest_drawn(&avatar, &frame.0), frame.1))
                .fold(
                    (f32::MAX, ""),
                    |most, low| if low.0 < most.0 { low } else { most },
                );
            let slack = UNDER_GROUND + GROUND_SLACK * trunk(&avatar.rig);
            if worst < standing - slack {
                failures.push(format!(
                    "{name} {carriage:?}: drew {worst:.4} m in its {at}, standing at {standing:.4}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **A takeoff crouches, pushes and opens its wings as it pushes** (#424, the
/// look review): each family carrying itself, as rolled, sinks its root at
/// least a twentieth of its height through a crouch of at least an eighth of a
/// second with its wings still folded, then pushes off for at least a
/// twelfth of a second, its wings opening during the push - spread a fifth
/// by the time its feet leave. Before: a glider crouched a centimetre for
/// six hundredths of a second and was gone, and a bat sprang up spread.
#[test]
fn a_takeoff_crouches_and_pushes_visibly() {
    for family in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        let avatar = built(&winged(Lineage::pure(family)));
        let rig = &avatar.rig;
        let pelvis = rig.in_zone(Zone::Pelvis)[0];
        let leg = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_reach(limb))
            .fold(0.0f32, f32::max);
        let frames = handed_on(&avatar, Carriage::Own);
        let height = |frame: &(Pose, &str, Option<Flown>)| frame.0.forward(rig).positions[pelvis].y;
        let standing = height(&frames[11]);
        let start = frames
            .iter()
            .position(|frame| frame.1 == "takeoff")
            .unwrap();
        let lift = (start..frames.len())
            .find(|&at| frames[at].2.is_some_and(|flown| flown.lifted))
            .expect("it lifts off");
        let lowest = (start..lift)
            .min_by(|&a, &b| height(&frames[a]).total_cmp(&height(&frames[b])))
            .unwrap();
        let depth = standing - height(&frames[lowest]);
        let step = 1.0 / 60.0;
        let crouch = (lowest - start) as f32 * step;
        let push = (lift - lowest) as f32 * step;
        let folded = frames[lowest].2.unwrap().extension;
        let opened = frames[lift].2.unwrap().extension;
        assert!(
            depth >= rig.extent() / 20.0,
            "{family:?}: crouched {depth:.3} m on a body {:.3} tall (leg {leg:.3})",
            rig.extent()
        );
        assert!(crouch >= 0.125, "{family:?}: crouched for {crouch:.3} s");
        assert!(push >= 0.08, "{family:?}: pushed for {push:.3} s");
        assert!(
            folded <= 0.05,
            "{family:?}: spread {folded:.2} at the crouch's foot"
        );
        assert!(
            opened >= 0.2,
            "{family:?}: spread only {opened:.2} as its feet left"
        );
    }
}

/// **A bird levels out of its upright push gradually** (#424, the look
/// review): after lift-off its trunk sets off levelling from rest - the
/// first frame turns it less than a hundredth of the way - and never levels
/// more than 3.5 hundredths of the way in a frame at 120 a second. Eased by
/// a first-order lag, it levelled fastest in its first frame, 5.4
/// hundredths, a snap.
#[test]
fn a_bird_levels_out_of_its_push_gradually() {
    let avatar = built(&winged(Lineage::pure(WingedFamily::Bird)));
    let rig = &avatar.rig;
    let mut driver = Driver::new(
        DriverConfig {
            carriage: Carriage::Own,
            ..DriverConfig::default()
        },
        7,
    );
    for _ in 0..24 {
        driver.drive(rig, &inputs(None, 0.0), level_ground);
    }
    let mut levels = Vec::new();
    let mut lifted = None;
    for frame in 0..(1.0 / STEP) as usize {
        driver.drive(
            rig,
            &inputs(Some(Flight::new(FlightMode::Takeoff)), 2.0),
            level_ground,
        );
        let flown = driver.flight(rig).unwrap();
        if flown.lifted && lifted.is_none() {
            lifted = Some(frame);
        }
        levels.push(flown.level);
    }
    let lifted = lifted.expect("it lifts off");
    let steps: Vec<f32> = levels.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(
        steps[lifted - 1] < 0.01,
        "levelled {:.3} in its first frame off the ground",
        steps[lifted - 1]
    );
    let most = steps[lifted - 1..].iter().copied().fold(0.0f32, f32::max);
    assert!(most <= 0.035, "levelled {most:.3} in a frame");
}

/// **A body standing on its wings folds down onto its feet over a span that
/// reads** (#424, the look review): a glider, and the bat-glider and
/// bird-glider crosses at 0.49 and 0.51, landing as a host hands on at 60
/// frames a second, under both carriages, turn no joint more than a quarter
/// radian in a frame from touchdown to the hand back. Folded at a wing's own
/// pace, a glider's spread legs swung down and planted in two frames and
/// turned 0.42 radians in one, its patagium flipping with them.
#[test]
fn a_glider_folds_onto_its_feet_over_a_span() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut lineages = vec![Lineage::pure(Glider)];
    for blend in [0.49, 0.51] {
        lineages.push(Lineage::cross(Bat, Glider, blend));
        lineages.push(Lineage::cross(Bird, Glider, blend));
    }
    let mut failures = Vec::new();
    for lineage in lineages {
        let avatar = built(&winged(lineage));
        let rig = &avatar.rig;
        for carriage in [Carriage::Chassis, Carriage::Own] {
            let frames = handed_on(&avatar, carriage);
            let rotations: Vec<Vec<symbios_avatar::Quat>> = frames
                .iter()
                .map(|frame| frame.0.forward(rig).rotations)
                .collect();
            let turns = turned(&rotations);
            let most = (1..frames.len())
                .filter(|&at| frames[at].1 == "land" || frames[at - 1].1 == "land")
                .map(|at| turns[at - 1])
                .fold(0.0f32, f32::max);
            if most > 0.25 {
                failures.push(format!(
                    "{lineage:?} {carriage:?}: a joint turned {most:.3} rad in a frame of its landing"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
