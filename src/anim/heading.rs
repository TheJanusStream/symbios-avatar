//! Which way a body travels, against the way it faces.
//!
//! A third input beside [`super::Speed`] and [`super::Turn`], and separate from
//! both on purpose. Speed says how fast, the turn says how the facing changes,
//! and this says where the travel goes relative to that facing — so **a
//! diagonal is a heading rather than a mode**. Walking forward-and-left is one
//! stride description at 45 degrees, not a forward walk crossfaded with a
//! sideways one, and there is nothing to pop when a body swings from one to the
//! other because there is no boundary to cross.
//!
//! # What a heading changes, and what it does not
//!
//! The arc [`super::gait::contact_offset`] walks every contact around was built
//! around [`super::Stride::direction`], so the *geometry* of
//! travelling sideways or backwards already works — a foot planted on the floor
//! stays planted whichever way the body is going over it. What a heading adds
//! is the three things that are not geometry:
//!
//! * **How far the body can step that way**, which is [`Heading::reach`]. A
//!   backward step is shorter than a forward one and a sideways step is shorter
//!   still, and neither is a preference.
//! * **Which end of the foot lands.** Forward walking lands on the heel and
//!   leaves from the toe; backwards it is exactly the other way, and sideways
//!   the sole comes down flat. All three fall out of scaling the roll by the
//!   heading's forward component — see [`Heading::along`].
//! * **How much the arms swing.** The arm swing answers the legs' fore-and-aft
//!   excursion, because that is what it counterbalances; a body shuffling
//!   sideways has none and swings its arms not at all.
//!
//! # The shuffle does NOT come free, and the test is what said so
//!
//! A lateral step sequence wants a shuffle rather than a crossover, on the
//! grounds that a crossover risks the feet intersecting. It is tempting to
//! claim that is not a choice available to get wrong —
//! [`super::gait::contact_offset`] moves every contact by the same screw, so
//! surely two feet keep the separation they start with.
//!
//! **They do not, and the guard written to confirm the claim refutes it
//! instead.** The two feet are at different points of the cycle: one is sliding
//! back through its stance while the other is swinging forward, so their
//! offsets differ by most of a stride's length rather than by nothing. Strafing
//! left on the default body, the left foot ends up 72 mm to the RIGHT of the
//! right one — a crossover, arrived at by accident, and exactly the
//! self-intersection the shuffle exists to avoid.
//!
//! So a sideways stride carries a second bound that a forward one does not, and
//! it is geometric rather than anatomical: **the feet have to stay apart**. On
//! the default body that bound is 88 mm against an anatomical reach of 298, so
//! it is very much the one that binds — which is why it cannot live in
//! [`Heading::reach`], a pure function of an angle. It arrives through
//! [`Heading::reach_within`], which [`super::Stride::toward`] calls with the
//! body's own stance, and it goes into the ellipse's **semi-axis** rather than
//! being applied over the top of it. Clipping afterwards was tried and pops;
//! the axis keeps the ellipse smooth.

use glam::Vec3;

/// How far a body steps backwards, against how far it steps forwards.
///
/// **Three quarters, and it is anatomical rather than derived** — which is
/// worth saying plainly, because most of this crate's constants are
/// consequences and this one is not. A stride is bounded at the back by how far
/// the hip extends and at the front by how far it flexes, and those are not the
/// same range: a hip flexes to about 120 degrees and extends to about 20 or 30.
/// Walking backwards puts the reaching leg on the extension side, so the step
/// is shorter, and the gait literature reports backward steps 20 to 30 percent
/// down on forward ones.
///
/// It would be a consequence if the rig carried joint limits, and the day it
/// does this should be deleted rather than retuned: the number wanted is
/// whatever that body's own hip allows.
pub const BACKWARD_REACH: f32 = 0.75;

/// How far a body steps sideways, against how far it steps forwards.
///
/// **Three fifths, on the same argument and more of it.** Abduction — the leg
/// swinging out from under the body — runs to about 45 degrees where flexion
/// runs to 120, so a sideways step is the shortest of the three. Also
/// anatomical, also a consequence waiting for joint limits.
pub const LATERAL_REACH: f32 = 0.6;

/// Which way a body is travelling, relative to the way it faces.
///
/// Zero is straight ahead. Positive turns toward the body's **left**, which is
/// `+X` — the same sense [`super::Turn`] uses, so a body strafing left and a
/// body turning left agree about which way that is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Heading {
    angle: f32,
}

