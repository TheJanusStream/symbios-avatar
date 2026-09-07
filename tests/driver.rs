//! What [`Driver`] does to a body, read off the pose the body is drawn in.
//!
//! Every test here drives the real thing and measures the pose it produced.
//! None of them re-runs the stage sequence itself and asserts on that, which is
//! the trap this suite exists because of: deleting a stage from the driver
//! leaves such a test passing on its own copy of the sequence, and that is
//! precisely how a walk went out with no heel-strike and no toe-off for as long
//! as the stage existed.
//!
//! # Where the bodies and the figures come from
//!
//! The figures below are the ones the consuming application's own instruments
//! read, and the bodies are the ones they stand on — reproduced by seed in
//! [`body_of`] rather than invented here. That is deliberate and it is most of
//! the value of this file: it makes each reading directly comparable with the
//! reading taken downstream, so a driver that has been moved between crates can
//! be shown to have arrived intact rather than merely to still work.
//!
//! A body's proportions decide every one of these numbers, so a figure quoted
//! against one seed says nothing about another.

use symbios_avatar::anim::driver::{
    Carriage, Driven, Driver, DriverConfig, Inputs, Showing, Source, level_ground, velocity_of,
};
use symbios_avatar::anim::gait;
use symbios_avatar::anim::gesture;
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Leap, Limb, Pose, Rig, Speed, Vec3, Zone, plan,
};

/// The seed every instrument here stands its body's idle and blink on.
///
/// **Pinned, because several of these figures are a function of it.** An idle's
/// seed decides when its settling weight shift fires and which leg it moves
/// first, which is exactly the mechanism that steps a stopped foot home — so a
/// stop measured on a body seeded from anywhere the test cannot see is a
/// measurement of how many bodies were built before it. Seven is what the
/// application's own stop instrument pins.
const INSTRUMENT_SEED: u64 = 7;

/// One frame at sixty a second, in seconds.
const STEP: f32 = 1.0 / 60.0;

/// FNV-1a over the bytes of a string.
///
/// Here only to reproduce the identity hash the consuming application seeds a
/// handed-out body from — see [`body_of`].
fn fnv1a_64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

/// The body the consuming application hands an identity that has never opened
/// an editor, for the identity `did`.
///
/// **Reproduced rather than invented**, so that every figure in this file is
/// the figure the same instrument reads downstream. Three steps, and all three
/// matter to the proportions: the roll, the stature clamp — the engine's
/// exploration envelope is right for an editor and wrong for a body somebody is
/// *given*, since roughly one seed in thirty would hand out a 20 cm or a 3 m
/// person — and the re-quantisation the clamp makes necessary.
///
/// Built at a small atlas because nothing here looks at the skin. It is the
/// **rig** these tests need, and a rig only exists once a body has been built:
/// the extremities bring their own joints, so the sole points every foot
/// measurement below reads are not in the record's skeleton at all.
fn body_of(did: &str) -> Avatar {
    let seed = fnv1a_64(did) as i64;
    let mut record = AvatarRecord::rolled("Wanderer", Archetype::default(), seed);
    let (low, high) = plan::humanoid_height_range();
    if let Archetype::Humanoid(params) = &mut record.archetype {
        params.height = params.height.clamp(low, high);
    }
    record.sanitize();
    Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("a seeded default body builds")
}

/// Something else carrying a body, exactly as a physics chassis does.
///
/// The driver is told where the body has got to and works its speed out from
/// that, which is the path a remote peer takes — and, on the first frame, the
/// path that has no previous position to difference against and therefore no
/// speed. That first standing frame is kept rather than smoothed away: it is
/// what the application does, and a harness that starts a body already walking
/// measures a different cycle.
struct Chassis {
    driver: Driver,
    last: Option<Vec3>,
}

impl Chassis {
    fn seeded(seed: u64) -> Self {
        Self {
            driver: Driver::seeded(seed),
            last: None,
        }
    }

    /// The same, carried by nothing — the instrument's body, which walks on the
    /// spot and owns its own root.
    fn carrying_itself(seed: u64) -> Self {
        let mut chassis = Self::seeded(seed);
        chassis.driver.set_config(DriverConfig {
            carriage: Carriage::Own,
            ..DriverConfig::default()
        });
        chassis
    }

    /// Put the body at `at` and drive one frame.
    fn step(&mut self, rig: &Rig, at: Vec3) -> Driven {
        self.drive(rig, at, false, None, None)
    }

    /// The same, in deep water.
    fn swim(&mut self, rig: &Rig, at: Vec3) -> Driven {
        self.drive(rig, at, true, None, None)
    }

    /// The same, asking for a gesture on this frame.
    fn ask(&mut self, rig: &Rig, at: Vec3, gesture: &str) -> Driven {
        self.drive(rig, at, false, Some(gesture), None)
    }

