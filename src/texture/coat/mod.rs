//! Painting a beast's coat into a baked atlas (#368).
//!
//! The skin painter's job, done for fur: [`paint_coat`] walks the same baked
//! texels and writes the same three maps, but every colour comes from the
//! coat's loci ([`crate::plan::Coat`]) through a [`palette`], and every mark
//! is placed by the beast's anatomy ([`body`]) and cut from body-space fields
//! ([`marks`]) - so a stripe crosses a chart's seam unbroken.
//!
//! The order is a breeder's: the agouti locus spreads the dark and the
//! light, a pattern draws over that, a colourpoint pales the body, the belly
//! lightens, merle, tortie, dun and roan break the colour up, and white
//! spotting goes over all of it - as it does in life, where white is the
//! absence of every pigment the rest put there. Bare parts are painted from
//! the same loci: the nose and pads its leather, the claws and hooves its
//! horn - the pads pink and the horn pale under a white foot, the nose pink
//! where piebald or extreme white reaches it. The inside of the mouth, which
//! the jaw and the tongue mark through the bake, is flesh on every coat.

mod body;
mod marks;
mod palette;

pub(crate) use body::CoatBody;

use glam::Vec3;
use symbios_texture::generator::TextureMap;
use symbios_texture::normal::{BoundaryMode, height_to_normal};

use super::bake::{AtlasGeometry, Texel};
use crate::plan::{Agouti, Coat, Feet, Pattern, Pigment, White, Zone};
use crate::rig::Rig;
use body::{leg, topside, trunk, underside};
use marks::{Fields, step};
use palette::{PALE_HORN, PINK, Palette, WHITE};

/// How strongly the fur's grain stands out of the normal map.
/// Provenance: **tuned by render** (#368), against the skin's 0.15.
const FUR_RELIEF: f32 = 0.25;

/// Each surface's roughness: fur dull, leather and horn with a sheen.
/// Provenance: **tuned by render** (#368).
const FUR_ROUGHNESS: f32 = 0.78;
const LEATHER_ROUGHNESS: f32 = 0.42;
const HORN_ROUGHNESS: f32 = 0.35;
const MOUTH_ROUGHNESS: f32 = 0.35;

/// What the atlas carries under the inside of a mouth: near white, so the
/// colours the jaw and the tongue carry on their vertices read as themselves
/// over any coat - a black dog's tongue is pink.
const FLESH: Vec3 = Vec3::new(0.96, 0.90, 0.90);

/// How a pattern's stripes waver: by what share of a band, and how often
/// per back height.
#[derive(Clone, Copy, Debug)]
struct Waver {
    share: f32,
    frequency: f32,
}

/// A tabby's stripes: gently.
/// Provenance: **tuned by render** (#368).
const TABBY: Waver = Waver {
    share: 0.35,
    frequency: 3.0,
};

/// A brindle's: much more, and more often, so no two streaks run alike.
/// Provenance: **tuned by render** (#368).
const BRINDLE: Waver = Waver {
    share: 0.6,
    frequency: 7.0,
};

/// What a texel is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Surface {
    /// The coat.
    Fur,
    /// Bare leather: the nose and the pads.
    Leather,
    /// Horn: a claw or a hoof, by how much of the texel is horn rather
    /// than coat - a hoof's rim is blended in under its coronet.
    Horn(f32),
}

