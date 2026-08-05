//! Putting feet on the ground.
//!
//! A walk cycle authored in the air lands wrong on everything else: on a slope
//! the downhill foot floats and the uphill one sinks, on a step both do. Foot
//! placement fixes that after the fact, and it is the single change that does
//! most to make a body look like it is *in* a place rather than played back near
//! one.
//!
//! The recipe is standard and worth stating because the order matters. Probe the
//! ground beneath each planted foot; drop the pelvis by the largest downward
//! correction, so the leg that has furthest to reach can still reach without
//! straightening; then solve each leg to its own ground contact. Skipping the
//! pelvis drop is the common mistake — legs hyperextend and the body appears to
//! hover on tiptoe.
//!
//! This crate never traces a ray itself. The caller passes a closure that
//! answers "what is beneath this point?", so the same code works against a
//! physics engine, a heightmap, or a flat plane in a test.
//!
//! Feet are placed **and now oriented**, which waited on the feet having any
//! orientation to correct. A body plan's foot used to be one node; since #111 it
//! is a heel, a ball and a toe lying across a sole, and [`level_feet`] holds
//! that sole against the ground instead of letting it ride the shin.
//!
//! What that was costing, measured over a walk cycle by `examples/walkaudit`:
//! **a planted foot sank 121 mm through the floor.** The leg IK aims the joint
//! the foot hangs from, and everything below it keeps whatever orientation the
//! rest pose left it with — so as the body travels over a planted foot, the foot
//! turns with the shin and drives its toe into the ground. The sole started a
//! stance 33 mm under and finished it 121 mm under. A swinging foot was no
//! better: at its lowest it was 101 mm below the floor it was supposed to be
//! swinging over.

use glam::{Quat, Vec3};

use super::ik::two_bone;
use super::pose::Pose;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// What lies beneath a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    /// Where the surface is.
    pub position: Vec3,
    /// Which way it faces. Normalised.
    pub normal: Vec3,
}

impl Ground {
    /// A patch of level ground at height `y`.
    #[must_use]
    pub fn level(position: Vec3) -> Self {
        Self {
            position,
            normal: Vec3::Y,
        }
    }
}

/// Tuning for [`plant_feet`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootingConfig {
    /// Furthest the pelvis may drop, in metres.
    ///
    /// Bounds how much of a fall or a hole the body will try to absorb by
    /// crouching before it should be doing something else entirely.
    pub max_pelvis_drop: f32,
    /// Furthest a foot may be lifted onto ground above it, in metres.
    pub max_step_up: f32,
    /// Furthest the ankle may be turned to keep a sole flat, in radians.
    ///
    /// Forty degrees, which is about what an ankle has. The clamp is what keeps
    /// a leg that is reaching hard from answering with a foot folded under
    /// itself: on ground the body cannot properly reach, a *visibly* strained
    /// ankle is the honest failure and a broken one is not.
    pub max_ankle: f32,
    /// How many times to probe and re-solve.
    ///
    /// Solving a leg moves its foot, which changes what is beneath it — so on
    /// anything but level ground one pass leaves the foot near the surface
    /// rather than on it. A second pass closes almost all of the remainder.
    pub passes: usize,
}

impl Default for FootingConfig {
    fn default() -> Self {
        Self {
            max_pelvis_drop: 0.35,
            max_step_up: 0.4,
            max_ankle: 0.70,
            passes: 2,
        }
    }
}

/// What foot placement did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Footing {
    /// Limbs that found ground and were solved onto it.
    pub planted: Vec<Limb>,
    /// Limbs whose ground was out of reach even after the pelvis dropped.
    pub straining: Vec<Limb>,
    /// How far the pelvis was lowered, in metres.
    pub pelvis_drop: f32,
}

impl Footing {
    /// Whether every contact found ground it could reach.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        self.straining.is_empty() && !self.planted.is_empty()
    }
}

/// Solves a body's ground contacts onto the surface beneath them.
///
/// `beneath` is asked, for each foot's current world position, what surface lies
/// under it — returning `None` where there is nothing, which leaves that leg
/// as the pose had it.
pub fn plant_feet<F>(rig: &Rig, pose: &mut Pose, beneath: F, config: &FootingConfig) -> Footing
where
    F: Fn(Vec3) -> Option<Ground>,
{
    plant_feet_of(rig, pose, &rig.ground_contacts(), beneath, config)
}

