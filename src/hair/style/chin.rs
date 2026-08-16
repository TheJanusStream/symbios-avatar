//! The chin's own catalogue.
//!
//! # The one region whose hair leaves the head
//!
//! A brow lies on a ridge, a moustache runs along a lip, a scalp lock walks a
//! skull. A beard does none of those: it grows on a patch the size of a thumb
//! and then hangs off the bottom of the face into the air, and everything hard
//! about it is what happens after it leaves.
//!
//! Two things follow, and both are constructions rather than corrections.
//!
//! **It hangs from the MENTON, wherever it grew.** A clump is pulled toward the
//! line under the chin's own tip as it falls — see
//! [`Pad::hangs_from`](super::super::follicle::chin::Pad::hangs_from) — so a
//! hair rooted well back on the submental plane moves FORWARD as it descends
//! rather than straight down into the throat. That is what a beard does, and it
//! is also the throat clearance: there is no separate check keeping hair off the
//! neck, because there is no way for it to get there.
//!
//! **It moves with the JAW.** Hair binds like the skin it grows out of now
//!, and the skin of a chin is held by the mandible — so a beard opens
//! with the mouth. It used to bind rigidly to the head joint with everything
//! else: measured at twenty-five degrees of jaw, the chin's own skin moves
//! 44.7 mm and the hair on it moved zero, which is a beard hanging in the air
//! where the closed mouth used to be.
//!
//! # The three styles
//!
//! [`ChinStyle::Goatee`] is the patch alone, rounded or pointed by its own axis.
//! [`ChinStyle::Full`] carries on down the submental band and hangs. And
//! [`ChinStyle::Braided`] gathers the whole hang into one rope and turns it,
//! which is the style that proves a twist can be a record axis at all.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::super::clump::{LIFT, Root, Shape};
use super::super::follicle::{Follicle, Follicles, chin::Pad};
use super::{Cut, Style, clumps_for};
use crate::plan::scaled;

/// The base styles of the chin.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum ChinStyle {
    /// Nothing is grown here: the chin is painted, or shaved.
    #[default]
    None,
    /// Hair on the chin's own pad and barely past it.
    Goatee {
        /// How the hang ends, `0` a rounded tuft and `1` drawn to a point.
        #[serde(with = "crate::plan::scaled")]
        point: f32,
    },
    /// The pad and the plane under it, hanging clear of the jaw.
    Full,
    /// The whole hang gathered into one turning rope.
    Braided {
        /// How tightly the rope turns, `0` a loose rope and `1` a hard twist.
        #[serde(with = "crate::plan::scaled")]
        twist: f32,
    },
}

/// How far a clump hangs past its root at full length, in metres.
///
/// One entry per style, in the order the enum declares them.
///
/// **In metres rather than in the patch's own share, and the chin is the one
/// region where that is right.** Every other length in this milestone is a
/// share of something measured, because a brow's streak means nothing except
/// against the brow. A beard's hang is against the CHEST — a hand's breadth of
/// beard is a hand's breadth on any head — and the patch it grew from has no
/// opinion about how far down a rope reaches.
///
/// Provenance: **tuned by render**, against the anatomy each is named
/// for: a goatee stops on the jaw, a full beard reaches the throat's own hollow,
/// a braid hangs past it.
const REACH: [f32; 3] = [0.030, 0.075, 0.130];

/// How wide one is at the root at the coarsest cut, in metres.
///
/// Coarser than a moustache's, a beard being the coarsest hair on a face, and
/// the braid coarsest of all because its whole mass is a few ropes rather than
/// many hairs.
///
/// Provenance: **tuned by render**.
const WIDTH: [f32; 3] = [0.0115, 0.0135, 0.0155];

/// What share of that is left at the tip.
///
/// Provenance: **tuned by render**.
const TAPER: [f32; 3] = [0.26, 0.22, 0.55];

/// How much of the way to the hanging line a clump is drawn over its own fall.
///
/// **The throat clearance and the beard's own silhouette are the same number.**
/// A clump pulled toward the line under the menton is a clump moving away from
/// the neck, so a beard that comes to a point below the chin cannot reach the
/// throat — and one that did not converge would hang straight off the submental
/// plane and into it.
///
/// The braid takes nearly all of it, being one rope; the goatee takes less,
/// being a tuft that keeps the pad's own width.
///
/// Provenance: **derived** from the clearance, **tuned by render**.
const GATHER: [f32; 3] = [0.0, 0.0, 0.75];

