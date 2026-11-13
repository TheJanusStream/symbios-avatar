//! A glider's folded patagium (#434): folded, it lies on the flank as a
//! band of the coat a few millimetres off the skin, not a pale plate
//! standing off the flank and under the belly, nor pleats standing a
//! centimetre or two off it - its outer face the coat's colour at rest and
//! in the curl, every free point of it near the skin at rest, walking,
//! bounding and through the curl; spread, its edges run along the foreleg and
//! the hind leg; no joint of it turns by more than a beast's paw does in a
//! frame; and a glider carries few joints for it.

use super::winged_bounds::turn_between;
use symbios_avatar::anim::driver::{Driver, Inputs, level_ground, velocity_of};
use symbios_avatar::anim::gesture;
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Limb, Pose, Quat, Rig, Role, Vec3, WingedFamily,
    WingedParams, Zone, texture,
};

const ATLAS: u32 = 256;

fn record(lineage: Lineage<WingedFamily>) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    AvatarRecord::new("Winged", Archetype::Winged(params))
}

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

/// Every body a glider leads: the glider, its crosses either way round on
/// the glider's side of the halfway blend, and the glider-led rolls of the
/// first twenty.
fn glider_led() -> Vec<(String, AvatarRecord)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut bodies = vec![("glider".to_owned(), record(Lineage::pure(Glider)))];
    for (name, lineage) in [
        ("bat x glider 0.7", Lineage::cross(Bat, Glider, 0.7)),
        ("bat x glider 0.51", Lineage::cross(Bat, Glider, 0.51)),
        ("glider x bat 0.3", Lineage::cross(Glider, Bat, 0.3)),
        ("glider x bird 0.3", Lineage::cross(Glider, Bird, 0.3)),
        ("glider x bird 0.49", Lineage::cross(Glider, Bird, 0.49)),
        ("bird x glider 0.7", Lineage::cross(Bird, Glider, 0.7)),
    ] {
        bodies.push((name.to_owned(), record(lineage)));
    }
    for seed in 1..=20 {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        let leads = matches!(&record.archetype,
            Archetype::Winged(params) if params.lineage().leading() == Glider);
        if leads {
            bodies.push((format!("roll {seed}"), record));
        }
    }
    bodies
}

/// The trunk's own joints: the pelvis, the abdomen and the chest.
fn trunk_joints(rig: &Rig) -> Vec<usize> {
    (0..rig.len())
        .filter(|&joint| {
            matches!(
                rig.joints[joint].zone,
                Zone::Pelvis | Zone::Abdomen | Zone::Chest
            ) && rig.joints[joint].role == Role::Deform
        })
        .collect()
}

/// The back's height at rest, which every distance here is a share of.
fn back(avatar: &Avatar) -> f32 {
    let rig = &avatar.rig;
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    rig.joints[withers].position.y
}

/// The membrane's free points in `pose` - past the first fifth of each
/// piece, where it roots - with the index of each.
fn free_points(avatar: &Avatar, pose: &Pose) -> Vec<(usize, Vec3, Vec3)> {
    let wings = avatar.parts.wings.as_ref().expect("wings");
    let moved = pose
        .forward(&avatar.rig)
        .deform_mesh(&avatar.rig, &wings.membrane);
    wings
        .membrane_reach
        .iter()
        .enumerate()
        .filter(|&(_, &reach)| reach >= 0.2)
        .map(|(index, _)| (index, moved.positions[index], moved.normals[index]))
        .collect()
}

/// The nearest point to `at` on the triangle `a`, `b`, `c` (Ericson's
/// closest point on a triangle).
fn nearest_on(at: Vec3, [a, b, c]: [Vec3; 3]) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, at - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = at - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = at - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denominator = 1.0 / (va + vb + vc);
    a + ab * (vb * denominator) + ac * (vc * denominator)
}

/// How far off the skin the folded patagium stands in `pose`, the most of
/// its free points, in metres: each point's distance to the body's skin,
/// posed alike, outside it or in.
fn off_skin(avatar: &Avatar, pose: &Pose) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    let body = &avatar.parts.body;
    let moved = posed.deform(rig, &body.positions, &avatar.parts.weights);
    let triangles: Vec<[Vec3; 3]> = body
        .triangulated()
        .iter()
        .map(|tri| tri.map(|index| moved[index as usize]))
        .collect();
    free_points(avatar, pose)
        .into_iter()
        .map(|(_, at, _)| {
            triangles
                .iter()
                .map(|&tri| nearest_on(at, tri).distance_squared(at))
                .fold(f32::MAX, f32::min)
                .sqrt()
        })
        .fold(0.0, f32::max)
}

