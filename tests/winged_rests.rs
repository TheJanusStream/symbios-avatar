//! A winged body's rests and emotes (#424): the perch's toes curled, the
//! roost's head behind the shoulder, the curl's tail round the body, a bat's
//! wrists drawn in; no wing through the trunk in any rest or emote; every
//! emote reachable through the driver and returning; a roster per family;
//! a bird's eyes nearly still while its head turns; and no person or beast
//! offered or moved by any of it.

use super::winged_bounds::{UNDER_GROUND, furthest, lowest_drawn};
use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
use symbios_avatar::anim::winged::rests as winged;
use symbios_avatar::anim::{Clip, Key, Target, Track, emote, gesture, look_at};
use symbios_avatar::face::eye::OCULAR_LIMIT;
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Limb, Pose, QuadrupedParams, Rig, Vec3, WingedFamily,
    WingedParams, Zone,
};

fn built(lineage: Lineage<WingedFamily>) -> Avatar {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    let record = AvatarRecord::new("Winged", Archetype::Winged(params));
    Avatar::build_for_shape(&record).expect("a winged body builds")
}

fn family(family: WingedFamily) -> Avatar {
    built(Lineage::pure(family))
}

/// `name` held at `time` on a body standing at rest.
fn posed_at(rig: &Rig, name: &str, time: f32) -> (Pose, Vec<Vec3>) {
    let clip = gesture::for_body(rig, name).unwrap_or_else(|| panic!("{name} is offered"));
    let mut pose = Pose::rest(rig);
    clip.apply(rig, &mut pose, time);
    let positions = pose.forward(rig).positions;
    (pose, positions)
}

/// Every chain of drawn toes, root to claw tip.
fn toes(rig: &Rig) -> Vec<Vec<usize>> {
    let mut chains = Vec::new();
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let pad = rig.in_zone(Zone::Extremity(limb))[0];
        for root in (0..rig.len()).filter(|&joint| {
            rig.joints[joint].parent == Some(pad)
                && rig.joints[joint].role == symbios_avatar::Role::Digit
        }) {
            let mut chain = vec![root];
            while let Some(next) = (0..rig.len()).find(|&joint| {
                rig.joints[joint].parent == chain.last().copied()
                    && rig.joints[joint].role == symbios_avatar::Role::Digit
            }) {
                chain.push(next);
            }
            chains.push(chain);
        }
    }
    chains
}

/// **A perching bird crouches and grips**: its pelvis comes down, each toe
/// closes - its tip at least a fifth nearer its root than standing - and no
/// claw's tip goes under the flat top it stands on.
#[test]
fn a_perching_bird_crouches_and_curls_its_toes() {
    let avatar = family(WingedFamily::Bird);
    let rig = &avatar.rig;
    let (_, at) = posed_at(rig, "Perch", 1.0);
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let sank = rig.joints[pelvis].position.y - at[pelvis].y;
    assert!(sank > 0.03, "the pelvis came down {:.0} mm", sank * 1000.0);
    let chains = toes(rig);
    assert_eq!(chains.len(), 8, "a bird's eight toes");
    for chain in &chains {
        let (root, tip) = (chain[0], *chain.last().expect("a toe"));
        let standing = rig.joints[root].position.distance(rig.joints[tip].position);
        let gripping = at[root].distance(at[tip]);
        assert!(
            gripping < 0.9 * standing,
            "toe {root}: tip {:.1} mm from its root gripping, {:.1} standing",
            gripping * 1000.0,
            standing * 1000.0
        );
        for &joint in chain {
            assert!(
                at[joint].y > -0.001,
                "toe {root}'s joint {joint} went {:.1} mm under the top",
                -at[joint].y * 1000.0
            );
        }
    }
}

