//! Unwrapping a body into a texture atlas.
//!
//! Because bodies are generated, charts can be *named*: every chart declares the
//! [`Zone`] it covers, so a procedural painter addresses "the chest" rather than
//! "island 7". That is the reason to unwrap here rather than with a
//! general-purpose atlas tool, which would hand back anonymous islands in an
//! arbitrary order.
//!
//! The zones charted are the same for every body of a given plan; the *number*
//! of charts is not. A zone can be genuinely disconnected — each clavicle is
//! separated from the torso by the shoulder's own zone — and how a body's
//! proportions fall decides how many pieces result. Painters must therefore key
//! on [`Chart::zone`] and expect more than one chart per zone.
//!
//! Charts follow the body's own [`Zone`]s, unwrapped cylindrically about the
//! bone entering each zone. Two conventions fall out of that and are worth
//! stating, because both are what character artists do by hand:
//!
//! * **The seam runs up the back.** The angle origin is the body's forward
//!   direction, so the front of a chart lands at its middle and the wrap lands
//!   behind. A face is therefore one continuous island — never split down the
//!   nose — which is what lets a face be painted as plain 2-D maths.
//! * **The face and hands get more texels.** Chart area is proportional to
//!   surface area times an importance weight, so detail lands where it is looked
//!   at rather than spread evenly over a body that is mostly forearm.
//!
//! Unwrapping duplicates vertices — at chart boundaries, and along each chart's
//! seam — so the result carries its own vertex list with a [`UvUnwrap::source`]
//! index back into the mesh it came from.

mod pack;

use glam::{Vec2, Vec3};
use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::{Rig, landmark};

pub use pack::Rect;

/// Tuning for [`unwrap`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvConfig {
    /// Empty margin kept around every chart, as a fraction of the atlas.
    ///
    /// Without it, a mip level or a bilinear tap pulls in the neighbouring
    /// chart and one body part bleeds onto another.
    pub gutter: f32,
    /// How much more texel area the head gets than an equal area of body.
    pub head_density: f32,
    /// How much more texel area hands and feet get.
    pub extremity_density: f32,
}

impl Default for UvConfig {
    fn default() -> Self {
        Self {
            gutter: 0.004,
            head_density: 4.0,
            extremity_density: 2.0,
        }
    }
}

/// One unwrapped region of the body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chart {
    /// The body zone this chart covers.
    pub zone: Zone,
    /// Where the chart sits in the atlas.
    pub rect: Rect,
    /// Surface area the chart covers, in square metres.
    pub area: f32,
}

impl Chart {
    /// Atlas area per unit of surface area — the chart's texel density.
    ///
    /// Comparing this between charts is how the density weighting is checked.
    #[must_use]
    pub fn texel_density(&self) -> f32 {
        if self.area > 0.0 {
            self.rect.area() / self.area
        } else {
            0.0
        }
    }
}

/// A mesh unwrapped into an atlas.
///
/// The vertex list is the unwrapped one, which is longer than the mesh's:
/// [`UvUnwrap::source`] maps each entry back to the mesh vertex it duplicates,
/// so positions, normals, and skin weights are looked up through it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UvUnwrap {
    /// Mesh vertex each unwrapped vertex came from.
    pub source: Vec<u32>,
    /// Atlas coordinate per unwrapped vertex, within `0..=1`.
    pub uvs: Vec<Vec2>,
    /// Faces, re-indexed against the unwrapped vertex list.
    pub faces: Vec<Vec<u32>>,
    /// Which chart each face belongs to.
    pub chart_of_face: Vec<u16>,
    /// Mesh face each unwrapped face came from.
    ///
    /// Faces are emitted chart by chart, so the order differs from the mesh's.
    pub source_face: Vec<u32>,
    /// The charts, in atlas order.
    pub charts: Vec<Chart>,
}

