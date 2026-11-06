//! Winged bodies' wings (#421): the membranes are closed shells, every wing
//! vertex is bound to at most four joints summing to one, a folded wing lies
//! outside the body it folds on, the extension poses continuously, every
//! family and cross builds and folds, the budget holds, and no wing is part
//! of a foot.

use symbios_avatar::plan::Blended;
use symbios_avatar::plan::Lineage;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Landmark, Limb, MAX_INFLUENCES, MeshKind, PolyMesh, Pose,
    Vec3, WingedFamily, WingedParams, Zone,
};

/// Every family, and a cross of each pair at both sides of the halfway blend.
fn lineages() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut all: Vec<(String, Lineage<WingedFamily>)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), Lineage::pure(family)))
        .collect();
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.3, 0.7] {
            all.push((
                format!("{a:?} x {b:?} {blend}"),
                Lineage::cross(a, b, blend),
            ));
        }
    }
    all
}

fn built(lineage: Lineage<WingedFamily>) -> Avatar {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    let record = AvatarRecord::new("Winged", Archetype::Winged(params));
    Avatar::build_for_shape(&record).expect("a winged body builds")
}

/// The wing parts as one mesh: membranes and digits, then feathers.
fn wing_parts(avatar: &Avatar) -> (PolyMesh, PolyMesh) {
    let wings = avatar
        .parts
        .wings
        .as_ref()
        .expect("a winged body grows wings");
    (wings.membrane.clone(), wings.feathers.clone())
}

#[test]
fn a_membrane_is_closed_shells_and_a_feathered_wing_has_none() {
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let (membrane, feathers) = wing_parts(&avatar);
        if lineage.leading() == WingedFamily::Bird {
            assert!(
                membrane.is_empty(),
                "{name}: a bird-led body has no membrane"
            );
            assert!(!feathers.is_empty(), "{name}: a bird-led body has feathers");
            continue;
        }
        assert!(
            feathers.is_empty(),
            "{name}: a membrane body has no feathers"
        );
        let report = membrane.manifold_report();
        assert!(
            report.is_clean(),
            "{name}: the membrane is not closed shells: {report:?}"
        );
    }
}

#[test]
fn every_wing_vertex_is_held_by_at_most_four_joints_summing_to_one() {
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let (membrane, feathers) = wing_parts(&avatar);
        for mesh in [&membrane, &feathers] {
            assert_eq!(mesh.skin.len(), mesh.vertex_count(), "{name}: unskinned");
            for skin in &mesh.skin {
                assert_eq!(skin.len(), MAX_INFLUENCES);
                let total: f32 = skin.iter().map(|hold| hold.weight).sum();
                assert!((total - 1.0).abs() < 1e-4, "{name}: weights sum to {total}");
                for hold in skin.iter().filter(|hold| hold.weight > 0.0) {
                    assert!((hold.joint as usize) < avatar.rig.len(), "{name}");
                }
            }
        }
    }
}

/// How many of a wing's free vertices stand inside the body's **trunk** -
/// where the body vertex nearest them is the trunk's own skin - and how many were tested, at `extension`, against the body as
/// that pose deforms it. Free: past the first fifth of its piece and off
/// the columns that ride inside a limb. A membrane rooted in an arm or a leg
/// rides inside that limb's skin where it attaches, which says nothing
/// about whether the wing passes through the body.
fn inside_trunk(avatar: &Avatar, extension: f32) -> (usize, usize) {
    let rig = &avatar.rig;
    let wings = avatar.parts.wings.as_ref().expect("wings");
    let mut pose = Pose::rest(rig);
    rig.spread_wings(&mut pose, extension, extension);
    rig.spread_tail(&mut pose, extension);
    let posed = pose.forward(rig);
    let mut body = avatar.parts.body.clone();
    body.positions = posed.deform(rig, &body.positions, &avatar.parts.weights);
    // The trunk's own skin: held nine tenths or more by the trunk's joints,
    // which no wing pose moves. Skin at an armpit or a hip, zoned to the
    // trunk but held partly by the limb, is carried out with the limb.
    let trunk = |at: Vec3| {
        let nearest = body
            .positions
            .iter()
            .enumerate()
            .min_by(|a, b| {
                a.1.distance_squared(at)
                    .total_cmp(&b.1.distance_squared(at))
            })
            .map_or(0, |(index, _)| index);
        let held: f32 = avatar.parts.weights.vertices[nearest]
            .iter()
            .filter(|hold| {
                matches!(
                    rig.joints[hold.joint as usize].zone,
                    Zone::Pelvis | Zone::Abdomen | Zone::Chest
                )
            })
            .map(|hold| hold.weight)
            .sum();
        held >= 0.9
    };
    let (mut inside, mut tested) = (0, 0);
    for (mesh, reach) in [
        (&wings.membrane, &wings.membrane_reach),
        (&wings.feathers, &wings.feather_reach),
    ] {
        let moved = posed.deform_mesh(rig, mesh);
        for (at, &out) in moved.positions.iter().zip(reach) {
            if out < 0.2 {
                continue;
            }
            tested += 1;
            if body.contains(*at) && trunk(*at) {
                inside += 1;
            }
        }
    }
    (inside, tested)
}

