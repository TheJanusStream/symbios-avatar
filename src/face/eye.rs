//! Eyes, and the lids that close over them.
//!
//! Eyes carry more of a face than their size suggests, and the research on
//! stylised characters is unanimous about why: it is not the shading, it is that
//! they *move*. A body that blinks and looks at things reads as inhabited; the
//! same body with painted-on eyes reads as a mannequin. So this is geometry with
//! a rotation on it, not a texture.
//!
//! Lids are **spherical shells that rotate**, upper and lower, meeting at the
//! eye's equator when shut. A face rig would deform eyelid geometry that is part
//! of the head, but a head here is a smooth blob with no eyelid to deform —
//! and a shell that rotates is both honest about that and convincing, because
//! that is very nearly what a real lid does.
//!
//! Everything is built in **head-local space**. A renderer parents the parts to
//! the head joint, which follows the body for free; nothing here needs skinning.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::prim;
use crate::rig::{Rig, landmark};

use super::canon::Canon;

/// How a body's eyes are shaped and set.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EyeParams {
    /// Eye size, `0` small and `1` large.
    ///
    /// **Not a fraction of the head's radius**, which is what it used to be and
    /// what put a globe 1.9 to 2.2 times life on every body: an eyeball is the
    /// one facial dimension that does not scale with the face around it. This
    /// stretches a near-constant anatomical globe by about a sixth either way,
    /// which is as far as a stylised eye can go before it stops being one.
    #[serde(with = "crate::plan::scaled")]
    pub size: f32,
    /// How far apart the eyes are set, `-1` close and `+1` wide.
    #[serde(with = "crate::plan::scaled")]
    pub spacing: f32,
    /// How deeply the eyes are set into the head, `-1` protruding and `+1` sunken.
    #[serde(with = "crate::plan::scaled")]
    pub depth: f32,
    /// How far open the lids rest, `0` shut and `1` wide.
    #[serde(with = "crate::plan::scaled")]
    pub aperture: f32,
}

impl Default for EyeParams {
    fn default() -> Self {
        Self {
            size: 0.5,
            spacing: 0.0,
            depth: 0.0,
            aperture: 0.8,
        }
    }
}

impl EyeParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.size = quantize(finite(self.size, 0.5).clamp(0.0, 1.0));
        self.aperture = quantize(finite(self.aperture, 0.8).clamp(0.0, 1.0));
        self.spacing = quantize(finite(self.spacing, 0.0).clamp(-1.0, 1.0));
        self.depth = quantize(finite(self.depth, 0.0).clamp(-1.0, 1.0));
    }
}

/// Substitutes `fallback` for a non-finite value.
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// One eye's parts, in head-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Eye {
    /// The eyeball, centred on [`Eye::pivot`].
    pub globe: PolyMesh,
    /// The upper lid, in its fully open position.
    pub upper_lid: PolyMesh,
    /// The lower lid, in its fully open position.
    pub lower_lid: PolyMesh,
    /// Where the eye turns about, in head-local space.
    pub pivot: Vec3,
    /// Radius of the globe.
    pub radius: f32,
    /// `-1` for the body's left eye, `+1` for its right.
    pub side: f32,
}

impl Eye {
    /// How far the lids swing between open and shut.
    ///
    /// The upper lid does most of the work, as a real one does; the lower barely
    /// moves. Splitting it evenly is the giveaway that a blink was animated by
    /// someone who did not look at one.
    const UPPER_SWING: f32 = 1.45;
    /// How far the lower lid swings.
    const LOWER_SWING: f32 = 0.45;

    /// The rotation to apply to a lid, about the eye's pivot.
    ///
    /// `closure` runs `0` for fully open to `1` for shut.
    #[must_use]
    pub fn lid_rotation(&self, closure: f32, upper: bool) -> Quat {
        let closure = closure.clamp(0.0, 1.0);
        // Positive about X carries the top of the eye forward over its front,
        // which is the direction an upper lid actually travels; the lower lid
        // starts underneath and so has to come the other way.
        let swing = if upper {
            Self::UPPER_SWING
        } else {
            -Self::LOWER_SWING
        };
        Quat::from_rotation_x(swing * closure)
    }

