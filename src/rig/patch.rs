//! Which triangles of a one-shell body are a given part of it.
//!
//! A body meshed from a capsule graph is a single closed surface, and that is
//! the whole point of it — the leg runs into the foot with no seam, as both
//! reference mannequins do. It also destroys every way of measuring a part that
//! this crate had. Both of the obvious ones fail on the same body:
//!
//! * **Connected components.** A foot that used to be an attached solid was its
//!   own component and could be pulled out by walking the mesh. Meshed into the
//!   leg it is not a component of anything.
//! * **A box, or a height crop.** A foot is at the bottom of a leg, so cropping
//!   below a height looks like it should work. It does not: the crop catches the
//!   shin, which reaches down past the ankle on the inside of it, and the first
//!   thing this cost was a sole reported 145 mm above the ground on a body whose
//!   sole was on it (#111).
//!
//! What survives is the binding. Every vertex says which joints hold it and how
//! strongly, so "the foot" can be asked as *the surface the foot's own joints
//! deform*, which is a question about the body rather than about where it
//! happens to be in space. That is also how the reference figures every foot
//! target is judged against were measured off the Quaternius GLBs — same
//! selector, same threshold — so the two sides of a comparison are made the same
//! way.
//!
//! # Which joints are a part's joints
//!
//! Not the ones its name suggests, and this is the trap. A joint deforms the
//! bones *leaving* it (see [`skin`](crate::rig::skin)), so a leaf joint holds no
//! body surface at all: our toe holds nothing, and a heel hung off the ankle as
//! a leaf would hold nothing either — its geometry binds to the **ankle**, the
//! joint whose bone runs out to it. The reference's foot is bound the same way
//! for the same reason: it has no heel bone, so its whole heel is held by the
//! ankle joint it calls `foot_l`. [`Rig::extremity_joints`] is the query that
//! gets this right; a set built from [`Zone::Extremity`] alone would drop the
//! heel on the floor and then report that the foot had none.
//!
//! [`Rig::extremity_joints`]: crate::rig::Rig::extremity_joints
//! [`Zone::Extremity`]: crate::plan::Zone::Extremity

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::rig::skin::SkinWeights;

/// How much of a vertex a set of joints must hold to own it.
///
/// A half, so ownership is decided by majority and no vertex belongs to two
/// parts at once. The figure is not a tuning knob — it is the one the reference
/// feet were measured with, and changing it here would silently stop our numbers
/// and theirs from being the same measurement.
pub const OWNED: f32 = 0.5;

/// The part of a body's surface a set of joints holds.
///
/// See the [module documentation](self) for why this exists and what it is not.
#[derive(Clone, Debug, PartialEq)]
pub struct Patch {
    /// One flag per vertex of the mesh this was measured on.
    owned: Vec<bool>,
    /// Indices of the faces every corner of which is owned.
    faces: Vec<usize>,
}

impl Patch {
    /// The surface `joints` hold, by majority of skinning weight.
    ///
    /// A face joins the patch only when **every** corner of it is owned, so the
    /// patch is a surface with an edge rather than a scattering of vertices —
    /// which is what anything measuring an area or casting a ray at it needs.
    #[must_use]
    pub fn held_by(mesh: &PolyMesh, weights: &SkinWeights, joints: &[usize]) -> Self {
        let owned: Vec<bool> = (0..mesh.vertex_count())
            .map(|vertex| {
                let held: f32 = weights
                    .vertices
                    .get(vertex)
                    .map(|influences| {
                        influences
                            .iter()
                            .filter(|influence| joints.contains(&(influence.joint as usize)))
                            .map(|influence| influence.weight)
                            .sum()
                    })
                    .unwrap_or(0.0);
                held > OWNED
            })
            .collect();

        let faces = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(_, face)| {
                !face.is_empty()
                    && face
                        .iter()
                        .all(|&corner| owned.get(corner as usize).copied().unwrap_or(false))
            })
            .map(|(index, _)| index)
            .collect();

