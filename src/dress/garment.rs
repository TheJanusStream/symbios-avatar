//! Tight clothing, cut from the body's own surface.
//!
//! A close-fitting garment is the body re-evaluated a few millimetres outside
//! itself. Building it that way rather than modelling it separately buys three
//! things at once, and each of them is a problem that clothing systems usually
//! have to solve the hard way:
//!
//! - **It cannot intersect the body**, because every point of it is a point of
//!   the body pushed outward along its own normal. No collision pass, no
//!   push-out solver, nothing to tune.
//! - **It deforms for free.** Each garment vertex comes from a body vertex, so
//!   it inherits that vertex's skin weights exactly. A shirt bends where the
//!   elbow bends because it is made of elbow.
//! - **It fits every body.** The parameters that made the body already made the
//!   garment; nothing has to be re-cut when a wearer is taller or broader.
//!
//! The result is a closed solid — an outer shell, an inner shell just inside the
//! skin, and a rim joining them at every hem. Cloth has two sides and a cut edge,
//! and a single offset surface has none of those: seen from underneath it
//! vanishes, and at the hem it reads as a sticker.

use std::collections::HashMap;

use glam::{Vec2, Vec3};

use super::surface::GarmentTexture;
use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone, ZoneSet};
use crate::rig::{Influence, MAX_INFLUENCES, Rig, SkinWeights};
use crate::uv::UvUnwrap;

/// How a garment is cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GarmentCut {
    /// Which parts of the body it covers.
    pub zones: ZoneSet,
    /// Parts it may spill into, but not claim on their own.
    ///
    /// A face is taken when every corner is in `zones` or `reach` and at least
    /// one is in `zones`. Without this, two garments meeting at the waist leave
    /// a ring of bare skin between them: the faces that straddle the seam are
    /// wholly inside neither, so neither takes them. Giving the lower garment a
    /// reach into the upper one's zone hands that ring to exactly one of them.
    pub reach: ZoneSet,
    /// Limbs it runs down part of the way: the limb, and how far, as a share
    /// of the limb — `0` its root, `½` its middle joint, `1` its end.
    ///
    /// **A zone is the wrong unit for a hem** (#314). Shorts claimed the
    /// pelvis and nothing else, so they ended exactly where the thigh's bone
    /// begins to win the skin — at the crotch — and "to just below the hip"
    /// left the crotch bare. A hem wants to sit a share of the way down a
    /// limb, and a share of a bone's length is a ring square to the bone,
    /// which is a cleaner hem than any zone boundary: those wander vertex by
    /// vertex wherever two bones hold the skin almost equally.
    ///
    /// **The whole limb, not one of its zones** (#356). Taking the upper zone
    /// whole and a share of the lower one's bone put a jump in the middle of
    /// the range: the upper zone's skin runs past the elbow, so a hem moving
    /// down through the elbow leapt a third of the forearm. Measured as a
    /// position along the limb's two principal bones instead, every corner of
    /// either zone has one place on one continuous axis, and the hem moves as
    /// smoothly as the length does. See [`Chain`].
    ///
    /// A share of `1` or more takes both zones whole, which is exactly what
    /// the old wrist and ankle cuts took. A share of `0` or less takes none of
    /// the limb, and a cut that wants none should leave the slot `None`.
    ///
    /// Two slots, one per side of a pair of limbs; a cut with fewer leaves
    /// the rest `None`.
    pub limbs: [Option<(Limb, f32)>; 2],
    /// How far its outer face stands off the skin, in metres.
    pub thickness: f32,
    /// How far its inner face sits *inside* the skin, in metres.
    ///
    /// Not zero. Coincident surfaces flicker against each other wherever depth
    /// precision runs out, and the inner face of a garment is coincident with
    /// the body over its whole area — the worst possible case for it.
    pub bite: f32,
}

impl Default for GarmentCut {
    fn default() -> Self {
        Self {
            limbs: [None; 2],
            zones: ZoneSet::default(),
            reach: ZoneSet::default(),
            thickness: 0.008,
            bite: 0.0015,
        }
    }
}

/// One piece of close-fitting clothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Garment {
    /// Its geometry, in body space, carrying the body's skin weights and colour.
    pub mesh: PolyMesh,
    /// The body vertex each garment vertex was cut from.
    ///
    /// Twice as long as the body's contribution, because the outer and inner
    /// shells are cut from the same points. This is the one lookup that gives a
    /// garment texture coordinates: it was made of body, so it is charted where
    /// the body is charted, and nothing has to unwrap it separately.
    pub source: Vec<u32>,
    /// The body faces this garment was cut from, in ascending order.
    ///
    /// The other half of [`source`](Self::source): that one says which body
    /// *vertex* each garment vertex came from, this one which body *faces* the
    /// garment stands over. It is what lets the body stop emitting the skin
    /// underneath — a claimed face is enclosed by this solid on every side, the
    /// rim included, so nothing can ever see it (`Avatar::charted_body`).
    pub claim: Vec<u32>,
    /// The body faces this garment hides, a subset of [`claim`](Self::claim).
    ///
    /// The claim minus the faces the garment cannot prove it stands over, and
    /// there are two of those. Most of the difference is `smooth_hem`'s bill: a
    /// hem free to leave the face boundaries it was cut along may retreat
    /// inside the face it crosses, and the skin there then has to be drawn;
    /// only the faces further in are covered whatever the hem does. Measured at
    /// about 274 triangles of the 1,490 a default outfit covers
    /// (`examples/garmentaudit`).
    ///
    /// The rest is `stood_off`'s: a column that had to be moved off its own
    /// normal to leave the body is a column that no longer stands over the
    /// faces it was cut from. About four faces per body, all at the crotch
    /// (#279).
    ///
    /// This is the set [`Outfit::covered`](crate::Outfit::covered) unions, and
    /// therefore the set the body does not emit.
    pub hidden: Vec<u32>,
    /// Its hem, as closed loops of its own outer-shell vertices.
    ///
    /// The cut edge as delivered — after `smooth_hem` has moved it — which is
    /// the only place it can be measured. The claim's boundary on the body says
    /// where the hem was *cut*, and the two differ by up to half a face, which
    /// is the whole of what that pass does. The rim quads run along these loops.
    ///
    /// Outer-shell vertices; the inner shell's twin of a column is that column
    /// plus half the vertex count.
    pub hem: Vec<Vec<u32>>,
    /// The body face and the corner of it that every corner of every face of
    /// [`mesh`](Self::mesh) was cut from, face for face.
    ///
    /// What charts a garment exactly. [`source`](Self::source) names a body
    /// VERTEX, and a body vertex on a chart boundary sits in the atlas more
    /// than once, so a garment charted by vertex takes whichever copy came
    /// first and a face across the boundary spans the atlas — harmless while
    /// a garment was one flat colour, and a smear across the shirt the moment
    /// it is textured. A corner names the face it belongs to, and a face is in
    /// exactly one chart. See [`Garment::charted`].
    pub corner_source: Vec<Vec<(u32, u8)>>,
    /// The limbs it runs down, the ones its cut named.
    ///
    /// What the cloth atlas lays a sleeve or a trouser leg's panel along; a
    /// garment wraps its trunk panel whatever this holds.
    pub limbs: Vec<Limb>,
    /// The colour it should be shaded.
    pub colour: [f32; 3],
    /// What it is made of, if not plain dyed cloth.
    pub texture: Option<GarmentTexture>,
}

/// How close to one of a zone's bones, in that bone's radii, a corner has to
/// lie to count as ON that part whatever the zone map says.
///
/// **The zone map is nearest-bone ownership, and a hand hanging beside a hip
/// owns the hip** (#314): on the masculine frame the arms fall close enough
/// that the skin over the hips reads `Extremity(ForeLeft)`, and a pair of
/// shorts claiming the pelvis by zone came out as scattered patches with
/// bare skin between. The skin of a part sits between two thirds and one
/// radius out from its bone; a hand beside it sits a hand's thickness
/// further. Just over one radius takes the part's own skin and not the
/// hand's. Consulted only for skin the map gave to a limb — see
/// [`claimed`] — so it can never move trunk skin between trunk garments.
const ON_BONE: f32 = 1.05;

