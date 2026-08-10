//! The upright biped body plan.
//!
//! Eleven semantic axes, each driving several skeleton quantities at once. The
//! correlations between them are written out longhand rather than fitted, and
//! they live next door in [`super::derive`]: `build` thickens the torso *and*
//! the limbs, `limb_length` raises the pelvis *and* therefore shortens the
//! torso at a fixed stature, and so on.
//!
//! **What is left in this file is the graph, not the geometry.** The axes, how
//! they are clamped, how they are rolled and how they are written to a share
//! code are here; every number a node is built from comes out of
//! [`Dimensions`], and so does every coefficient's provenance note. The split
//! is #163's, and its reasoning is in that module's docs — the short version is
//! that a composite axis has to reach a dozen quantities at once, and it can
//! only do that from one place.
//!
//! Several derived lengths are floored at a multiple of the joint radius they
//! serve. Those floors are not styling — they are what keeps every point of the
//! parameter space meshable (see the module docs for [`super`]), and they are
//! listed on [`Dimensions`].

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::derive::humanoid::Dimensions;
use super::{
    BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_span, take_length, take_signed,
    take_span, take_unit,
};
use super::{Limb, Zone};
use crate::cage::limb::HALF_SEGMENT;
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest stature this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (1.2, 2.2);

/// The stature envelope: [`HEIGHT_RANGE`] stretched about the default (#160).
fn height_envelope() -> (f32, f32) {
    super::explore_range(default_height(), HEIGHT_RANGE)
}

/// The envelope every signed axis clamps and encodes against (#160).
fn signed_envelope() -> (f32, f32) {
    super::explore_range(0.0, (-1.0, 1.0))
}

/// The `muscle` envelope: its default sits at the bottom of its range, so the
/// exploration stretch runs upward only (#160).
fn muscle_envelope() -> (f32, f32) {
    super::explore_range(0.0, (0.0, 1.0))
}

/// Parameters describing one biped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HumanoidParams {
    /// Standing height in metres, within [`super::humanoid_height_range`].
    #[serde(with = "super::scaled")]
    pub height: f32,
    /// Overall mass, from slight to heavy.
    #[serde(with = "super::scaled")]
    pub build: f32,
    /// Musculature, `0..=1`.
    #[serde(with = "super::scaled")]
    pub muscle: f32,
    /// Width of the shoulder girdle.
    #[serde(with = "super::scaled")]
    pub shoulder_width: f32,
    /// Width of the pelvis.
    #[serde(with = "super::scaled")]
    pub hip_width: f32,
    /// Limb length relative to the torso; longer limbs raise the pelvis and so
    /// shorten the torso at a fixed stature.
    #[serde(with = "super::scaled")]
    pub limb_length: f32,
    /// Neck length.
    #[serde(with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(with = "super::scaled")]
    pub head_size: f32,
    /// How broad the skull is across, at a fixed head size.
    ///
    /// Negative is a narrow head and positive a broad one. See
    /// `HEAD_BREADTH_SPAN`: this scales the head and crown nodes' lateral
    /// half-extent and nothing else, so a broad skull is broad from the parietal
    /// down through the angle of the jaw while staying exactly as deep and as
    /// tall.
    ///
    /// **The broad end is the one that binds.** A socket surfaces as a hull
    /// facet only when its own plane clears every sibling ring point, and the
    /// clearance a socket demands is its LARGEST half-extent — so narrowing a
    /// section is free and widening one is not. The head carries a single
    /// socket, down to the neck, which is why this reaches a fifth where
    /// `GIRDLE_SECTION` could not.
    #[serde(with = "super::scaled")]
    pub head_breadth: f32,
    /// How long the face is below the eyes, at a fixed head size.
    ///
    /// Negative is a short face and positive a long one. It moves the head's
    /// joint up its own neck, so what changes is how much of the skull sits
    /// BELOW that joint — the jaw, the chin and the whole feature stack — while
    /// the cranium above it is untouched. See `FACE_LENGTH_SPAN`.
    ///
    /// Separate from [`Self::head_size`] deliberately, and it is the separation
    /// that is the point: head size moved the crown and the chin together, so
    /// two seeds could differ in how big their heads were and never in how long
    /// their faces were (#61).
    #[serde(with = "super::scaled")]
    pub face_length: f32,
    /// Hand and foot size.
    #[serde(with = "super::scaled")]
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
            head_breadth: 0.0,
            face_length: 0.0,
            extremity_size: 0.0,
        }
    }
}

