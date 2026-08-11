//! Painting skin into a baked atlas.
//!
//! The layer stack is the one stylised character art converges on, and it is
//! mostly *texture* rather than shading — which is why it is worth doing well
//! here rather than deferring it to a shader:
//!
//! 1. a base tone along a melanin ramp, shifted by undertone;
//! 2. **subdermal colour** where blood runs close to the surface — cheeks, ears,
//!    knuckles, knees. Overwatch shipped a dedicated "blood map" for exactly
//!    this, and it does more for skin reading as skin than any lighting model;
//! 3. cavity shading, darkening and reddening creases;
//! 4. freckles and stubble as masked high-frequency detail.
//!
//! Every layer samples noise in **body space, never atlas space**. Atlas space
//! is discontinuous across chart boundaries, so a freckle field sampled there
//! would visibly break at every seam; body space is continuous by construction,
//! so detail flows over a shoulder without noticing the seam beneath it.
//!
//! Colours are authored and blended directly in sRGB, the way a texture painter
//! works, rather than in linear light. [`TextureMap::albedo`] is sRGB-encoded,
//! so this is also what the container asks for.

use glam::Vec3;
use noise::{NoiseFn, Simplex};
use serde::{Deserialize, Serialize};
use symbios_texture::generator::TextureMap;
use symbios_texture::normal::{BoundaryMode, height_to_normal};

use super::bake::AtlasGeometry;
use crate::plan::{Composites, Zone};
use crate::rig::Rig;

/// The melanin ramp, palest to deepest, in sRGB.
///
/// Fitted to the ten shades of the **Monk Skin Tone Scale** — Ellis Monk's open
/// scale, developed with Google to replace Fitzpatrick in computer vision —
/// rather than authored by eye. A skin-tone ramp is the one thing in this crate
/// where "it looks right to me" is the least trustworthy possible test, and a
/// published reference is available for free.
///
/// It replaces a two-stop cosine ramp between a pale and a deep colour, and
/// measuring the two against each other says exactly why two stops cannot work:
///
/// - **Hue was flat.** The old ramp held 18–21° from end to end. Real skin runs
///   30–40° at the pale end, falls through the high teens in the middle, and
///   comes back up toward 26° at the deepest. A straight line between two
///   colours cannot do that, which is what "give it an interior control point"
///   meant.
/// - **Saturation was inverted where it matters most.** The old ramp climbed
///   monotonically to `0.56` at full melanin. The reference *peaks* near `0.48`
///   around the seventh shade and falls to `0.22` at the tenth: deep skin is
///   darker and **less** saturated, not more. Climbing saturation into a dark
///   value is the definition of a garish orange, and that is what the deep end
///   was.
/// - **It did not go dark enough.** The old deepest tone sat at value `0.28`,
///   about the eighth shade. The scale reaches `0.16`.
///
/// **Hue and value are the reference's; saturation is held up at the ends.**
/// That deviation was forced by the render, and measuring the lit result is the
/// only way it showed. The scale's outermost chips are nearly neutral — 0.07
/// saturation at the palest, 0.22 at the deepest — which is right for a chip
/// judged flat under neutral light and wrong for a character: rendered against
/// a cool fill, melanin `0.05` came back at 0.05 saturation, a colourless
/// mannequin, and melanin `0.95` at 0.21, charcoal-grey. The middle of the
/// ramp, where the reference is already saturated, needed nothing — melanin
/// `0.70` measured 0.52 and read as skin immediately.
///
/// So the ends are lifted to 0.16 and 0.34, and the deepest value is raised
/// from `0.16` to `0.19`. That is a deliberate move from a colorimetric scale
/// toward the stylised semi-realistic target the project is aimed at, and it
/// keeps everything the reference is actually being used for: the hue curve,
/// and saturation that *peaks* in the deep middle and falls away either side
/// rather than climbing into the dark.
///
/// One more stop moved, for an unrelated reason. The scale is a set of sample
/// chips, not a ramp, and its third chip is *lighter* than its second — read as
/// a slider it briefly runs backwards. The third stop is darkened from `0.97`
/// to `0.94` to fix that. Imperceptible either way, and still worth doing:
/// dragging melanin up has to darken skin at every point on the axis, and
/// `the_ramp_runs_dark_monotonically` is what noticed.
const RAMP: [[f32; 3]; 10] = [
    [0.960, 0.883, 0.806],
    [0.950, 0.864, 0.779],
    [0.940, 0.877, 0.752],
    [0.920, 0.846, 0.699],
    [0.840, 0.732, 0.571],
    [0.630, 0.497, 0.340],
    [0.510, 0.362, 0.265],
    [0.380, 0.257, 0.205],
    [0.240, 0.189, 0.149],
    [0.190, 0.154, 0.125],
];

/// How far each end of the undertone axis rotates the hue, in degrees.
///
/// A rotation, in degrees, because that is the quantity the axis is *about* —
/// and getting there took two wrong answers, both of which measured as the same
/// failure at opposite ends of the ramp.
///
/// The original shift was **absolute**: `0.06` of blue whatever lay underneath
/// it. That is 15% of the blue in the palest complexion and **100%** of the blue
/// in the deepest, so full warmth drove deep skin to `rgb(71, 52, 15)` — a
/// garish orange rather than a complexion.
///
/// Making the shift **proportional to each channel** fixed the deep end and
/// broke the pale one, for the mirror-image reason. The palest tones are nearly
/// neutral — barely 7% saturated — so a 7.5% swing of green against blue is
/// enormous in *hue* terms even while it is tiny in absolute ones: it clipped
/// green and swung the pale end from magenta to yellow-green.
///
/// What is actually wanted is the same *hue* movement everywhere, and saying so
/// directly costs one conversion. Value and saturation are untouched, so no
/// channel can clip and no tone can leave the skin range it started in.
const UNDERTONE_DEGREES: f32 = 15.0;
/// How blood shifts the skin's colour where it runs close to the surface.
///
/// A *shift*, not a colour to blend toward: haemoglobin absorbs green and blue
/// and passes red, so it warms the existing tone rather than replacing it.
/// Blending toward a red target instead crushes the other channels and turns
/// every knuckle salmon-pink.
const SUBDERMAL_TINT: Vec3 = Vec3::new(0.30, -0.10, -0.13);

