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
//! made that no formula predicted.
//!
//! ## Why these live on the record rather than on a body plan
//!
//! Every other parameter struct sits with its one consumer — `SkinParams` in
//! `texture`, `HairRecord` in `hair`, `HumanoidParams` in `plan`. These have
//! three: the cage derives from them, the skull reads them, and so does the
//! skin — muscle definition at low body fat, creases with age.
//! Putting them inside an [`super::Archetype`] variant would make every one of
//! those consumers match on the variant to reach a number that has nothing to
//! do with which body plan is in use.
//!
//! ## Physical axes are not stretched, and that is a departure
//!
//! Every *shape* axis in this crate carries the exploration envelope — its
//! conservative range tripled about its own default — because going past
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
/// fills, which is the first place in this crate where two composites
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
/// neutral, the formulas must reproduce the plan's own base body unchanged.
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

/// The age past which a body starts to change, in years.
///
/// **Below this the axis does nothing at all, deliberately.** An eighteen- and
/// a twenty-eight-year-old differ in almost nothing this crate can draw: peak
/// bone mass, peak muscle mass and peak stature are all reached inside that
/// band, so the honest formula over it is the identity. That also happens to be
/// what the identity anchor needs — [`DEFAULT_AGE`] is under this pivot, so a
/// body no one has aged is bit-identical to the plan's own base body.
pub const AGE_PIVOT: u32 = 30;

