//! Sending geometry across a worker boundary.
//!
//! A derived `Serialize` hands the codec every scalar in a [`PolyMesh`] one at
//! a time, and across the msgpack boundary the `serde-avatar` feature exists
//! for, that is not a small overhead. The consuming worker encodes with
//! `rmp_serde::to_vec_named`, so a derived struct becomes a **named map**:
//! each of the four [`Influence`](crate::rig::skin::Influence)s a rigged
//! vertex carries spends its field names and type tags to say twelve bytes of
//! data, and positions, normals and colours pay a tag per component. Faces pay
//! one per index.
//!
//! So each channel rides as an **opaque byte blob** (`serde_bytes`) — the same
//! choice, for the same reason, as [`crate::texture::atlas_serde`] made for the
//! atlas: one header and a memcpy, decoded without a visitor call per scalar.
//! That adapter fixed the texture half of the payload and left the geometry
//! half element-wise; this is the other half.
//!
//! ## The format
//!
//! One blob per channel, plus two for the face list. Every scalar is
//! **little-endian**, written and read a component at a time rather than by
//! reinterpreting host memory: a worker boundary need not join two machines of
//! the same architecture, and `Vec3` makes no layout promise worth betting a
//! silently wrong mesh on.
//!
//! | blob | stride | holds |
//! |------|-------:|-------|
//! | `positions` | 12 | `x`, `y`, `z` as `f32` |
//! | `uvs`       |  8 | `u`, `v` as `f32` |
//! | `normals`   | 12 | as `positions` |
//! | `skin`      | 24 | four × (`joint` as `u16`, `weight` as `f32`) |
//! | `colours`   | 12 | as `positions` |
//! | `loops`     |  4 | every face's indices, run together |
//! | `arity`     |  4 | how many indices each face takes from `loops` |
//!
//! The vertex count is `positions.len() / 12` and is not sent: a second copy
//! of a number is a second thing that can disagree with the first. Every other
//! channel is either empty — which is how [`PolyMesh`] spells *absent*, and the
//! blob preserves that exactly — or exactly that many elements long.
//!
//! Splitting the face list into `loops` and `arity` costs four bytes per face
//! where msgpack's own array header spent one, and buys back rather more than
//! that on the indices, which it was tagging individually. It does **not**
//! remove the per-face heap allocation on decode: `PolyMesh::faces` is a
//! `Vec<Vec<u32>>`, so one `Vec` per face is what the type costs, on either
//! format. What it removes is the tag on every index and the visitor call that
//! reads it.
//!
//! ## Refusing a bad payload
//!
//! Every length is checked against the vertex count, and every index against
//! it too. This is the same argument the atlas adapter makes for its own
//! dimension check: what receives a mesh uploads it to a GPU and draws it by
//! index, so a truncated channel or an index past the end of `positions` is a
//! panic or a driver-level read past the end, somewhere with no idea what went
//! wrong. It is refused here, where the error can still name the channel.
//!
//! Only compiled under `serde-avatar`.

use glam::{Vec2, Vec3};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::mesh::{PolyMesh, VertexSkin};
use crate::rig::skin::{Influence, MAX_INFLUENCES};

/// Bytes one vertex takes in each channel's blob.
const POSITION: usize = 12;
const UV: usize = 8;
const NORMAL: usize = 12;
const COLOUR: usize = 12;
/// A `u16` joint and an `f32` weight, [`MAX_INFLUENCES`] times over.
const SKIN: usize = MAX_INFLUENCES * 6;
/// One face index.
const INDEX: usize = 4;

/// The mesh as it travels: the same channels, byte-tagged once each.
///
/// `deny_unknown_fields` because this format is internal to a worker boundary
/// whose two halves ship together. A field this decoder does not know about is
/// a version skew, and the useful thing to do with one is say so rather than
/// quietly draw whatever survived.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    #[serde(with = "serde_bytes")]
    positions: Vec<u8>,
    #[serde(with = "serde_bytes")]
    uvs: Vec<u8>,
    #[serde(with = "serde_bytes")]
    normals: Vec<u8>,
    #[serde(with = "serde_bytes")]
    skin: Vec<u8>,
    #[serde(with = "serde_bytes")]
    colours: Vec<u8>,
    #[serde(with = "serde_bytes")]
    loops: Vec<u8>,
    #[serde(with = "serde_bytes")]
    arity: Vec<u8>,
}

fn put_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_vec3(out: &mut Vec<u8>, value: Vec3) {
    put_f32(out, value.x);
    put_f32(out, value.y);
    put_f32(out, value.z);
}

