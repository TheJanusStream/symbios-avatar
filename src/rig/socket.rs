//! Named attachment points a prop can hang from.
//!
//! A host attaching a sword, a hat or a backpack needs two answers a
//! [`Landmark`] alone does not give: **which joint carries the prop** — so it
//! follows the animation — and where on this particular body the prop first
//! sits. The rig cannot answer by bone name because it has none: joints are
//! indices, identified by [`Zone`] and [`Limb`]. A [`Socket`] is the durable
//! name in between — stable across every body a re-roll can draw, and across
//! body plans, so "left hand" is a fore-left paw on a quadruped without
//! anybody special-casing it.
//!
//! The vocabulary is a wardrobe's, deliberately curated rather than one entry
//! per joint: a picker that lists every knee and toe is a picker nobody can
//! use, and a socket that exists on one seed's body and not another's is a
//! prop that falls off on a re-roll. Every socket here resolves on any body
//! whose plan has the parts — the only conditional one is [`Socket::Tail`],
//! and asking a biped for it returns `None` rather than a guess, exactly as
//! [`Landmarks`] answers.
//!
//! ## Attaching at runtime, and the trap
//!
//! [`Socket::joint`] returns an index into [`Rig::joints`], which is also the
//! index into whatever per-joint structure a renderer spawned — in the Bevy
//! crate, the entity list its `AvatarJoints` component carries, where
//! attachment is nothing more than parenting the prop to that entity.
//!
//! **Do not reach for [`Rig::attach`] on a built body.** It appends a joint,
//! and every skinned mesh was bound against the joint list as it stood at
//! [`crate::Avatar::build`]: the baked inverse bindposes are ordered by joint
//! index, so growing the rig afterwards hands the renderer two lists that no
//! longer describe each other. `Rig::attach` (with [`Role::Helper`], whose
//! docs call it an attachment point) is for *pre-build* rig authoring, where
//! the bind that follows will see the joint it added.
//!
//! [`Role::Helper`]: super::Role

use super::landmark::{Anchor, FORWARD, Landmark, Landmarks};
use super::{Rig, Surface};
use crate::plan::{Limb, Zone};

/// A named place a prop attaches to, durable across bodies and plans.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Socket {
    /// Top of the skull: hats, halos, antlers.
    Crown,
    /// The face at eye height: masks, glasses.
    Face,
    /// Where the neck meets the chest: collars, amulets.
    Neck,
    /// The front of the chest: a pendant, a badge.
    Chest,
    /// The back of the chest: backpacks, capes, wings.
    Back,
    /// The narrowest part of the torso: belts.
    Waist,
    /// The pelvis at its widest: skirts, tool belts that ride low.
    Hips,
    /// Where a tail leaves the body. The one socket a body may not have.
    Tail,
    /// The left arm's root.
    LeftShoulder,
    /// The right arm's root.
    RightShoulder,
    /// The left hand — a fore-left paw, on a body that walks on it.
    LeftHand,
    /// The right hand.
    RightHand,
    /// The left leg's root, where a holster rides.
    LeftHip,
    /// The right leg's root.
    RightHip,
    /// The left foot.
    LeftFoot,
    /// The right foot.
    RightFoot,
}

impl Socket {
    /// Every socket, in the order a picker should list them.
    pub const ALL: [Self; 16] = [
        Self::Crown,
        Self::Face,
        Self::Neck,
        Self::Chest,
        Self::Back,
        Self::Waist,
        Self::Hips,
        Self::Tail,
        Self::LeftShoulder,
        Self::RightShoulder,
        Self::LeftHand,
        Self::RightHand,
        Self::LeftHip,
        Self::RightHip,
        Self::LeftFoot,
        Self::RightFoot,
    ];

