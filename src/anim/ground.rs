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
use super::pose_clip::PoseClip;
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

/// How near the lowest point of a body a foot must be to count as standing on
/// it, in metres.
///
/// A hand's breadth. Loose enough that a walk's trailing foot still counts while
/// its heel is peeling, tight enough that a foot in flight does not: measured
/// over the shipped clip set, `Jog` alternates one contact and two, and `Sprint`
/// spends most of its cycle on one.
pub const CONTACT_SLACK: f32 = 0.1;

/// Which of a body's feet are on the ground **in this pose**.
///
/// **Not the same question as [`Rig::ground_contacts`].** That one asks which
/// limbs a body of this shape stands on — a fact about the rig, true of a biped
/// whatever it is doing. This asks which of them are down *now*, which is a
/// fact about the pose. Handing every contact to [`plant_feet_of`] drags a foot
/// that is in the air down onto the floor, which is the failure a run makes
/// obvious and a walk hides.
///
/// A foot counts if any of its extremity joints is within [`CONTACT_SLACK`] of
/// the lowest joint on the body. Measured against the body itself rather than
/// against a ground plane, because the caller may not have one yet — this is
/// what it passes to [`plant_feet_of`] to *find* the ground.
///
/// **What it does NOT do, recorded because the first version of this claimed
/// it.** It does not detect that a body is lying down or sitting. Measured on
/// the shipped artifact, `Sleeping` reports both feet and `Sitting_Idle` reports
/// both — correctly, because a body on its back has its heels on the floor
/// beside its back, and a body on a chair has its feet on the floor. The rule is
/// about height and nothing else, and a foot near the bottom of a lying body
/// really is near the ground.
///
/// A gait knows better and should say so: [`Steps::stance`] names the feet that
/// are carrying the body this instant, and passing those is strictly more
/// accurate than inferring them here. This is for motion that arrives without
/// that answer attached, which is every clip.
///
/// [`Steps::stance`]: super::gait::Steps::stance
#[must_use]
pub fn contacts_in(rig: &Rig, pose: &Pose) -> Vec<Limb> {
    if !pose.fits(rig) {
        return Vec::new();
    }
    let posed = pose.forward(rig);
    let floor = posed
        .positions
        .iter()
        .fold(f32::MAX, |low, at| low.min(at.y));
    rig.ground_contacts()
        .into_iter()
        .filter(|&limb| {
            rig.extremity_joints(limb)
                .iter()
                .any(|&joint| posed.positions[joint].y - floor < CONTACT_SLACK)
        })
        .collect()
}

/// How fast a foot may be moving, as a share of the fastest foot's speed, and
/// still count as planted.
///
/// Measured on the reference's own `Walk` at #139: the slowest frame-to-frame
/// step of a toe is 2.7 mm against a quickest of 97, a separation of thirty-six
/// times. A third leaves room for a foot that is peeling or settling without
/// letting a swinging one through.
pub const CONTACT_SPEED: f32 = 0.35;

