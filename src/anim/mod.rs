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
//! * [`Leap`] — jumping, falling and landing, which are one motion: the leg is
//!   a spring and the body is a projectile, so the wind-up, the flight and the
//!   landing all follow from a single dimensionless stiffness and cannot
//!   disagree at the seams.
//! * [`Speed`] — one dimensionless axis, the Froude number, from which the
//!   stride, the cadence, the duty and the choice of walking or running all
//!   follow. A caller says how fast the body is going and stops choosing the
//!   rest.
//! * [`Inertializer`] — transitions that carry momentum through, rather than
//!   crossfades that stall it — and [`transition`], the layer above it that
//!   decides when a transition is needed at all. Along the speed axis it is
//!   not: a walk becoming a run is one generator moving along one parameter.
//! * [`plant_feet`] — putting a body's contacts on whatever it is standing on,
//!   which is what makes it look like it is *in* a place rather than played back
//!   near one.
//! * [`gait`] — walking and running, for whatever number of legs a body turns
//!   out to have. [`Walk`] drives one frame of it start to finish; the stages
//!   underneath it stay public for anything that wants to run only some of
//!   them. A run is [`Gait::running`] or, better, whatever [`Speed::gait`] says
//!   at the speed in hand.
//! * [`Clip`] — authored motion, described by semantic queries and normalised
//!   goals so one description serves every body.
//! * [`PoseClip`] — the opposite trade, and it exists because the first one
//!   cannot be walked back: motion that was performed on a body carries joint
//!   angles no query recovers, so an imported clip keeps them. Addressed by
//!   [`Slot`] rather than by joint index, so one bake still meets many bodies.
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
pub mod leap;
pub mod library;
pub mod pose;
pub mod pose_clip;
pub mod speed;
pub mod spring;
pub mod transition;
pub mod turn;

pub use blend::Inertializer;
pub use clip::{Clip, Key, Space, Target, Track};
pub use gait::{Gait, Phase, Steps, Stride, Walk, Walked};
pub use gaze::{Gaze, GazeConfig, look_at};
pub use ground::{
    CONTACT_SLACK, CONTACT_SPEED, Footing, FootingConfig, Ground, contacts_during, contacts_in,
    level_feet, plant_feet, plant_feet_of,
};
pub use ik::{FabrikConfig, fabrik, two_bone};
pub use leap::{Leap, Leapt, Stage};
pub use library::{ClipLibrary, LibraryError};
pub use pose::{Pose, Posed};
pub use pose_clip::{Curve, JointTrack, Play, PoseClip, Slot};
pub use speed::Speed;
pub use spring::{SpringConfig, Springs};
pub use transition::{Entry, Family, Frames};
pub use turn::Turn;
