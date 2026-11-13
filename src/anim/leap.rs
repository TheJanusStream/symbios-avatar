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
//! **A winged body's leg is the one it stands on** (#456, #459): see
//! `leg_of`. Its bones reach far past where it stands its feet - a bat's
//! wing-arm 0.70 m against a wrist 0.28 m off its shoulder - so its leap is
//! sized, squashed, pitched and tucked on its legs as they stand, and its
//! feet are tucked, planted and set down square to the ground they leave
//! and land on, its flight's trunk sunk as its plant sinks it
//! (`stand_square`, `sink_to`).
//!
//! **A fall is a leap that never launched.** [`Leap::falling`] is the whole of
//! it: no wind-up, no upward speed, and a landing that absorbs whatever the
//! drop supplies. The ground vanishing under a walking body is not a different
//! motion, and giving it one would mean two things to keep in agreement.

use crate::det::{self, DetMath};
use glam::Vec3;

use super::ground::{Foot, FootingConfig, Ground, plant_feet_of, solve_contact_toward};
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
        (leg * (froude / LEG_STIFFNESS).sqrt()).min(leg * squash_most(rig))
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
            Stage::WindUp(t) => -squash * (t * std::f32::consts::PI).det_sin(),
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
            Stage::Landing(t) => -self.drop - squash * (t * std::f32::consts::PI).det_sin(),
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
        self.drive_riding(rig, pose, elapsed, &ground, &ground, None)
    }

    /// How far a four-legged body's trunk is pitched nose up at `stage`, in
    /// radians, nose down negative, the landing set off from `landing_from`
    /// where it is given (#414); nothing for any other body.
    fn pitched_at(self, rig: &Rig, stage: Stage, landing_from: Option<f32>) -> f32 {
        if !rig.four_legged() {
            return 0.0;
        }
        let ease = |t: f32| t * t * (3.0 - 2.0 * t);
        let most = leap_pitch(rig);
        match stage {
            Stage::WindUp(t) => most * ease(t),
            Stage::Flight(t) => most * (t * std::f32::consts::PI).det_cos(),
            Stage::Landing(t) => landing_from.unwrap_or(-most) * (1.0 - ease(t)),
            Stage::Standing => 0.0,
        }
    }

    /// How far a four-legged body's trunk is pitched `elapsed` seconds into
    /// this leap, as [`Self::drive`] pitches it: where a landing rebuilt at
    /// touchdown sets off from (#414).
    pub(crate) fn pitch_at(self, rig: &Rig, elapsed: f32) -> f32 {
        self.pitched_at(rig, self.stage_at(rig, elapsed), None)
    }

    /// [`Self::drive`], a four-legged body riding its girdles (#413) where
    /// `riding` is given, as the driver keeps what it rode last: `took_off` is the
    /// ground under its girdles on the last frame it stood on any, or `None`
    /// for the level. The public [`Self::drive`] rides nothing, standing a body
    /// level throughout as it always has.
    ///
    /// **The wind-up and the landing ride the ground they stand on, and the
    /// flight what it left.** The flight still reads no ground: a body in the
    /// air rides the girdles it took off from, as its last grounded frame stood
    /// it. Read in the air instead, the ground a consumer answers under a body
    /// its chassis carries is nothing to stand on, and a switch of what it
    /// answers at takeoff or touchdown turned a trunk 24 degrees in a frame.
    /// The landing rides what it comes down on as the driver hands it, which
    /// follows that ground from what the flight rode a few degrees a frame;
    /// eased over the stage as well, the two turns added, and a dog landing on
    /// a 25 degree slope from the level turned 5.8 degrees in a frame.
    ///
    /// **And the landing sets off from the pitch the flight came down in**
    /// (#414), `landing_from`, where the driver rebuilt the leap at touchdown:
    /// a fall's flight is cut short wherever the ground arrives, and from the
    /// landing's own nose down a beast stepping off a 0.6 m ledge pitched its
    /// trunk 24 degrees in the frame it came down.
    pub(crate) fn drive_riding<F, G>(
        &self,
        rig: &Rig,
        pose: &mut Pose,
        elapsed: f32,
        ground: F,
        under: G,
        riding: Option<Riding>,
    ) -> Leapt
    where
        F: Fn(Vec3) -> Option<Ground>,
        G: Fn(Vec3) -> Option<Ground>,
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

        // The arms hang throughout (#267): a leap says nothing about them, and
        // a body that jumps in the bind pose's splay reads as a thrown
        // mannequin. Before the stages, so the flight's rigid translation
        // carries hung arms and a landing's solve bends legs under them.
        super::gait::hang_arms(rig, pose);

        // **A beast leaps fore and hind in turn** (#370): it gathers on its
        // haunches, nose up, arcs over through the flight and lands on its
        // fores, nose down - the trunk turned by the end it sinks, as its gait
        // rocks, so one stage hands the next the same pitch.
        // Turned after the tuck in the air, which draws the legs up in the
        // body's own frame, and before the plant on the ground, which puts
        // the feet back down under it.
        let (riding, took_off, landing_from) = match riding {
            Some(Riding {
                took_off,
                landing_from,
            }) => (true, took_off, landing_from),
            None => (false, None, None),
        };
        let pitch = self.pitched_at(rig, leapt.stage, landing_from);
        // **And a beast that rides its girdles leaps from them and lands on
        // them** (#413): stood level for the leap, a dog on a 25 degree slope
        // turned its trunk 24 degrees flat in five frames as it gathered. Only
        // where the driver keeps what it took off from: see above.
        let ridden = |pose: &mut Pose, girdles: Option<super::quadruped::Girdles>| {
            if let Some(girdles) = girdles {
                super::quadruped::ride_ground(rig, pose, girdles);
            }
        };
        let standing_on = |ground: &F| {
            if riding {
                super::quadruped::girdle_ground(rig, ground)
            } else {
                None
            }
        };
        let four = rig.four_legged();
        match leapt.stage {
            Stage::Flight(t) => {
                // **Rigid, and the feet come with it.** A projectile does not
                // change shape, so the root simply moves and the legs keep
                // whatever they are holding — which is the tuck below, applied
                // in the body's own frame and therefore carried along with it.
                // A winged body is tucked once its trunk is turned:
                // see below.
                let winged = on_wings(rig);
                if !winged {
                    self.tuck(rig, pose, t, &mut leapt, None);
                }
                // **The turn of what it left, not its height** (#414): the
                // ground under a body about to drop is falling away before the
                // drop is seen, and the height followed down with it, kept
                // through the flight, flew a dog stepping off a 0.6 m ledge
                // 0.45 m under its own body and lifted it all of that in the
                // frame it landed.
                let took_off = took_off.filter(|_| riding && four);
                // **And no paw under the plane it stands on** (#414): pitched
                // nose down to come down on its fores, a horse's forepaws hung
                // 198 mm under that plane before they touched, and the landing
                // stood them back on it in one frame. Drawn up onto it, as the
                // landing would stand them; a paw above it is left in the air.
                // The plane is the one it left, turned as it rides it: under a
                // beast leaping up a hill, the hill.
                //
                // **A winged body on four takes out the height the plane it
                // left has under its origin, not under its girdles' middle**
                // (#456): a bat's origin stands near its hips, well behind
                // that middle, and with the middle's height taken out its
                // trunk flew 79 mm over where its landing set it down on a 20
                // degree slope it faced down, and dropped all of it in the
                // frame it landed.
                let turned = took_off.map(|girdles| {
                    if winged {
                        girdles.turned_at_origin(rig)
                    } else {
                        girdles.turned()
                    }
                });
                let stands = |at: Vec3| {
                    turned
                        .and_then(|girdles| girdles.plane_under(rig, at))
                        .unwrap_or_else(|| Ground::level(Vec3::new(at.x, 0.0, at.z)))
                };
                let plane = |at: Vec3| {
                    let stands = stands(at);
                    (at.y < stands.position.y).then_some(stands)
                };
                if pitch != 0.0 {
                    super::quadruped::sink_end(rig, pose, pitch);
                }
                ridden(pose, turned);
                if winged {
                    // **Tucked in the frame the trunk is turned to, each foot
                    // held off the plane it left in the one solve** (#459):
                    // tucked first and planted after, two solves that read a
                    // short, folded leg apart swung a glider's hind hip about
                    // half a turn in the frame a paw came up off that plane.
                    // Held off the plane whether or not the body rides its
                    // girdles: the level it stands on where it does not, so
                    // a wind-up planted on it hands its feet to the flight
                    // where they stood.
                    let onto = |at: Vec3| Some(stands(at));
                    sink_to(rig, pose, &onto);
                    self.tuck(rig, pose, t, &mut leapt, Some(&onto));
                } else if riding && four {
                    let contacts = rig.ground_contacts();
                    let config = FootingConfig {
                        max_pelvis_drop: 0.0,
                        max_step_up: stepped(rig),
                        ..FootingConfig::default()
                    };
                    plant_feet_of(rig, pose, &contacts, plane, &config);
                }
                pose.translation.y += leapt.height;
            }
            Stage::WindUp(_) | Stage::Landing(_) => {
                // Feet on the floor, body dropping between them: the legs have
                // to bend, so the sink goes in BEFORE the contacts are solved.
                pose.translation.y += leapt.height;
                let girdles = standing_on(&ground);
                let contacts = rig.ground_contacts();
                // **A beast's paw is stood on the ground however far under it
                // the landing put it, as far as its leg reaches** (#414):
                // pitched nose down onto its fores, a horse landing a level
                // jump had its forepaws 350 mm under the ground, past the 0.4 m
                // a plant steps a foot up, for six frames, and then up on it in
                // one, 435 mm. In the wind-up as at the landing, and for any
                // caller: a leap's ground is the ground it stands on, not a
                // step. (Folded that far, a horse landing 1.5 m down onto a 20
                // degree hill swings a fore knee over at 144 frames a second;
                // a step any shorter leaves its paws under the ground there
                // and snaps them up later: see the issue.)
                //
                // **And the landing takes up the ground really under them**
                // (#413), `under`, from the ground the trunk follows: coming
                // down from a level floor answered in the air onto a 20 degree
                // hill, the uphill paws stood on the followed plane, a horse's
                // 281 mm under the hill. Over the first [`TAKEN_UP`] of the
                // stage rather than at once: stood on the hill in the frame they
                // touched it, the legs went 9 times what a frame allows.
                if pitch != 0.0 {
                    super::quadruped::sink_end(rig, pose, pitch);
                }
                ridden(pose, girdles);
                let config = if four {
                    FootingConfig {
                        max_step_up: stepped(rig),
                        ..FootingConfig::default()
                    }
                } else {
                    FootingConfig::default()
                };
                let took = match leapt.stage {
                    Stage::Landing(t) if riding && four => {
                        let took = (t / TAKEN_UP).min(1.0);
                        Some(took * took * (3.0 - 2.0 * took))
                    }
                    _ => None,
                };
                let beneath = |at: Vec3| {
                    let Some(took) = took else {
                        return ground(at);
                    };
                    let (followed, real) = (ground(at), under(at));
                    match (followed, real) {
                        (Some(followed), Some(real)) => Some(Ground {
                            position: followed.position.lerp(real.position, took),
                            normal: followed
                                .normal
                                .lerp(real.normal, took)
                                .normalize_or(Vec3::Y),
                        }),
                        (followed, real) => real.or(followed),
                    }
                };
                let footing = plant_feet_of(rig, pose, &contacts, beneath, &config);
                // **And a winged body's feet stood square to that ground**
                // (#456's fixer), as its tuck holds them off the plane it
                // left: see `stand_square`.
                if on_wings(rig) {
                    stand_square(rig, pose, &footing.planted, &beneath);
                }
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
    ///
    /// **A winged body tucks each leg by its own standing length, from
    /// the first frame of the flight, as its stance stands it** (#456, #459),
    /// once its trunk is turned ([`Self::drive_riding`]), and keeps each foot
    /// off `onto` - the plane it took off from - in the same solve. Sized on
    /// the longest leg's bone reach, a bat's hind leg, 0.159 m from hip to
    /// pad, was asked to draw its foot 0.14 m up its wing-arm's reach of 0.70,
    /// and its hip turned over in a frame twelve times in a flight. Left in
    /// its rest pose until the tuck began and then solved, its legs came out
    /// of the stance's bend into the rig's own and back - a glider's hind hip
    /// 2.5 rad off the idle's - and turned over as the solve took over;
    /// solved as the plant solves a leg (its foot stood, about the pole the
    /// plant bends it about), a flight ends in the pose its landing begins in.
    /// **Its foot stood square to `onto`** (#456's fixer), as its landing
    /// stands it ([`stand_square`]).
    fn tuck(
        &self,
        rig: &Rig,
        pose: &mut Pose,
        through: f32,
        leapt: &mut Leapt,
        onto: Option<&dyn Fn(Vec3) -> Option<Ground>>,
    ) {
        if on_wings(rig) {
            self.tuck_winged(rig, pose, through, leapt, onto);
            return;
        }
        let reach = leg_of(rig);
        let lift = reach * TUCK_OF_REACH * (through * std::f32::consts::PI).det_sin();
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

    /// [`Self::tuck`] for a winged body (#456, #459).
    fn tuck_winged(
        &self,
        rig: &Rig,
        pose: &mut Pose,
        through: f32,
        leapt: &mut Leapt,
        onto: Option<&dyn Fn(Vec3) -> Option<Ground>>,
    ) {
        let share = TUCK_OF_REACH * (through * std::f32::consts::PI).det_sin();
        let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
            return;
        };
        let posed = pose.forward(rig);
        // Where a point of the rest pose is carried by the trunk as it is
        // turned: the tuck draws the legs up in the body's own frame.
        let carried = |at: Vec3| {
            posed.positions[root] + posed.rotations[root] * (at - rig.joints[root].position)
        };
        for limb in rig.ground_contacts() {
            let (Some(chain), Some(&contact), Some(pole), Some(leg)) = (
                rig.limb_chain(limb),
                rig.in_zone(Zone::Extremity(limb)).first(),
                rig.bend_pole(limb),
                standing_leg(rig, limb),
            ) else {
                continue;
            };
            let hip = carried(rig.joints[chain[0]].position);
            let home = carried(rig.joints[contact].position);
            let toward = (hip - home).normalize_or(Vec3::Y);
            let mut target = home + toward * (leg * share);
            // Off the plane as the landing stands it - raised straight up,
            // as the plant raises a foot, to stand on it - and square to it,
            // as the landing stands it (`stand_square`), so the drawn toes'
            // own lift after the blend finds nothing to do in the air or on
            // the ground. Held off along its up instead, a foot was set 20 mm
            // back from where the landing put it down; and held upright, its
            // toes went into the slope as it landed and their lift raised it
            // in that frame (see `stand_square`).
            let solved = match onto.and_then(|onto| onto(target)) {
                Some(stands) => {
                    target = held_off(rig, contact, target, &stands);
                    let square =
                        det::from_rotation_arc(Vec3::Y, stands.normal.normalize_or(Vec3::Y));
                    super::ground::solve_contact_held(rig, pose, limb, target, pole, square)
                }
                None => {
                    super::ground::solve_contact_with(rig, pose, limb, target, pole, Foot::Stood)
                }
            };
            if !solved {
                leapt.straining.push(limb);
            }
        }
    }
}

/// `target` for the pad `pad`, raised straight up until it stands as high
/// over `below` along its up as the pad stands over the floor at rest - left
/// where it is if it stands higher (#456's fixer).
fn held_off(rig: &Rig, pad: usize, target: Vec3, below: &Ground) -> Vec3 {
    let up = below.normal.normalize_or(Vec3::Y);
    let short = rig.joints[pad].position.y.max(0.0) - (target - below.position).dot(up);
    if short > 0.0 && up.y > f32::EPSILON {
        target + Vec3::Y * (short / up.y)
    } else {
        target
    }
}

/// Sinks a winged body in its flight as far as its plant would sink it on
/// the plane `onto` answers (#456's fixer): by as far as the foot that
/// stands highest over where that plane would stand it stands over it, up
/// to the most a plant sinks a body ([`FootingConfig::max_pelvis_drop`]).
///
/// **A flight's trunk rides where it stood and lands**: held where its
/// ride sets it, a bat's forefeet hung 31 mm over a 20 degree slope it
/// faced down, and its landing's plant sank its trunk that far in the frame
/// it came down - its elbow 50 mm in that frame at 60 and at 144 frames a
/// second. On the level its feet stand where the plane would stand them, and
/// nothing sinks.
fn sink_to(rig: &Rig, pose: &mut Pose, onto: &dyn Fn(Vec3) -> Option<Ground>) {
    let posed = pose.forward(rig);
    let deepest = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .filter_map(|pad| {
            let at = posed.positions[pad];
            let below = onto(at)?;
            Some(below.position.y + rig.joints[pad].position.y.max(0.0) - at.y)
        })
        .fold(0.0f32, f32::min);
    pose.translation.y += deepest.max(-FootingConfig::default().max_pelvis_drop);
}

/// Stands each of `limbs` a winged body's leap has planted square to the
/// ground `beneath` answers under its pad, the pad [`held_off`] it (#456's
/// fixer): what its flight's tuck holds its feet as, off the plane it left.
/// On the level the plant has stood them so already, and they are left.
///
/// **A foot comes down as it lands**: planted upright, as its tuck held it,
/// a winged body's foot on a slope had its toes in it, and
/// `winged::ground::keep_toes_up` lifted the pad out by them after the
/// blend in the frame the body came down - for a Bat x Glider cross at 0.5
/// walking up a 20 degree slope, its wrist 14 mm nearer a shoulder its
/// foreleg stood folded to within 3 mm of as far as it folds, the shoulder
/// turned 0.69 rad in the frame at 60 and at 144 frames a second; and a
/// Glider x Bird cross at 0.3 facing up one folded its hind leg past its
/// fold through the landing, its hip 1.32 rad in a frame at 30.
fn stand_square(
    rig: &Rig,
    pose: &mut Pose,
    limbs: &[Limb],
    beneath: &dyn Fn(Vec3) -> Option<Ground>,
) {
    for &limb in limbs {
        let (Some(&pad), Some(pole)) = (
            rig.in_zone(Zone::Extremity(limb)).first(),
            rig.bend_pole(limb),
        ) else {
            continue;
        };
        let at = pose.forward(rig).positions[pad];
        let Some(below) = beneath(at) else {
            continue;
        };
        let up = below.normal.normalize_or(Vec3::Y);
        if up.y >= 1.0 - 1.0e-6 {
            continue;
        }
        let target = held_off(rig, pad, at, &below);
        super::ground::solve_contact_held(
            rig,
            pose,
            limb,
            target,
            pole,
            det::from_rotation_arc(Vec3::Y, up),
        );
    }
}

/// What the driver hands a leap it keeps (#413, #414): the ground under the
/// girdles on the last frame the body stood on any, or `None` for the level,
/// and the pitch the flight came down in, for a landing rebuilt at touchdown.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Riding {
    pub(crate) took_off: Option<super::quadruped::Girdles>,
    pub(crate) landing_from: Option<f32>,
}