/// Which of a body's feet are planted at one moment of a **clip**.
///
/// [`contacts_in`] asks only how low a foot is, and on a walk that is not enough:
/// a walking foot lifts about 150 mm at its highest, so for much of its swing it
/// is within [`CONTACT_SLACK`] of the floor and gets planted — which drags it
/// down and ruins the very motion the solve was meant to settle. This adds the
/// question that actually separates them: **a planted foot is not moving.**
///
/// **Speed is measured with the clip's root motion in, and that is not optional.**
/// A foot planted on the ground is stationary in the world while the body travels
/// over it. Play the same clip in place — root translation zeroed, which is what
/// a viewer does to compare it against a gait that stays put — and that same
/// foot slides backwards at exactly walking pace, so nothing about the in-place
/// pose can tell a plant from a skate. The answer has to be taken from the
/// travelling version and applied to whichever one is being drawn.
///
/// A foot must pass both tests: near the floor, and slower than
/// [`CONTACT_SPEED`] of the fastest foot. Sampled one frame of the clip's own
/// rate either side, so it is a property of the clip rather than of how fast
/// anything is playing it — a scrubbed clip gives the same answer as a running
/// one.
#[must_use]
pub fn contacts_during(rig: &Rig, clip: &PoseClip, time: f32) -> Vec<Limb> {
    let near = contacts_in(rig, &clip.pose(rig, time));
    if near.len() < 2 {
        return near;
    }
    let step = if clip.rate > 0.0 {
        1.0 / clip.rate
    } else {
        return near;
    };

    // With root motion, deliberately — see above. `PoseClip::pose` carries it.
    let at = |when: f32| {
        let when = if clip.looping {
            when.rem_euclid(clip.duration().max(f32::EPSILON))
        } else {
            when.clamp(0.0, clip.duration())
        };
        let pose = clip.pose(rig, when);
        let travel = pose.translation;
        // `Posed::positions` are relative to the body's own root, so the clip's
        // travel is added back: the question is where a foot is in the world the
        // body is moving through.
        pose.forward(rig)
            .positions
            .iter()
            .map(|at| *at + travel)
            .collect::<Vec<_>>()
    };
    let before = at(time - step);
    let after = at(time + step);

    let speed = |limb: Limb| -> f32 {
        rig.extremity_joints(limb)
            .iter()
            .map(|&joint| before[joint].distance(after[joint]))
            .fold(0.0f32, f32::max)
    };
    let speeds: Vec<(Limb, f32)> = near.iter().map(|&limb| (limb, speed(limb))).collect();
    let fastest = rig
        .ground_contacts()
        .into_iter()
        .map(speed)
        .fold(0.0f32, f32::max);

    speeds
        .into_iter()
        .filter(|(_, moving)| *moving <= fastest * CONTACT_SPEED)
        .map(|(limb, _)| limb)
        .collect()
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

    #[test]
    fn a_lifted_foot_is_not_a_contact_and_the_other_one_still_is() {
        // **What the pose-level question buys over the rig-level one.**
        // `Rig::ground_contacts` answers about the SHAPE and says both feet
        // whatever the body is doing, so handing it to `plant_feet_of` drags a
        // foot that is in the air down onto the floor.
        let rig = biped();
        let standing = Pose::rest(&rig);
        assert_eq!(
            contacts_in(&rig, &standing),
            rig.ground_contacts(),
            "a body at rest is standing on every foot it has"
        );

        // One leg swung well clear at the hip. Which limb is irrelevant — what
        // matters is that the answer is per foot rather than all or nothing.
        let lifted = Limb::HindLeft;
        let hip = rig
            .in_zone(Zone::UpperLimb(lifted))
            .first()
            .copied()
            .expect("a leg");
        let mut raised = Pose::rest(&rig);
        raised.rotations[hip] = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);

        let down = contacts_in(&rig, &raised);
        assert!(
            !down.contains(&lifted),
            "a foot swung up to hip height was still called a contact"
        );
        assert!(
            down.contains(&lifted.mirrored()),
            "the foot still on the floor stopped being a contact"
        );
    }
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
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
        let beast =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
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

        // **Measured against the surface PLUS the foot's own stand-off**, the
        // same correction the flat-ground test above spells out: the contact
        // joint sits inside the foot rather than on its sole, so a joint exactly
        // on the surface would be a foot buried to its ankle. This used to
        // compare the joint against the bare slope and absorb the difference in
        // its tolerance, which held only while the foot happened to be thin
        // enough — #220 moved the sole onto the plan's ground plane, the
        // stand-off moved with it, and a tolerance standing in for a term
        // failed the moment the term changed.
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let landed = foot_of(&rig, &pose, limb);
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            let expected = landed.x * 0.25 + stand_off(&rig, foot);
            assert!(
                (landed.y - expected).abs() < 0.02,
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
        let rig =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
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