    fn drive(
        &mut self,
        rig: &Rig,
        at: Vec3,
        swimming: bool,
        gesture: Option<&str>,
        showing: Option<Showing>,
    ) -> Driven {
        let velocity = self
            .last
            .map_or(Vec3::ZERO, |last| velocity_of(last, at, STEP));
        self.last = Some(at);
        self.driver
            .drive(
                rig,
                &Inputs {
                    delta: STEP,
                    velocity,
                    at,
                    swimming,
                    gesture,
                    showing,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("an unheld body is posed every frame")
    }
}

/// How far a foot's sole is pitched from its own rest attitude, in degrees,
/// positive toe-up.
///
/// Measured on the POSED body between the rearmost and foremost joints past the
/// ankle, and referenced to the rest attitude rather than to level, because a
/// body's foot nodes run a few degrees uphill at rest and zero has to mean
/// "carried as it stands".
fn sole_pitch(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
    let joints = rig.extremity_joints(limb);
    let sole = &joints[1..];
    let along = |&joint: &usize| rig.joints[joint].position.z;
    let rear = *sole
        .iter()
        .min_by(|a, b| along(a).total_cmp(&along(b)))
        .expect("a foot");
    let fore = *sole
        .iter()
        .max_by(|a, b| along(a).total_cmp(&along(b)))
        .expect("a foot");
    let angle = |run: Vec3| {
        run.y
            .atan2((run.x * run.x + run.z * run.z).sqrt())
            .to_degrees()
    };
    let posed = pose.forward(rig);
    angle(posed.positions[fore] - posed.positions[rear])
        - angle(rig.joints[fore].position - rig.joints[rear].position)
}

/// The ankle joint of each hind limb, which is where a foot hangs from.
fn feet(rig: &Rig) -> Vec<usize> {
    [Limb::HindLeft, Limb::HindRight]
        .into_iter()
        .map(|limb| rig.in_zone(Zone::Extremity(limb))[0])
        .collect()
}

#[test]
fn the_procedural_walk_lands_toe_up_and_leaves_toe_down() {
    // **The stage that went missing.** A consumer that spells the drive
    // sequence out itself is a consumer that can forget `roll_feet`, and one
    // did: every procedurally driven body walked with its soles held at their
    // rest attitude — no heel-strike, no toe-off, the whole foot tilting with
    // the shin at full stride. Driving through the driver and reading the pose
    // it produced is the only form of this test that can see it.
    let avatar = body_of("did:plc:ankle-test");
    let rig = avatar.rig.clone();
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    const PACE: f32 = 1.3;

    let (mut lowest, mut highest) = (f32::MAX, f32::MIN);
    for frame in 0..150 {
        let driven = chassis.step(&rig, Vec3::Z * (PACE * STEP * frame as f32));
        if driven.source != Source::Gait {
            continue;
        }
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let pitch = sole_pitch(&rig, &driven.pose, limb);
            lowest = lowest.min(pitch);
            highest = highest.max(pitch);
        }
    }
    assert!(
        lowest < f32::MAX,
        "the gait never drove the body — the test measured nothing"
    );
    // The literature's bands, which the engine's constants are set against:
    // heel-strike 15-25 degrees toe-up, push-off 15-20 toe-down. Asserted
    // loosely, because what is guarded is that the stage RUNS — a sole held
    // flat all cycle reads zero to zero and is a shuffle.
    assert!(
        highest > 10.0,
        "the foot never landed toe-up: peak pitch {highest:.1} deg — the ankle roll is not running"
    );
    assert!(
        lowest < -10.0,
        "the foot never left toe-down: lowest pitch {lowest:.1} deg — the ankle roll is not running"
    );
}

/// Walks one body at a fixed speed and reports the widest fore-and-aft split
/// between its feet and the highest its pelvis rode above standing height.
fn walked_at(rig: &Rig, metres_per_second: f32) -> (f32, f32) {
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let pelvis = rig
        .joints
        .iter()
        .position(|joint| joint.parent.is_none())
        .expect("a root joint");
    let standing = rig.joints[pelvis].position.y;
    let feet = feet(rig);

    let (mut split, mut crest) = (0.0f32, f32::MIN);
    for frame in 0..240 {
        let driven = chassis.step(rig, Vec3::Z * (metres_per_second * STEP * frame as f32));
        if driven.source != Source::Gait {
            continue;
        }
        let posed = driven.pose.forward(rig);
        split = split.max((posed.positions[feet[0]].z - posed.positions[feet[1]].z).abs());
        crest = crest.max(posed.positions[pelvis].y - standing);
    }
    assert!(crest > f32::MIN, "the gait never drove the body");
    (split, crest)
}

#[test]
fn a_faster_body_takes_a_longer_step_and_eventually_leaves_the_ground() {
    // **One number in, everything out.** A driver that expresses speed by
    // bending the cadence alone gives a sprinting body a stroller's step taken
    // very quickly, and everything scaled by the stride inherits it — the trunk
    // lean is scaled by the stride against the legs taking it, so with the
    // stride pinned that ratio never moves. Both claims are read off the drawn
    // pose.
    let avatar = body_of("did:plc:speed-test");
    let rig = avatar.rig.clone();
    let (slow_split, slow_crest) = walked_at(&rig, 1.0);
    let (brisk_split, _) = walked_at(&rig, 1.8);
    let (_, fast_crest) = walked_at(&rig, 3.0);

    // **Both samples are walks, deliberately.** A foot's split is its excursion
    // — how far it slides back under the body across one stance — and that
    // legitimately FALLS when a body starts running, because a running foot is
    // down for a third of the cycle instead of two thirds. Comparing a walk
    // against a run here reads the gait change as a shorter stride and asserts
    // the opposite of the truth.
    assert!(
        brisk_split > slow_split * 1.15,
        "a body at 1.8 m/s split its feet {brisk_split:.3} m against {slow_split:.3} at 1.0 — \
         the stride is pinned rather than derived"
    );
    // A walking body never rises above its standing height; a running one is a
    // projectile between steps and does. That is the cleanest sign in a drawn
    // pose that the gait changed on its own.
    assert!(
        slow_crest <= 1e-3,
        "a walking body rode {:.1} mm above standing height",
        slow_crest * 1000.0
    );
    assert!(
        fast_crest > 0.005,
        "a body at 3 m/s never left the ground: crest {:.1} mm — it is still walking",
        fast_crest * 1000.0
    );
}

/// Accelerates one body across the walk-run transition and reports the largest
/// single-frame step the leading contact took **through its own step**, as a
/// fraction of one.
///
/// **Not a distance, and that is the point.** A foot legitimately travels every
/// frame, and at a run it travels a long way; millimetres cannot separate a body
/// moving fast from a body whose clock was relabelled under it. What a change of
/// gait must not do is move a contact to a different part of its step, so the
/// reading is taken on an axis that does not move when the duty does — half for
/// the stance, half for the swing.
fn worst_phase_step(rig: &Rig, from: f32, to: f32, seconds: f32, fps: f32) -> f32 {
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let step_secs = 1.0 / fps;
    let frames = (seconds / step_secs) as usize;
    let mut at = Vec3::ZERO;
    let mut last: Option<Vec3> = None;
    let mut last_phase: Option<f32> = None;
    let mut worst = 0.0f32;
    let mut crossed = (false, false);

    for frame in 0..frames {
        let pace = from + (to - from) * (frame as f32 / frames as f32);
        at += Vec3::Z * (pace * step_secs);
        let velocity = last.map_or(Vec3::ZERO, |last| velocity_of(last, at, step_secs));
        last = Some(at);
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta: step_secs,
                    velocity,
                    at,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("posed");
        if driven.source != Source::Gait {
            continue;
        }
        // **The speed the DRIVER fed the gait, not a re-derivation from the raw
        // ramp.** The pace the clock advances on is eased, so a gait rebuilt
        // here from the instantaneous speed crosses the walk-run duty step on a
        // different frame than the driven one did — and a phase read against
        // the wrong duty reports a near-full-step relabel that never reached a
        // body.
        let Some(speed) = driver.speed() else {
            continue;
        };
        crossed = (
            crossed.0 || !speed.is_running(),
            crossed.1 || speed.is_running(),
        );
        let phase = match speed.gait(rig).phase(0, driver.cycle()) {
            gait::Phase::Stance(t) => t * 0.5,
            gait::Phase::Swing(t) => 0.5 + t * 0.5,
        };
        if let Some(was) = last_phase {
            worst = worst.max((phase - was).rem_euclid(1.0));
        }
        last_phase = Some(phase);
    }
    assert!(
        crossed.0 && crossed.1,
        "the sweep never crossed the walk-run transition — it measured one gait"
    );
    worst
}

#[test]
fn crossing_the_walk_run_boundary_does_not_relabel_the_clock() {
    // The duty falls all the way along the speed axis and STEPS at the
    // transition, so a cycle fraction handed across unchanged means a different
    // part of the step on the other side, and a foot in mid-swing arrives
    // planted.
    //
    // Asked the only way a discontinuity can be told from a fast motion: sample
    // twice as finely and see whether the largest step halves. A cliff does not
    // halve; a cadence does.
    let avatar = body_of("did:plc:transition-test");
    let rig = avatar.rig.clone();
    let steps: Vec<f32> = [60.0, 120.0, 240.0]
        .into_iter()
        .map(|fps| worst_phase_step(&rig, 1.4, 3.2, 3.0, fps))
        .collect();
    // Six tenths rather than a half: halving is what a smooth motion does
    // exactly, and the slack is for where two samplings straddle the peak
    // differently.
    for pair in steps.windows(2) {
        assert!(
            pair[1] <= pair[0] * 0.6,
            "the leading contact's phase stepped {:.4} of a step and then {:.4} at twice the \
             frame rate — a step that does not halve when the sampling doubles is a cliff, and \
             the clock was relabelled under the body",
            pair[0],
            pair[1],
        );
    }
}

/// How far a foot standing on the ground slides while the body changes speed
/// without ever stopping, in metres.
///
/// **The control is the whole instrument.** A walking body slides its feet a
/// little at the best of times, so a figure from a decelerating walk means
/// nothing until the same body walking at a CONSTANT speed through the same
/// window has been read the same way. Pass `from == to` for that control.
///
/// The body never drops below the idle threshold, so whatever this measures
/// belongs to the gait.
fn skate_through(
    rig: &Rig,
    walk_frames: usize,
    from: f32,
    to: f32,
    ramp_frames: usize,
    over: Option<&str>,
) -> f32 {
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    // **The SOLE points, not the ankle and not the sole joints.** A planted
    // foot rolls heel to toe through its stance, which translates the ankle
    // horizontally while the sole under it has not moved at all — asking the
    // ankle reads the roll as a skate, identically at every speed, which is
    // what a measurement of the wrong thing looks like when the wrong thing is
    // deterministic. A sole joint sits above the sole it belongs to and has the
    // same problem one level down. The sole point is the joint's rest position
    // dropped to the ground plane, carried into the pose by the ankle it hangs
    // from — which is how the roll itself models a sole.
    let soles: Vec<(usize, Vec<usize>)> = [Limb::HindLeft, Limb::HindRight]
        .into_iter()
        .filter_map(|limb| {
            let joints = rig.extremity_joints(limb);
            let (&ankle, sole) = (joints.first()?, joints.get(1..)?);
            Some((ankle, sole.to_vec()))
        })
        .collect();

    let mut at = Vec3::ZERO;
    for _ in 0..walk_frames {
        at += Vec3::Z * (from * STEP);
        chassis.step(rig, at);
    }

    let mut track: Vec<Vec<Vec<Vec3>>> = Vec::with_capacity(120);
    let mut down: Vec<Vec<bool>> = Vec::with_capacity(120);
    for step in 0..120 {
        let speed = if step < ramp_frames {
            let done = (step as f32 + 0.5) / ramp_frames as f32;
            from + (to - from) * done
        } else {
            to
        };
        at += Vec3::Z * (speed * STEP);
        // The gesture is asked for on the first measured frame, so the window
        // covers its whole play plus the blend that ends it.
        let driven = match over {
            Some(name) if step == 0 => chassis.ask(rig, at, name),
            _ => chassis.step(rig, at),
        };
        let stance: Vec<bool> = {
            let gait = chassis.driver.speed().map(|speed| speed.gait(rig));
            let cycle = chassis.driver.cycle();
            [Limb::HindLeft, Limb::HindRight]
                .iter()
                .map(|limb| {
                    gait.as_ref().is_some_and(|gait| {
                        gait.limbs
                            .iter()
                            .position(|which| which == limb)
                            .is_some_and(|index| gait.phase(index, cycle).is_stance())
                    })
                })
                .collect()
        };
        let posed = driven.pose.forward(rig);
        track.push(
            soles
                .iter()
                .map(|(ankle, sole)| {
                    sole.iter()
                        .map(|&joint| {
                            let rest = rig.joints[joint].position;
                            at + posed.positions[*ankle]
                                + posed.rotations[*ankle]
                                    * (Vec3::new(rest.x, 0.0, rest.z) - rig.joints[*ankle].position)
                        })
                        .collect()
                })
                .collect(),
        );
        down.push(stance);
    }
    assert_eq!(
        chassis.driver.source(),
        Source::Gait,
        "the body must still be walking, or this is measuring a stop"
    );

    // **Each sole point judged while IT is on the ground, against itself.** Two
    // gates, because a stance foot is not one rigid thing: the gait says the
    // FOOT is bearing, and each point's own height says whether that point is
    // the one in contact right now. Contact transfers heel to toe through a
    // stance, and a heel that lifts has moved without sliding. Per point
    // against itself, never against the lowest point of the moment: an argmin
    // whose identity moves compares a heel against a toe.
    const CLEARANCE: f32 = 0.005;
    let mut skate = 0.0f32;
    for which in 0..soles.len() {
        for point in 0..soles[which].1.len() {
            let floor = track
                .iter()
                .map(|frame| frame[which][point].y)
                .fold(f32::MAX, f32::min);
            let mut anchor = None;
            for (frame, stance) in track.iter().zip(&down) {
                let world = frame[which][point];
                if stance[which] && world.y - floor <= CLEARANCE {
                    let from = *anchor.get_or_insert(world);
                    skate = skate.max(Vec3::new(world.x - from.x, 0.0, world.z - from.z).length());
                } else {
                    anchor = None;
                }
            }
        }
    }
    skate
}

/// How far a planted sole may slide at a constant speed, in metres.
///
/// Eight millimetres. The swept figure is a few, and the defect this guards
/// against read 16.6 to 17.7 on the same instrument, so it sits between the two
/// rather than just above the passing number.
const STEADY_SKATE_CEILING: f32 = 0.008;

#[test]
fn a_walking_body_holds_its_planted_sole_at_every_pace() {
    // **The acceptance is the CURVE rather than a figure.** The defect this
    // guards was filed on two speeds, and a pair cannot show a shape; swept, it
    // is a threshold between 1.0 and 1.2 m/s, which is neither of the two
    // candidates a pair suggested. A guard at 1.4 alone would have passed
    // throughout the defect's life at 0.7.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    for metres in [0.4f32, 0.7, 1.0, 1.2, 1.4, 1.6, 1.8] {
        let skate = skate_through(&rig, 120, metres, metres, 1, None);
        assert!(
            skate < STEADY_SKATE_CEILING,
            "at a constant {metres} m/s a planted sole slid {:.1} mm, against a ceiling of {:.1}",
            skate * 1000.0,
            STEADY_SKATE_CEILING * 1000.0
        );
    }
}

