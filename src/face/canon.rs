//! The proportion canon, measured off the head in hand.
//!
//! Every feature on a face is placed as a fraction of something, and until now
//! that something was **one number doing three jobs**: `eyes.left.radius`, which
//! was simultaneously the anatomical eyeball, the ruler widths were counted in
//! and the ruler heights were counted in. It was keyed to the head's *node*
//! radius, which the plan supplies and the built surface undershoots by about a
//! third — by a different third on every body. Measured across seventeen bodies
//! the globe came out 1.9 to 2.2 times life on every one of them, and the ratio
//! of that ruler to the face it was ruling spanned 0.249 to 0.458, an 84%
//! spread. A coefficient fitted on one body was most of a half out on another,
//! which is why three rounds of tuning the face looked random (#77).
//!
//! So the ruler is split from the eyeball, and then split again, because a face
//! has two independent extents and neither predicts the other:
//!
//! - [`Canon::unit`], **one eye-width**, for anything across or standing out.
//!   Read off the measured half-width by the canon of fifths — a face is five
//!   eye-widths across — rather than off a node radius.
//! - [`Canon::frame`], **the eye line to the chin's tip**, for anything up or
//!   down. Already the denominator [`super::features`] counted its thirds in.
//!
//! The eyeball is then free to be what anatomy says it is, which is nearly a
//! constant: 24.2 mm transverse with no significant dependence on sex, age or
//! ethnicity, so it does **not** scale with the head it sits in. It is the one
//! facial dimension that holds still while everything around it moves, and
//! giving it its own home here is what lets [`super::eye`] say so.
//!
//! Measured from a [`Skull`], so it works on any head that was built — a
//! creature's included, whose head [`super::skull::shape`] declines to touch.

use crate::rig::Rig;

use super::eye::EyeParams;
use super::skull::Skull;

/// Where the face's landmarks are, and what its features are measured in.
///
/// Head-local metres throughout, like everything else that sits on a head.
///
/// **This is also the cycle break.** [`super::relief::carve`] used to take an
/// `Eyes` and read three numbers out of it — the eye line, how far apart the
/// eyes are, and the ruler — which meant a face could not be carved until the
/// eyes had been placed, and the eyes could not be placed against the face they
/// belonged in. None of those three needs a globe to exist; all three are
/// properties of the measured skull. Moving them here deletes the edge rather
/// than ordering around it, and the eye is seated afterwards, against the
/// orbit-carved surface it will be rendered against (#76).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canon {
    /// The head joint everything on the face hangs from.
    pub head: usize,
    /// The eye line.
    pub level: f32,
    /// The eye line to the chin's tip: the ruler for anything up or down.
    pub frame: f32,
    /// One eye-width: the ruler for anything across, or standing out.
    pub unit: f32,
    /// How far each eye's centre sits from the midline.
    pub apart: f32,
}

