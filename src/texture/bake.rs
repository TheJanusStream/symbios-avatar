//! Rasterising a body into texture space.
//!
//! Painting a body procedurally needs the inverse of rendering: for each texel
//! of the atlas, *where on the body is this?* Rasterising the unwrapped charts
//! answers that once, producing a geometry buffer of position, normal, and zone
//! per texel. Every painter after that is a pure function of one texel's sample,
//! which is what makes procedural skin tractable — freckles on a cheek become
//! plain arithmetic on a position rather than a search through geometry.
//!
//! Beside the buffer, for the crate's own painter, the bake measures how much
//! skin each texel spans, along each of the atlas's axes: the footprint a
//! painted beard's edge is feathered over (#432); and it marks which of the
//! head's texels an attached part drew, an ear or a lid, which takes no painted
//! hair (#471).
//!
//! Charts do not tile the atlas, so texels between them start empty. They are
//! **dilated** outward afterwards, because a bilinear tap or a mip level near a
//! chart edge otherwise pulls in background and draws a dark seam around every
//! body part.

use glam::{Mat3, Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::uv::UvUnwrap;

/// What the body looks like at one texel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texel {
    /// Position on the body, in body space.
    pub position: Vec3,
    /// Smooth surface normal there.
    pub normal: Vec3,
    /// How deeply the surface folds back on itself here, `0` on open skin and
    /// rising to `1` in a cavity. Carried from the geometry rather than derived
    /// by the painter, because the mesh is the only thing that knows: see
    /// [`PolyMesh::crease`].
    pub crease: f32,
    /// Whether this texel is inside the mouth: `0` on skin, near `0.5`
    /// on the teeth ridge, `1` in the cavity. Carried per vertex from the
    /// surgery's own classes, because nothing derivable from position can
    /// tell the inside of a closed fold from the lip a millimetre outside it.
    pub mouth: f32,
    /// Which part of the body it belongs to.
    pub zone: Zone,
}

/// How much skin one texel spans at a place on the body (#432): the ellipse a
/// disc one texel across in the atlas covers on the skin there.
///
/// **An ellipse, because the atlas is often coarse one way and fine the
/// other.** Under the jaw and behind the cheek the head's chart stepped four to
/// eight millimetres a texel one way and one or two the other at #432, and
/// behind the cheek and at the nape still steps three or four round the head
/// and under one up it since #469, so anything sized by the coarse step alone -
/// a round disc as wide as the longest axis - reaches across several fine
/// texels the other way.
///
/// **Not the step to a neighbouring texel**, which is what a painter could
/// otherwise measure for itself: that step is a property of the atlas, and at a
/// chart's edge or a seam two texels at one place on the body read two
/// different steps - so anything sized by it is sized differently on the two
/// sides of the seam. This is a property of the surface: each vertex reads the
/// atlas's stretch as a tensor, which no chart's orientation enters, averaged
/// over the faces round it on every side of a seam it sits on, so the field is
/// continuous over the body, as every other field here is.
///
/// **Beside the [`AtlasGeometry`] rather than in a [`Texel`]**, which is public
/// with every field public, so a field added there is a break for any crate
/// that writes one out. [`bake_measured`] hands them out as [`Footprints`],
/// and only the crate's own painter reads them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Footprint {
    /// The ellipse's two semi-axes on the skin, in metres, the longer first: at
    /// right angles to each other and to the texel's normal. Both zero where
    /// nothing measured it, which is everywhere off the head: see
    /// [`bake_measured`].
    pub(crate) axes: [Vec3; 2],
}

impl Footprint {
    /// The ellipse a stretch tensor - see [`spreads`] - draws in the tangent
    /// plane of `normal`.
    ///
    /// Square roots only, so every platform reads the same bits.
    fn of(spread: Mat3, normal: Vec3) -> Self {
        let (one, two) = normal.any_orthonormal_pair();
        let (a, b, c) = (
            one.dot(spread * one),
            one.dot(spread * two),
            two.dot(spread * two),
        );
        let ([big, small], turn) = principal(a, b, c);
        let major = one * turn.x + two * turn.y;
        let minor = one * -turn.y + two * turn.x;
        Self {
            axes: [major * big.sqrt(), minor * small.sqrt()],
        }
    }
}

/// Nothing measured: what [`Footprints`] reads off the head.
const NOTHING: Footprint = Footprint {
    axes: [Vec3::ZERO; 2],
};

/// The side of one tile of [`Footprints`], in texels.
const TILE: usize = 16;

