//! A winged body on the ground (#424): the bird's walk with its head-bob, its
//! hop and its run; the bat's crawl and bound; the glider's bound; and the
//! drawn toes laid on the ground through every stride.
//!
//! Every winged body is driven by the gait that drives a person and a beast -
//! a bird as a two-legged body, a bat and a glider as four-legged ones - and
//! what this module adds is what research-11 says each family does that the
//! others do not. The family is the leading one's ([`crate::Kinship`]), as the
//! posture is: a cross under the halfway blend goes as its own family.
//!
//! * **A bird's head holds and thrusts** (research-11 2.2): one bob a step,
//!   the head still along the ground while the body walks under it and then
//!   thrust to its next hold in about half the step, the hold starting just
//!   after a foot lifts. It fades out as the walk quickens and is gone at a
//!   Froude number of a half, where a walk turns into a run. Only a bird
//!   whose kind bobs ([`crate::Gaits::bob`]).
//! * **A bird hops** (research-11 2.1): both feet together, a run's duty and
//!   step length, a moment in the air; a small bird at every pace, a
//!   straight-beaked one in place of a run. Its head retracts as it lands and
//!   extends as it springs.
//! * **A bird runs without a jump in its duty** (research-11 fact 6): a
//!   grounded run past a walk's end, its feet never both off the ground until
//!   well into it.
//! * **A bat crawls on its wrists and feet and bounds**, and **a glider
//!   walks and bounds** (research-11 2.3 and 2.4): a lateral-sequence walk,
//!   then a half-bound - the hinds landing together and the fores a little
//!   apart - where a beast would trot ([`crate::anim::Pattern::Bound`]); a
//!   crawling bat's legs are its own (`winged::crawls`).
//! * **The drawn toes stay on the ground** (`lay_toes`): through a stance
//!   every toe lies on the ground or over it, as a foot's pad peels off
//!   about its toes; in the air none passes into it, and after the driver's
//!   blend a pad or a toe it left in the ground is lifted out of it
//!   (`keep_toes_up`).
//!
//! Nothing here moves a person or a beast: every entry asks the rig's wings
//! first, which only a winged body has.

use glam::{Quat, Vec3};

use crate::anim::gait::{Gait, Phase, Steps, Stride};
use crate::anim::ground::Ground;
use crate::anim::pose::Pose;
use crate::anim::speed::{FROUDE_TRANSITION, Speed};
use crate::det::{self, DetMath, Rot};
use crate::plan::{Limb, Zone};
use crate::rig::{Rig, Role};

use super::bird;

/// How a two-legged winged body goes at a speed: a walk, which turns into a
/// grounded run, or a hop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BirdGait {
    /// Alternate feet: a walk, and past its end a run.
    Walk,
    /// Both feet together.
    Hop,
}

/// How far below its own Froude number a bird that has taken up a hop goes
/// before it walks again, as a share: a beast's change down (`pattern::DOWN`).
const HOP_DOWN: f32 = 0.8;

/// The gait a bird goes in at `froude`, having gone in `before`: a hop from
/// its kind's [`crate::Gaits::hops_from`] on - changing back below four fifths of it,
/// as a beast's gaits do - and a walk otherwise. `None` on any other body.
pub(crate) fn bird_gait(rig: &Rig, froude: f32, before: Option<BirdGait>) -> Option<BirdGait> {
    let gaits = bird(rig)?;
    let Some(from) = gaits.hops_from else {
        return Some(BirdGait::Walk);
    };
    let from = if before == Some(BirdGait::Hop) {
        from * HOP_DOWN
    } else {
        from
    };
    Some(if froude >= from {
        BirdGait::Hop
    } else {
        BirdGait::Walk
    })
}

/// A bird's duty at the end of its walk, at the walk's end and at a sprint,
/// and the Froude number its run's feet first both leave the ground at.
///
/// Provenance: **the literature**, research-11 2.1 and fact 6: a bird changes
/// from a walk to a run near a Froude number of a half without a jump in its
/// duty, into a grounded run with a duty over a half, as quail do from 0.35 to
/// 1.4 m/s; a person's run drops to a third there. Its sprint's duty is a
/// person's ([`crate::anim::speed::SPRINT_DUTY`]).
const BIRD_RUN: (f32, f32) = (0.55, 0.5);
const BIRD_AIRBORNE_FROM: f32 = 1.2;

