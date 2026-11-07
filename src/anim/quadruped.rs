//! How a four-legged body carries itself through a gait (#370, #378), where a
//! person's walk leans.
//!
//! A person leans into their pace; a beast's trunk is already horizontal, and
//! pitched forward it drops its withers and stands its head on its chin. What
//! a moving quadruped's trunk does instead, on the figures of
//! docs/research/research-08-quadruped-gait.md:
//!
//! - **Each girdle rides on its own feet.** At a walk a girdle vaults over its
//!   legs, highest at a leg's midstance and lowest with both down; at a trot
//!   and faster it springs, lowest at midstance and highest as it leaves. So
//!   the withers and the pelvis ride a quarter of a stride apart at a walk,
//!   together at a trot and apart again as a canter's feet part, and one trunk
//!   rising as a block is what a table does. [`Ride`].
//! - **The spine bends**, a dog's and a cat's a lot and a horse's a little:
//!   sideways once a stride, and at speed folding as the hinds reach under and
//!   stretching as they push off.
//! - **The head nods**: a horse's twice a stride at a walk, down as each fore
//!   bears, its neck swinging under a head that holds its own attitude. At
//!   speed the neck reaches forward, and the tail sways with the hips at a walk
//!   and streams out behind at a gallop.
//! - **Each leg is a pantograph** ([`stand`]): the shoulder blade turns with
//!   the forearm and the metatarsus with the femur, so a leg swings from high
//!   in the body as one long spring. A swinging leg folds at the carpus and
//!   the toes ([`articulate`]).
//!
//! **Read off the footfalls, not off the pattern's name.** The ride and the
//! fold are harmonics of a sum over the four contacts, so a walk's, a trot's
//! and a gallop's differ by when their feet are down and nothing here asks
//! which [`super::Pattern`] a gait is. **Nor which family a body is**: [`Kin`]
//! reads what the motion needs off the bones, so a cross moves between its
//! parents as it stands between them.

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec2, Vec3};

use super::gait::{
    Gait, Phase, Steps, Stride, contact_heading, contact_offset, foot_pitch, home_of, led,
    relevel_head, spine_to, stride_lead,
};
use super::ground::{Foot, Ground, solve_contact_as, solve_contact_toward};
use super::heading::Heading;
use super::ik::two_bone;
use super::pose::Pose;
use super::speed::leg_of;
use super::swim::Swum;
use crate::det::{self, DetMath, Rot};
use crate::plan::{HeadKind, Limb, Zone};
use crate::rig::{Rig, Role};

/// The duty a gait has all its flight at: a gallop's least (#370), where the
/// back folds furthest and the neck reaches furthest.
const FULL_FLIGHT: f32 = 0.27;

/// How far the back folds and stretches either way at a full gallop, in
/// radians summed along the spine: a dog's or a cat's supple back, a horse's
/// stiff one.
/// Provenance: **the literature** (#378), research-08 2.4 and 2.7: a cat's
/// spine moves through some 49 degrees galloping, a greyhound's an estimated
/// 20 to 40; a horse's thoracolumbar 9 at a canter with 6 to 7 at the
/// lumbosacral joint.
const FOLD_PAWS: f32 = 0.50;
const FOLD_HOOVES: f32 = 0.11;

/// How much of the trunk's turn the head takes back off: a dog's or a cat's
/// held steady, a horse's nodding with its neck.
/// Provenance: **tuned by render** (#370).
const STEADY_PAWS: f32 = 1.0;
const STEADY_HOOVES: f32 = 0.9;

/// How far the neck reaches forward and down at a full gallop, in radians,
/// and what share of that the head keeps rather than taking back.
/// Provenance: **tuned by render** (#370).
const REACH: f32 = 0.35;
const REACH_KEPT: f32 = 0.4;

/// How far each joint of a tail sways at a walk, in radians, and how far
/// behind the joint before it each swings, in cycles: a wave that runs out to
/// the tip.
/// Provenance: **tuned by render** (#370).
const TAIL_SWAY: f32 = 0.07;
const TAIL_LAG: f32 = 0.08;

/// How far a hanging tail rises toward the line of the back at a full
/// gallop, in radians, and how far each of its joints waves with the back.
/// Provenance: **tuned by render** (#370).
const TAIL_LIFT: f32 = 0.6;
const TAIL_WAVE: f32 = 0.06;

/// Where in its stance a leg bears most, as a share of it.
/// Provenance: **the literature**, research-08 fact 8: a trotting horse's
/// girdles are lowest at 37 to 45% of the stance, and a walking one's highest
/// at 42 to 57%.
const BEARS_AT: f32 = 0.45;

/// A girdle's whole rise and fall at a walk, a trot, a canter and a gallop, as
/// a share of the withers height: the pelvis's and the withers'.
/// Provenance: **the literature**, research-08 fact 7 and 2.7: a horse's in
/// hand, withers 2% and pelvis 4 to 5% at a walk, withers 3 to 6% and pelvis
/// 3.5 to 5.5% at a trot; a dog's by dynamic similarity. Faster than a trot is
/// **chosen** by render, the literature having no figure.
const RIDE_PELVIS: [f32; 4] = [0.045, 0.045, 0.055, 0.055];
const RIDE_WITHERS: [f32; 4] = [0.021, 0.045, 0.055, 0.065];

/// The duties those four rides are anchored at, a straight line between each.
const RIDE_DUTY: [f32; 4] = [0.60, 0.43, 0.35, 0.28];

/// The duty a girdle's ride changes from a vault to a spring at, and how far
/// either side of it the change takes. A body riding at a duty between the two
/// neither vaults nor springs, which only a change of gait passes through.
const VAULT_DUTY: f32 = 0.525;
const VAULT_BAND: f32 = 0.05;

/// The sideways bend of the back at a walk and at a trot, its whole swing from
/// one side to the other in radians between the trunk's two halves: a dog's or
/// a cat's, a horse's.
/// Provenance: **tuned by render** (#378) under research-08 2.7, which gives
/// the shape (one wave a stride, lateral at least the sagittal, a horse's a few
/// degrees) and no dog's figure.
const BEND_PAWS: f32 = 0.20;
const BEND_HOOVES: f32 = 0.05;

/// How far a walk's bend runs behind itself from the withers to the pelvis, in
/// cycles: it travels head to tail at a walk and is one bend at a trot
/// (research-08 2.7).
/// Provenance: **chosen** (#378).
const BEND_TRAVEL: f32 = 0.12;

/// The back's hollow as a girdle bears, its whole swing in radians, twice a
/// stride at a walk and a trot.
/// Provenance: **the literature**: a horse's thoracolumbar moves through about
/// 4 degrees at a walk, less at a trot (research-08 fact 9).
const HOLLOW: f32 = 0.06;

/// How far the neck swings either way from the withers twice a stride, in
/// radians, at a walk:
/// a horse's and a dog's or a cat's; and at a trot, where the head mostly rides
/// the withers.
/// Provenance: **derived** (#378). A walking horse's head rises and falls 5 to
/// 7% of its withers height against the withers' 2%, in antiphase, on a neck
/// and head about 0.7 of that height; a dog's head is carried nearer its back.
const NOD_HOOVES: f32 = 0.13;
const NOD_PAWS: f32 = 0.05;
const NOD_TROT: f32 = 0.008;

/// How many times the trunk's pitch the neck turns the other way, to take the
/// lever from the hips off the head: the hips to the head over the neck to the
/// head, about.
/// Provenance: **tuned by test** (#378), on a walking horse's antiphase.
const NECK_TAKES_PITCH: f32 = 2.2;

/// How much of the neck's nod the head takes back off, holding its attitude.
/// Provenance: **the literature**: a horse's head pitch is held small at every
/// gait (research-08 2.8).
const NOD_HELD: f32 = 0.8;

/// A leg's pantograph: how far the blade turns with the forearm, the metacarpus
/// with the forearm, and the metatarsus with the femur, each as a share.
/// Provenance: **the literature** for the coupling, research-08 fact 2,
/// parallel through the stride; the blade's share is a pony's 23 to 28 degrees
/// against a forearm's 40.
const BLADE_WITH_FOREARM: f32 = 0.45;
const CARPUS_WITH_FOREARM: f32 = 1.0;
const METATARSUS_WITH_FEMUR: f32 = 1.0;

/// How far along its blade from the top a fore leg's pivot stands.
/// Provenance: **the literature**, research-08 2.2: the scapular pivot, about a
/// third of the blade's height.
const PIVOT_ON_BLADE: f32 = 1.0 / 3.0;

/// How far behind the leg's pivot a paw passes at midstance, in radians: a fore
/// and a hind.
/// Provenance: **the literature**, research-08 2.2 and fact 3: a fore lands 19
/// to 22 degrees ahead and leaves 17 to 25 behind; a hind lands 17 to 21 ahead
/// and leaves 22 to 33 behind.
const MIDSTANCE_BEHIND_FORE: f32 = 0.03;
const MIDSTANCE_BEHIND_HIND: f32 = 0.09;

/// How much of a straight leg's length a stance may stretch to.
/// Provenance: **chosen** (#378): the 3 bones laid straight are a singularity,
/// and a stance's far end stops short of them by a hock's and a stifle's bend.
const STANCE_STRETCH: f32 = 0.975;

/// How high a hind paw lifts against a fore's, and how early each peaks (the
/// stride's `tuck`, without its raise).
/// Provenance: **the literature**, research-08 2.6: a horse's hind hoof rises
/// about half what its fore does. The hurry is **tuned by render**: a paw
/// peels up and back as it leaves, and reaches forward low.
const HIND_LIFT: f32 = 0.65;
const FORE_HURRY: f32 = 0.8;
const HIND_HURRY: f32 = 0.5;

/// How far the carpus folds at its deepest at a walk and at a trot and faster,
/// and the toes at a walk and faster, in radians.
/// Provenance: **the literature**, research-08 fact 5: a trotting horse's
/// carpus closes to 92 degrees, a shepherd's swings through 89 and its MCP
/// through 105; a flat walk's through about 54, and joint patterns are the
/// same with less amplitude (2.3).
const CARPUS_FOLD_PAWS: (f32, f32) = (1.15, 1.75);
const CARPUS_FOLD_HOOVES: (f32, f32) = (0.85, 1.5);
const TOES_FOLD_PAWS: (f32, f32) = (1.0, 1.75);
const TOES_FOLD_HOOVES: (f32, f32) = (0.75, 1.2);
/// How far a hock closes on its own in swing, past the pantograph, and the hind
/// toes. Provenance: **tuned by render** (#378); the hind paw breaks far less
/// than the fore (research-08 fact 5).
const HOCK_FOLD: f32 = 0.1;
const HIND_TOES_FOLD: f32 = 0.7;

/// How much of a fold's arc the paw goes round its wrist, rather than the wrist
/// round the paw.
/// Provenance: **tuned by render** (#378).
const FOLD_RAISES_PAWS: f32 = 0.6;
const FOLD_RAISES_HOOVES: f32 = 0.3;

/// What a body's motion reads off its bones.
///
/// **Not its family's name**, so a cross stands between its parents and a leggy
/// dog a little toward a horse: every term here is continuous in the bones.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Kin {
    /// How far toward a horse's leg this is, `0..1`: a short upper arm over a
    /// long forearm and cannon. A horse's stiff back and its nod go with it.
    hoof: f32,
    /// How tall the withers stand, in metres.
    withers: f32,
}

impl Kin {
    /// The upper arm over the forearm and metacarpus, for a dog's 0.70, a cat's
    /// 0.76 and a horse's 0.42: everything at or over 0.64 reads as paws.
    fn of(rig: &Rig) -> Self {
        let lengths = rig
            .ground_contacts()
            .into_iter()
            .find(|limb| limb.is_fore())
            .and_then(|limb| {
                let [shoulder, elbow, carpus] = rig.limb_chain(limb)?;
                let pad = *rig.in_zone(Zone::Extremity(limb)).first()?;
                let joint = |at: usize| rig.joints[at].position;
                let top = rig.joints[shoulder].parent.map_or(joint(shoulder), joint);
                Some((
                    joint(shoulder).distance(joint(elbow)),
                    joint(elbow).distance(joint(carpus)) + joint(carpus).distance(joint(pad)),
                    top.y,
                ))
            });
        let Some((upper, lower, top)) = lengths else {
            return Self {
                hoof: 0.0,
                withers: leg_of(rig),
            };
        };
        Self {
            hoof: ((0.64 - upper / lower.max(f32::EPSILON)) / 0.2).clamp(0.0, 1.0),
            withers: top / 0.92,
        }
    }

    /// `paws` on a horse-free body, `hooves` on a horse, between on a cross.
    fn of_feet(self, paws: f32, hooves: f32) -> f32 {
        paws + (hooves - paws) * self.hoof
    }
}

/// How high a four-legged body's hip joint stands at rest, in metres: the
/// length Alexander's relations are in (#378).
pub(super) fn hip_height(rig: &Rig) -> Option<f32> {
    let hind = rig
        .ground_contacts()
        .into_iter()
        .find(|limb| !limb.is_fore())?;
    let [hip, _, _] = rig.limb_chain(hind)?;
    Some(rig.joints[hip].position.y)
}

/// The point a leg swings from, at rest: the hip, or a third down the blade.
fn pivot_of(rig: &Rig, limb: Limb) -> Option<Vec3> {
    let [top, _, _] = rig.limb_chain(limb)?;
    let at = rig.joints[top].position;
    if !limb.is_fore() {
        return Some(at);
    }
    let blade = rig.joints[top].parent?;
    Some(rig.joints[blade].position.lerp(at, PIVOT_ON_BLADE))
}

/// How far forward of where its paw rests a leg's stride is centred, so the paw
/// passes [`MIDSTANCE_BEHIND_FORE`] or [`MIDSTANCE_BEHIND_HIND`] behind the
/// leg's pivot at midstance.
///
/// **A stride is about the leg's pivot, not about the paw's standing place**: a
/// dog stands with its hind paw 20% of its height behind its hip, and centred
/// on that, or on the joint above it, its legs landed 25 to 36 degrees ahead of
/// their pivots where an animal's land 16 to 22.
pub(super) fn centred(rig: &Rig, limb: Limb) -> Option<Vec3> {
    let pivot = pivot_of(rig, limb)?;
    let home = home_of(rig, limb)?;
    let behind = if limb.is_fore() {
        MIDSTANCE_BEHIND_FORE
    } else {
        MIDSTANCE_BEHIND_HIND
    };
    let height = (pivot.y - home.y).max(0.0);
    Some(Vec3::Z * (pivot.z - height * behind.det_tan() - home.z))
}

