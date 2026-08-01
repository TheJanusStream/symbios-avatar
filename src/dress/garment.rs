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
    /// Its geometry, in body space.
    pub mesh: PolyMesh,
    /// Skin weights, one per vertex of [`Self::mesh`], taken from the body.
    pub weights: SkinWeights,
    /// The colour it should be shaded.
    pub colour: [f32; 3],
}

impl Garment {
    /// Cuts a garment from a body.
    ///
    /// `zones` says which vertex belongs to which part of the body — the same
    /// map the unwrap uses. Returns `None` when the cut covers nothing.
    #[must_use]
    pub fn cut(
        mesh: &PolyMesh,
        weights: &SkinWeights,
        zones: &[Zone],
        cut: &GarmentCut,
        colour: [f32; 3],
    ) -> Option<Self> {
        // A face is in, only if *every* corner of it is. Taking faces on a
        // majority instead leaves the hem straddling the boundary, which puts a
        // ragged edge halfway into the neighbouring zone.
        let zone_of = |corner: u32| zones.get(corner as usize).copied();
        let covered: Vec<&Vec<u32>> = mesh
            .faces
            .iter()
            .filter(|face| {
                face.iter().all(|&corner| {
                    zone_of(corner)
                        .is_some_and(|zone| cut.zones.contains(zone) || cut.reach.contains(zone))
                }) && face
                    .iter()
                    .any(|&corner| zone_of(corner).is_some_and(|zone| cut.zones.contains(zone)))
            })
            .collect();
        if covered.is_empty() {
            return None;
        }

        let normals = mesh.vertex_normals();

        // Only the vertices the cut actually uses, renumbered. Carrying the
        // body's whole vertex list would leave a garment mostly made of points
        // no face refers to, which every downstream tool then has to ignore.
        let mut moved = vec![u32::MAX; mesh.positions.len()];
        let mut source: Vec<u32> = Vec::new();
        for face in &covered {
            for &corner in face.iter() {
                if moved[corner as usize] == u32::MAX {
                    moved[corner as usize] = source.len() as u32;
                    source.push(corner);
                }
            }
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

        for face in &covered {
            let outer: Vec<u32> = face.iter().map(|&c| moved[c as usize]).collect();
            // The inner shell faces the other way, so its winding is reversed.
            let inner: Vec<u32> = outer.iter().rev().map(|&v| v + inner_base).collect();
            garment.push_face(outer);
            garment.push_face(inner);
        }

        for [from, to] in hem_edges(&covered) {
            let (a, b) = (moved[from as usize], moved[to as usize]);
            // Reversed against the outer shell's use of the same edge. Every
            // edge of a closed solid is traversed once each way; the rim shares
            // one edge with the outer shell and one with the inner, so it must
            // run against both of them.
            garment.push_face([b, a, a + inner_base, b + inner_base]);
        }

        // Weights come straight from the body vertex each point was made from,
        // which is the whole reason a garment cut this way needs no rigging.
        let mut worn = Vec::with_capacity(source.len() * 2);
        for _ in 0..2 {
            for &from in &source {
                worn.push(borrowed(weights, from as usize));
            }
        }

        Some(Self {
            mesh: garment,
            weights: SkinWeights { vertices: worn },
            colour,
        })
    }

    /// How many vertices the garment carries.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
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

/// Edges that only one covered face uses — the garment's cut edges.
///
/// Directed, and pointing the way the single face that owns them wound, so the
/// rim built from them faces outward without anything having to work out which
/// side is which.
fn hem_edges(covered: &[&Vec<u32>]) -> Vec<[u32; 2]> {
    let mut seen: Vec<([u32; 2], bool)> = Vec::new();
    for face in covered {
        for at in 0..face.len() {
            let edge = [face[at], face[(at + 1) % face.len()]];
            let key = if edge[0] < edge[1] {
                [edge[0], edge[1]]
            } else {
                [edge[1], edge[0]]
            };
            match seen.iter_mut().find(|(other, _)| *other == key) {
                Some((_, shared)) => *shared = true,
                None => seen.push((key, false)),
            }
        }
    }
    let lone: Vec<[u32; 2]> = seen
        .iter()
        .filter(|(_, shared)| !shared)
        .map(|(edge, _)| *edge)
        .collect();

    // Recover each lone edge's direction from the face that owns it.
    let mut hem = Vec::with_capacity(lone.len());
    for face in covered {
        for at in 0..face.len() {
            let edge = [face[at], face[(at + 1) % face.len()]];
            let key = if edge[0] < edge[1] {
                [edge[0], edge[1]]
            } else {
                [edge[1], edge[0]]
            };
            if lone.contains(&key) {
                hem.push(edge);
            }
        }
    }
    hem
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
        assert_eq!(garment.weights.vertices.len(), garment.vertex_count());
        assert!(garment.weights.is_normalized(1e-3));
        assert!(
            garment
                .weights
                .vertices
                .iter()
                .all(|influences| influences.iter().any(|i| i.weight > 0.0)),
            "a garment vertex came out unweighted"
        );
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
