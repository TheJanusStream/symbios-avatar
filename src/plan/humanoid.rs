//! The upright biped body plan.
//!
//! Nine semantic axes. Each drives several skeleton quantities, and the
//! correlations between them are written out longhand below rather than fitted:
//! `build` thickens the torso *and* the limbs, `limb_length` raises the pelvis
//! *and* therefore shortens the torso at a fixed stature, and so on.
//!
//! Several derived lengths are floored at a multiple of the joint radius they
//! serve. Those floors are not styling — they are what keeps every point of the
//! parameter space meshable (see the module docs for [`super`]).

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

/// Smallest and largest stature this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (1.2, 2.2);

/// Parameters describing one biped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumanoidParams {
    /// Standing height in metres, within [`super::humanoid_height_range`].
    #[serde(default = "default_height", with = "super::scaled")]
    pub height: f32,
    /// Overall mass, from slight to heavy.
    #[serde(default, with = "super::scaled")]
    pub build: f32,
    /// Musculature, `0..=1`.
    #[serde(default, with = "super::scaled")]
    pub muscle: f32,
    /// Width of the shoulder girdle.
    #[serde(default, with = "super::scaled")]
    pub shoulder_width: f32,
    /// Width of the pelvis.
    #[serde(default, with = "super::scaled")]
    pub hip_width: f32,
    /// Limb length relative to the torso; longer limbs raise the pelvis and so
    /// shorten the torso at a fixed stature.
    #[serde(default, with = "super::scaled")]
    pub limb_length: f32,
    /// Neck length.
    #[serde(default, with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(default, with = "super::scaled")]
    pub head_size: f32,
    /// Hand and foot size.
    #[serde(default, with = "super::scaled")]
    pub extremity_size: f32,
}

/// Neutral stature, used when a record omits the field.
fn default_height() -> f32 {
    1.75
}

impl Default for HumanoidParams {
    fn default() -> Self {
        Self {
            height: default_height(),
            build: 0.0,
            muscle: 0.0,
            shoulder_width: 0.0,
            hip_width: 0.0,
            limb_length: 0.0,
            neck_length: 0.0,
            head_size: 0.0,
            extremity_size: 0.0,
        }
    }
}

impl HumanoidParams {
    /// How much thicker than neutral this body is.
    ///
    /// Mass and musculature both add girth, and girth feeds every torso and
    /// limb radius — that single correlation is most of what makes `build` read
    /// as one coherent slider rather than a dozen independent ones.
    fn girth(&self) -> f32 {
        1.0 + 0.28 * self.build + 0.15 * self.muscle
    }
}

