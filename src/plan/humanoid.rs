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
//! [`Dimensions`], and so does every coefficient's provenance note. The
//! reasoning for the split is in that module's docs — the short version is
//! that a composite axis has to reach a dozen quantities at once, and it can
//! only do that from one place.
//!
//! Several derived lengths are floored at a multiple of the joint radius they
//! serve. Those floors are not styling — they are what keeps every point of the
//! parameter space meshable (see the module docs for [`super`]), and they are
//! listed on [`Dimensions`].

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::derive::humanoid::{Dimensions, between};
use super::{Anatomy, CompositePriors, Composites, Limb, Zone};
use super::{
    BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_span, take_length, take_signed,
    take_span, take_unit,
};
use crate::cage::limb::HALF_SEGMENT;
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest stature this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (1.2, 2.2);

/// The stature envelope: [`HEIGHT_RANGE`] stretched about the default.
fn height_envelope() -> (f32, f32) {
    super::explore_range(default_height(), HEIGHT_RANGE)
}

/// How much of its old width a per-region axis draws at, now that it is an
/// offset on a composite rather than the thing that shapes the body.
///
/// **A third, and the number is bounded from both sides.** Too wide and the
/// offset out-swings the composite it is correcting, which is the incoherence
/// the composite scheme exists to remove; too narrow and a rolled population
/// is one body at several sizes, with every seed's shoulders exactly where
/// `femininity` and `mass` put them. At a third, `shoulder_width`'s typical
/// draw moves the
/// shoulders about 2.7% where the two composites move them 13% and 20% at their
/// own ±1 — a correction that can be seen and cannot take over.
///
/// The WILDCARD tail is deliberately not scaled: one seed in thirty still draws
/// uniformly over the whole exploration envelope, so an extreme offset stays
/// reachable by a roll. Rarely being a caricature is the envelope's own
/// decision and this does not revisit it.
///
/// Provenance: **judged by render**, on reroll contact sheets against the
/// same seeds at full width.
const OFFSET_SIGMA: f32 = 1.0 / 3.0;

/// How far each foot's level run turns away from the midline, in radians.
///
/// **Seven degrees, which is the low end of a relaxed stance and reads as
/// standing rather than as a duck** (#306). Measured values for a natural
/// stance run from about five to fifteen degrees per foot; the plan takes
/// the quiet end because a rest pose is the pose every still is judged in,
/// and because every foot-facing goal in the gait reads this direction as
/// the foot's forward. Sized by render, on the foot close-up's overhead view
/// and the walk sheets.
const TOE_OUT: f32 = 7.0_f32.to_radians();

/// Where a rolled stature centres at each end of the frame axis, in metres:
/// the measured adult means, men's then women's.
///
/// **The measurement itself, not a ratio on the plan's default** (#486,
/// Overlands #1953). Through generation 11 the centre was `default_height()`
/// times `frame(f, 1.75, 1.62)`, which kept the plan's 1.75 m as the neutral
/// body and moved it by the ratio of the means - and with the frame axis one
/// bell on the middle that was the honest bargain. Now the axis draws its two
/// populations at the references themselves, so the centre at each reference
/// is that population's own mean: `derive::humanoid::between` of the
/// two, and a body halfway between is the midpoint of the sexes, 1.69 m.
/// `default_height()` stays 1.75: it is what an unrolled record stands at,
/// and that is not this draw's question.
///
/// Provenance: **looked up**, NHANES 2011-2016 adult means (owner call on
/// the source, #1953).
const STATURE_MEANS: (f32, f32) = (1.757, 1.618);