impl UvUnwrap {
    /// How many unwrapped vertices there are.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.source.len()
    }

    /// Gathers a per-mesh-vertex attribute onto the unwrapped vertex list.
    ///
    /// This is how positions, normals, and skin weights follow the split.
    #[must_use]
    pub fn gather<T: Clone>(&self, attribute: &[T]) -> Vec<T> {
        self.source
            .iter()
            .map(|&index| attribute[index as usize].clone())
            .collect()
    }

    /// One atlas coordinate per *mesh* vertex, rather than per unwrapped one.
    ///
    /// A vertex on a chart boundary has several coordinates and this keeps the
    /// first, so the result is only ever an approximation — for anything painted
    /// with detail it would show a seam. It is what a garment needs, though: a
    /// garment is cut from body vertices and shaded rather than painted, so one
    /// lookup per vertex gives it the complexion of the skin beneath it without
    /// unwrapping it separately.
    ///
    /// Vertices no chart uses come back at the middle of the atlas.
    #[must_use]
    pub fn by_source(&self, vertices: usize) -> Vec<Vec2> {
        let mut out = vec![Vec2::splat(0.5); vertices];
        let mut taken = vec![false; vertices];
        for (index, &source) in self.source.iter().enumerate() {
            let at = source as usize;
            if at < vertices && !taken[at] {
                taken[at] = true;
                out[at] = self.uvs[index];
            }
        }
        out
    }

    /// Serialises the unwrapped mesh to Wavefront OBJ, texture coordinates and all.
    ///
    /// Open the result in any DCC tool to see the atlas laid out — which is the
    /// only way to judge whether a chart reads as the body part it covers.
    #[must_use]
    pub fn to_obj(&self, mesh: &PolyMesh) -> String {
        use std::fmt::Write as _;

        let mut out = String::from("# symbios-avatar unwrapped body\n");
        for &source in &self.source {
            let p = mesh.positions[source as usize];
            let _ = writeln!(out, "v {} {} {}", p.x, p.y, p.z);
        }
        for uv in &self.uvs {
            let _ = writeln!(out, "vt {} {}", uv.x, uv.y);
        }
        for face in &self.faces {
            out.push('f');
            for &index in face {
                let _ = write!(out, " {0}/{0}", index + 1);
            }
            out.push('\n');
        }
        out
    }
}

/// Unwraps `mesh` into a zone-keyed atlas.
///
/// `zones` is the per-vertex zone map from
/// [`crate::rig::SkinWeights::zone_map`], and `rig` supplies each zone's axis.
#[must_use]
pub fn unwrap(mesh: &PolyMesh, rig: &Rig, zones: &[Zone], config: &UvConfig) -> UvUnwrap {
    unwrap_with(mesh, rig, zones, config, &[]).0
}

/// Unwraps `mesh`, reserving atlas space for parts that are not part of it.
///
/// A body is not the whole of an avatar. Hands, ears, lips and every other
/// attached part carry texture coordinates of their own, and until they have
/// somewhere in the atlas to put them they can only be flat-shaded — which is
/// most of a character's look missing, since the parts a face is judged from are
/// all attached ones.
///
/// `extra` requests one region per part, in the same units the body's own charts
/// are sized in: a width and a height in metres of surface, which the packer
/// scales together until everything fits. Requests come back as rectangles in
/// the same order, ready for [`crate::PolyMesh::uvs_within`].
///
/// Packing them **together** with the body rather than afterwards is the point.
/// A second pass over the leftovers would give the parts whatever the body
/// happened not to want, and the parts are the half that needs the texels.
#[must_use]
pub fn unwrap_with(
    mesh: &PolyMesh,
    rig: &Rig,
    zones: &[Zone],
    config: &UvConfig,
    extra: &[Vec2],
) -> (UvUnwrap, Vec<Rect>) {
    if mesh.faces.is_empty() || zones.len() != mesh.positions.len() {
        return (UvUnwrap::default(), Vec::new());
    }

    let face_zones: Vec<Zone> = mesh
        .faces
        .iter()
        .map(|face| dominant_zone(face, zones))
        .collect();
    let groups = connected_groups(mesh, rig, &face_zones);

    // Project every chart into its own local square first, then pack.
    let mut projections = Vec::with_capacity(groups.len());
    for group in &groups {
        projections.push(project(mesh, rig, group));
    }

    let mut sizes: Vec<Vec2> = projections
        .iter()
        .map(|projection| projection.atlas_size(config))
        .collect();
    let charts = sizes.len();
    sizes.extend_from_slice(extra);
    let packed = pack::shelf_pack(&sizes, config.gutter);
    let (rects, reserved) = packed.split_at(charts);
    let reserved = reserved.to_vec();

    let mut out = UvUnwrap {
        chart_of_face: vec![0; mesh.faces.len()],
        ..Default::default()
    };
    // Key on (mesh vertex, chart, seam side) so one lookup handles both kinds of
    // duplication: a vertex shared between charts, and one straddling a seam.
    let mut emitted: HashMap<(u32, u16, u8), u32> = HashMap::new();

    for (chart_index, (group, projection)) in groups.iter().zip(&projections).enumerate() {
        let rect = rects[chart_index];
        let chart = chart_index as u16;

        for (slot, &face_index) in group.faces.iter().enumerate() {
            let face = &mesh.faces[face_index as usize];
            let shifts = &projection.shifts[slot];
            let mut mapped = Vec::with_capacity(face.len());

            for (corner, &vertex) in face.iter().enumerate() {
                let shift = shifts[corner];
                let key = (vertex, chart, shift);
                let index = *emitted.entry(key).or_insert_with(|| {
                    let unit = projection.unit_uv(vertex, shift);
                    out.source.push(vertex);
                    out.uvs.push(rect.lerp(unit));
                    (out.source.len() - 1) as u32
                });
                mapped.push(index);
            }

            out.chart_of_face[out.faces.len()] = chart;
            out.source_face.push(face_index);
            out.faces.push(mapped);
        }

        out.charts.push(Chart {
            zone: group.zone,
            rect,
            area: projection.area,
        });
    }

    (out, reserved)
}

