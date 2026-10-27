//! How a four-legged body carries itself through a gait (#370), where a
//! person's walk leans.
//!
//! A person leans into their pace; a beast's trunk is already horizontal, and
//! pitched forward it drops its withers and stands its head on its chin. What
//! a running quadruped's trunk does instead is **rock** - its hindquarters
//! settling while its hinds bear it, its forehand while its fores do - and,
//! galloping, **fold** its back as its hinds reach under it and stretch it as
//! they push off. Its head is held steady over all of that and reaches
//! forward at speed, and its tail sways with the hips at a walk and streams
//! out behind at a gallop.
//!
//! **Read off the footfalls, not off the pattern's name.** The rock and the
//! fold are each the first harmonic of a sum over the four contacts: how the
//! hinds' load and the fores' differ, and how far the hinds reach forward
//! while the fores reach back. A walk's, a trot's and a pace's pairs cancel
//! in both sums, so they carry a level, straight back; a canter's and a
//! gallop's hinds and fores land apart, so they rock and fold by as much as
//! their footfalls part. Nothing here asks which [`super::Pattern`] a gait is.

use std::f32::consts::TAU;

use glam::{Quat, Vec2, Vec3};

use super::gait::{Gait, relevel_head, spine_to};
use super::ground::{solve_contact, solve_contact_toward};
use super::pose::Pose;
use super::speed::leg_of;
use super::swim::Swum;
use crate::det::{self, DetMath, Rot};
use crate::plan::{FootKind, HeadKind, Limb, Zone};
use crate::rig::{Rig, Role};

/// The duty a gait has all its flight at: a gallop's least (#370), where the
/// back folds furthest and the neck reaches furthest.
const FULL_FLIGHT: f32 = 0.22;

/// How far the back folds and stretches either way at a full gallop, in
/// radians at the loin: a dog's or a cat's supple back, a horse's stiff one.
/// Provenance: **tuned by render** (#370), inside the literature's band - a
/// greyhound's back flexes through some forty degrees a stride, a horse's
/// through a few.
const FOLD_PAWS: f32 = 0.45;
const FOLD_HOOVES: f32 = 0.06;

/// How far the end of the trunk a rock sinks goes down at the rock's
/// deepest, in the leg's reach: on the default dog 14 mm, a degree and a
/// half.
///
/// **A drop, not an angle**, because a trunk is as long as its plan says: an
/// angle a short-bodied beast rocks by visibly drops a long-bodied one's
/// hindquarters through a sixth of its height - 0.08 radians on the default
/// dog's half-metre trunk sank an end by 43 mm.
/// Provenance: **tuned by test** (#370).
const ROCK: f32 = 0.05;

/// How far apart a gait's hinds and fores must load, as the rock's first
/// harmonic, before it rocks all the way: a canter's and a gallop's both do,
/// though a gallop's part twice as far - a canter's longer stride gives the
/// same moment longer to turn the body, which is why it is the gait that
/// rocks. Below it the rock fades, and a gait whose pairs cancel has none.
/// Provenance: **chosen** (#370), under a canter's `0.48`.
const ROCK_FULL: f32 = 0.4;

/// How much of the trunk's pitch the head takes back off: a dog's or a cat's
/// held steady, a horse's nodding with its neck.
/// Provenance: **tuned by render** (#370).
const STEADY_PAWS: f32 = 1.0;
const STEADY_HOOVES: f32 = 0.5;

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