/// The [`Footprint`]s of one bake, addressed by the index of a texel in
/// [`AtlasGeometry::texels`], **kept for the head only** (#432), and which of
/// the head's texels an attached part drew rather than the head's own skin
/// (#471).
///
/// Only the head's texels are ever measured, so one footprint for every texel
/// of the atlas was 24 MB at 1024 texels of which all but the head read
/// nothing - and Overlands builds every peer's avatar in a web worker, whose
/// linear memory never shrinks back. The head's charts lie all over the
/// atlas, so not even a grid over their bounds is small: the atlas is cut into
/// tiles of [`TILE`] texels a side, and a tile is only kept once a footprint is
/// written into it. A texel in a tile never written reads
/// [`Footprint::default`], as it did, and the head's own skin.
///
/// **Which texels are a part's** rides here because it is wanted where the
/// footprints are and nowhere else - on the head, by the painter - and because
/// a [`Texel`] cannot carry it: its fields are all public, so one more would
/// break any crate that writes one out. An ear is hung on the head, so its
/// texels are the head's [`Zone`]; it lies beside the scalp and the sideburn,
/// so a painter asking where hair grows by position alone paints both onto it.
/// One bit a texel, a tile's 256 in four words: 32 bytes a kept tile beside its
/// 6 KB of footprints.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Footprints {
    /// The atlas's width and height, in texels.
    atlas: [usize; 2],
    /// The atlas's tiles, row-major, each once something has been written into
    /// it.
    tiles: Vec<Option<Box<Tile>>>,
}

/// One tile of [`Footprints`], its texels row-major.
#[derive(Clone, Debug, PartialEq)]
struct Tile {
    /// Each texel's footprint.
    footprints: [Footprint; TILE * TILE],
    /// Whether an attached part drew each texel, a bit each (#471).
    parts: [u64; TILE * TILE / 64],
}

impl Footprints {
    /// Nothing measured, over an atlas `width` by `height` texels.
    fn over(width: u32, height: u32) -> Self {
        let atlas = [width as usize, height as usize];
        let [across, down] = atlas.map(|side| side.div_ceil(TILE));
        Self {
            atlas,
            tiles: vec![None; across * down],
        }
    }

    /// Which tile texel `index` of the atlas is in, and where in it.
    fn place(&self, index: usize) -> Option<(usize, usize)> {
        let [width, height] = self.atlas;
        let (x, y) = (index % width.max(1), index / width.max(1));
        (y < height).then(|| {
            let tile = (y / TILE) * width.div_ceil(TILE) + x / TILE;
            (tile, (y % TILE) * TILE + x % TILE)
        })
    }

    /// The footprint of texel `index` of the atlas, or `None` in a tile
    /// nothing was written into.
    pub(crate) fn get(&self, index: usize) -> Option<&Footprint> {
        let (tile, at) = self.place(index)?;
        self.tiles[tile].as_ref().map(|tile| &tile.footprints[at])
    }

    /// Whether an attached part - an ear, a lid - drew texel `index` of the
    /// atlas rather than the head's own skin (#471): `false` off the head.
    pub(crate) fn on_a_part(&self, index: usize) -> bool {
        self.place(index)
            .and_then(|(tile, at)| {
                let tile = self.tiles[tile].as_ref()?;
                Some((tile.parts[at / 64] >> (at % 64)) & 1 == 1)
            })
            .unwrap_or(false)
    }

    /// Writes texel `index` of the atlas: its footprint, and whether a part
    /// drew it. Its tile is kept from here on.
    fn write(&mut self, index: usize, footprint: Footprint, part: bool) {
        let Some((tile, at)) = self.place(index) else {
            return;
        };
        let tile = self.tiles[tile].get_or_insert_with(|| {
            Box::new(Tile {
                footprints: [NOTHING; TILE * TILE],
                parts: [0; TILE * TILE / 64],
            })
        });
        tile.footprints[at] = footprint;
        let bit = 1u64 << (at % 64);
        if part {
            tile.parts[at / 64] |= bit;
        } else {
            tile.parts[at / 64] &= !bit;
        }
    }
}

impl std::ops::Index<usize> for Footprints {
    type Output = Footprint;

    /// The footprint of texel `index` of the atlas, nothing off the head.
    fn index(&self, index: usize) -> &Footprint {
        self.get(index).unwrap_or(&NOTHING)
    }
}

