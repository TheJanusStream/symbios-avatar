//! Flattening a patch of skin the way it runs (#473).
//!
//! Every other chart is wrapped round an axis or flattened down one, and either
//! reads skin that faces along the line it is seen from edge-on: wrapped round
//! the head joint, the flanks of the nose and the edges of the philtrum run
//! nearly along the line from the axis, and a step along them barely turns the
//! angle, so they came out whole triangles to a texel. Nothing seen from one
//! place can chart a face whose skin faces every way, so the face is flattened
//! by its own shape instead: the chart in which every triangle of skin is as
//! near as it can be to itself, turned but neither stretched nor squeezed - the
//! side of the nose as much as its tip (Liu et al., *A local/global approach to
//! mesh parameterization*, 2008: "as rigid as possible").
//!
//! **Why not a conformal map.** A conformal chart keeps every little square a
//! square but lets its size run free, and a face is curved enough that its size
//! ran a long way: tried first, the chart grew toward its border and shrank
//! into the nose and the eye sockets, the eyes a third coarser than the face
//! round them and the plane under the jaw three times as fine. Held to each
//! triangle's own size as well as its shape, the chart pays for the face's
//! curvature in a little stretch spread over it instead.
//!
//! **The solve** alternates two steps from a starting chart. Each triangle
//! takes the turn that best lays its own shape over where the chart has it
//! (the closest rotation, in closed form); then the chart is the one that best
//! fits all those turned triangles at once, the cotangent Laplacian of the
//! patch solved by conjugate gradients from the chart before, one vertex held
//! where it was. The matrix is the mesh's own edges, a handful a vertex, so the
//! solve holds a few arrays the length of the patch and nothing dense: for
//! the largest faces, some 8,800 vertices and 17,500 triangles (roll 2 and
//! the largest slider heads), 4.4 to 7.15 MB while it runs (measured by #473's
//! critic), all of it freed before the bake; a person's crown and the band
//! round the back of the head are a few hundred vertices each.
//!
//! **The scale.** The chart comes back in metres of skin, scaled so that its
//! area is the patch's, so that it is sized for the atlas as honestly as a
//! wrapped chart is.
//!
//! **Detail.** A triangle may ask for more or less than its own size: its shape
//! is kept, grown by how much detail its skin carries, and the chart fits the
//! grown triangles instead (on a person's face the scalp and the skin under the
//! jaw ask less). The metres it comes back in are then the grown skin's, so
//! a patch that asks for more detail asks the atlas for more texels. Where the
//! asking changes, the chart cannot keep every triangle's shape, and pays in a
//! little stretch across the change, so it eases in rather than steps. A patch
//! that asks MORE in its middle than round it is squeezed by the skin round
//! it, and a squeezed needle turns over: so a person's mouth asks nothing
//! here, and is magnified once the chart is flat instead (the caller's
//! `reshape`, `PersonHead::magnify`).
//!
//! **Nothing laid twice.** A fit that keeps shapes as near as it can is not
//! kept from turning a triangle over, and a truncated one less so: the needles
//! at the corners of a person's mouth, and the skin where a face's border
//! turns under the jaw, came out turned over or laid over the skin beside them
//! on a third of the bodies #473's critics measured (`untangle`); and a smooth
//! reshaping bends a needle over. So once the chart is fitted, and reshaped,
//! every triangle turned over is laid back the least way it can be, a corner
//! moved to the nearest place where every triangle round it is the chart's way
//! round; and what that cannot do - a vertex its triangles go round twice, a
//! crossing of the border, a fold whose corners must move together - is
//! relaxed out of it, a move kept only where it turns nothing more over.

use glam::{DVec2, DVec3, Vec2, Vec3};

use crate::mesh::PolyMesh;

/// How many times the chart is fitted to its triangles' turns.
///
/// **The solve is truncated, not settled.** On a person's face every fit takes
/// all of [`MOST_STEPS`], and the last leaves a residual of a twenty-fifth to
/// three tenths of where it started; run to a hundred rounds of a thousand
/// steps, the chart's vertices move by 3 to 47 mm at their ninety-ninth
/// percentile and up to 97 mm (measured by #473's critic on its first face
/// chart, over the rolls and slider corners). What the chart is sized and
/// judged by barely moves with them: its rectangle's fill, its folds and the
/// texels it lays twice come out within a few hundredths of the long solve's,
/// and on the five heads of `tests/uv.rs` the face's median texel step within
/// a few hundredths of a millimetre of twelve rounds of two hundred, which cost
/// a person's build a seventh more where these cost it a twenty-fifth.
/// Provenance: **measured** (#473).
const ROUNDS: usize = 6;

/// The most conjugate gradient steps one fit takes: see [`ROUNDS`].
/// Provenance: **measured** (#473).
const MOST_STEPS: usize = 80;