impl BodyPlan for HumanoidParams {
    fn sanitize(&mut self) {
        self.height = super::scaled::quantize(self.height.clamp(HEIGHT_RANGE.0, HEIGHT_RANGE.1));
        self.muscle = super::scaled::quantize(self.muscle.clamp(0.0, 1.0));
        for axis in [
            &mut self.build,
            &mut self.shoulder_width,
            &mut self.hip_width,
            &mut self.limb_length,
            &mut self.neck_length,
            &mut self.head_size,
            &mut self.extremity_size,
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

        // Torso radii. The chest also answers to shoulder width, because a broad
        // frame reads as a deep chest, not just wide-set arms.
        let pelvis_r = h * 0.0657 * girth;
        let waist_r = h * 0.0714 * girth;
        let chest_r = h * 0.0771 * girth * (1.0 + 0.08 * self.shoulder_width);

        // Vertical layout, built from the ground up so that every spacing
        // requirement can be satisfied in turn. Doing it top-down instead lets
        // two floors fight — a heavy short body wants a torso longer than the
        // stature leaves room for — and whichever clamp is applied last wins,
        // silently producing a chest the waist cannot attach to.
        let ankle_y = h * 0.0686;
        let hip_drop = pelvis_r * 1.85;

        // The chest's four sockets — waist, neck, two clavicles — each need bone
        // length proportional to the radii involved, so a heavy body needs a
        // longer torso than a slight one. Room for it comes out of the legs
        // first: dropping the pelvis keeps the stature honest, where growing the
        // chest upward would leave a maxed-out body a quarter metre too tall.
        // Only once the pelvis reaches the leg's own floor does the chest rise.
        let torso_min = pelvis_r * 1.5 + chest_r * 1.55 + h * 0.01;
        let nominal_chest = h * 0.754;
        let pelvis_y = (h * (0.543 + 0.03 * self.limb_length))
            .min(nominal_chest - torso_min)
            .max(ankle_y + hip_drop + h * 0.10);
        let chest_y = nominal_chest.max(pelvis_y + torso_min);
        let neck_y = chest_y + (h * 0.086 * (1.0 + 0.25 * self.neck_length)).max(chest_r * 1.2);
        let head_r = h * 0.0571 * (1.0 + 0.25 * self.head_size);
        let neck_r = h * 0.0343 * girth;
        let head_y = neck_y + (h * 0.086).max(neck_r * 1.5 + head_r * 0.9);

        // Both bounds are now guaranteed compatible by `torso_min`.
        let waist_y = (pelvis_y + (chest_y - pelvis_y) * 0.46)
            .clamp(pelvis_y + pelvis_r * 1.5, chest_y - chest_r * 1.55);

        // The pelvis carries the spine and both legs; the drop to the hip is
        // what gives that joint the room to separate three sockets.
        let hip_x = pelvis_r * (1.75 + 0.35 * self.hip_width);
        let hip_y = pelvis_y - hip_drop;

        let knee_y = ankle_y + (hip_y - ankle_y) * 0.55;
        let foot_y = h * 0.0257;
        let foot_z = h * 0.057 * (1.0 + 0.3 * self.extremity_size);

        // The clavicle has to reach past the waist ring's corners before an arm
        // can attach — the single tightest constraint on the whole body.
        let clavicle_x = chest_r * (1.85 + 0.35 * self.shoulder_width);
        let clavicle_y = chest_y + h * 0.006;
        let shoulder_x = clavicle_x + h * 0.051;
        let elbow_x = shoulder_x + h * (0.103 + 0.025 * self.limb_length);
        let wrist_x = elbow_x + h * (0.091 + 0.025 * self.limb_length);
        let hand_x = wrist_x + h * 0.040 * (1.0 + 0.3 * self.extremity_size);

        let extremity = 1.0 + 0.3 * self.extremity_size;

        let mut skeleton = Skeleton::new();
        let pelvis = skeleton
            .add_node(Node::new(Vec3::new(0.0, pelvis_y, 0.0), pelvis_r).in_zone(Zone::Pelvis));
        let waist = skeleton.extend_from(
            pelvis,
            Node::new(Vec3::new(0.0, waist_y, 0.0), waist_r).in_zone(Zone::Abdomen),
        );
        let chest = skeleton.extend_from(
            waist,
            Node::new(Vec3::new(0.0, chest_y, 0.0), chest_r).in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            chest,
            Node::new(Vec3::new(0.0, neck_y, 0.0), neck_r).in_zone(Zone::Neck),
        );
        skeleton.extend_from(
            neck,
            Node::new(Vec3::new(0.0, head_y, 0.0), head_r).in_zone(Zone::Head),
        );

        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeLeft, Limb::HindLeft),
            (1.0, Limb::ForeRight, Limb::HindRight),
        ] {
            // Arms rest in a T-pose: VRM 1.0 requires it of exported humanoids.
            let clavicle = skeleton.extend_from(
                chest,
                Node::new(
                    Vec3::new(side * clavicle_x, clavicle_y, 0.0),
                    h * 0.0314 * girth,
                )
                .in_zone(Zone::Chest),
            );
            let shoulder = skeleton.extend_from(
                clavicle,
                Node::new(
                    Vec3::new(side * shoulder_x, clavicle_y, 0.0),
                    h * 0.0286 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let elbow = skeleton.extend_from(
                shoulder,
                Node::new(
                    Vec3::new(side * elbow_x, clavicle_y, 0.0),
                    h * 0.0240 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let wrist = skeleton.extend_from(
                elbow,
                Node::new(
                    Vec3::new(side * wrist_x, clavicle_y, 0.0),
                    h * 0.0189 * girth,
                )
                .in_zone(Zone::LowerLimb(fore)),
            );
            skeleton.extend_from(
                wrist,
                Node::new(
                    Vec3::new(side * hand_x, clavicle_y, 0.0),
                    h * 0.0217 * extremity,
                )
                .in_zone(Zone::Extremity(fore)),
            );

            let hip = skeleton.extend_from(
                pelvis,
                Node::new(Vec3::new(side * hip_x, hip_y, 0.0), h * 0.0429 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let knee = skeleton.extend_from(
                hip,
                Node::new(Vec3::new(side * hip_x, knee_y, 0.0), h * 0.0343 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let ankle = skeleton.extend_from(
                knee,
                Node::new(Vec3::new(side * hip_x, ankle_y, 0.0), h * 0.0240 * girth)
                    .in_zone(Zone::LowerLimb(hind)),
            );
            skeleton.extend_from(
                ankle,
                Node::new(
                    Vec3::new(side * hip_x, foot_y, foot_z),
                    h * 0.0257 * extremity,
                )
                .in_zone(Zone::Extremity(hind)),
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
                self.shoulder_width = rng.random_range(-1.0..=1.0);
                self.hip_width = rng.random_range(-1.0..=1.0);
            }
            Category::Proportions => {
                self.limb_length = rng.random_range(-1.0..=1.0);
                self.neck_length = rng.random_range(-1.0..=1.0);
            }
            Category::Features => {
                self.head_size = rng.random_range(-1.0..=1.0);
                self.extremity_size = rng.random_range(-1.0..=1.0);
            }
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        put_length(out, self.height);
        put_signed(out, self.build);
        put_unit(out, self.muscle);
        put_signed(out, self.shoulder_width);
        put_signed(out, self.hip_width);
        put_signed(out, self.limb_length);
        put_signed(out, self.neck_length);
        put_signed(out, self.head_size);
        put_signed(out, self.extremity_size);
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            build: take_signed(bytes)?,
            muscle: take_unit(bytes)?,
            shoulder_width: take_signed(bytes)?,
            hip_width: take_signed(bytes)?,
            limb_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            extremity_size: take_signed(bytes)?,
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
    fn the_neutral_body_has_the_expected_topology() {
        let skeleton = HumanoidParams::default().skeleton();
        skeleton.validate().expect("valid skeleton");

        // Pelvis carries spine plus two legs; chest carries spine, neck, two arms.
        assert_eq!(skeleton.kind(0), NodeKind::Joint);
        assert_eq!(skeleton.degree(0), 3, "pelvis");
        assert_eq!(skeleton.degree(2), 4, "chest");

        // Two hands, two feet, one head: five leaves.
        let leaves = (0..skeleton.nodes.len() as u32)
            .filter(|&node| skeleton.kind(node) == NodeKind::Leaf)
            .count();
        assert_eq!(leaves, 5);
    }

    #[test]
    fn stature_scales_the_whole_body() {
        let short = HumanoidParams {
            height: 1.3,
            ..Default::default()
        }
        .skeleton();
        let tall = HumanoidParams {
            height: 2.1,
            ..Default::default()
        }
        .skeleton();
        let head_of = |s: &Skeleton| s.nodes[4].position.y;
        assert!(head_of(&tall) > head_of(&short) * 1.5);
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = HumanoidParams {
            height: 99.0,
            build: 5.0,
            muscle: -3.0,
            head_size: f32::NAN,
            ..Default::default()
        };
        params.sanitize();
        assert_eq!(params.height, HEIGHT_RANGE.1);
        assert_eq!(params.build, 1.0);
        assert_eq!(params.muscle, 0.0);
        assert_eq!(params.head_size, 0.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn build_thickens_torso_and_limbs_together() {
        let slight = HumanoidParams {
            build: -1.0,
            ..Default::default()
        }
        .skeleton();
        let heavy = HumanoidParams {
            build: 1.0,
            ..Default::default()
        }
        .skeleton();
        // Chest (2) and an upper arm (6) both respond to the one axis.
        assert!(heavy.nodes[2].radius > slight.nodes[2].radius * 1.4);
        assert!(heavy.nodes[6].radius > slight.nodes[6].radius * 1.4);
    }
}
