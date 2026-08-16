//! Whether a goal-space gesture reads the same on every body.
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
//! 3b. **The trunk**, for a clip that pitches one: how far the chord from the
//!    pelvis to the shoulder girdle inclines off where it rests. **The chord
//!    and not the hinge's rotation** — those are different angles, and the gait
//!    found out how different by writing a constant against the wrong one.
//!
//! 3c. **The carriage**, for a clip that moves one: how far the root dropped,
//!    what angle that left the knee at, how far the body tipped, and how far
//!    its lowest SURFACE sank below where the same body's does when it stands.
//!    Two corrections, and the clip reads as broken without either: a joint is
//!    a point on an axis and the body hangs off it by the bone's own radius, so
//!    the surface is the joint less that; and a STANDING body already reads 50
//!    mm under, because a node's radius overstates a surface that subdivision
//!    pulls inside it. Against zero, every clip sinks. Against the body's own
//!    standing figure, only the ones that do. **The knee is the reading that
//!    matters** and it is an angle, because sitting is a right angle at the
//!    knee and nothing about a length: the drop and the reach that produce it
//!    are stated in the leg, and whether that was the right unit shows up here.
//!
//! 3a. **The gaze**, for a clip that has one: how far the head's facing pitches
//!    from rest, how much of that the chest took, and how far the head joint
//!    actually travelled. **A rotation cannot be judged by the spread of a
//!    displacement**, which is what every reading above is — so a gaze clip is
//!    judged on the spread of its ANGLE, and that needed the sweep to grow an
//!    axis that moves a neck against a body (see [`BODIES`]).
//!
//! 4. **Return.** How far from rest the body is at the first and last frame. A
//!    gesture that ends somewhere else leaves the body in a pose it never chose.
//!
//! 6. **Elbow and palm.** How far above the shoulder the elbow ever gets, and
//!    which way the palm faces at the gesture's peak. Both are defects the eye
//!    finds first: a raise reads as a stretch when the pole flares the elbow
//!    level with the shoulder, and the palm faces wherever the forearm's arc
//!    leaves it. The eye finds them; these read them.
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
//!
//! Every reading is taken over only what the clip actually addresses: a
//! one-handed wave leaves the other palm alone, and a nod's hand columns are
//! blank rather than zero. The distinction is not cosmetic — the palm reading's
//! first cut maxed over both hands and reported the untouched one's rest pose
//! as the greeting's failing.

use glam::Vec3;
use symbios_avatar::{
    Limb, Pose, Rig, Zone,
    anim::{
        Target,
        gesture::{self, ROSTER},
    },
    plan::{BodyPlan, Composites, HumanoidParams},
    rig::landmark,
};

/// How many points of a gesture every reading is taken from.
const SWEEP: usize = 120;