/// Which faces `cut` claims outright, one flag per face of `mesh`.
///
/// A face is claimed only if *every* corner of it is in the cut's zones or
/// reach. Taking faces on a majority instead leaves the hem straddling the
/// boundary, which puts a ragged edge halfway into the neighbouring zone.
///
/// A corner is "in" a zone when the zone map says so — or, for skin the map
/// gave to a limb, when the bone it is nearest in that bone's own radii is
/// the zone's, within `ON_BONE`. A corner of one of the cut's
/// [`limbs`](GarmentCut::limbs) counts only up to that limb's share, measured
/// as its position along the limb's [`Chain`], so the hem is a ring square to
/// the limb wherever it lands.
#[must_use]
pub fn claimed(mesh: &PolyMesh, rig: &Rig, zones: &[Zone], cut: &GarmentCut) -> Vec<bool> {
    let zone_of = |corner: u32| zones.get(corner as usize).copied();
    // Each named limb's reach, with its chain. A limb this body has no
    // chain for — no bone in one of its zones — is taken whole: the zones
    // are all a cut can go by there, and they are what the old cuts took.
    let limbs: Vec<(Limb, f32, Option<Chain>)> = cut
        .limbs
        .iter()
        .flatten()
        .filter(|(_, reach)| *reach > 0.0)
        .map(|&(limb, reach)| (limb, reach, Chain::of(rig, limb)))
        .collect();
    // Where a corner the map gave to a limb really lies: the zone of the bone
    // it is nearest in that bone's OWN radii, the mapped limb's bones set
    // aside, if that is within [`ON_BONE`] — one zone, so the hand-held hip
    // skin goes to the pelvis or the thigh and never to both the pelvis and
    // the abdomen at once (a fat trunk bone reaches a long way by radius).
    let rescued = |corner: u32, mapped: Zone| -> Option<Zone> {
        let position = mesh.positions[corner as usize];
        // Skin that really is on its mapped limb keeps its zone: the short
        // bone across the hip is the upper leg's, and its skin is the hip's.
        let mut own = f32::INFINITY;
        let mut best: Option<(f32, Zone)> = None;
        for joint in rig.surfaced() {
            let zone = rig.joints[joint].zone;
            let (start, end) = rig.bone(joint);
            let axis = end - start;
            let along = if axis.length_squared() <= f32::EPSILON {
                0.0
            } else {
                ((position - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
            };
            let (near, far) = rig.bone_radii(joint);
            let radius = (near + (far - near) * along).max(f32::EPSILON);
            let radii = position.distance(start + axis * along) / radius;
            if zone.limb().is_some() && zone.limb() == mapped.limb() {
                own = own.min(radii);
            } else if best.is_none_or(|(held, _)| radii < held) {
                best = Some((radii, zone));
            }
        }
        if own <= ON_BONE {
            return Some(mapped);
        }
        best.filter(|(radii, _)| *radii <= ON_BONE)
            .map(|(_, zone)| zone)
    };
    // The rescue applies only to skin the map gave to a LIMB — the hand
    // beside the hip — and never moves trunk skin between trunk zones: the
    // trunk's bones are fat, and by radius alone the top claimed down the
    // hips and the trousers up the belly, interleaved at the waist.
    let member = |corner: u32, zone: Zone| match zone_of(corner) {
        Some(mapped) if mapped == zone => true,
        Some(mapped @ (Zone::UpperLimb(_) | Zone::LowerLimb(_) | Zone::Extremity(_))) => {
            rescued(corner, mapped) == Some(zone)
        }
        _ => false,
    };
    let in_zones = |corner: u32| cut.zones.iter().any(|zone| member(corner, zone));
    let in_reach = |corner: u32| cut.reach.iter().any(|zone| member(corner, zone));
    // Which of the cut's limbs a corner is skin of, by its index in `limbs`.
    let limb_of = |corner: u32| {
        limbs.iter().position(|(limb, _, _)| {
            member(corner, Zone::UpperLimb(*limb)) || member(corner, Zone::LowerLimb(*limb))
        })
    };
    mesh.faces
        .iter()
        .map(|face| {
            // Every corner somewhere the cut may go, and at least one where it
            // goes on its own; a limb's corners gathered by limb, with where
            // along it each lies.
            let mut anchored = false;
            let mut along: Vec<(usize, f32)> = Vec::new();
            for &corner in face {
                if in_zones(corner) {
                    anchored = true;
                } else if let Some(index) = limb_of(corner) {
                    anchored = true;
                    let (_, reach, chain) = &limbs[index];
                    if let Some(chain) = chain.filter(|_| *reach < 1.0) {
                        along.push((index, chain.position(mesh.positions[corner as usize])));
                    }
                } else if !in_reach(corner) {
                    return false;
                }
            }
            // **Rounded to the nearer ring, not cut at it** (#356). A limb's
            // face is taken when the middle of its span along the limb is
            // within reach — so the claim's hem lands on whichever ring is
            // nearer the length, and `place_hems` has at most half a row to
            // slide it the rest of the way. A limb carries only four rows or
            // so per segment, and a hem that could only sit on a ring moved
            // in jumps of a sixth of the limb.
            anchored
                && limbs.iter().enumerate().all(|(index, (_, reach, _))| {
                    let (low, high) = along.iter().filter(|(limb, _)| *limb == index).fold(
                        (f32::INFINITY, f32::NEG_INFINITY),
                        |(low, high), &(_, at)| (low.min(at), high.max(at)),
                    );
                    low > high || 0.5 * (low + high) <= *reach
                })
        })
        .collect()
}

/// A limb as one axis: its upper and lower principal bones, end to end.
///
/// The axis a hem is placed on (#356). A position on it is a share of the
/// limb with its middle joint at `½` — `0` the shoulder or hip, `1` the wrist
/// or ankle — so a length names the same landmark on every body, and each
/// half moves along its own bone at the rate that bone's length sets.
///
/// **Each principal bone is its zone's longest**, the rule the old
/// per-zone share used: the thigh of an upper leg rather than the short bone
/// that crosses the hip, whose skin sits above the thigh's start and so at the
/// very top of the axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chain {
    /// The upper segment, root to middle joint.
    pub upper: (Vec3, Vec3),
    /// The lower segment, middle joint to end.
    pub lower: (Vec3, Vec3),
}

impl Chain {
    /// The chain of `limb`, if this body has a bone in both of its zones.
    #[must_use]
    pub fn of(rig: &Rig, limb: Limb) -> Option<Self> {
        let principal = |zone: Zone| {
            rig.in_zone(zone)
                .into_iter()
                .filter(|&joint| rig.joints[joint].role.deforms() && !rig.joints[joint].marker)
                .max_by(|&a, &b| {
                    let (sa, ea) = rig.bone(a);
                    let (sb, eb) = rig.bone(b);
                    sa.distance(ea).total_cmp(&sb.distance(eb))
                })
                .map(|joint| rig.bone(joint))
        };
        Some(Self {
            upper: principal(Zone::UpperLimb(limb))?,
            lower: principal(Zone::LowerLimb(limb))?,
        })
    }

    /// The point on the limb's axis at `position` along it, `0` the root and
    /// `1` the end, the middle joint at `½`.
    #[must_use]
    pub fn point(&self, position: f32) -> Vec3 {
        let (segment, share) = if position <= 0.5 {
            (self.upper, 2.0 * position)
        } else {
            (self.lower, 2.0 * position - 1.0)
        };
        segment.0 + (segment.1 - segment.0) * share.clamp(0.0, 1.0)
    }

    /// Where along the limb `point` lies, `0` at the root and `1` at the end.
    ///
    /// Taken on whichever segment the point is nearer, as that segment's own
    /// share, clamped to it: near the middle joint the two segments' answers
    /// meet at `½`, and skin past the root — a shoulder's cap, the hip's —
    /// reads as the root itself.
    #[must_use]
    pub fn position(&self, point: Vec3) -> f32 {
        let (upper_distance, upper_share) = along(self.upper, point);
        let (lower_distance, lower_share) = along(self.lower, point);
        if upper_distance <= lower_distance {
            0.5 * upper_share
        } else {
            0.5 + 0.5 * lower_share
        }
    }
}

/// How far `point` is from a segment, and where along it the nearest spot
/// lies as a share clamped to the segment.
fn along((start, end): (Vec3, Vec3), point: Vec3) -> (f32, f32) {
    let axis = end - start;
    let share = if axis.length_squared() <= f32::EPSILON {
        0.0
    } else {
        ((point - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
    };
    (point.distance(start + axis * share), share)
}

/// Fills the notches in a claim's hem, in place.
///
/// The zone boundary the claim follows is a nearest-bone classification, and
/// where two bones hold the surface almost equally it wanders vertex by vertex
/// across a ring — so the raw hem is a zigzag of one-face bites out of the
/// garment and one-face spurs of it into bare skin: a neck change
/// small enough to be invisible re-cuts the collar into sawteeth. Rather than
/// tune geometry until the boundary happens to land between rings, the hem is
/// smoothed here, where it is made: any unclaimed face sharing two or more
/// edges with claimed faces is a notch, and is claimed too, to a fixed point.
/// A face past a straight hem touches it on one edge only, so the fill climbs
/// concavities and never advances a straight front.
///
/// Filling is the only direction that is safe: a garment face is body pushed
/// outward, so an extra face lies over skin, while *shaving* a spur would open
/// a hole in anything that counts on the raw claim. `blocked` marks faces some
/// other garment holds — a notch is never filled across a seam, or the two
/// garments would claim the same face and flicker (`the_top_and_the_trousers_
/// do_not_claim_the_same_face`).
pub fn close(mesh: &PolyMesh, mine: &mut [bool], blocked: &[bool]) {
    let mut users: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        for at in 0..face.len() {
            let key = edge_key(face[at], face[(at + 1) % face.len()]);
            users.entry(key).or_default().push(index);
        }
    }
    loop {
        let mut filled = false;
        for index in 0..mesh.faces.len() {
            if mine[index] || blocked.get(index).copied().unwrap_or(false) {
                continue;
            }
            let face = &mesh.faces[index];
            let holding = (0..face.len())
                .filter(|&at| {
                    let key = edge_key(face[at], face[(at + 1) % face.len()]);
                    users[&key]
                        .iter()
                        .any(|&other| other != index && mine[other])
                })
                .count();
            if holding >= 2 {
                mine[index] = true;
                filled = true;
            }
        }
        if !filled {
            break;
        }
    }
}

impl Garment {
    /// Cuts a garment from a body.
    ///
    /// `zones` says which vertex belongs to which part of the body — the same
    /// map the unwrap uses. Returns `None` when the cut covers nothing.
    ///
    /// This is [`claimed`], [`close`], and [`sew`](Self::sew) in one call, for
    /// a garment worn alone. Garments that share a body go through the steps
    /// separately — see [`Outfit::wear`](crate::dress::Outfit::wear) — because
    /// each one's fill has to know what the others hold.
    #[must_use]
    pub fn cut(
        mesh: &PolyMesh,
        rig: &Rig,
        weights: &SkinWeights,
        zones: &[Zone],
        cut: &GarmentCut,
        colour: [f32; 3],
    ) -> Option<Self> {
        let mut mine = claimed(mesh, rig, zones, cut);
        close(mesh, &mut mine, &[]);
        Self::sew(mesh, rig, weights, zones, &mine, cut, colour)
    }

    /// Builds the garment solid over the faces `mine` marks.
    ///
    /// `rig` and `zones` are what places a limb's hem where its length says
    /// (#356): the claim puts the hem on the ring nearest the length, and the
    /// hem's columns are slid along the limb the rest of the way.
    ///
    /// Returns `None` when the claim covers nothing.
    #[must_use]
    pub fn sew(
        mesh: &PolyMesh,
        rig: &Rig,
        weights: &SkinWeights,
        zones: &[Zone],
        mine: &[bool],
        cut: &GarmentCut,
        colour: [f32; 3],
    ) -> Option<Self> {
        let covered: Vec<&Vec<u32>> = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|&(index, _)| mine.get(index).copied().unwrap_or(false))
            .map(|(_, face)| face)
            .collect();
        if covered.is_empty() {
            return None;
        }

        let normals = mesh.vertex_normals();

        // Only the vertices the cut actually uses, renumbered. Carrying the
        // body's whole vertex list would leave a garment mostly made of points
        // no face refers to, which every downstream tool then has to ignore.
        //
        // **One garment vertex per FAN, not per body vertex, and that is what
        // makes a pinched hem a closed solid** (#105). Where the covered
        // region's boundary touches itself — the top of a pair of shorts, all
        // the way round the waist — a single body vertex carries two separate
        // runs of covered faces and four boundary edges. Sharing one column
        // between them puts four rim quads on the side edge that column and its
        // inner twin span, and four faces on an edge is not a solid. Splitting
        // the vertex once per fan gives each boundary loop its own column and
        // its own side edge, and the two loops stop being one figure-eight.
        //
        // Measured before the split: the shorts came back with 12 nonmanifold
        // edges on ten of twelve seeds, always a multiple of four because each
        // pinch vertex contributes exactly one. Identical across all three
        // sleeve lengths, which is what ruled the sleeve out.
        let fan = fans(&covered);
        let mut source: Vec<u32> = Vec::new();
        // `corner_at[face][position]` is the garment vertex that corner uses.
        let mut corner_at: Vec<Vec<u32>> = Vec::with_capacity(covered.len());
        let mut column: HashMap<usize, u32> = HashMap::new();
        for (index, face) in covered.iter().enumerate() {
            let mut row = Vec::with_capacity(face.len());
            for at in 0..face.len() {
                let root = fan.root(index, at);
                let next = *column.entry(root).or_insert_with(|| {
                    source.push(face[at]);
                    source.len() as u32 - 1
                });
                row.push(next);
            }
            corner_at.push(row);
        }

        let hem = hem_edges(&covered);

        // Where each column stands, before the hem is smoothed. Held as its own
        // list rather than read out of the body twice, because the hem's
        // columns are about to stop standing over the vertex they were cut
        // from.
        let mut at: Vec<Vec3> = source
            .iter()
            .map(|&from| mesh.positions[from as usize])
            .collect();
        let rings: Vec<Vec<u32>> = hem_walk(&covered, &fan, &hem)
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|&(face, corner)| corner_at[face][corner])
                    .collect()
            })
            .collect();
        let near = Neighbourhood::of(mesh, &rings, &source);
        smooth_hem(mesh, &near, &rings, &source, &mut at);
        place_hems(mesh, rig, zones, cut, &near, &rings, &source, &mut at);

        // Where each column's outer shell stands. The body vertex pushed along
        // its own normal wherever that leaves the body, and a point measured
        // against the body wherever it does not — see [`stood_off`], which is
        // the whole of #279.
        let outer: Vec<Vec3> = source
            .iter()
            .enumerate()
            .map(|(column, &from)| {
                stood_off(mesh, at[column], normals[from as usize], cut.thickness)
            })
            .collect();

        let mut garment = PolyMesh::new();
        for &point in &outer {
            garment.push_vertex(point);
        }
        let inner_base = garment.vertex_count() as u32;
        // The inner shell takes the heading the outer one ended up with, not the
        // normal it started from. A normal that put the outer shell inside the
        // skin had the inner shell OUTSIDE it by the same mistake, and a garment
        // whose two shells are swapped is inside out where it matters most.
        for (column, &from) in source.iter().enumerate() {
            let heading = (outer[column] - at[column])
                .try_normalize()
                .unwrap_or(normals[from as usize]);
            garment.push_vertex(at[column] - heading * cut.bite);
        }

        // The body faces covered, in the order `covered` holds them: both are
        // the claim read in face order, so `claim[i]` is `covered[i]`'s face.
        let claim: Vec<u32> = (0..mesh.faces.len() as u32)
            .filter(|&face| mine.get(face as usize).copied().unwrap_or(false))
            .collect();

        let mut corner_source: Vec<Vec<(u32, u8)>> =
            Vec::with_capacity(corner_at.len() * 2 + hem.len());
        for (row, &body_face) in corner_at.iter().zip(&claim) {
            let outer: Vec<u32> = row.clone();
            // The inner shell faces the other way, so its winding is reversed.
            let inner: Vec<u32> = outer.iter().rev().map(|&v| v + inner_base).collect();
            let corners = row.len();
            corner_source.push(
                (0..corners)
                    .map(|corner| (body_face, corner as u8))
                    .collect(),
            );
            corner_source.push(
                (0..corners)
                    .rev()
                    .map(|corner| (body_face, corner as u8))
                    .collect(),
            );
            garment.push_face(outer);
            garment.push_face(inner);
        }

        for &(face, at) in &hem {
            let row = &corner_at[face];
            let next = (at + 1) % row.len();
            let (a, b) = (row[at], row[next]);
            // Reversed against the outer shell's use of the same edge. Every
            // edge of a closed solid is traversed once each way; the rim shares
            // one edge with the outer shell and one with the inner, so it must
            // run against both of them.
            garment.push_face([b, a, a + inner_base, b + inner_base]);
            // Both shells' ends of a rim are cut from the edge's two corners.
            let (from, to) = ((claim[face], at as u8), (claim[face], next as u8));
            corner_source.push(vec![to, from, from, to]);
        }
        // What the garment hides: the claim, less the faces it can no longer
        // prove it encloses. Two things forfeit that proof, and the second was
        // measured rather than reasoned about (#279).
        //
        // - **The row the hem runs through**, because the hem no longer runs
        //   along their edges after `smooth_hem` slid it.
        // - **Any face a repaired column touches.** Suppression rests on every
        //   garment point being a body point pushed along its own normal, so
        //   the solid stands over the faces it was cut from; a column
        //   [`stood_off`] had to move is exactly a column where that stopped
        //   being true. Measured before it was assumed: with the crotch columns
        //   repaired and this filter absent, four hidden faces of seed 9 have
        //   no cloth over them at all — by their own face normal as much as by
        //   the mean of their corners', so it is the cover that went and not
        //   the ray that asks. It is about four faces of nine and a half
        //   thousand per body, and they were only ever covered by cloth that
        //   was inside the skin.
        let mut forfeits = vec![false; mesh.vertex_count()];
        for ring in &rings {
            for &column in ring {
                forfeits[source[column as usize] as usize] = true;
            }
        }
        for (column, &from) in source.iter().enumerate() {
            if outer[column] != at[column] + normals[from as usize] * cut.thickness {
                forfeits[from as usize] = true;
            }
        }
        let hidden: Vec<u32> = claim
            .iter()
            .copied()
            .filter(|&face| {
                !mesh.faces[face as usize]
                    .iter()
                    .any(|&corner| forfeits[corner as usize])
            })
            .collect();

        // Weights come straight from the body vertex each point was made from,
        // which is the whole reason a garment cut this way needs no rigging.
        // Both shells were cut from the same points, hence the doubling.
        let cut_from: Vec<u32> = source.iter().chain(&source).copied().collect();
        garment.set_skin(
            cut_from
                .iter()
                .map(|&from| borrowed(weights, from as usize))
                .collect(),
        );
        garment.paint(Vec3::from_array(colour));

        Some(Self {
            mesh: garment,
            source: cut_from,
            claim,
            hidden,
            hem: rings,
            corner_source,
            limbs: cut
                .limbs
                .iter()
                .flatten()
                .filter(|(_, reach)| *reach > 0.0)
                .map(|&(limb, _)| limb)
                .collect(),
            colour,
            texture: None,
        })
    }

    /// How many vertices the garment carries.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
    }

    /// Charts the garment against the body's atlas, one coordinate per
    /// vertex.
    ///
    /// `body_uvs` is one coordinate per *body* vertex — the unwrap's first copy
    /// of each. **An approximation**, and one that is wrong on every face that
    /// crosses a chart boundary, where the first copy can belong to the chart
    /// next door; it is kept because it is what a garment's own
    /// [`mesh`](Self::mesh) can carry without being split. What is drawn goes
    /// through [`Self::charted`], which is exact.
    pub fn chart(&mut self, body_uvs: &[Vec2]) {
        self.mesh.set_uvs(
            self.source
                .iter()
                .map(|&from| body_uvs.get(from as usize).copied().unwrap_or(Vec2::ZERO))
                .collect(),
        );
    }

    /// The garment split along the body's chart seams, every corner charted
    /// where the body face it was cut from is charted.
    ///
    /// The mesh to DRAW a garment with once anything samples its atlas: a
    /// vertex carries one coordinate, a body vertex on a seam carries several,
    /// so a garment vertex is copied once per chart it is cut into — the same
    /// split the body's own draw mesh takes. Positions, normals, weights and
    /// colours follow each copy unchanged; normals are taken over the unsplit
    /// garment first, so the seams stay smooth.
    ///
    /// `face_of` maps a body face to its face in `unwrap`, as
    /// [`charted_faces`] builds it. A corner whose body face is not in the
    /// unwrap keeps the per-vertex coordinate [`Self::chart`] gave it.
    ///
    /// A rim is charted [`RIM_INSET`] of the way into the face its hem edge
    /// bounds rather than on the edge itself — see there.
    #[must_use]
    pub fn charted(&self, unwrap: &UvUnwrap, face_of: &[Option<u32>]) -> PolyMesh {
        let normals = if self.mesh.normals.len() == self.mesh.vertex_count() {
            self.mesh.normals.clone()
        } else {
            self.mesh.vertex_normals()
        };
        let mut copies: HashMap<(u32, u32), u32> = HashMap::new();
        // A rim's corners are keyed by the face they were cut from as well:
        // one vertex is drawn into a different face beside each rim it joins.
        let mut rim_copies: HashMap<(u32, u32, u8), u32> = HashMap::new();
        // Each covered face pushed its outer and its inner face; the rims
        // come after all of them.
        let rims = self.claim.len() * 2;
        let mut mesh = PolyMesh::new();
        let (mut uvs, mut normal_of, mut skin, mut colours) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut copy = |at: usize, uv: Vec2| {
            uvs.push(uv);
            normal_of.push(normals[at]);
            if let Some(influences) = self.mesh.skin.get(at) {
                skin.push(*influences);
            }
            if let Some(colour) = self.mesh.colours.get(at) {
                colours.push(*colour);
            }
            mesh.push_vertex(self.mesh.positions[at])
        };
        let mut faces = Vec::with_capacity(self.mesh.faces.len());
        for (index, (face, corners)) in self.mesh.faces.iter().zip(&self.corner_source).enumerate()
        {
            let mut charted = Vec::with_capacity(face.len());
            for (&vertex, &(body_face, corner)) in face.iter().zip(corners) {
                let ring = face_of
                    .get(body_face as usize)
                    .copied()
                    .flatten()
                    .and_then(|index| unwrap.faces.get(index as usize));
                let unwrapped = ring.and_then(|corners| corners.get(usize::from(corner)).copied());
                let at = vertex as usize;
                let inset = ring
                    .zip(unwrapped)
                    .filter(|_| index >= rims)
                    .and_then(|(ring, unwrapped)| rim_uv(unwrap, ring, unwrapped));
                let charted_at = match inset {
                    Some(uv) => *rim_copies
                        .entry((vertex, body_face, corner))
                        .or_insert_with(|| copy(at, uv)),
                    // A corner the unwrap does not chart shares one copy per
                    // vertex, keyed past every unwrapped index.
                    None => *copies
                        .entry((vertex, unwrapped.unwrap_or(u32::MAX)))
                        .or_insert_with(|| {
                            copy(
                                at,
                                unwrapped.map_or_else(
                                    || self.mesh.uvs.get(at).copied().unwrap_or(Vec2::ZERO),
                                    |index| unwrap.uvs[index as usize],
                                ),
                            )
                        }),
                };
                charted.push(charted_at);
            }
            faces.push(charted);
        }
        for face in faces {
            mesh.push_face(face);
        }
        mesh.set_uvs(uvs);
        mesh.set_normals(normal_of);
        if skin.len() == mesh.vertex_count() {
            mesh.set_skin(skin);
        }
        if colours.len() == mesh.vertex_count() {
            mesh.set_colours(colours);
        }
        mesh
    }
}

