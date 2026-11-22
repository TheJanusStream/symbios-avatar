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
//! A beast's legs are the exception to following the zones (#376). Its upper
//! arm and thigh lie inside its trunk, so the skin its zone map gives a leg
//! runs up over the shoulder and the flank: that skin is charted with the
//! trunk it is part of, and a leg's chart starts at the elbow or the stifle,
//! wrapped round the bone leaving it, so the coat is as fine where the leg
//! leaves the body as on either side.
//!
//! A beast's charts are another exception (#395): skin that faces along a
//! chart's axis has no height to spread over, so it smears. A person's trunk
//! mostly faces away from its axis, but a beast's breast and rump face straight
//! along its spine, and the underside of a head on a raised neck straight down
//! that neck. So a beast's head is wrapped round the head's own axis, from the
//! occiput to the nose, and its charts' heights go by how far the skin runs
//! (`BeastCharts`). A beast's tail curls away from the bone leaving its hips, so
//! it is wrapped round its own bones instead (#397), from the root to the tip
//! with the seam along its underside (`TailChain`).
//!
//! A person's head is the last (#469, #473). Wrapped round its vertices'
//! centroid, which its refined face pulls to just behind the lips, its back came
//! out six to eight millimetres a texel round the head and the plane under its
//! chin, which faces down the neck, up to thirty. So it is wrapped round the
//! head joint, its height counts the skin drawing in under the jaw as a beast's
//! does, and it asks for nearly the same share of the atlas as before
//! (`PersonHead`, `PERSON_HEAD_SHARE`).
//! But its face is a chart of its own, flattened the way its skin runs rather
//! than seen from anywhere (`unfold`): wrapped round anything, the sides of
//! the nose and the walls of the eye sockets face along the line they are seen
//! from. The face is still one island, the brows, eyes, nose, mouth, chin and
//! hairline all inside it, with its border in front of the ears and up over
//! the temples, and its lowest edge the head's own, at the neck. Its crown is
//! a chart of its own flattened the same way, and the rest of the head is a
//! band wrapped round the joint with its seam in the face's place.
//!
//! Unwrapping duplicates vertices — at chart boundaries, and along each chart's
//! seam — so the result carries its own vertex list with a [`UvUnwrap::source`]
//! index back into the mesh it came from.

mod pack;
mod unfold;

use glam::{DVec2, Vec2, Vec3};
use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::mesh::PolyMesh;
use crate::plan::{BeastHead, Zone};
use crate::rig::junction::Junctions;
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
    /// The body zone this chart covers: the zone map's, except that a beast's skin
    /// past the back of its skull is charted with its neck (#395).
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
    unwrap_framed(mesh, rig, zones, config, extra, None, &[])
}

/// What a beast's charts ask that a person's do not (#395).
///
/// **The head's axis.** A chart wraps round the bone entering its zone, and for a
/// head that is the top of the neck. On a person that runs up through the skull
/// from under the chin, which suits it. A beast's muzzle stands out sideways from
/// that bone, and on a raised neck the whole underside of its head faces straight
/// down it, so that skin all comes out at one height and streaks. The head's own
/// axis runs down the muzzle and leaves only the two ends of the head to face
/// along it. It also fixes the seam: the chart's middle is the crown, and the wrap
/// lands under the chin.
///
/// **The height.** Those two ends remain, and so do a beast's breast and rump,
/// which face straight along its spine. A height that also counts how far the skin
/// has drawn in toward the axis ([`along_the_skin`]) spreads them out as it does
/// any other skin. The atlas is positional, so none of this changes what a coat
/// paints, only how finely each part of it is sampled.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BeastCharts {
    /// A point on the head's axis, in body space.
    on_axis: Vec3,
    /// The head's axis, occiput to nose, in body space.
    axis: Vec3,
    /// The way the head's crown faces, in body space.
    up: Vec3,
}

impl BeastCharts {
    /// What the head `head` declares of its charts.
    pub(crate) fn of(head: &BeastHead) -> Self {
        let [back, front] = head.axis.map(|at| head.body(at));
        // **An owl's head is wrapped round its crown's line, not its face's**
        // (#422): its disc is a dish square to the line from the back of the
        // skull to the face, which a wrap round that line can only flatten by
        // its height along it, all one height across the dish - in streaks
        // from its middle out. Round the crown's line the dish is the wrap's
        // wall, and the seam runs down the back of the head.
        if head.disc.is_some() {
            return Self {
                on_axis: head.body(head.cranium.centre),
                axis: head.turn * Vec3::Y,
                up: head.turn * Vec3::Z,
            };
        }
        Self {
            on_axis: back,
            axis: (front - back).normalize_or(landmark::FORWARD),
            up: head.turn * Vec3::Y,
        }
    }

    /// The chart's axis and a point on it, when the zone's is the head's.
    fn head(beast: Option<&Self>, zone: Zone) -> Option<&Self> {
        beast.filter(|_| zone == Zone::Head)
    }
}

/// What a person's head charts ask that every other chart does not (#469,
/// #473).
///
/// **The head is wrapped round the head joint.** A chart wraps round the bone
/// entering its zone, through the centroid of its own vertices; a person's face
/// is refined ten times over where the vault is not, so that centroid stood
/// some 70 mm in front of the head joint, behind the lips, and the wrap's axis
/// ran up through the face. Seen from there the face filled most of the way
/// round and the whole back of the head a sliver of it: behind the cheek, at
/// the sideburn, over the scalp and at the nape a texel spanned six to eight
/// millimetres round the head while its rows stood under one apart, and the
/// hairline behind the ear and a beard's edge stepped. Round the head's own
/// axis every part of the head has its share of the way round.
///
/// **Its height counts the skin under the jaw.** That skin faces down the neck,
/// and by height alone it came out at a few rows, up to thirty millimetres a
/// texel on the plane under the chin. Below the head's waist the height counts
/// how far the skin has drawn in toward the axis as well, as a beast's does
/// ([`along_the_skin`]); above it only the last rings below the crown's chart
/// count ([`CROWN_CLOSING`]).
///
/// **Its face is a chart of its own** (#473). Round the joint, the skin that
/// stands out of the face and faces the side - the flanks of the nose, the
/// wings of the nostrils, the edges of the philtrum, the walls of the eye
/// sockets - runs nearly along the line from the axis, and a step along it
/// barely turns the angle: the side of the nose came out whole triangles to a
/// texel, fifteen millimetres one way at its worst hundredth where the chart
/// before #469 had spent two. Seeing the face from a point brought forward
/// toward it mended the nose's front and not its flanks, and no one point sees
/// skin that faces every way. So the face, the skin in front of the ears
/// ([`FACE_SIDE`]) from the head's own edge at the neck up over the forehead
/// and the front of the scalp, is flattened the way it runs ([`unfold`]). The
/// face is the largest island of that skin, and a head whose largest island is
/// too small to be a face keeps it in its wrap ([`FACE_LEAST`]).
///
/// **Its crown is a chart of its own, and the rest has no hole.** With the face
/// cut out below the hairline alone (#473's first chart), the wrap round the
/// joint kept the face's place in its rectangle empty, a third of it, and the
/// front of the scalp above the face and the crown, where its columns closed
/// on the pole, came out coarser than 0.24.5 had them - the front of the scalp
/// a median 1.6 times as coarse over the rolls, the crown an eighth. So the
/// face chart reaches up to [`CROWN_CUT`], everything above that is one chart
/// flattened the way it runs as the face is, from the crown seen from above,
/// and the rest - the sides and the back between the neck and the crown's
/// chart - is wrapped round the joint with its seam in front, in the face's
/// place: its rectangle is the skin it covers, a band two thirds of the way
/// round, and no texel of it is spent on nothing.
///
/// **Where the borders run, and why.** The face's runs down in front of the
/// ears, through the sideburn and over the cheek behind the corner of the jaw,
/// where a beard's flank is the one painted thing that crosses it, and up over
/// the temples to the crown's chart; the crown's is a ring round the head at
/// [`CROWN_CUT`]; and the head's at the neck, where its chart ended already. On
/// a bald head those are the seams there are: two lines up the sides of the
/// forehead in front of the ears and a ring over the top of the head, and none
/// down the back of it, where 0.24.5 and #473's first chart had theirs. The
/// brows, the eyes, the nose, the mouth, the moustache, the beard on the chin
/// and the hairline lie inside the face, never across a seam.
///
/// **What the face asks of its texels.** Every triangle of the face asks to
/// keep its own shape, the scalp above the hairline [`SCALP_DETAIL`] times its
/// own size and the skin that faces down under the jaw [`JAW_DETAIL`] times
/// (see [`PersonHead::detail`]); and once it is flat the mouth's band, where a
/// moustache grows, is magnified [`MOUTH_DETAIL`] times at its middle, every
/// place pushed out along the line from the mouth (see
/// [`PersonHead::magnify`]). Asked of the flattening instead, as #473's fixer
/// did, a band that asks more than the skin round it is squeezed by that skin,
/// and the needles at the mouth's corners turned over (#473's second fixer).
/// No triangle of any of the three charts is left turned over (`unfold`'s
/// `untangle`).
///
/// **It asks for the area it did, less a share.** The head's area is asked for
/// as before, round its vertices' centroid and by its extent along the axis,
/// without the cap at its pole, but only [`PERSON_HEAD_SHARE`] of it: laid flat
/// and with no hole, the head is finer for fewer texels. Of that the face is
/// given [`FACE_WEIGHT`] times the texels its skin would have on the band round
/// the back and the crown [`CROWN_WEIGHT`] times, each asked in the shape of
/// the skin it covers - and on a narrow head, whose sides and crown lie near
/// the joint's axis and its face far from it, the band and the crown more
/// again ([`BAND_FROM`], [`CROWN_FROM`]). A head whose face is kept in its wrap
/// asks all it did.
///
/// **What it does not change.** Every painter reads a texel's body position,
/// never its place in the atlas, so what is painted where stays where it was;
/// but a grain that fades by how finely its texel is sampled (the skin
/// painter's `resolvable`) shows more or less of itself as the sampling moves.
/// And the face is one island, never split down the nose.
#[derive(Clone, Copy, Debug)]
struct PersonHead<'a> {
    /// A point on the head's axis: the head joint, in body space.
    on_axis: Vec3,
    /// The head's axis, the bone entering it, and the two square to it: the
    /// way the body faces and its side.
    axis: Vec3,
    tangent: Vec3,
    binormal: Vec3,
    /// The hairline, as a height along the axis above the joint: above it the
    /// face chart asks [`SCALP_DETAIL`] of the face's texels.
    hairline: f32,
    /// Where the face chart takes in the whole crown, as a height along the
    /// axis above the joint ([`CROWN_CUT`]).
    cut: f32,
    /// The mouth's middle, to the side and up the axis from the joint, and
    /// half its width, from its pocket: the face chart is magnified
    /// [`MOUTH_DETAIL`] times there. `None` without a pocket.
    mouth: Option<(Vec2, f32)>,
    /// The mouth's pocket: vertices the face's flattening leaves out, ascending
    /// (#155: they are charted apart after the unwrap).
    aside: &'a [u32],
    /// Whether the face is a chart of its own, rather than kept in the wrap
    /// ([`FACE_LEAST`]): known once the head's charts are found.
    charted: bool,
}

