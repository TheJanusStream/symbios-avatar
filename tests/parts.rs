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
///
/// Only ears and extremities are weighed against it now. An ear that hugs the
/// head is less visible than one hanging off it, and the worst seed sits at
/// about 65% since the ear was conformed to the measured surface (#67).
const MUST_SHOW: f32 = 0.25;

/// What share of an eye may stand outside the face, and what share must.
///
/// **A ceiling, which this file did not have.** Everything above [`MUST_SHOW`]
/// is a floor, and a floor cannot see a part that is too far OUT — so a globe
/// with 59% of its surface outside the skin, standing 30 mm proud of the face
/// around it, passed every check in this crate with room to spare (#73). It was
/// also never checked at all: [`attached`] collected ears, hands and feet, and
/// the eyes were not in the sweep.
///
/// Both bounds, and the lower one is not padding. A ceiling on its own is the
/// same defect mirrored: a globe sunk a whole radius behind the skin scores 0%
/// outside and 0 mm proud, and so does an eye that was never built.
///
/// The face carries no eye opening — the body is a closed surface and the lids
/// are separate meshes — so whatever part of the globe lies outside the skin is
/// very nearly what a viewer sees. A real eye shows about a sixth of its
/// surface, which is what puts the ceiling here rather than near zero.
const EYE_SHOWS: (f32, f32) = (0.03, 0.25);

