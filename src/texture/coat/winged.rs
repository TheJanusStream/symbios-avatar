//! A winged body's coat (W4, #423): the marks its families draw over it, and
//! its membrane's own material.
//!
//! **The marks are the family's, not the coat's** (research-10 5.3): a fruit
//! bat's mantle - a paler, golden-brown collar over its neck and shoulders -
//! and a sugar glider's dark stripe from its forehead down its spine and one
//! through each eye to the ear. Neither is stored: each is as strong as its
//! family's share of the body ([`WingedMarks`]), laid over the coat after its
//! pattern and before its belly lightens, so white spotting still goes over
//! them, and placed as every other mark is, by anatomy in body space - the
//! spine, the junctions, the declared head's frame - so none stops on a
//! chart's line. Fur reads them as the atlas does ([`super::CoatSampler`]),
//! so a bat's ruff is its mantle's colour.
//!
//! **A membrane is skin, not coat** (10 5.2: "dark brown to black membranes
//! in *Pteropus*", "translucent enough for cross-polarised imaging; veins and
//! elastin show when backlit"). It has a region of the atlas to itself
//! ([`MembraneChart`]), and each of its texels says what it is by where it
//! lies there (`wing::charted`): a bat's sheet is dark, paler and warmer
//! toward its free edge as if lit through, with faint fibres running along
//! its rows; its digits are the bone under their skin, darker; and a glider's
//! patagium, which is furred, is its coat, paling a little toward its edge.
//!
//! **The light through it is faked in colour.** The skin's draw is one opaque
//! material with the body's, so a membrane cannot let light through; a real
//! one would need the renderer to give the membrane a draw of its own with a
//! two-sided material that transmits - diffuse transmission through a
//! thickness, or a thin-surface subsurface term, lit from behind by the sun -
//! and a texture of how thin it is where. The colour here is what such a
//! membrane looks like with the sun behind it at a game's distance.

use glam::{Vec2, Vec3};

use super::marks::step;
use super::palette::WHITE;
use super::{FUR_ROUGHNESS, LEATHER_ROUGHNESS, Painter};
use crate::plan::WingedMarks;
use crate::wing::Charted;

/// A flying fox's mantle, in sRGB as every colour here: tan to golden (10
/// 5.3), toward a golden brown.
/// Provenance: **chosen by render**.
const MANTLE: Vec3 = Vec3::new(0.66, 0.45, 0.21);

/// How much of the coat's own light the mantle takes, so a recoloured bat's
/// collar leans toward its coat. Provenance: **chosen by render**.
const MANTLE_LIGHT: f32 = 0.25;

/// How dark a glider's stripes are against its coat's dark: darker still,
/// as a sugar glider's black stripe is against its grey (S31).
/// Provenance: **chosen by render**, and taken from 0.55 to 0.25 at W4's
/// merge against the glider's stored blue-grey coat, which is paler than the
/// stand-in it was first chosen on: at 0.55 the stripe behind the eye was a
/// tenth darker than the cheek.
const STRIPE_SHADE: f32 = 0.25;

/// Half a glider's dorsal stripe's width, as a share of its spine, and its
/// eye stripe's, in head radii. Provenance: **chosen by render**.
const STRIPE_WIDTH: f32 = 0.032;
/// See [`STRIPE_WIDTH`].
const EYE_STRIPE: f32 = 0.2;
/// How far round a glider's eye its dark ring reaches, in the globe's radii.
/// Provenance: **chosen by render**.
const EYE_RING: f32 = 1.3;

/// A bat's membrane: dark brown, nearly black (10 5.2, bg).
/// Provenance: **chosen by render**.
const MEMBRANE: Vec3 = Vec3::new(0.105, 0.078, 0.066);

/// The membrane with the light through it: a warm, reddish brown, the blood
/// in it showing. Provenance: **chosen by render**.
const LIT: Vec3 = Vec3::new(0.36, 0.21, 0.14);

/// How far toward [`LIT`] a membrane comes at its free edge.
/// Provenance: **chosen by render**.
const LIT_EDGE: f32 = 0.5;