/// How much of a measured fold the painter is allowed to ink.
///
/// [`crate::mesh::PolyMesh::crease`] saturates on anything tighter than 9 mm,
/// and at full strength every saturated fold paints as ink: −35% albedo, −70%
/// baked occlusion and +80% blush, stacked. On a face that is most of what a
/// viewer sees — the lip grooves, the jaw hollow, and every resolution
/// boundary's curvature row painted as a ladder of dark bars between the mouth
/// and the chin, with a dark line across the whole throat where the refinement
/// passes end at the head zone's edge (#158). The upper skull stayed clean
/// only because its forms are broader than the crease measure's floor, which
/// is why the lower face read a class below it.
///
/// Zeroing the response cleaned the whole lower face in one move and cost the
/// body almost nothing — the occlusion pass was already near-white everywhere
/// but the artifacts — but it also erased the parting's own shadow, and a
/// mouth needs its line. So the response is *compressed*, not removed: fold
/// shading survives at this fraction, which keeps the parting, the nostrils
/// and the eye sockets readable while a saturated artifact row can no longer
/// paint darker than a deliberate groove.
///
/// Provenance: **tuned by render** (#158), closed and open profile shots
/// against the untouched upper skull.
const CREASE_INK: f32 = 0.30;

/// What a body's skin looks like.
///
/// Axes run `0..=1`, and every combination is a usable complexion — there is no
/// arrangement of these that produces something that reads as a mistake.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SkinParams {
    /// Base depth of tone, from palest to deepest.
    #[serde(with = "crate::plan::scaled")]
    pub melanin: f32,
    /// Hue beneath the tone: `-1` cool and pink, `+1` warm and olive.
    #[serde(with = "crate::plan::scaled")]
    pub undertone: f32,
    /// How much blood shows through at cheeks, ears, and knuckles.
    #[serde(with = "crate::plan::scaled")]
    pub blush: f32,
    /// Freckle density.
    #[serde(with = "crate::plan::scaled")]
    pub freckles: f32,
    /// Stubble across the lower face.
    #[serde(with = "crate::plan::scaled")]
    pub stubble: f32,
}

impl Default for SkinParams {
    fn default() -> Self {
        Self {
            melanin: 0.35,
            undertone: 0.0,
            blush: 0.45,
            freckles: 0.0,
            stubble: 0.0,
        }
    }
}

impl SkinParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.melanin = quantize(clamp_unit(self.melanin, 0.0));
        self.blush = quantize(clamp_unit(self.blush, 0.0));
        self.freckles = quantize(clamp_unit(self.freckles, 0.0));
        self.stubble = quantize(clamp_unit(self.stubble, 0.0));
        self.undertone = quantize(if self.undertone.is_finite() {
            self.undertone.clamp(-1.0, 1.0)
        } else {
            0.0
        });
    }

    /// The base complexion this melanin and undertone describe, in sRGB.
    #[must_use]
    pub fn base_tone(&self) -> Vec3 {
        let along = self.melanin.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32;
        let stop = (along.floor() as usize).min(RAMP.len() - 2);
        let blend = along - stop as f32;
        let tone = Vec3::from_array(RAMP[stop]).lerp(Vec3::from_array(RAMP[stop + 1]), blend);

        // Undertone rotates the hue and nothing else: cool skin turns toward
        // pink, warm skin toward olive, and neither gets lighter, darker or
        // more saturated on the way. See [`UNDERTONE_DEGREES`] for the two
        // simpler formulations that both failed.
        rotate_hue(tone, UNDERTONE_DEGREES * self.undertone.clamp(-1.0, 1.0))
    }
}

/// Turns an sRGB colour `degrees` around the hue circle, keeping its value and
/// its saturation.
///
/// Round-tripped through HSV rather than approximated with a channel matrix.
/// The matrix forms of a hue rotation are cheap because they do not renormalise,
/// and what they do instead is drift saturation — which is the one thing a skin
/// tone cannot afford, since an over-saturated dark tone is exactly the garish
/// orange this replaced. This runs once per body.
fn rotate_hue(colour: Vec3, degrees: f32) -> Vec3 {
    let max = colour.max_element();
    let min = colour.min_element();
    let range = max - min;
    if range <= f32::EPSILON || max <= f32::EPSILON {
        // A neutral has no hue to turn.
        return colour;
    }

    let hue = if max == colour.x {
        60.0 * (((colour.y - colour.z) / range).rem_euclid(6.0))
    } else if max == colour.y {
        60.0 * ((colour.z - colour.x) / range + 2.0)
    } else {
        60.0 * ((colour.x - colour.y) / range + 4.0)
    };
    // Rebuilt from the same `max` and `range` it was taken apart with, so both
    // value and saturation (`range / max`) survive the trip by construction
    // rather than by being carried and reapplied.
    let hue = (hue + degrees).rem_euclid(360.0);
    let sector = hue / 60.0;
    let fall = range * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (range, fall, 0.0),
        1 => (fall, range, 0.0),
        2 => (0.0, range, fall),
        3 => (0.0, fall, range),
        4 => (fall, 0.0, range),
        _ => (range, 0.0, fall),
    };
    let base = max - range;
    Vec3::new(r + base, g + base, b + base).clamp(Vec3::ZERO, Vec3::ONE)
}

/// Clamps to `0..=1`, substituting `fallback` for a non-finite value.
fn clamp_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

/// What the body under the skin is made of.
///
/// **The composites reach the painter through this and not directly** (#165).
/// Every other consumer of the record's high-level axes has a derived read of
/// its own — the cage has `plan::derive`, the skull has `face::Dimorphism` —
/// and the skin wants the same thing for the same reason: what a painter needs
/// is not a body-fat fraction, it is whether the muscle under this texel shows,
/// and the two are not the same question on a masculine and a feminine frame.
///
/// Everything here is derived. No axis was added to the record for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Condition {
    /// How sharply the muscle under the skin reads, `0` where a body carries
    /// enough fat to hide it and `1` at the leanest a record can describe.
    ///
    /// **The threshold it is measured from is dimorphic, and that is the whole
    /// derivation** (#165). `plan::BODY_FAT_RANGE` already records the bands
    /// this comes from: visible definition below about 10% of body fat on men
    /// and about 18% on women. Those are not two opinions about one number,
    /// they are essential fat differing by sex — so the same 0.16 fraction is a
    /// lean woman and a soft man, and a definition signal that read the
    /// fraction alone would have painted them the same.
    ///
    /// Provenance: **derived** from `BODY_FAT_RANGE`'s own looked-up bands
    /// (#165), interpolated by `femininity` the way every other cross-composite
    /// term in the crate is.
    pub definition: f32,
    /// How far into the changes of age this body is, straight off
    /// [`Composites::ageing`].
    ///
    /// The skin's share of #167: an old skin creases where a young one does
    /// not, and it is drier.
    pub ageing: f32,
}