        Self { owned, faces }
    }

    /// The vertices the patch owns.
    pub fn vertices(&self) -> impl Iterator<Item = usize> + '_ {
        self.owned
            .iter()
            .enumerate()
            .filter(|(_, owned)| **owned)
            .map(|(vertex, _)| vertex)
    }

    /// How many vertices the patch owns.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.owned.iter().filter(|owned| **owned).count()
    }

    /// Whether the patch owns `vertex`.
    #[must_use]
    pub fn owns(&self, vertex: usize) -> bool {
        self.owned.get(vertex).copied().unwrap_or(false)
    }

    /// Faces every corner of which the patch owns.
    #[must_use]
    pub fn faces(&self) -> &[usize] {
        &self.faces
    }

    /// Axis-aligned bounds of the owned vertices, or `(ZERO, ZERO)` if empty.
    #[must_use]
    pub fn bounds(&self, mesh: &PolyMesh) -> (Vec3, Vec3) {
        let mut vertices = self.vertices().map(|vertex| mesh.positions[vertex]);
        let Some(first) = vertices.next() else {
            return (Vec3::ZERO, Vec3::ZERO);
        };
        vertices.fold((first, first), |(lo, hi), at| (lo.min(at), hi.max(at)))
    }

    /// How many separate pieces the patch's faces form.
    ///
    /// One, for a part of a body. More than one means the selector reached
    /// somewhere it had no business being — the check that says a foot patch is
    /// a foot and not a foot plus a stray band of hip.
    #[must_use]
    pub fn components(&self, mesh: &PolyMesh) -> usize {
        let mut parent: Vec<usize> = (0..mesh.vertex_count()).collect();
        fn find(parent: &mut [usize], mut node: usize) -> usize {
            while parent[node] != node {
                parent[node] = parent[parent[node]];
                node = parent[node];
            }
            node
        }
        for &face in &self.faces {
            let corners = &mesh.faces[face];
            for pair in corners.windows(2) {
                let (a, b) = (
                    find(&mut parent, pair[0] as usize),
                    find(&mut parent, pair[1] as usize),
                );
                if a != b {
                    parent[a] = b;
                }
            }
        }
        let mut roots: Vec<usize> = self
            .faces
            .iter()
            .map(|&face| find(&mut parent, mesh.faces[face][0] as usize))
            .collect();
        roots.sort_unstable();
        roots.dedup();
        roots.len()
    }

    /// Samples the patch from below on a regular grid.
    ///
    /// See [`Footprint`] for what the result is and why a sole is measured this
    /// way rather than by binning vertices.
    #[must_use]
    pub fn footprint(&self, mesh: &PolyMesh, cells: usize) -> Footprint {
        let (lo, hi) = self.bounds(mesh);
        let cells = cells.max(1);
        let origin = Vec2::new(lo.x, lo.z);
        let step = Vec2::new(hi.x - lo.x, hi.z - lo.z) / cells as f32;

        let mut lowest = vec![f32::NAN; cells * cells];
        for row in 0..cells {
            for column in 0..cells {
                let at = origin + step * Vec2::new(row as f32 + 0.5, column as f32 + 0.5);
                let mut best = f32::INFINITY;
                for &face in &self.faces {
                    let corners = &mesh.faces[face];
                    let anchor = mesh.positions[corners[0] as usize];
                    for corner in 1..corners.len().saturating_sub(1) {
                        let b = mesh.positions[corners[corner] as usize];
                        let c = mesh.positions[corners[corner + 1] as usize];
                        if let Some(y) = height_under(at, anchor, b, c) {
                            best = best.min(y);
                        }
                    }
                }
                if best.is_finite() {
                    lowest[row * cells + column] = best;
                }
            }
        }

        Footprint {
            origin,
            step,
            cells,
            lowest,
        }
    }
}

/// The underside of a [`Patch`], sampled by vertical rays on a grid.
///
/// **A sole is measured by ray cast and not by binning vertices**, and the
/// difference is not academic: the reference foot carries 95 vertices over its
/// whole surface, so a grid of vertex bins holds one point per cell and it is as
/// likely to be on the instep as on the sole. A ray answers about the surface at
/// the place it was asked, which is what a sole profile is a statement about.
///
/// Rays are cast straight down the world's Y, because that is the axis the thing
/// being measured is defined against — a sole is flat with respect to the ground
/// a body stands on, not with respect to its own axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    /// Grid corner in the ground plane, as `(x, z)`.
    pub origin: Vec2,
    /// Cell size in the ground plane, as `(x, z)`.
    pub step: Vec2,
    /// Cells along each axis; the grid is square in cell count, not in metres.
    pub cells: usize,
    /// Lowest surface height per cell, row-major in `x` then `z`, `NaN` on a
    /// miss.
    pub lowest: Vec<f32>,
}

impl Footprint {
    /// Ground-plane centre of a cell, as `(x, z)`.
    #[must_use]
    pub fn at(&self, row: usize, column: usize) -> Vec2 {
        self.origin + self.step * Vec2::new(row as f32 + 0.5, column as f32 + 0.5)
    }

    /// Lowest surface height in a cell, or `None` where no ray hit anything.
    #[must_use]
    pub fn height(&self, row: usize, column: usize) -> Option<f32> {
        self.lowest
            .get(row * self.cells + column)
            .copied()
            .filter(|height| height.is_finite())
    }

