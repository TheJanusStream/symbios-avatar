//! The order a four-legged body's feet land in (#370).
//!
//! A [`super::Gait`] is an offset per contact and a duty, and on four legs the
//! offsets are what make it one gait or another. A walk lands each hind and
//! then the fore on its own side, a quarter of a cycle apart; a trot lands the
//! diagonal pairs together and a pace the lateral pairs; a canter lands a
//! hind, then the diagonal pair, then the leading fore, and flies; a gallop
//! lands the hinds and then the fores, each pair's leading foot last - with
//! both leads on one side (a horse's: transverse) or on opposite sides (a
//! dog's or a cat's: rotary). A [`Pattern`] names one.
//!
//! [`Pattern::at`] picks the one a body moves in at a speed, as every
//! quadruped does: by its Froude number, walking, trotting, cantering and
//! galloping as it goes faster - and [`Pattern::held`] changes down later than
//! it changed up, so a body at the edge of a trot does not flicker between it
//! and a walk.
//!
//! The sequences are the textbook's (Hildebrand), not the CC0 references':
//! mesh2motion's fox and horse walks and its horse "trot" pace, measured
//! footfall by footfall (#370); its runs are sound gallops. The clips stay
//! references for posture, not for order.

use crate::plan::Limb;

/// Which side leads an asymmetric gait: the side whose fore lands last, just
/// before the body leaves the ground.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Lead {
    /// The left fore lands last.
    Left,
    /// The right fore lands last.
    #[default]
    Right,
}

/// The order a four-legged body's feet land in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// A lateral-sequence walk: each hind and then the fore on its own side,
    /// a quarter of a cycle apart - every walking quadruped's.
    Walk,
    /// The diagonal pairs together.
    Trot,
    /// The lateral pairs together: a camel's, an ambling dog's.
    Pace,
    /// Three beats - the trailing hind, the diagonal pair, the leading fore -
    /// and a moment in the air.
    Canter(Lead),
    /// Four beats, the hinds and then the fores, each pair's leading foot
    /// last and both leads on one side: transverse, a horse's.
    Gallop(Lead),
    /// The same four beats round the body, the leads on opposite sides:
    /// rotary, a dog's and a cat's.
    RotaryGallop(Lead),
}

/// The Froude numbers a quadruped changes up at: walk to trot, trot to canter,
/// canter to gallop.
///
/// Provenance: **the literature**. Alexander and Jayes (1983) found mammals of
/// every size changing from a walk to a trot near a Froude number of a half
/// and from a trot to a gallop between two and three; the canter takes the
/// lower half of that change and the gallop the rest.
const UP: [f32; 3] = [0.5, 2.5, 4.0];

/// The share of a change's own Froude number a body falls below before it
/// changes back down: a horse that has just broken into a trot does not drop
/// back to a walk at the first hesitation.
/// Provenance: **chosen** (#370).
const DOWN: f32 = 0.8;

/// Each pattern's duty at the bottom and the top of its own band of Froude
/// numbers, which it keeps beyond them: falling with speed within a gait and
/// stepping down between gaits, as a quadruped's does.
///
/// Provenance: **the literature**, research-08 2.1 (#378): dog walks 0.56 to
/// 0.64 and a cat's 0.64 at their ordinary speeds, a slower walk longer; dog
/// trots 0.39 to 0.47; a horse's canter near 0.4, and gallops 0.31 at 9.5 m/s
/// and 0.28 at 12.5, a greyhound's near 0.3. A trot at or above a half reads
/// as a shuffle.
const WALK_DUTY: (f32, f32) = (0.68, 0.58);
const TROT_DUTY: (f32, f32) = (0.47, 0.39);
const CANTER_DUTY: (f32, f32) = (0.38, 0.33);
const GALLOP_DUTY: (f32, f32) = (0.32, 0.27);

/// The Froude number a gallop's duty reaches its least at.
const GALLOP_TOP: f32 = 8.0;

/// How far apart, in a cycle, the two feet of a gallop's pair land: the
/// hinds, then the fores.
/// Provenance: **the literature**, a gallop's footfall diagrams.
const GALLOP_PAIR: f32 = 0.1;

/// When, in a canter, the diagonal pair lands after the trailing hind, and the
/// leading fore after that.
/// Provenance: **the literature**, a canter's footfall diagram.
const CANTER_BEATS: (f32, f32) = (0.3, 0.6);