impl Condition {
    /// Reads the composites into what the painter needs.
    #[must_use]
    pub fn of(composites: &Composites) -> Self {
        use crate::plan::derive::humanoid::between;

        // The lean end of the axis, which is where definition is total: a body
        // cannot be leaner than the record allows, so the ramp is measured from
        // the threshold down to that floor rather than down to zero fat.
        let floor = crate::plan::BODY_FAT_RANGE.0;
        let shows = between(composites.femininity.clamp(-1.5, 1.5), 0.10, 0.18);
        let definition = ((shows - composites.body_fat) / (shows - floor)).clamp(0.0, 1.0);
        Self {
            definition,
            ageing: composites.ageing(),
        }
    }
}

impl Default for Condition {
    /// The body the painter drew before it could read a composite: a middling
    /// adult at the default fat fraction, which is well above every definition
    /// threshold, and under the age pivot.
    fn default() -> Self {
        Self::of(&Composites::default())
    }
}

/// How much of the crease ink definition may add, at the middle of the fold
/// measure.
///
/// **A gain on a BAND rather than on the signal** (#165, #158). `CREASE_INK`
/// was cut to 0.30 because saturated folds painted as ink ladders, and the
/// obvious way to make a lean body read — raising the ink — is the exact change
/// that was reverted. So this multiplies a hump that is zero at both ends of
/// the fold measure and one in the middle: a saturated artifact row cannot be
/// made darker by any body composition, and a shallow fold that a lean body
/// would actually show gets the whole of it.
///
/// **This one IS visible to the render**, unlike the two relief constants
/// below it: a crease multiplies the colour as well as the occlusion, so the
/// ink lands in the albedo the contact sheet samples (#177).
///
/// Provenance: **tuned by render** (#165), against the lean end of the body-fat
/// sweep at a fixed light.
const DEFINITION_INK: f32 = 0.55;
/// How much less rough taut skin over a muscle belly is.
///
/// Small: skin is skin, and this is the difference between a highlight that
/// finds an edge and one that does not.
///
/// **NOT judged by render, and it cannot be** (#177): `examples/render` samples
/// the albedo of this atlas and nothing else, so every roughness term in this
/// file — including `roughness_bias`, which has shipped for months — has never
/// been drawn. Sized instead against the span of the roughness axis it moves,
/// which is 0.25 to 0.85, and left deliberately at a sixteenth of it until an
/// instrument exists to look.
///
/// Provenance: **sized against the axis it moves** (#165).
const DEFINITION_SHEEN: f32 = 0.05;
/// Relief the striation grain adds over a muscle group at full definition, as a
/// share of what the pores add.
///
/// **Gated to the muscle zones and to the same resolvability fade the pores
/// take.** A striation is a 15 mm feature and most charts carry 3–5 mm of body
/// per texel, so it is inside what the atlas can hold — but only just, and the
/// fade is what keeps it from becoming per-texel noise on the finely-packed
/// charts (#158).
///
/// **NOT judged by render** (#177): this rides the height field, which becomes
/// the normal map, and the contact sheet reads only the albedo. Sized instead
/// against the field it sits beside — at a typical body chart the pore term
/// lands near 0.31 of the height range after its own resolvability fade, and
/// this is 0.45 at full definition, so a defined muscle carries about half
/// again the relief of the skin over it and no more.
///
/// Provenance: **sized against the pore field's own amplitude** (#165).
const STRIATION: f32 = 0.45;
/// How much of the crease ink age may add, on the same band as
/// [`DEFINITION_INK`].
///
/// Larger than the definition gain, because a fold is what age does to skin
/// most directly, and taken on the same hump for the same reason.
///
/// Visible to the render for the reason [`DEFINITION_INK`] gives.
///
/// Provenance: **tuned by render** (#167), on `--head --bare` at `--age` 18
/// against 80, where the change it carries is a low-contrast field over the
/// cheek and jaw — 5.3 grey levels mean over the quarter of the sheet it
/// touches — rather than a drawn line.
const AGE_INK: f32 = 0.85;
/// How much rougher old skin is.
///
/// Sebum production falls over a life and the surface loses its sheen. The
/// roughness axis in this painter spans 0.25 to 0.85, so this is about a
/// twelfth of it end to end.
///
/// **NOT judged by render** — see [`DEFINITION_SHEEN`] and #177.
///
/// Provenance: **looked up for the direction, sized against the axis it
/// moves** (#167).
const AGE_DRY: f32 = 0.05;
/// Relief the wrinkle grain adds at the top of the age ramp, as a share of what
/// the pores add.
///
/// **Where a face wrinkles, not everywhere.** Wrinkles are a photo-ageing and
/// expression phenomenon before they are a whole-body one: the face, the neck
/// and the backs of the hands carry them decades before covered skin does, and
/// a body-wide wrinkle field reads as dirt rather than as age.
///
/// **NOT judged by render** (#177), for the reason [`STRIATION`] gives. Sized
/// the same way: about three times the pore relief on a finely-charted face,
/// where the wrinkle's 6.7 mm period is inside what the atlas resolves and the
/// pore's 2.5 mm one is not.
///
/// Provenance: **looked up for where, sized against the pore field for how
/// much** (#167).
const WRINKLE: f32 = 0.9;

/// The share of a composite's crease gain a fold of this depth may take.
///
/// Zero at a flat texel, zero at a saturated one and one in the middle. The
/// second of those is the load-bearing end: `CREASE_INK` exists because
/// saturated folds painted as ink ladders (#158), so no body composition may
/// darken one.
fn crease_band(crease: f32) -> f32 {
    4.0 * crease * (1.0 - crease)
}