/// **A wing does not pass through the trunk** - asked of the geometry, since
/// a watertightness sweep passed a quadruped's limb through its body - folded,
/// partway and spread, on every family and cross, every axis at each end at
/// once and a dozen rolls: of each wing's free vertices, at most
/// [`INSIDE_SHARE`] stand inside the pelvis, the abdomen or the chest.
#[test]
fn a_wing_does_not_pass_through_the_trunk() {
    let mut bodies: Vec<(String, AvatarRecord)> = lineages()
        .into_iter()
        .map(|(name, lineage)| (name, corner(lineage, 0)))
        .collect();
    for family in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        for end in [-10.0f32, 10.0] {
            let mut params = WingedParams::default();
            params.set_lineage(Lineage::pure(family));
            params.size = if end < 0.0 { 0.0 } else { 10.0 };
            params.wing_length = end;
            params.neck_length = end;
            params.leg_length = end;
            params.tail_length = end;
            params.head_size = end;
            params.appeal = end;
            let mut record = AvatarRecord::new("Winged", Archetype::Winged(params));
            record.composites.mass = end;
            record.composites.body_fat = end;
            record.composites.age = if end > 0.0 { 80 } else { 18 };
            record.sanitize();
            bodies.push((format!("{family:?} every axis {end}"), record));
        }
    }
    for seed in 1..=12 {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        bodies.push((format!("roll {seed}"), record));
    }
    let mut worst = (String::new(), 0.0f32);
    let mut failures = Vec::new();
    for (name, record) in &bodies {
        let avatar = Avatar::build_for_shape(record).expect("a winged body builds");
        for extension in [0.0, 0.3, 0.6, 1.0] {
            let (inside, tested) = inside_trunk(&avatar, extension);
            assert!(tested > 100, "{name}: only {tested} wing vertices tested");
            let share = inside as f32 / tested as f32;
            if share > worst.1 {
                worst = (format!("{name} at {extension}"), share);
            }
            if share > INSIDE_SHARE {
                failures.push(format!(
                    "{name} at {extension}: {inside} of {tested} inside the trunk"
                ));
            }
        }
    }
    println!(
        "deepest into the trunk: {:.2}% at {}",
        worst.1 * 100.0,
        worst.0
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How many of a wing's free vertices may stand inside the trunk: a few
/// where an edge brushes it. Measured at 1.03 percent at most - four
/// vertices a wing, at the corner of a glider-led cross's patagium where its
/// free edge tucks under a bird-blended belly - and none on the families'
/// own bodies; a bird's wing folded into its trunk reads 61 percent.
const INSIDE_SHARE: f32 = 0.0125;

/// The largest step any drawn vertex takes between two of `steps` evenly
/// spaced extensions from 0 to 1, against the mean of those largest steps.
fn steps(avatar: &Avatar, steps: usize) -> (f32, f32, f32) {
    let rig = &avatar.rig;
    let at = |extension: f32| {
        let mut pose = Pose::rest(rig);
        rig.spread_wings(&mut pose, extension, extension);
        rig.spread_tail(&mut pose, extension);
        avatar
            .posed(&pose, 0.0)
            .into_iter()
            .filter(|drawn| drawn.kind != MeshKind::Eye)
            .flat_map(|drawn| drawn.mesh.positions)
            .collect::<Vec<Vec3>>()
    };
    let mut previous = at(0.0);
    let (mut worst, mut sum, mut at_worst) = (0.0f32, 0.0f32, 0.0f32);
    for step in 1..=steps {
        let extension = step as f32 / steps as f32;
        let now = at(extension);
        let moved = previous
            .iter()
            .zip(&now)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        sum += moved;
        if moved > worst {
            (worst, at_worst) = (moved, extension);
        }
        previous = now;
    }
    (worst, sum / steps as f32, at_worst)
}

/// A winged body of `lineage` at its axes' far ends: `0` as rolled, `1` the
/// longest wings on the shortest legs, `2` the longest wings, `3` the
/// heaviest, fattest and oldest composites.
fn corner(lineage: Lineage<WingedFamily>, which: usize) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    if which == 1 || which == 2 {
        params.wing_length = 10.0;
    }
    if which == 1 {
        params.leg_length = -10.0;
    }
    let mut record = AvatarRecord::new("Winged", Archetype::Winged(params));
    if which == 3 {
        record.composites.mass = 10.0;
        record.composites.body_fat = 10.0;
        record.composites.age = 80;
    }
    record.sanitize();
    record
}

