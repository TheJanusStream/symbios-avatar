//! Secondary motion: hair, hems, tails, ears, anything that follows late.
//!
//! A body that is entirely rigid outside its own animation reads as a puppet,
//! and the cheapest cure by a wide margin is a few chains of joints that lag
//! behind whatever they hang from. It is why VRM specifies spring bones at all,
//! and why every VTuber-adjacent pipeline has them.
//!
//! The vocabulary is **`VRMC_springBone`**'s — stiffness, drag, gravity, a chain
//! per dangling thing — which survives VRM itself being dropped from the export
//! plan, because it was chosen for describing the behaviour rather than for the
//! format. Two things here are deliberately *not* what that spec says, and both
//! are noted where they happen: the damping is framerate-independent, and
//! collision is against the body's own measured surface rather than against
//! hand-placed collider capsules.
//!
//! Nothing in this module knows what it is simulating. A chain is any run of
//! [`Role::Spring`] joints hanging off something that is not one, so hair, a
//! tail and a coat hem are the same code — which is the point of [`Role`]
//! existing at all.

use glam::Vec3;

use super::ik;
use super::pose::Pose;
use crate::rig::{Rig, Role, Surface};

/// Below this, a length or a direction is noise.
const EPSILON: f32 = 1e-5;

/// The longest step the simulation will take in one go, in seconds.
///
/// A spring integrated across a long frame overshoots, and an overshooting
/// spring does not settle — it oscillates wider until the chain is whipping. A
/// dropped frame is common enough that this is not a theoretical concern, so a
/// long step is subdivided rather than trusted.
const MAX_STEP: f32 = 1.0 / 60.0;

/// How far an anchor may move in one step before the chain is assumed to have
/// been teleported rather than to have moved, as a multiple of its own length.
///
/// Without this, putting a body somewhere else — a new region, a travel, a
/// re-seed — leaves its hair stretched between where it was and where it is,
/// snapping back over the following second. The chain is not being animated at
/// that point; it is being relocated, and the honest response is to relocate it.
const TELEPORT: f32 = 4.0;

/// How a spring chain behaves.
///
/// The defaults are tuned for hair, which is what there is most of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    /// How strongly a chain returns to the shape the animation put it in.
    ///
    /// This is a real spring toward the pose's own answer, not
    /// `VRMC_springBone`'s constant pull along the rest direction. A constant
    /// pull cannot settle: it is the same magnitude a hair's breadth from home
    /// as it is a hand's width away, so a chain arrives and then keeps going.
    pub stiffness: f32,
    /// How much of its speed a chain loses per second, as a fraction.
    ///
    /// Per *second*, applied as an exponential decay. The spec's `dragForce` is
    /// a per-frame multiplier, which silently makes the behaviour depend on the
    /// frame rate — the same hair is limp at 30 Hz and lively at 144 Hz. There
    /// is no reason to reproduce that.
    pub drag: f32,
    /// Acceleration on every joint of a chain, in metres per second squared.
    ///
    /// **Zero by default, which is not what the spec would do**, and the reason
    /// is worth knowing before turning it on. Gravity here is a real force, so
    /// it moves where a chain comes to rest: against the default stiffness even
    /// a gentle `1.2` sagged a four-link chain by 6.6 cm and left it there. That
    /// is correct physics and the wrong answer, because everything this crate
    /// generates is *already drooped* — a lock of hair is grown falling, with
    /// its own lean and its own clearance against the body. Adding gravity on
    /// top counts it twice and pulls the authored shape apart.
    ///
    /// Set it for a chain whose rest shape does not already hang: a tail
    /// authored straight out behind a body, an ear modelled upright. For hair,
    /// the swing comes from the lag and the drag, not from here.
    pub gravity: Vec3,
    /// How far clear of the body a chain is held, in metres.
    ///
    /// Only used when [`Springs::advance`] is given a surface to measure
    /// against. Hair that swings into the chest reads as a hole in the chest.
    pub clearance: f32,
}

impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            stiffness: 60.0,
            drag: 6.0,
            gravity: Vec3::ZERO,
            clearance: 0.01,
        }
    }
}

/// One run of spring joints, and what it hangs from.
#[derive(Clone, Debug, PartialEq)]
struct Chain {
    /// The anchor first, then the spring joints outward from it.
    ///
    /// The anchor is not simulated — it is whatever the animation says it is,
    /// which is exactly the thing the rest of the chain is lagging behind.
    joints: Vec<usize>,
    /// Rest distance between each pair, in metres.
    lengths: Vec<f32>,
}

/// The state a set of spring chains carries between frames.
///
/// Kept apart from the [`Rig`] because it is the one thing here that is not a
/// pure function of the pose: a spring's whole job is to remember where it was.
#[derive(Clone, Debug, PartialEq)]
pub struct Springs {
    chains: Vec<Chain>,
    /// Where each simulated joint is, and was, in world space. `None` until the
    /// first step, so a chain starts wherever the body starts rather than at
    /// the origin.
    at: Vec<Option<(Vec3, Vec3)>>,
}

impl Springs {
    /// Finds every spring chain in a rig.
    ///
    /// A chain starts at the first [`Role::Spring`] joint whose parent is not
    /// one, and runs outward. Branching is allowed and produces one chain per
    /// branch, which is what a head of hair is.
    #[must_use]
    pub fn of(rig: &Rig) -> Self {
        let sprung = |joint: usize| rig.joints[joint].role == Role::Spring;
        let mut chains = Vec::new();

        for start in 0..rig.len() {
            if !sprung(start) {
                continue;
            }
            let Some(anchor) = rig.joints[start].parent else {
                // A spring joint with no parent has nothing to hang from and
                // nothing to lag behind.
                continue;
            };
            if sprung(anchor) {
                continue;
            }

            // Outward until the chain stops or forks. A fork starts its own
            // chain at the branch, so the shared root is simulated once.
            let mut joints = vec![anchor, start];
            let mut here = start;
            loop {
                let children: Vec<usize> = (0..rig.len())
                    .filter(|&joint| rig.joints[joint].parent == Some(here) && sprung(joint))
                    .collect();
                let [only] = children[..] else { break };
                joints.push(only);
                here = only;
            }

            let lengths: Vec<f32> = joints
                .windows(2)
                .map(|pair| {
                    rig.joints[pair[0]]
                        .position
                        .distance(rig.joints[pair[1]].position)
                })
                .collect();
            if lengths.iter().any(|length| *length <= EPSILON) {
                continue;
            }
            chains.push(Chain { joints, lengths });
        }

        Self {
            chains,
            at: vec![None; rig.len()],
        }
    }