/// A walking bird's duty at `froude`: a person's walk to a walk's end, then
/// falling on without a step, through a grounded run to a person's sprint.
pub(crate) fn bird_duty(froude: f32) -> f32 {
    use crate::anim::speed::{FROUDE_SPRINT, SPRINT_DUTY, WALK_DUTY_FAST, WALK_DUTY_SLOW};
    if froude <= 0.0 {
        return 1.0;
    }
    if froude <= FROUDE_TRANSITION {
        let along = froude / FROUDE_TRANSITION;
        return WALK_DUTY_SLOW + (WALK_DUTY_FAST - WALK_DUTY_SLOW) * along;
    }
    if froude <= BIRD_AIRBORNE_FROM {
        let along = (froude - FROUDE_TRANSITION) / (BIRD_AIRBORNE_FROM - FROUDE_TRANSITION);
        return BIRD_RUN.0 + (BIRD_RUN.1 - BIRD_RUN.0) * along;
    }
    let along = ((froude - BIRD_AIRBORNE_FROM) / (FROUDE_SPRINT - BIRD_AIRBORNE_FROM)).min(1.0);
    BIRD_RUN.1 + (SPRINT_DUTY - BIRD_RUN.1) * along
}

/// A hop's duty at `froude`: a run's, as research-11 2.1 has it, "step length
/// and duty factor match the run's" - a person's run at its slowest
/// ([`crate::anim::gait::RUN_DUTY`]) to its sprint.
pub(crate) fn hop_duty(froude: f32) -> f32 {
    use crate::anim::gait::RUN_DUTY;
    use crate::anim::speed::{FROUDE_SPRINT, SPRINT_DUTY};
    if froude <= 0.0 {
        return 1.0;
    }
    let along =
        ((froude - FROUDE_TRANSITION) / (FROUDE_SPRINT - FROUDE_TRANSITION)).clamp(0.0, 1.0);
    RUN_DUTY + (SPRINT_DUTY - RUN_DUTY) * along
}

/// A bird's stride, as a share of its hip's height, at a Froude number of one,
/// and the power of the Froude number it grows by.
///
/// Provenance: **the literature**, research-11 2.1: "use Alexander's
/// `lambda/h = 2.3 Fr^0.3` with h the hip height ... Froude normalisation
/// removes most size effects" (Gatesy and Biewener). A person's step, from a
/// leg's reach by Grieve's relation, gave a bird's short leg a stride half as
/// long again, and the crouch for each long step dropped its body 22 mm in
/// two frames as each foot came down.
pub const BIRD_STRIDE: f32 = 2.3;
/// See [`BIRD_STRIDE`].
pub const BIRD_STRIDE_POWER: f32 = 0.3;

/// How much longer a hop is than a walking bird's step at the same Froude
/// number. Provenance: **chosen by render** (#424): at a run's step - research
/// 11 2.1, "step length matches the run's" - a songbird hopped at 5 Hz and its
/// feet cleared the ground by a quarter of its leg, which read as bouncing on
/// the spot; the dossier's rule of thumb (L) has a forage hop one to two body
/// lengths long and half to one high. Hopped this much further, as often a
/// second as it travels for, its time in the air throws it 2.6 times as high.
pub const HOP_STRIDE: f32 = 1.6;

/// How far a bird at `froude` covers between one footfall and the next, in
/// metres: half its stride walking, and [`HOP_STRIDE`] times that hopping.
pub(crate) fn bird_step(rig: &Rig, froude: f32, hops: bool) -> f32 {
    if froude <= 0.0 {
        return 0.0;
    }
    let step = crate::anim::speed::measure_of(rig)
        * BIRD_STRIDE
        * froude.det_powf(BIRD_STRIDE_POWER)
        * 0.5;
    if hops { step * HOP_STRIDE } else { step }
}

/// The Froude number a bird steps `step` metres a footfall at: [`bird_step`]
/// inverted.
pub(crate) fn bird_froude(rig: &Rig, step: f32, hops: bool) -> f32 {
    let measure = crate::anim::speed::measure_of(rig);
    if step <= 0.0 || measure <= f32::EPSILON {
        return 0.0;
    }
    let step = if hops { step / HOP_STRIDE } else { step };
    (2.0 * step / (measure * BIRD_STRIDE)).det_powf(1.0 / BIRD_STRIDE_POWER)
}

/// How much longer a bat's and a glider's bound is than a beast's stride at
/// the same Froude number. Provenance: **chosen by render** (#424): at a
/// beast's stride a bat bounded at 5 Hz, too fast for any moment off the
/// ground to show (3 mm at most), and read as a fast walk; a bounding
/// squirrel leaps a body length or two (bg, L). Longer still, 1.8, put a
/// glider's forelegs past their reach and their wrists turned 2 rad in a
/// frame.
pub const BOUND_STRIDE: f32 = 1.4;

/// A hop on `rig`'s legs: every foot down together, `duty` of the cycle.
pub(crate) fn hop_gait(rig: &Rig, duty: f32) -> Gait {
    let limbs = rig.ground_contacts();
    Gait {
        offsets: vec![0.0; limbs.len()],
        limbs,
        duty,
    }
}

