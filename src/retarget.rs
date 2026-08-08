//! Bringing motion from the CC0 reference rig onto ours.
//!
//! The reference is a production game skeleton of 66 joints and ours is built
//! from a record, so nothing about them lines up except the anatomy — and the
//! anatomy is the only thing that may be relied on. This module is the mapping
//! and the transfer; [`crate::gltf`] reads the file and
//! [`crate::anim::PoseClip`] is what comes out.
//!
//! # Why not by bone name
//!
//! Because the names lie, and this crate has the receipts. The reference's bone
//! called `head` is the jaw band, and slicing its T-pose at neck height cuts
//! both arms. The table below is hand-authored against measured anatomy for
//! that reason, not because two rigs happened to agree on spelling.
//!
//! **The sides do agree, and that took a fix on our end.** Measured at #139,
//! the reference's `_l` bones sat on the opposite side of `X` from our
//! `Limb::…Left` ones with both bodies facing `+Z` — and glTF's convention
//! (right-handed, `+Y` up, front at `+Z`) puts a character's left at `+X`, so
//! the reference was named correctly and we were not. For as long as that was
//! true this table mapped `_l` to our `…Right`, which looked like a bug and was
//! not. #142 moved our limb names onto the sides they describe, so the table is
//! now plain name to name. Thirty of the library's 162 clips are one-handed, so
//! which way round this goes is not a detail.
//!
//! Our side of the table is a [`Slot`] — a zone and an ordinal — never a joint
//! index, because indices depend on what a body turned out to have.
//!
//! # Why directions for a limb and deltas for a spine
//!
//! The obvious transfer is per-joint: take the rotation the animator applied
//! relative to the reference's rest, and apply it relative to ours. It is cheap,
//! it needs no solver, and it carries twist for free.
//!
//! It does not work for a LIMB, and the measurement that says so is at #139:
//! **the reference rests in a true T-pose and we rest in an A-pose, forty
//! degrees apart at the shoulder.** A delta reproduces our rest plus their
//! motion, so an absolute pose — folded arms, a hand on a hip, a fist at the
//! chin — lands forty degrees out. So bone DIRECTIONS are matched instead, which
//! is scale-free and puts the limb where the animator put it.
//!
//! **And a delta is the only thing that works for the SPINE, which took #143 to
//! find out.** The same rest difference exists in the neck and runs the other
//! way: the reference rests with its head carried **26.2 degrees** forward of
//! vertical and ours rests at **6.4**, so pointing our neck where theirs points
//! imported twenty degrees of somebody else's posture into every clip — whether
//! or not the animator had touched the neck at all. On a body whose neck is also
//! 1.8 times the reference's length, that carried the head 72 mm forward against
//! their 40, and it read as a broken neck rather than as a walk.
//!
//! The distinction is not which is *better*. It is what the two kinds of bone
//! mean. An arm folded across a chest is a place the animator chose and has to
//! be reproduced as a place; a nod is a nod **relative to the shoulders it sits
//! on**, and nobody authors an absolute neck angle. So the carriage is a per-row
//! property of the correspondence table — per row rather than per zone, because
//! the clavicles live in [`Zone::Chest`] with the spine and are limb roots by
//! function.
//!
//! The price is that a direction says nothing about roll about its own axis, and
//! a wave, a forearm and a sword swing are all roll. So the roll is not derived
//! from the direction at all: the transfer applies the reference's whole world
//! rotation first — which carries it — and then makes the smallest correction
//! that puts the bone back on the direction theirs points. That correction is
//! exactly the carriage import, so a relative bone is the same formula with the
//! correction left off. See [`Correspondence::pose_into`], which also records
//! what the first draft got wrong and what it cost.
//!
//! `anim::ik`'s own `retarget` was the obvious thing to reuse and does not
//! fit: it exists to make a chain land on positions an IK solve produced, it
//! deliberately *preserves* whatever twist the limb was already holding rather
//! than setting it, and it has no way to express a joint of ours that the
//! reference has no counterpart for — of which there are four on every body.
//!
//! # What it does not do
//!
//! Put feet on the ground. A baked clip carries the motion as performed; the
//! ground it is performed on belongs to wherever it is played, and
//! [`crate::anim::plant_feet_of`] is what puts it there. Baking a contact solve
//! would freeze one ground plane into a clip that has to play on every other.

use glam::{Mat4, Quat, Vec3};
use thiserror::Error;

use crate::anim::pose::{Pose, Posed};
use crate::anim::pose_clip::{Curve, JointTrack, PoseClip, Slot};
use crate::gltf::{Gltf, GltfError, Skin};
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// How close two samples must be for a track to count as still, in quaternion
/// component units.
///
/// The same unit the curve is stored in, so it means the same thing as the error
/// the packing introduces. Loose enough that a joint jittering under the
/// reference's own quantisation collapses, tight enough that real motion does
/// not: measured on the library, this leaves 45 of 66 tracks held through
/// `Walk`, which is the figure [`PoseClip`] is sized against.
///
/// **Public so that a tool measuring the source's own collapse rate asks the
/// same question with the same number.** `examples/bakeclips` compares how many
/// joints the reference moves against how many tracks we bake, and that
/// comparison means nothing if the two sides use different thresholds — which
/// is what a second copy of this constant would eventually become.
pub const STILL: f32 = 1e-3;

