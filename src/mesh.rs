//! Polygon mesh container shared by the cage builder and the subdivider.
//!
//! Faces are arbitrary convex polygons wound counter-clockwise when viewed from
//! outside the surface, so a swept quad tube and a triangulated hull patch can
//! live in one mesh. [`PolyMesh::manifold_report`] is the workhorse diagnostic:
//! a correct body cage is a closed, consistently wound 2-manifold, and every
//! stage of the pipeline is checked against that.
//!
//! ## Vertex attributes
//!
//! Positions are not enough to draw with. A mesh may carry four further
//! **channels** — texture coordinates, normals, skin influences, and colours —
//! each of which is either *absent* (empty) or exactly as long as `positions`.
//! Nothing is forced to carry them: a control cage has no use for UVs, and the
//! joint hulls it is built from would have nothing sensible to put there.
//!
//! Every operation that changes the vertex list keeps that invariant. In
//! particular [`PolyMesh::append`] pads whichever side is missing a channel the
//! other has, so merging a UV-mapped part into an un-mapped one cannot silently
//! shift every coordinate after the join — which is the failure mode parallel
//! arrays are famous for.
//!
//! ## Seams
//!
//! A tube's texture coordinates wrap: the last column of faces runs from `u`
//! near one back round to zero. Splitting the vertices that straddle it is the
//! only fix, and it costs the mesh its closed-manifold property — so it is not
//! done at build time. Authoring meshes stay closed and shareable;
//! [`PolyMesh::split_uv_seams`] makes the render-ready copy.

use glam::{Mat4, Vec2, Vec3};
use std::collections::HashMap;
use std::fmt::Write as _;

use crate::rig::skin::{Influence, MAX_INFLUENCES};

/// The two ends of an edge, in the order that makes both its faces agree.
///
/// A shared edge is visited once from each side, in opposite directions. Keying
/// on the ordered pair is what lets the second visit find the midpoint the first
/// one made, instead of building a duplicate and tearing the surface along it.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// How tight a fold [`PolyMesh::crease`] calls fully creased, in metres.
///
/// A curvature radius, not a gain, and that is what makes the measure independent
/// of how finely anything is tessellated. The discrete estimate below recovers a
/// mean curvature in reciprocal metres; multiplying by a length turns it into a
/// number, and the length says which folds count. Measured against cylindrical
/// arcs of known radius, a fold of radius `r` reads `CREASE_FOLD / 4r`: at
/// `0.035` anything tighter than 9 mm shades fully, the fillet where a jaw meets
/// a neck reads about `0.3`, and a curve on the scale of a waist stays near zero.
///
/// Tessellation independence is not academic here. The body is subdivided and
/// the attached parts are not, so a measure that scaled with edge length would
/// read the same physical fold differently on either side of a wrist — and put a
/// shading seam exactly where a hand meets an arm. It holds for surfaces that
/// curve smoothly, which is what a subdivided body is; a hard fold is a curvature
/// singularity and reads sharper the more finely it is cut, which is correct but
/// means nothing here reads a hard edge as a cavity by accident.
const CREASE_FOLD: f32 = 0.035;

/// A vertex's bone influences, strongest first — one entry of [`PolyMesh::skin`].
pub type VertexSkin = [Influence; MAX_INFLUENCES];

/// A polygon soup with shared vertices.
///
/// See the [module documentation](self#vertex-attributes) for the rule every
/// attribute channel obeys.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PolyMesh {
    /// Vertex positions, indexed by the entries of [`PolyMesh::faces`].
    pub positions: Vec<Vec3>,
    /// Face loops, each a counter-clockwise ring of indices into `positions`.
    pub faces: Vec<Vec<u32>>,
    /// Texture coordinate per vertex, or empty when the mesh carries none.
    pub uvs: Vec<Vec2>,
    /// Explicit normal per vertex, or empty to derive them from the surface.
    ///
    /// Worth carrying only where the derived ones would be wrong: after a seam
    /// split, neighbouring copies of one vertex are no longer joined, so
    /// [`PolyMesh::vertex_normals`] would crease the surface along every seam.
    pub normals: Vec<Vec3>,
    /// Bone influences per vertex, or empty when the mesh is not skinned.
    pub skin: Vec<VertexSkin>,
    /// Linear-space colour per vertex, or empty when the mesh is untinted.
    ///
    /// This is what lets parts of one material merge into a single draw and
    /// still differ: every lock of hair is its own shade, and drawn as one solid
    /// in one colour a head of hair reads as a helmet.
    pub colours: Vec<Vec3>,
}