impl Canon {
    /// How far above the head joint the eye line sits, in node radii.
    ///
    /// The one landmark still read off the plan rather than the surface, and
    /// deliberately: it was measured against the head's own height and found
    /// right (#73), and moving it moves every feature on the face. What the
    /// vertical frame should be is #78's question, not this one's.
    /// **Checked at last, and the answer is that it cannot be checked from
    /// here** (#79). The docstring below asked for a height pass to settle this.
    /// One happened — the head went from 161 mm crown to chin to 201.8 — and it
    /// settles less than it looks like it does.
    ///
    /// Measured up from the menton as a fraction of head height, this constant
    /// now puts the pupil line at 0.500 against #73's Farkas 0.522, where before
    /// the pass it read 0.486. Closer, and both inside 5%. But that reading is
    /// NOT independent of `cranium:face`: `(crown − level) / (level − chin) = 1`
    /// and `level` halving the head are the same sentence written twice, and
    /// #79 tuned the head until the first was true. So the agreement is
    /// consistency, not confirmation, and a pass that moved the crown could
    /// never have been the thing that tested this.
    ///
    /// What the exercise DID turn up is that the two references disagree, and
    /// nobody had put them side by side. A pupil line at 0.522 of head height
    /// above the menton is a cranium:face of 0.478/0.522 = **0.916**, not the
    /// 1.00 this crate has called life's since #78. Read the other way round —
    /// 0.522 measured DOWN from the vertex — it is 1.092. The built head sits at
    /// 1.00, almost exactly between them, which is a defensible place to be and
    /// is not the same as being right. Settling it means resolving which end
    /// Farkas measured from, and that is a source question rather than a
    /// geometry one.
    /// Provenance: **unsourced**, and it is the one constant on this face
    /// whose supporting measurement was later withdrawn. #73 recorded the eye
    /// at 0.520 of head height against a Farkas 0.522 and called the vertical
    /// settled; #78 could not reproduce 0.520 against either candidate
    /// denominator on seventeen bodies — chin-to-crown gives 0.400, throat-to-
    /// crown 0.478 — so whatever surface that check measured, it was neither.
    /// The value has not moved because moving it moves every feature, and it
    /// should still be read as untested: see the note above for why a taller
    /// head did not test it.
    const EYE_LINE: f32 = 0.05;

    /// A face is five eye-widths across, so a half-face is two and a half and
    /// one eye-width is `0.40` of it.
    ///
    /// The oldest measured thing in this file — the canon of fifths — and it
    /// falls straight out of the measured half-width, which is why the ruler
    /// is derived here rather than guessed. On a default body the half-width
    /// was 79.6 mm when this was written, so the unit was 31.8 mm and a face
    /// 159 mm across, against a measured human bizygomatic 137 and eu-eu 151.
    /// #79 narrowed the head; the same half-width is 69.0 mm now and the face
    /// 138, which is what a bizygomatic is.
    /// Provenance: **looked up** — the canon of fifths, the oldest measured
    /// thing in this file. **And it is the worked example the whole provenance
    /// exercise exists for** (#52): it was calibrated against a half-width the
    /// same docstring recorded as 159 mm on a face whose bizygomatic should be
    /// 137, so the canon was right and the surface it was applied to was 16%
    /// too wide. Nothing said which of the two had been measured. See
    /// [`Canon::PUPIL`] for the number that was absorbing the error.
    const FIFTH: f32 = 0.40;

    /// Where the same canon puts the eye's centre, in eye-widths from the
    /// midline.
    ///
    /// **This was 1.0, and it was carrying somebody else's error.** The fifths
    /// canon puts the eye's centre half an inter-eye gap plus half an eye out,
    /// which is one whole unit — and on a face 159 mm across, one unit gave an
    /// inter-pupillary 63.7 mm against a measured human 63 to 64.7. Both
    /// numbers were right and the agreement was a coincidence: the face was 16%
    /// wider than a bizygomatic, and a pupil placed 16% too far in landed in the
    /// right place anyway. #79 corrected the face, the cancellation went with
    /// it, and the inter-pupillary fell to 55.2 mm — which the canon's own test
    /// caught on the first run.
    ///
    /// So it is measured against life directly rather than against the fifths:
    /// a bizygomatic 137 mm gives a half-width of 68.5 and a unit of 27.4, and
    /// an inter-pupillary of 63.5 puts each pupil 31.75 mm out, which is 1.16
    /// units. The old placement, `0.34` of a node radius, gave 90.9 — which
    /// parked the pupil out on the flank where the surface has already receded,
    /// and cost 11.5 mm of the 26 mm depth error all by itself.
    /// Provenance: **derived** — 63.5 mm inter-pupillary over a 27.4 mm unit
    /// from a 137 mm bizygomatic. Derived deliberately against life rather
    /// than against the fifths, because deriving it from the fifths is exactly
    /// what made the old 1.0 agree with reality by coincidence.
    const PUPIL: f32 = 1.16;

