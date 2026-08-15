//! Swimming: a tread and a crawl, on one axis.
//!
//! A body in water has no ground to stand on, so almost none of
//! [`super::gait`] applies — there is no stance, no contact to pin, no
//! penetration to avoid. What is left is a cycle, and the whole of the design
//! is deciding how many cycles there are.
//!
//! **There is one.** A body holding station and a body swimming forward look
//! like two motions and are one: the same limbs going round the same loops,
//! with the loops opened out and the body laid down as it starts to travel.
//! Treading water is a crawl at zero speed, in the same sense that [`Leap`]'s
//! fall is a jump that never launched.
//!
//! [`Leap`]: super::Leap
//!
//! # The axis
//!
//! [`Swim::pace`], in metres per second, divided by the body's own length —
//! so what actually drives the motion is **specific speed, in body lengths per
//! second**, and a child and a giant swimming alike get the same stroke. A
//! human front crawl cruises at roughly 0.7 lengths per second, which is what
//! [`PRONE_AT`] says and what full effort means here.
//!
//! Everything rides that one number:
//!
//! * the trunk goes from **upright to prone**, because a slow swimmer hangs
//!   vertically in the water and a fast one lies along it;
//! * each arm's loop opens from a **small scull** in front of the chest into a
//!   **full sagittal windmill**, by growing its radius and tilting its plane;
//! * the **roll** about the body's long axis arrives with the stroke, because
//!   there is nothing to roll about until the body is lying down;
//! * the legs go from a **wide bent-knee cycle** to a **flutter**.
//!
//! # Why every loop closes
//!
//! A procedural cycle that does not return to where it started pops once per
//! cycle forever, and the epic's own reference clips do exactly that (#237). So
//! nothing here is allowed a frequency that is not a whole multiple of the
//! stroke: the legs run at [`KICKS_PER_ARM`] times the arms, the surge at
//! twice, the roll at once. That constraint is what rules out the obvious
//! reading of the literature's "flutter at two to three times the arm
//! frequency" — two and three close, two and a half does not.
//!
//! It also rules out the obvious way to open a scull into a windmill, which is
//! to grow the swing angle until it reaches a full turn. Anything short of a
//! whole turn leaves the arm somewhere else at the end of the cycle than it
//! began. The loop is opened by growing its **radius** instead, from a few
//! degrees of cone about a mean direction to the great circle that is a
//! windmill, and a circle of any radius closes.
//!
//! # What this draft does not do
//!
//! Named, rather than left to be discovered:
//!
//! * **One stroke.** Front crawl only: no breaststroke, no backstroke, no
//!   sidestroke.
//! * **No breathing.** A crawl breathes to one side every second or third
//!   stroke, which is a two- or three-cycle period — the one thing here that
//!   genuinely cannot close on a single cycle, and so wants its own clock
//!   rather than a fudge inside this one.
//! * **No turning and no surfacing.** The body swims along its own `+Z`, at
//!   whatever depth the caller puts it.
//! * **No water.** Nothing here knows where the surface is, and the surge is
//!   an authored bob rather than a buoyancy calculation.
//!
//! # Looking at it
//!
//! Four things about a swim can be measured and are, in `examples/swimaudit`:
//! whether the cycle closes, whether its two halves mirror, whether the hands
//! push water backwards on the side they can push from, and whether anything
//! sweeps through the body. **How it reads is not one of them.** For that:
//!
//! ```text
//! cargo run --release -F builtin-clips --example viewer -- --swim 1.3
//! ```
//!
//! in the sibling `bevy_symbios_avatar`, whose motion picker carries a swim
//! beside the walk and a slider for the one axis.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{Quat, Vec3};

use super::pose::Pose;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// Specific speed at which a body is swimming flat out, in body lengths per
/// second.
///
/// **Seven tenths of a length a second**, which is a human front crawl at a
/// steady cruise: about 1.3 m/s on a 1.8 m body. Sprinters reach half again as
/// much and this saturates below them on purpose — the axis exists to carry a
/// body from holding station to swimming properly, and a scale that only
/// reaches full effort at a competitive pace would spend its whole useful range
/// looking half-hearted.
pub const PRONE_AT: f32 = 0.7;

