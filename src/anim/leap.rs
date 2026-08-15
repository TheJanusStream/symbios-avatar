//! Leaving the ground on purpose, and arriving back.
//!
//! A jump is three motions that have to agree at two instants, and the whole
//! difficulty is the agreeing. The body winds up, flies, and lands; the wind-up
//! ends at exactly the speed the flight begins with, and the flight ends at
//! exactly the speed the landing has to absorb. Give the three phases
//! independent shapes and the body kinks twice — once as it leaves and once as
//! it arrives — which reads as the animation changing its mind.
//!
//! So none of them is authored. **The leg is a spring and the body is a
//! projectile**, which is the same model [`super::gait`] uses for a run, and
//! between them they leave exactly one number free:
//!
//! * **The flight is ballistic.** A body with nothing on the ground is under
//!   gravity alone, so its height is a parabola and its duration is `2v/g`.
//!   Nothing here to choose.
//! * **The wind-up and the landing are the same spring**, compressing and
//!   releasing. Their depth follows from the energy the launch needs — or the
//!   energy the arrival brings, which is the same energy — and their duration
//!   is that spring's half period. Also nothing to choose.
//! * **[`LEG_STIFFNESS`]** is the one free number, and it is dimensionless, so
//!   it is the same for a child and a giant.
//!
//! What that buys is that a higher jump crouches deeper and dwells longer
//! before it, all by itself, and that a body dropped from a height lands the
//! way it would land from a jump of that height, because as far as the leg is
//! concerned those are the same arrival.
//!
//! **A fall is a leap that never launched.** [`Leap::falling`] is the whole of
//! it: no wind-up, no upward speed, and a landing that absorbs whatever the
//! drop supplies. The ground vanishing under a walking body is not a different
//! motion, and giving it one would mean two things to keep in agreement.

use glam::Vec3;

use super::ground::{FootingConfig, Ground, plant_feet_of, solve_contact_toward};
use super::pose::Pose;
use super::speed::GRAVITY;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// Stiffness of a leg, as a multiple of the body's weight per unit of leg
/// length.
///
/// **Dimensionless, which is the point.** Written as `k·L/(m·g)`, it is the
/// same number for a child and a giant, so every depth and duration derived
/// from it scales with the body rather than having to be re-picked for one. The
/// gait literature reports a dimensionless vertical stiffness of roughly 10 to
/// 20 for human running, rising with speed as the leg is held stiffer.
///
/// **Ten, the compliant end**, because a jump and a landing are the softest
/// things a leg does: a runner's leg is tuned to return energy quickly and a
/// landing is tuned to absorb it slowly, and taking the stiff end here gives a
/// body that lands like a dropped chair.
pub const LEG_STIFFNESS: f32 = 10.0;

/// How far a body's feet draw up under it at the top of a leap, as a share of
/// the leg's reach.
///
/// **The one number here that is a choice rather than a consequence**, and it
/// is a choice about how a jump reads rather than about physics: a body in
/// flight can hold its legs anywhere, and what it actually does depends on
/// whether it is clearing something, reaching for a landing, or showing off.
/// A fifth of the leg's reach is a plain tuck — enough to be unmistakably
/// airborne, short of the knees-to-chest a jump has to *mean* something to be
/// worth.
pub const TUCK_OF_REACH: f32 = 0.2;

/// One leap, described by the only thing about it that is free.
///
/// Everything else — how high, how long in the air, how deep the wind-up, how
/// long the landing takes — follows from this and the body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leap {
    /// Upward speed at the moment the feet leave, in metres per second.
    launch: f32,
    /// How far below the takeoff point the landing surface is, in metres.
    ///
    /// **Separate from the launch, and keeping them separate is what makes a
    /// fall and a jump one type instead of two.** A jump returns to where it
    /// left, so this is zero; a fall never rises, so the launch is zero; and a
    /// body that jumps off a ledge has both, which needs no third case.
    drop: f32,
}

