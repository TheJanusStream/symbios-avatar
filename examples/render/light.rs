//! Turning a G-buffer into a picture.
//!
//! Three things matter far more here than any cleverness about light transport,
//! because all three are what an eye uses to decide whether it is looking at a
//! solid object:
//!
//! - **Occlusion.** Creases, the underside of a jaw, the gap between two locks
//!   of hair. Without it every form reads as inflated, and a figure looks like a
//!   balloon animal of itself.
//! - **Working in linear light.** Everything authored by eye is in sRGB, and
//!   adding two lights together in that space overbrightens the midtones and
//!   flattens every curve. This one change did more for these sheets than the
//!   rest put together.
//! - **Supersampling.** A stair-stepped silhouette reads as unfinished no matter
//!   what is inside it, and silhouette is the first thing being judged.

use glam::Vec3;

use crate::scene::{Frame, GBuffer, ShadowMap};

/// Which way the key light comes from, in the camera's frame.
///
/// Well off the camera's axis, which is the whole game. A key sitting near the
/// lens is a flash: an orthographic view of a limb shows the same normal over
/// almost its entire width, so a frontal key lights all of it equally and the
/// form disappears. Every gradient that tells an eye "this is a cylinder" comes
/// from the light being somewhere the camera is not.
pub const KEY: Vec3 = Vec3::new(-0.82, 0.46, 0.34);

/// How many rays the occlusion pass casts per pixel.
const OCCLUSION_SAMPLES: usize = 14;

/// How far occlusion reaches, as a share of the frame's span.
const OCCLUSION_REACH: f32 = 0.055;

/// A finished picture.
pub struct Image {
    /// Pixels across.
    pub width: usize,
    /// Pixels down.
    pub height: usize,
    /// Linear colour.
    pub colour: Vec<Vec3>,
}

impl Image {
    /// A blank image.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            colour: vec![Vec3::ZERO; width * height],
        }
    }

    /// Copies another image in at an offset.
    pub fn blit(&mut self, other: &Image, x: usize, y: usize) {
        for row in 0..other.height {
            for column in 0..other.width {
                let (to_x, to_y) = (x + column, y + row);
                if to_x < self.width && to_y < self.height {
                    self.colour[to_y * self.width + to_x] =
                        other.colour[row * other.width + column];
                }
            }
        }
    }

    /// Encodes to sRGB bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.colour.len() * 4);
        for colour in &self.colour {
            for channel in [colour.x, colour.y, colour.z] {
                out.push((to_srgb(channel) * 255.0).round().clamp(0.0, 255.0) as u8);
            }
            out.push(255);
        }
        out
    }
}

/// How much of the sky each pixel can see, in `0..=1`.
///
/// Cast in world space rather than screen space: the sample points are real
/// points near the surface, projected to see whether something nearer is in the
/// way. Screen-space variants are cheaper and produce haloes around every
/// silhouette, which on a figure means a bright outline exactly where the eye is
/// judging the shape.
pub fn occlusion(buffer: &GBuffer, frame: &Frame) -> Vec<f32> {
    let camera = frame.camera();
    let reach = frame.span * OCCLUSION_REACH;
    let mut ao = vec![1.0f32; buffer.width * buffer.height];

    for y in 0..buffer.height {
        for x in 0..buffer.width {
            let pixel = y * buffer.width + x;
            if !buffer.covered[pixel] {
                continue;
            }
            let at = buffer.world[pixel];
            let normal = buffer.normal[pixel];
            let (along, across) = basis(normal);
            // A different rotation per pixel, so the kernel's own pattern
            // becomes noise rather than a visible weave. The blur afterwards
            // takes the noise back out.
            let spin = hash(x, y) * std::f32::consts::TAU;

            let mut hidden = 0.0;
            for step in 0..OCCLUSION_SAMPLES {
                let ray = kernel(step, spin);
                let sample = at
                    + (along * ray.x + across * ray.y + normal * ray.z) * reach
                    + normal * (reach * 0.02);

                let (screen, depth) = buffer.project(&camera, sample);
                let (sx, sy) = (screen.x as isize, screen.y as isize);
                if sx < 0 || sy < 0 || sx >= buffer.width as isize || sy >= buffer.height as isize {
                    continue;
                }
                let other = sy as usize * buffer.width + sx as usize;
                if !buffer.covered[other] {
                    continue;
                }
                if buffer.depth[other] < depth {
                    // Only count what is close enough to be a neighbour; a wall
                    // across the room is not shading this crease.
                    let gap = (buffer.world[other] - at).length();
                    hidden += (1.0 - (gap / reach).min(1.0)).powf(0.6);
                }
            }
            let open = 1.0 - hidden / OCCLUSION_SAMPLES as f32;
            ao[pixel] = open.clamp(0.0, 1.0).powf(2.0);
        }
    }

    blur(&ao, buffer.width, buffer.height)
}