impl PolyMesh {
    /// An empty mesh.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of faces.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Appends a vertex and returns its index.
    ///
    /// Any channel the mesh already carries grows with it, so the invariant
    /// holds however a mesh is built up.
    pub fn push_vertex(&mut self, position: Vec3) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.pad_channels();
        index
    }

    /// Appends a vertex carrying a texture coordinate, and returns its index.
    ///
    /// Starts the UV channel if the mesh had none, back-filling the vertices
    /// already present.
    pub fn push_uv_vertex(&mut self, position: Vec3, uv: Vec2) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        self.uvs.resize(index as usize, Vec2::ZERO);
        self.uvs.push(uv);
        self.pad_channels();
        index
    }

    /// Appends a face loop.
    pub fn push_face(&mut self, face: impl Into<Vec<u32>>) {
        self.faces.push(face.into());
    }

    /// Grows every non-empty channel to one entry per vertex.
    fn pad_channels(&mut self) {
        let vertices = self.positions.len();
        if !self.uvs.is_empty() {
            self.uvs.resize(vertices, Vec2::ZERO);
        }
        if !self.normals.is_empty() {
            self.normals.resize(vertices, Vec3::Y);
        }
        if !self.skin.is_empty() {
            self.skin.resize(vertices, VertexSkin::default());
        }
        if !self.colours.is_empty() {
            self.colours.resize(vertices, Vec3::ONE);
        }
    }

    /// Whether every channel present has one entry per vertex.
    ///
    /// The invariant the whole module rests on, asserted in tests rather than
    /// checked on every call.
    #[must_use]
    pub fn channels_are_consistent(&self) -> bool {
        let vertices = self.positions.len();
        let fits = |len: usize| len == 0 || len == vertices;
        fits(self.uvs.len())
            && fits(self.normals.len())
            && fits(self.skin.len())
            && fits(self.colours.len())
    }

    /// Attaches texture coordinates.
    ///
    /// # Panics
    ///
    /// Panics if `uvs` is neither empty nor one per vertex. A mismatch is a
    /// generator bug, not bad input: nothing downstream could do anything
    /// sensible with a half-mapped mesh.
    #[track_caller]
    pub fn set_uvs(&mut self, uvs: Vec<Vec2>) {
        assert!(
            uvs.is_empty() || uvs.len() == self.positions.len(),
            "{} uvs for {} vertices",
            uvs.len(),
            self.positions.len()
        );
        self.uvs = uvs;
    }

    /// Attaches explicit normals.
    ///
    /// # Panics
    ///
    /// Panics if `normals` is neither empty nor one per vertex.
    #[track_caller]
    pub fn set_normals(&mut self, normals: Vec<Vec3>) {
        assert!(
            normals.is_empty() || normals.len() == self.positions.len(),
            "{} normals for {} vertices",
            normals.len(),
            self.positions.len()
        );
        self.normals = normals;
    }

    /// Attaches bone influences.
    ///
    /// # Panics
    ///
    /// Panics if `skin` is neither empty nor one per vertex.
    #[track_caller]
    pub fn set_skin(&mut self, skin: Vec<VertexSkin>) {
        assert!(
            skin.is_empty() || skin.len() == self.positions.len(),
            "{} skin entries for {} vertices",
            skin.len(),
            self.positions.len()
        );
        self.skin = skin;
    }

    /// Binds every vertex rigidly to one joint.
    ///
    /// How an attached part joins the body's skinned mesh: a hand does not
    /// deform, it rides the wrist, and saying so is all the skinning it needs.
    pub fn bind_rigidly(&mut self, joint: u16) {
        let mut influences = VertexSkin::default();
        influences[0] = Influence { joint, weight: 1.0 };
        self.skin = vec![influences; self.positions.len()];
    }

    /// Paints every vertex one colour.
    pub fn paint(&mut self, colour: Vec3) {
        self.colours = vec![colour; self.positions.len()];
    }

    /// Attaches a colour per vertex.
    ///
    /// # Panics
    ///
    /// Panics if `colours` is neither empty nor one per vertex.
    #[track_caller]
    pub fn set_colours(&mut self, colours: Vec<Vec3>) {
        assert!(
            colours.is_empty() || colours.len() == self.positions.len(),
            "{} colours for {} vertices",
            colours.len(),
            self.positions.len()
        );
        self.colours = colours;
    }

    /// The mesh's normals: the explicit ones if it carries any, else derived.
    #[must_use]
    pub fn shading_normals(&self) -> Vec<Vec3> {
        if self.normals.is_empty() {
            self.vertex_normals()
        } else {
            self.normals.clone()
        }
    }

    /// A copy of this mesh with every vertex transformed.
    ///
    /// A mirroring transform reverses each face's winding to match, so a
    /// left-handed copy of a right-handed part still faces outward. Normals go
    /// through the inverse transpose, which handles the mirror as well as any
    /// non-uniform scale; texture coordinates, influences and colours are
    /// properties of the vertex rather than of where it sits, and ride along
    /// unchanged.
    #[must_use]
    pub fn transformed(&self, transform: Mat4) -> PolyMesh {
        let mirrored = transform.determinant() < 0.0;
        let reorient = transform.inverse().transpose();
        PolyMesh {
            positions: self
                .positions
                .iter()
                .map(|&point| transform.transform_point3(point))
                .collect(),
            faces: self
                .faces
                .iter()
                .map(|face| {
                    if mirrored {
                        face.iter().rev().copied().collect()
                    } else {
                        face.clone()
                    }
                })
                .collect(),
            normals: self
                .normals
                .iter()
                .map(|&normal| reorient.transform_vector3(normal).normalize_or(normal))
                .collect(),
            uvs: self.uvs.clone(),
            skin: self.skin.clone(),
            colours: self.colours.clone(),
        }
    }

    /// Appends another mesh, re-indexing its faces.
    ///
    /// Channels are unioned: one the other mesh carries and this one does not is
    /// started here with defaults for the vertices already present, and one this
    /// mesh carries and the other does not is padded out over the new vertices.
    /// The alternative — dropping the channel, or letting the arrays fall out of
    /// step — turns a merge into silently wrong texturing.
    pub fn append(&mut self, other: &PolyMesh) {
        let base = self.positions.len() as u32;
        let joined = self.positions.len() + other.positions.len();
        self.positions.extend_from_slice(&other.positions);
        self.faces.extend(
            other
                .faces
                .iter()
                .map(|face| face.iter().map(|index| index + base).collect::<Vec<u32>>()),
        );

        merge_channel(&mut self.uvs, &other.uvs, base as usize, joined, Vec2::ZERO);
        merge_channel(
            &mut self.normals,
            &other.normals,
            base as usize,
            joined,
            Vec3::Y,
        );
        merge_channel(
            &mut self.skin,
            &other.skin,
            base as usize,
            joined,
            VertexSkin::default(),
        );
        merge_channel(
            &mut self.colours,
            &other.colours,
            base as usize,
            joined,
            Vec3::ONE,
        );
    }

    /// A copy whose texture coordinates no longer wrap across a seam.
    ///
    /// A face straddling the wrap has its corners bunched at both ends of `u`
    /// with nothing in between; the low ones are duplicated with `u` lifted by a
    /// whole turn, which makes the face continuous again. The same trick the
    /// body's unwrap uses, applied to parts that were built as closed solids.
    ///
    /// Emptiness in the middle is what identifies the wrap, and it is the reason
    /// this looks for a *gap* rather than a wide span. A tube's end cap also
    /// spans the whole of `u` — its corners sit at every angle around the ring
    /// at once — but they are spread evenly, because it is a pole and not a
    /// seam. Lifting half of one only moves the smear. Caps are therefore left
    /// as they are: see the note on end caps in [`crate::prim`].
    ///
    /// The result is **not** a closed manifold — a seam is a cut — so this is
    /// the last step before drawing, never part of authoring. A mesh carrying no
    /// UVs is returned unchanged.
    #[must_use]
    pub fn split_uv_seams(&self) -> PolyMesh {
        if self.uvs.is_empty() {
            return self.clone();
        }
        let mut out = self.clone();
        out.faces.clear();
        // One duplicate per original vertex at most: every straddling face lifts
        // the same corners by the same whole turn.
        let mut lifted: HashMap<u32, u32> = HashMap::new();

        for face in &self.faces {
            let Some(below) = wrap_threshold(face, &self.uvs) else {
                out.faces.push(face.clone());
                continue;
            };
            let mapped: Vec<u32> = face
                .iter()
                .map(|&corner| {
                    if self.uvs[corner as usize].x > below {
                        return corner;
                    }
                    *lifted.entry(corner).or_insert_with(|| {
                        let at = out.positions.len() as u32;
                        out.positions.push(self.positions[corner as usize]);
                        out.uvs
                            .push(self.uvs[corner as usize] + Vec2::new(1.0, 0.0));
                        if !out.normals.is_empty() {
                            out.normals.push(self.normals[corner as usize]);
                        }
                        if !out.skin.is_empty() {
                            out.skin.push(self.skin[corner as usize]);
                        }
                        if !out.colours.is_empty() {
                            out.colours.push(self.colours[corner as usize]);
                        }
                        at
                    })
                })
                .collect();
            out.faces.push(mapped);
        }

        out
    }

    /// A copy with every texture coordinate mapped into `rect`.
    ///
    /// How a part takes its place in the shared atlas: it is generated in its own
    /// unit square and moved into the region reserved for it, so the generator
    /// never has to know what else is being packed alongside.
    #[must_use]
    pub fn uvs_within(&self, min: Vec2, size: Vec2) -> PolyMesh {
        let mut out = self.clone();
        for uv in &mut out.uvs {
            *uv = min + *uv * size;
        }
        out
    }

    /// Fraction of faces that are quads, in `0.0..=1.0` (`1.0` for an empty mesh).
    ///
    /// Catmull-Clark drives this to `1.0` after a single level, so it doubles as
    /// a check that the subdivider ran.
    #[must_use]
    pub fn quad_fraction(&self) -> f32 {
        if self.faces.is_empty() {
            return 1.0;
        }
        let quads = self.faces.iter().filter(|f| f.len() == 4).count();
        quads as f32 / self.faces.len() as f32
    }

    /// Centroid of face `index`, or the origin if the face is empty.
    #[must_use]
    pub fn face_centroid(&self, index: usize) -> Vec3 {
        let face = &self.faces[index];
        if face.is_empty() {
            return Vec3::ZERO;
        }
        let sum: Vec3 = face.iter().map(|&i| self.positions[i as usize]).sum();
        sum / face.len() as f32
    }

    /// Axis-aligned bounds as `(min, max)`, or `(ZERO, ZERO)` when empty.
    #[must_use]
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut iter = self.positions.iter().copied();
        let Some(first) = iter.next() else {
            return (Vec3::ZERO, Vec3::ZERO);
        };
        iter.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)))
    }

    /// Smooth per-vertex normals, area-weighted across incident faces.
    ///
    /// Weighting by face area rather than averaging unit normals keeps a ring of
    /// small faces from outvoting the large face they abut, which is what makes
    /// a subdivided limb shade evenly instead of banding at every ring.
    #[must_use]
    pub fn vertex_normals(&self) -> Vec<Vec3> {
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for face in &self.faces {
            if face.len() < 3 {
                continue;
            }
            let anchor = self.positions[face[0] as usize];
            // The un-normalised cross product is twice the triangle's area, so
            // summing them weights by area for free.
            let weighted: Vec3 = (1..face.len() - 1)
                .map(|corner| {
                    let a = self.positions[face[corner] as usize] - anchor;
                    let b = self.positions[face[corner + 1] as usize] - anchor;
                    a.cross(b)
                })
                .sum();
            for &index in face {
                normals[index as usize] += weighted;
            }
        }
        for normal in &mut normals {
            *normal = normal.normalize_or(Vec3::Y);
        }
        normals
    }

    /// Splits the selected faces into four, leaving the rest of the mesh alone.
    ///
    /// Resolution where it is needed and nowhere else. A face has to be carried
    /// by the surface it is part of, and the head arrives from the cage as a
    /// four-sided tube: subdivided twice it is 189 faces with a **mean edge of
    /// 24 mm**, while a brow ridge is 10 mm tall and a nose one quad wide. There
    /// is nothing there to shape (#59). Subdividing the whole body again would
    /// buy that at four times the triangles everywhere, most of them on a shin.
    ///
    /// **Linear, not smooth.** New vertices are plain midpoints and centroids
    /// and no existing vertex moves, so this adds places to put detail without
    /// changing the shape. That separation is the point: refining and reshaping
    /// in one step makes it impossible to tell which of them moved a silhouette.
    ///
    /// **No T-junctions.** An unselected face that borders a selected one takes
    /// the new edge midpoints into its own corner list and becomes an n-gon,
    /// rather than being left with a vertex hanging in the middle of one of its
    /// edges — which is a crack the moment anything shades or deforms it.
    ///
    /// Positions only, like [`crate::subdiv::catmull_clark`], because like it
    /// this belongs to the rest mesh before anything is bound or unwrapped: a
    /// vertex that does not exist yet cannot have been given a weight or a
    /// texel. `selected` is indexed by face; a shorter slice refines nothing
    /// past its end.
    #[must_use]
    pub fn refine(&self, selected: &[bool]) -> PolyMesh {
        let chosen = |face: usize| selected.get(face).copied().unwrap_or(false);
        if !(0..self.faces.len()).any(chosen) {
            return self.clone();
        }

        let mut refined = PolyMesh::new();
        refined.positions.clone_from(&self.positions);

        // One midpoint per edge of a selected face, shared with whatever is on
        // the other side of it. Keyed on the ordered pair so the two faces
        // meeting at an edge find the same vertex.
        let mut midpoints: HashMap<(u32, u32), u32> = HashMap::new();
        for (index, face) in self.faces.iter().enumerate() {
            if !chosen(index) || face.len() < 3 {
                continue;
            }
            for corner in 0..face.len() {
                let next = (corner + 1) % face.len();
                let (a, b) = (face[corner], face[next]);
                if let std::collections::hash_map::Entry::Vacant(slot) =
                    midpoints.entry(edge_key(a, b))
                {
                    let at = (self.positions[a as usize] + self.positions[b as usize]) * 0.5;
                    slot.insert(refined.push_vertex(at));
                }
            }
        }

        for (index, face) in self.faces.iter().enumerate() {
            if face.len() < 3 {
                continue;
            }
            if chosen(index) {
                let centre = refined.push_vertex(self.face_centroid(index));
                for corner in 0..face.len() {
                    let previous = (corner + face.len() - 1) % face.len();
                    let next = (corner + 1) % face.len();
                    let ahead = midpoints[&edge_key(face[corner], face[next])];
                    let behind = midpoints[&edge_key(face[previous], face[corner])];
                    refined.push_face([behind, face[corner], ahead, centre]);
                }
            } else {
                // Absorb any midpoint a neighbour put on one of our edges.
                let mut corners = Vec::with_capacity(face.len());
                for corner in 0..face.len() {
                    let next = (corner + 1) % face.len();
                    corners.push(face[corner]);
                    if let Some(&between) = midpoints.get(&edge_key(face[corner], face[next])) {
                        corners.push(between);
                    }
                }
                refined.push_face(corners);
            }
        }

        refined
    }

    /// How deeply each vertex sits in a cavity: `0` on flat or convex surface,
    /// rising toward `1` in a fold.
    ///
    /// Measured from the surface itself rather than from the skeleton, which is
    /// the whole point. Comparing a normal against the direction away from the
    /// nearest bone only works where the surface was swept around that bone; an
    /// attached part — a hand, a nose, an ear — sits *past* the end of the
    /// nearest bone, so the direction away from it is the limb's own axis while
    /// the part's normals point every other way. That reads as a deep crease
    /// over the whole part, and cavity shading then darkened every hand, foot
    /// and facial feature by up to 35% (#63).
    ///
    /// The measure here is the discrete mean curvature: where the neighbours of
    /// a vertex sit relative to its tangent plane. Neighbours *above* the plane,
    /// along the normal, mean the surface folds back on itself — an armpit, a
    /// crotch, the cleft between two fingers. Neighbours below it mean a convex
    /// bulge, which is what a fingertip and a nose are. The offset scales with
    /// the square of the edge length, so dividing by it recovers a curvature in
    /// reciprocal metres, and a reference fold size turns it back into a number:
    /// a fold of radius `r` reads `0.035 / 4r`, so anything tighter than 9 mm
    /// shades fully and a curve on the scale of a waist stays near zero.
    #[must_use]
    pub fn crease(&self) -> Vec<f32> {
        let normals = self.vertex_normals();
        let mut sum = vec![Vec3::ZERO; self.positions.len()];
        let mut span = vec![0.0f32; self.positions.len()];
        let mut count = vec![0u32; self.positions.len()];

        for face in &self.faces {
            for corner in 0..face.len() {
                let here = face[corner] as usize;
                let next = face[(corner + 1) % face.len()] as usize;
                let edge = self.positions[next] - self.positions[here];
                // Both ends, so one walk over the faces covers every edge from
                // both directions and no vertex is left with an empty ring.
                sum[here] += edge;
                span[here] += edge.length();
                count[here] += 1;
                sum[next] -= edge;
                span[next] += edge.length();
                count[next] += 1;
            }
        }

        (0..self.positions.len())
            .map(|index| {
                if count[index] == 0 {
                    return 0.0;
                }
                let neighbours = count[index] as f32;
                let offset = sum[index] / neighbours;
                let length = span[index] / neighbours;
                if length <= 1e-9 {
                    return 0.0;
                }
                // Twice the mean curvature, near enough: the offset of a
                // neighbour ring from the tangent plane goes as curvature times
                // the square of its radius.
                let curvature = offset.dot(normals[index]) / (length * length);
                (curvature * CREASE_FOLD).clamp(0.0, 1.0)
            })
            .collect()
    }

    /// Fan-triangulates every face, for renderers that only take triangles.
    #[must_use]
    pub fn triangulated(&self) -> Vec<[u32; 3]> {
        let mut tris = Vec::new();
        for face in &self.faces {
            for i in 1..face.len().saturating_sub(1) {
                tris.push([face[0], face[i], face[i + 1]]);
            }
        }
        tris
    }

    /// Whether `point` lies inside this mesh.
    ///
    /// Exact for a closed, consistently wound manifold — which every authoring
    /// mesh in this crate is, and which [`PolyMesh::is_closed_manifold`] checks.
    /// A ray is cast along `+X` and its crossings counted: an odd number means
    /// the point started inside.
    ///
    /// This is the primitive the between-part checks are written against. The
    /// obvious alternatives are both wrong in the places it matters: nearest
    /// vertex plus its normal mis-reports any point sitting in a concave region
    /// — a mouth between a nose and a chin is exactly that — and comparing
    /// against a profile of the body reports a well-fitted part as buried,
    /// because a part that hugs a curved surface is behind its own midline.
    ///
    /// Rays that graze an edge are the classic failure. This one is nudged off
    /// the axes by an irrational-ish slope, so a shared edge is never hit twice.
    #[must_use]
    pub fn contains(&self, point: Vec3) -> bool {
        let direction = Vec3::new(1.0, 0.017_321, 0.010_101).normalize();
        let mut crossings = 0usize;
        // Fanned in place rather than through `triangulated`, which allocates:
        // this is called once per vertex of every attached part, over every seed
        // of a sweep, and a body carries a few thousand faces.
        for face in &self.faces {
            // Most of them cannot be hit at all. Bound where the ray can be by
            // the time it reaches this face's slab of `x`, and skip the face if
            // that window misses its extent. Conservative in both directions, so
            // no crossing is ever lost.
            let (lo, hi) = face_bounds(&self.positions, face);
            if hi.x <= point.x {
                continue;
            }
            let near = ((lo.x - point.x) / direction.x).max(0.0);
            let far = (hi.x - point.x) / direction.x;
            let misses = |axis: usize| {
                let from = point[axis] + direction[axis] * near;
                let to = point[axis] + direction[axis] * far;
                from.min(to) > hi[axis] || from.max(to) < lo[axis]
            };
            if misses(1) || misses(2) {
                continue;
            }

            let anchor = self.positions[face[0] as usize];
            for corner in 1..face.len().saturating_sub(1) {
                let b = self.positions[face[corner] as usize];
                let c = self.positions[face[corner + 1] as usize];
                if ray_hits_triangle(point, direction, anchor, b, c) {
                    crossings += 1;
                }
            }
        }
        crossings % 2 == 1
    }

    /// Audits topology and winding.
    #[must_use]
    pub fn manifold_report(&self) -> ManifoldReport {
        let mut report = ManifoldReport::default();
        // Directed edge -> use count. A closed, consistently wound manifold uses
        // every directed edge exactly once, and its reverse exactly once.
        let mut directed: HashMap<(u32, u32), usize> = HashMap::new();

        for face in &self.faces {
            let degenerate = face.len() < 3
                || face
                    .iter()
                    .enumerate()
                    .any(|(i, a)| face[i + 1..].iter().any(|b| a == b));
            if degenerate {
                report.degenerate_faces += 1;
                continue;
            }
            if face.iter().any(|&i| i as usize >= self.positions.len()) {
                report.out_of_range += 1;
                continue;
            }
            for i in 0..face.len() {
                let a = face[i];
                let b = face[(i + 1) % face.len()];
                *directed.entry((a, b)).or_default() += 1;
            }
        }

        for (&(a, b), &count) in &directed {
            if count > 1 {
                report.inconsistent_edges += 1;
            }
            let opposite = directed.get(&(b, a)).copied().unwrap_or(0);
            if opposite == 0 {
                report.boundary_edges += 1;
            } else if count + opposite > 2 {
                report.nonmanifold_edges += 1;
            }
        }

        report
    }

    /// Whether the mesh is a closed, consistently wound 2-manifold.
    #[must_use]
    pub fn is_closed_manifold(&self) -> bool {
        self.manifold_report().is_clean()
    }

    /// Serialises to Wavefront OBJ, keeping n-gons intact.
    ///
    /// Dependency-free debug affordance: dump a cage, open it in any DCC tool,
    /// and read the edge flow directly. Texture coordinates and normals are
    /// written when the mesh carries them, so a chart can be looked at in the
    /// same way rather than taken on trust.
    #[must_use]
    pub fn to_obj(&self) -> String {
        let mut out = String::new();
        out.push_str("# symbios-avatar mesh dump\n");
        for p in &self.positions {
            let _ = writeln!(out, "v {} {} {}", p.x, p.y, p.z);
        }
        for uv in &self.uvs {
            let _ = writeln!(out, "vt {} {}", uv.x, uv.y);
        }
        for normal in &self.normals {
            let _ = writeln!(out, "vn {} {} {}", normal.x, normal.y, normal.z);
        }
        let mapped = !self.uvs.is_empty();
        let shaded = !self.normals.is_empty();
        for face in &self.faces {
            out.push('f');
            for &i in face {
                let at = i + 1;
                let _ = match (mapped, shaded) {
                    (false, false) => write!(out, " {at}"),
                    (true, false) => write!(out, " {at}/{at}"),
                    (false, true) => write!(out, " {at}//{at}"),
                    (true, true) => write!(out, " {at}/{at}/{at}"),
                };
            }
            out.push('\n');
        }
        out
    }
}