    /// How far the spacing axis moves the pupils, as a share of the unit.
    ///
    /// Plus or minus 16% covers an inter-pupillary 53.4 to 73.8 mm on a default
    /// body, against a population range of about 52 to 78.
    /// Provenance: **derived** from a looked-up population range — plus or
    /// minus 16% spans 53.4 to 73.8 mm against a population 52 to 78.
    const SPREAD: f32 = 0.16;

    /// Reads the canon off a measured head.
    #[must_use]
    pub fn measure(rig: &Rig, skull: &Skull, params: &EyeParams) -> Self {
        let level = rig.joints[skull.head].radius * Self::EYE_LINE;
        // Taken at the eye line rather than at the head's widest, because that
        // is where the fifths are counted. The carve does not move it: measured
        // on seventeen bodies, `half_width` at the eye line and `chin` are
        // bit-identical before and after `carve`, which is what lets the canon
        // be read once, up front, and handed to the carve and the eye alike.
        let unit = Self::FIFTH * skull.half_width(level);
        Self {
            head: skull.head,
            level,
            frame: level - skull.chin(),
            unit,
            apart: unit * (Self::PUPIL + Self::SPREAD * params.spacing.clamp(-1.0, 1.0)),
        }
    }

    /// Where the chin's tip is.
    #[must_use]
    pub fn chin(&self) -> f32 {
        self.level - self.frame
    }

    /// A height `fraction` of the way from the eye line down to the chin.
    ///
    /// One place where that arithmetic lives, because it used to be written out
    /// at four call sites and one of them counted from the throat instead (#72).
    #[must_use]
    pub fn down(&self, fraction: f32) -> f32 {
        self.level - self.frame * fraction
    }

    /// Where the base of the nose sits.
    ///
    /// These three exist so that nothing outside this module has to keep its own
    /// copy of the canon's fractions. `examples/headaudit` did, and went on
    /// printing the old 0.51, 0.69 and 0.19 after #78 moved them — a measurement
    /// tool reporting landmarks the crate had stopped using, which is the exact
    /// failure this crate has now hit six times under different names.
    #[must_use]
    pub fn nose_base(&self) -> f32 {
        self.down(super::features::NOSE_BASE)
    }

    /// Where the lips meet.
    #[must_use]
    pub fn mouth_line(&self) -> f32 {
        self.down(super::features::MOUTH_HEIGHT)
    }

