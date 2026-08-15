//! Deciding *when* one motion becomes another, and when it simply does not have
//! to.
//!
//! [`Inertializer`] is the mechanism and it is already right: it carries the
//! body's momentum through a switch instead of stalling it, and it composes with
//! anything because it works on the offset between two poses. What was missing
//! is the layer above it — the one that answers whether a transition is needed
//! at all, when it may start, and what the incoming motion's clock should read
//! when it does.
//!
//! Four questions, and the first one is the one that saves the most work:
//!
//! * **Does this need a blend?** Within locomotion, no. Walking faster and
//!   running are the same generator at different points of one speed axis
//!   (#240), and a body moving along that axis is already continuous — blending
//!   it against itself would only smear it. A blend is for changing *family*:
//!   locomotion to a jump, a jump to a swim. See [`Family::needs_blend`].
//! * **May it start now?** Not while a foot is bearing weight. A transition that
//!   moves a planted contact makes it slide, which reads as skating rather than
//!   as changing what the body is doing. [`Gait::until_handoff`] names the next
//!   moment support is already moving, which is when a change is free.
//! * **What does the new clock read?** Not the old number. A cycle fraction
//!   means a different part of the step at a different duty, so carrying it
//!   across unchanged lands a swinging foot planted. [`Gait::phase_matched`]
//!   maps it exactly.
//! * **What if it is interrupted?** It composes, and that is a property of
//!   inertialization rather than something added here: the offset is measured
//!   from where the body actually is, so a transition started over a transition
//!   starts from the blended pose and carries its velocity. The tests here
//!   check it rather than assuming it, because "it should compose" is how a
//!   crossfade-shaped bug gets in.
//!
//! Nothing here holds state. Overlands' own motion component stays the thing
//! that remembers what the body was doing; these are the questions it asks.
//!
//! [`Gait::until_handoff`]: super::gait::Gait::until_handoff
//! [`Gait::phase_matched`]: super::gait::Gait::phase_matched

use super::blend::Inertializer;
use super::gait::Gait;
use super::pose::Pose;

/// What kind of thing a body is doing.
///
/// Coarse on purpose. The distinction that matters is whether two motions are
/// points on one continuum — in which case moving between them is just moving —
/// or genuinely different activities, which have to be joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// Travelling on the ground: every speed from a stroll to a sprint.
    Locomotion,
    /// Standing, with whatever a body does while it stands.
    Idle,
    /// Off the ground on purpose, and not as part of a stride.
    Jump,
    /// Travelling in water.
    Swim,
    /// A gesture — a wave, a bow — laid over whatever else is happening.
    Expressive,
}

impl Family {
    /// Whether moving from `self` to `other` needs a blend at all.
    ///
    /// **False within locomotion, and that is the point of the speed axis.**
    /// A walk becoming a run is one generator moving along one parameter; the
    /// pose is already continuous, and inertializing it against itself adds a
    /// decaying error to a motion that had none. Every other change is between
    /// activities that share no clock, and those need joining.
    ///
    /// Also false for a change to itself, which is not a change.
    #[must_use]
    pub fn needs_blend(self, other: Self) -> bool {
        self != other
    }

    /// Whether this family is one a body can be interrupted out of at any
    /// moment.
    ///
    /// [`Family::Expressive`] is, because a gesture is laid over the body
    /// rather than driving it and dropping it disturbs nothing that is carrying
    /// weight. The rest are not: they own the legs.
    #[must_use]
    pub fn is_overlay(self) -> bool {
        self == Family::Expressive
    }
}

/// How long a body will wait for a good moment before taking a bad one, as a
/// fraction of the gait cycle.
///
/// **A third, which is about one step.** Waiting is what buys a transition that
/// does not slide a planted foot; waiting *indefinitely* is how a body ignores
/// the thing that just happened to it. A third of a cycle is long enough to
/// reach the next handoff from almost anywhere in the step and short enough
/// that nobody reads it as a delay: at the cadence the speed axis gives a body
/// walking at 1.4 m/s, it is under three tenths of a second.
pub const HANDOFF_PATIENCE: f32 = 1.0 / 3.0;

/// When a change of family should begin.
///
/// `Wait` carries how far ahead the moment is, as a cycle fraction, so a caller
/// can decide for itself whether to hold or to take it now — the two callers
/// that exist want different things and neither wants this deciding for them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    /// Start it now: nothing is bearing weight that this would disturb.
    Now,
    /// Hold for this much of a cycle, at which point support transfers anyway.
    Wait(f32),
}