/// Errors raised while retargeting.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum RetargetError {
    /// The reference file could not be read.
    #[error("the reference could not be read: {0}")]
    Gltf(#[from] GltfError),
    /// A bone the correspondence names is not in the reference's skin.
    ///
    /// Raised rather than skipped: a missing bone means the file is not the rig
    /// this table was written against, and a retarget that quietly drops a limb
    /// produces a body with one arm and no error.
    #[error(
        "the reference has no bone called '{name}'; this is not the rig {count} names were written against"
    )]
    NoSuchBone {
        /// What was looked for.
        name: &'static str,
        /// How many the table names in total.
        count: usize,
    },
    /// Our rig has no joint at a slot the correspondence names.
    ///
    /// A body may legitimately lack one — a quadruped has no graspers — so this
    /// carries how many landed, and [`Correspondence::human`] only refuses when
    /// too few did to be the body the table describes.
    #[error(
        "this body filled only {landed} of {count} slots; it is not a humanoid built by this plan"
    )]
    NotThisBody {
        /// How many of our slots resolved.
        landed: usize,
        /// How many the table names.
        count: usize,
    },
}

/// Whether a bone takes the reference's posture or only its motion.
///
/// **The one thing a retarget cannot decide for itself**, because the two rigs
/// rest in different postures and there is no reading of the source that says
/// which of those two the animator meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Carriage {
    /// Point our bone where the reference's bone points.
    ///
    /// What a limb wants. Folded arms, a hand on a hip and a fist at the chin
    /// are absolute positions the animator chose, and reproducing them means
    /// reproducing the angle whatever our own rest happens to be — which at the
    /// shoulder is forty degrees away from theirs, A-pose against T-pose (#139).
    Absolute,
    /// Turn our bone by however much the reference turned its own.
    ///
    /// What the spine wants, and #143 is what it cost to find that out. The
    /// reference rests with its head carried **26.2 degrees** forward of
    /// vertical and ours rests at **6.4**, so pointing our neck where theirs
    /// points imported twenty degrees of somebody else's posture into every clip
    /// — whether or not the animator had touched the neck at all. Nobody authors
    /// an absolute neck angle; a nod is a nod relative to the shoulders it sits
    /// on.
    ///
    /// It is one term in the transfer: the `arc` that drags our bone the rest of
    /// the way onto theirs is exactly the carriage import, so a relative bone is
    /// the same formula without it.
    Relative,
}

/// One reference bone, and the joint of ours it drives.
struct Bone {
    /// What the reference calls it.
    name: &'static str,
    /// Which of our zones it lands in.
    zone: Zone,
    /// Which joint of that zone, in the order [`Rig::in_zone`] returns them.
    ordinal: u8,
    /// Whether it takes the reference's posture or only its motion.
    carriage: Carriage,
}

/// Shorthand for one row of [`HUMAN`] that points where the reference points.
const fn bone(name: &'static str, zone: Zone, ordinal: u8) -> Bone {
    Bone {
        name,
        zone,
        ordinal,
        carriage: Carriage::Absolute,
    }
}

/// Shorthand for one row of [`HUMAN`] that keeps our own posture.
///
/// The axial skeleton, and only it. The clavicles live in [`Zone::Chest`]
/// alongside the spine and are **not** axial: they are limb roots by function,
/// and their rest differs from ours enough that pointing them where the
/// reference points is earning its keep. So this cannot be decided by zone and
/// is decided per row.
const fn axial(name: &'static str, zone: Zone, ordinal: u8) -> Bone {
    Bone {
        name,
        zone,
        ordinal,
        carriage: Carriage::Relative,
    }
}