#[test]
fn a_bow_over_a_walk_keeps_its_planted_soles() {
    // A bow is one ankle-to-crown line: it pitches the pelvis — hip extension —
    // and swings the hip sockets on an arc, so at the clip level the leg chain
    // moves. The planted feet are the DRIVER's promise rather than the clip's,
    // because the settle tail plants stance contacts after the overlay, and
    // only a full-drive instrument can ask for that promise.
    //
    // The ceiling is not the steady-pace one, and the difference is the gesture
    // blend, priced deliberately: a gesture starts and ends through an
    // inertializer applied AFTER the settle tail, so through those two windows
    // the drawn foot is a mix of the settled walk and the bowed walk. A wave
    // moves no leg and its blend drags nothing; a whole-body bow gives the
    // blend a little hip line to mix across.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let bowed = skate_through(&rig, 120, 1.0, 1.0, 1, Some("Bow"));
    assert!(
        bowed < 0.020,
        "bowing over a walk slid a planted sole {:.1} mm, against a ceiling of 20.0 — a raw \
         uncompensated leg track reads an order of magnitude past that",
        bowed * 1000.0,
    );
}

/// Walks a body up to speed, stops it dead, and reports the furthest a foot
/// that was BEARING WEIGHT at that moment then slid through the world, in
/// metres.
fn skid_through_a_stop(rig: &Rig, walk_frames: usize) -> f32 {
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let feet = feet(rig);
    const PACE: f32 = 1.4;

    let mut at = Vec3::ZERO;
    for _ in 0..walk_frames {
        at += Vec3::Z * (PACE * STEP);
        chassis.step(rig, at);
    }
    assert_eq!(
        chassis.driver.source(),
        Source::Gait,
        "the body must be walking when it is stopped"
    );

    // Which feet are carrying the body at the instant it stops, and where they
    // are. Asked of the gait the driver is actually running.
    let gait = Speed::new(rig, PACE).gait(rig);
    let cycle = chassis.driver.cycle();
    let planted: Vec<usize> = gait
        .limbs
        .iter()
        .enumerate()
        .filter(|(index, _)| gait.phase(*index, cycle).is_stance())
        .filter_map(|(_, limb)| {
            [Limb::HindLeft, Limb::HindRight]
                .iter()
                .position(|which| which == limb)
        })
        .collect();
    assert!(!planted.is_empty(), "a walking body has a foot down");

    // **World positions, not body-local ones.** With a dead stop the two differ
    // by a constant and the distinction would not matter, but a body-local
    // reading calls a correctly planted foot's backward travel a skate the
    // moment the chassis is still moving during the measurement.
    let start: Vec<Vec3> = {
        let posed = chassis
            .driver
            .posed()
            .expect("the body has been driven")
            .forward(rig);
        planted
            .iter()
            .map(|&foot| at + posed.positions[feet[foot]])
            .collect()
    };

    // The chassis stops. A foot that was down must stay where the ground is
    // while the body works out that it has stopped.
    //
    // **Collected, then judged**, because whether a foot was DOWN on a given
    // frame is only knowable against the lowest that foot gets, and that is not
    // known until the window is over.
    let mut track: Vec<Vec<Vec3>> = Vec::with_capacity(60);
    for _ in 0..60 {
        let driven = chassis.step(rig, at);
        let posed = driven.pose.forward(rig);
        track.push(
            planted
                .iter()
                .map(|&foot| at + posed.positions[feet[foot]])
                .collect(),
        );
    }

    // **A foot ON THE GROUND may not move; a foot in the air may.** That is the
    // whole definition of a skate, it is what a viewer actually sees, and it
    // needs no bookkeeping from the driver — which is the point, because an
    // idle deliberately LIFTS an unweighted foot to step it home, and a rule
    // that asked the gait instead would count that step as a skate.
    const CLEARANCE: f32 = 0.005;
    let mut skid = 0.0f32;
    for which in 0..planted.len() {
        let floor = track
            .iter()
            .map(|frame| frame[which].y)
            .fold(f32::MAX, f32::min);
        let mut anchor = Some(start[which]);
        for frame in &track {
            let world = frame[which];
            if world.y - floor <= CLEARANCE {
                let from = *anchor.get_or_insert(world);
                skid = skid.max(world.distance(from));
            } else {
                anchor = None;
            }
        }
    }
    skid
}

