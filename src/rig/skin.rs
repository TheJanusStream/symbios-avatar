//! Attaching a mesh to a rig.
//!
//! Because the same code generates both the skeleton and the surface, skin
//! weights can be derived analytically rather than solved for: every vertex is
//! near a known bone, and how near is exactly the influence that bone should
//! have. A general auto-rigger has to infer the skeleton from the mesh first,
//! which is the hard part — and it is a part we simply do not have.
//!
//! The falloff has bounded support, so a vertex is only influenced by bones that
//! can plausibly reach it. Raw distance weights are then **smoothed across the
//! surface**, which is what keeps a broad region like a torso from creasing
//! where two bones' influence meets — the classic failure of purely
//! distance-based weighting.

use glam::Vec3;
use std::collections::BTreeSet;

use super::Rig;
use crate::mesh::PolyMesh;
use crate::plan::Zone;

/// How many bones may influence one vertex.
///
/// Four is what glTF, and every engine that reads it, expects per vertex.
pub const MAX_INFLUENCES: usize = 4;

/// One bone's hold on one vertex.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Influence {
    /// Index of the influencing joint in [`Rig::joints`].
    pub joint: u16,
    /// How strongly it pulls, in `0..=1`.
    pub weight: f32,
}

impl Default for Influence {
    fn default() -> Self {
        Self {
            joint: 0,
            weight: 0.0,
        }
    }
}

/// Tuning for [`bind`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinConfig {
    /// How far a bone reaches, in multiples of its own radius.
    ///
    /// Larger values blend more bones together and soften joints; smaller ones
    /// keep influence local and crease more readily.
    pub reach: f32,
    /// Exponent on the falloff. Higher concentrates weight near the bone.
    pub falloff: f32,
    /// Surface smoothing passes applied to the raw distance weights.
    pub smoothing_iterations: usize,
    /// How far each pass moves a vertex toward its neighbours' average.
    pub smoothing_strength: f32,
}

impl Default for SkinConfig {
    fn default() -> Self {
        Self {
            reach: 2.6,
            falloff: 2.0,
            smoothing_iterations: 3,
            smoothing_strength: 0.5,
        }
    }
}

/// Per-vertex bone influences.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinWeights {
    /// Influences per vertex, normalised and sorted strongest first.
    pub vertices: Vec<[Influence; MAX_INFLUENCES]>,
}

impl SkinWeights {
    /// The joint holding `vertex` most strongly.
    #[must_use]
    pub fn dominant(&self, vertex: usize) -> u16 {
        self.vertices[vertex][0].joint
    }

    /// Which body zone each vertex belongs to.
    ///
    /// This is what lets a garment suppress the skin beneath it: the body knows,
    /// per vertex, which zone it is in, so covered geometry is never emitted in
    /// the first place.
    ///
    /// A vertex takes the zone of the **nearer end** of the bone holding it, not
    /// the bone's own joint. A bone spans two nodes that may sit in different
    /// zones — the thigh bone runs from pelvis to knee — and the joint owning it
    /// is the far one. Reading the joint alone would label the whole crotch as
    /// thigh, and leave the root's zone with no surface at all, since every
    /// child's bone starts exactly where the root does.
    #[must_use]
    pub fn zone_map(&self, mesh: &PolyMesh, rig: &Rig) -> Vec<Zone> {
        (0..self.vertices.len())
            .map(|vertex| {
                let joint = self.dominant(vertex) as usize;
                let here = rig.joints[joint];
                let (start, end) = rig.bone(joint);
                let (_, along) = distance_to_segment(mesh.positions[vertex], start, end);
                match here.parent {
                    Some(parent) if along < 0.5 => rig.joints[parent].zone,
                    _ => here.zone,
                }
            })
            .collect()
    }

    /// Whether every vertex's weights sum to one, within `tolerance`.
    #[must_use]
    pub fn is_normalized(&self, tolerance: f32) -> bool {
        self.vertices.iter().all(|influences| {
            let total: f32 = influences.iter().map(|i| i.weight).sum();
            (total - 1.0).abs() <= tolerance
        })
    }
}

