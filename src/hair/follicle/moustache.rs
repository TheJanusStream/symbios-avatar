//! The upper lip, between the mouth and the nose, and on round the mouth.
//!
//! The cutaneous lip: the skin above the vermilion and below the nostrils,
//! philtrum included. Its floor is the tightest boundary on the head - a
//! moustache that reaches over the vermilion is a moustache in the mouth - so
//! it is read from the mouth itself rather than offset from a middle.
//!
//! # An outline, not a band (#475)
//!
//! The patch was a level band between two heights, cut off at a half-width,
//! and the owner's report is what that does: a straight bar that bleeds onto
//! the underside of the nose and never bends round the mouth. It is now an
//! `Outline` in the face's own plane:
//!
//! - **The floor is the top of the upper lip's vermilion as it curves down to
//!   the corners**: the lips are kept out as a lens round the parting the mouth
//!   was actually cut along (measured, see `face::skull::Parting`), from the
//!   vermilion on the midline down to the corners and a little past them, so
//!   the patch follows the lip down and clears the corners at every setting.
//! - **The ceiling stops under the nose**, where its underside begins: the
//!   6 mm between the nostril line and the nose's foot is the nose's own
//!   underside, which the old ceiling painted. Up into the subnasal crease and
//!   no further (#475's second fixer): to where the skull measured the skin
//!   turning to face the ground (`face::skull::Crease`) where the asker knows
//!   the skin's facing, a little over the foot where it does not, and the
//!   facing itself (see `FACING`) keeps the underside bare.
//! - **The width runs from a Chaplin to a horseshoe** on the record's one
//!   existing axis, [`Params::width`]: at `-1` a patch as wide as the face's
//!   own nose (its underside measured off the skull) with a straight bottom
//!   edge, at `0` a classic moustache whose ends, as thick as the band under
//!   the nose, turn down to the corners' height, at `+1` ends that run down
//!   beside the mouth to the chin, narrowing to a rounded bottom, joining a
//!   chin beard where there is one and stopping there where there is not.
//!   A sculpted moustache is cut at width `0` at every width above it (#475,
//!   the owner's decision; see `MoustacheStyle::Sculpted`).
//!
//! The grown styles read the same outline (`hair::style::moustache`), so
//! painted and grown cannot disagree about where the mouth is.

use serde::{Deserialize, Serialize};

use glam::Vec3;

use super::{At, MINIMUM_SPAN, Region, chin::Pad};
use crate::face::{Canon, FaceParams, Skull};

/// How the upper-lip patch is shaped on one head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Params {
    /// How far round the mouth it grows: `-1` a narrow patch under the nose
    /// (a Chaplin), `0` a moustache whose ends turn down at the mouth's
    /// corners, and `+1` ends that run down beside the mouth to the chin.
    ///
    /// A sculpted moustache reads it no wider than `0` (#475, the owner's
    /// decision): its solid and the paint under it keep the classic outline at
    /// every width above it.
    #[serde(with = "crate::plan::scaled")]
    pub width: f32,
    /// How far down toward the vermilion it grows, `-1` shy of it and `+1` onto
    /// its border.
    #[serde(with = "crate::plan::scaled")]
    pub drop: f32,
}

impl Params {
    /// Clamps each axis to the range its docstring promises.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.width = quantize(self.width.clamp(-1.0, 1.0));
        self.drop = quantize(self.drop.clamp(-1.0, 1.0));
    }
}

/// Where the upper lip's vermilion ends, as a share of mouth line to nose base.
///
/// Provenance: **looked up**, against the halving `examples/render`'s
/// `face_bands` uses for the same span. That convention — a band needing half
/// of a span takes the half between two landmarks — is a reasonable default
/// and slightly wrong here: an upper lip's vermilion is about 8 mm of a 19 mm
/// mouth-to-nose span, which is this rather than a half. Above it is skin that
/// grows hair; below it is lip that does not.
const VERMILION: f32 = 0.45;

/// How far that edge moves over the whole of [`Params::drop`], in the same
/// share.
///
/// Provenance: **tuned by render**.
const DROP_RANGE: f32 = 0.25;

/// Half a Chaplin's width, in [`Canon::unit`]s: the patch at `width` `-1`,
/// where the skull found no nose to measure.
///
/// **A Chaplin is as wide as the face's own nose** since #475's fixer: the
/// skull reads how far out the nose's underside reaches
/// (`Skull::nose_under`), and the patch's edge is half faded there. This is
/// the neutral nose's width for a head it could not read: the nose's wings
/// stand 16.3 mm either side of the midline on a default face
/// (`face::relief`'s neutral alar breadth of 32.6 mm), and one unit is 27.1 mm
/// there, so a patch whose edge is half faded at 15.5 mm is a toothbrush under
/// the nostrils.
///
/// **And never past the mouth's own corners** (#475's second fixer): a big
/// nose's underside reaches 28 mm out where the mouth's corners are 22 mm,
/// and a Chaplin as wide as that was wider than the classic's own band, so
/// the width slider narrowed from `-1` to `0`.
///
/// Provenance: **derived** from the relief's neutral nose and the fade.
const CHAPLIN: f32 = 0.62;

/// How broad a lip a grown moustache's clump count furnishes, in units.
///
/// 0.24.5's level band at `width` `0`, 0.95 units either side of the
/// midline: the count every style was tuned on. See [`Outline::spread_on`].
///
/// Provenance: **carried** from 0.24.5's band.
const BREADTH: f32 = 1.9;

/// How far past the mouth's corners a moustache's ends reach at `width` `+1`,
/// in units: the outer edge of the ends that run down to the chin there.
///
/// **At `0` the ends are as thick as the band is under the nose** (#475's
/// fixer): the classic's ends were 7.6 mm across where the band under the
/// nose is 4.6 mm, so the moustache read as two blobs on a thread, thinnest
/// exactly where a moustache is fullest. An end is the band's own height
/// across there, and never under [`END_FADES`], so it still comes on; it
/// widens to this as the width runs on to the chin.
///
/// Provenance: **tuned by render**.
const ENDS_WIDE: f32 = 0.55;