#[test]
fn a_stop_does_not_skate() {
    // **A body that stops walking must not drag the foot it was standing on.**
    // Blending from mid-stride into a stand does exactly that, and how much
    // depends entirely on WHEN in the step the body stopped — tens of
    // millimetres near midstance against hundreds at the worst phase, which is
    // why this is swept over a whole cycle's worth of stopping phases rather
    // than measured at one.
    //
    // TWO FIXES ON THE TRANSITION'S TIMING WERE TRIED AND BOTH LOST. Holding
    // the change until the next handoff made it worse; holding it until the
    // next midstance — the moment the drag is provably zero — also made it
    // worse. Neither can win, because a body that holds keeps striding while
    // whatever stopped it has stopped: the WAIT IS ITSELF A SKATE, at about
    // 1.23 m per cycle held on a body this size, and the best moment only saves
    // a third of that.
    //
    // What fixed it was a mechanism rather than a moment: the idle is handed
    // the stance the body arrived in, pins the contacts that were bearing
    // weight, and steps them home one at a time on its own weight shifts — a
    // foot only ever moving while it is unloaded and off the ground.
    //
    // **The figure is pinned, and it is the check on the port.** The consuming
    // application reads 57.85 mm from this instrument on this body and this
    // seed, in every process context. A reading a millimetre or two away is an
    // arithmetic site that has not been routed through the crate's own
    // deterministic maths; the high 50s to 80s is a body standing on an
    // unpinned idle seed; the regime this guard exists to catch reads hundreds.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let worst = (100..=130)
        .map(|frames| skid_through_a_stop(&rig, frames))
        .fold(0.0f32, f32::max);
    // Printed as well as judged, so every run records its figure rather than
    // only its verdict.
    println!("worst stop skid: {:.2} mm", worst * 1000.0);
    assert!(
        worst < 0.06,
        "a foot standing on the ground slid {:.1} mm through a stop, against 57.85 mm downstream \
         on the same body and seed",
        worst * 1000.0
    );
}

