//! Wings (#421): a winged body's flight surfaces, the joints that carry them,
//! and the one value that folds and spreads them.
//!
//! **Built at the fold, spread by a pose.** A winged body's rest is its wing
//! folded, as its plan stands it (`plan::derive::winged`): a bird's arm a Z
//! of markers along its flank, a bat's crawling on its wrists, a glider's an
//! ordinary foreleg. What this module adds to the rig is the joints a wing
//! needs beyond that arm - a bat's five digits, a bird's flight feathers one
//! joint each, a tail fan's two edges - and, for every joint the wing turns,
//! the rotation that carries it from the fold to the wing spread
//! ([`Drive`]). [`Wings::pose`] writes those rotations into a pose for any
//! extension from `0` (folded, the rest) to `1` (spread), each joint along
//! the shortest arc between the two and on a schedule of its own, so a pose
//! is continuous over the whole range and the motion phase (#424) drives a
//! wing with one number a side.
//!
//! **The surface follows the family that leads the body**, as its posture
//! does: feathers for a bird and a bird-led cross, a membrane for a bat, a
//! glider and every cross they lead. Its proportions are the body's own,
//! blended: a part reads its lengths off the arm it hangs from.
//!
//! - **A membrane** (`membrane`) is a closed thin shell in the skin's draw,
//!   whose material is single-sided: two sheets a millimetre or two apart and
//!   a rim between them. It is laid out **spread**, where its shape is
//!   simple - taut between its spars, scalloped between their tips - and
//!   bound there by weights interpolated from its edges, each edge's own
//!   (a digit's joints, the arm's, the body's skin where it roots, the hind
//!   leg's at the ankle), under the four-influence cap. Its rest is that
//!   surface carried back by the inverse of its own binding, so the fold is
//!   what the binding makes of it - the spars laid together, the membrane
//!   bunched between them - and a spread pose puts it back exactly.
//! - **Flight feathers** (`feather`) are cards in the hair's draw, which is
//!   double-sided and cut by the strand mask, each on a joint at its root
//!   turned by its own share of the fan: stacked along the flank when
//!   folded, fanned when spread. Coverts ride between the joints of the
//!   feathers they cover, graded.
//!
//! New geometry and trigonometry here go through `crate::det`, so a wing is
//! the same bits natively and in a browser.
//!
//! docs/winged-anatomy.md has the targets, their sources in research-10 and
//! research-11, and every departure.

mod feather;
mod membrane;

pub(crate) use feather::{Vane, vanes};
pub(crate) use membrane::{Charted, charted};

/// What a whole winged body may cost, in triangles (#421, #423): about a
/// beast's, its every draw counted - the skin with its membrane and toes, the
/// eyes and beak, and the hair's draw its flight feathers share with its
/// contour feathers or its fur. A winged body's cards are fitted to what its
/// other draws leave of it (`Avatar`'s winged cards), so none goes past it.
///
/// Provenance: **chosen** at W2 (#421) against the dearest body then, a
/// bat-led roll at 25,142, at 26,000; and **raised to 27,200 at W4 (#423),
/// the owner's call**, where the cards take what is left: at 26,000 a
/// bat-led cross kept as little as 156 of the 1,332 fur triangles it asked
/// for, and every winged body wears its cards whole under 27,158 (the
/// dearest, seed 2, a bat-glider cross, over 181 bodies of every family,
/// cross and appeal and 100 rolls). At W5 (#424) the contour cards under a
/// folded wing are thinned rather than cleared, so a spread wing shows no
/// bare skin, and the dearest is 27,164 (the same seed 2).
pub const MAX_TRIANGLES: usize = 27_200;

use glam::{Mat3, Quat, Vec3};

use crate::anim::Pose;
use crate::anim::dual::{self, DualQuat};
use crate::det::{self, Rot};
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::{Blended, Limb, WingedFamily, WingedParams, Zone};
use crate::rig::skin::{Influence, MAX_INFLUENCES};
use crate::rig::{Rig, Role, SkinWeights};

/// One joint a wing turns, and how far: the rotation that carries it from
/// the fold to the wing spread, in the body's frame; the joint above it that
/// rotation is reckoned against in a [`Pose`]; and the share of the
/// extension over which it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Drive {
    /// The joint.
    pub joint: usize,
    /// Its rotation with the wing spread, **in the body's frame** - as its
    /// world turn from the fold - and the fold the identity.
    pub spread: Quat,
    /// The nearest joint above it the wing drives, if any: what its own
    /// rotation in a pose is reckoned against.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub anchor: Option<usize>,
    /// The extension at which it starts to move.
    pub from: f32,
    /// The extension by which it has arrived.
    pub to: f32,
}

impl Drive {
    /// Its turn in the body's frame at `extension`, `0` folded to `1`
    /// spread, eased in and out over its share of the range, along its own
    /// quaternion's arc: the shortest, as the wing signs every drive (`sign`
    /// says why). **Near a half turn the shortest arc is a knife edge**: two
    /// spars' turns a few degrees either side of one, about axes a little
    /// apart, end close together and pass nearly a half turn apart midway,
    /// and a membrane blended between them flips. `fit` trims the turns
    /// wherever a blended pair would come that close. An extension that is not
    /// finite reads as the fold (#427).
    #[must_use]
    pub fn at(&self, extension: f32) -> Quat {
        let extension = if extension.is_finite() {
            extension
        } else {
            0.0
        };
        let span = (self.to - self.from).max(f32::EPSILON);
        let share = ((extension - self.from) / span).clamp(0.0, 1.0);
        arc(self.spread, share * share * (3.0 - 2.0 * share))
    }
}

/// An extension or a spread as a pose reads it, `0..=1` (#427): one that is not
/// a number or not finite reads as `0`, the fold. `f32::clamp` passes a NaN
/// through, and a NaN extension posed every driven joint NaN.
fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// An angle a stroke turns by, in radians: one that is not finite reads as
/// none (#427), so it cannot pose a joint NaN.
fn finite(angle: f32) -> f32 {
    if angle.is_finite() { angle } else { 0.0 }
}

/// `share` of the way along `turn`'s own arc from the identity: the turn by
/// `share` of its angle about its axis, taking the long way round when the
/// quaternion's sign says so.
fn arc(turn: Quat, share: f32) -> Quat {
    use crate::det::DetMath;
    let turn = turn.normalize();
    let axis = Vec3::new(turn.x, turn.y, turn.z);
    let length = axis.length();
    if length <= 1e-7 {
        return Quat::IDENTITY;
    }
    let half = length.det_atan2(turn.w) * share;
    let (sin, cos) = det::sin_cos(half);
    let axis = axis / length * sin;
    Quat::from_xyzw(axis.x, axis.y, axis.z, cos)
}

/// One wing's drives: its arm, its digits or its feathers, and on a body
/// with a membrane the hind leg its trailing edge reaches.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct WingSide {
    /// The fore limb the wing is.
    pub limb: Limb,
    /// Every joint it turns, parent before child.
    pub drives: Vec<Drive>,
}