impl Pattern {
    /// The pattern a four-legged body moves in at `froude`: a walk, then a
    /// trot, a canter and a gallop - rotary if `rotary`, on `lead`.
    #[must_use]
    pub fn at(froude: f32, rotary: bool, lead: Lead) -> Self {
        match UP.iter().filter(|&&up| froude >= up).count() {
            0 => Self::Walk,
            1 => Self::Trot,
            2 => Self::Canter(lead),
            _ if rotary => Self::RotaryGallop(lead),
            _ => Self::Gallop(lead),
        }
    }

    /// The pattern a body moving in `before` moves in at `froude`:
    /// [`Self::at`]'s on the way up, and on the way down only once the speed
    /// has fallen below four fifths of the change's own. A pattern [`Self::at`]
    /// never picks - a pace - is kept for as long as its speed band is.
    ///
    /// **A lead is kept once taken**: a canter quickening into a gallop, or
    /// a gallop slowing to one, leads on `before`'s side, and `lead` is the
    /// side a body breaking into a canter from a walk or a trot takes.
    ///
    /// **A gallop is the body's own**, whichever `before` was: a dog rebuilt
    /// on hooves mid-gallop gallops as a horse does at once, rather than
    /// rotary until its speed fell a band.
    #[must_use]
    pub fn held(froude: f32, before: Self, rotary: bool, lead: Lead) -> Self {
        let lead = before.lead().unwrap_or(lead);
        let now = Self::at(froude, rotary, lead);
        let kept = match now.rank().cmp(&before.rank()) {
            // Faster: change up at once.
            std::cmp::Ordering::Greater => now,
            std::cmp::Ordering::Equal => before,
            // Slower: only past the hysteresis.
            std::cmp::Ordering::Less => {
                let threshold = UP[before.rank() - 1] * DOWN;
                if froude < threshold {
                    Self::at(froude / DOWN, rotary, lead).lower_of(before)
                } else {
                    before
                }
            }
        };
        match kept {
            Self::Gallop(lead) | Self::RotaryGallop(lead) if rotary => Self::RotaryGallop(lead),
            Self::Gallop(lead) | Self::RotaryGallop(lead) => Self::Gallop(lead),
            other => other,
        }
    }

    /// Of `self` and `other`, the one lower on the speed axis.
    fn lower_of(self, other: Self) -> Self {
        if self.rank() <= other.rank() {
            self
        } else {
            other
        }
    }

    /// Where it sits along the speed axis: a walk, a trot or a pace, a
    /// canter, a gallop.
    fn rank(self) -> usize {
        match self {
            Self::Walk => 0,
            Self::Trot | Self::Pace => 1,
            Self::Canter(_) => 2,
            Self::Gallop(_) | Self::RotaryGallop(_) => 3,
        }
    }

    /// The same pattern on `lead`; a symmetric one has none to change.
    #[must_use]
    pub fn on(self, lead: Lead) -> Self {
        match self {
            Self::Canter(_) => Self::Canter(lead),
            Self::Gallop(_) => Self::Gallop(lead),
            Self::RotaryGallop(_) => Self::RotaryGallop(lead),
            Self::Walk | Self::Trot | Self::Pace => self,
        }
    }

    /// Its lead, if it is asymmetric.
    #[must_use]
    pub fn lead(self) -> Option<Lead> {
        match self {
            Self::Canter(lead) | Self::Gallop(lead) | Self::RotaryGallop(lead) => Some(lead),
            Self::Walk | Self::Trot | Self::Pace => None,
        }
    }

    /// Whether a body moving in it has a moment with no foot down at every
    /// speed: a canter's and a gallop's. A trot flies once its duty falls
    /// below a half; a walk never does.
    #[must_use]
    pub fn flies(self) -> bool {
        self.rank() >= 2
    }

    /// The share of its cycle each foot spends down at `froude`.
    #[must_use]
    pub fn duty(self, froude: f32) -> f32 {
        let (band, (slow, fast)) = match self {
            Self::Walk => ((0.0, UP[0]), WALK_DUTY),
            Self::Trot | Self::Pace => ((UP[0], UP[1]), TROT_DUTY),
            Self::Canter(_) => ((UP[1], UP[2]), CANTER_DUTY),
            Self::Gallop(_) | Self::RotaryGallop(_) => ((UP[2], GALLOP_TOP), GALLOP_DUTY),
        };
        let along = ((froude - band.0) / (band.1 - band.0)).clamp(0.0, 1.0);
        slow + (fast - slow) * along
    }

