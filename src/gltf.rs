//! Reading a glTF binary far enough to sample a skinned animation and to
//! measure a body, and no further.
//!
//! This began with one question, and the question was most of the
//! specification: **where is each of a skin's joints, at time `t` of animation
//! `n`?** The motion library it was written for
//! ([#102](https://github.com/TheJanusStream/symbios-avatar/issues/102)) is
//! 162 clips of skeletal animation and a skeleton to hang them on, and nothing
//! else in those files was wanted.
//!
//! It now answers a second: **what shape is the body those joints are in?**
//! Every reference figure in this crate — the landmark heights, the segment
//! lengths, the spans, the trunk silhouette, the limb-thickness ladder — is a
//! measurement of the two CC0 mannequins, and until [`Gltf::rest_meshes`]
//! existed each one was a number somebody wrote down once, with nothing able to
//! reproduce it and so nothing able to notice it was wrong. One was
//! ([#173](https://github.com/TheJanusStream/symbios-avatar/issues/173)). See
//! `examples/reference`, which prints the tables, and the mannequin test below,
//! which is the ratchet under them.
//!
//! Still nothing here reads a material, an image or a camera.
//!
//! # Why a reader rather than a crate
//!
//! `serde_json` is already a dependency — records are JSON on the wire — so the
//! document costs nothing new, and what is left is a container header, four
//! accessor shapes and two interpolation modes. The alternative pulls a general
//! glTF implementation, its image decoders and its extension registry into a
//! crate that ships to browsers, to read files that use none of it.
//!
//! **The surface below was measured off the files rather than taken from the
//! specification**, which is why it is this small. Both CC0 reference animation
//! GLBs report:
//!
//! ```text
//!   glTF version 2, one JSON chunk and one embedded BIN chunk, no external URI
//!   68 nodes, one skin of 66 joints, 87 and 75 animations
//!   samplers: STEP and LINEAR only — no CUBICSPLINE anywhere
//!   animation accessors: FLOAT only, as SCALAR, VEC3 and VEC4
//!   no sparse accessors, no normalized accessors, no bufferView byteStride
//!   nodes carry translation/rotation/scale — none carries a matrix
//! ```
//!
//! The two mannequin GLBs are a wider file than the animation library, and the
//! mesh side of this reader is measured off them the same way:
//!
//! ```text
//!   one mesh of one TRIANGLES primitive, skinned to the same 66-joint skin
//!   POSITION as FLOAT VEC3, indices as UNSIGNED_SHORT
//!   JOINTS_0 as UNSIGNED_BYTE, WEIGHTS_0 as normalized UNSIGNED_SHORT
//!   inverseBindMatrices present, as FLOAT MAT4
//!   7399 vertices on the male, 24037 on the female, both about 14k triangles
//! ```
//!
//! Which is why integers and the `normalized` flag are read now and were not
//! before: an animation accessor is FLOAT and a vertex attribute is very often
//! not.
//!
//! Everything on that list that is *absent* is refused loudly rather than
//! ignored: a file with a `matrix` node, a `CUBICSPLINE` sampler, an external
//! buffer or a sparse accessor gets an error naming what it is, because the one
//! failure mode a partial reader must not have is reading a file it does not
//! understand and returning a plausible answer. A silently wrong pose is
//! indistinguishable from a bad retarget, and this crate has lost days to
//! exactly that class of mistake.
//!
//! # Where it lives, and the number behind that
//!
//! In the library rather than behind a feature or in the bake tool, and the
//! number that decided it is **not** the number that was guessed. The bake is
//! offline, so nothing at runtime calls any of this, and a browser payload
//! should not carry it.
//!
//! Measured on the `wasm32-unknown-unknown` rlib: 4,134,098 bytes before this
//! module and 4,770,752 after, so **622 KiB**, against a guess of sixty. An
//! rlib is not a payload — it carries metadata and uninstantiated generic code,
//! and `serde`'s derive is most of what makes that figure large — so this is an
//! upper bound and a loose one. What settles it is a wasm consumer binary,
//! where nothing reachable from an entry point calls `Gltf::read` and the
//! link-time pass should strip the lot; the viewer (#141) is that binary's
//! nearest stand-in, and the figure has not been re-taken since.
//!
//! Being in the library is what lets `cargo test` reach it, which is the reason
//! it is not a feature today: a default-off feature is a module the shipped
//! test command silently stops testing, and this crate has been bitten enough
//! times by assertions that quietly select nothing. If the measurement at #141
//! disagrees, the remedy is that feature plus a test command that turns it on
//! — not a move into the bake tool, which no test runs at all.

use std::collections::HashMap;

use glam::{Mat4, Quat, Vec3};
use serde::Deserialize;
use thiserror::Error;

/// Magic at the head of every GLB.
const MAGIC: &[u8; 4] = b"glTF";

/// The only container version this reads.
const VERSION: u32 = 2;

/// `FLOAT`, the only component type any animation accessor uses.
const FLOAT: u32 = 5126;

/// `UNSIGNED_BYTE`. A small skin's `JOINTS_0`, and normalised `WEIGHTS_0`.
const UNSIGNED_BYTE: u32 = 5121;

/// `UNSIGNED_SHORT`. A large skin's `JOINTS_0`, and most index buffers.
const UNSIGNED_SHORT: u32 = 5123;

/// `UNSIGNED_INT`. An index buffer past 65535 vertices.
const UNSIGNED_INT: u32 = 5125;

/// How many bytes one component of each type this reads occupies.
fn width_of(component_type: u32) -> Result<usize, GltfError> {
    match component_type {
        UNSIGNED_BYTE => Ok(1),
        UNSIGNED_SHORT => Ok(2),
        UNSIGNED_INT | FLOAT => Ok(4),
        other => Err(GltfError::Unsupported(format!(
            "component type {other}; this reads unsigned byte, short, int and float"
        ))),
    }
}

/// Errors raised while reading a glTF binary.
///
/// Every variant is a refusal rather than a fallback. See the module docs for
/// why: a reader that guesses returns a pose that looks like a retargeting bug.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum GltfError {
    /// The bytes do not begin with the GLB magic.
    #[error("not a GLB: expected the magic 'glTF'")]
    NotGlb,
    /// The container version is not one this reads.
    #[error("GLB container version {found}, and this reads version {VERSION}")]
    Version {
        /// The version the file declares.
        found: u32,
    },
    /// A chunk header or its payload runs past the end of the file.
    #[error("the GLB is truncated: a chunk claims {claimed} bytes with {left} left")]
    Truncated {
        /// The length the chunk header declares.
        claimed: usize,
        /// How many bytes actually remain.
        left: usize,
    },
    /// There is no JSON chunk.
    #[error("the GLB has no JSON chunk")]
    NoJson,
    /// The JSON chunk did not parse, or did not have the shape glTF describes.
    #[error("the glTF document could not be read: {0}")]
    Json(String),
    /// The document points at a buffer this reader cannot reach.
    ///
    /// A GLB may hold one buffer with no URI — the embedded `BIN` chunk — and
    /// this reads exactly that. Anything else is a file on disk beside the GLB,
    /// which is a fetch this has no business performing.
    #[error("buffer {buffer} is external ({uri}); only the embedded BIN chunk is read")]
    ExternalBuffer {
        /// Which buffer.
        buffer: usize,
        /// Where it says its bytes are.
        uri: String,
    },
    /// An index in the document points at something that is not there.
    #[error("the document's {what} index {index} does not exist")]
    Missing {
        /// What was being looked up.
        what: &'static str,
        /// The index that missed.
        index: usize,
    },
    /// An accessor's bytes run past the end of the buffer they name.
    #[error("accessor {accessor} reads past the end of its buffer")]
    OutOfBounds {
        /// Which accessor.
        accessor: usize,
    },
    /// The file uses part of glTF this deliberately does not read.
    ///
    /// Carries what it was, so the message says whether the fix is to widen
    /// this reader or to re-export the file.
    #[error("unsupported: {0}")]
    Unsupported(String),
}

