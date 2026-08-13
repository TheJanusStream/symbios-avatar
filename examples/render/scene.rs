//! What is drawn, and how it lands in the G-buffer.
//!
//! Geometry is rasterised once into a buffer of surface properties — albedo,
//! normal, world position, how rough and how shiny — and lit afterwards. Shading
//! forward, per triangle, was what made everything on this sheet look like a
//! different program: each part carried its own ad-hoc colour rule, and nothing
//! could see its neighbours. Occlusion in particular is impossible to compute
//! while you are still drawing triangles, and occlusion is most of what tells an
//! eye that a form is solid.

use glam::{Mat4, Vec2, Vec3};

/// How a surface responds to light.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Base colour, in sRGB as authored.
    pub albedo: Vec3,
    /// `0` mirror, `1` chalk.
    pub roughness: f32,
    /// How much light bounces off the surface rather than through it.
    pub specular: f32,
    /// How far light bleeds past the terminator.
    ///
    /// Skin is not opaque: light entering it scatters and comes back out a
    /// little way around the curve, which is why a lit face has no hard edge
    /// between light and shadow. Rendering skin with a clean terminator is most
    /// of what makes it look like painted plastic.
    pub wrap: f32,
}

impl Material {
    /// Skin, or anything else soft and barely shiny.
    pub const fn skin(albedo: Vec3) -> Self {
        Self {
            albedo,
            roughness: 0.62,
            specular: 0.22,
            wrap: 0.22,
        }
    }

    /// Cloth: rough, matte, and opaque.
    pub const fn cloth(albedo: Vec3) -> Self {
        Self {
            albedo,
            roughness: 0.82,
            specular: 0.09,
            wrap: 0.04,
        }
    }

    /// Hair: a soft sheen along the locks rather than a hard gloss.
    ///
    /// Driven up to a mirror finish it stops reading as hair at all and becomes
    /// wet vinyl, which is what the first close-up showed.
    pub const fn hair(albedo: Vec3) -> Self {
        Self {
            albedo,
            roughness: 0.52,
            specular: 0.26,
            wrap: 0.08,
        }
    }

    /// A wet, glossy surface.
    pub const fn glossy(albedo: Vec3) -> Self {
        Self {
            albedo,
            roughness: 0.08,
            specular: 1.0,
            wrap: 0.0,
        }
    }
}

/// Where a surface's colour comes from.
pub enum Paint<'a> {
    /// One colour over the whole thing.
    Flat,
    /// Carried on the vertices.
    ///
    /// How a merged mesh keeps the colours its parts used to be drawn in: every
    /// lock of hair is its own shade and every garment its own dye, in one draw.
    Vertex(&'a [Vec3]),
    /// Sampled from a baked atlas.
    Atlas {
        /// Texture coordinates, one per vertex.
        uvs: &'a [Vec2],
        /// RGBA bytes.
        pixels: &'a [u8],
        /// ORM bytes beside them — occlusion, roughness, metallic — when the
        /// atlas carries a finish of its own. The G channel replaces the
        /// material's constant roughness per texel; R and B are deliberately
        /// not consumed (this renderer casts its own occlusion, and nothing on
        /// a body is metal), which is recorded here so their silence reads as
        /// a decision rather than an oversight (#45).
        orm: Option<&'a [u8]>,
        /// Tangent-space normal bytes beside them, when the atlas carries a
        /// relief of its own. Linear data, decoded `*2-1` with no sRGB pass;
        /// carried into world through a per-triangle `∂P/∂u, ∂P/∂v` frame
        /// measured from these very UVs, so the map's axis convention is
        /// honoured by construction rather than by a handedness guess (#45).
        normals: Option<&'a [u8]>,
        /// Pixels per side.
        side: u32,
    },
}

