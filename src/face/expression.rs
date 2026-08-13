//! What a face is saying when it is not saying anything.
//!
//! The sibling of [`super::blink`] and [`super::talk`], one layer up: those
//! two decide when the involuntary machinery moves, and this decides where
//! the face RESTS while they do. An expression here is four numbers — one per
//! bone pair the macro rig grew (#135, #215, #216) plus the lids — and a
//! preset is a named point in that space, not a pose: two expressions blend
//! by interpolating the numbers and applying once, never by mixing poses.
//! Blending in pose space dips through neutral on opposed channels — half a
//! smile plus half a frown is a straight mouth, not a flickering one — and
//! quaternion blends do not commute with per-side sign flips (#217).
//!
//! **The composition contract, per channel.** The brows and the corners are
//! written over: nothing else in the crate poses them, so an expression is
//! their single writer. The jaw is ADDED to whatever the pose already
//! carries, because [`super::talk`] owns speech and a happy body that starts
//! talking should keep smiling — the channel's gain is a third of talk's own
//! conversational open, so the sum stays under the yawn that `--jaw 20`
//! inspects. The lids are not written at all: [`super::eye::Eyes::blink`]
//! owns the four lid joints inside [`crate::Avatar::posed`], and an
//! expression that wrote them too would be a second writer fighting the
//! first — instead [`Expression::closure`] hands the caller a resting bias
//! to feed into the closure it already passes.

use glam::Quat;

use crate::anim::Pose;
use crate::rig::Rig;
use crate::rig::skin::{brow_joints, jaw_pivot, mouth_corners};

/// Degrees of whole-brow raise at `brows = 1.0`, and of lowering at `-1.0`.
///
/// The range `render --brows` was judged over on #215: 10° reads as surprise
/// and −8° as a scowl, so the channel spans what the render has already
/// vouched for and nothing it has not.
const BROW_GAIN: f32 = 10.0;

/// Degrees of smile at `corners = 1.0`, and of frown at `-1.0`.
///
/// 22° read as a smile and −10° as a frown on #216's sheets; the asymmetry is
/// real — a mouth droops less than it grins — and lives in the presets rather
/// than the gain, which stays one number so the channel is linear.
const CORNER_GAIN: f32 = 22.0;

/// Degrees of jaw parting at `jaw = 1.0`.
///
/// A third of [`super::talk::TalkConfig::open`]'s 12° conversational swing,
/// which is the composition bound: an expression's parting plus a syllable's
/// peak stays a comfortable open, and the 20° yawn stays the instrument's.
const JAW_GAIN: f32 = 4.0;

/// Closure bias at `lids = 1.0` (heavy).
///
/// Heavy-lidded is most of the closure range; the widening end at `-1.0` is
/// [`super::eye::Eye::WIDEN`], owned by the eye because the lid shells are
/// its to protect — `lid_rotation`'s clamp is the bound, and this layer asks
/// for exactly as much as that clamp will grant, no constant of its own to
/// drift (#217's P2, which arrived as a silently dead channel rather than a
/// cleared rim: the old clamp floor was zero).
const LID_HEAVY: f32 = 0.40;

/// A resting face, as a point in the macro rig's own space.
///
/// Channels are unitless `-1..=1`, `0` neutral; the degree gains are this
/// module's constants, derived from the judged instrument ranges. A record or
/// a preset carries these numbers, and what they mean in radians is decided
/// once, here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expression {
    /// Both brows: raised toward `+1`, lowered toward `-1`.
    pub brows: f32,
    /// Both mouth corners: a smile toward `+1`, a frown toward `-1`.
    pub corners: f32,
    /// The mandible's resting parting, `0..=1`; an expression never clenches
    /// past closed, so the negative half is clamped away.
    pub jaw: f32,
    /// The lids' resting weight: heavy toward `+1`, widened toward `-1`.
    pub lids: f32,
}

impl Default for Expression {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

impl Expression {
    /// Nothing: every channel at rest.
    pub const NEUTRAL: Self = Self {
        brows: 0.0,
        corners: 0.0,
        jaw: 0.0,
        lids: 0.0,
    };

    /// A smile with lifted brows and the mouth just parted.
    pub const HAPPY: Self = Self {
        brows: 0.25,
        corners: 0.85,
        jaw: 0.30,
        lids: 0.10,
    };