/// **A roosting bird's head is turned back behind its shoulder**: it faces
/// backward, and the end of its bill is behind the shoulders its wings hang
/// from, on its left.
#[test]
fn a_roosting_bird_lays_its_head_back_behind_its_shoulder() {
    let avatar = family(WingedFamily::Bird);
    let rig = &avatar.rig;
    let (pose, at) = posed_at(rig, "Roost", 1.0);
    let head = rig.in_zone(Zone::Head)[0];
    let facing = pose.forward(rig).rotations[head] * Vec3::Z;
    assert!(facing.z < -0.3, "the head faces {facing:?}");
    assert!(
        facing.x > 0.2,
        "turned back over its left shoulder: {facing:?}"
    );
    // The bill's tip: the head chain's last joint.
    let mut bill = head;
    while let Some(next) = (0..rig.len()).find(|&joint| {
        rig.joints[joint].parent == Some(bill)
            && rig.joints[joint].zone == Zone::Head
            && rig.joints[joint].role == symbios_avatar::Role::Deform
            && rig.joints[joint].radius > 0.0
    }) {
        bill = next;
    }
    let wings = rig.wings.as_ref().expect("wings");
    let shoulder = wings
        .sides
        .iter()
        .map(|side| at[side.drives[0].joint].z)
        .fold(f32::MIN, f32::max);
    assert!(
        at[bill].z < shoulder,
        "the bill stands {:.0} mm ahead of the shoulders",
        (at[bill].z - shoulder) * 1000.0
    );
    // And the body is down on its feet, lower than its perch.
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let (_, perched) = posed_at(rig, "Perch", 1.0);
    assert!(at[pelvis].y < perched[pelvis].y - 0.01);
}

/// How far round a tail's tip heads from where it leaves the body, in
/// radians about the vertical, and how far it is from the head.
fn tail_wrap(rig: &Rig, at: &[Vec3]) -> (f32, f32) {
    let tail = rig.in_zone(Zone::Tail);
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let first = flat(at[tail[1]] - at[tail[0]]);
    let mut turned = 0.0f32;
    let mut previous = first;
    for pair in tail.windows(2).skip(1) {
        let run = flat(at[pair[1]] - at[pair[0]]);
        let cross = previous.cross(run).y;
        turned += cross.atan2(previous.dot(run));
        previous = run;
    }
    let head = rig.in_zone(Zone::Head)[0];
    (turned, at[*tail.last().expect("a tail")].distance(at[head]))
}

/// **A curled glider wraps its tail round its body**: its tail laid along
/// the ground turning round toward its head, and no higher than it stood.
#[test]
fn a_curled_glider_wraps_its_tail_round_its_body() {
    let avatar = family(WingedFamily::Glider);
    let rig = &avatar.rig;
    let (_, standing) = posed_at(rig, "Curl", 0.0);
    let (_, at) = posed_at(rig, "Curl", 1.0);
    let (turned, near) = tail_wrap(rig, &at);
    let (straight, far) = tail_wrap(rig, &standing);
    assert!(
        // Its first bones come down off a trunk held clear of the ground by
        // its patagium's spars, so the turn is read past them: over a
        // quarter turn and a half along the ground, tip near the head.
        turned.abs() > 1.5,
        "the tail turned {:.0} degrees round",
        turned.abs().to_degrees()
    );
    assert!(straight.abs() < 0.5, "standing, it turned {straight}");
    assert!(
        near < 0.6 * far,
        "the tail's tip came to {near:.3} m of the head from {far:.3}"
    );
    for &joint in &rig.in_zone(Zone::Tail)[1..] {
        assert!(
            at[joint].y < 2.5 * rig.joints[joint].radius + 0.005,
            "tail joint {joint} stands {:.0} mm up",
            at[joint].y * 1000.0
        );
    }
    // No higher than it stood: it lies as low as its folded patagium, which
    // hangs to the ground already, lets it (`docs/winged-anatomy.md`); the
    // whole drawn body is held to the ground by
    // `no_rest_or_emote_sinks_the_body_or_jumps_a_joint`.
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    assert!(
        at[pelvis].y < standing[pelvis].y + 0.001,
        "the pelvis rose {:.0} mm",
        (at[pelvis].y - standing[pelvis].y) * 1000.0
    );
}

/// How far into the ground a resting belly may sink, and how far off it it
/// may stand, in metres: it rests on it.
const BELLY_SINK: f32 = 0.006;
const BELLY_LIFT: f32 = 0.012;

/// The lowest point of the trunk's own skin in `pose` - skin held nine
/// tenths or more by the pelvis, the abdomen and the chest - in metres above
/// the ground.
fn belly(avatar: &Avatar, pose: &Pose) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    let body = posed.deform(rig, &avatar.parts.body.positions, &avatar.parts.weights);
    body.iter()
        .zip(&avatar.parts.weights.vertices)
        .filter(|(_, skin)| {
            skin.iter()
                .filter(|hold| {
                    matches!(
                        rig.joints[hold.joint as usize].zone,
                        Zone::Pelvis | Zone::Abdomen | Zone::Chest
                    )
                })
                .map(|hold| hold.weight)
                .sum::<f32>()
                >= 0.9
        })
        .map(|(at, _)| at.y)
        .fold(f32::MAX, f32::min)
}

