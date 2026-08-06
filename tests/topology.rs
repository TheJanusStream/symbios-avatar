//! End-to-end topology guarantees for the cage pipeline.
//!
//! The contract every body must satisfy: the cage is a closed, consistently
//! wound 2-manifold, and stays one through subdivision. A hole means a joint
//! failed to weld to a limb; a winding conflict means a face was emitted
//! backwards. Both are silent in a renderer and fatal downstream — normals,
//! skinning, and the eventual glTF export all assume a watertight surface.

use symbios_avatar::{CageConfig, PolyMesh, Skeleton, Zone, build_cage, catmull_clark, demo};

/// Builds a cage and asserts the manifold contract, returning the mesh.
fn cage_of(skeleton: &Skeleton, label: &str) -> PolyMesh {
    let cage = build_cage(skeleton, &CageConfig::default())
        .unwrap_or_else(|error| panic!("{label} cage failed: {error}"));
    let report = cage.manifold_report();
    assert!(
        report.is_clean(),
        "{label} cage is not a closed manifold: {report:?}"
    );
    cage
}

#[test]
fn a_plain_chain_meshes_to_a_capped_tube() {
    let cage = cage_of(&demo::chain(3), "chain");
    // Three bones give four node rings plus two tip rings; five bands of one
    // quad per ring vertex, plus a cap at each end.
    //
    // The ring size is read off the cage rather than written out, because it is
    // `cage::limb::RING` and an integration test cannot name it. Writing `4`
    // here made this the last thing to fail when #107 took the ring to eight —
    // a count that has to be edited by hand every time is a count that tests the
    // editor.
    let ring = cage.vertex_count() / 6;
    assert_eq!(cage.face_count(), 5 * ring + 2);
    // Every band is a quad and the two caps are one polygon each over their own
    // ring. That used to read "a jointless limb is all quads", which was true
    // only because a cap over a four-point ring IS a quad; at eight it is an
    // octagon and nothing is wrong with it. Catmull-Clark quadrangulates both
    // either way — `lib.rs`'s own example still asserts the subdivided body is
    // all quads.
    assert_eq!(
        cage.faces.iter().filter(|face| face.len() == 4).count(),
        5 * ring,
        "every band of a jointless limb is a quad"
    );
    assert_eq!(
        cage.faces.iter().filter(|face| face.len() == ring).count(),
        2,
        "and each end is capped by a single polygon over its ring"
    );
}

#[test]
fn a_non_coplanar_joint_welds_its_limbs() {
    let cage = cage_of(&demo::tripod(), "tripod");
    assert!(cage.face_count() > 12);
}

#[test]
fn a_coplanar_joint_recovers_through_apex_points() {
    // Three limbs in one plane have no hull until the joint's own ball supplies
    // thickness. Without the apex path this skeleton cannot be meshed at all.
    let cage = cage_of(&demo::flat_tripod(), "flat tripod");
    let (lo, hi) = cage.bounds();
    assert!(
        hi.z - lo.z > 0.05,
        "apex points must give the flat joint real depth, got {:?}",
        hi.z - lo.z
    );
}

#[test]
fn the_humanoid_meshes_and_stays_closed_through_subdivision() {
    let cage = cage_of(&demo::humanoid(), "humanoid");

    let smooth = catmull_clark(&cage, 2);
    let report = smooth.manifold_report();
    assert!(
        report.is_clean(),
        "subdivided humanoid is not closed: {report:?}"
    );
    assert_eq!(smooth.quad_fraction(), 1.0, "subdivision yields all quads");

    // Proportions survive: roughly 1.75 m tall, standing on the ground.
    let (lo, hi) = smooth.bounds();
    assert!(
        (1.5..2.0).contains(&(hi.y - lo.y)),
        "humanoid height out of range: {}",
        hi.y - lo.y
    );
}

#[test]
fn the_quadruped_meshes_on_the_same_engine() {
    let cage = cage_of(&demo::quadruped(), "quadruped");
    let smooth = catmull_clark(&cage, 2);
    assert!(
        smooth.is_closed_manifold(),
        "subdivided quadruped is not closed: {:?}",
        smooth.manifold_report()
    );

    // Longer than it is tall — the girdles carried their legs without collapsing.
    let (lo, hi) = smooth.bounds();
    assert!(
        hi.z - lo.z > hi.y - lo.y,
        "quadruped should be long, not tall"
    );
}