/// One thing to draw.
pub struct Item<'a> {
    /// Vertex positions, in world space.
    pub positions: &'a [Vec3],
    /// Faces over those positions.
    pub faces: &'a [Vec<u32>],
    /// Vertex normals, when the caller has better ones than this can derive.
    ///
    /// The body's are computed over its own topology and then gathered through
    /// the unwrap, because deriving them from unwrapped geometry splits every
    /// UV seam into two normals and shows as a faceted band.
    pub normals: Option<&'a [Vec3]>,
    /// How its colour is found.
    pub paint: Paint<'a>,
    /// A per-vertex multiplier over whatever [`Item::paint`] resolved to, for
    /// marking a surface up without replacing what it is.
    ///
    /// **Multiplied rather than substituted, which is the whole point of it.**
    /// A debug overlay that replaces the albedo throws away the thing being
    /// debugged: skin drawn in flat false colour loses its complexion, its
    /// atlas and every shading cue that says what shape it is. Tinting keeps
    /// the render the render and puts the classification on top of it.
    pub tint: Option<&'a [Vec3]>,
    /// How it responds to light.
    pub material: Material,
}

/// One orthographic view: where it looks from, at what, and how close.
///
/// Framing is a parameter rather than a constant because the body and the head
/// need wildly different ones. A skull is about a twelfth of a body's height, so
/// in a full-body frame it lands in a few dozen pixels — which is most of why
/// the eye bulge, the floating hair and the bald crown all survived so long.
#[derive(Clone, Copy)]
pub struct Frame {
    /// Rotation about the body's own axis; zero looks at the face.
    pub turn: f32,
    /// Tilt, in radians. Positive looks down from above.
    pub pitch: f32,
    /// The point the view is centred on, in world space.
    pub centre: Vec3,
    /// How much of the world the frame covers, in metres.
    pub span: f32,
}

impl Frame {
    /// World space to normalised device space: x and y in -1..1, z away.
    pub fn camera(&self) -> Mat4 {
        Mat4::from_scale(Vec3::new(
            2.0 / self.span,
            2.0 / self.span,
            -1.0 / self.span,
        )) * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_y(self.turn)
            * Mat4::from_translation(-self.centre)
    }

    /// Carries a direction from the camera's frame into the world's.
    ///
    /// Lights are written in the camera's frame so that every view of the sheet
    /// is lit the same way — a world-fixed key leaves the back view a flat
    /// silhouette, which is how it looked for a long time without anyone
    /// noticing there was anything to see there.
    pub fn to_world(self, direction: Vec3) -> Vec3 {
        let turn = glam::Quat::from_rotation_x(self.pitch) * glam::Quat::from_rotation_y(self.turn);
        turn.inverse() * direction
    }
}

/// Surface properties at every pixel, before anything is lit.
pub struct GBuffer {
    /// Pixels across.
    pub width: usize,
    /// Pixels down.
    pub height: usize,
    /// Depth in normalised device space; larger is further away.
    pub depth: Vec<f32>,
    /// Base colour, converted to linear.
    pub albedo: Vec<Vec3>,
    /// Surface normal, in world space.
    pub normal: Vec<Vec3>,
    /// World position.
    pub world: Vec<Vec3>,
    /// Roughness, specular strength and wrap, per pixel.
    pub finish: Vec<Vec3>,
    /// Whether anything was drawn at all.
    pub covered: Vec<bool>,
}

impl GBuffer {
    /// An empty buffer, everything at the far plane.
    pub fn new(width: usize, height: usize) -> Self {
        let pixels = width * height;
        Self {
            width,
            height,
            depth: vec![f32::MAX; pixels],
            albedo: vec![Vec3::ZERO; pixels],
            normal: vec![Vec3::Y; pixels],
            world: vec![Vec3::ZERO; pixels],
            finish: vec![Vec3::new(1.0, 0.0, 0.0); pixels],
            covered: vec![false; pixels],
        }
    }