/// **The extension poses continuously, at every body a record reaches**
/// (#421; the critic's sweep): over 0 to 1 in two hundred steps no drawn
/// vertex jumps - every step moves at most six times the mean step - for
/// every family and every cross at blends 0.3, 0.5 and 0.6 either way round,
/// each as rolled, with its longest wings on its shortest legs, with its
/// longest wings, and with its heaviest composites. A jump is two joints a
/// vertex blends coming more than a half turn apart partway through, where
/// dual-quaternion skinning flips one of them: it moved a glider-led cross's
/// membrane 0.76 m in one step.
#[test]
fn the_extension_poses_every_wing_continuously() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut all: Vec<(String, Lineage<WingedFamily>)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), Lineage::pure(family)))
        .collect();
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.3, 0.5, 0.6] {
            all.push((
                format!("{a:?} x {b:?} {blend}"),
                Lineage::cross(a, b, blend),
            ));
            all.push((
                format!("{b:?} x {a:?} {blend}"),
                Lineage::cross(b, a, blend),
            ));
        }
    }
    let mut failures = Vec::new();
    for (name, lineage) in all {
        for which in 0..4 {
            let record = corner(lineage, which);
            let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
            let (worst, mean, at) = steps(&avatar, 200);
            assert!(mean > 0.0, "{name} corner {which}: the wings never moved");
            if worst > mean * 6.0 {
                failures.push(format!(
                    "{name} corner {which}: a step of {worst:.3} m at {at:.3} against a mean of {mean:.4} m"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **Every family and cross builds, rests folded and spreads**: its rest is
/// the extension's `0`, its wings are recorded for both sides, and spread
/// they reach well out from the body.
#[test]
fn every_family_and_cross_builds_rests_folded_and_spreads() {
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let rig = &avatar.rig;
        let wings = rig.wings.as_ref().expect("a winged rig records its wings");
        assert_eq!(wings.sides.len(), 2, "{name}: two wings");
        let mut folded = Pose::rest(rig);
        rig.spread_wings(&mut folded, 0.0, 0.0);
        rig.spread_tail(&mut folded, 0.0);
        for (rotation, rest) in folded.rotations.iter().zip(&Pose::rest(rig).rotations) {
            assert!(
                rotation.angle_between(*rest) < 2e-3,
                "{name}: rest is not the fold"
            );
        }
        let width = |pose: &Pose| {
            let posed = pose.forward(rig);
            let xs = posed.positions.iter().map(|at| at.x);
            xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min)
        };
        let mut spread = Pose::rest(rig);
        rig.spread_wings(&mut spread, 1.0, 1.0);
        let (closed, open) = (width(&folded), width(&spread));
        assert!(
            open > closed * 1.6,
            "{name}: spread {open} m across against {closed} m folded"
        );
        // Mirror images, at rest: every wing vertex has its reflection.
        let (membrane, feathers) = wing_parts(&avatar);
        for mesh in [&membrane, &feathers] {
            let mut worst = 0.0f32;
            for at in mesh.positions.iter().step_by(7) {
                let mirror = Vec3::new(-at.x, at.y, at.z);
                let nearest = mesh
                    .positions
                    .iter()
                    .map(|other| other.distance(mirror))
                    .fold(f32::MAX, f32::min);
                worst = worst.max(nearest);
            }
            assert!(
                worst < 1e-3,
                "{name}: a wing vertex is {worst} m off its mirror"
            );
        }
        // And spread: the left wing reaches as far as the right.
        let posed = spread.forward(rig);
        let reach = |left: bool| {
            (0..rig.len())
                .filter(|&joint| {
                    matches!(rig.joints[joint].zone, Zone::Extremity(limb) if limb.is_fore() && limb.is_left() == left)
                })
                .map(|joint| posed.positions[joint].x.abs())
                .fold(0.0f32, f32::max)
        };
        assert!(
            (reach(true) - reach(false)).abs() < 1e-3,
            "{name}: the wings spread to {} and {}",
            reach(true),
            reach(false)
        );
    }
}

/// **The budget**: a winged body near a beast's ceiling, its feathers under
/// the hair's own, in the draws a beast makes.
#[test]
fn a_winged_body_fits_its_budget() {
    let mut worst = (String::new(), 0usize);
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let (_, feathers) = wing_parts(&avatar);
        assert!(
            avatar.budget.meshes <= 4,
            "{name}: {} draws",
            avatar.budget.meshes
        );
        assert!(
            feathers.triangulated().len() <= symbios_avatar::hair::fur::MAX_TRIANGLES,
            "{name}: feathers past the hair's ceiling"
        );
        if avatar.budget.tris > worst.1 {
            worst = (name, avatar.budget.tris);
        }
    }
    // And rolls, which reach the axes' ends a family's own does not.
    for seed in 1..=12 {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        let avatar = Avatar::build_for_shape(&record).expect("a roll builds");
        if avatar.budget.tris > worst.1 {
            worst = (format!("seed {seed}"), avatar.budget.tris);
        }
    }
    println!("dearest winged body: {} at {}", worst.1, worst.0);
    assert!(worst.1 <= WINGED_CEILING, "{} costs {}", worst.0, worst.1);
}

/// A winged body's ceiling (#421): 26,000 triangles, raised to 27,200 at
/// W4 (#423) so a bat-led cross wears its fur whole (`wing::MAX_TRIANGLES`).
const WINGED_CEILING: usize = 27_200;

/// A wing is no part of a foot (#421, the end review). A bat's digits, a
/// glider's spars and a bird's feathers take the zone of the arm they grow
/// from, and before `Rig::extremity_joints` left them out a bat's forefoot
/// read to the gait as reaching half a metre behind its contact, a glider's
/// a fifth of one, and the tip a prop is held at was a wing's last joint.
#[test]
fn a_wing_is_no_part_of_a_foot() {
    // At the default size a foot spans a few centimetres to a decimetre; a
    // wing folded into one added two to five decimetres.
    const FOOT: f32 = 0.15;
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let rig = &avatar.rig;
        let wings = rig.wings.as_ref().expect("a winged body has wings");
        assert!(!wings.joints.is_empty(), "{name}: its wings list no joints");
        assert!(
            wings.joints.windows(2).all(|pair| pair[0] < pair[1]),
            "{name}: its wings' joints are not ascending"
        );
        let landmarks = rig.landmarks();
        for limb in Limb::ALL {
            let Some(&contact) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            let seat = rig.joints[contact].position;
            assert!(
                rig.extremity_joints(limb)
                    .iter()
                    .all(|&joint| !wings.owns(joint)),
                "{name}: {limb:?}'s foot holds a wing's joint"
            );
            for along in [Vec3::X, Vec3::Y, Vec3::Z] {
                let (behind, ahead) = rig
                    .extremity_extent(limb, along)
                    .expect("a limb with an extremity has an extent");
                let reach = behind.max(ahead);
                assert!(
                    reach < FOOT,
                    "{name}: {limb:?}'s foot reaches {reach:.3} m along {along}"
                );
            }
            if let Some(tip) = landmarks.get(Landmark::LimbTip(limb)) {
                let reach = tip.position.distance(seat);
                assert!(
                    reach < FOOT,
                    "{name}: {limb:?}'s tip is {reach:.3} m from its contact"
                );
            }
        }
    }
}
