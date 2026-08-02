//! A nose, brows, a mouth and ears.
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

use glam::{Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::prim;

use super::eye::Eyes;
use super::skull::Skull;

/// How prominent each feature is.
///
/// Stored as scaled integers like every other parameter block here: the AT
/// Protocol data model has no floating-point type, and a record that writes one
/// is a record other readers cannot round-trip.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FaceParams {
    /// How far the nose stands out, `0` flat and `1` prominent.
    #[serde(with = "crate::plan::scaled")]
    pub nose: f32,
    /// How heavy the brow ridge is.
    #[serde(with = "crate::plan::scaled")]
    pub brow: f32,
    /// How full the lips are.
    #[serde(with = "crate::plan::scaled")]
    pub mouth: f32,
    /// How far the ears stand out from the head.
    #[serde(with = "crate::plan::scaled")]
    pub ears: f32,
}

impl Default for FaceParams {
    fn default() -> Self {
        Self {
            nose: 0.5,
            brow: 0.5,
            mouth: 0.5,
            ears: 0.5,
        }
    }
}

impl FaceParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        for axis in [
            &mut self.nose,
            &mut self.brow,
            &mut self.mouth,
            &mut self.ears,
        ] {
            // Quantised as well as clamped, so a record equals itself after a
            // round trip through the thousandths the wire carries.
            *axis = quantize(if axis.is_nan() {
                0.5
            } else {
                axis.clamp(0.0, 1.0)
            });
        }
    }
}

/// Where each feature sits between the eye line and the chin, as a fraction of
/// that span on the **measured** head.
///
/// These reproduce the face the old eye-radii figures gave on a default body —
/// which is where they came from — and unlike those figures they follow a head
/// that is shallower, or eyes that are larger, instead of walking off the
/// bottom of it.
const EAR_HEIGHT: f32 = 0.25;
/// See [`EAR_HEIGHT`]. The base of the nose, where the nostrils are.
const NOSE_BASE: f32 = 0.45;
/// See [`EAR_HEIGHT`]. Placed by eye the mouth drifts onto the chin and the face
/// reads as a muzzle.
const MOUTH_HEIGHT: f32 = 0.62;

/// The features of one face, in head-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Features {
    /// The nose.
    pub nose: PolyMesh,
    /// One ridge per eye.
    pub brows: Vec<PolyMesh>,
    /// Upper and lower lip.
    pub lips: Vec<PolyMesh>,
    /// One per side.
    pub ears: Vec<PolyMesh>,
    /// The head joint everything hangs from.
    pub head: usize,
}

