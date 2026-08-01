//! Deciding when to blink.
//!
//! A face that never blinks is unsettling in a way people notice before they can
//! say why, and one that blinks on a fixed timer is worse — the regularity reads
//! as mechanical. So the interval is drawn at random, and blinks occasionally
//! come in pairs, which is what real ones do.
//!
//! The close is faster than the open. A blink that is symmetric in time looks
//! like a wink held slightly too long; roughly 70 ms shut and 130 ms back open is
//! what an eye actually does.

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Tuning for [`Blink`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlinkConfig {
    /// Shortest wait between blinks, in seconds.
    pub min_interval: f32,
    /// Longest wait between blinks, in seconds.
    pub max_interval: f32,
    /// How long the lids take to shut, in seconds.
    pub close_time: f32,
    /// How long they take to open again, in seconds.
    pub open_time: f32,
    /// Chance a blink is followed straight away by another.
    pub double_chance: f64,
}

impl Default for BlinkConfig {
    fn default() -> Self {
        Self {
            min_interval: 2.0,
            max_interval: 6.0,
            close_time: 0.07,
            open_time: 0.13,
            double_chance: 0.15,
        }
    }
}

/// What the lids are doing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Open, counting down to the next blink.
    Waiting(f32),
    /// Shutting.
    Closing(f32),
    /// Opening again; the flag carries whether another blink follows.
    Opening(f32, bool),
}

/// Drives a body's blinking.
#[derive(Clone, Debug)]
pub struct Blink {
    config: BlinkConfig,
    stage: Stage,
    rng: Pcg64Mcg,
}

impl Blink {
    /// A blinker with the given configuration and seed.
    ///
    /// Seeded rather than drawing from the thread's generator, so a recording of
    /// a body is reproducible and a test can assert on it.
    #[must_use]
    pub fn new(config: BlinkConfig, seed: u64) -> Self {
        let mut rng = Pcg64Mcg::seed_from_u64(seed);
        let first = draw_interval(&config, &mut rng);
        Self {
            config,
            stage: Stage::Waiting(first),
            rng,
        }
    }

    /// A blinker with the default configuration.
    #[must_use]
    pub fn seeded(seed: u64) -> Self {
        Self::new(BlinkConfig::default(), seed)
    }

    /// Advances time and returns how shut the lids now are, in `0..=1`.
    pub fn advance(&mut self, dt: f32) -> f32 {
        let mut left = dt.max(0.0);

        // A loop rather than a single step: a long frame can carry the lids
        // through more than one stage, and skipping the remainder would stall a
        // blink half-shut.
        while left > 0.0 {
            match self.stage {
                Stage::Waiting(remaining) => {
                    if left < remaining {
                        self.stage = Stage::Waiting(remaining - left);
                        break;
                    }
                    left -= remaining;
                    self.stage = Stage::Closing(0.0);
                }
                Stage::Closing(elapsed) => {
                    let step = (self.config.close_time - elapsed).max(0.0);
                    if left < step {
                        self.stage = Stage::Closing(elapsed + left);
                        break;
                    }
                    left -= step;
                    let again = self
                        .rng
                        .random_bool(self.config.double_chance.clamp(0.0, 1.0));
                    self.stage = Stage::Opening(0.0, again);
                }
                Stage::Opening(elapsed, again) => {
                    let step = (self.config.open_time - elapsed).max(0.0);
                    if left < step {
                        self.stage = Stage::Opening(elapsed + left, again);
                        break;
                    }
                    left -= step;
                    self.stage = if again {
                        // A double blink follows immediately, no pause.
                        Stage::Closing(0.0)
                    } else {
                        Stage::Waiting(draw_interval(&self.config, &mut self.rng))
                    };
                }
            }
        }

        self.closure()
    }

    /// How shut the lids are right now, in `0..=1`.
    #[must_use]
    pub fn closure(&self) -> f32 {
        match self.stage {
            Stage::Waiting(_) => 0.0,
            Stage::Closing(elapsed) => ease(fraction(elapsed, self.config.close_time)),
            Stage::Opening(elapsed, _) => 1.0 - ease(fraction(elapsed, self.config.open_time)),
        }
    }

    /// Whether the lids are anywhere other than fully open.
    #[must_use]
    pub fn is_blinking(&self) -> bool {
        !matches!(self.stage, Stage::Waiting(_))
    }