/// One node's local transform, as glTF stores it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trs {
    /// Where it sits, relative to its parent.
    pub translation: Vec3,
    /// How it is turned.
    pub rotation: Quat,
    /// How it is scaled. Present because these files animate it, and a scale
    /// track dropped on the floor is a limb that quietly does not shrink.
    pub scale: Vec3,
}

impl Default for Trs {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Trs {
    /// This transform as a matrix.
    #[must_use]
    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

/// A skin's joints, as a tree in the skin's own order.
///
/// **In the skin's order, not the document's**, because that is the order every
/// consumer of a skinned glTF indexes by and the order the joints' own
/// hierarchy has to be expressed in to be usable. `parents` indexes into this
/// list rather than into the document's nodes for the same reason.
#[derive(Clone, Debug, PartialEq)]
pub struct Skin {
    /// Which document node each joint is.
    pub nodes: Vec<usize>,
    /// What each joint is called.
    ///
    /// **For reading and for authoring a correspondence table, never for
    /// matching a rig to ours at run time.** This crate's standing finding
    /// about this reference is that rigs must not be matched by bone name — its
    /// bone called `head` is the jaw band, and slicing its T-pose at neck
    /// height cuts both arms.
    pub names: Vec<String>,
    /// Each joint's parent, as an index into this skin, or `None` for a root.
    ///
    /// A joint whose document parent is outside the skin reads as a root here,
    /// which is what it is as far as the skin is concerned.
    pub parents: Vec<Option<usize>>,
}

impl Skin {
    /// How many joints it has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether it has none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// One mesh of a file, in that file's own rest pose and in world space.
///
/// **Skinned into rest rather than handed over in mesh space.** For an
/// unskinned mesh the two differ by the node's transform; for a skinned one
/// they differ by whatever the bind pose was, and an exporter is not obliged to
/// make those agree. Measuring the mesh-space positions of a rig whose bind
/// pose is not its rest pose reports a body nobody has ever seen, and it does
/// it silently — so this applies `world · inverseBind` per joint and hands back
/// what the file actually draws.
#[derive(Clone, Debug, Default)]
pub struct RestMesh {
    /// What the mesh is called in the file, or empty.
    pub name: String,
    /// Every vertex, world space, at rest.
    pub positions: Vec<Vec3>,
    /// Every triangle, as indices into [`Self::positions`].
    pub triangles: Vec<[u32; 3]>,
    /// Which skin deforms it, as an index into the document's skins.
    pub skin: Option<usize>,
    /// What holds each vertex: `(joint, weight)` pairs indexed the way
    /// [`Skin::nodes`] is, with zero-weight influences dropped.
    ///
    /// Empty for an unskinned mesh, and empty for a vertex the file gives no
    /// weights — which is a vertex nailed to the model's origin, not one held
    /// by everything.
    pub influences: Vec<Vec<(usize, f32)>>,
}

impl RestMesh {
    /// The axis-aligned bounds of the whole mesh.
    ///
    /// Returns `(MAX, MIN)` for an empty mesh, so a caller that folds over
    /// several meshes gets the identity of the fold rather than a box at the
    /// origin.
    #[must_use]
    pub fn bounds(&self) -> (Vec3, Vec3) {
        self.positions.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(low, high), &at| (low.min(at), high.max(at)),
        )
    }

    /// How much of a vertex is held by joints `wanted` accepts.
    ///
    /// The measurement every reference column is taken with: it is what lets a
    /// T-posed arm be dropped out of a shoulder band without either skeleton
    /// being consulted by name.
    #[must_use]
    pub fn held_by(&self, vertex: usize, wanted: impl Fn(usize) -> bool) -> f32 {
        self.influences
            .get(vertex)
            .map(|held| {
                held.iter()
                    .filter(|(joint, _)| wanted(*joint))
                    .map(|(_, weight)| weight)
                    .sum()
            })
            .unwrap_or(0.0)
    }
}

/// A glTF binary, read far enough to sample its animations.
#[derive(Clone, Debug)]
pub struct Gltf {
    document: Document,
    binary: Vec<u8>,
}

impl Gltf {
    /// Reads a GLB.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if the bytes are not a version 2 GLB, if the
    /// document does not parse, or if it uses part of glTF this does not read.
    pub fn read(bytes: &[u8]) -> Result<Self, GltfError> {
        if bytes.len() < 12 || &bytes[0..4] != MAGIC {
            return Err(GltfError::NotGlb);
        }
        let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if version != VERSION {
            return Err(GltfError::Version { found: version });
        }

        // Chunks run to the end of the file rather than to the length in the
        // header: exporters have been known to disagree with themselves there,
        // and the header's total is the one figure nothing downstream needs.
        let (mut json, mut binary) = (None, Vec::new());
        let mut at = 12usize;
        while at + 8 <= bytes.len() {
            let length =
                u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
                    as usize;
            let kind = [bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]];
            let from = at + 8;
            let to = from.checked_add(length).ok_or(GltfError::Truncated {
                claimed: length,
                left: bytes.len() - from,
            })?;
            if to > bytes.len() {
                return Err(GltfError::Truncated {
                    claimed: length,
                    left: bytes.len() - from,
                });
            }
            match &kind {
                b"JSON" if json.is_none() => json = Some(&bytes[from..to]),
                b"BIN\0" if binary.is_empty() => binary = bytes[from..to].to_vec(),
                // An unknown chunk type is legal and is to be skipped, which is
                // the one place in this file where ignoring something is right.
                _ => {}
            }
            at = to;
        }

        let json = json.ok_or(GltfError::NoJson)?;
        let document: Document =
            serde_json::from_slice(json).map_err(|error| GltfError::Json(error.to_string()))?;
        for (index, buffer) in document.buffers.iter().enumerate() {
            if let Some(uri) = &buffer.uri {
                return Err(GltfError::ExternalBuffer {
                    buffer: index,
                    uri: uri.clone(),
                });
            }
        }
        Ok(Self { document, binary })
    }

    /// What each animation in the file is called, in the file's own order.
    #[must_use]
    pub fn clip_names(&self) -> Vec<&str> {
        self.document
            .animations
            .iter()
            .map(|animation| animation.name.as_deref().unwrap_or(""))
            .collect()
    }

    /// The index of the animation with this name, if there is one.
    #[must_use]
    pub fn clip(&self, name: &str) -> Option<usize> {
        self.document
            .animations
            .iter()
            .position(|animation| animation.name.as_deref() == Some(name))
    }

    /// How long an animation runs, in seconds.
    ///
    /// The last keyframe time over every sampler it drives. Zero for an
    /// animation with no keys at all, which is a pose rather than a motion.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if the animation does not exist or its samplers
    /// point at accessors that cannot be read.
    pub fn duration(&self, clip: usize) -> Result<f32, GltfError> {
        let animation = self.animation(clip)?;
        let mut last = 0.0f32;
        for sampler in &animation.samplers {
            let times = self.scalars(sampler.input)?;
            last = last.max(times.last().copied().unwrap_or(0.0));
        }
        Ok(last)
    }