/// The zone most of a face's corners agree on.
fn dominant_zone(face: &[u32], zones: &[Zone]) -> Zone {
    let mut tally: BTreeMap<Zone, usize> = BTreeMap::new();
    for &vertex in face {
        *tally.entry(zones[vertex as usize]).or_default() += 1;
    }
    tally
        .into_iter()
        // Ties break on the zone's own ordering, so the result is deterministic.
        .max_by_key(|&(zone, count)| (count, std::cmp::Reverse(zone)))
        .map(|(zone, _)| zone)
        .unwrap_or_default()
}

/// How a chart is flattened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Wrapped about the zone's axis — the body of a limb or torso.
    Cylindrical,
    /// Projected straight down the axis — the cap on the end of a limb.
    ///
    /// A cap is perpendicular to the axis, so its corners sit at every angle
    /// around it at once. Cylindrical projection smears such a face across the
    /// entire chart, and no amount of seam handling helps: the face has no seam,
    /// it has a pole. Flattening it along the axis instead is exact.
    Planar,
}

/// How far round its zone's axis a face's corners must reach before the face
/// is a cap, in radians.
///
/// The cap criterion is the cylindrical failure mode measured directly: a face
/// at the pole has corners at every angle round the axis at once, so
/// cylindrical projection smears it across the whole chart. A pole face spans
/// π or more; a face standing off the axis — however axis-aligned its normal —
/// spans only its own width over its radius, well under a radian for anything
/// off the pole. The threshold sits between the two regimes, and its exact
/// value moves almost nothing: on the default body the spans cluster under
/// 0.5 rad off the pole and above 2 off it.
///
/// Provenance: **derived** from the two regimes above; see #158 for the
/// confetti charts the orientation-only test produced.
const CAP_SPAN: f32 = 1.0;

/// A connected run of faces sharing one zone and one projection.
struct Group {
    zone: Zone,
    kind: Kind,
    faces: Vec<u32>,
}