/// How far out toward the free edge the light through a membrane starts to
/// show: the inner half is thick with muscle and vessels (10 5.2).
/// Provenance: **chosen by render**.
const LIT_FROM: f32 = 0.5;

/// How much paler and warmer a glider's furred patagium is at its edge,
/// where it is thinnest. Provenance: **chosen by render**, and raised from
/// 0.35 at W4's merge for the stored blue-grey coat, on which the paler
/// target sits close to the coat itself.
const FURRED_EDGE: f32 = 0.5;

/// The fibres along a membrane's rows: how many across its chart from root to
/// edge, how dark one is against the sheet, and how wide, as a share of the
/// gap between two. Provenance: **chosen by render**.
const FIBRES: f32 = 26.0;
/// See [`FIBRES`].
const FIBRE_SHADE: f32 = 0.12;
/// See [`FIBRES`].
const FIBRE_WIDTH: f32 = 0.18;

/// How glossy a membrane is: leather's sheen, a little duller.
/// Provenance: **chosen by render**.
const MEMBRANE_ROUGHNESS: f32 = 0.5;

/// A winged body's own surface as the painter reads it (#423).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WingedCoat {
    /// How much of each family's mark it draws.
    pub marks: WingedMarks,
    /// Where its membrane lies in the atlas: `None` on a bird.
    pub membrane: Option<MembraneChart>,
}

/// The region of the atlas a membrane is charted in (#423), as the build
/// reserved it, and what kind of membrane it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MembraneChart {
    /// Its corner nearest the atlas's origin, in texture coordinates.
    pub min: Vec2,
    /// How far it runs each way.
    pub size: Vec2,
    /// Whether it is a bat's naked membrane rather than a glider's furred
    /// patagium.
    pub naked: bool,
}

impl MembraneChart {
    /// The region `uvs` cover: a membrane's own texture coordinates fill the
    /// region it was placed in.
    pub(crate) fn of(uvs: &[Vec2], naked: bool) -> Option<Self> {
        let first = *uvs.first()?;
        let (min, max) = uvs.iter().fold((first, first), |(low, high), uv| {
            (low.min(*uv), high.max(*uv))
        });
        let size = max - min;
        (size.min_element() > 0.0).then_some(Self { min, size, naked })
    }

    /// Where in the region the texel `index` of a `width` by `height` atlas
    /// lies, `0..1` each way, if it lies in it or a texel short of it: the
    /// texels a membrane's triangles touch at its border are its own.
    pub(crate) fn local(&self, index: usize, width: u32, height: u32) -> Option<Vec2> {
        let (width, height) = (width.max(1), height.max(1));
        let (x, y) = (index as u32 % width, index as u32 / width);
        let uv = Vec2::new(
            (x as f32 + 0.5) / width as f32,
            (y as f32 + 0.5) / height as f32,
        );
        let local = (uv - self.min) / self.size;
        let margin = Vec2::new(1.0 / width as f32, 1.0 / height as f32) / self.size;
        let inside = local.cmpge(-margin).all() && local.cmple(Vec2::ONE + margin).all();
        inside.then(|| local.clamp(Vec2::ZERO, Vec2::ONE))
    }
}