    /// A skin's joints, as a tree.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if the skin or any joint it names does not exist.
    pub fn skin(&self, index: usize) -> Result<Skin, GltfError> {
        let skin = self.document.skins.get(index).ok_or(GltfError::Missing {
            what: "skin",
            index,
        })?;

        // Where each joint sits in the SKIN, so a document parent can be turned
        // into a skin-relative one — and so a joint parented outside the skin
        // comes out as a root, which is what it is here.
        let mut slot = HashMap::with_capacity(skin.joints.len());
        for (at, &node) in skin.joints.iter().enumerate() {
            slot.insert(node, at);
        }
        let parents = self.parents()?;

        let mut names = Vec::with_capacity(skin.joints.len());
        for &node in &skin.joints {
            let node = self.document.nodes.get(node).ok_or(GltfError::Missing {
                what: "node",
                index: node,
            })?;
            names.push(node.name.clone().unwrap_or_default());
        }
        Ok(Skin {
            names,
            parents: skin
                .joints
                .iter()
                .map(|node| parents[*node].and_then(|parent| slot.get(&parent).copied()))
                .collect(),
            nodes: skin.joints.clone(),
        })
    }

    /// Every mesh the file draws, in its own rest pose.
    ///
    /// One [`RestMesh`] per node that carries a mesh, with that mesh's
    /// primitives concatenated: a measurement wants the whole body, and which
    /// primitive a vertex came from is a fact about materials.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if a primitive is not a triangle list, if it has
    /// no `POSITION`, if it names a skin or accessor that is not there, or if
    /// an accessor is a shape this does not read.
    pub fn rest_meshes(&self) -> Result<Vec<RestMesh>, GltfError> {
        let world = self.rest()?;
        let mut out = Vec::new();
        for (index, node) in self.document.nodes.iter().enumerate() {
            let Some(mesh) = node.mesh else {
                continue;
            };
            let entry = self.document.meshes.get(mesh).ok_or(GltfError::Missing {
                what: "mesh",
                index: mesh,
            })?;

            // A skinned mesh ignores its own node transform by the
            // specification — the joints carry it — so the two branches here
            // are not a special case of one another.
            let joints = match node.skin {
                Some(skin) => Some(self.skin_matrices(skin, &world)?),
                None => None,
            };
            let place = world[index];

            let mut built = RestMesh {
                name: entry.name.clone().unwrap_or_default(),
                skin: node.skin,
                ..RestMesh::default()
            };
            for (at, primitive) in entry.primitives.iter().enumerate() {
                if primitive.mode != triangles() {
                    return Err(GltfError::Unsupported(format!(
                        "mesh {mesh} primitive {at} has mode {}; this reads triangle lists",
                        primitive.mode
                    )));
                }
                let base = built.positions.len() as u32;
                let local =
                    self.vec3s(*primitive.attributes.get("POSITION").ok_or_else(|| {
                        GltfError::Unsupported(format!(
                            "mesh {mesh} primitive {at} has no POSITION"
                        ))
                    })?)?;
                let held = self.primitive_influences(primitive, joints.as_ref().map(Vec::len))?;

                for (vertex, &at) in local.iter().enumerate() {
                    let placed = match (&joints, held.get(vertex)) {
                        (Some(joints), Some(held)) if !held.is_empty() => held
                            .iter()
                            .map(|&(joint, weight)| weight * joints[joint].transform_point3(at))
                            .fold(Vec3::ZERO, |sum, part| sum + part),
                        _ => place.transform_point3(at),
                    };
                    built.positions.push(placed);
                }
                built.influences.extend(held);

                match primitive.indices {
                    Some(indices) => {
                        let read = self.integers(indices, 1)?;
                        built.triangles.extend(
                            read.chunks_exact(3)
                                .map(|face| [base + face[0], base + face[1], base + face[2]]),
                        );
                    }
                    // No index buffer means the vertices are the triangles, in
                    // order, which is legal and which the reference files do
                    // not do.
                    None => {
                        built
                            .triangles
                            .extend((0..local.len() as u32 / 3).map(|face| {
                                [base + face * 3, base + face * 3 + 1, base + face * 3 + 2]
                            }))
                    }
                }
            }
            out.push(built);
        }
        Ok(out)
    }

    /// What each joint of a skin does to a vertex at rest: `world · inverseBind`.
    fn skin_matrices(&self, skin: usize, world: &[Mat4]) -> Result<Vec<Mat4>, GltfError> {
        let entry = self.document.skins.get(skin).ok_or(GltfError::Missing {
            what: "skin",
            index: skin,
        })?;
        let inverse = match entry.inverse_bind_matrices {
            Some(accessor) => self
                .floats(accessor, 16)?
                .chunks_exact(16)
                .map(Mat4::from_cols_slice)
                .collect(),
            None => vec![Mat4::IDENTITY; entry.joints.len()],
        };
        if inverse.len() != entry.joints.len() {
            return Err(GltfError::Unsupported(format!(
                "skin {skin} has {} joints and {} inverse bind matrices",
                entry.joints.len(),
                inverse.len()
            )));
        }
        entry
            .joints
            .iter()
            .zip(inverse)
            .map(|(&node, inverse)| {
                world
                    .get(node)
                    .map(|place| *place * inverse)
                    .ok_or(GltfError::Missing {
                        what: "node",
                        index: node,
                    })
            })
            .collect()
    }

    /// One primitive's per-vertex influences, or an empty list if it has none.
    ///
    /// glTF allows several `JOINTS_n`/`WEIGHTS_n` sets, four influences each.
    /// All of them are read: a body bound to five bones somewhere would
    /// otherwise come back holding four, with the missing weight silently
    /// dropped rather than named.
    fn primitive_influences(
        &self,
        primitive: &Primitive,
        joints: Option<usize>,
    ) -> Result<Vec<Vec<(usize, f32)>>, GltfError> {
        let Some(count) = joints else {
            return Ok(Vec::new());
        };
        let mut out: Vec<Vec<(usize, f32)>> = Vec::new();
        for set in 0.. {
            let (Some(&which), Some(&how_much)) = (
                primitive.attributes.get(&format!("JOINTS_{set}")),
                primitive.attributes.get(&format!("WEIGHTS_{set}")),
            ) else {
                break;
            };
            let which = self.integers(which, 4)?;
            let how_much = self.unit_floats(how_much, 4)?;
            if which.len() != how_much.len() {
                return Err(GltfError::Unsupported(format!(
                    "JOINTS_{set} has {} entries and WEIGHTS_{set} has {}",
                    which.len(),
                    how_much.len()
                )));
            }
            out.resize(out.len().max(which.len() / 4), Vec::new());
            for (vertex, (which, how_much)) in which
                .chunks_exact(4)
                .zip(how_much.chunks_exact(4))
                .enumerate()
            {
                for slot in 0..4 {
                    if how_much[slot] <= 0.0 {
                        continue;
                    }
                    let joint = which[slot] as usize;
                    if joint >= count {
                        return Err(GltfError::Missing {
                            what: "skin joint",
                            index: joint,
                        });
                    }
                    out[vertex].push((joint, how_much[slot]));
                }
            }
        }
        Ok(out)
    }