impl Leap {
    /// A leap that takes off at this speed and lands where it left.
    #[must_use]
    pub fn new(metres_per_second: f32) -> Self {
        Self {
            launch: metres_per_second.max(0.0),
            drop: 0.0,
        }
    }

    /// A leap that takes off at this speed and lands `drop` metres lower —
    /// jumping off a ledge, which is both of the other two at once.
    #[must_use]
    pub fn off_a_ledge(metres_per_second: f32, drop: f32) -> Self {
        Self {
            launch: metres_per_second.max(0.0),
            drop: drop.max(0.0),
        }
    }

    /// A leap that carries the body's root this far above where it stands.
    ///
    /// The inverse of [`Self::apex`]: `v = sqrt(2gh)`.
    #[must_use]
    pub fn to_height(metres: f32) -> Self {
        Self::new((2.0 * GRAVITY * metres.max(0.0)).sqrt())
    }

    /// A fall from `metres` up: no wind-up, no launch, and a landing that
    /// absorbs the drop.
    ///
    /// **The same type, deliberately.** A body whose ground vanished and a body
    /// that jumped are doing the same thing from the moment they are airborne,
    /// and the leg cannot tell the difference on arrival. Modelling them apart
    /// would mean two descriptions to keep in agreement across the one instant
    /// where it matters.
    #[must_use]
    pub fn falling(metres: f32) -> Self {
        Self {
            launch: 0.0,
            drop: metres.max(0.0),
        }
    }

    /// Upward speed at takeoff, in metres per second. Zero for a fall, which
    /// does not push off.
    #[must_use]
    pub fn launch(self) -> f32 {
        self.launch
    }

    /// How far the root rises above where it left, in metres.
    ///
    /// Zero for a fall, which never rises.
    #[must_use]
    pub fn apex(self) -> f32 {
        self.launch * self.launch / (2.0 * GRAVITY)
    }

    /// How long the body is off the ground, in seconds.
    ///
    /// The time for the arc to come back to a floor `drop` metres below where
    /// it left: the positive root of `vt - gt²/2 = -drop`. A jump lands where
    /// it took off and that reduces to the familiar `2v/g`; a fall has no
    /// launch and it reduces to `sqrt(2·drop/g)`; a leap off a ledge is neither
    /// and needs no case of its own.
    #[must_use]
    pub fn flight(self) -> f32 {
        (self.launch + (self.launch * self.launch + 2.0 * GRAVITY * self.drop).sqrt()) / GRAVITY
    }

    /// How fast the body arrives, in metres per second.
    ///
    /// **Energy, so the launch and the drop add**: `½v² + g·h` on the way to
    /// `½v_impact²`. A jump arrives at the speed it left with, because gravity
    /// is symmetric; a fall arrives at whatever the height gave it; and a leap
    /// off a ledge arrives faster than either.
    #[must_use]
    pub fn impact(self) -> f32 {
        (self.launch * self.launch + 2.0 * GRAVITY * self.drop).sqrt()
    }

    /// How far the leg compresses under the launch or the arrival, in metres.
    ///
    /// **Energy, not a curve.** The spring has to store what the launch needs —
    /// `½mv²` into `½kΔ²` — so `Δ = v·sqrt(m/k)`, which in the dimensionless
    /// stiffness of [`LEG_STIFFNESS`] is
    ///
    /// ```text
    /// Δ/L = sqrt( v² / (g·L·k̃) )
    /// ```
    ///
    /// the square root of a vertical Froude number over the stiffness. A higher
    /// jump therefore crouches deeper without anything being told to, and a
    /// body dropped from a height lands exactly as deep as it would have
    /// crouched to jump that high.
    ///
    /// Clamped so a leg cannot be asked to compress past its own length, which
    /// is a body that should be falling over rather than crouching.
    #[must_use]
    pub fn squash(self, rig: &Rig) -> f32 {
        let leg = leg_of(rig);
        if leg <= f32::EPSILON {
            return 0.0;
        }
        let froude = self.impact() * self.impact() / (GRAVITY * leg);
        (leg * (froude / LEG_STIFFNESS).sqrt()).min(leg * MAX_SQUASH)
    }

