//! The high-level axes a body is described by.
//!
//! A composite is an axis named for something about a *person* rather than
//! something about a body part, which fans out through the formulas in
//! [`super::derive`] to many quantities at once. `mass` reaches every girth,
//! `femininity` reaches the shoulders, the hips and the skull together, and
//! `bodyFat` reaches the waist one way and the wrist not at all.
//!
//! The rest of a plan's axes stay in the record beside these and become
//! *offsets*: a quantity is `formula(composites)` and then the per-region axis
//! applied on top. Neither tier replaces the other — the composite carries the
//! intent, which is what lets the formulas improve later without a stored
//! avatar losing what it meant, and the offset carries the choice a creator
//! made that no formula predicted (#161).
//!
//! ## Why these live on the record rather than on a body plan
//!
//! Every other parameter struct sits with its one consumer — `SkinParams` in
//! `texture`, `HairParams` in `hair`, `HumanoidParams` in `plan`. These have
//! three: the cage derives from them, the skull will (#166), and the skin will
//! (#165 wants muscle definition at low body fat, #167 wants creases with age).
//! Putting them inside an [`super::Archetype`] variant would make every one of
//! those consumers match on the variant to reach a number that has nothing to
//! do with which body plan is in use.
//!
//! ## Physical axes are not stretched, and that is a departure
//!
//! Every *shape* axis in this crate carries the exploration envelope of #160 —
//! its conservative range tripled about its own default — because going past
//! the population is a stylistic choice a creator is allowed to make. A giant
//! is a legible body; `height` is stretched to 3.1 m for exactly that reason.
//!
//! [`Composites::body_fat`] and [`Composites::age`] are not stretched, because
//! past their ends there is nothing to stylise: tripling the lean end of a body
//! fat fraction gives a NEGATIVE fraction, which is not a lean person but an
//! absence of one, and a hundred-and-fifty-year-old is not an old person but a
//! number. The line is whether the stretched value still names a body. Where it
//! does, stretch it; where it names nothing, the conservative range *is* the
//! range.

use serde::{Deserialize, Serialize};

use super::{Category, Rolls, explore_range, sanitize_axis};

/// Leanest and fattest a body may be, as a fraction of its mass.
///
/// The floor is roughly the essential fat of a very lean male athlete and the
/// ceiling is past the point where a silhouette stops changing much. Both ends
/// are outside what is healthy, deliberately: a creator tool has to be able to
/// draw people who exist.
///
/// **The same fraction is a different body on a masculine and a feminine
/// frame**, and that is anatomy rather than a defect in the axis. Essential fat
/// runs about 3% male against about 12% female, and the two store what they
/// carry in different places — abdominal against hip and thigh. Both facts are
/// the formulas' business: `femininity` drives the distribution that `body_fat`
/// fills (#164), which is the first place in this crate where two composites
/// have to be read together.
///
/// Provenance: **looked up** — the visual bands of body composition
/// photography, which put visible definition below about 10% on men and about
/// 18% on women, athletic through the high teens and low twenties, and average
/// through the low thirties.
pub const BODY_FAT_RANGE: (f32, f32) = (0.03, 0.60);

/// Body fat of a body no one has said anything about, as a fraction.
///
/// A middling adult, between the male and female midpoints of the population.
/// **This is the identity anchor**: at this value, with every other composite
/// neutral, the formulas must reproduce the body the plan built before
/// composites existed (#163).
pub const DEFAULT_BODY_FAT: f32 = 0.22;

/// Youngest and oldest a body may be, in whole years.
///
/// **Adults only, and the floor is a decision rather than a limitation.** Child
/// proportions are a different envelope entirely — a different head-to-height,
/// different girth exponents, different socket geometry — so they are a body
/// plan's worth of work and an owner's call, not the bottom end of a slider
/// somebody drags past by accident.
pub const AGE_RANGE: (u32, u32) = (18, 80);

/// Age of a body no one has said anything about, in years.
pub const DEFAULT_AGE: u32 = 28;