/// How far the trunk lies down at full effort, in radians.
///
/// Eighty degrees rather than ninety: a crawl swimmer is not level. The head
/// and chest ride a little higher than the hips, both because the lungs are
/// buoyant and because the swimmer is looking somewhere other than straight
/// down, and a body laid exactly flat reads as a plank being towed.
pub const PRONE: f32 = 1.40;

/// How far the body rolls about its own long axis each stroke, in radians.
///
/// Thirty-five degrees to each side, in the middle of the 30–45 the swimming
/// literature reports for front crawl. **The roll is not decoration**: it is
/// what lets the arm reach past the shoulder at the catch and what brings the
/// other shoulder clear of the water for the recovery, and a crawl animated
/// without it reads as a body being dragged rather than one swimming.
pub const ROLL: f32 = 0.61;

/// How far the body surges up and down each stroke, as a share of its length.
///
/// A hundredth, which on a human body is a couple of centimetres. Twice per
/// cycle, because there are two arms and each one's pull lifts the body a
/// little.
pub const SURGE_OF_LENGTH: f32 = 0.01;

/// How many times each leg kicks per stroke cycle.
///
/// **Three, which is the six-beat crawl** — six kicks to a cycle, three per
/// leg — and it is the standard distance-swimming kick. It is also a whole
/// number, which the module docs explain is not a coincidence: the two-beat
/// and six-beat kicks close on a stroke and the "two to three times" of a
/// casual reading does not.
pub const KICKS_PER_ARM: f32 = 3.0;

/// Angular radius of the sculling loop a treading body's hand traces, in
/// radians.
///
/// Twelve degrees. A scull is small, and the smallness is the point: a body
/// holding station is moving its hands enough to stay up and no more. This is
/// the radius the arm's loop grows *from* — at full effort it is a right
/// angle, which is a windmill.
pub const SCULL: f32 = 0.21;

/// How far a treading body holds its elbows bent, in radians.
///
/// Seventy degrees. A scull is worked with the forearm, close in, and a
/// treading body with straight arms is a body doing a star jump.
pub const TREAD_ELBOW: f32 = 1.22;

/// How far a crawl folds the elbow at the top of the recovery, in radians.
///
/// Ninety degrees, which is the high elbow every coaching text asks for. It is
/// spent entirely on the recovery here: this draft carries the arm through the
/// pull straight, where a real crawl bends into an early vertical forearm.
pub const RECOVERY_ELBOW: f32 = 1.57;

/// How far the hip swings through a flutter kick at full effort, in radians.
///
/// Fifteen degrees each way. A flutter is a small fast kick driven from the
/// hip with a loose knee, and most of what the eye reads as its amplitude is
/// the shin trailing rather than the thigh moving.
pub const FLUTTER: f32 = 0.26;

/// How far the hip swings through a treading body's leg cycle, in radians.
///
/// Thirty degrees, twice the flutter: holding station is done with big slow
/// bent-legged circles rather than a small fast kick.
pub const TREAD_KICK: f32 = 0.52;

/// How far a treading body holds its knees bent, in radians.
pub const TREAD_KNEE: f32 = 1.05;

/// How far a flutter kick lets the knee trail, in radians.
///
/// Twenty-five degrees at its deepest, and only on the downbeat: the knee
/// straightens as the foot finishes the kick, which is where the propulsion
/// comes from.
pub const FLUTTER_KNEE: f32 = 0.44;

/// How far the ankles point at full effort, in radians.
///
/// Thirty-five degrees of plantarflexion. A swimmer's foot is a fin, and a
/// flexed one is a brake — it is the single most visible difference between a
/// body that swims and a body doing a kicking motion in water.
pub const POINTED: f32 = 0.61;