/// How far the clump bends toward the ground over its length.
///
/// `0` is straight out along the skin and `1` is hanging. A beard hangs, and the
/// record's own droop moves it either side of this.
///
/// Provenance: **tuned by render**.
const DROOP: [f32; 3] = [0.85, 1.05, 1.20];

/// How much a clump lies along the skin where it leaves it.
///
/// Carried from [`Fall::lie`](super::super::clump::Fall::lie), whose render
/// settled it: hair leaves skin at a shallow angle, and the alternative is a
/// hedgehog.
///
/// Provenance: **carried**.
const LIE: f32 = 0.85;

/// How many clumps each style asks for at full density, as a share of the shared
/// count.
///
/// **The braid asks for a third, and that is the style rather than a saving**:
/// a braid is a few heavy ropes, and drawing it with a full beard's worth of
/// clumps would be drawing a full beard that happens to turn. It is also what
/// pays for the stations a turning clump costs.
///
/// Provenance: **derived** from what each style is, **sized by the budget**
///.
const CROWD: [f32; 3] = [1.0, 1.0, 0.34];

/// How far a rope turns about the hanging line over a metre of fall, in radians.
///
/// **A rope and not a corkscrew, and the budget is what decided that**, exactly
/// as it decided the scalp's ringlet. Cost here is curvature: the sampler
/// holds a drawn spine within a millimetre of its curve, so a turn of `k` radians
/// a metre at a radius `r` needs a station every `sqrt(8 x 0.001 / (k² r))`
/// metres. At this rate and the eight millimetres a braid's clumps sit out from
/// its axis, that is a station every 13 mm — six over a braid's whole hang, which
/// is affordable. Twice the rate would be four times the stations.
///
/// Provenance: **derived** from the sampler's own tolerance, **tuned by render**.
const TWIST: [f32; 2] = [6.0, 16.0];

/// How much shorter the outer clumps are than the middle at a `point` of one.
///
/// What turns a rounded tuft into a pointed one: the outline is the length
/// profile across the patch, and a beard drawn to a point is one whose edges
/// stop short while its middle carries on.
///
/// Provenance: **tuned by render**.
const POINT: f32 = 0.55;

/// How far a clump stands off the chin once it has left it, in metres.
///
/// Four millimetres, which is what clears the chin's own forward bulge for a
/// straight card crossing it, and is also about how far a beard stands off a
/// face. See [`Beard::at`].
///
/// Provenance: **derived** from the chord across the chin, measured.
const STAND: f32 = 0.004;

/// Over what share of a clump that standoff is taken up.
///
/// Early: the clump is against the skin at its root and clear of it by the time
/// it is crossing anything.
///
/// Provenance: **tuned by render**.
const LEAVE: f32 = 0.35;

/// How much forward lean the leaving direction carries, against straight down.
///
/// The direction is aimed at the line the beard hangs from as well as at the
/// ground, and this is how much of the aim the first of those gets. Enough that
/// a hair leaving the underside of a jaw is heading out from under it and not
/// along it; not so much that a beard reads as swept forward.
///
/// Provenance: **derived** from the neck's own measured profile — it recedes
/// 58 mm over the four centimetres below the menton — **tuned by render**
///.
const FORWARD: f32 = 0.85;

/// How much of a style's hang a clump rooted at the very back of the submental
/// plane gets.
///
/// A fifth: enough that the plane carries hair rather than reading as shaved,
/// and little enough that the hair there is lying on it rather than hanging off
/// it into the throat. See [`Beard::hangs`].
///
/// Provenance: **derived** from the clearance, **tuned by render**.
const BACK: f32 = 0.2;

/// How much of its hang the patch's own outer corner gives up.
///
/// Most of it. The corner of a submental patch is the jaw's ANGLE, and three
/// things are true there at once: there is no chin under it to hang over, the
/// flanks' own hair takes over, and the neck bulges forward below it — so
/// a clump given a hang there falls straight into the throat, because at the
/// side of a jaw the skin's normal points sideways and the forward lean has
/// almost nothing left after it is projected onto the surface.
///
/// Measured: at a third, one vertex of a braid was inside the neck at rest, and
/// no width or schedule moved it, because what was wrong was that there was hair
/// hanging there at all.
///
/// Provenance: **derived** from where the region ends, **measured** against the
/// body.
const CORNER: f32 = 0.78;

