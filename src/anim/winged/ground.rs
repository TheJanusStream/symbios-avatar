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
//!   about its toes; in the air none passes into it; standing, the idle
//!   lays them on the ground as a stance does (`stand_toes`, #436); and
//!   after the driver's blend a pad or a toe it left in the ground is lifted
//!   out of it (`keep_toes_up`).
//! * **A body on four legs is kept out of the ground it is handed**
//!   (`keep_clear`, #440 and #441): on the pose handed back, its belly
//!   lifted off the ground under it, a crawling body's hands laid along it -
//!   thumb, membrane and folded digits with the wrist - and its tail lifted
//!   off it, the ground under a hand, the belly and each joint of the tail
//!   read as a plane round it. A level floor is all the build knows; a slope
//!   is the pose's.
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
    pose.rotations[neck] = (above.inverse() * neck_world).normalize();
    pose.rotations[head] = (neck_world.inverse() * posed.rotations[head]).normalize();
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
/// **What motion adds, not where the build stands them**: a Bird x Bat cross
/// at 0.3, whose long folded feathers the hug lays down its flank to 6.6 mm
/// over the ground, took them 10.4 mm lower running at 3 m/s, by its crouch
/// and its lean, 3.8 mm into the ground (at 0.49, 14 mm, until #440 laid
/// that cross's wing back along the body, `wing::feather`); they are held
/// where they stood. Nothing on a body whose fore limbs are legs, or one
/// with no wings.
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
            (above.inverse() * Rot::axis_angle(axis, angle) * posed.rotations[root]).normalize();
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
///
/// **A turning stride's swing is timed by its working foot** (#446), as its
/// cadence is ([`working_stride`]): a body stepping round where it stands
/// covers no ground, the speed its stride's length reads back had no cadence,
/// and so every pivot folded its paws whole, a beast's fold over a swing of
/// less than a tenth of a second - a glider's forepaw turned 1.8 rad a
/// sixtieth at 144 frames a second and 1.5 in one frame at 60, and a Bat x
/// Glider cross's 2.0. **And by [`PIVOT_FOLD`] of that for as much of the
/// stride as is its turn**, a pivot's swing being over sooner than its
/// cadence says. A stride that does not turn is timed as it was, to the bit.
pub(crate) fn fold_share(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let (stride, turned) = working_stride(rig, stride);
    let cadence = Speed::of(rig, gait, &stride).cadence(rig);
    if cadence <= f32::EPSILON {
        return 1.0;
    }
    let share = ((1.0 - gait.duty) / cadence / FOLD_SWING).clamp(0.0, 1.0);
    share * (1.0 - (1.0 - PIVOT_FOLD) * turned)
}

/// `stride` as the foot furthest from the body's middle takes it, and how
/// much of it is its turn, `0..=1`: its stance the length of that foot's own
/// path, the stride's travel and the arc its turn carries the foot round
/// together, and the share of that the arc is. A stride that does not turn is
/// itself, to the bit, and none of it its turn.
///
/// **A turning stride is stepped at its working foot's pace**
/// ([`crate::anim::turn::Turn::cadence`]), not its middle's: a body stepping
/// round where it stands covers no ground at all, so the speed its stride's
/// length reads back is a standstill's, with no cadence to time a swing by.
fn working_stride(rig: &Rig, stride: &Stride) -> (Stride, f32) {
    if stride.yaw == 0.0 {
        return (*stride, 0.0);
    }
    let reach = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .map(|joint| {
            let at = rig.joints[joint].position;
            (at.x * at.x + at.z * at.z).sqrt()
        })
        .fold(0.0f32, f32::max);
    let round = reach * stride.yaw;
    let (along, round) = (stride.length * stride.length, round * round);
    if along + round <= f32::EPSILON {
        return (*stride, 0.0);
    }
    (
        Stride {
            length: (along + round).sqrt(),
            ..*stride
        },
        round / (along + round),
    )
}

/// How much of `stride` is its turn, `0..=1`, as [`working_stride`] reckons
/// it at the foot furthest from the body's middle: nought on any body but a
/// winged one, and on a stride that does not turn. Read by the swing's seat
/// (`gait::seated_offset`, #446), which looks along a turning swing's own
/// flight for ground to clear for as much of it as this.
pub(crate) fn turned_share(rig: &Rig, stride: &Stride) -> f32 {
    if rig.wings.is_none() {
        return 0.0;
    }
    working_stride(rig, stride).1
}

