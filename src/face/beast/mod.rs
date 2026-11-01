//! A beast's head (#367): its surface shaped onto the head its plan declares.
//!
//! [`shape`] does for a beast what `skull::refine_face` and `skull::shape` do
//! for a person, by another route. A person's skull is a set of profiles
//! measured off life; a beast's is a [`BeastHead`], a smooth union of
//! analytic parts the plan derives from its axes. So the head zone is refined
//! first, giving the surface the vertices a muzzle and a stop need, and every
//! head vertex is then moved along the ray from the head's axis to where that
//! ray leaves the declared head - faded to nothing toward the neck. The neck's
//! own upper run, sized off that same head (`Nape`), is shaped onto its sections
//! and joined to the head in the surface the rays land on (#388), so the two
//! agree where they meet and there is no crease to fade across. A neck carried
//! high hangs under its nodded head, so what the rays land on is asked of whichever
//! the vertex lies nearer ([`BeastHead::surface_through`]), and the few cells that
//! still come out of order in the crook are evened out along the surface
//! (`unfold`, #395).
//!
//! Applied to the **rest** mesh before anything is bound or unwrapped, as the
//! human skull is, so skin weights and texture charts follow it without
//! knowing it happened. The charts take one thing from here: the skin past the
//! skull's back is the neck's to chart (`chart_zones`).
//!
//! The parts that stand off the head are built against the same declaration
//! and kept together as a [`BeastFace`]: the [`ears`], the [`jaw`] with its
//! tongue, and the whiskers and the horn ([`adorn`]).

pub mod adorn;
pub mod ears;
pub mod jaw;

pub use adorn::Adornments;
pub use ears::{Ear, Ears};
pub use jaw::Jaw;

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::Vec3;

use super::smooth;
use crate::mesh::PolyMesh;
use crate::plan::{BeastHead, Zone};
use crate::rig::Rig;

/// The parts a beast's head wears (#367): its ears, its jaw and its tongue,
/// each riding a joint of its own off the head.
#[derive(Clone, Debug, PartialEq)]
pub struct BeastFace {
    /// The ears.
    pub ears: Ears,
    /// The lower jaw and the tongue.
    pub jaw: Jaw,
    /// The whiskers and the horn, drawn with the eyes.
    pub adornments: Adornments,
}

impl BeastFace {
    /// Builds every part a beast's head declares, for the head joint `head`,
    /// against `body`, the surface as built.
    #[must_use]
    pub fn build(rig: &Rig, head: usize, beast: &BeastHead, body: &PolyMesh) -> Self {
        Self {
            ears: Ears::build(rig, head, beast),
            jaw: Jaw::build(rig, head, beast, body),
            adornments: Adornments::build(beast, rig.joints[head].position),
        }
    }

    /// Hangs every part on its joint. After the body is bound, as the eyes'
    /// lids are, so nothing of the body's surface is offered to them.
    pub fn rig(&mut self, rig: &mut Rig) {
        self.ears.rig(rig);
        self.jaw.rig(rig);
    }

    /// Every part with the joint it rides, in a fixed order - the ears, the
    /// jaw, the tongue - which is the contract between whatever charts them
    /// and whatever places the charts.
    pub fn parts(&self) -> impl Iterator<Item = (&PolyMesh, usize)> {
        self.ears.parts().chain(self.jaw.parts())
    }

    /// Every part's mesh, in [`Self::parts`]' order, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        self.ears.meshes_mut().chain(self.jaw.meshes_mut())
    }

    /// Every part's mouth channel, in [`Self::parts`]' order: none for the
    /// ears, the jaw's inside and the whole tongue (#368).
    pub fn mouths(&self) -> impl Iterator<Item = Vec<f32>> {
        self.ears
            .parts()
            .map(|_| Vec::new())
            .chain(self.jaw.mouths())
    }
}

/// How many times the head zone is refined before it is shaped.
///
/// The cage hands the head over as a sixteen-sided tube, which can carry a
/// braincase and a muzzle but not the stop between them, nor a lip's lobe a
/// fifth of a radius across. Two passes put sixty-four facets round it.
/// Provenance: **tuned by render** (#367), against the triangle budget: the
/// head is the beast's face, and the budget a person spends on a face is the
/// measure of what one is worth.
const REFINEMENT: usize = 2;

/// How far round the nose, in head radii, a further pass refines: a pad a
/// fifth of a radius across with nostrils cut into it wants the vertices a
/// head's broad surface does not.
/// Provenance: **tuned by render** (#367).
const NOSE_REACH: f32 = 0.5;

/// How far from the neck, along the surface and in head radii, the shaping
/// fades in.
///
/// **The neck's vertices do not move, so whatever the head's do at the
/// junction is a step.** Faded over this distance, the head leaves the cage's
/// surface where the neck meets it and arrives at its own six tenths of a
/// radius up.
/// Provenance: **tuned by render** (#367).
const FADE: f32 = 0.60;

/// How many times the neck's upper run is refined: the cage's neck is two
/// segments long, which cannot draw its growth into the head as anything but a
/// bend. Provenance: **tuned by render** (#388).
const NAPE_REFINEMENT: usize = 2;

/// How far below the run's start a face is still refined, as a share of the
/// run: a face straddling the start is worth it.
const NAPE_MARGIN: f32 = 0.05;

/// Refines a beast's head and shapes it onto `head`.
#[must_use]
pub fn shape(mesh: &PolyMesh, rig: &Rig, head: &BeastHead) -> PolyMesh {
    let mut shaped = refine(mesh, rig, head);
    let owned = owned(&shaped, rig);
    let shares = fade(&shaped, &owned, head.radius * FADE);
    let carrying = carrying(&shaped, rig);
    let mut twins = twins(&shaped, &carrying, head);
    let cage = shaped.positions.clone();
    for ((point, share), &carries) in shaped.positions.iter_mut().zip(shares).zip(&carrying) {
        if !carries {
            continue;
        }
        // What the neck makes of the vertex; and for a vertex of the head,
        // where the ray through it lands. Cast through the neck's place for
        // it as far as the vertex is the neck's, so a vertex of the run stays
        // on the run rather than sliding along it.
        let raw = head.local(*point);
        let carried = head.carried(raw);
        if share <= 0.0 && carried == raw {
            continue;
        }
        let drawn = if share > 0.0 {
            let through = carried.lerp(raw, share);
            head.surface_through(through)
                .map_or(carried, |surface| carried.lerp(surface, share))
        } else {
            carried
        };
        *point = head.body(drawn);
    }
    // What the shaping left where the cage put it stays exactly there.
    twins.retain(|&(left, right)| {
        let moved = |vertex: u32| shaped.positions[vertex as usize] != cage[vertex as usize];
        moved(left) || moved(right)
    });
    mirror_over(&mut shaped, &twins);
    unfold(&mut shaped, head, &carrying, &twins);
    shaped
}

/// `selected`, with each selected face's image across the midline selected too.
///
/// Whether a face is refined is a threshold, and a face's two sides stand a little
/// apart on it, so now and then one side alone came out refined, with a few
/// vertices that had no twin.
fn on_both_sides(mesh: &PolyMesh, head: &BeastHead, selected: Vec<bool>) -> Vec<bool> {
    let near = TWIN_NEAR * head.radius;
    let centres: Vec<Vec3> = (0..mesh.face_count())
        .map(|face| mesh.face_centroid(face))
        .collect();
    let mut by_height: Vec<usize> = (0..mesh.face_count()).collect();
    by_height.sort_by(|&a, &b| centres[a].y.total_cmp(&centres[b].y));
    let mut both = selected.clone();
    for (face, _) in selected.iter().enumerate().filter(|&(_, &chosen)| chosen) {
        let at = centres[face];
        let image = Vec3::new(-at.x, at.y, at.z);
        let from = by_height.partition_point(|&other| centres[other].y < at.y - near);
        for &other in by_height[from..]
            .iter()
            .take_while(|&&other| centres[other].y < at.y + near)
        {
            if centres[other].distance_squared(image) < near * near {
                both[other] = true;
            }
        }
    }
    both
}

