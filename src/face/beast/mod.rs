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
//! agree where they meet and there is no crease to fade across.
//!
//! Applied to the **rest** mesh before anything is bound or unwrapped, as the
//! human skull is, so skin weights and texture charts follow it without
//! knowing it happened.
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
    for ((point, share), carries) in shaped.positions.iter_mut().zip(shares).zip(carrying) {
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
    shaped
}

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
        refined = refined.refine_curved(&selected);
    }
    let nose = head.nose_centre();
    let selected: Vec<bool> = (0..refined.face_count())
        .map(|face| {
            headed(&refined, face)
                && head.local(refined.face_centroid(face)).distance(nose) < NOSE_REACH * head.radius
        })
        .collect();
    let mut refined = refined.refine_curved(&selected);
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
        refined = refined.refine_curved(&selected);
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
    use glam::{Vec2, Vec3};

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