/// Splits faces into connected same-zone groups.
///
/// Connectivity matters: a zone that appears in two disconnected places would
/// otherwise share one chart, and the projection would fold one part onto the
/// other.
fn connected_groups(mesh: &PolyMesh, rig: &Rig, face_zones: &[Zone]) -> Vec<Group> {
    let mut axis_of: HashMap<Zone, Vec3> = HashMap::new();
    // Where each zone's material stands, for asking whether a face is AT the
    // axis or merely oriented along it. The mean of the zone's face centroids
    // is off the true axis line by a little on an asymmetric zone, which is
    // fine: the question below is a coarse classification, not a projection.
    let mut centre_of: HashMap<Zone, (Vec3, f32)> = HashMap::new();
    for (index, _) in mesh.faces.iter().enumerate() {
        let entry = centre_of
            .entry(face_zones[index])
            .or_insert((Vec3::ZERO, 0.0));
        entry.0 += mesh.face_centroid(index);
        entry.1 += 1.0;
    }
    let face_kinds: Vec<Kind> = (0..mesh.faces.len())
        .map(|face| {
            let axis = *axis_of
                .entry(face_zones[face])
                .or_insert_with(|| zone_axis(rig, face_zones[face]));
            if face_normal(mesh, face).dot(axis).abs() <= 0.85 {
                return Kind::Cylindrical;
            }
            // Oriented along the axis — but a cap is at the POLE: its corners
            // stand at every angle round the axis at once, which is the one
            // thing cylindrical projection cannot flatten. A face that merely
            // FACES along the axis while standing off it — the underside of a
            // jaw, the submental plane, a brow's shelf — wraps fine, and
            // classifying those as caps is what shredded the lower face into
            // confetti charts: each downward-facing island became its own
            // few-texel chart, whose rasterised texels then drowned in the
            // neighbouring charts' dilation (#158). So the cap test is the
            // failure mode itself, measured: the circular span of the face's
            // corners about the axis.
            let (centre, count) = centre_of[&face_zones[face]];
            let origin = centre / count.max(1.0);
            let (tangent, binormal) = frame(axis);
            let corners = &mesh.faces[face];
            let mut turns: Vec<f32> = corners
                .iter()
                .map(|&v| {
                    let offset = mesh.positions[v as usize] - origin;
                    let flat = offset - axis * offset.dot(axis);
                    flat.dot(binormal).atan2(flat.dot(tangent))
                })
                .collect();
            turns.sort_unstable_by(f32::total_cmp);
            let widest_gap = turns.windows(2).map(|pair| pair[1] - pair[0]).fold(
                std::f32::consts::TAU - (turns[turns.len() - 1] - turns[0]),
                f32::max,
            );
            let span = std::f32::consts::TAU - widest_gap;
            if span > CAP_SPAN {
                Kind::Planar
            } else {
                Kind::Cylindrical
            }
        })
        .collect();

    let mut by_edge: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for corner in 0..face.len() {
            let a = face[corner];
            let b = face[(corner + 1) % face.len()];
            by_edge
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(index as u32);
        }
    }

    let mut seen = vec![false; mesh.faces.len()];
    let mut groups = Vec::new();

    // Seed in face order so the chart list is deterministic.
    for start in 0..mesh.faces.len() {
        if seen[start] {
            continue;
        }
        let zone = face_zones[start];
        let kind = face_kinds[start];
        let mut faces = Vec::new();
        let mut queue = VecDeque::from([start as u32]);
        seen[start] = true;

        while let Some(face_index) = queue.pop_front() {
            faces.push(face_index);
            let face = &mesh.faces[face_index as usize];
            for corner in 0..face.len() {
                let a = face[corner];
                let b = face[(corner + 1) % face.len()];
                for &neighbor in by_edge
                    .get(&(a.min(b), a.max(b)))
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    let next = neighbor as usize;
                    if !seen[next] && face_zones[next] == zone && face_kinds[next] == kind {
                        seen[next] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        faces.sort_unstable();
        groups.push(Group { zone, kind, faces });
    }

    groups
}

/// One chart's flattening, before packing.
struct Projection {
    /// How this chart was flattened.
    kind: Kind,
    /// Raw `u` per mesh vertex used by this chart, before seam shifts.
    angle: HashMap<u32, f32>,
    /// Raw `v` per mesh vertex used by this chart.
    along: HashMap<u32, f32>,
    /// Seam shift per face corner, parallel to the group's face list.
    shifts: Vec<Vec<u8>>,
    /// Bounds of the shifted `u` and raw `v`.
    u_range: (f32, f32),
    v_range: (f32, f32),
    /// Mean distance from the chart's axis, used to size the chart honestly.
    mean_radius: f32,
    /// Surface area the chart covers.
    area: f32,
    /// Importance of the zone this chart covers.
    zone: Zone,
    /// Monotone remaps of the chart's normalized `u` and `v`, following the
    /// mesh's own vertex density. See [`density_warp`].
    u_warp: Vec<f32>,
    v_warp: Vec<f32>,
}

impl Projection {
    /// Where a vertex lands in the chart's own unit square.
    fn unit_uv(&self, vertex: u32, shift: u8) -> Vec2 {
        let u = self.angle[&vertex] + f32::from(shift);
        let v = self.along[&vertex];
        Vec2::new(
            warp(normalize(u, self.u_range), &self.u_warp),
            warp(normalize(v, self.v_range), &self.v_warp),
        )
    }

    /// The chart's requested size, in units that keep texel density honest.
    ///
    /// Width is the arc length the chart wraps through and height its extent
    /// along the axis, so a chart's aspect matches the surface it covers. The
    /// density weight multiplies texel *area*, hence the square root.
    fn atlas_size(&self, config: &UvConfig) -> Vec2 {
        let span = (self.u_range.1 - self.u_range.0).max(1e-4);
        let height = (self.v_range.1 - self.v_range.0).max(1e-4);
        // A cylindrical chart's width is the arc it wraps through; a planar
        // chart's is already a distance.
        let width = match self.kind {
            Kind::Cylindrical => span * std::f32::consts::TAU * self.mean_radius.max(1e-4),
            Kind::Planar => span,
        };
        Vec2::new(width, height) * density(self.zone, config).sqrt()
    }
}

/// How much texel area a zone earns per unit of surface area.
fn density(zone: Zone, config: &UvConfig) -> f32 {
    match zone {
        Zone::Head => config.head_density,
        Zone::Extremity(_) => config.extremity_density,
        _ => 1.0,
    }
}

/// Projects one group of faces cylindrically about its zone's axis.
fn project(mesh: &PolyMesh, rig: &Rig, group: &Group) -> Projection {
    let vertices: Vec<u32> = {
        let mut all: Vec<u32> = group
            .faces
            .iter()
            .flat_map(|&face| mesh.faces[face as usize].iter().copied())
            .collect();
        all.sort_unstable();
        all.dedup();
        all
    };

    let origin = vertices
        .iter()
        .map(|&v| mesh.positions[v as usize])
        .sum::<Vec3>()
        / vertices.len().max(1) as f32;
    let axis = zone_axis(rig, group.zone);
    let (tangent, binormal) = frame(axis);

    let mut angle = HashMap::with_capacity(vertices.len());
    let mut along = HashMap::with_capacity(vertices.len());
    let mut radius_sum = 0.0f32;

    for &vertex in &vertices {
        let offset = mesh.positions[vertex as usize] - origin;
        let height = offset.dot(axis);
        let flat = offset - axis * height;
        radius_sum += flat.length();
        match group.kind {
            Kind::Cylindrical => {
                // Angle zero points forward, so the wrap — and the seam — lands
                // behind the body.
                let turn = flat.dot(binormal).atan2(flat.dot(tangent));
                angle.insert(vertex, turn / std::f32::consts::TAU + 0.5);
                along.insert(vertex, height);
            }
            Kind::Planar => {
                // Flattened down the axis, so a cap keeps its shape.
                angle.insert(vertex, flat.dot(tangent));
                along.insert(vertex, flat.dot(binormal));
            }
        }
    }

    // A face straddling the seam has corners at both ends of the wrap. Lifting
    // the low ones by a full turn makes the face continuous again; the vertices
    // they refer to get duplicated when the chart is emitted.
    let mut shifts = Vec::with_capacity(group.faces.len());
    let mut u_range = (f32::INFINITY, f32::NEG_INFINITY);
    for &face_index in &group.faces {
        let face = &mesh.faces[face_index as usize];
        let (low, high) = face
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |acc, v| {
                let u = angle[v];
                (acc.0.min(u), acc.1.max(u))
            });
        let straddles = group.kind == Kind::Cylindrical && high - low > 0.5;

        let corner_shifts: Vec<u8> = face
            .iter()
            .map(|v| u8::from(straddles && angle[v] < 0.5))
            .collect();
        for (corner, &shift) in face.iter().zip(&corner_shifts) {
            let u = angle[corner] + f32::from(shift);
            u_range = (u_range.0.min(u), u_range.1.max(u));
        }
        shifts.push(corner_shifts);
    }

    let v_range = along
        .values()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |acc, &v| {
            (acc.0.min(v), acc.1.max(v))
        });

    // Every corner's normalized coordinates, for the density warp: corners
    // rather than vertices so a vertex shared by six fine faces counts as the
    // detail it carries.
    let mut u_samples = Vec::new();
    let mut v_samples = Vec::new();
    for (&face_index, corner_shifts) in group.faces.iter().zip(&shifts) {
        let face = &mesh.faces[face_index as usize];
        for (corner, &shift) in face.iter().zip(corner_shifts) {
            u_samples.push(normalize(angle[corner] + f32::from(shift), u_range));
            v_samples.push(normalize(along[corner], v_range));
        }
    }
    // Head only: `refine_face` is the one thing in the pipeline that makes a
    // region's vertex density mean "detail lives here". Elsewhere clustering
    // is a tessellation artifact — a tapering tail keeps its ring count while
    // its radius vanishes, so its tip reads dense while carrying nothing, and
    // warping toward it re-stretched the faces `tests/uv.rs` already names as
    // the worst on the body.
    let (u_warp, v_warp) = if group.zone == Zone::Head {
        (density_warp(&u_samples), density_warp(&v_samples))
    } else {
        (Vec::new(), Vec::new())
    };

    Projection {
        kind: group.kind,
        angle,
        along,
        shifts,
        u_range,
        v_range,
        mean_radius: radius_sum / vertices.len().max(1) as f32,
        area: group
            .faces
            .iter()
            .map(|&face| face_area(mesh, face as usize))
            .sum(),
        zone: group.zone,
        u_warp,
        v_warp,
    }
}

