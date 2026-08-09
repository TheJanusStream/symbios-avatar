//! Deciding how a jaw moves when a body talks.
//!
//! The sibling of [`super::blink`], for the joint #152 gave a territory: the
//! mandible region swings on the jaw pivot, and this decides how far, when.
//!
//! Speech is not a wave. A jaw talking is **utterances** — runs of a few
//! syllables — separated by silences, and inside an utterance the jaw never
//! quite closes: each syllable falls to a partial dip and rises again, because
//! the mouth is still holding the word. A sine on a timer reads as a puppet;
//! so, as with a blink, every duration and every excursion here is drawn from
//! a seeded generator, and a recording is reproducible.
//!
//! What this returns is an **angle in radians** for the pivot, not a fraction:
//! the full conversational open is anatomy, so it lives here with the rest of
//! the mouth's numbers rather than being re-picked by every consumer.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Tuning for [`Talk`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TalkConfig {
    /// Shortest silence between utterances, in seconds.
    pub min_pause: f32,
    /// Longest silence between utterances, in seconds.
    pub max_pause: f32,
    /// Fewest syllables an utterance carries.
    pub min_syllables: u32,
    /// Most syllables an utterance carries.
    pub max_syllables: u32,
    /// Quickest a syllable opens the jaw, in seconds.
    pub min_open_time: f32,
    /// Slowest it opens, in seconds.
    pub max_open_time: f32,
    /// Quickest it closes again, in seconds.
    pub min_close_time: f32,
    /// Slowest it closes, in seconds.
    pub max_close_time: f32,
    /// The full conversational open, in radians of pivot rotation.
    ///
    /// Talking is a small motion: a 20° open is a yawn, and `render --jaw 20`
    /// exists to inspect one. Conversation swings the jaw a fraction of that,
    /// with the peak drawn per syllable below this ceiling.
    pub open: f32,
    /// The quietest syllable's peak, as a share of `open`.
    pub min_peak: f32,
    /// How far the jaw falls back between syllables, as a share of `open`.
    ///
    /// The dip is what makes this speech rather than chatter: a jaw that
    /// returns to zero between syllables is snapping shut on every beat.
    pub dip: f32,
}

impl Default for TalkConfig {
    fn default() -> Self {
        // Syllable rate falls out of the open and close draws: 0.05–0.09 s up
        // and 0.06–0.12 s down is 4.8–9.1 syllables a second, spanning the
        // 5–8 Hz of ordinary speech with the slow tail for emphasis.
        Self {
            min_pause: 0.35,
            max_pause: 1.60,
            min_syllables: 3,
            max_syllables: 10,
            min_open_time: 0.05,
            max_open_time: 0.09,
            min_close_time: 0.06,
            max_close_time: 0.12,
            open: 12f32.to_radians(),
            min_peak: 0.45,
            dip: 0.25,
        }
    }
}

/// What the jaw is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Shut, counting down the silence to the next utterance.
    Pausing(f32),
    /// A syllable carrying the jaw from one angle toward another.
    ///
    /// `left` is how many syllables follow in this utterance; `falling` is
    /// whether this is the closing half of a syllable, whose end either dips
    /// into the next syllable or — on the last — returns to zero.
    Moving {
        elapsed: f32,
        time: f32,
        from: f32,
        to: f32,
        left: u32,
        falling: bool,
    },
}

/// Drives a body's jaw through speech.
#[derive(Clone, Debug)]
pub struct Talk {
    config: TalkConfig,
    stage: Stage,
    rng: Pcg64Mcg,
}

impl Talk {
    /// A talker with the given configuration and seed.
    ///
    /// Seeded rather than drawing from the thread's generator, so a recording
    /// of a body is reproducible and a test can assert on it — the same
    /// contract as [`super::Blink`].
    #[must_use]
    pub fn new(config: TalkConfig, seed: u64) -> Self {
        let mut rng = Pcg64Mcg::seed_from_u64(seed);
        let first = draw(&mut rng, config.min_pause, config.max_pause);
        Self {
            config,
            stage: Stage::Pausing(first),
            rng,
        }
    }