/// How far a four-legged body's spine bends round about the vertical: the
/// pelvis-to-loin line against the withers-to-neck line.
fn spine_bend(rig: &Rig, at: &[Vec3]) -> f32 {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let loin = rig.in_zone(Zone::Abdomen)[0];
    let neck = rig.in_zone(Zone::Neck)[0];
    let withers = rig.joints[neck].parent.expect("withers");
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    let (a, b) = (flat(at[loin] - at[pelvis]), flat(at[neck] - at[withers]));
    a.cross(b).y.atan2(a.dot(b))
}

/// **A resting bat lies low, its wrists drawn in**: down on its belly, each
/// wrist on the ground and nearer its shoulder than standing, its spine not
/// curled - a bat rests, a glider curls - and **a cross between them lies
/// between the two**, curled by its glider share.
#[test]
fn a_resting_bat_lies_low_and_a_cross_curls_by_its_glider_share() {
    let avatar = family(WingedFamily::Bat);
    let rig = &avatar.rig;
    let (pose, at) = posed_at(rig, "Rest", 1.0);
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    assert!(at[pelvis].y < rig.joints[pelvis].position.y - 0.01);
    let low = belly(&avatar, &pose);
    assert!(
        (-BELLY_SINK..BELLY_LIFT).contains(&low),
        "the belly rests {:.1} mm off the ground",
        low * 1000.0
    );
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        let wrist = rig.in_zone(Zone::Extremity(limb))[0];
        let shoulder = rig.limb_chain(limb).expect("an arm")[0];
        let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).length();
        let standing = flat(rig.joints[wrist].position - rig.joints[shoulder].position);
        let resting = flat(at[wrist] - at[shoulder]);
        assert!(
            resting < 0.8 * standing,
            "{limb:?}: the wrist {:.0} mm out resting, {:.0} standing",
            resting * 1000.0,
            standing * 1000.0
        );
        assert!(
            (at[wrist].y - rig.joints[wrist].position.y).abs() < 0.01,
            "{limb:?}'s wrist left the ground by {:.0} mm",
            (at[wrist].y - rig.joints[wrist].position.y) * 1000.0
        );
    }
    let bends: Vec<f32> = [
        Lineage::pure(WingedFamily::Bat),
        Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.3),
        Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.7),
        Lineage::pure(WingedFamily::Glider),
    ]
    .into_iter()
    .map(|lineage| {
        let avatar = built(lineage);
        let rig = &avatar.rig;
        let name = winged::roster(rig)[0];
        let (_, at) = posed_at(rig, name, 1.0);
        spine_bend(rig, &at)
    })
    .collect();
    assert!(bends[0].abs() < 0.05, "a bat's spine bent {}", bends[0]);
    for pair in bends.windows(2) {
        assert!(
            pair[1] > pair[0] + 0.1,
            "the curl by glider share: {bends:?}"
        );
    }
}

