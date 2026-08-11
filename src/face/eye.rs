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
        // Ranges are the exploration envelope (#160): the conservative range
        // stretched about each axis's own default. `sanitize_axis` rather than
        // clamp-by-hand, for #55's reason: the guard must precede the clamp.
        use crate::plan::{explore_range, sanitize_axis};
        self.size = sanitize_axis(self.size, 0.5, explore_range(0.5, (0.0, 1.0)));
        self.aperture = sanitize_axis(self.aperture, 0.8, explore_range(0.8, (0.0, 1.0)));
        self.spacing = sanitize_axis(self.spacing, 0.0, explore_range(0.0, (-1.0, 1.0)));
        self.depth = sanitize_axis(self.depth, 0.0, explore_range(0.0, (-1.0, 1.0)));
    }
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
    /// The sign of this eye's `x`, so `+1` for the body's left eye and `-1` for
    /// its right.
    ///
    /// Left is `+X` — see [`crate::plan::Limb`] for the convention and #142 for
    /// the pass that corrected it here. What this field actually keys is
    /// `CANTHAL_TILT` — private, so unlinked — which tilts the outer canthus
    /// above the inner, and that
    /// wants the sign of `x` rather than a name; the two only ever disagreed
    /// about what to call the eye.
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

    /// What a viewer can see of this eye, with the lids at rest.
    ///
    /// `skin` is the body as it will be drawn, paired with the head joint it is
    /// measured from, since the eye's own parts are head-local and the body is
    /// not. Passing `None` measures what the LIDS alone leave bare, and `lids`
    /// turns them off in the same way — the three readings together are what
    /// diagnose an aperture, because each names an owner:
    ///
    /// ```text
    ///                    share   centre az   spans az     owner
    ///   skin and lids    13.9%      +34.8    -23..+97
    ///   skin alone       18.5%      +38.2    -23..+97     the medial edge
    ///   lids alone       32.1%       +0.0   -180..+179    top and bottom
    /// ```
    ///
    /// Those were the shipped numbers before #81. Reading them together says
    /// what no single one of them does: the elevation is identical with and
    /// without the skin, so the lids own it; the medial azimuth is identical
    /// with and without the lids, so the skin owns it; and the lateral edge is
    /// the same in all three, so **nothing owned it** and the eye was bare 97°
    /// round the side of the head.
    ///
    /// Sampled on the globe's own surface and weighted by solid angle. It is not
    /// a cheap call — a containment test per sample per occluder — and nothing
    /// in a build path uses it.
    #[must_use]
    pub fn aperture(&self, skin: Option<(&PolyMesh, Vec3)>, lids: bool) -> Aperture {
        let upper = self.upper_lid.transformed(self.lid_transform(0.0, true));
        let lower = self.lower_lid.transformed(self.lid_transform(0.0, false));

        let (mut whole, mut bare, mut moment) = (0.0f32, 0.0f32, Vec3::ZERO);
        let mut span = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        let mut elevation = -std::f32::consts::FRAC_PI_2 + APERTURE_STEP;
        while elevation < std::f32::consts::FRAC_PI_2 {
            let band = elevation.cos();
            let mut azimuth = -std::f32::consts::PI;
            while azimuth < std::f32::consts::PI {
                let (sin, cos) = azimuth.sin_cos();
                let toward = Vec3::new(sin * band, elevation.sin(), cos * band);
                let on_globe = self.pivot + toward * self.radius;
                whole += band;
                let hidden = skin.is_some_and(|(mesh, head)| mesh.contains(head + on_globe))
                    || (lids && (upper.contains(on_globe) || lower.contains(on_globe)));
                if !hidden {
                    bare += band;
                    moment += toward * band;
                    span = (
                        span.0.min(azimuth),
                        span.1.max(azimuth),
                        span.2.min(elevation),
                        span.3.max(elevation),
                    );
                }
                azimuth += APERTURE_STEP;
            }
            elevation += APERTURE_STEP;
        }

        let axis = moment.normalize_or(Vec3::Z);
        Aperture {
            share: bare / whole.max(f32::EPSILON),
            centre: (axis.x.atan2(axis.z), axis.y.clamp(-1.0, 1.0).asin()),
            span,
        }
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

/// What a viewer can see of one eye, measured on the globe's own surface.
///
/// **The measurement the eye's whole shape turns on, and it lives here so that
/// nothing has to keep its own copy.** `examples/headaudit` reports it and
/// `the_eye_opens_on_the_gaze_rather_than_where_the_skin_falls_away` asserts on
/// it; the last time an instrument and the code it judged carried two copies of
/// the same angles they drifted 30° apart (#81), and the time before that a tool
/// went on printing fractions the canon had moved (#74).
///
/// Angles are about the **gaze**, not about the head: azimuth is the turn right
/// of dead ahead and elevation the rise above it, both in radians, both measured
/// from the eye's own pivot. That frame is the point — an aperture centred
/// anywhere but zero is one the skin cut rather than one the lids opened.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aperture {
    /// How much of the globe is bare, as a share of its whole surface.
    ///
    /// Solid-angle weighted, not a count of samples: a degree of latitude near
    /// the pole is a shorter arc than one at the equator, and a share of samples
    /// is a share of the sampling (#81).
    pub share: f32,
    /// Where the middle of the bare set points: azimuth, then elevation.
    pub centre: (f32, f32),
    /// How far it reaches: azimuth least and greatest, then elevation least and
    /// greatest.
    pub span: (f32, f32, f32, f32),
}