/// How much of its width an end running down beside the mouth gives up by its
/// bottom, either side of its own middle.
///
/// An end that runs to the chin at one width and stops in a flat cut read as
/// a strap; a moustache's ends thin as they go. Its bottom is rounded as
/// well, a half-disc as wide as the end is there.
///
/// Provenance: **tuned by render**.
const END_TAPER: f32 = 0.45;

/// How many of the mask's fades a classic moustache's end is across at the
/// least, past the lips' keep.
///
/// An end narrower than this never comes fully on and steps from bare skin to
/// its peak and back across a few texels: at two fades seed 237's painted
/// beard stepped 0.69 of its density between neighbouring texels (0.77 under
/// a nudge of libm) against the feather's bound of 0.70; at two and a half
/// 0.58, where it stood before #475's fixer (`texture::beard_tests`).
///
/// Provenance: **measured** against the beard feather's guard.
const END_FADES: f32 = 2.5;

/// How far past the cut's corners the lips are kept out, in units.
///
/// The parting's outermost coincident pair is one edge inside the weld, and a
/// corner stretches when the mouth moves: a moustache end painted against the
/// corner itself reads as hair in the mouth the first time it smiles. 2.7 mm
/// on a default face, which with the fade puts full weight 5.3 mm out.
///
/// Provenance: **derived** from the weld's edge and the fade, **tuned by
/// render**.
const CLEAR: f32 = 0.10;

/// How deep the lower lip is kept out below the parting, as a share of the
/// upper lip's height above it.
///
/// The lower lip's vermilion runs to about 0.9 plumps under the mouth line
/// where the upper runs to about 0.8 above it (`face::relief`'s lobes), and the
/// crease under it lower again; a little deeper keeps an end that runs down
/// beside the mouth off the lower lip's rounded corner.
///
/// Provenance: **derived** from the relief's lip lobes.
const BELOW: f32 = 1.3;

/// How far down toward the chin the ends reach at `width` `+1`, as a share of
/// chin to mouth line measured up from the chin.
///
/// Below the chin patch's own top at every `rise` the record has but the
/// lowest, which [`Moustache::of`] reads off the pad itself: the ends join a
/// chin beard where there is one, and where there is none they stop on the
/// chin, which is the horseshoe the owner asked for.
///
/// Provenance: **derived** from the chin region's ceiling (`chin::VERMILION`
/// 0.5 less its `RISE_RANGE` 0.28 is 0.22; this sits between).
const CHIN_JOIN: f32 = 0.35;

/// How steeply the top edge falls outward past the nose's wings, in metres a
/// metre.
///
/// A moustache is fullest under the nose and thins toward its ends; a top edge
/// held level out to the corners draws a slab.
///
/// Provenance: **tuned by render**.
const TOP_SLOPE: f32 = 0.5;

/// How gently the top edge starts to fall, in units: the run over which the
/// slope comes in, so the edge has no corner at the nose's wing.
///
/// Provenance: **tuned by render**.
const TOP_EASE: f32 = 0.10;

/// The skin's facing at which the nose's underside stops taking a moustache:
/// the vertical part of the skin's normal, `-1` facing the ground.
///
/// **The nose overhangs the lip on the carved face.** Measured on the default
/// face down the midline: the nose's underside faces the ground (normal `y`
/// -0.80 to -0.94) from 1 mm to 6 mm below the nose's base, the subnasal
/// crease under it -0.45 to -0.6, and the cutaneous lip under that faces
/// forward and UP (+0.37 to +0.74); at the wings, 12 mm out, -0.47 to -0.83
/// against the lip's +0.09. Seeds 175 and 237 read the same.
///
/// **A backstop, since #475's second fixer, not the edge.** The edge is the
/// ceiling the asker is given ([`Outline::seen`]: where the skull measured the
/// skin turning past this facing), which is a height and so is feathered with
/// the texels round it; a cut by facing alone is not - a texel reads its own
/// normal, which turns 0.16 to 0.28 from one texel to the next across the
/// crease - and every soft cut by facing tried stepped the paint 0.75 to 0.86
/// of its density between two texels (the feather's guard, 0.70). So this
/// only clears what the measured line misses, at the bound
/// `texture::beard_tests`' nose guard reads the underside by, over
/// [`FACING_FADE`]: skin facing the ground more steeply than this above the
/// vermilion is nose and is left bare. (#475 cut at -0.75 behind a ceiling at
/// the foot; the crease between is the moustache's.)
///
/// Provenance: **measured** (#475; #475's second fixer).
const FACING: f32 = -0.6;

/// How far over the nose's foot the patch reaches before it is bare, in
/// [`Canon::frame`]s: into the subnasal crease (#475's second fixer).
///
/// **The foot is where the nose's relief ends, not where its underside
/// begins.** Measured down the midline of seed 175's face and five of its
/// noses (the slider corners and the middle of `nose` and `noseWidth`): the
/// skin faces the ground past -0.45 from 4.9 to 5.7 mm over the vermilion,
/// within a millimetre of the foot (5.5 mm there), and past -0.6 from 5.7 to
/// 6.5 mm - the crease, which faces the ground less than the underside above
/// it (-0.80 to -0.94). A ceiling at the foot with the mask's whole fade under
/// it came fully on only in the lowest two millimetres of a lip five or six
/// tall, and the patch read as a pencil line along the vermilion where a
/// toothbrush was asked for. So the bare line stands this far over the foot,
/// with a shorter fade under it ([`TOP_FADE`]), for an asker blind to the
/// skin's facing; one that knows it is given the line the skull measured
/// instead ([`Outline::seen`]), and the facing ([`FACING`]) keeps whatever of
/// either is the nose's underside bare.
///
/// 0.89 mm on seed 175, under the millimetre over the foot at which the
/// Chaplin's guard asks the mask, blind to facing, for nothing.
///
/// Provenance: **measured** (#475's second fixer).
const CREASE: f32 = 0.007;

