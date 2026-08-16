//! Measures a standing body against the thing an idle is judged on: that it
//! never repeats, and that it never stops.
//!
//! An avatar is idle for most of the time it exists, so this is the state a
//! viewer sees longest — and it is the one place a procedural layer beats an
//! imported clip outright rather than merely matching it. **A clip is a loop,
//! and a loop repeats.** However good the performance, an eight-second idle
//! comes round every eight seconds, and once a viewer has noticed that they
//! cannot stop noticing it.
//!
//! So the headline reading here is not an amplitude, it is a **recurrence**:
//! over a long run, how close does the body ever come to a pose it already
//! held? The shipped `Idle_A` is measured beside the procedural layer for
//! exactly that comparison, and it is the reason the reading is trustworthy —
//! a clip must score zero at its own period, and an instrument that does not
//! find that is not measuring recurrence.
//!
//! The rest are the properties a still body has to have, each of which is a
//! way an idle goes wrong:
//!
//! 1. **Does it breathe, and at a person's rate?** 12–20 cycles a minute at
//!    rest, with the abdomen taking part rather than the chest moving alone.
//! 2. **Does it sway, and stay standing?** Quiet standing is an inverted
//!    pendulum about the ankles: a few millimetres to a centimetre of drift,
//!    almost all of it under 1 Hz, and never outside the feet — a body whose
//!    sway leaves its own support polygon is falling over.
//! 3. **Do the feet hold still?** A standing body's feet do not move at all.
//!    This is the reading that catches a sway applied to the root instead of
//!    about the ankles, which drags the whole body sideways, soles and all.
//! 4. **Is it smooth?** Noise sampled per frame rather than over time reads as
//!    a shiver. Measured as the largest single-frame movement of any joint.
//!
//! Reference bands are the quiet-standing and respiratory literature: resting
//! respiratory rate 12–20 per minute; centre-of-pressure excursion roughly
//! 5–15 mm RMS in quiet stance with the spectrum concentrated below 1 Hz;
//! postural sway an inverted pendulum whose natural frequency is `sqrt(g/h)`
//! about the ankle, which for an adult centre of mass is near 0.5 Hz.
//!
//! ```text
//! cargo run --release --example idleaudit
//! cargo run --release --example idleaudit -- --seconds 300
//! cargo run --release --example idleaudit -- --talking
//! cargo run --release --example idleaudit -- --listening
//! cargo run --release --example idleaudit -- --clip Idle_A   # the baseline
//! ```

use glam::Vec3;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, ClipLibrary, Idle, IdleConfig, Limb, Pose, Zone,
};

/// Where the shipped reference clips live.
const ARTIFACT: &str = "assets/clips.bin";

/// How often the body is posed, in seconds.
///
/// A sixtieth, because that is the rate a viewer draws at and the smoothness
/// reading is a per-frame quantity — measured on a coarser step it would report
/// whatever the step was rather than what the body does.
const STEP: f32 = 1.0 / 60.0;

/// How often a pose is kept for the recurrence search, in seconds.
///
/// Coarser than [`STEP`] because the search is quadratic in the number kept and
/// a quarter second is far finer than any repeat worth finding: a loop that
/// came round on a quarter-second boundary would be a shiver, not an idle.
const KEEP: f32 = 0.25;

/// How far apart two poses must be in time before their likeness counts as a
/// repeat, in seconds.
///
/// **Two seconds, and it is a floor rather than a tuning.** Any continuous
/// motion resembles itself a moment later — that is what continuous means — so
/// a recurrence search with no lag reports every frame as a repeat of the one
/// before it. The question is whether the body comes back to a pose it held
/// *long* ago, and two seconds is comfortably past the point where breath alone
/// would bring it round.
const LAG: f32 = 2.0;

