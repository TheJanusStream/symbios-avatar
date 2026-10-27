//! A beast's head (#367): its surface shaped onto the head its plan declares.
//!
//! [`shape`] does for a beast what `skull::refine_face` and `skull::shape` do
//! for a person, by another route. A person's skull is a set of profiles
//! measured off life; a beast's is a [`BeastHead`], a smooth union of
//! analytic parts the plan derives from its axes. So the head zone is refined
//! first, giving the surface the vertices a muzzle and a stop need, and every
//! head vertex is then moved along the ray from the head's axis to where that
//! ray leaves the declared head - faded to nothing toward the neck, whose
//! vertices stay where the cage put them.
//!
//! Applied to the **rest** mesh before anything is bound or unwrapped, as the
//! human skull is, so skin weights and texture charts follow it without
//! knowing it happened.
//!
//! The parts that stand off the head are built against the same declaration
//! and kept together as a [`BeastFace`]: the [`ears`], the [`jaw`] with its
//! tongue, and the whiskers and the horn ([`adorn`]).

pub mod adorn;
pub mod ears;
pub mod jaw;

pub use adorn::Adornments;
pub use ears::{Ear, Ears};
pub use jaw::Jaw;

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::smooth;
use crate::mesh::PolyMesh;
use crate::plan::{BeastHead, Zone};
use crate::rig::Rig;

/// The parts a beast's head wears (#367): its ears, its jaw and its tongue,
/// each riding a joint of its own off the head.
#[derive(Clone, Debug, PartialEq)]
pub struct BeastFace {
    /// The ears.
    pub ears: Ears,
    /// The lower jaw and the tongue.
    pub jaw: Jaw,
    /// The whiskers and the horn, drawn with the eyes.
    pub adornments: Adornments,
}

impl BeastFace {
    /// Builds every part a beast's head declares, for the head joint `head`,
    /// against `body`, the surface as built.
    #[must_use]
    pub fn build(rig: &Rig, head: usize, beast: &BeastHead, body: &PolyMesh) -> Self {
        Self {
            ears: Ears::build(rig, head, beast),
            jaw: Jaw::build(rig, head, beast, body),
            adornments: Adornments::build(beast, rig.joints[head].position),
        }
    }

    /// Hangs every part on its joint. After the body is bound, as the eyes'
    /// lids are, so nothing of the body's surface is offered to them.
    pub fn rig(&mut self, rig: &mut Rig) {
        self.ears.rig(rig);
        self.jaw.rig(rig);
    }

    /// Every part with the joint it rides, in a fixed order - the ears, the
    /// jaw, the tongue - which is the contract between whatever charts them
    /// and whatever places the charts.
    pub fn parts(&self) -> impl Iterator<Item = (&PolyMesh, usize)> {
        self.ears.parts().chain(self.jaw.parts())
    }

    /// Every part's mesh, in [`Self::parts`]' order, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        self.ears.meshes_mut().chain(self.jaw.meshes_mut())
    }

    /// Every part's mouth channel, in [`Self::parts`]' order: none for the
    /// ears, the jaw's inside and the whole tongue (#368).
    pub fn mouths(&self) -> impl Iterator<Item = Vec<f32>> {
        self.ears
            .parts()
            .map(|_| Vec::new())
            .chain(self.jaw.mouths())
    }
}

/// How many times the head zone is refined before it is shaped.
///
/// The cage hands the head over as a sixteen-sided tube, which can carry a
/// braincase and a muzzle but not the stop between them, nor a lip's lobe a
/// fifth of a radius across. Two passes put sixty-four facets round it.
/// Provenance: **tuned by render** (#367), against the triangle budget: the
/// head is the beast's face, and the budget a person spends on a face is the
/// measure of what one is worth.
const REFINEMENT: usize = 2;

/// How far round the nose, in head radii, a further pass refines: a pad a
/// fifth of a radius across with nostrils cut into it wants the vertices a
/// head's broad surface does not.
/// Provenance: **tuned by render** (#367).
const NOSE_REACH: f32 = 0.5;

