//! The strand mask: the one small image every hair card cuts its lock out of.
//!
//! **A card is a rectangle, and the picket fence was its end** (#340). Every
//! fringe, bob hem, beard rim and sideburn read as a row of full-width card tips,
//! and staggering and tapering the geometry made the sawtooth ragged rather than
//! gone. Cards are what low-poly hair is everywhere, and everywhere the answer is
//! the same: keep the card a rectangle and cut a lock out of it with an alpha
//! mask - one whose end frays into strands of different lengths, each coming to
//! a point, so no card ever stops on a straight line.
//!
//! **One image for every avatar, built once per process.** It depends on
//! nothing a record says, so it is a process-wide constant rather than a texture
//! per body: nothing new crosses the worker boundary, and a consumer uploads it
//! once. It is painted the first time anything asks for it, from constants, and
//! never changes afterwards - the one process-global this crate has, and an
//! immutable one.
//!
//! **Four lanes across, and a card spans exactly one of them**, picked by a hash
//! of its root in the loft (see [`lane_of`] and [`StrandMask::lane_span`]), so
//! cards side by side end in different locks without four textures. Along, a
//! card's `v` is the same share of its length its width and shade are asked at:
//! row 0 is the root, and the last row - the tip - is clear across every lane.
//!
//! **RGB is white; alpha is the lock.** The loft's vertex colour still carries
//! every tone, so a consumer that multiplies its base colour by this changes only
//! what is drawn, never what colour it is.
//!
//! **The same bytes on every target.** Painted from integer hashes and the
//! arithmetic IEEE-754 rounds exactly - addition, subtraction, multiplication,
//! division and comparison, never a sine or a power, whose last bits vary by
//! platform - so the checksum pinned in this module's tests holds on whatever
//! machine runs them.

use std::fmt;
use std::sync::LazyLock;

use glam::{Vec2, Vec3};

/// Texels along each side of the mask.
///
/// Four bytes a texel makes it 256 KiB, uploaded once for every avatar a
/// process draws.
///
/// Provenance: **from the issue** (#340).
pub const SIDE: u32 = 256;

/// How many locks the mask carries side by side.
///
/// Provenance: **from the issue** (#340): enough that cards side by side rarely
/// end in the same lock, few enough that each keeps the width to fray.
pub const LANES: u32 = 4;

/// Texels across each lane.
const LANE: u32 = SIDE / LANES;

/// Texels kept clear at each side of a lane.
///
/// A filtered sample at a card's very edge reaches half a texel past it, and a
/// lock that bled into the next lane would draw a sliver of its neighbour down
/// the side of every card.
///
/// Provenance: **derived** from a bilinear footprint, doubled.
const GUTTER: u32 = 2;

/// How many strands the fewest-stranded lane frays into.
///
/// Provenance: **tuned by render**.
const STRANDS: u32 = 5;

/// How many more strands than [`STRANDS`] a lane may fray into.
///
/// Provenance: **tuned by render**.
const STRANDS_SPREAD: u32 = 2;

/// How much a lane's strands vary in width, as a share of the width they would
/// each have if they were equal.
///
/// Provenance: **tuned by render**.
const UNEVEN: f32 = 0.6;

/// How much sooner a lock's outermost strands end than its middle ones, as a
/// share of the length.
///
/// **A lock comes to a soft point, not a square end**: a square end cut into
/// strands is still a line across the card where the strands stop.
///
/// Provenance: **tuned by render**.
const ROUND: f32 = 0.18;

/// How much sooner than that any one strand may end, as a share of the length.
///
/// **Staggered so the tips do not draw a line**: the lesson the fringe's
/// geometry learned first (#316), applied inside one card.
///
/// Provenance: **tuned by render**.
const RAGGED: f32 = 0.12;

/// Over how much of the length a strand narrows to its point.
///
/// Provenance: **tuned by render**.
const POINT: f32 = 0.10;

/// How far a lock's outer edge wanders in from true, at most, as a share of the
/// lane's width.
///
/// Provenance: **tuned by render**.
const WANDER: f32 = 0.04;

/// Rows between the points a wander is chosen at. Between them it is
/// interpolated, so an edge waves rather than frays.
///
/// Provenance: **tuned by render**.
const WANDER_ROWS: u32 = 16;

