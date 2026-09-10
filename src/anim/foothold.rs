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
//! # Turns, and a body that outruns its feet
//!
//! Two things a hold must not do, both found through a consumer whose chassis
//! assigns its velocity and steers its facing (#337). A body that TURNS over a
//! planted foot would swing that foot sideways through itself if the foot were
//! held to the world, so holds are carried round with every change of facing:
//! the stance keeps its shape in the body, and only the travel is held against
//! the world. And a body that travels much faster than the stride it is
//! walking, or in a direction it does not face, would leave a held foot
//! receding without limit — the crouch then sinks the body to reach it, and the
//! legs are forced apart — so each hold may stand only so far from the
//! stride's own answer, along the stride and across it. Past either bound the
//! foot yields continuously, dragged along the bound's edge: it slides by as
//! little as the bound allows, and nothing is re-planted, so nothing pops. Both
//! are traded against placement on purpose — the owner of that consumer asked
//! for smoothness over it — and neither touches a straight walk at the speed
//! changes the ledger was built for.
//!
//! # Warps
//!
//! A held point is only meaningful while the body is somewhere near it. A
//! teleport self-heals: a hold whose body-frame answer lands further from the
//! stride's own than the leg could ever reach is re-planted at the stride's
//! answer rather than lunged for. No continuous motion reaches that any more —
//! the bounds above yield long before it — so it answers for warps alone.
//! [`Footholds::reset`] exists for the caller that knows a warp happened and
//! wants no frame of doubt.

use crate::det::Rot;
use glam::Vec3;

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
    /// The facing the holds were last served under, or `None` before the first
    /// frame — kept so a body that turns carries its holds round with it.
    facing: Option<f32>,
}

/// How far along the stride a hold may stand from the stride's own answer
/// before the foot yields, as a share of the leg's reach.
///
/// **The bound that keeps #277 and stops the collapse** (#337). A change of
/// speed moves the stride's derivation out from under a planted foot along the
/// line of travel, and holding it there is the ledger's whole job: measured at
/// the consuming application's real chassis profile, walk speeds 1.4 <-> 0.7
/// m/s at 12/s, the stateless walk slides a sole 91.3 mm — 0.129 of the default
/// body's 709.5 mm reach — so the bound stands just clear of that. But the same
/// chassis starts a body from standing at up to 39 m/s^2 while the gait is
/// still built from a pace that has not caught up, and there the held foot
/// recedes behind the stride without limit: the crouch sinks the body to reach
/// it until `sink_needed` saturates at the hip's whole height, a pelvis 741.6
/// mm down one stance after the key.
///
/// Measured through the consumer's chassis (the replica in `tests/driver.rs`,
/// its stop-instrument body, a start from standing at a walk and a run),
/// against #277's own acceptance sweep:
///
/// ```text
///   bound       unbounded  0.25   0.20   0.15   0.10
///   start, mm   -741.6    -232.8 -196.0 -170.1 -156.3   (walk; steady bob -113.3)
///   start, mm   -735.3    -171.1 -142.1 -116.5 -100.8   (run;  steady bob  -64.4)
///   #277, mm       2.9       2.9    2.9    2.9   18.0   (sweep worst; control 2.4)
/// ```
///
/// 0.10 gives #277's sweep back most of the slide the ledger exists to remove;
/// 0.15 is the smallest bound tried that leaves it exactly where it was.
const HOLD_ALONG: f32 = 0.15;

/// How far across the stride a hold may stand from the stride's own answer
/// before the foot yields, as a share of the limb's own lateral offset from
/// the body's midline.
///
/// **Tight, because no straight walk ever needs any** (#337). A change of
/// speed moves a planted foot's derivation only along the line of travel; a
/// gap across it comes from a body travelling in a direction it does not face,
/// which is the consuming application's facing lagging its velocity through
/// every turn it makes — and a hold kept there reads as legs forced apart, up
/// to 297.9 mm on a quarter turn at a walk, where the owner of that app asked
/// for smoothness over placement. Half the limb's own offset lets each foot
/// stand at most a quarter of the stance's width out of line: 25.8 mm through
/// the same quarter turn on that body.
const HOLD_ACROSS: f32 = 0.5;

