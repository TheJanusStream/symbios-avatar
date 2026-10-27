//! What a beast's feet are shaped into (#367): a paw on four toes or a hoof.
//!
//! The leg's last two nodes, the pad and the toes, are a blob after
//! subdivision, which carries a beast's weight and says nothing of what it
//! walks on. So each foot is given an analytic shape in its own frame - a
//! palm and four toes on a flat sole, or a wall of horn round one - and
//! `extremity::beast` projects the foot's refined surface onto it, as
//! `face::beast` does the head. The frame comes from the leg's own pad and
//! toe nodes, so a foot is as large as its leg makes it and points where its
//! toes do.
//!
//! The units are the pad's radius: a toe is a third of one, a hoof a pad
//! high.

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::Feet;
use super::beast_head::{Blob, march, round_intersection, smooth_union};

/// A beast's feet, as its plan declares them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeastFeet {
    /// What every foot is.
    pub kind: Feet,
}

/// The toes of a paw, from the `-X` side across: where each sits in pad
/// radii - across, up off the ground, forward of the pad's centre - and its
/// radius. A dog's: splayed in an arc, the middle two longest.
/// Provenance: **tuned by render** (#367).
const TOES: [(Vec3, f32); 4] = [
    (Vec3::new(-0.68, 0.30, 0.80), 0.31),
    (Vec3::new(-0.24, 0.32, 0.98), 0.35),
    (Vec3::new(0.24, 0.32, 0.98), 0.35),
    (Vec3::new(0.68, 0.30, 0.80), 0.31),
];

/// How much shorter and rounder a sheathed paw's toes sit, as a share of a
/// clawed one's reach forward and a gain on their radius: a cat's foot is a
/// compact round one.
const SHEATHED: (f32, f32) = (0.86, 1.08);

/// A hoof's height, in pad radii: the coronet, where the horn ends and the
/// pastern begins.
const HOOF: f32 = 1.05;

/// The colour a claw is drawn in over the coat (#367): dark, until the coat
/// painter says otherwise (#368).
pub(crate) const CLAW: Vec3 = Vec3::new(0.26, 0.23, 0.23);

/// The colour a hoof's horn is drawn in over the coat (#367), as [`CLAW`].
pub(crate) const HORN: Vec3 = Vec3::new(0.36, 0.31, 0.29);

/// One foot's shape, in its own frame: the origin on the ground under the
/// pad's centre, `+Z` toward the toes, `+Y` up and `+X` across.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FootShape {
    /// What the foot is.
    pub(crate) kind: Feet,
    /// The pad's radius: the unit everything here is authored in.
    pub(crate) pad: f32,
    /// The frame's origin, in body space.
    origin: Vec3,
    /// The frame's turn out of body space: a yaw, since a foot stands level.
    turn: Quat,
}

impl FootShape {
    /// The foot whose pad node sits at `pad_at` with radius `pad`, its toes
    /// toward `toe_at`, all at rest in body space.
    pub(crate) fn of(kind: Feet, pad_at: Vec3, pad: f32, toe_at: Vec3) -> Self {
        let ahead = Vec3::new(toe_at.x - pad_at.x, 0.0, toe_at.z - pad_at.z);
        let ahead = ahead.normalize_or(Vec3::Z);
        Self {
            kind,
            pad,
            origin: Vec3::new(pad_at.x, 0.0, pad_at.z),
            turn: Quat::from_rotation_arc(Vec3::Z, ahead),
        }
    }

    /// A body-space point, in the foot's frame.
    pub(crate) fn local(&self, body: Vec3) -> Vec3 {
        self.turn.inverse() * (body - self.origin)
    }

    /// A point in the foot's frame, in body space.
    pub(crate) fn body(&self, local: Vec3) -> Vec3 {
        self.origin + self.turn * local
    }

    /// How far a point in the foot's frame is from its surface, negative
    /// inside: a bound, as the head's is.
    pub(crate) fn distance(&self, at: Vec3) -> f32 {
        let p = self.pad;
        let shape = match self.kind {
            Feet::Hooves => {
                // A wall of horn: round, a little longer than wide, wider at
                // the ground than at the coronet, and its toe set forward at
                // the bottom - the slope a hoof's front wall stands at. Open
                // above the coronet, where the pastern carries on.
                let rise = (at.y / (HOOF * p)).clamp(0.0, 1.0);
                let radius = p * (1.08 + (0.88 - 1.08) * rise);
                let toe = 0.22 * p * (1.0 - rise);
                Vec2::new(at.x, (at.z - toe) / 1.12).length() - radius
            }
            Feet::Claws | Feet::Pads => {
                // Both the palm and the toes reach a little below the
                // ground, so the sole that cuts them is flat across.
                let palm = Blob {
                    centre: Vec3::new(0.0, 0.62, 0.10) * p,
                    radii: Vec3::new(0.95, 0.74, 1.05) * p,
                };
                let mut distance = palm.distance(at);
                for (centre, radius) in self.toes() {
                    distance = smooth_union(distance, at.distance(centre) - radius, 0.06 * p);
                }
                distance
            }
        };
        // A flat sole, its edge rounded off.
        round_intersection(shape, -at.y, 0.10 * p)
    }

    /// Each toe's centre and radius, in the foot's frame.
    fn toes(&self) -> [(Vec3, f32); 4] {
        let (reach, girth) = match self.kind {
            Feet::Pads => SHEATHED,
            _ => (1.0, 1.0),
        };
        TOES.map(|(centre, radius)| {
            (
                Vec3::new(centre.x, centre.y, centre.z * reach) * self.pad,
                radius * girth * self.pad,
            )
        })
    }

