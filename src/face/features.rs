//! Ears.
//!
//! This file used to build a nose, two brows and two lips as well, as separate
//! closed solids appended to the body. They are displacements of the head's own
//! surface now — see [`super::relief`] — because a feature that is a solid laid
//! on a surface has a boundary, and a boundary is a shading seam, a UV seam, and
//! a rigid piece half-buried in skin that bends around it (#59).
//!
//! An ear did not go with them, and that is a judgement rather than an
//! oversight. A nose is a swelling of the face and an ear is a separate
//! structure standing off the side of the head with a hollow in it; a
//! displacement field over the skull cannot make one without the skull already
//! having somewhere to put it. It is conformed to the measured surface instead
//! (#67), which is the same relationship by another route.
//!
//! Placed from the published proportion canons rather than by eye, because those
//! canons are exactly the kind of thing that is cheap to look up and expensive to
//! rediscover by tweaking coefficients:
//!
//! - **Vertically, thirds.** Hairline to brow, brow to the base of the nose,
//!   base of the nose to chin, each about a third of the face.
//! - **Horizontally, fifths.** A face is about five eye-widths across, and the
//!   gap between the eyes is one of them. A nose is about one eye-width wide at
//!   the nostrils and a mouth about one and a half.
//!
//! Everything is anchored to the eyes, which are already placed, rather than
//! re-derived from the skull. Two features that each independently approximate
//! where the face is will not agree with each other, and a nose a few millimetres
//! off the eye line reads as a broken face rather than an unusual one.
//!
//! Attached parts, like the eyes and the hair, and for the same reason: a head
//! from the body plan is a smooth blob with no nose to pull out of it.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::prim;

use super::canon::Canon;
use super::skull::Skull;

/// How prominent each feature is, and how wide.
///
/// Stored as scaled integers like every other parameter block here: the AT
/// Protocol data model has no floating-point type, and a record that writes one
/// is a record other readers cannot round-trip.
///
/// **Four of these say how far a feature stands OUT and two say how far it
/// reaches ACROSS, and until #61 the second pair were hard constants.** A nose's
/// width across the wings and a mouth's width to the corners were fixed
/// multiples of [`Canon::unit`], so every seed had the same nose seen end-on and
/// the same mouth seen straight on however prominent either was. Those are two
/// of the loudest differences between two faces and neither was in the record.
///
/// The skull's own breadth and the face's length are NOT here: they are
/// [`crate::plan::HumanoidParams`] axes, because they are built into the cage
/// rather than carved into it. See `HEAD_BREADTH_SPAN` there for why.
///
/// **All six fall under the `Features` lock, which already covers six
/// categories of thing.** That bit now means skin, eyes, face, hair, head size
/// and extremity size, and these do not make it worse — they are the same kind
/// of choice as the four beside them. Whether a face deserves a lock of its own
/// is #53's question, and answering it here by adding a seventh category ad hoc
/// is how a lock set stops meaning anything.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FaceParams {
    /// How far the nose stands out, `0` flat and `1` prominent.
    #[serde(with = "crate::plan::scaled")]
    pub nose: f32,
    /// How wide the nose is across the wings, `0` narrow and `1` broad.
    ///
    /// Independent of [`Self::nose`], which is how far it stands off the face.
    /// A narrow prominent nose and a broad flat one are different faces and the
    /// record could say neither.
    #[serde(with = "crate::plan::scaled")]
    pub nose_width: f32,
    /// How heavy the brow ridge is.
    #[serde(with = "crate::plan::scaled")]
    pub brow: f32,
    /// How full the lips are.
    #[serde(with = "crate::plan::scaled")]
    pub mouth: f32,
    /// How wide the mouth is, corner to corner.
    ///
    /// **Split out of [`Self::mouth`] rather than added beside it.** Fullness
    /// used to widen the mouth as well as plump it, at exactly the gain this
    /// axis now carries — so the default face is unchanged and what moved is
    /// that a wide thin mouth and a small full one are now two records instead
    /// of one impossible one.
    #[serde(with = "crate::plan::scaled")]
    pub mouth_width: f32,
    /// How far the ears stand out from the head.
    #[serde(with = "crate::plan::scaled")]
    pub ears: f32,
}

impl Default for FaceParams {
    fn default() -> Self {
        Self {
            nose: 0.5,
            nose_width: 0.5,
            brow: 0.5,
            mouth: 0.5,
            mouth_width: 0.5,
            ears: 0.5,
        }
    }
}