/// How far a fit's residual falls, as a share of its right-hand side, before
/// the fit stops early. Provenance: **chosen** (#473): a fit on a person's face
/// takes all of [`MOST_STEPS`] before it gets here.
const SETTLED: f64 = 1e-4;

/// How thin a triangle may be, as the sine of its narrowest corner, before it is
/// left out of the fit: a fanned polygon with three corners in a line has a
/// triangle of no area and no shape to keep. Anything with a shape is kept,
/// needles and all: left out at three degrees, the needles a fanned polygon
/// makes beside the face's finer faces were free to turn over, and each face
/// chart came out with sixteen to thirty texels flipped and as many laid twice.
/// Provenance: **measured** (#473): at this, a thirtieth of a degree, and at a
/// fifth and a fiftieth of it, the five heads of `tests/uv.rs` lay no texel of
/// their faces twice; at a fifth of a degree, a few.
const THINNEST: f64 = 1e-6;

/// A patch flattened: its vertices, ascending, and each one's place in metres.
pub(super) struct Unfolded {
    /// The mesh vertices the patch's faces use, ascending.
    pub vertices: Vec<u32>,
    /// Each vertex's place in the chart, in metres of skin.
    pub places: Vec<Vec2>,
}

/// One triangle of the patch: its corners, its own shape laid flat, and the
/// cotangent weight of each edge.
struct Triangle {
    /// Its corners, as indices into the patch's vertices.
    corners: [u32; 3],
    /// Its second and third corners in its own plane, the first at the origin
    /// and the second on the first axis.
    shape: [DVec2; 2],
    /// Half the cotangent of the corner opposite each edge: `(1, 2)`, `(2, 0)`
    /// and `(0, 1)`.
    weights: [f64; 3],
    /// Its area as its shape asks for it, in square metres.
    area: f64,
    /// Whether it is skin, rather than a strip across one of the patch's holes.
    real: bool,
}

impl Triangle {
    /// The triangle over `corners`, which stand at `at`, its shape `grown` that
    /// many times, unless it is too thin to weigh ([`THINNEST`]).
    fn of(corners: [u32; 3], at: [DVec3; 3], grown: f64, real: bool) -> Option<Self> {
        let [a, b, c] = at;
        let normal = (b - a).cross(c - a);
        let doubled = normal.length();
        let lengths = [(c - b).length(), (a - c).length(), (b - a).length()];
        let longest = lengths.iter().copied().fold(0.0, f64::max);
        let shortest = lengths.iter().copied().fold(f64::INFINITY, f64::min);
        let middle = lengths.iter().sum::<f64>() - longest - shortest;
        if doubled <= THINNEST * longest * middle || doubled <= 0.0 {
            return None;
        }
        let first = (b - a) / lengths[2];
        let second = normal.cross(first) / doubled;
        let cot = |p: DVec3, q: DVec3, r: DVec3| (q - p).dot(r - p) / doubled;
        Some(Self {
            corners,
            shape: [
                DVec2::new(lengths[2], 0.0) * grown,
                DVec2::new((c - a).dot(first), (c - a).dot(second)) * grown,
            ],
            weights: [0.5 * cot(a, b, c), 0.5 * cot(b, c, a), 0.5 * cot(c, a, b)],
            area: 0.5 * doubled * grown * grown,
            real,
        })
    }
}

/// The holes in a patch whose triangles run along `directed` edges: every
/// closed loop of edges with no triangle beyond them, shorter by `apart` than
/// half the longest, which is the patch's border; each in the order its
/// triangles run along it.
fn holes(directed: &mut [(u32, u32)], apart: impl Fn(u32, u32) -> f64) -> Vec<Vec<u32>> {
    directed.sort_unstable();
    let open: Vec<(u32, u32)> = directed
        .iter()
        .filter(|&&(from, to)| directed.binary_search(&(to, from)).is_err())
        .copied()
        .collect();
    let mut used = vec![false; open.len()];
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for first in 0..open.len() {
        if used[first] {
            continue;
        }
        let mut lap = Vec::new();
        let mut edge = first;
        let closed = loop {
            used[edge] = true;
            lap.push(open[edge].0);
            let to = open[edge].1;
            if to == open[first].0 {
                break true;
            }
            let from = open.partition_point(|candidate| candidate.0 < to);
            match (from..open.len())
                .take_while(|&candidate| open[candidate].0 == to)
                .find(|&candidate| !used[candidate])
            {
                Some(next) => edge = next,
                None => break false,
            }
        };
        if closed {
            loops.push(lap);
        }
    }
    let length = |lap: &[u32]| -> f64 {
        (0..lap.len())
            .map(|at| apart(lap[at], lap[(at + 1) % lap.len()]))
            .sum()
    };
    let border = loops.iter().map(|lap| length(lap)).fold(0.0, f64::max);
    loops
        .into_iter()
        .filter(|lap| lap.len() >= 3 && length(lap) < 0.5 * border)
        .collect()
}

