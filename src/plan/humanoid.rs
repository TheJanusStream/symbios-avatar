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

/// How far below horizontal a resting arm lies, in radians.
///
/// The A in A-pose. Forty degrees: far enough that neither hanging the arms nor
/// raising them to horizontal asks for a rotation big enough to tear the
/// shoulder, and shallow enough that the armpit still opens up for the mesher.
const A_POSE: f32 = 0.70;

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

        // The torso carries a separate shoulder girdle above the ribcage. That
        // is not decoration: a single node carrying spine, neck, and both arms
        // needs every one of those sockets to clear the others, and the room
        // that takes scales with its own girth — so a chest wide enough to read
        // as a chest forces a neck long enough to read as a giraffe. Splitting
        // the two lets the ribcage be broad while the girdle above it stays
        // slim, which is what shortens the neck.
        let pelvis_r = h * 0.079 * girth;
        let waist_r = h * 0.078 * girth;
        let chest_r = h * 0.088 * girth * (1.0 + 0.08 * self.shoulder_width);
        let girdle_r = h * 0.062 * girth * (1.0 + 0.06 * self.shoulder_width);
        // A neck is a good deal narrower than the skull above it. At the old
        // figure it measured WIDER than the head — 0.098 m against 0.093 — which
        // reads as a tree trunk and, worse, swallows the jaw: the chin is shaped
        // and narrows properly, but a neck two and a half times its width leaves
        // nothing of it to see.
        let neck_r = h * 0.038 * girth;
        let head_r = h * 0.075 * (1.0 + 0.25 * self.head_size);

        let ankle_y = h * 0.0686;
        let hip_drop = pelvis_r * 1.85;

        // Only the pelvis and the girdle are joints; the waist and chest between
        // them are connectors and constrain nothing.
        let pelvis_gap = pelvis_r * 1.5;
        let chest_gap = girdle_r * 1.3;
        let torso_min = pelvis_gap + h * 0.06 + chest_gap;

        let nominal_girdle = h * 0.755;
        let pelvis_y = (h * (0.5 + 0.03 * self.limb_length))
            .min(nominal_girdle - torso_min)
            .max(ankle_y + hip_drop + h * 0.10);
        let girdle_y = nominal_girdle.max(pelvis_y + torso_min);
        let chest_y = girdle_y - chest_gap;
        let waist_y = (pelvis_y + (chest_y - pelvis_y) * 0.5)
            .clamp(pelvis_y + pelvis_gap, chest_y - h * 0.02);

        // The girdle is a joint, so its neck socket has to clear the clavicles'.
        // The neck above it is a plain connector and constrains nothing — an
        // earlier floor here was invented rather than required, and it alone
        // added half a head-height of giraffe.
        // The neck has to clear the girdle's socket, but the floor was doing all
        // the work: a neck sitting exactly as high as the sockets allow leaves
        // barely any column between the collar and the jaw.
        let neck_y = girdle_y + (h * 0.072 * (1.0 + 0.3 * self.neck_length)).max(girdle_r * 1.32);
        let head_y = neck_y + (h * 0.052).max(head_r * 0.45);

        // The pelvis carries the spine and both legs; the drop to the hip is
        // what gives that joint the room to separate three sockets.
        let hip_x = pelvis_r * (1.6 + 0.35 * self.hip_width);
        let hip_y = pelvis_y - hip_drop;

        let knee_y = ankle_y + (hip_y - ankle_y) * 0.60;
        let foot_y = h * 0.0257;
        let foot_z = h * 0.057 * (1.0 + 0.3 * self.extremity_size);

        // The clavicle has to reach past the chest socket's corners before an
        // arm can attach — the single tightest constraint on the whole body.
        let clavicle_x = girdle_r * (2.15 + 0.25 * self.shoulder_width);
        let clavicle_y = girdle_y + h * 0.004;
        let shoulder_x = clavicle_x + h * 0.048;
        // Arms hang at an angle, not straight out. Built in a T-pose, posing
        // them down to walk rotates each shoulder about 75 degrees and the
        // shoulder bulges — measured the same under dual quaternions and under
        // matrices, so it is the size of the rotation and not the skinning. An
        // A-pose roughly halves the worst case in both directions, down to a
        // hanging arm and up to a raised one, and is why production models are
        // built this way.
        let arm = Vec3::new(A_POSE.cos(), -A_POSE.sin(), 0.0);
        let upper_arm = h * (0.113 + 0.025 * self.limb_length);
        let forearm = h * (0.101 + 0.025 * self.limb_length);
        let hand_len = h * 0.040 * (1.0 + 0.3 * self.extremity_size);
        let shoulder_at = Vec3::new(shoulder_x, clavicle_y, 0.0);
        let elbow_at = shoulder_at + arm * upper_arm;
        let wrist_at = elbow_at + arm * forearm;
        let hand_at = wrist_at + arm * hand_len;

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
        let girdle = skeleton.extend_from(
            chest,
            Node::new(Vec3::new(0.0, girdle_y, 0.0), girdle_r).in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            girdle,
            Node::new(Vec3::new(0.0, neck_y, 0.0), neck_r).in_zone(Zone::Neck),
        );
        // A skull takes two nodes. One leaf gives a capped tube whose dome
        // collapses under subdivision, leaving a flat-topped stub with the head
        // joint sitting at the very top of the body — which is exactly what a
        // measured rendering showed. A crown above it fills the cranium out.
        let head = skeleton.extend_from(
            neck,
            Node::new(Vec3::new(0.0, head_y, 0.0), head_r).in_zone(Zone::Head),
        );
        skeleton.extend_from(
            head,
            Node::new(Vec3::new(0.0, head_y + head_r * 0.72, 0.0), head_r * 0.66)
                .in_zone(Zone::Head),
        );

        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeLeft, Limb::HindLeft),
            (1.0, Limb::ForeRight, Limb::HindRight),
        ] {
            // Arms rest in a T-pose: VRM 1.0 requires it of exported humanoids.
            let clavicle = skeleton.extend_from(
                girdle,
                Node::new(
                    Vec3::new(side * clavicle_x, clavicle_y, 0.0),
                    h * 0.040 * girth,
                )
                .in_zone(Zone::Chest),
            );
            let shoulder = skeleton.extend_from(
                clavicle,
                Node::new(
                    Vec3::new(side * shoulder_at.x, shoulder_at.y, shoulder_at.z),
                    h * 0.038 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let elbow = skeleton.extend_from(
                shoulder,
                Node::new(
                    Vec3::new(side * elbow_at.x, elbow_at.y, elbow_at.z),
                    h * 0.032 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let wrist = skeleton.extend_from(
                elbow,
                Node::new(
                    Vec3::new(side * wrist_at.x, wrist_at.y, wrist_at.z),
                    h * 0.025 * girth,
                )
                .in_zone(Zone::LowerLimb(fore)),
            );
            skeleton.extend_from(
                wrist,
                // Slim: this is the base of the hand, not a stand-in for one.
                // While the limbs ended in these nodes they were fattened to
                // read as a fist and a boot, and now that real hands and feet
                // hang off them a blob only pokes through the part it is meant
                // to be inside.
                Node::new(
                    Vec3::new(side * hand_at.x, hand_at.y, hand_at.z),
                    h * 0.020 * extremity,
                )
                .in_zone(Zone::Extremity(fore)),
            );

            let hip = skeleton.extend_from(
                pelvis,
                Node::new(Vec3::new(side * hip_x, hip_y, 0.0), h * 0.052 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let knee = skeleton.extend_from(
                hip,
                Node::new(Vec3::new(side * hip_x, knee_y, 0.0), h * 0.042 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let ankle = skeleton.extend_from(
                knee,
                Node::new(Vec3::new(side * hip_x, ankle_y, 0.0), h * 0.030 * girth)
                    .in_zone(Zone::LowerLimb(hind)),
            );
            skeleton.extend_from(
                ankle,
                Node::new(
                    Vec3::new(side * hip_x, foot_y, foot_z),
                    h * 0.019 * extremity,
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

        // The pelvis carries the spine and two legs; the shoulder girdle carries
        // the spine, the neck, and both arms. The chest between them is a plain
        // connector, which is the whole reason the girdle exists.
        assert_eq!(skeleton.kind(0), NodeKind::Joint);
        assert_eq!(skeleton.degree(0), 3, "pelvis");
        assert_eq!(skeleton.kind(2), NodeKind::Connector, "chest");
        assert_eq!(skeleton.degree(3), 4, "shoulder girdle");

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
        // The torso and a limb both answer to the one axis. Found by zone
        // rather than by index, so adding a node does not silently retarget the
        // assertion at some other part of the body.
        let radius_in = |skeleton: &Skeleton, zone: Zone| {
            skeleton
                .nodes
                .iter()
                .find(|node| node.zone == zone)
                .expect("zone exists")
                .radius
        };
        for zone in [Zone::Chest, Zone::UpperLimb(Limb::ForeLeft)] {
            assert!(
                radius_in(&heavy, zone) > radius_in(&slight, zone) * 1.4,
                "{zone:?} should thicken with build"
            );
        }
    }
}