impl FaceParams {
    /// Clamps every axis into range. Idempotent.
    ///
    /// **Through `crate::plan::sanitize_axis` rather than a second copy of the
    /// rule.** This carried its own substitute-then-clamp, and it differed from
    /// the shared one in exactly the case the shared one documents: it
    /// substituted for `NaN` alone, so an infinity clamped to the near bound and
    /// a mouth arrived at zero width rather than at its default. The fallbacks
    /// come from `Default` for the same reason they do there — a written-out
    /// fallback can drift from the documented default and this one cannot.
    pub fn sanitize(&mut self) {
        let default = Self::default();
        for (axis, fallback) in [
            (&mut self.nose, default.nose),
            (&mut self.nose_width, default.nose_width),
            (&mut self.brow, default.brow),
            (&mut self.mouth, default.mouth),
            (&mut self.mouth_width, default.mouth_width),
            (&mut self.ears, default.ears),
        ] {
            // Quantised as well as clamped, so a record equals itself after a
            // round trip through the thousandths the wire carries.
            *axis = crate::plan::sanitize_axis(*axis, fallback, (0.0, 1.0));
        }
    }
}

/// Where each feature sits between the eye line and the **chin's tip**, as a
/// fraction of that span on the measured head.
///
/// **These are the canon's figures now, and until #78 they were not.** They were
/// 0.19, 0.51 and 0.69, each about 0.09 too low as a fraction — and each landing
/// near its correct height in MILLIMETRES anyway, because the frame they were
/// fractions of was itself 39% short. Two errors cancelling, which is why they
/// survived being checked against a render three times, and why #78 required
/// them to move in the same commit that fixes the frame: correcting either one
/// alone makes the face worse than leaving both.
///
/// Against a 115 mm pupil-to-menton frame, Farkas puts subnasale 45.3 mm below
/// the pupil (0.394) and stomion 68.5 (0.596). The ear's is its CENTRE, half way
/// between the brow line and the base of the nose, which on the same frame is
/// 0.110 — it was 0.19, and only read correctly because a short frame turned it
/// into the right number of millimetres.
pub(super) const EAR_HEIGHT: f32 = 0.110;
/// See [`EAR_HEIGHT`]. The base of the nose, where the nostrils are.
pub(super) const NOSE_BASE: f32 = 0.394;
/// See [`EAR_HEIGHT`]. Placed by eye the mouth drifts onto the chin and the face
/// reads as a muzzle.
pub(super) const MOUTH_HEIGHT: f32 = 0.596;

/// The parts of one face that are not the head's own surface, in head-local
/// space.
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    /// One per side.
    pub ears: Vec<PolyMesh>,
    /// The head joint everything hangs from.
    pub head: usize,
}

impl Features {
    /// Builds the ears for a face whose proportions have been measured.
    ///
    /// `skull` is the head as it was actually built — **carved**, since an ear
    /// is conformed to the surface it sits against. Every landmark here is
    /// anchored to a measurement rather than to the sphere the plan asked for,
    /// because the two differ by a third and the difference is not constant:
    /// parts placed against the plan sat proud on one body and buried on the
    /// next.
    #[must_use]
    pub fn build(canon: &Canon, skull: &Skull, params: &FaceParams) -> Self {
        // Built once and mirrored. Building each side from its own signed
        // arithmetic looks equivalent and is not: it leaves the two halves
        // disagreeing by the width of a segment even when the maths is right.
        let ear = ear(canon, skull, canon.down(EAR_HEIGHT), params.ears);
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));

        Self {
            ears: vec![ear.transformed(flip), ear],
            head: canon.head,
        }
    }

    /// Every feature's mesh, in a fixed order.
    ///
    /// The order is the contract: whatever asks these for their size assigns
    /// them their atlas regions through [`Self::meshes_mut`], and the two only
    /// agree because they are the same walk. Two hand-written lists would drift
    /// the first time a feature was added.
    pub fn meshes(&self) -> impl Iterator<Item = &PolyMesh> {
        self.ears.iter()
    }

    /// The same walk, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        self.ears.iter_mut()
    }

    /// Every feature as one mesh.
    #[must_use]
    pub fn assembled(&self) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for part in self.meshes() {
            mesh.append(part);
        }
        mesh
    }
}

