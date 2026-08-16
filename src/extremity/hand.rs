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

/// How a digit's length divides between its phalanges, proximal first.
///
/// **Three, because a finger has three, and this is a rig as well as a
/// shape.** Four equal segments chosen for how smooth the curl looked would
/// not do: the reference divides every finger into three unequal ones, and
/// its bones sit on those divisions. Measured off the
/// Quaternius male, as fractions of each digit's own length:
///
/// ```text
///   index   0.419  0.322  0.260
///   middle  0.403  0.304  0.294
///   ring    0.394  0.306  0.300
///   pinky   0.390  0.311  0.299
///   thumb   0.362  0.367  0.271
/// ```
///
/// The four fingers agree closely enough to share one set; the thumb does not,
/// because its two long segments are nearly equal where a finger's taper.
///
/// Provenance: **looked up**, from the reference's own bone positions.
const PHALANGES: [f32; 3] = [0.402, 0.311, 0.287];
/// See [`PHALANGES`]. The thumb's own division, which is not a finger's.
const THUMB_PHALANGES: [f32; 3] = [0.362, 0.367, 0.271];

/// How many segments each digit is swept from, and so how many bones it has.
///
/// One per phalanx, so a station of the sweep and a joint of the rig are the
/// same thing and cannot drift apart.
pub const JOINTS: usize = PHALANGES.len();

/// Bones per hand: the wrist, then one per phalanx of each of five digits.
///
/// Twenty-one, which is what both reference hands carry — `hand_l`, then
/// `<digit>_01`, `_02`, `_03` and `_04_leaf` for thumb, index, middle, ring and
/// pinky. [`Hand::influences`] indexes into this order.
pub const BONES: usize = 1 + 5 * (JOINTS + 1);

/// Faces around a digit.
const DIGIT_SIDES: usize = 6;

/// Faces around the palm.
const PALM_SIDES: usize = 8;

/// Where each finger sits across the palm, how far out its knuckle stands, and
/// how long it is — all relative to the middle finger.
///
/// **The knuckles are not in a line.** The
/// reference's four sit at 10.98, 10.91, 10.07 and 9.06 cm out from the wrist,
/// so the little finger's is nearly two centimetres nearer in than the index's,
/// and the row curves. Built on one straight line the four roots read as a
/// comb's spine however well the fingers themselves are proportioned.
///
/// Reach from the wrist, which is what the eye actually reads, comes out at
/// 0.939, 1.000, 0.930 and 0.829 of the middle finger's on the reference.
///
/// Provenance: the offsets across the palm are **tuned by render** — the
/// constraint that decided them is that neighbouring fingers must touch, which
/// is a rendering observation. The knuckle set-back and the lengths are
/// **looked up**.
const FINGERS: [Finger; 4] = [
    Finger {
        across: -0.75,
        knuckle: 1.000,
        length: 0.869,
    },
    Finger {
        across: -0.25,
        knuckle: 0.994,
        length: 1.000,
    },
    Finger {
        across: 0.25,
        knuckle: 0.917,
        length: 0.936,
    },
    Finger {
        across: 0.75,
        knuckle: 0.825,
        length: 0.829,
    },
];

/// One finger's placement, in the units [`FINGERS`] documents.
struct Finger {
    /// Across the palm, in half-palm-widths from the middle.
    across: f32,
    /// How far out the knuckle stands, of the furthest knuckle's set-back.
    knuckle: f32,
    /// Length, of the middle finger's.
    length: f32,
}

/// A finger's radius, as a share of the palm's half-width.
///
/// Four of them side by side fill the palm, which is what sets this: a hand is
/// as wide as its fingers are, not wider.
/// Provenance: **derived** — four fingers side by side fill the palm, so
/// this is one quarter by construction.
const FINGER_RADIUS: f32 = 0.25;

/// One digit: its surface, and the chain of joints that bends it.
#[derive(Clone, Debug, PartialEq)]
pub struct Digit {
    /// The swept solid, in wrist-local space.
    pub mesh: PolyMesh,
    /// Knuckle, then one joint per phalanx boundary, ending at the tip.
    ///
    /// [`JOINTS`] + 1 of them, and they are the *stations of the sweep* rather
    /// than a second set of numbers that happens to agree with it — so a bone
    /// cannot drift away from the surface it bends. The last is a leaf, as the
    /// reference's `_04_leaf` is: it carries no geometry past it and exists to
    /// give the final phalanx a direction.
    pub joints: Vec<Vec3>,
}

/// A built hand, in wrist-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Hand {
    /// The palm slab.
    pub palm: PolyMesh,
    /// Four fingers and a thumb.
    pub digits: Vec<Digit>,
    /// How long the hand is from wrist to fingertip, in metres.
    pub length: f32,
}

