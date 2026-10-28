//! Resting on the ground (#377): a beast's sit and its lying down, posed from
//! where its body meets the ground and moved into and out of in the order the
//! animal moves.
//!
//! **Solved, not shared out.** P5 pitched a sitting trunk a fixed share of a
//! quarter turn and dropped a lying one a fixed share of its leg, and nothing
//! asked where the skin met the ground: over the roll's range the skin went
//! up to 413 mm into it and a lying horse's chest stood 194 mm off it. Here
//! every frame poses the whole body from where its supports rest: the trunk,
//! the legs, the tail along the ground and the head carried level, from rest,
//! so a rest owns the body while it lasts. The skin is read off the rig's
//! [`crate::rig::Outline`], the drawn skin's own lowest points; a rig rooted
//! straight from a skeleton has none, and its joints' rings stand in.
//!
//! **A sit** is solved from the forehand back: forelegs straight over paws
//! that stay, the chest pitched until the haunches touch, the hind legs folded
//! down from the hips onto flat metatarsi, and the hind paws stepped forward
//! to wherever that leaves them, before the body moves. **Lying down** goes
//! through the sit for a dog or a cat and ends in the sphinx, its sternum and
//! its back touching together and each foreleg fitted to its own skin; a
//! horse gathers its feet, drops onto its knees and folds its hindquarters
//! after, rolled onto a hip. **Getting up** is the animal's order, not the
//! way down played back. `docs/quadruped-anatomy.md` has the measurements
//! and where the slice departs from the research.
//!
//! **Whatever still sinks rises**: the trunk by what its own skin sinks, its
//! legs reaching down again, and the whole body with its feet by what a leg's
//! own skin sinks, where the skin blended across a closed joint folds in.
//!
//! **Continuous, and the same however it got there.** Each motion is a
//! continuous function of its time: the lying down goes on from exactly
//! where the sit ends, and no solve falls back to the end of a range or flips
//! a bend. A rest poses every joint it owns from rest - all but the head's
//! ears and jaw - so a moment reached two ways is posed one way, and what it
//! solves from the rig alone is kept between frames by whoever plays it
//! ([`Rests`]): a held sit costs what an idle frame does.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Quat, Vec3};

use super::gait::{relevel_head, spine_to};
use super::ground::{Foot, solve_contact_with};
use super::ik::two_bone;
use super::pose::{Pose, Posed};
use crate::det::{DetMath, Rot};
use crate::plan::{FootKind, Limb, Zone};
use crate::rig::{Rig, Role};

/// Which rest a body goes into, or gets up from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Rest {
    /// On its haunches: [`super::clip::Target::Sit`].
    Sit,
    /// On its sternum: [`super::clip::Target::Lie`].
    Lie,
}

/// Where the ground is, in the space a rig rests in: the plan stands every
/// body's soles on it.
const GROUND: f32 = 0.0;

/// One leg, as a rest folds it.
#[derive(Clone, Copy, Debug)]
struct Leg {
    limb: Limb,
    /// Hip, stifle and hock; or shoulder, elbow and carpus.
    chain: [usize; 3],
    /// The joint the metatarsus or the cannon ends at, the leg's contact.
    pad: usize,
    /// The toes hanging off it.
    toe: Option<usize>,
}

/// The parts of a four-legged body a rest moves, read off its rig.
#[derive(Clone, Debug)]
struct Body {
    root: usize,
    /// The trunk from the root's first child to the withers.
    spine: Vec<usize>,
    withers: usize,
    hinds: Vec<Leg>,
    fores: Vec<Leg>,
    tail: Vec<usize>,
}

impl Body {
    fn read(rig: &Rig) -> Option<Self> {
        let root = rig.joints.iter().position(|joint| joint.parent.is_none())?;
        let withers = rig.joints[*rig.in_zone(Zone::Neck).first()?].parent?;
        let spine = spine_to(rig, withers);
        let mut hinds = Vec::new();
        let mut fores = Vec::new();
        for limb in rig.ground_contacts() {
            let chain = rig.limb_chain(limb)?;
            let pad = *rig.in_zone(Zone::Extremity(limb)).first()?;
            let toe = rig
                .in_zone(Zone::Extremity(limb))
                .into_iter()
                .find(|&joint| rig.joints[joint].parent == Some(pad));
            let leg = Leg {
                limb,
                chain,
                pad,
                toe,
            };
            if limb.is_fore() {
                fores.push(leg);
            } else {
                hinds.push(leg);
            }
        }
        (hinds.len() == 2 && fores.len() == 2 && !spine.is_empty()).then(|| Self {
            root,
            spine,
            withers,
            hinds,
            fores,
            tail: rig.in_zone(Zone::Tail),
        })
    }
}

/// The angle of `run` in the body's side plane, counter-clockwise from
/// forward toward up.
fn angle_of(run: Vec3) -> f32 {
    run.y.det_atan2(run.z)
}

/// The unit run at `angle` in the side plane.
fn along(angle: f32) -> Vec3 {
    Vec3::new(0.0, angle.det_sin(), angle.det_cos())
}

/// The turn that carries a run in the side plane `angle` radians
/// counter-clockwise, forward toward up: nose up, for a trunk.
fn turn(angle: f32) -> Quat {
    Rot::x(-angle)
}

/// How long `run` is in the side plane.
fn side_length(run: Vec3) -> f32 {
    (run.y * run.y + run.z * run.z).sqrt()
}

