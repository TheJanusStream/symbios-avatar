//! Reference skeletons for tests, examples, and downstream smoke tests.
//!
//! These are hand-laid-out graphs, not the parametric body-plan layer that will
//! eventually generate skeletons from an avatar record. They exist so the
//! mesher can be exercised against the shapes it actually has to survive: a
//! coplanar three-way joint (the pelvis), a four-way joint (the chest), and a
//! quadruped whose girdles carry two legs each.
//!
//! Dimensions are metres, `+Y` up, `+Z` forward — the convention the whole
//! crate and the eventual VRM 1.0 export share.

use glam::Vec3;

use crate::skeleton::{Node, Skeleton};

/// An upright biped in T-pose, roughly 1.75 m tall.
///
/// Exercises a coplanar pelvis (spine plus two legs, all in the sagittal plane)
/// and a four-way chest joint. The rest pose is a T deliberately: VRM 1.0
/// requires it of exported humanoids, so the reference body is authored the way
/// it will have to be baked.
#[must_use]
pub fn humanoid() -> Skeleton {
    let mut skeleton = Skeleton::new();

    let pelvis = skeleton.add_node(Node::new(Vec3::new(0.0, 0.95, 0.0), 0.115));
    let waist = skeleton.extend_from(pelvis, Node::new(Vec3::new(0.0, 1.12, 0.0), 0.125));
    let chest = skeleton.extend_from(waist, Node::new(Vec3::new(0.0, 1.32, 0.0), 0.135));
    let neck = skeleton.extend_from(chest, Node::new(Vec3::new(0.0, 1.47, 0.0), 0.06));
    skeleton.extend_from(neck, Node::new(Vec3::new(0.0, 1.62, 0.0), 0.10));

    for side in [-1.0f32, 1.0] {
        // Arms leave the chest almost horizontally. Angling them up would crowd
        // the neck: three sockets sharing the top of a wide chest is the
        // tightest fan on the body, and the clavicle is what buys the room — it
        // has to be long enough for the arm socket to clear the waist ring's
        // corners, which is why it reaches past the shoulder line.
        let clavicle =
            skeleton.extend_from(chest, Node::new(Vec3::new(side * 0.21, 1.33, 0.0), 0.055));
        let shoulder =
            skeleton.extend_from(clavicle, Node::new(Vec3::new(side * 0.30, 1.33, 0.0), 0.05));
        let elbow = skeleton.extend_from(
            shoulder,
            Node::new(Vec3::new(side * 0.48, 1.33, 0.0), 0.042),
        );
        let wrist =
            skeleton.extend_from(elbow, Node::new(Vec3::new(side * 0.64, 1.33, 0.0), 0.033));
        skeleton.extend_from(wrist, Node::new(Vec3::new(side * 0.71, 1.33, 0.0), 0.038));

        // Legs spread far enough that the two hip sockets stay distinct.
        let hip = skeleton.extend_from(pelvis, Node::new(Vec3::new(side * 0.17, 0.74, 0.0), 0.075));
        let knee = skeleton.extend_from(hip, Node::new(Vec3::new(side * 0.17, 0.48, 0.0), 0.06));
        let ankle = skeleton.extend_from(knee, Node::new(Vec3::new(side * 0.17, 0.12, 0.0), 0.042));
        skeleton.extend_from(ankle, Node::new(Vec3::new(side * 0.17, 0.045, 0.10), 0.045));
    }

    skeleton
}

/// A four-legged creature with a tail, on the same engine as [`humanoid`].
///
/// Both girdles are four-way joints carrying a spine segment in each direction
/// plus two legs, which is the topology a quadruped shares with an insect or a
/// dragon — only the leg count changes.
#[must_use]
pub fn quadruped() -> Skeleton {
    let mut skeleton = Skeleton::new();

    let tail_tip = skeleton.add_node(Node::new(Vec3::new(0.0, 0.42, -0.86), 0.018));
    let tail = skeleton.extend_from(tail_tip, Node::new(Vec3::new(0.0, 0.50, -0.62), 0.032));
    let hips = skeleton.extend_from(tail, Node::new(Vec3::new(0.0, 0.56, -0.34), 0.105));
    let spine = skeleton.extend_from(hips, Node::new(Vec3::new(0.0, 0.58, -0.08), 0.115));
    let withers = skeleton.extend_from(spine, Node::new(Vec3::new(0.0, 0.58, 0.20), 0.11));
    let neck = skeleton.extend_from(withers, Node::new(Vec3::new(0.0, 0.63, 0.42), 0.06));
    skeleton.extend_from(neck, Node::new(Vec3::new(0.0, 0.62, 0.58), 0.075));

    for side in [-1.0f32, 1.0] {
        let stifle =
            skeleton.extend_from(hips, Node::new(Vec3::new(side * 0.16, 0.36, -0.35), 0.045));
        let hock = skeleton.extend_from(
            stifle,
            Node::new(Vec3::new(side * 0.16, 0.16, -0.30), 0.032),
        );
        skeleton.extend_from(hock, Node::new(Vec3::new(side * 0.16, 0.05, -0.26), 0.036));

        let knee = skeleton.extend_from(
            withers,
            Node::new(Vec3::new(side * 0.15, 0.36, 0.20), 0.045),
        );
        let fetlock =
            skeleton.extend_from(knee, Node::new(Vec3::new(side * 0.15, 0.16, 0.21), 0.032));
        skeleton.extend_from(
            fetlock,
            Node::new(Vec3::new(side * 0.15, 0.05, 0.25), 0.036),
        );
    }

    skeleton
}

/// Three limbs meeting at one non-coplanar joint — the smallest real joint.
#[must_use]
pub fn tripod() -> Skeleton {
    let mut skeleton = Skeleton::new();
    let hub = skeleton.add_node(Node::new(Vec3::ZERO, 0.2));
    for direction in [
        Vec3::Y,
        Vec3::new(0.8, -0.5, 0.0),
        Vec3::new(-0.3, -0.4, 0.8),
    ] {
        let tip = direction.normalize() * 0.9;
        skeleton.extend_from(hub, Node::new(tip, 0.12));
    }
    skeleton
}

/// Three limbs meeting in one plane — the degenerate joint that needs apexes.
#[must_use]
pub fn flat_tripod() -> Skeleton {
    let mut skeleton = Skeleton::new();
    let hub = skeleton.add_node(Node::new(Vec3::ZERO, 0.2));
    for direction in [
        Vec3::Y,
        Vec3::new(0.9, -0.6, 0.0),
        Vec3::new(-0.9, -0.6, 0.0),
    ] {
        let tip = direction.normalize() * 0.9;
        skeleton.extend_from(hub, Node::new(tip, 0.12));
    }
    skeleton
}

/// A straight chain of `segments` bones — a limb with no joints at all.
#[must_use]
pub fn chain(segments: usize) -> Skeleton {
    let mut skeleton = Skeleton::new();
    let mut previous = skeleton.add_node(Node::new(Vec3::ZERO, 0.15));
    for step in 1..=segments.max(1) {
        previous = skeleton.extend_from(
            previous,
            Node::new(Vec3::new(0.0, step as f32 * 0.4, 0.0), 0.15),
        );
    }
    skeleton
}