/// How far a rolled stature strays from its centre, in metres, at each end of
/// the frame axis: men's then women's.
///
/// **The measured within-sex standard deviation, less what the frame axis
/// already carries** (#486). NHANES 2011-2016 puts it at 7.6 cm for men and
/// 6.4 cm for women. A rolled frame is itself spread about its reference -
/// `composites::FRAME_SIGMA`, 0.4 - and the centre moves 0.0695 m per unit
/// of the axis, so a sex's statures already spread 0.028 m before this draw
/// adds anything. The two add in quadrature:
///
/// ```text
///   sqrt(0.0707^2 + 0.028^2) = 0.076      men
///   sqrt(0.0576^2 + 0.028^2) = 0.064      women
/// ```
///
/// Read at the frame axis clamped to its references, so a wildcard frame
/// past either end draws with that end's spread rather than an extrapolated
/// one.
///
/// **History.** Through generation 11 this was one sigma, 0.12 m, about 1.7x
/// the within-sex figure, because a frame axis drawn as one bell on the middle
/// could not carry the two sexes' spread and the stature draw had to fake the
/// mix. With the frame drawn as two populations it no longer has to, and the
/// narrower figure is the measured one. Before that (generation 2) the
/// blanket rule had given stature half its fence, 0.5 m, and a population
/// whose standard deviation measured 0.537 m. **The exploration a wide sigma
/// stood in for is not lost**: the wildcard is untouched, so one seed in
/// thirty still draws uniformly over the whole stature envelope and giants
/// and small bodies remain a roll away.
///
/// Provenance: **looked up** for the two deviations, **derived** for the
/// subtraction (#1953).
const STATURE_SIGMA: (f32, f32) = (0.0707, 0.0576);

/// The envelope every signed axis clamps and encodes against.
fn signed_envelope() -> (f32, f32) {
    super::explore_range(0.0, (-1.0, 1.0))
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
    /// their faces were.
    #[serde(with = "super::scaled")]
    pub face_length: f32,
    /// Hand and foot size.
    #[serde(with = "super::scaled")]
    pub extremity_size: f32,
    /// How much chest there is, over what the composites already predict.
    ///
    /// **An OFFSET on `torso::ChestTraits`, not a slider of its own** (#273).
    /// The two-tier contract is that a quantity is `formula(composites)` and
    /// then the per-region axis on top: the composite carries the intent, which
    /// is what lets the formulas improve later without a stored avatar losing
    /// what it meant, and this carries the choice a creator made that no
    /// formula predicted. Zero is whatever `femininity`, `mass`, `body_fat` and
    /// `age` came to on their own.
    ///
    /// **Not a route back to `build`.** Those went at #164/#169 because a
    /// global build axis and a composite tier cannot both be the truth; this is
    /// regional and signed about the formula's own answer, which is the shape
    /// every axis on this struct has.
    #[serde(with = "super::scaled")]
    pub chest_volume: f32,
    /// How far that chest stands off the ribcage against how far it spreads
    /// across it, at a fixed volume.
    ///
    /// **The shape axis rather than the size one**, and the separation is
    /// [`Self::head_size`] and [`Self::face_length`]'s: two bodies can carry
    /// the same amount of chest and differ entirely in whether it is a high
    /// tight one or a broad soft one, and an axis that moved both together
    /// could never say so. Negative spreads it and positive stands it off.
    #[serde(with = "super::scaled")]
    pub chest_projection: f32,
    /// How high the chest sits, over what age and body fat predict.
    ///
    /// Positive lifts and positive also refuses the descent `Composites::ageing`
    /// asks for; negative lets it hang. The descent itself stays the
    /// composites' — this says what they got wrong about one body.
    #[serde(with = "super::scaled")]
    pub chest_lift: f32,
    /// How far apart the pair sits, over the azimuth the trunk's own constant
    /// names.
    ///
    /// **The gap MakeHuman fills with `breast-dist` and Second Life with
    /// Cleavage** (#289), and until now it was a hard constant — the one
    /// quantity on the chest a creator could not say anything about. Negative
    /// is close-set and positive wide. It is an AZIMUTH about the trunk's own
    /// axis rather than a lateral offset, which `torso::SPACING` records as a
    /// refutation rather than a preference: written lateral, the delivered
    /// separation depended on how far the chest stood off.
    #[serde(with = "super::scaled")]
    pub chest_spacing: f32,
    /// Where the volume sits between the upper pole and the lower one, over
    /// what the shape derives.
    ///
    /// **MakeHuman's `breast-volume-vert`**, and a direct offset on
    /// `torso::POLES` per the two-tier contract: the derivation carries life's
    /// preferred 45:55 and this carries the choice no formula predicted.
    /// Positive is a fuller lower pole and negative an evenly divided one, at
    /// a fixed volume and a fixed projection — a lobe's crest is its
    /// coefficient whatever its span is.
    ///
    /// **Not a second ptosis axis.** Age descends the peak toward a fold that
    /// holds still, which is what the Regnault grades measure; this moves the
    /// split and the fold under it together, because that fold is seated as a
    /// share of the lower pole's own span.
    #[serde(with = "super::scaled")]
    pub chest_fullness: f32,
}