/// How much of the fold its swing's length earns a stride that only turns
/// folds its paws by: see [`fold_share`].
/// Provenance: **measured** (#446): a glider stepping round at a radian a
/// second swings a forepaw in 0.08 s where its working foot's cadence says
/// 0.15, the pivot's clock gaining on its cadence for as long as the turn
/// eases (`driver::carry_scaled`, #415), which is most of any turn. At a
/// half, no joint of a glider, a Bat x Glider or a Glider x Bird cross turns
/// more than 0.42 rad a sixtieth stepping or spinning round on the level at
/// 30, 60 or 144 frames a second, and on a 20 degree slope up, down or across
/// it no more than 0.51 (the Bat x Glider spinning at 144) once its swing
/// looks along its own flight for ground to clear (`gait::seated_offset`):
/// before that, a Glider x Bird spinning across the slope turned a joint
/// 4.12 rad, its elbow thrown, which was the seat's and not the fold's.
const PIVOT_FOLD: f32 = 0.5;

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

/// Lays a standing winged body's drawn toes on the ground as a stance lays
/// them (#436): each foot of `feet` square to the ground under it, its
/// lowest point on it ([`lay_toes`] down), for the idle to stand on - each
/// by as much as it is down, given beside it in `0..=1`: a foot the idle
/// steps is let go of as it lifts and laid again as it lands.
///
/// **In the idle's own pose, before any blend**, so a change into the idle -
/// a walk stopping, a landing handed back - eases the toes onto the ground
/// with every other joint. Left to the pass after the blend
/// ([`keep_toes_up`]), which has no state and no time, a stop on a slope
/// either left the toes level with the body for as long as it stood there,
/// or laid them on some frames and turned them about the pad on others, a
/// bat's hind toe flipping 12 mm for a frame or two. **And by as much as
/// each foot is down**: laid only once a stepping foot had landed, a cross's
/// hind toes turned 0.36 radians in the frame its step ended.
pub(crate) fn stand_toes<F>(rig: &Rig, pose: &mut Pose, feet: &[(Limb, f32)], ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if rig.wings.is_none() || !pose.fits(rig) {
        return;
    }
    for &(limb, down) in feet {
        let Some(&pad) = rig.in_zone(Zone::Extremity(limb)).first() else {
            continue;
        };
        let down = down.clamp(0.0, 1.0);
        if down <= 0.0 {
            continue;
        }
        let stood = pose.rotations[pad];
        lay_foot(rig, pose, limb, Some(Phase::Stance(0.5)), ground);
        if down < 1.0 {
            pose.rotations[pad] = det::slerp(stood, pose.rotations[pad], down);
        }
    }
}

/// How high a foot the idle steps is lifted before [`stand_toes`] lets its
/// toes go whole, in metres: the last two centimetres of its step, as
/// [`SOLE_REACH`] is of a stride's descent.
pub(crate) const STAND_REACH: f32 = 0.02;

/// Lifts any drawn toe of a winged body's feet that the frame's last layer
/// left in the ground back onto it (#424), whatever is driving the body: a
/// blend from a stand into a walk carries the toes along the difference
/// between the two, and laid on the ground in each, they dip between them.
/// A foot whose toes are on the ground or over it is left as it is.
pub(crate) fn keep_toes_up<F>(rig: &Rig, pose: &mut Pose, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    keep_feet_up(rig, pose, ground, &rig.ground_contacts());
}