/// What one driven jump did, read off the drawn body.
struct Jumped {
    /// Whether the walk cycle ever advanced while the body was in flight.
    marched: bool,
    /// Whether a foot was ever at the floor while the body was in flight.
    planted: bool,
    /// How high the lowest foot got at the apex, in metres.
    highest: f32,
    /// Whether the landing was ever reached.
    landed: bool,
    /// How far the root sank below its standing height during the landing.
    sank: f32,
    /// How far the lowest foot went under the floor during the landing.
    buried: f32,
}

/// Drives one body through a whole jump on a chassis that moves the way a
/// physics engine moves one — an impulse, then gravity, caught by a floor
/// `ledge` metres below the one it left.
fn jumped(rig: &Rig, launch: f32, ledge: f32) -> Jumped {
    const GRAVITY: f32 = 9.81;
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let feet = feet(rig);

    // Walk in first, so the body is in a gait when it leaves the ground and the
    // walk cycle has something to advance.
    let mut at = Vec3::ZERO;
    for _ in 0..90 {
        at += Vec3::Z * (1.4 * STEP);
        chassis.step(rig, at);
    }

    let landing_y = at.y - ledge;
    let mut vertical = launch;
    let (mut marched, mut planted, mut highest, mut landed) = (false, false, f32::MIN, false);
    let (mut sank, mut buried) = (0.0f32, 0.0f32);
    for _ in 0..600 {
        vertical -= GRAVITY * STEP;
        at += Vec3::new(0.0, vertical * STEP, 1.4 * STEP);
        let caught = at.y <= landing_y;
        if caught {
            at.y = landing_y;
            vertical = 0.0;
        }
        let before = chassis.driver.cycle();
        let driven = chassis.step(rig, at);
        let airborne = chassis.driver.airborne();
        let in_flight =
            airborne.is_some_and(|air| !air.leap.stage_at(rig, air.elapsed).is_grounded());
        landed |= airborne.is_some_and(|air| air.landed);
        if in_flight {
            marched |= (chassis.driver.cycle() - before).abs() > 1e-6;
            let posed = driven.pose.forward(rig);
            let lowest = feet
                .iter()
                .map(|&joint| posed.positions[joint].y)
                .fold(f32::MAX, f32::min);
            highest = highest.max(lowest);
            // A foot at the floor while the body is in the air is a foot the
            // plant grabbed.
            planted |= lowest.abs() < 1e-4;
        }
        if airborne.is_some_and(|air| air.landed) {
            sank = sank.max(-driven.pose.translation.y);
            let posed = driven.pose.forward(rig);
            let lowest = feet
                .iter()
                .map(|&joint| posed.positions[joint].y)
                .fold(f32::MAX, f32::min);
            buried = buried.max(-lowest);
        }
        if caught && driven.source != Source::Leap && landed {
            break;
        }
    }
    assert!(highest > f32::MIN, "the body never went airborne at all");
    Jumped {
        marched,
        planted,
        highest,
        landed,
        sank,
        buried,
    }
}

