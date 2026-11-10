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
//!
//! **A winged body's feet are declared here too** (#422, `BeastFeet::winged`):
//! a glider's paws with a glider's digits, and a bird's or a bat's foot shaped
//! into the ball or sole its drawn toes grow from (`plan::winged_foot`,
//! `extremity::toes`). A beast declares none, and every shape it is given is
//! what it was before there were winged feet.

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::Feet;
use super::beast_head::{Blob, march, round_intersection, smooth_union};
use super::winged_foot::{DrawnToe, WingedFoot};

/// A beast's feet, as its plan declares them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeastFeet {
    /// What every foot is.
    pub kind: Feet,
    /// What a winged body's feet carry beyond a paw (#422): a bird's or a
    /// bat's drawn toes, or a glider's digits. `None` on a beast, whose
    /// skeleton is written as it was before there were winged feet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winged: Option<WingedFoot>,
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

/// One foot's shape, in its own frame: the origin on the ground under the
/// pad's centre, `+Z` toward the toes, `+Y` up and `+X` across.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FootShape {
    /// What the foot is.
    pub(crate) kind: Feet,
    /// The pad's radius: the unit everything here is authored in.
    pub(crate) pad: f32,
    /// The frame's origin, in body space.
    origin: Vec3,
    /// The frame's turn out of body space: a yaw, since a foot stands level.
    turn: Quat,
    /// A winged body's foot (#422), or `None` on a beast's. **Private, and
    /// set only where [`Self::toes`] is laid out for it** (#439,
    /// [`Self::declared`]): the cached toes are this field's, so a foot whose
    /// field changed after it was declared would answer [`Self::horn`] and
    /// [`Self::bare`] from the toes of the foot it was, and a bird's talons
    /// would be painted as coat with no error.
    winged: Option<WingedFoot>,
    /// Whether it ends a fore limb: a glider's fore paw is five-toed.
    fore: bool,
    /// `+1` where the frame's `+X` is the foot's outside, a left foot's, and
    /// `-1` where it is its inside: what places a glider's hallux and a
    /// bird's back toes on the side they belong to.
    lateral: f32,
    /// How far ahead of the pad the foot's second node stands, in metres:
    /// what a drawn foot's ball or sole runs to and its toes are sized by.
    reach: f32,
    /// The toes drawn on it as a part ([`Self::drawn_toes`]), laid out once
    /// when the shape is declared (#430): the coat painter asks after them
    /// at every texel of the foot, and laying them out each time - four or
    /// five toes, a bisection each - made a winged build a tenth slower.
    toes: Vec<DrawnToe>,
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
            winged: None,
            fore: false,
            lateral: if pad_at.x < 0.0 { -1.0 } else { 1.0 },
            reach: Vec2::new(toe_at.x - pad_at.x, toe_at.z - pad_at.z).length(),
            toes: Vec::new(),
        }
    }

    /// The foot `feet` declares on a fore or a hind limb, its pad and toes as
    /// [`Self::of`] takes them.
    pub(crate) fn declared(
        feet: &BeastFeet,
        fore: bool,
        pad_at: Vec3,
        pad: f32,
        toe_at: Vec3,
    ) -> Self {
        let mut shape = Self {
            // A bat's fore "foot" is its wrist's pad and its thumb, the
            // wing's (#421): a paw, as it was, under no drawn toes.
            winged: feet.winged.filter(|winged| !(fore && winged.drawn())),
            fore,
            ..Self::of(feet.kind, pad_at, pad, toe_at)
        };
        shape.toes = shape
            .winged
            .filter(|winged| winged.drawn())
            .map_or_else(Vec::new, |winged| {
                super::winged_foot::drawn(winged, shape.pad, shape.reach, shape.lateral)
            });
        shape
    }

    /// The toes drawn on it as a part, in its frame: a bird's or a bat's
    /// (#422), and none on any other foot. Laid out when it was declared.
    pub(crate) fn drawn_toes(&self) -> &[DrawnToe] {
        debug_assert!(self.laid_out(), "a foot's toes are not its own");
        &self.toes
    }

    /// Whether the cached toes are this foot's (#439): some exactly when it
    /// draws toes as a part. What [`Self::of`] and [`Self::declared`] leave,
    /// and what every reader of the cache checks in a build with assertions.
    fn laid_out(&self) -> bool {
        self.toes.is_empty() != self.winged.is_some_and(WingedFoot::drawn)
    }

    /// What a drawn foot's toes grow from, in its frame: a bird's ball, a
    /// bat's sole - one rounded body from under the pad to the second node.
    fn sole(&self) -> Blob {
        let p = self.pad;
        // A bird's ball is the tarsometatarsus's end and the pad under it,
        // no wider than the leg; a bat's sole is long and narrow.
        let (wide, high, past) = match self.winged {
            Some(WingedFoot::Bat) => (0.72, 0.52, 0.60),
            _ => (0.64, 0.44, 0.40),
        };
        Blob {
            centre: Vec3::new(0.0, 0.48 * p, 0.45 * self.reach),
            radii: Vec3::new(wide * p, high * p, 0.5 * self.reach + past * p),
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
        if let Some(winged) = self.winged {
            let shape = if winged.drawn() {
                // The toes are a part of their own; the body's foot is what
                // they grow from.
                self.sole().distance(at)
            } else {
                self.glider_toes().into_iter().flatten().fold(
                    self.palm().distance(at),
                    |distance, (centre, radius, _)| {
                        smooth_union(distance, at.distance(centre) - radius, 0.06 * p)
                    },
                )
            };
            return round_intersection(shape, -at.y, 0.10 * p);
        }
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
                let mut distance = self.palm().distance(at);
                for (centre, radius) in self.toes() {
                    distance = smooth_union(distance, at.distance(centre) - radius, 0.06 * p);
                }
                distance
            }
        };
        // A flat sole, its edge rounded off.
        round_intersection(shape, -at.y, 0.10 * p)
    }

    /// A paw's palm, in its frame.
    fn palm(&self) -> Blob {
        Blob {
            centre: Vec3::new(0.0, 0.62, 0.10) * self.pad,
            radii: Vec3::new(0.95, 0.74, 1.05) * self.pad,
        }
    }

    /// A glider's digits (#422), each a ball's centre, its radius and
    /// whether a claw shows at its tip, in the foot's frame; `None` past the
    /// last. A fore paw's five, splayed (research-10 3.3: five clawed digits);
    /// a hind paw's hallux, large, set on the inside and turned out and back
    /// as a thumb is, with no claw, the second and third toes fused into one
    /// narrow pair, and the fourth the longest (S31; bg). The shares are
    /// **chosen by render**.
    fn glider_toes(&self) -> [Option<(Vec3, f32, bool)>; 6] {
        let p = self.pad;
        let side = self.lateral;
        let toe = |x: f32, y: f32, z: f32, radius: f32, claw: bool| {
            Some((Vec3::new(x * side, y, z) * p, radius * p, claw))
        };
        if self.fore {
            [
                toe(-0.95, 0.28, 0.40, 0.29, true),
                toe(-0.44, 0.31, 0.92, 0.30, true),
                toe(0.0, 0.32, 1.04, 0.32, true),
                toe(0.44, 0.31, 0.92, 0.30, true),
                toe(0.84, 0.28, 0.44, 0.27, true),
                None,
            ]
        } else {
            [
                // The hallux: a root and a rounded tip, out and back.
                toe(-0.95, 0.34, 0.0, 0.42, false),
                toe(-1.45, 0.32, -0.28, 0.36, false),
                // II and III fused: two narrow toes run together.
                toe(-0.32, 0.30, 0.95, 0.26, true),
                toe(-0.04, 0.30, 1.00, 0.26, true),
                toe(0.32, 0.34, 1.12, 0.36, true),
                toe(0.74, 0.31, 0.84, 0.31, true),
            ]
        }
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
        if self.winged.is_some_and(WingedFoot::drawn) {
            return [
                Vec3::new(0.0, 0.50 * p, -0.25 * p),
                Vec3::new(0.0, 0.45 * p, self.reach + 0.20 * p),
            ];
        }
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

    /// How much of a point of the foot's surface is horn rather than coat,
    /// `0..=1`: the tip of a showing claw, a hoof below its coronet, or
    /// none. In the foot's frame; the coat painter colours it (#368).
    pub(crate) fn horn(&self, at: Vec3) -> f32 {
        debug_assert!(self.laid_out(), "a foot's toes are not its own");
        match self.winged {
            Some(WingedFoot::Glider) => {
                let showing =
                    self.glider_toes()
                        .into_iter()
                        .flatten()
                        .any(|(centre, radius, claw)| {
                            let off = at - centre;
                            claw && off.z > 0.55 * radius && off.y < 0.15 * radius
                        });
                return if showing { 1.0 } else { 0.0 };
            }
            Some(_) => {
                // Past each claw's base, along its curve.
                let talon = self.toes.iter().any(|toe| {
                    let (base, next) = (toe.claw[0], toe.claw[1]);
                    (at - base).dot(next - base) > 0.0
                        && near_run(&toe.claw, at, 1.4 * toe.claw_radius_at(0.0))
                });
                return if talon { 1.0 } else { 0.0 };
            }
            None => {}
        }
        match self.kind {
            // Blended in under the coronet rather than cut at it: a hoof's
            // rim cut at one height reads as a saw across the facets it
            // crosses.
            Feet::Hooves => {
                let rise = at.y / (HOOF * self.pad);
                1.0 - crate::face::smooth((rise - 0.72) / 0.22)
            }
            Feet::Pads => 0.0,
            Feet::Claws => {
                let showing = self.toes().into_iter().any(|(centre, radius)| {
                    let off = at - centre;
                    off.z > 0.55 * radius && off.y < 0.15 * radius
                });
                if showing { 1.0 } else { 0.0 }
            }
        }
    }

    /// Whether a point of the foot, in its frame, is a drawn toe's bare,
    /// scaled skin rather than coat (#422): anywhere along a bird's or a
    /// bat's toes short of their claws, which [`Self::horn`] answers first.
    pub(crate) fn bare(&self, at: Vec3) -> bool {
        debug_assert!(self.laid_out(), "a foot's toes are not its own");
        self.winged.is_some_and(WingedFoot::drawn)
            && (at.y < BARE_BALL * self.pad
                || self.toes.iter().any(|toe| {
                    let reach = 1.7 * toe.radius * (1.0 + toe.pads);
                    near_run(&toe.joints, at, reach)
                }))
    }
}