    /// The straight line the surface is projected from, back to front: along
    /// a paw from its heel to its toes, up a hoof from its sole.
    fn axis(&self) -> [Vec3; 2] {
        let p = self.pad;
        match self.kind {
            Feet::Hooves => [
                Vec3::new(0.0, 0.30, 0.15) * p,
                Vec3::new(0.0, 1.50, 0.0) * p,
            ],
            Feet::Claws | Feet::Pads => [
                Vec3::new(0.0, 0.50, -0.35) * p,
                Vec3::new(0.0, 0.40, 0.75) * p,
            ],
        }
    }

    /// Where the foot's surface is, seen from its axis through `at`, in its
    /// frame: the head's projection, on the foot's own shape.
    pub(crate) fn surface_through(&self, at: Vec3) -> Option<Vec3> {
        let [back, front] = self.axis();
        let along = front - back;
        let share =
            ((at - back).dot(along) / along.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
        let anchor = back + along * share;
        march(
            |at| self.distance(at),
            anchor,
            (at - anchor).try_normalize()?,
            self.pad,
        )
    }

    /// The colour a point of the foot's surface is drawn in over its coat:
    /// the tip of a showing claw, a hoof's horn below its coronet, or
    /// nothing. In the foot's frame.
    pub(crate) fn tint(&self, at: Vec3) -> Option<Vec3> {
        match self.kind {
            // Blended in under the coronet rather than cut at it: a colour
            // that changes at one height steps across every facet the
            // height crosses, and a hoof's rim reads as a saw.
            Feet::Hooves => {
                let rise = at.y / (HOOF * self.pad);
                let horn = 1.0 - crate::face::smooth((rise - 0.72) / 0.22);
                (horn > 0.0).then(|| Vec3::ONE.lerp(HORN, horn))
            }
            Feet::Pads => None,
            Feet::Claws => self
                .toes()
                .into_iter()
                .any(|(centre, radius)| {
                    let off = at - centre;
                    off.z > 0.55 * radius && off.y < 0.15 * radius
                })
                .then_some(CLAW),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::beast_head::gradient;

    fn foot(kind: Feet) -> FootShape {
        FootShape::of(
            kind,
            Vec3::new(0.2, 0.025, 0.4),
            0.03,
            Vec3::new(0.2, 0.02, 0.45),
        )
    }

    #[test]
    fn every_foot_stands_on_a_flat_sole() {
        for kind in Feet::ALL {
            let foot = foot(kind);
            // Down through the middle of the sole, the surface is the ground,
            // and the ground faces straight down.
            let sole = foot
                .surface_through(Vec3::new(0.0, -1.0, 0.1) * foot.pad)
                .expect("a ray down lands");
            assert!(
                sole.y.abs() < 0.02 * foot.pad,
                "{kind:?}: the sole at {}",
                sole.y
            );
            let normal = gradient(|at| foot.distance(at), sole, foot.pad);
            assert!(normal.y < -0.95, "{kind:?}: the sole faces {normal}");
        }
    }

    #[test]
    fn a_paw_has_toes_and_a_hoof_does_not() {
        // Across the front, a paw dips between its middle toes and a hoof's
        // wall runs smooth.
        // How far forward the foot reaches at a height and a place across it,
        // marched straight along `+Z` from over the pad.
        let front = |kind: Feet, across: f32| {
            let foot = foot(kind);
            let p = foot.pad;
            let reach = march(
                |at| foot.distance(at),
                Vec3::new(across * p, 0.30 * p, 0.0),
                Vec3::Z,
                p,
            )
            .expect("a march forward lands");
            reach.z / p
        };
        for kind in [Feet::Claws, Feet::Pads] {
            assert!(
                front(kind, 0.24) > front(kind, 0.0) + 0.03,
                "{kind:?} has no toes"
            );
        }
        assert!(
            front(Feet::Hooves, 0.24) <= front(Feet::Hooves, 0.0),
            "a hoof is smooth"
        );
    }

    #[test]
    fn a_foot_turns_with_its_toes_and_keeps_its_frame() {
        let foot = FootShape::of(Feet::Claws, Vec3::ZERO, 0.03, Vec3::new(1.0, 0.0, 0.0));
        assert!(
            foot.body(Vec3::Z).abs_diff_eq(Vec3::X, 1.0e-5),
            "its toes point down +X"
        );
        let at = Vec3::new(0.1, 0.2, -0.3);
        assert!(foot.local(foot.body(at)).abs_diff_eq(at, 1.0e-5));
    }

    #[test]
    fn only_a_showing_claw_and_a_hoof_are_tinted() {
        let tip = |kind: Feet| {
            let foot = foot(kind);
            let (centre, radius) = foot.toes()[1];
            foot.tint(centre + Vec3::new(0.0, -0.1 * radius, 0.9 * radius))
        };
        assert_eq!(tip(Feet::Claws), Some(CLAW));
        assert_eq!(tip(Feet::Pads), None, "a cat's claws are sheathed");
        let hoof = foot(Feet::Hooves);
        assert_eq!(hoof.tint(Vec3::new(0.0, 0.5, 1.0) * hoof.pad), Some(HORN));
        assert_eq!(
            hoof.tint(Vec3::new(0.0, 1.5, 0.5) * hoof.pad),
            None,
            "the pastern is coat"
        );
        let coronet = hoof
            .tint(Vec3::new(0.0, 0.88, 1.0) * hoof.pad)
            .expect("horn, fading");
        assert!(
            coronet.x > HORN.x && coronet.x < 1.0,
            "the coronet blends, at {coronet}"
        );
    }
}
