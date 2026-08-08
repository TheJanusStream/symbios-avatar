//! Tight clothing, cut from the body's own surface.
//!
//! A close-fitting garment is the body re-evaluated a few millimetres outside
//! itself. Building it that way rather than modelling it separately buys three
//! things at once, and each of them is a problem that clothing systems usually
//! have to solve the hard way:
//!
//! - **It cannot intersect the body**, because every point of it is a point of
//!   the body pushed outward along its own normal. No collision pass, no
//!   push-out solver, nothing to tune.
//! - **It deforms for free.** Each garment vertex comes from a body vertex, so
//!   it inherits that vertex's skin weights exactly. A shirt bends where the
//!   elbow bends because it is made of elbow.
//! - **It fits every body.** The parameters that made the body already made the
//!   garment; nothing has to be re-cut when a wearer is taller or broader.
//!
//! The result is a closed solid — an outer shell, an inner shell just inside the
//! skin, and a rim joining them at every hem. Cloth has two sides and a cut edge,
//! and a single offset surface has none of those: seen from underneath it
//! vanishes, and at the hem it reads as a sticker.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::{Zone, ZoneSet};
use crate::rig::{Influence, MAX_INFLUENCES, SkinWeights};

/// How a garment is cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GarmentCut {
    /// Which parts of the body it covers.
    pub zones: ZoneSet,
    /// Parts it may spill into, but not claim on their own.
    ///
    /// A face is taken when every corner is in `zones` or `reach` and at least
    /// one is in `zones`. Without this, two garments meeting at the waist leave
    /// a ring of bare skin between them: the faces that straddle the seam are
    /// wholly inside neither, so neither takes them. Giving the lower garment a
    /// reach into the upper one's zone hands that ring to exactly one of them.
    pub reach: ZoneSet,
    /// How far its outer face stands off the skin, in metres.
    pub thickness: f32,
    /// How far its inner face sits *inside* the skin, in metres.
    ///
    /// Not zero. Coincident surfaces flicker against each other wherever depth
    /// precision runs out, and the inner face of a garment is coincident with
    /// the body over its whole area — the worst possible case for it.
    pub bite: f32,
}

impl Default for GarmentCut {
    fn default() -> Self {
        Self {
            zones: ZoneSet::default(),
            reach: ZoneSet::default(),
            thickness: 0.008,
            bite: 0.0015,
        }
    }
}

/// One piece of close-fitting clothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Garment {
    /// Its geometry, in body space, carrying the body's skin weights and colour.
    pub mesh: PolyMesh,
    /// The body vertex each garment vertex was cut from.
    ///
    /// Twice as long as the body's contribution, because the outer and inner
    /// shells are cut from the same points. This is the one lookup that gives a
    /// garment texture coordinates: it was made of body, so it is charted where
    /// the body is charted, and nothing has to unwrap it separately.
    pub source: Vec<u32>,
    /// The colour it should be shaded.
    pub colour: [f32; 3],
}

/// Which faces `cut` claims outright, one flag per face of `mesh`.
///
/// A face is claimed only if *every* corner of it is in the cut's zones or
/// reach. Taking faces on a majority instead leaves the hem straddling the
/// boundary, which puts a ragged edge halfway into the neighbouring zone.
#[must_use]
pub fn claimed(mesh: &PolyMesh, zones: &[Zone], cut: &GarmentCut) -> Vec<bool> {
    let zone_of = |corner: u32| zones.get(corner as usize).copied();
    mesh.faces
        .iter()
        .map(|face| {
            face.iter().all(|&corner| {
                zone_of(corner)
                    .is_some_and(|zone| cut.zones.contains(zone) || cut.reach.contains(zone))
            }) && face
                .iter()
                .any(|&corner| zone_of(corner).is_some_and(|zone| cut.zones.contains(zone)))
        })
        .collect()
}