    /// When in its cycle `limb` lands, in `0..1`, with the left hind first.
    #[must_use]
    pub fn offset(self, limb: Limb) -> f32 {
        // Every pattern written on its right lead, and the left lead the same
        // with the sides swapped.
        let limb = match self.lead() {
            Some(Lead::Left) => mirrored(limb),
            _ => limb,
        };
        let (one, two) = CANTER_BEATS;
        match (self, limb) {
            (Self::Walk, Limb::HindLeft) => 0.0,
            (Self::Walk, Limb::ForeLeft) => 0.25,
            (Self::Walk, Limb::HindRight) => 0.5,
            (Self::Walk, Limb::ForeRight) => 0.75,
            (Self::Trot, Limb::HindLeft | Limb::ForeRight) => 0.0,
            (Self::Trot, Limb::HindRight | Limb::ForeLeft) => 0.5,
            (Self::Pace, Limb::HindLeft | Limb::ForeLeft) => 0.0,
            (Self::Pace, Limb::HindRight | Limb::ForeRight) => 0.5,
            // The trailing hind, the diagonal pair, the leading fore.
            (Self::Canter(_), Limb::HindLeft) => 0.0,
            (Self::Canter(_), Limb::HindRight | Limb::ForeLeft) => one,
            (Self::Canter(_), Limb::ForeRight) => two,
            // Both leads on the right: the right hind and fore land last of
            // their pairs.
            (Self::Gallop(_), Limb::HindLeft) => 0.0,
            (Self::Gallop(_), Limb::HindRight) => GALLOP_PAIR,
            (Self::Gallop(_), Limb::ForeLeft) => 0.5,
            (Self::Gallop(_), Limb::ForeRight) => 0.5 + GALLOP_PAIR,
            // Round the body: the right hind, the left hind, the left fore,
            // the right fore - the hind lead on the left, the fore's on the
            // right.
            (Self::RotaryGallop(_), Limb::HindRight) => 0.0,
            (Self::RotaryGallop(_), Limb::HindLeft) => GALLOP_PAIR,
            (Self::RotaryGallop(_), Limb::ForeLeft) => 0.5,
            (Self::RotaryGallop(_), Limb::ForeRight) => 0.5 + GALLOP_PAIR,
        }
    }

    /// Every pattern a body can be asked for, on the right lead.
    pub const ALL: [Self; 6] = [
        Self::Walk,
        Self::Trot,
        Self::Pace,
        Self::Canter(Lead::Right),
        Self::Gallop(Lead::Right),
        Self::RotaryGallop(Lead::Right),
    ];

    /// Its name, for an instrument's flag and a viewer's row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Walk => "walk",
            Self::Trot => "trot",
            Self::Pace => "pace",
            Self::Canter(_) => "canter",
            Self::Gallop(_) => "gallop",
            Self::RotaryGallop(_) => "rotary gallop",
        }
    }
}