/// How much of the mask's fade the top edge fades over, under the bare line
/// ([`CREASE`]): the band under the nose is all the lip there is, and a whole
/// fade at each edge of a lip five millimetres tall leaves none of it full.
/// Three quarters still fades over two to three of the face chart's texels,
/// which the beard feather's step guard reads (`texture::beard_tests`).
///
/// Provenance: **tuned by render**, held under the beard feather's guards.
const TOP_FADE: f32 = 0.75;

/// How much of the mask's fade the edge along the lips fades over, over
/// them: the vermilion under the band. Beside the mouth, where an end runs
/// down past the corners, the lens keeps its whole fade.
///
/// Half, the Chaplin's own clipped bottom edge (`Outline::floor`'s level),
/// for every width since #475's second fixer: the classic's band under the
/// nose is the same five millimetres of lip, and a whole fade at its bottom
/// and its top left it a stroke along the vermilion. The bare line is where it
/// was, so nothing reaches nearer the lips than before.
///
/// Provenance: **tuned by render**, held under the beard feather's guards.
const LIP_FADE: f32 = 0.5;

/// How much facing the nose's cut fades over, linearly: full weight at
/// [`FACING`] plus this. Narrow, since it is a backstop (see [`FACING`]): a
/// texel it fades is one the measured ceiling has already all but cleared.
///
/// Provenance: **measured** (#475's second fixer), with [`FACING`].
const FACING_FADE: f32 = 0.05;

/// How softly the patch fades, in [`Canon::frame`]s.
///
/// Provenance: **tuned by render**.
const FADE: f32 = 0.025;

/// How much of the gap to the nose the patch stops short of, in the same share:
/// the nostril line, which is [`Lip::nostrils`].
///
/// **Under the nose the ceiling is [`Canon::nose_foot`] since #475**, where
/// the nose's relief has finished - 6.4 mm below the base on a default head,
/// and the 6 mm between is the nose's own underside, which took the paint (the
/// owner's report) - and the crease just over it since #475's second fixer
/// ([`CREASE`], [`Outline::seen`]). This line is the midline number the
/// styles are handed, and what bounds the foot from above.
///
/// Provenance: **derived** from the fade this region needs to clear.
const UNDER_NOSE: f32 = 0.05;

/// How far round the head the patch may reach.
///
/// In [`At::forward`]'s share. An upper lip is as far forward as a face gets,
/// so this is a high bar and exists to stop the region wrapping onto the cheek
/// on a narrow head.
///
/// Provenance: **tuned by render**.
const FRONT: f32 = 0.45;

/// The patch of lip a moustache grows on, as one object.
///
/// **Handed out to the styles rather than kept inside the mask**, following
/// [`Ridge`](super::brows::Ridge)'s discipline. If the style carried its own
/// copy of any of these the grown moustache could sit somewhere the painted
/// one does not, and the one boundary that cannot be got wrong - the mouth -
/// would have two opinions about where it is.
///
/// **These four numbers are the outline's middle and its extent, no longer
/// its whole shape** (#475): the floor curves down to the mouth's corners and
/// the ends run down beside the mouth, which the crate-private `Outline`
/// carries beside this. The vermilion and the nostrils are the floor and the
/// ceiling ON THE MIDLINE, and the half-width is how far out the outline
/// reaches anywhere.
///
/// It is also what makes the clearance a CONSTRUCTION rather than a check. A
/// clump knows the floor its own root stands above, so it can give up a share of
/// that room and never reach it; see `moustache::Whisker` in the style
/// catalogue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lip {
    /// The vermilion's top edge on the midline, in head-local metres: the
    /// floor there.
    ///
    /// Hair grows on skin and stops at it. Below this is lip, and a hair drawn
    /// past it is a hair in somebody's mouth. Out toward the corners the floor
    /// follows the vermilion down, so it is lower there than this.
    pub vermilion: f32,
    /// The nostril line, likewise: the ceiling on the midline.
    pub nostrils: f32,
    /// How far out the outline reaches either side of the midline, likewise.
    ///
    /// A nose's width at `width` `-1`, and past the mouth's own corners from a
    /// little short of `width` `0` onward, which is what makes "past the
    /// corners" a thing a style can say by reaching past this.
    pub half: f32,
    /// The edge's width, likewise.
    pub fade: f32,
}

impl Lip {
    /// How deep the band is on the midline, from the vermilion to the
    /// nostrils.
    ///
    /// Floored for the reason [`Ridge::span`](super::brows::Ridge::span) is: a
    /// record may put the two edges together and every caller divides by this.
    #[must_use]
    pub fn span(&self) -> f32 {
        (self.nostrils - self.vermilion).max(super::MINIMUM_SPAN)
    }

    /// The height a share of the way up the band on the midline, `0` at the
    /// vermilion and `1` at the nostrils.
    #[must_use]
    pub fn height(&self, up: f32) -> f32 {
        self.vermilion + self.span() * up
    }

    /// How far out along the lip a point sits: `0` on the midline, `1` at the
    /// outer edge, and more past it.
    ///
    /// Takes a signed offset and answers for whichever side it belongs to, the
    /// two being mirror images — the same convention [`Ridge::along`](super::brows::Ridge::along) keeps, and
    /// for the same reason: every caller has the signed number to hand.
    #[must_use]
    pub fn along(&self, across: f32) -> f32 {
        across.abs() / self.half.max(super::MINIMUM_SPAN)
    }
}