/// Fills the notches in a claim's hem, in place.
///
/// The zone boundary the claim follows is a nearest-bone classification, and
/// where two bones hold the surface almost equally it wanders vertex by vertex
/// across a ring — so the raw hem is a zigzag of one-face bites out of the
/// garment and one-face spurs of it into bare skin (#148, where a neck change
/// small enough to be invisible re-cut the collar into sawteeth). Rather than
/// tune geometry until the boundary happens to land between rings, the hem is
/// smoothed here, where it is made: any unclaimed face sharing two or more
/// edges with claimed faces is a notch, and is claimed too, to a fixed point.
/// A face past a straight hem touches it on one edge only, so the fill climbs
/// concavities and never advances a straight front.
///
/// Filling is the only direction that is safe: a garment face is body pushed
/// outward, so an extra face lies over skin, while *shaving* a spur would open
/// a hole in anything that counts on the raw claim. `blocked` marks faces some
/// other garment holds — a notch is never filled across a seam, or the two
/// garments would claim the same face and flicker (`the_top_and_the_trousers_
/// do_not_claim_the_same_face`).
pub fn close(mesh: &PolyMesh, mine: &mut [bool], blocked: &[bool]) {
    let mut users: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for at in 0..face.len() {
            let key = edge_key(face[at], face[(at + 1) % face.len()]);
            users.entry(key).or_default().push(index);
        }
    }
    loop {
        let mut filled = false;
        for index in 0..mesh.faces.len() {
            if mine[index] || blocked.get(index).copied().unwrap_or(false) {
                continue;
            }
            let face = &mesh.faces[index];
            let holding = (0..face.len())
                .filter(|&at| {
                    let key = edge_key(face[at], face[(at + 1) % face.len()]);
                    users[&key]
                        .iter()
                        .any(|&other| other != index && mine[other])
                })
                .count();
            if holding >= 2 {
                mine[index] = true;
                filled = true;
            }
        }
        if !filled {
            break;
        }
    }
}

impl Garment {
    /// Cuts a garment from a body.
    ///
    /// `zones` says which vertex belongs to which part of the body — the same
    /// map the unwrap uses. Returns `None` when the cut covers nothing.
    ///
    /// This is [`claimed`], [`close`], and [`sew`](Self::sew) in one call, for
    /// a garment worn alone. Garments that share a body go through the steps
    /// separately — see [`Outfit::wear`](crate::dress::Outfit::wear) — because
    /// each one's fill has to know what the others hold.
    #[must_use]
    pub fn cut(
        mesh: &PolyMesh,
        weights: &SkinWeights,
        zones: &[Zone],
        cut: &GarmentCut,
        colour: [f32; 3],
    ) -> Option<Self> {
        let mut mine = claimed(mesh, zones, cut);
        close(mesh, &mut mine, &[]);
        Self::sew(mesh, weights, &mine, cut, colour)
    }

    /// Builds the garment solid over the faces `mine` marks.
    ///
    /// Returns `None` when the claim covers nothing.
    #[must_use]
    pub fn sew(
        mesh: &PolyMesh,
        weights: &SkinWeights,
        mine: &[bool],
        cut: &GarmentCut,
        colour: [f32; 3],
    ) -> Option<Self> {
        let covered: Vec<&Vec<u32>> = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|&(index, _)| mine.get(index).copied().unwrap_or(false))
            .map(|(_, face)| face)
            .collect();
        if covered.is_empty() {
            return None;
        }

        let normals = mesh.vertex_normals();

        // Only the vertices the cut actually uses, renumbered. Carrying the
        // body's whole vertex list would leave a garment mostly made of points
        // no face refers to, which every downstream tool then has to ignore.
        //
        // **One garment vertex per FAN, not per body vertex, and that is what
        // makes a pinched hem a closed solid** (#105). Where the covered
        // region's boundary touches itself — the top of a pair of shorts, all
        // the way round the waist — a single body vertex carries two separate
        // runs of covered faces and four boundary edges. Sharing one column
        // between them puts four rim quads on the side edge that column and its
        // inner twin span, and four faces on an edge is not a solid. Splitting
        // the vertex once per fan gives each boundary loop its own column and
        // its own side edge, and the two loops stop being one figure-eight.
        //
        // Measured before the split: the shorts came back with 12 nonmanifold
        // edges on ten of twelve seeds, always a multiple of four because each
        // pinch vertex contributes exactly one. Identical across all three
        // sleeve lengths, which is what ruled the sleeve out.
        let fan = fans(&covered);
        let mut source: Vec<u32> = Vec::new();
        // `corner_at[face][position]` is the garment vertex that corner uses.
        let mut corner_at: Vec<Vec<u32>> = Vec::with_capacity(covered.len());
        let mut column: HashMap<usize, u32> = HashMap::new();
        for (index, face) in covered.iter().enumerate() {
            let mut row = Vec::with_capacity(face.len());
            for at in 0..face.len() {
                let root = fan.root(index, at);
                let next = *column.entry(root).or_insert_with(|| {
                    source.push(face[at]);
                    source.len() as u32 - 1
                });
                row.push(next);
            }
            corner_at.push(row);
        }

        let mut garment = PolyMesh::new();
        for &from in &source {
            let at = mesh.positions[from as usize];
            let out = normals[from as usize];
            garment.push_vertex(at + out * cut.thickness);
        }
        let inner_base = garment.vertex_count() as u32;
        for &from in &source {
            let at = mesh.positions[from as usize];
            let out = normals[from as usize];
            garment.push_vertex(at - out * cut.bite);
        }