/// The head's and neck's vertices that are each other's image across the midline,
/// the left first, read off the mesh before it is shaped, while it is still the
/// cage's own. A vertex on the midline is its own twin.
fn twins(mesh: &PolyMesh, carrying: &[bool], head: &BeastHead) -> Vec<(u32, u32)> {
    let near = TWIN_NEAR * head.radius;
    let height = |vertex: u32| mesh.positions[vertex as usize].y;
    let mut by_height: Vec<u32> = (0..mesh.vertex_count() as u32)
        .filter(|&vertex| carrying[vertex as usize])
        .collect();
    by_height.sort_by(|&a, &b| height(a).total_cmp(&height(b)));
    let mut twinned = vec![false; mesh.vertex_count()];
    let mut twins = Vec::new();
    for (place, &vertex) in by_height.iter().enumerate() {
        if twinned[vertex as usize] {
            continue;
        }
        let at = mesh.positions[vertex as usize];
        let image = Vec3::new(-at.x, at.y, at.z);
        let twin = by_height[place..]
            .iter()
            .take_while(|&&other| height(other) - at.y < near)
            .find(|&&other| {
                !twinned[other as usize]
                    && mesh.positions[other as usize].distance_squared(image) < near * near
            });
        if let Some(&twin) = twin {
            twinned[vertex as usize] = true;
            twinned[twin as usize] = true;
            twins.push(if at.x <= 0.0 {
                (vertex, twin)
            } else {
                (twin, vertex)
            });
        }
    }
    twins
}

/// How near its image across the midline a vertex has to lie to be its twin, in
/// head radii: far under the shortest edge a refined head has, and far over what
/// the sums of a subdivision leave between two sides built alike.
/// Provenance: **chosen** (#395).
const TWIN_NEAR: f32 = 0.002;

/// Makes each right vertex the image of its left twin.
///
/// **A beast's head is symmetric because this says so, not because each side comes
/// out alike.** Whether a ray clips a lobe and whether a cell counts as folded are
/// thresholds, and a body's two sides stand a little apart on them, by what a
/// bisection leaves: left alone, one side of a throat evened out a fold the other
/// kept, and the two came a tenth of a head radius apart.
fn mirror_over(mesh: &mut PolyMesh, twins: &[(u32, u32)]) {
    for &(left, right) in twins {
        let at = mesh.positions[left as usize];
        mesh.positions[right as usize] = Vec3::new(-at.x, at.y, at.z);
        if left == right {
            mesh.positions[left as usize].x = 0.0;
        }
    }
}

/// Each vertex's zone for charting: the zone map's, except that the skin past the
/// back of the skull is the neck's (#395).
///
/// The zone map hands a beast's skin over to the head halfway up the neck's upper
/// bone, for its weights and its fur, and a chart of the head wraps round the bone
/// entering it. That cylinder holds a skull; the skin running back from the skull
/// down the neck comes out along its axis, streaked past seventeen to one, and
/// shows as long hard-edged shapes in any coat. It runs along the neck's own bone
/// as the neck's own chart does, so it goes with that, up to the crease where the
/// head and the run meet ([`BeastHead::on_run`]), which is a line the coat has no
/// reason to mind. Only the chart: no weight and no fur asks this.
#[must_use]
pub(crate) fn chart_zones(mesh: &PolyMesh, head: &BeastHead, zones: &[Zone]) -> Vec<Zone> {
    mesh.positions
        .iter()
        .zip(zones)
        .map(|(&point, &zone)| {
            if zone == Zone::Head && head.on_run(head.local(point)) {
                Zone::Neck
            } else {
                zone
            }
        })
        .collect()
}

/// Evens out, along the drawn surface, whatever skin came out facing inward in
/// the crook between the head and the neck's upper run (#395).
///
/// Every vertex is placed by a ray of its own, and in a tight crook a few of
/// them land out of order: the cells between face the wrong way, and the shading
/// and the coat on them read as a dark seam. A vertex there goes to the middle of
/// its neighbours and back onto the surface, which draws a fold flat and leaves
/// the surface where it was, and a body with no such cell is untouched.
fn unfold(mesh: &mut PolyMesh, head: &BeastHead, carrying: &[bool], twins: &[(u32, u32)]) {
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); mesh.vertex_count()];
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let (a, b) = (face[corner], face[(corner + 1) % face.len()]);
            neighbours[a as usize].push(b);
            neighbours[b as usize].push(a);
        }
    }
    let crook: Vec<usize> = (0..mesh.face_count())
        .filter(|&face| head.in_crook(head.local(mesh.face_centroid(face))))
        .collect();
    for _ in 0..UNFOLD_PASSES {
        let mut folded = vec![false; mesh.vertex_count()];
        let mut any = false;
        for &index in &crook {
            let face = &mesh.faces[index];
            let at = |corner: usize| head.local(mesh.positions[face[corner] as usize]);
            // Round every corner, so a cell out of plane reads the same whichever
            // corner it starts from, as its twin may not start from the same one.
            let facing = (0..face.len())
                .map(|corner| at(corner).cross(at((corner + 1) % face.len())))
                .sum::<Vec3>()
                .normalize_or_zero();
            let centre = head.local(mesh.face_centroid(index));
            if facing.dot(head.drawn_normal(centre)) < UNFOLD_FACING {
                any = true;
                for &vertex in face {
                    folded[vertex as usize] = true;
                }
            }
        }
        if !any {
            return;
        }
        for _ in 0..UNFOLD_MARGIN {
            let grown = folded.clone();
            for (vertex, near) in neighbours.iter().enumerate() {
                if near.iter().any(|&other| grown[other as usize]) {
                    folded[vertex] = true;
                }
            }
        }
        for &(left, right) in twins {
            let either = folded[left as usize] || folded[right as usize];
            folded[left as usize] = either;
            folded[right as usize] = either;
        }
        let placed: Vec<(usize, Vec3)> = (0..mesh.vertex_count())
            .filter(|&vertex| {
                folded[vertex]
                    && carrying[vertex]
                    && head.in_crook(head.local(mesh.positions[vertex]))
            })
            .map(|vertex| {
                let near = &neighbours[vertex];
                let mean = near
                    .iter()
                    .map(|&other| mesh.positions[other as usize])
                    .sum::<Vec3>()
                    / near.len().max(1) as f32;
                let was = head.local(mesh.positions[vertex]);
                let off = head.drawn_distance(was).abs() / (ON_SURFACE * head.radius);
                let on = 1.0 - smooth(off);
                (
                    vertex,
                    head.body(was.lerp(head.settled(head.local(mean)), on)),
                )
            })
            .collect();
        for (vertex, at) in placed {
            mesh.positions[vertex] = at;
        }
        mirror_over(mesh, twins);
    }
}

/// How far off the drawn surface, in head radii, a vertex is no longer
/// [`unfold`]'s to move: the neck below its upper run stands off the surface the
/// head declares, and fading out by this keeps the neck's join where it is.
/// Provenance: **chosen** (#395): a head radius's quarter, where the run starts.
const ON_SURFACE: f32 = 0.25;

/// How many times [`unfold`] evens a fold out, at most.
/// Provenance: **tuned against the inward-face census** (#395).
const UNFOLD_PASSES: usize = 40;

/// How far past a folded cell's corners, in edges, [`unfold`] draws the skin
/// with it, so a fold does not just move next door.
/// Provenance: **tuned against the inward-face census** (#395).
const UNFOLD_MARGIN: usize = 2;

/// How little of a cell's facing may lie along the surface's outward normal
/// before it counts as folded. Provenance: **tuned against the inward-face
/// census** (#395).
const UNFOLD_FACING: f32 = 0.1;

/// Whether each vertex is the head's or the neck's: nothing of the trunk is
/// shaped here.
fn carrying(mesh: &PolyMesh, rig: &Rig) -> Vec<bool> {
    mesh.positions
        .iter()
        .map(|&point| {
            let zone = rig.joints[rig.nearest_bone(point).joint].zone;
            zone == Zone::Head || zone == Zone::Neck
        })
        .collect()
}

