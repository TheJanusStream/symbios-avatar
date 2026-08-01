//! Dual quaternions, for skinning that does not collapse.
//!
//! Linear blend skinning averages *matrices*, and the average of two rotations
//! written as matrices is not a rotation — it is a shrunken, sheared thing. Where
//! two bones rotate apart the surface between them collapses toward the axis,
//! which is the pinch every rigger knows as the candy wrapper. Measured on this
//! crate's own walk, the worst triangle at the pelvis lost 65% of its area while
//! the rest pose was untouched.
//!
//! A dual quaternion is a rigid transform written so that a weighted average of
//! several of them is still, after one normalisation, a rigid transform. Blending
//! those instead follows the shortest screw motion between the bones and keeps
//! the volume. It is the standard fix (Kavan et al. 2007) and cheap enough to run
//! per vertex on a GPU, which matters because whatever this does, an integration
//! has to be able to do in a shader.
//!
//! The trade is known and accepted: dual quaternion skinning can *bulge* where
//! linear blend pinches, most visibly at a joint twisted near half a turn. A
//! bulge reads as muscle. A pinch reads as broken.

use glam::{Quat, Vec3};

/// A rigid transform — rotation and translation, no scale.
///
/// The real part carries the rotation; the dual part carries the translation,
/// entangled with it. Nothing here handles scale, and nothing should: a
/// skinning transform that scales is a modelling mistake, and a dual quaternion
/// would silently drop it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DualQuat {
    /// Rotation.
    pub real: Quat,
    /// Translation, premultiplied by the rotation.
    pub dual: Quat,
}

impl Default for DualQuat {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl DualQuat {
    /// Leaves a point where it is.
    pub const IDENTITY: Self = Self {
        real: Quat::IDENTITY,
        dual: Quat::from_xyzw(0.0, 0.0, 0.0, 0.0),
    };

    /// The rigid transform that rotates by `rotation` then moves by
    /// `translation`.
    #[must_use]
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        let carried = Quat::from_xyzw(translation.x, translation.y, translation.z, 0.0);
        Self {
            real: rotation,
            dual: (carried * rotation) * 0.5,
        }
    }

    /// The rotation this carries.
    #[must_use]
    pub fn rotation(&self) -> Quat {
        self.real
    }

    /// The translation this carries.
    #[must_use]
    pub fn translation(&self) -> Vec3 {
        let (real, dual) = (self.real, self.dual);
        2.0 * (real.w * dual.xyz() - dual.w * real.xyz() + real.xyz().cross(dual.xyz()))
    }

    /// Moves a point.
    #[must_use]
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.real * point + self.translation()
    }

    /// Rescales so the rotation is a rotation again.
    ///
    /// A weighted sum of dual quaternions is not one until this has been done;
    /// it is the single step that makes blending them work at all.
    #[must_use]
    pub fn normalized(&self) -> Self {
        let length = self.real.length();
        if length <= f32::EPSILON {
            return Self::IDENTITY;
        }
        Self {
            real: self.real * length.recip(),
            dual: self.dual * length.recip(),
        }
    }

    /// Adds `other` scaled by `weight`, flipping it if it points the other way.
    ///
    /// Two quaternions describe the same rotation if one is the negative of the
    /// other, and averaging one against its own negative annihilates both. So a
    /// blend has to agree on a hemisphere first. Getting this wrong does not
    /// look like a shading artefact — vertices fly off to the far side of the
    /// body — and it only shows up on the poses where two bones happen to
    /// straddle the sign change.
    #[must_use]
    pub fn accumulate(&self, other: Self, weight: f32) -> Self {
        let agreeing = if self.real.dot(other.real) < 0.0 {
            -weight
        } else {
            weight
        };
        Self {
            real: self.real + other.real * agreeing,
            dual: self.dual + other.dual * agreeing,
        }
    }
}

