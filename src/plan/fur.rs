//! A beast's fur (#369): how long it grows in the zones where a beast grows
//! long hair - a mane and its forelock, a ruff, a brush along the tail,
//! feathering at the fetlocks and tufts at the ears' tips. Everywhere else is
//! the coat (#368), painted.
//!
//! **Lengths, not styles.** Each zone is one axis, none at zero, so a record
//! carries five small numbers and a reader that knows nothing of fur draws a
//! beast bald, which is what it drew before. What a zone's fur looks like is
//! `hair::fur`'s business; what a record says is how much of it there is.
//!
//! Rolled under the hair lock, which on a beast is the fur lock, with each
//! family's own odds: a horse a long mane and tail, a dog or a cat sometimes
//! a ruff or a brush, a cat sometimes lynx tips. Every zone is open to every
//! family, rarely, as the coat's loci are: a cat with a mane is a lion.
//!
//! **And one that is not a length** (#389): how stiff a mane is, from lying
//! flat to a short upright crest. Nothing rolls it, so a re-roll leaves it where
//! its owner put it and no record's generator stamp moves; every beast starts
//! at [`Fur::DEFAULT_MANE_STIFFNESS`], and a record at that default does not
//! write it.

use serde::{Deserialize, Serialize};

use super::{Family, PlanDecodeError, Rolls};

/// How long a beast's fur grows in each zone: `0` none, `1` long, as a share
/// of the zone's own full length, which scales with the body.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Fur {
    /// A mane down the neck's crest, and a forelock between the ears.
    #[serde(with = "super::scaled")]
    pub mane: f32,
    /// A ruff round the neck and over the chest's front.
    #[serde(with = "super::scaled")]
    pub ruff: f32,
    /// A brush along the tail.
    #[serde(with = "super::scaled")]
    pub tail: f32,
    /// Feathering down the back of the fetlocks.
    #[serde(with = "super::scaled")]
    pub feathering: f32,
    /// Tufts at the ears' tips.
    #[serde(with = "super::scaled")]
    pub tufts: f32,
    /// How stiff the mane is (#389), `0` lying flat over the neck and draping,
    /// `1` a short stiff upright crest, a zebra's or a hogged mane. Its length
    /// stays [`Self::mane`]'s. Left off the wire at its default.
    #[serde(
        with = "super::scaled",
        skip_serializing_if = "is_default_mane_stiffness"
    )]
    pub mane_stiffness: f32,
}

impl Default for Fur {
    /// No fur anywhere, and a mane that would stand at the default if it grew.
    fn default() -> Self {
        Self {
            mane: 0.0,
            ruff: 0.0,
            tail: 0.0,
            feathering: 0.0,
            tufts: 0.0,
            mane_stiffness: Self::DEFAULT_MANE_STIFFNESS,
        }
    }
}

/// Whether a record writes no mane stiffness: the default, left off the wire,
/// so that a record saved before the field existed writes the bytes it read.
/// Compared on the wire's own thousandths.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_mane_stiffness(stiffness: &f32) -> bool {
    super::scaled::quantize(*stiffness) == Fur::DEFAULT_MANE_STIFFNESS
}

/// The range a mane's stiffness is stored in. It is a share rather than a
/// length, so there is no envelope past it: past `1` there is nothing stiffer.
const STIFFNESS_SPAN: (f32, f32) = (0.0, 1.0);

/// The range every zone's length is stored in: none to long, explored past
/// long.
fn span() -> (f32, f32) {
    super::explore_range(0.0, (0.0, 1.0))
}

/// How often a family grows a zone's fur at all, and how long it runs when
/// it does.
#[derive(Clone, Copy, Debug)]
struct Odds {
    chance: f32,
    length: (f32, f32),
}

impl Odds {
    const NEVER: Self = Self {
        chance: 0.0,
        length: (0.0, 0.0),
    };

    /// `share` of the way from `self` to `other`: a cross's.
    fn toward(self, other: Self, share: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * share;
        // A zone one parent never grows takes the other's lengths whole,
        // rather than shortening toward a length nothing ever drew.
        let length = if self.chance <= 0.0 {
            other.length
        } else if other.chance <= 0.0 {
            self.length
        } else {
            (
                mix(self.length.0, other.length.0),
                mix(self.length.1, other.length.1),
            )
        };
        Self {
            chance: mix(self.chance, other.chance),
            length,
        }
    }
}

/// A family's odds for each zone, in [`Fur`]'s field order.
/// Provenance: **chosen** (#369), by the coats each family is known by: a
/// horse's mane and tail, a spitz's and a collie's ruff, a setter's
/// feathering, a Maine Coon's brush and lynx tips - and the owner's call that
/// a beast is cuter by default (#364), which a little fluff is.
fn odds(family: Family) -> [Odds; 5] {
    let odds = |chance, length| Odds { chance, length };
    match family {
        Family::Dog => [
            odds(0.03, (0.3, 0.6)),
            odds(0.40, (0.3, 0.9)),
            odds(0.45, (0.3, 0.9)),
            odds(0.30, (0.3, 0.8)),
            odds(0.05, (0.3, 0.7)),
        ],
        Family::Cat => [
            odds(0.04, (0.4, 0.8)),
            odds(0.40, (0.3, 0.9)),
            odds(0.50, (0.4, 1.0)),
            odds(0.10, (0.3, 0.6)),
            odds(0.25, (0.4, 1.0)),
        ],
        Family::Horse => [
            odds(0.97, (0.5, 1.0)),
            Odds::NEVER,
            odds(1.0, (0.6, 1.0)),
            odds(0.20, (0.4, 1.0)),
            Odds::NEVER,
        ],
    }
}

