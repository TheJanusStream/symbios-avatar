//! A beast's ears (#367): cards of leather with a cup in them, standing off
//! the braincase where the head's plan seats them.
//!
//! **A part, not more of the head's surface**, for the reason a person's ear
//! is one (`face::features`): an ear stands off the side of the skull with a
//! hollow in it, which no displacement of the skull's surface can make. And a
//! card that hung off the cage as nodes would meet the mesher's socket wall
//! beside the head's rings, which is why a person's jaw is a marker
//! (`Node::marker`).
//!
//! Each ear is a closed shell - a front that cups, a back the leather's
//! thickness behind it, and a rim round both - built upright in its own
//! frame, folded where its carriage says, and turned onto the head, and is
//! drawn with the skin.
//!
//! **It rides a joint at its base and swings on one at its tip.** A spring
//! chain runs from the joint it hangs from (`anim::spring`), and the
//! simulation turns that joint to lag the chain: an ear whose only spring
//! joint hung off the cranium made a chain of the cranium and the ear's base,
//! and the motion to come (#370) would have swung the head. So the ear's
//! base is a [`Role::Helper`] of its own, which binds nothing of the body,
//! and its tip a [`Role::Spring`]: the chain is the ear and nothing else.

use glam::{Mat4, Quat, Vec2, Vec3};

use crate::face::smooth;
use crate::mesh::PolyMesh;
use crate::plan::BeastHead;
use crate::plan::beast_head::EarSeat;
use crate::rig::{Rig, Role};

/// Rows of the card from base to tip, before the tip's own vertex.
const ROWS: usize = 12;

/// Columns across the card, front and back alike.
const COLUMNS: usize = 8;

/// How far up the card its last row stands, as a share of the length; the tip
/// vertex closes it from there.
const LAST_ROW: f32 = 0.96;

/// How wide the band an ear's fold is spread over, as a share of its length:
/// a fold done at one row is a crease, and leather does not crease.
const FOLD_BAND: f32 = 0.20;

/// One ear: its joint, once rigged, and its shell in that joint's space.
#[derive(Clone, Debug, PartialEq)]
pub struct Ear {
    /// The shell, in its joint's local space: relative to [`Ear::base`].
    pub mesh: PolyMesh,
    /// Where the ear's base is, in the head joint's local space.
    pub base: Vec3,
    /// Where its tip is, in the head joint's local space: the end its
    /// spring swings.
    pub tip: Vec3,
    /// The joint the ear rides, at its base, once [`Ears::rig`] has run.
    pub joint: Option<usize>,
    /// The spring joint at its tip, a child of [`Ear::joint`].
    pub tip_joint: Option<usize>,
    /// The sign of this ear's `x`: `+1` for the body's left.
    pub side: f32,
}

/// A beast's pair of ears.
#[derive(Clone, Debug, PartialEq)]
pub struct Ears {
    /// The body's left ear, at `+X`.
    pub left: Ear,
    /// The body's right ear, at `-X`.
    pub right: Ear,
    /// The head joint the pair hangs from.
    pub head: usize,
}

impl Ears {
    /// Builds the ears a beast's head declares, for the head joint `head`.
    #[must_use]
    pub fn build(rig: &Rig, head: usize, beast: &BeastHead) -> Self {
        let origin = rig.joints[head].position;
        let seat = beast.ear;
        // A head that declares no ear - a bird's (#422) - builds each empty,
        // which is no part and rides no joint.
        let card = if seat.length > 0.0 {
            card(&seat)
        } else {
            PolyMesh::new()
        };
        let place = |side: f32| {
            let mirror = Vec3::new(side, 1.0, 1.0);
            let base = beast.body(seat.base * mirror) - origin;
            // The card is built about its own base, so it only turns: into
            // the head's frame, across the midline for the right ear - which
            // `PolyMesh::transformed` winds back the right way round - and
            // into the head joint's space with the head.
            let mesh = card.transformed(
                Mat4::from_quat(beast.turn)
                    * Mat4::from_scale(mirror)
                    * Mat4::from_quat(seat.frame),
            );
            // The card's last vertex is its tip, after any fold.
            let tip = base + mesh.positions.last().copied().unwrap_or(Vec3::ZERO);
            Ear {
                mesh,
                base,
                tip,
                joint: None,
                tip_joint: None,
                side,
            }
        };
        Self {
            left: place(1.0),
            right: place(-1.0),
            head,
        }
    }

