//! Sculpted facial hair: a closed solid on each facial region (#349).
//!
//! **The scalp shell's second walk, feeding the same solid.** A scalp shell is
//! a cap over a pole: its columns descend meridians of the measured skull from
//! the crown and are welded there. None of the four facial regions is that
//! (#349, measured on three heads). A moustache is a strip 5 to 7 mm tall across
//! the lip, a brow a strip along the ridge, the chin a patch that wraps from the
//! lip's foot over the menton and back under the jaw, and a flank a band on the
//! side of the face - each a PATCH with four edges and no pole. And the skull is
//! not their surface: it is sampled in bands 11 mm tall, which is a whole
//! moustache.
//!
//! So a facial column is a SECTION of the built body: a plane through the
//! region, a fan of rays from a centre inside the head, and the point where each
//! ray leaves the skin. The solid is closed by [`super`]'s own bevel and
//! colours, around the patch's whole boundary rather than its last row.
//!
//! # Why along rays, and not along the skin's normal
//!
//! **An offset along the normal folds wherever the surface turns tighter than
//! the offset is thick**, which is #348's afro finding, and three of the four
//! facial regions turn that tightly (#349, measured): at the midline the
//! moustache band is the crease under the nose, turning 106 to 119 degrees over
//! 14 to 18 mm of arc, the chin turns about 95 degrees from the lip's foot to
//! under the jaw, and a flank turns up to 113 over the jaw's angle. Both of this
//! solid's surfaces are a distance along each row's own ray instead, and a graph
//! over a fan of rays cannot fold however it is shaped. Read along those rays,
//! every one of them leaves the skin once and does not enter the body again -
//! the nose's overhang and the eyelids included - from the centres each
//! region's column names below.

use glam::{Vec2, Vec3};

use super::{BEVEL, SHELL_ROW, THICKNESS, UNDER_SHADE, facet, head_radius};
use crate::hair::clump::{Bed, Root, Shape};
use crate::hair::follicle::{Follicle, Follicles};
use crate::hair::mask::{self, StrandMask};
use crate::mesh::{PolyMesh, VertexSkin};
use crate::rig::Influence;

/// Which facial region a sculpted solid covers, and the axis it carries.
///
/// **A description rather than a curve**, as [`super::Shell`] is: a style names
/// one of these and the geometry is decided here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sculpt {
    /// A closed mass over the chin's patch, hanging from the menton: `length`
    /// runs a short boxed beard to a long spade.
    Chin {
        /// How far below the menton the mass hangs, `0` to `1`.
        length: f32,
    },
    /// A closed band over each flank, from the beard line to the mandible's
    /// crease, meeting the chin's solid under the jaw.
    Flanks,
    /// A solid chevron riding the upper lip: `flare` carries its ends past the
    /// mouth's corners and turns them up.
    Moustache {
        /// How far the ends flare, `0` to `1`.
        flare: f32,
    },
    /// A solid wedge along each brow ridge.
    Brows,
}

impl Sculpt {
    /// The region this sculpt grows on.
    #[must_use]
    pub fn follicle(self) -> Follicle {
        match self {
            Self::Chin { .. } => Follicle::Chin,
            Self::Flanks => Follicle::Flanks,
            Self::Moustache { .. } => Follicle::Moustache,
            Self::Brows => Follicle::Brows,
        }
    }

    /// The patches it is drawn as: one for the chin and the moustache, one a
    /// side for the flanks and the brows.
    fn patches(self) -> Vec<Box<dyn Patch>> {
        match self {
            Self::Chin { length } => vec![Box::new(ChinPatch {
                length: length.clamp(0.0, 1.0),
            })],
            Self::Flanks => vec![
                Box::new(FlankPatch { side: 1.0 }),
                Box::new(FlankPatch { side: -1.0 }),
            ],
            Self::Moustache { flare } => vec![Box::new(LipPatch {
                flare: flare.clamp(0.0, 1.0),
            })],
            Self::Brows => vec![
                Box::new(BrowPatch { side: 1.0 }),
                Box::new(BrowPatch { side: -1.0 }),
            ],
        }
    }
}

/// The [`Shape`] a sculpted facial style grows: no cards, one solid.
///
/// **A shape whose every root is declined**, as a slicked head's rim is: the
/// solid IS the hair, and it is drawn because the region was asked for rather
/// than because a card grew (#346's rule, which every helmet since has kept).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sculpted(pub Sculpt);

impl Shape for Sculpted {
    fn length(&self, _root: &Root) -> f32 {
        0.0
    }

    fn at(&self, root: &Root, _along: f32) -> Vec3 {
        root.at
    }

    fn width(&self, _root: &Root) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn sculpt(&self) -> Option<Sculpt> {
        Some(self.0)
    }
}

/// The mask weight a facial solid's rim sits at.
///
/// **Higher than the scalp's `EDGE`, and the paint is why** (#349). Each sculpted
/// style floors its region's painted layer at full density, and at full density
/// a texel's coverage IS the mask's weight - so a rim at one half leaves the
/// painted fade from one half down to nothing OUTSIDE the solid, where it carries
/// the edge onto the skin, rather than under it.
///
/// Provenance: **derived** from the paint's own coverage.
const RIM: f32 = 0.5;

/// How far a facial solid's inner surface stands off the skin, along its ray,
/// in metres.
///
/// Provenance: **derived** from the measured row stray (#349: 0.2 to 2.5 mm
/// between rows at the grids below), **measured** against the built skin.
const STAND: f32 = 0.003;

/// The most a stand is lengthened along a ray that grazes the skin, as the
/// cosine it is floored at: three and a third times.
///
/// Provenance: **derived** from the rays the patches cast, which leave the skin
/// at a cosine under this only at a patch's very edge.
const GRAZE: f32 = 0.3;

/// How far a facial rim's bevel is carried at most, in thin edges.
///
/// Provenance: **derived** from the defect (see [`close`]).
const BEVEL_REACH: f32 = 2.0;

/// How many rays a column's fan is searched with before its two rim crossings
/// are refined.
///
/// Provenance: **derived**: a brow's span is about 40 degrees of its fan, so this
/// is four rays across it before refinement.
const FAN: usize = 160;

/// How many rays outside a column's rim a run of rays inside it is carried
/// across: a twentieth of the fan.
///
/// Provenance: **measured**, against the widest gap a flank's column has at the
/// crease (nine rays of 160).
const GAP: usize = 12;

