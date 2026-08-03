//! Hair as a sculpted shell rather than a head of separate locks.
//!
//! **Why the shape changed.** Per-lock geometry cost 15,976 triangles of a
//! 29,360-triangle body — 54% of the whole avatar — and every quality judgement
//! made of it said the same thing: at close range the locks read as flat ribbons
//! with hard rectangular ends, not as hair. Half the budget was buying a look
//! that did not arrive, while the face, which every judgement named as the
//! dominant defect, could not afford the surface to carry a nose (#59).
//!
//! A shell is what stylised hair is actually made of. The mass of it is one
//! sculpted surface; what makes it read as hair rather than as a swim cap is the
//! *edge* — a fringe that breaks the brow line, a parting, and tapered ends
//! where the mass leaves the head. Those stay locks, because that is what they
//! are, and there are a dozen of them rather than a hundred and twenty-eight.
//!
//! **The shell is lofted along the same curves the locks used.** Each column
//! samples the same sweep a strand would — down the scalp to the
//! hairline, then falling free, waved, and pushed clear of the shoulders — and
//! adjacent columns are joined into quads. So everything that made the locks sit
//! on the head correctly still applies, and the axes keep meaning what they
//! meant. This is a change of geometry, not of shaping.
//!
//! It is a closed solid, not a sheet. A one-sided surface disappears when seen
//! from behind and shows a paper edge at the hairline, which is the other half
//! of why cheap hair reads as a cap.

use glam::Vec3;

use crate::mesh::PolyMesh;

use super::scalp::Scalp;

/// Roughly how large a shell quad should be, as a fraction of the head radius.
///
/// Hair does not need the resolution a face does — it carries no feature
/// smaller than a lock — so this is set by where the silhouette stops looking
/// faceted rather than by anything it has to hold. About a fifth of a head
/// radius, which on a default body is 17 mm across and 20 mm down.
const CELL: f32 = 0.20;

/// The fewest and most columns the shell is swept with.
///
/// A floor because below about twenty the crown is visibly a polygon; a ceiling
/// because this is the axis that costs geometry and a record is not to be
/// trusted with it.
const MIN_COLUMNS: usize = 20;
/// See [`MIN_COLUMNS`].
const MAX_COLUMNS: usize = 64;
/// See [`MIN_COLUMNS`]. Rows run from the crown to the tips.
const MIN_ROWS: usize = 8;
/// See [`MIN_COLUMNS`].
const MAX_ROWS: usize = 40;

/// How thick the shell is, in head radii.
///
/// Enough that the hairline reads as an edge with a hair's depth to it rather
/// than as a cut sheet, and no more: everything thicker starts to read as a
/// helmet, which is the failure mode this whole shape has to avoid.
pub(super) const THICKNESS: f32 = 0.035;

/// One sculpted mass of hair.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    /// The outer surface, row-major: `columns` around by `rows` down.
    outer: Vec<Vec3>,
    /// The inner surface, against the scalp, in the same order.
    inner: Vec<Vec3>,
    columns: usize,
    rows: usize,
}

impl Shell {
    /// Lofts a shell from one profile curve per column.
    ///
    /// `profile` is handed an azimuth and returns the curve the hair follows at
    /// it — the same curve a strand rooted there would take. Curves of different
    /// lengths are fine and expected: a fringe is far shorter than the fall
    /// behind it, and each is resampled to the same number of rows by arc
    /// length so the surface between them stays even.
    #[must_use]
    pub fn loft(
        scalp: &Scalp,
        columns: usize,
        rows: usize,
        profile: impl Fn(f32) -> Vec<Vec3>,
    ) -> Self {
        let columns = columns.clamp(MIN_COLUMNS, MAX_COLUMNS);
        let rows = rows.clamp(MIN_ROWS, MAX_ROWS);
        let thickness = scalp.radius() * THICKNESS;

        let mut outer = Vec::with_capacity(columns * rows);
        let mut inner = Vec::with_capacity(columns * rows);
        for column in 0..columns {
            let azimuth = std::f32::consts::TAU * column as f32 / columns as f32;
            let curve = resample(&profile(azimuth), rows);
            for (row, &point) in curve.iter().enumerate() {
                // Tapered toward the tips. A shell of even thickness ends in a
                // flat slab, and a slab across the bottom of a head of hair is
                // most of what makes it read as a bonnet rather than as hair
                // that ends. Thickest over the skull, thinning to an edge.
                let down = row as f32 / (rows - 1).max(1) as f32;
                let thickness = thickness * (1.0 - 0.85 * down * down);
                // Inward along the curve's own normal — away from the head —
                // rather than toward the scalp's centre. Down the fall there is
                // no scalp beneath to point away from, and using the centre
                // there pinches the tips together into a spike.
                let along = tangent(&curve, row);
                let out = (point - scalp.origin()).reject_from_normalized(along);
                let away = out.normalize_or(Vec3::Y);
                outer.push(point + away * thickness * 0.5);
                inner.push(point - away * thickness * 0.5);
            }
        }
        Self {
            outer,
            inner,
            columns,
            rows,
        }
    }

