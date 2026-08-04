//! Hands.
//!
//! A hand is built as its own part, attached at the wrist, rather than as more
//! nodes in the capsule graph. A palm carrying five digits would need a node
//! with fifteen sockets on it, and joint hulls are the one thing the B-Mesh
//! construction is worst at — the sockets have to clear each other on a sphere,
//! and fifteen of them will not. It would also give every creature without
//! fingers a rig full of joints it does not use.
//!
//! The shape aims at what reads, not at anatomy. At the distance a game is
//! played from, a hand is a flat palm, four fingers of unequal length, and a
//! thumb set well down the side and pointing across the others. Getting those
//! four things right is most of the recognition; knuckle creases are not.
//!
//! Digits rest in a slight curl. A hand held perfectly flat reads as a
//! surrender, or as a mannequin — the relaxed hand has its fingers curved even
//! when nothing is being held.

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::prim;

/// How many segments each digit is swept from.
///
/// Enough that a curl is a curve rather than a hinge.
const JOINTS: usize = 4;

/// Faces around a digit.
const DIGIT_SIDES: usize = 6;

/// Faces around the palm.
const PALM_SIDES: usize = 8;

/// Where each finger sits across the palm and how long it is, relative to the
/// middle finger.
///
/// Index, middle, ring, little. The middle is longest and the little is much
/// shorter than people remember — the stagger across the knuckles is a large
/// part of why a hand reads as a hand rather than as a comb.
///
/// The offsets are in half-palm-widths and are spaced so that neighbouring
/// fingers *touch*. Spread apart with daylight between them, four fingers read
/// as the tines of a rake however well proportioned each one is.
/// Provenance: **tuned by render**. The relative lengths are the looked-up
/// part — middle longest, little much shorter than people remember — but the
/// figures themselves were set by eye, and the constraint that decided the
/// offsets is that neighbouring fingers must touch, which is a rendering
/// observation and not an anthropometric one.
const FINGERS: [(f32, f32); 4] = [(-0.75, 0.94), (-0.25, 1.0), (0.25, 0.92), (0.75, 0.76)];

/// A finger's radius, as a share of the palm's half-width.
///
/// Four of them side by side fill the palm, which is what sets this: a hand is
/// as wide as its fingers are, not wider.
/// Provenance: **derived** — four fingers side by side fill the palm, so
/// this is one quarter by construction.
const FINGER_RADIUS: f32 = 0.25;

/// A built hand, in wrist-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Hand {
    /// The palm slab.
    pub palm: PolyMesh,
    /// Four fingers and a thumb.
    pub digits: Vec<PolyMesh>,
    /// How long the hand is from wrist to fingertip, in metres.
    pub length: f32,
}

impl Hand {
    /// Builds a hand for a wrist of the given measured radius.
    ///
    /// `out` points along the forearm, away from the body; `up` is the back of
    /// the hand. Both are taken from the rig rather than assumed, so a left hand
    /// and a right hand come out mirrored without a special case.
    #[must_use]
    pub fn build(wrist: f32, out: Vec3, up: Vec3, curl: f32) -> Self {
        let out = out.normalize_or(Vec3::X);
        let up = (up - out * up.dot(out)).normalize_or(Vec3::Y);
        let across = out.cross(up);

        // Proportions of a hand against the wrist it grows from. A palm is
        // wider than the wrist and much flatter, and the whole hand runs a bit
        // over three wrist-breadths from crease to fingertip — which is longer
        // than it first looks, and a short palm gives a paw.
        let palm_length = wrist * 2.7;
        let palm_width = wrist * 1.35;
        let palm_depth = wrist * 0.46;

        let palm = prim::sweep(
            &[
                Vec3::ZERO,
                out * (palm_length * 0.35),
                out * (palm_length * 0.75),
                out * palm_length,
            ],
            &[
                // Round, and the same girth as the forearm it emerges from.
                // Starting the palm already flattened leaves the arm's rounded
                // tip standing proud around it, which reads as a cuff.
                Vec2::new(wrist, wrist),
                Vec2::new(palm_width, palm_depth),
                Vec2::new(palm_width, palm_depth * 0.94),
                Vec2::new(palm_width * 0.96, palm_depth * 0.86),
            ],
            PALM_SIDES,
            across,
        );

        let finger_length = palm_length * 0.95;
        let mut digits = Vec::with_capacity(5);
        for (offset, share) in FINGERS {
            digits.push(digit(
                out * (palm_length * 0.94) + across * (offset * palm_width),
                out,
                up,
                across,
                finger_length * share,
                palm_width * FINGER_RADIUS,
                // Fingers curl toward the palm, which is the way `up` is not.
                curl,
            ));
        }

        // The thumb is the whole difference between a hand and a paddle. It sits
        // low on the palm, points across rather than along, and is thicker and
        // shorter than any finger.
        digits.push(digit(
            out * (palm_length * 0.40) - across * (palm_width * 0.88),
            // Angled well forward, not straight out to the side. A thumb held
            // square to the palm reaches further across than the hand is long,
            // which reads as a claw.
            (out * 0.78 - across * 0.63).normalize(),
            up,
            across,
            finger_length * 0.66,
            palm_width * FINGER_RADIUS * 1.25,
            curl * 0.55,
        ));

        Self {
            palm,
            digits,
            length: palm_length + finger_length,
        }
    }

    /// Palm and digits as one mesh.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let mut mesh = self.palm.clone();
        for digit in &self.digits {
            mesh.append(digit);
        }
        mesh
    }
}