impl Fur {
    /// How stiff a mane is until its owner says otherwise: a modest crest
    /// standing along the neck, with the rest of the mane falling over it.
    /// Provenance: **tuned by render** (#389).
    pub const DEFAULT_MANE_STIFFNESS: f32 = 0.4;

    /// Each zone's length, in field order.
    fn zones_mut(&mut self) -> [&mut f32; 5] {
        [
            &mut self.mane,
            &mut self.ruff,
            &mut self.tail,
            &mut self.feathering,
            &mut self.tufts,
        ]
    }

    /// Clamps every length into its envelope.
    pub fn sanitize(&mut self) {
        for zone in self.zones_mut() {
            *zone = super::sanitize_axis(*zone, 0.0, span());
        }
        self.mane_stiffness = super::sanitize_axis(
            self.mane_stiffness,
            Self::DEFAULT_MANE_STIFFNESS,
            STIFFNESS_SPAN,
        );
    }

    /// Draws fresh fur for a body of `family`, crossed with `hybrid` `blend`
    /// of the way, each zone on streams of its own.
    pub(crate) fn reroll(
        &mut self,
        rolls: &Rolls,
        family: Family,
        hybrid: Option<Family>,
        blend: f32,
    ) {
        let own = odds(family);
        let odds = match hybrid {
            Some(other) => {
                let theirs = odds(other);
                let share = blend.clamp(0.0, 1.0);
                std::array::from_fn(|zone| own[zone].toward(theirs[zone], share))
            }
            None => own,
        };
        let streams = [
            ("fur.mane", "fur.maneLength"),
            ("fur.ruff", "fur.ruffLength"),
            ("fur.tail", "fur.tailLength"),
            ("fur.feathering", "fur.featheringLength"),
            ("fur.tufts", "fur.tuftsLength"),
        ];
        for ((zone, odds), (grows, long)) in self.zones_mut().into_iter().zip(odds).zip(streams) {
            *zone = if rolls.range(grows, 0.0, 1.0) < odds.chance {
                rolls.range(long, odds.length.0, odds.length.1)
            } else {
                0.0
            };
        }
    }