    /// Draws every item into the buffer.
    pub fn draw(&mut self, items: &[Item], frame: &Frame) {
        let camera = frame.camera();
        for item in items {
            let derived;
            let normals = match item.normals {
                Some(given) => given,
                None => {
                    derived = smooth_normals(item.positions, item.faces);
                    &derived
                }
            };
            for face in item.faces {
                for corner in 1..face.len().saturating_sub(1) {
                    let tri = [face[0], face[corner], face[corner + 1]];
                    self.triangle(item, normals, tri, &camera);
                }
            }
        }
    }

    /// Rasterises one triangle.
    fn triangle(&mut self, item: &Item, normals: &[Vec3], tri: [u32; 3], camera: &Mat4) {
        let world = tri.map(|at| item.positions[at as usize]);
        let clip = world.map(|point| camera.transform_point3(point));
        let size = Vec2::new(self.width as f32, self.height as f32);
        let screen = clip.map(|point| {
            Vec2::new(
                (point.x * 0.5 + 0.5) * size.x,
                (0.5 - point.y * 0.5) * size.y,
            )
        });

        // Twice the screen area, in pixels. A threshold near zero is not
        // enough: a plane seen exactly edge-on has an area of about nothing but
        // not quite, and dividing by it makes the barycentrics explode and smear
        // the triangle across its whole bounding box. That is how a floor
        // rendered as a black band the width of the frame.
        let doubled = edge(screen[0], screen[1], screen[2]);
        if doubled.abs() < 1.0 {
            return;
        }

        let lo = screen[0]
            .min(screen[1])
            .min(screen[2])
            .floor()
            .max(Vec2::ZERO);
        let hi = screen[0].max(screen[1]).max(screen[2]).ceil().min(size);
        if lo.x >= hi.x || lo.y >= hi.y {
            return;
        }

        let finish = Vec3::new(
            item.material.roughness,
            item.material.specular,
            item.material.wrap,
        );

        // The tangent frame this triangle's relief lives in, measured from its
        // own UVs. `None` off the atlas — and on a UV-degenerate triangle,
        // where the honest answer is the smooth normal rather than a frame
        // divided by nothing.
        let relief = match &item.paint {
            Paint::Atlas {
                uvs,
                normals: Some(map),
                side,
                ..
            } => tangent_frame(world, tri.map(|at| uvs[at as usize]))
                .map(|frame| (frame, *map, *side)),
            _ => None,
        };

        for y in (lo.y as usize)..(hi.y as usize).min(self.height) {
            for x in (lo.x as usize)..(hi.x as usize).min(self.width) {
                let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                // Two-sided. Testing the raw edge functions against zero would
                // cull whichever winding came out negative in screen space —
                // which silently dropped the ground plane entirely, and left
                // every figure standing on nothing while the background looked
                // deliberate.
                let raw = Vec3::new(
                    edge(screen[1], screen[2], at),
                    edge(screen[2], screen[0], at),
                    edge(screen[0], screen[1], at),
                );
                if (raw * doubled.signum()).min_element() < 0.0 {
                    continue;
                }
                let mut bary = raw / doubled;
                bary /= bary.element_sum();

                let depth = clip[0].z * bary.x + clip[1].z * bary.y + clip[2].z * bary.z;
                let pixel = y * self.width + x;
                if depth >= self.depth[pixel] {
                    continue;
                }

                let point = world[0] * bary.x + world[1] * bary.y + world[2] * bary.z;
                let (albedo, rough, uv) = match &item.paint {
                    Paint::Flat => (item.material.albedo, None, None),
                    Paint::Vertex(colours) => (
                        tri.iter()
                            .zip(bary.to_array())
                            .fold(Vec3::ZERO, |sum, (&index, weight)| {
                                sum + colours[index as usize] * weight
                            }),
                        None,
                        None,
                    ),
                    Paint::Atlas {
                        uvs,
                        pixels,
                        orm,
                        side,
                        ..
                    } => {
                        let uv = tri
                            .iter()
                            .zip(bary.to_array())
                            .fold(Vec2::ZERO, |sum, (&index, weight)| {
                                sum + uvs[index as usize] * weight
                            });
                        // ORM is linear data: the `.y` IS the roughness, with
                        // no sRGB decode — only the albedo goes through
                        // `to_linear` below.
                        (
                            sample(uv, pixels, *side),
                            orm.map(|orm| sample(uv, orm, *side).y),
                            Some(uv),
                        )
                    }
                };
                let albedo = match item.tint {
                    Some(tint) => {
                        albedo
                            * tri
                                .iter()
                                .zip(bary.to_array())
                                .fold(Vec3::ZERO, |sum, (&index, weight)| {
                                    sum + tint[index as usize] * weight
                                })
                    }
                    None => albedo,
                };

                self.depth[pixel] = depth;
                // A per-texel finish overrides only the channel the atlas
                // carries; specular and wrap stay the material's.
                self.finish[pixel] = match rough {
                    Some(rough) => Vec3::new(rough, finish.y, finish.z),
                    None => finish,
                };
                self.albedo[pixel] = to_linear(albedo);
                let smooth = tri
                    .iter()
                    .zip(bary.to_array())
                    .fold(Vec3::ZERO, |sum, (&index, weight)| {
                        sum + normals[index as usize] * weight
                    })
                    .normalize_or(Vec3::Y);
                self.normal[pixel] = match (&relief, uv) {
                    (Some(((tangent, bitangent), map, side)), Some(uv)) => {
                        // Linear bytes to a signed vector; the Z stays along
                        // the smooth normal, so a flat texel changes nothing.
                        let n = sample(uv, map, *side) * 2.0 - Vec3::ONE;
                        (*tangent * n.x + *bitangent * n.y + smooth * n.z).normalize_or(smooth)
                    }
                    _ => smooth,
                };
                self.world[pixel] = point;
                self.covered[pixel] = true;
            }
        }
    }