    /// How long the wind-up before takeoff lasts, in seconds.
    ///
    /// **Half of the spring's period**, `π·sqrt(m/k)`, which is how long a mass
    /// on a spring takes to go down and come back — so it is derived from the
    /// same stiffness as the depth and cannot disagree with it. Zero for a fall,
    /// which does not get one.
    #[must_use]
    pub fn wind_up(self, rig: &Rig) -> f32 {
        if self.launch <= f32::EPSILON {
            return 0.0;
        }
        contact_time(rig)
    }

    /// How long the landing takes to absorb and rise again, in seconds.
    ///
    /// The same half period as the wind-up. The two are the same spring doing
    /// the same thing in opposite directions.
    #[must_use]
    pub fn landing(self, rig: &Rig) -> f32 {
        contact_time(rig)
    }

    /// How long the whole leap takes, wind-up to standing again.
    #[must_use]
    pub fn duration(self, rig: &Rig) -> f32 {
        self.wind_up(rig) + self.flight() + self.landing(rig)
    }

    /// Where the body is at `elapsed` seconds into the leap.
    #[must_use]
    pub fn stage_at(self, rig: &Rig, elapsed: f32) -> Stage {
        let elapsed = elapsed.max(0.0);
        let wind_up = self.wind_up(rig);
        if elapsed < wind_up {
            return Stage::WindUp(elapsed / wind_up.max(f32::EPSILON));
        }
        let flight = self.flight();
        if elapsed < wind_up + flight {
            return Stage::Flight((elapsed - wind_up) / flight.max(f32::EPSILON));
        }
        let landing = self.landing(rig);
        let through = (elapsed - wind_up - flight) / landing.max(f32::EPSILON);
        if through >= 1.0 {
            Stage::Standing
        } else {
            Stage::Landing(through)
        }
    }

    /// How far the root sits above where it stands, in metres, at `elapsed`.
    ///
    /// Negative through the wind-up and the landing, where the spring is
    /// compressed; positive through the flight. **Continuous across both seams
    /// by construction** rather than by blending: the spring is at full
    /// extension at the instant the feet leave and at the instant they land, so
    /// all three pieces meet at zero.
    #[must_use]
    pub fn height_at(self, rig: &Rig, elapsed: f32) -> f32 {
        let squash = self.squash(rig);
        match self.stage_at(rig, elapsed) {
            // Down and back up: a half turn of the spring, which puts full
            // compression at the middle and nothing at either end.
            Stage::WindUp(t) => -squash * (t * std::f32::consts::PI).sin(),
            Stage::Flight(t) => {
                // The parabola gravity draws, in seconds from takeoff. It ends
                // at `-drop` by construction, because that is the root
                // `flight()` solved for.
                let at = t * self.flight();
                self.launch * at - 0.5 * GRAVITY * at * at
            }
            // Measured from the floor the body landed ON, which is `drop` below
            // the one it left. A jump makes those the same and a fall does not,
            // and forgetting the difference put a half-metre step at the seam.
            Stage::Landing(t) => -self.drop - squash * (t * std::f32::consts::PI).sin(),
            Stage::Standing => -self.drop,
        }
    }

    /// How far below the takeoff point the body lands, in metres. Zero for a
    /// jump, which returns to where it left.
    #[must_use]
    pub fn drop(self) -> f32 {
        self.drop
    }

    /// Poses one frame of the leap.
    ///
    /// `ground` answers what lies beneath a point, exactly as [`super::Walk`]
    /// requires, and it is used for the same thing: settling the contacts while
    /// the body has any on the ground. **In flight it is not consulted**, which
    /// is the whole difference — a body with nothing under it is not being
    /// solved onto anything, and asking would drag its feet back down to a floor
    /// it has left.
    pub fn drive<F>(&self, rig: &Rig, pose: &mut Pose, elapsed: f32, ground: F) -> Leapt
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let mut leapt = Leapt {
            stage: self.stage_at(rig, elapsed),
            height: 0.0,
            straining: Vec::new(),
        };
        if !pose.fits(rig) {
            return leapt;
        }
        leapt.height = self.height_at(rig, elapsed);

