//! A winged body's leaps and falls (#456, #459), carried as Overlands
//! carries it (`Carriage::Chassis`, the ground answered every frame in the
//! body's frame, against the chassis' own height): a jump on the level and
//! on a slope, a fall reported by its speed alone, and a fall off a deck's
//! edge - the way a winged body leaves the ground in Overlands, whose jump
//! is a flight's takeoff - turn no joint over in a frame and snap none, at
//! 30, 60 and 144 frames a second; on slopes, standing and walking, for the
//! bodies Overlands hands sloped ground to (those on four).

use super::winged_bounds::turn_between;
use symbios_avatar::anim::driver::{Driver, Inputs};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Ground, Quat, Rig, Vec3, WingedFamily, WingedParams,
};

/// The rates each case is driven at, frames a second.
const RATES: [f32; 3] = [30.0, 60.0, 144.0];

/// How fast any joint may turn through a leap or a fall, in radians a
/// sixtieth of a second: over the most any body turns one in any case here,
/// 0.78 (a glider falling by its speed as it bounds at 1.5 m/s, at 144
/// frames a second, as its bound takes it up again: 0.56 at 30 and 0.71 at
/// 60, a speed and not a step); every body on a slope turns one at most
/// 0.39 (a Bat x Bird cross at 0.3 landing a jump standing facing up one,
/// its hind hip). A joint turned over in a frame reads 3 at 60 frames a
/// second and 7 at 144.
const TURN: f32 = 0.85;

/// How fast any joint may move through a leap or a fall, in metres a
/// sixtieth of a second: over the most one moves in any case here, 94.4 mm
/// (a bird falling by its speed as it runs at 1.5 m/s, a wing's joint, at
/// 144 frames a second); on a slope at most 76.6 (a bat landing a jump it
/// took walking facing up one, its fore elbow).
const MOVE: f32 = 0.12;

/// A 20 degree slope rising by `grade` a metre - facing up it, down it and
/// across it both ways - as a plane and as the game hands it, each with its
/// words.
fn slopes(grade: f32) -> Vec<(&'static str, &'static str, Floor)> {
    let mut slopes = Vec::new();
    for (on, rise) in [
        ("up", Vec3::Z * grade),
        ("down", -Vec3::Z * grade),
        ("across, its left the higher", Vec3::X * grade),
        ("across, its right the higher", -Vec3::X * grade),
    ] {
        slopes.push((on, "a 20 degree slope", Floor::Plane(rise)));
        slopes.push((on, "the game's 20 degree slope", Floor::Grid(rise)));
    }
    slopes
}

/// A body of `lineage`.
fn body(lineage: Lineage<WingedFamily>) -> Avatar {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    Avatar::build_for_shape(&AvatarRecord::new("Winged", Archetype::Winged(params)))
        .expect("a winged body builds")
}

/// Every winged family and the crosses their leaps were seen to fail on:
/// a bat, a glider, a bird, a bat-led cross with a bird both ways named, a
/// Bat x Glider cross either side of halfway, the crosses a glider leads
/// with a bird and with a bat, and a Bird x Glider cross.
fn bodies() -> Vec<(&'static str, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    vec![
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bird", Lineage::pure(Bird)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
        ("bird x bat 0.7", Lineage::cross(Bird, Bat, 0.7)),
        ("bat x glider 0.49", Lineage::cross(Bat, Glider, 0.49)),
        ("bat x glider 0.5", Lineage::cross(Bat, Glider, 0.5)),
        ("glider x bird 0.3", Lineage::cross(Glider, Bird, 0.3)),
        ("glider x bat 0.3", Lineage::cross(Glider, Bat, 0.3)),
        ("bird x glider 0.3", Lineage::cross(Bird, Glider, 0.3)),
    ]
}

/// The world's floor under a body, as its host hands it.
#[derive(Clone, Copy, Debug)]
enum Floor {
    /// A plane through the origin rising by `rise` along each world axis.
    Plane(Vec3),
    /// The same plane handed over as Overlands hands its terrain: heights
    /// a quarter metre apart round the body, bilinear between them.
    Grid(Vec3),
    /// A deck `drop` metres over the ground, its edge 0.6 m ahead, handed
    /// over as Overlands hands a structure (`PAST_AN_EDGE`): level at the
    /// floor under the chassis, past the edge as well, and in the air the
    /// level at the chassis.
    Deck(f32),
}

