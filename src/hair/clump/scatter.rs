//! Where the clumps of one region are rooted.
//!
//! **Scattered over the built surface's own faces, not over a profile of it.**
//! Every other way of placing hair on a head approximates the surface: a node
//! radius floats hair a head-width off the scalp, a radius-against-height
//! profile lets the brow ridge and the occiput poke through, and a profile per
//! sector is better and still a table. The faces of
//! the mesh ARE the surface, so a point placed inside one is on the head by
//! construction and needs no clearance pass at all.
//!
//! It also makes the density right for free. A root every so many square
//! millimetres of real surface is what "how thick is this hair" means; scattering
//! per vertex instead would follow `refine_face`'s schedule, putting a thousand
//! roots on a chin and a dozen on a crown.

use glam::Vec3;
use rand::Rng;
use rand_pcg::Pcg64Mcg;

use super::super::follicle::{Follicle, Follicles};
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::Zone;
use crate::rig::Rig;
use crate::rig::skin::SkinWeights;

/// One clump's footing on the head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Root {
    /// Where it sits, in head-local metres — on the surface, not above it.
    pub at: Vec3,
    /// Which way the surface faces there, head-local and unit length.
    ///
    /// Interpolated from the corners' own shading normals rather than taken
    /// from the face, because a vault carries 24 mm faces and a per-face normal
    /// would send every clump in one of them the same way — which reads as
    /// tufts on facets.
    pub out: Vec3,
    /// What the region's mask said where it landed, `0` to `1`.
    ///
    /// Carried rather than re-queried because a style wants it: a clump rooted
    /// in the soft edge of a hairline should be shorter and thinner than one
    /// rooted in the middle of the scalp, and that is what makes an edge read
    /// as hair thinning out rather than as hair stopping.
    pub weight: f32,
    /// Which joints hold the skin this root grew out of, and how strongly.
    ///
    /// **Hair is bound like the skin it grows from, not like the head it is
    /// near.** Binding the whole crop rigidly to the head joint is right for a
    /// scalp and wrong for a face: measured with the jaw
    /// twenty-five degrees open, the chin's own skin moves 44.7 mm and hair
    /// bound to the head moves NOTHING — so a beard stays where the closed
    /// mouth was while the chin it grows on drops away from underneath it.
    ///
    /// Copied whole from the corner of the face the seat landed nearest, rather
    /// than reduced to one joint: the jaw's hold on the skin fades across the
    /// jawline, and a beard whose flank hairs snapped from all-skull to all-jaw
    /// would tear along the same line the flank mask is drawn on.
    pub skin: VertexSkin,
}

/// How thoroughly one region's surface is walked when scattering.
///
/// Roots are placed at even intervals of weighted area with a jitter inside
/// each, which is a stratified sample: it cannot leave the bald gaps a plain
/// uniform draw does at these counts, and cannot comb into rows the way a grid
/// does. This is how much of an interval the jitter may move a root.
///
/// Provenance: **derived** — a full interval of jitter is a uniform draw again,
/// and none is a grid, so it is the largest value that is neither.
const JITTER: f32 = 0.8;

/// How many times a root may be redrawn inside its face before it is given up.
///
/// The draw is accepted with probability equal to the mask's weight where it
/// landed, so a face wholly inside the region takes the first try and one
/// straddling the edge takes about two. Four is generous for a rejection whose
/// worst case is a face that barely touches the region at all — and giving up
/// drops the root rather than seating it where no hair grows.
///
/// Provenance: **derived** from the acceptance rate the weights imply.
const RETRIES: usize = 4;

