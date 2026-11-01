//! A beast's fur (#369): the hair system's grown layer, on a body's own zones.
//!
//! A person grows hair on five regions of a measured head. A beast grows it
//! where its plan says long hair grows - a mane down the neck's crest and a
//! forelock between the ears, a ruff round the neck, a brush along the tail,
//! feathering at the fetlocks, tufts at the ears' tips - and wears its coat,
//! painted (#368), everywhere else. The cards are the clump engine's own
//! ([`super::clump`]): each zone ([`FurZone`]) is a mask over the body's faces
//! that roots are scattered on by area and weight, and a curve each root's
//! card follows, lofted as a person's are. Three things differ from a head of
//! hair:
//!
//! - **Body space, bound by its root all the way down.** A head's cards are
//!   handed over to the head as they leave the skin; fur never lets go of the
//!   skin it grows from, so a brush rides the tail's springs and feathering
//!   the pasterns.
//! - **Coloured from the coat**: each card takes the coat's colour at its root,
//!   asked of the painter at a point, so a tabby's brush is ringed and a
//!   piebald's ruff patched - and a bay's or a dun's mane and tail are its
//!   dark.
//! - **Scaled with the body.** Lengths and widths are shares of the back's
//!   height and counts are fixed per zone, because the budget is triangles,
//!   not area.
//!
//! **A card goes in under the skin and lies on it** (#386). Seated on the skin
//! and standing off it, a card's root is a straight cut across a lock with
//! the coat right behind it, and every card showed one. So a card narrows to
//! a pinch at its root, starts below the skin, and comes out at a shallow
//! angle (`zone::Lock`); and a mane, whose every card a viewer sees, drapes
//! over the neck's measured section from under the crest's far side
//! (`Crest`), so the two sides' cards cross on it and cover each other.
//!
//! **A mane stands as stiffly as its record says** (#389): one set of cards at
//! every stiffness, as long as the record's `mane` says. How stiff they are says
//! how far a card stands before it bends over (`zone::Lock`), from lying on the
//! neck to straight up off the crest; each comes out of the skin the same way at
//! every value, which is what goes on hiding where it enters. Where it stands
//! too high for any card to lie over the seam, its colour hides it: the skin
//! under a mane in a coat's dark is painted that colour (`mane_roots`), as a
//! hair card's scalp is, and on any other coat a card is already the colour of
//! the skin it leaves.
//!
//! Drawn in the hair's draw: a beast grows no person's hair, so fur is the
//! one draw more a furred beast makes than a bald one - three in all, inside
//! the budget's four. Tiered under [`MAX_TRIANGLES`] as a head is under its
//! own: a zone's count is a request (`plan.md`, risk 5).

#[cfg(test)]
mod seam;
mod zone;

pub use zone::FurZone;

use glam::Vec3;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64Mcg;

use super::clump::Root;
use super::clump::loft::loft;
use super::clump::scatter::{area_of, inside};
use crate::face::beast::Ears;
use crate::face::step;
use crate::hair::shell::face::moller;
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::{Coat, Fur, Zone};
use crate::rig::{Influence, Rig, SkinWeights};
use crate::texture::coat::{CoatBody, CoatSampler, ManeRoots, darkens_points};
use zone::Lock;

/// What a beast's fur may cost, in triangles.
///
/// Provenance: **derived** from the budget. The dearest bald beast the sweep
/// in `tests/budget.rs` reaches is 20,042, and the dearest of 3,000 sampled
/// corners 21,000; this on that is 30,000 against the 32,200 target - a sum
/// the sweep re-measures rather than trusts, and holds only because the
/// ceiling is hard (`grow`). Not the whole leftover: a card is drawn over the
/// coat it grows from and over its neighbours, alpha-tested, and a triangle
/// of fur costs a browser more than a triangle of skin; 2,200 are kept back
/// for that. At every zone's full length a rolled horse grows
/// about 5,700, a dog 5,900 and a cat 4,800, the dearest roll 7,500 (a mane
/// draped over its neck, #386, costs about 750 of that). A mane's stiffness (#389)
/// bends the same cards and adds no card, only the stations a bend takes: the
/// dearest fur the sweep's long manes build is 8,350, about a quarter of the way up
/// the slider, so the ceiling bites only past that, toward the envelope's end.
pub const MAX_TRIANGLES: usize = 9_000;

/// How many times fur is regrown by the measured ratio to get under
/// [`MAX_TRIANGLES`], and how far under it such a regrowth aims:
/// [`super::clump::grow_head`]'s two, for its reasons.
const TIER_PASSES: usize = 3;
const TIER_AIM: f32 = 0.98;

/// How far under the ceiling a regrowth aims once those passes have not
/// landed, and how many more it may take: fur's ceiling is HARD, since the
/// budget sweep's sum (the dearest bald beast plus this ceiling) holds only
/// if nothing grows past it. A card's cost is its own path, so a ratio can
/// land a hair over; aiming a tenth low converges in one or two.
const LAST_AIM: f32 = 0.9;
const LAST_PASSES: usize = 5;