impl Floor {
    /// The world's height at `(x, z)`.
    fn height(self, x: f32, z: f32) -> f32 {
        match self {
            Floor::Plane(rise) | Floor::Grid(rise) => rise.x * x + rise.z * z,
            Floor::Deck(drop) => {
                if z < 0.6 {
                    0.0
                } else {
                    -drop
                }
            }
        }
    }
}

/// What a body is asked to do: travel ahead at `speed`, and jump at a
/// second with this upward speed, or report a fall of 3 m/s for four
/// tenths of a second from a second on, its ground staying where it stands.
#[derive(Clone, Copy, Debug)]
struct Scene {
    floor: Floor,
    speed: f32,
    jump: Option<f32>,
    fall: bool,
}

/// The grid's spacing, its first sample's place in the body's frame along
/// each axis and how many samples a side (as `keep_clear_tests` reads
/// Overlands' patch).
const STEP: f32 = 0.25;
const ORIGIN: f32 = -1.25;
const SIDE: usize = 11;

/// A patch of heights round the body, answered as Overlands answers it.
fn patch(heights: &[f32], point: Vec3) -> Option<Ground> {
    let last = (SIDE - 1) as f32;
    let along = |at: f32| {
        let grid = (at - ORIGIN) / STEP;
        let held = grid.clamp(0.0, last);
        let cell = held.floor().min(last - 1.0);
        (cell as usize, held - cell, (0.0..=last).contains(&grid))
    };
    let ((column, x, inside_x), (row, z, inside_z)) = (along(point.x), along(point.z));
    let at = |c: usize, r: usize| heights[r * SIDE + c];
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
    Some(Ground {
        position: Vec3::new(point.x, near + (far - near) * z, point.z),
        normal: Vec3::new(-rise_x, 1.0, -rise_z).normalize_or(Vec3::Y),
    })
}

/// Each frame's joints and their turns, `rig` driven at `hz` for three
/// seconds through `scene`, its chassis a body under gravity on the floor.
fn drive(rig: &Rig, hz: f32, scene: Scene) -> Vec<(Vec<Vec3>, Vec<Quat>)> {
    let delta = 1.0 / hz;
    let mut driver = Driver::seeded(7);
    let (mut at, mut y, mut rising) = (Vec3::ZERO, 0.0f32, 0.0f32);
    let mut jumped = false;
    let frames = (3.0 * hz).round() as usize;
    let mut out = Vec::with_capacity(frames);
    for frame in 0..frames {
        let t = frame as f32 * delta;
        at.z += scene.speed * delta;
        let floor = scene.floor.height(at.x, at.z);
        if let Some(up) = scene.jump.filter(|_| !jumped && t >= 1.0) {
            jumped = true;
            rising = up;
        }
        if y > floor + 1e-6 || rising > 0.0 {
            rising -= 9.81 * delta;
            y += rising * delta;
            if y <= floor {
                (y, rising) = (floor, 0.0);
            }
        } else {
            (y, rising) = (floor, 0.0);
        }
        let aloft = y > floor + 1e-6;
        let falling = scene.fall && (1.0..1.4).contains(&t);
        let inputs = Inputs {
            delta,
            velocity: Vec3::new(0.0, if falling { -3.0 } else { rising }, scene.speed),
            at,
            ..Inputs::default()
        };
        let heights: Vec<f32> = (0..SIDE * SIDE)
            .map(|index| {
                let (column, row) = (index % SIDE, index / SIDE);
                let (x, z) = (ORIGIN + column as f32 * STEP, ORIGIN + row as f32 * STEP);
                scene.floor.height(at.x + x, at.z + z) - y
            })
            .collect();
        let ground = |point: Vec3| match scene.floor {
            Floor::Plane(rise) => Some(Ground {
                position: Vec3::new(
                    point.x,
                    scene.floor.height(at.x + point.x, at.z + point.z) - y,
                    point.z,
                ),
                normal: Vec3::new(-rise.x, 1.0, -rise.z).normalize(),
            }),
            Floor::Grid(_) => patch(&heights, point),
            Floor::Deck(_) => Some(Ground::level(Vec3::new(
                point.x,
                if aloft { 0.0 } else { floor - y },
                point.z,
            ))),
        };
        let driven = driver
            .drive(rig, &inputs, ground)
            .expect("an unheld body is posed");
        let posed = driven.pose.forward(rig);
        out.push((posed.positions, posed.rotations));
    }
    out
}