/// The two eigenvalues of the symmetric 2 x 2 `[[a, b], [b, c]]`, the larger
/// first and neither below zero, and the unit direction of the larger.
///
/// Each direction read from the row that is well conditioned for it, so a
/// nearly round tensor still hands back a unit vector.
fn principal(a: f32, b: f32, c: f32) -> ([f32; 2], Vec2) {
    let half = 0.5 * (a - c);
    let root = (half * half + b * b).sqrt();
    let mean = 0.5 * (a + c);
    let (big, small) = ((mean + root).max(0.0), (mean - root).max(0.0));
    let turn = if a >= c {
        Vec2::new(big - c, b)
    } else {
        Vec2::new(b, big - a)
    };
    ([big, small], turn.normalize_or(Vec2::X))
}

/// The body, rasterised into texture space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AtlasGeometry {
    /// Atlas width in texels.
    pub width: u32,
    /// Atlas height in texels.
    pub height: u32,
    /// One entry per texel, row-major from the bottom-left. `None` where no
    /// chart covers the texel even after dilation.
    pub texels: Vec<Option<Texel>>,
}

impl AtlasGeometry {
    /// The sample at `(x, y)`, if the body covers it.
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> Option<Texel> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.texels[(y * self.width + x) as usize]
    }

    /// How many texels the body covers.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.texels.iter().filter(|texel| texel.is_some()).count()
    }

    /// Fraction of the atlas the body covers.
    #[must_use]
    pub fn coverage(&self) -> f32 {
        if self.texels.is_empty() {
            return 0.0;
        }
        self.covered() as f32 / self.texels.len() as f32
    }
}

/// How far, in texels, chart edges are grown outward.
///
/// Enough to survive a few mip levels; more than this and neighbouring charts
/// start growing into each other's gutters.
pub const DILATION: u32 = 8;

/// Rasterises `uv`'s charts into a geometry buffer of `size` texels square.
///
/// The body only. Use [`bake`] for an avatar that also carries attached parts,
/// which is every avatar with a face.
#[must_use]
pub fn bake_geometry(mesh: &PolyMesh, uv: &UvUnwrap, size: u32) -> AtlasGeometry {
    bake(mesh, uv, &[], &[], size)
}

/// Rasterises a body **and its attached parts** into one geometry buffer.
///
/// Each part is a mesh already carrying body-space positions and atlas
/// coordinates — that is, one whose reserved region from
/// [`crate::uv::unwrap_with`] has already been applied. Baking them into the
/// same buffer is what lets one painter cover the whole avatar: a nose is
/// painted by the same complexion arithmetic as the cheek beside it, because by
/// the time the painter runs there is no difference between them.
///
/// Each part brings its own mouth channel, as `mouth` is the body's: one
/// scalar a vertex, or empty for none. A beast's jaw and tongue are the parts
/// that carry one (#368), because they are the inside of its mouth and a
/// painter cannot tell them from the head they are sunk into by position.
///
/// Dilation happens once, at the end, over everything. Doing it per part would
/// let one part's grown edge overwrite a neighbour's genuine texels.
#[must_use]
pub fn bake(
    mesh: &PolyMesh,
    uv: &UvUnwrap,
    parts: &[(&PolyMesh, Zone, &[f32])],
    mouth: &[f32],
    size: u32,
) -> AtlasGeometry {
    bake_measured(mesh, uv, parts, mouth, size).0
}

