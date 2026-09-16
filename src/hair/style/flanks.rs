//! The jaw flanks' own catalogue, and the last of the five.
//!
//! # The region whose edges are somebody else's
//!
//! Every other region draws its own boundary. This one has three it does not
//! own: the shaved beard line above it, which the mask cuts
//! and [`Line::top`] carries; the mandible's crease below it, which
//! [`crate::face::skull`] carves and which must not be copied; and the
//! chin's own patch beside it, which this has to meet
//! without a stripe of bare jaw between them.
//!
//! So the styles here are shorter on invention than the other four and longer
//! on reading. A flank clump combs DOWN the cheek — downhill on the side of a
//! face is very nearly straight down, which is the one place the engine's own
//! default direction is simply right — and it stops where the beard stops
//! rather than where its own length runs out. That is what makes the edge a
//! LINE: the tips arrive at the crease together, from wherever they grew.
//!
//! [`FlankStyle::Sculpted`] draws the same band as one closed solid a side
//! (`hair::shell::face`, #349), from the beard line to the crease - and on in
//! under the jaw where the chin's patch is, beneath a sculpted chin's solid, so
//! the seam between the two is covered.
//!
use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, flanks::Line};
use super::super::shell::face::{Sculpt, Sculpted};
use super::{Cut, SCULPTED_PAINT, Style, clumps_for};
use crate::plan::scaled;

/// The base styles of the jaw's flanks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum FlankStyle {
    /// Nothing is grown here: the flank is painted, or shaved.
    #[default]
    None,
    /// A strip from the ear down toward the jaw's corner, and no cheek.
    Sideburns {
        /// How far down the strip runs, `0` a tab beside the ear and `1` down
        /// to the jaw's corner.
        #[serde(with = "crate::plan::scaled")]
        drop: f32,
    },
    /// The whole flank, carried forward to meet the chin's own patch.
    FullConnect {
        /// How far below the jawline the beard rides, `0` stopping on the
        /// crease and `1` down onto the upper neck.
        #[serde(with = "crate::plan::scaled")]
        reach: f32,
    },
    /// A closed sculpted band over each flank from the beard line to the
    /// mandible's crease, meeting a sculpted chin under the jaw (#349). No axis
    /// of its own.
    Sculpted,
}

impl FlankStyle {
    /// Every name this build writes, in declaration order.
    ///
    /// **What a reader checks a name against before it trusts one** (#351). A
    /// record from a newer build can name a style this one has never heard
    /// of, and [`crate::hair::HairRecord`] reads such a name as `none` and
    /// keeps it for the rewrite rather than refusing the whole avatar.
    /// `tests/lexicon.rs` holds this list to the lexicon's knownValues.
    pub const NAMES: &'static [&'static str] = &["none", "sideburns", "full_connect", "sculpted"];
}

/// How far one clump combs down the flank at full length, in metres.
///
/// One entry per style, in the order the enum declares them.
///
/// Short, and shorter than any other region's: beard hair on a cheek lies on the
/// skin and the mass comes from how many there are, not from how far each one
/// runs. A clump long enough to cross the whole flank would leave the surface
/// at the jaw and hang in the air, which is what the chin's own catalogue is for.
///
/// Provenance: **tuned by render**.
const REACH: [f32; 2] = [0.020, 0.026];

/// How wide one is at the root at the coarsest cut, in metres.
///
/// **The widest cards in the catalogue, because this is the widest region**
///. The flanks hold 7.5% of a head's own surface — measured by
/// `follicleaudit`, against the chin's 4.3 and the moustache's 0.5 — so the same
/// count of the same cards covers a fraction of what it covers anywhere else.
/// At half this the sheet read as a field of dark blobs with light between them,
/// and the arithmetic says why: fifty-eight cards a side over five thousand
/// square millimetres is one and a half times cover, and a random one and a half
/// leaves a fifth of it bare.
///
/// Width is free and count is four triangles a time, so the answer is width.
///
/// Provenance: **derived** from the region's measured area, **tuned by render**
///.
const WIDTH: [f32; 2] = [0.0165, 0.0210];