fn take_f32(bytes: &[u8; 4]) -> f32 {
    f32::from_le_bytes(*bytes)
}

fn take_vec3(chunk: &[u8; POSITION]) -> Vec3 {
    let (heads, _) = chunk.as_chunks::<4>();
    Vec3::new(
        take_f32(&heads[0]),
        take_f32(&heads[1]),
        take_f32(&heads[2]),
    )
}

/// The length a channel must have, or an error naming it.
///
/// A channel is empty (the mesh does not carry it) or exactly one entry per
/// vertex; anything between is a truncation, and the whole point of checking
/// here is that the caller still has a name to put in the message.
fn channel_len<E: serde::de::Error>(
    name: &str,
    blob: &[u8],
    stride: usize,
    vertices: usize,
) -> Result<usize, E> {
    if blob.is_empty() {
        return Ok(0);
    }
    if blob.len() != vertices * stride {
        return Err(E::custom(format!(
            "mesh {name} carries {} bytes for {vertices} vertices, which needs {} or none",
            blob.len(),
            vertices * stride
        )));
    }
    Ok(vertices)
}

impl Serialize for PolyMesh {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let vertices = self.positions.len();

        let mut positions = Vec::with_capacity(vertices * POSITION);
        for &p in &self.positions {
            put_vec3(&mut positions, p);
        }

        let mut uvs = Vec::with_capacity(self.uvs.len() * UV);
        for &uv in &self.uvs {
            put_f32(&mut uvs, uv.x);
            put_f32(&mut uvs, uv.y);
        }

        let mut normals = Vec::with_capacity(self.normals.len() * NORMAL);
        for &n in &self.normals {
            put_vec3(&mut normals, n);
        }

        let mut skin = Vec::with_capacity(self.skin.len() * SKIN);
        for influences in &self.skin {
            for influence in influences {
                skin.extend_from_slice(&influence.joint.to_le_bytes());
                put_f32(&mut skin, influence.weight);
            }
        }

        let mut colours = Vec::with_capacity(self.colours.len() * COLOUR);
        for &c in &self.colours {
            put_vec3(&mut colours, c);
        }

        let corners: usize = self.faces.iter().map(Vec::len).sum();
        let mut loops = Vec::with_capacity(corners * INDEX);
        let mut arity = Vec::with_capacity(self.faces.len() * INDEX);
        for face in &self.faces {
            // `u32` rather than `usize`: the wire has one width everywhere, and
            // a face with more than four billion corners is not a thing this
            // crate can hold in the first place.
            let len = u32::try_from(face.len()).map_err(|_| {
                serde::ser::Error::custom(format!("a face has {} corners", face.len()))
            })?;
            arity.extend_from_slice(&len.to_le_bytes());
            for &index in face {
                loops.extend_from_slice(&index.to_le_bytes());
            }
        }