/// The exploration envelope of the two signed composites.
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
    /// female 0.129 of stature, "the honest neutral until the frame axis
    /// carries that difference". This is that axis, and that one coefficient is
    /// where most of its travel will be.
    ///
    /// One axis for the whole body, read by the frame, by the fat
    /// distribution and by the skull — because a body whose
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
    /// could contradict each other.
    #[serde(with = "super::scaled")]
    pub mass: f32,
    /// What share of the body is fat, as a fraction within
    /// [`BODY_FAT_RANGE`].
    ///
    /// A fraction rather than an abstract `0..1` slider, so the formulas can be
    /// written against the thresholds the eye actually reads — definition and
    /// vascularity appear at the lean end, softening and a filled waist at the
    /// heavy end — and so the numbers in them can be sourced rather than tuned.
    /// It reaches the skin as well as the shape.
    #[serde(with = "super::scaled")]
    pub body_fat: f32,
    /// How old the body is, in whole years within [`AGE_RANGE`].
    ///
    /// **Whole years, and NOT through the thousandths encoder that every other
    /// axis uses.** A count is the honest representation — nothing wants a
    /// fractional year — and an age in years is the one axis whose natural
    /// unit is large: forty years is 40000 thousandths, the exact shape a
    /// narrowing writer would truncate in silence. See
    /// `plan::scaled::serialize` for the ceiling that encoder refuses to have.
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
    /// The envelope `femininity` and `mass` clamp to.
    ///
    /// Public so an editor's slider and the clamp cannot disagree about where
    /// the axis ends.
    #[must_use]
    pub fn signed_envelope() -> (f32, f32) {
        signed_envelope()
    }

    /// How far into the changes of age this body is: `0` at [`AGE_PIVOT`] and
    /// below, `1` at the top of [`AGE_RANGE`].
    ///
    /// **One ramp for the whole crate**, read by the body plan, by the skull
    /// and by the skin, so the trunk that settles, the lip that thins and the
    /// crease that deepens are all the same body getting older rather than
    /// three timetables that can disagree.
    ///
    /// **The square is fitted rather than chosen for smoothness.** Nothing age
    /// does to a body is linear in years — stature holds through the thirties
    /// and then falls away, muscle goes at about a percent a year only after
    /// sixty — so a linear ramp would age a forty-five-year-old about twice as
    /// much as life does. Squaring it fits the one curve here that is measured
    /// end to end. Against the cumulative height loss the Baltimore
    /// Longitudinal Study reports for men, and a settle sized to its 5 cm at
    /// eighty:
    ///
    /// ```text
    ///   age    this ramp   settle    reported
    ///    40      0.04       0.2 cm     ~0.2
    ///    55      0.25       1.3 cm     ~1.5
    ///    70      0.64       3.2 cm      3.0
    ///    80      1.00       5.0 cm      5.0
    /// ```
    ///
    /// Provenance: **derived from a looked-up curve** — Sorkin, Muller
    /// and Andres, *Longitudinal change in height of men and women*, Am J
    /// Epidemiol 1999, whose cumulative losses are the right-hand column.
    #[must_use]
    pub fn ageing(&self) -> f32 {
        let past = self.age.saturating_sub(AGE_PIVOT) as f32;
        let span = (AGE_RANGE.1 - AGE_PIVOT) as f32;
        let t = (past / span).clamp(0.0, 1.0);
        t * t
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
    /// to no other group and reaches all of them.
    ///
    /// **These are drawn before anything else on the record, and two of them
    /// read the ones drawn first.** `AvatarRecord::reroll` calls this for
    /// `Frame`, then `Age`, then `Build`, in that order and not in
    /// `Category::ALL`'s, because [`Self::body_fat`]'s draw reads both of the
    /// first two. A locked category is skipped and its STORED value is what the
    /// later draws read, which is what makes a lock mean "keep this and build
    /// around it".
    pub fn reroll(&mut self, category: Category, rolls: &Rolls) {
        match category {
            Category::Build => {
                self.mass = rolls.shape("composites.mass", 0.0, 1.0, signed_envelope());
                // **The correlated draw the epic was raised for** (#161, #169).
                // Independent draws made "heavy at 4% body fat" as likely as
                // any other body, and the two axes are separable BY DESIGN —
                // that separation is #164's point and this does not touch it.
                // What moves is only where the draw is CENTRED: a heavy roll
                // is fat by default and lean by choice, which is the way round
                // life has it.
                //
                // Three terms, each a real population fact:
                //
                // ```text
                //   mass       +0.09 per unit   BMI 22 → 33 across the axis, and
                //                               fat fraction tracks BMI hard
                //   femininity +0.05 per unit   the ~10-point difference between
                //                               the sexes at equal fitness
                //   age        +0.05 per ramp   fat fraction rises over a life
                //                               even at a held weight
                // ```
                //
                // Sigma is unchanged, so the SPREAD around the centre is what
                // it always was; only the centre moves.
                //
                // Provenance: **looked up**, all three (#169).
                let centre = DEFAULT_BODY_FAT
                    + 0.09 * self.mass
                    + 0.05 * self.femininity
                    + 0.05 * self.ageing();
                self.body_fat = rolls.shape("composites.bodyFat", centre, 0.07, BODY_FAT_RANGE);
            }
            Category::Frame => {
                self.femininity = rolls.shape("composites.femininity", 0.0, 1.0, signed_envelope());
            }
            Category::Age => {
                // **Widened, and the old prior could not reach the axis it
                // fed** (#167, #169). At the shipped mean of 28 with sigma 10,
                // six of the first eight seeds drew an age under
                // [`AGE_PIVOT`] — where the age axis is deliberately the
                // identity — so a randomise button produced a population of
                // twenty-somethings and none of #167's work was reachable from
                // it. That is a defect in the PRIOR and not in the ramp: the
                // ramp is fitted to a measured curve and the pivot is where a
                // body starts to change.
                //
                // 38 with sigma 15 spans the adult population instead of its
                // youngest decade: the median lands in the late thirties, a
                // third of rolls clear fifty, and eighteen and eighty are both
                // reachable rather than four sigma away.
                //
                // Provenance: **chosen against the adult age distribution**,
                // then measured over 400 seeds (#169).
                let years = rolls.shape(
                    "composites.age",
                    38.0,
                    15.0,
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
    fn the_ramp_of_age_is_flat_under_its_pivot_and_full_at_the_top() {
        // The identity anchor: the default body is under the pivot, so every
        // formula that reads this one gets zero and reproduces the body the
        // plan built before the axis existed (#161).
        for age in AGE_RANGE.0..=AGE_PIVOT {
            let composites = Composites {
                age,
                ..Default::default()
            };
            assert_eq!(composites.ageing(), 0.0, "age {age} must move nothing");
        }

        let oldest = Composites {
            age: AGE_RANGE.1,
            ..Default::default()
        };
        assert_eq!(oldest.ageing(), 1.0);

        // Monotone, and the fitted square rather than a line — the table on
        // `ageing` is what these two figures come from.
        let at = |age| {
            Composites {
                age,
                ..Default::default()
            }
            .ageing()
        };
        assert!(at(55) < at(70) && at(70) < at(80));
        assert!((at(55) - 0.25).abs() < 1e-3, "{}", at(55));
        assert!((at(70) - 0.64).abs() < 1e-3, "{}", at(70));
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