impl<'a> PersonHead<'a> {
    /// What `rig`'s head asks of its charts, if it is a person's: `mesh`'s
    /// faces in `face_zones`, the mouth's pocket `aside`, ascending.
    fn of(
        rig: &Rig,
        beast: Option<&BeastCharts>,
        mesh: &PolyMesh,
        face_zones: &[Zone],
        aside: &'a [u32],
    ) -> Option<Self> {
        if beast.is_some() || !rig.anatomy.human_head() {
            return None;
        }
        let joint = *rig.in_zone(Zone::Head).first()?;
        let on_axis = rig.joints[joint].position;
        let axis = zone_axis(rig, Zone::Head);
        let (tangent, binormal) = frame(axis);
        let crown = mesh
            .faces
            .iter()
            .zip(face_zones)
            .filter(|(_, zone)| **zone == Zone::Head)
            .flat_map(|(face, _)| face.iter())
            .map(|&vertex| (mesh.positions[vertex as usize] - on_axis).dot(axis))
            .fold(0.0, f32::max);
        let mouth = (!aside.is_empty()).then(|| {
            let at = |vertex: &u32| {
                let offset = mesh.positions[*vertex as usize] - on_axis;
                Vec2::new(offset.dot(binormal), offset.dot(axis))
            };
            let middle = aside.iter().map(at).sum::<Vec2>() / aside.len() as f32;
            let half = aside
                .iter()
                .map(|vertex| (at(vertex).x - middle.x).abs())
                .fold(0.0, f32::max);
            (middle, half.max(f32::EPSILON))
        });
        Some(Self {
            on_axis,
            axis,
            tangent,
            binormal,
            hairline: FACE_TOP * crown,
            cut: CROWN_CUT * crown,
            mouth,
            aside,
            charted: false,
        })
    }

    /// Whether skin at `at` lies on the face chart: in front of the ears and
    /// below the crown's chart.
    fn on_the_face(&self, at: Vec3) -> bool {
        let offset = at - self.on_axis;
        let height = offset.dot(self.axis);
        let turn = offset.dot(self.binormal).atan2(offset.dot(self.tangent));
        turn.abs() <= FACE_SIDE && height < self.cut
    }

    /// Whether `face` of `mesh` stands at the head's pole, which a wrap round
    /// the head cannot chart (see [`CAP_SPAN`]).
    fn at_the_pole(&self, mesh: &PolyMesh, face: u32) -> bool {
        face_normal(mesh, face as usize).dot(self.axis).abs() > 0.85
            && span_round(mesh, face as usize, self.on_axis, self.axis) > CAP_SPAN
    }

    /// Whether skin at `at` lies on the crown's chart.
    fn on_the_crown(&self, at: Vec3) -> bool {
        (at - self.on_axis).dot(self.axis) >= self.cut
    }

    /// How many times its own size the face chart asks for skin at `at` whose
    /// outward `normal` is that: the scalp above the hairline
    /// [`SCALP_DETAIL`], the skin that faces down under the jaw
    /// [`JAW_DETAIL`], each eased in over its edge.
    fn detail(&self, at: Vec3, normal: Vec3) -> f32 {
        let ease = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let offset = at - self.on_axis;
        let height = offset.dot(self.axis);
        let fade = HAIRLINE_FADE * self.hairline.abs().max(f32::EPSILON);
        let scalp =
            1.0 + (SCALP_DETAIL - 1.0) * ease((height - self.hairline + fade) / (2.0 * fade));
        let down = -normal.dot(self.axis);
        let jaw = 1.0 + (JAW_DETAIL - 1.0) * ease((down - JAW_FROM) / JAW_EASE);
        scalp * jaw
    }

    /// The face's chart, `places` over `vertices` (ascending), started from
    /// `start`, with the mouth's band magnified [`MOUTH_DETAIL`] times at its
    /// middle (#473's second fixer).
    ///
    /// Every place is pushed out along the line from the mouth's middle - the
    /// middle of `rim`, the vertices of the mouth's pocket on the skin - by an
    /// amount that grows with how far along that line it lies, measured in an
    /// ellipse [`MOUTH_WIDE`] half-widths of the mouth to each side and
    /// [`MOUTH_ABOVE`] and [`MOUTH_BELOW`] above and below it, turned as the
    /// chart turned from its start. Along each line the step outward is
    /// `1 + (k - 1)(1 - s(r))` times the step before it, `s` the smooth step and
    /// `r` the reach in the ellipse, out to its rim, and one past it: so the
    /// middle is magnified `k` each way, the band eases out to the face's own
    /// steps at its rim, and nothing is squeezed - across the line every place
    /// moves out more than it moved along it. Each line is laid on itself, in
    /// order, so the chart stays one sheet; but a triangle's sides stay
    /// straight where the push bends, and a needle - a fanned polygon's corners
    /// all but in a line - bends over: 24 to 50 on each of the 43 rolls, most
    /// round the band's rim below and beside the mouth's corners, where the
    /// ellipse turns tightest. `unfold`'s `untangle` lays each back by the few
    /// thousandths of a millimetre it bent (#473's third fixer).
    fn magnify(
        &self,
        mesh: &PolyMesh,
        rim: &[u32],
        vertices: &[u32],
        places: &mut [DVec2],
        start: impl Fn(Vec3) -> Vec2,
    ) {
        let Some((_, half)) = self.mouth else {
            return;
        };
        let (mut middle, mut count) = (DVec2::ZERO, 0.0f64);
        for vertex in rim {
            if let Ok(index) = vertices.binary_search(vertex) {
                middle += places[index];
                count += 1.0;
            }
        }
        if count == 0.0 {
            return;
        }
        let middle = middle / count;
        // The chart's turn from where it started.
        let begun: Vec<DVec2> = vertices
            .iter()
            .map(|&vertex| start(mesh.positions[vertex as usize]).as_dvec2())
            .collect();
        let centre = |places: &[DVec2]| places.iter().sum::<DVec2>() / places.len() as f64;
        let (from, to) = (centre(&begun), centre(places));
        let (mut cos, mut sin) = (0.0f64, 0.0f64);
        for (one, two) in begun.iter().zip(places.iter()) {
            let (one, two) = (*one - from, *two - to);
            cos += one.dot(two);
            sin += one.perp_dot(two);
        }
        let turn = DVec2::new(cos, sin).normalize_or(DVec2::X);
        let k = f64::from(MOUTH_DETAIL);
        let half = f64::from(half);
        for place in places.iter_mut() {
            let offset = *place - middle;
            let own = DVec2::new(
                turn.x * offset.x + turn.y * offset.y,
                -turn.y * offset.x + turn.x * offset.y,
            );
            let up = own.y
                / (half
                    * f64::from(if own.y > 0.0 {
                        MOUTH_ABOVE
                    } else {
                        MOUTH_BELOW
                    }));
            let across = own.x / (f64::from(MOUTH_WIDE) * half);
            let reach = (across * across + up * up).sqrt();
            let grown = if reach >= 1.0 {
                reach + 0.5 * (k - 1.0)
            } else {
                reach + (k - 1.0) * (reach - reach.powi(3) + 0.5 * reach.powi(4))
            };
            let scale = if reach > 1e-9 { grown / reach } else { k };
            *place = middle + offset * scale;
        }
    }

    /// Whether `face` of `mesh` is part of the mouth's pocket.
    fn set_aside(&self, mesh: &PolyMesh, face: u32) -> bool {
        mesh.faces[face as usize]
            .iter()
            .any(|vertex| self.aside.binary_search(vertex).is_ok())
    }

    /// Where the face's flattening starts `at`: wrapped round the joint at
    /// `radius`, in metres.
    fn start(&self, at: Vec3, radius: f32) -> Vec2 {
        let offset = at - self.on_axis;
        let turn = offset.dot(self.binormal).atan2(offset.dot(self.tangent));
        Vec2::new(turn * radius, offset.dot(self.axis))
    }

    /// Where the flattening of the rest of the head starts `at`: wrapped round
    /// the joint at `radius`, in metres, with its seam in front, in the face's
    /// place.
    fn round_the_back(&self, at: Vec3, radius: f32) -> Vec2 {
        let offset = at - self.on_axis;
        let turn = offset.dot(self.binormal).atan2(offset.dot(self.tangent));
        Vec2::new(
            turn.rem_euclid(std::f32::consts::TAU) * radius,
            offset.dot(self.axis),
        )
    }

    /// Where the crown's flattening starts `at`: seen from above, down the
    /// axis.
    fn seen_from_above(&self, at: Vec3) -> Vec2 {
        let offset = at - self.on_axis;
        Vec2::new(offset.dot(self.tangent), offset.dot(self.binormal))
    }

