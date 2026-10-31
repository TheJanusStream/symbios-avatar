//! A beast's whiskers and its horn (#367): the head's two parts drawn in
//! colours of their own.
//!
//! **In the eyes' draw, not the skin's.** A whisker is white and a horn
//! ivory on any coat, and the skin's draw can only darken its atlas - its
//! vertex colours multiply it - so neither could be drawn there on a dark
//! coat. The eyes' draw is vertex-coloured, glossy and already paid for:
//! a whisker catches the light as a hair does, and a horn as horn does.
//!
//! A whisker is a thin tapered tube rather than a card, so it shows from
//! every side with no alpha to cut it out.

use glam::{Mat4, Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::BeastHead;
use crate::plan::beast_head::{HornSeat, WhiskerSeat};
use crate::prim;

/// A whisker's colour.
const WHISKER: Vec3 = Vec3::new(0.93, 0.92, 0.89);

/// A horn's colour: ivory, a little gold toward the tip.
const IVORY: Vec3 = Vec3::new(0.96, 0.91, 0.80);

/// A whisker's radius at its root, as a share of its length: drawn thicker
/// than life, which would be a line a pixel wide at any distance a face is
/// seen from.
/// Provenance: **tuned by render** (#367).
const WHISKER_GIRTH: f32 = 0.014;

/// Stations along a whisker and along a horn.
const STATIONS: usize = 6;

/// Facets round a horn.
const HORN_SIDES: usize = 12;

/// How many times a horn's flutes turn about it from base to tip.
const HORN_TURNS: f32 = 1.5;

/// A beast's whiskers and horn, built in its head joint's space and
/// coloured, for the eyes' draw.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Adornments {
    /// Both sides' whiskers, as one mesh; empty when the head grows none.
    pub whiskers: PolyMesh,
    /// The horn; empty when the head wears none.
    pub horn: PolyMesh,
}

impl Adornments {
    /// Builds what a beast's head declares, for the head joint standing at
    /// `origin` in body space.
    #[must_use]
    pub fn build(beast: &BeastHead, origin: Vec3) -> Self {
        let into_head = Mat4::from_translation(beast.origin - origin) * Mat4::from_quat(beast.turn);
        let whiskers = beast.whiskers.as_ref().map_or_else(PolyMesh::new, |seat| {
            let mut mesh = whiskers(seat);
            mesh.paint(WHISKER);
            mesh.transformed(into_head)
        });
        let horn = beast.horn.as_ref().map_or_else(PolyMesh::new, |seat| {
            let mut mesh = horn(seat);
            mesh.paint(IVORY);
            mesh.transformed(into_head)
        });
        Self { whiskers, horn }
    }

    /// Both meshes as one, empty if the head wears neither.
    #[must_use]
    pub fn assembled(&self) -> PolyMesh {
        let mut mesh = self.whiskers.clone();
        mesh.append(&self.horn);
        mesh
    }
}

/// Every whisker, both sides, in the head's frame.
fn whiskers(seat: &WhiskerSeat) -> PolyMesh {
    let mut mesh = PolyMesh::new();
    for side in [1.0, -1.0] {
        let mirror = Vec3::new(side, 1.0, 1.0);
        for (&root, &direction) in seat.roots.iter().zip(&seat.directions) {
            let path: Vec<Vec3> = (0..STATIONS)
                .map(|station| {
                    let along = station as f32 / (STATIONS - 1) as f32;
                    let sag = Vec3::NEG_Y * (seat.droop * seat.length * along * along);
                    (root + direction * (seat.length * along) + sag) * mirror
                })
                .collect();
            let girth = WHISKER_GIRTH * seat.length;
            mesh.append(&prim::tube(
                &path,
                Vec2::splat(girth),
                Vec2::splat(girth * 0.25),
                3,
            ));
        }
    }
    mesh
}

/// A horn in the head's frame: a cone swept to a point, two flutes twisting
/// round it as it rises.
fn horn(seat: &HornSeat) -> PolyMesh {
    let path: Vec<Vec3> = (0..STATIONS)
        .map(|station| {
            seat.base + seat.direction * (seat.length * station as f32 / (STATIONS - 1) as f32)
        })
        .collect();
    let outlines: Vec<Vec<Vec2>> = (0..STATIONS)
        .map(|station| {
            let along = station as f32 / (STATIONS - 1) as f32;
            let radius = seat.radius * (1.0 - 0.94 * along);
            let twist = std::f32::consts::TAU * HORN_TURNS * along;
            (0..HORN_SIDES)
                .map(|side| {
                    let angle = std::f32::consts::TAU * side as f32 / HORN_SIDES as f32;
                    let flute = 1.0 + 0.12 * (2.0 * angle + twist).sin();
                    Vec2::new(angle.cos(), angle.sin()) * (radius * flute)
                })
                .collect()
        })
        .collect();
    prim::sweep_outline(&path, &outlines, Vec3::X)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, Horn, QuadrupedParams};

    fn adorned(whiskers: f32, horn: Horn) -> (BeastHead, Adornments) {
        let params = QuadrupedParams {
            whiskers,
            horn,
            ..QuadrupedParams::default()
        };
        let head = params
            .skeleton(&Composites::default())
            .beast_head
            .expect("a quadruped declares its head");
        (head, Adornments::build(&head, head.origin))
    }

    #[test]
    fn a_long_fan_of_whiskers_grows_and_none_is_none() {
        let (_, whiskered) = adorned(0.6, Horn::None);
        assert!(whiskered.whiskers.face_count() > 0);
        assert!(whiskered.horn.positions.is_empty(), "no horn was asked for");
        let (_, bare) = adorned(-1.0, Horn::None);
        assert!(bare.whiskers.positions.is_empty() && bare.assembled().positions.is_empty());
    }

    #[test]
    fn whiskers_reach_out_past_the_face_on_both_sides() {
        let (head, adorned) = adorned(0.6, Horn::None);
        let across = adorned
            .whiskers
            .positions
            .iter()
            .map(|point| point.x)
            .fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        let muzzle = head.muzzle.half_width_at(head.pads.centre.z);
        assert!(
            across.1 > muzzle * 1.5 && across.0 < -muzzle * 1.5,
            "{across:?}"
        );
    }

    #[test]
    fn a_unicorn_s_horn_rises_from_its_forehead_as_a_closed_solid() {
        let (head, adorned) = adorned(-1.0, Horn::Unicorn);
        assert!(adorned.horn.is_closed_manifold(), "the horn has a hole");
        let tip = adorned
            .horn
            .positions
            .iter()
            .copied()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("a horn");
        assert!(
            tip.y > head.radius,
            "the tip stands {} m up on a {} m head",
            tip.y,
            head.radius
        );
    }

    #[test]
    fn a_horn_rises_the_same_way_however_the_head_is_carried() {
        // Up and forward as the body stands: a nodding head turns its base,
        // never where it points.
        let pointing = |neck_carriage: f32| {
            let params = QuadrupedParams {
                neck_carriage,
                horn: Horn::Unicorn,
                ..QuadrupedParams::default()
            };
            let head = params
                .skeleton(&Composites::default())
                .beast_head
                .expect("a quadruped declares its head");
            head.turn * head.horn.expect("a unicorn's horn").direction
        };
        let (low, high) = (pointing(-1.0), pointing(1.0));
        assert!(low.abs_diff_eq(high, 1.0e-4), "{low} against {high}");
        assert!(low.y > 0.7, "it rises: {low}");
    }
}
