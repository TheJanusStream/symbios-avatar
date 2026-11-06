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
/// cross and appeal and 100 rolls).
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
    /// wherever a blended pair would come that close.
    #[must_use]
    pub fn at(&self, extension: f32) -> Quat {
        let span = (self.to - self.from).max(f32::EPSILON);
        let share = ((extension - self.from) / span).clamp(0.0, 1.0);
        arc(self.spread, share * share * (3.0 - 2.0 * share))
    }
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

    /// Writes the tail fan into `pose`, `0` closed to `1` fanned.
    pub fn fan_tail(&self, pose: &mut Pose, spread: f32) {
        write(&self.tail, pose, spread);
    }

    /// Whether the wings added `joint` to the rig ([`Self::joints`]).
    #[must_use]
    pub fn owns(&self, joint: usize) -> bool {
        self.joints.binary_search(&joint).is_ok()
    }
}

/// Writes `drives` into `pose` at `extension`: each joint's turn in the
/// body's frame, reckoned against its anchor's turn at the same extension.
fn write(drives: &[Drive], pose: &mut Pose, extension: f32) {
    let extension = extension.clamp(0.0, 1.0);
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
            Kind::Bat => Side::Membrane(membrane::bat(rig, &mut book, &trunk, &arm)),
            Kind::Glider => Side::Membrane(membrane::glider(rig, &mut book, &trunk, &arm)),
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
        let spread = {
            let mut pose = Pose::rest(rig);
            rig.spread_wings(&mut pose, 1.0, 1.0);
            pose.forward(rig).positions
        };
        for (arm, side) in &mut sides {
            if let Side::Membrane(plan) = side {
                plan.spars(rig, &mut book, &spread, arm, body);
            }
        }
        rig.wings = Some(book.resolve(rig));
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
    if fit(rig, &blends) {
        return Some(surfaces(rig));
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
/// pair a tenth at a time, never below [`SHARE_FLOOR`]. Returns whether anything
/// changed. (Putting a turn on its long arc instead mended a pair and swung
/// the membrane through the trunk on its way.) A long arm on a glider's crouch rests with its
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
                    let agree = sum.dot(part) / sum.length().max(f32::EPSILON);
                    if agree.abs() < BLEND_MARGIN || agree < 0.0 {
                        torn.insert((first, joint));
                    }
                    sum += part * (weight * agree.signum());
                }
            }
        }
        torn.into_iter().collect()
    };
    let drive_of = |wings: &mut Wings, joint: usize| -> Option<(usize, usize)> {
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
    let mut changed = false;
    let mut now = torn(&wings);
    for _ in 0..60 {
        let Some(&(a, b)) = now.first() else {
            break;
        };
        changed = true;
        let branches: Vec<(usize, usize)> = parting(a, b)
            .into_iter()
            .filter_map(|joint| drive_of(&mut wings, joint))
            .collect();
        // Held short, a tenth at a time.
        let mut gave = false;
        for &(side, at) in &branches {
            let share = shares.entry((side, at)).or_insert(1.0);
            if *share > SHARE_FLOOR {
                *share *= 0.9;
                gave = true;
                let was = original.sides[side].drives[at].spread;
                let sign = if wings.sides[side].drives[at].spread.dot(was) < 0.0 {
                    -1.0
                } else {
                    1.0
                };
                wings.sides[side].drives[at].spread = arc(was * sign, *share);
            }
        }
        if !gave {
            now.remove(0);
            continue;
        }
        now = torn(&wings);
    }
    rig.wings = Some(wings);
    changed
}

/// One wing's plan, before its surface is built.
enum Side {
    Feathers(feather::Wing),
    Membrane(membrane::Wing),
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
pub(crate) fn on_chain(rig: &Rig, joints: &[usize], bone: usize, t: f32) -> (Vec3, Hold) {
    let a = Arm::at(rig, joints[bone]);
    let b = Arm::at(rig, joints[bone + 1]);
    let at = a + (b - a) * t;
    let driver = joints[bone];
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
pub(crate) fn along_bones(rig: &Rig, joints: &[usize], count: usize) -> Vec<(Vec3, Hold)> {
    let bones = joints.len() - 1;
    (0..count)
        .map(|index| {
            let at = bones as f32 * index as f32 / (count - 1).max(1) as f32;
            let bone = (at.floor() as usize).min(bones - 1);
            on_chain(rig, joints, bone, (at - bone as f32).clamp(0.0, 1.0))
        })
        .collect()
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
            on_chain(rig, joints, bone, t)
        })
        .collect()
}

#[cfg(test)]
mod tests;
