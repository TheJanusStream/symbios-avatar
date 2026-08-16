//! Every dimension the four-legged cage is built from.
//!
//! Eight semantic axes in, one flat [`Dimensions`] out, and
//! [`crate::plan::quadruped`] does nothing afterwards but hang nodes on it. See
//! [`super`] for why this layer exists at all and what a refactor of it has to
//! prove.
//!
//! **This plan's girth is already a two-axis composite**, and the older of the
//! two in the crate: legginess slims the barrel as well as `build` and `muscle`
//! thickening it. See [`girth`], which is the shape every composite axis in
//! this crate takes.

use glam::{Vec2, Vec3};

use crate::plan::{Composites, QuadrupedParams};

/// Cross-sections of the trunk, as `(lateral, vertical)` multiples of the node's
/// radius.
///
/// The same idea as the humanoid's, and the axes come out the other way round
/// for a reason worth stating. The ring frame is transported from a world-up
/// reference, so `x` is lateral on any body; but a quadruped's spine runs
/// fore-and-aft rather than vertically, which leaves `y` pointing *up* instead
/// of forward. A galloping animal's ribcage is narrow and deep — the opposite
/// proportion to a person's, in the same two numbers.
///
/// As on the humanoid, every axis that moves moves *downward*: a socket's
/// clearance demand is its largest half-extent, so shrinking is free and growing
/// costs a joint its room. Both girdles here carry four sockets each.
const HIPS_SECTION: Vec2 = Vec2::new(0.86, 1.0);
/// See [`HIPS_SECTION`]. The deepest point of the barrel.
const SPINE_SECTION: Vec2 = Vec2::new(0.80, 1.0);
/// See [`HIPS_SECTION`].
const WITHERS_SECTION: Vec2 = Vec2::new(0.84, 1.0);
/// See [`HIPS_SECTION`]. A neck is rounder than the body it leaves.
const NECK_SECTION: Vec2 = Vec2::new(0.92, 1.0);

/// Every number the four-legged cage is built from.
///
/// **Leg positions are half-bodies**, given for the `+X` side only, and the
/// assembler mirrors them — which is what keeps the animal symmetric by
/// construction rather than by two expressions agreeing.
///
/// **Where this is bounded by the mesher rather than by anatomy:** `girth`
/// saturates at `0.50..1.80`, and each spine segment is floored at 2.5 times
/// the largest girdle it joins, so a short heavy animal stretches instead of
/// folding its two girdles into each other.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dimensions {
    /// Where the rear girdle sits.
    pub hips_at: Vec3,
    /// Radius of the rear girdle.
    pub hips_r: f32,
    /// Cross-section of the rear girdle.
    pub hips_section: Vec2,
    /// Where the deepest point of the barrel sits.
    pub spine_at: Vec3,
    /// Radius of the barrel.
    pub spine_r: f32,
    /// Cross-section of the barrel.
    pub spine_section: Vec2,
    /// Where the front girdle sits.
    pub withers_at: Vec3,
    /// Radius of the front girdle.
    pub withers_r: f32,
    /// Cross-section of the front girdle.
    pub withers_section: Vec2,
    /// Where the neck sits.
    pub neck_at: Vec3,
    /// Radius of the neck node.
    pub neck_r: f32,
    /// Cross-section of the neck node.
    pub neck_section: Vec2,
    /// Where the head sits.
    pub head_at: Vec3,
    /// Radius of the head node.
    pub head_r: f32,
    /// Where the tail leaves the rear girdle.
    pub tail_at: Vec3,
    /// Radius of the tail's first node.
    pub tail_r: f32,
    /// Where the tail ends.
    pub tip_at: Vec3,
    /// Radius of the tail's tip.
    pub tip_r: f32,
    /// Where the hind leg's upper joint sits, on the `+X` side.
    pub stifle_at: Vec3,
    /// Radius of that joint.
    pub stifle_r: f32,
    /// Where the hock sits, on the `+X` side.
    pub hock_at: Vec3,
    /// Radius of the hock.
    pub hock_r: f32,
    /// Where the hind foot sits, on the `+X` side.
    pub hind_foot_at: Vec3,
    /// Radius of the hind foot.
    pub hind_foot_r: f32,
    /// Where the fore leg's upper joint sits, on the `+X` side.
    pub knee_at: Vec3,
    /// Radius of that joint.
    pub knee_r: f32,
    /// Where the fetlock sits, on the `+X` side.
    pub fetlock_at: Vec3,
    /// Radius of the fetlock.
    pub fetlock_r: f32,
    /// Where the fore foot sits, on the `+X` side.
    pub fore_foot_at: Vec3,
    /// Radius of the fore foot.
    pub fore_foot_r: f32,
}

