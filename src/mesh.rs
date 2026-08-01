//! Polygon mesh container shared by the cage builder and the subdivider.
//!
//! Faces are arbitrary convex polygons wound counter-clockwise when viewed from
//! outside the surface, so a swept quad tube and a triangulated hull patch can
//! live in one mesh. [`PolyMesh::manifold_report`] is the workhorse diagnostic:
//! a correct body cage is a closed, consistently wound 2-manifold, and every
//! stage of the pipeline is checked against that.

use glam::Vec3;
use std::collections::HashMap;
use std::fmt::Write as _;

/// A polygon soup with shared vertices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PolyMesh {
    /// Vertex positions, indexed by the entries of [`PolyMesh::faces`].
    pub positions: Vec<Vec3>,
    /// Face loops, each a counter-clockwise ring of indices into `positions`.
    pub faces: Vec<Vec<u32>>,
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
    pub fn push_vertex(&mut self, position: Vec3) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(position);
        index
    }

    /// Appends a face loop.
    pub fn push_face(&mut self, face: impl Into<Vec<u32>>) {
        self.faces.push(face.into());
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
    /// and read the edge flow directly.
    #[must_use]
    pub fn to_obj(&self) -> String {
        let mut out = String::new();
        out.push_str("# symbios-avatar cage dump\n");
        for p in &self.positions {
            let _ = writeln!(out, "v {} {} {}", p.x, p.y, p.z);
        }
        for face in &self.faces {
            out.push('f');
            for &i in face {
                let _ = write!(out, " {}", i + 1);
            }
            out.push('\n');
        }
        out
    }
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
}
