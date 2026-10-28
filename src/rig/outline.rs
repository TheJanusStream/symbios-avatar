//! Where a body's skin reaches, kept on its rig (#377).
//!
//! A rig is joints and radii; the skin over it is the cage's hull, subdivided
//! and shaped, and nothing on the rig says where it ends up. A pose that only
//! moves limbs through the air never needs to know. One that rests a body on
//! the ground does: a sit is the haunches and the hocks ON the ground and
//! nothing else in it, and the radii a joint carries miss the skin over it
//! by a tenth of their size on a dog's thigh and by 38 mm at a horse's
//! folded hock, where the skin blended across the joint folds in.
//!
//! So a built body hands its rig the few points of its skin that can be its
//! lowest, each bound as the skin is: the extremes of every joint's own skin,
//! and of each band blended across two joints, along a fan of directions
//! round the body's side plane and out to either flank, which is where a body
//! that pitches, sits back or rolls onto a hip looks for its underside; and
//! every vertex held near half and half between two joints, whose skin bends
//! as no rigid part does. Posed through the skin's own transforms, those
//! points stand exactly where the drawn skin does.
//!
//! **It crosses a worker boundary as one blob**, as the meshes do (see
//! `mesh_serde`): two thousand points derived field by field became a named
//! map each under `rmp_serde::to_vec_named`, 337 to 365 KB of a beast's rig
//! in JSON where its joints are 8. Not rebuilt on the far side instead,
//! because what it is measured from - the body's own mesh and weights - is
//! exactly what an avatar does not send.

use glam::Vec3;

use crate::anim::{Posed, dual};
use crate::mesh::PolyMesh;
use crate::rig::{Influence, MAX_INFLUENCES, Rig, SkinWeights};

/// How many directions round the side plane each joint's skin is searched
/// along, and how far out to either flank.
///
/// Every fifteen degrees: a point found as the extreme along one direction
/// stands at most `1 - cos 7.5` of its distance - under a centimetre on a
/// horse's croup - above the true underside of a body turned between two.
const ROUND: usize = 24;
const FLANKS: [f32; 5] = [-1.0, -0.4, 0.0, 0.4, 1.0];

/// The share a second joint must hold of a vertex for its skin to count as
/// blended across the two.
const BLENDED: f32 = 0.1;

/// The share a second joint must hold of a vertex for the vertex to be kept
/// whatever its place: near half and half.
const EVEN: f32 = 0.4;

/// One point of the skin, bound as the skin binds it.
#[derive(Clone, Debug, PartialEq)]
struct Point {
    at: Vec3,
    held: [Influence; MAX_INFLUENCES],
}

/// The points of a body's skin that can be its lowest, however it is posed.
///
/// Under `serde-avatar` it travels as one byte blob, 36 bytes a point: the
/// place's three `f32`s, then each of the [`MAX_INFLUENCES`] holds on it as
/// a `u16` joint and an `f32` weight, all little-endian, as a mesh's skin
/// channel is written.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde-avatar",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "wire::Wire", try_from = "wire::Wire")
)]
pub struct Outline {
    points: Vec<Point>,
}

impl Outline {
    /// Measures `mesh`, bound by `weights`: for every joint, the skin it holds
    /// most of, at its extremes.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, weights: &SkinWeights) -> Self {
        let fan = |round: usize| {
            let mut directions = Vec::with_capacity(round * FLANKS.len());
            for step in 0..round {
                #[expect(clippy::cast_precision_loss, reason = "a few dozen directions")]
                let angle = std::f32::consts::TAU * step as f32 / round as f32;
                let (sin, cos) = crate::det::sin_cos(angle);
                for flank in FLANKS {
                    directions.push(Vec3::new(flank, sin, cos).normalize());
                }
            }
            directions
        };
        let directions = fan(ROUND);
        // Grouped by the joint that holds each vertex most and, where a
        // second holds a real share, by that one too: across a joint the
        // skin is blended, and folded it moves as neither bone does, so the
        // extremes of a blended band are searched apart from the rigid skin
        // either side of it.
        let mut owned: std::collections::BTreeMap<(u16, u16), Vec<usize>> =
            std::collections::BTreeMap::new();
        let mut chosen: Vec<usize> = Vec::new();
        for (vertex, held) in weights.vertices.iter().enumerate() {
            // Held half and half, a vertex moves as neither joint does when
            // the two bend apart: a dual quaternion's blend bulges there, as
            // the loin bends between two ribs.
            if vertex < mesh.positions.len() && held[1].weight >= EVEN {
                chosen.push(vertex);
            }
            if vertex < mesh.positions.len() && held[0].weight > 0.0 {
                let second = if held[1].weight >= BLENDED {
                    held[1].joint
                } else {
                    u16::MAX
                };
                owned
                    .entry((held[0].joint, second))
                    .or_default()
                    .push(vertex);
            }
        }
        for vertices in owned.values() {
            for direction in &directions {
                let best = vertices.iter().copied().max_by(|&a, &b| {
                    direction
                        .dot(mesh.positions[a])
                        .total_cmp(&direction.dot(mesh.positions[b]))
                });
                if let Some(best) = best {
                    chosen.push(best);
                }
            }
        }
        chosen.sort_unstable();
        chosen.dedup();
        Self {
            points: chosen
                .into_iter()
                .map(|vertex| Point {
                    at: mesh.positions[vertex],
                    held: weights.vertices[vertex],
                })
                .collect(),
        }
    }

    /// Whether it has no points: a body measured with no skin, which a rest
    /// reads as no outline at all.
    pub(crate) fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// How many points it has.
    pub(crate) fn len(&self) -> usize {
        self.points.len()
    }

    /// The lowest the skin reaches in `posed` in each of `lows`' slots, at
    /// once: a point held most by a joint goes in the slot `slot` names for
    /// that joint, or in none. A slot no point reaches stays as it was.
    ///
    /// One pass over the points and one set of the skin's transforms for
    /// every reading a solve takes of one pose (#377): two thousand points
    /// blended per question was what a rest's every frame cost.
    pub(crate) fn lowest_each(
        &self,
        rig: &Rig,
        posed: &Posed,
        slot: impl Fn(usize) -> Option<usize>,
        lows: &mut [f32],
    ) {
        let transforms = posed.skinning_transforms(rig);
        // Asked once a joint, not once a point.
        let slots: Vec<Option<usize>> = (0..transforms.len()).map(slot).collect();
        for point in &self.points {
            let Some(&Some(into)) = slots.get(usize::from(point.held[0].joint)) else {
                continue;
            };
            let Some(low) = lows.get_mut(into) else {
                continue;
            };
            // A hold on a joint the rig has not got - a point sent for
            // another body - holds nothing.
            let y = dual::blend(point.held.iter().filter_map(|held| {
                let transform = transforms.get(usize::from(held.joint))?;
                (held.weight > 0.0).then_some((*transform, held.weight))
            }))
            .transform_point(point.at)
            .y;
            *low = low.min(y);
        }
    }
}

