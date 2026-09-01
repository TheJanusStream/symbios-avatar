//! Holding planted feet still while the body's speed changes.
//!
//! The gait derives every stance offset from the stride, and the stride from
//! the current speed, fresh each frame. At a constant speed that is exact: the
//! offset marches backward under the body at precisely the body's own speed,
//! so a planted contact stands still in the world without anything remembering
//! it. The moment the speed *changes* the derivation slides: the offset scales
//! with the stride's length, so a change of `dL` moves a foot at stance phase
//! `t` by `|t − 0.5| · dL` — and the cadence changes alongside, so pinning the
//! length alone leaves `v · (1 − L_planted / L_current)` of slide, which is
//! half a fix (#277, measured at 91 mm against a 2 mm steady control under the
//! consuming app's real speed profile).
//!
//! [`Footholds`] is the memory that makes it exact under a changing speed too:
//! the same pattern as the idle being handed the stance the body arrived in
//! (#276), extended into the walk. It watches each contact's phase; when a
//! foot goes down it records the WORLD point the gait is planting at — which
//! is the point the eased swing was already landing toward, so touchdown picks
//! up the hold without a step — and while the foot stays down it serves that
//! point back, superseding the stride-derived march entirely. Only the
//! horizontal is held: the seat re-derives the height beneath the point every
//! frame, exactly as a derived stance does, so terrain stays the ground
//! closure's business.
//!
//! # What the caller provides, and the one asterisk
//!
//! The body's own world placement, because the engine deliberately has no
//! idea where the body is — `at` and `facing` are the transform the caller is
//! about to render the body under, and the ledger's points live in that frame.
//! A peer deriving a remote body's pose integrates its own *observed* travel,
//! so held points can diverge cosmetically between peers; that is not new —
//! the stride itself is already derived from per-peer-observed velocity — but
//! it is the one asterisk on the identical-pose-from-record-and-clock story,
//! and it is why the ledger holds nothing that feeds back into the clock.
//!
//! # Warps
//!
//! A held point is only meaningful while the body is somewhere near it. A
//! teleport self-heals: a hold whose body-frame answer lands further from the
//! stride's own than the leg could ever reach is re-planted at the stride's
//! answer rather than lunged for. [`Footholds::reset`] exists for the caller
//! that knows a warp happened and wants no frame of doubt.

use glam::{Quat, Vec3};

use super::gait::{Gait, Phase, Stride, Walk, Walked, contact_offset, home_of};
use super::ground::Ground;
use super::pose::Pose;
use crate::plan::Limb;
use crate::rig::Rig;

/// Where each planted contact actually went down, held in world space.
///
/// Drive the walk through [`Self::drive`] instead of [`Walk::drive`] and feed
/// it the body's transform; everything else is the module docs' story. A
/// default-constructed ledger simply plants on the first stance frame it
/// sees, so there is no warm-up to manage.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Footholds {
    /// One hold per contact the gait has shown this ledger, by limb: the
    /// world point the foot went down at, or `None` while it is in the air.
    held: Vec<(Limb, Option<Vec3>)>,
}

impl Footholds {
    /// A ledger holding nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget every hold, so the next stance frame plants afresh.
    ///
    /// For the caller that knows the body was just warped. An unnoticed warp
    /// self-heals anyway — see the module docs — but healing costs one frame
    /// of the foot answering for a point that no longer makes sense.
    pub fn reset(&mut self) {
        self.held.clear();
    }