/// The moustache's whole outline on one head, in the face's own plane (#475).
///
/// Head-local metres, `x` across and `y` up, and symmetric: every method takes
/// a signed `x` and answers for its side. The mask ([`Moustache`]'s
/// [`Region`]) and the grown styles both read it, so the floor a whisker
/// sweeps toward is the floor the paint stops at.
///
/// **The lips are a lens round the parting**: the seam the mouth was cut
/// along, dipping toward the corners as `face::relief` carves it, with the
/// upper lip's vermilion standing [`Self::rise`] over it on the midline and
/// thinning to nothing past the corners - an ellipse in the seam's frame,
/// [`Self::reach`] across, [`Self::rise`] up and [`Self::below`] down. The
/// floor of the moustache is that lens's top edge; the ends that run down
/// beside the mouth are kept off it by the same lens's side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Outline {
    /// The midline numbers, as handed out to anyone.
    pub lip: Lip,
    /// The parting's height on the midline.
    pub seam: f32,
    /// How far it has fallen by the corners.
    pub dip: f32,
    /// How far out the cut reaches.
    pub corner: f32,
    /// How far out the lips are kept: the corners and [`CLEAR`] past them.
    pub reach: f32,
    /// How far the floor stands over the parting on the midline: the upper
    /// lip's vermilion, `drop` included.
    pub rise: f32,
    /// How far under the parting the lower lip is kept out.
    pub below: f32,
    /// How far out the ceiling holds before it falls: the face's own nose's
    /// width, and never under the neutral nose's.
    pub nose: f32,
    /// The neutral nose's half-width, [`CHAPLIN`] units: how far out the
    /// density falls to the ends' ([`Self::crowd`]), on every nose.
    pub neutral: f32,
    /// Where the nose's relief has finished below its base
    /// (`Canon::nose_foot`): where the nose's underside starts.
    pub foot: f32,
    /// How far over the foot the patch reaches before it is bare: into the
    /// subnasal crease, short of the underside (see [`CREASE`]).
    pub crease: f32,
    /// Where the nose's underside begins, column by column, where the skull
    /// measured it (`face::skull::Crease`): see [`Self::seen`].
    pub under: Option<crate::face::skull::Crease>,
    /// How low the ends reach beside the mouth.
    pub bottom: f32,
    /// How gently the ceiling's fall comes in, in metres.
    pub ease: f32,
    /// How much the floor holds level at the vermilion's height on the
    /// midline rather than following the lip down to the corners: `1` for a
    /// Chaplin, whose bottom edge is straight, `0` from a classic on.
    pub level: f32,
    /// How many clumps a grown moustache sows for each one of its style's
    /// count, on this outline (see [`Outline::spread_on`]).
    pub spread: f32,
    /// How densely the ends past the lips' keep are sown against the lip,
    /// `0` to `1` (see [`Outline::crowd`]).
    pub ends: f32,
}

impl Outline {
    /// The parting's height at `x`, held at its end past the lens.
    #[must_use]
    pub fn seam_at(&self, x: f32) -> f32 {
        let out = x.abs().min(self.reach) / self.corner.max(MINIMUM_SPAN);
        self.seam - self.dip * out * out
    }

    /// The parting's height at its corners.
    #[must_use]
    pub fn side(&self) -> f32 {
        self.seam - self.dip
    }

    /// How far a point stands outside the lips' lens, in metres: negative
    /// inside it.
    ///
    /// The lens's own field over its gradient, which is exact on the midline
    /// above the lip and straight out from the corners - the two places the
    /// moustache meets it - and first-order everywhere else.
    #[must_use]
    pub fn clear(&self, x: f32, y: f32) -> f32 {
        let up = y - self.seam_at(x);
        let tall = if up >= 0.0 { self.rise } else { self.below }.max(MINIMUM_SPAN);
        let wide = self.reach.max(MINIMUM_SPAN);
        let (u, v) = (x.abs() / wide, up / tall);
        let r = (u * u + v * v).sqrt();
        if r <= f32::EPSILON {
            return -tall.min(wide);
        }
        let slope = ((u / wide).powi(2) + (v / tall).powi(2)).sqrt() / r;
        (r - 1.0) / slope.max(f32::EPSILON)
    }

    /// The lowest a moustache reaches at `x`: the vermilion's top over the
    /// lips, and the ends' bottom past them.
    ///
    /// **Held level toward a Chaplin** ([`Self::level`]): a toothbrush has a
    /// straight bottom edge at the vermilion's height on the midline, so its
    /// sides are as tall as its middle. Only ever raised by that, never
    /// lowered, so it is still clear of the lips.
    #[must_use]
    pub fn floor(&self, x: f32) -> f32 {
        let out = x.abs() / self.reach.max(MINIMUM_SPAN);
        let curve = if out >= 1.0 {
            self.bottom
        } else {
            (self.seam_at(x) + self.rise * (1.0 - out * out).max(0.0).sqrt()).max(self.bottom)
        };
        curve + (self.lip.vermilion - curve).max(0.0) * self.level.clamp(0.0, 1.0)
    }