/// Whether a zone is one whose muscle can show through at low body fat.
///
/// The limbs and the chest. The abdomen is deliberately out: it is where the
/// last of the fat leaves, so a defined belly is a body-fat statement the
/// definition ramp is already making at the waist's own girth, and painting
/// striations onto it reads as a drawn-on six-pack.
fn defined(zone: Zone) -> bool {
    matches!(zone, Zone::Chest | Zone::UpperLimb(_) | Zone::LowerLimb(_))
}

/// Whether a zone wrinkles with age.
///
/// See [`WRINKLE`]: the exposed skin, which is the face, the neck and the
/// hands and feet.
fn wrinkled(zone: Zone) -> bool {
    matches!(zone, Zone::Head | Zone::Neck | Zone::Extremity(_))
}

/// Paints skin over a baked body.
///
/// Returns a [`TextureMap`] — the same container every other symbios generator
/// produces, so an avatar's skin travels through the existing image-conversion
/// and upload path unchanged.
///
/// The result is *not* tileable and its size is fixed by `geometry`, which is
/// why this is a plain function rather than a
/// [`symbios_texture::generator::TextureGenerator`]: that trait promises to
/// generate at any requested size, and a baked atlas cannot honour it.
#[must_use]
pub fn paint_skin(
    geometry: &AtlasGeometry,
    rig: &Rig,
    params: &SkinParams,
    condition: &Condition,
) -> TextureMap {
    let count = geometry.texels.len();
    let mut albedo = vec![0u8; count * 4];
    let mut orm = vec![0u8; count * 4];
    let mut heights = vec![0.0f64; count];

    let base = params.base_tone();
    // The body's thickest point, which every other radius is read against.
    let stoutest = rig
        .joints
        .iter()
        .map(|joint| joint.radius)
        .fold(f32::MIN_POSITIVE, f32::max);
    // Distinct seeds so the freckle field and the skin's own mottling do not
    // line up with each other.
    let mottle = Simplex::new(11);
    let pores = Simplex::new(23);
    let freckle_field = Simplex::new(37);
    let stubble_field = Simplex::new(53);
    // Two more fields with seeds of their own, for the same reason the four
    // above have them: a grain that lines up with the pores is one grain at
    // twice the amplitude.
    let striation_field = Simplex::new(71);
    let wrinkle_field = Simplex::new(89);
    // How much of a fold's ink the body's own composition adds, at the middle
    // of the fold measure. One number for both terms, because they are the
    // same physical claim — this skin holds a crease the neutral body does not
    // — and they are additive rather than multiplicative so a lean eighty-year
    // -old is not creased twice.
    let ink_gain = DEFINITION_INK * condition.definition + AGE_INK * condition.ageing;

    for (index, sample) in geometry.texels.iter().enumerate() {
        let Some(texel) = sample else { continue };
        let p = texel.position;

        let mut colour = base;

        // Large-scale mottling keeps a broad expanse of skin from reading flat.
        colour *= 1.0 + noise3(&mottle, p, 9.0) * 0.035;

        // Subdermal blood, from how thin the flesh is rather than from which
        // zone this is. A per-zone weight steps abruptly where two zones meet,
        // and a visible seam in skin tone across a jaw or a wrist is far worse
        // than the slight loss of control. Thinness is smooth everywhere and
        // means the same thing anatomically: less flesh between blood and air.
        // How much the surface folds back on itself here, measured from the
        // geometry at bake time. It used to be derived from the nearest bone,
        // which is only meaningful where the surface was swept around one — an
        // attached hand, nose or ear sits past the end of the nearest bone and
        // read as a full crease everywhere, darkening every one of them by up
        // to 35% (#63).
        let hit = rig.nearest_bone(p);
        // **The band, and it is the whole of #158's caution honoured** — see
        // [`DEFINITION_INK`]. `4c(1−c)` is zero at a flat texel and zero at a
        // saturated fold, so nothing a composite can do reaches the rows that
        // painted as ladders, and a half-strength fold gets the full gain.
        let band = crease_band(texel.crease);
        let crease = texel.crease * CREASE_INK * (1.0 + ink_gain * band);
        let thinness = (1.0 - hit.radius / stoutest).clamp(0.0, 1.0).powf(0.7);
        // Melanin sits above the blood and absorbs what would have shown
        // through it, so the same blush axis has to mean less on deeper skin —
        // held at full strength it painted a red cheek onto a complexion that
        // physically cannot have one. Not all the way to nothing at the deepest
        // end, because deep skin does still warm where it is thin.
        let showing = 1.0 - 0.75 * params.melanin.clamp(0.0, 1.0);
        let blood = params.blush
            * showing
            * (0.25 + 0.95 * thinness)
            * (0.7 + 0.3 * noise3(&mottle, p, 6.0).abs())
            * (1.0 + crease * 0.8);
        colour *= Vec3::ONE + SUBDERMAL_TINT * blood.clamp(0.0, 0.9);

        // Cavity shading. Creases are darker because less light reaches them.
        colour *= 1.0 - crease * 0.35;

        // Freckles: thresholded so they read as discrete spots rather than a
        // haze, and only where sun reaches.
        if params.freckles > 0.0 && freckled(texel.zone) {
            let spots = noise3(&freckle_field, p, 120.0);
            let threshold = 1.0 - params.freckles * 0.55;
            if spots > threshold {
                let strength = ((spots - threshold) / (1.0 - threshold)).clamp(0.0, 1.0);
                colour = colour.lerp(colour * Vec3::new(0.62, 0.45, 0.34), strength * 0.75);
            }
        }

        // Stubble sits on the lower face only, and darkens without reddening.
        let stubble = if params.stubble > 0.0 {
            stubble_mask(rig, texel.zone, p) * params.stubble
        } else {
            0.0
        };
        if stubble > 0.0 {
            let grain = noise3(&stubble_field, p, 260.0) * 0.5 + 0.5;
            let shade = colour * Vec3::new(0.55, 0.55, 0.60);
            colour = colour.lerp(shade, stubble * grain * 0.8);
        }

        // The mouth's interior (#154), keyed by the surgery's own channel
        // rather than by anything inferred: the cavity goes to a deep warm
        // shadow — a mouth is blood over darkness — and the teeth ridge to an
        // ivory that no complexion axis touches. The two ends of one scalar,
        // so a texel between them blends gum-ward rather than banding.
        if texel.mouth > 0.05 {
            let toward = |edge: f32, width: f32| -> f32 {
                let t = ((texel.mouth - edge) / width).clamp(0.0, 1.0);
                t * t * (3.0 - 2.0 * t)
            };
            // The cavity's onset sits just past the teeth's own 0.5 and
            // rises steeply: the stretch membrane behind the ridge — the one
            // face of a split-territory pocket that must stretch — runs 0.5
            // to 1.0 across its height, and with a lazy onset it painted as a
            // light wall filling the open mouth (#156). A throat is dark from
            // the first centimetre.
            let cavity = toward(0.55, 0.18);
            let ridge = toward(0.15, 0.2) * (1.0 - cavity);
            let dark = Vec3::new(0.23, 0.08, 0.07);
            let ivory = Vec3::new(0.87, 0.84, 0.74);
            colour = colour.lerp(dark, cavity).lerp(ivory, ridge * 0.85);
        }

        // Micro-relief. Skin detail is mostly specular, so it belongs in the
        // normal and roughness maps rather than the albedo.
        //
        // **Faded by what the atlas can resolve.** Pores at 400 cycles per
        // metre have a 2.5 mm period, and most charts carry 3–5 mm of body
        // per texel — past Nyquist, so the sampled field was per-texel random.
        // Turned into normals that drew as a structured herringbone of wrong
        // directions across every grazing-lit surface, worst on the jaw flank
        // whose region packs finely-meshed skin into few texels (#158). The
        // frequency cannot vary per texel — `noise3` scales the domain, so a
        // varying frequency warps the field and rebuilds the same noise — but
        // the amplitude can: scale by the share of the pore period the local
        // texel spacing resolves, the same fade a mip chain applies. Spacing
        // is the widest step to a neighbouring texel, which sees through a
        // dilated texel's zero-distance clones to the real interior spacing.
        let spacing = {
            let x = (index as u32) % geometry.width;
            let y = (index as u32) / geometry.width;
            let mut widest = 0.0f32;
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                if nx < 0 || ny < 0 {
                    continue;
                }
                if let Some(near) = geometry.get(nx as u32, ny as u32) {
                    widest = widest.max((near.position - p).length());
                }
            }
            widest
        };
        // Hoisted into a closure by #165, which adds two more fields at two
        // more frequencies. The pore case is unchanged: `resolvable(400.0)` is
        // the expression that stood here.
        let resolvable = |cycles: f32| -> f32 {
            if spacing > f32::EPSILON {
                (1.0 / (2.0 * spacing * cycles)).clamp(0.0, 1.0)
            } else {
                1.0
            }
        };
        let pore = noise3(&pores, p, 400.0) * resolvable(400.0);
        // A muscle belly's own grain, on the zones that have one, and a life's
        // creasing on the skin that shows it. Both ride in the height map
        // rather than the albedo, because both are relief: what makes a lean
        // arm read is the light finding an edge, not a darker arm.
        let striation = if condition.definition > 0.0 && defined(texel.zone) {
            noise3(&striation_field, p, 65.0) * resolvable(65.0) * condition.definition * STRIATION
        } else {
            0.0
        };
        let wrinkle = if condition.ageing > 0.0 && wrinkled(texel.zone) {
            noise3(&wrinkle_field, p, 150.0) * resolvable(150.0) * condition.ageing * WRINKLE
        } else {
            0.0
        };
        heights[index] = f64::from(pore) * 0.35
            + f64::from(striation + wrinkle) * 0.35
            + f64::from(stubble) * f64::from(grain_bias(pore));

        // Roughness: an oily face against calloused hands, plus creases holding
        // moisture. Metallic is zero — skin is a dielectric.
        let roughness = (0.52 + roughness_bias(texel.zone) - blood * 0.12 + stubble * 0.15
            - crease * 0.05
            - DEFINITION_SHEEN * condition.definition * f32::from(defined(texel.zone))
            + AGE_DRY * condition.ageing)
            .clamp(0.25, 0.85);

        let at = index * 4;
        let rgb = colour.clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        albedo[at] = rgb.x as u8;
        albedo[at + 1] = rgb.y as u8;
        albedo[at + 2] = rgb.z as u8;
        albedo[at + 3] = 255;

        // ORM: occlusion, roughness, metallic.
        orm[at] = ((1.0 - crease * 0.7) * 255.0) as u8;
        orm[at + 1] = (roughness * 255.0) as u8;
        orm[at + 2] = 0;
        orm[at + 3] = 255;
    }

    TextureMap {
        albedo,
        // Clamped, not wrapped: an atlas has edges, and wrapping would fold the
        // far side of the texture into the near one's gradients.
        normal: height_to_normal(
            &heights,
            geometry.width,
            geometry.height,
            1.6,
            BoundaryMode::Clamp,
        ),
        roughness: orm,
        emissive: None,
        width: geometry.width,
        height: geometry.height,
        mip_level_count: 1,
    }
}