/// How many halvings refine a rim crossing, or a patch's lateral end.
///
/// Provenance: **derived**: twelve halvings of a fan's step is under a
/// hundredth of a millimetre at any of these radii.
const REFINE: usize = 12;

/// What share of the middle column's angular span a patch's end column keeps.
///
/// A mask's weight falls to nothing over its fade at a patch's lateral ends, and
/// the column there spans nothing: the end is placed where the span is still
/// this share of the middle's, so no row of the solid collapses to a point.
///
/// Provenance: **tuned by render**.
const END_SPAN: f32 = 0.5;

/// One column's plane and fan: the centre the rays leave from, the two in-plane
/// directions an angle is measured between, and the angles searched from the top
/// of the patch down.
#[derive(Clone, Copy, Debug)]
struct Plane {
    centre: Vec3,
    /// The direction at angle zero.
    out: Vec3,
    /// The direction at a quarter turn.
    up: Vec3,
    /// The first angle searched: the patch's top side.
    from: f32,
    /// The last.
    to: f32,
}

impl Plane {
    fn ray(&self, angle: f32) -> Vec3 {
        (self.out * angle.cos() + self.up * angle.sin()).normalize_or(self.out)
    }

    /// The plane's own normal, for choosing the triangles a column can hit.
    fn normal(&self) -> Vec3 {
        self.out.cross(self.up).normalize_or(Vec3::X)
    }
}

/// Where one ray leaves the skin: the point, how far along the ray it is, and
/// how the body binds the skin there.
#[derive(Clone, Copy, Debug)]
struct Exit {
    at: Vec3,
    distance: f32,
    /// How squarely the ray leaves the skin: the cosine between the ray and the
    /// skin's own normal there.
    square: f32,
    skin: VertexSkin,
}

/// One row of one column, walked: the ray it lies on, where the skin is along
/// it and where the solid's two surfaces are.
#[derive(Clone, Copy, Debug)]
struct Station {
    centre: Vec3,
    ray: Vec3,
    /// The inner surface, along it.
    inner: f32,
    /// The outer surface: along the ray for three of the four patches, and
    /// turned down it under a hanging chin.
    free: Vec3,
    /// How the body holds the skin under it, which every vertex of this
    /// station takes.
    binding: VertexSkin,
}

impl Station {
    fn inner(&self) -> Vec3 {
        self.centre + self.ray * self.inner
    }

    fn outer(&self) -> Vec3 {
        self.free
    }

    /// A point `extra` metres past the inner surface along the ray.
    fn past(&self, extra: f32) -> Vec3 {
        self.centre + self.ray * (self.inner + extra)
    }

    /// How far the outer surface stands off the inner.
    fn thickness(&self) -> f32 {
        self.free.distance(self.inner())
    }
}

/// What a patch is asked while it is walked.
struct Context<'a> {
    head: &'a Follicles,
    /// The head's own half-width: what thickness is sized in.
    radius: f32,
}

/// One facial patch: which plane each column lies in, what the region's rim
/// is, and how thick the solid stands off it.
///
/// **One per region, and per side for the two that have two**, so each
/// region's own geometry lives with it rather than in a match in the walk
/// (the catalogue's per-variant dispatch).
trait Patch {
    /// The region whose mask the rim is read from.
    fn follicle(&self) -> Follicle;

    /// How many columns and rows the solid has.
    fn grid(&self) -> (usize, usize);

    /// The plane of the column a share `u` across the patch's search range.
    fn plane(&self, cx: &Context, u: f32) -> Plane;

    /// Whether a point on the skin is inside this patch's rim: by default where
    /// its region's mask stands at [`RIM`].
    fn inside(&self, cx: &Context, at: Vec3) -> bool {
        cx.head.weight(self.follicle(), at) >= RIM
    }

    /// How far past the skin the inner surface stands, in metres, at a point.
    fn stand(&self, _cx: &Context, _at: Vec3) -> f32 {
        STAND
    }

    /// Where the outer surface is for one station, given where the inner
    /// surface is: `u` across the patch and `s` down it, both `0` to `1`.
    fn outer(&self, cx: &Context, u: f32, s: f32, station: &Station) -> Vec3;

    /// The columns past the walked ones, if the patch adds any. The moustache's
    /// flare.
    fn extend(&self, _cx: &Context, walked: Vec<Vec<Station>>) -> Vec<Vec<Station>> {
        walked
    }
}

/// The least a facial solid is thick anywhere, in metres, on a head of this
/// half-width: the catalogue's thin edge.
fn least(radius: f32) -> f32 {
    radius * THICKNESS[1]
}

/// The head-local triangles of the built body near the head, with the body
/// vertices each corner is.
struct Skin<'a> {
    triangles: Vec<([Vec3; 3], [usize; 3])>,
    weights: &'a crate::rig::SkinWeights,
}

impl<'a> Skin<'a> {
    /// Every triangle of the body within reach of the head.
    fn of(bed: &'a Bed) -> Self {
        let origin = bed.follicles.origin();
        let body = bed.body;
        let mut triangles = Vec::new();
        for face in &body.faces {
            if face.len() < 3 {
                continue;
            }
            let first = body.positions[face[0] as usize] - origin;
            if first.length() > REACH {
                continue;
            }
            for fan in 1..face.len() - 1 {
                let corners = [face[0] as usize, face[fan] as usize, face[fan + 1] as usize];
                triangles.push((
                    corners.map(|corner| body.positions[corner] - origin),
                    corners,
                ));
            }
        }
        Self {
            triangles,
            weights: bed.weights,
        }
    }

    /// The triangles a column in this plane can hit: the ones that straddle it.
    fn slice(&self, plane: &Plane) -> Vec<usize> {
        let normal = plane.normal();
        (0..self.triangles.len())
            .filter(|index| {
                let sides = self.triangles[*index]
                    .0
                    .map(|corner| (corner - plane.centre).dot(normal));
                sides.iter().copied().fold(f32::MAX, f32::min) <= 0.0
                    && sides.iter().copied().fold(f32::MIN, f32::max) >= 0.0
            })
            .collect()
    }