/// How far a beast's leap steps a paw up onto the ground under it, in metres
/// (#414): [`STEPPED`] of its leg, and at least what a plant steps a foot up.
fn stepped(rig: &Rig) -> f32 {
    (STEPPED * leg_of(rig)).max(FootingConfig::default().max_step_up)
}

/// How much of its leg's reach a beast's leap steps a paw up (#414).
/// Provenance: **measured** (#414): over drops of 0.6 and 1.5 m and jumps at
/// 2.5 and 4 m/s, level and on 20 degrees, a dog, a cat and a horse at 60 and
/// 144 frames a second, at half to nine tenths of the leg a horse's paws
/// were left up to 720 mm under the ground and snapped up later, 4.4 to 6.7
/// times what a frame allows; at the whole leg nothing stands under it.
const STEPPED: f32 = 1.0;

/// How far into its landing a beast stands its feet wholly on the ground
/// really under them rather than the ground its trunk follows, as a share of
/// the stage (#413). Provenance: **measured** (#413): taken at once, a horse
/// landing from a level floor answered in the air onto a 20 degree hill moved
/// a leg 9 times what a frame allows at 144 frames a second; over 0.15 to 0.8
/// of the stage the same landing reads 5.4, its worst frame elsewhere.
const TAKEN_UP: f32 = 0.3;

