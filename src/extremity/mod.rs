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

use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::{Limb, Zone};
use crate::rig::skin::Influence;
use crate::rig::{Rig, Role, Surface};

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
    ///
    /// **The rig is grown, not just read.** Each hand hangs
    /// [`hand::BONES`] − 1 further joints off its wrist — a knuckle and two more
    /// down each of five digits, in [`Role::Digit`] — and the hand mesh comes
    /// back skinned to them rather than bound rigidly. That is the reference's
    /// layout exactly: `hand_l`, then `<digit>_01`, `_02`, `_03` and `_04_leaf`
    /// for thumb, index, middle, ring and pinky, twenty-one bones a hand (#113).
    ///
    /// Taking `&mut Rig` rather than cloning one is the point: a hand skinned
    /// against a rig the caller does not hold is a hand bound to joints that do
    /// not exist.
    #[must_use]
    pub fn build(rig: &mut Rig, surface: &Surface, ground: f32) -> Self {
        let mut extremities = Self::default();
        let carries = rig.ground_contacts();
        // How tall the body actually stands, which is what a foot is in
        // proportion to. Taken from the MEASURED surface at the head rather than
        // from its node radius — subdivision pulls the mesh well inside the
        // radius the plan asked for, so a crown predicted from the plan overshoots
        // and every foot on the body grows with it.
        let stature = rig
            .in_zone(Zone::Head)
            .first()
            .map(|&head| rig.joints[head].position.y + surface.widest(head) - ground)
            .filter(|tall| *tall > f32::EPSILON);

        for limb in Limb::ALL {
            let extremity = rig.in_zone(Zone::Extremity(limb));
            let Some(&joint) = extremity.first() else {
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
                extremities
                    .hands
                    .push(grow_hand(rig, limb, joint, along, girth));
            } else if extremity.len() < 2 {
                // **A foot is only a part on a plan that has not got one.** The
                // humanoid meshes its feet as nodes in the capsule graph — ankle,
                // ball, toe — so the leg runs continuously into the foot and there
                // is no seam to hide (#111), and hanging a swept slab there as
                // well would bury one solid inside another.
                //
                // Asked of the graph rather than of the plan: a limb whose
                // extremity zone holds more than one node has its foot already.
                // The quadruped plan still ends each leg in a single node and
                // still gets a built foot, and will go on doing so until it grows
                // the same chain.
                let drop = rig.joints[joint].position.y - ground;
                extremities
                    .feet
                    .push(grow_foot(limb, joint, along, girth, drop, stature));
            }
        }

        extremities
    }

    /// Every hand and foot, in a fixed order.
    ///
    /// As with [`crate::face::Features::meshes`], the order is the contract
    /// between whatever sizes these and whatever places them.
    pub fn all(&self) -> impl Iterator<Item = &Attached> {
        self.hands.iter().chain(&self.feet)
    }

    /// The same walk, for writing.
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut Attached> {
        self.hands.iter_mut().chain(&mut self.feet)
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
///
/// **One hand is built and the other is its reflection, which is how both
/// reference bodies are made** (#113). Measured off the GLBs, the Quaternius
/// male's mesh is 3,619 vertices either side of the midline and reflecting one
/// onto the other lands to 0.000 mm — mean and worst alike — and every paired
/// bone from `upperarm` out to `thumb_04_leaf` reflects to the same 0.000 mm.
/// The body is one side, mirrored.
///
/// Ours was not. [`Hand::build`] derives its whole frame from the direction it
/// is handed, so feeding it each arm in turn *rotates* the hand rather than
/// reflecting it — a half turn about the body's axis, which carries the thumb
/// around with it. On the default body the two hands reflected onto each other
/// to 5.5 mm mean and 33.4 mm worst, and the thumbs pointed to opposite ends of
/// the fore-aft axis: one hand had its thumb in front of the palm and the other
/// behind, which is a pair of right hands.
///
/// So the hand is built once, in the half-space where `Hand::build`'s own
/// chirality is the wanted one, and reflected across the sagittal plane for the
/// arm on the other side. The reflection is a negative-determinant transform,
/// and [`PolyMesh::transformed`] reverses each face's winding to match, so the
/// reflected copy still faces outward.
///
/// The test is `the_two_hands_are_mirrors_of_each_other`, and it now measures
/// what its name says: every vertex of one hand against the nearest vertex of
/// the reflected other.
fn grow_hand(rig: &mut Rig, limb: Limb, joint: usize, along: Vec3, girth: f32) -> Attached {
    // Which side is canonical is a fact about `Hand::build`, not about which
    // limb this is: with `up` on world Y it seats the thumb toward +Z — the
    // body's front, where both reference thumbs are — only when `out.x` is
    // negative. Asked of the geometry rather than of `Limb`, so a plan that
    // puts its limbs somewhere else still gets a matched pair.
    let reflected = along.x > 0.0;
    let canonical = if reflected {
        Vec3::new(-along.x, along.y, along.z)
    } else {
        along
    };

    let hand = Hand::build(girth, canonical.normalize(), Vec3::Y, REST_CURL);
    // The palm starts at the wrist crease, which is behind the joint the part
    // hangs from — otherwise the hand floats off the end of the arm with the
    // limb's own rounded tip showing through between them.
    // Set back by the whole bone, so the palm's round base sits exactly at the
    // wrist joint whose girth it was sized from. Anywhere short of that puts it
    // partway up a tapering forearm, where the arm is thicker than the base is,
    // and the mismatch shows as a step.
    //
    // The same transform carries the digits' joints, because they were read off
    // the sweep's own stations and have to keep landing on it. A reflection
    // applied to one and not the other is a hand whose bones are inside the
    // other hand.
    let place = Mat4::from_translation(-canonical);
    let place = if reflected {
        // In the joint's own space, whose origin is on the wrist and whose axes
        // are the world's, so this is the sagittal plane.
        Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) * place
    } else {
        place
    };
    let mesh = hand.mesh().transformed(place);

    // The rig's own numbering for the hand's bones, in `Hand::influences`
    // order: the wrist first, then each digit from the knuckle out. Attached
    // parent-before-child down each digit, which is the order the whole crate
    // relies on and the order glTF requires.
    let wrist = rig.joints[joint].position;
    let mut bones = vec![joint];
    for digit in &hand.digits {
        let mut parent = joint;
        for &local in &digit.joints {
            let at = wrist + place.transform_point3(local);
            parent = rig
                .attach(parent, at, Role::Digit)
                .expect("the wrist is a joint of this rig");
            bones.push(parent);
        }
    }

    let mut mesh = mesh;
    mesh.set_skin(
        hand.influences()
            .into_iter()
            .map(|shares| {
                let mut skin = VertexSkin::default();
                for (slot, (bone, weight)) in shares.into_iter().enumerate() {
                    skin[slot] = Influence {
                        joint: bones[bone] as u16,
                        weight,
                    };
                }
                skin
            })
            .collect(),
    );

    Attached {
        limb,
        joint,
        reach: hand.length,
        mesh,
    }
}

