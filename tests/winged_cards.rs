//! Winged bodies' cards, marks and membrane (#423): a bird-led body is
//! feathered and its face's dish, its beak and its feet are bare, and no card
//! crosses its folded wing; a bat and a glider grow fur and none on their
//! membrane, at rest or spread; no card lies on an eye; a fruit bat wears its mantle and
//! a sugar glider its stripes; a bat's membrane is paler toward its free edge
//! than in its middle; and every winged body, cards and all, is under its
//! budget.

use symbios_avatar::hair::fur::FurZone;
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, BeastHead, MeshKind, PolyMesh, Pose, Vec2, Vec3,
    WingedFamily, WingedParams, Zone, texture,
};

/// Every family, and a cross of each pair, either way round, at both sides
/// of the halfway blend and near it.
fn lineages() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut all: Vec<(String, Lineage<WingedFamily>)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), Lineage::pure(family)))
        .collect();
    for (a, b) in [
        (Bird, Bat),
        (Bat, Bird),
        (Bat, Glider),
        (Glider, Bat),
        (Glider, Bird),
        (Bird, Glider),
    ] {
        for blend in [0.3, 0.45, 0.55, 0.7] {
            all.push((
                format!("{a:?} x {b:?} {blend}"),
                Lineage::cross(a, b, blend),
            ));
        }
    }
    all
}

fn params(lineage: Lineage<WingedFamily>) -> WingedParams {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    params
}

fn record(params: WingedParams) -> AvatarRecord {
    AvatarRecord::new("Winged", Archetype::Winged(params))
}

/// Rolled winged bodies, by seed.
fn rolls(count: i64) -> impl Iterator<Item = (String, AvatarRecord)> {
    (1..=count).map(|seed| {
        let mut record = record(WingedParams::default());
        record.reroll(seed);
        (format!("seed {seed}"), record)
    })
}

/// Rolled winged bodies, by the seeds given.
fn seeded(seeds: &[i64]) -> impl Iterator<Item = (String, AvatarRecord)> + '_ {
    seeds.iter().map(|&seed| {
        let mut record = record(WingedParams::default());
        record.reroll(seed);
        (format!("seed {seed}"), record)
    })
}

fn leading(record: &AvatarRecord) -> WingedFamily {
    match &record.archetype {
        Archetype::Winged(params) => params.lineage().leading(),
        _ => unreachable!("a winged record"),
    }
}

/// The body's cards: a bird's contour feathers or a bat's or a glider's fur.
fn cards(avatar: &Avatar) -> Option<&symbios_avatar::hair::fur::FurGrowth> {
    avatar.parts.fur.as_ref()
}

/// A bird's contour cards, nine vertices each, and where each roots: the
/// middle of its quill's row.
fn feathers(avatar: &Avatar) -> Vec<(Vec3, &[Vec3])> {
    let fur = cards(avatar).expect("a bird-led body is feathered");
    fur.mesh
        .positions
        .chunks(9)
        .map(|card| ((card[0] + card[1]) * 0.5, card))
        .collect()
}

/// `points`, riding the head joint the eyes ride, in the head's frame in its
/// radii.
fn in_head(avatar: &Avatar, head: &BeastHead, points: &[Vec3], body: bool) -> Vec<Vec3> {
    let eyes = avatar.parts.eyes.as_ref().expect("a winged body has eyes");
    let joint = if body {
        Vec3::ZERO
    } else {
        avatar.rig.joints[eyes.head].position
    };
    points
        .iter()
        .map(|&point| head.local(joint + point) / head.radius)
        .collect()
}