/// The exploration envelope of the two signed composites (#160).
fn signed_envelope() -> (f32, f32) {
    explore_range(0.0, (-1.0, 1.0))
}

/// The high-level description of one body.
///
/// Every axis here is read by more than one part of the crate, and every axis
/// here is a *formula input* rather than a quantity: nothing in this struct is
/// a length, a radius or a colour. See the module docs for the two tiers and
/// why physical axes are bounded differently from shape ones.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Composites {
    /// Where the body sits between the masculine and feminine references,
    /// `-1` fully masculine and `+1` fully feminine.
    ///
    /// **Zero is the midpoint of the two measured references, which is the body
    /// this plan already builds.** `plan::derive::humanoid`'s arm segments say
    /// so in as many words: the upper arm is the midpoint of a male 0.162 and a
    /// female 0.129 of stature, "the honest neutral until the frame axis (#100)
    /// carries that difference". This is that axis, and that one coefficient is
    /// where most of its travel will be.
    ///
    /// One axis for the whole body, read by the frame (#100), by the fat
    /// distribution (#164) and by the skull (#166) — because a body whose
    /// shoulders and whose jaw disagree about this reads as two people.
    #[serde(with = "super::scaled")]
    pub femininity: f32,
    /// How much body there is, from slight to heavy, `-1` to `+1`.
    ///
    /// Sets the girth budget; [`Self::body_fat`] decides how it is spent
    /// between fat and lean, and where. Mass does **not** scale a body
    /// uniformly: a heavier body carries it in the fleshy sites and barely at
    /// all in the bony ones, and the head hardly moves.
    ///
    /// Retires `build` and `muscle`, which said the same thing in two axes that
    /// could contradict each other (#164).
    #[serde(with = "super::scaled")]
    pub mass: f32,
    /// What share of the body is fat, as a fraction within
    /// [`BODY_FAT_RANGE`].
    ///
    /// A fraction rather than an abstract `0..1` slider, so the formulas can be
    /// written against the thresholds the eye actually reads — definition and
    /// vascularity appear at the lean end, softening and a filled waist at the
    /// heavy end — and so the numbers in them can be sourced rather than tuned.
    /// It reaches the skin as well as the shape (#165).
    #[serde(with = "super::scaled")]
    pub body_fat: f32,
    /// How old the body is, in whole years within [`AGE_RANGE`].
    ///
    /// **Whole years, and NOT through the thousandths encoder that every other
    /// axis uses, because that encoder cannot carry them.**
    /// `plan::scaled::serialize` clamps to `i16` before it writes, so an axis
    /// whose thousandths pass 32767 — that is, whose value passes 32.767 — is
    /// silently truncated. Every axis in the crate is inside `±3` or a stature
    /// in metres, so nothing had ever met that ceiling; an age in years walks
    /// straight into it and would have stored a 40-year-old as 32.767. A count
    /// is the honest representation anyway: nothing wants a fractional year.
    #[serde(
        deserialize_with = "super::scaled::deserialize_count",
        serialize_with = "super::scaled::serialize_count"
    )]
    pub age: u32,
}

impl Default for Composites {
    fn default() -> Self {
        Self {
            femininity: 0.0,
            mass: 0.0,
            body_fat: DEFAULT_BODY_FAT,
            age: DEFAULT_AGE,
        }
    }
}

impl Composites {
    /// The envelope `femininity` and `mass` clamp to (#160).
    ///
    /// Public so an editor's slider and the clamp cannot disagree about where
    /// the axis ends.
    #[must_use]
    pub fn signed_envelope() -> (f32, f32) {
        signed_envelope()
    }

    /// Clamps every axis into range.
    ///
    /// Idempotent, and non-finite axes take their documented default rather
    /// than the near bound — see `plan::sanitize_axis` for why the order
    /// substitute, clamp, quantise is the whole point.
    pub fn sanitize(&mut self) {
        let default = Self::default();
        self.femininity = sanitize_axis(self.femininity, default.femininity, signed_envelope());
        self.mass = sanitize_axis(self.mass, default.mass, signed_envelope());
        self.body_fat = sanitize_axis(self.body_fat, default.body_fat, BODY_FAT_RANGE);
        self.age = self.age.clamp(AGE_RANGE.0, AGE_RANGE.1);
    }