/// The same limb on the body's other side.
fn mirrored(limb: Limb) -> Limb {
    match limb {
        Limb::ForeLeft => Limb::ForeRight,
        Limb::ForeRight => Limb::ForeLeft,
        Limb::HindLeft => Limb::HindRight,
        Limb::HindRight => Limb::HindLeft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The patterns' feet in the order they land, from the left hind.
    fn order(pattern: Pattern) -> Vec<(Limb, f32)> {
        let mut feet: Vec<(Limb, f32)> = Limb::ALL
            .into_iter()
            .map(|limb| (limb, pattern.offset(limb)))
            .collect();
        feet.sort_by(|a, b| a.1.total_cmp(&b.1));
        feet
    }

    #[test]
    fn each_pattern_lands_its_feet_in_its_own_order() {
        use Limb::{ForeLeft as LF, ForeRight as RF, HindLeft as LH, HindRight as RH};
        let feet = |pattern| {
            order(pattern)
                .into_iter()
                .map(|(limb, _)| limb)
                .collect::<Vec<_>>()
        };
        assert_eq!(feet(Pattern::Walk), [LH, LF, RH, RF], "a lateral sequence");
        assert_eq!(
            feet(Pattern::Gallop(Lead::Right)),
            [LH, RH, LF, RF],
            "transverse"
        );
        assert_eq!(
            feet(Pattern::RotaryGallop(Lead::Right)),
            [RH, LH, LF, RF],
            "rotary"
        );
        // Pairs that land together.
        let together = |pattern: Pattern, a, b| pattern.offset(a) == pattern.offset(b);
        assert!(together(Pattern::Trot, LH, RF) && together(Pattern::Trot, RH, LF));
        assert!(together(Pattern::Pace, LH, LF) && together(Pattern::Pace, RH, RF));
        assert!(
            together(Pattern::Canter(Lead::Right), RH, LF),
            "a canter's diagonal"
        );
        // A left lead is the right one with the sides swapped.
        let asymmetric: [fn(Lead) -> Pattern; 3] =
            [Pattern::Canter, Pattern::Gallop, Pattern::RotaryGallop];
        for pattern in asymmetric {
            for limb in Limb::ALL {
                assert_eq!(
                    pattern(Lead::Left).offset(limb),
                    pattern(Lead::Right).offset(mirrored(limb))
                );
            }
        }
    }

    #[test]
    fn a_body_walks_trots_canters_and_gallops_as_it_goes_faster() {
        let at = |froude| Pattern::at(froude, false, Lead::Right);
        assert_eq!(at(0.2), Pattern::Walk);
        assert_eq!(at(1.0), Pattern::Trot);
        assert_eq!(at(3.0), Pattern::Canter(Lead::Right));
        assert_eq!(at(6.0), Pattern::Gallop(Lead::Right));
        assert_eq!(
            Pattern::at(6.0, true, Lead::Left),
            Pattern::RotaryGallop(Lead::Left)
        );
        // And the duty falls the whole way, stepping down at each change.
        let mut last = 1.0;
        for step in 0..=80 {
            let froude = step as f32 * 0.1;
            let duty = at(froude).duty(froude);
            assert!(duty <= last + 1.0e-6, "the duty rose at {froude}");
            last = duty;
        }
    }

    #[test]
    fn each_patterns_duty_is_the_literatures() {
        // Fact 4 (#378): before it a walk ran up to 0.75, and a trot from 0.50
        // and a gallop down to 0.22, past anything measured.
        for froude in [0.05, 0.25, 0.49] {
            let duty = Pattern::Walk.duty(froude);
            assert!((0.56..=0.70).contains(&duty), "a walk at {froude}: {duty}");
        }
        for froude in [0.5, 1.2, 2.5] {
            let duty = Pattern::Trot.duty(froude);
            assert!((0.38..=0.48).contains(&duty), "a trot at {froude}: {duty}");
        }
        for froude in [4.0, 7.0, 12.0] {
            let duty = Pattern::Gallop(Lead::Right).duty(froude);
            assert!(
                (0.26..=0.34).contains(&duty),
                "a gallop at {froude}: {duty}"
            );
        }
    }

    #[test]
    fn a_body_changes_down_later_than_it_changed_up() {
        let held = |froude, before| Pattern::held(froude, before, false, Lead::Right);
        // Just under the change up, a trot stays a trot; well under, it walks.
        assert_eq!(held(0.45, Pattern::Trot), Pattern::Trot);
        assert_eq!(held(0.3, Pattern::Trot), Pattern::Walk);
        assert_eq!(
            held(2.2, Pattern::Canter(Lead::Left)),
            Pattern::Canter(Lead::Left)
        );
        assert_eq!(held(1.5, Pattern::Canter(Lead::Left)), Pattern::Trot);
        // A gallop slowing hard drops a gait at a time, past each threshold.
        assert_eq!(
            held(3.5, Pattern::Gallop(Lead::Right)),
            Pattern::Gallop(Lead::Right)
        );
        assert_eq!(
            held(3.0, Pattern::Gallop(Lead::Right)),
            Pattern::Canter(Lead::Right)
        );
        // Up is at once.
        assert_eq!(held(0.6, Pattern::Walk), Pattern::Trot);
        // A lead is kept once taken, and only a body breaking into a canter
        // takes the one it is offered.
        assert_eq!(
            held(5.0, Pattern::Canter(Lead::Left)),
            Pattern::Gallop(Lead::Left)
        );
        assert_eq!(
            Pattern::held(3.0, Pattern::Trot, false, Lead::Left),
            Pattern::Canter(Lead::Left)
        );
        assert_eq!(
            Pattern::held(3.0, Pattern::Gallop(Lead::Left), false, Lead::Right),
            Pattern::Canter(Lead::Left)
        );
        // A pace is kept while its band is.
        assert_eq!(held(1.0, Pattern::Pace), Pattern::Pace);
    }

    #[test]
    fn a_body_rebuilt_mid_gallop_gallops_as_its_new_feet_do() {
        // #370. A dog re-rolled on hooves while it gallops gallops as a horse
        // at once, on the lead it had, and a horse rebuilt on paws gallops
        // rotary: inside the hysteresis's band as well as above it.
        for froude in [5.0, 3.5] {
            for lead in [Lead::Left, Lead::Right] {
                assert_eq!(
                    Pattern::held(froude, Pattern::RotaryGallop(lead), false, Lead::Right),
                    Pattern::Gallop(lead),
                    "a rotary gallop on hooves at {froude}"
                );
                assert_eq!(
                    Pattern::held(froude, Pattern::Gallop(lead), true, Lead::Right),
                    Pattern::RotaryGallop(lead),
                    "a transverse gallop on paws at {froude}"
                );
            }
        }
    }
}