/// The CC0 reference humanoid, bone by bone.
///
/// **Name to name, side for side** — the reference's `_l` bones and our
/// `Limb::…Left` ones both sit at `+X` on a body facing `+Z`, which is glTF's
/// convention for a character's left. That agreement is younger than this
/// table: until #142 our names were mirrored and these rows read `_l` to our
/// `…Right`.
///
/// Sixty-five of the reference's sixty-six joints are here. The missing one is
/// its `root`, which sits above the pelvis and carries the whole body; our rig is
/// rooted at the pelvis itself, so root motion arrives as
/// [`Pose::translation`] rather than as a joint.
///
/// Four of OUR joints are deliberately absent, because the reference has no
/// counterpart for them: the hand base at each wrist, and the foot node and heel
/// in each ankle. They are structural nodes inside a part rather than bones an
/// animator turned, and they ride their parents rigidly. The transfer expects
/// gaps and reads through them — an ankle's direction runs to the ball whether or
/// not there is a node in between.
///
/// The ordinals are measured off a body built by [`crate::Avatar::build`], not
/// off a plan's skeleton: the plan rigs 33 joints and a built body 73, and the
/// difference is the hands and feet where forty of these names land.
/// Provenance: **measured** (#139) — `examples/retargetaudit` prints both
/// skeletons, and every row below was read off that table.
#[rustfmt::skip]
const HUMAN: &[Bone] = &[
    // The spine, seven against seven — and every one of them RELATIVE, so a
    // body keeps its own carriage and gains the reference's motion (#143).
    axial("pelvis",    Zone::Pelvis,  0),
    axial("spine_01",  Zone::Abdomen, 0),
    axial("spine_02",  Zone::Chest,   0),
    axial("spine_03",  Zone::Chest,   1),
    axial("neck_01",   Zone::Neck,    0),
    axial("head",      Zone::Head,    0),
    axial("head_leaf", Zone::Head,    1),

    // The reference's LEFT arm onto our left arm. Our clavicle at +X is
    // Chest[3] — the clavicles are the one pair addressed by ordinal rather than
    // by `Limb`, because both of them live in `Zone::Chest`. The wrist is what
    // the reference calls `hand`, because that is the joint its fingers hang
    // from.
    bone("clavicle_l", Zone::Chest, 3),
    bone("upperarm_l", Zone::UpperLimb(Limb::ForeLeft), 0),
    bone("lowerarm_l", Zone::UpperLimb(Limb::ForeLeft), 1),
    bone("hand_l",     Zone::LowerLimb(Limb::ForeLeft), 0),
    bone("index_01_l",      Zone::Extremity(Limb::ForeLeft),  1),
    bone("index_02_l",      Zone::Extremity(Limb::ForeLeft),  2),
    bone("index_03_l",      Zone::Extremity(Limb::ForeLeft),  3),
    bone("index_04_leaf_l", Zone::Extremity(Limb::ForeLeft),  4),
    bone("middle_01_l",      Zone::Extremity(Limb::ForeLeft),  5),
    bone("middle_02_l",      Zone::Extremity(Limb::ForeLeft),  6),
    bone("middle_03_l",      Zone::Extremity(Limb::ForeLeft),  7),
    bone("middle_04_leaf_l", Zone::Extremity(Limb::ForeLeft),  8),
    bone("ring_01_l",      Zone::Extremity(Limb::ForeLeft),  9),
    bone("ring_02_l",      Zone::Extremity(Limb::ForeLeft), 10),
    bone("ring_03_l",      Zone::Extremity(Limb::ForeLeft), 11),
    bone("ring_04_leaf_l", Zone::Extremity(Limb::ForeLeft), 12),
    bone("pinky_01_l",      Zone::Extremity(Limb::ForeLeft), 13),
    bone("pinky_02_l",      Zone::Extremity(Limb::ForeLeft), 14),
    bone("pinky_03_l",      Zone::Extremity(Limb::ForeLeft), 15),
    bone("pinky_04_leaf_l", Zone::Extremity(Limb::ForeLeft), 16),
    bone("thumb_01_l",      Zone::Extremity(Limb::ForeLeft), 17),
    bone("thumb_02_l",      Zone::Extremity(Limb::ForeLeft), 18),
    bone("thumb_03_l",      Zone::Extremity(Limb::ForeLeft), 19),
    bone("thumb_04_leaf_l", Zone::Extremity(Limb::ForeLeft), 20),

    // And its right arm onto ours, whose clavicle is the one at −X.
    bone("clavicle_r", Zone::Chest, 2),
    bone("upperarm_r", Zone::UpperLimb(Limb::ForeRight), 0),
    bone("lowerarm_r", Zone::UpperLimb(Limb::ForeRight), 1),
    bone("hand_r",     Zone::LowerLimb(Limb::ForeRight), 0),
    bone("index_01_r",      Zone::Extremity(Limb::ForeRight),  1),
    bone("index_02_r",      Zone::Extremity(Limb::ForeRight),  2),
    bone("index_03_r",      Zone::Extremity(Limb::ForeRight),  3),
    bone("index_04_leaf_r", Zone::Extremity(Limb::ForeRight),  4),
    bone("middle_01_r",      Zone::Extremity(Limb::ForeRight),  5),
    bone("middle_02_r",      Zone::Extremity(Limb::ForeRight),  6),
    bone("middle_03_r",      Zone::Extremity(Limb::ForeRight),  7),
    bone("middle_04_leaf_r", Zone::Extremity(Limb::ForeRight),  8),
    bone("ring_01_r",      Zone::Extremity(Limb::ForeRight),  9),
    bone("ring_02_r",      Zone::Extremity(Limb::ForeRight), 10),
    bone("ring_03_r",      Zone::Extremity(Limb::ForeRight), 11),
    bone("ring_04_leaf_r", Zone::Extremity(Limb::ForeRight), 12),
    bone("pinky_01_r",      Zone::Extremity(Limb::ForeRight), 13),
    bone("pinky_02_r",      Zone::Extremity(Limb::ForeRight), 14),
    bone("pinky_03_r",      Zone::Extremity(Limb::ForeRight), 15),
    bone("pinky_04_leaf_r", Zone::Extremity(Limb::ForeRight), 16),
    bone("thumb_01_r",      Zone::Extremity(Limb::ForeRight), 17),
    bone("thumb_02_r",      Zone::Extremity(Limb::ForeRight), 18),
    bone("thumb_03_r",      Zone::Extremity(Limb::ForeRight), 19),
    bone("thumb_04_leaf_r", Zone::Extremity(Limb::ForeRight), 20),

    // The legs, name to name as well. `foot` is the ankle; `ball` and its leaf
    // are the two nodes ours carries forward of it.
    bone("thigh_l",     Zone::UpperLimb(Limb::HindLeft), 0),
    bone("calf_l",      Zone::UpperLimb(Limb::HindLeft), 1),
    bone("foot_l",      Zone::LowerLimb(Limb::HindLeft), 0),
    bone("ball_l",      Zone::Extremity(Limb::HindLeft), 2),
    bone("ball_leaf_l", Zone::Extremity(Limb::HindLeft), 3),
    bone("thigh_r",     Zone::UpperLimb(Limb::HindRight), 0),
    bone("calf_r",      Zone::UpperLimb(Limb::HindRight), 1),
    bone("foot_r",      Zone::LowerLimb(Limb::HindRight), 0),
    bone("ball_r",      Zone::Extremity(Limb::HindRight), 2),
    bone("ball_leaf_r", Zone::Extremity(Limb::HindRight), 3),
];

/// How few slots may land before a body is not the one [`HUMAN`] describes.
///
/// A share rather than a count, and low enough that a body missing a hand still
/// retargets what it has. Below it, the likelier explanation is that the caller
/// passed a quadruped or a plan's skeleton instead of a built body — the second
/// of which lands only the trunk and would otherwise produce a clip with no
/// hands in it and no error.
const ENOUGH: f32 = 0.75;

/// One joint of ours, the reference joint it follows, and where each points.
struct Pair {
    /// Our joint.
    ours: usize,
    /// The reference's document node.
    theirs: usize,
    /// The next mapped joint down the chain, ours and theirs.
    ///
    /// `None` for a leaf. A leaf's rotation moves nothing on the body — see
    /// [`crate::rig::skin`] — so it is left at rest rather than guessed at.
    toward: Option<(usize, usize)>,
    /// Whether this bone takes the reference's posture or only its motion.
    carriage: Carriage,
}

