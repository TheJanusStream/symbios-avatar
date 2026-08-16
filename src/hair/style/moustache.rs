//! The upper lip's own catalogue.
//!
//! # Why a whisker is not a brow, though it looks like one
//!
//! Both run sideways along a line on the face, so this file starts where
//! [`brows`](super::brows) ended: streaks combed out from the midline, gathered
//! onto the line the mask was cut around, thin at both ends so a row of them
//! reads as one mass rather than as a row of objects.
//!
//! What is different is underneath. A brow's line sits in the middle of a ridge
//! with skin above and below it; a moustache's sits on a patch whose floor is a
//! MOUTH. Ten millimetres below the nostrils is lip, and a millimetre past that
//! is the parting the jaw opens along — so the one thing this catalogue may not
//! do, at any setting of any axis on any head, is put a hair below the
//! vermilion.
//!
//! **So the clearance is a construction and not a check.** A whisker sweeps down
//! as it runs — that is what a moustache does — but by a share of the room its
//! own root stands in above the floor, never by a fixed drop. A clump rooted
//! high sweeps a long way and a clump rooted on the vermilion sweeps not at all,
//! which is both the correct behaviour and a guarantee: see `SAG`'s ceiling of
//! four fifths, which leaves the last fifth of every clump's room unspent.
//! Nothing here clamps, and nothing needs to be checked afterwards — though
//! `a_moustache_stays_out_of_the_mouth_it_grows_over` checks it anyway, against
//! the mouth's own cut seam rather than against this file's arithmetic.
//!
//! # Talk is the easy half, and it is worth saying why
//!
//! Hair binds rigidly to the HEAD joint (`Avatar::meshes`), and the mouth's
//! parting has a skull-held upper seam and a jaw-held lower one that are
//! coincident at rest. So a moustache rides the skull with the upper lip, and
//! when the jaw opens the lower lip moves AWAY from it. Rest is the worst case,
//! and a moustache that clears the parting at rest clears it at every jaw angle.
//!
//! # The three styles, and what separates them
//!
//! One shape with different numbers, as the brows are: [`MoustacheStyle::Chevron`]
//! is the full patch, [`MoustacheStyle::Handlebar`] is the same with its outer
//! clumps carrying on past the corners and curling up, and
//! [`MoustacheStyle::Pencil`] is the same gathered hard onto a line just above
//! the vermilion, thin and few. The handlebar is this milestone's first style
//! whose guides leave the surface on purpose, and its own axis is how far.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, moustache::Lip};
use super::{Cut, Style, clumps_for};
use crate::plan::scaled;

/// The base styles of the upper lip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum MoustacheStyle {
    /// Nothing is grown here: the lip is painted, or bare.
    #[default]
    None,
    /// A full moustache over the whole patch, sweeping down and out.
    ///
    /// The one style with no axis of its own, for the same reason a crop has
    /// none: a chevron is what the shared four already describe.
    Chevron,
    /// The same, with the ends carried past the mouth's corners and turned up.
    Handlebar {
        /// How far the ends sweep, `0` barely past the corners and `1` a full
        /// waxed curl.
        #[serde(with = "crate::plan::scaled")]
        sweep: f32,
    },
    /// A thin line along the top of the vermilion.
    Pencil {
        /// How high the line sits, `0` on the lip's own border and `1` halfway
        /// to the nostrils.
        #[serde(with = "crate::plan::scaled")]
        ride: f32,
    },
}

/// How far out along the lip one clump runs at full length, as a share of the
/// patch's own half-width.
///
/// One entry per style, in the order the enum declares them.
///
/// **A share of the patch and not millimetres**, for the reason every length in
/// the follicle module is one: a 9 mm streak is half a moustache on one face and
/// a third of one on another.
///
/// A little over a third, so that the clumps a full patch can afford overlap
/// about three deep along it and the row reads continuous — the same reading
/// [`brows`](super::brows)'s own reach was tuned to. The pencil runs longer
/// because it has fewer clumps to cover the same width with.
///
/// Provenance: **tuned by render**.
const REACH: [f32; 3] = [0.38, 0.62, 0.55];