/// Over how much of its own reach, below the menton, a clump finishes closing on
/// the line it hangs from.
///
/// Half: the convergence is done well before the tip, so the bottom half of a
/// beard is one mass hanging from one place rather than a fan still drawing
/// itself in. Shorter and the hairs kink where they arrive; longer and a full
/// beard is still spreading at its own ends.
///
/// Provenance: **tuned by render**.
const FALLEN: f32 = 0.5;

/// The shortest clump worth growing, as a share of the style's full reach.
///
/// Provenance: **carried** from [`brows`](super::brows), whose render settled it
///.
const LEAST_WORTH: f32 = 0.08;

/// What share of its width a clump keeps at the root.
///
/// **Thin, full, thin again — a leaf**. A beard's clumps overlap heavily
/// on a patch this small, and a wedge ending in a blunt face at every root is
/// what makes a row of them read as a row of objects.
///
/// **Thinner at the root than the brows chose, and a chin is why**. A
/// beard's cards are the widest in the catalogue — width is what makes a mass
/// read as a mass — and their width here lies ACROSS the fall, level with the
/// ground. A chin is round, so a wide card centred on its front has its own
/// edges inside it: measured, a full-width root put two vertices five
/// millimetres into the chin. By the middle of a clump the hair has left the
/// skin and the width costs nothing.
///
/// Provenance: **carried** from [`brows`](super::brows), **cut by the
/// chin's own curvature**.
const ENDS: f32 = 0.16;

impl Style for ChinStyle {
    fn grows(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn shape(&self, cut: &Cut, _follicle: Follicle, head: &Follicles) -> Option<Box<dyn Shape>> {
        let slot = self.slot()?;
        let pad = head.pad();
        let length = 0.35 + 0.9 * cut.length.clamp(0.0, 1.0);
        let coarse = 0.7 + 0.6 * cut.thickness.clamp(0.0, 1.0);
        let point = match self {
            Self::Goatee { point } => point.clamp(0.0, 1.0),
            _ => 0.0,
        };
        let twist = match self {
            Self::Braided { twist } => TWIST[0] + (TWIST[1] - TWIST[0]) * twist.clamp(0.0, 1.0),
            _ => 0.0,
        };
        Some(Box::new(Beard {
            pad,
            reach: REACH[slot] * length,
            width: WIDTH[slot] * coarse,
            taper: TAPER[slot],
            gather: GATHER[slot],
            // Either side of the style's own hang, so a record can ask for a
            // beard held out or one combed flat without either end reading as a
            // different style.
            droop: DROOP[slot] * (0.6 + 0.8 * cut.droop.clamp(0.0, 1.0)),
            point,
            twist,
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
            Self::None | Self::Full => {}
            Self::Goatee { point } => *point = scaled::quantize(point.clamp(0.0, 1.0)),
            Self::Braided { twist } => *twist = scaled::quantize(twist.clamp(0.0, 1.0)),
        }
    }
}

impl ChinStyle {
    /// Where this style's numbers sit in the tables above, or `None` if it grows
    /// nothing.
    fn slot(self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Goatee { .. } => Some(0),
            Self::Full => Some(1),
            Self::Braided { .. } => Some(2),
        }
    }
}

/// One clump of beard: it leaves the chin along the skin, falls, and is drawn
/// toward the line the whole beard hangs from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Beard {
    /// The patch this grew on, and the line it hangs from.
    pad: Pad,
    /// How far one clump hangs past its root at full length, in metres.
    reach: f32,
    /// How wide one is at the root, in metres.
    width: f32,
    /// What share of the section is left at the tip.
    taper: f32,
    /// How much of the way to the hanging line it is drawn over its fall.
    gather: f32,
    /// How far it bends toward the ground over its length.
    droop: f32,
    /// How much shorter the outer clumps are than the middle.
    point: f32,
    /// How far it turns about the hanging line over a metre, in radians.
    twist: f32,
}

impl Beard {
    /// How much of the full length this root gets.
    ///
    /// The mask's own weight, square-rooted as everywhere, times the outline the
    /// style's own `point` draws across the patch.
    fn share(&self, root: &Root) -> f32 {
        let out = self.pad.along(root.at.x).clamp(0.0, 1.0);
        let outline = 1.0 - self.point * POINT * out * out;
        root.weight.clamp(0.0, 1.0).sqrt() * outline * self.hangs(root)
    }