/// How long a stretch of motion has to match before it counts as a repeat, in
/// seconds.
///
/// **Two, and the reason there is a window at all is that an instant was not
/// enough** — see [`recurrence`]. Long enough that matching it means the body
/// really is retracing a path, short enough that a clip of any length still
/// contains one.
const WINDOW: f32 = 2.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let text = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let seconds = number("--seconds").unwrap_or(120.0).max(LAG * 2.0);
    let clip = text("--clip");
    let config = if args.iter().any(|arg| arg == "--talking") {
        IdleConfig::talking()
    } else if args.iter().any(|arg| arg == "--listening") {
        IdleConfig::listening()
    } else {
        IdleConfig::default()
    };

    let record = AvatarRecord::new("Idler", Archetype::default());
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the standing body would not build");
        std::process::exit(1);
    };
    let rig = &avatar.rig;

    // The joints each reading is taken at. Named by zone rather than by index
    // so a body with a different spine still answers.
    let first = |zone: Zone| rig.in_zone(zone).first().copied();
    let chest = first(Zone::Chest);
    // **Not the abdomen joint.** Rotating a joint moves its children, not
    // itself, so the hinge the breath drives never moves and reading it
    // reported the abdomen contributing exactly nothing. What the second column
    // wants is the cue the eye actually reads, which on this rig is the
    // shoulder.
    // ...measured at the ELBOW, because the shoulder joint is the pivot the
    // breath turns and a joint cannot move itself. On a rig with no clavicle
    // that is all a breath can reach: the arm opens slightly, the shoulder does
    // not rise. Reading the shoulder reported the cue contributing 1% of the
    // chest, which was the ruler and not the layer.
    let shoulder = rig.limb_chain(Limb::ForeLeft).map(|chain| chain[1]);
    let head = first(Zone::Head);
    let root = rig.joints.iter().position(|joint| joint.parent.is_none());
    let contacts: Vec<usize> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| first(Zone::Extremity(limb)))
        .collect();

    // **The support polygon, as the feet actually stand.** A sway is only a
    // sway while the body stays over its own feet; past that edge it is a step
    // the body has not taken. Taken as the extent of the contacts in each
    // horizontal axis, which for two feet side by side is the width that
    // matters and a generous bound on the fore-aft one.
    //
    // **Every joint of every foot, not the contact joints.** Two feet side by
    // side put their contacts at the same `z`, so a polygon built from those
    // alone has no fore-and-aft depth at all — it is a line — and the
    // containment reading came out negative on a body that was standing
    // perfectly well. A foot's support runs heel to toe, and
    // `extremity_joints` is what knows where those are.
    let sole: Vec<Vec3> = rig
        .ground_contacts()
        .into_iter()
        .flat_map(|limb| rig.extremity_joints(limb))
        .map(|joint| rig.joints[joint].position)
        .collect();
    let span = |axis: fn(Vec3) -> f32| {
        let low = sole.iter().copied().map(axis).fold(f32::MAX, f32::min);
        let high = sole.iter().copied().map(axis).fold(f32::MIN, f32::max);
        (low, high)
    };
    let across = span(|at| at.x);
    let fore = span(|at| at.z);

    // One source of poses, whichever is being measured, so every reading below
    // is taken the same way on both. That is the whole reason the clip is
    // driven through here rather than measured by a second instrument.
    let library = clip.as_ref().map(|name| {
        let bytes = std::fs::read(ARTIFACT).unwrap_or_else(|_| {
            eprintln!("{ARTIFACT} is not there; run examples/bakeclips first");
            std::process::exit(1);
        });
        let library = ClipLibrary::read(&bytes).expect("the artifact parses");
        if library.get(name).is_none() {
            eprintln!(
                "no clip called {name}; the artifact holds {:?}",
                library.names()
            );
            std::process::exit(1);
        }
        library
    });
    let reference = library
        .as_ref()
        .zip(clip.as_ref())
        .and_then(|(library, name)| library.get(name));

    let mut idle = Idle::new(config, 0x1de);
    let frames = (seconds / STEP) as usize;
    let keep_every = (KEEP / STEP).max(1.0) as usize;

    // The arm chains, for the carriage reading: how far off vertical the
    // upper arm hangs and how bent the elbow is, both off the POSED skeleton.
    let arms: Vec<[usize; 3]> = [Limb::ForeLeft, Limb::ForeRight]
        .into_iter()
        .filter_map(|limb| rig.limb_chain(limb))
        .collect();

    let mut kept: Vec<(f32, Vec<Vec3>)> = Vec::new();
    let mut breath_chest: Vec<f32> = Vec::new();
    let mut breath_abdomen: Vec<f32> = Vec::new();
    let mut head_path: Vec<Vec3> = Vec::new();
    let mut foot_drift = 0.0f32;
    let mut jitter = 0.0f32;
    let mut jitter_at = 0.0f32;
    let mut shifts = 0usize;
    let mut fidgets = 0usize;
    let mut bearing: Option<Limb> = None;
    let mut previous: Option<Vec<Vec3>> = None;
    let (mut arm_hang, mut arm_bend) = ((f32::MAX, f32::MIN), (f32::MAX, f32::MIN));
    let rest = Pose::rest(rig).forward(rig).positions;

    for frame in 0..frames {
        let at = frame as f32 * STEP;
        let mut pose = Pose::rest(rig);
        match reference {
            // Wrapped at the clip's own duration, which is exactly what a
            // player does with it and exactly why it repeats.
            Some(clip) => clip.apply(rig, &mut pose, at.rem_euclid(clip.duration())),
            None => {
                let idled = idle.drive(rig, &mut pose, STEP);
                if idled.bearing != bearing {
                    // Not counted at the start, where there is no previous
                    // state to have changed from.
                    if bearing.is_some() || frame > 0 {
                        shifts += 1;
                    }
                    bearing = idled.bearing;
                }
                if idled.fidgeting {
                    fidgets += 1;
                }
            }
        }
        let posed = pose.forward(rig).positions;

        // **Displacement, not rise, and relative to the ROOT.** Two rulers
        // were wrong here before this line settled. A `Pose` is rotations and
        // one root translation — no per-joint translation, no scale — so a body
        // on this rig cannot lift its chest straight up, and asking how far it
        // rose reads the small drop an extension arc leaves and calls the
        // breath backwards. And measuring against the rest pose in world space
        // reads the sway and the weight shift too: the chest came out moving
        // 90 mm at 1.8 cycles a minute, which is the shift schedule and not a
        // breath at all. A breath is a motion of the spine, so it is measured
        // in the frame the spine hangs from.
        let base = root.map_or(Vec3::ZERO, |joint| posed[joint] - rest[joint]);
        if let Some(joint) = chest {
            breath_chest.push((posed[joint] - rest[joint] - base).length());
        }
        if let Some(joint) = shoulder {
            breath_abdomen.push((posed[joint] - rest[joint] - base).y);
        }
        if let Some(joint) = head {
            head_path.push(posed[joint]);
        }
        for &joint in &contacts {
            foot_drift = foot_drift.max(posed[joint].distance(rest[joint]));
        }
        if let Some(before) = &previous {
            for (a, b) in before.iter().zip(&posed) {
                if a.distance(*b) > jitter {
                    jitter = a.distance(*b);
                    jitter_at = at;
                }
            }
        }

        for &[shoulder, elbow, wrist] in &arms {
            let upper = posed[elbow] - posed[shoulder];
            let hang = upper
                .normalize_or(Vec3::NEG_Y)
                .dot(Vec3::NEG_Y)
                .clamp(-1.0, 1.0);
            let fore = posed[wrist] - posed[elbow];
            let bend = upper
                .normalize_or(Vec3::NEG_Y)
                .dot(fore.normalize_or(Vec3::NEG_Y))
                .clamp(-1.0, 1.0);
            arm_hang = (arm_hang.0.min(hang.acos()), arm_hang.1.max(hang.acos()));
            arm_bend = (arm_bend.0.min(bend.acos()), arm_bend.1.max(bend.acos()));
        }
        previous = Some(posed);

        if frame % keep_every == 0 {
            kept.push((at, previous.clone().unwrap_or_default()));
        }
    }

    println!(
        "{} for {seconds:.0} s at {:.0} Hz, on the default body",
        clip.as_deref()
            .map_or("the procedural idle".to_string(), |name| format!(
                "the clip {name}"
            )),
        1.0 / STEP
    );

    // ---- arms ------------------------------------------------------------
    // The carriage, against the bind pose it must NOT be showing (#267): a
    // relaxed arm hangs a few degrees off vertical with a slightly bent
    // elbow, and the A-pose the body is modelled in is neither.
    {
        let rest_arm = |chain: &[usize; 3]| {
            let upper = rest[chain[1]] - rest[chain[0]];
            let fore = rest[chain[2]] - rest[chain[1]];
            (
                upper
                    .normalize_or(Vec3::NEG_Y)
                    .dot(Vec3::NEG_Y)
                    .clamp(-1.0, 1.0)
                    .acos(),
                upper
                    .normalize_or(Vec3::NEG_Y)
                    .dot(fore.normalize_or(Vec3::NEG_Y))
                    .clamp(-1.0, 1.0)
                    .acos(),
            )
        };
        let bind: Vec<(f32, f32)> = arms.iter().map(rest_arm).collect();
        let bind_hang = bind.iter().map(|arm| arm.0).fold(0.0f32, f32::max);
        let bind_bend = bind.iter().map(|arm| arm.1).fold(0.0f32, f32::max);
        println!(
            "  arms:   hung {:.0} to {:.0} deg off vertical, elbow bent {:.0} to {:.0} deg \
             (the BIND pose splays {:.0} deg with {:.0} of elbow — a standing body showing \
             those numbers is showing the modelling pose, #267. A relaxed hang is ~5-15 deg \
             of abduction with ~10-25 deg of elbow)",
            arm_hang.0.to_degrees(),
            arm_hang.1.to_degrees(),
            arm_bend.0.to_degrees(),
            arm_bend.1.to_degrees(),
            bind_hang.to_degrees(),
            bind_bend.to_degrees(),
        );
    }

    // ---- breath ----------------------------------------------------------
    //
    // **On a second body, doing nothing else.** A breath moves the chest about
    // a millimetre and a weight shift moves it twenty, so a breath measured on
    // a body that is also swaying and shifting is a measurement of the shift —
    // it read 20.2 mm at 1.8 cycles a minute, which is the shift schedule with
    // a breath's name on it. Isolating the layer is the only ruler that works
    // at this ratio, and it is honest as long as it is said: these two columns
    // are the breath alone.
    let (rate, swing, belly) = if reference.is_some() {
        let (rate, swing) = oscillation(&breath_chest, STEP);
        let (_, belly) = oscillation(&breath_abdomen, STEP);
        (rate, swing, belly)
    } else {
        let mut alone = Idle::new(
            IdleConfig {
                sway: 0.0,
                min_shift: f32::MAX,
                max_shift: f32::MAX,
                min_fidget: f32::MAX,
                max_fidget: f32::MAX,
                ..config
            },
            0x1de,
        );
        let (mut chests, mut bellies) = (Vec::new(), Vec::new());
        for _ in 0..frames {
            let mut pose = Pose::rest(rig);
            alone.drive(rig, &mut pose, STEP);
            let posed = pose.forward(rig).positions;
            let base = root.map_or(Vec3::ZERO, |joint| posed[joint] - rest[joint]);
            // **Signed, along the axis the motion actually runs.** A magnitude
            // is always positive, so it returns to zero TWICE a cycle and reads
            // the rate as double — measured, 32.5 breaths a minute on a body
            // breathing 16. The spine's extension is a rotation about `x`, so
            // its excursion is along `z`.
            if let Some(joint) = chest {
                chests.push((posed[joint] - rest[joint] - base).z);
            }
            if let Some(joint) = shoulder {
                bellies.push((posed[joint] - rest[joint] - base).y);
            }
        }
        let (rate, swing) = oscillation(&chests, STEP);
        let (_, belly) = oscillation(&bellies, STEP);
        (rate, swing, belly)
    };
    println!(
        "\n  breath: the chest moved {:.1} mm peak to peak at {:.1} cycles per minute, and \
         the arms {:.1} mm ({:.0}% of the chest — this rig has no clavicle, so a breath \
         cannot raise a shoulder at all)",
        swing * 1000.0,
        rate * 60.0,
        belly * 1000.0,
        if swing > f32::EPSILON {
            belly / swing * 100.0
        } else {
            0.0
        },
    );
    println!(
        "          (reference: 12-20 per minute at rest, and the abdomen takes part rather \
         than the chest moving alone. Measured as DISPLACEMENT: this rig has no per-joint \
         translation, so a breath is spine and shoulders and the chest does not travel \
         straight up)"
    );

    // ---- sway ------------------------------------------------------------
    let flat: Vec<Vec3> = head_path
        .iter()
        .map(|at| Vec3::new(at.x, 0.0, at.z))
        .collect();
    let centre = flat.iter().copied().sum::<Vec3>() / flat.len().max(1) as f32;
    let rms = (flat
        .iter()
        .map(|at| at.distance_squared(centre))
        .sum::<f32>()
        / flat.len().max(1) as f32)
        .sqrt();
    let reach = flat
        .iter()
        .map(|at| at.distance(centre))
        .fold(0.0f32, f32::max);
    let lateral: Vec<f32> = flat.iter().map(|at| at.x - centre.x).collect();
    let (sway_rate, _) = oscillation(&lateral, STEP);
    println!(
        "  sway:   the head wandered {:.1} mm rms and {:.1} mm at its furthest, crossing its \
         own centre {:.2} times a second",
        rms * 1000.0,
        reach * 1000.0,
        sway_rate,
    );
    println!(
        "          (reference: 5-15 mm rms in quiet stance, almost all of it under 1 Hz — an \
         inverted pendulum about the ankles, whose own frequency is sqrt(g/h))"
    );

    // Containment, measured where it matters: the horizontal extent the body
    // reached against the ground it is standing on.
    let out_x = flat
        .iter()
        .map(|at| (across.0 - at.x).max(at.x - across.1))
        .fold(f32::MIN, f32::max);
    let out_z = flat
        .iter()
        .map(|at| (fore.0 - at.z).max(at.z - fore.1))
        .fold(f32::MIN, f32::max);
    println!(
        "  stance: the sway stayed {:.0} mm inside the feet across and {:.0} mm fore-and-aft \
         (a body outside its own support polygon is falling, not swaying)",
        -out_x * 1000.0,
        -out_z * 1000.0,
    );

    // ---- the feet, and the frame -----------------------------------------
    println!(
        "  feet:   moved {:.2} mm at the furthest (a standing body's feet do not move at all; \
         anything here is a sway applied to the root rather than about the ankles)",
        foot_drift * 1000.0
    );
    println!(
        "  frame:  the largest single-frame movement of any joint was {:.3} mm at {jitter_at:.1} s, \
         which at {:.0} Hz is {:.1} mm/s (noise drawn per frame rather than over time reads \
         as a shiver)",
        jitter * 1000.0,
        1.0 / STEP,
        jitter * 1000.0 / STEP,
    );

    if reference.is_none() {
        // The fidget flag is raised for every frame one lasts, so this is a
        // duration and is reported as one rather than dressed up as a count.
        println!(
            "  events: {shifts} weight shifts, and a fidget was running for {:.1} s of the \
             {seconds:.0}",
            fidgets as f32 * STEP
        );
    }

    // ---- the headline ----------------------------------------------------
    let (closest, apart) = recurrence(&kept, LAG);
    println!(
        "\n  REPEAT: the closest the body ever came to a pose it already held, at least \
         {LAG:.0} s earlier, was {:.1} mm at the furthest joint of a {WINDOW:.0} s \
         stretch — {:.0} s apart",
        closest * 1000.0,
        apart,
    );
    match reference {
        Some(clip) => println!(
            "          (this is a LOOP of {:.1} s, so it must score ~0 at that lag; a reading \
             here that is not near zero means the search is not finding recurrence and no \
             other number below it can be trusted)",
            clip.duration()
        ),
        None => println!(
            "          (nothing here loops: the sway is noise over time and the shifts and \
             fidgets are drawn from a schedule, so the body should never come back. Compare \
             `--clip Idle_A`)"
        ),
    }
}

