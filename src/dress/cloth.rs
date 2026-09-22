//! The cloth atlas: an outfit's textures, baked into one image.
//!
//! A textured garment could be drawn with a material of its own, sampling its
//! tile with the hardware's own wrap — and every textured garment would then
//! be one more draw on a body whose budget is four (owner decision, #6). So
//! the textures are baked instead, the way the skin is (symbios-avatar #357,
//! owner call): every garment is charted where the body under it is charted
//! ([`Garment::charted`]), and this paints each garment's texels from its tile
//! into one atlas laid out like the skin's. The cloth stays one mesh with one
//! material, whatever the outfit wears.
//!
//! ## Panels
//!
//! A tile needs coordinates that tile — metres along the cloth, not atlas
//! space, whose charts are packed at different scales and turned any way the
//! packer liked. They come from wrapping each garment in panels, the way a
//! garment is cut from cloth:
//!
//! - **a top** is a trunk panel round the spine and a sleeve round each arm,
//!   joined at the armhole;
//! - **trousers** are a leg each, running up over the hip and the seat, and
//!   joined down the centre front and back — which is how trousers are cut.
//!
//! A panel is a cylinder about its axis. Its vertical coordinate is distance
//! up the axis, and its horizontal one is distance round it, measured with
//! the ellipse the panel's own cloth measures out rather than a circle: an
//! angle round a torso half again as wide as it is deep stretches a pattern
//! by that much at the sides. The angle used is the mean of the polar and
//! the ellipse's parametric angle, which leans one way as much as the other
//! leans the other, and stays within a couple of percent of true arc length
//! at a torso's proportions.
//!
//! Each panel wraps once and so has one seam: up the back of a trunk, down
//! the inside of an arm or a leg — where real garments put theirs.
//!
//! ## What is painted
//!
//! Albedo, relief and finish, as the skin atlas carries them. The albedo is
//! the tile multiplied by the garment's colour in linear light, which is what
//! a material's base colour does to its texture. The relief is the tile's
//! normal turned from the tile's frame on the cloth into the atlas's frame on
//! the same cloth, so it lights the way the tile would have lit drawn on its
//! own. A garment with no texture is painted plain — its colour, the plain
//! cloth finish, no relief — because the atlas covers the whole outfit or
//! none of it.

use glam::Vec3;
use symbios_texture::generator::TextureMap;

use super::Outfit;
use super::garment::{Chain, Garment};
use super::surface::GarmentTexture;
use crate::plan::Zone;
use crate::rig::{Rig, landmark};
use crate::uv::UvUnwrap;

/// Texels a side of one generated tile.
///
/// A tile at the default four tiles to the metre puts a texel every
/// millimetre of cloth, and the cloth atlas carries one every five or so
/// (see [`side`]), so the bake mostly reads the tile's second or third mip
/// level and this could be smaller. It is not, so that a finer tile setting
/// or a larger atlas still has detail to sample.
pub const TILE: u32 = 256;

/// How the cloth atlas's side relates to the skin atlas's.
///
/// Half. The atlas is laid out like the skin's so that the garments need no
/// unwrap of their own, and three quarters of that layout is charts no
/// garment covers — the face, the hands, the feet — so a cloth atlas at the
/// skin's full side is four times the memory for the same cloth.
///
/// What half buys, **measured** over the default body and seeds 1 and 9 in
/// long sleeves and full trousers at the default atlas: a median of 4.6 to
/// 6.0 mm of cloth per texel, a tenth of the cloth under 3.3 to 4.0 and a
/// tenth over 7.6 to 11.3. The 12 m chase camera in `symbios-overlands`
/// spends a pixel on about 9 mm, so the cloth holds up there; a close-up at
/// two metres sees it soften, as it would the skin at a quarter of its
/// density.
#[must_use]
pub fn side(atlas: u32) -> u32 {
    (atlas / 2).max(16)
}

/// The finish of plain cloth: the roughness a garment with no texture is
/// painted with.
///
/// The constant `bevy_symbios_avatar` drew all cloth with before the atlas
/// existed, moved here so a texture's own finish and a plain garment's are
/// in one place.
pub const PLAIN_ROUGHNESS: f32 = 0.92;

/// How far, in texels, painted cloth grows into the empty atlas around it.
///
/// The skin atlas's figure and for its reason: a bilinear tap or a mip level
/// at a chart's edge pulls in whatever is next to it.
const DILATION: u32 = 8;

/// How far outside a triangle, in texels, a texel centre may sit and still
/// be painted by it — the skin bake's cover margin, for the same slivers.
///
/// **Only a texel no triangle contains.** The skin bake lets the margin
/// overwrite its neighbours' texels, and one material over both sides of an
/// edge makes that harmless. Two garments meet at the waist on one chart,
/// though, and the top, baked second, painted its colour up to 0.71 texels
/// past its own edge all the way round: a band of it, stair-stepped, along
/// the trousers' waistband. A texel therefore belongs to the triangle that
/// contains its centre, and the margin only settles the ones none does,
/// nearest first.
///
/// What is left is the atlas's resolution, and no rule can take it away: a
/// boundary between two colours on one chart can only fall on a texel's
/// edge, so a renderer sampling the nearest texel moves the waist seam up
/// to half a texel either way (see [`side`] for a texel's size), and one
/// sampling bilinearly blends it across one.
const COVER: f32 = 0.71;