/// Lights the buffer.
pub fn shade(buffer: &GBuffer, ao: &[f32], frame: &Frame, shadow: &ShadowMap) -> Image {
    // Written in the camera's frame, so every view of a sheet is lit the same.
    let key = frame.to_world(KEY.normalize());
    let fill = frame.to_world(Vec3::new(0.72, -0.12, 0.68).normalize());
    let back = frame.to_world(Vec3::new(0.38, 0.50, -0.78).normalize());
    let eye = frame.to_world(Vec3::Z);

    // The ratio between key and ambient is what decides whether a body has
    // form. Ambient here is a hemisphere, so it varies only with how far a
    // surface tilts up or down — which for an upright torso means it does not
    // vary at all. Push it up to soften the shadows and every vertical surface
    // flattens to one even colour, whatever else is done to the picture.
    let key_colour = Vec3::new(1.0, 0.95, 0.88) * 2.10;
    let fill_colour = Vec3::new(0.55, 0.66, 0.90) * 0.22;
    let back_colour = Vec3::new(0.82, 0.88, 1.0) * 0.70;
    let sky = Vec3::new(0.36, 0.44, 0.58) * 0.42;
    let ground = Vec3::new(0.24, 0.20, 0.17) * 0.42;

    let mut image = Image::new(buffer.width, buffer.height);
    for (pixel, &open) in ao.iter().enumerate().take(buffer.colour_len()) {
        if !buffer.covered[pixel] {
            image.colour[pixel] = backdrop(pixel, buffer.width, buffer.height);
            continue;
        }

        let normal = buffer.normal[pixel];
        let albedo = buffer.albedo[pixel];
        let finish = buffer.finish[pixel];
        let (roughness, specular, wrap) = (finish.x, finish.y, finish.z);

        // Hemisphere ambient: what a surface sees is sky above and floor below.
        let dome = ground.lerp(sky, 0.5 + 0.5 * normal.y) * open;

        let mut lit = albedo * dome;
        // Never fully dark. A real shadow is filled by light bouncing off
        // everything around it, and a shadow rendered as an absence of light is
        // a silhouette cut out of the picture.
        let cast = 0.14
            + 0.86
                * shadow.lit(
                    buffer.world[pixel],
                    normal,
                    (1.0 - normal.dot(key).abs()).clamp(0.0, 1.0),
                );
        for (direction, colour, occludes, shadowing) in [
            (key, key_colour, true, cast),
            (fill, fill_colour, false, 1.0),
            (back, back_colour, true, 1.0),
        ] {
            let raw = normal.dot(direction);
            // Wrapped diffuse. Skin is not opaque, and a clean terminator on a
            // face is most of what makes it read as painted plastic.
            let level = ((raw + wrap) / (1.0 + wrap)).max(0.0);
            let shadowed = shadowing
                * if occludes {
                    level * (0.30 + 0.70 * open)
                } else {
                    level
                };
            lit += albedo * colour * shadowed;

            // Blinn-Phong, which is enough at this size and does not need a
            // full microfacet model to say "this is glossy and that is cloth".
            let half = (direction + eye).normalize_or(normal);
            let sharpness = 2.0 / (roughness * roughness * roughness * roughness + 1e-4) - 2.0;
            let highlight = normal.dot(half).max(0.0).powf(sharpness.clamp(1.0, 4096.0));
            lit += colour * (highlight * specular * level.min(1.0) * shadowing * 0.35);
        }

        // A grazing sheen, which separates a body from its background without
        // drawing an outline around it.
        let facing = 1.0 - normal.dot(eye).abs().clamp(0.0, 1.0);
        lit += back_colour * (facing.powf(4.0) * 0.16 * open);

        image.colour[pixel] = tonemap(lit);
    }
    image
}

/// A raw buffer, drawn as an image.
///
/// When a picture comes out wrong the useful question is which stage produced
/// the wrongness, and that is not answerable from the finished frame. Shading is
/// four passes stacked on each other, and any of them can be silently doing
/// nothing at all.
pub fn inspect(
    buffer: &GBuffer,
    ao: &[f32],
    frame: &Frame,
    shadow: &ShadowMap,
    pass: &str,
) -> Image {
    let mut image = Image::new(buffer.width, buffer.height);
    let key = frame.to_world(KEY.normalize());
    for (pixel, &open) in ao.iter().enumerate() {
        let covered = buffer.covered[pixel];
        image.colour[pixel] = match pass {
            "ao" => Vec3::splat(if covered { open } else { 0.0 }),
            "normal" => {
                if covered {
                    buffer.normal[pixel] * 0.5 + Vec3::splat(0.5)
                } else {
                    Vec3::ZERO
                }
            }
            "albedo" => {
                if covered {
                    buffer.albedo[pixel]
                } else {
                    Vec3::ZERO
                }
            }
            // The finish's roughness channel, which was one flat grey for the
            // whole life of the tool: the material constant. Per-texel since
            // #45, and this view is what makes that claim checkable.
            "roughness" => Vec3::splat(if covered { buffer.finish[pixel].x } else { 0.0 }),
            "shadow" => Vec3::splat(if covered {
                shadow.lit(
                    buffer.world[pixel],
                    buffer.normal[pixel],
                    (1.0 - buffer.normal[pixel].dot(key).abs()).clamp(0.0, 1.0),
                )
            } else {
                0.0
            }),
            _ => Vec3::splat(if covered { 1.0 } else { 0.0 }),
        };
        // Written straight out, so what is shown is what the buffer holds.
        image.colour[pixel] = to_view(image.colour[pixel]);
    }
    image
}