/// What share of that is left at the tip.
///
/// Provenance: **tuned by render**.
const TAPER: [f32; 2] = [0.30, 0.26];

/// How much a clump lies along the skin where it leaves it.
///
/// Flatter than a beard's and flatter than a moustache's: hair on a cheek is
/// pressed against the face, and standing it off reads as a fur collar.
///
/// Provenance: **tuned by render**.
const LIE: [f32; 2] = [0.94, 0.92];

/// How many clumps each style asks for at full density, as a share of the shared
/// count.
///
/// **Sideburns ask for a third**, because a sideburn is a strip and the shared
/// count is sized for a whole flank — and because the roots it declines are
/// roots the budget gets back rather than clumps drawn somewhere they should not
/// be.
///
/// Provenance: **derived** from what each style is.
const CROWD: [f32; 2] = [0.34, 1.0];

/// How far forward of the ear a sideburn reaches, in the azimuth's cosine.
///
/// Sideburns are the region BEHIND this; the full connection is everything. A
/// cosine rather than a distance for the reason the mask's own edges are: it is
/// what a boundary that runs round a head is written in, and it changes smoothly
/// everywhere the region reaches.
///
/// Provenance: **derived** from the mask's own `DIAGONAL`, which is where the
/// beard line has finished dropping to the cheek — a sideburn stops well before
/// that, on the flat of the flank.
const BURNS_FRONT: f32 = 0.28;

/// How softly that edge comes on, in the same cosine.
///
/// Provenance: **tuned by render**.
const BURNS_FADE: f32 = 0.16;

/// How far down its own reach a sideburn runs at each end of its own axis.
///
/// As a share of the way from the beard line to the jawline, so a sideburn ends
/// where a sideburn ends on a face of any length rather than at a fixed depth.
///
/// Provenance: **tuned by render**, against the anatomy it is named for.
const BURNS_DOWN: [f32; 2] = [0.30, 1.0];

/// How far below the jawline a full connection may ride, in metres, at each end
/// of its own axis.
///
/// **Under it, not to it, and the mask says the same**: beard growth
/// crosses the jawline and stops on the upper neck, and a beard that ended on
/// the crease would draw a bright line down an edge the skull works to keep
/// smooth.
///
/// Provenance: **tuned by render**, inside the mask's own reach below the
/// border.
const RIDES: [f32; 2] = [0.002, 0.014];

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Provenance: **carried** from [`brows`](super::brows).
const LEAST_WORTH: f32 = 0.08;

/// How thin a clump is at each of its ends, as a share of its middle.
///
/// A leaf rather than a wedge, so a row of overlapping clumps has no ends in it.
///
/// **Blunter than the other four regions, and the render is why**. A leaf
/// pinched to a third of its width tiles a patch like a lattice of lenses: the
/// cards overlap three deep and the flank still read as a field of dark diamonds
/// with light between them, because what was between them was the gaps the
/// pinched ends left. A row of streaks along a brow wants a fine end; a sheet
/// covering a cheek wants a blunt one.
///
/// Provenance: **carried** from [`brows`](super::brows), **re-tuned by
/// render** for a region that tiles rather than rows.
const ENDS: f32 = 0.58;

/// Whether a sideburn is a strip from the beard line down to its drop.
///
/// **A sideburn was two dashes** (#344): its clumps comb down the flank by the
/// flanks' own short reach and stop part-way to the jaw, so the few that are not
/// declined as too short are tabs beside the ear. A strip is every one of them
/// running the whole height, gathered into a band in front of the ear, so three
/// or four overlap across it.
///
/// Provenance: **the owner's brief** for #344.
const STRIP: bool = true;

/// Where a sideburn strip's middle sits round the head, in the azimuth's cosine.
///
/// Between the region's back edge, which is behind the ear, and
/// [`BURNS_FRONT`]. At a tenth the strip lay over the ear's own front edge.
///
/// Provenance: **tuned by render** (#344).
const STRIP_AT: f32 = 0.20;