impl Features {
    /// Builds a face around eyes that have already been placed.
    ///
    /// `skull` is the head as it was actually built. Every landmark here is
    /// anchored to it rather than to the sphere the plan asked for, because the
    /// two differ by a third and the difference is not constant: features placed
    /// against the plan sat proud on one body and buried on the next.
    #[must_use]
    pub fn build(eyes: &Eyes, skull: &Skull, params: &FaceParams) -> Self {
        // An eye's radius is the unit the canons are expressed in: a face is
        // five eye-widths across, and every other landmark follows from that.
        let unit = eyes.left.radius;
        let apart = eyes.right.pivot.x.abs();
        let level = eyes.left.pivot.y;
        // The bottom of the measured head. Feature HEIGHTS used to be counted in
        // eye-radii down from the eye line, which quietly assumes the eye and the
        // head are the same size — and they are separate axes. On a body with
        // large eyes in a shallow head the mouth was placed below where the head
        // has any surface at all, which is what 'buried' turned out to mean.
        // Counting them as fractions of the eye-line-to-chin span instead
        // reproduces the default face and follows every other one.
        let chin = skull.span().0;
        let down = |fraction: f32| level + (chin - level) * fraction;

        // Where the face's surface is AT EACH FEATURE'S OWN HEIGHT. One depth
        // for the whole face is not enough: the skull carries a brow ridge that
        // stands proud and a chin that projects further still, so a mouth placed
        // at the eye line's depth ends up inside the jaw beneath it. That is
        // exactly what happened when the chin was sharpened.
        //
        // Measured, not derived. This used to reshape a point on the planned
        // sphere, which overstates the built head by about a third — enough that
        // every inset below had been tuned as a *fraction* of a number that was
        // wrong, and the insets are now distances in eye-radii instead.
        let surface = |height: f32| skull.depth(height);

        // Sided features are built once and mirrored. Building each side from
        // its own signed arithmetic looks equivalent and is not: a brow arches
        // from the midline outward, and the expression that says so on the right
        // says the opposite on the left. It also leaves the two halves disagreeing
        // by the width of a segment even when the maths is right.
        let brow = brow(unit, apart, level, &surface, params.brow);
        let ear = ear(unit, skull, down(EAR_HEIGHT), params.ears);
        let flip = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));

        Self {
            nose: nose(unit, level, down(NOSE_BASE), &surface, params.nose),
            brows: vec![brow.transformed(flip), brow],
            lips: lips(unit, down(MOUTH_HEIGHT), skull, params.mouth),
            ears: vec![ear.transformed(flip), ear],
            head: eyes.head,
        }
    }

    /// Every feature's mesh, in a fixed order.
    ///
    /// The order is the contract: whatever asks these for their size assigns
    /// them their atlas regions through [`Self::meshes_mut`], and the two only
    /// agree because they are the same walk. Two hand-written lists would drift
    /// the first time a feature was added.
    pub fn meshes(&self) -> impl Iterator<Item = &PolyMesh> {
        std::iter::once(&self.nose)
            .chain(&self.brows)
            .chain(&self.lips)
            .chain(&self.ears)
    }

    /// The same walk, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        std::iter::once(&mut self.nose)
            .chain(&mut self.brows)
            .chain(&mut self.lips)
            .chain(&mut self.ears)
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

/// The nose: a wedge from between the eyes down to the nostrils.
///
/// Its root is at the eye line, not above it. The bridge starts where the brows
/// meet — put it higher and the face grows a snout.
fn nose(
    unit: f32,
    level: f32,
    base: f32,
    surface: &dyn Fn(f32) -> f32,
    prominence: f32,
) -> PolyMesh {
    let reach = unit * (0.38 + 0.30 * prominence);

    let path = [
        Vec3::new(
            0.0,
            level + unit * 0.40,
            surface(level + unit * 0.40) - unit * 0.22,
        ),
        Vec3::new(
            0.0,
            level - unit * 0.55,
            surface(level - unit * 0.55) + reach * 0.34,
        ),
        Vec3::new(
            0.0,
            base + unit * 0.55,
            surface(base + unit * 0.55) + reach * 0.80,
        ),
        Vec3::new(0.0, base, surface(base) + reach * 0.86),
        Vec3::new(
            0.0,
            base - unit * 0.16,
            surface(base - unit * 0.16) - unit * 0.24 + reach * 0.46,
        ),
    ];
    // Narrow at the bridge, widest at the nostrils, and about one eye-width
    // across there.
    let sections = [
        Vec2::new(unit * 0.22, unit * 0.22),
        Vec2::new(unit * 0.26, unit * 0.30),
        Vec2::new(unit * 0.44, unit * 0.40),
        Vec2::new(unit * 0.50, unit * 0.34),
        Vec2::new(unit * 0.38, unit * 0.15),
    ];
    // An even number of sides. An odd ring has a vertex at zero and none
    // opposite it, so a shape swept down the midline comes out very slightly
    // lopsided — which on a nose is the one place it will be noticed.
    prim::sweep(&path, &sections, 8, Vec3::X)
}

/// One brow ridge, arching over its eye.
fn brow(unit: f32, across: f32, level: f32, surface: &dyn Fn(f32) -> f32, weight: f32) -> PolyMesh {
    let rise = unit * (0.62 + 0.30 * weight);
    let thickness = unit * (0.10 + 0.10 * weight);
    let span = unit * 1.05;

    // Three points: inner end, over the pupil, outer end — the outer end sitting
    // lower and further back, which is the arch.
    let path = [
        Vec3::new(
            across - span * 0.85,
            level + rise * 0.86,
            surface(level + rise * 0.86) - unit * 0.16,
        ),
        Vec3::new(across, level + rise, surface(level + rise)),
        Vec3::new(
            across + span * 0.95,
            level + rise * 0.74,
            surface(level + rise * 0.74) - unit * 0.42,
        ),
    ];
    let sections = [
        Vec2::new(thickness * 0.7, thickness * 0.8),
        Vec2::new(thickness, thickness),
        Vec2::new(thickness * 0.55, thickness * 0.6),
    ];
    prim::sweep(&path, &sections, 6, Vec3::Y)
}