/// How far a rim is charted from its hem edge toward the middle of the face
/// that edge bounds, as a share of the way.
///
/// A rim spans no area in the atlas, so charted on its hem edge it samples
/// the texels either side of that edge — and at the waist the far side is
/// the other garment's: on seed 7, 232 of 3,248 samples along the trousers'
/// rims landed on the top's cloth (#357). Halfway in, a rim samples only
/// cloth its own face painted.
pub const RIM_INSET: f32 = 0.5;

/// Where a rim corner is charted: [`RIM_INSET`] of the way from `corner`,
/// one of `ring`'s unwrapped corners, toward the middle of `ring`.
fn rim_uv(unwrap: &UvUnwrap, ring: &[u32], corner: u32) -> Option<Vec2> {
    let points: Vec<Vec2> = ring
        .iter()
        .filter_map(|&at| unwrap.uvs.get(at as usize).copied())
        .collect();
    if points.is_empty() {
        return None;
    }
    let middle = points.iter().copied().sum::<Vec2>() / points.len() as f32;
    Some(unwrap.uvs.get(corner as usize)?.lerp(middle, RIM_INSET))
}

/// Which face of `unwrap` each body face became, one entry per body face.
///
/// The index [`Garment::charted`] and the cloth atlas look a body face's
/// corners up through. `None` for a face the unwrap does not hold, which a
/// complete unwrap never leaves.
#[must_use]
pub fn charted_faces(unwrap: &UvUnwrap, body_faces: usize) -> Vec<Option<u32>> {
    let mut face_of = vec![None; body_faces];
    for (index, &from) in unwrap.source_face.iter().enumerate() {
        if let Some(slot) = face_of.get_mut(from as usize) {
            *slot = Some(index as u32);
        }
    }
    face_of
}