/// How many of a wing's free vertices stand inside the body's trunk in
/// `pose`, and how many were tested: `tests/wings.rs`'s measure, asked of a
/// rest or an emote rather than of the extension alone.
fn inside_trunk(avatar: &Avatar, pose: &Pose) -> (usize, usize) {
    let rig = &avatar.rig;
    let wings = avatar.parts.wings.as_ref().expect("wings");
    let posed = pose.forward(rig);
    let mut body = avatar.parts.body.clone();
    body.positions = posed.deform(rig, &body.positions, &avatar.parts.weights);
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

/// The share of a wing's free vertices `tests/wings.rs` allows inside the
/// trunk: a few where an edge brushes it.
const INSIDE_SHARE: f32 = 0.0125;

/// **No wing passes through the trunk in any rest or emote**, on every
/// family, each pair crossed at 0.49, 0.5 and 0.51, a bird-led and a
/// bat-led cross further out, and ten rolls: of each wing's free vertices
/// at most [`INSIDE_SHARE`] inside the pelvis, the abdomen or the chest, at
/// each emote's middle and both its quarters and each rest's end.
#[test]
fn no_wing_passes_through_the_trunk_in_any_rest_or_emote() {
    let mut failures = Vec::new();
    let mut worst = (String::new(), 0.0f32);
    let mut bodies = sweep_bodies();
    for lineage in [
        Lineage::cross(WingedFamily::Bird, WingedFamily::Glider, 0.3),
        Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.6),
    ] {
        let mut params = WingedParams::default();
        params.set_lineage(lineage);
        bodies.push((
            format!("{lineage:?}"),
            AvatarRecord::new("Winged", Archetype::Winged(params)),
        ));
    }
    for (lineage, record) in &bodies {
        let avatar = Avatar::build_for_shape(record).expect("a winged body builds");
        let rig = &avatar.rig;
        for name in winged::roster(rig) {
            if *name == emote::GET_UP {
                continue;
            }
            let times: &[f32] = if emote::seconds(rig, name).is_some() {
                &[0.5, 1.0]
            } else {
                &[0.3, 0.5, 0.75]
            };
            for &time in times {
                let (pose, _) = posed_at(rig, name, time);
                let (inside, tested) = inside_trunk(&avatar, &pose);
                let share = inside as f32 / tested.max(1) as f32;
                let label = format!("{lineage} {name} at {time}");
                if share > worst.1 {
                    worst = (label.clone(), share);
                }
                if share > 0.004 {
                    println!("{label}: {:.2}%", share * 100.0);
                }
                if share > INSIDE_SHARE {
                    failures.push(format!("{label}: {inside} of {tested} inside the trunk"));
                }
            }
        }
    }
    println!("deepest: {:.2}% at {}", worst.1 * 100.0, worst.0);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Joint positions through the driver, `frames` sixtieths of a second, the
/// gesture `asked` at frame 10 and [`emote::GET_UP`] at `up`.
fn driven(rig: &Rig, asked: Option<&str>, up: usize, frames: usize) -> (Vec<Vec3>, Vec<bool>) {
    let mut driver = Driver::seeded(3);
    let mut last = Vec::new();
    let mut playing = Vec::new();
    for frame in 0..frames {
        let gesture = match frame {
            10 => asked,
            frame if frame == up => Some(emote::GET_UP),
            _ => None,
        };
        let inputs = Inputs {
            delta: 1.0 / 60.0,
            gesture,
            ..Inputs::default()
        };
        if let Some(driven) = driver.drive(rig, &inputs, level_ground) {
            last = driven.pose.forward(rig).positions;
        }
        playing.push(driver.gesture().is_some());
    }
    (last, playing)
}

/// **Every emote is reachable and returns**: each family's every name is
/// offered; a rest keeps its own time, ends down and gets up; an emote ends
/// where it began; and through the driver each plays once asked, ends, and
/// leaves the body where a body never asked stands.
#[test]
fn every_emote_is_reachable_through_the_driver_and_returns() {
    for kin in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        let avatar = family(kin);
        let rig = &avatar.rig;
        let (idle, _) = driven(rig, None, usize::MAX, 600);
        for name in winged::roster(rig) {
            let clip = gesture::for_body(rig, name).unwrap_or_else(|| panic!("{kin:?} {name}"));
            if *name == emote::GET_UP {
                continue;
            }
            let state = emote::seconds(rig, name).is_some();
            assert_eq!(
                gesture::returns_to_rest(&clip),
                !state,
                "{kin:?} {name}: a rest ends down, an emote at rest"
            );
            assert_eq!(state, emote::getting_up(rig, name).is_some());
            let (at, playing) = driven(rig, Some(name), 200, 600);
            assert!(playing[11], "{kin:?} {name} did not start");
            assert!(!playing[599], "{kin:?} {name} never ended");
            let moved = at
                .iter()
                .zip(&idle)
                .map(|(a, b)| a.distance(*b))
                .fold(0.0f32, f32::max);
            assert!(
                moved < 0.002,
                "{kin:?} {name} left a joint {:.1} mm from where it stands",
                moved * 1000.0
            );
        }
    }
}