    /// How much of the style's hang this root is allowed, `0` to `1`.
    ///
    /// **A beard's mass hangs from the front of the jaw and lies flat behind
    /// it, and that is anatomy before it is arithmetic**. The submental
    /// plane runs backward and meets the neck in a crease; hair rooted just in
    /// front of that crease, given a full hang, goes straight through the neck
    /// under it — which is what the body's own containment test caught, and what
    /// no schedule of the convergence could fix, because the problem was never
    /// where the hair was GOING but how much of it there was back there.
    ///
    /// So the hang follows how far forward the root is: full at the chin's own
    /// tip, and a short lie-down well back under the jaw. See [`BACK`].
    fn hangs(&self, root: &Root) -> f32 {
        let forward = (root.at.z / self.pad.front.max(f32::EPSILON)).clamp(0.0, 1.0);
        // And less again toward the corners, where the patch has run back along
        // the jaw and there is nothing under it to hang over: that corner is
        // where a beard's mass STOPS and the flanks' begins. Hair given a hang
        // there swings into the jaw's own angle when the mandible turns, which
        // is the last two vertices the containment test found.
        let out = self.pad.along(root.at.x).clamp(0.0, 1.0);
        (BACK + (1.0 - BACK) * crate::face::smooth(forward)) * (1.0 - CORNER * out * out)
    }