    /// The socket's stable name, for wire formats and pickers.
    ///
    /// These strings are a contract in the way stream names are
    /// ([`crate::plan::Rolls`]): a host that stored one expects
    /// [`Self::from_name`] to return the same socket in every later build.
    /// Renaming one orphans every prop attached through it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Crown => "crown",
            Self::Face => "face",
            Self::Neck => "neck",
            Self::Chest => "chest",
            Self::Back => "back",
            Self::Waist => "waist",
            Self::Hips => "hips",
            Self::Tail => "tail",
            Self::LeftShoulder => "left-shoulder",
            Self::RightShoulder => "right-shoulder",
            Self::LeftHand => "left-hand",
            Self::RightHand => "right-hand",
            Self::LeftHip => "left-hip",
            Self::RightHip => "right-hip",
            Self::LeftFoot => "left-foot",
            Self::RightFoot => "right-foot",
        }
    }

    /// The socket a stored name means, or `None` for one this build does not
    /// know — a host should keep the record and skip the prop, the same
    /// forward-compatibility answer the record's open unions give.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|socket| socket.name() == name)
    }

    /// The limb a limb socket lives on.
    ///
    /// Left is `+X` on a body facing [`FORWARD`] — see [`Limb`], and the
    /// landmark tests that pin the sides by absolute sign.
    fn limb(self) -> Option<Limb> {
        match self {
            Self::LeftShoulder | Self::LeftHand => Some(Limb::ForeLeft),
            Self::RightShoulder | Self::RightHand => Some(Limb::ForeRight),
            Self::LeftHip | Self::LeftFoot => Some(Limb::HindLeft),
            Self::RightHip | Self::RightFoot => Some(Limb::HindRight),
            _ => None,
        }
    }

    /// The joint that carries this socket, as an index into [`Rig::joints`].
    ///
    /// This is the joint a prop is parented to, so it is chosen for what
    /// *moves* the prop, not for where the prop sits: a hand socket rides the
    /// joint that deforms the extremity — the wrist, per
    /// [`Rig::extremity_joints`] — while its [`Self::anchor`] sits out at the
    /// limb's tip. Returns `None` for a part this body does not have.
    #[must_use]
    pub fn joint(self, rig: &Rig) -> Option<usize> {
        match self {
            Self::Crown | Self::Face => rig.in_zone(Zone::Head).first().copied(),
            Self::Neck => rig.in_zone(Zone::Neck).first().copied(),
            Self::Chest | Self::Back => rig.in_zone(Zone::Chest).first().copied(),
            Self::Waist => rig.in_zone(Zone::Abdomen).first().copied(),
            Self::Hips => rig.in_zone(Zone::Pelvis).first().copied(),
            Self::Tail => rig.in_zone(Zone::Tail).first().copied(),
            Self::LeftShoulder | Self::RightShoulder | Self::LeftHip | Self::RightHip => {
                let limb = self.limb()?;
                rig.in_zone(Zone::UpperLimb(limb)).first().copied()
            }
            Self::LeftHand | Self::RightHand | Self::LeftFoot | Self::RightFoot => {
                let limb = self.limb()?;
                // The joint that deforms the extremity, falling back to the
                // limb's last articulated joint on a plan that ends without
                // one — a future serpent's arm should still offer a hand.
                rig.extremity_joints(limb)
                    .first()
                    .copied()
                    .or_else(|| rig.in_zone(Zone::LowerLimb(limb)).last().copied())
            }
        }
    }

    /// Where a prop first sits, in rest-pose body space.
    ///
    /// Mostly read off [`Landmarks`]; the two chest sockets are the exception
    /// because a landmark ring faces along the spine and a pendant or a
    /// backpack faces out of the body. Rest pose, like every figure on the
    /// rig: a posed position is the carrying joint's transform applied to
    /// this, which is exactly what parenting a prop to the joint does.
    #[must_use]
    pub fn anchor(self, rig: &Rig) -> Option<Anchor> {
        let marks = Landmarks::from_rig(rig);
        match self {
            Self::Crown => marks.get(Landmark::Crown),
            Self::Face => marks.get(Landmark::EyeLine),
            Self::Neck => marks.get(Landmark::NeckBase),
            Self::Chest | Self::Back => {
                let joint = rig.joints[self.joint(rig)?];
                let direction = if self == Self::Chest {
                    FORWARD
                } else {
                    -FORWARD
                };
                Some(Anchor {
                    position: joint.position,
                    direction,
                    radius: joint.radius,
                })
            }
            Self::Waist => marks.get(Landmark::WaistRing),
            Self::Hips => marks.get(Landmark::HipRing),
            Self::Tail => marks.get(Landmark::TailBase),
            Self::LeftShoulder | Self::RightShoulder | Self::LeftHip | Self::RightHip => {
                marks.get(Landmark::LimbRoot(self.limb()?))
            }
            Self::LeftHand | Self::RightHand | Self::LeftFoot | Self::RightFoot => {
                marks.get(Landmark::LimbTip(self.limb()?))
            }
        }
    }

    /// The anchor, seated outside this body's measured surface.
    ///
    /// [`Self::anchor`] works from the rig's nodes, and the rendered surface
    /// does not sit at a node radius — it is measured, which is the whole
    /// reason [`Surface`] exists. This takes the anchor one radius out along
    /// its own direction and then asks [`Surface::clear`] to push it the rest
    /// of the way if the body is thicker there than its node claims, so the
    /// default placement of a prop starts visible rather than embedded in a
    /// chest. `margin` is in metres, and is the gap left between prop origin
    /// and skin.
    #[must_use]
    pub fn seat(self, rig: &Rig, surface: &Surface, margin: f32) -> Option<Anchor> {
        let anchor = self.anchor(rig)?;
        let position = surface.clear(rig, anchor.offset(1.0), margin);
        Some(Anchor { position, ..anchor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::BodyPlan;
    use crate::{
        Archetype, AvatarRecord, CageConfig, Composites, HumanoidParams, QuadrupedParams,
        build_cage, catmull_clark,
    };

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&Composites::default()))
            .expect("rigs")
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
            .expect("rigs")
    }

    #[test]
    fn every_socket_resolves_on_a_biped_except_the_tail() {
        let rig = biped();
        for socket in Socket::ALL {
            let joint = socket.joint(&rig);
            let anchor = socket.anchor(&rig);
            if socket == Socket::Tail {
                assert_eq!(joint, None, "a biped has no tail to carry a prop");
                assert_eq!(anchor, None);
                continue;
            }
            let joint = joint.unwrap_or_else(|| panic!("{socket:?} did not resolve"));
            assert!(joint < rig.len());
            assert!(anchor.is_some(), "{socket:?} has no anchor");
        }
    }

    #[test]
    fn every_socket_resolves_on_a_quadruped() {
        let rig = quadruped();
        for socket in Socket::ALL {
            assert!(
                socket.joint(&rig).is_some(),
                "{socket:?} did not resolve on a quadruped"
            );
            assert!(socket.anchor(&rig).is_some(), "{socket:?} has no anchor");
        }
    }

    #[test]
    fn left_and_right_sockets_land_on_their_own_sides() {
        // Absolute signs, not left-versus-right: comparing the pair against
        // each other would pass on a body that was simply the wrong way round.
        // Left is `+X` on a body facing `+Z` (#142).
        let rig = biped();
        for (left, right) in [
            (Socket::LeftShoulder, Socket::RightShoulder),
            (Socket::LeftHand, Socket::RightHand),
            (Socket::LeftHip, Socket::RightHip),
            (Socket::LeftFoot, Socket::RightFoot),
        ] {
            let x = |socket: Socket| {
                let joint = socket.joint(&rig).expect("resolves");
                rig.joints[joint].position.x
            };
            assert!(x(left) > 0.0, "{left:?} sits at x {}", x(left));
            assert!(x(right) < 0.0, "{right:?} sits at x {}", x(right));
        }
    }

    #[test]
    fn names_round_trip_and_unknown_names_do_not() {
        for socket in Socket::ALL {
            assert_eq!(
                Socket::from_name(socket.name()),
                Some(socket),
                "{socket:?} did not survive its own name"
            );
        }
        assert_eq!(Socket::from_name("third-elbow"), None);
    }

    #[test]
    fn the_chest_faces_out_of_the_body_and_the_back_the_other_way() {
        let rig = biped();
        let chest = Socket::Chest.anchor(&rig).expect("a chest");
        let back = Socket::Back.anchor(&rig).expect("a back");
        assert!(chest.direction.dot(FORWARD) > 0.9);
        assert!(back.direction.dot(FORWARD) < -0.9);
        // Same joint, opposite faces: a pendant and a backpack ride the same
        // bone.
        assert_eq!(Socket::Chest.joint(&rig), Socket::Back.joint(&rig));
    }

    #[test]
    fn a_hand_prop_rides_the_wrist_and_sits_at_the_hand() {
        // The split the docs promise: the carrying joint is what deforms the
        // extremity, the anchor is out at the limb's tip.
        let rig = biped();
        let joint = Socket::LeftHand.joint(&rig).expect("a wrist");
        let anchor = Socket::LeftHand.anchor(&rig).expect("a hand");
        let carried = rig.joints[joint].position;
        let root = Socket::LeftShoulder.joint(&rig).expect("a shoulder");
        let shoulder = rig.joints[root].position;
        assert!(
            anchor.position.distance(shoulder) >= carried.distance(shoulder),
            "the anchor should sit at least as far down the limb as its joint"
        );
    }

    #[test]
    fn a_seated_socket_stands_clear_of_the_measured_body() {
        // Built the way the body ships, like the surface's own tests: the
        // node radii alone would pass this trivially and prove nothing.
        let mut record = AvatarRecord::new("Seated", Archetype::default());
        record.reroll(7);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let surface = Surface::measure(&mesh, &rig);

        const MARGIN: f32 = 0.02;
        for socket in [
            Socket::Crown,
            Socket::Chest,
            Socket::Back,
            Socket::Waist,
            Socket::LeftHand,
        ] {
            let seated = socket.seat(&rig, &surface, MARGIN).expect("seats");
            // Half the margin, because clearing moved the point and the
            // nearest bone under it may differ by a whisker afterwards; what
            // must hold is that the prop is not inside the body.
            let residue = surface.clearance(&rig, seated.position, MARGIN * 0.5);
            assert_eq!(
                residue,
                crate::Vec3::ZERO,
                "{socket:?} seated inside the body, still owing {residue}"
            );
        }
    }
}