/// Paints the cloth atlas for `outfit`, `side` texels square, its mip chain
/// appended.
///
/// `unwrap` is the body's, and `face_of` the index from each body face into
/// it that [`super::charted_faces`] builds. `None` when no garment wears a
/// texture this build can draw: an outfit in plain colours is drawn from its
/// vertex colours and needs no atlas at all.
#[must_use]
pub fn paint(
    outfit: &Outfit,
    rig: &Rig,
    unwrap: &UvUnwrap,
    face_of: &[Option<u32>],
    side: u32,
) -> Option<TextureMap> {
    if side == 0 || !outfit.is_textured() {
        return None;
    }
    let dressings: Vec<Dressing> = outfit
        .garments
        .iter()
        .map(|garment| Dressing::of(garment, rig))
        .collect();
    let mut canvas = Canvas::new(side);
    for (index, garment) in outfit.garments.iter().enumerate() {
        canvas.rasterise(index, garment, unwrap, face_of);
    }
    // With its mip chain, unlike the skin's atlas: a woven or printed tile is
    // detail at the pitch of a texel, and a single level drawn from twelve
    // metres off shimmers where a skin's slow gradients do not. The chain is
    // symbios-texture's own, filtered per map — sRGB colour, renormalised
    // relief, linear finish — and appended after the base level, which is
    // all a renderer that ignores it reads.
    Some(canvas.paint(&dressings).with_mips())
}

/// One garment, ready to paint: its colour, its tile if it has one, and the
/// panels its cloth is wrapped in.
struct Dressing {
    /// The garment's colour in linear light.
    colour: Vec3,
    /// Its colour as the atlas stores it.
    srgb: [u8; 3],
    /// Its texture, generated, if it has one this build can draw.
    tile: Option<Tile>,
    /// The panels its texels are laid out on.
    panels: Vec<Panel>,
}

impl Dressing {
    fn of(garment: &Garment, rig: &Rig) -> Self {
        let colour = Vec3::from_array(garment.colour.map(to_linear));
        let tile = garment.texture.as_ref().and_then(Tile::of);
        let panels = if tile.is_some() {
            Panel::wrap(garment, rig)
        } else {
            Vec::new()
        };
        Self {
            colour,
            srgb: garment.colour.map(byte),
            tile,
            panels,
        }
    }

    /// Which panel a point of this garment lies in, and where on it.
    fn place(&self, point: Vec3) -> Option<Place> {
        self.panels
            .iter()
            .map(|panel| panel.place(point))
            .min_by(|a, b| a.reach.total_cmp(&b.reach))
    }
}

/// A texture, generated and ready to sample.
struct Tile {
    /// The generated map, its mip chain appended.
    map: TextureMap,
    /// Where each mip level starts in the map's buffers, in bytes.
    levels: Vec<usize>,
    /// Tile repeats per metre of cloth.
    scale: f32,
    /// The pattern's turn, as its cosine and sine.
    turn: (f32, f32),
}

impl Tile {
    /// Generates a garment's tile, or `None` if its surface cannot be drawn.
    fn of(texture: &GarmentTexture) -> Option<Self> {
        let map = texture.surface.generate(TILE)?.ok()?.with_mips();
        let mut levels = Vec::with_capacity(map.mip_level_count as usize);
        let mut offset = 0;
        for level in 0..map.mip_level_count {
            levels.push(offset);
            let size = (map.width >> level).max(1) as usize;
            offset += size * size * 4;
        }
        let radians = texture.rotation.to_radians();
        Some(Self {
            map,
            levels,
            scale: texture.scale,
            turn: (radians.cos(), radians.sin()),
        })
    }

    /// Bilinear, wrapping sample of one of the map's buffers at a tile
    /// position in level-0 texels, from mip level `level`.
    fn sample(&self, buffer: &[u8], col: f32, row: f32, level: usize) -> [f32; 4] {
        let size = (self.map.width >> level).max(1) as i64;
        let scale = 1.0 / (1u32 << level) as f32;
        let (x, y) = (col * scale - 0.5, row * scale - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let base = self.levels[level];
        let at = |dx: i64, dy: i64| {
            let px = (x0 as i64 + dx).rem_euclid(size);
            let py = (y0 as i64 + dy).rem_euclid(size);
            let index = base + ((py * size + px) * 4) as usize;
            let texel = &buffer[index..index + 4];
            [texel[0], texel[1], texel[2], texel[3]].map(f32::from)
        };
        let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
        std::array::from_fn(|channel| {
            let top = a[channel] + (b[channel] - a[channel]) * fx;
            let bottom = c[channel] + (d[channel] - c[channel]) * fx;
            top + (bottom - top) * fy
        })
    }
}

/// A cylinder a garment's cloth is wrapped in.
struct Panel {
    axis: Axis,
    /// The half-widths of the cloth round the axis, along the angle's zero
    /// and a quarter-turn from it.
    semi: (f32, f32),
}

/// What a panel is wrapped about.
#[derive(Clone, Copy)]
enum Axis {
    /// A straight line: the spine. Its angle's zero is the body's front.
    Line { origin: Vec3, up: Vec3, front: Vec3 },
    /// A limb, its two segments end to end. Its angle's zero on each is the
    /// limb's outside, carried from the upper segment onto the lower.
    Limb {
        chain: Chain,
        outside: (Vec3, Vec3),
        length: (f32, f32),
    },
}

/// Where on a panel a point lies.
struct Place {
    /// Distance round the panel from its front or its outside, in metres.
    round: f32,
    /// Distance up the panel, in metres.
    up: f32,
    /// The direction of increasing `round` at the point, and of `up`.
    frame: (Vec3, Vec3),
    /// How far the point is from the panel, in the panel's own radii: the
    /// nearest panel is the one a point is wrapped in.
    reach: f32,
    /// How much faster than the cloth itself the pattern runs round the
    /// panel here, at least one: near a panel's axis a step across the cloth
    /// turns the angle a long way, and the tile has to be read coarser there
    /// or it aliases.
    stretch: f32,
}

impl Panel {
    /// The panels `garment` is wrapped in: a leg each for trousers, the
    /// trunk and a sleeve per arm for anything else.
    fn wrap(garment: &Garment, rig: &Rig) -> Vec<Panel> {
        let legs = !garment.limbs.is_empty() && garment.limbs.iter().all(|limb| !limb.is_fore());
        let spine = trunk(rig);
        let mut axes: Vec<(Axis, f32)> = Vec::new();
        if !legs && let Some(trunk) = spine {
            axes.push(trunk);
        }
        let spine = spine.map(|(axis, _)| axis);
        for &limb in &garment.limbs {
            let Some(chain) = Chain::of(rig, limb) else {
                continue;
            };
            let radius = rig
                .in_zone(Zone::UpperLimb(limb))
                .into_iter()
                .chain(rig.in_zone(Zone::LowerLimb(limb)))
                .map(|joint| rig.joints[joint].radius)
                .fold(0.0f32, f32::max)
                .max(0.02);
            axes.push((limb_axis(chain, spine.as_ref()), radius));
        }
        // Measure each panel's cloth, as wrapped with the rig's own radii.
        // Only the outer shell: the inner one is the same cloth a bite inside
        // the skin.
        let outer = &garment.mesh.positions[..garment.mesh.vertex_count() / 2];
        let provisional: Vec<Panel> = axes
            .into_iter()
            .map(|(axis, radius)| Panel {
                axis,
                semi: (radius, radius),
            })
            .collect();
        let mut spans: Vec<Vec<(f32, f32)>> = vec![Vec::new(); provisional.len()];
        for &point in outer {
            let nearest = provisional
                .iter()
                .enumerate()
                .map(|(index, panel)| (index, panel.offset(point)))
                .min_by(|(_, a), (_, b)| a.2.total_cmp(&b.2));
            // Measured only alongside the axis: a trouser leg's cloth runs up
            // over the hip and the seat, far out from the thigh's line, and
            // counted, it inflated the leg's half-widths until the whole thigh
            // wore its pattern squeezed round it by half again.
            if let Some((index, (across, deep, _, true))) = nearest {
                spans[index].push((across.abs(), deep.abs()));
            }
        }
        provisional
            .into_iter()
            .zip(spans)
            .map(|(panel, mut span)| {
                let semi = if span.len() < 8 {
                    panel.semi
                } else {
                    (
                        percentile(&mut span, |s| s.0),
                        percentile(&mut span, |s| s.1),
                    )
                };
                Panel { semi, ..panel }
            })
            .collect()
    }