/// Whether each vertex is the head's, asked of the rig as `skull::shape`
/// asks it: the neck runs up into the same space, and a throat shaped as a
/// jaw pinches away from the head it is meant to meet.
fn owned(mesh: &PolyMesh, rig: &Rig) -> Vec<bool> {
    mesh.positions
        .iter()
        .map(|&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
        .collect()
}

/// The head zone, [`REFINEMENT`] times over, and the nose's own region once
/// more. Curved, because the tube's facets are chords of a round surface
/// and a flat split would keep them.
fn refine(mesh: &PolyMesh, rig: &Rig, head: &BeastHead) -> PolyMesh {
    let headed = |mesh: &PolyMesh, face: usize| {
        let centre = mesh.face_centroid(face);
        rig.joints[rig.nearest_bone(centre).joint].zone == Zone::Head
    };
    let mut refined = mesh.clone();
    for _ in 0..REFINEMENT {
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| headed(&refined, face))
            .collect();
        refined = refined.refine_curved(&on_both_sides(&refined, head, selected));
    }
    let nose = head.nose_centre();
    let selected: Vec<bool> = (0..refined.face_count())
        .map(|face| {
            headed(&refined, face)
                && head.local(refined.face_centroid(face)).distance(nose) < NOSE_REACH * head.radius
        })
        .collect();
    let mut refined = refined.refine_curved(&on_both_sides(&refined, head, selected));
    // And the neck's upper run.
    for _ in 0..NAPE_REFINEMENT {
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| {
                let centre = refined.face_centroid(face);
                rig.joints[rig.nearest_bone(centre).joint].zone == Zone::Neck
                    && head
                        .along_nape(head.local(centre))
                        .is_some_and(|along| along > -NAPE_MARGIN)
            })
            .collect();
        refined = refined.refine_curved(&on_both_sides(&refined, head, selected));
    }
    refined
}

