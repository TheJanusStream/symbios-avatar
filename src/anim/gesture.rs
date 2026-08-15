//! The expressive roster, written as goal-space clips rather than baked angles.
//!
//! Every gesture here is a [`Clip`]: a few keyed goals in normalised body space
//! and the timing between them. Nothing is stored per body, so a gesture costs
//! the bytes of its own description and reads on a body that did not exist when
//! it was written — which is the whole point of the format and the whole reason
//! the baked roster can go (#237, #248).
//!
//! # What a gesture is allowed to say
//!
//! [`Clip`] addresses **limbs**, by where their extremity should be, measured
//! in multiples of that limb's own reach. That is enough for a gesture made of
//! arms and not enough for one made of anything else, and the roster splits
//! cleanly along that line:
//!
//! * expressible here and now — the **wave** and the **refusal**, which are
//!   hands and only hands;
//! * needing the head — the **nod**, and the gaze drop that finishes a bow;
//! * needing the trunk — the **bow**;
//! * needing the whole carriage — **sitting** and **sleeping**, which move the
//!   pelvis and fold the legs rather than reaching anywhere.
//!
//! Two more of the roster are already procedural and want wiring rather than
//! authoring: [`IdleConfig::talking`] and [`IdleConfig::listening`] are the
//! talking and listening idles, and have been since #246.
//!
//! [`IdleConfig::talking`]: super::IdleConfig::talking
//! [`IdleConfig::listening`]: super::IdleConfig::listening
//!
//! # How a gesture is judged
//!
//! `examples/gestureaudit`, on a sweep of bodies from 1.2 to 2.2 m — and the
//! reading that matters is not any one figure but the **spread** of it across
//! that sweep. A gesture written in reaches should put the hand in the same
//! place on every body, as a fraction of that body; a figure that drifts with
//! stature is a goal that is normalised in name only.

use glam::Vec3;

use super::clip::{Clip, Key, Target, Track};
use crate::plan::Limb;

/// How far above its own rest position a raised hand goes, in body heights.
///
/// **Measured on the body, not on the arm** ([`super::Scale::Body`]), because a wave
/// is a hand beside the head and a head is where it is whatever the arm's
/// length. Stated in reaches instead, the audit put the same gesture 0.115 of a
/// body height apart between the short-limbed and long-limbed ends of the
/// sweep — a hand at the ear on one body and over the crown on the other.
///
/// The number is the anatomy of a CASUAL wave, which is a forearm gesture: the
/// hand stops at jaw height — about 0.06 above the shoulder, where the contact
/// hangs 0.221 below it at rest — with the elbow staying down (see
/// [`wave`]'s bend). Raised to ear height and above, the same gesture needs the
/// whole arm and reads as hailing rescue.
pub const RAISE: f32 = 0.284;

/// How far IN from its rest position a raised hand comes, in body heights.
///
/// A hanging arm on this plan rests 0.263 of a body height out from the
/// shoulder; a casual wave brings the hand in to about 0.14 — still clearly
/// out to the side, past the head rather than in front of it, which is what
/// keeps the elbow low and the face clear.
pub const RAISE_IN: f32 = 0.119;

/// How far forward a raised hand is held, in body heights.
///
/// A touch: enough that the hand reads to someone in front of the body without
/// turning the wave into a reach toward them.
pub const RAISE_FORWARD: f32 = 0.04;

/// How wide the wave itself is, in body heights, each way from the raise.
///
/// The gesture is the oscillation, not the raise: an arm held up and still is
/// a body hailing a taxi. Along the body's lateral axis, which on a standing
/// body is world horizontal — a casual wave swings the hand side to side, it
/// does not bob it.
pub const WAVE: f32 = 0.05;

/// How many times a greeting waves.
///
/// Three, which is what a greeting is: one is a flick, and five is a signal to
/// someone far away.
pub const WAVES: usize = 3;