    /// Where `point` sits against this panel's axis: its offset along the
    /// angle's zero and a quarter-turn from it, and its distance in the
    /// panel's radii.
    fn offset(&self, point: Vec3) -> (f32, f32, f32, bool) {
        let local = self.axis.at(point);
        let side = local.up.cross(local.zero);
        let radius = 0.5 * (self.semi.0 + self.semi.1);
        (
            local.radial.dot(local.zero),
            local.radial.dot(side),
            local.distance / radius.max(1e-4),
            local.within,
        )
    }

    /// Where on this panel `point` lies.
    fn place(&self, point: Vec3) -> Place {
        let local = self.axis.at(point);
        let side = local.up.cross(local.zero);
        let (x, y) = (local.radial.dot(local.zero), local.radial.dot(side));
        let (a, b) = (self.semi.0.max(1e-4), self.semi.1.max(1e-4));
        // The mean of the polar and the parametric angle: see the module
        // documentation. The two share a quadrant, so the mean never wraps.
        let angle = 0.5 * (y.atan2(x) + (y / b).atan2(x / a));
        let radius = 0.5 * (a + b);
        let outward = local.radial.try_normalize().unwrap_or(local.zero);
        Place {
            round: angle * radius,
            up: local.height,
            frame: (local.up.cross(outward), local.up),
            reach: local.distance / radius,
            stretch: (radius / local.radial.length().max(1e-4)).max(1.0),
        }
    }
}

impl Axis {
    /// A point's offset from the axis, square to it; the axis's up and the
    /// angle's zero there; and the point's height up the axis.
    fn at(&self, point: Vec3) -> Local {
        match self {
            Axis::Line { origin, up, front } => {
                let height = (point - *origin).dot(*up);
                let radial = point - (*origin + *up * height);
                Local {
                    radial,
                    up: *up,
                    zero: *front,
                    height,
                    distance: radial.length(),
                    within: true,
                }
            }
            Axis::Limb {
                chain,
                outside,
                length,
            } => {
                // The nearer segment's, as the hem's chain position is — so
                // a sleeve's pattern turns the elbow where its hem would.
                let (upper, lower) = (chain.upper, chain.lower);
                let near = |(start, end): (Vec3, Vec3)| {
                    let axis = end - start;
                    let share = if axis.length_squared() <= f32::EPSILON {
                        0.0
                    } else {
                        (point - start).dot(axis) / axis.length_squared()
                    };
                    let closest = start + axis * share.clamp(0.0, 1.0);
                    (share, point.distance(closest))
                };
                let (upper_share, upper_distance) = near(upper);
                let (lower_share, lower_distance) = near(lower);
                let total = length.0 + length.1;
                let (start, end, share, run, zero) = if upper_distance <= lower_distance {
                    (
                        upper.0,
                        upper.1,
                        upper_share,
                        upper_share * length.0,
                        outside.0,
                    )
                } else {
                    (
                        lower.0,
                        lower.1,
                        lower_share,
                        length.0 + lower_share * length.1,
                        outside.1,
                    )
                };
                // Up the limb is toward its root, so a pattern reads the same
                // way up on a sleeve as on the trunk it is sewn to.
                let up = (start - end).try_normalize().unwrap_or(landmark::UP);
                Local {
                    radial: point - (start + (end - start) * share),
                    up,
                    zero,
                    height: total - run,
                    // To the SEGMENT, not the line it lies on: the angle
                    // wants the line, so skin past a shoulder still wraps,
                    // but a limb's line runs on past its root, and in an
                    // A-pose an arm's runs straight through the upper chest.
                    // Chosen by the line, the chest was wrapped in the arm's
                    // cylinder and a brick course ran diagonally across it.
                    distance: upper_distance.min(lower_distance),
                    within: if upper_distance <= lower_distance {
                        upper_share >= 0.0
                    } else {
                        lower_share <= 1.0
                    },
                }
            }
        }
    }
}

/// A point against a panel's axis.
struct Local {
    /// Its offset from the axis, square to it — from the axis's LINE, so the
    /// angle round it is defined past the ends of a limb's segments too.
    radial: Vec3,
    /// The axis's up there, toward a limb's root or the top of the trunk.
    up: Vec3,
    /// The angle's zero there.
    zero: Vec3,
    /// How far up the axis the point is.
    height: f32,
    /// How far the point is from the axis itself — its segments, for a limb —
    /// which is what decides which panel wraps it.
    distance: f32,
    /// Whether it lies alongside the axis rather than past an end of it: a
    /// hip or a seat wrapped by a trouser leg lies above the thigh's root.
    within: bool,
}

/// How far off the body's midline a trunk joint may stand and still be the
/// spine, in metres.
///
/// The spine's joints stand on it exactly on every body a plan builds; the
/// margin is for a body a future plan builds a little off it.
const MIDLINE: f32 = 0.01;

/// The trunk's panel axis and a radius for it: the spine, from its lowest
/// joint to its highest, its angle's zero the body's front.
///
/// **The spine, not every trunk joint.** The chest's zone holds the shoulders
/// too, and the highest joint in it stands at one of them: an axis drawn to
/// it leaned twenty degrees to that side and sheared every course of a
/// pattern across the chest into a diagonal (measured, #357).
fn trunk(rig: &Rig) -> Option<(Axis, f32)> {
    let joints: Vec<usize> = rig
        .surfaced()
        .filter(|&joint| {
            matches!(
                rig.joints[joint].zone,
                Zone::Pelvis | Zone::Abdomen | Zone::Chest
            ) && rig.joints[joint].position.x.abs() <= MIDLINE
        })
        .collect();
    let height = |joint: &usize| rig.joints[*joint].position.y;
    let low = *joints
        .iter()
        .min_by(|a, b| height(a).total_cmp(&height(b)))?;
    let high = *joints
        .iter()
        .max_by(|a, b| height(a).total_cmp(&height(b)))?;
    let origin = rig.joints[low].position;
    let up = (rig.joints[high].position - origin)
        .try_normalize()
        .unwrap_or(landmark::UP);
    let front = (landmark::FORWARD - up * landmark::FORWARD.dot(up))
        .try_normalize()
        .unwrap_or(landmark::FORWARD);
    let radius = joints
        .iter()
        .map(|&joint| {
            let node = rig.joints[joint];
            node.radius * 0.5 * (node.scale.x + node.scale.y)
        })
        .fold(0.0f32, f32::max)
        .max(0.05);
    Some((Axis::Line { origin, up, front }, radius))
}

/// A limb's panel axis: its chain, with the angle's zero on its outside.
///
/// The outside is away from the spine, square to the upper segment; the
/// lower segment takes the same direction squared to itself, so the angle
/// turns the elbow or the knee without a jump.
fn limb_axis(chain: Chain, spine: Option<&Axis>) -> Axis {
    let square = |direction: Vec3, to: (Vec3, Vec3)| {
        let axis = (to.1 - to.0).try_normalize().unwrap_or(landmark::UP);
        (direction - axis * direction.dot(axis)).try_normalize()
    };
    let middle = 0.5 * (chain.upper.0 + chain.upper.1);
    let away = match spine {
        Some(Axis::Line { origin, up, .. }) => {
            let height = (middle - *origin).dot(*up);
            middle - (*origin + *up * height)
        }
        _ => Vec3::X * middle.x.signum(),
    };
    let upper = square(away, chain.upper)
        .or_else(|| square(landmark::FORWARD, chain.upper))
        .unwrap_or(landmark::FORWARD);
    let lower = square(upper, chain.lower).unwrap_or(upper);
    Axis::Limb {
        chain,
        outside: (upper, lower),
        length: (
            chain.upper.0.distance(chain.upper.1),
            chain.lower.0.distance(chain.lower.1),
        ),
    }
}

/// The ninetieth percentile of one component of `spans` — the cloth's
/// half-width, robust to the few points a neighbouring panel reaches into.
fn percentile(spans: &mut [(f32, f32)], component: impl Fn(&(f32, f32)) -> f32) -> f32 {
    spans.sort_unstable_by(|a, b| component(a).total_cmp(&component(b)));
    let at = ((spans.len() - 1) as f32 * 0.9).round() as usize;
    component(&spans[at]).max(1e-3)
}

/// One texel of garment the rasteriser found.
#[derive(Clone, Copy)]
struct Covered {
    /// Which garment of the outfit.
    garment: u16,
    /// Where on it, in body space.
    position: Vec3,
    /// Its smooth normal there.
    normal: Vec3,
    /// How far the cloth moves per texel across the atlas and down it.
    step: (Vec3, Vec3),
    /// How far outside the triangle that painted it the texel's centre
    /// lies, in texels; nothing when inside. What decides a texel two
    /// triangles reach — see [`COVER`].
    outside: f32,
}

/// The atlas being painted: which garment covers each texel, then the maps.
struct Canvas {
    side: u32,
    covered: Vec<Option<Covered>>,
}

impl Canvas {
    fn new(side: u32) -> Self {
        Self {
            side,
            covered: vec![None; (side * side) as usize],
        }
    }

