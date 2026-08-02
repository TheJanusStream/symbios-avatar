//! Posing a rig, and deforming a body with the result.
//!
//! A [`Pose`] holds one local rotation per joint, relative to the rest pose, plus
//! a root translation. Storing rotations rather than positions is what makes a
//! pose **proportion-independent**: the same shoulder rotation reads as the same
//! gesture on a short body and a tall one, because the bone lengths come from
//! the rig rather than from the pose. That property is the whole reason motion
//! can be authored once here and replayed on bodies that do not exist yet.
//!
//! Bones carry no rest orientation of their own — a joint's offset from its
//! parent is simply the difference of their rest positions — so an identity
//! rotation everywhere reproduces the rest pose exactly.

use glam::{Mat4, Quat, Vec3};

use crate::anim::dual::{self, DualQuat};
use crate::mesh::PolyMesh;
use crate::rig::{Rig, SkinWeights};

/// One local rotation per joint, plus where the root sits.
#[derive(Clone, Debug, PartialEq)]
pub struct Pose {
    /// Rotation of each joint relative to its rest orientation, in parent space.
    pub rotations: Vec<Quat>,
    /// Offset of the root from its rest position, in body space.
    pub translation: Vec3,
}

impl Pose {
    /// The rig's rest pose: every joint unrotated, root where it was built.
    #[must_use]
    pub fn rest(rig: &Rig) -> Self {
        Self {
            rotations: vec![Quat::IDENTITY; rig.len()],
            translation: Vec3::ZERO,
        }
    }

    /// How many joints this pose covers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rotations.len()
    }

    /// Whether the pose covers no joints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rotations.is_empty()
    }

    /// Whether this pose has one rotation per joint of `rig`.
    #[must_use]
    pub fn fits(&self, rig: &Rig) -> bool {
        self.rotations.len() == rig.len()
    }

    /// Resolves local rotations into world positions and orientations.
    ///
    /// Joints are ordered parent-before-child, so one forward pass suffices.
    #[must_use]
    pub fn forward(&self, rig: &Rig) -> Posed {
        let count = rig.len().min(self.rotations.len());
        let mut positions = Vec::with_capacity(count);
        let mut rotations = Vec::with_capacity(count);

        for index in 0..count {
            let joint = rig.joints[index];
            match joint.parent {
                Some(parent) => {
                    let parent_rotation: Quat = rotations[parent];
                    let rest_offset = joint.position - rig.joints[parent].position;
                    positions.push(positions[parent] + parent_rotation * rest_offset);
                    rotations.push(parent_rotation * self.rotations[index]);
                }
                None => {
                    positions.push(joint.position + self.translation);
                    rotations.push(self.rotations[index]);
                }
            }
        }

        Posed {
            positions,
            rotations,
        }
    }

    /// Interpolates between two poses.
    ///
    /// Rotations take the shortest arc; the root translation moves linearly.
    #[must_use]
    pub fn lerp(&self, other: &Pose, t: f32) -> Pose {
        let t = t.clamp(0.0, 1.0);
        Pose {
            rotations: self
                .rotations
                .iter()
                .zip(&other.rotations)
                .map(|(a, b)| a.slerp(*b, t))
                .collect(),
            translation: self.translation.lerp(other.translation, t),
        }
    }
}

/// A rig resolved into world space.
#[derive(Clone, Debug, PartialEq)]
pub struct Posed {
    /// World position of each joint.
    pub positions: Vec<Vec3>,
    /// World orientation of each joint.
    pub rotations: Vec<Quat>,
}

impl Posed {
    /// How many joints were resolved.
    #[must_use]
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    /// Whether nothing was resolved.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    /// Matrices taking rest-pose geometry into this pose.
    ///
    /// Each is `translate(world) · rotate(world) · translate(−rest)`, which is
    /// the inverse-bind-matrix product glTF expects, pre-multiplied.
    #[must_use]
    pub fn skinning_matrices(&self, rig: &Rig) -> Vec<Mat4> {
        (0..self.len())
            .map(|index| {
                Mat4::from_rotation_translation(self.rotations[index], self.positions[index])
                    * Mat4::from_translation(-rig.joints[index].position)
            })
            .collect()
    }