/// Neutral stature, used when a record omits the field.
fn default_height() -> f32 {
    1.75
}

impl Default for HumanoidParams {
    fn default() -> Self {
        Self {
            height: default_height(),
            shoulder_width: 0.0,
            hip_width: 0.0,
            limb_length: 0.0,
            neck_length: 0.0,
            head_size: 0.0,
            head_breadth: 0.0,
            face_length: 0.0,
            extremity_size: 0.0,
            chest_volume: 0.0,
            chest_projection: 0.0,
            chest_lift: 0.0,
            chest_spacing: 0.0,
            chest_fullness: 0.0,
        }
    }
}

impl HumanoidParams {
    /// The stature envelope `sanitize` clamps to, in metres.
    ///
    /// Public so an editor's slider and the clamp cannot disagree about
    /// where the axis ends.
    #[must_use]
    pub fn height_envelope() -> (f32, f32) {
        height_envelope()
    }
}

impl BodyPlan for HumanoidParams {
    fn anatomy(&self) -> Anatomy {
        Anatomy::HUMAN
    }

    fn composite_priors(&self) -> CompositePriors {
        CompositePriors::PERSON
    }

    fn sanitize(&mut self) {
        // Fallbacks come from `Default` rather than being written out again, so
        // they cannot drift from the documented defaults. See
        // [`super::sanitize_axis`] for why the guard has to precede the clamp.
        // Ranges are the exploration envelope — the conservative spans
        // stretched [`super::EXPLORE`]× about each default (#160); the stored
        // unit's meaning is unchanged, the clamp just stops sooner refusing.
        let default = Self::default();
        self.height = super::sanitize_axis(self.height, default.height, height_envelope());
        for (axis, fallback) in [
            (&mut self.shoulder_width, default.shoulder_width),
            (&mut self.hip_width, default.hip_width),
            (&mut self.limb_length, default.limb_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.head_size, default.head_size),
            (&mut self.head_breadth, default.head_breadth),
            (&mut self.face_length, default.face_length),
            (&mut self.extremity_size, default.extremity_size),
            (&mut self.chest_volume, default.chest_volume),
            (&mut self.chest_projection, default.chest_projection),
            (&mut self.chest_lift, default.chest_lift),
            (&mut self.chest_spacing, default.chest_spacing),
            (&mut self.chest_fullness, default.chest_fullness),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, signed_envelope());
        }
    }

    fn skeleton(&self, composites: &Composites) -> Skeleton {
        let d = Dimensions::of(self, composites);

        let mut skeleton = Skeleton::new();
        skeleton.anatomy = self.anatomy();
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
        // The brows and the mouth corners: one marker per side of each,
        // rig-only like the jaw's pair, each a LEAF hanging straight off the
        // head — which is also how `skin::bind` tells the families apart with
        // no names to read: the jaw is the marker chain (a marker whose parent
        // is a marker), a lone marker leaf ABOVE the head joint is a brow and
        // one BELOW it is a mouth corner, and the side each drives is the sign
        // of its `x`. See [`super::derive::humanoid::BROW_JOINT`] and
        // [`super::derive::humanoid::CORNER_JOINT`] for why each sits on the
        // skull's axis rather than on the feature it drives.
        for (at, radius) in [(d.brow_at, d.brow_r), (d.corner_at, d.corner_r)] {
            for side in [1.0f32, -1.0] {
                skeleton.extend_from(
                    head,
                    Node::new(Vec3::new(at.x * side, at.y, at.z), radius)
                        .as_marker()
                        .in_zone(Zone::Head),
                );
            }
        }

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
                // From the ankle down the chain sits at `foot_x`, which is
                // `hip_x` unless the stance clearance floor binds (#305).
                Node::new(Vec3::new(side * d.foot_x, d.ankle_y, 0.0), d.ankle_r)
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
            // Every node reaches the same DEPTH, and that is not the same thing
            // as every node carrying the same section (#220). The foot's nodes
            // do not share a radius — the ball is the widest of them and the toe
            // the narrowest — so one shared squash asked each of them for a
            // different depth, and four capsules bottoming out at four different
            // heights make a sole that is convex rather than flat. The body then
            // stood 11.7 mm into its own ground plane at the ball while its heel
            // and toe hung 5.3 and 7.8 mm clear, and every consumer inherited it:
            // standing, walking, and — once the ankle articulated — a roll that
            // rested the foot on a rim above its own deepest point.
            //
            // So the section is solved per node instead, and solved against what
            // the mesh DELIVERS rather than what the node asks for. A node whose
            // centre sits `y` above the ground and whose section is asked to
            // reach `reach` below it lands its surface at `y - reach * kept`,
            // where `kept` is the share subdivision leaves behind; setting that
            // to zero gives `reach = y / kept`, which is the whole rule. The
            // level run and the cap carry different `kept` because the cap is a
            // short stub between larger neighbours and Catmull-Clark smooths a
            // small node harder — the same effect that [`FOOT_KEPT`] records
            // across the foot's width, measured again down its depth.
            //
            // Asking the cap for more than the plane is therefore not a fudge to
            // push it through the floor: it is asking for exactly the surplus
            // that smoothing removes, and it is what puts the back of the sole
            // on the ground. With one shared depth it bore 8.2% of the contact
            // behind the ankle against a reference 11.2% (male) and 13.7%
            // (female); the heel is where a foot's weight arrives, and a heel
            // riding clear of the floor is a body on tiptoe.
            //
            // The roll is unchanged and still load-bearing: it stands a section
            // on a flat edge instead of on a vertex, and without it a foot meshed
            // from the graph rests on a keel for exactly the reason the swept one
            // did.
            //
            // [`FOOT_KEPT`]: super::derive::humanoid
            // **A few degrees of toe-out** (#306): both feet ran dead parallel
            // and read as stood on rails. The level run turns about the ankle
            // in the ground plane, toe away from the midline, so the heel
            // swings in by as much as the toe swings out; the sole stays level
            // because the turn is about the vertical. `TOE_OUT` is a plan
            // constant rather than a record axis.
            let toe_out = |at: Vec3| {
                let (sin, cos) = (side * TOE_OUT).sin_cos();
                let x = side * d.foot_x;
                Vec3::new(x + at.z * sin, at.y, at.z * cos)
            };
            let sole_section = |at: Vec3, radius: f32, kept: f32| {
                let reach = at.y / kept;
                Node::new(at, radius)
                    .with_scale(Vec2::new(1.0, reach / radius))
                    .with_roll(HALF_SEGMENT)
                    .in_zone(Zone::Extremity(hind))
            };
            let heel = skeleton.extend_from(
                ankle,
                sole_section(
                    toe_out(Vec3::new(0.0, d.foot_y, d.heel_z)),
                    d.heel_r,
                    d.sole_kept,
                ),
            );
            skeleton.extend_from(
                heel,
                sole_section(
                    toe_out(Vec3::new(0.0, d.cap_y, d.cap_z)),
                    d.cap_r,
                    d.cap_kept,
                ),
            );
            let ball = skeleton.extend_from(
                heel,
                sole_section(
                    toe_out(Vec3::new(0.0, d.foot_y, d.ball_z)),
                    d.ball_r,
                    d.sole_kept,
                ),
            );
            skeleton.extend_from(
                ball,
                sole_section(
                    toe_out(Vec3::new(0.0, d.foot_y, d.toe_z)),
                    d.toe_r,
                    d.sole_kept,
                ),
            );
        }

        skeleton
    }

    fn reroll(&mut self, category: Category, rolls: &Rolls, composites: &Composites) {
        // Every axis is a [`Rolls::shape`] draw (#160): a Gaussian on the
        // axis's own default whose sigma was half the OLD uniform fence, with
        // a wildcard tail reaching the whole exploration envelope. The old
        // fences survive as sigmas: the "every third seed a caricature"
        // judgement that narrowed `headBreadth` and `faceLength` now simply
        // gives them a tighter sigma than their neighbours.
        //
        // **Every sigma here is `OFFSET_SIGMA` of what it was, and the axes are
        // offsets now rather than the body** (#169, generation 3). Under the
        // two-tier model a quantity is `formula(composites)` and then this
        // applied on top, so a body's shoulders should come from `femininity`
        // and `mass` — which move them 13% and 20% at their own ±1 — and this
        // axis should say what those two got wrong, which is a few percent.
        // Drawn at the old width it was the reverse: the offset out-swung the
        // composite and produced the tall-heavy-gaunt incoherence this
        // generation exists to fix.
        //
        // Stream names are unchanged, so axis independence holds on every
        // stored seed; the DISTRIBUTION change is what `GENERATOR_VERSION` 3
        // is for.
        let signed = signed_envelope();
        let offset = |axis: &str, sigma: f32| rolls.shape(axis, 0.0, sigma * OFFSET_SIGMA, signed);
        match category {
            Category::Stature => {
                // **The one correlation on this plan, and it is the loudest
                // one a body has** (#169). Stature is dimorphic, and until
                // generation 3 the frame axis and the height axis were drawn in
                // ignorance of each other, so a seed was as likely to be a 1.9 m
                // body with a fully feminine frame as anything else.
                //
                // **At the measured means since generation 12** (#486,
                // Overlands #1953): the centre is the measured mean between
                // the two references and the spread is each sex's measured
                // deviation less what the frame axis carries - see
                // `STATURE_MEANS` and `STATURE_SIGMA`. Through generation 11 it
                // was a ratio on `default_height()`, 1.818 and 1.683 m at the
                // two ends, with one wide sigma standing in for the sexes'
                // mix.
                //
                // Provenance: **looked up**, NHANES 2011-2016 (#1953).
                let f = composites.femininity;
                let stature = between(f, STATURE_MEANS.0, STATURE_MEANS.1);
                let sigma = between(f.clamp(-1.0, 1.0), STATURE_SIGMA.0, STATURE_SIGMA.1);
                self.height = rolls.shape("humanoid.height", stature, sigma, height_envelope());
            }
            Category::Build => {
                // **Build rather than Frame, and the choice is a promise to a
                // creator with a lock set** (#273). Frame is the skeleton — the
                // frame axis, the shoulders, the hips — and a chest is soft
                // tissue carried on it: what fills it is `mass` and `body_fat`,
                // which are this category's own composites. A creator who locks
                // Build to keep a heavy body heavy expects its chest to stay
                // with it, and one who locks Frame to keep a silhouette expects
                // the skeleton and not the flesh.
                //
                // It also fills the arm that had nothing in it, which is the
                // sign the category was right: Build named the two composites
                // and no plan axis, because the axes it would have held were
                // the two that retired into them.
                self.chest_volume = offset("humanoid.chestVolume", 1.0);
                // Tighter, for `headBreadth`'s reason: a shape axis at the same
                // sigma as its own size axis produces a caricature about every
                // third seed, and the composites already place the chest.
                self.chest_projection = offset("humanoid.chestProjection", 0.7);
                self.chest_lift = offset("humanoid.chestLift", 0.7);
                // The same tighter sigma and for the same reason: these two say
                // where a chest's volume sits rather than how much of it there
                // is, and a shape axis drawn as wide as its own size axis
                // caricatures about every third seed. Their streams are new
                // names, so every stored seed's other axes are untouched — the
                // property `adding_an_axis_does_not_move_the_axes_beside_it`
                // holds and `GENERATOR_VERSION` does not move (#289).
                self.chest_spacing = offset("humanoid.chestSpacing", 0.7);
                self.chest_fullness = offset("humanoid.chestFullness", 0.7);
            }
            Category::Frame => {
                self.shoulder_width = offset("humanoid.shoulderWidth", 1.0);
                self.hip_width = offset("humanoid.hipWidth", 1.0);
            }
            Category::Proportions => {
                self.limb_length = offset("humanoid.limbLength", 1.0);
                self.neck_length = offset("humanoid.neckLength", 1.0);
                // Joined the proportions from `Features` in #53. The stream
                // name is unchanged, so which stream this axis draws from has
                // not moved — only which lock holds it, and now how wide.
                self.extremity_size = offset("humanoid.extremitySize", 1.0);
            }
            Category::Head => {
                self.head_size = offset("humanoid.headSize", 1.0);
                self.head_breadth = offset("humanoid.headBreadth", 0.7);
                self.face_length = offset("humanoid.faceLength", 0.7);
            }
            // Nothing on the body plan is a colour, a hair, an age or a
            // garment.
            Category::Colouring | Category::Hair | Category::Age | Category::Outfit => {}
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        // Version-4 bytes span each axis's exploration envelope — the same
        // constants `sanitize` clamps with, so the code and the clamp cannot
        // disagree (#160). Height stays in millimetres, which already covers
        // the envelope.
        //
        // **The two reserved bytes are gone, which is what makes this version
        // 6** (#169). #164 retired `build` and `muscle` into `mass` and
        // `bodyFat` and held their slots so codes minted before the retirement
        // kept decoding at the right offsets; this is the versioning pass that
        // collects that removal, so the slots go and `decode_reserved` reads
        // the layout that had them.
        put_length(out, self.height);
        put_span(out, self.shoulder_width, signed_envelope());
        put_span(out, self.hip_width, signed_envelope());
        put_span(out, self.limb_length, signed_envelope());
        put_span(out, self.neck_length, signed_envelope());
        put_span(out, self.head_size, signed_envelope());
        put_span(out, self.head_breadth, signed_envelope());
        put_span(out, self.face_length, signed_envelope());
        put_span(out, self.extremity_size, signed_envelope());
        // **Appended, which is what makes this version 8** (#273). Three axes
        // at the end of the payload rather than beside the ones they belong
        // with: a byte stream has no field names, so inserting anywhere but the
        // end would move every axis after the insertion and silently re-read an
        // old code as a different body — the trap the version gate exists for.
        put_span(out, self.chest_volume, signed_envelope());
        put_span(out, self.chest_projection, signed_envelope());
        put_span(out, self.chest_lift, signed_envelope());
        // **Appended again, which is what makes this version 9** (#289), for
        // the reason the block above already gives: a byte stream has no field
        // names, so the only safe place for a new axis is the end and the
        // version gate is what stops an older code being read through this
        // layout.
        put_span(out, self.chest_spacing, signed_envelope());
        put_span(out, self.chest_fullness, signed_envelope());
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            shoulder_width: take_span(bytes, signed_envelope())?,
            hip_width: take_span(bytes, signed_envelope())?,
            limb_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            head_breadth: take_span(bytes, signed_envelope())?,
            face_length: take_span(bytes, signed_envelope())?,
            extremity_size: take_span(bytes, signed_envelope())?,
            chest_volume: take_span(bytes, signed_envelope())?,
            chest_projection: take_span(bytes, signed_envelope())?,
            chest_lift: take_span(bytes, signed_envelope())?,
            chest_spacing: take_span(bytes, signed_envelope())?,
            chest_fullness: take_span(bytes, signed_envelope())?,
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_pre_families(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Version 9 is today's layout for a person: what version 10 appends is
        // the quadruped's (#366).
        Self::decode(bytes)
    }

    fn decode_pre_distribution(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Version 8: today's layout without `chestSpacing` and `chestFullness`
        // on the end. A code minted before them described a chest sitting where
        // `torso::SPACING` put it, split the way `torso::POLES` derived — which
        // IS the neutral offset, exactly as a pre-chest code means the chest
        // its composites came to.
        let mut params = Self {
            height: take_length(bytes)?,
            shoulder_width: take_span(bytes, signed_envelope())?,
            hip_width: take_span(bytes, signed_envelope())?,
            limb_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            head_breadth: take_span(bytes, signed_envelope())?,
            face_length: take_span(bytes, signed_envelope())?,
            extremity_size: take_span(bytes, signed_envelope())?,
            chest_volume: take_span(bytes, signed_envelope())?,
            chest_projection: take_span(bytes, signed_envelope())?,
            chest_lift: take_span(bytes, signed_envelope())?,
            ..Self::default()
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_pre_chest(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Versions 6 and 7: today's layout without the three chest axes on the
        // end. A code minted before them described a body whose chest was
        // whatever its composites came to, which IS the neutral offset — not a
        // gap to be guessed at, exactly as a pre-composites code means the
        // neutral composites.
        let mut params = Self {
            height: take_length(bytes)?,
            shoulder_width: take_span(bytes, signed_envelope())?,
            hip_width: take_span(bytes, signed_envelope())?,
            limb_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            head_breadth: take_span(bytes, signed_envelope())?,
            face_length: take_span(bytes, signed_envelope())?,
            extremity_size: take_span(bytes, signed_envelope())?,
            ..Self::default()
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_reserved(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Versions 4 and 5: today's spans, with `build` and `muscle`'s slots
        // still on the wire. They are CONSUMED and discarded rather than
        // skipped — a payload is a byte stream, and every axis after them reads
        // at the wrong offset if they are not taken off it (#164, #169).
        let height = take_length(bytes)?;
        let _retired_build = take_span(bytes, signed_envelope())?;
        let _retired_muscle = take_span(bytes, signed_envelope())?;
        let mut params = Self {
            height,
            shoulder_width: take_span(bytes, signed_envelope())?,
            hip_width: take_span(bytes, signed_envelope())?,
            limb_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            head_breadth: take_span(bytes, signed_envelope())?,
            face_length: take_span(bytes, signed_envelope())?,
            extremity_size: take_span(bytes, signed_envelope())?,
            ..Self::default()
        };
        params.sanitize();
        Ok(params)
    }

    fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // The version-3 byte layout, whose bytes map ±1 and 0..1. Kept exactly
        // as it was so an old code decodes to the body it always named (#160).
        // The two retired axes still have to be CONSUMED here, discarded
        // rather than skipped: a version-3 code is a byte stream and every
        // axis after these two reads at the wrong offset if they are not
        // taken off it (#164).
        let height = take_length(bytes)?;
        let _retired_build = take_signed(bytes)?;
        let _retired_muscle = take_unit(bytes)?;
        let mut params = Self {
            height,
            shoulder_width: take_signed(bytes)?,
            hip_width: take_signed(bytes)?,
            limb_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            head_breadth: take_signed(bytes)?,
            face_length: take_signed(bytes)?,
            extremity_size: take_signed(bytes)?,
            ..Self::default()
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
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
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
        .skeleton(&crate::Composites::default());
        let tall = HumanoidParams {
            height: 2.1,
            ..Default::default()
        }
        .skeleton(&crate::Composites::default());
        let head_of = |s: &Skeleton| s.nodes[4].position.y;
        assert!(head_of(&tall) > head_of(&short) * 1.5);
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = HumanoidParams {
            height: 99.0,
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
        assert_eq!(params.head_size, 0.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn mass_thickens_the_torso_and_the_limbs_together() {
        // **What `build` used to guard, asked of the axis that replaced it**
        // (#164). The old assertion was that one factor reached both the torso
        // and a limb, which is exactly what `build` did and exactly what was
        // wrong with it — every radius moved by the same 28%. `mass` still has
        // to reach both, and the test that it does is worth keeping; what it no
        // longer asserts is that they move by the SAME amount, because they do
        // not and should not.
        let body = |mass: f32| {
            HumanoidParams::default().skeleton(&crate::Composites {
                mass,
                ..crate::Composites::default()
            })
        };
        let (slight, heavy) = (body(-1.0), body(1.0));
        // Found by zone rather than by index, so adding a node does not
        // silently retarget the assertion at some other part of the body.
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
                "{zone:?} should thicken with mass"
            );
        }
    }

    #[test]
    fn the_allometry_is_not_a_uniform_scale() {
        // The whole point of #164, as one assertion: a heavier body is heavier
        // in the places a heavier body is heavier. The waist has to outgrow the
        // wrist by a wide margin, where the axis this replaced moved the two by
        // the same 28% and could not do otherwise.
        let body = |mass: f32| {
            HumanoidParams::default().skeleton(&crate::Composites {
                mass,
                ..crate::Composites::default()
            })
        };
        let (slight, heavy) = (body(-1.0), body(1.0));
        let radius_in = |skeleton: &Skeleton, zone: Zone| {
            skeleton
                .nodes
                .iter()
                .find(|node| node.zone == zone)
                .expect("zone exists")
                .radius
        };
        let waist = radius_in(&heavy, Zone::Abdomen) / radius_in(&slight, Zone::Abdomen);
        let wrist = radius_in(&heavy, Zone::Extremity(Limb::ForeLeft))
            / radius_in(&slight, Zone::Extremity(Limb::ForeLeft));
        assert!(
            waist > wrist * 1.5,
            "the waist grows {waist:.2}x over the axis and the wrist {wrist:.2}x, which is \
             close enough to a uniform scale that the allometry is not reaching the cage"
        );
    }
}