    /// Every node's world transform at `time` of an animation.
    ///
    /// Indexed by DOCUMENT node, not by skin joint: a joint's parent chain may
    /// run through nodes the skin does not list, and evaluating the whole tree
    /// is both simpler and the only way that stays correct when it does. Index
    /// it with [`Skin::nodes`] to get the joints.
    ///
    /// `time` is clamped to the animation's own range at each sampler, so
    /// asking before the first key or after the last holds the end pose rather
    /// than extrapolating.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if the animation does not exist, if it uses an
    /// interpolation mode this does not read, or if an accessor is malformed.
    pub fn sample(&self, clip: usize, time: f32) -> Result<Vec<Mat4>, GltfError> {
        let animation = self.animation(clip)?;

        // The rest pose first, then each channel overwrites the one component
        // it drives. A channel-less node keeps its rest transform, which is
        // what a glTF animation means by not mentioning it.
        let mut local: Vec<Trs> = self
            .document
            .nodes
            .iter()
            .map(Node::trs)
            .collect::<Result<_, _>>()?;

        for channel in &animation.channels {
            let Some(node) = channel.target.node else {
                continue;
            };
            if node >= local.len() {
                return Err(GltfError::Missing {
                    what: "node",
                    index: node,
                });
            }
            let sampler = animation
                .samplers
                .get(channel.sampler)
                .ok_or(GltfError::Missing {
                    what: "sampler",
                    index: channel.sampler,
                })?;
            let times = self.scalars(sampler.input)?;
            let (before, after, blend) = span(&times, time);

            match channel.target.path.as_str() {
                "translation" | "scale" => {
                    let values = self.vec3s(sampler.output)?;
                    let (a, b) = pair(&values, before, after, sampler.output)?;
                    let held = match sampler.interpolation.as_str() {
                        "STEP" => a,
                        "LINEAR" => a.lerp(b, blend),
                        other => return Err(unsupported_interpolation(other)),
                    };
                    if channel.target.path == "translation" {
                        local[node].translation = held;
                    } else {
                        local[node].scale = held;
                    }
                }
                "rotation" => {
                    let values = self.vec4s(sampler.output)?;
                    let (a, b) = pair(&values, before, after, sampler.output)?;
                    // Normalised on the way in: a quaternion read from a file
                    // is only as unit as its exporter left it, and slerp on a
                    // non-unit pair scales the mesh it drives.
                    let (a, b) = (quat(a), quat(b));
                    local[node].rotation = match sampler.interpolation.as_str() {
                        "STEP" => a,
                        "LINEAR" => a.slerp(b, blend),
                        other => return Err(unsupported_interpolation(other)),
                    };
                }
                // `weights` drives morph targets, which nothing here has.
                other => {
                    return Err(GltfError::Unsupported(format!(
                        "animation channel path '{other}'"
                    )));
                }
            }
        }

        self.compose(&local)
    }

    /// Every node's world transform in the file's rest pose.
    ///
    /// What a retargeter needs beside a sampled pose: the difference between
    /// them is the motion, and the rest pose alone is the skeleton's shape.
    ///
    /// # Errors
    ///
    /// Returns [`GltfError`] if a node carries a transform this does not read.
    pub fn rest(&self) -> Result<Vec<Mat4>, GltfError> {
        let local: Vec<Trs> = self
            .document
            .nodes
            .iter()
            .map(Node::trs)
            .collect::<Result<_, _>>()?;
        self.compose(&local)
    }

    /// Composes local transforms into world ones, parents before children.
    fn compose(&self, local: &[Trs]) -> Result<Vec<Mat4>, GltfError> {
        let parents = self.parents()?;
        let mut world = vec![None::<Mat4>; local.len()];
        // Iterative rather than recursive, and by chain rather than by sorted
        // order: glTF does not promise nodes come parent-before-child.
        for start in 0..local.len() {
            let mut chain = Vec::new();
            let mut at = Some(start);
            while let Some(node) = at {
                if world[node].is_some() {
                    break;
                }
                chain.push(node);
                if chain.len() > local.len() {
                    return Err(GltfError::Unsupported(
                        "the node hierarchy has a cycle".into(),
                    ));
                }
                at = parents[node];
            }
            for &node in chain.iter().rev() {
                let above = parents[node].and_then(|parent| world[parent]);
                world[node] = Some(above.unwrap_or(Mat4::IDENTITY) * local[node].matrix());
            }
        }
        Ok(world.into_iter().map(Option::unwrap_or_default).collect())
    }

    /// Each node's parent, by document index.
    fn parents(&self) -> Result<Vec<Option<usize>>, GltfError> {
        let mut parents = vec![None; self.document.nodes.len()];
        for (index, node) in self.document.nodes.iter().enumerate() {
            for &child in &node.children {
                if child >= parents.len() {
                    return Err(GltfError::Missing {
                        what: "node",
                        index: child,
                    });
                }
                parents[child] = Some(index);
            }
        }
        Ok(parents)
    }

    /// The animation at `clip`.
    fn animation(&self, clip: usize) -> Result<&Animation, GltfError> {
        self.document
            .animations
            .get(clip)
            .ok_or(GltfError::Missing {
                what: "animation",
                index: clip,
            })
    }

    /// An accessor's floats, as many per element as its type says.
    fn floats(&self, accessor: usize, components: usize) -> Result<Vec<f32>, GltfError> {
        let component_type = self.accessor(accessor)?.component_type;
        if component_type != FLOAT {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} has component type {component_type}, and this reads it as \
                 FLOAT only"
            )));
        }
        self.walk(accessor, components, |_, bytes| {
            f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        })
    }

    /// An accessor read as unsigned integers — joint indices and triangle
    /// indices.
    ///
    /// Widened to `u32` whatever the file stores, because the caller cares
    /// which vertex or which joint and not how many bytes it took to say so.
    fn integers(&self, accessor: usize, components: usize) -> Result<Vec<u32>, GltfError> {
        let component_type = self.accessor(accessor)?.component_type;
        if !matches!(
            component_type,
            UNSIGNED_BYTE | UNSIGNED_SHORT | UNSIGNED_INT
        ) {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} has component type {component_type} where an unsigned \
                 integer was wanted"
            )));
        }
        self.walk(
            accessor,
            components,
            |component_type, bytes| match component_type {
                UNSIGNED_BYTE => u32::from(bytes[0]),
                UNSIGNED_SHORT => u32::from(u16::from_le_bytes([bytes[0], bytes[1]])),
                _ => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            },
        )
    }

    /// An accessor read as a `0..=1` quantity — skin weights.
    ///
    /// **Normalisation is the accessor's `normalized` flag and not a guess from
    /// the component type**, because an unsigned short is a joint index in one
    /// attribute and 1/65535ths of a weight in the next. A file that stores
    /// weights as integers without saying they are normalised is refused rather
    /// than divided by a number nobody wrote down.
    fn unit_floats(&self, accessor: usize, components: usize) -> Result<Vec<f32>, GltfError> {
        let read = self.accessor(accessor)?;
        let (component_type, normalized) = (read.component_type, read.normalized);
        if component_type != FLOAT && !normalized {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} stores a weight as component type {component_type} without \
                 the normalized flag, so its scale is unknown"
            )));
        }
        self.walk(
            accessor,
            components,
            |component_type, bytes| match component_type {
                UNSIGNED_BYTE => f32::from(bytes[0]) / 255.0,
                UNSIGNED_SHORT => f32::from(u16::from_le_bytes([bytes[0], bytes[1]])) / 65535.0,
                UNSIGNED_INT => {
                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32
                        / 4_294_967_295.0
                }
                _ => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            },
        )
    }

    /// One accessor, looked up and refused if it is a shape this cannot read.
    fn accessor(&self, accessor: usize) -> Result<&Accessor, GltfError> {
        let read = self
            .document
            .accessors
            .get(accessor)
            .ok_or(GltfError::Missing {
                what: "accessor",
                index: accessor,
            })?;
        if read.sparse.is_some() {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} is sparse"
            )));
        }
        Ok(read)
    }

    /// Walks an accessor's elements, handing each component's bytes to `parse`.
    ///
    /// The one place in this file that knows about buffer views, strides and
    /// bounds, so every attribute reader above gets the same refusals rather
    /// than three copies of them that drift.
    fn walk<T>(
        &self,
        accessor: usize,
        components: usize,
        parse: impl Fn(u32, &[u8]) -> T,
    ) -> Result<Vec<T>, GltfError> {
        let read = self.accessor(accessor)?;
        let wanted = components_of(&read.kind)?;
        if wanted != components {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} is {} where {components} components were wanted",
                read.kind
            )));
        }
        let Some(view) = read.buffer_view else {
            // A view-less accessor is all zeroes by the specification. Nothing
            // this reads has any business being one.
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} has no bufferView"
            )));
        };
        let view = self
            .document
            .buffer_views
            .get(view)
            .ok_or(GltfError::Missing {
                what: "bufferView",
                index: view,
            })?;

        let width = width_of(read.component_type)?;
        let element = components * width;
        let stride = view.byte_stride.unwrap_or(element);
        if stride < element {
            return Err(GltfError::Unsupported(format!(
                "bufferView stride {stride} is shorter than its own {element}-byte element"
            )));
        }
        let start = view.byte_offset + read.byte_offset;
        let mut out = Vec::with_capacity(read.count * components);
        for index in 0..read.count {
            let at = start + index * stride;
            if at + element > self.binary.len()
                || at + element > view.byte_offset + view.byte_length
            {
                return Err(GltfError::OutOfBounds { accessor });
            }
            for component in 0..components {
                let from = at + component * width;
                out.push(parse(read.component_type, &self.binary[from..from + width]));
            }
        }
        Ok(out)
    }

    /// An accessor read as scalars — keyframe times.
    fn scalars(&self, accessor: usize) -> Result<Vec<f32>, GltfError> {
        self.floats(accessor, 1)
    }

    /// An accessor read as three-vectors — translations and scales.
    fn vec3s(&self, accessor: usize) -> Result<Vec<Vec3>, GltfError> {
        Ok(self
            .floats(accessor, 3)?
            .chunks_exact(3)
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect())
    }

    /// An accessor read as four-vectors — rotations, `xyzw` as glTF stores them.
    fn vec4s(&self, accessor: usize) -> Result<Vec<[f32; 4]>, GltfError> {
        Ok(self
            .floats(accessor, 4)?
            .chunks_exact(4)
            .map(|v| [v[0], v[1], v[2], v[3]])
            .collect())
    }
}

