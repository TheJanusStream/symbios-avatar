//! A winged body's head (#422): the beast's head (`quadruped::head`), and what
//! a bird, a fruit bat and a sugar glider make of it.
//!
//! **The beast's construction, dressed afterwards.** `quadruped::head` builds
//! the braincase, the muzzle, the jaw and the seats every beast has, from the
//! stand-in axes a family's `Face` gives it (`derive::winged`); this module
//! then carves an owl's disc into the face, seats the eyes again against what
//! the head now is - larger, a glider's standing out of its skull, an owl's
//! looking straight out of its dish - sizes the ears, and on a bird-led head
//! declares the beak in place of the muzzle's mouth. Nothing here runs for a
//! beast, so not one of a beast's numbers moves.
//!
//! **What a cross takes, and from whom.** The kind of mouth is the leading
//! family's, switching at the halfway blend as the posture does: a bird-led
//! head has a beak and no fleshy nose, whiskers or teeth, a bat- or glider-led
//! one a muzzle. The eyes' size and set, the ears' size and the snout's length
//! are blended across the cross, so they move continuously through the switch;
//! the disc, a part of a bird's face, is carved as deep as the bird's share of
//! it.
//!
//! Every coefficient is in cranium radii and **chosen by render** (#422), on
//! `render --winged --head` across the families and the bird's reach - research-10
//! measures no bird's beak against its skull but a raven's culmen, and no
//! face of the other two at all.

use glam::{Vec2, Vec3};

use super::quadruped::head::{self, Look};
use crate::det::DetMath;
use crate::plan::QuadrupedParams;
use crate::plan::beast_head::{BeakSeat, BeastHead, Blob, Disc};
use crate::plan::kin::Lineage;
use crate::plan::winged::{WingedFamily, WingedParams};

/// Where a bird's eyes are set at no disc, on the `eyeSet` scale the beast
/// head reads: round the side of the skull, as a crow's and a songbird's.
const LATERAL_EYES: f32 = 0.4;

/// And at a full disc: on the front of the face, close together, an owl's.
/// The bird's `Face` holds this, the bird's default.
pub(super) const FRONTAL_EYES: f32 = -1.4;

/// How much larger a bird's eyes are at no disc than at a full one, against
/// the beast's: an owl's are the largest of any bird's for its head.
const DISCLESS_EYES: f32 = -0.2;

/// How far a full disc turns an eye's gaze from its dish's normal toward
/// straight ahead: the dish leans every normal in toward its middle, which
/// would cross the owl's eyes.
const DISC_AHEAD: f32 = 0.7;

/// The disc's dish at full strength: how deep it is under its rim, and the
/// rim's own radius, in cranium radii.
const DISH_DEPTH: f32 = 0.08;
/// See [`DISH_DEPTH`].
const RUFF: f32 = 0.05;

/// How far across the face the dish reaches from its middle, in cranium
/// radii, and how far below the braincase's centre that middle is: an owl's
/// disc frames its eyes above and its beak below the middle.
const DISH_REACH: f32 = 0.80;
/// See [`DISH_REACH`].
const DISH_DROP: f32 = 0.12;

/// How softly the dish's edge turns into the head, and the ruff joins it, in
/// cranium radii: rounded (the owner's call at W3's review), where at 0.05
/// each the rim stood sharp enough that in profile the face read as a hood
/// with its front cut away. Chosen by render against 0.10 and 0.20, and
/// against a smaller dish (reach 0.70), which softened the profile more but
/// shrank the disc and creased the brow above the beak.
const DISH_EDGE: f32 = 0.30;
/// See [`DISH_EDGE`].
const RUFF_JOIN: f32 = 0.12;

/// How far below level the beak's base is seated, as the angle of the ray
/// from the braincase's centre that seats it, in radians: between and under
/// the eyes.
const BEAK_SEAT: f32 = 0.22;

/// The horn a beak is drawn in until a plumage stores its colours (W4,
/// #423): an owl's pale horn, darkening toward a crow's black as the beak
/// lengthens.
const HORN: Vec3 = Vec3::new(0.60, 0.55, 0.42);
/// See [`HORN`].
const CROW_BLACK: Vec3 = Vec3::new(0.08, 0.08, 0.09);

/// What a winged body asks of its head beyond a beast's stand-in axes.
#[derive(Clone, Copy, Debug)]
pub(super) struct Asked {
    /// The beak's length axis.
    beak_length: f32,
    /// The beak's hook axis.
    beak_hook: f32,
    /// How full the disc is, `0` none to `1.5`: `facialDisc` plus one.
    disc: f32,
    /// How young the body is drawn, `-0.5..=1.33`: the derive's `young`.
    young: f32,
    /// How much of the body is bird, `0..=1`.
    bird: f32,
    /// Whether the bird leads it, and so whether it has a beak.
    beaked: bool,
}