/// How many bins a chart's density warp is built over.
const WARP_BINS: usize = 32;

/// How far a chart's texels follow its vertices, `0` uniform, `1` fully.
///
/// At `1.0` texel rows are allotted purely by vertex count, which starves any
/// span of surface the mesh crosses in a few big faces — bilinear samples
/// there would cross the whole span inside one texel. Halfway keeps every
/// region at least half its uniform share while the dense regions roughly
/// double theirs.
///
/// Provenance: **tuned by render** (#158), against the lower face.
const WARP_FOLLOW: f32 = 0.5;

/// A monotone remap of a chart axis toward the mesh's own vertex density.
///
/// A chart's texels used to be spread uniformly over its extent, but a chart's
/// DETAIL is not: the head chart spends four refinement passes on the mouth
/// band, so the lower face carries most of the chart's faces in a tenth of its
/// height — and got a tenth of its texels. Painted at that density the jaw
/// flank came out as magnified single-texel rectangles (#158). The atlas is
/// per-texel positional — every painter is a function of `texel.position` — so
/// warping where the texels go changes only how finely each region is
/// sampled, never what is painted there.
///
/// Returns piecewise-linear knots mapping normalized `[0, 1]` onto itself:
/// the corner-count CDF blended [`WARP_FOLLOW`] of the way from identity.
/// Charts with fewer corners than fill its bins keep the identity — a CDF
/// built from a handful of samples is noise, and the charts that small are
/// exactly the ones a warp cannot help.
fn density_warp(samples: &[f32]) -> Vec<f32> {
    if samples.len() < WARP_BINS * 4 {
        return Vec::new();
    }
    let mut counts = [0.0f32; WARP_BINS];
    for &at in samples {
        let bin = ((at * WARP_BINS as f32) as usize).min(WARP_BINS - 1);
        counts[bin] += 1.0;
    }
    // Bound how far any one bin can depart from the uniform share before the
    // blend, so a single spike — a pole, a seam pile-up — cannot starve the
    // rest of the chart or blow a face past the stretch guard.
    let uniform_share = samples.len() as f32 / WARP_BINS as f32;
    for count in &mut counts {
        *count = count.clamp(uniform_share / WARP_SLOPE, uniform_share * WARP_SLOPE);
    }
    let total: f32 = counts.iter().sum();
    let mut knots = Vec::with_capacity(WARP_BINS + 1);
    let mut running = 0.0;
    knots.push(0.0);
    for (bin, &count) in counts.iter().enumerate() {
        running += count / total;
        let uniform = (bin + 1) as f32 / WARP_BINS as f32;
        knots.push(uniform + (running - uniform) * WARP_FOLLOW);
    }
    knots
}