impl Entry {
    /// Whether this says to go.
    #[must_use]
    pub fn is_now(self) -> bool {
        self == Entry::Now
    }
}

/// When a transition out of `gait` may begin, given where in the cycle it is.
///
/// Returns [`Entry::Now`] when the body is already at a handoff or beyond
/// [`HANDOFF_PATIENCE`] from one — the first because the moment is free, the
/// second because a body that waits longer than that has stopped responding.
/// An overlay family never waits: a gesture does not move what is under it.
#[must_use]
pub fn entry(gait: &Gait, cycle: f32, into: Family) -> Entry {
    // A body with nothing on the ground has no planted foot to protect, so
    // there is nothing for it to wait for. That covers the case waiting would
    // be actively wrong for: the ground vanishing under a walking body is not
    // an elective change and cannot be held until a convenient moment (#243).
    if into.is_overlay() || gait.is_empty() || gait.grounded(cycle) == 0 {
        return Entry::Now;
    }
    let ahead = gait.until_handoff(cycle);
    if ahead <= f32::EPSILON || ahead > HANDOFF_PATIENCE {
        Entry::Now
    } else {
        Entry::Wait(ahead)
    }
}

/// Where the incoming gait's clock should be set when a body changes gait.
///
/// A thin name over [`Gait::phase_matched`], here because this is where a
/// caller is looking when it needs it: the number a cycle carries means a
/// different part of the step at a different duty, and handing it across
/// unchanged is what puts a swinging foot on the floor.
///
/// [`Gait::phase_matched`]: super::gait::Gait::phase_matched
#[must_use]
pub fn carry_cycle(from: &Gait, into: &Gait, cycle: f32) -> f32 {
    into.phase_matched(from, cycle)
}

/// The three poses a transition is measured between.
///
/// **`previous` and `current` are the body as it was DRAWN, blend and all** —
/// not what the outgoing generator would have produced. That distinction is the
/// whole of interruptibility: measured from the drawn pose, a transition started
/// over a running transition begins from exactly where the body is and carries
/// the velocity it actually had, so nothing moves on the frame it starts.
/// Measured from the generator, the body snaps to wherever the motion it was
/// already leaving would have put it.
#[derive(Clone, Copy, Debug)]
pub struct Frames<'a> {
    /// The frame before last, as drawn. Supplies the velocity.
    pub previous: &'a Pose,
    /// The last frame, as drawn.
    pub current: &'a Pose,
    /// Where the incoming motion puts the body this frame.
    pub target: &'a Pose,
}