/// Paints a beast's coat into the atlas its body was baked into.
#[must_use]
pub(crate) fn paint_coat(
    geometry: &AtlasGeometry,
    rig: &Rig,
    coat: &Coat,
    body: &CoatBody,
) -> TextureMap {
    let palette = Palette::of(coat);
    let fields = Fields::new(coat.marks, body.height);
    let count = geometry.texels.len();
    let mut albedo = vec![0u8; count * 4];
    let mut orm = vec![0u8; count * 4];
    let mut heights = vec![0.0f64; count];

    for (index, sample) in geometry.texels.iter().enumerate() {
        let Some(texel) = sample else { continue };
        let painter = Painter {
            texel,
            coat,
            palette: &palette,
            fields: &fields,
            body,
        };
        let surface = painter.surface();
        let (colour, roughness, fur) = match surface {
            Surface::Fur => (painter.fur(), FUR_ROUGHNESS, 1.0),
            Surface::Leather => (painter.leather(), LEATHER_ROUGHNESS, 0.0),
            Surface::Horn(share) => (
                painter.fur().lerp(painter.horn(), share),
                FUR_ROUGHNESS + (HORN_ROUGHNESS - FUR_ROUGHNESS) * share,
                1.0 - share,
            ),
        };
        // The inside of a mouth is flesh, whatever the coat: the jaw's and the
        // tongue's, which carry it as a channel of their own through the bake.
        let mouth = texel.mouth.clamp(0.0, 1.0);
        let colour = colour.lerp(FLESH, mouth);
        let roughness = roughness + (MOUTH_ROUGHNESS - roughness) * mouth;
        let fur = fur * (1.0 - mouth);
        if fur > 0.0 {
            heights[index] = f64::from(painter.grain(rig) * fur);
        }
        let at = index * 4;
        let rgb = colour.clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        albedo[at..at + 4].copy_from_slice(&[rgb.x as u8, rgb.y as u8, rgb.z as u8, 255]);
        let occlusion = (1.0 - texel.crease * 0.7) * 255.0;
        orm[at..at + 4].copy_from_slice(&[occlusion as u8, (roughness * 255.0) as u8, 0, 255]);
    }

    TextureMap {
        albedo,
        normal: height_to_normal(
            &heights,
            geometry.width,
            geometry.height,
            FUR_RELIEF,
            BoundaryMode::Clamp,
        ),
        roughness: orm,
        emissive: None,
        width: geometry.width,
        height: geometry.height,
        mip_level_count: 1,
    }
}

/// One texel and everything its colour is read from.
struct Painter<'a> {
    texel: &'a Texel,
    coat: &'a Coat,
    palette: &'a Palette,
    fields: &'a Fields,
    body: &'a CoatBody,
}