    /// The face's chart, over `group`, whose faces' corners are `vertices`.
    ///
    /// The mouth's pocket is left out of the flattening (#155): a fold behind
    /// the lips has no place in a chart of the face, and it is charted apart
    /// after the unwrap. Its own vertices wait at the middle of the lips' seam.
    fn flatten(&self, mesh: &PolyMesh, group: &Group, vertices: Vec<u32>) -> Projection {
        let (pocket, skin): (Vec<u32>, Vec<u32>) = group
            .faces
            .iter()
            .partition(|&&face| self.set_aside(mesh, face));
        let (solved, loose) = joined(mesh, &skin);
        let radius = vertices
            .iter()
            .map(|&vertex| {
                let offset = mesh.positions[vertex as usize] - self.on_axis;
                (offset - self.axis * offset.dot(self.axis)).length()
            })
            .sum::<f32>()
            / vertices.len().max(1) as f32;
        let start = |at: Vec3| match group.kind {
            Kind::Unfolded => self.start(at, radius),
            Kind::Planar => self.seen_from_above(at),
            Kind::Cylindrical => self.round_the_back(at, radius),
        };
        let flat = match group.kind {
            Kind::Unfolded => {
                let mut rim: Vec<u32> = pocket
                    .iter()
                    .flat_map(|&face| mesh.faces[face as usize].iter().copied())
                    .collect();
                rim.sort_unstable();
                rim.dedup();
                unfold::unfold(
                    mesh,
                    &solved,
                    start,
                    |at, normal| self.detail(at, normal),
                    |vertices, places| self.magnify(mesh, &rim, vertices, places, start),
                )
            }
            _ => unfold::unfold(mesh, &solved, start, |_, _| 1.0, |_, _| {}),
        };
        let mut angle = HashMap::with_capacity(vertices.len());
        let mut along = HashMap::with_capacity(vertices.len());
        for (&vertex, place) in flat.vertices.iter().zip(&flat.places) {
            angle.insert(vertex, place.x);
            along.insert(vertex, place.y);
        }
        // A scrap joined to the chart at a corner alone ([`merge_scraps`]) is
        // laid beside that corner as its start lays it.
        let mut placing = true;
        while placing {
            placing = false;
            for &face in &loose {
                let corners = &mesh.faces[face as usize];
                let Some(&anchor) = corners.iter().find(|vertex| angle.contains_key(*vertex))
                else {
                    continue;
                };
                let from = start(mesh.positions[anchor as usize]);
                let at = Vec2::new(angle[&anchor], along[&anchor]);
                for &vertex in corners {
                    if let std::collections::hash_map::Entry::Vacant(empty) = angle.entry(vertex) {
                        let place = at + start(mesh.positions[vertex as usize]) - from;
                        empty.insert(place.x);
                        along.insert(vertex, place.y);
                        placing = true;
                    }
                }
            }
        }
        let (mut seam, mut count) = (Vec2::ZERO, 0.0f32);
        for &face in &pocket {
            for vertex in &mesh.faces[face as usize] {
                if let (Some(&u), Some(&v)) = (angle.get(vertex), along.get(vertex)) {
                    seam += Vec2::new(u, v);
                    count += 1.0;
                }
            }
        }
        let seam = seam / count.max(1.0);
        for &vertex in &vertices {
            angle.entry(vertex).or_insert(seam.x);
            along.entry(vertex).or_insert(seam.y);
        }
        let range = |values: &HashMap<u32, f32>| {
            vertices
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), vertex| {
                    (low.min(values[vertex]), high.max(values[vertex]))
                })
        };
        Projection {
            kind: match group.kind {
                Kind::Planar => Kind::Planar,
                _ => Kind::Unfolded,
            },
            u_range: range(&angle),
            v_range: range(&along),
            angle,
            along,
            shifts: group
                .faces
                .iter()
                .map(|&face| vec![0; mesh.faces[face as usize].len()])
                .collect(),
            mean_radius: radius,
            radii: HashMap::new(),
            area: group
                .faces
                .iter()
                .map(|&face| face_area(mesh, face as usize))
                .sum(),
            zone: group.zone,
            u_warp: Vec::new(),
            v_warp: Vec::new(),
        }
    }
}

/// How far round the head from straight ahead a person's face chart reaches,
/// in radians (#473): sixty degrees, in front of the ears.
///
/// Measured on the five heads of `tests/uv.rs`, before #474's packer: at fifty
/// degrees the plane under the jaw, cut at the face's corners, came out
/// coarser - its ninetieth percentile 4.05 mm a texel against 2.76 here - and
/// at seventy the face spread round to where it faces the side, its median
/// 1.41 against 1.35.
/// Provenance: **measured** (#473).
pub(crate) const FACE_SIDE: f32 = 1.05;

/// Where a person's hairline stands, as a share of the way from the head joint
/// to the crown (#473): `hair::follicle::scalp` puts it half a frame above the
/// eye line, about half way up a head two frames tall whose eye line is at its
/// joint. Above it the face chart asks [`SCALP_DETAIL`] of the face's texels.
/// (It was where #473's first face chart ended above; measured as
/// [`FACE_SIDE`] was, at 0.4 the top of the forehead went back to the wrap and
/// the face's ninetieth percentile with it, 2.68 mm a texel against 2.10.)
/// Provenance: **derived** from the scalp's hairline.
const FACE_TOP: f32 = 0.5;

/// Above what share of the way from the head joint to the crown a person's
/// head is one chart of its own, its crown, flattened the way it runs from the
/// crown seen from above (#473's fixer): see [`PersonHead`].
///
/// Chosen as the height where the skin of a head two frames tall has turned
/// about half way from facing out to facing up, so that both the wrap below and
/// the crown's chart above see their skin at less than half a right angle from
/// square: the crown seen from above, under that, flattens with less than a
/// sixth of stretch to take out, and the band below it ends before its columns
/// close on the pole, where 0.24.5's wrap streaked round the crown.
/// Provenance: **chosen** (#473's fixer).
pub(crate) const CROWN_CUT: f32 = 0.7;

/// How many times the texels an equal area of the band round the back of a
/// person's head gets its crown's chart gets ([`CROWN_CUT`]).
///
/// Measured over the 43 rolls against 0.24.5, the crown's median mm a texel
/// on each body over 0.24.5's: at 1.15 (the face at 4.3), 0.98 at the median
/// and 1.10 at worst; at 1.6, 0.85 and 0.96; at 2.0 (the face at 5.5), 0.83,
/// the band round the back paying for it, the nape 1.12 times 0.24.5's on the
/// worst body.
/// Provenance: **measured** (#473's fixer).
const CROWN_WEIGHT: f32 = 1.6;

/// How many times its own size the face chart asks for the scalp above the
/// hairline ([`FACE_TOP`]): the front of the scalp is not the face, but 0.24.5
/// spent the face's columns on it, 2.00 mm a texel at the median over the
/// rolls where #473's first chart, which left it to the wrap, spent 2.99.
///
/// Measured over the 43 rolls, at 0.66 the front of the scalp 0.92 of 0.24.5's
/// at the median and 1.04 at worst; at 0.7, 0.87 and 0.97, the face paying; at
/// 0.6, 0.96 and 1.08. With the mouth's band magnified ([`MOUTH_DETAIL`]) the
/// face's other texels are fewer: at 0.66, 1.07 at the median and 17 of the
/// rolls more than a twentieth coarser than 0.24.5's; at 0.8, 0.89 and 0.97.
/// Provenance: **measured** (#473's fixer, and its second).
const SCALP_DETAIL: f32 = 0.8;

/// Over what share of the hairline's height the face chart eases from the
/// face's detail to the scalp's ([`SCALP_DETAIL`]), either side of it.
/// Provenance: **chosen** (#473's fixer): a step in the detail asked is a fold
/// of stretch across it, and the hairline lies inside the face chart.
const HAIRLINE_FADE: f32 = 0.15;

/// How many times the face chart is magnified at the middle of the mouth's
/// band, each way: the upper lip, where a moustache grows, the lips and the top
/// of the chin, easing out to the face's own steps at the band's rim
/// ([`MOUTH_WIDE`], [`MOUTH_ABOVE`], [`MOUTH_BELOW`]; see
/// [`PersonHead::magnify`]).
///
/// 0.24.5's head chart warped its columns to the refined lips and spent them
/// on the whole of the mouth's width, so its moustache and the philtrum were
/// charted finely - the philtrum 1.01 mm a texel at the median over the rolls,
/// against the face's 1.70. #473's fixer asked the flattening for 1.55 times
/// the size there; squeezed by the face round it, the needles at the mouth's
/// corners turned over, 13 texels of rc10's face laid twice and 9.7 turned
/// over, and the philtrum still 1.15 times 0.24.5's at the median. Magnified
/// once the face is flat, at 1.55, the philtrum 1.08 times 0.24.5's at the
/// median and 27 of the 43 rolls a twentieth coarser or more; at two, 0.89 and
/// none, the moustache 0.67 of 0.24.5's at the median - on seeds 175, 237 and
/// 7, 1,089, 673 and 1,090 of the moustache's texels, against 0.24.5's 769,
/// 494 and 779, their footprints 0.90, 0.82 and 0.83 mm long at the median
/// against 1.07, 0.88 and 1.03.
/// Provenance: **measured** (#473's fixer, and its second).
const MOUTH_DETAIL: f32 = 2.0;

/// How far the mouth's band reaches to each side, in half the width of the
/// mouth's pocket. At 2.4, the skin level with the mouth, out to where it
/// faces the side, 0.84 of 0.24.5's mm a texel at the median over the rolls
/// and roll 11's 1.09 times; at 3.5, 0.77 and none above 0.96.
/// Provenance: **measured** with [`MOUTH_DETAIL`] (#473's second fixer).
const MOUTH_WIDE: f32 = 3.5;

/// How far the mouth's band reaches above the mouth, in half the width of the
/// mouth's pocket: past the upper lip to the foot of the nose. Reaching less
/// than half as far, the band's edge lay across the moustache and stretched
/// it, a coarser moustache for a finer mouth. Provenance: **measured** with
/// [`MOUTH_DETAIL`].
const MOUTH_ABOVE: f32 = 3.0;