#[test]
fn a_body_in_the_air_does_not_march_or_plant_a_foot() {
    // **The apex is the whole of it.** A driver that calls a body airborne when
    // its vertical speed exceeds a threshold is right at launch, WRONG through
    // the middle of the jump — at the apex the vertical speed is zero, which is
    // the most airborne a body ever is — and right again on the way down. So
    // the walk cycle resumes and the feet are planted at the top of every jump.
    // Airborne has to be a state.
    //
    // Driven at a real preset's launch speed: 450 newton-seconds on an 80 kg
    // body is 5.6 m/s, which is a 1.6 m apex and over a second of flight — a
    // long time to be marching.
    let avatar = body_of("did:plc:leap-test");
    let rig = avatar.rig.clone();
    let jump = jumped(&rig, 5.6, 0.0);
    assert!(
        !jump.marched,
        "the walk cycle advanced while the body was in the air"
    );
    assert!(
        !jump.planted,
        "a foot was planted on the floor while the body was in the air"
    );
    // The tuck: the feet draw up under the body at mid-flight, so a foot at the
    // apex sits well above where it stands.
    assert!(
        jump.highest > 0.05,
        "the feet never drew up under the body: the highest the lowest foot reached was {:.1} mm \
         above the floor",
        jump.highest * 1000.0
    );
    assert!(jump.landed, "the leap never reached its landing");
}

#[test]
fn a_landing_bends_the_legs_and_does_not_bury_the_body() {
    // **Reported as "the landings end up underground", and they did.** A
    // landing built as a fall carries the drop in the pose for the whole stage,
    // because in the engine's own model the body really is that much lower,
    // having landed on a floor below the one it left. Where a chassis has
    // already carried it down, the two add and the root goes under by the
    // entire fall height — and the legs then cannot reach up to the floor, so
    // the body stays buried through the landing.
    //
    // Two readings, both of the symptom rather than of the arithmetic behind
    // it: how far the ROOT sank, which must be a leg compressing and no more,
    // and how far the lowest FOOT went under the floor, which must be nothing.
    let avatar = body_of("did:plc:leap-test");
    let rig = avatar.rig.clone();
    let jump = jumped(&rig, 5.6, 0.0);
    assert!(jump.landed, "the leap never reached its landing");
    // A leg is clamped to a fraction of its own reach, so half a metre is past
    // anything a landing can legitimately ask for and nowhere near the metre
    // and a half the defect produced.
    assert!(
        jump.sank < 0.5,
        "the root sank {:.0} mm into the floor on landing — a leg compressing cannot account \
         for that",
        jump.sank * 1000.0,
    );
    assert!(
        jump.buried < 0.01,
        "a foot ended {:.0} mm under the floor on landing",
        jump.buried * 1000.0,
    );
    // And the landing must actually be a landing: a body that absorbs nothing
    // has not bent its legs at all.
    assert!(
        jump.sank > 0.02,
        "the root sank {:.0} mm — the landing is not absorbing anything",
        jump.sank * 1000.0,
    );
}

#[test]
fn stepping_off_a_ledge_flies_before_it_lands() {
    // The second defect in the same line. A body that walks off an edge never
    // launches, and a leap built from no speed has a flight of zero — so the
    // stage machine divides by an epsilon and reports a LANDING from the first
    // airborne frame: feet planted in mid-air, all the way down. Built from the
    // speed instead, a fall gets a real arc.
    //
    // Driven with no launch at all, which is the case the jump test cannot see.
    let avatar = body_of("did:plc:leap-test");
    let rig = avatar.rig.clone();
    let fall = jumped(&rig, 0.0, 2.0);
    assert!(
        !fall.planted,
        "a foot was planted in mid-air while the body was falling"
    );
    assert!(
        !fall.marched,
        "the walk cycle advanced while the body was falling"
    );
    assert!(fall.landed, "the fall never reached its landing");
    assert!(
        fall.buried < 0.01,
        "a foot ended {:.0} mm under the floor after a fall",
        fall.buried * 1000.0,
    );
}

