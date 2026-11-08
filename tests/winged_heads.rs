//! Winged bodies' heads (#422): each family's head has the parts it should and
//! no others, a bird's beak lengthens and hooks with its axes, an owl's disc
//! comes and goes with its own and turns the eyes with it, the lower mandible
//! opens with the jaw, a record and a share code from before the axes read as
//! an owl's head, and every family and cross builds clean at the axes' ends.

use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, BeastHead, MeshKind, PolyMesh, Pose, Vec3, WingedFamily,
    WingedParams,
};

/// A body of `lineage` with its head's axes `[beakLength, beakHook, facialDisc]`.
fn params(lineage: Lineage<WingedFamily>, head: [f32; 3]) -> WingedParams {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    [params.beak_length, params.beak_hook, params.facial_disc] = head;
    params
}

/// The body built, and the head its plan declared.
fn built(params: WingedParams) -> (Avatar, BeastHead) {
    let record = AvatarRecord::new("Winged", Archetype::Winged(params));
    let head = record
        .skeleton()
        .beast_head
        .expect("a winged body declares a beast's head");
    let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
    (avatar, head)
}

/// The owl, every axis at its own.
const OWL: [f32; 3] = [0.0, 0.0, 0.0];

/// A mesh riding the head joint, in the head's own frame.
fn in_head(avatar: &Avatar, head: &BeastHead, mesh: &PolyMesh) -> Vec<Vec3> {
    let eyes = avatar.parts.eyes.as_ref().expect("a winged body has eyes");
    let joint = avatar.rig.joints[eyes.head].position;
    mesh.positions
        .iter()
        .map(|&point| head.local(joint + point))
        .collect()
}

/// The upper mandible and the lower, in the head's frame: what rides the head
/// and what rides the jaw, a winged body wearing nothing else there.
fn mandibles(avatar: &Avatar, head: &BeastHead) -> (Vec<Vec3>, Vec<Vec3>) {
    let worn = avatar
        .parts
        .adornments
        .as_ref()
        .expect("a beaked head wears its beak");
    (
        in_head(avatar, head, &worn.on_head),
        in_head(avatar, head, &worn.on_jaw),
    )
}

fn lowest(points: &[Vec3]) -> f32 {
    points.iter().map(|point| point.y).fold(f32::MAX, f32::min)
}

fn furthest(points: &[Vec3]) -> f32 {
    points.iter().map(|point| point.z).fold(f32::MIN, f32::max)
}

#[test]
fn each_family_s_head_has_its_parts_and_no_others() {
    use WingedFamily::{Bat, Bird, Glider};
    for (name, lineage) in [
        ("bird", Lineage::pure(Bird)),
        ("bird-led cross", Lineage::cross(Bird, Glider, 0.3)),
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bat-led cross", Lineage::cross(Bird, Bat, 0.6)),
    ] {
        let (avatar, head) = built(params(lineage, OWL));
        let face = avatar.parts.beast.as_ref().expect("a beast's face");
        let worn = avatar.parts.adornments.clone().unwrap_or_default();
        let ears = face.ears.left.mesh.vertex_count() + face.ears.right.mesh.vertex_count();
        let leather = head.on_leather(head.nose_centre(), 0.0);
        if lineage.leading() == Bird {
            // A beak of two mandibles, no jaw of skin, no tongue, no fleshy nose,
            // no whiskers; a disc at an owl's axis.
            assert!(head.beaked(), "{name}: no beak");
            assert!(!worn.on_head.is_empty(), "{name}: no upper mandible");
            assert!(!worn.on_jaw.is_empty(), "{name}: no lower mandible");
            assert!(face.jaw.mesh.is_empty(), "{name}: a jaw of skin");
            assert!(face.jaw.tongue.is_empty(), "{name}: a tongue");
            assert!(!leather, "{name}: a leather nose");
            assert!(worn.whiskers.is_empty(), "{name}: whiskers");
            assert!(head.has_disc(), "{name}: no disc at an owl's axis");
            if lineage.hybrid.is_none() {
                assert_eq!(ears, 0, "{name}: ears to see");
                assert!(
                    face.ears.left.joint.is_none(),
                    "{name}: an ear's joint with no ear"
                );
            }
        } else {
            // A muzzle: a jaw and a tongue of its own, a leather nose, large ears,
            // no beak and no disc.
            assert!(!head.beaked(), "{name}: a beak");
            assert!(!head.has_disc(), "{name}: a disc");
            assert!(worn.on_head.is_empty() && worn.on_jaw.is_empty(), "{name}");
            assert!(!face.jaw.mesh.is_empty(), "{name}: no jaw");
            assert!(!face.jaw.tongue.is_empty(), "{name}: no tongue");
            assert!(leather, "{name}: no leather nose");
            assert!(ears > 0, "{name}: no ears");
        }
    }
}