    /// Finds every texel `garment`'s outer shell covers.
    ///
    /// The outer shell only. The inner shell is the same cloth a bite inside
    /// the skin, charted where the outer is, and a rim is a strip of zero
    /// area in the atlas.
    fn rasterise(
        &mut self,
        index: usize,
        garment: &Garment,
        unwrap: &UvUnwrap,
        face_of: &[Option<u32>],
    ) {
        let mesh = &garment.mesh;
        let normals = mesh.vertex_normals();
        let scale = self.side as f32;
        // Outer shell faces are the even ones before the rims: each covered
        // body face pushed its outer face and then its inner one.
        for face in (0..garment.claim.len()).map(|covered| covered * 2) {
            let (Some(corners), Some(sources)) =
                (mesh.faces.get(face), garment.corner_source.get(face))
            else {
                continue;
            };
            let uv = |corner: usize| {
                let (body_face, at) = sources[corner];
                let unwrapped = face_of.get(body_face as usize).copied().flatten()?;
                let vertex = *unwrap.faces.get(unwrapped as usize)?.get(usize::from(at))?;
                unwrap.uvs.get(vertex as usize).map(|uv| *uv * scale)
            };
            for fan in 1..corners.len().saturating_sub(1) {
                let slots = [0, fan, fan + 1];
                let (Some(a), Some(b), Some(c)) = (uv(slots[0]), uv(slots[1]), uv(slots[2])) else {
                    continue;
                };
                let vertices = slots.map(|slot| corners[slot] as usize);
                self.triangle(
                    index as u16,
                    [a, b, c],
                    vertices.map(|v| mesh.positions[v]),
                    vertices.map(|v| normals[v]),
                );
            }
        }
    }

