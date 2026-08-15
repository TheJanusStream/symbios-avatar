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
//! in multiples of that limb's own reach; a **gaze**, by the angle it turns
//! through; and a **trunk**, by the angle it inclines through. Two of those
//! three arrived because a gesture here needed them, and each is an angle
//! because what it names is a rotation. The roster splits along that line:
//!
//! * hands and only hands — the **wave** and the **refusal**;
//! * a gaze and only a gaze — the **nod**;
//! * a trunk and a gaze — the **bow**;
//! * the whole carriage — **sitting** and **sleeping**, which move the pelvis
//!   and fold the legs rather than reaching or turning anywhere, and which are
//!   the part of the roster this module cannot say yet.
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
//!
//! **Which spread depends on what the gesture is made of, and the sweep has to
//! be able to see the part** (#248). A reach is judged by displacement, and the
//! axis that moves a displacement is limb proportion. A nod is a rotation, so
//! displacement says nothing about it — it is judged by the ANGLE it delivers,
//! and the axes that move an angle are the neck's length and the head's size.
//! Neither was in the sweep until a nod needed them, and across limb proportion
//! alone the neck stays 0.250 of a body's height on every body: the reading
//! would have been a flat zero for any authoring at all.

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

/// How far a nod drops the gaze, as a tangent — see [`Key::offset`].
///
/// **Derived from the joint it drives, not chosen for it.** A clip's gaze is
/// the neck's and the head's, and [`gaze_config`] clamps that chain at 0.9
/// radians because that is about where cervical flexion runs out. A nod of
/// agreement is a third of the neck's own range — clear at a distance, and
/// nowhere near the end stop, so it reads as a nod rather than as a body
/// running out of neck. That is 0.3 radians, or 17.2 degrees, and this is its
/// tangent because a gaze key's offset is one.
///
/// A number with its argument written down: if [`gaze_config`]'s limit moves,
/// this is wrong, and `a_nod_is_a_third_of_the_neck_it_has` says so.
///
/// [`gaze_config`]: super::clip::gaze_config
/// [`Key::offset`]: super::Key::offset
pub const NOD_DIP: f32 = 0.309_336_2;

/// How many times a nod dips.
///
/// Two. One dip is a twitch that a viewer half-catches, and three is a body
/// agreeing rather too hard — the same argument [`PUSHES`] makes for the
/// refusal, one gesture over.
pub const NODS: usize = 2;

/// Share of one dip spent going down, the rest coming back up.
///
/// **Under a half, because a nod falls faster than it recovers.** The drop is
/// the gesture and the return is the body getting ready to make it again; even
/// timing reads as a metronome rather than as agreement.
pub const NOD_FALL: f32 = 0.45;

/// A nod: the gaze dips and comes back level, twice.
///
/// **A rotation rather than a place, and the whole of the gesture is in the
/// angle** — see [`Target::Gaze`] for the measurement that settled that, and
/// [`NOD_DIP`] for where the angle comes from. Nothing else moves: the trunk
/// staying out of it is what keeps this a nod rather than a small bow, and it
/// is [`gaze_config`]'s convention rather than anything written here.
///
/// **Only where there is a head to nod.** A body without one resolves to
/// nothing and is left alone, the same per-item refusal [`Target::Grasper`]
/// makes for a body with no free hand.
///
/// Not looping, and both ends are level, so a caller can play it over anything
/// and get the body's own gaze back.
///
/// [`gaze_config`]: super::clip::gaze_config
/// [`Key::offset`]: super::Key::offset
#[must_use]
pub fn nod() -> Clip {
    let dip = Vec3::new(0.0, -NOD_DIP, 0.0);
    let unit = 1.0 / NODS as f32;
    let mut keys = vec![Key::new(0.0, Vec3::ZERO)];
    for step in 0..NODS {
        let base = step as f32 * unit;
        keys.push(Key::new(base + unit * NOD_FALL, dip));
        keys.push(Key::new(base + unit, Vec3::ZERO));
    }
    Clip::new([Track::new(Target::Gaze, keys)])
}

/// How far a bow inclines the trunk, in quarter turns — see [`Key::offset`].
///
/// **Thirty degrees, and this one is a social number rather than an anatomical
/// one.** There is nothing in a body that says how deep a bow is: lumbar
/// flexion runs past 60 degrees and a body could fold to its knees. What sets
/// it is what the gesture means — the band runs from a 15 degree nod of the
/// whole body, which is an acknowledgement in passing, to a 45 degree bow,
/// which is an apology — and 30 is the ordinary respectful one in the middle of
/// it. Saying that plainly is better than deriving it from a joint that does
/// not constrain it.
pub const BOW_PITCH: f32 = 1.0 / 3.0;

/// Where a bow looks, as a tangent below the horizon — an absolute pitch, not
/// an increment.
///
/// **Stated in the world rather than against the trunk, and that is the one
/// thing this gesture had to decide for itself.** A trunk pitched at a single
/// hinge above a pelvis that cannot move turns everything above that hinge
/// further than it inclines the chord — 54.9 degrees of segment for a 30 degree
/// chord, measured. That is fine for the gait, whose lean is five degrees and
/// which takes the whole of it back off at the neck anyway, and it is not fine
/// for a head: carried, the head of a 30 degree bow ends up looking 63 degrees
/// down, which is a body inspecting its own shoes.
///
/// So the bow's gaze track is [`Space::World`] and says where the head points
/// rather than how much further than the trunk it turns. Thirty-five degrees:
/// five past the trunk's own inclination, so the neck contributes a little and
/// the head is neither slack nor held insolently level. It is the same job
/// [`super::gait`]'s lean does with its neck counter-rotation, said as a goal
/// instead of as a subtraction.
///
/// [`Space::World`]: super::Space::World
pub const BOW_GAZE: f32 = 0.700_207_5;