/// What a winged body's rig carries to fold and spread its wings and its
/// tail (#421): read by [`Rig::spread_wings`] and [`Rig::spread_tail`].
///
/// **Data, not a rebuild**: a pose is all it writes, so it crosses a worker
/// boundary with the rig and a motion on the other side drives it. Every
/// field is `serde(default)`, and a rig without wings carries `None`.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Wings {
    /// One per wing.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub sides: Vec<WingSide>,
    /// A bird's tail fan: its two edges' joints. Empty on a body that has no
    /// fan.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub tail: Vec<Drive>,
    /// Every joint the wings added to the rig, ascending: a bat's digits and
    /// thumb, a glider's spars, a bird's feather roots and its tail fan's
    /// edges.
    ///
    /// **They take the zone of what they grow from and are no part of it**
    /// (#421, the end review): a digit or a feather is zoned as its arm's
    /// extremity and a fan's edge as the tail, so a query by zone finds them,
    /// but none of them is a foot, and [`Rig::extremity_joints`] leaves them
    /// out. Read by [`Self::owns`].
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub joints: Vec<usize>,
    /// Which families the body is (#424): each one's share of its lineage
    /// and the one that leads it, written once when the wings are grown -
    /// what every winged motion reads ([`Self::kinship`]): the ground gaits
    /// and the roster by the leading family, a flight's beat and a rest's
    /// curl by the shares.
    ///
    /// **`None` on a rig from an older build**, read across a worker
    /// boundary. [`Self::kinship`] then reads the family off the rig's stance,
    /// for every reader alike: wholly a bird on two legs, wholly a bat on
    /// four - a glider from before it was written goes as a bat, which no
    /// rig's joints can tell it from.
    #[cfg_attr(
        feature = "serde-avatar",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub kinship: Option<Kinship>,
    /// How a bird goes on the ground (#424): its head-bob and its hop, its
    /// kind's and its size's ([`Gaits::of`]). Neither on a body a bird does
    /// not lead; default on a rig from an older build, whose bird then walks
    /// and runs without either.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub gaits: Gaits,
    /// How high the lowest point of the drawn wing parts stands at the fold,
    /// in body space, the ground at nought (#424): how far a perching body
    /// can come down before its folded wing meets the ground. `None` on a rig
    /// from an older build, which a rest then does not limit.
    #[cfg_attr(
        feature = "serde-avatar",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub low: Option<f32>,
}

/// Which families a winged body is (#424), and which leads it: each family's
/// share of its lineage, summing to one - a family's own body all of one, a
/// cross shared by its blend - and its leading family, as its posture and its
/// surface are that family's ([`crate::plan::Lineage::leading`]).
///
/// **Every choice of one family is the leading one's, every blend the
/// shares'**: the gait a body goes in, whether it beats its wings and how it
/// takes off, which roster it is offered; a beat's rate and width, a rest's
/// curl and its times. The shares cannot name a family at an even blend, so
/// a cross at exactly 0.5 takes the leading family's answer to every one of
/// those choices, the one its surface wears.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Kinship {
    /// How far a bird, `0..=1`.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub bird: f32,
    /// How far a bat.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub bat: f32,
    /// How far a glider.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub glider: f32,
    /// The family that leads it.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub leading: WingedFamily,
}

impl Kinship {
    /// The kinship a body of `params` is drawn from.
    #[must_use]
    pub fn of(params: &WingedParams) -> Self {
        let lineage = params.lineage();
        let mut kinship = Self::pure(lineage.family);
        if let Some(other) = lineage.crossed() {
            let blend = lineage.blend.clamp(0.0, 1.0);
            *kinship.share_mut(lineage.family) = 1.0 - blend;
            *kinship.share_mut(other) += blend;
        }
        kinship.leading = lineage.leading();
        kinship
    }

    /// A body wholly of `family`.
    #[must_use]
    pub fn pure(family: WingedFamily) -> Self {
        let mut kinship = Self {
            bird: 0.0,
            bat: 0.0,
            glider: 0.0,
            leading: family,
        };
        *kinship.share_mut(family) = 1.0;
        kinship
    }

    fn share_mut(&mut self, family: WingedFamily) -> &mut f32 {
        match family {
            WingedFamily::Bird => &mut self.bird,
            WingedFamily::Bat => &mut self.bat,
            WingedFamily::Glider => &mut self.glider,
        }
    }

    /// A per-family number blended by these shares: `bird`, `bat` and
    /// `glider` each family's own.
    #[must_use]
    pub fn mix(&self, bird: f32, bat: f32, glider: f32) -> f32 {
        let total = (self.bird + self.bat + self.glider).max(f32::EPSILON);
        (self.bird * bird + self.bat * bat + self.glider * glider) / total
    }

    /// Whether the body beats its wings at all: a glider glides (the
    /// owner's decision, #418), and so does a cross a glider leads, as its
    /// posture and its surface are a glider's - [`Self::leading`], so a cross
    /// at an even blend beats exactly when it wears a beater's surface.
    #[must_use]
    pub fn flaps(&self) -> bool {
        self.leading != WingedFamily::Glider
    }
}

/// How a winged body goes on the ground (#424), read off its plan when its
/// wings are grown: a bird's two habits, which are its kind's and its size's
/// (research-11 2.1 and 2.2) - pigeons, fowl and corvids bob and raptors,
/// owls and parrots do not; small birds hop, and a crow walks and hops.
/// Which gaits a body goes by at all is its leading family's
/// ([`Kinship::leading`], the owner's decision 5): a bird walks and runs on
/// two legs, a bat crawls and bounds on its wrists and feet, a glider walks
/// and bounds on all four.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Gaits {
    /// How far a bird's head holds and thrusts as it walks, `0` none to `1`
    /// the whole of each step: none on a hooked beak - an owl's, a parrot's -
    /// and all of it on a straight one, a crow's or a songbird's. Nought on a
    /// bat and a glider.
    #[cfg_attr(feature = "serde-avatar", serde(default))]
    pub bob: f32,
    /// The Froude number from which a bird hops rather than walks or runs:
    /// nought on a small bird, which hops at every pace; a walk's end on a
    /// straight-beaked one, which walks and then hops, as a crow does; `None`
    /// on one that walks and runs, and on a bat and a glider.
    #[cfg_attr(
        feature = "serde-avatar",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub hops_from: Option<f32>,
}

/// A bird whose `size` is at most this hops at every pace, in metres of head
/// and body: the bottom eighth of the plan's range, a songbird's end of it.
/// Provenance: **chosen** (#424) on research-11 2.1, "small passerines
/// usually hop": the dossier gives the habit, not a size.
pub const HOP_SIZE: f32 = 0.45;

/// The beak's hook from which a bird bobs none, and the one from which it
/// bobs whole: the toes' cut (`WingedParams::toes`, -0.5) is where a hooked
/// beak ends, and the crow's straight beak (-1) bobs all of it.
const BOB_HOOK: (f32, f32) = (-0.5, -1.0);