/// Which two keys `time` falls between, and how far between them it is.
///
/// Clamped at both ends: before the first key and after the last, the nearest
/// key is held. Returns `(0, 0, 0.0)` for an empty track, which the callers
/// turn into a missing-value error rather than a silent identity.
fn span(times: &[f32], time: f32) -> (usize, usize, f32) {
    if times.len() < 2 {
        return (0, 0, 0.0);
    }
    if time <= times[0] {
        return (0, 0, 0.0);
    }
    if time >= times[times.len() - 1] {
        let last = times.len() - 1;
        return (last, last, 0.0);
    }
    // Linear rather than binary: these tracks are tens of keys long, and a
    // search that is correct at a glance is worth more here than one that is
    // fast in a loop nothing runs per frame.
    let after = times.iter().position(|&at| at > time).unwrap_or(1);
    let before = after - 1;
    let range = times[after] - times[before];
    let blend = if range > f32::EPSILON {
        (time - times[before]) / range
    } else {
        0.0
    };
    (before, after, blend)
}

/// The two values a span names, or an error saying which accessor was short.
fn pair<T: Copy>(
    values: &[T],
    before: usize,
    after: usize,
    accessor: usize,
) -> Result<(T, T), GltfError> {
    match (values.get(before), values.get(after)) {
        (Some(&a), Some(&b)) => Ok((a, b)),
        _ => Err(GltfError::OutOfBounds { accessor }),
    }
}

/// A glTF `xyzw` quaternion as a unit [`Quat`].
fn quat(value: [f32; 4]) -> Quat {
    Quat::from_xyzw(value[0], value[1], value[2], value[3]).normalize()
}

/// How many components one element of an accessor type has.
fn components_of(kind: &str) -> Result<usize, GltfError> {
    match kind {
        "SCALAR" => Ok(1),
        "VEC2" => Ok(2),
        "VEC3" => Ok(3),
        "VEC4" => Ok(4),
        "MAT4" => Ok(16),
        other => Err(GltfError::Unsupported(format!("accessor type {other}"))),
    }
}

/// The error for a sampler this does not read.
fn unsupported_interpolation(mode: &str) -> GltfError {
    GltfError::Unsupported(format!(
        "sampler interpolation '{mode}'; this reads STEP and LINEAR, which is what the \
         reference library uses"
    ))
}

/// The parts of a glTF document this reads.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    #[serde(default)]
    nodes: Vec<Node>,
    #[serde(default)]
    meshes: Vec<MeshEntry>,
    #[serde(default)]
    skins: Vec<SkinEntry>,
    #[serde(default)]
    animations: Vec<Animation>,
    #[serde(default)]
    accessors: Vec<Accessor>,
    #[serde(default)]
    buffer_views: Vec<BufferView>,
    #[serde(default)]
    buffers: Vec<Buffer>,
}

/// One node of the hierarchy.
#[derive(Clone, Debug, Default, Deserialize)]
struct Node {
    name: Option<String>,
    #[serde(default)]
    children: Vec<usize>,
    /// Which mesh this node draws, if any.
    mesh: Option<usize>,
    /// Which skin deforms that mesh, if it is skinned.
    skin: Option<usize>,
    translation: Option<[f32; 3]>,
    rotation: Option<[f32; 4]>,
    scale: Option<[f32; 3]>,
    /// Refused rather than read. A node may carry either a matrix or a TRS
    /// triple, never both, and decomposing one into the other loses shear and
    /// makes the rest pose subtly wrong — which is the failure this reader is
    /// most concerned with. Neither reference file uses it.
    matrix: Option<[f32; 16]>,
}

impl Node {
    /// This node's local transform.
    fn trs(&self) -> Result<Trs, GltfError> {
        if self.matrix.is_some() {
            return Err(GltfError::Unsupported(
                "a node with a matrix transform; this reads translation, rotation and scale, \
                 which is what the reference library exports"
                    .into(),
            ));
        }
        Ok(Trs {
            translation: self.translation.map_or(Vec3::ZERO, Vec3::from_array),
            rotation: self.rotation.map_or(Quat::IDENTITY, quat),
            scale: self.scale.map_or(Vec3::ONE, Vec3::from_array),
        })
    }
}

/// A skin, as the document lists it.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkinEntry {
    #[serde(default)]
    joints: Vec<usize>,
    /// One matrix per joint, taking a vertex from mesh space into that joint's
    /// space. Absent means every joint's is the identity, which the
    /// specification allows and which no rigged export actually does.
    inverse_bind_matrices: Option<usize>,
}

/// One mesh, as the document lists it.
#[derive(Clone, Debug, Default, Deserialize)]
struct MeshEntry {
    name: Option<String>,
    #[serde(default)]
    primitives: Vec<Primitive>,
}