    /// Where a ray from inside the head first leaves the skin.
    ///
    /// **The FIRST hit, not the farthest**: from a centre inside the head the
    /// first crossing is where the ray leaves the skin, and a later one is the
    /// body again - the nose's own far side, past the lip under it.
    fn exit(&self, among: &[usize], centre: Vec3, ray: Vec3) -> Option<Exit> {
        let mut best: Option<(f32, usize, f32, f32)> = None;
        for index in among {
            let ([a, b, c], _) = self.triangles[*index];
            if let Some((distance, u, v)) = moller(centre, ray, a, b, c)
                && best.is_none_or(|(near, ..)| distance < near)
            {
                best = Some((distance, *index, u, v));
            }
        }
        let (distance, index, u, v) = best?;
        let ([a, b, c], corners) = self.triangles[index];
        Some(Exit {
            at: centre + ray * distance,
            distance,
            square: (b - a).cross(c - a).normalize_or(ray).dot(ray).abs(),
            skin: self.blend(corners, [1.0 - u - v, u, v]),
        })
    }

    /// The body's binding at a point inside one of its triangles: the three
    /// corners' influences blended by where the point is, the strongest four
    /// kept.
    fn blend(&self, corners: [usize; 3], shares: [f32; 3]) -> VertexSkin {
        let mut held: Vec<(u16, f32)> = Vec::new();
        for (corner, share) in corners.iter().zip(shares) {
            for influence in &self.weights.vertices[*corner] {
                if influence.weight <= 0.0 {
                    continue;
                }
                let weight = influence.weight * share.max(0.0);
                match held.iter_mut().find(|(joint, _)| *joint == influence.joint) {
                    Some((_, sum)) => *sum += weight,
                    None => held.push((influence.joint, weight)),
                }
            }
        }
        held.sort_by(|one, two| two.1.total_cmp(&one.1).then(one.0.cmp(&two.0)));
        let total: f32 = held.iter().take(4).map(|(_, weight)| weight).sum();
        let mut skin = VertexSkin::default();
        for (slot, (joint, weight)) in held.into_iter().take(4).enumerate() {
            skin[slot] = Influence {
                joint,
                weight: if total > f32::EPSILON {
                    weight / total
                } else {
                    0.0
                },
            };
        }
        skin
    }
}

/// How far from the head's origin a body triangle may be and still be walked,
/// in metres.
///
/// Provenance: **derived**: the farthest facial point on the three measured
/// heads is a flank's back edge, under 130 mm out.
const REACH: f32 = 0.25;

/// How far outside a triangle, in barycentric shares, a ray may pass and still
/// count as meeting it.
///
/// **Because a ray fanned along the body's midline must not fall through it**
/// (found on CI after #351). The body's midline vertices sit a few nanometres
/// off the plane x = 0 rather than on it, and the chin's middle column fans its
/// rays exactly in that plane, so each ray passes through a sliver the width of
/// that error between the triangles either side. Tested strictly, whether it
/// met one of them was decided by the last bit of the `sin` that turned the
/// ray, and when it met neither the walk lost a row and dropped the whole chin.
/// glibc 2.39's `sinf` (CI) and 2.43's round that bit differently, so seed 42's
/// chin was missing on CI and present here; nudging `sinf` or `acosf` one ulp
/// up reproduced CI's five failures exactly, and the two rays that fell through
/// missed their nearest triangle by 5.2e-7 and 4.8e-7 of a share (captured in
/// this module's test).
///
/// Twenty times that, so a libm a few ulps further off still lands: a share of
/// 1e-5 is under 40 nanometres across a 3.6 mm face cell, a ten-thousandth of
/// anything a facial solid draws. A ray within it of an edge meets the triangle
/// there at the point the edge would have given.
const EDGE_SLACK: f32 = 1e-5;

/// Möller-Trumbore: how far along a unit ray it meets a triangle, and where in
/// the triangle, if it does - including a ray along one of its edges, within
/// [`EDGE_SLACK`].
fn moller(origin: Vec3, ray: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, f32, f32)> {
    moller_within(origin, ray, [a, b, c], EDGE_SLACK)
}

/// [`moller`] with the slack given, so a test can ask what the strict test did.
fn moller_within(
    origin: Vec3,
    ray: Vec3,
    [a, b, c]: [Vec3; 3],
    slack: f32,
) -> Option<(f32, f32, f32)> {
    let (one, two) = (b - a, c - a);
    let p = ray.cross(two);
    let det = one.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / det;
    let t = origin - a;
    let u = t.dot(p) * inverse;
    if !(-slack..=1.0 + slack).contains(&u) {
        return None;
    }
    let q = t.cross(one);
    let v = ray.dot(q) * inverse;
    if v < -slack || u + v > 1.0 + slack {
        return None;
    }
    let distance = two.dot(q) * inverse;
    (distance > 0.0).then_some((distance, u, v))
}

/// One column's span: the two angles its rim is crossed at, top first.
fn span(
    skin: &Skin,
    among: &[usize],
    patch: &dyn Patch,
    cx: &Context,
    plane: &Plane,
) -> Option<(f32, f32)> {
    let inside = |angle: f32| {
        skin.exit(among, plane.centre, plane.ray(angle))
            .is_some_and(|exit| patch.inside(cx, exit.at))
    };
    // The longest run of rays inside the rim, a run carried across a gap of
    // at most [`GAP`] rays.
    //
    // **Carried across, because a patch's rim is not always one interval of a
    // column** (#349, measured): a flank's column leaves its rim for one to nine
    // rays at the crease, where the chin's mask is not yet full and the flank's
    // ride under the crease has not begun, and comes back in under the jaw.
    // Kept to the longest run alone, every such column dropped its part under
    // the jaw, and the seam there read a quarter bare on seed 7.
    let angle = |step: usize| plane.from + (plane.to - plane.from) * step as f32 / FAN as f32;
    let mut best: Option<(usize, usize)> = None;
    let mut run: Option<(usize, usize)> = None;
    for step in 0..=FAN {
        if inside(angle(step)) {
            let start = match run {
                Some((start, last)) if step - last <= GAP + 1 => start,
                _ => step,
            };
            run = Some((start, step));
            if best.is_none_or(|(from, to)| step - start > to - from) {
                best = Some((start, step));
            }
        }
    }
    let (first, last) = best?;
    if last == first {
        return None;
    }
    // Refined to where the rim is crossed, between the last ray outside and the
    // first inside at each end.
    let refine = |mut outside: f32, mut within: f32| {
        for _ in 0..REFINE {
            let middle = (outside + within) * 0.5;
            if inside(middle) {
                within = middle;
            } else {
                outside = middle;
            }
        }
        within
    };
    let top = if first == 0 {
        angle(0)
    } else {
        refine(angle(first - 1), angle(first))
    };
    let bottom = if last == FAN {
        angle(FAN)
    } else {
        refine(angle(last + 1), angle(last))
    };
    Some((top, bottom))
}

