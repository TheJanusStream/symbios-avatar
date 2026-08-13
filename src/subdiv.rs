//! Catmull-Clark subdivision.
//!
//! The cage is deliberately coarse — eight-point limb rings and convex joint
//! patches — because subdivision is what turns it into a body. One level makes
//! any polygon mesh all-quad and, over an eight-point cage, already smooth
//! enough to ship ([`crate::BODY_SUBDIVISIONS`]); the edge loops still follow
//! the skeleton, which is what a deforming character needs.
//!
//! Interior vertices use the standard rules. Boundary and corner rules are
//! implemented too, so the subdivider is safe on open meshes even though
//! [`crate::cage::build_cage`] always produces closed ones.

use glam::Vec3;
use std::collections::HashMap;

use crate::mesh::PolyMesh;

/// An undirected edge and the faces meeting along it.
struct Edge {
    ends: [u32; 2],
    faces: Vec<u32>,
}

impl Edge {
    /// The end that is not `vertex`.
    fn other(&self, vertex: u32) -> u32 {
        if self.ends[0] == vertex {
            self.ends[1]
        } else {
            self.ends[0]
        }
    }

    /// Whether this edge lies on a boundary or is non-manifold.
    fn is_boundary(&self) -> bool {
        self.faces.len() != 2
    }
}

/// Subdivides `mesh` `levels` times.
///
/// The result is all-quad for `levels >= 1`, and winding is preserved.
#[must_use]
pub fn catmull_clark(mesh: &PolyMesh, levels: usize) -> PolyMesh {
    let mut current = mesh.clone();
    for _ in 0..levels {
        current = subdivide_once(&current);
    }
    current
}

/// One subdivision step.
fn subdivide_once(mesh: &PolyMesh) -> PolyMesh {
    let vertex_count = mesh.positions.len();
    let face_points: Vec<Vec3> = (0..mesh.faces.len())
        .map(|i| mesh.face_centroid(i))
        .collect();

    let (edges, edge_lookup) = collect_edges(mesh);

    let edge_points: Vec<Vec3> = edges
        .iter()
        .map(|edge| {
            let midpoint = (mesh.positions[edge.ends[0] as usize]
                + mesh.positions[edge.ends[1] as usize])
                * 0.5;
            if edge.is_boundary() {
                midpoint
            } else {
                let neighbours = (face_points[edge.faces[0] as usize]
                    + face_points[edge.faces[1] as usize])
                    * 0.5;
                (midpoint + neighbours) * 0.5
            }
        })
        .collect();

    let mut incident_faces: Vec<Vec<u32>> = vec![Vec::new(); vertex_count];
    for (index, face) in mesh.faces.iter().enumerate() {
        for &vertex in face {
            incident_faces[vertex as usize].push(index as u32);
        }
    }
    let mut incident_edges: Vec<Vec<usize>> = vec![Vec::new(); vertex_count];
    for (index, edge) in edges.iter().enumerate() {
        incident_edges[edge.ends[0] as usize].push(index);
        incident_edges[edge.ends[1] as usize].push(index);
    }

    let vertex_points: Vec<Vec3> = (0..vertex_count)
        .map(|vertex| {
            let position = mesh.positions[vertex];
            let boundary: Vec<usize> = incident_edges[vertex]
                .iter()
                .copied()
                .filter(|&edge| edges[edge].is_boundary())
                .collect();

            match boundary.len() {
                // Interior: (F + 2R + (n-3)P) / n.
                0 => {
                    let valence = incident_edges[vertex].len() as f32;
                    if valence < 1.0 {
                        return position;
                    }
                    let faces: Vec3 = incident_faces[vertex]
                        .iter()
                        .map(|&face| face_points[face as usize])
                        .sum::<Vec3>()
                        / incident_faces[vertex].len().max(1) as f32;
                    let rims: Vec3 = incident_edges[vertex]
                        .iter()
                        .map(|&edge| {
                            (mesh.positions[edges[edge].ends[0] as usize]
                                + mesh.positions[edges[edge].ends[1] as usize])
                                * 0.5
                        })
                        .sum::<Vec3>()
                        / valence;
                    (faces + rims * 2.0 + position * (valence - 3.0)) / valence
                }
                // Boundary curve: (P*6 + neighbours) / 8.
                2 => {
                    let first = edges[boundary[0]].other(vertex as u32);
                    let second = edges[boundary[1]].other(vertex as u32);
                    (position * 6.0
                        + mesh.positions[first as usize]
                        + mesh.positions[second as usize])
                        / 8.0
                }
                // Corner or non-manifold: pinned.
                _ => position,
            }
        })
        .collect();

    let mut out = PolyMesh::new();
    out.positions.extend(vertex_points);
    let edge_base = out.positions.len() as u32;
    out.positions.extend(edge_points);
    let face_base = out.positions.len() as u32;
    out.positions.extend(face_points);

    for (index, face) in mesh.faces.iter().enumerate() {
        let corners = face.len();
        for corner in 0..corners {
            let previous = face[(corner + corners - 1) % corners];
            let current = face[corner];
            let next = face[(corner + 1) % corners];
            let outgoing = edge_lookup[&key(current, next)] as u32;
            let incoming = edge_lookup[&key(previous, current)] as u32;
            out.push_face([
                current,
                edge_base + outgoing,
                face_base + index as u32,
                edge_base + incoming,
            ]);
        }
    }

    out
}