/// Whether `gait` is a hop: two feet that land together and lift.
pub(crate) fn hops(gait: &Gait) -> bool {
    gait.limbs.len() == 2
        && gait.duty < 1.0
        && gait.offsets.windows(2).all(|pair| {
            (pair[0] - pair[1])
                .rem_euclid(1.0)
                .min((pair[1] - pair[0]).rem_euclid(1.0))
                < 1.0e-6
        })
}

/// The share of a step a bird's head thrusts through, the rest of it held.
/// Provenance: **the literature**, research-11 2.2: "about half of each
/// step".
pub const THRUST: f32 = 0.5;

/// How long after the trailing foot lifts a bird's head starts to hold, as a
/// share of a step. Provenance: **chosen** (#424) on research-11 2.2, "the
/// hold begins slightly after a foot lifts off; the thrust slightly before the
/// next touchdown".
pub const HOLD_LAG: f32 = 0.05;

/// The Froude numbers a bird's head-bob starts to fade at and is gone by.
/// Provenance: **the literature**, research-11 2.2: "fade the hold to zero as
/// Fr rises past about 0.5", where the walk turns into a run; and the fast
/// runner's neck "stretched forward continuously; no hold phase".
pub const BOB_FADE: (f32, f32) = (0.3, 0.5);

/// How far a hopping bird's head retracts and extends either side of where
/// it rides, as a share of its leg. Provenance: **chosen by render** (#424)
/// on research-11 2.1's hop sequence: the head retracts as the legs fold,
/// extends with them as the bird leaves the ground, and retracts again before
/// it lands.
const HOP_HEAD: f32 = 0.2;

/// Where a bird's head stands along its walk, ahead of where its body carries
/// it, as a share of the ground the body covers in a step: at `step`, how far
/// through the step it is, `0..1` from a foot landing, `lift` being where in
/// the step the other foot lifted.
///
/// **Held still along the ground, then thrust**: from just after the lift
/// the head goes back through the body as fast as the body goes forward, for
/// half the step; then it is thrust forward through the rest of it to its
/// next hold, eased in and out, so its speed over the ground rises from
/// nothing and falls back to it. Centred, so a walk carries the head where it
/// rode. **Not in height**: the head rides the walk's own bounce and dips as
/// the neck reaches, about a centimetre over a hold - a pigeon's head bobs a
/// little up and down too.
#[must_use]
pub fn bob_offset(step: f32, lift: f32) -> f32 {
    let held = 1.0 - THRUST;
    let at = (step - lift - HOLD_LAG).rem_euclid(1.0);
    let thrust = if at < held {
        0.0
    } else {
        let t = (at - held) / THRUST;
        t * t * (3.0 - 2.0 * t)
    };
    thrust - at + 0.5 * held
}