/// The poses a resting and walking glider takes: standing, walking at
/// 0.5 m/s and bounding at 1.2 m/s through the driver at 60 frames a second
/// (every third frame of a second and a half after a second's start), and
/// the curl at each quarter.
fn poses(rig: &Rig) -> Vec<(String, Pose)> {
    let mut out = vec![("rest".to_owned(), Pose::rest(rig))];
    for (name, speed) in [("walk", 0.5f32), ("bound", 1.2)] {
        for (index, pose) in driven(rig, speed).into_iter().enumerate().step_by(3) {
            out.push((format!("{name} frame {index}"), pose));
        }
    }
    let curl = gesture::for_body(rig, "Curl").expect("a glider curls");
    for quarter in 1..=4 {
        let mut pose = Pose::rest(rig);
        curl.apply(rig, &mut pose, quarter as f32 / 4.0);
        out.push((format!("curl at {}", quarter as f32 / 4.0), pose));
    }
    out
}

/// A second and a half of going at `speed`, after a second standing and a
/// second getting into its stride, at 60 frames a second.
fn driven(rig: &Rig, speed: f32) -> Vec<Pose> {
    let step = 1.0 / 60.0;
    let mut driver = Driver::seeded(7);
    let (mut at, mut time) = (Vec3::ZERO, 0.0f32);
    let mut last: Option<Vec3> = None;
    let mut out = Vec::new();
    while time < 3.5 {
        let moving = if time < 1.0 { 0.0 } else { speed };
        at += Vec3::Z * (moving * step);
        let velocity = last.map_or(Vec3::ZERO, |last| velocity_of(last, at, step));
        last = Some(at);
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta: step,
                    velocity,
                    at,
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("an unheld body is posed");
        if time >= 2.0 {
            out.push(driven.pose);
        }
        time += step;
    }
    out
}

/// How far off the skin any free point of a folded patagium may stand, as a
/// share of the back's height: a few millimetres on the default glider.
/// Measured at 0.065 at the most after #434 (a roll's curl, about 9 mm on
/// its back of 14 cm); 0.72 on 0.23.0, whose plate stood off the flank and
/// under the belly (a glider-led cross's curl), and 0.43 after #434's first
/// round (a roll bounding), whose edge spars ran out past the pleats.
const OFF_SKIN: f32 = 0.09;