/// A smooth step from 0 at `from` to 1 at `to`.
fn step(from: f32, to: f32, at: f32) -> f32 {
    let t = ((at - from) / (to - from).max(f32::EPSILON)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A hind leg's shape in its side plane.
#[derive(Clone, Copy, Debug)]
struct Fold {
    /// The metatarsus, hock to pad, in radians counter-clockwise off forward.
    meta: f32,
    /// The hock's included angle, metatarsus to tibia.
    hock: f32,
    /// The stifle's included angle, tibia to femur.
    stifle: f32,
}

/// A hind leg read off the rig at rest: its bones' lengths in the side plane
/// and the fold it stands in.
#[derive(Clone, Copy, Debug)]
struct HindLeg {
    femur: f32,
    tibia: f32,
    meta: f32,
    stood: Fold,
}

impl HindLeg {
    fn read(rig: &Rig, leg: &Leg) -> Self {
        let at = |joint: usize| rig.joints[joint].position;
        let [hip, stifle, hock] = leg.chain.map(at);
        let pad = at(leg.pad);
        let meta = angle_of(pad - hock);
        let tibia = angle_of(stifle - hock);
        let femur = angle_of(hip - stifle);
        Self {
            femur: side_length(stifle - hip),
            tibia: side_length(stifle - hock),
            meta: side_length(pad - hock),
            stood: Fold {
                meta,
                hock: (tibia - meta).rem_euclid(2.0 * PI),
                stifle: (tibia + PI - femur).rem_euclid(2.0 * PI),
            },
        }
    }

    /// Where the hock, the stifle and the hip stand over a pad at `pad`
    /// folded as `fold` says, in the side plane, and the femur's angle.
    fn up_from(&self, pad: Vec3, fold: Fold) -> ([Vec3; 3], f32) {
        let hock = pad - along(fold.meta) * self.meta;
        let tibia = fold.meta + fold.hock;
        let stifle = hock + along(tibia) * self.tibia;
        let femur = tibia + PI - fold.stifle;
        let hip = stifle + along(femur) * self.femur;
        ([hip, stifle, hock], femur + PI)
    }
}

/// The lowest skin a meshed joint's ring is expected to reach in `posed`: an
/// ellipse its section's size across the bone that ends at it, turned with
/// that bone. The root carries a cap behind it as far as its radius.
fn ring_low(rig: &Rig, posed: &Posed, joint: usize) -> Option<f32> {
    let it = &rig.joints[joint];
    if it.marker || it.role != Role::Deform {
        return None;
    }
    let (bone, turned, cap) = match it.parent {
        Some(parent) => (
            it.position - rig.joints[parent].position,
            posed.rotations[parent],
            0.0,
        ),
        None => {
            // Toward whichever child carries the trunk on.
            let child = (0..rig.len()).find(|&child| {
                rig.joints[child].parent == Some(joint) && rig.joints[child].zone.is_core()
            })?;
            (
                rig.joints[child].position - it.position,
                posed.rotations[joint],
                it.radius,
            )
        }
    };
    let bone = bone.normalize_or(Vec3::Z);
    let deep = bone.cross(Vec3::X).normalize_or(Vec3::Y);
    let (across, deep, bone) = ((turned * Vec3::X).y, (turned * deep).y, (turned * bone).y);
    let (wide, tall) = (it.radius * it.scale.x, it.radius * it.scale.y);
    let reach = ((wide * across).powi(2) + (tall * deep).powi(2) + (cap * bone).powi(2)).sqrt();
    Some(posed.positions[joint].y - reach)
}

/// The rig's outline of its skin, if it has one with any skin in it: an
/// empty one - [`crate::rig::Outline::default`], or one measured over no
/// weights - is none, and the rings stand in.
fn outline(rig: &Rig) -> Option<&crate::rig::Outline> {
    rig.outline.as_ref().filter(|outline| !outline.is_empty())
}

/// The lowest skin reaches in `posed` in each of `N` slots at once, a joint's
/// skin in the slot `slot` gives it: read off the rig's outline of its own
/// skin where it has one, and off its joints' rings for a slot it holds no
/// skin for. One reading of a pose for every question a solve asks of it.
fn lowest_each<const N: usize>(
    rig: &Rig,
    posed: &Posed,
    slot: impl Fn(usize) -> Option<usize>,
) -> [f32; N] {
    let mut lows = [f32::MAX; N];
    if let Some(outline) = outline(rig) {
        outline.lowest_each(rig, posed, &slot, &mut lows);
    }
    // Nothing of the outline there: an empty slot would read as a skin
    // infinitely far down, and a trunk set on it as infinitely far up.
    let bare = lows.map(|low| low == f32::MAX);
    if bare.contains(&true) {
        for joint in 0..rig.len() {
            if let Some(into) = slot(joint).filter(|&into| into < N && bare[into])
                && let Some(low) = ring_low(rig, posed, joint)
            {
                lows[into] = lows[into].min(low);
            }
        }
    }
    lows
}

/// The lowest skin held by joints in `zones` reaches in `posed`: see
/// [`lowest_each`].
fn lowest_in(rig: &Rig, posed: &Posed, zones: impl Fn(Zone) -> bool) -> f32 {
    lowest_each::<1>(rig, posed, |joint| {
        zones(rig.joints[joint].zone).then_some(0)
    })[0]
}

/// How far a sitting beast's loin rounds, as a share of the pitch its chest
/// takes: the pelvis pitches that much further than the ribs - a dog's or a
/// cat's supple loin, a horse's stiff one.
/// Provenance: **tuned by render** (#377), after 09 (2.1.1): the pelvis
/// pitches with the femur, the loin rounds, the chest keeps its shape.
const SIT_LOIN: [f32; 2] = [1.0, 0.3];

/// How far a sitting beast's femur slopes down from its hip to its stifle,
/// in radians: near level, the stifle beside the flank.
/// Provenance: **the research's** (09, 2.1.1): the femur near horizontal.
const SIT_FEMUR: f32 = 0.1;

/// How far a sitting beast's forelegs lean back from its wrists to its
/// shoulders, in radians: a dog's or a cat's forehand a little back over
/// paws that stay; a horse's hooves planted out ahead of its chest.
/// Provenance: **the research's** (09, 2.1.2 and 2.3.4): the forehand moves
/// slightly back and up over the forepaws; a rising horse extends its
/// forelegs in front of its chest. How far, **tuned by render** (#377).
const SIT_LEAN: [f32; 2] = [0.12, 0.4];

/// Which column of a two-way constant a body reads: paws, or hooves.
fn hooved(rig: &Rig) -> usize {
    usize::from(rig.anatomy.feet == FootKind::Hooves)
}

/// How far the forelegs of a resting beast stand closer than they do
/// standing, shoulder to wrist: straight, and not locked.
const FORE_GIVE: f32 = 0.99;

/// How high a stepping paw lifts, in its leg's reach.
const STEP_LIFT: f32 = 0.08;

/// A sit's end, solved from the forehand back.
#[derive(Clone, Copy, Debug)]
struct Sat {
    /// The pelvis's pitch, nose up.
    pelvis: f32,
    /// The hind legs' fold.
    fold: Fold,
    /// How far forward the hind pads come.
    step: f32,
}

/// Turns the trunk: the pelvis `pelvis` nose up, the chest `chest`, the loin
/// rounding between them.
fn trunk(body: &Body, pose: &mut Pose, pelvis: f32, chest: f32) {
    rolled(body, pose, pelvis, chest, 0.0);
}

/// [`trunk`], the pelvis rolled `roll` radians onto its left hip and the
/// loin twisting most of that back out of the chest.
fn rolled(body: &Body, pose: &mut Pose, pelvis: f32, chest: f32, roll: f32) {
    pose.rotations[body.root] = turn(pelvis) * Rot::z(-roll);
    let twist = roll * (1.0 - LIE_THORAX) / 2.0;
    for (index, joint) in body.spine.iter().enumerate() {
        pose.rotations[*joint] = if index < 2 {
            turn(-(pelvis - chest) / 2.0) * Rot::z(twist)
        } else {
            Quat::IDENTITY
        };
    }
}

/// Where a sitting beast's left shoulder stands: over its planted wrist, a
/// straight leg's length off it, leaning back by [`SIT_LEAN`].
fn shoulder_over_wrist(rig: &Rig, body: &Body) -> Vec3 {
    let lean = SIT_LEAN[hooved(rig)];
    let leg = &body.fores[0];
    let at = |joint: usize| rig.joints[joint].position;
    let (shoulder, wrist) = (at(leg.chain[0]), at(leg.chain[2]));
    let length = shoulder.distance(wrist) * FORE_GIVE;
    let mut over = wrist + along(FRAC_PI_2 + lean) * length;
    over.x = shoulder.x;
    over
}

/// The end of a sit: the chest pitched until the haunches touch the ground
/// behind forelegs that stand, the hind legs folded down from the hips onto
/// metatarsi laid flat, and how far that brings the hind pads forward.
fn sat(rig: &Rig, body: &Body, hind: &HindLeg) -> Sat {
    let target = shoulder_over_wrist(rig, body);
    let shoulder = body.fores[0].chain[0];
    let loin = 1.0 + SIT_LOIN[hooved(rig)];
    let pad = rig.joints[body.hinds[0].pad].position;
    // The body sat with its chest pitched `chest`, its hind legs folded
    // down from where that leaves the hips: the hock on the ground at the
    // pad's own height, the femur sloping down to the stifle, the hock
    // closing as far as the hip is high.
    let seated = |chest: f32| {
        let mut pose = Pose::rest(rig);
        trunk(body, &mut pose, chest * loin, chest);
        pose.translation = target - pose.forward(rig).positions[shoulder];
        let hip = pose.forward(rig).positions[body.hinds[0].chain[0]];
        let fold = folded_under(hind, hip.y - pad.y, SIT_FEMUR, HOCK_CLOSED[hooved(rig)]);
        let ([from, _, _], _) = hind.up_from(Vec3::ZERO, fold);
        let step = hip.z - from.z - pad.z;
        for leg in &body.hinds {
            let at = rig.joints[leg.pad].position + Vec3::Z * step;
            fold_hind(rig, &mut pose, leg, hind, fold, at);
        }
        let sat = Sat {
            pelvis: chest * loin,
            fold,
            step,
        };
        (sat, haunch(rig, &pose.forward(rig)))
    };
    let (mut low, mut high) = (0.0f32, 1.0f32);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        if seated(middle).1 > GROUND {
            low = middle;
        } else {
            high = middle;
        }
    }
    seated(high).0
}

/// How far a hock may close, in radians, paws and hooves: past it the skin
/// blended across the joint folds into itself.
/// Provenance: **the research's** (09, 2.0 and 2.3.1): a dog's tarsus
/// flexes to 39 degrees and a cat's to 21, a lying horse's hock to 40 to
/// 70; measured (#377), a horse's closed to 15 degrees sank the back of its
/// cannon 38 mm into the ground, to 34 still 31, and to 49 still 22.
const HOCK_CLOSED: [f32; 2] = [0.6, 1.0];

/// A hind leg folded under a hip `rise` above its pad, the metatarsus flat
/// on the ground: the femur sloping `femur` radians down to the stifle and
/// the hock closing as far as that leaves the hip high, and where a hock
/// closed to [`HOCK_CLOSED`] still stands the hip too high, the stifle
/// rising toward it instead.
fn folded_under(hind: &HindLeg, rise: f32, femur: f32, closed: f32) -> Fold {
    let hock = ((rise - hind.femur * femur.det_sin()) / hind.tibia)
        .clamp(-1.0, 1.0)
        .det_asin();
    let (hock, femur) = if hock >= closed {
        (hock, femur)
    } else {
        let left = rise - hind.tibia * closed.det_sin();
        (closed, (left / hind.femur).clamp(-1.0, 1.0).det_asin())
    };
    Fold {
        meta: 0.0,
        hock,
        stifle: hock + femur,
    }
}

/// The lowest skin of the haunches: the pelvis and the thighs.
fn haunch(rig: &Rig, posed: &Posed) -> f32 {
    lowest_in(rig, posed, |zone| {
        zone == Zone::Pelvis || matches!(zone, Zone::UpperLimb(limb) if !limb.is_fore())
    })
}

/// Places the root over hind legs folded as `fold` says on pads at `pads`,
/// its pelvis pitched `pitch` nose up.
fn place(
    rig: &Rig,
    body: &Body,
    hind: &HindLeg,
    pose: &mut Pose,
    fold: Fold,
    pad: Vec3,
    pitch: f32,
) {
    let leg = &body.hinds[0];
    let ([at_hip, _, _], _) = hind.up_from(pad, fold);
    let root_rest = rig.joints[body.root].position;
    let hip_rest = rig.joints[leg.chain[0]].position;
    let turned = turn(pitch);
    let mut offset = hip_rest - root_rest;
    offset.x = 0.0;
    let mut root = Vec3::new(0.0, at_hip.y, at_hip.z) - turned * offset;
    root.x = root_rest.x;
    pose.rotations[body.root] = turned;
    pose.translation = root - root_rest;
}

/// Everything a rest is solved from that the rig alone decides.
#[derive(Clone, Debug)]
struct Ctx {
    body: Body,
    hind: HindLeg,
    /// Where a sit ends.
    end: Sat,
    /// A hind leg's reach, what a stepping paw lifts by a share of.
    reach: f32,
    /// Every joint a rest poses, which it poses from rest: all but the
    /// head's own parts past the skull - its ears and its jaw, which go on
    /// doing whatever the body was doing with them.
    owned: Vec<usize>,
}

impl Ctx {
    fn read(rig: &Rig) -> Option<Self> {
        let body = Body::read(rig)?;
        let hind = HindLeg::read(rig, &body.hinds[0]);
        let end = sat(rig, &body, &hind);
        let reach = rig.limb_reach(body.hinds[0].limb).unwrap_or(0.0);
        let head = rig.in_zone(Zone::Head).first().copied();
        let owned = (0..rig.len())
            .filter(|&joint| rig.joints[joint].zone != Zone::Head || Some(joint) == head)
            .collect();
        Some(Self {
            body,
            hind,
            end,
            reach,
            owned,
        })
    }

    /// Puts every joint a rest poses back at rest, so that what it poses is
    /// a function of the rig and how far into the rest the body is, and of
    /// nothing the body was doing before (#377): an IK solve keeps the twist
    /// a bone already had and a level head is turned from the neck it found,
    /// so two ways to the same moment - the end of a sit and the start of the
    /// lying down that goes on from it - posed it twice, differently.
    fn reset(&self, pose: &mut Pose) {
        for &joint in &self.owned {
            pose.rotations[joint] = Quat::IDENTITY;
        }
        pose.translation = Vec3::ZERO;
    }
}

/// Where a sitting body has got to: what a lying one starts from.
#[derive(Clone, Copy, Debug)]
struct Seated {
    pelvis: f32,
    chest: f32,
    /// The left hip, in the side plane.
    hip: Vec3,
    pads: [Vec3; 2],
    /// How far every foot was lifted off `pads` and the rest pads for the
    /// skin of a folded leg not to sink.
    lift: f32,
    translation: Vec3,
}

/// How much of lying down a dog or a cat spends sitting first, and how much
/// of getting up from lying any beast spends rising from its sit.
/// Provenance: **the research's** (09 fact 6 and 2.3.4): dogs and cats lie
/// down through a sit and rise back through it, a horse rises through its
/// dog-sit. The shares are `emote`'s own times: a second of a dog's 2.4 to
/// sit, 1.1 of its 2.0 to rise from the sit.
const LIE_SAT: [f32; 2] = [0.4, 0.5];

/// What a rest solves from the rig alone, solved once and kept (#377).
///
/// A sit's end is a 24-step search, a lying one's a 20-step search over a
/// trunk placed four times and forelegs fitted six times each, every step a
/// reading of two thousand points of skin: solved again every frame, a held
/// Lie Down cost 1.7 to 1.9 ms a frame against an idle frame's 0.035. None
/// of it changes while the rig does not, and the end of each rest - the pose
/// a body holds - is kept whole.
///
/// The [`super::driver::Driver`] keeps one for its body; a clip applied on
/// its own solves into a fresh one.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rests {
    /// What of the rig the solve was made for, and the solve, or `None` for
    /// a body a rest does not pose.
    solved: Option<(Key, Option<Box<Solved>>)>,
}

