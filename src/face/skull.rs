//! Giving the head a skull's shape.
//!
//! The body plan builds a head from two nodes, which sweep into a capped tube
//! and subdivide into an egg. An egg has no jaw, no chin, no cheekbones and no
//! occiput, so a face built on one reads as features stuck onto a blank — which
//! is exactly how it read.
//!
//! **The capsule graph cannot fix this, and adding nodes to it will not either.**
//! A jaw is not a tube. It is wide at the angle below the ear, narrows forward
//! through the cheek, and finishes in a chin that projects past everything above
//! it. A ball with sockets on it has no way to say that, and hanging another
//! capsule off the head to serve as a jaw produces a snout.
//!
//! So the shape is applied to the built mesh, analytically, in head-local space:
//! a breadth profile down the skull, a fore-aft elongation, an occipital
//! fullness that cuts away under the ear, and a chin. Applied to the **rest**
//! mesh before anything is bound or unwrapped, so skin weights, texture charts,
//! hair and every other attached part follow it without knowing it happened.
//!
//! Heights here are in skull radii above the head joint, which is the same unit
//! [`crate::hair::Scalp`] profiles in, and the same one the features are placed
//! in. One unit for the head, everywhere.

use glam::Vec3;

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

/// How much longer a head is than it is wide.
///
/// About a quarter, on a human. A head with a circular cross-section reads as a
/// ball however well the rest of it is shaped.
const ELONGATION: f32 = 0.24;

/// How wide the skull is at each height, relative to its unshaped width.
///
/// Widest at the cheekbones, which sit just below the eye line — not at the
/// cranium, which is where an unshaped head is widest and part of why it reads
/// as a ball.
const BREADTH: [(f32, f32); 7] = [
    (0.86, 0.62),  // crown
    (0.55, 0.90),  // upper cranium
    (0.25, 0.99),  // temples
    (-0.05, 1.03), // cheekbones, the widest part of a head
    (-0.25, 0.93), // below the cheek
    (-0.42, 0.74), // the angle of the jaw
    (-0.58, 0.46), // under the chin
];

/// How deep the skull is at each height, relative to its unshaped depth.
///
/// Separate from [`BREADTH`], and that separation is the point: run the breadth
/// profile on the fore-aft axis too and narrowing the jaw drags the chin
/// backwards, which cancels the chin out entirely. A jaw narrows across; it does
/// not retreat.
const DEPTH: [(f32, f32); 6] = [
    (0.86, 0.66),
    (0.55, 0.94),
    (0.20, 1.00),
    (-0.10, 1.00),
    (-0.42, 0.90),
    (-0.58, 0.68),
];

/// How much fuller the back of the skull is than the front, at each height.
///
/// Positive behind the ear is the occiput, which on an unshaped head is missing
/// entirely. Negative below it is the jaw cutting in — the hollow between the
/// jaw's angle and the neck, without which a head sits on its neck like a ball
/// on a post.
const OCCIPUT: [(f32, f32); 5] = [
    (0.70, 0.04),
    (0.35, 0.14),
    (0.05, 0.08),
    (-0.25, -0.10),
    (-0.55, -0.26),
];

/// How far the brow ridge stands proud, in skull radii, at each height.
///
/// The bone above the eye, not the hair on it. Without it the forehead runs
/// straight down into the eye socket and the face has no ledge for the eyes to
/// sit under — which is a large part of why a smooth head reads as a doll.
const BROW: [(f32, f32); 5] = [
    (0.58, 0.0),
    (0.42, 0.018),
    (0.28, 0.042),
    (0.14, 0.030),
    (0.02, 0.0),
];

/// How far the temples are drawn in, in skull radii, at each height.
///
/// The flat at the side of the skull between the brow and the ear. A head
/// without it is a barrel from every angle above the cheekbone.
const TEMPLE: [(f32, f32); 4] = [(0.62, 0.0), (0.40, 0.040), (0.14, 0.034), (-0.06, 0.0)];

/// How far the chin and jaw project forward at each height, in skull radii.
const CHIN: [(f32, f32); 5] = [
    (0.05, 0.0),
    (-0.20, 0.04),
    (-0.38, 0.15),
    (-0.52, 0.22),
    (-0.62, 0.12),
];