/// **Every bird-led body is feathered**, and its face's dish, its beak and its
/// feet are bare: an owl's dish, which a full disc carves; nowhere near the
/// mandibles; and no card bound to a foot. Of a crow, a parrot and a songbird
/// too, and of crosses either side of the halfway blend.
#[test]
fn a_bird_is_feathered_and_its_dish_its_beak_and_its_feet_are_bare() {
    use WingedFamily::{Bat, Bird, Glider};
    let heads: [(&str, [f32; 3]); 4] = [
        ("owl", [0.0, 0.0, 0.0]),
        ("crow", [1.0, -1.0, -1.0]),
        ("parrot", [0.3, 1.0, -1.0]),
        ("songbird", [0.2, -1.3, -1.0]),
    ];
    let mut bodies = Vec::new();
    for (name, lineage) in [
        ("bird", Lineage::pure(Bird)),
        ("bird x bat 0.3", Lineage::cross(Bird, Bat, 0.3)),
        ("glider x bird 0.7", Lineage::cross(Glider, Bird, 0.7)),
    ] {
        for (kind, head) in heads {
            let mut params = params(lineage);
            [params.beak_length, params.beak_hook, params.facial_disc] = head;
            bodies.push((format!("{name}, {kind}"), record(params), kind == "owl"));
        }
    }
    // A bird-bat cross at the naturalistic end, whose folded wing lay over
    // the most cards.
    let mut naturalistic = params(Lineage::cross(Bird, Bat, 0.3));
    naturalistic.appeal = -1.0;
    bodies.push((
        "bird x bat 0.3, appeal -1".to_string(),
        record(naturalistic),
        false,
    ));
    // An owl's disc at either end of the young's axis, and the disc weaker.
    for (kind, appeal, disc) in [
        ("naturalistic owl", -1.0, 0.0),
        ("owlet", 1.5, 0.0),
        ("half-disc owl", 0.0, -0.5),
    ] {
        let mut params = params(Lineage::pure(Bird));
        params.appeal = appeal;
        params.facial_disc = disc;
        bodies.push((kind.to_string(), record(params), disc == 0.0));
    }
    bodies.extend(
        rolls(24)
            .filter(|(_, record)| leading(record) == Bird)
            .map(|(name, record)| (name, record, false)),
    );
    let mut owls = 0;
    for (name, record, owl) in bodies {
        let head = record.skeleton().beast_head.expect("a beast's head");
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let fur = cards(&avatar).unwrap_or_else(|| panic!("{name}: not feathered"));
        let contour = fur
            .grown
            .iter()
            .find(|grown| grown.zone == FurZone::Contour)
            .unwrap_or_else(|| panic!("{name}: no contour feathers"));
        assert!(
            contour.clumps >= 500,
            "{name}: {} contour feathers",
            contour.clumps
        );
        let cards = feathers(&avatar);
        // The dish: the face's front, within the rim's circle - 0.8 radii
        // across round a middle 0.12 under the centre - drawn in by a quarter.
        if owl {
            owls += 1;
            let roots: Vec<Vec3> = cards.iter().map(|(root, _)| *root).collect();
            let in_dish = in_head(&avatar, &head, &roots, true)
                .into_iter()
                .filter(|local| local.z > 0.3 && local.x.hypot(local.y + 0.12) < 0.3)
                .count();
            assert_eq!(in_dish, 0, "{name}: {in_dish} feathers root in the dish");
        }
        // The beak: no card's vertex near a mandible's.
        let worn = avatar.parts.adornments.as_ref().expect("a beak");
        let beak: Vec<Vec3> = [&worn.on_head, &worn.on_jaw]
            .into_iter()
            .flat_map(|mesh| in_head(&avatar, &head, &mesh.positions, false))
            .collect();
        let near_beak = cards
            .iter()
            .flat_map(|(_, card)| in_head(&avatar, &head, card, true))
            .map(|local| {
                beak.iter()
                    .map(|point| point.distance(local))
                    .fold(f32::MAX, f32::min)
            })
            .fold(f32::MAX, f32::min);
        assert!(
            near_beak > BEAK_GAP,
            "{name}: a feather {near_beak:.3} head radii from the beak"
        );
        // The folded wing: no edge of a card passes through a feather as
        // the wing lies at rest, folded.
        let folded = &avatar.parts.wings.as_ref().expect("wings").feathers;
        let through = crossings(&fur.mesh, folded);
        assert_eq!(
            through, 0,
            "{name}: {through} card edges cross the folded wing"
        );
        // The feet: no card bound to a foot.
        for (vertex, skin) in fur.mesh.skin.iter().enumerate() {
            let most = skin
                .iter()
                .max_by(|a, b| a.weight.total_cmp(&b.weight))
                .expect("held");
            assert!(
                !matches!(
                    avatar.rig.joints[most.joint as usize].zone,
                    Zone::Extremity(_)
                ),
                "{name}: card vertex {vertex} grows on a foot"
            );
        }
    }
    assert!(owls >= 3, "the premise: owls were measured");
}