/// Computes skin weights binding `mesh` to `rig`.
///
/// Every vertex ends up with at most [`MAX_INFLUENCES`] joints whose weights sum
/// to one, sorted strongest first.
#[must_use]
pub fn bind(mesh: &PolyMesh, rig: &Rig, config: &SkinConfig) -> SkinWeights {
    let vertices = mesh.positions.len();
    let joints = rig.len();
    if vertices == 0 || joints == 0 {
        return SkinWeights::default();
    }

    let mut dense = vec![0.0f32; vertices * joints];
    for (vertex, &position) in mesh.positions.iter().enumerate() {
        let row = &mut dense[vertex * joints..(vertex + 1) * joints];
        let mut nearest = (f32::INFINITY, 0usize);

        for (joint, weight) in row.iter_mut().enumerate() {
            let (start, end) = rig.bone(joint);
            let (start_radius, end_radius) = rig.bone_radii(joint);
            let (distance, along) = distance_to_segment(position, start, end);
            let radius = start_radius + (end_radius - start_radius) * along;

            if distance < nearest.0 {
                nearest = (distance, joint);
            }
            let span = radius * config.reach;
            if span > 0.0 {
                *weight = (1.0 - distance / span).max(0.0).powf(config.falloff);
            }
        }

        // A vertex beyond every bone's reach still has to belong somewhere.
        if row.iter().all(|w| *w <= 0.0) {
            row[nearest.1] = 1.0;
        }
        normalize(row);
    }

    smooth(&mut dense, mesh, joints, config);

    SkinWeights {
        vertices: (0..vertices)
            .map(|vertex| strongest(&dense[vertex * joints..(vertex + 1) * joints]))
            .collect(),
    }
}

/// Distance from `point` to the segment `start..end`, and how far along it fell.
fn distance_to_segment(point: Vec3, start: Vec3, end: Vec3) -> (f32, f32) {
    let axis = end - start;
    let length_squared = axis.length_squared();
    if length_squared <= f32::EPSILON {
        return (point.distance(start), 0.0);
    }
    let along = ((point - start).dot(axis) / length_squared).clamp(0.0, 1.0);
    (point.distance(start + axis * along), along)
}

/// Scales a row so its weights sum to one.
fn normalize(row: &mut [f32]) {
    let total: f32 = row.iter().sum();
    if total > 0.0 {
        for weight in row.iter_mut() {
            *weight /= total;
        }
    }
}

/// Blends each vertex's weights toward its neighbours'.
///
/// Distance alone gives a sharp boundary wherever two bones' influence meets,
/// which shows up as a crease across a broad region under animation. Averaging
/// over the surface turns that boundary into a gradient.
fn smooth(dense: &mut [f32], mesh: &PolyMesh, joints: usize, config: &SkinConfig) {
    if config.smoothing_iterations == 0 || config.smoothing_strength <= 0.0 {
        return;
    }
    let neighbors = adjacency(mesh);

    for _ in 0..config.smoothing_iterations {
        let previous = dense.to_vec();
        for (vertex, near) in neighbors.iter().enumerate() {
            if near.is_empty() {
                continue;
            }
            let row = &mut dense[vertex * joints..(vertex + 1) * joints];
            for (joint, weight) in row.iter_mut().enumerate() {
                let average: f32 = near
                    .iter()
                    .map(|&other| previous[other * joints + joint])
                    .sum::<f32>()
                    / near.len() as f32;
                *weight += (average - *weight) * config.smoothing_strength;
            }
            normalize(row);
        }
    }
}

/// Vertices sharing an edge with each vertex.
fn adjacency(mesh: &PolyMesh) -> Vec<Vec<usize>> {
    let mut sets: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); mesh.positions.len()];
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let a = face[corner] as usize;
            let b = face[(corner + 1) % face.len()] as usize;
            if a < sets.len() && b < sets.len() {
                sets[a].insert(b);
                sets[b].insert(a);
            }
        }
    }
    sets.into_iter()
        .map(|set| set.into_iter().collect())
        .collect()
}