impl HumanoidParams {
    /// The stature envelope `sanitize` clamps to, in metres (#160).
    ///
    /// Public so an editor's slider and the clamp cannot disagree about
    /// where the axis ends.
    #[must_use]
    pub fn height_envelope() -> (f32, f32) {
        height_envelope()
    }
}

impl BodyPlan for HumanoidParams {
    fn sanitize(&mut self) {
        // Fallbacks come from `Default` rather than being written out again, so
        // they cannot drift from the documented defaults. See
        // [`super::sanitize_axis`] for why the guard has to precede the clamp.
        // Ranges are the exploration envelope — the conservative spans
        // stretched [`super::EXPLORE`]× about each default (#160); the stored
        // unit's meaning is unchanged, the clamp just stops sooner refusing.
        let default = Self::default();
        self.height = super::sanitize_axis(self.height, default.height, height_envelope());
        self.muscle = super::sanitize_axis(self.muscle, default.muscle, muscle_envelope());
        for (axis, fallback) in [
            (&mut self.build, default.build),
            (&mut self.shoulder_width, default.shoulder_width),
            (&mut self.hip_width, default.hip_width),
            (&mut self.limb_length, default.limb_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.head_size, default.head_size),
            (&mut self.head_breadth, default.head_breadth),
            (&mut self.face_length, default.face_length),
            (&mut self.extremity_size, default.extremity_size),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, signed_envelope());
        }
    }

    fn skeleton(&self) -> Skeleton {
        let d = Dimensions::of(self);

        let mut skeleton = Skeleton::new();
        let pelvis = skeleton.add_node(
            Node::new(Vec3::new(0.0, d.pelvis_y, 0.0), d.pelvis_r)
                .with_scale(d.pelvis_section)
                .in_zone(Zone::Pelvis),
        );
        let waist = skeleton.extend_from(
            pelvis,
            Node::new(Vec3::new(0.0, d.waist_y, 0.0), d.waist_r)
                .with_scale(d.waist_section)
                .in_zone(Zone::Abdomen),
        );
        let chest = skeleton.extend_from(
            waist,
            Node::new(Vec3::new(0.0, d.chest_y, 0.0), d.chest_r)
                .with_scale(d.chest_section)
                .in_zone(Zone::Chest),
        );
        let girdle = skeleton.extend_from(
            chest,
            Node::new(Vec3::new(0.0, d.girdle_y, 0.0), d.girdle_r)
                .with_scale(d.girdle_section)
                .in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            girdle,
            Node::new(d.neck_at, d.neck_r)
                .with_scale(d.neck_section)
                // The joint stays on the axis and the mass goes behind it. See
                // [`NECK_LOBE`], which is half of one shape with
                // [`NECK_SECTION`]'s depth.
                .with_offset(d.neck_offset)
                .in_zone(Zone::Neck),
        );
        // A skull takes two nodes. One leaf gives a capped tube whose dome
        // collapses under subdivision, leaving a flat-topped stub with the head
        // joint sitting at the very top of the body — which is exactly what a
        // measured rendering showed. A crown above it fills the cranium out.
        //
        let head = skeleton.extend_from(
            neck,
            Node::new(Vec3::new(0.0, d.head_y, 0.0), d.head_r)
                .with_scale(d.skull_section)
                .in_zone(Zone::Head),
        );
        skeleton.extend_from(
            head,
            Node::new(Vec3::new(0.0, d.crown_y, 0.0), d.crown_r)
                .with_scale(d.skull_section)
                .in_zone(Zone::Head),
        );
        // The jaw: a hinge and a bone in the rig, no geometry in the cage
        // (#134). See [`JAW_PIVOT`] for why it cannot be a socket and
        // `face::skull` for where the mandible's mass actually comes from. The
        // pivot hangs off the head so the whole jaw turns with a head turn; the
        // tip hangs off the pivot so rotating the PIVOT is what opens the
        // mouth, about the same centre a nod uses — which is what a jaw does.
        let jaw_pivot = skeleton.extend_from(
            head,
            Node::new(d.jaw_pivot_at, d.jaw_pivot_r)
                .as_marker()
                .in_zone(Zone::Head),
        );
        // The tip's height is a share of the head's reach BELOW ITS JOINT, not
        // of its radius: that span is what `face_length` stretches, and the chin
        // rides it. See [`JAW_TIP`] for the three measurements.
        skeleton.extend_from(
            jaw_pivot,
            Node::new(d.jaw_tip_at, d.jaw_tip_r)
                .as_marker()
                .in_zone(Zone::Head),
        );

        // **A body's left limbs are the ones at `+X`** (#142). This body faces
        // `+Z` — measured off its own foot, whose toe is ahead of its heel — and
        // glTF, which is the convention every consumer reads this rig through,
        // is right-handed with `+Y` up. For a character facing `+Z` with up `+Y`,
        // right is forward cross up, which is `Z × Y`, which is `−X`; so left is
        // `+X`. Ours were the other way round until #142, and nothing had ever
        // noticed because a humanoid is mirror-symmetric and nothing in the crate
        // had ever asked which side was which. The moment a clip plays or a
        // garment is asymmetric it stops being invisible.
        //
        // **The `−X` side is built first, and that order is load-bearing.**
        // [`Rig::from_skeleton`] numbers joints breadth-first, so siblings keep
        // the order this loop inserted them, and a [`Slot`](crate::anim::Slot) is
        // a zone and an ordinal. Both clavicles live in [`Zone::Chest`], so which
        // clavicle is `Chest[2]` is decided here and nowhere else —
        // `retarget::HUMAN` addresses them by that ordinal, and it is the only
        // place in the crate where a side rides on an ordinal rather than on a
        // [`Limb`]. Reordering these two rows would move it in silence, which is
        // what `the_clavicles_are_pinned_to_their_sides_by_ordinal` exists to
        // prevent. Correcting the names was therefore done by moving the *names*
        // and leaving the geometry where it was, so not one vertex of any body
        // moved.
        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeRight, Limb::HindRight),
            (1.0, Limb::ForeLeft, Limb::HindLeft),
        ] {
            // Arms rest in an A-pose, about forty degrees below horizontal at
            // the shoulder — measured at #139, which compared this rest against
            // the CC0 reference's true T-pose and found 40.1°. The comment that
            // used to sit here claimed a T-pose because VRM 1.0 required one of
            // exported humanoids, and was stale on both counts: VRM was dropped
            // at #27. Which rest this is matters to a retarget, so it is
            // recorded rather than asserted.
            let clavicle = skeleton.extend_from(
                girdle,
                Node::new(
                    Vec3::new(side * d.clavicle_x, d.clavicle_y, 0.0),
                    d.clavicle_r,
                )
                .in_zone(Zone::Chest),
            );
            let shoulder = skeleton.extend_from(
                clavicle,
                Node::new(
                    Vec3::new(side * d.shoulder_at.x, d.shoulder_at.y, d.shoulder_at.z),
                    d.shoulder_r,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let elbow = skeleton.extend_from(
                shoulder,
                Node::new(
                    Vec3::new(side * d.elbow_at.x, d.elbow_at.y, d.elbow_at.z),
                    d.elbow_r,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let wrist = skeleton.extend_from(
                elbow,
                Node::new(
                    Vec3::new(side * d.wrist_at.x, d.wrist_at.y, d.wrist_at.z),
                    d.wrist_r,
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
                    Vec3::new(side * d.hand_at.x, d.hand_at.y, d.hand_at.z),
                    d.hand_r,
                )
                .in_zone(Zone::Extremity(fore)),
            );

            // The hip carries the pelvic silhouette now, which is new: with
            // [`PELVIS_SECTION`] narrowed, the width at hip level is these two
            // nodes and not the pelvis between them. Measured on the reference,
            // the thigh's mean radius at the hip is 0.0455 of stature, which is
            // 0.0439 of nominal `h` once the 0.965 a built body loses to
            // subdivision is taken out; 0.047 is that with a little back,
            // because these are the radii *before* subdivision shrinks them.
            //
            // Down from 0.052, and it had to come down or the body would have
            // *gained* width from #98: the hips moved in but the blobs on them
            // did not, and the silhouette is the sum. 0.047 puts the outer edge
            // at 0.0943 of stature against the reference's 0.0910.
            let hip = skeleton.extend_from(
                pelvis,
                Node::new(Vec3::new(side * d.hip_x, d.hip_y, 0.0), d.hip_r)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let knee = skeleton.extend_from(
                hip,
                Node::new(Vec3::new(side * d.hip_x, d.knee_y, d.knee_z), d.knee_r)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let ankle = skeleton.extend_from(
                knee,
                Node::new(Vec3::new(side * d.hip_x, d.ankle_y, 0.0), d.ankle_r)
                    .in_zone(Zone::LowerLimb(hind)),
            );
            // **The foot is part of the leg, not a slab hung off its end** (#111).
            // Four nodes carry it — heel, the stub that closes the heel, ball and
            // toe — so the cage rings run through it and the ankle is continuous
            // surface. Both reference bodies are built that way: the male's foot
            // is inside the leg shell and the female's inside one shell running
            // crown to sole, which is why neither has a seam where ours had one.
            //
            // The heel takes two of the four because a *bend* in a chain tilts
            // the section under it and a *joint* does not; the stub is what makes
            // the heel a joint. See the placement figures above for what that is
            // worth, measured.
            //
            // Every node carries the same section: squashed to [`FOOT_FLAT`] of
            // its width, and rolled half a ring segment so it stands on a flat
            // edge instead of on a vertex. Without the roll a foot meshed from
            // the graph rests on a keel, for exactly the reason the swept one did.
            let sole_section = |at: Vec3, radius: f32| {
                Node::new(at, radius)
                    .with_scale(d.sole_section)
                    .with_roll(HALF_SEGMENT)
                    .in_zone(Zone::Extremity(hind))
            };
            let heel = skeleton.extend_from(
                ankle,
                sole_section(Vec3::new(side * d.hip_x, d.foot_y, d.heel_z), d.heel_r),
            );
            skeleton.extend_from(
                heel,
                sole_section(Vec3::new(side * d.hip_x, d.cap_y, d.cap_z), d.cap_r),
            );
            let ball = skeleton.extend_from(
                heel,
                sole_section(Vec3::new(side * d.hip_x, d.foot_y, d.ball_z), d.ball_r),
            );
            skeleton.extend_from(
                ball,
                sole_section(Vec3::new(side * d.hip_x, d.foot_y, d.toe_z), d.toe_r),
            );
        }

        skeleton
    }

    fn reroll(&mut self, category: Category, rolls: &Rolls) {
        // Every axis is a [`Rolls::shape`] draw (#160): a Gaussian on the
        // axis's own default whose sigma is half the OLD uniform fence — so a
        // typical seed still lands where it always did — with the wildcard
        // tail reaching the whole exploration envelope. The old fences
        // survive as sigmas: the "every third seed a caricature" judgement
        // that narrowed `headBreadth` and `faceLength` to ±0.7 now simply
        // gives them a tighter sigma than their neighbours.
        //
        // Stream names are unchanged, so axis independence holds on every
        // stored seed; the distribution change is `GENERATOR_VERSION` 2's.
        let signed = signed_envelope();
        match category {
            Category::Stature => {
                self.height = rolls.shape(
                    "humanoid.height",
                    default_height(),
                    (HEIGHT_RANGE.1 - HEIGHT_RANGE.0) * 0.5,
                    height_envelope(),
                );
            }
            Category::Build => {
                self.build = rolls.shape("humanoid.build", 0.0, 1.0, signed);
                self.muscle = rolls.shape("humanoid.muscle", 0.0, 0.5, muscle_envelope());
            }
            Category::Frame => {
                self.shoulder_width = rolls.shape("humanoid.shoulderWidth", 0.0, 1.0, signed);
                self.hip_width = rolls.shape("humanoid.hipWidth", 0.0, 1.0, signed);
            }
            Category::Proportions => {
                self.limb_length = rolls.shape("humanoid.limbLength", 0.0, 1.0, signed);
                self.neck_length = rolls.shape("humanoid.neckLength", 0.0, 1.0, signed);
                // Joined the proportions from `Features` in #53. The stream
                // name is unchanged, so which VALUE a seed gives this axis has
                // not moved — only which lock holds it.
                self.extremity_size = rolls.shape("humanoid.extremitySize", 0.0, 1.0, signed);
            }
            Category::Head => {
                self.head_size = rolls.shape("humanoid.headSize", 0.0, 1.0, signed);
                self.head_breadth = rolls.shape("humanoid.headBreadth", 0.0, 0.7, signed);
                self.face_length = rolls.shape("humanoid.faceLength", 0.0, 0.7, signed);
            }
            // Nothing on the body plan is a colour, a hair or an age.
            Category::Colouring | Category::Hair | Category::Age => {}
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        // Version-4 bytes span each axis's exploration envelope — the same
        // constants `sanitize` clamps with, so the code and the clamp cannot
        // disagree (#160). Height stays in millimetres, which already covers
        // the envelope.
        put_length(out, self.height);
        put_span(out, self.build, signed_envelope());
        put_span(out, self.muscle, muscle_envelope());
        put_span(out, self.shoulder_width, signed_envelope());
        put_span(out, self.hip_width, signed_envelope());
        put_span(out, self.limb_length, signed_envelope());
        put_span(out, self.neck_length, signed_envelope());
        put_span(out, self.head_size, signed_envelope());
        put_span(out, self.head_breadth, signed_envelope());
        put_span(out, self.face_length, signed_envelope());
        put_span(out, self.extremity_size, signed_envelope());
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            build: take_span(bytes, signed_envelope())?,
            muscle: take_span(bytes, muscle_envelope())?,
            shoulder_width: take_span(bytes, signed_envelope())?,
            hip_width: take_span(bytes, signed_envelope())?,
            limb_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            head_breadth: take_span(bytes, signed_envelope())?,
            face_length: take_span(bytes, signed_envelope())?,
            extremity_size: take_span(bytes, signed_envelope())?,
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // The version-3 byte layout, whose bytes map ±1 and 0..1. Kept exactly
        // as it was so an old code decodes to the body it always named (#160).
        let mut params = Self {
            height: take_length(bytes)?,
            build: take_signed(bytes)?,
            muscle: take_unit(bytes)?,
            shoulder_width: take_signed(bytes)?,
            hip_width: take_signed(bytes)?,
            limb_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            head_breadth: take_signed(bytes)?,
            face_length: take_signed(bytes)?,
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

        // Two hands, one head, and each foot ending in both a toe and the stub
        // that closes its heel: seven leaves. The stub is what makes the heel a
        // joint rather than a bend, which is what keeps the sole flat (#111).
        //
        // MESHED leaves: this count is a contract about the cage's topology,
        // and the jaw's two rig-only markers (#134) are not the cage's — a
        // marker reads as a leaf because its meshed degree is zero, which is
        // the point of it.
        let leaves = (0..skeleton.nodes.len() as u32)
            .filter(|&node| {
                !skeleton.nodes[node as usize].marker && skeleton.kind(node) == NodeKind::Leaf
            })
            .count();
        assert_eq!(leaves, 7);

        // And the heel is that joint on both legs. Said here rather than left
        // implicit in the leaf count, because it is the property the sole
        // depends on: a degree-2 heel is a bend, and the cage stands a bend's
        // ring on the bisector and tilts the surface under it.
        let heels = (0..skeleton.nodes.len() as u32)
            .filter(|&node| {
                skeleton.kind(node) == NodeKind::Joint
                    && matches!(skeleton.nodes[node as usize].zone, Zone::Extremity(limb) if !limb.is_fore())
            })
            .count();
        assert_eq!(heels, 2, "each foot's heel has to be a joint, not a bend");
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
        // Bounds are the exploration envelope (#160): the conservative range
        // stretched EXPLORE-fold about each default, through the same
        // thousandths quantisation every sanitised value takes.
        assert_eq!(
            params.height,
            crate::plan::scaled::quantize(height_envelope().1)
        );
        assert_eq!(params.build, 3.0);
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