/// `stride` as `limb` takes it: a hind paw lifts lower than a fore, and each
/// peels up early and reaches forward low. A person's is the stride to the bit,
/// and so is a standing body's.
pub(super) fn limb_stride(limb: Limb, gait: &Gait, stride: &Stride) -> Stride {
    if gait.limbs.len() != 4 || gait.duty >= 1.0 {
        return *stride;
    }
    let (share, hurry) = if limb.is_fore() {
        (1.0, FORE_HURRY)
    } else {
        (HIND_LIFT, HIND_HURRY)
    };
    // A tuck raises the arc as it hurries it: the height is taken back off.
    Stride {
        lift: stride.lift * share / (1.0 + hurry),
        tuck: hurry,
        ..*stride
    }
}

/// Where a joint's turn stands in the world, by the turns above it: a pose's
/// [`Pose::forward`] for one joint, without the whole rig's positions.
fn turned_at(rig: &Rig, pose: &Pose, joint: usize) -> Quat {
    let mut turn = pose.rotations[joint];
    let mut at = rig.joints[joint].parent;
    while let Some(parent) = at {
        turn = pose.rotations[parent] * turn;
        at = rig.joints[parent].parent;
    }
    turn
}

/// Stands a beast's leg in its pantograph for its paw to land on `goal`, as far
/// down as a leg's solve leaves to the rest of it: `chain` is what
/// [`Rig::limb_chain`] hands it (#378).
///
/// **The forearm carries the blade and the metacarpus, and the femur the
/// metatarsus** (research-08 fact 2), so a leg swings as a pair of parallel
/// bones at each end of a third, from the top of the blade or the hip. Stood
/// upright instead, as they were before, the 75 to 120 degrees of a stride's
/// swing were all the thigh's or the upper arm's, and the paw at either end of a
/// stance hung off a compass's leg. `fold` turns the metapodial further, for a
/// swinging leg; a stance's is nothing.
///
/// **Solved, not iterated with the leg.** The two-bone solve below it turns the
/// very bone this reads, and near a leg's full reach a millimetre at the ankle
/// turns the thigh by more than the metapodial gives back, so alternating the
/// two diverged at the ends of a stance. So the coupling is found first, in the
/// side plane: the one swing of the driving bone that, with the blade and the
/// metapodial turned by it, the two-bone leg itself takes. With the metapodial
/// then fixed in the world the leg's solve lands in a single pass.
pub(super) fn stand(rig: &Rig, pose: &mut Pose, chain: [usize; 3], goal: Vec3, fold: f32) {
    let [top, elbow, ankle] = chain;
    let Zone::UpperLimb(limb) = rig.joints[top].zone else {
        return;
    };
    let Some(&pad) = rig.in_zone(Zone::Extremity(limb)).first() else {
        return;
    };
    let rest = |joint: usize| rig.joints[joint].position;
    // `(z, y)` of a vector: the plane the swing is in, its angle as a turn
    // about X carries it.
    let angle = |v: Vec3| v.z.det_atan2(v.y);
    let (upper, lower) = (
        rest(top).distance(rest(elbow)),
        rest(elbow).distance(rest(ankle)),
    );
    let hang = rest(pad) - rest(ankle);
    let thigh_rest = angle(rest(elbow) - rest(top));
    let shank_rest = angle(rest(ankle) - rest(elbow));
    let line_rest = angle(rest(ankle) - rest(top));
    let bend = wrapped(thigh_rest - line_rest).signum();
    let fore = limb.is_fore();
    let posed = pose.forward(rig);
    // A fore's root turns on its blade with the forearm; a hind's stays.
    let blade = rig.joints[top]
        .parent
        .filter(|&blade| fore && rig.joints[blade].zone == Zone::UpperLimb(limb));
    let root_of = |swing: f32| match blade {
        Some(blade) => {
            let girdle = rig.joints[blade]
                .parent
                .map_or(Quat::IDENTITY, |girdle| posed.rotations[girdle]);
            posed.positions[blade]
                + girdle * (Rot::x(BLADE_WITH_FOREARM * swing) * (rest(top) - rest(blade)))
        }
        None => posed.positions[top],
    };
    let share = if fore {
        CARPUS_WITH_FOREARM
    } else {
        METATARSUS_WITH_FEMUR
    };
    // The swing the leg takes, given the swing asked of it, less that swing:
    // falling as the swing rises, with its one root the coupling.
    let error = |swing: f32| {
        let root = root_of(swing);
        let wrist = goal - Rot::x(share * swing + fold) * hang;
        let span = wrist - root;
        let asked = span.length();
        let asked = if fold == 0.0 {
            asked
        } else {
            within_reach(asked, upper + lower)
        };
        let reach = asked.clamp((upper - lower).abs() + 1.0e-4, (upper + lower) * 0.999);
        let cosine = (upper * upper + reach * reach - lower * lower) / (2.0 * upper * reach);
        let thigh = angle(span) + bend * cosine.clamp(-1.0, 1.0).det_acos();
        let taken = if fore {
            let knee = Vec3::new(
                wrist.x,
                root.y + upper * thigh.det_cos(),
                root.z + upper * thigh.det_sin(),
            );
            wrapped(angle(wrist - knee) - shank_rest)
        } else {
            wrapped(thigh - thigh_rest)
        };
        taken - swing
    };
    // Regula falsi, with the Illinois halving so a bent curve cannot pin one
    // end: within a tenth of a degree in a handful of calls.
    let (mut low, mut high) = (-SWING_LIMIT, SWING_LIMIT);
    let (mut at_low, mut at_high) = (error(low), error(high));
    let mut swing = 0.0;
    if at_low > 0.0 && at_high < 0.0 {
        let mut left = 0.0f32;
        for _ in 0..SWING_PASSES {
            swing = (low * at_high - high * at_low) / (at_high - at_low);
            let at = error(swing);
            left = at;
            if at.abs() < SWING_CLOSE {
                break;
            }
            if at > 0.0 {
                low = swing;
                at_low = at;
                at_high *= 0.5;
            } else {
                high = swing;
                at_high = at;
                at_low *= 0.5;
            }
        }
        // A sign change is not a root where it is a tear.
        if left.abs() > SWING_TORN {
            swing = short_of_tear(error);
        }
    } else {
        swing = if at_low <= 0.0 { low } else { high };
    }
    if let Some(blade) = blade {
        pose.rotations[blade] = Rot::x(BLADE_WITH_FOREARM * swing);
    }
    let carried = turned_at(rig, pose, elbow);
    pose.rotations[ankle] = carried.inverse() * Rot::x(share * swing + fold);
}