    /// One frame of a walk with its planted feet held: advance the ledger,
    /// then run [`Walk::drive`]'s whole sequence with the holds applied.
    ///
    /// `at` and `facing` are the body's placement in the caller's world — the
    /// origin the pose will be rendered under (only its horizontal is read;
    /// the vertical belongs to the ground closure) and the yaw about `+Y`
    /// carrying the body's `+Z` onto its world heading. `ground` answers in
    /// the body's frame, exactly as [`Walk::drive`] takes it.
    //
    // The count is `Walk::drive`'s own shape plus the two transform terms the
    // ledger exists to be fed; a struct invented to appease the lint would
    // have no second use.
    #[allow(clippy::too_many_arguments)]
    pub fn drive<F>(
        &mut self,
        walk: Walk,
        rig: &Rig,
        pose: &mut Pose,
        gait: &Gait,
        stride: &Stride,
        at: Vec3,
        facing: f32,
        ground: F,
    ) -> Walked
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let into_world = |body: Vec3| {
            let world = at + Quat::from_rotation_y(facing) * body;
            Vec3::new(world.x, 0.0, world.z)
        };
        let into_body = |world: Vec3| Quat::from_rotation_y(-facing) * (world - at);

        let mut anchors: Vec<(Limb, Vec3)> = Vec::new();
        for (index, &limb) in gait.limbs.iter().enumerate() {
            let Some(home) = home_of(rig, limb) else {
                continue;
            };
            let phase = gait.phase(index, walk.cycle);
            // A gait that never lifts a contact is a body standing still, and
            // standing is the idle's business, not this ledger's — it already
            // pins an arrival stance of its own (#276). Holding through duty
            // 1.0 would fight it.
            let down = matches!(phase, Phase::Stance(_)) && gait.duty < 1.0;

            let hold = match self.held.iter_mut().find(|(of, _)| *of == limb) {
                Some((_, hold)) => hold,
                None => {
                    self.held.push((limb, None));
                    &mut self.held.last_mut().expect("just pushed").1
                }
            };
            if !down {
                *hold = None;
                continue;
            }

            // What the stride would derive this frame — the plant when a foot
            // has just gone down, and the yardstick for a hold that has
            // stopped making sense.
            let derived = contact_offset(home, stride, phase);
            let derived = Vec3::new(derived.x, 0.0, derived.z);
            let reach = rig.limb_reach(limb).unwrap_or(f32::INFINITY);

            let anchor = match *hold {
                Some(point) => {
                    let served = into_body(point) - home;
                    let served = Vec3::new(served.x, 0.0, served.z);
                    // The self-heal: a hold further from the stride's own
                    // answer than the leg is long is a warp's leftover, not a
                    // stance — no legitimate speed change moves the two apart
                    // by more than a fraction of a stride.
                    if served.distance(derived) > reach {
                        *hold = Some(into_world(home + derived));
                        derived
                    } else {
                        served
                    }
                }
                None => {
                    *hold = Some(into_world(home + derived));
                    derived
                }
            };
            anchors.push((limb, anchor));
        }

        walk.drive_anchored(rig, pose, gait, stride, ground, &anchors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Speed;
    use crate::plan::{BodyPlan, HumanoidParams, Zone};

    fn rig() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    fn flat(point: Vec3) -> Option<Ground> {
        Some(Ground::level(Vec3::new(point.x, 0.0, point.z)))
    }

    #[test]
    fn a_held_walk_at_constant_speed_is_the_stateless_walk() {
        // The property the whole design leans on: at a constant speed the
        // derived offset marches backward at exactly the body's speed, so a
        // world-held point and the fresh derivation are the same answer — the
        // ledger must cost a steady walk nothing.
        let rig = rig();
        let speed = Speed::new(&rig, 1.4);
        let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));
        let cadence = speed.cadence(&rig);
        let feet: Vec<usize> = gait
            .limbs
            .iter()
            .map(|&limb| rig.in_zone(Zone::Extremity(limb))[0])
            .collect();