/// [`bake`], and how much skin each texel spans beside it: one [`Footprint`] a
/// texel, addressed like [`AtlasGeometry::texels`], carried across each
/// triangle and out into the gutters with the texel it belongs to (#432), and
/// whether an attached part drew it (#471).
///
/// **The head's texels only**, the one part of the body anything is feathered
/// on (the painted beard); every other texel's footprint is left at nothing.
/// Read over the whole atlas it cost a fifth of the bake again, about twenty
/// milliseconds a body at 1024 texels, persons and beasts alike; and stored
/// over the whole atlas it cost 24 MB there, which [`Footprints`] keeps to the
/// head's corner.
pub(crate) fn bake_measured(
    mesh: &PolyMesh,
    uv: &UvUnwrap,
    parts: &[(&PolyMesh, Zone, &[f32])],
    mouth: &[f32],
    size: u32,
) -> (AtlasGeometry, Footprints) {
    if size == 0 || (uv.faces.is_empty() && parts.is_empty()) {
        return (AtlasGeometry::default(), Footprints::default());
    }

    let mut geometry = AtlasGeometry {
        width: size,
        height: size,
        texels: vec![None; (size * size) as usize],
    };
    let mut measured = Footprints::over(size, size);

    if !uv.faces.is_empty() {
        let normals = uv.gather(&mesh.vertex_normals());
        let positions = uv.gather(&mesh.positions);
        // Measured on the body before it was split at its seams, so a crease
        // that runs across a chart boundary is the same depth on both sides.
        let creases = uv.gather(&mesh.crease());
        let mouths = if mouth.len() == mesh.vertex_count() {
            uv.gather(mouth)
        } else {
            vec![0.0; uv.uvs.len()]
        };
        // Read per body vertex, over every chart face it touches on every side
        // of its seams, and only then split, so that both sides agree.
        let spreads = uv.gather(&spreads(
            &uv.uvs,
            &positions,
            &uv.faces,
            |vertex| uv.source[vertex as usize] as usize,
            mesh.vertex_count(),
            size,
        ));
        for (index, face) in uv.faces.iter().enumerate() {
            let zone = uv.charts[uv.chart_of_face[index] as usize].zone;
            // Fan-triangulate: a quad from a subdivided body is planar enough
            // that which diagonal is chosen makes no visible difference.
            for corner in 1..face.len().saturating_sub(1) {
                let triangle = [face[0], face[corner], face[corner + 1]];
                rasterize(
                    &mut geometry,
                    &mut measured,
                    zone,
                    false,
                    &Corners {
                        uvs: triangle.map(|v| uv.uvs[v as usize]),
                        positions: triangle.map(|v| positions[v as usize]),
                        normals: triangle.map(|v| normals[v as usize]),
                        creases: triangle.map(|v| creases[v as usize]),
                        mouths: triangle.map(|v| mouths[v as usize]),
                        spreads: triangle.map(|v| spreads[v as usize]),
                    },
                );
            }
        }
    }

    for &(part, zone, mouth) in parts {
        draw_part(&mut geometry, &mut measured, part, zone, mouth);
    }

    dilate(&mut geometry, &mut measured, DILATION);
    (geometry, measured)
}

/// Rasterises one attached part, using the coordinates it carries and its
/// mouth channel, if it has one.
fn draw_part(
    geometry: &mut AtlasGeometry,
    measured: &mut Footprints,
    part: &PolyMesh,
    zone: Zone,
    mouth: &[f32],
) {
    if part.uvs.len() != part.positions.len() {
        return;
    }
    let normals = part.shading_normals();
    let creases = part.crease();
    let mouths = |vertex: u32| {
        if mouth.len() == part.positions.len() {
            mouth[vertex as usize]
        } else {
            0.0
        }
    };
    let spreads = spreads(
        &part.uvs,
        &part.positions,
        &part.faces,
        |vertex| vertex as usize,
        part.positions.len(),
        geometry.width,
    );
    for face in &part.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let triangle = [face[0], face[corner], face[corner + 1]];
            rasterize(
                geometry,
                measured,
                zone,
                true,
                &Corners {
                    uvs: triangle.map(|v| part.uvs[v as usize]),
                    positions: triangle.map(|v| part.positions[v as usize]),
                    normals: triangle.map(|v| normals[v as usize]),
                    creases: triangle.map(|v| creases[v as usize]),
                    mouths: triangle.map(mouths),
                    spreads: triangle.map(|v| spreads[v as usize]),
                },
            );
        }
    }
}