    /// Its bytes on a share code: a span for each zone. The mane's stiffness
    /// is not among them: see [`Self::stiffness_byte`].
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        for zone in [self.mane, self.ruff, self.tail, self.feathering, self.tufts] {
            super::put_span(out, zone, span());
        }
    }

    /// Fur back off a share code.
    pub(crate) fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut fur = Self::default();
        for zone in fur.zones_mut() {
            *zone = super::take_span(bytes, span())?;
        }
        fur.sanitize();
        Ok(fur)
    }

    /// The mane's stiffness as the one byte a version 11 share code ends with.
    pub(crate) fn stiffness_byte(&self) -> u8 {
        let mut out = Vec::with_capacity(1);
        super::put_span(&mut out, self.mane_stiffness, STIFFNESS_SPAN);
        out[0]
    }

    /// Whether a share code has to carry the mane's stiffness: not if a reader
    /// assuming the default gets the same byte back.
    pub(crate) fn stiffness_needs_a_byte(&self) -> bool {
        self.stiffness_byte() != Self::default().stiffness_byte()
    }

    /// Reads the mane's stiffness off the end of a version 11 share code.
    pub(crate) fn take_stiffness(&mut self, bytes: &mut &[u8]) -> Result<(), PlanDecodeError> {
        self.mane_stiffness = super::take_span(bytes, STIFFNESS_SPAN)?;
        self.sanitize();
        Ok(())
    }

    /// Whether any zone grows anything.
    #[must_use]
    pub fn any(&self) -> bool {
        [self.mane, self.ruff, self.tail, self.feathering, self.tufts]
            .into_iter()
            .any(|zone| zone > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolled(seed: i64, family: Family, hybrid: Option<Family>) -> Fur {
        let mut fur = Fur::default();
        fur.reroll(&Rolls::new(seed), family, hybrid, 0.5);
        fur
    }

    #[test]
    fn fur_survives_its_bytes_to_a_step() {
        for seed in 0..100 {
            for family in Family::ALL {
                let fur = rolled(seed, family, None);
                let mut out = Vec::new();
                fur.encode(&mut out);
                assert_eq!(out.len(), 5, "a span a zone");
                let back = Fur::decode(&mut out.as_slice()).expect("decodes");
                let step = (span().1 - span().0) / 255.0;
                for (a, b) in [
                    (fur.mane, back.mane),
                    (fur.ruff, back.ruff),
                    (fur.tail, back.tail),
                    (fur.feathering, back.feathering),
                    (fur.tufts, back.tufts),
                ] {
                    assert!((a - b).abs() <= step, "{a} came back as {b}");
                }
            }
        }
        assert!(
            Fur::decode(&mut [0u8; 4].as_slice()).is_err(),
            "a short code fails"
        );
    }

    #[test]
    fn each_family_grows_the_fur_it_is_known_by() {
        let count = |family: Family, zone: fn(&Fur) -> f32| {
            (0..400)
                .filter(|&seed| zone(&rolled(seed, family, None)) > 0.0)
                .count()
        };
        // A horse nearly always has a mane and always a tail, and never a
        // ruff or tufts.
        assert!(count(Family::Horse, |fur| fur.mane) > 370);
        assert_eq!(count(Family::Horse, |fur| fur.tail), 400);
        assert_eq!(count(Family::Horse, |fur| fur.ruff), 0);
        assert_eq!(count(Family::Horse, |fur| fur.tufts), 0);
        // A cat sometimes has lynx tips and a dog seldom; both sometimes a
        // ruff; either now and then a mane.
        assert!(count(Family::Cat, |fur| fur.tufts) > count(Family::Dog, |fur| fur.tufts));
        assert!((100..240).contains(&count(Family::Dog, |fur| fur.ruff)));
        assert!((1..40).contains(&count(Family::Cat, |fur| fur.mane)));
    }

    #[test]
    fn a_cross_grows_what_either_parent_might() {
        // A dog crossed with a horse may have a mane and may have a ruff, and
        // a ruff it grows is as long as a dog's: the horse never grows one,
        // so it does not pull the length toward nothing.
        let crosses: Vec<Fur> = (0..400)
            .map(|seed| rolled(seed, Family::Dog, Some(Family::Horse)))
            .collect();
        assert!(crosses.iter().filter(|fur| fur.mane > 0.0).count() > 100);
        let ruffs: Vec<f32> = crosses
            .iter()
            .map(|fur| fur.ruff)
            .filter(|&ruff| ruff > 0.0)
            .collect();
        assert!(!ruffs.is_empty());
        assert!(ruffs.iter().all(|&ruff| (0.3..=0.9).contains(&ruff)));
    }

    #[test]
    fn a_length_past_the_envelope_is_clamped_and_none_is_bald() {
        let mut fur = Fur {
            mane: 9.0,
            ruff: -1.0,
            ..Fur::default()
        };
        fur.sanitize();
        assert_eq!(fur.mane, span().1);
        assert_eq!(fur.ruff, 0.0);
        assert!(!Fur::default().any(), "a record without fur reads bald");
    }

    #[test]
    fn a_mane_s_stiffness_is_clamped_and_a_bad_one_is_the_default() {
        let mut fur = Fur {
            mane_stiffness: 1.7,
            ..Fur::default()
        };
        fur.sanitize();
        assert_eq!(fur.mane_stiffness, 1.0);
        fur.mane_stiffness = -0.2;
        fur.sanitize();
        assert_eq!(fur.mane_stiffness, 0.0);
        fur.mane_stiffness = f32::NAN;
        fur.sanitize();
        assert_eq!(fur.mane_stiffness, Fur::DEFAULT_MANE_STIFFNESS);
    }

    #[test]
    fn a_re_roll_leaves_a_mane_s_stiffness_where_its_owner_put_it() {
        for seed in 0..50 {
            for family in Family::ALL {
                let mut fur = Fur {
                    mane_stiffness: 0.85,
                    ..Fur::default()
                };
                fur.reroll(&Rolls::new(seed), family, None, 0.0);
                assert_eq!(fur.mane_stiffness, 0.85);
            }
        }
    }

    #[test]
    fn a_default_stiffness_is_left_off_the_wire_and_a_record_without_one_reads_the_default() {
        let saved_today = r#"{"mane":800,"ruff":0,"tail":700,"feathering":0,"tufts":0}"#;
        let read: Fur = serde_json::from_str(saved_today).expect("a record saved today reads");
        assert_eq!(read.mane_stiffness, Fur::DEFAULT_MANE_STIFFNESS);
        assert_eq!(
            serde_json::to_string(&read).expect("writes"),
            saved_today,
            "a record saved today writes other bytes"
        );
        let stiff = Fur {
            mane_stiffness: 1.0,
            ..read
        };
        let written = serde_json::to_string(&stiff).expect("writes");
        assert!(written.contains(r#""maneStiffness":1000"#), "{written}");
        let back: Fur = serde_json::from_str(&written).expect("reads");
        assert_eq!(back, stiff);
    }

    #[test]
    fn a_reader_that_knows_no_field_reads_past_it() {
        // What a client older than `maneStiffness` meets: a key it has no field
        // for, on the same struct. The next key after it must not be lost.
        let newer = r#"{"mane":800,"aKeyFromTheFuture":1000,"tail":700}"#;
        let read: Fur = serde_json::from_str(newer).expect("an unknown fur key is read past");
        assert_eq!((read.mane, read.tail), (0.8, 0.7));
    }
}