    /// Draws fresh values for the composites belonging to `category`.
    ///
    /// Mass and body fat are what [`Category::Build`] has always meant and the
    /// frame axis is what [`Category::Frame`] has always meant, so those two
    /// join the group they belong to. Age has its own bit, because it belongs
    /// to no other group and reaches all of them (#53).
    ///
    /// **These roll but do not yet reach any geometry.** The derivation that
    /// reads them is #164's and #166's; until then a rolled composite is stored
    /// intent and nothing more, and the body a seed builds is unchanged. The
    /// day it stops being unchanged is the day `GENERATOR_VERSION` moves, which
    /// is batched once at the end of the epic (#169) rather than per axis.
    pub fn reroll(&mut self, category: Category, rolls: &Rolls) {
        match category {
            Category::Build => {
                self.mass = rolls.shape("composites.mass", 0.0, 1.0, signed_envelope());
                self.body_fat =
                    rolls.shape("composites.bodyFat", DEFAULT_BODY_FAT, 0.07, BODY_FAT_RANGE);
            }
            Category::Frame => {
                self.femininity = rolls.shape("composites.femininity", 0.0, 1.0, signed_envelope());
            }
            Category::Age => {
                let years = rolls.shape(
                    "composites.age",
                    DEFAULT_AGE as f32,
                    10.0,
                    (AGE_RANGE.0 as f32, AGE_RANGE.1 as f32),
                );
                self.age = years.round() as u32;
            }
            Category::Stature
            | Category::Proportions
            | Category::Head
            | Category::Colouring
            | Category::Hair => {}
        }
    }