impl Hand {
    /// Builds a hand for a wrist of the given measured radius.
    ///
    /// `out` points along the forearm, away from the body; `up` is the back of
    /// the hand.
    ///
    /// **This builds one chirality, and handing it the other arm's direction
    /// does not produce the other hand.** A hand is chiral — the thumb is
    /// on one particular edge of the palm and no rotation moves it to the other
    /// — and everything here is derived from a frame that turns with `out`:
    /// `across` is `out × up`, and the thumb is seated at `-across`. Feed it a
    /// reversed `out` and the whole construction rotates half a turn, thumb
    /// included, so what comes back is the *same* hand pointing the other way.
    /// Measured on the built body, our two hands mirrored onto each other to
    /// 5.5 mm mean and 33.4 mm worst error, against 0.000 mm for both Quaternius
    /// references, whose entire mesh is one side reflected.
    ///
    /// A bounds test cannot catch this: comparing the two hands' *x* bounds
    /// is satisfied exactly as well by a half-turn as by a reflection. See
    /// [`crate::extremity`] for how the other hand is actually made.
    ///
    /// Which chirality comes out follows from `across = out × up`: with `up`
    /// along world Y, a hand built with `out.x` negative seats its thumb toward
    /// `+Z`, which is the body's front, and that is where both of the
    /// reference's thumbs are.
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
        for finger in &FINGERS {
            digits.push(digit(
                // The knuckles curve: the little finger's stands nearly two
                // centimetres nearer the wrist than the index's on the
                // reference, and a straight row of four reads as a comb.
                out * (palm_length * 0.94 * finger.knuckle) + across * (finger.across * palm_width),
                out,
                up,
                across,
                finger_length * finger.length,
                palm_width * FINGER_RADIUS,
                &PHALANGES,
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
            &THUMB_PHALANGES,
            curl * 0.55,
        ));

        Self {
            palm,
            digits,
            length: palm_length + finger_length,
        }
    }

    /// Palm and digits as one mesh.
    ///
    /// Palm first, then the digits in the order [`Hand::digits`] holds them,
    /// which is the order [`Hand::influences`] assumes. The two walk the same
    /// list, so a vertex and its weights cannot fall out of step.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let mut mesh = self.palm.clone();
        for digit in &self.digits {
            mesh.append(&digit.mesh);
        }
        mesh
    }

    /// Which of the hand's own bones holds each vertex of [`Hand::mesh`].
    ///
    /// Bones are numbered as the reference names them: `0` is the wrist, and
    /// digit `d`'s phalanx `j` is `1 + d * (JOINTS + 1) + j`, for [`BONES`] in
    /// all. A caller that has attached those bones to a rig maps the numbers
    /// through its own list; nothing here needs to know what a rig is.
    ///
    /// **Weights are shared across every joint, not assigned to the nearest
    /// one.** A digit is four rings and they sit exactly *on* the joints, so
    /// binding each ring to one bone would hinge the surface at the only places
    /// it has any geometry, and a curling finger would come out as three rigid
    /// tubes. Each ring is split evenly between the phalanx before it and the
    /// phalanx after, which is what makes the fold a curve. The rule for which
    /// bone owns which phalanx is the crate's own: the joint at a bone's
    /// **proximal** end turns it (see [`crate::rig::skin`]), so the knuckle
    /// drives the first phalanx and the tip drives nothing.
    ///
    /// The palm is the wrist's, whole. It does not deform.
    #[must_use]
    pub fn influences(&self) -> Vec<[(usize, f32); 2]> {
        const WRIST: usize = 0;
        let mut out = vec![[(WRIST, 1.0), (WRIST, 0.0)]; self.palm.vertex_count()];

        for (index, digit) in self.digits.iter().enumerate() {
            let base = 1 + index * (JOINTS + 1);
            for &point in &digit.mesh.positions {
                let along = digit.station(point);
                let bone = (along.floor() as usize).min(JOINTS - 1);
                let across = along - bone as f32;
                // Half of a ring's hold goes to the phalanx it starts and half
                // to the one it ends, so the two share the fold between them.
                // The first ring shares with the wrist, which is what lets a
                // knuckle bend at all.
                out.push(if across < 0.5 {
                    let before = if bone == 0 { WRIST } else { base + bone - 1 };
                    [(before, 0.5 - across), (base + bone, 0.5 + across)]
                } else {
                    [(base + bone, 1.5 - across), (base + bone + 1, across - 0.5)]
                });
            }
        }
        out
    }
}

impl Digit {
    /// Where along the digit's chain a point sits, in phalanges from the
    /// knuckle.
    ///
    /// Runs `0.0` at the knuckle to [`JOINTS`] at the tip. Found by projecting
    /// onto each segment in turn and keeping the nearest, rather than by
    /// distance to the joints themselves: a fat ring around a short phalanx is
    /// nearer the *next* joint than its own, and reading it that way would bind
    /// the base of a finger to its fingertip.
    #[must_use]
    pub fn station(&self, point: Vec3) -> f32 {
        let mut best = (f32::MAX, 0.0f32);
        for (index, pair) in self.joints.windows(2).enumerate() {
            let axis = pair[1] - pair[0];
            let span = axis.length_squared();
            let along = if span <= f32::EPSILON {
                0.0
            } else {
                ((point - pair[0]).dot(axis) / span).clamp(0.0, 1.0)
            };
            let off = point.distance(pair[0] + axis * along);
            if off < best.0 {
                best = (off, index as f32 + along);
            }
        }
        best.1
    }
}