    /// Hangs each ear on a joint of its own at its base, a [`Role::Helper`],
    /// with a [`Role::Spring`] at its tip: a chain of the ear alone, for the
    /// secondary motion to swing (#370). Neither binds the body's surface.
    /// Rigging twice does nothing.
    pub fn rig(&mut self, rig: &mut Rig) {
        if self.left.joint.is_some() {
            return;
        }
        let Some(origin) = rig.joints.get(self.head).map(|joint| joint.position) else {
            return;
        };
        for ear in [&mut self.left, &mut self.right] {
            if ear.mesh.is_empty() {
                continue;
            }
            ear.joint = rig.attach(self.head, origin + ear.base, Role::Helper);
            ear.tip_joint = ear
                .joint
                .and_then(|base| rig.attach(base, origin + ear.tip, Role::Spring));
        }
    }

    /// Both ears with their joints, left first; an ear not yet rigged rides
    /// the head.
    pub fn parts(&self) -> impl Iterator<Item = (&PolyMesh, usize)> {
        [&self.left, &self.right]
            .into_iter()
            .map(|ear| (&ear.mesh, ear.joint.unwrap_or(self.head)))
    }

    /// Both meshes, left first, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        [&mut self.left.mesh, &mut self.right.mesh].into_iter()
    }
}

/// One ear's shell, built in its own frame about its base, and folded.
///
/// The front cups toward `+Z`, recessed at the middle and level with the
/// rim at the edges, and the back stands the leather's thickness behind it.
/// Each row is one closed loop - across the front left to right, back across
/// the back - so the rows sew into a flattened tube, capped at the base and
/// closed to a point at the tip.
fn card(seat: &EarSeat) -> PolyMesh {
    let mut mesh = PolyMesh::new();
    let per_row = 2 * (COLUMNS + 1);
    for row in 0..=ROWS {
        let along = LAST_ROW * row as f32 / ROWS as f32;
        let half = 0.5 * seat.width * outline(along, seat.point);
        let depth = seat.cup * half * (1.0 - 0.3 * along);
        let thickness = seat.thickness * (1.0 - 0.6 * along);
        let height = seat.length * along;
        for column in 0..=COLUMNS {
            let across = -1.0 + 2.0 * column as f32 / COLUMNS as f32;
            let front = Vec3::new(across * half, height, depth * (across * across - 1.0));
            mesh.push_uv_vertex(front, Vec2::new((across + 1.0) * 0.25, along));
        }
        for column in (0..=COLUMNS).rev() {
            let across = -1.0 + 2.0 * column as f32 / COLUMNS as f32;
            let back = Vec3::new(
                across * half,
                height,
                depth * (across * across - 1.0) - thickness,
            );
            mesh.push_uv_vertex(back, Vec2::new(0.5 + (1.0 - across) * 0.25, along));
        }
    }
    let at = |row: usize, step: usize| (row * per_row + step % per_row) as u32;
    for row in 0..ROWS {
        for step in 0..per_row {
            mesh.push_face([
                at(row, step),
                at(row, step + 1),
                at(row + 1, step + 1),
                at(row + 1, step),
            ]);
        }
    }
    let base = mesh.push_uv_vertex(
        Vec3::new(
            0.0,
            0.0,
            -0.5 * (seat.cup * 0.5 * seat.width + seat.thickness),
        ),
        Vec2::new(0.25, 0.0),
    );
    let tip = mesh.push_uv_vertex(Vec3::new(0.0, seat.length, 0.0), Vec2::new(0.25, 1.0));
    for step in 0..per_row {
        mesh.push_face([base, at(0, step + 1), at(0, step)]);
        mesh.push_face([tip, at(ROWS, step), at(ROWS, step + 1)]);
    }
    fold(&mut mesh, seat);
    mesh
}

/// How wide the card is at `along` its length, as a share of its base:
/// a round-tipped leaf at `point` zero, a triangle at one.
fn outline(along: f32, point: f32) -> f32 {
    let leaf = (1.0 - along * along).max(0.0).sqrt() * (1.0 - 0.2 * along);
    let triangle = 1.0 - along;
    leaf + (triangle - leaf) * point
}