    /// Covers the texels of one triangle.
    fn triangle(
        &mut self,
        garment: u16,
        pixels: [glam::Vec2; 3],
        positions: [Vec3; 3],
        normals: [Vec3; 3],
    ) {
        let edge = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2| {
            (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
        };
        let area = edge(pixels[0], pixels[1], pixels[2]);
        if area.abs() < 1e-9 {
            return;
        }
        // How the cloth moves per texel: the affine map from atlas pixels to
        // body space this triangle defines.
        let (e1, e2) = (pixels[1] - pixels[0], pixels[2] - pixels[0]);
        let (p1, p2) = (positions[1] - positions[0], positions[2] - positions[0]);
        let det = e1.x * e2.y - e1.y * e2.x;
        let step = ((p1 * e2.y - p2 * e1.y) / det, (p2 * e1.x - p1 * e2.x) / det);

        let side = self.side as i64;
        let lo = pixels[0].min(pixels[1]).min(pixels[2]) - glam::Vec2::splat(COVER);
        let hi = pixels[0].max(pixels[1]).max(pixels[2]) + glam::Vec2::splat(COVER);
        let x0 = (lo.x.floor() as i64).clamp(0, side);
        let y0 = (lo.y.floor() as i64).clamp(0, side);
        let x1 = (hi.x.ceil() as i64).clamp(0, side);
        let y1 = (hi.y.ceil() as i64).clamp(0, side);
        let lengths = [
            pixels[1].distance(pixels[2]).max(f32::EPSILON),
            pixels[2].distance(pixels[0]).max(f32::EPSILON),
            pixels[0].distance(pixels[1]).max(f32::EPSILON),
        ];
        let inward = area.signum();
        for y in y0..y1 {
            for x in x0..x1 {
                let point = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let raw = Vec3::new(
                    edge(pixels[1], pixels[2], point),
                    edge(pixels[2], pixels[0], point),
                    edge(pixels[0], pixels[1], point),
                );
                let inside = Vec3::new(
                    raw.x * inward / lengths[0],
                    raw.y * inward / lengths[1],
                    raw.z * inward / lengths[2],
                );
                if inside.min_element() < -COVER {
                    continue;
                }
                let outside = (-inside.min_element()).max(0.0);
                let slot = (y * side + x) as usize;
                if self.covered[slot].is_some_and(|held| held.outside <= outside) {
                    continue;
                }
                let mut weights = (raw / area).max(Vec3::ZERO);
                let total = weights.element_sum();
                if total <= 0.0 {
                    continue;
                }
                weights /= total;
                let position =
                    positions[0] * weights.x + positions[1] * weights.y + positions[2] * weights.z;
                let normal =
                    (normals[0] * weights.x + normals[1] * weights.y + normals[2] * weights.z)
                        .normalize_or(landmark::UP);
                self.covered[slot] = Some(Covered {
                    garment,
                    position,
                    normal,
                    step,
                    outside,
                });
            }
        }
    }

    /// Paints every covered texel and grows the result into the gutters.
    fn paint(self, dressings: &[Dressing]) -> TextureMap {
        let texels = self.covered.len();
        let mut albedo = vec![0u8; texels * 4];
        let mut normal = vec![0u8; texels * 4];
        let mut orm = vec![0u8; texels * 4];
        let mut painted = vec![false; texels];
        for (index, covered) in self.covered.iter().enumerate() {
            let Some(covered) = covered else {
                continue;
            };
            let Some(dressing) = dressings.get(usize::from(covered.garment)) else {
                continue;
            };
            let (colour, relief, finish) = shade(dressing, covered);
            let at = index * 4;
            albedo[at..at + 4].copy_from_slice(&colour);
            normal[at..at + 4].copy_from_slice(&relief);
            orm[at..at + 4].copy_from_slice(&finish);
            painted[index] = true;
        }
        dilate(
            self.side,
            &mut painted,
            [&mut albedo, &mut normal, &mut orm],
        );
        // Texels no garment reaches, even grown: plain white, flat, the cloth
        // finish, so a stray tap reads as nothing rather than as black.
        for (index, painted) in painted.iter().enumerate() {
            if *painted {
                continue;
            }
            let at = index * 4;
            albedo[at..at + 4].copy_from_slice(&[255; 4]);
            normal[at..at + 4].copy_from_slice(&FLAT);
            orm[at..at + 4].copy_from_slice(&plain_finish());
        }
        TextureMap {
            albedo,
            normal,
            roughness: orm,
            emissive: None,
            width: self.side,
            height: self.side,
            mip_level_count: 1,
        }
    }
}

/// A flat tangent-space normal, encoded.
const FLAT: [u8; 4] = [128, 128, 255, 255];

/// The finish plain cloth is painted with: unoccluded, [`PLAIN_ROUGHNESS`],
/// not metal.
fn plain_finish() -> [u8; 4] {
    [255, byte(PLAIN_ROUGHNESS), 0, 255]
}

/// Where within an atlas texel the bake samples the tile, in texels from
/// its centre.
///
/// Four, on a rotated grid: a texel covers a patch of cloth, not a point of
/// it, and one sample per texel of a tile with sharp edges — a brick's
/// mortar, a Truchet's arcs — aliased into streaks wherever the mapping
/// stretched the pattern. Averaged, a seam between two panels or the wrap of
/// one comes out as a blend a texel wide rather than a stair.
const SUBSAMPLES: [(f32, f32); 4] = [
    (-0.125, -0.375),
    (0.375, -0.125),
    (0.125, 0.375),
    (-0.375, 0.125),
];

/// One texel's albedo, relief and finish.
fn shade(dressing: &Dressing, covered: &Covered) -> ([u8; 4], [u8; 4], [u8; 4]) {
    let plain = || {
        let [r, g, b] = dressing.srgb;
        ([r, g, b, 255], FLAT, plain_finish())
    };
    let Some(tile) = &dressing.tile else {
        return plain();
    };
    let (mut colour, mut finish, mut relief, mut taken) = (Vec3::ZERO, [0.0f32; 3], Vec3::ZERO, 0);
    for (dx, dy) in SUBSAMPLES {
        let point = covered.position + covered.step.0 * dx + covered.step.1 * dy;
        let Some(place) = dressing.place(point) else {
            continue;
        };
        let sample = sample_at(tile, &place, covered);
        colour += sample.colour;
        for (sum, channel) in finish.iter_mut().zip(sample.finish) {
            *sum += channel;
        }
        relief += sample.relief;
        taken += 1;
    }
    if taken == 0 {
        return plain();
    }
    let share = 1.0 / taken as f32;
    let albedo = (colour * share * dressing.colour)
        .to_array()
        .map(to_srgb)
        .map(byte);
    let finish = finish.map(|sum| (sum * share).round().clamp(0.0, 255.0) as u8);

    // Into the atlas's own frame at this texel: along its columns, along its
    // rows — the directions a renderer rebuilds from the atlas coordinates.
    let normal = covered.normal;
    let turned = relief.try_normalize().unwrap_or(normal);
    let x_axis = flatten(covered.step.0, normal);
    let handed = normal.cross(x_axis);
    let y_axis = if handed.dot(covered.step.1) < 0.0 {
        -handed
    } else {
        handed
    };
    let encoded = Vec3::new(turned.dot(x_axis), turned.dot(y_axis), turned.dot(normal))
        .try_normalize()
        .unwrap_or(Vec3::Z);
    let relief = (encoded * 0.5 + Vec3::splat(0.5)).to_array().map(byte);

    (
        [albedo[0], albedo[1], albedo[2], 255],
        [relief[0], relief[1], relief[2], 255],
        [finish[0], finish[1], finish[2], 255],
    )
}

/// What one sample of a tile says about the cloth at a place.
struct Sample {
    /// The tile's colour there, in linear light, before the garment's own.
    colour: Vec3,
    /// Its finish — occlusion, roughness, metal — as bytes.
    finish: [f32; 3],
    /// Its surface normal, in body space.
    relief: Vec3,
}

/// Samples `tile` where `place` puts a point of the cloth.
fn sample_at(tile: &Tile, place: &Place, covered: &Covered) -> Sample {
    let normal = covered.normal;
    // The tile's own frame on the cloth: its columns turned `rotation` from
    // round the panel, its rows running DOWN the cloth so that the tile reads
    // the way up it was painted.
    let (cos, sin) = tile.turn;
    let (round, up) = place.frame;
    let across = flatten(round * cos + up * sin, normal);
    let down = flatten(round * sin - up * cos, normal);
    let s = tile.scale * (place.round * cos + place.up * sin);
    let t = tile.scale * (-place.round * sin + place.up * cos);
    let size = tile.map.width as f32;
    let (col, row) = (s * size, -t * size);

    // The mip level whose texels are about the size of the patch of cloth one
    // sample stands for: half a texel of atlas, stretched where the pattern
    // runs fast round the panel.
    let patch = 0.5 * covered.step.0.length().max(covered.step.1.length());
    let footprint = patch * place.stretch * tile.scale * size;
    let deepest = tile.levels.len().saturating_sub(1);
    let level = if footprint > 1.0 {
        (footprint.log2().round() as usize).min(deepest)
    } else {
        0
    };

    let albedo = tile.sample(&tile.map.albedo, col, row, level);
    let finish = tile.sample(&tile.map.roughness, col, row, level);
    let relief = tile.sample(&tile.map.normal, col, row, level);
    let tangent = Vec3::new(relief[0], relief[1], relief[2]) / 255.0 * 2.0 - Vec3::ONE;
    Sample {
        colour: Vec3::new(albedo[0], albedo[1], albedo[2]) / 255.0,
        finish: [finish[0], finish[1], finish[2]],
        relief: (across * tangent.x + down * tangent.y + normal * tangent.z)
            .try_normalize()
            .unwrap_or(normal),
    }
    .linear()
}

impl Sample {
    /// The same sample with its colour decoded from sRGB.
    fn linear(self) -> Self {
        Self {
            colour: Vec3::from_array(self.colour.to_array().map(to_linear)),
            ..self
        }
    }
}

/// `direction` laid into the surface whose normal is `normal`, unit length.
fn flatten(direction: Vec3, normal: Vec3) -> Vec3 {
    (direction - normal * direction.dot(normal))
        .try_normalize()
        .unwrap_or(Vec3::ZERO)
}

/// Grows painted texels outward into the unpainted ones, a texel a pass.
///
/// The first painted neighbour wins rather than a blend, as in the skin bake:
/// this is padding for filtering, and a blend of two garments is neither.
fn dilate(side: u32, painted: &mut [bool], maps: [&mut Vec<u8>; 3]) {
    let side = side as i64;
    let [albedo, normal, orm] = maps;
    for _ in 0..DILATION {
        let was = painted.to_vec();
        let mut grew = false;
        for y in 0..side {
            for x in 0..side {
                let here = (y * side + x) as usize;
                if was[here] {
                    continue;
                }
                let found = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)]
                    .into_iter()
                    .find_map(|(dx, dy)| {
                        let (nx, ny) = (x + dx, y + dy);
                        let inside = (0..side).contains(&nx) && (0..side).contains(&ny);
                        let there = (ny * side + nx) as usize;
                        (inside && was[there]).then_some(there)
                    });
                let Some(there) = found else {
                    continue;
                };
                for map in [&mut *albedo, &mut *normal, &mut *orm] {
                    map.copy_within(there * 4..there * 4 + 4, here * 4);
                }
                painted[here] = true;
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
}

/// An sRGB channel in linear light.
fn to_linear(channel: f32) -> f32 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// A linear channel in sRGB.
fn to_srgb(channel: f32) -> f32 {
    let channel = channel.clamp(0.0, 1.0);
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

/// A unit channel as a byte.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a clamped unit float scaled to 0..=255 fits a byte by construction"
)]
fn byte(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dress::{GarmentTexture, OutfitParams, SurfaceConfig, charted_faces};
    use crate::rig::{SkinConfig, skin};
    use crate::uv::{UvConfig, unwrap};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    struct Dressed {
        rig: Rig,
        unwrap: UvUnwrap,
        face_of: Vec<Option<u32>>,
        outfit: Outfit,
    }

    fn dressed(params: &OutfitParams) -> Dressed {
        let record = AvatarRecord::new("Cloth", Archetype::default());
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        let unwrap = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let face_of = charted_faces(&unwrap, mesh.face_count());
        let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, params);
        Dressed {
            rig,
            unwrap,
            face_of,
            outfit,
        }
    }

