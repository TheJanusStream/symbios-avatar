//! Where a winged body's cards keep clear of its wings and its eyes (#423),
//! and the nearest-point lookup the cards are laid with.
//!
//! **Read off the wing parts as built**, at rest: a bird's folded wing lies
//! over its flank as a stack of feathers a few millimetres off the skin, and
//! a bat's cape and propatagium as membrane a centimetre off it. A card rooted
//! under either is drawn where no camera reaches - or worse, through the
//! wing - so the body within a reach of any of their points grows none, and
//! a little further out grows fewer, so the edge is not a line (`Clear`).
//!
//! **And no card through one, in any pose it is seen in**: a card rooted
//! clear of the wing's points may still reach across it - a long feather's
//! vertices are sparse along it - or ride its arm out through a membrane as
//! the wing spreads, so every card one of whose edges passes through a wing
//! part's triangles (`Sheet`, read at rest and in each spread pose) or comes
//! near an eye's globe (`mark_near`) is left out whole (`leave_out`).

use std::collections::HashMap;

use glam::Vec3;

use super::FurGrowth;
use crate::face::step;
use crate::mesh::PolyMesh;

/// Points binned on a cube grid, for the nearest of them to a point.
pub(super) struct Grid {
    cell: f32,
    bins: HashMap<(i32, i32, i32), Vec<u32>>,
    points: Vec<Vec3>,
}

impl Grid {
    /// `points` binned in cubes `cell` metres on a side.
    pub(super) fn of(points: &[Vec3], cell: f32) -> Self {
        let cell = cell.max(1e-4);
        let mut bins: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (index, point) in points.iter().enumerate() {
            bins.entry(Self::bin(*point, cell))
                .or_default()
                .push(index as u32);
        }
        Self {
            cell,
            bins,
            points: points.to_vec(),
        }
    }

    pub(super) fn bin(at: Vec3, cell: f32) -> (i32, i32, i32) {
        let at = (at / cell).floor();
        (at.x as i32, at.y as i32, at.z as i32)
    }

    /// The nearest point to `at` within `rings` cells of it each way, and how
    /// far it is; `None` if there is none that near.
    fn near(&self, at: Vec3, rings: i32) -> Option<(usize, f32)> {
        let (x, y, z) = Self::bin(at, self.cell);
        let mut best: Option<(usize, f32)> = None;
        for dx in -rings..=rings {
            for dy in -rings..=rings {
                for dz in -rings..=rings {
                    let Some(bin) = self.bins.get(&(x + dx, y + dy, z + dz)) else {
                        continue;
                    };
                    for &index in bin {
                        let distance = self.points[index as usize].distance_squared(at);
                        if best.is_none_or(|(_, most)| distance < most) {
                            best = Some((index as usize, distance));
                        }
                    }
                }
            }
        }
        best.map(|(index, squared)| (index, squared.sqrt()))
    }

    /// The nearest point to `at` of all of them: looked for in the cells
    /// round it, then every point when none is that near.
    pub(super) fn nearest(&self, at: Vec3) -> Option<usize> {
        self.near(at, 1)
            .or_else(|| {
                self.points
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        a.1.distance_squared(at)
                            .total_cmp(&b.1.distance_squared(at))
                    })
                    .map(|(index, point)| (index, point.distance(at)))
            })
            .map(|(index, _)| index)
    }
}

/// How much further than the cards thin they are shortened near a wing part
/// (#424), as a multiple of the thinning's outer reach.
const SHORT_REACH: f32 = 2.5;

/// The body a winged body's cards keep clear of (#423): within `near` of any
/// point of its wing parts none roots, and from there out to `far` fewer.
pub(crate) struct Clear {
    grid: Grid,
    near: f32,
    far: f32,
    floor: f32,
    short: f32,
}

impl Clear {
    /// Clear of `points` by `near` metres, thinning out to `far`.
    pub(crate) fn around(points: &[Vec3], near: f32, far: f32) -> Self {
        let far = far.max(near + 1e-4);
        Self {
            grid: Grid::of(points, far),
            near,
            far,
            floor: 0.0,
            short: 1.0,
        }
    }