/// One finger or thumb, swept along a curling path.
///
/// `phalanges` divides `length` between the segments, proximal first, so the
/// stations of the sweep land on the joints of a real digit rather than at
/// equal intervals.
#[allow(clippy::too_many_arguments)]
fn digit(
    root: Vec3,
    along: Vec3,
    up: Vec3,
    across: Vec3,
    length: f32,
    radius: f32,
    phalanges: &[f32; JOINTS],
    curl: f32,
) -> Digit {
    // Curl bends the digit toward the palm, a little more at each joint, the way
    // a finger closes from the tip inward.
    let bend = curl * 0.55;

    let mut path = Vec::with_capacity(JOINTS + 1);
    let mut at = root;
    let mut direction = along;
    path.push(at);
    for (joint, share) in phalanges.iter().enumerate() {
        let turn = bend * (joint as f32 + 1.0) / JOINTS as f32;
        direction = (direction * turn.cos() - up * turn.sin()).normalize_or(direction);
        at += direction * (length * share);
        path.push(at);
    }

    // Tapering by how far along the digit each station actually is, not by which
    // number it is: the phalanges are of unequal length, so counting stations
    // would step the taper unevenly and pinch the shortest one.
    let mut travelled = 0.0;
    let sections: Vec<Vec2> = std::iter::once(0.0)
        .chain(phalanges.iter().map(|share| {
            travelled += share;
            travelled
        }))
        .map(|reached| {
            // Tapering, but not to a point: a fingertip is rounded, and a cone
            // reads as a claw.
            let taper = 1.0 - 0.32 * reached;
            Vec2::new(radius * taper, radius * taper * 0.88)
        })
        .collect();

    Digit {
        mesh: prim::sweep(&path, &sections, DIGIT_SIDES, across),
        joints: path,
    }
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
                digit.mesh.is_closed_manifold(),
                "digit {index}: {:?}",
                digit.mesh.manifold_report()
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
            .map(|digit| digit.mesh.bounds().1.x)
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
        let thumb = hand.digits[4].mesh.bounds();
        let fingers = hand.digits[1].mesh.bounds();
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
    fn reversing_the_arm_rotates_the_hand_rather_than_reflecting_it() {
        // The trap this test used to fall into (#113). It asserted that the two
        // builds agreed on their *x* bounds and called that mirroring — which a
        // half-turn about Y satisfies exactly as well as a reflection does, so
        // the check passed on a body wearing two right hands.
        //
        // What separates the two is the THUMB, because that is the only part of
        // a hand that knows which one it is. Under a reflection across the
        // sagittal plane the thumb keeps its `z`; under the half-turn this
        // actually performs, `z` flips with everything else.
        let one = Hand::build(0.025, Vec3::X, Vec3::Y, 0.3);
        let other = Hand::build(0.025, -Vec3::X, Vec3::Y, 0.3);
        let thumb = |hand: &Hand| hand.digits[4].mesh.bounds();

        let (rlo, rhi) = one.mesh().bounds();
        let (llo, lhi) = other.mesh().bounds();
        assert!((rhi.x + llo.x).abs() < 1e-5, "reach did not turn about Y");
        assert!((rlo.x + lhi.x).abs() < 1e-5, "the wrist end did not turn");
        assert_eq!(one.digits.len(), other.digits.len());

        // And here is the defect, stated so it cannot come back: the thumbs end
        // up on opposite sides of the body's fore-aft axis, which is two of the
        // same hand.
        assert!(
            thumb(&one).1.z * thumb(&other).0.z < 0.0,
            "the thumbs sat on the same side of z, so this is a reflection after \
             all and the caller no longer needs to make one: {:?} against {:?}",
            thumb(&one),
            thumb(&other)
        );
    }

    #[test]
    fn a_hand_built_along_negative_x_puts_its_thumb_forward() {
        // Which chirality `build` makes, pinned to the one fact that decides it:
        // `across` is `out × up`, the thumb sits at `-across`, so with `up` on
        // world Y an `out` with negative x seats the thumb toward +Z. That is
        // the body's front, and it is where both of the reference's thumbs are —
        // `thumb_01` at z −0.029 running out to `thumb_04_leaf` at z +0.033, on
        // the left hand and the right alike.
        let hand = Hand::build(0.025, -Vec3::X, Vec3::Y, 0.0);
        let thumb = hand.digits[4].mesh.bounds();
        let fingers = hand.digits[1].mesh.bounds();
        assert!(
            thumb.1.z > fingers.1.z,
            "the thumb reached to z {} against the middle finger's {}",
            thumb.1.z,
            fingers.1.z
        );
    }

    #[test]
    fn a_hand_scales_with_the_wrist_it_grows_from() {
        let small = Hand::build(0.02, Vec3::X, Vec3::Y, 0.3);
        let large = Hand::build(0.04, Vec3::X, Vec3::Y, 0.3);
        assert!((large.length / small.length - 2.0).abs() < 1e-4);
    }
}