/// How far a root may move inside its share of a zone's area, and how many
/// times one is redrawn inside its face before it is given up:
/// [`super::clump::scatter`]'s, for its reasons.
const JITTER: f32 = 0.8;
const RETRIES: usize = 4;

/// How far a mane's root leans toward the side it falls, as the sideways part
/// of its facing: past anything the crest's own facing says once its sideways
/// part is taken out, so that it is the root's side and no vertex's.
const PARTING: f32 = 0.4;

/// The weakest root a mane grows a card from.
const THINNEST: f32 = 0.05;

/// One zone's fur, grown: a line of [`FurGrowth`]'s ledger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrownZone {
    /// Which zone it belongs to.
    pub zone: FurZone,
    /// How many cards were rooted.
    pub clumps: usize,
    /// What they cost, in triangles.
    pub tris: usize,
}

/// A beast's fur, in one mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FurGrowth {
    /// The cards, in body space, every vertex bound as the skin its card
    /// grows from is.
    pub mesh: PolyMesh,
    /// What each zone grew.
    pub grown: Vec<GrownZone>,
}

impl FurGrowth {
    /// What the whole coat of fur costs, in triangles.
    #[must_use]
    pub fn tris(&self) -> usize {
        self.grown.iter().map(|grown| grown.tris).sum()
    }
}

/// The body fur is grown on, and everything a card asks of it.
pub(crate) struct Bed<'a> {
    /// The built body, whose faces the roots are scattered over.
    pub body: &'a PolyMesh,
    /// Each of its vertices' zone.
    pub zones: &'a [Zone],
    /// The rig it was built against.
    pub rig: &'a Rig,
    /// How that rig holds each of its vertices, which a root takes.
    pub weights: &'a SkinWeights,
    /// The body as the coat painter reads it.
    pub anatomy: &'a CoatBody,
    /// The coat a card takes its colour from.
    pub coat: &'a Coat,
    /// The ears, rigged, for the tufts at their tips.
    pub ears: Option<&'a Ears>,
}

impl Bed<'_> {
    /// The body's forward: from the croup toward the withers.
    fn forward(&self) -> Vec3 {
        (self.anatomy.withers - self.anatomy.hips).normalize_or(Vec3::Z)
    }

    /// The head's forward, or the body's on a body without a declared head.
    fn head_forward(&self) -> Vec3 {
        self.anatomy.head.as_ref().map_or_else(
            || self.forward(),
            |head| (head.body(Vec3::Z) - head.body(Vec3::ZERO)).normalize_or(self.forward()),
        )
    }

    /// Up off the bone `at` hangs from: the world's up with the bone's own
    /// run taken out of it, which is where a neck's crest faces however the
    /// neck is carried.
    fn dorsal(&self, at: Vec3) -> Vec3 {
        let (from, to) = self.rig.bone(self.rig.nearest_bone(at).joint);
        let axis = (to - from).normalize_or(Vec3::Y);
        (Vec3::Y - axis * axis.y).normalize_or(Vec3::Y)
    }

    /// Along the bone `at` hangs from, toward its child: down a tail, toward
    /// its tip.
    fn distal(&self, at: Vec3) -> Vec3 {
        let (from, to) = self.rig.bone(self.rig.nearest_bone(at).joint);
        (to - from).normalize_or(-self.forward())
    }
}

/// Grows `fur` on `bed`, tiered to fit [`MAX_TRIANGLES`], every zone on a
/// stream of its own drawn from `seed`.
pub(crate) fn grow(bed: &Bed, fur: &Fur, seed: i64) -> FurGrowth {
    let mut share = 1.0;
    let mut growth = grow_share(bed, fur, seed, share);
    for pass in 0..TIER_PASSES + LAST_PASSES {
        let tris = growth.tris();
        if tris <= MAX_TRIANGLES {
            break;
        }
        let aim = if pass < TIER_PASSES {
            TIER_AIM
        } else {
            LAST_AIM
        };
        share *= MAX_TRIANGLES as f32 / tris as f32 * aim;
        growth = grow_share(bed, fur, seed, share);
    }
    growth
}

/// How far off its line a mane's colour reaches into the skin, in a limp card's
/// half widths at its root: past where a card, thin there, leaves the neck.
/// Provenance: **tuned by render** (#389).
const ROOTS_REACH: f32 = 1.5;

/// The skin a mane in a coat's dark grows out of, for the painter, if it has one:
/// on any other coat a card is the colour of the skin it leaves.
pub(crate) fn mane_roots(bed: &Bed, fur: &Fur) -> Option<ManeRoots> {
    if fur.mane <= 0.0 || !darkens_points(bed.coat) {
        return None;
    }
    let line = Crest::of(bed)?.line;
    let last = line.len().saturating_sub(1).max(1) as f32;
    let reach =
        FurZone::Mane.cut().width * bed.anatomy.height * fur.mane.min(1.0).sqrt() * ROOTS_REACH;
    Some(ManeRoots::new(
        line.iter()
            .enumerate()
            .map(|(point, crested)| (crested.at, reach * thins(point as f32 / last).sqrt()))
            .collect(),
    ))
}