        match leapt.stage {
            Stage::Flight(t) => {
                // **Rigid, and the feet come with it.** A projectile does not
                // change shape, so the root simply moves and the legs keep
                // whatever they are holding — which is the tuck below, applied
                // in the body's own frame and therefore carried along with it.
                self.tuck(rig, pose, t, &mut leapt);
                pose.translation.y += leapt.height;
            }
            Stage::WindUp(_) | Stage::Landing(_) => {
                // Feet on the floor, body dropping between them: the legs have
                // to bend, so the sink goes in BEFORE the contacts are solved.
                pose.translation.y += leapt.height;
                let contacts = rig.ground_contacts();
                let footing =
                    plant_feet_of(rig, pose, &contacts, ground, &FootingConfig::default());
                leapt.straining = footing.straining;
            }
            Stage::Standing => {}
        }
        leapt
    }

    /// Draws the feet up under the body through the flight.
    ///
    /// Peaks at mid-flight and returns to nothing at both ends, so the legs are
    /// straight at the instant they leave and the instant they land — which is
    /// what lets the landing spring start from a known shape rather than from
    /// wherever the tuck happened to be.
    fn tuck(&self, rig: &Rig, pose: &mut Pose, through: f32, leapt: &mut Leapt) {
        let reach = leg_of(rig);
        let lift = reach * TUCK_OF_REACH * (through * std::f32::consts::PI).sin();
        if lift <= f32::EPSILON {
            return;
        }
        for limb in rig.ground_contacts() {
            let (Some(chain), Some(&contact)) = (
                rig.limb_chain(limb),
                rig.in_zone(Zone::Extremity(limb)).first(),
            ) else {
                continue;
            };
            let Some(pole) = rig.bend_pole(limb) else {
                continue;
            };
            // Toward the hip rather than straight up, so a body tucking while
            // it is pitched draws its legs under ITSELF and not toward the sky.
            let hip = rig.joints[chain[0]].position;
            let home = rig.joints[contact].position;
            let toward = (hip - home).normalize_or(Vec3::Y);
            if !solve_contact_toward(rig, pose, limb, home + toward * lift, pole) {
                leapt.straining.push(limb);
            }
        }
    }
}

/// The deepest a leg will be asked to compress, as a share of its own reach.
///
/// A leg squashed past this is not crouching, it is collapsing, and the honest
/// answer for a body arriving that fast is that it should be falling over —
/// which is a motion this does not have and should not fake.
const MAX_SQUASH: f32 = 0.45;

/// How long the leg spends in contact through a wind-up or a landing, in
/// seconds.
///
/// Half the period of the spring it is: `π·sqrt(m/k)`, which in the
/// dimensionless [`LEG_STIFFNESS`] is `π·sqrt(L/(k̃·g))`. On a 0.71 m leg that
/// is about 270 ms, which is the order a countermovement takes.
fn contact_time(rig: &Rig) -> f32 {
    let leg = leg_of(rig);
    if leg <= f32::EPSILON {
        return 0.0;
    }
    std::f32::consts::PI * (leg / (LEG_STIFFNESS * GRAVITY)).sqrt()
}

/// The reach of the longest leg the body stands on, in metres.
fn leg_of(rig: &Rig) -> f32 {
    rig.ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max)
}

/// Which part of a leap a body is in, and how far through that part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    /// Compressing before takeoff, `0..1`.
    WindUp(f32),
    /// Off the ground, `0..1`.
    Flight(f32),
    /// Absorbing the arrival, `0..1`.
    Landing(f32),
    /// Done, and standing.
    Standing,
}

impl Stage {
    /// Whether the body has anything on the ground.
    #[must_use]
    pub fn is_grounded(self) -> bool {
        !matches!(self, Stage::Flight(_))
    }
}