/// Walks one patch: its columns placed across the region where the rim spans
/// enough of each, and its rows at even shares of each column's span.
fn walk(skin: &Skin, patch: &dyn Patch, cx: &Context) -> Option<Vec<Vec<Station>>> {
    let (columns, rows) = patch.grid();
    let measure = |u: f32| {
        let plane = patch.plane(cx, u);
        let among = skin.slice(&plane);
        span(skin, &among, patch, cx, &plane).map(|(top, bottom)| (bottom - top).abs())
    };
    // The patch's lateral ends: where the span has fallen to END_SPAN of the
    // middle's, found by halving from the middle outward.
    let middle = measure(0.5)?;
    let enough = |u: f32| measure(u).is_some_and(|width| width >= middle * END_SPAN);
    let end = |outside: f32| {
        let (mut out, mut within) = (outside, 0.5f32);
        if enough(out) {
            return out;
        }
        for _ in 0..REFINE {
            let halfway = (out + within) * 0.5;
            if enough(halfway) {
                within = halfway;
            } else {
                out = halfway;
            }
        }
        within
    };
    let (low, high) = (end(0.0), end(1.0));
    let mut walked = Vec::with_capacity(columns);
    for column in 0..columns {
        let u_search = low + (high - low) * column as f32 / (columns - 1).max(1) as f32;
        let u = column as f32 / (columns - 1).max(1) as f32;
        let plane = patch.plane(cx, u_search);
        let among = skin.slice(&plane);
        let (top, bottom) = span(skin, &among, patch, cx, &plane)?;
        let mut stations = Vec::with_capacity(rows);
        for row in 0..rows {
            let s = row as f32 / (rows - 1).max(1) as f32;
            let angle = top + (bottom - top) * s;
            let ray = plane.ray(angle);
            let exit = skin.exit(&among, plane.centre, ray)?;
            // **A stand is off the SKIN, so it is taken square to it** (#349,
            // measured): along a ray that grazes the jaw a few millimetres of
            // ray is under one of skin, and the chin's side column on seed 42
            // put its corner inside the flank it was lifted over.
            let inner = exit.distance + patch.stand(cx, exit.at) / exit.square.max(GRAZE);
            let mut station = Station {
                centre: plane.centre,
                ray,
                inner,
                free: Vec3::ZERO,
                binding: exit.skin,
            };
            station.free = station.past(0.0);
            let free = patch.outer(cx, u, s, &station);
            // Never thinner than the thin edge: a surface that met its inner
            // partner would weld two vertices and open the solid.
            // And never not a number: a comparison with NaN is false, so a
            // profile that produced one would pass the edge test above and put
            // the NaN in the mesh.
            station.free = if !free.is_finite() || free.distance(station.inner()) < least(cx.radius)
            {
                station.past(least(cx.radius))
            } else {
                free
            };
            stations.push(station);
        }
        walked.push(stations);
    }
    Some(patch.extend(cx, walked))
}

/// Draws one sculpted facial solid into `into` and returns what it cost in
/// triangles.
///
/// **Bound like the skin it covers**, vertex by vertex, every vertex of a
/// station taking the binding of the skin its ray leaves (#349, measured): the
/// chin's skin is the mandible's on every vertex and a brow's and a moustache's
/// the head's, while a flank's runs from the head at the beard line to the jaw
/// under it and stretches up to five times over as the jaw opens. A solid bound
/// rigidly to either bone tears off the skin it covers; one bound as that skin
/// is shears with it. So the beard opens with the mouth and the moustache does
/// not, and with the jaw open twenty degrees no vertex of any of the four is
/// under the skin on three heads.
///
/// **A hanging chin is NOT handed over to the head along its hang**, which is
/// what a beard's cards do and what this was built with first. Measured on the
/// posed mesh, every way of doing it broke the solid: handed over by each
/// vertex's own depth the hang sheared through itself (up to 175 degrees
/// creased, half its volume gone); by one share for the whole hang, 36 to 48
/// edges still creased at 0.5 and at 1.0; and a bevel bound to the skin half way
/// down the hang's back wall swung 37 mm into the neck. Bound as the chin's skin
/// is, the solid moves rigidly with the mandible and cannot shear - and what had
/// swung into the throat was the hang's BACK, which the chin's underside now
/// slopes up from (see `ChinPatch`).
pub(in crate::hair) fn loft(
    into: &mut PolyMesh,
    bed: &Bed,
    sculpt: &Sculpt,
    roots: Vec3,
    tips: Vec3,
) -> usize {
    let skin = Skin::of(bed);
    let cx = Context {
        head: bed.follicles,
        radius: head_radius(bed.follicles),
    };
    let before = into.faces.len();
    for patch in sculpt.patches() {
        if let Some(walked) = walk(&skin, patch.as_ref(), &cx) {
            close(into, &walked, roots, tips, cx.radius);
        }
    }
    facet(into, before..into.faces.len());
    into.faces[before..].iter().map(|face| face.len() - 2).sum()
}

