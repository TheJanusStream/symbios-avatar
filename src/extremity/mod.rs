//! Hands and feet.
//!
//! Both are attached parts rather than more of the capsule graph, for the
//! reasons set out in [`hand`] and [`foot`]. They are placed here, because
//! placing them is the part that has to know about the rig: which joint a limb
//! ends at, which way that limb points, and — the standing lesson of this crate
//! — how thick the body actually is where they join it, which is a question only
//! [`Surface`] can answer.
//!
//! Everything is built in the extremity joint's local space, as the eyes and
//! hair are built in the head's, so a renderer parents each part to its joint
//! and it follows the body for free.

pub mod foot;
pub mod hand;

use glam::{Mat4, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone};
use crate::rig::{Rig, Surface};

pub use foot::Foot;
pub use hand::Hand;

/// How far a resting hand's fingers curl.
///
/// Not zero. A hand held flat reads as a surrender or as a mannequin; a relaxed
/// hand has curved fingers even when it is holding nothing.
const REST_CURL: f32 = 0.38;

/// A part built and placed in its joint's local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Attached {
    /// Which limb it finishes.
    pub limb: Limb,
    /// The joint it is parented to.
    pub joint: usize,
    /// Its geometry, in that joint's local space.
    pub mesh: PolyMesh,
    /// How far it extends from the joint, in metres.
    pub reach: f32,
}

/// Every hand and foot on a body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extremities {
    /// Parts on limbs that reach.
    pub hands: Vec<Attached>,
    /// Parts on limbs that carry.
    pub feet: Vec<Attached>,
}

impl Extremities {
    /// Builds hands and feet for a body.
    ///
    /// A limb that carries the body gets a foot; one that does not gets a hand.
    ///
    /// Asked of the rig, not assumed from which end of the body the limb is on.
    /// Fore limbs are hands only on something that stands upright: a quadruped
    /// walks on all four, and giving its front legs fingers — which is what
    /// reading `is_fore` did — puts a pair of human hands on the ground.
    ///
    /// `ground` is the plane the body **stands on** — `0` for the humanoid
    /// plan, which builds bodies with the floor at the origin. Feet reach down
    /// to it, because how deep a foot is *is* the distance from the ankle to the
    /// floor, and no property of the ankle itself says that.
    ///
    /// Not the lowest point of the body's mesh. That is the bottom of the leg's
    /// last node, which floats above the floor by whatever the plan left it, and
    /// measuring from there gives a foot a couple of centimetres deep that the
    /// leg then pokes straight through.
    #[must_use]
    pub fn build(rig: &Rig, surface: &Surface, ground: f32) -> Self {
        let mut extremities = Self::default();
        let carries = rig.ground_contacts();

        for limb in Limb::ALL {
            let Some(&joint) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            let Some(parent) = rig.joints[joint].parent else {
                continue;
            };
            let along = rig.joints[joint].position - rig.joints[parent].position;
            if along.length_squared() <= f32::EPSILON {
                continue;
            }

            // Measured at the *start* of the final bone — the wrist or the
            // ankle — which is where the part actually joins the body. The node
            // radius there overstates it by half again.
            let girth = surface.radius(joint, 0.0);
            if girth <= f32::EPSILON {
                continue;
            }

            if !carries.contains(&limb) {
                extremities.hands.push(grow_hand(limb, joint, along, girth));
            } else {
                let drop = rig.joints[joint].position.y - ground;
                extremities
                    .feet
                    .push(grow_foot(limb, joint, along, girth, drop));
            }
        }

        extremities
    }

    /// Every part on the body, as one mesh, given each joint's transform.
    ///
    /// For tools that want the whole body in one piece; a renderer is better off
    /// drawing each part under its own joint.
    #[must_use]
    pub fn assembled(&self, transform: impl Fn(usize) -> Mat4) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for part in self.hands.iter().chain(&self.feet) {
            mesh.append(&part.mesh.transformed(transform(part.joint)));
        }
        mesh
    }

    /// How many parts were built.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hands.len() + self.feet.len()
    }

    /// Whether the body got no hands or feet at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds one hand and sets it back so the palm straddles the wrist.