        let mut ledger = Footholds::new();
        let dt = 1.0 / 60.0;
        let mut worst = 0.0f32;
        for frame in 0..240 {
            let elapsed = frame as f32 * dt;
            let cycle = (cadence * elapsed).rem_euclid(1.0);
            let travel = Vec3::Z * (1.4 * elapsed);

            let mut stateless = Pose::rest(&rig);
            Walk::at(cycle).drive(&rig, &mut stateless, &gait, &stride, flat);
            let mut held = Pose::rest(&rig);
            ledger.drive(
                Walk::at(cycle),
                &rig,
                &mut held,
                &gait,
                &stride,
                travel,
                0.0,
                flat,
            );

            let (a, b) = (stateless.forward(&rig), held.forward(&rig));
            for &foot in &feet {
                worst = worst.max(a.positions[foot].distance(b.positions[foot]));
            }
        }
        assert!(
            worst < 1e-3,
            "holding footholds moved a steady walk's foot {:.2} mm",
            worst * 1000.0
        );
    }

    #[test]
    fn a_warped_body_re_plants_instead_of_lunging() {
        // A held point is only meaningful near the body. After a teleport the
        // self-heal must re-plant at the stride's own answer — a foot reaching
        // for a point a hundred metres behind is a leg-length of strain and a
        // frame nobody should ever see.
        let rig = rig();
        let speed = Speed::new(&rig, 1.4);
        let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));
        let cadence = speed.cadence(&rig);
        let dt = 1.0 / 60.0;

        let mut ledger = Footholds::new();
        for frame in 0..30 {
            let elapsed = frame as f32 * dt;
            let mut pose = Pose::rest(&rig);
            ledger.drive(
                Walk::at((cadence * elapsed).rem_euclid(1.0)),
                &rig,
                &mut pose,
                &gait,
                &stride,
                Vec3::Z * (1.4 * elapsed),
                0.0,
                flat,
            );
        }

        // The warp: same cycle stream, position suddenly 100 m away.
        let elapsed = 30.0 * dt;
        let cycle = (cadence * elapsed).rem_euclid(1.0);
        let mut warped = Pose::rest(&rig);
        let held = ledger.drive(
            Walk::at(cycle),
            &rig,
            &mut warped,
            &gait,
            &stride,
            Vec3::new(100.0, 0.0, 100.0),
            0.0,
            flat,
        );
        let mut fresh = Pose::rest(&rig);
        let stateless = Walk::at(cycle).drive(&rig, &mut fresh, &gait, &stride, flat);

        assert_eq!(
            held.steps.straining, stateless.steps.straining,
            "the warp left a limb straining that the stateless walk does not"
        );
        let (a, b) = (warped.forward(&rig), fresh.forward(&rig));
        for &limb in &gait.limbs {
            let foot = rig.in_zone(Zone::Extremity(limb))[0];
            assert!(
                a.positions[foot].distance(b.positions[foot]) < 1e-3,
                "{limb:?} answered for a pre-warp hold: {:.1} mm from the re-planted answer",
                a.positions[foot].distance(b.positions[foot]) * 1000.0
            );
        }
    }

    #[test]
    fn a_standing_gait_holds_nothing() {
        // Standing is the idle's business (#276): at duty 1.0 the ledger must
        // clear rather than pin feet under a body something else is placing.
        let rig = rig();
        let walking = Speed::new(&rig, 1.4);
        let mut ledger = Footholds::new();
        let mut pose = Pose::rest(&rig);
        ledger.drive(
            Walk::at(0.25),
            &rig,
            &mut pose,
            &walking.gait(&rig),
            &walking.stride(&rig),
            Vec3::ZERO,
            0.0,
            flat,
        );
        assert!(
            ledger.held.iter().any(|(_, hold)| hold.is_some()),
            "the walking frame should have planted something"
        );

        let standing = Gait::standing(&rig);
        let mut pose = Pose::rest(&rig);
        ledger.drive(
            Walk::at(0.30),
            &rig,
            &mut pose,
            &standing,
            &walking.stride(&rig),
            Vec3::ZERO,
            0.0,
            flat,
        );
        assert!(
            ledger.held.iter().all(|(_, hold)| hold.is_none()),
            "a standing gait left holds in the ledger"
        );
    }
}