/// The swing that answers `error` when a search settled on its tear (#399): the
/// first root up from [`SWING_LIMIT`] below, or where there is none short of the
/// tear, the swing that comes nearest one.
///
/// **A tear looks like a root.** A wrist come up by its shoulder asks for a forearm
/// or a thigh turned half way round, where `wrapped` jumps a whole turn, and the
/// error falls through nought there as it does through a root. On a steep grade
/// the search settled on either from one frame to the next, and an elbow read y
/// 759, 58, 711 mm on three of them. Below the tear the real root rises to meet
/// a second, rising crossing, and the two vanish together; the least error goes
/// on from where they met, so a leg losing its root does not jump.
fn short_of_tear(error: impl Fn(f32) -> f32) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "a few dozen steps")]
    let width = 2.0 * SWING_LIMIT / TEAR_STEPS as f32;
    let (mut from, mut at_from) = (-SWING_LIMIT, error(-SWING_LIMIT));
    let mut nearest = (from, at_from);
    for index in 1..=TEAR_STEPS {
        #[expect(clippy::cast_precision_loss, reason = "a few dozen steps")]
        let to = -SWING_LIMIT + width * index as f32;
        let at_to = error(to);
        if at_to <= 0.0 {
            if at_from - at_to < PI {
                let (mut low, mut high) = (from, to);
                for _ in 0..TEAR_NARROWING {
                    let middle = 0.5 * (low + high);
                    if error(middle) > 0.0 {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                return 0.5 * (low + high);
            }
            break;
        }
        if at_to < nearest.1 {
            nearest = (to, at_to);
        }
        from = to;
        at_from = at_to;
    }
    let (mut low, mut high) = (
        (nearest.0 - width).max(-SWING_LIMIT),
        (nearest.0 + width).min(SWING_LIMIT),
    );
    for _ in 0..TEAR_NARROWING {
        let (one, two) = ((2.0 * low + high) / 3.0, (low + 2.0 * high) / 3.0);
        // Past the tear reads as far below any root: nearest is above nought.
        let read = |swing: f32| {
            let at = error(swing);
            if at > 0.0 { at } else { f32::MAX }
        };
        if read(one) <= read(two) {
            high = two;
        } else {
            low = one;
        }
    }
    0.5 * (low + high)
}

/// Where a beast's leg's middle joint bends toward with its root at `root` and
/// its tip going to `tip`: square to the line between them in the side plane, on
/// the side the leg bends at rest.
///
/// **Not [`Rig::bend_pole`]'s one point in space.** A swinging fore's wrist, its
/// paw folded back, sweeps up behind its shoulder and across the line to that
/// point, and the elbow flips to the other side of the leg: 200 to 380 mm in one
/// sample at a gallop (it did before the fold as well, 280 on a horse).
pub(super) fn side_pole(rig: &Rig, chain: [usize; 3], root: Vec3, tip: Vec3) -> Vec3 {
    let rest = |joint: usize| rig.joints[joint].position;
    let angle = |v: Vec3| v.z.det_atan2(v.y);
    let bend =
        wrapped(angle(rest(chain[1]) - rest(chain[0])) - angle(rest(chain[2]) - rest(chain[0])));
    let square = angle(tip - root) + bend.signum() * PI * 0.5;
    root + Vec3::new(0.0, square.det_cos(), square.det_sin()) * rig.extent()
}

/// An angle folded into `-PI..PI`.
fn wrapped(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// The furthest a leg's driving bone swings from its rest, in radians, and how
/// closely and in how many calls the coupling is found.
/// Provenance: **chosen** (#378). A tenth of a degree is under what a skin shows.
const SWING_LIMIT: f32 = 1.7;
const SWING_CLOSE: f32 = 2.0e-3;
const SWING_PASSES: usize = 8;

/// How much error a search may leave and still have found a root, in radians,
/// and in how many steps of the swing's range and how many narrowings of one
/// [`short_of_tear`] looks for the real answer.
/// Provenance: **chosen** (#399). A root the search has finished with reads a few
/// hundredths and a tear on a gentle grade well over this, but on a steep one the
/// two run together, with thousands of searches stopping between .2 and 1.4, so
/// this is only a threshold. It clears a downhill grade of .4 and does not settle
/// an uphill one, which is why #399 stays open.
const SWING_TORN: f32 = 0.5;
const TEAR_STEPS: usize = 32;
const TEAR_NARROWING: usize = 12;

/// A girdle's ride: the first two harmonics of the load on its legs, each as a
/// turning point in the plane, so a sum over the legs is a sum of points.
#[derive(Clone, Copy, Debug)]
struct Ride {
    /// The load on the hinds' two harmonics and the fores'.
    hinds: [Vec2; 2],
    fores: [Vec2; 2],
    /// The whole rise and fall of each, in metres, positive for a spring.
    pelvis: f32,
    withers: f32,
}

impl Ride {
    fn of(gait: &Gait, kin: Kin) -> Self {
        let duty = gait.duty.clamp(0.0, 1.0);
        // A half sine's Fourier coefficients, `d` long and centred where it
        // bears. At `2kd = 1` the ratio is its limit.
        let weight = |k: f32| {
            let across = 2.0 * k * duty;
            let denominator = 1.0 - across * across;
            let ratio = if denominator.abs() < 1.0e-3 {
                PI / 4.0
            } else {
                (PI * k * duty).det_cos() / denominator
            };
            4.0 * duty / PI * ratio
        };
        let harmonic = |fore: bool, k: f32| {
            let mut sum = Vec2::ZERO;
            for (&limb, &offset) in gait.limbs.iter().zip(&gait.offsets) {
                if limb.is_fore() == fore {
                    sum += turned(k * (offset + duty * BEARS_AT));
                }
            }
            sum * weight(k)
        };
        let spring = ((VAULT_DUTY - duty) / VAULT_BAND).clamp(-1.0, 1.0);
        let anchored = |table: [f32; 4]| {
            let along =
                |from: usize| (RIDE_DUTY[from] - duty) / (RIDE_DUTY[from] - RIDE_DUTY[from + 1]);
            if duty >= RIDE_DUTY[0] {
                table[0]
            } else if duty >= RIDE_DUTY[1] {
                table[0] + (table[1] - table[0]) * along(0)
            } else if duty >= RIDE_DUTY[2] {
                table[1] + (table[2] - table[1]) * along(1)
            } else {
                table[2] + (table[3] - table[2]) * along(2).min(1.0)
            }
        };
        Self {
            hinds: [harmonic(false, 1.0), harmonic(false, 2.0)],
            fores: [harmonic(true, 1.0), harmonic(true, 2.0)],
            pelvis: anchored(RIDE_PELVIS) * kin.withers * spring,
            withers: anchored(RIDE_WITHERS) * kin.withers * spring,
        }
    }

    /// How far above their mean the pelvis and the withers ride at `cycle`, in
    /// metres.
    fn at(&self, cycle: f32) -> (f32, f32) {
        let one = turned(cycle);
        let two = turned(2.0 * cycle);
        let wave = |sum: [Vec2; 2]| {
            let peak = sum[0].length() + sum[1].length();
            if peak <= f32::EPSILON {
                0.0
            } else {
                (sum[0].dot(one) + sum[1].dot(two)) / peak
            }
        };
        (
            -0.5 * self.pelvis * wave(self.hinds),
            -0.5 * self.withers * wave(self.fores),
        )
    }

    /// The fores' load's second harmonic at `cycle`, in `-1..=1`: what a nod and
    /// a hollow back keep time to.
    fn twice(&self, cycle: f32) -> f32 {
        let sum = self.fores[1];
        if sum.length() <= f32::EPSILON {
            return 0.0;
        }
        sum.dot(turned(2.0 * cycle)) / sum.length()
    }

    /// How far above their mean either girdle ever rides, in metres.
    fn height(&self) -> f32 {
        0.5 * self.pelvis.abs().max(self.withers.abs())
    }
}

/// How far a folding leg's wrist lies from the leg's top, given how far it is asked
/// to: as asked until the leg is nearly straight, then easing toward its full
/// length rather than running into it (#378).
///
/// **A two-bone solve has a square-root singularity at full reach.** The fold
/// raises a hoof by the swing it folds through and throws the wrist out with it,
/// past the leg's length on a horse, where the elbow swings through the last of
/// its angle in one sample: a pop of 186 mm in a frame, every stride.
fn within_reach(asked: f32, length: f32) -> f32 {
    let knee = FOLD_REACH_KNEE * length;
    if asked <= knee {
        return asked;
    }
    let room = (FOLD_REACH_MOST - FOLD_REACH_KNEE) * length;
    let over = (asked - knee) / room;
    knee + room * over / (1.0 + over)
}

/// How much of a leg's length a folding wrist reaches before [`within_reach`] eases
/// it, and the most it ever reaches. Provenance: **tuned against the elbow's pop**
/// (#378), which fell from 130 to 24 mm a sample on the worst rolled horse.
const FOLD_REACH_KNEE: f32 = 0.92;
const FOLD_REACH_MOST: f32 = 0.997;

/// How far a beast's body must sink for the foot of `limb`, standing `offset` from
/// where it rests, to be in reach: the stride's own measure of a leg (#378), which
/// counts the metatarsus. [`super::gait::sink_needed`] reads a thigh and a shank,
/// and past those it asks for the whole height of the leg, so a stopping horse
/// plunged .74 m in a fifth of a second.
pub(super) fn sink_toward(rig: &Rig, limb: Limb, offset: Vec3) -> Option<f32> {
    let home = rig.joints[*rig.in_zone(Zone::Extremity(limb)).first()?].position;
    Some(sink_for(rig, limb, home + offset)?.max(0.0))
}

/// How far a leg's pivot must sink for its paw to reach `paw`, in metres:
/// negative where it has slack to spare.
fn sink_for(rig: &Rig, limb: Limb, paw: Vec3) -> Option<f32> {
    let [top, mid, tip] = rig.limb_chain(limb)?;
    let pad = *rig.in_zone(Zone::Extremity(limb)).first()?;
    let joint = |at: usize| rig.joints[at].position;
    let length = (joint(top).distance(joint(mid))
        + joint(mid).distance(joint(tip))
        + joint(tip).distance(joint(pad)))
        * STANCE_STRETCH;
    let along = paw - joint(top);
    // The blade's swing takes up a share of a fore's reach.
    let share = if limb.is_fore() { 0.8 } else { 1.0 };
    let flat = Vec2::new(along.x, along.z * share);
    Some(-along.y - (length * length - flat.length_squared()).max(0.0).sqrt())
}

/// How deep a four-legged body holds itself through `gait`, in metres: the
/// least depth at which every paw reaches every stance moment with the girdle
/// above it riding where it rides then (#378).
///
/// **One depth, and the bob is the [`Ride`]'s.** Sinking moment by moment for
/// whichever leg asked most jolted a trot 14 mm a frame as one leg's ask handed
/// to another's, put every girdle's low point at the ends of a stride whatever
/// the gait, and rode the withers and pelvis as a block.
pub(super) fn depth(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    depth_rising(rig, gait, stride, 0.0, None)
}

/// [`depth`] for a stride whose paws stand on ground rising `rise` per metre
/// ahead of where each rests (#399), as they do under a trunk ridden over its
/// girdles' ground on a slope, for the legs of one girdle, the fores where
/// `fores` says so, or all four: a paw at the back of its stance up a slope is
/// the lower by the slope's share of its reach behind, so the whole stride asks
/// more of the leg, the same through it. Sunk only for the paw furthest down
/// at each moment, a horse up a .4 grade sank its pelvis 300 mm through a
/// stance and rose that far in the frame its lowest paw left the ground.
pub(super) fn depth_rising(
    rig: &Rig,
    gait: &Gait,
    stride: &Stride,
    rise: f32,
    fores: Option<bool>,
) -> f32 {
    let ride = Ride::of(gait, Kin::of(rig));
    let mut deepest = 0.0f32;
    for (index, &limb) in gait.limbs.iter().enumerate() {
        if fores.is_some_and(|fores| fores != limb.is_fore()) {
            continue;
        }
        let (Some(home), offset) = (home_of(rig, limb), gait.offsets[index]) else {
            continue;
        };
        let lead = stride_lead(rig, limb, gait);
        for share in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let reached = led(
                contact_offset(led(home, lead), stride, Phase::Stance(share)),
                lead,
            );
            let (pelvis, withers) = ride.at(offset + gait.duty * share);
            let rides = if limb.is_fore() { withers } else { pelvis };
            let reached = if rise == 0.0 {
                reached
            } else {
                reached + Vec3::Y * (rise * reached.z)
            };
            if let Some(sink) = sink_for(rig, limb, home + reached) {
                deepest = deepest.max(sink + rides);
            }
        }
    }
    deepest
}

/// [`depth`] for the moment `cycle` with its stance paws where they really are,
/// on real ground or held by a ledger: a slope's or a change of speed's ask, and
/// on a flat steady walk no more than [`depth`].
pub(super) fn depth_at(
    rig: &Rig,
    gait: &Gait,
    cycle: f32,
    standing: impl Iterator<Item = (Limb, Vec3)>,
) -> f32 {
    let (pelvis, withers) = Ride::of(gait, Kin::of(rig)).at(cycle);
    standing
        .filter_map(|(limb, paw)| {
            Some(sink_for(rig, limb, paw)? + if limb.is_fore() { withers } else { pelvis })
        })
        .fold(0.0, f32::max)
}

/// [`depth_at`] for one girdle's stance paws on ground that is not level (#399),
/// each measured from its own girdle's ground, which the trunk is ridden over,
/// each with how far through its stance it is: a paw leaving the ground asks
/// less of the sink the nearer it is to leaving, by [`LEAVING`], and nothing as
/// it lifts, and in a stride that goes nowhere or turns, `settling` of `0..=1`,
/// or a fore's in any stride, a paw landing nothing as it lands
/// and more as it settles, all by as much as the ground is [`Girdles::steep`]:
/// on the level, as much as [`depth_at`] asks of it. Asked whole to the last, a
/// girdle sank all through a stance for the paw at its back, lowest on a slope,
/// and rose by all of it in the frame that paw left the ground: a horse's pelvis
/// 300 mm up a .4 grade.
pub(super) fn depth_leaving(
    rig: &Rig,
    gait: &Gait,
    cycle: f32,
    standing: impl Iterator<Item = (Limb, Vec3, f32)>,
    girdles: Girdles,
    settling: f32,
) -> f32 {
    let (pelvis, withers) = Ride::of(gait, Kin::of(rig)).at(cycle);
    standing
        .filter_map(|(limb, paw, through)| {
            let paw = paw - Vec3::Y * girdles.under(limb);
            let rides = if limb.is_fore() { withers } else { pelvis };
            let sink = sink_for(rig, limb, paw)? + rides;
            let left = ((1.0 - through) / LEAVING).clamp(0.0, 1.0);
            let fade = left * left * (3.0 - 2.0 * left);
            // And, in a stride that goes nowhere or turns, comes into it as it
            // lands: asked whole the frame it came down, a pivot's paw landing
            // down a slope tipped a horse's trunk seven degrees in that frame.
            // A straight stride plans for its landing hind paw on the slope,
            // and easing that in as well changed where a walk breaking into a
            // trot threw it (#412).
            //
            // **A fore's at every stride** (#413, #412): a fore's stride is
            // planned on the level, so nothing of a slope is planned for its
            // landing paw whichever way the stride goes, and a dog walking
            // down a 25 degree slope tipped its trunk 4.4 degrees in the frame
            // each forepaw came down, trotting down it 6.6 and a cat 8.2.
            // Eased in only at a walk while a walk's change to a trot threw a
            // hind paw past what a frame allows (#412), which carrying every
            // foot across the change ended.
            let landed = (through / LEAVING).clamp(0.0, 1.0);
            let landed = landed * landed * (3.0 - 2.0 * landed);
            let settling = if limb.is_fore() { 1.0 } else { settling };
            let fade = fade * (1.0 - settling * (1.0 - landed));
            Some(sink * (1.0 - girdles.steep() * (1.0 - fade)))
        })
        .fold(0.0, f32::max)
}

/// How short a stride is that goes nowhere, as a share of the hip's height: a
/// pivot's, whose paws land with nothing planned for them. Provenance:
/// **chosen** (#413): a quarter, under the shortest walk's and over a pivot's
/// none.
pub(super) const STILL_STRIDE: f32 = 0.25;

/// How far a stride's heading turns across a stance, in radians, for its
/// landing paws to have nothing planned for them, as a pivot's: the stride's
/// plan is a straight one's. Provenance: **chosen** (#413): about nine
/// degrees, which a body turning where it stands or walking a tight curve
/// turns and a straight stride, at none, never does.
pub(super) const TURNING_STRIDE: f32 = 0.15;

/// How late in its stance a paw on a slope begins to ask less of the sink, and
/// how early in it it has come to ask all of it, as a share of the stance.
/// Provenance: **chosen** (#399): the last quarter, where a paw is rolling off
/// its toes, and the first, where it is taking the weight.
const LEAVING: f32 = 0.25;

/// The deepest a girdle of a four-legged body rides in `gait`: [`depth`] and
/// the [`Ride`] below it, the planning figure `gait::crouch_for` hands out.
pub(super) fn deepest(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    depth(rig, gait, stride) + Ride::of(gait, Kin::of(rig)).height()
}

/// Carries a four-legged body's trunk, head and tail through `gait` at
/// `cycle`.
///
/// Call where a person's walk leans: after [`super::gait::step`] and before
/// the footing tail. The legs go back where `steps` sent them, a stance's and
/// a swing's both: nothing below a girdle moves as the trunk does but the bones
/// joining them. `head_level: false` leaves the head to ride the trunk, the
/// ablation [`super::gait::Walk::head_level`] names. A body standing on any
/// other number of legs, or a gait that never lifts one, is left alone.
pub(super) fn carry(
    rig: &Rig,
    pose: &mut Pose,
    gait: &Gait,
    steps: &mut Steps,
    cycle: f32,
    head_level: bool,
) {
    if !pose.fits(rig) || gait.duty >= 1.0 || gait.limbs.len() != 4 {
        return;
    }
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
        return;
    };
    let Some(girdle) = rig.joints[neck].parent else {
        return;
    };
    let kin = Kin::of(rig);
    // How much of a gallop this is, `0..1`, off the duty: nothing until a
    // foot's stance is shorter than its swing.
    let flight = ((0.5 - gait.duty) / (0.5 - FULL_FLIGHT)).clamp(0.0, 1.0);
    let ride = Ride::of(gait, kin);
    let (pelvis, withers) = ride.at(cycle);
    let trunk = rig.joints[girdle].position - rig.joints[root].position;
    let pitch = (withers - pelvis).det_atan2(trunk.length().max(f32::EPSILON));
    let fold = harmonics(gait, cycle) * flight * kin.of_feet(FOLD_PAWS, FOLD_HOOVES);
    // A bat's and a glider's bound folds and stretches its back the more
    // (#424): see `winged::ground::BOUND_FLEX`.
    let fold = if super::winged::bounds(rig) {
        fold * super::winged::ground::BOUND_FLEX
    } else {
        fold
    };
    let twice = ride.twice(cycle);

    // The trunk's first duty is to leave a foot on the ground where the step
    // stood it, which the turns below must not move. Each stance leg keeps the
    // attitude the step gave it and is solved back to its place, or a fore at
    // the back of its stance swung further back and up as the forehand tipped,
    // and the plant dropped the body to reach it.
    let held = pose.forward(rig).rotations;
    let stepped = pose.rotations.clone();

    // The girdles' ride: the trunk's height at its hips, and its pitch to the
    // withers about them.
    let tops: Vec<Vec3> = rig
        .ground_contacts()
        .into_iter()
        .filter(|limb| !limb.is_fore())
        .filter_map(|limb| rig.limb_chain(limb))
        .map(|[top, _, _]| rig.joints[top].position)
        .collect();
    if !tops.is_empty() {
        #[expect(clippy::cast_precision_loss, reason = "two legs")]
        let hips = tops.iter().sum::<Vec3>() / tops.len() as f32;
        let lever = hips - rig.joints[root].position;
        let before = pose.rotations[root] * lever;
        pose.rotations[root] *= Rot::x(-pitch);
        pose.translation += before - pose.rotations[root] * lever;
    }
    pose.translation.y += pelvis;

    // The spine's bends, spread along it and heaviest at its middle, where
    // the thoracolumbar junction bends most (research-08 2.7), with the trunk
    // turned back by half of each so the two halves bend about its middle.
    // Sideways once a stride, one bend at a trot and a wave from the withers
    // to the pelvis at a walk, dying away as the body takes flight.
    let spine = spine_to(rig, girdle);
    let hind = gait
        .limbs
        .iter()
        .position(|limb| !limb.is_fore() && limb.is_left())
        .map_or(0.0, |index| gait.offsets[index]);
    let side = kin.of_feet(BEND_PAWS, BEND_HOOVES) * 0.5 * (1.0 - flight);
    let walking = ((gait.duty - 0.45) / 0.15).clamp(0.0, 1.0);
    let hollow = -HOLLOW * 0.5 * twice * (1.0 - flight);
    let mut across = 0.0f32;
    let mut along = 0.0f32;
    #[expect(clippy::cast_precision_loss, reason = "a spine is a handful of joints")]
    {
        let count = spine.len().max(1) as f32;
        let total: f32 = (0..spine.len())
            .map(|rank| ((rank as f32 + 0.5) / count * PI).det_sin())
            .sum();
        for (rank, &joint) in spine.iter().enumerate() {
            let at = (rank as f32 + 0.5) / count;
            let share = (at * PI).det_sin() / total.max(f32::EPSILON);
            let late = (1.0 - at) * BEND_TRAVEL * walking;
            let yaw = side * share * (TAU * (cycle - hind - late)).det_cos();
            let bow = (fold + hollow) * share;
            pose.rotations[joint] *= Rot::y(yaw) * Rot::x(bow);
            across += yaw;
            along += bow;
        }
    }
    pose.rotations[root] *= Rot::y(-0.5 * across) * Rot::x(-0.5 * along);

    let mut inherited = Rot::x(-pitch) * Rot::y(0.5 * across) * Rot::x(0.5 * along);
    if head_level {
        inherited = det::slerp(
            Quat::IDENTITY,
            inherited,
            kin.of_feet(STEADY_PAWS, STEADY_HOOVES),
        );
        relevel_head(rig, pose, inherited);
    }
    // At speed the neck reaches forward and the head goes down with it, less
    // than the neck does. **From the neck's own joint, not the withers**: the
    // fores hang from the withers, and turned there a horse's shoulders swung
    // 10 cm under its neck, further than a planted leg could follow. And twice
    // a stride it nods, most at a walk and on a horse, down as each fore bears.
    // The trunk's pitch would carry the head on the whole lever from the hips,
    // so the neck takes that back first: the head rides the trunk's height, not
    // the far end of a see-saw.
    let nod = kin.of_feet(NOD_PAWS, NOD_HOOVES) * walking + NOD_TROT * (1.0 - walking);
    let nodding = nod * twice * (1.0 - flight);
    let swing = pitch * NECK_TAKES_PITCH;
    let reach = REACH * flight;
    // A nod swings the whole neck, so from the withers: the neck's own joint
    // stands halfway up it, and takes four times the angle for the same drop.
    pose.rotations[girdle] *= Rot::x(nodding);
    pose.rotations[neck] *= Rot::x(reach + swing);
    if let Some(&head) = rig.in_zone(Zone::Head).first() {
        pose.rotations[head] *= Rot::x(-reach * (1.0 - REACH_KEPT) - (swing + nodding) * NOD_HELD);
    }

    // And the legs go back. A leg in the air goes back to where the step sent
    // it too: a girdle's ride is no more than a foot going where it was going.
    let turned_now = pose.forward(rig).rotations;
    for (index, &limb) in gait.limbs.iter().enumerate() {
        let (Some(chain), Some(home)) = (rig.limb_chain(limb), home_of(rig, limb)) else {
            continue;
        };
        let Some(&(_, offset, _)) = steps.asked.iter().find(|&&(of, _, _)| of == limb) else {
            continue;
        };
        let top = chain[0];
        let standing = gait.phase(index, cycle).is_stance();
        if standing && let Some(parent) = rig.joints[top].parent {
            pose.rotations[top] = turned_now[parent].inverse() * held[parent] * stepped[top];
        }
        let kept = chain.map(|joint| pose.rotations[joint]);
        // A stance pinned beyond its reach would have the plant drop the whole
        // body to make up the difference. A swing past it reaches as far as it
        // goes, and hunting back to where it was jolted it 240 mm in a sample.
        if !solve_contact_as(rig, pose, limb, home + offset, Foot::Striding) && standing {
            for (joint, rotation) in chain.into_iter().zip(kept) {
                pose.rotations[joint] = rotation;
            }
        }
    }

    // Where the swinging paws now stand is what a clip laid over the walk would
    // move them from: short of their goals, as a leg at its reach is, they would
    // otherwise read as a clip's, and fold and unfold between one frame and the
    // next.
    let placed = pose.forward(rig);
    for (limb, at) in &mut steps.placed {
        if let Some(&paw) = rig.in_zone(Zone::Extremity(*limb)).first() {
            *at = placed.positions[paw];
        }
    }

    swish(rig, pose, gait, cycle, flight);
}

