//! # symbios-avatar
//!
//! **Parametric humanoid and creature bodies, generated entirely from code.**
//!
//! symbios-avatar grows game-ready character geometry from small parameter
//! records — no base mesh, no shipped assets, no third-party model licences.
//! It is the engine-agnostic half of the pair; `bevy_symbios_avatar` binds it
//! to Bevy.
//!
//! ## Pipeline
//!
//! ```text
//! Skeleton  ──►  control cage  ──►  Catmull-Clark  ──►  render mesh
//! (capsules)     (quad-dominant)    (smooth, all-quad)
//! ```
//!
//! The [`Skeleton`] is a graph of key balls. [`build_cage`] sweeps four-sided
//! rings along its limbs and hulls its joints into a closed, quad-dominant
//! control cage with edge loops that follow the body. [`catmull_clark`] then
//! smooths that cage into the surface a character actually deforms with.
//!
//! Humanoids and creatures are the same machinery: only the graph differs.
//!
//! ## Example
//!
//! ```rust
//! use symbios_avatar::{CageConfig, build_cage, catmull_clark, demo};
//!
//! let skeleton = demo::humanoid();
//! let cage = build_cage(&skeleton, &CageConfig::default())?;
//! let body = catmull_clark(&cage, 2);
//!
//! // The cage — and every subdivision of it — is a closed 2-manifold.
//! assert!(cage.is_closed_manifold());
//! assert!(body.is_closed_manifold());
//! assert_eq!(body.quad_fraction(), 1.0);
//! # Ok::<(), symbios_avatar::CageError>(())
//! ```
//!
//! ## Design notes
//!
//! * **Joints are the hard part.** Where three or more limbs meet, the surface
//!   is built by hulling the limbs' socket rings and deleting the socket facets
//!   to leave openings the tubes plug into. Because the rings are *shared*, the
//!   result is watertight by construction rather than by stitching. See
//!   [`cage::build_cage`] for the two degeneracies that must be handled: flat
//!   socket fans and buried sockets.
//! * **Everything is deterministic.** The same skeleton always yields the same
//!   vertex layout, so a record round-trips to identical geometry and downstream
//!   caches stay valid.
//! * **Errors point at the skeleton.** [`CageError::SocketsOverlap`] and
//!   [`CageError::SocketNotOnHull`] name the limbs involved, because the fix is
//!   nearly always to widen a joint or spread its limbs rather than to retune
//!   the mesher.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod anim;
pub mod cage;
pub mod demo;
pub mod face;
pub mod hair;
pub mod hull;
pub mod mesh;
pub mod plan;
pub mod prim;
pub mod record;
pub mod rig;
pub mod skeleton;
pub mod subdiv;
pub mod texture;
pub mod uv;

pub use anim::{
    Clip, Footing, FootingConfig, Gait, Gaze, GazeConfig, Ground, Inertializer, Key, Pose, Posed,
    Stride, Target, Track, look_at, plant_feet,
};
pub use cage::{CageConfig, CageError, build_cage};
pub use face::{Blink, EyeParams, Eyes};
pub use hair::{Hair, HairParams, Scalp, Strand};
pub use hull::{HullError, MAX_HULL_POINTS, convex_hull};
pub use mesh::{ManifoldReport, PolyMesh};
pub use plan::{
    Archetype, BodyPlan, Category, HumanoidParams, Limb, QuadrupedParams, Zone, ZoneSet,
};
pub use record::{AvatarRecord, LockSet, ProfileRecord, ShareCodeError};
pub use rig::{
    Anchor, Joint, Landmark, Landmarks, Rig, RigError, SkinConfig, SkinWeights, Surface,
};
pub use skeleton::{Chain, Node, NodeKind, Skeleton, SkeletonError};
pub use subdiv::catmull_clark;
pub use texture::{AtlasGeometry, SkinParams, bake_geometry, paint_skin};
pub use uv::{Chart, UvConfig, UvUnwrap, unwrap};

pub use glam::{Vec2, Vec3};