    /// Thinned to `floor` of a card's weight under a wing part rather than
    /// cleared, and its cards there `short` of their length (#424): what a
    /// bird's flank keeps under its folded wing, which shows when the wing
    /// spreads.
    pub(crate) fn thinned_to(self, floor: f32, short: f32) -> Self {
        Self {
            floor: floor.clamp(0.0, 1.0),
            short: short.clamp(0.0, 1.0),
            ..self
        }
    }

    /// How much of its length a card rooted at `at` keeps, `0..=1`: all of
    /// it clear of a wing part, and under one the share it is thinned to.
    ///
    /// **Out to [`SHORT_REACH`] times as far as the cards thin**: a card is
    /// longer than the thinning's reach, so one rooted beside the fold lies
    /// under it as surely as one rooted beneath.
    pub(crate) fn length(&self, at: Vec3) -> f32 {
        if self.short >= 1.0 {
            return 1.0;
        }
        let far = self.far * SHORT_REACH;
        self.grid
            .near(at, SHORT_REACH.ceil() as i32)
            .map_or(1.0, |(_, distance)| {
                self.short + (1.0 - self.short) * step(self.near, far, distance)
            })
    }

    /// How much of a card's weight a root at `at` keeps, `0..=1`: the floor
    /// under a wing part - none, unless thinned - and all of it clear of one.
    pub(crate) fn kept(&self, at: Vec3) -> f32 {
        self.grid.near(at, 1).map_or(1.0, |(_, distance)| {
            self.floor + (1.0 - self.floor) * step(self.near, self.far, distance)
        })
    }
}

/// A wing part's triangles as they lie in one pose, binned on a grid, for
/// whether a card passes through it (#423): a membrane, or a bird's folded
/// flight feathers.
pub(crate) struct Sheet {
    triangles: Vec<[Vec3; 3]>,
    cell: f32,
    bins: HashMap<(i32, i32, i32), Vec<u32>>,
}

impl Sheet {
    /// How many cells the sheet's longest side is cut into.
    const CELLS: f32 = 48.0;

    /// `mesh`'s triangles, binned.
    pub(crate) fn of(mesh: &PolyMesh) -> Self {
        let triangles: Vec<[Vec3; 3]> = mesh
            .triangulated()
            .iter()
            .map(|triangle| triangle.map(|vertex| mesh.positions[vertex as usize]))
            .collect();
        let (low, high) = mesh.bounds();
        let cell = ((high - low).max_element() / Self::CELLS).max(1e-4);
        let mut bins: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        for (index, triangle) in triangles.iter().enumerate() {
            let (lo, hi) = (
                Grid::bin(triangle[0].min(triangle[1]).min(triangle[2]), cell),
                Grid::bin(triangle[0].max(triangle[1]).max(triangle[2]), cell),
            );
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        bins.entry((x, y, z)).or_default().push(index as u32);
                    }
                }
            }
        }
        Self {
            triangles,
            cell,
            bins,
        }
    }

    /// Whether the segment from `from` to `to` passes through the sheet.
    pub(crate) fn crossed(&self, from: Vec3, to: Vec3) -> bool {
        let (lo, hi) = (
            Grid::bin(from.min(to), self.cell),
            Grid::bin(from.max(to), self.cell),
        );
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    let Some(bin) = self.bins.get(&(x, y, z)) else {
                        continue;
                    };
                    if bin
                        .iter()
                        .any(|&index| crosses(from, to, &self.triangles[index as usize]))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Marks in `out` both ends of every edge of `faces` that passes through
    /// the sheet, each vertex at its place in `positions`: the cards as they
    /// lie in the pose the sheet was read in.
    pub(crate) fn mark_crossing(&self, faces: &[Vec<u32>], positions: &[Vec3], out: &mut [bool]) {
        for face in faces {
            for (index, &a) in face.iter().enumerate() {
                let b = face[(index + 1) % face.len()];
                if self.crossed(positions[a as usize], positions[b as usize]) {
                    out[a as usize] = true;
                    out[b as usize] = true;
                }
            }
        }
    }
}

