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
//!
//! **Where a leg leaves the trunk, nothing asks the zone** (#376). Every mark
//! that differs between the barrel and a leg - a stripe's frame, a spot's
//! size, a point, a sock, a bay's black, a roan's and a dun's thinning, a
//! bib, a piebald's lean, the fur's grain - blends by how much of a leg a
//! texel is ([`body`]'s junctions), so it turns over a band at the elbow and
//! the stifle, as an animal's own pattern does, instead of on the line the
//! skin weights draw. Below it a leg wears its own frame - rings closer than
//! the barrel's bands, spots finer - and its colour regions where its zone
//! used to put them.
//!
//! **The skin under a dark mane is the mane's colour** (#389). A bay's or a
//! dun's mane is drawn in its dark, so where it leaves the neck the skin beside
//! it would show as a line of another colour. So the painter darkens the line the
//! mane grows from ([`ManeRoots`]), as a hair card's scalp is painted: nothing
//! else on the coat knows a mane is there, and no other coat changes. Since #407
//! the mane is one solid in one colour, whole, white spotting under it included,
//! and the line is painted that colour.
//!
//! **Nor where the neck meets the head** (#395), for the same reason as a leg:
//! the zone map's line there is nothing an animal's coat knows of, and a tan
//! throat, a bib, a piebald's lean, an extreme white's cap, a colourpoint's mask
//! and the fur's grain each stopped on it. They fade by how far a texel lies off
//! the head instead.
//!
//! **A creator's colours go in through the palette** (#401), and what each reaches
//! is stated on [`crate::plan::CoatColours`]. Nothing here asks a family, and a
//! mark's layout does not move. Two are new to the painter: a tail in a colour of
//! its own, which fades in from the rump by how far out along the dock a texel lies
//! ([`DOCK_FADE`]) so that the zone's line never shows, and a mane in a colour of
//! its own, which [`ManeRoots`] paints under it, light on a dark coat as readily
//! as dark on a light one.

mod body;
mod marks;
mod palette;
mod plumage;
mod winged;

pub(crate) use body::CoatBody;
pub use palette::{coat_colour, coat_swatches};
pub(crate) use plumage::{Feather, FeatherTone, PlumageSampler};
pub(crate) use winged::{MembraneChart, WingedCoat};

use glam::Vec3;
use symbios_texture::generator::TextureMap;
use symbios_texture::normal::{BoundaryMode, height_to_normal};

use super::bake::{AtlasGeometry, Texel};
use crate::det::DetMath;
use crate::plan::{Agouti, Coat, Feet, Pattern, Pigment, White, Zone};
use crate::rig::Rig;
use crate::rig::junction::LegShare;
use body::{leg, topside, trunk, underside};
use marks::{Fields, step};
use palette::{Palette, WHITE};

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

/// How much finer a pattern's marks are down a leg than over the barrel:
/// its rings closer than the barrel's bands, its spots smaller.
/// Provenance: **tuned by render** (#368), the rings' own share since then;
/// the spots' since #376.
const LEG_FINER: f32 = 1.6;

/// How far down a leg, in the radii of the joint it leaves the trunk at, a
/// pattern's frame takes to turn from the barrel's to the leg's: far enough
/// that the bands crossing a thick thigh close into rings without crowding.
/// Provenance: **derived** (#376): round a leg of radius `r` the bands
/// differ by about `count x r / spine` stripes, and closed over `TURN x r`
/// they press the stripes at most `1 / TURN` of the barrel's own density
/// closer.
const TURN: f32 = 1.5;

/// How many of the trunk's, neck's and head's bones a texel's grain reads at most:
/// where several meet, at the poll, a few at once.
const GRAIN_BONES: usize = 5;

/// How much further than the nearest bone another still gives a texel's grain any
/// of its own, as a share of the back's height: wide enough that the strands turn
/// over a band as wide as a neck's skin is off its bone.
/// Provenance: **tuned by render** (#395).
const GRAIN_BAND: f32 = 0.06;

/// How far short of the last bone read another has its whole share, as a share of
/// [`GRAIN_BAND`]. Provenance: **chosen** (#395).
const GRAIN_HANDOVER: f32 = 0.25;

/// How far out along the tail's first bone a picked colour takes to come in, in
/// back heights, starting past where the zone map's line can lie, so that the line
/// never shows. Cards rooted before it has come in start as the coat does and run to
/// the colour: on a horse about a third of them, which from behind reads as a small
/// patch of the coat where the tail leaves the rump.
/// Along the bone rather than from the tail's root, which reads the same line at
/// anything up to 0.3, because the line runs round the dock and across this measure.
/// Provenance: **measured** (#401): the line lies at 0.069 at most over 600 rolls,
/// and `a_tail_in_a_picked_colour_meets_the_rump_with_no_line` holds the margin;
/// **tuned by render** for its end.
const DOCK_FADE: (f32, f32) = (0.08, 0.14);

/// And by [`CoatBody::along_tail`], which a tail that curls back past its own
/// first bone still climbs. Provenance: **measured** (#401): the line lies at 0.25
/// at most, tails at the envelope's ends among them.
const TAIL_FADE: (f32, f32) = (0.26, 0.42);

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

/// How much darker than its coat a mane that follows the coat is drawn (#407): a
/// mane of exactly the coat's colour lying on the neck has no outline.
/// Provenance: **tuned by render** (#407), the owner's decision that it is "a
/// little darker"; 0.75 first, which a pale coat's mane (horse 11, cross 73) all
/// but lost at play size, then 0.6 at the owner's asking.
pub(crate) const MANE_SHADE: f32 = 0.6;

/// Whether a coat draws its long hair in its own dark: a bay's, a dun's.
pub(crate) fn darkens_points(coat: &Coat) -> bool {
    coat.agouti == Agouti::Bay || coat.dun
}

/// The skin a mane in a coat's dark grows out of (#389): the line it is rooted
/// along, which the painter darkens as hair cards' scalps are painted, so that
/// what shows where the mane leaves the neck is the mane again.
#[derive(Clone, Debug)]
pub(crate) struct ManeRoots {
    /// The line, and how far off it the colour reaches at each of its points.
    line: Vec<(Vec3, f32)>,
    /// A box round all of it, so that nearly every texel asks one question.
    bounds: (Vec3, Vec3),
}

impl ManeRoots {
    /// How much of a soft edge the colour has, as a share of its reach.
    /// Provenance: **tuned by render** (#389).
    const SOFT: f32 = 0.4;

    /// The roots along `line`, each reaching as far as it says.
    pub(crate) fn new(line: Vec<(Vec3, f32)>) -> Self {
        let most = line.iter().fold(0.0f32, |most, point| most.max(point.1));
        let bounds = line.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(low, high), point| {
                (
                    low.min(point.0 - Vec3::splat(most)),
                    high.max(point.0 + Vec3::splat(most)),
                )
            },
        );
        Self { line, bounds }
    }

    /// How much of the mane's colour the skin at `at` takes, `0..=1`.
    fn share(&self, at: Vec3) -> f32 {
        if at.cmplt(self.bounds.0).any() || at.cmpgt(self.bounds.1).any() {
            return 0.0;
        }
        let mut most = 0.0f32;
        for piece in self.line.windows(2) {
            let ((from, from_reach), (to, to_reach)) = (piece[0], piece[1]);
            let run = to - from;
            let t = ((at - from).dot(run) / run.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
            let reach = from_reach + (to_reach - from_reach) * t;
            if reach > 0.0 {
                let off = (at - from - run * t).length();
                most = most.max(step(reach, reach * (1.0 - Self::SOFT), off));
            }
        }
        most
    }
}

/// Paints a beast's coat into the atlas its body was baked into, the skin
/// under `mane`, if it has one, in the mane's colour.
#[must_use]
pub(crate) fn paint_coat(
    geometry: &AtlasGeometry,
    rig: &Rig,
    coat: &Coat,
    body: &CoatBody,
    mane: Option<&ManeRoots>,
) -> TextureMap {
    paint(geometry, rig, coat, body, mane, None)
}

/// Paints a bird's plumage into the atlas its body was baked into (#423):
/// every feathered texel from `plumage` (`PlumageSampler::colour` and its
/// roughness), and the bare parts - the scaled toes and their talons - from
/// `coat`'s leather and horn as W3 painted them.
#[must_use]
pub(crate) fn paint_plumage(
    geometry: &AtlasGeometry,
    rig: &Rig,
    plumage: &PlumageSampler,
    coat: &Coat,
    body: &CoatBody,
) -> TextureMap {
    paint(geometry, rig, coat, body, None, Some(plumage))
}