/// How much of its head-bob a bird walking at `froude` shows: all of it to
/// [`BOB_FADE`]'s start, none from its end.
#[must_use]
pub fn bob_share(froude: f32) -> f32 {
    let t = ((froude - BOB_FADE.0) / (BOB_FADE.1 - BOB_FADE.0)).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

/// Moves a walking bird's head through its hold and thrust, and a hopping
/// one's back and forth through its hop: along the walk, in the body, the
/// neck turning to carry the head and the head turned back so it keeps its
/// bearing. Call after the trunk is carried and the head levelled
/// ([`crate::anim::gait::Walk::drive`] does). Nothing on any other body, or on a
/// bird that is not stepping.
pub(crate) fn bob(rig: &Rig, pose: &mut Pose, gait: &Gait, stride: &Stride, cycle: f32) {
    let Some(gaits) = bird(rig) else {
        return;
    };
    if !pose.fits(rig) || gait.duty >= 1.0 || gait.is_empty() {
        return;
    }
    let along = Vec3::new(stride.direction.x, 0.0, stride.direction.z);
    let Some(along) = along.try_normalize() else {
        return;
    };
    let ahead = if hops(gait) {
        // Extended as it springs, at the end of its stance, and retracted
        // half a cycle later, before it lands.
        let at = (cycle - gait.offsets[0]).rem_euclid(1.0);
        let leg = crate::anim::speed::leg_of(rig);
        HOP_HEAD * leg * (std::f32::consts::TAU * (at - gait.duty)).det_cos()
    } else {
        if gaits.bob <= 0.0 {
            return;
        }
        let froude = Speed::of(rig, gait, stride).froude();
        let share = bob_share(froude) * gaits.bob;
        #[expect(clippy::cast_precision_loss, reason = "two feet")]
        let footfalls = gait.footfalls() as f32;
        let covered = stride.length / (gait.duty * footfalls).max(f32::EPSILON);
        if share <= 0.0 || covered <= f32::EPSILON {
            return;
        }
        // A step from each landing, and where in it the other foot lifts.
        let step = ((cycle - gait.offsets[0]) * footfalls).rem_euclid(1.0);
        let lift = (gait.duty * footfalls - 1.0).rem_euclid(1.0);
        bob_offset(step, lift) * covered * share
    };
    carry_head(rig, pose, along, ahead);
}

/// Moves the head `ahead` along `along`, in the body, by turning its neck, the
/// head turned back to keep the bearing it had.
fn carry_head(rig: &Rig, pose: &mut Pose, along: Vec3, ahead: f32) {
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some(neck) = rig.joints[head].parent else {
        return;
    };
    if ahead == 0.0 {
        return;
    }
    let posed = pose.forward(rig);
    let from = posed.positions[head] - posed.positions[neck];
    // Exactly `ahead` along the walk, the neck keeping its length: the head
    // dips a little as it reaches, and the hold is still along the ground.
    let level = from - Vec3::Y * from.y;
    let reached = level + along * ahead;
    let length = from.length();
    let rise = (length * length - reached.length_squared()).max(0.0).sqrt();
    let to = reached + Vec3::Y * rise.copysign(from.y);
    let (Some(was), Some(now)) = (from.try_normalize(), to.try_normalize()) else {
        return;
    };
    let turn = det::from_rotation_arc(was, now);
    let above = rig.joints[neck]
        .parent
        .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
    let neck_world = turn * posed.rotations[neck];
    pose.rotations[neck] = above.inverse() * neck_world;
    pose.rotations[head] = neck_world.inverse() * posed.rotations[head];
}
/// How much further a bat's and a glider's back folds and stretches in a
/// bound than a beast's at a gallop. Provenance: **chosen by render** (#424):
/// at a beast's, a bat's and a glider's bounding backs moved through 0.38
/// and 0.40 rad and read as a fast walk, and now through 0.61 and 0.63; a
/// bounding squirrel's back gathers and stretches through most of a right
/// angle (bg, L).
pub const BOUND_FLEX: f32 = 1.8;

/// Throws a bat's or a glider's bounding body up through the moment its feet
/// are all off the ground (#424): `g t^2 / 8` at the top for its time in the
/// air, as [`hop_rise`] throws a hop, the whole body rigidly, feet and all.
/// After the trunk is carried, which stands every leg back where the step
/// sent it. Nothing on any other body, or in any other gait.
pub(crate) fn bound(
    rig: &Rig,
    pose: &mut Pose,
    gait: &Gait,
    stride: &Stride,
    cycle: f32,
    steps: &mut Steps,
) {
    if !super::bounds(rig) || gait.limbs.len() != 4 || gait.duty >= 0.5 || !pose.fits(rig) {
        return;
    }
    let Some((progress, span)) = gait.flight_at(cycle) else {
        return;
    };
    let cadence = Speed::of(rig, gait, stride).cadence(rig);
    if cadence <= f32::EPSILON {
        return;
    }
    let time = span / cadence;
    let apex = BOUND_LEAP * crate::anim::speed::GRAVITY * time * time / 8.0;
    let lift = apex * 4.0 * progress * (1.0 - progress);
    pose.translation.y += lift;
    steps.rise += lift;
    // And the swinging feet go up with it as the step's own: left where the
    // step put them, the settle took a foot lifted past its two centimetres
    // for a layer's, stopped folding it, and its wrist snapped back.
    for (_, at) in &mut steps.placed {
        at.y += lift;
    }
}

/// How much higher a bat's or a glider's bound is thrown than its time in
/// the air would throw it. Provenance: **chosen by render** (#424): thrown
/// for its flight alone, a glider's bound cleared the ground by 8% of its
/// hip, which a side strip does not show, and a stride long enough to throw
/// it higher put its forelegs past their reach; thrown so, every foot leaves
/// the ground by 29% of a glider's hip and 36% of a bat's.
pub const BOUND_LEAP: f32 = 3.0;

/// How high a hopping bird rides above the ground at `cycle`, in metres, if
/// `gait` is a hop: the arc of a body thrown for as long as both feet are
/// off the ground, `g t^2 / 8` at its top. `None` for any other gait, which
/// rises as [`crate::anim::gait::flight_rise`] says.
///
/// **Ballistic, not a run's spring**: a hop is a jump on two feet, and its
/// height is what its time in the air throws a body to, as
/// [`crate::anim::Leap`]'s is.
pub(crate) fn hop_rise(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> Option<f32> {
    bird(rig)?;
    if !hops(gait) {
        return None;
    }
    let speed = Speed::of(rig, gait, stride);
    let cadence = speed.cadence(rig);
    let Some((progress, span)) = gait.flight_at(cycle) else {
        return Some(0.0);
    };
    if cadence <= f32::EPSILON {
        return Some(0.0);
    }
    let time = span / cadence;
    let apex = crate::anim::speed::GRAVITY * time * time / 8.0;
    Some(apex * 4.0 * progress * (1.0 - progress))
}

/// Lifts a bird-led body's folded wings, as a walk or a run carries the body
/// lower than it stands, back to no lower over the ground than they rest
/// (#424): each wing turned up about its root, across the body, by as much
/// as its lowest joint stands under its height at rest over the wing's
/// longest reach.
///
/// **What motion adds, not what the plan built**: a Bird x Bat cross at 0.49
/// stands with its folded primaries 83 mm under the ground (#433, the body's
/// own), and running at 3 m/s its crouch and lean took them 14 mm further;
/// they are held where they stood. Nothing on a body whose fore limbs are
/// legs, or one with no wings.
pub(crate) fn hold_wings<F>(rig: &Rig, pose: &mut Pose, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let Some(wings) = rig.wings.as_ref() else {
        return;
    };
    if rig.four_legged() || !pose.fits(rig) {
        return;
    }
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        let Some(&root) = rig.in_zone(Zone::UpperLimb(limb)).first() else {
            continue;
        };
        // The joints the wing carries: its own, below its root.
        let carried: Vec<usize> = (0..rig.len())
            .filter(|&joint| {
                wings.owns(joint) && {
                    let mut at = rig.joints[joint].parent;
                    while let Some(up) = at {
                        if up == root {
                            break;
                        }
                        at = rig.joints[up].parent;
                    }
                    at == Some(root)
                }
            })
            .collect();
        // The wing's furthest joint from its root, across the ground, and
        // how far: the lever every lift is reckoned on and turned toward, so
        // a lift is as smooth as the depth it answers, whichever joint is
        // lowest.
        let Some((far, reach)) = carried
            .iter()
            .map(|&joint| {
                let lever = rig.joints[joint].position - rig.joints[root].position;
                (joint, Vec3::new(lever.x, 0.0, lever.z).length())
            })
            .max_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        if reach <= f32::EPSILON {
            continue;
        }
        let posed = pose.forward(rig);
        // About the axis square to it, so the furthest joint rises; by the
        // most any joint well out along it asks, its depth over its own lever.
        let lever = posed.positions[far] - posed.positions[root];
        let toward = Vec3::new(lever.x, 0.0, lever.z);
        let Some(axis) = toward.cross(Vec3::Y).try_normalize() else {
            continue;
        };
        let toward = toward.normalize_or_zero();
        let asked = carried
            .iter()
            .filter_map(|&joint| {
                let at = posed.positions[joint];
                let out = (at - posed.positions[root]).dot(toward);
                if out < 0.5 * reach {
                    return None;
                }
                let below = ground(at)?;
                let up = below.normal.normalize_or(Vec3::Y);
                let short = rig.joints[joint].position.y - (at - below.position).dot(up);
                Some((short - WING_SLACK) / out)
            })
            .fold(0.0f32, f32::max);
        if asked <= 0.0 {
            continue;
        }
        let angle = -asked.clamp(0.0, 1.0).det_asin();
        let above = rig.joints[root]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        pose.rotations[root] =
            above.inverse() * Rot::axis_angle(axis, angle) * posed.rotations[root];
    }
}

/// How far under its rest height a folded wing may ride before
/// [`hold_wings`] lifts it, in metres: a millimetre.
const WING_SLACK: f32 = 0.001;

/// How long a swing a winged body's paw folds its whole fold over, in
/// seconds: a shorter one folds it the less.
/// Provenance: **measured** (#424): a glider bounding at 3 m/s swings a
/// forepaw in a tenth of a second, and folding its toes as a dog's in that
/// turned them 2.8 rad in a frame, where a dog at the same Froude number,
/// whose swing takes three times as long, turns 1.3.
const FOLD_SWING: f32 = 0.4;

/// How much of a beast's swing fold a winged body's four-legged stride
/// folds its paws by (#424): all of it over a swing of [`FOLD_SWING`] or
/// longer, less over a shorter, so a small body's fast stride turns its toes
/// no faster than a beast's.
pub(crate) fn fold_share(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let cadence = Speed::of(rig, gait, stride).cadence(rig);
    if cadence <= f32::EPSILON {
        return 1.0;
    }
    ((1.0 - gait.duty) / cadence / FOLD_SWING).clamp(0.0, 1.0)
}

/// How far ahead of its rest a foot's swing carries the toes' stance attitude
/// into the air before letting it go, as a share of the swing.
const TOES_LET_GO: f32 = 0.3;

/// The steepest a foot's toes are turned to meet the ground, either way, in
/// radians, and how finely the turn is sought.
const TOE_TURN: f32 = 1.2;
const TOE_STEPS: usize = 60;

/// The small turn [`lay_toes`] reads which way a turn moves a point with.
const TOE_NUDGE: f32 = 0.01;

/// Lays a winged foot's drawn toes on the ground (#424): through its stance
/// every toe lies on it or above it, the lowest touching, as its pad peels
/// off about them; in the air none passes into it, and the turn the stance
/// left them in lets go over the first part of the swing.
///
/// **About the pad**, which the roll has already put where its heel lift
/// takes it: the toes and the foot's own second node, which the toes grow
/// beside, are turned together about the pad's axis across the foot, by the
/// least turn that brings each of them to its own height at rest above the
/// ground - the height the plan built it at over a floor at nought. A beast's
/// paw, a person's foot and a glider's (which draws no toes) are left as they
/// were.
///
/// Call last, after the roll and a beast's [`crate::anim::quadruped`] articulation
/// ([`crate::anim::gait::Walk::settle`] does).
pub(crate) fn lay_toes<F>(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if rig.wings.is_none() || !pose.fits(rig) || gait.duty >= 1.0 {
        return;
    }
    for (index, &limb) in gait.limbs.iter().enumerate() {
        let phase = gait.phase(index, cycle);
        lay_foot(rig, pose, limb, Some(phase), ground);
    }
}

/// Lifts any drawn toe of a winged body's feet that the frame's last layer
/// left in the ground back onto it (#424), whatever is driving the body: a
/// blend from a stand into a walk carries the toes along the difference
/// between the two, and laid on the ground in each, they dip between them.
/// A foot whose toes are on the ground or over it is left as it is.
pub(crate) fn keep_toes_up<F>(rig: &Rig, pose: &mut Pose, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if rig.wings.is_none() || !pose.fits(rig) {
        return;
    }
    for limb in rig.ground_contacts() {
        // The pad first: a blend turns each joint of a leg on its own, and
        // the foot it carries between two that stand on the ground goes into
        // it - a bird setting off sank its pads 28 mm in the first frame.
        let Some(&pad) = rig.in_zone(Zone::Extremity(limb)).first() else {
            continue;
        };
        let posed = pose.forward(rig);
        let at = posed.positions[pad];
        if let Some(below) = ground(at) {
            let up = below.normal.normalize_or(Vec3::Y);
            // As far as the pad, its sole or the paw's second node stands in
            // the ground: a glider's forepaw, rolled by a blend as its pace
            // changed, put its sole 6 to 13 mm under with its pad on it (a
            // beast's does too, which is left to it).
            let held = posed.rotations[pad];
            let second = rig.in_zone(Zone::Extremity(limb)).get(1).map(|&joint| {
                (
                    held.inverse() * (posed.positions[joint] - at),
                    rig.joints[joint].position.y.max(0.0),
                )
            });
            let reach = |(offset, height): (Vec3, f32)| {
                height - (at + held * offset - below.position).dot(up)
            };
            // Only as the foot comes down: a swinging paw folds its toes
            // under it in the air, and lifted for them at once its leg jumped
            // 13 mm in a frame. Taken in over the last of its descent.
            let pad_short = rig.joints[pad].position.y.max(0.0) - (at - below.position).dot(up);
            let down = 1.0 - (-pad_short / SOLE_REACH).clamp(0.0, 1.0);
            let down = down * down * (3.0 - 2.0 * down);
            let rolled = sole(rig, pad)
                .into_iter()
                .chain(second)
                .map(|point| reach(point) - SOLE_SLACK)
                .fold(0.0f32, f32::max);
            let short = pad_short.max(rolled * down);
            // Lifted in the plane the leg is bent in as the blend draws it,
            // not solved afresh about the bend it was built with: a glider-led
            // cross's elbow, solved so, swung 43 mm in the frame it stopped.
            if short > PAD_SLACK
                && let Some(chain) = rig.limb_chain(limb)
            {
                let hang = posed.positions[chain[2]] - at;
                let [root, mid, tip] = chain.map(|joint| posed.positions[joint]);
                let line = (tip - root).normalize_or_zero();
                let bend = (mid - root) - line * (mid - root).dot(line);
                // A leg the blend holds nearly straight has no bend of its own
                // to keep: it is lifted about the one it was built with.
                let pole = if bend.length() > 0.1 * root.distance(mid) {
                    mid + bend.normalize() * rig.extent()
                } else {
                    rig.bend_pole(limb).unwrap_or(mid)
                };
                crate::anim::ik::two_bone(rig, pose, chain, at + up * short + hang, pole);
            }
        }
        lay_foot(rig, pose, limb, None, ground);
    }
}

/// A pad's sole: a ring of its radius round under it, on the floor at rest,
/// in its own frame, each with the height it stands at then.
fn sole(rig: &Rig, pad: usize) -> [(Vec3, f32); 4] {
    let (low, radius) = (rig.joints[pad].position.y, rig.joints[pad].radius);
    [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)]
        .map(|(across, ahead)| (Vec3::new(across * radius, -low, ahead * radius), 0.0))
}