/// **A roosting bird shuts its eyes**, and opens them as it gets up; a
/// perching one keeps them open.
#[test]
fn a_roosting_bird_shuts_its_eyes() {
    let avatar = family(WingedFamily::Bird);
    let rig = &avatar.rig;
    let closure = |name: &str| {
        let mut driver = Driver::seeded(5);
        let mut most = 0.0f32;
        let mut last = 0.0f32;
        for frame in 0..400 {
            let gesture = match frame {
                5 => Some(name),
                300 => Some(emote::GET_UP),
                _ => None,
            };
            let inputs = Inputs {
                delta: 1.0 / 60.0,
                gesture,
                eyes: avatar.parts.eyes.as_ref(),
                lids: Some(0.0),
                ..Inputs::default()
            };
            if let Some(driven) = driver.drive(rig, &inputs, level_ground) {
                if frame == 290 {
                    most = driven.closure;
                }
                last = driven.closure;
            }
        }
        (most, last)
    };
    let (asleep, woken) = closure("Roost");
    assert!(asleep > 0.95, "roosting, the lids are {asleep:.2} shut");
    assert!(woken < 0.05, "awake again, {woken:.2}");
    let (perched, _) = closure("Perch");
    assert!(perched < 0.05, "perching, {perched:.2}");
}

/// **Each family is offered its own roster**, and nothing else: a bird
/// never a person's wave, a bat or a glider never a beast's sit; a cross its
/// leading family's; a person and a beast what they always were.
#[test]
fn each_family_is_offered_its_own_roster() {
    for (lineage, roster) in [
        (Lineage::pure(WingedFamily::Bird), winged::BIRD),
        (Lineage::pure(WingedFamily::Bat), winged::BAT),
        (Lineage::pure(WingedFamily::Glider), winged::GLIDER),
        (
            Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.3),
            winged::BIRD,
        ),
        (
            Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.7),
            winged::BAT,
        ),
        (
            Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.7),
            winged::GLIDER,
        ),
    ] {
        let avatar = built(lineage);
        let rig = &avatar.rig;
        assert_eq!(gesture::roster(rig), roster, "{lineage:?}");
        for name in gesture::ROSTER.iter().chain(emote::ROSTER) {
            if roster.contains(name) {
                continue;
            }
            assert!(
                gesture::for_body(rig, name).is_none(),
                "{lineage:?} is offered a person's or a beast's {name}"
            );
        }
        for name in roster {
            assert!(gesture::for_body(rig, name).is_some(), "{lineage:?} {name}");
        }
    }
    let person = Avatar::build_for_shape(&AvatarRecord::default()).expect("builds");
    assert_eq!(gesture::roster(&person.rig), gesture::ROSTER);
    let beast = Avatar::build_for_shape(&AvatarRecord::new(
        "Beast",
        Archetype::Quadruped(QuadrupedParams::default()),
    ))
    .expect("builds");
    assert_eq!(gesture::roster(&beast.rig), emote::ROSTER);
    for name in winged::ROSTER {
        // A shake and a head tilt are a beast's too.
        if emote::ROSTER.contains(name) || gesture::ROSTER.contains(name) {
            continue;
        }
        assert!(
            gesture::for_body(&person.rig, name).is_none(),
            "a person's {name}"
        );
        assert!(
            gesture::for_body(&beast.rig, name).is_none(),
            "a beast's {name}"
        );
    }
}

/// How far each eye's globe turned from straight ahead, the larger.
fn eye_turn(avatar: &Avatar, pose: &Pose) -> f32 {
    let eyes = avatar.parts.eyes.as_ref().expect("eyes");
    [&eyes.left, &eyes.right]
        .into_iter()
        .filter_map(|eye| eye.globe_joint)
        .map(|joint| pose.rotations[joint].angle_between(symbios_avatar::Quat::IDENTITY))
        .fold(0.0f32, f32::max)
}