/// How long an ear is, base to tip, on a body of `lineage`.
fn ear(lineage: Lineage<WingedFamily>) -> f32 {
    let (avatar, _) = built(params(lineage, OWL));
    let ear = &avatar
        .parts
        .beast
        .as_ref()
        .expect("a beast's face")
        .ears
        .left;
    ear.tip.distance(ear.base)
}

/// The globes' radius on a body of `lineage`.
fn eye(lineage: Lineage<WingedFamily>) -> f32 {
    let (avatar, head) = built(params(lineage, OWL));
    let eyes = avatar.parts.eyes.as_ref().expect("eyes");
    let (lo, hi) = eyes.left.globe.bounds();
    (hi - lo).max_element() * 0.5 / head.radius
}

#[test]
fn a_cross_blends_its_ears_and_eyes_through_the_switch() {
    use WingedFamily::{Bat, Bird, Glider};
    // None on a bird, growing toward a bat's as the cross leans to it.
    let ears: Vec<f32> = [0.3, 0.49, 0.51, 0.75]
        .into_iter()
        .map(|blend| ear(Lineage::cross(Bird, Bat, blend)))
        .chain([ear(Lineage::pure(Bat))])
        .collect();
    assert!(
        ears.windows(2).all(|pair| pair[0] < pair[1]),
        "ears {ears:?}"
    );
    // A glider's eyes larger than a bird's, and no step where the mouth's kind
    // switches.
    let (before, after) = (
        eye(Lineage::cross(Bird, Glider, 0.49)),
        eye(Lineage::cross(Bird, Glider, 0.51)),
    );
    assert!(eye(Lineage::pure(Glider)) > eye(Lineage::pure(Bird)));
    assert!(
        (after - before).abs() < 0.05 * before,
        "{before} to {after} across the switch"
    );
}

#[test]
fn a_beak_lengthens_with_its_axis_and_hooks_with_its_own() {
    let bird = Lineage::pure(WingedFamily::Bird);
    let reach = |length: f32| {
        let (avatar, head) = built(params(bird, [length, 0.0, -1.0]));
        furthest(&mandibles(&avatar, &head).0) / head.radius
    };
    let reaches = [-1.0, 0.0, 1.0, 2.0].map(reach);
    assert!(
        reaches.windows(2).all(|pair| pair[0] < pair[1]),
        "reach {reaches:?}"
    );
    // How far the upper's tip comes down past the lower mandible's lowest point:
    // a straight bill's stops above it, a parrot's curls under it.
    let curl = |hook: f32| {
        let (avatar, head) = built(params(bird, [0.3, hook, -1.0]));
        let (upper, lower) = mandibles(&avatar, &head);
        (lowest(&lower) - lowest(&upper)) / head.radius
    };
    let curls = [-1.5, -0.5, 0.5, 1.5].map(curl);
    assert!(
        curls.windows(2).all(|pair| pair[0] < pair[1]),
        "curl {curls:?}"
    );
    assert!(curls[0] < 0.0 && curls[3] > 0.0, "curl {curls:?}");
}

/// How far behind the face's middle the face stands at the side of the disc,
/// in head radii: negative on a face that bulges, toward nought or past it on
/// one that is a dish.
fn dish(head: &BeastHead) -> f32 {
    let at = |round: f32| {
        let toward = Vec3::new(round.sin(), -0.15, round.cos()).normalize();
        head.leave(Vec3::ZERO, toward)
            .expect("the ray leaves the head")
            .z
    };
    (at(0.6) - at(0.0)) / head.radius
}

/// How far ahead of the head's centre the middle of a pure bird's face
/// stands at `disc` on its axis, in head radii, along a ray a little below
/// level.
fn face_reach(disc: f32) -> f32 {
    let record = AvatarRecord::new(
        "Winged",
        Archetype::Winged(params(Lineage::pure(WingedFamily::Bird), [0.0, 0.0, disc])),
    );
    let head = record
        .skeleton()
        .beast_head
        .expect("a winged body declares a beast's head");
    let toward = Vec3::new(0.0, -0.15, 1.0).normalize();
    head.leave(Vec3::ZERO, toward)
        .expect("the ray leaves the head")
        .z
        / head.radius
}