/// How far from the neck, along the surface and in head radii, the shaping
/// fades in.
///
/// **The neck's vertices do not move, so whatever the head's do at the
/// junction is a step.** Faded over this distance, the head leaves the cage's
/// surface where the neck meets it and arrives at its own six tenths of a
/// radius up.
/// Provenance: **tuned by render** (#367).
const FADE: f32 = 0.60;

/// Refines a beast's head and shapes it onto `head`.
#[must_use]
pub fn shape(mesh: &PolyMesh, rig: &Rig, head: &BeastHead) -> PolyMesh {
    let mut shaped = refine(mesh, rig, head);
    let owned = owned(&shaped, rig);
    let shares = fade(&shaped, &owned, head.radius * FADE);
    for (point, share) in shaped.positions.iter_mut().zip(shares) {
        if share <= 0.0 {
            continue;
        }
        if let Some(surface) = head.surface_through(head.local(*point)) {
            *point += (head.body(surface) - *point) * share;
        }
    }
    shaped
}

/// Whether each vertex is the head's, asked of the rig as `skull::shape`
/// asks it: the neck runs up into the same space, and a throat shaped as a
/// jaw pinches away from the head it is meant to meet.
fn owned(mesh: &PolyMesh, rig: &Rig) -> Vec<bool> {
    mesh.positions
        .iter()
        .map(|&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
        .collect()
}

/// The head zone, [`REFINEMENT`] times over, and the nose's own region once
/// more. Curved, because the tube's facets are chords of a round surface
/// and a flat split would keep them.
fn refine(mesh: &PolyMesh, rig: &Rig, head: &BeastHead) -> PolyMesh {
    let headed = |mesh: &PolyMesh, face: usize| {
        let centre = mesh.face_centroid(face);
        rig.joints[rig.nearest_bone(centre).joint].zone == Zone::Head
    };
    let mut refined = mesh.clone();
    for _ in 0..REFINEMENT {
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| headed(&refined, face))
            .collect();
        refined = refined.refine_curved(&selected);
    }
    let nose = head.nose_centre();
    let selected: Vec<bool> = (0..refined.face_count())
        .map(|face| {
            headed(&refined, face)
                && head.local(refined.face_centroid(face)).distance(nose) < NOSE_REACH * head.radius
        })
        .collect();
    refined.refine_curved(&selected)
}