/// How high a drawn foot's ball or sole is bare skin, in pad radii: the
/// scaled pad a bird's toes meet on, the leg feathered above it.
/// Provenance: **chosen by render** (#422).
const BARE_BALL: f32 = 0.9;

/// Whether `at` lies within `reach` of the run of points `run`.
fn near_run(run: &[Vec3], at: Vec3, reach: f32) -> bool {
    run.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let along = b - a;
        let share =
            ((at - a).dot(along) / along.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
        at.distance(a + along * share) < reach
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::beast_head::gradient;
    use crate::plan::winged_foot::{CLAW_SAMPLES, Toes};

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

    fn winged(winged: WingedFoot, fore: bool, x: f32) -> FootShape {
        let feet = BeastFeet {
            kind: Feet::Claws,
            winged: Some(winged),
        };
        FootShape::declared(
            &feet,
            fore,
            Vec3::new(x, 0.025, 0.4),
            0.03,
            Vec3::new(x, 0.02, 0.45),
        )
    }

    #[test]
    fn a_glider_s_hind_hallux_is_inside_behind_and_clawless_and_its_fore_paw_five_clawed() {
        for x in [0.2, -0.2] {
            let hind = winged(WingedFoot::Glider, false, x);
            let digits: Vec<_> = hind.glider_toes().into_iter().flatten().collect();
            let (hallux, rest) = digits.split_at(2);
            for &(centre, _, claw) in hallux {
                assert!(!claw, "a glider's hallux has no claw");
                assert!(
                    centre.x * hind.lateral < 0.0,
                    "its hallux is not on the inside"
                );
                assert!(
                    rest.iter().all(|&(toe, _, _)| toe.z > centre.z),
                    "its hallux is not behind the other digits"
                );
            }
            let fore = winged(WingedFoot::Glider, true, x);
            let clawed = fore
                .glider_toes()
                .into_iter()
                .flatten()
                .filter(|d| d.2)
                .count();
            assert_eq!(clawed, 5, "a glider's fore paw");
        }
    }

    #[test]
    fn a_drawn_foot_s_talons_are_horn_its_toes_bare_and_a_bat_s_wrist_a_paw() {
        let foot = winged(WingedFoot::Bird(Toes::Anisodactyl), false, 0.2);
        let toes = foot.drawn_toes();
        assert_eq!(toes.len(), 4);
        for toe in toes {
            let tip = toe.claw[CLAW_SAMPLES - 1];
            assert_eq!(foot.horn(tip), 1.0, "toe {}'s talon", toe.digit);
            let middle = (toe.joints[0] + toe.joints[1]) * 0.5;
            assert_eq!(foot.horn(middle), 0.0, "toe {}'s skin", toe.digit);
            assert!(foot.bare(middle), "toe {}'s skin is coat", toe.digit);
        }
        assert!(
            !foot.bare(Vec3::new(0.0, 3.0, 0.0) * foot.pad),
            "the leg is bare"
        );
        let wrist = winged(WingedFoot::Bat, true, 0.2);
        assert_eq!(wrist.winged, None, "a bat's wrist grows drawn toes");
        assert!(wrist.drawn_toes().is_empty());
    }

    #[test]
    fn every_foot_made_has_its_own_toes_laid_out() {
        // #439: the cache is some exactly on a foot that draws toes, however
        // the foot was made.
        for kind in [Feet::Claws, Feet::Pads, Feet::Hooves] {
            assert!(foot(kind).laid_out(), "{kind:?}");
        }
        let feet = [
            WingedFoot::Bird(Toes::Anisodactyl),
            WingedFoot::Bird(Toes::Zygodactyl),
            WingedFoot::Bat,
            WingedFoot::Glider,
        ];
        for which in feet {
            for fore in [false, true] {
                for x in [-0.2, 0.2] {
                    let shape = winged(which, fore, x);
                    assert!(shape.laid_out(), "{which:?}, fore {fore}, at {x}");
                    assert_eq!(
                        shape.drawn_toes().is_empty(),
                        !(which.drawn() && !fore),
                        "{which:?}, fore {fore}, at {x}"
                    );
                }
            }
        }
    }

    /// A foot whose kind was changed after its toes were laid out is caught
    /// where the toes are read (#439), in a build with assertions: before, it
    /// read no toes and answered a bird's talon as coat.
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "a foot's toes are not its own")]
    fn a_foot_changed_after_it_was_declared_is_caught() {
        let mut shape = foot(Feet::Claws);
        shape.winged = Some(WingedFoot::Bird(Toes::Anisodactyl));
        let _ = shape.horn(Vec3::new(0.0, 0.01, 0.05));
    }

    #[test]
    fn only_a_showing_claw_and_a_hoof_are_horn() {
        let tip = |kind: Feet| {
            let foot = foot(kind);
            let (centre, radius) = foot.toes()[1];
            foot.horn(centre + Vec3::new(0.0, -0.1 * radius, 0.9 * radius))
        };
        assert_eq!(tip(Feet::Claws), 1.0);
        assert_eq!(tip(Feet::Pads), 0.0, "a cat's claws are sheathed");
        let hoof = foot(Feet::Hooves);
        assert_eq!(hoof.horn(Vec3::new(0.0, 0.5, 1.0) * hoof.pad), 1.0);
        assert_eq!(
            hoof.horn(Vec3::new(0.0, 1.5, 0.5) * hoof.pad),
            0.0,
            "the pastern is coat"
        );
        let coronet = hoof.horn(Vec3::new(0.0, 0.88, 1.0) * hoof.pad);
        assert!(
            coronet > 0.0 && coronet < 1.0,
            "the coronet blends, at {coronet}"
        );
    }
}