/// Carries a four-legged body's trunk, head and tail through `gait` at
/// `cycle`.
///
/// Call where a person's walk leans: after [`super::gait::step`] and before
/// the footing tail, which replants the stance feet the rock and the fold
/// moved and leaves the swinging ones reaching where the back sent them.
/// `head_level: false` leaves the head to ride the trunk, the ablation
/// [`super::gait::Walk::head_level`] names. A body standing on any other
/// number of legs, or a gait that never lifts one, is left alone.
pub(super) fn carry(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32, head_level: bool) {
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
    let paws = rig.anatomy.feet != FootKind::Hooves;
    // How much of a gallop this is, `0..1`, off the duty: nothing until a
    // foot's stance is shorter than its swing.
    let flight = ((0.5 - gait.duty) / (0.5 - FULL_FLIGHT)).clamp(0.0, 1.0);
    let (gather, rock) = harmonics(gait, cycle);
    let fold = gather * flight * if paws { FOLD_PAWS } else { FOLD_HOOVES };
    let trunk = rig.joints[girdle].position - rig.joints[root].position;
    let pitch = (rock * ROCK * leg_of(rig) / trunk.length().max(f32::EPSILON))
        .clamp(-1.0, 1.0)
        .det_asin();

    // **The rock only ever sinks the end it moves.** Nose up, it turns the
    // trunk about the withers and the hindquarters settle under the hinds
    // that bear them; nose down, about the hips, and the forehand settles
    // onto its fores. A girdle that sinks asks its legs only to bend further,
    // where one that rises asks them to reach: turned about the trunk's
    // middle, a canter lifted its withers off a fore planted at the front of
    // its stride and the plant dropped the whole body 35 mm to get it back.
    // The fold turns the loin up under the arch and the back down from it,
    // about the hips, so neither girdle moves and only the back between them
    // bends. A positive turn about X carries the nose down.
    //
    // Where the step stood each foot on the ground, which the trunk's turns
    // below must not move.
    let held = pose.forward(rig).rotations;
    let stepped = pose.rotations.clone();
    let stood: Vec<(Limb, Vec3)> = {
        let posed = pose.forward(rig);
        gait.limbs
            .iter()
            .enumerate()
            .filter(|&(index, _)| gait.phase(index, cycle).is_stance())
            .filter_map(|(_, &limb)| {
                let paw = *rig.in_zone(Zone::Extremity(limb)).first()?;
                Some((limb, posed.positions[paw]))
            })
            .collect()
    };
    sink_end(rig, pose, pitch);
    let middle = spine_to(rig, girdle)
        .first()
        .copied()
        .filter(|&joint| joint != girdle);
    let mut inherited = Rot::x(-pitch);
    if let Some(loin) = middle {
        pose.rotations[root] *= Rot::x(-fold * 0.5);
        pose.rotations[loin] *= Rot::x(fold);
        inherited *= Rot::x(fold * 0.5);
    }
    if head_level {
        let steady = if paws { STEADY_PAWS } else { STEADY_HOOVES };
        relevel_head(rig, pose, det::slerp(Quat::IDENTITY, inherited, steady));
    }
    // And at speed the neck reaches forward and the head goes down with it,
    // less than the neck does. **From the neck's own joint, not the
    // withers**: the fores hang from the withers, and turned there a horse's
    // shoulders swung 10 cm under its neck, further than a planted leg could
    // follow.
    let reach = REACH * flight;
    if reach > 0.0 {
        pose.rotations[neck] *= Rot::x(reach);
        if let Some(&head) = rig.in_zone(Zone::Head).first() {
            pose.rotations[head] *= Rot::x(-reach * (1.0 - REACH_KEPT));
        }
    }

    // **A foot on the ground stays where the step stood it.** Turned with
    // its girdle, a fore at the back of its stance swung further back and up
    // as the fold tipped the forehand, and the plant dropped the body 16 mm
    // to reach it - and let go the frame that foot lifted, a 30 mm jolt into
    // the gallop's flight. So its leg first keeps the attitude the step gave
    // it, and then the foot is put back where it stood: the girdle still
    // moves, and the plant puts a foot back down without putting it back
    // across the ground - 15 mm of slide through a gallop's stance. **Only
    // where the leg reaches**: pinned beyond its reach, the plant dropped the
    // whole body to make up the difference, 25 mm in a frame at a gallop.
    // A leg in the air goes with its girdle, and gathers and stretches with
    // the back.
    let turned = pose.forward(rig).rotations;
    for (limb, at) in stood {
        let Some(chain) = rig.limb_chain(limb) else {
            continue;
        };
        let top = chain[0];
        if let Some(parent) = rig.joints[top].parent {
            pose.rotations[top] = turned[parent].inverse() * held[parent] * stepped[top];
        }
        let kept = chain.map(|joint| pose.rotations[joint]);
        if !solve_contact(rig, pose, limb, at) {
            for (joint, rotation) in chain.into_iter().zip(kept) {
                pose.rotations[joint] = rotation;
            }
        }
    }

    swish(rig, pose, gait, cycle, flight);
}

