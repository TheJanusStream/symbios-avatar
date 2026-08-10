//! The four-legged body plan.
//!
//! Structurally this is the humanoid's problem twice over: two girdles, each a
//! four-way joint carrying a spine segment in each direction plus two legs. The
//! same radius-relative floors apply, and for the same reason.
//!
//! **What is left in this file is the graph, not the geometry.** The axes, how
//! they are clamped, rolled and encoded are here; every number a node is built
//! from comes out of [`Dimensions`], including the correlation this plan is
//! most worth reading for — legginess slims the barrel rather than changing the
//! back height, because at a given height a leggy animal is a slender one and a
//! squat animal is a barrel.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::derive::quadruped::Dimensions;
use super::{
    BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_span, take_length, take_signed,
    take_span, take_unit,
};
use super::{Composites, Limb, Zone};
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest back height this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (0.25, 1.8);

/// Parameters describing one quadruped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QuadrupedParams {
    /// Height of the back line in metres, within [`super::quadruped_height_range`].
    #[serde(with = "super::scaled")]
    pub height: f32,
    /// Body length from shoulder to hip.
    #[serde(with = "super::scaled")]
    pub body_length: f32,
    /// Overall mass, from slight to heavy.
    #[serde(with = "super::scaled")]
    pub build: f32,
    /// Musculature, `0..=1`.
    #[serde(with = "super::scaled")]
    pub muscle: f32,
    /// Legginess. Longer legs slim the body at a fixed back height.
    #[serde(with = "super::scaled")]
    pub leg_length: f32,
    /// Neck length.
    #[serde(with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(with = "super::scaled")]
    pub head_size: f32,
    /// Tail length.
    #[serde(with = "super::scaled")]
    pub tail_length: f32,
}

/// Neutral back height, used when a record omits the field.
fn default_height() -> f32 {
    0.58
}

/// The stature envelope: [`HEIGHT_RANGE`] stretched about the default (#160).
///
/// The raw stretch runs to −0.41 m — the conservative floor sits closer to
/// the default than the ceiling does, and tripling the short side crosses
/// zero. Every dimension of the plan scales with height, so any positive
/// value meshes (the sweep bisected the wall to +0.001); the floor is five
/// centimetres because a creature is a thing in a world, not a float.
fn height_envelope() -> (f32, f32) {
    let raw = super::explore_range(default_height(), HEIGHT_RANGE);
    (raw.0.max(0.05), raw.1)
}

/// The envelope every signed axis clamps and encodes against (#160).
fn signed_envelope() -> (f32, f32) {
    super::explore_range(0.0, (-1.0, 1.0))
}

/// The `muscle` envelope; its default sits at the bottom of its range (#160).
fn muscle_envelope() -> (f32, f32) {
    super::explore_range(0.0, (0.0, 1.0))
}

impl Default for QuadrupedParams {
    fn default() -> Self {
        Self {
            height: default_height(),
            body_length: 0.0,
            build: 0.0,
            muscle: 0.0,
            leg_length: 0.0,
            neck_length: 0.0,
            head_size: 0.0,
            tail_length: 0.0,
        }
    }
}

impl QuadrupedParams {
    /// The stature envelope `sanitize` clamps to, in metres (#160).
    ///
    /// Public so an editor's slider and the clamp cannot disagree about
    /// where the axis ends.
    #[must_use]
    pub fn height_envelope() -> (f32, f32) {
        height_envelope()
    }
}