/// Axis-aligned bounds of one face.
fn face_bounds(positions: &[Vec3], face: &[u32]) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for &corner in face {
        let point = positions[corner as usize];
        lo = lo.min(point);
        hi = hi.max(point);
    }
    (lo, hi)
}

/// Whether a ray from `origin` along `direction` crosses the triangle.
///
/// Moller-Trumbore, front and back faces alike: a crossing count does not care
/// which way a face points, only how many it passed through.
fn ray_hits_triangle(origin: Vec3, direction: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let edge1 = b - a;
    let edge2 = c - a;
    let across = direction.cross(edge2);
    let determinant = edge1.dot(across);
    if determinant.abs() < 1e-12 {
        return false;
    }
    let inverse = 1.0 / determinant;
    let to_origin = origin - a;
    let u = to_origin.dot(across) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let along = to_origin.cross(edge1);
    let v = direction.dot(along) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    edge2.dot(along) * inverse > 1e-9
}

/// The `u` below which a face's corners belong to the far side of the wrap.
///
/// `None` when the face does not straddle a seam. A wrapping face leaves half
/// the chart empty between its two clusters of corners, so the widest gap
/// between neighbouring `u` values identifies both that it wraps and where the
/// cut falls. A face merely spanning a lot of `u` — a tube's end cap, whose
/// corners are spread evenly all the way round — has no such gap.
fn wrap_threshold(face: &[u32], uvs: &[Vec2]) -> Option<f32> {
    let mut spread: Vec<f32> = face.iter().map(|&c| uvs[c as usize].x).collect();
    spread.sort_by(f32::total_cmp);
    spread.dedup();
    spread
        .windows(2)
        .map(|pair| (pair[1] - pair[0], pair[0]))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|(gap, _)| *gap > 0.5)
        .map(|(_, below)| below)
}