/// Undoes the sRGB encoding that writing applies, so a buffer shows raw.
fn to_view(value: Vec3) -> Vec3 {
    Vec3::new(
        to_linear_channel(value.x),
        to_linear_channel(value.y),
        to_linear_channel(value.z),
    )
}

fn to_linear_channel(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Averages each block of `factor` pixels down to one.
pub fn resolve(image: &Image, factor: usize) -> Image {
    if factor <= 1 {
        return Image {
            width: image.width,
            height: image.height,
            colour: image.colour.clone(),
        };
    }
    let (width, height) = (image.width / factor, image.height / factor);
    let mut out = Image::new(width, height);
    let share = 1.0 / (factor * factor) as f32;
    for y in 0..height {
        for x in 0..width {
            let mut sum = Vec3::ZERO;
            for row in 0..factor {
                for column in 0..factor {
                    sum += image.colour[(y * factor + row) * image.width + x * factor + column];
                }
            }
            out.colour[y * width + x] = sum * share;
        }
    }
    out
}

impl GBuffer {
    /// How many pixels the buffer holds.
    fn colour_len(&self) -> usize {
        self.width * self.height
    }
}

/// The empty background: a soft vertical gradient, not a flat fill.
fn backdrop(pixel: usize, width: usize, height: usize) -> Vec3 {
    let down = (pixel / width) as f32 / height.max(1) as f32;
    let top = Vec3::new(0.055, 0.062, 0.078);
    let bottom = Vec3::new(0.021, 0.024, 0.031);
    top.lerp(bottom, down * down)
}

/// Compresses high values without clipping to white.
///
/// A filmic curve rather than plain Reinhard: Reinhard rolls off so early that
/// the midtones — which is where a body's whole form lives — come out with
/// barely any separation between them.
fn tonemap(colour: Vec3) -> Vec3 {
    let curve = |x: f32| {
        let x = x.max(0.0);
        ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
    };
    Vec3::new(curve(colour.x), curve(colour.y), curve(colour.z))
}

/// Linear to sRGB.
fn to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

/// Two directions perpendicular to `normal` and to each other.
fn basis(normal: Vec3) -> (Vec3, Vec3) {
    let reference = if normal.y.abs() > 0.99 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let along = reference.cross(normal).normalize_or(Vec3::X);
    (along, normal.cross(along))
}

/// One point of the occlusion kernel, in a hemisphere about the normal.
fn kernel(step: usize, spin: f32) -> Vec3 {
    let along = (step as f32 + 0.5) / OCCLUSION_SAMPLES as f32;
    // Golden angle around, so the points never line up into spokes.
    let angle = step as f32 * 2.399_963 + spin;
    let rise = (0.15 + 0.85 * along).min(1.0);
    let radius = (1.0 - rise * rise).max(0.0).sqrt();
    // Clustered near the surface, where occlusion actually varies.
    let scale = 0.25 + 0.75 * along * along;
    Vec3::new(angle.cos() * radius, angle.sin() * radius, rise) * scale
}

/// A cheap per-pixel value in `0..1`.
fn hash(x: usize, y: usize) -> f32 {
    let mixed = (x as u32).wrapping_mul(73_856_093) ^ (y as u32).wrapping_mul(19_349_663);
    (mixed % 4096) as f32 / 4096.0
}

/// Small box blur, to take the kernel's noise back out.
fn blur(values: &[f32], width: usize, height: usize) -> Vec<f32> {
    const REACH: isize = 2;
    let mut out = vec![1.0; values.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            let mut count = 0.0;
            for dy in -REACH..=REACH {
                for dx in -REACH..=REACH {
                    let (sx, sy) = (x as isize + dx, y as isize + dy);
                    if sx < 0 || sy < 0 || sx >= width as isize || sy >= height as isize {
                        continue;
                    }
                    sum += values[sy as usize * width + sx as usize];
                    count += 1.0;
                }
            }
            out[y * width + x] = sum / count;
        }
    }
    out
}