/// How finely [`Eye::aperture`] samples the globe, in radians.
///
/// A degree. The features it has to resolve — a canthus closing, a lid margin
/// crossing the skin's own edge — are tens of degrees across, and the cost is a
/// containment test per sample per occluder.
/// Provenance: **derived** — a degree, chosen against the angular size of
/// the features it must resolve and the cost per sample.
const APERTURE_STEP: f32 = 0.017_453;

/// A body's pair of eyes, and where they belong.
#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
    /// The body's left eye, which is the one at `+X`.
    pub left: Eye,
    /// The body's right eye, which is the one at `−X`.
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
/// Provenance: **looked up** — 24.2 mm transverse, 23.7 mm axial. No
/// citation is attached and one should be: these are standard ocular
/// dimensions, quoted here from general knowledge rather than from a named
/// table. The invariance claim (no dependence on sex, age past infancy or
/// ethnicity) is the load-bearing part and is the part most worth a source.
const GLOBE: f32 = 0.0121;

/// The measured height of the reference body's skin, in metres.
///
/// The stature the globe above belongs to, measured the way the globe is placed:
/// off the body in hand. A 1.75 m adult's *skin* spans about 1.64 m — the crown
/// of the head sits below the nominal stature — and across seventeen bodies that
/// ratio holds to within 4%.
/// Provenance: **derived**, and measured in-crate rather than looked up — it
/// is this pipeline's own skin height for a 1.75 m body, checked across
/// seventeen bodies. It is therefore a fact about the mesher, not about
/// people, and it moves when the body plan does.
const REFERENCE: f32 = 1.65;

/// How much of a change in stature the globe follows.
///
/// Not none, because a small body's eye really is a little smaller, and not one,
/// because it is nothing like proportional. A third takes a 1.25 m body to
/// 11.2 mm and a 2.09 m body to 13.3 against life's 12.1 — a millimetre or two
/// either way, which is what the anthropometry says the spread actually is.
/// Provenance: **tuned by render**, bounded by looked-up spread — a third
/// puts the extremes at 11.2 and 13.3 mm against life's 12.1.
const STATURE_GAIN: f32 = 0.35;