/// How far either side of that middle a strip's cards are spread, in metres
/// along the skin.
///
/// Provenance: **tuned by render** (#344).
const STRIP_HALF: f32 = 0.010;

/// How wide one of a strip's cards is at the root, in metres, before the cut's
/// own coarseness.
///
/// Provenance: **tuned by render** (#344), against [`STRIP_HALF`]: three or four
/// of them overlap across the band.
const STRIP_WIDTH: f32 = 0.013;

/// How far below the beard line a strip card's top may start, in metres.
///
/// Staggered by the card's own salt, so the strip's top is a ragged edge of
/// several ends rather than one ruled line.
///
/// Provenance: **tuned by render** (#344).
const STRIP_STAGGER: f32 = 0.006;

/// What share of a full connection's clumps run along the jawline instead of
/// down the cheek.
///
/// **The flanks and the chin met under the jaw with bare skin between**
/// (#344): every flank clump combs down and stops ON the jawline, which draws
/// the line and nothing past it, and the chin's own corners give up their hang.
/// A row of cards lying along the mandible's border, from wherever each is
/// rooted toward the chin, crosses that edge and carries the flank into the
/// chin's patch.
///
/// Provenance: **the owner's brief** for #344, **tuned by render**.
const JAW_ROW: f32 = 0.20;

/// How far one of those runs along the jaw at full length, in metres.
///
/// Provenance: **tuned by render** (#344).
const JAW_RUN: f32 = 0.034;

/// How wide one is at the root, in metres, before the cut's own coarseness.
///
/// **Narrow, because it lies across a crease.** A card is a tangent plane, so
/// its edges stand off a curve of radius `R` by `w^2 / 2R`: at 6 mm a side over
/// a border some 10 mm round, under 2 mm - where the flanks' own 21 mm cards
/// would stand 5 mm off it.
///
/// Provenance: **derived** from that standoff, **tuned by render** (#344).
const JAW_WIDTH: f32 = 0.012;

/// How far under the jawline a jaw card's spine rides, as a share of the
/// style's own ride under it.
///
/// Provenance: **tuned by render** (#344).
const JAW_UNDER: f32 = 0.5;

/// How far forward a jaw card may run, in the azimuth's cosine: the chin's
/// own patch is past it.
///
/// Provenance: **tuned by render** (#344).
const JAW_FRONT: f32 = 0.97;

/// The salt lane a full connection's roles are drawn from.
///
/// Provenance: **derived**: any lane no other draw in this file uses.
const ROLE_SALT: u32 = 5;

/// The salt lane a strip card's staggered top is drawn from.
///
/// Provenance: **derived**, likewise.
const STAGGER_SALT: u32 = 6;

impl Style for FlankStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        if matches!(self, Self::Sculpted) {
            return Some(Box::new(Sculpted(Sculpt::Flanks)));
        }
        let slot = self.slot()?;
        let length = 0.6 + 0.8 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        let (front, down, rides) = match self {
            Self::None | Self::Sculpted => return None,
            Self::Sideburns { drop } => {
                let drop = drop.clamp(0.0, 1.0);
                (
                    BURNS_FRONT,
                    BURNS_DOWN[0] + (BURNS_DOWN[1] - BURNS_DOWN[0]) * drop,
                    RIDES[0],
                )
            }
            // Forward of anything a head has, so nothing is declined: a full
            // connection is the style with no front edge of its own.
            Self::FullConnect { reach } => (
                1.5,
                1.0,
                RIDES[0] + (RIDES[1] - RIDES[0]) * reach.clamp(0.0, 1.0),
            ),
        };
        Some(Box::new(Flank {
            line: head.beard_line(),
            regions: head.clone(),
            reach: REACH[slot] * length,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            lie: LIE[slot],
            front,
            down,
            rides,
            strip: STRIP && matches!(self, Self::Sideburns { .. }),
            connect: matches!(self, Self::FullConnect { .. }),
            stretch: length,
            coarse,
        }))
    }

    fn clumps(&self, cut: &Cut, follicle: Follicle) -> usize {
        let Some(slot) = self.slot() else {
            return 0;
        };
        ((clumps_for(cut, follicle) as f32) * CROWD[slot]).round() as usize
    }

    fn sanitize(&mut self) {
        match self {
            Self::None | Self::Sculpted => {}
            Self::Sideburns { drop } => *drop = scaled::quantize(drop.clamp(0.0, 1.0)),
            Self::FullConnect { reach } => *reach = scaled::quantize(reach.clamp(0.0, 1.0)),
        }
    }

    fn paint_floor(&self) -> Option<f32> {
        matches!(self, Self::Sculpted).then_some(SCULPTED_PAINT)
    }
}