/// One finger or thumb, swept along a curling path.
fn digit(
    root: Vec3,
    along: Vec3,
    up: Vec3,
    across: Vec3,
    length: f32,
    radius: f32,
    curl: f32,
) -> PolyMesh {
    let segment = length / JOINTS as f32;
    // Curl bends the digit toward the palm, a little more at each joint, the way
    // a finger closes from the tip inward.
    let bend = curl * 0.55;

    let mut path = Vec::with_capacity(JOINTS + 1);
    let mut at = root;
    let mut direction = along;
    path.push(at);
    for joint in 0..JOINTS {
        let turn = bend * (joint as f32 + 1.0) / JOINTS as f32;
        direction = (direction * turn.cos() - up * turn.sin()).normalize_or(direction);
        at += direction * segment;
        path.push(at);
    }

    let sections: Vec<Vec2> = (0..=JOINTS)
        .map(|joint| {
            // Tapering, but not to a point: a fingertip is rounded, and a cone
            // reads as a claw.
            let along = joint as f32 / JOINTS as f32;
            let taper = 1.0 - 0.32 * along;
            Vec2::new(radius * taper, radius * taper * 0.88)
        })
        .collect();

    prim::sweep(&path, &sections, DIGIT_SIDES, across)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand(curl: f32) -> Hand {
        Hand::build(0.025, Vec3::X, Vec3::Y, curl)
    }

    #[test]
    fn a_hand_has_a_palm_and_five_digits() {
        let hand = hand(0.3);
        assert_eq!(hand.digits.len(), 5);
        assert!(hand.palm.face_count() > 8);
        assert!(hand.mesh().face_count() > 100);
    }

    #[test]
    fn every_part_of_a_hand_is_a_closed_solid() {
        let hand = hand(0.3);
        assert!(
            hand.palm.is_closed_manifold(),
            "palm: {:?}",
            hand.palm.manifold_report()
        );
        for (index, digit) in hand.digits.iter().enumerate() {
            assert!(
                digit.is_closed_manifold(),
                "digit {index}: {:?}",
                digit.manifold_report()
            );
        }
    }

    #[test]
    fn a_hand_reaches_out_along_the_arm() {
        let hand = hand(0.0);
        let (lo, hi) = hand.mesh().bounds();
        assert!(hi.x > 0.0, "the hand did not extend along +X");
        assert!(lo.x > -0.03, "the hand reached back up the arm");
        // Longer than it is wide, as a hand is.
        assert!(hi.x - lo.x > hi.z - lo.z);
    }

    #[test]
    fn a_palm_is_flatter_than_it_is_wide() {
        // Measured at the knuckles, not over the whole solid: the palm's base is
        // deliberately round, to match the forearm it emerges from, and that
        // round end would otherwise set the depth for the whole bounding box.
        let palm = hand(0.0).palm;
        let far = palm.bounds().1.x;
        let knuckles: Vec<&Vec3> = palm.positions.iter().filter(|p| p.x > far * 0.85).collect();
        let across = knuckles.iter().map(|p| p.z.abs()).fold(0.0f32, f32::max);
        let through = knuckles.iter().map(|p| p.y.abs()).fold(0.0f32, f32::max);
        assert!(
            across > through * 1.4,
            "palm measured {across} across and {through} through at the knuckles"
        );
    }

    #[test]
    fn fingers_are_of_unequal_length() {
        // A comb reads as a comb. The stagger across the knuckles is much of
        // what makes a hand recognisable.
        let hand = hand(0.0);
        let reach: Vec<f32> = hand.digits[..4]
            .iter()
            .map(|digit| digit.bounds().1.x)
            .collect();
        let longest = reach.iter().fold(0.0f32, |a, b| a.max(*b));
        let shortest = reach.iter().fold(f32::MAX, |a, b| a.min(*b));
        assert!(
            longest > shortest * 1.1,
            "fingers reached between {shortest} and {longest}"
        );
    }

    #[test]
    fn the_thumb_sits_across_the_palm_not_along_it() {
        let hand = hand(0.0);
        let thumb = hand.digits[4].bounds();
        let fingers = hand.digits[1].bounds();
        // It reaches further to one side and not as far forward.
        assert!(thumb.0.z < fingers.0.z, "the thumb did not sit to the side");
        assert!(
            thumb.1.x < fingers.1.x,
            "the thumb reached as far as a finger"
        );
    }

    #[test]
    fn curling_closes_the_fingers_toward_the_palm() {
        let flat = hand(0.0).mesh().bounds();
        let closed = hand(1.0).mesh().bounds();
        assert!(
            closed.1.x < flat.1.x,
            "a curled hand should not reach as far: {} against {}",
            closed.1.x,
            flat.1.x
        );
        assert!(closed.0.y < flat.0.y, "curling should dip below the palm");
    }

    #[test]
    fn a_hand_mirrors_without_a_special_case() {
        let right = Hand::build(0.025, Vec3::X, Vec3::Y, 0.3);
        let left = Hand::build(0.025, -Vec3::X, Vec3::Y, 0.3);
        let (rlo, rhi) = right.mesh().bounds();
        let (llo, lhi) = left.mesh().bounds();
        assert!((rhi.x + llo.x).abs() < 1e-5, "reach did not mirror");
        assert!((rlo.x + lhi.x).abs() < 1e-5, "the wrist end did not mirror");
        assert_eq!(right.digits.len(), left.digits.len());
    }

    #[test]
    fn a_hand_scales_with_the_wrist_it_grows_from() {
        let small = Hand::build(0.02, Vec3::X, Vec3::Y, 0.3);
        let large = Hand::build(0.04, Vec3::X, Vec3::Y, 0.3);
        assert!((large.length / small.length - 2.0).abs() < 1e-4);
    }
}
