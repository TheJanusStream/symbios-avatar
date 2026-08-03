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
pub mod avatar;
pub mod cage;
pub mod demo;
pub mod dress;
pub mod extremity;
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
pub use avatar::{Avatar, AvatarConfig, AvatarMesh, Budget, MeshKind, Parts};
pub use cage::{CageConfig, CageError, build_cage};
pub use dress::{Garment, GarmentCut, Leg, Outfit, OutfitParams, Sleeve};
pub use extremity::{Attached, Extremities, Foot, Hand};
pub use face::{Blink, Canon, EyeParams, Eyes, FaceParams, Features, refine_face, shape_skull};
pub use hair::{Hair, HairParams, Scalp, Strand};
pub use hull::{HullError, MAX_HULL_POINTS, convex_hull};
pub use mesh::{ManifoldReport, PolyMesh};
pub use plan::{
    Archetype, BodyPlan, Category, HumanoidParams, Limb, QuadrupedParams, Zone, ZoneSet,
};
pub use record::{AvatarRecord, LockSet, ProfileRecord, ShareCodeError};
pub use rig::{
    Anchor, Joint, Landmark, Landmarks, Rig, RigError, Role, SkinConfig, SkinWeights, Surface,
};
pub use skeleton::{Chain, Node, NodeKind, Skeleton, SkeletonError};
pub use subdiv::catmull_clark;
/// The painted atlas an [`Avatar`] carries.
///
/// Re-exported because it is the type of a public field and a consumer cannot
/// otherwise name it without depending on `symbios-texture` at exactly the
/// version this crate pins.
pub use symbios_texture::generator::TextureMap;

/// How many extra times the front of the head is split.
///
/// Not a subdivision level for the body: the whole body at one more level costs
/// four times the triangles everywhere, most of them on a shin. Two extra splits
/// of the face alone take its mean edge from 24 mm to about 6 mm, which is what
/// a 10 mm brow ridge needs to exist at all (#59), and leave the rest of the
/// body exactly as it was.
///
/// Measured across the four bands the features occupy — brow, eye, nose, mouth —
/// as the median edge of a face on the front of the head:
///
/// ```text
///  0 passes   25.9  28.2  26.6  26.6 mm
///  1 pass     13.2  14.1  14.1  13.7 mm
///  2 passes    6.6   7.1   7.2   7.3 mm
///  3 passes    3.3   3.6   3.6   3.6 mm
///  4 passes    3.3   3.6   3.6   1.8 mm   <- here
/// ```
///
/// The fourth pass covers the mouth band only — see [`face::skull`]'s
/// `FACE_PASSES` — which is why only the last column halves.
///
/// **The third pass is what makes a nose a nose.** Carved into 7 mm cells
/// (#59), the nose was a soft mound with no bridge, no tip and no wing — a
/// nostril crease is about 5 mm wide and simply cannot exist there. At 3.6 mm
/// the same field, unchanged, comes out with all three. The alternative was
/// exaggerating the amplitude until the feature read through a coarse surface,
/// and the owner's stylisation call rules that out by name.
///
/// **The fourth pass is what makes a mouth a mouth**, for the same reason and
/// with the arithmetic stated in advance this time. At 3.6 mm every term in the
/// lip field was about one cell wide — the lip line's groove 0.99, the sulcus
/// 1.29, the lobes 1.67 and 1.75 — and a Gaussian one cell wide renders as a
/// single displaced row of vertices, which is a bar. The prediction was that
/// halving the cell would remove the bars and keep the lips; it did (#85).
///
/// **The fifth pass is the fourth's again, and it is here because #78 took the
/// fourth one's margin away.** The cage lays one ring per node, so lengthening
/// the head below its joint from 0.69 radii to 1.19 spread the rings under the
/// face by the same 1.7 and the mouth's cells grew with them. Measured, the
/// narrowest lip term went from 2.0–3.5 cells to 1.43–1.76 — back under the 1.5
/// a Gaussian needs to survive sampling — and the bars came back on screen,
/// plainly, in the same place they were in #85. The mouth field was rebased to
/// hold its millimetres, so the terms did not shrink; the surface under them
/// coarsened.
///
/// It is affordable because the same stretch made the face refinement CHEAPER:
/// the bands are fixed heights in head radii, so a taller head puts less of
/// itself inside them. The body went 23,182 triangles to 20,668 on the stretch
/// alone, and this spends part of that back.
///
/// The three passes cost about 3,500 triangles and the fourth about 2,150,
/// measured on the default body before #78. The ceiling and every seed still
/// pass; what it moves is the balance, and that is recorded in
/// `tests/budget.rs`.
///
/// It sat at one for a long time for a reason that was nothing to do with cost:
/// the second pass moved the profile the ears are placed from, and one seed's
/// ear fell to 18% visible against a 25% floor. That was a defect in the
/// measurement rather than in the refinement, and it is fixed (#67).
const FACE_REFINEMENT: usize = 5;

/// Builds a body's surface from its skeleton, shaped and ready to bind.
///
/// The whole of it: cage, subdivision, the face refinement that gives a head
/// somewhere to put a face, and the skull shaping that a capsule graph cannot
/// express. Kept as one call because the order matters and the
/// shaping has to happen before anything is bound or unwrapped — do it after
/// and the skin weights, the texture charts and every attached part are fitted
/// to a head that no longer exists.
///
/// # Errors
///
/// Returns [`CageError`] if the skeleton cannot be meshed.
pub fn build_body(
    skeleton: &Skeleton,
    config: &CageConfig,
    subdivisions: usize,
) -> Result<PolyMesh, CageError> {
    let cage = build_cage(skeleton, config)?;
    let mut mesh = catmull_clark(&cage, subdivisions);
    if let Ok(rig) = Rig::from_skeleton(skeleton) {
        // Resolution first, then shape. `refine_face` only adds vertices and
        // moves none, so `shape_skull` maps all of them onto the skull together
        // and the face is sampled finely rather than subdivided after the fact.
        mesh = face::refine_face(&mesh, &rig, FACE_REFINEMENT);
        face::shape_skull(&mut mesh, &rig);
    }
    Ok(mesh)
}
pub use texture::{AtlasGeometry, SkinParams, bake_geometry, paint_skin};
pub use uv::{Chart, UvConfig, UvUnwrap, unwrap};

pub use glam::{Vec2, Vec3};