/// Closes one walked patch into a solid: the outer and inner surfaces and the
/// bevelled ring round the whole of its boundary.
///
/// **The scalp shell's own closure with its topology changed** (see
/// [`super::loft`]): the same bevel carried past the two surfaces it joins, the
/// same row of the strand mask, the same under-shade on the surface facing the
/// skin. What differs is that a patch has no pole to weld and its rim is its
/// whole boundary - first row, last column, last row, first column - rather than
/// its last row alone.
fn close(into: &mut PolyMesh, walked: &[Vec<Station>], roots: Vec3, tips: Vec3, radius: f32) {
    let columns = walked.len();
    let rows = walked.first().map_or(0, Vec::len);
    if columns < 2 || rows < 2 {
        return;
    }
    let first = into.positions.len() as u32;
    let at = |column: usize, row: usize| column * rows + row;
    let outer: Vec<Vec3> = walked.iter().flatten().map(Station::outer).collect();
    let inner: Vec<Vec3> = walked.iter().flatten().map(Station::inner).collect();
    let stations: Vec<&Station> = walked.iter().flatten().collect();
    let (lane_from, lane_to) = StrandMask::lane_span(mask::lane_of(outer[0]));
    let uv = Vec2::new((lane_from + lane_to) * 0.5, SHELL_ROW);
    let under = (roots * UNDER_SHADE).clamp(Vec3::ZERO, Vec3::ONE);
    // How far the solid stands off anywhere, which the outer surface's colour
    // is a share of: roots where it lies on the skin, tips where it is free.
    let deepest = stations
        .iter()
        .map(|station| station.thickness())
        .fold(least(radius), f32::max);
    let push = |into: &mut PolyMesh, point: Vec3, normal: Vec3, colour: Vec3, skin: VertexSkin| {
        into.positions.push(point);
        into.normals.push(normal);
        into.uvs.push(uv);
        into.colours.push(colour);
        into.skin.push(skin);
    };
    for station in &stations {
        let free = (station.thickness() / deepest).clamp(0.0, 1.0);
        push(
            into,
            station.outer(),
            station.ray,
            roots.lerp(tips, free),
            station.binding,
        );
    }
    for station in &stations {
        push(into, station.inner(), -station.ray, under, station.binding);
    }
    // Which way round the grid's quads face out: the patch's own columns and
    // rows may run either way across the face, so it is read off the first
    // quad rather than assumed.
    let quad = [at(0, 0), at(0, 1), at(1, 1), at(1, 0)];
    let mut newell = Vec3::ZERO;
    for (index, corner) in quad.iter().enumerate() {
        let here = outer[*corner];
        let there = outer[quad[(index + 1) % 4]];
        newell += (here - there).cross(here + there);
    }
    let flip = newell.dot(stations[at(0, 0)].ray) < 0.0;
    let outer_of = |index: usize| first + index as u32;
    let inner_of = |index: usize| first + (columns * rows) as u32 + index as u32;
    // The boundary, in one loop: along the first row, down the last column,
    // back along the last row and up the first column. Each entry is a grid
    // index and the interior neighbour it heads away from.
    let mut boundary: Vec<(usize, Vec<usize>)> = Vec::new();
    for column in 0..columns {
        boundary.push((at(column, 0), vec![at(column, 1)]));
    }
    for row in 1..rows {
        boundary.push((at(columns - 1, row), vec![at(columns - 2, row)]));
    }
    for column in (0..columns - 1).rev() {
        boundary.push((at(column, rows - 1), vec![at(column, rows - 2)]));
    }
    for row in (1..rows - 1).rev() {
        boundary.push((at(0, row), vec![at(1, row)]));
    }
    // The corners head away from both neighbours.
    let corners = [
        (at(0, 0), at(0, 1), at(1, 0)),
        (at(columns - 1, 0), at(columns - 1, 1), at(columns - 2, 0)),
        (
            at(columns - 1, rows - 1),
            at(columns - 1, rows - 2),
            at(columns - 2, rows - 1),
        ),
        (at(0, rows - 1), at(0, rows - 2), at(1, rows - 1)),
    ];
    for (corner, one, two) in corners {
        if let Some(entry) = boundary.iter_mut().find(|(index, _)| *index == corner) {
            entry.1 = vec![one, two];
        }
    }
    let bevel_first = first + (2 * columns * rows) as u32;
    let mut bevel_of = std::collections::HashMap::new();
    for (slot, (index, from)) in boundary.iter().enumerate() {
        bevel_of.insert(*index, bevel_first + slot as u32);
        let (out, inside) = (outer[*index], inner[*index]);
        let heading = from
            .iter()
            .map(|neighbour| (inside - inner[*neighbour]).normalize_or(Vec3::ZERO))
            .sum::<Vec3>()
            .normalize_or(-stations[*index].ray);
        let across = (out - inside).normalize_or(stations[*index].ray);
        // **Carried by the thin edge's measure, not the wall's** (#349,
        // measured): a hanging chin's back wall is tens of millimetres thick,
        // and a bevel carried six tenths of that past it went 24 mm back into
        // the neck on seed 42.
        let thick = out.distance(inside).min(least(radius) * BEVEL_REACH);
        push(
            into,
            (out + inside) * 0.5 + heading * (thick * BEVEL),
            (across + heading).normalize_or(across),
            roots,
            stations[*index].binding,
        );
    }
    let mut face = |corners: [u32; 4]| {
        if flip {
            into.faces
                .push(vec![corners[3], corners[2], corners[1], corners[0]]);
        } else {
            into.faces.push(corners.to_vec());
        }
    };
    for column in 0..columns - 1 {
        for row in 0..rows - 1 {
            face([
                outer_of(at(column, row)),
                outer_of(at(column, row + 1)),
                outer_of(at(column + 1, row + 1)),
                outer_of(at(column + 1, row)),
            ]);
            face([
                inner_of(at(column, row)),
                inner_of(at(column + 1, row)),
                inner_of(at(column + 1, row + 1)),
                inner_of(at(column, row + 1)),
            ]);
        }
    }
    // The ring, one pair of quads a boundary edge, wound against the edge as
    // the outer quad beside it has it.
    let loop_len = boundary.len();
    for slot in 0..loop_len {
        let (one, _) = &boundary[slot];
        let (two, _) = &boundary[(slot + 1) % loop_len];
        // The loop runs along the first row with increasing column, which is
        // the direction the outer quad [c,0 .. c+1,0] traverses in reverse
        // (its last edge runs c+1,0 -> c,0); so the ring quad on edge
        // one -> two is [two, bevel two, bevel one, one] in the unflipped
        // winding.
        face([outer_of(*two), bevel_of[two], bevel_of[one], outer_of(*one)]);
        face([bevel_of[two], inner_of(*two), inner_of(*one), bevel_of[one]]);
    }
}

/// The chin's patch: columns fanned round one vertical axis behind the chin, all
/// its rays leaving one centre inside the jaw.
struct ChinPatch {
    length: f32,
}

/// The moustache's patch: sagittal columns across the lip, rays from behind it at
/// the rim's own floor.
struct LipPatch {
    flare: f32,
}

/// One flank's patch: meridian columns, rays from the head's axis.
struct FlankPatch {
    side: f32,
}

/// One brow's patch: meridian columns, rays from the head's axis at the ridge.
struct BrowPatch {
    side: f32,
}

/// How far round its fan the chin's columns are searched, as a share of the
/// patch's half-width seen from the fan's centre.
///
/// The patch's mask decides where its columns actually end (see `END_SPAN`);
/// this only bounds the search. Past about this far a column grazes the side of
/// the jaw (#349, measured), and the flanks cover the jaw from there - running on
/// in under the chin's solid, so the two overlap.
///
/// Provenance: **derived** from the measured sections.
const CHIN_ACROSS: f32 = 0.85;

/// How far behind the chin's front its rays leave from, in metres.
///
/// Provenance: **derived** from the room read along the rays (#349): from
/// here every row's ray leaves the skin once.
const CHIN_BEHIND: f32 = 0.045;