/// **A glider's folded patagium lies on its flank** (#434): on every
/// glider-led body with no bat in it, standing, walking, bounding and
/// through the curl, no free point of it stands further off the skin than
/// [`OFF_SKIN`].
#[test]
fn a_gliders_folded_patagium_lies_on_its_flank() {
    let mut failures = Vec::new();
    let mut worst = (0.0f32, String::new());
    for (name, record) in glider_led() {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        // A bat in it moves its wing on the ground and in its rests as a
        // bat's (`rests::wing`), so its patagium is not folded there.
        if avatar
            .rig
            .wings
            .as_ref()
            .expect("wings")
            .kinship(&avatar.rig)
            .bat
            > 0.0
        {
            continue;
        }
        let height = back(&avatar);
        for (moment, pose) in poses(&avatar.rig) {
            let off = off_skin(&avatar, &pose) / height;
            if off > worst.0 {
                worst = (off, format!("{name} {moment}"));
            }
            if off > OFF_SKIN {
                failures.push(format!(
                    "{name} {moment}: {off:.3} of the back off the skin"
                ));
            }
        }
    }
    println!(
        "furthest off the skin: {:.3} of the back ({})",
        worst.0, worst.1
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The colour at texel `index`, `0..=1`.
fn colour(avatar: &Avatar, index: usize) -> Vec3 {
    let at = index * 4;
    let rgba = &avatar.skin.albedo[at..at + 3];
    Vec3::new(rgba[0].into(), rgba[1].into(), rgba[2].into()) / 255.0
}

fn luminance(colour: Vec3) -> f32 {
    colour.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// The mean colour of the folded patagium's outer face in `pose` - its free
/// points that face away from the trunk joint nearest them, at their texels -
/// and the mean colour of the body's skin nearest each of them at rest.
fn outer_face(avatar: &Avatar, pose: &Pose) -> (Vec3, Vec3) {
    let rig = &avatar.rig;
    let parts = &avatar.parts;
    let wings = parts.wings.as_ref().expect("wings");
    let geometry = texture::bake(&parts.body, &parts.unwrap, &[], &[], ATLAS);
    let texels: Vec<(Vec3, Vec3)> = geometry
        .texels
        .iter()
        .enumerate()
        .filter_map(|(index, texel)| texel.map(|texel| (texel.position, colour(avatar, index))))
        .collect();
    let posed = pose.forward(rig);
    let trunk = trunk_joints(rig);
    let side = avatar.skin.width as f32;
    let last = avatar.skin.width as usize - 1;
    let (mut fold, mut coat, mut count) = (Vec3::ZERO, Vec3::ZERO, 0usize);
    for (index, at, normal) in free_points(avatar, pose) {
        let joint = *trunk
            .iter()
            .min_by(|&&a, &&b| {
                posed.positions[a]
                    .distance_squared(at)
                    .total_cmp(&posed.positions[b].distance_squared(at))
            })
            .expect("a trunk");
        if normal.dot(at - posed.positions[joint]) <= 0.0 {
            continue;
        }
        let uv = wings.membrane.uvs[index];
        let (x, y) = (
            ((uv.x * side) as usize).min(last),
            ((uv.y * side) as usize).min(last),
        );
        fold += colour(avatar, y * avatar.skin.width as usize + x);
        let rest = wings.membrane.positions[index];
        coat += texels
            .iter()
            .min_by(|a, b| {
                a.0.distance_squared(rest)
                    .total_cmp(&b.0.distance_squared(rest))
            })
            .expect("texels")
            .1;
        count += 1;
    }
    assert!(count > 100, "the premise: {count} points face out");
    (fold / count as f32, coat / count as f32)
}

/// How much paler or darker than the coat beside it a folded patagium's
/// outer face may be, as a ratio of their luminances. Measured within 1.09
/// after #434 (a glider-led cross with a bird, whose coat beside the fold
/// is a little darker than its fur); it was 1.15 on the glider and up to
/// 1.24 on a glider-led cross with a bat on 0.23.0, the rim's paling over the
/// fold's whole lower half.
const COAT_RATIO: f32 = 1.12;

/// **A glider's folded patagium wears its coat outside** (#434): on every
/// glider-led body, standing and curled, the outer face of the fold is the
/// colour of the coat on the skin beside it, within [`COAT_RATIO`] in
/// luminance either way.
#[test]
fn a_gliders_folded_patagium_wears_its_coat_outside() {
    let mut failures = Vec::new();
    let mut worst = (1.0f32, String::new());
    for (name, record) in glider_led() {
        let avatar = painted(&record);
        let rig = &avatar.rig;
        let curl = gesture::for_body(rig, "Curl").expect("a glider curls");
        let mut curled = Pose::rest(rig);
        curl.apply(rig, &mut curled, 1.0);
        for (moment, pose) in [("standing", Pose::rest(rig)), ("curled", curled)] {
            let (fold, coat) = outer_face(&avatar, &pose);
            let ratio = luminance(fold) / luminance(coat).max(1e-6);
            let off = ratio.max(1.0 / ratio);
            if off > worst.0 {
                worst = (off, format!("{name} {moment}"));
            }
            if off > COAT_RATIO {
                failures.push(format!(
                    "{name} {moment}: the fold {fold} against the coat {coat}"
                ));
            }
        }
    }
    println!("furthest from the coat: {:.3} ({})", worst.0, worst.1);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far any joint of a winged body's wings may turn in a sixtieth of a
/// second walking or bounding, in radians: a beast's paw turns about 0.1, and
/// a glider's toe flipped by 2.8 inside a millimetre's travel once (#424).
/// A tenth: the most any wing joint turns here is 0.063, read on normalised
/// rotations (#458).
const FRAME_TURN: f32 = 0.1;

/// **No joint of a glider's folded wing turns over between frames** (#434):
/// walking and bounding, every joint the wings added - the folded spars and
/// their helpers - turns by less than [`FRAME_TURN`] from one frame to the
/// next in the world.
#[test]
fn no_joint_of_a_gliders_folded_wing_turns_over_between_frames() {
    let mut worst = (0.0f32, String::new());
    for (name, record) in glider_led().into_iter().take(4) {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let rig = &avatar.rig;
        let joints = rig.wings.as_ref().expect("wings").joints.clone();
        for speed in [0.5f32, 1.2] {
            let mut before: Option<Vec<Quat>> = None;
            for pose in driven(rig, speed) {
                let now = pose.forward(rig).rotations;
                if let Some(before) = &before {
                    for &joint in &joints {
                        let turned = turn_between(before[joint], now[joint]);
                        if turned > worst.0 {
                            worst = (turned, format!("{name} at {speed} m/s, joint {joint}"));
                        }
                    }
                }
                before = Some(now);
            }
        }
    }
    println!(
        "the most a wing joint turned in a frame: {:.3} rad ({})",
        worst.0, worst.1
    );
    assert!(
        worst.0 < FRAME_TURN,
        "{} turned {:.3} rad",
        worst.1,
        worst.0
    );
}

/// Every glider-led body of the first forty rolls, beside [`glider_led`]'s.
fn more_glider_led() -> Vec<(String, AvatarRecord)> {
    let mut bodies = glider_led();
    for seed in 21..=40 {
        let mut record = AvatarRecord::new("Roll", Archetype::Winged(WingedParams::default()));
        record.reroll(seed);
        let leads = matches!(&record.archetype,
            Archetype::Winged(params) if params.lineage().leading() == WingedFamily::Glider);
        if leads {
            bodies.push((format!("roll {seed}"), record));
        }
    }
    bodies
}

/// How many joints a glider-led body may carry, its wings' included: a
/// skinned mesh holds at most 256, and a glider's patagium takes a hinge per
/// joint holding the skin along its seam and a pin and a carrier per spar -
/// 100 at the most on a glider-led body after #434, against 68 on 0.23.0 and
/// 230 after #434's first round, whose pleated spars took a joint per pleat
/// and a helper at each fold.
const GLIDER_JOINTS: usize = 104;

/// **A glider carries few joints for its patagium** (#434): every glider-led
/// body - the glider, its crosses and the glider-led rolls of the first
/// forty - has at most [`GLIDER_JOINTS`].
#[test]
fn a_glider_carries_few_joints_for_its_patagium() {
    let mut most = (0usize, String::new());
    for (name, record) in more_glider_led() {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let joints = avatar.rig.len();
        if joints > most.0 {
            most = (joints, name);
        }
    }
    println!("the most joints: {} ({})", most.0, most.1);
    assert!(
        most.0 <= GLIDER_JOINTS,
        "{} carries {} joints",
        most.1,
        most.0
    );
}

/// How far from the patagium a spread glider's elbow, wrist, forefoot, knee
/// or ankle may stand, as a share of
/// the back's height: the edge runs along the limb, through it.
/// Measured at 0.008 at the most after #434 (0.031 on 0.23.0, whose edges
/// were the limbs themselves; 0.11 after #434's first round, which laid its
/// edges out before the extension held a long arm short).
const EDGE_OFF: f32 = 0.05;

/// **A spread glider's patagium runs along its legs** (#434): on every
/// glider-led body, with the wings spread, the elbow, the wrist and the
/// forefoot, the knee and the ankle each stand within [`EDGE_OFF`] of the
/// membrane - its edges laid out where the limbs reach, even where the
/// extension holds a long arm short.
#[test]
fn a_spread_gliders_patagium_runs_along_its_legs() {
    let mut failures = Vec::new();
    let mut worst = (0.0f32, String::new());
    for (name, record) in more_glider_led() {
        let avatar = Avatar::build_for_shape(&record).expect("builds");
        let rig = &avatar.rig;
        let mut pose = Pose::rest(rig);
        rig.spread_wings(&mut pose, 1.0, 1.0);
        let posed = pose.forward(rig);
        let membrane =
            posed.deform_mesh(rig, &avatar.parts.wings.as_ref().expect("wings").membrane);
        let height = back(&avatar);
        for limb in [
            Limb::ForeLeft,
            Limb::ForeRight,
            Limb::HindLeft,
            Limb::HindRight,
        ] {
            // The elbow or the knee, the wrist or the ankle, and the forefoot.
            let mut joints: Vec<usize> = rig
                .in_zone(Zone::UpperLimb(limb))
                .last()
                .into_iter()
                .copied()
                .collect();
            joints.extend(rig.in_zone(Zone::LowerLimb(limb)).first());
            if limb.is_fore() {
                joints.extend(rig.in_zone(Zone::Extremity(limb)).first());
            }
            for joint in joints {
                let at = posed.positions[joint];
                let off = membrane
                    .positions
                    .iter()
                    .map(|point| point.distance(at))
                    .fold(f32::MAX, f32::min)
                    / height;
                if off > worst.0 {
                    worst = (off, format!("{name} {limb:?} joint {joint}"));
                }
                if off > EDGE_OFF {
                    failures.push(format!(
                        "{name} {limb:?} joint {joint}: {off:.3} of the back off the membrane"
                    ));
                }
            }
        }
    }
    println!(
        "furthest from the membrane: {:.3} of the back ({})",
        worst.0, worst.1
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