    /// The same transforms as [`Self::skinning_matrices`], as dual quaternions.
    ///
    /// Every skinning transform here is rigid — a rotation about the joint's
    /// rest position followed by a move to where it ended up — so nothing is
    /// lost in the conversion. A transform that scaled would be, silently, which
    /// is why nothing in this crate produces one.
    #[must_use]
    pub fn skinning_transforms(&self, rig: &Rig) -> Vec<DualQuat> {
        (0..self.len())
            .map(|index| {
                let rotation = self.rotations[index];
                let carried = self.positions[index] - rotation * rig.joints[index].position;
                DualQuat::from_rotation_translation(rotation, carried)
            })
            .collect()
    }

    /// Deforms rest-pose vertices by dual quaternion skinning.
    ///
    /// The reference implementation: a renderer does this on the GPU, but having
    /// it here means a pose can be measured — that a foot reached the ground, or
    /// that a hip did not pinch — without rendering anything.
    ///
    /// Dual quaternions rather than matrices, because averaging matrices does
    /// not average rotations: where two bones turn apart, the surface between
    /// them collapses toward the axis. See [`crate::anim::dual`]. An integration
    /// that skins on the GPU has to match this, which is why the method chosen
    /// here is one a shader can run.
    #[must_use]
    pub fn deform(&self, rig: &Rig, positions: &[Vec3], weights: &SkinWeights) -> Vec<Vec3> {
        let transforms = self.skinning_transforms(rig);
        positions
            .iter()
            .zip(&weights.vertices)
            .map(|(&position, influences)| {
                dual::blend(
                    influences
                        .iter()
                        .filter(|influence| influence.weight > 0.0)
                        .map(|influence| (transforms[influence.joint as usize], influence.weight)),
                )
                .transform_point(position)
            })
            .collect()
    }

    /// Deforms a whole skinned mesh, normals and all.
    ///
    /// The mesh's own [`PolyMesh::skin`] channel says which bones hold each
    /// vertex, so nothing has to be passed alongside it and nothing can be
    /// passed that does not match. Normals are carried by the *rotation* of the
    /// same blended transform, which is what a skinning shader does and what
    /// keeps a seam-split copy shading as one surface: derived afresh from the
    /// deformed geometry they would crease along every cut.
    ///
    /// Texture coordinates, influences and colours come through untouched. A
    /// mesh with no skin channel is returned as it was — it is not attached to
    /// anything, so nothing moves it.
    #[must_use]
    pub fn deform_mesh(&self, rig: &Rig, mesh: &PolyMesh) -> PolyMesh {
        if mesh.skin.is_empty() {
            return mesh.clone();
        }
        let transforms = self.skinning_transforms(rig);
        let blended: Vec<DualQuat> = mesh
            .skin
            .iter()
            .map(|influences| {
                dual::blend(
                    influences
                        .iter()
                        .filter(|influence| influence.weight > 0.0)
                        .map(|influence| (transforms[influence.joint as usize], influence.weight)),
                )
            })
            .collect();

        let mut out = mesh.clone();
        for (vertex, position) in out.positions.iter_mut().enumerate() {
            *position = blended[vertex].transform_point(*position);
        }
        for (vertex, normal) in out.normals.iter_mut().enumerate() {
            *normal = blended[vertex].rotation() * *normal;
        }
        out
    }