    /// Which way the clump leaves the skin.
    ///
    /// **Down and FORWARD, and the engine's own downhill is exactly wrong here**
    ///. [`Fall`](super::super::clump::Fall) combs downhill — the ground
    /// projected onto the tangent plane — which is right on a scalp and on a
    /// brow and is the direction into somebody's throat on the underside of a
    /// jaw: that surface slopes BACKWARD as it descends, so downhill along it
    /// points back at the neck.
    ///
    /// Measured, that is the whole of why a beard kept ending up inside the
    /// body. The neck's own front runs from +98 mm at the menton to +40 at its
    /// narrowest four centimetres lower and then bulges out again, so a clump
    /// leaving backward crosses it however its length or its convergence is
    /// scheduled — and three different schedules each fixed one case by
    /// breaking another.
    ///
    /// Aimed instead: down, plus a share of the way toward the line the beard
    /// hangs from, projected onto the skin so it still LEAVES along the surface
    /// rather than standing off it. The path is then a straight line down and
    /// forward from a point on the surface, and a surface that is receding
    /// cannot be re-entered by one.
    fn leaves(&self, root: &Root) -> Vec3 {
        let hang = self.pad.hangs_from();
        let toward = Vec3::new(hang.x - root.at.x, 0.0, hang.z - root.at.z).normalize_or(Vec3::Z);
        let aim = (Vec3::NEG_Y + toward * FORWARD).normalize_or(Vec3::NEG_Y);
        let flow = (aim - root.out * aim.dot(root.out)).normalize_or(root.out);
        let leaves = root.out.lerp(flow, LIE).normalize_or(root.out);
        // **And never backward, which is a clamp on one DIRECTION rather than
        // on a path.** Both terms above can carry a little backward lean — the
        // skin's own normal does under the jaw, and the aim's tangential part
        // does where the surface is turned away from it — and a clump is a
        // straight line plus a fall, so a millimetre of backward lean at the
        // root is a millimetre of neck at the tip. Taken out here, once per
        // clump, it puts no kink in anything: the line is simply aimed level
        // instead of back.
        Vec3::new(leaves.x, leaves.y, leaves.z.max(0.0)).normalize_or(Vec3::NEG_Y)
    }
}

impl Shape for Beard {
    fn length(&self, root: &Root) -> f32 {
        let length = self.reach * self.share(root);
        if length < self.reach * LEAST_WORTH {
            return 0.0;
        }
        length
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let along = along.clamp(0.0, 1.0);
        let travel = self.length(root) * along;
        // Out of the skin, then increasingly toward the ground: the bend is
        // quadratic in the distance travelled, which is what a hanging thing
        // does and what a lerp between two directions does not.
        let heading = (self.leaves(root) + Vec3::NEG_Y * (self.droop * along * along))
            .normalize_or(self.leaves(root));
        // Standing off the skin as it leaves, over the first part of the clump.
        //
        // **A chin is convex and a card is straight, so the chord cuts the
        // bulge** (#207). A clump leaving a root above the menton heading down
        // and forward passes INSIDE the chin's own front, which juts further
        // forward than either end of the chord: measured, four millimetres at
        // the worst. It is the scalp's chord problem (#204) on a much smaller
        // surface, and a beard has the answer a scalp does not — hair on a chin
        // genuinely stands off it, so the clearance is a real property of the
        // hair rather than a correction to the path.
        // **Never backward, for the same reason the leaving direction is not**:
        // the skin under a jaw faces down and BACK, so a standoff along its own
        // normal is a standoff into the neck.
        let off = Vec3::new(root.out.x, root.out.y, root.out.z.max(0.0)).normalize_or(root.out);
        let stand = off * (LIFT + STAND * crate::face::smooth(along / LEAVE));
        let mut at = root.at + stand + heading * travel;
        // **And drawn toward the line the beard hangs from, in the plane only.**
        // Height is the fall's own business; what this closes is the horizontal
        // distance from the chin's tip, which is what makes a beard a shape
        // rather than a fringe hanging off a jaw — and what keeps it off the
        // throat, since every root on the submental plane is BEHIND this line.
        //
        // **Scheduled on how far below the MENTON the clump has fallen, and
        // neither of the two obvious schedules works** (#207). On the clump's
        // own square it does nothing over the first half — exactly where it is
        // needed, since the underside of a jaw runs backward while the neck's
        // surface is forward of it a centimetre lower, and thirty-one vertices
        // of a goatee were inside the throat at rest. Linear in the clump was no
        // better and failed the other way: it pulls a hair sideways ACROSS the
        // chin while the hair is still on it, and a chin is round, so the same
        // thirty-one vertices went inside the chin instead.
        //
        // What separates the two cases is not how far the clump has gone but
        // WHERE IT IS. Above the menton there is still face under the hair and
        // it must not move sideways at all; below it there is nothing but throat
        // and it must move at once. So the schedule is the height, and a clump
        // rooted under the jaw — already below the menton — converges from its
        // first station, which is the case the throat cares about.
        let hang = self.pad.hangs_from();
        let below =
            ((self.pad.under - at.y) / (self.reach * FALLEN).max(f32::EPSILON)).clamp(0.0, 1.0);
        // Squared in the descent, so nothing moves sideways until the clump is
        // well clear. A gather already scheduled below the patch's own edge
        // still had a seventh of itself applied a centimetre under it, which on
        // the outer jaw is enough to walk a rope back into the neck: the surface
        // there comes forward four millimetres for every three the hair moves in.
        let closed = self.gather * below * below;
        at.x += (hang.x - at.x) * closed;
        at.z += (hang.z - at.z) * closed;
        // And turned about that line, if this style is a rope. A turn and not a
        // helix: the radius is whatever the gather has left, so the rope closes
        // and winds at once, which is what a braid does.
        if self.twist > 0.0 {
            // **Turning only once it is a rope**, on the same schedule the
            // convergence runs on. A twist applied from the root turns the clump
            // about the hanging line while it is still lying against the jaw,
            // and a jaw is not a cylinder: eleven vertices of a braid were
            // inside the chin's own underside at rest, all of them within a
            // centimetre of where they grew.
            let (sin, cos) = (self.twist * travel * below * below).sin_cos();
            let from = Vec3::new(at.x - hang.x, 0.0, at.z - hang.z);
            at.x = hang.x + from.x * cos - from.z * sin;
            at.z = hang.z + from.x * sin + from.z * cos;
        }
        at
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // Thin, full, thin — see [`ENDS`], and fullest a little past the middle
        // so the ragged end of the mass is at the bottom where a beard's is.
        let (base, tip) = self.width(root);
        let along = along.clamp(0.0, 1.0);
        if along < 0.45 {
            return base * (ENDS + (1.0 - ENDS) * (along / 0.45));
        }
        let past = (along - 0.45) / 0.55;
        base + (tip - base) * past
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        // As thick as it is long, so a clump the outline cut short is the fine
        // hair a beard's edge ends in rather than a lozenge (#205).
        let base = self.width * 0.5 * self.share(root);
        (base, base * self.taper.clamp(0.0, 1.0))
    }