/// [`keep_toes_up`] for the feet of `limbs` alone: what a clip's wing keeps
/// out of the ground of the legs it lifts (`rests::wing_on`, #457).
pub(crate) fn keep_feet_up<F>(rig: &Rig, pose: &mut Pose, ground: &F, limbs: &[Limb])
where
    F: Fn(Vec3) -> Option<Ground> + ?Sized,
{
    if rig.wings.is_none() || !pose.fits(rig) {
        return;
    }
    for &limb in limbs {
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

/// Keeps what a winged body on four legs draws beside its feet out of the
/// ground it is handed (#441): its belly lifted off it ([`lift_belly`]), a
/// crawling body's hands laid along it ([`lay_hands`]) and its tail lifted
/// off it ([`lift_tail`]). Each reads the ground under the part itself and
/// the pose it is given, and nothing kept from a frame before, so a frame
/// and the next answer alike; on a level floor a hand is left to the bit,
/// and a belly and a tail that stand clear of it. `share` of the belly and
/// the tail, and `hands` of the hands, each `0..=1`: the driver eases them
/// in and out. A hand a clip's wing has taken, by how far it has taken it
/// (`spread`, from `rests::wings_taken`, which the driver eases out of once
/// a gesture is cut short), is let go of: see [`lay_hands`].
///
/// Call on the pose as it is handed back, after the driver's blend and
/// never on the pose kept to blend from ([`crate::anim::Driver::drive`]
/// does, at the whole share whatever carries the body but the water, which
/// floats it, eased between over two blends; and the belly and the tail
/// not in a rest, which lays them on the ground itself - its hands it
/// stands as the idle does, for this to lay, #450).
///
/// **The ground under a hand, the belly and each joint of the tail is read
/// as a plane round it** ([`plane_under`]), not by the normal at one point
/// (the tail's since #453): the ground a game
/// hands a body is often heights on a grid, bilinear between them, whose
/// tilt jumps at every cell's edge - Overlands' every quarter metre, edges
/// that stand still under a body as it goes - and a fore pad crosses one
/// every stride. Read at the pad, a bat's hand turned 7.3 degrees, then 1.5,
/// then 7.0 in single frames walking toward a 10 cm crest, and everything
/// it carries swung 45 mm a sixtieth and back. Each axis of the plane takes
/// the gentler of its two sides, so a hand on the flat beside a step reads
/// the flat, not half the ramp a grid makes of the step.
pub(crate) fn keep_clear<F>(
    rig: &Rig,
    pose: &mut Pose,
    ground: &F,
    share: f32,
    (hands, spread): (f32, &[(Limb, f32)]),
) where
    F: Fn(Vec3) -> Option<Ground>,
{
    if rig.wings.is_none()
        || !rig.four_legged()
        || !pose.fits(rig)
        || (share <= 0.0 && hands <= 0.0)
    {
        return;
    }
    #[cfg(test)]
    if LEFT_OUT.with(std::cell::Cell::get) {
        return;
    }
    let (share, hands) = (share.min(1.0), hands.min(1.0));
    if share > 0.0 {
        lift_belly(rig, pose, ground, share);
    }
    if hands > 0.0 {
        lay_hands(rig, pose, ground, hands, spread);
    }
    if share > 0.0 {
        lift_tail(rig, pose, ground, share);
    }
}

#[cfg(test)]
thread_local! {
    /// Whether [`keep_clear`] is left out on this thread: the motion its own
    /// tests measure it against (`winged::keep_clear_tests`). Per thread, so
    /// a test that leaves it out changes nothing for one run beside it.
    pub(super) static LEFT_OUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// How much lift [`keep_elbows_up`] gives an elbow `short` of its ring's
/// height over the ground, eased in over `ease` either side of touching:
/// none from `ease` clear of it, all of `short` from `ease` into it, and
/// between the two a curve with the slope of each at its ends.
fn eased_lift(short: f32, ease: f32) -> f32 {
    if ease <= 0.0 {
        return short.max(0.0);
    }
    if short <= -ease {
        0.0
    } else if short >= ease {
        short
    } else {
        (short + ease) * (short + ease) / (4.0 * ease)
    }
}

/// Over how much of its ring's height either side of touching the ground
/// [`keep_elbows_up`] eases an elbow's lift in ([`eased_lift`]), as a share
/// of that height. The lift never falls short of keeping the ring out of the
/// ground: it only begins sooner.
/// Provenance: **measured** (#462's fixer), over roll 9's emotes and rests
/// on slopes of 10 to 30 degrees: no easing turned its foreleg 0.64 rad a
/// sixtieth at 144 frames a second, half of the ring 0.31 and the whole
/// ring 0.23 (its Stretch across a 20 degree slope, against 0.10 for the
/// clip alone).
const ELBOW_EASE: f32 = 1.0;

/// How near the lowest point of its circle about the shoulder-to-wrist line,
/// in radians round it, [`keep_elbows_up`] hands the swing of an elbow drawn
/// on the side away from the body over to the side toward it.
/// Provenance: **measured** (#462's fixer): over roll 9's emotes and rests
/// on slopes of 10 to 30 degrees, a Stretch draws a foreleg's elbow on the
/// side away from the body from 0.3 rad off that lowest point and is swung
/// out there, as it always was; its Curl crosses the lowest point, and over
/// this its shoulder turns no faster than the clip's own 0.21 rad a
/// sixtieth.
const ELBOW_WINDOW: f32 = 0.3;

#[cfg(test)]
thread_local! {
    /// Whether [`keep_elbows_up`] is left out on this thread: the motion its
    /// own test measures it against (`winged::keep_clear_tests`).
    pub(super) static ELBOWS_LEFT_OUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// How far round a pad [`lay_hands`] reads the ground's tilt, in the hip's
/// height either side ([`plane_under`]).
///
/// Provenance: **measured** (#441), on the game's ground (`keep_clear_tests`):
/// what a bat's hands are turned by stepped 14.4 mm a sixtieth from one
/// frame to the next, walking over a 0.25 m bilinear grid toward a 25 cm
/// crest, read at one hip either side, 8.6 at two and 112.7 read at the pad
/// alone; and its drawn points stood 2.5 mm under the crest at one, one and
/// a half and two, 9.0 read at the pad and 16.9 at seven (a leg's reach,
/// which is a bat's wing arm). One and a half: for a bat 0.15 m, more than
/// half a cell either side, the hand's span with its thumb.
const HAND_PLANE: f32 = 1.5;

/// How far round the trunk's middle [`lift_belly`] reads the ground's tilt,
/// in the trunk's length either side: half, so the plane spans the trunk.
pub(super) const BELLY_PLANE: f32 = 0.5;

/// How far round each of its joints [`lift_tail`] reads the ground's tilt,
/// in the tail's length either side ([`plane_under`]): half, so the plane
/// spans the tail, as [`BELLY_PLANE`]'s spans the trunk.
///
/// Provenance: **chosen** (#453). Read at the joint alone, a Bat x Glider
/// cross's tail lifted out of the game's ground rising 20 degrees behind it
/// stepped 2.2 mm at each cell's edge it was carried over (a step that stayed
/// as the sampling doubled, `keep_clear_tests`); read over a plane a quarter,
/// a half or a whole of its length either side, it moves smoothly, and the
/// game's ground walking, bounding and turning reads alike at all three.
const TAIL_PLANE: f32 = 0.5;

/// The ground under `at` as a plane over `reach` either side of it: where
/// it stands, and the tilt read along each axis from the ground `reach` to
/// either side, or the ground's own normal where any of those answers
/// nothing.
///
/// **Each axis takes the gentler of its two sides** ([`limited`]), and none
/// where they rise opposite ways. A central difference across both sides
/// read a step's ramp - a curb or a deck's edge, which a host's height grid
/// hands over as a ramp a cell wide - beside a hand standing on the flat,
/// turned the hand down into the flat by up to 0.33 rad and laid its edge
/// 14.8 mm into ground it cleared without `keep_clear` (a bat beside a 20 cm
/// step; a bat over a 20 cm drop ahead read 37.3 mm under, worse than with
/// no `keep_clear` at all). Taking the gentler side reads level beside a
/// step and over a crest, and it is still a plane's own tilt on a plane and
/// exactly up on a level floor; and on ground whose heights are continuous -
/// a bilinear grid's are, where its normal is not - the tilt is continuous
/// too.
pub(super) fn plane_under<F>(ground: &F, at: Vec3, reach: f32) -> Option<Ground>
where
    F: Fn(Vec3) -> Option<Ground> + ?Sized,
{
    let below = ground(at)?;
    if reach <= 0.0 {
        return Some(below);
    }
    let sample = |offset: Vec3| ground(at + offset).map(|answer| answer.position);
    let (Some(right), Some(left), Some(ahead), Some(behind)) = (
        sample(Vec3::X * reach),
        sample(-Vec3::X * reach),
        sample(Vec3::Z * reach),
        sample(-Vec3::Z * reach),
    ) else {
        return Some(below);
    };
    let here = below.position;
    // How fast the ground rises from `from` to `to` along `axis`.
    let rise = |from: Vec3, to: Vec3, axis: Vec3| {
        let run = (to - from).dot(axis);
        if run.abs() <= f32::EPSILON {
            0.0
        } else {
            (to.y - from.y) / run
        }
    };
    let across = limited(rise(here, right, Vec3::X), rise(left, here, Vec3::X));
    let along = limited(rise(here, ahead, Vec3::Z), rise(behind, here, Vec3::Z));
    // Taken from nought rather than negated, so a level ground's tilt has no
    // negative zero in it and reads exactly up.
    let normal = Vec3::new(0.0 - across, 1.0, 0.0 - along);
    if !normal.is_finite() {
        return Some(below);
    }
    Some(Ground {
        position: below.position,
        normal: normal.normalize(),
    })
}

/// The gentler of two rises where they agree in sign, and none where they
/// do not (a minmod limiter): what [`plane_under`] takes along each axis. It
/// is continuous in both, so a tilt read from continuous heights stays
/// continuous.
fn limited(one: f32, other: f32) -> f32 {
    if one * other <= 0.0 {
        0.0
    } else if one.abs() <= other.abs() {
        one
    } else {
        other
    }
}

/// How far over the ground the trunk's joints stand, each in its own ring's
/// radius, for [`lift_belly`] to know its skin is clear without reading it:
/// a ring and three twentieths.
///
/// Provenance: **measured** (#441): the skin of a bat, a glider, their
/// crosses at the halfway blend and the crosses with a bird a bat or a
/// glider leads reaches at most 6.1 mm below its joint less its ring (a
/// glider's, on a ring of 85 mm: 0.072 of it), standing, walking, bounding
/// and turning on the level and on 20 degree slopes every way. A bat's
/// rings are 94 mm on a belly 32 mm off the ground, so a ring and a quarter
/// never let it off.
const TRUNK_CLEAR: f32 = 1.15;

/// The turn `joint` is drawn in, `top` and the joints down to it turned
/// since `posed` was read and everything above `top` not: what a fresh
/// [`Pose::forward`] answers for it, to the bit, without one.
fn turned_below(
    rig: &Rig,
    pose: &Pose,
    posed: &crate::anim::pose::Posed,
    top: usize,
    joint: usize,
) -> Quat {
    let mut path = Vec::new();
    let mut at = Some(joint);
    while let Some(here) = at {
        path.push(here);
        if here == top {
            break;
        }
        at = rig.joints[here].parent;
    }
    let base = rig.joints[top]
        .parent
        .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
    path.iter()
        .rev()
        .fold(base, |turn, &here| turn * pose.rotations[here])
}

/// The bend a leg is drawn in, as the point its middle joint bends toward:
/// its own in `posed`, and where that is nearly straight the one it was
/// built with ([`keep_toes_up`]'s, which says why).
fn drawn_pole(rig: &Rig, posed: &crate::anim::pose::Posed, limb: Limb, chain: [usize; 3]) -> Vec3 {
    let [root, mid, tip] = chain.map(|joint| posed.positions[joint]);
    let line = (tip - root).normalize_or_zero();
    let bend = (mid - root) - line * (mid - root).dot(line);
    if bend.length() > 0.1 * root.distance(mid) {
        mid + bend.normalize() * rig.extent()
    } else {
        rig.bend_pole(limb).unwrap_or(mid)
    }
}

/// Lifts a four-legged winged body by as far as its trunk's skin stands in
/// the ground under it (#441), its feet left where they stand: the skin read
/// off the rig's outline of it, square to the ground under the trunk's
/// middle, and every leg solved back to its foot in the bend it is drawn in.
///
/// **A stride sinks the trunk for its legs' reach** (`quadruped::depth`),
/// and up a slope the more, a paw at the back of its stance the lower by the
/// slope's share of its reach behind: a beast's belly stands a leg's length
/// over the ground and has it to spare, where a bat's stands 32 mm over it
/// and bounding up a 20 degree slope went 19 mm in, a bat-led cross with a
/// bird's deep keel 22. A leg that cannot reach its foot from there lifts
/// it; a trunk that stands clear is left as it is, and **one whose every
/// joint stands its ring and more over the ground is not read at all**
/// ([`TRUNK_CLEAR`]): read every frame, a bat standing on the level paid 25
/// microseconds of a 233 frame for nothing.
fn lift_belly<F>(rig: &Rig, pose: &mut Pose, ground: &F, share: f32)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let (Some(&pelvis), Some(&withers)) = (
        rig.in_zone(Zone::Pelvis).first(),
        rig.in_zone(Zone::Chest).last(),
    ) else {
        return;
    };
    let posed = pose.forward(rig);
    let middle = 0.5 * (posed.positions[pelvis] + posed.positions[withers]);
    let reach = BELLY_PLANE * posed.positions[pelvis].distance(posed.positions[withers]);
    let Some(below) = plane_under(ground, middle, reach) else {
        return;
    };
    let up = below.normal.normalize_or(Vec3::Y);
    let trunk = |zone: Zone| matches!(zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest);
    // Nothing to read where every joint of the trunk stands well over the
    // ground by its own ring: a body standing or going on the level, which
    // is most of them, pays no reading of its skin.
    let floor = below.position.dot(up);
    if (0..rig.len()).all(|joint| {
        !trunk(rig.joints[joint].zone)
            || posed.positions[joint].dot(up) - floor > TRUNK_CLEAR * rig.joints[joint].radius
    }) {
        return;
    }
    // The outline reads heights; turned so the ground's own up is the
    // height, it reads how far off the ground's plane the skin stands.
    let low = if up == Vec3::Y {
        crate::anim::rest::lowest_in(rig, &posed, trunk)
    } else {
        let level = det::from_rotation_arc(up, Vec3::Y);
        let turned = crate::anim::pose::Posed {
            positions: posed.positions.iter().map(|&at| level * at).collect(),
            rotations: posed.rotations.iter().map(|&turn| level * turn).collect(),
        };
        crate::anim::rest::lowest_in(rig, &turned, trunk)
    };
    let short = share * (below.position.dot(up) - low);
    if short <= 0.0 || !short.is_finite() {
        return;
    }
    let feet: Vec<(Limb, [usize; 3], Vec3, Quat, Vec3)> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| {
            let chain = rig.limb_chain(limb)?;
            Some((
                limb,
                chain,
                posed.positions[chain[2]],
                posed.rotations[chain[2]],
                drawn_pole(rig, &posed, limb, chain),
            ))
        })
        .collect();
    pose.translation += up * short;
    for (_, chain, at, held, pole) in feet {
        crate::anim::ik::two_bone(rig, pose, chain, at, pole + up * short);
        let above = turned_below(rig, pose, &posed, chain[0], chain[1]);
        pose.rotations[chain[2]] = (above.inverse() * held).normalize();
    }
}

/// Lays a crawling body's hands along the ground under them (#441): each
/// wrist it stands on turned with the ground, from the level it was built
/// over to the slope under its pad, about the pad, which stays where and
/// as it stands - and with the wrist every part of the wing it carries: the
/// thumb ahead of it, the membrane's leading edge beside it and the folded
/// digits over it.
///
/// **A crawling wrist stands upright, whatever it stands on**
/// (`winged::crawls`), and its thumb is built along a level floor ahead of
/// it (`wing::membrane`, #433): on ground rising 20 degrees ahead a bat's
/// ran 37 mm into it, walking and bounding, and the membrane's edge beside
/// the wrist 17 mm into ground rising to one side. **While the hand is
/// near the ground**: whole within a leg's reach of where it stands and let
/// go of over the next, so a wing in the air is the flight's. The slope is
/// the plane round the pad ([`plane_under`], [`HAND_PLANE`]). On a level
/// floor nothing is turned.
///
/// **And let go of as a clip's wing takes the hand** (#457), by `spread`'s
/// share for its limb: a Wing Stretch lifts the hand only a few centimetres
/// as its wing opens, and turned with the slope there, about a pad no longer
/// on it, the thumb of a wing being laid out went 12 mm into a slope a bat
/// faced down. The wing lays itself out over the ground (`rests::wing_on`).
fn lay_hands<F>(rig: &Rig, pose: &mut Pose, ground: &F, share: f32, spread: &[(Limb, f32)])
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if !super::crawls(rig) {
        return;
    }
    let leg = crate::anim::speed::leg_of(rig);
    if leg <= f32::EPSILON {
        return;
    }
    // Read again only once a hand is turned.
    let mut posed = pose.forward(rig);
    for limb in rig.ground_contacts() {
        if !limb.is_fore() {
            continue;
        }
        let (Some(chain), Some(&pad)) = (
            rig.limb_chain(limb),
            rig.in_zone(Zone::Extremity(limb)).first(),
        ) else {
            continue;
        };
        let wrist = chain[2];
        let at = posed.positions[pad];
        let hip = crate::anim::quadruped::hip_height(rig).unwrap_or(0.0);
        let Some(below) = plane_under(ground, at, HAND_PLANE * hip) else {
            continue;
        };
        let up = below.normal.normalize_or(Vec3::Y);
        if up == Vec3::Y {
            continue;
        }
        // Whole while the hand is within a leg's reach of where it stands,
        // and let go of over the next: a bound throws a bat's hands a third
        // of its hip's height off the ground every cycle, and let go of by
        // that, the folded digits over each wrist swung 67 mm in a frame.
        let lifted = (at - below.position).dot(up) - rig.joints[pad].position.y;
        let down = 1.0 - ((lifted - leg) / leg).clamp(0.0, 1.0);
        let spread = spread
            .iter()
            .find(|(spread, _)| *spread == limb)
            .map_or(0.0, |&(_, spread)| spread.clamp(0.0, 1.0));
        let down = share * (1.0 - spread) * down * down * (3.0 - 2.0 * down);
        if down <= 0.0 {
            continue;
        }
        let turn = det::slerp(Quat::IDENTITY, det::from_rotation_arc(Vec3::Y, up), down);
        let held = posed.rotations[wrist];
        let to = at + turn * (posed.positions[wrist] - at);
        let pole = drawn_pole(rig, &posed, limb, chain);
        crate::anim::ik::two_bone(rig, pose, chain, to, pole);
        let above = turned_below(rig, pose, &posed, chain[0], chain[1]);
        pose.rotations[wrist] = (above.inverse() * turn * held).normalize();
        // The pad keeps the attitude it stood in: the footing has laid its
        // sole on the ground already, and turned again with the wrist it
        // pitched twice over, its front edge 5 mm into a slope falling away.
        pose.rotations[pad] = ((turn * held).inverse() * posed.rotations[pad]).normalize();
        posed = pose.forward(rig);
    }
}

/// Lifts a four-legged winged body's tail off the ground (#440, #441): turned
/// up about its root, across the body, by as much as the lowest of its joints
/// stands in the ground under it - each kept its own ring's depth over it, or
/// as high as it rests if that is lower - the ground under each read as a
/// plane round it, half the tail's length either side ([`TAIL_PLANE`],
/// #453).
///
/// **Nothing else holds a tail up**: it is carried rigidly on the pelvis and
/// waved, and a Bat x Glider cross's, which rests 19 mm off the floor, went
/// 21.6 mm into it as its back first gathered into a bound, and 53 mm
/// bounding up a 20 degree slope, the ground rising under a tail the trunk's
/// pitch had not yet lifted. A tail that stands clear is left as it is.
fn lift_tail<F>(rig: &Rig, pose: &mut Pose, ground: &F, share: f32)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let tail = rig.tail();
    let (Some(&dock), Some(&tip)) = (tail.first(), tail.last()) else {
        return;
    };
    if dock == tip {
        return;
    }
    let length: f32 = tail
        .windows(2)
        .map(|pair| {
            rig.joints[pair[0]]
                .position
                .distance(rig.joints[pair[1]].position)
        })
        .sum();
    let reach = TAIL_PLANE * length;
    // Twice: a tail is no straight lever, and the first turn leaves a
    // little of a curved one's depth.
    for _ in 0..2 {
        let posed = pose.forward(rig);
        let from = posed.positions[dock];
        let mut asked = 0.0f32;
        let mut over = Vec3::Y;
        for &joint in &tail[1..] {
            let at = posed.positions[joint];
            let Some(below) = plane_under(ground, at, reach) else {
                continue;
            };
            let up = below.normal.normalize_or(Vec3::Y);
            let room = rig.joints[joint]
                .radius
                .min(rig.joints[joint].position.y)
                .max(0.0);
            let short = room - (at - below.position).dot(up);
            let lever = at.distance(from);
            if short > 0.0 && lever > f32::EPSILON {
                let turn = (short / lever).min(1.0).det_asin();
                if turn > asked {
                    (asked, over) = (turn, up);
                }
            }
        }
        if asked <= 0.0 {
            return;
        }
        let run = posed.positions[tip] - from;
        let Some(axis) = run.cross(over).try_normalize() else {
            return;
        };
        let above = rig.joints[dock]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        pose.rotations[dock] =
            (above.inverse() * Rot::axis_angle(axis, share * asked) * posed.rotations[dock])
                .normalize();
    }
}