    /// A talker with the default configuration.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self::new(TalkConfig::default(), seed)
    }

    /// Advances time and returns the pivot angle the jaw now holds, in radians.
    pub fn advance(&mut self, dt: f32) -> f32 {
        let mut remaining = dt.max(0.0);

        // A loop rather than a single step, for the reason `Blink` loops: a
        // long frame can carry the jaw through several syllables, and taking
        // only one would strand it mid-word.
        while remaining > 0.0 {
            match self.stage {
                Stage::Pausing(wait) => {
                    if remaining < wait {
                        self.stage = Stage::Pausing(wait - remaining);
                        break;
                    }
                    remaining -= wait;
                    let syllables = self.rng.random_range(
                        self.config.min_syllables
                            ..=self.config.max_syllables.max(self.config.min_syllables),
                    );
                    self.stage = self.rise(0.0, syllables.saturating_sub(1));
                }
                Stage::Moving {
                    elapsed,
                    time,
                    from,
                    to,
                    left,
                    falling,
                } => {
                    let step = (time - elapsed).max(0.0);
                    if remaining < step {
                        self.stage = Stage::Moving {
                            elapsed: elapsed + remaining,
                            time,
                            from,
                            to,
                            left,
                            falling,
                        };
                        break;
                    }
                    remaining -= step;
                    self.stage = if !falling {
                        // The syllable's closing half: fall to the dip, or all
                        // the way shut if this was the utterance's last.
                        let floor = if left > 0 {
                            self.config.dip * self.config.open
                        } else {
                            0.0
                        };
                        Stage::Moving {
                            elapsed: 0.0,
                            time: draw(
                                &mut self.rng,
                                self.config.min_close_time,
                                self.config.max_close_time,
                            ),
                            from: to,
                            to: floor,
                            left,
                            falling: true,
                        }
                    } else if left > 0 {
                        self.rise(to, left - 1)
                    } else {
                        Stage::Pausing(draw(
                            &mut self.rng,
                            self.config.min_pause,
                            self.config.max_pause,
                        ))
                    };
                }
            }
        }

        self.angle()
    }

    /// The pivot angle the jaw holds right now, in radians.
    #[must_use]
    pub fn angle(&self) -> f32 {
        match self.stage {
            Stage::Pausing(_) => 0.0,
            Stage::Moving {
                elapsed,
                time,
                from,
                to,
                ..
            } => from + (to - from) * ease(fraction(elapsed, time)),
        }
    }

    /// Whether an utterance is under way.
    #[must_use]
    pub fn is_talking(&self) -> bool {
        !matches!(self.stage, Stage::Pausing(_))
    }

    /// Starts an utterance now, whatever the silence had left.
    ///
    /// For the moments a body should speak because something happened rather
    /// than because time passed.
    pub fn trigger(&mut self) {
        if !self.is_talking() {
            let syllables = self.rng.random_range(
                self.config.min_syllables
                    ..=self.config.max_syllables.max(self.config.min_syllables),
            );
            self.stage = self.rise(0.0, syllables.saturating_sub(1));
        }
    }

    /// The opening half of a syllable, from `from`, with `left` to follow.
    fn rise(&mut self, from: f32, left: u32) -> Stage {
        let peak = self.config.open * draw(&mut self.rng, self.config.min_peak.min(1.0), 1.0);
        Stage::Moving {
            elapsed: 0.0,
            time: draw(
                &mut self.rng,
                self.config.min_open_time,
                self.config.max_open_time,
            ),
            from,
            to: peak,
            left,
            falling: false,
        }
    }
}

/// How far through a stage of the given length.
fn fraction(elapsed: f32, length: f32) -> f32 {
    if length <= f32::EPSILON {
        1.0
    } else {
        (elapsed / length).clamp(0.0, 1.0)
    }
}