/// **A bird looks with its head, its eyes nearly still**: asked to look 40
/// degrees aside, an owl's eyes turn at most 4 degrees (research-10 6.1: a
/// barn owl's 1 to 4) and a songbird's more but under 12, while the head
/// turns the rest; a bat's turn as a beast's, and a person's as they did.
#[test]
fn a_bird_looks_with_its_head_and_its_eyes_nearly_still() {
    let look = |avatar: &Avatar, config| {
        let rig = &avatar.rig;
        let head = rig.in_zone(Zone::Head)[0];
        let from = rig.joints[head].position;
        let aside = 40f32.to_radians();
        let target = from + Vec3::new(aside.sin(), 0.0, aside.cos()) * 2.0;
        let mut pose = Pose::rest(rig);
        let gaze = look_at(rig, &mut pose, target, &config);
        let eyes = avatar.parts.eyes.as_ref().expect("eyes");
        eyes.look(rig, &mut pose, target);
        (gaze.turned, eye_turn(avatar, &pose), eyes.range())
    };
    let owl = family(WingedFamily::Bird);
    let (head, eyes, range) = look(&owl, winged::glance_config());
    assert!(
        eyes <= 4f32.to_radians() + 1e-4,
        "an owl's eyes turned {:.1} degrees",
        eyes.to_degrees()
    );
    assert!(
        head > 30f32.to_radians(),
        "its head turned {:.1}",
        head.to_degrees()
    );
    assert!(range < 0.07);

    let params = WingedParams {
        beak_length: 0.2,
        beak_hook: -1.3,
        facial_disc: -1.0,
        ..WingedParams::default()
    };
    let songbird = Avatar::build_for_shape(&AvatarRecord::new("Song", Archetype::Winged(params)))
        .expect("builds");
    let (_, sung, _) = look(&songbird, winged::glance_config());
    assert!(
        sung > eyes + 0.03 && sung < 12f32.to_radians(),
        "a songbird's {:.1}",
        sung.to_degrees()
    );

    let bat = family(WingedFamily::Bat);
    assert_eq!(bat.parts.eyes.as_ref().expect("eyes").range(), OCULAR_LIMIT);
    let person = Avatar::build_for_shape(&AvatarRecord::default()).expect("builds");
    let (_, theirs, range) = look(
        &person,
        symbios_avatar::anim::GazeConfig {
            limit: 0.0,
            shares: [0.0, 0.0, 1.0],
        },
    );
    assert_eq!(range, OCULAR_LIMIT);
    assert!(
        theirs > 25f32.to_radians(),
        "a person's eyes turned {:.1}",
        theirs.to_degrees()
    );
}

/// **No person or beast is moved by a winged body's parts**: a track of each
/// new target, at its widest, leaves either at rest to the bit.
#[test]
fn a_person_and_a_beast_are_not_moved_by_a_winged_bodys_targets() {
    let person = Avatar::build_for_shape(&AvatarRecord::default()).expect("builds");
    let beast = Avatar::build_for_shape(&AvatarRecord::new(
        "Beast",
        Archetype::Quadruped(QuadrupedParams::default()),
    ))
    .expect("builds");
    for avatar in [&person, &beast] {
        let rig = &avatar.rig;
        for target in [
            Target::Wing(Limb::ForeLeft),
            Target::Wing(Limb::ForeRight),
            Target::Fan,
            Target::Grip,
            Target::Tuck,
            Target::Curl,
        ] {
            let clip = Clip::new([Track::new(target, [Key::new(0.0, Vec3::ONE)])]);
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, 0.5);
            assert_eq!(
                pose,
                Pose::rest(rig),
                "{target:?} moved a body with no wings"
            );
        }
        // And a roll, which a bird's shake now makes on two legs, still does
        // nothing to a person.
        if !rig.four_legged() {
            let clip = Clip::new([Track::new(Target::Roll, [Key::new(0.0, Vec3::ONE)])]);
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, 0.5);
            assert_eq!(pose, Pose::rest(rig), "a roll moved a person");
        }
    }
}

/// How far a bird's head turns from where it faced at rest over `seconds`
/// standing, frame by frame, through the driver: the idle's glances.
fn glances(rig: &Rig, seconds: f32) -> Vec<f32> {
    let mut driver = Driver::seeded(9);
    let head = rig.in_zone(Zone::Head)[0];
    let step = 1.0 / 60.0;
    let mut turns = Vec::new();
    for _ in 0..(seconds / step) as usize {
        let inputs = Inputs {
            delta: step,
            ..Inputs::default()
        };
        if let Some(driven) = driver.drive(rig, &inputs, level_ground) {
            let facing = driven.pose.forward(rig).rotations[head] * Vec3::Z;
            let flat = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
            turns.push(flat.x.atan2(flat.z));
        }
    }
    turns
}

/// **A bird's glance is a head saccade**: its head snaps round to look and
/// holds, as an idle bird's does (10 fact 5) - its fastest turn more than
/// twice as fast as the idle's 0.45 s ease would carry the same glance.
#[test]
fn a_birds_glance_is_a_quick_head_saccade() {
    let avatar = family(WingedFamily::Bird);
    let turns = glances(&avatar.rig, 40.0);
    let widest = turns.iter().map(|turn| turn.abs()).fold(0.0f32, f32::max);
    assert!(
        widest > 0.2,
        "the bird glanced {:.1} degrees at most",
        widest.to_degrees()
    );
    let fastest = turns
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs() * 60.0)
        .fold(0.0f32, f32::max);
    // The idle's ramp is a smoothstep over 0.45 s: its fastest is 1.5 times
    // the glance over that time.
    let eased = 1.5 * widest / 0.45;
    assert!(
        fastest > 2.0 * eased,
        "the head turned at most {:.0} degrees a second, an eased glance {:.0}",
        fastest.to_degrees(),
        eased.to_degrees()
    );
}