    fn across(&self, root: &Root) -> Vec3 {
        // Across the fall and level with the ground, which for a hanging clump
        // is the sheet it lies in — the engine's own default, and right here
        // because a beard genuinely does hang. Named rather than inherited so
        // that the one region whose hair leaves the head says what its cards do.
        let heading = self.leaves(root);
        heading
            .cross(Vec3::Y)
            .normalize_or(heading.cross(Vec3::X).normalize_or(Vec3::X))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{Canon, Skull};
    use crate::hair::follicle::FollicleParams;
    use crate::{Archetype, Avatar, AvatarRecord};

    /// How far inside the body a vertex has to be before it is in it, in metres.
    ///
    /// Two millimetres, which is over the millimetre the surface's own
    /// containment answer is trustworthy to (`clump`'s own root test straddles
    /// at one) and well under anything a person would call hair in a neck.
    const DEEP: f32 = 0.002;

    /// The regions of one built head, which a style is fitted against.
    fn head() -> Follicles {
        let record = AvatarRecord::new("Beard", Archetype::default());
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default())
    }

    /// A root on the patch, a share of the way back under it and out across it.
    fn root(pad: &Pad, back: f32, out: f32) -> Root {
        let height = pad.lip + (pad.under - pad.lip) * back;
        Root {
            at: Vec3::new(pad.half * out, height, pad.front * (1.0 - 0.5 * back)),
            // Facing forward on the pad and turning under it, which is what the
            // skin of a chin does.
            out: Vec3::new(0.35 * out, -back, 1.0 - back * 0.6).normalize(),
            weight: 1.0,
            skin: Default::default(),
        }
    }

    /// Every style, at a middling axis.
    fn every_style() -> [ChinStyle; 3] {
        [
            ChinStyle::Goatee { point: 0.6 },
            ChinStyle::Full,
            ChinStyle::Braided { twist: 0.6 },
        ]
    }

    #[test]
    fn no_clump_of_a_beard_ever_falls_backward() {
        // **The construction the throat clearance rests on, stated as the
        // inequality it is** (#207). The neck's own front recedes 58 mm over the
        // four centimetres below the menton, so a clump that never moves
        // backward from where it grew cannot cross it — and one that does will,
        // whatever its length or its shape.
        //
        // This replaces a test that asked whether the tips CONVERGE on the
        // chin's tip, which was a claim about the design that preceded this one:
        // three schedules of a lateral gather each fixed one case by burying
        // another, and what actually fixed it was aiming the clump when it
        // leaves rather than correcting it afterwards. Convergence was never the
        // property that mattered; not going backward is.
        let head = head();
        let pad = head.pad();
        let mut measured = 0usize;
        for style in every_style() {
            let shape = style
                .shape(&Cut::default(), Follicle::Chin, &head)
                .expect("a grown style has a shape");
            for back in [0.2f32, 0.5, 0.8, 1.0] {
                for out in [0.0f32, 0.4, 0.8] {
                    let root = root(&pad, back, out);
                    if shape.length(&root) <= 0.0 {
                        continue;
                    }
                    measured += 1;
                    for step in 0..=16 {
                        let at = shape.at(&root, step as f32 / 16.0);
                        assert!(
                            at.z >= root.at.z - 0.0005,
                            "a {style:?} clump rooted {back} back and {out} out reaches {:.1} mm \
                             behind its own root, which on a head is into the throat",
                            (root.at.z - at.z) * 1000.0
                        );
                    }
                }
            }
        }
        assert!(measured > 20, "only {measured} clumps grew to measure");
    }