    /// Ground plane area one cell stands for, in square metres.
    #[must_use]
    pub fn cell_area(&self) -> f32 {
        (self.step.x * self.step.y).abs()
    }

    /// Every cell a ray hit, as `(x, z, height)`.
    pub fn hits(&self) -> impl Iterator<Item = (Vec2, f32)> + '_ {
        (0..self.cells).flat_map(move |row| {
            (0..self.cells).filter_map(move |column| {
                self.height(row, column)
                    .map(|height| (self.at(row, column), height))
            })
        })
    }

    /// The lowest point of the whole underside, or `None` if nothing was hit.
    #[must_use]
    pub fn ground(&self) -> Option<f32> {
        self.hits()
            .map(|(_, height)| height)
            .fold(None, |best: Option<f32>, height| {
                Some(best.map_or(height, |a| a.min(height)))
            })
    }

    /// Cells whose surface sits within `band` metres of the lowest point.
    ///
    /// What a body actually rests on. The whole shadow of a foot includes its
    /// instep, which overhangs the ground and touches nothing.
    pub fn contact(&self, band: f32) -> impl Iterator<Item = (Vec2, f32)> + '_ {
        let floor = self.ground().unwrap_or(f32::INFINITY);
        self.hits()
            .filter(move |(_, height)| *height - floor <= band)
    }
}