impl Asked {
    /// What `params` asks, `young` as the derive reads `appeal`.
    pub(super) fn of(params: &WingedParams, young: f32) -> Self {
        let lineage = Lineage {
            family: params.family,
            hybrid: params.hybrid,
            blend: params.blend,
        };
        Self {
            beak_length: params.beak_length.clamp(-1.5, 2.0),
            beak_hook: params.beak_hook.clamp(-1.5, 1.5),
            disc: (1.0 + params.facial_disc).clamp(0.0, 1.5),
            young,
            bird: share(lineage, WingedFamily::Bird),
            beaked: lineage.leading() == WingedFamily::Bird,
        }
    }

    /// How much of a full disc the bird's own face has, `0..=1`.
    fn disc_share(&self) -> f32 {
        self.disc.min(1.0)
    }

    /// How far the disc moves a blended face's `eyeSet` from where its
    /// families' own faces put it: the bird's share of the bird's own move
    /// from an owl's frontal eyes toward a crow's lateral ones.
    pub(super) fn eye_set_shift(&self) -> f32 {
        self.bird * (1.0 - self.disc_share()) * (LATERAL_EYES - FRONTAL_EYES)
    }

    /// And its eyes' size, likewise.
    fn eye_scale_shift(&self) -> f32 {
        self.bird * (1.0 - self.disc_share()) * DISCLESS_EYES
    }

    /// Whether the head has a beak.
    pub(super) fn beaked(&self) -> bool {
        self.beaked
    }
}

/// How much of a body of `lineage` is `kin`, `0..=1`.
fn share(lineage: Lineage<WingedFamily>, kin: WingedFamily) -> f32 {
    let blend = lineage.blend.clamp(0.0, 1.0);
    match lineage.crossed() {
        None => f32::from(u8::from(lineage.family == kin)),
        Some(other) => {
            let own = if lineage.family == kin {
                1.0 - blend
            } else {
                0.0
            };
            own + if other == kin { blend } else { 0.0 }
        }
    }
}

/// How a blended family face sizes what the beast's construction does not:
/// the eyes' globes against a beast's, how far they stand out, and the ears'
/// size against a beast's.
#[derive(Clone, Copy, Debug)]
pub(super) struct Sizes {
    /// The globes' size against a beast's.
    pub eyes: f32,
    /// How much of a globe stands proud of the head.
    pub proud: f32,
    /// The ears' size against a beast's: none on a bird.
    pub ears: f32,
}

/// Dresses `head`, the beast's construction of a winged body's head, as
/// `asked`: the disc, the eyes, the ears and the beak, in that order, since
/// the eyes and the beak are seated on the dish. `proxy` and `appeal` are what
/// the head was built from.
pub(super) fn dress(
    head: &mut BeastHead,
    proxy: &QuadrupedParams,
    appeal: f32,
    sizes: Sizes,
    asked: &Asked,
) {
    let r = head.radius;
    // The disc, as deep as the bird's share of its face: a part of a bird's
    // face, carved only into a bird-led head.
    let disc = asked.disc * asked.bird;
    if asked.beaked && disc > 0.0 {
        carve(head, disc);
    }
    // The eyes, against the head as it now stands.
    let ahead = if head.disc.is_some() {
        DISC_AHEAD * asked.disc_share()
    } else {
        0.0
    };
    head.eye = head::eye_seat(
        head,
        proxy,
        appeal,
        &Look {
            scale: (sizes.eyes + asked.eye_scale_shift()).max(0.5),
            proud: sizes.proud,
            ahead,
        },
    );
    // The ears, as large as the blend makes them: none on a bird, whose ear
    // is an opening under its feathers. An ear much smaller than a beast's
    // smallest is none at all.
    let ears = sizes.ears.max(0.0);
    let seat = &mut head.ear;
    if ears * seat.length < 0.15 * r {
        seat.length = 0.0;
        seat.width = 0.0;
    } else {
        let rise = seat.frame * Vec3::Y;
        // Rooted where it was: the base stands 0.12 of the length down the
        // ear from the surface.
        seat.base += rise * (0.12 * seat.length * (1.0 - ears));
        seat.length *= ears;
        seat.width *= ears;
        seat.thickness *= ears;
    }
    if asked.beaked {
        beak(head, asked);
    }
}

