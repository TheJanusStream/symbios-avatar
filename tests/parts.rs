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

use symbios_avatar::face::Skull;
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, PolyMesh, QuadrupedParams, Vec3, Zone,
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
///
/// **Measured as AREA, and it used to be measured as a share of vertices.** The
/// two agree only while a mesh's vertices are spread evenly over it, which the
/// globe's were until #81 placed six of its twelve rings inside the first 30° so
/// the iris could have edges. Nothing about the eye's placement changed and this
/// number went from 14% to 51%, which is a test reporting the vertex budget it
/// was handed rather than the surface a viewer sees. Area is what the sixth
/// above was always about.
const EYE_SHOWS: (f32, f32) = (0.03, 0.25);

/// How far an eye may stand proud of the face around it, in metres.
///
/// Measured on the shipped build: 30.4 mm on the default and 24.2–29.8 across
/// seeds, which is 0.18–0.26 of a head radius on every one of them — systematic,
/// not a tuning slip. In life the corneal apex sits roughly level with the
/// surrounding lids, so anything past a few millimetres is a bulge.
const EYE_PROUD: f32 = 0.005;

/// What share of `part`'s vertices lie outside `body`.
///
/// Still vertices, and only for the parts whose vertices are spread evenly over
/// them — an ear, a hand, a foot. See [`EYE_SHOWS`] for why the globe needs
/// [`exposed`] instead.
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

/// Hands every triangle of `part` to `visit` with its centroid, its area, and
/// whether that centroid lies outside `body`.
///
/// One walk, because two tests need the same one split different ways: how much
/// of the eye shows, and what colour the part that shows is.
fn triangles(
    body: &PolyMesh,
    part: &PolyMesh,
    offset: Vec3,
    mut visit: impl FnMut(Vec3, f64, bool),
) {
    for face in &part.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let (a, b, c) = (
                part.positions[face[0] as usize],
                part.positions[face[corner] as usize],
                part.positions[face[corner + 1] as usize],
            );
            let at = (a + b + c) / 3.0;
            let area = f64::from((b - a).cross(c - a).length() * 0.5);
            visit(at, area, !body.contains(at + offset));
        }
    }
}