    #[test]
    fn a_beard_keeps_off_the_throat_at_rest_and_with_the_jaw_open() {
        // **The issue's own requirement, measured against the body rather than
        // against this file's arithmetic** (#207). The construction above says
        // a clump converges on the chin's tip; this says what that is FOR — no
        // part of a beard may be inside the neck it hangs in front of.
        //
        // And with the jaw open, because that is the pose where it could go
        // wrong in a way rest cannot show. Hair binds like the skin it grows out
        // of now, so a beard swings forward and down with the mandible while the
        // throat stays where it is — which moves the beard AWAY from the neck,
        // and this is what checks that it does rather than assuming it.
        use crate::anim::Pose;
        use glam::Quat;

        for style in every_style() {
            let mut record = AvatarRecord::new("Throat", Archetype::default());
            record.hair = crate::hair::HairRecord {
                chin: crate::hair::Tress {
                    style,
                    cut: Cut {
                        length: 1.0,
                        droop: 1.0,
                        density: 1.0,
                        thickness: 1.0,
                    },
                    ..Default::default()
                },
                ..crate::hair::HairRecord::bald()
            };
            let avatar = Avatar::build(&record).expect("a biped builds");
            let rig = &avatar.rig;
            let tip = (0..rig.len())
                .find(|&tip| {
                    rig.joints[tip].marker
                        && rig.joints[tip]
                            .parent
                            .is_some_and(|at| rig.joints[at].marker)
                })
                .expect("a humanoid has a jaw");
            let pivot = rig.joints[tip].parent.expect("the tip hangs off the pivot");
            // **Talk's own angle, read from Talk rather than picked** (#207).
            // The first cut of this opened the jaw twenty-five degrees, which is
            // a yawn: `TalkConfig::open` is twelve, and its syllables reach
            // between 0.45 and 1.0 of that. A test at twice the angle the system
            // can produce is a test of a pose nothing renders — and it fails,
            // because hair binds rigidly to the jaw and a mandible swinging that
            // far carries a hanging beard back into the neck. That limit is
            // real and named on the issue; what this asserts is the range Talk
            // actually drives.
            let mut open = Pose::rest(rig);
            open.rotations[pivot] = Quat::from_rotation_x(crate::face::TalkConfig::default().open);
            for (name, pose) in [("at rest", Pose::rest(rig)), ("talking", open)] {
                let posed = pose.forward(rig);
                // The closed solid, posed: `parts.body` is the un-charted one,
                // which is the only version `contains` can be asked about.
                let mut body = avatar.parts.body.clone();
                body.skin = avatar.parts.weights.vertices.clone();
                let body = posed.deform_mesh(rig, &body);
                let drawn = avatar.posed(&pose, 0.0);
                let hair = drawn
                    .iter()
                    .find(|mesh| mesh.kind == crate::MeshKind::Hair)
                    .unwrap_or_else(|| panic!("a {style:?} grew no hair to check"));
                assert!(
                    hair.mesh.positions.len() > 50,
                    "a {style:?} grew {} vertices, which is not a beard",
                    hair.mesh.positions.len()
                );
                let skull = Skull::measure(&avatar.parts.body, rig).expect("a head measures");
                let canon = Canon::measure(rig, &skull, &record.eyes);
                let follicles = Follicles::of(rig, &skull, &canon, &record.hair.regions);
                let pad = follicles.pad();
                let origin = rig.joints[follicles.head].position;
                // **Buried rather than touching, and the difference is the
                // instrument** (#207). `PolyMesh::contains` casts a ray, and on
                // a surface with a face's worth of detail it answers about a
                // point within a millimetre of the skin by a coin: measured, six
                // of 266 of the BODY'S OWN vertices read inside when lifted a
                // millimetre and a half off themselves. A test on containment
                // alone reads that 2% as a beard in the throat.
                //
                // So a vertex counts only if it is inside AND still inside two
                // millimetres away in every direction — which is what being in
                // somebody's neck means, and what hair grazing a jaw does not.
                let buried = |at: Vec3| {
                    body.contains(at)
                        && [
                            Vec3::X,
                            Vec3::NEG_X,
                            Vec3::Y,
                            Vec3::NEG_Y,
                            Vec3::Z,
                            Vec3::NEG_Z,
                        ]
                        .into_iter()
                        .all(|step| body.contains(at + step * DEEP))
                };
                let mut inside = 0usize;
                let mut worst = Vec3::ZERO;
                for at in &hair.mesh.positions {
                    if buried(*at) {
                        inside += 1;
                        worst = *at - origin;
                    }
                }
                assert!(
                    inside == 0,
                    "a {style:?} {name} has {inside} vertices inside the body, one at head-local \
                     {:+.1}, {:+.1}, {:+.1} mm — the pad runs {:+.1} to {:+.1} with its menton at \
                     {:+.1} and its front at {:+.1}",
                    worst.x * 1000.0,
                    worst.y * 1000.0,
                    worst.z * 1000.0,
                    pad.under * 1000.0,
                    pad.lip * 1000.0,
                    pad.menton * 1000.0,
                    pad.front * 1000.0
                );
            }
        }
    }