/// Grows every zone the record asks for, at `share` of its count.
fn grow_share(bed: &Bed, fur: &Fur, seed: i64, share: f32) -> FurGrowth {
    let sampler = CoatSampler::new(bed.coat, bed.anatomy);
    let mut growth = FurGrowth::default();
    for zone in FurZone::ALL {
        let length = zone.length_in(fur);
        if length <= 0.0 {
            continue;
        }
        // At least one card in a zone asked for, as a person's thinned region
        // keeps one: a zone that grows keeps growing.
        let count = ((zone.cut().count as f32 * share).round() as usize).max(1);
        let mut stream = Pcg64Mcg::seed_from_u64(stream_seed(seed, zone));
        let line = (zone == FurZone::Mane).then(|| Crest::of(bed)).flatten();
        let lock = Lock::new(zone, bed, line.as_ref(), length, fur.mane_stiffness);
        let roots = match zone {
            FurZone::Mane => line
                .as_ref()
                .map_or_else(Vec::new, |line| crest(bed, line, count, &mut stream)),
            FurZone::Tufts => tufts(bed, count),
            _ => scatter(bed, zone, count, &mut stream),
        };
        let before = growth.mesh.face_count();
        let mut clumps = 0;
        for (root, at_zone) in &roots {
            let colour = if zone == FurZone::Tufts {
                sampler.dark()
            } else if zone.points() {
                sampler.points(root.at, root.out, *at_zone)
            } else {
                sampler.colour(root.at, root.out, *at_zone)
            };
            // Tipped a shade lighter, as fur in the light is.
            let tips = colour.lerp(Vec3::ONE, 0.08);
            if loft(&mut growth.mesh, root, &lock, None, colour, tips) > 0 {
                clumps += 1;
            }
        }
        let tris: usize = (before..growth.mesh.face_count())
            .map(|face| growth.mesh.faces[face].len().saturating_sub(2))
            .sum();
        if clumps > 0 {
            growth.grown.push(GrownZone { zone, clumps, tris });
        }
    }
    growth
}