/// `growth` with every card a vertex of which `out` marks left out (#423): a
/// card is a run of faces sharing vertices, and its line in the ledger gives
/// back what it cost. Returns how many triangles were left out.
pub(crate) fn leave_out(growth: &mut FurGrowth, out: &[bool]) -> usize {
    if !out.iter().any(|&marked| marked) {
        return 0;
    }
    let mesh = &growth.mesh;
    // Each face's card: the faces joined through the vertices they share.
    let mut parent: Vec<usize> = (0..mesh.vertex_count()).collect();
    fn root(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            parent[at] = parent[parent[at]];
            at = parent[at];
        }
        at
    }
    for face in &mesh.faces {
        for pair in face.windows(2) {
            let (a, b) = (
                root(&mut parent, pair[0] as usize),
                root(&mut parent, pair[1] as usize),
            );
            parent[a.max(b)] = a.min(b);
        }
    }
    let mut crossing = vec![false; mesh.vertex_count()];
    for (vertex, &marked) in out.iter().enumerate().take(mesh.vertex_count()) {
        if marked {
            let card = root(&mut parent, vertex);
            crossing[card] = true;
        }
    }
    // Each face's line in the ledger: the zones' faces lie in the order
    // they were grown, each line as many triangles long as it cost.
    let mut zone_of = Vec::with_capacity(mesh.faces.len());
    let (mut line, mut spent) = (0, 0);
    for face in &mesh.faces {
        while line + 1 < growth.grown.len() && spent >= growth.grown[line].tris {
            spent -= growth.grown[line].tris;
            line += 1;
        }
        zone_of.push(line);
        spent += face.len().saturating_sub(2);
    }
    let mut kept = Vec::with_capacity(mesh.faces.len());
    let mut given_back = vec![(0usize, std::collections::HashSet::new()); growth.grown.len()];
    for (face, &line) in mesh.faces.iter().zip(&zone_of) {
        let card = root(&mut parent, face[0] as usize);
        if crossing[card] {
            if let Some((tris, cards)) = given_back.get_mut(line) {
                *tris += face.len().saturating_sub(2);
                cards.insert(card);
            }
        } else {
            kept.push(face.clone());
        }
    }
    let mut left_out = 0;
    for (line, (tris, cards)) in growth.grown.iter_mut().zip(given_back) {
        left_out += tris.min(line.tris);
        line.tris = line.tris.saturating_sub(tris);
        line.clumps = line.clumps.saturating_sub(cards.len());
    }
    growth.mesh.faces = kept;
    growth.mesh.compact();
    growth.grown.retain(|line| line.clumps > 0 && line.tris > 0);
    left_out
}

/// Marks in `out` both ends of every edge of `faces` that comes within
/// `margin` of one of `globes` (a centre and a radius each), as a share of
/// that globe's radius: a card lying across an eye, or near enough that it
/// reads as lying in it.
pub(crate) fn mark_near(
    faces: &[Vec<u32>],
    positions: &[Vec3],
    globes: &[(Vec3, f32)],
    margin: f32,
    out: &mut [bool],
) {
    if globes.is_empty() {
        return;
    }
    for face in faces {
        for (index, &a) in face.iter().enumerate() {
            let b = face[(index + 1) % face.len()];
            let (from, to) = (positions[a as usize], positions[b as usize]);
            let run = to - from;
            let near = globes.iter().any(|&(centre, radius)| {
                let t = ((centre - from).dot(run) / run.length_squared().max(f32::EPSILON))
                    .clamp(0.0, 1.0);
                from.lerp(to, t).distance(centre) < radius * (1.0 + margin)
            });
            if near {
                out[a as usize] = true;
                out[b as usize] = true;
            }
        }
    }
}

/// Whether the segment `from`-`to` passes through `triangle`.
fn crosses(from: Vec3, to: Vec3, [a, b, c]: &[Vec3; 3]) -> bool {
    let run = to - from;
    let (ab, ac) = (*b - *a, *c - *a);
    let p = run.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-14 {
        return false;
    }
    let off = from - *a;
    let u = off.dot(p) / det;
    let q = off.cross(ab);
    let v = run.dot(q) / det;
    let t = ac.dot(q) / det;
    (0.0..=1.0).contains(&u) && v >= 0.0 && u + v <= 1.0 && (0.0..=1.0).contains(&t)
}