/// Share of a bow spent going down, and the same again coming up.
///
/// **The rest is the hold, and the hold is what makes it a bow.** A body that
/// pitches and immediately straightens has stumbled; the pause at the bottom is
/// the whole of the courtesy. At 0.3 either end the hold is the middle 40 per
/// cent of the gesture.
pub const BOW_FALL: f32 = 0.3;

/// A bow: the trunk pitches forward, holds, and comes back up, with the gaze
/// dropping a little further than the trunk carries it.
///
/// **Two tracks for the two parts, and neither is a limb** — the trunk's is
/// [`Target::Trunk`] and the head's is [`Target::Gaze`], which is the nod's
/// answer inherited whole (#248). [`Clip::apply`] runs the trunk before the
/// gaze whatever order they are written in, so the gaze is aimed from a body
/// that has already bowed.
///
/// **The gaze is stated in the world, not against the bowed trunk**, which is
/// the one thing the bow had to decide that the nod did not — see [`BOW_GAZE`].
///
/// **The arms are deliberately not addressed, and what that leaves them doing
/// was measured rather than assumed.** They keep the relation to the chest they
/// rest in — 49.9 degrees off the trunk's own axis at rest, 54.3 at the bottom
/// of the bow — and are carried round with it. Against the WORLD that means
/// they swing further from hanging rather than nearer: 49.9 degrees off
/// vertical standing, 68.3 bowed, with the hand ending up 45 mm further from
/// the knee than it started.
///
/// That is the honest description and it is not the same as a bow with its arms
/// at its sides. The cause is the rest pose rather than this gesture: these
/// bodies rest with their arms out at about 50 degrees, so every gesture that
/// leaves an arm alone leaves it there. Bringing the hands in would be a pair
/// of grasper tracks and a decision about whether a bow's arms hang, press the
/// thighs, or meet in front — a real authorial choice, and one worth making
/// against a body whose rest arms hang, rather than one compensating for arms
/// that do not.
///
/// Not looping, and both ends are the body's own rest pose.
///
/// [`Key::offset`]: super::Key::offset
#[must_use]
pub fn bow() -> Clip {
    let pitched = Vec3::new(0.0, 0.0, BOW_PITCH);
    let dropped = Vec3::new(0.0, -BOW_GAZE, 0.0);
    let keys = |held: Vec3| {
        vec![
            Key::new(0.0, Vec3::ZERO),
            Key::new(BOW_FALL, held),
            Key::new(1.0 - BOW_FALL, held),
            Key::new(1.0, Vec3::ZERO),
        ]
    };
    Clip::new([
        Track::new(Target::Trunk, keys(pitched)),
        Track::new(Target::Gaze, keys(dropped)).in_world(),
    ])
}

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

/// How much of its own leg a seated body drops, and moves its feet forward.
///
/// **One constant for both, and it is the thigh** — read off the anatomy rather
/// than dialled. Sitting IS a thigh held horizontal over a shin held vertical,
/// and that pose is reached by dropping the pelvis exactly one thigh and moving
/// the foot forward exactly one thigh: the hip then sits level with the knee,
/// the ankle hangs directly under it, and the knee arrives at a right angle by
/// construction rather than by being asked for.
///
/// In the standing leg's reach, which is the unit a carriage is measured in
/// (see [`Target::Root`]). The plan holds the thigh at 0.4872 of the leg on
/// every body of the sweep, spread 0.0000, so this number is the plan's own; a
/// plan that varied the ratio would want it read from the rig instead, and the
/// right angle would survive either way.
pub const SEAT: f32 = 0.487_2;

/// Share of the descent by which a sitting body's feet have arrived.
///
/// **The feet lead, and the reading that says how much is the floor.** Root and
/// feet moving together over the same span is the obvious authoring and it
/// buckles: early in the descent the pelvis has fallen while the foot is still
/// under it, so the leg folds with nowhere to put the shin and the heel is
/// driven 14 mm through the floor. Nothing is wrong with the held pose — the
/// dip lives entirely in the transition — but a foot through a floor is a foot
/// through a floor.
///
/// Set against a criterion rather than by eye — the lowest joint must clear the
/// floor for the whole descent — and the criterion is what picks it out of a
/// band rather than a taste for the number. On the reference body, whose lowest
/// joint rests 18 mm up:
///
/// | lead | 0.9 | 0.7 | 0.5 | 0.35 | 0.2 |
/// |------|-----|-----|-----|------|-----|
/// | lowest joint | -10 mm | 1 | 13 | 15 | 14 |
///
/// Half, which clears by 13 mm with room either side, and which is also what a
/// body sitting down does: the feet go where they are going to be while it is
/// still on the way down, rather than after. Earlier buys two more millimetres
/// and starts to read as a step followed by a squat.
pub const SEAT_LEAD: f32 = 0.25;

/// Share of a sit spent lowering, and of a lie spent going down.
///
/// **The rest is the pose, because these two are states rather than gestures.**
/// A wave and a bow end where they began; sitting down ends up sitting, and a
/// clip that stood the body back up at its last key could not express that.
pub const SETTLE: f32 = 0.35;

/// How far a lying body drops, in the standing leg's reach.
///
/// **The pelvis's own rest height, less what a body lying on it is thick**, and
/// the second term is why this is measured rather than written down. The tilt
/// turns the body about its root, so a body tipped and not dropped lies
/// horizontally at hip height with nothing under it; this is the drop that puts
/// its BACK on the floor rather than its spine.
///
/// The correction is the deepest surface point of the lying pose — every
/// joint's height less its own bone radius — and it reads 0.0145 of the leg on
/// every body of the sweep, which is what makes it a constant here rather than
/// a term. Erring shallow on purpose: a node's radius overstates the surface,
/// because subdivision pulls the mesh inside it, and a body floating a
/// millimetre is better than one sunk in the floor.
pub const SLEEP_DROP: f32 = 1.009_5;