/// Each vertex's share of the shaping: none off the head, rising smoothly to
/// all of it `reach` along the surface from the nearest vertex that is not
/// the head's.
///
/// Measured along the mesh's edges rather than straight through the body, so
/// a vertex on the far side of the jaw from the throat is not faded for being
/// near it through the head.
pub(crate) fn fade(mesh: &PolyMesh, owned: &[bool], reach: f32) -> Vec<f32> {
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); mesh.vertex_count()];
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let (a, b) = (face[corner], face[(corner + 1) % face.len()]);
            neighbours[a as usize].push(b);
            neighbours[b as usize].push(a);
        }
    }
    // Dijkstra from every vertex that is not the head's at once. Distances
    // are never negative, so their bits sort as the distances do.
    let mut along = vec![f32::INFINITY; mesh.vertex_count()];
    let mut queue = BinaryHeap::new();
    for (vertex, _) in owned.iter().enumerate().filter(|(_, mine)| !**mine) {
        along[vertex] = 0.0;
        queue.push(Reverse((0.0f32.to_bits(), vertex as u32)));
    }
    while let Some(Reverse((bits, vertex))) = queue.pop() {
        let at = f32::from_bits(bits);
        if at > along[vertex as usize] || at >= reach {
            continue;
        }
        let from = mesh.positions[vertex as usize];
        for &next in &neighbours[vertex as usize] {
            let step = at + from.distance(mesh.positions[next as usize]);
            if step < along[next as usize] {
                along[next as usize] = step;
                queue.push(Reverse((step.to_bits(), next)));
            }
        }
    }
    along
        .iter()
        .map(|&distance| smooth(distance / reach))
        .collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, Composites, QuadrupedParams};
    use crate::skeleton::Skeleton;
    use crate::subdiv::catmull_clark;

    /// A quadruped's skeleton, its subdivided cage before any shaping, and
    /// its rig.
    fn beast(params: &QuadrupedParams) -> (Skeleton, PolyMesh, Rig) {
        let skeleton = params.skeleton(&Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        (skeleton, mesh, rig)
    }

    /// The head a quadruped with `params` declares.
    fn declared(params: &QuadrupedParams) -> BeastHead {
        params
            .skeleton(&Composites::default())
            .beast_head
            .expect("a quadruped declares its head")
    }

    /// How high the head's top line runs along its midline at `z`, in its
    /// own frame: marched down from above until the first point inside.
    fn top_at(head: &BeastHead, z: f32) -> f32 {
        let step = head.radius / 400.0;
        let mut y = 3.0 * head.radius;
        while head.distance(Vec3::new(0.0, y, z)) > 0.0 {
            y -= step;
            assert!(
                y > -3.0 * head.radius,
                "the midline at {z} never entered the head"
            );
        }
        y
    }

    /// How far the head reaches to its side at `z`, at the height `y`.
    fn side_at(head: &BeastHead, y: f32, z: f32) -> f32 {
        let step = head.radius / 400.0;
        let mut x = 3.0 * head.radius;
        while head.distance(Vec3::new(x, y, z)) > 0.0 {
            x -= step;
            assert!(x > 0.0, "the line at {y}, {z} never entered the head");
        }
        x
    }

    #[test]
    fn the_shaped_head_is_the_head_the_plan_declared() {
        let params = QuadrupedParams::default();
        let (skeleton, mesh, rig) = beast(&params);
        let head = skeleton.beast_head.expect("a quadruped declares its head");
        let shaped = shape(&mesh, &rig, &head);
        // Each vertex's share as the pass measured it: on the refined mesh,
        // before anything moved and changed the lengths it walks.
        let refined = refine(&mesh, &rig, &head);
        let shares = fade(&refined, &owned(&refined, &rig), head.radius * FADE);
        let mut outright = 0;
        for (point, share) in shaped.positions.iter().zip(&shares) {
            if *share < 1.0 {
                continue;
            }
            let off = head.distance(head.local(*point));
            assert!(
                off.abs() < 1.0e-3 * head.radius,
                "{point} sits {off} m off the declared head"
            );
            outright += 1;
        }
        assert!(
            outright > 1000,
            "only {outright} vertices were the head's outright"
        );
        assert!(shaped.is_closed_manifold(), "the shaped body has a hole");
    }

    #[test]
    fn the_neck_is_left_where_the_cage_put_it() {
        let params = QuadrupedParams::default();
        let (skeleton, mesh, rig) = beast(&params);
        let head = skeleton.beast_head.expect("a quadruped declares its head");
        let refined = refine(&mesh, &rig, &head);
        let shaped = shape(&mesh, &rig, &head);
        assert_eq!(refined.positions.len(), shaped.positions.len());
        let mut kept = 0;
        for ((before, after), mine) in refined
            .positions
            .iter()
            .zip(&shaped.positions)
            .zip(owned(&refined, &rig))
        {
            if !mine {
                assert_eq!(before, after, "a vertex off the head moved");
                kept += 1;
            }
        }
        assert!(kept > 1000, "only {kept} vertices were off the head");
    }

    #[test]
    fn every_corner_of_the_head_shapes_closed() {
        // The head's own axes, each at both ends of its envelope at once:
        // what saturates in the derive has to hold the surface together.
        let ends = [-3.0, 3.0];
        for muzzle_length in ends {
            for muzzle_breadth in ends {
                for stop in ends {
                    for appeal in ends {
                        let params = QuadrupedParams {
                            muzzle_length,
                            muzzle_breadth,
                            stop,
                            appeal,
                            ..QuadrupedParams::default()
                        };
                        let what = format!(
                            "muzzle {muzzle_length}, breadth {muzzle_breadth}, stop {stop}, appeal {appeal}"
                        );
                        let (skeleton, mesh, rig) = beast(&params);
                        let head = skeleton.beast_head.expect("a quadruped declares its head");
                        let shaped = shape(&mesh, &rig, &head);
                        assert!(shaped.is_closed_manifold(), "{what}: the head has a hole");
                        // No ray ran off the head: every vertex stays within
                        // the reach a march starts from.
                        for point in &shaped.positions {
                            assert!(point.is_finite(), "{what}: a vertex left the world");
                        }
                        let far = shaped
                            .positions
                            .iter()
                            .zip(&mesh.positions)
                            .map(|(after, before)| after.distance(*before))
                            .fold(0.0f32, f32::max);
                        assert!(
                            far < 1.5 * head.radius,
                            "{what}: a vertex moved {far} m on a {} m head",
                            head.radius
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_deep_stop_sets_the_bridge_under_the_brow() {
        // The fall from the forehead to the bridge, along the midline.
        let fall = |stop: f32| {
            let head = declared(&QuadrupedParams {
                stop,
                ..QuadrupedParams::default()
            });
            let r = head.radius;
            (top_at(&head, 0.2 * r) - top_at(&head, 0.9 * r)) / r
        };
        let (roman, deep) = (fall(-1.0), fall(1.0));
        assert!(
            deep > roman + 0.15,
            "a deep stop falls {deep:.2} radii to the bridge, a Roman nose {roman:.2}"
        );
    }

    #[test]
    fn a_broad_muzzle_is_broad_at_the_nose() {
        let across = |muzzle_breadth: f32| {
            let head = declared(&QuadrupedParams {
                muzzle_breadth,
                ..QuadrupedParams::default()
            });
            let [_, nose] = head.axis;
            side_at(&head, nose.y, nose.z) / head.radius
        };
        let (wedge, boxy) = (across(-1.0), across(1.0));
        assert!(
            boxy > wedge * 1.3,
            "a broad muzzle is {boxy:.2} radii across the nose, a narrow one {wedge:.2}"
        );
    }

    #[test]
    fn a_beast_looks_out_of_its_head_where_its_eye_set_says() {
        use crate::{Archetype, Avatar, AvatarRecord};
        let mut outward = Vec::new();
        for eye_set in [-1.0, 0.0, 1.0] {
            let params = QuadrupedParams {
                eye_set,
                ..QuadrupedParams::default()
            };
            let record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
            let avatar = Avatar::build(&record).expect("the beast builds");
            let eyes = avatar.parts.eyes.as_ref().expect("a beast has eyes");
            let head = avatar.rig.joints[eyes.head].position;
            let body = &avatar.parts.body;
            for eye in [&eyes.left, &eyes.right] {
                let gaze = eye.rest * Vec3::Z;
                assert!(
                    body.contains(head + eye.pivot),
                    "eye set {eye_set}: a globe floats outside its head"
                );
                assert!(
                    !body.contains(head + eye.pivot + gaze * eye.radius),
                    "eye set {eye_set}: an eye looks out from under the skin"
                );
            }
            let (left, right) = (eyes.left.rest * Vec3::Z, eyes.right.rest * Vec3::Z);
            assert!(
                left.abs_diff_eq(Vec3::new(-right.x, right.y, right.z), 1.0e-4),
                "eye set {eye_set}: the right eye is not the left's reflection"
            );
            outward.push(left.x);
        }
        assert!(
            outward[0] < outward[1] && outward[1] < outward[2],
            "the eyes turn out {outward:?} as eyeSet runs frontal to lateral"
        );
    }

    #[test]
    fn a_beast_wears_its_ears_on_joints_of_their_own_bound_into_its_skin() {
        use crate::avatar::MeshKind;
        use crate::rig::Role;
        use crate::{Archetype, Avatar, AvatarRecord};
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        let avatar = Avatar::build(&record).expect("the beast builds");
        let ears = &avatar.parts.beast.as_ref().expect("a beast has ears").ears;
        let skin = &avatar
            .meshes
            .iter()
            .find(|mesh| mesh.kind == MeshKind::Skin)
            .expect("a skin")
            .mesh;
        for (ear, joint) in ears.parts() {
            let joint_of = &avatar.rig.joints[joint];
            assert_eq!(
                joint_of.role,
                Role::Helper,
                "an ear rides a joint of its own"
            );
            assert_eq!(joint_of.parent, Some(ears.head), "hung from the head");
            let held = skin
                .skin
                .iter()
                .filter(|vertex| vertex[0].joint as usize == joint && vertex[0].weight == 1.0)
                .count();
            assert!(
                held >= ear.vertex_count(),
                "the skin holds {held} vertices on the ear's joint, of its {}",
                ear.vertex_count()
            );
        }
        // And a person has none.
        let person = Avatar::build(&AvatarRecord::new("Person", Archetype::default()))
            .expect("the person builds");
        assert!(person.parts.beast.is_none());
    }

    #[test]
    fn a_nostril_is_a_hollow_on_either_nose() {
        use crate::plan::Nose;
        for nose in [Nose::Leather, Nose::Soft] {
            let head = declared(&QuadrupedParams {
                nose,
                ..QuadrupedParams::default()
            });
            let nostril = head.nose.nostril.centre;
            for side in [1.0, -1.0] {
                let at = Vec3::new(side * nostril.x, nostril.y, nostril.z);
                assert!(
                    head.distance(at) > 0.0,
                    "{nose:?}: the nostril at {at} is filled"
                );
            }
            assert_eq!(head.nose.leather.is_some(), nose == Nose::Leather);
        }
    }

    #[test]
    fn an_ear_s_spring_swings_the_ear_and_never_the_head() {
        use crate::anim::{Pose, SpringConfig, Springs};
        use crate::{Archetype, Avatar, AvatarRecord};
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        let avatar = Avatar::build(&record).expect("the beast builds");
        let rig = &avatar.rig;
        let ears = &avatar.parts.beast.as_ref().expect("a beast has ears").ears;
        let bases: Vec<usize> = [&ears.left, &ears.right]
            .into_iter()
            .map(|ear| ear.joint.expect("rigged"))
            .collect();
        let mut springs = Springs::of(rig);
        assert_eq!(
            springs.len(),
            2,
            "one chain an ear, and nothing else swings"
        );
        // Throw the head about, from the neck, and let the springs answer.
        let config = SpringConfig::default();
        let neck = rig.in_zone(Zone::Neck)[0];
        let mut pose = Pose::rest(rig);
        springs.advance(rig, &mut pose, None, 1.0 / 60.0, &config);
        pose.rotations[neck] = glam::Quat::from_rotation_y(1.0);
        springs.advance(rig, &mut pose, None, 1.0 / 60.0, &config);
        assert_eq!(
            pose.rotations[ears.head],
            glam::Quat::IDENTITY,
            "a spring turned the head"
        );
        assert!(
            bases
                .iter()
                .any(|&base| pose.rotations[base].angle_between(glam::Quat::IDENTITY) > 1.0e-3),
            "no ear lagged the head"
        );
    }

    #[test]
    fn an_eye_is_seated_on_the_braincase_with_room_for_its_pair() {
        // The envelope's corners that put an eye on a Roman nose or ran the
        // pair into each other (#367's critic): every eye stays behind the
        // braincase's front and clear of the midline by its globe.
        let ends = [-3.0, 3.0];
        for eye_set in ends {
            for stop in ends {
                for muzzle_length in ends {
                    for appeal in ends {
                        let head = declared(&QuadrupedParams {
                            eye_set,
                            stop,
                            muzzle_length,
                            appeal,
                            ..QuadrupedParams::default()
                        });
                        let what = format!(
                            "eye set {eye_set}, stop {stop}, muzzle {muzzle_length}, appeal {appeal}"
                        );
                        let eye = head.eye;
                        let brow = head.cranium.centre.z + head.cranium.radii.z;
                        assert!(eye.centre.z <= brow, "{what}: the eye sits on the muzzle");
                        assert!(
                            eye.centre.x >= eye.radius,
                            "{what}: the eyes run into each other"
                        );
                    }
                }
            }
        }
    }
}