    /// Dropped corners under heavy lids.
    ///
    /// The macro rig has one bone per brow and sadness lives in the INNER
    /// brow's tilt, so this preset leans on the mouth and the lids instead of
    /// asking the brows for a shape they cannot make — see #217's P3 for the
    /// residual, and the reroll of that judgement if a second brow bone ever
    /// lands.
    ///
    /// The corner droop is HALF the smile's reach on purpose, not taste: past
    /// about −11° the dropped seam edges punch through the lower lip's own
    /// bulge below them and surface as tabs (#217, measured at −15.4° and
    /// clean at −10°). A smile has no such collision — the upper lip recedes
    /// where the lower one protrudes — so the asymmetry is the face's
    /// geometry, and every droop in this catalogue stays inside it.
    pub const SAD: Self = Self {
        brows: -0.10,
        corners: -0.50,
        jaw: 0.0,
        lids: 0.40,
    };

    /// Lowered brows over a hardened mouth. The droop bound is [`Self::SAD`]'s.
    pub const ANGRY: Self = Self {
        brows: -0.90,
        corners: -0.35,
        jaw: 0.0,
        lids: 0.10,
    };

    /// High brows, wide eyes, dropped jaw.
    pub const SURPRISED: Self = Self {
        brows: 1.00,
        corners: -0.10,
        jaw: 0.75,
        lids: -1.00,
    };

    /// The preset catalogue, by the names the instruments and any UI use.
    pub const PRESETS: [(&str, Self); 5] = [
        ("neutral", Self::NEUTRAL),
        ("happy", Self::HAPPY),
        ("sad", Self::SAD),
        ("angry", Self::ANGRY),
        ("surprised", Self::SURPRISED),
    ];

    /// The preset called `name`, or `None` — the instruments' spelling.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::PRESETS
            .iter()
            .find(|(preset, _)| *preset == name)
            .map(|(_, expression)| *expression)
    }