/// One frame of a swim.
///
/// A pure function of the cycle, like [`Walk`] and unlike [`Idle`]: there is
/// nothing to remember between frames, so nothing here is stateful and the
/// same cycle always gives the same pose.
///
/// [`Walk`]: super::Walk
/// [`Idle`]: super::Idle
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swim {
    /// Where in the stroke this frame is, `0..1`, wrapping.
    ///
    /// **One cycle is both arms**, so each arm strokes once and each leg kicks
    /// [`KICKS_PER_ARM`] times. How long that takes in seconds is the caller's:
    /// a tread and a crawl run the same loops on the same clock, and the
    /// difference in their real frequency is the difference in how fast the
    /// caller advances this.
    pub cycle: f32,
    /// How fast the body is travelling through the water, in metres per second.
    ///
    /// Zero treads. See [`PRONE_AT`] for what full effort is.
    pub pace: f32,
    /// Whether the trunk layer runs: the pitch onto the front, the roll about
    /// the body's long axis, and the surge.
    ///
    /// One flag for the three because they answer one question — what the body
    /// is doing as a whole, rather than what its limbs are doing — and because
    /// an instrument wanting to read the limbs alone wants all three off at
    /// once.
    pub carriage: bool,
}

/// What one frame of a swim did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Swum {
    /// Where on the tread-to-crawl axis this frame sat, `0..1`.
    pub effort: f32,
    /// How far the trunk was pitched onto its front, in radians.
    pub pitch: f32,
    /// How far the body was rolled about its own long axis, in radians,
    /// positive toward the body's left.
    pub roll: f32,
    /// How far the body was carried above its rest height by the surge, in
    /// metres.
    pub surge: f32,
}

impl Swim {
    /// A swim at this point of the stroke, treading water.
    #[must_use]
    pub fn at(cycle: f32) -> Self {
        Self {
            cycle,
            pace: 0.0,
            carriage: true,
        }
    }

    /// The same swim, travelling at `pace` metres per second.
    #[must_use]
    pub fn toward(self, pace: f32) -> Self {
        Self {
            pace: pace.max(0.0),
            ..self
        }
    }