/// One ear, on the side of the head.
///
/// Between the brow line and the base of the nose, which is where a real one
/// sits — people place them too low from memory.
///
/// Placed by **measuring the shell it just built**. Two things made that
/// necessary. The head's true half-width is a third less than the planned
/// radius the old placement was derived from; and the shell is built around
/// `+Y` and turned by a quarter turn about `Z`, which carries `+Y` to `-X` —
/// so the ear pointed *into* the head, and its body sat inward of wherever its
/// origin was put. Both errors ran the same way, which is why an ear was buried
/// on every seed rather than on some of them.
fn ear(canon: &Canon, skull: &Skull, seat: f32, stand: f32) -> PolyMesh {
    /// How far behind the midline the ear sits, in eye-widths.
    ///
    /// A depth, so counted in the width ruler. The figure is the old eye-radius
    /// one rebased by 0.7423 (#77), which holds a default body's ear exactly
    /// where it was — it was validated on screen there (#67) and the change of
    /// ruler is not a reason to move it.
    const BACK: f32 = 0.2598;
    /// How much of the ear's own depth is buried in the head.
    ///
    /// **A fraction of the ear, not of an eye.** This was 0.30 eye-radii, and an
    /// ear is 0.56 of one thick — so more than half of it was inside the head
    /// before the skull's curvature was considered at all, which left nothing in
    /// hand when the measurement it was seated from moved (#67). A quarter is
    /// enough to read as attached rather than stuck on, and it now scales with
    /// the ear: a flatter ear sinks less deep, which is what "attached" means.
    const SINK: f32 = 0.25;
    /// How wide the lobe is against the helix above it.
    ///
    /// **An ear is not symmetric about its middle and this one was.** A cap
    /// scaled into an oval is the same shape at the top as at the bottom, and
    /// what stands furthest out is its centre — so on a head that narrows
    /// downward, the part that clears the skull and gets seen is the BOTTOM
    /// half, tapering to a point. It read as a fin hanging off the jaw. Nearly
    /// half again: the helix is broad and stands proud, the lobe is small and
    /// tucks in, and that asymmetry is most of what says which way up an ear is.
    const LOBE: f32 = 0.45;
    /// How far the ear turns to face forward, in radians.
    const FACING: f32 = 0.28;

    // An ear's long axis is vertical, so its size is a HEIGHT and belongs in the
    // frame; how far it stands off the head is a depth and belongs in the unit.
    // The two used to be the same number, and that number was the eyeball.
    //
    // **Down from 0.4520, because the ear was half again too big** (#90). This
    // is the cap's RADIUS rather than the ear's height — the built shell spans
    // about 1.83 of it — so the name flattered it: at 0.4520 the ear measured
    // 89.5 mm tall on a 214.2 mm head, a ratio of 0.418 against life's 0.267 to
    // 0.289. That is 49% oversize, and an ear half again too big reads as an elf
    // ear on its own, before anything about its outline is considered. #90 named
    // the point and not the size; the size was the larger error of the two.
    let height = canon.frame * 0.3030;
    let shell = prim::cap_shell(height, height * (0.22 + 0.16 * stand), 1.15, 3, 10);

    // Turned so the hollow faces OUT. `cap_shell` domes around `+Y`, and the
    // quarter turn that carries `+Y` to `+X` points that dome away from the
    // head — which is a smooth convex bump, an ear with the concha on the
    // inside. The positive quarter turn puts the pole against the skull and
    // leaves the rim standing proud, which is the way round an ear is.
    //
    // Then a yaw, which is what the rotation about the cap's own `X` becomes
    // once that axis has been carried to vertical: it turns the opening toward
    // the front of the head. It used to run the other way and aim the ear
    // behind the listener.
    let turn = Quat::from_rotation_y(-FACING) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    // Flattened along its own pole into a dish. At full height the cap's pole is
    // the outermost point of the ear and it reads as a cone — which is what a
    // sphere cap is.
    let oriented =
        shell.transformed(Mat4::from_quat(turn) * Mat4::from_scale(Vec3::new(1.0, 0.42, 0.62)));

    // Now sit it against the head — **every vertex against the surface under
    // that vertex**, not the whole ear against one number.
    //
    // Seated on a plane, the ear stood 19.5 mm off the head at its bottom and
    // 0.0 mm at its top, measured (#67). Nothing about the ear caused that: a
    // head narrows by 26 mm over the 62 mm an ear spans, so a rigid dish laid
    // on one is flush at the temple and hanging off the jaw. It read as a fin,
    // which is the right thing to call an ear whose only visible part is its
    // bottom half.
    //
    // Conforming it makes the stand-off the EAR'S OWN shape instead: the rim
    // proud, the pole sunk, the lobe tucked in by its taper. Which is what
    // decides whether it reads as an ear.
    //
    // The depth axis earns its keep here too. An ear sits behind the cheekbone,
    // and `half_width` is the widest the head gets anywhere in a band of
    // heights — which at the ear line *is* the cheekbone, several millimetres
    // in front of the ear and a couple of millimetres wider.
    let back = -canon.unit * BACK;
    let mut placed = taper(&oriented, LOBE);
    let (near, far) = placed.bounds();
    let sink = near.x + (far.x - near.x) * SINK;
    for point in &mut placed.positions {
        let (up, deep) = (seat + point.y, back + point.z);
        *point = Vec3::new(point.x + skull.width_across(up, deep) - sink, up, deep);
    }
    placed
}