/// How far a bin's texel share may depart from uniform, either way.
///
/// See [`density_warp`]; the clamp runs before [`WARP_FOLLOW`]'s blend, so the
/// final per-axis stretch is bounded by `1 + (WARP_SLOPE - 1) * WARP_FOLLOW`.
///
/// Provenance: **derived** from `tests/uv.rs`'s 11.5x stretch guard and the
/// 5.3x worst face outside a tail it records.
const WARP_SLOPE: f32 = 2.5;

/// Evaluates a [`density_warp`] table at `at`; an empty table is the identity.
fn warp(at: f32, knots: &[f32]) -> f32 {
    if knots.is_empty() {
        return at;
    }
    let clamped = at.clamp(0.0, 1.0);
    let scaled = clamped * (knots.len() - 1) as f32;
    let low = (scaled as usize).min(knots.len() - 2);
    let between = scaled - low as f32;
    knots[low] + (knots[low + 1] - knots[low]) * between
}

/// The axis a zone unwraps about: the bone entering it.
///
/// The entering bone is the right choice rather than an average of the zone's
/// bones — a chest carries two clavicles whose directions would cancel the spine
/// out — and it is also what a rigger would pick.
fn zone_axis(rig: &Rig, zone: Zone) -> Vec3 {
    let Some(&first) = rig.in_zone(zone).first() else {
        return landmark::UP;
    };
    let (start, end) = rig.bone(first);
    let entering = end - start;
    if entering.length_squared() > 1e-12 {
        return entering.normalize();
    }

    // The root has no entering bone; the spine leaving it serves instead, which
    // is vertical on a biped and horizontal on a quadruped.
    rig.joints
        .iter()
        .find(|joint| {
            joint.parent == Some(first) && matches!(joint.zone, Zone::Abdomen | Zone::Chest)
        })
        .map(|joint| (joint.position - rig.joints[first].position).normalize_or(landmark::UP))
        .unwrap_or(landmark::UP)
}

