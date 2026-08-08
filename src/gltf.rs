//! Reading a glTF binary far enough to sample a skinned animation, and no
//! further.
//!
//! This exists for one question, and the question is the whole specification:
//! **where is each of a skin's joints, at time `t` of animation `n`?** Nothing
//! here reads a material, a mesh, an image or a camera, because nothing that
//! consumes this wants one — the motion library it was written for
//! ([#102](https://github.com/TheJanusStream/symbios-avatar/issues/102)) is
//! 162 clips of skeletal animation and a skeleton to hang them on.
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
//! link-time pass should strip the lot; the viewer becomes that binary at #141,
//! and the figure gets re-taken there.
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

/// `FLOAT`, the only component type any animation accessor here uses.
const FLOAT: u32 = 5126;

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
        if read.component_type != FLOAT {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} has component type {}, and animation is read as FLOAT only",
                read.component_type
            )));
        }
        let wanted = components_of(&read.kind)?;
        if wanted != components {
            return Err(GltfError::Unsupported(format!(
                "accessor {accessor} is {} where {components} components were wanted",
                read.kind
            )));
        }
        let Some(view) = read.buffer_view else {
            // A view-less accessor is all zeroes by the specification. Nothing
            // in an animation has any business being one.
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

        let element = components * 4;
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
                let from = at + component * 4;
                out.push(f32::from_le_bytes([
                    self.binary[from],
                    self.binary[from + 1],
                    self.binary[from + 2],
                    self.binary[from + 3],
                ]));
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
struct SkinEntry {
    #[serde(default)]
    joints: Vec<usize>,
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
