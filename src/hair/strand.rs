//! One group of hairs, swept as a ribbon.
//!
//! Nothing here models a hair. Real-time hair is built from *strand groups* —
//! flat cards, each standing for a few hundred hairs — because a head carries
//! something like a hundred thousand of them and no renderer is going to sweep
//! geometry down each one. VRoid's models are built this way, and so is every
//! stylised game character worth copying.
//!
//! The ribbon is a solid rather than a plane. A flat card needs a cut-out alpha
//! texture to stop reading as a strip of plastic, and alpha on hair is a famous
//! source of sorting artefacts; a thin swept solid has a silhouette of its own
//! and needs no transparency at all. It costs more triangles than a card, which
//! at this scale is a trade worth making.

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::prim;

/// How many faces go around a ribbon's cross-section.
const SIDES: usize = 6;

/// One swept lock of hair.
#[derive(Clone, Debug, PartialEq)]
pub struct Strand {
    /// The centre-line, in head-local space, running from root to tip.
    pub path: Vec<Vec3>,
    /// Half-width across the ribbon at the root, in metres.
    pub width: f32,
    /// Half-thickness of the ribbon at the root, in metres.
    pub thickness: f32,
    /// Which way the ribbon lies flat — around the head, not away from it.
    pub across: Vec3,
}

impl Strand {
    /// The root end of the centre-line.
    #[must_use]
    pub fn root(&self) -> Vec3 {
        self.path.first().copied().unwrap_or(Vec3::ZERO)
    }

    /// The tip end of the centre-line.
    #[must_use]
    pub fn tip(&self) -> Vec3 {
        self.path.last().copied().unwrap_or(Vec3::ZERO)
    }

    /// How far the centre-line runs, in metres.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.path
            .windows(2)
            .map(|step| step[0].distance(step[1]))
            .sum()
    }

    /// Sweeps the ribbon into geometry.
    ///
    /// The taper is deliberately late. A lock has to finish in a wisp — held
    /// near its root width it reads as a strap cut off square — but thinning it
    /// steadily from the root takes the covering with it, and the whole head
    /// turns into separate tendrils with scalp showing between them. So it keeps
    /// almost its full width for most of the fall and gives it up in the last
    /// quarter, which is what a clump of hair actually does.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let sections: Vec<Vec2> = (0..self.path.len())
            .map(|at| {
                let along = at as f32 / (self.path.len().max(2) - 1) as f32;
                let taper = 1.0 - 0.88 * along * along * along;
                Vec2::new(self.width * taper, self.thickness * taper.max(0.34))
            })
            .collect();
        prim::sweep(&self.path, &sections, SIDES, self.across)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight() -> Strand {
        Strand {
            path: (0..8)
                .map(|step| Vec3::new(0.0, -0.02 * step as f32, 0.05))
                .collect(),
            width: 0.01,
            thickness: 0.003,
            across: Vec3::X,
        }
    }

    #[test]
    fn a_strand_knows_its_ends() {
        let strand = straight();
        assert_eq!(strand.root(), Vec3::new(0.0, 0.0, 0.05));
        assert_eq!(strand.tip(), Vec3::new(0.0, -0.14, 0.05));
        assert!((strand.length() - 0.14).abs() < 1e-5);
    }

    #[test]
    fn a_swept_strand_is_a_closed_solid() {
        // Sweeping as a solid rather than a card is what lets hair render
        // without an alpha cut-out, so it had better actually be closed.
        let mesh = straight().mesh();
        assert!(mesh.is_closed_manifold(), "{:?}", mesh.manifold_report());
    }

    #[test]
    fn a_ribbon_is_wider_than_it_is_thick() {
        let strand = straight();
        let mesh = strand.mesh();
        let (lo, hi) = mesh.bounds();
        assert!(
            hi.x - lo.x > hi.z - lo.z,
            "the ribbon measured {} across and {} through",
            hi.x - lo.x,
            hi.z - lo.z
        );
    }

    #[test]
    fn a_degenerate_path_makes_no_geometry_rather_than_panicking() {
        let strand = Strand {
            path: vec![Vec3::ZERO],
            width: 0.01,
            thickness: 0.003,
            across: Vec3::X,
        };
        assert_eq!(strand.mesh().face_count(), 0);
    }
}