/// Which of a gait's swinging legs are still where [`super::gait::step`] left
/// them, and so its own to fold: a clip laid over the walk owns what it moved.
/// None on any other body than four-legged.
pub(super) fn owned_swings(
    rig: &Rig,
    pose: &Pose,
    gait: &Gait,
    steps: &Steps,
    kept: f32,
) -> Vec<Limb> {
    if gait.limbs.len() != 4 || gait.duty >= 1.0 || steps.placed.is_empty() {
        return Vec::new();
    }
    let posed = pose.forward(rig);
    steps
        .placed
        .iter()
        .filter(|&&(limb, was)| {
            rig.in_zone(Zone::Extremity(limb))
                .first()
                .is_some_and(|&foot| posed.positions[foot].distance(was) <= kept)
        })
        .map(|&(limb, _)| limb)
        .collect()
}

/// A swinging leg's fold through its swing, `0` at both ends and `1` a little
/// before its middle, leaving and arriving without a jolt.
fn fold_along(t: f32) -> f32 {
    let up = (t.clamp(0.0, 1.0).det_powf(0.8) * PI)
        .det_sin()
        .clamp(0.0, 1.0);
    up * up * (3.0 - 2.0 * up)
}

/// The last of a beast's walk: folds the legs in the air and lays the toes on
/// the ground, the feet where the plant and the roll left them (#378).
///
/// **The pad does not move**, so nothing in the ground's account of a paw
/// changes, and every leg is continuous across its landing and its lift-off: a
/// fold is nothing at either end of a swing, and the toes' attitude is the
/// roll's from the stance into the swing.
///
/// `owned` names the swinging legs a clip has not taken: see [`owned_swings`].
pub(super) fn articulate<F>(
    rig: &Rig,
    pose: &mut Pose,
    gait: &Gait,
    stride: &Stride,
    cycle: f32,
    owned: &[Limb],
    ground: &F,
) where
    F: Fn(Vec3) -> Option<Ground>,
{
    if !pose.fits(rig) || gait.limbs.len() != 4 || gait.duty >= 1.0 {
        return;
    }
    let kin = Kin::of(rig);
    let trotting = ((0.58 - gait.duty) / 0.15).clamp(0.0, 1.0);
    let at_speed = |(walk, faster): (f32, f32)| walk + (faster - walk) * trotting;
    let along = Heading::toward(stride.direction).along();
    let fold = if super::winged::kinship(rig).is_some() {
        super::winged::ground::fold_share(rig, gait, stride)
    } else {
        1.0
    };
    for (index, &limb) in gait.limbs.iter().enumerate() {
        let Some(chain) = rig.limb_chain(limb) else {
            continue;
        };
        let extremity = rig.in_zone(Zone::Extremity(limb));
        let [pad, ..] = extremity[..] else {
            continue;
        };
        let phase = gait.phase(index, cycle);
        let swinging = match phase {
            Phase::Swing(t) => fold_along(t),
            Phase::Stance(_) => 0.0,
        };
        // A small winged body's fast stride folds its paws the less (#424).
        let swinging = if super::winged::kinship(rig).is_some() {
            swinging * fold
        } else {
            swinging
        };
        let (joint_fold, toe_fold) = if limb.is_fore() {
            (
                at_speed((
                    kin.of_feet(CARPUS_FOLD_PAWS.0, CARPUS_FOLD_HOOVES.0),
                    kin.of_feet(CARPUS_FOLD_PAWS.1, CARPUS_FOLD_HOOVES.1),
                )),
                at_speed((
                    kin.of_feet(TOES_FOLD_PAWS.0, TOES_FOLD_HOOVES.0),
                    kin.of_feet(TOES_FOLD_PAWS.1, TOES_FOLD_HOOVES.1),
                )),
            )
        } else {
            (-HOCK_FOLD, HIND_TOES_FOLD)
        };
        // A crawling bat's paw is its wrist, and is not folded (#424).
        if swinging > 0.0 && owned.contains(&limb) && !super::winged::crawls(rig) {
            // The paw folds up and back about the wrist, the wrist staying near
            // where the leg was going: left low with its wrist thrown out ahead,
            // a folding fore straightened its elbow and flicked through it.
            let fold = joint_fold * swinging;
            let hang = rig.joints[pad]
                .position
                .distance(rig.joints[chain[2]].position);
            let folded = Vec3::new(0.0, hang * (1.0 - fold.det_cos()), -hang * fold.det_sin())
                * kin.of_feet(FOLD_RAISES_PAWS, FOLD_RAISES_HOOVES);
            let goal = pose.forward(rig).positions[pad] + folded;
            stand(rig, pose, chain, goal, fold);
            let posed = pose.forward(rig);
            let hang = posed.positions[chain[2]] - posed.positions[pad];
            let root = posed.positions[chain[0]];
            let joint = |at: usize| rig.joints[at].position;
            let length = joint(chain[0]).distance(joint(chain[1]))
                + joint(chain[1]).distance(joint(chain[2]));
            let span = goal + hang - root;
            let wrist = root + span.normalize_or_zero() * within_reach(span.length(), length);
            two_bone(rig, pose, chain, wrist, side_pole(rig, chain, root, wrist));
        }
        // The toes: flat on the ground as the leg stands over them, turned to
        // hold their bearing on a turn and peeled by the roll; in the air they
        // go from that to the fold, with the metapodial's.
        let at = pose.forward(rig).positions[pad];
        let up = ground(at)
            .map_or(Vec3::Y, |ground| ground.normal)
            .normalize_or(Vec3::Y);
        let roll = foot_pitch(phase).min(0.0) * along;
        let flat = Rot::axis_angle(up, contact_heading(stride, phase))
            * det::from_rotation_arc(Vec3::Y, up);
        let axis = (flat * Vec3::Z)
            .cross(up)
            .try_normalize()
            .unwrap_or(Vec3::X);
        let ankle = turned_at(rig, pose, chain[2]);
        let laid = ankle.inverse() * Rot::axis_angle(axis, roll) * flat;
        // About the side's axis the long way if need be, not a slerp's short
        // way: a fold past the half turn from flat flipped the toes through 230
        // mm in a sample.
        let flexed = 2.0 * laid.x.det_atan2(laid.w);
        // A crawling bat's foot trails its toes a sole's length behind the
        // pad, and folded as a paw's toes are it flicked up through 55 mm a
        // frame as it lifted (#424): it is carried flat.
        let swinging = if super::winged::crawls(rig) {
            0.0
        } else {
            swinging
        };
        pose.rotations[pad] = laid * Rot::x(swinging * (toe_fold - wrapped(flexed)));
    }
}

/// How high the ground stands under each girdle of a four-legged body (#399):
/// the hips' and the shoulders', and how far ahead of the one the other is
/// read, all in the frame the body is posed in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Girdles {
    /// How high the ground stands under the hind feet and under the fore.
    pub(super) hips: f32,
    pub(super) shoulders: f32,
    /// How far ahead of the hind feet the fore are read.
    between: f32,
    /// How much higher the ground stands under the left feet than under the
    /// right, and how far apart across the body they are read.
    across: f32,
    apart: f32,
}

impl Girdles {
    /// How high the ground stands under the girdle `limb` hangs from.
    pub(super) fn under(self, limb: Limb) -> f32 {
        if limb.is_fore() {
            self.shoulders
        } else {
            self.hips
        }
    }

    /// How far the ground rises per metre ahead, from the hips' to the
    /// shoulders'.
    pub(super) fn rise(self) -> f32 {
        (self.shoulders - self.hips) / self.between
    }

    /// How steep the ground under them is, `0..=1`: nothing on the level, and
    /// all of it from [`STEEP`] of pitch and roll together (#399). What a
    /// slope's own answers ask is taken in by this, so that near level ground,
    /// a flat heightmap's millimetres or a plane a hair off level, is ridden
    /// as the level is, and going from one to the other moves nothing at once.
    pub(super) fn steep(self) -> f32 {
        let t = ((self.pitch().abs() + self.roll().abs()) / STEEP).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// These girdles with their middle height taken out (#414): the turn the
    /// ground asks of the trunk, and none of its height.
    pub(super) fn turned(self) -> Self {
        let middle = (self.hips + self.shoulders) * 0.5;
        Self {
            hips: self.hips - middle,
            shoulders: self.shoulders - middle,
            ..self
        }
    }

    /// The plane these stand for under `at`, in the frame the body is posed
    /// in: its height there and its up (#414). `None` for a body that does not
    /// stand on four legs.
    pub(super) fn plane_under(self, rig: &Rig, at: Vec3) -> Option<Ground> {
        let [behind, _, left, right] = girdle_spacing(rig)?;
        let wide = left - right;
        let across = if wide.abs() > f32::EPSILON {
            self.across / wide
        } else {
            0.0
        };
        let along = self.rise();
        let height = self.hips + along * (at.z - behind) + across * (at.x - (left + right) * 0.5);
        Some(Ground {
            position: Vec3::new(at.x, height, at.z),
            normal: Vec3::new(-across, 1.0, -along).normalize(),
        })
    }

    /// The level ground read where these are: nothing under either girdle.
    fn level(self) -> Self {
        Self {
            hips: 0.0,
            shoulders: 0.0,
            across: 0.0,
            ..self
        }
    }

    /// These girdles `share` of the way from the level (#435): every height
    /// they read taken by `share`, so a ride over them turns and raises the
    /// trunk that share of the way, and all of it at `1`, to the bit. What a
    /// winged body's landing rides as its feet come onto sloped ground.
    pub(super) fn scaled(self, share: f32) -> Self {
        Self {
            hips: self.hips * share,
            shoulders: self.shoulders * share,
            across: self.across * share,
            ..self
        }
    }

    /// These girdles with the shoulders sunk `by` further than the hips.
    pub(super) fn shoulders_sunk(self, by: f32) -> Self {
        Self {
            shoulders: self.shoulders - by,
            ..self
        }
    }

    /// The pitch the ground asks of the trunk, positive with the shoulders the
    /// higher.
    pub(super) fn pitch(self) -> f32 {
        (self.shoulders - self.hips).det_atan2(self.between)
    }

    /// The roll the ground asks of the trunk, positive with its left the
    /// higher.
    pub(super) fn roll(self) -> f32 {
        self.across.det_atan2(self.apart)
    }

    /// The whole turn the ground asks of the trunk: [`Self::pitch`] about the
    /// body's across, then [`Self::roll`] about its length.
    pub(super) fn turn(self) -> Quat {
        Rot::x(-self.pitch()) * Rot::z(self.roll())
    }
}

/// The ground under each girdle of `rig`, or `None` where it is level, or the
/// body does not stand on four legs (#399).
///
/// **Read over the feet's footprints, not at a point.** Each girdle stands over
/// its own pair of feet, so its ground is the mean of what `ground` answers over
/// a cross [`GIRDLE_SPREAD`] of the hip's height wide about where each of them
/// rests: a body on a slope has its hips over lower ground than its shoulders,
/// or higher, and a heightmap's step under one foot raises its girdle by a
/// tenth of the step at each of the points that crosses it, not all at once.
/// Nothing answering under a girdle leaves it on the other's ground.
pub(super) fn girdle_ground<F>(rig: &Rig, ground: &F) -> Option<Girdles>
where
    F: Fn(Vec3) -> Option<Ground>,
{
    if !rig.four_legged() {
        return None;
    }
    let spread = GIRDLE_SPREAD * hip_height(rig)?;
    let around = [
        Vec3::ZERO,
        Vec3::X * spread,
        -Vec3::X * spread,
        Vec3::Z * spread,
        -Vec3::Z * spread,
    ];
    let mut level = true;
    // Height and count of what answered, and where along the body it rests:
    // behind and ahead, and on the right and the left.
    let mut read = [(0.0f32, 0usize, 0.0f32); 2];
    let mut sides = [(0.0f32, 0usize, 0.0f32); 2];
    for limb in rig.ground_contacts() {
        let home = home_of(rig, limb)?;
        let slot = &mut read[usize::from(limb.is_fore())];
        slot.2 += home.z * 0.5;
        let side = &mut sides[usize::from(limb.is_left())];
        side.2 += home.x * 0.5;
        for &off in &around {
            if let Some(below) = ground(home + off) {
                level &= below.position.y == 0.0 && below.normal == Vec3::Y;
                slot.0 += below.position.y;
                slot.1 += 1;
                side.0 += below.position.y;
                side.1 += 1;
            }
        }
    }
    if level {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "ten points")]
    let mean = |(sum, count, _): (f32, usize, f32)| (count > 0).then(|| sum / count as f32);
    let (hips, shoulders) = match (mean(read[0]), mean(read[1])) {
        (Some(hips), Some(shoulders)) => (hips, shoulders),
        (Some(either), None) | (None, Some(either)) => (either, either),
        (None, None) => return None,
    };
    let across = match (mean(sides[0]), mean(sides[1])) {
        (Some(right), Some(left)) => left - right,
        _ => 0.0,
    };
    Some(Girdles {
        hips,
        shoulders,
        between: (read[1].2 - read[0].2).max(f32::EPSILON),
        across,
        apart: (sides[1].2 - sides[0].2).abs().max(f32::EPSILON),
    })
}