fn grow_hand(limb: Limb, joint: usize, along: Vec3, girth: f32) -> Attached {
    let out = along.normalize();
    let hand = Hand::build(girth, out, Vec3::Y, REST_CURL);
    // The palm starts at the wrist crease, which is behind the joint the part
    // hangs from — otherwise the hand floats off the end of the arm with the
    // limb's own rounded tip showing through between them.
    // Set back by the whole bone, so the palm's round base sits exactly at the
    // wrist joint whose girth it was sized from. Anywhere short of that puts it
    // partway up a tapering forearm, where the arm is thicker than the base is,
    // and the mismatch shows as a step.
    let mesh = hand.mesh().transformed(Mat4::from_translation(-along));
    Attached {
        limb,
        joint,
        reach: hand.length,
        mesh,
    }
}

/// Builds one foot, pointing the way the ankle bone leans.
fn grow_foot(limb: Limb, joint: usize, along: Vec3, girth: f32, drop: f32) -> Attached {
    // Only the horizontal part of the bone. The ankle bone drops as well as
    // reaching forward, and a foot built along it would point its toes into the
    // ground.
    let flat = Vec3::new(along.x, 0.0, along.z);
    let forward = flat.normalize_or(Vec3::Z);
    // The plan has already said how long a foot is, by how far forward of the
    // ankle it put the last node. Toes carry on past it.
    let length = (flat.length() * 1.85).max(girth * 2.4);
    let foot = Foot::build(girth, forward, Vec3::Y, length, drop.max(girth * 0.6));

    // Built about the ankle, which sits behind the joint the part hangs from.
    let ankle = Mat4::from_translation(-flat);
    Attached {
        limb,
        joint,
        reach: foot.length,
        mesh: foot.mesh.transformed(ankle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn body(seed: i64) -> (Rig, Surface, f32) {
        let mut record = AvatarRecord::new("Handed", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let surface = Surface::measure(&mesh, &rig);
        // The plan stands its bodies on the origin.
        (rig, surface, 0.0)
    }

    #[test]
    fn a_biped_gets_two_hands_and_two_feet() {
        let (rig, surface, ground) = body(1);
        let built = Extremities::build(&rig, &surface, ground);
        assert_eq!(built.hands.len(), 2);
        assert_eq!(built.feet.len(), 2);
        assert_eq!(built.len(), 4);
        assert!(!built.is_empty());
    }

    #[test]
    fn feet_go_on_the_limbs_that_carry_the_body() {
        let (rig, surface, ground) = body(7);
        let built = Extremities::build(&rig, &surface, ground);
        let carries = rig.ground_contacts();
        assert!(built.feet.iter().all(|part| carries.contains(&part.limb)));
        assert!(built.hands.iter().all(|part| !carries.contains(&part.limb)));
    }

    #[test]
    fn a_quadruped_walks_on_four_feet_and_has_no_hands() {
        // Read from which end of the body a limb is on, a quadruped's front
        // legs come out as a pair of human hands, on the ground.
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let surface = Surface::measure(&mesh, &rig);

        let built = Extremities::build(&rig, &surface, 0.0);
        assert_eq!(built.feet.len(), 4, "a quadruped stands on four");
        assert!(built.hands.is_empty(), "a quadruped has no hands");
    }

    #[test]
    fn every_part_hangs_from_an_extremity_joint() {
        let (rig, surface, ground) = body(23);
        let built = Extremities::build(&rig, &surface, ground);
        for part in built.hands.iter().chain(&built.feet) {
            assert_eq!(rig.joints[part.joint].zone, Zone::Extremity(part.limb));
        }
    }

    #[test]
    fn hands_and_feet_are_sized_from_the_measured_body_not_the_planned_one() {
        // If this ever reads the node radius instead, hands come out half again
        // too big — which is the mistake this crate keeps making.
        let (rig, surface, ground) = body(3);
        let built = Extremities::build(&rig, &surface, ground);
        let hand = &built.hands[0];
        let planned = rig.joints[hand.joint].radius;
        let measured = surface.radius(hand.joint, 0.0);
        assert!(
            measured < planned,
            "the wrist measured {measured} against a planned {planned}"
        );
        // Stated against the shape itself rather than a hard-coded ratio, so
        // retuning a hand's proportions cannot quietly invalidate the check.
        let unit = Hand::build(1.0, Vec3::X, Vec3::Y, REST_CURL).length;
        assert!(
            (hand.reach - unit * measured).abs() < 1e-4,
            "reach {} is not {unit} wrists of {measured}",
            hand.reach
        );
        assert!(
            (hand.reach - unit * planned).abs() > 1e-3,
            "the hand was sized from the planned radius"
        );
    }

    #[test]
    fn feet_reach_forward_and_hands_reach_outward() {
        let (rig, surface, ground) = body(11);
        let built = Extremities::build(&rig, &surface, ground);

        for foot in &built.feet {
            let (lo, hi) = foot.mesh.bounds();
            assert!(lo.y < 0.0, "a foot did not sit below its joint");
            // Measured from the ankle, which sits behind the joint the part
            // hangs from — the last node of the leg is out near the ball of the
            // foot, so most of the foot is behind it and that says nothing.
            let parent = rig.joints[foot.joint].parent.expect("a foot has an ankle");
            let ankle = rig.joints[parent].position - rig.joints[foot.joint].position;
            assert!(
                hi.z - ankle.z > (ankle.z - lo.z) * 1.5,
                "a foot reached {} in front of its ankle and {} behind",
                hi.z - ankle.z,
                ankle.z - lo.z
            );
        }
        for hand in &built.hands {
            // Along the limb's own axis, not along world X: the part is set back
            // by the whole wrist bone, so measuring from the joint it hangs from
            // understates how far it reaches.
            let parent = rig.joints[hand.joint].parent.expect("a hand has a wrist");
            let out = (rig.joints[hand.joint].position - rig.joints[parent].position).normalize();
            let across = out.cross(Vec3::Y).normalize();
            let span = |axis: Vec3| {
                let reach = hand.mesh.positions.iter().map(|p| p.dot(axis));
                reach.clone().fold(f32::MIN, f32::max) - reach.fold(f32::MAX, f32::min)
            };
            assert!(
                span(out) > span(across),
                "a hand reached {} along the arm and {} across it",
                span(out),
                span(across)
            );
        }
    }

    #[test]
    fn the_two_hands_are_mirrors_of_each_other() {
        let (rig, surface, ground) = body(5);
        let built = Extremities::build(&rig, &surface, ground);
        let spans: Vec<f32> = built
            .hands
            .iter()
            .map(|hand| {
                let (lo, hi) = hand.mesh.bounds();
                hi.x - lo.x
            })
            .collect();
        assert!((spans[0] - spans[1]).abs() < 1e-5, "{spans:?}");
    }

    #[test]
    fn assembling_places_every_part_somewhere_different() {
        let (rig, surface, ground) = body(9);
        let built = Extremities::build(&rig, &surface, ground);
        let whole = built.assembled(|joint| Mat4::from_translation(rig.joints[joint].position));
        let (lo, hi) = whole.bounds();
        // Two hands out to the sides and two feet on the ground: the assembly
        // should span most of the body.
        assert!(hi.x - lo.x > 0.5, "parts spanned only {}", hi.x - lo.x);
        assert!(hi.y - lo.y > 0.5, "parts spanned only {}", hi.y - lo.y);
    }

    #[test]
    fn a_foot_reaches_the_ground_the_body_stands_on() {
        // The point of taking a ground plane at all. Sized from anything about
        // the ankle itself, a foot comes out a wafer the leg pokes through.
        let (rig, surface, ground) = body(1);
        let built = Extremities::build(&rig, &surface, ground);
        for foot in &built.feet {
            let sole = foot.mesh.bounds().0.y + rig.joints[foot.joint].position.y;
            assert!(
                (sole - ground).abs() < 1e-3,
                "a sole sat at {sole} against a ground of {ground}"
            );
        }
    }

    #[test]
    fn extremities_are_reproducible() {
        let (rig, surface, ground) = body(13);
        assert_eq!(
            Extremities::build(&rig, &surface, ground),
            Extremities::build(&rig, &surface, ground)
        );
    }
}