/// Carves an owl's facial disc into the front of `head` at `strength`
/// (`Disc`): a dish whose rim stands [`DISH_REACH`] across the face from its
/// middle, [`DISH_DEPTH`] deep at full strength, ringed by a ruff. Under full
/// strength the full dish is blended back toward the bare face by `strength`
/// (`Disc::weight`); over it, up to 1.2, the dish deepens.
fn carve(head: &mut BeastHead, strength: f32) {
    let r = head.radius;
    let middle = Vec2::new(0.0, head.cranium.centre.y - DISH_DROP * r);
    let reach = DISH_REACH * r * head.cranium.radii.x / (0.93 * r);
    // Where the rim stands along the face: on the braincase's front, at the
    // dish's reach from its middle.
    let rim_z = {
        let radii = head.cranium.radii;
        let off = Vec2::new(
            reach / radii.x,
            (middle.y - head.cranium.centre.y) / radii.y,
        );
        head.cranium.centre.z + radii.z * (1.0 - off.length_squared()).max(0.04).sqrt()
    };
    // The ball through the rim whose cap sinks `depth` under it at the
    // middle: radius (reach^2 + depth^2) / (2 depth).
    let deep = strength.clamp(1.0, 1.2);
    let depth = DISH_DEPTH * r * deep;
    let radius = (reach * reach + depth * depth) / (2.0 * depth);
    head.disc = Some(Disc {
        centre: Vec3::new(0.0, middle.y, rim_z - depth + radius),
        radius,
        carve: DISH_EDGE * r,
        rim: RUFF * r * deep.sqrt(),
        blend: RUFF_JOIN * r,
        weight: strength.clamp(0.0, 1.0),
    });
    // The surface is projected from the head's axis, so its front end has to
    // stay inside the head: on a bird the nose ends the axis, and the dish
    // carves it away. Drawn back along the axis until it is well inside, so the
    // charts' axis keeps its line.
    let [back, mut front] = head.axis;
    for _ in 0..40 {
        if head.distance(front) < -0.15 * r {
            break;
        }
        front = front.lerp(back, 0.05);
    }
    head.axis[1] = front;
}

/// Declares a bird's beak on `head` (`BeakSeat`), as `asked`, in place of the
/// muzzle's mouth: no nostrils cut, no whiskers, and the jaw a joint for the
/// lower mandible rather than a part of its own (`face::beast::jaw`).
///
/// **Its length** is an owl's short beak at the axis's nought and a crow's,
/// about as long as the rest of its head (research-10 6.1: a raven's culmen
/// 0.07 of its length), at one; **its hook** a curve of the upper mandible's
/// line from a songbird's straight one to most of a right angle on a parrot's;
/// **its depth** follows both, a parrot's "deep hooked bill" (10 3.1) against a
/// songbird's thin one. A young bird's is shorter (10 fact 10).
fn beak(head: &mut BeastHead, asked: &Asked) {
    let r = head.radius;
    let length = asked.beak_length;
    let hook = asked.beak_hook;
    let visible = if length >= 0.0 {
        0.45 + 0.85 * length
    } else {
        0.45 * (1.0 + 0.45 * length)
    }
    .clamp(0.2, 2.6)
        * r
        * (1.0 - 0.3 * asked.young.max(0.0));
    let turn = (1.2 + 0.7 * hook).clamp(0.0, 2.2);
    let depth = visible * (0.55 + 0.22 * hook).clamp(0.36, 1.0);
    // Down from the head's forward: a crow's nearly level, an owl's steeply
    // down out of its dish.
    let pitch = (0.15 + 0.2 * hook.clamp(-1.0, 1.0) + 0.3 * asked.disc_share() * asked.bird)
        .clamp(0.05, 0.9);
    let forward = Vec3::new(0.0, -pitch.det_sin(), pitch.det_cos());
    let up = Vec3::new(0.0, pitch.det_cos(), pitch.det_sin());
    let centre = head.cranium.centre;
    let toward = Vec3::new(0.0, -BEAK_SEAT.det_sin(), BEAK_SEAT.det_cos());
    let surface = head.leave(centre, toward).unwrap_or(centre + toward * r);
    // Sunk into the face, so the base's end lies inside it.
    let sink = 0.45 * depth;
    let base = surface - forward * sink;
    let dark = (0.8 * length - 0.1 * hook).clamp(0.0, 1.0);
    let colour = HORN.lerp(CROW_BLACK, dark);
    head.beak = Some(BeakSeat {
        base,
        forward,
        up,
        length: visible + sink,
        depth,
        lower_depth: 0.55 * depth,
        half_width: 0.6 * depth,
        hook: turn,
        lower_reach: (0.95 - 0.18 * turn).clamp(0.55, 0.97),
        colours: [colour, (colour * 1.18).min(Vec3::ONE)],
    });
    // No fleshy nose: the nostril is declared with no size, at the beak's
    // base, which the nose's own refinement then gathers round.
    head.nose.leather = None;
    head.nose.nostril = Blob {
        centre: surface * Vec3::new(0.0, 1.0, 1.0),
        radii: Vec3::ZERO,
    };
    head.whiskers = None;
    // Nor a muzzle's lips: the lobes either side of a beast's nose drew a
    // mouth's line under the beak, and full cheeks a mammal's jowls. The lobes
    // sink into the braincase, where they add nothing, and the cheeks draw in.
    head.pads = Blob {
        centre: head.cranium.centre,
        radii: Vec3::splat(0.05 * r),
    };
    let disc = (asked.disc * asked.bird).clamp(0.0, 1.0);
    head.cheeks.radii *= CHEEKS_BARE + (CHEEKS - CHEEKS_BARE) * disc;
}

/// How far a beaked head's cheeks draw in against a beast's under a full
/// disc, which carves the face round them.
const CHEEKS: f32 = 0.8;
/// How far they draw in with no disc (the owner's call at W3's review): at
/// [`CHEEKS`] a crow's and a songbird's face kept a mammal's cheek either
/// side of the beak.
const CHEEKS_BARE: f32 = 0.6;