/// Two axes perpendicular to `axis`, with the first pointing as forward as it can.
fn frame(axis: Vec3) -> (Vec3, Vec3) {
    // A body whose axis runs fore-and-aft — a quadruped's spine — has no forward
    // to measure from, so the seam moves to the belly instead of the back.
    let reference = if axis.dot(landmark::FORWARD).abs() < 0.9 {
        landmark::FORWARD
    } else {
        landmark::UP
    };
    let tangent = (reference - axis * reference.dot(axis)).normalize_or(landmark::UP);
    (tangent, axis.cross(tangent))
}

/// Maps `value` from `range` onto `0..=1`.
fn normalize(value: f32, range: (f32, f32)) -> f32 {
    let span = range.1 - range.0;
    if span.abs() <= f32::EPSILON {
        0.5
    } else {
        ((value - range.0) / span).clamp(0.0, 1.0)
    }
}

/// Unit normal of one polygon face.
///
/// Summing the fan's triangle normals rather than taking the first three
/// corners keeps a slightly non-planar quad — which subdivision routinely
/// produces — from reporting the normal of one sliver of itself.
fn face_normal(mesh: &PolyMesh, face_index: usize) -> Vec3 {
    let face = &mesh.faces[face_index];
    if face.len() < 3 {
        return landmark::UP;
    }
    let anchor = mesh.positions[face[0] as usize];
    let total: Vec3 = (1..face.len() - 1)
        .map(|corner| {
            let a = mesh.positions[face[corner] as usize] - anchor;
            let b = mesh.positions[face[corner + 1] as usize] - anchor;
            a.cross(b)
        })
        .sum();
    total.normalize_or(landmark::UP)
}