/// Where [`girdle_ground`] reads `rig`'s girdles, in the frame it is posed in:
/// along it the hind feet's and the fore feet's, and across it the left feet's
/// and the right's, or `None` for a body that does not stand on four (#413).
pub(super) fn girdle_spacing(rig: &Rig) -> Option<[f32; 4]> {
    if !rig.four_legged() {
        return None;
    }
    let mut spacing = [0.0f32; 4];
    for limb in rig.ground_contacts() {
        let home = home_of(rig, limb)?;
        spacing[usize::from(limb.is_fore())] += home.z * 0.5;
        spacing[2 + usize::from(!limb.is_left())] += home.x * 0.5;
    }
    Some(spacing)
}

/// Stands a four-legged body ridden over `girdles` on its feet (#413): each
/// foot solved back over where it rests, onto the ground under it, and planted,
/// by as much as the ground is [`Girdles::steep`], so ground near the level
/// leaves the pose as the ride left it. What a body standing in its rest pose
/// needs and the idle does for itself: the ride turns the legs with the trunk,
/// and on a slope that carries every foot off the ground under it.
pub(super) fn stand_on<F>(rig: &Rig, pose: &mut Pose, girdles: Girdles, ground: F)
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let steep = girdles.steep();
    if steep <= 0.0 || !rig.four_legged() {
        return;
    }
    let ridden = pose.clone();
    let contacts = rig.ground_contacts();
    for &limb in &contacts {
        let Some(home) = home_of(rig, limb) else {
            continue;
        };
        let goal = ground(home).map_or(home, |below| home + Vec3::Y * below.position.y);
        super::ground::solve_contact(rig, pose, limb, goal);
    }
    super::ground::plant_feet_of(
        rig,
        pose,
        &contacts,
        &ground,
        &super::ground::FootingConfig::default(),
    );
    if steep < 1.0 {
        for (rotation, was) in pose.rotations.iter_mut().zip(&ridden.rotations) {
            *rotation = det::slerp(*was, *rotation, steep);
        }
        pose.translation = ridden.translation.lerp(pose.translation, steep);
    }
}

/// The ground a four-legged body's trunk rides this frame (#413): `read`, what
/// [`girdle_ground`] answers now, or as far toward it from `ridden`, what the
/// trunk rode the last frame, as [`RIDE_TURN`] and [`RIDE_RISE`] allow in
/// `delta` seconds, and on the frame a body comes down from the air,
/// `landed`, its height at once. Either `None` is the level.
///
/// **A consumer's ground can change source in a frame**: Overlands goes between
/// its terrain and a level floor at the edge of a pad or a deck, on a lost
/// physics step and at a landing, and taken whole each switch on a slope turned
/// the trunk by the slope in that frame, a dog's by 25 degrees. Followed, it
/// turns over a few frames. The rates are far above anything ground the body
/// walks over asks, so everywhere else, and on the level, what is ridden is
/// `read` itself, to the bit.
///
/// **What it costs: the feet go with the trunk.** While the trunk follows, every
/// state is handed the ground laid on what is ridden ([`Shift`]), so each foot
/// stands on that plane and not on the ground really under it. Through a 25
/// degree switch a standing dog's feet stood up to 132 mm off the real ground in
/// the switch frame, a horse's 351, and more than 10 mm for eight or nine frames
/// after; walking, up to 261 and 501 mm. Stood on the real ground instead, the
/// feet would snap by the whole switch in its frame, the very jump this is here
/// to spread, and reach from a trunk that had not yet turned.
pub(super) fn follow(
    rig: &Rig,
    ridden: Option<Girdles>,
    read: Option<Girdles>,
    delta: f32,
    landed: bool,
) -> Option<Girdles> {
    if !rig.four_legged() {
        return None;
    }
    let (from, to) = match (ridden, read) {
        (None, None) => return None,
        (Some(from), None) => (from, from.level()),
        (None, Some(to)) => (to.level(), to),
        (Some(from), Some(to)) => (from, to),
    };
    let Some(hip) = hip_height(rig).filter(|hip| *hip > 0.0) else {
        return read;
    };
    let rise = RIDE_RISE * (super::speed::GRAVITY * hip).sqrt() * delta;
    let turn = RIDE_TURN * delta;
    let middle = |girdles: Girdles| (girdles.hips + girdles.shoulders) * 0.5;
    let lift = |girdles: Girdles| girdles.shoulders - girdles.hips;
    let toward = super::rest::toward;
    // **Coming down, the height is the ground's at once** (#414): what was
    // ridden last on the ground was a ground falling away under a body about
    // to leave it, followed as it went, and a beast stepping off a 0.6 m ledge
    // landed with its paws 300 mm under the ground for nine frames. Its pitch
    // and its roll are followed still; the landing is blended into.
    let height = if landed {
        middle(to)
    } else {
        toward(middle(from), middle(to), rise)
    };
    let lifted = toward(lift(from), lift(to), turn * to.between);
    let across = toward(from.across, to.across, turn * to.apart);
    if height == middle(to) && lifted == lift(to) && across == to.across {
        return read;
    }
    Some(Girdles {
        hips: height - lifted * 0.5,
        shoulders: height + lifted * 0.5,
        across,
        ..to
    })
}

/// How fast a four-legged body's trunk follows the pitch and the roll of the
/// ground under its girdles, in radians a second, and how fast its height, at
/// a share of `sqrt(g H)` of its hip's height `H`: see [`follow`]. The turn is
/// not scaled by the body, because what turns the ground under a body fastest
/// short of a switch is the body turning on a slope, which goes at the turn
/// whatever its size. Provenance: **measured** (#413): over the sweeps' ground
/// the most asked was 1.8 radians a second of pitch and 1.4 of roll, a dog
/// crossing the step on rolling ground, 0.8 turning where it stands on a 25
/// degree slope, and 0.31 of `sqrt(g H)` of height, the same step, all at
/// sixty frames a second; a step's read jumps in the frame a point crosses it,
/// so faster it asks more and is followed over a frame or two. A 25 degree
/// switch is followed over about nine frames at sixty a second, 2.9 degrees a
/// frame.
const RIDE_TURN: f32 = 3.0;
const RIDE_RISE: f32 = 1.0;

/// How the ground a body is given differs from the ground read under it
/// (#413): a plane, laid in the frame the body is posed in, which added to what
/// is read makes [`girdle_ground`] answer what the trunk rides, so its feet are
/// stood where its trunk is and not on ground it has yet to turn onto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Shift {
    /// How much higher the ground is laid under the hips' read.
    height: f32,
    /// How much it rises per metre ahead, and per metre to the left.
    along: f32,
    across: f32,
    /// Where the hips' read is along the body, and the middle across it.
    behind: f32,
    middle: f32,
}

impl Shift {
    /// The plane that lays the ground `read` stands for on `ridden`'s, or
    /// `None` where the two are the same, or the body does not stand on four.
    pub(super) fn between(
        rig: &Rig,
        read: Option<Girdles>,
        ridden: Option<Girdles>,
    ) -> Option<Self> {
        if !rig.four_legged() {
            return None;
        }
        let (read, ridden) = match (read, ridden) {
            (None, None) => return None,
            (Some(read), None) => (read, read.level()),
            (None, Some(ridden)) => (ridden.level(), ridden),
            (Some(read), Some(ridden)) => (read, ridden),
        };
        if read == ridden {
            return None;
        }
        let [behind, _, left, right] = girdle_spacing(rig)?;
        let wide = left - right;
        if wide.abs() <= f32::EPSILON {
            return None;
        }
        Some(Self {
            height: ridden.hips - read.hips,
            along: ridden.rise() - read.rise(),
            across: (ridden.across - read.across) / wide,
            behind,
            middle: (left + right) * 0.5,
        })
    }

    /// `below` laid on the plane: raised by it where it is, and its normal
    /// turned by its slope.
    pub(super) fn apply(self, below: Ground) -> Ground {
        let at = below.position;
        let raised =
            self.height + self.along * (at.z - self.behind) + self.across * (at.x - self.middle);
        let normal = if below.normal.y > f32::EPSILON {
            let slope = Vec2::new(below.normal.x, below.normal.z) / -below.normal.y
                + Vec2::new(self.across, self.along);
            Vec3::new(-slope.x, 1.0, -slope.y).normalize()
        } else {
            below.normal
        };
        Ground {
            position: Vec3::new(at.x, at.y + raised, at.z),
            normal,
        }
    }
}

/// How steep, pitch and roll together in radians, ground is where a beast
/// rides it wholly as a slope (#399). Provenance: **chosen** (#399): about six
/// degrees, under the gentlest grade a stride was seen to pop on, .15, and far
/// over a flat heightmap's few thousandths, where splitting the crouch per
/// girdle at no pitch at all pitched a horse's trunk and swung its tail 95 mm
/// off where level ground holds it.
const STEEP: f32 = 0.1;

/// How wide the cross each foot's ground is read over is, as a share of the
/// hip's height. Provenance: **chosen** (#399): about a foot's own length either
/// way, which spreads a heightmap's step under it over a quarter of a stride.
const GIRDLE_SPREAD: f32 = 0.15;

/// Rides a four-legged body's trunk over the ground under its girdles (#399):
/// its hips raised to theirs and its shoulders to theirs, turned about the hips,
/// and the turn taken back off the head, which looks where it was looking.
///
/// **Each girdle over its own feet.** Without it the trunk stood level over the
/// ground under its middle and the stride sank the whole body for whichever paw
/// was lowest: up a .3 grade a horse's pelvis sank 380 mm through each hind's
/// stance and came back up in the frame the hind left the ground, and its
/// forelegs, folded up under a trunk sunk for the hinds, came by their shoulders
/// until the leg's solve lost its root, a paw going 636 mm in a frame.
pub(super) fn ride_ground(rig: &Rig, pose: &mut Pose, girdles: Girdles) {
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let tops: Vec<Vec3> = rig
        .ground_contacts()
        .into_iter()
        .filter(|limb| !limb.is_fore())
        .filter_map(|limb| rig.limb_chain(limb))
        .map(|[top, _, _]| rig.joints[top].position)
        .collect();
    if tops.is_empty() {
        return;
    }
    #[expect(clippy::cast_precision_loss, reason = "two legs")]
    let hips = tops.iter().sum::<Vec3>() / tops.len() as f32;
    let lever = hips - rig.joints[root].position;
    let turn = girdles.turn();
    let before = pose.rotations[root] * lever;
    pose.rotations[root] *= turn;
    pose.translation += before - pose.rotations[root] * lever;
    pose.translation.y += girdles.hips;
    relevel_head(rig, pose, turn);
}

/// The joints a beast's ears hang from (#370): each the parent of a spring at
/// an ear's tip, on a beast's head. None on any other body, whatever hangs
/// off it: a chain a consumer gives a person's head is not an ear.
pub(super) fn ear_roots(rig: &Rig) -> impl Iterator<Item = usize> + '_ {
    let joints = if rig.anatomy.head == HeadKind::Beast {
        rig.len()
    } else {
        0
    };
    (0..joints)
        .filter(|&joint| rig.joints[joint].role == Role::Spring)
        .filter_map(|tip| rig.joints[tip].parent)
        .filter(|&root| {
            rig.joints[root]
                .parent
                .is_some_and(|head| rig.joints[head].zone == Zone::Head)
        })
}

/// Turns a four-legged body's trunk `pitch` radians nose up about its
/// shoulders, so its hindquarters sink, or nose down about its hips, so its
/// forehand does: whichever end moves goes down, and the legs under it bend.
///
/// **About where the legs that stay meet the body**, the midpoint of their
/// top joints: turned about the withers above them, a sitting dog's shoulders
/// swung forward under its trunk and its fore paws 118 mm off where they
/// stood.
///
/// What [`super::clip::Target::Pitch`] turns a sit about.
pub(super) fn sink_end(rig: &Rig, pose: &mut Pose, pitch: f32) {
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let tops: Vec<Vec3> = rig
        .ground_contacts()
        .into_iter()
        .filter(|limb| limb.is_fore() == (pitch > 0.0))
        .filter_map(|limb| rig.limb_chain(limb))
        .map(|[top, _, _]| rig.joints[top].position)
        .collect();
    if tops.is_empty() {
        return;
    }
    #[expect(clippy::cast_precision_loss, reason = "two legs")]
    let pivot = tops.iter().sum::<Vec3>() / tops.len() as f32;
    let lever = pivot - rig.joints[root].position;
    let before = pose.rotations[root] * lever;
    pose.rotations[root] *= Rot::x(-pitch);
    pose.translation += before - pose.rotations[root] * lever;
}