/// How far the mouth's band reaches below the mouth, in half the width of the
/// mouth's pocket. Provenance: **measured** with [`MOUTH_DETAIL`].
const MOUTH_BELOW: f32 = 1.2;

/// How many times its own size the face chart asks for the skin that faces
/// down under the jaw, past [`JAW_FROM`] of straight down and eased in over
/// [`JAW_EASE`] more (#473's second fixer).
///
/// Laid flat with the face, the plane under the chin came out 0.56 of 0.24.5's
/// mm a texel at the median over the rolls, the finest skin on the head
/// relative to it, which a camera at a person's side rarely sees. At four
/// fifths, 0.67, and no roll above 0.75; at three quarters, slider corner rc4
/// 1.10 times 0.24.5's.
/// Provenance: **measured** (#473's second fixer).
const JAW_DETAIL: f32 = 0.8;

/// How far toward straight down, as the cosine from the head's axis, the skin
/// under the jaw starts asking [`JAW_DETAIL`]: the underside of the chin and
/// the throat, not the front of the chin. Provenance: **chosen** (#473's
/// second fixer).
const JAW_FROM: f32 = 0.3;

/// Over how much more of the cosine [`JAW_DETAIL`] eases in. Provenance:
/// **chosen** (#473's second fixer).
const JAW_EASE: f32 = 0.4;

/// How much of the atlas a person's head asks for, as a share of what its one
/// chart round its vertices' centroid asked before #469 (#473).
///
/// A head laid flat spends its texels as evenly each way as it can, where the
/// wrap round the joint spent them on rows, so it is finer for the same
/// texels. The ceiling is the footprints the bake keeps for the painter's
/// beard (`texture::bake::Footprints`): on the default head 396 sixteen-texel
/// tiles at 0.24.5, and since the owner raised it for #473 (2026-11-14) at
/// most 576, about 0.4 MB more a build in the web worker, to be spent so that
/// no region of the head is charted coarser than 0.24.5's. At 0.86 (#473's
/// fixer) 452 tiles, 460 to 484 under a nudge of libm, and behind the cheek on
/// narrow heads up to 1.9 times 0.24.5's mm a texel; here 526, 517 to 543
/// under a nudge, every region of the 43 rolls within a twentieth of 0.24.5's
/// at the median and the face 0.85 of it. At 1.0, 545 tiles and 560 under a
/// nudge; at 1.05, 555 and 576.
///
/// **Since #477 the ceiling is held in bytes**, the 3.56 MB those 576 tiles
/// weighed (`texture::bake::CEILING`): a tile keeps 4 KB since then, and the
/// default head here 516 of them, 2.2 MB. And it is read on real builds, where
/// the head is a larger share of the atlas than on the bare default skeleton:
/// the dearest person measured, at the budget sweep's lean, thin and tall
/// corner with its head long and broad, keeps 3.26 MB.
/// Provenance: **measured** (#473, its fixer and its second).
const PERSON_HEAD_SHARE: f32 = 0.97;

/// The least share of a person's head, by area, its face chart may cover
/// (#473): a head whose face comes out smaller keeps its face in its wrap.
///
/// A face covers a quarter of a head and more. But a head shrunk to the end of
/// its slider comes apart into islands, and the largest there was a scrap of
/// the refined lips a square centimetre across, laid out over itself at a
/// thirtieth of a millimetre a texel.
/// Provenance: **measured** (#473): over the 132 rolls, slider corners and
/// draws measured the faces cover 0.24 to 0.48 of their heads, and the five
/// heads that came apart 0.16 and less.
const FACE_LEAST: f32 = 0.2;

/// How many times the texels an equal area of the band round the back of a
/// person's head gets its face chart gets (#473).
///
/// The face chart is most of the head's skin as it asks for it, so this moves
/// the band round the back more than the face: over the 43 rolls, at 4.3 and
/// at 5.5 (the crown at 1.15 and at 2.0) the face's median 1.40 and 1.39 mm a
/// texel, where the scalp went from 0.73 of 0.24.5's to 0.82 of it; the face
/// is finer only for a larger [`PERSON_HEAD_SHARE`]. (On #473's first chart, at
/// four times, the face 1.32 mm a texel and behind the cheek 2.81, against
/// 0.24.5's 1.71 and 3.31.) With the mouth's band magnified and the band round
/// the back weighted on narrow heads (#473's second fixer), at five the face
/// 0.85 of 0.24.5's at the median over the rolls and behind the cheek 0.79; at
/// six 0.82 and 0.84, the band's slack on the rolls spent on the face.
/// Provenance: **measured** (#473, its fixer and its second).
const FACE_WEIGHT: f32 = 6.0;

/// From how many times as far from the head's axis as the band round the back
/// of a person's head its face lies, on average, the band is weighted more
/// (#473's second fixer): by that ratio over this, to the power
/// [`BAND_POWER`].
///
/// **Why.** 0.24.5 wrapped the whole head round its joint, so its columns were
/// narrow where the skin lies near the axis: on a narrow or long head the
/// sides and the back, beside a face that lies far forward. Laid flat, every
/// chart is as fine as its share, and behind the cheek such heads came out up
/// to 1.9 times 0.24.5's mm a texel (slider corner rc6; the narrowest head 1.75
/// times) where the face came out half of it. Over the 127 rolls, slider
/// corners and draws measured, 0.24.5's mm a texel behind the cheek over its
/// face's ran as this ratio to the power -1.62 (correlation -0.93), flat at
/// about two below 1.5 and down to 0.63 at 2.6: a texel's area as the square,
/// hence [`BAND_POWER`]. From 1.55, behind the cheek within 1.11 of 0.24.5's on
/// every body, but the large head of draw u23's scalp grain drawn at 0.269 of
/// its resolvable fraction against 0.24.5's 0.298 (`texture::skin`'s
/// `ScalpFloor`); from 1.4, 0.300, and every body's band within 1.07 of
/// 0.24.5's.
/// Provenance: **measured** (#473's second fixer).
const BAND_FROM: f32 = 1.4;

/// The power [`BAND_FROM`]'s ratio weights the band by: twice the -1.62 of
/// 0.24.5's band-to-face ratio, rounded. Provenance: **derived** (#473's
/// second fixer).
const BAND_POWER: f32 = 3.2;

/// From how many times as far from the head's axis as the crown's chart its
/// face lies the crown is weighted more ([`BAND_FROM`]'s reasoning, the crown
/// in 0.24.5's last rings round its pole): by that ratio over this, to the
/// power [`CROWN_POWER`]. Over the 127 bodies, at 2.0 and the power two, the
/// crown on 8 of the 24 draws more than a twentieth coarser than 0.24.5's
/// (draw u3 1.27 times) once the band was weighted from 1.4; here 1 (u3,
/// 1.06), and of the slider corners rc15 1.29 times, rc3 1.09 and fallback rc0
/// 1.17.
/// Provenance: **measured** (#473's second fixer).
const CROWN_FROM: f32 = 1.8;

/// The power [`CROWN_FROM`]'s ratio weights the crown by.
/// Provenance: **measured** with [`CROWN_FROM`] (#473's second fixer).
const CROWN_POWER: f32 = 2.5;

/// A beast's tail, charted along its bones rather than round the first of them
/// (#397).
///
/// Wrapped round the bone leaving the hips, a tail that curls away from it runs
/// across that axis rather than along it, so the whole of its upper length comes
/// out at a handful of heights: a few texels along a tail whose coat is painted in
/// rings along it, which blurred them into a line down its top. Here the height
/// is how far along the bones the skin lies and the angle is taken round the bone
/// beside it, in a frame carried down the chain without twisting, so the chart
/// unrolls the tail however it curls.
///
/// **Which bone is beside a point** is decided by the planes that halve the bend
/// at each joint, not by the nearest bone: the skin on the outside of a bend lies
/// past the end of one bone and short of the start of the next, and the nearest
/// bone put all of it at the joint's one height. Between two such planes the
/// height runs from one joint's to the next in proportion, so it is continuous
/// across every joint, and so is the angle, which a frame without twist keeps.
///
/// **The seam runs along the underside**, which a camera behind and above the
/// beast sees least, and the chart's middle is the tail's top.
#[derive(Clone, Debug)]
struct TailChain {
    /// The hips, then every joint of the tail, root to tip.
    joints: Vec<Vec3>,
    /// How far along the bones each joint lies.
    reach: Vec<f32>,
    /// Each bone's direction, root to tip.
    bones: Vec<Vec3>,
    /// The plane each joint's bend is halved by, as its normal: the first and
    /// last bones' own directions at the two ends.
    halves: Vec<Vec3>,
    /// The top of each bone: square to it, and carried down the chain from the
    /// first without twisting.
    tops: Vec<Vec3>,
}

/// Where a point lies by a [`TailChain`].
#[derive(Clone, Copy, Debug)]
struct OnTheTail {
    /// How far along the bones.
    along: f32,
    /// The point's offset from the bone beside it, square to that bone.
    flat: Vec3,
    /// That bone's direction.
    axis: Vec3,
    /// That bone's top.
    top: Vec3,
}