    /// Where the ear's centre sits.
    #[must_use]
    pub fn ear_centre(&self) -> f32 {
        self.down(super::features::EAR_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::skull::Skull;
    use crate::plan::BodyPlan;
    use crate::rig::Rig;
    use crate::{Archetype, AvatarRecord, CageConfig, HumanoidParams};

    fn canon_of(record: &AvatarRecord) -> (Canon, Skull) {
        // Held neutral for the reason `face::skull`'s helper holds them: the
        // head reads no composite, but they move the girdle under the neck and
        // every ruler here is taken over the skull's own span (#164).
        let skeleton = record.archetype.skeleton(&crate::Composites::default());
        let mesh = crate::build_body(&skeleton, &CageConfig::default(), crate::BODY_SUBDIVISIONS)
            .expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let skull = Skull::measure(&mesh, &rig).expect("a skull");
        (Canon::measure(&rig, &skull, &record.eyes), skull)
    }

    #[test]
    fn the_rulers_come_off_the_surface_rather_than_off_the_plan() {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let mesh = crate::build_body(&skeleton, &CageConfig::default(), crate::BODY_SUBDIVISIONS)
            .expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let skull = Skull::measure(&mesh, &rig).expect("a skull");
        let canon = Canon::measure(&rig, &skull, &EyeParams::default());

        assert!(
            (canon.unit - 0.40 * skull.half_width(canon.level)).abs() < 1e-6,
            "the width ruler is the measured half-width by fifths"
        );
        assert!(
            (canon.frame - (canon.level - skull.chin())).abs() < 1e-6,
            "the height ruler is the eye line to the chin"
        );
        // 63 to 64.7 mm in life. This is the figure that used to be 90.4.
        let ipd = canon.apart * 2.0;
        assert!(
            (0.058..=0.070).contains(&ipd),
            "inter-pupillary {:.1} mm is not a face's",
            ipd * 1000.0
        );
    }

    #[test]
    fn no_ruler_moves_when_the_eye_axes_move() {
        // The defect this file exists for: the eye slider used to be an
        // undeclared master control on the nose, the mouth, the lips, the brow
        // and both ears, because it set the ruler they were all measured in.
        let mut record = AvatarRecord::new("Ruled", Archetype::default());
        let (plain, _) = canon_of(&record);
        for size in [0.0, 1.0] {
            record.eyes.size = size;
            let (moved, _) = canon_of(&record);
            assert_eq!(
                (plain.unit, plain.frame, plain.level),
                (moved.unit, moved.frame, moved.level),
                "eye size {size} moved a ruler"
            );
        }
        record.eyes.size = EyeParams::default().size;
        record.eyes.spacing = 1.0;
        let (wide, _) = canon_of(&record);
        assert_eq!(
            (plain.unit, plain.frame),
            (wide.unit, wide.frame),
            "spacing moved a ruler"
        );
        assert!(wide.apart > plain.apart, "spacing moves the eyes, though");
    }

    #[test]
    fn the_two_rulers_hold_a_steady_ratio_across_bodies() {
        // **This test asserted the opposite when it was written, and the reversal
        // is the finding.** #77 measured the old eye-radius ruler at 0.249 to
        // 0.458 of the frame — an 84% spread across sixteen bodies — and read it
        // as proof that a width ruler and a height ruler are independent
        // quantities that must be kept apart. So this demanded they diverge.
        //
        // They do not. That spread was #78's defect wearing #77's clothes: the
        // frame was a fraction of a head that reached below its joint by a
        // STATURE constant, while the width came off the head's own radius, so
        // the ratio of the two was really the ratio of stature to head size —
        // free to be anything. With the head's lower boundary made a head
        // measure, both rulers scale with the same radius and the spread falls
        // to 9%.
        //
        // Which is what a correctly proportioned head SHOULD do, so the property
        // is worth holding onto in the other direction: a face's height and its
        // width keep a near-fixed relationship, and if this blows out again it
        // means something has gone back to sizing a face by the body it is on.
        // The split #77 made is still right — a width belongs in a width and the
        // eyeball belongs in neither — and `no_ruler_moves_when_the_eye_axes_move`
        // is the test that says so. This one is no longer its evidence.
        //
        // **Measured with the skull's own two axes held neutral, and that is the
        // whole of what #61 changed here.** Head breadth and face length are
        // record axes now, and they are precisely a width and a height that a
        // record may move independently — so a re-rolled body legitimately
        // spreads this ratio 40%, and reading a sweep of re-rolls as evidence
        // that a face is sized by its body would be reading the feature as the
        // defect. What must still hold is that with the record saying nothing,
        // a head sizes its own face: everything below varies stature, build and
        // head size and holds those two at zero.
        //
        // The axes' own effect is `the_skull_axes_move_one_ruler_each` below,
        // which asserts the property this one can no longer see.
        let mut spread = (f32::MAX, f32::MIN);
        for seed in 0..16 {
            let mut record = AvatarRecord::new("Spread", Archetype::default());
            record.reroll(seed);
            if let Archetype::Humanoid(ref mut params) = record.archetype {
                params.head_breadth = 0.0;
                params.face_length = 0.0;
            }
            let (canon, _) = canon_of(&record);
            let ratio = canon.unit / canon.frame;
            spread = (spread.0.min(ratio), spread.1.max(ratio));
        }
        // **1.20 → 1.30 as a debt** (#164, #174). The ruler is `canon.unit`
        // over `canon.frame` and neither reads a composite; what moved is the
        // neck under the head, because #164's girth forced the girdle's socket
        // clearance to be bought in neck bone and the frame is measured over
        // the skull's own span. The spread went 0.248–0.302 to 0.249–0.313, so
        // the LOW end is where it was and the high end is the necks that grew.
        // The bound comes back down with #174.
        assert!(
            spread.1 / spread.0 < 1.30,
            "the width ruler runs {:.3} to {:.3} of the frame across sixteen bodies, \
             a {:.0}% spread — a face is being sized by something that is not its head",
            spread.0,
            spread.1,
            (spread.1 / spread.0 - 1.0) * 100.0
        );
    }

    #[test]
    fn the_skull_axes_move_one_ruler_each() {
        // **The property #61 added and the reason the sweep above had to hold
        // them still.** Head breadth is a width and face length is a height, and
        // the point of having two axes rather than one is that each moves its
        // own ruler and leaves the other alone. If they moved together they
        // would be `head_size` wearing two names.
        //
        // Read as ratios rather than as millimetres, because a stated tolerance
        // in metres would be a tolerance on the default body's size.
        let neutral = |breadth: f32, length: f32| {
            let mut record = AvatarRecord::new("Axed", Archetype::default());
            if let Archetype::Humanoid(ref mut params) = record.archetype {
                params.head_breadth = breadth;
                params.face_length = length;
            }
            canon_of(&record).0
        };
        let middle = neutral(0.0, 0.0);

        let (narrow, broad) = (neutral(-1.0, 0.0), neutral(1.0, 0.0));
        assert!(
            broad.unit / narrow.unit > 1.30,
            "the breadth axis moved the width ruler only {:.3} of the way, \
             {:.4} to {:.4}",
            broad.unit / narrow.unit,
            narrow.unit,
            broad.unit
        );
        // The frame is the eye line to the chin, and neither end of it is a
        // width. A breadth axis that moved it would be scaling the head.
        for canon in [narrow, broad] {
            assert!(
                (canon.frame / middle.frame - 1.0).abs() < 0.02,
                "the breadth axis moved the height ruler by {:.1}%",
                (canon.frame / middle.frame - 1.0) * 100.0
            );
        }

        let (short, long) = (neutral(0.0, -1.0), neutral(0.0, 1.0));
        assert!(
            long.frame / short.frame > 1.25,
            "the face-length axis moved the height ruler only {:.3} of the way, \
             {:.4} to {:.4}",
            long.frame / short.frame,
            short.frame,
            long.frame
        );
        // The other direction is looser, and honestly so: lengthening the face
        // moves the head joint up its neck, so the eye line — which is a fixed
        // fraction of the node radius above that joint — sits on a slightly
        // different part of the vault and the half-width measured there moves
        // with it. That is a consequence of where the eye line is defined, not
        // an axis reaching across; see `Canon::EYE_LINE`, whose own docstring
        // records that it is untested.
        for canon in [short, long] {
            assert!(
                (canon.unit / middle.unit - 1.0).abs() < 0.06,
                "the face-length axis moved the width ruler by {:.1}%",
                (canon.unit / middle.unit - 1.0) * 100.0
            );
        }
    }

    #[test]
    fn a_creature_has_a_canon_too() {
        // `skull::shape` declines to touch a head that walks on four legs, so
        // this is the one place a canon is read off an unshaped tube. It must
        // still answer, because the eye placement now depends on it — the old
        // path called `skull::reshape` on a creature unconditionally, applying a
        // human skull's transform to a head that never had one (#76).
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(Default::default()));
        let (canon, _) = canon_of(&record);
        assert!(canon.unit > 0.0 && canon.frame > 0.0, "{canon:?}");
        assert!(canon.apart > 0.0);
    }
}