    /// Poses `pose` for this frame of the swim.
    ///
    /// The whole motion, in one call, for the same reason [`Walk::drive`]
    /// exists: the stages have an order and the record is that callers do not
    /// keep it.
    ///
    /// [`Walk::drive`]: super::Walk::drive
    pub fn drive(&self, rig: &Rig, pose: &mut Pose) -> Swum {
        let mut swum = Swum::default();
        if !pose.fits(rig) {
            return swum;
        }
        let cycle = self.cycle.rem_euclid(1.0);
        let length = length_is(rig);
        swum.effort = if length > f32::EPSILON {
            (self.pace / (length * PRONE_AT)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        stroke(rig, pose, cycle, swum.effort);
        kick(rig, pose, cycle, swum.effort);
        if self.carriage {
            swum = carry(rig, pose, cycle, length, swum);
        }
        swum
    }
}

/// How long a body is, in metres: the whole of it, head to heel.
///
/// Public because an instrument measuring a swim has to normalise by the same
/// length the swim does, and a second implementation of it would be a second
/// answer.
///
/// **Not the leg**, which is what [`super::Speed`] normalises a walk by and
/// would be wrong here. A walk is a pendulum hung from the hip and its period
/// is set by the leg; a swim is a body lying in the water being pushed along
/// it, and what it costs to push is set by how long the whole body is.
///
/// Taken off the rest skeleton's own vertical extent rather than from a
/// landmark, so it holds for a body whose head is not its highest joint and
/// for one that has no head at all.
#[must_use]
pub fn length_is(rig: &Rig) -> f32 {
    let span = rig.joints.iter().fold((f32::MAX, f32::MIN), |span, joint| {
        (span.0.min(joint.position.y), span.1.max(joint.position.y))
    });
    (span.1 - span.0).max(0.0)
}

/// How an arm is turned at this moment of the stroke, from its rest attitude.
///
/// **The loop is a rotation about its own axis, not a direction to aim at.**
/// Aiming — building the shoulder's rotation as the shortest arc from the rest
/// arm to wherever the hand should be — is the obvious construction and it
/// breaks twice. It has no answer at all when the target is opposite the rest
/// direction, which for an arm that hangs down is straight up, which is the top
/// of every windmill; and short of that it picks the shortest arc, so the
/// shoulder's own twist jumps about as the loop carries the arm past the
/// vertical. Measured, before this was a rotation: a 232 mm lurch in one joint
/// between two adjacent samples of a mid-effort stroke, against 30 mm
/// everywhere else.
///
/// So the arm is aimed **once** — at where the loop starts, which is a constant
/// for a given effort — and then turned about the loop's axis. A rotation about
/// a fixed axis is continuous, is periodic by construction, and carries a twist
/// that goes round with the arm instead of snapping back.
///
/// **The loop is opened by its radius, never by its swing.** `effort` grows the
/// cone the hand traces about that axis from [`SCULL`] to a right angle, and
/// tilts the axis from the arm's own resting direction — a scull circles about
/// where the arm already hangs — to the body's lateral axis, where a cone of a
/// right angle is a great circle in the sagittal plane, which is a windmill.
/// Every radius closes on itself, which a growing swing does not: see the
/// module docs.
///
/// `phase` is `0` at the catch, so the propulsive half of the loop is the first
/// half of it: the hand enters ahead of the shoulder, sweeps down under the
/// body, and leaves behind it.
///
/// **Written for the body's left arm only**, and the right one is this
/// reflected — see [`mirrored`]. Building both from a `side` factor is what
/// this did first and it is a trap: a cross product changes sign under a
/// reflection and a rotation angle changes with it, so a construction that
/// looks symmetric is off by two signs that cancel at full effort and nowhere
/// else. Measured, before the reflection was taken properly: the two halves of
/// the stroke disagreed by 1191 mm in the middle of the axis and by 0.001 mm at
/// the end of it, which is exactly the shape of a bug that hides where anyone
/// would look for it.
fn arm_turn(phase: f32, effort: f32, rest: Vec3) -> Quat {
    let axis = loop_axis(effort);
    let radius = SCULL + (FRAC_PI_2 - SCULL) * effort;
    // Where on the circle the stroke starts. Tilted off the axis by the loop's
    // radius, about whichever perpendicular puts the catch out in FRONT — and
    // **front is the body's `+Y`, not its `+Z`**, which is the one thing about
    // this that is not obvious. A swimming body is lying down: the direction it
    // travels is the one that pointed at the ceiling while it stood up, and its
    // `+Z` is the way its belly faces. Starting the loop at `+Z` is what this
    // did first, and it put phase zero in the middle of the pull rather than at
    // the catch — the motion was the same shape a quarter turn round, and every
    // sentence written about which half of it was propulsive was wrong.
    //
    // At full effort the axis is `+X` and this is `+Z`, and a right angle about
    // `+Z` carries `+X` to `+Y`.
    let out = axis.cross(Vec3::Y).normalize_or(Vec3::Z);
    let start = Quat::from_axis_angle(out, radius) * axis;
    Quat::from_axis_angle(axis, phase * TAU) * Quat::from_rotation_arc(rest, start)
}

/// The axis the hand circles, for the body's left arm.
///
/// **Out in front while the body is treading**, because that is where a scull
/// works: the hands sweep in and out ahead of the chest, and an arm circling
/// about where it already hangs is a body standing at ease with a twitch. The
/// first draft had it down and only slightly forward, and together with an
/// elbow that folded about the wrong axis that put a treading hand 148 mm from
/// the trunk's own axis on a body whose chest is 150 mm through — inside
/// itself. Neither half of that alone breaches, so the reason this one is where
/// it is, is the anatomy rather than the measurement; what the measurement says
/// is how much room the pair of them buys. At 0.3 m/s a hand clears the trunk
/// by 484 mm here and by 356 with the axis put back.
///
/// The body's lateral axis once it is swimming, which puts the loop in the
/// sagittal plane and makes it a windmill.
fn loop_axis(effort: f32) -> Vec3 {
    Vec3::new(0.45, -0.55, 0.70)
        .normalize()
        .lerp(Vec3::X, effort)
        .normalize_or(Vec3::X)
}

/// A rotation reflected across the body's midline.
///
/// **The whole of what makes the two arms agree.** Reflecting is improper — it
/// reverses handedness — so a rotation does not simply get its axis mirrored:
/// the angle reverses too, and writing that out by hand is two sign errors
/// waiting to happen. `M R M` is the rotation itself reflected, and in
/// quaternion terms it is this one line.
fn mirrored(turn: Quat) -> Quat {
    Quat::from_xyzw(turn.x, -turn.y, -turn.z, turn.w)
}

/// Swings both arms through their loops.
fn stroke(rig: &Rig, pose: &mut Pose, cycle: f32, effort: f32) {
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        let Some([shoulder, elbow, _]) = rig.limb_chain(limb) else {
            continue;
        };
        // Half a cycle apart, which is what makes it a crawl rather than a
        // butterfly.
        let phase = (cycle - if limb == Limb::ForeLeft { 0.0 } else { 0.5 }).rem_euclid(1.0);
        // The rest direction of the upper arm, which the loop is built from.
        // Taken from the rig rather than assumed to hang straight down, because
        // a body built with its arms out to the sides would otherwise be swung
        // from the wrong rest attitude.
        let rest = (rig.joints[elbow].position - rig.joints[shoulder].position).normalize_or_zero();
        if rest != Vec3::ZERO {
            // **One arm's motion, and the other one's reflection of it.** The
            // loop is built in the body's left, so a right arm hands it a
            // reflected rest direction and reflects the answer back.
            let right = rig.joints[shoulder].position.x < 0.0;
            let canonical = if right {
                Vec3::new(-rest.x, rest.y, rest.z)
            } else {
                rest
            };
            let turn = arm_turn(phase, effort, canonical);

            // The elbow folds on the recovery and carries a standing bend while
            // the body is treading. `-sin` is negative over the propulsive half
            // of the loop and peaks in the middle of the recovery, so the fold
            // arrives where the hand is furthest from the water and nowhere
            // else.
            let recovery = (-(phase * TAU).sin()).max(0.0);
            let bend = TREAD_ELBOW * (1.0 - effort) + RECOVERY_ELBOW * effort * recovery;

            // **About the body's lateral axis, expressed in the elbow's own
            // frame** — and the two wrong answers either side of it are worth
            // recording, because they are the same wrong answer the walk's arm
            // swing made (see [`super::gait::swing_arms`] on the elbow folding
            // about the wrong axis and spending itself spinning the forearm).
            //
            // Folding about the elbow's own local `X` is what this did first.
            // That axis is carried round by the shoulder, so it is the body's
            // lateral axis only for an arm that has not been swung — and this
            // rig rests its arms 45 degrees out to begin with. Measured, at 0.3
            // m/s it costs 184 mm of the 484 a hand clears the trunk by.
            //
            // Folding about the LOOP's axis is what it did next, and it is
            // right at full effort and useless at rest: a sculling arm points
            // very nearly along its own loop axis, so a fold about it barely
            // bends the elbow at all and mostly twists the forearm. Looked at
            // in the viewer, a treading body stood with its arms hanging.
            //
            // The body's own `+X` is right at both ends. At full effort it IS
            // the loop's axis, so the windmill is unchanged; at rest it is the
            // axis that carries a hanging forearm forward, which is where a
            // scull works.
            let fold = Quat::from_axis_angle(turn.inverse() * Vec3::X, -bend);

            pose.rotations[shoulder] *= if right { mirrored(turn) } else { turn };
            pose.rotations[elbow] *= if right { mirrored(fold) } else { fold };
        }
    }
}

/// Kicks both legs.
fn kick(rig: &Rig, pose: &mut Pose, cycle: f32, effort: f32) {
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let Some([hip, knee, ankle]) = rig.limb_chain(limb) else {
            continue;
        };
        // The two legs in antiphase, at a whole number of kicks to the stroke.
        let phase = cycle * KICKS_PER_ARM * TAU + if limb == Limb::HindLeft { 0.0 } else { PI };
        let swing = TREAD_KICK * (1.0 - effort) + FLUTTER * effort;
        pose.rotations[hip] *= Quat::from_rotation_x(swing * phase.sin());

        // **The knee trails the hip**, which is the whole look of a flutter: the
        // thigh leads, the shin follows a beat behind, and the foot finishes the
        // kick as the knee straightens. A quarter cycle of lag is that beat.
        let trail = ((phase - PI / 2.0).sin() + 1.0) / 2.0;
        let bend = TREAD_KNEE * (1.0 - effort) + FLUTTER_KNEE * effort * trail;
        pose.rotations[knee] *= Quat::from_rotation_x(bend);

        // A swimmer's foot is a fin. Held pointed rather than cycled, because
        // the ankle is loose in the water and follows the shin.
        pose.rotations[ankle] *= Quat::from_rotation_x(-POINTED * effort);
    }
}