impl TailChain {
    /// The tail on `rig`, if it has one.
    fn of(rig: &Rig) -> Option<Self> {
        let tail = rig.in_zone(Zone::Tail);
        // The tip: the tail joint no other tail joint hangs from.
        let tip = *tail.iter().find(|&&joint| {
            !tail
                .iter()
                .any(|&other| rig.joints[other].parent == Some(joint))
        })?;
        let mut chain = vec![tip];
        let mut at = tip;
        while let Some(parent) = rig.joints[at].parent {
            chain.push(parent);
            if rig.joints[parent].zone != Zone::Tail {
                break;
            }
            at = parent;
        }
        chain.reverse();
        let mut joints: Vec<Vec3> = chain
            .iter()
            .map(|&joint| rig.joints[joint].position)
            .collect();
        joints.dedup_by(|next, previous| next.distance_squared(*previous) < 1e-12);
        if joints.len() < 2 {
            return None;
        }

        let bones: Vec<Vec3> = joints
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).normalize())
            .collect();
        let mut reach = vec![0.0];
        for pair in joints.windows(2) {
            reach.push(reach[reach.len() - 1] + pair[0].distance(pair[1]));
        }
        let mut halves = vec![bones[0]];
        for pair in bones.windows(2) {
            halves.push((pair[0] + pair[1]).normalize_or(pair[1]));
        }
        halves.push(bones[bones.len() - 1]);

        // The top of a tail leaving the hips straight back is up, and it turns
        // with the tail in the body's middle plane: back for one hanging down,
        // forward for one carried straight up.
        let square = |top: Vec3, bone: Vec3| (top - bone * top.dot(bone)).normalize_or_zero();
        let first = square(Vec3::X.cross(bones[0]), bones[0]);
        let mut tops = vec![if first == Vec3::ZERO {
            frame(bones[0]).0
        } else {
            first
        }];
        for pair in bones.windows(2) {
            let carried = crate::det::from_rotation_arc(pair[0], pair[1]) * tops[tops.len() - 1];
            let top = square(carried, pair[1]);
            tops.push(if top == Vec3::ZERO {
                frame(pair[1]).0
            } else {
                top
            });
        }
        Some(Self {
            joints,
            reach,
            bones,
            halves,
            tops,
        })
    }

    /// Where `at` lies along the tail and round it.
    fn place(&self, at: Vec3) -> OnTheTail {
        let last = self.bones.len() - 1;
        // The bone between the two planes `at` lies between, the nearest such if
        // a tight curl puts it between more than one pair, and the nearest bone if
        // it is between none.
        let mut best: Option<(bool, f32, usize)> = None;
        for bone in 0..=last {
            let (start, end) = (self.joints[bone], self.joints[bone + 1]);
            let past = (at - start).dot(self.halves[bone]);
            let short = (end - at).dot(self.halves[bone + 1]);
            let between = (bone == 0 || past >= 0.0) && (bone == last || short >= 0.0);
            let length = self.reach[bone + 1] - self.reach[bone];
            let t = ((at - start).dot(self.bones[bone]) / length).clamp(0.0, 1.0);
            let off = at.distance(start.lerp(end, t));
            let better = match best {
                None => true,
                Some((was_between, was_off, _)) => {
                    (between && !was_between) || (between == was_between && off < was_off)
                }
            };
            if better {
                best = Some((between, off, bone));
            }
        }
        let bone = best.map_or(0, |(_, _, bone)| bone);

        let (start, end) = (self.joints[bone], self.joints[bone + 1]);
        let axis = self.bones[bone];
        let past = (at - start).dot(self.halves[bone]);
        let short = (end - at).dot(self.halves[bone + 1]);
        let along = if bone == 0 && past < 0.0 {
            // Short of the hips, along the first bone.
            past
        } else if bone == last && short < 0.0 {
            // Past the tip, along the last bone.
            self.reach[bone + 1] - short
        } else {
            let share = past.max(0.0) / (past.max(0.0) + short.max(0.0)).max(f32::EPSILON);
            self.reach[bone] + (self.reach[bone + 1] - self.reach[bone]) * share
        };
        let offset = at - start;
        OnTheTail {
            along,
            flat: offset - axis * offset.dot(axis),
            axis,
            top: self.tops[bone],
        }
    }
}