/// Builds one foot, pointing the way the ankle bone leans.
fn grow_foot(
    limb: Limb,
    joint: usize,
    along: Vec3,
    girth: f32,
    drop: f32,
    stature: Option<f32>,
) -> Attached {
    // Only the horizontal part of the bone. The ankle bone drops as well as
    // reaching forward, and a foot built along it would point its toes into the
    // ground.
    let flat = Vec3::new(along.x, 0.0, along.z);
    let forward = flat.normalize_or(Vec3::Z);
    // **A foot's length is set by how high its ankle is, not by the bone.** The
    // ankle bone only says which way the toes point; how far forward the plan put
    // that node is a fact about the shin, and reading a foot off it gave one
    // 10.6% of stature long against a measured 15.7–16.4% — two thirds the size
    // it should be, which is the single most visible thing about our feet (#110).
    //
    // Measured on both Quaternius bodies, a foot is 16.4% of stature (male) and
    // 15.7% (female). **Against STATURE, not against `drop`**, and that was
    // decided by measuring the alternative rather than by argument: a foot is
    // also close to three times its own ankle height on the reference (3.06 and
    // 2.95), and that relation gives the wrong answer here — it produced a foot
    // 7.8% of stature, shorter than the 10.6% the previous guess managed.
    //
    // The reason is that `drop` is NOT the ankle's height. It is the height of
    // the plan's last leg node, the one this part hangs from, which sits at 2.57%
    // of stature where the ankle proper is at 6.86%. Reading the reference's
    // ankle against it compares two different landmarks — the reference ankle is
    // at 5.3–5.4% of stature, so our ankle is if anything slightly HIGH, and the
    // thing that is low is the node the foot is grown from.
    //
    // That is also why the foot comes out thin: its depth is `drop`, so it is as
    // thick as a node that sits barely above the floor, against a reference foot
    // about 4.8% of stature thick. Fixing it means meshing the foot from the
    // capsule graph instead of hanging a slab off one low node (#111), which is
    // what is happening next; this keeps the length honest in the meantime.
    //
    // Falls back to the `drop` relation on a body with no head to measure against.
    let length = stature
        .map_or(drop * 3.0, |tall| tall * 0.16)
        .max(girth * 2.4);
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
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let surface = Surface::measure(&mesh, &rig);
        // The plan stands its bodies on the origin.
        (rig, surface, 0.0)
    }

    #[test]
    fn a_biped_gets_two_hands_and_leaves_its_feet_to_the_cage() {
        // **A biped's feet are not parts any more** (#111). They are ankle, ball
        // and toe nodes in the capsule graph, so the leg is continuous into the
        // foot and nothing is hung off the end of it. Two hands is the whole of
        // what this builds for an upright body, and a foot appearing here again
        // would mean a swept slab buried inside the meshed one.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        assert_eq!(built.hands.len(), 2);
        assert_eq!(built.feet.len(), 0, "a humanoid foot comes from the cage");
        assert_eq!(built.len(), 2);
        assert!(!built.is_empty());
    }

    #[test]
    fn feet_go_on_the_limbs_that_carry_the_body() {
        let (mut rig, surface, ground) = body(7);
        let built = Extremities::build(&mut rig, &surface, ground);
        let carries = rig.ground_contacts();
        assert!(built.feet.iter().all(|part| carries.contains(&part.limb)));
        assert!(built.hands.iter().all(|part| !carries.contains(&part.limb)));
    }

    #[test]
    fn a_quadruped_walks_on_four_feet_and_has_no_hands() {
        // Read from which end of the body a limb is on, a quadruped's front
        // legs come out as a pair of human hands, on the ground.
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let mut rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let surface = Surface::measure(&mesh, &rig);

        let built = Extremities::build(&mut rig, &surface, 0.0);
        assert_eq!(built.feet.len(), 4, "a quadruped stands on four");
        assert!(built.hands.is_empty(), "a quadruped has no hands");
    }

    #[test]
    fn every_part_hangs_from_an_extremity_joint() {
        let (mut rig, surface, ground) = body(23);
        let built = Extremities::build(&mut rig, &surface, ground);
        for part in built.hands.iter().chain(&built.feet) {
            assert_eq!(rig.joints[part.joint].zone, Zone::Extremity(part.limb));
        }
    }

    #[test]
    fn hands_and_feet_are_sized_from_the_measured_body_not_the_planned_one() {
        // If this ever reads the node radius instead, hands come out half again
        // too big — which is the mistake this crate keeps making.
        //
        // Seed re-picked for generator 2 (#160): the premise below — the
        // measured wrist is thinner than the planned one — holds on a body
        // whose build and extremity axes agree, and the exploration draw can
        // hand seed 3 a thick forearm over a small hand, where the surface at
        // the wrist ring is honestly FATTER than the hand's own node asks.
        // Seed 29 rolls both axes inside the old range.
        let (mut rig, surface, ground) = body(29);
        let built = Extremities::build(&mut rig, &surface, ground);
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
        let (mut rig, surface, ground) = body(11);
        let built = Extremities::build(&mut rig, &surface, ground);

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
        // **Measured as a reflection, not as a pair of bounding boxes** (#113).
        // This used to compare the two hands' x spans, which a hand rotated half
        // a turn satisfies exactly as well as a reflected one — and that is what
        // the body had: two right hands, agreeing on every box measurement
        // anyone thought to take.
        //
        // The check is the one the reference passes. Reflect one hand across the
        // sagittal plane and ask, for every vertex of it, how far the nearest
        // vertex of the other hand is. On the Quaternius male that answer is
        // 0.000 mm at both the mean and the worst, over 3,619 vertices a side;
        // its body is one half mirrored, and so is ours now.
        let (mut rig, surface, ground) = body(5);
        let built = Extremities::build(&mut rig, &surface, ground);
        assert_eq!(built.hands.len(), 2);

        // In world space, because the two parts live in different joints' spaces
        // and reflecting inside one of them would compare a hand with itself.
        let world: Vec<PolyMesh> = built
            .hands
            .iter()
            .map(|hand| {
                hand.mesh
                    .transformed(Mat4::from_translation(rig.joints[hand.joint].position))
            })
            .collect();
        let reflected = world[0].transformed(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));

        let mut worst = 0.0f32;
        for &point in &reflected.positions {
            let nearest = world[1]
                .positions
                .iter()
                .map(|&other| other.distance(point))
                .fold(f32::MAX, f32::min);
            worst = worst.max(nearest);
        }
        // A tenth of a millimetre. The construction is exact up to float error —
        // the reflected hand is built from the same numbers with one sign
        // changed — so this is a check for a wrong *shape*, not a tolerance on a
        // fit. Before the fix it read 33.4 mm.
        assert!(
            worst < 1e-4,
            "reflecting one hand onto the other left {:.3} mm; the pair are not \
             reflections, which means one of them is the wrong hand",
            worst * 1000.0
        );

        // And the thumbs agree about which way the body faces, which is the
        // property a reflection preserves and a half-turn destroys.
        for hand in &built.hands {
            let (lo, hi) = hand.mesh.bounds();
            let _ = lo;
            assert!(
                hi.z > 0.0,
                "a hand reached only to z {}, so its thumb is behind the palm",
                hi.z
            );
        }
    }

    #[test]
    fn assembling_places_every_part_somewhere_different() {
        let (mut rig, surface, ground) = body(9);
        let built = Extremities::build(&mut rig, &surface, ground);
        let whole = built.assembled(|joint| Mat4::from_translation(rig.joints[joint].position));
        let (lo, hi) = whole.bounds();
        // Two hands, one at the end of each arm. The span is lateral now that
        // feet are meshed with the body rather than assembled here (#111), so
        // the vertical reach this used to check went with them — what is left is
        // the property it was really testing, that each part is placed at its own
        // joint instead of all of them landing on the origin.
        assert!(hi.x - lo.x > 0.5, "parts spanned only {}", hi.x - lo.x);
        assert!(
            lo.x < 0.0 && hi.x > 0.0,
            "both hands ended up on the same side"
        );
    }

    #[test]
    fn a_foot_reaches_the_ground_the_body_stands_on() {
        // The point of taking a ground plane at all. Sized from anything about
        // the ankle itself, a foot comes out a wafer the leg pokes through.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
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
        // Two bodies, not one built twice. Building grows the rig, so a second
        // pass over the same rig hangs a second set of finger bones off it and
        // is a different question — one whose honest answer is "no".
        let (mut first, surface, ground) = body(13);
        let (mut again, _, _) = body(13);
        assert_eq!(
            Extremities::build(&mut first, &surface, ground),
            Extremities::build(&mut again, &surface, ground)
        );
    }

    #[test]
    fn each_hand_carries_the_references_twenty_one_bones() {
        // `hand_l`, then four joints down each of five digits. The reference
        // rigs its hands exactly this way and the whole point of matching it is
        // that a hand pose or a grip retargets between the two without a table.
        let (mut rig, surface, ground) = body(1);
        let before = rig.len();
        let built = Extremities::build(&mut rig, &surface, ground);

        let digits: Vec<usize> = (0..rig.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit)
            .collect();
        assert_eq!(digits.len(), (hand::BONES - 1) * built.hands.len());
        assert_eq!(rig.len(), before + digits.len());
        assert_eq!(hand::BONES, 21, "the reference carries twenty-one a hand");

        // Parent-before-child down every digit, which glTF requires and the rest
        // of the crate assumes.
        for &joint in &digits {
            let parent = rig.joints[joint]
                .parent
                .expect("a digit hangs off something");
            assert!(parent < joint, "joint {joint} precedes its parent {parent}");
            assert_eq!(
                rig.joints[joint].zone, rig.joints[parent].zone,
                "a finger left its own hand's zone"
            );
        }
    }

    #[test]
    fn the_two_hands_bones_are_mirrors_of_each_other() {
        // The same claim as for the meshes, about the rig. Every paired bone of
        // the reference — `upperarm` out to `thumb_04_leaf` — reflects onto its
        // opposite to 0.000 mm, and a hand rig that does not is one where a
        // shared animation puts a left thumb where a right one should be.
        let (mut rig, surface, ground) = body(1);
        // Only the rig matters here; the parts are checked elsewhere.
        let _ = Extremities::build(&mut rig, &surface, ground);

        let digits: Vec<Vec3> = (0..rig.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit)
            .map(|joint| rig.joints[joint].position)
            .collect();
        assert!(!digits.is_empty());

        let mut worst = 0.0f32;
        for &at in &digits {
            let mirror = Vec3::new(-at.x, at.y, at.z);
            worst = worst.max(
                digits
                    .iter()
                    .map(|&other| other.distance(mirror))
                    .fold(f32::MAX, f32::min),
            );
        }
        assert!(
            worst < 1e-6,
            "a digit joint reflected onto nothing nearer than {:.4} mm",
            worst * 1000.0
        );
    }

    #[test]
    fn a_finger_is_bound_to_its_own_bones_and_to_no_others() {
        // What the rig is for. Every vertex of a digit has to be held by that
        // digit's chain or by the wrist it hangs from — one stray weight and a
        // closing fist drags a knuckle from the other side of the hand with it.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        let hand = &built.hands[0];
        assert_eq!(
            hand.mesh.skin.len(),
            hand.mesh.vertex_count(),
            "the hand came back unskinned, so it is glued shut"
        );

        // The bones this hand may legally use: its wrist, and the digit joints
        // descended from it.
        let mut allowed = vec![hand.joint];
        for joint in 0..rig.len() {
            if rig.joints[joint].role == Role::Digit {
                let mut walk = rig.joints[joint].parent;
                while let Some(parent) = walk {
                    if parent == hand.joint {
                        allowed.push(joint);
                        break;
                    }
                    walk = rig.joints[parent].parent;
                }
            }
        }
        assert_eq!(allowed.len(), hand::BONES);

        for influences in &hand.mesh.skin {
            let total: f32 = influences.iter().map(|hold| hold.weight).sum();
            assert!(
                (total - 1.0).abs() < 1e-5,
                "a hand vertex was held {total} times over"
            );
            for hold in influences.iter().filter(|hold| hold.weight > 0.0) {
                assert!(
                    allowed.contains(&(hold.joint as usize)),
                    "a hand vertex was bound to joint {}, which is not one of its own",
                    hold.joint
                );
            }
        }
    }

    #[test]
    fn bending_a_knuckle_closes_one_finger_and_leaves_the_rest_alone() {
        // The measurement that says the rig deforms rather than decorates, and
        // the one a rigid bind fails outright: before #113 the whole hand rode
        // the wrist, so turning any finger joint moved nothing at all.
        use crate::anim::Pose;
        use glam::Quat;

        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        let hand = &built.hands[0];
        let weights = crate::rig::SkinWeights {
            vertices: hand.mesh.skin.clone(),
        };

        // The index finger's knuckle: digit 0, phalanx 0, so the first digit
        // joint attached for this hand.
        let knuckle = (0..rig.len())
            .find(|&joint| {
                rig.joints[joint].role == Role::Digit
                    && rig.joints[joint].parent == Some(hand.joint)
            })
            .expect("a hand has knuckles");

        let mut pose = Pose::rest(&rig);
        pose.rotations[knuckle] = Quat::from_rotation_x(1.0);
        let moved = pose
            .forward(&rig)
            .deform(&rig, &hand.mesh.positions, &weights);

        let stirred = (0..hand.mesh.vertex_count())
            .filter(|&vertex| hand.mesh.positions[vertex].distance(moved[vertex]) > 1e-4)
            .count();
        assert!(stirred > 8, "only {stirred} vertices followed the knuckle");
        assert!(
            stirred < hand.mesh.vertex_count() / 4,
            "{stirred} of {} vertices moved: one knuckle is carrying the whole hand",
            hand.mesh.vertex_count()
        );
    }
}