impl FlankStyle {
    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None | Self::Sculpted => None,
            Self::Sideburns { .. } => Some(0),
            Self::FullConnect { .. } => Some(1),
        }
    }
}

/// One clump of flank beard: it combs down the cheek and stops at the crease.
#[derive(Clone, Debug, PartialEq)]
struct Flank {
    /// The beard line above it.
    line: Line,
    /// The measured head, for the jawline under it.
    ///
    /// Cloned for the reason the scalp's is: a [`Shape`] outlives the call that
    /// built it, and the mandible's border is a function of azimuth rather than
    /// a number that could be carried instead.
    regions: Follicles,
    /// How far one clump combs down at full length, in metres.
    reach: f32,
    /// How wide one is at the root, in metres.
    width: f32,
    /// What share of the section is left at the tip.
    taper: f32,
    /// How flat against the skin it lies.
    lie: f32,
    /// How far forward the style reaches, in the azimuth's cosine.
    front: f32,
    /// How far down its own span the style runs, as a share.
    down: f32,
    /// How far below the jawline it may ride, in metres.
    rides: f32,
    /// Whether this is a sideburn drawn as a strip. See [`STRIP`].
    strip: bool,
    /// Whether this is a full connection, whose roots take roles. See
    /// [`JAW_ROW`].
    connect: bool,
    /// The cut's own length factor, which a jaw card's run scales by.
    stretch: f32,
    /// The cut's own coarseness, which a jaw or strip card's width scales by.
    coarse: f32,
}

/// What one root of a flank grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// A clump combing down the cheek from where it is rooted.
    Comb,
    /// A card along the mandible's border. See [`JAW_ROW`].
    Jaw,
    /// A card down a sideburn strip. See [`STRIP`].
    Strip,
}

impl Flank {
    /// What this root grows.
    fn role(&self, root: &Root) -> Role {
        if self.strip {
            return Role::Strip;
        }
        if !self.connect {
            return Role::Comb;
        }
        if super::salt(root, ROLE_SALT) < JAW_ROW {
            Role::Jaw
        } else {
            Role::Comb
        }
    }

    /// The skull's own surface at a height and an azimuth, head-local.
    fn skin(&self, height: f32, azimuth: f32) -> Vec3 {
        self.regions.skull().surface_at(height, azimuth)
    }

    /// Which way that surface faces, out of the head.
    ///
    /// Across the surface's two directions, round and up, which on either side
    /// of the head is outward by the order they are taken in.
    fn normal(&self, height: f32, azimuth: f32) -> Vec3 {
        const STEP: f32 = 0.001;
        let round = self.skin(height, azimuth + STEP) - self.skin(height, azimuth - STEP);
        let up = self.skin(height + STEP, azimuth) - self.skin(height - STEP, azimuth);
        let at = self.skin(height, azimuth);
        round
            .cross(up)
            .normalize_or(Vec3::new(at.x, 0.0, at.z).normalize_or(Vec3::Z))
    }