/// **The disc fades with its axis and never cuts the face away** (#422, the
/// critic): a dish made shallower as the axis fell hung from a ball that
/// grew without end, so just short of no disc the front of the face was cut
/// flat, 0.46 head radii out against the bare face's 1.12, and it jumped
/// back over the axis's last thousandth.
#[test]
fn the_disc_fades_with_its_axis_and_never_cuts_the_face_away() {
    let axis = [-1.0, -0.999, -0.99, -0.9, -0.5, 0.0, 0.2];
    let reaches = axis.map(face_reach);
    assert!(
        (reaches[0] - reaches[1]).abs() < 0.01,
        "the face jumps at the disc's last thousandth: {reaches:?}"
    );
    assert!(
        reaches.windows(2).all(|pair| pair[1] <= pair[0] + 1.0e-4),
        "the face does not sink steadily as the disc deepens: {reaches:?} at {axis:?}"
    );
}

#[test]
fn the_disc_comes_with_its_axis_and_turns_the_eyes_with_it() {
    let bird = Lineage::pure(WingedFamily::Bird);
    let spacing = |avatar: &Avatar, head: &BeastHead| {
        let eyes = avatar.parts.eyes.as_ref().expect("eyes");
        (eyes.left.pivot.x - eyes.right.pivot.x).abs() / head.radius
    };
    let (owl, disc) = built(params(bird, OWL));
    let (crow, bare) = built(params(bird, [0.0, 0.0, -1.0]));
    assert!(disc.has_disc() && !bare.has_disc());
    assert!(
        dish(&disc) > dish(&bare) + 0.1,
        "the face is no flatter with a disc: {} against {}",
        dish(&disc),
        dish(&bare)
    );
    // Close-set on the face with a disc, apart on the sides of the head without.
    assert!(
        spacing(&owl, &disc) < 0.85 * spacing(&crow, &bare),
        "{} against {}",
        spacing(&owl, &disc),
        spacing(&crow, &bare)
    );
    // And a bat has none whatever its axis says.
    let (_, bat) = built(params(Lineage::pure(WingedFamily::Bat), OWL));
    assert!(!bat.has_disc());
}

/// How many of the eyes' draw's vertices `pose` moves from rest.
fn moved(avatar: &Avatar, pose: &Pose) -> usize {
    let eye = |meshes: Vec<symbios_avatar::AvatarMesh>| {
        meshes
            .into_iter()
            .find(|mesh| mesh.kind == MeshKind::Eye)
            .expect("an eyes' draw")
            .mesh
    };
    let rest = eye(avatar.posed(&Pose::rest(&avatar.rig), 0.0));
    let posed = eye(avatar.posed(pose, 0.0));
    rest.positions
        .iter()
        .zip(&posed.positions)
        .filter(|(one, other)| one.distance(**other) > 1.0e-4)
        .count()
}

#[test]
fn the_lower_mandible_opens_with_the_jaw_and_the_upper_stays() {
    for head in [OWL, [0.3, 1.0, -1.0], [1.0, -1.0, -1.0]] {
        let (avatar, _) = built(params(Lineage::pure(WingedFamily::Bird), head));
        let face = avatar.parts.beast.as_ref().expect("a beast's face");
        let worn = avatar.parts.adornments.as_ref().expect("a beak");
        assert!(face.jaw.joint.is_some(), "{head:?}: the jaw is rigged");
        assert_eq!(worn.jaw_joint, face.jaw.joint, "{head:?}");
        let mut open = Pose::rest(&avatar.rig);
        face.jaw.pose(&mut open, 1.0, 0.0);
        // The lower mandible's every vertex (the draw splits a few more along
        // its texture's seam), and nothing of the upper's or the eyes'.
        let (opened, lower, upper) = (
            moved(&avatar, &open),
            worn.on_jaw.vertex_count(),
            worn.on_head.vertex_count(),
        );
        assert!(lower > 0, "{head:?}: no lower mandible on the jaw");
        assert!(
            opened >= lower && opened < lower + upper / 2,
            "{head:?}: {opened} moved of a lower mandible of {lower}"
        );
    }
}