impl Heading {
    /// Straight ahead.
    pub const FORWARD: Self = Self { angle: 0.0 };
    /// Straight back.
    pub const BACKWARD: Self = Self {
        angle: std::f32::consts::PI,
    };
    /// A strafe to the body's left.
    pub const LEFT: Self = Self {
        angle: std::f32::consts::FRAC_PI_2,
    };
    /// A strafe to the body's right.
    pub const RIGHT: Self = Self {
        angle: -std::f32::consts::FRAC_PI_2,
    };

    /// From an angle off forward, in radians, positive toward the body's left.
    #[must_use]
    pub fn new(radians: f32) -> Self {
        Self { angle: radians }
    }

    /// From an angle off forward, in degrees.
    #[must_use]
    pub fn degrees(degrees: f32) -> Self {
        Self::new(degrees.to_radians())
    }

    /// From a direction in body space. The vertical is ignored.
    ///
    /// A direction of no length is forward, which is the answer that keeps a
    /// caller's zero vector from becoming a body facing nowhere.
    #[must_use]
    pub fn toward(direction: Vec3) -> Self {
        if direction.x == 0.0 && direction.z == 0.0 {
            return Self::FORWARD;
        }
        Self::new(direction.x.atan2(direction.z))
    }

    /// The angle off forward, in radians.
    #[must_use]
    pub fn angle(self) -> f32 {
        self.angle
    }

    /// The direction of travel in body space, as a unit vector.
    #[must_use]
    pub fn direction(self) -> Vec3 {
        Vec3::new(self.angle.sin(), 0.0, self.angle.cos())
    }

    /// How much of the travel is fore-and-aft: `+1` forward, `-1` backwards,
    /// zero sideways.
    ///
    /// **The one number three separate layers answer to**, which is why it is
    /// named rather than each of them writing `cos`. The foot's roll is scaled
    /// by it, so a backward step lands toe-first and a sideways one lands flat;
    /// the arm swing is scaled by it, so a shuffle swings the arms not at all;
    /// and the trunk's lean rides along the direction rather than along `+Z`.
    /// Each of those is continuous in the heading because this is, which is
    /// what the acceptance test means by no pop crossing from forward to
    /// diagonal.
    #[must_use]
    pub fn along(self) -> f32 {
        self.angle.cos()
    }

    /// How much of the travel is sideways: `+1` to the body's left.
    #[must_use]
    pub fn across(self) -> f32 {
        self.angle.sin()
    }

    /// What share of a forward stride a body takes going this way.
    ///
    /// **An ellipse quadrant by quadrant**, with semi-axes of one going
    /// forward, [`BACKWARD_REACH`] going back and [`LATERAL_REACH`] going
    /// sideways:
    ///
    /// ```text
    /// 1/reach² = (along/a)² + (across/b)²
    /// ```
    ///
    /// where `a` is one or [`BACKWARD_REACH`] depending on which way the travel
    /// leans. That shape is the point of it rather than a curve fitted to
    /// anything: it is what a *reach* looks like — the boundary of how far a
    /// limb gets in each direction — and it means a diagonal is a stride in its
    /// own right rather than two strides blended, which is what the acceptance
    /// note asked for.
    ///
    /// **Smooth where the axis changes, which is not obvious and is the reason
    /// to write it this way.** The forward semi-axis switches at exactly the
    /// sideways heading — but `along` is zero there, so the term carrying the
    /// switch vanishes, and so does its slope. The seam is invisible in the
    /// value and in the first derivative, which is what "no pop" means when the
    /// heading is being swung continuously rather than stepped between presets.
    #[must_use]
    pub fn reach(self) -> f32 {
        self.reach_within(LATERAL_REACH)
    }

    /// As [`Self::reach`], with the sideways semi-axis given rather than taken
    /// from [`LATERAL_REACH`].
    ///
    /// **This is where a body's own geometry gets in**, and it goes into the
    /// axis rather than being applied afterwards. A sideways stride is bounded
    /// by more than the hip: two feet at opposite points of the cycle can pass
    /// each other, and how far they may travel before they do is a property of
    /// the stance — see `super::gait::Stride::toward`, which is what calls
    /// this.
    ///
    /// **Taking the smaller of the two afterwards is the obvious alternative
    /// and it pops.** Measured: a hard `min` against the geometric limit cut a
    /// 30-degree diagonal from 414 mm to 177 and put a step at 60 degrees where
    /// the bound stopped binding, because a clearance is a hard constraint and
    /// hard constraints have corners. Moving it into the semi-axis keeps the
    /// ellipse an ellipse, so a diagonal is interpolated rather than clipped
    /// and the sweep stays smooth all the way round.
    #[must_use]
    pub fn reach_within(self, lateral: f32) -> f32 {
        let (along, across) = (self.along(), self.across());
        let fore = if along >= 0.0 { 1.0 } else { BACKWARD_REACH };
        let lateral = lateral.max(f32::EPSILON);
        let sum = (along / fore).powi(2) + (across / lateral).powi(2);
        if sum <= f32::EPSILON {
            return 1.0;
        }
        sum.sqrt().recip()
    }