    /// Where a jaw or strip card's station sits: its height and signed azimuth.
    fn station(&self, root: &Root, along: f32) -> (f32, f32) {
        let along = along.clamp(0.0, 1.0);
        let side = if root.at.x < 0.0 { -1.0 } else { 1.0 };
        let from = root.at.x.atan2(root.at.z).abs();
        let reach = (root.at.x * root.at.x + root.at.z * root.at.z)
            .sqrt()
            .max(0.02);
        if self.role(root) == Role::Jaw {
            // Toward the chin, or away from it for a root already in front of
            // where the row stops.
            let turn = JAW_RUN * self.stretch / reach;
            let front = JAW_FRONT.clamp(-1.0, 1.0).acos();
            let to = if from - turn >= front {
                from - turn
            } else {
                from + turn
            };
            let azimuth = side * (from + (to - from) * along);
            (
                self.regions.jawline(azimuth.cos()) - self.rides * JAW_UNDER,
                azimuth,
            )
        } else {
            let (azimuth, top, bottom) = self.strip_span(root);
            (top + (bottom - top) * along, azimuth)
        }
    }

    /// A strip card's signed azimuth, its top and its bottom, in head-local
    /// metres.
    fn strip_span(&self, root: &Root) -> (f32, f32, f32) {
        let side = if root.at.x < 0.0 { -1.0 } else { 1.0 };
        let from = root.at.x.atan2(root.at.z).abs();
        let reach = (root.at.x * root.at.x + root.at.z * root.at.z)
            .sqrt()
            .max(0.02);
        let middle = STRIP_AT.clamp(-1.0, 1.0).acos();
        let azimuth =
            side * (middle + ((from - middle) * reach).clamp(-STRIP_HALF, STRIP_HALF) / reach);
        let facing = azimuth.cos();
        // A fade under the line, where the mask is whole: started ON the line,
        // a strip's top stood where its own region's weight is nothing.
        let top = self.line.top(facing)
            - self.line.fade
            - STRIP_STAGGER * super::salt(root, STAGGER_SALT);
        (azimuth, top, self.floor_at(facing))
    }

    /// How long a combing clump from this root is.
    fn comb_length(&self, root: &Root) -> f32 {
        let share = self.share(root);
        // Never past the crease: the tips arrive at the line together, whatever
        // height they grew from, which is what makes a beard's edge an edge.
        let room = (root.at.y - self.floor(root)).max(0.0);
        let length = (self.reach * share).min(room);
        if length < self.reach * LEAST_WORTH {
            return 0.0;
        }
        length
    }

    /// A point on a combing clump from this root.
    fn comb_at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let travel = self.comb_length(root) * along;
        root.at + root.out * LIFT + self.run(root) * travel
    }

    /// How far round the head a root sits, as a cosine of its azimuth.
    ///
    /// The same reading the mask takes, so a style and the paint under it agree
    /// about which part of a flank a point is on.
    fn facing(root: &Root) -> f32 {
        let reach = (root.at.x * root.at.x + root.at.z * root.at.z).sqrt();
        if reach > f32::EPSILON {
            root.at.z / reach
        } else {
            1.0
        }
    }

    /// How much of the style this root grows, `0` to `1`.
    ///
    /// The mask's own weight, and the style's own back edge: a sideburn declines
    /// everything forward of it rather than growing a short one there, because a
    /// short clump on a cheek is a stray hair and a declined root is triangles
    /// back.
    fn share(&self, root: &Root) -> f32 {
        let ahead = crate::face::smooth((self.front - Self::facing(root)) / BURNS_FADE);
        root.weight.clamp(0.0, 1.0).sqrt() * ahead
    }

    /// The lowest this clump's tip may reach, in head-local metres.
    ///
    /// **The mandible's own carved border, less however far the style rides
    /// under it.** The beard's edge and the crease are one line by construction,
    /// which is the whole of why this file reads [`Follicles::jawline`] rather
    /// than carrying a copy — and it is what makes the tips arrive together and
    /// read as an edge rather than as a fringe of whatever length each clump
    /// happened to have.
    fn floor(&self, root: &Root) -> f32 {
        self.floor_at(Self::facing(root))
    }

    /// The same at one azimuth's cosine.
    fn floor_at(&self, facing: f32) -> f32 {
        let border = self.regions.jawline(facing) - self.rides;
        // A sideburn stops part of the way down instead, on its own axis.
        let top = self.line.top(facing);
        border + (top - border) * (1.0 - self.down.clamp(0.0, 1.0))
    }

    /// Which way this clump combs: down the flank, along the skin.
    ///
    /// Downhill, which on the side of a face is very nearly straight down — the
    /// one region in this catalogue where the engine's own default direction is
    /// simply right. Under a jaw it points backward into the throat and
    /// on a brow ridge it points into the eye; on a cheek it points where
    /// a beard grows.
    fn run(&self, root: &Root) -> Vec3 {
        let down = Vec3::NEG_Y;
        let flow = (down - root.out * down.dot(root.out)).normalize_or(root.out);
        root.out
            .lerp(flow, self.lie.clamp(0.0, 1.0))
            .normalize_or(root.out)
    }
}