/// The influences of one body vertex.
fn borrowed(weights: &SkinWeights, vertex: usize) -> [Influence; MAX_INFLUENCES] {
    weights
        .vertices
        .get(vertex)
        .copied()
        .unwrap_or([Influence::default(); MAX_INFLUENCES])
}

/// The undirected key an edge is looked up by, endpoints in order.
fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// Which covered faces use each edge, as `(face, position)` within [`Garment::cut`]'s
/// `covered` list.
fn edge_faces(covered: &[&Vec<u32>]) -> HashMap<(u32, u32), Vec<(usize, usize)>> {
    let mut users: HashMap<(u32, u32), Vec<(usize, usize)>> = HashMap::new();
    for (index, face) in covered.iter().enumerate() {
        for at in 0..face.len() {
            let key = edge_key(face[at], face[(at + 1) % face.len()]);
            users.entry(key).or_default().push((index, at));
        }
    }
    users
}

/// Edges that only one covered face uses — the garment's cut edges.
///
/// Returned as `(face, position)` rather than as a pair of body vertices, and
/// that is not bookkeeping: at a vertex where the boundary touches itself the
/// garment carries more than one column for the same body vertex (see
/// [`fans`]), so a body vertex no longer names a garment one. The corner does.
///
/// Directed by construction — position `at` runs from `face[at]` to its
/// successor — so the rim built from them faces outward without anything having
/// to work out which side is which.
fn hem_edges(covered: &[&Vec<u32>]) -> Vec<(usize, usize)> {
    let mut hem: Vec<(usize, usize)> = edge_faces(covered)
        .into_values()
        .filter(|users| users.len() == 1)
        .map(|users| users[0])
        .collect();
    // A `HashMap` hands its values back in whatever order it likes, and a
    // garment's face list is compared between runs by `tests/record.rs`.
    hem.sort_unstable();
    hem
}