/// The reference rig, matched to one of ours.
pub struct Correspondence {
    /// Mapped joints in our rig's order, which is parents before children.
    pairs: Vec<Pair>,
    /// Our joint for each mapped pair, indexed by our joint.
    of_joint: Vec<Option<usize>>,
    /// The reference's rest world transforms.
    rest: Vec<Mat4>,
    /// Our root joint, whose motion becomes [`Pose::translation`].
    root: usize,
    /// The reference's root joint.
    their_root: usize,
    /// How much shorter or taller we are, as a ratio of hip height.
    ///
    /// Root motion is the one quantity in a transfer that is NOT scale-free: a
    /// stride is metres and a rotation is not. Taken at the hip because that is
    /// what sets both stride and how far a body bobs.
    scale: f32,
}

impl Correspondence {
    /// Matches the CC0 reference humanoid to a built body's rig.
    ///
    /// # Errors
    ///
    /// Returns [`RetargetError::NoSuchBone`] if the file is not the rig this
    /// table was written against, and [`RetargetError::NotThisBody`] if too few
    /// of our slots landed for the body to be a humanoid built by this plan —
    /// which is what a plan's skeleton, rather than a built body, looks like.
    pub fn human(rig: &Rig, library: &Gltf, skin: &Skin) -> Result<Self, RetargetError> {
        let node_of = |name: &'static str| -> Result<usize, RetargetError> {
            skin.names
                .iter()
                .position(|had| had == name)
                .map(|at| skin.nodes[at])
                .ok_or(RetargetError::NoSuchBone {
                    name,
                    count: HUMAN.len(),
                })
        };

        // Our joint for each reference bone, and the reverse. Both are needed:
        // the walk goes down OUR hierarchy and has to find the reference joint
        // beside each of ours.
        let mut theirs_of_ours: Vec<Option<(usize, Carriage)>> = vec![None; rig.len()];
        let mut landed = 0usize;
        for entry in HUMAN {
            let node = node_of(entry.name)?;
            if let Some(joint) = Slot::new(entry.zone, entry.ordinal).resolve(rig) {
                theirs_of_ours[joint] = Some((node, entry.carriage));
                landed += 1;
            }
        }
        if (landed as f32) < HUMAN.len() as f32 * ENOUGH {
            return Err(RetargetError::NotThisBody {
                landed,
                count: HUMAN.len(),
            });
        }

        // The next mapped joint down each chain, found by walking OUR hierarchy
        // rather than by naming it. That is what reads through the four joints
        // the reference has no counterpart for: an ankle's direction runs to the
        // ball whether or not a node sits in between.
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); rig.len()];
        for joint in 0..rig.len() {
            if let Some(parent) = rig.joints[joint].parent {
                children[parent].push(joint);
            }
        }

        let mut pairs = Vec::with_capacity(landed);
        let mut of_joint = vec![None; rig.len()];
        for ours in 0..rig.len() {
            let Some((theirs, carriage)) = theirs_of_ours[ours] else {
                continue;
            };
            of_joint[ours] = Some(pairs.len());
            pairs.push(Pair {
                ours,
                theirs,
                toward: next_mapped(ours, &children, &theirs_of_ours),
                carriage,
            });
        }

        let root = 0;
        let their_root = node_of("pelvis")?;
        let rest = library.rest()?;
        let our_hip = rig.joints[root].position.y;
        let their_hip = rest[their_root].transform_point3(Vec3::ZERO).y;
        Ok(Self {
            pairs,
            of_joint,
            rest,
            root,
            their_root,
            scale: if their_hip.abs() > f32::EPSILON {
                our_hip / their_hip
            } else {
                1.0
            },
        })
    }

    /// How many joints it matched.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether it matched none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// How far each of our bones points from the one it follows, in degrees.
    ///
    /// **The transfer's whole contract as a number**, and the reason it is a
    /// method rather than arithmetic repeated in a test and an audit: a check
    /// that recomputes a contract from its own copy of the definition stops
    /// meaning anything the first time one of them moves.
    ///
    /// What it catches is a wrong parent frame, and it catches it in the awkward
    /// direction: getting that wrong doubles a correction rather than dropping
    /// it, so the body arrives in a plausible pose that is bent twice as far as
    /// the one it was copying.
    #[must_use]
    pub fn bone_errors(&self, posed: &[Mat4], ours: &Posed) -> Vec<(usize, f32)> {
        let at = |node: usize| posed[node].transform_point3(Vec3::ZERO);
        self.pairs
            .iter()
            .filter_map(|pair| {
                let (child_ours, child_theirs) = pair.toward?;
                let wants = (at(child_theirs) - at(pair.theirs)).normalize_or_zero();
                let got =
                    (ours.positions[child_ours] - ours.positions[pair.ours]).normalize_or_zero();
                if wants == Vec3::ZERO || got == Vec3::ZERO {
                    return None;
                }
                Some((pair.ours, got.angle_between(wants).to_degrees()))
            })
            .collect()
    }

    /// Writes the reference's pose onto ours.
    ///
    /// `posed` is the reference's world transforms at one instant, as
    /// [`Gltf::sample`] returns them.
    ///
    /// **Their whole world rotation, then the smallest correction that fixes the
    /// direction.** In world space, our joint's orientation becomes
    ///
    /// ```text
    ///   W = arc(D · was, wants) · D        D = their posed rotation / their rest one
    /// ```
    ///
    /// `D` is everything the animator did to that bone since the rest pose,
    /// **including the roll**, which is the half a direction cannot carry — a
    /// wave, a forearm's pronation and a sword swing are all roll. Applying `D`
    /// first is what brings it across. The correction after it is the minimal
    /// rotation putting our bone back on the direction theirs actually points,
    /// and it is needed only because our rest pose is not theirs: forty degrees
    /// apart at the shoulder, A-pose against T-pose (#139).
    ///
    /// **The `arc` is dropped for a bone marked relative in the table**, leaving
    /// `W = D`: our own rest direction turned by exactly what the animator did.
    /// That is what the spine wants, and the module docs carry the 26.2 against
    /// 6.4 degrees that made it necessary (#143).
    ///
    /// **The order matters and the first draft had it wrong.** Reading the roll
    /// out of `D` as an angle and applying it about our own bone in the LOCAL
    /// frame mixes two frames, and the cost is not subtle: a finger rigidly
    /// carried by a hand picked up the roll its parent underwent, so it wobbled
    /// where the source held it still. Measured, that turned 21 moving tracks
    /// into 52 of 65 — caught by the bake's own collapse rate rather than by
    /// anything visible.
    ///
    /// The formulation above has the property that fixes it: for a joint the
    /// source holds rigid, `D` and the parent's `D` are the same rotation, so the
    /// local rotation left after dividing the parent out is **constant**, and the
    /// track collapses the way it should.
    ///
    /// Joints with no counterpart, and leaves, keep their rest rotation and are
    /// carried rigidly by their parents.
    pub fn pose_into(&self, rig: &Rig, posed: &[Mat4], pose: &mut Pose) {
        let at = |world: &[Mat4], node: usize| world[node].transform_point3(Vec3::ZERO);
        let spin = |world: &[Mat4], node: usize| {
            let (_, rotation, _) = world[node].to_scale_rotation_translation();
            rotation
        };
        let mut world = vec![Quat::IDENTITY; rig.len()];

        for joint in 0..rig.len() {
            let parent = rig.joints[joint]
                .parent
                .map_or(Quat::IDENTITY, |parent| world[parent]);
            let mine = self
                .of_joint
                .get(joint)
                .copied()
                .flatten()
                .and_then(|pair| {
                    let pair = &self.pairs[pair];
                    let (child_ours, child_theirs) = pair.toward?;
                    let was = (rig.joints[child_ours].position - rig.joints[pair.ours].position)
                        .normalize_or_zero();
                    let wants =
                        (at(posed, child_theirs) - at(posed, pair.theirs)).normalize_or_zero();
                    if was == Vec3::ZERO || wants == Vec3::ZERO {
                        return None;
                    }
                    let moved = spin(posed, pair.theirs) * spin(&self.rest, pair.theirs).inverse();
                    Some(match pair.carriage {
                        // The arc is the carriage: it drags our bone the rest of
                        // the way onto theirs, which is what a limb wants and
                        // what a spine must not have (#143).
                        Carriage::Absolute => Quat::from_rotation_arc(moved * was, wants) * moved,
                        Carriage::Relative => moved,
                    })
                })
                .unwrap_or(parent);

            if joint < pose.rotations.len() {
                pose.rotations[joint] = parent.inverse() * mine;
            }
            world[joint] = mine;
        }

        // Root motion is the one part of a transfer that is NOT scale-free: a
        // stride is metres where a rotation is not.
        if self.root < pose.rotations.len() {
            let moved = at(posed, self.their_root) - at(&self.rest, self.their_root);
            pose.translation = moved * self.scale;
        }
    }
}

