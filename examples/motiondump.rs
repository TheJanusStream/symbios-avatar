//! People and beasts driven through one scenario, a line a frame, for comparing two
//! trees' motion bit for bit: `scripts/compare-motion.sh` runs it on HEAD and on the
//! working tree, as `scripts/compare-published.sh` runs `fleetdump` for bodies at rest.
//!
//! Each body stands, walks off, quickens through a trot or a run to a gallop or a
//! sprint, walks a curve, stops, pivots where it stands and idles - through
//! [`Driver::drive`], carried by a chassis as an application carries it - and every
//! frame's pose is written as a hash of its bits, with the source the driver chose.
//!
//! ```text
//! cargo run --release --example motiondump -- OUT [--hz 60] [--winged] [--sample K] [--scenario S] [--only NAME]
//! ```
//!
//! `--scenario pivot` (or `idle`) drives only a pivot from a standstill (or only an idle);
//! `--scenario slope` the whole scenario on ground rising 15 degrees ahead and 10 to the
//! left; `--scenario gestures` each name of a person's and a beast's roster the body is
//! offered, asked standing and then told to get up;
//! `--hz` drives at another frame rate; `--sample K` writes every joint's position at
//! every K-th frame instead of the hash, for reading two frame rates against each other
//! at the moments they share (#415); `--winged` drives the winged families too.
use std::fmt::Write as _;
use symbios_avatar::anim::driver::{Driver, Inputs, level_ground, velocity_of};
use symbios_avatar::anim::{Ground, emote, gesture};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Family, QuadrupedParams, Rig, Vec3,
    WingedFamily, WingedParams,
};

/// FNV-1a over the bits of every float of a pose.
fn pose_hash(rotations: &[symbios_avatar::Quat], translation: Vec3) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |value: f32| {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for rotation in rotations {
        for value in rotation.to_array() {
            eat(value);
        }
    }
    for value in translation.to_array() {
        eat(value);
    }
    hash
}

/// The scenario at `t` seconds: the speed asked, in metres a second, and the yaw
/// rate, in radians a second.
fn scenario(name: &str, t: f32) -> (f32, f32) {
    match name {
        "pivot" => return (0.0, if (1.0..4.0).contains(&t) { 1.0 } else { 0.0 }),
        "idle" => return (0.0, 0.0),
        _ => {}
    }
    match t {
        t if t < 1.0 => (0.0, 0.0),
        t if t < 3.0 => (1.0, 0.0),
        t if t < 4.5 => (2.2, 0.0),
        t if t < 6.0 => (4.5, 0.0),
        t if t < 8.0 => (1.2, 0.8),
        t if t < 9.0 => (0.0, 0.0),
        t if t < 11.0 => (0.0, 1.0),
        _ => (0.0, 0.0),
    }
}

const SECONDS: f32 = 13.0;

fn bodies(winged: bool) -> Vec<(String, AvatarRecord)> {
    let mut out = Vec::new();
    for seed in 0..4i64 {
        let mut person = AvatarRecord::new("P", Archetype::default());
        person.reroll(seed);
        out.push((format!("person-{seed}"), person));
    }
    let lineages = [
        ("dog", Lineage::pure(Family::Dog)),
        ("cat", Lineage::pure(Family::Cat)),
        ("horse", Lineage::pure(Family::Horse)),
        ("dog-cat", Lineage::cross(Family::Dog, Family::Cat, 0.5)),
        ("cat-horse", Lineage::cross(Family::Cat, Family::Horse, 0.3)),
        ("horse-dog", Lineage::cross(Family::Horse, Family::Dog, 0.7)),
    ];
    for (name, lineage) in lineages {
        let mut params = QuadrupedParams::default();
        params.set_lineage(lineage);
        out.push((
            format!("beast-{name}"),
            AvatarRecord::new("B", Archetype::Quadruped(params)),
        ));
    }
    for seed in 0..4i64 {
        let mut beast = AvatarRecord::new("B", Archetype::Quadruped(QuadrupedParams::default()));
        beast.reroll(seed);
        out.push((format!("beast-seed{seed}"), beast));
    }
    if winged {
        let lineages = [
            ("bird", Lineage::pure(WingedFamily::Bird)),
            ("bat", Lineage::pure(WingedFamily::Bat)),
            ("glider", Lineage::pure(WingedFamily::Glider)),
            (
                "bird-bat",
                Lineage::cross(WingedFamily::Bird, WingedFamily::Bat, 0.3),
            ),
            (
                "bat-glider",
                Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.4),
            ),
        ];
        for (name, lineage) in lineages {
            let mut params = WingedParams::default();
            params.set_lineage(lineage);
            out.push((
                format!("winged-{name}"),
                AvatarRecord::new("W", Archetype::Winged(params)),
            ));
        }
    }
    out
}