/// The strongest influences in a row, normalised and sorted.
fn strongest(row: &[f32]) -> [Influence; MAX_INFLUENCES] {
    let mut ranked: Vec<Influence> = row
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight > 0.0)
        .map(|(joint, &weight)| Influence {
            joint: joint as u16,
            weight,
        })
        .collect();
    ranked.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    ranked.truncate(MAX_INFLUENCES);

    let total: f32 = ranked.iter().map(|i| i.weight).sum();
    let mut out = [Influence::default(); MAX_INFLUENCES];
    if total <= 0.0 {
        out[0].weight = 1.0;
        return out;
    }
    for (slot, influence) in out.iter_mut().zip(ranked) {
        *slot = Influence {
            joint: influence.joint,
            weight: influence.weight / total,
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::subdiv::catmull_clark;

    fn humanoid() -> (PolyMesh, Rig) {
        let skeleton = HumanoidParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        (catmull_clark(&cage, 1), rig)
    }

    #[test]
    fn every_vertex_is_bound_and_normalized() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());

        assert_eq!(skin.vertices.len(), mesh.vertex_count());
        assert!(skin.is_normalized(1e-4), "weights must sum to one");
        for (vertex, influences) in skin.vertices.iter().enumerate() {
            assert!(
                influences[0].weight > 0.0,
                "vertex {vertex} has no dominant joint"
            );
            assert!(
                influences.iter().all(|i| (i.joint as usize) < rig.len()),
                "vertex {vertex} references a joint that does not exist"
            );
        }
    }

    #[test]
    fn influences_are_sorted_strongest_first() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());
        for influences in &skin.vertices {
            for pair in influences.windows(2) {
                assert!(pair[0].weight >= pair[1].weight);
            }
        }
    }

    #[test]
    fn vertices_bind_to_the_body_part_they_sit_on() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());
        let zones = skin.zone_map(&mesh, &rig);

        // The highest vertex is on the head; the lowest is on a foot.
        let highest = argmax(&mesh, |p| p.y);
        assert_eq!(zones[highest], Zone::Head);

        let lowest = argmax(&mesh, |p| -p.y);
        assert!(
            matches!(zones[lowest], Zone::Extremity(limb) if !limb.is_fore()),
            "lowest vertex should be a foot, got {:?}",
            zones[lowest]
        );

        // The most lateral vertices are hands, and they are on opposite sides.
        let left = argmax(&mesh, |p| -p.x);
        let right = argmax(&mesh, |p| p.x);
        let (Zone::Extremity(left_limb), Zone::Extremity(right_limb)) = (zones[left], zones[right])
        else {
            panic!(
                "outermost vertices should be hands: {:?}",
                (zones[left], zones[right])
            );
        };
        assert!(left_limb.is_fore() && right_limb.is_fore());
        assert_eq!(left_limb.mirrored(), right_limb);
    }

    #[test]
    fn smoothing_softens_the_boundary_between_bones() {
        let (mesh, rig) = humanoid();
        let sharp = bind(
            &mesh,
            &rig,
            &SkinConfig {
                smoothing_iterations: 0,
                ..Default::default()
            },
        );
        let soft = bind(&mesh, &rig, &SkinConfig::default());

        // A smoothed bind spreads each vertex over more bones.
        let blended =
            |skin: &SkinWeights| skin.vertices.iter().filter(|i| i[1].weight > 0.01).count();
        assert!(
            blended(&soft) > blended(&sharp),
            "smoothing should blend more vertices across bones"
        );
        assert!(soft.is_normalized(1e-4));
    }

    #[test]
    fn binding_is_deterministic() {
        let (mesh, rig) = humanoid();
        let config = SkinConfig::default();
        assert_eq!(bind(&mesh, &rig, &config), bind(&mesh, &rig, &config));
    }

    #[test]
    fn an_empty_mesh_binds_to_nothing() {
        let (_, rig) = humanoid();
        let skin = bind(&PolyMesh::new(), &rig, &SkinConfig::default());
        assert!(skin.vertices.is_empty());
    }

    /// Index of the vertex scoring highest under `score`.
    fn argmax(mesh: &PolyMesh, score: impl Fn(Vec3) -> f32) -> usize {
        mesh.positions
            .iter()
            .enumerate()
            .max_by(|a, b| score(*a.1).total_cmp(&score(*b.1)))
            .expect("mesh has vertices")
            .0
    }
}