/// Upper and lower lip.
///
/// Two pieces rather than one: a mouth modelled as a single bar has no line
/// across it, and the line is the whole feature.
fn lips(unit: f32, mouth: f32, skull: &Skull, fullness: f32) -> Vec<PolyMesh> {
    let half = unit * 0.98;
    let plump = unit * (0.15 + 0.15 * fullness);

    [(1.0f32, 0.9f32), (-1.0, 1.1)]
        .iter()
        .map(|&(up, size)| {
            let lift = plump * up * 0.62;
            // Anchored ON the measured face, with the lip's own body standing
            // proud of it. Tucked back by a fraction of the surface — which is
            // how this read before the surface was measured — a mouth sits
            // inside the jaw on any body whose face is shallower than average.
            // Each end placed against the face AT ITS OWN WIDTH. A mouth
            // spans nearly two eye-widths, and over that distance the face has
            // curved back by several millimetres; anchoring the corners to the
            // midline depth is what left a third of the lip inside the jaw on
            // the worst bodies and standing off it on the best.
            let corner = skull.depth_across(mouth - plump * 0.30, half) + plump * 0.30;
            let middle = skull.depth_across(mouth + lift, half * 0.35) + plump * 0.45;
            let path = [
                Vec3::new(-half, mouth - plump * 0.30, corner),
                Vec3::new(-half * 0.35, mouth + lift, middle),
                Vec3::new(half * 0.35, mouth + lift, middle),
                Vec3::new(half, mouth - plump * 0.30, corner),
            ];
            let sections = [
                Vec2::new(plump * 0.30, plump * 0.45),
                Vec2::new(plump * size, plump * 0.95),
                Vec2::new(plump * size, plump * 0.95),
                Vec2::new(plump * 0.30, plump * 0.45),
            ];
            prim::sweep(&path, &sections, 6, Vec3::Y)
        })
        .collect()
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
fn ear(unit: f32, skull: &Skull, seat: f32, stand: f32) -> PolyMesh {
    let height = unit * 1.35;
    let shell = prim::cap_shell(height, height * (0.22 + 0.16 * stand), 1.15, 3, 10);

    // Turned to face outward — away from the head, which is the negative
    // quarter turn — and tipped back a little.
    let turn = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(-0.28);
    // Flattened along its own pole into a dish. At full height the cap's pole is
    // the outermost point of the ear and it reads as a cone — which is what a
    // sphere cap is. Nobody had seen it: pointing the wrong way, it had been
    // inside the head since it was written.
    let oriented =
        shell.transformed(Mat4::from_quat(turn) * Mat4::from_scale(Vec3::new(1.0, 0.42, 0.62)));

    // Now that the shape is known, sit it against the head: its innermost point
    // a little under the surface, so the ear is attached rather than floating,
    // and everything beyond that standing proud.
    let inner = oriented.bounds().0.x;
    let out = skull.half_width(seat) - inner - unit * 0.30;
    oriented.transformed(Mat4::from_translation(Vec3::new(out, seat, -unit * 0.35)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::EyeParams;
    use crate::rig::Rig;
    use crate::{HumanoidParams, plan::BodyPlan};

    fn face(params: &FaceParams) -> (Features, Eyes) {
        // Built, not planned: the whole point of these features is that they are
        // placed against the head the body actually grew.
        let skeleton = HumanoidParams::default().skeleton();
        let mesh = crate::build_body(&skeleton, &crate::CageConfig::default(), 2).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let eyes = Eyes::build(&rig, &EyeParams::default()).expect("a humanoid has eyes");
        let skull = Skull::measure(&mesh, &rig).expect("a humanoid has a skull");
        (Features::build(&eyes, &skull, params), eyes)
    }

    #[test]
    fn a_face_gets_every_feature() {
        let (features, _) = face(&FaceParams::default());
        assert!(features.nose.face_count() > 8);
        assert_eq!(features.brows.len(), 2);
        assert_eq!(features.lips.len(), 2);
        assert_eq!(features.ears.len(), 2);
        assert!(features.assembled().face_count() > 100);
    }

    #[test]
    fn every_feature_is_a_closed_solid() {
        let (features, _) = face(&FaceParams::default());
        let mut parts = vec![("nose", &features.nose)];
        for (index, part) in features.brows.iter().enumerate() {
            let _ = index;
            parts.push(("brow", part));
        }
        for part in &features.lips {
            parts.push(("lip", part));
        }
        for part in &features.ears {
            parts.push(("ear", part));
        }
        for (name, part) in parts {
            assert!(
                part.is_closed_manifold(),
                "{name}: {:?}",
                part.manifold_report()
            );
        }
    }

    #[test]
    fn the_features_stack_down_the_face_in_order() {
        // Brow above eye above nose above mouth. Stated because it is the one
        // thing that makes a face a face, and a sign error anywhere in the
        // placement breaks it without breaking anything else.
        let (features, eyes) = face(&FaceParams::default());
        let top = |mesh: &PolyMesh| mesh.bounds().1.y;
        let bottom = |mesh: &PolyMesh| mesh.bounds().0.y;

        assert!(bottom(&features.brows[0]) > eyes.left.pivot.y);
        assert!(bottom(&features.nose) < eyes.left.pivot.y);
        assert!(top(&features.lips[0]) < bottom(&features.nose));
    }

    #[test]
    fn the_nose_stands_out_in_front_of_the_eyes() {
        let (features, eyes) = face(&FaceParams::default());
        assert!(
            features.nose.bounds().1.z > eyes.left.pivot.z,
            "the nose sat behind the eye line"
        );
    }

    #[test]
    fn a_more_prominent_nose_stands_further_out() {
        let flat = face(&FaceParams {
            nose: 0.0,
            ..Default::default()
        })
        .0;
        let sharp = face(&FaceParams {
            nose: 1.0,
            ..Default::default()
        })
        .0;
        assert!(sharp.nose.bounds().1.z > flat.nose.bounds().1.z);
    }

    #[test]
    fn the_ears_sit_between_the_brow_and_the_base_of_the_nose() {
        // People place them too low from memory, which is a good reason to
        // assert where they go.
        let (features, _) = face(&FaceParams::default());
        for ear in &features.ears {
            let (lo, hi) = ear.bounds();
            assert!(
                hi.y < features.brows[0].bounds().1.y + 1e-3,
                "an ear rode too high"
            );
            assert!(lo.y > features.lips[0].bounds().0.y, "an ear hung too low");
        }
    }

    #[test]
    fn the_face_is_symmetric_about_the_midline() {
        let (features, _) = face(&FaceParams::default());
        let (lo, hi) = features.assembled().bounds();
        assert!(
            (hi.x + lo.x).abs() < 1e-4,
            "the face spanned {lo:?} to {hi:?}"
        );
        // The nose is on the midline.
        let nose = features.nose.bounds();
        assert!((nose.1.x + nose.0.x).abs() < 1e-5);
    }

    #[test]
    fn the_mouth_is_wider_than_the_nose() {
        // One and a half eye-widths against one, per the canon of fifths.
        let (features, _) = face(&FaceParams::default());
        let width = |mesh: &PolyMesh| {
            let (lo, hi) = mesh.bounds();
            hi.x - lo.x
        };
        assert!(width(&features.lips[0]) > width(&features.nose) * 1.2);
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
            brow: -2.0,
            mouth: f32::NAN,
            ears: f32::INFINITY,
        };
        params.sanitize();
        assert_eq!(params.nose, 1.0);
        assert_eq!(params.brow, 0.0);
        assert_eq!(params.mouth, 0.5);
        assert_eq!(params.ears, 1.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }
}