#[test]
fn the_two_carriages_differ_by_exactly_the_flight() {
    // **The one decision a caller cannot avoid, made visible.** A body carried
    // by something else must not be carried again by its own leap, and a body
    // that carries itself must be. Same leap, same phase, same body: the only
    // difference between the two poses is the flight height, and it is the
    // whole difference.
    let avatar = body_of("did:plc:leap-test");
    let rig = avatar.rig.clone();
    let leap = Leap::to_height(0.4);
    // A quarter of the way through, which is inside the flight for a jump this
    // size rather than in the wind-up or the landing.
    let phase = 0.5;

    let showing = |carriage: Carriage| {
        let mut driver = Driver::new(
            DriverConfig {
                carriage,
                ..DriverConfig::default()
            },
            INSTRUMENT_SEED,
        );
        driver
            .drive(
                &rig,
                &Inputs {
                    delta: STEP,
                    cycle: Some(phase),
                    showing: Some(Showing::Leap(leap)),
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("posed")
    };
    let carried = showing(Carriage::Chassis);
    let itself = showing(Carriage::Own);

    assert_eq!(carried.source, Source::Leap);
    assert_eq!(itself.source, Source::Leap);
    let flight = leap.height_at(&rig, phase * leap.duration(&rig));
    assert!(
        flight > 0.05,
        "the sampled phase is not in the flight: the root is {:.0} mm up",
        flight * 1000.0
    );
    assert!(
        (itself.pose.translation.y - carried.pose.translation.y - flight).abs() < 1e-4,
        "a self-carried body rode {:.1} mm above a chassis-carried one, against a flight of \
         {:.1} mm — the height is not being given back exactly",
        (itself.pose.translation.y - carried.pose.translation.y) * 1000.0,
        flight * 1000.0
    );
}

/// Drives one body through deep water at `pace` and reports whether a foot was
/// ever planted and how far the trunk ended up pitched onto its front.
///
/// The pitch is read as the angle between the body's own long axis — root to
/// head on the posed skeleton — and the world's vertical, so it is a property
/// of the drawn body rather than a number handed back by the thing under test.
fn swam(rig: &Rig, pace: f32) -> (bool, f32) {
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let feet = feet(rig);
    let pelvis = rig
        .joints
        .iter()
        .position(|joint| joint.parent.is_none())
        .expect("a root joint");
    let head = *rig.in_zone(Zone::Head).first().expect("a head");

    let mut at = Vec3::ZERO;
    let mut planted = false;
    let mut pitch = 0.0f32;
    for _ in 0..180 {
        at += Vec3::Z * (pace * STEP);
        let driven = chassis.swim(rig, at);
        assert_eq!(
            driven.source,
            Source::Swim,
            "a body under the surface must be swimming"
        );
        let posed = driven.pose.forward(rig);
        planted |= feet
            .iter()
            .any(|&joint| posed.positions[joint].y.abs() < 1e-4);
        let along = posed.positions[head] - posed.positions[pelvis];
        pitch = along
            .normalize_or(Vec3::Y)
            .dot(Vec3::Y)
            .clamp(-1.0, 1.0)
            .acos();
    }
    (planted, pitch.to_degrees())
}

#[test]
fn a_swimming_body_treads_at_rest_and_lies_down_to_travel() {
    // A body crossing a pond must not walk through it, striding at whatever its
    // horizontal speed implies. The swim is one axis from a tread to a crawl,
    // so the claim worth guarding is that BOTH ends arrive: a body holding
    // station hangs upright and sculls, and a travelling one lies along the
    // water. A driver that pinned the effort would get one of the two and pass
    // any test that only looked at the other.
    let avatar = body_of("did:plc:swim-test");
    let rig = avatar.rig.clone();
    let (planted, upright) = swam(&rig, 0.0);
    assert!(!planted, "a treading body planted a foot on the bottom");
    let (_, prone) = swam(&rig, 1.4);
    assert!(
        upright < 25.0,
        "a body treading water hung {upright:.0} deg off vertical — a tread is upright"
    );
    assert!(
        prone > 60.0,
        "a body swimming at 1.4 m/s lay {prone:.0} deg off vertical — a crawl lies along the water"
    );
}

#[test]
fn a_standing_body_breathes_instead_of_freezing() {
    // **The statue regression.** The failure mode of forgetting to wire the
    // idle is silent: a rest pose is a perfectly valid pose, written every
    // frame, forever. So the guard is change over time, read off the drawn
    // pose — a breathing body's joints move between frames a second apart and a
    // statue's do not. Blinking cannot satisfy it: the lids are four joints
    // that move nothing else.
    let avatar = body_of("did:plc:idle-test");
    let rig = avatar.rig.clone();
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);

    // The body never moves. Sampled once a second for four seconds, because a
    // breath is slow: adjacent frames of a breathing body are nearly identical
    // too.
    let mut samples: Vec<Pose> = Vec::new();
    for frame in 0..240 {
        let driven = chassis.step(&rig, Vec3::ZERO);
        if frame % 60 == 0 {
            samples.push(driven.pose.clone());
        }
    }
    assert_eq!(
        chassis.driver.source(),
        Source::Idle,
        "a standing body's source must be the idle"
    );
    let apart = |a: symbios_avatar::Quat, b: symbios_avatar::Quat| 1.0 - a.dot(b).abs();
    let moved = samples
        .windows(2)
        .map(|pair| {
            (0..pair[0].rotations.len())
                .map(|joint| apart(pair[0].rotations[joint], pair[1].rotations[joint]))
                .fold(0.0f32, f32::max)
        })
        .fold(0.0f32, f32::max);
    assert!(
        moved > 1e-6,
        "four seconds of standing drew a bit-identical skeleton — the idle is not running and \
         every standing body is a statue"
    );
}

#[test]
fn a_flood_of_requests_gestures_once() {
    // The rate limit, and the reason it is measured from the START of a
    // gesture: a sender pasting the same word four times must wave once and
    // then stand there. Playback progress is the one thing a restart cannot
    // fake — a stamp read back is identical whether the cooldown ran or not.
    let avatar = body_of("did:plc:emote-test");
    let rig = avatar.rig.clone();
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);

    chassis.ask(&rig, Vec3::ZERO, "Greeting");
    let (_, first) = chassis
        .driver
        .gesture()
        .expect("the first request gestures");
    assert!(
        (first - STEP).abs() < 1e-6,
        "a gesture started this frame should have advanced exactly one frame, not {first}"
    );
    for _ in 0..20 {
        chassis.ask(&rig, Vec3::ZERO, "Greeting");
    }
    let (name, elapsed) = chassis
        .driver
        .gesture()
        .expect("the gesture is still running");
    assert_eq!(name, "Greeting");
    assert!(
        (elapsed - STEP * 21.0).abs() < 1e-4,
        "a request inside the cooldown restarted the gesture: {elapsed} s in, against {} expected",
        STEP * 21.0
    );

    // A name the roster does not carry starts nothing and spends nothing.
    let mut fresh = Chassis::seeded(INSTRUMENT_SEED);
    fresh.ask(&rig, Vec3::ZERO, "Pirouette");
    assert!(
        fresh.driver.gesture().is_none(),
        "an unknown gesture name started a source change with no clip under it"
    );
    fresh.ask(&rig, Vec3::ZERO, "Greeting");
    assert!(
        fresh.driver.gesture().is_some(),
        "an unknown name spent the cooldown that a real gesture then needed"
    );
}

