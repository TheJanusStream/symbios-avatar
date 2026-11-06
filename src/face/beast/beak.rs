//! A bird's beak (#422): its two mandibles, swept along the gape from the
//! seat its head declares (`BeakSeat`).
//!
//! **Two closed solids of horn.** The upper mandible stands over the line of
//! the gape and turns down along it as far as the seat's hook, tapering to its
//! tip, so a parrot's curls over its lower mandible and a songbird's stays a
//! straight cone; its section is an arch, narrower toward the culmen than at
//! the cutting edges. The lower mandible hangs under the same line, straighter
//! and shorter, its top tucked up inside the upper's, so the closed beak shows
//! no seam but the gape's. The upper rides the head and the lower the jaw
//! (`Adornments::on_jaw`), so the mouth that opens a beast's jaw parts them.
//!
//! Built in the head's frame and coloured from base to tip, for the eyes'
//! draw (`adorn`), which carries a part's own colours on any plumage.

use glam::{Vec2, Vec3};

use crate::det::DetMath;
use crate::mesh::PolyMesh;
use crate::plan::beast_head::BeakSeat;
use crate::prim;

/// Stations along a mandible, base to tip.
const STATIONS: usize = 11;

/// Points round a mandible's section.
const SIDES: usize = 12;

/// How many steps each stretch between stations is integrated in, along a
/// curving line.
const SUBSTEPS: usize = 4;

/// How much of its size a mandible keeps at its tip: a point, but not one
/// fine enough to fold its last facets over.
const TIP: f32 = 0.06;

/// How deep the upper mandible's floor sinks under the gape, as a share of
/// its height: the roof of the mouth, which the lower mandible closes on.
const PALATE: f32 = 0.18;

/// How far up inside the upper the lower mandible's top reaches, as a share
/// of the upper's depth at the base.
const TUCK: f32 = 0.22;

/// How much narrower the lower mandible is than the upper over it, so its
/// sides close inside the upper's cutting edges.
const LOWER_WIDTH: f32 = 0.88;

/// How much the arch narrows toward its top, as a share of the half-width:
/// a beak's culmen is a ridge, not a dome.
const RIDGE: f32 = 0.35;

/// How far the lower mandible's line rises toward its tip, in radians: the
/// gonys turns up a little to meet the upper.
const GONYS: f32 = 0.12;

/// The upper mandible, in the head's frame.
#[must_use]
pub(crate) fn upper(seat: &BeakSeat) -> PolyMesh {
    let line = line(seat, seat.length, |along| -seat.hook * along * along);
    let outlines: Vec<Vec<Vec2>> = (0..STATIONS)
        .map(|station| {
            let left = remaining(station);
            let half = seat.half_width * left.det_powf(0.55).max(TIP);
            let height = seat.depth * left.det_powf(0.65).max(TIP);
            section(half, height, PALATE * height)
        })
        .collect();
    coloured(prim::sweep_outline(&line, &outlines, Vec3::X), seat)
}

/// The lower mandible, in the head's frame: on the same gape, so the jaw it
/// rides opens it about the jaw's hinge.
#[must_use]
pub(crate) fn lower(seat: &BeakSeat) -> PolyMesh {
    let reach = seat.length * seat.lower_reach;
    let line = line(seat, reach, |along| GONYS * along);
    let outlines: Vec<Vec<Vec2>> = (0..STATIONS)
        .map(|station| {
            let left = remaining(station);
            let half = LOWER_WIDTH * seat.half_width * left.det_powf(0.7).max(TIP);
            let below = seat.lower_depth * left.det_powf(0.8).max(TIP);
            let above = TUCK * seat.depth * left.max(TIP);
            section(half, above, below)
        })
        .collect();
    coloured(prim::sweep_outline(&line, &outlines, Vec3::X), seat)
}

/// How much of a mandible's length is still ahead of `station`, `1..=0`.
fn remaining(station: usize) -> f32 {
    1.0 - station as f32 / (STATIONS - 1) as f32
}

/// A mandible's line, `length` long from the seat's base, turning from the
/// seat's forward toward its up by `turn(along)` radians `along` of the way:
/// negative turns down.
fn line(seat: &BeakSeat, length: f32, turn: impl Fn(f32) -> f32) -> Vec<Vec3> {
    let steps = (STATIONS - 1) * SUBSTEPS;
    let step = length / steps as f32;
    let mut at = seat.base;
    let mut line = vec![at];
    for index in 0..steps {
        let middle = turn((index as f32 + 0.5) / steps as f32);
        at += (seat.forward * middle.det_cos() + seat.up * middle.det_sin()) * step;
        if (index + 1) % SUBSTEPS == 0 {
            line.push(at);
        }
    }
    line
}

/// One section about the gape: `half` its half-width, `above` how far its top
/// stands over the gape and `below` how far its floor hangs under it. An arch
/// over the gape narrowing toward its ridge, and a shallow round under it.
fn section(half: f32, above: f32, below: f32) -> Vec<Vec2> {
    (0..SIDES)
        .map(|side| {
            let angle = std::f32::consts::TAU * side as f32 / SIDES as f32;
            let (sin, cos) = (angle.det_sin(), angle.det_cos());
            if sin >= 0.0 {
                Vec2::new(half * cos * (1.0 - RIDGE * sin), above * sin)
            } else {
                Vec2::new(half * cos, below * sin)
            }
        })
        .collect()
}

/// `mesh`, a sweep of [`STATIONS`] rings of [`SIDES`], coloured from the
/// seat's base colour to its tip's.
fn coloured(mut mesh: PolyMesh, seat: &BeakSeat) -> PolyMesh {
    let [base, tip] = seat.colours;
    mesh.set_colours(
        (0..STATIONS)
            .flat_map(|station| {
                let along = 1.0 - remaining(station);
                vec![base.lerp(tip, along * along); SIDES]
            })
            .collect(),
    );
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(hook: f32) -> BeakSeat {
        BeakSeat {
            base: Vec3::ZERO,
            forward: Vec3::Z,
            up: Vec3::Y,
            length: 1.0,
            depth: 0.4,
            lower_depth: 0.22,
            half_width: 0.2,
            hook,
            lower_reach: 0.8,
            colours: [Vec3::splat(0.5), Vec3::splat(0.6)],
        }
    }

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

    #[test]
    fn each_mandible_is_a_closed_solid_wound_outward_and_coloured() {
        for hook in [0.0, 0.9, 1.8] {
            for mesh in [upper(&seat(hook)), lower(&seat(hook))] {
                assert!(mesh.manifold_report().is_clean(), "hook {hook}");
                assert!(volume(&mesh) > 0.0, "hook {hook}: wound inward");
                assert_eq!(mesh.colours.len(), mesh.vertex_count());
            }
        }
    }

    #[test]
    fn a_hook_carries_the_tip_down_past_the_lower_mandible() {
        let lowest = |mesh: &PolyMesh| {
            mesh.positions
                .iter()
                .map(|point| point.y)
                .fold(f32::MAX, f32::min)
        };
        let straight = upper(&seat(0.0));
        let hooked = upper(&seat(1.6));
        assert!(
            lowest(&straight) > -0.1,
            "a straight beak stays on its gape"
        );
        assert!(
            lowest(&hooked) < lowest(&lower(&seat(1.6))),
            "a parrot's hook reaches under its lower mandible"
        );
    }
}