    /// Appends these axes to a share code.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        use super::put_span;
        put_span(out, self.femininity, signed_envelope());
        put_span(out, self.mass, signed_envelope());
        put_span(out, self.body_fat, BODY_FAT_RANGE);
        put_span(
            out,
            self.age as f32,
            (AGE_RANGE.0 as f32, AGE_RANGE.1 as f32),
        );
    }

    /// Reads these axes back from a share code.
    ///
    /// # Errors
    ///
    /// Returns [`super::PlanDecodeError::Truncated`] if the payload ends early.
    pub(crate) fn decode(bytes: &mut &[u8]) -> Result<Self, super::PlanDecodeError> {
        use super::take_span;
        let mut composites = Self {
            femininity: take_span(bytes, signed_envelope())?,
            mass: take_span(bytes, signed_envelope())?,
            body_fat: take_span(bytes, BODY_FAT_RANGE)?,
            age: take_span(bytes, (AGE_RANGE.0 as f32, AGE_RANGE.1 as f32))?.round() as u32,
        };
        composites.sanitize();
        Ok(composites)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_identity_anchor() {
        // Neutral on both shape axes and a middling adult on both physical
        // ones. The formulas of #164 and #166 are written so that THIS body is
        // the one the plan built before composites existed; if the default
        // moves, that anchor moves with it and every coefficient tuned against
        // it is out.
        let neutral = Composites::default();
        assert_eq!(neutral.femininity, 0.0);
        assert_eq!(neutral.mass, 0.0);
        assert_eq!(neutral.body_fat, DEFAULT_BODY_FAT);
        assert_eq!(neutral.age, DEFAULT_AGE);
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut composites = Composites {
            femininity: 99.0,
            mass: f32::NAN,
            body_fat: -4.0,
            age: 3,
        };
        composites.sanitize();
        assert_eq!(composites.femininity, signed_envelope().1);
        assert_eq!(composites.mass, 0.0, "a non-finite axis takes its default");
        assert_eq!(composites.body_fat, BODY_FAT_RANGE.0);
        assert_eq!(composites.age, AGE_RANGE.0);

        let once = composites;
        composites.sanitize();
        assert_eq!(once, composites, "sanitize must reach a fixpoint");
    }

    #[test]
    fn a_non_finite_axis_takes_its_documented_default_rather_than_zero() {
        // The #55 defect, which was invisible on fifteen of seventeen axes
        // because zero happened to be their neutral. `body_fat`'s neutral is
        // not zero, so this is the axis that would show it.
        for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut composites = Composites {
                body_fat: poison,
                ..Default::default()
            };
            composites.sanitize();
            assert_eq!(
                composites.body_fat, DEFAULT_BODY_FAT,
                "body_fat={poison} should fall back to its default, not to zero"
            );
        }
    }

    #[test]
    fn an_age_survives_the_wire_that_the_thousandths_encoder_would_have_eaten() {
        // The trap this axis is a count for: 40 years through
        // `scaled::serialize` is 40000 thousandths, past the i16 ceiling it
        // clamps at, and would come back as 32.767.
        let composites = Composites {
            age: 40,
            ..Default::default()
        };
        let json = serde_json::to_string(&composites).expect("serialises");
        assert!(
            json.contains("\"age\":40"),
            "age is written as years: {json}"
        );
        let back: Composites = serde_json::from_str(&json).expect("reads back");
        assert_eq!(back.age, 40);
    }

    #[test]
    fn a_partial_object_keeps_the_defaults_of_the_axes_it_omits() {
        // The container-level default, which is the whole of the #19-25 lesson:
        // a field-level default would give `body_fat` the TYPE's zero here.
        let partial: Composites =
            serde_json::from_str(r#"{"femininity": 500}"#).expect("reads back");
        assert_eq!(partial.femininity, 0.5);
        assert_eq!(partial.body_fat, DEFAULT_BODY_FAT);
        assert_eq!(partial.age, DEFAULT_AGE);
    }

    #[test]
    fn a_share_code_round_trip_keeps_a_look_recognisable() {
        let composites = Composites {
            femininity: 0.75,
            mass: -0.4,
            body_fat: 0.31,
            age: 52,
        };
        let mut payload = Vec::new();
        composites.encode(&mut payload);
        let mut slice = payload.as_slice();
        let back = Composites::decode(&mut slice).expect("reads back");

        // Codes are deliberately lossy — one byte an axis — so this is a
        // tolerance rather than an equality, as everywhere else in `code`.
        assert!((back.femininity - composites.femininity).abs() < 0.03);
        assert!((back.mass - composites.mass).abs() < 0.03);
        assert!((back.body_fat - composites.body_fat).abs() < 0.01);
        assert!(back.age.abs_diff(composites.age) <= 1);
    }

    #[test]
    fn rolling_one_category_leaves_the_others_alone() {
        let rolls = Rolls::new(7);
        let mut composites = Composites::default();
        composites.reroll(Category::Build, &rolls);
        assert_ne!(composites.mass, 0.0, "build draws mass");
        assert_eq!(
            composites.femininity, 0.0,
            "build must not disturb the frame axis"
        );
        assert_eq!(composites.age, DEFAULT_AGE, "build must not disturb age");
    }

    #[test]
    fn every_rolled_composite_is_already_in_range() {
        // `sanitize` runs after a re-roll anyway, but an axis whose draw needs
        // clamping is an axis whose distribution is wrong: the clamp would pile
        // mass on the bound.
        for seed in 0..400i64 {
            let rolls = Rolls::new(seed);
            let mut composites = Composites::default();
            for category in Category::ALL {
                composites.reroll(category, &rolls);
            }
            let rolled = composites;
            composites.sanitize();
            assert!(
                (rolled.body_fat - composites.body_fat).abs() < 1e-3,
                "seed {seed}: body_fat {} needed clamping",
                rolled.body_fat
            );
            assert!(
                (AGE_RANGE.0..=AGE_RANGE.1).contains(&rolled.age),
                "seed {seed}: age {} is outside the adult range",
                rolled.age
            );
        }
    }
}