/// How much thicker than neutral this body is.
///
/// **Three axes, not two, and the third is the one worth keeping**: at a given
/// back height a leggy animal is a slender one and a squat animal is a barrel,
/// so legginess reduces girth rather than changing the height. Letting the two
/// move independently produces bodies that read as mistakes.
///
/// That makes this the crate's oldest composite — an axis reaching a quantity
/// it is not named for, because the correlation is real — and the pattern the
/// crate's newer composites generalise.
fn girth(params: &QuadrupedParams) -> f32 {
    // Saturated for the exploration range (#160), like the humanoid's:
    // radii scale with this while the plan's lengths do not, and the pair
    // sweep failed exactly at `build` −3 compounded by leg length. The
    // old range spans 0.59..1.69 and is untouched.
    ((1.0 + 0.28 * params.build + 0.15 * params.muscle) * (1.0 - 0.18 * params.leg_length))
        .clamp(0.50, 1.80)
}

impl Dimensions {
    /// Derives every dimension of one quadruped from its record's axes.
    /// The composites are taken and not yet read.
    ///
    /// **The frame axis is a humanoid one**: it is anchored on a
    /// measured male and female mannequin, and there is no such pair for a
    /// beast — a quadruped's dimorphism is a species question before it is a
    /// body question. The parameter is in the signature so the two plans keep
    /// one shape and so whatever reaches a beast first, most likely `mass`,
    /// has somewhere to land rather than a signature change to make.
    pub(crate) fn of(params: &QuadrupedParams, _composites: &Composites) -> Self {
        let h = params.height;
        let girth = girth(params);

        let girdle_r = h * 0.181 * girth;
        let withers_r = h * 0.190 * girth;
        let spine_r = h * 0.198 * girth;
        let neck_r = h * 0.103 * girth;

        // Spine segments are floored at a multiple of the girdles they join, so
        // a short-bodied heavy animal stretches rather than folding its girdles
        // into each other.
        let stretch = 1.0 + 0.25 * params.body_length;
        let segment_floor = 2.5 * girdle_r.max(withers_r).max(spine_r);

        let withers_z = h * 0.345 * stretch;
        let spine_z = withers_z - (h * 0.483 * stretch).max(segment_floor);
        let hips_z = spine_z - (h * 0.448 * stretch).max(segment_floor);

        let neck_z =
            withers_z + (h * 0.379 * (1.0 + 0.3 * params.neck_length)).max(withers_r * 2.4);
        let head_z = neck_z + (h * 0.276).max(neck_r * 2.2);

        let tail_reach = h * 0.45 * (1.0 + 0.6 * params.tail_length);
        let tail_z = hips_z - tail_reach.max(girdle_r * 2.7);
        let tail_r = h * 0.055 * girth;
        let tip_z = tail_z - tail_reach.max(tail_r * 2.5);

        // Each girdle spreads its legs far enough that the two sockets separate,
        // and drops far enough that the bone can carry them there.
        let rear_x = girdle_r * 1.55;
        let front_x = withers_r * 1.55;
        let rear_leg_y = h * 0.966 - girdle_r * 1.95;
        let front_leg_y = h - withers_r * 1.95;
        let foot_y = h * 0.086;

        Self {
            hips_at: Vec3::new(0.0, h * 0.966, hips_z),
            hips_r: girdle_r,
            hips_section: HIPS_SECTION,
            spine_at: Vec3::new(0.0, h, spine_z),
            spine_r,
            spine_section: SPINE_SECTION,
            withers_at: Vec3::new(0.0, h, withers_z),
            withers_r,
            withers_section: WITHERS_SECTION,
            neck_at: Vec3::new(0.0, h * 1.086, neck_z),
            neck_r,
            neck_section: NECK_SECTION,
            head_at: Vec3::new(0.0, h * 1.069, head_z),
            head_r: h * 0.129 * (1.0 + 0.25 * params.head_size),
            tail_at: Vec3::new(0.0, h * 0.862, tail_z),
            tail_r,
            tip_at: Vec3::new(0.0, h * 0.724, tip_z),
            tip_r: h * 0.031 * girth,
            stifle_at: Vec3::new(rear_x, rear_leg_y, hips_z - h * 0.017),
            stifle_r: h * 0.078 * girth,
            hock_at: Vec3::new(
                rear_x,
                foot_y + (rear_leg_y - foot_y) * 0.36,
                hips_z + h * 0.052,
            ),
            hock_r: h * 0.055 * girth,
            hind_foot_at: Vec3::new(rear_x, foot_y, hips_z + h * 0.121),
            hind_foot_r: h * 0.062 * girth,
            // Set slightly behind the withers, as a real shoulder is. Attaching
            // it at exactly the girdle's depth would leave the leg's socket ring
            // tied with the spine axis, and a hull cannot resolve four points on
            // a knife edge.
            knee_at: Vec3::new(front_x, front_leg_y, withers_z - h * 0.014),
            knee_r: h * 0.078 * girth,
            fetlock_at: Vec3::new(
                front_x,
                foot_y + (front_leg_y - foot_y) * 0.36,
                withers_z + h * 0.017,
            ),
            fetlock_r: h * 0.055 * girth,
            fore_foot_at: Vec3::new(front_x, foot_y, withers_z + h * 0.086),
            fore_foot_r: h * 0.062 * girth,
        }
    }
}