/// What of a rig a rest reads: every joint's place, size, parent and zone,
/// how it stands, and how much of its skin it measured.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    joints: Vec<(Vec3, f32, glam::Vec2, Option<usize>, Zone)>,
    feet: FootKind,
    skin: usize,
}

impl Key {
    fn of(rig: &Rig) -> Self {
        Self {
            joints: rig
                .joints
                .iter()
                .map(|joint| {
                    (
                        joint.position,
                        joint.radius,
                        joint.scale,
                        joint.parent,
                        joint.zone,
                    )
                })
                .collect(),
            feet: rig.anatomy.feet,
            skin: rig.outline.as_ref().map_or(0, crate::rig::Outline::len),
        }
    }
}

/// The parts of a rest that stay put: the sit's end, the lying end, and the
/// pose each rest holds at its end.
#[derive(Clone, Debug)]
struct Solved {
    ctx: Ctx,
    /// The sit's end and the pose it holds there.
    seated: Option<(Seated, Pose)>,
    /// The lying end, and the trunk lowered into it under its head.
    lain: Option<(Lain, Pose)>,
    /// What a lying beast holds.
    lying: Option<Pose>,
}

impl Rests {
    /// The solve for `rig`, made now if it was made for another, or `None`
    /// for a body that does not stand on four legs.
    fn of(&mut self, rig: &Rig) -> Option<&mut Solved> {
        let key = Key::of(rig);
        if self.solved.as_ref().is_none_or(|(made, _)| *made != key) {
            let solved = Ctx::read(rig).map(|ctx| {
                Box::new(Solved {
                    ctx,
                    seated: None,
                    lain: None,
                    lying: None,
                })
            });
            self.solved = Some((key, solved));
        }
        self.solved.as_mut()?.1.as_deref_mut()
    }
}

impl Solved {
    /// The sit's end.
    fn seated(&mut self, rig: &Rig) -> (Seated, &Pose) {
        let ctx = &self.ctx;
        let (seated, pose) = self.seated.get_or_insert_with(|| {
            let mut pose = Pose::rest(rig);
            let seated = sit(rig, ctx, &mut pose, 1.0, false);
            (seated, pose)
        });
        (*seated, pose)
    }

    /// The lying end, from the sit's: a dog's or a cat's sphinx over the
    /// sit's own hind pads; a horse's on its folded legs, its hind legs
    /// gathered out beside it and its pelvis rolled onto a hip.
    fn lain(&mut self, rig: &Rig) -> (Seated, Lain, &Pose) {
        let (seated, _) = self.seated(rig);
        let ctx = &self.ctx;
        let (lain, lowered) = self.lain.get_or_insert_with(|| {
            let (pads, roll, folded) = if hooved(rig) == 1 {
                (
                    beside(rig, &ctx.body, &ctx.hind, seated.pads),
                    LIE_ROLL,
                    true,
                )
            } else {
                (seated.pads, 0.0, false)
            };
            lain(rig, ctx, seated.hip.z, pads, roll, folded)
        });
        (seated, *lain, lowered)
    }

    /// Poses `rig` `down` of the way into `rest`, or out of it if `rising`,
    /// over a pose its joints have been put back at rest in.
    fn pose(&mut self, rig: &Rig, pose: &mut Pose, rest: Rest, down: f32, rising: bool) {
        match rest {
            Rest::Sit => {
                sit(rig, &self.ctx, pose, down, rising);
            }
            Rest::Lie if hooved(rig) == 1 && !rising => {
                let (_, end, lowered) = self.lain(rig);
                let lowered = lowered.clone();
                lie_hooved(rig, &self.ctx, pose, &end, &lowered, down);
            }
            Rest::Lie => {
                // Through the sit, both ways, but for a horse going down.
                let sat = LIE_SAT[usize::from(rising)];
                if down <= sat {
                    sit(rig, &self.ctx, pose, down / sat, rising);
                } else {
                    let (seated, end, lowered) = self.lain(rig);
                    let down = (down - sat) / (1.0 - sat);
                    if hooved(rig) == 1 {
                        let lowered = lowered.clone();
                        rise_hooved(rig, &self.ctx, pose, &seated, &end, &lowered, down);
                    } else {
                        lie(rig, &self.ctx, pose, &seated, &end, down, rising);
                    }
                }
            }
        }
    }

    /// The pose `rest` holds at its end: solved once, down, and kept.
    fn held(&mut self, rig: &Rig, rest: Rest) -> &Pose {
        if rest == Rest::Lie && self.lying.is_none() {
            let mut pose = Pose::rest(rig);
            self.pose(rig, &mut pose, rest, 1.0, false);
            self.lying = Some(pose);
        }
        match rest {
            Rest::Sit => self.seated(rig).1,
            Rest::Lie => self
                .lying
                .as_ref()
                .unwrap_or_else(|| unreachable!("solved above")),
        }
    }
}

/// Poses `rig` `down` of the way into `rest`, or out of it if `rising`, the
/// parts of the solve that stay put kept in `rests`.
pub(super) fn rest(
    rig: &Rig,
    pose: &mut Pose,
    rest: Rest,
    down: f32,
    rising: bool,
    rests: &mut Rests,
) {
    if !pose.fits(rig) || down <= 0.0 {
        return;
    }
    let Some(solved) = rests.of(rig) else {
        return;
    };
    solved.ctx.reset(pose);
    if down >= 1.0 {
        // Held: whichever way the clip runs, the same pose, solved once.
        let held = solved.held(rig, rest).clone();
        for &joint in &solved.ctx.owned {
            pose.rotations[joint] = held.rotations[joint];
        }
        pose.translation = held.translation;
        return;
    }
    solved.pose(rig, pose, rest, down, rising);
}

/// Poses `down` of the way into a sit, or out of it.
fn sit(rig: &Rig, ctx: &Ctx, pose: &mut Pose, down: f32, rising: bool) -> Seated {
    let (body, hind, end) = (&ctx.body, &ctx.hind, &ctx.end);
    // Each joint's own share of the way down: the hind paws come forward
    // first, then one braked descent, the metatarsi flat before the
    // haunches land; up, hip, stifle and hock in turn, and the paws back.
    let (hip, stifle, hock, steps) = if rising {
        let up = 1.0 - down;
        (
            1.0 - step(0.0, 0.4, up),
            1.0 - step(0.1, 0.65, up),
            1.0 - step(0.25, 0.85, up),
            [1.0 - step(0.7, 0.9, up), 1.0 - step(0.8, 1.0, up)],
        )
    } else {
        let eased = step(0.1, 1.0, down);
        (
            eased,
            eased,
            step(0.1, 0.85, down),
            [step(0.0, 0.25, down), step(0.1, 0.35, down)],
        )
    };
    let lerp = |from: f32, to: f32, share: f32| from + (to - from) * share;
    let fold = Fold {
        meta: lerp(hind.stood.meta, end.fold.meta, hock),
        hock: lerp(hind.stood.hock, end.fold.hock, hock),
        stifle: lerp(hind.stood.stifle, end.fold.stifle, stifle),
    };
    let pads = [0, 1].map(|side| {
        let share = steps[side];
        rig.joints[body.hinds[side].pad].position
            + Vec3::new(
                0.0,
                STEP_LIFT * ctx.reach * (share * PI).det_sin(),
                end.step * share,
            )
    });
    // The hips go where the fold carries them over pads that have come as
    // far forward as the fold has: the paws step first, under a body that
    // has not yet moved.
    // Rising, over the feet where they are until they step back at the end.
    let under = if rising {
        stifle.max(0.5 * (steps[0] + steps[1]))
    } else {
        stifle
    };
    let middle = rig.joints[body.hinds[0].pad].position + Vec3::Z * (end.step * under);
    // **The pelvis turns with the femur, less the hip's own flexion**, which
    // is what the schedule above times: the hip's angle, not the pelvis's
    // pitch. Rising with the pelvis levelled first, the hip closed under a
    // femur still lying forward and opened last, as the stifle swung the
    // femur back - where a dog's hip extends first, its weight going back
    // onto its hindquarters (09 fact 5).
    let femur = |fold: Fold| hind.up_from(Vec3::ZERO, fold).1;
    let flexed = femur(end.fold) - femur(hind.stood) - end.pelvis;
    let pitch = femur(fold) - femur(hind.stood) - flexed * hip;
    trunk(body, pose, pitch, pitch);
    place(rig, body, hind, pose, fold, middle, pitch);
    let relay = |pose: &mut Pose, pads: &[Vec3]| {
        for (leg, pad) in body.hinds.iter().zip(pads) {
            fold_hind(rig, pose, leg, hind, fold, *pad);
        }
    };
    relay(pose, &pads);
    // Nothing of the haunches below the ground on the way: what the fold
    // would sink, the body rises by, and the legs reach down again.
    let below = GROUND - haunch(rig, &pose.forward(rig));
    if below > 0.0 {
        pose.translation.y += below;
    }
    // The forelegs come off locked as the hips start down, and lock again
    // as they finish rising: standing, the sit is the stance itself.
    let give = 1.0 - (1.0 - FORE_GIVE) * hip;
    let chest = bend_to_fores(rig, body, pose, pitch, give);
    stand_fores(rig, body, pose, 0.0);
    relay(pose, &pads);
    let inherited = pose.forward(rig).rotations[body.withers];
    relevel_head(rig, pose, inherited);
    let lift = settle(rig, body, pose, true, |pose: &mut Pose, lift: f32| {
        let raised = pads.map(|pad| pad + Vec3::Y * lift);
        relay(pose, &raised);
        stand_fores(rig, body, pose, lift);
    });
    let at = pose.forward(rig).positions[body.hinds[0].chain[0]];
    Seated {
        pelvis: pitch,
        chest,
        hip: at,
        pads,
        lift,
        translation: pose.translation,
    }
}