/// Share of a greeting spent lifting the arm, and the same again putting it
/// down.
///
/// **Both ends matter and the second one is the one that gets forgotten.** A
/// gesture that ends with the arm still up leaves the body in a pose it never
/// chose, and a caller blending out of it hides that by accident rather than on
/// purpose.
pub const REACH_TIME: f32 = 0.18;

/// Which way a gesturing arm's elbow points: down, a little out, a little
/// forward.
///
/// **The elbow is half the difference between a wave and a stretch.** The
/// solver's default pole for an arm is the plan's — backward — and a hand
/// raised beside the head with a backward pole gets an elbow flared up level
/// with the shoulder. Pointing it down keeps the upper arm hanging and makes
/// the raise a forearm's, which is what a casual gesture is.
///
/// Written for the body's left arm; [`wave`] and [`reject`] mirror the lateral
/// component per side, the same way they mirror their offsets.
pub const ELBOW_DOWN: Vec3 = Vec3::new(0.35, -1.0, 0.1);

/// Where every gesturing palm faces: forward, tilted a little up.
///
/// An open palm shown to the other person is the whole difference between a
/// greeting and a salute, and between a refusal and a shove. The tilt keeps
/// the hand from reading as a traffic stop.
pub const PALM_FORWARD: Vec3 = Vec3::new(0.0, 0.25, 1.0);

/// How high a refusing hand is held, in body heights above its rest position.
///
/// Chest height — about 0.06 below the shoulder, like the wave one gesture
/// over: a refusal is made in front of the body where it can be seen, and not
/// at the hip.
pub const PUSH_UP: f32 = 0.161;

/// How far in from its rest position a refusing hand comes, in body heights.
///
/// **In front of the body, not beside it** — the hands land about 0.07 of a
/// body height either side of the midline, just inside their own shoulders,
/// because a defence is made between yourself and the thing declined. The
/// first cut left them at the torso's flanks (0.10 outside the shoulder) and
/// the render read as a fitness exercise; the palms cannot refuse anything
/// from out there.
pub const PUSH_IN: f32 = 0.30;

/// How far forward a refusing hand pushes, in body heights, and where it sits
/// between pushes.
///
/// Close. The first draft pushed to 0.17 of a body height with the palms down
/// and read as a fitness exercise; a defensive refusal keeps the hands near
/// the chest and lets the palms do the refusing.
pub const PUSH: f32 = 0.20;

/// Where a refusing hand sits between pushes, in body heights forward.
pub const PUSH_READY: f32 = 0.14;

/// How many times a refusal pushes.
///
/// Two. A refusal is emphatic and brief; a third push is a body arguing.
pub const PUSHES: usize = 2;

/// A greeting: one hand raised and waved, and put down again.
///
/// **One hand, and only where there is a hand free to raise** — see
/// [`Target::Grasper`]. A body walking on all four of its limbs makes no
/// greeting at all, which is the right answer rather than a missing case: this
/// clip resolves to nothing and leaves it alone.
///
/// Not looping. A greeting has a beginning and an end, and both are the body's
/// own rest pose, so a caller can play it over anything and get the body back.
#[must_use]
pub fn wave(hand: Limb) -> Clip {
    let side = if hand.is_left() { 1.0 } else { -1.0 };
    let raised = Vec3::new(-side * RAISE_IN, RAISE, RAISE_FORWARD);
    let mut keys = vec![Key::new(0.0, Vec3::ZERO)];
    // The oscillation, sampled at its turning points: a lerp between two
    // extremes IS a triangle wave, and a hand waving is nearer a triangle than
    // a sine — it turns sharply at each end and travels at speed between them.
    let span = 1.0 - 2.0 * REACH_TIME;
    for step in 0..=(WAVES * 2) {
        let at = REACH_TIME + span * step as f32 / (WAVES * 2) as f32;
        let swing = if step % 2 == 0 { WAVE } else { -WAVE };
        keys.push(Key::new(at, raised + Vec3::X * (side * swing)));
    }
    keys.push(Key::new(1.0, Vec3::ZERO));
    let bend = Vec3::new(side * ELBOW_DOWN.x, ELBOW_DOWN.y, ELBOW_DOWN.z);
    Clip::new([Track::new(Target::Grasper(hand), keys)
        .on_body()
        .bending_toward(bend)
        .facing(PALM_FORWARD)])
}