/// Share of the descent by which a lying body has finished tipping, the drop
/// finishing after it.
///
/// **Over before down, and the measurement said so against my expectation.** A
/// body that gets low first and reclines after is what a person does and it is
/// the worse of the two here, badly: dropping while still upright puts a
/// standing body's feet most of a leg below the floor. Tipping first swings
/// them up and round instead, and only then does the body settle onto its back.
///
/// **And this constant buys one defect with another, which is worth saying
/// plainly rather than hiding in a chosen number.** A faster tilt sinks less
/// and jumps more, because a body swinging a quarter turn about its own pelvis
/// moves its ends a long way in very little time. Sank below the standing
/// surface, and the furthest any joint travels between two samples:
///
/// | lead | 1.0 | 0.7 | 0.5 | 0.3 | 0.15 |
/// |------|-----|-----|-----|-----|------|
/// | sank | -139 mm | -70 | -38 | -14 | 0 |
/// | step | 47 mm | 59 | 77 | 116 | 216 |
///
/// There is no free point on that curve, and the reason is that the authoring
/// is a draft: **a body does not lie down by rotating about its pelvis.** It
/// gets low, reclines, and then extends — three stages, of which [`sit`]
/// already is the first. Written that way the drop happens while the legs fold
/// and the turn happens from a body whose ends are already near its root, and
/// neither reading has to be paid for. That is a real piece of work rather than
/// a constant, and it is #263.
///
/// Three tenths in the meantime: 14 mm under and 116 mm of step, both moderate,
/// neither pretending to be a solution.
pub const SLEEP_LEAD: f32 = 0.3;

/// Which way a body lies down: a quarter turn, feet toward `-Z`.
///
/// **Face up rather than face down, and the sign is the whole difference.** A
/// body tipped toward `+Z` rotates its ventral `+Z` down into the floor and
/// sleeps on its face; toward `-Z` the belly comes up and the body lies on its
/// back, which is what a resting body does.
pub const SLEEP_TILT: f32 = -1.0;

/// A sitting idle: the pelvis drops one thigh, the feet come forward one thigh,
/// and the legs fold because they have nowhere else to go.
///
/// **The item that looked least like this format and turned out to be its
/// cleanest case.** Nothing here says anything about a knee. The root drops,
/// the contacts are held still in the world ([`Space::World`], which exists for
/// exactly this), and the solve is left with no way to reach them except to
/// bend — which is the whole argument for goals over angles, arriving at the
/// one roster item that is not a reach.
///
/// **A state, not a gesture**: it ends sitting rather than standing, so
/// [`returns_to_rest`] is false for it and that is the answer rather than a
/// gap. A caller playing this holds it at its end.
///
/// **The life on top is the caller's**, not this clip's. A seated body still
/// breathes and shifts, and [`super::IdleConfig`] is where that lives; a clip
/// that carried its own idle would have to decide whether this body is alert or
/// dozing, which is exactly the thing the caller knows and the clip does not.
///
/// [`Target::Root`]: super::Target::Root
/// [`Space::World`]: super::Space::World
#[must_use]
pub fn sit() -> Clip {
    let down = Vec3::new(0.0, -SEAT, 0.0);
    let out = Vec3::new(0.0, 0.0, SEAT);
    Clip::new([
        Track::new(
            Target::Root,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(SETTLE, down),
                Key::new(1.0, down),
            ],
        ),
        Track::new(
            Target::Contacts,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(SETTLE * SEAT_LEAD, out),
                Key::new(1.0, out),
            ],
        )
        .in_world(),
    ])
}

/// A sleeping idle: the body lies down on its back.
///
/// A quarter turn and a drop, and nothing else — the limbs are left where the
/// carriage puts them, which for a body on its back is lying beside it. See
/// [`Target::Tilt`] for why a rotation this large needed the unit changing, and
/// [`SLEEP_TILT`] for why it is negative.
///
/// **The feet point at the ceiling and that is a draft's cost, measured.** An
/// ankle holds whatever angle it rests at — 74.8 degrees to the shin, standing
/// or lying — so a body tipped through a quarter turn takes its standing feet
/// with it and the toe ends up 375 mm off the floor with the sole facing very
/// nearly straight up. A relaxed supine foot flops the other way. Nothing in
/// this format addresses an ankle's angle, and the fix belongs with the staged
/// descent that would place the feet along the floor rather than carry them
/// round: #263.
///
/// **A state, not a gesture**, for [`sit`]'s reason and in the same shape.
///
/// [`Target::Tilt`]: super::Target::Tilt
#[must_use]
pub fn sleep() -> Clip {
    let over = Vec3::new(0.0, 0.0, SLEEP_TILT);
    let down = Vec3::new(0.0, -SLEEP_DROP, 0.0);
    let held = |at: Vec3| {
        vec![
            Key::new(0.0, Vec3::ZERO),
            Key::new(SETTLE, at),
            Key::new(1.0, at),
        ]
    };
    Clip::new([
        Track::new(
            Target::Tilt,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(SETTLE * SLEEP_LEAD, over),
                Key::new(1.0, over),
            ],
        ),
        Track::new(Target::Root, held(down)),
    ])
}

/// Whether a gesture is one a body at rest can hold, or one it plays and leaves.
///
/// **Both kinds are in the roster now.** A wave, a refusal, a nod and a bow are
/// the second kind and both ends of every one of them is the body's own rest
/// pose (see [`REACH_TIME`]); [`sit`] and [`sleep`] are the first, and end in
/// the pose they are for. This is what tells them apart, and a caller that
/// plays a held pose and expects the body back is the mistake it catches.
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
        "Head Nod" => Some(nod()),
        "Bow" => Some(bow()),
        "Sitting_Idle" => Some(sit()),
        "Sleeping" => Some(sleep()),
        _ => None,
    }
}