/// One drawable piece of a mesh.
#[derive(Clone, Debug, Default, Deserialize)]
struct Primitive {
    #[serde(default)]
    attributes: HashMap<String, usize>,
    indices: Option<usize>,
    /// glTF's default is 4, `TRIANGLES`. Anything else is refused: a strip or
    /// a fan measured as if it were a triangle list reports a shape that is not
    /// in the file.
    #[serde(default = "triangles")]
    mode: u32,
}

/// glTF's default primitive mode, `TRIANGLES`.
fn triangles() -> u32 {
    4
}

/// One animation.
#[derive(Clone, Debug, Default, Deserialize)]
struct Animation {
    name: Option<String>,
    #[serde(default)]
    channels: Vec<Channel>,
    #[serde(default)]
    samplers: Vec<Sampler>,
}

/// One animation channel: a sampler pointed at one component of one node.
#[derive(Clone, Debug, Default, Deserialize)]
struct Channel {
    sampler: usize,
    target: ChannelTarget,
}

/// What a channel drives.
#[derive(Clone, Debug, Default, Deserialize)]
struct ChannelTarget {
    /// Absent means the channel drives nothing, which is legal and skipped.
    node: Option<usize>,
    path: String,
}

/// One animation sampler: times in, values out.
#[derive(Clone, Debug, Deserialize)]
struct Sampler {
    input: usize,
    output: usize,
    #[serde(default = "linear")]
    interpolation: String,
}

/// glTF's default when a sampler does not say.
fn linear() -> String {
    "LINEAR".into()
}

/// One accessor.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Accessor {
    buffer_view: Option<usize>,
    #[serde(default)]
    byte_offset: usize,
    component_type: u32,
    count: usize,
    #[serde(rename = "type")]
    kind: String,
    /// Whether an integer component is a fraction of its own maximum.
    ///
    /// glTF's own default is false, which is what an index buffer wants.
    #[serde(default)]
    normalized: bool,
    /// Refused rather than read, and named in the error.
    sparse: Option<serde_json::Value>,
}

/// One view into a buffer.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BufferView {
    #[serde(default)]
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

/// One buffer.
#[derive(Clone, Debug, Default, Deserialize)]
struct Buffer {
    /// Present only for a buffer that lives outside the GLB, which is refused.
    uri: Option<String>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A two-joint GLB, for tests outside this module that need a readable file
    /// which is plainly NOT the CC0 reference rig.
    ///
    /// Shared rather than rebuilt next door: a second hand-rolled GLB writer is
    /// a second thing that can be wrong about the container.
    pub(crate) fn a_two_joint_glb() -> Vec<u8> {
        arm("LINEAR")
    }

    /// A skinned quad on the two-joint arm, in the shapes a real export uses.
    ///
    /// Deliberately awkward where the reference files are awkward: `JOINTS_0`
    /// is unsigned bytes, `WEIGHTS_0` is normalised unsigned shorts, the
    /// indices are unsigned shorts, and the skin carries real inverse bind
    /// matrices — so the test exercises every path `floats()` used to refuse.
    /// The mesh node also carries a translation that a skinned mesh must
    /// IGNORE, which is the one part of this nobody would notice being wrong.
    fn a_skinned_quad() -> Vec<u8> {
        let mut binary = Vec::new();
        // Four vertices, a metre apart up Y, in mesh space.
        for at in [
            [0.0f32, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
        ] {
            for component in at {
                binary.extend_from_slice(&component.to_le_bytes());
            }
        }
        let positions = 0..binary.len();
        // Two triangles.
        for index in [0u16, 1, 2, 2, 1, 3] {
            binary.extend_from_slice(&index.to_le_bytes());
        }
        let indices = positions.end..binary.len();
        // The lower pair on the root, the upper pair on the tip.
        for joints in [[0u8, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]] {
            binary.extend_from_slice(&joints);
        }
        let joints = indices.end..binary.len();
        for _ in 0..4 {
            for weight in [u16::MAX, 0, 0, 0] {
                binary.extend_from_slice(&weight.to_le_bytes());
            }
        }
        let weights = joints.end..binary.len();
        // Inverse binds: the root at the origin, the tip a metre up.
        for inverse in [
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0)),
        ] {
            for component in inverse.to_cols_array() {
                binary.extend_from_slice(&component.to_le_bytes());
            }
        }
        let inverses = weights.end..binary.len();

        let view = |range: &std::ops::Range<usize>| {
            serde_json::json!({
                "buffer": 0,
                "byteOffset": range.start,
                "byteLength": range.len(),
            })
        };
        let document = serde_json::json!({
            "asset": { "version": "2.0" },
            "nodes": [
                { "name": "root", "children": [1] },
                { "name": "tip", "translation": [0.0, 1.0, 0.0] },
                { "name": "body", "mesh": 0, "skin": 0, "translation": [9.0, 9.0, 9.0] },
            ],
            "skins": [{ "joints": [0, 1], "inverseBindMatrices": 4 }],
            "meshes": [{
                "name": "Quad",
                "primitives": [{
                    "attributes": { "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 },
                    "indices": 1,
                }],
            }],
            "buffers": [{ "byteLength": binary.len() }],
            "bufferViews": [
                view(&positions),
                view(&indices),
                view(&joints),
                view(&weights),
                view(&inverses),
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3" },
                { "bufferView": 1, "componentType": 5123, "count": 6, "type": "SCALAR" },
                { "bufferView": 2, "componentType": 5121, "count": 4, "type": "VEC4" },
                {
                    "bufferView": 3, "componentType": 5123, "count": 4,
                    "type": "VEC4", "normalized": true,
                },
                { "bufferView": 4, "componentType": 5126, "count": 2, "type": "MAT4" },
            ],
        });
        glb(&document, &binary)
    }

    #[test]
    fn a_skinned_mesh_reads_into_its_own_rest_pose() {
        let file = Gltf::read(&a_skinned_quad()).expect("a GLB");
        let meshes = file.rest_meshes().expect("meshes");
        assert_eq!(meshes.len(), 1, "one node carries a mesh");
        let mesh = &meshes[0];
        assert_eq!(mesh.name, "Quad");
        assert_eq!(mesh.skin, Some(0));
        assert_eq!(mesh.triangles, vec![[0, 1, 2], [2, 1, 3]]);

        // The rest pose puts every vertex back where mesh space had it: the
        // tip's world transform and its inverse bind cancel. The node's own
        // (9, 9, 9) is ignored, which is what a skinned mesh means.
        for (vertex, want) in [
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            assert!(
                mesh.positions[vertex].distance(want) < 1e-5,
                "vertex {vertex} rests at {:?}, wanted {want:?}",
                mesh.positions[vertex]
            );
        }

        // Normalised unsigned shorts come back as a full weight, not as 65535.
        assert_eq!(mesh.influences[0], vec![(0, 1.0)]);
        assert_eq!(mesh.influences[3], vec![(1, 1.0)]);
        assert!((mesh.held_by(3, |joint| joint == 1) - 1.0).abs() < 1e-6);
        assert_eq!(mesh.held_by(3, |joint| joint == 0), 0.0);
    }

    #[test]
    fn a_weight_stored_as_integers_without_the_flag_is_refused() {
        // The trap this exists for: an unsigned short is a joint index in one
        // attribute and 1/65535th of a weight in the next, so reading the
        // component type alone would divide an index by 65535 and hand back a
        // body held together by nothing.
        let mut document: serde_json::Value =
            serde_json::from_slice(&json_of(&a_skinned_quad())).expect("the document parses");
        document["accessors"][3]
            .as_object_mut()
            .expect("an accessor")
            .remove("normalized");
        let rebuilt = glb(&document, &binary_of(&a_skinned_quad()));
        let error = Gltf::read(&rebuilt)
            .expect("a GLB")
            .rest_meshes()
            .expect_err("an unmarked integer weight is refused");
        assert!(
            matches!(&error, GltfError::Unsupported(what) if what.contains("normalized")),
            "{error}"
        );
    }

