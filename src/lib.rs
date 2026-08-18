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
//! The [`Skeleton`] is a graph of key balls. [`build_cage`] sweeps eight-point
//! rings along its limbs and hulls its joints into a closed, quad-dominant
//! control cage with edge loops that follow the body. [`catmull_clark`] then
//! smooths that cage into the surface a character actually deforms with.
//!
//! Humanoids and creatures are the same machinery: only the graph differs.
//!
//! Every stage is public, but [`Avatar::build`] is the one recipe: it takes an
//! [`AvatarRecord`] and returns merged skinned meshes grouped by material, a
//! painted skin atlas, the rig they are bound to, and the bill ([`Budget`]).
//!
//! ## Example
//!
//! ```rust
//! use symbios_avatar::{BODY_SUBDIVISIONS, CageConfig, build_cage, catmull_clark, demo};
//!
//! // A mesher fixture with no zones — see [`demo`]. For a finished body —
//! // face, skin atlas, rig, hair, outfit — build an [`Avatar`] from an
//! // [`AvatarRecord`] instead; `Avatar::build` is the one recipe.
//! let skeleton = demo::humanoid();
//! let cage = build_cage(&skeleton, &CageConfig::default())?;
//! let body = catmull_clark(&cage, BODY_SUBDIVISIONS);
//!
//! // The cage — and every subdivision of it — is a closed 2-manifold.
//! assert!(cage.is_closed_manifold());
//! assert!(body.is_closed_manifold());
//! assert_eq!(body.quad_fraction(), 1.0);
//! # Ok::<(), symbios_avatar::CageError>(())
//! ```
//!
//! ## Cargo features
//!
//! Both are off by default:
//!
//! * **`builtin-clips`** embeds the baked reference clips (`assets/clips.bin`,
//!   ~200 KiB) and turns on `ClipLibrary::builtin`. A development aid: the
//!   baked set is a comparison reference rather than a runtime motion source,
//!   and a consumer that would rather fetch the file at run time reads it with
//!   [`ClipLibrary::read`] and pays nothing here.
//! * **`serde-avatar`** makes a built [`Avatar`] serialisable, so one can
//!   cross a process or worker boundary. A round-tripped avatar is drawable,
//!   not rebuildable: it keeps its measured surface, its eyes and its
//!   handedness, and drops the intermediates the build was made from.
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
//!
//! ## Where the numbers come from
//!
//! This crate ships no data, only constants: profile tables, facial canons and
//! body coefficients, several hundred numbers that together decide what a body
//! looks like. Each is tagged in its own docstring with one of four
//! provenances, and the tags are worth reading before changing anything.
//!
//! * **Looked up** — a published proportion, with the source named. There are
//!   few of these and they are the only numbers that mean anything outside this
//!   crate.
//! * **Derived** — computed from another constant, with the arithmetic written
//!   out so it can be re-run when what it depends on moves. A derived constant
//!   that does not show its working is indistinguishable from a guess.
//! * **Tuned by render** — chosen by building the body and looking at it.
//!   This is clean provenance, not an admission: a number honestly labelled
//!   this way is one nobody will later mistake for a measurement.
//! * **Unsourced** — carried from an early implementation and never checked
//!   against anything. Most of [`HumanoidParams`] is in this state.
//!
//! **The failure this exists to prevent is two wrong numbers that agree.** The
//! face's `FIFTH` was once calibrated against a face the same file recorded as
//! 16% too wide, with `PUPIL = 1.0` silently absorbing the error; both read as
//! correct and their agreement was a coincidence, and it took a test failure to
//! find. Nothing about either number said which had been measured and
//! which had been fitted to it. Neither is *licence* provenance, which is the
//! other reason this matters: a crate that claims to ship no encumbered data
//! should be able to say where every number in it came from.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The README's code blocks, compiled and run as doctests so they cannot
/// drift from the API the way un-checked examples always eventually do.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

pub mod anim;
pub mod avatar;
pub mod cage;
pub mod demo;
pub mod dress;
pub mod extremity;
pub mod face;
pub mod gltf;
pub mod hair;
pub mod hull;
pub mod mesh;
pub mod plan;
pub mod prim;
pub mod record;
pub mod retarget;
pub mod rig;
pub mod skeleton;
pub mod subdiv;
pub mod texture;
pub mod torso;
pub mod uv;