        for row in &corner_at {
            let outer: Vec<u32> = row.clone();
            // The inner shell faces the other way, so its winding is reversed.
            let inner: Vec<u32> = outer.iter().rev().map(|&v| v + inner_base).collect();
            garment.push_face(outer);
            garment.push_face(inner);
        }

        for (face, at) in hem_edges(&covered) {
            let row = &corner_at[face];
            let (a, b) = (row[at], row[(at + 1) % row.len()]);
            // Reversed against the outer shell's use of the same edge. Every
            // edge of a closed solid is traversed once each way; the rim shares
            // one edge with the outer shell and one with the inner, so it must
            // run against both of them.
            garment.push_face([b, a, a + inner_base, b + inner_base]);
        }

        // Weights come straight from the body vertex each point was made from,
        // which is the whole reason a garment cut this way needs no rigging.
        // Both shells were cut from the same points, hence the doubling.
        let cut_from: Vec<u32> = source.iter().chain(&source).copied().collect();
        garment.set_skin(
            cut_from
                .iter()
                .map(|&from| borrowed(weights, from as usize))
                .collect(),
        );
        garment.paint(Vec3::from_array(colour));

        Some(Self {
            mesh: garment,
            source: cut_from,
            colour,
        })
    }

    /// How many vertices the garment carries.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
    }

    /// Charts the garment against the body's atlas.
    ///
    /// `body_uvs` is one coordinate per *body* vertex — the unwrap's first copy
    /// of each, which is enough because a garment is shaded rather than painted
    /// with detail that would show a seam.
    pub fn chart(&mut self, body_uvs: &[Vec2]) {
        self.mesh.set_uvs(
            self.source
                .iter()
                .map(|&from| body_uvs.get(from as usize).copied().unwrap_or(Vec2::ZERO))
                .collect(),
        );
    }
}

/// The influences of one body vertex.
fn borrowed(weights: &SkinWeights, vertex: usize) -> [Influence; MAX_INFLUENCES] {
    weights
        .vertices
        .get(vertex)
        .copied()
        .unwrap_or([Influence::default(); MAX_INFLUENCES])
}

/// The undirected key an edge is looked up by, endpoints in order.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// Which covered faces use each edge, as `(face, position)` within [`Garment::cut`]'s
/// `covered` list.
fn edge_faces(covered: &[&Vec<u32>]) -> HashMap<(u32, u32), Vec<(usize, usize)>> {
    let mut users: HashMap<(u32, u32), Vec<(usize, usize)>> = HashMap::new();
    for (index, face) in covered.iter().enumerate() {
        for at in 0..face.len() {
            let key = edge_key(face[at], face[(at + 1) % face.len()]);
            users.entry(key).or_default().push((index, at));
        }
    }
    users
}

/// Edges that only one covered face uses — the garment's cut edges.
///
/// Returned as `(face, position)` rather than as a pair of body vertices, and
/// that is not bookkeeping: at a vertex where the boundary touches itself the
/// garment carries more than one column for the same body vertex (see
/// [`fans`]), so a body vertex no longer names a garment one. The corner does.
///
/// Directed by construction — position `at` runs from `face[at]` to its
/// successor — so the rim built from them faces outward without anything having
/// to work out which side is which.
fn hem_edges(covered: &[&Vec<u32>]) -> Vec<(usize, usize)> {
    let mut hem: Vec<(usize, usize)> = edge_faces(covered)
        .into_values()
        .filter(|users| users.len() == 1)
        .map(|users| users[0])
        .collect();
    // A `HashMap` hands its values back in whatever order it likes, and a
    // garment's face list is compared between runs by `tests/record.rs`.
    hem.sort_unstable();
    hem
}

/// Runs of covered faces that meet edge to edge, one class per corner.
///
/// **A vertex is not a column** (#105). The covered region is a patch cut out of
/// the body, and its boundary is a set of loops only so long as it never touches
/// itself. It does touch itself — at the top of a pair of shorts, all the way
/// round the waist — and there a single body vertex carries two separate runs of
/// covered faces with four boundary edges between them. [`Garment::cut`] gives
/// each run its own garment vertex, so each boundary loop gets its own rim
/// column and its own side edge; sharing one put four rim quads on one edge.
///
/// Two corners at the same vertex are in the same run when a covered face
/// spans the edge between them, so this is a union-find over corners with one
/// union per interior edge. It says nothing about winding: the corners at each
/// end of a shared edge are matched by vertex, so a face wound the wrong way
/// round joins the same run it would have anyway.
struct Fans {
    /// Where each face's corners start in the flat corner numbering.
    offset: Vec<usize>,
    parent: Vec<usize>,
}