/// How far behind the chin's front its solid reaches back under the jaw, in
/// metres.
///
/// **Short of the throat, because the chin's solid moves with the mandible**
/// (#349, the posed census): at 45 mm its back corners swung 10 mm into the
/// throat with the jaw open twenty degrees, on the short beard as on the long
/// one. The flanks carry on under the jaw behind it.
///
/// Provenance: **bounded by the posed census**.
const CHIN_BACK: f32 = 0.030;

/// How far above the menton its rays leave from, in metres.
///
/// Provenance: **derived** from the room read along the rays (#349), with
/// [`CHIN_BEHIND`].
const CHIN_ABOVE: f32 = 0.012;

/// How thick the chin's solid stands over its front, as a share of the head's
/// half-width.
///
/// Provenance: **tuned by render**.
const CHIN_THICK: f32 = 0.09;

/// How far below the menton the chin's mass hangs at each end of its axis, in
/// metres.
///
/// In metres for the chin catalogue's own reason: a hand's breadth of beard is
/// a hand's breadth on any head.
///
/// Provenance: **tuned by render**, from the card beards' reach.
const CHIN_HANG: [f32; 2] = [0.014, 0.070];

/// Where down the chin's rows its hang comes in, as a share of the column.
///
/// Provenance: **tuned by render**.
const CHIN_HANG_FROM: f32 = 0.10;

/// Where down the chin's rows its hang starts to go out again, as a share of the
/// column: it is gone by the back row.
///
/// **So nothing hangs behind the chin** (#349, the posed census): a hang carried
/// to the back row is a wall straight down from the throat's crease, and moving
/// with the mandible that wall's foot swung into the throat.
///
/// Provenance: **bounded by the posed census**, **tuned by render**.
const CHIN_HANG_TO: f32 = 0.60;

/// Which way a chin's hang falls: down, and a little forward.
///
/// Provenance: **tuned by render**.
const CHIN_DOWN: Vec3 = Vec3::new(0.0, -1.0, 0.25);

/// How much shorter the hang is at the chin's sides than at its middle.
///
/// Provenance: **tuned by render**.
const CHIN_POINT: f32 = 0.45;

/// How far the chin's inner surface is lifted over the flanks where the two
/// overlap, as a share of the head's half-width: the flanks' own thickness and
/// a gap, so the flanks' front rim lies under the chin's solid.
///
/// Provenance: **derived** from [`FLANK_THICK`], the gap **bounded by the overlap
/// guard** (at a gap of 0.02 a flank's front corner still sat inside the chin on
/// seed 42).
const CHIN_OVER_FLANKS: f32 = FLANK_THICK + 0.04;

/// What share of the rim's weight the flanks' mask has to reach before the chin
/// is lifted over it in full.
///
/// **Well under the rim, because the flanks' solid starts AT the rim** (#349,
/// measured): eased in over the whole of it, the chin's last side column on seed
/// 42 was lifted only part way where the flanks' front corner is, and four of its
/// vertices sat inside each flank's solid and ten of each flank's inside the
/// chin's.
///
/// Provenance: **bounded by the overlap guard**.
const OVER_FROM: f32 = 0.2;

impl Patch for ChinPatch {
    fn follicle(&self) -> Follicle {
        Follicle::Chin
    }

    /// Nine columns by seven rows, 304 triangles: a column every 7 to 8 mm
    /// across a 50 to 73 mm patch, and rows enough for the lip's foot, the
    /// menton's turn and the hang (#349, costed against the dearest card chin's
    /// 546).
    fn grid(&self) -> (usize, usize) {
        (9, 7)
    }

    fn plane(&self, cx: &Context, u: f32) -> Plane {
        let pad = cx.head.pad();
        // **Fanned round one vertical axis behind the chin, not sliced
        // parallel** (#349, rendered and measured): a sagittal slice near the
        // patch's side grazes the jaw and wraps back along it, which folded the
        // side columns on the narrow seed 42 head and, cut in to avoid it, left
        // a bar a goatee wide. Round an axis, a side column faces out along the
        // jaw's own curve, and every ray of the patch leaves one centre.
        let reach = (pad.half * CHIN_ACROSS).atan2(CHIN_BEHIND);
        let azimuth = reach * (u * 2.0 - 1.0);
        Plane {
            centre: Vec3::new(0.0, pad.menton + CHIN_ABOVE, pad.front - CHIN_BEHIND),
            out: Vec3::new(azimuth.sin(), 0.0, azimuth.cos()),
            up: Vec3::Y,
            from: 1.2,
            to: -2.6,
        }
    }

    fn inside(&self, cx: &Context, at: Vec3) -> bool {
        // And nothing behind the fan's own centre: past it a section has run
        // back along the side of the jaw toward the neck.
        cx.head.weight(Follicle::Chin, at) >= RIM && at.z >= cx.head.pad().front - CHIN_BACK
    }

    fn stand(&self, cx: &Context, at: Vec3) -> f32 {
        STAND
            + cx.radius
                * CHIN_OVER_FLANKS
                * crate::face::smooth(cx.head.weight(Follicle::Flanks, at) / (RIM * OVER_FROM))
    }

    fn outer(&self, cx: &Context, u: f32, s: f32, station: &Station) -> Vec3 {
        let pad = cx.head.pad();
        let across = (u * 2.0 - 1.0).abs();
        // Full over the front from a thin edge at the lip's foot, along the ray.
        let thick = cx.radius * CHIN_THICK * crate::face::smooth(s / 0.35);
        // **And hanging: blended toward the point below this one on the hang's
        // floor** (#349, measured). From a centre behind the chin the rays of
        // its front rows point nearly forward, and along them the floor was
        // reached by none. Turning the DIRECTION toward down instead threw the
        // rows where the hang begins 90 mm forward, since a direction still
        // nearly level has to travel far to fall at all. Blending the two
        // POSITIONS - the front's thickness along the ray, and the floor below -
        // moves every row down by its own share and no further.
        let hangs = crate::face::smooth((s - CHIN_HANG_FROM) / 0.30)
            * (1.0 - crate::face::smooth((s - CHIN_HANG_TO) / (1.0 - CHIN_HANG_TO)));
        let hang = CHIN_HANG[0] + (CHIN_HANG[1] - CHIN_HANG[0]) * self.length;
        let floor = pad.menton - hang * (1.0 - CHIN_POINT * across * across);
        let front = station.past(thick);
        let down = CHIN_DOWN.normalize();
        let below = front + down * ((front.y - floor) / -down.y).max(0.0);
        front.lerp(below, hangs)
    }
}