/// Every winged body the ground-and-travel sweep reads: the three families,
/// each pair crossed either side of and at the halfway blend, a Bird x Bat
/// cross at 0.4 and ten rolls.
fn sweep_bodies() -> Vec<(String, AvatarRecord)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies = Vec::new();
    for family in [Bird, Bat, Glider] {
        let mut params = WingedParams::default();
        params.set_lineage(Lineage::pure(family));
        bodies.push((
            format!("{family:?}"),
            AvatarRecord::new("Winged", Archetype::Winged(params)),
        ));
    }
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.49, 0.5, 0.51] {
            let mut params = WingedParams::default();
            params.set_lineage(Lineage::cross(a, b, blend));
            bodies.push((
                format!("{a:?} x {b:?} {blend}"),
                AvatarRecord::new("Winged", Archetype::Winged(params)),
            ));
        }
    }
    // And a neighbour of the cross whose long folded wing hung into the
    // ground until #440: what it hid at 0.49, it hid here.
    let mut params = WingedParams::default();
    params.set_lineage(Lineage::cross(Bird, Bat, 0.4));
    bodies.push((
        "Bird x Bat 0.4".to_string(),
        AvatarRecord::new("Winged", Archetype::Winged(params)),
    ));
    for seed in 1..=10 {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        bodies.push((format!("roll {seed}"), record));
    }
    bodies
}

/// The furthest any joint may travel in one sixtieth of a second, in metres:
/// four metres a second, past any wing's flick, and short of a solve that
/// flips.
const FRAME_TRAVEL: f32 = 0.067;