/// How much skin one texel spans at each vertex (#432), as a stretch tensor:
/// over the faces round the vertex, the outer products of the skin each step
/// along the atlas's two axes spans, averaged by the skin each face covers.
///
/// `key` names the vertex a corner belongs to - the body's own vertex for an
/// unwrapped corner, so that the copies a seam makes of one vertex share one
/// reading - and `keys` is how many there are.
///
/// - **A tensor rather than the two steps themselves**, because the steps are
///   the atlas's and the tensor is the skin's: `u u' + v v'` is the same
///   whichever way a chart is turned or mirrored, so faces from the two charts
///   either side of a seam average to one reading, where averaging two charts'
///   steps would add a step along one chart's rows to a step along the other's
///   columns. [`Footprint::of`] reads the ellipse back out of it.
/// - **Averaged by skin, not by texels**, because where a chart is coarse its
///   faces are thin in the atlas: under the jaw most of the skin lies in faces
///   the projection has flattened to a fraction of a texel, and weighed by the
///   texels each takes, the few whole faces between them outvoted them and the
///   jaw's underside read two millimetres a texel where its neighbouring texels
///   stand seven apart.
/// - **Each face's stretch held to [`COARSEST`] along each of its axes**,
///   because a face flattened to a sliver reads metres a texel the long way
///   (its own texels belong to the faces round it), and one sliver beside a
///   nostril would otherwise read as coarse as the whole underside of the jaw.
///
/// Square roots only, so every platform reads the same bits.
fn spreads(
    uvs: &[Vec2],
    positions: &[Vec3],
    faces: &[Vec<u32>],
    key: impl Fn(u32) -> usize,
    keys: usize,
    size: u32,
) -> Vec<Mat3> {
    let scale = size as f32;
    let mut sums = vec![(Mat3::ZERO, 0.0f32); keys];
    for face in faces {
        for corner in 1..face.len().saturating_sub(1) {
            let triangle = [face[0], face[corner], face[corner + 1]];
            let [t0, t1, t2] = triangle.map(|v| uvs[v as usize] * scale);
            let [p0, p1, p2] = triangle.map(|v| positions[v as usize]);
            let (d1, d2) = (t1 - t0, t2 - t0);
            let det = d1.x * d2.y - d2.x * d1.y;
            if det.abs() < 1e-9 {
                continue;
            }
            let (e1, e2) = (p1 - p0, p2 - p0);
            // The surface's rate of change along the atlas's two axes.
            let along_u = (e1 * d2.y - e2 * d1.y) / det;
            let along_v = (e2 * d1.x - e1 * d2.x) / det;
            // That 3 x 2 map's singular directions on the skin, each as long
            // as its singular value, and each held to the coarsest.
            let ([big, small], turn) = principal(
                along_u.length_squared(),
                along_u.dot(along_v),
                along_v.length_squared(),
            );
            let held = |squared: f32| {
                if squared > COARSEST * COARSEST {
                    COARSEST * COARSEST / squared
                } else {
                    1.0
                }
            };
            let major = along_u * turn.x + along_v * turn.y;
            let minor = along_u * -turn.y + along_v * turn.x;
            let spread = outer(major) * held(big) + outer(minor) * held(small);
            let skin = 0.5 * e1.cross(e2).length();
            for vertex in triangle {
                if let Some(sum) = sums.get_mut(key(vertex)) {
                    sum.0 += spread * skin;
                    sum.1 += skin;
                }
            }
        }
    }
    sums.into_iter()
        .map(|(spread, skin)| {
            if skin > 0.0 {
                spread * (1.0 / skin)
            } else {
                Mat3::ZERO
            }
        })
        .collect()
}

/// `v v'`, the tensor of one direction.
fn outer(v: Vec3) -> Mat3 {
    Mat3::from_cols(v * v.x, v * v.y, v * v.z)
}

/// The coarsest a face reads along either axis in [`spreads`], in metres a
/// texel.
///
/// The head's chart was at its coarsest under the jaw and behind the cheek, where
/// a texel stood up to about eight millimetres from the next (#432), and since
/// #469 is nowhere coarser than about six, over the scalp; a face reading more
/// than eight is a sliver of the projection, not a texel of skin.
///
/// Provenance: **measured** on seeds 175 and 237 (#432).
const COARSEST: f32 = 0.008;

/// How far outside a triangle, in texels, a texel centre may sit and still be
/// filled by it.
///
/// **Conservative on purpose** (#309). A triangle owns every texel it
/// touches, not only those whose centres it contains: a chart with a
/// singularity — the arm chart's cylindrical projection at the armpit, where
/// a face lying across the arm's axis flattens to a sliver a texel tall with
/// a needle at the vertex on the axis — produced faces that covered no texel
/// centre at all, and the render sampled the empty gutter under them as
/// black. Dilation does not reach a needle's tip twenty texels out. Half the
/// texel diagonal is the smallest margin that guarantees a sliver of any
/// thickness still lands on the row it crosses; shared edges were already
/// double-written by the hairline tolerance this replaces, and the overdraw
/// into a neighbour's texels is the same width the gutter dilation grows
/// anyway.
const COVER: f32 = 0.71;

/// One triangle's corners, everything the rasteriser carries across it.
struct Corners {
    uvs: [Vec2; 3],
    positions: [Vec3; 3],
    normals: [Vec3; 3],
    creases: [f32; 3],
    mouths: [f32; 3],
    /// The stretch tensors of [`spreads`].
    spreads: [Mat3; 3],
}