/// Canonical key for an undirected edge.
fn key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

/// Builds the edge table and its lookup index.
fn collect_edges(mesh: &PolyMesh) -> (Vec<Edge>, HashMap<(u32, u32), usize>) {
    let mut edges: Vec<Edge> = Vec::new();
    let mut lookup: HashMap<(u32, u32), usize> = HashMap::new();

    for (index, face) in mesh.faces.iter().enumerate() {
        for corner in 0..face.len() {
            let a = face[corner];
            let b = face[(corner + 1) % face.len()];
            let entry = key(a, b);
            let edge = *lookup.entry(entry).or_insert_with(|| {
                edges.push(Edge {
                    ends: [entry.0, entry.1],
                    faces: Vec::new(),
                });
                edges.len() - 1
            });
            edges[edge].faces.push(index as u32);
        }
    }

    (edges, lookup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube() -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for corner in [
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
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
    fn one_level_quadruples_the_faces_and_stays_closed() {
        let subdivided = catmull_clark(&cube(), 1);
        assert_eq!(subdivided.face_count(), 24);
        assert_eq!(subdivided.quad_fraction(), 1.0);
        assert!(
            subdivided.is_closed_manifold(),
            "{:?}",
            subdivided.manifold_report()
        );
    }

    #[test]
    fn a_triangle_mesh_becomes_all_quads() {
        let mut tetra = PolyMesh::new();
        for corner in [
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ] {
            tetra.push_vertex(corner);
        }
        for face in [[0u32, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]] {
            tetra.push_face(face);
        }
        assert_eq!(tetra.quad_fraction(), 0.0);

        let subdivided = catmull_clark(&tetra, 1);
        assert_eq!(subdivided.quad_fraction(), 1.0);
        assert_eq!(subdivided.face_count(), 12);
        assert!(subdivided.is_closed_manifold());
    }

    #[test]
    fn subdivision_contracts_toward_the_limit_surface() {
        let cube = cube();
        let (lo, hi) = cube.bounds();
        let subdivided = catmull_clark(&cube, 2);
        let (slo, shi) = subdivided.bounds();
        assert!(shi.x < hi.x && slo.x > lo.x, "the cage shrinks inward");
        assert!(shi.x > 0.5, "but stays close to the cage");
    }

    #[test]
    fn an_open_mesh_keeps_its_boundary() {
        let mut strip = PolyMesh::new();
        for corner in [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ] {
            strip.push_vertex(corner);
        }
        strip.push_face([0u32, 1, 2, 3]);

        let subdivided = catmull_clark(&strip, 1);
        assert_eq!(subdivided.face_count(), 4);
        // Corners are pinned, so the patch keeps its extent.
        let (lo, hi) = subdivided.bounds();
        assert_eq!(lo, Vec3::ZERO);
        assert_eq!(hi, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn levels_compose() {
        let once = catmull_clark(&cube(), 1);
        let twice = catmull_clark(&cube(), 2);
        assert_eq!(catmull_clark(&once, 1).face_count(), twice.face_count());
        assert_eq!(catmull_clark(&cube(), 0), cube());
    }
}
