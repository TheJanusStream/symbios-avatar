//! Named anchors on a body.
//!
//! Hair sits on a scalp, a hat sits on a crown, a belt sits on a waist ring. All
//! three need to know *where* those things are on this particular body, and none
//! of them should have to know which body plan built it or how its nodes were
//! numbered.
//!
//! Landmarks are read off the rig's zone tags, so they work for any zone-tagged
//! skeleton — including plans that do not exist yet. A quadruped simply reports
//! the landmarks it has; asking for a biped's shoulder on a body without one
//! returns nothing rather than a guess.

use glam::Vec3;
use std::collections::BTreeMap;

use super::Rig;
use crate::plan::{Limb, Zone};

/// The body's forward direction, shared with glTF and VRM 1.0.
pub const FORWARD: Vec3 = Vec3::Z;

/// The body's up direction.
pub const UP: Vec3 = Vec3::Y;

/// A named place on a body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Landmark {
    /// Top of the skull, where a hat sits.
    Crown,
    /// Front of the skull at eye height, where a face is.
    EyeLine,
    /// Where the neck meets the skull, where hair falls from.
    NapeOfNeck,
    /// Where the neck meets the chest, where a collar sits.
    NeckBase,
    /// The chest at its widest.
    ChestRing,
    /// The narrowest part of the torso, where a belt sits.
    WaistRing,
    /// The pelvis at its widest.
    HipRing,
    /// Where a tail leaves the body.
    TailBase,
    /// Where a limb meets the body: a shoulder, or a hip socket.
    LimbRoot(Limb),
    /// A limb's middle joint: an elbow, or a knee.
    LimbMid(Limb),
    /// A limb's far joint: a wrist, or an ankle.
    LimbWrist(Limb),
    /// The end of a limb: a hand, or a foot.
    LimbTip(Limb),
}

/// A place on the body, with enough context to fit something to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    /// Where it is, in body space.
    pub position: Vec3,
    /// Which way it faces — outward from the body, or along the limb.
    pub direction: Vec3,
    /// How wide the body is here, for sizing whatever attaches.
    pub radius: f32,
}

impl Anchor {
    /// A point on the surface, `distance` radii out along the anchor's direction.
    #[must_use]
    pub fn offset(&self, distance: f32) -> Vec3 {
        self.position + self.direction * (self.radius * distance)
    }
}

/// Every anchor a body offers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Landmarks {
    anchors: BTreeMap<Landmark, Anchor>,
}

impl Landmarks {
    /// Reads the anchors off a rig's zone tags.
    #[must_use]
    pub fn from_rig(rig: &Rig) -> Self {
        let mut anchors = BTreeMap::new();

        // Torso rings sit at the body's own nodes, facing along the spine.
        for (zone, landmark) in [
            (Zone::Chest, Landmark::ChestRing),
            (Zone::Abdomen, Landmark::WaistRing),
            (Zone::Pelvis, Landmark::HipRing),
        ] {
            if let Some(&joint) = rig.in_zone(zone).first() {
                anchors.insert(landmark, anchor_at(rig, joint, UP));
            }
        }

        if let Some(&neck) = rig.in_zone(Zone::Neck).first() {
            anchors.insert(Landmark::NeckBase, anchor_at(rig, neck, UP));
        }

        if let Some(&head) = rig.in_zone(Zone::Head).first() {
            let joint = rig.joints[head];
            // The head's own bone gives the skull's axis, so a tilted head — a
            // quadruped's, say — still reports a crown that is on top of it.
            let (start, end) = rig.bone(head);
            let axis = (end - start).normalize_or(UP);
            anchors.insert(
                Landmark::Crown,
                Anchor {
                    position: joint.position,
                    direction: axis,
                    radius: joint.radius,
                },
            );
            anchors.insert(
                Landmark::NapeOfNeck,
                Anchor {
                    position: joint.position,
                    direction: -(axis + FORWARD).normalize_or(-FORWARD),
                    radius: joint.radius,
                },
            );
            anchors.insert(
                Landmark::EyeLine,
                Anchor {
                    position: joint.position + axis * (joint.radius * 0.25),
                    direction: FORWARD,
                    radius: joint.radius,
                },
            );
        }

        if let Some(&tail) = rig.in_zone(Zone::Tail).first() {
            anchors.insert(Landmark::TailBase, anchor_at(rig, tail, -FORWARD));
        }

        for limb in Limb::ALL {
            // Within a zone the rig is ordered outward from the body, so the
            // first upper-limb joint is the socket and the last is the mid joint.
            let upper = rig.in_zone(Zone::UpperLimb(limb));
            if let Some(&root) = upper.first() {
                anchors.insert(Landmark::LimbRoot(limb), limb_anchor(rig, root));
            }
            if let Some(&mid) = upper.last() {
                anchors.insert(Landmark::LimbMid(limb), limb_anchor(rig, mid));
            }
            if let Some(&wrist) = rig.in_zone(Zone::LowerLimb(limb)).last() {
                anchors.insert(Landmark::LimbWrist(limb), limb_anchor(rig, wrist));
            }
            if let Some(&tip) = rig.in_zone(Zone::Extremity(limb)).last() {
                anchors.insert(Landmark::LimbTip(limb), limb_anchor(rig, tip));
            }
        }

        Self { anchors }
    }

    /// The anchor for `landmark`, if this body has one.
    #[must_use]
    pub fn get(&self, landmark: Landmark) -> Option<Anchor> {
        self.anchors.get(&landmark).copied()
    }

    /// Whether this body offers `landmark`.
    #[must_use]
    pub fn has(&self, landmark: Landmark) -> bool {
        self.anchors.contains_key(&landmark)
    }