    /// How many chains were found.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chains.len()
    }

    /// Whether the rig has nothing that swings.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chains.is_empty()
    }

    /// Forgets where every chain was, so the next step starts from rest.
    ///
    /// What to call when a body is put somewhere else deliberately, rather than
    /// leaving the teleport threshold to notice.
    pub fn settle(&mut self) {
        self.at.iter_mut().for_each(|at| *at = None);
    }

    /// Advances every chain by `dt` and writes the result into `pose`.
    ///
    /// `surface` is the body the chains must not swing into; passing `None`
    /// simulates them in free space, which is right for a tail on the outside of
    /// a body and wrong for hair against a chest.
    ///
    /// Long steps are subdivided, so a dropped frame slows
    /// the simulation down rather than making it explode.
    pub fn advance(
        &mut self,
        rig: &Rig,
        pose: &mut Pose,
        surface: Option<&Surface>,
        dt: f32,
        config: &SpringConfig,
    ) {
        if self.chains.is_empty() || !dt.is_finite() || dt <= 0.0 || !pose.fits(rig) {
            return;
        }
        let steps = (dt / MAX_STEP).ceil().min(8.0);
        let step = dt / steps;
        for _ in 0..steps as usize {
            self.step(rig, pose, surface, step, config);
        }
    }

    /// One substep.
    fn step(
        &mut self,
        rig: &Rig,
        pose: &mut Pose,
        surface: Option<&Surface>,
        dt: f32,
        config: &SpringConfig,
    ) {
        // Where the animation alone would put everything. This is both the
        // anchor's truth and the shape the springs are pulled back toward, so a
        // chain that is not moving sits exactly where the pose says.
        let posed = pose.forward(rig);
        let keep = (-config.drag.max(0.0) * dt).exp();

        for chain in &self.chains {
            let anchored = posed.positions[chain.joints[0]];
            // A chain whose anchor jumped was moved, not animated.
            let span: f32 = chain.lengths.iter().sum();
            if let Some((was, _)) = self.at[chain.joints[1]]
                && was.distance(anchored) > span * TELEPORT
            {
                for &joint in &chain.joints[1..] {
                    self.at[joint] = None;
                }
            }

            let mut solved = vec![anchored];
            for index in 1..chain.joints.len() {
                let joint = chain.joints[index];
                let rigid = posed.positions[joint];
                let (previous, current) = self.at[joint].unwrap_or((rigid, rigid));

                // Verlet: the step it just took, damped, carries it into the
                // next one. This is where the lag comes from — nothing here
                // tells a chain to trail, it simply has not caught up yet.
                let carried = (current - previous) * keep;
                let pull = (rigid - current) * config.stiffness;
                let mut next = current + carried + (pull + config.gravity) * dt * dt;

                // A bone does not stretch.
                let from = solved[index - 1];
                let along = (next - from).normalize_or_zero();
                next = if along == Vec3::ZERO {
                    from + (rigid - from).normalize_or(Vec3::NEG_Y) * chain.lengths[index - 1]
                } else {
                    from + along * chain.lengths[index - 1]
                };

                // And it does not swing into the body it hangs off.
                if let Some(surface) = surface {
                    next += surface.clearance(rig, next, config.clearance);
                }

                self.at[joint] = Some((current, next));
                solved.push(next);
            }

            let original: Vec<Vec3> = chain
                .joints
                .iter()
                .map(|&joint| posed.positions[joint])
                .collect();
            ik::retarget(
                rig,
                pose,
                &chain.joints,
                &original,
                &solved,
                &posed.rotations,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, Zone};
    use glam::Quat;

    /// A humanoid with a four-link chain hanging off the head, of the kind a
    /// lock of hair or a long ear would want.
    fn sprung() -> (Rig, Vec<usize>) {
        let mut rig = Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs");
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let mut chain = Vec::new();
        let mut parent = head;
        for link in 1..=4 {
            let at = rig.joints[head].position + Vec3::new(0.06, -0.05 * link as f32, 0.0);
            parent = rig
                .attach(parent, at, Role::Spring)
                .expect("the head exists");
            chain.push(parent);
        }
        (rig, chain)
    }

    /// The world position of every joint of a pose.
    fn at(rig: &Rig, pose: &Pose) -> Vec<Vec3> {
        pose.forward(rig).positions
    }

    #[test]
    fn a_chain_is_found_from_its_roles_alone() {
        let (rig, chain) = sprung();
        let springs = Springs::of(&rig);
        assert_eq!(springs.len(), 1, "one chain hangs off the head");
        assert_eq!(springs.chains[0].joints.len(), chain.len() + 1);
        assert_eq!(
            springs.chains[0].joints[0],
            rig.joints[chain[0]].parent.expect("anchored"),
            "the anchor leads the chain and is not itself simulated"
        );
        assert!(
            Springs::of(&Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs"))
                .is_empty(),
            "a body with nothing dangling has no chains"
        );
    }

    #[test]
    fn a_still_body_leaves_its_chains_exactly_where_the_pose_put_them() {
        // The property everything else rests on. Secondary motion that drifts
        // when nothing is happening is not secondary motion, it is a bug with a
        // physical excuse.
        let (rig, chain) = sprung();
        let mut springs = Springs::of(&rig);
        let mut pose = Pose::rest(&rig);
        let rest = at(&rig, &pose);

        for _ in 0..120 {
            springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &SpringConfig::default());
        }
        let after = at(&rig, &pose);
        for &joint in &chain {
            assert!(
                after[joint].distance(rest[joint]) < 2e-3,
                "joint {joint} drifted {} m over two seconds of standing still",
                after[joint].distance(rest[joint])
            );
        }
    }

    #[test]
    fn a_chain_lags_behind_the_body_and_then_catches_up() {
        // What the module is for. Turning the head has to leave the hair
        // behind, and the hair has to arrive afterwards rather than never.
        let (rig, chain) = sprung();
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let tip = *chain.last().expect("a tip");
        let mut springs = Springs::of(&rig);

        let mut pose = Pose::rest(&rig);
        springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &SpringConfig::default());

        // Turn the head, hard.
        let mut turned = Pose::rest(&rig);
        turned.rotations[head] = Quat::from_rotation_y(1.2);
        let rigid = at(&rig, &turned)[tip];

        let mut sprung_pose = turned.clone();
        springs.advance(
            &rig,
            &mut sprung_pose,
            None,
            1.0 / 60.0,
            &SpringConfig::default(),
        );
        let lagged = at(&rig, &sprung_pose)[tip];
        assert!(
            lagged.distance(rigid) > 0.01,
            "the tip followed the head rigidly, by {} m",
            lagged.distance(rigid)
        );

        // And it arrives.
        for _ in 0..180 {
            let mut settling = turned.clone();
            springs.advance(
                &rig,
                &mut settling,
                None,
                1.0 / 60.0,
                &SpringConfig::default(),
            );
            sprung_pose = settling;
        }
        let arrived = at(&rig, &sprung_pose)[tip];
        assert!(
            arrived.distance(rigid) < 0.01,
            "the tip never caught up; it is {} m short after three seconds",
            arrived.distance(rigid)
        );
    }

    #[test]
    fn a_chain_never_stretches() {
        // A spring that solves positions has to be told bones do not stretch,
        // and it is the one constraint whose failure is instantly visible.
        let (rig, _) = sprung();
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let mut springs = Springs::of(&rig);
        let rest: Vec<f32> = springs.chains[0].lengths.to_vec();

        for frame in 0..90 {
            let mut pose = Pose::rest(&rig);
            // Shake it, so the solver is never near its comfortable answer.
            pose.rotations[head] = Quat::from_rotation_z((frame as f32 * 0.7).sin() * 1.1);
            springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &SpringConfig::default());

            let world = at(&rig, &pose);
            let joints = &springs.chains[0].joints;
            for index in 1..joints.len() {
                let length = world[joints[index - 1]].distance(world[joints[index]]);
                assert!(
                    (length - rest[index - 1]).abs() < 1e-3,
                    "frame {frame} link {index} measured {length} against {}",
                    rest[index - 1]
                );
            }
        }
    }

    #[test]
    fn gravity_pulls_a_chain_down() {
        let (rig, chain) = sprung();
        let tip = *chain.last().expect("a tip");
        let hang = |gravity: Vec3| {
            let mut springs = Springs::of(&rig);
            let mut pose = Pose::rest(&rig);
            for _ in 0..240 {
                let mut next = Pose::rest(&rig);
                springs.advance(
                    &rig,
                    &mut next,
                    None,
                    1.0 / 60.0,
                    &SpringConfig {
                        gravity,
                        ..Default::default()
                    },
                );
                pose = next;
            }
            at(&rig, &pose)[tip].y
        };
        assert!(
            hang(Vec3::new(0.0, -9.8, 0.0)) < hang(Vec3::ZERO) - 1e-3,
            "gravity should hang a chain lower than no gravity"
        );
    }

    #[test]
    fn drag_settles_a_chain_sooner() {
        // Drag is the axis that decides whether hair reads as hair or as rope,
        // so it had better do something monotonic.
        let (rig, chain) = sprung();
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let tip = *chain.last().expect("a tip");

        let swing = |drag: f32| {
            let mut springs = Springs::of(&rig);
            let config = SpringConfig {
                drag,
                ..Default::default()
            };
            let mut pose = Pose::rest(&rig);
            springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &config);

            let mut turned = Pose::rest(&rig);
            turned.rotations[head] = Quat::from_rotation_y(1.0);
            let rigid = at(&rig, &turned)[tip];
            // How far it still is from home after half a second.
            let mut last = turned.clone();
            for _ in 0..30 {
                let mut next = turned.clone();
                springs.advance(&rig, &mut next, None, 1.0 / 60.0, &config);
                last = next;
            }
            at(&rig, &last)[tip].distance(rigid)
        };
        assert!(
            swing(12.0) < swing(1.0),
            "more drag should be closer to home, not further"
        );
    }

    #[test]
    fn a_teleported_body_does_not_trail_its_hair_across_the_world() {
        // A body being *moved* is not a body being animated, and the difference
        // is invisible to a spring that only sees positions. Without this a
        // re-seed or a travel leaves hair stretched between two places.
        let (rig, chain) = sprung();
        let tip = *chain.last().expect("a tip");
        let mut springs = Springs::of(&rig);

        let mut pose = Pose::rest(&rig);
        springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &SpringConfig::default());

        let mut moved = Pose::rest(&rig);
        moved.translation = Vec3::new(120.0, 0.0, -80.0);
        let rigid = at(&rig, &moved)[tip];
        springs.advance(&rig, &mut moved, None, 1.0 / 60.0, &SpringConfig::default());
        let landed = at(&rig, &moved)[tip];
        assert!(
            landed.distance(rigid) < 0.01,
            "the chain trailed {} m behind a teleport",
            landed.distance(rigid)
        );
    }

    #[test]
    fn a_long_frame_does_not_explode_a_chain() {
        // A spring integrated across a long step overshoots, and an
        // overshooting spring oscillates wider rather than settling.
        let (rig, chain) = sprung();
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let mut springs = Springs::of(&rig);
        let mut pose = Pose::rest(&rig);
        pose.rotations[head] = Quat::from_rotation_y(1.4);

        for _ in 0..20 {
            let mut next = Pose::rest(&rig);
            next.rotations[head] = Quat::from_rotation_y(1.4);
            // A third of a second: twenty frames' worth, in one.
            springs.advance(&rig, &mut next, None, 0.33, &SpringConfig::default());
            pose = next;
        }
        let world = at(&rig, &pose);
        for &joint in &chain {
            assert!(
                world[joint].is_finite(),
                "joint {joint} left the world at {:?}",
                world[joint]
            );
        }
        let anchor = world[rig.joints[chain[0]].parent.expect("anchored")];
        assert!(
            world[*chain.last().expect("a tip")].distance(anchor) < 1.0,
            "the chain whipped out to {} m from its anchor",
            world[*chain.last().expect("a tip")].distance(anchor)
        );
    }

    #[test]
    fn settling_forgets_where_a_chain_was() {
        let (rig, chain) = sprung();
        let mut springs = Springs::of(&rig);
        let mut pose = Pose::rest(&rig);
        springs.advance(&rig, &mut pose, None, 1.0 / 60.0, &SpringConfig::default());
        assert!(springs.at[chain[0]].is_some());
        springs.settle();
        assert!(springs.at.iter().all(Option::is_none));
    }

    #[test]
    fn simulation_is_deterministic() {
        let (rig, _) = sprung();
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let run = || {
            let mut springs = Springs::of(&rig);
            let mut pose = Pose::rest(&rig);
            for frame in 0..40 {
                let mut next = Pose::rest(&rig);
                next.rotations[head] = Quat::from_rotation_x((frame as f32 * 0.3).sin());
                springs.advance(&rig, &mut next, None, 1.0 / 60.0, &SpringConfig::default());
                pose = next;
            }
            at(&rig, &pose)
        };
        assert_eq!(run(), run());
    }
}