impl Painter<'_> {
    /// What this texel is: asked of the declared head and feet.
    ///
    /// Not whether it is the inside of the mouth, which the jaw and the
    /// tongue say themselves through the texel's mouth channel: asked of the
    /// declared head, everything sunk into it would answer - the root of
    /// every ear, the lids. The palate is the head's own skin, which the
    /// shut jaw covers, and it wears the coat under the tint its vertices
    /// carry.
    fn surface(&self) -> Surface {
        let at = self.texel.position;
        if let Some(head) = &self.body.head
            && self.texel.zone == Zone::Head
            && head.on_leather(head.local(at), 0.02 * head.radius)
        {
            return Surface::Leather;
        }
        if let Some((foot, local)) = self.body.foot(self.texel.zone, at) {
            let horn = foot.horn(local);
            if horn > 0.0 {
                return Surface::Horn(horn);
            }
            if foot.kind != Feet::Hooves && local.y < 0.06 * foot.pad {
                return Surface::Leather;
            }
        }
        Surface::Fur
    }

    /// The coat at this texel, every locus applied in a breeder's order.
    fn fur(&self) -> Vec3 {
        let mut colour = self.agouti();
        // A brindle is striped over a light coat, as a fawn brindle's is: on
        // a solid one it brings the light out to stripe, where in life the
        // stripes would be lost in the dark.
        if self.coat.pattern == Pattern::Brindle && self.coat.agouti == Agouti::Solid {
            colour = self.palette.light;
        }
        colour = colour.lerp(self.palette.dark, self.pattern());
        if self.coat.pattern == Pattern::Dapple {
            colour = self.dapple(colour);
        }
        if self.coat.points {
            colour = self.pointed(colour);
        }
        colour = self.countershaded(colour);
        colour = self.broken(colour);
        colour.lerp(WHITE, self.white())
    }

    /// The nose's or a pad's leather: a pad pink under any white that
    /// reaches it, the nose only where piebald or extreme white does - a
    /// blaze runs down to a dark nose.
    fn leather(&self) -> Vec3 {
        let bare = if leg(self.texel.zone) {
            self.white()
        } else {
            match self.coat.white {
                White::Piebald | White::Extreme => self.white(),
                _ => 0.0,
            }
        };
        self.palette.leather.lerp(PINK, bare)
    }

    /// A claw's or a hoof's horn: pale under a white sock.
    fn horn(&self) -> Vec3 {
        self.palette.horn.lerp(PALE_HORN, self.white())
    }

    /// How the agouti locus spreads the dark and the light here.
    fn agouti(&self) -> Vec3 {
        let (at, normal) = (self.texel.position, self.texel.normal);
        let palette = self.palette;
        match self.coat.agouti {
            Agouti::Solid => palette.ground,
            // Tipped dark along the back, the tipping broken a little.
            Agouti::Sable => {
                let saddle =
                    step(0.15, 0.75, topside(normal)) * (0.8 + 0.2 * self.fields.warp(at, 6.0));
                palette.light.lerp(palette.dark, saddle * 0.85)
            }
            // Every hair banded: a grizzle of the two.
            Agouti::Wild => {
                let band = 0.42 + 0.22 * self.fields.grain(at, 90.0);
                palette.light.lerp(palette.dark, band)
            }
            Agouti::TanPoints => palette.ground.lerp(palette.light, self.tan()),
            Agouti::Bay => palette.light.lerp(palette.dark, self.bay()),
        }
    }

    /// Where a black-and-tan's tan is: the lower legs, the muzzle and
    /// cheeks, a spot over each eye, the chest and the underside of the
    /// tail.
    fn tan(&self) -> f32 {
        let (at, normal, zone) = (self.texel.position, self.texel.normal, self.texel.zone);
        let legs = if leg(zone) {
            step(0.42, 0.30, self.body.rise(at))
        } else {
            0.0
        };
        let face = self.head_local().map_or(0.0, |local| {
            let muzzle = step(0.45, 0.75, local.z) * step(0.15, -0.05, local.y);
            let brow = Vec3::new(local.x.abs(), local.y, local.z);
            let spot = step(0.16, 0.10, brow.distance(Vec3::new(0.38, 0.40, 0.62)));
            muzzle.max(spot)
        });
        let chest = if zone == Zone::Chest || zone == Zone::Neck {
            step(0.2, 0.6, underside(normal))
                * step(0.0, 0.1, (at - self.body.withers).z / self.body.height)
        } else {
            0.0
        };
        let tail = if zone == Zone::Tail {
            step(0.2, 0.6, underside(normal))
        } else {
            0.0
        };
        legs.max(face).max(chest).max(tail)
    }

    /// Where a bay's black is: the lower legs, the tail, the ear tips and
    /// round the nose.
    fn bay(&self) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let legs = if leg(zone) {
            step(0.50, 0.38, self.body.rise(at))
        } else {
            0.0
        };
        let tail = if zone == Zone::Tail {
            step(0.2, 0.5, self.body.along_tail(at))
        } else {
            0.0
        };
        let face = self.head_local().map_or(0.0, |local| {
            let ears = step(0.95, 1.15, local.y);
            let nose = self.body.head.as_ref().map_or(0.0, |head| {
                step(0.45, 0.25, local.distance(head.nose_centre() / head.radius))
            });
            ears.max(nose)
        });
        legs.max(tail).max(face)
    }

    /// How strongly a pattern's dark mark lies here, `0..=1`.
    fn pattern(&self) -> f32 {
        let (at, normal, zone) = (self.texel.position, self.texel.normal, self.texel.zone);
        let fields = self.fields;
        // A pattern fades out under the belly, as a tabby's does.
        let belly = 1.0 - 0.7 * step(0.3, 0.9, underside(normal));
        let mark = match self.coat.pattern {
            Pattern::None | Pattern::Dapple => 0.0,
            Pattern::Mackerel => step(0.25, 0.55, self.stripes(7.0, TABBY)),
            // Thin streaks, crowded, wavering and broken.
            Pattern::Brindle => {
                let broken = step(-0.55, -0.1, fields.patches(at, 12.0));
                step(0.45, 0.75, self.stripes(17.0, BRINDLE)) * broken * 0.85
            }
            Pattern::Ticked => {
                if leg(zone) {
                    step(0.55, 0.85, self.stripes(6.0, TABBY)) * 0.6
                } else {
                    0.0
                }
            }
            // A swirl round each flank's middle, giving way to a tabby's
            // stripes away from it: over the shoulders, down the legs and
            // the tail.
            Pattern::Blotched => {
                let (swirl, flank) = self.swirl();
                let stripes = step(0.25, 0.55, self.stripes(7.0, TABBY));
                stripes + (step(0.2, 0.5, swirl) - stripes) * flank
            }
            Pattern::Spotted => step(0.36, 0.26, fields.cells(at, 9.0).0),
            // A ring round a centre a shade darker than the ground, wavering
            // and broken into three or four marks, as a leopard's is.
            Pattern::Rosettes => {
                let near = fields.cells(at, 6.5).0 + 0.07 * fields.warp(at, 16.0);
                let ring = step(0.20, 0.29, near) * step(0.47, 0.38, near);
                let broken = step(-0.45, -0.05, fields.patches(at, 26.0));
                (ring * broken).max(0.3 * step(0.27, 0.17, near))
            }
        };
        mark * belly
    }

    /// Bands round the body at `count` per spine length, rings round the
    /// legs and the tail, every one wavering by `waver`: `-1..=1`.
    fn stripes(&self, count: f32, waver: Waver) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let body = self.body;
        let phase = if leg(zone) {
            body.rise(at) * count * 1.6
        } else if zone == Zone::Tail {
            body.tail_tip_distance(at) * count * 2.0
        } else {
            body.along_spine(at) * count
        };
        let wavering = waver.share * self.fields.warp(at, waver.frequency);
        (std::f32::consts::TAU * (phase + wavering)).sin()
    }

    /// A blotched tabby's bullseye, ringed round the middle of each flank,
    /// `-1..=1`, and how much of that middle `at` is in, `0..=1`.
    fn swirl(&self) -> (f32, f32) {
        let at = self.texel.position;
        let (body, fields) = (self.body, self.fields);
        let centre = body.hips.lerp(body.withers, 0.45) + Vec3::Y * (0.05 * body.height);
        let across = Vec3::new(0.0, at.y - centre.y, at.z - centre.z).length() / body.height;
        let wavering = 0.4 * fields.warp(at, 2.5) + 0.1 * fields.warp(at, 8.0);
        let rings = (std::f32::consts::TAU * (across * 4.5 + wavering)).sin();
        (rings, step(0.6, 0.4, across + 0.08 * fields.warp(at, 4.0)))
    }

    /// A dapple grey's coat: the colour greyed, with lighter rounds over
    /// it, the grey between them reading as a darker net.
    fn dapple(&self, colour: Vec3) -> Vec3 {
        let at = self.texel.position;
        let near = self.fields.cells(at, 6.0).0 + 0.06 * self.fields.warp(at, 20.0);
        let grey = colour.lerp(Vec3::new(0.42, 0.42, 0.44), 0.6);
        grey.lerp(grey.lerp(WHITE, 0.45), step(0.6, 0.34, near))
    }

    /// A colourpoint: the body paled, the face, ears, legs and tail left
    /// their colour.
    fn pointed(&self, colour: Vec3) -> Vec3 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let pale = colour.lerp(WHITE.lerp(self.palette.light, 0.15), 0.78);
        let point = if leg(zone) {
            step(0.62, 0.30, self.body.rise(at))
        } else if zone == Zone::Tail {
            step(0.0, 0.35, self.body.along_tail(at))
        } else {
            // A mask round the nose that takes the eyes in, and the ears.
            self.head_local().map_or(0.0, |local| {
                let nose = self
                    .body
                    .head
                    .as_ref()
                    .map_or(Vec3::Z, |head| head.nose_centre() / head.radius);
                step(1.1, 0.6, local.distance(nose)).max(step(0.85, 1.05, local.y))
            })
        };
        pale.lerp(colour, point)
    }

    /// Lighter underneath: every coat's belly and the inside of its legs.
    fn countershaded(&self, colour: Vec3) -> Vec3 {
        let under = step(0.15, 0.85, underside(self.texel.normal));
        colour.lerp(
            colour.lerp(self.palette.light.lerp(WHITE, 0.5), 0.5),
            under * 0.3,
        )
    }

    /// The loci that break a coat's colour up: merle, tortie, dun and roan.
    fn broken(&self, mut colour: Vec3) -> Vec3 {
        let at = self.texel.position;
        let (coat, fields) = (self.coat, self.fields);
        if coat.merle {
            // The dark broken into patches over a diluted ground.
            let patch = fields.patches(at, 5.0);
            let diluted = colour.lerp(Vec3::new(0.56, 0.57, 0.60), 0.62);
            colour = diluted.lerp(colour, step(0.10, 0.30, patch));
        }
        if coat.tortie && coat.pigment != Pigment::Red {
            colour = colour.lerp(
                self.palette.light,
                step(0.05, 0.25, fields.patches(at * 1.3, 4.0)),
            );
        }
        if coat.dun {
            let diluted = colour.lerp(self.palette.light.lerp(WHITE, 0.3), 0.5);
            colour = colour.lerp(diluted, 0.8 * self.dun_body());
            colour = colour.lerp(self.palette.dark, 0.8 * self.dun_marks());
        }
        if coat.roan {
            let hairs = step(-0.1, 0.45, fields.grain(at, 70.0));
            colour = colour.lerp(WHITE, 0.5 * hairs * self.roan());
        }
        colour
    }

    /// Where a roan's white hairs grow, `0..=1`: through the body and the
    /// neck, thinning down the upper legs, along the tail and out before the
    /// head, which a roan keeps solid.
    fn roan(&self) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let body = if trunk(zone) || zone == Zone::Neck {
            1.0
        } else if matches!(zone, Zone::UpperLimb(_)) {
            step(0.4, 0.65, self.body.rise(at))
        } else if zone == Zone::Tail {
            step(0.45, 0.15, self.body.along_tail(at))
        } else {
            0.0
        };
        body * self.off_head()
    }

    /// How much of a dun's dilution reaches here, `0..=1`: all of the body,
    /// thinning out down the legs and toward the tail's tip, which keep
    /// their colour.
    fn dun_body(&self) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        if leg(zone) {
            step(0.25, 0.45, self.body.rise(at))
        } else if zone == Zone::Tail {
            step(0.6, 0.2, self.body.along_tail(at))
        } else {
            1.0
        }
    }

    /// A dun's dark marks, `0..=1`: a stripe down the middle of the back,
    /// from behind the head on along the tail, and bars across the lower
    /// legs.
    fn dun_marks(&self) -> f32 {
        let (at, normal, zone) = (self.texel.position, self.texel.normal, self.texel.zone);
        let rise = self.body.rise(at);
        let stripe = step(0.05, 0.03, at.x.abs() / self.body.height)
            * step(0.6, 0.85, topside(normal))
            * self.off_head();
        let bars = if leg(zone) {
            step(0.45, 0.35, rise) * step(0.3, 0.8, (std::f32::consts::TAU * rise * 9.0).sin())
        } else {
            0.0
        };
        stripe.max(bars)
    }

    /// How far off the head this texel lies, `0` on it to `1` a head radius
    /// away: what lets a mark that stops short of the face fade out there
    /// rather than at the line between two zones.
    fn off_head(&self) -> f32 {
        let at = self.texel.position;
        self.body.head.as_ref().map_or(1.0, |head| {
            step(0.3, 1.0, head.distance(head.local(at)) / head.radius)
        })
    }

    /// How white the coat is here, `0..=1`: the white spotting locus.
    fn white(&self) -> f32 {
        let (at, normal, zone) = (self.texel.position, self.texel.normal, self.texel.zone);
        let fields = self.fields;
        let socks = |height: f32| {
            if leg(zone) {
                step(
                    height + 0.03,
                    height - 0.03,
                    self.body.rise(at) + 0.02 * fields.warp(at, 8.0),
                )
            } else {
                0.0
            }
        };
        match self.coat.white {
            White::None => 0.0,
            White::Socks => socks(0.16),
            White::Irish => socks(0.24)
                .max(self.blaze())
                .max(self.bib(normal, zone))
                .max(step(0.14, 0.10, self.body.tail_tip_distance(at))),
            White::Piebald => {
                let lean = 0.55 * underside(normal) - 0.25 * topside(normal)
                    + if leg(zone) { 0.3 } else { 0.0 }
                    - if zone == Zone::Head { 0.4 } else { 0.0 };
                step(0.05, 0.25, fields.patches(at, 2.2) + lean)
            }
            White::Extreme => {
                let kept = if zone == Zone::Head {
                    self.head_local()
                        .map_or(0.0, |local| step(0.15, 0.4, local.y))
                } else {
                    // The tail's root and a patch over the croup it springs
                    // from.
                    let tail = if zone == Zone::Tail {
                        step(0.5, 0.2, self.body.along_tail(at))
                    } else {
                        0.0
                    };
                    tail.max(step(0.3, 0.18, self.body.tail_root_distance(at)))
                };
                1.0 - kept.max(step(0.45, 0.6, fields.patches(at, 2.0)) * 0.8)
            }
        }
    }

    /// A white blaze down the middle of the face, from the forehead to the
    /// nose, on the faces that look up or ahead: not under the chin.
    fn blaze(&self) -> f32 {
        let (Some(head), Some(local)) = (self.body.head.as_ref(), self.head_local()) else {
            return 0.0;
        };
        let (at, normal) = (self.texel.position, self.texel.normal);
        let facing = head.local(at + normal) - head.local(at);
        let width = 0.16 + 0.04 * self.fields.warp(at, 10.0);
        step(width + 0.04, width, local.x.abs())
            * step(0.30, 0.42, local.z)
            * step(1.0, 0.85, local.y)
            * step(-0.45, -0.15, facing.y)
    }

    /// A white bib: down the middle of the chin, the throat, the chest and
    /// the belly - on the head only what faces the ground, where the chest's
    /// front counts too.
    fn bib(&self, normal: Vec3, zone: Zone) -> f32 {
        if leg(zone) || zone == Zone::Tail {
            return 0.0;
        }
        let width = 0.12 * (1.0 + 0.4 * self.fields.warp(self.texel.position, 4.0));
        let middle = step(
            width + 0.03,
            width,
            self.texel.position.x.abs() / self.body.height,
        );
        let facing = if zone == Zone::Head {
            underside(normal)
        } else {
            underside(normal).max(normal.z * 0.8)
        };
        middle * step(0.3, 0.6, facing)
    }

    /// The fur's grain at this texel: noise stretched along the bone it
    /// grows on, which the normal map turns into strands catching the light.
    fn grain(&self, rig: &Rig) -> f32 {
        let at = self.texel.position;
        let hit = rig.nearest_bone(at);
        let (from, to) = rig.bone(hit.joint);
        let along = (to - from).normalize_or(Vec3::Y);
        let across = along.any_orthonormal_vector();
        let other = along.cross(across);
        let height = self.body.height;
        let stretched = Vec3::new(
            at.dot(along) * 8.0,
            at.dot(across) * 140.0,
            at.dot(other) * 140.0,
        ) / height;
        0.35 * self.fields.grain(stretched * height, 1.0)
    }

    /// This texel in the declared head's frame, in head radii, if the body
    /// has one and the texel is the head's.
    fn head_local(&self) -> Option<Vec3> {
        if self.texel.zone != Zone::Head {
            return None;
        }
        let head = self.body.head.as_ref()?;
        Some(head.local(self.texel.position) / head.radius)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Family, Nose, QuadrupedParams};
    use crate::{Archetype, Avatar, AvatarRecord, Rig};

    /// A beast of `family` with its family's parts, as `tweak` leaves it,
    /// built, and the palette its coat was painted from.
    fn painted(family: Family, tweak: impl FnOnce(&mut QuadrupedParams)) -> (Avatar, Palette) {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        tweak(&mut params);
        let palette = Palette::of(&params.coat);
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
        (Avatar::build(&record).expect("the beast builds"), palette)
    }

    /// How many of the atlas's texels are `colour`, as the atlas stores it,
    /// give or take a step of rounding.
    fn texels_of(avatar: &Avatar, colour: Vec3) -> usize {
        let rgb = colour.clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        let stored = [rgb.x as u8, rgb.y as u8, rgb.z as u8];
        avatar
            .skin
            .albedo
            .chunks_exact(4)
            .filter(|texel| (0..3).all(|channel| texel[channel].abs_diff(stored[channel]) <= 1))
            .count()
    }

    #[test]
    fn a_leather_nose_wears_its_coat_s_leather_and_a_soft_muzzle_does_not() {
        // A horse's, whose feet are horn: any leather on it is its nose's.
        let leathered = |nose: Nose| {
            let (avatar, palette) = painted(Family::Horse, |params| params.nose = nose);
            texels_of(&avatar, palette.leather)
        };
        assert!(
            leathered(Nose::Leather) > 20,
            "a leather nose went unpainted"
        );
        assert_eq!(leathered(Nose::Soft), 0, "a soft muzzle wore leather");
    }

    #[test]
    fn a_hoof_and_a_dog_s_claws_are_horn_and_a_cat_s_are_not() {
        let horned = |family: Family| {
            let (avatar, palette) = painted(family, |_| ());
            texels_of(&avatar, palette.horn)
        };
        assert!(horned(Family::Horse) > 20, "a horse went unshod");
        assert!(horned(Family::Dog) > 0, "a dog's claws went unpainted");
        assert_eq!(horned(Family::Cat), 0, "a cat's claws are sheathed");
    }

    /// A beast of `family`'s body as the painter reads it: its rig's spine
    /// and tail, and the head and feet its plan declares.
    fn coat_body(family: Family) -> CoatBody {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        let skeleton = Archetype::Quadruped(params).skeleton(&crate::plan::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("the beast rigs");
        CoatBody::of(
            &rig,
            skeleton.beast_head.as_ref(),
            skeleton.beast_feet.as_ref(),
        )
    }

    /// What `ask` makes of `coat` on `body`, at a texel at `position`
    /// facing along `normal` in `zone`.
    fn paint<R>(
        body: &CoatBody,
        coat: &Coat,
        (position, normal, zone): (Vec3, Vec3, Zone),
        ask: impl FnOnce(&Painter) -> R,
    ) -> R {
        let (palette, fields) = (Palette::of(coat), Fields::new(coat.marks, body.height));
        let texel = Texel {
            position,
            normal,
            crease: 0.0,
            mouth: 0.0,
            zone,
        };
        ask(&Painter {
            texel: &texel,
            coat,
            palette: &palette,
            fields: &fields,
            body,
        })
    }

    #[test]
    fn a_sock_is_white_below_its_line_and_not_above_it() {
        // The marks' steps fall as often as they rise, and a falling one
        // read backwards once painted every sock from the knee up.
        let body = coat_body(Family::Dog);
        let coat = Coat {
            pigment: Pigment::Black,
            white: White::Socks,
            ..Coat::default()
        };
        let white = |rise: f32| {
            let at = Vec3::new(0.1, rise * body.height, 0.2);
            let zone = Zone::LowerLimb(crate::plan::Limb::ForeLeft);
            paint(&body, &coat, (at, Vec3::X, zone), |painter| painter.white())
        };
        assert!(white(0.05) > 0.99, "the foot went unsocked");
        assert!(white(0.5) < 0.01, "the knee wore the sock");
    }

    #[test]
    fn an_extreme_white_is_white_over_the_barrel_and_keeps_its_croup() {
        // The colour an extreme white keeps at the tail's root once reached
        // every texel the tail's root was nearer than its tip: the whole
        // barrel, which came out a wash where it should be white.
        for family in Family::ALL {
            let body = coat_body(family);
            let (root, _) = body.tail.expect("a beast has a tail");
            let mut whites = Vec::new();
            for marks in 0..16u16 {
                let coat = Coat {
                    pigment: Pigment::Black,
                    white: White::Extreme,
                    marks: marks * 4099,
                    ..Coat::default()
                };
                for step in 0..=10u8 {
                    let along = 0.25 + 0.06 * f32::from(step);
                    let at = body.hips.lerp(body.withers, along)
                        + Vec3::new(0.12, -0.15, 0.0) * body.height;
                    whites.push(paint(
                        &body,
                        &coat,
                        (at, Vec3::X, Zone::Abdomen),
                        |painter| painter.white(),
                    ));
                }
                let croup = paint(
                    &body,
                    &coat,
                    (root + Vec3::Y * 0.02, Vec3::Y, Zone::Pelvis),
                    |painter| painter.white(),
                );
                assert!(croup < 0.2, "{family:?}: the croup went white");
            }
            // Near 0.98 on every family; the ungated term left a horse's 0.88
            // and a cat's 0.45.
            let mean = whites.iter().sum::<f32>() / whites.len() as f32;
            assert!(
                mean > 0.93,
                "{family:?}: the barrel is only {mean:.2} white"
            );
        }
    }

    #[test]
    fn a_blaze_runs_down_the_face_and_not_under_the_chin() {
        let body = coat_body(Family::Horse);
        let head = body.head.expect("a beast declares its head");
        let coat = Coat {
            white: White::Irish,
            ..Coat::default()
        };
        let local = Vec3::new(0.0, 0.3, 0.8) * head.radius;
        let at = head.body(local);
        let up = head.body(local + Vec3::Y) - at;
        let blaze = |normal| {
            paint(&body, &coat, (at, normal, Zone::Head), |painter| {
                painter.blaze()
            })
        };
        assert!(blaze(up) > 0.9, "the face went unblazed");
        assert_eq!(blaze(-up), 0.0, "the blaze ran under the chin");
    }

    #[test]
    fn a_black_beast_s_tongue_is_flesh() {
        // The jaw and the tongue carry the inside of the mouth through the
        // bake, so the coat stops at the lips: painted as fur, a black dog's
        // tongue was black under its tint.
        let (avatar, _) = painted(Family::Dog, |params| {
            params.coat = Coat {
                pigment: Pigment::Black,
                ..Coat::default()
            };
        });
        assert!(texels_of(&avatar, FLESH) > 50, "the mouth went unfleshed");
    }

    #[test]
    fn the_body_wears_no_colour_but_the_palate_s() {
        let (avatar, _) = painted(Family::Dog, |_| ());
        let inside = crate::face::beast::jaw::INSIDE;
        let tint = &avatar.parts.tint;
        assert!(tint.contains(&inside), "the palate went untinted");
        assert!(
            tint.iter()
                .all(|&colour| colour == Vec3::ONE || colour == inside),
            "a coat's colours are the atlas's"
        );
    }
}