/// The cut edges of a claim, as closed loops of body vertices.
///
/// The hem itself, in the order it is walked, which is what anything wanting to
/// *measure* or *reshape* a hem needs and what the rim is built along. A garment
/// solid has no boundary edges to read this off afterwards, so it is offered
/// here from the claim.
///
/// Successors are found per fan rather than per vertex, for the reason `fans`
/// gives: where the boundary touches itself, one body vertex carries two
/// outgoing hem edges and only the fan says which loop each continues. A vertex
/// at such a pinch therefore appears in the returned loops twice, once for each
/// loop passing through it — the loops are sequences of body vertices, not sets.
///
/// Each loop runs in its faces' winding, so a loop is traversed with the claim
/// on one consistent side.
#[must_use]
pub fn hem_loops(mesh: &PolyMesh, mine: &[bool]) -> Vec<Vec<u32>> {
    let covered: Vec<&Vec<u32>> = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|&(index, _)| mine.get(index).copied().unwrap_or(false))
        .map(|(_, face)| face)
        .collect();
    let fan = fans(&covered);
    let hem = hem_edges(&covered);
    hem_walk(&covered, &fan, &hem)
        .iter()
        .map(|ring| ring.iter().map(|&(face, at)| covered[face][at]).collect())
        .collect()
}

/// The hem's edges chained into loops, as corners of `covered`.
///
/// Corners rather than vertices, because [`Garment::sew`] needs the columns and
/// only the corner names one: at a pinch the same body vertex carries two.
/// [`hem_loops`] is this walk read out as body vertices.
fn hem_walk(covered: &[&Vec<u32>], fan: &Fans, hem: &[(usize, usize)]) -> Vec<Vec<(usize, usize)>> {
    // Where each hem edge starts, keyed by the fan it starts in. Within one fan
    // a vertex has exactly one outgoing hem edge and one incoming, which is what
    // makes this a lookup rather than a search.
    let mut leaving: HashMap<(u32, usize), usize> = HashMap::new();
    for (index, &(face, at)) in hem.iter().enumerate() {
        leaving.insert((covered[face][at], fan.root(face, at)), index);
    }

    let mut walked = vec![false; hem.len()];
    let mut loops = Vec::new();
    for start in 0..hem.len() {
        if walked[start] {
            continue;
        }
        let mut path = Vec::new();
        let mut here = start;
        while !walked[here] {
            walked[here] = true;
            let (face, at) = hem[here];
            path.push((face, at));
            let onward = (at + 1) % covered[face].len();
            // A body already non-manifold along this edge is the only way out
            // of this walk, and it is the body's defect rather than the hem's.
            let Some(&next) = leaving.get(&(covered[face][onward], fan.root(face, onward))) else {
                break;
            };
            here = next;
        }
        loops.push(path);
    }
    loops
}

/// How far a hem column may slide, as a share of the way to the nearest body
/// vertex that is not itself on the hem.
///
/// **Under a half, and that is the whole safety argument for suppressing the
/// skin underneath.** A face whose corners are all off the hem is
/// separated from the hem by a whole face, so a hem that cannot travel even
/// half a face can never uncover one. Above a half the smoothing would have to
/// re-decide which faces are covered as it went, and [`Garment::hidden`] could
/// no longer be a plain subset of the claim.
const HEM_SLIDE: f32 = 0.45;

/// How many smoothing passes the hem gets.
const HEM_PASSES: usize = 8;

/// The pass that smooths, and the pass that undoes its shrinkage.
///
/// Taubin's pair. Smoothing alone drags a hem loop toward its own chords, and
/// a collar that shrinks is a collar that ends up inside the neck; the second,
/// larger, negative pass restores the low frequencies while leaving the
/// zigzag — which is what is being removed — where the first pass put it.
const HEM_SMOOTH: f32 = 0.5;
const HEM_UNSHRINK: f32 = -0.53;

/// How many stand-offs from the nearest skin the walk is allowed.
///
/// Measured over the six-seed sweep: the worst column settles in seven, and
/// most in one. Twenty-four is a backstop against a body nobody has built yet,
/// not a working figure — and it IS reached, on an extreme record, which is
/// why the walk is not the only mechanism here.
const ESCAPE_STEPS: usize = 24;

/// How many directions the escape sweep tries when the walk does not settle.
///
/// **Not a resolution knob, and it was one until the selection was fixed.** A
/// Fibonacci sphere of `n` points is a different point set for every `n`, so
/// whether one of them lands in a slit's mouth is luck: while the sweep was
/// load-bearing, 128 and 256 cleared every column of every record the tests
/// build and 96 and 192 did not — a constant whose value decided correctness by
/// chance. It stopped being load-bearing when the walk's own converged POINT
/// was allowed to be the answer instead of only its direction, and the same
/// experiment now passes at 32, 96, 192 and 512 alike. Five hundred and twelve
/// stays because it costs nothing on the columns that never reach it.
const ESCAPE_DIRECTIONS: usize = 512;

/// How many bisections narrow the escape onto the normal it started from.
///
/// Eight halvings is a two-hundred-and-fiftieth of the angle turned, which is
/// finer than the mesh the direction is asked of.
const ESCAPE_REFINEMENTS: usize = 8;

/// Where a column's outer shell stands, `thickness` clear of the body.
///
/// Returns `seat + heading * thickness` wherever that is already outside the
/// body, which is every column of a garment but a handful at the crotch. Where
/// it is not, a position that does leave the body is found — by a walk that
/// follows the body's own geometry and, where that will not settle, by a sweep
/// that enumerates — and then turned back toward `heading` as far as the body
/// allows, so the cloth moves as little as it can.
///
/// **A POSITION, not a direction, and the difference is 2 to 6 vertices per
/// body.** Handing back a heading for the caller to re-apply is only the same
/// thing while the standoff is; the first version of this walked the point out
/// to where it stood `thickness` from the nearest SKIN, which at the bottom of
/// a slit is further from the seat than `thickness`, and then returned the
/// direction. Re-applying `thickness` to it pulled the vertex back down the
/// bridge and inside the body again, leaving 2 of the original 6 on three of
/// six seeds while the probe that judged it read zero — because the probe was
/// checking the converged point and the code was shipping the heading.
///
/// **This is a global question and it was measured before it was answered**
/// (#279). The premise of an offset garment is that a point of the body pushed
/// along its own normal leaves the body, and at the crotch it does not: 4 to 14
/// outer columns per body come back 1.2 to 8.0 mm INSIDE the skin, at the very
/// bottom of the notch between the thighs, in zone `UpperLimb`, none of them on
/// a hem and none of them a pinch column. Eight millimetres is the whole
/// thickness, so the worst of them are offset in exactly the wrong direction.
/// The verdict is not the crate's own single-ray `contains`, which is unusable
/// that close to a surface — probing every body face 50 µm either side of its
/// own plane, 10 to 21% of them deny that the body separates its own sides —
/// but nine rays that agree, on points 1.2 mm and further in, where they do.
///
/// The issue's own three candidates were each measured on the six seeds
/// `garmentaudit` sweeps, and each is refuted by its number:
///
/// - **Clamp the offset to the local concave radius.** It cannot reach the
///   columns whose direction is wrong at any distance, and of the rest only 0
///   to 4 per body keep more standoff than [`GarmentCut::bite`] — an outer
///   shell behind its own inner one.
/// - **Smooth the offset field before applying it.** One relaxation pass of the
///   normals clears 2 to 4 of them, four passes 2 to 6; it never reaches zero,
///   because at a crease that closes to a slit the neighbouring normals are
///   wrong in the same direction. Smoothing the body's POSITIONS and taking
///   normals from that is worse still — it raises the count on four of the six
///   seeds, since a relaxed body is a thinner one.
/// - **A projection pass.** That is the walk below, and on its own it clears
///   every column of every seed `garmentaudit` sweeps — but not of every
///   record a slider can ask for.
///
/// Two more were tried and are refuted with them: relaxing the failing columns'
/// positions into the ones that clear reaches zero on three of six seeds, and
/// diffusing their headings from the same neighbours on two of six. The crease
/// is genuinely degenerate — the local data does not contain the answer, and
/// only the body itself does.
///
/// If neither mechanism finds a direction the column is left where its own
/// normal put it, because a garment that still builds is worth more than one
/// that refuses a body. `no_garment_stands_inside_the_body_it_was_cut_from` is
/// what says that has not happened.
fn stood_off(mesh: &PolyMesh, seat: Vec3, heading: Vec3, thickness: f32) -> Vec3 {
    let plain = seat + heading * thickness;
    if !mesh.contains(plain) {
        return plain;
    }

    // Every candidate that leaves the body, judged by ONE rule: the least the
    // cloth has to move. Both mechanisms below can fail on a body the other
    // handles, and neither ranks above the other — where they both answer, the
    // nearer answer wins, which is also what keeps the hem where it was.
    let mut best: Option<Vec3> = None;
    let mut consider = |point: Vec3| {
        if mesh.contains(point) {
            return;
        }
        if best.is_none_or(|held| point.distance(plain) < held.distance(plain)) {
            best = Some(point);
        }
    };

    // The walk: stand off the nearest skin, over and over, so each step takes
    // its direction from the body at the place the cloth actually landed rather
    // than from the crease it was cut in. One to seven steps settle it on every
    // seed `garmentaudit` sweeps. It is a fixed-point iteration with no fixed
    // point in a slit narrower than twice the thickness, where it bounces wall
    // to wall — hence the cap, and hence the sweep below it.
    let mut point = plain;
    for _ in 0..ESCAPE_STEPS {
        let near = onto_surface(mesh, 0..mesh.faces.len(), point);
        let out = (near - point).try_normalize().unwrap_or(heading);
        point = near + out * thickness;
        if !mesh.contains(point) {
            consider(point);
            break;
        }
    }

    // The sweep: enumerate, because enumerating cannot fail to terminate. See
    // [`ESCAPE_DIRECTIONS`] for why its count is not a resolution knob.
    for index in 0..ESCAPE_DIRECTIONS {
        consider(seat + spread(index) * thickness);
    }

    let Some(found) = best else {
        return plain;
    };

    // Turn back toward the normal as far as the body allows, holding whatever
    // standoff the winning candidate has. A column that turns less disturbs the
    // hem it may be part of less, and the hem is what this must not be paid for.
    let stand = found.distance(seat);
    let mut clear = (found - seat) / stand;
    let mut blocked = heading;
    for _ in 0..ESCAPE_REFINEMENTS {
        let Some(between) = (clear + blocked).try_normalize() else {
            break;
        };
        if mesh.contains(seat + between * stand) {
            blocked = between;
        } else {
            clear = between;
        }
    }
    seat + clear * stand
}