    #[test]
    fn a_primitive_that_is_not_a_triangle_list_is_refused() {
        let mut document: serde_json::Value =
            serde_json::from_slice(&json_of(&a_skinned_quad())).expect("the document parses");
        document["meshes"][0]["primitives"][0]["mode"] = serde_json::json!(5);
        let rebuilt = glb(&document, &binary_of(&a_skinned_quad()));
        let error = Gltf::read(&rebuilt)
            .expect("a GLB")
            .rest_meshes()
            .expect_err("a triangle strip is refused rather than measured");
        assert!(
            matches!(&error, GltfError::Unsupported(what) if what.contains("mode 5")),
            "{error}"
        );
    }

    /// The two mannequins every reference column in this crate is taken off.
    const MANNEQUINS: [(&str, &str); 2] = [
        (
            "male",
            "../mesh2motion-app/static/models-variation/human-male.glb",
        ),
        (
            "female",
            "../mesh2motion-app/static/models-variation/human-female.glb",
        ),
    ];

    #[test]
    fn the_reference_mannequins_measure_as_the_published_tables_say() {
        // **The ratchet under every reference figure in the crate** (#173).
        // Those figures were measured by hand once and written into
        // `examples/bodyaudit` as constants, where nothing could reproduce them
        // and so nothing could notice one being wrong — and re-deriving them
        // found one that was. This asserts a handful of the load-bearing ones
        // against the files themselves, so the next time a mannequin is
        // re-exported, or this reader's rest-pose skinning drifts, the tables
        // stop being quietly wrong and start being loudly wrong.
        //
        // Skips when the sibling checkout is absent, like the library test
        // above and for the same reason.
        for (who, path) in MANNEQUINS {
            let Ok(bytes) = std::fs::read(path) else {
                eprintln!("skipping: {path} is not checked out beside this repository");
                return;
            };
            let file = Gltf::read(&bytes).expect("a mannequin reads");
            let mesh = file
                .rest_meshes()
                .expect("its mesh reads")
                .into_iter()
                .next()
                .expect("it has one");
            let skin = file
                .skin(mesh.skin.expect("it is skinned"))
                .expect("a skin");
            let world = file.rest().expect("a rest pose");
            let at = |bone: &str| {
                let joint = skin
                    .names
                    .iter()
                    .position(|name| name == bone)
                    .unwrap_or_else(|| panic!("{who} has a bone called {bone}"));
                world[skin.nodes[joint]].w_axis.truncate()
            };

            let (low, high) = mesh.bounds();
            let height = high.y - low.y;
            let (stature, shoulder, hip, pelvis) = match who {
                "male" => (1.830, 0.1899, 0.0973, 0.5013),
                _ => (1.806, 0.1560, 0.0986, 0.5179),
            };
            assert!(
                (height - stature).abs() < 0.002,
                "{who} renders {height:.3} m against a published {stature:.3}"
            );

            let span = |left: &str, right: &str| (at(left).x - at(right).x).abs() / height;
            assert!(
                (span("upperarm_l", "upperarm_r") - shoulder).abs() < 0.0005,
                "{who} shoulder span {:.4} against a published {shoulder:.4}",
                span("upperarm_l", "upperarm_r")
            );
            assert!(
                (span("thigh_l", "thigh_r") - hip).abs() < 0.0005,
                "{who} hip span {:.4} against a published {hip:.4}",
                span("thigh_l", "thigh_r")
            );
            let up = (at("pelvis").y - low.y) / height;
            assert!(
                (up - pelvis).abs() < 0.0005,
                "{who} pelvis at {up:.4} against a published {pelvis:.4}"
            );

            // Every vertex weighted, and every weight summing to one: a mesh
            // read with the wrong accessor width comes back holding nothing,
            // and would still measure a plausible height.
            let loose = (0..mesh.positions.len())
                .filter(|&vertex| (mesh.held_by(vertex, |_| true) - 1.0).abs() > 0.01)
                .count();
            assert_eq!(
                loose, 0,
                "{who} has {loose} vertices whose weights do not sum to one"
            );
        }
    }

    /// The JSON chunk of a GLB this module built.
    fn json_of(glb: &[u8]) -> Vec<u8> {
        chunk(glb, b"JSON")
    }

    /// The BIN chunk of a GLB this module built.
    fn binary_of(glb: &[u8]) -> Vec<u8> {
        chunk(glb, b"BIN\0")
    }

    /// One named chunk of a GLB.
    fn chunk(glb: &[u8], want: &[u8; 4]) -> Vec<u8> {
        let mut at = 12usize;
        while at + 8 <= glb.len() {
            let length =
                u32::from_le_bytes([glb[at], glb[at + 1], glb[at + 2], glb[at + 3]]) as usize;
            let kind = [glb[at + 4], glb[at + 5], glb[at + 6], glb[at + 7]];
            if &kind == want {
                return glb[at + 8..at + 8 + length].to_vec();
            }
            at = at + 8 + length;
        }
        Vec::new()
    }

    /// Where the CC0 reference animations sit, relative to this checkout.
    ///
    /// **A sibling checkout, deliberately not vendored.** The files are 11 MB
    /// of CC0 GLB belonging to mesh2motion; this repository records where they
    /// are rather than carrying a copy, so every test that reads them skips
    /// cleanly when they are absent. The assertions that must hold everywhere
    /// are the ones below that build their own GLB.
    const REFERENCE: &str = "../mesh2motion-app/static/animations/human-base-animations.glb";

    /// Builds a GLB from a document and a binary chunk, the way an exporter
    /// would.
    ///
    /// Test-only, and the reason the parser can be asserted on at all without
    /// the reference checkout: a fixture this file writes itself is a fixture
    /// whose right answer is known by construction.
    fn glb(document: &serde_json::Value, binary: &[u8]) -> Vec<u8> {
        let json = serde_json::to_vec(document).expect("a document serialises");
        let pad = |bytes: &mut Vec<u8>, filler: u8| {
            while !bytes.len().is_multiple_of(4) {
                bytes.push(filler);
            }
        };
        let mut json = json;
        pad(&mut json, b' ');
        let mut binary = binary.to_vec();
        pad(&mut binary, 0);

        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        let total = 12 + 8 + json.len() + 8 + binary.len();
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json);
        out.extend_from_slice(&(binary.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&binary);
        out
    }