/// Samples 3-D noise at `frequency` cycles per metre, in `-1..=1`.
fn noise3(field: &Simplex, point: Vec3, frequency: f32) -> f32 {
    let scaled = point * frequency;
    field.get([
        f64::from(scaled.x),
        f64::from(scaled.y),
        f64::from(scaled.z),
    ]) as f32
}

/// Whether a part of the body ever freckles.
fn freckled(zone: Zone) -> bool {
    matches!(
        zone,
        Zone::Head | Zone::Neck | Zone::Chest | Zone::Extremity(_) | Zone::UpperLimb(_)
    )
}

/// How much rougher than average a part of the body is.
fn roughness_bias(zone: Zone) -> f32 {
    match zone {
        Zone::Extremity(_) => 0.12,
        Zone::LowerLimb(_) => 0.06,
        Zone::Head => -0.08,
        _ => 0.0,
    }
}

/// Stubble coverage at a point: the lower half of the head, and only in front.
fn stubble_mask(rig: &Rig, zone: Zone, point: Vec3) -> f32 {
    if zone != Zone::Head {
        return 0.0;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return 0.0;
    };
    let joint = rig.joints[head];
    let below = ((joint.position.y - point.y) / joint.radius.max(1e-4)).clamp(0.0, 1.0);
    let front = ((point.z - joint.position.z) / joint.radius.max(1e-4)).clamp(0.0, 1.0);
    (below * 1.3).min(1.0) * (0.35 + 0.65 * front)
}