/// How near a mandible, in head radii, a contour feather may come: about half
/// a centimetre on the default owl, a feather at the beak's edge and not on it.
const BEAK_GAP: f32 = 0.05;

/// **A bat and a glider grow fur, and none on their membrane**: a bat-led
/// body its ruff, a glider-led one its tail's brush, and no edge of any card
/// passing through a membrane at rest, half spread or spread, the fur moved
/// by its own skin as the pose moves the arm under it.
#[test]
fn a_bat_and_a_glider_grow_fur_and_none_on_the_membrane() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies: Vec<(String, AvatarRecord)> = lineages()
        .into_iter()
        .filter(|(_, lineage)| lineage.leading() != Bird)
        .map(|(name, lineage)| (name, record(params(lineage))))
        .collect();
    bodies.push((
        "Bat x Glider 0.51".to_string(),
        record(params(Lineage::cross(Bat, Glider, 0.51))),
    ));
    bodies.extend(rolls(16).filter(|(_, record)| leading(record) != Bird));
    // Rolls whose ruff rode the arm through the propatagium.
    bodies.extend(seeded(&[41, 94]).filter(|(_, record)| leading(record) != Bird));
    let mut seen = [false; 2];
    for (name, record) in bodies {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let fur = cards(&avatar).unwrap_or_else(|| panic!("{name}: grows no fur"));
        assert!(fur.tris() > 0, "{name}: grows no fur");
        assert!(
            fur.grown.iter().all(|grown| grown.zone != FurZone::Contour),
            "{name}: a mammal grows feathers"
        );
        let led = leading(&record);
        let zone = if led == Bat {
            FurZone::Ruff
        } else {
            FurZone::Tail
        };
        if record.archetype == Archetype::Winged(params(Lineage::pure(led))) {
            seen[usize::from(led == Glider)] = true;
            assert!(
                fur.grown.iter().any(|grown| grown.zone == zone),
                "{name}: no {}",
                zone.name()
            );
        }
        let membrane = &avatar.parts.wings.as_ref().expect("wings").membrane;
        for extension in [0.0, 0.5, 1.0] {
            let mut pose = Pose::rest(&avatar.rig);
            avatar.rig.spread_wings(&mut pose, extension, extension);
            let posed = pose.forward(&avatar.rig);
            let crossings = crossings(
                &posed.deform_mesh(&avatar.rig, &fur.mesh),
                &posed.deform_mesh(&avatar.rig, membrane),
            );
            assert!(
                crossings == 0,
                "{name}: {crossings} edges of fur pass through the membrane spread {extension}"
            );
        }
    }
    assert_eq!(seen, [true, true], "the premise: a bat and a glider");
}