pub use anim::{
    Clip, ClipLibrary, Continuity, Curve, Footing, FootingConfig, Gait, Gaze, GazeConfig, Ground,
    Heading, Idle, IdleConfig, Idled, Inertializer, JointTrack, Key, Leap, Leapt, LibraryError,
    Play, Pose, PoseClip, Posed, Scale, Slot, Space, Speed, Stride, Swim, Swum, Target, Track,
    Turn, Walk, Walked, contacts_in, look_at, plant_feet,
};
pub use avatar::{Avatar, AvatarConfig, AvatarMesh, Budget, MeshKind, Parts};
pub use cage::{CageConfig, CageError, build_cage};
pub use dress::{Garment, GarmentCut, Leg, Outfit, OutfitParams, Sleeve};
pub use extremity::{Attached, Extremities, Foot, Hand};
pub use face::{
    Aperture, Blink, Canon, Expression, EyeParams, Eyes, FaceParams, Features, HeadTraits,
    SaccadeConfig, Saccaded, Saccades, Talk, TalkConfig, Viseme, refine_face, shape_skull,
};
pub use hair::{
    BrowStyle, ChinStyle, Cut, FlankStyle, Follicle, FollicleParams, Follicles, Growth, HairRecord,
    MoustacheStyle, Paint, PaintedHair, ScalpStyle, Tress,
};
pub use hull::{HullError, MAX_HULL_POINTS, convex_hull};
pub use mesh::{ManifoldReport, PolyMesh};
pub use plan::{
    Archetype, BodyPlan, Category, Composites, HumanoidParams, Limb, QuadrupedParams, Zone, ZoneSet,
};
pub use record::{AvatarRecord, GENERATOR_VERSION, LockSet, ProfileRecord, ShareCodeError};
pub use rig::{
    Anchor, Footprint, Influence, Joint, Landmark, Landmarks, MAX_INFLUENCES, Patch, Rig, RigError,
    Role, SkinConfig, SkinWeights, Socket, Surface,
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
/// four times the triangles everywhere, most of them on a shin. Refining the
/// face alone buys the millimetre cells the features need and leaves the rest
/// of the body exactly as it was.
///
/// Measured across the four bands the features occupy — brow, eye, nose, mouth —
/// as the median edge of a face on the front of the head:
///
/// ```text
///  0 passes   25.9  28.2  26.6  26.6 mm
///  1 pass     13.2  14.1  14.1  13.7 mm
///  2 passes    6.6   7.1   7.2   7.3 mm
///  3 passes    3.3   3.6   3.6   3.6 mm
///  4 passes    3.3   3.6   3.6   1.8 mm
/// ```
///
/// Later passes cover only the band whose feature needs them — see
/// [`face::skull`]'s `FACE_PASSES` — which is why only the last column halves
/// at the fourth. Each pass is there because a feature is measured to need it:
///
/// * **The third pass is what makes a nose a nose.** A nostril crease is about
///   5 mm wide and simply cannot exist in 7 mm cells; at 3.6 mm the same
///   relief field, unchanged, comes out with a bridge, a tip and a wing. The
///   alternative was exaggerating the amplitude until the feature read through
///   a coarse surface, and the stylisation target rules that out.
/// * **The fourth and fifth are what make a mouth a mouth.** The narrow terms
///   of the lip field — the lip line's groove, the sulcus, the lobes — sit
///   between one and two cells at 3.6 mm, and a Gaussian about one cell wide
///   renders as a single displaced row of vertices, which is a bar. The mouth
///   band needs its cells under half the narrowest term, and the cage's ring
///   spacing under the face (set by the head's length below its joint) is what
///   these two passes are correcting for.
/// * **The sixth and seventh are the jaw flank, and they are the cheapest
///   passes here by an order of magnitude.** Every pass above reaches from
///   dead ahead round to a cosine, so widening one to take in the angle of the
///   jaw would pay for another refinement of a nose as well. Past about 57°
///   from dead ahead the lower face would otherwise stay at the base
///   subdivision — 24 mm cells against 1.8 mm on the front — and half the
///   mandible's border lives out there, where its 5 mm knee would be a fifth
///   of a cell. Giving a pass a near AND a far cosine lets these two take the
///   strip alone: they cost 652 triangles between them and quarter the cells
///   they cover.
/// * **The ninth is the nose's dorsum.** Every pass after the third stops
///   below the nose base, and a nose sampled once between its ridge and its
///   shoulder is a tent with a crease down it. This one reaches from the nose
///   base pair's own ceiling to the root of the nose, at a cosine of 0.97 —
///   twice the reach the feature needs — and costs 382 triangles on the
///   default body and 548 at the dearest corner of `tests/budget.rs`'s sweep.
/// * **The tenth is the front of the mandible's border**, the strip between
///   33° and 57° that no other band covers, where the crease's knee otherwise
///   scallops at cell pitch. It was judged worth its price on an A/B render
///   sheet, and it is the dearest pass in the table — 1,340 on the default
///   body, 2,848 at the dearest sweep corner — which `tests/budget.rs`'s
///   ratchet carries.
const FACE_REFINEMENT: usize = 10;

/// How many Catmull-Clark passes a body's cage gets.
///
/// One constant rather than a literal at every call site, because **anything
/// measuring the body's surface has to build it the way the body ships**: a
/// test that builds at a level of its own measures a head nobody renders, and
/// goes on passing when the shipped level moves underneath it.
///
/// The value is one because the cage's rings are eight-pointed, and the pair
/// moves together or not at all. Ring size governs how far a control cage sits
/// outside the surface it approximates, and an eight-point cage sits close
/// enough to its limit surface that a second pass is a smoothing of something
/// already smooth. Measured on the default body: 24,776 triangles at four
/// points and two passes, 43,196 at eight and two, and **15,862 at eight and
/// one**.
pub const BODY_SUBDIVISIONS: usize = 1;

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
    traits: &face::HeadTraits,
) -> Result<PolyMesh, CageError> {
    let cage = build_cage(skeleton, config)?;
    let mut mesh = catmull_clark(&cage, subdivisions);
    if let Ok(rig) = Rig::from_skeleton(skeleton) {
        // Resolution first, then shape. `refine_face` only adds vertices and
        // moves none, so `shape_skull` maps all of them onto the skull together
        // and the face is sampled finely rather than subdivided after the fact.
        mesh = face::refine_face(&mesh, &rig, FACE_REFINEMENT);
        mesh = face::refine_neck(&mesh, &rig, traits);
        face::shape_skull(&mut mesh, &rig, traits);
        // And then the column under it, which spans the junction the skull's
        // own shaping stops at. Second because it measures the surface the
        // skull left: see `face::neck`, whose whole argument is that the neck's
        // width has to be the head's business rather than the cage's.
        // The fairing over everything the systems above drew, which is where
        // the mandible-to-throat band stops being their seams (#193) — and it
        // runs BEFORE the column's own narrowing, not after. Fairing is
        // curvature removal and a waist IS curvature: run last, it relaxed the
        // waist `shape_neck` had just cut by 3.3 mm at the small-head corner
        // and took that body's neck-to-skull ratio from 0.803 to 0.864, which
        // is the knob-on-a-post `face::neck`'s whole module docstring is about.
        // Faired first, the narrowing measures the smooth column and cuts its
        // waist into a surface that has no seams left to sharpen.
        face::fair_neck(&mut mesh, &rig, traits);
        face::shape_neck(&mut mesh, &rig, traits);
    }
    Ok(mesh)
}
pub use texture::{AtlasGeometry, Condition, SkinParams, bake_geometry, paint_skin};
pub use uv::{Chart, UvConfig, UvUnwrap, unwrap};

/// The vector and rotation types every public signature here is written in.
///
/// `Quat` is one of them because [`Pose::rotations`] is a public `Vec<Quat>`:
/// without this a consumer can be handed a pose and read it, but cannot write
/// one without taking a direct dependency on the exact `glam` version this
/// crate resolved to.
///
/// [`Pose::rotations`]: anim::Pose::rotations
pub use glam::{Quat, Vec2, Vec3};