/// The `index`th of [`ESCAPE_DIRECTIONS`] directions spread over the sphere.
///
/// The Fibonacci spiral: as even a spacing as a sphere allows without solving
/// for it, and the only construction here that needs no table.
fn spread(index: usize) -> Vec3 {
    let y = 1.0 - (index as f32 + 0.5) / ESCAPE_DIRECTIONS as f32 * 2.0;
    let radius = (1.0 - y * y).max(0.0).sqrt();
    let turn = index as f32 * std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    Vec3::new(turn.cos() * radius, y, turn.sin() * radius)
}

/// The point of a triangle closest to `point`, by region.
///
/// Ericson's Voronoi-region form: the six cases are the three corners, the
/// three edges, and the interior, and testing them in this order is what makes
/// it branch-only arithmetic with no square roots.
fn nearest_on_triangle(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, point - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = point - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = point - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// The nearest point of `faces` to `point`, as a point of the body's surface.
///
/// **Which faces are offered is the caller's decision and it matters.**
/// [`smooth_hem`] offers only the ring around the vertex a hem column was cut
/// from, which is both the fast answer and the right one: [`HEM_SLIDE`] keeps
/// the slide inside that ring, and a global search could snap a collar onto the
/// shoulder it happens to be nearest in space. [`stood_off`] offers the
/// whole body, because a column buried in the crotch is buried in the far
/// thigh as often as in its own and the ring around its seat cannot say so.
fn onto_surface(mesh: &PolyMesh, faces: impl IntoIterator<Item = usize>, point: Vec3) -> Vec3 {
    let mut nearest = point;
    let mut best = f32::MAX;
    for face in faces {
        let corners = &mesh.faces[face];
        for at in 1..corners.len().saturating_sub(1) {
            let (a, b, c) = (
                mesh.positions[corners[0] as usize],
                mesh.positions[corners[at] as usize],
                mesh.positions[corners[at + 1] as usize],
            );
            let candidate = nearest_on_triangle(point, a, b, c);
            let span = candidate.distance(point);
            if span < best {
                best = span;
                nearest = candidate;
            }
        }
    }
    nearest
}

/// Slides each hem column along the surface onto a smooth curve, in place.
///
/// The cut takes whole faces, so a hem can only ever be a staircase of them:
/// measured on the delivered body its steps are about 24 mm, a quarter of the
/// corners turn by more than 45 degrees, and it stands 2 to 4 mm RMS — up to 20
/// mm — off the smooth ring it is trying to be (`examples/garmentaudit`). None
/// of that is a tuning problem; the boundary is a nearest-bone classification
/// and it wanders vertex by vertex.
///
/// What is *not* done here is re-cutting the faces. A garment vertex is charted
/// and weighted by the body vertex it was cut from ([`Garment::source`]), so
/// splitting a face on the boundary invents points with no chart, no weights and
/// no fan; sliding the columns that already exist invents nothing, costs no
/// triangles, and removes the amplitude rather than the step count. A hem of
/// fourteen straight segments around a neck reads as a curve; a hem that dips a
/// whole face at the throat does not.
///
/// The slide is tangential only in the sense that it stays on the loop's own
/// smooth form: the column keeps the normal of the body vertex it came from, so
/// the shell still stands off the skin, and the loop sinks toward its chords by
/// about a millimetre — second order in the step, and an order under the eight
/// the garment stands out.
///
/// Every column is clamped to [`HEM_SLIDE`] of the way to the nearest body
/// vertex that is not itself on the hem — plus the millimetre the projection
/// back onto the surface takes — which is what keeps a hem inside the row of
/// faces it was cut along and lets [`Garment::hidden`] stay a subset of the
/// claim. A column with no such neighbour cannot be bounded and does not move,
/// which is why a two-ring band like a pair of shorts keeps its staircase:
/// every one of its vertices is on a hem.
fn smooth_hem(
    mesh: &PolyMesh,
    near: &Neighbourhood,
    rings: &[Vec<u32>],
    source: &[u32],
    at: &mut [Vec3],
) {
    for ring in rings {
        if ring.len() < 4 {
            continue;
        }
        let mut smoothed: Vec<Vec3> = ring.iter().map(|&column| at[column as usize]).collect();
        for _ in 0..HEM_PASSES {
            for step in [HEM_SMOOTH, HEM_UNSHRINK] {
                let was = smoothed.clone();
                for index in 0..was.len() {
                    let last = was[(index + was.len() - 1) % was.len()];
                    let next = was[(index + 1) % was.len()];
                    smoothed[index] = was[index] + ((last + next) * 0.5 - was[index]) * step;
                }
            }
        }
        for (index, &column) in ring.iter().enumerate() {
            let from = at[column as usize];
            let limit = near.room(mesh, source[column as usize]) * HEM_SLIDE;
            let moved = smoothed[index] - from;
            let slid = if moved.length() > limit {
                from + moved.normalize_or_zero() * limit
            } else {
                smoothed[index]
            };
            // Back onto the skin. A smoothed ring cuts its own corners and the
            // unshrinking pass overshoots them, both by about a millimetre on a
            // 24 mm step — an eighth of the standoff the garment is built with,
            // and enough to read as the garment standing off the body by more
            // than its own thickness. The column is a point OF the body, and
            // this is what keeps that true after it moves.
            at[column as usize] = near.onto(mesh, source[column as usize], slid);
        }
    }
}

/// What surrounds each hem vertex on the body: the faces its column may land
/// on when it slides, and the vertices beside it that bound how far.
struct Neighbourhood {
    /// The faces round each hem vertex.
    around: HashMap<u32, Vec<usize>>,
    /// The body vertices sharing a face with each hem vertex that are not on
    /// a hem themselves — the ones holding the first face a garment hides.
    beside: HashMap<u32, Vec<u32>>,
}

impl Neighbourhood {
    /// The neighbourhood of every vertex on `rings`' hems.
    fn of(mesh: &PolyMesh, rings: &[Vec<u32>], source: &[u32]) -> Self {
        let mut on_hem = vec![false; mesh.vertex_count()];
        for ring in rings {
            for &column in ring {
                on_hem[source[column as usize] as usize] = true;
            }
        }
        let mut around: HashMap<u32, Vec<usize>> = HashMap::new();
        let mut beside: HashMap<u32, Vec<u32>> = HashMap::new();
        for (index, face) in mesh.faces.iter().enumerate() {
            for &here in face {
                if !on_hem[here as usize] {
                    continue;
                }
                around.entry(here).or_default().push(index);
                let next = beside.entry(here).or_default();
                for &other in face {
                    if other != here && !on_hem[other as usize] && !next.contains(&other) {
                        next.push(other);
                    }
                }
            }
        }
        Self { around, beside }
    }

    /// How far a hem vertex's column may travel in any direction before it
    /// reaches a vertex that is off the hem. Zero with no such vertex.
    fn room(&self, mesh: &PolyMesh, vertex: u32) -> f32 {
        let here = mesh.positions[vertex as usize];
        self.beside
            .get(&vertex)
            .into_iter()
            .flatten()
            .map(|&other| here.distance(mesh.positions[other as usize]))
            .reduce(f32::min)
            .unwrap_or(0.0)
    }

    /// How far it may travel along `direction`: the least advance along that
    /// direction that reaches a vertex off the hem. Zero with none that way.
    fn room_along(&self, mesh: &PolyMesh, vertex: u32, direction: Vec3) -> f32 {
        let here = mesh.positions[vertex as usize];
        self.beside
            .get(&vertex)
            .into_iter()
            .flatten()
            .map(|&other| (mesh.positions[other as usize] - here).dot(direction))
            .filter(|&ahead| ahead > 0.0)
            .reduce(f32::min)
            .unwrap_or(0.0)
    }

    /// `point` put back on the skin round a hem vertex.
    fn onto(&self, mesh: &PolyMesh, vertex: u32, point: Vec3) -> Vec3 {
        self.around.get(&vertex).map_or(point, |faces| {
            onto_surface(mesh, faces.iter().copied(), point)
        })
    }
}

/// How far a hem column may slide to meet its length, as a share of the way
/// to the next ring in the direction it moves.
///
/// The claim rounds to the nearer ring, so no hem needs more than about half
/// a row; this is the backstop, and it stays under one for the reason
/// [`HEM_SLIDE`] stays under a half — a column that reached the next ring
/// inward would uncover the first face the garment hides.
const PLACE_SLIDE: f32 = 0.9;

/// Slides each limb's hem along the limb onto the ring its length names
/// (#356).
///
/// The claim takes whole faces, and a limb carries about four rows of them
/// per segment, so a hem left where the claim put it sat on one of a handful
/// of rings and a length moved it in jumps of a sixth of the limb. Sliding the
/// columns the rest of the way — at most half a row, since the claim rounds to
/// the nearer ring — puts the hem where the length says on every body,
/// between rings as readily as on one, and invents nothing: the columns stay
/// points of the body, charted and weighted by the vertices they were cut
/// from, as [`smooth_hem`]'s do.
///
/// A ring is a limb's hem when most of its columns were cut from that limb's
/// skin: a hand hanging beside a thigh zones a little of it as the hand, and
/// the hem round the thigh is still the leg's.
#[allow(
    clippy::too_many_arguments,
    reason = "the sew's own context, passed through; a struct for one call site is ceremony"
)]
fn place_hems(
    mesh: &PolyMesh,
    rig: &Rig,
    zones: &[Zone],
    cut: &GarmentCut,
    near: &Neighbourhood,
    rings: &[Vec<u32>],
    source: &[u32],
    at: &mut [Vec3],
) {
    let targets: Vec<(Limb, f32, Chain)> = cut
        .limbs
        .iter()
        .flatten()
        .filter(|(_, reach)| *reach > 0.0 && *reach < 1.0)
        .filter_map(|&(limb, reach)| Some((limb, reach, Chain::of(rig, limb)?)))
        .collect();
    for ring in rings {
        let owner = targets.iter().find(|(limb, _, _)| {
            let own = ring
                .iter()
                .filter(|&&column| {
                    matches!(
                        zones.get(source[column as usize] as usize),
                        Some(Zone::UpperLimb(of) | Zone::LowerLimb(of)) if of == limb
                    )
                })
                .count();
            own * 2 > ring.len()
        });
        let Some((_, reach, chain)) = owner else {
            continue;
        };
        let goal = chain.point(*reach);
        for &column in ring {
            let vertex = source[column as usize];
            let from = at[column as usize];
            let shift = goal - chain.point(chain.position(from));
            let Some(direction) = shift.try_normalize() else {
                continue;
            };
            let limit = near.room_along(mesh, vertex, direction) * PLACE_SLIDE;
            let moved = from + direction * shift.length().min(limit);
            at[column as usize] = near.onto(mesh, vertex, moved);
        }
    }
}