/// Lays the body down, rolls it, and lets it surge.
fn carry(rig: &Rig, pose: &mut Pose, cycle: f32, length: f32, swum: Swum) -> Swum {
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return swum;
    };
    let pitch = PRONE * swum.effort;
    // **Once per stroke, and toward the arm that is pulling.** The body rolls
    // onto the side whose hand is under it, which is what gives that arm its
    // reach and lifts the other shoulder clear.
    let roll = ROLL * swum.effort * (cycle * TAU).sin();
    // Twice per stroke: two arms, two pulls, two lifts.
    let surge = SURGE_OF_LENGTH * length * swum.effort * (cycle * 2.0 * TAU).sin();

    // **Roll first, then pitch.** The roll is about the body's own long axis,
    // which in the rest skeleton is `+Y`; composing it inside the pitch is what
    // keeps it that axis rather than the world's vertical once the body is
    // lying down.
    pose.rotations[root] = Quat::from_rotation_x(pitch) * Quat::from_rotation_y(roll);
    pose.translation.y += surge;

    // The head comes back toward the line of travel, so a swimming body is
    // looking where it is going rather than at the bottom.
    if let Some(&neck) = rig.in_zone(Zone::Neck).first() {
        pose.rotations[neck] *= Quat::from_rotation_x(-pitch * 0.25);
    }

    Swum {
        pitch,
        roll,
        surge,
        ..swum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    /// How many points of the stroke the tests sample.
    const SWEEP: usize = 240;

    /// Every joint's world position at this moment of a swim.
    fn posed(rig: &Rig, cycle: f32, pace: f32) -> Vec<Vec3> {
        with_ventral(rig, cycle, pace).0
    }

    /// The same, and which way the body's belly is facing.
    ///
    /// The body's own `+Z`, carried by the root — which is the way it looks
    /// while it stands and the way it looks at the bottom once it is prone.
    fn with_ventral(rig: &Rig, cycle: f32, pace: f32) -> (Vec<Vec3>, Vec3) {
        let mut pose = Pose::rest(rig);
        Swim::at(cycle).toward(pace).drive(rig, &mut pose);
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a rig has a root");
        (pose.forward(rig).positions, pose.rotations[root] * Vec3::Z)
    }

    /// The furthest apart any one joint is between two posed skeletons.
    fn apart(one: &[Vec3], other: &[Vec3]) -> f32 {
        one.iter()
            .zip(other)
            .fold(0.0f32, |most, (a, b)| most.max(a.distance(*b)))
    }

    /// Every joint's mirror partner, matched by rest position.
    fn mirrors(rig: &Rig) -> Vec<usize> {
        (0..rig.len())
            .map(|joint| {
                let at = rig.joints[joint].position;
                let want = Vec3::new(-at.x, at.y, at.z);
                (0..rig.len())
                    .min_by(|&a, &b| {
                        rig.joints[a]
                            .position
                            .distance(want)
                            .total_cmp(&rig.joints[b].position.distance(want))
                    })
                    .unwrap_or(joint)
            })
            .collect()
    }

    #[test]
    fn a_stroke_closes_on_itself() {
        // **The reading the whole design is shaped by** (#244). A procedural
        // cycle whose limbs do not all return to where they started pops once
        // per cycle for as long as the body swims, and the epic's own reference
        // clips carry exactly that defect (#237).
        //
        // Asked as a RATIO rather than a distance, and the reason is that the
        // obvious form of this test proves nothing: `Swim::drive` wraps its own
        // cycle, so the pose at 1.0 IS the pose at 0.0 and comparing them is an
        // identity however badly the motion jumps. What a loop failing to close
        // actually looks like is a step across the wrap out of family with every
        // other step in the cycle.
        //
        // Measured with the kick at two and a half beats to the stroke, which is
        // what a casual reading of the literature's "two to three times the arm
        // frequency" gives: 10.5 times the median step. Whole multiples close;
        // that is the whole of why [`KICKS_PER_ARM`] is a whole number.
        let rig = biped();
        for pace in [0.0, 0.4, 0.8, 1.3] {
            let sweep: Vec<Vec<Vec3>> = (0..SWEEP)
                .map(|frame| posed(&rig, frame as f32 / SWEEP as f32, pace))
                .collect();
            let mut steps: Vec<f32> = (0..SWEEP)
                .map(|frame| apart(&sweep[frame], &sweep[(frame + 1) % SWEEP]))
                .collect();
            let across = steps[SWEEP - 1];
            steps.sort_by(f32::total_cmp);
            let median = steps[SWEEP / 2];
            assert!(
                across < median * 3.0,
                "at {pace} m/s the step across the wrap was {:.1} mm against a median of {:.1}",
                across * 1000.0,
                median * 1000.0,
            );
        }
    }

    #[test]
    fn the_two_halves_of_a_stroke_mirror_each_other() {
        // A crawl is one arm's stroke done twice, half a cycle apart, so the
        // body at `t + 0.5` is the body at `t` reflected — every joint, not just
        // the arms, because the roll and the surge have to agree too.
        //
        // **The bug this exists for is a sign, and it hides at the ends of the
        // axis.** Building the two arms from a `side` factor rather than
        // reflecting one into the other is off by the two signs a reflection
        // carries — the axis and the angle — and those cancel exactly where the
        // loop has no lateral component, which is at full effort. Measured
        // before [`mirrored`]: 0.001 mm at full effort and 1191 mm in the middle
        // of the axis.
        let rig = biped();
        let partners = mirrors(&rig);
        for pace in [0.0, 0.4, 0.8, 1.3] {
            for frame in 0..SWEEP {
                let now = posed(&rig, frame as f32 / SWEEP as f32, pace);
                let half = posed(&rig, frame as f32 / SWEEP as f32 + 0.5, pace);
                for (joint, &partner) in partners.iter().enumerate() {
                    let reflected = Vec3::new(-now[joint].x, now[joint].y, now[joint].z);
                    assert!(
                        half[partner].distance(reflected) < 0.001,
                        "at {pace} m/s joint {joint} sat {:.1} mm from its partner's reflection",
                        half[partner].distance(reflected) * 1000.0,
                    );
                }
            }
        }
    }

    #[test]
    fn a_swimming_hand_pushes_back_on_the_side_it_can_push_from() {
        // A hand is what a swimmer has instead of a foot, and a stroke is
        // propulsive exactly while the hand travels backwards relative to the
        // body. A loop that only carries the hands up and down is a body miming
        // a swim, and nothing else here would notice: it would close, it would
        // mirror, and it would be perfectly continuous.
        //
        // **Asked of the half of the loop that is under the body, and two
        // weaker forms of this question came first.** Measuring the longest
        // unbroken backward sweep passes for a loop run BACKWARDS, because a
        // circle traversed either way has a backward half. Measuring the net
        // travel over the first half of the cycle passes too, and for a better
        // reason: reversing a loop does not move where it is at the half-way
        // point, only which way it got there, so both directions have the same
        // two endpoints.
        //
        // What separates a stroke from its own reverse is WHERE the backward
        // travel happens. A hand pushes water when it is on the belly side of
        // the body and recovers when it is on the back side, so the propulsive
        // travel has to be ventral. Reversing the loop moves it to the other
        // side, and only this form of the question notices.
        //
        // Taken along the body's OWN axes, read off the trunk and the root,
        // because the body lies down as it speeds up and a direction fixed in
        // world space would stop meaning what it meant.
        let rig = biped();
        let trunk: Vec<usize> = [Zone::Chest, Zone::Abdomen, Zone::Pelvis]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
            .collect();
        let (Some(&head_end), Some(&tail_end)) = (trunk.first(), trunk.last()) else {
            panic!("the biped has a trunk");
        };
        let hand = rig.extremity_joints(Limb::ForeLeft)[0];
        for pace in [0.2, 0.4, 0.8, 1.3] {
            // Backward travel accumulated on the belly side, and on the back
            // side. A stroke puts almost all of it on the first.
            let mut back = [0.0f32; 2];
            for frame in 0..SWEEP {
                let (now, ventral) = with_ventral(&rig, frame as f32 / SWEEP as f32, pace);
                let next = posed(&rig, (frame + 1) as f32 / SWEEP as f32, pace);
                let ahead = (now[head_end] - now[tail_end]).normalize_or(Vec3::Z);
                let along = (next[hand] - now[hand]).dot(ahead);
                if along >= 0.0 {
                    continue;
                }
                let under = (now[hand] - now[tail_end]).dot(ventral) > 0.0;
                back[usize::from(!under)] -= along;
            }
            // Four times as much of the pull on the belly side as on the back
            // side, which is a loose bound around a clean result: measured, ALL
            // of it is ventral — 322 mm under the body and none at all over it
            // at the slowest pace asked for here, 1011 and none at the fastest.
            // A loop run backwards splits it evenly, 256 against 258.
            assert!(
                back[0] > back[1] * 4.0,
                "at {pace} m/s a hand pushed back {:.0} mm under the body and {:.0} mm over it",
                back[0] * 1000.0,
                back[1] * 1000.0,
            );
        }
    }

    #[test]
    fn the_axis_lays_the_body_down_and_never_overshoots_it() {
        // The one axis, end to end: a body holding station is upright and does
        // not roll, a body swimming is prone and rolls with every stroke, and
        // nothing beyond full effort goes any further — a swim faster than the
        // axis saturates at is still a swim, not a body folded in half.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let tread = Swim::at(0.25).drive(&rig, &mut pose);
        assert_eq!(tread.effort, 0.0);
        assert_eq!(tread.pitch, 0.0);
        assert_eq!(tread.roll, 0.0);
        assert_eq!(tread.surge, 0.0);

        let flat_out = length_is(&rig) * PRONE_AT;
        let mut last = 0.0;
        for step in 1..=8 {
            let mut pose = Pose::rest(&rig);
            let swum = Swim::at(0.25)
                .toward(flat_out * step as f32 / 8.0)
                .drive(&rig, &mut pose);
            assert!(
                swum.pitch >= last,
                "the pitch fell back to {:.2} from {last:.2} on the way up the axis",
                swum.pitch,
            );
            last = swum.pitch;
        }
        assert!((last - PRONE).abs() < 1e-5, "full effort pitched {last:.3}");

        let mut pose = Pose::rest(&rig);
        let past = Swim::at(0.25).toward(flat_out * 4.0).drive(&rig, &mut pose);
        assert_eq!(past.effort, 1.0);
        assert!((past.pitch - PRONE).abs() < 1e-5);
    }

    #[test]
    fn an_arm_never_sweeps_through_the_body() {
        // An arm loop that opens out with an effort axis is exactly the sort of
        // thing that reaches its widest by passing through the chest, and a
        // pose that does it looks like a glitch rather than like a swim. Asked
        // of the trunk's own axis rather than of its surface, so the bound is a
        // radius: a hand inside the trunk's own half-width is inside the body.
        //
        // **This is a constraint rather than a regression, and the difference is
        // worth stating.** It fired once, on the first draft, at 148 mm against
        // a 150 mm half-width — but neither half of the fix reproduces that
        // alone: at 0.3 m/s a hand clears by 484 mm as it stands, by 356 with
        // the old loop axis back and by 300 with the old elbow fold back. It
        // guards the two together, and on this body it has room to spare, so
        // what it will actually catch is a future loop aimed somewhere new
        // rather than a return of the bug it was written for.
        let rig = biped();
        let trunk: Vec<usize> = [Zone::Chest, Zone::Abdomen, Zone::Pelvis]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
            .collect();
        let (Some(&head_end), Some(&tail_end)) = (trunk.first(), trunk.last()) else {
            panic!("the biped has a trunk");
        };
        let widest = trunk
            .iter()
            .fold(0.0f32, |most, &joint| most.max(rig.joints[joint].radius));

        for pace in [0.0, 0.4, 0.8, 1.3] {
            for frame in 0..SWEEP {
                let now = posed(&rig, frame as f32 / SWEEP as f32, pace);
                let (from, to) = (now[tail_end], now[head_end]);
                let axis = to - from;
                for limb in [Limb::ForeLeft, Limb::ForeRight] {
                    let hand = now[rig.extremity_joints(limb)[0]];
                    let along = ((hand - from).dot(axis) / axis.length_squared().max(f32::EPSILON))
                        .clamp(0.0, 1.0);
                    let clear = (hand - (from + axis * along)).length();
                    assert!(
                        clear > widest,
                        "at {pace} m/s a hand came {:.0} mm from the trunk axis, inside its own \
                         {:.0} mm half-width",
                        clear * 1000.0,
                        widest * 1000.0,
                    );
                }
            }
        }
    }
}