impl Painter<'_> {
    /// `colour` with this body's families' marks laid over it.
    pub(super) fn marked(&self, colour: Vec3, marks: WingedMarks) -> Vec3 {
        let mut colour = colour;
        if marks.mantle > 0.0 {
            let mantle = MANTLE.lerp(self.palette.light, MANTLE_LIGHT);
            colour = colour.lerp(mantle, self.mantle() * marks.mantle);
        }
        if marks.stripes > 0.0 {
            colour = colour.lerp(
                self.palette.dark * STRIPE_SHADE,
                self.glider_stripes() * marks.stripes,
            );
        }
        colour
    }

    /// How much of a fruit bat's mantle lies here, `0..=1`: over the neck and
    /// the shoulders, all round the neck but strongest along its top, fading
    /// back along the barrel, down the fore limbs and before the head - which
    /// a flying fox wears darker - over the junctions and the head's reach,
    /// not on a zone's line.
    fn mantle(&self) -> f32 {
        let (at, normal) = (self.texel.position, self.texel.normal);
        let along = self.body.along_spine(at) + 0.05 * self.fields.warp(at, 5.0);
        let shoulders = step(0.5, 0.8, along);
        // Over the top and down the sides, a little under the throat, and not
        // down the chest, which a flying fox wears dark.
        let round = 0.15 + 0.85 * step(-0.1, 0.5, normal.y);
        let head = 0.25 + 0.75 * self.off_head();
        shoulders * round * (1.0 - self.limb()) * head
    }

    /// How much of a sugar glider's stripes lies here, `0..=1`: down the
    /// middle of the back from the forehead, fading out over the rump, and
    /// through each eye to the ear. As wide as a share of the spine: a
    /// crouched glider's back stands low, and a share of its height drew a
    /// pencil line.
    fn glider_stripes(&self) -> f32 {
        let (at, normal) = (self.texel.position, self.texel.normal);
        let spine = self.body.withers.distance(self.body.hips).max(f32::EPSILON);
        let width = STRIPE_WIDTH + 0.006 * self.fields.warp(at, 7.0);
        let middle = step(width + 0.012, width, at.x.abs() / spine);
        let dorsal = step(0.15, 0.55, normal.y);
        let back = step(0.02, 0.32, self.body.along_spine(at));
        // Ending at the forehead, before the snout, whose top faces up too.
        let snout = self.on_head(|local| step(0.42, 0.62, local.z));
        let spine = middle * dorsal * back * (1.0 - snout);
        let eyes = self.on_head(|local| {
            let Some(head) = self.body.head.as_ref() else {
                return 0.0;
            };
            let eye = head.eye.centre / head.radius;
            let ear = head.ear.base / head.radius;
            let (eye, ear) = (eye.with_x(eye.x.abs()), ear.with_x(ear.x.abs()));
            // From a little ahead of the eye, through it, back to the ear's root.
            let from = eye + (eye - ear).normalize_or(Vec3::Z) * 0.3;
            let point = local.with_x(local.x.abs());
            let run = eye.lerp(ear, 0.85) - from;
            let t =
                ((point - from).dot(run) / run.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
            let stripe = step(EYE_STRIPE, EYE_STRIPE * 0.6, point.distance(from + run * t));
            // And round the eye, the lids with it, which the stripe runs through.
            let globe = head.eye.radius / head.radius * EYE_RING;
            stripe.max(step(globe, globe * 0.75, point.distance(eye)))
        });
        spine.max(eyes)
    }

    /// A membrane's texel (#423): its colour, how rough it is and how much of
    /// a coat's grain it carries, at `local` in its chart, which says it is
    /// `charted`.
    pub(super) fn membrane(&self, naked: bool, charted: Charted, local: Vec2) -> (Vec3, f32, f32) {
        let base = MEMBRANE.lerp(self.palette.leather, 0.25);
        match charted {
            // The bone under a digit's skin, darker than the sheet it carries.
            Charted::Digit => (base * 0.75, LEATHER_ROUGHNESS, 0.0),
            Charted::Sheet { edge } if naked => {
                let lit = step(LIT_FROM, 1.0, edge);
                let colour = base.lerp(LIT, LIT_EDGE * lit * lit);
                let at = self.texel.position;
                let across = (local.y * FIBRES + 0.35 * self.fields.warp(at, 9.0)).fract();
                // How far from the nearest fibre's middle, in halves of a gap.
                let off = 1.0 - (across - 0.5).abs() * 2.0;
                let fibre = step(FIBRE_WIDTH, 0.0, off);
                (
                    colour * (1.0 - FIBRE_SHADE * fibre),
                    MEMBRANE_ROUGHNESS,
                    0.0,
                )
            }
            Charted::Sheet { edge } => {
                let fur = self.fur();
                let lit = step(0.4, 1.0, edge);
                let warm = fur.lerp(WHITE, 0.5).lerp(LIT, 0.1);
                (fur.lerp(warm, FURRED_EDGE * lit), FUR_ROUGHNESS, 1.0)
            }
        }
    }
}