/// The seed of one zone's stream: the record's, mixed with the zone's own.
fn stream_seed(seed: i64, zone: FurZone) -> u64 {
    let salt = FurZone::ALL
        .iter()
        .position(|&each| each == zone)
        .unwrap_or(0) as u64
        + 1;
    (seed as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ salt.wrapping_mul(0xd1b5_4a32_d192_ed03)
}

/// Scatters `count` roots over one zone of the body, each with the body zone
/// it landed in.
///
/// [`super::clump::scatter::scatter`]'s stratified sample with its mask
/// swapped for the zone's: a face holds roots in proportion to its area
/// times the zone's weight there, and a root is redrawn inside its face
/// until the weight where it lands accepts it.
fn scatter(bed: &Bed, zone: FurZone, count: usize, stream: &mut Pcg64Mcg) -> Vec<(Root, Zone)> {
    if count == 0 {
        return Vec::new();
    }
    let mesh = bed.body;
    let normals = mesh.shading_normals();
    let weighed: Vec<f32> = (0..mesh.vertex_count())
        .map(|vertex| {
            zone.weight(
                bed,
                mesh.positions[vertex],
                normals[vertex],
                bed.zones[vertex],
            )
        })
        .collect();
    let mut patches: Vec<(usize, f32)> = Vec::new();
    let mut total = 0.0;
    for face in 0..mesh.face_count() {
        let corners = &mesh.faces[face];
        if corners.len() < 3 {
            continue;
        }
        let weight = corners
            .iter()
            .map(|&corner| weighed[corner as usize])
            .sum::<f32>()
            / corners.len() as f32;
        // Against nothing rather than a fixed epsilon: a face's claim is
        // square metres, and on a beast a few centimetres tall a whole
        // forelock's faces claimed less than `f32::EPSILON` between them.
        let claim = area_of(mesh, face) * weight;
        if claim <= 0.0 {
            continue;
        }
        total += claim;
        patches.push((face, total));
    }
    if patches.is_empty() || total <= 0.0 {
        return Vec::new();
    }
    let step = total / count as f32;
    let mut roots = Vec::with_capacity(count);
    for index in 0..count {
        let jitter = stream.random_range(-0.5..0.5) * JITTER;
        let wanted = (step * (index as f32 + 0.5 + jitter)).clamp(0.0, total);
        let slot = patches.partition_point(|(_, upto)| *upto <= wanted);
        let (face, _) = patches[slot.min(patches.len() - 1)];
        // A seat's zone is its nearest corner's, not its face's first: a face
        // that starts on the neck may run onto a foreleg, and weighed as the
        // neck a seat there grew a ruff card on the leg.
        let zone_at = |at: Vec3| {
            let corners = &mesh.faces[face];
            let nearest = corners
                .iter()
                .min_by(|a, b| {
                    mesh.positions[**a as usize]
                        .distance_squared(at)
                        .total_cmp(&mesh.positions[**b as usize].distance_squared(at))
                })
                .copied()
                .unwrap_or(corners[0]);
            bed.zones[nearest as usize]
        };
        let mut seat = inside(mesh, &normals, bed.weights, face, stream);
        let mut at_zone = zone_at(seat.0);
        let mut weight = zone.weight(bed, seat.0, seat.1, at_zone);
        for _ in 0..RETRIES {
            if weight > stream.random_range(0.0..1.0) {
                break;
            }
            seat = inside(mesh, &normals, bed.weights, face, stream);
            at_zone = zone_at(seat.0);
            weight = zone.weight(bed, seat.0, seat.1, at_zone);
        }
        if weight <= f32::EPSILON {
            continue;
        }
        roots.push((
            Root {
                at: seat.0,
                out: seat.1,
                weight,
                skin: seat.2,
            },
            at_zone,
        ));
    }
    roots
}

/// One point of a neck's crest.
#[derive(Clone, Copy, Debug)]
pub(super) struct Crested {
    /// Where the crest's skin is.
    pub at: Vec3,
    /// Up off the neck's bone there.
    pub up: Vec3,
    /// Along the crest toward the withers.
    pub run: Vec3,
    /// How far the skin stands off the bone, round from the crest: the greater
    /// of the neck's two sides, every [`Crest::ROUND`] over [`Crest::RAYS`].
    pub skin: [f32; Crest::RAYS],
}

impl Crested {
    /// How far the skin stands off the bone `turned` radians round from the
    /// crest.
    pub(super) fn skin_at(&self, turned: f32) -> f32 {
        let at = (turned / Crest::ROUND).clamp(0.0, (Crest::RAYS - 1) as f32);
        let ray = (at as usize).min(Crest::RAYS - 2);
        self.skin[ray] + (self.skin[ray + 1] - self.skin[ray]) * (at - ray as f32)
    }
}

/// The line along a neck's crest that a mane grows from and lies along.
///
/// **Read off the rig, then off the skin.** A neck is a tube whose rings run
/// far apart down its length, and on a horse's long neck the crest showed on
/// two of them; a mask over its faces rooted the whole mane at the withers.
/// The rig knows the neck's line (the chest's joint, the neck's, the occiput)
/// and each joint's radius says about how far above its bone the crest runs,
/// but only about: a deep neck's skin stands well over it, and a card rooted
/// on the rig's line starts inside the neck and comes out wherever the nearest
/// ring lets it (#386). So the skin is asked where it is: straight up off the
/// bone, at every point of the line.
///
/// Evened out along its length, because the bones are not even: each is
/// straight and the next one turns.
pub(super) struct Crest {
    /// From the trunk's joint up to the occiput.
    pub(super) line: Vec<Crested>,
}

impl Crest {
    /// How many points a crest is read at.
    const POINTS: usize = 21;

    /// How many times it is evened out.
    const EVENED: usize = 3;

    /// How many rays read the neck's section at each point.
    pub(super) const RAYS: usize = 7;

    /// How far apart they are, in radians.
    pub(super) const ROUND: f32 = 0.3;

    /// How far along the bone from the neck joint to the skull the crest ends.
    /// Provenance: **tuned by render** (#388).
    const POLL: f32 = 0.70;

    /// `bed`'s crest, if its body has a neck.
    pub(super) fn of(bed: &Bed) -> Option<Self> {
        let rig = bed.rig;
        // From the head's first joint down the neck to the trunk's.
        let occiput = rig.joints.iter().position(|joint| {
            joint.zone == Zone::Head
                && joint
                    .parent
                    .is_some_and(|parent| rig.joints[parent].zone == Zone::Neck)
        })?;
        let mut chain = vec![occiput];
        let mut at = occiput;
        while let Some(parent) = rig.joints[at].parent {
            chain.push(parent);
            if rig.joints[parent].zone != Zone::Neck {
                break;
            }
            at = parent;
        }
        chain.reverse();
        if chain.len() < 2 {
            return None;
        }
        // Each joint: where it is, and its bone's own up.
        let joints: Vec<(Vec3, Vec3)> = chain
            .iter()
            .map(|&joint| {
                let here = rig.joints[joint].position;
                (here, bed.dorsal(here))
            })
            .collect();
        let lengths: Vec<f32> = joints
            .windows(2)
            .map(|pair| pair[0].0.distance(pair[1].0))
            .collect();
        let total: f32 = lengths.iter().sum();
        if total <= f32::EPSILON {
            return None;
        }
        // Only what a ray from inside the neck can meet first: the legs, the
        // rump and the tail are most of the body and none of its neck.
        let triangles: Vec<[Vec3; 3]> = bed
            .body
            .triangulated()
            .iter()
            .filter(|corners| {
                corners.iter().any(|&corner| {
                    !matches!(
                        bed.zones[corner as usize],
                        Zone::Pelvis | Zone::LowerLimb(_) | Zone::Extremity(_) | Zone::Tail
                    )
                })
            })
            .map(|corners| corners.map(|corner| bed.body.positions[corner as usize]))
            .collect();
        // First the crest itself: where each point of the bone, looking up,
        // leaves the skin.
        let mut bones = Vec::with_capacity(Self::POINTS);
        let mut crest: Vec<Vec3> = Vec::with_capacity(Self::POINTS);
        for point in 0..Self::POINTS {
            // Where along the line, and which of its pieces that is.
            let mut left = point as f32 / (Self::POINTS - 1) as f32 * total;
            let mut piece = 0;
            while piece + 1 < lengths.len() && left > lengths[piece] {
                left -= lengths[piece];
                piece += 1;
            }
            let t = (left / lengths[piece].max(f32::EPSILON)).clamp(0.0, 1.0);
            let ((from, up_from), (to, up_to)) = (joints[piece], joints[piece + 1]);
            let bone = from.lerp(to, t);
            let up = up_from.lerp(up_to, t).normalize_or(Vec3::Y);
            let reach = first_exit(&triangles, bone, up).unwrap_or(rig.joints[chain[piece]].radius);
            bones.push(bone);
            crest.push(bone + up * reach);
        }
        // Up to the poll, and no further: past it a line stops being a crest
        // and starts climbing the back of the head, where a card has no neck to
        // lie on. Read off the bones rather than the section, because the neck
        // grows into the head it carries (#388), so nothing narrows there.
        let neck_run: f32 = lengths[..lengths.len() - 1].iter().sum();
        let poll = (neck_run + Self::POLL * lengths[lengths.len() - 1]) / total;
        let poll = ((poll * (Self::POINTS - 1) as f32).round() as usize).min(Self::POINTS - 1);
        bones.truncate(poll + 1);
        crest.truncate(poll + 1);
        for _ in 0..Self::EVENED {
            crest = even(&crest);
        }
        // Then its run, and square to that its up: on a bone's own up the
        // cards fan apart wherever the neck's bones turn, however smoothly.
        let mut runs: Vec<Vec3> = (0..crest.len())
            .map(|point| {
                let (down, on) = (point.saturating_sub(1), (point + 1).min(crest.len() - 1));
                (crest[down] - crest[on]).normalize_or(Vec3::NEG_Z)
            })
            .collect();
        for _ in 0..Self::EVENED {
            runs = even(&runs);
        }
        let line = (0..crest.len())
            .map(|point| {
                let run = runs[point].normalize_or(Vec3::NEG_Z);
                let off = crest[point] - bones[point];
                let up = (off - run * off.dot(run)).normalize_or(Vec3::Y);
                let up = (up - Vec3::X * up.x).normalize_or(Vec3::Y);
                // The middle of the neck's section there, and how far each way
                // round from the crest its skin stands off that.
                let reach = off.dot(up).max(f32::EPSILON);
                let middle = crest[point] - up * reach;
                // Evened out, a line along a neck that hollows on its way into
                // the head stands off the hollow: back down onto the skin (#388).
                let reach =
                    first_exit(&triangles, middle, up).map_or(reach, |skin| reach.min(skin));
                let at = middle + up * reach;
                let mut skin = [reach; Self::RAYS];
                for (ray, standing) in skin.iter_mut().enumerate().skip(1) {
                    let (sin, cos) = (ray as f32 * Self::ROUND).sin_cos();
                    let exit = |side: f32| {
                        first_exit(&triangles, middle, up * cos + Vec3::X * (side * sin))
                    };
                    *standing = exit(1.0)
                        .into_iter()
                        .chain(exit(-1.0))
                        .max_by(f32::total_cmp)
                        .unwrap_or(reach);
                }
                Crested { at, up, run, skin }
            })
            .collect();
        Some(Self { line })
    }

    /// The crest where it passes nearest `at`, read between its points.
    ///
    /// By the nearest **piece** of the line, not its nearest point and a
    /// neighbour: a reading of the second kind jumps where the nearest point
    /// changes, by more the more the line bends, and a card lying along it
    /// inherits the jump as a kink it turns over at (#388).
    #[cfg(test)]
    pub(super) fn near(&self, at: Vec3) -> Crested {
        let (piece, t) = self.place(at);
        self.between(piece, t)
    }

    /// The piece of the line nearest `at` and how far along that piece.
    fn place(&self, at: Vec3) -> (usize, f32) {
        let mut best = (0, 0.0, f32::INFINITY);
        for piece in 0..self.line.len().saturating_sub(1) {
            let (from, to) = (self.line[piece].at, self.line[piece + 1].at);
            let run = to - from;
            let t = ((at - from).dot(run) / run.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
            let off = from.lerp(to, t).distance_squared(at);
            if off < best.2 {
                best = (piece, t, off);
            }
        }
        (best.0, best.1)
    }

    /// The line a share `t` along `piece`.
    fn between(&self, piece: usize, t: f32) -> Crested {
        let here = self.line[piece];
        let Some(there) = self.line.get(piece + 1) else {
            return here;
        };
        Crested {
            at: here.at.lerp(there.at, t),
            up: here.up.lerp(there.up, t).normalize_or(here.up),
            run: here.run.lerp(there.run, t).normalize_or(here.run),
            skin: std::array::from_fn(|ray| {
                here.skin[ray] + (there.skin[ray] - here.skin[ray]) * t
            }),
        }
    }

    /// The crest `by` metres down it from where it passes nearest `from`,
    /// walked along the line, and how far past its lower end that is.
    ///
    /// **Along the line rather than off it**: a straight step from a root leaves
    /// a crest that bends, as a neck growing into its head does (#388), and which
    /// point of it is then nearest stops being a question of how far the card
    /// has come.
    pub(super) fn walked(&self, from: Vec3, by: f32) -> (Crested, f32) {
        let (mut piece, t) = self.place(from);
        let length = |piece: usize| self.line[piece].at.distance(self.line[piece + 1].at);
        if self.line.len() < 2 {
            return (self.between(0, 0.0), by.max(0.0));
        }
        // Index 0 is the withers, so down the crest is down the line.
        let mut left = by - t * length(piece);
        if left <= 0.0 {
            // Still on this piece, or above where it started.
            let mut up = -left;
            while up > length(piece) && piece + 2 < self.line.len() {
                up -= length(piece);
                piece += 1;
            }
            let share = (up / length(piece).max(f32::EPSILON)).clamp(0.0, 1.0);
            return (self.between(piece, share), 0.0);
        }
        while piece > 0 {
            piece -= 1;
            let span = length(piece);
            if left <= span {
                let share = 1.0 - left / span.max(f32::EPSILON);
                return (self.between(piece, share), 0.0);
            }
            left -= span;
        }
        (self.between(0, 0.0), left)
    }
}

/// How a mane's root weighs, a share of the way up the crest: thinning out at
/// either end, where a mane does: slowly into the withers, or its last whole card
/// is a straight edge down the shoulder, and quickly at the poll, which a mane
/// reaches. Gone before a card is too small to come out of the skin it is sunk in.
fn thins(share: f32) -> f32 {
    step(0.0, 0.3, share) * step(1.0, 0.95, share)
}

/// `count` roots along the neck's crest, from the withers up to the poll: the
/// line a mane grows from.
///
/// A root sits where [`Crest`] passes, takes the binding of the surface
/// nearest it, and is parted to one side or the other, card by card.
fn crest(bed: &Bed, line: &Crest, count: usize, stream: &mut Pcg64Mcg) -> Vec<(Root, Zone)> {
    let normals = bed.body.shading_normals();
    let mut roots = Vec::with_capacity(count);
    for index in 0..count {
        let jitter = stream.random_range(-0.5..0.5) * JITTER;
        let share = ((index as f32 + 0.5 + jitter) / count as f32).clamp(0.0, 1.0);
        let weight = thins(share);
        if weight < THINNEST {
            continue;
        }
        let along = share * (line.line.len() - 1) as f32;
        let piece = (along as usize).min(line.line.len() - 2);
        let t = along - piece as f32;
        let (from, to) = (line.line[piece], line.line[piece + 1]);
        let at = from.at.lerp(to.at, t);
        let up = from.up.lerp(to.up, t).normalize_or(Vec3::Y);
        let Some(nearest) = (0..bed.body.vertex_count()).min_by(|&a, &b| {
            bed.body.positions[a]
                .distance_squared(at)
                .total_cmp(&bed.body.positions[b].distance_squared(at))
        }) else {
            return roots;
        };
        // Parted card by card, so that a mane falls both ways evenly and no
        // run of cards leaves a side bare. The card reads its side off this.
        let parting = if index % 2 == 0 { PARTING } else { -PARTING };
        let faces = normals[nearest] - Vec3::X * normals[nearest].x;
        roots.push((
            Root {
                at,
                out: ((faces + up).normalize_or(up) + Vec3::X * parting).normalize_or(up),
                weight,
                skin: bed.weights.vertices[nearest],
            },
            Zone::Neck,
        ));
    }
    roots
}

/// `line` with each of its inner points moved toward its neighbours.
fn even(line: &[Vec3]) -> Vec<Vec3> {
    let mut evened = line.to_vec();
    for point in 1..line.len().saturating_sub(1) {
        evened[point] = (line[point - 1] + line[point] * 2.0 + line[point + 1]) * 0.25;
    }
    evened
}

/// How far along `ray` from `from` it first crosses a triangle.
fn first_exit(triangles: &[[Vec3; 3]], from: Vec3, ray: Vec3) -> Option<f32> {
    triangles
        .iter()
        .filter_map(|&[a, b, c]| moller(from, ray, a, b, c).map(|(distance, _, _)| distance))
        .min_by(f32::total_cmp)
}

/// `count` roots at each ear's tip, splayed round the ear's own run and
/// bound to the joint at its tip, so a tuft swings with its ear.
fn tufts(bed: &Bed, count: usize) -> Vec<(Root, Zone)> {
    let Some(ears) = bed.ears else {
        return Vec::new();
    };
    let head = bed.rig.joints[ears.head].position;
    let mut roots = Vec::new();
    for ear in [&ears.left, &ears.right] {
        let Some(joint) = ear.tip_joint.or(ear.joint) else {
            continue;
        };
        let run = (ear.tip - ear.base).normalize_or(Vec3::Y);
        let (across, other) = run.any_orthonormal_pair();
        let mut skin = VertexSkin::default();
        skin[0] = Influence {
            joint: joint as u16,
            weight: 1.0,
        };
        let at = head + ear.tip - run * (0.01 * bed.anatomy.height);
        for index in 0..count {
            let turn = std::f32::consts::TAU * index as f32 / count.max(1) as f32;
            let splay = (across * turn.cos() + other * turn.sin()) * 0.35;
            roots.push((
                Root {
                    at,
                    out: (run + splay).normalize_or(run),
                    weight: 1.0,
                    skin,
                },
                Zone::Head,
            ));
        }
    }
    roots
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Agouti, Family, Pigment, QuadrupedParams};
    use crate::rig::Role;
    use crate::{Archetype, Avatar, AvatarRecord};

    /// A beast of `family` with its family's parts, wearing `fur`.
    fn built(family: Family, fur: Fur, coat: Coat) -> Avatar {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        params.fur = fur;
        params.coat = coat;
        Avatar::build(&AvatarRecord::new("Beast", Archetype::Quadruped(params)))
            .expect("the beast builds")
    }

    /// Every zone at `length`.
    fn all(length: f32) -> Fur {
        Fur {
            mane: length,
            ruff: length,
            tail: length,
            feathering: length,
            tufts: length,
            ..Fur::default()
        }
    }

    fn zones_of(avatar: &Avatar) -> Vec<FurZone> {
        avatar.parts.fur.as_ref().map_or_else(Vec::new, |fur| {
            fur.grown.iter().map(|grown| grown.zone).collect()
        })
    }

    #[test]
    fn a_bald_beast_grows_no_fur_and_a_furry_one_grows_what_it_asks_for() {
        let bald = built(Family::Dog, Fur::default(), Coat::default());
        assert!(
            bald.parts.fur.is_none(),
            "a beast that asks for none grew fur"
        );
        let brushed = built(
            Family::Dog,
            Fur {
                tail: 0.8,
                ..Fur::default()
            },
            Coat::default(),
        );
        assert_eq!(zones_of(&brushed), [FurZone::Tail]);
        assert_eq!(
            zones_of(&built(Family::Dog, all(1.0), Coat::default())),
            FurZone::ALL,
            "a zone asked for grew nothing"
        );
    }

    #[test]
    fn fur_rides_the_bones_it_grows_off() {
        // Bound as the skin it grows from all the way down, never handed over
        // to the head as a beard is: every quad of a card carries its root's
        // one binding, a brush rides the tail and a tuft its ear's spring.
        let avatar = built(Family::Cat, all(1.0), Coat::default());
        let fur = avatar.parts.fur.as_ref().expect("fur");
        let mesh = &fur.mesh;
        let held = |vertex: u32| {
            let strongest = mesh.skin[vertex as usize]
                .iter()
                .max_by(|a, b| a.weight.total_cmp(&b.weight))
                .expect("a binding");
            &avatar.rig.joints[strongest.joint as usize]
        };
        // The ledger's zones in order, each a run of the mesh's quads.
        let mut face = 0;
        let (mut brush, mut on_tail) = (0, 0);
        for grown in &fur.grown {
            let faces = face..face + grown.tris / 2;
            face = faces.end;
            for corners in faces.map(|face| &mesh.faces[face]) {
                let binding = mesh.skin[corners[0] as usize];
                assert!(
                    corners
                        .iter()
                        .all(|&corner| mesh.skin[corner as usize] == binding),
                    "a {} card changes its binding along its length",
                    grown.zone.name()
                );
                let joint = held(corners[0]);
                match grown.zone {
                    // Near the dock the skin is the croup's as much as the
                    // tail's, and the cards there are held as it is.
                    FurZone::Tail => {
                        brush += 1;
                        on_tail += usize::from(joint.zone == Zone::Tail);
                    }
                    FurZone::Tufts => {
                        assert_eq!(joint.role, Role::Spring, "a tuft off its ear's spring");
                    }
                    _ => {}
                }
            }
        }
        assert!(
            on_tail * 5 >= brush * 4,
            "only {on_tail} of a brush's {brush} quads ride the tail"
        );
    }

    #[test]
    fn the_ceiling_holds_at_the_envelope_s_end() {
        for family in Family::ALL {
            let avatar = built(family, all(10.0), Coat::default());
            let fur = avatar.parts.fur.as_ref().expect("fur");
            assert!(
                fur.tris() <= MAX_TRIANGLES,
                "{family:?}'s fur costs {} against {MAX_TRIANGLES}",
                fur.tris()
            );
            assert_eq!(
                fur.mesh.triangulated().len(),
                fur.tris(),
                "the ledger is not the mesh"
            );
        }
    }

    #[test]
    fn a_bay_s_mane_and_tail_are_its_dark_and_a_chestnut_s_its_red() {
        let mane_and_tail = Fur {
            mane: 1.0,
            tail: 1.0,
            ..Fur::default()
        };
        let colours = |coat: Coat| {
            let avatar = built(Family::Horse, mane_and_tail, coat);
            avatar.parts.fur.expect("fur").mesh.colours
        };
        let bay = Coat {
            pigment: Pigment::Black,
            agouti: Agouti::Bay,
            ..Coat::default()
        };
        assert!(
            colours(bay)
                .iter()
                .all(|colour| colour.max_element() < 0.25),
            "a bay's mane and tail are black, every card"
        );
        let chestnut = Coat {
            intensity: 1.0,
            ..Coat::default()
        };
        assert!(
            colours(chestnut)
                .iter()
                .all(|colour| colour.x > 0.5 && colour.x > colour.z * 2.0),
            "a chestnut's mane and tail are its red, every card"
        );
    }

    #[test]
    fn a_record_grows_the_same_fur_every_time() {
        let once = built(Family::Horse, all(1.0), Coat::default());
        let again = built(Family::Horse, all(1.0), Coat::default());
        assert_eq!(once.parts.fur, again.parts.fur);
    }

    #[test]
    fn a_ruff_thins_out_down_a_foreleg_rather_than_stopping_at_its_zone() {
        // Grown on the chest's zone and not the leg's, a ruff over the
        // chest's front stopped on the line the skin's weights draw round the
        // top of each foreleg (#376): through the band where the leg leaves
        // the trunk, a root's weight is the same whichever zone it is in.
        for family in Family::ALL {
            let avatar = built(family, all(1.0), Coat::default());
            let parts = &avatar.parts;
            let anatomy = CoatBody::of(&avatar.rig, None, None);
            let coat = Coat::default();
            let bed = Bed {
                body: &parts.body,
                zones: &parts.zones,
                rig: &avatar.rig,
                weights: &parts.weights,
                anatomy: &anatomy,
                coat: &coat,
                ears: None,
            };
            let mut ruffed = 0;
            for (&at, normal) in parts.body.positions.iter().zip(parts.body.vertex_normals()) {
                let Some(leg) = anatomy.junctions.leg(at).filter(|leg| leg.share < 1.0) else {
                    continue;
                };
                let weight = |zone| FurZone::Ruff.weight(&bed, at, normal, zone);
                let chest = weight(Zone::Chest);
                assert_eq!(
                    weight(Zone::UpperLimb(leg.limb)),
                    chest,
                    "{family:?}: the ruff stops at the zone line at {at}"
                );
                ruffed += usize::from(chest > 0.0);
            }
            assert!(ruffed > 0, "{family:?}: no ruff reaches a junction");
        }
    }

    #[test]
    fn a_ruff_does_not_run_down_a_foreleg() {
        // The other half of the blend: the ruff thins out through the band
        // where the leg leaves the trunk, and is gone below it. Grown on the
        // upper leg's zone without the leg's share against it, it ran on
        // down the front of each forearm to the carpus.
        for family in Family::ALL {
            let avatar = built(family, all(1.0), Coat::default());
            let parts = &avatar.parts;
            let anatomy = CoatBody::of(&avatar.rig, None, None);
            let coat = Coat::default();
            let bed = Bed {
                body: &parts.body,
                zones: &parts.zones,
                rig: &avatar.rig,
                weights: &parts.weights,
                anatomy: &anatomy,
                coat: &coat,
                ears: None,
            };
            let mut legs = 0;
            for ((&at, &zone), normal) in parts
                .body
                .positions
                .iter()
                .zip(&parts.zones)
                .zip(parts.body.vertex_normals())
            {
                if !matches!(zone, Zone::Chest | Zone::UpperLimb(_))
                    || anatomy.junctions.share(at) < 1.0
                {
                    continue;
                }
                let weight = FurZone::Ruff.weight(&bed, at, normal, zone);
                assert_eq!(weight, 0.0, "{family:?}: a ruff on the leg at {at}");
                legs += 1;
            }
            assert!(legs > 20, "{family:?}: only {legs} roots down the legs");
        }
    }
}
