//! What keeps a winged body on four legs out of the ground it is handed
//! (`ground::keep_clear`, #440 and #441), measured against the same motion
//! without it: on the ground Overlands hands such a body - heights on a
//! quarter-metre grid, bilinear between them - and across every edge where it
//! comes in or goes out. And what the feet an emote holds do on such ground
//! (#451, #454), how a glider crossed with a bird bows on it (#451's critic),
//! a seat blended into (#452), a rest laid on it (#450) and a tail lifted
//! over its grid (#453).
//!
//! Here and not in `tests/` because the motion it is measured against is
//! the driver's own with `keep_clear` left out, which only the crate can ask
//! for (`ground::LEFT_OUT`, per thread).

use glam::{Quat, Vec3};

use crate::anim::driver::{Driver, Inputs};
use crate::anim::ground::Ground;
use crate::plan::{Blended, Lineage, WingedFamily};
use crate::{Archetype, Avatar, AvatarConfig, AvatarRecord, Flight, FlightMode, WingedParams};

use super::ground::{ELBOWS_LEFT_OUT, LEFT_OUT};

/// A winged body of `lineage`.
fn body(lineage: Lineage<WingedFamily>) -> Avatar {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    Avatar::build_with(
        &AvatarRecord::new("Winged", Archetype::Winged(params)),
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("a winged body builds")
}

/// The winged body a record rolled from `seed` draws.
fn rolled(seed: i64) -> Avatar {
    let mut record = AvatarRecord::new("Winged", Archetype::Winged(WingedParams::default()));
    record.reroll(seed);
    Avatar::build_with(
        &record,
        &AvatarConfig {
            atlas: 64,
            ..Default::default()
        },
    )
    .expect("a rolled winged body builds")
}

/// What a frame asks of the body.
#[derive(Clone, Copy, Default)]
struct Ask {
    /// Metres a second, straight ahead.
    speed: f32,
    /// Radians a second.
    turn: f32,
    gesture: Option<&'static str>,
    swimming: bool,
    flight: Option<FlightMode>,
    /// Metres a second, upward.
    rise: f32,
}

/// The ground a frame is driven over: a plane through the body's origin
/// rising by `rise` along each axis of the frame the body is posed in, or
/// the world's terrain ([`world`]) handed over as Overlands hands it
/// ([`Patch`]).
#[derive(Clone, Copy)]
enum Terrain {
    Plane(Vec3),
    Game(Vec3),
}

/// The world's height at `(x, z)` on ground rising by `rise` along each
/// world axis: bumps of a centimetre either way on a 0.8 by 1.1 m swell,
/// and a crest 0.1 m high across the way ahead, a metre and a half from
/// where the body starts.
fn world(rise: Vec3, x: f32, z: f32) -> f32 {
    use crate::det::DetMath;
    let tau = std::f32::consts::TAU;
    rise.x * x
        + rise.z * z
        + 0.01 * (tau * x / 0.8 + 0.3).det_sin() * (tau * z / 1.1).det_sin()
        + 0.1 * (-((z - 1.5) / 0.6) * ((z - 1.5) / 0.6)).det_exp()
}

/// The ground as bevy_symbios_avatar hands it over (`ground.rs`): heights a
/// quarter of a metre apart on a grid laid in the body's frame and read
/// afresh from the world every frame, bilinear between them, the normal
/// that surface's own - so its tilt jumps at every cell's edge, and the
/// edges stand still under the body as it goes.
struct Patch {
    heights: Vec<f32>,
}

/// The grid's spacing (Overlands' `GROUND_STEP`), its first sample's place
/// in the body's frame along each axis, and how many samples a side.
const STEP: f32 = 0.25;
const ORIGIN: f32 = -1.25;
const SIDE: usize = 11;

impl Patch {
    /// The grid under a body standing at `at`, facing `facing`, on ground
    /// rising by `rise`: each height against the world's under its origin.
    fn new(rise: Vec3, at: Vec3, facing: f32) -> Self {
        use crate::det::DetMath;
        let (sin, cos) = (facing.det_sin(), facing.det_cos());
        let base = world(rise, at.x, at.z);
        let mut heights = Vec::with_capacity(SIDE * SIDE);
        for row in 0..SIDE {
            for column in 0..SIDE {
                let (x, z) = (ORIGIN + column as f32 * STEP, ORIGIN + row as f32 * STEP);
                let (wx, wz) = (at.x + cos * x + sin * z, at.z - sin * x + cos * z);
                heights.push(world(rise, wx, wz) - base);
            }
        }
        Self { heights }
    }

    /// The grid under a body standing at the origin facing `+Z`, on ground
    /// whose height at `(x, z)` of its frame is `height`.
    fn of(height: impl Fn(f32, f32) -> f32) -> Self {
        let mut heights = Vec::with_capacity(SIDE * SIDE);
        for row in 0..SIDE {
            for column in 0..SIDE {
                heights.push(height(
                    ORIGIN + column as f32 * STEP,
                    ORIGIN + row as f32 * STEP,
                ));
            }
        }
        Self { heights }
    }

    /// The ground at `point`, as `bevy_symbios_avatar::Patch::ground` answers.
    fn ground(&self, point: Vec3) -> Option<Ground> {
        let last = (SIDE - 1) as f32;
        let along = |at: f32| {
            let grid = (at - ORIGIN) / STEP;
            let held = grid.clamp(0.0, last);
            let cell = held.floor().min(last - 1.0);
            (cell as usize, held - cell, (0.0..=last).contains(&grid))
        };
        let ((column, x, inside_x), (row, z, inside_z)) = (along(point.x), along(point.z));
        let at = |c: usize, r: usize| self.heights[r * SIDE + c];
        let (h00, h10, h01, h11) = (
            at(column, row),
            at(column + 1, row),
            at(column, row + 1),
            at(column + 1, row + 1),
        );
        let near = h00 + (h10 - h00) * x;
        let far = h01 + (h11 - h01) * x;
        let rise_x = if inside_x {
            ((h10 - h00) + ((h11 - h01) - (h10 - h00)) * z) / STEP
        } else {
            0.0
        };
        let rise_z = if inside_z { (far - near) / STEP } else { 0.0 };
        let normal = if rise_x == 0.0 && rise_z == 0.0 {
            Vec3::Y
        } else {
            Vec3::new(-rise_x, 1.0, -rise_z).normalize_or(Vec3::Y)
        };
        Some(Ground {
            position: Vec3::new(point.x, near + (far - near) * z, point.z),
            normal,
        })
    }
}

/// How far a joint turned from `before` to `now`, in radians, read on the
/// rotations normalised (#458): see `tests/winged_bounds.rs`'s
/// `turn_between`.
fn turn_between(before: Quat, now: Quat) -> f32 {
    now.normalize().angle_between(before.normalize())
}

/// Each frame's joints and their turns, `rig` driven at `hz` for `seconds`
/// over `terrain` as `plan` asks at each moment, with `keep_clear` or
/// `left_out`.
fn drive(
    rig: &crate::Rig,
    hz: f32,
    seconds: f32,
    terrain: Terrain,
    plan: &dyn Fn(f32) -> Ask,
    left_out: bool,
) -> Vec<(Vec<Vec3>, Vec<Quat>)> {
    drive_poses(rig, hz, seconds, terrain, plan, left_out)
        .into_iter()
        .map(|pose| {
            let posed = pose.forward(rig);
            (posed.positions, posed.rotations)
        })
        .collect()
}

/// [`drive`]'s frames as the poses the driver handed back.
fn drive_poses(
    rig: &crate::Rig,
    hz: f32,
    seconds: f32,
    terrain: Terrain,
    plan: &dyn Fn(f32) -> Ask,
    left_out: bool,
) -> Vec<crate::anim::pose::Pose> {
    use crate::det::DetMath;
    LEFT_OUT.with(|out| out.set(left_out));
    let delta = 1.0 / hz;
    let mut driver = Driver::seeded(7);
    let (mut at, mut facing) = (Vec3::ZERO, 0.0f32);
    let frames = (seconds * hz).round() as usize;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let ask = plan(frame as f32 * delta);
        facing += ask.turn * delta;
        let ahead = Vec3::new(facing.det_sin(), 0.0, facing.det_cos());
        at += ahead * (ask.speed * delta);
        let inputs = Inputs {
            delta,
            velocity: ahead * ask.speed + Vec3::Y * ask.rise,
            at,
            facing,
            turn: ask.turn,
            gesture: ask.gesture,
            swimming: ask.swimming,
            flight: ask.flight.map(Flight::new),
            ..Inputs::default()
        };
        let driven = match terrain {
            Terrain::Plane(rise) => {
                let up = Vec3::new(-rise.x, 1.0, -rise.z).normalize();
                driver.drive(rig, &inputs, move |point: Vec3| {
                    Some(Ground {
                        position: Vec3::new(point.x, rise.x * point.x + rise.z * point.z, point.z),
                        normal: up,
                    })
                })
            }
            Terrain::Game(rise) => {
                let patch = Patch::new(rise, at, facing);
                let patch = &patch;
                driver.drive(rig, &inputs, move |point: Vec3| patch.ground(point))
            }
        }
        .expect("an unheld body is posed");
        out.push(driven.pose);
    }
    LEFT_OUT.with(|out| out.set(false));
    out
}

