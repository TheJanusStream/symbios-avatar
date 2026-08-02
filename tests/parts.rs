//! Attached parts must sit on the body, not in it.
//!
//! A nose, an ear, a hand — none of these is part of the capsule graph. Each is
//! built separately and placed against the body, and every one of them has at
//! some point been placed against a number the *plan* supplied rather than one
//! the *body* had. The plan overstates the head by about a third, and by a
//! different third on every body, so a part placed that way is proud on one
//! avatar and buried on the next.
//!
//! That is the defect class this file exists to catch. It has been fixed by
//! hand three times — for eyes, for hair, and for hands — each time verified on
//! a single body, and each time it came back somewhere else. A sweep is the only
//! thing that settles it.

use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, PolyMesh, QuadrupedParams, Vec3,
};

/// Build settings for a sweep about geometry.
///
/// The atlas is dropped to a token size. Baking and painting a 1024-square skin
/// is most of what building an avatar costs — it dominated this sweep by a
/// factor of thirty — and not one texel of it bears on whether an ear is inside
/// a head.
fn geometry_only() -> AvatarConfig {
    AvatarConfig {
        atlas: 32,
        ..Default::default()
    }
}

/// How many rerolled bodies each check runs over.
const SEEDS: i64 = 16;

/// What share of a part must stand outside the body.
///
/// Not all of it: a feature is *attached*, so its root is meant to be inside.
/// Not its centroid either — a lip correctly hugging a curved face has its
/// centre-line at the surface, and demanding the centroid be outside would ask
/// for a bead stuck on the mouth. What matters is that a meaningful part of it
/// is visible, which is exactly what "buried" denies.
///
/// Measured across 32 seeds when this was written, the worst body gave: brow
/// 89%, ear 57%, nose 60%, lip 33%. A quarter leaves room for the parameter
/// space to move without leaving room for a part to vanish.
const MUST_SHOW: f32 = 0.25;

/// What share of `part`'s vertices lie outside `body`.
fn proud(body: &PolyMesh, part: &PolyMesh, offset: Vec3) -> f32 {
    if part.positions.is_empty() {
        return 1.0;
    }
    let outside = part
        .positions
        .iter()
        .filter(|point| !body.contains(**point + offset))
        .count();
    outside as f32 / part.positions.len() as f32
}

/// Every attached part of one avatar, named, with the transform placing it.
fn attached(avatar: &Avatar) -> Vec<(String, &PolyMesh, Vec3)> {
    let parts = &avatar.parts;
    let mut out: Vec<(String, &PolyMesh, Vec3)> = Vec::new();

    if let Some(features) = &parts.features {
        let head = avatar.rig.joints[features.head].position;
        out.push(("nose".into(), &features.nose, head));
        for (side, brow) in features.brows.iter().enumerate() {
            out.push((format!("brow {side}"), brow, head));
        }
        for (side, lip) in features.lips.iter().enumerate() {
            out.push((format!("lip {side}"), lip, head));
        }
        for (side, ear) in features.ears.iter().enumerate() {
            out.push((format!("ear {side}"), ear, head));
        }
    }
    for part in parts
        .extremities
        .hands
        .iter()
        .chain(&parts.extremities.feet)
    {
        let at = avatar.rig.joints[part.joint].position;
        out.push((format!("{:?}", part.limb), &part.mesh, at));
    }
    out
}

#[test]
fn no_attached_part_is_buried_in_the_body() {
    for (plan, archetype) in [
        ("biped", Archetype::default()),
        (
            "quadruped",
            Archetype::Quadruped(QuadrupedParams::default()),
        ),
    ] {
        for seed in 0..SEEDS {
            let mut record = AvatarRecord::new("Sweep", archetype.clone());
            record.reroll(seed);
            let avatar = Avatar::build_with(&record, &geometry_only()).expect("the body builds");

            for (name, part, offset) in attached(&avatar) {
                let shows = proud(&avatar.parts.body, part, offset);
                assert!(
                    shows >= MUST_SHOW,
                    "{plan} seed {seed}: {name} is {:.0}% visible, under the {:.0}% floor",
                    shows * 100.0,
                    MUST_SHOW * 100.0
                );
            }
        }
    }
}

#[test]
fn containment_is_exact_on_a_closed_body() {
    // The sweep above is only worth anything if this primitive is right, and a
    // ray-crossing test fails in exactly one interesting way: a ray that grazes
    // a shared edge counts it twice and reports an inside point as outside.
    let avatar = Avatar::build_with(&AvatarRecord::default(), &geometry_only()).expect("builds");
    let body = &avatar.parts.body;
    assert!(body.is_closed_manifold(), "the test's premise");

    // A point deep inside the torso, and one well clear of the body.
    let (lo, hi) = body.bounds();
    let middle = (lo + hi) * 0.5;
    assert!(body.contains(middle), "the centre of a body is inside it");
    assert!(
        !body.contains(hi + Vec3::splat(1.0)),
        "a point in the air is not"
    );

    // And every vertex of the body is on its own boundary, so neither answer is
    // wrong — but pushing one along its normal must land outside every time.
    let normals = body.vertex_normals();
    let reach = (hi - lo).max_element() * 0.05;
    for (vertex, point) in body.positions.iter().enumerate() {
        assert!(
            !body.contains(*point + normals[vertex] * reach),
            "vertex {vertex} pushed out along its own normal is still inside"
        );
    }
}