/// [`sink_end`], with the legs under the girdle that stays still pointing
/// where they did: a sitting beast's fores stay straight under its withers
/// and a bowing one's hinds under its hips, while the legs under the end
/// that sinks go down with it for whatever holds their feet to fold.
pub(super) fn sink_end_standing(rig: &Rig, pose: &mut Pose, pitch: f32) {
    let held = pose.forward(rig).rotations;
    sink_end(rig, pose, pitch);
    let turned = pose.forward(rig).rotations;
    for limb in rig.ground_contacts() {
        // Nose up turns about the withers, whose legs are the fores.
        if limb.is_fore() != (pitch > 0.0) {
            continue;
        }
        let Some([top, _, _]) = rig.limb_chain(limb) else {
            continue;
        };
        if let Some(parent) = rig.joints[top].parent {
            pose.rotations[top] = turned[parent].inverse() * held[parent] * pose.rotations[top];
        }
    }
}

/// How a four-legged body swims (#370): a paddle, not a crawl.
///
/// A person swims by lying down along the water and windmilling; a beast is
/// already lying along it, and swims as it trots - its legs in diagonal
/// pairs, each paw going round a loop under its chest or belly, forward high
/// and back low, the power stroke along the bottom. Its trunk rides a little
/// nose up, its rump lower at a tread than under way, its head held level
/// and high, and its tail trails straight as a rudder.
///
/// `effort` is [`super::swim`]'s axis, `0` treading to `1` under way: it
/// opens the loops and levels the trunk. Hands back what it did to the trunk
/// as a [`Swum`], whose roll a paddle never has.
pub(super) fn paddle(rig: &Rig, pose: &mut Pose, cycle: f32, effort: f32) -> Swum {
    let effort = effort.clamp(0.0, 1.0);
    let mut swum = Swum {
        effort,
        ..Swum::default()
    };
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return swum;
    };
    let leg = leg_of(rig);
    // Nose up, the rump low: more at a tread, where nothing carries the body
    // forward to plane it level.
    swum.pitch = PADDLE_PITCH * (1.0 - 0.5 * effort);
    pose.rotations[root] *= Rot::x(-swum.pitch);
    swum.surge = PADDLE_SURGE * leg * (cycle * 2.0 * TAU).det_sin();
    pose.translation.y += swum.surge;
    relevel_head(rig, pose, Rot::x(-swum.pitch));

    // Each paw round its loop, in the frame the trunk now has.
    let posed = pose.forward(rig);
    let (centre, turned) = (posed.positions[root], posed.rotations[root]);
    let carried = |at: Vec3| centre + turned * (at - rig.joints[root].position);
    let reach = (PADDLE_REACH.0 + (PADDLE_REACH.1 - PADDLE_REACH.0) * effort) * leg;
    let depth = (PADDLE_DEPTH.0 + (PADDLE_DEPTH.1 - PADDLE_DEPTH.0) * effort) * leg;
    for limb in rig.ground_contacts() {
        let (Some(&paw), Some(pole)) = (
            rig.in_zone(Zone::Extremity(limb)).first(),
            rig.bend_pole(limb),
        ) else {
            continue;
        };
        // In diagonal pairs, as a trot's.
        let phase = if limb.is_fore() == limb.is_left() {
            0.0
        } else {
            0.5
        };
        let around = TAU * (cycle - phase);
        // Drawn up under the body, the fores a little ahead and the hinds
        // behind where they stand.
        let tucked = Vec3::new(
            0.0,
            PADDLE_TUCK * leg,
            if limb.is_fore() { 0.1 } else { -0.1 } * leg,
        );
        let round = Vec3::new(0.0, -depth * around.det_sin(), reach * around.det_cos());
        let goal = carried(rig.joints[paw].position + tucked + round);
        solve_contact_toward(rig, pose, limb, goal, carried(pole));
    }

    // The tail out behind, a little side to side with the stroke.
    let tail = rig.tail();
    if let (Some(&dock), Some(&tip)) = (tail.first(), tail.last()) {
        let run = rig.joints[tip].position - rig.joints[dock].position;
        let hangs = (-run.y).det_atan2(-run.z);
        if hangs > 0.0 {
            pose.rotations[dock] *= Rot::x(hangs.min(TAIL_LIFT));
        }
        for &joint in &tail {
            pose.rotations[joint] *= Rot::y(PADDLE_RUDDER * (cycle * TAU).det_sin());
        }
    }
    swum
}

/// How far a paddling beast rides nose up at a tread, in radians; it halves
/// under way.
/// Provenance: **tuned by render** (#370).
const PADDLE_PITCH: f32 = 0.2;

/// A paddle's loop, in the leg's reach: how far each paw swings forward and
/// back of its loop's centre, and how far up and down, at a tread and under
/// way.
/// Provenance: **tuned by render** (#370).
const PADDLE_REACH: (f32, f32) = (0.18, 0.32);
const PADDLE_DEPTH: (f32, f32) = (0.12, 0.2);

/// How far a paddling beast's legs are drawn up under it, in the leg's reach.
/// Provenance: **tuned by render** (#370).
const PADDLE_TUCK: f32 = 0.2;

/// How far a paddling body bobs with its strokes, twice a cycle, in the leg's
/// reach.
/// Provenance: **tuned by render** (#370).
const PADDLE_SURGE: f32 = 0.02;

/// How far each tail joint swings either way as it steers, in radians.
/// Provenance: **tuned by render** (#370).
const PADDLE_RUDDER: f32 = 0.03;

/// The back's fold at `cycle`, in `-1..=1`.
///
/// Positive **gathered** - the hinds reaching under the body as they land, the
/// fores back under it as they lift - and negative stretched, and the first
/// harmonic of that reach summed over the contacts, so it is as large as the
/// hinds and the fores part.
fn harmonics(gait: &Gait, cycle: f32) -> f32 {
    let duty = gait.duty;
    let mut gather = Vec2::ZERO;
    for (&limb, &offset) in gait.limbs.iter().zip(&gait.offsets) {
        gather += turned(if limb.is_fore() {
            offset + duty
        } else {
            offset
        });
    }
    gather.dot(turned(cycle)) * 0.25
}

/// The point a moment `0..1` of the cycle sits at on the unit circle.
fn turned(moment: f32) -> Vec2 {
    let angle = TAU * moment;
    Vec2::new(angle.det_cos(), angle.det_sin())
}

