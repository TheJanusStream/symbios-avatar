//! What keeps a winged body on four legs out of the ground it is handed
//! (`ground::keep_clear`, #440 and #441), measured against the same motion
//! without it: on the ground Overlands hands such a body - heights on a
//! quarter-metre grid, bilinear between them - and across every edge where it
//! comes in or goes out.
//!
//! Here and not in `tests/` because the motion it is measured against is
//! the driver's own with `keep_clear` left out, which only the crate can ask
//! for (`ground::LEFT_OUT`, per thread).

use glam::{Quat, Vec3};

use crate::anim::driver::{Driver, Inputs};
use crate::anim::ground::Ground;
use crate::plan::{Blended, Lineage, WingedFamily};
use crate::{Archetype, Avatar, AvatarConfig, AvatarRecord, Flight, FlightMode, WingedParams};

use super::ground::LEFT_OUT;

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
            velocity: ahead * ask.speed,
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
        let posed = driven.pose.forward(rig);
        out.push((posed.positions, posed.rotations));
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
            .map(|(a, b)| a.angle_between(*b))
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
/// any pace turned one (0.086).
const GAME_TURN: f32 = 0.12;

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
/// the most any edge turned one (0.084).
const EDGE: (f32, f32) = (0.02, 0.15);

/// The same through a takeoff and a landing, which carry it the whole way:
/// over the most one moved and turned a joint (17.0 mm, a bat facing up the
/// slope, and 0.325 rad, a Bat x Glider cross at 0.49, both at 144 frames a
/// second), where the hands are let go of as the body rises off the slope
/// and taken up as it comes down onto it, before #441's fixes and after.
const FLIGHT_EDGE: (f32, f32) = (0.025, 0.45);

/// **What keeps a body on four out of the ground comes in and goes out
/// without a jump** (#441): a bat, a Bat x Glider cross at 0.49 and a Bat x
/// Bird cross at 0.3, named both ways round, on a 20 degree slope - facing
/// up it, down it and across it both ways - into and out of a Head Tilt (an
/// emote, which keeps the stance and the hold), a Rest and its Get Up (which
/// let it go), a swim (which the water floats) and a takeoff and a landing,
/// at 30, 60 and 144 frames a second: **no joint moves or turns further in
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
    let scenes: [(&str, Plan); 4] = [
        ("a Head Tilt", |t| Ask {
            gesture: (1.0..1.03).contains(&t).then_some("Head Tilt"),
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