impl Gaits {
    /// What a body built from `params`, of `kinship`, goes by: nothing of a
    /// bird's unless a bird leads it.
    #[must_use]
    pub fn of(params: &WingedParams, kinship: &Kinship) -> Self {
        if kinship.leading != WingedFamily::Bird {
            return Self::default();
        }
        let straight =
            ((BOB_HOOK.0 - params.beak_hook) / (BOB_HOOK.0 - BOB_HOOK.1)).clamp(0.0, 1.0);
        let hops_from = if params.size <= HOP_SIZE {
            Some(0.0)
        } else if params.beak_hook <= BOB_HOOK.0 {
            Some(crate::anim::speed::FROUDE_TRANSITION)
        } else {
            None
        };
        Self {
            bob: straight * straight * (3.0 - 2.0 * straight),
            hops_from,
        }
    }
}

/// One wing's pose beyond its extension (#424): how far it is spread, and
/// how its stroke carries it, all in the trunk's own frame so a body's rest
/// pitch is no part of it.
///
/// **The stroke turns the wing about its own roots**: the arm about its
/// shoulder and a bat's digits and a bird's feathers with it, wholly; a
/// membrane's spars about their roots on the flank, each by its place
/// between the hip (none of the stroke) and the shoulder (all of it), so the
/// sheet between the arm and the leg turns by a share of the whole; the hind
/// leg a membrane reaches not at all. `reach` swings every drive alike, the
/// legs included: all four limbs forward into a flare.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stroke {
    /// `0` folded, as the body rests, to `1` spread: [`Wings::pose`]'s.
    pub extension: f32,
    /// How far the wing is raised about the trunk's long axis, in radians,
    /// up positive: the beat's up and down, and a glide's dihedral.
    pub elevation: f32,
    /// How far it is swept forward about the trunk's up, in radians, back
    /// negative: the beat's fore and aft.
    pub sweep: f32,
    /// How far every limb a wing drives swings forward about the trunk's up,
    /// in radians, back negative.
    pub reach: f32,
}

impl Stroke {
    /// A wing at `extension` with no stroke: [`Wings::pose`]'s.
    #[must_use]
    pub fn at(extension: f32) -> Self {
        Self {
            extension,
            ..Self::default()
        }
    }
}

impl Wings {
    /// Writes each wing's joints into `pose` at its extension, `0` folded to
    /// `1` spread, `left` for the wing on the body's left (`+X`).
    ///
    /// **Writes, never composes**: every joint a wing drives takes the
    /// rotation its extension asks for, and every other joint keeps what the
    /// pose held. A bird's elbow and wrist move as one (research-10 4.1: the
    /// linkage couples them in all 61 species measured); its feathers fan a
    /// little behind its arm.
    ///
    /// **Each joint turns along its own arc in the body's frame**, and its
    /// rotation in the pose is that turn reckoned against its anchor's (#421,
    /// the critic). Slerped joint by joint in their parents' frames instead,
    /// a chain of large turns about different axes brought two joints a
    /// membrane blends - a glider's forearm and the spar beside it - more
    /// than a half turn apart partway through, and the membrane flipped;
    /// along arcs from one fold, two joints drift apart smoothly toward how
    /// far apart they are spread.
    pub fn pose(&self, pose: &mut Pose, left: f32, right: f32) {
        for side in &self.sides {
            let extension = if side.limb.is_left() { left } else { right };
            write(&side.drives, pose, extension);
        }
    }

    /// Writes the wing that is `limb` into `pose` at `extension`, as
    /// [`Self::pose`] writes both, and leaves the other alone (#424): a
    /// stretch lays out one wing at a time.
    pub fn pose_side(&self, pose: &mut Pose, limb: Limb, extension: f32) {
        for side in self.sides.iter().filter(|side| side.limb == limb) {
            write(&side.drives, pose, extension);
        }
    }

    /// Writes the tail fan into `pose`, `0` closed to `1` fanned.
    pub fn fan_tail(&self, pose: &mut Pose, spread: f32) {
        write(&self.tail, pose, spread);
    }

    /// Which families the body is (#424): what [`Self::kinship`] holds, and on
    /// a rig that holds none, from an older build, the family its stance
    /// belongs to - wholly a bird on two legs, wholly a bat on four.
    #[must_use]
    pub fn kinship(&self, rig: &Rig) -> Kinship {
        self.kinship.unwrap_or_else(|| {
            Kinship::pure(if rig.four_legged() {
                WingedFamily::Bat
            } else {
                WingedFamily::Bird
            })
        })
    }

    /// Writes each wing at its [`Stroke`] (#424): [`Self::pose`] at its
    /// extension, then turned by its stroke about each part's own roots. A
    /// stroke of nothing is [`Self::pose`] to the bit. An extension or an angle
    /// that is not finite reads as `0` (#427).
    pub fn stroke(&self, rig: &Rig, pose: &mut Pose, left: Stroke, right: Stroke) {
        let [left, right] = [left, right].map(|stroke| Stroke {
            extension: unit(stroke.extension),
            elevation: finite(stroke.elevation),
            sweep: finite(stroke.sweep),
            reach: finite(stroke.reach),
        });
        let Some(trunk) = Trunk::of(rig) else {
            self.pose(pose, left.extension, right.extension);
            return;
        };
        let pelvis = rig
            .in_zone(Zone::Pelvis)
            .first()
            .map_or(Vec3::ZERO, |&joint| rig.joints[joint].position);
        for side in &self.sides {
            let (stroke, sign) = if side.limb.is_left() {
                (left, 1.0)
            } else {
                (right, -1.0)
            };
            if stroke.elevation == 0.0 && stroke.sweep == 0.0 && stroke.reach == 0.0 {
                write(&side.drives, pose, stroke.extension);
                continue;
            }
            let reach = Rot::axis_angle(trunk.up, -sign * stroke.reach);
            let turned = |share: f32| {
                reach
                    * Rot::axis_angle(trunk.up, -sign * stroke.sweep * share)
                    * Rot::axis_angle(trunk.along, sign * stroke.elevation * share)
            };
            let extension = stroke.extension;
            let mut done: Vec<(usize, Quat)> = Vec::with_capacity(side.drives.len());
            for drive in &side.drives {
                let share = self.stroke_share(rig, drive.joint, pelvis, &trunk);
                let world = (turned(share) * drive.at(extension)).normalize();
                let anchor = drive
                    .anchor
                    .and_then(|anchor| done.iter().find(|(joint, _)| *joint == anchor))
                    .map_or(Quat::IDENTITY, |&(_, world)| world);
                done.push((drive.joint, world));
                if let Some(rotation) = pose.rotations.get_mut(drive.joint) {
                    *rotation = (anchor.inverse() * world).normalize();
                }
            }
        }
    }