    /// A name for this heading, for an instrument to print.
    #[must_use]
    pub fn describe(self) -> &'static str {
        let degrees = self.angle.to_degrees().rem_euclid(360.0);
        match degrees {
            d if !(22.5..337.5).contains(&d) => "forward",
            d if d < 67.5 => "forward and left",
            d if d < 112.5 => "left",
            d if d < 157.5 => "back and left",
            d if d < 202.5 => "backwards",
            d if d < 247.5 => "back and right",
            d if d < 292.5 => "right",
            _ => "forward and right",
        }
    }
}

impl Default for Heading {
    fn default() -> Self {
        Self::FORWARD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_and_a_direction_are_the_same_thing_said_twice() {
        for degrees in [0.0f32, 30.0, 90.0, 150.0, 180.0, -45.0, -90.0] {
            let heading = Heading::degrees(degrees);
            let back = Heading::toward(heading.direction());
            assert!(
                (back.direction() - heading.direction()).length() < 1e-5,
                "{degrees} deg went out as {:?} and came back as {:?}",
                heading.direction(),
                back.direction()
            );
        }
        // Forward is `+Z`, and left is `+X` — the same sense `Turn` uses.
        assert!((Heading::FORWARD.direction() - Vec3::Z).length() < 1e-6);
        assert!((Heading::LEFT.direction() - Vec3::X).length() < 1e-6);
        assert!((Heading::BACKWARD.direction() - Vec3::NEG_Z).length() < 1e-6);
        assert!(Heading::toward(Vec3::ZERO) == Heading::FORWARD);
    }

    #[test]
    fn the_reach_is_shortest_sideways_and_longest_forward() {
        assert!((Heading::FORWARD.reach() - 1.0).abs() < 1e-6);
        assert!((Heading::BACKWARD.reach() - BACKWARD_REACH).abs() < 1e-6);
        assert!((Heading::LEFT.reach() - LATERAL_REACH).abs() < 1e-6);
        assert!((Heading::RIGHT.reach() - LATERAL_REACH).abs() < 1e-6);
        // And the literature's band for a backward step, which is what
        // `BACKWARD_REACH` is anchored on.
        assert!((0.70..=0.80).contains(&Heading::BACKWARD.reach()));
    }

    #[test]
    fn a_diagonal_is_a_stride_and_not_a_crossfade() {
        // **The acceptance test: no pop.** Swept a degree at a time all the way
        // round, no single step may stand out from the rest — which is not the
        // same as no step being large. The reach genuinely falls from 1.0 to
        // 0.6 across a quarter turn, so the honest curve moves about four
        // thousandths a degree and reaches eight; bounding the step at five
        // thousandths, as this first did, fails on the curve being a curve.
        // What a pop looks like is one step unlike its neighbours, so that is
        // what is asked: the largest step against the typical one.
        //
        // Reintroducing the defect means picking the reach by quadrant —
        // `if forward { 1.0 } else { BACKWARD_REACH }` — whose seam at 90
        // degrees is a step of 0.25 against a median of nothing at all.
        let steps: Vec<f32> = (0..=360)
            .map(|degrees| Heading::degrees(degrees as f32).reach())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .collect();
        let mut sorted = steps.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2];
        let (worst_at, worst) = steps
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .expect("a sweep");
        assert!(
            *worst < median * 5.0,
            "the reach stepped {worst:.4} at {worst_at} degrees against a typical {median:.4}"
        );
    }

    #[test]
    fn the_seam_at_sideways_is_smooth_in_the_slope_too() {
        // A value that is continuous but kinked still pops, because what a
        // viewer sees is the body's SPEED of change. The forward semi-axis
        // switches at exactly 90 degrees, and it is `along` being zero there
        // that hides the seam in the slope as well as in the value.
        let slope = |degrees: f32| {
            let step = 0.01;
            (Heading::degrees(degrees + step).reach() - Heading::degrees(degrees - step).reach())
                / (2.0 * step)
        };
        for seam in [90.0f32, -90.0] {
            let (before, after) = (slope(seam - 0.5), slope(seam + 0.5));
            assert!(
                (before - after).abs() < 1e-3,
                "the reach kinked at {seam} deg: {before} into {after}"
            );
        }
    }
}