/// How far a lying beast's pelvis stays pitched nose up, in radians: near
/// level, its belly and thighs on its folded hind legs.
/// Provenance: **the research's** (09, 2.1.4): the pelvis level again. A
/// horse's croup pitched further down (09, 2.3.1: "lower and tilted") lifted
/// its loin and chest 40 to 80 mm off the ground (#377): its croup is low
/// by the roll instead.
const LIE_PELVIS: f32 = 0.05;

/// How far a lying beast's chest may pitch either way to rest its sternum
/// on the ground with its back, in radians, paws and hooves: a back that
/// lies near level, and a horse's chest coming down to the ground in front
/// of a pelvis rolled onto a hip, which stands its hips higher.
/// Provenance: **the research's** (09, 2.1.4 and 2.3.1): the spine roughly
/// horizontal; the horse's, **tuned by test** (#377). Pitched further, a
/// dog's loin carried its belly down with it before its sternum.
const LIE_CHEST: [f32; 2] = [0.3, 0.6];

/// A sphinx's end: the pelvis near level and its haunch on the ground, the
/// chest pitched until its sternum touches too, and where each forepaw lies
/// for a forearm flat on the ground from the elbow, the elbow under and
/// behind its shoulder.
#[derive(Clone, Copy, Debug)]
struct Lain {
    chest: f32,
    paws: [Vec3; 2],
    elbows: [Vec3; 2],
    /// How high the left hip lies, the trunk lowered as far as its folded
    /// forelegs let its chest sink between them.
    hip: f32,
    /// Where the hind pads lie.
    pads: [Vec3; 2],
}

/// The back of a lying body, what it rests on behind its chest: the pelvis
/// and the belly. Not the thighs: the stifle's joint holds the gaskin, which
/// lies on the ground at the hock whatever the trunk above it does.
fn back(zone: Zone) -> bool {
    matches!(zone, Zone::Pelvis | Zone::Abdomen)
}

/// Places a lying trunk: pitched `pelvis` and `chest` and rolled `roll`,
/// its hip `hip` along the ground, as low as its lowest skin lets it, and its
/// hind legs folded onto pads at `pads`.
#[expect(clippy::too_many_arguments, reason = "a trunk's whole attitude")]
fn lay_trunk(
    rig: &Rig,
    ctx: &Ctx,
    pose: &mut Pose,
    hip: f32,
    pads: [Vec3; 2],
    pelvis: f32,
    chest: f32,
    roll: f32,
) {
    let body = &ctx.body;
    rolled(body, pose, pelvis, chest, roll);
    pose.translation = Vec3::ZERO;
    let at = pose.forward(rig).positions[body.hinds[0].chain[0]];
    pose.translation.z = hip - at.z;
    // Started near the ground: each pass refolds the thighs, whose skin is
    // part of what the trunk rests on.
    pose.translation.y = pads[0].y + ctx.hind.femur - at.y;
    for (leg, pad) in body.hinds.iter().zip(pads) {
        reach_hind(rig, pose, leg, &ctx.hind, pad);
    }
    for _ in 0..4 {
        // The trunk and the hind legs folded under it: a leg folded as far as
        // it goes holds the hips up off the ground.
        let low = lowest_in(rig, &pose.forward(rig), |zone| {
            back(zone)
                || zone == Zone::Chest
                || matches!(zone, Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) if !limb.is_fore())
        });
        pose.translation.y += GROUND - low;
        for (leg, pad) in body.hinds.iter().zip(pads) {
            reach_hind(rig, pose, leg, &ctx.hind, pad);
        }
    }
}

/// Where a folded hind leg's stifle bends toward, sitting or lying: ahead
/// of a leg that hangs from its hip, up beside the belly about as high as
/// the hip for one whose hock lies level with it, and out to the side as
/// far as the hock stands out from the hip.
///
/// **Turned over through the side, never flipped** (#377). Square to the
/// leg on its forward side, the pole pointed down when a lying horse's hip
/// stood a hair above its hock and was flipped up when a hair below, and
/// the stifle went from beside its belly into the ground between two
/// frames: its whole body rose 250 mm. Between hanging and lying the bend
/// now turns over through the leg's own side instead. A sit's own pole, out
/// along the tibia past the stifle, came round into line with the leg as a
/// rising horse's hip opened over a femur still lying forward, and the
/// bend spun about the leg: the stifle swept 25 mm in a thousandth of the
/// rise.
fn hind_pole(hip: Vec3, hock: Vec3, hind: &HindLeg) -> Vec3 {
    let line = hock - hip;
    let ahead = Vec3::new(0.0, line.z, -line.y).normalize_or(Vec3::Z);
    // Square to the leg and the bend ahead of it, toward the leg's own side.
    let out = (line.cross(ahead) * -hip.x.signum()).normalize_or(Vec3::X);
    // How far over from ahead toward up: none while the hock is a fifth of
    // a right angle or more below the hip, all of it once level with it -
    // however far behind it or out beside it the hock lies.
    let behind = (line.x * line.x + line.z * line.z).sqrt().copysign(-line.z);
    let over = PI * (1.0 - step(0.0, LYING_LEG, (-line.y).det_atan2(behind)));
    let bend = ahead * over.det_cos() + out * over.det_sin();
    let side = Vec3::X * (hock.x - hip.x) * 2.0;
    let length = hind.femur + hind.tibia;
    (hip + hock) * 0.5 + (bend + Vec3::Y * 0.5) * length + side
}

/// How far below the hip a lying hind leg's hock lies, in radians from
/// level, before its stifle starts turning from ahead of the leg over to
/// up beside the belly.
/// Provenance: **chosen** (#377): a fifth of a right angle, short of every
/// lying dog's and cat's, so their sphinx bends its legs as it did.
const LYING_LEG: f32 = 0.35;

/// Folds a hind leg from wherever its hip is onto a pad at `pad`, the
/// metatarsus flat behind it and the stifle forward beside the belly.
fn reach_hind(rig: &Rig, pose: &mut Pose, leg: &Leg, hind: &HindLeg, pad: Vec3) {
    reach_hind_by(rig, pose, leg, hind, pad, 1.0);
}

/// [`reach_hind`], the metatarsus laid `lay` of the way from standing.
fn reach_hind_by(rig: &Rig, pose: &mut Pose, leg: &Leg, hind: &HindLeg, pad: Vec3, lay: f32) {
    unbend(pose, leg);
    let posed = pose.forward(rig);
    let hip = posed.positions[leg.chain[0]];
    let meta = hind.stood.meta * (1.0 - lay);
    let mut hock = pad - along(meta) * hind.meta;
    hock.x = pad.x;
    two_bone(rig, pose, leg.chain, hock, hind_pole(hip, hock, hind));
    let carried = pose.forward(rig).rotations[leg.chain[1]];
    let laid = turn(meta - hind.stood.meta);
    pose.rotations[leg.chain[2]] = carried.inverse() * laid;
    if leg.toe.is_some() {
        pose.rotations[leg.pad] = laid.inverse();
    }
}

/// The end of lying down, the hips `hip` along the ground over hind pads
/// at `pads`, the pelvis rolled `roll` onto its hip: the forelegs laid out
/// ahead, or with `folded`, tucked under the chest on their knees; and the
/// trunk lowered into it, its hind legs folded and its head up.
fn lain(rig: &Rig, ctx: &Ctx, hip: f32, pads: [Vec3; 2], roll: f32, folded: bool) -> (Lain, Pose) {
    let body = &ctx.body;
    // How far the sternum stands above the back, the trunk resting on
    // whichever is lower: the chest pitched until both touch, its sternum
    // and its belly or haunch, within a back that lies near level.
    let apart = |chest: f32| {
        let mut pose = Pose::rest(rig);
        lay_trunk(rig, ctx, &mut pose, hip, pads, LIE_PELVIS, chest, roll);
        // A horse rests behind on the hip it rolls onto, its belly clear.
        let behind = |zone: Zone| back(zone) && !(folded && zone == Zone::Abdomen);
        let [chest, rest] = lowest_each(rig, &pose.forward(rig), |joint| {
            let zone = rig.joints[joint].zone;
            if zone == Zone::Chest {
                Some(0)
            } else {
                behind(zone).then_some(1)
            }
        });
        chest - rest
    };
    let limit = LIE_CHEST[usize::from(folded)];
    let (mut low, mut high) = (-limit, limit);
    let chest = if apart(low) >= 0.0 {
        low
    } else if apart(high) <= 0.0 {
        high
    } else {
        for _ in 0..20 {
            let middle = 0.5 * (low + high);
            if apart(middle) > 0.0 {
                high = middle;
            } else {
                low = middle;
            }
        }
        0.5 * (low + high)
    };
    // The forelegs fitted to the trunk as it will be drawn, and the trunk
    // lowered by what its own skin then clears and the forelegs fitted
    // again: folding a foreleg carries the skin under the shoulder with it.
    let mut paws = [Vec3::ZERO; 2];
    let mut elbows = [Vec3::ZERO; 2];
    let mut sunk = 0.0f32;
    for _ in 0..3 {
        let trunk = lowered(rig, ctx, hip, pads, LIE_PELVIS, chest, roll, sunk);
        (paws, elbows) = fit_fores(rig, ctx, &trunk, folded);
        let mut trial = trunk;
        for (side, leg) in body.fores.iter().enumerate() {
            let pole = elbows[side] - Vec3::Z * ctx.reach;
            pose_fore(rig, &mut trial, leg, paws[side], pole, 1.0, folded);
        }
        let clear = lowest_in(rig, &trial.forward(rig), |zone| {
            back(zone) || zone == Zone::Chest
        }) - GROUND;
        if clear < 1.0e-3 {
            break;
        }
        sunk += clear;
    }
    // A horse's thighs out of the ground again, its forelegs where they were
    // fitted: lowered as far as its chest lets it, the trunk sank a heavy
    // horse's folded hind legs 229 mm into the ground (#377). Left to the
    // rest's own settle, which raises a lying or rising horse's feet with
    // its thighs, the whole body rose with them.
    let thighs = |zone: Zone| matches!(zone, Zone::UpperLimb(limb) if !limb.is_fore());
    for _ in 0..6 {
        if !folded {
            break;
        }
        let trunk = lowered(rig, ctx, hip, pads, LIE_PELVIS, chest, roll, sunk);
        let below = GROUND - lowest_in(rig, &trunk.forward(rig), thighs);
        if below <= 1.0e-4 {
            break;
        }
        sunk -= below;
    }
    let trunk = lowered(rig, ctx, hip, pads, LIE_PELVIS, chest, roll, sunk);
    let low = trunk.forward(rig).positions[body.hinds[0].chain[0]].y;
    let lain = Lain {
        chest,
        paws,
        elbows,
        hip: low,
        pads,
    };
    (lain, trunk)
}