/// **No rest or emote puts the body through the ground or jumps a joint**:
/// every name of every family's roster, on the three families, each pair
/// crossed at 0.49, 0.5 and 0.51, a Bird x Bat cross at 0.4 and ten rolls,
/// at 25 moments of its clip and through the driver from asking to standing
/// again - the whole drawn
/// body's lowest point (skin, toes, membrane and feathers) never more than
/// [`UNDER_GROUND`] below the ground beyond what stood below it before the
/// motion began, and through the driver no joint travelling more than
/// [`FRAME_TRAVEL`] in a frame.
///
/// **What stood below it before hid a wing stretch** until #440: a Bird x
/// Bat cross at 0.49 stood its folded feathers 83 mm under the ground and
/// one at 0.4 36, and under that its stretch pointed the claws of its
/// stretched leg 13.7 and 10.8 mm into the ground, and laid as its fold's
/// height let it lie, the wing's primaries at 0.4 into it as well.
#[test]
fn no_rest_or_emote_sinks_the_body_or_jumps_a_joint() {
    let bodies = sweep_bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for chunk in bodies.chunks(bodies.len().div_ceil(4)) {
            let failures = &failures;
            scope.spawn(move || {
                for (label, record) in chunk {
                    let avatar = Avatar::build_for_shape(record).expect("a winged body builds");
                    for line in sweep_one(label, &avatar) {
                        failures.lock().expect("a lock").push(line);
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// [`no_rest_or_emote_sinks_the_body_or_jumps_a_joint`] on one body: every
/// failure, one line each.
fn sweep_one(label: &str, avatar: &Avatar) -> Vec<String> {
    let rig = &avatar.rig;
    let mut failures = Vec::new();
    let standing = lowest_drawn(avatar, &Pose::rest(rig)).min(0.0);
    for name in winged::roster(rig) {
        if *name == emote::GET_UP {
            continue;
        }
        let clip = gesture::for_body(rig, name).expect("offered");
        let mut worst = (f32::MAX, 0.0f32);
        for step in 0..=24 {
            let time = step as f32 / 24.0;
            let mut pose = Pose::rest(rig);
            clip.apply(rig, &mut pose, time);
            let low = lowest_drawn(avatar, &pose);
            if low < worst.0 {
                worst = (low, time);
            }
        }
        if worst.0 < standing - UNDER_GROUND {
            failures.push(format!(
                "{label} {name}: {:.1} mm below the ground at {:.2} of its clip, standing {:.1}",
                -worst.0 * 1000.0,
                worst.1,
                -standing * 1000.0
            ));
        }
        // Through the driver: asked, held as long as a rest takes to go down,
        // told to get up, and run until it stands again.
        let step = 1.0 / 60.0;
        let mut driver = Driver::seeded(13);
        let up = emote::seconds(rig, name).map(|down| 4 + ((down + 0.3) / step) as usize);
        let mut before: Option<Vec<Vec3>> = None;
        let mut first: Option<f32> = None;
        let (mut lowest, mut travel) = ((f32::MAX, 0usize), (0.0f32, 0usize));
        let mut stood = 0;
        for frame in 0..900 {
            let gesture = match frame {
                3 => Some(*name),
                frame if Some(frame) == up => Some(emote::GET_UP),
                _ => None,
            };
            let inputs = Inputs {
                delta: step,
                gesture,
                ..Inputs::default()
            };
            let Some(driven) = driver.drive(rig, &inputs, level_ground) else {
                continue;
            };
            let at = driven.pose.forward(rig).positions;
            if let Some(before) = &before {
                let moved = furthest(before, &at);
                if moved > travel.0 {
                    travel = (moved, frame);
                }
            }
            before = Some(at);
            if frame == 2 || frame % 3 == 0 {
                let low = lowest_drawn(avatar, &driven.pose);
                if frame == 2 {
                    first = Some(low.min(0.0));
                } else if low < lowest.0 {
                    lowest = (low, frame);
                }
            }
            if frame > 3 && driver.gesture().is_none() {
                stood += 1;
                if stood > 30 {
                    break;
                }
            }
        }
        let first = first.unwrap_or(standing);
        if lowest.0 < first - UNDER_GROUND {
            failures.push(format!(
                "{label} {name}: driven, {:.1} mm below the ground at frame {}, standing {:.1}",
                -lowest.0 * 1000.0,
                lowest.1,
                -first * 1000.0
            ));
        }
        if travel.0 > FRAME_TRAVEL {
            failures.push(format!(
                "{label} {name}: driven, a joint travelled {:.0} mm in frame {}",
                travel.0 * 1000.0,
                travel.1
            ));
        }
    }
    failures
}

/// How far an emote moves the body at its widest: the largest distance any
/// of the body's own joints travels from where it stood, over the clip, as
/// a share of the body's height. Its own: the skeleton's deforming joints
/// but the tail's - not a tail, an ear or a wing's feather or digit, which
/// swing far for little and would let a long tail's sway pass for a shake.
fn peak_motion(rig: &Rig, name: &str) -> f32 {
    let clip = gesture::for_body(rig, name).expect("offered");
    let rest = Pose::rest(rig).forward(rig).positions;
    let own: Vec<usize> = (0..rig.len())
        .filter(|&joint| {
            let it = &rig.joints[joint];
            it.role == symbios_avatar::Role::Deform && it.zone != Zone::Tail && it.node.is_some()
        })
        .collect();
    let mut most = 0.0f32;
    for step in 0..=60 {
        let mut pose = Pose::rest(rig);
        clip.apply(rig, &mut pose, step as f32 / 60.0);
        let at = pose.forward(rig).positions;
        for &joint in &own {
            most = most.max(at[joint].distance(rest[joint]));
        }
    }
    most / rig.extent()
}

/// **Every emote reads at a glance** (#424, the look review): at its widest
/// it moves some part of the body at least [`EMOTE_FLOOR`] of the body's
/// height from where it stood, on every family - a glider's head tilt, shake
/// and stretch once barely moved.
#[test]
fn every_emote_moves_enough_to_read() {
    let mut failures = Vec::new();
    for kin in [WingedFamily::Bird, WingedFamily::Bat, WingedFamily::Glider] {
        let avatar = family(kin);
        let rig = &avatar.rig;
        for name in winged::roster(rig) {
            if *name == emote::GET_UP || emote::seconds(rig, name).is_some() {
                continue;
            }
            let peak = peak_motion(rig, name);
            println!("{kin:?} {name}: {:.1}% of its height", peak * 100.0);
            if peak < EMOTE_FLOOR {
                failures.push(format!("{kin:?} {name}: {:.1}%", peak * 100.0));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The least an emote's widest moment may move a part of the body, as a
/// share of its height.
const EMOTE_FLOOR: f32 = 0.10;