impl Fans {
    /// The run a corner belongs to, named by its representative corner.
    fn root(&self, face: usize, at: usize) -> usize {
        let mut node = self.offset[face] + at;
        while self.parent[node] != node {
            node = self.parent[node];
        }
        node
    }

    /// Joins the runs two corners belong to.
    fn join(&mut self, a: usize, b: usize) {
        let (mut a, mut b) = (a, b);
        while self.parent[a] != a {
            a = self.parent[a];
        }
        while self.parent[b] != b {
            b = self.parent[b];
        }
        if a != b {
            self.parent[a] = b;
        }
    }
}

/// Classifies every corner of `covered` into the run of faces it sits in.
fn fans(covered: &[&Vec<u32>]) -> Fans {
    let mut offset = Vec::with_capacity(covered.len());
    let mut total = 0;
    for face in covered {
        offset.push(total);
        total += face.len();
    }
    let mut built = Fans {
        offset,
        parent: (0..total).collect(),
    };

    for ((p, q), users) in edge_faces(covered) {
        // Exactly two, because an edge with more than that is a body already
        // non-manifold along it and no split here would mend the garment. One
        // is a boundary edge, which is what separates two runs rather than
        // joining them.
        let [(left, left_at), (right, right_at)] = users[..] else {
            continue;
        };
        // Matched by vertex rather than by winding: `at` and its successor are
        // the edge's two ends, and which of them is `p` depends on how the face
        // was wound.
        for end in [p, q] {
            let corner_of = |face: usize, at: usize| {
                let len = covered[face].len();
                let corner = if covered[face][at] == end {
                    at
                } else {
                    (at + 1) % len
                };
                built.offset[face] + corner
            };
            let (a, b) = (corner_of(left, left_at), corner_of(right, right_at));
            built.join(a, b);
        }
    }
    built
}