/// Blends rigid transforms by weight, following the shortest screw motion.
///
/// Weights need not sum to one; the normalisation at the end takes care of it.
/// An empty list leaves points where they are.
#[must_use]
pub fn blend(parts: impl IntoIterator<Item = (DualQuat, f32)>) -> DualQuat {
    let mut parts = parts.into_iter();
    let Some((first, weight)) = parts.next() else {
        return DualQuat::IDENTITY;
    };

    let mut sum = DualQuat {
        real: first.real * weight,
        dual: first.dual * weight,
    };
    // Every later part is judged against the running sum rather than against the
    // first, so a chain of bones each slightly turned from the last cannot walk
    // its way across the sign change one step at a time.
    for (part, weight) in parts {
        sum = sum.accumulate(part, weight);
    }
    sum.normalized()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn about(a: Vec3, b: Vec3) -> bool {
        a.distance(b) < 1e-4
    }

    #[test]
    fn the_identity_leaves_a_point_alone() {
        let point = Vec3::new(0.3, -0.7, 1.1);
        assert!(about(DualQuat::IDENTITY.transform_point(point), point));
        assert_eq!(DualQuat::default(), DualQuat::IDENTITY);
    }

    #[test]
    fn a_rigid_transform_survives_the_round_trip() {
        let rotation = Quat::from_rotation_y(FRAC_PI_2);
        let translation = Vec3::new(1.0, 2.0, -3.0);
        let dq = DualQuat::from_rotation_translation(rotation, translation);

        assert!((dq.rotation().dot(rotation).abs() - 1.0).abs() < 1e-5);
        assert!(about(dq.translation(), translation));

        let point = Vec3::new(0.5, 0.25, 0.75);
        assert!(about(
            dq.transform_point(point),
            rotation * point + translation
        ));
    }

    #[test]
    fn blending_one_transform_reproduces_it() {
        let dq = DualQuat::from_rotation_translation(
            Quat::from_rotation_x(0.7),
            Vec3::new(-0.2, 0.4, 0.9),
        );
        let point = Vec3::new(1.0, 0.0, 0.0);
        assert!(about(
            blend([(dq, 1.0)]).transform_point(point),
            dq.transform_point(point)
        ));
    }

    #[test]
    fn blending_nothing_leaves_a_point_alone() {
        let point = Vec3::new(0.1, 0.2, 0.3);
        assert!(about(blend([]).transform_point(point), point));
    }

    #[test]
    fn weights_need_not_sum_to_one() {
        let a = DualQuat::from_rotation_translation(Quat::from_rotation_z(0.4), Vec3::X);
        let b = DualQuat::from_rotation_translation(Quat::from_rotation_z(-0.4), Vec3::Y);
        let point = Vec3::new(0.6, 0.2, -0.1);
        assert!(about(
            blend([(a, 0.5), (b, 0.5)]).transform_point(point),
            blend([(a, 5.0), (b, 5.0)]).transform_point(point)
        ));
    }

    #[test]
    fn a_blend_of_rotations_is_still_a_rotation() {
        // The whole point. Averaging the matrices of these two shrinks whatever
        // sits between them; averaging the dual quaternions does not.
        let a = DualQuat::from_rotation_translation(Quat::from_rotation_y(-1.2), Vec3::ZERO);
        let b = DualQuat::from_rotation_translation(Quat::from_rotation_y(1.2), Vec3::ZERO);
        let blended = blend([(a, 0.5), (b, 0.5)]);
        assert!((blended.real.length() - 1.0).abs() < 1e-5);

        for point in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.3, -0.6, 0.2)] {
            let moved = blended.transform_point(point);
            assert!(
                (moved.length() - point.length()).abs() < 1e-4,
                "{point:?} of length {} became {moved:?} of length {}",
                point.length(),
                moved.length()
            );
        }
    }

    #[test]
    fn a_blend_beats_a_matrix_average_for_keeping_length() {
        let turn = 1.4f32;
        let (a, b) = (Quat::from_rotation_z(-turn), Quat::from_rotation_z(turn));
        let point = Vec3::X;

        let dual = blend([
            (DualQuat::from_rotation_translation(a, Vec3::ZERO), 0.5),
            (DualQuat::from_rotation_translation(b, Vec3::ZERO), 0.5),
        ])
        .transform_point(point);
        let linear = (a * point) * 0.5 + (b * point) * 0.5;

        assert!(
            (dual.length() - 1.0).abs() < 1e-4,
            "dual blending lost length: {}",
            dual.length()
        );
        assert!(
            linear.length() < 0.9,
            "the matrix average was supposed to shrink, and gave {}",
            linear.length()
        );
    }

    #[test]
    fn opposite_signs_blend_instead_of_cancelling() {
        // The same rotation written both ways round. Summed naively these
        // annihilate and the vertex ends up wherever the zero quaternion sends
        // it, which is nowhere near the body.
        let rotation = Quat::from_rotation_x(0.9);
        let same = DualQuat {
            real: -rotation,
            dual: -DualQuat::from_rotation_translation(rotation, Vec3::ZERO).dual,
        };
        let plain = DualQuat::from_rotation_translation(rotation, Vec3::ZERO);

        let point = Vec3::new(0.0, 1.0, 0.0);
        let blended = blend([(plain, 0.5), (same, 0.5)]).transform_point(point);
        assert!(
            about(blended, rotation * point),
            "{blended:?} against {:?}",
            rotation * point
        );
    }

    #[test]
    fn a_half_turn_apart_still_produces_a_finite_point() {
        // The one case dual quaternion blending is known to handle badly. It
        // must not produce a NaN, whatever else it does.
        let a = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::ZERO);
        let b = DualQuat::from_rotation_translation(Quat::from_rotation_y(PI), Vec3::ZERO);
        let moved = blend([(a, 0.5), (b, 0.5)]).transform_point(Vec3::new(1.0, 0.5, 0.0));
        assert!(moved.is_finite(), "{moved:?}");
    }

    #[test]
    fn translation_alone_blends_linearly() {
        let a = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::new(2.0, 0.0, 0.0));
        let b = DualQuat::from_rotation_translation(Quat::IDENTITY, Vec3::new(0.0, 4.0, 0.0));
        let blended = blend([(a, 0.25), (b, 0.75)]);
        assert!(about(blended.translation(), Vec3::new(0.5, 3.0, 0.0)));
    }
}