/// The outline as it travels (#377): see [`Outline`].
#[cfg(feature = "serde-avatar")]
mod wire {
    use glam::Vec3;

    use super::{Outline, Point};
    use crate::rig::{Influence, MAX_INFLUENCES};

    /// Bytes one point takes: its place, and a joint and a weight for each
    /// hold on it.
    pub(super) const POINT: usize = 12 + MAX_INFLUENCES * 6;

    /// One blob of every point, `deny_unknown_fields` as a mesh's is: the two
    /// halves of a worker boundary ship together, and a field this does not
    /// know is a version skew.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Wire {
        #[serde(with = "serde_bytes")]
        points: Vec<u8>,
    }

    impl From<Outline> for Wire {
        fn from(outline: Outline) -> Self {
            let mut points = Vec::with_capacity(outline.points.len() * POINT);
            for point in &outline.points {
                for value in point.at.to_array() {
                    points.extend_from_slice(&value.to_le_bytes());
                }
                for held in &point.held {
                    points.extend_from_slice(&held.joint.to_le_bytes());
                    points.extend_from_slice(&held.weight.to_le_bytes());
                }
            }
            Self { points }
        }
    }

    impl TryFrom<Wire> for Outline {
        type Error = String;

        fn try_from(wire: Wire) -> Result<Self, Self::Error> {
            let (chunks, rest) = wire.points.as_chunks::<POINT>();
            if !rest.is_empty() {
                return Err(format!(
                    "an outline of {} bytes is not a whole number of {POINT}-byte points",
                    wire.points.len()
                ));
            }
            let f32_at = |chunk: &[u8; POINT], at: usize| {
                f32::from_le_bytes([chunk[at], chunk[at + 1], chunk[at + 2], chunk[at + 3]])
            };
            let points = chunks
                .iter()
                .map(|chunk| {
                    let mut held = [Influence::default(); MAX_INFLUENCES];
                    for (index, slot) in held.iter_mut().enumerate() {
                        let at = 12 + index * 6;
                        *slot = Influence {
                            joint: u16::from_le_bytes([chunk[at], chunk[at + 1]]),
                            weight: f32_at(chunk, at + 2),
                        };
                    }
                    Point {
                        at: Vec3::new(f32_at(chunk, 0), f32_at(chunk, 4), f32_at(chunk, 8)),
                        held,
                    }
                })
                .collect();
            Ok(Self { points })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::anim::emote;
    use crate::anim::rest::tests::built;
    use crate::plan::Family;

    #[test]
    fn the_outline_reads_a_folded_body_as_its_skin_is_drawn() {
        // The lowest of the outline's points, posed through the skin's own
        // transforms, against the lowest of every vertex of the drawn skin:
        // across a closed hock, a folded knee, a loin bent between two ribs.
        for family in [Family::Dog, Family::Horse] {
            let (rig, mesh) = built(family);
            let outline = rig.outline.as_ref().expect("built with its outline");
            let (up, _) = emote::getting_up(&rig, "Sit").expect("a beast gets up");
            let mut clips = vec![(emote::sit(), 1.0), (emote::lie_down(), 1.0)];
            for step in 0..=10 {
                #[expect(clippy::cast_precision_loss, reason = "ten moments")]
                clips.push((up.clone(), step as f32 / 10.0));
            }
            for (clip, time) in clips {
                let posed = clip.pose(&rig, time).forward(&rig);
                let drawn = posed.deform_mesh(&rig, &mesh);
                let skin = drawn
                    .positions
                    .iter()
                    .map(|at| at.y)
                    .fold(f32::MAX, f32::min);
                let mut read = [f32::MAX];
                outline.lowest_each(&rig, &posed, |_| Some(0), &mut read);
                let read = read[0];
                assert!(
                    (read - skin).abs() < 5.0e-4,
                    "{family:?} at {time:.1}: the outline read {:.1} mm, the skin is {:.1}",
                    read * 1000.0,
                    skin * 1000.0
                );
            }
        }
    }
}