/// Starts a transition, or declines to.
///
/// **The whole governor in one call.** `None` means no blend is wanted — either
/// the families match, so the motion is already continuous, or the moment is
/// wrong and the caller should ask again next frame. `Some` is an
/// [`Inertializer`] already carrying the body's momentum.
///
/// See [`Frames`] for which poses to hand it and why that choice is what makes
/// an interrupted transition compose.
#[must_use]
pub fn begin(
    from: Family,
    into: Family,
    gait: &Gait,
    cycle: f32,
    frames: Frames<'_>,
    dt: f32,
    duration: f32,
) -> Option<Inertializer> {
    if !from.needs_blend(into) {
        return None;
    }
    if !entry(gait, cycle, into).is_now() {
        return None;
    }
    Some(Inertializer::start(
        frames.previous,
        frames.current,
        frames.target,
        dt,
        duration,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Speed;
    use crate::anim::gait::{Phase, step};
    use crate::anim::ground::Ground;
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::Rig;
    use glam::Vec3;

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    #[test]
    fn moving_along_the_speed_axis_is_not_a_transition() {
        // **The most valuable answer the governor gives is "no".** A walk
        // becoming a run is one generator moving along one parameter, and the
        // pose is already continuous; inertializing it against itself would add
        // a decaying error to a motion that had none. Blends are for changing
        // activity.
        assert!(!Family::Locomotion.needs_blend(Family::Locomotion));
        assert!(Family::Locomotion.needs_blend(Family::Jump));
        assert!(Family::Idle.needs_blend(Family::Locomotion));
        assert!(Family::Swim.needs_blend(Family::Locomotion));
        // And a gesture rides over whatever is underneath rather than owning
        // the legs, so it is the one that can be dropped at any moment.
        assert!(Family::Expressive.is_overlay());
        for owning in [Family::Locomotion, Family::Idle, Family::Jump, Family::Swim] {
            assert!(!owning.is_overlay(), "{owning:?} owns the legs");
        }
    }

    #[test]
    fn a_transition_waits_for_the_foot_that_is_carrying_the_body() {
        // A contact bearing weight is pinned to the ground; moving it makes it
        // slide, which reads as skating. The free moment is the handoff, where
        // support is already changing hands.
        let rig = biped();
        let gait = Speed::new(&rig, 1.4).gait(&rig);

        // **Just BEFORE a handoff, not just after one.** The rule is not "wait
        // whenever a foot is down" — a foot is down almost always — it is "if
        // the moment is nearly here, take it". A body at 0.15 has 0.35 of a
        // cycle to wait and should go now; one at 0.45 has 0.05 and should
        // hold, because holding costs nothing and sliding a planted foot does.
        let mid = 0.45;
        match entry(&gait, mid, Family::Jump) {
            Entry::Wait(ahead) => {
                let landing = (gait.until_handoff(mid) - ahead).abs();
                assert!(landing < 1e-6, "the wait does not end at a handoff");
                assert!(gait.until_handoff(mid + ahead) < 1e-4);
            }
            Entry::Now => panic!("started a jump on a foot that was carrying the body"),
        }
        // At a handoff itself, free.
        assert!(entry(&gait, 0.0, Family::Jump).is_now());
        assert!(entry(&gait, 0.5, Family::Jump).is_now());
        // And far from one, free as well: a body that holds out for a perfect
        // moment has stopped answering. This is the half the patience buys.
        assert!(entry(&gait, 0.15, Family::Jump).is_now());
        // And a gesture never waits, because it does not move what is under it.
        assert!(entry(&gait, mid, Family::Expressive).is_now());
    }

    #[test]
    fn a_body_does_not_wait_longer_than_it_would_take_to_respond() {
        // The other half of waiting: a body that holds out for a perfect moment
        // has stopped answering. Nowhere in the cycle may the governor ask for
        // more than `HANDOFF_PATIENCE`.
        let rig = biped();
        for metres in [1.0f32, 1.4, 3.0] {
            let gait = Speed::new(&rig, metres).gait(&rig);
            for sample in 0..500 {
                let cycle = sample as f32 / 500.0;
                if let Entry::Wait(ahead) = entry(&gait, cycle, Family::Jump) {
                    assert!(
                        ahead <= HANDOFF_PATIENCE,
                        "at {metres} m/s, cycle {cycle}, the governor asked to wait {ahead}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_clock_carries_the_step_across_rather_than_the_number() {
        // **The discontinuity a crossfade is usually hiding.** A duty of 0.59
        // puts the leading contact's takeoff at 0.59 and a duty of 0.32 puts it
        // at 0.32, so handing the cycle across unchanged lands a foot in a
        // different part of its step — planted where it was swinging, or the
        // reverse. Asserted as the thing itself: the phase KIND and its
        // progress survive the change.
        let rig = biped();
        let walk = Speed::new(&rig, 1.4).gait(&rig);
        let run = Speed::new(&rig, 3.0).gait(&rig);
        assert!(
            walk.duty > run.duty + 0.2,
            "the two gaits must really differ"
        );

        let mut carried_would_have_broken = 0;
        for sample in 0..200 {
            let cycle = sample as f32 / 200.0;
            let mapped = carry_cycle(&walk, &run, cycle);
            let (before, after) = (walk.phase(0, cycle), run.phase(0, mapped));
            match (before, after) {
                (Phase::Stance(a), Phase::Stance(b)) | (Phase::Swing(a), Phase::Swing(b)) => {
                    assert!(
                        (a - b).abs() < 2e-3,
                        "cycle {cycle} carried {a:.4} of its step across as {b:.4}"
                    );
                }
                _ => panic!("cycle {cycle} changed phase kind: {before:?} to {after:?}"),
            }
            // And record how often the naive carry would have got it wrong.
            if std::mem::discriminant(&run.phase(0, cycle)) != std::mem::discriminant(&before) {
                carried_would_have_broken += 1;
            }
        }
        assert!(
            carried_would_have_broken > 20,
            "the naive carry only broke {carried_would_have_broken} of 200 samples — this \
             test is not exercising the thing it guards"
        );
    }

    #[test]
    fn a_body_can_stop_where_both_feet_are_down_and_a_run_cannot_stop_at_all() {
        // Walk to idle "waits for a settling step rather than freezing
        // mid-swing". A body with all its feet down can simply hold still; one
        // caught mid-swing has to put a foot somewhere.
        let rig = biped();
        let walk = Speed::new(&rig, 1.4).gait(&rig);
        for sample in 0..200 {
            let cycle = sample as f32 / 200.0;
            let ahead = walk.until_settled(cycle).expect("a walk can always stop");
            assert!(
                walk.is_settled((cycle + ahead).rem_euclid(1.0) + 1e-4),
                "the moment named at {cycle} was not a settled one"
            );
            assert!(ahead <= 0.5 + 1e-3, "a biped settles twice a cycle");
        }
        // A run never has both feet down, and saying so is the honest answer:
        // it has to slow to a walk first, which along the speed axis it does by
        // itself.
        let run = Speed::new(&rig, 3.0).gait(&rig);
        assert!(run.has_flight());
        for sample in 0..200 {
            assert_eq!(
                run.until_settled(sample as f32 / 200.0),
                None,
                "a run reported a moment it could stop at"
            );
        }
    }

    #[test]
    fn a_transition_interrupted_by_another_composes_rather_than_snapping() {
        // **Checked rather than assumed.** The claim is that inertialization
        // composes because the offset is measured from where the body actually
        // is; "it should compose" is exactly how a crossfade-shaped bug gets
        // in. What must hold is that restarting mid-blend does not move the
        // body on the frame it restarts — the pose the second transition
        // produces at t=0 has to be the pose the first one was already showing.
        let rig = biped();
        let ground = |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)));
        let speed = Speed::new(&rig, 1.4);
        let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));

        let posed_at = |cycle: f32| {
            let mut pose = Pose::rest(&rig);
            step(&rig, &mut pose, &gait, &stride, cycle, ground);
            pose
        };
        let dt = 1.0 / 60.0;
        let (previous, current) = (posed_at(0.30), posed_at(0.32));
        let standing = Pose::rest(&rig);

        let mut first = Inertializer::start(&previous, &current, &standing, dt, 0.25);
        first.advance(0.08);
        let mid_blend = first.apply(&standing);

        // Interrupted: a third motion arrives while the first is still running.
        // The outgoing pose is the one the body is being DRAWN in, blend and
        // all, which is what makes this compose.
        let mut drifting = standing.clone();
        step(&rig, &mut drifting, &gait, &stride, 0.60, ground);
        let mut second = Inertializer::start(&mid_blend, &mid_blend, &drifting, dt, 0.25);
        let at_switch = second.apply(&drifting);

        for (index, (a, b)) in at_switch
            .rotations
            .iter()
            .zip(&mid_blend.rotations)
            .enumerate()
        {
            // Compared by dot rather than by an angle: `acos` of a dot near one
            // loses all its precision exactly here, and two bit-identical
            // rotations read as a third of a milliradian apart.
            assert!(
                1.0 - a.dot(*b).abs() < 1e-6,
                "joint {index} jumped on the frame the transition was interrupted"
            );
        }

        // And it still finishes, on the new target rather than the old one.
        second.advance(1.0);
        assert!(second.finished());
        let landed = second.apply(&drifting);
        for (index, (a, b)) in landed.rotations.iter().zip(&drifting.rotations).enumerate() {
            assert!(
                1.0 - a.dot(*b).abs() < 1e-6,
                "joint {index} did not arrive at the motion it was going to"
            );
        }
    }

    #[test]
    fn the_governor_declines_a_blend_it_does_not_need_and_takes_one_it_does() {
        let rig = biped();
        let speed = Speed::new(&rig, 1.4);
        let gait = speed.gait(&rig);
        let pose = Pose::rest(&rig);
        let frames = Frames {
            previous: &pose,
            current: &pose,
            target: &pose,
        };
        let call = |from, into, cycle| begin(from, into, &gait, cycle, frames, 1.0 / 60.0, 0.25);
        assert!(
            call(Family::Locomotion, Family::Locomotion, 0.0).is_none(),
            "the speed axis is the transition; nothing should be blended"
        );
        assert!(
            call(Family::Locomotion, Family::Jump, 0.45).is_none(),
            "a jump should not start on a foot that is carrying the body"
        );
        assert!(
            call(Family::Locomotion, Family::Jump, 0.0).is_some(),
            "at a handoff the change is free and should be taken"
        );
        assert!(
            call(Family::Locomotion, Family::Expressive, 0.45).is_some(),
            "a gesture rides over the legs and never waits"
        );
    }
}