/// How wide one is at the root, in metres.
///
/// **The lever that replaces count**, as everywhere in this milestone: a wider
/// card costs exactly what a narrow one does. A moustache is coarser hair than a
/// brow and sits on a patch about twice as deep, so these are up on
/// [`brows`](super::brows)'s.
///
/// Provenance: **tuned by render**.
const WIDTH: [f32; 3] = [0.0038, 0.0042, 0.0026];

/// What share of that is left at the tip.
///
/// Provenance: **tuned by render**.
const TAPER: [f32; 3] = [0.30, 0.34, 0.20];

/// Where in the band each style's mass sits, `0` at the vermilion and `1` at the
/// nostrils.
///
/// A chevron and a handlebar fill the patch, so they gather onto its middle. A
/// pencil is a line above the lip's own border, which is the low end of this and
/// what its own axis moves.
///
/// Provenance: **tuned by render**, against the anatomy each is named for.
const LINE: [f32; 3] = [0.46, 0.46, 0.16];

/// How much of its offset from that line a clump closes over its own length.
///
/// **What turns a scatter of streaks into one moustache.** Roots land
/// anywhere in the band's ten millimetres, and a streak running straight out
/// from wherever it started stays there — which reads as three stacked bars
/// rather than as one moustache.
///
/// The pencil gathers hardest, because a pencil moustache IS a line: it is the
/// only one of the three whose whole character is that its hairs agree about
/// where they are.
///
/// Provenance: **tuned by render**.
const GATHER: [f32; 3] = [0.15, 0.18, 0.88];

/// How far a clump may run past the patch's outer edge, as a share of it.
///
/// A chevron stops about where the paint does. A handlebar is the style whose
/// whole point is that it does not — its ends carry past the mouth's corners,
/// which is where its own curl happens — and a pencil ends at the corner because
/// a pencil that overshot would be a handlebar drawn thin.
///
/// Provenance: **tuned by render**.
const OVERSHOOT: [f32; 3] = [0.06, 0.55, 0.02];

/// How flat against the skin the clumps lie, `1` flat and `0` standing out.
///
/// A moustache stands off the lip rather more than a brow stands off its ridge —
/// that is most of what makes it read as bristle rather than as paint — and the
/// pencil is the one that lies almost flat, being nearly the painted layer with
/// a few hairs on it.
///
/// Provenance: **tuned by render**.
const LIE: [f32; 3] = [0.86, 0.84, 0.96];

/// How many clumps each style asks for at full density, as a share of the shared
/// count.
///
/// **The pencil asks for a third and that is the style, not a saving**: a pencil
/// moustache is a few hairs on a painted line, and drawing it with a full
/// patch's worth of clumps would be drawing a thin chevron.
///
/// Provenance: **derived** from what each style is, **sized by the budget**
///.
const CROWD: [f32; 3] = [1.0, 1.0, 0.34];

/// The most of a clump's clearance above the vermilion its own half-width may
/// take.
///
/// A little over half, so that a card is never so deep that there is no sweep
/// left to spend — the two together are what [`Whisker::room`] divides, and a
/// width that ate the whole clearance would leave a moustache that lay
/// perfectly level.
///
/// Provenance: **derived** from the split between the two.
const WIDTH_SHARE: f32 = 0.55;

/// The most of its own room above the vermilion a clump may sweep down through.
///
/// **The clearance, written as an inequality rather than as a clamp**.
/// The floor a whisker may not reach is the vermilion, so the drop is a share of
/// how far the clump's own seat stands above it — and at four fifths the last
/// fifth of that room is never spent, on any head, at any cut, for any root.
/// A clump rooted on the border sweeps nothing, which is also what a hair
/// growing out of the edge of a lip does.
///
/// Clamping to the floor instead would put a kink in every clump that reached
/// it, and the sampler spends stations on kinks: the guarantee would cost
/// triangles as well as reading as a hair with a corner in it.
///
/// Provenance: **derived** from the clearance it has to leave.
const SAG: f32 = 0.8;

/// How much of that a moustache actually asks for at [`Cut::droop`] of one.
///
/// Droop on this region is how heavily the moustache sweeps toward the mouth
/// rather than how far it hangs, so the axis runs from a whisker held out level
/// to one combed down over the lip. Its top end is [`SAG`] itself.
///
/// Provenance: **tuned by render**.
const SWEEP_FROM: f32 = 0.25;