    /// How many columns and rows it was swept with.
    #[must_use]
    pub fn grid(&self) -> (usize, usize) {
        (self.columns, self.rows)
    }

    /// How low the mass reaches on the front of the head, in head-local metres.
    ///
    /// The hairline, measured rather than assumed. `forward` is how far in front
    /// of the head's centre a point has to be to count as the forehead at all —
    /// without it this finds the fall down the back, which is far lower and says
    /// nothing about where the hair starts.
    #[must_use]
    pub fn front_edge(&self, forward: f32) -> f32 {
        self.outer
            .iter()
            .filter(|point| point.z > forward)
            .fold(f32::MAX, |low, point| low.min(point.y))
    }

    /// The lowest the shell hangs, in head-local metres.
    #[must_use]
    pub fn drop(&self) -> f32 {
        -self
            .outer
            .iter()
            .chain(&self.inner)
            .fold(0.0f32, |low, point| low.min(point.y))
    }

    /// What the shell costs to build, in triangles.
    ///
    /// Counted rather than measured after the fact, so a budget can be checked
    /// before anything is swept.
    #[must_use]
    pub fn tris(&self) -> usize {
        let bands = self.rows.saturating_sub(1);
        // Two surfaces of `columns * bands` quads, closed top and bottom by a
        // ring of `columns` each. Two triangles per quad.
        (self.columns * bands * 2 + self.columns * 2) * 2
    }

    /// The shell as one closed solid, shaded.
    ///
    /// Carries a brightness walk on its vertices rather than one flat colour.
    /// This is the whole helmet risk in one place: a sculpted mass drawn in a
    /// single tone reads as a swim cap however well it is shaped, and the locks
    /// it replaced avoided that by accident, because each carried its own shade.
    /// Darker at the roots where light does not reach, lighter toward the tips,
    /// with a slow variation around the head so neighbouring columns are not
    /// identical — and the inner surface darker than the outer at the same
    /// place, because it faces the head and almost nothing reaches it.
    #[must_use]
    pub fn painted(&self, tone: Vec3) -> PolyMesh {
        let mut mesh = self.mesh();
        let colours = (0..mesh.vertex_count())
            .map(|index| {
                let facing = if index < self.outer.len() { 1.0 } else { 0.82 };
                let at = index % self.outer.len().max(1);
                let (column, row) = (at / self.rows, at % self.rows);
                let down = row as f32 / (self.rows - 1).max(1) as f32;
                let around = std::f32::consts::TAU * column as f32 / self.columns as f32;
                tone * (0.70 + 0.42 * down) * (1.0 + 0.06 * (around * 3.0).sin()) * facing
            })
            .collect();
        mesh.set_colours(colours);
        mesh
    }

    /// The shell as one closed solid.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for point in self.outer.iter().chain(&self.inner) {
            mesh.push_vertex(*point);
        }
        let count = self.outer.len() as u32;
        let outer = |column: usize, row: usize| (column * self.rows + row) as u32;
        let inner = |column: usize, row: usize| outer(column, row) + count;

        for column in 0..self.columns {
            // Wraps, because a head is a closed loop and a seam down the back of
            // it is a stripe of daylight.
            let next = (column + 1) % self.columns;
            for row in 0..self.rows - 1 {
                mesh.push_face([
                    outer(column, row),
                    outer(column, row + 1),
                    outer(next, row + 1),
                    outer(next, row),
                ]);
                // Wound the other way: the inner surface faces the head.
                mesh.push_face([
                    inner(column, row),
                    inner(next, row),
                    inner(next, row + 1),
                    inner(column, row + 1),
                ]);
            }
            // The rim at the crown and the cut ends at the tips, so the shell is
            // a solid and shows a hair's depth rather than a paper edge.
            mesh.push_face([
                outer(column, 0),
                outer(next, 0),
                inner(next, 0),
                inner(column, 0),
            ]);
            let last = self.rows - 1;
            mesh.push_face([
                outer(next, last),
                outer(column, last),
                inner(column, last),
                inner(next, last),
            ]);
        }
        mesh
    }
}

/// How many columns and rows a scalp of this size wants, for even-sized quads.
///
/// Taken from the hair's own measurements rather than fixed, so a small head and
/// a large one are swept at the same fineness instead of the same count.
#[must_use]
pub fn grid_for(scalp: &Scalp, longest: f32) -> (usize, usize) {
    let radius = scalp.radius().max(f32::EPSILON);
    let around = std::f32::consts::TAU * scalp.width_at(0.0).max(0.25) * radius;
    let columns = (around / (CELL * radius)).round() as usize;
    let rows = (longest / (CELL * radius)).round() as usize;
    (
        columns.clamp(MIN_COLUMNS, MAX_COLUMNS),
        rows.clamp(MIN_ROWS, MAX_ROWS),
    )
}

