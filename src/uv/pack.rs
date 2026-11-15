//! Packing charts into a texture atlas.
//!
//! Charts are laid out on shelves - sorted tallest first and laid left to right
//! in rows - and on a skyline, in a few orders, and the densest of those
//! packings is kept (#474). None is the tightest algorithm available, but each
//! is deterministic and cheap, a body has a few dozen charts rather than
//! thousands, and which one packs a body best is luck, so trying them all
//! takes the luck out.

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
///
/// **Several packings are tried and the densest kept** (#474). How full a
/// packing comes out depends on the boxes' shapes by luck: one box a little
/// wider and a shelf of others no longer fits beside it, and every chart on the
/// body loses a tenth of its texels for it. So the boxes are laid out in each
/// of a few orders ([`ORDERS`]), on shelves and on a skyline, and whichever
/// fits them at the largest scale wins: the shelves alone are always among the
/// candidates, so no body packs looser than they would have packed it.
#[must_use]
pub fn pack(sizes: &[Vec2], gutter: f32) -> Vec<Rect> {
    if sizes.is_empty() {
        return Vec::new();
    }
    let gutter = gutter.clamp(0.0, 0.1);

    let mut best: Option<(f32, Vec<Rect>)> = None;
    for order in ORDERS {
        let order = order.sorted(sizes);
        for layout in [shelves as Layout, skyline] {
            if let Some((scale, rects)) = largest(sizes, &order, gutter, layout)
                && best.as_ref().is_none_or(|(most, _)| scale > *most)
            {
                best = Some((scale, rects));
            }
        }
    }

    match best {
        Some((_, rects)) => rects,
        // Degenerate input - everything collapses to a point. Give each chart an
        // equal slice rather than returning nothing.
        None => fallback_grid(sizes.len(), gutter),
    }
}

/// A way of laying boxes out at one scale, in one order.
type Layout = fn(&[Vec2], &[usize], f32, f32) -> Option<Vec<Rect>>;

/// The largest scale `layout` fits the boxes at in `order`, and where it puts
/// them there, by bisection: a closed form does not exist for either layout,
/// and two dozen halvings land within a fraction of a percent.
fn largest(
    sizes: &[Vec2],
    order: &[usize],
    gutter: f32,
    layout: Layout,
) -> Option<(f32, Vec<Rect>)> {
    let mut low = 0.0f32;
    let mut high = 4.0f32;
    let mut best = None;
    for _ in 0..24 {
        let scale = 0.5 * (low + high);
        match layout(sizes, order, scale, gutter) {
            Some(rects) => {
                best = Some((scale, rects));
                low = scale;
            }
            None => high = scale,
        }
    }
    best
}

/// The orders boxes are laid out in, each broken by the next and last by the
/// boxes' own order, so that every packing is the same on every run.
#[derive(Clone, Copy, Debug)]
enum Order {
    /// Tallest first, which is what keeps shelves from wasting height.
    Tallest,
    /// Widest first.
    Widest,
    /// Largest first.
    Largest,
}

/// Every order a packing is tried in. Provenance: **measured** (#474).
const ORDERS: [Order; 3] = [Order::Tallest, Order::Widest, Order::Largest];

impl Order {
    /// The boxes of `sizes`, by index, in this order.
    fn sorted(self, sizes: &[Vec2]) -> Vec<usize> {
        let key = |index: usize| {
            let size = sizes[index];
            match self {
                Self::Tallest => (size.y, size.x),
                Self::Widest => (size.x, size.y),
                Self::Largest => (size.x * size.y, size.y),
            }
        };
        let mut order: Vec<usize> = (0..sizes.len()).collect();
        order.sort_by(|&a, &b| {
            let (one, two) = (key(a), key(b));
            two.0
                .total_cmp(&one.0)
                .then_with(|| two.1.total_cmp(&one.1))
                .then(a.cmp(&b))
        });
        order
    }
}