/// Swings each fore elbow of a winged body on four up out of the ground under
/// it (#462), about the line from its shoulder to its wrist, so the wrist,
/// the hand and the shoulder stay where and as they stand: by as far as the
/// elbow stands nearer the ground than its own ring, the ground read as a
/// plane round it ([`plane_under`]). An elbow clear of the ground is left
/// as it is, so on the level, and wherever an emote does not fold a foreleg
/// into a slope, nothing moves.
///
/// **Swung toward the body, whichever side of the line the clip draws it**
/// (#462's fixer): the lift reaches the first point of its circle about
/// that line high enough on the side the clip draws the elbow - the nearer
/// way - except where the clip draws it within [`ELBOW_WINDOW`] of the
/// circle's lowest point on the side away from the body, where the swing is
/// handed over to the side toward it as the elbow nears that point. Swung
/// the nearer way wherever it was, an elbow the clip pointed straight down
/// flipped from one side to the other as it crossed that point: roll 9's
/// Curl facing down a 30 degree slope swung its right elbow 30 mm across
/// and turned the shoulder over it 0.63 rad in one frame at 30, 60 and 144
/// frames a second. Toward the body because that is where the clips draw
/// a folded foreleg's elbow nearly every frame it is lifted (its Stretch
/// and its Curl, on every slope); the side is the body's own, so the left
/// and the right foreleg mirror each other.
///
/// **And lifted as it comes near the ground, not only once it is in it**
/// ([`eased_lift`]): held at the ground from the frame it touched, roll 9's
/// Stretch across a 20 degree slope swung the foreleg 0.42 rad a sixtieth
/// at 60 frames a second and 0.64 at 144 (on the game's ground) as each hold
/// began and ended, against 0.09 for the clip alone.
pub(crate) fn keep_elbows_up<F>(rig: &Rig, pose: &mut Pose, ground: &F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if rig.wings.is_none() || !rig.four_legged() || !pose.fits(rig) {
        return;
    }
    #[cfg(test)]
    if ELBOWS_LEFT_OUT.with(std::cell::Cell::get) {
        return;
    }
    let hip = crate::anim::quadruped::hip_height(rig).unwrap_or(0.0);
    for limb in rig.ground_contacts() {
        if !limb.is_fore() {
            continue;
        }
        let Some(chain) = rig.limb_chain(limb) else {
            continue;
        };
        let posed = pose.forward(rig);
        let [root, mid, tip] = chain.map(|joint| posed.positions[joint]);
        let Some(below) = plane_under(ground, mid, HAND_PLANE * hip) else {
            continue;
        };
        let up = below.normal.normalize_or(Vec3::Y);
        let ring = rig.joints[chain[1]].radius;
        let short = ring - (mid - below.position).dot(up);
        let lift = eased_lift(short, ELBOW_EASE * ring);
        if lift <= 0.0 || !lift.is_finite() {
            continue;
        }
        let Some(axis) = (tip - root).try_normalize() else {
            continue;
        };
        let foot = root + axis * (mid - root).dot(axis);
        let arm = mid - foot;
        // The way round the line the elbow swings that lifts it most, and
        // square to that the way that swings it in toward the body; and
        // where round its circle, from that top, the clip draws it.
        let upright = up - axis * up.dot(axis);
        let (Some(top), reach) = (upright.try_normalize(), arm.length() * upright.length()) else {
            continue;
        };
        if reach <= f32::EPSILON {
            continue;
        }
        let side = rig.joints[chain[0]].position.x.signum();
        let shoulders = rig.joints[chain[0]]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        let mut aside = axis.cross(top);
        if aside.dot(shoulders * (Vec3::X * side)) > 0.0 {
            aside = -aside;
        }
        let at = arm.dot(aside).det_atan2(arm.dot(top));
        let off = ((arm.dot(up) + lift) / reach).clamp(-1.0, 1.0).det_acos();
        if at.abs() <= off {
            continue;
        }
        // Toward the body, or the nearer way, handed over near the bottom.
        let turn = if at >= 0.0 {
            off
        } else {
            let pi = std::f32::consts::PI;
            let window = (pi - off).clamp(f32::EPSILON, ELBOW_WINDOW);
            let t = (1.0 - (pi + at) / window).clamp(0.0, 1.0);
            -off - (std::f32::consts::TAU - 2.0 * off) * t * t * (3.0 - 2.0 * t)
        };
        let swung = top * turn.det_cos() + aside * turn.det_sin();
        let held = posed.rotations[chain[2]];
        let pole = foot + swung * rig.extent();
        crate::anim::ik::two_bone(rig, pose, chain, tip, pole);
        let above = turned_below(rig, pose, &posed, chain[0], chain[1]);
        pose.rotations[chain[2]] = (above.inverse() * held).normalize();
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
    F: Fn(Vec3) -> Option<Ground> + ?Sized,
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
    // The pad's sole as [`keep_toes_up`] leaves it after a blend: up to
    // [`SOLE_SLACK`] in the ground, which a sole drawn round its ring takes
    // in its curve (#436). Held to the floor here, no turn about the pad
    // could bring a sole already that far in out of it - one side of its
    // ring rises as the other sinks - so the toes were laid square to the
    // ground on some frames and turned about the pad on others, a bat's hind
    // toe flipping 12 to 18 mm for a frame or two on a slope.
    let slack = if phase.is_none() { SOLE_SLACK } else { 0.0 };
    rest.extend(
        sole(rig, pad)
            .into_iter()
            .map(|(offset, height)| (offset, height - slack)),
    );
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
    // Normalised (#458): `above` is a product down the leg, not quite of unit
    // length, and its inverse taken as if it were left the pad's rotation up
    // to 7e-4 off it, which a turn read raw (`Quat::angle_between`) took for
    // up to 0.07 rad a frame that alternated. Every rotation this module
    // writes back from the frame as posed is normalised alike.
    pose.rotations[pad] = (above.inverse() * turned).normalize();
}

/// The least turn on from `start`, whichever way lifts it, at which
/// `clearance` - below nought at `start` - comes back to nought.
///
/// **Where no turn within [`TOE_TURN`] brings it out, the turn that brings
/// it nearest out, taken up as that comes within [`LIFT_NEAR`] of out**
/// (#446): `start` itself while it is further in than that. Answered with
/// `start` until a turn first brought the toes out and with that turn from
/// then on, a foot lifting a millimetre in a slow step - a bat's hind foot as
/// a turn on the spot dies away - left its toes in the ground for five frames
/// and turned them an eighth of a radian in the sixth: 12 mm in one frame at
/// a rolled bat's toe tips, 28 mm a sixtieth at 144 frames a second against
/// 10 around it.
fn lift_from(clearance: &impl Fn(f32) -> f32, start: f32) -> f32 {
    let way = if clearance(start + TOE_NUDGE) >= clearance(start - TOE_NUDGE) {
        1.0
    } else {
        -1.0
    };
    #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
    let step = TOE_TURN / TOE_STEPS as f32;
    let mut inside = start;
    // Where it comes nearest out of the ground, if no turn brings it out.
    let mut nearest = (clearance(start), start);
    for index in 1..=TOE_STEPS {
        #[expect(clippy::cast_precision_loss, reason = "a handful of steps")]
        let outside = start + step * index as f32 * way;
        let there = clearance(outside);
        if there >= 0.0 {
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
        if there > nearest.0 {
            nearest = (there, outside);
        }
        inside = outside;
    }
    if nearest.1 == start {
        return start;
    }
    // About the nearest step, narrowed to where it comes nearest out.
    let (mut low, mut high) = (nearest.1 - step, nearest.1 + step);
    for _ in 0..24 {
        let (one, two) = ((2.0 * low + high) / 3.0, (low + 2.0 * high) / 3.0);
        if clearance(one) >= clearance(two) {
            high = two;
        } else {
            low = one;
        }
    }
    let best = 0.5 * (low + high);
    let near = (1.0 + clearance(best) / LIFT_NEAR).clamp(0.0, 1.0);
    start + (best - start) * (near * near * (3.0 - 2.0 * near))
}

/// How far short of out of the ground the turn that brings a foot's toes
/// nearest out of it is when [`lift_from`] begins to take it up, in metres:
/// two millimetres, [`SOLE_SLACK`]'s, so the turn is whole by the time a turn
/// can bring the toes out and nothing of it where they are deep in.
const LIFT_NEAR: f32 = 0.002;

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