/// Bakes one of the reference's animations against a body.
///
/// Sampled at `rate` frames a second rather than at the source's own keys: it is
/// two thirds `STEP` samplers at irregular times, so resampling loses nothing and
/// buys a flat array with no times beside it. `looping` decides whether the last
/// frame runs back into the first.
///
/// # Errors
///
/// Returns [`RetargetError`] if the animation does not exist or cannot be
/// sampled.
pub fn clip(
    rig: &Rig,
    library: &Gltf,
    matched: &Correspondence,
    animation: usize,
    rate: f32,
    looping: bool,
) -> Result<PoseClip, RetargetError> {
    let name = library
        .clip_names()
        .get(animation)
        .map(|name| (*name).to_string())
        .unwrap_or_default();
    let duration = library.duration(animation)?;
    // At least one frame: a pose-only entry is a real thing in this library, and
    // a clip of no frames poses nothing.
    let frames = ((duration * rate).round() as usize).max(1);

    let mut samples: Vec<Vec<Quat>> = vec![Vec::with_capacity(frames); rig.len()];
    let mut root = Vec::with_capacity(frames);
    let mut pose = Pose::rest(rig);
    for frame in 0..frames {
        let time = if frames > 1 {
            duration * frame as f32 / frames as f32
        } else {
            0.0
        };
        let posed = library.sample(animation, time)?;
        matched.pose_into(rig, &posed, &mut pose);
        for (joint, rotation) in pose.rotations.iter().enumerate() {
            samples[joint].push(*rotation);
        }
        root.push(pose.translation);
    }

    // Only the joints the correspondence actually drives get a track. Every
    // other joint of ours is at rest through the whole clip, and a track saying
    // so is a track that does nothing at eight bytes a frame.
    let mut tracks = Vec::new();
    for pair in &matched.pairs {
        let Some(slot) = slot_of(rig, pair.ours) else {
            continue;
        };
        tracks.push(JointTrack {
            slot,
            rotation: Curve::bake(&samples[pair.ours], STILL),
        });
    }
    let still = root.iter().all(|at| at.abs_diff_eq(root[0], 1e-5));
    Ok(PoseClip {
        name,
        rate,
        frames,
        looping,
        tracks,
        root: if still { Vec::new() } else { root },
    })
}

/// The slot naming one of our joints, if a zone and ordinal can reach it.
fn slot_of(rig: &Rig, joint: usize) -> Option<Slot> {
    let zone = rig.joints[joint].zone;
    let ordinal = rig.in_zone(zone).iter().position(|&had| had == joint)?;
    u8::try_from(ordinal)
        .ok()
        .map(|ordinal| Slot::new(zone, ordinal))
}