/// Sways the tail with the hips and lifts it out behind at speed.
///
/// The sway follows the left hind, one swing each way a cycle, each joint a
/// little behind the one before so the wave runs out to the tip; it dies away
/// as the body takes flight. At speed a tail carried below the line of the
/// back rises toward it - a tail already carried higher keeps its own - and
/// waves with the fold, behind it.
fn swish(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32, flight: f32) {
    let tail = rig.tail();
    let (Some(&dock), Some(&tip)) = (tail.first(), tail.last()) else {
        return;
    };
    let hind = gait
        .limbs
        .iter()
        .position(|limb| !limb.is_fore() && limb.is_left())
        .map_or(0.0, |index| gait.offsets[index]);
    #[expect(clippy::cast_precision_loss, reason = "a tail is a handful of joints")]
    for (rank, &joint) in tail.iter().enumerate() {
        let late = cycle - rank as f32 * TAIL_LAG;
        let sway = TAIL_SWAY * (1.0 - flight) * (TAU * (late - hind)).det_sin();
        let wave = TAIL_WAVE * flight * harmonics(gait, late);
        pose.rotations[joint] *= Rot::y(sway) * Rot::x(wave);
    }
    // How far below the horizontal behind the tail's chord hangs: a turn
    // about X carries a tail pointing back up.
    let run = rig.joints[tip].position - rig.joints[dock].position;
    let hangs = (-run.y).det_atan2(-run.z);
    if hangs > 0.0 {
        pose.rotations[dock] *= Rot::x(hangs.min(TAIL_LIFT) * flight);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::gait::Walk;
    use crate::anim::ground::Ground;
    use crate::anim::pose::Posed;
    use crate::anim::{Lead, Pattern, Speed};
    use crate::plan::{BodyPlan, QuadrupedParams};

    #[test]
    fn a_tear_is_not_taken_for_a_root() {
        // #399. Falling through nought at 0.8 and torn at 1.2, it finds the root.
        let torn = |swing: f32| {
            if swing < 1.2 {
                0.8 - swing
            } else {
                0.8 - swing - std::f32::consts::TAU
            }
        };
        assert!((short_of_tear(torn) - 0.8).abs() < 1.0e-3);
        // No root short of the tear: the swing that comes nearest, a parabola's
        // lowest at 0.3, off nought by a tenth.
        let rootless = |swing: f32| {
            if swing < 1.2 {
                0.1 + (swing - 0.3) * (swing - 0.3)
            } else {
                swing - 1.2 - std::f32::consts::PI
            }
        };
        assert!((short_of_tear(rootless) - 0.3).abs() < 5.0e-3);
    }

    fn quadruped() -> Rig {
        beast(QuadrupedParams::default())
    }

    fn beast(params: QuadrupedParams) -> Rig {
        Rig::from_skeleton(&params.skeleton(&crate::Composites::default())).expect("rigs")
    }

    /// One moment of a cycle, walked through every stage at a Froude
    /// number: the posed joints and which feet were down.
    struct Moment {
        posed: Posed,
        hinds: usize,
        fores: usize,
    }

    /// `rig` at `froude`, posed by the whole walk at `frames` moments of
    /// one cycle.
    fn moments(rig: &Rig, froude: f32, head_level: bool, frames: usize) -> Vec<Moment> {
        let speed = Speed::new(rig, Speed::from_froude(froude).metres_per_second(rig));
        let gait = speed.gait(rig);
        let stride = speed.stride(rig);
        (0..frames)
            .map(|frame| {
                let cycle = frame as f32 / frames as f32;
                let mut pose = Pose::rest(rig);
                Walk {
                    head_level,
                    ..Walk::at(cycle)
                }
                .drive(rig, &mut pose, &gait, &stride, |at| {
                    Some(Ground::level(Vec3::new(at.x, 0.0, at.z)))
                });
                let down = |fore: bool| {
                    gait.limbs
                        .iter()
                        .enumerate()
                        .filter(|&(index, limb)| {
                            limb.is_fore() == fore && gait.phase(index, cycle).is_stance()
                        })
                        .count()
                };
                Moment {
                    posed: pose.forward(rig),
                    hinds: down(false),
                    fores: down(true),
                }
            })
            .collect()
    }

    /// The pelvis, the loin and the withers: the girdle the neck hangs
    /// from, which is not the chest's first joint on a six-ring trunk (#375).
    fn trunk(rig: &Rig) -> [usize; 3] {
        let first = |zone: Zone| *rig.in_zone(zone).first().expect("a beast's trunk");
        let withers = rig.joints[first(Zone::Neck)].parent.expect("a girdle");
        [first(Zone::Pelvis), first(Zone::Abdomen), withers]
    }

    /// The trunk's pitch nose up, in degrees, and how far the loin stands
    /// above the line from the pelvis to the withers, in metres.
    fn line(rig: &Rig, at: &[Vec3]) -> (f32, f32) {
        let [hips, loin, withers] = trunk(rig);
        let run = at[withers] - at[hips];
        let along = (at[loin] - at[hips]).dot(run) / run.length_squared();
        (
            run.y.atan2(run.z).to_degrees(),
            at[loin].y - (at[hips].y + run.y * along),
        )
    }

    /// The head's pitch nose up, in degrees, from its first joint to its
    /// last.
    fn head_pitch(rig: &Rig, at: &[Vec3]) -> f32 {
        let head = rig.in_zone(Zone::Head);
        let run = at[*head.last().expect("a head")] - at[head[0]];
        run.y.atan2(run.z).to_degrees()
    }

    fn mean(values: impl Iterator<Item = f32>) -> f32 {
        let (sum, count) =
            values.fold((0.0, 0.0), |(sum, count), value| (sum + value, count + 1.0));
        sum / count
    }

    #[test]
    fn a_beast_carries_its_back_level_at_every_pace() {
        // A person leans into their pace; a beast pitched forward drops its
        // withers. Its trunk rocks about its line at a canter and a gallop
        // and leaves it where it stood on average, at every pace.
        let rig = quadruped();
        let rest = line(&rig, &Pose::rest(&rig).forward(&rig).positions).0;
        for froude in [0.3, 1.4, 3.2, 7.0] {
            let pitch = mean(
                moments(&rig, froude, true, 40)
                    .iter()
                    .map(|moment| line(&rig, &moment.posed.positions).0),
            );
            assert!(
                (pitch - rest).abs() < 1.0,
                "at Fr {froude} the trunk is pitched {:+.1} degrees from where it stands",
                pitch - rest
            );
        }
    }

    #[test]
    fn a_beast_rocks_onto_whichever_end_bears_it() {
        // Its hindquarters settle while only its hinds are down and its
        // forehand while only its fores are, so the trunk pitches nose up on
        // the hinds and down on the fores - read against the same body with
        // its posture off, which carries neither.
        for froude in [3.2, 7.0] {
            let rig = quadruped();
            let [hips, _, withers] = trunk(&rig);
            let carried = moments(&rig, froude, true, 40);
            let plain = {
                let speed = Speed::new(&rig, Speed::from_froude(froude).metres_per_second(&rig));
                let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));
                (0..40)
                    .map(|frame| {
                        let mut pose = Pose::rest(&rig);
                        Walk {
                            posture: false,
                            ..Walk::at(frame as f32 / 40.0)
                        }
                        .drive(&rig, &mut pose, &gait, &stride, |at| {
                            Some(Ground::level(Vec3::new(at.x, 0.0, at.z)))
                        });
                        pose.forward(&rig).positions
                    })
                    .collect::<Vec<_>>()
            };
            // How far the posture sank a joint, on average, over the moments
            // only one pair bears the body.
            let sunk = |joint: usize, hinds: bool| {
                mean(
                    carried
                        .iter()
                        .zip(&plain)
                        .filter(|(moment, _)| {
                            if hinds {
                                moment.fores == 0 && moment.hinds > 0
                            } else {
                                moment.hinds == 0 && moment.fores > 0
                            }
                        })
                        .map(|(moment, plain)| plain[joint].y - moment.posed.positions[joint].y),
                )
            };
            let bearing = |hinds: bool| (sunk(hips, hinds), sunk(withers, hinds));
            let (on_hinds, on_fores) = (bearing(true), bearing(false));
            assert!(
                on_hinds.0 > on_hinds.1 + 0.005 && on_fores.1 > on_fores.0 + 0.005,
                "at Fr {froude}, on its hinds the hips sank {:.0} mm and the withers {:.0}; on \
                 its fores the hips {:.0} and the withers {:.0}",
                on_hinds.0 * 1000.0,
                on_hinds.1 * 1000.0,
                on_fores.0 * 1000.0,
                on_fores.1 * 1000.0,
            );
        }
    }

    #[test]
    fn a_galloping_back_arches_gathered_and_hollows_stretched() {
        // Two moments in the air a stride: gathered, the fores just lifted
        // and the hinds about to land, every foot under the body; and
        // stretched, the hinds just pushed off and the fores reaching.
        let rig = quadruped();
        let moments = moments(&rig, 7.0, true, 40);
        // Which pair last stood, for each moment in the air.
        let mut flights = Vec::new();
        for (index, moment) in moments.iter().enumerate() {
            if moment.hinds + moment.fores > 0 {
                continue;
            }
            let before = (1..moments.len())
                .map(|back| &moments[(index + moments.len() - back) % moments.len()])
                .find(|moment| moment.hinds + moment.fores > 0)
                .expect("a gallop lands");
            let arch = line(&rig, &moment.posed.positions).1;
            flights.push((before.fores > 0, arch));
        }
        let arched = |gathered: bool| {
            mean(
                flights
                    .iter()
                    .filter(|&&(fores, _)| fores == gathered)
                    .map(|&(_, arch)| arch),
            )
        };
        let (gathered, stretched) = (arched(true), arched(false));
        assert!(
            gathered > stretched + 0.03,
            "the loin stood {:.0} mm above the trunk's line gathered and {:.0} mm stretched",
            gathered * 1000.0,
            stretched * 1000.0
        );
    }

    #[test]
    fn a_galloping_head_holds_steadier_than_its_trunk() {
        // The ablation is the control: the same gallop with the head left to
        // ride the trunk.
        let rig = quadruped();
        let swing = |head_level: bool| {
            let pitches: Vec<f32> = moments(&rig, 7.0, head_level, 40)
                .iter()
                .map(|moment| head_pitch(&rig, &moment.posed.positions))
                .collect();
            let low = pitches.iter().copied().fold(f32::INFINITY, f32::min);
            let high = pitches.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            high - low
        };
        let (steady, riding) = (swing(true), swing(false));
        assert!(
            steady < riding * 0.5,
            "the head swung {steady:.1} degrees held and {riding:.1} riding the trunk"
        );
    }

    #[test]
    fn a_hanging_tail_streams_out_at_a_gallop_and_a_raised_one_stays_raised() {
        let tip = |carriage: f32, froude: f32| {
            let rig = beast(QuadrupedParams {
                tail_carriage: carriage,
                ..QuadrupedParams::default()
            });
            let tip = *rig.in_zone(Zone::Tail).last().expect("a tail");
            let [hips, ..] = trunk(&rig);
            mean(
                moments(&rig, froude, true, 40)
                    .iter()
                    .map(|moment| moment.posed.positions[tip].y - moment.posed.positions[hips].y),
            )
        };
        let hanging = tip(-1.0, 7.0) - tip(-1.0, 0.3);
        let raised = tip(1.0, 7.0) - tip(1.0, 0.3);
        assert!(
            hanging > 0.05,
            "a hanging tail rose {hanging:.3} m at a gallop"
        );
        assert!(
            raised.abs() < hanging * 0.25,
            "a raised tail moved {raised:.3} m at a gallop, against a hanging one's {hanging:.3}"
        );
    }

    /// The largest of the back's fold across a cycle.
    fn peak(gait: &Gait) -> f32 {
        (0..100)
            .map(|step| harmonics(gait, step as f32 * 0.01).abs())
            .fold(0.0, f32::max)
    }

    /// How far a gait's girdles part once a stride: what rocks a trunk.
    fn parted(gait: &Gait) -> f32 {
        let kin = Kin {
            hoof: 0.0,
            withers: 1.0,
        };
        let ride = Ride::of(gait, kin);
        (ride.hinds[0] - ride.fores[0]).length()
    }

    /// A beast paddling at `effort`, posed at `cycle`.
    fn paddled(rig: &Rig, cycle: f32, effort: f32) -> (Posed, Swum) {
        let mut pose = Pose::rest(rig);
        let swum = paddle(rig, &mut pose, cycle, effort);
        (pose.forward(rig), swum)
    }

    #[test]
    fn a_paddle_closes_on_itself_and_rides_level() {
        // Nose up a little, never stood on end the way a person's crawl lays
        // a body prone; and the end of a stroke is its beginning.
        let rig = quadruped();
        for effort in [0.0, 0.5, 1.0] {
            let (start, _) = paddled(&rig, 0.0, effort);
            let (end, _) = paddled(&rig, 1.0, effort);
            let apart = start
                .positions
                .iter()
                .zip(&end.positions)
                .map(|(a, b)| a.distance(*b))
                .fold(0.0f32, f32::max);
            assert!(
                apart < 1.0e-4,
                "at effort {effort} the stroke ended {apart} m off its start"
            );
            for step in 0..20 {
                let (_, swum) = paddled(&rig, step as f32 / 20.0, effort);
                assert!(
                    swum.pitch.abs() < 0.3,
                    "at effort {effort} the trunk pitched {:.0} degrees",
                    swum.pitch.to_degrees()
                );
            }
        }
        // And a swim asked of a beast is this paddle, at full effort too.
        let mut pose = Pose::rest(&rig);
        let swum = crate::anim::Swim::at(0.3)
            .toward(10.0)
            .drive(&rig, &mut pose);
        assert!(
            swum.pitch.abs() < 0.3,
            "a swimming beast pitched {:.0} degrees",
            swum.pitch.to_degrees()
        );
    }

    #[test]
    fn each_paw_strokes_back_along_the_bottom_of_its_loop_in_diagonal_pairs() {
        // The power stroke: at its lowest a paw is moving backward, which is
        // what pushes the body on. And a trot's pairs paddle together.
        let rig = quadruped();
        let steps = 40;
        let paw = |limb: Limb| *rig.in_zone(Zone::Extremity(limb)).first().expect("a paw");
        let track = |limb: Limb| -> Vec<Vec3> {
            (0..steps)
                .map(|step| paddled(&rig, step as f32 / steps as f32, 0.6).0.positions[paw(limb)])
                .collect()
        };
        let mut lowest = Vec::new();
        for limb in Limb::ALL {
            let path = track(limb);
            let (low, _) = path
                .iter()
                .enumerate()
                .fold((0, f32::INFINITY), |best, (at, point)| {
                    if point.y < best.1 {
                        (at, point.y)
                    } else {
                        best
                    }
                });
            let ahead = path[(low + 1) % steps].z - path[(low + steps - 1) % steps].z;
            assert!(
                ahead < 0.0,
                "{limb:?} moved {ahead:+.3} m along its loop's bottom"
            );
            lowest.push((limb, low));
        }
        let at = |limb: Limb| {
            lowest
                .iter()
                .find(|(of, _)| *of == limb)
                .expect("every limb")
                .1
        };
        for (one, other) in [
            (Limb::ForeLeft, Limb::HindRight),
            (Limb::ForeRight, Limb::HindLeft),
        ] {
            let apart = (at(one) as i32 - at(other) as i32).rem_euclid(steps as i32);
            assert!(
                apart.min(steps as i32 - apart) <= 2,
                "{one:?} and {other:?} bottomed out {apart} steps apart"
            );
        }
    }

    // ---- #378: the gait against docs/research/research-08-quadruped-gait.md.

    /// The bodies a gait is judged on, by the family: and a cross.
    fn kin() -> Vec<(&'static str, Rig, f32)> {
        use crate::plan::Family;
        [
            ("dog", Family::Dog, None),
            ("cat", Family::Cat, None),
            ("horse", Family::Horse, None),
            ("cat+horse", Family::Cat, Some(Family::Horse)),
        ]
        .into_iter()
        .map(|(name, own, other)| {
            let mut params = QuadrupedParams::default();
            params.set_kind(own, other, if other.is_some() { 0.5 } else { 0.0 });
            let rig = beast(params);
            let withers = Kin::of(&rig).withers;
            (name, rig, withers)
        })
        .collect()
    }

    /// One moment of a cycle as the whole walk poses it.
    struct Sampled {
        at: Vec<Vec3>,
        phase: Vec<(Limb, Phase)>,
        cycle: f32,
    }

    /// `pattern` at `froude`, `frames` moments of one cycle.
    fn cycle_of(rig: &Rig, pattern: Pattern, froude: f32, frames: usize) -> Vec<Sampled> {
        let speed = Speed::from_froude(froude).in_pattern(pattern);
        let (gait, stride) = (speed.gait(rig), speed.stride(rig));
        (0..frames)
            .map(|frame| {
                let cycle = frame as f32 / frames as f32;
                let mut pose = Pose::rest(rig);
                Walk::at(cycle).drive(rig, &mut pose, &gait, &stride, |at| {
                    Some(Ground::level(Vec3::new(at.x, 0.0, at.z)))
                });
                Sampled {
                    at: pose.forward(rig).positions,
                    phase: gait
                        .limbs
                        .iter()
                        .enumerate()
                        .map(|(index, &limb)| (limb, gait.phase(index, cycle)))
                        .collect(),
                    cycle,
                }
            })
            .collect()
    }

    /// A bone's angle to straight down in the side plane, in degrees, its end
    /// ahead positive: the dossier's SEG-V.
    fn seg(v: Vec3) -> f32 {
        v.z.atan2(-v.y).to_degrees()
    }

    fn included(upper: Vec3, lower: Vec3) -> f32 {
        180.0 + ((seg(lower) - seg(upper) + 540.0).rem_euclid(360.0) - 180.0)
    }

    fn range(values: impl Iterator<Item = f32>) -> (f32, f32) {
        values.fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
            (low.min(value), high.max(value))
        })
    }

    /// How far `values` part across their second harmonic, and its phase in
    /// strides.
    fn twice_a_stride(samples: &[Sampled], of: impl Fn(&Sampled) -> f32) -> (f32, f32) {
        let mean = samples.iter().map(&of).sum::<f32>() / samples.len() as f32;
        let (mut re, mut im) = (0.0, 0.0);
        for sample in samples {
            let angle = 2.0 * TAU * sample.cycle;
            re += (of(sample) - mean) * angle.cos();
            im += (of(sample) - mean) * angle.sin();
        }
        let count = samples.len() as f32;
        (
            2.0 * (re * re + im * im).sqrt() / count,
            im.atan2(re) / (2.0 * TAU),
        )
    }

    #[test]
    fn each_leg_is_a_pantograph() {
        // Fact 2: the femur and the metatarsus move as one, and the blade with
        // the forearm. Before #378 the blade never moved and the metatarsus
        // kept to 17 degrees under a femur's 75 to 108.
        for (name, rig, _) in kin() {
            for (pattern, froude) in [(Pattern::Walk, 0.25), (Pattern::Trot, 1.2)] {
                let samples = cycle_of(&rig, pattern, froude, 96);
                let upper = rig.in_zone(Zone::UpperLimb(Limb::HindLeft));
                let hock = rig.in_zone(Zone::LowerLimb(Limb::HindLeft))[0];
                let pad = rig.in_zone(Zone::Extremity(Limb::HindLeft))[0];
                let gaps: Vec<f32> = samples
                    .iter()
                    .map(|s| seg(s.at[upper[1]] - s.at[upper[0]]) - seg(s.at[pad] - s.at[hock]))
                    .collect();
                let (low, high) = range(gaps.iter().copied());
                assert!(
                    high - low < 10.0,
                    "{name} {pattern:?}: femur and metatarsus part by {:.1} degrees",
                    high - low
                );
                let (low, high) = range(samples.iter().map(|s| seg(s.at[pad] - s.at[hock])));
                assert!(
                    high - low > 30.0,
                    "{name} {pattern:?}: the metatarsus swings {:.1} degrees",
                    high - low
                );
                let fore = rig.in_zone(Zone::UpperLimb(Limb::ForeLeft));
                let (low, high) = range(samples.iter().map(|s| seg(s.at[fore[1]] - s.at[fore[0]])));
                assert!(
                    (15.0..60.0).contains(&(high - low)),
                    "{name} {pattern:?}: the blade swings {:.1} degrees",
                    high - low
                );
            }
        }
    }

    #[test]
    fn a_paw_lands_under_the_chest_and_leaves_under_the_loin() {
        // Fact 3: lands 16 to 22 degrees ahead of the leg's pivot and leaves 14
        // to 33 behind, the leaving further at a walk's pace. Before #378 a fore
        // landed 32 to 36 ahead and left 16 to 22 behind.
        for (name, rig, _) in kin() {
            for limb in [Limb::ForeLeft, Limb::HindLeft] {
                let samples = cycle_of(&rig, Pattern::Walk, 0.25, 192);
                let chain = rig.limb_chain(limb).expect("a leg");
                let pad = rig.in_zone(Zone::Extremity(limb))[0];
                let parent = rig.joints[chain[0]].parent.expect("a blade or a hip's");
                let pivot = |s: &Sampled| {
                    if limb.is_fore() {
                        s.at[parent].lerp(s.at[chain[0]], PIVOT_ON_BLADE)
                    } else {
                        s.at[chain[0]]
                    }
                };
                let stance: Vec<(f32, f32)> = samples
                    .iter()
                    .filter_map(|s| {
                        let (_, phase) = *s.phase.iter().find(|(of, _)| *of == limb)?;
                        match phase {
                            Phase::Stance(t) => Some((t, seg(s.at[pad] - pivot(s)))),
                            Phase::Swing(_) => None,
                        }
                    })
                    .collect();
                let first = stance
                    .iter()
                    .copied()
                    .fold((2.0, 0.0), |a, b| if b.0 < a.0 { b } else { a });
                let last = stance
                    .iter()
                    .copied()
                    .fold((-1.0, 0.0), |a, b| if b.0 > a.0 { b } else { a });
                assert!(
                    (12.0..27.0).contains(&first.1),
                    "{name} {limb:?} landed {:.1} degrees ahead",
                    first.1
                );
                assert!(
                    (15.0..35.0).contains(&-last.1),
                    "{name} {limb:?} left {:.1} degrees behind",
                    -last.1
                );
                assert!(
                    -last.1 > first.1,
                    "{name} {limb:?}: a leg leaves further behind than it lands ahead"
                );
            }
        }
    }

    #[test]
    fn a_fore_folds_in_the_air_and_stands_past_straight_on_the_ground() {
        // Fact 5. Before #378 a carpus closed only to 135 in a trot's swing and
        // bent to 150 under load, and the toes never moved from 244.
        for (name, rig, _) in kin() {
            let samples = cycle_of(&rig, Pattern::Trot, 1.2, 192);
            let limb = Limb::ForeLeft;
            let [_, elbow, carpus] = rig.limb_chain(limb).expect("a leg");
            let feet = rig.in_zone(Zone::Extremity(limb));
            let of = |s: &Sampled| s.phase.iter().find(|(l, _)| *l == limb).expect("a leg").1;
            let wrist =
                |s: &Sampled| included(s.at[carpus] - s.at[elbow], s.at[feet[0]] - s.at[carpus]);
            let toes =
                |s: &Sampled| included(s.at[feet[0]] - s.at[carpus], s.at[feet[1]] - s.at[feet[0]]);
            let swinging = || samples.iter().filter(|s| !of(s).is_stance());
            let standing = || samples.iter().filter(|s| of(s).is_stance());
            let (least, _) = range(swinging().map(wrist));
            assert!(
                least < 125.0,
                "{name}: a swinging carpus closes to {least:.0}"
            );
            let (least, _) = range(standing().map(wrist));
            assert!(
                least > 175.0,
                "{name}: a standing carpus closes to {least:.0}"
            );
            let (least, _) = range(swinging().map(toes));
            assert!(least < 200.0, "{name}: swinging toes close to {least:.0}");
            let (least, _) = range(standing().map(toes));
            assert!(least > 190.0, "{name}: standing toes close to {least:.0}");
        }
    }

    #[test]
    fn the_girdles_ride_apart_at_a_walk_and_together_at_a_trot() {
        // Facts 7 and 8. Before #378 the pelvis, the withers and the head went
        // up and down as one, at the same height, at every gait.
        for (name, rig, wh) in kin() {
            let [hips, _, withers] = trunk(&rig);
            for (pattern, froude, apart) in [(Pattern::Walk, 0.25, 0.25), (Pattern::Trot, 1.2, 0.0)]
            {
                let samples = cycle_of(&rig, pattern, froude, 96);
                let (pelvis, from) = twice_a_stride(&samples, |s| s.at[hips].y);
                let (shoulders, to) = twice_a_stride(&samples, |s| s.at[withers].y);
                let gap = (to - from).rem_euclid(0.5);
                let gap = gap.min(0.5 - gap);
                assert!(
                    (gap - apart).abs() < 0.04,
                    "{name} {pattern:?}: the girdles ride {gap:.2} of a stride apart"
                );
                assert!(
                    (0.014..0.035).contains(&(pelvis / wh)),
                    "{name} {pattern:?}: the pelvis rides {:.1}% of the withers' height",
                    pelvis / wh * 200.0
                );
                let _ = shoulders;
            }
        }
    }

    #[test]
    fn a_walking_trunk_is_lowest_at_each_end_of_a_legs_stance_and_a_trotting_one_at_its_middle() {
        // Fact 8's timing: a vault and a spring.
        let rig = quadruped();
        let [hips, ..] = trunk(&rig);
        for (pattern, froude, deep) in [(Pattern::Walk, 0.25, false), (Pattern::Trot, 1.2, true)] {
            let samples = cycle_of(&rig, pattern, froude, 192);
            let height = |t0: f32, t1: f32| {
                let inside: Vec<f32> = samples
                    .iter()
                    .filter_map(
                        |s| match s.phase.iter().find(|(l, _)| *l == Limb::HindLeft)?.1 {
                            Phase::Stance(t) if (t0..t1).contains(&t) => Some(s.at[hips].y),
                            _ => None,
                        },
                    )
                    .collect();
                inside.iter().sum::<f32>() / inside.len() as f32
            };
            let (mid, ends) = (height(0.35, 0.55), height(0.0, 0.15));
            assert_eq!(
                mid < ends,
                deep,
                "{pattern:?}: the hips ride {mid:.4} at midstance and {ends:.4} at the landing"
            );
        }
    }

    #[test]
    fn a_dogs_back_bends_and_a_horses_hardly() {
        // Fact 9. Before #378 no back bent at a walk or a trot at all.
        let bodies = kin();
        let bend = |rig: &Rig, pattern, froude| {
            let [hips, loin, withers] = trunk(rig);
            let samples = cycle_of(rig, pattern, froude, 96);
            let middle = spine_to(rig, withers)[spine_to(rig, withers).len() / 2];
            let _ = loin;
            let (low, high) = range(samples.iter().map(|s| {
                let (rear, front) = (s.at[middle] - s.at[hips], s.at[withers] - s.at[middle]);
                (front.x.atan2(front.z) - rear.x.atan2(rear.z)).to_degrees()
            }));
            high - low
        };
        let dog = &bodies[0].1;
        let horse = &bodies[2].1;
        for (pattern, froude) in [(Pattern::Walk, 0.25), (Pattern::Trot, 1.2)] {
            let (dog, horse) = (bend(dog, pattern, froude), bend(horse, pattern, froude));
            assert!(
                dog > 4.0,
                "{pattern:?}: a dog's back bends {dog:.1} degrees"
            );
            assert!(
                dog > horse * 2.5,
                "{pattern:?}: a dog's back bends {dog:.1} degrees and a horse's {horse:.1}"
            );
        }
    }

    #[test]
    fn a_walking_horse_nods_against_its_withers_twice_a_stride() {
        // Fact 8: the head lowest as each fore bears, moving about three times
        // what the withers do, a quarter of a stride off them.
        let bodies = kin();
        let horse = &bodies[2].1;
        let head = horse.in_zone(Zone::Head)[0];
        let [_, _, withers] = trunk(horse);
        let samples = cycle_of(horse, Pattern::Walk, 0.25, 96);
        let (nod, from) = twice_a_stride(&samples, |s| s.at[head].y);
        let (rise, to) = twice_a_stride(&samples, |s| s.at[withers].y);
        assert!(
            nod > rise * 1.8,
            "the head moves {nod:.4} and the withers {rise:.4}"
        );
        let gap = (to - from).rem_euclid(0.5);
        assert!(
            (gap - 0.25).abs() < 0.08,
            "the head and the withers are {gap:.2} of a stride apart"
        );
    }

    #[test]
    fn no_joint_jolts_through_a_cycle() {
        // A leg flipped through its pole, toes turned the short way round past
        // the half turn, a swing hunting back from its reach: each jumped a
        // joint by 90 to 280 mm between two samples. Second differences at 240
        // samples a second of the slowest thing that can be seen. On rolled horses
        // too, whose legs reach least for how far a fold throws the wrist: an elbow
        // popped 130 mm a sample there, every stride, against 72 on the default.
        let rolled = [6, 8, 9, 11].map(|seed| {
            let record = crate::AvatarRecord::rolled(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            );
            let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
            let withers = Kin::of(&rig).withers;
            ("rolled", rig, withers)
        });
        for (name, rig, wh) in kin().into_iter().chain(rolled) {
            for (pattern, froude, bound) in [
                (Pattern::Walk, 0.25, 0.03),
                (Pattern::Trot, 1.2, 0.03),
                (Pattern::Canter(Lead::Right), 3.2, 0.03),
                (Pattern::Gallop(Lead::Right), 7.0, 0.05),
            ] {
                let speed = Speed::from_froude(froude).in_pattern(pattern);
                let frames = (240.0 / speed.cadence(&rig)) as usize;
                let samples = cycle_of(&rig, pattern, froude, frames);
                let count = samples.len();
                let mut worst = 0.0f32;
                for index in 0..count {
                    let (a, b, c) = (
                        &samples[index].at,
                        &samples[(index + 1) % count].at,
                        &samples[(index + 2) % count].at,
                    );
                    for joint in 0..a.len() {
                        worst = worst.max((a[joint] - b[joint] * 2.0 + c[joint]).length());
                    }
                }
                assert!(
                    worst < wh * bound,
                    "{name} {pattern:?} jolted {:.1} mm, {:.1}% of its height",
                    worst * 1000.0,
                    worst / wh * 100.0
                );
            }
        }
    }

    #[test]
    fn a_cross_moves_between_its_parents_with_no_jump_along_the_slider() {
        use crate::plan::Family;
        let posed = |own, other, blend: f32| {
            let mut params = QuadrupedParams::default();
            params.set_kind(own, Some(other), blend);
            let rig = beast(params);
            let kin = Kin::of(&rig);
            let mut pose = Pose::rest(&rig);
            let speed = Speed::from_froude(1.2).in_pattern(Pattern::Trot);
            Walk::at(0.3).drive(
                &rig,
                &mut pose,
                &speed.gait(&rig),
                &speed.stride(&rig),
                |at| Some(Ground::level(Vec3::new(at.x, 0.0, at.z))),
            );
            // Against the withers, for a body whose size moves along the slider.
            // What the motion does, not the body: its rest moves as it will.
            let at: Vec<Vec3> = pose
                .forward(&rig)
                .positions
                .iter()
                .zip(&rig.joints)
                .map(|(at, joint)| (*at - joint.position) / kin.withers)
                .collect();
            (kin.hoof, at)
        };
        for (own, other) in [(Family::Dog, Family::Horse), (Family::Cat, Family::Horse)] {
            let mut last = posed(own, other, 0.0);
            let mut worst = (0.0f32, 0.0f32);
            for step in 1..=20 {
                let now = posed(own, other, step as f32 / 20.0);
                worst.0 = worst.0.max((now.0 - last.0).abs());
                for (a, b) in now.1.iter().zip(&last.1) {
                    worst.1 = worst.1.max(a.distance(*b));
                }
                last = now;
            }
            assert!(
                worst.0 < 0.15 && worst.1 < 0.06,
                "{own:?}+{other:?}: a twentieth of the slider moved the hoof {:.3} and a joint \
                 {:.3} of the withers' height",
                worst.0,
                worst.1
            );
        }
    }

    #[test]
    fn a_beast_gathers_nose_up_arcs_over_and_lands_nose_down() {
        // #370. The trunk's line through a leap, against where it stands:
        // up as it gathers, past level through the flight, down as its fores
        // take the landing.
        let rig = quadruped();
        let leap = crate::anim::Leap::new(3.0);
        let rest = line(&rig, &Pose::rest(&rig).forward(&rig).positions).0;
        let pitched = |elapsed: f32| {
            let mut pose = Pose::rest(&rig);
            leap.drive(&rig, &mut pose, elapsed, |at| {
                Some(Ground::level(Vec3::new(at.x, 0.0, at.z)))
            });
            line(&rig, &pose.forward(&rig).positions).0 - rest
        };
        let gathered = pitched(leap.wind_up(&rig) * 0.95);
        let launched = pitched(leap.wind_up(&rig) + leap.flight() * 0.1);
        let landing = pitched(leap.wind_up(&rig) + leap.flight() * 0.9);
        let landed = pitched(leap.wind_up(&rig) + leap.flight() + leap.landing(&rig) * 0.1);
        let standing = pitched(leap.duration(&rig));
        assert!(
            gathered > 5.0 && launched > 5.0,
            "gathered {gathered:+.1}, launched {launched:+.1}"
        );
        assert!(
            landing < -5.0 && landed < -5.0,
            "landing {landing:+.1}, landed {landed:+.1}"
        );
        assert!(
            standing.abs() < 0.5,
            "it stood {standing:+.1} degrees off level"
        );
    }

    #[test]
    fn a_symmetric_gait_neither_rocks_nor_folds_and_a_gallop_does_both() {
        // A walk's, a trot's and a pace's pairs cancel; a canter's and a
        // gallop's do not, and a gallop's part further than a canter's.
        let rig = quadruped();
        let gait = |pattern| Gait::pattern(&rig, pattern, 0.3);
        for pattern in [Pattern::Walk, Pattern::Trot, Pattern::Pace] {
            assert!(peak(&gait(pattern)) < 1.0e-5, "{pattern:?} folds");
            assert!(parted(&gait(pattern)) < 1.0e-5, "{pattern:?} rocks");
        }
        let canter = gait(Pattern::Canter(Lead::Right));
        let gallop = gait(Pattern::RotaryGallop(Lead::Right));
        assert!(parted(&canter) > 0.3);
        assert!(peak(&gallop) > peak(&canter));
    }

    #[test]
    fn a_gallop_is_gathered_between_its_fores_lifting_and_its_hinds_landing() {
        // The gathered flight: the last fore has lifted and the first hind
        // has yet to land, so every foot is under the body.
        let rig = quadruped();
        let duty = 0.25;
        let gait = Gait::pattern(&rig, Pattern::RotaryGallop(Lead::Right), duty);
        let landing = |fore: bool| {
            gait.limbs
                .iter()
                .zip(&gait.offsets)
                .filter(|(limb, _)| limb.is_fore() == fore)
                .map(|(_, &offset)| offset)
                .collect::<Vec<_>>()
        };
        // On the turned clock: the fores' last lift and the hinds' next
        // landing, and the moment halfway between them.
        let lifted = landing(true).into_iter().fold(0.0f32, f32::max) + duty;
        let landed = landing(false)
            .into_iter()
            .map(|offset| {
                if offset < lifted - duty {
                    offset + 1.0
                } else {
                    offset
                }
            })
            .fold(f32::INFINITY, f32::min);
        let gathered = (lifted + landed) * 0.5;
        let fold = harmonics(&gait, gathered.rem_euclid(1.0));
        assert!(
            fold > 0.5 * peak(&gait),
            "the back is gathered at {gathered:.2}: {fold:.2}"
        );
    }
}