/// A lying trunk as [`lay_trunk`] places it, `sunk` lower, its hind legs
/// folded again under it and its head held up.
#[expect(clippy::too_many_arguments, reason = "a trunk's whole attitude")]
fn lowered(
    rig: &Rig,
    ctx: &Ctx,
    hip: f32,
    pads: [Vec3; 2],
    pelvis: f32,
    chest: f32,
    roll: f32,
    sunk: f32,
) -> Pose {
    let mut pose = Pose::rest(rig);
    lay_trunk(rig, ctx, &mut pose, hip, pads, pelvis, chest, roll);
    pose.translation.y -= sunk;
    for (leg, pad) in ctx.body.hinds.iter().zip(pads) {
        reach_hind(rig, &mut pose, leg, &ctx.hind, pad);
    }
    let inherited = pose.forward(rig).rotations[ctx.body.withers];
    relevel_head(rig, &mut pose, inherited);
    pose
}

/// Where each foreleg of a lying trunk lies: the point of the elbow on the
/// ground behind its shoulder, the forearm ahead of it to the wrist, and
/// there either the paw laid flat on its pads or, `folded`, the knee on the
/// ground with the cannon laid back under the forearm - the elbow and the
/// wrist each at the height their own skin just touches the ground at.
fn fit_fores(rig: &Rig, ctx: &Ctx, trunk: &Pose, folded: bool) -> ([Vec3; 2], [Vec3; 2]) {
    let posed = trunk.forward(rig);
    let place = |leg: &Leg, high: f32, lift: f32| {
        let at = |joint: usize| rig.joints[joint].position;
        let [shoulder, elbow, wrist] = leg.chain.map(at);
        let (upper, fore, hand) = (
            shoulder.distance(elbow),
            elbow.distance(wrist),
            wrist.distance(at(leg.pad)),
        );
        let top = posed.positions[leg.chain[0]];
        let low = if folded {
            rig.joints[leg.chain[2]].radius
        } else {
            at(leg.pad).y
        } + lift;
        let drop = (top.y - high).clamp(0.0, upper * 0.999);
        let back = (upper * upper - drop * drop).max(0.0).sqrt();
        let elbow = Vec3::new(at(leg.chain[1]).x, top.y - drop, top.z - back);
        let fall = (elbow.y - low).clamp(-fore * 0.9, fore * 0.9);
        let ahead = (fore * fore - fall * fall).max(0.0).sqrt();
        let reach = if folded { 0.0 } else { hand };
        let paw = Vec3::new(at(leg.pad).x, low, elbow.z + ahead + reach);
        (paw, elbow)
    };
    let mut paws = [Vec3::ZERO; 2];
    let mut elbows = [Vec3::ZERO; 2];
    for (side, leg) in ctx.body.fores.iter().enumerate() {
        let (mut high, mut lift) = (rig.joints[leg.chain[1]].radius, 0.0f32);
        for _ in 0..6 {
            let (paw, elbow) = place(leg, high, lift);
            (paws[side], elbows[side]) = (paw, elbow);
            let mut trial = trunk.clone();
            pose_fore(
                rig,
                &mut trial,
                leg,
                paw,
                elbow - Vec3::Z * ctx.reach,
                1.0,
                folded,
            );
            // The wrist and the paw, and then the forearm from the elbow.
            let [hand, arm] = lowest_each(rig, &trial.forward(rig), |joint| {
                match rig.joints[joint].zone {
                    Zone::LowerLimb(limb) | Zone::Extremity(limb) if limb == leg.limb => Some(0),
                    Zone::UpperLimb(limb) if limb == leg.limb => Some(1),
                    _ => None,
                }
            });
            if (hand - GROUND).abs() < 5.0e-4 && (arm - GROUND).abs() < 5.0e-4 {
                break;
            }
            lift = (lift + GROUND - hand).max(-0.5 * rig.joints[leg.chain[2]].radius);
            high += GROUND - arm;
        }
    }
    (paws, elbows)
}

/// Poses a foreleg `share` of the way to lying: its paw laid flat at
/// `target`, or with `folded`, its wrist at `target` and the cannon folded
/// back under the forearm, the elbow bending toward `pole`.
fn pose_fore(
    rig: &Rig,
    pose: &mut Pose,
    leg: &Leg,
    target: Vec3,
    pole: Vec3,
    share: f32,
    folded: bool,
) {
    if !folded {
        loosen(pose, leg);
        super::ground::solve_contact_laid_by(rig, pose, leg.limb, target, pole, share);
        return;
    }
    unbend(pose, leg);
    two_bone(rig, pose, leg.chain, target, pole);
    // The cannon laid back along the ground from the knee, under a forearm
    // that rises from the knee to the elbow, and the fetlock flexed with
    // it, the hoof turned back over the top, its sole up: hanging off the
    // cannon, the hoof stood on the ground and held the knee off it.
    let posed = pose.forward(rig);
    let rest = (rig.joints[leg.pad].position - rig.joints[leg.chain[2]].position)
        .normalize_or(Vec3::NEG_Y);
    let cannon = crate::det::slerp(
        Quat::IDENTITY,
        crate::det::from_rotation_arc(rest, Vec3::NEG_Z),
        share,
    );
    pose.rotations[leg.chain[2]] = posed.rotations[leg.chain[1]].inverse() * cannon;
    if leg.toe.is_some() {
        let hoof = crate::det::slerp(cannon, turn(PI), share);
        pose.rotations[leg.pad] = cannon.inverse() * hoof;
    }
}

/// How far a lying horse's pelvis rolls onto its hip, in radians, and how
/// much of that its chest keeps: the forehand upright.
/// Provenance: **the research's** (09, 2.3.1): 30 to 60 degrees of roll
/// between a near upright thorax and a pelvis on its side.
const LIE_ROLL: f32 = 0.3;
const LIE_THORAX: f32 = 0.15;

/// Hind pads `pads` moved out to either side as far as a folded hind leg
/// needs to lie on its outer face: a horse's tibia is so much longer than
/// its femur that the leg folds no tighter than the difference, more than a
/// lying horse's hip stands above a hock beneath it.
fn beside(rig: &Rig, body: &Body, hind: &HindLeg, pads: [Vec3; 2]) -> [Vec3; 2] {
    let out = (hind.tibia - hind.femur).abs();
    [0, 1].map(|side| {
        pads[side] + Vec3::X * out.copysign(rig.joints[body.hinds[side].pad].position.x)
    })
}

/// Poses `down` of the way into a horse's lying down (09, 2.3.3): the feet
/// gathered under the belly, the forehand dropping onto its knees first,
/// the hindquarters folding after it and rolling onto a hip, into `end`,
/// the trunk lowered into it as `finished`. Its hindquarters end where its
/// dog-sit has them, which it rises through: its hind legs folded under,
/// the croup down.
fn lie_hooved(rig: &Rig, ctx: &Ctx, pose: &mut Pose, end: &Lain, finished: &Pose, down: f32) {
    let body = &ctx.body;
    let at = |joint: usize| rig.joints[joint].position;
    let gathered = end.pads;
    let (gather, front, rear) = (
        [step(0.0, 0.25, down), step(0.08, 0.33, down)],
        step(0.25, 0.65, down),
        step(0.5, 1.0, down),
    );
    let pads = [0, 1].map(|side| {
        let share = gather[side];
        at(body.hinds[side].pad).lerp(gathered[side], share)
            + Vec3::Y * (STEP_LIFT * ctx.reach * (share * PI).det_sin())
    });
    let withers = |pose: &Pose| pose.forward(rig).positions[body.withers].y;
    let dropped = withers(finished) - at(body.withers).y;
    // The hindquarters come down as the rear does; the chest pitches as
    // far as brings the withers down as the front does.
    let pitched = |pose: &mut Pose, chest: f32| {
        rolled(body, pose, LIE_PELVIS * rear, chest, LIE_ROLL * rear);
        pose.translation = finished.translation * rear;
    };
    let wanted = at(body.withers).y + dropped * front;
    if false {
        let mut line = format!("down {down:.3}:");
        for k in 0..=20 {
            let c = -1.0 + k as f32 / 10.0;
            pitched(pose, c);
            line += &format!(" {:+.0}", (withers(pose) - wanted) * 1000.0);
        }
        eprintln!("{line}");
    }
    let (mut low, mut high) = (-1.0f32, 1.0f32);
    for _ in 0..20 {
        let middle = 0.5 * (low + high);
        pitched(pose, middle);
        if withers(pose) > wanted {
            high = middle;
        } else {
            low = middle;
        }
    }
    pitched(pose, 0.5 * (low + high));
    let legs = |pose: &mut Pose, lift: f32| {
        for (leg, pad) in body.hinds.iter().zip(pads) {
            reach_hind_by(rig, pose, leg, &ctx.hind, pad + Vec3::Y * lift, rear);
        }
        for (side, leg) in body.fores.iter().enumerate() {
            let wrist = at(leg.chain[2]).lerp(end.paws[side], front) + Vec3::Y * lift;
            let stood = rig.bend_pole(leg.limb).unwrap_or(wrist);
            let pole = stood.lerp(end.elbows[side] - Vec3::Z * ctx.reach, front) + Vec3::Y * lift;
            pose_fore(rig, pose, leg, wrist, pole, front, true);
        }
    };
    legs(pose, 0.0);
    let inherited = pose.forward(rig).rotations[body.withers];
    relevel_head(rig, pose, inherited);
    settle(rig, body, pose, false, legs);
}