    fn woven() -> Option<GarmentTexture> {
        Some(GarmentTexture::new(
            SurfaceConfig::named("Fabric").expect("a surface"),
        ))
    }

    #[test]
    fn an_outfit_in_plain_colours_paints_no_atlas() {
        let body = dressed(&OutfitParams::default());
        assert!(paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, 64).is_none());
    }

    #[test]
    fn a_textured_outfit_paints_an_atlas_covering_both_garments() {
        let mut params = OutfitParams::default();
        params.top.texture = woven();
        params.trousers.colour = [0.2, 0.3, 0.8];
        let body = dressed(&params);
        // At the default body's own cloth side: a smaller atlas resolves the
        // weave into one averaged colour, which is the mip choice working.
        let side = side(crate::AvatarConfig::default().atlas);
        let map = paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, side)
            .expect("a textured outfit has an atlas");
        assert_eq!((map.width, map.height), (side, side));
        // Every level down to one texel, each a quarter of the one above.
        assert_eq!(map.mip_level_count, side.ilog2() + 1);
        let chain: usize = (0..map.mip_level_count)
            .map(|level| ((side >> level).max(1) as usize).pow(2) * 4)
            .sum();
        for buffer in [&map.albedo, &map.normal, &map.roughness] {
            assert_eq!(buffer.len(), chain);
        }