/// A refusal: both hands up and pushed away, twice.
///
/// **Both hands, because that is what the gesture is** — a refusal made with one
/// hand is a wave-off, and this is the emphatic one. Written as one track per
/// hand rather than one over [`Target::Graspers`], because the two hands are
/// held apart and a track carries one offset for every limb it resolves to; a
/// body with one arm still refuses with the one it has.
///
/// What is missing is the head. The gesture people actually make is a shake as
/// well as a push, and the head is not something this format can address yet —
/// so this is the hands' half of it, and saying so is better than pretending the
/// roster item is finished.
#[must_use]
pub fn reject() -> Clip {
    Clip::new([push_with(Limb::ForeLeft), push_with(Limb::ForeRight)])
}

/// One hand's half of a refusal.
fn push_with(hand: Limb) -> Track {
    let side = if hand.is_left() { 1.0 } else { -1.0 };
    let ready = Vec3::new(-side * PUSH_IN, PUSH_UP, PUSH_READY);
    let pushed = Vec3::new(-side * PUSH_IN, PUSH_UP, PUSH);
    let mut keys = vec![Key::new(0.0, Vec3::ZERO)];
    let span = 1.0 - 2.0 * REACH_TIME;
    for step in 0..PUSHES {
        let at = REACH_TIME + span * step as f32 / PUSHES as f32;
        keys.push(Key::new(at, ready));
        keys.push(Key::new(at + span / PUSHES as f32 * 0.5, pushed));
    }
    keys.push(Key::new(1.0 - REACH_TIME, ready));
    keys.push(Key::new(1.0, Vec3::ZERO));
    let bend = Vec3::new(side * ELBOW_DOWN.x, ELBOW_DOWN.y, ELBOW_DOWN.z);
    Track::new(Target::Grasper(hand), keys)
        .on_body()
        .bending_toward(bend)
        .facing(PALM_FORWARD)
}

/// Whether a gesture is one a body at rest can hold, or one it plays and leaves.
///
/// Every gesture in this module is the second kind, and both ends of every one
/// of them is the body's own rest pose — see [`REACH_TIME`].
#[must_use]
pub fn returns_to_rest(clip: &Clip) -> bool {
    !clip.looping
        && clip.tracks.iter().all(|track| {
            let ends = (track.keys.first(), track.keys.last());
            matches!(ends, (Some(first), Some(last))
                if first.offset == Vec3::ZERO && last.offset == Vec3::ZERO)
        })
}

/// Every gesture this module can build, by name.
///
/// The names are the baked roster's, so a caller swapping a procedural gesture
/// in for a baked one can look it up by the name it already has (#248).
#[must_use]
pub fn by_name(name: &str) -> Option<Clip> {
    match name {
        "Greeting" => Some(wave(Limb::ForeRight)),
        "Reject" => Some(reject()),
        _ => None,
    }
}