/// Salt that keeps [`lane_of`]'s hash apart from every other hash of a root.
const LANE_SALT: u32 = 0x5157_4D41;

/// The mask itself: RGBA8, square, row 0 at a card's root.
#[derive(Clone, PartialEq, Eq)]
pub struct StrandMask {
    /// Texels along each side.
    pub side: u32,
    /// How many locks it carries side by side.
    pub lanes: u32,
    /// RGBA8, row by row from the root to the tip. RGB is white everywhere.
    pub rgba: Vec<u8>,
}

impl fmt::Debug for StrandMask {
    // Not derived: a quarter of a megabyte of bytes is not a debugging aid.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StrandMask")
            .field("side", &self.side)
            .field("lanes", &self.lanes)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

impl StrandMask {
    /// The span of `u` a card laned into `lane` covers, edge to edge.
    ///
    /// Inset by the gutter at each side, so a card's edge samples its own lock's
    /// edge and never its neighbour's.
    #[must_use]
    pub fn lane_span(lane: u32) -> (f32, f32) {
        let lane = lane % LANES;
        (
            (lane * LANE + GUTTER) as f32 / SIDE as f32,
            ((lane + 1) * LANE - GUTTER) as f32 / SIDE as f32,
        )
    }

    /// How opaque the mask is at a texture coordinate, `0..=1`, from the nearest
    /// texel.
    #[must_use]
    pub fn alpha(&self, uv: Vec2) -> f32 {
        let texel = |at: f32| ((at.clamp(0.0, 1.0) * self.side as f32) as u32).min(self.side - 1);
        let at = ((texel(uv.y) * self.side + texel(uv.x)) * 4 + 3) as usize;
        f32::from(self.rgba[at]) / 255.0
    }
}

/// The strand mask, painted the first time anything asks for it.
#[must_use]
pub fn strand_mask() -> &'static StrandMask {
    static MASK: LazyLock<StrandMask> = LazyLock::new(paint);
    &MASK
}

/// Which lane of the mask the card rooted at `at` is cut from.
///
/// **Hashed from the root itself, not from a style's salt or the root stream.**
/// The scalp's salt would serve the scalp and nothing else: the brows, the lip,
/// the chin and the flanks loft through the same function with no salt of their
/// own, and every region's cards need a lane. And the stream has moved on by the
/// time a card is lofted, while a lane has to be a function of the card.
#[must_use]
pub fn lane_of(at: Vec3) -> u32 {
    hash(&[LANE_SALT, at.x.to_bits(), at.y.to_bits(), at.z.to_bits()]) % LANES
}

/// Paints the mask, every lane from its own lock.
fn paint() -> StrandMask {
    let mut rgba = vec![255u8; (SIDE * SIDE * 4) as usize];
    for lane in 0..LANES {
        let lock = Lock::of(lane);
        for row in 0..SIDE {
            for column in 0..LANE {
                let alpha = if (GUTTER..LANE - GUTTER).contains(&column) {
                    lock.coverage((column - GUTTER) as f32 + 0.5, row)
                } else {
                    0.0
                };
                let at = ((row * SIDE + lane * LANE + column) * 4 + 3) as usize;
                rgba[at] = (alpha * 255.0 + 0.5) as u8;
            }
        }
    }
    StrandMask {
        side: SIDE,
        lanes: LANES,
        rgba,
    }
}

/// One strand of a lock: where it lies across the lane and where it ends.
struct Strand {
    /// Its left and right edges across the lane's clear width, `0..=1`.
    from: f32,
    to: f32,
    /// The share of the length at which it has narrowed to nothing.
    end: f32,
}

/// One lane's lock: whole from the root, fraying at its end into strands that
/// touch until each of them ends.
struct Lock {
    lane: u32,
    strands: Vec<Strand>,
}