/// How many of `cards`' edges pass through one of `membrane`'s triangles, as
/// both lie at rest: the triangles binned on a grid, each edge tested against
/// those in the cells its box covers.
fn crossings(cards: &PolyMesh, membrane: &PolyMesh) -> usize {
    use std::collections::{HashMap, HashSet};
    let triangles: Vec<[Vec3; 3]> = membrane
        .triangulated()
        .iter()
        .map(|t| t.map(|v| membrane.positions[v as usize]))
        .collect();
    let (low, high) = membrane.bounds();
    let cell = (high - low).max_element() / 48.0;
    let bin = |at: Vec3| {
        let at = (at / cell).floor();
        (at.x as i32, at.y as i32, at.z as i32)
    };
    let mut bins: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
    for (index, triangle) in triangles.iter().enumerate() {
        let (lo, hi) = (
            bin(triangle[0].min(triangle[1]).min(triangle[2])),
            bin(triangle[0].max(triangle[1]).max(triangle[2])),
        );
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                for z in lo.2..=hi.2 {
                    bins.entry((x, y, z)).or_default().push(index);
                }
            }
        }
    }
    let mut edges = HashSet::new();
    for face in &cards.faces {
        for (index, &a) in face.iter().enumerate() {
            let b = face[(index + 1) % face.len()];
            edges.insert((a.min(b), a.max(b)));
        }
    }
    edges
        .into_iter()
        .filter(|&(a, b)| {
            let (from, to) = (cards.positions[a as usize], cards.positions[b as usize]);
            let (lo, hi) = (bin(from.min(to)), bin(from.max(to)));
            let mut seen = HashSet::new();
            for x in lo.0..=hi.0 {
                for y in lo.1..=hi.1 {
                    for z in lo.2..=hi.2 {
                        for &triangle in bins.get(&(x, y, z)).into_iter().flatten() {
                            if seen.insert(triangle) && crosses(from, to, &triangles[triangle]) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        })
        .count()
}

/// Whether the segment `from`-`to` passes through `triangle`.
fn crosses(from: Vec3, to: Vec3, [a, b, c]: &[Vec3; 3]) -> bool {
    let run = to - from;
    let (ab, ac) = (*b - *a, *c - *a);
    let p = run.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return false;
    }
    let off = from - *a;
    let u = off.dot(p) / det;
    let q = off.cross(ab);
    let v = run.dot(q) / det;
    let t = ac.dot(q) / det;
    (0.0..=1.0).contains(&u) && v >= 0.0 && u + v <= 1.0 && (0.0..=1.0).contains(&t)
}

/// **No card lies on an eye**: no vertex of a winged body's cards inside its
/// eye's globe or within a tenth of its radius of it (about three millimetres
/// on a bat's), on every family and cross, the young end of a glider-bat
/// cross, and rolls - those whose ruff lay across the eye among them.
#[test]
fn no_card_lies_on_an_eye() {
    use WingedFamily::{Bat, Glider};
    let mut bodies: Vec<(String, AvatarRecord)> = lineages()
        .into_iter()
        .map(|(name, lineage)| (name, record(params(lineage))))
        .collect();
    for (a, b, blend) in [(Glider, Bat, 0.55), (Glider, Bat, 0.7), (Bat, Glider, 0.3)] {
        let mut young = params(Lineage::cross(a, b, blend));
        young.appeal = 1.5;
        bodies.push((format!("{a:?} x {b:?} {blend}, appeal 1.5"), record(young)));
    }
    bodies.extend(rolls(12));
    bodies.extend(seeded(&[32, 71, 94]));
    let mut bats = 0;
    for (name, record) in bodies {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let fur = cards(&avatar).unwrap_or_else(|| panic!("{name}: grows no cards"));
        let eyes = avatar.parts.eyes.as_ref().expect("a winged body has eyes");
        let head = avatar.rig.joints[eyes.head].position;
        bats += usize::from(leading(&record) == Bat);
        for eye in [&eyes.left, &eyes.right] {
            // The ball about the pivot the globe turns on.
            let centre = head + eye.pivot;
            let radius = eye
                .globe
                .positions
                .iter()
                .map(|point| point.distance(eye.pivot))
                .fold(0.0, f32::max);
            let nearest = fur
                .mesh
                .positions
                .iter()
                .map(|at| at.distance(centre) - radius)
                .fold(f32::MAX, f32::min);
            assert!(
                nearest > EYE_GAP * radius,
                "{name}: a card {:.1} mm off a globe of {:.1} mm radius",
                nearest * 1000.0,
                radius * 1000.0
            );
        }
    }
    assert!(bats >= 6, "the premise: bats were measured");
}

/// How near an eye's globe a card may come, in its radii.
const EYE_GAP: f32 = 0.1;

fn withers(avatar: &Avatar) -> Vec3 {
    let rig = &avatar.rig;
    let neck = rig.in_zone(Zone::Neck)[0];
    rig.joints[rig.joints[neck].parent.expect("a girdle")].position
}

fn hips(avatar: &Avatar) -> Vec3 {
    avatar.rig.joints[avatar.rig.in_zone(Zone::Pelvis)[0]].position
}

/// A body painted on a small atlas.
fn painted(record: &AvatarRecord) -> Avatar {
    Avatar::build_with(
        record,
        &AvatarConfig {
            atlas: ATLAS,
            ..AvatarConfig::default()
        },
    )
    .expect("builds")
}

const ATLAS: u32 = 256;

/// The colour at texel `index`, `0..=1`.
fn colour(avatar: &Avatar, index: usize) -> Vec3 {
    let at = index * 4;
    let rgba = &avatar.skin.albedo[at..at + 3];
    Vec3::new(rgba[0].into(), rgba[1].into(), rgba[2].into()) / 255.0
}

fn mean(colours: &[Vec3]) -> Vec3 {
    colours.iter().copied().sum::<Vec3>() / colours.len().max(1) as f32
}

fn luminance(colour: Vec3) -> f32 {
    colour.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// The body's own texels, where on it each is and its colour.
///
/// **Only those its charts cover**, not the gutters dilation grows round
/// them (#424): baked without the parts the avatar baked beside it, the
/// body's gutters grow over texels a part's region holds in the atlas - a
/// bat's toes, packed beside its rump once its foot trails, read here as
/// 150 pale texels on its back. A grown texel copies its neighbour's
/// position, so a position held by more than one texel is a gutter's.
fn body_texels(avatar: &Avatar) -> Vec<(texture::Texel, Vec3)> {
    let parts = &avatar.parts;
    let geometry = texture::bake(&parts.body, &parts.unwrap, &[], &[], ATLAS);
    let key = |texel: &texture::Texel| texel.position.to_array().map(f32::to_bits);
    let mut held = std::collections::HashMap::new();
    for texel in geometry.texels.iter().flatten() {
        *held.entry(key(texel)).or_insert(0usize) += 1;
    }
    geometry
        .texels
        .iter()
        .enumerate()
        .filter_map(|(index, texel)| texel.map(|texel| (texel, colour(avatar, index))))
        .filter(|(texel, _)| held[&key(texel)] == 1)
        .collect()
}

/// **A fruit bat wears its mantle and a sugar glider its stripes**: a bat's
/// shoulders paler and warmer than the middle of its back, a glider's spine
/// darker than its flank beside it, and the line behind its eye darker than its
/// cheek under it.
#[test]
fn a_bat_wears_its_mantle_and_a_glider_its_stripes() {
    use WingedFamily::{Bat, Glider};
    // The bat.
    let bat = painted(&record(params(Lineage::pure(Bat))));
    let (hips, withers) = (hips(&bat), withers(&bat));
    let spine = withers - hips;
    let along = |at: Vec3| (at - hips).dot(spine) / spine.length_squared();
    let texels = body_texels(&bat);
    let group = |from: f32, to: f32| -> Vec<Vec3> {
        texels
            .iter()
            .filter(|(texel, _)| {
                texel.zone != Zone::Head
                    && texel.normal.y > 0.5
                    && (from..to).contains(&along(texel.position))
            })
            .map(|(_, colour)| *colour)
            .collect()
    };
    let (shoulders, back) = (mean(&group(0.85, 1.15)), mean(&group(0.15, 0.45)));
    let warmth = |colour: Vec3| colour.x - colour.z;
    assert!(
        luminance(shoulders) > luminance(back) * 1.6 && warmth(shoulders) > warmth(back) + 0.1,
        "a bat's mantle {shoulders} against its back {back}"
    );

    // The glider.
    let glider = painted(&record(params(Lineage::pure(Glider))));
    let (hips, withers) = (self::hips(&glider), self::withers(&glider));
    let spine = withers - hips;
    let height = withers.y;
    let along = |at: Vec3| (at - hips).dot(spine) / spine.length_squared();
    let texels = body_texels(&glider);
    let back = |texel: &texture::Texel| {
        texel.zone != Zone::Head && (0.4..0.9).contains(&along(texel.position))
    };
    let middle: Vec<Vec3> = texels
        .iter()
        .filter(|(texel, _)| {
            back(texel) && texel.position.x.abs() < 0.015 * height && texel.normal.y > 0.7
        })
        .map(|(_, colour)| *colour)
        .collect();
    let flank: Vec<Vec3> = texels
        .iter()
        .filter(|(texel, _)| {
            back(texel)
                && (0.12..0.3).contains(&(texel.position.x.abs() / height))
                && texel.normal.y > 0.5
        })
        .map(|(_, colour)| *colour)
        .collect();
    assert!(!middle.is_empty() && !flank.is_empty(), "the premise");
    let (middle, flank) = (mean(&middle), mean(&flank));
    assert!(
        luminance(middle) < luminance(flank) * 0.7,
        "a glider's spine {middle} against its flank {flank}"
    );
    // Its face: behind the eye toward the ear, against the cheek under it.
    let record = record(params(Lineage::pure(Glider)));
    let head = record.skeleton().beast_head.expect("a head");
    let eyes = glider.parts.eyes.as_ref().expect("eyes");
    let eye = head.local(glider.rig.joints[eyes.head].position + eyes.left.pivot) / head.radius;
    let around = |keep: &dyn Fn(Vec3) -> bool| -> Vec3 {
        let picked: Vec<Vec3> = texels
            .iter()
            .filter(|(texel, _)| texel.zone == Zone::Head)
            .filter(|(texel, _)| {
                let local = head.local(texel.position) / head.radius;
                local.x > 0.0 && keep(local - eye)
            })
            .map(|(_, colour)| *colour)
            .collect();
        assert!(!picked.is_empty(), "the premise: texels round the eye");
        mean(&picked)
    };
    let behind = around(&|off| off.z < -0.15 && off.z > -0.4 && off.y.abs() < 0.12);
    // The cheek under the eye's dark ring, which reaches a globe's radius
    // round it: nearer, the "cheek" is the ring and darkens with the stripe.
    let below = around(&|off| off.y < -0.40 && off.y > -0.60 && off.z.abs() < 0.12);
    assert!(
        luminance(behind) < luminance(below) * 0.7,
        "behind a glider's eye {behind} against its cheek {below}"
    );
}

/// The membrane's colours at its vertices' texels, by how far out each lies:
/// those `0.95` of the way to the free edge and on, and those between `0.3`
/// and `0.5`, on the pieces whose rows run to the free edge.
fn membrane_colours(avatar: &Avatar, bat: bool) -> (Vec3, Vec3) {
    let wings = avatar.parts.wings.as_ref().expect("wings");
    let membrane = &wings.membrane;
    let (low, high) = membrane.uvs.iter().fold(
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
        |(low, high), uv| (low.min(*uv), high.max(*uv)),
    );
    let side = avatar.skin.width as f32;
    let (mut edge, mut middle) = (Vec::new(), Vec::new());
    for (uv, reach) in membrane.uvs.iter().zip(&wings.membrane_reach) {
        let local = (*uv - low) / (high - low);
        // A bat's digits' corner, and its propatagium, whose free edge runs
        // across its column.
        if bat && (local.x > 0.64 || (local.x < 0.06 && local.y < 0.06)) {
            continue;
        }
        let (x, y) = (
            ((uv.x * side) as usize).min(avatar.skin.width as usize - 1),
            ((uv.y * side) as usize).min(avatar.skin.height as usize - 1),
        );
        let colour = colour(avatar, y * avatar.skin.width as usize + x);
        if *reach >= 0.95 {
            edge.push(colour);
        } else if (0.3..=0.5).contains(reach) {
            middle.push(colour);
        }
    }
    assert!(!edge.is_empty() && !middle.is_empty(), "the premise");
    (mean(&edge), mean(&middle))
}

/// **A membrane is paler toward its free edge than in its middle**, as if lit
/// through: a bat's naked membrane paler and warmer, a glider's furred
/// patagium paler.
#[test]
fn a_membrane_is_paler_at_its_free_edge_than_in_its_middle() {
    use WingedFamily::{Bat, Glider};
    for (name, lineage) in [
        ("bat", Lineage::pure(Bat)),
        ("bat-led cross", Lineage::cross(Glider, Bat, 0.7)),
    ] {
        let avatar = painted(&record(params(lineage)));
        let (edge, middle) = membrane_colours(&avatar, true);
        assert!(
            luminance(edge) > luminance(middle) * 1.5 && edge.x - edge.z > middle.x - middle.z,
            "{name}: edge {edge} against middle {middle}"
        );
        // And dark: a membrane, not a coat.
        assert!(luminance(middle) < 0.15, "{name}: middle {middle}");
    }
    let glider = painted(&record(params(Lineage::pure(Glider))));
    let (edge, middle) = membrane_colours(&glider, false);
    assert!(
        luminance(edge) > luminance(middle) * 1.1,
        "glider: edge {edge} against middle {middle}"
    );
}

/// **Every winged body is under its budget, its cards included**: every
/// family, each cross either way round at both sides of the halfway blend and
/// near it, and rolls; and every one grows its cards.
#[test]
fn every_winged_body_fits_its_budget_cards_and_all() {
    let mut worst = (String::new(), 0usize);
    let bodies = lineages()
        .into_iter()
        .map(|(name, lineage)| (name, record(params(lineage))))
        .chain(rolls(40));
    for (name, record) in bodies {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let fur = cards(&avatar).map_or(0, |fur| fur.tris());
        assert!(fur > 0, "{name}: grows no cards");
        assert!(
            avatar.budget.meshes <= 4,
            "{name}: {} draws",
            avatar.budget.meshes
        );
        let hair = avatar
            .meshes
            .iter()
            .filter(|drawn| drawn.kind == MeshKind::Hair)
            .count();
        assert_eq!(hair, 1, "{name}: its cards are not in one hair draw");
        assert!(
            avatar.budget.tris <= symbios_avatar::wing::MAX_TRIANGLES,
            "{name} costs {}",
            avatar.budget.tris
        );
        if avatar.budget.tris > worst.1 {
            worst = (name, avatar.budget.tris);
        }
    }
    println!("dearest winged body: {} at {}", worst.1, worst.0);
}

/// **A bat-led body wears its fur whole** (the owner's call at W4's review,
/// `wing::MAX_TRIANGLES` 27,200): under 26,000 the dearest, seed 2, a
/// bat-glider cross, kept 156 of the 1,332 fur triangles it asked for.
#[test]
fn a_bat_led_body_wears_its_fur_whole() {
    for (name, record) in seeded(&[2, 13]) {
        let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
        let fur = cards(&avatar).expect("a bat-led body grows fur");
        assert!(
            fur.tris() >= 1_000,
            "{name}: its fur is {} triangles",
            fur.tris()
        );
        assert!(avatar.budget.tris <= symbios_avatar::wing::MAX_TRIANGLES);
    }
}