    /// How much of a wing's stroke `joint` takes: all of it on the arm and
    /// whatever grows from it, none on a hind leg, and on a spar rooted on
    /// the flank its root's place from the hip to the shoulder.
    fn stroke_share(&self, rig: &Rig, joint: usize, pelvis: Vec3, trunk: &Trunk) -> f32 {
        let mut at = joint;
        let mut root = joint;
        while self.owns(at) {
            root = at;
            match rig.joints[at].parent {
                Some(parent) => at = parent,
                None => break,
            }
        }
        match rig.joints[at].zone {
            Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) => {
                if limb.is_fore() { 1.0 } else { 0.0 }
            }
            _ => ((rig.joints[root].position - pelvis).dot(trunk.along)
                / trunk.length.max(f32::EPSILON))
            .clamp(0.0, 1.0),
        }
    }

    /// Writes the tail fan at `spread` and turns it (#424): `yaw` radians
    /// toward the body's left about the trunk's up - a rudder - and `pitch`
    /// radians down, a tail depressed into a flare. A value that is not finite
    /// reads as `0` (#427).
    pub fn steer_tail(&self, rig: &Rig, pose: &mut Pose, spread: f32, yaw: f32, pitch: f32) {
        let (spread, yaw, pitch) = (unit(spread), finite(yaw), finite(pitch));
        let Some(trunk) = Trunk::of(rig) else {
            write(&self.tail, pose, spread);
            return;
        };
        if yaw == 0.0 && pitch == 0.0 {
            write(&self.tail, pose, spread);
            return;
        }
        let turn = Rot::axis_angle(trunk.up, yaw)
            * Rot::axis_angle(trunk.along.cross(trunk.up).normalize_or(Vec3::NEG_X), pitch);
        let mut done: Vec<(usize, Quat)> = Vec::with_capacity(self.tail.len());
        for drive in &self.tail {
            let world = (turn * drive.at(spread)).normalize();
            let anchor = drive
                .anchor
                .and_then(|anchor| done.iter().find(|(joint, _)| *joint == anchor))
                .map_or(Quat::IDENTITY, |&(_, world)| world);
            done.push((drive.joint, world));
            if let Some(rotation) = pose.rotations.get_mut(drive.joint) {
                *rotation = (anchor.inverse() * world).normalize();
            }
        }
    }

    /// Whether the wings added `joint` to the rig ([`Self::joints`]).
    #[must_use]
    pub fn owns(&self, joint: usize) -> bool {
        self.joints.binary_search(&joint).is_ok()
    }
}

/// Writes `drives` into `pose` at `extension`: each joint's turn in the
/// body's frame, reckoned against its anchor's turn at the same extension.
/// An extension that is not finite writes the fold ([`unit()`]).
fn write(drives: &[Drive], pose: &mut Pose, extension: f32) {
    let extension = unit(extension);
    let mut turned: Vec<(usize, Quat)> = Vec::with_capacity(drives.len());
    for drive in drives {
        let world = drive.at(extension);
        let anchor = drive
            .anchor
            .and_then(|anchor| turned.iter().find(|(joint, _)| *joint == anchor))
            .map_or(Quat::IDENTITY, |&(_, world)| world);
        turned.push((drive.joint, world));
        if let Some(rotation) = pose.rotations.get_mut(drive.joint) {
            *rotation = (anchor.inverse() * world).normalize();
        }
    }
}

impl Rig {
    /// Folds or spreads this body's wings in `pose` (#421): `0` folded, as
    /// the body rests, to `1` spread, `left` for the wing on the body's left
    /// (`+X`). See [`Wings::pose`]. A body with no wings is left as it was.
    pub fn spread_wings(&self, pose: &mut Pose, left: f32, right: f32) {
        if let Some(wings) = &self.wings {
            wings.pose(pose, left, right);
        }
    }

    /// Closes or fans this body's tail in `pose`, `0` closed to `1` fanned:
    /// a bird's rectrices (#421). A body with no fan is left as it was.
    pub fn spread_tail(&self, pose: &mut Pose, spread: f32) {
        if let Some(wings) = &self.wings {
            wings.fan_tail(pose, spread);
        }
    }

    /// Poses this body's wings at a [`Stroke`] each (#424): see
    /// [`Wings::stroke`]. A body with no wings is left as it was.
    pub fn stroke_wings(&self, pose: &mut Pose, left: Stroke, right: Stroke) {
        if let Some(wings) = &self.wings {
            wings.stroke(self, pose, left, right);
        }
    }

    /// Fans this body's tail and turns it (#424): see [`Wings::steer_tail`].
    /// A body with no fan is left as it was.
    pub fn steer_tail(&self, pose: &mut Pose, spread: f32, yaw: f32, pitch: f32) {
        if let Some(wings) = &self.wings {
            wings.steer_tail(self, pose, spread, yaw, pitch);
        }
    }
}

/// What kind of surface a winged body's wings carry: its leading family's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A bird's flight feathers.
    Feathers,
    /// A bat's membrane, on five digits.
    Bat,
    /// A glider's patagium, from the forefoot to the ankle.
    Glider,
}

impl Kind {
    /// The leading family's.
    pub(crate) fn of(params: &WingedParams) -> Self {
        match params.lineage().leading() {
            WingedFamily::Bird => Kind::Feathers,
            WingedFamily::Bat => Kind::Bat,
            WingedFamily::Glider => Kind::Glider,
        }
    }
}

/// The wing parts a winged body is drawn with, in body space at the fold and
/// skinned to the rig they were grown on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WingParts {
    /// The membranes and a bat's digits, one closed shell per piece, for the
    /// skin's draw: their own texture coordinates, `0..1`, until the build
    /// moves them into the atlas region it reserves for them.
    pub membrane: PolyMesh,
    /// The flight feathers and the tail fan, for the hair's draw: cards cut
    /// from the strand mask, coloured per vertex.
    pub feathers: PolyMesh,
    /// How far out each vertex of [`Self::membrane`] stands along its piece,
    /// `0` where it roots in the body or rides inside a limb, to `1` at its
    /// free edge: what a check that the fold clears the body asks of.
    pub membrane_reach: Vec<f32>,
    /// The same for [`Self::feathers`]: `0` at a quill, `1` at its tip.
    pub feather_reach: Vec<f32>,
}