        Wire {
            positions,
            uvs,
            normals,
            skin,
            colours,
            loops,
            arity,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PolyMesh {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(deserializer)?;

        if !wire.positions.len().is_multiple_of(POSITION) {
            return Err(D::Error::custom(format!(
                "mesh positions carry {} bytes, which is not a whole number of {POSITION}-byte vertices",
                wire.positions.len()
            )));
        }
        let vertices = wire.positions.len() / POSITION;

        let uv_count = channel_len::<D::Error>("uvs", &wire.uvs, UV, vertices)?;
        let normal_count = channel_len::<D::Error>("normals", &wire.normals, NORMAL, vertices)?;
        let skin_count = channel_len::<D::Error>("skin", &wire.skin, SKIN, vertices)?;
        let colour_count = channel_len::<D::Error>("colours", &wire.colours, COLOUR, vertices)?;

        let (position_chunks, _) = wire.positions.as_chunks::<POSITION>();
        let mut positions = Vec::with_capacity(vertices);
        positions.extend(position_chunks.iter().map(take_vec3));

        let (uv_chunks, _) = wire.uvs.as_chunks::<UV>();
        let mut uvs = Vec::with_capacity(uv_count);
        uvs.extend(uv_chunks.iter().map(|chunk| {
            let (heads, _) = chunk.as_chunks::<4>();
            Vec2::new(take_f32(&heads[0]), take_f32(&heads[1]))
        }));

        let (normal_chunks, _) = wire.normals.as_chunks::<NORMAL>();
        let mut normals = Vec::with_capacity(normal_count);
        normals.extend(normal_chunks.iter().map(take_vec3));

        let (colour_chunks, _) = wire.colours.as_chunks::<COLOUR>();
        let mut colours = Vec::with_capacity(colour_count);
        colours.extend(colour_chunks.iter().map(take_vec3));

        let (skin_chunks, _) = wire.skin.as_chunks::<SKIN>();
        let mut skin = Vec::with_capacity(skin_count);
        skin.extend(skin_chunks.iter().map(|chunk| {
            let (entries, _) = chunk.as_chunks::<6>();
            let mut vertex: VertexSkin = [Influence::default(); MAX_INFLUENCES];
            for (slot, entry) in vertex.iter_mut().zip(entries) {
                let (joint, weight) = entry.split_at(2);
                slot.joint = u16::from_le_bytes([joint[0], joint[1]]);
                slot.weight = f32::from_le_bytes([weight[0], weight[1], weight[2], weight[3]]);
            }
            vertex
        }));

        if !wire.arity.len().is_multiple_of(INDEX) || !wire.loops.len().is_multiple_of(INDEX) {
            return Err(D::Error::custom(
                "mesh faces carry a partial index: `loops` and `arity` are both flat `u32`",
            ));
        }
        let (arity_chunks, _) = wire.arity.as_chunks::<INDEX>();
        let (loop_chunks, _) = wire.loops.as_chunks::<INDEX>();
        let corners: usize = arity_chunks
            .iter()
            .map(|a| u32::from_le_bytes(*a) as usize)
            .sum();
        if corners != loop_chunks.len() {
            return Err(D::Error::custom(format!(
                "mesh faces declare {corners} corners in `arity` but carry {} in `loops`",
                loop_chunks.len()
            )));
        }

        let mut faces = Vec::with_capacity(arity_chunks.len());
        let mut next = 0usize;
        for a in arity_chunks {
            let len = u32::from_le_bytes(*a) as usize;
            let mut face = Vec::with_capacity(len);
            for chunk in &loop_chunks[next..next + len] {
                let index = u32::from_le_bytes(*chunk);
                // Checked here rather than left to whatever draws it: a face
                // index past the end of `positions` is an out-of-bounds panic
                // in this crate's own traversals and a read past the end of a
                // vertex buffer in a renderer's.
                if index as usize >= vertices {
                    return Err(D::Error::custom(format!(
                        "a mesh face indexes vertex {index} of {vertices}"
                    )));
                }
                face.push(index);
            }
            next += len;
            faces.push(face);
        }

        Ok(PolyMesh {
            positions,
            faces,
            uvs,
            normals,
            skin,
            colours,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mesh with every channel filled and faces of three different arities,
    /// so nothing here depends on a mesh being all-quads.
    fn furnished() -> PolyMesh {
        PolyMesh {
            positions: vec![
                Vec3::new(0.0, 1.5, -2.25),
                Vec3::new(3.0, -0.5, 0.125),
                Vec3::new(-1.0, 0.0, 4.0),
                Vec3::new(2.5, 2.5, 2.5),
                Vec3::new(-3.75, 1.0, 0.5),
            ],
            faces: vec![vec![0, 1, 2], vec![1, 2, 3, 4], vec![0, 1, 2, 3, 4]],
            uvs: vec![
                Vec2::new(0.0, 1.0),
                Vec2::new(0.25, 0.75),
                Vec2::new(0.5, 0.5),
                Vec2::new(0.75, 0.25),
                Vec2::new(1.0, 0.0),
            ],
            normals: vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Y],
            skin: (0..5)
                .map(|v| {
                    let mut vertex: VertexSkin = [Influence::default(); MAX_INFLUENCES];
                    for (slot, i) in vertex.iter_mut().zip(0u16..) {
                        slot.joint = v as u16 * 7 + i;
                        slot.weight = 1.0 / f32::from(i + 2);
                    }
                    vertex
                })
                .collect(),
            colours: vec![
                Vec3::new(0.1, 0.2, 0.3),
                Vec3::new(0.4, 0.5, 0.6),
                Vec3::ONE,
                Vec3::ZERO,
                Vec3::splat(0.5),
            ],
        }
    }

    /// Round-tripped through JSON rather than the worker's own msgpack, for
    /// the reason `avatar.rs` gives for the same choice: this crate should not
    /// take a codec dependency to prove a contract about its own types, and
    /// serde's model is the same either way.
    fn round_trip(mesh: &PolyMesh) -> PolyMesh {
        let wire = serde_json::to_string(mesh).expect("serialises");
        serde_json::from_str(&wire).expect("deserialises")
    }

    #[test]
    fn every_channel_survives_bit_exactly() {
        let mesh = furnished();
        assert_eq!(round_trip(&mesh), mesh);
    }

    /// An empty channel is how `PolyMesh` spells *absent*, and a mesh that
    /// came back carrying five zeroed normals instead of none would be
    /// silently reshaded rather than obviously broken.
    #[test]
    fn an_absent_channel_stays_absent() {
        let mesh = PolyMesh {
            positions: furnished().positions,
            faces: vec![vec![0, 1, 2]],
            ..PolyMesh::default()
        };
        let back = round_trip(&mesh);
        assert!(back.uvs.is_empty(), "uvs came back present");
        assert!(back.normals.is_empty(), "normals came back present");
        assert!(back.skin.is_empty(), "skin came back present");
        assert!(back.colours.is_empty(), "colours came back present");
        assert_eq!(back, mesh);
    }

    #[test]
    fn a_mesh_with_nothing_in_it_round_trips() {
        assert_eq!(round_trip(&PolyMesh::default()), PolyMesh::default());
    }

    /// The format itself, pinned. A derived impl writes `positions` as a
    /// sequence of three-element sequences and `skin` as a sequence of named
    /// maps; this asserts the flat blobs that replaced them, which is the
    /// whole change.
    #[test]
    fn the_channels_go_out_as_flat_blobs() {
        let mesh = furnished();
        let value: serde_json::Value = serde_json::to_value(&mesh).expect("serialises");
        let object = value.as_object().expect("a struct");
        let mut names: Vec<&str> = object.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "arity",
                "colours",
                "loops",
                "normals",
                "positions",
                "skin",
                "uvs"
            ]
        );
        let blob = |name: &str| object[name].as_array().expect("a byte blob").len();
        assert_eq!(blob("positions"), 5 * POSITION);
        assert_eq!(blob("uvs"), 5 * UV);
        assert_eq!(blob("normals"), 5 * NORMAL);
        assert_eq!(blob("skin"), 5 * SKIN);
        assert_eq!(blob("colours"), 5 * COLOUR);
        assert_eq!(blob("arity"), 3 * INDEX);
        assert_eq!(blob("loops"), (3 + 4 + 5) * INDEX);
    }

    /// Tamper with one blob and read back the refusal.
    fn refuse(edit: impl FnOnce(&mut serde_json::Map<String, serde_json::Value>)) -> String {
        let mut value = serde_json::to_value(furnished()).expect("serialises");
        edit(value.as_object_mut().expect("a struct"));
        match serde_json::from_value::<PolyMesh>(value) {
            Ok(mesh) => panic!("a malformed mesh was accepted: {mesh:?}"),
            Err(error) => error.to_string(),
        }
    }

    /// A channel short for its vertex count is the failure the derived impl
    /// could not have: parallel arrays of different lengths break the
    /// invariant the whole module documents, and every consumer indexes them
    /// with one loop counter.
    #[test]
    fn a_truncated_channel_is_refused_by_name() {
        let error = refuse(|object| {
            let skin = object["skin"].as_array_mut().expect("a blob");
            skin.truncate(SKIN);
        });
        assert!(
            error.contains("skin"),
            "the refusal should name it: {error}"
        );
    }

    #[test]
    fn a_channel_that_is_not_a_whole_number_of_vertices_is_refused() {
        let error = refuse(|object| {
            object["positions"]
                .as_array_mut()
                .expect("a blob")
                .truncate(5 * POSITION - 2);
        });
        assert!(
            error.contains("vertices"),
            "the refusal should say what is partial: {error}"
        );
    }

    /// The one that matters most: an index past the end of `positions` is an
    /// out-of-bounds panic in this crate's traversals and a read past the end
    /// of a vertex buffer in a renderer's. The derived impl accepted it.
    #[test]
    fn a_face_index_past_the_end_is_refused() {
        let error = refuse(|object| {
            let loops = object["loops"].as_array_mut().expect("a blob");
            // The first index, widened to a vertex that does not exist.
            loops[0] = serde_json::json!(99u8);
        });
        assert!(
            error.contains("indexes vertex"),
            "the refusal should name the index: {error}"
        );
    }

    #[test]
    fn a_face_list_that_disagrees_with_itself_is_refused() {
        let error = refuse(|object| {
            object["loops"]
                .as_array_mut()
                .expect("a blob")
                .truncate(4 * INDEX);
        });
        assert!(
            error.contains("corners"),
            "the refusal should say the two halves disagree: {error}"
        );
    }
}