    /// The middle of an end beside the mouth, how far it reaches either side
    /// of it at a height `y`, and its rounded bottom's height at `x`: the
    /// ends' own edges below the corners' height (#475's fixer).
    ///
    /// Above the corners' height an end is the band's outer part, between the
    /// lips' keep and [`Lip::half`]; below it, it narrows toward its bottom by
    /// [`END_TAPER`] either side of its middle, and its bottom is a half-disc.
    #[must_use]
    pub fn end(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let middle = (self.reach + self.lip.half) * 0.5;
        let full = ((self.lip.half - self.reach) * 0.5).max(0.0);
        let run = self.side() - self.bottom;
        let down = if run > MINIMUM_SPAN {
            ((self.side() - y) / run).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let narrow = full * (1.0 - END_TAPER * down);
        let round = full * (1.0 - END_TAPER * if run > MINIMUM_SPAN { 1.0 } else { 0.0 });
        let off = (x.abs() - middle).clamp(-round, round);
        let bottom = self.bottom + round - (round * round - off * off).max(0.0).sqrt();
        (middle, narrow, bottom)
    }

    /// The highest it reaches at `x`: the nose's foot, held under the nose and
    /// falling outward past its wings.
    ///
    /// **The foot, not the nostrils** (#475): the nose's relief runs down to
    /// [`Canon::nose_foot`] and ends there on every face, at every column of
    /// it, whatever its prominence or width - `face::relief` reads the same
    /// landmark - so the skin between the nostril line and the foot is the
    /// underside of the nose and its wings, which faces the ground and took the
    /// moustache's paint.
    #[must_use]
    pub fn ceiling(&self, x: f32) -> f32 {
        let past = (x.abs() - self.nose).max(0.0);
        self.foot + self.crease - TOP_SLOPE * past * past / (past + self.ease.max(MINIMUM_SPAN))
    }

    /// How much of a point lies beside the mouth below its corners, where a
    /// moustache's hair runs down rather than out: `0` at and above the
    /// corners' height, `1` a fade below it.
    #[must_use]
    pub fn down(&self, at: Vec3) -> f32 {
        crate::face::smooth((self.side() - at.y) / self.lip.fade.max(MINIMUM_SPAN))
    }

    /// How much of a point lies out past the lips' lens, beside the mouth: `0`
    /// over the lips and `1` a fade past their keep.
    #[must_use]
    pub fn beside(&self, x: f32) -> f32 {
        crate::face::smooth((x.abs() - self.reach) / self.lip.fade.max(MINIMUM_SPAN))
    }

    /// How many clumps a grown moustache sows for each one its style asks for
    /// (#475's fixer): what `HairRecord::sowing` multiplies the count by.
    ///
    /// **A region's clump count is a fixed number whatever its area**
    /// (`hair::style`'s `clumps_for`), and the scatter spreads it by weighted
    /// area: so the ends that turn down past the corners and run down beside
    /// the mouth took their roots FROM the lip, and a wide chevron's upper lip
    /// was left with a sixth of the hair it had before the outline - wisps
    /// between two side tufts. So the count is read as two densities:
    ///
    /// - **The lip keeps its clumps per breadth.** The style's count was tuned
    ///   on 0.24.5's level band, [`BREADTH`] units across at `width` `0`; the
    ///   band under the nose is now the lip's own (a third as tall as the old
    ///   band, which ran up the nose's underside), and the lip is furnished
    ///   with that many clumps across that breadth at the height it has.
    /// - **The ends are sown at 0.24.5's density by area**, the count over the
    ///   old band's breadth and the lip's whole span: so a horseshoe's ends
    ///   are paid for on top, at what a moustache always cost a square
    ///   centimetre, and not at the lip's thicker set.
    ///
    /// Integrated over the face's own plane on the measured skull, each sample
    /// weighed by the outline's shape, by [`Self::crowd`] and by how far the
    /// skin slants away from that plane there - an end beside the mouth faces
    /// out as much as forward, and the scatter weighs the skin's own area.
    /// Read once per head, about four thousand samples.
    #[must_use]
    pub fn spread_on(&self, skull: &Skull, unit: f32) -> f32 {
        let fade = self.lip.fade.max(MINIMUM_SPAN);
        let breadth = BREADTH * unit;
        let step = fade * 0.2;
        let top = self.ceiling(0.0) + fade;
        let low = self.bottom.min(self.side() - self.below) - fade;
        let wide = self.lip.half + fade;
        let mut whole = 0.0f32;
        let columns = (wide / step).ceil() as usize;
        let rows = ((top - low) / step).ceil() as usize;
        for column in 0..columns {
            let x = (column as f32 + 0.5) * step;
            for row in 0..rows {
                let y = low + (row as f32 + 0.5) * step;
                let shape = self.shape(x, y);
                if shape <= 0.0 {
                    continue;
                }
                let depth = |y: f32, x: f32| skull.depth_across(y, x);
                let slope_x = (depth(y, x + step) - depth(y, (x - step).max(0.0)))
                    / (x + step - (x - step).max(0.0));
                let slope_y = (depth(y + step, x) - depth(y - step, x)) / (2.0 * step);
                whole += shape
                    * self.crowd(x)
                    * (1.0 + slope_x * slope_x + slope_y * slope_y).sqrt()
                    * step
                    * step;
            }
        }
        // Both sides, over the lip's breadth at the band's height under the
        // nose: the area one style's count furnishes.
        2.0 * whole / (breadth * self.band()).max(MINIMUM_SPAN * MINIMUM_SPAN)
    }

    /// How tall the band under the nose is, as the count reads it: the
    /// patch's height on the midline weighed by its own shape, as the area
    /// [`Self::spread_on`] sets against it is, and never under half a fade.
    ///
    /// **Under the nose's foot** (#475's second fixer): the lip the count is
    /// furnished over is the one under the foot; the crease over it that the
    /// patch now reaches into ([`CREASE`]) is sown at the lip's own density on
    /// top, so a fuller band under the nose is not paid for by thinning it.
    #[must_use]
    pub fn band(&self) -> f32 {
        self.band_with(TOP_FADE, LIP_FADE)
    }

    /// [`Self::band`] read with the top edge faded over `top` of the fade and
    /// the bottom over `lips` of it.
    ///
    /// The ends' density ([`Self::ends`]) is read against the band as the
    /// mask's whole fade at both edges draws it, which is what #475's fixer
    /// set it by: the second fixer's clipped edges make the band under the
    /// nose taller to the eye, and read against that the ends took a third
    /// more of the count than they did (0.28 of the lip's density against
    /// 0.19), which put seed 102's rolled face under a cap 46 triangles over
    /// the hair ceiling (`tests/budget.rs`'s
    /// `the_tier_bites_only_where_a_record_asks_for_more_than_the_budget_holds`).
    #[must_use]
    pub fn band_with(&self, top: f32, lips: f32) -> f32 {
        let fade = self.lip.fade.max(MINIMUM_SPAN);
        let step = fade * 0.05;
        let low = self.floor(0.0) - fade;
        let rows = ((self.ceiling(0.0) + fade - low) / step).ceil() as usize;
        let under = Self {
            crease: 0.0,
            ..*self
        };
        let tall = (0..rows)
            .map(|row| {
                let y = low + (row as f32 + 0.5) * step;
                under.edged(0.0, y, top, lips, under.ceiling(0.0)) * step
            })
            .sum::<f32>();
        tall.max(fade * 0.5)
    }

    /// How densely a point `x` across is sown against the band on the
    /// midline, `0` to `1` (#475's fixer; see [`Self::spread_on`]): `1` on the
    /// midline, falling to [`Self::ends`] by the neutral nose's wings
    /// ([`Self::neutral`]) and held there past them. A moustache is fullest under the nose and thins toward its
    /// ends, in its hair as in its outline; and the lip out toward the corners,
    /// where the floor falls away with the vermilion, is three times as tall
    /// as the band under the nose, so sown at the midline's density it took
    /// most of the count and cost a full head of hair past its budget
    /// (`hair::style`'s `every_region_can_be_grown_and_costs_what_it_should`;
    /// thinned only by the lips' keep, seed 102's rolled face under a cap
    /// still cost 3,334 triangles of hair against the ceiling's 3,300,
    /// `tests/budget.rs`'s `the_tier_bites_only_where_a_record_asks_for_more_than_the_budget_holds`).
    ///
    /// **The neutral nose, not the face's own** (#475's second fixer): read off
    /// the measured nose, a big nose's 28 mm wings stood past the classic's
    /// own half, so the whole moustache was sown at the midline's density and
    /// its count multiplier ran from 1.26 to 2.51 at `width` `0` and from 2.89
    /// to 4.43 at `+1` - about 1,190 more triangles of moustache on a wide
    /// handlebar, which the tier then took from the scalp, brows and chin.
    #[must_use]
    pub fn crowd(&self, x: f32) -> f32 {
        let out = crate::face::smooth(x.abs() / self.neutral.max(self.lip.fade));
        1.0 - (1.0 - self.ends.clamp(0.0, 1.0)) * out
    }

    /// How much of the patch may grow at a point, before its facing is read:
    /// the outline's own edges, every one a smoothstep over the fade.
    fn shape(&self, x: f32, y: f32) -> f32 {
        self.edged(x, y, TOP_FADE, LIP_FADE, self.ceiling(x))
    }

    /// [`Self::shape`] for an asker that knows the skin's facing, whose
    /// ceiling is [`Self::seen`].
    fn shape_seen(&self, x: f32, y: f32) -> f32 {
        self.edged(x, y, TOP_FADE, LIP_FADE, self.seen(x))
    }

    /// The ceiling for an asker that knows the skin's facing: where the
    /// nose's underside begins as the skull measured it, under the nose,
    /// where that is over [`Self::ceiling`] (#475's second fixer).
    ///
    /// **The foot is not where the lip ends on every face**
    /// (`face::skull::Crease`): on seed 237's small nose the lip runs on
    /// 2.1 mm over it, and a ceiling at the foot left a third of that lip
    /// bare. An asker blind to facing cannot tell that lip from the nose's
    /// underside, so it keeps [`Self::ceiling`]; one that knows it is kept off
    /// the underside by [`FACING`] wherever the measured line misses. Eased
    /// back to [`Self::ceiling`] past the nose's wings, so the edge has no
    /// step where the measure ends.
    #[must_use]
    pub fn seen(&self, x: f32) -> f32 {
        let blind = self.ceiling(x);
        let Some(under) = self.under.and_then(|under| under.at(x)) else {
            return blind;
        };
        let held = 1.0 - crate::face::smooth((x.abs() - self.nose) / self.ease.max(MINIMUM_SPAN));
        blind + (under - blind) * held
    }

    /// [`Self::shape`] with its top edge faded over `top` of the fade under
    /// `ceiling`, and its edge along the lips over `lips` of it.
    fn edged(&self, x: f32, y: f32, top: f32, lips: f32, ceiling: f32) -> f32 {
        let fade = self.lip.fade.max(f32::EPSILON);
        let smooth = crate::face::smooth;
        // Off the lips, by the lens: clipped over them, the band's bottom
        // edge, and the lens's whole fade beside the mouth, where an end runs
        // down past the corners.
        let lip_fade = fade * (lips + (1.0 - lips) * self.beside(x));
        let lips = smooth(self.clear(x, y) / lip_fade);
        // The ends' own edges below the corners' height: narrowing toward a
        // rounded bottom, either side of their middle (#475's fixer).
        let (middle, narrow, bottom) = self.end(x, y);
        let below = y < self.side();
        let inner = if below { middle - narrow } else { self.reach };
        let outer = if below {
            middle + narrow
        } else {
            self.lip.half
        };
        // And off the skin under the mouth: below the corners only beside
        // them. A soft union of the two, so the inner edge of an end turns the
        // corner without a crease.
        let beside = smooth((x.abs() - inner.max(self.reach)) / fade);
        let above = smooth((y - self.side()) / fade);
        let off_the_chin = 1.0 - (1.0 - beside) * (1.0 - above);
        // Held level toward a Chaplin, as the floor is (see `Self::floor`),
        // over half the fade: a toothbrush's bottom edge is a clipped one, and
        // the band under the nose is all the lip there is. In place of the
        // lips' own edge rather than as well as it, the two being one edge on
        // the midline; the level floor stands over the lips everywhere, so it
        // keeps them clear on its own.
        let level = self.level.clamp(0.0, 1.0);
        let bottom_edge = lips + (smooth((y - self.floor(x)) / (fade * 0.5)) - lips) * level;
        bottom_edge
            * off_the_chin
            * smooth((ceiling - y) / (fade * top))
            * smooth((y - bottom) / fade)
            * smooth((outer.min(self.lip.half) - x.abs()) / fade)
    }
}

/// The upper lip, cut from one head's landmarks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moustache {
    /// The outline, which the styles grow on and this masks.
    outline: Outline,
}