/// [`paint_coat`]'s walk over the atlas, every feathered texel from `feathers`
/// where a bird's plumage is given.
fn paint(
    geometry: &AtlasGeometry,
    rig: &Rig,
    coat: &Coat,
    body: &CoatBody,
    mane: Option<&ManeRoots>,
    feathers: Option<&PlumageSampler>,
) -> TextureMap {
    let palette = Palette::of(coat);
    let fields = Fields::new(coat.marks, body.height);
    let count = geometry.texels.len();
    let mut albedo = vec![0u8; count * 4];
    let mut orm = vec![0u8; count * 4];
    let mut heights = vec![0.0f64; count];

    for (index, sample) in geometry.texels.iter().enumerate() {
        let Some(texel) = sample else { continue };
        let painter = Painter::new(texel, coat, &palette, &fields, body);
        let surface = painter.surface();
        let coat_of = |painter: &Painter| match feathers {
            Some(plumage) => (
                plumage.colour(texel.position, texel.normal, texel.zone),
                plumage.roughness(texel.position, texel.normal, texel.zone),
            ),
            None => (painter.fur(), FUR_ROUGHNESS),
        };
        // A winged body's membrane (#423), in its own region of the atlas.
        let membrane = body
            .winged
            .and_then(|winged| winged.membrane)
            .and_then(|chart| {
                let local = chart.local(index, geometry.width, geometry.height)?;
                Some((chart.naked, crate::wing::charted(local, chart.naked), local))
            });
        let (colour, roughness, fur) = match (membrane, surface) {
            (Some((naked, charted, local)), _) => painter.membrane(naked, charted, local),
            (None, Surface::Fur) => {
                let (colour, roughness) = coat_of(&painter);
                (colour, roughness, 1.0)
            }
            (None, Surface::Leather) => (painter.leather(), LEATHER_ROUGHNESS, 0.0),
            (None, Surface::Horn(share)) => {
                let (colour, roughness) = coat_of(&painter);
                (
                    colour.lerp(painter.horn(), share),
                    roughness + (HORN_ROUGHNESS - roughness) * share,
                    1.0 - share,
                )
            }
        };
        let colour = match mane {
            Some(mane) if surface == Surface::Fur => {
                let share = mane.share(texel.position);
                if share > 0.0 {
                    colour.lerp(painter.mane(), share)
                } else {
                    colour
                }
            }
            _ => colour,
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

/// The coat asked at one point of the body rather than over an atlas
/// (#369): what colour the fur growing there is.
pub(crate) struct CoatSampler<'a> {
    coat: &'a Coat,
    body: &'a CoatBody,
    palette: Palette,
    fields: Fields,
}

impl<'a> CoatSampler<'a> {
    /// The coat `coat` on the body `body`, ready to be asked.
    pub(crate) fn new(coat: &'a Coat, body: &'a CoatBody) -> Self {
        Self {
            coat,
            body,
            palette: Palette::of(coat),
            fields: Fields::new(coat.marks, body.height),
        }
    }

    /// The coat's colour at `at`, on a surface facing along `normal` in
    /// `zone`, every locus applied as the painter applies it.
    pub(crate) fn colour(&self, at: Vec3, normal: Vec3, zone: Zone) -> Vec3 {
        self.ask(at, normal, zone, |painter| painter.fur())
    }

    /// The colour of a mane's or a tail's long hair rooted at `at`: the
    /// coat's own there, or its dark on a coat that darkens its points -
    /// a bay's, a dun's - and white where white spotting reaches. A mane's is the
    /// colour picked for it, if one was (#401), and a picked tail's root is what
    /// its skin is there.
    pub(crate) fn points(&self, at: Vec3, normal: Vec3, zone: Zone, tail: bool) -> Vec3 {
        if !tail && let Some(mane) = self.palette.mane {
            mane
        } else if darkens_points(self.coat) && !(tail && self.palette.tail.is_some()) {
            self.ask(at, normal, zone, |painter| painter.points())
        } else {
            self.colour(at, normal, zone)
        }
    }

    /// The one colour a mane is drawn in, forelock and all (#407): see
    /// `Palette::mane`, and [`coat_colour`], which names it to a creator.
    pub(crate) fn mane(&self) -> Vec3 {
        self.palette.mane(darkens_points(self.coat))
    }

    /// The colour picked for the tail, if one was: what a card rooted anywhere on
    /// it runs to, whatever its root has to match on the skin.
    pub(crate) fn tail(&self) -> Option<Vec3> {
        self.palette.tail
    }

    /// The coat's dark: a lynx's ear tips, whatever the coat round them.
    pub(crate) fn dark(&self) -> Vec3 {
        self.palette.dark
    }

    /// What `question` makes of a texel standing at `at`.
    fn ask<R>(
        &self,
        at: Vec3,
        normal: Vec3,
        zone: Zone,
        question: impl FnOnce(&Painter) -> R,
    ) -> R {
        let texel = Texel {
            position: at,
            normal,
            crease: 0.0,
            mouth: 0.0,
            zone,
        };
        question(&Painter::new(
            &texel,
            self.coat,
            &self.palette,
            &self.fields,
            self.body,
        ))
    }
}

/// One texel and everything its colour is read from.
struct Painter<'a> {
    texel: &'a Texel,
    coat: &'a Coat,
    palette: &'a Palette,
    fields: &'a Fields,
    body: &'a CoatBody,
    /// The leg this texel is most a part of, and how much (#376).
    leg: Option<LegShare>,
}

impl<'a> Painter<'a> {
    /// The painter of `texel`, which reads the junctions once for every mark
    /// that asks them.
    fn new(
        texel: &'a Texel,
        coat: &'a Coat,
        palette: &'a Palette,
        fields: &'a Fields,
        body: &'a CoatBody,
    ) -> Self {
        Self {
            texel,
            coat,
            palette,
            fields,
            body,
            leg: body.junctions.leg(texel.position),
        }
    }

    /// How much of a leg this texel is, `0` on the barrel to `1` below the
    /// elbow or the stifle: what every mark that differs between the two
    /// blends by, where the zone it reads would cut it (#376).
    fn limb(&self) -> f32 {
        self.leg.map_or(0.0, |leg| leg.share)
    }

    /// How far a pattern's frame has turned from the barrel's to a leg's,
    /// `0..=1`: none above the junction, all of it [`TURN`] of the
    /// junction's radii down the leg.
    fn turn(&self) -> f32 {
        self.leg
            .map_or(0.0, |leg| leg.share * step(0.0, TURN, leg.depth))
    }

    /// The distance to the nearest cell centre at `frequency` cells per back
    /// height, in cell widths: the field spots, rosettes and dapples are cut
    /// from, [`LEG_FINER`] times as fine down a leg - an animal's spots
    /// shrink toward its feet rather than wrapping a thin leg whole - and
    /// the two blended as the frame turns.
    fn cells(&self, frequency: f32) -> f32 {
        let (at, share) = (self.texel.position, self.turn());
        let barrel = if share < 1.0 {
            self.fields.cells(at, frequency).0
        } else {
            0.0
        };
        let leg = if share > 0.0 {
            self.fields.cells(at, frequency * LEG_FINER).0
        } else {
            0.0
        };
        barrel + (leg - barrel) * share
    }

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
            // A pad's leather, and a bird's or a bat's scaled toes (#422).
            if (foot.kind != Feet::Hooves && local.y < 0.06 * foot.pad) || foot.bare(local) {
                return Surface::Leather;
            }
        }
        Surface::Fur
    }

    /// The coat at this texel, every locus applied in a breeder's order, and a
    /// tail's own colour over that.
    fn fur(&self) -> Vec3 {
        self.tailed(self.coat_fur())
    }

    /// `colour` with a picked tail colour over it, as far as the tail has come in
    /// here (#401).
    fn tailed(&self, colour: Vec3) -> Vec3 {
        match self.palette.tail {
            Some(tail) if self.texel.zone == Zone::Tail => {
                colour.lerp(tail, self.body.tail_share(self.texel.position))
            }
            _ => colour,
        }
    }

    /// The coat at this texel, every locus applied in a breeder's order.
    fn coat_fur(&self) -> Vec3 {
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
        // A winged body's families' marks (#423), over its pattern.
        if let Some(winged) = &self.body.winged {
            colour = self.marked(colour, winged.marks);
        }
        colour = self.countershaded(colour);
        colour = self.broken(colour);
        colour.lerp(self.palette.white, self.white())
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
        self.palette.leather.lerp(self.palette.bare_leather, bare)
    }

    /// A claw's or a hoof's horn: pale under a white sock.
    fn horn(&self) -> Vec3 {
        self.palette.horn.lerp(self.palette.bare_horn, self.white())
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
        let legs = step(0.42, 0.30, self.body.rise(at)) * self.limb();
        let face = self.on_head(|local| {
            let muzzle = step(0.45, 0.75, local.z) * step(0.15, -0.05, local.y);
            let brow = Vec3::new(local.x.abs(), local.y, local.z);
            let spot = step(0.16, 0.10, brow.distance(Vec3::new(0.38, 0.40, 0.62)));
            muzzle.max(spot)
        });
        // The chest's front, which fades out down the top of each foreleg and up
        // the neck before the head rather than stopping where the zone map gives
        // the skin to either (#395).
        let chest = if zone == Zone::Tail {
            0.0
        } else {
            step(0.2, 0.6, underside(normal))
                * step(0.0, 0.1, (at - self.body.withers).z / self.body.height)
                * (1.0 - self.limb())
                * self.off_head()
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
        let legs = step(0.50, 0.38, self.body.rise(at)) * self.limb();
        let tail = if zone == Zone::Tail {
            step(0.2, 0.5, self.body.along_tail(at))
        } else {
            0.0
        };
        let face = self.on_head(|local| {
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
        let (at, normal) = (self.texel.position, self.texel.normal);
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
            // Barred down the legs only, fading in through the junction.
            Pattern::Ticked => step(0.55, 0.85, self.stripes(6.0, TABBY)) * 0.6 * self.limb(),
            // A swirl round each flank's middle, giving way to a tabby's
            // stripes away from it: over the shoulders, down the legs and
            // the tail.
            Pattern::Blotched => {
                let (swirl, flank) = self.swirl();
                let stripes = step(0.25, 0.55, self.stripes(7.0, TABBY));
                stripes + (step(0.2, 0.5, swirl) - stripes) * flank
            }
            Pattern::Spotted => step(0.36, 0.26, self.cells(9.0)),
            // A ring round a centre a shade darker than the ground, wavering
            // and broken into three or four marks, as a leopard's is.
            Pattern::Rosettes => {
                let near = self.cells(6.5) + 0.07 * fields.warp(at, 16.0);
                let ring = step(0.20, 0.29, near) * step(0.47, 0.38, near);
                let broken = step(-0.45, -0.05, fields.patches(at, 26.0));
                (ring * broken).max(0.3 * step(0.27, 0.17, near))
            }
        };
        mark * belly
    }

    /// Bands round the body at `count` per spine length, rings round the
    /// legs and the tail, every one wavering by `waver`: `-1..=1`.
    ///
    /// A band turns into a ring down the top of a leg, as a tiger's stripes
    /// curve round into its legs' (#376): the phase is blended there rather
    /// than chosen by zone, and each leg's rings are counted from the band
    /// that crosses its junction, so the two meet stripe for stripe. Counted
    /// from anywhere else they would differ by a dozen stripes there, and the
    /// blend would crowd those dozen into the turn. The bands that cross a
    /// thick thigh's front and back still have to close into rings, so the
    /// turn runs [`TURN`] radii down the leg: over the crease alone it pressed
    /// them into a whorl of fine stripes at the stifle.
    fn stripes(&self, count: f32, waver: Waver) -> f32 {
        let at = self.texel.position;
        let wavering = waver.share * self.fields.warp(at, waver.frequency);
        (std::f32::consts::TAU * (self.stripe_phase(count) + wavering)).det_sin()
    }

    /// How many stripes along [`Self::stripes`] counts to this texel,
    /// before the wavering.
    fn stripe_phase(&self, count: f32) -> f32 {
        let (at, body) = (self.texel.position, self.body);
        if self.texel.zone == Zone::Tail {
            return body.tail_tip_distance(at) * count * 2.0;
        }
        let (band, turn) = (body.along_spine(at) * count, self.turn());
        match self.leg {
            Some(leg) if turn > 0.0 => {
                let ring = (body.rise(at) - body.rise(leg.junction)) * count * LEG_FINER
                    + body.along_spine(leg.junction) * count;
                band + (ring - band) * turn
            }
            _ => band,
        }
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
        let near = self.cells(6.0) + 0.06 * self.fields.warp(at, 20.0);
        let grey = if self.palette.picked {
            colour
        } else {
            colour.lerp(Vec3::new(0.42, 0.42, 0.44), 0.6)
        };
        grey.lerp(grey.lerp(WHITE, 0.45), step(0.6, 0.34, near))
    }

    /// A colourpoint: the body paled, the face, ears, legs and tail left
    /// their colour.
    fn pointed(&self, colour: Vec3) -> Vec3 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let pale = colour.lerp(self.palette.pale, 0.78);
        let point = if zone == Zone::Tail {
            step(0.0, 0.35, self.body.along_tail(at))
        } else {
            // A mask round the nose that takes the eyes in, and the ears.
            let face = self.on_head(|local| {
                let nose = self
                    .body
                    .head
                    .as_ref()
                    .map_or(Vec3::Z, |head| head.nose_centre() / head.radius);
                step(1.1, 0.6, local.distance(nose)).max(step(0.85, 1.05, local.y))
            });
            face.max(step(0.62, 0.30, self.body.rise(at)) * self.limb())
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
            let diluted = colour.lerp(self.palette.merle, 0.62);
            colour = diluted.lerp(colour, step(0.10, 0.30, patch));
        }
        if coat.tortie && coat.pigment != Pigment::Red {
            colour = colour.lerp(
                self.palette.light,
                step(0.05, 0.25, fields.patches(at * 1.3, 4.0)),
            );
        }
        if coat.dun {
            let diluted = self
                .palette
                .dun
                .unwrap_or_else(|| colour.lerp(self.palette.light.lerp(WHITE, 0.3), 0.5));
            colour = colour.lerp(diluted, 0.8 * self.dun_body());
            colour = colour.lerp(self.palette.dark, 0.8 * self.dun_marks());
        }
        if coat.roan {
            let hairs = step(-0.1, 0.45, fields.grain(at, 70.0));
            colour = colour.lerp(self.palette.white, 0.5 * hairs * self.roan());
        }
        colour
    }

    /// Where a roan's white hairs grow, `0..=1`: through the body and the
    /// neck, thinning down the legs, along the tail and out before the head,
    /// which a roan keeps solid.
    fn roan(&self) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        let body = if trunk(zone) || zone == Zone::Neck || leg(zone) {
            1.0 + (step(0.4, 0.65, self.body.rise(at)) - 1.0) * self.limb()
        } else if zone == Zone::Tail {
            step(0.45, 0.15, self.body.along_tail(at))
        } else {
            0.0
        };
        body * self.off_head()
    }

    /// A coat's dark, and white where white spotting reaches: its long hair's
    /// colour, on a coat that darkens its points.
    fn points(&self) -> Vec3 {
        self.palette.dark.lerp(self.palette.white, self.white())
    }

    /// A mane's skin: the colour picked for it, or on a coat that darkens its points
    /// its dark, whole, as the mane over it is (#407): white spotting under it would
    /// show as a patch the mane does not have.
    fn mane(&self) -> Vec3 {
        self.palette.mane(true)
    }

    /// How much of a dun's dilution reaches here, `0..=1`: all of the body,
    /// thinning out down the legs and toward the tail's tip, which keep
    /// their colour.
    fn dun_body(&self) -> f32 {
        let (at, zone) = (self.texel.position, self.texel.zone);
        if zone == Zone::Tail {
            step(0.6, 0.2, self.body.along_tail(at))
        } else {
            1.0 + (step(0.25, 0.45, self.body.rise(at)) - 1.0) * self.limb()
        }
    }

    /// A dun's dark marks, `0..=1`: a stripe down the middle of the back,
    /// from behind the head on along the tail, and bars across the lower
    /// legs.
    fn dun_marks(&self) -> f32 {
        let (at, normal) = (self.texel.position, self.texel.normal);
        let rise = self.body.rise(at);
        let stripe = step(0.05, 0.03, at.x.abs() / self.body.height)
            * step(0.6, 0.85, topside(normal))
            * self.off_head();
        let bars = step(0.45, 0.35, rise)
            * step(0.3, 0.8, (std::f32::consts::TAU * rise * 9.0).det_sin())
            * self.limb();
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
            step(
                height + 0.03,
                height - 0.03,
                self.body.rise(at) + 0.02 * fields.warp(at, 8.0),
            ) * self.limb()
        };
        match self.coat.white {
            White::None => 0.0,
            White::Socks => socks(0.16),
            White::Irish => socks(0.24)
                .max(self.blaze())
                .max(self.bib(normal, zone))
                .max(step(0.14, 0.10, self.body.tail_tip_distance(at))),
            White::Piebald => {
                let lean = 0.55 * underside(normal) - 0.25 * topside(normal) + 0.3 * self.limb()
                    - 0.4 * (1.0 - self.off_head());
                step(0.05, 0.25, fields.patches(at, 2.2) + lean)
            }
            White::Extreme => {
                // A cap on the head.
                let cap = self.on_head(|local| step(0.15, 0.4, local.y));
                // The tail's root and a patch over the croup it springs from.
                let tail = if zone == Zone::Tail {
                    step(0.5, 0.2, self.body.along_tail(at))
                } else {
                    0.0
                };
                let kept = cap
                    .max(tail)
                    .max(step(0.3, 0.18, self.body.tail_root_distance(at)));
                1.0 - kept.max(step(0.45, 0.6, fields.patches(at, 2.0)) * 0.8)
            }
        }
    }

    /// A white blaze down the middle of the face, from the forehead to the
    /// nose, on the faces that look up or ahead: not under the chin.
    fn blaze(&self) -> f32 {
        let Some(head) = self.body.head.as_ref() else {
            return 0.0;
        };
        let (at, normal) = (self.texel.position, self.texel.normal);
        self.on_head(|local| {
            let facing = head.local(at + normal) - head.local(at);
            let width = 0.16 + 0.04 * self.fields.warp(at, 10.0);
            step(width + 0.04, width, local.x.abs())
                * step(0.30, 0.42, local.z)
                * step(1.0, 0.85, local.y)
                * step(-0.45, -0.15, facing.y)
        })
    }

    /// A white bib: down the middle of the chin, the throat, the chest and
    /// the belly - on the head only what faces the ground, and off it the
    /// chest's front too - and not down the legs.
    fn bib(&self, normal: Vec3, zone: Zone) -> f32 {
        if zone == Zone::Tail {
            return 0.0;
        }
        let width = 0.12 * (1.0 + 0.4 * self.fields.warp(self.texel.position, 4.0));
        let middle = step(
            width + 0.03,
            width,
            self.texel.position.x.abs() / self.body.height,
        );
        // The chest's front counts too, fading out up the neck before the head
        // rather than stopping where the zone map gives the skin to it (#395).
        let facing = underside(normal).max(normal.z * 0.8 * self.off_head());
        middle * step(0.3, 0.6, facing) * (1.0 - self.limb())
    }

    /// The fur's grain at this texel: noise stretched along the bone it
    /// grows on, which the normal map turns into strands catching the light.
    ///
    /// Through the junction the barrel's grain and the leg's are blended
    /// rather than switched where a leg's bone becomes the nearer (#376):
    /// switched, the strands turned on a line round the top of the leg. The
    /// blend is scaled to keep the grain's strength, which two independent
    /// halves would otherwise lose in the band's middle.
    fn grain(&self, rig: &Rig) -> f32 {
        let share = self.limb();
        let leg_axis = self
            .leg
            .filter(|_| share > 0.0)
            .and_then(|leg| self.body.junctions.leg_axis(leg.limb, self.texel.position));
        let Some(leg_axis) = leg_axis else {
            return self.trunk_grain(rig);
        };
        if share >= 1.0 {
            return self.grain_along(leg_axis);
        }
        let (barrel, leg) = (self.trunk_grain(rig), self.grain_along(leg_axis));
        (barrel + (leg - barrel) * share) / ((1.0 - share).powi(2) + share.powi(2)).sqrt()
    }

    /// The grain of the bones nearest this texel that are not a leg's, blended by
    /// how much further each is than the nearest (#395): switched where one becomes
    /// the nearer, the strands turned on a straight line round the neck and the
    /// back of the head. As in [`Self::grain`], scaled to keep its strength.
    fn trunk_grain(&self, rig: &Rig) -> f32 {
        let at = self.texel.position;
        let near = rig.nearest_bones_where::<GRAIN_BONES>(at, |joint| !leg(rig.joints[joint].zone));
        let band = GRAIN_BAND * self.body.height;
        let last = near[GRAIN_BONES - 1].1;
        let (mut sum, mut squares) = (0.0, 0.0);
        for (joint, distance) in near {
            // The last bone read has no share, so the bone it hands over to, when
            // two swap places, comes in from nothing.
            let share = step(band, 0.0, distance - near[0].1)
                * step(0.0, GRAIN_HANDOVER * band, last - distance);
            if share > 0.0 {
                let (from, to) = rig.bone(joint);
                sum += share * self.grain_along((to - from).normalize_or(Vec3::Y));
                squares += share * share;
            }
        }
        sum / squares.sqrt().max(f32::EPSILON)
    }

    /// The grain's noise stretched along `along`.
    fn grain_along(&self, along: Vec3) -> f32 {
        let at = self.texel.position;
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

    /// What `mark` makes of this texel in the declared head's frame, in head radii:
    /// all of it on the head, and fading out down the neck rather than stopping
    /// where the zone map gives the skin to it (#395). Nothing on a body with no
    /// head declared.
    fn on_head(&self, mark: impl FnOnce(Vec3) -> f32) -> f32 {
        let Some(head) = self.body.head.as_ref() else {
            return 0.0;
        };
        let share = if self.texel.zone == Zone::Head {
            1.0
        } else {
            1.0 - self.off_head()
        };
        if share > 0.0 {
            mark(head.local(self.texel.position) / head.radius) * share
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{CoatChannel, Family, Fantasy, Nose, QuadrupedParams};
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

    /// One picked colour per channel, none a coat would derive.
    fn marker(channel: CoatChannel) -> [f32; 3] {
        match channel {
            CoatChannel::Dark => [0.10, 0.20, 0.90],
            CoatChannel::Light => [0.95, 0.85, 0.10],
            CoatChannel::White => [0.95, 0.30, 0.80],
            CoatChannel::Leather => [0.10, 0.90, 0.20],
            CoatChannel::Horn => [0.90, 0.10, 0.10],
            CoatChannel::Mane => [0.15, 0.95, 0.95],
            CoatChannel::Tail => [0.60, 0.10, 0.95],
        }
    }

    /// A dark coat that uses every channel: tanned, spotted white, roan, on a beast
    /// with claws and a leather nose, its mane and tail long.
    fn every_channel_shows(params: &mut QuadrupedParams) {
        params.coat.pigment = Pigment::Black;
        params.coat.agouti = Agouti::TanPoints;
        params.coat.white = White::Irish;
        params.coat.roan = true;
        params.feet = Feet::Claws;
        params.nose = Nose::Leather;
        params.fur.mane = 0.8;
        params.fur.tail = 0.8;
        params.fur.ruff = 0.0;
    }

    #[test]
    fn a_picked_colour_paints_its_own_channel_and_no_other() {
        // Read of the painter at every vertex of the body: which surface a point
        // is, and which colour changed there, against the same beast with nothing
        // picked.
        let (avatar, _) = painted(Family::Dog, every_channel_shows);
        let Archetype::Quadruped(base) = ({
            let mut params = QuadrupedParams::default();
            params.set_kind(Family::Dog, None, 0.0);
            every_channel_shows(&mut params);
            Archetype::Quadruped(params)
        }) else {
            unreachable!()
        };
        let skeleton = Archetype::Quadruped(base).skeleton(&crate::plan::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let anatomy = CoatBody::of(
            &rig,
            skeleton.beast_head.as_ref(),
            skeleton.beast_feet.as_ref(),
        );
        let (mesh, zones) = (&avatar.parts.body, &avatar.parts.zones);
        let normals = mesh.vertex_normals();
        let plain = Palette::of(&base.coat);
        let fields = Fields::new(base.coat.marks, anatomy.height);
        for channel in CoatChannel::ALL {
            let mut coat = base.coat;
            *coat.colours.get_mut(channel) = Some(marker(channel));
            let palette = Palette::of(&coat);
            let chosen = Vec3::from_array(marker(channel));
            let (mut fur, mut leather, mut horn) = (0, 0, 0);
            for vertex in 0..mesh.vertex_count() {
                let texel = Texel {
                    position: mesh.positions[vertex],
                    normal: normals[vertex],
                    crease: 0.0,
                    mouth: 0.0,
                    zone: zones[vertex],
                };
                let before = Painter::new(&texel, &base.coat, &plain, &fields, &anatomy);
                let after = Painter::new(&texel, &coat, &palette, &fields, &anatomy);
                match before.surface() {
                    Surface::Fur => {
                        if before.fur() != after.fur() {
                            fur += 1;
                            assert!(
                                matches!(
                                    channel,
                                    CoatChannel::Dark
                                        | CoatChannel::Light
                                        | CoatChannel::White
                                        | CoatChannel::Tail
                                ),
                                "{channel:?} moved the coat at {texel:?}"
                            );
                            if channel == CoatChannel::Tail {
                                assert_eq!(texel.zone, Zone::Tail, "a tail's colour left the tail");
                            }
                            if channel == CoatChannel::White {
                                assert!(
                                    before.white() > 0.0 || before.roan() > 0.0,
                                    "white reached a texel with no white on it"
                                );
                            }
                        }
                    }
                    Surface::Leather => {
                        if before.leather() != after.leather() {
                            leather += 1;
                            assert_eq!(channel, CoatChannel::Leather);
                            assert_eq!(after.leather(), chosen, "under white as well");
                        }
                    }
                    Surface::Horn(_) => {
                        if before.horn() != after.horn() {
                            horn += 1;
                            assert_eq!(channel, CoatChannel::Horn);
                            assert_eq!(after.horn(), chosen, "under white as well");
                        }
                    }
                }
            }
            // The other half: each reached something.
            match channel {
                CoatChannel::Dark | CoatChannel::Light | CoatChannel::White | CoatChannel::Tail => {
                    assert!(fur > 20, "{channel:?} reached {fur} vertices");
                }
                CoatChannel::Leather => assert!(leather > 20, "{leather} leather vertices"),
                CoatChannel::Horn => assert!(horn > 20, "{horn} horn vertices"),
                // Its fur and its roots: below.
                CoatChannel::Mane => assert_eq!(fur + leather + horn, 0),
            }
        }
    }

    #[test]
    fn every_picked_colour_reaches_the_atlas_and_a_coat_that_picks_none_does_not_move() {
        let (plain, _) = painted(Family::Dog, every_channel_shows);
        for channel in CoatChannel::ALL {
            let (avatar, _) = painted(Family::Dog, |params| {
                every_channel_shows(params);
                *params.coat.colours.get_mut(channel) = Some(marker(channel));
            });
            let chosen = Vec3::from_array(marker(channel));
            assert_eq!(texels_of(&plain, chosen), 0, "{channel:?}: the premise");
            assert!(
                texels_of(&avatar, chosen) > 20,
                "{channel:?} painted {} texels",
                texels_of(&avatar, chosen)
            );
        }
    }

    #[test]
    fn a_picked_colour_wins_over_the_loci_and_a_derived_one_follows_it() {
        let picked = [0.3, 0.6, 0.9];
        let chosen = Vec3::from_array(picked);
        for fantasy in [Fantasy::None, Fantasy::Rose] {
            for dilute in [false, true] {
                for intensity in [-3.0, 0.0, 3.0] {
                    for pigment in Pigment::ALL {
                        let base = Coat {
                            fantasy,
                            dilute,
                            intensity,
                            pigment,
                            ..Coat::default()
                        };
                        let with = |channel: CoatChannel| {
                            let mut coat = base;
                            *coat.colours.get_mut(channel) = Some(picked);
                            Palette::of(&coat)
                        };
                        assert_eq!(with(CoatChannel::Dark).dark, chosen);
                        assert_eq!(with(CoatChannel::Light).light, chosen);
                        assert_eq!(with(CoatChannel::White).white, chosen);
                        assert_eq!(with(CoatChannel::Leather).leather, chosen);
                        assert_eq!(with(CoatChannel::Leather).bare_leather, chosen);
                        assert_eq!(with(CoatChannel::Horn).horn, chosen);
                        assert_eq!(with(CoatChannel::Horn).bare_horn, chosen);
                        // Nothing else moves on the channel's own account.
                        let plain = Palette::of(&base);
                        let horned = with(CoatChannel::Horn);
                        assert_eq!((horned.dark, horned.light), (plain.dark, plain.light));
                        assert_eq!(horned.leather, plain.leather);
                    }
                }
            }
        }
        // A red coat's dark stays its light deepened, whichever light that is,
        // until it is picked itself.
        let mut coat = Coat {
            pigment: Pigment::Red,
            ..Coat::default()
        };
        coat.colours.light = Some(picked);
        let palette = Palette::of(&coat);
        assert_eq!(palette.ground, chosen);
        assert_eq!(palette.dark, chosen * 0.62);
        coat.colours.dark = Some([0.1, 0.1, 0.1]);
        assert_eq!(Palette::of(&coat).dark, Vec3::splat(0.1));
        assert_eq!(Palette::of(&coat).ground, chosen);
        // And a dark coat's ground is its dark.
        let mut coat = Coat {
            pigment: Pigment::Black,
            ..Coat::default()
        };
        coat.colours.dark = Some(picked);
        assert_eq!(Palette::of(&coat).ground, chosen);
    }

    #[test]
    fn what_a_picked_white_and_a_picked_light_reach_is_as_settled() {
        let (avatar, _) = painted(Family::Horse, |_| ());
        let mut params = QuadrupedParams::default();
        params.set_kind(Family::Horse, None, 0.0);
        let skeleton = Archetype::Quadruped(params).skeleton(&crate::plan::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let anatomy = CoatBody::of(
            &rig,
            skeleton.beast_head.as_ref(),
            skeleton.beast_feet.as_ref(),
        );
        let (mesh, zones) = (&avatar.parts.body, &avatar.parts.zones);
        let normals = mesh.vertex_normals();
        // Over the barrel's flank, level, so that no belly and no leg reads.
        let flank = (0..mesh.vertex_count())
            .find(|&vertex| {
                zones[vertex] == Zone::Pelvis
                    && normals[vertex].x > 0.9
                    && anatomy.junctions.share(mesh.positions[vertex]) == 0.0
            })
            .expect("a flank");
        let texel = Texel {
            position: mesh.positions[flank],
            normal: normals[flank],
            crease: 0.0,
            mouth: 0.0,
            zone: Zone::Pelvis,
        };
        let fur = |coat: Coat| {
            let palette = Palette::of(&coat);
            let fields = Fields::new(coat.marks, anatomy.height);
            Painter::new(&texel, &coat, &palette, &fields, &anatomy).fur()
        };
        let picked = [0.2, 0.9, 0.3];
        let chosen = Vec3::from_array(picked);
        let with = |base: Coat, channel: CoatChannel| {
            let mut coat = base;
            *coat.colours.get_mut(channel) = Some(picked);
            fur(coat)
        };
        let black = Coat {
            pigment: Pigment::Black,
            ..Coat::default()
        };
        // A colourpoint's paled body is the light's and not the white's.
        let point = Coat {
            points: true,
            ..black
        };
        assert_eq!(with(point, CoatChannel::White), fur(point));
        assert!(with(point, CoatChannel::Light).distance(chosen) < 0.3);
        assert!(fur(point).distance(chosen) > 0.5, "the premise");
        // A roan's hairs are the white's, on some grain or other.
        let roan = Coat {
            roan: true,
            ..black
        };
        let roan_at = (0..600)
            .map(|marks| Coat { marks, ..roan })
            .find(|coat| {
                fur(*coat).distance(fur(Coat {
                    roan: false,
                    ..*coat
                })) > 0.05
            })
            .expect("a hair");
        // Which shows as how much greener than it is red, the colour being green.
        let lean = |colour: Vec3| colour.y - colour.x;
        let hairs = fur(Coat {
            colours: crate::plan::CoatColours {
                white: Some(picked),
                ..crate::plan::CoatColours::default()
            },
            ..roan_at
        });
        assert!(lean(hairs) > lean(fur(roan_at)) + 0.02);
        // A dun's body is the light's, nearly whole.
        let dun = Coat { dun: true, ..black };
        assert!(with(dun, CoatChannel::Light).distance(chosen) < 0.3);
        // A dark merle's ground leans to the light; a red merle's cannot.
        let merle = Coat {
            merle: true,
            ..black
        };
        let merled = (0..600)
            .map(|marks| Coat { marks, ..merle })
            .find(|coat| {
                fur(*coat).distance(fur(Coat {
                    merle: false,
                    ..*coat
                })) > 0.05
            })
            .expect("a patch");
        assert!(with(merled, CoatChannel::Light).distance(chosen) < fur(merled).distance(chosen));
        let mut red = Coat {
            merle: true,
            pigment: Pigment::Red,
            ..Coat::default()
        };
        red.colours.light = Some(picked);
        assert_eq!(Palette::of(&red).merle, Vec3::new(0.56, 0.57, 0.60));
        // A dapple is not greyed under a picked colour: its net is the colour.
        let dapple = Coat {
            pattern: Pattern::Dapple,
            ..black
        };
        assert!(with(dapple, CoatChannel::Dark).distance(chosen) < fur(dapple).distance(chosen));
        let grey_in = |colour: Vec3| colour.max_element() - colour.min_element();
        assert!(grey_in(with(dapple, CoatChannel::Dark)) > grey_in(fur(dapple)) + 0.2);
    }

    #[test]
    fn every_channel_offers_its_presets_and_reads_what_it_paints() {
        for channel in CoatChannel::ALL {
            let swatches = coat_swatches(channel);
            assert!(swatches.len() >= 3, "{channel:?}");
            let coat = Coat::default();
            let mut picked = coat;
            *picked.colours.get_mut(channel) = Some(marker(channel));
            assert_eq!(coat_colour(&picked, channel), marker(channel));
            assert_ne!(coat_colour(&coat, channel), marker(channel));
        }
        // A black coat's dark is one of the presets, and so is every fantasy's.
        let black = Coat {
            pigment: Pigment::Black,
            ..Coat::default()
        };
        let darks = coat_swatches(CoatChannel::Dark);
        assert!(
            darks
                .iter()
                .any(|(_, tone)| *tone == coat_colour(&black, CoatChannel::Dark))
        );
        for fantasy in FANTASIES_FOR_TEST {
            let coat = Coat {
                fantasy,
                pigment: Pigment::Black,
                ..Coat::default()
            };
            assert!(
                darks
                    .iter()
                    .any(|(_, tone)| *tone == coat_colour(&coat, CoatChannel::Dark))
            );
        }
    }

    const FANTASIES_FOR_TEST: [Fantasy; 6] = [
        Fantasy::Sky,
        Fantasy::Mint,
        Fantasy::Lilac,
        Fantasy::Rose,
        Fantasy::Sunny,
        Fantasy::Ocean,
    ];

    /// What a mane's or a tail's cards were coloured: root and tip of each
    /// vertex of `zone`'s faces.
    fn fur_colours(avatar: &Avatar, zone: crate::hair::fur::FurZone) -> Vec<Vec3> {
        let Some(fur) = avatar.parts.fur.as_ref() else {
            return Vec::new();
        };
        let mesh = &fur.mesh;
        let mut face = 0;
        let mut out = Vec::new();
        for grown in &fur.grown {
            let faces = face..face + grown.tris / 2;
            face = faces.end;
            if grown.zone == zone {
                for corners in &mesh.faces[faces] {
                    out.extend(corners.iter().map(|&corner| mesh.colours[corner as usize]));
                }
            }
        }
        out
    }

    /// The colour at the tip of each card of `zone`.
    fn fur_tips(fur: &crate::hair::fur::FurGrowth, zone: crate::hair::fur::FurZone) -> Vec<Vec3> {
        let mesh = &fur.mesh;
        let mut face = 0;
        let mut out = Vec::new();
        for grown in &fur.grown {
            let faces = face..face + grown.tris / 2;
            face = faces.end;
            if grown.zone == zone {
                for corners in &mesh.faces[faces] {
                    out.extend(
                        corners
                            .iter()
                            .filter(|&&corner| mesh.uvs[corner as usize].y >= 0.999)
                            .map(|&corner| mesh.colours[corner as usize]),
                    );
                }
            }
        }
        assert!(!out.is_empty(), "fixture: no tips");
        out
    }

    #[test]
    fn a_mane_in_a_picked_colour_is_that_colour_and_so_is_the_skin_under_it() {
        use crate::hair::fur::FurZone;
        // A light mane on a dark coat, which the roots never had to paint before:
        // the cards in it, from root to a shade lighter at the tip, and the skin
        // along the crest in it too, or a line of coat shows between two cards.
        let light = [0.92, 0.90, 0.78];
        let chosen = Vec3::from_array(light);
        for family in Family::ALL {
            let beast = |pick: bool, mane: f32| {
                painted(family, |params| {
                    params.coat.pigment = Pigment::Black;
                    params.coat.agouti = Agouti::Solid;
                    params.fur.mane = mane;
                    if pick {
                        params.coat.colours.mane = Some(light);
                    }
                })
                .0
            };
            let (bare, maned) = (beast(true, 0.0), beast(true, 0.8));
            let before = texels_of(&bare, chosen);
            let after = texels_of(&maned, chosen);
            assert_eq!(before, 0, "{family:?}: the premise");
            // Measured, per family: 2,700 to 3,700, a line down the neck.
            assert!(
                (1500..6500).contains(&after),
                "{family:?}: {after} texels under its mane"
            );
            // The mane and the forelock are one solid (#407): every vertex of it on
            // the way from the colour to a shade lighter, its underside no more than
            // a little darker.
            let tips = chosen.lerp(Vec3::ONE, 0.08);
            let colours = fur_colours(&maned, FurZone::Mane);
            assert!(colours.len() > 40, "{family:?}: {}", colours.len());
            for colour in colours {
                let on_the_way = (0..=50).any(|step| {
                    let along = chosen.lerp(tips, step as f32 / 50.0);
                    let shade = colour.dot(along) / along.length_squared();
                    (0.85..=1.0001).contains(&shade) && colour.distance(along * shade) < 2.0e-3
                });
                assert!(on_the_way, "{family:?} wore {colour:?}");
            }
            // The rest of the coat is what it was: only the line moved.
            let unpicked = beast(false, 0.8);
            let moved = unpicked
                .skin
                .albedo
                .chunks_exact(4)
                .zip(maned.skin.albedo.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            assert!(
                moved < after * 2,
                "{family:?}: {moved} texels moved for {after} on the line"
            );
        }
    }

    #[test]
    fn a_picked_mane_with_no_mane_to_wear_it_changes_nothing() {
        for family in Family::ALL {
            let beast = |pick: bool| {
                painted(family, |params| {
                    params.fur.tail = 0.5;
                    if pick {
                        params.coat.colours.mane = Some([0.9, 0.1, 0.5]);
                    }
                })
                .0
            };
            let (plain, picked) = (beast(false), beast(true));
            assert!(plain.skin.albedo == picked.skin.albedo, "{family:?}");
            assert_eq!(plain.parts.fur, picked.parts.fur, "{family:?}");
        }
    }

    #[test]
    fn a_tail_in_a_picked_colour_meets_the_rump_with_no_line() {
        // Rolled beasts, crosses among them, and tails at the envelope's ends: at
        // every face that crosses from the tail onto the rest of the body, the
        // tail's side has yet to take any of the colour, so nothing shows where
        // the zone map draws its line. Asked of the vertices, as a texel's zone is
        // its nearest's.
        let (mut highest, mut furthest) = (0.0f32, 0.0f32);
        for seed in 0..60 {
            for tail_length in [None, Some(-3.0), Some(3.0)] {
                let mut record =
                    AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
                record.reroll(seed);
                let Archetype::Quadruped(params) = &mut record.archetype else {
                    panic!("a beast");
                };
                if let Some(tail_length) = tail_length {
                    params.tail_length = tail_length;
                }
                record.sanitize();
                let config = crate::AvatarConfig {
                    atlas: 64,
                    ..crate::AvatarConfig::default()
                };
                let avatar = Avatar::build_with(&record, &config).expect("builds");
                let skeleton = record.skeleton();
                let body = CoatBody::of(&avatar.rig, skeleton.beast_head.as_ref(), None);
                let (mesh, zones) = (&avatar.parts.body, &avatar.parts.zones);
                for face in &mesh.faces {
                    let on_tail = |corner: &&u32| zones[**corner as usize] == Zone::Tail;
                    if face.iter().any(|corner| on_tail(&corner))
                        && !face.iter().all(|corner| on_tail(&corner))
                    {
                        for corner in face.iter().filter(on_tail) {
                            let at = mesh.positions[*corner as usize];
                            highest = highest.max(body.along_dock(at));
                            furthest = furthest.max(body.along_tail(at));
                            let share = body.tail_share(at);
                            assert!(share <= 0.0, "seed {seed}: {share} at the rump's line");
                        }
                    }
                }
            }
        }
        // More than 3 % clear of what was read.
        assert!(
            highest * 1.03 < DOCK_FADE.0 && furthest * 1.03 < TAIL_FADE.0,
            "the line lies at {highest} and {furthest}"
        );
    }

    #[test]
    fn a_picked_white_reaches_the_long_hair_of_a_coat_that_darkens_its_points() {
        // `points` still went to the constant white: on a bay's or a dun's white
        // patches the mane and the tail, and the skin under the mane, stayed white
        // on a beast whose every other white was the colour picked (#401).
        let picked = [0.90, 0.10, 0.80];
        let chosen = Vec3::from_array(picked);
        let (rig, body) = rigged(Family::Horse);
        let along: Vec<Vec3> = rig
            .joints
            .iter()
            .filter(|joint| !joint.marker && trunk(joint.zone))
            .map(|joint| joint.position + Vec3::Y * 0.1 * body.height)
            .collect();
        for (agouti, dun) in [(Agouti::Bay, false), (Agouti::Solid, true)] {
            let coat = Coat {
                pigment: Pigment::Black,
                agouti,
                dun,
                white: White::Extreme,
                colours: crate::plan::CoatColours {
                    white: Some(picked),
                    ..crate::plan::CoatColours::default()
                },
                ..Coat::default()
            };
            assert!(darkens_points(&coat), "the premise: {agouti:?}");
            let dark = Palette::of(&coat).dark;
            let mut most: f32 = 0.0;
            for &at in &along {
                let (points, white) = paint(&body, &coat, (at, Vec3::Y, Zone::Chest), |painter| {
                    (painter.points(), painter.white())
                });
                most = most.max(white);
                assert!(
                    points.distance(dark.lerp(chosen, white)) < 1.0e-4,
                    "{agouti:?} at {at}: {points} with {white} of the white"
                );
            }
            assert!(most > 0.5, "{agouti:?}: the premise, white reaching {most}");
        }
    }

    #[test]
    fn a_tail_s_skin_and_its_cards_agree_at_every_root_and_it_runs_to_the_colour() {
        use crate::hair::fur::FurZone;
        let picked = [0.60, 0.10, 0.95];
        let chosen = Vec3::from_array(picked);
        for family in Family::ALL {
            let tweak = |agouti: Agouti| {
                move |params: &mut QuadrupedParams| {
                    params.fur.tail = 0.8;
                    params.coat.pigment = Pigment::Black;
                    params.coat.agouti = agouti;
                    params.coat.colours.tail = Some(picked);
                }
            };
            // On a coat that draws its long hair in its own colour and one that
            // draws it in its dark.
            for agouti in [Agouti::Solid, Agouti::Bay] {
                let (avatar, _) = painted(family, tweak(agouti));
                let colours = fur_colours(&avatar, FurZone::Tail);
                assert!(colours.len() > 40, "{family:?}: {} corners", colours.len());
                let tips = chosen.lerp(Vec3::ONE, 0.08);
                // Cards rooted past the fade are the colour from root to tip. Only a
                // share of them are: see [`DOCK_FADE`] for the rest.
                let span = tips - chosen;
                let at_the_colour = colours
                    .iter()
                    .filter(|colour| {
                        let along = (**colour - chosen).dot(span) / span.length_squared();
                        colour.distance(chosen + span * along.clamp(0.0, 1.0)) < 0.01
                    })
                    .count();
                assert!(
                    at_the_colour * 3 > colours.len(),
                    "{family:?} {agouti:?}: {at_the_colour} of {} at the colour",
                    colours.len()
                );
                // And every card, however its root started, ends there.
                let fur = avatar.parts.fur.as_ref().expect("a tail's fur");
                for colour in fur_tips(fur, FurZone::Tail) {
                    assert!(
                        colour.distance(tips) < 0.01,
                        "{family:?} {agouti:?}: a tip of {colour:?}"
                    );
                }
                // The whole skin past the fade is the colour.
                assert!(texels_of(&avatar, chosen) > 200, "{family:?} {agouti:?}");
            }
            // A card rooted before the colour has come in starts as the coat does.
            let (_, body) = rigged(family);
            for agouti in [Agouti::Solid, Agouti::Bay] {
                let coat = Coat {
                    pigment: Pigment::Black,
                    agouti,
                    colours: crate::plan::CoatColours {
                        tail: Some(picked),
                        ..crate::plan::CoatColours::default()
                    },
                    ..Coat::default()
                };
                let bare = Coat {
                    colours: crate::plan::CoatColours::default(),
                    ..coat
                };
                let (root, tip) = body.tail.expect("a tail");
                let sampler = CoatSampler::new(&coat, &body);
                let plain = CoatSampler::new(&bare, &body);
                let mut previous = None;
                for step_along in 0..=60 {
                    let at = root.lerp(tip, step_along as f32 / 60.0);
                    let card = sampler.points(at, Vec3::Y, Zone::Tail, true);
                    let skin = sampler.colour(at, Vec3::Y, Zone::Tail);
                    assert_eq!(card, skin, "{family:?} at {step_along}: card against skin");
                    let share = body.tail_share(at);
                    if share <= 0.0 {
                        assert_eq!(card, plain.colour(at, Vec3::Y, Zone::Tail));
                    }
                    if share >= 1.0 {
                        assert_eq!(card, chosen);
                    }
                    // No step between two points a sixtieth of the tail apart: under
                    // half the whole gap in one.
                    if let Some(previous) = previous {
                        let gap = chosen.distance(plain.colour(at, Vec3::Y, Zone::Tail));
                        let moved = card.distance(previous);
                        assert!(
                            moved < gap.max(0.3) * 0.5,
                            "{family:?} at {step_along}: {moved} of {gap}"
                        );
                    }
                    previous = Some(card);
                }
            }
        }
    }

    #[test]
    fn a_coat_that_picks_nothing_paints_what_it_always_painted() {
        // The palette's new fields say what the old constants said.
        for fantasy in FANTASIES_FOR_TEST.into_iter().chain([Fantasy::None]) {
            for pigment in Pigment::ALL {
                let palette = Palette::of(&Coat {
                    fantasy,
                    pigment,
                    ..Coat::default()
                });
                assert_eq!(palette.white, WHITE);
                assert_eq!(palette.pale, WHITE.lerp(palette.light, 0.15));
                assert_eq!(palette.merle, Vec3::new(0.56, 0.57, 0.60));
                assert_eq!(palette.dun, None);
                assert!(!palette.picked);
                assert_eq!((palette.mane, palette.tail), (None, None));
            }
        }
    }

    #[test]
    fn the_skin_under_a_dark_mane_takes_its_colour_and_no_other_coat_changes() {
        let with_mane = |dun: bool, agouti: Agouti, mane: f32| {
            painted(Family::Horse, |params| {
                params.coat.dun = dun;
                params.coat.agouti = agouti;
                params.fur.mane = mane;
            })
        };
        for (dun, agouti) in [(false, Agouti::Bay), (true, Agouti::Solid)] {
            let (bare, palette) = with_mane(dun, agouti, 0.0);
            let (maned, _) = with_mane(dun, agouti, 0.8);
            // The mane's colour: the coat's dark, shaded down to its outline where
            // that is no darker than it (#407), a red bay's.
            let before = texels_of(&bare, palette.mane(true));
            let after = texels_of(&maned, palette.mane(true));
            // Measured: about 3,650, a line down the neck, and no further than that.
            assert!(
                (2000..6000).contains(&(after - before)),
                "dun {dun} {agouti:?}: {before} dark texels to {after} under its mane"
            );
        }
        // A coat that draws its mane in its own colour has nothing to hide.
        let (bare, _) = with_mane(false, Agouti::Solid, 0.0);
        let (maned, _) = with_mane(false, Agouti::Solid, 0.8);
        assert!(bare.skin.albedo == maned.skin.albedo);
    }

    #[test]
    fn a_mane_s_roots_reach_as_far_as_their_line_says_and_fade_there() {
        let reach = 0.1;
        let roots = ManeRoots::new(vec![
            (Vec3::ZERO, reach),
            (Vec3::Y, reach),
            (Vec3::new(0.0, 2.0, 0.0), 0.0),
        ]);
        let up = Vec3::new(0.0, 0.5, 0.0);
        assert_eq!(roots.share(up), 1.0, "on the line");
        assert_eq!(
            roots.share(up + Vec3::X * 0.05),
            1.0,
            "inside its soft edge"
        );
        let edge = roots.share(up + Vec3::X * 0.08);
        assert!(edge > 0.0 && edge < 1.0, "in its soft edge: {edge}");
        assert_eq!(roots.share(up + Vec3::X * reach), 0.0, "at its reach");
        assert_eq!(roots.share(Vec3::splat(9.0)), 0.0, "out of its bounds");
        // Narrowing to nothing at the end of the line, which has no width to read.
        assert_eq!(roots.share(Vec3::new(0.05, 2.0, 0.0)), 0.0);
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
        rigged(family).1
    }

    /// A beast of `family`'s rig, and its body as the painter reads it.
    fn rigged(family: Family) -> (Rig, CoatBody) {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        let skeleton = Archetype::Quadruped(params).skeleton(&crate::plan::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("the beast rigs");
        let body = CoatBody::of(
            &rig,
            skeleton.beast_head.as_ref(),
            skeleton.beast_feet.as_ref(),
        );
        (rig, body)
    }

    /// Every vertex of a built beast of `family` with its normal, and the
    /// body the painter reads.
    fn surface(family: Family) -> (Vec<(Vec3, Vec3)>, Rig, CoatBody) {
        let (avatar, _) = painted(family, |_| ());
        let body = &avatar.parts.body;
        let points = body
            .positions
            .iter()
            .copied()
            .zip(body.vertex_normals())
            .collect();
        let (rig, coat_body) = rigged(family);
        (points, rig, coat_body)
    }

    /// The coats the junction is judged on: every pattern, every white,
    /// a colourpoint, the agouti states that mark the legs, and the loci
    /// that thin down them.
    fn junction_coats() -> Vec<Coat> {
        let mut coats = Vec::new();
        for (index, &pattern) in Pattern::ALL.iter().enumerate() {
            coats.push(Coat {
                pattern,
                points: index % 2 == 1,
                white: White::ALL[index % White::ALL.len()],
                marks: 1 + index as u16 * 977,
                ..Coat::default()
            });
        }
        for agouti in [Agouti::TanPoints, Agouti::Bay] {
            coats.push(Coat {
                agouti,
                roan: agouti == Agouti::Bay,
                dun: agouti == Agouti::TanPoints,
                ..Coat::default()
            });
        }
        coats
    }

    #[test]
    fn a_leg_s_coat_does_not_ask_the_zone() {
        // The zone map's line round the top of a leg is where the skin
        // weights cross, which is nowhere a coat changes: asked there, a
        // stripe turned from a band to a ring on it and a point, a roan's
        // thinning, a bib, a piebald's lean were cut along it (#376); and on a
        // cat the chest's zone runs down the forearm, where a sock or a
        // bay's black asked of it stopped short. Through the band where a leg
        // leaves the trunk and all the way down the leg, a point is the same
        // colour whichever zone the map gave it.
        for family in Family::ALL {
            let (points, _, body) = surface(family);
            let band: Vec<_> = points
                .iter()
                .filter_map(|&(at, normal)| {
                    let leg = body.junctions.leg(at)?;
                    Some((at, normal, leg.limb))
                })
                .collect();
            assert!(
                band.len() > 20,
                "{family:?}: only {} points on a leg",
                band.len()
            );
            for coat in junction_coats() {
                for &(at, normal, limb) in &band {
                    let colour = |zone| paint(&body, &coat, (at, normal, zone), |p| p.fur());
                    let upper = colour(Zone::UpperLimb(limb));
                    for zone in [
                        Zone::Chest,
                        Zone::Abdomen,
                        Zone::Pelvis,
                        Zone::LowerLimb(limb),
                    ] {
                        assert_eq!(
                            colour(zone),
                            upper,
                            "{family:?} {:?}: {zone:?} and the upper leg differ at {at}",
                            coat.pattern
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_band_closes_into_rings_without_crowding() {
        // Turned over the crease alone, the bands crossing a thigh's front
        // and back closed into rings in a whorl of fine stripes at the
        // stifle; and counted from the ground rather than from the band at
        // the junction, the rings met the bands a dozen stripes out, which
        // the turn then crowded together (#376). Down the whole turn the
        // stripes along the skin lie under twice as close as the leg's own
        // rings: 1.4 to 1.7 at the slice, 4.1 and 2.6 with either undone.
        const COUNT: f32 = 17.0;
        for family in Family::ALL {
            let (points, _, body) = surface(family);
            let coat = Coat::default();
            let phase = |at: Vec3| {
                paint(
                    &body,
                    &coat,
                    (at, Vec3::Y, Zone::UpperLimb(crate::plan::Limb::ForeLeft)),
                    |p| p.stripe_phase(COUNT),
                )
            };
            let ring = COUNT * LEG_FINER / body.height;
            let mut worst: f32 = 0.0;
            for &(at, normal) in &points {
                let turn = paint(&body, &coat, (at, normal, Zone::Chest), |p| p.turn());
                if turn <= 0.0 || turn >= 1.0 {
                    continue;
                }
                // Along the skin: the gradient less its part through it.
                let step = 1.0e-3 * body.height;
                let gradient = Vec3::new(
                    phase(at + Vec3::X * step) - phase(at - Vec3::X * step),
                    phase(at + Vec3::Y * step) - phase(at - Vec3::Y * step),
                    phase(at + Vec3::Z * step) - phase(at - Vec3::Z * step),
                ) / (2.0 * step);
                let along = gradient - normal * gradient.dot(normal);
                worst = worst.max(along.length() / ring);
            }
            assert!(worst > 0.0, "{family:?}: no turn was measured");
            assert!(
                worst < 2.0,
                "{family:?}: stripes crowd to {worst:.2}x the rings' density in the turn"
            );
        }
    }

    #[test]
    fn a_spot_down_a_leg_is_cut_finer_than_one_on_the_barrel() {
        // A barrel's cell wrapped a thin leg whole, so a spot or a dapple ran
        // round it as a band; an animal's spots shrink toward its feet.
        let (points, _, body) = surface(Family::Dog);
        let coat = Coat::default();
        let fields = Fields::new(coat.marks, body.height);
        let mut down = 0;
        for &(at, normal) in &points {
            let (turn, cells) = paint(&body, &coat, (at, normal, Zone::Chest), |p| {
                (p.turn(), p.cells(9.0))
            });
            if turn >= 1.0 {
                down += 1;
                assert_eq!(cells, fields.cells(at, 9.0 * LEG_FINER).0, "at {at}");
            } else if turn <= 0.0 {
                assert_eq!(cells, fields.cells(at, 9.0).0, "at {at}");
            }
        }
        assert!(down > 100, "only {down} points down the legs");
    }

    /// A body's vertices, each with its normal and its zone.
    type Zoned = Vec<(Vec3, Vec3, Zone)>;

    /// Two dozen rolled beasts, every family and their hybrids: each
    /// vertex with its normal and the zone the skin map gives it, and the
    /// body the painter reads.
    fn rolled() -> Vec<(i64, Zoned, CoatBody)> {
        // Only the mesh and its zones are read.
        let config = crate::AvatarConfig {
            atlas: 64,
            ..crate::AvatarConfig::default()
        };
        (0..24)
            .map(|seed| {
                let mut record =
                    AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
                record.reroll(seed);
                let skeleton = record.skeleton();
                let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
                let body = &avatar.parts.body;
                let points = body
                    .positions
                    .iter()
                    .zip(body.vertex_normals())
                    .zip(&avatar.parts.zones)
                    .map(|((&at, normal), &zone)| (at, normal, zone))
                    .collect();
                let coat_body = CoatBody::of(
                    &avatar.rig,
                    skeleton.beast_head.as_ref(),
                    skeleton.beast_feet.as_ref(),
                );
                (seed, points, coat_body)
            })
            .collect()
    }

    #[test]
    fn a_point_and_a_bay_s_black_lie_down_the_legs_and_nowhere_else_as_low() {
        // A colourpoint's dark legs and a bay's black ones are measured by
        // height, which every low part of the body shares: without the leg's
        // share against them, the points' dark ran over everything below
        // their line and the bay's black over everything below its own -
        // a low belly, a cat's chin, a hanging tail (#376). Asked as the
        // barrel, so that the tail's own points stand aside, and off the head,
        // whose own fade by how far a texel lies from it (#395), neither is ever
        // darker than a texel is leg, and low on a leg both are all dark.
        let (pointed, bay) = (
            Coat {
                points: true,
                ..Coat::default()
            },
            Coat {
                agouti: Agouti::Bay,
                ..Coat::default()
            },
        );
        let (mut under_points, mut under_bay) = (0, 0);
        for (seed, points, body) in rolled() {
            // As pale as a point gets: the top of the back's.
            let pale = paint(&body, &pointed, (body.withers, Vec3::Y, Zone::Chest), |p| {
                p.pointed(Vec3::ZERO)
            });
            let mut dark_legs = 0;
            for &(at, normal, _) in &points {
                let texel = (at, normal, Zone::Chest);
                if paint(&body, &pointed, texel, |p| p.off_head()) < 1.0 {
                    continue;
                }
                let (share, rise) = (body.junctions.share(at), body.rise(at));
                let point =
                    1.0 - paint(&body, &pointed, texel, |p| p.pointed(Vec3::ZERO)).x / pale.x;
                let black = paint(&body, &bay, texel, |p| p.bay());
                assert!(
                    point <= share + 1.0e-5,
                    "seed {seed}: a point {point:.2} dark on {share:.2} leg at {at}"
                );
                assert!(
                    black <= share,
                    "seed {seed}: a bay {black:.2} black on {share:.2} leg at {at}"
                );
                // Where height alone would have darkened it.
                under_points += usize::from(step(0.62, 0.30, rise) > share + 0.02);
                under_bay += usize::from(step(0.50, 0.38, rise) > share + 0.02);
                if share == 1.0 && rise < 0.3 {
                    assert!(point > 1.0 - 1.0e-5, "seed {seed}: a pale leg at {at}");
                    assert_eq!(black, 1.0, "seed {seed}: a bay's leg not black at {at}");
                    dark_legs += 1;
                }
            }
            assert!(dark_legs > 100, "seed {seed}: {dark_legs} down the legs");
        }
        assert!(
            under_points > 50 && under_bay > 50,
            "only {under_points} and {under_bay} points lie under the lines"
        );
    }

    #[test]
    fn a_bib_runs_down_the_chest_and_not_down_the_legs() {
        // A bib is measured from the midline, and a narrow chest's forelegs
        // stand inside its width, and a short-legged beast's hind legs too:
        // without the leg's share against it, a bib ran on down the inside of
        // the legs (#376).
        let coat = Coat::default();
        let mut inside = 0;
        for (seed, points, body) in rolled() {
            let mut chest = 0;
            for &(at, normal, zone) in &points {
                let bib = paint(&body, &coat, (at, normal, zone), |p| p.bib(normal, zone));
                let share = body.junctions.share(at);
                assert!(
                    bib <= 1.0 - share,
                    "seed {seed}: a bib {bib:.2} on {share:.2} leg at {at}"
                );
                chest += usize::from(trunk(zone) && bib > 0.5);
                // Where the midline alone would have whitened it.
                inside += usize::from(share == 1.0 && at.x.abs() < 0.12 * body.height);
            }
            assert!(chest > 20, "seed {seed}: only {chest} under the bib");
        }
        assert!(inside > 50, "only {inside} of the legs lie inside a bib");
    }

    #[test]
    fn no_mark_stops_on_the_line_between_the_neck_and_the_head() {
        // Both sides of every edge that crosses the line, asked of the same point:
        // all that differs is what the zone says, which no animal's coat knows of.
        // A tan throat, a bib, a piebald's lean, an extreme white's cap and a
        // colourpoint's mask each stopped on it (#395).
        let config = crate::AvatarConfig {
            atlas: 64,
            ..crate::AvatarConfig::default()
        };
        // On black, whose dark and light differ, or half of these marks draw nothing.
        let black = Coat {
            pigment: Pigment::Black,
            ..Coat::default()
        };
        let coats = [
            Coat {
                agouti: Agouti::TanPoints,
                ..black
            },
            Coat {
                agouti: Agouti::Bay,
                ..black
            },
            Coat {
                white: White::Piebald,
                ..black
            },
            Coat {
                white: White::Extreme,
                ..black
            },
            Coat {
                white: White::Irish,
                ..black
            },
            Coat {
                points: true,
                ..black
            },
        ];
        let mut worst = (0.0f32, 0i64, 0usize);
        let mut crossed = 0;
        for seed in 0..24i64 {
            let mut record =
                AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
            record.reroll(seed);
            let skeleton = record.skeleton();
            let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
            let body = &avatar.parts.body;
            // The line a texel's zone changes on is its chart's.
            let head = skeleton.beast_head.as_ref().expect("a beast's head");
            let zones = crate::face::beast::chart_zones(body, head, &avatar.parts.zones);
            let normals = body.vertex_normals();
            let coat_body = CoatBody::of(
                &avatar.rig,
                skeleton.beast_head.as_ref(),
                skeleton.beast_feet.as_ref(),
            );
            for face in &body.faces {
                for corner in 0..face.len() {
                    let (a, b) = (
                        face[corner] as usize,
                        face[(corner + 1) % face.len()] as usize,
                    );
                    if !matches!(
                        (zones[a], zones[b]),
                        (Zone::Head, Zone::Neck) | (Zone::Neck, Zone::Head)
                    ) {
                        continue;
                    }
                    crossed += 1;
                    let at = (body.positions[a] + body.positions[b]) * 0.5;
                    let normal = (normals[a] + normals[b]).normalize_or(Vec3::Y);
                    for (index, coat) in coats.iter().enumerate() {
                        let neck = paint(&coat_body, coat, (at, normal, Zone::Neck), |p| p.fur());
                        let head = paint(&coat_body, coat, (at, normal, Zone::Head), |p| p.fur());
                        let jump = (neck - head).abs().max_element();
                        if jump > worst.0 {
                            worst = (jump, seed, index);
                        }
                    }
                }
            }
        }
        assert!(crossed > 200, "only {crossed} edges cross the line");
        assert!(worst.0 < 0.05, "a mark jumps on the line: {worst:?}");
    }

    #[test]
    fn the_fur_s_grain_runs_unbroken_up_the_neck_onto_the_head() {
        // Switched where the next bone became the nearer, the strands turned on a
        // straight line round the neck and the back of the head (#395).
        for family in Family::ALL {
            let (rig, body) = rigged(family);
            let coat = Coat::default();
            let skull = rig.joints[rig.in_zone(Zone::Head)[0]].position;
            let up = Vec3::Y * 0.1 * body.height;
            let (from, to) = (body.withers + up, skull + up);
            let grain =
                |at: Vec3| paint(&body, &coat, (at, Vec3::Y, Zone::Neck), |p| p.grain(&rig));
            let samples = 4000;
            let mut last = grain(from);
            let mut worst: f32 = 0.0;
            for sample in 1..=samples {
                let here = grain(from.lerp(to, sample as f32 / samples as f32));
                worst = worst.max((here - last).abs());
                last = here;
            }
            assert!(worst < 0.08, "{family:?}: the grain jumps {worst:.3}");
        }
    }

    #[test]
    fn the_fur_s_grain_runs_unbroken_from_the_barrel_onto_a_leg() {
        // Switched where a leg's bone became the nearer, the strands turned
        // on a line round the top of every leg (#376).
        for family in Family::ALL {
            let (rig, body) = rigged(family);
            let coat = Coat::default();
            for limb in crate::plan::Limb::ALL {
                let junction = rig.joints[rig
                    .in_zone(Zone::UpperLimb(limb))
                    .into_iter()
                    .find(|&joint| !rig.joints[joint].marker)
                    .expect("a joint outside the trunk")]
                .position;
                // From straight above the junction where the trunk is all
                // trunk, to as far below it.
                let reach = (1..20)
                    .map(|step| step as f32 * 0.05 * body.height)
                    .find(|&reach| body.junctions.share(junction + Vec3::Y * reach) == 0.0)
                    .expect("the trunk above a junction");
                let (from, to) = (junction + Vec3::Y * reach, junction - Vec3::Y * reach);
                let samples = 4000;
                let grain = |at: Vec3| {
                    paint(&body, &coat, (at, Vec3::X, Zone::UpperLimb(limb)), |p| {
                        (p.grain(&rig), p.limb())
                    })
                };
                let (mut last, start) = grain(from);
                assert_eq!(start, 0.0, "{family:?} {limb:?}: the path starts on a leg");
                let mut worst: f32 = 0.0;
                let mut end = 0.0;
                for sample in 1..=samples {
                    let (here, share) = grain(from.lerp(to, sample as f32 / samples as f32));
                    worst = worst.max((here - last).abs());
                    (last, end) = (here, share);
                }
                assert_eq!(end, 1.0, "{family:?} {limb:?}: the path ends off the leg");
                assert!(
                    worst < 0.08,
                    "{family:?} {limb:?}: the grain jumps {worst:.3}"
                );
            }
        }
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
        ask(&Painter::new(&texel, coat, &palette, &fields, body))
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
