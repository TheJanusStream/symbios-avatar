//! The four-legged body plan.
//!
//! Structurally this is the humanoid's problem twice over: two girdles, each a
//! four-way joint carrying a spine segment in each direction plus two legs. The
//! same radius-relative floors apply, and for the same reason.
//!
//! One correlation is worth calling out. `leg_length` reduces girth rather than
//! changing the back height: at a given height a leggy animal is a slender one,
//! and a squat animal is a barrel. Letting the two axes move independently
//! produces bodies that read as mistakes.

use glam::Vec3;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use serde::{Deserialize, Serialize};

use super::{
    BodyPlan, Category, PlanDecodeError, put_length, put_signed, put_unit, take_length,
    take_signed, take_unit,
};
use super::{Limb, Zone};
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest back height this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (0.25, 1.8);

/// Parameters describing one quadruped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuadrupedParams {
    /// Height of the back line in metres, within [`super::quadruped_height_range`].
    #[serde(default = "default_height", with = "super::scaled")]
    pub height: f32,
    /// Body length from shoulder to hip.
    #[serde(default, with = "super::scaled")]
    pub body_length: f32,
    /// Overall mass, from slight to heavy.
    #[serde(default, with = "super::scaled")]
    pub build: f32,
    /// Musculature, `0..=1`.
    #[serde(default, with = "super::scaled")]
    pub muscle: f32,
    /// Legginess. Longer legs slim the body at a fixed back height.
    #[serde(default, with = "super::scaled")]
    pub leg_length: f32,
    /// Neck length.
    #[serde(default, with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(default, with = "super::scaled")]
    pub head_size: f32,
    /// Tail length.
    #[serde(default, with = "super::scaled")]
    pub tail_length: f32,
}

/// Neutral back height, used when a record omits the field.
fn default_height() -> f32 {
    0.58
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
    /// How much thicker than neutral this body is.
    fn girth(&self) -> f32 {
        (1.0 + 0.28 * self.build + 0.15 * self.muscle) * (1.0 - 0.18 * self.leg_length)
    }
}

impl BodyPlan for QuadrupedParams {
    fn sanitize(&mut self) {
        self.height = super::scaled::quantize(self.height.clamp(HEIGHT_RANGE.0, HEIGHT_RANGE.1));
        self.muscle = super::scaled::quantize(self.muscle.clamp(0.0, 1.0));
        for axis in [
            &mut self.body_length,
            &mut self.build,
            &mut self.leg_length,
            &mut self.neck_length,
            &mut self.head_size,
            &mut self.tail_length,
        ] {
            *axis = super::scaled::quantize(if axis.is_finite() {
                axis.clamp(-1.0, 1.0)
            } else {
                0.0
            });
        }
        if !self.height.is_finite() {
            self.height = default_height();
        }
        if !self.muscle.is_finite() {
            self.muscle = 0.0;
        }
    }

