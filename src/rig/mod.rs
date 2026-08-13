//! Turning a capsule graph into something that can be posed.
//!
//! A [`Skeleton`] is an undirected graph, which is the right shape for meshing
//! but not for animation: posing needs a hierarchy, so that rotating a shoulder
//! carries the whole arm with it. A [`Rig`] is that hierarchy — the same nodes,
//! rooted and ordered parent-before-child, which is also the order glTF and VRM
//! want joints written in.
//!
//! From there, [`skin::bind`] attaches a mesh to the rig and [`Landmarks`]
//! extracts the named anchors that hats, garments and other attachments fit
//! against.
//!
//! ```rust
//! use symbios_avatar::{AvatarRecord, Landmark, Limb, Rig, Zone};
//!
//! let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;
//!
//! // Semantic queries instead of bone names — the same call works on a quadruped.
//! // A hind extremity is a graph of its own — heel, the stub that closes it,
//! // ball and toe — so this finds every joint of both feet rather than one
//! // apiece.
//! let feet = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
//! assert_eq!(feet.len(), 8);
//!
//! // And the joints that DEFORM a foot are those plus the ankle they hang from,
//! // because a joint moves the bones leaving it. See `Rig::extremity_joints`.
//! let left = rig.extremity_joints(Limb::HindLeft);
//! assert_eq!(left.len(), 5);
//!
//! // Named anchors for fitting hats, garments, and other attachments.
//! let marks = rig.landmarks();
//! let hat = marks.get(Landmark::Crown).expect("every body has a crown");
//! let shoulders = marks.span(
//!     Landmark::LimbRoot(Limb::ForeLeft),
//!     Landmark::LimbRoot(Limb::ForeRight),
//! );
//! assert!(shoulders.is_some());
//! # Ok::<(), symbios_avatar::RigError>(())
//! ```

pub mod landmark;
pub mod patch;
pub mod skin;
pub mod socket;
pub mod surface;

use glam::{Vec2, Vec3};
use std::collections::VecDeque;
use thiserror::Error;

use crate::plan::{Limb, Zone};
use crate::skeleton::{Skeleton, SkeletonError};

pub use landmark::{Anchor, Landmark, Landmarks};
pub use patch::{Footprint, Patch};
pub use skin::{Influence, MAX_INFLUENCES, SkinConfig, SkinWeights};
pub use socket::Socket;
pub use surface::Surface;

/// Where the body's skeleton lies beneath a point on its surface.
///
/// Answers questions about the *skeleton* — how thick the body is here, how far
/// under the skin the bone runs. It deliberately answers nothing about the shape
/// of the surface. This type used to carry a `crease` that compared a normal
/// against the direction away from the bone, which holds only where the surface
/// was swept around that bone: an attached hand, nose or ear sits past the end
/// of the nearest bone, so the direction away from it is the limb's own axis
/// while the part's normals point every other way, and every one of them read as
/// a deep cavity (#63). Surface shape is [`crate::mesh::PolyMesh::crease`], which
/// measures the mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneHit {
    /// The nearest joint's index.
    pub joint: usize,
    /// Distance from the query point to that joint's bone.
    pub distance: f32,
    /// The body's radius where the bone was met.
    pub radius: f32,
    /// The nearest point on the bone itself.
    pub closest: Vec3,
}

/// What a joint is for.
///
/// Every joint a skeleton produces deforms the body, because every skeleton
/// node is a capsule the cage was swept over — the two are the same list. The
/// other roles are joints the body has *no* geometry for, and the fact they all
/// share is the one that matters: the body's own surface must not be bound to
/// them. A spring bone hanging in the hair that drags a patch of scalp with it
/// is not a subtle defect.
///
/// Nothing here makes a joint behave differently when it is posed. A helper
/// joint is a joint: it has a parent, it inherits its parent's transform, and
/// [`crate::anim::Pose`] evaluates it like any other. The role says who is
/// allowed to bind to it, not how it moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Role {
    /// Drives the body's own surface. Every joint from a skeleton is one.
    #[default]
    Deform,
    /// Carries something without deforming anything — an attachment point for a
    /// prop, a pivot a constraint aims at, the target end of a twist.
    Helper,
    /// Driven by simulation rather than by a pose: hair, hems, tails, ears.
    Spring,
    /// Moves a face. Kept apart from [`Role::Helper`] because a face rig is
    /// addressed as a set — retargeted, named against ARKit, driven by an
    /// expression track — and asking for "the facial joints" is a question
    /// something will need to answer.
    Facial,
    /// Bends a finger or a toe.
    ///
    /// A digit joint *does* deform something — the hand it belongs to — which is
    /// why it is not a [`Role::Helper`]. What it must never deform is the
    /// **body's own surface**: it sits inside the wrist's reach, so a bind that
    /// considered it would attach a patch of forearm to a fingertip. Keeping it
    /// out of [`Role::Deform`] is what stops that, and it is the same reason a
    /// hand rig is worth naming as a set: hand poses, grips and retargeting all
    /// want to ask for the digits and nothing else.
    Digit,
}