/// Folds the card above `fold_at` about `fold_axis`, through the card's
/// middle at that height, spreading the turn over [`FOLD_BAND`] so the
/// leather bends rather than creases.
fn fold(mesh: &mut PolyMesh, seat: &EarSeat) {
    if seat.fold.abs() <= f32::EPSILON || seat.length <= f32::EPSILON {
        return;
    }
    let hinge = Vec3::Y * (seat.fold_at * seat.length);
    let axis = seat.fold_axis.normalize_or(Vec3::X);
    for point in &mut mesh.positions {
        let along = point.y / seat.length;
        let share = smooth((along - seat.fold_at) / FOLD_BAND + 0.5);
        if share <= 0.0 {
            continue;
        }
        *point = hinge + Quat::from_axis_angle(axis, seat.fold * share) * (*point - hinge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, QuadrupedParams};

    /// The signed volume a closed mesh encloses: positive when its faces are
    /// wound outward.
    fn volume(mesh: &PolyMesh) -> f32 {
        mesh.triangulated()
            .iter()
            .map(|&[a, b, c]| {
                let [a, b, c] = [a, b, c].map(|index| mesh.positions[index as usize]);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    fn seat(carriage: f32) -> EarSeat {
        QuadrupedParams {
            ear_carriage: carriage,
            ..QuadrupedParams::default()
        }
        .skeleton(&Composites::default())
        .beast_head
        .expect("a quadruped declares its head")
        .ear
    }

    #[test]
    fn an_ear_is_a_closed_shell_wound_outward() {
        for carriage in [-1.0, 0.0, 1.0] {
            let mesh = card(&seat(carriage));
            assert!(
                mesh.is_closed_manifold(),
                "carriage {carriage}: the ear has a hole"
            );
            assert!(
                volume(&mesh) > 0.0,
                "carriage {carriage}: the ear is inside out"
            );
        }
    }

    #[test]
    fn a_pricked_ear_stands_and_a_dropped_one_hangs() {
        // The tip's height above the base, in the ear's own frame turned
        // onto the head: up for a pricked ear, below the base for a drop.
        let tip_rise = |carriage: f32| {
            let seat = seat(carriage);
            let mesh = card(&seat);
            let tip = mesh.positions[mesh.positions.len() - 1];
            (seat.frame * tip).y / seat.length
        };
        assert!(
            tip_rise(1.0) > 0.8,
            "a pricked ear's tip rose {}",
            tip_rise(1.0)
        );
        assert!(
            tip_rise(-1.0) < 0.0,
            "a dropped ear's tip rose {}",
            tip_rise(-1.0)
        );
    }

    #[test]
    fn the_two_ears_are_mirrors_and_both_wound_outward() {
        let params = QuadrupedParams::default();
        let skeleton = params.skeleton(&Composites::default());
        let beast = skeleton.beast_head.expect("a quadruped declares its head");
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        let head = rig
            .in_zone(crate::plan::Zone::Head)
            .into_iter()
            .min_by(|&a, &b| {
                let off = |joint: usize| rig.joints[joint].position.distance(beast.origin);
                off(a).total_cmp(&off(b))
            })
            .expect("a head");
        let ears = Ears::build(&rig, head, &beast);
        assert!(volume(&ears.left.mesh) > 0.0 && volume(&ears.right.mesh) > 0.0);
        assert!((ears.left.base.x + ears.right.base.x).abs() < 1.0e-5);
        for (left, right) in ears
            .left
            .mesh
            .positions
            .iter()
            .zip(&ears.right.mesh.positions)
        {
            assert!(left.abs_diff_eq(Vec3::new(-right.x, right.y, right.z), 1.0e-5));
        }
    }

    #[test]
    fn no_carriage_folds_an_ear_through_the_head() {
        // Past the third of it sunk into the skull, an ear stands outside the
        // head it grows from: a dropped one hangs clear of the cheek, and no
        // size or carriage at the envelope's ends puts it through the skull.
        for carriage in [-3.0, -1.0, -0.9, -0.8, -0.7, -0.4, 0.0, 1.0] {
            for ear_size in [-3.0, 0.0, 3.0] {
                let params = QuadrupedParams {
                    ear_carriage: carriage,
                    ear_size,
                    ..QuadrupedParams::default()
                };
                let skeleton = params.skeleton(&Composites::default());
                let beast = skeleton.beast_head.expect("a quadruped declares its head");
                let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
                let head = rig
                    .in_zone(crate::plan::Zone::Head)
                    .into_iter()
                    .min_by(|&a, &b| {
                        let off = |joint: usize| rig.joints[joint].position.distance(beast.origin);
                        off(a).total_cmp(&off(b))
                    })
                    .expect("a head");
                let origin = rig.joints[head].position;
                let ears = Ears::build(&rig, head, &beast);
                // Up the ear by the card's own rows, which a fold leaves where
                // they were in the ear: a third of the way is a third of them.
                let per_row = 2 * (COLUMNS + 1);
                let past_a_third = |index: usize| index / per_row > ROWS / 3;
                let deepest = [&ears.left, &ears.right]
                    .into_iter()
                    .flat_map(|ear| {
                        ear.mesh
                            .positions
                            .iter()
                            .take((ROWS + 1) * per_row)
                            .enumerate()
                            .filter(move |(index, _)| past_a_third(*index))
                            .map(move |(_, point)| {
                                beast.distance(beast.local(origin + ear.base + *point))
                            })
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!(
                    deepest > -0.02 * beast.radius,
                    "carriage {carriage}, size {ear_size}: an ear runs {:.2} radii into the head",
                    -deepest / beast.radius
                );
            }
        }
    }
}