/// How far a leaping beast's trunk turns nose up as it gathers and nose down
/// as it lands, in radians (#370).
/// Provenance: **tuned by test** (#370).
const LEAP_PITCH: f32 = 0.25;

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

/// The reach of the longest leg the body stands on, in metres - or, on a
/// winged body, the shortest of its legs from hip to pad as it stands
/// ([`standing_leg`], #456).
///
/// **A winged body's leg is not its bones' reach**: a bat stands on its
/// wing-arm, whose bones reach 0.70 m and fold to stand its wrist 0.28 m
/// from its shoulder, and on a hind leg reaching 0.34 m that stands its pad
/// 0.16 m from its hip; a glider's legs stand folded to half their reach.
/// Read off the reach, a bat's landing sank its trunk 0.25 m on a hip 0.10 m
/// off the ground, and its spring half period was a quarter of a second for
/// a body a tenth of a metre tall. A person's leg stands straight and a
/// beast's nearly so: theirs is read as it always was.
fn leg_of(rig: &Rig) -> f32 {
    if on_wings(rig) {
        let shortest = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| standing_leg(rig, limb))
            .fold(f32::INFINITY, f32::min);
        return if shortest.is_finite() { shortest } else { 0.0 };
    }
    rig.ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max)
}

/// Whether `rig` is a winged body, whose leap is sized and solved on the legs
/// it stands on (#456, #459): a bat, a glider and the crosses either leads on
/// four, and a bird on two.
///
/// **A bird too**: its legs reach about as far as they stand, but read off
/// their reach a Bird x Glider cross landed a fall 0.12 m deep in a sixth of
/// a second, and its ears, springs stepped after the pose, flicked 2.4 rad
/// in a frame at 30 frames a second; landing on its standing leg it sinks
/// 0.05 m and they follow.
pub(super) fn on_wings(rig: &Rig) -> bool {
    rig.wings.is_some()
}