impl Lock {
    fn of(lane: u32) -> Self {
        let count = STRANDS + hash(&[lane, 1]) % (STRANDS_SPREAD + 1);
        let weights: Vec<f32> = (0..count)
            .map(|strand| 1.0 + UNEVEN * (unit(hash(&[lane, 2, strand])) - 0.5))
            .collect();
        let total: f32 = weights.iter().sum();
        let mut strands = Vec::with_capacity(weights.len());
        let mut from = 0.0f32;
        for (strand, weight) in (0..count).zip(&weights) {
            // The last strand meets the lock's edge exactly, whatever the sum
            // of the shares rounded to.
            let to = if strand + 1 == count {
                1.0
            } else {
                from + weight / total
            };
            let off_middle = (from + to - 1.0).abs();
            let end = 1.0 - ROUND * off_middle - RAGGED * unit(hash(&[lane, 3, strand]));
            strands.push(Strand { from, to, end });
            from = to;
        }
        Self { lane, strands }
    }

    /// How much of the texel centred `at` texels into the lane's clear width, on
    /// `row`, the lock covers: exactly, as the share of the texel its strands
    /// overlap, so a strand narrowing to nothing fades rather than ending on a
    /// half-covered thread.
    fn coverage(&self, at: f32, row: u32) -> f32 {
        let texels = (LANE - 2 * GUTTER) as f32;
        let along = row as f32 / (SIDE - 1) as f32;
        let last = self.strands.len() - 1;
        let mut covered = 0.0f32;
        for (index, strand) in self.strands.iter().enumerate() {
            let left_of_it = 1.0 - smooth((along - (strand.end - POINT)) / POINT);
            if left_of_it <= 0.0 {
                continue;
            }
            // Only the lock's two outer edges wander. An inner edge is where two
            // strands meet, and wandering it would open a slit down the lock.
            let inset_left = if index == 0 { self.wander(0, row) } else { 0.0 };
            let inset_right = if index == last {
                self.wander(1, row)
            } else {
                0.0
            };
            let centre = (strand.from + strand.to) * 0.5;
            let left = centre - (centre - strand.from - inset_left) * left_of_it;
            let right = centre + (strand.to - centre - inset_right) * left_of_it;
            covered += overlap(left * texels, right * texels, at);
        }
        covered.min(1.0)
    }

    /// How far one outer edge of this lock is inset at `row`, as a share of the
    /// lane.
    fn wander(&self, edge: u32, row: u32) -> f32 {
        let step = row / WANDER_ROWS;
        let from = unit(hash(&[self.lane, 4, edge, step]));
        let to = unit(hash(&[self.lane, 4, edge, step + 1]));
        let share = (row % WANDER_ROWS) as f32 / WANDER_ROWS as f32;
        WANDER * (from + (to - from) * share)
    }
}

/// How much of the texel centred at `at` the span `from..to` covers, all in
/// texels.
fn overlap(from: f32, to: f32, at: f32) -> f32 {
    (to.min(at + 0.5) - from.max(at - 0.5)).clamp(0.0, 1.0)
}

/// An integer hash of a few words: the same bits on every target.
fn hash(words: &[u32]) -> u32 {
    let mut hash = 0x9E37_79B9u32;
    for word in words {
        hash ^= *word;
        hash = hash.wrapping_mul(0x85EB_CA6B);
        hash ^= hash >> 13;
        hash = hash.wrapping_mul(0xC2B2_AE35);
        hash ^= hash >> 16;
    }
    hash
}

/// A hash as a number in `0..1`, exactly.
fn unit(hash: u32) -> f32 {
    (hash >> 8) as f32 / (1u32 << 24) as f32
}