/// How far a pad's sole may roll into the ground before [`keep_toes_up`]
/// lifts the pad by the rest, in metres: two millimetres, which a sole drawn
/// round its pad's ring takes in its curve, so a walking paw is left alone.
const SOLE_SLACK: f32 = 0.002;

/// How high a pad stands before its sole's roll is none of [`keep_toes_up`]'s
/// business, in metres: the last two centimetres of its descent.
const SOLE_REACH: f32 = 0.02;

/// How far a pad may stand in the ground before [`keep_toes_up`] lifts it out,
/// in metres: a tenth of a millimetre, under anything drawn.
const PAD_SLACK: f32 = 1.0e-4;

/// [`lay_toes`] for one foot, at `phase`, or [`keep_toes_up`] for it at none.
fn lay_foot<F>(rig: &Rig, pose: &mut Pose, limb: Limb, phase: Option<Phase>, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let own = rig.extremity_joints(limb);
    let zone = rig.in_zone(Zone::Extremity(limb));
    let (Some(&pad), Some(&second)) = (zone.first(), zone.get(1)) else {
        return;
    };
    if !own
        .iter()
        .any(|&joint| rig.joints[joint].role == Role::Digit)
    {
        return;
    }
    // Everything past the pad that stands on the ground: its second node and
    // its toes, with the height each was built at above the floor, and the
    // pad's own sole ([`sole`]).
    let points: Vec<(usize, f32)> = own
        .iter()
        .copied()
        .filter(|&joint| joint != pad && rig.joints[joint].zone == Zone::Extremity(limb))
        .map(|joint| (joint, rig.joints[joint].position.y.max(0.0)))
        .collect();
    let posed = pose.forward(rig);
    let at = posed.positions[pad];
    let Some(below) = ground(at) else {
        return;
    };
    let up = below.normal.normalize_or(Vec3::Y);
    let held = posed.rotations[pad];
    // The foot as it was built, square to the ground under it and pointing
    // where it points now: what a stance turns the toes from, so a foot the
    // leg's solve has rolled about its length is laid flat across it first.
    let flat = det::from_rotation_arc((held * Vec3::Y).normalize_or(Vec3::Y), up) * held;
    let mut rest: Vec<(Vec3, f32)> = points
        .iter()
        .map(|&(joint, height)| (held.inverse() * (posed.positions[joint] - at), height))
        .collect();
    rest.extend(sole(rig, pad));
    let rest = &rest;
    // How far the lowest point stands over its own height above the ground,
    // the foot held at `base` and turned by `angle` about its axis across.
    let clearance_from = |base: Quat| {
        let run = base * (rig.joints[second].position - rig.joints[pad].position);
        let axis = run.cross(up).try_normalize().unwrap_or(Vec3::X);
        move |angle: f32| {
            let turned = Rot::axis_angle(axis, angle) * base;
            rest.iter()
                .map(|&(offset, height)| (at + turned * offset - below.position).dot(up) - height)
                .fold(f32::MAX, f32::min)
        }
    };
    // Which way a turn goes about the foot's axis: the way that lifts its
    // lowest point out of the ground if one is in it, and otherwise the way
    // that lays it down toward its toes' tips - along its run, ahead of a
    // bird's pad and behind a bat's - so a stance's turn never jumps from
    // one side of the pad to the other between two frames.
    let way = |base: Quat| {
        let run = base * (rig.joints[second].position - rig.joints[pad].position);
        let axis = run.cross(up).try_normalize().unwrap_or(Vec3::X);
        let height = |angle: f32, offset: Vec3, rest: f32| {
            (at + Rot::axis_angle(axis, angle) * base * offset - below.position).dot(up) - rest
        };
        let lowest = rest
            .iter()
            .copied()
            .min_by(|a, b| height(0.0, a.0, a.1).total_cmp(&height(0.0, b.0, b.1)));
        let tip = rest
            .iter()
            .copied()
            .max_by(|a, b| (base * a.0).dot(run).total_cmp(&(base * b.0).dot(run)));
        let (Some(lowest), Some(tip)) = (lowest, tip) else {
            return 1.0;
        };
        if height(0.0, lowest.0, lowest.1) < 0.0 {
            if height(TOE_NUDGE, lowest.0, lowest.1) > height(0.0, lowest.0, lowest.1) {
                1.0
            } else {
                -1.0
            }
        } else if height(TOE_NUDGE, tip.0, tip.1) < height(0.0, tip.0, tip.1) {
            1.0
        } else {
            -1.0
        }
    };
    let laid = |base: Quat| {
        let clearance = clearance_from(base);
        let angle = nearest_root(&clearance, clearance(0.0), way(base));
        (base, angle)
    };
    let (base, angle) = match phase {
        // Down: square to the ground, the lowest point on it, however the
        // pad stands.
        Some(Phase::Stance(_)) => laid(flat),
        // Lifting: the stance's attitude let go of over the first of the
        // swing, and never into the ground.
        Some(Phase::Swing(t)) if t < TOES_LET_GO => {
            let t = t / TOES_LET_GO;
            let keep = 1.0 - t * t * (3.0 - 2.0 * t);
            let (_, rooted) = laid(flat);
            let base = det::slerp(held, flat, keep);
            let carried = rooted * keep;
            // Into the ground, lifted out from where it was carried to, the
            // way that lifts it: sought afresh from nought, a turn could land
            // on the pad's other side and whip the toes 30 mm in a frame.
            let clearance = clearance_from(base);
            if clearance(carried) < 0.0 {
                (base, lift_from(&clearance, carried))
            } else {
                (base, carried)
            }
        }
        // In the air, or under a blend: lifted out of the ground if it is
        // in it, square to it if a turn alone would not do it.
        _ => {
            if clearance_from(held)(0.0) >= 0.0 {
                return;
            }
            let (base, angle) = laid(held);
            if clearance_from(base)(angle) < -1.0e-4 {
                laid(flat)
            } else {
                (base, angle)
            }
        }
    };
    let run = base * (rig.joints[second].position - rig.joints[pad].position);
    let axis = run.cross(up).try_normalize().unwrap_or(Vec3::X);
    let turned = Rot::axis_angle(axis, angle) * base;
    let above = rig.joints[pad]
        .parent
        .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
    pose.rotations[pad] = above.inverse() * turned;
}

