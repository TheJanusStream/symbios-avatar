//! A beast's feet (#367): each shaped onto the paw or hoof its plan
//! declares.
//!
//! The feet's half of what `face::beast` does for the head. The extremity
//! zone is refined, and each foot's vertices are moved along rays from its
//! axis onto its `FootShape`, whose frame is read off the leg's own pad and
//! toe joints. **Faded twice**: along the surface from the leg's vertices,
//! which stay where the cage put them, and by height above the sole - the
//! pad joint's bone runs up the pastern to the hock or the carpus, and
//! everything on it is the foot's by the rig's reckoning, so without a
//! ceiling the whole pastern would be pressed down onto the paw.

use crate::face::beast::fade;
use crate::face::smooth;
use crate::mesh::PolyMesh;
use crate::plan::beast_foot::FootShape;
use crate::plan::{BeastFeet, Limb, Zone};
use crate::rig::Rig;

/// How many times a foot is refined before it is shaped: a toe is a third of
/// a pad across, and the cage hands a foot over as a few rings of sixteen.
/// Provenance: **tuned by render** (#367), against the budget: four feet.
const REFINEMENT: usize = 1;

/// How far up the leg, along the surface and in pad radii, the shaping fades
/// in from the leg's vertices.
/// Provenance: **tuned by render** (#367).
const FADE: f32 = 0.8;

/// The heights above the sole, in pad radii, over which a foot's shaping
/// fades out: all of it below the first, none above the second. Above a
/// hoof's coronet and a paw's top, the pastern is the leg's.
/// Provenance: **tuned by render** (#367).
const CEILING: (f32, f32) = (1.3, 1.8);

/// Refines a beast's feet and shapes each onto its declared paw or hoof.
#[must_use]
pub fn shape(mesh: &PolyMesh, rig: &Rig, feet: &BeastFeet) -> PolyMesh {
    let shapes = shapes(rig, feet);
    if shapes.is_empty() {
        return mesh.clone();
    }
    let mut shaped = refine(mesh, rig, &shapes);
    let owners = owners(&shaped, rig, &shapes);
    let owned: Vec<bool> = owners.iter().map(Option::is_some).collect();
    let pad = shapes
        .iter()
        .map(|(_, shape)| shape.pad)
        .fold(f32::INFINITY, f32::min);
    let shares = fade(&shaped, &owned, pad * FADE);
    for ((point, owner), share) in shaped.positions.iter_mut().zip(owners).zip(shares) {
        let Some(foot) = owner.map(|index| &shapes[index].1) else {
            continue;
        };
        let local = foot.local(*point);
        let (low, high) = CEILING;
        let under = 1.0 - smooth((local.y / foot.pad - low) / (high - low));
        let share = share * under;
        if share <= 0.0 {
            continue;
        }
        if let Some(surface) = foot.surface_through(local) {
            *point += (foot.body(surface) - *point) * share;
        }
    }
    shaped
}

/// Every foot the rig ends a leg in, with its shape: the first two joints of
/// each extremity zone are its pad and its toes.
pub(crate) fn shapes(rig: &Rig, feet: &BeastFeet) -> Vec<(Limb, FootShape)> {
    Limb::ALL
        .into_iter()
        .filter_map(|limb| {
            let nodes = rig.in_zone(Zone::Extremity(limb));
            let (&pad, &toe) = (nodes.first()?, nodes.get(1)?);
            let pad_joint = &rig.joints[pad];
            (pad_joint.radius > f32::EPSILON).then(|| {
                let shape = FootShape::of(
                    feet.kind,
                    pad_joint.position,
                    pad_joint.radius,
                    rig.joints[toe].position,
                );
                (limb, shape)
            })
        })
        .collect()
}

/// Which foot, as an index into `shapes`, each vertex belongs to: the one
/// whose extremity zone holds the nearest bone, as the rig reckons it.
fn owners(mesh: &PolyMesh, rig: &Rig, shapes: &[(Limb, FootShape)]) -> Vec<Option<usize>> {
    mesh.positions
        .iter()
        .map(
            |&point| match rig.joints[rig.nearest_bone(point).joint].zone {
                Zone::Extremity(limb) => shapes.iter().position(|(foot, _)| *foot == limb),
                _ => None,
            },
        )
        .collect()
}

/// Each foot below its shaping's ceiling, [`REFINEMENT`] times over, curved
/// as the head is. Not the whole extremity zone: the pad's bone runs up the
/// pastern, and refining all of it spent most of a beast's triangles on legs.
fn refine(mesh: &PolyMesh, rig: &Rig, shapes: &[(Limb, FootShape)]) -> PolyMesh {
    let mut refined = mesh.clone();
    for _ in 0..REFINEMENT {
        let owners = face_owners(&refined, rig, shapes);
        let selected: Vec<bool> = owners
            .iter()
            .enumerate()
            .map(|(face, owner)| {
                owner.is_some_and(|index| {
                    let foot = &shapes[index].1;
                    foot.local(refined.face_centroid(face)).y < CEILING.1 * foot.pad
                })
            })
            .collect();
        refined = refined.refine_curved(&selected);
    }
    refined
}

/// Which foot each face belongs to, by its centroid, as [`owners`] asks of a
/// vertex.
fn face_owners(mesh: &PolyMesh, rig: &Rig, shapes: &[(Limb, FootShape)]) -> Vec<Option<usize>> {
    (0..mesh.face_count())
        .map(
            |face| match rig.joints[rig.nearest_bone(mesh.face_centroid(face)).joint].zone {
                Zone::Extremity(limb) => shapes.iter().position(|(foot, _)| *foot == limb),
                _ => None,
            },
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, Composites, Feet, QuadrupedParams};
    use crate::subdiv::catmull_clark;

    fn shaped(feet: Feet) -> (PolyMesh, PolyMesh, Rig) {
        let params = QuadrupedParams {
            feet,
            ..QuadrupedParams::default()
        };
        let skeleton = params.skeleton(&Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        let declared = skeleton.beast_feet.expect("a quadruped declares its feet");
        (mesh.clone(), shape(&mesh, &rig, &declared), rig)
    }

    #[test]
    fn every_foot_shapes_closed_and_leaves_the_pastern_alone() {
        for feet in Feet::ALL {
            let (before, after, rig) = shaped(feet);
            assert!(after.is_closed_manifold(), "{feet:?}: a foot has a hole");
            let shapes = shapes(&rig, &BeastFeet { kind: feet });
            let refined = refine(&before, &rig, &shapes);
            assert_eq!(shapes.len(), 4, "four feet");
            for (index, (was, now)) in refined.positions.iter().zip(&after.positions).enumerate() {
                let tall = shapes
                    .iter()
                    .map(|(_, foot)| foot.local(*was).y / foot.pad)
                    .fold(f32::INFINITY, f32::min);
                if tall > CEILING.1 {
                    assert_eq!(was, now, "{feet:?}: vertex {index} above every sole moved");
                }
            }
        }
    }

    #[test]
    fn every_foot_still_stands_on_the_ground() {
        for feet in Feet::ALL {
            let (_, after, _) = shaped(feet);
            let lowest = after
                .positions
                .iter()
                .map(|point| point.y)
                .fold(f32::MAX, f32::min);
            assert!(
                lowest.abs() < 0.004,
                "{feet:?}: the lowest point is at {lowest} m"
            );
        }
    }
}