/// What share of `part`'s surface area lies outside `body`.
fn exposed(body: &PolyMesh, part: &PolyMesh, offset: Vec3) -> f32 {
    let (mut outside, mut whole) = (0.0f64, 0.0f64);
    triangles(body, part, offset, |_, area, out| {
        whole += area;
        if out {
            outside += area;
        }
    });
    if whole <= 0.0 {
        return 1.0;
    }
    (outside / whole) as f32
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
            let shows = exposed(body, &eye.globe, centre);
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
fn an_eye_shows_more_white_than_pupil() {
    // The other half of "the eyes look popped out" (#73), and it outlived the
    // placement fix: seated correctly, the eye still read as a black bead,
    // because `iris_of` thresholded at 38.7° and the whole visible cap is only
    // 40.5°. Measured before this was fixed, the near-black pupil covered 91.7%
    // of the globe's exposed surface and no sclera was drawn at all.
    //
    // Measured as AREA, not as a vertex count: the mesh's vertices cluster where
    // its rings are, which is exactly where this change put them, so counting
    // vertices would measure the fix rather than the face. Classified by asking
    // `iris_of` itself for the two colours it uses at the poles, so the test
    // carries no copy of an angle that the geometry could drift away from.
    let pupil = symbios_avatar::face::eye::iris_of(Vec3::new(0.0, 0.0, 1.0));
    let sclera = symbios_avatar::face::eye::iris_of(Vec3::new(0.0, 0.0, -1.0));
    assert_ne!(pupil, sclera, "the test's premise");

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
            let mut share = [0.0f64; 3];
            triangles(body, &eye.globe, centre, |at, area, outside| {
                if !outside {
                    return;
                }
                let colour = symbios_avatar::face::eye::iris_of(at - eye.pivot);
                let class = if colour == pupil {
                    0
                } else if colour == sclera {
                    2
                } else {
                    1
                };
                share[class] += area;
            });
            let total: f64 = share.iter().sum();
            assert!(
                total > 0.0,
                "seed {seed}: the {side} eye shows nothing at all"
            );
            let pct = |at: usize| share[at] / total * 100.0;
            // Three bounds, because they fail independently. A pupil that eats
            // the eye is the defect; no sclera at all is the same defect; and an
            // iris that has shrunk to nothing is that defect mirrored, which a
            // ceiling on its own could not see.
            assert!(
                pct(0) < 10.0,
                "seed {seed}: the {side} eye is {:.0}% pupil, against 92% before this \
                 was fixed and about 4% of a circular aperture",
                pct(0)
            );
            assert!(
                pct(2) > 25.0,
                "seed {seed}: the {side} eye shows {:.0}% sclera; a face with no white \
                 in its eyes reads as a bead",
                pct(2)
            );
            assert!(
                pct(1) > 8.0,
                "seed {seed}: only {:.0}% of the {side} eye is iris — it has shrunk \
                 past the point of being visible",
                pct(1)
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
    // Two ears and two hands. Feet stopped being attached parts in #111 — they
    // are meshed with the body now — so they are charted as body surface and are
    // covered by whatever tests the body's own unwrap, not here.
    assert!(regions.len() >= 4, "two ears and two hands at least");

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

#[test]
fn an_eye_shows_white_on_both_sides_of_its_iris() {
    // #91. `an_eye_shows_more_white_than_pupil` above already demands 25% sclera
    // and passed throughout, because it measures the TOTAL and every one of
    // those percent was on the outer side: the skin beside the nose stood in
    // front of the globe, the iris met it directly, and the eye read as looking
    // sideways. A total that cannot see which side its white is on is not a
    // check on this — the failing case and the passing case give the same
    // number, which is why this is a second test rather than a tighter bound on
    // that one.
    //
    // Classified by asking `iris_of` for its own pole colours, as that test
    // does, so no angle is copied here for the geometry to drift away from.
    let sclera = symbios_avatar::face::eye::iris_of(Vec3::new(0.0, 0.0, -1.0));

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
            let (mut medial, mut lateral) = (0.0f64, 0.0f64);
            triangles(body, &eye.globe, centre, |at, area, outside| {
                if !outside || symbios_avatar::face::eye::iris_of(at - eye.pivot) != sclera {
                    return;
                }
                // `eye.side` is +1 for the body's right eye, so this is positive
                // away from the midline whichever eye it is.
                if (at.x - eye.pivot.x) * eye.side > 0.0 {
                    lateral += area;
                } else {
                    medial += area;
                }
            });
            let total = medial + lateral;
            assert!(total > 0.0, "seed {seed}: the {side} eye shows no white");
            let share = medial / total * 100.0;
            // **Two percent, and the bar is low on purpose rather than by
            // accident.** It is not asking for parity: the outer white is
            // genuinely the larger, because the lid's margin closes the aperture
            // at 59° laterally while the medial side ends in a corner. Nor is it
            // asking for as much as the geometry can give, because it cannot
            // give much — the medial aperture here is bounded by SKIN rather
            // than by the lid, and how far the socket may be deepened is capped
            // by `an_eye_is_seated_in_the_face_rather_than_resting_on_it` above,
            // which fails at 26% globe exposure one step past where the orbit
            // now sits. Measured across these sixteen seeds the nasal share runs
            // 3.9% to 31.2%, and it ran 0.0% on all but one of them before (#91).
            //
            // So this catches a return to zero, which is what the defect was,
            // and it is set two points under the worst seed because the measure
            // itself is triangulation-noisy: a symmetric body reports 3.9% for
            // one eye and 5.7% for the other.
            assert!(
                share > 2.0,
                "seed {seed}: only {share:.1}% of the {side} eye's white is on the \
                 nasal side, so the iris runs into the skin there and the eye reads \
                 as turned outward"
            );
        }
    }
}

#[test]
fn the_underside_of_the_jaw_does_not_bulge() {
    // #94. From the side, the run from the chin's tip back to the throat read as
    // one convex arc — a soft double chin where life has a straight-to-hollow
    // line. Measured as the forward deviation from the CHORD joining those two
    // points, which is the whole requirement in one number: positive is a bulge.
    //
    // **The bound is the state, not the target**, and the target is near zero.
    // 10.5 is a hair above the worst of these sixteen seeds, which is 10.0. Its
    // job is to stop the defect deepening and to give whoever finds the cause a
    // number to drive down; tightening it IS the fix.
    //
    // **Sixteen seeds, because four were not enough and that is the finding.**
    // #94's analysis ran on the default body plus three seeds and read 9.2 mm
    // falling to 7.0 after the `CHIN` knot came onto its chord. Over the full
    // sweep the worst is 10.0 mm AFTER that fix — the four-seed sample missed
    // the tail of the population entirely, so the "2.2 mm recovered" figure
    // describes those four bodies and not this one.
    //
    // What the residual is not, all measured: the cage is straight there (-0.1
    // to -0.7 mm before any profile runs), the relief face carve contributes
    // exactly nothing (identical to the last decimal with it and without), and
    // deleting every below-joint knot of `CHIN` still leaves +3.4 mm.
    for seed in 0..SEEDS {
        let mut record = AvatarRecord::new("Jaw", Archetype::default());
        record.reroll(seed);
        let avatar = Avatar::build_with(&record, &geometry_only()).expect("the body builds");
        let body = &avatar.parts.body;
        let Some(head) = avatar.rig.in_zone(Zone::Head).first().copied() else {
            continue;
        };
        let at = avatar.rig.joints[head].position;
        let Some(skull) = Skull::measure(body, &avatar.rig) else {
            continue;
        };

        // How far the midline surface reaches forward at a height, bisected —
        // never binned, for the reason the head audit gives.
        let reach = |y: f32| -> Option<f32> {
            if !body.contains(Vec3::new(at.x, y, at.z)) {
                return None;
            }
            let (mut inside, mut outside) = (at.z, at.z + 0.40);
            for _ in 0..32 {
                let mid = 0.5 * (inside + outside);
                if body.contains(Vec3::new(at.x, y, mid)) {
                    inside = mid;
                } else {
                    outside = mid;
                }
            }
            Some(inside)
        };

        let chin_y = at.y + skull.chin();
        let throat_y = at.y + skull.throat_and_crown().0;
        let (Some(chin_z), Some(throat_z)) = (reach(chin_y), reach(throat_y)) else {
            continue;
        };

        let mut worst = 0.0f32;
        let mut worst_at = 0.0f32;
        for step in 1..20 {
            let t = step as f32 / 20.0;
            let y = chin_y + (throat_y - chin_y) * t;
            if let Some(z) = reach(y) {
                let out = (z - (chin_z + (throat_z - chin_z) * t)) * 1000.0;
                if out > worst {
                    worst = out;
                    worst_at = (y - at.y) * 1000.0;
                }
            }
        }
        assert!(
            worst < 10.5,
            "seed {seed}: the underside of the jaw stands {worst:.1} mm forward of \
             the chord from the chin to the throat, at {worst_at:.1} mm. A \
             jawline should be straight to hollow, so the target is near zero."
        );
    }
}