    #[test]
    fn the_goatees_point_axis_draws_its_edges_in() {
        // A variant carrying an axis owes a test that the axis does what its
        // name says, in order.
        let head = head();
        let pad = head.pad();
        let edge = root(&pad, 0.4, 0.85);
        let reach = |point: f32| {
            ChinStyle::Goatee { point }
                .shape(&Cut::default(), Follicle::Chin, &head)
                .expect("a goatee has a shape")
                .length(&edge)
        };
        assert!(
            reach(0.0) > reach(0.5) && reach(0.5) > reach(1.0),
            "the point axis does not order a goatee's edge: {:.1}, {:.1}, {:.1} mm",
            reach(0.0) * 1000.0,
            reach(0.5) * 1000.0,
            reach(1.0) * 1000.0
        );
        // And that it is the EDGE it draws in rather than the whole beard: a
        // pointed goatee is not a short one.
        let middle = root(&pad, 0.4, 0.0);
        let long = |point: f32| {
            ChinStyle::Goatee { point }
                .shape(&Cut::default(), Follicle::Chin, &head)
                .expect("a goatee has a shape")
                .length(&middle)
        };
        assert!(
            (long(1.0) - long(0.0)).abs() < 1e-6,
            "the point axis shortened the middle of a goatee as well as its edge"
        );
    }

    #[test]
    fn the_braids_twist_axis_turns_the_rope() {
        // Measured as the arc the clump sweeps about the hanging line, summed
        // station by station rather than taken between its two ends: an angle
        // read off `atan2` wraps, and the first cut of this reported a hard
        // twist and no twist at all as the same seventeen degrees.
        //
        // SIGNED, and the second cut of it was not. A rope's own convergence
        // moves it about that line as well as the twist does, and unsigned that
        // wandering swamped the axis — 24, 23 and 26 degrees for none, half and
        // all of it. A twist turns one way; a path that merely wanders does not.
        let head = head();
        let pad = head.pad();
        let hang = pad.hangs_from();
        let root = root(&pad, 0.6, 0.7);
        let swept = |twist: f32| {
            let shape = ChinStyle::Braided { twist }
                .shape(&Cut::default(), Follicle::Chin, &head)
                .expect("a braid has a shape");
            let angle = |at: Vec3| (at.x - hang.x).atan2(at.z - hang.z);
            let mut total = 0.0f32;
            let mut last = angle(root.at);
            for step in 1..=32 {
                let here = angle(shape.at(&root, step as f32 / 32.0));
                let mut step = here - last;
                while step > std::f32::consts::PI {
                    step -= std::f32::consts::TAU;
                }
                while step < -std::f32::consts::PI {
                    step += std::f32::consts::TAU;
                }
                total += step;
                last = here;
            }
            total
        };
        // Monotone rather than growing in size: the rope's own convergence
        // sweeps it one way about the line and the twist sweeps it the other, so
        // at the loose end of the axis the two nearly cancel and the total is
        // small and POSITIVE. What the axis has to do is move the total one way,
        // every step of the way, which is what an axis means.
        assert!(
            swept(1.0) < swept(0.5) && swept(0.5) < swept(0.0),
            "the twist axis does not order a braid: {:.0}, {:.0}, {:.0} degrees",
            swept(1.0).to_degrees(),
            swept(0.5).to_degrees(),
            swept(0.0).to_degrees()
        );
    }

    #[test]
    fn a_beard_hangs_below_the_chin_it_grew_on() {
        // The other half of what a beard is, and the half a converging test
        // cannot see: it has to go DOWN. A style whose gather was the whole of
        // its motion would draw a beard that shrank into the menton.
        let head = head();
        let pad = head.pad();
        // At a full length, because what this asks is whether the style hangs
        // at all — a goatee at the shortest cut the record allows is a stubble
        // patch, and reading that as "it does not hang" is reading the fixture.
        let cut = Cut {
            length: 1.0,
            ..Cut::default()
        };
        for style in every_style() {
            let shape = style
                .shape(&cut, Follicle::Chin, &head)
                .expect("a grown style has a shape");
            let root = root(&pad, 0.6, 0.2);
            let tip = shape.at(&root, 1.0);
            assert!(
                tip.y < pad.under,
                "a {style:?} ends at {:+.1} mm, above the patch's own back edge at {:+.1} — it is \
                 not hanging at all",
                tip.y * 1000.0,
                pad.under * 1000.0
            );
        }
    }
}