/// Smooths the ends of a syllable's travel, as [`super::blink`] smooths a lid's.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Draws a duration or share from the closed range `low..=high`.
fn draw(rng: &mut Pcg64Mcg, low: f32, high: f32) -> f32 {
    let low = low.max(0.0);
    let high = high.max(low + f32::EPSILON);
    rng.random_range(low..=high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_starts_with_its_mouth_shut() {
        let talk = Talk::seeded(1);
        assert_eq!(talk.angle(), 0.0);
        assert!(!talk.is_talking());
    }

    #[test]
    fn the_jaw_stays_within_its_travel() {
        let mut talk = Talk::seeded(7);
        let ceiling = TalkConfig::default().open + f32::EPSILON;
        for _ in 0..8000 {
            let angle = talk.advance(1.0 / 60.0);
            assert!(
                (0.0..=ceiling).contains(&angle),
                "the jaw left its travel at {angle}"
            );
        }
    }

    #[test]
    fn an_utterance_opens_the_jaw_and_silence_shuts_it() {
        let mut talk = Talk::seeded(3);
        let mut opened = false;
        let mut shut_again = false;
        for _ in 0..8000 {
            let angle = talk.advance(1.0 / 240.0);
            if angle > 0.5 * TalkConfig::default().open {
                opened = true;
            }
            if opened && !talk.is_talking() && angle == 0.0 {
                shut_again = true;
                break;
            }
        }
        assert!(opened, "the jaw never opened");
        assert!(shut_again, "the jaw never came back to rest");
    }

    #[test]
    fn the_jaw_does_not_snap_shut_between_syllables() {
        // The dip is what separates speech from chatter: inside an utterance
        // the jaw falls back only partway before the next syllable lifts it.
        let config = TalkConfig::default();
        let mut talk = Talk::new(config, 11);
        let step = 1.0 / 480.0;
        // Find an utterance and watch it end to end.
        while !talk.is_talking() {
            talk.advance(step);
        }
        let mut minima_mid_utterance = Vec::new();
        let (mut previous, mut before_that) = (0.0f32, 0.0f32);
        while talk.is_talking() {
            let angle = talk.advance(step);
            if before_that > previous && angle > previous && previous > 0.0 {
                minima_mid_utterance.push(previous);
            }
            before_that = previous;
            previous = angle;
        }
        // A one-syllable utterance has no interior minimum to show; draw again
        // on a seed whose first utterance is longer rather than weakening the
        // assertion. Seed 11's first utterance carries interior minima.
        assert!(
            !minima_mid_utterance.is_empty(),
            "the first utterance had no interior minimum to check"
        );
        for dip in minima_mid_utterance {
            assert!(
                dip > 0.5 * config.dip * config.open,
                "the jaw fell to {dip} between syllables, against a dip floor of {}",
                config.dip * config.open
            );
        }
    }

    #[test]
    fn utterances_come_at_irregular_intervals() {
        // A fixed rhythm reads as a metronome, which is most of why this is
        // not simply a timer.
        let mut talk = Talk::seeded(23);
        let step = 1.0 / 120.0;
        let mut gaps = Vec::new();
        let mut since = 0.0;
        let mut was_talking = false;
        for _ in 0..60_000 {
            talk.advance(step);
            since += step;
            let now = talk.is_talking();
            if now && !was_talking {
                gaps.push(since);
                since = 0.0;
            }
            was_talking = now;
        }
        assert!(
            gaps.len() > 40,
            "only {} utterances in 8 minutes",
            gaps.len()
        );
        let spread = gaps.iter().fold(f32::MIN, |a, b| a.max(*b))
            - gaps.iter().fold(f32::MAX, |a, b| a.min(*b));
        assert!(spread > 0.4, "silences varied by only {spread:.2}s");
    }

    #[test]
    fn a_long_frame_does_not_strand_the_jaw_open() {
        // A single-step implementation leaves the jaw wherever a dropped frame
        // caught it — mid-syllable, half open, mouth frozen.
        let mut talk = Talk::seeded(9);
        let angle = talk.advance(30.0);
        let open = TalkConfig::default().open;
        assert!(
            !(0.3 * open..0.95 * open).contains(&angle),
            "a 30-second step left the jaw at {angle}"
        );
    }

    #[test]
    fn an_utterance_can_be_asked_for() {
        let mut talk = Talk::seeded(2);
        assert!(!talk.is_talking());
        talk.trigger();
        assert!(talk.is_talking());

        // Asking again mid-utterance does not restart it.
        talk.advance(0.03);
        let partway = talk.angle();
        talk.trigger();
        assert_eq!(talk.angle(), partway);
    }

    #[test]
    fn talking_is_reproducible_for_a_seed() {
        let run = |seed: u64| {
            let mut talk = Talk::seeded(seed);
            (0..1200)
                .map(|_| talk.advance(1.0 / 60.0))
                .collect::<Vec<f32>>()
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(43));
    }
}