impl Role {
    /// Whether the body's own surface may be bound to a joint in this role.
    #[must_use]
    pub fn deforms(self) -> bool {
        matches!(self, Self::Deform)
    }
}

/// One posable joint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joint {
    /// The skeleton node this joint came from, or `None` if it came from no
    /// node — every joint that is not a [`Role::Deform`] one is attached to the
    /// rig after the fact and has no capsule behind it.
    pub node: Option<u32>,
    /// Index of this joint's parent, or `None` for the root.
    pub parent: Option<usize>,
    /// Rest position in body space.
    pub position: Vec3,
    /// The node's radius, which sets how far this joint's influence reaches.
    pub radius: f32,
    /// The node's elliptical cross-section, as multiples of that radius.
    ///
    /// **A radius alone does not say how wide a body part is, and anything
    /// measuring one has to know that.** Every trunk node in the humanoid plan
    /// carries a section, and #61 gave the skull one too — so a head's lateral
    /// reach is `radius * section.x` and reading it as `radius` puts a broad
    /// skull's surface outside its own node and a narrow one's well inside.
    /// Mirrored here rather than looked up in the skeleton because the rig is
    /// what everything downstream is handed: `refine_face` selects by the angle
    /// round the head, and on a sectioned head that angle is not the angle on
    /// the cage's own ring.
    ///
    /// `Vec2::ONE` for a joint attached after the fact, which has no node behind
    /// it and no section either.
    pub scale: Vec2,
    /// Which part of the body this joint drives.
    pub zone: Zone,
    /// What the joint is for, and so what may bind to it.
    pub role: Role,
    /// Whether the skeleton node behind this joint was a marker — a joint the
    /// cage never meshed (#134). The generic falloff must not bind the body to
    /// one: the jaw's pivot and tip get their skin from the mandible REGION
    /// instead (#152), and a marker that also falloff-bound would hold the
    /// same skin twice. Distinct from [`Role`], because a marker still
    /// deforms — its held skin must follow it when it is posed.
    pub marker: bool,
}

/// Errors raised while rooting a skeleton.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum RigError {
    /// The skeleton itself is malformed.
    #[error("skeleton is not usable: {0}")]
    Skeleton(#[from] SkeletonError),
    /// The requested root does not exist.
    #[error("root node {root} does not exist; the skeleton has {count} nodes")]
    RootOutOfRange {
        /// The requested root.
        root: u32,
        /// How many nodes exist.
        count: usize,
    },
    /// Some nodes cannot be reached from the root.
    ///
    /// A body has to be one connected piece; a detached island could never be
    /// posed, and would have been meshed as a separate surface anyway.
    #[error("nodes {unreachable:?} cannot be reached from root {root}")]
    Disconnected {
        /// The root the walk started from.
        root: u32,
        /// Nodes the walk never reached.
        unreachable: Vec<u32>,
    },
}

/// A skeleton rooted into a posable hierarchy.
#[derive(Clone, Debug, PartialEq)]
pub struct Rig {
    /// Joints ordered parent-before-child, root first.
    pub joints: Vec<Joint>,
}

impl Rig {
    /// Roots a skeleton at the node best suited to carry the body.
    ///
    /// The pelvis is preferred, then the rest of the torso, then any joint —
    /// the same choice a rigger makes, and the one that keeps a limb's hierarchy
    /// running outward from the body rather than inward from a fingertip.
    ///
    /// # Errors
    ///
    /// Returns [`RigError`] if the skeleton is malformed or disconnected.
    pub fn from_skeleton(skeleton: &Skeleton) -> Result<Self, RigError> {
        skeleton.validate()?;
        // Preference order, not body order: the pelvis carries the most of the
        // body, so rooting there keeps every limb's hierarchy running outward.
        const PREFERRED: [Zone; 3] = [Zone::Pelvis, Zone::Abdomen, Zone::Chest];
        let root = PREFERRED
            .into_iter()
            .find_map(|zone| skeleton.nodes.iter().position(|node| node.zone == zone))
            .unwrap_or(0);
        Self::rooted_at(skeleton, root as u32)
    }