    /// Deforms rest-pose vertices by linear blend skinning.
    ///
    /// Kept for comparison and for integrations that cannot do better —
    /// [`Self::deform`] is what a body should use. This is the method that
    /// pinches; the tests measure the difference rather than asserting it.
    #[must_use]
    pub fn deform_linear(&self, rig: &Rig, positions: &[Vec3], weights: &SkinWeights) -> Vec<Vec3> {
        let matrices = self.skinning_matrices(rig);
        positions
            .iter()
            .zip(&weights.vertices)
            .map(|(&position, influences)| {
                influences
                    .iter()
                    .filter(|influence| influence.weight > 0.0)
                    .fold(Vec3::ZERO, |sum, influence| {
                        let matrix = matrices[influence.joint as usize];
                        sum + matrix.transform_point3(position) * influence.weight
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, Limb, Zone};

    fn rig() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs")
    }

    #[test]
    fn the_rest_pose_reproduces_the_rig() {
        let rig = rig();
        let posed = Pose::rest(&rig).forward(&rig);
        for (index, joint) in rig.joints.iter().enumerate() {
            assert!(
                posed.positions[index].distance(joint.position) < 1e-5,
                "joint {index} moved in the rest pose"
            );
        }
    }

    #[test]
    fn a_rotation_carries_every_descendant_with_it() {
        // The point of a hierarchy: rotating a shoulder moves the whole arm.
        let rig = rig();
        let shoulder = rig.in_zone(Zone::UpperLimb(Limb::ForeLeft))[0];
        let hand = rig.in_zone(Zone::Extremity(Limb::ForeLeft))[0];

        let mut pose = Pose::rest(&rig);
        pose.rotations[shoulder] = Quat::from_rotation_z(0.5);
        let posed = pose.forward(&rig);

        let rest = Pose::rest(&rig).forward(&rig);
        assert!(
            posed.positions[hand].distance(rest.positions[hand]) > 0.05,
            "the hand should follow the shoulder"
        );
        // And the distance between them is unchanged: a rotation is rigid.
        let rest_span = rest.positions[hand].distance(rest.positions[shoulder]);
        let posed_span = posed.positions[hand].distance(posed.positions[shoulder]);
        assert!(
            (rest_span - posed_span).abs() < 1e-4,
            "the arm changed length"
        );
    }

    #[test]
    fn the_root_translation_moves_the_whole_body() {
        let rig = rig();
        let mut pose = Pose::rest(&rig);
        pose.translation = Vec3::new(1.0, 2.0, 3.0);
        let posed = pose.forward(&rig);
        let rest = Pose::rest(&rig).forward(&rig);

        for index in 0..rig.len() {
            let moved = posed.positions[index] - rest.positions[index];
            assert!(moved.distance(Vec3::new(1.0, 2.0, 3.0)) < 1e-5);
        }
    }

    #[test]
    fn rest_pose_skinning_leaves_the_body_where_it_was() {
        use crate::cage::{CageConfig, build_cage};
        use crate::rig::{SkinConfig, skin};
        use crate::subdiv::catmull_clark;

        let skeleton = HumanoidParams::default().skeleton();
        let mesh = catmull_clark(
            &build_cage(&skeleton, &CageConfig::default()).expect("meshes"),
            1,
        );
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());

        let posed = Pose::rest(&rig).forward(&rig);
        let deformed = posed.deform(&rig, &mesh.positions, &weights);

        for (index, (before, after)) in mesh.positions.iter().zip(&deformed).enumerate() {
            assert!(
                before.distance(*after) < 1e-4,
                "vertex {index} drifted in the rest pose"
            );
        }
    }

    #[test]
    fn poses_interpolate() {
        let rig = rig();
        let rest = Pose::rest(&rig);
        let mut turned = rest.clone();
        turned.rotations[0] = Quat::from_rotation_y(1.0);
        turned.translation = Vec3::Y;

        assert_eq!(rest.lerp(&turned, 0.0), rest);
        let half = rest.lerp(&turned, 0.5);
        assert!((half.translation.y - 0.5).abs() < 1e-5);
        let angle = half.rotations[0].to_axis_angle().1;
        assert!((angle - 0.5).abs() < 1e-4, "half the rotation, got {angle}");
    }
}
