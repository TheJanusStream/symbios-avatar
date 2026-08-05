//! Making a body move.
//!
//! Motion here is described by **goals** rather than by joint angles, because a
//! joint angle bakes in the skeleton it was authored on. A body whose
//! proportions come from a record has no fixed skeleton, so the only description
//! that survives is one phrased in terms of what the body is *doing*: this foot
//! is on the ground at this place, this hand is on that handle. Solvers turn
//! those goals into a pose for whatever body is actually present.
//!
//! Three layers, each usable on its own:
//!
//! * [`Pose`] — local rotations per joint, resolved to world space by forward
//!   kinematics and applied to geometry by linear blend skinning.
//! * [`ik`] — [`ik::two_bone`] for limbs, which has a closed form, and
//!   [`ik::fabrik`] for spines and tails, which does not.
//! * [`Inertializer`] — transitions that carry momentum through, rather than
//!   crossfades that stall it.
//! * [`plant_feet`] — putting a body's contacts on whatever it is standing on,
//!   which is what makes it look like it is *in* a place rather than played back
//!   near one.
//! * [`gait`] — walking, for whatever number of legs a body turns out to have.
//! * [`Clip`] — authored motion, described by semantic queries and normalised
//!   goals so one description serves every body.
//! * [`look_at`] — turning a body toward something, shared down the chain from
//!   the torso rather than swivelled by the skull alone.
//! * [`Springs`] — the hair, hems and tails that follow late, which is most of
//!   what stops a body reading as a puppet.
//!
//! ```rust
//! use symbios_avatar::{
//!     AvatarRecord, Limb, Rig, Zone,
//!     anim::{Pose, ik},
//! };
//!
//! let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;
//! let mut pose = Pose::rest(&rig);
//!
//! // Reach with the left arm: shoulder, elbow, wrist.
//! let upper = rig.in_zone(Zone::UpperLimb(Limb::ForeLeft));
//! let chain = [upper[0], upper[1], rig.in_zone(Zone::LowerLimb(Limb::ForeLeft))[0]];
//!
//! let shoulder = pose.forward(&rig).positions[chain[0]];
//! let target = shoulder + symbios_avatar::Vec3::new(-0.2, -0.15, 0.15);
//! assert!(ik::two_bone(&rig, &mut pose, chain, target, target + symbios_avatar::Vec3::Z));
//!
//! let reached = pose.forward(&rig).positions[chain[2]];
//! assert!(reached.distance(target) < 1e-3);
//! # Ok::<(), symbios_avatar::RigError>(())
//! ```

pub mod blend;
pub mod clip;
pub mod dual;
pub mod gait;
pub mod gaze;
pub mod ground;
pub mod ik;
pub mod pose;
pub mod spring;

pub use blend::Inertializer;
pub use clip::{Clip, Key, Target, Track};
pub use gait::{Gait, Phase, Steps, Stride};
pub use gaze::{Gaze, GazeConfig, look_at};
pub use ground::{Footing, FootingConfig, Ground, level_feet, plant_feet, plant_feet_of};
pub use ik::{FabrikConfig, fabrik, two_bone};
pub use pose::{Pose, Posed};
pub use spring::{SpringConfig, Springs};