/// Grows a winged body's wing parts on `rig` (#421): attaches the joints
/// they need, records every drive on [`Rig::wings`], and builds the
/// surfaces. `body` and `weights` are the body's surface and binding, which
/// a membrane roots in.
pub(crate) fn grow(
    rig: &mut Rig,
    body: &PolyMesh,
    weights: &SkinWeights,
    params: &WingedParams,
) -> Option<WingParts> {
    let kind = Kind::of(params);
    let trunk = Trunk::of(rig)?;
    let normals = body.vertex_normals();
    // The plumage every feather is coloured from (#423), on the body as it is
    // before any wing joint is attached: the tail zone's last joint is then
    // still the plan's own, not a fan's edge.
    let read = crate::texture::coat::CoatBody::of(rig, None, None);
    let plumage = crate::texture::coat::PlumageSampler::new(params.plumage, &read);
    let mut book = Book::new(rig.len());
    let mut sides = Vec::new();
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        let Some(arm) = Arm::of(rig, limb) else {
            continue;
        };
        let side = match kind {
            Kind::Feathers => Side::Feathers(feather::wing(
                rig,
                &mut book,
                &trunk,
                &arm,
                (body, &normals),
                &plumage,
            )),
            Kind::Bat => Side::Membrane(Box::new(membrane::bat(rig, &mut book, &trunk, &arm))),
            Kind::Glider => {
                Side::Membrane(Box::new(membrane::glider(rig, &mut book, &trunk, &arm)))
            }
        };
        sides.push((arm, side));
    }
    let tail = (kind == Kind::Feathers)
        .then(|| feather::tail(rig, &mut book, &trunk, params, body, &plumage))
        .flatten();
    rig.wings = Some(book.resolve(rig));

    // A membrane's spars, which follow the arm and the leg spread, so after
    // them; then every drive again.
    if kind != Kind::Feathers {
        // **A glider's arm and leg are held short first** (#434): its
        // membrane hangs from joints that all turn alike, which no blend
        // tears, and its spars are laid out to where the arm and the leg
        // reach spread, so they have to reach there - held short after, as a
        // long-winged cross's are, the patagium's edge stood off its arm.
        if kind == Kind::Glider {
            fit(rig, &blended_skins(weights.vertices.iter()));
        }
        let spread = {
            let mut pose = Pose::rest(rig);
            rig.spread_wings(&mut pose, 1.0, 1.0);
            pose.forward(rig).positions
        };
        for (arm, side) in &mut sides {
            if let Side::Membrane(plan) = side {
                plan.spars(rig, &mut book, &spread, arm, (body, weights));
            }
        }
        let mut wings = book.resolve(rig);
        if let Some(fitted) = rig.wings.as_ref().filter(|_| kind == Kind::Glider) {
            for drive in wings.sides.iter_mut().flat_map(|side| &mut side.drives) {
                if let Some(held) = fitted
                    .sides
                    .iter()
                    .flat_map(|side| &side.drives)
                    .find(|held| held.joint == drive.joint)
                {
                    drive.spread = held.spread;
                }
            }
        }
        rig.wings = Some(wings);
    }

    // Which families the body is and how it goes on the ground (#424), which
    // nothing below rewrites: every winged motion reads them.
    if let Some(wings) = &mut rig.wings {
        let kinship = Kinship::of(params);
        wings.gaits = Gaits::of(params, &kinship);
        wings.kinship = Some(kinship);
    }

    // The surfaces, now every drive is known: a membrane is laid out with the
    // whole body spread. Then the extension is held short of tearing any
    // blend a drawn vertex makes, and the surfaces laid out again if it was.
    let surfaces = |rig: &Rig| {
        let spread = Spread::of(rig);
        let mut parts = WingParts::default();
        for (arm, side) in &sides {
            match side {
                Side::Feathers(plan) => {
                    let (mesh, reach) = plan.mesh();
                    parts.feathers.append(&mesh);
                    parts.feather_reach.extend(reach);
                }
                Side::Membrane(plan) => {
                    let (mesh, reach) = plan.mesh(rig, &spread, body, weights, arm);
                    parts.membrane.append(&mesh);
                    parts.membrane_reach.extend(reach);
                }
            }
        }
        if let Some(tail) = &tail {
            let (mesh, reach) = tail.mesh();
            parts.feathers.append(&mesh);
            parts.feather_reach.extend(reach);
        }
        parts
    };
    let parts = surfaces(rig);
    let blends = blended_skins(
        weights
            .vertices
            .iter()
            .chain(&parts.membrane.skin)
            .chain(&parts.feathers.skin),
    );
    let parts = if fit(rig, &blends) {
        surfaces(rig)
    } else {
        parts
    };
    // How far the folded wing stands off the ground, for a rest (#424).
    let low = parts
        .membrane
        .positions
        .iter()
        .chain(&parts.feathers.positions)
        .map(|at| at.y)
        .fold(f32::MAX, f32::min);
    if let Some(wings) = &mut rig.wings
        && low < f32::MAX
    {
        wings.low = Some(low);
    }
    Some(parts)
}

/// How near a half turn apart two joints a drawn vertex blends may come at
/// any extension, as the least the cosine of half their angle may fall to:
/// 0.05, about 174 degrees - a bat's knuckle, blended across two digits,
/// comes to 160 and holds; the flips the critic found crossed 180.
const BLEND_MARGIN: f32 = 0.05;

/// The least share of its spread turn [`fit`] scales a drive back to: on the
/// families' own bodies it never acts, and the far corners it does (the
/// longest wings on the shortest legs of a glider-led cross) open their
/// patagium partway rather than tear it.
const SHARE_FLOOR: f32 = 0.2;

/// How many steps of the extension the blends are checked at.
const BLEND_STEPS: usize = 96;

/// How many times at most [`fit`] trims a pair's branches.
const ROUNDS: usize = 60;

/// A share of a drive's spread turn held a tenth shorter (#427): a tenth
/// at a time, and at [`SHARE_FLOOR`] at the least - from above the floor a
/// tenth less would undershoot it, to about 0.185.
fn held_short(share: f32) -> f32 {
    (share * 0.9).max(SHARE_FLOOR)
}

/// **The loop [`fit`] runs** (#427): trims the first pair `torn` reports of
/// `state` with `give`, and asks `torn` again after every trim, at most
/// [`ROUNDS`] times. A pair `give` can trim no further - every branch below
/// it at the floor - is passed over from then on, and costs no round: left
/// in, it came back after the next trim and spent another, so several of
/// them ahead of a pair that could be mended ran the loop out before it was.
/// Returns whether anything was trimmed.
fn mend<S, P: Copy + Ord>(
    state: &mut S,
    torn: impl Fn(&S) -> Vec<P>,
    mut give: impl FnMut(&mut S, P) -> bool,
) -> bool {
    let mut spent = std::collections::BTreeSet::new();
    let mut now = torn(state);
    let mut rounds = 0;
    while rounds < ROUNDS {
        let Some(&pair) = now.iter().find(|pair| !spent.contains(*pair)) else {
            break;
        };
        if give(state, pair) {
            rounds += 1;
            now = torn(state);
        } else {
            spent.insert(pair);
        }
    }
    rounds > 0
}

/// Every distinct binding a drawn vertex carries of more than one joint,
/// its shares rounded to a twentieth: what the guard ([`fit`]) blends as
/// skinning does.
fn blended_skins<'a>(skins: impl Iterator<Item = &'a VertexSkin>) -> Vec<Vec<(usize, f32)>> {
    let mut seen = std::collections::BTreeSet::new();
    let mut blends = Vec::new();
    for skin in skins {
        let mut parts: Vec<(usize, f32)> = skin
            .iter()
            .filter(|hold| hold.weight > 0.02)
            .map(|hold| (hold.joint as usize, hold.weight))
            .collect();
        if parts.len() < 2 {
            continue;
        }
        parts.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let key: Vec<(usize, u32)> = parts
            .iter()
            .map(|&(joint, weight)| (joint, (weight * 20.0).round() as u32))
            .collect();
        if seen.insert(key) {
            blends.push(parts);
        }
    }
    blends
}