/// Fills every texel a triangle covers or touches - see [`COVER`] - and the
/// [`Footprint`] beside each, and whether an attached `part` drew it.
fn rasterize(
    geometry: &mut AtlasGeometry,
    measured: &mut Footprints,
    zone: Zone,
    part: bool,
    corners: &Corners,
) {
    let Corners {
        uvs,
        positions,
        normals,
        creases,
        mouths,
        spreads,
    } = corners;
    let scale = Vec2::new(geometry.width as f32, geometry.height as f32);
    let pixels = uvs.map(|uv| uv * scale);

    let area = edge(pixels[0], pixels[1], pixels[2]);
    if area.abs() < 1e-9 {
        return;
    }

    let lo = pixels[0].min(pixels[1]).min(pixels[2]) - Vec2::splat(COVER);
    let hi = pixels[0].max(pixels[1]).max(pixels[2]) + Vec2::splat(COVER);
    let x0 = lo.x.floor().max(0.0) as u32;
    let y0 = lo.y.floor().max(0.0) as u32;
    let x1 = (hi.x.ceil() as i64).clamp(0, i64::from(geometry.width)) as u32;
    let y1 = (hi.y.ceil() as i64).clamp(0, i64::from(geometry.height)) as u32;
    // Each edge function over its edge's length is a signed distance in
    // texels, which is what the cover margin is measured in.
    let lengths = [
        pixels[1].distance(pixels[2]).max(f32::EPSILON),
        pixels[2].distance(pixels[0]).max(f32::EPSILON),
        pixels[0].distance(pixels[1]).max(f32::EPSILON),
    ];
    let inward = area.signum();

    for y in y0..y1 {
        for x in x0..x1 {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let raw = Vec3::new(
                edge(pixels[1], pixels[2], point),
                edge(pixels[2], pixels[0], point),
                edge(pixels[0], pixels[1], point),
            );
            let distances = Vec3::new(
                raw.x * inward / lengths[0],
                raw.y * inward / lengths[1],
                raw.z * inward / lengths[2],
            );
            if distances.min_element() < -COVER {
                continue;
            }
            let mut weights = (raw / area).max(Vec3::ZERO);
            let total = weights.element_sum();
            if total <= 0.0 {
                continue;
            }
            weights /= total;

            let at = (y * geometry.width + x) as usize;
            let normal = (normals[0] * weights.x + normals[1] * weights.y + normals[2] * weights.z)
                .normalize_or(Vec3::Y);
            geometry.texels[at] = Some(Texel {
                position: positions[0] * weights.x
                    + positions[1] * weights.y
                    + positions[2] * weights.z,
                normal,
                crease: creases[0] * weights.x + creases[1] * weights.y + creases[2] * weights.z,
                mouth: mouths[0] * weights.x + mouths[1] * weights.y + mouths[2] * weights.z,
                zone,
            });
            if zone == Zone::Head {
                measured.write(
                    at,
                    Footprint::of(
                        spreads[0] * weights.x + spreads[1] * weights.y + spreads[2] * weights.z,
                        normal,
                    ),
                    part,
                );
            }
        }
    }
}