/// Extra relief stubble adds, beyond the skin's own pores.
fn grain_bias(pore: f32) -> f32 {
    0.6 + 0.4 * pore.abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::{SkinConfig, skin};
    use crate::subdiv::catmull_clark;
    use crate::texture::bake::{Texel, bake_geometry};
    use crate::uv::{UvConfig, unwrap};

    fn painted(params: &SkinParams) -> (AtlasGeometry, Rig, TextureMap) {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let geometry = bake_geometry(&mesh, &uv, 128);
        let map = paint_skin(&geometry, &rig, params, &Condition::default());
        (geometry, rig, map)
    }

    /// Mean albedo over the texels the body actually covers.
    fn mean_albedo(geometry: &AtlasGeometry, map: &TextureMap) -> Vec3 {
        let mut total = Vec3::ZERO;
        let mut count = 0.0f32;
        for (index, sample) in geometry.texels.iter().enumerate() {
            if sample.is_none() {
                continue;
            }
            let at = index * 4;
            total += Vec3::new(
                f32::from(map.albedo[at]),
                f32::from(map.albedo[at + 1]),
                f32::from(map.albedo[at + 2]),
            );
            count += 1.0;
        }
        total / count.max(1.0)
    }

    #[test]
    fn the_map_is_the_right_shape_for_the_atlas() {
        let (geometry, _, map) = painted(&SkinParams::default());
        assert_eq!(map.width, geometry.width);
        assert_eq!(map.height, geometry.height);
        assert_eq!(map.albedo.len(), map.base_len());
        assert_eq!(map.roughness.len(), map.base_len());
        assert_eq!(map.normal.len(), map.base_len());
        assert!(map.emissive.is_none(), "skin does not glow");
    }

    #[test]
    fn melanin_darkens_the_whole_body() {
        let pale = SkinParams {
            melanin: 0.05,
            ..Default::default()
        };
        let deep = SkinParams {
            melanin: 0.95,
            ..Default::default()
        };
        let (geometry, _, light) = painted(&pale);
        let (_, _, dark) = painted(&deep);
        assert!(
            mean_albedo(&geometry, &light).element_sum()
                > mean_albedo(&geometry, &dark).element_sum() * 1.8,
            "melanin should move the tone substantially"
        );
    }

    /// Hue in degrees, saturation and value, for a tone in `0..=1` sRGB.
    fn hsv(c: Vec3) -> (f32, f32, f32) {
        let (max, min) = (c.max_element(), c.min_element());
        let range = max - min;
        let hue = if range <= f32::EPSILON {
            0.0
        } else if max == c.x {
            60.0 * ((c.y - c.z) / range).rem_euclid(6.0)
        } else if max == c.y {
            60.0 * ((c.z - c.x) / range + 2.0)
        } else {
            60.0 * ((c.x - c.y) / range + 4.0)
        };
        (hue, if max <= 0.0 { 0.0 } else { range / max }, max)
    }

    #[test]
    fn undertone_turns_the_hue_and_changes_nothing_else() {
        // This test used to assert that warm skin "loses blue and gains green".
        // That was a description of the arithmetic rather than of the axis: red
        // is the largest channel in every complexion and blue the smallest, so
        // turning the hue between them moves *green* and leaves blue exactly
        // where it was. The old assertion passed only because the old shift was
        // a hand-written offset that happened to move both.
        //
        // What the axis is actually for is hue, so that is what is checked —
        // along with the two things it must not do, which is where both earlier
        // attempts failed. See `UNDERTONE_DEGREES`.
        for melanin in [0.0f32, 0.2, 0.5, 0.8, 1.0] {
            let at = |undertone: f32| {
                hsv(SkinParams {
                    melanin,
                    undertone,
                    ..Default::default()
                }
                .base_tone())
            };
            let (cool_h, cool_s, cool_v) = at(-1.0);
            let (warm_h, warm_s, warm_v) = at(1.0);

            let turned = (warm_h - cool_h + 540.0).rem_euclid(360.0) - 180.0;
            assert!(
                (turned - 2.0 * UNDERTONE_DEGREES).abs() < 1.0,
                "melanin {melanin} turned {turned}°, not {}°",
                2.0 * UNDERTONE_DEGREES
            );
            assert!(
                (warm_v - cool_v).abs() < 1e-4,
                "melanin {melanin} changed how light the skin is"
            );
            assert!(
                (warm_s - cool_s).abs() < 1e-4,
                "melanin {melanin} changed how saturated the skin is"
            );
        }
    }

    #[test]
    fn no_complexion_clips_a_channel() {
        // The failure mode of the second attempt: a swing proportional to each
        // channel drove green past 1.0 on the palest tones, which both clips and
        // silently changes the hue it was trying to preserve.
        for step in 0..=20 {
            let melanin = step as f32 / 20.0;
            for undertone in [-1.0f32, -0.5, 0.0, 0.5, 1.0] {
                let tone = SkinParams {
                    melanin,
                    undertone,
                    ..Default::default()
                }
                .base_tone();
                for channel in [tone.x, tone.y, tone.z] {
                    assert!(
                        (0.02..=0.999).contains(&channel),
                        "melanin {melanin} undertone {undertone} reached {tone:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn saturation_peaks_in_the_middle_of_the_ramp_and_falls_at_both_ends() {
        // The defect that made deep skin read as a garish orange. The ramp this
        // replaced climbed saturation monotonically to 0.56 at full melanin,
        // which is what "saturated colour at a dark value" means. Real skin
        // peaks in the deep middle and falls away either side, and that shape —
        // not any particular colour — is what the reference scale is here for.
        let saturation = |melanin: f32| {
            hsv(SkinParams {
                melanin,
                undertone: 0.0,
                ..Default::default()
            }
            .base_tone())
            .1
        };
        let peak = (0..=20)
            .map(|step| (step as f32 / 20.0, saturation(step as f32 / 20.0)))
            .fold(
                (0.0f32, 0.0f32),
                |best, at| if at.1 > best.1 { at } else { best },
            );
        assert!(
            (0.5..=0.9).contains(&peak.0),
            "saturation peaked at melanin {}, not in the deep middle",
            peak.0
        );
        assert!(
            saturation(1.0) < peak.1 * 0.85,
            "the deepest tone is {} saturated against a peak of {}; a dark tone \
             that stays this saturated is the orange this ramp replaced",
            saturation(1.0),
            peak.1
        );
        assert!(
            saturation(0.0) < peak.1 * 0.5,
            "the palest tone is nearly as saturated as the peak"
        );
    }

    #[test]
    fn the_ramp_runs_dark_monotonically() {
        // Whatever else it does, more melanin is never lighter skin. The value
        // curve is the reference's and this is the one property of it that a
        // creator would notice being wrong immediately.
        let value = |melanin: f32| {
            hsv(SkinParams {
                melanin,
                undertone: 0.0,
                ..Default::default()
            }
            .base_tone())
            .2
        };
        for step in 0..40 {
            let (here, next) = (step as f32 / 40.0, (step + 1) as f32 / 40.0);
            assert!(
                value(next) <= value(here) + 1e-4,
                "melanin {next} is lighter than {here}"
            );
        }
        assert!(value(0.0) > 0.9, "the palest tone should be pale");
        assert!(value(1.0) < 0.25, "the deepest tone should be deep");
    }

    #[test]
    fn blush_reddens_the_face_more_than_the_torso() {
        let params = SkinParams {
            blush: 1.0,
            ..Default::default()
        };
        let (geometry, _, map) = painted(&params);

        let redness = |zone: Zone| {
            let mut total = 0.0;
            let mut count = 0.0f32;
            for (index, sample) in geometry.texels.iter().enumerate() {
                let Some(texel) = sample else { continue };
                if texel.zone != zone {
                    continue;
                }
                let at = index * 4;
                let red = f32::from(map.albedo[at]);
                let green = f32::from(map.albedo[at + 1]);
                total += red - green;
                count += 1.0;
            }
            total / count.max(1.0)
        };

        assert!(
            redness(Zone::Head) > redness(Zone::Abdomen),
            "blood shows through a face more than a belly"
        );
    }

    #[test]
    fn blush_shows_through_pale_skin_more_than_deep_skin() {
        // Melanin sits above the blood and absorbs what would have shown
        // through it. Held at full strength regardless, the blush axis painted
        // a red cheek onto a complexion that physically cannot have one.
        //
        // Measured as how far the face's red runs ahead of its green, against
        // the same body with no blush at all — the difference the axis makes,
        // rather than the absolute redness, which melanin moves on its own.
        let face_warmth = |melanin: f32, blush: f32| {
            let params = SkinParams {
                melanin,
                blush,
                ..Default::default()
            };
            let (geometry, _, map) = painted(&params);
            let mut total = 0.0;
            let mut count = 0.0f32;
            for (index, sample) in geometry.texels.iter().enumerate() {
                let Some(texel) = sample else { continue };
                if texel.zone != Zone::Head {
                    continue;
                }
                let at = index * 4;
                total += f32::from(map.albedo[at]) - f32::from(map.albedo[at + 1]);
                count += 1.0;
            }
            total / count.max(1.0)
        };

        let pale = face_warmth(0.15, 1.0) - face_warmth(0.15, 0.0);
        let deep = face_warmth(0.95, 1.0) - face_warmth(0.95, 0.0);
        assert!(
            pale > deep * 1.5,
            "blush added {pale} to pale skin and {deep} to deep skin; it should \
             show through far less where there is melanin above it"
        );
        assert!(
            deep > 0.0,
            "deep skin still warms where it is thin; blush should not vanish"
        );
    }

    #[test]
    fn freckles_only_appear_where_they_are_asked_for() {
        let clear = SkinParams {
            freckles: 0.0,
            ..Default::default()
        };
        let spotted = SkinParams {
            freckles: 1.0,
            ..Default::default()
        };
        let (geometry, _, plain) = painted(&clear);
        let (_, _, marked) = painted(&spotted);

        let differing = geometry
            .texels
            .iter()
            .enumerate()
            .filter(|(_, sample)| sample.is_some())
            .filter(|(index, _)| plain.albedo[index * 4] != marked.albedo[index * 4])
            .count();
        assert!(differing > 0, "freckles must change something");
        assert!(
            differing < geometry.covered() / 2,
            "freckles are spots, not a wash: {differing} of {} texels",
            geometry.covered()
        );
    }

    #[test]
    fn creases_are_darker_and_more_occluded_than_open_skin() {
        // Driven by two texels differing in nothing but their crease, rather
        // than by whichever texels of a body happen to be creased. The earlier
        // version compared "occluded" against "open" texels of the default
        // body, and passed because the crease it read was derived from the
        // nearest bone and so was non-zero almost everywhere. Measured against
        // the geometry, that body has no cavity below the jaw at all (#63) —
        // the test was asserting a property of a defect.
        let mut geometry = AtlasGeometry {
            width: 2,
            height: 1,
            texels: vec![None; 2],
        };
        let at = Vec3::new(0.0, 1.2, 0.1);
        for (index, crease) in [0.0f32, 0.8].into_iter().enumerate() {
            geometry.texels[index] = Some(Texel {
                position: at,
                normal: Vec3::Z,
                crease,
                mouth: 0.0,
                zone: Zone::Chest,
            });
        }

        let rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        let map = paint_skin(
            &geometry,
            &rig,
            &SkinParams::default(),
            &Condition::default(),
        );

        assert!(
            map.albedo[4] < map.albedo[0],
            "a crease must be painted darker than open skin: {} against {}",
            map.albedo[4],
            map.albedo[0]
        );
        assert!(
            map.roughness[4] < map.roughness[0],
            "a crease must be more occluded than open skin: {} against {}",
            map.roughness[4],
            map.roughness[0]
        );
    }

    #[test]
    fn a_body_is_creased_only_where_it_actually_folds() {
        // The regression #63 was: crease came from the nearest bone, which only
        // means anything where the surface was swept around that bone. Anything
        // attached — a hand, a nose, an ear — sits past the end of the nearest
        // bone and read as a deep cavity over its whole surface, so the painter
        // darkened every one of them by up to 35%.
        let avatar = crate::avatar::demo().expect("a default body builds");

        let features = avatar.parts.features.as_ref().expect("a face");
        for (index, mesh) in features.meshes().enumerate() {
            let mean = mesh.crease().iter().sum::<f32>() / mesh.vertex_count() as f32;
            assert!(
                mean < 0.30,
                "facial feature {index} reads as a cavity over its whole surface: mean crease {mean:.3}"
            );
        }

        for part in avatar.parts.extremities.all() {
            let crease = part.mesh.crease();
            let mean = crease.iter().sum::<f32>() / crease.len() as f32;
            assert!(
                mean < 0.05,
                "{:?} reads as a cavity: mean crease {mean:.3}",
                part.limb
            );
        }
    }

    #[test]
    fn detail_is_continuous_across_chart_seams() {
        // The reason noise is sampled in body space: two texels that are far
        // apart in the atlas but adjacent on the body must agree. Sampling in
        // atlas space would make them unrelated, and every seam would show.
        let (geometry, rig, _) = painted(&SkinParams::default());
        let params = SkinParams::default();

        let mut pairs = 0;
        for (index, sample) in geometry.texels.iter().enumerate() {
            let Some(texel) = sample else { continue };
            // Find a texel elsewhere in the atlas covering nearly the same spot.
            let twin =
                geometry.texels[index + 1..]
                    .iter()
                    .enumerate()
                    .find_map(|(offset, other)| {
                        let other = (*other)?;
                        (other.position.distance(texel.position) < 1e-4
                            && offset > geometry.width as usize)
                            .then_some(index + 1 + offset)
                    });
            let Some(twin) = twin else { continue };

            let map = paint_skin(&geometry, &rig, &params, &Condition::default());
            assert_eq!(
                map.albedo[index * 4],
                map.albedo[twin * 4],
                "the same point on the body painted differently in two charts"
            );
            pairs += 1;
            if pairs >= 3 {
                break;
            }
        }
        assert!(pairs > 0, "no duplicated surface points found to compare");
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = SkinParams {
            melanin: 5.0,
            undertone: f32::NAN,
            blush: -2.0,
            freckles: 0.5,
            stubble: f32::INFINITY,
        };
        params.sanitize();
        assert_eq!(params.melanin, 1.0);
        assert_eq!(params.undertone, 0.0);
        assert_eq!(params.blush, 0.0);
        assert_eq!(params.stubble, 0.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn no_body_composition_can_darken_a_saturated_fold() {
        // The #158 revert, held as arithmetic. `CREASE_INK` was cut because a
        // saturated fold painted as an ink ladder, and #165 wants MORE ink on a
        // lean body — so the gain rides a band that is zero exactly where the
        // ladders were.
        assert_eq!(crease_band(0.0), 0.0);
        assert_eq!(crease_band(1.0), 0.0);
        assert!((crease_band(0.5) - 1.0).abs() < 1e-6);
        assert!(crease_band(0.25) < crease_band(0.5));
    }

    #[test]
    fn definition_is_read_against_a_dimorphic_threshold() {
        use crate::plan::{BODY_FAT_RANGE, Composites, DEFAULT_BODY_FAT};

        // The default body is well above every threshold: nothing shows.
        assert_eq!(Condition::default().definition, 0.0);
        assert_eq!(Condition::of(&Composites::default()).definition, 0.0);

        // The leanest a record can be is total definition on any frame.
        for femininity in [-1.0f32, 0.0, 1.0] {
            let leanest = Condition::of(&Composites {
                femininity,
                body_fat: BODY_FAT_RANGE.0,
                ..Composites::default()
            });
            assert_eq!(leanest.definition, 1.0, "femininity {femininity}");
        }

        // **The point of the type.** 0.14 of body fat is a lean woman and a
        // soft man, and a signal that read the fraction alone would paint them
        // the same.
        let at = |femininity: f32| {
            Condition::of(&Composites {
                femininity,
                body_fat: 0.14,
                ..Composites::default()
            })
            .definition
        };
        assert_eq!(at(-1.0), 0.0, "a masculine body at 0.14 shows nothing");
        assert!(
            at(1.0) > 0.25,
            "a feminine body at 0.14 is a quarter of the way into the ramp, not off it: {}",
            at(1.0)
        );
        // 0.14 is exactly the neutral frame's own threshold — the midpoint of
        // 0.10 and 0.18 — so the middle of the axis sits on the edge of the
        // ramp here by arithmetic rather than by coincidence, and a body a
        // point leaner is on it.
        assert_eq!(at(0.0), 0.0);
        assert!(
            Condition::of(&Composites {
                body_fat: 0.13,
                ..Composites::default()
            })
            .definition
                > 0.0
        );

        // Monotone in the fraction, which is the direction the whole term is
        // for.
        let leaner = |fat| {
            Condition::of(&Composites {
                body_fat: fat,
                ..Composites::default()
            })
            .definition
        };
        assert!(leaner(0.08) > leaner(0.12));
        assert_eq!(leaner(DEFAULT_BODY_FAT), 0.0);
    }

    #[test]
    fn the_body_under_the_skin_reaches_the_paint() {
        // The plumbing #165 was raised for, end to end: the same complexion on
        // two bodies of different composition has to paint differently, and the
        // neutral body has to paint what it painted before the axis existed.
        use crate::plan::{AGE_RANGE, BODY_FAT_RANGE, Composites};

        let params = SkinParams::default();
        let (geometry, rig, neutral) = painted(&params);
        let paint = |composites: Composites| {
            paint_skin(&geometry, &rig, &params, &Condition::of(&composites))
        };

        assert_eq!(
            neutral.roughness,
            paint(Composites::default()).roughness,
            "the neutral composites must paint the body that was already painted"
        );

        let lean = paint(Composites {
            body_fat: BODY_FAT_RANGE.0,
            ..Composites::default()
        });
        assert_ne!(
            lean.roughness, neutral.roughness,
            "definition reaches the skin"
        );

        let old = paint(Composites {
            age: AGE_RANGE.1,
            ..Composites::default()
        });
        assert_ne!(old.roughness, neutral.roughness, "age reaches the skin");
        assert_ne!(old.normal, neutral.normal, "age reaches the relief");
    }

    #[test]
    fn painting_is_deterministic() {
        let params = SkinParams::default();
        let (geometry, rig, first) = painted(&params);
        let second = paint_skin(&geometry, &rig, &params, &Condition::default());
        assert_eq!(first.albedo, second.albedo);
        assert_eq!(first.roughness, second.roughness);
    }
}