#[test]
fn a_record_and_a_code_from_before_the_axes_read_as_an_owl_s_head() {
    // A bird-led cross as 0.20.0 wrote it: no head axes.
    let json = r#"{"$type":"network.symbios.avatar.defs#winged","size":640,"wingLength":200,
        "neckLength":-100,"legLength":0,"tailLength":300,"headSize":100,"appeal":1000,
        "family":"bird","hybrid":"bat","blend":300}"#;
    let Archetype::Winged(read) = serde_json::from_str::<Archetype>(json).expect("reads") else {
        panic!("still winged");
    };
    assert_eq!(
        [read.beak_length, read.beak_hook, read.facial_disc],
        OWL,
        "{read:?}"
    );
    assert_eq!(read.hybrid, Some(WingedFamily::Bat));
    // That body's code, as 0.20.0 wrote it: version 10, nothing appended.
    let code = AvatarRecord::new("Old", Archetype::Winged(read)).share_code();
    assert_eq!(code, OLD_CODE, "an owl-headed body's code moved");
    let back = AvatarRecord::from_share_code("Old", OLD_CODE).expect("a 0.20 code decodes");
    let Archetype::Winged(back) = back.archetype else {
        panic!("still winged");
    };
    assert_eq!([back.beak_length, back.beak_hook, back.facial_disc], OWL);
    assert!((back.tail_length - 0.3).abs() < 0.03, "{back:?}");
}

/// The record above's share code, written by the published 0.20.0 (#422).
const OLD_CODE: &str = "181R0-0M8FE-08S15-A000M-V040A-MMNK0-3K00Q-0";

#[test]
fn every_family_and_cross_builds_clean_at_the_head_s_ends() {
    use WingedFamily::{Bat, Bird, Glider};
    let mut lineages = vec![
        Lineage::pure(Bird),
        Lineage::pure(Bat),
        Lineage::pure(Glider),
    ];
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.25, 0.49, 0.51, 0.75] {
            lineages.push(Lineage::cross(a, b, blend));
        }
    }
    for lineage in lineages {
        for head in [OWL, [-3.0, -3.0, -3.0], [3.0, 3.0, 3.0], [3.0, -3.0, 1.5]] {
            let mut asked = params(lineage, head);
            asked.appeal = 1.5;
            let (avatar, _) = built(asked);
            let label = format!("{lineage:?} {head:?}");
            for mesh in &avatar.meshes {
                assert!(
                    mesh.mesh.positions.iter().all(|point| point.is_finite()),
                    "{label}: a {:?} vertex is not finite",
                    mesh.kind
                );
            }
            if let Some(worn) = &avatar.parts.adornments {
                for mesh in [&worn.on_head, &worn.on_jaw] {
                    let report = mesh.manifold_report();
                    assert!(report.is_clean(), "{label}: {report:?}");
                }
            }
        }
    }
}

/// **A rolled bird draws a kind of head first** (the owner's call at W3's
/// review): an owl most often, and a crow's or a songbird's straight beak
/// about one bird in four, where drawn about the owl alone it was one in
/// eight - and a disc comes with an owl's hooked beak, not a crow's.
#[test]
fn a_rolled_bird_draws_a_kind_of_head_and_a_quarter_have_a_straight_beak() {
    use symbios_avatar::plan::{Category, Rolls};
    const ROLLS: i64 = 400;
    let composites = symbios_avatar::Composites::default();
    let (mut straight, mut discs, mut hooked) = (0, 0, 0);
    for seed in 0..ROLLS {
        let mut archetype = Archetype::Winged(params(Lineage::pure(WingedFamily::Bird), OWL));
        archetype.reroll(Category::Head, &Rolls::new(seed), &composites);
        let Archetype::Winged(bird) = archetype else {
            panic!("a re-roll changed the plan");
        };
        if bird.beak_hook < -0.5 {
            straight += 1;
        }
        if bird.facial_disc > -1.0 {
            discs += 1;
            if bird.beak_hook >= -0.5 {
                hooked += 1;
            }
        }
    }
    let share = straight as f32 / ROLLS as f32;
    assert!(
        (0.18..=0.32).contains(&share),
        "{share} of rolled birds have a straight beak"
    );
    assert!(
        hooked as f32 >= 0.9 * discs as f32,
        "{hooked} of {discs} rolled discs come with a hooked beak"
    );
}

/// **A bat keeps the bird's beak it carries through a re-roll of its head**
/// (#429): the axes shape only a bird-led head, so a bat's roll draws none of
/// them, and the beak comes back with the bird.
#[test]
fn a_bat_keeps_a_carried_beak_through_a_head_re_roll() {
    use symbios_avatar::plan::{Category, Rolls};
    let mut bird = params(Lineage::pure(WingedFamily::Bird), [0.8, -0.6, -1.2]);
    let carried = [bird.beak_length, bird.beak_hook, bird.facial_disc];
    bird.set_kind(WingedFamily::Bat, None, 0.0);
    let mut archetype = Archetype::Winged(bird);
    archetype.reroll(
        Category::Head,
        &Rolls::new(7),
        &symbios_avatar::Composites::default(),
    );
    let Archetype::Winged(mut back) = archetype else {
        panic!("a re-roll changed the plan");
    };
    back.set_kind(WingedFamily::Bird, None, 0.0);
    assert_eq!(
        [back.beak_length, back.beak_hook, back.facial_disc].map(f32::to_bits),
        carried.map(f32::to_bits),
        "the beak came back as {:?}, not {carried:?}",
        [back.beak_length, back.beak_hook, back.facial_disc]
    );
}

