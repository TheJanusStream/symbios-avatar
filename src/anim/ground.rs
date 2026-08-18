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
//! Feet are placed **and oriented**. A body plan's foot is not one node but a
//! heel, a ball and a toe lying across a sole, and [`level_feet`] holds
//! that sole against the ground instead of letting it ride the shin.
//!
//! What riding the shin costs, measured over a walk cycle by
//! `examples/walkaudit`: **a planted foot sinks 121 mm through the floor.**
//! The leg IK aims the joint the foot hangs from, and everything below it
//! keeps whatever orientation the rest pose left it with — so as the body
//! travels over a planted foot, the foot turns with the shin and drives its
//! toe into the ground. The sole starts a stance 33 mm under and finishes it
//! 121 mm under. A swinging foot is no better: at its lowest it is 101 mm
//! below the floor it is supposed to be swinging over.

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
/// Measured on the reference library's own `Walk`: the slowest frame-to-frame
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

    for limb in rig.ground_contacts() {
        // Re-read each limb, because levelling the last one moved the leg it
        // hangs from. One sweep for all of them was right while this only
        // assigned a rotation; it stopped being right when it started putting
        // the contact back (#257).
        let posed = pose.forward(rig);
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
        let Some(&contact) = rig.in_zone(Zone::Extremity(limb)).first() else {
            continue;
        };
        // Where the contact stands before the ankle turns. Turning an ankle
        // swings everything hanging off it, and this is the thing that must not
        // move.
        let held = posed.positions[contact];

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

        // **And then put the contact back** (#257). Levelling used to be
        // described here as free, on the grounds that a swinging foot is pinned
        // by nothing. It is not free: turning the ankle swings the contact
        // hanging off it — measured at 41.7 mm on a walk and 54.3 on a run —
        // and for a foot nothing is about to solve, nothing took it out again.
        //
        // It hid behind a second defect for as long as both existed. The
        // one-pass solve aimed the ANKLE at the goal plus the rest hang, which
        // is roughly straight up from where the contact belongs, so a foot
        // levelled afterwards landed its sole at zero by accident. Fixing the
        // solve (#254) removed the accident and left the drag showing at 11 mm.
        // The two had to land together.
        solve_contact(rig, pose, limb, held);
    }
}

/// Solves one limb so its ground contact lands on `target`.
///
/// The leg solves to the joint *above* the contact, because the foot hangs off
/// it — aiming the ankle itself at the ground would bury the foot in it. Shared
/// with the gait engine, which places contacts for a different reason but has
/// exactly the same problem.
///
/// **Iterated, because the hang turns as the limb solves.** The offset
/// below is read off the pose, and the solve it feeds then rotates the joint it
/// was read from — so one pass lands the extremity where it *was* hanging
/// rather than on `target`, and `two_bone` reports success when it does.
///
/// **What one pass costs, measured on the default biped.** A hand lands 69.7
/// mm from its goal on a rest pose. A stance foot sits 36 mm above a flat
/// floor at pace 0.5 and 67 mm at pace 1.5 — pace-dependent, because the miss
/// is a fraction of how far the limb turned, and pace-dependent foot placement
/// is what a skate looks like. Worst of all, the **body travels 12.3% further
/// than its stride says**: a planted contact slides back under the hip by
/// `d(1 + hang/reach)` when the goal moves by `d`, and 1 + 0.09/0.71 is 1.127
/// against 1.123 measured. Every walk produced that way is 12% out against the
/// floor it is walking on.
///
/// Re-reading and re-solving is the same fixed point [`FootingConfig::passes`],
/// [`level_feet`] and the gait's own trunk angle each iterate, and for the same
/// reason: the answer moves what the question was asked about.
pub(crate) fn solve_contact(rig: &Rig, pose: &mut Pose, limb: Limb, target: Vec3) -> bool {
    // Which way the joint folds is the rig's to say, not this function's. It
    // used to be hardcoded forward here, which is right for a biped's knee and
    // a quadruped's stifle and backwards for everything else that can be
    // solved. See [`Rig::bend_pole`].
    let Some(pole) = rig.bend_pole(limb) else {
        return false;
    };
    solve_contact_toward(rig, pose, limb, target, pole)
}