/// The roster this module covers, in the baked set's own names.
pub const ROSTER: &[&str] = &["Greeting", "Reject"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Pose;
    use crate::plan::{BodyPlan, Composites, HumanoidParams, QuadrupedParams, Zone};
    use crate::rig::Rig;

    /// The bodies every gesture is judged on: a stature in metres and a limb
    /// length against the torso.
    ///
    /// **The limb proportion is the half that tests anything.** The plan scales
    /// a body uniformly with stature, so a goal in any unit lands in the same
    /// relative place by arithmetic; only a change of proportion moves the
    /// shoulder, the reach and the hand's rest position against each other.
    const BODIES: [(f32, f32); 4] = [(1.2, 0.0), (2.2, 0.0), (1.7, -1.0), (1.7, 1.0)];

    /// How many points of a gesture the tests sample.
    const SWEEP: usize = 60;

    fn body(height: f32, limbs: f32) -> Rig {
        let params = HumanoidParams {
            height,
            limb_length: limbs,
            ..HumanoidParams::default()
        };
        Rig::from_skeleton(&params.skeleton(&Composites::default())).expect("the plan builds a rig")
    }

    /// How far a gesture moves the hand at its furthest, as a fraction of the
    /// body's own height.
    fn furthest(rig: &Rig, clip: &Clip) -> Vec3 {
        let height = rig.extent();
        // **The joint the clip drives**, which is not the one `extremity_joints`
        // names first — that list carries the wrist at its head and `Clip`
        // places the contact past it. Measured against the wrist this read a
        // wave landing 117 mm from where it was sent, with no strain reported,
        // because the wrist hangs off the joint that arrived. The gait
        // documented the same two lists and the same trap for the ankle.
        let hands: Vec<usize> = [Limb::ForeLeft, Limb::ForeRight]
            .into_iter()
            .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
            .collect();
        let rest = Pose::rest(rig).forward(rig).positions;
        (0..=SWEEP).fold(Vec3::ZERO, |most, frame| {
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, frame as f32 / SWEEP as f32);
            let places = pose.forward(rig).positions;
            hands.iter().fold(most, |most, &hand| {
                let moved = (places[hand] - rest[hand]) / height;
                Vec3::new(
                    most.x.max(moved.x.abs()),
                    most.y.max(moved.y),
                    most.z.max(moved.z),
                )
            })
        })
    }

    #[test]
    fn a_gesture_lands_in_the_same_place_on_every_body() {
        // **The claim the whole format is for** (#248): a motion described by
        // normalised goals reads the same on a body that did not exist when it
        // was written. Asked as a SPREAD across bodies rather than as a figure
        // on one, because a single body cannot disagree with itself.
        //
        // Measured with the wave stated in arm reaches, which is what it said
        // first: 0.115 of a body height between the short-limbed and long-limbed
        // ends of the sweep — a hand at the ear on one body and over the crown on
        // the other. A wave is a hand beside the head, and a head is where it is
        // whatever the arm's length, so the goal belongs on the body.
        //
        // **Reproducing it needs the constants as well as the unit.** Flipping
        // today's numbers back to `Scale::Reach` and leaving them otherwise
        // alone gives 0.028, because they were re-derived for the body when the
        // unit changed. That still fails this, which is the point; the 0.115 is
        // what the gesture as originally authored actually read.
        for clip in [wave(Limb::ForeRight), reject()] {
            let reached: Vec<Vec3> = BODIES
                .iter()
                .map(|&(height, limbs)| furthest(&body(height, limbs), &clip))
                .collect();
            let spread = |pick: fn(Vec3) -> f32| {
                let (low, high) = reached.iter().fold((f32::MAX, f32::MIN), |range, at| {
                    (range.0.min(pick(*at)), range.1.max(pick(*at)))
                });
                high - low
            };
            for (axis, spread) in [
                ("rise", spread(|at| at.y)),
                ("forward reach", spread(|at| at.z)),
                ("swing", spread(|at| at.x)),
            ] {
                // **Five thousandths, and the true figure is zero.** The goal
                // is met exactly on every body — the solve lands the contact on
                // it to well under a millimetre — so any drift at all is the
                // goal itself moving with the body, which is the defect. The
                // slack is for the solve, not for the format.
                assert!(
                    spread < 0.005,
                    "the gesture's {axis} varied by {spread:.3} of a body height across the sweep",
                );
            }
        }
    }

    #[test]
    fn no_gesture_asks_for_a_place_the_arm_cannot_reach() {
        // A goal outside the arm's own sphere is a gesture the body plays with
        // its elbow locked, pointing at where it was asked to be. `Clip::apply`
        // reports it rather than failing, which means nothing notices unless
        // something asks — and the small and short-limbed bodies of the sweep
        // are where it happens.
        //
        // Measured on the first draft, which raised the hand 1.15 arm-reaches
        // and carried it outward as it rose: the goal sat 63 mm outside the
        // sphere and half the frames of the gesture strained, on every body.
        for clip in [wave(Limb::ForeRight), reject()] {
            for &(height, limbs) in &BODIES {
                let rig = body(height, limbs);
                let strained: usize = (0..=SWEEP)
                    .map(|frame| {
                        let mut pose = Pose::rest(&rig);
                        clip.apply(&rig, &mut pose, frame as f32 / SWEEP as f32)
                            .len()
                    })
                    .sum();
                assert_eq!(
                    strained, 0,
                    "a {height} m body with limb length {limbs} strained in {strained} frames",
                );
            }
        }
    }

    #[test]
    fn a_gesture_gives_the_body_back() {
        // Both ends of every gesture here are the body's own rest pose, so a
        // caller can play one over anything and get the body back. A gesture
        // that ends with the arm still up leaves the body in a pose it never
        // chose, and a caller blending out of it hides that by accident.
        for clip in [wave(Limb::ForeLeft), wave(Limb::ForeRight), reject()] {
            assert!(returns_to_rest(&clip));
            for &(height, limbs) in &BODIES {
                let rig = body(height, limbs);
                let rest = Pose::rest(&rig).forward(&rig).positions;
                for time in [0.0, 1.0] {
                    let mut pose = Pose::rest(&rig);
                    clip.apply(&rig, &mut pose, time);
                    let places = pose.forward(&rig).positions;
                    let worst = places
                        .iter()
                        .zip(&rest)
                        .fold(0.0f32, |most, (a, b)| most.max(a.distance(*b)));
                    // **Five millimetres, and the slack is the SOLVE's, not the
                    // gesture's.** A key of zero asks the limb for exactly where
                    // it already is, and putting a limb back where it is turns
                    // out not to be free: the solve reaches the goal but settles
                    // the bend on the pole's plane rather than on the rest
                    // arm's, which moves the elbow a little and the hand less.
                    // Measured across the sweep it is under 2 mm and it is the
                    // same on every gesture, being a property of
                    // `solve_contact_toward` rather than of any of them (#262).
                    //
                    // What this bound is for is tens of millimetres: an arm left
                    // up, a hand left out, a gesture with no closing key.
                    assert!(
                        worst < 0.005,
                        "at time {time} a {height} m body with limb length {limbs} sat {:.1} mm \
                         from rest",
                        worst * 1000.0,
                    );
                }
            }
        }
    }

    #[test]
    fn a_gesturing_elbow_stays_below_its_shoulder() {
        // **The elbow is half the difference between a wave and a stretch**
        // (#248, owner's eye). The contact goal fixes the hand and says nothing
        // about the middle of the limb; the solver's default pole for an arm is
        // backward, and a hand raised beside the head with a backward pole
        // flares the elbow up level with the shoulder — measured, 7 mm ABOVE it
        // on the smallest body, and the flare is what read as a stretch long
        // before it crossed the line. [`Track::bending`] pointed down is what
        // holds it low, and with it the same elbow reads 106 to 200 mm below
        // across the whole sweep. The zero bound is honest for the two gestures
        // that exist: both keep the whole upper arm hanging, so an elbow even
        // AT shoulder height means the pole has stopped doing its work.
        for clip in [wave(Limb::ForeRight), reject()] {
            for &(height, limbs) in &BODIES {
                let rig = body(height, limbs);
                let addressed: Vec<Limb> = clip
                    .tracks
                    .iter()
                    .flat_map(|track| track.target.resolve(&rig))
                    .collect();
                for frame in 0..=SWEEP {
                    let mut pose = Pose::rest(&rig);
                    clip.apply(&rig, &mut pose, frame as f32 / SWEEP as f32);
                    let places = pose.forward(&rig).positions;
                    for &limb in &addressed {
                        let [shoulder, elbow, _] = rig.limb_chain(limb).expect("an arm");
                        assert!(
                            places[elbow].y < places[shoulder].y,
                            "on a {height} m body with limb length {limbs} the elbow rose \
                             {:.0} mm above the shoulder",
                            (places[elbow].y - places[shoulder].y) * 1000.0,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_gesture_shows_its_palm_with_the_fingers_up() {
        // **A palm is half a gesture's meaning** (#248, owner's eye): a raised
        // hand palm-forward is a greeting, edge-on it is a salute, and the
        // refusal with the palms anywhere else is a fitness exercise. Two
        // separate claims, because they fail separately: the NORMAL is what
        // [`Track::facing`] aims, and the FINGERS are the roll about it, which
        // the minimal arc leaves wherever the arm's configuration happens to
        // put it — measured, palms correctly forward with the fingers pointing
        // at each other, a body presenting its chest.
        for clip in [wave(Limb::ForeRight), reject()] {
            for &(height, limbs) in &BODIES {
                let rig = body(height, limbs);
                let addressed: Vec<Limb> = clip
                    .tracks
                    .iter()
                    .flat_map(|track| track.target.resolve(&rig))
                    .collect();
                let mut pose = Pose::rest(&rig);
                clip.apply(&rig, &mut pose, 0.5);
                let posed = pose.forward(&rig);
                for &limb in &addressed {
                    let contact = *rig.in_zone(Zone::Extremity(limb)).first().expect("a hand");
                    let parent = rig.joints[contact].parent.expect("a wrist");
                    let out = (rig.joints[contact].position - rig.joints[parent].position)
                        .normalize_or_zero();
                    let flat = -(Vec3::Y - out * out.dot(Vec3::Y)).normalize_or_zero();
                    let showing = posed.rotations[contact] * flat;
                    let off = showing.angle_between(Vec3::Z).to_degrees();
                    assert!(
                        off < 30.0,
                        "on a {height} m body with limb length {limbs} the palm faced {off:.0} \
                         degrees away from forward",
                    );
                    let fingers = posed.rotations[contact] * out;
                    assert!(
                        fingers.y > 0.5,
                        "on a {height} m body with limb length {limbs} the fingers pointed \
                         ({:.2}, {:.2}, {:.2}) instead of up",
                        fingers.x,
                        fingers.y,
                        fingers.z,
                    );
                }
            }
        }
    }

    #[test]
    fn a_body_that_walks_on_its_hands_makes_no_greeting() {
        // **The per-item refusal the roster asks for** (#248), and it needs no
        // special path: [`Target::Grasper`] resolves to nothing on a body with
        // no free limb, a track that finds nothing does nothing, and a clip of
        // such tracks leaves the body alone.
        //
        // The trap this guards is the obvious alternative. `Target::Just` names
        // a limb whatever it is for — a march lifts a limb the body stands on,
        // so it has to — and a one-handed wave written with it waves a
        // quadruped's front leg at you.
        let quadruped =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
                .expect("the plan builds a quadruped");
        let rest = Pose::rest(&quadruped).forward(&quadruped).positions;
        for clip in [wave(Limb::ForeLeft), wave(Limb::ForeRight), reject()] {
            for frame in 0..=SWEEP {
                let mut pose = Pose::rest(&quadruped);
                clip.apply(&quadruped, &mut pose, frame as f32 / SWEEP as f32);
                let places = pose.forward(&quadruped).positions;
                let worst = places
                    .iter()
                    .zip(&rest)
                    .fold(0.0f32, |most, (a, b)| most.max(a.distance(*b)));
                assert!(
                    worst < 1e-6,
                    "a gesture moved a quadruped by {:.1} mm",
                    worst * 1000.0,
                );
            }
        }
    }
}