/// Each vertex's share of the shaping: none off the head, rising smoothly to
/// all of it `reach` along the surface from the nearest vertex that is not
/// the head's.
///
/// Measured along the mesh's edges rather than straight through the body, so
/// a vertex on the far side of the jaw from the throat is not faded for being
/// near it through the head.
pub(crate) fn fade(mesh: &PolyMesh, owned: &[bool], reach: f32) -> Vec<f32> {
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); mesh.vertex_count()];
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let (a, b) = (face[corner], face[(corner + 1) % face.len()]);
            neighbours[a as usize].push(b);
            neighbours[b as usize].push(a);
        }
    }
    // Dijkstra from every vertex that is not the head's at once. Distances
    // are never negative, so their bits sort as the distances do.
    let mut along = vec![f32::INFINITY; mesh.vertex_count()];
    let mut queue = BinaryHeap::new();
    for (vertex, _) in owned.iter().enumerate().filter(|(_, mine)| !**mine) {
        along[vertex] = 0.0;
        queue.push(Reverse((0.0f32.to_bits(), vertex as u32)));
    }
    while let Some(Reverse((bits, vertex))) = queue.pop() {
        let at = f32::from_bits(bits);
        if at > along[vertex as usize] || at >= reach {
            continue;
        }
        let from = mesh.positions[vertex as usize];
        for &next in &neighbours[vertex as usize] {
            let step = at + from.distance(mesh.positions[next as usize]);
            if step < along[next as usize] {
                along[next as usize] = step;
                queue.push(Reverse((step.to_bits(), next)));
            }
        }
    }
    along
        .iter()
        .map(|&distance| smooth(distance / reach))
        .collect()
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, Composites, QuadrupedParams};
    use crate::skeleton::Skeleton;
    use crate::subdiv::catmull_clark;

    /// A quadruped's skeleton, its subdivided cage before any shaping, and
    /// its rig.
    fn beast(params: &QuadrupedParams) -> (Skeleton, PolyMesh, Rig) {
        let skeleton = params.skeleton(&Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        (skeleton, mesh, rig)
    }

    /// The head a quadruped with `params` declares.
    fn declared(params: &QuadrupedParams) -> BeastHead {
        params
            .skeleton(&Composites::default())
            .beast_head
            .expect("a quadruped declares its head")
    }

    /// How high the head's top line runs along its midline at `z`, in its
    /// own frame: marched down from above until the first point inside.
    fn top_at(head: &BeastHead, z: f32) -> f32 {
        let step = head.radius / 400.0;
        let mut y = 3.0 * head.radius;
        while head.distance(Vec3::new(0.0, y, z)) > 0.0 {
            y -= step;
            assert!(
                y > -3.0 * head.radius,
                "the midline at {z} never entered the head"
            );
        }
        y
    }

    /// How far the head reaches to its side at `z`, at the height `y`.
    fn side_at(head: &BeastHead, y: f32, z: f32) -> f32 {
        let step = head.radius / 400.0;
        let mut x = 3.0 * head.radius;
        while head.distance(Vec3::new(x, y, z)) > 0.0 {
            x -= step;
            assert!(x > 0.0, "the line at {y}, {z} never entered the head");
        }
        x
    }

    #[test]
    fn the_shaped_head_is_the_head_the_plan_declared() {
        let params = QuadrupedParams::default();
        let (skeleton, mesh, rig) = beast(&params);
        let head = skeleton.beast_head.expect("a quadruped declares its head");
        let shaped = shape(&mesh, &rig, &head);
        // Each vertex's share as the pass measured it: on the refined mesh,
        // before anything moved and changed the lengths it walks.
        let refined = refine(&mesh, &rig, &head);
        let shares = fade(&refined, &owned(&refined, &rig), head.radius * FADE);
        let mut outright = 0;
        for (point, share) in shaped.positions.iter().zip(&shares) {
            if *share < 1.0 {
                continue;
            }
            let off = head.drawn_distance(head.local(*point));
            assert!(
                off.abs() < 1.0e-3 * head.radius,
                "{point} sits {off} m off the declared head"
            );
            outright += 1;
        }
        assert!(
            outright > 1000,
            "only {outright} vertices were the head's outright"
        );
        assert!(shaped.is_closed_manifold(), "the shaped body has a hole");
    }

    #[test]
    fn the_neck_below_its_upper_run_is_left_where_the_cage_put_it() {
        // And a short neck off a deep chest, whose trunk stands past the run's
        // start: nothing of the trunk moves, however far along the run it reads.
        let mut deep = QuadrupedParams::default();
        deep.set_kind(crate::plan::Family::Cat, None, 0.0);
        deep.neck_length = -3.0;
        deep.chest_depth = 3.0;
        for params in [QuadrupedParams::default(), deep] {
            let (skeleton, mesh, rig) = beast(&params);
            let head = skeleton.beast_head.expect("a quadruped declares its head");
            let refined = refine(&mesh, &rig, &head);
            let shaped = shape(&mesh, &rig, &head);
            assert_eq!(refined.positions.len(), shaped.positions.len());
            let mut kept = 0;
            for (before, after) in refined.positions.iter().zip(&shaped.positions) {
                let run = head.along_nape(head.local(*before)).expect("a run");
                let mine = rig.joints[rig.nearest_bone(*before).joint].zone;
                if run <= 0.0 && mine != Zone::Head {
                    assert_eq!(before, after, "a vertex off the run moved");
                    kept += 1;
                }
                if mine != Zone::Head && mine != Zone::Neck {
                    assert_eq!(before, after, "the trunk at {before} moved");
                }
            }
            assert!(kept > 1000, "only {kept} vertices were off the run");
        }
    }

    // ---- The throat and the charts of the upper neck (#395).

    /// A rolled beast's record.
    fn rolled(seed: i64) -> crate::AvatarRecord {
        crate::AvatarRecord::rolled(
            "Beast",
            crate::Archetype::Quadruped(QuadrupedParams::default()),
            seed,
        )
    }

    /// A built beast. The charts do not depend on the atlas's size.
    fn built(record: &crate::AvatarRecord) -> crate::Avatar {
        let config = crate::AvatarConfig {
            atlas: 64,
            ..crate::AvatarConfig::default()
        };
        crate::Avatar::build_with(record, &config).expect("the beast builds")
    }

    /// How much skin about the head and the neck faces inward, against the bone
    /// nearest it: how many faces, and their area in the body's squared height.
    fn inward(avatar: &crate::Avatar) -> (usize, f32) {
        let (body, rig) = (&avatar.parts.body, &avatar.rig);
        let height = body.positions.iter().map(|p| p.y).fold(f32::MIN, f32::max);
        let (mut count, mut area) = (0, 0.0);
        for (index, face) in body.faces.iter().enumerate() {
            let centre = body.face_centroid(index);
            let hit = rig.nearest_bone(centre);
            if !matches!(
                rig.joints[hit.joint].zone,
                Zone::Neck | Zone::Head | Zone::Chest
            ) {
                continue;
            }
            let at = |corner: usize| body.positions[face[corner] as usize];
            let normal = (at(1) - at(0)).cross(at(face.len() - 1) - at(0));
            let out = (centre - hit.closest).normalize_or_zero();
            if normal.normalize_or_zero().dot(out) < -0.2 {
                count += 1;
                area += normal.length();
            }
        }
        (count, area / (height * height))
    }

    /// Runs `each` over `seeds` on up to ten threads, in order of the seeds.
    fn across<T: Send>(seeds: Vec<i64>, each: impl Fn(i64) -> T + Sync) -> Vec<T> {
        std::thread::scope(|scope| {
            let each = &each;
            let handles: Vec<_> = seeds
                .chunks(seeds.len().div_ceil(10).max(1))
                .map(|some| {
                    scope.spawn(move || some.iter().map(|&seed| each(seed)).collect::<Vec<_>>())
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().expect("a thread"))
                .collect()
        })
    }

    /// A table to read: `cargo test --release --lib beast::tests::throat_table
    /// -- --ignored --nocapture`.
    #[test]
    #[ignore = "a table to read, not a check"]
    fn throat_table() {
        let rows = across((0..THROAT_ROLLS).collect(), |seed| {
            let record = rolled(seed);
            let (count, area) = inward(&built(&record));
            let chart = upper_neck_charts(&record, &built(&record));
            (seed, count, area, chart)
        });
        let folded = rows.iter().filter(|row| row.1 >= FOLDED).count();
        let worst = rows.iter().map(|row| row.2).fold(0.0, f32::max);
        let stretched = rows.iter().map(|row| row.3.0).fold(0.0, f32::max);
        let thin = rows.iter().map(|row| row.3.1).fold(f32::MAX, f32::min);
        println!(
            "rolled: {folded} folded, worst {worst:.4}, stretch {stretched:.2}, thin {thin:.3}"
        );
        for (name, record) in carriage_bodies() {
            let swept = across(carriage_sweep(), |carriage| {
                let (count, area) = inward(&built(&with_carriage(&record, carriage)));
                (carriage, count, area)
            });
            println!("{name}: {swept:?}");
        }
        let charts = charted_bodies();
        let read = |zone: Zone| {
            let mut rows = across((0..charts.len() as i64).collect(), |index| {
                chart_reading(&built(&charts[index as usize]), zone)
            });
            let worst = rows.iter().fold([0.0f32; 2], |worst, row| {
                [worst[0].max(row[0]), worst[1].max(row[1])]
            });
            println!(
                "{zone:?}: worst tenth {:.2}, worst hundredth {:.2}",
                worst[0], worst[1]
            );
            let swept = rows.split_off(THROAT_ROLLS as usize / 2);
            let rolled = rows.iter().map(|row| row[1]).fold(0.0, f32::max);
            println!("{zone:?}: rolled {rolled:.2}, swept {swept:.1?}");
        };
        read(Zone::Head);
        read(Zone::Chest);
        read(Zone::Neck);
        let off = across((0..charts.len() as i64).collect(), |index| {
            let record = &charts[index as usize];
            let head = record.skeleton().beast_head.expect("a beast's head");
            unmirrored(&built(record), &head)
        });
        println!("unmirrored: {:.6}", off.iter().copied().fold(0.0, f32::max));
    }

    /// How many beasts [`throat_table`] and the checks roll.
    const THROAT_ROLLS: i64 = 160;

    /// How many inward faces make a fold, rather than a cell at a crease.
    const FOLDED: usize = 10;

    /// The bodies the carriage is swept on: a family's own from each and a cross.
    fn carriage_bodies() -> Vec<(&'static str, crate::AvatarRecord)> {
        use crate::plan::Family;
        [
            ("dog", Family::Dog, None),
            ("cat", Family::Cat, None),
            ("horse", Family::Horse, None),
            ("cat x horse", Family::Cat, Some(Family::Horse)),
        ]
        .into_iter()
        .map(|(name, family, other)| {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, other, 0.5);
            (
                name,
                crate::AvatarRecord::new("Beast", crate::Archetype::Quadruped(params)),
            )
        })
        .collect()
    }

    /// `neckCarriage` from under anything that folded to where rolls end, in the
    /// record's thousandths.
    fn carriage_sweep() -> Vec<i64> {
        (0..=28).map(|step| step * 50).collect()
    }

    /// `record` with its neck carried at `carriage` thousandths.
    fn with_carriage(record: &crate::AvatarRecord, carriage: i64) -> crate::AvatarRecord {
        let mut record = record.clone();
        if let crate::Archetype::Quadruped(params) = &mut record.archetype {
            params.neck_carriage = carriage as f32 / 1000.0;
        }
        record
    }

    #[test]
    fn the_throat_does_not_fold_on_a_rolled_beast() {
        // 114 of 600 rolled beasts folded before #395, from a carriage of .3 up,
        // and the fold's area ran up to .0797 of the height squared. See the bounds.
        let rows = across((0..THROAT_ROLLS).collect(), |seed| {
            let (count, area) = inward(&built(&rolled(seed)));
            (seed, count, area)
        });
        let folded: Vec<_> = rows.iter().filter(|row| row.1 >= FOLDED).collect();
        assert!(
            folded.len() <= ROLLED_FOLDED,
            "{} of {THROAT_ROLLS} rolled beasts fold at the throat: {folded:?}",
            folded.len()
        );
        for row in &rows {
            assert!(row.2 < ROLLED_AREA, "seed {}: {} inward", row.0, row.2);
        }
    }

    /// See [`the_throat_does_not_fold_on_a_rolled_beast`]. Set after reading
    /// `throat_table`: no beast folds and the worst is .0005 in these rolls. With
    /// the cast undone 8 fold, up to .0165, and with [`unfold`] undone 28.
    const ROLLED_FOLDED: usize = 1;
    const ROLLED_AREA: f32 = 0.002;

    #[test]
    fn the_throat_does_not_fold_as_the_neck_rises() {
        for (name, record) in carriage_bodies() {
            let swept = across(carriage_sweep(), |carriage| {
                let (count, area) = inward(&built(&with_carriage(&record, carriage)));
                (carriage, count, area)
            });
            for (carriage, count, area) in swept {
                assert!(
                    area < SWEPT_AREA,
                    "{name} at {carriage}: {count} faces, {area} inward"
                );
            }
        }
    }

    /// See [`the_throat_does_not_fold_as_the_neck_rises`]. Set after reading
    /// `throat_table`: the worst is .0171, a cat at the top of the sweep. Any one
    /// of the cast, [`unfold`] and the straightened columns undone comes over this,
    /// at .0225, .0236 and .0288.
    const SWEPT_AREA: f32 = 0.022;

    /// What the atlas does to the skin of one face of a chart: the stretch, the
    /// density it keeps, and the face's area. `None` for a face with no area.
    fn face_on_chart(
        uv: &crate::uv::UvUnwrap,
        body: &PolyMesh,
        face: &[u32],
    ) -> Option<(f32, f32, f32)> {
        let corners = [face[0], face[1], face[2]];
        let at = corners.map(|v| body.positions[uv.source[v as usize] as usize]);
        let (edge, other) = (at[1] - at[0], at[2] - at[0]);
        let normal = edge.cross(other);
        if normal.length_squared() < 1e-20 {
            return None;
        }
        let (x, y) = (edge.normalize(), normal.cross(edge).normalize());
        let flat = glam::Mat2::from_cols(
            Vec2::new(edge.dot(x), edge.dot(y)),
            Vec2::new(other.dot(x), other.dot(y)),
        );
        let atlas = corners.map(|v| uv.uvs[v as usize]);
        let map = glam::Mat2::from_cols(atlas[1] - atlas[0], atlas[2] - atlas[0]) * flat.inverse();
        let sum = map.x_axis.length_squared() + map.y_axis.length_squared();
        let area = map.determinant().abs();
        let spread = (sum * sum - 4.0 * area * area).max(0.0).sqrt();
        let most = ((sum + spread) / 2.0).sqrt();
        let least = ((sum - spread) / 2.0).max(1e-9).sqrt();
        Some((most / least, area.sqrt(), normal.length()))
    }

    /// What `key` stays under over the share `at` of `rows`' skin by area.
    fn by_area(rows: &mut [(f32, f32, f32)], key: fn(&(f32, f32, f32)) -> f32, at: f32) -> f32 {
        let total: f32 = rows.iter().map(|row| row.2).sum();
        rows.sort_by(|a, b| key(a).total_cmp(&key(b)));
        let mut run = 0.0;
        for row in rows.iter() {
            run += row.2;
            if run >= at * total {
                return key(row);
            }
        }
        0.0
    }

    /// How the charts treat the skin of the upper neck, off the head: the
    /// stretch that the worst hundredth of it by area reaches, and the density
    /// its thinnest twentieth by area keeps as a share of its median.
    fn upper_neck_charts(record: &crate::AvatarRecord, avatar: &crate::Avatar) -> (f32, f32) {
        let skeleton = record.skeleton();
        let head = skeleton.beast_head.expect("a beast's head");
        let (uv, body) = (&avatar.parts.unwrap, &avatar.parts.body);
        let mut rows: Vec<(f32, f32, f32)> = Vec::new();
        for face in &uv.faces {
            let at =
                [face[0], face[1], face[2]].map(|v| body.positions[uv.source[v as usize] as usize]);
            let centre = head.local((at[0] + at[1] + at[2]) / 3.0);
            if head.distance(centre) < 0.15 * head.radius
                || !head.along_nape(centre).is_some_and(|u| u > 0.2)
                || head.drawn_distance(centre).abs() > 0.1 * head.radius
            {
                continue;
            }
            rows.extend(face_on_chart(uv, body, face));
        }
        let stretch = by_area(&mut rows, |row| row.0, 0.99);
        let (thin, median) = (
            by_area(&mut rows, |row| row.1, 0.05),
            by_area(&mut rows, |row| row.1, 0.5),
        );
        (stretch, thin / median.max(1e-9))
    }

    /// The stretch of a zone's charts, all of them: what the worst tenth of its
    /// skin by area stays under, and the worst hundredth.
    fn chart_reading(avatar: &crate::Avatar, zone: Zone) -> [f32; 2] {
        let (uv, body) = (&avatar.parts.unwrap, &avatar.parts.body);
        let mut rows: Vec<(f32, f32, f32)> = uv
            .faces
            .iter()
            .enumerate()
            .filter(|&(index, _)| uv.charts[uv.chart_of_face[index] as usize].zone == zone)
            .filter_map(|(_, face)| face_on_chart(uv, body, face))
            .collect();
        [
            by_area(&mut rows, |row| row.0, 0.9),
            by_area(&mut rows, |row| row.0, 0.99),
        ]
    }

    /// How far the head and the neck's upper run stand from their own mirror
    /// image: for each vertex within two head radii of the head, how far its image
    /// is from the nearest vertex, in head radii, at worst.
    fn unmirrored(avatar: &crate::Avatar, head: &BeastHead) -> f32 {
        let mut near: Vec<Vec3> = avatar
            .parts
            .body
            .positions
            .iter()
            .copied()
            .filter(|&at| head.local(at).length() < 2.0 * head.radius)
            .collect();
        near.sort_by(|a, b| a.y.total_cmp(&b.y));
        let reach = 0.3 * head.radius;
        let mut worst: f32 = 0.0;
        for at in &near {
            let image = Vec3::new(-at.x, at.y, at.z);
            let from = near.partition_point(|other| other.y < at.y - reach);
            let nearest = near[from..]
                .iter()
                .take_while(|other| other.y < at.y + reach)
                .map(|other| other.distance(image))
                .fold(reach, f32::min);
            worst = worst.max(nearest);
        }
        worst / head.radius
    }

    #[test]
    fn a_beast_s_head_stays_mirror_symmetric() {
        // Each side's folds were evened out by a threshold of its own, and the two
        // came .115 of a head radius apart on a rolled beast and .144 on a swept
        // one (#395). A face refined on one side alone reads here too, as a vertex
        // with no image. See the bound.
        let bodies = charted_bodies();
        let rows = across((0..bodies.len() as i64).collect(), |index| {
            let record = &bodies[index as usize];
            let head = record.skeleton().beast_head.expect("a beast's head");
            (index, unmirrored(&built(record), &head))
        });
        for (index, off) in rows {
            assert!(off < UNMIRRORED, "body {index}: {off} of a head radius");
        }
    }

    /// See [`a_beast_s_head_stays_mirror_symmetric`]. Set after reading `throat_table`:
    /// the worst is under a millionth of a head radius, over 300 rolls as well. This
    /// is a little over how far apart [`TWIN_NEAR`] lets a pair of twins start.
    const UNMIRRORED: f32 = 0.005;

    /// The bodies the charts are judged on: rolled beasts, and the swept bodies
    /// with their necks carried from low to the top of the envelope, where a
    /// head's underside faces straight down the neck.
    fn charted_bodies() -> Vec<crate::AvatarRecord> {
        let mut bodies: Vec<_> = (0..THROAT_ROLLS / 2).map(rolled).collect();
        for (_, record) in carriage_bodies() {
            bodies.extend(
                carriage_sweep()
                    .into_iter()
                    .step_by(4)
                    .map(|carriage| with_carriage(&record, carriage)),
            );
        }
        bodies
    }

    #[test]
    fn a_beast_s_head_and_breast_are_charted_evenly() {
        // Wrapped round the neck's bone and counted by height along it, the
        // underside of a head on a raised neck and a beast's breast each came out
        // at one height: the worst hundredth of a head's skin stretched 70 to one
        // on an average roll and 286 on the worst, as hard-edged streaks down the
        // jaw. See the bounds.
        let bodies = charted_bodies();
        let read = |zone: Zone| {
            across((0..bodies.len() as i64).collect(), |index| {
                (index, chart_reading(&built(&bodies[index as usize]), zone))
            })
        };
        let rolls = THROAT_ROLLS / 2;
        for (index, [tenth, hundredth]) in read(Zone::Head) {
            assert!(tenth < HEAD_TENTH, "body {index}: a head's tenth {tenth}");
            let bound = if index < rolls {
                HEAD_ROLLED_HUNDREDTH
            } else {
                HEAD_SWEPT_HUNDREDTH
            };
            assert!(
                hundredth < bound,
                "body {index}: a head's hundredth {hundredth}"
            );
        }
        for (index, [_, hundredth]) in read(Zone::Chest) {
            assert!(
                hundredth < CHEST_HUNDREDTH,
                "body {index}: a chest's hundredth {hundredth}"
            );
        }
        for (index, [tenth, hundredth]) in read(Zone::Neck) {
            assert!(tenth < NECK_TENTH, "body {index}: a neck's tenth {tenth}");
            assert!(
                hundredth < NECK_HUNDREDTH,
                "body {index}: a neck's hundredth {hundredth}"
            );
        }
    }

    /// See [`a_beast_s_head_and_breast_are_charted_evenly`]. Set after reading
    /// `throat_table`: a head's worst tenth is 5.45, and a chest's worst hundredth
    /// 19.2, on a dog. A head's worst hundredth is 8.68 on a rolled beast and 6.3 on
    /// a swept one up to a carriage of 1; past that the head lies against the neck,
    /// and the crook between them reaches 55 on a cross carried at 1.2, as a neck's
    /// worst tenth reaches 15.2 there. A neck's worst hundredth is 36.2, on a roll.
    /// None of that crook shows on the surface at those carriages.
    const HEAD_TENTH: f32 = 6.0;
    const HEAD_ROLLED_HUNDREDTH: f32 = 10.0;
    const HEAD_SWEPT_HUNDREDTH: f32 = 60.0;
    const CHEST_HUNDREDTH: f32 = 22.0;
    const NECK_TENTH: f32 = 17.0;
    const NECK_HUNDREDTH: f32 = 40.0;

    #[test]
    fn the_upper_neck_is_charted_evenly() {
        // Wrapped with the head, about a cylinder it runs along rather than round,
        // this skin stretched at 17.5 to one on the very first roll and streaked
        // hard shapes through any coat. See the bounds.
        let rows = across((0..THROAT_ROLLS).step_by(2).collect(), |seed| {
            let record = rolled(seed);
            (seed, upper_neck_charts(&record, &built(&record)))
        });
        for (seed, (stretch, thin)) in rows {
            assert!(stretch < UPPER_STRETCH, "seed {seed}: stretches {stretch}");
            assert!(
                thin > UPPER_THIN,
                "seed {seed}: thins to {thin} of its median"
            );
        }
    }

    /// See [`the_upper_neck_is_charted_evenly`]. Set after reading `throat_table`:
    /// the worst stretch is 3.24 and the thinnest .805 of its median.
    const UPPER_STRETCH: f32 = 4.0;
    const UPPER_THIN: f32 = 0.6;

    #[test]
    fn every_corner_of_the_head_shapes_closed() {
        // The head's own axes, each at both ends of its envelope at once:
        // what saturates in the derive has to hold the surface together.
        let ends = [-3.0, 3.0];
        for muzzle_length in ends {
            for muzzle_breadth in ends {
                for stop in ends {
                    for appeal in ends {
                        let params = QuadrupedParams {
                            muzzle_length,
                            muzzle_breadth,
                            stop,
                            appeal,
                            ..QuadrupedParams::default()
                        };
                        let what = format!(
                            "muzzle {muzzle_length}, breadth {muzzle_breadth}, stop {stop}, appeal {appeal}"
                        );
                        let (skeleton, mesh, rig) = beast(&params);
                        let head = skeleton.beast_head.expect("a quadruped declares its head");
                        let shaped = shape(&mesh, &rig, &head);
                        assert!(shaped.is_closed_manifold(), "{what}: the head has a hole");
                        // No ray ran off the head: every vertex stays within
                        // the reach a march starts from.
                        for point in &shaped.positions {
                            assert!(point.is_finite(), "{what}: a vertex left the world");
                        }
                        let far = shaped
                            .positions
                            .iter()
                            .zip(&mesh.positions)
                            .map(|(after, before)| after.distance(*before))
                            .fold(0.0f32, f32::max);
                        assert!(
                            far < 1.5 * head.radius,
                            "{what}: a vertex moved {far} m on a {} m head",
                            head.radius
                        );
                    }
                }
            }
        }
    }

    // ---- The back of the head and the join (#388).

    /// Every body the join is judged on: each family and a cross of two, each
    /// head from the roll's smallest to its largest, under a young face and an
    /// old one.
    fn heads() -> Vec<(String, QuadrupedParams)> {
        use crate::plan::Family;
        let kinds = [
            ("dog", Family::Dog, None),
            ("cat", Family::Cat, None),
            ("horse", Family::Horse, None),
            ("dog x horse", Family::Dog, Some(Family::Horse)),
            ("horse x cat", Family::Horse, Some(Family::Cat)),
        ];
        let mut all = Vec::new();
        for (name, family, other) in kinds {
            for head_size in [-1.5, 0.0, 1.5] {
                for appeal in [-1.0, 0.5, 2.0] {
                    let mut params = QuadrupedParams::default();
                    params.set_kind(family, other, 0.5);
                    params.head_size = head_size;
                    params.appeal = appeal;
                    all.push((format!("{name} head {head_size} appeal {appeal}"), params));
                }
            }
        }
        all
    }

    /// How many sections a neck is read at, and how far apart, in head radii.
    const STATIONS: usize = 14;
    const STATION_STEP: f32 = 0.3;

    /// How far down the neck's lower bone its sections are read.
    const NECK_READ: f32 = 0.35;

    /// The drawn surface's half-extents `(across, deep)`, read on the neck's
    /// own bones up to where they enter the skull: for each station, the
    /// furthest the skin stands off the line in the plane square to it. Off the
    /// rig and the mesh and nothing the build's sizing knows about.
    fn drawn_sections(params: &QuadrupedParams) -> (BeastHead, Vec<Vec2>, usize) {
        let (skeleton, mesh, rig) = beast(params);
        let head = skeleton.beast_head.expect("a quadruped declares its head");
        let body = shape(&mesh, &rig, &head);
        let neck = rig.in_zone(Zone::Neck)[0];
        let (withers, neck_at) = rig.bone(neck);
        let occiput = rig
            .joints
            .iter()
            .position(|joint| joint.parent == Some(neck))
            .expect("the neck carries the head");
        let occiput_at = rig.joints[occiput].position;
        let step = STATION_STEP * head.radius;
        // From the occiput joint back down the bones, station by station.
        let upper = (occiput_at - neck_at).length();
        let walk = |gone: f32| -> (Vec3, Vec3) {
            if gone < upper {
                let run = (occiput_at - neck_at).normalize();
                (occiput_at - run * gone, run)
            } else {
                let run = (neck_at - withers).normalize();
                (neck_at - run * (gone - upper), run)
            }
        };
        let joint_station = (upper / step).round() as usize;
        // Every edge of the skin the plane square to the line cuts, so that a
        // reading does not depend on where the neck's rings happen to fall.
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for face in &body.faces {
            for corner in 0..face.len() {
                edges.push((face[corner], face[(corner + 1) % face.len()]));
            }
        }
        // No further down than the neck's upper run: below that a section is
        // the chest's business.
        let lowest = upper + NECK_READ * neck_at.distance(withers);
        let sections = (0..STATIONS)
            .map(|station| station as f32 * step)
            .take_while(|&gone| gone <= lowest)
            .map(|gone| {
                let (at, run) = walk(gone);
                let deep = Vec3::X.cross(run).normalize_or(Vec3::Y);
                let (mut across, mut lo, mut hi) = (0.0f32, f32::MAX, f32::MIN);
                for &(a, b) in &edges {
                    let (a, b) = (body.positions[a as usize], body.positions[b as usize]);
                    let (from_a, from_b) = ((a - at).dot(run), (b - at).dot(run));
                    if from_a * from_b >= 0.0 {
                        continue;
                    }
                    let off = a.lerp(b, from_a / (from_a - from_b)) - at;
                    if off.length() > 2.0 * head.radius {
                        continue;
                    }
                    across = across.max(off.x.abs());
                    lo = lo.min(off.dot(deep));
                    hi = hi.max(off.dot(deep));
                }
                Vec2::new(across, 0.5 * (hi - lo))
            })
            .collect();
        (head, sections, joint_station)
    }

    /// The head proper's own half-extents behind the eye, read by marching in
    /// from outside: what a neck's size is judged against.
    fn head_section(head: &BeastHead) -> Vec2 {
        let r = head.radius;
        let z = -0.45 * r;
        let mut across = 0.0f32;
        let mut y = -1.5 * r;
        while y < 1.5 * r {
            if head.distance(Vec3::new(0.0, y, z)) < 0.0 {
                across = across.max(side_at(head, y, z));
            }
            y += 0.05 * r;
        }
        let at_x = |x: f32, y: f32| head.distance(Vec3::new(x, y, z)) < 0.0;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        let mut y = -2.0 * r;
        while y < 2.0 * r {
            let mut x = 0.0;
            while x < 1.5 * r {
                if at_x(x, y) {
                    lo = lo.min(y);
                    hi = hi.max(y);
                }
                x += 0.05 * r;
            }
            y += 0.01 * r;
        }
        Vec2::new(across, 0.5 * (hi - lo))
    }

    /// A table to read: `cargo test --release --lib beast::tests::nape_table
    /// -- --ignored --nocapture`.
    #[test]
    #[ignore = "a table to read, not a check"]
    fn nape_table() {
        for (name, params) in heads() {
            let (head, sections, joint) = drawn_sections(&params);
            let own = head_section(&head);
            let at = sections[joint.min(sections.len() - 1)];
            let steps: Vec<f32> = sections
                .windows(2)
                .map(|pair| (pair[0] - pair[1]).abs().max_element() / head.radius)
                .collect();
            let worst = steps.iter().copied().fold(0.0, f32::max);
            println!(
                "{name:32} r {:.3} head {:.2} {:.2} neck {:.2} {:.2} worst step {worst:.3} sections {:?}",
                head.radius,
                own.x / head.radius,
                own.y / head.radius,
                at.x / own.x,
                at.y / own.y,
                sections
                    .iter()
                    .map(|s| (
                        ((s.x / head.radius) * 100.0).round() / 100.0,
                        ((s.y / head.radius) * 100.0).round() / 100.0
                    ))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn the_neck_grows_into_the_head_without_a_step() {
        // Between neighbouring sections, three tenths of a head radius apart,
        // in head radii. Bound set after reading `nape_table`: the worst of
        // the sized neck is 0.43, on a cat's small head where the run meets
        // the chest; the cage's neck stepped 0.43 to 0.74 at a horse's jowl.
        // A dog's or a cat's cage neck stepped less, 0.23: it pinched rather
        // than stepped, which the next test catches. **A number on the mesh,
        // not a proof of how it looks.**
        for (name, params) in heads() {
            let (head, sections, _) = drawn_sections(&params);
            for pair in sections.windows(2) {
                let step = (pair[0] - pair[1]).abs().max_element() / head.radius;
                assert!(
                    step < NECK_STEP,
                    "{name}: a step of {step:.2} radii in {sections:?}"
                );
            }
        }
    }

    /// See [`the_neck_grows_into_the_head_without_a_step`].
    const NECK_STEP: f32 = 0.45;

    #[test]
    fn the_neck_at_the_head_is_sized_by_the_head() {
        // The neck joint's section as a share of the head's, across every head
        // of one family from smallest to largest. The cage's neck does not know
        // the head, so its share runs from 0.44 to 1.83 over this sweep; this
        // one holds 0.58 to 0.67. And it narrows at the throat: well under the
        // head's whole size, which is not a tube the width of the skull.
        for family in ["dog", "cat", "horse", "dog x horse", "horse x cat"] {
            let shares: Vec<Vec2> = heads()
                .iter()
                .filter(|(name, _)| {
                    name.starts_with(family) && name.chars().nth(family.len()) == Some(' ')
                })
                .map(|(name, params)| {
                    let (head, sections, joint) = drawn_sections(params);
                    let own = head_section(&head);
                    let share = sections[joint.min(sections.len() - 1)] / own;
                    assert!(
                        share.max_element() < THROAT_MOST,
                        "{name}: a neck {share} of its head is no throat"
                    );
                    assert!(
                        share.min_element() > THROAT_LEAST,
                        "{name}: a neck {share} of its head is a post in a ball"
                    );
                    share
                })
                .collect();
            let spread = |of: fn(&Vec2) -> f32| {
                let values: Vec<f32> = shares.iter().map(of).collect();
                values.iter().copied().fold(0.0, f32::max)
                    / values.iter().copied().fold(f32::MAX, f32::min)
            };
            let (across, deep) = (spread(|share| share.x), spread(|share| share.y));
            assert!(
                across < THROAT_SPREAD && deep < THROAT_SPREAD,
                "{family}: the neck's share of the head runs over {across:.2} and {deep:.2}"
            );
        }
    }

    /// See [`the_neck_at_the_head_is_sized_by_the_head`]. Set after reading
    /// `nape_table`.
    const THROAT_MOST: f32 = 0.78;
    const THROAT_LEAST: f32 = 0.55;
    const THROAT_SPREAD: f32 = 1.20;

    /// How far down a head its back is read, in head radii below its centre.
    const BOW_LOW: f32 = -0.6;

    /// How round the back of a head is, side on: of the silhouette's rearmost
    /// reach from under the poll to the jaw's angle, how far it bows behind the
    /// chord between those ends, as a share of the chord: nothing for a flat nape.
    fn bow_behind(head: &BeastHead) -> f32 {
        let r = head.radius;
        let rearmost = |y: f32| {
            let mut z = -3.0 * r;
            while z < 0.5 * r {
                let mut x = 0.0;
                while x < 1.2 * r {
                    if head.distance(Vec3::new(x, y, z)) < 0.0 {
                        return Some(z);
                    }
                    x += 0.04 * r;
                }
                z += 0.01 * r;
            }
            None
        };
        let mut top = 0.0f32;
        let mut y = 0.0;
        while y < 2.0 * r {
            if rearmost(y).is_some() {
                top = y;
            }
            y += 0.02 * r;
        }
        // From a little under the poll to the height of the jaw's angle: below
        // that a head without a jowl ends in the muzzle's underside, far
        // forward of the skull, which says nothing of the skull's back.
        let (high, low) = (top - 0.15 * r, BOW_LOW * r);
        let (Some(z_high), Some(z_low)) = (rearmost(high), rearmost(low)) else {
            return 0.0;
        };
        let chord = Vec2::new(high - low, z_high - z_low);
        let mut furthest = 0.0f32;
        for step in 1..20 {
            let y = low + (high - low) * step as f32 / 20.0;
            if let Some(z) = rearmost(y) {
                let share = (y - low) / (high - low);
                furthest = furthest.max(z_low + (z_high - z_low) * share - z);
            }
        }
        furthest / chord.length()
    }

    #[test]
    fn a_horse_s_nape_is_flat_and_a_cat_s_skull_is_round() {
        use crate::plan::Family;
        let bow = |family: Family, other: Option<Family>| {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, other, 0.5);
            bow_behind(&declared(&params))
        };
        let (horse, dog, cat) = (
            bow(Family::Horse, None),
            bow(Family::Dog, None),
            bow(Family::Cat, None),
        );
        let cross = bow(Family::Horse, Some(Family::Cat));
        println!("bow behind: horse {horse:.3} dog {dog:.3} cat {cat:.3} cross {cross:.3}");
        // Set after reading those: horse 0.10, dog 0.35, cross 0.37, cat
        // 0.40. A unit sphere bows 0.22 over this span, and the cat's head
        // over it, being longer than round, more. How it reads is the render's
        // to say.
        assert!(horse < 0.15, "a horse's nape bows {horse:.3}");
        assert!(cat > horse * 2.0, "cat {cat:.3}, horse {horse:.3}");
        assert!(horse < dog && dog < cat, "{horse:.3}, {dog:.3}, {cat:.3}");
        assert!(horse < cross && cross < cat, "a cross blends: {cross:.3}");
    }

    #[test]
    fn what_rides_on_the_head_still_sits_on_it() {
        // Every seat is cast before the back of the head is reshaped (#388), so
        // its base must still be in the new skull and not buried by it: the
        // ear's base is sunk a little and its first hand of length stands out.
        use crate::plan::Horn;
        for (name, mut params) in heads() {
            params.horn = Horn::Unicorn;
            let head = declared(&params);
            let r = head.radius;
            let ear = head.ear;
            let sunk = head.distance(ear.base);
            assert!(sunk < 0.0, "{name}: an ear's base floats {sunk:.4} off");
            assert!(sunk > -0.5 * r, "{name}: an ear's base is buried {sunk:.4}");
            let out = ear.base + ear.frame * Vec3::Y * (0.5 * ear.length);
            assert!(
                head.distance(out) > 0.0,
                "{name}: the middle of an ear is inside the skull"
            );
            let eye = head.eye;
            assert!(
                head.distance(eye.centre) < 0.0,
                "{name}: an eye's centre is off the skull"
            );
            assert!(
                head.distance(eye.centre + eye.facing * Vec3::Z * eye.radius) > -0.02 * r,
                "{name}: an eye is buried"
            );
            let horn = head.horn.expect("a horn");
            assert!(head.distance(horn.base) < 0.0, "{name}: a horn floats");
        }
    }

    #[test]
    fn a_deep_stop_sets_the_bridge_under_the_brow() {
        // The fall from the forehead to the bridge, along the midline.
        let fall = |stop: f32| {
            let head = declared(&QuadrupedParams {
                stop,
                ..QuadrupedParams::default()
            });
            let r = head.radius;
            (top_at(&head, 0.2 * r) - top_at(&head, 0.9 * r)) / r
        };
        let (roman, deep) = (fall(-1.0), fall(1.0));
        assert!(
            deep > roman + 0.15,
            "a deep stop falls {deep:.2} radii to the bridge, a Roman nose {roman:.2}"
        );
    }

    #[test]
    fn a_broad_muzzle_is_broad_at_the_nose() {
        let across = |muzzle_breadth: f32| {
            let head = declared(&QuadrupedParams {
                muzzle_breadth,
                ..QuadrupedParams::default()
            });
            let [_, nose] = head.axis;
            side_at(&head, nose.y, nose.z) / head.radius
        };
        let (wedge, boxy) = (across(-1.0), across(1.0));
        assert!(
            boxy > wedge * 1.3,
            "a broad muzzle is {boxy:.2} radii across the nose, a narrow one {wedge:.2}"
        );
    }

    #[test]
    fn a_beast_looks_out_of_its_head_where_its_eye_set_says() {
        use crate::{Archetype, Avatar, AvatarRecord};
        let mut outward = Vec::new();
        for eye_set in [-1.0, 0.0, 1.0] {
            let params = QuadrupedParams {
                eye_set,
                ..QuadrupedParams::default()
            };
            let record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
            let avatar = Avatar::build(&record).expect("the beast builds");
            let eyes = avatar.parts.eyes.as_ref().expect("a beast has eyes");
            let head = avatar.rig.joints[eyes.head].position;
            let body = &avatar.parts.body;
            for eye in [&eyes.left, &eyes.right] {
                let gaze = eye.rest * Vec3::Z;
                assert!(
                    body.contains(head + eye.pivot),
                    "eye set {eye_set}: a globe floats outside its head"
                );
                assert!(
                    !body.contains(head + eye.pivot + gaze * eye.radius),
                    "eye set {eye_set}: an eye looks out from under the skin"
                );
            }
            let (left, right) = (eyes.left.rest * Vec3::Z, eyes.right.rest * Vec3::Z);
            assert!(
                left.abs_diff_eq(Vec3::new(-right.x, right.y, right.z), 1.0e-4),
                "eye set {eye_set}: the right eye is not the left's reflection"
            );
            outward.push(left.x);
        }
        assert!(
            outward[0] < outward[1] && outward[1] < outward[2],
            "the eyes turn out {outward:?} as eyeSet runs frontal to lateral"
        );
    }

    #[test]
    fn a_beast_wears_its_ears_on_joints_of_their_own_bound_into_its_skin() {
        use crate::avatar::MeshKind;
        use crate::rig::Role;
        use crate::{Archetype, Avatar, AvatarRecord};
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        let avatar = Avatar::build(&record).expect("the beast builds");
        let ears = &avatar.parts.beast.as_ref().expect("a beast has ears").ears;
        let skin = &avatar
            .meshes
            .iter()
            .find(|mesh| mesh.kind == MeshKind::Skin)
            .expect("a skin")
            .mesh;
        for (ear, joint) in ears.parts() {
            let joint_of = &avatar.rig.joints[joint];
            assert_eq!(
                joint_of.role,
                Role::Helper,
                "an ear rides a joint of its own"
            );
            assert_eq!(joint_of.parent, Some(ears.head), "hung from the head");
            let held = skin
                .skin
                .iter()
                .filter(|vertex| vertex[0].joint as usize == joint && vertex[0].weight == 1.0)
                .count();
            assert!(
                held >= ear.vertex_count(),
                "the skin holds {held} vertices on the ear's joint, of its {}",
                ear.vertex_count()
            );
        }
        // And a person has none.
        let person = Avatar::build(&AvatarRecord::new("Person", Archetype::default()))
            .expect("the person builds");
        assert!(person.parts.beast.is_none());
    }

    #[test]
    fn a_nostril_is_a_hollow_on_either_nose() {
        use crate::plan::Nose;
        for nose in [Nose::Leather, Nose::Soft] {
            let head = declared(&QuadrupedParams {
                nose,
                ..QuadrupedParams::default()
            });
            let nostril = head.nose.nostril.centre;
            for side in [1.0, -1.0] {
                let at = Vec3::new(side * nostril.x, nostril.y, nostril.z);
                assert!(
                    head.distance(at) > 0.0,
                    "{nose:?}: the nostril at {at} is filled"
                );
            }
            assert_eq!(head.nose.leather.is_some(), nose == Nose::Leather);
        }
    }

    #[test]
    fn an_ear_s_spring_swings_the_ear_and_never_the_head() {
        use crate::anim::{Pose, SpringConfig, Springs};
        use crate::{Archetype, Avatar, AvatarRecord};
        let record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        let avatar = Avatar::build(&record).expect("the beast builds");
        let rig = &avatar.rig;
        let ears = &avatar.parts.beast.as_ref().expect("a beast has ears").ears;
        let bases: Vec<usize> = [&ears.left, &ears.right]
            .into_iter()
            .map(|ear| ear.joint.expect("rigged"))
            .collect();
        let mut springs = Springs::of(rig);
        assert_eq!(
            springs.len(),
            2,
            "one chain an ear, and nothing else swings"
        );
        // Throw the head about, from the neck, and let the springs answer.
        let config = SpringConfig::default();
        let neck = rig.in_zone(Zone::Neck)[0];
        let mut pose = Pose::rest(rig);
        springs.advance(rig, &mut pose, None, 1.0 / 60.0, &config);
        pose.rotations[neck] = glam::Quat::from_rotation_y(1.0);
        springs.advance(rig, &mut pose, None, 1.0 / 60.0, &config);
        assert_eq!(
            pose.rotations[ears.head],
            glam::Quat::IDENTITY,
            "a spring turned the head"
        );
        assert!(
            bases
                .iter()
                .any(|&base| pose.rotations[base].angle_between(glam::Quat::IDENTITY) > 1.0e-3),
            "no ear lagged the head"
        );
    }

    #[test]
    fn an_eye_is_seated_on_the_braincase_with_room_for_its_pair() {
        // The envelope's corners that put an eye on a Roman nose or ran the
        // pair into each other (#367's critic): every eye stays behind the
        // braincase's front and clear of the midline by its globe.
        let ends = [-3.0, 3.0];
        for eye_set in ends {
            for stop in ends {
                for muzzle_length in ends {
                    for appeal in ends {
                        let head = declared(&QuadrupedParams {
                            eye_set,
                            stop,
                            muzzle_length,
                            appeal,
                            ..QuadrupedParams::default()
                        });
                        let what = format!(
                            "eye set {eye_set}, stop {stop}, muzzle {muzzle_length}, appeal {appeal}"
                        );
                        let eye = head.eye;
                        let brow = head.cranium.centre.z + head.cranium.radii.z;
                        assert!(eye.centre.z <= brow, "{what}: the eye sits on the muzzle");
                        assert!(
                            eye.centre.x >= eye.radius,
                            "{what}: the eyes run into each other"
                        );
                    }
                }
            }
        }
    }
}