#[test]
fn a_gesture_leaves_the_legs_to_the_locomotion_layer() {
    // **The load-bearing claim of the gesture layer**: a gesture plays over a
    // walk rather than replacing it, so the joints that carry the body come out
    // of the application untouched while the upper body takes it. Without it a
    // wave while walking stands the body still and slides its feet along the
    // ground.
    //
    // It is a property of the goal-space format — a clip writes only the parts
    // its tracks address — and THIS is the test that keeps it one: the roster
    // is free to grow, and a gesture that grows a leg or a root track fails
    // here before a walking body ever slides a foot.
    let avatar = body_of("did:plc:emote-test");
    let rig = &avatar.rig;

    // A walk pose, so the legs hold something a gesture could destroy.
    let mut walking = Pose::rest(rig);
    let speed = Speed::new(rig, 1.4);
    let gait = speed.gait(rig);
    let stride = speed.stride(rig);
    gait::step(rig, &mut walking, &gait, &stride, 0.25, |_| None);
    gait::swing_arms(rig, &mut walking, &gait, &stride, 0.25);

    // Compared by DOT rather than by an angle, which is the arccosine of the
    // dot and loses all its precision exactly where this test looks: two
    // bit-identical rotations read as a third of a milliradian apart.
    let apart = |a: symbios_avatar::Quat, b: symbios_avatar::Quat| 1.0 - a.dot(b).abs();
    // Judged against the rig's own zones — the same question the arm swing asks
    // to decide which limbs are legs, so a body plan nobody has written yet
    // answers it correctly too.
    let carried: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .flat_map(|limb| {
            [
                Zone::UpperLimb(limb),
                Zone::LowerLimb(limb),
                Zone::Extremity(limb),
            ]
        })
        .flat_map(|zone| rig.in_zone(zone))
        .collect();
    assert!(
        carried.len() > 3,
        "a biped should carry itself on more than {} joints",
        carried.len()
    );
    let planted = walking.forward(rig);

    // The gestures that ride OVER locomotion. The roster's sit and sleep are
    // deliberately not here: those are whole-body postures that replace what
    // carries the body rather than laying over it, so a leg they move is the
    // point of them rather than a defect.
    for name in ["Greeting", "Reject", "Head Nod", "Bow"] {
        let clip = gesture::by_name(name).expect("the roster carries it");
        for through in [0.25, 0.5, 0.9] {
            let mut posed = walking.clone();
            clip.apply(rig, &mut posed, through);
            assert_eq!(
                posed.translation, walking.translation,
                "{name} moved the root at {through} — a root track has no business in a gesture"
            );
            let gestured = posed.forward(rig);
            // A bow is the one gesture whose vocabulary is the whole body line:
            // it pitches the pelvis, which swings the hip sockets on an arc, so
            // its leg chain legitimately translates a little at the clip level.
            // The planted feet are the DRIVER's promise, guarded through the
            // full drive elsewhere in this file. What the clip level still owes
            // is a ceiling — a raw leg track without the distribution's
            // compensation swings a foot by hundreds of millimetres.
            let allowance = if name == "Bow" { 5e-2 } else { 2e-3 };
            for &joint in &carried {
                let moved = gestured.positions[joint].distance(planted.positions[joint]);
                assert!(
                    moved < allowance,
                    "joint {joint} ({:?}) carries the body and {name} moved it {:.1} mm at \
                     {through} — a leg belongs to the locomotion layer",
                    rig.joints[joint].zone,
                    moved * 1000.0
                );
            }
            if through == 0.5 {
                let upper_moved = (0..posed.rotations.len())
                    .filter(|&joint| !carried.contains(&joint))
                    .filter(|&joint| apart(posed.rotations[joint], walking.rotations[joint]) > 1e-6)
                    .count();
                assert!(
                    upper_moved > 0,
                    "{name} changed nothing at all mid-play — the gesture is not applying"
                );
            }
        }
    }
}

#[test]
fn a_body_walking_on_the_spot_is_carried_by_its_own_gait() {
    // The instrument's case, and the reason the foothold ledger is not
    // unconditional: a body that walks without travelling has no world to hold
    // its feet against, so a ledger would pin a treadmill's feet to the floor
    // and tear the walk apart. Under its own carriage the same body strides
    // normally with its place never changing.
    let avatar = body_of("did:plc:ankle-test");
    let rig = avatar.rig.clone();
    let mut chassis = Chassis::carrying_itself(INSTRUMENT_SEED);
    let feet = feet(&rig);

    let mut widest = 0.0f32;
    for _ in 0..240 {
        // The body is told it is travelling and never told it has moved, which
        // is exactly what a viewer does.
        let driven = chassis
            .driver
            .drive(
                &rig,
                &Inputs {
                    delta: STEP,
                    velocity: Vec3::Z * 1.4,
                    at: Vec3::ZERO,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("posed");
        if driven.source != Source::Gait {
            continue;
        }
        let posed = driven.pose.forward(&rig);
        widest = widest.max((posed.positions[feet[0]].z - posed.positions[feet[1]].z).abs());
    }
    assert!(
        widest > 0.2,
        "a body walking on the spot split its feet only {:.0} mm — the gait is not running",
        widest * 1000.0
    );
}
