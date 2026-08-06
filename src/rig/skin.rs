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
//!
//! # Which joint owns a bone
//!
//! A bone spans two joints, and only one of them deforms it. [`Pose::forward`]
//! places a joint at `positions[parent] + parent_rotation * offset`, so the
//! rotation stored at a joint turns that joint's *children* about it: rotating
//! a hip swings the thigh, and rotating a knee does not touch the thigh at all.
//! The joint that deforms the bone `parent → child` is therefore the **parent**
//! — the proximal end.
//!
//! [`Rig::bone`] names its segments by the distal end, because that is the
//! joint the segment leads into and there is exactly one of them. Binding has to
//! undo that: the weight a vertex earns from lying near the segment `hip → knee`
//! belongs to the **hip**, not to the knee.
//!
//! **This was wrong until #97, and it is what made limbs bend like rope.**
//! Binding the thigh to the knee meant flexing a knee rotated the thigh about
//! the knee as well, so the whole leg curved instead of hinging. Measured by
//! `examples/bodyaudit` on the default body: turning only the knee moved the
//! mid-thigh 39.8 mm against the mid-shank's 73.1 mm, a 54% leak into the
//! segment that must not move; the elbow leaked 76%. Both are zero once the
//! bone is owned by the joint that actually turns it.
//!
//! A consequence worth stating, because it looks like a bug: **a leaf joint gets
//! no weight from the body's own surface.** A hand, a foot, a crown has no bone
//! leaving it, so nothing on the body is deformed by rotating one. That is
//! correct — those joints are position markers for the body, and they earn
//! their weight from the geometry *attached* at them, which is bound
//! separately.
//!
//! [`Pose::forward`]: crate::anim::Pose::forward

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
        // Swept against the reference mannequin's own binding rather than
        // picked (#97). The two figures a bind is judged on are how many bones
        // hold a vertex — the reference averages 1.88, with 85% of its vertices
        // on two or fewer — and how much of a bone moves when the joint *after*
        // it turns, which the reference makes zero by giving the grandparent
        // bone no weight at all.
        //
        // Down from `reach: 2.6, smoothing_iterations: 3`, which averaged 3.12
        // bones per vertex with only 31% on two or fewer: three bones shared
        // the middle of every bone, and the ankle held 0.18 of the mid-thigh.
        // Measured across the sweep, at one smoothing pass:
        //
        // ```text
        //   reach   bones/vertex   <=2 bones   knee leak   area kept
        //    2.6        3.12          31%         13%        100%
        //    2.0        2.71          49%          6%        100%
        //    1.6        2.44          58%          3%        101%
        //    1.3        2.13          68%          2%         99%
        //    1.1        1.92          80%          1%         98%
        //    0.9        1.67          92%          0%         96%
        // ```
        //
        // 1.1 is where the locality lands on the reference; 0.9 overshoots it
        // and starts spending surface area on the inside of a fold. The area
        // column is why this is a floor rather than a preference — it is the
        // crease the smoothing exists to prevent, and it only becomes visible
        // below about 1.3. That it stays as high as 98% at all is dual
        // quaternion skinning doing its job; under matrices this column would
        // decide the value on its own.
        //
        // `falloff` and `smoothing_strength` were swept too and neither moved
        // anything worth having: 1.5 through 3.0 span 1.94 to 1.85 bones per
        // vertex, buying a percent of locality for a percent of area. They stay
        // where they were.
        Self {
            reach: 1.1,
            falloff: 2.0,
            smoothing_iterations: 1,
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
    /// zones — the thigh bone runs from pelvis to knee — and reading one end
    /// alone would label the whole crotch as thigh, and leave the root's zone
    /// with no surface at all.
    ///
    /// The dominant joint is the bone's *proximal* end (see the module docs), so
    /// the bone in question is one of those **leaving** it. A joint may have
    /// several — a pelvis has three — which is why the nearest has to be
    /// searched for rather than looked up.
    #[must_use]
    pub fn zone_map(&self, mesh: &PolyMesh, rig: &Rig) -> Vec<Zone> {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); rig.len()];
        for joint in 0..rig.len() {
            if let Some(parent) = rig.joints[joint].parent {
                children[parent].push(joint);
            }
        }

        (0..self.vertices.len())
            .map(|vertex| {
                let joint = self.dominant(vertex) as usize;
                let here = rig.joints[joint];
                let position = mesh.positions[vertex];
                let nearest = children[joint]
                    .iter()
                    .map(|&child| {
                        let (distance, along) = distance_to_segment(
                            position,
                            here.position,
                            rig.joints[child].position,
                        );
                        (distance, along, child)
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                match nearest {
                    Some((_, along, child)) if along >= 0.5 => rig.joints[child].zone,
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
///
/// Only [`crate::rig::Role::Deform`] joints are bound against. A rig may carry
/// joints the body is not made of — a spring chain down a lock of hair, a face
/// rig, a socket a prop hangs from — and the body's surface has to ignore every
/// one of them. Binding against a bone with no capsule behind it does not fail
/// loudly; it quietly attaches a patch of skin to something that was never
/// meant to move it.
#[must_use]
pub fn bind(mesh: &PolyMesh, rig: &Rig, config: &SkinConfig) -> SkinWeights {
    let vertices = mesh.positions.len();
    let joints = rig.len();
    let Some(first_deforming) = rig.deforming().next() else {
        return SkinWeights::default();
    };
    if vertices == 0 || joints == 0 {
        return SkinWeights::default();
    }

    let mut dense = vec![0.0f32; vertices * joints];
    for (vertex, &position) in mesh.positions.iter().enumerate() {
        let row = &mut dense[vertex * joints..(vertex + 1) * joints];
        let mut nearest = (f32::INFINITY, first_deforming);
        // Which body part this vertex is ON, asked exactly the way
        // [`crate::face::skull::shape`] asks it. See [`owner_of`]: for one bone
        // in the rig the answer decides which end of it turns this vertex.
        let mine = rig.joints[rig.nearest_bone(position).joint].zone;

        // Iterated by *segment*, credited to the joint that deforms it. See the
        // module docs: `rig.bone(segment)` runs `parent → segment`, and it is
        // the parent's rotation that turns it.
        for segment in 0..joints {
            if !rig.joints[segment].role.deforms() {
                continue;
            }
            let (start, end) = rig.bone(segment);
            let (start_radius, end_radius) = rig.bone_radii(segment);
            let (distance, along) = distance_to_segment(position, start, end);
            let radius = start_radius + (end_radius - start_radius) * along;
            let owner = owner_of(rig, segment, mine, along, start.distance(end));
            if !rig.joints[owner].role.deforms() {
                continue;
            }

            if distance < nearest.0 {
                nearest = (distance, owner);
            }
            let span = radius * config.reach;
            if span > 0.0 {
                // Several bones may leave one joint — a pelvis has three — and
                // a joint's hold on a vertex is the strongest of them rather
                // than their sum, which would give a crotch vertex twice the
                // pull of a thigh one for no reason but the topology above it.
                let pull = (1.0 - distance / span).max(0.0).powf(config.falloff);
                row[owner] = row[owner].max(pull);
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

/// Which joint's rotation turns the bone leading into `segment`, for a vertex
/// lying on the body part `on`.
///
/// **Almost always the parent, and the exception is the head** (#123). The
/// module docs above give the rule and why it is the parent: rotating a hip
/// swings the thigh, so the bone `hip → knee` is the hip's. That is right for
/// every hinge in the body, and a head is not a hinge.
///
/// A head is a **rigid body that extends below its own joint**, and it is the
/// only part of this rig that does. The head node sits `HEAD_BELOW_JOINT`
/// radii above the neck node, so the whole lower face — the jaw, the chin, the
/// mouth, the base of the nose — hangs off the `neck → head` bone, below the
/// joint that ought to turn it. Credited to the parent, every one of those
/// vertices belongs to the NECK. Measured on the default body before this: the
/// mouth line and the chin were held 1.00 by the neck and moved **zero
/// millimetres** when the head turned thirty degrees, against the 57.4 and
/// 60.6 mm a rigid head moves them; the brow was 0.75/0.25 and lost 14.7 mm of
/// 58.7. The face stayed behind while the skull turned out from under it, which
/// is what the owner saw as the nose and chin smearing under look-at.
///
/// **It got worse twice without anything here changing.** The head used to
/// reach 0.69 radii below its joint; #78 took that to 1.19 and #107's cage flip
/// took the built floor to 1.07–1.16. Each of those moved more of the face onto
/// the neck's bone, and nothing measured a head turn, so nothing said so.
///
/// The test is the vertex's own body part rather than a distance, because a
/// distance cannot answer it: the chin's tip sits **1.34 head radii** from the
/// head joint — `shape` pushes it there — so no reach keyed to the node radius
/// covers the chin without also swallowing the throat. `on` comes from
/// [`Rig::nearest_bone`], which is the same question `skull::shape` and
/// `relief::carve` ask to decide what the head is. Two functions disagreeing
/// about which vertices are the head is the defect; agreeing is the fix.
///
/// The neck keeps the bone for vertices whose part is the neck, so the throat
/// still follows the neck and rotating the neck still deforms something —
/// giving the whole bone to the head would have left the neck joint owning
/// nothing at all.
fn owner_of(rig: &Rig, segment: usize, on: Zone, along: f32, length: f32) -> usize {
    let parent = rig.joints[segment].parent.unwrap_or(segment);
    if on != Zone::Head
        || rig.joints[segment].zone != Zone::Head
        || rig.joints[parent].zone == Zone::Head
    {
        return parent;
    }
    // **How far down the bone the head reaches is the head node's own radius,
    // and that is what keeps the throat the neck's.** The zone test alone hands
    // the whole bone over, because the head's SURFACE runs on past its jaw to
    // meet the neck — `shape` says so, and `SETTLE` exists because it does — so
    // a vertex at the head's floor answers `Zone::Head` while plainly belonging
    // to the throat. Measured on the default body with the zone test alone, the
    // throat came out held 0.79 by the head and 0.03 by the neck, which turns a
    // windpipe with a glance.
    //
    // The head node is a sphere of `radius` about the head joint, and the part
    // of the bone inside it is the part the head was swept from. On the default
    // body that is the top 84% of the bone: the chin's projection falls at 0.41
    // and the throat's at 0.10, so the split lands between them without a
    // constant. `along` is a projection onto the bone, which is vertical here —
    // that the chin also stands 1.34 radii FORWARD does not enter into it, and
    // is why a plain distance to the node cannot make this call.
    if length <= f32::EPSILON {
        return segment;
    }
    let covered = (rig.joints[segment].radius / length).min(1.0);
    if along >= 1.0 - covered {
        segment
    } else {
        parent
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
        (catmull_clark(&cage, crate::BODY_SUBDIVISIONS), rig)
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

    #[test]
    fn the_whole_face_turns_with_the_head() {
        // **The defect this test exists for is that a head turn left the face
        // behind** (#123). The head node sits a head-and-a-fifth of its own
        // radius above the neck node, so the jaw, the chin, the mouth and the
        // base of the nose all hang off the `neck → head` bone — below the joint
        // that has to turn them. Credited to the bone's parent, as every other
        // bone correctly is, all of it belonged to the NECK: measured on the
        // default body, the mouth line and the chin were held 1.00 by the neck
        // and moved ZERO millimetres when the head turned thirty degrees,
        // against the 57.4 and 60.6 mm a rigid head moves them. Under look-at
        // the skull turned out from under its own face.
        //
        // Nothing said so because nothing had ever posed a head. The gait tests
        // turn hips and knees, the reference comparison is a rest pose, and the
        // software renderer had no way to ask for a gaze until this issue — so
        // the only instrument that could see it was the Bevy viewer, and it took
        // the owner looking at one.
        //
        // A head is rigid, so the assertion is exact rather than a tolerance on
        // a blend: every head-owned vertex must land where rotating it about the
        // head joint would put it. Half a millimetre is the arithmetic's noise,
        // not a budget for lag.
        use crate::anim::Pose;
        use glam::Quat;

        let record = crate::AvatarRecord::new("Turned", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let head = *rig
            .in_zone(Zone::Head)
            .first()
            .expect("a humanoid has a head");
        let centre = rig.joints[head].position;
        // The FACE: forward of the head joint and at or above the chin's tip.
        // Both bounds are the blend's, and both were found by running without
        // them. Below the chin the surface is the underside of the jaw running
        // back into the throat, and behind the joint it is the nape — each is a
        // place the head's hold must hand over to the neck's, so each is a blend
        // by design and `the_throat_stays_with_the_neck` is what checks that
        // side. The first version bounded the face at one head radius below the
        // joint and failed on a vertex 98 mm down at the angle of the jaw; the
        // second bounded it at the chin alone and failed on one 52 mm behind the
        // joint at the nape. Both were the blend doing its job, and neither is
        // the face.
        let chin = crate::face::Skull::measure(&avatar.parts.body, rig)
            .expect("a humanoid has a skull")
            .chin();

        let turn = Quat::from_rotation_y(30f32.to_radians());
        let mut pose = Pose::rest(rig);
        pose.rotations[head] = turn;
        let matrices = pose.forward(rig).skinning_matrices(rig);

        let body = &avatar.parts.body;
        let mut worst = (0usize, 0.0f32, Vec3::ZERO);
        let mut checked = 0usize;
        for (vertex, &rest) in body.positions.iter().enumerate() {
            if rig.joints[rig.nearest_bone(rest).joint].zone != Zone::Head {
                continue;
            }
            if rest.y < centre.y + chin || rest.z <= centre.z {
                continue;
            }
            checked += 1;
            let mut skinned = Vec3::ZERO;
            for influence in avatar.parts.weights.vertices[vertex] {
                if influence.weight > 0.0 {
                    skinned += influence.weight
                        * matrices[influence.joint as usize].transform_point3(rest);
                }
            }
            let rigid = centre + turn * (rest - centre);
            let lag = (rigid - skinned).length();
            if lag > worst.1 {
                worst = (vertex, lag, rest - centre);
            }
        }
        // A filter that selects nothing passes every assertion after it, which
        // is the shape of half the instrument failures this crate has found.
        assert!(
            checked > 500,
            "only {checked} vertices were read as the face; the filter, not the \
             binding, is what this would be measuring"
        );
        assert!(
            worst.1 < 0.0005,
            "a head turn left a face vertex {:.1} mm behind, at {:+.0},{:+.0},{:+.0} mm \
             from the head joint, of {checked} read — the face is bound to something \
             that is not the head",
            worst.1 * 1000.0,
            worst.2.x * 1000.0,
            worst.2.y * 1000.0,
            worst.2.z * 1000.0,
        );
    }

    #[test]
    fn the_throat_stays_with_the_neck() {
        // The other side of the same boundary, and the reason `owner_of` is
        // bounded by the head node's own extent rather than by the zone alone.
        // The head's SURFACE runs past its jaw down to meet the neck — `shape`
        // says so and `SETTLE` exists because it does — so a vertex at the
        // head's floor answers `Zone::Head` while plainly being a throat. With
        // the zone test alone it came out held 0.79 by the head and 0.03 by the
        // neck, which turns a windpipe with a glance.
        let record = crate::AvatarRecord::new("Throated", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let neck = rig.joints[head].parent.expect("a head sits on a neck");
        let centre = rig.joints[head].position;

        // The forward-most vertex at the head's own floor: the throat.
        let floor = avatar
            .parts
            .body
            .positions
            .iter()
            .filter(|p| rig.joints[rig.nearest_bone(**p).joint].zone == Zone::Head)
            .fold(f32::MAX, |low, p| low.min(p.y));
        let throat = avatar
            .parts
            .body
            .positions
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                (p.y - floor).abs() < rig.joints[head].radius * 0.03
                    && (p.x - centre.x).abs() < rig.joints[head].radius * 0.05
            })
            .max_by(|a, b| a.1.z.total_cmp(&b.1.z))
            .map(|(vertex, _)| vertex)
            .expect("a body has a throat");

        let held = |joint: usize| -> f32 {
            avatar.parts.weights.vertices[throat]
                .iter()
                .find(|influence| influence.joint as usize == joint)
                .map_or(0.0, |influence| influence.weight)
        };
        assert!(
            held(neck) > held(head),
            "the throat is held {:.2} by the head and {:.2} by the neck",
            held(head),
            held(neck)
        );
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