/// Twice the signed area of the triangle `a, b, c`.
fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// Grows filled texels outward into the gutters, each with its [`Footprint`]
/// and whether a part drew it.
fn dilate(geometry: &mut AtlasGeometry, measured: &mut Footprints, passes: u32) {
    let (width, height) = (geometry.width as i64, geometry.height as i64);

    for _ in 0..passes {
        let previous = geometry.texels.clone();
        let mut grew = false;

        for y in 0..height {
            for x in 0..width {
                let here = (y * width + x) as usize;
                if previous[here].is_some() {
                    continue;
                }
                // Take the first filled neighbour rather than blending: this is
                // padding for filtering, and an averaged value between two body
                // parts belongs to neither.
                let found = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)]
                    .into_iter()
                    .find_map(|(dx, dy)| {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= width || ny >= height {
                            return None;
                        }
                        let there = (ny * width + nx) as usize;
                        previous[there].map(|texel| (texel, there))
                    });
                if let Some((texel, there)) = found {
                    geometry.texels[here] = Some(texel);
                    // Filled before this pass, so already what it ends as.
                    if let Some(&footprint) = measured.get(there) {
                        let part = measured.on_a_part(there);
                        measured.write(here, footprint, part);
                    }
                    grew = true;
                }
            }
        }

        if !grew {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::subdiv::catmull_clark;
    use crate::uv::{UvConfig, unwrap};

    fn baked(size: u32) -> (PolyMesh, UvUnwrap, AtlasGeometry) {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let geometry = bake_geometry(&mesh, &uv, size);
        (mesh, uv, geometry)
    }

    #[test]
    fn the_body_covers_most_of_the_atlas() {
        let (_, _, geometry) = baked(256);
        assert_eq!(geometry.width, 256);
        // Coverage tracks how tightly the charts packed — around 60-80% — since
        // the body fills its charts and the charts fill the atlas.
        assert!(
            geometry.coverage() > 0.5,
            "only {:.0}% of the atlas has a body under it",
            geometry.coverage() * 100.0
        );
    }

    #[test]
    fn every_sample_lands_on_the_body() {
        let (mesh, _, geometry) = baked(128);
        let (lo, hi) = mesh.bounds();
        let slack = (hi - lo) * 0.05;

        for texel in geometry.texels.iter().flatten() {
            assert!(
                texel.position.cmpge(lo - slack).all() && texel.position.cmple(hi + slack).all(),
                "sample at {:?} is off the body",
                texel.position
            );
            assert!(
                (texel.normal.length() - 1.0).abs() < 1e-3,
                "normal is not unit length"
            );
        }
    }

    #[test]
    fn zones_survive_into_texture_space() {
        let (_, uv, geometry) = baked(256);
        let charted: Vec<Zone> = uv.charts.iter().map(|chart| chart.zone).collect();

        for zone in &charted {
            assert!(
                geometry
                    .texels
                    .iter()
                    .flatten()
                    .any(|texel| texel.zone == *zone),
                "{zone:?} has a chart but no texels"
            );
        }
    }

    #[test]
    fn dilation_fills_the_gutters() {
        let (mesh, uv, _) = baked(128);
        let mut bare = AtlasGeometry {
            width: 128,
            height: 128,
            texels: vec![None; 128 * 128],
        };
        let mut measured = Footprints::over(128, 128);
        let normals = uv.gather(&mesh.vertex_normals());
        let positions = uv.gather(&mesh.positions);
        for (index, face) in uv.faces.iter().enumerate() {
            let zone = uv.charts[uv.chart_of_face[index] as usize].zone;
            for corner in 1..face.len().saturating_sub(1) {
                let triangle = [face[0], face[corner], face[corner + 1]];
                rasterize(
                    &mut bare,
                    &mut measured,
                    zone,
                    false,
                    &Corners {
                        uvs: triangle.map(|v| uv.uvs[v as usize]),
                        positions: triangle.map(|v| positions[v as usize]),
                        normals: triangle.map(|v| normals[v as usize]),
                        creases: [0.0; 3],
                        mouths: [0.0; 3],
                        spreads: [Mat3::ZERO; 3],
                    },
                );
            }
        }
        let before = bare.coverage();
        dilate(&mut bare, &mut measured, DILATION);
        assert!(
            bare.coverage() > before,
            "dilation must grow coverage past {before:.3}"
        );
    }

    #[test]
    fn a_footprint_is_one_texel_of_skin_each_way_and_one_reading_at_a_seam() {
        // **What the beard's feather is sized by** (#432), so it must say how
        // far apart the texels round it stand on the skin, along each of the
        // atlas's axes, and it must say the same on both sides of a seam: a
        // painter sized by its own step to the next texel read two different
        // steps at one place on the jaw, and two texels a fifth of a millimetre
        // apart painted a third of a beard's density apart.
        //
        // Read as the step to the next texel along each axis, measured in the
        // footprint's own ellipse: one where the ellipse is a texel of skin each
        // way. Measured at #432 on the default head: 0.978 at the median, 0.959
        // where the atlas is twice as coarse one way as the other; a round
        // footprint as wide as the ellipse's long axis reads 0.900 and 0.764
        // there, the fine way's steps a fraction of one. On #469's chart 0.977
        // and 0.917 over 9,100 readings; 0.976 and 0.928 to 0.933 under a
        // nudge of libm.
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        let size = 1024;
        let (geometry, measured) = bake_measured(&mesh, &uv, &[], &[], size);
        // Kept for the head only: measured at #432 on the default head, 410 of
        // the atlas's 4096 tiles, 2.5 MB of footprints where one a texel was 24;
        // 396 on #469's chart, with which texels are a part's (#471) at 32
        // bytes a tile beside them.
        let kept = measured.tiles.iter().flatten().count();
        assert!(
            kept * 8 <= measured.tiles.len(),
            "the footprints keep {kept} of {} tiles",
            measured.tiles.len()
        );
        assert_eq!(
            geometry,
            bake_geometry(&mesh, &uv, size),
            "measuring must not change what is baked"
        );
        let width = geometry.width as usize;
        // The step `s` in the ellipse's own measure, `|M^-1/2 s|`.
        let reading = |footprint: &Footprint, step: Vec3| {
            let [major, minor] = footprint.axes;
            Vec2::new(
                step.dot(major) / major.length_squared(),
                step.dot(minor) / minor.length_squared(),
            )
            .length()
        };
        let (mut steps, mut long) = (Vec::new(), Vec::new());
        for (at, texel) in geometry.texels.iter().enumerate() {
            let Some(here) = texel else { continue };
            let [major, minor] = measured[at].axes;
            if here.zone != Zone::Head {
                assert_eq!(
                    measured[at],
                    Footprint::default(),
                    "a footprint off the head at {:?}",
                    here.position
                );
                continue;
            }
            assert!(
                major.is_finite()
                    && minor.is_finite()
                    && major.length() >= minor.length() * (1.0 - 1e-5)
                    && major.dot(minor).abs() <= 1e-3 * major.length_squared()
                    && major.dot(here.normal).abs() <= 1e-3 * major.length(),
                "a footprint of {major:?} and {minor:?} at {:?}",
                here.position
            );
            // Where a face is held to the coarsest it reads, the step past it
            // is longer than the footprint by design.
            if minor.length() < 1e-5 || major.length() > 0.9 * COARSEST {
                continue;
            }
            let along = [(at % width + 1 < width).then_some(at + 1), Some(at + width)];
            let both = along.map(|next| {
                let there = geometry.texels.get(next?).copied().flatten()?;
                let step = there.position - here.position;
                (here.zone == there.zone
                    && here.normal.dot(there.normal) >= 0.9
                    && step.length() >= 1e-6)
                    .then(|| (step.length(), reading(&measured[at], step)))
                    .filter(|&(_, read)| read < 3.0)
            });
            for &(_, read) in both.iter().flatten() {
                steps.push(read);
            }
            // Where the atlas itself is twice as coarse one way as the other.
            if let [Some((one, first)), Some((two, second))] = both
                && one.max(two) >= 2.0 * one.min(two)
            {
                long.extend([first, second]);
            }
        }
        let median = |all: &mut Vec<f32>| {
            all.sort_by(f32::total_cmp);
            all[all.len() / 2]
        };
        assert!(long.len() > 1000, "only {} stretched readings", long.len());
        let (everywhere, stretched) = (median(&mut steps), median(&mut long));
        println!(
            "the step to the next texel reads {everywhere:.3} of the footprint at the median, \
             {stretched:.3} over {} readings where it is twice as coarse one way as the other",
            long.len()
        );
        assert!(
            (0.85..=1.18).contains(&everywhere) && (0.85..=1.18).contains(&stretched),
            "the step to the next texel is {everywhere:.2} of a footprint at the median, \
             {stretched:.2} where the atlas is coarse one way"
        );

        // The two sides of a seam on the head: texels at one place on the
        // body, from two places in the atlas farther apart than the gutters
        // grow. (Off the head nothing is measured, and nothing is feathered.)
        let cell = 5e-5f32;
        let mut cells = std::collections::HashMap::<[i64; 3], Vec<usize>>::new();
        for (at, texel) in geometry.texels.iter().enumerate() {
            if let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) {
                let key = (texel.position / cell).floor().as_i64vec3().to_array();
                cells.entry(key).or_default().push(at);
            }
        }
        let tensor = |footprint: &Footprint| {
            let [major, minor] = footprint.axes;
            outer(major) + outer(minor)
        };
        // Two copies the gutters grew from one texel stand up to twice their
        // reach apart.
        let reach = 2 * DILATION as usize + 2;
        let (mut twins, mut worst) = (0usize, 0.0f32);
        for found in cells.values() {
            for (one, &mine) in found.iter().enumerate() {
                for &theirs in &found[one + 1..] {
                    let (dx, dy) = (
                        (mine % width).abs_diff(theirs % width),
                        (mine / width).abs_diff(theirs / width),
                    );
                    if dx.max(dy) <= reach {
                        continue;
                    }
                    twins += 1;
                    let gap = tensor(&measured[mine]) - tensor(&measured[theirs]);
                    let frobenius = (gap.x_axis.length_squared()
                        + gap.y_axis.length_squared()
                        + gap.z_axis.length_squared())
                    .sqrt();
                    let scale = measured[mine].axes[0].length_squared().max(1e-12);
                    worst = worst.max(frobenius / scale);
                }
            }
        }
        println!(
            "{twins} texels on two sides of a seam differ by at most {worst:.4} of a footprint"
        );
        assert!(
            twins > 100,
            "only {twins} texels share a place across a seam"
        );
        assert!(
            worst < 0.02,
            "two texels at one place on two sides of a seam read footprints {worst:.3} apart"
        );
    }

    #[test]
    fn baking_is_deterministic() {
        let (_, _, first) = baked(64);
        let (_, _, second) = baked(64);
        assert_eq!(first, second);
    }

    #[test]
    fn a_zero_sized_atlas_bakes_to_nothing() {
        let (mesh, uv, _) = baked(32);
        assert_eq!(bake_geometry(&mesh, &uv, 0), AtlasGeometry::default());
    }
}