/// How far an eye may stand proud of the face around it, in metres.
///
/// Measured on the shipped build: 30.4 mm on the default and 24.2–29.8 across
/// seeds, which is 0.18–0.26 of a head radius on every one of them — systematic,
/// not a tuning slip. In life the corneal apex sits roughly level with the
/// surrounding lids, so anything past a few millimetres is a bulge.
const EYE_PROUD: f32 = 0.005;

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

    // Ears only, since #59: the nose, the brows and the lips are displacements
    // of the body's own surface now, so "is it buried in the body" is not a
    // question that can be asked of them — they ARE the body. What replaced this
    // check for them is in `face::relief`, which measures how far the surface
    // moved rather than how much of a solid is outside another one.
    if let Some(features) = &parts.features {
        let head = avatar.rig.joints[features.head].position;
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
#[ignore = "the target, not the state: the eye is placed by a prediction that is 26 mm wrong off the midline (#76)"]
fn an_eye_is_seated_in_the_face_rather_than_resting_on_it() {
    // The instrument that did not exist for the defect the owner reported as
    // "the eyes look popped out" (#73). Two quantities, because they fail
    // independently: how much of the globe is outside the face, and how far the
    // furthest part of it stands from the face around it. A globe can be mostly
    // inside and still poke through like a knuckle.
    //
    // Measured on the shipped build, so the failure message is the diagnosis:
    // 41–69% outside and 24–30 mm proud, on every seed.
    for seed in 0..SEEDS {
        let mut record = AvatarRecord::new("Sweep", Archetype::default());
        record.reroll(seed);
        let avatar = Avatar::build_with(&record, &geometry_only()).expect("the body builds");
        let Some(eyes) = &avatar.parts.eyes else {
            continue;
        };
        let body = &avatar.parts.body;
        let centre = avatar.rig.joints[eyes.head].position;

        for (side, eye) in [("left", &eyes.left), ("right", &eyes.right)] {
            let shows = proud(body, &eye.globe, centre);
            assert!(
                (EYE_SHOWS.0..=EYE_SHOWS.1).contains(&shows),
                "seed {seed}: the {side} eye has {:.0}% of its surface outside the face, \
                 against the {:.0}–{:.0}% a seated eye shows",
                shows * 100.0,
                EYE_SHOWS.0 * 100.0,
                EYE_SHOWS.1 * 100.0
            );

            // How far the globe's front pole stands past the skin on its own
            // column — bisected, because the surface curves fast enough across
            // the eye that the nearest VERTEX is a different question (#71).
            let column = Vec3::new(centre.x + eye.pivot.x, centre.y + eye.pivot.y, centre.z);
            if !body.contains(column) {
                continue;
            }
            let (mut near, mut far) = (0.0f32, 0.30f32);
            for _ in 0..30 {
                let mid = 0.5 * (near + far);
                if body.contains(column + Vec3::Z * mid) {
                    near = mid;
                } else {
                    far = mid;
                }
            }
            let stands = (eye.pivot.z + eye.radius) - (column.z + near - centre.z);
            assert!(
                stands <= EYE_PROUD,
                "seed {seed}: the {side} eye stands {:.1} mm proud of the face around it, \
                 against a ceiling of {:.1}",
                stands * 1000.0,
                EYE_PROUD * 1000.0
            );
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

    // And every vertex of the body is on its own boundary, so a vertex pushed
    // along its own normal must ESCAPE the body within a couple of millimetres.
    //
    // Not "must land outside at one fixed distance", which is what this said
    // for a long time, because no fixed distance exists (#72). It pushed 5% of
    // the body — 82 mm — which quietly assumed no overhang on the body faces
    // the body; then the nose gained a real underside, whose normals aim up
    // and back through the skull, and three of its vertices pushed 82 mm
    // landed inside the head. And the small distances are no better: at 0.7 mm
    // a temple vertex sits stably inside the fold where the temple hollow
    // meets the cheekbone — geometry this change never touched. A face is made
    // of overhangs and near-touching folds, so the invariant that survives is
    // the escape: an outward normal leaves its own flesh quickly, and a vertex
    // that stays buried for 2 mm in every direction sample means an inverted
    // normal or a self-intersection, which is what this is for.
    let normals = body.vertex_normals();
    let step = (hi - lo).max_element() * 0.0002;
    for (vertex, point) in body.positions.iter().enumerate() {
        let escapes =
            (1..=10).any(|out| !body.contains(*point + normals[vertex] * step * out as f32));
        assert!(
            escapes,
            "vertex {vertex} pushed along its own normal is still inside at every \
             distance to {:.1} mm",
            step * 10.0 * 1000.0
        );
    }
}

#[test]
fn every_attached_part_owns_a_region_of_the_atlas() {
    // A part with no region can only be flat-shaded, and the parts are most of
    // what a face is judged on. Each must land somewhere in the atlas, somewhere
    // of its own, and somewhere the painter actually covered.
    let avatar = Avatar::build(&AvatarRecord::default()).expect("builds");
    let parts = &avatar.parts;
    let features = parts.features.as_ref().expect("a biped has a face");

    let mut regions: Vec<(String, symbios_avatar::Vec2, symbios_avatar::Vec2)> = Vec::new();
    for (index, mesh) in features.meshes().enumerate() {
        regions.push((format!("feature {index}"), uv_min(mesh), uv_max(mesh)));
    }
    for part in parts.extremities.all() {
        regions.push((
            format!("{:?}", part.limb),
            uv_min(&part.mesh),
            uv_max(&part.mesh),
        ));
    }
    assert!(regions.len() >= 6, "two ears and four extremities at least");

    for (name, lo, hi) in &regions {
        assert!(
            (0.0..=1.0).contains(&lo.x) && (0.0..=1.0).contains(&hi.y),
            "{name} is charted outside the atlas: {lo:?}..{hi:?}"
        );
        assert!(
            hi.x - lo.x > 1e-4 && hi.y - lo.y > 1e-4,
            "{name} was given a region with no area — the degenerate chart is back"
        );
    }

    // No two parts share texels, and none overlaps a body chart.
    for (index, (name, lo, hi)) in regions.iter().enumerate() {
        for (other, olo, ohi) in &regions[index + 1..] {
            assert!(
                lo.x >= ohi.x || olo.x >= hi.x || lo.y >= ohi.y || olo.y >= hi.y,
                "{name} and {other} overlap in the atlas"
            );
        }
        for chart in &parts.unwrap.charts {
            let (clo, chi) = (chart.rect.min, chart.rect.max);
            assert!(
                lo.x >= chi.x || clo.x >= hi.x || lo.y >= chi.y || clo.y >= hi.y,
                "{name} overlaps the {:?} chart",
                chart.zone
            );
        }
    }
}

#[test]
fn the_painter_covers_the_parts_as_well_as_the_body() {
    // Reserving a region is only half of it: if nothing rasterises the part into
    // the atlas, its texels stay empty and it draws as background.
    use symbios_avatar::texture;

    let avatar = Avatar::build_with(&AvatarRecord::default(), &geometry_only()).expect("builds");
    let parts = &avatar.parts;

    // Everything attached, moved into body space the way the build does it.
    let mut placed: Vec<(PolyMesh, symbios_avatar::Zone)> = Vec::new();
    if let Some(features) = &parts.features {
        let head = avatar.rig.joints[features.head].position;
        for mesh in features.meshes() {
            placed.push((translated(mesh, head), symbios_avatar::Zone::Head));
        }
    }
    for part in parts.extremities.all() {
        let at = avatar.rig.joints[part.joint].position;
        placed.push((
            translated(&part.mesh, at),
            symbios_avatar::Zone::Extremity(part.limb),
        ));
    }
    let borrowed: Vec<(&PolyMesh, symbios_avatar::Zone)> =
        placed.iter().map(|(mesh, zone)| (mesh, *zone)).collect();

    // The same charts either way, so the only difference is the parts.
    let bare = texture::bake(&parts.body, &parts.unwrap, &[], 256);
    let whole = texture::bake(&parts.body, &parts.unwrap, &borrowed, 256);
    assert!(
        whole.covered() > bare.covered(),
        "baking {} parts covered no new texels: {} against {}",
        borrowed.len(),
        whole.covered(),
        bare.covered()
    );

    // And what they covered is the body, not somewhere off it: a part's texels
    // must sample a position inside the body's own bounds.
    let (lo, hi) = parts.body.bounds();
    let slack = (hi - lo).max_element() * 0.25;
    for y in 0..whole.height {
        for x in 0..whole.width {
            let (Some(texel), None) = (whole.get(x, y), bare.get(x, y)) else {
                continue;
            };
            assert!(
                texel.position.cmpge(lo - slack).all() && texel.position.cmple(hi + slack).all(),
                "a part texel samples {:?}, outside the body",
                texel.position
            );
        }
    }
}

/// Lowest atlas coordinate a mesh uses.
fn uv_min(mesh: &PolyMesh) -> symbios_avatar::Vec2 {
    mesh.uvs
        .iter()
        .fold(symbios_avatar::Vec2::splat(f32::MAX), |a, b| a.min(*b))
}

/// Highest atlas coordinate a mesh uses.
fn uv_max(mesh: &PolyMesh) -> symbios_avatar::Vec2 {
    mesh.uvs
        .iter()
        .fold(symbios_avatar::Vec2::splat(f32::MIN), |a, b| a.max(*b))
}

/// A copy of `mesh` moved by `offset`.
///
/// Written out rather than reached for through `glam`, which is not a
/// dependency of the tests.
fn translated(mesh: &PolyMesh, offset: Vec3) -> PolyMesh {
    let mut moved = mesh.clone();
    for point in &mut moved.positions {
        *point += offset;
    }
    moved
}