/// Solves only the named contacts onto the surface beneath them.
///
/// What a gait needs: its stance feet are carrying the body and belong on the
/// ground, while its swinging feet are travelling over that ground and must be
/// left alone. Planting everything would drag each swinging foot down and reduce
/// the walk to a shuffle.
pub fn plant_feet_of<F>(
    rig: &Rig,
    pose: &mut Pose,
    limbs: &[Limb],
    beneath: F,
    config: &FootingConfig,
) -> Footing
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if !pose.fits(rig) {
        return Footing::default();
    }

    let contacts: Vec<Limb> = limbs.to_vec();
    let mut footing = Footing::default();

    for _ in 0..config.passes.max(1) {
        // **Level before solving, every pass, and the order is the whole of why
        // it converges.** Turning an ankle swings the contact joint hanging off
        // it, so levelling a foot that has just been planted un-plants it. The
        // solve, though, measures the contact-to-ankle offset from the pose in
        // front of it — see [`solve_contact`] — so a foot levelled first is a
        // foot the solve places correctly. What is left over is only the shin
        // rotation that same solve introduced, and the next pass takes most of
        // that out again. Measured on the default walk, the sole settles from
        // 19 mm under the floor to under 4 mm.
        //
        // Every contact, not only the planted ones: a swinging foot is pinned
        // by nothing, so levelling it is free, and it is the foot most likely
        // to be ploughing through the ground.
        level_feet(rig, pose, &beneath, config);
        let posed = pose.forward(rig);

        // Probe before moving anything: the corrections have to be known against
        // one consistent pose, or each leg would be measured against a body that
        // had already shifted under it.
        let mut probes: Vec<(Limb, usize, Vec3)> = Vec::new();
        for &limb in &contacts {
            let Some(&foot) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            if let Some(ground) = beneath(posed.positions[foot]) {
                // **Where the joint goes, not where the ground is.** A contact
                // joint is inside the foot, not on its sole: on a body whose
                // foot is a chain of its own the heel node sits 29 mm up. Aimed
                // at the surface itself, every planted foot spends the whole
                // stance that far under it — which is what the sole measured
                // before this, and no amount of levelling the ankle could fix a
                // target that was simply too low.
                probes.push((limb, foot, ground.position + Vec3::Y * stand_off(rig, foot)));
            }
        }
        if probes.is_empty() {
            return footing;
        }

        // The lowest correction sets how far the body has to sink for its most
        // stretched leg to reach; ground above a foot is met by bending instead.
        let deepest = probes
            .iter()
            .map(|(_, foot, target)| target.y - posed.positions[*foot].y)
            .fold(0.0f32, f32::min);
        let remaining = config.max_pelvis_drop - footing.pelvis_drop;
        let drop = deepest.clamp(-remaining.max(0.0), 0.0);
        pose.translation.y += drop;
        footing.pelvis_drop -= drop;

        footing.planted.clear();
        footing.straining.clear();

        for (limb, foot, target) in probes {
            if target.y - posed.positions[foot].y > config.max_step_up {
                footing.straining.push(limb);
                continue;
            }

            if solve_contact(rig, pose, limb, target) {
                footing.planted.push(limb);
            } else {
                footing.straining.push(limb);
            }
        }
    }

    footing
}

/// How far above the floor a contact joint rests when the body stands.
///
/// **Read off the rest pose, because the rest pose is a body standing up.** Every
/// plan in this crate builds its bodies on `y = 0` — it is what
/// [`crate::extremity::Extremities::build`] takes a ground plane for — so the
/// height a contact joint sits at when nothing has been posed is exactly the
/// height it should be held at when it is planted. Nothing has to be measured
/// off a mesh, and a plan that puts its feet somewhere else is right for free.
///
/// Floored at zero so a body built below its own floor cannot drive its feet
/// further down.
fn stand_off(rig: &Rig, foot: usize) -> f32 {
    rig.joints[foot].position.y.max(0.0)
}