/// An angle folded into `(-PI, PI]`, so a turn is carried the short way round.
fn wrapped(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let folded = (angle + PI).rem_euclid(TAU) - PI;
    if folded <= -PI { folded + TAU } else { folded }
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
        self.facing = None;
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
            let world = at + Rot::y(facing) * body;
            Vec3::new(world.x, 0.0, world.z)
        };
        let into_body = |world: Vec3| Rot::y(-facing) * (world - at);

        // **A body that turns carries its planted feet round with it** (#337).
        // A hold served at its world point through a yaw swings the foot
        // sideways in the body by `2 * |offset| * sin(yaw / 2)` — tenths of a
        // metre over a quarter turn, which no guard below is sized for — and a
        // stride that is not told about the turn keeps walking straight ahead
        // under it. So the holds are turned about the body by whatever it
        // turned since the last frame: the stance keeps its shape in the body,
        // and the travel alone is held against the world. A straight walk never
        // turns, so this costs every straight-line hold nothing.
        if let Some(last) = self.facing {
            let turned = wrapped(facing - last);
            if turned != 0.0 {
                let pivot = Vec3::new(at.x, 0.0, at.z);
                let turn = Rot::y(turned);
                for (_, hold) in &mut self.held {
                    if let Some(point) = hold {
                        *point = pivot + turn * (*point - pivot);
                    }
                }
            }
        }
        self.facing = Some(facing);

        // The stride's own axes, which the two bounds below are taken along.
        let along = Vec3::new(stride.direction.x, 0.0, stride.direction.z)
            .try_normalize()
            .unwrap_or(Vec3::Z);
        let across = Vec3::new(along.z, 0.0, -along.x);

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
                    let gap = served - derived;
                    // The self-heal: a hold further from the stride's own
                    // answer than the leg is long is a warp's leftover, not a
                    // stance — no legitimate speed change moves the two apart
                    // by more than a fraction of a stride.
                    if gap.length() > reach {
                        *hold = Some(into_world(home + derived));
                        derived
                    } else {
                        // **The yield** (#337): past either bound the foot
                        // gives, dragged along the bound's edge rather than
                        // re-planted, so the slide is as small as the bound
                        // allows and nothing pops. The world point moves with
                        // it, so the next frame holds from where it gave.
                        let (way, side) = (gap.dot(along), gap.dot(across));
                        let bound_along = reach * HOLD_ALONG;
                        let bound_across = home.x.abs() * HOLD_ACROSS;
                        let kept = (
                            way.clamp(-bound_along, bound_along),
                            side.clamp(-bound_across, bound_across),
                        );
                        if kept == (way, side) {
                            served
                        } else {
                            let yielded = derived + along * kept.0 + across * kept.1;
                            *hold = Some(into_world(home + yielded));
                            yielded
                        }
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

    /// The widest any contact joint of `a` stands from the same joint of `b`.
    fn apart(rig: &Rig, a: &Pose, b: &Pose) -> f32 {
        let (a, b) = (a.forward(rig), b.forward(rig));
        [Limb::HindLeft, Limb::HindRight]
            .into_iter()
            .map(|limb| {
                let foot = rig.in_zone(Zone::Extremity(limb))[0];
                a.positions[foot].distance(b.positions[foot])
            })
            .fold(0.0f32, f32::max)
    }

    #[test]
    fn a_steady_walk_that_turns_is_the_stateless_walk() {
        // **#337.** A body turning over a planted foot, the foot held at its
        // world point, sees that foot swing sideways through the body by
        // `2 * |offset| * sin(yaw / 2)` — legs forced apart through every turn,
        // which is what the consuming app's owner saw. Carried round with the
        // body instead, the stance keeps its shape: at a constant speed along
        // a steady curve the held walk is the stateless walk again, exactly as
        // it is on a straight line. The body here steps along the facing it
        // had, then turns — the order in which one frame's travel and one
        // frame's turn compose with no remainder.
        //
        // Measured over two seconds of turning at 1.5 rad/s: the held feet
        // stood 90.6 mm from the stateless walk's on the ledger as shipped at
        // 0.7.0, and 0.6 mm carried round.
        let rig = rig();
        let speed = Speed::new(&rig, 1.4);
        let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));
        let cadence = speed.cadence(&rig);
        let dt = 1.0 / 60.0;
        let mut ledger = Footholds::new();
        let (mut at, mut facing) = (Vec3::ZERO, 0.0f32);
        let mut worst = 0.0f32;
        for frame in 0..120 {
            let cycle = (cadence * frame as f32 * dt).rem_euclid(1.0);
            if frame > 0 {
                at += Rot::y(facing) * Vec3::Z * (1.4 * dt);
                facing += 1.5 * dt;
            }
            let mut stateless = Pose::rest(&rig);
            Walk::at(cycle).drive(&rig, &mut stateless, &gait, &stride, flat);
            let mut held = Pose::rest(&rig);
            ledger.drive(
                Walk::at(cycle),
                &rig,
                &mut held,
                &gait,
                &stride,
                at,
                facing,
                flat,
            );
            worst = worst.max(apart(&rig, &stateless, &held));
        }
        println!(
            "turning steady walk: held feet {:.1} mm from the stateless walk's",
            worst * 1000.0
        );
        assert!(
            worst < 1e-3,
            "a steady walk along a curve held its feet {:.1} mm from the stateless walk's — \
             the ledger is holding the turn against the world",
            worst * 1000.0
        );
    }

    #[test]
    fn a_body_outrunning_its_planted_foot_is_not_pulled_to_the_floor() {
        // **#337.** The stride is built for 0.7 m/s and the body travels at
        // 1.6: a start from standing, where a chassis that assigns its velocity
        // outruns a gait built from a pace that has not caught up yet. Held to
        // its world point, a planted foot recedes past the leg's horizontal
        // reach, and the crouch that must cover an anchored stance sinks the
        // body toward the hip's whole height. Bounded, the foot yields and the
        // crouch stays near the stride's own.
        //
        // Measured, deepest crouch over the stateless walk's (the same stride,
        // no ledger): 779.2 mm on the ledger as shipped at 0.7.0, 62.3 mm
        // bounded. The ceiling sits between the two.
        let rig = rig();
        let slow = Speed::new(&rig, 0.7);
        let (gait, stride) = (slow.gait(&rig), slow.stride(&rig));
        let cadence = slow.cadence(&rig);
        let dt = 1.0 / 60.0;
        let mut ledger = Footholds::new();
        let mut excess = 0.0f32;
        for frame in 0..120 {
            let elapsed = frame as f32 * dt;
            let cycle = (cadence * elapsed).rem_euclid(1.0);
            let mut stateless = Pose::rest(&rig);
            let unheld = Walk::at(cycle).drive(&rig, &mut stateless, &gait, &stride, flat);
            let mut held = Pose::rest(&rig);
            let holding = ledger.drive(
                Walk::at(cycle),
                &rig,
                &mut held,
                &gait,
                &stride,
                Vec3::Z * (1.6 * elapsed),
                0.0,
                flat,
            );
            excess = excess.max(holding.steps.crouch - unheld.steps.crouch);
        }
        println!(
            "outrun stance: {:.1} mm deeper than the stride's own crouch",
            excess * 1000.0
        );
        assert!(
            excess < 0.15,
            "outrun by its body, a held stance crouched {:.1} mm deeper than the stride's own \
             — the hold is dragging the body to the floor",
            excess * 1000.0
        );
    }

    #[test]
    fn a_hold_across_the_stride_yields_at_its_bound() {
        // **#337.** A body travelling in a direction it does not face — the
        // consuming app's chassis while its facing catches up with a turn —
        // leaves a planted foot receding sideways through the body, and a hold
        // kept there is legs forced apart. No straight walk ever opens a gap
        // across the stride, so the bound there is tight: the foot yields at
        // half its own lateral offset and never stands further from the
        // stride's answer than that. Measured: 495.1 mm across on the ledger as
        // shipped at 0.7.0; 43.2 mm bounded, against a 43.1 mm bound.
        let rig = rig();
        let speed = Speed::new(&rig, 1.0);
        let (gait, stride) = (speed.gait(&rig), speed.stride(&rig));
        let cadence = speed.cadence(&rig);
        let dt = 1.0 / 60.0;
        let mut ledger = Footholds::new();
        let mut widest = 0.0f32;
        let mut bound = 0.0f32;
        for frame in 0..120 {
            let elapsed = frame as f32 * dt;
            let cycle = (cadence * elapsed).rem_euclid(1.0);
            let mut stateless = Pose::rest(&rig);
            Walk::at(cycle).drive(&rig, &mut stateless, &gait, &stride, flat);
            let mut held = Pose::rest(&rig);
            // Facing +Z, travelling +X: the whole of the travel is across.
            ledger.drive(
                Walk::at(cycle),
                &rig,
                &mut held,
                &gait,
                &stride,
                Vec3::X * (1.0 * elapsed),
                0.0,
                flat,
            );
            let (a, b) = (stateless.forward(&rig), held.forward(&rig));
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let foot = rig.in_zone(Zone::Extremity(limb))[0];
                widest = widest.max((a.positions[foot].x - b.positions[foot].x).abs());
                bound =
                    bound.max(home_of(&rig, limb).map_or(0.0, |home| home.x.abs()) * HOLD_ACROSS);
            }
        }
        println!(
            "sideways travel: widest {:.1} mm across, bound {:.1} mm",
            widest * 1000.0,
            bound * 1000.0
        );
        // The contact joint answers to its goal through a leg the settle also
        // moves, so the reading gets a millimetre and a half on the bound.
        assert!(
            widest < bound + 0.0015,
            "a foot the body travelled sideways over stood {:.1} mm across from the stride's \
             answer, past the {:.1} mm bound",
            widest * 1000.0,
            bound * 1000.0
        );
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