    /// Roots a skeleton at a specific node.
    ///
    /// # Errors
    ///
    /// Returns [`RigError`] if the skeleton is malformed, the root does not
    /// exist, or some nodes cannot be reached from it.
    pub fn rooted_at(skeleton: &Skeleton, root: u32) -> Result<Self, RigError> {
        skeleton.validate()?;
        if root as usize >= skeleton.nodes.len() {
            return Err(RigError::RootOutOfRange {
                root,
                count: skeleton.nodes.len(),
            });
        }

        // Breadth-first, so joints come out parent-before-child: the order glTF
        // requires and the order a pose has to be evaluated in.
        let mut joint_of: Vec<Option<usize>> = vec![None; skeleton.nodes.len()];
        let mut joints = Vec::with_capacity(skeleton.nodes.len());
        let mut queue = VecDeque::from([(root, None)]);

        while let Some((node, parent)) = queue.pop_front() {
            if joint_of[node as usize].is_some() {
                continue;
            }
            let source = skeleton.nodes[node as usize];
            joint_of[node as usize] = Some(joints.len());
            joints.push(Joint {
                node: Some(node),
                parent,
                position: source.position,
                radius: source.radius,
                scale: source.scale,
                zone: source.zone,
                role: Role::Deform,
                // Carried through so the binding can treat marker joints
                // specially without needing the skeleton back: since #152 the
                // jaw's pivot and tip take their skin from
                // `face::skull::mandible_hold`, written into the weights
                // directly, and the generic falloff must not also bind them.
                // They stay [`Role::Deform`] because the deform path composes
                // only deforming joints — a role change here was measured to
                // leave their skin behind entirely (0.09 mm of lip travel on a
                // 20-degree open).
                marker: source.marker,
            });
            let here = joints.len() - 1;
            for neighbor in skeleton.neighbors(node) {
                if joint_of[neighbor as usize].is_none() {
                    queue.push_back((neighbor, Some(here)));
                }
            }
        }

        let unreachable: Vec<u32> = joint_of
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.is_none())
            .map(|(node, _)| node as u32)
            .collect();
        if !unreachable.is_empty() {
            return Err(RigError::Disconnected { root, unreachable });
        }