/// Triangles across the hole `lap`, run the other way round from it so that
/// they wind as the patch does: a strip zipped from its two ends - the two of
/// its vertices furthest apart by `apart` - along its two sides, each step to
/// whichever side keeps the strip's rung the shorter.
fn zip(lap: &[u32], apart: impl Fn(u32, u32) -> f64) -> Vec<[u32; 3]> {
    let ring: Vec<u32> = lap.iter().rev().copied().collect();
    let count = ring.len();
    let (mut from, mut to, mut widest) = (0, 0, -1.0);
    for one in 0..count {
        for two in one + 1..count {
            let span = apart(ring[one], ring[two]);
            if span > widest {
                (from, to, widest) = (one, two, span);
            }
        }
    }
    // One side forward from one end to the other, the other side back.
    let ahead: Vec<u32> = (0..=(to + count - from) % count)
        .map(|step| ring[(from + step) % count])
        .collect();
    let behind: Vec<u32> = (0..=(from + count - to) % count)
        .map(|step| ring[(from + count - step) % count])
        .collect();
    let (last_ahead, last_behind) = (ahead.len() - 1, behind.len() - 1);
    if last_ahead < 2 || last_behind < 2 {
        return (1..count - 1)
            .map(|at| [ring[0], ring[at], ring[at + 1]])
            .collect();
    }
    let mut out = vec![[behind[1], ahead[0], ahead[1]]];
    let (mut i, mut j) = (1, 1);
    while i < last_ahead - 1 || j < last_behind - 1 {
        let forward = j == last_behind - 1
            || (i < last_ahead - 1
                && apart(ahead[i + 1], behind[j]) <= apart(behind[j + 1], ahead[i]));
        if forward {
            out.push([ahead[i], ahead[i + 1], behind[j]]);
            i += 1;
        } else {
            out.push([behind[j + 1], behind[j], ahead[i]]);
            j += 1;
        }
    }
    out.push([ahead[i], ahead[last_ahead], behind[j]]);
    out
}