/// How much further than without `keep_clear` any joint moves and turns in
/// a frame, at the worst frame of `with` against `without`, each read as a
/// sixtieth of a second's at `hz`, from `from` on: in metres and radians,
/// and the frames they were read at.
fn excess(
    with: &[(Vec<Vec3>, Vec<Quat>)],
    without: &[(Vec<Vec3>, Vec<Quat>)],
    hz: f32,
    from: usize,
) -> ((f32, usize), (f32, usize)) {
    let scale = hz / 60.0;
    let step = |frames: &[(Vec<Vec3>, Vec<Quat>)], at: usize| {
        let ((was, was_turned), (is, turned)) = (&frames[at - 1], &frames[at]);
        let moved = is
            .iter()
            .zip(was)
            .map(|(a, b)| a.distance(*b))
            .fold(0.0f32, f32::max);
        let turn = turned
            .iter()
            .zip(was_turned)
            .map(|(a, b)| turn_between(*b, *a))
            .fold(0.0f32, f32::max);
        (moved * scale, turn * scale)
    };
    let (mut moved, mut turned) = ((0.0f32, 0), (0.0f32, 0));
    for at in from.max(1)..with.len().min(without.len()) {
        let (a, b) = (step(with, at), step(without, at));
        if a.0 - b.0 > moved.0 {
            moved = (a.0 - b.0, at);
        }
        if a.1 - b.1 > turned.0 {
            turned = (a.1 - b.1, at);
        }
    }
    (moved, turned)
}

/// What a scene asks of the body at each moment.
type Plan = fn(f32) -> Ask;

/// The bodies on four legs measured: a bat, a Bat x Glider cross just short
/// of halfway and a bat-led cross with a bird, named both ways round.
fn bodies() -> Vec<(&'static str, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    vec![
        ("bat", Lineage::pure(Bat)),
        ("bat x glider 0.49", Lineage::cross(Bat, Glider, 0.49)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
        ("bird x bat 0.7", Lineage::cross(Bird, Bat, 0.7)),
    ]
}