/// Narrows a shape toward its own base, leaving its top alone.
///
/// Not a matrix: a taper is not affine, and doing it with a shear instead
/// leaves the base the same size and merely leans it. Both the fore-aft extent
/// and the stand-off from the head shrink, so the narrow end tucks against the
/// surface rather than hanging off it in mid-air.
fn taper(mesh: &PolyMesh, base: f32) -> PolyMesh {
    let (near, far) = mesh.bounds();
    let rise = (far.y - near.y).max(f32::EPSILON);
    let middle = (near.z + far.z) * 0.5;

    let mut tapered = mesh.clone();
    for point in &mut tapered.positions {
        let along = (point.y - near.y) / rise;
        let scale = base + (1.0 - base) * along;
        point.x = near.x + (point.x - near.x) * scale;
        point.z = middle + (point.z - middle) * scale;
    }
    tapered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::EyeParams;
    use crate::rig::Rig;
    use crate::{HumanoidParams, plan::BodyPlan};

    fn face(params: &FaceParams) -> (Features, Canon) {
        let (canon, _, skull) = built();
        (Features::build(&canon, &skull, params), canon)
    }

    /// A default body's canon and measured skull.
    ///
    /// Built, not planned: the whole point of these parts is that they are
    /// placed against the head the body actually grew.
    fn built() -> (Canon, PolyMesh, Skull) {
        let skeleton = HumanoidParams::default().skeleton();
        let mesh = crate::build_body(
            &skeleton,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
        )
        .expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let skull = Skull::measure(&mesh, &rig).expect("a humanoid has a skull");
        let canon = Canon::measure(&rig, &skull, &EyeParams::default());
        (canon, mesh, skull)
    }

    #[test]
    fn a_face_gets_an_ear_on_each_side() {
        let (features, _) = face(&FaceParams::default());
        assert_eq!(features.ears.len(), 2);
        assert!(features.assembled().face_count() > 40);
    }

    #[test]
    fn an_ear_is_the_size_of_an_ear() {
        // #90 named the ear's outline — a cap has an apex, so the helix comes to
        // a point — and the SIZE turned out to be the larger error. Nothing
        // measured it, because the constant that sets it is the shell's radius
        // rather than the ear's height, and the built shell spans about 1.83 of
        // that: a name that flattered the number by nearly two to one.
        //
        // Against the head the ear is on, not against the frame, because that is
        // the comparison an eye makes and the one life is quoted in: an ear runs
        // 60 to 65 mm on a head about 225 mm tall.
        let (canon, _, skull) = built();
        let features = Features::build(&canon, &skull, &FaceParams::default());
        let head = skull.throat_and_crown().1 - skull.chin();
        for ear in &features.ears {
            let (lo, hi) = ear.bounds();
            let share = (hi.y - lo.y) / head;
            assert!(
                (0.24..=0.33).contains(&share),
                "an ear spanning {:.1} mm on a {:.1} mm head is {share:.3} of it,                  against life's 0.267 to 0.289; it was 0.418 before #90",
                (hi.y - lo.y) * 1000.0,
                head * 1000.0
            );
        }
    }

    #[test]
    fn every_ear_is_a_closed_solid() {
        let (features, _) = face(&FaceParams::default());
        for ear in &features.ears {
            assert!(
                ear.is_closed_manifold(),
                "an ear: {:?}",
                ear.manifold_report()
            );
        }
    }

    #[test]
    fn the_ears_sit_between_the_brow_and_the_base_of_the_nose() {
        // People place them too low from memory, which is a good reason to
        // assert where they go — and this test used to say so while checking
        // something much weaker: that an ear was below the brow MESH and above
        // the bottom of the lip. It passed with the ear four millimetres above
        // the eye line and ten below the base of the nose (#67), which is a
        // whole ear's worth of wrong in the direction the comment warns about.
        //
        // Both landmarks are now derived the way `super::relief` derives them,
        // since neither is a mesh any more.
        let (features, canon) = face(&FaceParams::default());
        // Both landmarks in the ruler each belongs to, which is what #77 split:
        // the brow's rise is a height and so counted in the frame, and the
        // tolerance below is a height too.
        let brow = canon.level + canon.frame * (0.2076 + 0.1004 * FaceParams::default().brow);
        let nose = canon.down(NOSE_BASE);
        let slack = canon.frame * 0.2;

        for ear in &features.ears {
            let (lo, hi) = ear.bounds();
            assert!(
                (hi.y - brow).abs() < slack,
                "the ear's top is {:.2} frames from the brow line",
                (hi.y - brow) / canon.frame
            );
            assert!(
                (lo.y - nose).abs() < slack,
                "the ear's bottom is {:.2} frames from the base of the nose",
                (lo.y - nose) / canon.frame
            );
        }
    }

    #[test]
    fn an_ear_follows_the_head_instead_of_lying_on_a_plane() {
        // What decides whether an ear reads as an ear rather than as a fin.
        // Seated against one number, the ear stood 19.5 mm off the head at its
        // bottom and 0.0 mm at its top, because a head narrows by 26 mm over the
        // 62 mm an ear spans (#67). Conformed, the stand-off is the ear's own
        // shape, so it is roughly even all the way up.
        let (features, _) = face(&FaceParams::default());
        let skull = built().2;
        let ear = &features.ears[1];
        let (lo, hi) = ear.bounds();

        let stands = |at: f32| {
            let y = lo.y + (hi.y - lo.y) * at;
            let slice = (hi.y - lo.y) / 12.0;
            ear.positions
                .iter()
                .filter(|point| (point.y - y).abs() < slice)
                .map(|point| point.x - skull.width_across(point.y, point.z))
                .fold(f32::MIN, f32::max)
        };
        // Directional, not symmetric: the taper MEANS to leave the lobe closer
        // in than the helix, so demanding one ratio would be asserting against
        // the design. What must never come back is the ordering — flush at the
        // top and standing off at the bottom, which is an ear upside down.
        let (low, high) = (stands(0.1), stands(0.9));
        assert!(
            low > 0.0 && high > 0.0,
            "the ear is inside the head somewhere: {low} at the lobe, {high} at the helix"
        );
        assert!(
            high >= low && high < low * 4.0,
            "the ear stands {low} proud at the lobe and {high} at the helix"
        );
    }

    #[test]
    fn the_ears_are_a_mirrored_pair() {
        let (features, _) = face(&FaceParams::default());
        let (lo, hi) = features.assembled().bounds();
        assert!(
            (hi.x + lo.x).abs() < 1e-4,
            "the ears spanned {lo:?} to {hi:?}"
        );
    }

    #[test]
    fn a_face_survives_a_round_trip_through_json() {
        let params = FaceParams::default();
        let text = serde_json::to_string(&params).expect("serialises");
        assert_eq!(
            params,
            serde_json::from_str::<FaceParams>(&text).expect("deserialises")
        );
        // Thousandths, not floating point: the wire format has no floats.
        assert!(text.contains("500"), "{text}");
        assert!(!text.contains('.'), "{text}");
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = FaceParams {
            nose: 4.0,
            nose_width: -0.5,
            brow: -2.0,
            mouth: f32::NAN,
            mouth_width: f32::NEG_INFINITY,
            ears: 3.0,
        };
        params.sanitize();
        assert_eq!(params.nose, 1.0);
        assert_eq!(params.nose_width, 0.0);
        assert_eq!(params.brow, 0.0);
        assert_eq!(params.ears, 1.0);
        // **A non-finite value takes the DEFAULT, not the near bound**, and this
        // file used to do the second for infinities and the first for `NaN`
        // alone. A slider cannot produce an infinity; an arithmetic accident
        // upstream can, and answering it with a mouth of zero width is a worse
        // guess than answering with a neutral one. See
        // [`crate::plan::sanitize_axis`], which is where the rule lives now.
        assert_eq!(params.mouth, 0.5);
        assert_eq!(params.mouth_width, 0.5);

        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }
}