    /// Fills only the depth buffer, for a shadow pass.
    pub fn draw_depth(&mut self, items: &[Item], camera: &Mat4) {
        let size = Vec2::new(self.width as f32, self.height as f32);
        for item in items {
            for face in item.faces {
                for corner in 1..face.len().saturating_sub(1) {
                    let tri = [face[0], face[corner], face[corner + 1]];
                    let clip = tri.map(|at| camera.transform_point3(item.positions[at as usize]));
                    let screen = clip.map(|point| {
                        Vec2::new(
                            (point.x * 0.5 + 0.5) * size.x,
                            (0.5 - point.y * 0.5) * size.y,
                        )
                    });
                    let doubled = edge(screen[0], screen[1], screen[2]);
                    if doubled.abs() < 1.0 {
                        continue;
                    }
                    let lo = screen[0]
                        .min(screen[1])
                        .min(screen[2])
                        .floor()
                        .max(Vec2::ZERO);
                    let hi = screen[0].max(screen[1]).max(screen[2]).ceil().min(size);
                    if lo.x >= hi.x || lo.y >= hi.y {
                        continue;
                    }
                    for y in (lo.y as usize)..(hi.y as usize).min(self.height) {
                        for x in (lo.x as usize)..(hi.x as usize).min(self.width) {
                            let at = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                            let raw = Vec3::new(
                                edge(screen[1], screen[2], at),
                                edge(screen[2], screen[0], at),
                                edge(screen[0], screen[1], at),
                            );
                            if (raw * doubled.signum()).min_element() < 0.0 {
                                continue;
                            }
                            let mut bary = raw / doubled;
                            bary /= bary.element_sum();
                            let depth =
                                clip[0].z * bary.x + clip[1].z * bary.y + clip[2].z * bary.z;
                            let pixel = y * self.width + x;
                            if depth < self.depth[pixel] {
                                self.depth[pixel] = depth;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Where a world point lands, in pixels and depth.
    pub fn project(&self, camera: &Mat4, point: Vec3) -> (Vec2, f32) {
        let clip = camera.transform_point3(point);
        (
            Vec2::new(
                (clip.x * 0.5 + 0.5) * self.width as f32,
                (0.5 - clip.y * 0.5) * self.height as f32,
            ),
            clip.z,
        )
    }
}

/// Depth of the scene as the key light sees it.
///
/// A cast shadow is the strongest single cue that a figure is standing on
/// something rather than floating in front of it, and no amount of ambient
/// occlusion substitutes: occlusion darkens a crease, a shadow places a body in
/// a room.
pub struct ShadowMap {
    side: usize,
    depth: Vec<f32>,
    view: Mat4,
    /// World size of one shadow texel, for normal-offset sampling.
    texel: f32,
}

impl ShadowMap {
    /// Renders the scene from the light's direction.
    ///
    /// `toward` points from the scene toward the light.
    pub fn cast(items: &[Item], toward: Vec3, centre: Vec3, span: f32, side: usize) -> Self {
        // An orthographic camera looking along the light, framed generously:
        // the caster can sit well outside the part of the scene being viewed and
        // still throw a shadow across it.
        let reach = span * 1.6;
        let up = if toward.y.abs() > 0.95 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let forward = -toward.normalize_or(Vec3::NEG_Y);
        let right = forward.cross(up).normalize_or(Vec3::X);
        let true_up = right.cross(forward);
        let rotate = Mat4::from_cols(
            right.extend(0.0),
            true_up.extend(0.0),
            (-forward).extend(0.0),
            glam::Vec4::W,
        )
        .transpose();
        let view = Mat4::from_scale(Vec3::new(2.0 / reach, 2.0 / reach, -1.0 / reach))
            * rotate
            * Mat4::from_translation(-centre);

        let mut map = Self {
            side,
            depth: vec![f32::MAX; side * side],
            view,
            texel: reach / side as f32,
        };
        let mut buffer = GBuffer::new(side, side);
        let frame = Frame {
            turn: 0.0,
            pitch: 0.0,
            centre: Vec3::ZERO,
            span: 1.0,
        };
        let _ = frame;
        buffer.draw_depth(items, &view);
        map.depth = buffer.depth;
        map
    }

    /// How much light reaches a point, in `0..=1`.
    ///
    /// Filtered over a small neighbourhood: a single comparison gives a hard
    /// stair-stepped edge that reads worse than no shadow at all.
    ///
    /// `normal` is the surface normal at the point, for **normal-offset
    /// sampling**: the depth comparison happens one shadow texel off the
    /// surface, so a texel whose stored depth straddles its own surface cannot
    /// shadow it. A depth bias alone cannot do this — the bias needed where
    /// the surface runs steeply across the map grows without bound, and the
    /// slope-scaled 0.006 shipped here still striped every grazing surface
    /// with diagonal acne. The stripes drew as a woven, dirty crumple over the
    /// jaw flank, the neck and the temple in every lit render, and were
    /// chased as skin, texture and geometry before `--pass shadow` put them in
    /// this map (#158) — the twenty-first instrument caught reporting its own
    /// artifact as the body's.
    pub fn lit(&self, point: Vec3, normal: Vec3, slope: f32) -> f32 {
        let clip = self
            .view
            .transform_point3(point + normal * (self.texel * 1.5));
        let x = (clip.x * 0.5 + 0.5) * self.side as f32;
        let y = (0.5 - clip.y * 0.5) * self.side as f32;
        if !(0.0..self.side as f32).contains(&x) || !(0.0..self.side as f32).contains(&y) {
            return 1.0;
        }
        // Bias grows where the surface is edge-on to the light, which is where
        // a fixed bias gives the self-shadowing stripes known as acne.
        let bias = 0.0015 + 0.006 * slope;
        let mut open = 0.0;
        let mut taken = 0.0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (sx, sy) = (x as isize + dx, y as isize + dy);
                if sx < 0 || sy < 0 || sx >= self.side as isize || sy >= self.side as isize {
                    continue;
                }
                let stored = self.depth[sy as usize * self.side + sx as usize];
                open += f32::from(stored >= clip.z - bias);
                taken += 1.0;
            }
        }
        if taken == 0.0 { 1.0 } else { open / taken }
    }
}

/// Area-weighted vertex normals.
///
/// Taken over the mesh's own topology. Computing them over an unwrapped copy
/// instead gives every UV seam two different normals, one per chart, each blind
/// to the faces opposite — and the seam shows as a hard faceted band. Charts
/// split by zone, so those bands land at the hips and shoulders and read exactly
/// like a skinning defect. That cost a day.
pub fn smooth_normals(positions: &[Vec3], faces: &[Vec<u32>]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for face in faces {
        if face.len() < 3 {
            continue;
        }
        for corner in 1..face.len() - 1 {
            let (a, b, c) = (
                positions[face[0] as usize],
                positions[face[corner] as usize],
                positions[face[corner + 1] as usize],
            );
            let weighted = (b - a).cross(c - a);
            for &index in [face[0], face[corner], face[corner + 1]].iter() {
                normals[index as usize] += weighted;
            }
        }
    }
    for normal in &mut normals {
        *normal = normal.normalize_or(Vec3::Y);
    }
    normals
}

/// The tangent frame of one triangle, from its positions and UVs.
///
/// This is the frame a tangent-space normal map's X and Y mean: the map was
/// baked against the atlas's own axes, so measuring both derivatives — rather
/// than measuring one and crossing for the other — carries its convention
/// whole, handedness included. `None` where the UVs span no area, which is a
/// triangle the atlas cannot say anything about.
///
/// **Directions measured, magnitudes discarded.** `∂P/∂u` carries the chart's
/// texel density — metres per unit of UV — and the map's slopes were baked
/// against UNIT axes. Passed through un-normalised, every chart's relief is
/// amplified by its own density: measured on the first render, the pore field
/// drew as tree bark.
fn tangent_frame(points: [Vec3; 3], uvs: [Vec2; 3]) -> Option<(Vec3, Vec3)> {
    let edge1 = points[1] - points[0];
    let edge2 = points[2] - points[0];
    let delta1 = uvs[1] - uvs[0];
    let delta2 = uvs[2] - uvs[0];
    let det = delta1.x * delta2.y - delta1.y * delta2.x;
    if det.abs() < 1e-12 {
        return None;
    }
    let tangent = (edge1 * delta2.y - edge2 * delta1.y) / det;
    let bitangent = (edge2 * delta1.x - edge1 * delta2.x) / det;
    match (tangent.try_normalize(), bitangent.try_normalize()) {
        (Some(tangent), Some(bitangent)) => Some((tangent, bitangent)),
        _ => None,
    }
}

/// Reads one texel.
fn sample(uv: Vec2, pixels: &[u8], side: u32) -> Vec3 {
    let x = ((uv.x * side as f32) as u32).min(side.saturating_sub(1));
    let y = ((uv.y * side as f32) as u32).min(side.saturating_sub(1));
    let at = ((y * side + x) * 4) as usize;
    if at + 2 >= pixels.len() {
        return Vec3::splat(0.5);
    }
    Vec3::new(
        f32::from(pixels[at]),
        f32::from(pixels[at + 1]),
        f32::from(pixels[at + 2]),
    ) / 255.0
}

/// sRGB to linear, so that light can be added up correctly.
///
/// Everything authored by eye — a dye, a hair colour, a baked atlas — is in sRGB.
/// Adding two lights together in that space gives an answer that is too bright
/// in the midtones and washes every form flat, which is exactly how these sheets
/// looked.
pub fn to_linear(colour: Vec3) -> Vec3 {
    Vec3::new(
        channel_to_linear(colour.x),
        channel_to_linear(colour.y),
        channel_to_linear(colour.z),
    )
}

fn channel_to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Twice the signed area of a triangle.
fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