    fn skeleton(&self) -> Skeleton {
        let h = self.height;
        let girth = self.girth();

        let girdle_r = h * 0.181 * girth;
        let withers_r = h * 0.190 * girth;
        let spine_r = h * 0.198 * girth;
        let neck_r = h * 0.103 * girth;

        // Spine segments are floored at a multiple of the girdles they join, so
        // a short-bodied heavy animal stretches rather than folding its girdles
        // into each other.
        let stretch = 1.0 + 0.25 * self.body_length;
        let segment_floor = 2.5 * girdle_r.max(withers_r).max(spine_r);

        let withers_z = h * 0.345 * stretch;
        let spine_z = withers_z - (h * 0.483 * stretch).max(segment_floor);
        let hips_z = spine_z - (h * 0.448 * stretch).max(segment_floor);

        let neck_z = withers_z + (h * 0.379 * (1.0 + 0.3 * self.neck_length)).max(withers_r * 2.4);
        let head_z = neck_z + (h * 0.276).max(neck_r * 2.2);

        let tail_reach = h * 0.45 * (1.0 + 0.6 * self.tail_length);
        let tail_z = hips_z - tail_reach.max(girdle_r * 2.7);
        let tail_r = h * 0.055 * girth;
        let tip_z = tail_z - tail_reach.max(tail_r * 2.5);

        // Each girdle spreads its legs far enough that the two sockets separate,
        // and drops far enough that the bone can carry them there.
        let rear_x = girdle_r * 1.55;
        let front_x = withers_r * 1.55;
        let rear_leg_y = h * 0.966 - girdle_r * 1.95;
        let front_leg_y = h - withers_r * 1.95;
        let foot_y = h * 0.086;

        let mut skeleton = Skeleton::new();
        let hips = skeleton
            .add_node(Node::new(Vec3::new(0.0, h * 0.966, hips_z), girdle_r).in_zone(Zone::Pelvis));
        let spine = skeleton.extend_from(
            hips,
            Node::new(Vec3::new(0.0, h, spine_z), spine_r).in_zone(Zone::Abdomen),
        );
        let withers = skeleton.extend_from(
            spine,
            Node::new(Vec3::new(0.0, h, withers_z), withers_r).in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            withers,
            Node::new(Vec3::new(0.0, h * 1.086, neck_z), neck_r).in_zone(Zone::Neck),
        );
        skeleton.extend_from(
            neck,
            Node::new(
                Vec3::new(0.0, h * 1.069, head_z),
                h * 0.129 * (1.0 + 0.25 * self.head_size),
            )
            .in_zone(Zone::Head),
        );

        let tail = skeleton.extend_from(
            hips,
            Node::new(Vec3::new(0.0, h * 0.862, tail_z), tail_r).in_zone(Zone::Tail),
        );
        skeleton.extend_from(
            tail,
            Node::new(Vec3::new(0.0, h * 0.724, tip_z), h * 0.031 * girth).in_zone(Zone::Tail),
        );

        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeLeft, Limb::HindLeft),
            (1.0, Limb::ForeRight, Limb::HindRight),
        ] {
            let stifle = skeleton.extend_from(
                hips,
                Node::new(
                    Vec3::new(side * rear_x, rear_leg_y, hips_z - h * 0.017),
                    h * 0.078 * girth,
                )
                .in_zone(Zone::UpperLimb(hind)),
            );
            let hock = skeleton.extend_from(
                stifle,
                Node::new(
                    Vec3::new(
                        side * rear_x,
                        foot_y + (rear_leg_y - foot_y) * 0.36,
                        hips_z + h * 0.052,
                    ),
                    h * 0.055 * girth,
                )
                .in_zone(Zone::LowerLimb(hind)),
            );
            skeleton.extend_from(
                hock,
                Node::new(
                    Vec3::new(side * rear_x, foot_y, hips_z + h * 0.121),
                    h * 0.062 * girth,
                )
                .in_zone(Zone::Extremity(hind)),
            );

            // Set slightly behind the withers, as a real shoulder is. Attaching
            // it at exactly the girdle's depth would leave the leg's socket ring
            // tied with the spine axis, and a hull cannot resolve four points on
            // a knife edge.
            let knee = skeleton.extend_from(
                withers,
                Node::new(
                    Vec3::new(side * front_x, front_leg_y, withers_z - h * 0.014),
                    h * 0.078 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let fetlock = skeleton.extend_from(
                knee,
                Node::new(
                    Vec3::new(
                        side * front_x,
                        foot_y + (front_leg_y - foot_y) * 0.36,
                        withers_z + h * 0.017,
                    ),
                    h * 0.055 * girth,
                )
                .in_zone(Zone::LowerLimb(fore)),
            );
            skeleton.extend_from(
                fetlock,
                Node::new(
                    Vec3::new(side * front_x, foot_y, withers_z + h * 0.086),
                    h * 0.062 * girth,
                )
                .in_zone(Zone::Extremity(fore)),
            );
        }

        skeleton
    }

    fn reroll(&mut self, category: Category, rng: &mut Pcg64Mcg) {
        match category {
            Category::Stature => {
                self.height = rng.random_range(HEIGHT_RANGE.0..=HEIGHT_RANGE.1);
            }
            Category::Build => {
                self.build = rng.random_range(-1.0..=1.0);
                self.muscle = rng.random_range(0.0..=1.0);
            }
            Category::Frame => {
                self.body_length = rng.random_range(-1.0..=1.0);
            }
            Category::Proportions => {
                self.leg_length = rng.random_range(-1.0..=1.0);
                self.neck_length = rng.random_range(-1.0..=1.0);
                self.tail_length = rng.random_range(-1.0..=1.0);
            }
            Category::Features => {
                self.head_size = rng.random_range(-1.0..=1.0);
            }
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        put_length(out, self.height);
        put_signed(out, self.body_length);
        put_signed(out, self.build);
        put_unit(out, self.muscle);
        put_signed(out, self.leg_length);
        put_signed(out, self.neck_length);
        put_signed(out, self.head_size);
        put_signed(out, self.tail_length);
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
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
        let skeleton = QuadrupedParams::default().skeleton();
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
            leggy.skeleton().nodes[1].radius < squat.skeleton().nodes[1].radius,
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
        let skeleton = params.skeleton();
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
        assert_eq!(params.height, HEIGHT_RANGE.0);
        assert_eq!(params.tail_length, 0.0);
        assert_eq!(params.muscle, 1.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }
}