/// Lays the boxes out on shelves at one scale, or `None` if they run off the
/// top: left to right, a new shelf above the tallest of the last when one would
/// overrun the right edge.
fn shelves(sizes: &[Vec2], order: &[usize], scale: f32, gutter: f32) -> Option<Vec<Rect>> {
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

/// Lays the boxes out on a skyline at one scale, or `None` if one runs off the
/// top: each box goes where it sits lowest on what is already laid, the
/// leftmost such place, and the skyline rises under it.
///
/// Where shelves leave the space above a short box on a tall box's shelf
/// empty, a skyline fills it with whatever fits.
fn skyline(sizes: &[Vec2], order: &[usize], scale: f32, gutter: f32) -> Option<Vec<Rect>> {
    let mut rects = vec![Rect::new(Vec2::ZERO, Vec2::ZERO); sizes.len()];
    // Each step of the skyline: where it starts, and how high it stands there,
    // until the next step starts or the atlas ends.
    let mut steps: Vec<Vec2> = vec![Vec2::splat(gutter)];

    for &index in order {
        let size = (sizes[index] * scale).max(Vec2::splat(1e-4));
        let reach = size + Vec2::splat(gutter);
        // The lowest place it fits, the leftmost of those.
        let mut best: Option<(usize, f32)> = None;
        for first in 0..steps.len() {
            let left = steps[first].x;
            if left + reach.x > 1.0 {
                break;
            }
            let floor = steps[first..]
                .iter()
                .take_while(|step| step.x < left + reach.x)
                .map(|step| step.y)
                .fold(0.0, f32::max);
            if floor + reach.y <= 1.0 && best.is_none_or(|(_, lowest)| floor < lowest) {
                best = Some((first, floor));
            }
        }
        let (first, floor) = best?;
        let corner = Vec2::new(steps[first].x, floor);
        rects[index] = Rect::new(corner, corner + size);

        // The skyline under it rises to its top; the step it ends on carries on
        // past its right side.
        let right = corner.x + reach.x;
        let covered = steps[first..]
            .iter()
            .take_while(|step| step.x < right)
            .count();
        let after = steps[first + covered - 1].y;
        let continues = steps.get(first + covered).is_none_or(|next| next.x > right);
        let mut replaced = vec![Vec2::new(corner.x, floor + reach.y)];
        if continues && right < 1.0 {
            replaced.push(Vec2::new(right, after));
        }
        steps.splice(first..first + covered, replaced);
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
        let rects = pack(&sizes, 0.004);

        assert_eq!(rects.len(), sizes.len());
        assert!(!any_overlap(&rects), "charts must not share texels");
        for rect in &rects {
            assert!(rect.min.x >= 0.0 && rect.min.y >= 0.0, "{rect:?}");
            assert!(rect.max.x <= 1.0 && rect.max.y <= 1.0, "{rect:?}");
        }
    }

    #[test]
    fn relative_sizes_are_preserved() {
        let rects = pack(&[Vec2::splat(1.0), Vec2::splat(0.5)], 0.0);
        let big = rects[0].size();
        let small = rects[1].size();
        assert!((big.x / small.x - 2.0).abs() < 0.01, "{big:?} vs {small:?}");
    }

    #[test]
    fn packing_uses_a_useful_share_of_the_atlas() {
        let sizes = vec![Vec2::new(0.4, 0.3); 6];
        let rects = pack(&sizes, 0.004);
        let used: f32 = rects.iter().map(Rect::area).sum();
        assert!(used > 0.5, "packed only {used} of the atlas");
    }

    #[test]
    fn a_gutter_separates_every_chart() {
        let sizes = vec![Vec2::new(0.3, 0.3); 4];
        let gutter = 0.02;
        let rects = pack(&sizes, gutter);

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
        assert_eq!(pack(&sizes, 0.004), pack(&sizes, 0.004));
    }

    #[test]
    fn degenerate_input_still_yields_distinct_slots() {
        let rects = pack(&[Vec2::ZERO; 4], 0.01);
        assert_eq!(rects.len(), 4);
        assert!(!any_overlap(&rects));
        assert!(rects.iter().all(|r| r.area() > 0.0));
    }

    #[test]
    fn short_charts_are_stacked_beside_a_tall_one() {
        // **A shelf is as tall as its tallest chart** (#474): laid on shelves,
        // the small charts beside a tall one run along its foot and leave the
        // rest of its height empty, and every chart is scaled down until the
        // rest fit on a shelf above. Laid on a skyline they stack up beside it.
        let mut sizes = vec![Vec2::new(0.25, 0.9)];
        sizes.extend(std::iter::repeat_n(Vec2::new(0.2, 0.15), 10));
        let gutter = 0.004;
        let rects = pack(&sizes, gutter);
        let scale = rects[0].size().y / 0.9;
        let order = Order::Tallest.sorted(&sizes);
        let (shelved, _) = largest(&sizes, &order, gutter, shelves).expect("shelves fit");
        println!("packed at {scale:.3}, on shelves alone at {shelved:.3}");
        assert!(!any_overlap(&rects));
        assert!(
            scale >= 1.0,
            "the charts were packed at {scale:.3} of their asked size"
        );
    }

    #[test]
    fn an_empty_atlas_packs_to_nothing() {
        assert!(pack(&[], 0.004).is_empty());
    }
}