/// The most any joint moves and turns in a frame from `from` on, each read
/// as a sixtieth of a second's at `hz`, in metres and radians, and the
/// frames they were read at.
fn fastest(
    frames: &[(Vec<Vec3>, Vec<Quat>)],
    hz: f32,
    from: usize,
) -> ((f32, usize), (f32, usize)) {
    let scale = hz / 60.0;
    let (mut moved, mut turned) = ((0.0f32, 0), (0.0f32, 0));
    for at in from.max(1)..frames.len() {
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

/// What each body is asked, by its name and rig: each scene and its words.
type Scenes<'a> = dyn Fn(&str, &Rig) -> Vec<(String, Scene)> + Sync + 'a;

/// Every body driven through `scenes` at every rate, from just before a
/// second in: what any joint moved and turned faster than [`MOVE`] and
/// [`TURN`].
fn past_bounds(scenes: &Scenes<'_>) -> Vec<String> {
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, lineage) in bodies() {
            let failures = &failures;
            scope.spawn(move || {
                let avatar = body(lineage);
                let rig = &avatar.rig;
                let mut found = Vec::new();
                for (what, scene) in scenes(name, rig) {
                    for hz in RATES {
                        let frames = drive(rig, hz, scene);
                        let from = (0.9 * hz).round() as usize;
                        let ((moved, moved_at), (turned, turned_at)) = fastest(&frames, hz, from);
                        if moved > MOVE || turned > TURN {
                            found.push(format!(
                                "{name} {what} at {hz}: a joint moved {:.1} mm a sixtieth (frame \
                                 {moved_at}) and turned {turned:.3} rad (frame {turned_at})",
                                moved * 1000.0
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

/// **A winged body's leap turns no joint over in a frame** (#456, #459):
/// every body of [`bodies`], jumping at 3 m/s standing and walking at 0.5
/// m/s on the level, and the bodies on four standing and walking at 0.5 m/s
/// on a 20 degree slope - facing up it, down it and across it both ways -
/// as a plane and as the game hands it, at 30, 60 and 144 frames a second:
/// **no joint moves faster than [`MOVE`] or turns faster than [`TURN`]**,
/// from the frame before the jump to two seconds on. Birds and the crosses
/// a bird leads are asked on the level alone: Overlands stands them on the
/// level floor (a Bird x Glider cross landing facing down a 20 degree slope
/// still flicks an ear spring over, as before #456: see the issue).
///
/// Before, a leap was sized on the reach of the longest leg's bones (a
/// bat's wing-arm, 0.70 m, over a hind leg standing 0.16 m from hip to pad),
/// and a glider's legs were left in the rest pose until its tuck began,
/// then tucked and held off the plane it left by two solves that read a
/// folded leg apart, under a trunk pitched as far as a dog's: 270 of the
/// two tests' cases and rates failed, a joint turning 3.14 rad a sixtieth at
/// 60 frames a second and 7.54 at 144 (a glider facing down the slope, half
/// a turn in a frame), and moving up to 731 mm a sixtieth (a bat facing up
/// it at 144). And with the legs read as they stand, a foot was still
/// tucked upright and held off the plane it left along its up, and laid on
/// the ground by the toes' own lift only in the frame the body came down: a
/// Bat x Glider cross at 0.5 walking up the slope turned its fore shoulder
/// 0.69 rad in that frame at 60 and at 144 (1.65 a sixtieth) and moved its
/// elbow 294 mm a sixtieth, a Glider x Bird cross at 0.3 its hind hip 1.09
/// rad a sixtieth, 1.32 rad in the frame at 30.
#[test]
fn a_winged_leap_turns_no_joint_over_in_a_frame() {
    let grade = 20.0f32.to_radians().tan();
    let failures = past_bounds(&|_, rig| {
        let level = Floor::Plane(Vec3::ZERO);
        let mut scenes = vec![
            (
                "leaping on the level".to_string(),
                Scene {
                    floor: level,
                    speed: 0.0,
                    jump: Some(3.0),
                    fall: false,
                },
            ),
            (
                "leaping as it walks on the level".to_string(),
                Scene {
                    floor: level,
                    speed: 0.5,
                    jump: Some(3.0),
                    fall: false,
                },
            ),
        ];
        if rig.four_legged() {
            for (on, ground, floor) in slopes(grade) {
                for (how, speed) in [("standing", 0.0), ("walking", 0.5)] {
                    scenes.push((
                        format!("leaping {how} facing {on} {ground}"),
                        Scene {
                            floor,
                            speed,
                            jump: Some(3.0),
                            fall: false,
                        },
                    ));
                }
            }
        }
        scenes
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// **A winged body's fall lands without turning a joint over** (#456, the
/// in-game path, since Overlands hands a winged body's jump to its flight):
/// every body of [`bodies`] walking off a deck 0.6 and 1.5 m high at 0.5
/// and 1.5 m/s, handed the deck as Overlands hands a structure, and falling
/// for four tenths of a second at 3 m/s by its speed alone, standing,
/// walking and bounding (0.5 and 1.5 m/s) on the level, and the bodies on
/// four standing and walking at 0.5 m/s on [`slopes`], at 30, 60 and 144
/// frames a second: **no joint moves faster than [`MOVE`] or turns faster
/// than [`TURN`]**, from before it falls to two seconds on.
///
/// Before, on the misread legs above, a joint turned 3.14 rad a sixtieth
/// at 60 frames a second (a Bat x Glider cross at 0.5 falling standing) and
/// 7.52 at 144 (a Glider x Bird cross at 0.3 falling as it walks), and
/// moved up to 719 mm a sixtieth (the Bat x Glider cross off a 0.6 m deck);
/// and with the legs read as they stand, a fall was still joined at the
/// start of a jump's arc, its legs drawn up as at a jump's height wherever
/// the ground cut it short: stepping off a 0.6 m deck, a bat's forelimb
/// moved 64 mm in the frame it landed. On a slope a fall held its feet
/// upright, short of ground falling away under them, and landed laying them
/// on it and sinking its trunk to reach it in one frame: a Glider x Bird
/// cross at 0.3 facing up a 20 degree slope turned its hind hip 1.08 rad a
/// sixtieth at 144 frames a second (1.32 in the frame at 30), a Bat x
/// Glider cross at 0.5 its fore shoulder 1.06, and a bat facing down one
/// moved its elbow 171 mm a sixtieth at 144, 71 mm in the frame at every
/// rate.
#[test]
fn a_winged_fall_lands_without_turning_a_joint_over() {
    let grade = 20.0f32.to_radians().tan();
    let failures = past_bounds(&|_, rig| {
        let level = Floor::Plane(Vec3::ZERO);
        let mut scenes = Vec::new();
        if rig.four_legged() {
            for (on, ground, floor) in slopes(grade) {
                for (how, speed) in [("standing", 0.0), ("walking", 0.5)] {
                    scenes.push((
                        format!("falling by its speed {how} facing {on} {ground}"),
                        Scene {
                            floor,
                            speed,
                            jump: None,
                            fall: true,
                        },
                    ));
                }
            }
        }
        for speed in [0.0f32, 0.5, 1.5] {
            scenes.push((
                format!("falling by its speed at {speed} m/s"),
                Scene {
                    floor: level,
                    speed,
                    jump: None,
                    fall: true,
                },
            ));
        }
        for drop in [0.6f32, 1.5] {
            for speed in [0.5f32, 1.5] {
                scenes.push((
                    format!("walking off a {drop} m deck at {speed} m/s"),
                    Scene {
                        floor: Floor::Deck(drop),
                        speed,
                        jump: None,
                        fall: false,
                    },
                ));
            }
        }
        scenes
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