/// Poses `down` of the way between a horse lying in `end`, its trunk lowered
/// into it as `finished`, and its dog-sit `seated`, rising (09, 2.3.4): the
/// forelegs out ahead of the chest one and then the other, the forehand up
/// after them, the pelvis rolling back off its hip, and the hind legs drawn
/// in under the body from beside it into the sit's own fold.
fn rise_hooved(
    rig: &Rig,
    ctx: &Ctx,
    pose: &mut Pose,
    seated: &Seated,
    end: &Lain,
    finished: &Pose,
    down: f32,
) {
    let body = &ctx.body;
    let at = |joint: usize| rig.joints[joint].position;
    let lying = end.pads;
    let up = 1.0 - down;
    let (fores, risen) = (
        [step(0.0, 0.45, up), step(0.15, 0.6, up)],
        step(0.3, 1.0, up),
    );
    let lerp = |from: f32, to: f32, share: f32| from + (to - from) * share;
    rolled(
        body,
        pose,
        lerp(LIE_PELVIS, seated.pelvis, risen),
        lerp(end.chest, seated.chest, risen),
        LIE_ROLL * (1.0 - risen),
    );
    pose.translation = finished.translation.lerp(seated.translation, risen);
    // Up to where the sit has its feet, lifted as far as it lifts them.
    let raised = Vec3::Y * seated.lift;
    let pads = [0, 1].map(|side| lying[side].lerp(seated.pads[side] + raised, risen));
    let legs = |pose: &mut Pose, lift: f32| {
        for (leg, pad) in body.hinds.iter().zip(pads) {
            let pad = pad + Vec3::Y * lift;
            fold_down(rig, pose, leg, &ctx.hind, ctx.end.fold, pad, 1.0 - risen);
        }
        for (side, leg) in body.fores.iter().enumerate() {
            let share = fores[side];
            let wrist = end.paws[side].lerp(at(leg.chain[2]) + raised, share) + Vec3::Y * lift;
            let stood = rig.bend_pole(leg.limb).unwrap_or(wrist);
            let pole = (end.elbows[side] - Vec3::Z * ctx.reach).lerp(stood, share) + Vec3::Y * lift;
            pose_fore(rig, pose, leg, wrist, pole, 1.0 - share, true);
        }
    };
    legs(pose, 0.0);
    let inherited = pose.forward(rig).rotations[body.withers];
    relevel_head(rig, pose, inherited);
    settle(rig, body, pose, false, legs);
}

/// Poses `down` of the way from `seated` to lying down in `end`, or back up
/// to it.
///
/// **On from the sit exactly, and back into it** (#377). The trunk turns
/// from the sit's attitude to the sphinx's about hips held where the sit
/// left them along the ground, coming down onto the sphinx's as the pelvis
/// levels; the hind legs fold on from the sit's own fold, and every foot
/// comes down off whatever the sit lifted it by. Placed by the sphinx's own
/// rule from the first moment - as low as the trunk's skin lets it - a
/// dog's hips stood 161.5 mm off where its sit had left them a moment
/// before, and its whole body jumped there in one frame, going down and
/// getting up.
fn lie(
    rig: &Rig,
    ctx: &Ctx,
    pose: &mut Pose,
    seated: &Seated,
    end: &Lain,
    down: f32,
    rising: bool,
) {
    let body = &ctx.body;
    // Down, the chest first, the forepaws walking out from under it and
    // laid flat as the elbows come down, the pelvis levelling last; up, the
    // forelegs first, as a carnivore rises.
    let (chest, pelvis, paws, lay) = if rising {
        let up = 1.0 - down;
        (
            1.0 - step(0.0, 0.6, up),
            1.0 - step(0.3, 0.9, up),
            1.0 - step(0.1, 0.65, up),
            1.0 - step(0.05, 0.5, up),
        )
    } else {
        (
            step(0.0, 0.75, down),
            step(0.35, 1.0, down),
            step(0.0, 0.65, down),
            step(0.25, 0.9, down),
        )
    };
    let lerp = |from: f32, to: f32, share: f32| from + (to - from) * share;
    rolled(
        body,
        pose,
        lerp(seated.pelvis, LIE_PELVIS, pelvis),
        lerp(seated.chest, end.chest, chest),
        0.0,
    );
    hips_at(
        rig,
        body,
        pose,
        lerp(seated.hip.y, end.hip, pelvis),
        seated.hip.z,
    );
    let raised = |share: f32| Vec3::Y * (seated.lift * (1.0 - share));
    let hinds = |pose: &mut Pose, lift: f32| {
        for (leg, pad) in body.hinds.iter().zip(seated.pads) {
            let pad = pad + raised(pelvis) + Vec3::Y * lift;
            fold_down(rig, pose, leg, &ctx.hind, ctx.end.fold, pad, pelvis);
        }
    };
    hinds(pose, 0.0);
    let fores = |pose: &mut Pose, lift: f32| {
        for (side, leg) in body.fores.iter().enumerate() {
            let from = rig.joints[leg.pad].position + raised(paws);
            let target = from.lerp(end.paws[side], paws) + Vec3::Y * lift;
            let stood = rig.bend_pole(leg.limb).unwrap_or(target);
            let pole = stood.lerp(end.elbows[side] - Vec3::Z * ctx.reach, paws) + Vec3::Y * lift;
            pose_fore(rig, pose, leg, target, pole, lay, false);
        }
    };
    fores(pose, 0.0);
    let inherited = pose.forward(rig).rotations[body.withers];
    relevel_head(rig, pose, inherited);
    settle(rig, body, pose, true, |pose: &mut Pose, lift: f32| {
        hinds(pose, lift);
        fores(pose, lift);
    });
}

/// Puts the root where the trunk, turned as it is, stands its left hip `y`
/// high and `z` along the ground.
fn hips_at(rig: &Rig, body: &Body, pose: &mut Pose, y: f32, z: f32) {
    pose.translation = Vec3::ZERO;
    let at = pose.forward(rig).positions[body.hinds[0].chain[0]];
    pose.translation = Vec3::new(0.0, y - at.y, z - at.z);
}

/// Folds a hind leg onto its pad at `pad` `share` of the way from a sit's
/// fold `sat` to lying: the hock behind the pad with the metatarsus flat,
/// set where the sit sets it at `0` and where [`reach_hind`] does at `1`.
fn fold_down(
    rig: &Rig,
    pose: &mut Pose,
    leg: &Leg,
    hind: &HindLeg,
    sat: Fold,
    pad: Vec3,
    share: f32,
) {
    let at = |joint: usize| rig.joints[joint].position;
    let ([_, _, hock], _) = hind.up_from(pad, sat);
    let seated = Vec3::new(at(leg.chain[2]).x, hock.y, hock.z);
    let meta = sat.meta * (1.0 - share);
    let mut lying = pad - along(meta) * hind.meta;
    lying.x = pad.x;
    let hock = seated.lerp(lying, share);
    unbend(pose, leg);
    let hip = pose.forward(rig).positions[leg.chain[0]];
    two_bone(rig, pose, leg.chain, hock, hind_pole(hip, hock, hind));
    let carried = pose.forward(rig).rotations[leg.chain[1]];
    let laid = turn(meta - hind.stood.meta);
    pose.rotations[leg.chain[2]] = carried.inverse() * laid;
    if leg.toe.is_some() {
        pose.rotations[leg.pad] = laid.inverse();
    }
}

/// Puts a leg back at rest, so that solving it answers for where it is sent
/// and not for where it was.
fn unbend(pose: &mut Pose, leg: &Leg) {
    for joint in leg.chain.into_iter().chain([leg.pad]).chain(leg.toe) {
        pose.rotations[joint] = Quat::IDENTITY;
    }
}

/// [`unbend`], the knee or elbow turned by [`LOOSE`] for a contact solve:
/// a leg at rest whose foot already stands within half a millimetre of
/// where it is sent is left as it is, its foot not turned to the attitude
/// asked of it - a lying dog's paw passed that close once on its way out,
/// and for that one moment its wrist stood 43 mm off (#377).
fn loosen(pose: &mut Pose, leg: &Leg) {
    unbend(pose, leg);
    pose.rotations[leg.chain[1]] = Rot::x(LOOSE);
}

/// How far [`loosen`] turns a leg off rest, in radians: ten times what a
/// contact solve reads as untouched, and nothing the solve keeps, as it
/// turns the leg from there to where its foot is sent.
/// Provenance: **chosen** (#377).
const LOOSE: f32 = 0.01;

/// Whatever of the body still sinks rises out of the ground: the trunk by
/// what its own skin sinks - the rump's goes with a tail turned off the
/// ground - its legs reaching back down to their feet; and the whole body,
/// legs, feet and all, by what a leg's own skin sinks: raised under a body
/// that stays, a folded leg folds tighter and sinks again, where the skin
/// blended across a closed hock or wrist folds in under it.
///
/// `haunches` says whether the trunk sits on its thighs; see below.
///
/// The tail is laid along the ground as each pass leaves the trunk, so that
/// it lies as the trunk's last height lays it however the body got there.
///
/// `legs` poses every leg for its feet lifted by the given height, and
/// whatever a leg bends toward lifted with them: a foreleg folded toward a
/// pole left where it was turned as the body rose over it, swung its cannon
/// down, sank further, and took a horse's whole body 170 mm into the air in
/// six passes (#377). How far the feet were lifted in all comes back.
fn settle(
    rig: &Rig,
    body: &Body,
    pose: &mut Pose,
    haunches: bool,
    legs: impl Fn(&mut Pose, f32),
) -> f32 {
    // With `haunches`, a thigh is the trunk's: the haunch it is part of
    // rests on the ground, and the legs reach back down from under it. A
    // horse lying or rising lies on its hind legs folded beside it instead,
    // and raising its hips over a hind leg folded that tight swings the
    // thigh down as far as the hips rise: a heavy horse's sank 14 mm however
    // far its trunk was raised (#377), until its feet rose with it.
    let limb = |zone: Zone| {
        matches!(
            zone,
            Zone::UpperLimb(_) | Zone::LowerLimb(_) | Zone::Extremity(_)
        ) && !(haunches && matches!(zone, Zone::UpperLimb(limb) if !limb.is_fore()))
    };
    // Of the tail, only its root, the rump's, where the skin is read: the
    // rest is laid along the ground first.
    let skin = outline(rig).is_some();
    let slot = |joint: usize| {
        let zone = rig.joints[joint].zone;
        if limb(zone) {
            Some(1)
        } else if zone != Zone::Tail || (skin && docked(rig, joint)) {
            Some(0)
        } else {
            None
        }
    };
    let mut lifted = 0.0f32;
    for _ in 0..6 {
        lay_tail(rig, body, pose);
        let [trunk, leg] = lowest_each(rig, &pose.forward(rig), slot).map(|low| GROUND - low);
        if trunk <= 1.0e-4 && leg <= 1.0e-4 {
            break;
        }
        pose.translation.y += trunk.max(leg).max(0.0);
        lifted += leg.max(0.0);
        legs(pose, lifted);
    }
    lifted
}