/// How high the handlebar's ends rise past the corner at a sweep of one, as a
/// share of the patch's half-width.
///
/// **Measured against the patch rather than in millimetres**, so a waxed end is
/// the same gesture on a small face and a large one.
///
/// Provenance: **tuned by render**.
const CURL: f32 = 0.62;

/// How far out the pencil's own axis may raise its line, in the band's share.
///
/// Half the band: at the top of the axis the pencil sits midway to the nostrils,
/// which is a pencil worn high, and past that it would not be a pencil.
///
/// Provenance: **derived** from what the axis is named for.
const RIDE: f32 = 0.34;

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Only a floor against the degenerate, and the same one the brows settled on
/// after learning that shortness is not what makes a stub read badly — a section
/// that follows the clump's own length is.
///
/// Provenance: **derived** from what the render can resolve.
const LEAST_WORTH: f32 = 0.08;

/// How thin a clump is at each of its ends, as a share of its middle.
///
/// A leaf rather than a wedge: the overlaps have no ends in them, so the union
/// is one mass with a ragged edge. See [`Shape::width_at`], whose default is the
/// wedge.
///
/// Provenance: **carried** from [`brows`](super::brows), whose render settled it
///.
const ENDS: f32 = 0.3;

impl Style for MoustacheStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        let slot = self.slot()?;
        let lip = head.lip();
        // **A narrow length axis, as the brows have** (#205): a moustache hair
        // is a moustache hair, and the shared floor of a quarter would leave a
        // three-millimetre streak — a row of arrowheads on a lip. A record still
        // asks for a sparse or a heavy moustache through `density` and
        // `thickness`.
        let length = 0.75 + 0.5 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        let line = match self {
            Self::Pencil { ride } => LINE[slot] + RIDE * ride.clamp(0.0, 1.0),
            _ => LINE[slot],
        };
        let curl = match self {
            Self::Handlebar { sweep } => CURL * sweep.clamp(0.0, 1.0),
            _ => 0.0,
        };
        Some(Box::new(Whisker {
            lip,
            reach: lip.half * REACH[slot] * length,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            line,
            gather: GATHER[slot],
            overshoot: OVERSHOOT[slot],
            lie: LIE[slot],
            sag: SAG * (SWEEP_FROM + (1.0 - SWEEP_FROM) * cut.droop.clamp(0.0, 1.0)),
            curl,
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
            Self::None | Self::Chevron => {}
            Self::Handlebar { sweep } => *sweep = scaled::quantize(sweep.clamp(0.0, 1.0)),
            Self::Pencil { ride } => *ride = scaled::quantize(ride.clamp(0.0, 1.0)),
        }
    }
}

impl MoustacheStyle {
    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    ///
    /// One place the order is written down, so a new style is a variant, an arm
    /// here, and one entry in each table rather than seven chances to misalign.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Chevron => Some(0),
            Self::Handlebar { .. } => Some(1),
            Self::Pencil { .. } => Some(2),
        }
    }
}

/// One whisker: a streak running out along the lip, sweeping toward the mouth
/// without ever reaching it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Whisker {
    /// The patch this grew on, which is also where its floor is.
    lip: Lip,
    /// How far out along the lip one clump runs at full length, in metres.
    reach: f32,
    /// How wide one is at the root, in metres.
    width: f32,
    /// What share of the section is left at the tip.
    taper: f32,
    /// Where in the band the style's mass sits, in the band's own share.
    line: f32,
    /// How much of its offset from that a clump closes over its length.
    gather: f32,
    /// How far a clump may run past the patch's outer edge, as a share of it.
    overshoot: f32,
    /// How flat against the skin it lies.
    lie: f32,
    /// What share of its room above the vermilion it sweeps down through.
    sag: f32,
    /// How high its end rises past the corner, as a share of the half-width.
    curl: f32,
}