/// The bodies the roster is judged on: a stature in metres, then limb length,
/// neck length and head size against it.
///
/// **Stature alone is not a test of this format.** The plan scales a body
/// uniformly with height, so every length in it stays in the same proportion
/// and a goal measured in reaches lands in the same place by arithmetic rather
/// than by design — the spread reads a flat zero and proves nothing. What can
/// move a normalised goal is a change of PROPORTION: `limb_length` raises the
/// pelvis and shortens the torso at a fixed stature, so it moves the shoulder,
/// the reach and the hand's rest position against each other. The sweep runs
/// both, and the extremes of the second are where a gesture actually breaks.
///
/// **And limb proportion is blind to the head.** Across `limb_length`
/// -1, 0 and +1 the neck stays 0.250 of a body's height and the head sits at
/// 0.955 of it, to three figures — so a sweep of stature and limbs cannot tell
/// a nod written as an angle from one written as a displacement. Both read a
/// spread of 0.000 and the reading endorses whichever was written first. The
/// axes that move a head against a body are `neck_length` and `head_size`, and
/// on those the two authorings come apart by 14 degrees. They are here because
/// a rotation asked for them; they cost the reach readings nothing.
const BODIES: [(f32, f32, f32, f32); 11] = [
    (1.2, 0.0, 0.0, 0.0),
    (1.5, 0.0, 0.0, 0.0),
    (1.7, 0.0, 0.0, 0.0),
    (2.0, 0.0, 0.0, 0.0),
    (2.2, 0.0, 0.0, 0.0),
    (1.7, -1.0, 0.0, 0.0),
    (1.7, 1.0, 0.0, 0.0),
    (1.7, 0.0, -1.0, 0.0),
    (1.7, 0.0, 1.0, 0.0),
    (1.7, 0.0, 0.0, -1.0),
    (1.7, 0.0, 0.0, 1.0),
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
        let gazing = clip.tracks.iter().any(|track| track.target == Target::Gaze);
        let bowing = clip
            .tracks
            .iter()
            .any(|track| track.target == Target::Trunk);
        let carrying = clip
            .tracks
            .iter()
            .any(|track| matches!(track.target, Target::Root | Target::Tilt));
        println!(
            "\n{:>7} {:>5} {:>5} {:>5} {:>7} {:>6} {:>7} {:>7} {:>7} {:>8} {:>7} {:>8} {:>8} \
             {:>7} {:>8} {:>7} {:>7} {:>7} {:>8} {:>8} {:>8} {:>8} {:>7}",
            "stature",
            "limbs",
            "neck",
            "head",
            "reach m",
            "strain",
            "up",
            "forward",
            "across",
            "clear mm",
            "trunk",
            "elbow mm",
            "palm deg",
            "nod deg",
            "chest deg",
            "gaze mm",
            "bow deg",
            "girdle",
            "root/leg",
            "knee deg",
            "tilt deg",
            "under mm",
            "step mm"
        );

        let mut spread: Vec<(f32, f32, f32)> = Vec::new();
        let mut dipped: Vec<f32> = Vec::new();
        let mut bowed: Vec<f32> = Vec::new();
        let mut kneed: Vec<f32> = Vec::new();
        for (stature, limbs, neck, headsize) in BODIES {
            let rig = body(stature, limbs, neck, headsize);
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
            // Every joint the gaze could have recruited that is NOT the neck
            // or the head: what separates a nod from a bow is whether these
            // moved, so they are read separately rather than folded into the
            // pitch.
            let chest = rig.in_zone(Zone::Chest);
            // The trunk's chord: the pelvis to the girdle the neck hangs off.
            // The joints between it are the ones that swing, so neither of its
            // ends is one of them.
            let chord = rig
                .in_zone(Zone::Neck)
                .first()
                .and_then(|&neck| rig.joints[neck].parent)
                .and_then(|girdle| {
                    let root = rig.joints.iter().position(|joint| joint.parent.is_none())?;
                    Some((root, girdle))
                });
            let trunk: Vec<usize> = [Zone::Chest, Zone::Abdomen, Zone::Pelvis]
                .into_iter()
                .flat_map(|zone| rig.in_zone(zone))
                .collect();
            let arms: Vec<[usize; 3]> = [Limb::ForeLeft, Limb::ForeRight]
                .into_iter()
                .filter_map(|limb| rig.limb_chain(limb))
                .collect();
            // The palm's rest normal, per hand: the hand builder's own frame,
            // re-derived — fingers curl away from world up projected off the
            // wrist bone, so the palm faces the other way. The same expression
            // `Track::facing` aims, so this reading and that control agree
            // about what a palm is.
            // Only the hands the clip actually addresses: a right-hand wave
            // leaves the left palm at rest, and a max over both would report
            // the rest pose's 90 degrees as the gesture's failing.
            let addressed: Vec<Limb> = clip
                .tracks
                .iter()
                .flat_map(|track| track.target.resolve(&rig))
                .collect();
            let palms: Vec<(usize, Vec3)> = addressed
                .into_iter()
                .filter_map(|limb| {
                    let contact = *rig.in_zone(Zone::Extremity(limb)).first()?;
                    let parent = rig.joints[contact].parent?;
                    let out = (rig.joints[contact].position - rig.joints[parent].position)
                        .normalize_or_zero();
                    let flat = -(Vec3::Y - out * out.dot(Vec3::Y)).normalize_or_zero();
                    Some((contact, flat))
                })
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
            // The highest any elbow gets above its own shoulder, and the
            // furthest any palm points from forward at mid-gesture.
            let mut elbow_over = f32::MIN;
            let mut palm_off = 0.0f32;
            // The gaze's three: how far the facing pitched below rest, how much
            // of it the chest took, and how far the head joint travelled. A nod
            // that translates the head is a body leaning in, not agreeing.
            let mut dip = 0.0f32;
            let mut chest_dip = 0.0f32;
            let mut head_moved = 0.0f32;
            let mut inclined = 0.0f32;
            // **How far the shoulder girdle itself turns, which is not the
            // chord's angle.** A trunk pitched at one hinge above a pelvis that
            // cannot move turns the whole segment above that hinge, and the
            // chord — a length-weighted mix of the still stub and the turned
            // segment — arrives at a fraction of it. The gait's constant lives
            // on the chord and its lean is small enough that nobody had to look
            // at the other number; a 30 degree bow is not.
            let mut girdle_turn = 0.0f32;
            // The carriage's four: how far the root fell as a share of the leg
            // that holds it up, the angle that left at the knee, how far the
            // body tipped, and how near the floor its lowest joint came.
            let mut fell = 0.0f32;
            let mut knee_angle = 180.0f32;
            let mut tipped = 0.0f32;
            let mut floor = f32::MAX;
            // The same reading taken on the body standing, which is what the
            // clip's is compared against.
            let standing = rest.iter().enumerate().fold(f32::MAX, |low, (joint, at)| {
                let (near, far) = rig.bone_radii(joint);
                low.min(at.y - near.max(far))
            });
            let leg = rig
                .ground_contacts()
                .into_iter()
                .filter_map(|limb| rig.limb_reach(limb))
                .fold(0.0f32, f32::max);
            let knees: Vec<[usize; 3]> = rig
                .ground_contacts()
                .into_iter()
                .filter_map(|limb| rig.limb_chain(limb))
                .collect();
            let root = rig
                .joints
                .iter()
                .position(|joint| joint.parent.is_none())
                .expect("a rig has a root");
            let rested = Pose::rest(&rig).forward(&rig).rotations;

            for frame in 0..=samples {
                let time = frame as f32 / samples as f32;
                let mut pose = Pose::rest(&rig);
                strained += clip.apply(&rig, &mut pose, time).len();
                let posed = pose.forward(&rig);
                let places = posed.positions;
                for &[shoulder, elbow, _] in &arms {
                    elbow_over = elbow_over.max(places[elbow].y - places[shoulder].y);
                }
                if gazing {
                    dip = dip.max(pitch_between(
                        rested[head] * landmark::FORWARD,
                        posed.rotations[head] * landmark::FORWARD,
                    ));
                    for &joint in &chest {
                        chest_dip = chest_dip.max(
                            pitch_between(
                                rested[joint] * landmark::FORWARD,
                                posed.rotations[joint] * landmark::FORWARD,
                            )
                            .abs(),
                        );
                    }
                    head_moved = head_moved.max(places[head].distance(rest[head]));
                }
                if bowing && let Some((root, girdle)) = chord {
                    inclined = inclined.max(pitch_between(
                        rest[girdle] - rest[root],
                        places[girdle] - places[root],
                    ));
                    girdle_turn = girdle_turn.max(pitch_between(
                        rested[girdle] * landmark::FORWARD,
                        posed.rotations[girdle] * landmark::FORWARD,
                    ));
                }
                if carrying {
                    fell = fell.max(-pose.translation.y);
                    tipped = tipped.max(
                        (posed.rotations[root] * Vec3::Y)
                            .angle_between(Vec3::Y)
                            .to_degrees(),
                    );
                    // **The lowest SURFACE, not the lowest joint.** A joint is
                    // a point on an axis and the body hangs off it by the bone's
                    // own radius, so a pose whose lowest JOINT rests at zero has
                    // its back through the floor by half a torso. The radius
                    // overstates it a little — subdivision pulls the mesh inside
                    // the node — and overstating a clearance is the safe way to
                    // be wrong about one.
                    floor = floor.min(places.iter().enumerate().fold(
                        f32::MAX,
                        |low, (joint, at)| {
                            let (near, far) = rig.bone_radii(joint);
                            low.min(at.y - near.max(far))
                        },
                    ));
                    for &[hip, knee, ankle] in &knees {
                        let thigh = (places[hip] - places[knee]).normalize_or_zero();
                        let shin = (places[ankle] - places[knee]).normalize_or_zero();
                        knee_angle = knee_angle.min(thigh.angle_between(shin).to_degrees());
                    }
                }
                if frame == samples / 2 {
                    for &(contact, flat) in &palms {
                        let showing = posed.rotations[contact] * flat;
                        palm_off = palm_off.max(showing.angle_between(Vec3::Z).to_degrees());
                    }
                }
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
            // Blank rather than zero wherever the clip does not address the
            // part: a nod moves no hand and a wave turns no head, and printing
            // a 0 for either invites reading it as a measurement.
            let handed = !palms.is_empty();
            if handed {
                spread.push(most);
            }
            let reading = |value: f32| handed.then_some(value);
            let gaze = |value: f32| gazing.then_some(value);
            if gazing {
                dipped.push(dip);
            }
            if bowing {
                bowed.push(inclined);
            }
            if carrying {
                kneed.push(knee_angle);
            }
            let carried = |value: f32| carrying.then_some(value);
            println!(
                "{stature:>7.2} {limbs:>5.1} {neck:>5.1} {headsize:>5.1} {arm:>7.3} \
                 {strained:>6} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {step_mm:>7.1}",
                maybe(reading(most.0), 7, 3),
                maybe(reading(most.1), 7, 3),
                maybe(reading(most.2), 7, 3),
                maybe(reading(nearest_head * 1000.0), 8, 0),
                maybe(reading(nearest_trunk * 1000.0), 7, 0),
                maybe(reading(elbow_over * 1000.0), 8, 0),
                maybe(reading(palm_off), 8, 0),
                maybe(gaze(dip), 7, 1),
                maybe(gaze(chest_dip), 8, 1),
                maybe(gaze(head_moved * 1000.0), 7, 0),
                maybe(bowing.then_some(inclined), 7, 1),
                maybe(bowing.then_some(girdle_turn), 7, 1),
                maybe(carried(fell / leg.max(f32::EPSILON)), 8, 3),
                maybe(carried(knee_angle), 8, 1),
                maybe(carried(tipped), 8, 1),
                maybe(carried((floor - standing) * 1000.0), 8, 0),
                step_mm = step * 1000.0,
            );
        }

        // Only what the clip addressed gets a spread. A nod moves no hand, and
        // "the hand's rise varies by 0.000" is a sentence that reads as a
        // result rather than as an absence.
        if !spread.is_empty() {
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
        if !bowed.is_empty() {
            let (low, high) = bowed.iter().fold((f32::MAX, f32::MIN), |range, at| {
                (range.0.min(*at), range.1.max(*at))
            });
            println!(
                "\n  trunk spread: the chord's inclination varies by {:.3} of a degree across the \
                 sweep, {low:.2} to {high:.2}",
                high - low
            );
        }
        if !kneed.is_empty() {
            let (low, high) = kneed.iter().fold((f32::MAX, f32::MIN), |range, at| {
                (range.0.min(*at), range.1.max(*at))
            });
            println!(
                "\n  carriage spread: the knee closes to between {low:.2} and {high:.2} degrees \
                 across the sweep, a spread of {:.3}",
                high - low
            );
            println!(
                "          (a carriage is stated in the LEG, because what holds a root up is the \
                 leg — the same drop stated in body heights drifts 0.044 of a body across this \
                 sweep. The knee is where that shows: sitting is a right angle at it)"
            );
        }
        if !dipped.is_empty() {
            let (low, high) = dipped.iter().fold((f32::MAX, f32::MIN), |range, at| {
                (range.0.min(*at), range.1.max(*at))
            });
            println!(
                "\n  gaze spread: the head's pitch varies by {:.3} of a degree across the sweep, \
                 {low:.2} to {high:.2}",
                high - low
            );
            println!(
                "          (the spread above is a DISPLACEMENT and cannot judge a rotation. This \
                 is the same claim in the unit the gesture is actually stated in: a nod written \
                 as an angle is the same nod on every neck, and one written as a place is not)"
            );
        }
    }
    println!(
        "\nhow it READS is not in this table. `cargo run --release -F builtin-clips --example \
         viewer -- --gesture Greeting`, in the sibling `bevy_symbios_avatar`."
    );
}

/// A humanoid rig of the given stature and proportions.
fn body(height: f32, limbs: f32, neck: f32, head: f32) -> Rig {
    let params = HumanoidParams {
        height,
        limb_length: limbs,
        neck_length: neck,
        head_size: head,
        ..HumanoidParams::default()
    };
    Rig::from_skeleton(&params.skeleton(&Composites::default())).expect("the plan builds a rig")
}

/// How far `to` sits below `from`, in degrees, measuring each direction's own
/// inclination rather than the angle between them.
///
/// **Inclination rather than the arc**, because a nod's claim is about pitch:
/// the angle between two directions is positive whichever way the second one
/// went, and a head that lifted would read the same as one that dropped.
fn pitch_between(from: Vec3, to: Vec3) -> f32 {
    let angle = |run: Vec3| run.y.atan2(run.z.hypot(run.x));
    (angle(from) - angle(to)).to_degrees()
}

/// A reading a body may not have, formatted for the table.
fn maybe(value: Option<f32>, width: usize, places: usize) -> String {
    match value {
        Some(value) => format!("{value:>width$.places$}"),
        None => format!("{:>width$}", "-"),
    }
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