/// Area of one polygon face.
fn face_area(mesh: &PolyMesh, face_index: usize) -> f32 {
    let face = &mesh.faces[face_index];
    if face.len() < 3 {
        return 0.0;
    }
    let anchor = mesh.positions[face[0] as usize];
    (1..face.len() - 1)
        .map(|corner| {
            let a = mesh.positions[face[corner] as usize] - anchor;
            let b = mesh.positions[face[corner + 1] as usize] - anchor;
            a.cross(b).length() * 0.5
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::rig::{SkinConfig, skin};
    use crate::subdiv::catmull_clark;

    fn unwrapped() -> (PolyMesh, UvUnwrap) {
        let skeleton = HumanoidParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
        (mesh, uv)
    }

    #[test]
    fn every_face_survives_and_keeps_its_corners() {
        let (mesh, uv) = unwrapped();
        assert_eq!(uv.faces.len(), mesh.faces.len());

        // Each unwrapped face must refer to the same mesh vertices, in order.
        let mut original: Vec<Vec<u32>> = mesh.faces.clone();
        let mut mapped: Vec<Vec<u32>> = uv
            .faces
            .iter()
            .map(|face| face.iter().map(|&v| uv.source[v as usize]).collect())
            .collect();
        original.sort();
        mapped.sort();
        assert_eq!(original, mapped);
    }

    #[test]
    fn uvs_stay_inside_the_atlas() {
        let (_, uv) = unwrapped();
        assert!(!uv.uvs.is_empty());
        for (index, coordinate) in uv.uvs.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&coordinate.x) && (0.0..=1.0).contains(&coordinate.y),
                "vertex {index} lands outside the atlas at {coordinate:?}"
            );
        }
    }

    #[test]
    fn no_face_is_smeared_across_its_chart() {
        // The defect seam handling exists to prevent: a face whose corners sit at
        // both ends of the wrap covers the whole chart, smearing the entire
        // texture across one quad. Measuring atlas area against surface area
        // catches that directly — and unlike a span test, it does not mistake a
        // small chart's honestly large faces for a defect.
        let (mesh, uv) = unwrapped();
        for (index, face) in uv.faces.iter().enumerate() {
            let chart = uv.charts[uv.chart_of_face[index] as usize];
            let world = face_area(&mesh, uv.source_face[index] as usize);
            if world <= 0.0 {
                continue;
            }
            let stretch = uv_area(&uv, face) / world / chart.texel_density().max(1e-9);
            assert!(
                stretch < 8.0,
                "face {index} in {:?} is stretched {stretch:.1}x its chart's average",
                chart.zone
            );
        }
    }

    /// Area a face covers in the atlas, by the shoelace formula.
    fn uv_area(uv: &UvUnwrap, face: &[u32]) -> f32 {
        let mut doubled = 0.0;
        for corner in 0..face.len() {
            let a = uv.uvs[face[corner] as usize];
            let b = uv.uvs[face[(corner + 1) % face.len()] as usize];
            doubled += a.x * b.y - b.x * a.y;
        }
        (doubled * 0.5).abs()
    }

    #[test]
    fn charts_do_not_share_texels() {
        let (_, uv) = unwrapped();
        for (index, chart) in uv.charts.iter().enumerate() {
            for other in &uv.charts[index + 1..] {
                assert!(
                    !chart.rect.overlaps(&other.rect),
                    "{:?} and {:?} overlap",
                    chart.zone,
                    other.zone
                );
            }
        }
    }

    #[test]
    fn the_face_lands_in_the_middle_of_its_chart() {
        // The convention that makes a face paintable as plain 2-D maths: the
        // seam is behind the head, so the face is one unbroken island.
        let (mesh, uv) = unwrapped();
        let head = uv
            .charts
            .iter()
            .position(|chart| chart.zone == Zone::Head)
            .expect("a head chart");
        let rect = uv.charts[head].rect;

        // The frontmost vertex of the head chart should sit mid-chart in u.
        let frontmost = (0..uv.vertex_count())
            .filter(|&v| {
                uv.faces.iter().enumerate().any(|(f, face)| {
                    uv.chart_of_face[f] as usize == head && face.contains(&(v as u32))
                })
            })
            .max_by(|&a, &b| {
                let za = mesh.positions[uv.source[a] as usize].z;
                let zb = mesh.positions[uv.source[b] as usize].z;
                za.total_cmp(&zb)
            })
            .expect("head chart has vertices");

        let unit_u = (uv.uvs[frontmost].x - rect.min.x) / rect.size().x;
        assert!(
            (unit_u - 0.5).abs() < 0.12,
            "the front of the head sits at u={unit_u:.3}, not mid-chart"
        );
    }

    #[test]
    fn the_head_is_given_more_texels_than_the_body() {
        let (_, uv) = unwrapped();
        let density = |zone: Zone| {
            uv.charts
                .iter()
                .find(|chart| chart.zone == zone)
                .map(Chart::texel_density)
                .expect("chart exists")
        };
        assert!(
            density(Zone::Head) > density(Zone::Abdomen) * 1.5,
            "head {:.3} vs abdomen {:.3}",
            density(Zone::Head),
            density(Zone::Abdomen)
        );
    }

    #[test]
    fn splitting_is_confined_to_seams_and_chart_edges() {
        let (mesh, uv) = unwrapped();
        assert!(uv.vertex_count() >= mesh.vertex_count());
        assert!(
            uv.vertex_count() < mesh.vertex_count() * 2,
            "unwrapping duplicated {} of {} vertices",
            uv.vertex_count() - mesh.vertex_count(),
            mesh.vertex_count()
        );
    }

    #[test]
    fn attributes_follow_the_split() {
        let (mesh, uv) = unwrapped();
        let positions = uv.gather(&mesh.positions);
        assert_eq!(positions.len(), uv.vertex_count());
        for (index, &source) in uv.source.iter().enumerate() {
            assert_eq!(positions[index], mesh.positions[source as usize]);
        }
    }

    #[test]
    fn unwrapping_is_deterministic() {
        let (_, first) = unwrapped();
        let (_, second) = unwrapped();
        assert_eq!(first, second);
    }

    #[test]
    fn a_mismatched_zone_map_is_refused_rather_than_guessed() {
        let (mesh, _) = unwrapped();
        let skeleton = HumanoidParams::default().skeleton();
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let uv = unwrap(&mesh, &rig, &[Zone::Head], &UvConfig::default());
        assert_eq!(uv, UvUnwrap::default());
    }
}