/// **Keeps every blend whole across the extension** (#421, the critic):
/// walks it from the fold to the spread and, wherever a pair of joints in
/// `blends` - each vertex binding, accumulated as dual-quaternion skinning
/// accumulates it - would bring a part within [`BLEND_MARGIN`] of a half
/// turn from the running sum, where skinning flips it and the surface
/// jumps, scales back the turns on the two branches that part below the
/// pair a tenth at a time, never below [`SHARE_FLOOR`] ([`mend`]). Returns
/// whether anything changed. The test is reckoned in a fixed order
/// ([`det::dot4`]), so a pair an ulp from the margin is trimmed natively and
/// in a browser alike (#427). (Putting a turn on its long arc instead mended a
/// pair and swung the membrane through the trunk on its way.) A long arm on
/// a glider's crouch rests with its
/// elbow nearly shut, and opening it all the way is a half turn for the skin
/// across it whatever arc it takes: such an elbow opens short of straight.
fn fit(rig: &mut Rig, blends: &[Vec<(usize, f32)>]) -> bool {
    let Some(mut wings) = rig.wings.clone() else {
        return false;
    };
    let ancestors = |joint: usize| {
        let mut chain = vec![joint];
        let mut at = rig.joints[joint].parent;
        while let Some(joint) = at {
            chain.push(joint);
            at = rig.joints[joint].parent;
        }
        chain
    };
    // The joints on the two branches that part below a pair, and the joint
    // they part at: turns are scaled in the body's frame, so a child scaled
    // under a parent that is not comes no nearer it.
    let parting = |a: usize, b: usize| -> Vec<usize> {
        let (up_a, up_b) = (ancestors(a), ancestors(b));
        let mut joints: Vec<usize> = up_a
            .iter()
            .filter(|joint| !up_b.contains(joint))
            .chain(up_b.iter().filter(|joint| !up_a.contains(joint)))
            .copied()
            .collect();
        if let Some(&common) = up_a.iter().find(|joint| up_b.contains(joint)) {
            joints.push(common);
        }
        joints
    };
    // Every pair that comes too near a half turn, at any step.
    let torn = |wings: &Wings| -> Vec<(usize, usize)> {
        let mut torn = std::collections::BTreeSet::new();
        for step in 1..=BLEND_STEPS {
            let extension = step as f32 / BLEND_STEPS as f32;
            let mut pose = Pose::rest(rig);
            wings.pose(&mut pose, extension, extension);
            let posed = pose.forward(rig);
            // As dual-quaternion skinning blends: each part against the
            // running sum of those before it, which a part near a half
            // turn from it flips.
            for parts in blends {
                let (first, weight) = parts[0];
                let mut sum = posed.rotations[first] * weight;
                for &(joint, weight) in &parts[1..] {
                    let part = posed.rotations[joint];
                    let agree = det::dot4(sum, part) / det::dot4(sum, sum).sqrt().max(f32::EPSILON);
                    if agree.abs() < BLEND_MARGIN || agree < 0.0 {
                        torn.insert((first, joint));
                    }
                    sum += part * (weight * agree.signum());
                }
            }
        }
        torn.into_iter().collect()
    };
    let drive_of = |wings: &Wings, joint: usize| -> Option<(usize, usize)> {
        wings.sides.iter().enumerate().find_map(|(side, drives)| {
            drives
                .drives
                .iter()
                .position(|drive| drive.joint == joint)
                .map(|at| (side, at))
        })
    };
    let original = wings.clone();
    let mut shares: std::collections::BTreeMap<(usize, usize), f32> =
        std::collections::BTreeMap::new();
    let changed = mend(&mut wings, torn, |wings, (a, b)| {
        let branches: Vec<(usize, usize)> = parting(a, b)
            .into_iter()
            .filter_map(|joint| drive_of(wings, joint))
            .collect();
        // Held short, a tenth at a time.
        let mut gave = false;
        for &(side, at) in &branches {
            let share = shares.entry((side, at)).or_insert(1.0);
            if *share > SHARE_FLOOR {
                *share = held_short(*share);
                gave = true;
                let was = original.sides[side].drives[at].spread;
                let sign = if det::dot4(wings.sides[side].drives[at].spread, was) < 0.0 {
                    -1.0
                } else {
                    1.0
                };
                wings.sides[side].drives[at].spread = arc(was * sign, *share);
            }
        }
        gave
    });
    rig.wings = Some(wings);
    changed
}

/// One wing's plan, before its surface is built.
enum Side {
    Feathers(feather::Wing),
    Membrane(Box<membrane::Wing>),
}

/// The trunk's own frame, which a spread wing is laid out in: `along` from
/// the hip toward the shoulders, `up` out of the back, both in the body's
/// midplane.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Trunk {
    pub along: Vec3,
    pub up: Vec3,
    /// The trunk's length, shoulder to hip, in metres.
    pub length: f32,
}

impl Trunk {
    fn of(rig: &Rig) -> Option<Self> {
        let pelvis = *rig.in_zone(Zone::Pelvis).first()?;
        let withers = *rig.in_zone(Zone::Chest).last()?;
        let run = rig.joints[withers].position - rig.joints[pelvis].position;
        let along = Vec3::new(0.0, run.y, run.z).normalize_or(Vec3::Z);
        let up = (Vec3::Y - along * along.y).normalize_or(Vec3::Y);
        Some(Self {
            along,
            up,
            length: run.length(),
        })
    }

    /// The plane a spread wing lies in, on the side `side` (`+1` the left),
    /// raised `dihedral` radians out of the trunk's own.
    pub(crate) fn plane(&self, side: f32, dihedral: f32) -> Plane {
        let lateral = Vec3::new(side, 0.0, 0.0);
        let (sin, cos) = det::sin_cos(dihedral);
        Plane {
            out: lateral * cos + self.up * sin,
            forward: self.along,
            dorsal: self.up * cos - lateral * sin,
        }
    }
}

/// A spread wing's plane: `out` from the body, `forward` along it, `dorsal`
/// its upper side's normal.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plane {
    pub out: Vec3,
    pub forward: Vec3,
    pub dorsal: Vec3,
}

impl Plane {
    /// The direction `theta` radians from straight out toward forward:
    /// negative angles sweep back, toward the trailing edge.
    pub(crate) fn dir(&self, theta: f32) -> Vec3 {
        let (sin, cos) = det::sin_cos(theta);
        self.out * cos + self.forward * sin
    }
}