/// What one frame of a leap did.
#[derive(Clone, Debug, PartialEq)]
pub struct Leapt {
    /// Which part of the leap this frame was.
    pub stage: Stage,
    /// How far the root sat above where it stands, in metres.
    pub height: f32,
    /// Contacts whose goal was out of reach.
    pub straining: Vec<Limb>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::transition::{Entry, Family, entry};
    use crate::anim::{Gait, Speed};
    use crate::plan::{BodyPlan, HumanoidParams};

    fn body(height: f32) -> Rig {
        Rig::from_skeleton(
            &HumanoidParams {
                height,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
    }

    fn biped() -> Rig {
        body(1.75)
    }

    fn level() -> impl Fn(Vec3) -> Option<Ground> + Copy {
        |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)))
    }

    #[test]
    fn the_flight_is_the_arc_gravity_draws_and_not_a_curve_that_looks_like_one() {
        // Nothing to choose here, and the test says so: the height through the
        // flight is `vt - gt²/2` to the millimetre, the apex is `v²/2g`, and the
        // duration is `2v/g`.
        let rig = biped();
        for height in [0.15f32, 0.3, 0.6] {
            let leap = Leap::to_height(height);
            let wind_up = leap.wind_up(&rig);
            assert!(
                (leap.apex() - height).abs() < 1e-4,
                "asked for {height} m and got {:.4}",
                leap.apex()
            );
            assert!((leap.flight() - 2.0 * leap.launch() / GRAVITY).abs() < 1e-5);

            for sample in 0..=40 {
                let t = leap.flight() * sample as f32 / 40.0;
                let wanted = leap.launch() * t - 0.5 * GRAVITY * t * t;
                let got = leap.height_at(&rig, wind_up + t);
                assert!(
                    (got - wanted).abs() < 1e-3,
                    "at {t:.3} s of flight the root was at {got:.4} m, not {wanted:.4}"
                );
            }
        }
    }

    #[test]
    fn a_higher_jump_crouches_deeper_without_being_told_to() {
        // The depth is the energy the launch needs, so it follows the height on
        // its own — as the square root of it, which is what an energy store
        // does.
        let rig = biped();
        let squash = |height: f32| Leap::to_height(height).squash(&rig);
        assert!(
            squash(0.4) > squash(0.2),
            "a higher jump must crouch deeper"
        );
        // Four times the height is twice the depth: E = kΔ²/2 against E = mgh.
        let ratio = squash(0.4) / squash(0.1);
        assert!(
            (ratio - 2.0).abs() < 0.02,
            "quadrupling the height should double the crouch, got {ratio:.3}"
        );
        assert_eq!(Leap::to_height(0.0).squash(&rig), 0.0);
    }

    #[test]
    fn a_fall_lands_exactly_as_a_jump_of_that_height_would_have_taken_off() {
        // **The reason a fall is the same type.** The leg cannot tell the
        // difference between storing the energy for a launch and absorbing the
        // energy of an arrival, because they are the same energy — so a body
        // dropped a third of a metre lands as deep as it would have crouched to
        // jump one.
        let rig = biped();
        for metres in [0.1f32, 0.3, 0.6] {
            let jumped = Leap::to_height(metres);
            let fell = Leap::falling(metres);
            assert!((jumped.squash(&rig) - fell.squash(&rig)).abs() < 1e-5);
            assert!((jumped.impact() - fell.impact()).abs() < 1e-4);
            // And a fall has no wind-up, because nothing chose it.
            assert_eq!(fell.wind_up(&rig), 0.0);
            assert!(jumped.wind_up(&rig) > 0.1);
            assert_eq!(fell.apex(), 0.0, "a fall never rises");
            assert!((fell.drop() - metres).abs() < 1e-4);
        }
    }

    #[test]
    fn the_body_does_not_kink_where_one_phase_hands_over_to_the_next() {
        // **The whole difficulty of a jump, asserted directly.** Three motions
        // meet at two instants, and if they meet at different heights the body
        // steps. They meet at zero by construction — the spring is at full
        // extension when the feet leave and when they land — and this is what
        // says the construction held.
        let rig = biped();
        for leap in [Leap::to_height(0.35), Leap::falling(0.5)] {
            let (wind_up, flight) = (leap.wind_up(&rig), leap.flight());
            for seam in [wind_up, wind_up + flight] {
                let before = leap.height_at(&rig, seam - 1e-4);
                let after = leap.height_at(&rig, seam + 1e-4);
                assert!(
                    (before - after).abs() < 1e-3,
                    "the root stepped {:.2} mm at the seam {seam:.3} s in",
                    (before - after).abs() * 1000.0
                );
            }
            // And nowhere across the whole leap does it move faster than the
            // launch did, which is what a step would show up as.
            let total = leap.duration(&rig);
            let step = total / 600.0;
            let mut worst = 0.0f32;
            for sample in 1..600 {
                let now = leap.height_at(&rig, step * sample as f32);
                let then = leap.height_at(&rig, step * (sample - 1) as f32);
                worst = worst.max((now - then).abs() / step);
            }
            assert!(
                worst < leap.impact() * 1.6,
                "the root moved at {worst:.2} m/s against an impact of {:.2}",
                leap.impact()
            );
        }
    }

    #[test]
    fn the_same_leap_is_the_same_leap_on_any_body() {
        // Dimensionless throughout, so a child and a giant jumping at the same
        // vertical Froude number crouch by the same share of their legs and take
        // the same number of their own time units doing it.
        let (small, large) = (body(1.2), body(2.1));
        let froude = 0.5f32;
        let scaled = |rig: &Rig| {
            let leg = super::leg_of(rig);
            let leap = Leap::new((froude * GRAVITY * leg).sqrt());
            (
                leap.squash(rig) / leg,
                leap.wind_up(rig) / (leg / GRAVITY).sqrt(),
            )
        };
        let (a_squash, a_time) = scaled(&small);
        let (b_squash, b_time) = scaled(&large);
        assert!(
            (a_squash - b_squash).abs() < 1e-5,
            "{a_squash} vs {b_squash}"
        );
        assert!((a_time - b_time).abs() < 1e-5);
        // And in metres they differ, or nothing scaled at all.
        assert!(super::contact_time(&large) > super::contact_time(&small) * 1.2);
    }

    #[test]
    fn a_body_in_flight_is_not_solved_onto_the_floor_it_has_left() {
        // The one thing the ground closure must NOT do while the body is
        // airborne. Given a floor at the body's own feet, a stage that consulted
        // it would drag them straight back down.
        let rig = biped();
        let leap = Leap::to_height(0.4);
        let mid = leap.wind_up(&rig) + leap.flight() * 0.5;

        let mut pose = Pose::rest(&rig);
        let leapt = leap.drive(&rig, &mut pose, mid, level());
        assert!(matches!(leapt.stage, Stage::Flight(_)));
        assert!(!leapt.stage.is_grounded());
        assert!(
            (leapt.height - leap.apex()).abs() < 1e-3,
            "mid-flight should be at the apex: {:.3} against {:.3}",
            leapt.height,
            leap.apex()
        );

        let posed = pose.forward(&rig);
        for limb in rig.ground_contacts() {
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            let rest = rig.joints[foot].position.y;
            assert!(
                posed.positions[foot].y > rest + leap.apex() * 0.5,
                "{limb:?} was left near the floor while the body was at its apex"
            );
        }
    }

    #[test]
    fn the_legs_are_straight_again_before_they_land_on_them() {
        // The tuck peaks mid-flight and returns to nothing, so the landing
        // spring starts from a known shape rather than from wherever the tuck
        // happened to be when the ground arrived.
        let rig = biped();
        let leap = Leap::to_height(0.4);
        let wind_up = leap.wind_up(&rig);
        let foot = rig.in_zone(Zone::Extremity(Limb::HindLeft))[0];

        let drawn_up = |t: f32| {
            let mut pose = Pose::rest(&rig);
            let _ = leap.drive(&rig, &mut pose, t, level());
            // Against the root, so the flight's own rise does not count as a
            // tuck: what is asked is how far the foot came up under the BODY.
            let posed = pose.forward(&rig);
            let root = posed.positions[0].y - rig.joints[0].position.y;
            (posed.positions[foot].y - root) - rig.joints[foot].position.y
        };

        let middle = drawn_up(wind_up + leap.flight() * 0.5);
        assert!(
            middle > super::leg_of(&rig) * TUCK_OF_REACH * 0.5,
            "the feet barely came up: {middle:.3} m"
        );
        for edge in [wind_up + 1e-3, wind_up + leap.flight() - 1e-3] {
            assert!(
                drawn_up(edge).abs() < 5e-3,
                "the legs were still tucked at the edge of the flight: {:.3} m",
                drawn_up(edge)
            );
        }
    }

    #[test]
    fn a_leg_is_asked_to_crouch_and_never_to_collapse() {
        // A body arriving fast enough to fold its legs past this is a body that
        // should be falling over, which is a motion this has not got and must
        // not fake.
        let rig = biped();
        let leg = super::leg_of(&rig);
        for drop in [1.0f32, 4.0, 20.0, 100.0] {
            let squash = Leap::falling(drop).squash(&rig);
            assert!(
                squash <= leg * MAX_SQUASH + 1e-6,
                "a {drop} m fall asked the leg to compress {squash:.3} m of {leg:.3}"
            );
        }
        assert!(Leap::falling(20.0).squash(&rig) > Leap::falling(0.2).squash(&rig));
    }

    #[test]
    fn jumping_off_a_ledge_is_both_at_once_and_needs_no_case_of_its_own() {
        // The reason the launch and the drop are separate fields. A body that
        // pushes off AND lands lower rises like the jump and arrives like the
        // fall, and the arithmetic that gets both right is one expression.
        let rig = biped();
        let launch = Leap::to_height(0.3).launch();
        let ledge = Leap::off_a_ledge(launch, 0.8);

        assert!(
            (ledge.apex() - 0.3).abs() < 1e-4,
            "it still rises by the launch"
        );
        assert!(
            ledge.impact() > Leap::falling(0.8).impact(),
            "it arrives faster than the drop alone"
        );
        assert!(
            ledge.flight() > Leap::falling(0.8).flight(),
            "it is airborne longer than the drop alone"
        );
        // Energy: half v squared plus g h, both ways.
        let wanted = (launch * launch + 2.0 * GRAVITY * 0.8).sqrt();
        assert!((ledge.impact() - wanted).abs() < 1e-3);
        // And it ends a ledge lower than it began, not back where it left.
        let done = ledge.height_at(&rig, ledge.duration(&rig) + 1e-3);
        assert!(
            (done + 0.8).abs() < 1e-4,
            "it finished at {done:.3} m, not -0.800"
        );
    }

    #[test]
    fn a_leap_ends_standing_where_it_started() {
        let rig = biped();
        let leap = Leap::to_height(0.3);
        let mut pose = Pose::rest(&rig);
        let leapt = leap.drive(&rig, &mut pose, leap.duration(&rig) + 1e-3, level());
        assert_eq!(leapt.stage, Stage::Standing);
        assert_eq!(leapt.height, 0.0, "a jump returns to where it took off");
        assert!(leapt.straining.is_empty());
        assert!(Stage::Standing.is_grounded());
    }

    #[test]
    fn a_fall_does_not_wait_for_a_polite_moment() {
        // #243's small case: no jump input, the ground vanishes. The governor
        // holds an elective change until support transfers, so a planted foot is
        // never slid — but a body whose floor has gone has no planted foot to
        // protect, and waiting to fall is not something a body does.
        let rig = biped();
        let gait = Speed::new(&rig, 1.4).gait(&rig);
        // Mid-stance, where an elective change would be made to wait.
        let mid = 0.45;
        assert!(matches!(entry(&gait, mid, Family::Jump), Entry::Wait(_)));
        // Airborne already: nothing down, nothing to disturb.
        let flying = Gait {
            duty: 0.0,
            ..gait.clone()
        };
        assert!(entry(&flying, mid, Family::Jump).is_now());
    }
}