/// Turns each foot so its sole lies along the ground rather than along the shin.
///
/// **A foot is not a fixed part of the shin, and a walk is where that shows.**
/// The leg solve aims the joint the foot hangs from and stops there; everything
/// past it inherits the shin's orientation, so a planted foot rotates as the
/// body passes over it and a swinging foot points wherever the knee left it
/// pointing. Measured on the default body before this existed, the sole reached
/// 121 mm below the floor during stance and 101 mm below it mid-swing.
///
/// `beneath` answers what lies under a foot, exactly as [`plant_feet`] asks it;
/// a foot over nothing is levelled against world up, which is the right answer
/// for a foot in the air and the only one available for a foot over a hole.
///
/// **The ankle's rotation is assigned, not composed.** It is a constraint on
/// where the foot ends up rather than a contribution to a gesture — the same
/// thing the contact solve does to the hip and knee, in the same pass, and for
/// the same reason. That also makes it idempotent: levelling an already-level
/// foot changes nothing, so a caller that runs it twice is not punished.
///
/// Call after the legs are placed. Running it before [`plant_feet`] would level
/// the feet against a pose the solve is about to change.
pub fn level_feet<F>(rig: &Rig, pose: &mut Pose, beneath: F, config: &FootingConfig)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let beneath = &beneath;
    if !pose.fits(rig) {
        return;
    }
    let posed = pose.forward(rig);

    for limb in rig.ground_contacts() {
        // The joint the foot hangs from — the ankle on a body whose foot is a
        // chain of its own, the last leg node on one whose foot is an attached
        // part. `extremity_joints` answers that without either being assumed.
        let joints = rig.extremity_joints(limb);
        let (Some(&ankle), Some(&foot)) = (joints.first(), joints.get(1)) else {
            continue;
        };
        let Some(parent) = rig.joints[ankle].parent else {
            continue;
        };

        // Level against the ground under the foot itself, not under the ankle:
        // on a slope those are a step apart, and the sole is what has to lie
        // flat.
        let up = beneath(posed.positions[foot]).map_or(Vec3::Y, |ground| ground.normal);
        let want = Quat::from_rotation_arc(Vec3::Y, up.normalize_or(Vec3::Y));

        // What the ankle must hold locally for the foot to end up there, and
        // then how far that is from leaving it alone, so it can be clamped.
        let local = posed.rotations[parent].inverse() * want;
        let (axis, angle) = local.to_axis_angle();
        let angle = angle.rem_euclid(std::f32::consts::TAU);
        // `to_axis_angle` reports the turn the short way round or the long way
        // depending on the sign of the scalar part; fold it into `-PI..=PI` so a
        // small correction is never mistaken for a nearly-full turn.
        let angle = if angle > std::f32::consts::PI {
            angle - std::f32::consts::TAU
        } else {
            angle
        };
        pose.rotations[ankle] =
            Quat::from_axis_angle(axis, angle.clamp(-config.max_ankle, config.max_ankle));
    }
}