/// How far the globe's front pole stands proud of the skin at rest, in metres.
///
/// In life the corneal apex sits roughly level with the lids around it, so this
/// is small on purpose. It is also the whole of what decides how much eye shows:
/// the body is a closed surface with no opening cut for an eye, so the visible
/// part of the globe is exactly the cap that clears the skin. Three millimetres
/// on a 12.4 mm globe is a lens 16.2 mm wide, against a globe that until now
/// stood 19 to 51 mm proud on every body (#76).
/// Provenance: **looked up, then tuned by render** (#76). The looked-up part
/// is that the corneal apex sits roughly level with the lids; 3 mm is what
/// that came to on a 12.4 mm globe.
const PROUD: f32 = 0.003;

/// How far the depth axis moves that, in metres.
///
/// Kept inside `tests/parts.rs`'s 5 mm ceiling at both ends of the axis, so a
/// sunken eye and a protruding one are both eyes that are seated.
/// Provenance: **derived** from a test ceiling — `tests/parts.rs` allows
/// 5 mm, and this is what keeps both ends of the axis inside it.
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
        // The anatomical globe, CAPPED by the head that has to hold it (#160).
        // An eyeball is the one facial dimension that does not scale with the
        // face — which is anatomy on any head a person has, and a broken
        // invariant on the exploration range's: at `headSize` −2.8 the head is
        // a third of its neutral radius, the un-capped globe reaches 0.39 of
        // it, and the iris runs into the skin of the nose on every rolled body
        // that deep. The neutral proportion is 0.12 of the head's radius; the
        // cap at 0.16 never binds inside the old ±1 range and holds an
        // extreme-small head's eyes at eye-like proportion instead.
        let radius = globe_radius(mesh, params).min(0.16 * rig.joints[canon.head].radius);
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

        // `canon.apart` is a half-separation, so the pivot as measured is the
        // `+X` eye — the body's LEFT (#142). Both eyes are built from the one
        // figure and differ only in the sign of `x`, which is also what `side`
        // carries; nothing here is asymmetric but the name.
        let pivot = Vec3::new(canon.apart, canon.level, skin - radius + proud);
        Self {
            left: eye(1.0, pivot, radius, params),
            right: eye(-1.0, Vec3::new(-pivot.x, pivot.y, pivot.z), radius, params),
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
fn globe_radius(mesh: &PolyMesh, params: &EyeParams) -> f32 {
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

/// Half-angle of the pupil, in radians.
///
/// **The eye's landmarks are ANGLES, and they live here so that the geometry and
/// the colours cannot drift apart.** They were two bare cosines in
/// `avatar::iris_of` — 0.78 and 0.50, which is 38.7° and 60° — and the globe was
/// a sphere with rings 18° apart about `+Y`, so neither threshold landed near a
/// ring and the pupil covered 91.7% of the visible cap by solid angle (#81).
///
/// A pupil is about 3.5 mm across a 24.2 mm globe at an ordinary indoor light
/// level, which is this.
/// Provenance: **looked up** (#81) — 3.5 mm across a 24.2 mm globe at an
/// ordinary indoor light level. **This is the constant #52 was written
/// about.** Its predecessor was the bare cosine 0.78 sitting in another file
/// with nothing saying whether it had been measured or fitted, and it was
/// silently absorbing an error in the face's width.
const PUPIL: f32 = 0.1431;

/// Half-angle of the iris.
///
/// An 11.7 mm iris on a 24.2 mm globe. This is the best-sourced figure on the
/// whole face: visible iris diameter is near-constant across sex, age past
/// infancy and ethnicity, in the same way the globe itself is (#77).
/// Provenance: **looked up** (#77) — 11.7 mm visible iris on a 24.2 mm
/// globe, and the docstring's claim that this is the best-sourced figure on
/// the face is true only in the sense that it is the most invariant; it
/// carries no more citation than the rest.
const LIMBUS: f32 = 0.5044;

/// How much of the iris, at its outer edge, is the darker limbal ring.
///
/// Life has one and it is most of what makes an iris read as an iris rather than
/// as a coloured disc.
/// Provenance: **tuned by render**.
const LIMBAL: f32 = 0.0454;

/// How tight a colour boundary is: the gap between the ring pair straddling it.
///
/// A colour boundary on a Gouraud-shaded mesh is only as sharp as the gap
/// between the two rings that carry the two colours. At 1.4° that is 0.3 mm on a
/// 12.5 mm globe, which reads as an edge; at the 18° the old ring spacing gave,
/// it read as a smear.
/// Provenance: **derived** from the ring spacing — 1.4 degrees is 0.3 mm on
/// a 12.5 mm globe, which is the width a colour boundary reads as an edge at.
const EDGE: f32 = 0.0244;

/// Where a point on the globe falls, as a colour.
///
/// Takes an offset from the eye's own pivot, so it turns with the gaze for free.
/// Lives here rather than in [`crate::avatar`] because it and `globe` below have to
/// agree about the angles above, and a threshold in one file with the rings that
/// carry it in another is how they came to disagree by 30° in the first place.
#[must_use]
pub fn iris_of(offset: Vec3) -> Vec3 {
    let polar = offset.normalize_or(Vec3::Z).z.clamp(-1.0, 1.0).acos();
    if polar < PUPIL {
        Vec3::new(0.05, 0.06, 0.08)
    } else if polar < LIMBUS - LIMBAL {
        Vec3::new(0.24, 0.38, 0.46)
    } else if polar < LIMBUS {
        // The limbal ring: the same hue, well down in value.
        Vec3::new(0.10, 0.15, 0.19)
    } else {
        Vec3::new(0.93, 0.92, 0.90)
    }
}

/// The eyeball, with a latitude ring either side of every colour boundary.
///
/// The filler rings past the limbus only have to round a sphere off; nothing is
/// drawn on them and nothing looks at them, since the back of an eye is inside a
/// head.
fn globe(radius: f32) -> PolyMesh {
    /// How many facets round the iris. Sixteen puts a 12 mm iris on a 16-gon,
    /// which is under a third of a millimetre of chord error.
    const SEGMENTS: usize = 16;

    let mut polars = Vec::with_capacity(12);
    for boundary in [PUPIL, LIMBUS - LIMBAL, LIMBUS] {
        polars.push(boundary - EDGE * 0.5);
        polars.push(boundary + EDGE * 0.5);
    }
    // Enough to be a sphere behind the iris, and no more.
    polars.extend([0.30, 0.70, 1.2, 1.7, 2.2, 2.7]);
    prim::sphere_rings(radius, &polars, SEGMENTS)
}

/// Where the lids meet, as an azimuth either side of the gaze.
///
/// The canthi, and the only two points on the eye where both lids are. Sixty
/// degrees lands the visible corner at 0.87 of the globe's radius off the
/// midline, which is a fissure as long as the eye is wide — and it falls exactly
/// on a vertex at [`SEGMENTS`], so the corner is a corner rather than a chord
/// across one.
/// Provenance: **derived**, and doubly so — 60 degrees puts the corner at
/// 0.87 R off the midline, a fissure as long as the eye is wide, and it is
/// also exactly a multiple of the [`SEGMENTS`] spacing so the corner lands on
/// a vertex. Two independent reasons for one number is the strongest kind.
const CANTHUS: f32 = std::f32::consts::FRAC_PI_3;

/// Where the upper lid's margin sits at the midline, in radians above the gaze,
/// with the lids shut and with them at their widest.
///
/// **The aperture lives in the margin now, not in a rest rotation.** It used to
/// be a rigid turn applied to a circular cap, which could open and shut the eye
/// but could not change the SHAPE of what it left bare. Shut is the upper margin
/// BELOW the lower one, so the two overlap; there is no separate closed state to
/// keep in step.
/// Provenance: **tuned by render**; see [`LOWER_MARGIN`] for the measurement
/// the pair was checked against.
const UPPER_MARGIN: (f32, f32) = (-0.105, 0.681);

/// The same for the lower lid, in radians below the gaze.
///
/// At the default aperture the pair open 52° of latitude, against life's 11 mm
/// of palpebral fissure on a 12.1 mm globe, which is 52°.
/// Provenance: **looked up, then tuned by render** — 11 mm of palpebral
/// fissure on a 12.1 mm globe is 52 degrees, which is what the pair opens.
const LOWER_MARGIN: (f32, f32) = (-0.070, -0.463);

/// How far the lateral canthus sits above the medial one, in radians.
///
/// One to two millimetres in life, and absent from the first specification for
/// this entirely: level canthi read flat and sad. It cancels out of where the
/// two margins meet, so it tilts the fissure without opening a gap at either
/// corner.
/// Provenance: **looked up, then tuned by render** — one to two millimetres
/// in life.
const CANTHAL_TILT: f32 = 0.035;

/// How far round the lid's rim is sampled.
///
/// Twenty-four puts a vertex every 15°, which is what makes [`CANTHUS`] a point
/// rather than a chord, and keeps four lids inside `tests/budget.rs`'s dominance
/// guard — they are charted into the head's atlas, so they count as skin.
/// Provenance: **derived** from two constraints at once — it must divide
/// [`CANTHUS`] exactly, and four lids must stay inside `tests/budget.rs`'s
/// dominance guard.
const SEGMENTS: usize = 24;

/// How far the lid's rim may reach below the pole, in radians.
///
/// The margin would carry on past the canthus to the far side of the globe,
/// where nothing can see it and every ring spent on it sags the shell into the
/// eye. Capped at 112°, which is past where the skin closes over on every seed
/// measured (97° to 101° lateral).
/// Provenance: **derived** from the built surface — 112 degrees is past
/// where the skin closes over on every seed measured, 97 to 101 lateral.
const RIM_LIMIT: f32 = 1.955;

/// Where a lid's margin sits at an azimuth from the gaze, in radians of
/// elevation.
///
/// **The two margins cross at exactly one pair of azimuths, and that is the
/// point.** Both are the same cosine, which is zero at `±`[`CANTHUS`] and
/// nowhere else in range, so `upper` and `lower` are equal there whatever the
/// aperture and whatever the tilt — the lids meet at two corners by
/// construction. Past the canthus the cosine goes negative, so the upper margin
/// drops below the eye's equator while the lower rises above it and the two
/// overlap: there is no bare band outside the fissure, which is the whole defect
/// this replaces (#81).
fn margin(azimuth: f32, side: f32, open: f32, upper: bool) -> f32 {
    let (shut, wide) = if upper { UPPER_MARGIN } else { LOWER_MARGIN };
    let reach = shut + (wide - shut) * open;
    let along = (azimuth / (2.0 * CANTHUS)).clamp(-1.0, 1.0);
    reach * (std::f32::consts::PI * along).cos()
        + CANTHAL_TILT * (azimuth / CANTHUS).clamp(-1.0, 1.0) * side
}

/// Builds one eye at `pivot`.
fn eye(side: f32, pivot: Vec3, radius: f32, params: &EyeParams) -> Eye {
    let globe = globe(radius).transformed(Mat4::from_translation(pivot));

    // A lid is a shell clear of the globe, so it never intersects the eye it
    // covers. **1.08 rather than 1.06, and it is the rim's doing.** The surface
    // between two rings is a chord, and a rim that now reaches 112° instead of
    // 72° puts three rings 37° apart — a chord that sags to 0.947 of the shell,
    // which at 1.06 passes 0.9996 of the globe and z-fights it.
    let shell = radius * 1.08;
    let thickness = radius * 0.10;
    let open = params.aperture.clamp(0.0, 1.0);

    let lid = |upper: bool| {
        // `prim::margin_shell` domes around +Y and counts its segments from +X,
        // so a segment at `turn` sits at gaze azimuth `FRAC_PI_2 - turn` and the
        // rim's polar angle is a quarter turn less the margin's elevation. The
        // lower lid is the same shell brought under the eye by a half turn about
        // X, which negates both, hence the mirrored azimuth and the sign.
        let half = std::f32::consts::FRAC_PI_2;
        let rim: Vec<f32> = (0..SEGMENTS)
            .map(|segment| {
                let turn = std::f32::consts::TAU * segment as f32 / SEGMENTS as f32;
                let azimuth = if upper { half - turn } else { half + turn };
                let at = margin(wrap(azimuth), side, open, upper);
                let polar = if upper { half - at } else { half + at };
                polar.min(RIM_LIMIT)
            })
            .collect();
        let flip = if upper {
            Quat::IDENTITY
        } else {
            Quat::from_rotation_x(std::f32::consts::PI)
        };
        prim::margin_shell(shell, thickness, &rim, 3)
            .transformed(Mat4::from_translation(pivot) * Mat4::from_quat(flip))
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

/// An angle brought into `-π..=π`, so an azimuth round the back of the globe is
/// read as the far side of the fissure rather than as many turns from it.
fn wrap(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let turned = (angle + PI).rem_euclid(TAU);
    turned - PI
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
        let mut mesh = crate::build_body(
            skeleton,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &Default::default(),
        )
        .expect("mesh");
        let skull = Skull::measure(&mesh, &rig).expect("a skull");
        let canon = super::Canon::measure(&rig, &skull, params);
        crate::face::carve_face(&mut mesh, &rig, &canon, &Default::default());
        Eyes::build(&rig, &mesh, &canon, params)
    }

    fn eyes(params: &EyeParams) -> Eyes {
        seated(
            &HumanoidParams::default().skeleton(&crate::Composites::default()),
            params,
        )
    }

    #[test]
    fn the_eye_opens_on_the_gaze_rather_than_where_the_skin_falls_away() {
        // **The question this whole part turns on, and it is about WHERE the eye
        // opens rather than how much of it shows.** A head here has no socket:
        // the body is a closed surface and the globe pokes through it, so the
        // opening is whatever the skin happens to be doing at the eye's column —
        // and that surface tilts laterally. Before the lids were given a margin
        // (#81) the bare set ran from 16° medial to 99° LATERAL, an opening 115°
        // wide whose middle sat 42° off the direction the eye was looking; the
        // iris is the one thing on an eye that is centred, so it read as shoved
        // to the nasal side.
        //
        // The lid margin took the lateral edge, and the orbital hollow (#88)
        // took most of the medial one. Measured after both: +13.1 / +11.8 /
        // +8.6 / +1.3 degrees on the seeds below, against +35 to +40 before
        // either. Five degrees was the ask and one seed in four met it, so this
        // was written as an IGNORED TARGET carrying its own failure numbers.
        //
        // **IT IS THE STATE NOW, AND NOTHING WAS AIMED AT IT** (#88, promoted
        // 2026-08-11). Re-measured: **+0.30 / −0.02 / +0.09 / −3.75 degrees**.
        // Three of the four sit within a third of a degree of the gaze and the
        // fourth has 1.25 degrees of margin; `examples/headaudit`'s sweep reads
        // the same column within 5 degrees on all eight of its seeds. The last
        // change made FOR this was #91's `MEDIAL` 0.28 → 0.14; everything since
        // was head work — the eight-point cage, the narrowed and lengthened
        // skull of #79, the neck of #125/#129/#131 — and #91's warning on this
        // issue was that the orbit is a place where any head change costs
        // something. It has now paid instead, which is a thing worth recording
        // because the reverse is what this issue expected.
        //
        // Un-ignored deliberately: an acceptance criterion that is met and still
        // skipped is a guard that rots silently, which is the failure this crate
        // keeps finding in its own tests. **The cost is recorded rather than
        // absorbed, and the figure to record is the SUITE's and not this
        // test's**: each seed builds a whole avatar and then asks `contains` of
        // the body and both lids at every degree of the globe, which is 8.2
        // seconds for the four in release and 195 run alone in debug — but the
        // debug lib suite goes 291.5 s to 298.0, because 504 other tests hold
        // the other threads and this one overlaps them. Six and a half seconds,
        // not three minutes. Quoting a test's own wall clock as what it costs a
        // parallel suite over-reads it thirtyfold; if the cost ever does matter
        // the lever is the seed count and not `APERTURE_STEP`, which
        // `examples/headaudit` shares.
        //
        // The residual it leaves is not the eye's and not the lids': it is how
        // far the skin beside the NOSE reaches across the globe, and it varies
        // with the head rather than with the eye, because the lateral edge is
        // authored and the medial one is not. Seed 42 is where that still shows.
        let mut worst: Vec<(i64, f32)> = Vec::new();
        for seed in [1i64, 7, 23, 42] {
            let mut record = crate::AvatarRecord::new("Opened", crate::Archetype::default());
            record.reroll(seed);
            let avatar = crate::Avatar::build(&record).expect("a biped builds");
            let eyes = avatar.parts.eyes.as_ref().expect("a humanoid has eyes");
            let head = avatar.rig.joints[eyes.head].position;
            let at = eyes.right.aperture(Some((&avatar.parts.body, head)), true);
            worst.push((seed, at.centre.0.to_degrees()));
        }
        assert!(
            worst.iter().all(|&(_, off)| off.abs() < 5.0),
            "the bare eye's centre sits this far off the gaze, by seed: {:?}",
            worst
                .iter()
                .map(|&(seed, off)| (seed, (off * 10.0).round() / 10.0))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_body_gets_two_eyes_set_in_its_face() {
        let pair = eyes(&EyeParams::default());
        // Named by the coordinate convention rather than by which is further
        // along X: left is `+X` on a body facing `+Z` (#142, and see
        // [`crate::plan::Limb`]). Comparing the two against each other would
        // pass on a pair that was simply the wrong way round.
        assert!(pair.left.pivot.x > 0.0, "the left eye is at +X");
        assert!(pair.right.pivot.x < 0.0, "the right eye is at -X");
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
            .skeleton(&crate::Composites::default());
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
        // Measured as the SEPARATION rather than as the signed `x` of one eye,
        // which is what the slider actually moves. Reading it off one eye's
        // coordinate meant this test was quietly asserting which side that eye
        // was on, and it failed when #142 corrected the answer — which is a
        // slider test failing over a naming change, so it was measuring the
        // wrong thing.
        let apart = |pair: &Eyes| (pair.left.pivot.x - pair.right.pivot.x).abs();
        assert!(apart(&wide) > apart(&close), "wider is wider");

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

    /// How much of the globe the lids leave bare at one azimuth from the gaze,
    /// in degrees of latitude, and where the middle of that gap sits.
    ///
    /// **Sampled on the globe's own surface, not off the lids' bounding box.**
    /// A lid's rim is a curve now, so its lowest point is out at the canthus
    /// rather than over the pupil, and a test that reads `bounds()` is reading
    /// the corner while claiming to measure the opening — which is how
    /// `a_resting_aperture_sets_how_open_the_eyes_start` came to invert when the
    /// rim stopped being a circle (#81).
    fn bare(eye: &Eye, azimuth: f32) -> (f32, f32) {
        let upper = eye.upper_lid.transformed(eye.lid_transform(0.0, true));
        let lower = eye.lower_lid.transformed(eye.lid_transform(0.0, false));
        let (sin, cos) = azimuth.to_radians().sin_cos();
        let (mut gap, mut sum) = (0.0f32, 0.0f32);
        for tick in -890..=890 {
            let latitude = (tick as f32 * 0.1).to_radians();
            let on_globe = eye.pivot
                + Vec3::new(sin * latitude.cos(), latitude.sin(), cos * latitude.cos())
                    * eye.radius;
            if !upper.contains(on_globe) && !lower.contains(on_globe) {
                gap += 0.1;
                sum += tick as f32 * 0.1;
            }
        }
        (gap, if gap > 0.0 { sum * 0.1 / gap } else { 0.0 })
    }

    #[test]
    fn the_lids_close_at_corners_rather_than_leaving_a_band() {
        // What makes an eye read as an eye rather than as a bead in a ring: the
        // lids MEET, at a medial and a lateral canthus, so the uncovered region
        // is a lens. Both lids used to be `cap_shell` domes concentric with the
        // globe, 71.6° in half-angle and set 163° apart at the default aperture.
        // Two caps summing 143° cannot close a 163° gap, so the uncovered set
        // was an ANNULUS: measured round the front, 54 / 54 / 53 / 52 / 50 / 48
        // / 44 / 41 / 38 degrees of bare globe at every azimuth out to 90 —
        // there was no corner at any aperture at which the eye was open, and the
        // aperture's whole lateral edge was wherever the skin happened to fall
        // away, 97° round the side of the head.
        //
        // Sampled round the globe rather than on the midline, which is the whole
        // point: `a_blink_reaches_further_down_the_front_of_the_globe_than_an_
        // open_lid` looks only where the lids always did meet.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;
        let open: Vec<(i32, f32)> = (0..=9)
            .map(|step| {
                let azimuth = 10.0 * step as f32;
                (azimuth as i32, bare(eye, azimuth).0)
            })
            .collect();

        // Shut by the canthus and stays shut past it. Sampled at 70° and beyond
        // rather than at the canthus itself, because a lens closes to nothing
        // gradually and its last degree is a chord of the rim.
        for &(azimuth, gap) in &open {
            if azimuth >= 70 {
                assert!(
                    gap < 2.0,
                    "past the canthus the lids leave {gap:.1}° of the globe bare at \
                     {azimuth}°; the gap by azimuth is {open:?}"
                );
            }
        }
        // And it is a lens rather than a slit: still open over the pupil.
        assert!(
            open[0].1 > 30.0,
            "the fissure is only {:.1}° tall at the midline",
            open[0].1
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
        // **Measured over the pupil, not off the lid's bounding box.** This read
        // `upper_lid.bounds().0.y` and asked that a narrowed lid hang lower.
        // That was true only while the rim was a circle: with a margin curve the
        // lid's lowest point is out at the canthus, where it dips furthest below
        // the eye's equator precisely BECAUSE the eye is wide open, so the test
        // inverted while the aperture did exactly what it says (#81).
        let at = |aperture: f32| {
            let pair = eyes(&EyeParams {
                aperture,
                ..Default::default()
            });
            bare(&pair.left, 0.0).0
        };
        let (shut, narrow, wide) = (at(0.0), at(0.3), at(1.0));
        assert!(
            shut < 1.0,
            "at aperture zero the lids left {shut:.1}° of the globe bare over the pupil"
        );
        assert!(
            narrow < wide * 0.8,
            "a narrowed eye opened {narrow:.1}° against a wide one's {wide:.1}°"
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
        let mesh = crate::build_body(
            &bare,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &Default::default(),
        )
        .expect("mesh");
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
            &QuadrupedParams::default().skeleton(&crate::Composites::default()),
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
            let mut mesh = crate::build_body(
                &skeleton,
                &crate::CageConfig::default(),
                crate::BODY_SUBDIVISIONS,
                &Default::default(),
            )
            .expect("mesh");
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
        // Bounds are the exploration envelope (#160).
        assert_eq!(params.size, 2.0);
        assert_eq!(params.spacing, 0.0);
        assert_eq!(params.depth, -3.0);
        assert_eq!(params.aperture, 0.8);

        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }
}