/// The least turn on from `start`, whichever way lifts it, at which
/// `clearance` - below nought at `start` - comes back to nought: `start`
/// itself if no turn within [`TOE_TURN`] of it does.
fn lift_from(clearance: &impl Fn(f32) -> f32, start: f32) -> f32 {
    let way = if clearance(start + TOE_NUDGE) >= clearance(start - TOE_NUDGE) {
        1.0
    } else {
        -1.0
    };
    #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
    let step = TOE_TURN / TOE_STEPS as f32;
    let mut inside = start;
    for index in 1..=TOE_STEPS {
        #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
        let outside = start + step * index as f32 * way;
        if clearance(outside) >= 0.0 {
            let mut outside = outside;
            for _ in 0..24 {
                let middle = 0.5 * (inside + outside);
                if clearance(middle) < 0.0 {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            return outside;
        }
        inside = outside;
    }
    start
}

/// The turn nearest nought, `way` (`1` or `-1`) within [`TOE_TURN`], at which
/// `clearance` crosses nought - `at_zero` being what it reads at nought - or
/// nought if it never does.
fn nearest_root(clearance: &impl Fn(f32) -> f32, at_zero: f32, way: f32) -> f32 {
    if at_zero == 0.0 {
        return 0.0;
    }
    let below = at_zero < 0.0;
    #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
    let step = TOE_TURN / TOE_STEPS as f32;
    // Where it came nearest nought, if it never reaches it: the toes
    // reaching as far as they go toward the ground, or out of it, so the
    // turn eases to that as the root runs out rather than dropping to
    // nought - a pad lifting past its toes' reach in its swing let go of a
    // whole turn in one frame.
    let mut nearest = (at_zero.abs(), 0.0f32);
    for index in 1..=TOE_STEPS {
        #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
        let angle = step * index as f32 * way;
        let there = clearance(angle);
        if there.abs() < nearest.0 {
            nearest = (there.abs(), angle);
        }
        if (there < 0.0) != below || there == 0.0 {
            // Between the last step and this one: halve the gap.
            let (mut inside, mut outside) = (angle - step * way, angle);
            for _ in 0..24 {
                let middle = 0.5 * (inside + outside);
                if (clearance(middle) < 0.0) == below {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            return outside;
        }
    }
    if nearest.1 == 0.0 {
        return 0.0;
    }
    // About the nearest step, narrowed to where it comes nearest.
    let (mut low, mut high) = (nearest.1 - step, nearest.1 + step);
    for _ in 0..24 {
        let (one, two) = ((2.0 * low + high) / 3.0, (low + 2.0 * high) / 3.0);
        if clearance(one).abs() <= clearance(two).abs() {
            high = two;
        } else {
            low = one;
        }
    }
    0.5 * (low + high)
}