impl Whisker {
    /// Which way this clump runs along the skin: outward, toward the corner.
    ///
    /// **Sideways only, with the rise and the fall left out on purpose** — the
    /// same separation [`brows`](super::brows) had to make. How much a clump
    /// climbs or sweeps is stated once in [`Shape::at`], and a direction that
    /// also carried it would mean the two were added and one of them subtracted
    /// again, which is how a droop axis comes to do nothing at all.
    fn run(&self, root: &Root) -> Vec3 {
        let side = if root.at.x < 0.0 { -1.0 } else { 1.0 };
        // **Round the face, not along world X** (#206). A lip is not flat: by
        // the corner of a mouth the skin's own normal has turned to face
        // sideways, and world X minus its normal component is nothing at all
        // there — so the direction fell back to the NORMAL and the clump ran
        // straight out of the cheek. The sheet showed it as black blades
        // standing in the air beside the face, which is the same shape of defect
        // the brows' own `across` had (#205) and for the same reason: a
        // direction that degenerates does not fail, it substitutes.
        //
        // The azimuthal tangent cannot degenerate against a normal that is
        // roughly radial, which on a face it is, and it is also what a moustache
        // actually does: the hairs follow the lip round toward the cheek rather
        // than pointing at a wall.
        let azimuth = root.at.x.atan2(root.at.z);
        let round = Vec3::new(azimuth.cos(), 0.0, -azimuth.sin()) * side;
        let out = root.out;
        (round - out * round.dot(out)).normalize_or(round)
    }

    /// Where this clump's body sits, in head-local metres.
    ///
    /// The gathered height: where the clump has arrived by its tip, rather than
    /// where its root happened to land. **The floor's clearance is measured from
    /// the LOWER of this and the root**, because a clump rooted below the line
    /// climbs toward it and one rooted above it descends — and the room a sweep
    /// may spend is the room the clump has at its lowest, not at either end
    /// alone.
    fn seat(&self, root: &Root) -> f32 {
        let line = self.lip.height(self.line);
        root.at.y + (line - root.at.y) * self.gather.clamp(0.0, 1.0)
    }

    /// How far this clump's SPINE stands above the vermilion at its lowest, in
    /// metres.
    fn clear(&self, root: &Root) -> f32 {
        (root.at.y.min(self.seat(root)) - self.lip.vermilion).max(0.0)
    }

    /// How much of that is left for the sweep, once the card's own half-width is
    /// paid for.
    ///
    /// **A card is a ribbon and its width here is VERTICAL**, so a spine that
    /// clears the vermilion is not a whisker that does: the card hangs half its
    /// width below its own spine. Measured on the built head, a full chevron's
    /// spine stayed above the line and its lower edge was 2.1 mm under it —
    /// hair drawn on the red of a lip, with no paint beneath it, which is the
    /// one thing this epic's two layers may not do.
    ///
    /// So the width comes out of the clearance first (see [`WIDTH_SHARE`]) and
    /// the sweep spends a share of what is left. Both together are still less
    /// than the room, which is the guarantee.
    fn room(&self, root: &Root) -> f32 {
        (self.clear(root) - self.width(root).0).max(0.0)
    }

    /// How much of the full section and length this root gets.
    ///
    /// The mask's own weight, square-rooted for the reason every region does it:
    /// the last tenth of an edge should keep some substance rather than
    /// collapsing, so the patch's rim reads as hair thinning out and not as hair
    /// stopping.
    fn share(&self, root: &Root) -> f32 {
        root.weight.clamp(0.0, 1.0).sqrt()
    }

    /// How much of the full section this clump gets.
    ///
    /// Its own length as a share of the full reach, so a clump is as thick as it
    /// is long: a short one cut off by the corner is the fine hair a moustache
    /// ends in rather than a lozenge sitting off it.
    fn stoutness(&self, root: &Root) -> f32 {
        self.length(root) / self.reach.max(f32::EPSILON)
    }
}