    /// A two-joint arm: a root at the origin and a child one metre along `Y`,
    /// with one rotation track turning the root a quarter turn about `Z` over
    /// one second.
    fn arm(interpolation: &str) -> Vec<u8> {
        let mut binary = Vec::new();
        // Keyframe times, then two quaternions: identity, then 90° about Z.
        for time in [0.0f32, 1.0] {
            binary.extend_from_slice(&time.to_le_bytes());
        }
        let turn = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        for value in [Quat::IDENTITY, turn] {
            for component in [value.x, value.y, value.z, value.w] {
                binary.extend_from_slice(&component.to_le_bytes());
            }
        }
        let document = serde_json::json!({
            "asset": { "version": "2.0" },
            "nodes": [
                { "name": "root", "children": [1] },
                { "name": "tip", "translation": [0.0, 1.0, 0.0] },
            ],
            "skins": [{ "joints": [0, 1] }],
            "buffers": [{ "byteLength": binary.len() }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": 8 },
                { "buffer": 0, "byteOffset": 8, "byteLength": 32 },
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 2, "type": "SCALAR" },
                { "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC4" },
            ],
            "animations": [{
                "name": "Turn",
                "samplers": [{ "input": 0, "output": 1, "interpolation": interpolation }],
                "channels": [{ "sampler": 0, "target": { "node": 0, "path": "rotation" } }],
            }],
        });
        glb(&document, &binary)
    }

    #[test]
    fn a_glb_reads_its_animations_and_its_skin() {
        let read = Gltf::read(&arm("LINEAR")).expect("the fixture reads");
        assert_eq!(read.clip_names(), vec!["Turn"]);
        assert_eq!(read.clip("Turn"), Some(0));
        assert_eq!(read.clip("Nothing"), None);
        assert!((read.duration(0).expect("a duration") - 1.0).abs() < 1e-6);

        let skin = read.skin(0).expect("a skin");
        assert_eq!(skin.len(), 2);
        assert_eq!(skin.names, vec!["root".to_string(), "tip".to_string()]);
        assert_eq!(skin.parents, vec![None, Some(0)]);
        assert_eq!(skin.nodes, vec![0, 1]);
    }

    #[test]
    fn a_sampled_pose_swings_the_child_through_the_arc_its_parent_turns() {
        // The whole point of the reader, on a case whose answer is known by
        // construction: a tip one metre up, swung a quarter turn about Z, ends
        // one metre out along −X. Half way through a LINEAR track it is at 45°,
        // which is the assertion that catches a reader that reads keys but not
        // interpolation — and one that slerps a non-unit pair, because that
        // lands the tip off the unit circle rather than off the angle.
        let read = Gltf::read(&arm("LINEAR")).expect("the fixture reads");
        let skin = read.skin(0).expect("a skin");
        let tip = |time: f32| {
            let world = read.sample(0, time).expect("a pose");
            world[skin.nodes[1]].transform_point3(Vec3::ZERO)
        };

        assert!(
            tip(0.0).abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-5),
            "{:?}",
            tip(0.0)
        );
        assert!(
            tip(1.0).abs_diff_eq(Vec3::new(-1.0, 0.0, 0.0), 1e-5),
            "{:?}",
            tip(1.0)
        );

        let half = tip(0.5);
        let root = std::f32::consts::FRAC_1_SQRT_2;
        assert!(
            half.abs_diff_eq(Vec3::new(-root, root, 0.0), 1e-5),
            "{half:?}"
        );
        assert!(
            (half.length() - 1.0).abs() < 1e-5,
            "the arm changed length: {half:?}"
        );

        // Clamped at both ends rather than extrapolated.
        assert!(tip(-5.0).abs_diff_eq(tip(0.0), 1e-6));
        assert!(tip(9.0).abs_diff_eq(tip(1.0), 1e-6));
    }

    #[test]
    fn a_step_track_holds_its_key_until_the_next_one() {
        // STEP is two thirds of the samplers in the reference library, so a
        // reader that quietly treated it as LINEAR would be wrong about most of
        // what it reads while looking almost right.
        let read = Gltf::read(&arm("STEP")).expect("the fixture reads");
        let skin = read.skin(0).expect("a skin");
        let tip = |time: f32| {
            read.sample(0, time).expect("a pose")[skin.nodes[1]].transform_point3(Vec3::ZERO)
        };

        assert!(
            tip(0.5).abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-5),
            "{:?}",
            tip(0.5)
        );
        assert!(
            tip(1.0).abs_diff_eq(Vec3::new(-1.0, 0.0, 0.0), 1e-5),
            "{:?}",
            tip(1.0)
        );
    }

    #[test]
    fn the_rest_pose_is_the_document_with_no_animation_applied() {
        let read = Gltf::read(&arm("LINEAR")).expect("the fixture reads");
        let skin = read.skin(0).expect("a skin");
        let rest = read.rest().expect("a rest pose");
        assert!(
            rest[skin.nodes[1]]
                .transform_point3(Vec3::ZERO)
                .abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-6)
        );
    }

    #[test]
    fn what_it_does_not_read_is_refused_by_name() {
        // Every one of these is a file this reader would otherwise answer
        // plausibly and wrongly about, which is the failure mode a partial
        // reader must not have.
        assert_eq!(
            Gltf::read(b"not a glb at all").unwrap_err(),
            GltfError::NotGlb
        );

        let mut wrong = arm("LINEAR");
        wrong[4..8].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            Gltf::read(&wrong).unwrap_err(),
            GltfError::Version { found: 1 }
        );

        let cubic = Gltf::read(&arm("CUBICSPLINE")).expect("it reads; the sampler is the problem");
        assert!(matches!(
            cubic.sample(0, 0.5).unwrap_err(),
            GltfError::Unsupported(ref what) if what.contains("CUBICSPLINE")
        ));

        let external = glb(
            &serde_json::json!({
                "asset": { "version": "2.0" },
                "buffers": [{ "byteLength": 4, "uri": "elsewhere.bin" }],
            }),
            &[],
        );
        assert!(matches!(
            Gltf::read(&external).unwrap_err(),
            GltfError::ExternalBuffer { buffer: 0, .. }
        ));

        let matrixed = glb(
            &serde_json::json!({
                "asset": { "version": "2.0" },
                "nodes": [{ "name": "m", "matrix": vec![1.0f32; 16] }],
                "animations": [],
            }),
            &[],
        );
        let matrixed = Gltf::read(&matrixed).expect("it reads; the node is the problem");
        assert!(matches!(
            matrixed.rest().unwrap_err(),
            GltfError::Unsupported(ref what) if what.contains("matrix")
        ));
    }

    #[test]
    fn the_reference_library_reads() {
        // **Skips rather than fails when the sibling checkout is absent**, so
        // this repository stays self-contained while the one assertion that
        // matters most — that the reader handles the actual files it was
        // written for — is available to anyone who has them.
        let Ok(bytes) = std::fs::read(REFERENCE) else {
            eprintln!("skipping: {REFERENCE} is not checked out beside this repository");
            return;
        };
        let read = Gltf::read(&bytes).expect("the reference library reads");

        // Measured off the file 2026-08-07 (#137). If these move, the library
        // was re-exported and every baked clip wants re-baking.
        assert_eq!(read.clip_names().len(), 87);
        let skin = read.skin(0).expect("the reference has a skin");
        assert_eq!(skin.len(), 66);
        assert_eq!(skin.parents[0], None, "the reference's root has no parent");
        assert!(skin.names.iter().any(|name| name == "pelvis"));

        let walk = read.clip("Walk").expect("the reference has a Walk");
        let duration = read.duration(walk).expect("a duration");
        assert!(
            duration > 0.1 && duration < 10.0,
            "a walk cycle of {duration} s is not a walk cycle"
        );

        // A pose at every joint, and a body that has not exploded: the whole
        // skeleton must sit inside a couple of metres of its own root.
        let world = read.sample(walk, duration * 0.5).expect("a pose");
        let root = world[skin.nodes[0]].transform_point3(Vec3::ZERO);
        let mut furthest = 0.0f32;
        for &node in &skin.nodes {
            let at = world[node].transform_point3(Vec3::ZERO);
            assert!(at.is_finite(), "joint at {at:?} is not finite");
            furthest = furthest.max(at.distance(root));
        }
        assert!(
            furthest > 0.5 && furthest < 3.0,
            "the furthest joint is {furthest} m from the root, which is not a human being"
        );
    }
}
