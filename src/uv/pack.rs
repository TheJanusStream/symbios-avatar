//! Packing charts into a texture atlas.
//!
//! Shelf packing: charts are sorted tallest-first and laid left to right in
//! rows. It is not the tightest algorithm available, but it is deterministic and
//! its waste is bounded and predictable — and a body has a dozen or so charts,
//! not thousands, so the difference never shows.

use glam::Vec2;

/// An axis-aligned rectangle in atlas space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Lower-left corner.
    pub min: Vec2,
    /// Upper-right corner.
    pub max: Vec2,
}

impl Rect {
    /// A rectangle from its corners.
    #[must_use]
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Width and height.
    #[must_use]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Fraction of the atlas this rectangle occupies.
    #[must_use]
    pub fn area(&self) -> f32 {
        let size = self.size();
        (size.x * size.y).max(0.0)
    }

    /// Whether two rectangles share any interior area.
    #[must_use]
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    /// Maps a point given in `0..=1` of this rectangle into atlas space.
    #[must_use]
    pub fn lerp(&self, unit: Vec2) -> Vec2 {
        self.min + self.size() * unit
    }
}

/// Places boxes of the given sizes into the unit square.
///
/// Sizes are relative; they are scaled together until everything fits, so the
/// caller expresses only the *ratio* of one chart's texel density to another's.
/// Returns one rectangle per input, in input order.
///
/// `gutter` is the empty margin kept around every chart, which is what stops a
/// mip level or a bilinear tap from bleeding one body part onto another.
#[must_use]
pub fn shelf_pack(sizes: &[Vec2], gutter: f32) -> Vec<Rect> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let gutter = gutter.clamp(0.0, 0.1);

    // Tallest first, which is what keeps shelves from wasting vertical space.
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        sizes[b]
            .y
            .total_cmp(&sizes[a].y)
            .then_with(|| sizes[b].x.total_cmp(&sizes[a].x))
            .then(a.cmp(&b))
    });

    // Search for the largest uniform scale that still fits, by bisection. A
    // closed form does not exist for shelf packing, and a dozen iterations lands
    // within a fraction of a percent.
    let mut low = 0.0f32;
    let mut high = 4.0f32;
    let mut best = Vec::new();
    for _ in 0..24 {
        let scale = 0.5 * (low + high);
        match try_pack(sizes, &order, scale, gutter) {
            Some(rects) => {
                best = rects;
                low = scale;
            }
            None => high = scale,
        }
    }

    if best.is_empty() {
        // Degenerate input — everything collapses to a point. Give each chart an
        // equal slice rather than returning nothing.
        return fallback_grid(sizes.len(), gutter);
    }
    best
}

/// Lays the boxes out at one scale, or `None` if they run off the top.
fn try_pack(sizes: &[Vec2], order: &[usize], scale: f32, gutter: f32) -> Option<Vec<Rect>> {
    let mut rects = vec![Rect::new(Vec2::ZERO, Vec2::ZERO); sizes.len()];
    let mut cursor = Vec2::splat(gutter);
    let mut shelf_height = 0.0f32;

    for &index in order {
        let size = (sizes[index] * scale).max(Vec2::splat(1e-4));
        if size.x + gutter * 2.0 > 1.0 || size.y + gutter * 2.0 > 1.0 {
            return None;
        }

        // Start a new shelf when this chart would overrun the right edge.
        if cursor.x + size.x + gutter > 1.0 {
            cursor.x = gutter;
            cursor.y += shelf_height + gutter;
            shelf_height = 0.0;
        }
        if cursor.y + size.y + gutter > 1.0 {
            return None;
        }

        rects[index] = Rect::new(cursor, cursor + size);
        cursor.x += size.x + gutter;
        shelf_height = shelf_height.max(size.y);
    }

    Some(rects)
}

/// An equal-slice layout, used when the requested sizes carry no information.
fn fallback_grid(count: usize, gutter: f32) -> Vec<Rect> {
    let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
    let rows = count.div_ceil(columns);
    let cell = Vec2::new(1.0 / columns as f32, 1.0 / rows as f32);

    (0..count)
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            let min = Vec2::new(column as f32, row as f32) * cell + Vec2::splat(gutter);
            Rect::new(min, min + cell - Vec2::splat(gutter * 2.0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether any two rectangles overlap.
    fn any_overlap(rects: &[Rect]) -> bool {
        rects
            .iter()
            .enumerate()
            .any(|(index, rect)| rects[index + 1..].iter().any(|other| rect.overlaps(other)))
    }

    #[test]
    fn charts_never_overlap_and_stay_inside_the_atlas() {
        let sizes: Vec<Vec2> = (1..=12)
            .map(|n| Vec2::new(0.1 * n as f32, 0.07 * (13 - n) as f32))
            .collect();
        let rects = shelf_pack(&sizes, 0.004);

        assert_eq!(rects.len(), sizes.len());
        assert!(!any_overlap(&rects), "charts must not share texels");
        for rect in &rects {
            assert!(rect.min.x >= 0.0 && rect.min.y >= 0.0, "{rect:?}");
            assert!(rect.max.x <= 1.0 && rect.max.y <= 1.0, "{rect:?}");
        }
    }

    #[test]
    fn relative_sizes_are_preserved() {
        let rects = shelf_pack(&[Vec2::splat(1.0), Vec2::splat(0.5)], 0.0);
        let big = rects[0].size();
        let small = rects[1].size();
        assert!((big.x / small.x - 2.0).abs() < 0.01, "{big:?} vs {small:?}");
    }

    #[test]
    fn packing_uses_a_useful_share_of_the_atlas() {
        let sizes = vec![Vec2::new(0.4, 0.3); 6];
        let rects = shelf_pack(&sizes, 0.004);
        let used: f32 = rects.iter().map(Rect::area).sum();
        assert!(used > 0.5, "packed only {used} of the atlas");
    }

    #[test]
    fn a_gutter_separates_every_chart() {
        let sizes = vec![Vec2::new(0.3, 0.3); 4];
        let gutter = 0.02;
        let rects = shelf_pack(&sizes, gutter);

        for (index, rect) in rects.iter().enumerate() {
            for other in &rects[index + 1..] {
                let gap_x = (rect.min.x - other.max.x).max(other.min.x - rect.max.x);
                let gap_y = (rect.min.y - other.max.y).max(other.min.y - rect.max.y);
                assert!(
                    gap_x >= gutter - 1e-5 || gap_y >= gutter - 1e-5,
                    "{rect:?} and {other:?} are closer than the gutter"
                );
            }
        }
    }

    #[test]
    fn packing_is_deterministic() {
        let sizes: Vec<Vec2> = (1..=8).map(|n| Vec2::splat(0.05 * n as f32)).collect();
        assert_eq!(shelf_pack(&sizes, 0.004), shelf_pack(&sizes, 0.004));
    }

    #[test]
    fn degenerate_input_still_yields_distinct_slots() {
        let rects = shelf_pack(&[Vec2::ZERO; 4], 0.01);
        assert_eq!(rects.len(), 4);
        assert!(!any_overlap(&rects));
        assert!(rects.iter().all(|r| r.area() > 0.0));
    }

    #[test]
    fn an_empty_atlas_packs_to_nothing() {
        assert!(shelf_pack(&[], 0.004).is_empty());
    }
}