/// As [`solve_contact`], but with the fold direction given rather than read off
/// the rest pose.
///
/// [`Rig::bend_pole`] answers in the rig's **rest** space — it is a point thrown
/// a body's length out from where the chain's root sits on an unposed body. That
/// is the right answer for a goal which is itself in rest space, which every
/// ground contact is: the floor does not move when the pelvis does. It is the
/// wrong answer for a goal carried along by the body, because the pole would
/// then stay behind while the goal travelled, and the limb would fold on a plane
/// that drifts with the crouch. A caller that moves the goal moves the pole with
/// it, through the same transform.
pub(crate) fn solve_contact_toward(
    rig: &Rig,
    pose: &mut Pose,
    limb: Limb,
    target: Vec3,
    pole: Vec3,
) -> bool {
    let Some(chain) = rig.limb_chain(limb) else {
        return false;
    };
    let Some(&foot) = rig.in_zone(Zone::Extremity(limb)).first() else {
        return false;
    };

    // **A limb nothing has posed, asked for exactly where it already is, is
    // left alone** (#262). Solving is not free: `two_bone` reaches the goal and
    // then settles the bend on the POLE's plane, which is not quite the plane a
    // rest limb sits in, so the mid joint moves a little and the extremity
    // less. Every gesture in [`super::gesture`] opens and closes on a zero key
    // and paid 1.6 mm of it at both ends, which is what held their return guard
    // at five millimetres against a real drift it could not then have caught.
    //
    // **Both halves are needed, and the second one is what the gait's foot roll
    // costs.** Gating on arrival alone regressed
    // `the_part_of_the_sole_bearing_weight_does_not_move_as_the_foot_rolls`
    // from 2.2 mm of slide to 4.6: [`level_feet`] alternates pinning the ankle
    // at the attitude a roll wants against solving the leg under it, and
    // neither constraint is exact until the two agree — so a pass that arrives
    // is still a pass that has to run. What tells the two apart is not the
    // distance but whether anything has posed the limb at all: the rest
    // configuration is itself a valid answer to a rest goal, and the one the
    // body's author chose.
    let untouched = chain
        .iter()
        .all(|&joint| pose.rotations[joint].angle_between(Quat::IDENTITY) <= REST_SLACK);
    if untouched && pose.forward(rig).positions[foot].distance(target) <= CONTACT_TOLERANCE {
        return true;
    }

    let mut reached = false;
    for _ in 0..CONTACT_PASSES {
        let posed = pose.forward(rig);
        // The convergence check reads the pose the *previous* pass left, which
        // costs at most one extra pass and saves a whole forward-kinematics
        // sweep every other one. A limb that cannot reach its goal never meets
        // the tolerance and spends the full budget, which is the right way for
        // a straining limb to be bounded rather than to spin.
        //
        // Guarded on `reached` rather than on the pass index so a limb that
        // something HAS posed always gets one solve: callers reach for this to
        // make a leg answer for an ankle they have just turned, and a foot
        // already standing where it is being sent still needs the leg moved
        // under it. The limb nothing has posed is the one exception, and it
        // returned above.
        if reached && posed.positions[foot].distance(target) <= CONTACT_TOLERANCE {
            break;
        }
        let offset = posed.positions[chain[2]] - posed.positions[foot];
        reached = two_bone(rig, pose, chain, target + offset, pole);
    }

    // **The verdict gets its own tolerance, and a looser one.** `reached` means
    // "this limb strained" to every caller, and neither of the obvious readings
    // says that. `two_bone`'s own answer is about the joint IT was given, and
    // the goal handed to it moves every pass as the hang is re-read, so near
    // the edge of a limb's reach its verdict straddles the limit and a caller
    // sees whichever pass happened to be last — that is how a gait came to
    // report a strained leg walking on flat ground. Arrival within
    // [`CONTACT_TOLERANCE`] is too strict at the same edge: a rest-pose leg
    // stands at exactly full extension, which is where the solver deliberately
    // holds back from the singularity, so a goal that IS within reach — by two
    // tenths of a millimetre, measured — never closes the last of the distance.
    //
    // A limb missing by [`CONTACT_STRAIN`] is not straining; one missing by
    // centimetres is, and that is the case worth reporting.
    reached || pose.forward(rig).positions[foot].distance(target) <= CONTACT_STRAIN
}

