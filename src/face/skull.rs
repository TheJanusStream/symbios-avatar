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

/// The region each refinement pass covers: how far round the head it reaches as
/// a cosine of the angle from dead ahead, then its lowest and highest point in
/// skull radii above the head joint.
///
/// **Graded, not uniform, and that is what makes it affordable.** Refining the
/// whole front of the head twice costs 2,660 triangles and spends most of them
/// on a forehead and a pair of cheeks, which carry nothing. The first pass is
/// broad — the front of the head plus the sides that hold the temples and the
/// jaw, stopping around the ears so the boundary between fine and coarse
/// geometry falls where the hair usually is. The second is only the band the
/// features actually occupy, brow to chin.
///
/// Measured, not guessed: at one pass the median edge under the brow is 12.7 mm
/// and a brow ridge is 10 mm tall, so a whole feature spans one quad. At two it
/// is 6.2 mm.
const FACE_PASSES: [(f32, f32, f32); 2] = [(0.25, -1.05, 0.60), (0.55, -0.90, 0.50)];

/// Gives the face enough surface to carry features, before anything shapes it.
///
/// The head arrives from the cage as a four-sided tube. Subdivided twice it is
/// 189 faces with a mean edge of 24 mm, and every feature a face needs is at or
/// below that: a brow ridge is 10 mm tall and a nose one quad wide. Nothing can
/// be shaped into a surface that has no vertices where the shape goes (#59).
///
/// Refines only the front of the head, because the cost is triangles and the
/// back of a skull carries nothing. Runs BEFORE [`shape`], so the vertices it
/// adds are placed on the sphere and then mapped onto the skull by [`reshape`]
/// along with every other one — which samples the skull more finely, rather than
/// subdividing the facets of an already-shaped one.
///
/// Does nothing to a body with no head, or to one that walks on four legs: this
/// is a human skull's geometry and a creature's head is its own shape.
#[must_use]
pub fn refine_face(mesh: &PolyMesh, rig: &Rig, levels: usize) -> PolyMesh {
    if levels == 0 || rig.ground_contacts().len() > 2 {
        return mesh.clone();
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return mesh.clone();
    };
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return mesh.clone();
    }

    let mut refined = mesh.clone();
    for pass in 0..levels {
        // Passes past the last named one repeat the tightest region rather than
        // widening again, so asking for more resolution never spends it on a
        // forehead.
        let (reach, low, high) = FACE_PASSES[pass.min(FACE_PASSES.len() - 1)];
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| {
                let at = refined.face_centroid(face);
                // Asked of the rig rather than cut by height, for the same
                // reason `shape` does: the neck runs up into the same band and
                // refining it would spend triangles on a throat.
                if rig.joints[rig.nearest_bone(at).joint].zone != Zone::Head {
                    return false;
                }
                let local = at - centre;
                let height = local.y / radius;
                if height < low || height > high {
                    return false;
                }
                let across = Vec3::new(local.x, 0.0, local.z);
                let span = across.length();
                span > f32::EPSILON && across.z / span > reach
            })
            .collect();
        refined = refined.refine(&selected);
    }
    refined
}

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

    /// Where the built body's surface actually is, by bisecting on the mesh
    /// itself along `axis` from the head joint.
    ///
    /// The only thing here that does not go through [`Skull`], and so the only
    /// thing that can catch [`Skull`] being wrong. `contains` is the same
    /// primitive `tests/parts.rs` judges a buried feature with.
    fn probe(mesh: &PolyMesh, from: Vec3, axis: Vec3) -> Option<f32> {
        let inside = |reach: f32| mesh.contains(from + axis * reach);
        if !inside(0.0) || inside(0.3) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, 0.3f32);
        for _ in 0..40 {
            let middle = (near + far) * 0.5;
            if inside(middle) {
                near = middle
            } else {
                far = middle
            }
        }
        Some(near)
    }

    /// A measured skull, and the head it was measured from.
    fn skull(seed: i64, levels: usize) -> (PolyMesh, Skull, Vec3) {
        let mut record = AvatarRecord::new("Skulled", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let mut mesh = refine_face(&catmull_clark(&cage, 2), &rig, levels);
        shape(&mut mesh, &rig);
        let measured = Skull::measure(&mesh, &rig).expect("a humanoid has a skull");
        let centre = rig.joints[measured.head].position;
        (mesh, measured, centre)
    }

    #[test]
    fn the_profile_agrees_with_the_surface_it_was_measured_from() {
        // The whole contract. Every figure below is millimetres, and every one
        // of them was a measurement before it was an assertion (#67). Measured
        // over sixteen seeds and thirteen heights each: the midline depth runs
        // -1.7 to +4.0, the width -2.2 to +5.8, and the depth off the midline
        // +0.1 to +9.0. The bounds here are those with room to move.
        //
        // ASYMMETRIC ON PURPOSE, both ways. A bin's answer is the outermost
        // sample in it, so it sits slightly OUTSIDE the surface at the bin's
        // centre — hence the wide upper bound. And the failure this guards
        // against is a profile that reports the head too NARROW, which is what
        // buries a feature; too wide only stands one off. The vertex-binned
        // profile this replaced would fail the lower bound at four of these
        // thirteen heights on the first seed alone.
        for seed in 0..6 {
            let (mesh, skull, centre) = skull(seed, 1);
            let (lo, hi) = skull.span();
            for step in 0..=12 {
                let height = lo + (hi - lo) * (0.15 + 0.55 * step as f32 / 12.0);
                let from = centre + Vec3::Y * height;

                if let Some(surface) = probe(&mesh, from, Vec3::Z) {
                    let error = (skull.depth(height) - surface) * 1000.0;
                    assert!(
                        (-4.0..9.0).contains(&error),
                        "seed {seed} at {height:.3}: the midline depth is {error:.1} mm out"
                    );
                }
                if let Some(surface) = probe(&mesh, from, Vec3::X) {
                    let error = (skull.width_across(height, 0.0) - surface) * 1000.0;
                    assert!(
                        (-4.0..9.0).contains(&error),
                        "seed {seed} at {height:.3}: the width is {error:.1} mm out"
                    );
                }
                // Off the midline, where a mouth's corners sit and where the
                // per-band normalisation earns its keep.
                let across = skull.half_width(height) * 0.5;
                if let Some(surface) = probe(&mesh, from + Vec3::X * across, Vec3::Z) {
                    let error = (skull.depth_across(height, across) - surface) * 1000.0;
                    assert!(
                        (-4.0..14.0).contains(&error),
                        "seed {seed} at {height:.3}: the depth off the midline is {error:.1} mm out"
                    );
                }
            }
        }
    }

    #[test]
    fn refining_the_face_does_not_move_the_profile() {
        // Refinement adds vertices and moves none, so the surface is unchanged
        // and the measurement of it must be too. It was NOT: binning the raw
        // vertex list, the half-width at the ear line moved eleven millimetres
        // between one refinement pass and two while the mesh moved half of one,
        // and an ear seated from it fell from 53% visible to 11% (#67).
        //
        // Eight millimetres, not zero, and the gap is a known residual rather
        // than slack. On five seeds in sixteen, refining re-labels one row of
        // vertices at the jaw from neck-owned to head-owned, and the measured
        // chin steps 6.6 mm lower — the surface is identical, the LABELLING of
        // it is finer. That slides the band grid, which moves the profile by up
        // to 4.2 mm where the head's width is changing fastest. Mean movement
        // is 0.3 mm. It is bounded, it does not bite in the one refinement
        // setting that ships, and fixing it means cutting the head from the
        // skeleton instead of by nearest bone, which would redefine the chin
        // every feature height is measured down from.
        for seed in 0..6 {
            let (_, coarse, centre) = skull(seed, 1);
            let (_, fine, _) = skull(seed, 2);
            let (lo, hi) = coarse.span();
            for step in 0..=12 {
                let height = lo + (hi - lo) * (0.15 + 0.55 * step as f32 / 12.0);
                let _ = centre;
                let moved =
                    (fine.width_across(height, 0.0) - coarse.width_across(height, 0.0)).abs();
                assert!(
                    moved < 0.008,
                    "seed {seed} at {height:.3}: refining moved the width {:.1} mm",
                    moved * 1000.0
                );
                let deeper = (fine.depth(height) - coarse.depth(height)).abs();
                assert!(
                    deeper < 0.008,
                    "seed {seed} at {height:.3}: refining moved the depth {:.1} mm",
                    deeper * 1000.0
                );
            }
        }
    }

    #[test]
    fn the_width_falls_away_behind_the_cheekbone() {
        // The axis this profile gained for the ear. If it answered the same at
        // every depth it would be the band maximum again under a longer name,
        // and the test would pass while measuring nothing.
        let (_, skull, _) = skull(3, 1);
        let (lo, hi) = skull.span();
        let height = lo + (hi - lo) * 0.45;
        let reach = (hi - lo) * 0.25;
        let front = skull.width_across(height, reach * 0.5);
        let back = skull.width_across(height, -reach);
        assert!(
            back < front,
            "the head was no narrower behind the cheek: {back:.4} against {front:.4}"
        );
        assert!(
            back > front * 0.5,
            "the head fell away implausibly fast: {back:.4} against {front:.4}"
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

/// How many heights the measured profile is sampled at.
const BANDS: usize = 20;

/// How many lateral columns the depth map is sampled at.
///
/// A face is not equally deep across its width, and a feature that spans it —
/// a mouth reaches nearly two eye-widths — cannot be placed from a profile that
/// only knows height. Placed against the midline depth, a lip's corners sit
/// well proud of a face that has curved away from them, or well inside one that
/// has not.
///
/// **Columns are a fraction of the band's own width, not of the head's.** Scaled
/// against one figure for the whole head, the chin — which is a third as wide as
/// the cheekbones — puts every sample it has into the first two columns and
/// leaves the rest to be filled in from them, so the map answers with the
/// midline wherever it is asked. Measured that way the forward reach came back
/// 14 mm too deep at the chin against 5 mm at the cheek (#67).
///
/// Fifteen, not nine, and not twenty-one. A bin's answer is the outermost sample
/// in it, so halving the bin halves how far the surface has curved away inside
/// it: nine columns reported the face 4.2 mm too deep on average, fifteen
/// 1.9 mm. At twenty-one the bins start coming up empty and are filled from a
/// neighbour instead — the mean keeps falling, to 1.1 mm, while the worst case
/// turns round and becomes 16 mm too *shallow*. The mean is the wrong thing to
/// tune against; the tail is what buries a lip.
const COLUMNS: usize = 15;

/// How many fore-aft columns the width map is sampled at.
///
/// The mirror of [`COLUMNS`], and it exists for the ear: an ear sits on the side
/// of the head and about a third of an eye-radius *behind* the midline, where
/// the head is a couple of millimetres narrower than at the cheekbone in front
/// of it. A width that only knows height reports the cheekbone (#67).
///
/// Normalised per band for the same reason [`COLUMNS`] is.
const DEPTHS: usize = 15;

/// How far either side of a bin's centre a sample still counts, in bins.
///
/// Half a bin, so bins share only their boundaries — a sample exactly between
/// two centres counts for both, and nothing else does.
///
/// **Wider is not safer.** [`crate::hair::Scalp`] carries three quarters of a
/// bin, which is right there: it needs a profile that clears the head
/// *everywhere*, so overstating is the safe direction. This one is a
/// measurement, and a maximum taken over a wide window is not a measurement of
/// the middle of it. Measured at three quarters, the face came back 2.2 mm too
/// wide and 7.9 mm too deep off the midline; at a half, 0.9 mm and 4.2 mm, with
/// no bins left empty and the ear's visibility unchanged between refinement
/// passes either way.
const WINDOW: f32 = 0.5;

/// The skull as it was actually built.
///
/// [`shape`] carves a jaw, a chin, a brow and an occiput into a subdivided egg,
/// and [`reshape`] says where a point on the *planned* sphere lands under that
/// carving. Neither answers the question a feature actually asks, which is
/// "where is the surface, at this height, in this direction" — because
/// subdivision has already pulled the mesh well inside the node radius the plan
/// named, by a factor that depends on the body.
///
/// So this measures it. Two profiles against height in head-local metres: how
/// far the surface reaches sideways, and how far it reaches forward. Ears were
/// placed against the planned radius and sat 11 to 39 mm *inside* the head on
/// every seed measured; lips were buried on some bodies and proud on others,
/// which is what a guess looks like when the thing guessed at varies.
///
/// The same argument as [`crate::hair::Scalp`], and for the same reason: measure
/// the body in hand rather than the plan that asked for it.
///
/// Sampled from the **surface**, not from the vertex list. A head carries a few
/// hundred vertices and a feature is placed to within a couple of millimetres,
/// so a bin holding two or three quad corners reports a number that is several
/// millimetres under the surface and jumps about as the mesh is refined. Face
/// centroids and edge midpoints cost nothing and take the sample count up by
/// nearly an order of magnitude; overlapping bins do the rest.
#[derive(Clone, Debug, PartialEq)]
pub struct Skull {
    /// The head joint everything on the face hangs from.
    pub head: usize,
    lo: f32,
    hi: f32,
    across: [f32; BANDS],
    ahead: [f32; BANDS],
    behind: [f32; BANDS],
    front: [[f32; COLUMNS]; BANDS],
    side: [[f32; DEPTHS]; BANDS],
}

impl Skull {
    /// Measures a built head.
    ///
    /// Returns `None` for a body with no head, or one carrying too little
    /// surface to profile.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, rig: &Rig) -> Option<Self> {
        let head = *rig.in_zone(Zone::Head).first()?;
        let centre = rig.joints[head].position;
        let mine = samples(mesh, rig, centre);
        if mine.len() < BANDS {
            return None;
        }

        let (lo, hi) = mine.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        if hi - lo <= f32::EPSILON {
            return None;
        }
        let height = |point: &Vec3| (point.y - lo) / (hi - lo) * (BANDS - 1) as f32;

        let mut across = [0.0f32; BANDS];
        let mut ahead = [f32::MIN; BANDS];
        let mut behind = [f32::MAX; BANDS];
        for point in &mine {
            for band in window(height(point), BANDS) {
                across[band] = across[band].max(point.x.abs());
                ahead[band] = ahead[band].max(point.z);
                behind[band] = behind[band].min(point.z);
            }
        }
        // A band with no sample in it takes its neighbour's, so a sparse sample
        // never reports a skull that pinches to nothing.
        fill(&mut across);
        carry(&mut ahead, f32::MIN);
        carry(&mut behind, f32::MAX);

        // The same reaches, now also binned across the other axis: forward reach
        // by how far across the face it was measured, and lateral reach by how
        // far back. Mirrored into one half, because a head is symmetric and
        // folding doubles the samples in every bin.
        let mut front = [[f32::MIN; COLUMNS]; BANDS];
        let mut side = [[f32::MIN; DEPTHS]; BANDS];
        for point in &mine {
            for band in window(height(point), BANDS) {
                let lateral = lateral(across[band], point.x.abs());
                for column in window(lateral, COLUMNS) {
                    front[band][column] = front[band][column].max(point.z);
                }
                let fore = fore(behind[band], ahead[band], point.z);
                for column in window(fore, DEPTHS) {
                    side[band][column] = side[band][column].max(point.x.abs());
                }
            }
        }
        // Each row falls back to ITS OWN band. Falling back to one shared value
        // puts the bottom of the skull's depth on every band that happened to
        // sample thinly.
        for (band, row) in front.iter_mut().enumerate() {
            spread(row, ahead[band]);
        }
        for (band, row) in side.iter_mut().enumerate() {
            spread(row, across[band]);
        }

        Some(Self {
            head,
            lo,
            hi,
            across,
            ahead,
            behind,
            front,
            side,
        })
    }

    /// How far the skull reaches sideways at `height`, in head-local metres.
    ///
    /// The widest the head gets anywhere in that band of heights. Use
    /// [`Self::width_across`] for anything seated at a known depth, which on a
    /// head means anything behind the cheekbone.
    #[must_use]
    pub fn half_width(&self, height: f32) -> f32 {
        self.sample(&self.across, height)
    }

    /// How far the skull reaches sideways at `height`, `depth` in front of the
    /// head joint.
    ///
    /// All three in head-local metres; the answer is a half-width, a head being
    /// symmetric. The mirror of [`Self::depth_across`], and it exists for the
    /// same reason: a head is no more a cylinder than it is a surface of
    /// revolution. An ear seats about a third of an eye-radius behind the
    /// midline, where the skull has already begun to fall away, and
    /// [`Self::half_width`] answers there with the cheekbone in front of it
    /// (#67).
    #[must_use]
    pub fn width_across(&self, height: f32, depth: f32) -> f32 {
        self.bilinear(&self.side, height, |band| {
            fore(self.behind[band], self.ahead[band], depth)
        })
    }

    /// How far the skull reaches forward at `height`, in head-local metres.
    ///
    /// Measured on the midline. Use [`Self::depth_across`] for anything wide
    /// enough that the face curves away beneath it.
    #[must_use]
    pub fn depth(&self, height: f32) -> f32 {
        self.sample(&self.ahead, height)
    }

    /// How far the skull reaches forward at `height`, `across` from the midline.
    ///
    /// Both in head-local metres; `across` is taken by magnitude, a head being
    /// symmetric. This is what a mouth or a brow needs: a feature two eye-widths
    /// across sits on a surface that has curved back by several millimetres at
    /// its corners, and placing those corners at the midline depth leaves them
    /// standing off the face.
    #[must_use]
    pub fn depth_across(&self, height: f32, across: f32) -> f32 {
        self.bilinear(&self.front, height, |band| {
            lateral(self.across[band], across)
        })
    }

    /// The lowest and highest the measured profile reaches, in head-local metres.
    #[must_use]
    pub fn span(&self) -> (f32, f32) {
        (self.lo, self.hi)
    }

    /// Reads a two-axis table at a height and an already-scaled column.
    ///
    /// One walk for both maps, so the fore-aft one cannot drift from the lateral
    /// one the way two hand-written interpolations would.
    fn bilinear<const COLS: usize>(
        &self,
        table: &[[f32; COLS]; BANDS],
        height: f32,
        column: impl Fn(usize) -> f32,
    ) -> f32 {
        let at = ((height - self.lo) / (self.hi - self.lo) * (BANDS - 1) as f32)
            .clamp(0.0, (BANDS - 1) as f32);
        let band = (at.floor() as usize).min(BANDS - 2);
        let blend = at - band as f32;

        // The column is asked of each band separately, because the axis it
        // indexes is a fraction of THAT band's own extent. Taking one column
        // index for both is what put the chin's whole width into two columns.
        let row = |band: usize| {
            let column = column(band).clamp(0.0, (COLS - 1) as f32);
            let left = (column.floor() as usize).min(COLS - 2);
            let along = column - left as f32;
            table[band][left] + (table[band][left + 1] - table[band][left]) * along
        };
        row(band) + (row(band + 1) - row(band)) * blend
    }

    /// Reads one profile at a height, interpolating between bands.
    fn sample(&self, profile: &[f32; BANDS], height: f32) -> f32 {
        let at = ((height - self.lo) / (self.hi - self.lo) * (BANDS - 1) as f32)
            .clamp(0.0, (BANDS - 1) as f32);
        let band = (at.floor() as usize).min(BANDS - 2);
        let blend = at - band as f32;
        profile[band] + (profile[band + 1] - profile[band]) * blend
    }
}

/// Where a lateral offset falls in a band's own columns.
///
/// Zero on the midline, [`COLUMNS`] minus one at the band's widest. Anything
/// wider than the band clamps, which is the honest answer: a mouth wider than
/// the jaw it sits on has its corners at the edge of the jaw.
fn lateral(width: f32, across: f32) -> f32 {
    (across.abs() / width.max(f32::EPSILON)) * (COLUMNS - 1) as f32
}

/// Where a fore-aft offset falls in a band's own columns.
///
/// Zero at the back of the band, [`DEPTHS`] minus one at the front.
fn fore(behind: f32, ahead: f32, depth: f32) -> f32 {
    ((depth - behind) / (ahead - behind).max(f32::EPSILON)) * (DEPTHS - 1) as f32
}

/// Every bin whose window covers `at`, clamped to the table.
///
/// Samples outside the table entirely still land in the nearest bin: a head is
/// convex enough that the edge of a profile is a better answer than a hole.
fn window(at: f32, bins: usize) -> std::ops::RangeInclusive<usize> {
    let last = bins - 1;
    let at = at.clamp(0.0, last as f32);
    let first = (at - WINDOW).ceil().max(0.0) as usize;
    let end = ((at + WINDOW).floor().max(0.0) as usize).min(last);
    // A window narrower than half a bin can fall between two centres, and a
    // sample that lands in no bin at all is a sample thrown away.
    if end < first {
        let nearest = (at.round() as usize).min(last);
        return nearest..=nearest;
    }
    first..=end
}

/// The head's surface, in head-local metres, as points to bin.
///
/// Vertices, plus each fully-head face's centroid and the midpoint of each of
/// its edges and of each corner-to-centroid span. Those interior samples are on
/// the same surface [`PolyMesh::contains`] tests against, and they are what
/// takes a 283-vertex head to something a twenty-band profile can be read from.
///
/// Only the head's own surface. A vertex whose nearest bone is the neck belongs
/// to the neck however close to the jaw it sits, and including it would report
/// the throat as the face. Faces are held to a stricter rule than vertices —
/// every corner head-owned, not just the centroid — because a face straddling
/// the jaw would otherwise drag samples off the throat into the lowest bands,
/// which is exactly where the chin is read from.
fn samples(mesh: &PolyMesh, rig: &Rig, centre: Vec3) -> Vec<Vec3> {
    let mine = |point: Vec3| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head;
    let owned: Vec<bool> = mesh.positions.iter().map(|&point| mine(point)).collect();

    let mut out: Vec<Vec3> = mesh
        .positions
        .iter()
        .zip(&owned)
        .filter(|&(_, &ours)| ours)
        .map(|(&point, _)| point - centre)
        .collect();

    for (face, corners) in mesh.faces.iter().enumerate() {
        if !corners.iter().all(|&corner| owned[corner as usize]) {
            continue;
        }
        let centroid = mesh.face_centroid(face);
        out.push(centroid - centre);
        for (at, &corner) in corners.iter().enumerate() {
            let here = mesh.positions[corner as usize];
            let next = mesh.positions[corners[(at + 1) % corners.len()] as usize];
            out.push((here + next) * 0.5 - centre);
            out.push((here + centroid) * 0.5 - centre);
        }
    }
    out
}

/// Fills a row's empty columns from the nearest measured one.
fn spread(row: &mut [f32], fallback: f32) {
    let mut last = f32::MIN;
    for value in row.iter_mut() {
        if *value == f32::MIN {
            *value = last;
        } else {
            last = *value;
        }
    }
    let mut next = f32::MIN;
    for value in row.iter_mut().rev() {
        if *value == f32::MIN {
            *value = next;
        } else {
            next = *value;
        }
    }
    for value in row.iter_mut() {
        if *value == f32::MIN {
            *value = fallback;
        }
    }
}

/// Replaces bands that no sample reached with the nearest one that was.
///
/// Separate from [`fill`] because these profiles are signed — the back of a
/// skull is a negative depth — so "empty" cannot be spelled as "not positive".
fn carry(profile: &mut [f32; BANDS], empty: f32) {
    let Some(first) = profile.iter().position(|value| *value != empty) else {
        profile.fill(0.0);
        return;
    };
    for band in 0..first {
        profile[band] = profile[first];
    }
    for band in first + 1..BANDS {
        if profile[band] == empty {
            profile[band] = profile[band - 1];
        }
    }
}

/// Replaces empty bands with the nearest filled one.
fn fill(profile: &mut [f32; BANDS]) {
    let mut last = 0.0f32;
    for value in profile.iter_mut() {
        if *value <= 0.0 {
            *value = last;
        } else {
            last = *value;
        }
    }
    let mut next = 0.0f32;
    for value in profile.iter_mut().rev() {
        if *value <= 0.0 {
            *value = next;
        } else {
            next = *value;
        }
    }
}