/// Turns `v` in the plane whose dorsal normal is `normal`, `angle` radians
/// toward the leading edge on the side `side`: the same sense as
/// [`Plane::dir`]'s angle on a wing of that side, folded or spread.
pub(crate) fn swing(v: Vec3, normal: Vec3, side: f32, angle: f32) -> Vec3 {
    Rot::axis_angle((normal * -side).normalize_or(Vec3::Y), angle) * v
}

/// A right-handed frame from a direction and a normal: the direction, the
/// normal made square to it, and their cross.
pub(crate) fn frame(direction: Vec3, normal: Vec3) -> Mat3 {
    let d = direction.normalize_or(Vec3::Z);
    let n = (normal - d * normal.dot(d)).normalize_or(d.any_orthonormal_vector());
    Mat3::from_cols(d, n, d.cross(n))
}

/// The rotation carrying the frame `rest` onto `spread`.
pub(crate) fn turn(rest: Mat3, spread: Mat3) -> Quat {
    Quat::from_mat3(&(spread * rest.transpose())).normalize()
}

/// One fore limb's arm, as its plan hangs it: the shoulder, the elbow and
/// the wrist, the hand's two nodes after them, and the side it is on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arm {
    pub limb: Limb,
    pub side: f32,
    pub shoulder: usize,
    pub elbow: usize,
    pub wrist: usize,
    pub palm: usize,
    pub tip: usize,
}

impl Arm {
    fn of(rig: &Rig, limb: Limb) -> Option<Self> {
        let upper = rig.in_zone(Zone::UpperLimb(limb));
        let lower = rig.in_zone(Zone::LowerLimb(limb));
        let hand = rig.in_zone(Zone::Extremity(limb));
        let [.., shoulder, elbow] = upper[..] else {
            return None;
        };
        let (&wrist, &palm, &tip) = (lower.first()?, hand.first()?, hand.get(1)?);
        Some(Self {
            limb,
            side: if limb.is_left() { 1.0 } else { -1.0 },
            shoulder,
            elbow,
            wrist,
            palm,
            tip,
        })
    }

    /// Where a joint of the rig rests.
    pub(crate) fn at(rig: &Rig, joint: usize) -> Vec3 {
        rig.joints[joint].position
    }

    /// The arm's own plane at rest, as a dorsal normal: the one its spread
    /// counterpart turns onto the wing's upper side, read off the sense the
    /// elbow bends in, which a folded arm and a spread one share.
    pub(crate) fn rest_normal(&self, rig: &Rig) -> Vec3 {
        let (s, e, w) = (
            Self::at(rig, self.shoulder),
            Self::at(rig, self.elbow),
            Self::at(rig, self.wrist),
        );
        ((e - s).cross(w - e) * -self.side).normalize_or(Vec3::new(self.side, 0.0, 0.0))
    }
}

/// Every joint a wing turns, with the rotation it has in the world with the
/// wing spread.
pub(crate) struct Book {
    world: Vec<Option<Entry>>,
    /// The rig's length before the wings grew: every joint from here on is
    /// one of theirs ([`Wings::joints`]).
    first: usize,
}

/// One joint's entry in a [`Book`]: its rotation in the world spread, the
/// share of the extension it moves over, and the wing it belongs to (`None`
/// the tail).
type Entry = (Quat, f32, f32, Option<Limb>);

impl Book {
    fn new(joints: usize) -> Self {
        Self {
            world: vec![None; joints],
            first: joints,
        }
    }

    /// Records that `joint`, of the wing on `limb` (or of the tail, `None`),
    /// turns by `world` in the world when spread, over `from..to` of the
    /// extension.
    pub(crate) fn turn(
        &mut self,
        joint: usize,
        world: Quat,
        schedule: (f32, f32),
        limb: Option<Limb>,
    ) {
        if self.world.len() <= joint {
            self.world.resize(joint + 1, None);
        }
        self.world[joint] = Some((world, schedule.0, schedule.1, limb));
    }

    /// The rotation `joint` has in the world spread, as far as the book
    /// knows: its own, or its nearest named ancestor's.
    pub(crate) fn world_of(&self, rig: &Rig, joint: usize) -> Quat {
        let mut at = Some(joint);
        while let Some(joint) = at {
            if let Some((world, ..)) = self.world.get(joint).copied().flatten() {
                return world;
            }
            at = rig.joints[joint].parent;
        }
        Quat::IDENTITY
    }

    /// Attaches a joint of the wing's own to `rig`, zoned as `zone`, and
    /// returns it.
    pub(crate) fn attach(rig: &mut Rig, parent: usize, at: Vec3, zone: Zone) -> usize {
        let joint = rig
            .attach(parent, at, Role::Digit)
            .expect("a wing hangs from a joint of its own rig");
        rig.joints[joint].zone = zone;
        joint
    }

    /// Every drive, each with the nearest driven joint above it as its
    /// anchor, and every wing's drives signed alike ([`sign`]).
    fn resolve(&self, rig: &Rig) -> Wings {
        let mut wings = Wings {
            joints: (self.first..rig.len()).collect(),
            ..Wings::default()
        };
        for joint in 0..rig.len() {
            let Some((target, from, to, limb)) = self.world.get(joint).copied().flatten() else {
                continue;
            };
            let mut anchor = rig.joints[joint].parent;
            while let Some(at) = anchor {
                if self.world.get(at).copied().flatten().is_some() {
                    break;
                }
                anchor = rig.joints[at].parent;
            }
            let drive = Drive {
                joint,
                spread: target.normalize(),
                anchor,
                from,
                to,
            };
            match limb {
                Some(limb) => match wings.sides.iter_mut().find(|s| s.limb == limb) {
                    Some(side) => side.drives.push(drive),
                    None => wings.sides.push(WingSide {
                        limb,
                        drives: vec![drive],
                    }),
                },
                None => wings.tail.push(drive),
            }
        }
        for side in &mut wings.sides {
            sign(&mut side.drives);
        }
        sign(&mut wings.tail);
        wings
    }
}

/// Each drive the shortest arc from the fold: a quaternion's sign picks the
/// arc its turn travels (`Drive::at`). Signed instead to agree with the
/// wing's mean turn, two neighbouring spars' near-half turns left the fold
/// on opposite arcs and passed a half turn apart midway.
fn sign(drives: &mut [Drive]) {
    for drive in drives.iter_mut() {
        if drive.spread.w < 0.0 {
            drive.spread = -drive.spread;
        }
    }
}

/// The skinning transforms of a body with both wings spread and its tail
/// fanned: what a membrane is laid out against.
pub(crate) struct Spread {
    transforms: Vec<DualQuat>,
}

impl Spread {
    fn of(rig: &Rig) -> Self {
        let mut pose = Pose::rest(rig);
        rig.spread_wings(&mut pose, 1.0, 1.0);
        Self {
            transforms: pose.forward(rig).skinning_transforms(rig),
        }
    }

    fn blended(&self, hold: &VertexSkin) -> DualQuat {
        dual::blend(
            hold.iter()
                .filter(|influence| influence.weight > 0.0)
                .map(|influence| (self.transforms[influence.joint as usize], influence.weight)),
        )
    }