impl Shape for Flank {
    fn length(&self, root: &Root) -> f32 {
        match self.role(root) {
            Role::Comb => self.comb_length(root),
            // A card along the jaw or down a strip is as long as its run, and
            // declined only where the style declines the root at all.
            Role::Jaw | Role::Strip if self.share(root) < LEAST_WORTH => 0.0,
            Role::Jaw => JAW_RUN * self.stretch,
            Role::Strip => {
                let (_, top, bottom) = self.strip_span(root);
                let length = top - bottom;
                if length < self.reach * LEAST_WORTH {
                    return 0.0;
                }
                length
            }
        }
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        match self.role(root) {
            Role::Comb => self.comb_at(root, along),
            // On the skull's own surface station by station, because a card
            // this long on a face this curved would cut a chord into it.
            Role::Jaw | Role::Strip => {
                let (height, azimuth) = self.station(root, along);
                self.skin(height, azimuth) + self.normal(height, azimuth) * LIFT
            }
        }
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`].
        let (base, _) = self.width(root);
        let from_middle = ((along.clamp(0.0, 1.0) - 0.5) / 0.5).abs().min(1.0);
        base * (1.0 - (1.0 - ENDS) * from_middle * from_middle)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let full = match self.role(root) {
            Role::Comb => self.width,
            Role::Jaw => JAW_WIDTH * self.coarse,
            Role::Strip => STRIP_WIDTH * self.coarse,
        };
        let base = full * 0.5 * self.share(root);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // The card lies IN the plane of the skin, its width across the comb.
        root.out.cross(self.run(root))
    }

    fn across_at(&self, root: &Root, along: f32) -> Vec3 {
        match self.role(root) {
            Role::Comb => self.across(root),
            // In the plane of the skin at this station, across the card's own
            // run: over the jaw's border that is up and down the crease.
            Role::Jaw | Role::Strip => {
                const STEP: f32 = 0.02;
                let (height, azimuth) = self.station(root, along);
                let tangent =
                    self.at(root, (along + STEP).min(1.0)) - self.at(root, (along - STEP).max(0.0));
                self.normal(height, azimuth).cross(tangent)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull};
    use crate::hair::follicle::FollicleParams;
    use crate::{Archetype, Avatar, AvatarRecord};

    /// The regions of one built head, which a style is fitted against.
    fn head() -> Follicles {
        let record = AvatarRecord::new("Flanks", Archetype::default());
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default())
    }

    /// A root on the flank, a share of the way round from the front and down
    /// from the beard line to the jawline.
    fn root(head: &Follicles, facing: f32, down: f32) -> Root {
        let line = head.beard_line();
        let top = line.top(facing);
        let border = head.jawline(facing);
        let height = top + (border - top) * down;
        let azimuth = facing.clamp(-1.0, 1.0).acos();
        let at = head.skull().surface_at(height, azimuth);
        Root {
            at,
            // The side of a face: the skin looks outward and a little forward.
            out: Vec3::new(at.x, 0.0, at.z).normalize_or(Vec3::X),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    /// Both styles, at a middling axis.
    fn every_style() -> [FlankStyle; 2] {
        [
            FlankStyle::Sideburns { drop: 0.6 },
            FlankStyle::FullConnect { reach: 0.5 },
        ]
    }

    #[test]
    fn no_clump_reaches_past_the_jawline_it_is_edged_on() {
        // **The claim the whole file is arranged around** (#208, #196's lesson
        // in its geometric form). The beard's lower edge and the mandible's
        // carved crease are one line, so a clump may come down to it and no
        // further — whatever height it grew from and whatever the style's own
        // reach happens to be. Hair that ran past would draw a ragged second
        // line under the one #195 spent a day making smooth.
        let head = head();
        let mut measured = 0usize;
        for style in every_style() {
            let cut = Cut {
                length: 1.0,
                thickness: 1.0,
                density: 1.0,
                droop: 1.0,
            };
            let shape = style
                .shape(&cut, Follicle::Flanks, &head)
                .expect("a grown style has a shape");
            for facing in [-0.2f32, 0.1, 0.4, 0.7, 0.9] {
                for down in [0.0f32, 0.35, 0.7, 0.95] {
                    let root = root(&head, facing, down);
                    if shape.length(&root) <= 0.0 {
                        continue;
                    }
                    measured += 1;
                    for step in 0..=12 {
                        let at = shape.at(&root, step as f32 / 12.0);
                        // At the station's own azimuth, because the jawline is a
                        // function of azimuth and a card that runs along the jaw
                        // (#344) is not where its root is.
                        let reach = (at.x * at.x + at.z * at.z).sqrt();
                        let here = if reach > f32::EPSILON {
                            at.z / reach
                        } else {
                            1.0
                        };
                        let floor = head.jawline(here) - RIDES[1];
                        assert!(
                            at.y >= floor - 0.0005,
                            "a {style:?} clump at facing {facing}, {down} down reaches {:.1} mm \
                             past the jawline it is edged on",
                            (floor - at.y) * 1000.0
                        );
                    }
                }
            }
        }
        assert!(measured > 15, "only {measured} clumps grew to measure");
    }

    #[test]
    fn sideburns_stay_behind_and_a_full_connection_does_not() {
        // The one difference a person would point at, measured rather than
        // asserted by name: a sideburn is the strip beside the ear and stops
        // well before the cheek; a full connection carries on to meet the chin.
        let head = head();
        let cut = Cut::default();
        let grows = |style: FlankStyle, facing: f32| {
            style
                .shape(&cut, Follicle::Flanks, &head)
                .expect("a grown style has a shape")
                .length(&root(&head, facing, 0.4))
                > 0.0
        };
        let burns = FlankStyle::Sideburns { drop: 1.0 };
        let full = FlankStyle::FullConnect { reach: 0.5 };
        assert!(
            grows(burns, -0.1),
            "a sideburn grows nothing beside the ear"
        );
        assert!(
            !grows(burns, 0.75),
            "a sideburn grew onto the cheek, which makes it a full beard"
        );
        assert!(
            grows(full, 0.75) && grows(full, -0.1),
            "a full connection has a hole in it"
        );
    }

    #[test]
    fn the_beard_crosses_the_jaw_without_a_bald_stripe() {
        // **The reason the flanks land after the chin** (#208's own brief). The
        // two regions meet along the jaw — this one hands the midline over at a
        // share of the skull's half-width and the chin's patch is placed in
        // eye-widths from it — and the mask was tuned at #199 so the two overlap.
        // Whether the GROWN layers overlap is a different question, and it is
        // this one: a seam nobody can see in a mask is a bald stripe down
        // somebody's jaw once there is hair on either side of it.
        //
        // Walked along the jawline itself rather than across the boundary,
        // because the boundary is where the answer is and a sweep over the whole
        // face would average it away.
        let mut record = AvatarRecord::new("Seam", Archetype::default());
        record.hair = crate::hair::HairRecord {
            chin: crate::hair::Tress {
                style: crate::hair::ChinStyle::Full,
                ..Default::default()
            },
            flanks: crate::hair::Tress {
                style: FlankStyle::FullConnect { reach: 0.5 },
                ..Default::default()
            },
            ..crate::hair::HairRecord::bald()
        };
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let head = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        let hair = grown(&record, &avatar, &head);
        assert!(!hair.is_empty(), "the beard grew nothing to look at");
        // Just above the crease, which is where both regions are thinnest and
        // where a stripe would show.
        //
        // Measured before it was asserted, and the instrument discriminates:
        // with both regions grown the widest gap anywhere along the jaw is
        // 8.1 mm, and with the flanks shaved it is 71.0. A centimetre is the
        // bound because that is about the spacing of the clumps themselves —
        // wider than that is not thin hair, it is no hair.
        let mut worst = (0.0f32, 0.0f32);
        for step in 0..=24 {
            let facing = 0.85 - 1.05 * step as f32 / 24.0;
            let azimuth = facing.clamp(-1.0, 1.0).acos();
            let height = head.jawline(facing) + 0.004;
            let on = skull.surface_at(height, azimuth);
            let near = hair
                .iter()
                .map(|at| at.distance(on))
                .fold(f32::MAX, f32::min);
            if near > worst.0 {
                worst = (near, facing);
            }
        }
        assert!(
            worst.0 < 0.010,
            "the nearest hair to the jawline is {:.1} mm away at facing {:.2} — that is a bald \
             stripe where the chin's patch and the flanks meet",
            worst.0 * 1000.0,
            worst.1
        );
    }

    /// The chin and the flanks one record grows on one built head, head-local.
    ///
    /// From one stream in [`Follicle::ALL`]'s order, which is what
    /// `Avatar::build` does — so these are the roots that shipped rather than a
    /// second sample from the same distribution.
    fn grown(record: &AvatarRecord, avatar: &Avatar, head: &Follicles) -> Vec<Vec3> {
        use crate::hair::Growth;
        use crate::hair::clump::{Bed, Sowing};
        use rand::SeedableRng;
        let bed = Bed {
            body: &avatar.parts.body,
            rig: &avatar.rig,
            weights: &avatar.parts.weights,
            follicles: head,
        };
        let mut stream = rand_pcg::Pcg64Mcg::seed_from_u64(record.seed as u64);
        let mut growth = Growth::on(head.head);
        for follicle in Follicle::ALL {
            let Some(sown) = record.hair.sowing(follicle, head) else {
                continue;
            };
            growth.grow(
                &bed,
                &Sowing {
                    follicle,
                    count: sown.clumps,
                    shape: sown.shape.as_ref(),
                    roots: Vec3::from_array(sown.roots),
                    tips: Vec3::from_array(sown.tips),
                },
                &mut stream,
            );
        }
        growth.mesh.positions
    }

    #[test]
    fn the_drop_axis_orders_a_sideburns_length() {
        let head = head();
        let root = root(&head, 0.0, 0.55);
        let reach = |drop: f32| {
            FlankStyle::Sideburns { drop }
                .shape(&Cut::default(), Follicle::Flanks, &head)
                .expect("a sideburn has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            reach(1.0) < reach(0.5) && reach(0.5) < reach(0.0),
            "the drop axis does not order a sideburn: {:+.1}, {:+.1}, {:+.1} mm",
            reach(1.0) * 1000.0,
            reach(0.5) * 1000.0,
            reach(0.0) * 1000.0
        );
    }

    #[test]
    fn the_reach_axis_rides_a_full_connection_under_the_jaw() {
        let head = head();
        let root = root(&head, 0.5, 0.9);
        let tip = |reach: f32| {
            FlankStyle::FullConnect { reach }
                .shape(&Cut::default(), Follicle::Flanks, &head)
                .expect("a full connection has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            tip(1.0) < tip(0.5) && tip(0.5) < tip(0.0),
            "the reach axis does not order a full connection: {:+.1}, {:+.1}, {:+.1} mm",
            tip(1.0) * 1000.0,
            tip(0.5) * 1000.0,
            tip(0.0) * 1000.0
        );
    }
}