/// Unfolds the patch of `mesh` made of `faces`, from the chart `start` puts each
/// vertex at, in metres, each triangle asking for `detail` times its own size
/// at its centroid and its outward normal there; then `reshape`s the fitted
/// places, over the patch's vertices, ascending, and lays every triangle the
/// chart's way round ([`untangle`]) (see [`Unfolded`]).
pub(super) fn unfold(
    mesh: &PolyMesh,
    faces: &[u32],
    start: impl Fn(Vec3) -> Vec2,
    detail: impl Fn(Vec3, Vec3) -> f32,
    reshape: impl FnOnce(&[u32], &mut [DVec2]),
) -> Unfolded {
    let mut vertices: Vec<u32> = faces
        .iter()
        .flat_map(|&face| mesh.faces[face as usize].iter().copied())
        .collect();
    vertices.sort_unstable();
    vertices.dedup();
    let local = |vertex: u32| {
        vertices
            .binary_search(&vertex)
            .expect("a face's corner is among the patch's vertices") as u32
    };
    let count = vertices.len();

    let position = |index: u32| mesh.positions[vertices[index as usize] as usize].as_dvec3();
    let grown = |corners: [u32; 3]| {
        let [a, b, c] = corners.map(position);
        let centroid = (a + b + c) / 3.0;
        let normal = (b - a).cross(c - a).normalize_or_zero();
        f64::from(detail(centroid.as_vec3(), normal.as_vec3()).max(f32::EPSILON))
    };
    let fans: Vec<[u32; 3]> = faces
        .iter()
        .flat_map(|&face| {
            let corners = &mesh.faces[face as usize];
            (1..corners.len().saturating_sub(1))
                .map(move |fan| [corners[0], corners[fan], corners[fan + 1]])
        })
        .map(|corners| corners.map(local))
        .collect();
    // Every edge each way, for finding the patch's holes.
    let mut directed: Vec<(u32, u32)> = fans
        .iter()
        .flat_map(|&[a, b, c]| [(a, b), (b, c), (c, a)])
        .collect();
    let mut triangles: Vec<Triangle> = Vec::new();
    let mut skin = 0.0f64;
    for &corners in &fans {
        if let Some(triangle) = Triangle::of(corners, corners.map(position), grown(corners), true) {
            skin += triangle.area;
            triangles.push(triangle);
        }
    }
    // The patch's holes are closed for the fit, each with a strip of its own
    // shape across it (see [`holes`]): a person's face has the mouth's slit,
    // its pocket set aside, and laid flat with nothing between them the inner
    // roll of each lip unrolled over the other's.
    for hole in holes(&mut directed, |one, two| {
        position(one).distance(position(two))
    }) {
        for corners in zip(&hole, |one, two| position(one).distance(position(two))) {
            if let Some(triangle) =
                Triangle::of(corners, corners.map(position), grown(corners), false)
            {
                triangles.push(triangle);
            }
        }
    }
    drop(directed);

    // The Laplacian's rows: per neighbour, the summed weight of their edge.
    let mut entries: Vec<(u32, u32, f64)> = Vec::with_capacity(6 * triangles.len());
    for triangle in &triangles {
        let [a, b, c] = triangle.corners;
        for ((from, to), weight) in [(b, c), (c, a), (a, b)].into_iter().zip(triangle.weights) {
            entries.push((from, to, weight));
            entries.push((to, from, weight));
        }
    }
    entries.sort_unstable_by_key(|entry| (entry.0, entry.1));
    let mut first = vec![0u32; count + 1];
    let mut neighbour: Vec<u32> = Vec::with_capacity(entries.len() / 2);
    let mut weight: Vec<f64> = Vec::with_capacity(entries.len() / 2);
    let mut diagonal = vec![0.0f64; count];
    let mut at = 0;
    for row in 0..count {
        first[row] = neighbour.len() as u32;
        while at < entries.len() && entries[at].0 as usize == row {
            let column = entries[at].1;
            let mut sum = 0.0;
            while at < entries.len() && entries[at].0 as usize == row && entries[at].1 == column {
                sum += entries[at].2;
                at += 1;
            }
            diagonal[row] += sum;
            neighbour.push(column);
            weight.push(sum);
        }
    }
    first[count] = neighbour.len() as u32;
    drop(entries);

    let mut places: Vec<DVec2> = vertices
        .iter()
        .map(|&vertex| start(mesh.positions[vertex as usize]).as_dvec2())
        .collect();
    // One vertex is held where it starts, which fixes the chart's place; a
    // vertex no triangle weighs is held too. (A row's diagonal can be negative
    // where its triangles are obtuse; the matrix is still the sum of each
    // triangle's own energy, never negative, and the preconditioner takes the
    // diagonal's size.)
    let held = |index: usize| index == 0 || first[index] == first[index + 1];
    let apply = |x: &[DVec2], out: &mut [DVec2]| {
        for row in 0..count {
            let mut sum = diagonal[row] * x[row];
            for entry in first[row] as usize..first[row + 1] as usize {
                sum -= weight[entry] * x[neighbour[entry] as usize];
            }
            out[row] = if held(row) { DVec2::ZERO } else { sum };
        }
    };
    let scale = |index: usize| 1.0 / diagonal[index].abs().max(1e-3);
    let dot = |one: &[DVec2], two: &[DVec2]| {
        one.iter()
            .zip(two)
            .map(|(a, b)| a.x * b.x + a.y * b.y)
            .sum::<f64>()
    };

    let mut target = vec![DVec2::ZERO; count];
    let mut residual = vec![DVec2::ZERO; count];
    let mut step = vec![DVec2::ZERO; count];
    let mut along = vec![DVec2::ZERO; count];
    let mut product = vec![DVec2::ZERO; count];
    for _ in 0..ROUNDS {
        // Each triangle's turn, and what the turned triangles ask of the chart.
        target.iter_mut().for_each(|value| *value = DVec2::ZERO);
        for triangle in &triangles {
            let [a, b, c] = triangle.corners.map(|corner| corner as usize);
            let own = [DVec2::ZERO, triangle.shape[0], triangle.shape[1]];
            let now = [places[a], places[b], places[c]];
            let edges = [(1, 2), (2, 0), (0, 1)];
            let (mut cos, mut sin) = (0.0, 0.0);
            for ((i, j), w) in edges.into_iter().zip(triangle.weights) {
                let (charted, shape) = (now[i] - now[j], own[i] - own[j]);
                cos += w * (charted.x * shape.x + charted.y * shape.y);
                sin += w * (charted.y * shape.x - charted.x * shape.y);
            }
            let length = (cos * cos + sin * sin).sqrt();
            let (cos, sin) = if length > 0.0 {
                (cos / length, sin / length)
            } else {
                (1.0, 0.0)
            };
            let turn = |v: DVec2| DVec2::new(cos * v.x - sin * v.y, sin * v.x + cos * v.y);
            let corners = [a, b, c];
            for ((i, j), w) in edges.into_iter().zip(triangle.weights) {
                let pull = w * turn(own[i] - own[j]);
                target[corners[i]] += pull;
                target[corners[j]] -= pull;
            }
        }
        for (index, value) in target.iter_mut().enumerate() {
            if held(index) {
                *value = DVec2::ZERO;
            }
        }
        // The chart that fits them, from the chart before.
        apply(&places, &mut product);
        for index in 0..count {
            residual[index] = target[index] - product[index];
            step[index] = residual[index] * scale(index);
        }
        along.copy_from_slice(&step);
        let mut reduced = dot(&residual, &step);
        let wanted = SETTLED * dot(&target, &target).sqrt();
        for _ in 0..MOST_STEPS {
            if dot(&residual, &residual).sqrt() <= wanted || reduced <= 0.0 {
                break;
            }
            apply(&along, &mut product);
            let curvature = dot(&along, &product);
            if curvature <= 0.0 {
                break;
            }
            let length = reduced / curvature;
            for index in 0..count {
                places[index] += length * along[index];
                residual[index] -= length * product[index];
                step[index] = residual[index] * scale(index);
            }
            let next = dot(&residual, &step);
            let turn = next / reduced;
            reduced = next;
            for index in 0..count {
                along[index] = step[index] + turn * along[index];
            }
        }
    }

    reshape(&vertices, &mut places);
    untangle(&mut places, &fans, &triangles, &first, &neighbour, position);

    // In metres of skin.
    let charted: f64 = triangles
        .iter()
        .filter(|triangle| triangle.real)
        .map(|triangle| {
            let [a, b, c] = triangle.corners.map(|corner| places[corner as usize]);
            0.5 * (b - a).perp_dot(c - a)
        })
        .sum();
    let scale = (skin / charted.abs().max(f64::MIN_POSITIVE)).sqrt();
    Unfolded {
        vertices,
        places: places
            .iter()
            .map(|place| (*place * scale).as_vec2())
            .collect(),
    }
}