    /// How many anchors this body offers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Whether the body offers no anchors at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Every anchor, in landmark order.
    pub fn iter(&self) -> impl Iterator<Item = (Landmark, Anchor)> + '_ {
        self.anchors.iter().map(|(&name, &anchor)| (name, anchor))
    }

    /// Distance between two landmarks, if the body has both.
    ///
    /// The measurement side of the API: interocular distance, shoulder span, and
    /// inseam are all just distances between anchors.
    #[must_use]
    pub fn span(&self, from: Landmark, to: Landmark) -> Option<f32> {
        Some(self.get(from)?.position.distance(self.get(to)?.position))
    }
}

/// An anchor at a joint, facing `fallback` when it has no bone of its own.
fn anchor_at(rig: &Rig, joint: usize, fallback: Vec3) -> Anchor {
    let here = rig.joints[joint];
    Anchor {
        position: here.position,
        direction: fallback,
        radius: here.radius,
    }
}

/// An anchor at a limb joint, facing outward along the limb.
fn limb_anchor(rig: &Rig, joint: usize) -> Anchor {
    let here = rig.joints[joint];
    let (start, end) = rig.bone(joint);
    let axis = (end - start).normalize_or(UP);
    Anchor {
        position: here.position,
        direction: axis,
        radius: here.radius,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn biped() -> Landmarks {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
            .landmarks()
    }

    #[test]
    fn a_biped_offers_the_anchors_a_wardrobe_needs() {
        let marks = biped();
        for landmark in [
            Landmark::Crown,
            Landmark::EyeLine,
            Landmark::NeckBase,
            Landmark::ChestRing,
            Landmark::WaistRing,
            Landmark::HipRing,
        ] {
            assert!(marks.has(landmark), "missing {landmark:?}");
        }
        for limb in Limb::ALL {
            assert!(marks.has(Landmark::LimbRoot(limb)));
            assert!(marks.has(Landmark::LimbTip(limb)));
        }
        assert!(!marks.has(Landmark::TailBase), "a biped has no tail");
    }

    #[test]
    fn anchors_are_stacked_the_way_a_body_is() {
        let marks = biped();
        let height = |l: Landmark| marks.get(l).expect("present").position.y;

        assert!(height(Landmark::Crown) > height(Landmark::NeckBase));
        assert!(height(Landmark::NeckBase) > height(Landmark::ChestRing));
        assert!(height(Landmark::ChestRing) > height(Landmark::WaistRing));
        assert!(height(Landmark::WaistRing) > height(Landmark::HipRing));
        assert!(height(Landmark::HipRing) > height(Landmark::LimbTip(Limb::HindLeft)));
    }

    #[test]
    fn limb_anchors_land_on_the_correct_side() {
        let marks = biped();
        let x = |l: Landmark| marks.get(l).expect("present").position.x;

        // Left is `+X` on a body facing `+Z` — see [`crate::plan::Limb`], and
        // #142 for the pass that moved our names onto the sides they name. The
        // signs here are absolute for the reason given there: comparing the two
        // limbs against each other would pass on a body that was simply the
        // wrong way round.
        assert!(x(Landmark::LimbTip(Limb::ForeLeft)) > 0.0);
        assert!(x(Landmark::LimbTip(Limb::ForeRight)) < 0.0);
        assert!(x(Landmark::LimbRoot(Limb::HindLeft)) > 0.0);
        assert!(x(Landmark::LimbRoot(Limb::HindRight)) < 0.0);
    }

    #[test]
    fn the_crown_sits_on_top_of_the_head() {
        let marks = biped();
        let crown = marks.get(Landmark::Crown).expect("present");
        assert!(crown.direction.dot(UP) > 0.9, "crown faces up");
        assert!(crown.offset(1.0).y > crown.position.y);
    }

    #[test]
    fn spans_measure_the_body() {
        let marks = biped();
        let shoulders = marks
            .span(
                Landmark::LimbRoot(Limb::ForeLeft),
                Landmark::LimbRoot(Limb::ForeRight),
            )
            .expect("both shoulders");
        let arms = marks
            .span(
                Landmark::LimbTip(Limb::ForeLeft),
                Landmark::LimbTip(Limb::ForeRight),
            )
            .expect("both hands");
        assert!(arms > shoulders, "arm span exceeds shoulder span");
        assert!(shoulders > 0.1);

        assert_eq!(marks.span(Landmark::TailBase, Landmark::Crown), None);
    }

    #[test]
    fn a_quadruped_reports_a_tail_and_four_limbs() {
        let marks =
            Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
                .expect("rigs")
                .landmarks();

        assert!(marks.has(Landmark::TailBase));
        assert!(marks.has(Landmark::Crown));
        for limb in Limb::ALL {
            assert!(marks.has(Landmark::LimbTip(limb)), "missing {limb:?} foot");
        }

        // The head leads and the tail trails, along the body's forward axis.
        let head = marks.get(Landmark::Crown).expect("present").position.z;
        let tail = marks.get(Landmark::TailBase).expect("present").position.z;
        assert!(head > tail);
    }

    #[test]
    fn scaling_the_body_scales_its_landmarks() {
        let short = Rig::from_skeleton(
            &HumanoidParams {
                height: 1.3,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
        .landmarks();
        let tall = Rig::from_skeleton(
            &HumanoidParams {
                height: 2.1,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs")
        .landmarks();

        let crown = |m: &Landmarks| m.get(Landmark::Crown).expect("present").position.y;
        assert!(crown(&tall) > crown(&short) * 1.4);
        assert_eq!(
            short.len(),
            tall.len(),
            "the same body offers the same anchors"
        );
    }
}