/// Whether `joint` is the root of a tail, set on the pelvis.
fn docked(rig: &Rig, joint: usize) -> bool {
    rig.joints[joint].zone == Zone::Tail
        && rig.joints[joint]
            .parent
            .is_some_and(|parent| rig.joints[parent].zone == Zone::Pelvis)
}

/// Stands each foreleg on its paw where it stood.
fn stand_fores(rig: &Rig, body: &Body, pose: &mut Pose, lift: f32) {
    for leg in &body.fores {
        let target = rig.joints[leg.pad].position + Vec3::Y * lift;
        let pole = rig.bend_pole(leg.limb).unwrap_or(target) + Vec3::Y * lift;
        loosen(pose, leg);
        solve_contact_with(rig, pose, leg.limb, target, pole, Foot::Stood);
    }
}

/// Lifts a tail that would reach below the ground until it lies along it,
/// from the tail as it hangs, joint by joint from the root out, each turned up no further than the
/// skin it carries needs: the rest of the tail rides on.
fn lay_tail(rig: &Rig, body: &Body, pose: &mut Pose) {
    // From the tail as it hangs, whatever it was turned to before.
    for &joint in &body.tail {
        pose.rotations[joint] = Quat::IDENTITY;
    }
    // **No further under the body than it hangs** (#377): a croup pitched
    // far up swings the tail under the haunches, and a tail turned back up
    // from there first swings down through hanging straight before it
    // comes out of the ground - a horse's sat 108 degrees out on one frame
    // and not at all on the next. Turned back as far as it hangs at rest,
    // or as [`TAIL_UNDER`], whichever is further under, the turn up that
    // clears it only ever raises it.
    let under = |run: Vec3| (-run.y).det_atan2(-run.z);
    for pair in body.tail.windows(2) {
        let (joint, child) = (pair[0], pair[1]);
        let posed = pose.forward(rig);
        let run = posed.positions[child] - posed.positions[joint];
        let hangs = rig.joints[child].position - rig.joints[joint].position;
        let past = under(run) - under(hangs).max(TAIL_UNDER);
        if past > 0.0 {
            let above = rig.joints[joint]
                .parent
                .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
            pose.rotations[joint] = above.inverse() * Rot::x(past) * posed.rotations[joint];
        }
    }
    for (index, &from) in body.tail.iter().enumerate() {
        if index + 1 >= body.tail.len() {
            break;
        }
        let posed = pose.forward(rig);
        // The skin past this joint, which turning it carries; its own, at
        // the root, the joint above holds where it is.
        let carried = &body.tail[index + 1..];
        if lowest_in_joints(rig, &posed, carried) >= GROUND {
            continue;
        }
        let above = rig.joints[from]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        let mut lifted = |angle: f32| {
            // Back and up about the body's across: from hanging down, out
            // behind along the ground.
            let turned = Rot::x(angle) * posed.rotations[from];
            pose.rotations[from] = above.inverse() * turned;
            lowest_in_joints(rig, &pose.forward(rig), carried) - GROUND
        };
        // **The least turn that clears, found from hanging up** (#377), as
        // far as over the back, for a long tail hanging off a croup pitched
        // down: a quarter turn left a horse's in the ground. A turn up first
        // swings a hanging tail further into the ground before it comes out;
        // halved over the whole range at once, the search landed on either
        // side of that, and a horse's tail jumped from one to the other.
        // Where nothing clears, the turn that comes nearest.
        let mut below = (0.0f32, f32::MIN);
        let mut nearest = below;
        let mut clear = None;
        #[expect(clippy::cast_precision_loss, reason = "a few steps")]
        for index in 1..=TAIL_STEPS {
            let angle = TAIL_TURN * index as f32 / TAIL_STEPS as f32;
            let height = lifted(angle);
            if height >= 0.0 {
                clear = Some(angle);
                break;
            }
            below = (angle, height);
            if height > nearest.1 {
                nearest = below;
            }
        }
        let Some(mut high) = clear else {
            // Narrowed within a step either side, to go on from where the
            // last turn that cleared left it.
            let step = TAIL_TURN / TAIL_STEPS as f32;
            let (mut from, mut to) = (nearest.0 - step, (nearest.0 + step).min(TAIL_TURN));
            for _ in 0..16 {
                let (one, two) = ((2.0 * from + to) / 3.0, (from + 2.0 * to) / 3.0);
                if lifted(one) > lifted(two) {
                    to = two;
                } else {
                    from = one;
                }
            }
            lifted(0.5 * (from + to));
            continue;
        };
        let mut low = below.0;
        for _ in 0..12 {
            let middle = 0.5 * (low + high);
            if lifted(middle) >= 0.0 {
                high = middle;
            } else {
                low = middle;
            }
        }
        lifted(high);
    }
}

/// How far under the body a tail may hang from any of its joints, in
/// radians below straight back, where it does not hang further at rest.
/// Provenance: **chosen** (#377): five sixths of the way to straight down.
const TAIL_UNDER: f32 = 5.0 * PI / 12.0;

/// How far a tail joint turns up at most, and in how many even steps the
/// turn that clears the ground is searched for from hanging.
/// Provenance: **chosen** (#377): over the back, nine tenths of a half turn,
/// in steps of 13.5 degrees.
const TAIL_TURN: f32 = 0.9 * PI;
const TAIL_STEPS: usize = 12;

/// [`lowest_in`], over the skin `joints` hold.
fn lowest_in_joints(rig: &Rig, posed: &Posed, joints: &[usize]) -> f32 {
    lowest_each::<1>(rig, posed, |joint| joints.contains(&joint).then_some(0))[0]
}

/// How many even steps the loin's range is searched in for the bend that
/// best stands the shoulders on the forelegs, before the best is narrowed
/// within a step either side of it to a hundred-thousandth of a radian.
/// Provenance: **chosen** (#377).
const BEND_STEPS: usize = 16;

/// How far off a straight foreleg's length the shoulders may stand, as a
/// share of the leg, for as much as a radian of bend at the loin: what the
/// bend trades between standing the forelegs straight and bending no more
/// than it must.
/// Provenance: **chosen** (#377): two per cent of the leg, so that a
/// shoulder a millimetre off its leg costs the bend about a thousandth of a
/// radian wherever the leg's reach is changing at all, and a rising horse's
/// chest moves through the moment its shoulders come back within reach
/// instead of leaping there.
const REACH_SLACK: f32 = 0.1;

/// Bends the back at the loin until the shoulders stand a straight fore
/// leg's length, `give` of it, off the wrists over the planted paws.
///
/// **The bend that best does it with the least bending, never the end of a
/// range** (#377). Bent further and further, the chest swings the shoulders
/// through the forelegs' reach twice, or only comes near it: a horse rising
/// from its dog-sit, its pelvis pitched 81 degrees, found both crossings at
/// once, took the end of the range and its chest went from 0.22 rad to
/// -0.30 between two frames and its head 906 mm; and where its shoulders
/// first came back within reach, the crossing sprang 0.04 rad up the chest
/// in a thousandth of the motion. So the bend is the one that costs least,
/// a shoulder off its leg's length against a chest turned off the pelvis:
/// where the leg reaches, the crossing nearest the unbent chest to a
/// fraction of a millimetre; where it only just reaches or does not, as
/// near as it comes, moving smoothly between the two.
fn bend_to_fores(rig: &Rig, body: &Body, pose: &mut Pose, pitch: f32, give: f32) -> f32 {
    let leg = &body.fores[0];
    let at = |joint: usize| rig.joints[joint].position;
    let wrist = at(leg.chain[2]);
    let length = at(leg.chain[0]).distance(wrist) * give;
    let slack = (length * REACH_SLACK).max(f32::EPSILON);
    let cost = |pose: &mut Pose, chest: f32| {
        trunk(body, pose, pitch, chest);
        let off = (pose.forward(rig).positions[leg.chain[0]].distance(wrist) - length) / slack;
        off * off + (chest - pitch) * (chest - pitch)
    };
    let (low, high) = (pitch.min(0.0) - 0.3, pitch.max(0.0));
    #[expect(clippy::cast_precision_loss, reason = "a few steps")]
    let along = |step: usize| high + (low - high) * step as f32 / BEND_STEPS as f32;
    let best = (0..=BEND_STEPS)
        .map(|step| (step, cost(pose, along(step))))
        .min_by(|one, two| one.1.total_cmp(&two.1))
        .map_or(0, |(step, _)| step);
    let (mut from, mut to) = (
        along(best.saturating_sub(1)),
        along((best + 1).min(BEND_STEPS)),
    );
    for _ in 0..24 {
        let (one, two) = ((2.0 * from + to) / 3.0, (from + 2.0 * to) / 3.0);
        if cost(pose, one) < cost(pose, two) {
            to = two;
        } else {
            from = one;
        }
    }
    let chest = 0.5 * (from + to);
    trunk(body, pose, pitch, chest);
    chest
}