/// Runs of covered faces that meet edge to edge, one class per corner.
///
/// **A vertex is not a column.** The covered region is a patch cut out of
/// the body, and its boundary is a set of loops only so long as it never touches
/// itself. It does touch itself — at the top of a pair of shorts, all the way
/// round the waist — and there a single body vertex carries two separate runs of
/// covered faces with four boundary edges between them. [`Garment::cut`] gives
/// each run its own garment vertex, so each boundary loop gets its own rim
/// column and its own side edge; sharing one put four rim quads on one edge.
///
/// Two corners at the same vertex are in the same run when a covered face
/// spans the edge between them, so this is a union-find over corners with one
/// union per interior edge. It says nothing about winding: the corners at each
/// end of a shared edge are matched by vertex, so a face wound the wrong way
/// round joins the same run it would have anyway.
struct Fans {
    /// Where each face's corners start in the flat corner numbering.
    offset: Vec<usize>,
    parent: Vec<usize>,
}

impl Fans {
    /// The run a corner belongs to, named by its representative corner.
    fn root(&self, face: usize, at: usize) -> usize {
        let mut node = self.offset[face] + at;
        while self.parent[node] != node {
            node = self.parent[node];
        }
        node
    }

    /// Joins the runs two corners belong to.
    fn join(&mut self, a: usize, b: usize) {
        let (mut a, mut b) = (a, b);
        while self.parent[a] != a {
            a = self.parent[a];
        }
        while self.parent[b] != b {
            b = self.parent[b];
        }
        if a != b {
            self.parent[a] = b;
        }
    }
}

/// Classifies every corner of `covered` into the run of faces it sits in.
fn fans(covered: &[&Vec<u32>]) -> Fans {
    let mut offset = Vec::with_capacity(covered.len());
    let mut total = 0;
    for face in covered {
        offset.push(total);
        total += face.len();
    }
    let mut built = Fans {
        offset,
        parent: (0..total).collect(),
    };

    for ((p, q), users) in edge_faces(covered) {
        // Exactly two, because an edge with more than that is a body already
        // non-manifold along it and no split here would mend the garment. One
        // is a boundary edge, which is what separates two runs rather than
        // joining them.
        let [(left, left_at), (right, right_at)] = users[..] else {
            continue;
        };
        // Matched by vertex rather than by winding: `at` and its successor are
        // the edge's two ends, and which of them is `p` depends on how the face
        // was wound.
        for end in [p, q] {
            let corner_of = |face: usize, at: usize| {
                let len = covered[face].len();
                let corner = if covered[face][at] == end {
                    at
                } else {
                    (at + 1) % len
                };
                built.offset[face] + corner
            };
            let (a, b) = (corner_of(left, left_at), corner_of(right, right_at));
            built.join(a, b);
        }
    }
    built
}

