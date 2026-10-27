//! The fields a coat's marks are cut from (#368): noise and cells sampled in
//! body space, so a mark runs over a chart's seam without knowing it is
//! there, as the skin's freckles do.
//!
//! Every field is seeded from the coat's own `marks`, so a coat's spots and
//! patches are its own and survive the wire, and every frequency is in cycles
//! per back height, so a cat's stripes and a horse's are the same coat at two
//! sizes.

use glam::Vec3;
use noise::{NoiseFn, Simplex};

/// The fields one coat's marks are cut from.
pub(super) struct Fields {
    /// Broad patches: piebald, merle, tortie.
    patches: Simplex,
    /// The wavering every stripe and ring is warped by.
    warp: Simplex,
    /// Fine grain: ticking, roan.
    grain: Simplex,
    /// The seed the cells are laid out by.
    cells: u32,
    /// The back's height, which every frequency is measured in.
    height: f32,
}

impl Fields {
    /// The fields of the coat seeded `marks`, on a back `height` high.
    pub(super) fn new(marks: u16, height: f32) -> Self {
        let seed = u32::from(marks);
        Self {
            patches: Simplex::new(seed.wrapping_mul(2_654_435_761) ^ 0x5eed_0001),
            warp: Simplex::new(seed.wrapping_mul(2_246_822_519) ^ 0x5eed_0002),
            grain: Simplex::new(seed.wrapping_mul(3_266_489_917) ^ 0x5eed_0003),
            cells: seed.wrapping_mul(668_265_263) ^ 0x5eed_0004,
            height: height.max(f32::EPSILON),
        }
    }

    /// `at` in back heights: the unit every frequency here is in.
    fn scaled(&self, at: Vec3) -> Vec3 {
        at / self.height
    }

    /// Broad patches at `frequency` per back height, `-1..=1`.
    pub(super) fn patches(&self, at: Vec3, frequency: f32) -> f32 {
        sample(&self.patches, self.scaled(at) * frequency)
    }

    /// The wavering at `at`, `-1..=1`.
    pub(super) fn warp(&self, at: Vec3, frequency: f32) -> f32 {
        sample(&self.warp, self.scaled(at) * frequency)
    }

    /// Fine grain at `frequency` per back height, `-1..=1`.
    pub(super) fn grain(&self, at: Vec3, frequency: f32) -> f32 {
        sample(&self.grain, self.scaled(at) * frequency)
    }

    /// The distances from `at` to the nearest and the second nearest cell
    /// centre, `frequency` cells per back height, in cell widths: the field
    /// spots, rosettes and dapples are cut from.
    pub(super) fn cells(&self, at: Vec3, frequency: f32) -> (f32, f32) {
        let point = self.scaled(at) * frequency;
        let home = point.floor();
        let (mut nearest, mut next) = (f32::MAX, f32::MAX);
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let cell = home + Vec3::new(x as f32, y as f32, z as f32);
                    let centre = cell + jitter(cell, self.cells);
                    let distance = point.distance(centre);
                    if distance < nearest {
                        next = nearest;
                        nearest = distance;
                    } else if distance < next {
                        next = distance;
                    }
                }
            }
        }
        (nearest, next)
    }
}

/// A noise field at `at`, `-1..=1`.
fn sample(field: &Simplex, at: Vec3) -> f32 {
    field.get([f64::from(at.x), f64::from(at.y), f64::from(at.z)]) as f32
}

/// Where in its cell a cell's centre sits, `0..1` on each axis: an integer
/// hash of the cell and the seed, so every platform lays the same cells.
fn jitter(cell: Vec3, seed: u32) -> Vec3 {
    let mut hash = seed;
    for coordinate in [cell.x, cell.y, cell.z] {
        hash ^= (coordinate as i32 as u32).wrapping_mul(0x9e37_79b9);
        hash = hash.rotate_left(13).wrapping_mul(0x85eb_ca6b);
        hash ^= hash >> 16;
    }
    let unit = |shift: u32| ((hash >> shift) & 0x3ff) as f32 / 1024.0;
    // A cell's centre kept off its walls, so two centres never sit on top
    // of each other across one.
    Vec3::new(unit(0), unit(10), unit(20)) * 0.8 + Vec3::splat(0.1)
}

/// A smooth step of `at` from nothing at `from` to all of it at `to`,
/// clamped: rising when `to` is the higher, falling when it is the lower.
pub(super) fn step(from: f32, to: f32, at: f32) -> f32 {
    let span = to - from;
    if span.abs() < f32::EPSILON {
        return if (at - from) * span.signum() >= 0.0 {
            1.0
        } else {
            0.0
        };
    }
    crate::face::smooth((at - from) / span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_coat_s_cells_are_its_own_and_the_same_every_time() {
        let at = Vec3::new(0.11, 0.42, -0.3);
        let (a, b) = (Fields::new(7, 0.6), Fields::new(7, 0.6));
        assert_eq!(a.cells(at, 9.0), b.cells(at, 9.0));
        assert_ne!(a.cells(at, 9.0), Fields::new(8, 0.6).cells(at, 9.0));
        let (near, next) = a.cells(at, 9.0);
        assert!(near <= next && next < 2.0);
    }

    #[test]
    fn a_step_rises_or_falls_the_way_its_edges_run() {
        assert_eq!(step(0.2, 0.4, 0.1), 0.0);
        assert_eq!(step(0.2, 0.4, 0.5), 1.0);
        // Falling: all of it below the lower edge, none above the higher -
        // a sock is white under its line, not over it.
        assert_eq!(step(0.4, 0.2, 0.1), 1.0);
        assert_eq!(step(0.4, 0.2, 0.5), 0.0);
        assert!((step(0.4, 0.2, 0.3) - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn a_coat_scales_with_its_body() {
        // Twice the body and twice as far along it: the same mark.
        let (small, large) = (Fields::new(3, 0.5), Fields::new(3, 1.0));
        let at = Vec3::new(0.2, 0.3, 0.1);
        assert!((small.patches(at, 3.0) - large.patches(at * 2.0, 3.0)).abs() < 1.0e-5);
        assert_eq!(small.cells(at, 7.0), large.cells(at * 2.0, 7.0));
    }
}