impl BodyPlan for QuadrupedParams {
    fn sanitize(&mut self) {
        // See [`super::humanoid::HumanoidParams::sanitize`]: same shape, same
        // reason, and it carried the same bug (#55).
        let default = Self::default();
        self.height = super::sanitize_axis(self.height, default.height, height_envelope());
        self.muscle = super::sanitize_axis(self.muscle, default.muscle, muscle_envelope());
        for (axis, fallback) in [
            (&mut self.body_length, default.body_length),
            (&mut self.build, default.build),
            (&mut self.leg_length, default.leg_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.head_size, default.head_size),
            (&mut self.tail_length, default.tail_length),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, signed_envelope());
        }
    }

    fn skeleton(&self, composites: &Composites) -> Skeleton {
        // Passed through: see `derive::quadruped::Dimensions::of` for why a beast
        // does not read the frame axis.
        let d = Dimensions::of(self, composites);

        let mut skeleton = Skeleton::new();
        let hips = skeleton.add_node(
            Node::new(d.hips_at, d.hips_r)
                .with_scale(d.hips_section)
                .in_zone(Zone::Pelvis),
        );
        let spine = skeleton.extend_from(
            hips,
            Node::new(d.spine_at, d.spine_r)
                .with_scale(d.spine_section)
                .in_zone(Zone::Abdomen),
        );
        let withers = skeleton.extend_from(
            spine,
            Node::new(d.withers_at, d.withers_r)
                .with_scale(d.withers_section)
                .in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            withers,
            Node::new(d.neck_at, d.neck_r)
                .with_scale(d.neck_section)
                .in_zone(Zone::Neck),
        );
        skeleton.extend_from(neck, Node::new(d.head_at, d.head_r).in_zone(Zone::Head));

        let tail = skeleton.extend_from(hips, Node::new(d.tail_at, d.tail_r).in_zone(Zone::Tail));
        skeleton.extend_from(tail, Node::new(d.tip_at, d.tip_r).in_zone(Zone::Tail));

        // Left is `+X`, as it is on every plan — see `plan::humanoid` for the
        // convention and for why the `−X` side is built first. This body faces
        // `+Z` too: its head sits ahead of its tail.
        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeRight, Limb::HindRight),
            (1.0, Limb::ForeLeft, Limb::HindLeft),
        ] {
            // [`Dimensions`] gives every leg on the `+X` side; this is the
            // mirror, and it is the only place a side is decided.
            let at = |node: Vec3| Vec3::new(side * node.x, node.y, node.z);

            let stifle = skeleton.extend_from(
                hips,
                Node::new(at(d.stifle_at), d.stifle_r).in_zone(Zone::UpperLimb(hind)),
            );
            let hock = skeleton.extend_from(
                stifle,
                Node::new(at(d.hock_at), d.hock_r).in_zone(Zone::LowerLimb(hind)),
            );
            skeleton.extend_from(
                hock,
                Node::new(at(d.hind_foot_at), d.hind_foot_r).in_zone(Zone::Extremity(hind)),
            );

            let knee = skeleton.extend_from(
                withers,
                Node::new(at(d.knee_at), d.knee_r).in_zone(Zone::UpperLimb(fore)),
            );
            let fetlock = skeleton.extend_from(
                knee,
                Node::new(at(d.fetlock_at), d.fetlock_r).in_zone(Zone::LowerLimb(fore)),
            );
            skeleton.extend_from(
                fetlock,
                Node::new(at(d.fore_foot_at), d.fore_foot_r).in_zone(Zone::Extremity(fore)),
            );
        }

        skeleton
    }

    fn reroll(&mut self, category: Category, rolls: &Rolls) {
        // See [`super::humanoid::HumanoidParams::reroll`]: same shape draws,
        // same reasons, same stream names as before (#160).
        let signed = signed_envelope();
        match category {
            Category::Stature => {
                self.height = rolls.shape(
                    "quadruped.height",
                    default_height(),
                    (HEIGHT_RANGE.1 - HEIGHT_RANGE.0) * 0.5,
                    height_envelope(),
                );
            }
            Category::Build => {
                self.build = rolls.shape("quadruped.build", 0.0, 1.0, signed);
                self.muscle = rolls.shape("quadruped.muscle", 0.0, 0.5, muscle_envelope());
            }
            Category::Frame => {
                self.body_length = rolls.shape("quadruped.bodyLength", 0.0, 1.0, signed);
            }
            Category::Proportions => {
                self.leg_length = rolls.shape("quadruped.legLength", 0.0, 1.0, signed);
                self.neck_length = rolls.shape("quadruped.neckLength", 0.0, 1.0, signed);
                self.tail_length = rolls.shape("quadruped.tailLength", 0.0, 1.0, signed);
            }
            Category::Head => {
                self.head_size = rolls.shape("quadruped.headSize", 0.0, 1.0, signed);
            }
            // Nothing on this plan is a colour, a hair or an age (#53).
            Category::Colouring | Category::Hair | Category::Age => {}
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        // Version-4 bytes span the exploration envelope; see
        // [`super::humanoid::HumanoidParams::encode`] (#160).
        put_length(out, self.height);
        put_span(out, self.body_length, signed_envelope());
        put_span(out, self.build, signed_envelope());
        put_span(out, self.muscle, muscle_envelope());
        put_span(out, self.leg_length, signed_envelope());
        put_span(out, self.neck_length, signed_envelope());
        put_span(out, self.head_size, signed_envelope());
        put_span(out, self.tail_length, signed_envelope());
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            body_length: take_span(bytes, signed_envelope())?,
            build: take_span(bytes, signed_envelope())?,
            muscle: take_span(bytes, muscle_envelope())?,
            leg_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            tail_length: take_span(bytes, signed_envelope())?,
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // The version-3 byte layout, kept exactly as it was (#160).
        let mut params = Self {
            height: take_length(bytes)?,
            body_length: take_signed(bytes)?,
            build: take_signed(bytes)?,
            muscle: take_unit(bytes)?,
            leg_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            tail_length: take_signed(bytes)?,
        };
        params.sanitize();
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::NodeKind;

    #[test]
    fn the_neutral_body_has_two_four_way_girdles() {
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        skeleton.validate().expect("valid skeleton");
        assert_eq!(skeleton.degree(0), 4, "hips: spine, tail, two legs");
        assert_eq!(skeleton.degree(2), 4, "withers: spine, neck, two legs");

        // Head, tail tip, four feet.
        let leaves = (0..skeleton.nodes.len() as u32)
            .filter(|&node| skeleton.kind(node) == NodeKind::Leaf)
            .count();
        assert_eq!(leaves, 6);
    }

    #[test]
    fn legginess_slims_the_body() {
        let leggy = QuadrupedParams {
            leg_length: 1.0,
            ..Default::default()
        };
        let squat = QuadrupedParams {
            leg_length: -1.0,
            ..Default::default()
        };
        assert!(
            leggy.skeleton(&crate::Composites::default()).nodes[1].radius
                < squat.skeleton(&crate::Composites::default()).nodes[1].radius,
            "a leggy animal is a slender one at the same back height"
        );
    }

    #[test]
    fn a_short_heavy_body_stretches_instead_of_folding() {
        let params = QuadrupedParams {
            body_length: -1.0,
            build: 1.0,
            muscle: 1.0,
            leg_length: -1.0,
            ..Default::default()
        };
        let skeleton = params.skeleton(&crate::Composites::default());
        let hips = skeleton.nodes[0].position.z;
        let spine = skeleton.nodes[1].position.z;
        let girdle = skeleton.nodes[0].radius;
        assert!(
            (spine - hips) >= girdle * 2.4,
            "spine segment floored against the girdle radius"
        );
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = QuadrupedParams {
            height: -5.0,
            tail_length: f32::INFINITY,
            muscle: 9.0,
            ..Default::default()
        };
        params.sanitize();
        // Bounds are the exploration envelope (#160).
        assert_eq!(params.height, height_envelope().0);
        assert_eq!(params.tail_length, 0.0);
        assert_eq!(params.muscle, 3.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }
}