    /// Starts a blink now, whatever the timer said.
    ///
    /// For the moments a body should blink because something happened rather
    /// than because time passed.
    pub fn trigger(&mut self) {
        if !self.is_blinking() {
            self.stage = Stage::Closing(0.0);
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

/// Smooths the ends of a lid's travel, so it does not start or stop abruptly.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Draws the wait until the next blink.
fn draw_interval(config: &BlinkConfig, rng: &mut Pcg64Mcg) -> f32 {
    let low = config.min_interval.max(0.0);
    let high = config.max_interval.max(low + f32::EPSILON);
    rng.random_range(low..=high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_starts_with_its_eyes_open() {
        let blink = Blink::seeded(1);
        assert_eq!(blink.closure(), 0.0);
        assert!(!blink.is_blinking());
    }

    #[test]
    fn the_lids_stay_within_their_travel() {
        let mut blink = Blink::seeded(7);
        for _ in 0..4000 {
            let closure = blink.advance(1.0 / 60.0);
            assert!(
                (0.0..=1.0).contains(&closure),
                "closure left its range at {closure}"
            );
        }
    }

    #[test]
    fn a_blink_shuts_the_eyes_and_opens_them_again() {
        let mut blink = Blink::seeded(3);
        let mut shut = false;
        let mut reopened = false;

        for _ in 0..2000 {
            let closure = blink.advance(1.0 / 240.0);
            if closure > 0.95 {
                shut = true;
            }
            if shut && closure < 0.05 {
                reopened = true;
                break;
            }
        }
        assert!(shut, "the eyes never shut");
        assert!(reopened, "the eyes never opened again");
    }

    #[test]
    fn the_close_is_quicker_than_the_open() {
        // A symmetric blink reads as a wink held too long.
        let config = BlinkConfig::default();
        assert!(config.close_time < config.open_time);

        let mut blink = Blink::new(config, 11);
        let step = 1.0 / 480.0;
        // Advance to the start of a blink.
        while !blink.is_blinking() {
            blink.advance(step);
        }
        let mut closing = 0;
        while blink.closure() < 0.999 {
            blink.advance(step);
            closing += 1;
        }
        let mut opening = 0;
        while blink.closure() > 0.001 {
            blink.advance(step);
            opening += 1;
        }
        assert!(
            opening > closing,
            "closing took {closing} steps and opening {opening}"
        );
    }

    #[test]
    fn blinks_come_at_irregular_intervals() {
        // A fixed rhythm reads as mechanical, which is most of why this is not
        // simply a timer.
        let mut blink = Blink::seeded(23);
        let step = 1.0 / 120.0;
        let mut gaps = Vec::new();
        let mut since = 0.0;
        let mut was_blinking = false;

        for _ in 0..40_000 {
            blink.advance(step);
            since += step;
            let now = blink.is_blinking();
            if now && !was_blinking {
                gaps.push(since);
                since = 0.0;
            }
            was_blinking = now;
        }

        assert!(gaps.len() > 20, "only {} blinks in 5 minutes", gaps.len());
        let spread = gaps.iter().fold(f32::MIN, |a, b| a.max(*b))
            - gaps.iter().fold(f32::MAX, |a, b| a.min(*b));
        assert!(spread > 1.0, "intervals varied by only {spread:.2}s");
    }

    #[test]
    fn blinks_sometimes_come_in_pairs() {
        let mut blink = Blink::new(
            BlinkConfig {
                double_chance: 1.0,
                ..Default::default()
            },
            5,
        );
        let step = 1.0 / 240.0;
        while !blink.is_blinking() {
            blink.advance(step);
        }
        // Run through one full blink; with certainty of doubling, another
        // follows without the eyes resting open.
        let mut opened_fully = false;
        for _ in 0..200 {
            blink.advance(step);
            if !blink.is_blinking() {
                opened_fully = true;
                break;
            }
        }
        assert!(
            !opened_fully,
            "a doubled blink should not rest open between"
        );
    }

    #[test]
    fn a_long_frame_does_not_strand_the_lids_half_shut() {
        // A single-step implementation leaves the lids wherever a dropped frame
        // happened to catch them.
        let mut blink = Blink::seeded(9);
        let closure = blink.advance(30.0);
        assert!(
            !(0.05..0.95).contains(&closure),
            "a 30-second step left the lids at {closure}"
        );
    }

    #[test]
    fn a_blink_can_be_asked_for() {
        let mut blink = Blink::seeded(2);
        assert!(!blink.is_blinking());
        blink.trigger();
        assert!(blink.is_blinking());

        // Asking again mid-blink does not restart it.
        blink.advance(0.05);
        let partway = blink.closure();
        blink.trigger();
        assert_eq!(blink.closure(), partway);
    }

    #[test]
    fn blinking_is_reproducible_for_a_seed() {
        let run = |seed: u64| {
            let mut blink = Blink::seeded(seed);
            (0..600)
                .map(|_| blink.advance(1.0 / 60.0))
                .collect::<Vec<f32>>()
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(43));
    }
}