/// Height of the triangle `a, b, c` directly above or below `at`, if it covers
/// it.
///
/// Barycentric in the ground plane: a vertical ray meets the triangle exactly
/// where its shadow covers the point, so the three-dimensional intersection
/// never has to be formed. A triangle standing on edge has no shadow area and is
/// rejected by the determinant, which is correct — it is a silhouette, not a
/// surface anything rests on.
fn height_under(at: Vec2, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let ground = |p: Vec3| Vec2::new(p.x, p.z);
    let (pa, pb, pc) = (ground(a), ground(b), ground(c));
    let area = (pb - pa).perp_dot(pc - pa);
    if area.abs() < 1e-12 {
        return None;
    }
    let u = (pb - at).perp_dot(pc - at) / area;
    let v = (pc - at).perp_dot(pa - at) / area;
    let w = 1.0 - u - v;
    (u >= -1e-6 && v >= -1e-6 && w >= -1e-6).then_some(a.y * u + b.y * v + c.y * w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Limb;
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    /// The default body, its rig, and its binding.
    fn body(seed: i64) -> (PolyMesh, Rig, SkinWeights) {
        let mut record = AvatarRecord::new("Patched", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        (mesh, rig, weights)
    }

    fn foot(mesh: &PolyMesh, rig: &Rig, weights: &SkinWeights, limb: Limb) -> Patch {
        Patch::held_by(mesh, weights, &rig.extremity_joints(limb))
    }

    #[test]
    fn a_foot_patch_is_one_piece_of_surface() {
        // The check that says the selector found a foot and not a foot plus a
        // stray band of somewhere else. A weight threshold can pick up an
        // island anywhere on the body and no proportion table would ever show
        // it.
        let (mesh, rig, weights) = body(1);
        let patch = foot(&mesh, &rig, &weights, Limb::HindLeft);
        assert!(
            patch.vertex_count() > 32,
            "{} vertices",
            patch.vertex_count()
        );
        assert_eq!(patch.components(&mesh), 1, "the foot came out in pieces");
    }

    #[test]
    fn the_foot_patch_holds_the_lowest_point_of_the_body() {
        // Independent of the selector: whatever a standing body's lowest vertex
        // is, it is on a sole. This is the check a height crop failed — it
        // caught the shin and reported a sole 145 mm above the ground, and no
        // amount of staring at the resulting table said so (#111).
        let (mesh, rig, weights) = body(1);
        let lowest = (0..mesh.vertex_count())
            .min_by(|&a, &b| mesh.positions[a].y.total_cmp(&mesh.positions[b].y))
            .expect("a body has vertices");
        let feet = [Limb::HindLeft, Limb::HindRight].map(|limb| foot(&mesh, &rig, &weights, limb));
        assert!(
            feet.iter().any(|patch| patch.owns(lowest)),
            "the body's lowest vertex at {:?} belongs to no foot",
            mesh.positions[lowest]
        );
    }

    #[test]
    fn a_foot_patch_stops_below_the_knee() {
        // The other half of the same claim. A selector that runs up the leg
        // would make every length and every fraction of one wrong, in the
        // direction that flatters the foot.
        let (mesh, rig, weights) = body(1);
        let patch = foot(&mesh, &rig, &weights, Limb::HindLeft);
        let [_, knee, _] = rig.limb_chain(Limb::HindLeft).expect("a leg bends");
        let (_, high) = patch.bounds(&mesh);
        assert!(
            high.y < rig.joints[knee].position.y,
            "the foot patch reached {:.3} m, past a knee at {:.3} m",
            high.y,
            rig.joints[knee].position.y
        );
    }

    #[test]
    fn the_two_feet_are_measured_the_same() {
        // The plan builds a body's two legs from one set of figures, so anything
        // that reads differently left and right is the instrument — up to the
        // body's own asymmetry, which is NOT zero and was measured rather than
        // assumed. The cage is mirror-symmetric to the last bit; the subdivided
        // surface is not, because a joint hull is triangulated without regard to
        // any plane of symmetry and Catmull-Clark weights a vertex by the faces
        // around it. Worst case on the whole body is 13.8 mm, and at the foot it
        // is 0.2 mm on a 208 mm foot (#112).
        //
        // A millimetre is therefore the tightest this can honestly be asked. It
        // is still far below anything the foot is judged on — the loosest
        // reference target here is a heel projection of 15.6% of foot length,
        // which is 32 mm.
        let (mesh, rig, weights) = body(5);
        let left = foot(&mesh, &rig, &weights, Limb::HindLeft);
        let right = foot(&mesh, &rig, &weights, Limb::HindRight);
        // Counts, not equal counts. The heel is a hulled joint, and a hull is
        // triangulated without regard to any plane of symmetry, so the two feet
        // are cut into slightly different numbers of faces — 553 vertices
        // against 545 on the default body. What has to match is what is
        // measured off them.
        let (fewer, more) = (
            left.vertex_count().min(right.vertex_count()) as f32,
            left.vertex_count().max(right.vertex_count()) as f32,
        );
        assert!(
            more / fewer < 1.05,
            "{fewer} vertices against {more}: the two feet are not the same part"
        );
        let (llo, lhi) = left.bounds(&mesh);
        let (rlo, rhi) = right.bounds(&mesh);
        assert!(
            (lhi.z - llo.z - (rhi.z - rlo.z)).abs() < 1e-3,
            "lengths differ: {} against {}",
            lhi.z - llo.z,
            rhi.z - rlo.z
        );
        assert!(
            (lhi.y - llo.y - (rhi.y - rlo.y)).abs() < 1e-3,
            "heights differ: {} against {}",
            lhi.y - llo.y,
            rhi.y - rlo.y
        );
    }

    #[test]
    fn the_underside_is_sampled_where_the_surface_is() {
        // A footprint is only worth anything if its cells are the sole. Every
        // cell that hit something must sit at or below the patch's own top, and
        // the lowest of them must be the patch's lowest point — if a ray missed
        // the sole and caught the instep instead, this is what says so.
        let (mesh, rig, weights) = body(1);
        let patch = foot(&mesh, &rig, &weights, Limb::HindLeft);
        let print = patch.footprint(&mesh, 24);
        let (lo, hi) = patch.bounds(&mesh);

        let ground = print.ground().expect("a foot has an underside");
        assert!(
            (ground - lo.y).abs() < 2e-3,
            "the lowest ray hit {ground:.4} against a patch bottom of {:.4}",
            lo.y
        );
        assert!(print.hits().all(|(_, height)| height <= hi.y + 1e-4));
        assert!(
            print.hits().count() * 3 > print.cells * print.cells,
            "only {} of {} cells hit the foot",
            print.hits().count(),
            print.cells * print.cells
        );
    }

    #[test]
    fn a_footprint_of_a_flat_slab_is_flat() {
        // The measure, against a shape whose answer is known before it is run.
        let mut mesh = PolyMesh::new();
        for corner in [
            Vec3::new(-0.05, 0.0, -0.1),
            Vec3::new(0.05, 0.0, -0.1),
            Vec3::new(0.05, 0.0, 0.1),
            Vec3::new(-0.05, 0.0, 0.1),
        ] {
            mesh.push_vertex(corner);
        }
        mesh.push_face([0u32, 1, 2, 3]);
        let patch = Patch {
            owned: vec![true; 4],
            faces: vec![0],
        };
        let print = patch.footprint(&mesh, 8);
        assert_eq!(print.hits().count(), 64, "a slab covers its own footprint");
        assert!(print.contact(1e-4).all(|(_, height)| height.abs() < 1e-6));
        assert!((print.cell_area() * 64.0 - 0.1 * 0.2).abs() < 1e-6);
    }
}