/// A colour, from a hue around the wheel and a lightness.
///
/// Dyed cloth, not paint: the ramp never reaches full saturation, because a
/// fully saturated garment reads as plastic next to skin that never is.
#[must_use]
pub fn dye(hue: f32, shade: f32) -> [f32; 3] {
    let turn = hue.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let level = 0.18 + 0.62 * shade.clamp(0.0, 1.0);
    let spread = 0.26 * (1.0 - (level - 0.45).abs());
    let wheel = |offset: f32| level + spread * (turn + offset).cos();
    [
        wheel(0.0).clamp(0.0, 1.0),
        wheel(2.094).clamp(0.0, 1.0),
        wheel(4.189).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Limb;
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};
    use glam::Vec3;

    fn body() -> (PolyMesh, SkinWeights, Vec<Zone>) {
        let record = AvatarRecord::new("Dressed", Archetype::default());
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        (mesh, weights, zones)
    }

    fn torso() -> ZoneSet {
        ZoneSet::default().with(Zone::Chest).with(Zone::Abdomen)
    }

    #[test]
    fn a_garment_can_be_cut_from_a_body() {
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment =
            Garment::cut(&mesh, &weights, &zones, &cut, [0.3, 0.3, 0.4]).expect("a torso exists");
        assert!(garment.vertex_count() > 40);
        assert!(garment.mesh.face_count() > 20);
    }

    #[test]
    fn a_cut_that_covers_nothing_makes_no_garment() {
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            // No humanoid has these.
            zones: ZoneSet::default().with(Zone::Extremity(Limb::ForeLeft)),
            ..Default::default()
        };
        // Extremities exist but are a single ring, so this may or may not cover
        // whole faces; what must not happen is a panic or an empty mesh.
        if let Some(garment) = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]) {
            assert!(garment.mesh.face_count() > 0);
        }

        let nothing = GarmentCut {
            zones: ZoneSet::default(),
            ..Default::default()
        };
        assert!(Garment::cut(&mesh, &weights, &zones, &nothing, [0.5; 3]).is_none());
    }

    #[test]
    fn a_garment_is_a_closed_solid() {
        // Cloth has two faces and a cut edge. A single offset surface vanishes
        // when seen from underneath and reads as a sticker at the hem.
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(
            garment.mesh.is_closed_manifold(),
            "{:?}",
            garment.mesh.manifold_report()
        );
    }

    #[test]
    fn a_garment_stands_off_the_skin_it_was_cut_from() {
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            thickness: 0.01,
            bite: 0.002,
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]).expect("a torso");

        // Every garment point is within a hair of the body, and the outer half
        // is outside it: it hugs, and it cannot be inside.
        let nearest = |point: Vec3| {
            mesh.positions
                .iter()
                .map(|body| body.distance(point))
                .fold(f32::MAX, f32::min)
        };
        for point in &garment.mesh.positions {
            assert!(
                nearest(*point) <= 0.011,
                "a garment point sat {} from the body",
                nearest(*point)
            );
        }
    }

    #[test]
    fn a_garment_wears_the_skin_weights_of_the_body_beneath_it() {
        // The whole reason for cutting one this way: it needs no rigging.
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(garment.mesh.channels_are_consistent());
        assert_eq!(garment.mesh.skin.len(), garment.vertex_count());
        let worn = SkinWeights {
            vertices: garment.mesh.skin.clone(),
        };
        assert!(worn.is_normalized(1e-3));
        assert!(
            garment
                .mesh
                .skin
                .iter()
                .all(|influences| influences.iter().any(|i| i.weight > 0.0)),
            "a garment vertex came out unweighted"
        );
        // Each garment vertex names the body vertex it was cut from, and the
        // two shells name the same ones.
        assert_eq!(garment.source.len(), garment.vertex_count());
        let half = garment.vertex_count() / 2;
        assert_eq!(garment.source[..half], garment.source[half..]);
    }

    #[test]
    fn a_garment_is_charted_where_the_body_is_charted() {
        let (mesh, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let mut garment = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(
            garment.mesh.uvs.is_empty(),
            "a cut garment is not yet charted"
        );

        // Stand-in for the body's atlas: one coordinate per body vertex.
        let body_uvs: Vec<Vec2> = (0..mesh.vertex_count())
            .map(|v| Vec2::new(v as f32 / mesh.vertex_count() as f32, 0.25))
            .collect();
        garment.chart(&body_uvs);

        assert!(garment.mesh.channels_are_consistent());
        for (vertex, &from) in garment.source.iter().enumerate() {
            assert_eq!(garment.mesh.uvs[vertex], body_uvs[from as usize]);
        }
    }

    #[test]
    fn a_wider_cut_covers_more() {
        let (mesh, weights, zones) = body();
        let narrow = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let wide = GarmentCut {
            zones: torso().with(Zone::Pelvis),
            ..Default::default()
        };
        let small = Garment::cut(&mesh, &weights, &zones, &narrow, [0.5; 3]).expect("a torso");
        let large = Garment::cut(&mesh, &weights, &zones, &wide, [0.5; 3]).expect("a torso");
        assert!(large.vertex_count() > small.vertex_count());
    }

    #[test]
    fn thickness_sets_how_far_a_garment_stands_out() {
        // Measured against the skin, not against the bounding box. A box grows
        // by whatever the normal at its widest vertex happens to be pointing at,
        // which is not the quantity being set.
        let (mesh, weights, zones) = body();
        let furthest = |thickness: f32| {
            let cut = GarmentCut {
                zones: torso(),
                thickness,
                bite: 0.001,
                ..Default::default()
            };
            let garment = Garment::cut(&mesh, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
            garment
                .mesh
                .positions
                .iter()
                .map(|point| {
                    mesh.positions
                        .iter()
                        .map(|body| body.distance(*point))
                        .fold(f32::MAX, f32::min)
                })
                .fold(0.0f32, f32::max)
        };
        for thickness in [0.004f32, 0.012, 0.02] {
            let stood = furthest(thickness);
            assert!(
                (stood - thickness).abs() < thickness * 0.25,
                "a garment {thickness} thick stood {stood} off the skin"
            );
        }
    }

    #[test]
    fn dye_stays_in_range_and_never_reaches_full_saturation() {
        for step in 0..12 {
            let hue = step as f32 / 12.0;
            for shade in [0.0, 0.35, 0.7, 1.0] {
                let colour = dye(hue, shade);
                assert!(
                    colour.iter().all(|c| (0.0..=1.0).contains(c)),
                    "dye({hue}, {shade}) gave {colour:?}"
                );
                let wide = colour.iter().fold(0.0f32, |a, b| a.max(*b))
                    - colour.iter().fold(f32::MAX, |a, b| a.min(*b));
                assert!(wide < 0.75, "dye({hue}, {shade}) was {wide} saturated");
            }
        }
    }

    #[test]
    fn dye_moves_with_both_of_its_axes() {
        assert_ne!(dye(0.0, 0.5), dye(0.4, 0.5));
        let dark = dye(0.3, 0.05);
        let light = dye(0.3, 0.95);
        let sum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(sum(light) > sum(dark) * 1.5);
    }
}