/// Ground rising by `grade` a metre facing up it, down it and across it
/// both ways.
fn rises(grade: f32) -> [(&'static str, Vec3); 4] {
    [
        ("up", Vec3::Z * grade),
        ("down", -Vec3::Z * grade),
        ("across, its left the higher", Vec3::X * grade),
        ("across, its right the higher", -Vec3::X * grade),
    ]
}

/// The leg's height a body's speeds are reckoned on.
fn hip(rig: &crate::Rig) -> f32 {
    crate::anim::quadruped::hip_height(rig).expect("a body on four legs")
}

/// How much further than without `keep_clear` a joint may move in a frame
/// on the game's ground, in metres a sixtieth of a second: twice the most
/// any pace moved one (a bat bounding up it at 144 frames a second, 10.7
/// mm, the belly lifted as the bound sinks it on the crest).
const GAME_MOVE: f32 = 0.02;

/// How much further than without `keep_clear` a joint may turn in a frame
/// on the game's ground, in radians a sixtieth of a second: over the most
/// any pace turned one, 0.046 read on normalised rotations (#458; 0.086
/// read raw, a pad's rotation a few ten-thousandths off unit length).
const GAME_TURN: f32 = 0.08;

/// **What keeps a body on four out of the ground does not pop on the
/// ground the game hands it** (#441): a bat, a Bat x Glider cross at 0.49
/// and a Bat x Bird cross at 0.3, standing, walking (a Froude number of
/// 0.15 on the hip), bounding (of one) and turning on the spot at 2 rad/s,
/// at 30, 60 and 144 frames a second, on ground rising 10 degrees - ahead,
/// behind and to either side - with centimetre bumps and a crest 0.1 m
/// high, handed over as Overlands hands it ([`Patch`]): **no joint moves
/// more than [`GAME_MOVE`] or turns more than [`GAME_TURN`] further in a
/// frame than in the same motion without it**.
///
/// Before, the hands were laid by the normal at the pad alone and the
/// belly by the normal at the trunk's middle, and the bilinear surface's
/// tilt jumps at every cell's edge, which a fore pad crosses every stride:
/// a joint moved up to 121.3 mm a sixtieth more walking (a bat facing down
/// it at 144 frames a second), 127.3 bounding and 55.9 turning on the spot.
/// Each is now read over a plane through the ground around it
/// (`ground::plane_under`).
#[test]
fn keep_clear_does_not_pop_on_the_games_ground() {
    let grade = 10.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            for (on, rise) in rises(grade) {
                let failures = &failures;
                scope.spawn(move || {
                    let avatar = body(lineage);
                    let rig = &avatar.rig;
                    let at = |froude: f32| (froude * 9.81 * hip(rig)).sqrt().max(0.4);
                    let (walk, bound) = (at(0.15), at(1.0));
                    let paces: [(&str, Ask); 4] = [
                        ("standing", Ask::default()),
                        (
                            "walking",
                            Ask {
                                speed: walk,
                                ..Ask::default()
                            },
                        ),
                        (
                            "bounding",
                            Ask {
                                speed: bound,
                                ..Ask::default()
                            },
                        ),
                        (
                            "turning on the spot",
                            Ask {
                                turn: 2.0,
                                ..Ask::default()
                            },
                        ),
                    ];
                    let mut found = Vec::new();
                    for (pace, ask) in paces {
                        let plan = move |t: f32| if t < 0.5 { Ask::default() } else { ask };
                        for hz in [30.0f32, 60.0, 144.0] {
                            let terrain = Terrain::Game(rise);
                            let with = drive(rig, hz, 5.0, terrain, &plan, false);
                            let without = drive(rig, hz, 5.0, terrain, &plan, true);
                            let ((moved, at), (turned, turned_at)) = excess(&with, &without, hz, 1);
                            if moved > GAME_MOVE || turned > GAME_TURN {
                                found.push(format!(
                                    "{name} {pace}, rising 10 degrees {on}, at {hz}: a joint moved \
                                     {:.1} mm a sixtieth more than without it (frame {at}) and \
                                     turned {turned:.3} rad more (frame {turned_at})",
                                    moved * 1000.0
                                ));
                            }
                        }
                    }
                    failures.lock().expect("a lock").extend(found);
                });
            }
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much further than without `keep_clear` a joint may move and turn
/// in a frame across an edge, in metres and radians a sixtieth of a second:
/// twice the most a rest moved one (a bat facing up the slope, 10.1 mm,
/// its hands and belly eased out over three tenths of a second), and over
/// the most any edge turned one: 0.011, read on normalised rotations (#458),
/// where the raw readings' noise made it 0.084.
const EDGE: (f32, f32) = (0.02, 0.05);

/// The same through a takeoff and a landing, which carry it the whole way:
/// over the most one moved and turned a joint (17.0 mm, a bat facing up the
/// slope, and 0.325 rad, a Bat x Glider cross at 0.49, both at 144 frames a
/// second), where the hands are let go of as the body rises off the slope
/// and taken up as it comes down onto it, before #441's fixes and after.
const FLIGHT_EDGE: (f32, f32) = (0.025, 0.45);

/// **What keeps a body on four out of the ground comes in and goes out
/// without a jump** (#441): a bat, a Bat x Glider cross at 0.49 and a Bat x
/// Bird cross at 0.3, named both ways round, on a 20 degree slope - facing
/// up it, down it and across it both ways - into and out of a Head Tilt, a
/// Groom and a Shake (emotes, which keep the stance and the hold; the Groom
/// and the Shake since #454), a Rest and its Get Up (which let it go), a
/// swim (which the water floats) and a takeoff and a landing, at 30, 60 and
/// 144 frames a second: **no joint moves or turns further in
/// a frame than in the same motion without it by more than [`EDGE`]**, or
/// [`FLIGHT_EDGE`] through the flight.
///
/// Before, it was switched with the frame's source and never kept to blend
/// from, so the whole hand turn and belly lift went in one frame at either
/// edge of every emote, rest and swim: a joint moved up to 289.6 mm a
/// sixtieth more into a Head Tilt (a bat facing up the slope at 144 frames
/// a second; 114.9 at 60), 276.4 into a rest and 291.1 into a swim, and
/// turned up to 0.84 rad more, the slope's whole 20 degrees in a frame.
#[test]
fn keep_clear_comes_and_goes_without_a_jump() {
    let grade = 20.0f32.to_radians().tan();
    let scenes: [(&str, Plan); 6] = [
        ("a Head Tilt", |t| Ask {
            gesture: (1.0..1.03).contains(&t).then_some("Head Tilt"),
            ..Ask::default()
        }),
        ("a Groom", |t| Ask {
            gesture: (1.0..1.03).contains(&t).then_some("Groom"),
            ..Ask::default()
        }),
        ("a Shake", |t| Ask {
            gesture: (1.0..1.03).contains(&t).then_some("Shake"),
            ..Ask::default()
        }),
        ("a Rest and its Get Up", |t| Ask {
            gesture: if (1.0..1.03).contains(&t) {
                Some("Rest")
            } else if (4.0..4.03).contains(&t) {
                Some("Get Up")
            } else {
                None
            },
            ..Ask::default()
        }),
        ("a swim", |t| Ask {
            swimming: (1.0..3.0).contains(&t),
            ..Ask::default()
        }),
        ("a takeoff and a landing", |t| Ask {
            flight: if (1.0..2.5).contains(&t) {
                Some(FlightMode::Takeoff)
            } else if (2.5..3.0).contains(&t) {
                Some(FlightMode::Flap)
            } else if (3.0..5.0).contains(&t) {
                Some(FlightMode::Land)
            } else {
                None
            },
            speed: if (1.0..3.0).contains(&t) { 2.0 } else { 0.0 },
            ..Ask::default()
        }),
    ];
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies() {
            for (on, rise) in rises(grade) {
                let failures = &failures;
                scope.spawn(move || {
                    let avatar = body(lineage);
                    let rig = &avatar.rig;
                    let mut found = Vec::new();
                    for (scene, plan) in scenes {
                        for hz in [30.0f32, 60.0, 144.0] {
                            let terrain = Terrain::Plane(rise);
                            let with = drive(rig, hz, 6.5, terrain, &plan, false);
                            let without = drive(rig, hz, 6.5, terrain, &plan, true);
                            let ((moved, at), (turned, turned_at)) = excess(&with, &without, hz, 1);
                            let (most, most_turned) = if scene.contains("takeoff") {
                                FLIGHT_EDGE
                            } else {
                                EDGE
                            };
                            if moved > most || turned > most_turned {
                                found.push(format!(
                                    "{name} into and out of {scene}, facing {on} a 20 degree \
                                     slope, at {hz}: a joint moved {:.1} mm a sixtieth more than \
                                     without it (frame {at}) and turned {turned:.3} rad more \
                                     (frame {turned_at})",
                                    moved * 1000.0
                                ));
                            }
                        }
                    }
                    failures.lock().expect("a lock").extend(found);
                });
            }
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The lowest of everything drawn of `avatar` that can meet the ground - its
/// skin with its toes and membrane, and its flight feathers - in `pose`, over
/// `patch`'s ground straight below each point (as `tests/winged_bounds.rs`
/// reads it over a plane).
fn lowest_over(avatar: &Avatar, pose: &crate::anim::pose::Pose, patch: &Patch) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    avatar
        .meshes
        .iter()
        .filter(|mesh| mesh.kind == crate::MeshKind::Skin)
        .map(|mesh| &mesh.mesh)
        .chain(avatar.parts.wings.as_ref().map(|wings| &wings.feathers))
        .flat_map(|mesh| posed.deform_mesh(rig, mesh).positions)
        .map(|at| at.y - patch.ground(at).map_or(0.0, |below| below.position.y))
        .fold(f32::MAX, f32::min)
}

/// The lowest drawn point over `patch` ([`lowest_over`]) at the worst frame
/// of the second half of a second standing on it at 60 frames a second,
/// with `keep_clear` or `left_out`.
fn standing_low(avatar: &Avatar, patch: &Patch, left_out: bool) -> f32 {
    LEFT_OUT.with(|out| out.set(left_out));
    let mut driver = Driver::seeded(7);
    let inputs = Inputs {
        delta: 1.0 / 60.0,
        ..Inputs::default()
    };
    let mut low = f32::MAX;
    for frame in 0..60 {
        let driven = driver
            .drive(&avatar.rig, &inputs, move |point: Vec3| patch.ground(point))
            .expect("an unheld body is posed");
        if frame >= 30 {
            low = low.min(lowest_over(avatar, &driven.pose, patch));
        }
    }
    LEFT_OUT.with(|out| out.set(false));
    low
}

/// **Beside a step or over a drop, `keep_clear` lays no drawn point lower
/// than the body stands without it** (#441, the round's end review): a bat,
/// a Bat x Glider cross at 0.49 and a Bat x Bird cross at 0.3, standing a
/// second on the flat beside a step 20 cm up on its left and over a drop
/// 20 cm down ahead, each handed over as a host's grid hands a curb or a
/// deck's edge - a ramp one cell wide, beginning at the grid's first line
/// past the left forepad (2 cm past a bat's) and past the forepads: **no
/// drawn point stands more than a millimetre lower over that ground with
/// `keep_clear` than without it**.
///
/// Before, the tilt was a central difference read across both sides of the
/// hand at once, so a hand standing on the flat read half the ramp beside
/// it and was turned down into the flat: a bat beside the step stood 14.8 mm
/// under where it stood clear without `keep_clear`, Bat x Glider 0.49 18.2
/// against 2.8. Each axis takes the gentler of its two sides now
/// (`ground::plane_under`).
#[test]
fn keep_clear_lays_no_hand_into_the_flat_beside_a_step() {
    use crate::plan::{Limb, Zone};
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                // The grid's first line past the forepads, ahead.
                let fore = Limb::ALL
                    .into_iter()
                    .filter(|limb| limb.is_fore())
                    .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
                    .map(|pad| rig.joints[pad].position.z)
                    .fold(f32::MIN, f32::max);
                let ahead = ORIGIN + ((fore - ORIGIN) / STEP).ceil() * STEP;
                let scenes = [
                    (
                        "beside a step up on its left",
                        Patch::of(|x, _| if x > 0.3 { 0.2 } else { 0.0 }),
                    ),
                    (
                        "over a drop ahead",
                        Patch::of(|_, z| if z > ahead + 0.5 * STEP { -0.2 } else { 0.0 }),
                    ),
                ];
                let mut found = Vec::new();
                for (scene, patch) in &scenes {
                    let with = standing_low(&avatar, patch, false);
                    let without = standing_low(&avatar, patch, true);
                    if with < without - 0.001 {
                        found.push(format!(
                            "{name} standing {scene}: its lowest drawn point {:.1} mm over the \
                             ground with keep_clear, {:.1} without it",
                            with * 1000.0,
                            without * 1000.0
                        ));
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The bodies on four legs an emote is measured on: [`bodies`]' three bat-led
/// ones and a glider, each emote by its name in the body's roster.
fn on_four() -> Vec<(&'static str, Lineage<WingedFamily>)> {
    let mut on_four: Vec<_> = bodies().into_iter().take(3).collect();
    on_four.push(("glider", Lineage::pure(WingedFamily::Glider)));
    on_four
}

/// The most any joint moves and turns in a frame over `frames[from..to]`,
/// each read as a sixtieth of a second's at `hz`, in metres and radians,
/// and the frames they were read at.
fn fastest(
    frames: &[(Vec<Vec3>, Vec<Quat>)],
    hz: f32,
    from: usize,
    to: usize,
) -> ((f32, usize), (f32, usize)) {
    let scale = hz / 60.0;
    let (mut moved, mut turned) = ((0.0f32, 0), (0.0f32, 0));
    for at in from.max(1)..to.min(frames.len()) {
        let ((was, was_turned), (is, turns)) = (&frames[at - 1], &frames[at]);
        for (a, b) in is.iter().zip(was) {
            if a.distance(*b) * scale > moved.0 {
                moved = (a.distance(*b) * scale, at);
            }
        }
        for (a, b) in turns.iter().zip(was_turned) {
            let turn = turn_between(*b, *a) * scale;
            if turn > turned.0 {
                turned = (turn, at);
            }
        }
    }
    (moved, turned)
}

/// How much faster than on the level any joint may move and turn in a frame
/// as an emote is gone into and handed back on a slope, in metres and
/// radians a sixtieth of a second.
const HAND_BACK: (f32, f32) = (0.02, 0.1);

/// **A winged body on four legs goes into an emote and hands back to its
/// stance on a slope as it does on the level** (#451): a bat, a Bat x
/// Glider cross at 0.49, a Bat x Bird cross at 0.3 and a glider, standing,
/// through a Groom, a Shake and a stretch - each emote that holds its feet -
/// on a 20 degree
/// slope - facing up it, down it and across it both ways - as a plane and
/// as the game hands it ([`Patch`]), at 30, 60 and 144 frames a second: **no
/// joint moves or turns faster than [`HAND_BACK`] past the fastest of the
/// same emote on the level**, over its first fifth of a second and over its
/// last tenth and the three tenths after it.
///
/// Before, an emote held each foot where it rests on the rig, at the level
/// floor's height whatever the ground under it - a bat's forepads 172 mm in
/// a slope it faced up through its whole Groom - and the drawn toes kept out
/// of the ground on the stance's frames and not the emote's lifted them out
/// in the frame it ended: a joint moved 248.1 mm a sixtieth (a bat's forearm
/// facing up the slope at 60 frames a second; 615.4 at 144), and turned 3.1
/// radians (a glider's, at 144).
#[test]
fn an_emote_hands_back_to_the_stance_without_a_jump() {
    let grade = 20.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in on_four() {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let roster = super::rests::roster(rig);
                let mut grounds = Vec::new();
                for (on, rise) in rises(grade) {
                    grounds.push((
                        format!("facing {on} a 20 degree slope"),
                        Terrain::Plane(rise),
                    ));
                    grounds.push((
                        format!("facing {on} the game's 20 degree slope"),
                        Terrain::Game(rise),
                    ));
                }
                let mut found = Vec::new();
                for emote in ["Groom", "Shake", "Wing Stretch", "Stretch"] {
                    if !roster.contains(&emote) {
                        continue;
                    }
                    let plan = move |t: f32| Ask {
                        gesture: (1.0..1.03).contains(&t).then_some(emote),
                        ..Ask::default()
                    };
                    for hz in [30.0f32, 60.0, 144.0] {
                        let at = |t: f32| (t * hz).round() as usize;
                        let edges = [("going into", 1.0, 1.2), ("handing back from", 2.35, 2.8)];
                        let level = drive(rig, hz, 3.0, Terrain::Plane(Vec3::ZERO), &plan, false);
                        for (on, terrain) in &grounds {
                            let frames = drive(rig, hz, 3.0, *terrain, &plan, false);
                            for (edge, from, to) in edges {
                                let ((moved, moved_at), (turned, turned_at)) =
                                    fastest(&frames, hz, at(from), at(to));
                                let ((flat, _), (flat_turned, _)) =
                                    fastest(&level, hz, at(from), at(to));
                                if moved > flat + HAND_BACK.0 || turned > flat_turned + HAND_BACK.1
                                {
                                    found.push(format!(
                                        "{name} {edge} its {emote} {on} at {hz}: a joint moved \
                                         {:.1} mm a sixtieth (frame {moved_at}) against {:.1} on \
                                         the level, and turned {turned:.3} rad (frame \
                                         {turned_at}) against {flat_turned:.3}",
                                        moved * 1000.0,
                                        flat * 1000.0
                                    ));
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much faster than the same walk with no emote over it and the same
/// Shake standing on the level together any joint may move and turn in a
/// frame while a Shake is laid over a walk, in metres and radians a
/// sixtieth of a second: no frame turns a joint within 0.09 rad of the two
/// together, read on normalised rotations (#458).
const SHAKE_WALKING: (f32, f32) = (0.01, 0.02);

/// **A Shake laid over a walk moves no joint faster than the walk and the
/// Shake do together** (#454): a bat, a Bat x Glider cross at 0.49, a Bat x Bird cross at
/// 0.3 and a glider walking (a Froude number of 0.15 on the hip) on the
/// level and on a 20 degree slope - up it, down it and across it both
/// ways - and asked for a Shake, with what keeps a body on four out of the
/// ground and without it, at 30, 60 and 144 frames a second: **no joint
/// moves or turns faster in any frame of it than [`SHAKE_WALKING`] past the
/// fastest of the same walk without it and of the same Shake standing on
/// the level together**: the two motions laid one over the other, and
/// nothing jumping between them.
///
/// Before, the Shake held each foot where it rests on the rig, at the level
/// floor's height, and the stride's settle laid each foot it set down onto
/// the ground under it and let it go as it lifted, and the drawn toes a
/// slope had in the ground were lifted out in the frame it ended: across the
/// slope a joint of a bat moved 207.7 mm a sixtieth at 60 frames a second
/// and 596.0 at 144, against 68.1 and 73.9 for the walk and the Shake
/// together, with what keeps it out of the ground left out.
#[test]
fn a_shake_laid_over_a_walk_moves_no_joint_faster_than_the_two() {
    let failures = laid_over_a_walk("Shake", &[None], true, SHAKE_WALKING, false);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much faster than the same walk with no emote over it and the same
/// Wing Stretch standing on the level together any joint may move and turn
/// in a frame while a Wing Stretch is laid over a walk, in metres and
/// radians a sixtieth of a second: [`SHAKE_WALKING`]'s.
const STRETCH_WALKING: (f32, f32) = SHAKE_WALKING;

/// **A Wing Stretch laid over a walk moves no joint faster than the walk
/// and the stretch do together** (#460): [`laid_over_a_walk`]'s bodies that
/// have one - a bat, a Bat x Glider cross at 0.49 and a Bat x Bird cross at
/// 0.3 on four, and a bird, a Bird x Glider cross at 0.3, a Bird x Bat
/// cross at 0.7 (which the bat leads) and roll 5 (a bird) - walking at 0.3
/// m/s and at a Froude number of 0.15 on the hip, on the level, with what
/// keeps a body on four out of the ground and without it, at 30, 60 and 144
/// frames a second: **no joint moves or turns faster in any frame than
/// [`STRETCH_WALKING`] past the fastest of the same walk and the same
/// stretch standing together**. On a 20 degree slope a hand a wing lets go
/// of still rejoins the stride with a step as each wing folds (see the
/// issue): not held here; Overlands stands a bird on the level.
///
/// Before, the stride counted the legs the wing lifts as standing, sank the
/// trunk for a hand in the air and stood a foot the wing held as its step
/// came round: walking at 0.3 m/s on the level a bat moved a joint 111.8 mm
/// in a frame at 60 frames a second and 116 mm at 144 (273.8 a sixtieth),
/// against 50.3 for the stretch standing. And a bird's stretch drew back
/// the leg the stride was walking on and held the other where it stood,
/// and the stride planted each again as its step came round: a bird walking
/// at 0.3 m/s moved a wing's tip 299.7 mm in a frame and turned a hip 1.25
/// rad at 60 frames a second, roll 5 387.2 mm, a Bird x Glider cross 7.50
/// rad a sixtieth at 144, against 57.1 mm for a bird's walk and its stretch
/// together.
#[test]
fn a_wing_stretch_laid_over_a_walk_moves_no_joint_faster_than_the_two() {
    let failures = laid_over_a_walk(
        "Wing Stretch",
        &[Some(0.3), None],
        false,
        STRETCH_WALKING,
        true,
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every body of [`on_four`] - and, where `birds`, a bird, a Bird x Glider
/// cross at 0.3, a Bird x Bat cross at 0.7 and roll 5 (a bird) - whose
/// roster has `emote`, walking at each of `speeds` (`None` a Froude number
/// of 0.15 on the hip, at least 0.4 m/s) and asked for `emote` a second in,
/// on the level and, where `sloped`, on a 20 degree slope every way, with
/// `keep_clear` and without it (on four), at 30, 60 and 144 frames a
/// second: what moved or turned faster than `bound` past the same walk and
/// the same emote standing on the level together.
fn laid_over_a_walk(
    emote: &'static str,
    speeds: &[Option<f32>],
    sloped: bool,
    bound: (f32, f32),
    birds: bool,
) -> Vec<String> {
    use WingedFamily::{Bat, Bird, Glider};
    let grade = 20.0f32.to_radians().tan();
    let mut bodies: Vec<(&'static str, Avatar)> = on_four()
        .into_iter()
        .map(|(name, lineage)| (name, body(lineage)))
        .collect();
    if birds {
        bodies.extend([
            ("bird", body(Lineage::pure(Bird))),
            ("bird x glider 0.3", body(Lineage::cross(Bird, Glider, 0.3))),
            ("bird x bat 0.7", body(Lineage::cross(Bird, Bat, 0.7))),
            ("roll 5 (a bird)", rolled(5)),
        ]);
    }
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in bodies {
            let failures = &failures;
            scope.spawn(move || {
                let rig = &avatar.rig;
                if !super::rests::roster(rig).contains(&emote) {
                    return;
                }
                let standing = move |t: f32| Ask {
                    gesture: (1.0..1.03).contains(&t).then_some(emote),
                    ..Ask::default()
                };
                let mut grounds = vec![("on the level", Vec3::ZERO)];
                if sloped {
                    grounds.extend(rises(grade).map(|(on, rise)| match on {
                        "up" => ("up a 20 degree slope", rise),
                        "down" => ("down a 20 degree slope", rise),
                        "across, its left the higher" => {
                            ("across a 20 degree slope, its left the higher", rise)
                        }
                        _ => ("across a 20 degree slope, its right the higher", rise),
                    }));
                }
                let kept: &[bool] = if rig.four_legged() {
                    &[false, true]
                } else {
                    &[false]
                };
                let mut found = Vec::new();
                for &speed in speeds {
                    let leg = crate::anim::quadruped::hip_height(rig)
                        .unwrap_or_else(|| crate::anim::speed::leg_of(rig));
                    let speed = speed.unwrap_or_else(|| (0.15 * 9.81 * leg).sqrt().max(0.4));
                    let laid = move |t: f32| Ask {
                        speed,
                        gesture: (1.0..1.03).contains(&t).then_some(emote),
                        ..Ask::default()
                    };
                    let walking = move |_: f32| Ask {
                        speed,
                        ..Ask::default()
                    };
                    for &(on, rise) in &grounds {
                        for &left_out in kept {
                            for hz in [30.0f32, 60.0, 144.0] {
                                let terrain = Terrain::Plane(rise);
                                let with = drive(rig, hz, 3.0, terrain, &laid, left_out);
                                let without = drive(rig, hz, 3.0, terrain, &walking, left_out);
                                let (from, to) = ((0.9 * hz) as usize, with.len());
                                let ((moved, at), (turned, turned_at)) =
                                    fastest(&with, hz, from, to);
                                let level = Terrain::Plane(Vec3::ZERO);
                                let stood = drive(rig, hz, 3.0, level, &standing, left_out);
                                let ((walked, _), (walk_turned, _)) =
                                    fastest(&without, hz, from, to);
                                let ((still, _), (still_turned, _)) = fastest(&stood, hz, from, to);
                                let (walked, walk_turned) =
                                    (walked + still, walk_turned + still_turned);
                                if moved > walked + bound.0 || turned > walk_turned + bound.1 {
                                    found.push(format!(
                                        "{name} laying a {emote} over a walk at {speed:.2} m/s \
                                         {on}, keep_clear {}, at {hz}: a \
                                         joint moved {:.1} mm a sixtieth (frame {at}) against \
                                         the walk's and the {emote}'s {:.1}, and turned \
                                         {turned:.3} rad (frame {turned_at}) against \
                                         {walk_turned:.3}",
                                        if left_out { "left out" } else { "on" },
                                        moved * 1000.0,
                                        walked * 1000.0
                                    ));
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    failures.into_inner().expect("a lock")
}

/// Each frame's joints and their turns, `rig` standing a second on
/// `terrain` at `hz`, then seated at its rest held ([`Driver::seat`]) and
/// told to blend into it ([`Driver::blend_now`]), and driven half a second
/// more, with `keep_clear` or `left_out`.
fn seated_and_blended(
    rig: &crate::Rig,
    hz: f32,
    terrain: Terrain,
    left_out: bool,
) -> Vec<(Vec<Vec3>, Vec<Quat>)> {
    LEFT_OUT.with(|out| out.set(left_out));
    let rest = super::rests::roster(rig)
        .iter()
        .copied()
        .find(|&name| matches!(name, "Rest" | "Curl"))
        .expect("a body on four rests");
    let mut driver = Driver::seeded(7);
    let inputs = Inputs {
        delta: 1.0 / hz,
        ..Inputs::default()
    };
    let frames = (1.5 * hz).round() as usize;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        if frame == hz.round() as usize {
            driver.seat(rig, rest, 1.0);
            driver.blend_now();
        }
        let driven = match terrain {
            Terrain::Plane(rise) => {
                let up = Vec3::new(-rise.x, 1.0, -rise.z).normalize();
                driver.drive(rig, &inputs, move |point: Vec3| {
                    Some(Ground {
                        position: Vec3::new(point.x, rise.x * point.x + rise.z * point.z, point.z),
                        normal: up,
                    })
                })
            }
            Terrain::Game(rise) => {
                let patch = Patch::new(rise, Vec3::ZERO, 0.0);
                let patch = &patch;
                driver.drive(rig, &inputs, move |point: Vec3| patch.ground(point))
            }
        }
        .expect("an unheld body is posed");
        let posed = driven.pose.forward(rig);
        out.push((posed.positions, posed.rotations));
    }
    LEFT_OUT.with(|out| out.set(false));
    out
}

/// How much further than without `keep_clear` a joint of a bat facing across
/// the slope at 144 frames a second may move and turn in a frame as it is
/// seated at its rest and blended into it, in metres and radians a sixtieth
/// of a second: over the most one moved (40.1 mm on a plane, 37.3 on the
/// game's slope), not in the seat's frame but the fourteenth after it, where
/// its hand laid on the slope (`ground::lay_hands`, at two thirds of its
/// weight) moves 17 mm in a frame as the blend carries the leg into the
/// rest: `keep_clear` acting through the fast blend the seat's restored weight
/// allows, a fair trade against the 207 to 213 mm (0.75 rad) it moved in the
/// seat's frame before. Its turn is [`EDGE`]'s: read on normalised rotations
/// (#458) it turns 0.002 rad more than without it, where the raw readings
/// made it 0.183 and 0.108. Every other body, slope and rate is within
/// [`EDGE`] (at most 10.7 mm and 0.003 rad).
const SEAT_EDGE: (f32, f32) = (0.045, 0.05);

/// **A body seated at a rest and blended into it eases what keeps it out of
/// the ground with the pose** (#452): a bat, a Bat x Glider cross at 0.49 and
/// a Bat x Bird cross at 0.3, standing a second on a 20 degree slope - facing
/// up it, down it and across it both ways, as a plane and as the game hands
/// it - then seated at its rest held and told to blend, as a host leaving a
/// scrub does, at 30, 60 and 144 frames a second: **no joint moves or turns
/// further in a frame than in the same motion without it by more than
/// [`EDGE`]**, or [`SEAT_EDGE`] for a bat facing across the slope at 144.
///
/// Before, the seat set the rest's weight at once - none - while the blend
/// eased the pose from the frame last drawn, so the hands and belly the
/// slope had turned and lifted went in the seat's frame: a joint moved up to
/// 92.9 mm a sixtieth more than without it at 60 frames a second and 305.0
/// at 144 (a bat facing up the game's slope), and turned 0.89 rad more
/// (Bat x Bird 0.3 there, at 144).
#[test]
fn a_seat_blended_into_eases_keep_clear_with_the_pose() {
    let grade = 20.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let mut found = Vec::new();
                for (on, rise) in rises(grade) {
                    for (ground, terrain) in [
                        ("a plane", Terrain::Plane(rise)),
                        ("the game's", Terrain::Game(rise)),
                    ] {
                        for hz in [30.0f32, 60.0, 144.0] {
                            let with = seated_and_blended(rig, hz, terrain, false);
                            let without = seated_and_blended(rig, hz, terrain, true);
                            let ((moved, at), (turned, turned_at)) = excess(&with, &without, hz, 1);
                            let (most, most_turned) =
                                if name == "bat" && on.starts_with("across") && hz == 144.0 {
                                    SEAT_EDGE
                                } else {
                                    EDGE
                                };
                            if moved > most || turned > most_turned {
                                found.push(format!(
                                    "{name} seated and blended facing {on} a 20 degree slope \
                                     ({ground}) at {hz}: a joint moved {:.1} mm a sixtieth more \
                                     than without it (frame {at}) and turned {turned:.3} rad more \
                                     (frame {turned_at})",
                                    moved * 1000.0
                                ));
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The glider-led crosses with a bird the bow is measured on: Glider x Bird
/// at 0.2 to 0.45, and the same bodies named the other way round.
fn bowing() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bird, Glider};
    let mut bowing = Vec::new();
    for share in [0.2f32, 0.25, 0.3, 0.35, 0.4, 0.45] {
        bowing.push((
            format!("Glider x Bird {share}"),
            Lineage::cross(Glider, Bird, share),
        ));
        bowing.push((
            format!("Bird x Glider {}", 1.0 - share),
            Lineage::cross(Bird, Glider, 1.0 - share),
        ));
    }
    bowing
}

/// **A glider-led cross with a bird bows on a slope as smoothly as a glider
/// does** (#451's critic): Glider x Bird crosses at 0.2, 0.25, 0.3, 0.35,
/// 0.4 and 0.45, and the same bodies named the other way round, standing
/// through a Stretch on the level and on a 20 degree slope - facing up it,
/// down it and across it both ways - as a plane and as the game hands it
/// ([`Patch`]), at 30, 60 and 144 frames a second: **over the whole of it,
/// going into the bow, holding it and coming back, no joint moves or turns
/// faster than [`HAND_BACK`] past the fastest of a glider's own Stretch on
/// the same ground**. Against the glider on its own ground and not on the
/// level, since a slope speeds a glider's bow too (0.13 rad a sixtieth
/// facing down it, against 0.06 on the level), and a bird's share turns the
/// head up in a quicker saccade facing up one.
///
/// Before, a cross bowed as deep as a glider on a shorter foreleg standing
/// higher, folding it nearly shut (`rests::bow_sink`), and its elbow went
/// from bent back to bent out in a frame as the bow arrived or left: a fore
/// upper limb turned 1.15 rad a sixtieth across the game's slope at 60
/// frames a second (Glider x Bird 0.3) and 1.71 at 144 (0.35), and 0.11 on
/// the level, against the glider's 0.07 and 0.06.
#[test]
fn a_glider_crossed_with_a_bird_bows_as_smoothly_as_a_glider() {
    let failures = bows_past_a_glider(&[20.0], HAND_BACK);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much faster than a glider's own Stretch on the same ground any joint
/// of a glider-led cross with a bird may move and turn in a frame of its bow
/// on slopes of 25 and 30 degrees, in metres and radians a sixtieth of a
/// second: over the most any moves and turns past it there (2.2 mm and 0.109
/// rad, a Glider x Bird 0.45 facing up a 30 degree slope at 144 frames a
/// second, its head's saccade up the slope as the bow begins).
const STEEP_BOW: (f32, f32) = (0.02, 0.15);

/// **And on slopes steeper than 20 degrees, with no flick** (#455): the same
/// crosses, Stretch and grounds as
/// [`a_glider_crossed_with_a_bird_bows_as_smoothly_as_a_glider`] but on slopes
/// of 25 and 30 degrees, at 30, 60 and 144 frames a second: **no joint moves
/// or turns faster than [`STEEP_BOW`] past the fastest of a glider's own
/// Stretch on the same ground**.
///
/// Before, the forepaws the bow walks out ahead were held at the height they
/// stood at, whatever the ground ahead: facing down a slope they reached out
/// over it and the foreleg folded nearly shut, its shoulder-to-wrist line
/// swinging round to stand opposite the way the elbow bends, where the solve's
/// bend plane is undetermined (`Rig::bend_pole`) - a Glider x Bird 0.45 facing
/// down a 25 degree plane turned a joint 0.58 rad a sixtieth at 144 frames a
/// second against the glider's 0.17, and on 30 degrees a Glider x Bird 0.3
/// flipped one 7.5 rad (92 of the sweep's lines past this bound). Walked out
/// along the ground under them (`Clip::apply_resting`, #457), the paws come
/// down the slope with the bow and the foreleg stays open.
#[test]
fn a_glider_crossed_with_a_bird_bows_without_a_flick_on_steeper_slopes() {
    let failures = bows_past_a_glider(&[25.0, 30.0], STEEP_BOW);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every frame of a glider-led cross's Stretch ([`bowing`]) faster than
/// `bound` past the fastest of a glider's own on the same ground: on slopes
/// of each of `grades` degrees, facing up, down and across them both ways,
/// as a plane and as the game hands it ([`Patch`]), and on the level, at 30,
/// 60 and 144 frames a second, over the whole bow - one line each.
fn bows_past_a_glider(grades: &[f32], bound: (f32, f32)) -> Vec<String> {
    let plan = |t: f32| Ask {
        gesture: (1.0..1.03).contains(&t).then_some("Stretch"),
        ..Ask::default()
    };
    let mut grounds = vec![("on the level".to_owned(), Terrain::Plane(Vec3::ZERO))];
    for &degrees in grades {
        for (on, rise) in rises(degrees.to_radians().tan()) {
            grounds.push((
                format!("facing {on} a {degrees} degree slope"),
                Terrain::Plane(rise),
            ));
            grounds.push((
                format!("facing {on} the game's {degrees} degree slope"),
                Terrain::Game(rise),
            ));
        }
    }
    let rates = [30.0f32, 60.0, 144.0];
    let window = |hz: f32| ((1.0 * hz).round() as usize, (2.9 * hz).round() as usize);
    let glider = body(Lineage::pure(WingedFamily::Glider));
    // The glider's fastest move and turn on each ground, at each rate.
    let own: Vec<Vec<_>> = grounds
        .iter()
        .map(|(_, terrain)| {
            rates
                .iter()
                .map(|&hz| {
                    let (from, to) = window(hz);
                    fastest(
                        &drive(&glider.rig, hz, 3.0, *terrain, &plan, false),
                        hz,
                        from,
                        to,
                    )
                })
                .collect()
        })
        .collect();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bowing() {
            let (failures, grounds, own) = (&failures, &grounds, &own);
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                assert!(
                    super::rests::roster(rig).contains(&"Stretch"),
                    "{name} bows as a glider"
                );
                let mut found = Vec::new();
                for ((on, terrain), own) in grounds.iter().zip(own) {
                    for (&hz, &((flat, _), (flat_turned, _))) in rates.iter().zip(own) {
                        let (from, to) = window(hz);
                        let frames = drive(rig, hz, 3.0, *terrain, &plan, false);
                        let ((moved, moved_at), (turned, turned_at)) =
                            fastest(&frames, hz, from, to);
                        if moved > flat + bound.0 || turned > flat_turned + bound.1 {
                            found.push(format!(
                                "{name} bowing {on} at {hz}: a joint moved {:.1} mm a sixtieth \
                                 (frame {moved_at}) against a glider's {:.1}, and turned \
                                 {turned:.3} rad (frame {turned_at}) against {flat_turned:.3}",
                                moved * 1000.0,
                                flat * 1000.0
                            ));
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    failures.into_inner().expect("a lock")
}

/// The lowest of everything drawn of `avatar` that can meet the ground in
/// `pose` ([`lowest_over`]) over `terrain` under a body that stands where it
/// started, at the origin facing `+Z`.
fn lowest_on(avatar: &Avatar, pose: &crate::anim::pose::Pose, terrain: Terrain) -> f32 {
    match terrain {
        Terrain::Plane(rise) => {
            let posed = pose.forward(&avatar.rig);
            avatar
                .meshes
                .iter()
                .filter(|mesh| mesh.kind == crate::MeshKind::Skin)
                .map(|mesh| &mesh.mesh)
                .chain(avatar.parts.wings.as_ref().map(|wings| &wings.feathers))
                .flat_map(|mesh| posed.deform_mesh(&avatar.rig, mesh).positions)
                .map(|at| at.y - (rise.x * at.x + rise.z * at.z))
                .fold(f32::MAX, f32::min)
        }
        Terrain::Game(rise) => lowest_over(avatar, pose, &Patch::new(rise, Vec3::ZERO, 0.0)),
    }
}

/// How much lower than on the level a resting body's lowest drawn point may
/// stand over a slope, and how much faster any of its joints may move and
/// turn in a frame, in metres and radians a sixtieth of a second.
const REST_SLOPE: (f32, f32, f32) = (0.006, 0.02, 0.1);

/// **A winged body on four legs rests on a slope as it does on the level**
/// (#450): a bat, a Bat x Glider cross at 0.49, a Bat x Bird cross at 0.3, a
/// glider and a Glider x Bat cross at the halfway blend, standing, lying down
/// to rest (a bat's Rest or a glider's Curl), held there and getting up, on
/// a 20 degree slope - facing up it, down it and across it both ways - as a
/// plane and as the game hands it ([`Patch`]), at 30, 60 and 144 frames a
/// second: **no drawn point stands more than [`REST_SLOPE`]'s first lower
/// over the ground than the same rest's lowest on the level** (read at 60),
/// **and no joint moves or turns faster in a frame than its second and third
/// past the fastest of the same rest on the level and of the same body
/// standing on the same ground** - whose drawn toes and pads a slope stirs
/// more than the level, a bat's 0.03 rad a frame at any rate against 0.01.
///
/// Before, a winged rest was solved against the level floor whatever the
/// ground under it: a bat resting facing up the slope put its forearm 222 mm
/// into it (238 on the game's; a Bat x Bird cross 232 and 249, a Glider x Bat
/// cross 208 and 218), facing down it 70, across it 93, and in the frame its
/// getting up handed back to the stance, standing on the slope, a joint
/// moved 250 mm a sixtieth at 60 frames a second and 634 at 144. Laid on the
/// slope, no drawn point stands more than 5.0 mm lower than on the level (a
/// bat across the game's slope, 5.1 mm into it against 0.1), and no joint
/// moves more than 8.9 mm a sixtieth (a Bat x Bird cross across the game's
/// slope at 144) or turns more than 0.06 rad faster than the same body
/// resting on the level or standing there.
#[test]
fn a_winged_rest_lies_on_the_slope_under_it() {
    let grade = 20.0f32.to_radians().tan();
    let mut bodies = on_four();
    bodies.push((
        "glider x bat 0.5",
        Lineage::cross(WingedFamily::Glider, WingedFamily::Bat, 0.5),
    ));
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let rest = if super::rests::roster(rig).contains(&"Rest") {
                    "Rest"
                } else {
                    "Curl"
                };
                let plan = move |t: f32| Ask {
                    gesture: if (0.5..0.53).contains(&t) {
                        Some(rest)
                    } else if (4.0..4.03).contains(&t) {
                        Some(crate::anim::emote::GET_UP)
                    } else {
                        None
                    },
                    ..Ask::default()
                };
                let mut found = Vec::new();
                for hz in [30.0f32, 60.0, 144.0] {
                    let level = Terrain::Plane(Vec3::ZERO);
                    let flat = drive_poses(rig, hz, 6.0, level, &plan, false);
                    let posed = |poses: &[crate::anim::pose::Pose]| -> Vec<(Vec<Vec3>, Vec<Quat>)> {
                        poses
                            .iter()
                            .map(|pose| {
                                let posed = pose.forward(rig);
                                (posed.positions, posed.rotations)
                            })
                            .collect()
                    };
                    let every = (hz / 20.0).round() as usize;
                    let lowest = |poses: &[crate::anim::pose::Pose], terrain: Terrain| {
                        poses
                            .iter()
                            .step_by(every)
                            .map(|pose| lowest_on(&avatar, pose, terrain))
                            .fold(f32::MAX, f32::min)
                    };
                    let ((most, _), (most_turned, _)) = fastest(&posed(&flat), hz, 1, flat.len());
                    let low = (hz == 60.0).then(|| lowest(&flat, level));
                    for (on, rise) in rises(grade) {
                        for (ground, terrain) in [
                            ("a 20 degree slope", Terrain::Plane(rise)),
                            ("the game's 20 degree slope", Terrain::Game(rise)),
                        ] {
                            let poses = drive_poses(rig, hz, 6.0, terrain, &plan, false);
                            let ((moved, moved_at), (turned, turned_at)) =
                                fastest(&posed(&poses), hz, 1, poses.len());
                            // And the same body standing there, whose own toes
                            // and pads a slope stirs more than the level does.
                            let standing = drive(rig, hz, 6.0, terrain, &|_| Ask::default(), false);
                            let ((stands, _), (stands_turned, _)) =
                                fastest(&standing, hz, 1, standing.len());
                            let (most, most_turned) =
                                (most.max(stands), most_turned.max(stands_turned));
                            if std::env::var("W22_DIAG").is_ok() {
                                println!(
                                    "{name} {rest} facing {on} {ground} at {hz}: moved {:.1} \
                                     ({:.1}), turned {turned:.3} ({most_turned:.3}){}",
                                    moved * 1000.0,
                                    most * 1000.0,
                                    low.map_or(String::new(), |low| format!(
                                        ", lowest {:.1} ({:.1} level)",
                                        lowest(&poses, terrain) * 1000.0,
                                        low * 1000.0
                                    ))
                                );
                            }
                            if moved > most + REST_SLOPE.1 || turned > most_turned + REST_SLOPE.2 {
                                found.push(format!(
                                    "{name} resting facing {on} {ground} at {hz}: a joint moved \
                                     {:.1} mm a sixtieth (frame {moved_at}) against {:.1} resting \
                                     on the level or standing there, and turned {turned:.3} rad \
                                     (frame {turned_at}) against {most_turned:.3}",
                                    moved * 1000.0,
                                    most * 1000.0
                                ));
                            }
                            if let Some(low) = low {
                                let sloped = lowest(&poses, terrain);
                                if sloped < low - REST_SLOPE.0 {
                                    found.push(format!(
                                        "{name} resting facing {on} {ground}: a drawn point \
                                         {:.1} mm over the ground, {:.1} on the level",
                                        sloped * 1000.0,
                                        low * 1000.0
                                    ));
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **A tail lifted out of the game's ground is lifted continuously as it
/// goes over it** (#453): a Bat x Glider cross at 0.49 and a glider, each
/// standing at rest over the game's ground rising 20 degrees behind it - a
/// bilinear grid ([`Patch`]), whose tilt jumps at every cell's edge -
/// carried back over one cell in 200 even steps and in 400, and its tail
/// lifted out of the ground by `keep_clear` at each: **the largest step any
/// of its tail's joints takes about the tail's first joint at least nearly
/// halves at twice the sampling**, as a motion's does and a jump's does not.
///
/// Before, each joint read the ground at one point, its tilt the cell's own,
/// so the lift stepped at each cell's edge: the largest step stayed where it
/// was at twice the sampling (a Bat x Glider cross's 2.19 mm and 2.20),
/// where read over a plane round each joint (`ground::plane_under`) it
/// halves (0.47 and 0.23). A glider's lift halves either way here (0.45 and
/// 0.23), and is held to it.
#[test]
fn a_tail_is_lifted_over_the_games_ground_without_a_step() {
    let ground = Patch::new(-Vec3::Z * 20.0f32.to_radians().tan(), Vec3::ZERO, 0.0);
    let mut failures = Vec::new();
    for (name, lineage) in [
        (
            "bat x glider 0.49",
            Lineage::cross(WingedFamily::Bat, WingedFamily::Glider, 0.49),
        ),
        ("glider", Lineage::pure(WingedFamily::Glider)),
    ] {
        let avatar = body(lineage);
        let rig = &avatar.rig;
        let tail = rig.tail();
        assert!(tail.len() > 2, "{name} has a tail");
        // The tail about its first joint, carried `back` metres back.
        let lifted = |back: f32| {
            let mut pose = crate::anim::pose::Pose::rest(rig);
            pose.translation.z -= back;
            super::ground::keep_clear(
                rig,
                &mut pose,
                &|point| ground.ground(point),
                1.0,
                (0.0, &[]),
            );
            let posed = pose.forward(rig);
            tail.iter()
                .map(|&joint| posed.positions[joint] - posed.positions[tail[0]])
                .collect::<Vec<_>>()
        };
        let worst = |steps: usize| {
            let mut was = lifted(0.0);
            let mut worst = 0.0f32;
            for step in 1..=steps {
                let now = lifted(STEP * step as f32 / steps as f32);
                for (a, b) in was.iter().zip(&now) {
                    worst = worst.max(a.distance(*b));
                }
                was = now;
            }
            worst
        };
        let (coarse, fine) = (worst(200), worst(400));
        if std::env::var("W22_DIAG").is_ok() {
            println!(
                "{name}: {:.3} mm and {:.3} mm",
                coarse * 1000.0,
                fine * 1000.0
            );
        }
        if coarse <= 0.0 || fine > 0.6 * coarse {
            failures.push(format!(
                "{name}: the tail stepped {:.3} mm and then {:.3} mm at twice the sampling - a \
                 step that does not shrink as the sampling doubles is a jump",
                coarse * 1000.0,
                fine * 1000.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much lower than on the level - or than the same body standing on the
/// same ground, whichever is lower - an emote's lowest drawn point may stand
/// over a slope, in metres: [`REST_SLOPE`]'s.
const EMOTE_SLOPE: f32 = REST_SLOPE.0;

/// **An emote of a body on four reaches no further into a slope than it does
/// on the level** (#457): a bat, a Bat x Glider cross at 0.49 and one at the
/// halfway blend (which a glider leads), a Bat x Bird cross at 0.3, a glider
/// and a Glider x Bird cross at 0.3, standing, through every emote of its
/// roster, on a 20 degree slope - facing up it, down it and across it both
/// ways - as a plane and as the game hands it ([`Patch`]), at 30, 60 and 144
/// frames a second: **no drawn point stands more than [`EMOTE_SLOPE`] lower
/// over the ground, from the frame it is asked to two seconds on, than the
/// same emote's lowest on the level or the same body standing on the same
/// ground**, read twenty times a second.
///
/// Before, an emote was laid out over the level whatever the ground under it:
/// a bat's Wing Stretch put its hind toes 57.1 mm into the game's slope it
/// faced up (the wing spreading the leg its stance had laid along the slope)
/// and its thumb 27.1 mm into one rising on its side, a Bat x Glider cross's
/// 40.3 and 22.9; a glider's Stretch walked its forepaws out level, 12.4 mm
/// into a slope it faced up, the halfway cross's 27.1; and the feet every
/// emote holds were stood upright off the slope, a rolled glider's stifle 7.6
/// mm in as it groomed or shook.
#[test]
fn an_emote_reaches_no_further_into_a_slope_than_on_the_level() {
    use WingedFamily::{Bat, Bird, Glider};
    let grade = 20.0f32.to_radians().tan();
    let bodies = [
        ("bat", Lineage::pure(Bat)),
        ("bat x glider 0.49", Lineage::cross(Bat, Glider, 0.49)),
        ("bat x glider 0.5", Lineage::cross(Bat, Glider, 0.5)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
        ("glider", Lineage::pure(Glider)),
        ("glider x bird 0.3", Lineage::cross(Glider, Bird, 0.3)),
    ];
    let bodies = bodies
        .into_iter()
        .map(|(name, lineage)| (name, body(lineage)))
        .chain(std::iter::once(("roll 9 (a glider)", rolled(9))));
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in bodies {
            let failures = &failures;
            scope.spawn(move || {
                let rig = &avatar.rig;
                let mut found = Vec::new();
                for &emote in super::rests::roster(rig) {
                    if super::rests::seconds(rig, emote).is_some()
                        || emote == crate::anim::emote::GET_UP
                    {
                        continue;
                    }
                    let plan = move |t: f32| Ask {
                        gesture: (1.0..1.03).contains(&t).then_some(emote),
                        ..Ask::default()
                    };
                    for hz in [30.0f32, 60.0, 144.0] {
                        let (from, every) =
                            ((1.0 * hz).round() as usize, (hz / 20.0).round() as usize);
                        let lowest = |terrain: Terrain, plan: &dyn Fn(f32) -> Ask| {
                            drive_poses(rig, hz, 3.0, terrain, plan, false)[from..]
                                .iter()
                                .step_by(every)
                                .map(|pose| lowest_on(&avatar, pose, terrain))
                                .fold(f32::MAX, f32::min)
                        };
                        let level = lowest(Terrain::Plane(Vec3::ZERO), &plan);
                        for (on, rise) in rises(grade) {
                            for (ground, terrain) in [
                                ("a 20 degree slope", Terrain::Plane(rise)),
                                ("the game's 20 degree slope", Terrain::Game(rise)),
                            ] {
                                let low = lowest(terrain, &plan);
                                let standing = lowest(terrain, &|_| Ask::default());
                                if low < level.min(standing) - EMOTE_SLOPE {
                                    found.push(format!(
                                        "{name}'s {emote} facing {on} {ground} at {hz}: a drawn \
                                         point {:.1} mm over the ground, {:.1} on the level and \
                                         {:.1} standing there",
                                        low * 1000.0,
                                        level * 1000.0,
                                        standing * 1000.0
                                    ));
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How far from unit length any rotation the driver hands back for a
/// crawling body may drift, in its frame as posed.
const UNIT_DRIFT: f32 = 1.5e-4;

/// **What a crawling body's stance turns, it turns by rotations of unit
/// length** (#458): a bat, a Bat x Glider cross at 0.49 and a Bat x Bird
/// cross at 0.3, standing, walking (a Froude number of 0.15 on the hip) and
/// turning on the spot at 2 rad/s for two seconds at 60 frames a second, on
/// the level and on a 20 degree slope - facing up it, down it and across it
/// both ways - as a plane and as the game hands it ([`Patch`]): **no joint's
/// rotation, as posed, drifts more than [`UNIT_DRIFT`] from unit length**.
///
/// Before, what lays a winged body's feet, hands, belly and tail on the
/// ground turned each joint from the frame as posed back into its parent's by
/// that parent's rotation inverted as if it were of unit length, which, a
/// product down the leg, it is not quite, and wrote the result unnormalised:
/// a bat's forepad drifted 7.4e-4 from unit length turning on the spot facing
/// up the slope (a Bat x Bird cross's 5.8e-4, a Bat x Glider cross's 4.7e-4
/// walking up it), which `Quat::angle_between` read as a turn of up to 0.07
/// rad a frame that alternated. Each such turn is normalised where it is
/// written now (`ground::lay_foot` and its neighbours); what is left, under
/// 8e-5, is the leg solve's, which every body shares.
#[test]
fn a_crawling_bodys_rotations_stay_of_unit_length() {
    let grade = 20.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let speed = (0.15 * 9.81 * hip(rig)).sqrt().max(0.4);
                let walking = move |_: f32| Ask {
                    speed,
                    ..Ask::default()
                };
                let turning = |_: f32| Ask {
                    turn: 2.0,
                    ..Ask::default()
                };
                let scenes: [(&str, &dyn Fn(f32) -> Ask); 3] = [
                    ("standing", &|_| Ask::default()),
                    ("walking", &walking),
                    ("turning", &turning),
                ];
                let mut grounds = vec![("on the level", Vec3::ZERO)];
                grounds.extend(rises(grade));
                let mut found = Vec::new();
                for (scene, plan) in scenes {
                    for &(on, rise) in &grounds {
                        for terrain in [Terrain::Plane(rise), Terrain::Game(rise)] {
                            let worst = drive(rig, 60.0, 2.0, terrain, plan, false)
                                .iter()
                                .flat_map(|(_, turns)| turns.iter())
                                .map(|turn| (turn.length() - 1.0).abs())
                                .fold(0.0f32, f32::max);
                            if worst > UNIT_DRIFT {
                                found.push(format!(
                                    "{name} {scene} facing {on}: a rotation {worst:.2e} off unit \
                                     length"
                                ));
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// When a Wing Stretch is cut short in [`a_takeoff_cuts_a_wing_stretch_short_without_a_jump`]
/// and [`a_swim_or_a_fall_ends_a_winged_gesture`], in seconds from the
/// start: as its left wing is held out, and as its right one is.
const CUTS: [f32; 2] = [1.40, 2.15];

/// A Wing Stretch asked a second in, if `stretch`, and from `cut` on a
/// takeoff, a swim or a fall of three metres a second for four tenths of a
/// second, as `how` names.
fn cut_short(stretch: bool, cut: f32, how: &'static str) -> impl Fn(f32) -> Ask {
    move |t: f32| Ask {
        gesture: (stretch && (1.0..1.03).contains(&t)).then_some("Wing Stretch"),
        flight: (how == "a takeoff" && t >= cut).then_some(FlightMode::Takeoff),
        swimming: how == "a swim" && t >= cut,
        rise: if how == "a fall" && (cut..cut + 0.4).contains(&t) {
            -3.0
        } else {
            0.0
        },
        ..Ask::default()
    }
}

/// **A takeoff that cuts a Wing Stretch short lets go of the hand the wing
/// had taken over two blends, never in a frame** (#457's critic): a bat, a
/// Bat x Glider cross at 0.49 and a Bat x Bird cross at 0.3 on a 20 degree
/// slope - facing up it, down it and across it both ways - as a plane and as
/// the game hands it ([`Patch`]), asked for a Wing Stretch and taking off
/// with either wing held out ([`CUTS`]), at 30, 60 and 144 frames a second:
/// **from the frame before the takeoff to half a second on, no joint moves
/// or turns further in a frame than in the same motion without `keep_clear`
/// by more than [`FLIGHT_EDGE`]**, the bound a takeoff from standing keeps.
///
/// Before, `keep_clear` let go of the hand by the share the clip playing
/// had taken it, read afresh every frame, and the takeoff ends the clip: the
/// spread hand was turned back onto the slope and laid there in the frame
/// the body took off, a joint 395.5 mm a sixtieth further than without it (a
/// bat across the slope at 144 frames a second, its right wing out; 134.3 at
/// 60, 45.1 at 30) and 0.849 rad (a Bat x Bird cross facing up the game's
/// slope). Now the driver eases out of what a gesture cut short had taken
/// (`Driver::take_wings`): at most 20.0 mm (a bat facing up the game's slope
/// at 144) and 0.354 rad (a Bat x Glider cross there, as much as its
/// takeoff from standing turns one: the flight's own).
#[test]
fn a_takeoff_cuts_a_wing_stretch_short_without_a_jump() {
    let grade = 20.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let mut found = Vec::new();
                for (on, rise) in rises(grade) {
                    for (ground, terrain) in [
                        ("a 20 degree slope", Terrain::Plane(rise)),
                        ("the game's 20 degree slope", Terrain::Game(rise)),
                    ] {
                        for hz in [30.0f32, 60.0, 144.0] {
                            for cut in CUTS {
                                let plan = cut_short(true, cut, "a takeoff");
                                let seconds = cut + 0.5;
                                let with = drive(rig, hz, seconds, terrain, &plan, false);
                                let without = drive(rig, hz, seconds, terrain, &plan, true);
                                let from = (cut * hz).round() as usize - 1;
                                let ((moved, at), (turned, turned_at)) =
                                    excess(&with, &without, hz, from);
                                if moved > FLIGHT_EDGE.0 || turned > FLIGHT_EDGE.1 {
                                    found.push(format!(
                                        "{name} taking off {cut} s into a Wing Stretch facing \
                                         {on} {ground} at {hz}: a joint moved {:.1} mm a \
                                         sixtieth more than without keep_clear (frame {at}) and \
                                         turned {turned:.3} rad more (frame {turned_at})",
                                        moved * 1000.0
                                    ));
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much faster than the same swim from standing any joint may move and
/// turn in a frame as a swim cuts a Wing Stretch short, in metres and
/// radians a sixtieth of a second: over the most one moved and turned, 27.1
/// mm (a bat across the game's slope at 144 frames a second, the wing folding
/// as the body blends into the swim) and 0.039 rad.
const SWUM_INTO: (f32, f32) = (0.04, 0.06);

/// The same as a fall cuts it short, against the same fall from standing:
/// over the most one moved and turned, 147.1 mm (a bat across the game's
/// slope at 144, the wing folding over the blend into the leap, 143.8 at 60
/// and 131.3 at 30) and 0.264 rad (a bat facing up it at 144, 0.256 at 60
/// and 0.233 at 30: the blend's own speed, not a step). Since #456 a winged
/// body's fall is joined at the end of its arc, nose down and its legs
/// reaching, and the blend from a spread wing to it turns faster than to
/// the jump's start it was joined at (141.0 mm and 0.182 rad before).
const FALLEN_INTO: (f32, f32) = (0.2, 0.3);

/// **A swim or a fall ends a winged body's gesture, as a flight does**
/// (#461): a bat, a Bat x Glider cross at 0.49 and a Bat x Bird cross at 0.3
/// on a 20 degree slope - facing up it, down it and across it both ways - as
/// a plane and as the game hands it ([`Patch`]), asked for a Wing Stretch and
/// swimming or falling with either wing held out ([`CUTS`]), at 30, 60 and
/// 144 frames a second: **from the frame before to a second on, no joint
/// moves or turns faster in a frame than in the same swim or fall from
/// standing by more than [`SWUM_INTO`] or [`FALLEN_INTO`]**; and on the level
/// no gesture plays from the swim's or the fall's first frame, nor starts
/// while either lasts.
///
/// Before, the gesture played on over the swim and the leap: the source
/// stayed the gesture's, so the swim or the leap came in under it with no
/// blend, the feet it holds held to the floor and the hands kept on it, and
/// a joint moved 1315.3 mm a sixtieth faster than swimming from standing (a
/// bat facing down the slope at 144 frames a second; 538.9 at 60, 256.0 at
/// 30) and turned 7.205 rad (a Bat x Bird cross there), and falling 623.0 mm
/// (a bat facing up the game's slope at 144) and 6.671 rad.
#[test]
fn a_swim_or_a_fall_ends_a_winged_gesture() {
    let grade = 20.0f32.to_radians().tan();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies().into_iter().take(3) {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let mut found = Vec::new();
                for (how, bound) in [("a swim", SWUM_INTO), ("a fall", FALLEN_INTO)] {
                    // On the level, the gesture itself: gone from the cut's
                    // frame, and none started while the swim or fall lasts.
                    let mut driver = Driver::seeded(7);
                    let delta = 1.0 / 60.0;
                    for frame in 0..150 {
                        let t = frame as f32 * delta;
                        let ask = cut_short(true, CUTS[0], how)(t);
                        let asked = (1.6..1.63).contains(&t).then_some("Wing Stretch");
                        let inputs = Inputs {
                            delta,
                            velocity: Vec3::Y * ask.rise,
                            gesture: ask.gesture.or(asked),
                            swimming: ask.swimming,
                            ..Inputs::default()
                        };
                        driver
                            .drive(rig, &inputs, crate::anim::driver::level_ground)
                            .expect("an unheld body is posed");
                        let cut = (CUTS[0]..CUTS[0] + 0.35).contains(&t);
                        if cut && let Some((playing, _)) = driver.gesture() {
                            found.push(format!(
                                "{name} still plays {playing} {t:.2} s in, {how} begun at {}",
                                CUTS[0]
                            ));
                            break;
                        }
                    }
                    for (on, rise) in rises(grade) {
                        for (ground, terrain) in [
                            ("a 20 degree slope", Terrain::Plane(rise)),
                            ("the game's 20 degree slope", Terrain::Game(rise)),
                        ] {
                            for hz in [30.0f32, 60.0, 144.0] {
                                for cut in CUTS {
                                    let seconds = cut + 1.0;
                                    let cut_into = drive(
                                        rig,
                                        hz,
                                        seconds,
                                        terrain,
                                        &cut_short(true, cut, how),
                                        false,
                                    );
                                    let standing = drive(
                                        rig,
                                        hz,
                                        seconds,
                                        terrain,
                                        &cut_short(false, cut, how),
                                        false,
                                    );
                                    let from = (cut * hz).round() as usize - 1;
                                    let ((moved, at), (turned, turned_at)) =
                                        excess(&cut_into, &standing, hz, from);
                                    if moved > bound.0 || turned > bound.1 {
                                        found.push(format!(
                                            "{name} into {how} {cut} s into a Wing Stretch facing \
                                             {on} {ground} at {hz}: a joint moved {:.1} mm a \
                                             sixtieth faster than from standing (frame {at}) \
                                             and turned {turned:.3} rad faster (frame {turned_at})",
                                            moved * 1000.0
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                failures.lock().expect("a lock").extend(found);
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How much faster than the same clip without `keep_elbows_up` any joint may
/// move and turn in a frame, in metres and radians a sixtieth of a second.
const ELBOWS_UP: (f32, f32) = (0.015, 0.2);

/// **What lifts a winged body's elbows out of a slope swings no foreleg over
/// in a frame** (#462's fixer): roll 9 (a glider, whose emotes #462 was
/// seen on), a bat and a glider, through every emote and
/// rest of their rosters, on the level and on slopes of 20 degrees (as a
/// plane and as the game hands it) and 30 (a plane) - facing up, down and
/// across them both ways - at 30, 60 and 144 frames a second: **no joint
/// moves or turns faster than [`ELBOWS_UP`] past the same clip with
/// `keep_elbows_up` left out**, from the frame before it is asked to its end.
///
/// Before, an elbow was swung the nearer way round, and held at the ground
/// from the frame it touched: roll 9's Curl facing down the 30 degree slope,
/// its elbow pointing straight down and crossing from one side of that to
/// the other, turned its right shoulder 1.58 rad a sixtieth past the clip's
/// at 144 frames a second (0.63 at 60 and at 30, the one frame), moving its
/// elbow 110.5 mm past it; its Stretch across the game's 20 degree slope
/// 0.55 rad and 47.2 mm, at 144.
#[test]
fn keep_elbows_up_swings_no_foreleg_over_in_a_frame() {
    use WingedFamily::{Bat, Glider};
    let bodies = [
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
    ]
    .into_iter()
    .map(|(name, lineage)| (name, body(lineage)))
    .chain(std::iter::once(("roll 9 (a glider)", rolled(9))));
    let mut grounds = vec![("the level".to_string(), Terrain::Plane(Vec3::ZERO))];
    for (on, rise) in rises(20.0f32.to_radians().tan()) {
        grounds.push((
            format!("facing {on} a 20 degree slope"),
            Terrain::Plane(rise),
        ));
        grounds.push((
            format!("facing {on} the game's 20 degree slope"),
            Terrain::Game(rise),
        ));
    }
    for (on, rise) in rises(30.0f32.to_radians().tan()) {
        grounds.push((
            format!("facing {on} a 30 degree slope"),
            Terrain::Plane(rise),
        ));
    }
    let (bodies, grounds): (Vec<_>, _) = (bodies.collect(), &grounds);
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let rig = &avatar.rig;
            for &clip in super::rests::roster(rig) {
                if clip == crate::anim::emote::GET_UP {
                    continue;
                }
                let failures = &failures;
                scope.spawn(move || {
                    let plan = move |t: f32| Ask {
                        gesture: (1.0..1.03).contains(&t).then_some(clip),
                        ..Ask::default()
                    };
                    // A rest goes down, holds and gets up again when asked:
                    // never asked here, it is held to the end.
                    let seconds = super::rests::seconds(rig, clip).map_or(3.0, |down| 2.0 + down);
                    let mut found = Vec::new();
                    for (ground, terrain) in grounds {
                        for hz in [30.0f32, 60.0, 144.0] {
                            let with = drive(rig, hz, seconds, *terrain, &plan, false);
                            ELBOWS_LEFT_OUT.with(|out| out.set(true));
                            let without = drive(rig, hz, seconds, *terrain, &plan, false);
                            ELBOWS_LEFT_OUT.with(|out| out.set(false));
                            let from = (0.9 * hz).round() as usize;
                            let ((moved, moved_at), (turned, turned_at)) =
                                excess(&with, &without, hz, from);
                            if moved > ELBOWS_UP.0 || turned > ELBOWS_UP.1 {
                                found.push(format!(
                                    "{name}'s {clip} on {ground} at {hz}: a joint moved {:.1} mm \
                                     a sixtieth past the clip's (frame {moved_at}) and turned \
                                     {turned:.3} rad past it (frame {turned_at})",
                                    moved * 1000.0
                                ));
                            }
                        }
                    }
                    failures.lock().expect("a lock").extend(found);
                });
            }
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