/// Joins one attribute channel onto another during [`PolyMesh::append`].
///
/// `base` is how many vertices the receiving mesh had and `joined` how many it
/// has now. Whichever side lacks the channel is filled with `absent`, so the
/// channel is present on the result exactly when either input carried it.
fn merge_channel<T: Clone>(into: &mut Vec<T>, from: &[T], base: usize, joined: usize, absent: T) {
    if into.is_empty() && from.is_empty() {
        return;
    }
    into.resize(base, absent.clone());
    into.extend_from_slice(from);
    into.resize(joined, absent);
}

/// Result of auditing a [`PolyMesh`]'s topology and winding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManifoldReport {
    /// Directed edges whose reverse is missing — the surface has a hole here.
    pub boundary_edges: usize,
    /// Undirected edges shared by more than two faces.
    pub nonmanifold_edges: usize,
    /// Directed edges emitted more than once — two faces wound the same way.
    pub inconsistent_edges: usize,
    /// Faces with fewer than three corners or a repeated corner.
    pub degenerate_faces: usize,
    /// Faces referencing a vertex index past the end of `positions`.
    pub out_of_range: usize,
}

impl ManifoldReport {
    /// Whether every audited property is defect-free.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.boundary_edges == 0
            && self.nonmanifold_edges == 0
            && self.inconsistent_edges == 0
            && self.degenerate_faces == 0
            && self.out_of_range == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit cube, faces wound counter-clockwise seen from outside.
    fn cube() -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for corner in [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.0, 1.0, 1.0),
        ] {
            mesh.push_vertex(corner);
        }
        for face in [
            [0u32, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ] {
            mesh.push_face(face);
        }
        mesh
    }

    #[test]
    fn cube_is_a_closed_manifold() {
        let mesh = cube();
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());
        assert_eq!(mesh.quad_fraction(), 1.0);
        assert_eq!(mesh.triangulated().len(), 12);
    }

    #[test]
    fn open_box_reports_boundary_edges() {
        let mut mesh = cube();
        mesh.faces.pop();
        let report = mesh.manifold_report();
        assert_eq!(report.boundary_edges, 4);
        assert!(!report.is_clean());
    }

    #[test]
    fn flipped_face_reports_inconsistent_winding() {
        let mut mesh = cube();
        mesh.faces[0].reverse();
        let report = mesh.manifold_report();
        assert!(report.inconsistent_edges > 0);
    }

    #[test]
    fn bounds_and_centroid_are_reported() {
        let mesh = cube();
        let (lo, hi) = mesh.bounds();
        assert_eq!(lo, Vec3::ZERO);
        assert_eq!(hi, Vec3::ONE);
        assert_eq!(mesh.face_centroid(0), Vec3::new(0.5, 0.5, 0.0));
    }

    #[test]
    fn obj_round_trips_counts() {
        let obj = cube().to_obj();
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 8);
        assert_eq!(obj.lines().filter(|l| l.starts_with('f')).count(), 6);
    }

    /// A strip bent around a cylinder of `radius`, `cells` quads along the arc.
    ///
    /// `concave` winds it so its normals point at the axis — the inside of the
    /// curve, which is what a cavity is — rather than away from it. Curvature is
    /// then known in advance, which is what makes it worth testing against.
    fn arc(cells: usize, radius: f32, concave: bool) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        let sweep = std::f32::consts::FRAC_PI_2;
        let width = radius * sweep / cells as f32;
        for i in 0..=cells {
            let angle = sweep * i as f32 / cells as f32;
            let point = Vec3::new(0.0, radius * angle.cos(), radius * angle.sin());
            mesh.push_vertex(point);
            mesh.push_vertex(point + Vec3::X * width);
        }
        for segment in 0..cells {
            let base = (segment * 2) as u32;
            if concave {
                mesh.push_face([base, base + 1, base + 3, base + 2]);
            } else {
                mesh.push_face([base, base + 2, base + 3, base + 1]);
            }
        }
        mesh
    }

    fn peak_crease(mesh: &PolyMesh) -> f32 {
        mesh.crease().iter().copied().fold(0.0f32, f32::max)
    }

    #[test]
    fn refining_adds_vertices_without_moving_any() {
        // The whole reason refinement is linear: it buys places to put detail
        // and changes nothing about the shape, so anything that moves after it
        // moved because it was reshaped and not because it was refined.
        let cube = cube();
        let before = cube.positions.clone();
        let refined = cube.refine(&vec![true; cube.face_count()]);

        assert_eq!(
            &refined.positions[..before.len()],
            &before[..],
            "refinement moved a vertex that already existed"
        );
        // Each quad becomes four, and every new vertex is a midpoint or centre.
        assert_eq!(refined.face_count(), cube.face_count() * 4);
        assert!(refined.is_closed_manifold(), "refinement tore the surface");

        let (lo, hi) = refined.bounds();
        let (was_lo, was_hi) = cube.bounds();
        assert!(
            lo.abs_diff_eq(was_lo, 1e-6) && hi.abs_diff_eq(was_hi, 1e-6),
            "refinement changed the silhouette: {lo:?}..{hi:?} against {was_lo:?}..{was_hi:?}"
        );
    }

    #[test]
    fn refining_part_of_a_mesh_leaves_no_hanging_vertices() {
        // A vertex sitting in the middle of a neighbour's edge is a crack: the
        // two sides interpolate differently and daylight shows between them the
        // moment anything shades or deforms the surface. The neighbour has to
        // take the new midpoint into its own corner list instead.
        let cube = cube();
        let mut selected = vec![false; cube.face_count()];
        selected[0] = true;
        let refined = cube.refine(&selected);

        assert!(
            refined.is_closed_manifold(),
            "a partly refined mesh is not watertight: {:?}",
            refined.manifold_report()
        );
        // The four faces around the refined one each took one midpoint, so each
        // is now a pentagon; the face opposite is untouched.
        let pentagons = refined.faces.iter().filter(|face| face.len() == 5).count();
        assert_eq!(pentagons, 4, "the neighbours did not absorb the midpoints");
        assert_eq!(
            refined.faces.iter().filter(|face| face.len() == 4).count(),
            4 + 1,
            "expected the four new quads and the one untouched face"
        );
    }

    #[test]
    fn refining_nothing_changes_nothing() {
        let cube = cube();
        let refined = cube.refine(&vec![false; cube.face_count()]);
        assert_eq!(refined.positions, cube.positions);
        assert_eq!(refined.faces, cube.faces);
    }

    #[test]
    fn a_convex_solid_has_no_creases() {
        // Every vertex of a cube is a convex corner, and cavity shading must not
        // touch any of it. This is the shape of every attached part — a hand, a
        // nose, an ear is a convex solid — and reading them as creased darkened
        // all of them (#63).
        for value in cube().crease() {
            assert_eq!(value, 0.0, "a cube corner is convex, not a cavity");
        }
    }

    #[test]
    fn a_cavity_creases_and_the_same_curve_seen_from_outside_does_not() {
        // The one thing the measure has to get right: which side of the surface
        // the material is on. A ridge and a groove have identical geometry and
        // opposite normals, and only one of them is a place light cannot reach.
        let inside = peak_crease(&arc(8, 0.035, true));
        assert!(
            inside > 0.2,
            "the inside of a 35 mm curve should read as a cavity: {inside:.3}"
        );
        assert_eq!(
            peak_crease(&arc(8, 0.035, false)),
            0.0,
            "the outside of the same curve is a ridge, not a cavity"
        );
    }

    #[test]
    fn crease_reads_curvature_rather_than_how_finely_a_shape_is_cut() {
        // The body is subdivided and the attached parts are not. A measure that
        // scaled with edge length would read the same physical fold differently
        // on either side of a wrist and put a shading seam exactly there.
        let coarse = peak_crease(&arc(4, 0.035, true));
        let fine = peak_crease(&arc(32, 0.035, true));
        assert!(
            (coarse - fine).abs() < 0.01,
            "an eightfold change in tessellation moved the same curve from \
             {coarse:.3} to {fine:.3}"
        );

        // And it does track the curvature itself: halving the radius doubles it.
        let tight = peak_crease(&arc(8, 0.020, true));
        let broad = peak_crease(&arc(8, 0.040, true));
        assert!(
            (tight / broad - 2.0).abs() < 0.1,
            "halving the radius should double the crease: {tight:.3} against {broad:.3}"
        );
    }
}