    /// Where a point resting at `rest`, held by `hold`, stands spread.
    pub(crate) fn forward(&self, hold: &VertexSkin, rest: Vec3) -> Vec3 {
        self.blended(hold).transform_point(rest)
    }

    /// Where a point standing at `spread` when spread, held by `hold`,
    /// rests, and which way a direction there faced: the inverse of its own
    /// binding.
    pub(crate) fn inverse(&self, hold: &VertexSkin, spread: Vec3, normal: Vec3) -> (Vec3, Vec3) {
        let blended = self.blended(hold);
        let back = blended.rotation().inverse();
        (back * (spread - blended.translation()), back * normal)
    }
}

/// A binding before it is cut to the four influences a vertex carries: any
/// number of joints, each with a share.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Hold(pub Vec<(usize, f32)>);

impl Hold {
    /// Held wholly by one joint.
    pub(crate) fn one(joint: usize) -> Self {
        Self(vec![(joint, 1.0)])
    }

    /// A vertex's binding, as it stands.
    pub(crate) fn of(skin: &VertexSkin) -> Self {
        Self(
            skin.iter()
                .filter(|influence| influence.weight > 0.0)
                .map(|influence| (influence.joint as usize, influence.weight))
                .collect(),
        )
    }

    /// The sum of `parts`, each scaled by its weight, joint by joint.
    pub(crate) fn mix(parts: &[(&Hold, f32)]) -> Self {
        let mut sum: Vec<(usize, f32)> = Vec::new();
        for (hold, weight) in parts {
            for &(joint, share) in &hold.0 {
                match sum.iter_mut().find(|(at, _)| *at == joint) {
                    Some(entry) => entry.1 += share * weight,
                    None => sum.push((joint, share * weight)),
                }
            }
        }
        Self(sum)
    }

    /// Cut to [`MAX_INFLUENCES`]: the largest shares, none negative,
    /// summing to one. Ties keep the joint listed first.
    pub(crate) fn cut(&self) -> VertexSkin {
        let mut shares: Vec<(usize, f32)> =
            self.0.iter().copied().filter(|&(_, w)| w > 1e-6).collect();
        shares.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        shares.truncate(MAX_INFLUENCES);
        let total: f32 = shares.iter().map(|&(_, w)| w).sum();
        let mut skin = VertexSkin::default();
        if total <= f32::EPSILON {
            return skin;
        }
        for (slot, (joint, weight)) in shares.into_iter().enumerate() {
            skin[slot] = Influence {
                joint: joint as u16,
                weight: weight / total,
            };
        }
        skin
    }
}

/// A point on a chain of joints, `t` of the way from `joints[bone]` to the
/// next, held as the crate holds a bone (by its proximal joint) and shared
/// with the bone either side across each joint, so a chain bends rather
/// than hinging its surface.
///
/// **Across a joint a helper splits** (`helpers`, #424: a bat's knuckles),
/// the surface is shared with the helper instead of the bone beyond: wholly
/// the helper's at the joint, the bone's own by the bone's middle. The
/// helper turns halfway between the two bones, so no vertex blends two
/// turns more than half a knuckle's fold apart, and a hand folded flat
/// skins without flipping.
pub(crate) fn on_chain(
    rig: &Rig,
    joints: &[usize],
    bone: usize,
    t: f32,
    helpers: &[Helper],
) -> (Vec3, Hold) {
    let a = Arm::at(rig, joints[bone]);
    let b = Arm::at(rig, joints[bone + 1]);
    let at = a + (b - a) * t;
    let driver = joints[bone];
    let helper = |from: usize, to: usize| {
        helpers
            .iter()
            .find(|helper| helper.from == from && helper.to == to)
            .map(|helper| helper.joint)
    };
    if t < 0.5
        && bone > 0
        && let Some(joint) = helper(joints[bone - 1], driver)
    {
        let share = (1.0 - 2.0 * t) * (1.0 - 2.0 * t);
        return (at, Hold(vec![(joint, share), (driver, 1.0 - share)]));
    }
    if t > 0.5
        && bone + 2 < joints.len()
        && let Some(joint) = helper(driver, joints[bone + 1])
    {
        let share = (2.0 * t - 1.0) * (2.0 * t - 1.0);
        return (at, Hold(vec![(driver, 1.0 - share), (joint, share)]));
    }
    let hold = if t < 0.5 && bone > 0 {
        let share = 0.5 * (1.0 - 2.0 * t) * (1.0 - 2.0 * t);
        Hold(vec![(joints[bone - 1], share), (driver, 1.0 - share)])
    } else if t > 0.5 && bone + 2 < joints.len() {
        let share = 0.5 * (2.0 * t - 1.0) * (2.0 * t - 1.0);
        Hold(vec![(driver, 1.0 - share), (joints[bone + 1], share)])
    } else {
        Hold::one(driver)
    };
    (at, hold)
}

/// `count` points along a chain of joints, evenly bone by bone - each bone
/// the same share of the run whatever its length - root to tip: so two
/// chains of as many bones, sampled alike, meet bone to bone, and a strip
/// between two digits never blends one's phalanx with the other's
/// metacarpal, whose turns can lie more than a half turn apart.
pub(crate) fn along_bones(
    rig: &Rig,
    joints: &[usize],
    count: usize,
    helpers: &[Helper],
) -> Vec<(Vec3, Hold)> {
    let bones = joints.len() - 1;
    (0..count)
        .map(|index| {
            let at = bones as f32 * index as f32 / (count - 1).max(1) as f32;
            let bone = (at.floor() as usize).min(bones - 1);
            on_chain(
                rig,
                joints,
                bone,
                (at - bone as f32).clamp(0.0, 1.0),
                helpers,
            )
        })
        .collect()
}

/// A joint splitting the turn across another (#424): `joint` rests where
/// `to` does, hangs from `from`, and turns halfway between the two, so a
/// surface shared across the joint blends no two turns more than half the
/// joint's fold apart ([`on_chain`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Helper {
    pub from: usize,
    pub to: usize,
    pub joint: usize,
}

/// `count` points along a chain of joints, evenly by length, root to tip.
pub(crate) fn along_chain(rig: &Rig, joints: &[usize], count: usize) -> Vec<(Vec3, Hold)> {
    let lengths: Vec<f32> = joints
        .windows(2)
        .map(|pair| Arm::at(rig, pair[0]).distance(Arm::at(rig, pair[1])))
        .collect();
    let total: f32 = lengths.iter().sum::<f32>().max(f32::EPSILON);
    (0..count)
        .map(|index| {
            let mut left = total * index as f32 / (count - 1).max(1) as f32;
            let mut bone = 0;
            while bone + 1 < lengths.len() && left > lengths[bone] {
                left -= lengths[bone];
                bone += 1;
            }
            let t = (left / lengths[bone].max(f32::EPSILON)).clamp(0.0, 1.0);
            on_chain(rig, joints, bone, t, &[])
        })
        .collect()
}

#[cfg(test)]
mod tests;