    /// Every channel clamped to its documented range.
    ///
    /// The jaw's floor is `0`: an expression parts a mouth or leaves it, and
    /// a negative parting would clench the mandible up through the skull.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            brows: self.brows.clamp(-1.0, 1.0),
            corners: self.corners.clamp(-1.0, 1.0),
            jaw: self.jaw.clamp(0.0, 1.0),
            lids: self.lids.clamp(-1.0, 1.0),
        }
    }

    /// The straight line from `self` to `other`, at `t` of the way.
    ///
    /// This is the ONLY blending an expression supports, and it is in
    /// expression space on purpose — see the module docs for what pose-space
    /// mixing does to opposed channels. Endpoint-EXACT: a transition has to
    /// land on the expression it was heading for, and `a + (b − a) * t`
    /// misses by a bit-rounding at `t = 1` (the guard caught −0.099999994
    /// arriving where −0.1 was due).
    #[must_use]
    pub fn toward(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        if t <= 0.0 {
            return self;
        }
        if t >= 1.0 {
            return other;
        }
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        Self {
            brows: lerp(self.brows, other.brows),
            corners: lerp(self.corners, other.corners),
            jaw: lerp(self.jaw, other.jaw),
            lids: lerp(self.lids, other.lids),
        }
    }

    /// Writes this expression onto `pose`, touching only the face bones.
    ///
    /// Brows and corners are written over; the jaw is composed onto whatever
    /// rotation the pose already carries (a talking jaw keeps talking). The
    /// lids are deliberately not here — see [`Self::closure`].
    pub fn apply(&self, rig: &Rig, pose: &mut Pose) {
        let this = self.sanitized();
        for (brow, _) in brow_joints(rig) {
            if let Some(rotation) = pose.rotations.get_mut(brow) {
                *rotation = Quat::from_rotation_x(-(this.brows * BROW_GAIN).to_radians());
            }
        }
        for (corner, sign, _) in mouth_corners(rig) {
            if let Some(rotation) = pose.rotations.get_mut(corner) {
                *rotation = Quat::from_rotation_z(sign * (this.corners * CORNER_GAIN).to_radians());
            }
        }
        if let Some(pivot) = jaw_pivot(rig)
            && let Some(rotation) = pose.rotations.get_mut(pivot)
        {
            *rotation *= Quat::from_rotation_x((this.jaw * JAW_GAIN).to_radians());
        }
    }

    /// The lids' closure with a blink at `phase` composed over this
    /// expression's rest.
    ///
    /// **The compositor, and not an addition** — the guard found the hole:
    /// summing a widened rest (−0.12) with a full blink (1.0) leaves the lids
    /// at 0.88, an eye that never quite shuts on a surprised face. A blink
    /// interpolates from wherever the lids REST to fully shut, so this is
    /// `rest + (1 − rest) · phase`: phase 0 is the expression's own rest,
    /// phase 1 is shut whatever the expression says. Callers compose through
    /// this rather than adding, and [`crate::Avatar::posed`] gets the result
    /// as its `closure`.
    #[must_use]
    pub fn closure_at(&self, phase: f32) -> f32 {
        let rest = self.closure();
        rest + (1.0 - rest) * phase.clamp(0.0, 1.0)
    }

    /// The lids' resting closure under this expression: negative widens, up
    /// to the bound [`super::eye::Eye::WIDEN`] owns; positive weighs them
    /// down by `LID_HEAVY`. Equal to [`Self::closure_at`] of a blink at
    /// phase zero.
    #[must_use]
    pub fn closure(&self) -> f32 {
        let lids = self.sanitized().lids;
        if lids >= 0.0 {
            lids * LID_HEAVY
        } else {
            lids * super::eye::Eye::WIDEN
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A built default avatar, its rig's rest pose, and the same pose with
    /// `expression` applied.
    fn applied(expression: Expression) -> (crate::Avatar, Pose, Pose) {
        let record = crate::AvatarRecord::new("Expressed", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rest = Pose::rest(&avatar.rig);
        let mut posed = rest.clone();
        expression.apply(&avatar.rig, &mut posed);
        (avatar, rest, posed)
    }

    #[test]
    fn an_expression_touches_exactly_the_face_bones() {
        // #217's P4, as the guard: the layer is a POSE layer, so the whole of
        // what it may do is rotate the five face joints it names — two brows,
        // two corners, the jaw's pivot. A sixth joint moving under an
        // expression is a body that shrugs when it smiles, and the failure
        // names it.
        let (avatar, rest, posed) = applied(Expression::HAPPY);
        let rig = &avatar.rig;
        let expected: Vec<usize> = crate::rig::skin::brow_joints(rig)
            .into_iter()
            .map(|(joint, _)| joint)
            .chain(
                crate::rig::skin::mouth_corners(rig)
                    .into_iter()
                    .map(|(joint, _, _)| joint),
            )
            .chain(crate::rig::skin::jaw_pivot(rig))
            .collect();
        assert_eq!(expected.len(), 5, "the macro face rig is five joints");

        let moved: Vec<usize> = (0..rig.len())
            .filter(|&joint| posed.rotations[joint] != rest.rotations[joint])
            .collect();
        assert_eq!(
            moved,
            {
                let mut sorted = expected.clone();
                sorted.sort_unstable();
                sorted
            },
            "HAPPY moved joints other than the five face bones"
        );
    }

    #[test]
    fn a_blink_still_closes_over_every_preset() {
        // The lids' single-writer contract, per preset: an expression biases
        // the RESTING closure and the blink writes the joints, so a closure
        // of 1.0 has to land every preset's lids at the same full shut — a
        // preset that kept an eye open through a blink would mean two writers
        // fighting, which is exactly what Expression::apply not touching the
        // lids exists to prevent.
        let record = crate::AvatarRecord::new("Blinked", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let eyes = avatar.parts.eyes.as_ref().expect("a humanoid has eyes");

        let mut shut = Pose::rest(rig);
        eyes.blink(&mut shut, 1.0);
        for (name, expression) in Expression::PRESETS {
            let mut posed = Pose::rest(rig);
            expression.apply(rig, &mut posed);
            // The expression's resting closure, then a full blink over it —
            // the order the shipped path composes in.
            eyes.blink(&mut posed, expression.closure_at(1.0));
            for eye in [&eyes.left, &eyes.right] {
                for joint in [eye.upper_joint, eye.lower_joint].into_iter().flatten() {
                    assert_eq!(
                        posed.rotations[joint], shut.rotations[joint],
                        "{name}: a full blink left a lid short of shut"
                    );
                }
            }
        }
    }

    #[test]
    fn expressions_blend_in_their_own_space() {
        // #217's P5 as arithmetic: halfway from HAPPY to SAD the corners pass
        // through the midpoint of the two values, not through a pose blend's
        // detour — and the endpoints are exact, which is what a transition
        // needs to land on.
        let half = Expression::HAPPY.toward(Expression::SAD, 0.5);
        assert!((half.corners - (0.85 - 0.50) / 2.0).abs() < 1e-6);
        assert_eq!(
            Expression::HAPPY.toward(Expression::SAD, 0.0),
            Expression::HAPPY
        );
        assert_eq!(
            Expression::HAPPY.toward(Expression::SAD, 1.0),
            Expression::SAD
        );
        // And the widening bound is the eye's, not this module's: the most a
        // preset may widen is exactly what `lid_rotation` will grant.
        assert!(Expression::SURPRISED.closure() >= -crate::face::eye::Eye::WIDEN);
    }
}