/// How thick a moustache stands at its middle, as a share of the head's
/// half-width.
///
/// Provenance: **tuned by render**.
const LIP_THICK: f32 = 0.075;

/// How much of that its ends keep.
///
/// Provenance: **tuned by render**.
const LIP_ENDS: f32 = 0.45;

/// How far behind the lip the moustache's rays leave from, in metres.
///
/// Provenance: **derived** from the room read along the rays (#349): from here
/// every ray leaves the skin once, the nose's 13 mm overhang included.
const LIP_BEHIND: f32 = 0.040;

/// How far past the lip's half-width a full flare carries the ends, as a share
/// of the half-width.
///
/// Provenance: **tuned by render**.
const FLARE_OUT: f32 = 0.45;

/// How high a full flare turns the ends up, as a share of the half-width.
///
/// Provenance: **tuned by render**.
const FLARE_UP: f32 = 0.35;

impl Patch for LipPatch {
    fn follicle(&self) -> Follicle {
        Follicle::Moustache
    }

    /// Eleven columns by four rows, 224 triangles, and 264 with the flare's two
    /// end columns: a column every 4 to 6 mm across a 39 to 57 mm lip, and four
    /// rows over a 5 to 7 mm band that turns up to 119 degrees under the nose
    /// (#349, costed against the dearest card moustache's 336).
    fn grid(&self) -> (usize, usize) {
        (11, 4)
    }

    fn plane(&self, cx: &Context, u: f32) -> Plane {
        let lip = cx.head.lip();
        let x = lip.half * (u * 2.0 - 1.0);
        // **At the rim's own floor, so no ray points down** (#349): every
        // vertex of the solid is then at least as high as the lowest point of
        // its rim, which the mask puts above the vermilion - raised by the most
        // the bottom edge's bevel can carry below it, so the bevel may head down
        // the lip like every other rim's. Turned level instead, the bevel ring
        // folded back over the edge (#349, 20 to 28 edges creased, up to 180
        // degrees, all along the bottom row).
        let floor = lip.vermilion + lip.fade * 0.5 + least(cx.radius) * BEVEL;
        let front = cx.head.skull().surface_at(lip.vermilion, 0.0).z;
        Plane {
            centre: Vec3::new(x, floor, front - LIP_BEHIND),
            out: Vec3::Z,
            up: Vec3::Y,
            from: 1.4,
            to: 0.0,
        }
    }

    fn outer(&self, cx: &Context, u: f32, s: f32, station: &Station) -> Vec3 {
        let across = (u * 2.0 - 1.0).abs();
        // Floored at nothing before the power: `sin(pi)` is a hair under zero in
        // floating point, and a fractional power of it is NaN (#349, found by
        // the closed-solid guard as one open edge and three solids welded into
        // one at the NaN they shared).
        let profile = (std::f32::consts::PI * s.clamp(0.0, 1.0))
            .sin()
            .max(0.0)
            .powf(0.6);
        let thick = cx.radius * LIP_THICK * profile * (1.0 - (1.0 - LIP_ENDS) * across * across);
        station.past(thick)
    }

    fn extend(&self, cx: &Context, walked: Vec<Vec<Station>>) -> Vec<Vec<Station>> {
        if self.flare <= 0.0 || walked.len() < 2 {
            return walked;
        }
        let lip = cx.head.lip();
        let out = lip.half * FLARE_OUT * self.flare;
        let up = lip.half * FLARE_UP * self.flare;
        // One column past each end, carried out along the lip, turned up and
        // drawn to a point about its own middle row.
        let tip = |from: &Vec<Station>, side: f32| -> Vec<Station> {
            let rows = from.len();
            let middle = (from[rows / 2].inner() + from[rows / 2].outer()) * 0.5;
            from.iter()
                .map(|station| {
                    let shrink = |point: Vec3| middle + (point - middle) * 0.25;
                    let lift = Vec3::new(side * out, up, -out * 0.3);
                    let inner = shrink(station.inner()) + lift;
                    let outer = shrink(station.outer()) + lift;
                    let centre = inner - station.ray * station.inner;
                    let thick = (outer - inner).dot(station.ray).max(least(cx.radius) * 0.5);
                    Station {
                        centre,
                        ray: station.ray,
                        inner: station.inner,
                        free: inner + station.ray * thick,
                        binding: station.binding,
                    }
                })
                .collect()
        };
        let mut all = Vec::with_capacity(walked.len() + 2);
        let first = tip(&walked[0], -1.0);
        let last = tip(&walked[walked.len() - 1], 1.0);
        all.push(first);
        all.extend(walked);
        all.push(last);
        all
    }
}

/// How thick a flank's solid stands over the cheek, as a share of the head's
/// half-width.
///
/// Provenance: **tuned by render**.
const FLANK_THICK: f32 = 0.05;

/// What share of the rim's weight a flank's solid runs in to where it lies under
/// the chin's patch.
///
/// Above the weight the chin's lift over the flanks comes in fully at
/// (`OVER_FROM`), so wherever a flank's solid is the chin's stands clear of it.
///
/// Provenance: **derived** from the chin's lift, **bounded by the overlap
/// guard**.
const UNDER_CHIN: f32 = 0.4;

/// How far below the mandible's crease a flank's solid may reach, in metres,
/// where there is no chin's patch to run on under.
///
/// Provenance: **carried** from the card flanks' own least ride (`RIDES`).
const FLANK_RIDE: f32 = 0.002;

/// How far round from dead ahead a flank's columns are searched, in radians:
/// from the chin's own patch to behind the ear.
///
/// Provenance: **derived** from the measured mask (#349: 17 to 101 degrees at
/// weight 0.35 on three heads), searched a little wider both ways.
const FLANK_FROM: [f32; 2] = [0.20, 1.95];

impl Patch for FlankPatch {
    fn follicle(&self) -> Follicle {
        Follicle::Flanks
    }

    /// Eight columns by six rows a side, 236 triangles: rows every 5 to 16 mm
    /// down columns of 26 to 81 mm, which strayed 0.4 to 2.4 mm at six rows
    /// against the stand (#349, costed at 472 for both against the dearest card
    /// flanks' 588).
    fn grid(&self) -> (usize, usize) {
        (8, 6)
    }

    fn plane(&self, cx: &Context, u: f32) -> Plane {
        let line = cx.head.beard_line();
        let azimuth = self.side * (FLANK_FROM[0] + (FLANK_FROM[1] - FLANK_FROM[0]) * u);
        Plane {
            centre: Vec3::new(0.0, (line.sideburn + cx.head.jawline(0.0)) * 0.5, 0.0),
            out: Vec3::new(azimuth.sin(), 0.0, azimuth.cos()),
            up: Vec3::Y,
            from: 1.2,
            to: -1.45,
        }
    }