/// Solves one limb so its ground contact lands on `target`.
///
/// The leg solves to the joint *above* the contact, because the foot hangs off
/// it — aiming the ankle itself at the ground would bury the foot in it. Shared
/// with the gait engine, which places contacts for a different reason but has
/// exactly the same problem.
pub(crate) fn solve_contact(rig: &Rig, pose: &mut Pose, limb: Limb, target: Vec3) -> bool {
    let Some(chain) = rig.limb_chain(limb) else {
        return false;
    };
    let Some(&foot) = rig.in_zone(Zone::Extremity(limb)).first() else {
        return false;
    };

    let posed = pose.forward(rig);
    let offset = posed.positions[chain[2]] - posed.positions[foot];
    // Which way the joint folds is the rig's to say, not this function's. It
    // used to be hardcoded forward here, which is right for a biped's knee and
    // a quadruped's stifle and backwards for everything else that can be
    // solved. See [`Rig::bend_pole`].
    let Some(pole) = rig.bend_pole(limb) else {
        return false;
    };

    two_bone(rig, pose, chain, target + offset, pole)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    /// Level ground at a fixed height.
    fn flat(height: f32) -> impl Fn(Vec3) -> Option<Ground> {
        move |point: Vec3| Some(Ground::level(Vec3::new(point.x, height, point.z)))
    }

    /// A slope rising toward `+x`.
    fn slope(grade: f32) -> impl Fn(Vec3) -> Option<Ground> {
        move |point: Vec3| {
            Some(Ground {
                position: Vec3::new(point.x, point.x * grade, point.z),
                normal: Vec3::new(-grade, 1.0, 0.0).normalize(),
            })
        }
    }

    /// The world position of one foot in the given pose.
    fn foot_of(rig: &Rig, pose: &Pose, limb: Limb) -> Vec3 {
        let joint = rig.in_zone(Zone::Extremity(limb))[0];
        pose.forward(rig).positions[joint]
    }

    #[test]
    fn a_biped_stands_on_two_feet_and_a_quadruped_on_four() {
        assert_eq!(biped().ground_contacts().len(), 2);
        let beast = Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs");
        assert_eq!(beast.ground_contacts().len(), 4);
    }

    #[test]
    fn feet_meet_ground_that_is_lower_than_they_are() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let start = foot_of(&rig, &pose, Limb::HindLeft).y;

        let footing = plant_feet(
            &rig,
            &mut pose,
            flat(start - 0.1),
            &FootingConfig::default(),
        );
        assert!(footing.is_settled(), "{footing:?}");
        assert!(footing.pelvis_drop > 0.0, "the pelvis should sink");

        // **The contact joint lands a foot's thickness above the ground, not on
        // it.** It is inside the foot rather than on its sole, and this used to
        // assert it went to the surface itself — which put the sole of every
        // planted foot that far under the floor for the whole stance. What has
        // to hold is that the joint keeps the height it stands at, which is what
        // `stand_off` reads off the rest pose.
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let landed = foot_of(&rig, &pose, limb).y;
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            let wanted = start - 0.1 + stand_off(&rig, foot);
            assert!(
                (landed - wanted).abs() < 0.02,
                "{limb:?} landed at {landed}, wanted {wanted}"
            );
        }
    }

    #[test]
    fn a_slope_is_met_by_each_foot_separately() {
        // The defect this guards against: both feet placed at one height, which
        // leaves the downhill one floating and the uphill one buried.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let footing = plant_feet(&rig, &mut pose, slope(0.25), &FootingConfig::default());
        assert!(footing.is_settled(), "{footing:?}");

        for limb in [Limb::HindLeft, Limb::HindRight] {
            let landed = foot_of(&rig, &pose, limb);
            let expected = landed.x * 0.25;
            assert!(
                (landed.y - expected).abs() < 0.03,
                "{limb:?} sits {:.3} off the slope",
                landed.y - expected
            );
        }

        let left = foot_of(&rig, &pose, Limb::HindLeft).y;
        let right = foot_of(&rig, &pose, Limb::HindRight).y;
        assert!(
            (left - right).abs() > 0.02,
            "the two feet should end at different heights on a slope"
        );
    }

    #[test]
    fn the_pelvis_drop_is_bounded() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let config = FootingConfig {
            max_pelvis_drop: 0.1,
            ..Default::default()
        };
        let footing = plant_feet(&rig, &mut pose, flat(-5.0), &config);
        assert!(
            footing.pelvis_drop <= 0.1 + 1e-5,
            "dropped {} past its limit",
            footing.pelvis_drop
        );
        assert!(
            !footing.straining.is_empty(),
            "an unreachable floor strains"
        );
    }

    #[test]
    fn ground_far_above_a_foot_is_refused_rather_than_climbed() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let config = FootingConfig {
            max_step_up: 0.05,
            ..Default::default()
        };
        let footing = plant_feet(&rig, &mut pose, flat(1.0), &config);
        assert_eq!(footing.planted.len(), 0);
        assert_eq!(footing.straining.len(), 2);
    }

    #[test]
    fn ground_that_is_not_there_leaves_the_pose_alone() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let before = pose.clone();
        let footing = plant_feet(&rig, &mut pose, |_| None, &FootingConfig::default());
        assert_eq!(footing, Footing::default());
        assert_eq!(pose, before);
    }

    /// The world orientation of the foot hanging off `limb`'s contact.
    fn foot_tilt(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let joints = rig.extremity_joints(limb);
        let posed = pose.forward(rig);
        // Where the foot's own up axis has ended up, against the world's.
        let up = posed.rotations[joints[0]] * Vec3::Y;
        up.dot(Vec3::Y).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_planted_foot_lies_flat_however_the_leg_leans() {
        // The defect in one sentence: the leg IK aims the joint the foot hangs
        // from and stops, so the foot rides the shin. Lean the shin and the foot
        // tips with it — which on a walk drove the sole 121 mm through the floor
        // by the end of a stance (#114).
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        // Shove the body well forward of its feet, which is what the second half
        // of a stance is: the pelvis has travelled past the planted foot.
        pose.translation.z += 0.2;
        plant_feet(&rig, &mut pose, flat(0.0), &FootingConfig::default());

        for limb in [Limb::HindLeft, Limb::HindRight] {
            let tilt = foot_tilt(&rig, &pose, limb);
            assert!(
                tilt < 5.0,
                "{limb:?} sat {tilt:.1} degrees off level with the body over it"
            );
        }
    }

    #[test]
    fn levelling_a_foot_does_not_unplant_it() {
        // The two constraints fight: turning an ankle swings the contact joint
        // hanging off it. Levelling after planting therefore undoes the plant,
        // which is why the levelling happens *inside* the solve loop and before
        // each probe. What must hold is that both are true at the end.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        pose.translation.z += 0.15;
        let footing = plant_feet(&rig, &mut pose, flat(-0.05), &FootingConfig::default());
        assert!(footing.is_settled(), "{footing:?}");

        for limb in [Limb::HindLeft, Limb::HindRight] {
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            let landed = foot_of(&rig, &pose, limb).y;
            let wanted = -0.05 + stand_off(&rig, foot);
            assert!(
                (landed - wanted).abs() < 0.01,
                "{limb:?} levelled itself off its own footing: {landed} against {wanted}"
            );
            assert!(foot_tilt(&rig, &pose, limb) < 5.0, "{limb:?} is not flat");
        }
    }

    #[test]
    fn a_foot_on_a_slope_lies_along_it() {
        // A level foot is not the goal — a foot on the ground is. On a ramp the
        // sole should follow the ramp, which is the whole reason `level_feet`
        // takes the surface normal rather than assuming world up.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let grade: f32 = 0.25;
        let normal = Vec3::new(0.0, 1.0, -grade).normalize();
        let ramp = |at: Vec3| {
            Some(Ground {
                position: Vec3::new(at.x, at.z * grade, at.z),
                normal,
            })
        };
        plant_feet(&rig, &mut pose, ramp, &FootingConfig::default());

        let want = normal.dot(Vec3::Y).acos().to_degrees();
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let tilt = foot_tilt(&rig, &pose, limb);
            assert!(
                (tilt - want).abs() < 5.0,
                "{limb:?} tilted {tilt:.1} degrees on a slope of {want:.1}"
            );
        }
    }

    #[test]
    fn the_ankle_will_not_fold_further_than_an_ankle_folds() {
        // On ground the body cannot properly reach, a visibly strained ankle is
        // the honest failure; one folded through itself is not.
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let config = FootingConfig::default();
        // A surface standing on end, so levelling against it asks for a right
        // angle the clamp has to refuse.
        let wall = |at: Vec3| {
            Some(Ground {
                position: Vec3::new(at.x, 0.0, at.z),
                normal: Vec3::Z,
            })
        };
        level_feet(&rig, &mut pose, wall, &config);

        for limb in [Limb::HindLeft, Limb::HindRight] {
            let tilt = foot_tilt(&rig, &pose, limb);
            assert!(
                tilt <= config.max_ankle.to_degrees() + 1.0,
                "{limb:?} turned {tilt:.1} degrees against a clamp of {:.1}",
                config.max_ankle.to_degrees()
            );
        }
    }

    #[test]
    fn a_quadruped_plants_all_four() {
        let rig = Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs");
        let mut pose = Pose::rest(&rig);
        let start = foot_of(&rig, &pose, Limb::HindLeft).y;

        let footing = plant_feet(
            &rig,
            &mut pose,
            flat(start - 0.05),
            &FootingConfig::default(),
        );
        assert_eq!(footing.planted.len(), 4, "{footing:?}");
        for limb in Limb::ALL {
            let landed = foot_of(&rig, &pose, limb).y;
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            let wanted = start - 0.05 + stand_off(&rig, foot);
            assert!(
                (landed - wanted).abs() < 0.03,
                "{limb:?} landed at {landed}, wanted {wanted}"
            );
        }
    }
}