impl Moustache {
    /// Cuts the region from a measured head.
    ///
    /// The mouth is the parting the skull found where one was cut (see
    /// `face::skull::Parting`): the ends turn down at its corners. Where none
    /// was - a skull measured before the cut - the relief's own mouth at a
    /// neutral width stands in.
    #[must_use]
    pub(super) fn of(skull: &Skull, canon: &Canon, params: &Params, pad: &Pad) -> Self {
        let mouth = canon.mouth_line();
        let nose = canon.nose_base();
        // Down toward the mouth as `drop` rises, so `+1` reaches the vermilion.
        let vermilion = mouth + (nose - mouth) * (VERMILION - params.drop * DROP_RANGE);
        let nostrils = nose - (nose - mouth) * UNDER_NOSE;
        let (seam, corner, side) = match skull.parting() {
            Some(parting) => (parting.middle, parting.corner, parting.side),
            None => {
                let (corner, dip) = crate::face::mouth::corners(canon, &FaceParams::default());
                (mouth, corner, mouth - dip)
            }
        };
        let fade = FADE * canon.frame;
        let reach = corner + CLEAR * canon.unit;
        let width = params.width.clamp(-1.0, 1.0);
        // How far out, and how far down beside the mouth: a Chaplin's half at
        // `-1`, ends as thick as the band under the nose past the lips' keep
        // at `0` with the ends at the corners' height, and past the corners by
        // `ENDS_WIDE` at `+1` with the ends on the chin - below the chin
        // patch's own top, so the two meet.
        // The face's own nose where the skull found its underside, its edge
        // half faded at the nostrils' wings; the canon's neutral nose where
        // it found none.
        let wings = skull
            .nose_under()
            .map_or(canon.unit * CHAPLIN, |wings| wings + fade * 0.5);
        // And never past the mouth's own corners (#475's second fixer): a big
        // nose's underside reaches 28 mm out on a 22 mm half-mouth, and a
        // Chaplin that wide ran past the corners and past the classic's own
        // ends, so the slider narrowed on its way from `-1` to `0`.
        let narrow = wings.min(corner);
        let foot = canon
            .nose_foot()
            .min(nostrils)
            .max(vermilion + canon.frame * 0.02);
        // An end as thick as the band under the nose, and never under
        // `END_FADES` (see `ENDS_WIDE`).
        let classic = reach + (foot - vermilion).max(END_FADES * fade);
        let wide = (corner + canon.unit * ENDS_WIDE).max(classic);
        let half = if width <= 0.0 {
            narrow + (classic - narrow) * (width + 1.0)
        } else {
            classic + (wide - classic) * width
        };
        let chin = canon.chin();
        let joined = (chin + (mouth - chin) * CHIN_JOIN).min(pad.lip - 2.0 * pad.fade);
        let bottom = side + (joined - side) * width.max(0.0);
        let rise = (vermilion - seam).max(MINIMUM_SPAN);
        let mut moustache = Self {
            outline: Outline {
                lip: Lip {
                    vermilion,
                    // Ordered for the same reason the brow's ends are: the two
                    // edges move on different axes and a record may cross them.
                    nostrils: nostrils.max(vermilion + canon.frame * 0.02),
                    half,
                    fade,
                },
                seam,
                dip: (seam - side).max(0.0),
                corner,
                reach,
                rise,
                below: rise * BELOW,
                // Never narrower than the neutral nose: a narrow nose's
                // Chaplin is narrow, but the top of a wider moustache does
                // not start to fall at its wings, which would cut the band
                // short of its own ends.
                nose: wings.max(canon.unit * CHAPLIN),
                neutral: canon.unit * CHAPLIN,
                foot,
                crease: canon.frame * CREASE,
                under: skull.crease(),
                bottom,
                ease: canon.unit * TOP_EASE,
                level: (-width).max(0.0),
                spread: 1.0,
                ends: 1.0,
            },
        };
        // The ends at 0.24.5's density by area: the lip's band against the
        // whole span the old band was tall (see `Outline::spread_on`).
        let outline = &mut moustache.outline;
        outline.ends = (outline.band_with(1.0, 1.0) / outline.lip.span()).min(1.0);
        outline.spread = outline.spread_on(skull, canon.unit);
        moustache
    }