/// Folds a hind leg onto its pad at `pad` as `fold` says: the hock where
/// the fold puts it, the stifle bending where it does, the metatarsus and
/// the toes laid at the fold's angle.
fn fold_hind(rig: &Rig, pose: &mut Pose, leg: &Leg, hind: &HindLeg, fold: Fold, pad: Vec3) {
    let at = |joint: usize| rig.joints[joint].position;
    let ([_, _, hock], _) = hind.up_from(pad, fold);
    let hock = Vec3::new(at(leg.chain[2]).x, hock.y, hock.z);
    unbend(pose, leg);
    let hip = pose.forward(rig).positions[leg.chain[0]];
    two_bone(rig, pose, leg.chain, hock, hind_pole(hip, hock, hind));
    let carried = pose.forward(rig).rotations[leg.chain[1]];
    let laid = turn(fold.meta - hind.stood.meta);
    pose.rotations[leg.chain[2]] = carried.inverse() * laid;
    if leg.toe.is_some() {
        pose.rotations[leg.pad] = laid.inverse();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::anim::emote;
    use crate::mesh::PolyMesh;
    use crate::plan::{BodyPlan, Composites, Family, QuadrupedParams};
    use crate::rig::Outline;
    use crate::rig::skin::{SkinConfig, bind};

    /// A beast of `family` built as an avatar builds it: its body meshed and
    /// bound, and its rig given the outline of that skin.
    pub(crate) fn built(family: Family) -> (Rig, PolyMesh) {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        let composites = Composites::default();
        let skeleton = params.skeleton(&composites);
        let mut mesh = crate::build_body(
            &skeleton,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &crate::face::HeadTraits::of(&composites),
        )
        .expect("a beast meshes");
        let mut rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = bind(&mesh, &rig, &SkinConfig::default());
        mesh.skin.clone_from(&weights.vertices);
        rig.outline = Some(Outline::measure(&mesh, &weights));
        (rig, mesh)
    }

    /// `clip` at `time` on `rig`, from rest.
    fn at(rig: &Rig, clip: &crate::anim::Clip, time: f32) -> Posed {
        let mut pose = Pose::rest(rig);
        clip.apply(rig, &mut pose, time);
        pose.forward(rig)
    }

    /// The included angle at `joint` between the bones to `from` and `to`.
    fn angle(posed: &Posed, from: usize, joint: usize, to: usize) -> f32 {
        let at = posed.positions[joint];
        (posed.positions[from] - at).angle_between(posed.positions[to] - at)
    }

    /// When each of `readings`, sampled over a clip, has gone half of the
    /// way it goes in all, as a share of the clip.
    fn halfway(readings: &[Vec<f32>]) -> Vec<f32> {
        let count = readings.len();
        (0..readings[0].len())
            .map(|which| {
                let (start, end) = (readings[0][which], readings[count - 1][which]);
                let half = 0.5 * (start + end);
                let crossed = readings
                    .iter()
                    .position(|reading| (reading[which] - half) * (end - start) >= 0.0)
                    .unwrap_or(count - 1);
                #[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
                let share = crossed as f32 / (count - 1) as f32;
                share
            })
            .collect()
    }

    #[test]
    fn getting_up_from_a_sit_unfolds_hip_then_stifle_then_hock() {
        // 09 fact 5 (Ellis2018, measured): rising, the hip extends at 0 to
        // 20% of the movement, the stifle at 10 to 50, the hock at 25 to 60.
        let (rig, _) = built(Family::Dog);
        let (up, _) = emote::getting_up(&rig, "Sit").expect("a dog gets up from a sit");
        let body = Body::read(&rig).expect("a beast");
        let leg = body.hinds[0];
        let readings: Vec<Vec<f32>> = (0..=40)
            .map(|step| {
                #[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
                let posed = at(&rig, &up, step as f32 / 40.0);
                let [hip, stifle, hock] = leg.chain;
                // The hip's angle, femur to the pelvis's run back to the
                // root's own joint.
                let root = body.root;
                let loin = body.spine[0];
                let pelvis = posed.positions[root] - posed.positions[loin];
                let femur = posed.positions[stifle] - posed.positions[hip];
                vec![
                    pelvis.angle_between(femur),
                    angle(&posed, hip, stifle, hock),
                    angle(&posed, stifle, hock, leg.pad),
                ]
            })
            .collect();
        let [hip, stifle, hock] = halfway(&readings)[..] else {
            panic!("three joints")
        };
        assert!(
            hip < stifle && stifle < hock,
            "half open at {hip:.2} (hip), {stifle:.2} (stifle), {hock:.2} (hock)"
        );
    }

    #[test]
    fn a_dog_lies_down_through_its_sit_and_gets_up_back_through_it() {
        // 09 fact 6: dogs and cats lie down through a sit and get up back
        // through it. Either side of the moment the lying takes over from
        // the sit, the body is the sit's end: placed again by the lying's own
        // rule, it stood 161.5 mm off where the sit had left it (#377).
        let (rig, _) = built(Family::Dog);
        let sitting = at(&rig, &emote::sit(), 1.0);
        let lay = emote::lie_down();
        let (up, _) = emote::getting_up(&rig, "Lie Down").expect("a dog gets up");
        // Getting up runs the other way: the sit takes over at a share of the
        // way up, which is its time.
        for (clip, moment) in [(&lay, LIE_SAT[0]), (&up, 1.0 - LIE_SAT[1])] {
            for near in [moment - 1.0e-3, moment + 1.0e-3] {
                let through = at(&rig, clip, near);
                let apart = (0..rig.len())
                    .map(|joint| sitting.positions[joint].distance(through.positions[joint]))
                    .fold(0.0f32, f32::max);
                assert!(
                    apart < 3.0e-3,
                    "at {near:.3} a joint stood {:.1} mm off the sit's end",
                    apart * 1000.0
                );
            }
        }
    }

    #[test]
    fn an_empty_outline_is_no_outline() {
        // #377: an outline with no skin in it - the public default, or one
        // measured over no weights - read as a skin infinitely far down, and
        // a lying trunk set on it stood at y = -inf. It is the joints' rings
        // that stand in for a missing skin, and for an empty one.
        let (mut rig, _) = built(Family::Dog);
        let lay = emote::lie_down();
        rig.outline = None;
        let ringed = lay.pose(&rig, 1.0);
        rig.outline = Some(Outline::default());
        let empty = lay.pose(&rig, 1.0);
        assert!(
            empty.translation.is_finite(),
            "lay at {:?}",
            empty.translation
        );
        assert_eq!(empty, ringed, "an empty outline posed otherwise than none");
    }

    #[test]
    fn a_horse_drops_onto_its_knees_before_its_hindquarters() {
        // 09 fact 10: a horse lies down by flexing all four legs and
        // dropping, its knees touching first.
        let (rig, _) = built(Family::Horse);
        let lay = emote::lie_down();
        let body = Body::read(&rig).expect("a beast");
        let knee = body.fores[0].chain[2];
        let (rest, end) = (at(&rig, &lay, 0.0), at(&rig, &lay, 1.0));
        let share = |posed: &Posed, joint: usize| {
            (rest.positions[joint].y - posed.positions[joint].y)
                / (rest.positions[joint].y - end.positions[joint].y)
        };
        let middle = at(&rig, &lay, 0.6);
        let (knees, hips) = (share(&middle, knee), share(&middle, body.root));
        assert!(
            knees > hips + 0.5,
            "at 0.6 of the way the knees were {knees:.2} of the way down, the hips {hips:.2}"
        );
    }

    #[test]
    fn a_horse_rises_forelegs_first_one_and_then_the_other() {
        // 09 fact 9: a horse rises front first, extending one foreleg and
        // then the other ahead of its chest, its hindquarters after.
        let (rig, _) = built(Family::Horse);
        let (up, _) = emote::getting_up(&rig, "Lie Down").expect("a horse gets up");
        let body = Body::read(&rig).expect("a beast");
        let readings: Vec<Vec<f32>> = (0..=40)
            .map(|step| {
                #[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
                let posed = at(&rig, &up, step as f32 / 40.0);
                vec![
                    posed.positions[body.fores[0].chain[2]].y,
                    posed.positions[body.fores[1].chain[2]].y,
                    posed.positions[body.root].y,
                ]
            })
            .collect();
        let [left, right, hips] = halfway(&readings)[..] else {
            panic!("three readings")
        };
        assert!(
            left < right && right < hips,
            "half way up at {left:.2} (left knee), {right:.2} (right), {hips:.2} (hips)"
        );
    }

    #[test]
    fn a_sitting_dogs_forepaws_stay_where_they_stood() {
        // 09 fact 4: the feet do not move during a sit; the hind paws step
        // forward first (2.1.2), the forepaws stay planted.
        let (rig, _) = built(Family::Dog);
        let body = Body::read(&rig).expect("a beast");
        let sat = emote::sit();
        let moved = (0..=20)
            .map(|step| {
                #[expect(clippy::cast_precision_loss, reason = "a few dozen samples")]
                let posed = at(&rig, &sat, step as f32 / 20.0);
                body.fores
                    .iter()
                    .map(|leg| {
                        let (now, then) = (posed.positions[leg.pad], rig.joints[leg.pad].position);
                        Vec3::new(now.x - then.x, 0.0, now.z - then.z).length()
                    })
                    .fold(0.0f32, f32::max)
            })
            .fold(0.0f32, f32::max);
        assert!(moved < 0.005, "a forepaw slid {:.1} mm", moved * 1000.0);
    }

    #[test]
    fn a_foreleg_sent_where_it_already_stands_is_still_laid() {
        // #377: a leg put back at rest whose paw already stood within half a
        // millimetre of where it was sent was left at rest by the contact
        // solve, its paw not turned to the attitude asked of it: a lying
        // dog's paw passed that close once on its way out, and for that one
        // moment its wrist stood 43 mm off.
        let (rig, _) = built(Family::Dog);
        let body = Body::read(&rig).expect("a beast");
        let leg = body.fores[0];
        let target = rig.joints[leg.pad].position;
        let pole = rig.bend_pole(leg.limb).unwrap_or(target);
        let mut pose = Pose::rest(&rig);
        pose_fore(&rig, &mut pose, &leg, target, pole, 0.5, false);
        let laid = pose.forward(&rig).rotations[leg.chain[2]];
        assert!(
            laid.angle_between(Quat::IDENTITY) > 0.3,
            "the paw was left as it stood, turned {:.3} rad of the 0.785 asked",
            laid.angle_between(Quat::IDENTITY)
        );
    }
}