/// **A young bird's beak is shorter** (#429; research-10 fact 10): the same
/// beak reaches less far out of the head, in its radii, on a bird drawn all
/// the way to its young (`appeal` 1.5) than on one drawn as an adult, and a
/// crow's long beak by more than an owl's short one.
#[test]
fn a_young_bird_s_beak_is_shorter() {
    let bird = Lineage::pure(WingedFamily::Bird);
    let reach = |head: [f32; 3], appeal: f32| {
        let mut asked = params(bird, head);
        asked.appeal = appeal;
        let (avatar, head) = built(asked);
        furthest(&mandibles(&avatar, &head).0) / head.radius
    };
    let mut cut = Vec::new();
    for head in [OWL, [1.0, -1.0, -1.0]] {
        let (adult, young) = (reach(head, 0.0), reach(head, 1.5));
        assert!(
            young < adult - 0.05,
            "{head:?}: a young bird's beak reaches {young:.3} radii, an adult's {adult:.3}"
        );
        cut.push(adult - young);
    }
    assert!(
        cut[1] > cut[0],
        "the crow's beak was cut {:.3}, the owl's {:.3}",
        cut[1],
        cut[0]
    );
}

/// **The head's streams moved no other axis's roll** (#429): a winged
/// record rolled from each of eight seeds draws the family, the cross and
/// every axis 0.20.0 drew - before the head's axes and their streams existed -
/// as 0.20.0 drew them (read off the published 0.20.0, 2026-11-07). The rolls
/// go through the platform's `ln` and `cos` before they are rounded to
/// thousandths, so a value is held within one and a half of them: a libm a
/// unit off at a half-thousandth moves it by one, and a stream renamed moves
/// it wholesale (seed 1's headSize to -0.529).
#[test]
fn the_head_s_streams_moved_no_other_roll() {
    use WingedFamily::{Bat, Bird, Glider};
    /// A seed, its family, cross, blend and size, then its wingLength,
    /// neckLength, legLength, tailLength, headSize and appeal.
    type Rolled = (i64, WingedFamily, Option<WingedFamily>, f32, f32, [f32; 6]);
    #[rustfmt::skip]
    const ROLLED: [Rolled; 8] = [
        (1, Bat, None, 0.0, 0.697, [0.487, -2.93, -0.289, 0.104, 0.207, 0.851]),
        (2, Bat, Some(Glider), 0.329, 0.576, [0.056, 0.414, 0.257, 0.106, -0.22, 1.543]),
        (3, Bat, None, 0.0, 0.737, [-0.074, 0.63, 0.004, -0.356, -0.198, 1.696]),
        (5, Bird, None, 0.0, 0.437, [0.209, -0.727, -0.51, -0.128, 0.211, 0.534]),
        (8, Glider, Some(Bat), 0.748, 0.572, [0.129, -0.401, -0.107, 0.12, 0.086, 0.651]),
        (13, Bat, Some(Glider), 0.331, 0.635, [-0.18, -0.431, -0.292, -0.108, 0.013, 0.915]),
        (21, Bat, None, 0.0, 0.847, [-0.441, 0.643, 0.074, -0.412, 0.544, 0.267]),
        (34, Bird, None, 0.0, 0.488, [0.124, -0.581, -1.33, 0.121, 0.115, 1.028]),
    ];
    for (seed, family, hybrid, blend, size, axes) in ROLLED {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        let Archetype::Winged(rolled) = record.archetype else {
            panic!("a roll changed the plan");
        };
        let now = [
            rolled.wing_length,
            rolled.neck_length,
            rolled.leg_length,
            rolled.tail_length,
            rolled.head_size,
            rolled.appeal,
        ];
        assert_eq!(
            (rolled.family, rolled.hybrid),
            (family, hybrid),
            "seed {seed} rolls another body than 0.20.0 rolled"
        );
        let near = |a: f32, b: f32| (a - b).abs() < 0.0015;
        assert!(
            near(rolled.blend, blend)
                && near(rolled.size, size)
                && now.iter().zip(axes).all(|(&a, b)| near(a, b)),
            "seed {seed} rolls other than 0.20.0 rolled it: blend {}, size {}, axes {now:?}",
            rolled.blend,
            rolled.size
        );
    }
}