    /// The transform placing a lid for a given closure.
    #[must_use]
    pub fn lid_transform(&self, closure: f32, upper: bool) -> Mat4 {
        Mat4::from_translation(self.pivot)
            * Mat4::from_quat(self.lid_rotation(closure, upper))
            * Mat4::from_translation(-self.pivot)
    }

    /// The rotation turning this eye toward a point in head-local space.
    ///
    /// Clamped, because an eye that can point anywhere looks deranged rather
    /// than attentive.
    #[must_use]
    pub fn gaze_rotation(&self, target: Vec3, limit: f32) -> Quat {
        let toward = (target - self.pivot).normalize_or_zero();
        if toward == Vec3::ZERO {
            return Quat::IDENTITY;
        }
        let (axis, angle) = Quat::from_rotation_arc(landmark::FORWARD, toward).to_axis_angle();
        Quat::from_axis_angle(axis, angle.min(limit.max(0.0)))
    }

    /// Every part of this eye as one mesh, posed at the given closure.
    ///
    /// Convenient for inspection and export; a renderer keeps the parts separate
    /// so the lids can move without rebuilding anything.
    #[must_use]
    pub fn assembled(&self, closure: f32) -> PolyMesh {
        let mut mesh = self.globe.clone();
        mesh.append(
            &self
                .upper_lid
                .transformed(self.lid_transform(closure, true)),
        );
        mesh.append(
            &self
                .lower_lid
                .transformed(self.lid_transform(closure, false)),
        );
        mesh
    }
}

/// A body's pair of eyes, and where they belong.
#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
    /// The body's left eye.
    pub left: Eye,
    /// The body's right eye.
    pub right: Eye,
    /// The joint the pair is parented to.
    pub head: usize,
}

/// The globe's radius on a body of reference stature, in metres.
///
/// A measured human eyeball is 24.2 mm across the transverse axis and 23.7 front
/// to back, and it is the one facial dimension that holds still: no significant
/// dependence on sex, on age past infancy, or on ethnicity. Uniformly scaling a
/// head by 8% mis-sizes its eye by 2 mm, so an eyeball keyed to the head is
/// wrong by construction rather than by tuning — which is how this crate came to
/// carry a globe twice life size on every body it built (#77).
const GLOBE: f32 = 0.0121;

/// The measured height of the reference body's skin, in metres.
///
/// The stature the globe above belongs to, measured the way the globe is placed:
/// off the body in hand. A 1.75 m adult's *skin* spans about 1.64 m — the crown
/// of the head sits below the nominal stature — and across seventeen bodies that
/// ratio holds to within 4%.
const REFERENCE: f32 = 1.65;

/// How much of a change in stature the globe follows.
///
/// Not none, because a small body's eye really is a little smaller, and not one,
/// because it is nothing like proportional. A third takes a 1.25 m body to
/// 11.2 mm and a 2.09 m body to 13.3 against life's 12.1 — a millimetre or two
/// either way, which is what the anthropometry says the spread actually is.
const STATURE_GAIN: f32 = 0.35;

/// How far the globe's front pole stands proud of the skin at rest, in metres.
///
/// In life the corneal apex sits roughly level with the lids around it, so this
/// is small on purpose. It is also the whole of what decides how much eye shows:
/// the body is a closed surface with no opening cut for an eye, so the visible
/// part of the globe is exactly the cap that clears the skin. Three millimetres
/// on a 12.4 mm globe is a lens 16.2 mm wide, against a globe that until now
/// stood 19 to 51 mm proud on every body (#76).
const PROUD: f32 = 0.003;

/// How far the depth axis moves that, in metres.
///
/// Kept inside `tests/parts.rs`'s 5 mm ceiling at both ends of the axis, so a
/// sunken eye and a protruding one are both eyes that are seated.
const PROUD_RANGE: f32 = 0.0018;