/// The rate and the peak-to-peak swing of a signal that oscillates about its
/// own mean.
///
/// **Counted as crossings of the mean rather than fitted**, because the signal
/// this is asked about is deliberately not a sine: a breath is close to one and
/// a sway is not one at all, and a fit would report how badly the wrong model
/// matched. Crossings of the mean are what "how often does it come back" means
/// for any shape, and they are the reading the frequency band in the literature
/// is quoted against.
///
/// The swing is taken from the 5th and 95th percentiles rather than the
/// extremes, so one excursion in a five-minute run does not become the
/// amplitude.
fn oscillation(signal: &[f32], step: f32) -> (f32, f32) {
    if signal.len() < 2 || step <= 0.0 {
        return (0.0, 0.0);
    }
    let mean = signal.iter().sum::<f32>() / signal.len() as f32;
    let crossings = signal
        .windows(2)
        .filter(|pair| (pair[0] - mean).signum() != (pair[1] - mean).signum())
        .count();
    // Two crossings to a cycle.
    let rate = crossings as f32 / 2.0 / (signal.len() as f32 * step);

    let mut sorted: Vec<f32> = signal.to_vec();
    sorted.sort_by(f32::total_cmp);
    let at = |share: f32| sorted[((sorted.len() - 1) as f32 * share) as usize];
    (rate, at(0.95) - at(0.05))
}