        Ok(Self { joints })
    }

    /// Hangs a joint off an existing one, and returns its index.
    ///
    /// This is the only way a rig gets a joint the body has no capsule for — a
    /// spring chain down a lock of hair, a bone under a cheek, a socket a prop
    /// hangs from. It appends, so the parent-before-child order the rest of the
    /// crate relies on survives: a new joint's parent is always already in the
    /// list.
    ///
    /// The joint takes its parent's zone, because a bone under a cheek is part
    /// of the head and a chain down a tail is part of the tail, and a radius of
    /// zero, because it stands for no geometry. Both are public fields if a
    /// caller wants otherwise.
    ///
    /// Returns `None` if `parent` is not a joint of this rig. Attaching a
    /// [`Role::Deform`] joint is allowed and does what it says — the body binds
    /// to it — but the point of this is the roles that do not.
    pub fn attach(&mut self, parent: usize, position: Vec3, role: Role) -> Option<usize> {
        let zone = self.joints.get(parent)?.zone;
        self.joints.push(Joint {
            node: None,
            parent: Some(parent),
            position,
            radius: 0.0,
            scale: Vec2::ONE,
            zone,
            role,
            marker: false,
        });
        Some(self.joints.len() - 1)
    }

    /// The joints the body's own surface may be bound to, in hierarchy order.
    pub fn deforming(&self) -> impl Iterator<Item = usize> + '_ {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.role.deforms())
            .map(|(index, _)| index)
    }

    /// The joints the body's surface is actually made OF, in hierarchy order.
    ///
    /// [`Self::deforming`] less the markers. **The two are different questions
    /// and [`Role`] answers only one of them** (#136): a marker is a joint the
    /// surface BINDS to and is not made of — the mandible's pivot and tip hold
    /// skin so that skin follows a jaw when it opens, but the cage never meshed
    /// them and there is no surface under them to measure. Binding wants
    /// `deforming`; anything asking what lies beneath a point wants this.
    ///
    /// `Role` cannot carry that distinction as it stands, because `Deform` is
    /// both the only role that binds and the role a surface query trusts. #136
    /// records the open question of whether it should have been a role rather
    /// than a bit; the bit is what the skeleton already carries, so the bit is
    /// what this reads.
    pub fn surfaced(&self) -> impl Iterator<Item = usize> + '_ {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.role.deforms() && !joint.marker)
            .map(|(index, _)| index)
    }

    /// How many joints the rig has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.joints.len()
    }

    /// Whether the rig has no joints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// The bone leading into `joint`, as `(start, end)` positions.
    ///
    /// The root has no bone, so its segment is the single point it occupies.
    #[must_use]
    pub fn bone(&self, joint: usize) -> (Vec3, Vec3) {
        let here = self.joints[joint];
        match here.parent {
            Some(parent) => (self.joints[parent].position, here.position),
            None => (here.position, here.position),
        }
    }

    /// The radii at each end of the bone leading into `joint`.
    #[must_use]
    pub fn bone_radii(&self, joint: usize) -> (f32, f32) {
        let here = self.joints[joint];
        match here.parent {
            Some(parent) => (self.joints[parent].radius, here.radius),
            None => (here.radius, here.radius),
        }
    }

    /// The bone nearest `point`, and where on it the nearest spot lies.
    ///
    /// Only bones the body is actually made of. This answers "what part of the
    /// body lies under this point", which everything that measures a surface
    /// asks — the scalp profile, the garment shells, the skin shading — and a
    /// joint with no geometry behind it has no answer to give. A face rig would
    /// otherwise put a dozen bones inside the skull and win every one of those
    /// queries.
    ///
    /// **That paragraph was a statement of intent for six weeks and is a
    /// filter now** (#136). The condition it describes arrived the moment
    /// #134's jaw markers did: a marker has to be [`Role::Deform`] or the skin
    /// could not be bound to it, so this loop saw it, and the mandible runs
    /// diagonally through the middle of the skull where it is nearer to most of
    /// the face than any bone the head is actually made of. Measured on the
    /// shipped body, the two markers won **6,425 of 8,965 body vertices** —
    /// essentially the whole head, since `refine_face` puts most of the mesh
    /// there.
    ///
    /// Nothing that reads only the hit's ZONE could see it, which is why it
    /// went unnoticed: a marker's zone is `Head`, and so is the head's.
    /// [`crate::texture::paint_skin`] and [`Surface::measure`] read more than
    /// the zone — the first takes `radius` as a thinness and the second credits
    /// `distance` to `joint` — so those two were quietly being answered by a
    /// bone with no surface of its own.
    #[must_use]
    pub fn nearest_bone(&self, point: Vec3) -> BoneHit {
        let mut best = BoneHit {
            joint: self.surfaced().next().unwrap_or(0),
            distance: f32::INFINITY,
            radius: 1.0,
            closest: point,
        };
        for joint in self.surfaced() {
            let (start, end) = self.bone(joint);
            let (start_radius, end_radius) = self.bone_radii(joint);
            let axis = end - start;
            let along = if axis.length_squared() <= f32::EPSILON {
                0.0
            } else {
                ((point - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
            };
            let closest = start + axis * along;
            let distance = point.distance(closest);
            if distance < best.distance {
                best = BoneHit {
                    joint,
                    distance,
                    radius: start_radius + (end_radius - start_radius) * along,
                    closest,
                };
            }
        }
        best
    }

    /// Joints belonging to `zone`, in hierarchy order.
    #[must_use]
    pub fn in_zone(&self, zone: Zone) -> Vec<usize> {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| joint.zone == zone)
            .map(|(index, _)| index)
            .collect()
    }

    /// The joints that deform a limb's extremity, in hierarchy order.
    ///
    /// **Not the same as the extremity zone**, and the difference is the whole
    /// reason this exists. A joint deforms the bones *leaving* it, so a leaf
    /// joint holds no body surface at all: on a foot meshed as ankle, ball and
    /// toe the toe holds nothing, and a heel hung off the ankle as a leaf would
    /// hold nothing either — its surface is deformed by the **ankle**, the joint
    /// whose bone runs out to it. The joint the extremity hangs from is
    /// therefore part of the foot as far as anything reading the binding is
    /// concerned, and it is included here.
    ///
    /// The reference mannequins are bound the same way for the same reason:
    /// their rig has no heel bone at all, so their entire heel is held by the
    /// joint they call `foot_l`. Selecting a foot from [`Zone::Extremity`] alone
    /// drops the heel and then reports that the foot has not got one.
    ///
    /// Returns an empty vector for a limb this body does not have.
    #[must_use]
    pub fn extremity_joints(&self, limb: Limb) -> Vec<usize> {
        let mut joints = self.in_zone(Zone::Extremity(limb));
        if let Some(&first) = joints.first()
            && let Some(parent) = self.joints[first].parent
            && !joints.contains(&parent)
        {
            joints.insert(0, parent);
        }
        joints
    }

    /// The three joints an [`crate::anim::ik::two_bone`] solve needs for `limb`.
    ///
    /// Returns `[root, mid, tip]` — hip, knee, ankle, or shoulder, elbow, wrist —
    /// or `None` if the body does not articulate that limb far enough to solve.
    ///
    /// The tip is the last joint *above* the extremity where a limb has enough
    /// of them, so a biped solves to its ankle and lets the foot hang off it. A
    /// quadruped's leg has one segment fewer, so its extremity is the tip. Both
    /// are handled by taking whatever the limb has and topping up from the end,
    /// rather than by assuming a particular anatomy.
    #[must_use]
    pub fn limb_chain(&self, limb: Limb) -> Option<[usize; 3]> {
        let mut chain = self.in_zone(Zone::UpperLimb(limb));
        chain.extend(self.in_zone(Zone::LowerLimb(limb)));
        if chain.len() < 3 {
            chain.extend(self.in_zone(Zone::Extremity(limb)));
        }
        if chain.len() < 3 {
            return None;
        }
        let tail = &chain[chain.len() - 3..];
        Some([tail[0], tail[1], tail[2]])
    }

    /// Limbs whose extremity rests near the ground.
    ///
    /// Which limbs carry a body is a property of the body, not of its plan: a
    /// biped stands on two of its four, a quadruped on all four. Reading it off
    /// the rest pose gets both right without either having to declare it, and
    /// gets a future six-legged plan right for free.
    #[must_use]
    pub fn ground_contacts(&self) -> Vec<Limb> {
        let extremities: Vec<(Limb, f32)> = Limb::ALL
            .into_iter()
            .filter_map(|limb| {
                let joint = *self.in_zone(Zone::Extremity(limb)).first()?;
                Some((limb, self.joints[joint].position.y))
            })
            .collect();

        let Some(lowest) = extremities
            .iter()
            .map(|(_, y)| *y)
            .fold(None, |acc: Option<f32>, y| {
                Some(acc.map_or(y, |a| a.min(y)))
            })
        else {
            return Vec::new();
        };
        // A quarter of the body's height of slack: enough that four legs of
        // slightly different length all count, far too little for a T-posed
        // biped's hands to.
        let slack = self.extent() * 0.25;

        extremities
            .into_iter()
            .filter(|(_, y)| *y - lowest <= slack)
            .map(|(limb, _)| limb)
            .collect()
    }

    /// A point the middle joint of `limb` should bend toward.
    ///
    /// Every two-bone solve needs one, because the bend plane is undetermined
    /// whenever the limb is straight. Callers were inventing it, and all of them
    /// invented the same forward direction — right for a knee, backwards for
    /// everything else that can be solved.
    ///
    /// **The rest pose answers it wherever the rest pose has an opinion.** A
    /// body plan that builds a limb with a bend in it has already said which way
    /// that limb folds, and reading it is strictly better than any rule about
    /// limb names: it is measured from the body in hand rather than assumed
    /// about bodies in general, and a plan nobody has written yet gets it right
    /// for free. Measured on the plans that exist, a quadruped's fore and hind
    /// limbs *both* fold backward — the carpus and the hock — which is exactly
    /// the case a fore-versus-hind rule gets wrong.
    ///
    /// **Only a straight limb needs the fallback, and then it is anatomy:** fore
    /// folds back, hind folds forward. Every limb of the humanoid plan is dead
    /// straight at rest — the arms by design, an A-pose being what it is, and
    /// the legs because the plan puts hip, knee and ankle on one line — so this
    /// is the branch a biped always takes.
    ///
    /// Returns `None` for a limb this body does not articulate far enough to
    /// solve.
    #[must_use]
    pub fn bend_pole(&self, limb: Limb) -> Option<Vec3> {
        let chain = self.limb_chain(limb)?;
        let [root, mid, tip] = chain.map(|joint| self.joints[joint].position);

        // How far the middle joint stands off the line from root to tip: the
        // bend the plan built in, if it built one.
        let line = (tip - root).normalize_or(landmark::UP);
        let offset = mid - root;
        let bend = offset - line * offset.dot(line);

        // Half a percent of the body's height. Below that it is arithmetic
        // noise rather than an articulation — the humanoid's limbs measure
        // exactly zero, and the quadruped's smallest genuine bend is six times
        // this.
        let toward = if bend.length() > self.extent() * 0.005 {
            bend.normalize()
        } else if limb.is_fore() {
            -landmark::FORWARD
        } else {
            landmark::FORWARD
        };
        // Anchored at the chain's root and thrown a body's length out, so the
        // direction survives the limb swinging about underneath it.
        Some(root + toward * self.extent())
    }

    /// How far a limb can reach: the sum of the bones an IK solve controls.
    ///
    /// The straight-line distance to the extremity is the wrong measure twice
    /// over — it stops short of the bones' true extent when the limb rests bent,
    /// and it includes the foot, which hangs off the chain rather than being part
    /// of it. Summing the chain's bones is what a solver can actually deliver.
    #[must_use]
    pub fn limb_reach(&self, limb: Limb) -> Option<f32> {
        let chain = self.limb_chain(limb)?;
        let joint = |index: usize| self.joints[chain[index]].position;
        Some(joint(0).distance(joint(1)) + joint(1).distance(joint(2)))
    }

    /// Vertical extent of the rig's rest pose, in metres.
    #[must_use]
    pub fn extent(&self) -> f32 {
        let (low, high) = self.joints.iter().fold((f32::MAX, f32::MIN), |acc, joint| {
            (acc.0.min(joint.position.y), acc.1.max(joint.position.y))
        });
        (high - low).max(f32::EPSILON)
    }

    /// Named anchors for fitting hats, garments, and other attachments.
    #[must_use]
    pub fn landmarks(&self) -> Landmarks {
        Landmarks::from_rig(self)
    }

    /// Joints whose zone satisfies `predicate`, in hierarchy order.
    ///
    /// The general form of a semantic query — "every ground contact", "every
    /// grasper" — which is how motion gets described without naming bones.
    #[must_use]
    pub fn query(&self, predicate: impl Fn(Zone) -> bool) -> Vec<usize> {
        self.joints
            .iter()
            .enumerate()
            .filter(|(_, joint)| predicate(joint.zone))
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Archetype, BodyPlan, HumanoidParams, Limb, QuadrupedParams};

    #[test]
    fn joints_come_out_parent_before_child() {
        let rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        for (index, joint) in rig.joints.iter().enumerate() {
            if let Some(parent) = joint.parent {
                assert!(parent < index, "joint {index} precedes its parent {parent}");
            }
        }
        assert_eq!(rig.joints[0].parent, None, "exactly one root, first");
        assert_eq!(rig.joints.iter().filter(|j| j.parent.is_none()).count(), 1);
    }

    #[test]
    fn no_marker_bone_answers_a_surface_query() {
        // The contract `nearest_bone`'s own docstring states, held as a test
        // for the first time (#136). It was true by luck until #134 added the
        // jaw's markers, and then false for six weeks over 6,425 of 8,965 body
        // vertices — with nothing to notice, because the two callers that read
        // more than the hit's zone assert nothing. Every joint the face rig
        // adds inside the skull (#118: brows, lids, mouth corners) is another
        // chance to break it the same way, which is why the guard goes in now
        // rather than after.
        //
        // Sampled at the CAGE's nodes rather than at a built mesh's vertices:
        // this module knows nothing about meshing, and the failure is a bone
        // winning points near the skull, which the cage's own head nodes sit
        // among.
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        assert!(
            rig.joints.iter().any(|joint| joint.marker),
            "this body has no markers, so the test would pass vacuously"
        );
        for node in &skeleton.nodes {
            for offset in [
                Vec3::ZERO,
                Vec3::new(node.radius, 0.0, 0.0),
                Vec3::new(0.0, 0.0, node.radius),
            ] {
                let hit = rig.nearest_bone(node.position + offset);
                assert!(
                    !rig.joints[hit.joint].marker,
                    "a marker bone answered for a point at {:?}: it has no surface to answer with",
                    node.position + offset
                );
            }
        }
    }

    #[test]
    fn a_body_roots_at_its_pelvis() {
        for archetype in [
            Archetype::Humanoid(HumanoidParams::default()),
            Archetype::Quadruped(QuadrupedParams::default()),
        ] {
            let rig = Rig::from_skeleton(&archetype.skeleton(&crate::Composites::default()))
                .expect("rigs");
            assert_eq!(
                rig.joints[0].zone,
                Zone::Pelvis,
                "{} should root at the pelvis",
                archetype.name()
            );
        }
    }

    #[test]
    fn every_node_becomes_exactly_one_joint() {
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        let rig = Rig::rooted_at(&skeleton, 0).expect("rigs");
        assert_eq!(rig.len(), skeleton.nodes.len());

        let mut nodes: Vec<u32> = rig
            .joints
            .iter()
            .map(|j| j.node.expect("a joint from a skeleton came from a node"))
            .collect();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.len(), skeleton.nodes.len());
    }

    /// A rig with a three-link spring chain hanging off the head, of the kind
    /// hair or an ear would want, and the head joint it hangs from.
    fn with_a_spring_chain() -> (Rig, usize, Vec<usize>) {
        let mut rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let mut chain = Vec::new();
        let mut parent = head;
        for link in 1..=3 {
            // Hung out to the side and downward, well inside the body's reach,
            // so that if the skin did bind to these it would certainly show.
            let at = rig.joints[head].position + Vec3::new(0.04 * link as f32, -0.02, 0.0);
            parent = rig
                .attach(parent, at, Role::Spring)
                .expect("the head exists");
            chain.push(parent);
        }
        (rig, head, chain)
    }

    #[test]
    fn an_attached_joint_hangs_off_its_parent_in_order() {
        let (mut rig, head, chain) = with_a_spring_chain();
        assert_eq!(rig.joints[chain[0]].parent, Some(head));
        for (index, &joint) in chain.iter().enumerate() {
            assert!(
                rig.joints[joint].parent.expect("attached") < joint,
                "attaching broke parent-before-child at {joint}"
            );
            assert_eq!(rig.joints[joint].role, Role::Spring);
            assert_eq!(rig.joints[joint].node, None, "link {index} claims a node");
            // The zone comes down the chain from the head it hangs off.
            assert_eq!(rig.joints[joint].zone, Zone::Head);
        }
        assert_eq!(rig.attach(rig.len(), Vec3::ZERO, Role::Helper), None);
    }

    #[test]
    fn the_body_does_not_bind_to_a_joint_it_has_no_geometry_for() {
        // The whole issue. A rig that carries hair, a face or a prop socket has
        // joints with no capsule behind them, and the body's own surface has to
        // ignore every one — silently binding to them attaches a patch of skin
        // to something that was never meant to move it.
        use crate::cage::{CageConfig, build_cage};
        use crate::rig::skin;
        use crate::subdiv::catmull_clark;

        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let mesh = catmull_clark(
            &build_cage(&skeleton, &CageConfig::default()).expect("meshes"),
            1,
        );
        let plain = Rig::from_skeleton(&skeleton).expect("rigs");
        let (sprung, _, chain) = with_a_spring_chain();

        let before = skin::bind(&mesh, &plain, &SkinConfig::default());
        let after = skin::bind(&mesh, &sprung, &SkinConfig::default());
        assert_eq!(
            before, after,
            "adding joints the body is not made of changed how it is skinned"
        );
        for vertex in &after.vertices {
            for influence in vertex {
                assert!(
                    !chain.contains(&(influence.joint as usize)),
                    "a body vertex was bound to a spring bone"
                );
            }
        }
        assert!(
            after.is_normalized(1e-4),
            "the weights stopped summing to one"
        );
    }

    #[test]
    fn the_body_does_not_bind_to_a_finger() {
        // A digit joint is the one kind that deforms real geometry and still
        // must be kept away from the body's own surface: it sits *inside* the
        // wrist's reach, so a bind that considered it would attach a patch of
        // forearm to a fingertip and the sleeve would follow a curling hand
        // (#113). This is the same claim as the spring-bone test above, and it
        // matters more, because unlike a spring bone a finger has a mesh and so
        // looks like something worth binding to.
        use crate::extremity::Extremities;
        use crate::rig::skin;
        use crate::subdiv::catmull_clark;
        use crate::{Archetype, AvatarRecord, CageConfig, build_cage};

        let record = AvatarRecord::new("Fingered", Archetype::default());
        let skeleton = record.skeleton();
        let mesh = catmull_clark(
            &build_cage(&skeleton, &CageConfig::default()).expect("meshes"),
            crate::BODY_SUBDIVISIONS,
        );
        let mut rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let plain = rig.clone();
        let surface = Surface::measure(&mesh, &rig);
        let _ = Extremities::build(&mut rig, &surface, 0.0);

        let digits: Vec<usize> = (0..rig.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit)
            .collect();
        assert!(!digits.is_empty(), "the body grew no fingers to check");

        let before = skin::bind(&mesh, &plain, &SkinConfig::default());
        let after = skin::bind(&mesh, &rig, &SkinConfig::default());
        assert_eq!(
            before, after,
            "hanging fingers on the rig changed how the body is skinned"
        );
        for vertex in &after.vertices {
            for influence in vertex {
                assert!(
                    !digits.contains(&(influence.joint as usize)),
                    "a body vertex was bound to a finger"
                );
            }
        }
        // And the same for the query everything that measures a surface asks.
        for &joint in &digits {
            assert_ne!(
                rig.nearest_bone(rig.joints[joint].position).joint,
                joint,
                "a finger answered for the point it sits on"
            );
        }
    }

    #[test]
    fn a_joint_with_no_geometry_never_wins_a_surface_query() {
        // nearest_bone answers "what part of the body is under this point", and
        // a face rig would otherwise put a dozen bones inside the skull and win
        // that question at every vertex of the head.
        let (rig, head, chain) = with_a_spring_chain();
        for &joint in &chain {
            let at = rig.joints[joint].position;
            assert_ne!(
                rig.nearest_bone(at).joint,
                joint,
                "a spring bone answered for the point it sits on"
            );
        }
        // And the answer is the one it was before the chain was hung there.
        let plain =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        let probe = rig.joints[head].position + Vec3::new(0.04, -0.02, 0.0);
        assert_eq!(
            rig.nearest_bone(probe).joint,
            plain.nearest_bone(probe).joint
        );
    }

    #[test]
    fn an_attached_joint_is_still_posed_like_any_other() {
        // The role says who may bind to a joint, not how it moves. A spring
        // chain that did not follow the head would be useless for the thing it
        // exists for.
        use crate::anim::Pose;
        use glam::Quat;

        let (rig, head, chain) = with_a_spring_chain();
        let rest = Pose::rest(&rig).forward(&rig);
        for &joint in &chain {
            assert!(rest.positions[joint].distance(rig.joints[joint].position) < 1e-5);
        }

        let mut pose = Pose::rest(&rig);
        pose.rotations[head] = Quat::from_rotation_z(0.6);
        let turned = pose.forward(&rig);
        for &joint in &chain {
            assert!(
                turned.positions[joint].distance(rest.positions[joint]) > 0.005,
                "a spring bone did not follow the head it hangs from"
            );
        }
    }

    #[test]
    fn semantic_queries_find_limbs_without_naming_bones() {
        let rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");

        // Asked as LIMBS, not as nodes. A hind extremity is a chain now — ball
        // and toe — so counting nodes counts the foot's own joints and answers
        // four (#111). What this test is about is that the query finds the right
        // limbs without naming a bone, and that is unchanged.
        let contacts = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
        let standing: std::collections::BTreeSet<_> = contacts
            .iter()
            .filter_map(|&joint| match rig.joints[joint].zone {
                Zone::Extremity(limb) => Some(limb),
                _ => None,
            })
            .collect();
        assert_eq!(standing.len(), 2, "a biped stands on two feet");

        let graspers = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if limb.is_fore()));
        assert_eq!(graspers.len(), 2, "and has two hands");

        // Skull and crown, which a head needs to have a dome, plus the jaw's
        // pivot and tip (#134), the two brow markers (#215) and the two mouth
        // corners (#216) — rig-only markers, so they are joints here and
        // nothing in the cage. Anything that wants THE head joint takes the
        // first, which is the skull.
        assert_eq!(rig.in_zone(Zone::Head).len(), 8);
        assert_eq!(rig.in_zone(Zone::UpperLimb(Limb::ForeLeft)).len(), 2);
    }

    #[test]
    fn a_quadruped_stands_on_four_feet() {
        let rig =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        let contacts = rig.query(|zone| matches!(zone, Zone::Extremity(_)));
        assert_eq!(contacts.len(), 4);
        assert_eq!(rig.in_zone(Zone::Tail).len(), 2);
    }

    #[test]
    fn bones_run_from_parent_to_child() {
        let rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs");
        let (start, end) = rig.bone(0);
        assert_eq!(start, end, "the root has no bone, only a position");

        let child = rig.joints.iter().position(|j| j.parent == Some(0)).unwrap();
        let (start, end) = rig.bone(child);
        assert_eq!(start, rig.joints[0].position);
        assert_eq!(end, rig.joints[child].position);
    }

    #[test]
    fn a_bad_root_is_reported() {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        assert!(matches!(
            Rig::rooted_at(&skeleton, 999),
            Err(RigError::RootOutOfRange { root: 999, .. })
        ));
    }

    #[test]
    fn a_detached_island_is_reported() {
        use crate::skeleton::Node;

        let mut skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let a = skeleton.add_node(Node::new(Vec3::new(9.0, 9.0, 9.0), 0.1));
        let b = skeleton.add_node(Node::new(Vec3::new(9.0, 9.5, 9.0), 0.1));
        skeleton.connect(a, b);

        match Rig::rooted_at(&skeleton, 0) {
            Err(RigError::Disconnected { unreachable, .. }) => {
                assert_eq!(unreachable, vec![a, b]);
            }
            other => panic!("expected a disconnection report, got {other:?}"),
        }
    }
}