/// Whether `rig` stands on four legs, and so moves as a beast does (#370).
pub(super) fn four_legged(rig: &Rig) -> bool {
    rig.ground_contacts().len() == 4
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
/// The gait's rock and [`super::clip::Target::Pitch`] both, so a sit and a
/// canter's settling hindquarters are one motion.
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
    let tail = rig.in_zone(Zone::Tail);
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

/// The back's fold and the trunk's rock at `cycle`, each in `-1..=1`.
///
/// The fold is positive **gathered** - the hinds reaching under the body as
/// they land, the fores back under it as they lift - and negative stretched,
/// and is the first harmonic of that reach summed over the contacts, so it is
/// as large as the hinds and the fores part. The rock is positive nose up,
/// while the hinds bear more of the body than the fores - each contact is
/// loaded most at its midstance - and rocks all the way once the loads part
/// by [`ROCK_FULL`].
fn harmonics(gait: &Gait, cycle: f32) -> (f32, f32) {
    let duty = gait.duty;
    // Each sum as a complex number, a point turning once a cycle: its length
    // is how far the contacts part and its angle when the motion peaks.
    let mut gather = Vec2::ZERO;
    let mut rock = Vec2::ZERO;
    for (&limb, &offset) in gait.limbs.iter().zip(&gait.offsets) {
        let (reaching, load) = if limb.is_fore() {
            (offset + duty, -1.0)
        } else {
            (offset, 1.0)
        };
        gather += turned(reaching);
        rock += turned(offset + duty * 0.5) * load;
    }
    let now = turned(cycle);
    let at = |sum: Vec2| sum.dot(now) * 0.25;
    let parted = rock.length() * 0.25;
    let rocking = if parted > f32::EPSILON {
        at(rock) / parted * (parted / ROCK_FULL).min(1.0)
    } else {
        0.0
    };
    (at(gather), rocking)
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
    let tail = rig.in_zone(Zone::Tail);
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
        let wave = TAIL_WAVE * flight * harmonics(gait, late).0;
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
    use crate::plan::{BodyPlan, Limb, QuadrupedParams};

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

    /// The pelvis, the loin and the withers.
    fn trunk(rig: &Rig) -> [usize; 3] {
        [Zone::Pelvis, Zone::Abdomen, Zone::Chest]
            .map(|zone| *rig.in_zone(zone).first().expect("a beast's trunk"))
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

    /// The largest of a harmonic across a cycle.
    fn peak(gait: &Gait, of: impl Fn((f32, f32)) -> f32) -> f32 {
        (0..100)
            .map(|step| of(harmonics(gait, step as f32 * 0.01)).abs())
            .fold(0.0, f32::max)
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
            assert!(
                peak(&gait(pattern), |(fold, _)| fold) < 1.0e-5,
                "{pattern:?} folds"
            );
            assert!(
                peak(&gait(pattern), |(_, rock)| rock) < 1.0e-5,
                "{pattern:?} rocks"
            );
        }
        let canter = gait(Pattern::Canter(Lead::Right));
        let gallop = gait(Pattern::RotaryGallop(Lead::Right));
        assert!(peak(&canter, |(_, rock)| rock) > 0.3);
        assert!(peak(&gallop, |(fold, _)| fold) > peak(&canter, |(fold, _)| fold));
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
        let (fold, _) = harmonics(&gait, gathered.rem_euclid(1.0));
        assert!(
            fold > 0.5 * peak(&gait, |(fold, _)| fold),
            "the back is gathered at {gathered:.2}: {fold:.2}"
        );
    }
}