/// Shapes the head of a built body, in place.
///
/// Does nothing to a body with no head. Idempotent only in the sense that it is
/// a function of the rest positions — call it once, on the rest mesh, before
/// binding or unwrapping.
pub fn shape(mesh: &mut PolyMesh, rig: &Rig) {
    // These are a HUMAN skull's proportions — a chin, a brow ridge, cheekbones
    // widest. On something that walks on all fours they are simply wrong, in the
    // same way that giving its front legs fingers was wrong. A creature's head
    // is its own shape and belongs with the rest of the creature work.
    if rig.ground_contacts().len() > 2 {
        return;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return;
    }

    // Which vertices belong to the head, asked of the rig rather than cut by
    // height: the neck runs up into the same band and must not be reshaped, or
    // the throat pinches away from the jaw it is supposed to meet.
    let owned: Vec<bool> = mesh
        .positions
        .iter()
        .map(|&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
        .collect();

    for (point, &mine) in mesh.positions.iter_mut().zip(&owned) {
        if !mine {
            continue;
        }
        *point = centre + reshape(*point - centre, radius);
    }
}

/// Where a point on an unshaped head ends up once the skull is shaped.
///
/// Public because everything that sits on a head has to agree with it: the eyes,
/// and through them every feature anchored to the eyes. Placed against the
/// unshaped sphere they end up buried, because the shaped face is a quarter
/// further forward than the sphere it came from. One function, used both to move
/// the mesh and to place what sits on it, is the only way those two stay in
/// step.
///
/// Takes and returns a position relative to the head joint, in metres.
#[must_use]
pub fn reshape(local: Vec3, radius: f32) -> Vec3 {
    if radius <= f32::EPSILON {
        return local;
    }
    let height = local.y / radius;
    let across = Vec3::new(local.x, 0.0, local.z);
    let reach = across.length();
    if reach <= f32::EPSILON {
        return local;
    }

    // How far round the head this point is: +1 dead ahead, -1 behind.
    let facing = across.z / reach;
    let ahead = facing.max(0.0);
    let behind = (-facing).max(0.0);

    // Breadth across, depth fore and aft, and the head longer than it is wide.
    // The occiput swells the back of the cranium, and the same curve gone
    // negative lower down cuts the jaw in under the ear.
    let wide = knot(&BREADTH, height);
    let deep = knot(&DEPTH, height)
        * (1.0 + ELONGATION)
        * (1.0 + knot(&OCCIPUT, height) * behind * behind);

    // The chin is a narrow central prominence, so its push falls off much faster
    // round the jaw than the other terms do. Spread evenly across the front — an
    // `ahead` squared, as the brow uses — it carries the whole lower face
    // forward and reads as a muzzle rather than as a chin.
    let point = ahead * ahead * ahead * ahead;
    let ledge = knot(&BROW, height) * ahead * ahead;
    let hollow = knot(&TEMPLE, height) * (local.x / reach) * (local.x / reach);

    Vec3::new(
        local.x * (wide - hollow),
        local.y,
        local.z * deep + (knot(&CHIN, height) * point + ledge) * radius,
    )
}

/// Reads a piecewise-linear profile, which is given from the crown downward.
fn knot(profile: &[(f32, f32)], height: f32) -> f32 {
    let Some(&(top, first)) = profile.first() else {
        return 0.0;
    };
    if height >= top {
        return first;
    }
    for pair in profile.windows(2) {
        let ((upper, above), (lower, below)) = (pair[0], pair[1]);
        if height >= lower {
            let along = (upper - height) / (upper - lower).max(f32::EPSILON);
            return above + (below - above) * along;
        }
    }
    profile.last().map_or(0.0, |&(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn head(seed: i64) -> (PolyMesh, PolyMesh, Rig, Vec3, f32) {
        let mut record = AvatarRecord::new("Skulled", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let plain = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let mut shaped = plain.clone();
        shape(&mut shaped, &rig);
        let joint = *rig.in_zone(Zone::Head).first().expect("a head");
        let (centre, radius) = (rig.joints[joint].position, rig.joints[joint].radius);
        (plain, shaped, rig, centre, radius)
    }

    /// The widest and deepest the head gets in a band of heights.
    ///
    /// Head vertices only. The neck runs up through the same heights and is
    /// broader than a chin, so a band that takes everything reports the throat
    /// and concludes the jaw got wider.
    fn band(mesh: &PolyMesh, rig: &Rig, centre: Vec3, radius: f32, at: f32) -> (f32, f32) {
        let mut wide: f32 = 0.0;
        let mut deep: f32 = 0.0;
        for point in &mesh.positions {
            let height = (point.y - centre.y) / radius;
            if (height - at).abs() > 0.08
                || rig.joints[rig.nearest_bone(*point).joint].zone != Zone::Head
            {
                continue;
            }
            wide = wide.max((point.x - centre.x).abs());
            deep = deep.max((point.z - centre.z).abs());
        }
        (wide / radius, deep / radius)
    }

    #[test]
    fn shaping_leaves_the_topology_alone() {
        // Vertices move; nothing is added, removed or re-joined, which is what
        // lets this run before binding and unwrapping without either caring.
        let (plain, shaped, ..) = head(1);
        assert_eq!(plain.vertex_count(), shaped.vertex_count());
        assert_eq!(plain.faces, shaped.faces);
        assert!(
            shaped.is_closed_manifold(),
            "{:?}",
            shaped.manifold_report()
        );
    }

    #[test]
    fn a_shaped_head_is_longer_than_it_is_wide() {
        // The single clearest difference between a head and a ball.
        let (plain, shaped, rig, centre, radius) = head(1);
        let (was_wide, was_deep) = band(&plain, &rig, centre, radius, 0.0);
        let (wide, deep) = band(&shaped, &rig, centre, radius, 0.0);
        assert!(
            (was_deep / was_wide - 1.0).abs() < 0.06,
            "the unshaped head was already {:.2} times longer than wide",
            was_deep / was_wide
        );
        assert!(
            deep / wide > 1.12,
            "the shaped head came out only {:.2} times longer than wide",
            deep / wide
        );
    }

    #[test]
    fn the_cheekbones_are_the_widest_part_of_the_head() {
        // Not the cranium, which is where an unshaped head is widest.
        let (_, shaped, rig, centre, radius) = head(7);
        let cheek = band(&shaped, &rig, centre, radius, -0.05).0;
        assert!(
            cheek > band(&shaped, &rig, centre, radius, 0.45).0,
            "the crown was wider"
        );
        assert!(
            cheek > band(&shaped, &rig, centre, radius, -0.45).0,
            "the jaw was wider"
        );
    }

    #[test]
    fn the_jaw_narrows_toward_the_chin() {
        let (_, shaped, rig, centre, radius) = head(23);
        let cheek = band(&shaped, &rig, centre, radius, -0.05).0;
        let angle = band(&shaped, &rig, centre, radius, -0.42).0;
        let chin = band(&shaped, &rig, centre, radius, -0.56).0;
        assert!(
            angle < cheek * 0.85,
            "the jaw did not narrow: {angle} of {cheek}"
        );
        assert!(
            chin < angle * 0.80,
            "the chin did not narrow: {chin} of {angle}"
        );
    }

    #[test]
    fn the_chin_projects_further_forward_than_the_brow() {
        // A face whose chin sits behind its brow reads as receding, and an
        // unshaped head has exactly that: a sphere's widest point is its middle.
        let (plain, shaped, rig, centre, radius) = head(3);
        let front = |mesh: &PolyMesh, at: f32| {
            mesh.positions
                .iter()
                .filter(|p| {
                    ((p.y - centre.y) / radius - at).abs() < 0.09
                        && rig.joints[rig.nearest_bone(**p).joint].zone == Zone::Head
                })
                .map(|p| p.z - centre.z)
                .fold(f32::MIN, f32::max)
        };
        let gained = front(&shaped, -0.45) - front(&plain, -0.45);
        assert!(
            gained > radius * 0.05,
            "the chin only came forward by {gained}"
        );
    }

    #[test]
    fn the_back_of_the_cranium_is_fuller_than_the_back_of_the_jaw() {
        let (_, shaped, rig, centre, radius) = head(11);
        let back = |at: f32| {
            shaped
                .positions
                .iter()
                .filter(|p| {
                    ((p.y - centre.y) / radius - at).abs() < 0.09
                        && rig.joints[rig.nearest_bone(**p).joint].zone == Zone::Head
                })
                .map(|p| centre.z - p.z)
                .fold(f32::MIN, f32::max)
                / radius
        };
        assert!(
            back(0.30) > back(-0.42) * 1.25,
            "the occiput measured {} against a jaw of {}",
            back(0.30),
            back(-0.42)
        );
    }

    #[test]
    fn nothing_below_the_head_is_touched() {
        // The neck runs up into the same band of heights. Reshaping it would
        // pinch the throat away from the jaw it has to meet.
        let (plain, shaped, rig, ..) = head(5);
        for (index, (was, now)) in plain.positions.iter().zip(&shaped.positions).enumerate() {
            let _ = index;
            if rig.joints[rig.nearest_bone(*was).joint].zone != Zone::Head {
                assert_eq!(was, now, "a vertex outside the head moved");
            }
        }
    }

    #[test]
    fn shaping_the_same_mesh_twice_is_not_the_same_as_once() {
        // Stated because it is a trap rather than a feature: this is a function
        // of the REST positions, so running it on an already-shaped head shapes
        // the shaping. It belongs exactly once in the build.
        let (plain, shaped, rig, ..) = head(9);
        let mut twice = shaped.clone();
        shape(&mut twice, &rig);
        assert_ne!(twice.positions, shaped.positions);
        assert_eq!(twice.vertex_count(), plain.vertex_count());
    }

    #[test]
    fn a_body_that_walks_on_all_fours_keeps_its_own_head() {
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let plain = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");

        let mut shaped = plain.clone();
        shape(&mut shaped, &rig);
        assert_eq!(
            plain.positions, shaped.positions,
            "a quadruped was given a human chin and brow"
        );
    }

    #[test]
    fn a_profile_reads_between_its_knots() {
        assert_eq!(knot(&BREADTH, 2.0), BREADTH[0].1);
        assert_eq!(knot(&BREADTH, -5.0), BREADTH[BREADTH.len() - 1].1);
        let middle = knot(&BREADTH, (BREADTH[0].0 + BREADTH[1].0) * 0.5);
        assert!((middle - (BREADTH[0].1 + BREADTH[1].1) * 0.5).abs() < 1e-5);
    }
}