/// How many times at most [`untangle`] relaxes the skin round what is still
/// laid twice once every triangle it could has been laid back by its own
/// shape, each time a ring of faces wider every fourth time.
///
/// On the faces read while it was written, one or two passes laid a person's
/// face chart once (slider corner rc5's 21 turned triangles in one); roll 0,
/// whose border by the neck crossed itself, took nine, and slider corner rc8
/// takes them all (see [`untangle`]). Since #473's third fixer it is left
/// only what laying back cannot do: a border crossing itself, and a fold whose
/// corners must move together - on 8 of the 126 rolls, slider corners and
/// uniform draws #473's critics read, roll 0's lower lip among them.
/// Provenance: **chosen** (#473's second fixer).
const UNTANGLE_PASSES: usize = 24;

/// How many sweeps over the skin it frees one pass of [`untangle`] makes.
/// Provenance: **chosen** (#473's second fixer).
const UNTANGLE_SWEEPS: usize = 16;

/// How many sweeps at most [`untangle`] makes over what is turned over, laying
/// each triangle back the least way it can, before it relaxes what is left: a
/// corner moved may turn a needle beside it over, laid back on the next.
/// Provenance: **chosen** (#473's third fixer): sixty-four laid no more of the
/// rolls' faces than eight.
const LAY_BACK_SWEEPS: usize = 8;

/// How far at the least a corner [`untangle`] moves stands off the edge across
/// from it in each triangle round it, as a share of that triangle's own height
/// over that edge: enough that the triangle stays the chart's way round when
/// its place is rounded to a texture coordinate, little enough that only the
/// thinnest triangles are held to it.
/// Provenance: **chosen** (#473's third fixer).
const LAY_BACK_SHARE: f64 = 0.1;

/// The least height of a triangle, as a share of its longest edge, that
/// [`untangle`] holds a corner off the edge across from it by: a fanned
/// polygon's corners in a line have no height to keep.
/// Provenance: **chosen** (#473's third fixer).
const LAY_BACK_THINNEST: f64 = 1e-4;