/// Scatters `count` roots over one region of a built head.
///
/// Density follows the mask: a face holds roots in proportion to its own area
/// times the weight there, so the soft edge of a region thins out rather than
/// stopping, and both layers thin together.
///
/// Deterministic in `stream`, which the caller draws from a record's seed, so
/// the same body grows the same hair every time it is built.
#[must_use]
pub fn scatter(
    mesh: &PolyMesh,
    rig: &Rig,
    weights: &SkinWeights,
    follicles: &Follicles,
    follicle: Follicle,
    count: usize,
    stream: &mut Pcg64Mcg,
) -> Vec<Root> {
    if count == 0 {
        return Vec::new();
    }
    let origin = follicles.origin();
    let normals = mesh.shading_normals();
    // One entry per face the region touches at all: its corners, its area, and
    // the mask's mean over its corners.
    let mut patches: Vec<(usize, f32)> = Vec::new();
    let mut total = 0.0;
    for face in 0..mesh.face_count() {
        let corners = &mesh.faces[face];
        if corners.len() < 3 {
            continue;
        }
        let centre = mesh.face_centroid(face);
        if rig.joints[rig.nearest_bone(centre).joint].zone != Zone::Head {
            continue;
        }
        // Averaged over the corners rather than read at the centroid: a 24 mm
        // face on the vault straddles a hairline, and a centroid reads that
        // face as wholly in or wholly out — which puts a straight edge through
        // the one boundary the whole region exists to make soft.
        let weight = corners
            .iter()
            .map(|corner| follicles.weight(follicle, mesh.positions[*corner as usize] - origin))
            .sum::<f32>()
            / corners.len() as f32;
        if weight <= f32::EPSILON {
            continue;
        }
        let claim = area_of(mesh, face) * weight;
        if claim <= f32::EPSILON {
            continue;
        }
        total += claim;
        patches.push((face, total));
    }
    if patches.is_empty() || total <= f32::EPSILON {
        return Vec::new();
    }

    let step = total / count as f32;
    let mut roots = Vec::with_capacity(count);
    for index in 0..count {
        let jitter = stream.random_range(-0.5..0.5) * JITTER;
        let wanted = step * (index as f32 + 0.5 + jitter);
        let wanted = wanted.clamp(0.0, total - f32::EPSILON);
        let slot = patches.partition_point(|(_, upto)| *upto <= wanted);
        let (face, _) = patches[slot.min(patches.len() - 1)];
        // **Rejection-sampled inside the face, because a face is not one
        // weight.** A 24 mm quad on the vault straddles the hairline, and its
        // corners average to something well over zero while a third of it is
        // outside the region entirely — so a root drawn uniformly in it lands
        // where the mask says no hair. Accepting a point with probability equal
        // to its own weight makes the density follow the mask WITHIN a face as
        // well as between faces, which is what makes a soft edge thin out
        // smoothly rather than in steps of one face.
        let mut seat = inside(mesh, &normals, weights, face, stream);
        let mut weight = follicles.weight(follicle, seat.0 - origin);
        for _ in 0..RETRIES {
            if weight > stream.random_range(0.0..1.0) {
                break;
            }
            seat = inside(mesh, &normals, weights, face, stream);
            weight = follicles.weight(follicle, seat.0 - origin);
        }
        if weight <= f32::EPSILON {
            continue;
        }
        roots.push(Root {
            at: seat.0 - origin,
            out: seat.1,
            weight,
            skin: seat.2,
        });
    }
    roots
}

/// A point inside one face, and the surface normal there.
///
/// Fanned from the first corner and drawn over the fan by area, so a face that
/// subdivision left as a long thin quad is sampled evenly rather than favouring
/// whichever triangle came first.
fn inside(
    mesh: &PolyMesh,
    normals: &[Vec3],
    weights: &SkinWeights,
    face: usize,
    stream: &mut Pcg64Mcg,
) -> (Vec3, Vec3, VertexSkin) {
    let corners = &mesh.faces[face];
    let fan = corners.len() - 2;
    let areas: Vec<f32> = (0..fan)
        .map(|step| triangle_area(mesh, corners, step))
        .collect();
    let total: f32 = areas.iter().sum();
    let mut want = stream.random_range(0.0..1.0) * total.max(f32::EPSILON);
    let mut pick = 0;
    for (step, area) in areas.iter().enumerate() {
        if want <= *area {
            pick = step;
            break;
        }
        want -= area;
        pick = step;
    }
    let (one, two, three) = (
        corners[0] as usize,
        corners[pick + 1] as usize,
        corners[pick + 2] as usize,
    );
    // Uniform barycentric coordinates over a triangle: the square root is what
    // stops the sample bunching toward the first corner.
    let a: f32 = stream.random_range(0.0..1.0);
    let b: f32 = stream.random_range(0.0..1.0);
    let a = a.sqrt();
    let (u, v, w) = (1.0 - a, a * (1.0 - b), a * b);
    let at = mesh.positions[one] * u + mesh.positions[two] * v + mesh.positions[three] * w;
    let out = (normals[one] * u + normals[two] * v + normals[three] * w).normalize_or(Vec3::Y);
    // **The nearest corner's binding, taken whole rather than blended** (#207).
    // Two influence lists cannot be averaged entry by entry — they are sorted by
    // strength and index different joints, so entry `k` of one is a different
    // bone from entry `k` of the other, and the mean of the two is a joint list
    // nobody wrote. A clump is a few millimetres across on a face whose own
    // faces are larger than that, so the corner it sits nearest is the honest
    // answer and the blend it would have wanted is already in that corner's own
    // four weights.
    let nearest = if u >= v && u >= w {
        one
    } else if v >= w {
        two
    } else {
        three
    };
    (at, out, weights.vertices[nearest])
}

/// One triangle of a face's fan, by area.
fn triangle_area(mesh: &PolyMesh, corners: &[u32], step: usize) -> f32 {
    let one = mesh.positions[corners[step + 1] as usize] - mesh.positions[corners[0] as usize];
    let two = mesh.positions[corners[step + 2] as usize] - mesh.positions[corners[0] as usize];
    one.cross(two).length() * 0.5
}

/// A whole face's area.
fn area_of(mesh: &PolyMesh, face: usize) -> f32 {
    let corners = &mesh.faces[face];
    (0..corners.len() - 2)
        .map(|step| triangle_area(mesh, corners, step))
        .sum()
}