impl Eyes {
    /// Builds a pair of eyes, seated in a head that has already been built.
    ///
    /// `mesh` is the body **as it will be rendered** — carved, since that is the
    /// surface the eye is seen against. Where the last version of this predicted
    /// the surface by warping an interior point through [`super::skull::reshape`],
    /// this bisects the real one.
    ///
    /// That is not a refinement, it is the fix: `reshape` scales `z` by a single
    /// factor with no dependence on `x`, so its answer is right on the midline
    /// (98.5 predicted against 97.1 measured) and 26.3 mm too deep at the eye's
    /// own column — a globe whose *centre* stood 6.8 mm outside the head, with
    /// 41 to 69% of its surface in the air. Nor is [`super::skull::Skull::depth_across`]
    /// the answer, which is what #76 proposed: measured against a bisection of
    /// the same column it overstates by 2.0 to 6.0 mm across seventeen bodies,
    /// which is most of a 5 mm budget. Bins are for profiles; a seat wants the
    /// surface.
    ///
    /// It also works on a head this crate does not shape. `reshape` was called
    /// unconditionally while [`super::skull::shape`] bails for anything with more
    /// than two feet, so a creature's eyes were placed by a human skull's
    /// transform that had never been applied to its head.
    #[must_use]
    pub fn build(rig: &Rig, mesh: &PolyMesh, canon: &Canon, params: &EyeParams) -> Self {
        let centre = rig.joints[canon.head].position;
        let radius = globe(mesh, params);
        let proud = PROUD - PROUD_RANGE * params.depth.clamp(-1.0, 1.0);

        // The eye's own column, not the midline: the face has curved away by
        // several millimetres by the time it reaches a pupil, and that curve is
        // exactly what the old prediction could not express.
        //
        // Falling back inward rather than outward, because a column that is not
        // inside the body at all is a head narrower than its own canon — and the
        // midline always is.
        let far = {
            let (lo, hi) = mesh.bounds();
            (hi.z - lo.z).max(f32::EPSILON)
        };
        let level = centre.y + canon.level;
        let skin = reach(
            mesh,
            Vec3::new(centre.x + canon.apart, level, centre.z),
            far,
        )
        .or_else(|| reach(mesh, Vec3::new(centre.x, level, centre.z), far))
        .or_else(|| reach(mesh, centre, far))
        .unwrap_or(0.0);

        let pivot = Vec3::new(canon.apart, canon.level, skin - radius + proud);
        Self {
            left: eye(-1.0, Vec3::new(-pivot.x, pivot.y, pivot.z), radius, params),
            right: eye(1.0, pivot, radius, params),
            head: canon.head,
        }
    }

    /// Both eyes as one mesh, posed at the given closure.
    #[must_use]
    pub fn assembled(&self, closure: f32) -> PolyMesh {
        let mut mesh = self.left.assembled(closure);
        mesh.append(&self.right.assembled(closure));
        mesh
    }
}

/// How large this body's eyeball is, in metres.
///
/// Keyed to the body's own measured height and to nothing else on the head. See
/// [`GLOBE`] for why that is the anatomy rather than a simplification.
fn globe(mesh: &PolyMesh, params: &EyeParams) -> f32 {
    let (lo, hi) = mesh.bounds();
    let stature = (hi.y - lo.y).max(f32::EPSILON);
    let grown = GLOBE * (1.0 + STATURE_GAIN * (stature / REFERENCE - 1.0));
    grown * (0.80 + 0.45 * params.size.clamp(0.0, 1.0))
}

