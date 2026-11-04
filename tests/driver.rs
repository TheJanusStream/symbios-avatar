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
use symbios_avatar::anim::heading::Heading;
use symbios_avatar::anim::{Lead, Pattern};
use symbios_avatar::anim::{emote, gesture};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Leap, Limb, Pose, QuadrupedParams, Rig, Role,
    Speed, Vec3, Zone, plan,
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
    rig.ground_contacts()
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

/// The default four-legged body, built as [`body_of`] builds a person's: a
/// dog, whose paws gallop round itself.
fn beast() -> Avatar {
    let record = AvatarRecord::new("Runner", Archetype::Quadruped(QuadrupedParams::default()));
    Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("the default quadruped builds")
}

/// What a four-legged body's driver moved it in, driven one Froude number a
/// frame and turning at `turn` radians a second: the speed it built the gait
/// from on each frame it travelled.
fn gaited(rig: &Rig, froudes: &[f32], turn: f32) -> Vec<Speed> {
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let mut at = Vec3::ZERO;
    froudes
        .iter()
        .filter_map(|&froude| {
            let pace = Speed::from_froude(froude).metres_per_second(rig);
            at += Vec3::Z * (pace * STEP);
            driver
                .drive(
                    rig,
                    &Inputs {
                        delta: STEP,
                        velocity: Vec3::Z * pace,
                        at,
                        turn,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed every frame");
            driver.speed()
        })
        .collect()
}

/// Every change of pattern in `driven`: from, to, and the Froude number the
/// driver changed at.
fn changes(driven: &[Speed]) -> Vec<(Pattern, Pattern, f32)> {
    driven
        .windows(2)
        .filter_map(|pair| {
            let (before, now) = (pair[0].pattern()?, pair[1].pattern()?);
            (before != now).then_some((before, now, pair[1].froude()))
        })
        .collect()
}

/// The Froude numbers a body standing on `rig` changes up at, read off the
/// speed axis rather than restated: where [`Speed::new`] first picks each
/// pattern.
fn changes_up(rig: &Rig) -> Vec<f32> {
    let pattern = |froude: f32| {
        Speed::new(rig, Speed::from_froude(froude).metres_per_second(rig))
            .pattern()
            .expect("a four-legged body moves in a pattern")
    };
    let froudes: Vec<f32> = (0..=1_000).map(|step| step as f32 * 0.01).collect();
    froudes
        .windows(2)
        .filter(|pair| pattern(pair[0]) != pattern(pair[1]))
        .map(|pair| pair[1])
        .collect()
}

#[test]
fn a_four_legged_body_changes_gait_once_each_way_at_every_threshold() {
    // #370. Speeding from a stroll to a flat-out gallop and back, a dog walks,
    // trots, canters and gallops and then does the same in reverse - each
    // change once, and each change down at a lower speed than the change up,
    // so a body at the edge of a gait does not flicker across it.
    let beast = beast();
    let rig = &beast.rig;
    let frames = (8.0 / STEP) as usize;
    let up: Vec<f32> = (0..frames)
        .map(|frame| 0.2 + 6.8 * frame as f32 / frames as f32)
        .collect();
    // And a moment at the stroll it set out at, for the eased pace to get
    // there.
    let there_and_back: Vec<f32> = up
        .iter()
        .chain(up.iter().rev())
        .chain(&[0.2; 120])
        .copied()
        .collect();
    let changes = changes(&gaited(rig, &there_and_back, 0.0));
    let walk = Pattern::Walk;
    let trot = Pattern::Trot;
    let canter = Pattern::Canter(Lead::Right);
    let gallop = Pattern::RotaryGallop(Lead::Right);
    assert_eq!(
        changes
            .iter()
            .map(|&(from, to, _)| (from, to))
            .collect::<Vec<_>>(),
        [
            (walk, trot),
            (trot, canter),
            (canter, gallop),
            (gallop, canter),
            (canter, trot),
            (trot, walk),
        ],
        "a straight run there and back, on paws"
    );
    for (up, down) in changes[..3].iter().zip(changes[3..].iter().rev()) {
        assert!(
            down.2 < up.2 * 0.9,
            "{} changed up to {} at a Froude number of {:.2} and back down at {:.2}: a body \
             that changes back where it changed is one that flickers between them",
            up.0.name(),
            up.1.name(),
            up.2,
            down.2,
        );
    }
}

#[test]
fn a_four_legged_body_at_the_edge_of_a_gait_does_not_flicker() {
    // Hovering about each change - a trot that keeps nearly dropping back to
    // a walk - the body changes once and stays changed. The speed swings a
    // tenth either side of the change, inside the band a body changes down
    // across.
    let beast = beast();
    let rig = &beast.rig;
    let thresholds = changes_up(rig);
    assert_eq!(
        thresholds.len(),
        3,
        "a walk, a trot, a canter and a gallop: {thresholds:?}"
    );
    for threshold in thresholds {
        let frames = (8.0 / STEP) as usize;
        let froudes: Vec<f32> = (0..frames)
            .map(|frame| {
                let t = frame as f32 * STEP;
                // From well inside the slower gait up to the change, then
                // round it.
                let swing = if t < 2.0 {
                    -0.15 * (1.0 - t / 2.0) - 0.1
                } else {
                    -0.1 * (std::f32::consts::TAU * (t - 2.0) / 2.0).cos()
                };
                threshold * (1.0 + swing)
            })
            .collect();
        let changes = changes(&gaited(rig, &froudes, 0.0));
        assert_eq!(
            changes.len(),
            1,
            "hovering about a Froude number of {threshold:.2}, the body changed {changes:?}"
        );
    }
}

#[test]
fn a_body_breaking_into_a_canter_on_a_curve_leads_with_its_inside_legs() {
    // And the mirror: a left turn takes the left lead, a right turn the
    // right, whichever the body would have taken going straight.
    let beast = beast();
    let rig = &beast.rig;
    let frames = (4.0 / STEP) as usize;
    let froudes: Vec<f32> = (0..frames)
        .map(|frame| 1.0 + 2.5 * frame as f32 / frames as f32)
        .collect();
    for (turn, lead) in [(0.6, Lead::Left), (-0.6, Lead::Right)] {
        let driven = gaited(rig, &froudes, turn);
        let cantering = driven
            .last()
            .and_then(|speed| speed.pattern())
            .expect("a quadruped moves in a pattern");
        assert_eq!(cantering, Pattern::Canter(lead), "turning at {turn} rad/s");
    }
}

#[test]
fn a_pattern_asked_for_by_name_is_the_one_a_beast_moves_in() {
    // #370. The viewer's picker: a transverse gallop asked of a dog at a
    // canter's speed - a pattern its own choice would never make - is the
    // pattern it moves in, its cycle still the speed's so its feet keep to
    // the ground; and a person asked for one walks as a person.
    let drive = |rig: &Rig, pattern: Pattern| {
        let mut driver = Driver::seeded(INSTRUMENT_SEED);
        let pace = Speed::from_froude(3.2).metres_per_second(rig);
        let mut at = Vec3::ZERO;
        for _ in 0..120 {
            at += Vec3::Z * (pace * STEP);
            driver
                .drive(
                    rig,
                    &Inputs {
                        delta: STEP,
                        velocity: Vec3::Z * pace,
                        at,
                        pattern: Some(pattern),
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("posed");
        }
        driver.speed().and_then(|speed| speed.pattern())
    };
    let beast = beast();
    let asked = Pattern::Gallop(Lead::Right);
    assert_eq!(drive(&beast.rig, asked), Some(asked));
    let person = body_of("did:plc:transition-test");
    assert_eq!(
        drive(&person.rig, asked),
        None,
        "a person has no pattern to be asked"
    );
}

#[test]
fn a_person_asked_for_a_beasts_emote_is_driven_as_if_asked_for_nothing() {
    // #370. A beast's emotes are a beast's: asked of a person through the
    // driver, each is a name the person's roster does not carry, ignored as
    // any such name is - the person's pose to the bit, no gesture started and
    // no cooldown spent. Looked up by name alone, a sit asked of a person
    // folded their legs under them and laid their feet flat.
    let drive = |rig: &Rig, name: Option<&str>| {
        let mut driver = Driver::seeded(INSTRUMENT_SEED);
        let mut gestured = false;
        let poses: Vec<Pose> = (0..90)
            .map(|frame| {
                let driven = driver
                    .drive(
                        rig,
                        &Inputs {
                            delta: STEP,
                            gesture: name.filter(|_| frame == 10),
                            ..Inputs::default()
                        },
                        level_ground,
                    )
                    .expect("posed");
                gestured |= driver.gesture().is_some();
                driven.pose
            })
            .collect();
        (poses, gestured)
    };
    let person = body_of("did:plc:transition-test");
    let (standing, _) = drive(&person.rig, None);
    let beast = beast();
    for &name in emote::ROSTER {
        let (asked, gestured) = drive(&person.rig, Some(name));
        assert!(!gestured, "a person started {name}");
        assert!(asked == standing, "{name} moved a person");
        // Every emote but the one that gets a beast up out of a state, which
        // a standing beast has nothing to get up from (#377).
        assert_eq!(
            drive(&beast.rig, Some(name)).1,
            name != emote::GET_UP,
            "a beast took {name} wrongly"
        );
    }
}

#[test]
fn a_beasts_ears_bob_as_it_trots_and_settle_when_it_stands() {
    // #370. Each ear is a spring at its tip, swung where the body is in the
    // world: read in the head's own frame, a still ear sits where the pose
    // put it, a trotting one bobs off that and back, and one that has
    // stopped settles home.
    let beast = beast();
    let rig = &beast.rig;
    let tips: Vec<usize> = (0..rig.len())
        .filter(|&joint| rig.joints[joint].role == Role::Spring)
        .collect();
    assert_eq!(tips.len(), 2, "two ears, each a spring at its tip");
    let head = |tip: usize| {
        let base = rig.joints[tip].parent.expect("an ear's base");
        rig.joints[base].parent.expect("the head it hangs off")
    };
    // Where each tip sits in its head's frame, against where it sits at rest.
    let rest = Pose::rest(rig).forward(rig);
    let in_head = |posed: &symbios_avatar::anim::pose::Posed, tip: usize| {
        posed.rotations[head(tip)].inverse() * (posed.positions[tip] - posed.positions[head(tip)])
    };
    let off = |pose: &Pose| {
        let posed = pose.forward(rig);
        tips.iter()
            .map(|&tip| in_head(&posed, tip).distance(in_head(&rest, tip)))
            .fold(0.0f32, f32::max)
    };
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let pace = Speed::from_froude(1.4).metres_per_second(rig);
    let mut at = Vec3::ZERO;
    let trotting: Vec<f32> = (0..120)
        .map(|_| {
            at += Vec3::Z * (pace * STEP);
            off(&chassis.step(rig, at).pose)
        })
        .collect();
    let standing: Vec<f32> = (0..240).map(|_| off(&chassis.step(rig, at).pose)).collect();
    let bobbed = trotting[60..].iter().copied().fold(0.0f32, f32::max);
    let settled = standing[180..].iter().copied().fold(0.0f32, f32::max);
    assert!(
        bobbed > 0.002,
        "a trotting ear bobbed {:.1} mm off its pose",
        bobbed * 1000.0
    );
    assert!(
        settled < bobbed * 0.2,
        "standing three seconds, an ear sat {:.1} mm off its pose, against {:.1} mm trotting",
        settled * 1000.0,
        bobbed * 1000.0
    );
}

#[test]
fn a_stopped_beast_steps_its_feet_home_one_at_a_time_and_stands_level() {
    // #370. A person's idle on four legs rolled a standing dog's loin through
    // thirty degrees - its spine bend is sized by the trunk's height, which on
    // four legs is a few centimetres - stepped three feet at once, and its
    // plant then dropped the hips 11 cm in a frame to reach the fores. A beast
    // that stops steps each foot home in turn, its back level, and stands.
    let beast = beast();
    let rig = &beast.rig;
    let hips = *rig.in_zone(Zone::Pelvis).first().expect("hips");
    let loin = *rig.in_zone(Zone::Abdomen).first().expect("a loin");
    let pads: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .map(|limb| rig.in_zone(Zone::Extremity(limb))[0])
        .collect();
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let pace = Speed::from_froude(1.4).metres_per_second(rig);
    let mut at = Vec3::ZERO;
    for _ in 0..120 {
        at += Vec3::Z * (pace * STEP);
        chassis.step(rig, at);
    }
    let mut last: Option<f32> = None;
    let (mut jolt, mut roll, mut most_up) = (0.0f32, 0.0f32, 0usize);
    let mut home = None;
    for frame in 0..240 {
        let driven = chassis.step(rig, at);
        let posed = driven.pose.forward(rig);
        let height = posed.positions[hips].y;
        jolt = jolt.max(last.map_or(0.0, |last| (height - last).abs()));
        last = Some(height);
        roll = roll.max(driven.pose.rotations[loin].to_axis_angle().1.to_degrees());
        // Once the blend out of the trot has landed the feet it was carrying.
        if frame >= 12 {
            most_up = most_up.max(
                pads.iter()
                    .filter(|&&pad| posed.positions[pad].y - rig.joints[pad].position.y > 0.005)
                    .count(),
            );
        }
        // Where each foot rests in the chassis's frame, which is the pose's:
        // not against the pelvis, which sways over the bearing side by more
        // than this test's centimetre with every foot home.
        let away = pads
            .iter()
            .map(|&pad| {
                let off = posed.positions[pad] - rig.joints[pad].position;
                Vec3::new(off.x, 0.0, off.z).length()
            })
            .fold(0.0f32, f32::max);
        if home.is_none() && frame > 2 && away < 0.01 {
            home = Some(frame);
        }
    }
    // **A share of the leg, re-derived from 6 mm** (#374): the bound was set
    // on a leg of 0.284 m (2.1% of it), measured from a hip joint that stood
    // at the trunk's underside. The hip joint now stands inside the croup,
    // where an animal's does, and the leg it reaches with is 0.466 m; the
    // blend out of the trot, five frames as it was, lifts the hips out of a
    // crouch that is the same share of that leg (P5's body 3.7 mm in a frame,
    // 1.3% of its leg; this body 6.6 mm, 1.4%).
    let leg = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    assert!(
        jolt < 0.021 * leg,
        "the hips moved {:.1} mm in one frame, {:.1}% of the leg",
        jolt * 1000.0,
        jolt / leg * 100.0
    );
    assert!(roll < 2.0, "the loin turned {roll:.1} degrees standing");
    assert!(most_up <= 1, "{most_up} feet were off the ground at once");
    assert!(
        home.is_some_and(|frame| frame < 120),
        "the feet were home by frame {home:?}, not within two seconds"
    );
}

#[test]
fn a_swimming_beast_paddles_at_its_legs_pace() {
    // #370. A crawl is timed by the whole body's length; a paddle by the
    // leg, so a dog paddles about twice a second under way, and treads at
    // its leg's own pendulum.
    let beast = beast();
    let rig = &beast.rig;
    let leg = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    for (pace, wanted) in [
        (0.0, (9.81 / leg).sqrt() / std::f32::consts::TAU),
        (0.8, 0.8 / (leg * 1.5)),
    ] {
        let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
        let mut at = Vec3::ZERO;
        let mut turns = 0.0f32;
        let mut last: Option<f32> = None;
        for _ in 0..120 {
            at += Vec3::Z * (pace * STEP);
            chassis.swim(rig, at);
            let cycle = chassis.driver.cycle();
            if let Some(last) = last {
                turns += (cycle - last).rem_euclid(1.0);
            }
            last = Some(cycle);
        }
        let rate = turns / (119.0 * STEP);
        assert!(
            (rate - wanted).abs() < wanted * 0.15,
            "at {pace} m/s the paddle ran at {rate:.2} Hz, not {wanted:.2}"
        );
    }
}

/// How far a body's feet move in the world through the middle of their
/// stances, at `pace` m/s - swinging `swing` of that either way every two
/// seconds - turning at `turn` rad/s and travelling `heading` off the way it
/// faces, after two seconds to settle: the worst, in metres.
///
/// **The middle, from a tenth of the way in to six tenths**, because that is
/// where nothing turns a foot: before it a landing settles, after it the heel
/// lifts about a toe that stays put. A foot that moves in between is sliding.
fn midstance_slide(rig: &Rig, pace: f32, swing: f32, turn: f32, heading: Option<Heading>) -> f32 {
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let feet: Vec<(Limb, usize)> = rig
        .ground_contacts()
        .into_iter()
        .map(|limb| (limb, rig.in_zone(Zone::Extremity(limb))[0]))
        .collect();
    let (mut at, mut facing) = (Vec3::ZERO, 0.0f32);
    let mut planted: Vec<Option<Vec3>> = vec![None; feet.len()];
    let mut worst = 0.0f32;
    for frame in 0..300 {
        let travel = heading.map_or(0.0, Heading::angle) + facing;
        let paced = pace * (1.0 + swing * (std::f32::consts::TAU * frame as f32 / 120.0).sin());
        let velocity = Vec3::new(travel.sin(), 0.0, travel.cos()) * paced;
        at += velocity * STEP;
        facing += turn * STEP;
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta: STEP,
                    velocity,
                    at,
                    facing,
                    heading,
                    turn,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("an unheld body is posed every frame");
        let posed = driven.pose.forward(rig);
        let gait = driver.speed().map(|speed| speed.gait(rig));
        let turned = symbios_avatar::Quat::from_rotation_y(facing);
        for (index, &(limb, foot)) in feet.iter().enumerate() {
            let middle = driven.source == Source::Gait
                && gait.as_ref().is_some_and(|gait| {
                    gait.limbs
                        .iter()
                        .position(|&of| of == limb)
                        .is_some_and(|at| {
                            matches!(
                                gait.phase(at, driver.cycle()),
                                gait::Phase::Stance(t) if (0.1..0.6).contains(&t)
                            )
                        })
                });
            let here = at + turned * posed.positions[foot];
            planted[index] = match (middle, planted[index]) {
                (true, Some(was)) => {
                    if frame >= 120 {
                        worst = worst.max(here.distance(was));
                    }
                    Some(was)
                }
                (true, None) => Some(here),
                (false, _) => None,
            };
        }
    }
    worst
}

#[test]
fn a_beast_keeps_its_paws_still_through_every_stance() {
    // #370. Walking, trotting, cantering, galloping, changing speed,
    // turning, strafing and backing on four legs, a paw on the ground does
    // not move while it bears the body.
    let beast = beast();
    let rig = &beast.rig;
    let walk = Speed::from_froude(0.3).metres_per_second(rig);
    let trot = Speed::from_froude(1.4).metres_per_second(rig);
    let canter = Speed::from_froude(3.2).metres_per_second(rig);
    let gallop = Speed::from_froude(7.0).metres_per_second(rig);
    let sideways = Some(Heading::new(std::f32::consts::FRAC_PI_2));
    let backward = Some(Heading::new(std::f32::consts::PI));
    for (name, pace, swing, turn, heading) in [
        ("walking", walk, 0.0, 0.0, None),
        ("trotting", trot, 0.0, 0.0, None),
        // Where the trunk rocks and folds over the feet (#370).
        ("cantering", canter, 0.0, 0.0, None),
        ("galloping", gallop, 0.0, 0.0, None),
        // Where the foothold ledger earns its keep: a stance held still in
        // the world while the speed under it changes.
        ("speeding up and slowing", trot, 0.4, 0.0, None),
        ("walking round", walk, 0.0, 0.8, None),
        ("trotting round", trot, 0.0, 0.8, None),
        ("strafing", walk, 0.0, 0.0, sideways),
        ("backing", walk, 0.0, 0.0, backward),
    ] {
        let slid = midstance_slide(rig, pace, swing, turn, heading);
        assert!(
            slid < 0.006,
            "{name}, a paw moved {:.1} mm while it bore the body",
            slid * 1000.0
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
    // application reads 79.94 mm from this instrument on this body and this
    // seed, in every process context. A reading a millimetre or two away is an
    // arithmetic site that has not been routed through the crate's own
    // deterministic maths; anything from the high 50s to the high 80s is the
    // knife edge below, or a body standing on an unpinned idle seed; the
    // regime this guard exists to catch reads hundreds.
    //
    // **And the worst phase is a knife edge** (#363). It is the stop after 127
    // frames, where the hand-over to the idle is chaotic in the moment it
    // lands: on 0.12.0 it read 55.69 mm, under the 57.85 of the stop after
    // 103 frames that was then the worst, and walking up to six parts in a
    // million faster or slower read anything from 56 to 87 mm there. Since
    // #363 the walk's clock carries the instrument's velocity, differenced
    // from positions in floating point, a few millionths of a cycle away by
    // the stop, and that phase reads 79.94 where the other thirty move by
    // about half a millimetre at most. A change that moves the figure inside
    // that band has moved the sample, not the stop — so the median of the
    // thirty-one stops is held as well, which a regression lifting the
    // ordinary phases moves and a knife edge cannot: 19.07 mm.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let mut skids: Vec<f32> = (100..=130)
        .map(|frames| skid_through_a_stop(&rig, frames))
        .collect();
    let worst = skids.iter().copied().fold(0.0f32, f32::max);
    skids.sort_by(f32::total_cmp);
    let median = skids[skids.len() / 2];
    // Printed as well as judged, so every run records its figure rather than
    // only its verdict.
    println!(
        "worst stop skid: {:.2} mm, median {:.2} mm",
        worst * 1000.0,
        median * 1000.0
    );
    assert!(
        worst < 0.082,
        "a foot standing on the ground slid {:.1} mm through a stop, against 79.94 mm downstream \
         on the same body and seed",
        worst * 1000.0
    );
    assert!(
        median < 0.03,
        "the median stop slid a standing foot {:.1} mm, against 19.07 mm on the same body and seed",
        median * 1000.0
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
fn a_beast_leaps_tucks_and_lands_on_all_fours() {
    // #370. The same airborne state machine as a person's, on four legs: no
    // marching and no planted paw in the air, the paws drawn up under it, and
    // a landing every leg absorbs without a paw going through the floor.
    let beast = beast();
    let jump = jumped(&beast.rig, 3.0, 0.0);
    assert!(
        !jump.marched,
        "the gait advanced while the beast was in the air"
    );
    assert!(
        !jump.planted,
        "a paw was planted while the beast was in the air"
    );
    assert!(
        jump.highest > 0.03,
        "the paws never drew up: the lowest reached {:.0} mm",
        jump.highest * 1000.0
    );
    assert!(jump.landed, "the leap never reached its landing");
    assert!(
        jump.sank > 0.02 && jump.sank < 0.3,
        "the landing sank the body {:.0} mm",
        jump.sank * 1000.0
    );
    assert!(
        jump.buried < 0.01,
        "a paw went {:.0} mm through the floor",
        jump.buried * 1000.0
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

// ---------------------------------------------------------------------------
// The consuming application's chassis, turning (#337, overlands #1323)
// ---------------------------------------------------------------------------

/// The consuming application's fixed step, which its chassis controller runs
/// at and which its harness drives the body at.
const CONTROLLER_STEP: f32 = 1.0 / 64.0;

/// The consuming application's humanoid chassis, as its controller moves it.
///
/// **A replica, and that is the one thing this file otherwise refuses to do.**
/// Every other instrument here drives the real thing; this crate cannot run the
/// application's controller, and the question it answers — what a planted foot
/// does when the body turns and accelerates under it the way a player's does —
/// cannot be asked without one. So its arithmetic is copied, with the
/// application's defaults (overlands `apply_humanoid_walk` as shipped by
/// overlands #1323, 2026-09-11): the planar velocity is ASSIGNED each step,
/// an exponential approach toward the keys' direction at `acceleration` while
/// any is held and a decay at `stop_damping` when none is, and that step is
/// then held to the application's ramp ([`ramped`]); the facing turns toward
/// the velocity's own direction at `turn_rate` whenever the body moves faster
/// than 0.1 m/s; and the position integrates the velocity, which is all the
/// application's physics does to it on flat open ground. The walk is the
/// application's own derivation, [`WALK_FROUDE`] on the speed axis; the run is
/// its default travel speed.
///
/// Until overlands #1323 this replica carried the 0.7.0-era chassis (walk
/// Froude 0.43, run 4.0 m/s, no ramp), and was validated against the
/// application's harness within 0.2 mm on it; it moved with the application
/// and was re-validated against the same harness.
///
/// Its whole claim to be the application's chassis is that it reproduces the
/// application's readings on the same body, seed and script — see
/// `probe_what_a_turn_does_to_the_stance`.
struct Controller {
    velocity: Vec3,
    facing: f32,
    at: Vec3,
}

/// The consuming application's velocity approach and decay rates, per second,
/// and its facing rate — its humanoid record's defaults.
const ACCELERATION: f32 = 12.0;
const STOP_DAMPING: f32 = 20.0;
const TURN_RATE: f32 = 12.0;
/// The Froude number the consuming application walks at unshifted.
const WALK_FROUDE: f32 = 0.49;
/// The consuming application's default travel speed, which its run key asks
/// for, in m/s.
const RUN: f32 = 5.0;
/// The consuming application's ramp: the most its planar velocity may speed
/// up and slow down, in m/s² (overlands `SPEED_UP_LIMIT`, `SLOW_DOWN_LIMIT`).
const SPEED_UP_LIMIT: f32 = 9.0;
const SLOW_DOWN_LIMIT: f32 = 13.5;

/// One step of the planar velocity held to the application's ramp: the step
/// the controller proposed, with its part ALONG the current velocity capped at
/// [`SPEED_UP_LIMIT`] and [`SLOW_DOWN_LIMIT`] and its part across it — the
/// turn — left as asked. Under 0.1 m/s the along axis is the keys' own
/// direction. Along the velocity rather than on the speed, as the application
/// does it, because a cap on the magnitude alone flips a reversal at its zero
/// crossing.
fn ramped(current: Vec3, proposed: Vec3, toward: Vec3, dt: f32) -> Vec3 {
    let axis = if current.length_squared() > 0.01 {
        current.normalize()
    } else {
        toward.normalize_or_zero()
    };
    if axis == Vec3::ZERO {
        return proposed;
    }
    let change = proposed - current;
    let along = change.dot(axis);
    let across = change - axis * along;
    current + axis * along.clamp(-SLOW_DOWN_LIMIT * dt, SPEED_UP_LIMIT * dt) + across
}

/// An angle folded into `(-PI, PI]`.
///
/// **The half-open side is the application's.** A reversal asks the facing
/// for an exact half turn, and which way round it goes is decided by a tie:
/// the application's quaternion slerp breaks it toward positive yaw, so a fold
/// into `[-PI, PI)` turns every reversal the other way and reads its splay
/// mirrored.
fn wrapped(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let folded = (angle + PI).rem_euclid(TAU) - PI;
    if folded <= -PI { folded + TAU } else { folded }
}

impl Controller {
    /// Standing at the origin, facing world `-Z` — the application's chassis at
    /// rest, whose forward is Bevy's.
    fn standing() -> Self {
        Self {
            velocity: Vec3::ZERO,
            facing: std::f32::consts::PI,
            at: Vec3::ZERO,
        }
    }

    /// One fixed step with the keys asking for `toward` (world, unnormalised,
    /// zero for no key) at `speed`.
    fn step(&mut self, toward: Vec3, speed: f32) {
        let dt = CONTROLLER_STEP;
        let planar = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
        let proposed = if toward == Vec3::ZERO {
            planar * (-STOP_DAMPING * dt).exp()
        } else {
            planar.lerp(
                toward.normalize() * speed,
                (ACCELERATION * dt).clamp(0.0, 1.0),
            )
        };
        self.velocity = ramped(planar, proposed, toward, dt);
        if self.velocity.length_squared() > 0.01 {
            let target = self.velocity.x.atan2(self.velocity.z);
            let alpha = (TURN_RATE * dt).clamp(0.0, 1.0);
            self.facing = wrapped(self.facing + wrapped(target - self.facing) * alpha);
        }
        self.at += self.velocity * dt;
    }
}

/// A key script: the world direction the keys ask for over consecutive spans,
/// and whether the run key is held through each, after a second of standing.
/// The application's camera orbits freely, so its keys are camera-fixed: W is
/// world `-Z`, S `+Z`, D `+X`.
struct Script<'a> {
    name: &'static str,
    spans: &'a [(f32, Vec3, bool)],
}

const STAND_SECS: f32 = 1.0;
const SCRIPT_SECS: f32 = 4.0;
const W: Vec3 = Vec3::new(0.0, 0.0, -1.0);
const S: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const D: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const W_D: Vec3 = Vec3::new(1.0, 0.0, -1.0);

const SCRIPTS: [Script<'static>; 7] = [
    Script {
        name: "straight W (control)",
        spans: &[(SCRIPT_SECS, W, false)],
    },
    Script {
        name: "W 2 s then W+D (45 deg)",
        spans: &[(2.0, W, false), (2.0, W_D, false)],
    },
    Script {
        name: "W 2 s then D (90 deg)",
        spans: &[(2.0, W, false), (2.0, D, false)],
    },
    Script {
        name: "W 2 s then S (reversal)",
        spans: &[(2.0, W, false), (2.0, S, false)],
    },
    Script {
        name: "D from standing",
        spans: &[(SCRIPT_SECS, D, false)],
    },
    Script {
        name: "W 2 s then Shift (walk to run)",
        spans: &[(2.0, W, false), (2.0, W, true)],
    },
    Script {
        name: "Shift+W 2 s then W (run to walk)",
        spans: &[(2.0, W, true), (2.0, W, false)],
    },
];

impl Script<'_> {
    /// The direction asked for at `t`, and whether the run key is held.
    fn keys(&self, t: f32) -> (Vec3, bool) {
        let mut from = STAND_SECS;
        if t < from {
            return (Vec3::ZERO, false);
        }
        for &(secs, toward, shift) in self.spans {
            if t < from + secs {
                return (toward, shift);
            }
            from += secs;
        }
        (Vec3::ZERO, false)
    }
}

/// One frame of a turning run, read off the drawn pose.
struct Stance {
    t: f32,
    source: Source,
    /// The feet's lateral separation less its rest value, body frame, metres:
    /// the splay.
    splay: f32,
    /// The feet's fore-aft separation, body frame, metres.
    split: f32,
    /// The pelvis less its standing height, metres.
    root: f32,
    strained: bool,
    /// Each ankle in the WORLD, left foot first — where the feet are, for the
    /// question of whether they meet.
    feet: [Vec3; 2],
    /// Every sole point of each foot, in the WORLD, left foot first: the
    /// joint's rest position dropped to the ground plane and carried by the
    /// ankle, which is how the roll models a sole (#1082) — the only point of
    /// a planted foot that is pinned, so the only one a slide can be read off.
    soles: [Vec<Vec3>; 2],
    /// Whether the gait the driver ran this frame has each foot in stance.
    down: [bool; 2],
}

/// What every frame of a replica run is read with: the body's own landmarks,
/// taken once.
struct Reader {
    pelvis: usize,
    standing: f32,
    feet: Vec<usize>,
    rest: f32,
    soles: [(usize, Vec<usize>); 2],
}

impl Reader {
    fn of(rig: &Rig) -> Self {
        let pelvis = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a root joint");
        let feet = feet(rig);
        Self {
            pelvis,
            standing: rig.joints[pelvis].position.y,
            rest: (rig.joints[feet[0]].position.x - rig.joints[feet[1]].position.x).abs(),
            feet,
            soles: [Limb::HindLeft, Limb::HindRight].map(|limb| {
                let joints = rig.extremity_joints(limb);
                (joints[0], joints[1..].to_vec())
            }),
        }
    }

    /// One frame, read off the pose `driven` drew a body at `at` facing
    /// `facing` in.
    fn read(
        &self,
        rig: &Rig,
        driver: &Driver,
        driven: &Driven,
        t: f32,
        at: Vec3,
        facing: f32,
    ) -> Stance {
        let posed = driven.pose.forward(rig);
        let (left, right) = (posed.positions[self.feet[0]], posed.positions[self.feet[1]]);
        let into_world = |body: Vec3| at + symbios_avatar::Quat::from_rotation_y(facing) * body;
        let gait = driver.speed().map(|speed| speed.gait(rig));
        let limbs = [Limb::HindLeft, Limb::HindRight];
        Stance {
            t,
            source: driven.source,
            splay: (left.x - right.x) - self.rest,
            split: left.z - right.z,
            root: posed.positions[self.pelvis].y - self.standing,
            strained: driven.strained,
            feet: [into_world(left), into_world(right)],
            soles: self.soles.clone().map(|(ankle, sole)| {
                sole.iter()
                    .map(|&joint| {
                        let at_rest = rig.joints[joint].position;
                        into_world(
                            posed.positions[ankle]
                                + posed.rotations[ankle]
                                    * (Vec3::new(at_rest.x, 0.0, at_rest.z)
                                        - rig.joints[ankle].position),
                        )
                    })
                    .collect()
            }),
            down: limbs.map(|limb| {
                gait.as_ref().is_some_and(|gait| {
                    gait.limbs
                        .iter()
                        .position(|&of| of == limb)
                        .is_some_and(|index| gait.phase(index, driver.cycle()).is_stance())
                })
            }),
        }
    }
}

/// Drives `script` through the replica chassis and a driver configured by
/// `config`, with the run key held throughout if `run`, reading every frame.
fn turned(rig: &Rig, script: &Script, run: bool, config: DriverConfig) -> Vec<Stance> {
    let walk = Speed::from_froude(WALK_FROUDE).metres_per_second(rig);
    turned_at(rig, script, if run { RUN } else { walk }, config)
}

/// As [`turned`], with the keys asking for `speed` wherever the script does
/// not hold the run key, and [`RUN`] where it does.
fn turned_at(rig: &Rig, script: &Script, speed: f32, config: DriverConfig) -> Vec<Stance> {
    let mut driver = Driver::new(config, INSTRUMENT_SEED);
    let mut chassis = Controller::standing();
    let reader = Reader::of(rig);
    let frames = ((STAND_SECS + SCRIPT_SECS) / CONTROLLER_STEP).round() as usize;
    (0..frames)
        .map(|frame| {
            let t = frame as f32 * CONTROLLER_STEP;
            let (toward, shift) = script.keys(t);
            chassis.step(toward, if shift { RUN } else { speed });
            let driven = driver
                .drive(
                    rig,
                    &Inputs {
                        delta: CONTROLLER_STEP,
                        velocity: chassis.velocity,
                        at: chassis.at,
                        facing: chassis.facing,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            reader.read(rig, &driver, &driven, t, chassis.at, chassis.facing)
        })
        .collect()
}

/// The extremes of a window of frames: splay low and high, the widest split,
/// the lowest pelvis, and the frames strained.
fn extremes(frames: &[&Stance]) -> (f32, f32, f32, f32, usize) {
    frames.iter().fold(
        (f32::MAX, f32::MIN, 0.0f32, f32::MAX, 0usize),
        |(lo, hi, split, root, strained), frame| {
            (
                lo.min(frame.splay),
                hi.max(frame.splay),
                split.max(frame.split.abs()),
                root.min(frame.root),
                strained + usize::from(frame.strained),
            )
        },
    )
}

/// The furthest any sole point slid through the world while it was down, in
/// metres, over a window of frames.
///
/// **Each point against itself, inside its own stance episode** — #277's
/// ruler: a point counts as down while the gait has its foot in stance AND
/// its own height is within 5 mm of the lowest it gets in the window (contact
/// passes heel to toe, and a heel that lifts has moved without sliding), and
/// the episode's anchor is where it first came down. Horizontal only.
fn slide(frames: &[&Stance]) -> f32 {
    const CLEARANCE: f32 = 0.005;
    let mut worst = 0.0f32;
    for foot in 0..2 {
        let points = frames.first().map_or(0, |frame| frame.soles[foot].len());
        for point in 0..points {
            let floor = frames
                .iter()
                .map(|frame| frame.soles[foot][point].y)
                .fold(f32::MAX, f32::min);
            let mut anchor: Option<Vec3> = None;
            for frame in frames {
                let world = frame.soles[foot][point];
                if frame.down[foot] && world.y - floor <= CLEARANCE {
                    let from = *anchor.get_or_insert(world);
                    worst = worst.max(Vec3::new(world.x - from.x, 0.0, world.z - from.z).length());
                } else {
                    anchor = None;
                }
            }
        }
    }
    worst
}

#[test]
#[ignore = "probe for #337: prints the turning tables through the replica chassis"]
fn probe_what_a_turn_does_to_the_stance() {
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let configs = [
        ("driver default", DriverConfig::default()),
        (
            "pace_response 0",
            DriverConfig {
                pace_response: 0.0,
                ..DriverConfig::default()
            },
        ),
        (
            "Carriage::Own",
            DriverConfig {
                carriage: Carriage::Own,
                ..DriverConfig::default()
            },
        ),
    ];
    for (label, config) in configs {
        for script in &SCRIPTS {
            for run in [false, true] {
                let frames = turned(&rig, script, run, config);
                for (window, from, to) in [
                    ("start", STAND_SECS, STAND_SECS + 2.0),
                    ("change", STAND_SECS + 2.0, STAND_SECS + SCRIPT_SECS),
                ] {
                    let within: Vec<&Stance> = frames
                        .iter()
                        .filter(|frame| frame.t >= from && frame.t < to)
                        .collect();
                    let (lo, hi, split, root, strained) = extremes(&within);
                    let idled = within
                        .iter()
                        .filter(|frame| frame.source != Source::Gait)
                        .count();
                    println!(
                        "{label} | {} | {} | {window}: splay {:+.1}..{:+.1} mm; |split| max {:.1} mm; root \
                     {:+.1} mm; slide {:.1} mm; strained {strained}/{}; not-gait {idled}",
                        script.name,
                        if run { "run" } else { "walk" },
                        lo * 1000.0,
                        hi * 1000.0,
                        split * 1000.0,
                        root * 1000.0,
                        slide(&within) * 1000.0,
                        within.len(),
                    );
                }
            }
        }
    }
}

/// The orbit's changes of keys at every moment of the stride: `first` held a
/// little longer each time, over more than a walk's whole cycle, then `second`
/// for two seconds — each with whether the run key is held. The orbit's twin
/// of [`swept`].
fn swept_orbit(
    first: (Vec3, bool),
    second: (Vec3, bool),
) -> impl Iterator<Item = [(f32, Vec3, bool); 2]> {
    (0..16).map(move |step| {
        [
            (0.8 + step as f32 * 0.05, first.0, first.1),
            (2.0, second.0, second.1),
        ]
    })
}

/// The worst of a window of an orbit run, read by [`worst_of`].
#[derive(Clone, Copy)]
struct Worst {
    /// The furthest a planted sole slid, metres ([`slide`]).
    slide: f32,
    /// The widest splay either way, metres.
    splay: f32,
    /// The lowest the pelvis went against its standing height, metres.
    root: f32,
    /// The frames a leg strained in.
    strained: usize,
}

/// The worst of a window, for the orbit's probe and guard.
fn worst_of(frames: &[&Stance]) -> Worst {
    let (lo, hi, _, root, strained) = extremes(frames);
    Worst {
        slide: slide(frames),
        splay: lo.abs().max(hi.abs()),
        root,
        strained,
    }
}

#[test]
#[ignore = "probe for #363: prints what each orbit change costs at every moment of the stride"]
fn probe_what_the_orbit_costs_at_every_moment() {
    // A start from standing begins at cycle 0 whatever the stand, so it is
    // swept by the speed the keys ask for; a change of keys by the moment of
    // the stride it lands at.
    let rig = body_of("did:plc:stop-test").rig;
    // Each reading is what it was swept by - a speed or a lead span - and
    // the worst of its window.
    let summary = |name: &str, readings: &[(f32, Worst)]| {
        let mean =
            readings.iter().map(|(_, worst)| worst.slide).sum::<f32>() / readings.len() as f32;
        let (at, worst) = readings.iter().map(|&(at, worst)| (at, worst.slide)).fold(
            (0.0, 0.0f32),
            |kept, reading| {
                if reading.1 > kept.1 { reading } else { kept }
            },
        );
        let splay = readings
            .iter()
            .map(|(_, worst)| worst.splay)
            .fold(0.0f32, f32::max);
        let root = readings
            .iter()
            .map(|(_, worst)| worst.root)
            .fold(f32::MAX, f32::min);
        let strained: usize = readings.iter().map(|(_, worst)| worst.strained).sum();
        println!(
            "SWEPT {name}: slide mean {:.1} mm, worst {:.1} mm (at {at:.2}); |splay| max {:.1} mm; \
             root min {:+.1} mm; strained {strained}",
            mean * 1000.0,
            worst * 1000.0,
            splay * 1000.0,
            root * 1000.0,
        );
    };
    for (pace, speeds) in [("walk", (1.0f32, 1.95f32)), ("run", (2.0, 6.0))] {
        let readings: Vec<_> = (0..16)
            .map(|step| {
                let speed = speeds.0 + (speeds.1 - speeds.0) * step as f32 / 15.0;
                let script = Script {
                    name: "W from standing",
                    spans: &[(2.0, W, false)],
                };
                let frames = turned_at(&rig, &script, speed, DriverConfig::default());
                let start: Vec<&Stance> = frames
                    .iter()
                    .filter(|frame| frame.t >= STAND_SECS && frame.t < STAND_SECS + 2.0)
                    .collect();
                (speed, worst_of(&start))
            })
            .collect();
        summary(
            &format!(
                "W from standing | {pace} {:.2}..{:.2} m/s",
                speeds.0, speeds.1
            ),
            &readings,
        );
    }
    let walk = Speed::from_froude(WALK_FROUDE).metres_per_second(&rig);
    for (name, first, second) in [
        ("W then W+D (45 deg)", (W, false), (W_D, false)),
        ("W then D (90 deg)", (W, false), (D, false)),
        ("W then S (reversal)", (W, false), (S, false)),
        ("W then Shift (walk to run)", (W, false), (W, true)),
        ("Shift+W then W (run to walk)", (W, true), (W, false)),
    ] {
        for run in [false, true] {
            let readings: Vec<_> = swept_orbit(first, second)
                .map(|spans| {
                    let script = Script {
                        name,
                        spans: &spans,
                    };
                    let frames = turned_at(
                        &rig,
                        &script,
                        if run { RUN } else { walk },
                        DriverConfig::default(),
                    );
                    let change = STAND_SECS + spans[0].0;
                    let after: Vec<&Stance> = frames
                        .iter()
                        .filter(|frame| frame.t >= change && frame.t < change + 2.0)
                        .collect();
                    (spans[0].0, worst_of(&after))
                })
                .collect();
            summary(
                &format!("{name} | {}", if run { "run" } else { "walk" }),
                &readings,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The consuming application's chassis, looking out of its own eyes (#361, #362)
// ---------------------------------------------------------------------------

/// Under this travel the consuming application hands the driver no heading, in
/// m/s (overlands `HEADING_MIN_SPEED`).
const HEADING_MIN_SPEED: f32 = 0.1;

impl Controller {
    /// One fixed step looking out of the body's own eyes (overlands #1746): the
    /// keys ask for `keys` in the BODY's frame — `+Z` forward, `+X` left —
    /// turned onto the world by where the eyes look, `look`; the velocity
    /// approaches that exactly as [`Self::step`]'s does, held to the same ramp;
    /// and the facing turns toward the eyes at [`TURN_RATE`] every step, moving
    /// or not, rather than toward the travel.
    fn step_looking(&mut self, keys: Vec3, speed: f32, look: f32) {
        let dt = CONTROLLER_STEP;
        let toward = symbios_avatar::Quat::from_rotation_y(look) * keys;
        let planar = Vec3::new(self.velocity.x, 0.0, self.velocity.z);
        let proposed = if toward == Vec3::ZERO {
            planar * (-STOP_DAMPING * dt).exp()
        } else {
            planar.lerp(
                toward.normalize() * speed,
                (ACCELERATION * dt).clamp(0.0, 1.0),
            )
        };
        self.velocity = ramped(planar, proposed, toward, dt);
        let alpha = (TURN_RATE * dt).clamp(0.0, 1.0);
        self.facing = wrapped(self.facing + wrapped(look - self.facing) * alpha);
        self.at += self.velocity * dt;
    }
}

/// A first-person script: spans of keys held in the body's own frame, the run
/// key, and how fast the eyes turn, in radians a second positive toward the
/// body's left, after a second of standing.
struct Looking<'a> {
    name: &'static str,
    spans: &'a [(f32, Vec3, bool, f32)],
}

const AHEAD: Vec3 = Vec3::new(0.0, 0.0, 1.0);
const BACK: Vec3 = Vec3::new(0.0, 0.0, -1.0);
const LEFTWARD: Vec3 = Vec3::new(1.0, 0.0, 0.0);
const RIGHTWARD: Vec3 = Vec3::new(-1.0, 0.0, 0.0);
const BACK_LEFT: Vec3 = Vec3::new(1.0, 0.0, -1.0);
const AHEAD_RIGHT: Vec3 = Vec3::new(-1.0, 0.0, 1.0);
const STILL: Vec3 = Vec3::ZERO;

/// How long a first-person run goes on standing still after its script, in
/// seconds: long enough for a turn to hand back to the idle.
const TAIL_SECS: f32 = 1.5;

impl Looking<'_> {
    /// When the script's last span ends.
    fn length(&self) -> f32 {
        STAND_SECS + self.spans.iter().map(|span| span.0).sum::<f32>()
    }

    /// The keys, the run key and the eyes' turn at `t`.
    fn at(&self, t: f32) -> (Vec3, bool, f32) {
        let mut from = STAND_SECS;
        if t < from {
            return (STILL, false, 0.0);
        }
        for &(secs, keys, shift, rate) in self.spans {
            if t < from + secs {
                return (keys, shift, rate);
            }
            from += secs;
        }
        (STILL, false, 0.0)
    }
}

/// Drives `script` looking out of the body's eyes through the replica chassis,
/// a driver configured by `config`, and the inputs the consuming application
/// gives a first-person body: the heading its travel makes against its facing,
/// and the turn its facing makes, read every frame.
fn looked(rig: &Rig, script: &Looking, run: bool, config: DriverConfig) -> Vec<Stance> {
    let mut driver = Driver::new(config, INSTRUMENT_SEED);
    let mut chassis = Controller::standing();
    let mut look = chassis.facing;
    let walk = Speed::from_froude(WALK_FROUDE).metres_per_second(rig);
    let reader = Reader::of(rig);
    let frames = ((script.length() + TAIL_SECS) / CONTROLLER_STEP).round() as usize;
    (0..frames)
        .map(|frame| {
            let t = frame as f32 * CONTROLLER_STEP;
            let (keys, shift, rate) = script.at(t);
            look = wrapped(look + rate * CONTROLLER_STEP);
            let before = chassis.facing;
            chassis.step_looking(keys, if run || shift { RUN } else { walk }, look);
            let planar = Vec3::new(chassis.velocity.x, 0.0, chassis.velocity.z);
            let heading = (planar.length() >= HEADING_MIN_SPEED).then(|| {
                symbios_avatar::Heading::toward(
                    symbios_avatar::Quat::from_rotation_y(-chassis.facing) * planar,
                )
            });
            let driven = driver
                .drive(
                    rig,
                    &Inputs {
                        delta: CONTROLLER_STEP,
                        velocity: chassis.velocity,
                        at: chassis.at,
                        facing: chassis.facing,
                        heading,
                        turn: wrapped(chassis.facing - before) / CONTROLLER_STEP,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            reader.read(rig, &driver, &driven, t, chassis.at, chassis.facing)
        })
        .collect()
}

/// How close the two ankles came to each other across a window, in metres.
fn closest_feet(frames: &[&Stance]) -> f32 {
    frames
        .iter()
        .map(|frame| {
            let gap = frame.feet[0] - frame.feet[1];
            Vec3::new(gap.x, 0.0, gap.z).length()
        })
        .fold(f32::MAX, f32::min)
}

const LOOKS: [Looking<'static>; 17] = [
    Looking {
        name: "W (control)",
        spans: &[(2.0, AHEAD, false, 0.0)],
    },
    Looking {
        name: "A (strafe left)",
        spans: &[(2.0, LEFTWARD, false, 0.0)],
    },
    Looking {
        name: "D (strafe right)",
        spans: &[(2.0, RIGHTWARD, false, 0.0)],
    },
    Looking {
        name: "S (back up)",
        spans: &[(2.0, BACK, false, 0.0)],
    },
    Looking {
        name: "S+A (back and left)",
        spans: &[(2.0, BACK_LEFT, false, 0.0)],
    },
    Looking {
        name: "W+D (ahead and right)",
        spans: &[(2.0, AHEAD_RIGHT, false, 0.0)],
    },
    Looking {
        name: "A then S+A (the backward corner)",
        spans: &[(1.0, LEFTWARD, false, 0.0), (1.5, BACK_LEFT, false, 0.0)],
    },
    Looking {
        name: "look round standing, 90 deg/s",
        spans: &[(2.0, STILL, false, std::f32::consts::FRAC_PI_2)],
    },
    Looking {
        name: "look round standing, 360 deg/s",
        spans: &[(1.0, STILL, false, std::f32::consts::TAU)],
    },
    Looking {
        name: "look round standing, 5 deg/s",
        spans: &[(2.0, STILL, false, 0.087_266_46)],
    },
    Looking {
        name: "W looking round, 60 deg/s",
        spans: &[(2.0, AHEAD, false, std::f32::consts::FRAC_PI_3)],
    },
    Looking {
        name: "W then A (into a strafe)",
        spans: &[(1.0, AHEAD, false, 0.0), (1.5, LEFTWARD, false, 0.0)],
    },
    Looking {
        name: "W then W+D (into a diagonal)",
        spans: &[(1.0, AHEAD, false, 0.0), (1.5, AHEAD_RIGHT, false, 0.0)],
    },
    Looking {
        name: "A then W (out of a strafe)",
        spans: &[(1.0, LEFTWARD, false, 0.0), (1.5, AHEAD, false, 0.0)],
    },
    Looking {
        name: "looking round, then A (setting off)",
        spans: &[
            (1.0, STILL, false, std::f32::consts::FRAC_PI_2),
            (1.5, LEFTWARD, false, std::f32::consts::FRAC_PI_2),
        ],
    },
    Looking {
        name: "looking round, then W (setting off)",
        spans: &[
            (1.0, STILL, false, std::f32::consts::FRAC_PI_2),
            (1.5, AHEAD, false, std::f32::consts::FRAC_PI_2),
        ],
    },
    Looking {
        name: "A looking round, then stopping",
        spans: &[
            (1.5, LEFTWARD, false, std::f32::consts::FRAC_PI_2),
            (1.0, STILL, false, std::f32::consts::FRAC_PI_2),
        ],
    },
];

/// The furthest a planted ankle moved through the world between two frames it
/// was down in, and the furthest any ankle did, in metres, over a window.
fn jumps(frames: &[&Stance]) -> (f32, f32) {
    let (mut planted, mut any) = (0.0f32, 0.0f32);
    for pair in frames.windows(2) {
        for foot in 0..2 {
            let moved = pair[1].feet[foot] - pair[0].feet[foot];
            let moved = Vec3::new(moved.x, 0.0, moved.z).length();
            any = any.max(moved);
            if pair[0].down[foot] && pair[1].down[foot] {
                planted = planted.max(moved);
            }
        }
    }
    (planted, any)
}

#[test]
#[ignore = "probe for #361 and #362: prints what a pivot does setting off, stopping and drifting"]
fn probe_what_a_pivot_does_at_its_edges() {
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let steady = looked(&rig, &LOOKS[7], false, DriverConfig::default());
    let (planted, any) = jumps(&held(&steady, &LOOKS[7]));
    println!(
        "steady pivot, 90 deg/s: per frame a planted ankle {:.1} mm, any {:.1} mm",
        planted * 1000.0,
        any * 1000.0
    );
    for script in [&LOOKS[0], &LOOKS[1]] {
        for run in [false, true] {
            let frames = looked(&rig, script, run, DriverConfig::default());
            let onset: Vec<&Stance> = frames
                .iter()
                .filter(|frame| frame.t >= STAND_SECS - 0.25 && frame.t < STAND_SECS + 1.0)
                .collect();
            let (planted, any) = jumps(&onset);
            println!(
                "{} | {}: setting off from standing, slide {:.1} mm; per frame a planted ankle \
                 {:.1} mm, any {:.1} mm",
                script.name,
                if run { "run" } else { "walk" },
                slide(&onset) * 1000.0,
                planted * 1000.0,
                any * 1000.0,
            );
        }
    }
    for script in &LOOKS[14..] {
        for run in [false, true] {
            let frames = looked(&rig, script, run, DriverConfig::default());
            let across = changing(&frames, script);
            let (planted, any) = jumps(&across);
            println!(
                "{} | {}: slide {:.1} mm; per frame a planted ankle {:.1} mm, any {:.1} mm; feet \
                 {:.1} mm apart at closest",
                script.name,
                if run { "run" } else { "walk" },
                slide(&across) * 1000.0,
                planted * 1000.0,
                any * 1000.0,
                closest_feet(&across) * 1000.0,
            );
        }
    }
    for rate in [0.5f32, 0.75, 1.0] {
        for drift in [0.15f32, 0.2, 0.25] {
            let frames = drifting(&rig, rate, drift);
            let (before, _) = jumps(
                &frames
                    .iter()
                    .filter(|frame| frame.t >= 1.0 && frame.t < DRIFT_FROM)
                    .collect::<Vec<_>>(),
            );
            let (planted, any) = jumps(&appearing(&frames));
            println!(
                "pivot {rate} rad/s drifting left {drift} m/s: as the heading appears, per frame \
                 a planted ankle {:.1} mm (before it {:.1}), any {:.1} mm",
                planted * 1000.0,
                before * 1000.0,
                any * 1000.0,
            );
        }
    }
}

/// When a [`drifting`] body starts to drift, in seconds.
const DRIFT_FROM: f32 = 2.0;

/// A body stepping round on the spot at `rate` that starts drifting to its
/// left at `drift` m/s after [`DRIFT_FROM`] — under the gait's threshold, and
/// handed the heading its drift makes as the consuming application hands one
/// from a tenth of a metre a second.
fn drifting(rig: &Rig, rate: f32, drift: f32) -> Vec<Stance> {
    let reader = Reader::of(rig);
    let mut driver = Driver::new(DriverConfig::default(), INSTRUMENT_SEED);
    let (mut facing, mut at) = (0.0f32, Vec3::ZERO);
    (0..((DRIFT_FROM + 1.0) / CONTROLLER_STEP) as usize)
        .map(|frame| {
            let t = frame as f32 * CONTROLLER_STEP;
            facing = wrapped(facing + rate * CONTROLLER_STEP);
            let drifting = t >= DRIFT_FROM;
            let velocity = if drifting {
                symbios_avatar::Quat::from_rotation_y(facing) * Vec3::X * drift
            } else {
                Vec3::ZERO
            };
            at += velocity * CONTROLLER_STEP;
            let driven = driver
                .drive(
                    rig,
                    &Inputs {
                        delta: CONTROLLER_STEP,
                        velocity,
                        at,
                        facing,
                        heading: drifting.then_some(symbios_avatar::Heading::LEFT),
                        turn: rate,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            reader.read(rig, &driver, &driven, t, at, facing)
        })
        .collect()
}

/// The frames of a [`drifting`] run around the one its heading appears on.
fn appearing(frames: &[Stance]) -> Vec<&Stance> {
    frames
        .iter()
        .filter(|frame| frame.t >= DRIFT_FROM - 0.1 && frame.t < DRIFT_FROM + 0.2)
        .collect()
}

#[test]
#[ignore = "probe for #361 and #362: prints the first-person tables through the replica chassis"]
fn probe_what_first_person_does_to_the_stance() {
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    for script in &LOOKS {
        for run in [false, true] {
            let frames = looked(&rig, script, run, DriverConfig::default());
            let end = script.length();
            for (window, from, to) in [
                ("held", (end - 1.0).max(STAND_SECS), end),
                ("across", STAND_SECS + 0.5, end),
                ("after", end + 1.0, end + TAIL_SECS),
            ] {
                let within: Vec<&Stance> = frames
                    .iter()
                    .filter(|frame| frame.t >= from && frame.t < to)
                    .collect();
                let (lo, hi, split, root, strained) = extremes(&within);
                let gait = within
                    .iter()
                    .filter(|frame| frame.source == Source::Gait)
                    .count();
                println!(
                    "{} | {} | {window}: slide {:.1} mm; feet {:.1} mm apart at closest; splay \
                     {:+.1}..{:+.1} mm; |split| max {:.1} mm; root {:+.1} mm; strained {strained}/{}; \
                     gait {gait}",
                    script.name,
                    if run { "run" } else { "walk" },
                    slide(&within) * 1000.0,
                    closest_feet(&within) * 1000.0,
                    lo * 1000.0,
                    hi * 1000.0,
                    split * 1000.0,
                    root * 1000.0,
                    within.len(),
                );
            }
        }
    }
}

/// The frames of a first-person run inside its script's last second.
fn held<'a>(frames: &'a [Stance], script: &Looking) -> Vec<&'a Stance> {
    let end = script.length();
    frames
        .iter()
        .filter(|frame| frame.t >= (end - 1.0).max(STAND_SECS) && frame.t < end)
        .collect()
}

/// The nearest a body's feet may come to each other: half the width they
/// stand at, the gait's own clearance contract (`SHUFFLE_CLEARANCE`).
fn clearance(rig: &Rig) -> f32 {
    let feet = feet(rig);
    (rig.joints[feet[0]].position.x - rig.joints[feet[1]].position.x).abs() * 0.5
}

#[test]
fn a_strafe_keeps_its_planted_soles_and_its_feet_apart() {
    // **The defect #362 was filed for**, on the consuming application's own
    // chassis and body. Through 0.11 a strafe at the application's walk slid
    // its planted soles 533 mm over a second held — 331 at the run — and its
    // feet passed within a millimetre of each other: the shuffle held the
    // stride to 88 mm on the default body while the cadence came from the
    // body's whole speed. Now the stride is the heading's working one, a
    // crossover in a split stance: the soles hold as a forward walk's do, and
    // the feet pass each other a split's depth apart.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let apart = clearance(&rig);
    let control: Vec<Stance> = looked(&rig, &LOOKS[0], false, DriverConfig::default());
    let walked = slide(&held(&control, &LOOKS[0]));
    assert!(
        walked < 0.005,
        "control: the forward walk itself slid {:.1} mm",
        walked * 1000.0
    );
    for script in [&LOOKS[1], &LOOKS[2]] {
        for run in [false, true] {
            let frames = looked(&rig, script, run, DriverConfig::default());
            let window = held(&frames, script);
            let pace = if run { "run" } else { "walk" };
            assert!(
                window.iter().all(|frame| frame.source == Source::Gait),
                "{} at a {pace}: the body stopped walking",
                script.name
            );
            let slid = slide(&window);
            assert!(
                slid < 0.005,
                "{} at a {pace}: the planted soles slid {:.1} mm (the forward walk: {:.1})",
                script.name,
                slid * 1000.0,
                walked * 1000.0
            );
            let closest = closest_feet(&window);
            assert!(
                closest > apart,
                "{} at a {pace}: the feet passed {:.1} mm apart, inside the {:.1} mm clearance",
                script.name,
                closest * 1000.0,
                apart * 1000.0
            );
            assert!(
                window.iter().all(|frame| !frame.strained),
                "{} at a {pace}: a leg strained",
                script.name
            );
            // The trailing foot stands in front for the whole cycle, swing and
            // flight included: a crossing swing passes the planted foot at
            // its own depth. The eased flight once took its path from the
            // square stance's homes, and every swing crossed back through
            // the middle — with the planted feet split, and the clearance
            // above still held.
            let leading_left = std::ptr::eq(script, &LOOKS[1]);
            let behind = window
                .iter()
                .map(|frame| {
                    if leading_left {
                        -frame.split
                    } else {
                        frame.split
                    }
                })
                .fold(f32::MAX, f32::min);
            assert!(
                behind > apart,
                "{} at a {pace}: the trailing foot stood only {:.1} mm in front at one frame",
                script.name,
                behind * 1000.0
            );
        }
    }
}

#[test]
fn a_back_up_and_the_diagonals_keep_their_planted_soles() {
    // The other half of #362: every heading's stride is the working one, so a
    // body backing up or going diagonally covers exactly the ground it treads.
    // Through 0.11 the stride was the reach's share of the travel at the
    // cadence of the whole: S slid 91 mm a second held, S+A 148, W+D 106.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    for script in [&LOOKS[3], &LOOKS[4], &LOOKS[5]] {
        for run in [false, true] {
            let frames = looked(&rig, script, run, DriverConfig::default());
            let window = held(&frames, script);
            let slid = slide(&window);
            assert!(
                slid < 0.005,
                "{} at a {}: the planted soles slid {:.1} mm",
                script.name,
                if run { "run" } else { "walk" },
                slid * 1000.0
            );
        }
    }
}

#[test]
fn a_strafe_turning_into_a_backward_diagonal_keeps_its_stance() {
    // **The heading the split stance is held through** (#362). A strafe and a
    // backward diagonal both stand split — the same foot in front, the same
    // depth between the feet — so the change between them re-arranges
    // nothing: the feet stay a split apart and only the travel turns. Through
    // 0.11 the same keys slid the soles 299 mm walking and 170 running, and
    // passed the feet 3-4 cm apart; a square backward diagonal passes them at
    // half its stance width (72 mm on this body) where the split keeps 150.
    // Across every moment of the stride the keys can change at: 23 mm on
    // average at a walk and 48 at worst, a millimetre or two at a run.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let apart = clearance(&rig);
    for spans in swept((LEFTWARD, 0.0), (BACK_LEFT, 0.0)) {
        let script = Looking {
            name: "A then S+A",
            spans: &spans,
        };
        for run in [false, true] {
            let frames = looked(&rig, &script, run, DriverConfig::default());
            let across = changing(&frames, &script);
            let pace = if run { "run" } else { "walk" };
            let slid = slide(&across);
            assert!(
                slid < 0.06,
                "at a {pace}, keys changing at {:.2} s: the planted soles slid {:.1} mm crossing \
                 from the strafe",
                spans[0].0,
                slid * 1000.0
            );
            let closest = closest_feet(&across);
            assert!(
                closest > apart,
                "at a {pace}, keys changing at {:.2} s: the feet passed {:.1} mm apart, inside \
                 the {:.1} mm clearance",
                spans[0].0,
                closest * 1000.0,
                apart * 1000.0
            );
        }
    }
}

/// The frames of a first-person run from a quarter second before its keys
/// change to the end of its script: the change, and what it settles into.
fn changing<'a>(frames: &'a [Stance], script: &Looking) -> Vec<&'a Stance> {
    let change = STAND_SECS + script.spans[0].0;
    frames
        .iter()
        .filter(|frame| frame.t >= change - 0.25 && frame.t < script.length())
        .collect()
}

/// A change of keys at every moment of the stride: the two-span script
/// `first` then `second` — keys and the eyes' turn — with its first span held
/// a little longer each time, over more than a walk's whole cycle. Where in
/// its stride a body is when the keys change decides most of what the change
/// costs it, so a guard on one moment is a guard on luck.
fn swept(
    first: (Vec3, f32),
    second: (Vec3, f32),
) -> impl Iterator<Item = [(f32, Vec3, bool, f32); 2]> {
    (0..16).map(move |step| {
        [
            (0.8 + step as f32 * 0.05, first.0, false, first.1),
            (1.5, second.0, false, second.1),
        ]
    })
}

#[test]
fn a_change_of_heading_slides_less_than_the_turn_the_stance_agreed() {
    // **What a change of keys costs, written down rather than fixed** (#362).
    // A planted foot was put down for the travel the body had; when the keys
    // turn it, the stride the foot is held against turns with them, and the
    // footholds' bounds — tight across the stride, for the orbit body's
    // lagging facing (#337) — let it give the difference. How much depends on
    // where in its stride the body is when the keys change, so every moment
    // is tried: on this body at a walk, into a strafe 183 mm on average and
    // 381 at worst, out of one 123 and 254, into a diagonal 54 and 116; at a
    // run 46 and 163 at most. The ruler is the one the consuming application
    // agreed for a quarter turn in the orbit (#1323): that turn slides 339 mm
    // against a 420 mm ceiling.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    for (name, first, second) in [
        ("W then A", AHEAD, LEFTWARD),
        ("W then W+D", AHEAD, AHEAD_RIGHT),
        ("A then W", LEFTWARD, AHEAD),
    ] {
        for spans in swept((first, 0.0), (second, 0.0)) {
            let script = Looking {
                name,
                spans: &spans,
            };
            for run in [false, true] {
                let frames = looked(&rig, &script, run, DriverConfig::default());
                let slid = slide(&changing(&frames, &script));
                assert!(
                    slid < 0.42,
                    "{name} at a {}, keys changing at {:.2} s: the planted soles slid {:.1} mm \
                     through the change",
                    if run { "run" } else { "walk" },
                    spans[0].0,
                    slid * 1000.0
                );
            }
        }
    }
}

#[test]
fn a_body_turning_where_it_stands_steps_round_on_planted_feet() {
    // **#361.** A body looking round on the spot used to idle under its own
    // turn: the idle poses its feet in the body, so they went round on the
    // floor with it. Now it walks a pivot — a stride of no length whose
    // stances turn — and its planted feet stay where they are while the other
    // steps round. Held at 90 degrees a second, and handed back to the idle
    // once the turn stops.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let apart = clearance(&rig);
    let script = &LOOKS[7];
    let frames = looked(&rig, script, false, DriverConfig::default());
    let window = held(&frames, script);
    assert!(
        window.iter().all(|frame| frame.source == Source::Gait),
        "a body turning where it stands did not step"
    );
    // It steps: each foot is down at some frame and up at another.
    for foot in 0..2 {
        assert!(
            window.iter().any(|frame| frame.down[foot])
                && window.iter().any(|frame| !frame.down[foot]),
            "foot {foot} never stepped"
        );
    }
    let slid = slide(&window);
    assert!(
        slid < 0.005,
        "the planted feet slid {:.1} mm",
        slid * 1000.0
    );
    let closest = closest_feet(&window);
    assert!(
        closest > apart,
        "the feet passed {:.1} mm apart, inside the {:.1} mm clearance",
        closest * 1000.0,
        apart * 1000.0
    );
    assert!(window.iter().all(|frame| !frame.strained), "a leg strained");
    let end = script.length();
    assert!(
        frames
            .iter()
            .filter(|frame| frame.t >= end + 1.0)
            .all(|frame| frame.source == Source::Idle),
        "a second after the turn stopped, the body was not idling"
    );

    // The control on the threshold: a look round under `PIVOT_FROM` is the
    // idle's, and never steps.
    let slow = &LOOKS[9];
    let frames = looked(&rig, slow, false, DriverConfig::default());
    assert!(
        frames.iter().all(|frame| frame.source != Source::Gait),
        "a body turning five degrees a second stepped"
    );
}

#[test]
fn a_walk_turning_with_its_eyes_keeps_its_feet() {
    // A body walking ahead while its eyes turn sixty degrees a second turns
    // its stride with them (#361): the stance's yaw holds each planted foot's
    // bearing and the ledger stops carrying it round, and a turning swing
    // eases along its arc rather than across the chord, which cut toward the
    // other foot. Through 0.11, fed the same turn, the walk slid its soles
    // 45.5 mm a second and passed its feet 0.8 mm apart.
    let avatar = body_of("did:plc:stop-test");
    let rig = avatar.rig.clone();
    let apart = clearance(&rig);
    let script = &LOOKS[10];
    for run in [false, true] {
        let frames = looked(&rig, script, run, DriverConfig::default());
        let window = held(&frames, script);
        let pace = if run { "run" } else { "walk" };
        let slid = slide(&window);
        assert!(
            slid < 0.015,
            "at a {pace}: the planted soles slid {:.1} mm",
            slid * 1000.0
        );
        let closest = closest_feet(&window);
        assert!(
            closest > apart,
            "at a {pace}: the feet passed {:.1} mm apart, inside the {:.1} mm clearance",
            closest * 1000.0,
            apart * 1000.0
        );
    }
}

#[test]
fn a_pivot_whose_heading_appears_keeps_its_planted_feet() {
    // **The heading arrives before the gait does** (#361, #362). The consuming
    // application hands a first-person body its heading from a tenth of a
    // metre a second and the gait starts at three tenths, so a body looking
    // round as it begins to drift walks its pivot under a heading. The first
    // cut of 0.12 took the pivot's stride of no length toward it, split stance
    // and all, and a planted foot moved 126-141 mm in the frame the heading
    // appeared; the drift itself moves one about 4 mm a frame.
    let rig = body_of("did:plc:stop-test").rig;
    for rate in [0.5f32, 0.75, 1.0] {
        for drift in [0.15f32, 0.2, 0.25] {
            let frames = drifting(&rig, rate, drift);
            let (planted, _) = jumps(&appearing(&frames));
            assert!(
                planted < 0.01,
                "turning {rate} rad/s, drifting {drift} m/s: a planted foot moved {:.1} mm in \
                 one frame as the heading appeared",
                planted * 1000.0
            );
        }
    }
}

#[test]
fn setting_off_from_a_pivot_blends_as_setting_off_from_standing_does() {
    // **The one change inside locomotion where the stride jumps** (#361): a
    // pivot's has no length and a walk's starts at the crossing speed's, so
    // setting off from a turn moves every contact at once, and so does
    // stopping into one. Through the first cut of 0.12 nothing blended it and
    // a planted foot moved up to 157 mm in one frame; blended, as the idle's
    // start is, the worst frame at any moment of the stride is 41 mm setting
    // off and 75 stopping, where a walk set off from standing moves one 38.
    let rig = body_of("did:plc:stop-test").rig;
    let turning = std::f32::consts::FRAC_PI_2;
    for (name, first, second) in [
        ("looking round, then A", STILL, LEFTWARD),
        ("looking round, then W", STILL, AHEAD),
        ("A looking round, then stopping", LEFTWARD, STILL),
    ] {
        for spans in swept((first, turning), (second, turning)) {
            let script = Looking {
                name,
                spans: &spans,
            };
            for run in [false, true] {
                let frames = looked(&rig, &script, run, DriverConfig::default());
                let (planted, _) = jumps(&changing(&frames, &script));
                assert!(
                    planted < 0.08,
                    "{name} at a {}, keys changing at {:.2} s: a planted foot moved {:.1} mm in \
                     one frame",
                    if run { "run" } else { "walk" },
                    spans[0].0,
                    planted * 1000.0
                );
            }
        }
    }
}

#[test]
fn a_turn_that_ends_hands_back_the_straight_walk() {
    // **The turn is eased like the pace, and the ease lands** (#361). A lag
    // closes a share of its gap every frame, so on its own it never closes
    // it: the first cut of 0.12 stalled 1.3e-44 short of zero, and a body
    // that had once turned walked a turning stride ever after — 5.4 mm from
    // the straight walk at a joint ninety seconds on, which is every
    // first-person body in the consuming application that ever looked round.
    // Two bodies on one clock, one of them told a second's turn: once the
    // turn has eased out and each foot has stepped since, they are one body
    // to the bit.
    let rig = body_of("did:plc:stop-test").rig;
    let cadence = Speed::new(&rig, 1.4).cadence(&rig);
    let (mut turned, mut straight) = (
        Driver::seeded(INSTRUMENT_SEED),
        Driver::seeded(INSTRUMENT_SEED),
    );
    for frame in 0..(12.0 / STEP) as usize {
        let t = frame as f32 * STEP;
        let inputs = |turn: f32| Inputs {
            delta: STEP,
            velocity: Vec3::Z * 1.4,
            at: Vec3::Z * (1.4 * t),
            cycle: Some((cadence * t).fract()),
            turn,
            ..Inputs::default()
        };
        let told = if (1.0..2.0).contains(&t) { 1.5 } else { 0.0 };
        let a = turned
            .drive(&rig, &inputs(told), level_ground)
            .expect("an unheld body is posed");
        let b = straight
            .drive(&rig, &inputs(0.0), level_ground)
            .expect("an unheld body is posed");
        if t >= 8.0 {
            assert!(
                a == b,
                "{t:.2} s: the body that stopped turning at 2 s walks differently"
            );
        }
    }
    assert_eq!(turned.turning(), 0.0, "the eased turn never landed");
}

#[test]
fn a_heading_straight_ahead_is_no_heading_at_all() {
    // A first-person body walking ahead is handed a heading straight ahead
    // where an orbit body is handed none, and at a steady pace the two are
    // one walk to the bit — the orbit's, which every measurement here was
    // taken on. The first cut of 0.12 stepped a heading's stride at a cadence
    // of its own spelling, which at a steady pace is the speed axis's to a
    // float's resolution and no closer: at most of these speeds the clocks
    // drifted apart by a ULP a frame, a millimetre at a joint by the end.
    let rig = body_of("did:plc:stop-test").rig;
    for speed in [0.4f32, 0.5, 0.9, 1.1, 1.4, 1.6, 1.8, 2.0, 2.9, 3.6, 4.0] {
        let (mut ahead, mut none) = (
            Driver::seeded(INSTRUMENT_SEED),
            Driver::seeded(INSTRUMENT_SEED),
        );
        for frame in 0..(4.0 / STEP) as usize {
            let t = frame as f32 * STEP;
            let inputs = |heading| Inputs {
                delta: STEP,
                velocity: Vec3::Z * speed,
                at: Vec3::Z * (speed * t),
                heading,
                ..Inputs::default()
            };
            let a = ahead
                .drive(
                    &rig,
                    &inputs(Some(symbios_avatar::Heading::FORWARD)),
                    level_ground,
                )
                .expect("an unheld body is posed");
            let b = none
                .drive(&rig, &inputs(None), level_ground)
                .expect("an unheld body is posed");
            assert!(
                a == b,
                "{speed} m/s, {t:.2} s: the heading straight ahead walked differently"
            );
        }
    }
}

#[test]
fn a_heading_steps_the_ground_the_body_covers() {
    // **Not the ground its pace covers** (#362). The pace eases behind a
    // change of speed so the posture does not snap, and a heading's stance
    // stepped at the pace's cadence covers the pace's ground while the body
    // covers its own. Stepped so, a first-person walk set off from standing
    // slid its planted soles 365 mm and a run 529 — the orbit body's own
    // start too, until #363 stepped it the same way — and setting off ahead
    // from a turn slid them 390 mm at worst walking and 665 running. Stepped
    // at the ground the body covers: 163 from standing, and 103 at worst from
    // a turn.
    let rig = body_of("did:plc:stop-test").rig;
    for run in [false, true] {
        let pace = if run { "run" } else { "walk" };
        let frames = looked(&rig, &LOOKS[0], run, DriverConfig::default());
        let start: Vec<&Stance> = frames
            .iter()
            .filter(|frame| frame.t >= STAND_SECS - 0.25 && frame.t < STAND_SECS + 1.0)
            .collect();
        let slid = slide(&start);
        assert!(
            slid < 0.25,
            "a {pace} set off from standing: the planted soles slid {:.1} mm",
            slid * 1000.0
        );
        let turning = std::f32::consts::FRAC_PI_2;
        for spans in swept((STILL, turning), (AHEAD, turning)) {
            let script = Looking {
                name: "looking round, then W",
                spans: &spans,
            };
            let frames = looked(&rig, &script, run, DriverConfig::default());
            let slid = slide(&changing(&frames, &script));
            assert!(
                slid < 0.15,
                "a {pace} set off from a turn at {:.2} s: the planted soles slid {:.1} mm",
                spans[0].0,
                slid * 1000.0
            );
        }
    }
}

#[test]
fn a_body_with_no_heading_steps_the_ground_it_covers_too() {
    // **The orbit's own starts and changes of pace** (#363). A body handed no
    // heading kept its pace's cadence through 0.12.0, so its stance covered
    // the eased pace's ground while the consuming application's chassis
    // ramped away from it: setting off from standing slid its planted soles
    // 316 mm on average and 394 at worst over the walking speeds, 506 and 529
    // over the running ones, and a change of pace 160 at a walk into a run
    // and 114 out of one. Stepped at the ground the body covers, as a
    // heading's stride is: 164 from standing at any speed, and 19 at worst
    // through a change of pace.
    let rig = body_of("did:plc:stop-test").rig;
    // A start from standing begins at cycle 0 whatever the stand, so it is
    // swept by the speed the keys ask for, from a slow walk to a fast run.
    for step in 0..16 {
        let speed = 1.0 + 5.0 * step as f32 / 15.0;
        let script = Script {
            name: "W from standing",
            spans: &[(2.0, W, false)],
        };
        let frames = turned_at(&rig, &script, speed, DriverConfig::default());
        let start: Vec<&Stance> = frames
            .iter()
            .filter(|frame| frame.t >= STAND_SECS && frame.t < STAND_SECS + 2.0)
            .collect();
        let slid = slide(&start);
        assert!(
            slid < 0.2,
            "{speed:.2} m/s set off from standing: the planted soles slid {:.1} mm",
            slid * 1000.0
        );
    }
    // A change of pace, at every moment of the stride.
    let walk = Speed::from_froude(WALK_FROUDE).metres_per_second(&rig);
    for (name, first, second) in [
        ("W then Shift", (W, false), (W, true)),
        ("Shift+W then W", (W, true), (W, false)),
    ] {
        for spans in swept_orbit(first, second) {
            let script = Script {
                name,
                spans: &spans,
            };
            let frames = turned_at(&rig, &script, walk, DriverConfig::default());
            let change = STAND_SECS + spans[0].0;
            let after: Vec<&Stance> = frames
                .iter()
                .filter(|frame| frame.t >= change && frame.t < change + 2.0)
                .collect();
            let slid = slide(&after);
            assert!(
                slid < 0.05,
                "{name}, the keys changing at {:.2} s: the planted soles slid {:.1} mm",
                spans[0].0,
                slid * 1000.0
            );
        }
    }
    // **And the quarter turn the consuming application agreed (#1323) no
    // worse for it**, which was the condition of the change: at every moment
    // of the stride a walk turned 313 mm on average and 455 at worst on the
    // pace's cadence, and 317 and 466 on the ground's. Its one agreed moment
    // is the application's own guard.
    let readings: Vec<f32> = swept_orbit((W, false), (D, false))
        .map(|spans| {
            let script = Script {
                name: "W then D",
                spans: &spans,
            };
            let frames = turned_at(&rig, &script, walk, DriverConfig::default());
            let change = STAND_SECS + spans[0].0;
            let after: Vec<&Stance> = frames
                .iter()
                .filter(|frame| frame.t >= change && frame.t < change + 2.0)
                .collect();
            let worst = worst_of(&after);
            assert_eq!(
                worst.strained, 0,
                "the quarter turn at {:.2} s strained {} frames",
                spans[0].0, worst.strained
            );
            worst.slide
        })
        .collect();
    let mean = readings.iter().sum::<f32>() / readings.len() as f32;
    let worst = readings.iter().copied().fold(0.0f32, f32::max);
    assert!(
        mean < 0.33 && worst < 0.48,
        "the quarter turn slid {:.1} mm on average and {:.1} at worst",
        mean * 1000.0,
        worst * 1000.0
    );
}

#[test]
fn a_jittering_velocity_is_stepped_at_its_mean() {
    // A heading's stance covers the ground the body covers, read off the
    // velocity it is handed (#362) — and a velocity differenced from a peer's
    // smoothed transform jitters. Stepped as often above its mean as below
    // it, the jitter costs the cadence nothing: a fifth of noise steps a walk
    // ahead 0.5% more cycles than none and a strafe at 3 m/s 0.7%. Floored at
    // the pace's own cadence, as the first cut of 0.12 was, the walk ahead
    // stepped 7% fast; capped at the pace's own heading cadence, the strafe
    // 4.6% slow.
    let rig = body_of("did:plc:stop-test").rig;
    let stepped = |heading: symbios_avatar::Heading, speed: f32, noise: f32| {
        let mut driver = Driver::seeded(INSTRUMENT_SEED);
        let toward = heading.direction();
        let (mut seed, mut at) = (12_345u32, Vec3::ZERO);
        let (mut cycles, mut last) = (0.0f32, 0.0f32);
        for _ in 0..(6.0 / CONTROLLER_STEP) as usize {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let jitter = (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0;
            at += toward * (speed * CONTROLLER_STEP);
            driver
                .drive(
                    &rig,
                    &Inputs {
                        delta: CONTROLLER_STEP,
                        velocity: toward * (speed * (1.0 + noise * jitter)),
                        at,
                        heading: Some(heading),
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            cycles += (driver.cycle() - last).rem_euclid(1.0);
            last = driver.cycle();
        }
        cycles
    };
    for (heading, speed) in [
        (symbios_avatar::Heading::FORWARD, 1.4),
        (symbios_avatar::Heading::LEFT, 3.0),
    ] {
        let (steady, jittered) = (stepped(heading, speed, 0.0), stepped(heading, speed, 0.2));
        assert!(
            (jittered / steady - 1.0).abs() < 0.015,
            "{} at {speed} m/s: a fifth of velocity noise stepped {:+.1}% the cycles",
            heading.describe(),
            (jittered / steady - 1.0) * 100.0
        );
    }
}

#[test]
fn a_squeezed_stride_steps_no_faster_than_its_heading_or_a_sprint() {
    // **The ceiling on a heading's cadence** (#362). Where a split stance
    // lines its feet up — the backward diagonals near 150-165 degrees on this
    // body — the stance bound shortens the stride to a few centimetres, and
    // covering the travel in steps that short would cycle the legs as fast
    // as the stride is short: without the ceiling a walk there stepped 4.55
    // cycles a second and a run 6.02, twice what the heading's own reach
    // asks. Held to the faster of that and the legs' sprint cadence, the
    // stance covers what it can and the planted foot gives the rest.
    let rig = body_of("did:plc:stop-test").rig;
    let sprint = Speed::from_froude(symbios_avatar::anim::speed::FROUDE_SPRINT).cadence(&rig);
    for speed in [1.4f32, 4.0] {
        for degrees in 90..=180 {
            let heading = symbios_avatar::Heading::degrees(degrees as f32);
            let toward = heading.direction();
            let mut driver = Driver::seeded(INSTRUMENT_SEED);
            let (mut cycles, mut last) = (0.0f32, 0.0f32);
            for frame in 0..(2.0 / STEP) as usize {
                let t = frame as f32 * STEP;
                driver
                    .drive(
                        &rig,
                        &Inputs {
                            delta: STEP,
                            velocity: toward * speed,
                            at: toward * (speed * t),
                            heading: Some(heading),
                            ..Inputs::default()
                        },
                        level_ground,
                    )
                    .expect("an unheld body is posed");
                cycles += (driver.cycle() - last).rem_euclid(1.0);
                last = driver.cycle();
            }
            let stepped = cycles / 2.0;
            let ceiling = (Speed::new(&rig, speed).cadence(&rig) / heading.reach()).max(sprint);
            assert!(
                stepped < ceiling * 1.01,
                "{speed} m/s at {degrees} degrees: stepped {stepped:.2} cycles a second, past \
                 the {ceiling:.2} its heading and a sprint allow"
            );
        }
    }
}

/// A beast of `family` as the instruments here build one, and how high its
/// withers stand, in metres.
fn beast_of(family: plan::Family) -> (Avatar, f32) {
    let mut params = QuadrupedParams::default();
    params.set_kind(family, None, 0.0);
    let withers = params.height * 1.2;
    let record = AvatarRecord::new("Rester", Archetype::Quadruped(params));
    let avatar = Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("a beast builds");
    (avatar, withers)
}

/// What gets a sitting or lying beast up (#377).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Interrupt {
    /// The body carried off at a walk.
    Walk,
    /// The body thrown into the air.
    Leap,
    /// The body in deep water.
    Swim,
    /// Another gesture asked for.
    Gesture,
    /// [`emote::GET_UP`] asked for.
    Stand,
}

/// When it comes: part of the way down, held, or part of the way up.
#[derive(Clone, Copy, Debug, PartialEq)]
enum When {
    GoingDown,
    Held,
    GettingUp,
}

/// The gesture an interrupting [`Interrupt::Gesture`] asks for.
const SECOND: &str = "Shake";

/// How long after a beast is up its pose is still the rest's to answer for,
/// in seconds: the blend that hands it back, and a frame either side.
const HANDED: f32 = 0.2;

/// Puts a beast into `state`, gets it up `when` by `interrupt`, and answers
/// everything wrong with how it went: a joint that jumped in a frame, a
/// foot in the ground, a beast not up when it should be, a second gesture
/// that did not play.
fn interrupted(family: plan::Family, state: &str, interrupt: Interrupt, when: When) -> Vec<String> {
    let (avatar, withers) = beast_of(family);
    let rig = &avatar.rig;
    let what = format!("{family:?} {state}, {interrupt:?} {when:?}");
    let down = emote::seconds(rig, state).expect("a state keeps its own time");
    let (_, up) = emote::getting_up(rig, state).expect("a beast gets up");
    // In seconds from the request, when the interrupt comes, and by when
    // the beast must be up: the way down undone, or the rise, in a hurry
    // where the body is moving.
    let (asked, hurried) = (
        0.5,
        !matches!(interrupt, Interrupt::Gesture | Interrupt::Stand),
    );
    let pace = if hurried { emote::HURRY } else { 1.0 };
    let (at, rest) = match when {
        When::GoingDown => (asked + 0.4 * down, 0.4 * down),
        When::Held => (asked + down + 0.5, up),
        When::GettingUp => (asked + down + 0.5 + 0.3 * up, 0.7 * up),
    };
    let by = at + rest * pace + 0.1;
    // The feet's own skin, to read where the drawn feet are.
    let body = &avatar.parts.body;
    let weights = &avatar.parts.weights;
    let feet: Vec<usize> = (0..body.positions.len())
        .filter(|&vertex| {
            matches!(
                rig.joints[usize::from(weights.vertices[vertex][0].joint)].zone,
                Zone::Extremity(_)
            )
        })
        .collect();
    let positions: Vec<Vec3> = feet.iter().map(|&vertex| body.positions[vertex]).collect();
    let held = symbios_avatar::SkinWeights {
        vertices: feet
            .iter()
            .map(|&vertex| weights.vertices[vertex])
            .collect(),
    };
    let sink = (0.005 * withers).max(0.002);
    let mut chassis = Chassis::seeded(377);
    let mut wrong = Vec::new();
    let mut before: Option<symbios_avatar::anim::Posed> = None;
    let (mut second, mut handed) = (false, 0.0f32);
    #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
    let frames = ((by + 1.5) / STEP) as usize;
    for frame in 0..frames {
        #[expect(clippy::cast_precision_loss, reason = "a few hundred frames")]
        let time = frame as f32 * STEP;
        let after = (time - at).max(0.0);
        let into = time >= at;
        let place = match interrupt {
            Interrupt::Walk if into => Vec3::Z * (1.2 * after),
            Interrupt::Leap if into => Vec3::Y * (3.0 * after - 4.9 * after * after).max(0.0),
            _ => Vec3::ZERO,
        };
        let swimming = interrupt == Interrupt::Swim && into;
        let first = |moment: f32| time >= moment && time - STEP < moment;
        let request = if first(asked) {
            Some(state)
        } else if when == When::GettingUp && first(asked + down + 0.5) {
            Some(emote::GET_UP)
        } else if first(at) && interrupt == Interrupt::Gesture {
            Some(SECOND)
        } else if first(at) && interrupt == Interrupt::Stand {
            Some(emote::GET_UP)
        } else {
            None
        };
        let driven = chassis.drive(rig, place, swimming, request, None);
        let posed = driven.pose.forward(rig);
        let gesture = chassis.driver.gesture().map(|(name, _)| name.to_string());
        let resting = gesture.as_deref() == Some(state);
        if resting {
            handed = time;
        }
        // Frame to frame, while the beast rests and as it is handed back to
        // whatever carries it next: at an honest pace, or a hurried one from
        // the moment something hurries it up. Whatever carries it after that
        // is not this test's.
        let fast = if hurried && into {
            1.0 / emote::HURRY
        } else {
            1.0
        };
        if let Some(before) = before.as_ref().filter(|_| time - handed <= HANDED) {
            let (moved, joint) = crate::rest::fastest(rig, before, &posed);
            let most = crate::rest::swift(withers, STEP) * fast;
            if moved > most {
                wrong.push(format!(
                    "{what}: joint {joint} ({:?}) moved {:.0} mm at {time:.2} s, past {:.0}",
                    rig.joints[joint].zone,
                    moved * 1000.0,
                    most * 1000.0
                ));
            }
        }
        // Resting on the ground, no foot in it.
        let grounded = chassis.driver.airborne().is_none() && !swimming;
        if grounded && resting {
            let low = posed
                .deform(rig, &positions, &held)
                .iter()
                .map(|at| at.y)
                .fold(f32::MAX, f32::min);
            if low < -sink {
                wrong.push(format!(
                    "{what}: a foot {:.0} mm in the ground at {time:.2} s",
                    -low * 1000.0
                ));
            }
        }
        second |= gesture.as_deref() == Some(SECOND);
        if first(by) && gesture.as_deref() == Some(state) {
            wrong.push(format!("{what}: still in its {state} at {time:.2} s"));
        }
        before = Some(posed);
    }
    if interrupt == Interrupt::Gesture && !second {
        wrong.push(format!("{what}: {SECOND} never played"));
    }
    // A wrong reading every frame of a long fault says nothing the first
    // five do not.
    wrong.truncate(5);
    wrong
}

/// Every interrupt from every moment, on a dog and a horse, for `state`.
fn every_interrupt(state: &str) {
    let mut wrong = Vec::new();
    for family in [plan::Family::Dog, plan::Family::Horse] {
        for interrupt in [
            Interrupt::Walk,
            Interrupt::Leap,
            Interrupt::Swim,
            Interrupt::Gesture,
            Interrupt::Stand,
        ] {
            for when in [When::GoingDown, When::Held, When::GettingUp] {
                wrong.extend(interrupted(family, state, interrupt, when));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_sitting_beast_gets_up_from_wherever_it_is_for_whatever_gets_it_up() {
    // #377: a beast's state is held until something gets it up - being
    // moved, leaving the ground, swimming, another gesture, or Get Up - and
    // then it rises from wherever its body is, never snapping. Held for a
    // fixed time and then replaced by a second gesture, a dog's sit stood it
    // up 644 mm in a frame; walked off while seated its hips stayed 423 mm
    // down and its pads dragged 115 mm through the ground.
    every_interrupt("Sit");
}

#[test]
fn a_lying_beast_gets_up_from_wherever_it_is_for_whatever_gets_it_up() {
    // As the sit: a horse's lying down replaced by a greeting moved it
    // 1113 mm in a frame.
    every_interrupt("Lie Down");
}

#[test]
fn a_beasts_state_is_held_until_something_gets_it_up() {
    // #377: down at its own pace and held, standing still, long past any
    // gesture's time; and the gesture asked for while it was down plays once
    // it is up.
    let (avatar, _) = beast_of(plan::Family::Dog);
    let rig = &avatar.rig;
    let down = emote::seconds(rig, "Sit").expect("a state keeps its own time");
    let (_, up) = emote::getting_up(rig, "Sit").expect("a beast gets up");
    let mut chassis = Chassis::seeded(377);
    let held = down + 10.0;
    #[expect(clippy::cast_possible_truncation, reason = "a thousand frames")]
    let frames = ((held + up + 1.0) / STEP) as usize;
    let mut seen = Vec::with_capacity(frames);
    for frame in 0..frames {
        #[expect(clippy::cast_precision_loss, reason = "a thousand frames")]
        let time = frame as f32 * STEP;
        let request = match frame {
            0 => Some("Sit"),
            _ if (time - held).abs() < STEP * 0.5 => Some(SECOND),
            _ => None,
        };
        chassis.drive(rig, Vec3::ZERO, false, request, None);
        seen.push(chassis.driver.gesture().map(|(name, _)| name.to_string()));
    }
    #[expect(clippy::cast_possible_truncation, reason = "a thousand frames")]
    let at = |seconds: f32| seen[((seconds / STEP) as usize).min(frames - 1)].clone();
    assert_eq!(
        at(held - 0.1).as_deref(),
        Some("Sit"),
        "the sit was not held ten seconds"
    );
    assert_eq!(
        at(held + up * 0.5).as_deref(),
        Some("Sit"),
        "the second gesture cut the getting up short"
    );
    assert_eq!(
        at(held + up + 0.2).as_deref(),
        Some(SECOND),
        "the gesture asked for while down did not play once it was up"
    );
}

#[test]
fn a_persons_gesture_reads_its_length_as_it_plays() {
    // #377 left a person's gestures as they were, and that includes this:
    // `gesture_secs` is read every frame, so a caller shortening it part way
    // through a wave ends the wave by the new length, not the old one.
    let person = body_of("did:plc:transition-test");
    let rig = &person.rig;
    let mut chassis = Chassis::seeded(INSTRUMENT_SEED);
    let shorter = 0.6;
    #[expect(clippy::cast_possible_truncation, reason = "a hundred frames")]
    let (changed, ended) = (
        (0.3 / STEP) as usize,
        ((shorter + 2.0 * STEP) / STEP) as usize,
    );
    for frame in 0..=ended {
        if frame == changed {
            chassis.driver.set_config(DriverConfig {
                gesture_secs: shorter,
                ..chassis.driver.config()
            });
        }
        if frame == 0 {
            chassis.ask(rig, Vec3::ZERO, "Greeting");
        } else {
            chassis.step(rig, Vec3::ZERO);
        }
    }
    assert!(
        chassis.driver.gesture().is_none(),
        "the greeting played on to {:?} past a length of {shorter} s",
        chassis.driver.gesture()
    );
}

/// How far any joint of a standing body's neck and head moves in one frame,
/// over a minute of its idle, as a share of its height.
fn head_jump(record: &AvatarRecord, seed: u64) -> f32 {
    let avatar = Avatar::build_with(
        record,
        &AvatarConfig {
            atlas: 64,
            ..AvatarConfig::default()
        },
    )
    .expect("a beast builds");
    let rig = &avatar.rig;
    let height = rig
        .joints
        .iter()
        .map(|joint| joint.position.y)
        .fold(0.0, f32::max);
    let watched: Vec<usize> = (0..rig.joints.len())
        .filter(|&joint| {
            !rig.joints[joint].marker && matches!(rig.joints[joint].zone, Zone::Neck | Zone::Head)
        })
        .collect();
    let mut driver = Driver::seeded(seed);
    let mut last: Option<Vec<Vec3>> = None;
    let mut worst: f32 = 0.0;
    for _ in 0..3600 {
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta: STEP,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("an unheld body is posed every frame");
        let at = driven.pose.forward(rig).positions;
        if let Some(last) = &last {
            for &joint in &watched {
                worst = worst.max(last[joint].distance(at[joint]));
            }
        }
        last = Some(at);
    }
    worst / height
}

#[test]
fn a_standing_beast_s_head_turns_to_a_glance_rather_than_snapping() {
    // Taken whole the moment a fidget started and dropped the moment it ended, a
    // glance moved a horse's nose 445 mm in one frame, .18 of its height (#382).
    let mut worst = (0.0f32, String::new());
    let mut read = |name: String, record: &AvatarRecord, seed: u64| {
        let jump = head_jump(record, seed);
        if jump > worst.0 {
            worst = (jump, name);
        }
    };
    for family in plan::Family::ALL {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
        for seed in [0, 9, 21] {
            read(format!("{family:?} driver {seed}"), &record, seed);
        }
    }
    for roll in [1, 6, 9, 23, 72, 258] {
        let record = AvatarRecord::rolled(
            "Beast",
            Archetype::Quadruped(QuadrupedParams::default()),
            roll,
        );
        read(format!("roll {roll}"), &record, 9);
    }
    assert!(
        worst.0 < HEAD_JUMP,
        "{}: {} of its height",
        worst.1,
        worst.0
    );
}

#[test]
fn a_standing_person_s_head_turns_to_a_glance_rather_than_snapping() {
    // The same step on a far shorter lever: 20 to 25 mm in one frame (#398).
    for roll in [0, 3, 9] {
        let mut record = AvatarRecord::new("Person", Archetype::default());
        record.reroll(roll);
        let jump = head_jump(&record, 9);
        assert!(
            jump < PERSON_HEAD_JUMP,
            "person {roll}: {jump} of their height"
        );
    }
}

/// See [`a_standing_person_s_head_turns_to_a_glance_rather_than_snapping`]. Set after
/// reading it: the worst is .0011 of a person's height, and .0147 with the fix undone.
const PERSON_HEAD_JUMP: f32 = 0.0015;

/// See [`a_standing_beast_s_head_turns_to_a_glance_rather_than_snapping`]. Set after
/// reading it: the worst is .0123, on a rolled beast.
const HEAD_JUMP: f32 = 0.015;

/// How a beast comes to a stop from `froude` at about the game's braking, at the
/// worst of ten moments of its stride: how far anything of its upper legs falls
/// below where it rests, and how far any joint moves in one frame, each as a share
/// of its height.
fn stopping(rig: &Rig, froude: f32) -> (f32, f32) {
    let height = rig
        .joints
        .iter()
        .map(|joint| joint.position.y)
        .fold(0.0, f32::max);
    let pace = Speed::from_froude(froude).metres_per_second(rig);
    let (mut fall, mut most) = (0.0f32, 0.0f32);
    for moment in 0..10 {
        let stop = 2.0 + moment as f32 * 0.12;
        let mut driver = Driver::seeded(7);
        let (mut at, mut velocity) = (Vec3::ZERO, 0.0f32);
        let mut last: Option<Vec<Vec3>> = None;
        for frame in 0..((stop + 1.2) / STEP) as usize {
            let time = frame as f32 * STEP;
            let wanted = if time < stop { pace } else { 0.0 };
            velocity += (wanted - velocity).clamp(-40.0 * STEP, 40.0 * STEP);
            at += Vec3::Z * velocity * STEP;
            let driven = driver
                .drive(
                    rig,
                    &Inputs {
                        delta: STEP,
                        velocity: Vec3::Z * velocity,
                        at,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed every frame");
            let posed = driven.pose.forward(rig).positions;
            if time >= stop {
                for (joint, here) in posed.iter().enumerate() {
                    if matches!(rig.joints[joint].zone, Zone::UpperLimb(_)) {
                        fall = fall.max(rig.joints[joint].position.y - here.y);
                    }
                    if let Some(last) = &last {
                        most = most.max(last[joint].distance(*here));
                    }
                }
            }
            last = Some(posed);
        }
    }
    (fall / height, most / height)
}

#[test]
fn a_stopping_beast_does_not_plunge() {
    // A stride's feet out of a thigh and a shank's reach asked for the whole height
    // of the leg: a stopping horse's root dropped .74 m in a fifth of a second, with
    // a joint moving up to 1.8 m in a frame, on most stops (#378).
    let mut bodies = Vec::new();
    for family in plan::Family::ALL {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        bodies.push(AvatarRecord::new("Beast", Archetype::Quadruped(params)));
    }
    for roll in [6, 8, 9, 11] {
        bodies.push(AvatarRecord::rolled(
            "Beast",
            Archetype::Quadruped(QuadrupedParams::default()),
            roll,
        ));
    }
    for (index, record) in bodies.iter().enumerate() {
        let avatar = Avatar::build_with(
            record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("a beast builds");
        // The game's walking and running speeds.
        for froude in [0.49, 3.586] {
            let (fall, most) = stopping(&avatar.rig, froude);
            assert!(
                fall < STOP_FALL,
                "body {index} at {froude} falls {fall} of its height"
            );
            assert!(
                most < STOP_MOVES,
                "body {index} at {froude} moves {most} of its height in a frame"
            );
        }
    }
}

/// See [`a_stopping_beast_does_not_plunge`]. Set after reading it: the worst fall is
/// .121 of a height and the worst move in a frame .118, both a cat's at a run, where
/// a canter's swinging foot moves about that anyway.
const STOP_FALL: f32 = 0.135;
const STOP_MOVES: f32 = 0.13;

/// How far the worst joint of `rig` jumps past its own motion in a frame, at a
/// steady `speed` up a plane rising `grade` per metre.
fn pop_on_grade(rig: &Rig, grade: f32, speed: f32) -> f32 {
    let normal = Vec3::new(0.0, 1.0, -grade).normalize();
    let ground = move |at: Vec3| {
        Some(symbios_avatar::anim::Ground {
            position: Vec3::new(at.x, at.z * grade, at.z),
            normal,
        })
    };
    let frame = 1.0 / 60.0;
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let mut at = Vec3::ZERO;
    let mut last: Vec<Vec<Vec3>> = Vec::new();
    let mut worst = 0.0f32;
    for step in 0..300 {
        at += Vec3::Z * speed * frame;
        let inputs = Inputs {
            delta: frame,
            velocity: Vec3::Z * speed,
            at,
            ..Inputs::default()
        };
        let now = driver
            .drive(rig, &inputs, ground)
            .expect("a walking body is posed")
            .pose
            .forward(rig)
            .positions;
        if step > 60
            && let [two, one] = &last[..]
        {
            for joint in (0..now.len()).filter(|&joint| !rig.joints[joint].marker) {
                worst = worst.max((now[joint] - one[joint] * 2.0 + two[joint]).length());
            }
        }
        last.push(now);
        if last.len() > 2 {
            last.remove(0);
        }
    }
    worst
}

#[test]
fn a_leg_solve_does_not_take_a_tear_for_a_root_on_a_steep_grade() {
    // #399: where a wrist comes up by its shoulder, `wrapped` tears and the
    // stride's search settled on the tear or the root, from one frame to the
    // next. Second differences on a grade of .30 in mm, before the search was
    // guarded and after: the cross 1254 and 486, roll 1 1083 and 341, roll 2 625
    // and 166; at .4 still 429, 646 and 425. **Changed deliberately** since
    // each girdle rides its own ground: the trunk pitches with the slope, so no
    // wrist is brought up by its shoulder on a plane at all, and the worst
    // reads 67, 50 and 73 at .3 and 62, 48 and 74 at .4, what a stride on a
    // slope moves anyway. The bounds were each body's own ceiling under the
    // guard, 520, 370 and 180; now one for all three, at both grades.
    let mut cross = QuadrupedParams::default();
    cross.set_kind(plan::Family::Dog, Some(plan::Family::Horse), 0.5);
    let rolled = |roll| {
        AvatarRecord::rolled(
            "Beast",
            Archetype::Quadruped(QuadrupedParams::default()),
            roll,
        )
    };
    for (what, record, speed) in [
        (
            "the cross",
            AvatarRecord::new("Beast", Archetype::Quadruped(cross)),
            1.2,
        ),
        ("roll 1", rolled(1), 1.2),
        ("roll 2", rolled(2), 3.5),
    ] {
        let mut record = record;
        record.sanitize();
        let avatar = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("a beast builds");
        for grade in [0.3, 0.4] {
            let pop = pop_on_grade(&avatar.rig, grade, speed);
            assert!(
                pop < TORN_POP,
                "{what} on {grade}: a joint jumped {:.0} mm past its motion, past {:.0}",
                pop * 1000.0,
                TORN_POP * 1000.0
            );
        }
    }
}

/// How far a joint of a beast walking up a steep plane may jump past its own
/// motion in a frame, in metres: see the test above. Provenance: **measured**
/// (#399): the worst of the three bodies is 74 mm, and a torn leg reads 166 or
/// more.
const TORN_POP: f32 = 0.1;

/// The deepest any foot of `rig` stood into a plane rising `grade` per metre
/// ahead and `camber` across, over the two seconds after it stopped walking at
/// `froude`: its contact joint against where it stands on the plane, in metres.
fn sunk_stepping_home(rig: &Rig, grade: f32, camber: f32, froude: f32) -> f32 {
    let normal = Vec3::new(-camber, 1.0, -grade).normalize();
    let height = move |at: Vec3| at.x * camber + at.z * grade;
    let ground = move |at: Vec3| {
        Some(symbios_avatar::anim::Ground {
            position: Vec3::new(at.x, height(at), at.z),
            normal,
        })
    };
    let pads: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .map(|limb| rig.in_zone(Zone::Extremity(limb))[0])
        .collect();
    let pace = Speed::from_froude(froude).metres_per_second(rig);
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let mut at = Vec3::ZERO;
    let mut worst = 0.0f32;
    for frame in 0..240 {
        let velocity = if frame < 120 {
            Vec3::Z * pace
        } else {
            Vec3::ZERO
        };
        at += velocity * STEP;
        let inputs = Inputs {
            delta: STEP,
            velocity,
            at,
            ..Inputs::default()
        };
        let posed = driver
            .drive(rig, &inputs, ground)
            .expect("an unheld body is posed")
            .pose
            .forward(rig);
        if frame >= 120 {
            for &pad in &pads {
                let foot = posed.positions[pad];
                let stands = height(foot) + rig.joints[pad].position.y;
                worst = worst.max(stands - foot.y);
            }
        }
    }
    worst
}

#[test]
fn a_beast_stepping_its_feet_home_on_a_slope_steps_over_it() {
    // Found beside #400: a stopped beast's recovering step was solved at its
    // rest height over the level floor and lowered with the plant's sink, never
    // asking the ground, so on a .2 grade a fore stepping home uphill passed
    // through the slope for a third of a second: a dog's contact joint 108 mm
    // further into it than it ever stands on the level, a cat's 87 and a
    // horse's 388. Each foot is measured where the ground stands it, both ways
    // up a grade and across one, after a walk and after a trot, against the
    // same reading on the level, where a planted joint stands a few centimetres
    // under its own height as the sole is laid flat.
    let mut wrong = Vec::new();
    for family in plan::Family::ALL {
        let (avatar, withers) = beast_of(family);
        let most = STEPPED_INTO * withers;
        for froude in [0.3, 1.4] {
            let level = sunk_stepping_home(&avatar.rig, 0.0, 0.0, froude);
            for (grade, camber) in [(0.2, 0.0), (-0.2, 0.0), (0.0, 0.2)] {
                let sunk = sunk_stepping_home(&avatar.rig, grade, camber, froude);
                if sunk - level > most {
                    wrong.push(format!(
                        "{family:?} stopped from {froude} on {grade} by {camber}: a foot \
                         stood {:.0} mm into the ground against {:.0} on the level, past {:.0} \
                         more",
                        sunk * 1000.0,
                        level * 1000.0,
                        most * 1000.0
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How much further into a slope than into the level ground any contact joint
/// of a beast stepping its feet home may stand, in withers heights. Provenance:
/// **measured** (found beside #400): at most 29 mm more on a horse trotting to
/// a stop down a .2 grade, 0.017 of its height, and a dog's 24 mm, 0.034 of
/// its; before, 0.15 of a dog's, 0.18 of a cat's and 0.22 of a horse's.
const STEPPED_INTO: f32 = 0.04;

/// A ground of rolling bumps as a consumer answers one from a heightmap: heights
/// sampled on a grid of [`TERRAIN_CELL`] and interpolated bilinearly, its slope
/// the interpolation's own, which changes at once from one cell to the next.
/// The bumps rise to slopes of about .3 at full `bumps`, the grid steps up
/// `step` across the line `edge` metres ahead of the origin, which the
/// interpolation spreads over one cell, and the whole rises `grade` per metre
/// ahead and `camber` across. In the world, its heights and its up.
#[derive(Clone, Copy)]
struct Terrain {
    bumps: f32,
    step: f32,
    edge: f32,
    grade: f32,
    camber: f32,
}

impl Terrain {
    /// The plane rising `grade` per metre ahead and `camber` across.
    fn plane(grade: f32, camber: f32) -> Self {
        Self {
            bumps: 0.0,
            step: 0.0,
            edge: f32::MAX,
            grade,
            camber,
        }
    }
}

/// The spacing of [`Terrain`]'s grid, in metres.
const TERRAIN_CELL: f32 = 0.25;

impl Terrain {
    fn node(self, i: f32, j: f32) -> f32 {
        let (x, z) = (i * TERRAIN_CELL, j * TERRAIN_CELL);
        let bumps = 0.85
            * self.bumps
            * (0.09 * libm::sinf(1.9 * x) * libm::cosf(1.3 * z + 0.4)
                + 0.05 * libm::sinf(2.7 * z + 1.1 * x + 0.7)
                + 0.03 * libm::sinf(4.3 * x - 3.1 * z));
        bumps + if z > self.edge { self.step } else { 0.0 } + z * self.grade + x * self.camber
    }

    fn at(self, x: f32, z: f32) -> (f32, Vec3) {
        let (i, j) = ((x / TERRAIN_CELL).floor(), (z / TERRAIN_CELL).floor());
        let (u, v) = (x / TERRAIN_CELL - i, z / TERRAIN_CELL - j);
        let (a, b) = (self.node(i, j), self.node(i + 1.0, j));
        let (c, d) = (self.node(i, j + 1.0), self.node(i + 1.0, j + 1.0));
        let height = a * (1.0 - u) * (1.0 - v) + b * u * (1.0 - v) + c * (1.0 - u) * v + d * u * v;
        let across = ((b - a) * (1.0 - v) + (d - c) * v) / TERRAIN_CELL;
        let ahead = ((c - a) * (1.0 - u) + (d - b) * u) / TERRAIN_CELL;
        (height, Vec3::new(-across, 1.0, -ahead).normalize())
    }
}

/// The sine and cosine of `facing`, from the pure-Rust `libm` rather than the
/// platform's: a test's own input must not move with the machine's libm, and
/// under the gate's nudged libm the platform's `cos(0)` read a hair under one,
/// so a ramp landing on the idle threshold set off a frame later.
fn sin_cos(facing: f32) -> (f32, f32) {
    (libm::sinf(facing), libm::cosf(facing))
}

/// Where a body `at` facing `facing` stands a point of its own frame in the
/// world, along the ground.
fn into_world(at: Vec3, facing: f32, point: Vec3) -> Vec3 {
    let (sin, cos) = sin_cos(facing);
    Vec3::new(
        at.x + point.x * cos + point.z * sin,
        point.y,
        at.z - point.x * sin + point.z * cos,
    )
}

/// One stretch of [`over_terrain`]'s drive: how many frames, at what Froude number
/// (nought standing), turning how fast, and the gesture asked on its first frame.
type Stretch = (usize, f32, f32, Option<&'static str>);

/// Walking, trotting, stopping, turning where it stands, sitting and getting up,
/// lying down and getting up, and walking off.
const OVER_TERRAIN: [Stretch; 11] = [
    (30, 0.0, 0.0, None),
    (120, 0.3, 0.0, None),
    (90, 1.4, 0.0, None),
    (60, 0.0, 0.0, None),
    (90, 0.0, 2.0, None),
    (30, 0.0, 0.0, None),
    (180, 0.0, 0.0, Some("Sit")),
    (150, 0.0, 0.0, Some(emote::GET_UP)),
    (180, 0.0, 0.0, Some("Lie Down")),
    (180, 0.0, 0.0, Some(emote::GET_UP)),
    (60, 0.3, 0.0, None),
];

/// How fast the drive over the terrain changes speed, in metres a second
/// squared: a caller's ramp. Stepped at once, a walk breaking into a trot swings
/// a paw past the guard on the level too, 1.25 times it on roll 6, and at three
/// the level's own worst comes within 3% of it; at this every body here stays
/// under 0.97 of it.
const RAMP: f32 = 2.0;

/// `speed` a frame on toward `pace` at [`RAMP`], setting off from standing by
/// half a frame's step more: the idle threshold of 0.3 m/s then falls between
/// two frames, at 0.283 and 0.317, rather than on one of them, where whether a
/// walk starts that frame or the next was the last bit of a sum.
fn speed_toward(speed: f32, pace: f32) -> f32 {
    let most = RAMP * STEP;
    let start = if speed == 0.0 && pace > 0.0 {
        0.5 * most
    } else {
        0.0
    };
    speed + start + (pace - speed - start).clamp(-most, most)
}

/// Where [`OVER_TERRAIN`] has `rig` when it first asks a gesture, along `+Z`.
fn where_it_stops(rig: &Rig) -> f32 {
    let (mut along, mut speed) = (0.0f32, 0.0f32);
    for (frames, froude, _, gesture) in OVER_TERRAIN {
        if gesture.is_some() {
            break;
        }
        let pace = if froude > 0.0 {
            Speed::from_froude(froude).metres_per_second(rig)
        } else {
            0.0
        };
        for _ in 0..frames {
            speed = speed_toward(speed, pace);
            along += speed * STEP;
        }
    }
    along
}

/// What [`over_terrain`] found: the furthest a joint moved in a frame, in shares of
/// what a frame allows, the frame and the joint; and the deepest a contact
/// joint stood into its ground while the body stood stopped there, and when
/// and which.
struct Swept {
    moved: (f32, usize, usize),
    sunk: f32,
    sunk_at: (usize, usize),
}

/// `rig`, `withers` high, driven through [`OVER_TERRAIN`] at sixty frames a
/// second over `terrain`, or over level ground, answering in the body's own
/// frame as the driver asks: `y` nought under its origin.
fn over_terrain(rig: &Rig, withers: f32, terrain: Option<Terrain>) -> Swept {
    let height = |x: f32, z: f32| terrain.map_or((0.0, Vec3::Y), |terrain| terrain.at(x, z));
    let most = crate::rest::swift(withers, STEP);
    let pads: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .map(|limb| rig.in_zone(Zone::Extremity(limb))[0])
        .collect();
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let (mut at, mut facing, mut speed) = (Vec3::ZERO, 0.0f32, 0.0f32);
    let mut before: Option<Vec<Vec3>> = None;
    let mut found = Swept {
        moved: (0.0, 0, 0),
        sunk: 0.0,
        sunk_at: (0, 0),
    };
    let mut frame = 0usize;
    for (stretch, (frames, froude, turn, gesture)) in OVER_TERRAIN.into_iter().enumerate() {
        let pace = if froude > 0.0 {
            Speed::from_froude(froude).metres_per_second(rig)
        } else {
            0.0
        };
        for index in 0..frames {
            facing += turn * STEP;
            speed = speed_toward(speed, pace);
            let (sin, cos) = sin_cos(facing);
            let velocity = Vec3::new(sin, 0.0, cos) * speed;
            at += velocity * STEP;
            let base = height(at.x, at.z).0;
            let ground = move |point: Vec3| {
                let world = into_world(at, facing, point);
                let (rise, up) = height(world.x, world.z);
                Some(symbios_avatar::anim::Ground {
                    position: Vec3::new(point.x, rise - base, point.z),
                    normal: Vec3::new(up.x * cos - up.z * sin, up.y, up.x * sin + up.z * cos),
                })
            };
            let inputs = Inputs {
                delta: STEP,
                velocity,
                at,
                facing,
                turn,
                gesture: gesture.filter(|_| index == 0),
                ..Inputs::default()
            };
            let posed = driver
                .drive(rig, &inputs, ground)
                .expect("an unheld body is posed")
                .pose
                .forward(rig);
            let now: Vec<Vec3> = posed
                .positions
                .iter()
                .map(|&point| into_world(at, facing, point) + Vec3::Y * base)
                .collect();
            if frame > 30
                && let Some(before) = &before
            {
                for joint in (0..now.len()).filter(|&joint| !rig.joints[joint].marker) {
                    let moved = now[joint].distance(before[joint]) / most;
                    if moved > found.moved.0 {
                        found.moved = (moved, frame, joint);
                    }
                }
            }
            // Stopped, as it steps its feet home: before it turns where it
            // stands, and after. A turn's own steps are a stride's, which the
            // guard on joints above answers for.
            if stretch == 3 || stretch == 5 {
                for &pad in &pads {
                    let stands = height(now[pad].x, now[pad].z).0 + rig.joints[pad].position.y;
                    if stands - now[pad].y > found.sunk {
                        found.sunk = stands - now[pad].y;
                        found.sunk_at = (frame, pad);
                    }
                }
            }
            before = Some(now);
            frame += 1;
        }
    }
    found
}

#[test]
fn a_beast_goes_about_its_business_over_rolling_ground_with_a_step_without_a_jump() {
    // #399, #400 and the step home found beside them: everything a beast does,
    // over ground answered from a heightmap - a different height and slope under
    // every foot, changing as it moves, slopes to .3 and a .15 step at a grid
    // line ahead of where it stops - against the guard a rest's joints are held
    // to, and its feet stepped home no further into the ground than on the
    // level. The step stands under its forefeet as it sits and lies.
    let mut bodies = Vec::new();
    for family in plan::Family::ALL {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        bodies.push((
            format!("{family:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    for roll in [1, 2] {
        bodies.push((
            format!("roll {roll}"),
            AvatarRecord::rolled(
                "Beast",
                Archetype::Quadruped(QuadrupedParams::default()),
                roll,
            ),
        ));
    }
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let runs: Vec<_> = bodies
            .into_iter()
            .map(|(what, mut record)| {
                scope.spawn(move || {
                    record.sanitize();
                    let Archetype::Quadruped(params) = &record.archetype else {
                        unreachable!("every record here is a beast")
                    };
                    let withers = params.height * 1.2;
                    let avatar = Avatar::build_with(
                        &record,
                        &AvatarConfig {
                            atlas: 64,
                            ..AvatarConfig::default()
                        },
                    )
                    .expect("a beast builds");
                    let rig = &avatar.rig;
                    let edge = where_it_stops(rig) + STEP_AHEAD * withers;
                    let level = over_terrain(rig, withers, None);
                    let terrain = Terrain {
                        bumps: 1.0,
                        step: 0.15,
                        ..Terrain::plane(0.0, 0.0)
                    };
                    let over = over_terrain(rig, withers, Some(Terrain { edge, ..terrain }));
                    let (moved, frame, joint) = over.moved;
                    let (sunk_frame, sunk_pad) = over.sunk_at;
                    println!(
                        "{what}: worst {moved:.2} of the guard, frame {frame} {:?}; feet \
                         {:.0} mm in stepping home, frame {sunk_frame} {:?}, against {:.0} on \
                         the level",
                        rig.joints[joint].zone,
                        over.sunk * 1000.0,
                        rig.joints[sunk_pad].zone,
                        level.sunk * 1000.0
                    );
                    let mut wrong = Vec::new();
                    if moved > 1.0 {
                        wrong.push(format!(
                            "{what}: joint {joint} ({:?}) moved {moved:.2} times what a frame \
                             allows, in frame {frame}",
                            rig.joints[joint].zone
                        ));
                    }
                    if over.sunk - level.sunk > STEPPED_INTO * withers {
                        wrong.push(format!(
                            "{what}: a foot stepping home stood {:.0} mm into the ground, \
                             {:.0} on the level",
                            over.sunk * 1000.0,
                            level.sunk * 1000.0
                        ));
                    }
                    wrong
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("a beast is driven"))
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_beast_goes_about_its_business_on_a_steep_slope_without_a_jump() {
    // #399: on a plane the trunk stood level and the stride sank the whole
    // body for its lowest paw, so up a .3 grade a horse's pelvis sank 380 mm
    // through each hind's stance and rose in a frame as the hind left the
    // ground, and its forelegs, crushed under it, lost their solve's root: a
    // forepaw 2.4 times what a frame allows, roll 6 2.7, roll 4 1.8. Each girdle
    // now rides its own ground. Everything a beast does, both ways up a .4
    // grade, across one and on a corner of both, for each family, a cross of
    // each and three rolls, against the guard a rest's joints are held to.
    // The walk breaking into a trot was sensitive to where in the stride it
    // fell, on level ground too, up to 1.31 times the guard for a horse at an
    // unlucky point of its stride (#412), and
    // `a_beast_breaking_into_a_trot_carries_every_foot_across` sweeps that;
    // this test's input is portable and its ramp off the idle threshold's
    // edge, so the trot comes at the same point however a machine's libm
    // rounds.
    let mut bodies = Vec::new();
    for family in plan::Family::ALL {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        bodies.push((
            format!("{family:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    for (family, other) in [
        (plan::Family::Dog, plan::Family::Horse),
        (plan::Family::Horse, plan::Family::Dog),
        (plan::Family::Cat, plan::Family::Dog),
    ] {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, Some(other), 0.5);
        bodies.push((
            format!("{family:?} crossed with {other:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    for roll in [1, 4, 6] {
        bodies.push((
            format!("roll {roll}"),
            AvatarRecord::rolled(
                "Beast",
                Archetype::Quadruped(QuadrupedParams::default()),
                roll,
            ),
        ));
    }
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let runs: Vec<_> = bodies
            .into_iter()
            .map(|(what, mut record)| {
                scope.spawn(move || {
                    record.sanitize();
                    let Archetype::Quadruped(params) = &record.archetype else {
                        unreachable!("every record here is a beast")
                    };
                    let withers = params.height * 1.2;
                    let avatar = Avatar::build_with(
                        &record,
                        &AvatarConfig {
                            atlas: 64,
                            ..AvatarConfig::default()
                        },
                    )
                    .expect("a beast builds");
                    let mut wrong = Vec::new();
                    let mut worst = (0.0f32, 0.0f32, 0.0f32);
                    for (grade, camber) in
                        [(0.4, 0.0), (-0.4, 0.0), (0.0, 0.4), (0.3, 0.2), (-0.3, 0.2)]
                    {
                        let over =
                            over_terrain(&avatar.rig, withers, Some(Terrain::plane(grade, camber)));
                        let (moved, frame, joint) = over.moved;
                        if moved > worst.0 {
                            worst = (moved, grade, camber);
                        }
                        if moved > 1.0 {
                            wrong.push(format!(
                                "{what} on {grade} by {camber}: joint {joint} ({:?}) moved \
                                 {moved:.2} times what a frame allows, in frame {frame}",
                                avatar.rig.joints[joint].zone
                            ));
                        }
                    }
                    println!(
                        "{what}: worst {:.2} of the guard, on {} by {}",
                        worst.0, worst.1, worst.2
                    );
                    wrong
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("a beast is driven"))
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Ground a hair off the level, as a consumer's flat terrain answers it: see
/// [`a_beast_on_nearly_level_ground_moves_as_it_does_on_the_level`].
#[derive(Clone, Copy, Debug)]
enum NearLevel {
    Level,
    /// Level, this high.
    Raised(f32),
    /// Rising this much per metre ahead.
    Grade(f32),
    /// A heightmap of this much noise, a quarter metre a cell.
    Noise(f32),
}

/// A hashed noise height in `-1..1` at a grid node.
fn noise_node(i: i32, j: i32) -> f32 {
    #[expect(clippy::cast_sign_loss, reason = "the bits are hashed, not counted")]
    let mut hash = (i as u32).wrapping_mul(0x9e37_79b1) ^ (j as u32).wrapping_mul(0x85eb_ca77);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x2c1b_3c6d);
    hash ^= hash >> 12;
    #[expect(clippy::cast_precision_loss, reason = "a noise value")]
    let unit = hash as f32 / u32::MAX as f32;
    unit * 2.0 - 1.0
}

impl NearLevel {
    /// How high the ground stands under `point` of a body at `at` facing
    /// `facing`, in the body's frame, and its up there.
    fn answer(self, at: Vec3, facing: f32, point: Vec3) -> symbios_avatar::anim::Ground {
        use symbios_avatar::anim::Ground;
        match self {
            Self::Level => Ground::level(Vec3::new(point.x, 0.0, point.z)),
            Self::Raised(by) => Ground::level(Vec3::new(point.x, by, point.z)),
            Self::Grade(rise) => Ground {
                position: Vec3::new(point.x, point.z * rise, point.z),
                normal: Vec3::new(0.0, 1.0, -rise).normalize(),
            },
            Self::Noise(size) => {
                let world = into_world(at, facing, point);
                let cell = 0.25;
                let (i, j) = ((world.x / cell).floor(), (world.z / cell).floor());
                let (u, v) = (world.x / cell - i, world.z / cell - j);
                #[expect(clippy::cast_possible_truncation, reason = "a grid's index")]
                let (i, j) = (i as i32, j as i32);
                let node = |di: i32, dj: i32| size * noise_node(i + di, j + dj);
                let (a, b, c, d) = (node(0, 0), node(1, 0), node(0, 1), node(1, 1));
                let height =
                    a * (1.0 - u) * (1.0 - v) + b * u * (1.0 - v) + c * (1.0 - u) * v + d * u * v;
                let across = ((b - a) * (1.0 - v) + (d - c) * v) / cell;
                let ahead = ((c - a) * (1.0 - u) + (d - b) * u) / cell;
                let up = Vec3::new(-across, 1.0, -ahead).normalize();
                let (sin, cos) = sin_cos(facing);
                Ground {
                    position: Vec3::new(point.x, height, point.z),
                    normal: Vec3::new(up.x * cos - up.z * sin, up.y, up.x * sin + up.z * cos),
                }
            }
        }
    }

    /// How high it stands everywhere, where it is level at a height.
    fn raised(self) -> f32 {
        match self {
            Self::Raised(by) => by,
            _ => 0.0,
        }
    }
}

/// Stand, walk, trot, stop, turn where it stands, stand, walk on: (frames,
/// Froude number or nought, turn in radians a second).
const NEAR_LEVEL_DRIVE: [(usize, f32, f32); 7] = [
    (30, 0.0, 0.0),
    (150, 0.3, 0.0),
    (120, 1.4, 0.0),
    (90, 0.0, 0.0),
    (90, 0.0, 2.0),
    (60, 0.0, 0.0),
    (90, 0.3, 0.0),
];

/// The frames [`NEAR_LEVEL_DRIVE`] switches its ground at: walking, trotting,
/// stopping and turning.
const NEAR_LEVEL_SWITCHES: [usize; 4] = [120, 260, 350, 440];

/// `rig` driven through [`NEAR_LEVEL_DRIVE`] on the ground `pick` gives each
/// frame, speed ramped by [`RAMP`]: every frame's joints in the body's frame,
/// and in the world.
fn near_level(rig: &Rig, pick: impl Fn(usize) -> NearLevel) -> (Vec<Vec<Vec3>>, Vec<Vec<Vec3>>) {
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let (mut at, mut facing, mut speed) = (Vec3::ZERO, 0.0f32, 0.0f32);
    let (mut own, mut world) = (Vec::new(), Vec::new());
    for (frames, froude, turn) in NEAR_LEVEL_DRIVE {
        let pace = if froude > 0.0 {
            Speed::from_froude(froude).metres_per_second(rig)
        } else {
            0.0
        };
        for _ in 0..frames {
            facing += turn * STEP;
            speed = speed_toward(speed, pace);
            let (sin, cos) = sin_cos(facing);
            let velocity = Vec3::new(sin, 0.0, cos) * speed;
            at += velocity * STEP;
            let ground = pick(own.len());
            let inputs = Inputs {
                delta: STEP,
                velocity,
                at,
                facing,
                turn,
                ..Inputs::default()
            };
            let posed = driver
                .drive(rig, &inputs, move |point| {
                    Some(ground.answer(at, facing, point))
                })
                .expect("an unheld body is posed")
                .pose
                .forward(rig);
            world.push(
                posed
                    .positions
                    .iter()
                    .map(|&point| into_world(at, facing, point))
                    .collect(),
            );
            own.push(posed.positions);
        }
    }
    (own, world)
}

/// The largest second difference of any joint of `rig` over `frames` of
/// `world`, in metres.
fn jolt(rig: &Rig, world: &[Vec<Vec3>], frames: std::ops::Range<usize>) -> f32 {
    let mut most = 0.0f32;
    for frame in frames.start.max(1)..frames.end.min(world.len() - 1) {
        for joint in (0..rig.len()).filter(|&joint| !rig.joints[joint].marker) {
            let second =
                world[frame + 1][joint] - world[frame][joint] * 2.0 + world[frame - 1][joint];
            most = most.max(second.length());
        }
    }
    most
}

#[test]
fn a_beast_on_nearly_level_ground_moves_as_it_does_on_the_level() {
    // #399's girdles, against what a consumer's flat ground really answers: a
    // heightmap is never exactly level. Splitting the crouch per girdle and
    // solving a stepping foot again after the plant took effect at the least
    // departure from the level, so on a grade of 1e-4, or a millimetre of noise,
    // a horse's tail stood 110 mm off where level ground holds it at every phase
    // of a stride, a dog's head 44 mm at its stop, and going between the two
    // mid-stride jolted a horse 71 mm past either run alone. The slope's own
    // answers are taken in by how steep the ground is, and on ground a hair off
    // the level a body moves as it does on the level, within what HEAD's own
    // floats make of it, and goes between the two without a jolt of its own.
    // Ground raised level by 2 cm stands the body 2 cm up.
    let mut bodies = Vec::new();
    for family in [plan::Family::Dog, plan::Family::Horse] {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        bodies.push((
            format!("{family:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    bodies.push((
        "roll 1".to_string(),
        AvatarRecord::rolled("Beast", Archetype::Quadruped(QuadrupedParams::default()), 1),
    ));
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let runs: Vec<_> = bodies
            .into_iter()
            .map(|(what, mut record)| {
                scope.spawn(move || {
                    record.sanitize();
                    let avatar = Avatar::build_with(
                        &record,
                        &AvatarConfig {
                            atlas: 64,
                            ..AvatarConfig::default()
                        },
                    )
                    .expect("a beast builds");
                    let rig = &avatar.rig;
                    let (level, level_world) = near_level(rig, |_| NearLevel::Level);
                    let mut wrong = Vec::new();
                    for ground in [
                        NearLevel::Grade(1.0e-4),
                        NearLevel::Noise(1.0e-3),
                        NearLevel::Raised(0.02),
                        NearLevel::Raised(-0.02),
                    ] {
                        let (off, off_world) = near_level(rig, |_| ground);
                        let raised = ground.raised();
                        let (most, jolted) = if raised == 0.0 {
                            (NEAR_LEVEL_OFF, NEAR_LEVEL_JOLT)
                        } else {
                            (RAISED_OFF, RAISED_JOLT)
                        };
                        let apart = level
                            .iter()
                            .zip(&off)
                            .flat_map(|(level, off)| {
                                level
                                    .iter()
                                    .zip(off)
                                    .map(|(level, off)| off.distance(*level + Vec3::Y * raised))
                            })
                            .fold(0.0f32, f32::max);
                        if apart > most {
                            wrong.push(format!(
                                "{what} on {ground:?}: a joint stood {:.1} mm off where level \
                                 ground stands it, past {:.1}",
                                apart * 1000.0,
                                most * 1000.0
                            ));
                        }
                        // On and off it at each switch: the jolt beyond what
                        // either ground's own run reads there.
                        let (_, switched) = near_level(rig, |frame| {
                            let [a, b, c, d] = NEAR_LEVEL_SWITCHES;
                            if (a..b).contains(&frame) || (c..d).contains(&frame) {
                                ground
                            } else {
                                NearLevel::Level
                            }
                        });
                        for at in NEAR_LEVEL_SWITCHES {
                            let around = at - 2..at + 2;
                            let own = jolt(rig, &level_world, around.clone()).max(jolt(
                                rig,
                                &off_world,
                                around.clone(),
                            ));
                            let extra = jolt(rig, &switched, around) - own;
                            if extra > jolted {
                                wrong.push(format!(
                                    "{what} going onto or off {ground:?} at frame {at}: a joint \
                                     jolted {:.1} mm past either ground's own, past {:.1}",
                                    extra * 1000.0,
                                    jolted * 1000.0
                                ));
                            }
                        }
                    }
                    wrong
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("a beast is driven"))
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Standing, sitting, getting up, lying and getting up again, where it stands:
/// (seconds, gesture on the first frame).
const NEAR_LEVEL_RESTS: [(f32, Option<&str>); 5] = [
    (1.0, None),
    (3.0, Some("Sit")),
    (2.5, Some(emote::GET_UP)),
    (3.0, Some("Lie Down")),
    (3.0, Some(emote::GET_UP)),
];

#[test]
fn a_beast_rests_on_nearly_level_ground_as_it_does_on_the_level() {
    // #413: a rest laid on ground a hair off the level re-solved its folds for
    // the millimetres the ground was off, and its answers came out tens of
    // millimetres off what the level holds: a horse sitting on a millimetre of
    // noise sat 59 mm off it and moved 32 mm in a frame more than the level's
    // sit, a rolled body getting up 39 and 36. Ground near the level is now
    // laid as the level is, raised to it, by as much as it is near: every joint
    // within a few millimetres of where the level stands it, raised with the
    // ground, and moving in a frame within as much again of what it moves on
    // the level.
    let mut wrong = Vec::new();
    for (what, avatar) in slope_beasts() {
        let rig = &avatar.rig;
        let rested = |ground: NearLevel| {
            let mut driver = Driver::seeded(INSTRUMENT_SEED);
            let mut posed = Vec::new();
            for (seconds, gesture) in NEAR_LEVEL_RESTS {
                #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
                #[expect(clippy::cast_sign_loss, reason = "a positive time")]
                let frames = (seconds / STEP).round() as usize;
                for index in 0..frames {
                    let inputs = Inputs {
                        delta: STEP,
                        gesture: gesture.filter(|_| index == 0),
                        ..Inputs::default()
                    };
                    let pose = driver
                        .drive(rig, &inputs, move |point| {
                            Some(ground.answer(Vec3::ZERO, 0.0, point))
                        })
                        .expect("an unheld body is posed")
                        .pose;
                    posed.push(pose.forward(rig).positions);
                }
            }
            posed
        };
        let level = rested(NearLevel::Level);
        for ground in [
            NearLevel::Grade(1.0e-4),
            NearLevel::Noise(1.0e-3),
            NearLevel::Raised(0.02),
            NearLevel::Raised(-0.02),
        ] {
            let raised = Vec3::Y * ground.raised();
            let off: Vec<Vec<Vec3>> = rested(ground)
                .iter()
                .zip(&level)
                .map(|(on, flat)| {
                    on.iter()
                        .zip(flat)
                        .map(|(on, flat)| *on - *flat - raised)
                        .collect()
                })
                .collect();
            let apart = off
                .iter()
                .flatten()
                .map(|by| by.length())
                .fold(0.0f32, f32::max);
            let popped = off
                .windows(2)
                .flat_map(|pair| {
                    pair[0]
                        .iter()
                        .zip(&pair[1])
                        .map(|(was, now)| was.distance(*now))
                })
                .fold(0.0f32, f32::max);
            if apart > REST_NEAR_OFF || popped > REST_NEAR_POP {
                wrong.push(format!(
                    "{what} on {ground:?}: a joint rested {:.1} mm off where level ground \
                     rests it, past {:.1}, and moved {:.1} mm in a frame more than there, past \
                     {:.1}",
                    apart * 1000.0,
                    REST_NEAR_OFF * 1000.0,
                    popped * 1000.0,
                    REST_NEAR_POP * 1000.0
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far any joint of a beast resting on ground a hair off the level, or the
/// level raised 2 cm, may stand from where the level stands it, and how much
/// more than there it may move in a frame, in metres. Provenance: **measured**
/// (#413): at most 4.8 mm off, a rolled body on the raised level as it comes
/// up from lying onto the idle, which stands it there off where the level does
/// (see [`RAISED_OFF`]), as HEAD's 3.9 for a dog; and 1.5 mm more in a frame, a
/// dog on the noise. A horse sat 59 mm off on the noise and moved 32 more.
const REST_NEAR_OFF: f32 = 0.006;
const REST_NEAR_POP: f32 = 0.003;

/// A plane rising `grade` per metre ahead of a body standing at the origin
/// facing `+Z`, or [`level_ground`] itself at none.
fn graded(grade: f32) -> impl Fn(Vec3) -> Option<symbios_avatar::anim::Ground> + Copy {
    move |point: Vec3| {
        if grade == 0.0 {
            return level_ground(point);
        }
        Some(symbios_avatar::anim::Ground {
            position: Vec3::new(point.x, grade * point.z, point.z),
            normal: Vec3::new(0.0, 1.0, -grade).normalize(),
        })
    }
}

/// How high each pad of `rig` stands over a plane rising `grade` per metre
/// ahead, against how high it stands over the level, in metres: after eleven
/// seconds standing where it is, `gesture` asked a second in, with the idle or
/// without it.
fn pads_over(rig: &Rig, gesture: Option<&str>, grade: f32, idle: bool) -> Vec<(Limb, f32)> {
    let stood = |grade: f32| {
        let mut driver = Driver::new(
            DriverConfig {
                idle,
                ..DriverConfig::default()
            },
            INSTRUMENT_SEED,
        );
        let mut posed = Vec::new();
        for frame in 0..660 {
            let inputs = Inputs {
                delta: STEP,
                gesture: gesture.filter(|_| frame == 60),
                ..Inputs::default()
            };
            posed = driver
                .drive(rig, &inputs, graded(grade))
                .expect("an unheld body is posed")
                .pose
                .forward(rig)
                .positions;
        }
        posed
    };
    let (level, sloped) = (stood(0.0), stood(grade));
    rig.ground_contacts()
        .into_iter()
        .map(|limb| {
            let pad = rig.in_zone(Zone::Extremity(limb))[0];
            (limb, sloped[pad].y - grade * sloped[pad].z - level[pad].y)
        })
        .collect()
}

#[test]
fn a_beast_at_rest_on_a_gentle_slope_keeps_its_feet_on_it() {
    // #413: laying a rest near the level as the level is, raised to the plane,
    // was asked of any plane rising under a tenth, about six degrees, and laid
    // level the feet of a body sitting on one went through it and stood off it:
    // a horse sitting on a grade of .04 had its hind soles 24.5 mm into it and
    // its fore 11.2 off, against a tenth of a millimetre in HEAD. Only ground
    // truly near the level is laid as the level is now. And a beast with no
    // idle stood its rest pose's feet turned with its ridden trunk, a dog's 120
    // mm into a 25 degree slope: they stand on it now, as the idle's do.
    let mut wrong = Vec::new();
    for (what, avatar) in slope_beasts() {
        let rig = &avatar.rig;
        let mut asked = Vec::new();
        for gesture in ["Sit", "Lie Down"] {
            for grade in [0.01, 0.02, 0.04, 0.06, 0.1] {
                asked.push((gesture, grade, true));
            }
        }
        for grade in [0.1, GRADE_25] {
            asked.push(("standing with no idle", grade, false));
        }
        for (rest, grade, idle) in asked {
            let gesture = Some(rest).filter(|_| idle);
            for (limb, off) in pads_over(rig, gesture, grade, idle) {
                if off.abs() > GENTLE_SLOPE_OFF {
                    wrong.push(format!(
                        "{what} {rest} on a grade of {grade}: its {limb:?} pad stood {:.1} mm \
                         off the slope against the level, past {:.1}",
                        off * 1000.0,
                        GENTLE_SLOPE_OFF * 1000.0
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far a pad of a beast at rest on a slope may stand off it, against how
/// far it stands off the level, in metres. Provenance: **measured** (#413):
/// HEAD's own, a horse lying on a grade of .1, 0.46 mm.
const GENTLE_SLOPE_OFF: f32 = 0.001;

#[test]
fn what_a_body_stood_on_goes_no_further_than_the_body() {
    // #413: the ground a four-legged trunk last rode and stood on is kept on
    // its driver, and was kept when the driver went on to another body: a
    // dog's carried onto a person shifted the person's ground 138 mm in its
    // first frame, 348 if it leapt, and a horse's carried onto a dog in the air
    // flew the dog on the horse's heights, 20 cm low. A driver goes on from the
    // ground a body stood on now exactly as from the level: what a person or
    // another beast does next is the same to the bit.
    let bodies = slope_beasts();
    let (dog, horse) = (&bodies[0].1.rig, &bodies[1].1.rig);
    let mut person = AvatarRecord::rolled("Person", Archetype::default(), 0);
    person.sanitize();
    let person = Avatar::build_with(
        &person,
        &AvatarConfig {
            atlas: 64,
            ..AvatarConfig::default()
        },
    )
    .expect("a person builds");
    let leaping = |frame: usize, from: usize| {
        Vec3::Y
            * if frame >= from && frame < from + 36 {
                3.0 - 9.81 * STEP * (frame - from) as f32
            } else {
                0.0
            }
    };
    let mut wrong = Vec::new();
    for (what, first, then, leap_at, swap) in [
        (
            "a dog's driver standing, then a person's",
            dog,
            &person.rig,
            None,
            120,
        ),
        (
            "a dog's driver leaping, then a person's",
            dog,
            &person.rig,
            Some(100),
            120,
        ),
        (
            "a horse's driver leaping, then a dog's in the air",
            horse,
            dog,
            Some(60),
            70,
        ),
    ] {
        let driven = |grade: f32| {
            let mut driver = Driver::seeded(INSTRUMENT_SEED);
            let mut posed = Vec::new();
            for frame in 0..swap + 120 {
                let rig = if frame < swap { first } else { then };
                let inputs = Inputs {
                    delta: STEP,
                    velocity: leap_at.map_or(Vec3::ZERO, |from| leaping(frame, from)),
                    ..Inputs::default()
                };
                let ground = if frame < swap {
                    graded(grade)
                } else {
                    graded(0.0)
                };
                let pose = driver
                    .drive(rig, &inputs, ground)
                    .expect("an unheld body is posed")
                    .pose;
                if frame >= swap {
                    posed.push(pose);
                }
            }
            posed
        };
        if driven(GRADE_25) != driven(0.0) {
            wrong.push(format!(
                "{what}: the second body moved by what the first stood on"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_beast_warped_in_the_air_lands_as_it_would_have() {
    // #413: a warp forgot what the trunk rode, and in the air the flight went
    // on riding what it left while the touchdown took the ground it came down
    // on at once: a dog warped a quarter second into a leap off a 25 degree
    // slope onto the level turned 25 degrees in the frame it landed. A warp in
    // the air now leaves the landing as it would have been.
    let mut wrong = Vec::new();
    for (what, avatar) in slope_beasts() {
        let rig = &avatar.rig;
        for (from, to) in [(GRADE_25, 0.0), (0.0, GRADE_25)] {
            let turned = |warp: bool| {
                let mut driver = Driver::seeded(INSTRUMENT_SEED);
                let mut before: Option<(f32, f32)> = None;
                let mut most = 0.0f32;
                for frame in 0..240usize {
                    if warp && frame == 75 {
                        driver.warped();
                    }
                    let rising = if (60..96).contains(&frame) {
                        3.0 - 9.81 * STEP * (frame - 60) as f32
                    } else {
                        0.0
                    };
                    let inputs = Inputs {
                        delta: STEP,
                        velocity: Vec3::Y * rising,
                        ..Inputs::default()
                    };
                    let ground = graded(if frame < 75 { from } else { to });
                    let posed = driver
                        .drive(rig, &inputs, ground)
                        .expect("an unheld body is posed")
                        .pose
                        .forward(rig);
                    let now = trunk_turn(rig, &posed.positions);
                    if let Some((pitch, roll)) = before {
                        most = most.max((now.0 - pitch).abs()).max((now.1 - roll).abs());
                    }
                    before = Some(now);
                }
                most
            };
            let (warped, unwarped) = (turned(true), turned(false));
            if warped > unwarped + 0.1 {
                wrong.push(format!(
                    "{what} warped in the air from a grade of {from} to {to}: the trunk turned \
                     {warped:.1} degrees in a frame, against {unwarped:.1} unwarped"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far any joint of a beast on ground a hair off the level may stand from
/// where level ground stands it, and how much further going between the two
/// may jolt it than either does alone, in metres; and the same for ground
/// raised level by 2 cm, where the jolt is the body rising with the ground.
/// Provenance: **measured** (#399). A hair off the level, HEAD's own floats read
/// up to 2.7 mm and jolt nothing; this tree 2.8 and nothing, against 110 and
/// 71 before the slope's answers were taken in by its steepness. Raised 2 cm,
/// HEAD stood its joints up to 33 mm off and jolted 20; this tree 21 and 20,
/// against 110 and 91.
const NEAR_LEVEL_OFF: f32 = 0.004;
const NEAR_LEVEL_JOLT: f32 = 0.005;
const RAISED_OFF: f32 = 0.03;
const RAISED_JOLT: f32 = 0.025;

#[test]
fn a_driver_carried_onto_a_rebuilt_body_mid_rest_does_not_jump() {
    // #400's glide keeps what the last frame of a rest was laid on, and a
    // driver carried onto another body went on from the first one's: a horse's
    // driver moved onto a cat half a second into getting up, on 4 cm of noise,
    // jumped the cat 2.36 times what a frame allows, onto a dog 2.12, onto a cat
    // starting to lie down on a -.3 grade 1.21. The body a rest is laid for is rebuilt and
    // what it was laid on forgotten together. From the frame after the swap,
    // against what the new body allows.
    let body = |family| {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
        record.sanitize();
        let Archetype::Quadruped(params) = &record.archetype else {
            unreachable!("a beast")
        };
        let withers = params.height * 1.2;
        let avatar = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("a beast builds");
        (avatar, withers)
    };
    let [dog, cat, horse] = [plan::Family::Dog, plan::Family::Cat, plan::Family::Horse].map(body);
    // A walk, a stop, down into a sit, up, down into lying and up: (frames,
    // Froude number or nought, gesture).
    let script = [
        (30, 0.0, None),
        (90, 0.3, None),
        (60, 0.0, None),
        (180, 0.0, Some("Sit")),
        (150, 0.0, Some(emote::GET_UP)),
        (180, 0.0, Some("Lie Down")),
        (180, 0.0, Some(emote::GET_UP)),
    ];
    // Half a second into getting up from the sit, and two and ten frames into
    // lying down.
    let swaps = [392, 400, 512, 520];
    let mut wrong = Vec::new();
    for (what, from, onto) in [
        ("horse onto cat", &horse, &cat),
        ("horse onto dog", &horse, &dog),
        ("dog onto cat", &dog, &cat),
    ] {
        for ground in [NearLevel::Noise(0.04), NearLevel::Grade(-0.3)] {
            for swap in swaps {
                let mut driver = Driver::seeded(INSTRUMENT_SEED);
                let mut before: Option<Vec<Vec3>> = None;
                let (mut frame, mut at) = (0usize, Vec3::ZERO);
                let mut worst = (0.0f32, 0usize);
                for (frames, froude, gesture) in script {
                    let pace = if froude > 0.0 {
                        Speed::from_froude(froude).metres_per_second(&from.0.rig)
                    } else {
                        0.0
                    };
                    for index in 0..frames {
                        let (avatar, withers) = if frame >= swap { onto } else { from };
                        let rig = &avatar.rig;
                        if frame == swap {
                            driver = driver.clone();
                            before = None;
                        }
                        let velocity = Vec3::Z * pace;
                        at += velocity * STEP;
                        let inputs = Inputs {
                            delta: STEP,
                            velocity,
                            at,
                            gesture: gesture.filter(|_| index == 0),
                            ..Inputs::default()
                        };
                        let here = at;
                        let posed = driver
                            .drive(rig, &inputs, move |point| {
                                Some(ground.answer(here, 0.0, point))
                            })
                            .expect("an unheld body is posed")
                            .pose
                            .forward(rig)
                            .positions
                            .iter()
                            .map(|&point| point + at)
                            .collect::<Vec<_>>();
                        if let Some(before) = &before
                            && frame > swap
                        {
                            let most = crate::rest::swift(*withers, STEP);
                            for joint in (0..rig.len()).filter(|&joint| !rig.joints[joint].marker) {
                                let moved = posed[joint].distance(before[joint]) / most;
                                if moved > worst.0 {
                                    worst = (moved, frame);
                                }
                            }
                        }
                        before = Some(posed);
                        frame += 1;
                    }
                }
                if worst.0 > 1.0 {
                    wrong.push(format!(
                        "{what} on {ground:?} from frame {swap}: a joint moved {:.2} times \
                         what a frame allows, in frame {}",
                        worst.0, worst.1
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The pitch and the roll of `rig`'s trunk posed at `at`, in degrees: the line
/// from the middle of its hips to the middle of its shoulders against the
/// level, and the line across both girdles from right to left.
fn trunk_turn(rig: &Rig, at: &[Vec3]) -> (f32, f32) {
    let top = |limb: Limb| {
        rig.limb_chain(limb)
            .map_or(Vec3::ZERO, |chain| at[chain[0]])
    };
    let hips = (top(Limb::HindLeft) + top(Limb::HindRight)) * 0.5;
    let shoulders = (top(Limb::ForeLeft) + top(Limb::ForeRight)) * 0.5;
    let run = shoulders - hips;
    let across =
        top(Limb::HindLeft) - top(Limb::HindRight) + top(Limb::ForeLeft) - top(Limb::ForeRight);
    let lean = |v: Vec3| libm::atan2f(v.y, (v.x * v.x + v.z * v.z).sqrt()).to_degrees();
    (lean(run), lean(across))
}

/// One stretch of [`trunk_through_states`]: (seconds, Froude number or nought,
/// turn in radians a second, gesture on its first frame, upward speed on its
/// first frame, in the water).
type State = (f32, f32, f32, Option<&'static str>, f32, bool);

/// Standing, a leap and its landing and into the water and out, all facing up
/// the slope, then turning where it stands both ways, walking off, curving and
/// straightening, stopping, and sitting and getting up.
const THROUGH_STATES: [(&str, State); 14] = [
    ("standing", (1.0, 0.0, 0.0, None, 0.0, false)),
    ("leaping", (0.7, 0.0, 0.0, None, 3.0, false)),
    ("landed", (1.0, 0.0, 0.0, None, 0.0, false)),
    ("swimming", (2.0, 0.0, 0.0, None, 0.0, true)),
    ("out of the water", (1.5, 0.0, 0.0, None, 0.0, false)),
    ("turning", (1.5, 0.0, 1.5, None, 0.0, false)),
    ("standing again", (1.0, 0.0, 0.0, None, 0.0, false)),
    ("turning back", (1.0, 0.0, -1.5, None, 0.0, false)),
    ("walking off", (1.5, 0.3, 0.0, None, 0.0, false)),
    ("curving", (1.0, 0.3, 0.8, None, 0.0, false)),
    ("straightening", (1.0, 0.3, 0.0, None, 0.0, false)),
    ("stopping", (1.5, 0.0, 0.0, None, 0.0, false)),
    ("sitting", (3.0, 0.0, 0.0, Some("Sit"), 0.0, false)),
    (
        "getting up",
        (2.5, 0.0, 0.0, Some(emote::GET_UP), 0.0, false),
    ),
];

/// The most `rig`'s trunk turns in a frame in each of [`THROUGH_STATES`], pitch
/// or roll, in degrees, at sixty frames a second on a plane rising `grade` per
/// metre toward the world's `+Z`, whichever way the body faces on it.
fn trunk_through_states(rig: &Rig, grade: f32, idle: bool) -> Vec<(&'static str, f32)> {
    trunk_over(rig, &THROUGH_STATES, &|_, _| grade, false, idle)
        .into_iter()
        .map(|state| (state.name, state.turned))
        .collect()
}

/// The frames of `states` that `name` takes, at sixty frames a second.
fn frames_in(states: &[(&'static str, State)], name: &str) -> std::ops::Range<usize> {
    let mut from = 0;
    for &(state, (seconds, ..)) in states {
        #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
        #[expect(clippy::cast_sign_loss, reason = "a positive time")]
        let frames = (seconds / STEP).round() as usize;
        if state == name {
            return from..from + frames;
        }
        from += frames;
    }
    from..from
}

/// What [`trunk_over`] finds a trunk did through one of its states.
struct Through {
    name: &'static str,
    /// The most it turned in a frame, pitch or roll, in degrees.
    turned: f32,
    /// Every pose it was drawn in, and how far a chassis carried it up.
    poses: Vec<(Pose, f32)>,
}

/// `rig` driven through `states` at sixty frames a second over a plane rising
/// `grade(frame, height)` per metre toward the world's `+Z`, whichever way the
/// body faces on it: `0` is [`level_ground`] itself, and NaN nothing at all. `height` is how far a
/// chassis has carried the body up off the plane in a leap; `carried`, the
/// plane is answered that far below the body, as a consumer whose chassis
/// carries the body through the air answers what is under it. Without `idle`,
/// a body that stops stands in its rest pose, [`DriverConfig::idle`] off.
fn trunk_over(
    rig: &Rig,
    states: &[(&'static str, State)],
    grade: &dyn Fn(usize, f32) -> f32,
    carried: bool,
    idle: bool,
) -> Vec<Through> {
    let mut driver = Driver::new(
        DriverConfig {
            idle,
            ..DriverConfig::default()
        },
        INSTRUMENT_SEED,
    );
    let (mut at, mut facing, mut speed, mut rising) = (Vec3::ZERO, 0.0f32, 0.0f32, 0.0f32);
    let mut height = 0.0f32;
    let mut before: Option<(f32, f32)> = None;
    let mut frame = 0usize;
    let mut found = Vec::new();
    for &(name, (seconds, froude, turn, gesture, lift, swimming)) in states {
        let pace = if froude > 0.0 {
            Speed::from_froude(froude).metres_per_second(rig)
        } else {
            0.0
        };
        let mut through = Through {
            name,
            turned: 0.0,
            poses: Vec::new(),
        };
        #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
        #[expect(clippy::cast_sign_loss, reason = "a positive time")]
        let frames = (seconds / STEP).round() as usize;
        for index in 0..frames {
            facing += turn * STEP;
            speed = speed_toward(speed, pace);
            if index == 0 {
                rising = lift;
            }
            if lift > 0.0 && carried {
                height = (height + rising * STEP).max(0.0);
            }
            let (sin, cos) = sin_cos(facing);
            let velocity = Vec3::new(sin * speed, rising, cos * speed);
            if lift > 0.0 {
                rising -= 9.81 * STEP;
                if carried && height <= 0.0 && rising < 0.0 {
                    rising = 0.0;
                }
            }
            at += Vec3::new(velocity.x, 0.0, velocity.z) * STEP;
            let here = at;
            let below = if carried { height } else { 0.0 };
            let grade = grade(frame, height);
            let ground = move |point: Vec3| {
                if grade.is_nan() {
                    return None;
                }
                if grade == 0.0 && below == 0.0 {
                    return level_ground(point);
                }
                let world = into_world(here, facing, point);
                Some(symbios_avatar::anim::Ground {
                    position: Vec3::new(point.x, (world.z - here.z) * grade - below, point.z),
                    normal: Vec3::new(grade * sin, 1.0, -grade * cos).normalize(),
                })
            };
            let inputs = Inputs {
                delta: STEP,
                velocity,
                at: here + Vec3::Y * below,
                facing,
                turn,
                swimming,
                gesture: gesture.filter(|_| index == 0),
                ..Inputs::default()
            };
            let pose = driver
                .drive(rig, &inputs, ground)
                .expect("an unheld body is posed")
                .pose;
            let now = trunk_turn(rig, &pose.forward(rig).positions);
            if let Some((pitch, roll)) = before {
                through.turned = through
                    .turned
                    .max((now.0 - pitch).abs())
                    .max((now.1 - roll).abs());
            }
            before = Some(now);
            through.poses.push((pose, below));
            frame += 1;
        }
        found.push(through);
    }
    found
}

#[test]
fn a_beasts_trunk_goes_from_state_to_state_on_a_slope_without_snapping_level() {
    // #413: a beast on a slope rode its girdles walking, trotting and standing,
    // and in every other state stood its trunk level, so going into one and out
    // of it turned the trunk by the whole slope over a blend or in a frame: on
    // a 25 degree slope a horse turning where it stands pitched 6.9 degrees in a
    // frame as a paw landed, a dog sitting down or getting up rolled 4.5, a
    // leap's gathering turned it 6.9 flat and going into the water 6.4. Every
    // state now rides the girdles, the rest from its start and the leap through
    // its flight, and the water is gone into over a longer blend. Each state
    // turns the trunk in a frame no more than `TRUNK_TURN_OVER` past what it
    // turns it on the level, and into and out of the water no more than
    // `WATER_TURN_OVER` past. The landing of a fall is not here: it pitches a
    // trunk 21 degrees in a frame on the level in HEAD (#414).
    // With the idle and without it, where a body stopped stands in its rest
    // pose: stood level there, a sit set off from the ridden trunk turned it
    // 5.4 degrees in a frame.
    let mut wrong = Vec::new();
    for ((what, avatar), idle) in slope_beasts()
        .iter()
        .flat_map(|beast| [(beast, true), (beast, false)])
    {
        let what = if idle {
            what.to_string()
        } else {
            format!("{what} with no idle")
        };
        let level = trunk_through_states(&avatar.rig, 0.0, idle);
        for ((name, most), (_, flat)) in trunk_through_states(&avatar.rig, GRADE_25, idle)
            .into_iter()
            .zip(level)
        {
            // Into the water and out the trunk turns from the slope to
            // floating level: no faster than it turns from standing to
            // floating on the level.
            let over = if name.contains("water") || name == "swimming" {
                WATER_TURN_OVER
            } else {
                TRUNK_TURN_OVER
            };
            if most > flat + over {
                wrong.push(format!(
                    "{what} {name}: the trunk turned {most:.1} degrees in a frame, past \
                     {flat:.1} on the level and {over:.2} more"
                ));
            }
        }
        // And as Overlands answers a swimmer, nothing under it in the water:
        // the longer blend is asked of the ground last stood on, which with
        // nothing read in the frame the body went in it never was.
        if idle {
            let swimming = frames_in(&THROUGH_STATES, "swimming");
            let dropped = trunk_over(
                &avatar.rig,
                &THROUGH_STATES,
                &|frame, _| {
                    if swimming.contains(&frame) {
                        f32::NAN
                    } else {
                        GRADE_25
                    }
                },
                false,
                true,
            );
            let flat = trunk_through_states(&avatar.rig, 0.0, true);
            for (state, (name, flat)) in dropped.iter().zip(flat) {
                if (name.contains("water") || name == "swimming")
                    && state.turned > flat + WATER_TURN_OVER
                {
                    wrong.push(format!(
                        "{what} {name} with nothing under it in the water: the trunk turned \
                         {:.1} degrees in a frame, past {flat:.1} on the level and \
                         {WATER_TURN_OVER:.2} more",
                        state.turned
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How much more than on the level a beast's trunk may pitch or roll in a
/// frame at sixty frames a second in each of [`THROUGH_STATES`] on a 25 degree
/// slope, in degrees. Provenance: **measured** (#413): the most is 1.5 more, a
/// dog walking off down the slope and a horse turning back where it stands; a
/// sit was 3.2 more and a leap's gathering 3.8 when they stood the trunk level.
const TRUNK_TURN_OVER: f32 = 2.0;

/// How much faster than on the level a beast's trunk may turn in a frame going
/// into the water on a slope or out of it, in degrees. Provenance: **measured**
/// (#413): on the level it turns 2.6 to 2.7 a frame; up a 25 degree slope it
/// turned 3.1 to 3.2 over the one blend, and over the longer one 1.0 to 1.1.
const WATER_TURN_OVER: f32 = 0.25;

/// The beasts the slope sweeps below are driven as: a dog, a horse and a rolled
/// body.
fn slope_beasts() -> Vec<(&'static str, Avatar)> {
    [
        ("dog", {
            let mut params = QuadrupedParams::default();
            params.set_kind(plan::Family::Dog, None, 0.0);
            AvatarRecord::new("Beast", Archetype::Quadruped(params))
        }),
        ("horse", {
            let mut params = QuadrupedParams::default();
            params.set_kind(plan::Family::Horse, None, 0.0);
            AvatarRecord::new("Beast", Archetype::Quadruped(params))
        }),
        (
            "roll 1",
            AvatarRecord::rolled("Beast", Archetype::Quadruped(QuadrupedParams::default()), 1),
        ),
    ]
    .into_iter()
    .map(|(what, mut record)| {
        record.sanitize();
        let avatar = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..AvatarConfig::default()
            },
        )
        .expect("a beast builds");
        (what, avatar)
    })
    .collect()
}

/// Twenty-five degrees, as a grade.
const GRADE_25: f32 = 0.466_307_66;

/// Standing, walking off, stopping and turning where it stands, through which
/// [`a_beasts_trunk_follows_its_ground_switching_source_over_a_few_frames`]
/// switches the ground.
const SWITCHED_STATES: [(&str, State); 4] = [
    ("standing", (1.5, 0.0, 0.0, None, 0.0, false)),
    ("walking", (2.0, 0.3, 0.0, None, 0.0, false)),
    ("stopping", (1.5, 0.0, 0.0, None, 0.0, false)),
    ("turning", (2.0, 0.0, 1.5, None, 0.0, false)),
];

#[test]
fn a_beasts_trunk_follows_its_ground_switching_source_over_a_few_frames() {
    // #413: a consumer's ground can change source in a frame - Overlands goes
    // between its terrain and a level floor at the edge of a pad or a deck, on
    // a lost physics step and at a landing - and every such switch on a slope
    // turned a beast's trunk by the whole slope in that frame. The trunk now
    // follows the ground under its girdles no faster than a few degrees a
    // frame. Here the ground goes from a 25 degree plane to the level and back,
    // each in one frame, in the middle of standing, walking and turning: the
    // trunk turns in a frame no more than the same states turn it on the
    // unbroken slope, by more than the follow allows.
    let mut wrong = Vec::new();
    // Level from half a second into standing, and six tenths of a second into
    // walking and turning, for half a second or six tenths.
    let switched = |frame: usize, _: f32| {
        let level = [(30, 60), (126, 162), (336, 372)]
            .iter()
            .any(|&(from, to)| (from..to).contains(&frame));
        if level { 0.0 } else { GRADE_25 }
    };
    for (what, avatar) in slope_beasts() {
        let unbroken = trunk_over(&avatar.rig, &SWITCHED_STATES, &|_, _| GRADE_25, false, true);
        let broken = trunk_over(&avatar.rig, &SWITCHED_STATES, &switched, false, true);
        for (whole, cut) in unbroken.iter().zip(&broken) {
            if cut.turned > whole.turned + SWITCH_TURN_OVER {
                wrong.push(format!(
                    "{what} {}: the trunk turned {:.1} degrees in a frame as its ground \
                     switched, past {:.1} and {SWITCH_TURN_OVER:.1} more",
                    cut.name, cut.turned, whole.turned
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How much more than on an unbroken 25 degree slope a beast's trunk may turn
/// in a frame as its ground switches between the slope and the level, in
/// degrees. Provenance: **measured** (#413): the follow turns it 2.9 a frame,
/// and as much over what the states turn it themselves, standing; walking and
/// turning, where those turns fall in other frames, a horse's 2.6.
const SWITCH_TURN_OVER: f32 = 3.0;

/// A leap up off a slope and its landing.
const LEAPT: [(&str, State); 3] = [
    ("standing", (1.0, 0.0, 0.0, None, 0.0, false)),
    ("leaping", (0.7, 0.0, 0.0, None, 3.0, false)),
    ("landed", (1.0, 0.0, 0.0, None, 0.0, false)),
];

#[test]
fn a_beast_in_the_air_rides_the_ground_it_left_whatever_is_under_it() {
    // #413: a leap's flight rode the ground the consumer answered under the
    // body. Carried by a chassis, what is honestly under a body in the air is
    // the ground that far below it, and riding that a dog leaping at 3 m/s off
    // a 25 degree slope peaked 0.57 m up where it peaks 1.05 on the level; a
    // consumer that answers the level floor while the body is up snapped the
    // trunk between the slope and the level at takeoff and at touchdown. The
    // flight now reads nothing, rides what the body stood on as it left, and
    // takes up what it lands on over the landing: the same leap whatever is
    // answered under it in the air, rising as far as on the level, and the
    // trunk turning no faster than the same leap turns it on the level when
    // the ground under it switches at takeoff or at touchdown.
    let mut wrong = Vec::new();
    for (what, avatar) in slope_beasts() {
        let rig = &avatar.rig;
        let honest = trunk_over(rig, &LEAPT, &|_, _| GRADE_25, true, true);
        // The level floor answered while the body is up.
        let floored = trunk_over(
            rig,
            &LEAPT,
            &|_, up| if up > 0.0 { 0.0 } else { GRADE_25 },
            true,
            true,
        );
        for (one, other) in honest.iter().zip(&floored) {
            if one
                .poses
                .iter()
                .map(|(pose, _)| pose)
                .ne(other.poses.iter().map(|(pose, _)| pose))
            {
                wrong.push(format!(
                    "{what} {}: the pose read the ground under it in the air",
                    one.name
                ));
            }
        }
        // As far up as on the level.
        let level = trunk_over(rig, &LEAPT, &|_, _| 0.0, true, true);
        let rise = |through: &[Through]| {
            let root = rig
                .joints
                .iter()
                .position(|joint| joint.parent.is_none())
                .unwrap_or(0);
            let flight = &through[1].poses;
            let height = |(pose, up): &(Pose, f32)| pose.forward(rig).positions[root].y + up;
            let from = height(&flight[0]);
            flight.iter().map(height).fold(f32::MIN, f32::max) - from
        };
        let (risen, flat) = (rise(&honest), rise(&level));
        if (risen - flat).abs() > FLIGHT_RISE_OFF {
            wrong.push(format!(
                "{what}: off the slope the body rose {:.0} mm, on the level {:.0}",
                risen * 1000.0,
                flat * 1000.0
            ));
        }
        // The ground going level as the body leaves the slope, and the slope
        // as it lands from the level.
        let left = std::cell::Cell::new(false);
        let took_off = |_: usize, up: f32| {
            left.set(left.get() || up > 0.0);
            if left.get() { 0.0 } else { GRADE_25 }
        };
        let landing = std::cell::Cell::new(false);
        let landed = |_: usize, up: f32| {
            landing.set(landing.get() || up > 0.0);
            if landing.get() { GRADE_25 } else { 0.0 }
        };
        for (switch, grade) in [
            ("takeoff", &took_off as &dyn Fn(usize, f32) -> f32),
            ("touchdown", &landed),
        ] {
            for (flat, cut) in level.iter().zip(trunk_over(rig, &LEAPT, grade, true, true)) {
                if cut.turned > flat.turned + LEAP_TURN_OVER {
                    wrong.push(format!(
                        "{what} {}: the ground switching at {switch} turned the trunk {:.1} \
                         degrees in a frame, past {:.1} and {LEAP_TURN_OVER:.1} more",
                        cut.name, cut.turned, flat.turned
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far a beast leaping off a slope may rise otherwise than on the level,
/// in metres. Provenance: **measured** (#413, #414): 35 to 36 mm further, a
/// dog's, a horse's and a rolled body's, since the flight rides the turn of
/// the ground it left and not its height (a ground falling away under a body
/// about to drop flew it 0.45 m under its own), against the 480 mm a dog fell
/// short when its flight rode the ground under it.
const FLIGHT_RISE_OFF: f32 = 0.045;

/// How much more than the same leap on the level a beast's trunk may turn in
/// a frame as the ground under it switches between a 25 degree slope and the
/// level at takeoff or at touchdown, in degrees. Provenance: **measured**
/// (#413): landing from the level on the slope a dog's turns 2.6 more, the
/// follow's 2.9 a frame less what the landing's own pitch takes back.
const LEAP_TURN_OVER: f32 = 3.0;

/// A dog and a horse with their withers heights, in metres.
fn dog_and_horse() -> Vec<(&'static str, Avatar, f32)> {
    [("dog", plan::Family::Dog), ("horse", plan::Family::Horse)]
        .into_iter()
        .map(|(what, family)| {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, None, 0.0);
            let withers = params.height * 1.2;
            let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
            record.sanitize();
            let avatar = Avatar::build_with(
                &record,
                &AvatarConfig {
                    atlas: 64,
                    ..AvatarConfig::default()
                },
            )
            .expect("a beast builds");
            (what, avatar, withers)
        })
        .collect()
}

#[test]
fn a_beast_breaking_into_a_trot_carries_every_foot_across() {
    // #412: a walk's four footfalls stand a quarter of a cycle off a trot's
    // two, so at the change a hind paw planted at the back of the walk's
    // stance came out in the middle of the trot's swing and the one blend
    // swept it a stride forward: a horse's 1.31 times what a frame allows on
    // the level, a dog's 1.03, depending on where in its stride it was asked
    // to trot. The change is blended over longer now and the feet held
    // through it let go at once: on the level, wherever the trot is asked, no
    // joint moves past what a frame allows.
    let mut wrong = Vec::new();
    for (what, avatar, withers) in dog_and_horse() {
        let rig = &avatar.rig;
        let walk = Speed::from_froude(0.3).metres_per_second(rig);
        let trot = Speed::from_froude(1.4).metres_per_second(rig);
        let most = crate::rest::swift(withers, STEP);
        let mut worst = (0.0f32, 0usize);
        for late in 0..40 {
            let walking = 90 + late;
            let mut driver = Driver::seeded(INSTRUMENT_SEED);
            let (mut speed, mut at) = (0.0f32, Vec3::ZERO);
            let mut before: Option<Vec<Vec3>> = None;
            for frame in 0..walking + 60 {
                speed = speed_toward(speed, if frame < walking { walk } else { trot });
                let velocity = Vec3::Z * speed;
                at += velocity * STEP;
                let inputs = Inputs {
                    delta: STEP,
                    velocity,
                    at,
                    ..Inputs::default()
                };
                let posed: Vec<Vec3> = driver
                    .drive(rig, &inputs, level_ground)
                    .expect("an unheld body is posed")
                    .pose
                    .forward(rig)
                    .positions
                    .iter()
                    .map(|&point| point + at)
                    .collect();
                if let Some(before) = before.as_ref().filter(|_| frame >= walking) {
                    for (now, was) in posed.iter().zip(before) {
                        let moved = now.distance(*was) / most;
                        if moved > worst.0 {
                            worst = (moved, late);
                        }
                    }
                }
                before = Some(posed);
            }
        }
        if worst.0 > 1.0 {
            wrong.push(format!(
                "{what}: asked to trot {} frames late, a joint moved {:.2} times what a frame \
                 allows",
                worst.1, worst.0
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// What a beast does coming down: off the ground at `launch` metres a second
/// up, or stepping off a ledge `drop` metres high, onto a plane rising `grade`
/// per metre ahead, at `fps` frames a second, its ground answered under it in
/// the air as a chassis carrying it answers it, or with `floored` a level
/// floor answered in the air. The most its trunk pitches, in degrees a second;
/// over the landing the furthest any pad stands above and below where it
/// stands at rest on that ground, in metres; and from three frames before
/// touchdown to three after the most any joint moves in a frame, in what a
/// frame allows a body `withers` high.
fn landed(
    rig: &Rig,
    withers: f32,
    fps: f32,
    launch: f32,
    drop: f32,
    (grade, floored): (f32, bool),
) -> (f32, f32, f32, f32) {
    let step = 1.0 / fps;
    let pads: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .collect();
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let (mut height, mut rising) = (0.0f32, 0.0f32);
    let mut before: Option<f32> = None;
    let mut turned = 0.0f32;
    let mut touched: Option<usize> = None;
    let mut clear = Vec::new();
    let most = crate::rest::swift(withers, step);
    let mut drawn: Option<Vec<Vec3>> = None;
    let mut moves = Vec::new();
    #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
    #[expect(clippy::cast_sign_loss, reason = "a positive rate")]
    let (off, frames, after) = (fps as usize, (3.0 * fps) as usize, (0.6 * fps) as usize);
    for frame in 0..frames {
        if frame == off {
            if launch > 0.0 {
                rising = launch;
                height = 1.0e-4;
            } else {
                height = drop;
            }
        }
        if frame >= off && height > 0.0 {
            rising -= 9.81 * step;
            height = (height + rising * step).max(0.0);
            if height <= 0.0 {
                touched.get_or_insert(frame);
            }
        }
        if height <= 0.0 {
            rising = 0.0;
        }
        let below = height;
        // A level floor answered in the air, the ground really under the
        // body only once it is down on it, as a consumer may answer.
        let answered = if floored && below > 0.0 { 0.0 } else { grade };
        let ground = move |point: Vec3| {
            Some(symbios_avatar::anim::Ground {
                position: Vec3::new(point.x, answered * point.z - below, point.z),
                normal: Vec3::new(0.0, 1.0, -answered).normalize(),
            })
        };
        let inputs = Inputs {
            delta: step,
            velocity: Vec3::Y * rising,
            ..Inputs::default()
        };
        let posed = driver
            .drive(rig, &inputs, ground)
            .expect("an unheld body is posed")
            .pose
            .forward(rig);
        let pitch = trunk_turn(rig, &posed.positions).0;
        let world: Vec<Vec3> = posed
            .positions
            .iter()
            .map(|&at| at + Vec3::Y * below)
            .collect();
        moves.push(drawn.as_ref().map_or(0.0, |was: &Vec<Vec3>| {
            world
                .iter()
                .zip(was)
                .map(|(now, was)| now.distance(*was) / most)
                .fold(0.0f32, f32::max)
        }));
        drawn = Some(world);
        if let Some(was) = before.filter(|_| frame > off) {
            turned = turned.max((pitch - was).abs() * fps);
        }
        before = Some(pitch);
        if touched.is_some_and(|at| frame >= at && frame <= at + after) || frame + 1 == frames {
            clear.push(
                pads.iter()
                    .map(|&pad| {
                        let at = posed.positions[pad];
                        at.y - (grade * at.z - below)
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }
    let rest = clear.pop().expect("the last frame is read");
    let (mut above, mut under) = (0.0f32, 0.0f32);
    for frame in &clear {
        for (now, stands) in frame.iter().zip(&rest) {
            above = above.max(now - stands);
            under = under.max(stands - now);
        }
    }
    let moved = touched.map_or(0.0, |at| {
        moves[at.saturating_sub(3)..(at + 4).min(moves.len())]
            .iter()
            .copied()
            .fold(0.0f32, f32::max)
    });
    (turned, above, under, moved)
}

#[test]
fn a_beast_comes_down_from_a_jump_or_a_ledge_on_its_feet_without_a_snap() {
    // #414: at touchdown the leap is rebuilt from the impact that arrived and
    // its clock set to the head of its landing, whose first pose is nose down,
    // where a fall's flight, cut short wherever the ground arrives, was nose
    // up: a beast stepping off a 0.6 m ledge pitched its trunk 23.8 degrees in
    // the frame it came down, at sixty frames a second and at 144. The landing
    // sets off from the pitch the flight came down in now. And it stands the
    // feet on the ground: a horse's forepaws went 437 mm under it landing a
    // level jump, past what a plant steps a foot up, and every beast's 300 mm
    // under it for nine frames after a ledge, from the ground it rode falling
    // away under it before it left it; the flight rode that height, the drawn
    // body 0.45 m under its own, and came up all of it in the frame it landed.
    // Every family and eight rolled bodies, jumping at 2.5 and 4 m/s and off a
    // ledge 0.6 and 1.5 m high, on the level and on a 20 degree hill, at 60
    // and 144 frames a second: the trunk turns no faster than `LANDING_TURN`,
    // no pad stands further than `LANDING_OFF` off where it stands at rest, or
    // `LANDING_OFF_HILL` on the hill, and over touchdown no joint moves past
    // `LANDING_MOVE` in a frame.
    let mut bodies: Vec<(String, AvatarRecord)> = plan::Family::ALL
        .into_iter()
        .map(|family| {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, None, 0.0);
            (
                format!("{family:?}"),
                AvatarRecord::new("Beast", Archetype::Quadruped(params)),
            )
        })
        .collect();
    for seed in 0..8 {
        bodies.push((
            format!("roll {seed}"),
            AvatarRecord::rolled(
                "Beast",
                Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            ),
        ));
    }
    let wrong: Vec<String> = std::thread::scope(|scope| {
        let runs: Vec<_> = bodies
            .into_iter()
            .map(|(what, mut record)| {
                scope.spawn(move || {
                    record.sanitize();
                    let avatar = Avatar::build_with(
                        &record,
                        &AvatarConfig {
                            atlas: 64,
                            ..AvatarConfig::default()
                        },
                    )
                    .expect("a beast builds");
                    let withers = match &record.archetype {
                        Archetype::Quadruped(params) => params.height * 1.2,
                        _ => unreachable!("every record here is a beast"),
                    };
                    let mut wrong = Vec::new();
                    for fps in [60.0f32, 144.0] {
                        for (how, launch, drop, ground) in [
                            ("jumping at 2.5 m/s", 2.5, 0.0, (0.0, false)),
                            ("jumping at 4 m/s", 4.0, 0.0, (0.0, false)),
                            ("jumping at 4 m/s up a hill", 4.0, 0.0, (0.364, false)),
                            ("off a ledge", 0.0, 0.6, (0.0, false)),
                            ("off a high ledge", 0.0, 1.5, (0.0, false)),
                            ("off a ledge onto a hill", 0.0, 0.6, (0.364, false)),
                            ("off a high ledge onto a hill", 0.0, 1.5, (0.364, false)),
                            (
                                "off a ledge, a floor answered in the air",
                                0.0,
                                0.6,
                                (0.364, true),
                            ),
                            (
                                "off a high ledge, a floor answered in the air",
                                0.0,
                                1.5,
                                (0.364, true),
                            ),
                        ] {
                            let (turned, above, under, moved) =
                                landed(&avatar.rig, withers, fps, launch, drop, ground);
                            let off = match ground {
                                (0.0, _) => LANDING_OFF,
                                (_, false) => LANDING_OFF_HILL,
                                (_, true) => LANDING_OFF_FLOORED,
                            };
                            let fast = if fps > 60.0 {
                                LANDING_MOVE_FAST
                            } else {
                                LANDING_MOVE
                            };
                            if turned > LANDING_TURN || above.max(under) > off || moved > fast {
                                wrong.push(format!(
                                    "{what} {how} at {fps} frames a second: the trunk pitched \
                                     {turned:.0} degrees a second, past {LANDING_TURN:.0}; a \
                                     pad stood {:.0} mm above where it stands and {:.0} mm \
                                     under it, past {:.0}; a joint moved {moved:.2} times what \
                                     a frame allows at touchdown, past {fast:.2}",
                                    above * 1000.0,
                                    under * 1000.0,
                                    off * 1000.0
                                ));
                            }
                        }
                    }
                    wrong
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("a beast lands"))
            .collect()
    });
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far any pad of a beast coming down may stand off where it stands at
/// rest, above or below, from touchdown to six tenths of a second after, in
/// metres: on the level, on a hill, and on a hill where a level floor was
/// answered in the air. Provenance: **measured** (#414): on the level 26 mm, a
/// rolled body's paw coming off a ledge, against 437 and 350 under the ground
/// in HEAD; on a 20 degree hill 76 mm, the feet standing on the ground the
/// trunk turns onto as it follows the hill (see `quadruped::follow`), against
/// 620; from a level floor in the air onto the hill 341 mm under it as the
/// landing takes the hill up over its first three tenths, against 794.
const LANDING_OFF: f32 = 0.04;
const LANDING_OFF_HILL: f32 = 0.09;
const LANDING_OFF_FLOORED: f32 = 0.4;

/// The most a joint of a beast may move in a frame from three frames before
/// touchdown to three after, in what a frame allows, at sixty frames a second
/// and at 144. Provenance: **measured** (#414): 1.55 and 3.03, a 4 m/s jump up
/// a 20 degree hill, against HEAD's 2.24 and 5.39 off a 0.6 m ledge, where the
/// flight rode the height of a ground falling away under the body and its paws
/// came down 300 mm under the ground.
const LANDING_MOVE: f32 = 2.0;
const LANDING_MOVE_FAST: f32 = 3.5;

/// How fast a beast's trunk may pitch through a jump or a drop and its
/// landing, in degrees a second. Provenance: **measured** (#414): 256, coming
/// down 1.5 m onto a 20 degree hill from a level floor answered in the air,
/// the trunk taking the hill up as it lands, and 195 everywhere else, against
/// 1429 at sixty frames a second and 3186 at 144 when the touchdown snapped.
const LANDING_TURN: f32 = 280.0;

/// How far ahead of where it stops [`Terrain`]'s step stands, in withers
/// heights: under the forefeet of a beast sitting or lying there.
const STEP_AHEAD: f32 = 0.33;

#[test]
fn a_body_seated_in_a_state_is_where_going_down_would_have_it() {
    // #383: one call for what took a consumer unseen frames.
    let beast = beast();
    let rig = &beast.rig;
    let frame = 1.0 / 60.0;
    let idle = Inputs {
        delta: frame,
        ..Inputs::default()
    };
    for name in ["Sit", "Lie Down"] {
        let clip = gesture::for_body(rig, name).expect("a beast's state");
        let down = emote::seconds(rig, name).expect("a state keeps its time");
        for through in [0.25f32, 0.7, 1.0, 2.0] {
            let mut seated = Driver::seeded(INSTRUMENT_SEED);
            assert!(seated.seat(rig, name, through), "{name} is a state");
            let (playing, elapsed) = seated.gesture().expect("seated in it");
            assert_eq!(playing, name);
            assert!((elapsed - through.min(1.0) * down).abs() < 1.0e-4);
            // A frame of nothing moves it on by nothing to speak of.
            let drawn = seated
                .drive(
                    rig,
                    &Inputs {
                        delta: 1.0e-5,
                        ..Inputs::default()
                    },
                    level_ground,
                )
                .expect("an unheld body is posed");
            assert_eq!(drawn.source, Source::Gesture);
            let wanted = clip.pose(rig, through).forward(rig);
            let posed = drawn.pose.forward(rig);
            let apart = wanted
                .positions
                .iter()
                .zip(&posed.positions)
                .map(|(one, two)| one.distance(*two))
                .fold(0.0f32, f32::max);
            assert!(
                apart < 2.0e-3,
                "{name} at {through}: a joint stood {:.1} mm off the way down",
                apart * 1000.0
            );
        }
        // From part way down it goes on down, and held it stays, and each gets
        // up: from the held state as the animal does, from part way as it came.
        let mut part = Driver::seeded(INSTRUMENT_SEED);
        part.seat(rig, name, 0.5);
        for _ in 0..3 {
            part.drive(rig, &idle, level_ground);
        }
        let (_, moved_on) = part.gesture().expect("still going down");
        assert!(moved_on > 0.5 * down, "it did not go on down");
        let up = |driver: &mut Driver| {
            let asked = Inputs {
                delta: frame,
                gesture: Some(emote::GET_UP),
                ..Inputs::default()
            };
            driver.drive(rig, &asked, level_ground);
            for _ in 0..1200 {
                driver.drive(rig, &idle, level_ground);
                if driver.gesture().is_none() {
                    return true;
                }
            }
            false
        };
        assert!(up(&mut part), "{name}: never got up from part way");
        let mut held = Driver::seeded(INSTRUMENT_SEED);
        held.seat(rig, name, 1.0);
        for _ in 0..600 {
            held.drive(rig, &idle, level_ground);
        }
        assert!(held.gesture().is_some(), "{name}: a held state let go");
        assert!(up(&mut held), "{name}: never got up from held");
        // None of it, and it is out of it.
        held.seat(rig, name, 1.0);
        assert!(held.seat(rig, name, 0.0));
        assert_eq!(held.gesture(), None);
    }
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    assert!(!driver.seat(rig, "Wag", 0.5), "a wag is no state");
    assert_eq!(driver.gesture(), None, "and it changed nothing");
    let person = AvatarRecord::new("Person", Archetype::default());
    let person = Avatar::build(&person).expect("a person builds");
    assert!(!driver.seat(&person.rig, "Sit", 0.5), "nor is a person's");
}

#[test]
fn what_stirs_a_body_is_what_gets_a_beast_up() {
    // #383: the driver's own reading, by construction what its frame asks.
    let beast = beast();
    let rig = &beast.rig;
    let frame = 1.0 / 60.0;
    for (speed, stirs) in [(0.0f32, false), (0.09, false), (0.11, true), (2.0, true)] {
        let inputs = Inputs {
            delta: frame,
            velocity: Vec3::new(speed * 0.6, 0.0, speed * 0.8),
            ..Inputs::default()
        };
        let mut driver = Driver::seeded(INSTRUMENT_SEED);
        assert_eq!(driver.stirred(&inputs), stirs, "at {speed}");
        driver.seat(rig, "Sit", 1.0);
        driver.drive(rig, &inputs, level_ground);
        let mut hold = 0;
        for _ in 0..600 {
            driver.drive(rig, &inputs, level_ground);
            hold += usize::from(driver.gesture().is_some());
        }
        assert_eq!(
            hold == 600,
            !stirs,
            "at {speed} it {} up",
            if stirs { "did not get" } else { "got" }
        );
    }
    let mut driver = Driver::seeded(INSTRUMENT_SEED);
    let swimming = Inputs {
        swimming: true,
        ..Inputs::default()
    };
    assert!(driver.stirred(&swimming), "a swimming body is stirred");
    let leaping = Inputs {
        showing: Some(Showing::Leap(Leap::new(3.0))),
        ..Inputs::default()
    };
    assert!(driver.stirred(&leaping), "a body shown leaping is stirred");
    // Off the ground, whatever the frame says.
    let launch = Inputs {
        delta: frame,
        velocity: Vec3::Y * 4.0,
        ..Inputs::default()
    };
    driver.drive(rig, &launch, level_ground);
    assert!(driver.airborne().is_some(), "launched");
    assert!(driver.stirred(&Inputs::default()), "an airborne body is");
}