/// Lays the chart at `places` once (#473's second fixer): every one of `fans`
/// and of `triangles`' strips across the patch's holes the way the chart runs,
/// no vertex inside the patch gone round twice by its triangles, and the
/// border not crossing itself. `position` is each vertex's place on the skin.
///
/// **First each triangle turned over is laid back the least way it can be**
/// (#473's third fixer). One of its corners is moved to the nearest place
/// where every triangle round that corner is the chart's way round, standing
/// at least [`LAY_BACK_SHARE`] of its own height off the edge across from the
/// corner: the corner of the three that moves least, as long as that turns
/// over less of the triangles round it than before, and no more of them. Most
/// of what a chart has turned over is needles - a fanned polygon's corners all
/// but in a line - that a smooth reshaping bent, the magnified mouth's band
/// (`PersonHead::magnify`) 24 to 50 of them on each of the 43 rolls, and a
/// needle a few thousandths of a millimetre wide is laid back by about as
/// much: over the rolls, a median of 0.007 mm of skin, nine in ten under
/// 0.07 mm. The flattening itself leaves the lips folded too on most faces,
/// up to 28 of their triangles on the rolls, laid back the same way by a
/// median of 0.12 mm. #473's second fixer relaxed every one of these toward
/// the middle of its neighbours over a growing ring of the skin round it,
/// and the needles came out coarse along their length beside the mouth's
/// corners, 3 m a texel at worst.
///
/// **What it cannot lay back is relaxed.** Each fault left's corners, and the
/// skin within a ring or more round them, are moved toward where the
/// triangles round each would have it, each its own shape laid on the edge
/// across from it, on the mean (a corner on the border toward its neighbours
/// off it, the middle of `first` and `neighbour`, the Laplacian's rows), and a
/// move is kept only where it turns over no more of the triangles round it,
/// and less of their area, than before; the border moves only where it is at
/// fault. A fault that will not relax out ([`UNTANGLE_PASSES`]) is left as the
/// fewest faults seen left it: on slider corner rc8, whose neck's zone takes
/// in half the lower lip, the face's border runs through the mouth and crosses
/// itself there.
fn untangle(
    places: &mut [DVec2],
    fans: &[[u32; 3]],
    triangles: &[Triangle],
    first: &[u32],
    neighbour: &[u32],
    position: impl Fn(u32) -> DVec3,
) {
    let signed = |places: &[DVec2], [a, b, c]: [u32; 3]| {
        let [a, b, c] = [a, b, c].map(|corner| places[corner as usize]);
        0.5 * (b - a).perp_dot(c - a)
    };
    let fans: Vec<[u32; 3]> = fans
        .iter()
        .copied()
        .chain(
            triangles
                .iter()
                .filter(|triangle| !triangle.real)
                .map(|triangle| triangle.corners),
        )
        .collect();
    let fans = &fans[..];
    let way: f64 = fans.iter().map(|&fan| signed(places, fan)).sum();
    let count = places.len();
    // The border: every corner on an edge that only one triangle runs along.
    let mut directed: Vec<(u32, u32)> = fans
        .iter()
        .copied()
        .flat_map(|[a, b, c]| [(a, b), (b, c), (c, a)])
        .collect();
    directed.sort_unstable();
    directed.dedup();
    let mut border = vec![false; count];
    let mut rim: Vec<(u32, u32)> = Vec::new();
    for &(from, to) in &directed {
        if directed.binary_search(&(to, from)).is_err() {
            border[from as usize] = true;
            border[to as usize] = true;
            rim.push((from, to));
        }
    }
    drop(directed);
    // The fans round each vertex.
    let mut round = vec![0u32; count + 1];
    for fan in fans {
        for &corner in fan {
            round[corner as usize + 1] += 1;
        }
    }
    for index in 0..count {
        round[index + 1] += round[index];
    }
    let mut filled = round.clone();
    let mut around = vec![0u32; round[count] as usize];
    for (index, fan) in fans.iter().enumerate() {
        for &corner in fan {
            around[filled[corner as usize] as usize] = index as u32;
            filled[corner as usize] += 1;
        }
    }
    drop(filled);
    let turned = |places: &[DVec2], vertex: usize| {
        around[round[vertex] as usize..round[vertex + 1] as usize]
            .iter()
            .map(|&fan| signed(places, fans[fan as usize]) * way)
            .filter(|area| *area < 0.0)
            .fold((0usize, 0.0f64), |(count, sum), area| {
                (count + 1, sum - area)
            })
    };
    // Whether two edges of the border cross, at `places`.
    let crosses = |places: &[DVec2], (a, b): (u32, u32), (c, d): (u32, u32)| {
        if a == c || a == d || b == c || b == d {
            return false;
        }
        let [a, b, c, d] = [a, b, c, d].map(|corner| places[corner as usize]);
        let side = |p: DVec2, q: DVec2, r: DVec2| (q - p).perp_dot(r - p);
        side(a, b, c) * side(a, b, d) < 0.0 && side(c, d, a) * side(c, d, b) < 0.0
    };
    // First each triangle turned over is laid back the least way it can be:
    // one of its corners moved as little as puts every triangle round it the
    // chart's way round, each at least a share of its own shape's height off
    // the edge across from that corner (`LAY_BACK_SHARE`). A needle a smooth
    // reshaping bent is laid back by a few thousandths of a millimetre.
    let height = |[corner, from, to]: [u32; 3]| {
        let edge = position(to) - position(from);
        edge.cross(position(corner) - position(from)).length()
            / edge.length_squared().max(f64::MIN_POSITIVE)
    };
    // Where `corner` stands nearest where it is with every triangle round it
    // laid the chart's way round, if it can: each triangle asks it to stand on
    // its side of a line, and the nearest place that keeps every ask is where
    // it is, or on one line, or where two lines cross.
    let settle = |places: &[DVec2], corner: usize| {
        let mut lines: Vec<(DVec2, f64)> = Vec::new();
        let mut reach = 0.0f64;
        for &fan in &around[round[corner] as usize..round[corner + 1] as usize] {
            let fan = fans[fan as usize];
            let Some(first) = fan.iter().position(|&one| one as usize == corner) else {
                continue;
            };
            let order = [fan[first], fan[(first + 1) % 3], fan[(first + 2) % 3]];
            let (from, to) = (places[order[1] as usize], places[order[2] as usize]);
            let edge = to - from;
            let length = edge.length();
            if length <= 0.0 {
                continue;
            }
            // Stand where `normal . at >= least`.
            reach = reach
                .max(from.distance(places[corner]))
                .max(to.distance(places[corner]));
            let normal = edge.perp() * (way.signum() / length);
            let least = LAY_BACK_SHARE * height(order).max(LAY_BACK_THINNEST) * length;
            lines.push((normal, normal.dot(from) + least));
        }
        let here = places[corner];
        let broken = |at: DVec2| {
            lines
                .iter()
                .filter(|&&(normal, least)| normal.dot(at) < least - 1e-12 * least.abs().max(1.0))
                .count()
        };
        // The fewest asks broken, and the nearest place that breaks no more.
        let mut best = (broken(here), 0.0, here);
        let mut consider = |at: DVec2| {
            let far = at.distance(here);
            if far <= reach {
                let candidate = (broken(at), far, at);
                if (candidate.0, candidate.1) < (best.0, best.1) {
                    best = candidate;
                }
            }
        };
        for &(normal, least) in &lines {
            consider(here + normal * (least - normal.dot(here)));
        }
        for one in 0..lines.len() {
            for two in one + 1..lines.len() {
                let ((n1, l1), (n2, l2)) = (lines[one], lines[two]);
                let det = n1.perp_dot(n2);
                if det.abs() > 1e-12 {
                    consider(DVec2::new(l1 * n2.y - l2 * n1.y, n1.x * l2 - n2.x * l1) / det);
                }
            }
        }
        (best.1 > 0.0).then_some(best.2)
    };
    // Where the triangles round `corner` would have it, on the mean: each
    // its own shape laid on the edge across from it.
    let shaped = |places: &[DVec2], corner: usize| {
        let (mut sum, mut total) = (DVec2::ZERO, 0.0f64);
        for &fan in &around[round[corner] as usize..round[corner + 1] as usize] {
            let fan = fans[fan as usize];
            let Some(first) = fan.iter().position(|&one| one as usize == corner) else {
                continue;
            };
            let order = [fan[first], fan[(first + 1) % 3], fan[(first + 2) % 3]];
            let (from, to) = (places[order[1] as usize], places[order[2] as usize]);
            let edge = position(order[2]) - position(order[1]);
            let length = edge.length_squared();
            if length <= 0.0 || from == to {
                continue;
            }
            let along = (position(order[0]) - position(order[1])).dot(edge) / length;
            sum += from + (to - from) * along + (to - from).perp() * (height(order) * way.signum());
            total += 1.0;
        }
        if total > 0.0 {
            sum / total
        } else {
            places[corner]
        }
    };
    for _ in 0..LAY_BACK_SWEEPS {
        let mut moved = false;
        for &fan in fans {
            if signed(places, fan) * way >= 0.0 {
                continue;
            }
            let mut tries = fan.map(|corner| {
                let corner = corner as usize;
                let place = settle(places, corner);
                let far = place.map_or(f64::INFINITY, |place| place.distance(places[corner]));
                (far, corner, place)
            });
            tries.sort_by(|one, two| one.0.total_cmp(&two.0));
            for (_, corner, place) in tries {
                let Some(place) = place else {
                    continue;
                };
                let here = places[corner];
                let before = turned(places, corner);
                places[corner] = place;
                let after = turned(places, corner);
                if after.0 <= before.0 && after.1 < before.1 {
                    moved = true;
                    break;
                }
                places[corner] = here;
            }
        }
        if !moved {
            break;
        }
    }
    let mut free = vec![false; count];
    let mut bad = vec![false; count];
    // Which vertices a crossing of the border frees, and its ring.
    let mut crossed = vec![false; count];
    // The fewest faults seen, and the chart that had them.
    let mut best = (usize::MAX, places.to_vec());
    for pass in 0..=UNTANGLE_PASSES {
        bad.iter_mut().for_each(|flag| *flag = false);
        crossed.iter_mut().for_each(|flag| *flag = false);
        let mut faults = 0usize;
        let mut turn = vec![0.0f64; count];
        for &fan in fans {
            if signed(places, fan) * way < 0.0 {
                faults += 1;
                for corner in fan {
                    bad[corner as usize] = true;
                }
            }
            // The angle each corner of the fan takes.
            for at in 0..3 {
                let [here, next, last] = [fan[at], fan[(at + 1) % 3], fan[(at + 2) % 3]]
                    .map(|corner| places[corner as usize]);
                let (one, two) = (next - here, last - here);
                turn[fan[at] as usize] += one.perp_dot(two).abs().atan2(one.dot(two));
            }
        }
        // A vertex inside the patch whose triangles go round it twice lays
        // its skin twice without one of them turned over.
        for index in 0..count {
            if !border[index] && turn[index] > 3.0 * std::f64::consts::PI {
                faults += 1;
                bad[index] = true;
            }
        }
        // And the border crossing itself.
        for one in 0..rim.len() {
            for two in one + 1..rim.len() {
                if crosses(places, rim[one], rim[two]) {
                    faults += 1;
                    for corner in [rim[one].0, rim[one].1, rim[two].0, rim[two].1] {
                        bad[corner as usize] = true;
                        crossed[corner as usize] = true;
                    }
                }
            }
        }
        if faults < best.0 {
            best = (faults, places.to_vec());
        }
        if faults == 0 || pass == UNTANGLE_PASSES {
            break;
        }
        free.copy_from_slice(&bad);
        for _ in 0..1 + pass / 4 {
            let ring: Vec<usize> = (0..count)
                .filter(|&index| free[index])
                .flat_map(|index| first[index] as usize..first[index + 1] as usize)
                .map(|entry| neighbour[entry] as usize)
                .collect();
            for index in ring {
                free[index] = true;
            }
            let ring: Vec<usize> = (0..count)
                .filter(|&index| crossed[index])
                .flat_map(|index| first[index] as usize..first[index + 1] as usize)
                .map(|entry| neighbour[entry] as usize)
                .collect();
            for index in ring {
                crossed[index] = true;
            }
        }
        for _ in 0..UNTANGLE_SWEEPS {
            for index in 0..count {
                let entries = first[index] as usize..first[index + 1] as usize;
                if !free[index] || (border[index] && !bad[index]) || entries.is_empty() {
                    continue;
                }
                // A vertex on the border is drawn in toward the skin inside it.
                let inner = entries
                    .clone()
                    .map(|entry| neighbour[entry] as usize)
                    .filter(|&near| !border[index] || !border[near]);
                let (sum, total) = inner.fold((DVec2::ZERO, 0.0f64), |(sum, total), near| {
                    (sum + places[near], total + 1.0)
                });
                let middle = if !border[index] && !crossed[index] {
                    shaped(places, index)
                } else if total > 0.0 {
                    sum / total
                } else {
                    entries
                        .clone()
                        .map(|entry| places[neighbour[entry] as usize])
                        .sum::<DVec2>()
                        / entries.len() as f64
                };
                let here = places[index];
                let before = turned(places, index);
                let mut kept = false;
                for share in [1.0, 0.5, 0.25, 0.125] {
                    places[index] = here + (middle - here) * share;
                    let after = turned(places, index);
                    if after.0 < before.0 || (after.0 == before.0 && after.1 <= before.1) {
                        kept = true;
                        break;
                    }
                }
                if !kept {
                    places[index] = here;
                }
            }
        }
    }
    places.copy_from_slice(&best.1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_needle_bent_over_is_laid_back_by_as_much_as_it_bent() {
        // **A needle turned over is laid back, not relaxed** (#473's third
        // fixer). A polygon with three corners all but in a line, fanned,
        // makes a needle; a smooth reshaping of the chart (the mouth's
        // magnifier) bends the line and turns the needle over. #473's second
        // fixer relaxed every turned triangle's corners toward the middle of
        // their neighbours, and on a person's face left the skin beside the
        // mouth's corners coarse along every needle it laid back. Laid back by
        // its own shape it moves by about as much as it bent.
        //
        // Here a needle (0, 1, 2) a thousandth high over the edge from 0 to 1,
        // inside a patch of seven corners, bent a thousandth the other way:
        // relaxed as #473's second fixer did, its middle corner moved 0.401, a
        // fifth of the needle's length; laid back, 0.0011.
        let skin = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(1.0, 1e-3, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(1.0, -1.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(3.0, 0.0, 0.0),
        ];
        let fans = [
            [0u32, 1, 2],
            [0, 2, 3],
            [2, 1, 3],
            [0, 4, 1],
            [5, 4, 0],
            [5, 0, 3],
            [1, 4, 6],
            [1, 6, 3],
        ];
        // Each vertex's neighbours, as the Laplacian's rows hold them.
        let rows: [&[u32]; 7] = [
            &[1, 2, 3, 4, 5],
            &[0, 2, 3, 4, 6],
            &[0, 1, 3],
            &[0, 1, 2, 5, 6],
            &[0, 1, 5, 6],
            &[0, 3, 4],
            &[1, 3, 4],
        ];
        let mut first = vec![0u32];
        let mut neighbour = Vec::new();
        for row in rows {
            neighbour.extend_from_slice(row);
            first.push(neighbour.len() as u32);
        }
        let mut places: Vec<DVec2> = skin.iter().map(|at| at.truncate()).collect();
        places[2].y = -1e-3;
        let signed = |places: &[DVec2], [a, b, c]: [u32; 3]| {
            let [a, b, c] = [a, b, c].map(|corner| places[corner as usize]);
            (b - a).perp_dot(c - a)
        };
        assert!(
            signed(&places, fans[0]) < 0.0,
            "the needle starts turned over"
        );
        let bent = places[2];
        untangle(&mut places, &fans, &[], &first, &neighbour, |at| {
            skin[at as usize]
        });
        for fan in fans {
            assert!(
                signed(&places, fan) > 0.0,
                "{fan:?} is still turned over at {places:?}"
            );
        }
        let moved = places[2].distance(bent);
        println!("the needle's middle corner moved {moved:.4}");
        assert!(
            moved < 0.01,
            "the needle's middle corner moved {moved:.4} to lay it back"
        );
    }
}