        // The plain garment's texels carry its colour exactly, somewhere.
        let blue = [byte(0.2), byte(0.3), byte(0.8)];
        assert!(
            map.albedo.chunks_exact(4).any(|texel| texel[..3] == blue),
            "the plain trousers' colour is nowhere in the atlas"
        );
        // And the woven top varies: a weave is not one colour.
        let mut colours: Vec<[u8; 3]> = map
            .albedo
            .chunks_exact(4)
            .map(|texel| [texel[0], texel[1], texel[2]])
            .filter(|texel| *texel != blue && *texel != [255; 3])
            .collect();
        colours.sort_unstable();
        colours.dedup();
        assert!(colours.len() > 20, "only {} woven colours", colours.len());
    }

    #[test]
    fn a_texel_inside_a_garments_face_is_painted_by_that_garment() {
        // The waist: the top and the trousers meet on one chart, and the top,
        // baked second, used to paint its margin over the trousers' own
        // texels — a band of the top's colour round the waistband. Asked of
        // every texel whose centre lies inside a face of the plain trousers
        // as drawn: it holds their colour, whatever is baked beside it.
        let mut params = OutfitParams::default();
        params.top.texture = woven();
        params.trousers.colour = [0.2, 0.3, 0.8];
        let body = dressed(&params);
        let side = side(crate::AvatarConfig::default().atlas);
        let map = paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, side)
            .expect("a textured outfit has an atlas");
        let blue = [byte(0.2), byte(0.3), byte(0.8)];
        let trousers = &body.outfit.garments[0];
        assert!(trousers.texture.is_none(), "the trousers are drawn first");
        let drawn = trousers.charted(&body.unwrap, &body.face_of);
        let scale = side as f32;
        let (mut inside, mut wrong) = (0usize, Vec::new());
        for face in (0..trousers.claim.len()).map(|covered| covered * 2) {
            let pixels: Vec<glam::Vec2> = drawn.faces[face]
                .iter()
                .map(|&vertex| drawn.uvs[vertex as usize] * scale)
                .collect();
            for fan in 1..pixels.len() - 1 {
                let tri = [pixels[0], pixels[fan], pixels[fan + 1]];
                let edge = |a: glam::Vec2, b: glam::Vec2, c: glam::Vec2| {
                    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
                };
                let area = edge(tri[0], tri[1], tri[2]);
                if area.abs() < 1e-6 {
                    continue;
                }
                let lo = tri[0].min(tri[1]).min(tri[2]).floor();
                let hi = tri[0].max(tri[1]).max(tri[2]).ceil();
                for y in (lo.y as u32)..(hi.y as u32).min(side) {
                    for x in (lo.x as u32)..(hi.x as u32).min(side) {
                        let point = glam::Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                        // Clear of every edge by a thousandth of a texel, so a
                        // centre on the seam itself asks nothing.
                        let clear = [
                            edge(tri[1], tri[2], point),
                            edge(tri[2], tri[0], point),
                            edge(tri[0], tri[1], point),
                        ]
                        .into_iter()
                        .zip([(1, 2), (2, 0), (0, 1)])
                        .all(|(raw, (a, b))| raw * area.signum() / tri[a].distance(tri[b]) > 1e-3);
                        if !clear {
                            continue;
                        }
                        inside += 1;
                        let at = ((y * side + x) * 4) as usize;
                        if map.albedo[at..at + 3] != blue {
                            wrong.push((
                                x,
                                y,
                                [map.albedo[at], map.albedo[at + 1], map.albedo[at + 2]],
                            ));
                        }
                    }
                }
            }
        }
        assert!(inside > 1000, "only {inside} texels inside the trousers");
        assert!(
            wrong.is_empty(),
            "{} of {inside} trouser texels painted another colour, first {:?}",
            wrong.len(),
            &wrong[..wrong.len().min(4)]
        );
    }

    #[test]
    fn a_rim_is_drawn_from_its_own_garments_cloth() {
        // A rim spans no area in the atlas. Charted on its hem edge, a rim at
        // the waist samples the other garment's texels across the seam;
        // charted into its own face it cannot.
        let mut params = OutfitParams::default();
        params.top.texture = woven();
        params.trousers.colour = [0.2, 0.3, 0.8];
        let body = dressed(&params);
        let side = side(crate::AvatarConfig::default().atlas);
        let map = paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, side)
            .expect("a textured outfit has an atlas");
        let blue = [byte(0.2), byte(0.3), byte(0.8)];
        // Nearest, as the reference renderer samples.
        let texel = |uv: glam::Vec2| {
            let x = ((uv.x * side as f32) as u32).min(side - 1);
            let y = ((uv.y * side as f32) as u32).min(side - 1);
            let at = ((y * side + x) * 4) as usize;
            [map.albedo[at], map.albedo[at + 1], map.albedo[at + 2]]
        };
        let trousers = &body.outfit.garments[0];
        assert!(trousers.texture.is_none(), "the trousers are drawn first");
        let drawn = trousers.charted(&body.unwrap, &body.face_of);
        let rims = trousers.claim.len() * 2..trousers.mesh.faces.len();
        assert!(!rims.is_empty(), "the trousers have no rim");
        let (mut drawn_wrong, mut on_edge_wrong, mut samples) = (0, 0, 0);
        for face in rims {
            let points: Vec<glam::Vec2> = drawn.faces[face]
                .iter()
                .map(|&vertex| drawn.uvs[vertex as usize])
                .collect();
            // The same corners where the hem edge itself is charted.
            let edge: Vec<glam::Vec2> = trousers.corner_source[face]
                .iter()
                .map(|&(body_face, corner)| {
                    let index = body.face_of[body_face as usize].expect("charted");
                    body.unwrap.uvs[body.unwrap.faces[index as usize][usize::from(corner)] as usize]
                })
                .collect();
            for step in 0..=8 {
                let t = step as f32 / 8.0;
                samples += 1;
                drawn_wrong += usize::from(texel(points[0].lerp(points[1], t)) != blue);
                on_edge_wrong += usize::from(texel(edge[0].lerp(edge[1], t)) != blue);
            }
        }
        assert!(
            on_edge_wrong > 0,
            "no rim reaches the waist seam, so this asks nothing"
        );
        assert_eq!(
            drawn_wrong, 0,
            "{drawn_wrong} of {samples} rim samples off the trousers' own cloth \
             ({on_edge_wrong} on the hem edge itself)"
        );
    }

    #[test]
    fn a_texture_is_tinted_by_its_garments_colour() {
        // White shows the tile as painted; a dark colour darkens every texel.
        let mean = |colour: [f32; 3]| {
            let mut params = OutfitParams::default();
            params.top.texture = woven();
            params.top.colour = colour;
            params.trousers.texture = woven();
            params.trousers.colour = colour;
            let body = dressed(&params);
            let map =
                paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, 64).expect("an atlas");
            let sum: f64 = map.albedo.iter().map(|&b| f64::from(b)).sum();
            sum / map.albedo.len() as f64
        };
        assert!(mean([0.2, 0.2, 0.2]) < mean([1.0, 1.0, 1.0]) * 0.7);
    }

    #[test]
    fn painted_relief_is_unit_length_and_faces_out() {
        let mut params = OutfitParams::default();
        params.top.texture = woven();
        params.trousers.texture = woven();
        let body = dressed(&params);
        let map =
            paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, 64).expect("an atlas");
        for texel in map.normal.chunks_exact(4) {
            let n = Vec3::new(
                f32::from(texel[0]),
                f32::from(texel[1]),
                f32::from(texel[2]),
            ) / 255.0
                * 2.0
                - Vec3::ONE;
            assert!((n.length() - 1.0).abs() < 0.02, "{n:?}");
            assert!(n.z > 0.0, "a relief normal facing into the cloth: {n:?}");
        }
    }

    #[test]
    fn a_trunk_panel_measures_round_the_body_as_arc_length() {
        // The ellipse correction, asked of the real torso: going once round
        // the trunk panel is its circumference, which for the default body's
        // chest is well over half a metre and under a metre and a half.
        let mut params = OutfitParams::default();
        params.top.texture = woven();
        let body = dressed(&params);
        let top = body.outfit.garments.last().expect("a top");
        let panels = Panel::wrap(top, &body.rig);
        let trunk = &panels[0];
        let (a, b) = trunk.semi;
        let around = std::f32::consts::PI * (a + b);
        assert!((0.5..1.5).contains(&around), "{around} m round the chest");
    }

    #[test]
    fn baking_is_deterministic() {
        let mut params = OutfitParams::default();
        params.trousers.texture = woven();
        let body = dressed(&params);
        let bake = || {
            paint(&body.outfit, &body.rig, &body.unwrap, &body.face_of, 64)
                .map(|map| (map.albedo, map.normal, map.roughness))
        };
        assert_eq!(bake(), bake());
    }
}