/// How close the body ever came to repeating a **stretch** of itself, in
/// metres, and how far apart in time the two stretches were.
///
/// **In positions rather than in rotations, and as the FURTHEST joint rather
/// than the average.** Both halves of that were learned by getting them wrong:
/// an average joint rotation over a whole skeleton reported 0.00 degrees for
/// the procedural idle and 0.00 for a three-second loop alike, because on a
/// forty-joint body a handful of moving joints divided by forty is nothing
/// whichever body it is. A metric with no resolution agrees with everything.
///
/// Positions are also the honest frame for the question. Nobody watches
/// quaternions; the question is whether the body LOOKS like it did before, and
/// two poses look alike exactly when every joint is near where it was.
///
/// **A stretch and not an instant**, which is the difference between the
/// question a viewer is asking and one that flatters everything. Any slow
/// motion passes through poses that resemble ones it held before — a body
/// swaying gently comes within half a millimetre of an earlier moment several
/// times a minute, and so did this one, which said nothing about whether it
/// loops. A LOOP repeats what comes next as well. So the comparison runs over
/// [`WINDOW`] of consecutive samples and takes the worst joint of the worst of
/// them: two stretches match only if the body did the same thing for the whole
/// of both.
///
/// Pairs closer together than `lag` are skipped: every continuous motion
/// resembles itself a moment later, and that is not a repeat.
fn recurrence(kept: &[(f32, Vec<Vec3>)], lag: f32) -> (f32, f32) {
    let span = (WINDOW / KEEP).max(1.0) as usize;
    let mut closest = f32::MAX;
    let mut apart = 0.0;
    for (one, (at, _)) in kept.iter().enumerate() {
        if one + span > kept.len() {
            break;
        }
        for (other, (later, _)) in kept.iter().enumerate() {
            if later - at < lag || other + span > kept.len() {
                continue;
            }
            let distance = (0..span)
                .map(|step| {
                    kept[one + step]
                        .1
                        .iter()
                        .zip(&kept[other + step].1)
                        .map(|(a, b)| a.distance(*b))
                        .fold(0.0f32, f32::max)
                })
                .fold(0.0f32, f32::max);
            if distance < closest {
                closest = distance;
                apart = later - at;
            }
        }
    }
    if closest == f32::MAX {
        (0.0, 0.0)
    } else {
        (closest, apart)
    }
}