    /// The patch this mask is cut around, as its midline numbers.
    #[must_use]
    pub(super) fn lip(&self) -> Lip {
        self.outline.lip
    }

    /// The whole outline.
    #[must_use]
    pub(super) fn outline(&self) -> Outline {
        self.outline
    }
}

impl Region for Moustache {
    fn weight(&self, at: &At) -> f32 {
        let outline = &self.outline;
        // Up to where the nose's underside begins where the asker knows the
        // skin's facing, and to the blind ceiling where it does not.
        let edged = match at.normal {
            Some(_) => outline.shape_seen(at.across, at.height),
            None => outline.shape(at.across, at.height),
        };
        let shaped = edged * crate::face::smooth((at.forward - FRONT) / 0.30);
        // The nose's underside, by its facing, where the asker knows the skin's
        // normal: only above the vermilion, so the lip's own underside and the
        // cheek beside the mouth are never asked.
        match at.normal {
            Some(normal) if shaped > 0.0 => {
                let above = crate::face::smooth(
                    (at.height - outline.lip.vermilion) / outline.lip.fade.max(f32::EPSILON),
                );
                let nose = 1.0 - ((normal.y - FACING) / FACING_FADE).clamp(0.0, 1.0);
                shaped * (1.0 - above * nose)
            }
            _ => shaped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::skull::HeadTraits;
    use crate::hair::{Follicle, FollicleParams, Follicles};
    use crate::{Archetype, Avatar, AvatarRecord};

    /// One built head and its regions with the moustache at `width`.
    struct Head {
        follicles: Follicles,
        canon: Canon,
        record: AvatarRecord,
    }

    fn head(seed: Option<i64>, width: f32) -> Head {
        let mut record = AvatarRecord::new("Moustache", Archetype::default());
        if let Some(seed) = seed {
            record.reroll(seed);
        }
        let avatar = Avatar::build_for_shape(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let mut params = FollicleParams::default();
        params.moustache.width = width;
        Head {
            follicles: Follicles::of(&avatar.rig, &skull, &canon, &params),
            canon,
            record,
        }
    }

    impl Head {
        /// The head-local point on the measured surface at `x` across and `y`
        /// up.
        fn on(&self, x: f32, y: f32) -> Vec3 {
            Vec3::new(x, y, self.follicles.skull().depth_across(y, x.abs()))
        }

        fn weight(&self, follicle: Follicle, x: f32, y: f32) -> f32 {
            self.follicles.weight(follicle, self.on(x, y))
        }
    }

    /// The heads every guard here reads: the default and two rolls with
    /// different mouths, 0.31 and 0.55 of the mouth-width axis.
    const SEEDS: [Option<i64>; 3] = [None, Some(175), Some(237)];

    #[test]
    fn a_moustache_s_ends_turn_down_at_the_corners_and_run_to_the_chin() {
        // **The owner's report, #475**: the moustache was a straight bar that
        // never bent round the mouth, and its width only slid the bar's ends.
        // At `0` the ends turn down to the mouth's corners; at `+1` they run
        // down beside the mouth to the chin, and with no chin beard still stop
        // on it (the horseshoe) - so down the middle of an end, from the
        // corners' height to the chin patch's own top, the moustache alone
        // comes fully on below the corners and is still at half weight where
        // the chin's patch begins, and with the chin's it leaves no gap.
        //
        // Measured: 1.00 at every one of these readings on all three heads.
        // Before #475 the patch was cut at the vermilion by height alone: 0.00
        // under it at every width, down to the chin's top.
        for seed in SEEDS {
            let classic = head(seed, 0.0);
            let outline = classic.follicles.outline();
            let x = (outline.reach + outline.lip.half) * 0.5;
            let below = outline.lip.vermilion - outline.lip.fade;
            let turned = classic.weight(Follicle::Moustache, x, below);
            assert!(
                turned >= 0.9,
                "seed {seed:?}: a classic moustache stands {turned:.2} at {:.1} mm out a fade under \
                 the vermilion, so its ends do not turn down",
                x * 1000.0
            );
            let wide = head(seed, 1.0);
            let outline = wide.follicles.outline();
            let pad = wide.follicles.pad();
            let x = (outline.reach + outline.lip.half) * 0.5;
            let half_way = outline.side() + (pad.lip - outline.side()) * 0.5;
            let down = wide.weight(Follicle::Moustache, x, half_way);
            let at_chin = wide.weight(Follicle::Moustache, x, pad.lip);
            let mut thinnest = f32::MAX;
            for step in 0..=100 {
                let y = outline.side() + (pad.lip - outline.side()) * step as f32 / 100.0;
                thinnest = thinnest.min(
                    wide.weight(Follicle::Moustache, x, y) + wide.weight(Follicle::Chin, x, y),
                );
            }
            println!(
                "seed {seed:?}: classic {turned:.2}; wide {down:.2} half way, {at_chin:.2} at the \
                 chin's top, {thinnest:.2} thinnest with the chin"
            );
            assert!(
                down >= 0.9 && at_chin >= 0.5 && thinnest >= 0.8,
                "seed {seed:?}: a wide moustache's end stands {down:.2} half way down beside the \
                 mouth and {at_chin:.2} at the chin's top, and {thinnest:.2} with the chin's at \
                 its thinnest"
            );
        }
    }

    #[test]
    fn a_chaplin_is_as_wide_as_the_nose_and_stops_under_it() {
        // **`width` `-1` is a Chaplin** (#475, the owner's decision): a patch
        // about as wide as the nose's base, the full height from under the
        // nose to the vermilion - and nothing on the nose itself. Read across
        // the middle of the cutaneous lip, where it is at half weight, against
        // the relief's own wings; and just above the nose's foot, where the
        // nose's underside is.
        //
        // **On the face's own nose** since #475's fixer: the patch's edge is
        // half faded where the skull found the nose's underside to end
        // (`Skull::nose_under`), so seed 237's narrow nose (0.04 of its width
        // axis), which wore a canon-width Chaplin 1.70 of its relief wings
        // wide, wears one 0.84 of them - the carved underside's own reach,
        // which `SEEDS` below holds to its height and its bare nose.
        //
        // Measured: half weight at 0.93 and 0.89 of the wings' reach, 0.96 on
        // the midline, 0.00 above the foot. Before #475 the patch ran to the
        // nostril line, and a millimetre over the foot stood at 1.00.
        for seed in [None, Some(175)] {
            let chaplin = head(seed, -1.0);
            let outline = chaplin.follicles.outline();
            let face = chaplin
                .record
                .face
                .on(&HeadTraits::of(&chaplin.record.composites));
            let wings = crate::face::relief::wings(&chaplin.canon, &face);
            let middle = (outline.floor(0.0) + outline.ceiling(0.0)) * 0.5;
            let full = chaplin.weight(Follicle::Moustache, 0.0, middle);
            let reach = (0..400)
                .map(|step| step as f32 * 0.0001)
                .take_while(|&x| chaplin.weight(Follicle::Moustache, x, middle) >= 0.5)
                .last()
                .unwrap_or(0.0);
            let above = [0.0f32, 0.5, 0.9]
                .iter()
                .map(|share| {
                    chaplin.weight(
                        Follicle::Moustache,
                        wings * share,
                        chaplin.canon.nose_foot() + 0.001,
                    )
                })
                .fold(0.0f32, f32::max);
            println!(
                "seed {seed:?}: a Chaplin is {:.2} of the nose's wings wide, {full:.2} on the \
                 midline, {above:.2} over the nose's foot",
                reach / wings
            );
            assert!(
                (0.85..=1.15).contains(&(reach / wings)) && full >= 0.8 && above <= 0.0,
                "seed {seed:?}: a Chaplin reaches {:.2} of the nose's wings, stands {full:.2} on \
                 the midline and {above:.2} over the nose's foot",
                reach / wings
            );
        }
        for seed in SEEDS {
            let chaplin = head(seed, -1.0);
            let outline = chaplin.follicles.outline();
            let middle = (outline.floor(0.0) + outline.ceiling(0.0)) * 0.5;
            let full = chaplin.weight(Follicle::Moustache, 0.0, middle);
            let above = chaplin.weight(Follicle::Moustache, 0.0, chaplin.canon.nose_foot() + 0.001);
            assert!(
                full >= 0.8 && above <= 0.0,
                "seed {seed:?}: a Chaplin stands {full:.2} on the midline and {above:.2} over the \
                 nose's foot"
            );
        }
    }
}