/// Resamples a curve to exactly `count` points, evenly by arc length.
///
/// The curves come in at whatever step count their own cap and fall asked for —
/// a fringe is sampled far more coarsely than the hair behind it — and a surface
/// lofted between curves of different lengths shears wherever they disagree.
fn resample(curve: &[Vec3], count: usize) -> Vec<Vec3> {
    if curve.is_empty() {
        return vec![Vec3::ZERO; count];
    }
    if curve.len() == 1 {
        return vec![curve[0]; count];
    }

    let mut along = Vec::with_capacity(curve.len());
    let mut total = 0.0;
    along.push(0.0);
    for pair in curve.windows(2) {
        total += pair[0].distance(pair[1]);
        along.push(total);
    }
    if total <= f32::EPSILON {
        return vec![curve[0]; count];
    }

    (0..count)
        .map(|step| {
            let want = total * step as f32 / (count - 1).max(1) as f32;
            let at = along.partition_point(|&reached| reached < want).max(1);
            let (before, after) = (at - 1, at.min(curve.len() - 1));
            let span = along[after] - along[before];
            let blend = if span > f32::EPSILON {
                (want - along[before]) / span
            } else {
                0.0
            };
            curve[before].lerp(curve[after], blend)
        })
        .collect()
}

/// The direction a curve runs at one of its points.
fn tangent(curve: &[Vec3], at: usize) -> Vec3 {
    if curve.len() < 2 {
        return Vec3::Y;
    }
    let before = curve[at.saturating_sub(1)];
    let after = curve[(at + 1).min(curve.len() - 1)];
    (after - before).normalize_or(Vec3::Y)
}

/// Where the silhouette pieces go, as fractions round the head from the front.
///
/// Not evenly spaced. A shell is a solid mass and what breaks it up is where the
/// hair naturally parts from itself: over the brow, at the temples where the
/// fringe ends, and at the two lines where the fall leaves the shoulders. Spread
/// them evenly and they read as a fence.
pub(super) const SILHOUETTE: [f32; 12] = [
    0.0, 0.05, 0.10, 0.90, 0.95, // the fringe, densest at the parting
    0.17, 0.83, // the temples
    0.30, 0.70, // where the fall leaves the jaw
    0.42, 0.58, // either side of the back
    0.50, // the centre back
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Rig;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    /// A scalp measured off a real body, as the rest of the hair tests do.
    fn scalp(seed: i64) -> Scalp {
        let mut record = AvatarRecord::new("Shelled", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        Scalp::measure(&mesh, &rig).expect("a humanoid has a head")
    }

    fn straight(count: usize) -> Vec<Vec3> {
        (0..count)
            .map(|step| Vec3::new(0.0, -(step as f32) * 0.01, 0.0))
            .collect()
    }

    #[test]
    fn resampling_keeps_the_ends_and_evens_the_middle() {
        let curve = vec![
            Vec3::ZERO,
            Vec3::new(0.0, -0.01, 0.0),
            Vec3::new(0.0, -0.50, 0.0),
        ];
        let even = resample(&curve, 6);
        assert_eq!(even.len(), 6);
        assert!(even[0].abs_diff_eq(curve[0], 1e-6), "the start moved");
        assert!(even[5].abs_diff_eq(curve[2], 1e-6), "the end moved");

        // Every step the same length, which is the point: the input was bunched
        // at one end and a surface lofted from it would shear there.
        let steps: Vec<f32> = even.windows(2).map(|p| p[0].distance(p[1])).collect();
        let first = steps[0];
        for step in &steps {
            assert!(
                (step - first).abs() < 1e-4,
                "steps ran {first} then {step}, so it is still bunched"
            );
        }
    }

    #[test]
    fn a_shell_is_a_closed_manifold() {
        // The whole reason it carries an inner surface: a one-sided sheet
        // vanishes seen from behind and shows a paper edge at the hairline.
        let scalp = scalp(1);
        let shell = Shell::loft(&scalp, 24, 12, |_| straight(20));
        let mesh = shell.mesh();
        assert!(
            mesh.is_closed_manifold(),
            "the shell is not watertight: {:?}",
            mesh.manifold_report()
        );
    }

    #[test]
    fn the_counted_cost_is_the_cost() {
        // `tris` is consulted before anything is swept, so it has to agree with
        // what sweeping actually produces rather than approximate it.
        let scalp = scalp(3);
        for (columns, rows) in [(20, 8), (32, 16), (64, 40)] {
            let shell = Shell::loft(&scalp, columns, rows, |_| straight(30));
            assert_eq!(
                shell.tris(),
                shell.mesh().triangulated().len(),
                "counted {} triangles for {columns}x{rows}",
                shell.tris()
            );
        }
    }

    #[test]
    fn a_bigger_head_is_swept_at_the_same_fineness_not_the_same_count() {
        // Same head, same fineness, whatever the hair does: the columns follow
        // the head's own girth and the rows follow how far the hair travels.
        let head = scalp(1);
        let (columns, short) = grid_for(&head, 0.10);
        let (again, long) = grid_for(&head, 0.40);
        assert_eq!(
            columns, again,
            "the girth did not change but the columns did"
        );
        assert!(
            long > short,
            "hair four times as long was swept with {long} rows against {short}"
        );
    }
}