/// How far `limb` stands its pad from its hip in the rest pose, in metres:
/// the leg a winged body is sized on (#456).
fn standing_leg(rig: &Rig, limb: Limb) -> Option<f32> {
    let chain = rig.limb_chain(limb)?;
    let pad = *rig.in_zone(Zone::Extremity(limb)).first()?;
    Some(
        rig.joints[chain[0]]
            .position
            .distance(rig.joints[pad].position),
    )
}

/// The deepest a leg is asked to compress, as a share of [`leg_of`]:
/// [`MAX_SQUASH`], or [`WINGED_SQUASH`] on a winged body.
fn squash_most(rig: &Rig) -> f32 {
    if on_wings(rig) {
        WINGED_SQUASH
    } else {
        MAX_SQUASH
    }
}

/// How far a leaping beast's trunk turns as it gathers and lands, in
/// radians: [`LEAP_PITCH`], and on a winged body on four no further than
/// sinks the end that goes down [`WINGED_SINK`] of its leg (#459).
///
/// **A winged body's trunk is long over short legs**: turned the quarter
/// radian a dog's is about its shoulders, a glider's hips sank 70 mm on a
/// leg standing 116 mm, and its hind legs, held off the plane it took off
/// from, folded shut and turned over in a frame.
fn leap_pitch(rig: &Rig) -> f32 {
    if !on_wings(rig) {
        return LEAP_PITCH;
    }
    let tops = |fore: bool| {
        let (sum, count) = rig
            .ground_contacts()
            .into_iter()
            .filter(|limb| limb.is_fore() == fore)
            .filter_map(|limb| rig.limb_chain(limb))
            .fold((Vec3::ZERO, 0.0f32), |(sum, count), [top, _, _]| {
                (sum + rig.joints[top].position, count + 1.0)
            });
        (count > 0.0).then(|| sum / count)
    };
    let (Some(fore), Some(hind)) = (tops(true), tops(false)) else {
        return LEAP_PITCH;
    };
    let span = fore.distance(hind);
    if span <= f32::EPSILON {
        return LEAP_PITCH;
    }
    (WINGED_SINK * leg_of(rig) / span)
        .min(1.0)
        .det_asin()
        .min(LEAP_PITCH)
}