/// How far a joint may be turned and still count as unposed, in radians.
///
/// **A milliradian, which is not a tolerance so much as a guard against
/// arithmetic.** The question [`solve_contact_toward`] asks with it is whether
/// anything has posed this limb at all, and the honest answer is exact
/// identity; this is only enough slack that a caller who composed a turn and
/// its inverse still reads as having done nothing. A thousandth of a radian
/// moves the reference body's ankle 0.7 mm — below the tolerance the solve
/// aims for — so nothing a pose actually means can hide under it.
const REST_SLACK: f32 = 1e-3;

/// How far an extremity may land from its goal before the limb counts as
/// straining, in metres.
///
/// **Five millimetres, an order of magnitude above [`CONTACT_TOLERANCE`].** The
/// tighter number is what the iteration aims for; this is what a caller is told
/// about. Two are needed because a limb at full extension cannot meet the first
/// even when its goal is inside its reach — see [`solve_contact_toward`] — and
/// a report that cries strain there is a report nobody can act on.
pub(crate) const CONTACT_STRAIN: f32 = 5e-3;

/// Most times [`solve_contact_toward`] will re-aim before giving up.
///
/// **Six, and in the ordinary case [`CONTACT_TOLERANCE`] stops it sooner.** The
/// correction shrinks by about a factor of four a pass — 69.7 mm, 10.8, 2.9,
/// 0.76, and so on — so the cap only binds on a goal the limb cannot reach,
/// where the extremity never arrives and no tolerance is ever met.
const CONTACT_PASSES: usize = 6;

/// How near its target an extremity has to land for the solve to stop, in
/// metres.
///
/// **Half a millimetre.** The residual after that is smaller than the line a
/// renderer would draw it with, and each further pass costs a whole
/// forward-kinematics sweep over the rig. Measured on the default biped over a
/// walk: this leaves a hand 0.1–0.3 mm from its goal and takes a walk frame
/// from 8.1 to 16.4 microseconds, against 24.0 for chasing it to a tenth of a
/// millimetre.
const CONTACT_TOLERANCE: f32 = 5e-4;

#[cfg(test)]
mod contact_tests {
    use super::*;
    use crate::anim::gait::{self, Gait, Stride};
    use crate::plan::{BodyPlan, HumanoidParams, Zone};
    use crate::rig::Rig;

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    #[test]
    fn a_contact_lands_where_it_was_aimed_rather_than_where_it_was_hanging() {
        // **#254.** `solve_contact` aims the joint the extremity hangs off, and
        // corrects for the hang with an offset read BEFORE the solve — which
        // the solve then turns. One pass left a hand 69.7 mm from its goal on a
        // rest pose while `two_bone` reported success, so nothing surfaced it.
        let rig = biped();
        let limb = Limb::ForeLeft;
        let reach = rig.limb_reach(limb).expect("reach");
        let hand = rig.in_zone(Zone::Extremity(limb))[0];
        let home = rig.joints[hand].position;

        for offset in [
            Vec3::new(0.0, 0.5, 0.35),
            Vec3::new(0.0, 0.3, 0.2),
            Vec3::new(0.05, 0.15, 0.1),
        ] {
            let mut pose = Pose::rest(&rig);
            let goal = home + offset * reach;
            let reported = solve_contact(&rig, &mut pose, limb, goal);
            let miss = pose.forward(&rig).positions[hand].distance(goal);
            assert!(
                reported,
                "{offset:?} was out of reach: missed by {miss:.4} m"
            );
            assert!(
                miss < 1e-3,
                "the hand landed {:.1} mm from the goal it was aimed at",
                miss * 1000.0
            );
        }
    }

    #[test]
    fn a_stance_foot_lands_the_same_distance_from_the_floor_at_every_pace() {
        // **#254 on the gait, which shares the solver.** The miss is the hang
        // the solve did not re-read, so it grew with how far the leg had to
        // turn: a stance contact sat 36 mm above a flat floor at pace 0.5 and
        // 67 mm at pace 1.5. What is left is the contact joint's own height
        // above the sole, which is a constant of the body and not a pace.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let ground = |point: Vec3| Some(Ground::level(Vec3::new(point.x, 0.0, point.z)));

        let worst_at = |pace: f32| {
            let stride = Stride::for_body(&rig, pace);
            (0..60).fold(0.0f32, |worst, sample| {
                let mut pose = Pose::rest(&rig);
                let steps = gait::step(
                    &rig,
                    &mut pose,
                    &gait,
                    &stride,
                    sample as f32 / 60.0,
                    ground,
                );
                let posed = pose.forward(&rig);
                steps.stance.iter().fold(worst, |worst, &limb| {
                    let foot = rig.in_zone(Zone::Extremity(limb))[0];
                    worst.max(posed.positions[foot].y.abs())
                })
            })
        };

        let (slow, fast) = (worst_at(0.5), worst_at(1.5));
        assert!(
            (fast - slow).abs() < 1e-3,
            "the miss still grows with pace: {:.1} mm at 0.5 against {:.1} mm at 1.5",
            slow * 1000.0,
            fast * 1000.0
        );
    }