/// Unwraps `mesh` as [`unwrap_with`] does, by a beast's rules if it is given
/// them, leaving the vertices `aside` - a person's mouth pocket, charted apart
/// afterwards (#155) - out of a person's face chart's flattening.
pub(crate) fn unwrap_framed(
    mesh: &PolyMesh,
    rig: &Rig,
    zones: &[Zone],
    config: &UvConfig,
    extra: &[Vec2],
    beast: Option<&BeastCharts>,
    aside: &[u32],
) -> (UvUnwrap, Vec<Rect>) {
    if mesh.faces.is_empty() || zones.len() != mesh.positions.len() {
        return (UvUnwrap::default(), Vec::new());
    }

    let junctions = Junctions::of(rig);
    let face_zones: Vec<Zone> = mesh
        .faces
        .iter()
        .enumerate()
        .map(|(index, face)| {
            junction_zone(
                &junctions,
                dominant_zone(face, zones),
                mesh.face_centroid(index),
            )
        })
        .collect();
    let tail = beast.and_then(|_| TailChain::of(rig));
    let mut aside = aside.to_vec();
    aside.sort_unstable();
    aside.dedup();
    let mut person = PersonHead::of(rig, beast, mesh, &face_zones, &aside);
    let groups = connected_groups(
        mesh,
        rig,
        &face_zones,
        beast,
        tail.as_ref(),
        person.as_ref(),
    );
    if let Some(person) = &mut person {
        person.charted = groups.iter().any(|group| group.kind == Kind::Unfolded);
    }

    // Project every chart into its own local square first, then pack.
    let mut projections = Vec::with_capacity(groups.len());
    let mut sizes: Vec<Vec2> = Vec::with_capacity(groups.len() + extra.len());
    for group in &groups {
        let projection = project(mesh, rig, group, beast, tail.as_ref(), person.as_ref());
        sizes.push(projection.atlas_size(config));
        projections.push(projection);
    }
    // A person's head asks for the atlas it always asked for (#469), its face
    // its share of that (#473): see [`PersonHead`].
    if person.as_ref().is_some_and(|person| person.charted) {
        // Asked round the vertices' centroid without the cap at the pole, as
        // before #473: faces there stand at every angle round that wrap at
        // once, and where one lands decides whether it reaches the whole way
        // round, the head's ask a seventh larger or not under a last-bit change
        // of libm.
        let mut head: Vec<u32> = groups
            .iter()
            .filter(|group| group.zone == Zone::Head)
            .flat_map(|group| group.faces.iter().copied())
            .filter(|&face| {
                !person
                    .as_ref()
                    .is_some_and(|person| person.at_the_pole(mesh, face))
            })
            .collect();
        head.sort_unstable();
        let whole = Group {
            zone: Zone::Head,
            kind: Kind::Cylindrical,
            faces: head,
        };
        let asked = project(mesh, rig, &whole, beast, tail.as_ref(), None).atlas_size(config);
        let asked = PERSON_HEAD_SHARE * asked.x * asked.y;
        // How far from the axis the face lies, and the rest of the head.
        let radius = |kind: Kind| {
            groups
                .iter()
                .zip(&projections)
                .filter(|(group, _)| group.zone == Zone::Head && group.kind == kind)
                .map(|(_, projection)| projection.mean_radius)
                .fold(0.0f32, f32::max)
        };
        let face = radius(Kind::Unfolded);
        let narrow = |kind: Kind, from: f32, power: f32| {
            (face / radius(kind).max(f32::EPSILON) / from)
                .max(1.0)
                .powf(power)
        };
        let band = narrow(Kind::Cylindrical, BAND_FROM, BAND_POWER);
        let crown = narrow(Kind::Planar, CROWN_FROM, CROWN_POWER);
        let weight = |group: &Group| match group.kind {
            Kind::Unfolded => FACE_WEIGHT,
            Kind::Planar => CROWN_WEIGHT * crown,
            Kind::Cylindrical => band,
        };
        let own: f32 = groups
            .iter()
            .zip(&sizes)
            .filter(|(group, _)| group.zone == Zone::Head)
            .map(|(group, size)| weight(group) * size.x * size.y)
            .sum();
        let scale = (asked / own.max(f32::EPSILON)).sqrt();
        for (group, size) in groups.iter().zip(&mut sizes) {
            if group.zone == Zone::Head {
                *size *= scale * weight(group).sqrt();
            }
        }
    } else if person.is_some() {
        // A head whose face is kept in its wrap asks all it did at 0.24.5, each
        // wrapped chart the area it asked round its own vertices' centroid, in
        // its own shape, less what the cap at its pole asks.
        let caps: f32 = groups
            .iter()
            .zip(&sizes)
            .filter(|(group, _)| group.zone == Zone::Head && group.kind == Kind::Planar)
            .map(|(_, size)| size.x * size.y)
            .sum();
        for (group, size) in groups.iter().zip(&mut sizes) {
            if group.zone == Zone::Head && group.kind == Kind::Cylindrical {
                let asked =
                    project(mesh, rig, group, beast, tail.as_ref(), None).atlas_size(config);
                let asked = asked.x * asked.y;
                *size *=
                    ((asked - caps).max(0.5 * asked) / (size.x * size.y).max(f32::EPSILON)).sqrt();
            }
        }
    }
    let charts = sizes.len();
    sizes.extend_from_slice(extra);
    let packed = pack::pack(&sizes, config.gutter);
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

/// The zone a face near a beast's leg is charted with (#376): the leg's if
/// the face is more leg than trunk and below the junction, the trunk's
/// otherwise.
///
/// A beast's upper arm and thigh lie inside its trunk, so the skin the zone
/// map gives a leg runs up over the shoulder and the flank, and wrapped round
/// a bone inside the trunk that skin - a wall beside the bone, not a tube
/// round it - came out at a third to a half of the barrel's texel density and
/// stretched two to three to one, which painted as streaked fur and smeared
/// marks in a crescent over the elbow. Charted with the trunk it wraps round
/// the spine like the barrel it is; and the skin of a leg the zone map gave
/// the trunk joins the leg's chart the same way.
///
/// Only below the junction, though. The fold over a horse's stifle, where
/// the thigh's front meets the flank and the belly, is as much leg as trunk
/// by share, but it lies above the bone the leg's chart wraps round: wrapped
/// round it, a tenth of a horse's hind leg stretched up to twelve to one.
/// A face that is neither a trunk's nor an upper leg's keeps its zone, and
/// so does every face of a body without a junction.
fn junction_zone(junctions: &Junctions, zone: Zone, centroid: Vec3) -> Zone {
    if !matches!(
        zone,
        Zone::Pelvis | Zone::Abdomen | Zone::Chest | Zone::UpperLimb(_)
    ) {
        return zone;
    }
    match junctions.leg(centroid) {
        Some(leg) if leg.share >= 0.5 && leg.depth >= 0.0 => Zone::UpperLimb(leg.limb),
        _ => junctions.girdle(zone).unwrap_or(zone),
    }
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
    /// Flattened the way its skin runs: a person's face (#473). See
    /// [`PersonHead`] and [`unfold`].
    Unfolded,
}

/// How far round `axis`, through `origin`, the corners of `face` reach: the
/// whole turn less the widest gap between them.
fn span_round(mesh: &PolyMesh, face: usize, origin: Vec3, axis: Vec3) -> f32 {
    let (tangent, binormal) = frame(axis);
    let mut turns: Vec<f32> = mesh.faces[face]
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
    std::f32::consts::TAU - widest_gap
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
/// Provenance: **derived** from the two regimes above; an orientation-only
/// test produced confetti charts.
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
fn connected_groups(
    mesh: &PolyMesh,
    rig: &Rig,
    face_zones: &[Zone],
    beast: Option<&BeastCharts>,
    tail: Option<&TailChain>,
    person: Option<&PersonHead>,
) -> Vec<Group> {
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
    let mut face_kinds: Vec<Kind> = (0..mesh.faces.len())
        .map(|face| {
            let zone = face_zones[face];
            let on_head = BeastCharts::head(beast, zone);
            // A beast's tail asks the bone beside the face, as its chart does.
            let on_tail = tail.filter(|_| zone == Zone::Tail).map(|tail| {
                let centroid = mesh.face_centroid(face);
                (centroid, tail.place(centroid))
            });
            let axis = match on_tail {
                Some((_, spot)) => spot.axis,
                None => *axis_of.entry(zone).or_insert_with(|| {
                    on_head.map_or_else(|| zone_axis(rig, zone), |head| head.axis)
                }),
            };
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
            let (centre, count) = centre_of[&zone];
            let origin = match on_tail {
                Some((centroid, spot)) => centroid - spot.flat,
                None => match (on_head, person.filter(|_| zone == Zone::Head)) {
                    (Some(head), _) => head.on_axis,
                    (None, Some(person)) => person.on_axis,
                    (None, None) => centre / count.max(1.0),
                },
            };
            if span_round(mesh, face, origin, axis) > CAP_SPAN {
                Kind::Planar
            } else {
                Kind::Cylindrical
            }
        })
        .collect();
    // A person's face is a chart of its own (#473): see [`PersonHead`].
    if let Some(person) = person {
        for (face, kind) in face_kinds.iter_mut().enumerate() {
            if face_zones[face] != Zone::Head {
                continue;
            }
            if *kind == Kind::Cylindrical && person.on_the_face(mesh.face_centroid(face)) {
                *kind = Kind::Unfolded;
            }
        }
    }

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
    let by_edge = by_edge;
    let edges = &by_edge;
    let neighbours = |face: usize| {
        let corners = &mesh.faces[face];
        (0..corners.len()).flat_map(move |corner| {
            let (a, b) = (corners[corner], corners[(corner + 1) % corners.len()]);
            edges
                .get(&(a.min(b), a.max(b)))
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|&next| next as usize)
        })
    };
    // The face is ONE island, the largest by area: a face whose centroid falls
    // inside the border's line but which touches the face only at a corner goes
    // back to the head. And a head whose largest island is no face at all - a
    // shrunken head whose zone has come apart, where the island is a scrap of
    // the refined lips - keeps the face in its wrap (see [`FACE_LEAST`]).
    if person.is_some() {
        let mut island = vec![u32::MAX; mesh.faces.len()];
        let mut areas: Vec<f32> = Vec::new();
        for start in 0..mesh.faces.len() {
            if face_kinds[start] != Kind::Unfolded || island[start] != u32::MAX {
                continue;
            }
            let label = areas.len() as u32;
            let mut queue = VecDeque::from([start]);
            island[start] = label;
            let mut area = 0.0;
            while let Some(face) = queue.pop_front() {
                area += face_area(mesh, face);
                for next in neighbours(face) {
                    if face_kinds[next] == Kind::Unfolded && island[next] == u32::MAX {
                        island[next] = label;
                        queue.push_back(next);
                    }
                }
            }
            areas.push(area);
        }
        let head: f32 = (0..mesh.faces.len())
            .filter(|&face| face_zones[face] == Zone::Head)
            .map(|face| face_area(mesh, face))
            .sum();
        let largest = (0..areas.len())
            .max_by(|&one, &two| areas[one].total_cmp(&areas[two]).then(two.cmp(&one)))
            .filter(|&label| areas[label] >= FACE_LEAST * head);
        for (face, kind) in face_kinds.iter_mut().enumerate() {
            if *kind == Kind::Unfolded && Some(island[face] as usize) != largest {
                *kind = Kind::Cylindrical;
            }
        }
        // And a head with a face of its own has a crown of its own.
        if let Some(person) = person.filter(|_| largest.is_some()) {
            for (face, kind) in face_kinds.iter_mut().enumerate() {
                if face_zones[face] == Zone::Head && person.on_the_crown(mesh.face_centroid(face)) {
                    *kind = Kind::Planar;
                }
            }
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

    if person.is_some() {
        merge_scraps(mesh, &mut groups);
    }
    groups
}

/// The largest set of `faces` joined edge to edge, and the rest of them.
fn joined(mesh: &PolyMesh, faces: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, &face) in faces.iter().enumerate() {
        let corners = &mesh.faces[face as usize];
        for corner in 0..corners.len() {
            let (a, b) = (corners[corner], corners[(corner + 1) % corners.len()]);
            by_edge.entry((a.min(b), a.max(b))).or_default().push(index);
        }
    }
    let mut label = vec![usize::MAX; faces.len()];
    let mut sizes: Vec<usize> = Vec::new();
    for first in 0..faces.len() {
        if label[first] != usize::MAX {
            continue;
        }
        let mine = sizes.len();
        label[first] = mine;
        let mut queue = VecDeque::from([first]);
        let mut size = 0;
        while let Some(index) = queue.pop_front() {
            size += 1;
            let corners = &mesh.faces[faces[index] as usize];
            for corner in 0..corners.len() {
                let (a, b) = (corners[corner], corners[(corner + 1) % corners.len()]);
                for &next in &by_edge[&(a.min(b), a.max(b))] {
                    if label[next] == usize::MAX {
                        label[next] = mine;
                        queue.push_back(next);
                    }
                }
            }
        }
        sizes.push(size);
    }
    let largest = (0..sizes.len()).max_by_key(|&one| (sizes[one], usize::MAX - one));
    let (mut solved, mut loose) = (Vec::new(), Vec::new());
    for (&face, &mine) in faces.iter().zip(&label) {
        if Some(mine) == largest {
            solved.push(face);
        } else {
            loose.push(face);
        }
    }
    (solved, loose)
}

/// Folds every scrap of a person's head - a chart of a face or two that touches
/// the rest of the head only at a corner, such as a face at the corner of the
/// jaw whose centroid falls just inside the face's border and whose edges meet
/// only the face's island and the neck - into the largest of the head's charts
/// it touches (#473). Alone it was a chart a texel or two across with a seam
/// all round it, inside a beard.
fn merge_scraps(mesh: &PolyMesh, groups: &mut Vec<Group>) {
    let area = |group: &Group| -> f32 {
        group
            .faces
            .iter()
            .map(|&face| face_area(mesh, face as usize))
            .sum()
    };
    let head: f32 = groups
        .iter()
        .filter(|group| group.zone == Zone::Head)
        .map(area)
        .sum();
    let mut from = 0;
    loop {
        let Some(scrap) = (from..groups.len()).find(|&index| {
            groups[index].zone == Zone::Head && area(&groups[index]) < SCRAP_SHARE * head
        }) else {
            return;
        };
        let corners: Vec<u32> = groups[scrap]
            .faces
            .iter()
            .flat_map(|&face| mesh.faces[face as usize].iter().copied())
            .collect();
        let host = (0..groups.len())
            .filter(|&other| {
                other != scrap
                    && groups[other].zone == Zone::Head
                    && groups[other].faces.iter().any(|&face| {
                        mesh.faces[face as usize]
                            .iter()
                            .any(|vertex| corners.contains(vertex))
                    })
            })
            .max_by(|&one, &two| area(&groups[one]).total_cmp(&area(&groups[two])));
        let Some(host) = host.filter(|&host| area(&groups[host]) >= SCRAP_SHARE * head) else {
            from = scrap + 1;
            continue;
        };
        let faces = groups.remove(scrap).faces;
        let host = if host > scrap { host - 1 } else { host };
        groups[host].faces.extend(faces);
        groups[host].faces.sort_unstable();
        from = from.min(scrap);
    }
}

/// How small a share of a person's head a chart of it may be before it is
/// folded into the chart beside it ([`merge_scraps`]). Provenance: **chosen**
/// (#473): the scraps measured are under a ten-thousandth of their heads, the
/// smallest real chart, a shrunken head's crown, a fiftieth.
const SCRAP_SHARE: f32 = 0.002;

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
    /// The chart's girth where each vertex lies along it, when the chart is
    /// unrolled by it: a beast's tail (#397). See [`unrolled`].
    radii: HashMap<u32, f32>,
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
        let u = unrolled(self.angle[&vertex], shift, self.radii.get(&vertex));
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
            Kind::Cylindrical if !self.radii.is_empty() => span * std::f32::consts::TAU,
            Kind::Cylindrical => span * std::f32::consts::TAU * self.mean_radius.max(1e-4),
            Kind::Planar | Kind::Unfolded => span,
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
fn project(
    mesh: &PolyMesh,
    rig: &Rig,
    group: &Group,
    beast: Option<&BeastCharts>,
    tail: Option<&TailChain>,
    person: Option<&PersonHead>,
) -> Projection {
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

    // A person's face, flattened the way it runs (#473).
    // And the rest of it, its crown and the band round its back, when its face
    // is a chart of its own: see [`PersonHead`].
    if let Some(person) = person.filter(|person| {
        group.kind == Kind::Unfolded
            || (group.zone == Zone::Head && (group.kind == Kind::Planar || person.charted))
    }) {
        return person.flatten(mesh, group, vertices);
    }

    let on_head = BeastCharts::head(beast, group.zone);
    // A person's head, wrapped round the head's own axis rather than round its
    // vertices' centroid (#469), and its cap at the crown flattened down it.
    let person = person.filter(|_| group.zone == Zone::Head);
    let origin = match (on_head, person) {
        (Some(head), _) => head.on_axis,
        (None, Some(person)) => person.on_axis,
        (None, None) => {
            vertices
                .iter()
                .map(|&v| mesh.positions[v as usize])
                .sum::<Vec3>()
                / vertices.len().max(1) as f32
        }
    };
    let on_tail = tail.filter(|_| group.zone == Zone::Tail);
    // A tail's cap is flattened down the bone beside it, its top up the chart.
    let tip = on_tail
        .filter(|_| group.kind == Kind::Planar)
        .map(|tail| tail.place(origin));
    let axis = match tip {
        Some(spot) => spot.axis,
        None => on_head.map_or_else(|| zone_axis(rig, group.zone), |head| head.axis),
    };
    let (tangent, binormal) = match tip {
        Some(spot) => (spot.top, spot.axis.cross(spot.top)),
        None => on_head.map_or_else(
            || frame(axis),
            |head| {
                let up = (head.up - axis * head.up.dot(axis)).normalize_or(landmark::UP);
                (up, axis.cross(up))
            },
        ),
    };
    // The rest of a tail is wrapped round its bones.
    let along_the_bones = on_tail.filter(|_| group.kind == Kind::Cylindrical);

    let mut angle = HashMap::with_capacity(vertices.len());
    let mut along = HashMap::with_capacity(vertices.len());
    let mut radius_sum = 0.0f32;
    let mut radii = HashMap::new();
    // The charts whose skin closes on their axis at an end. A neck is open at
    // both and curves away from its bone, so counting its skin as closing took its
    // worst stretch from 3 to 30 on one roll, and a pelvis and a tail gained nothing.
    let closes = matches!(group.zone, Zone::Head | Zone::Chest | Zone::Extremity(_));
    // And a person's head (#469), at its crown only within its last rings: see
    // [`PersonHead`].
    let beyond = match (beast, person) {
        (Some(_), _) if closes => Some(CLOSING),
        (None, Some(_)) => Some(CROWN_CLOSING),
        _ => None,
    };
    let skin = beyond
        .filter(|_| group.kind == Kind::Cylindrical)
        .map(|beyond| along_the_skin(mesh, &vertices, origin, axis, beyond));

    for &vertex in &vertices {
        let at = mesh.positions[vertex as usize];
        let (height, flat, tangent, binormal) = match along_the_bones {
            Some(tail) => {
                let spot = tail.place(at);
                (spot.along, spot.flat, spot.top, spot.axis.cross(spot.top))
            }
            None => {
                let offset = at - origin;
                let height = offset.dot(axis);
                (height, offset - axis * height, tangent, binormal)
            }
        };
        radius_sum += flat.length();
        if along_the_bones.is_some() {
            radii.insert(vertex, flat.length());
        }
        match group.kind {
            Kind::Cylindrical => {
                // Angle zero points forward, so the wrap — and the seam — lands
                // behind the body. On a beast's head it points at the crown, so
                // the seam lands under the chin, and on its tail at the tail's
                // top, so the seam runs along the underside.
                let turn = flat.dot(binormal).atan2(flat.dot(tangent));
                angle.insert(vertex, turn / std::f32::consts::TAU + 0.5);
                along.insert(
                    vertex,
                    skin.as_ref()
                        .map_or(height, |skin| skin.height(height, flat.length())),
                );
            }
            Kind::Planar | Kind::Unfolded => {
                // Flattened down the axis, so a cap keeps its shape.
                angle.insert(vertex, flat.dot(tangent));
                along.insert(vertex, flat.dot(binormal));
            }
        }
    }

    // A tail is unrolled by its girth where each vertex lies along it, not by
    // the vertex's own distance from the bone: a ring is not quite round, nor
    // centred on its bone where the tail leaves the rump, and unrolled vertex by
    // vertex those rings folded over.
    //
    // And past the tip the skin rounds over its end, facing along the last bone
    // with nowhere along it to go, so how far it has drawn in counts as far as
    // how far it has gone, as `AlongTheSkin` counts it on a head: counted only
    // by the way along, the last centimetre of every tail came out at a third of
    // the rest of it. Counted in as the skin goes past the tip, so the height
    // runs on across the last joint's plane without a step.
    if let Some(tail) = along_the_bones {
        let end = tail.reach[tail.reach.len() - 1];
        let rim = Girth::of(&vertices, &along, &radii)
            .at(end)
            .max(f32::EPSILON);
        for (vertex, height) in &mut along {
            let past = *height - end;
            if past > 0.0 {
                *height += (rim - radii[vertex]).max(0.0) * (past / rim).min(1.0);
            }
        }
        let girth = Girth::of(&vertices, &along, &radii);
        for (vertex, radius) in &mut radii {
            *radius = girth.at(along[vertex]);
        }
    }

    // A face straddling the seam has corners at both ends of the wrap. Lifting
    // the low ones by a full turn makes the face continuous again; the vertices
    // they refer to get duplicated when the chart is emitted.
    //
    // **Which corners are "the low ones" is decided against the face's own
    // circular mean, not against the half-turn mark** (#309). The old rule —
    // lift every corner under 0.5 whenever the span exceeds 0.5 — is the same
    // answer for a face that really crosses the seam, and the wrong one for a
    // face that merely sits near the chart's AXIS, where a few millimetres
    // move a corner's angle by a large fraction of a turn: an armpit quad of
    // the arm chart got one corner lifted a whole turn, became a sliver
    // spanning most of the chart, fell outside every filled texel and rendered
    // black — only once a swelled chest carve had nudged the chart's centroid
    // enough to tip it. Taking each corner's nearest representative to the
    // mean keeps such a face compact; it still has no good projection, but
    // it stays under its neighbours' texels. Shifts are then normalised so
    // the lowest is zero, which keeps them `+1` lifts exactly as before.
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

        let corner_shifts: Vec<u8> = if straddles {
            let (sin, cos) = face.iter().fold((0.0f32, 0.0f32), |(sin, cos), v| {
                let turn = angle[v] * std::f32::consts::TAU;
                (sin + turn.sin(), cos + turn.cos())
            });
            let mean = (sin.atan2(cos) / std::f32::consts::TAU).rem_euclid(1.0);
            let raw: Vec<i8> = face
                .iter()
                .map(|v| {
                    let off = angle[v] - mean;
                    if off < -0.5 {
                        1
                    } else if off > 0.5 {
                        -1
                    } else {
                        0
                    }
                })
                .collect();
            let floor = raw.iter().copied().min().unwrap_or(0);
            raw.iter().map(|&shift| (shift - floor) as u8).collect()
        } else {
            vec![0; face.len()]
        };
        for (corner, &shift) in face.iter().zip(&corner_shifts) {
            let u = unrolled(angle[corner], shift, radii.get(corner));
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
            u_samples.push(normalize(
                unrolled(angle[corner], shift, radii.get(corner)),
                u_range,
            ));
            v_samples.push(normalize(along[corner], v_range));
        }
    }
    // Head only: `refine_face` is the one thing in the pipeline that makes a
    // region's vertex density mean "detail lives here". Elsewhere clustering
    // is a tessellation artifact — a tapering tail keeps its ring count while
    // its radius vanishes, so its tip reads dense while carrying nothing, and
    // warping toward it re-stretched the faces `tests/uv.rs` already names as
    // the worst on the body.
    let (u_warp, v_warp) = match person {
        // A person's head follows its detail by its own measure (#469): see
        // [`PersonHead`].
        // Only a head whose face is kept in its wrap gets here (#473).
        Some(_) => (
            density_warp(&u_samples, PERSON_FOLLOW_ROUND, WARP_SLOPE),
            density_warp(&v_samples, PERSON_FOLLOW_UP, WARP_SLOPE),
        ),
        None if group.zone == Zone::Head => (
            density_warp(&u_samples, WARP_FOLLOW, WARP_SLOPE),
            density_warp(&v_samples, WARP_FOLLOW, WARP_SLOPE),
        ),
        None => (Vec::new(), Vec::new()),
    };

    Projection {
        kind: group.kind,
        angle,
        along,
        shifts,
        u_range,
        v_range,
        mean_radius: radius_sum / vertices.len().max(1) as f32,
        radii,
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

/// A cylindrical chart's `u` for a corner `angle` turns round, lifted `shift`
/// whole turns over the seam: unrolled by the chart's girth at the corner,
/// `radius`, if the chart is, so that `u` is how far round the skin it lies
/// rather than how far round the axis.
///
/// A tail tapers to a point with as many faces round its tip as round its root,
/// so taken by angle alone the tip spread across the whole width a chart sized
/// by its mean radius has, five to one on the worst tails, and the root came out
/// at a third of the barrel's density round it. Unrolled, the chart is a wedge
/// that is as fine across the tail as along it everywhere, for the corners of
/// its rectangle that nothing uses (#397).
fn unrolled(angle: f32, shift: u8, radius: Option<&f32>) -> f32 {
    let turn = angle + f32::from(shift);
    radius.map_or(turn, |radius| (turn - 0.5) * radius)
}

/// How far a chart's skin stands off its axis, by how far along it lies.
struct Girth {
    /// Where along the chart the first bin starts, and how long each is.
    start: f32,
    step: f32,
    /// The mean distance off the axis in each bin, empty bins filled from their
    /// neighbours.
    bins: Vec<f32>,
}

impl Girth {
    /// The girth of `vertices`, by how far `along` the chart each lies and how
    /// far off its axis, `radii`.
    fn of(vertices: &[u32], along: &HashMap<u32, f32>, radii: &HashMap<u32, f32>) -> Self {
        let (low, high) = along
            .values()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), &at| {
                (low.min(at), high.max(at))
            });
        let step = ((high - low) / GIRTH_BINS as f32).max(f32::EPSILON);
        let mut sums = [(0.0f32, 0.0f32); GIRTH_BINS];
        // Summed in the vertices' order rather than the map's, so a chart comes
        // out the same to the bit on every run.
        for vertex in vertices {
            let bin = (((along[vertex] - low) / step) as usize).min(GIRTH_BINS - 1);
            sums[bin].0 += radii[vertex];
            sums[bin].1 += 1.0;
        }
        let mut bins: Vec<Option<f32>> = sums
            .iter()
            .map(|&(sum, count)| (count > 0.0).then(|| sum / count))
            .collect();
        // The first bin and the last hold the chart's two ends, so an empty bin
        // has a full one either side; it takes the nearer.
        for bin in 0..GIRTH_BINS {
            if bins[bin].is_none() {
                bins[bin] = (1..GIRTH_BINS).find_map(|reach| {
                    let before = bin.checked_sub(reach).and_then(|at| bins[at]);
                    before.or_else(|| bins.get(bin + reach).copied().flatten())
                });
            }
        }
        Self {
            start: low,
            step,
            bins: bins.into_iter().map(|bin| bin.unwrap_or(0.0)).collect(),
        }
    }

    /// The girth `at` this far along, between the middles of the bins.
    fn at(&self, at: f32) -> f32 {
        let place = ((at - self.start) / self.step - 0.5).clamp(0.0, (GIRTH_BINS - 1) as f32);
        let low = (place as usize).min(GIRTH_BINS - 2);
        let between = place - low as f32;
        self.bins[low] + (self.bins[low + 1] - self.bins[low]) * between
    }
}

/// How many bins a tail's girth is read in: a few to each bone.
/// Provenance: **chosen** (#397).
const GIRTH_BINS: usize = 24;

/// A chart's height that follows its skin, rather than only the skin's way along
/// the axis (#395): where the skin faces along the axis it has no such way to go,
/// and all comes out at one height, in streaks.
struct AlongTheSkin {
    /// How far from the axis the chart's widest skin lies.
    widest: f32,
    /// Where along the axis that is.
    waist: f32,
    /// How near the axis, as a share of the widest, skin past the waist the way
    /// the axis runs has to lie before its drawing in counts: [`CLOSING`], as
    /// short of the waist, on a beast's charts, which close on their axis at
    /// both ends; [`CROWN_CLOSING`] on a person's head (#469), whose crown's rows
    /// are its face's.
    beyond: f32,
}

/// The widest of a chart's skin, about `axis` through `origin`, its drawing in
/// past the waist counted within `beyond` of the widest.
fn along_the_skin(
    mesh: &PolyMesh,
    vertices: &[u32],
    origin: Vec3,
    axis: Vec3,
    beyond: f32,
) -> AlongTheSkin {
    let read: Vec<(f32, f32)> = vertices
        .iter()
        .map(|&vertex| {
            let offset = mesh.positions[vertex as usize] - origin;
            let height = offset.dot(axis);
            (height, (offset - axis * height).length())
        })
        .collect();
    let widest = read.iter().map(|row| row.1).fold(0.0, f32::max);
    let (mut heights, mut count) = (0.0, 0.0);
    for &(height, radius) in &read {
        if radius >= WAIST_NEAR * widest {
            heights += height;
            count += 1.0;
        }
    }
    AlongTheSkin {
        widest,
        waist: heights / f32::max(count, 1.0),
        beyond,
    }
}

impl AlongTheSkin {
    /// The chart's height for skin at `height` along the axis, `radius` off it:
    /// away from the waist, how far the skin has drawn in on the way counts as far
    /// as how far it has gone.
    ///
    /// **Never short of the way the skin runs**: the two legs of a run are both
    /// counted where the true length is the hypotenuse, which spends at most four
    /// tenths too many texels on a slope and none too few, and a chart can afford
    /// that where it cannot afford a streak. Only a wall facing back toward the
    /// waist, a whisker pad's back, runs against the count.
    fn height(&self, height: f32, radius: f32) -> f32 {
        let off = height - self.waist;
        let side = (off / (WAIST_BAND * self.widest).max(f32::EPSILON)).clamp(-1.0, 1.0);
        let closing = if side > 0.0 { self.beyond } else { CLOSING };
        height + side * (closing * self.widest - radius).max(0.0)
    }
}

/// How near the axis, as a share of the widest, skin has to lie before it counts
/// as closing on the axis: the walls of a body's own section stand further off
/// than this, and counting those shears the whole chart for nothing.
/// Provenance: **tuned against the charts' stretch** (#395).
const CLOSING: f32 = 0.6;

/// How near a person's head's axis, as a share of its widest, skin above the
/// waist has to lie before its drawing in counts (#469): the crown's last few
/// rings and no more. The rows a crown spreads over are the face's rows too, and
/// counting the whole vault, as [`CLOSING`] does under the jaw, took a tenth of
/// the face's and the eyes' rows; at this share the scalp's rows at the crown
/// close from 2.8 to 2.2 mm apart (their 90th percentile) for a tenth of a
/// millimetre on the face's.
/// Provenance: **measured** on the rolls and corners of #469.
const CROWN_CLOSING: f32 = 0.3;

/// How near the widest a vertex has to lie, as a share of it, to say where the
/// chart's waist is. Provenance: **chosen** (#395): a ring's worth, not one vertex.
const WAIST_NEAR: f32 = 0.9;

/// Over how much of the axis, either side of the waist and in the chart's widest
/// radii, drawing in goes from not counting to counting whole: a step at the waist
/// would be a seam across the chart, on skin that is not quite the widest.
/// Provenance: **chosen** (#395).
const WAIST_BAND: f32 = 0.5;

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
/// Provenance: **tuned by render**, against the lower face.
const WARP_FOLLOW: f32 = 0.5;

/// A monotone remap of a chart axis toward the mesh's own vertex density.
///
/// Spread uniformly over a chart's extent, texels miss where the DETAIL is:
/// the head chart spends four refinement passes on the mouth band, so the
/// lower face carries most of the chart's faces in a tenth of its height —
/// and would get a tenth of its texels. Painted at that density the jaw
/// flank comes out as magnified single-texel rectangles. The atlas is
/// per-texel positional — every painter is a function of `texel.position` — so
/// warping where the texels go changes only how finely each region is
/// sampled, never what is painted there.
///
/// Returns piecewise-linear knots mapping normalized `[0, 1]` onto itself:
/// the corner-count CDF, each bin held within `slope` of its uniform share,
/// blended `follow` of the way from identity - [`WARP_FOLLOW`] and
/// [`WARP_SLOPE`], or a person's head's own (#469).
/// Charts with fewer corners than fill its bins keep the identity — a CDF
/// built from a handful of samples is noise, and the charts that small are
/// exactly the ones a warp cannot help.
fn density_warp(samples: &[f32], follow: f32, slope: f32) -> Vec<f32> {
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
        *count = count.clamp(uniform_share / slope, uniform_share * slope);
    }
    let total: f32 = counts.iter().sum();
    let mut knots = Vec::with_capacity(WARP_BINS + 1);
    let mut running = 0.0;
    knots.push(0.0);
    for (bin, &count) in counts.iter().enumerate() {
        running += count / total;
        let uniform = (bin + 1) as f32 / WARP_BINS as f32;
        knots.push(uniform + (running - uniform) * follow);
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

/// How far the columns of a person's head chart follow its detail round the
/// head when its face is kept in the wrap ([`FACE_LEAST`]), in place of
/// [`WARP_FOLLOW`] (#469, #473): 0.24.5's figure for every person's head.
///
/// Round the head joint every part of the head has its share of the way round,
/// and a face in the wrap takes its extra columns from this alone. At
/// [`WARP_FOLLOW`]'s half the mouth was a fifth coarser round the head than it
/// had been; at seven tenths the face's and the eyes' coarser step is finer
/// than it was and the mouth's within a tenth of it.
/// Provenance: **measured** on the rolls and corners of #469.
const PERSON_FOLLOW_ROUND: f32 = 0.7;

/// How far the rows of a person's head chart, round the joint, follow its
/// detail up the head, in place of [`WARP_FOLLOW`] (#469, #473).
///
/// Its columns follow nothing: they did, when the face was in the chart and
/// took its extra columns from the warp, but with the face a chart of its own
/// the corners crowd only where the face's finer faces spill past its border,
/// and following them took the back of the head's columns for it - the scalp's
/// median 3.25 mm a texel against 3.13 uniform, its ninetieth 4.28 against
/// 3.86. Its rows still close at the crown by following the rings there, the
/// crown's median 2.71 mm a texel against 3.37 uniform and its ninetieth 3.66
/// against 5.11, for nothing at the nape.
/// Provenance: **measured** on the rolls and corners of #469, and on the five
/// heads of `tests/uv.rs` for #473.
const PERSON_FOLLOW_UP: f32 = 0.3;

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
    let joints = rig.in_zone(zone);
    let Some(&first) = joints.first() else {
        return landmark::UP;
    };
    // A beast's upper leg starts inside its trunk (#376): its first bones are
    // markers, and its chart is the skin below its first joint outside - the
    // elbow, the stifle - wrapped round the bone leaving that joint.
    if rig.joints[first].marker
        && let Some(&outside) = joints.iter().find(|&&joint| !rig.joints[joint].marker)
        && let Some(below) = rig
            .joints
            .iter()
            .find(|joint| joint.parent == Some(outside))
    {
        let leaving = below.position - rig.joints[outside].position;
        if leaving.length_squared() > 1e-12 {
            return leaving.normalize();
        }
    }
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
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
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
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let uv = unwrap(&mesh, &rig, &[Zone::Head], &UvConfig::default());
        assert_eq!(uv, UvUnwrap::default());
    }
}