/// How far `mesh` reaches forward from `from`, or `None` if `from` is outside it.
///
/// Bisected against [`PolyMesh::contains`], which is the same primitive
/// `tests/parts.rs` judges the result with and the one the head audit measures
/// through. Binning the frontmost vertex in a band reported six millimetres of
/// ripple that is not in the mesh, off the midline where the surface curves fast
/// across a band (#71) — and off the midline is precisely where an eye sits.
fn reach(mesh: &PolyMesh, from: Vec3, far: f32) -> Option<f32> {
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, far);
    // Thirty halvings takes any head to well under a micron.
    for _ in 0..30 {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from + Vec3::Z * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// Builds one eye at `pivot`.
fn eye(side: f32, pivot: Vec3, radius: f32, params: &EyeParams) -> Eye {
    let globe = prim::sphere(radius, 10, 14).transformed(Mat4::from_translation(pivot));

    // A lid is a shell just clear of the globe, so it never intersects it as it
    // swings. Its rest position is set by the aperture: a wide-open eye starts
    // with the lids further back.
    let shell = radius * 1.06;
    let thickness = radius * 0.10;
    let open = params.aperture.clamp(0.0, 1.0);

    let lid = |upper: bool| {
        let swing = if upper {
            Eye::UPPER_SWING
        } else {
            -Eye::LOWER_SWING
        };
        // Built around +Y then turned so the pair meet across the eye when shut.
        //
        // The sign matters and it was wrong: written as `swing * (1 - open)`
        // this CLOSES the lids as the aperture rises, and the default aperture
        // left a fifteen-degree slit — an eye that read as a letterbox with a
        // stripe of iris in it. Opening the upper lid is a negative rotation
        // about X and opening the lower one is positive, which is exactly what
        // multiplying by each lid's own swing gives.
        let rest = Quat::from_rotation_x(swing * (0.42 - 0.72 * open))
            * if upper {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_x(std::f32::consts::PI)
            };
        prim::cap_shell(shell, thickness, 1.25, 3, 14)
            .transformed(Mat4::from_translation(pivot) * Mat4::from_quat(rest))
    };

    Eye {
        globe,
        upper_lid: lid(true),
        lower_lid: lid(false),
        pivot,
        radius,
        side,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::skull::Skull;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};
    use crate::skeleton::Skeleton;

    /// A body's eyes, seated in the head it actually grew.
    ///
    /// Carved, because that is the surface the seat is bisected against — and
    /// the difference is not decorative: the orbit the brow cuts is 1.2 to
    /// 3.3 mm deep at the eye's own column on every body measured.
    fn seated(skeleton: &Skeleton, params: &EyeParams) -> Eyes {
        let rig = Rig::from_skeleton(skeleton).expect("rigs");
        let mut mesh = crate::build_body(skeleton, &crate::CageConfig::default(), 2).expect("mesh");
        let skull = Skull::measure(&mesh, &rig).expect("a skull");
        let canon = super::Canon::measure(&rig, &skull, params);
        crate::face::carve_face(&mut mesh, &rig, &canon, &Default::default());
        Eyes::build(&rig, &mesh, &canon, params)
    }

    fn eyes(params: &EyeParams) -> Eyes {
        seated(&HumanoidParams::default().skeleton(), params)
    }

    #[test]
    fn a_body_gets_two_eyes_set_in_its_face() {
        let pair = eyes(&EyeParams::default());
        assert!(pair.left.pivot.x < 0.0, "the left eye is on the left");
        assert!(pair.right.pivot.x > 0.0);
        assert!(
            pair.left.pivot.z > 0.0,
            "eyes belong on the front of a head"
        );
        assert_eq!(pair.left.pivot.x, -pair.right.pivot.x, "and are symmetric");
    }

    #[test]
    fn every_part_of_an_eye_is_a_solid() {
        let pair = eyes(&EyeParams::default());
        for (name, mesh) in [
            ("globe", &pair.left.globe),
            ("upper lid", &pair.left.upper_lid),
            ("lower lid", &pair.left.lower_lid),
        ] {
            assert!(
                mesh.is_closed_manifold(),
                "{name} is not closed: {:?}",
                mesh.manifold_report()
            );
        }
    }

    #[test]
    fn an_eyeball_barely_notices_the_head_it_is_in() {
        // **This test used to assert the opposite**, under the name
        // `eyes_scale_with_the_head_they_sit_in`: it demanded that the eye/skull
        // ratio be constant, which is the one facial dimension anatomy holds
        // constant AGAINST head size. It passed for three rounds while shipping
        // a globe 1.9 to 2.2 times life on every body (#77).
        //
        // What is measured instead is what the anthropometry actually says: an
        // eyeball follows stature weakly and head size not at all, so growing
        // the head must make the eye a SMALLER share of it.
        let of = |height: f32, head_size: f32| {
            let skeleton = HumanoidParams {
                height,
                head_size,
                ..Default::default()
            }
            .skeleton();
            let rig = Rig::from_skeleton(&skeleton).expect("rigs");
            let pair = seated(&skeleton, &EyeParams::default());
            (pair.left.radius, rig.joints[pair.head].radius)
        };

        // Head size alone, at a fixed stature: the globe must hold still.
        let (small_head, small_skull) = of(1.75, 0.0);
        let (large_head, large_skull) = of(1.75, 1.0);
        assert!(
            large_skull > small_skull * 1.2,
            "the fixture did not actually change the head: {small_skull} to {large_skull}"
        );
        assert!(
            (large_head - small_head).abs() < 0.0005,
            "the globe moved {:.2} mm when only the head grew",
            (large_head - small_head).abs() * 1000.0
        );
        assert!(
            large_head / large_skull < small_head / small_skull * 0.85,
            "a bigger head must wear its eye as a smaller share of itself"
        );

        // Stature: a millimetre or two, not a proportion.
        let (short, _) = of(1.3, 0.5);
        let (tall, _) = of(2.1, 0.5);
        assert!(tall > short, "a taller body's eye is a little larger");
        assert!(
            tall - short < 0.004,
            "stature moved the globe {:.1} mm, which is a proportion rather than a \
             millimetre or two",
            (tall - short) * 1000.0
        );
    }

    #[test]
    fn an_eyeball_is_about_the_size_of_an_eyeball() {
        // 24.2 mm across in life, on everyone. The globe used to run 34 to
        // 72 mm across depending on the body, which is what made it read as
        // goggles and what made every feature keyed to it unreliable.
        for seed in 0..8i64 {
            let mut record = crate::AvatarRecord::new("Sized", crate::Archetype::default());
            record.reroll(seed);
            let pair = seated(&record.skeleton(), &record.eyes);
            let across = pair.left.radius * 2000.0;
            assert!(
                (18.0..=32.0).contains(&across),
                "seed {seed}: a {across:.1} mm eyeball, against life's 24.2"
            );
        }
    }

    #[test]
    fn the_sliders_move_the_eyes_the_way_they_say() {
        let wide = eyes(&EyeParams {
            spacing: 1.0,
            ..Default::default()
        });
        let close = eyes(&EyeParams {
            spacing: -1.0,
            ..Default::default()
        });
        assert!(wide.left.pivot.x < close.left.pivot.x, "wider is wider");

        let big = eyes(&EyeParams {
            size: 1.0,
            ..Default::default()
        });
        let small = eyes(&EyeParams {
            size: 0.0,
            ..Default::default()
        });
        assert!(big.left.radius > small.left.radius * 1.4);

        let sunken = eyes(&EyeParams {
            depth: 1.0,
            ..Default::default()
        });
        let bulging = eyes(&EyeParams {
            depth: -1.0,
            ..Default::default()
        });
        assert!(sunken.left.pivot.z < bulging.left.pivot.z);
    }

    #[test]
    fn a_blink_reaches_further_down_the_front_of_the_globe_than_an_open_lid() {
        // Renamed to what it checks. It folds every lid vertex to its frontmost
        // point and compares that on the MIDLINE, so it is a profile test, not
        // a coverage test — and the thing it cannot see is that the lids never
        // meet at the sides at all (#81). The property it does check is real,
        // so it keeps it under an honest name and
        // `the_lids_close_at_corners_rather_than_leaving_a_band` asks the rest.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;

        let exposed = |closure: f32| {
            let upper = eye.upper_lid.transformed(eye.lid_transform(closure, true));
            let lower = eye.lower_lid.transformed(eye.lid_transform(closure, false));
            // How far down the front of the globe each lid reaches, measured as
            // the frontmost point each covers.
            let front_of = |mesh: &PolyMesh| {
                mesh.positions
                    .iter()
                    .map(|point| (*point - eye.pivot).z)
                    .fold(f32::MIN, f32::max)
            };
            let covered = front_of(&upper).max(front_of(&lower));
            eye.radius - covered
        };

        assert!(
            exposed(1.0) < exposed(0.0),
            "shutting the lids should cover more of the eye"
        );
        assert!(
            exposed(1.0) < eye.radius * 0.25,
            "a shut eye should be almost entirely covered"
        );
    }

    #[test]
    #[ignore = "the target, not the state: the lids are caps of the globe and never meet at the sides (#81)"]
    fn the_lids_close_at_corners_rather_than_leaving_a_band() {
        // What makes an eye read as an eye rather than as a bead in a ring: the
        // lids MEET, at a medial and a lateral canthus, so the uncovered region
        // is a lens. Here both lids are `cap_shell` domes concentric with the
        // globe, 71.6° in half-angle and set 163° apart at the default
        // aperture. Two caps summing 143° cannot close a 163° gap, so the
        // uncovered set is an ANNULUS running the full width of the globe.
        //
        // Sampled round the globe's equator rather than on the midline, which
        // is the whole point — the test above looks only where the lids do
        // meet.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;
        let upper = eye.upper_lid.transformed(eye.lid_transform(0.0, true));
        let lower = eye.lower_lid.transformed(eye.lid_transform(0.0, false));

        // For each azimuth round the front of the globe, how tall the gap
        // between the lids is, in degrees of latitude.
        let mut open = Vec::new();
        for step in 0..=8 {
            let azimuth = std::f32::consts::FRAC_PI_2 * step as f32 / 8.0;
            let (sin, cos) = azimuth.sin_cos();
            let mut gap = 0.0f32;
            for tick in -60..=60 {
                let latitude = (tick as f32).to_radians();
                let on_globe = eye.pivot
                    + Vec3::new(sin * latitude.cos(), latitude.sin(), cos * latitude.cos())
                        * eye.radius;
                if !upper.contains(on_globe) && !lower.contains(on_globe) {
                    gap += 1.0;
                }
            }
            open.push((azimuth.to_degrees().round() as i32, gap));
        }
        // At the outer corner the lids must have closed on each other.
        let corner = open.last().expect("sampled").1;
        assert!(
            corner < 5.0,
            "at the outer corner the lids leave {corner:.0}° of the globe bare; \
             the gap by azimuth is {open:?}"
        );
    }

    #[test]
    fn lids_swing_further_above_the_eye_than_below_it() {
        // Real blinks are mostly the upper lid. Splitting the motion evenly is
        // the giveaway that nobody looked at one.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;
        let upper = eye.lid_rotation(1.0, true).to_axis_angle().1;
        let lower = eye.lid_rotation(1.0, false).to_axis_angle().1;
        assert!(upper > lower * 2.0, "upper {upper:.2} vs lower {lower:.2}");
    }

    #[test]
    fn a_resting_aperture_sets_how_open_the_eyes_start() {
        let narrow = eyes(&EyeParams {
            aperture: 0.1,
            ..Default::default()
        });
        let wide = eyes(&EyeParams {
            aperture: 1.0,
            ..Default::default()
        });

        let gap = |pair: &Eyes| {
            let (lo, _) = pair.left.upper_lid.bounds();
            lo.y
        };
        assert!(
            gap(&narrow) < gap(&wide),
            "narrowed lids should hang lower over the eye"
        );
    }

    #[test]
    fn an_eye_looks_where_it_is_told_but_only_so_far() {
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;

        let ahead = eye.gaze_rotation(eye.pivot + Vec3::Z, 0.6);
        assert!(
            ahead.is_near_identity(),
            "looking straight ahead is no turn"
        );

        let aside = eye.gaze_rotation(eye.pivot + Vec3::new(1.0, 0.0, 1.0), 0.6);
        assert!((aside.to_axis_angle().1 - 0.6).abs() < 1e-4, "clamped");

        let slight = eye.gaze_rotation(eye.pivot + Vec3::new(0.1, 0.0, 1.0), 0.6);
        assert!(
            slight.to_axis_angle().1 < 0.6,
            "a small look is not clamped"
        );
    }

    #[test]
    fn a_body_without_a_head_gets_no_eyes() {
        // The `None` moved up a level with the placement (#76): there is nothing
        // to measure a canon from, so there is nothing to seat an eye against,
        // and `Avatar::build_with` skips both together.
        use crate::skeleton::Node;
        let mut bare = Skeleton::new();
        let a = bare.add_node(Node::new(Vec3::ZERO, 0.2));
        bare.extend_from(a, Node::new(Vec3::Y, 0.2));
        let rig = Rig::from_skeleton(&bare).expect("rigs");
        let mesh = crate::build_body(&bare, &crate::CageConfig::default(), 2).expect("mesh");
        assert_eq!(Skull::measure(&mesh, &rig), None);
    }

    #[test]
    fn a_creature_gets_eyes_too() {
        // The trap on #76: the old placement warped an interior point through
        // `skull::reshape` UNCONDITIONALLY, while `skull::shape` bails for
        // anything with more than two feet — so a creature's eyes were placed by
        // a human skull's transform that had never been applied to its head.
        // Bisecting the built surface asks the head in hand instead, whatever
        // shape it came out.
        let pair = seated(
            &QuadrupedParams::default().skeleton(),
            &EyeParams::default(),
        );
        assert!(pair.left.radius > 0.0);
        assert!(pair.assembled(0.0).is_closed_manifold());
        assert!(
            pair.left.pivot.z > 0.0,
            "a muzzle has a front too: {:?}",
            pair.left.pivot
        );
    }

    #[test]
    fn the_globe_is_seated_against_the_surface_rather_than_against_a_prediction() {
        // The measurement the whole of #76 turns on, in the file that does the
        // seating: how far the globe's front pole stands past the skin on its
        // own column. Measured on the shipped build it was 19 to 51 mm on every
        // body, because the prediction it was seated from had no dependence on
        // x and so was right only on the midline.
        //
        // Bisected, not binned, and against the CARVED head — the same
        // instrument `tests/parts.rs` judges the built avatar with, so the two
        // cannot quietly disagree about the same column.
        for seed in 0..8i64 {
            let mut record = crate::AvatarRecord::new("Seated", crate::Archetype::default());
            record.reroll(seed);
            let skeleton = record.skeleton();
            let rig = Rig::from_skeleton(&skeleton).expect("rigs");
            let mut mesh =
                crate::build_body(&skeleton, &crate::CageConfig::default(), 2).expect("mesh");
            let skull = Skull::measure(&mesh, &rig).expect("a skull");
            let canon = super::Canon::measure(&rig, &skull, &record.eyes);
            crate::face::carve_face(&mut mesh, &rig, &canon, &record.face);
            let pair = Eyes::build(&rig, &mesh, &canon, &record.eyes);

            let centre = rig.joints[canon.head].position;
            let far = {
                let (lo, hi) = mesh.bounds();
                hi.z - lo.z
            };
            let column = Vec3::new(
                centre.x + pair.right.pivot.x,
                centre.y + canon.level,
                centre.z,
            );
            let skin = reach(&mesh, column, far).expect("the eye's column is inside the head");
            let stands = (pair.right.pivot.z + pair.right.radius) - skin;
            assert!(
                (0.0005..=0.005).contains(&stands),
                "seed {seed}: the globe's pole stands {:.2} mm past the skin",
                stands * 1000.0
            );
        }
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = EyeParams {
            size: 9.0,
            spacing: f32::NAN,
            depth: -7.0,
            aperture: f32::INFINITY,
        };
        params.sanitize();
        assert_eq!(params.size, 1.0);
        assert_eq!(params.spacing, 0.0);
        assert_eq!(params.depth, -1.0);
        assert_eq!(params.aperture, 0.8);

        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }
}