/// Hermite smoothstep over `0..1`, clamped: exact arithmetic only.
fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FNV-1a over the bytes: enough to notice any one of them move.
    fn checksum(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |sum, byte| {
            (sum ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }

    /// The painted mask's checksum, blessed when the painting last changed on
    /// purpose.
    const PINNED: u64 = 0x1043_a619_d131_a62c;

    /// The alpha byte of one texel.
    fn alpha_at(mask: &StrandMask, column: u32, row: u32) -> u8 {
        mask.rgba[((row * mask.side + column) * 4 + 3) as usize]
    }

    #[test]
    fn the_mask_is_the_same_bytes_every_time_it_is_built() {
        // #340. Painted from integer hashes and exactly rounded arithmetic, so a
        // second painting is the same bytes as the first, the global is painted
        // once, and a checksum blessed here holds wherever this runs. A change to
        // the painting that is meant re-blesses PINNED; one that is not, fails.
        let painted = paint();
        assert!(
            painted == *strand_mask(),
            "two paintings of the mask disagree"
        );
        assert!(
            std::ptr::eq(strand_mask(), strand_mask()),
            "the mask was painted twice"
        );
        let sum = checksum(&painted.rgba);
        assert_eq!(
            sum, PINNED,
            "the mask's bytes moved: its checksum is {sum:#018x}"
        );
    }

    #[test]
    fn every_lock_ends_in_nothing() {
        // The whole point of the mask: no card ends on a straight line, because
        // at its tip there is no card left.
        let mask = strand_mask();
        for column in 0..SIDE {
            assert_eq!(
                alpha_at(mask, column, SIDE - 1),
                0,
                "column {column} is drawn at the tip"
            );
        }
    }

    #[test]
    fn every_lock_is_whole_until_it_frays() {
        // A card's root lies under other hair or on the scalp, and most of its
        // length is the lock itself; a mask that thinned it there would cut the
        // coverage the card exists to give. Only its outer edges may wander in,
        // so the middle four fifths of every lane is solid over the first three
        // fifths of the length. On the mask the #340 sheets were judged on, it is
        // whole to 66-73 percent.
        let mask = strand_mask();
        let clear = LANE - 2 * GUTTER;
        for lane in 0..LANES {
            let first = lane * LANE + GUTTER;
            for row in 0..SIDE * 3 / 5 {
                for column in first + clear / 10..first + clear - clear / 10 {
                    let alpha = alpha_at(mask, column, row);
                    assert_eq!(
                        alpha, 255,
                        "lane {lane} is {alpha} at column {column}, row {row}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_lock_frays_rather_than_stopping_on_a_line() {
        // #340's point, stated as the mask's own shape: across the middle of a
        // lane, the rows at which its columns stop being drawn are many and far
        // apart, so a card ends in strands of different lengths and never in a
        // cut across it. A lock ending square - every column stopping on one row
        // - is the picket fence back. On the mask the #340 sheets were judged on,
        // the middle columns stop over 54-72 rows, at 32-42 different rows a lane.
        let mask = strand_mask();
        let clear = LANE - 2 * GUTTER;
        for lane in 0..LANES {
            let first = lane * LANE + GUTTER;
            let mut stops: Vec<u32> = (first + clear / 10..first + clear - clear / 10)
                .map(|column| {
                    (0..SIDE)
                        .rev()
                        .find(|row| alpha_at(mask, column, *row) >= 128)
                        .unwrap_or(0)
                })
                .collect();
            stops.sort_unstable();
            let (earliest, latest) = (stops[0], stops[stops.len() - 1]);
            stops.dedup();
            assert!(
                latest - earliest >= SIDE / 10 && stops.len() >= 12,
                "lane {lane} stops over rows {earliest}..{latest}, at {} different rows",
                stops.len()
            );
        }
    }

    #[test]
    fn the_mask_is_white_so_the_vertices_keep_the_colour() {
        let mask = strand_mask();
        assert!(
            mask.rgba
                .chunks_exact(4)
                .all(|texel| texel[..3] == [255, 255, 255]),
            "the mask carries colour, which would tint every card it cuts"
        );
    }

    #[test]
    fn no_lane_reaches_its_neighbour() {
        // Every card samples its own lock and nothing of the next: the spans are
        // ordered and apart, and the gutters between them are clear root to tip.
        let mask = strand_mask();
        let mut last = 0.0f32;
        for lane in 0..LANES {
            let (from, to) = StrandMask::lane_span(lane);
            assert!(
                from > last && to > from && to < 1.0,
                "lane {lane} spans {from}..{to}"
            );
            last = to;
            for row in 0..SIDE {
                for column in (lane * LANE..lane * LANE + GUTTER)
                    .chain((lane + 1) * LANE - GUTTER..(lane + 1) * LANE)
                {
                    let alpha = alpha_at(mask, column, row);
                    assert_eq!(alpha, 0, "lane {lane}'s gutter is drawn at row {row}");
                }
            }
        }
    }
}
