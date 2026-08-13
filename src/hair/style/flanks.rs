//! The jaw flanks' own catalogue, and the last of the five.
//!
//! # The region whose edges are somebody else's
//!
//! Every other region in this milestone draws its own boundary. This one has
//! three it does not own: the shaved beard line above it, which the mask cuts
//! and [`Line::top`] carries; the mandible's crease below it, which
//! [`crate::face::skull`] carved and #196 proved must not be copied; and the
//! chin's own patch beside it, which #207 grew and which this has to meet
//! without a stripe of bare jaw between them.
//!
//! So the styles here are shorter on invention than the other four and longer
//! on reading. A flank clump combs DOWN the cheek — downhill on the side of a
//! face is very nearly straight down, which is the one place the engine's own
//! default direction is simply right — and it stops where the beard stops
//! rather than where its own length runs out. That is what makes the edge a
//! LINE: the tips arrive at the crease together, from wherever they grew.
//!
//! # And it is the last, so the macro dies here
//!
//! `styles!` in [`super`] declared the regions whose base style genuinely was
//! the shared [`Fall`](super::super::clump::Fall). The brows left at #205, the
//! scalp at #204, the moustache at #206, the chin at #207, and this one leaves
//! now — so there is nothing left for it to declare and it goes with them.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, flanks::Line};
use super::{Cut, Style, clumps_for};
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
/// Provenance: **tuned by render** (#208).
const REACH: [f32; 2] = [0.020, 0.026];

/// How wide one is at the root at the coarsest cut, in metres.
///
/// **The widest cards in the catalogue, because this is the widest region**
/// (#208). The flanks hold 7.5% of a head's own surface — measured by
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
/// (#208).
const WIDTH: [f32; 2] = [0.0165, 0.0210];

/// What share of that is left at the tip.
///
/// Provenance: **tuned by render** (#208).
const TAPER: [f32; 2] = [0.30, 0.26];

/// How much a clump lies along the skin where it leaves it.
///
/// Flatter than a beard's and flatter than a moustache's: hair on a cheek is
/// pressed against the face, and standing it off reads as a fur collar.
///
/// Provenance: **tuned by render** (#208).
const LIE: [f32; 2] = [0.94, 0.92];

/// How many clumps each style asks for at full density, as a share of the shared
/// count.
///
/// **Sideburns ask for a third**, because a sideburn is a strip and the shared
/// count is sized for a whole flank — and because the roots it declines are
/// roots the budget gets back rather than clumps drawn somewhere they should not
/// be.
///
/// Provenance: **derived** from what each style is (#208).
const CROWD: [f32; 2] = [0.34, 1.0];

/// How far forward of the ear a sideburn reaches, in the azimuth's cosine.
///
/// Sideburns are the region BEHIND this; the full connection is everything. A
/// cosine rather than a distance for the reason the mask's own edges are: it is
/// what a boundary that runs round a head is written in, and it changes smoothly
/// everywhere the region reaches (#199).
///
/// Provenance: **derived** from the mask's own `DIAGONAL`, which is where the
/// beard line has finished dropping to the cheek — a sideburn stops well before
/// that, on the flat of the flank.
const BURNS_FRONT: f32 = 0.28;

/// How softly that edge comes on, in the same cosine.
///
/// Provenance: **tuned by render** (#208).
const BURNS_FADE: f32 = 0.16;

/// How far down its own reach a sideburn runs at each end of its own axis.
///
/// As a share of the way from the beard line to the jawline, so a sideburn ends
/// where a sideburn ends on a face of any length rather than at a fixed depth.
///
/// Provenance: **tuned by render** (#208), against the anatomy it is named for.
const BURNS_DOWN: [f32; 2] = [0.30, 1.0];

/// How far below the jawline a full connection may ride, in metres, at each end
/// of its own axis.
///
/// **Under it, not to it, and the mask says the same** (#199): beard growth
/// crosses the jawline and stops on the upper neck, and a beard that ended on the
/// crease would draw a bright line down the one edge #195 spent a day making
/// smooth.
///
/// Provenance: **tuned by render** (#208), inside the mask's own reach below the
/// border.
const RIDES: [f32; 2] = [0.002, 0.014];

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Provenance: **carried** from [`brows`](super::brows) (#205).
const LEAST_WORTH: f32 = 0.08;

/// How thin a clump is at each of its ends, as a share of its middle.
///
/// A leaf rather than a wedge, so a row of overlapping clumps has no ends in it.
///
/// **Blunter than the other four regions, and the render is why** (#208). A leaf
/// pinched to a third of its width tiles a patch like a lattice of lenses: the
/// cards overlap three deep and the flank still read as a field of dark diamonds
/// with light between them, because what was between them was the gaps the
/// pinched ends left. A row of streaks along a brow wants a fine end; a sheet
/// covering a cheek wants a blunt one.
///
/// Provenance: **carried** from [`brows`](super::brows) (#205), **re-tuned by
/// render** for a region that tiles rather than rows (#208).
const ENDS: f32 = 0.58;

impl Style for FlankStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        let slot = self.slot()?;
        let length = 0.6 + 0.8 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        let (front, down, rides) = match self {
            Self::None => return None,
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
            Self::None => {}
            Self::Sideburns { drop } => *drop = scaled::quantize(drop.clamp(0.0, 1.0)),
            Self::FullConnect { reach } => *reach = scaled::quantize(reach.clamp(0.0, 1.0)),
        }
    }
}

impl FlankStyle {
    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None => None,
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
}

impl Flank {
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
        let facing = Self::facing(root);
        let border = self.regions.jawline(facing) - self.rides;
        // A sideburn stops part of the way down instead, on its own axis.
        let top = self.line.top(facing);
        border + (top - border) * (1.0 - self.down.clamp(0.0, 1.0))
    }

    /// Which way this clump combs: down the flank, along the skin.
    ///
    /// Downhill, which on the side of a face is very nearly straight down — the
    /// one region in this catalogue where the engine's own default direction is
    /// simply right. Under a jaw it points backward into the throat (#207) and
    /// on a brow ridge it points into the eye (#205); on a cheek it points where
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

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let travel = self.length(root) * along;
        root.at + root.out * LIFT + self.run(root) * travel
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`].
        let (base, _) = self.width(root);
        let from_middle = ((along.clamp(0.0, 1.0) - 0.5) / 0.5).abs().min(1.0);
        base * (1.0 - (1.0 - ENDS) * from_middle * from_middle)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let base = self.width * 0.5 * self.share(root);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // The card lies IN the plane of the skin, its width across the comb.
        root.out.cross(self.run(root))
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
                    let floor = head.jawline(facing) - RIDES[1];
                    for step in 0..=12 {
                        let at = shape.at(&root, step as f32 / 12.0);
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