/// The roster this module covers, in the baked set's own names.
pub const ROSTER: &[&str] = &[
    "Greeting",
    "Reject",
    "Head Nod",
    "Bow",
    "Sitting_Idle",
    "Sleeping",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Pose;
    use crate::anim::clip::gaze_config;
    use crate::plan::{BodyPlan, Composites, HumanoidParams, QuadrupedParams, Zone};
    use crate::rig::Rig;

    /// The bodies every gesture is judged on: a stature in metres, then limb
    /// length, neck length and head size against it.
    ///
    /// **Proportion is the half that tests anything.** The plan scales a body
    /// uniformly with stature, so a goal in any unit lands in the same relative
    /// place by arithmetic; only a change of proportion moves the parts of a
    /// body against each other.
    ///
    /// **And which proportion depends on what the gesture is made of** (#248).
    /// `limb_length` moves the shoulder, the reach and the hand's rest position
    /// — and moves the head by nothing at all: across its whole range the neck
    /// stays 0.250 of a body's height and the head sits at 0.955 of it, to
    /// three figures. A nod tested on stature and limbs alone reads a flat
    /// spread whatever it is written in. `neck_length` and `head_size` are the
    /// axes a rotation can be wrong on.
    const BODIES: [(f32, f32, f32, f32); 8] = [
        (1.2, 0.0, 0.0, 0.0),
        (2.2, 0.0, 0.0, 0.0),
        (1.7, -1.0, 0.0, 0.0),
        (1.7, 1.0, 0.0, 0.0),
        (1.7, 0.0, -1.0, 0.0),
        (1.7, 0.0, 1.0, 0.0),
        (1.7, 0.0, 0.0, -1.0),
        (1.7, 0.0, 0.0, 1.0),
    ];

    /// How many points of a gesture the tests sample.
    const SWEEP: usize = 60;

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

    /// How far below its rest facing a gesture ever pitches the head, in
    /// degrees, and how far any chest joint pitches with it.
    ///
    /// **Two readings and not one, because what separates a nod from a bow is
    /// which joints did it.** The head's own pitch cannot tell them apart: a
    /// head carried down by a chest is pointing exactly where a nodded one is.
    ///
    /// The chest's is taken as a magnitude — a chest that pitched BACK to hold
    /// the head level would be a posture too — while the head's is signed, so a
    /// gesture that lifts the head does not read as one that dropped it.
    ///
    /// **Sampled on the clip's own key times as well as on a grid, and the
    /// grid alone reported a defect that was not there.** These gestures
    /// interpolate linearly, so a peak sits exactly ON a key; the nod's are at
    /// 0.225 and 0.725, which a 60-point grid straddles. It read the quadruped
    /// nodding 16.6 degrees of an asked-for 17.19 and the shortfall looked like
    /// the gaze falling short on a long neck. It was the ruler's phase — the
    /// same reading the gait's own audit was caught making.
    fn dipped(rig: &Rig, clip: &Clip) -> (f32, f32) {
        let head = *rig
            .in_zone(Zone::Head)
            .first()
            .expect("the plan builds a head");
        let chest = rig.in_zone(Zone::Chest);
        let rested = Pose::rest(rig).forward(rig).rotations;
        let pitch = |from: Vec3, to: Vec3| {
            let angle = |run: Vec3| run.y.atan2(run.z.hypot(run.x));
            (angle(from) - angle(to)).to_degrees()
        };
        let times = (0..=SWEEP).map(|frame| frame as f32 / SWEEP as f32).chain(
            clip.tracks
                .iter()
                .flat_map(|track| &track.keys)
                .map(|key| key.time),
        );
        times.fold((0.0f32, 0.0f32), |most, time| {
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, time);
            let turned = pose.forward(rig).rotations;
            let facing = |joint: usize| {
                pitch(
                    rested[joint] * crate::rig::landmark::FORWARD,
                    turned[joint] * crate::rig::landmark::FORWARD,
                )
            };
            (
                most.0.max(facing(head)),
                chest
                    .iter()
                    .fold(most.1, |most, &joint| most.max(facing(joint).abs())),
            )
        })
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
                .map(|&(height, limbs, neck, head)| {
                    furthest(&body(height, limbs, neck, head), &clip)
                })
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
        for clip in [
            wave(Limb::ForeRight),
            reject(),
            nod(),
            bow(),
            sit(),
            sleep(),
        ] {
            for &(height, limbs, neck, head) in &BODIES {
                let rig = body(height, limbs, neck, head);
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
        for clip in [
            wave(Limb::ForeLeft),
            wave(Limb::ForeRight),
            reject(),
            nod(),
            bow(),
        ] {
            assert!(returns_to_rest(&clip));
            for &(height, limbs, neck, head) in &BODIES {
                let rig = body(height, limbs, neck, head);
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
        for clip in [
            wave(Limb::ForeRight),
            reject(),
            nod(),
            bow(),
            sit(),
            sleep(),
        ] {
            for &(height, limbs, neck, head) in &BODIES {
                let rig = body(height, limbs, neck, head);
                // **Only the limbs a clip addresses, and only the ones that
                // are arms.** The first half is why the palm reading stopped
                // reporting an untouched hand's rest pose as the greeting's
                // failing; the second is why a seated body does not report its
                // knee as an elbow above a shoulder. A carriage clip addresses
                // the legs, and `limb_chain` will hand back a leg's three
                // joints as happily as an arm's.
                let stance = rig.ground_contacts();
                let addressed: Vec<Limb> = clip
                    .tracks
                    .iter()
                    .flat_map(|track| track.target.resolve(&rig))
                    .filter(|limb| !stance.contains(limb))
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
        for clip in [
            wave(Limb::ForeRight),
            reject(),
            nod(),
            bow(),
            sit(),
            sleep(),
        ] {
            for &(height, limbs, neck, head) in &BODIES {
                let rig = body(height, limbs, neck, head);
                // **Only the limbs a clip addresses, and only the ones that
                // are arms.** The first half is why the palm reading stopped
                // reporting an untouched hand's rest pose as the greeting's
                // failing; the second is why a seated body does not report its
                // knee as an elbow above a shoulder. A carriage clip addresses
                // the legs, and `limb_chain` will hand back a leg's three
                // joints as happily as an arm's.
                let stance = rig.ground_contacts();
                let addressed: Vec<Limb> = clip
                    .tracks
                    .iter()
                    .flat_map(|track| track.target.resolve(&rig))
                    .filter(|limb| !stance.contains(limb))
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

    /// How far a gesture ever inclines the trunk's chord off where it rests,
    /// in degrees, and how far the shoulder girdle itself turns.
    ///
    /// **Two readings, because a one-hinge trunk makes them different
    /// numbers.** The pelvis cannot rotate — it carries the legs out from under
    /// the footing solve — so the joint above it swings a segment, and the
    /// chord from pelvis to girdle is a length-weighted mix of that turned
    /// segment and a stub that stayed put. It arrives at a fraction of the
    /// angle applied: 30 degrees of chord costs 54.9 degrees of girdle on the
    /// default body. The chord is the quantity the gesture is stated in and the
    /// girdle is what the head would inherit if anything let it.
    fn inclined(rig: &Rig, clip: &Clip) -> (f32, f32) {
        let girdle = rig
            .in_zone(Zone::Neck)
            .first()
            .and_then(|&neck| rig.joints[neck].parent)
            .expect("the plan hangs a neck off a girdle");
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a rig has a root");
        let resting = Pose::rest(rig).forward(rig);
        let pitch = |from: Vec3, to: Vec3| {
            let angle = |run: Vec3| run.y.atan2(run.z.hypot(run.x));
            (angle(from) - angle(to)).to_degrees()
        };
        let times = (0..=SWEEP).map(|frame| frame as f32 / SWEEP as f32).chain(
            clip.tracks
                .iter()
                .flat_map(|track| &track.keys)
                .map(|key| key.time),
        );
        times.fold((0.0f32, 0.0f32), |most, time| {
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, time);
            let posed = pose.forward(rig);
            (
                most.0.max(pitch(
                    resting.positions[girdle] - resting.positions[root],
                    posed.positions[girdle] - posed.positions[root],
                )),
                most.1.max(pitch(
                    resting.rotations[girdle] * crate::rig::landmark::FORWARD,
                    posed.rotations[girdle] * crate::rig::landmark::FORWARD,
                )),
            )
        })
    }

    #[test]
    fn a_bow_inclines_the_trunk_it_asked_for_on_every_body() {
        // **The chord, not the rotation that delivers it** — the two are the
        // same number only on a body whose pelvis has no height, and the gait
        // found that out by writing a constant against the wrong one (#223).
        // `trunk_angle_for` inverts the relation and the bow asks it for the
        // same thing the walk does, so there is one description of how a trunk
        // pitches in this crate rather than two that drift.
        //
        // Reintroduced by putting the asked-for angle straight at the hinge
        // instead of solving for it: the chord arrives at 16.4 degrees of the
        // 30 asked, and drifts by 1.5 across limb proportion because the stub
        // below the hinge is a different share of the trunk on every body.
        let wanted = (BOW_PITCH * std::f32::consts::FRAC_PI_2).to_degrees();
        let chords: Vec<f32> = BODIES
            .iter()
            .map(|&(height, limbs, neck, head)| {
                inclined(&body(height, limbs, neck, head), &bow()).0
            })
            .collect();
        for (body, chord) in BODIES.iter().zip(&chords) {
            assert!(
                (chord - wanted).abs() < 0.1,
                "{body:?} asked for a {wanted:.2} degree bow and made a {chord:.2} degree one",
            );
        }
    }

    #[test]
    fn a_bow_does_not_look_at_its_own_shoes() {
        // **The gesture the trunk track cannot finish, and the reason the bow's
        // gaze is stated in the world.** A trunk pitched at one hinge turns
        // everything above it by the girdle's 54.9 degrees rather than the
        // chord's 30, and a head carried by that inherits the whole of it. The
        // first cut wrote the gaze as an increment on the bowed trunk, the way
        // the nod's is written on the body, and asked for a small extra drop:
        // 63.5 degrees. Asking today's number that way gives 89.9, which is a
        // body looking straight at the ground.
        //
        // Stated absolutely it is 35 degrees on every body: five past the
        // trunk's own inclination, so the neck contributes and the head is
        // neither slack nor held level. Reintroduced by dropping `in_world`
        // from the track.
        let wanted = BOW_GAZE.atan().to_degrees();
        for &(height, limbs, neck, head) in &BODIES {
            let rig = body(height, limbs, neck, head);
            let (dip, girdle) = (dipped(&rig, &bow()).0, inclined(&rig, &bow()).1);
            assert!(
                (dip - wanted).abs() < 0.5,
                "a {height} m body bowed with its head {dip:.2} degrees down, not {wanted:.2},                  while its girdle turned {girdle:.2}",
            );
        }
    }

    /// What a carriage clip does to the body: the tightest angle it closes a
    /// knee to, how far it tips the body, how far its lowest SURFACE sinks
    /// below where the same body's sits standing, and how far a contact strays
    /// from the height it rests at.
    ///
    /// **The surface, and against the body's own standing figure.** A joint is
    /// a point on an axis and the body hangs off it by the bone's radius, so
    /// the floor a pose has to clear is the joint less that; and a STANDING
    /// body already reads 50 mm under, because a node's radius overstates a
    /// surface subdivision pulls inside it. Measured against zero, every clip
    /// in the roster sinks and the reading says nothing.
    fn carried(rig: &Rig, clip: &Clip) -> (f32, f32, f32, f32) {
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a rig has a root");
        let legs: Vec<[usize; 3]> = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_chain(limb))
            .collect();
        let feet: Vec<usize> = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
            .collect();
        let under = |places: &[Vec3]| {
            places
                .iter()
                .enumerate()
                .fold(f32::MAX, |low, (joint, at)| {
                    let (near, far) = rig.bone_radii(joint);
                    low.min(at.y - near.max(far))
                })
        };
        let rest = Pose::rest(rig).forward(rig).positions;
        let standing = under(&rest);
        let times = (0..=SWEEP).map(|frame| frame as f32 / SWEEP as f32).chain(
            clip.tracks
                .iter()
                .flat_map(|track| &track.keys)
                .map(|key| key.time),
        );
        times.fold((180.0f32, 0.0f32, 0.0f32, 0.0f32), |most, time| {
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, time);
            let posed = pose.forward(rig);
            let places = &posed.positions;
            let knee = legs.iter().fold(most.0, |tightest, &[hip, knee, ankle]| {
                let thigh = (places[hip] - places[knee]).normalize_or_zero();
                let shin = (places[ankle] - places[knee]).normalize_or_zero();
                tightest.min(thigh.angle_between(shin).to_degrees())
            });
            let foot = feet.iter().fold(most.3, |worst, &joint| {
                worst.max((places[joint].y - rest[joint].y).abs())
            });
            (
                knee,
                most.1.max(
                    (posed.rotations[root] * Vec3::Y)
                        .angle_between(Vec3::Y)
                        .to_degrees(),
                ),
                most.2.min(under(places) - standing),
                foot,
            )
        })
    }

    #[test]
    fn a_seated_body_folds_its_legs_to_a_right_angle_on_every_body() {
        // **The claim a carriage makes, and it is an angle** — sitting IS a
        // right angle at the knee, and nothing about a length. Nothing in
        // `sit` says so: the root drops one thigh, the contacts are held in the
        // world, and the solve has no way to reach them but to bend. That is
        // the whole argument for goals over angles, arriving at the roster item
        // that is not a reach.
        //
        // **Two asserts, and they catch two different mistakes** — the same
        // pair the wave's spread test documents. Measuring the drop in body
        // heights rather than in the leg that holds the root up, and leaving
        // the number alone, closes the knee to 46 degrees: a squat, caught by
        // the depth. Re-deriving the number for the new unit hides that and
        // leaves the drift, because the leg is between 0.3975 and 0.4415 of a
        // body across this sweep — the knee then runs 86.73 to 88.59, which the
        // spread catches and the depth does not.
        let angles: Vec<f32> = BODIES
            .iter()
            .map(|&(height, limbs, neck, head)| carried(&body(height, limbs, neck, head), &sit()).0)
            .collect();
        let (low, high) = angles.iter().fold((f32::MAX, f32::MIN), |range, at| {
            (range.0.min(*at), range.1.max(*at))
        });
        assert!(
            high - low < 1.0,
            "the seated knee ran {low:.2} to {high:.2} degrees across the sweep",
        );
        // Not merely consistent but right: a pose that agreed with itself at
        // 60 degrees on every body would pass the spread and be a crouch.
        assert!(
            (low - 90.0).abs() < 3.0,
            "the seated knee closed to {low:.2} degrees, which is not a seat",
        );
    }

    #[test]
    fn a_seated_body_leaves_its_feet_where_it_found_them() {
        // **What [`Space::World`] is for, and the one line of `sit` that does
        // the work.** A body-relative contact goal travels with the body, so a
        // pelvis dropping a third of a metre would take the feet with it and
        // the body would sink through the floor with its legs straight. Held in
        // the world, the feet stay and the legs have to fold instead.
        //
        // Reintroduced by dropping `in_world`: the feet fall 336 mm, the whole
        // of the drop, and the knee never bends at all.
        for &(height, limbs, neck, head) in &BODIES {
            let rig = body(height, limbs, neck, head);
            let (_, _, _, strayed) = carried(&rig, &sit());
            assert!(
                strayed < 0.005,
                "a {height} m body's foot moved {:.0} mm off the height it rests at",
                strayed * 1000.0,
            );
        }
    }

    #[test]
    fn a_carriage_stays_on_top_of_the_floor() {
        // A pose that puts a body through the ground is not a pose, and both of
        // these can: sitting folds a leg past where the foot is, and lying
        // swings a standing body's feet down through the floor if the turn is
        // slow enough. Bounded at 25 mm, where the two read -4 and -14 — see
        // [`SEAT_LEAD`] and [`SLEEP_LEAD`], whose whole job is these numbers.
        //
        // Reintroduced by moving either lead: at a seat lead of 0.9 the sit
        // sinks 41 mm, and at a tilt lead of 1.0 the lie sinks 139.
        for (name, clip) in [("sit", sit()), ("sleep", sleep())] {
            for &(height, limbs, neck, head) in &BODIES {
                let rig = body(height, limbs, neck, head);
                let (_, _, sank, _) = carried(&rig, &clip);
                assert!(
                    sank > -0.025,
                    "{name} sank a {height} m body {:.0} mm below where it stands",
                    -sank * 1000.0,
                );
            }
        }
    }

    #[test]
    fn a_sleeping_body_lies_on_its_back() {
        // A quarter turn, and the SIGN of it is the whole difference between
        // sleeping and being face down in the floor. Asserted on where the
        // body's ventral axis ends up rather than on the angle, because the
        // angle is the same either way — which is exactly how a sign error
        // survives a test.
        //
        // Reintroduced by flipping [`SLEEP_TILT`]: still a 90 degree tilt, and
        // the belly points at the ground.
        for &(height, limbs, neck, head) in &BODIES {
            let rig = body(height, limbs, neck, head);
            let (_, tipped, _, _) = carried(&rig, &sleep());
            assert!(
                (tipped - 90.0).abs() < 0.5,
                "a {height} m body tipped {tipped:.1} degrees, not a quarter turn",
            );
            let root = rig
                .joints
                .iter()
                .position(|joint| joint.parent.is_none())
                .expect("a root");
            let mut pose = Pose::rest(&rig);
            sleep().apply(&rig, &mut pose, 1.0);
            let ventral = pose.forward(&rig).rotations[root] * crate::rig::landmark::FORWARD;
            assert!(
                ventral.y > 0.9,
                "a sleeping body's belly pointed ({:.2}, {:.2}, {:.2}), which is not upward",
                ventral.x,
                ventral.y,
                ventral.z,
            );
        }
    }

    #[test]
    fn a_held_pose_is_not_a_gesture_and_says_so() {
        // **The roster has two kinds now** and `returns_to_rest` is what tells
        // them apart. A wave, a refusal, a nod and a bow end where they began;
        // sitting down ends up sitting. A caller that plays a held pose and
        // expects the body back is the mistake this catches, and it could only
        // ever be caught by asking the clip.
        for clip in [wave(Limb::ForeRight), reject(), nod(), bow()] {
            assert!(returns_to_rest(&clip), "a gesture must give the body back");
        }
        for clip in [sit(), sleep()] {
            assert!(
                !returns_to_rest(&clip),
                "a held pose must not claim to give the body back",
            );
        }
    }

    #[test]
    fn a_clip_pitches_its_trunk_before_it_aims_its_gaze() {
        // **Both parts of a bow are measured in a frame the other one moves**,
        // so one of them has to go first and it cannot be whichever the author
        // happened to type first. `Clip::apply` sorts: trunk, then limbs, then
        // gaze.
        //
        // The guard is that writing the bow backwards changes nothing. Without
        // the sort it changes a great deal — the gaze aims the head 35 degrees
        // down and the trunk then carries it another 54.9, to 90.
        let rig = body(1.7, 0.0, 0.0, 0.0);
        let forwards = bow();
        let mut backwards = forwards.clone();
        backwards.tracks.reverse();
        for frame in 0..=SWEEP {
            let time = frame as f32 / SWEEP as f32;
            let (front, back) = (forwards.pose(&rig, time), backwards.pose(&rig, time));
            let worst = front
                .forward(&rig)
                .positions
                .iter()
                .zip(back.forward(&rig).positions)
                .fold(0.0f32, |most, (a, b)| most.max(a.distance(b)));
            assert!(
                worst < 1e-5,
                "the bow written backwards posed the body {:.1} mm differently at time {time}",
                worst * 1000.0,
            );
        }
    }

    #[test]
    fn a_nod_is_the_same_angle_on_every_neck() {
        // **The same claim as `a_gesture_lands_in_the_same_place_on_every_body`
        // and it cannot be asked the same way** (#248). That one reads a
        // displacement, and a nod displaces the head by 12 to 22 mm — an
        // artefact of where the neck's joints happen to sit, not the gesture.
        // What a nod IS is the angle, so the angle is what the spread is taken
        // of.
        //
        // Measured against the alternative this rejected: a nod written as a
        // place — a goal the neck chain solves toward, the way a limb's contact
        // is — put the same authored displacement at 25.7 degrees on a
        // big-headed body and 39.9 on a small-headed one, a spread of 14.2. In
        // its own unit that authoring is exact, holding the head's travel to
        // 0.0000 of a body height while this one varies; each normalisation
        // holds constant the quantity it is stated in, and only one of the two
        // is what the gesture means.
        //
        // The bug this catches in the shipped form is the same mistake wearing
        // the format's own clothes: consulting `Scale` for a gaze. Scaled by
        // the arm's reach instead of cancelling, the nod runs 4.4 degrees on
        // the short-limbed body and 6.2 on the long-limbed one.
        let angles: Vec<f32> = BODIES
            .iter()
            .map(|&(height, limbs, neck, head)| dipped(&body(height, limbs, neck, head), &nod()).0)
            .collect();
        let (low, high) = angles.iter().fold((f32::MAX, f32::MIN), |range, at| {
            (range.0.min(*at), range.1.max(*at))
        });
        // **Half a degree, and the true figure is 0.07.** What is left is the
        // gaze aiming from the head's posed position at a point pinned to its
        // rest one, which moves by a hair as the neck bends; the format's own
        // contribution is nothing.
        assert!(
            high - low < 0.5,
            "the nod ran {low:.2} to {high:.2} degrees across the sweep",
        );
        // And it is the angle that was asked for, not merely a consistent one:
        // a gesture that agreed with itself at the wrong depth on every body
        // would pass the spread and still be the wrong gesture.
        let wanted = NOD_DIP.atan().to_degrees();
        assert!(
            (low - wanted).abs() < 0.5,
            "the nod asked for {wanted:.2} degrees and delivered {low:.2}",
        );
    }

    #[test]
    fn a_nod_is_a_third_of_the_neck_it_has() {
        // **A tuned constant is a number with a hidden argument**, and this is
        // [`NOD_DIP`]'s written down. It is not a depth that looked right: it
        // is a third of the range [`gaze_config`] clamps a clip's gaze at, in
        // the tangent a gaze key is stated in. Move that limit and this becomes
        // a different fraction of a different neck without a line of it
        // changing, which is exactly how the elbow constant went wrong in #223.
        let third = (gaze_config().limit / 3.0).tan();
        assert!(
            (NOD_DIP - third).abs() < 1e-4,
            "the nod dips {NOD_DIP}, a third of the neck's range is {third}",
        );
    }

    #[test]
    fn a_nod_does_not_bow() {
        // **What separates a nod from a bow is which joints made it**, and the
        // head's own pitch cannot tell them apart — a head carried down by a
        // leaning chest points exactly where a nodded one does. So the chest is
        // read separately, and a nod's answer is that it did nothing.
        //
        // This is a convention rather than a field, and the convention is
        // [`gaze_config`]'s leading share of zero. Reintroduced by handing the
        // gaze `GazeConfig::default` instead, whose 0.25 chest share is the
        // sensible one for looking at something on purpose: 3.5 degrees of the
        // 17.2 goes into the chest, a quarter of the gesture spent bowing.
        for &(height, limbs, neck, head) in &BODIES {
            let rig = body(height, limbs, neck, head);
            let (dip, chest) = dipped(&rig, &nod());
            assert!(
                chest < 0.5,
                "a {height} m body's chest pitched {chest:.2} degrees of a {dip:.2} degree nod",
            );
        }
    }

    #[test]
    fn a_body_that_walks_on_its_hands_still_nods() {
        // **The refusal is per item, and this is the item that does not
        // refuse.** A quadruped has no hand free to wave and every reason to
        // have a head, so the nod is the one gesture of the three that lands on
        // it — which is the whole point of naming parts semantically rather
        // than deciding per body plan.
        //
        // The guard is worth its line because the cheap way to write a gaze
        // track would have been to hang it off a limb the way every other track
        // is, and a quadruped's front limb is a leg.
        let quadruped =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
                .expect("the plan builds a quadruped");
        let (dip, chest) = dipped(&quadruped, &nod());
        let wanted = NOD_DIP.atan().to_degrees();
        // A fifth of a degree, and the tenth of it that is not zero is
        // [`GAZE_AHEAD`]'s measured residual on exactly this body.
        assert!(
            (dip - wanted).abs() < 0.2,
            "a quadruped asked for a {wanted:.2} degree nod and made a {dip:.2} degree one",
        );
        assert!(
            chest < 0.5,
            "a quadruped's chest pitched {chest:.2} degrees"
        );
    }

    #[test]
    fn a_gaze_is_measured_in_neither_of_the_units_a_track_can_name() {
        // **A rotation has no unit to be normalised in**, so [`Scale`] has
        // nothing to say about a gaze track and is not consulted for one. The
        // guard is that saying either thing changes nothing at all — the same
        // nod, to the last bit, whichever unit the track claims.
        //
        // Written as a test rather than as a type because the alternative is a
        // third `Scale` arm that multiplies by one, and a builder method a
        // gesture is free to call is better answered than forbidden.
        let rig = body(1.7, 0.0, 0.0, 0.0);
        let plain = nod();
        let claimed = Clip::new(
            plain
                .tracks
                .iter()
                .cloned()
                .map(Track::on_body)
                .collect::<Vec<_>>(),
        );
        for frame in 0..=SWEEP {
            let time = frame as f32 / SWEEP as f32;
            assert_eq!(
                plain.pose(&rig, time).rotations,
                claimed.pose(&rig, time).rotations,
                "the nod changed when its track claimed a unit, at time {time}",
            );
        }

        // The trunk ignores the space as well as the unit, and for a narrower
        // reason: its inclination is solved from the rig's rest chord, so there
        // is no posed frame to hold it against. It composes onto whatever lean
        // the body already carries, which is [`Space::Body`]'s answer, and a
        // track that says otherwise gets it anyway rather than getting silence
        // in some third form.
        let trunk = Clip::new([Track::new(Target::Trunk, bow().tracks[0].keys.clone())]);
        let dressed = Clip::new([Track::new(Target::Trunk, bow().tracks[0].keys.clone())
            .on_body()
            .in_world()]);
        for frame in 0..=SWEEP {
            let time = frame as f32 / SWEEP as f32;
            assert_eq!(
                trunk.pose(&rig, time).rotations,
                dressed.pose(&rig, time).rotations,
                "the bow's trunk changed when its track claimed a unit and a space, at {time}",
            );
        }
    }

    #[test]
    fn a_body_built_to_stand_on_all_fours_does_not_bow() {
        // **The bow's own per-item refusal, and it is read off the body rather
        // than off the plan that built it.** Inclining a trunk off vertical is
        // only a movement a body has if its trunk stands up: a quadruped's rest
        // chord lies 88 degrees off vertical, and the solve that finds the
        // hinge angle for a wanted inclination goes through the pitch of that
        // chord, which for a horizontal body is `atan2` of a vertical run it
        // does not have. Reintroduced by dropping the check, a 30 degree bow
        // threw joints 690 mm across a body 580 mm tall.
        //
        // **The gaze half is not refused and should not be**, which is the
        // point of refusing per track. A quadruped can look down; what it
        // cannot do is fold. Whether a deep head-drop is an acceptable
        // quadruped bow is #248's deferred quadruped clause, and this guard is
        // about the trunk not breaking either way.
        let quadruped =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
                .expect("the plan builds a quadruped");
        let (chord, girdle) = inclined(&quadruped, &bow());
        assert!(
            chord.abs() < 0.01 && girdle.abs() < 0.01,
            "a quadruped's trunk pitched {chord:.2} degrees at the chord and {girdle:.2} at the \
             girdle",
        );
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