impl Shape for Whisker {
    fn length(&self, root: &Root) -> f32 {
        // Never further out than the style lets it go: a clump rooted near the
        // corner has only the room that is left, or it hangs in the air beside
        // the cheek. The handlebar's own room is most of a half-width past the
        // patch, which is what its ends are.
        let room = (self.lip.half * (1.0 + self.overshoot) - root.at.x.abs()).max(0.0);
        let length = (self.reach * self.share(root)).min(room);
        if length < self.reach * LEAST_WORTH {
            return 0.0;
        }
        length
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let length = self.length(root);
        let travel = length * along;
        let run = self.run(root);
        // Gathering onto the style's own line as it runs, which is what makes a
        // scatter of streaks read as one moustache. Linear in the travel, so it
        // adds no curvature and no stations: a straight clump aimed slightly
        // differently, which is all this is.
        let gather = (self.seat(root) - root.at.y) * along;
        // And sweeping down toward the mouth — by a share of the room this clump
        // has above the vermilion, never by a distance. See [`SAG`]: this is the
        // whole of the clearance, and it is why nothing below clamps.
        //
        // Quadratic in the travel, because a whisker sweeps hardest at its end,
        // which is also what keeps the row's inner half lying along the lip.
        let sink = self.room(root) * self.sag * along * along;
        // Standing off the skin as it goes, which is what `lie` is: a moustache
        // lying flat is a painted one, and one that does not is bristle. The
        // engine's own root lift rides in the same direction and so is written
        // here with it rather than beside it.
        let stand = root.out * (LIFT + (1.0 - self.lie.clamp(0.0, 1.0)) * travel);
        // **And every one of those terms gives up its own rise, so that `gather`
        // and `sink` are the WHOLE of the vertical motion rather than most of
        // it** — which is what makes the clearance a construction. It is
        // [`brows`](super::brows)'s discipline about `run` extended to the lift
        // and the standoff, and it had to be: the skin of an upper lip faces
        // forward and DOWN, so a millimetre and a half of lift plus a
        // millimetre of standoff along that normal is two and a half
        // millimetres of drop nobody budgeted for. Measured on the built head,
        // that put a full chevron 2.9 mm below the vermilion — under its own
        // paint — while the synthetic roots this file's other test uses, whose
        // normals point straight out, saw nothing at all.
        let rise = Vec3::Y * (gather - sink - run.y * travel - stand.y);
        // And the handlebar's own end: past the patch's outer edge the clump
        // turns up and away, and nowhere else does it do anything at all.
        //
        // **Smoothstepped in how far past the corner it has come**, so the curl
        // starts at nothing exactly where the patch ends — a linear rise would
        // put a corner in every clump that crossed the edge, and the whole row
        // would crease along one line.
        //
        // Smoothed rather than SQUARED, which was the first cut and did nothing
        // visible: the furthest any clump gets is about a quarter of a
        // half-width past the edge, and a quarter squared is a sixteenth, so a
        // handlebar's ends rose 0.8 mm where the axis had asked for five. A
        // smoothstep has the same flat start and reaches its full value inside
        // the overshoot the style actually uses.
        // How far past the patch's outer edge the clump has actually come,
        // which with a run that curves round the face is not the same as how
        // far it has travelled.
        let past = (self.lip.along(root.at.x + run.x * travel) - 1.0).max(0.0);
        let curl = Vec3::Y
            * (self.curl
                * self.lip.half
                * crate::face::smooth(past / self.overshoot.max(f32::EPSILON)));
        root.at + run * travel + rise + stand + curl
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`]. Fullest a little past the middle,
        // which keeps the ragged end of the row on the outer side where a
        // moustache's own ends belong.
        let (base, _) = self.width(root);
        let from_middle = ((along.clamp(0.0, 1.0) - 0.55) / 0.55).abs().min(1.0);
        base * (1.0 - (1.0 - ENDS) * from_middle * from_middle)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        // Never wider than the room its own spine has above the vermilion: see
        // [`Whisker::room`]. A clump rooted close to the lip's border is the
        // fine hair a moustache ends in downward as well as outward, which is
        // both what the anatomy does and what keeps the card off the lip.
        let base = (self.width * 0.5 * self.share(root) * self.stoutness(root))
            .min(self.clear(root) * WIDTH_SHARE);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // The card lies IN the plane of the skin, so its width runs across the
        // streak and its face turns outward — which is where the light is and
        // where the camera is for a lip. The engine's default is across a fall,
        // which here is parallel to the spine (#205).
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
        let record = AvatarRecord::new("Moustache", Archetype::default());
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default())
    }

    /// A root in the lip's band, a share of the way up it and out along it.
    ///
    /// **Its normal points forward and DOWN, which is what an upper lip's skin
    /// does**. The first cut of this used a level `+Z` and could
    /// therefore not see the defect the built head had: every term that rides
    /// the normal — the root lift, the standoff — carries a drop with it there,
    /// and on a lip that drop is millimetres. A synthetic root with a level
    /// normal is a synthetic lip, and a test on one measures the fixture.
    fn root(lip: &Lip, up: f32, out: f32) -> Root {
        root_facing(lip, up, out, -0.55)
    }

    /// The same, with the normal's downward tilt named.
    fn root_facing(lip: &Lip, up: f32, out: f32, tilt: f32) -> Root {
        Root {
            at: Vec3::new(lip.half * out, lip.height(up), 0.09),
            out: Vec3::new(0.0, tilt, 1.0).normalize(),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    /// Every style, at a middling axis.
    fn every_style() -> [MoustacheStyle; 3] {
        [
            MoustacheStyle::Chevron,
            MoustacheStyle::Handlebar { sweep: 0.6 },
            MoustacheStyle::Pencil { ride: 0.4 },
        ]
    }

    #[test]
    fn no_whisker_reaches_the_vermilion_at_any_setting() {
        // **The claim this file is built around, and it is asserted at the
        // extremes rather than at a default** (#206): the floor is a MOUTH, and
        // a moustache that reaches it at one cut of one style on one head is a
        // moustache in somebody's lip. The construction that guarantees it is
        // [`SAG`]'s unspent fifth, so what this measures is that the
        // construction holds — including for roots rooted ON the border, where
        // the room is nothing and the sweep must be nothing too.
        let head = head();
        let lip = head.lip();
        let mut grown = 0usize;
        for style in every_style() {
            for droop in [0.0f32, 0.5, 1.0] {
                let cut = Cut {
                    droop,
                    length: 1.0,
                    thickness: 1.0,
                    density: 1.0,
                };
                let shape = style
                    .shape(&cut, Follicle::Moustache, &head)
                    .expect("a grown style has a shape");
                for up in [0.0f32, 0.15, 0.5, 0.85, 1.0] {
                    for out in [0.0f32, 0.3, 0.6, 0.9] {
                        // **Over a range of surface tilts**, because the drop
                        // the normal carries is what the built head caught and
                        // the fixture did not. A lip faces forward and down by
                        // about this much; the ends of the sweep are a flat one
                        // and a steep one.
                        for tilt in [0.0f32, -0.3, -0.55, -0.9] {
                            let root = root_facing(&lip, up, out, tilt);
                            if shape.length(&root) <= 0.0 {
                                continue;
                            }
                            grown += 1;
                            for step in 0..=24 {
                                let along = step as f32 / 24.0;
                                // **The card's lower EDGE, not its spine.** A
                                // whisker's card is a ribbon whose width lies
                                // vertically, so half of it hangs below the
                                // curve `at` describes — and a test on the
                                // spine passes while the moustache is on the
                                // lip (#206).
                                let at = shape.at(&root, along);
                                let edge = at.y - shape.width_at(&root, along);
                                assert!(
                                    edge >= lip.vermilion,
                                    "a {style:?} whisker rooted {up} up and {out} out on skin \
                                     tilted {tilt} at droop {droop} reaches {:.2} mm below the \
                                     vermilion",
                                    (lip.vermilion - edge) * 1000.0
                                );
                            }
                        }
                    }
                }
            }
        }
        // And that it looked at anything: a sweep whose every root is declined
        // asserts nothing at all (#210).
        assert!(
            grown > 500,
            "only {grown} of the sweep's roots grew a whisker to measure"
        );
    }

    #[test]
    fn a_moustache_stays_out_of_the_mouth_it_grows_over() {
        // **The same claim as the one above, made against the mouth itself
        // rather than against this file's own arithmetic** (#206). The test
        // above measures the construction on synthetic roots at the extremes;
        // this one grows the record's own moustache on a built head and
        // compares it with the seam `face::mouth` actually CUT — which is a
        // curve dipping toward the corners, and is not a number this file knows.
        //
        // **And it is the whole Talk claim too.** The parting's upper seam is
        // held by the skull and its lower seam by the jaw, coincident at rest;
        // hair binds rigidly to the head joint. So a moustache rides the upper
        // lip, the lower lip moves AWAY from it when the jaw opens, and rest is
        // the worst case. Clearing the seam here clears it at every jaw angle,
        // which is why there is no posed variant of this test.
        for style in every_style() {
            let mut record = AvatarRecord::new("Talking", Archetype::default());
            record.hair = crate::hair::HairRecord {
                moustache: crate::hair::Tress {
                    style,
                    cut: Cut {
                        length: 1.0,
                        droop: 1.0,
                        density: 1.0,
                        thickness: 1.0,
                    },
                    ..Default::default()
                },
                ..crate::hair::HairRecord::default()
            };
            let avatar = Avatar::build(&record).expect("a biped builds");
            let Some(mouth) = &avatar.parts.mouth else {
                panic!("the default body grew no openable mouth to clear");
            };
            let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
            let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
            let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
            let hair = grown(&record, &avatar, &follicles);
            assert!(
                !hair.is_empty(),
                "a {style:?} at a full cut grew nothing at all"
            );
            let origin = follicles.origin();
            let seam: Vec<Vec3> = mouth
                .upper
                .iter()
                .map(|at| avatar.parts.body.positions[*at as usize] - origin)
                .collect();
            let mut checked = 0usize;
            let mut closest = f32::MAX;
            for at in &hair {
                // The seam under this hair, if there is any: past the mouth's
                // own corners there is none, which is exactly where a
                // handlebar's ends are.
                let Some(under) = seam
                    .iter()
                    .filter(|point| (point.x - at.x).abs() < 0.002)
                    .map(|point| point.y)
                    .fold(None, |worst: Option<f32>, y| {
                        Some(worst.map_or(y, |worst: f32| worst.max(y)))
                    })
                else {
                    continue;
                };
                checked += 1;
                closest = closest.min(at.y - under);
                assert!(
                    at.y > under,
                    "a {style:?} hangs {:.2} mm below the mouth's own cut at x {:+.1} mm",
                    (under - at.y) * 1000.0,
                    at.x * 1000.0
                );
            }
            // And that it looked at the moustache rather than past it: a filter
            // that matches nothing passes every assertion inside it (#210).
            //
            // A count rather than a share, because the share is a property of
            // the STYLE: a handlebar's ends are past the mouth's own corners by
            // design, where there is no seam to clear, so more than half of its
            // vertices legitimately have nothing under them. Asking for a share
            // would be asking the handlebar to stop being one.
            assert!(
                checked > 40,
                "only {checked} of {} {style:?} vertices had any mouth under them",
                hair.len()
            );
            let lip = follicles.lip();
            let (lo, hi) = hair
                .iter()
                .fold((f32::MAX, f32::MIN), |s, at| (s.0.min(at.y), s.1.max(at.y)));
            println!(
                "{style:?}: {checked} vertices, closest {:.2} mm above the cut; band {:+.1}..{:+.1}, hair {:+.1}..{:+.1}",
                closest * 1000.0,
                lip.vermilion * 1000.0,
                lip.nostrils * 1000.0,
                lo * 1000.0,
                hi * 1000.0
            );
        }
    }

    /// The moustache one record grows on one built head, head-local.
    ///
    /// **From one stream in [`Follicle::ALL`]'s order**, which is what
    /// `Avatar::build` does — so these are the roots that shipped rather than a
    /// second sample from the same distribution, which is the form
    /// `examples/follicleaudit` takes it in.
    fn grown(record: &AvatarRecord, avatar: &Avatar, follicles: &Follicles) -> Vec<Vec3> {
        use crate::hair::Growth;
        use crate::hair::clump::{Bed, Sowing};
        use rand::SeedableRng;
        let bed = Bed {
            body: &avatar.parts.body,
            rig: &avatar.rig,
            weights: &avatar.parts.weights,
            follicles,
        };
        let mut stream = rand_pcg::Pcg64Mcg::seed_from_u64(record.seed as u64);
        let mut mine = Vec::new();
        for follicle in Follicle::ALL {
            let Some(sown) = record.hair.sowing(follicle, follicles) else {
                continue;
            };
            let mut growth = Growth::on(follicles.head);
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
            if follicle == Follicle::Moustache {
                mine = growth.mesh.positions.clone();
            }
        }
        mine
    }

    #[test]
    fn only_the_handlebar_leaves_the_patch() {
        // The three styles are one shape with different numbers, so what makes
        // them different has to be measured rather than asserted by name. This
        // is the difference a person would point at: a handlebar has ENDS, past
        // the corners of the mouth, and the other two stop on the lip.
        let head = head();
        let lip = head.lip();
        let cut = Cut::default();
        let furthest = |style: MoustacheStyle| {
            let shape = style
                .shape(&cut, Follicle::Moustache, &head)
                .expect("a grown style has a shape");
            let root = root(&lip, 0.5, 0.85);
            (0..=16)
                .map(|step| lip.along(shape.at(&root, step as f32 / 16.0).x))
                .fold(0.0f32, f32::max)
        };
        let (chevron, handlebar, pencil) = (
            furthest(MoustacheStyle::Chevron),
            furthest(MoustacheStyle::Handlebar { sweep: 1.0 }),
            furthest(MoustacheStyle::Pencil { ride: 0.0 }),
        );
        assert!(
            handlebar > 1.1,
            "a handlebar reaches {handlebar:.2} of the patch's own half-width, which is not past \
             the corners"
        );
        assert!(
            chevron <= 1.1 && pencil <= 1.1,
            "a chevron reaches {chevron:.2} and a pencil {pencil:.2} of the half-width, and \
             neither is meant to leave the patch"
        );
    }

    #[test]
    fn the_handlebars_sweep_axis_turns_its_ends_up() {
        // A variant carrying an axis owes a render that the axis does something,
        // and a test that it does it in order (#204's convention for the scalp's
        // five). Measured at the tip of a clump rooted near the corner, which is
        // the only place a handlebar differs from a chevron at all.
        let head = head();
        let lip = head.lip();
        let root = root(&lip, 0.5, 0.85);
        let tip = |sweep: f32| {
            MoustacheStyle::Handlebar { sweep }
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a handlebar has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            tip(1.0) > tip(0.5) && tip(0.5) > tip(0.0),
            "the sweep axis does not order a handlebar: {:+.1}, {:+.1}, {:+.1} mm",
            tip(1.0) * 1000.0,
            tip(0.5) * 1000.0,
            tip(0.0) * 1000.0
        );
    }

    #[test]
    fn the_pencils_ride_axis_lifts_its_line_off_the_lip() {
        let head = head();
        let lip = head.lip();
        // Rooted at the top of the band, so what is measured is where the clump
        // GATHERS to rather than where it started: a pencil's line is the whole
        // of what it is.
        let root = root(&lip, 1.0, 0.4);
        let seat = |ride: f32| {
            MoustacheStyle::Pencil { ride }
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a pencil has a shape")
                .at(&root, 1.0)
                .y
        };
        assert!(
            seat(1.0) > seat(0.5) && seat(0.5) > seat(0.0),
            "the ride axis does not order a pencil: {:+.1}, {:+.1}, {:+.1} mm",
            seat(1.0) * 1000.0,
            seat(0.5) * 1000.0,
            seat(0.0) * 1000.0
        );
        // And that it is still a line low on the lip at the top of its axis,
        // rather than a chevron: half the band is what the axis is worth.
        assert!(
            seat(1.0) < lip.height(0.6),
            "a pencil worn as high as it goes seats at {:.2} of the band, which is a chevron",
            (seat(1.0) - lip.vermilion) / lip.span()
        );
    }

    #[test]
    fn a_whisker_gathers_toward_its_line_rather_than_staying_where_it_landed() {
        // #205's lesson, which this file starts from: roots land anywhere in the
        // band and streaks that run straight out from where they landed read as
        // three stacked bars. Measured as the spread of the tips against the
        // spread of the roots.
        let head = head();
        let lip = head.lip();
        for style in every_style() {
            let shape = style
                .shape(&Cut::default(), Follicle::Moustache, &head)
                .expect("a grown style has a shape");
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for up in [0.05f32, 0.3, 0.55, 0.8, 1.0] {
                let tip = shape.at(&root(&lip, up, 0.35), 1.0).y;
                lo = lo.min(tip);
                hi = hi.max(tip);
            }
            let spread = (hi - lo) / lip.span();
            assert!(
                spread < 0.8,
                "a {style:?}'s tips are spread over {:.0}% of the band from roots spread over \
                 95% of it, which is a row of bars rather than a moustache",
                spread * 100.0
            );
        }
    }
}