    #[test]
    fn levelling_a_foot_leaves_its_contact_where_it_found_it() {
        // **#257, which had to land with #254 and did not exist on its own.**
        // Levelling was described as free on the grounds that a swinging foot
        // is pinned by nothing. Turning an ankle swings the contact hanging off
        // it — measured at 41.7 mm on a walk and 54.3 on a run — and for a foot
        // nothing was about to solve, nothing took it out again.
        //
        // It hid behind #254 for as long as both existed: the one-pass solve
        // aimed the ANKLE at the goal plus the rest hang, which is roughly
        // straight up from where the contact belongs, so a foot levelled
        // afterwards landed its sole at zero by accident.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let ground = |point: Vec3| Some(Ground::level(Vec3::new(point.x, 0.0, point.z)));

        let mut worst = 0.0f32;
        for sample in 0..120 {
            let cycle = sample as f32 / 120.0;
            let mut pose = Pose::rest(&rig);
            let steps = gait::step(&rig, &mut pose, &gait, &stride, cycle, ground);
            let before = pose.forward(&rig).positions;
            level_feet(&rig, &mut pose, ground, &FootingConfig::default());
            let after = pose.forward(&rig).positions;
            for &limb in steps.swing.iter().chain(&steps.stance) {
                let foot = rig.in_zone(Zone::Extremity(limb))[0];
                worst = worst.max(before[foot].distance(after[foot]));
            }
        }
        assert!(
            worst < 2e-3,
            "levelling dragged a contact {:.1} mm and nothing put it back",
            worst * 1000.0
        );
    }

    #[test]
    fn a_leg_that_falls_short_makes_the_body_sink_the_difference() {
        // **The crouch is chosen before the solve that would tell you better.**
        // It is sized with the hang the rest pose has; the solve then turns the
        // shin, which swings that hang and moves the ankle position the goal
        // implies — occasionally past full extension. Measured at 0.29 mm on
        // the default biped at one frame in 240, which `two_bone` correctly
        // refuses, so the gait reported a strained leg on level ground. `step`
        // now reads the shortfall back off the solved pose and sinks by it.
        //
        // **Asserted on where the foot ENDED UP, not on the strain flag.** At
        // full extension that flag is the wrong instrument twice over: the
        // solver holds back from the singularity by design, and the goal handed
        // to `two_bone` moves every pass as the hang is re-read, so its verdict
        // straddles the limit. How far the contact is from where the gait sent
        // it is what #254 is about and what a body shows.
        let rig = biped();
        for gait in [Gait::wave(&rig), Gait::natural(&rig)] {
            let stride = Stride::for_body(&rig, 1.0);
            let mut worst = 0.0f32;
            for sample in 0..240 {
                let cycle = sample as f32 / 240.0;
                let mut pose = Pose::rest(&rig);
                gait::step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
                let posed = pose.forward(&rig);
                for (index, &limb) in gait.limbs.iter().enumerate() {
                    let foot = rig.in_zone(Zone::Extremity(limb))[0];
                    let home = rig.joints[foot].position;
                    let goal = home + gait::contact_offset(home, &stride, gait.phase(index, cycle));
                    worst = worst.max(posed.positions[foot].distance(goal));
                }
            }
            // **41.72 mm before this, 2.42 mm after** — the worst any contact
            // lands from where the gait sent it, over a whole cycle on level
            // ground. The remainder is the solver's own hold-back at full
            // extension, which a rest-pose leg is permanently at.
            assert!(
                worst < 3e-3,
                "a contact ended {:.1} mm from where the gait sent it on level ground",
                worst * 1000.0
            );
        }
    }
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