/// The nearest mapped descendant of a joint, and the reference joint beside it.
///
/// Breadth-first, so a joint with several mapped descendants takes the nearest
/// rather than the first branch it happens to walk into — a wrist has five, and
/// the one that should define its direction is whichever finger root sits
/// closest down the tree.
fn next_mapped(
    from: usize,
    children: &[Vec<usize>],
    theirs_of_ours: &[Option<(usize, Carriage)>],
) -> Option<(usize, usize)> {
    let mut queue: Vec<usize> = children[from].clone();
    let mut at = 0usize;
    while at < queue.len() {
        let joint = queue[at];
        at += 1;
        if let Some((theirs, _)) = theirs_of_ours[joint] {
            return Some((joint, theirs));
        }
        queue.extend_from_slice(&children[joint]);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf::Gltf;

    /// Where the CC0 reference animations sit, relative to this checkout.
    ///
    /// Not vendored: eleven megabytes of someone else's CC0. Every test that
    /// reads them skips with a printed reason when they are absent.
    const LIBRARY: &str = "../mesh2motion-app/static/animations/human-base-animations.glb";

    /// A clip performed with one hand, for the test that asks which hand.
    ///
    /// A cross is thrown with the rear hand and the other stays at the guard,
    /// which is about as one-sided as this library gets. Thirty of its 162
    /// clips are one-handed; any of them would do, and this one is named for
    /// the punch rather than for a side, which is the point.
    const CROSS: &str = "Punch_Cross";

    /// The reference library and a body matched to it, or `None` to skip.
    fn matched() -> Option<(Gltf, Correspondence, crate::Avatar)> {
        let Ok(bytes) = std::fs::read(LIBRARY) else {
            eprintln!("skipping: {LIBRARY} is not checked out beside this repository");
            return None;
        };
        let library = Gltf::read(&bytes).expect("the reference reads");
        let skin = library.skin(0).expect("the reference has a skin");
        let record = crate::AvatarRecord::new("Retargeted", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let matched =
            Correspondence::human(&avatar.rig, &library, &skin).expect("the reference matches");
        Some((library, matched, avatar))
    }

    #[test]
    fn the_clavicles_are_pinned_to_their_sides_by_ordinal() {
        // **The one place in the crate where a side rides on an ordinal.** Both
        // clavicles live in `Zone::Chest`, so `HUMAN` can only address them as
        // `Chest[2]` and `Chest[3]` — and which of them is which is decided by
        // the order `plan::humanoid` inserts its two limbs, since
        // `Rig::from_skeleton` numbers breadth-first and siblings keep insertion
        // order. Reordering those two rows is a tidy anybody might make, and it
        // would put every clip's shoulders on the wrong sides with no other
        // symptom. This is what stops it.
        //
        // Needs no reference file: it is a fact about our own rig.
        let record = crate::AvatarRecord::new("Pinned", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let chest = avatar.rig.in_zone(Zone::Chest);
        assert_eq!(chest.len(), 4, "spine, spine, and two clavicles");
        assert!(
            avatar.rig.joints[chest[2]].position.x < 0.0,
            "Chest[2] is the clavicle at -X, which is the body's RIGHT"
        );
        assert!(
            avatar.rig.joints[chest[3]].position.x > 0.0,
            "Chest[3] is the clavicle at +X, which is the body's LEFT"
        );

        // And the rows that read those ordinals still say so.
        let of = |name: &str| {
            HUMAN
                .iter()
                .find(|bone| bone.name == name)
                .expect("a clavicle row")
                .ordinal
        };
        assert_eq!(of("clavicle_l"), 3, "the reference's left is our +X");
        assert_eq!(of("clavicle_r"), 2);
    }

    #[test]
    fn every_bone_of_the_reference_finds_a_joint_of_ours() {
        let Some((_, matched, _)) = matched() else {
            return;
        };
        // 65 of the reference's 66: its `root` sits above the pelvis and arrives
        // as root translation rather than as a joint.
        assert_eq!(
            matched.len(),
            HUMAN.len(),
            "the correspondence dropped joints silently"
        );
        assert_eq!(HUMAN.len(), 65);
    }

    #[test]
    fn a_plans_skeleton_is_refused_rather_than_silently_fingerless() {
        // **The mistake this exists to catch.** `Rig::from_skeleton` gives 33
        // joints and `Avatar::build` gives 73; the difference is the hands and
        // feet, where forty of the reference's sixty-six names land. Retargeting
        // against the wrong one produces a clip with no fingers and no error.
        let Ok(bytes) = std::fs::read(LIBRARY) else {
            eprintln!("skipping: {LIBRARY} is not checked out beside this repository");
            return;
        };
        let library = Gltf::read(&bytes).expect("the reference reads");
        let skin = library.skin(0).expect("a skin");
        let record = crate::AvatarRecord::default();
        let bare = Rig::from_skeleton(&record.skeleton()).expect("a humanoid rigs");
        assert!(matches!(
            Correspondence::human(&bare, &library, &skin),
            Err(RetargetError::NotThisBody { .. })
        ));
    }

    #[test]
    fn a_file_that_is_not_the_reference_rig_is_refused_by_name() {
        let record = crate::AvatarRecord::default();
        let rig = Rig::from_skeleton(&record.skeleton()).expect("a humanoid rigs");
        // A GLB with a skin that has none of the names the table expects.
        let library =
            Gltf::read(&crate::gltf::tests::a_two_joint_glb()).expect("the fixture reads");
        let skin = library.skin(0).expect("a skin");
        assert!(matches!(
            Correspondence::human(&rig, &library, &skin),
            Err(RetargetError::NoSuchBone { .. })
        ));
    }

    #[test]
    fn a_retargeted_limb_points_where_the_reference_points_its_own() {
        // The contract of the swing, measured as an angle rather than trusted:
        // for every mapped LIMB bone with a child, the direction ours ends up
        // pointing must match the direction theirs points, to within the
        // arithmetic's noise. This is what a wrong parent frame breaks, and it
        // breaks it by doubling a correction rather than by ignoring it — so the
        // failure is a limb bent twice as far, which reads as a plausible pose.
        //
        // **Limbs only, and the axial bones get their own assertion below rather
        // than a loosened threshold here.** They are [`Carriage::Relative`] and
        // deliberately do NOT point where the reference points (#143); folding
        // them in would mean raising this bound to twenty degrees, at which
        // point it would no longer catch the defect it was written for.
        let Some((library, matched, avatar)) = matched() else {
            return;
        };
        let rig = &avatar.rig;
        let walk = library.clip("Walk").expect("the reference has a Walk");
        let duration = library.duration(walk).expect("a duration");

        let mut worst: f32 = 0.0;
        let mut checked = 0usize;
        for step in 0..8 {
            let time = duration * step as f32 / 8.0;
            let posed = library.sample(walk, time).expect("a pose");
            let mut pose = Pose::rest(rig);
            matched.pose_into(rig, &posed, &mut pose);
            let ours = pose.forward(rig);

            for (joint, off) in matched.bone_errors(&posed, &ours) {
                if rig.joints[joint].zone.is_core() {
                    continue;
                }
                checked += 1;
                worst = worst.max(off);
            }
        }
        assert!(
            checked > 350,
            "only {checked} limb bones were read; the filter, not the retarget, is what \
             this measures"
        );
        assert!(
            worst < 0.5,
            "a retargeted limb bone points {worst:.2} degrees away from the one it follows, \
             of {checked} read"
        );
    }

    #[test]
    fn the_spine_keeps_our_carriage_rather_than_the_reference_s() {
        // **The other half, and it has to be asserted in both directions.** A
        // relative bone must not point where the reference's points -- that is
        // the whole object -- but it must not wander either: it should sit near
        // OUR rest direction, turned by what the animator actually did.
        //
        // Measured at #143: the reference rests with its neck 26.2 degrees
        // forward of vertical and ours rests at 6.4, so an absolute transfer
        // dragged twenty degrees of somebody else's posture into every clip.
        let Some((library, matched, avatar)) = matched() else {
            return;
        };
        let rig = &avatar.rig;
        let walk = library.clip("Walk").expect("the reference has a Walk");
        let duration = library.duration(walk).expect("a duration");

        let skin = library.skin(0).expect("a skin");
        let node = |name: &str| {
            skin.names
                .iter()
                .position(|had| had == name)
                .map(|at| skin.nodes[at])
                .expect("a named bone")
        };
        let rest = library.rest().expect("a rest pose");
        let neck = Slot::new(Zone::Neck, 0).resolve(rig).expect("a neck");
        let head = Slot::new(Zone::Head, 0).resolve(rig).expect("a head");
        let lean = |from: Vec3, to: Vec3| (to - from).z.atan2((to - from).y).to_degrees();
        let at = |world: &[Mat4], node: usize| world[node].transform_point3(Vec3::ZERO);

        // **Each rig's drift from its OWN rest**, which is what a relative
        // transfer promises to carry across. Comparing absolute leans would only
        // restate the twenty degrees of carriage that this exists to remove.
        let our_rest = lean(rig.joints[neck].position, rig.joints[head].position);
        let their_rest = lean(at(&rest, node("neck_01")), at(&rest, node("head")));

        let mut worst_apart: f32 = 0.0;
        let mut moved: f32 = 0.0;
        let mut disagreed: f32 = 0.0;
        for step in 0..8 {
            let time = duration * step as f32 / 8.0;
            let posed = library.sample(walk, time).expect("a pose");
            let mut pose = Pose::rest(rig);
            matched.pose_into(rig, &posed, &mut pose);
            let ours = pose.forward(rig);

            let theirs = lean(at(&posed, node("neck_01")), at(&posed, node("head"))) - their_rest;
            let mine = lean(ours.positions[neck], ours.positions[head]) - our_rest;
            worst_apart = worst_apart.max((mine - theirs).abs());
            moved = moved.max(theirs.abs());

            for (joint, off) in matched.bone_errors(&posed, &ours) {
                if rig.joints[joint].zone.is_core() {
                    disagreed = disagreed.max(off);
                }
            }
        }

        // The animator's own neck motion has to be worth carrying, or agreeing
        // about it says nothing — a transfer that froze the neck solid would
        // pass an agreement test against a source that never moved it.
        assert!(
            moved > 3.0,
            "the reference barely moves its neck through this clip ({moved:.1} degrees), \
             so agreeing with it proves nothing"
        );
        assert!(
            worst_apart < 2.0,
            "our neck's drift from its own rest disagrees with the reference's by \
             {worst_apart:.1} degrees; a relative bone should carry the motion and \
             nothing else"
        );
        // And it must actually differ from the reference in ABSOLUTE terms, or
        // the arc is still being applied and the distinction does nothing.
        assert!(
            disagreed > 10.0,
            "the axial bones still point within {disagreed:.1} degrees of the reference's, \
             so the carriage is not being kept after all"
        );
    }

    #[test]
    fn a_one_sided_clip_lands_on_the_hand_that_performed_it() {
        // **The one thing the audit's numbers cannot say.** Joints matched,
        // worst bone error, moving-track count and baked size are every one of
        // them invariant under a mirror, so a correspondence with its sides
        // swapped scores exactly as well as a correct one — which is how #142
        // went unnoticed until somebody measured X. This asks the only question
        // that separates them: the reference punches with ONE hand, so which of
        // ours moves?
        //
        // Asked of the motion rather than of the clip's title, because a title
        // is somebody else's word for a side and this crate's whole standing
        // finding is that names lie. Travel is measured against the pelvis so
        // that root motion, which both hands share, cannot drown the answer.
        let Some((library, matched, avatar)) = matched() else {
            return;
        };
        let skin = library.skin(0).expect("a skin");
        let animation = library.clip(CROSS).expect("the reference has a cross");
        let node = |name: &str| {
            skin.names
                .iter()
                .position(|had| had == name)
                .map(|at| skin.nodes[at])
                .expect("a named bone")
        };
        let (their_left, their_right, their_root) =
            (node("hand_l"), node("hand_r"), node("pelvis"));

        let rig = &avatar.rig;
        let ours = |limb| {
            Slot::new(Zone::LowerLimb(limb), 0)
                .resolve(rig)
                .expect("a wrist")
        };
        let (our_left, our_right) = (ours(Limb::ForeLeft), ours(Limb::ForeRight));

        let baked = clip(rig, &library, &matched, animation, 30.0, false).expect("it retargets");
        let frames = 24;
        let mut travel = [0.0f32; 4];
        let mut last: Option<[Vec3; 4]> = None;
        for frame in 0..frames {
            let time = baked.duration() * frame as f32 / frames as f32;
            let posed = library.sample(animation, time).expect("the source samples");
            let at = |node: usize| posed[node].transform_point3(Vec3::ZERO);
            let their_hip = at(their_root);

            let ours_posed = baked.pose(rig, time).forward(rig);
            let our_hip = ours_posed.positions[0];

            let now = [
                at(their_left) - their_hip,
                at(their_right) - their_hip,
                ours_posed.positions[our_left] - our_hip,
                ours_posed.positions[our_right] - our_hip,
            ];
            if let Some(before) = last {
                // In millimetres, because both rigs are in metres and a figure
                // printed as "1" against "1" says nothing about a margin.
                for (sum, (a, b)) in travel.iter_mut().zip(before.iter().zip(now.iter())) {
                    *sum += a.distance(*b) * 1000.0;
                }
            }
            last = Some(now);
        }

        // A margin, not a tie-break: the idle hand in a cross still drifts, so
        // the busy one has to be clearly busier before either body is said to
        // have a punching side at all. Measured on Punch_Cross, the reference's
        // busy hand travels about four times its other.
        let clear = |busy: f32, idle: f32| busy > idle * 1.5;
        let [their_l, their_r, our_l, our_r] = travel;
        assert!(
            clear(their_l, their_r) || clear(their_r, their_l),
            "{CROSS} is not one-sided on the reference: hand_l travelled \
             {their_l:.0} mm and hand_r {their_r:.0} mm, so it cannot judge a side"
        );
        assert_eq!(
            their_l > their_r,
            our_l > our_r,
            "the reference punched with its {} hand and we punched with our {} \
             one: the correspondence is mirrored. Reference travel {their_l:.0} \
             mm left against {their_r:.0} right; ours {our_l:.0} against {our_r:.0}",
            if their_l > their_r { "left" } else { "right" },
            if our_l > our_r { "left" } else { "right" }
        );
        assert!(
            clear(our_l.max(our_r), our_l.min(our_r)),
            "our punch is not one-sided at all: {our_l:.0} mm against {our_r:.0}"
        );
    }

    #[test]
    fn the_twist_survives_a_clip_that_is_mostly_roll() {
        // **A twist bug is invisible in a walk**, which is why this does not use
        // one. Rolling a forearm moves the hand's own axes and barely moves its
        // position, so the direction assertion above passes at any twist
        // whatsoever — including none. What catches it is comparing the
        // reference's own roll about a bone against ours about the same bone.
        let Some((library, matched, avatar)) = matched() else {
            return;
        };
        let rig = &avatar.rig;
        let Some(clip) = library
            .clip("Pistol_Aim_Up")
            .or_else(|| library.clip("Idle_Talking"))
        else {
            eprintln!("skipping: the library has no roll-heavy clip to check against");
            return;
        };
        let duration = library.duration(clip).expect("a duration");

        // The forearm: our wrist follows the reference's `hand`, and the roll
        // about it is pronation, which no direction can express.
        let wrist = Slot::new(Zone::LowerLimb(Limb::ForeRight), 0)
            .resolve(rig)
            .expect("a wrist");
        let mut rolled: f32 = 0.0;
        for step in 0..12 {
            let time = duration * step as f32 / 12.0;
            let posed = library.sample(clip, time).expect("a pose");
            let mut pose = Pose::rest(rig);
            matched.pose_into(rig, &posed, &mut pose);

            // How much of our own joint's local rotation is roll about the bone
            // it turns. Zero at every frame means the twist was never applied.
            let (axis, angle) = pose.rotations[wrist].to_axis_angle();
            let child = matched.pairs[matched.of_joint[wrist].expect("a mapped wrist")]
                .toward
                .expect("a wrist points somewhere");
            let bone = (rig.joints[child.0].position - rig.joints[wrist].position).normalize();
            rolled = rolled.max((angle * axis.dot(bone)).abs().to_degrees());
        }
        assert!(
            rolled > 5.0,
            "the wrist rolled at most {rolled:.2} degrees across a clip that is mostly roll; \
             the twist is not being transferred"
        );
    }

    #[test]
    fn a_baked_clip_holds_the_joints_that_do_not_move() {
        let Some((library, matched, avatar)) = matched() else {
            return;
        };
        let rig = &avatar.rig;
        let walk = library.clip("Walk").expect("a Walk");
        let baked = clip(rig, &library, &matched, walk, 30.0, true).expect("it bakes");

        assert_eq!(baked.name, "Walk");
        assert!(baked.frames > 10, "a walk of {} frames", baked.frames);
        assert_eq!(baked.tracks.len(), matched.len());
        // The measurement the format is sized against: most tracks are still,
        // because most joints are fingers and a walking body does not use them.
        assert!(
            baked.moving() < baked.tracks.len() / 2,
            "{} of {} tracks move; the collapse is not working",
            baked.moving(),
            baked.tracks.len()
        );
        // And a walk carries root motion, which is what makes it a walk.
        assert!(!baked.root.is_empty(), "a walk that does not travel");
    }
}