/// A colour, from a hue around the wheel and a lightness.
///
/// Dyed cloth, not paint: the ramp never reaches full saturation, because a
/// fully saturated garment reads as plastic next to skin that never is.
#[must_use]
pub fn dye(hue: f32, shade: f32) -> [f32; 3] {
    let turn = hue.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let level = 0.18 + 0.62 * shade.clamp(0.0, 1.0);
    let spread = 0.26 * (1.0 - (level - 0.45).abs());
    let wheel = |offset: f32| level + spread * (turn + offset).cos();
    [
        wheel(0.0).clamp(0.0, 1.0),
        wheel(2.094).clamp(0.0, 1.0),
        wheel(4.189).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Limb;
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};
    use glam::Vec3;

    fn body() -> (PolyMesh, Rig, SkinWeights, Vec<Zone>) {
        let record = AvatarRecord::new("Dressed", Archetype::default());
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        (mesh, rig, weights, zones)
    }

    fn torso() -> ZoneSet {
        ZoneSet::default().with(Zone::Chest).with(Zone::Abdomen)
    }

    #[test]
    fn a_garment_can_be_cut_from_a_body() {
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.3, 0.3, 0.4])
            .expect("a torso exists");
        assert!(garment.vertex_count() > 40);
        assert!(garment.mesh.face_count() > 20);
    }

    #[test]
    fn a_cut_that_covers_nothing_makes_no_garment() {
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            // No humanoid has these.
            zones: ZoneSet::default().with(Zone::Extremity(Limb::ForeLeft)),
            ..Default::default()
        };
        // Extremities exist but are a single ring, so this may or may not cover
        // whole faces; what must not happen is a panic or an empty mesh.
        if let Some(garment) = Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]) {
            assert!(garment.mesh.face_count() > 0);
        }

        let nothing = GarmentCut {
            zones: ZoneSet::default(),
            ..Default::default()
        };
        assert!(Garment::cut(&mesh, &rig, &weights, &zones, &nothing, [0.5; 3]).is_none());
    }

    #[test]
    fn a_garment_is_a_closed_solid() {
        // Cloth has two faces and a cut edge. A single offset surface vanishes
        // when seen from underneath and reads as a sticker at the hem.
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(
            garment.mesh.is_closed_manifold(),
            "{:?}",
            garment.mesh.manifold_report()
        );
    }

    #[test]
    fn the_hem_walks_as_closed_loops() {
        // What the rim is built along, and what anything reshaping a hem has to
        // read. Two claims: every cut edge is walked exactly once, and every
        // loop closes — a walk that runs off the end of a chain would report a
        // hem shorter than the one that exists and quietly hide the rest.
        let (mesh, rig, _, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let mut mine = claimed(&mesh, &rig, &zones, &cut);
        close(&mesh, &mut mine, &[]);
        let covered: Vec<&Vec<u32>> = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|&(index, _)| mine[index])
            .map(|(_, face)| face)
            .collect();
        let loops = hem_loops(&mesh, &mine);
        assert!(!loops.is_empty(), "a torso has a neck and a waist at least");
        assert_eq!(
            loops.iter().map(Vec::len).sum::<usize>(),
            hem_edges(&covered).len(),
            "the walk visited a different number of edges than the hem has"
        );
        for ring in &loops {
            assert!(ring.len() >= 3, "a loop of {} vertices", ring.len());
            // Each step of a loop is an edge of the body, which is what makes
            // it a walk over the surface rather than a list of vertices that
            // happen to be on the boundary.
            for at in 0..ring.len() {
                let (here, next) = (ring[at], ring[(at + 1) % ring.len()]);
                assert!(
                    mesh.faces.iter().any(|face| {
                        (0..face.len())
                            .any(|c| face[c] == here && face[(c + 1) % face.len()] == next)
                    }),
                    "{here} to {next} is not an edge of the body"
                );
            }
        }
    }

    #[test]
    fn a_garment_knows_the_faces_it_was_cut_from() {
        // The claim is what lets the body stop emitting the skin underneath, so
        // it has to name the faces the garment actually stands over — not the
        // raw zone claim it started from, which `close` has since grown.
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let mut mine = claimed(&mesh, &rig, &zones, &cut);
        let raw = mine.iter().filter(|&&mine| mine).count();
        close(&mesh, &mut mine, &[]);
        let garment =
            Garment::sew(&mesh, &rig, &weights, &zones, &mine, &cut, [0.5; 3]).expect("a torso");
        assert_eq!(
            garment.claim.len(),
            mine.iter().filter(|&&mine| mine).count()
        );
        assert!(garment.claim.len() >= raw, "the claim shrank under close");
        assert!(garment.claim.windows(2).all(|pair| pair[0] < pair[1]));
        for &face in &garment.claim {
            assert!(mine[face as usize]);
        }
    }

    /// How far a point lies off the surface of a mesh, in metres.
    ///
    /// **The surface, not the nearest vertex** (`docs/instruments.md` rule 2).
    /// Both of the tests below used to ask for the nearest body VERTEX, which
    /// was exact only for as long as every garment column stood over one. The
    /// hem's columns no longer do — [`smooth_hem`] slides them along the
    /// surface, up to 7 mm on the bodies here — and the vertex proxy read that
    /// slide as the garment standing off the skin by twice its own thickness.
    fn off_the_surface(mesh: &PolyMesh, point: Vec3) -> f32 {
        mesh.triangulated()
            .iter()
            .map(|corners| {
                let [a, b, c] = corners.map(|corner| mesh.positions[corner as usize]);
                super::nearest_on_triangle(point, a, b, c).distance(point)
            })
            .fold(f32::MAX, f32::min)
    }

    #[test]
    fn a_garment_stands_off_the_skin_it_was_cut_from() {
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            thickness: 0.01,
            bite: 0.002,
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]).expect("a torso");

        // Every garment point is within a hair of the body, and the outer half
        // is outside it: it hugs, and it cannot be inside.
        for point in &garment.mesh.positions {
            let off = off_the_surface(&mesh, *point);
            assert!(
                off <= 0.011,
                "a garment point sat {off} from the body's surface"
            );
        }
        let half = garment.vertex_count() / 2;
        for (index, point) in garment.mesh.positions.iter().enumerate() {
            let inside = mesh.contains(*point);
            assert_eq!(
                inside,
                index >= half,
                "garment point {index} of {} was {}side the body",
                garment.vertex_count(),
                if inside { "in" } else { "out" }
            );
        }
    }

    #[test]
    fn a_garment_wears_the_skin_weights_of_the_body_beneath_it() {
        // The whole reason for cutting one this way: it needs no rigging.
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let garment = Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(garment.mesh.channels_are_consistent());
        assert_eq!(garment.mesh.skin.len(), garment.vertex_count());
        let worn = SkinWeights {
            vertices: garment.mesh.skin.clone(),
        };
        assert!(worn.is_normalized(1e-3));
        assert!(
            garment
                .mesh
                .skin
                .iter()
                .all(|influences| influences.iter().any(|i| i.weight > 0.0)),
            "a garment vertex came out unweighted"
        );
        // Each garment vertex names the body vertex it was cut from, and the
        // two shells name the same ones.
        assert_eq!(garment.source.len(), garment.vertex_count());
        let half = garment.vertex_count() / 2;
        assert_eq!(garment.source[..half], garment.source[half..]);
    }

    #[test]
    fn a_garment_is_charted_where_the_body_is_charted() {
        let (mesh, rig, weights, zones) = body();
        let cut = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let mut garment =
            Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
        assert!(
            garment.mesh.uvs.is_empty(),
            "a cut garment is not yet charted"
        );

        // Stand-in for the body's atlas: one coordinate per body vertex.
        let body_uvs: Vec<Vec2> = (0..mesh.vertex_count())
            .map(|v| Vec2::new(v as f32 / mesh.vertex_count() as f32, 0.25))
            .collect();
        garment.chart(&body_uvs);

        assert!(garment.mesh.channels_are_consistent());
        for (vertex, &from) in garment.source.iter().enumerate() {
            assert_eq!(garment.mesh.uvs[vertex], body_uvs[from as usize]);
        }
    }

    #[test]
    fn a_wider_cut_covers_more() {
        let (mesh, rig, weights, zones) = body();
        let narrow = GarmentCut {
            zones: torso(),
            ..Default::default()
        };
        let wide = GarmentCut {
            zones: torso().with(Zone::Pelvis),
            ..Default::default()
        };
        let small =
            Garment::cut(&mesh, &rig, &weights, &zones, &narrow, [0.5; 3]).expect("a torso");
        let large = Garment::cut(&mesh, &rig, &weights, &zones, &wide, [0.5; 3]).expect("a torso");
        assert!(large.vertex_count() > small.vertex_count());
    }

    #[test]
    fn thickness_sets_how_far_a_garment_stands_out() {
        // Measured against the skin, not against the bounding box. A box grows
        // by whatever the normal at its widest vertex happens to be pointing at,
        // which is not the quantity being set.
        let (mesh, rig, weights, zones) = body();
        let furthest = |thickness: f32| {
            let cut = GarmentCut {
                zones: torso(),
                thickness,
                bite: 0.001,
                ..Default::default()
            };
            let garment =
                Garment::cut(&mesh, &rig, &weights, &zones, &cut, [0.5; 3]).expect("a torso");
            garment.mesh.positions[..garment.vertex_count() / 2]
                .iter()
                .map(|point| off_the_surface(&mesh, *point))
                .fold(0.0f32, f32::max)
        };
        for thickness in [0.004f32, 0.012, 0.02] {
            let stood = furthest(thickness);
            assert!(
                (stood - thickness).abs() < thickness * 0.25,
                "a garment {thickness} thick stood {stood} off the skin"
            );
        }
    }

    #[test]
    fn dye_stays_in_range_and_never_reaches_full_saturation() {
        for step in 0..12 {
            let hue = step as f32 / 12.0;
            for shade in [0.0, 0.35, 0.7, 1.0] {
                let colour = dye(hue, shade);
                assert!(
                    colour.iter().all(|c| (0.0..=1.0).contains(c)),
                    "dye({hue}, {shade}) gave {colour:?}"
                );
                let wide = colour.iter().fold(0.0f32, |a, b| a.max(*b))
                    - colour.iter().fold(f32::MAX, |a, b| a.min(*b));
                assert!(wide < 0.75, "dye({hue}, {shade}) was {wide} saturated");
            }
        }
    }

    #[test]
    fn dye_moves_with_both_of_its_axes() {
        assert_ne!(dye(0.0, 0.5), dye(0.4, 0.5));
        let dark = dye(0.3, 0.05);
        let light = dye(0.3, 0.95);
        let sum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!(sum(light) > sum(dark) * 1.5);
    }
}