/// How far a winged body on four sinks the end its leap's pitch turns down,
/// as a share of its leg ([`leap_pitch`], #459).
/// Provenance: **measured** (#459): over a level leap at 3 m/s, a glider, a
/// Glider x Bird cross at 0.3 and a Bat x Glider cross at 0.5 turned a hind
/// hip half a turn in a frame at every pitch from the whole quarter radian
/// to a quarter of it while their legs were still tucked from the rest pose
/// and planted after; once tucked in one solve ([`Leap::tuck`]), a quarter
/// of the leg leaves no joint of any of them turning faster than its
/// landing does.
const WINGED_SINK: f32 = 0.25;

/// The deepest a winged body is asked to compress its leg, as a share of
/// [`leg_of`] (#456, #459).
/// Provenance: **measured** (#459): such a leg stands folded already - a
/// glider's to half its reach, a Bat x Glider cross's foreleg to a third -
/// and any landing faster than about 1.5 m/s asks for more than the whole
/// [`MAX_SQUASH`]. At 0.45 and at 0.3 of it, landing on a 20 degree slope
/// it faced up folded a glider's hind leg until its knee stood over its
/// hip and turned half a turn a frame (3.0 rad a sixtieth at 60 frames a
/// second at 0.45, 2.3 at 0.3); at 0.2, its feet set down square to the
/// slope ([`stand_square`]), no body landing on one turns a joint faster
/// than 0.39 rad a sixtieth (a Bat x Bird cross at 0.3, its hind hip, at
/// 144 frames a second).
const WINGED_SQUASH: f32 = 0.2;

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
    fn a_leaping_body_hangs_its_arms_rather_than_flying_in_the_bind_pose() {
        // **#267's leap half.** A leap says nothing about the arms, and
        // saying nothing used to mean the bind pose: a body jumping with its
        // arms splayed 50 degrees reads as a thrown mannequin. The idle's own
        // guard proves the carriage itself; this proves the LEAP calls it,
        // which is a separate line that can be separately lost.
        let rig = body(1.8);
        let leap = Leap::new(3.0);
        let mut pose = Pose::rest(&rig);
        // Mid-flight, where nothing else touches the arms.
        let elapsed = leap.wind_up(&rig) + leap.flight() * 0.5;
        leap.drive(&rig, &mut pose, elapsed, |at| {
            Some(Ground {
                position: Vec3::new(at.x, 0.0, at.z),
                normal: Vec3::Y,
            })
        });
        let posed = pose.forward(&rig).positions;
        let splay = [crate::plan::Limb::ForeLeft, crate::plan::Limb::ForeRight]
            .into_iter()
            .filter_map(|limb| rig.limb_chain(limb))
            .map(|[shoulder, elbow, _]| {
                (posed[elbow] - posed[shoulder])
                    .normalize_or(Vec3::NEG_Y)
                    .dot(Vec3::NEG_Y)
                    .clamp(-1.0, 1.0)
                    .det_acos()
            })
            .fold(0.0f32, f32::max);
        assert!(
            splay.to_degrees() < 25.0,
            "a body at mid-flight carried its arm {:.0} deg off vertical — the bind \
             pose's splay is 50 and a hung arm is under 20",
            splay.to_degrees(),
        );
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