#[test]
fn meshing_is_deterministic() {
    let skeleton = demo::humanoid();
    let first = build_cage(&skeleton, &CageConfig::default()).expect("builds");
    let second = build_cage(&skeleton, &CageConfig::default()).expect("builds");
    assert_eq!(
        first, second,
        "the same skeleton must give the same vertices"
    );
}

#[test]
fn subdivision_levels_keep_the_surface_closed() {
    let cage = cage_of(&demo::tripod(), "tripod");
    let mut faces = cage.face_count();
    for level in 1..=3 {
        let smooth = catmull_clark(&cage, level);
        assert!(
            smooth.is_closed_manifold(),
            "level {level} broke the manifold: {:?}",
            smooth.manifold_report()
        );
        assert!(smooth.face_count() > faces, "level {level} must refine");
        faces = smooth.face_count();
    }
}

#[test]
fn every_vertex_is_used_by_a_face() {
    // Orphaned vertices mean an apex point or ring was allocated and abandoned,
    // which would quietly bloat exports.
    let cage = cage_of(&demo::humanoid(), "humanoid");
    let mut used = vec![false; cage.vertex_count()];
    for face in &cage.faces {
        for &index in face {
            used[index as usize] = true;
        }
    }
    let orphans: Vec<usize> = used
        .iter()
        .enumerate()
        .filter(|&(_, &seen)| !seen)
        .map(|(index, _)| index)
        .collect();
    assert!(orphans.is_empty(), "orphaned vertices: {orphans:?}");
}

#[test]
fn a_crowded_joint_reports_which_bone_to_lengthen() {
    use symbios_avatar::{CageError, Node, Vec3};

    // A fat hub with a stubby third limb: the socket cannot slide far enough
    // along its own bone to clear its siblings.
    let mut skeleton = Skeleton::new();
    let hub = skeleton.add_node(Node::new(Vec3::ZERO, 0.5));
    skeleton.extend_from(hub, Node::new(Vec3::new(0.0, 1.2, 0.0), 0.45));
    skeleton.extend_from(hub, Node::new(Vec3::new(0.0, -1.2, 0.0), 0.45));
    skeleton.extend_from(hub, Node::new(Vec3::new(0.22, 0.0, 0.0), 0.05));

    match build_cage(&skeleton, &CageConfig::default()) {
        Err(CageError::SocketNotOnHull {
            needed, available, ..
        }) => {
            assert!(
                needed > available,
                "the error must show the shortfall: {needed} vs {available}"
            );
        }
        other => panic!("expected a crowded-joint diagnostic, got {other:?}"),
    }
}

#[test]
fn the_demo_skeletons_carry_no_zones() {
    // #54. This was stated in `demo`'s module docs and believed by nobody who
    // needed to believe it: a reviewer audited UV charts on `demo::humanoid`
    // and reported an anisotropy defect that is not there, because an unzoned
    // body unwraps into a handful of charts where a real one unwraps into
    // thirty. Prose did not hold the line, so this does.
    //
    // Asserting the ABSENCE is what makes this a check rather than a comment.
    // The tempting version — assert the cage still meshes — passes whatever the
    // zones say, since the mesher does not read them at all.
    for (label, skeleton) in [
        ("humanoid", demo::humanoid()),
        ("quadruped", demo::quadruped()),
        ("tripod", demo::tripod()),
        ("flat tripod", demo::flat_tripod()),
        ("chain", demo::chain(3)),
    ] {
        for (index, node) in skeleton.nodes.iter().enumerate() {
            assert_eq!(
                node.zone,
                Zone::default(),
                "{label} node {index} carries a zone. These are mesher fixtures: \
                 the head here is one node where a plan body's is two, so zoning \
                 it runs the skull profiles down a capped tube and produces a \
                 fixture that lies quietly instead of one that is obviously \
                 empty. Add zone-driven coverage to tests/plan.rs instead."
            );
        }
    }
}
