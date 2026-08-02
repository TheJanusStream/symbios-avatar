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
//! Feet are placed but not *oriented*. Rolling a foot onto a slope needs a foot
//! with a sole — a heel and a toe to lie across it — and a body plan's foot is
//! currently one node, which has no orientation of its own to correct. Measuring
//! it bore that out: every formulation tried moved the foot further from the
//! angle it holds at rest rather than closer, because the leg leaning to reach
//! the ground already does most of the work. It waits for feet with real
//! geometry rather than shipping a knob that cannot be justified.

use glam::Vec3;

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
        let posed = pose.forward(rig);

        // Probe before moving anything: the corrections have to be known against
        // one consistent pose, or each leg would be measured against a body that
        // had already shifted under it.
        let mut probes: Vec<(Limb, usize, Ground)> = Vec::new();
        for &limb in &contacts {
            let Some(&foot) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            if let Some(ground) = beneath(posed.positions[foot]) {
                probes.push((limb, foot, ground));
            }
        }
        if probes.is_empty() {
            return footing;
        }

        // The lowest correction sets how far the body has to sink for its most
        // stretched leg to reach; ground above a foot is met by bending instead.
        let deepest = probes
            .iter()
            .map(|(_, foot, ground)| ground.position.y - posed.positions[*foot].y)
            .fold(0.0f32, f32::min);
        let remaining = config.max_pelvis_drop - footing.pelvis_drop;
        let drop = deepest.clamp(-remaining.max(0.0), 0.0);
        pose.translation.y += drop;
        footing.pelvis_drop -= drop;

        footing.planted.clear();
        footing.straining.clear();

        for (limb, foot, ground) in probes {
            if ground.position.y - posed.positions[foot].y > config.max_step_up {
                footing.straining.push(limb);
                continue;
            }

            if solve_contact(rig, pose, limb, ground.position) {
                footing.planted.push(limb);
            } else {
                footing.straining.push(limb);
            }
        }
    }

    footing
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

        for limb in [Limb::HindLeft, Limb::HindRight] {
            let landed = foot_of(&rig, &pose, limb).y;
            assert!(
                (landed - (start - 0.1)).abs() < 0.02,
                "{limb:?} landed at {landed}, wanted {}",
                start - 0.1
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
            assert!(
                (landed - (start - 0.05)).abs() < 0.03,
                "{limb:?} landed at {landed}"
            );
        }
    }
}