    fn inside(&self, cx: &Context, at: Vec3) -> bool {
        let reach = (at.x * at.x + at.z * at.z).sqrt();
        let facing = if reach > f32::EPSILON {
            at.z / reach
        } else {
            1.0
        };
        let flanks = cx.head.weight(Follicle::Flanks, at);
        // **Carried in under the chin's patch past its own rim** (#349,
        // measured by the overlap guard): at its own rim the flank's front edge
        // stopped where the chin's side edge did, 0.2 to 0.3 mm short of it on
        // two heads - two solids touching, with the seam between them in plain
        // view. Where the chin's own mask is full the flank runs on in to a
        // lower weight, under the chin's solid, which is lifted over it there.
        let under_chin = cx.head.weight(Follicle::Chin, at) >= RIM;
        let rim = if under_chin { RIM * UNDER_CHIN } else { RIM };
        // And under the crease there too, as far as the flanks' own mask reaches
        // below the border: the chin's solid stops short of the neck at the
        // jaw's sides, and between the two the skin under the jaw showed - 19
        // per cent of the seam bare on the default head, all 4 to 13 mm under
        // the crease, and on seed 7's bigger head 25 per cent down to 16 mm
        // (#349, the seam guard's column reading). A reach in millimetres was
        // the first cut and is the head-size trap again.
        let ride = if under_chin {
            cx.head.beard_line().under
        } else {
            FLANK_RIDE
        };
        flanks >= rim && at.y >= cx.head.jawline(facing) - ride
    }

    fn outer(&self, cx: &Context, u: f32, s: f32, station: &Station) -> Vec3 {
        let body = (std::f32::consts::PI * s).sin().max(0.0)
            * (std::f32::consts::PI * u).sin().max(0.0).sqrt();
        station.past(cx.radius * FLANK_THICK * body)
    }
}

/// How thick a brow's wedge stands at its thickest, as a share of the head's
/// half-width.
///
/// Provenance: **tuned by render**.
const BROW_THICK: f32 = 0.07;

/// How much of that the tail keeps.
///
/// Provenance: **tuned by render**, after the card brows' own thinning tail.
const BROW_TAIL: f32 = 0.35;

impl Patch for BrowPatch {
    fn follicle(&self) -> Follicle {
        Follicle::Brows
    }

    /// Six columns by two rows a side, 68 triangles: a wedge's two rows are its
    /// two edges, across a 9 to 14 mm arc they stray at most 1.5 mm from, under
    /// the stand (#349, costed at 136 for both against the dearest card brows'
    /// 140).
    fn grid(&self) -> (usize, usize) {
        (6, 2)
    }

    fn plane(&self, cx: &Context, u: f32) -> Plane {
        let ridge = cx.head.brow_ridge();
        // Searched a little past both ends, so the patch's own ends are where
        // the mask says.
        let along = -0.1 + 1.2 * u;
        let x = ridge.inner + (ridge.outer - ridge.inner) * along;
        let level = ridge.height(along);
        // The azimuth whose surface point at the ridge's level is `x` out.
        let mut azimuth = 0.3f32;
        for _ in 0..3 {
            let at = cx.head.skull().surface_at(level, azimuth);
            let round = (at.x * at.x + at.z * at.z).sqrt().max(0.01);
            azimuth = (x / round).clamp(-1.0, 1.0).asin();
        }
        let azimuth = azimuth * self.side;
        Plane {
            centre: Vec3::new(0.0, level, 0.0),
            out: Vec3::new(azimuth.sin(), 0.0, azimuth.cos()),
            up: Vec3::Y,
            from: 0.8,
            to: -0.8,
        }
    }

    fn outer(&self, cx: &Context, u: f32, s: f32, station: &Station) -> Vec3 {
        // Thickest along its upper edge's middle, down to the thin edge at its
        // lower one, so nothing hangs over the eye; and thinning to the tail.
        // Both sides' columns run from the inner end to the tail.
        let tail = 1.0 - (1.0 - BROW_TAIL) * u;
        station.past(cx.radius * BROW_THICK * (1.0 - s) * tail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits(v: [u32; 3]) -> Vec3 {
        Vec3::new(
            f32::from_bits(v[0]),
            f32::from_bits(v[1]),
            f32::from_bits(v[2]),
        )
    }

    #[test]
    fn a_ray_fanned_along_the_midline_does_not_fall_through_it() {
        // The two rays CI's glibc sent between the body's triangles on seed 42's
        // chin, captured bit for bit off the built body with `sinf` nudged one
        // ulp up (which reproduced CI's failures), each with the triangle it
        // came nearest to meeting. Strictly, each misses; with the slack, each
        // meets it. Pure arithmetic on stored bits, so the same on every libm.
        let centre = bits([0x0, 0xbdb2_9466, 0x3d56_09f4]);
        let cases = [
            (
                bits([0x0, 0x3f67_6491, 0x3edb_0748]),
                [
                    bits([0xbafb_6c0c, 0x3adc_3c00, 0x3dc1_7004]),
                    bits([0x313b_ea84, 0x3adf_b400, 0x3dc1_ec99]),
                    bits([0x2efc_b260, 0x3b8b_f100, 0x3dbf_72bc]),
                ],
            ),
            (
                bits([0x1, 0x3e8a_6a80, 0x3f76_77b1]),
                [
                    bits([0x3a0c_ef46, 0xbd9b_c440, 0x3dbc_105e]),
                    bits([0xae9d_32c0, 0xbd9b_b140, 0x3dbc_1001]),
                    bits([0xb086_5d3e, 0xbda0_1a10, 0x3dbd_872a]),
                ],
            ),
        ];
        for (at, (ray, triangle)) in cases.into_iter().enumerate() {
            // Liveness: the strict test really drops this ray.
            assert!(
                moller_within(centre, ray, triangle, 0.0).is_none(),
                "case {at}: the strict test meets this ray, so it is no longer the crack"
            );
            let (distance, u, v) = moller(centre, ray, triangle[0], triangle[1], triangle[2])
                .unwrap_or_else(|| {
                    panic!("case {at}: a ray along the midline fell between the body's triangles")
                });
            assert!(distance > 0.0 && u > -EDGE_SLACK && v > -EDGE_SLACK);
        }
    }
}
