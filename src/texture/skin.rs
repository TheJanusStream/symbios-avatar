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
use symbios_texture::palette::CosinePalette;

use super::bake::AtlasGeometry;
use crate::plan::Zone;
use crate::rig::Rig;

/// The palest skin tone on the melanin ramp, in sRGB.
const PALE: [f32; 3] = [0.98, 0.85, 0.78];
/// The deepest skin tone on the melanin ramp, in sRGB.
const DEEP: [f32; 3] = [0.28, 0.17, 0.12];
/// How blood shifts the skin's colour where it runs close to the surface.
///
/// A *shift*, not a colour to blend toward: haemoglobin absorbs green and blue
/// and passes red, so it warms the existing tone rather than replacing it.
/// Blending toward a red target instead crushes the other channels and turns
/// every knuckle salmon-pink.
const SUBDERMAL_TINT: Vec3 = Vec3::new(0.30, -0.10, -0.13);

/// What a body's skin looks like.
///
/// Axes run `0..=1`, and every combination is a usable complexion — there is no
/// arrangement of these that produces something that reads as a mistake.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinParams {
    /// Base depth of tone, from palest to deepest.
    #[serde(default, with = "crate::plan::scaled")]
    pub melanin: f32,
    /// Hue beneath the tone: `-1` cool and pink, `+1` warm and olive.
    #[serde(default, with = "crate::plan::scaled")]
    pub undertone: f32,
    /// How much blood shows through at cheeks, ears, and knuckles.
    #[serde(default, with = "crate::plan::scaled")]
    pub blush: f32,
    /// Freckle density.
    #[serde(default, with = "crate::plan::scaled")]
    pub freckles: f32,
    /// Stubble across the lower face.
    #[serde(default, with = "crate::plan::scaled")]
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
        let ramp = CosinePalette::between(PALE, DEEP);
        let tone = Vec3::from_array(ramp.sample(self.melanin.clamp(0.0, 1.0)));
        // Undertone rotates the hue without moving the value: cool skin loses
        // green and gains blue, warm skin does the reverse.
        let shift = Vec3::new(0.0, 0.035, -0.06) * self.undertone;
        (tone + shift).clamp(Vec3::ZERO, Vec3::ONE)
    }
}

/// Clamps to `0..=1`, substituting `fallback` for a non-finite value.
fn clamp_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
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
pub fn paint_skin(geometry: &AtlasGeometry, rig: &Rig, params: &SkinParams) -> TextureMap {
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
        // How much the surface folds back on itself here.
        let hit = rig.nearest_bone(p);
        let crease = hit.crease(p, texel.normal);
        let thinness = (1.0 - hit.radius / stoutest).clamp(0.0, 1.0).powf(0.7);
        let blood = params.blush
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

        // Micro-relief. Skin detail is mostly specular, so it belongs in the
        // normal and roughness maps rather than the albedo.
        let pore = noise3(&pores, p, 400.0);
        heights[index] = f64::from(pore) * 0.35 + f64::from(stubble) * f64::from(grain_bias(pore));

        // Roughness: an oily face against calloused hands, plus creases holding
        // moisture. Metallic is zero — skin is a dielectric.
        let roughness = (0.52 + roughness_bias(texel.zone) - blood * 0.12 + stubble * 0.15
            - crease * 0.05)
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
    use crate::texture::bake::bake_geometry;
    use crate::uv::{UvConfig, unwrap};

    fn painted(params: &SkinParams) -> (AtlasGeometry, Rig, TextureMap) {
        let skeleton = HumanoidParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let geometry = bake_geometry(&mesh, &uv, 128);
        let map = paint_skin(&geometry, &rig, params);
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

    #[test]
    fn undertone_shifts_hue_without_moving_value() {
        let cool = SkinParams {
            undertone: -1.0,
            ..Default::default()
        }
        .base_tone();
        let warm = SkinParams {
            undertone: 1.0,
            ..Default::default()
        }
        .base_tone();

        assert!(warm.z < cool.z, "warm skin loses blue");
        assert!(warm.y > cool.y, "and gains green");
        let value = |c: Vec3| c.element_sum();
        assert!(
            (value(warm) - value(cool)).abs() < 0.1,
            "undertone must not change how light the skin is"
        );
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
        let (geometry, _, map) = painted(&SkinParams::default());
        let mut creased = (0.0, 0.0);
        let mut open = (0.0, 0.0);

        for (index, sample) in geometry.texels.iter().enumerate() {
            if sample.is_none() {
                continue;
            }
            let occlusion = f32::from(map.roughness[index * 4]);
            if occlusion < 200.0 {
                creased = (
                    creased.0 + f32::from(map.albedo[index * 4]),
                    creased.1 + 1.0,
                );
            } else {
                open = (open.0 + f32::from(map.albedo[index * 4]), open.1 + 1.0);
            }
        }

        assert!(creased.1 > 0.0, "the body has creases to shade");
        assert!(
            creased.0 / creased.1 < open.0 / open.1,
            "creases must be darker than open skin"
        );
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

            let map = paint_skin(&geometry, &rig, &params);
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
    fn painting_is_deterministic() {
        let params = SkinParams::default();
        let (geometry, rig, first) = painted(&params);
        let second = paint_skin(&geometry, &rig, &params);
        assert_eq!(first.albedo, second.albedo);
        assert_eq!(first.roughness, second.roughness);
    }
}