/// Ground rising 15 degrees ahead and 10 to the left, through the body's origin, in the
/// frame the body is posed in.
fn sloped(point: Vec3) -> Option<Ground> {
    let normal = Vec3::new(
        -(10.0f32.to_radians().tan()),
        1.0,
        -(15.0f32.to_radians().tan()),
    )
    .normalize();
    // The height at `point` of the plane through the origin with that normal.
    let y = -(normal.x * point.x + normal.z * point.z) / normal.y;
    Some(Ground {
        position: Vec3::new(point.x, y, point.z),
        normal,
    })
}

/// Each roster name the body is offered, asked after a fifth of a second standing and
/// told to get up two and a half seconds later, four seconds each.
fn gestures(out: &mut String, label: &str, rig: &Rig, hz: f32) {
    let delta = 1.0 / hz;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a frame count"
    )]
    let frames = (4.0 * hz).round() as usize;
    for &name in gesture::ROSTER.iter().chain(emote::ROSTER) {
        if gesture::for_body(rig, name).is_none() {
            continue;
        }
        let mut driver = Driver::seeded(7);
        for frame in 0..frames {
            #[expect(clippy::cast_precision_loss, reason = "a frame count")]
            let t = frame as f32 * delta;
            let asked = if (t - 0.2).abs() < delta * 0.5 {
                Some(name)
            } else if (t - 2.7).abs() < delta * 0.5 {
                Some(emote::GET_UP)
            } else {
                None
            };
            let driven = driver.drive(
                rig,
                &Inputs {
                    delta,
                    gesture: asked,
                    ..Inputs::default()
                },
                level_ground,
            );
            let Some(driven) = driven else { continue };
            let _ = writeln!(
                out,
                "{label}/{} {frame} {:?} {:016x}",
                name.replace(' ', "-"),
                driven.source,
                pose_hash(&driven.pose.rotations, driven.pose.translation)
            );
        }
    }
}

fn drive(out: &mut String, label: &str, rig: &Rig, hz: f32, sample: Option<usize>, name: &str) {
    let delta = 1.0 / hz;
    let mut driver = Driver::seeded(7);
    let (mut at, mut facing) = (Vec3::ZERO, 0.0f32);
    let mut last: Option<Vec3> = None;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a frame count"
    )]
    let frames = (SECONDS * hz).round() as usize;
    for frame in 0..frames {
        #[expect(clippy::cast_precision_loss, reason = "a frame count")]
        let t = frame as f32 * delta;
        let (speed, turn) = scenario(name, t);
        facing += turn * delta;
        at += Vec3::new(facing.sin(), 0.0, facing.cos()) * (speed * delta);
        let velocity = last.map_or(Vec3::ZERO, |last| velocity_of(last, at, delta));
        last = Some(at);
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta,
                    velocity,
                    at,
                    facing,
                    turn,
                    ..Inputs::default()
                },
                if name == "slope" {
                    sloped
                } else {
                    level_ground
                },
            )
            .expect("an unheld body is posed");
        match sample {
            None => {
                let _ = writeln!(
                    out,
                    "{label} {frame} {:?} {:016x}",
                    driven.source,
                    pose_hash(&driven.pose.rotations, driven.pose.translation)
                );
            }
            Some(every) if frame % every == 0 => {
                let posed = driven.pose.forward(rig);
                let _ = write!(out, "{label} {t:.4} {:?}", driven.source);
                for at in &posed.positions {
                    let _ = write!(out, " {:.6} {:.6} {:.6}", at.x, at.y, at.z);
                }
                let _ = writeln!(out);
            }
            Some(_) => {}
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("an output path");
    let flag = |name: &str| {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
    };
    let hz: f32 = flag("--hz").map_or(60.0, |hz| hz.parse().expect("a rate"));
    let sample: Option<usize> = flag("--sample").map(|k| k.parse().expect("a count"));
    let only = flag("--only");
    let scenario_name = flag("--scenario").map_or("all", String::as_str);
    let winged = args.iter().any(|arg| arg == "--winged");
    let mut out = String::new();
    for (label, record) in bodies(winged) {
        if only.is_some_and(|only| !label.contains(only.as_str())) {
            continue;
        }
        let avatar = Avatar::build_with(
            &record,
            &AvatarConfig {
                atlas: 64,
                ..Default::default()
            },
        )
        .expect("builds");
        if scenario_name == "gestures" {
            gestures(&mut out, &label, &avatar.rig, hz);
        } else {
            drive(&mut out, &label, &avatar.rig, hz, sample, scenario_name);
        }
    }
    std::fs::write(path, out).expect("written");
}
