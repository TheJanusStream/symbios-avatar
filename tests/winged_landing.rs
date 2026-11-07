//! A winged body landing and going up off a landing (#435), carried as
//! Overlands carries it (`Carriage::Chassis`, the ground answered every
//! frame): a landing folds its wings no faster than its gaits move it, a
//! landing on a slope hands back to the ground without a pop and with
//! nothing drawn into the slope, and a takeoff asked of a landing down on its
//! feet crouches and pushes as one from standing does.

use super::winged_bounds::{UNDER_GROUND, furthest};
use symbios_avatar::anim::driver::{Carriage, Driver, DriverConfig, Inputs, Source, level_ground};
use symbios_avatar::anim::winged::flight::{Flight, FlightMode, Flown};
use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Ground, MeshKind, Pose, Quat, Rig, Vec3, WingedFamily,
    WingedParams, Zone,
};

/// The frame every step is read as, a sixtieth of a second.
const REFERENCE: f32 = 1.0 / 60.0;

/// The rates each case is driven at, frames a second.
const RATES: [f32; 3] = [30.0, 60.0, 144.0];

fn winged(lineage: Lineage<WingedFamily>) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    AvatarRecord::new("Winged", Archetype::Winged(params))
}

/// A bird, a bat, a glider, two crosses at the halfway blend - a bat with a
/// glider, a bird with a bat - and a cross a bat leads, with a bird.
fn bodies() -> Vec<(&'static str, Avatar)> {
    use WingedFamily::{Bat, Bird, Glider};
    [
        ("bird", Lineage::pure(Bird)),
        ("bat", Lineage::pure(Bat)),
        ("glider", Lineage::pure(Glider)),
        ("bat x glider", Lineage::cross(Bat, Glider, 0.5)),
        ("bird x bat", Lineage::cross(Bird, Bat, 0.5)),
        ("bat x bird 0.3", Lineage::cross(Bat, Bird, 0.3)),
    ]
    .into_iter()
    .map(|(name, lineage)| {
        let avatar = Avatar::build_for_shape(&winged(lineage)).expect("a winged body builds");
        (name, avatar)
    })
    .collect()
}

/// The trunk's length, shoulder to hip.
fn trunk(rig: &Rig) -> f32 {
    let pelvis = rig.in_zone(Zone::Pelvis)[0];
    let withers = *rig.in_zone(Zone::Chest).last().expect("a chest");
    rig.joints[pelvis]
        .position
        .distance(rig.joints[withers].position)
}

/// A plane through the body's origin, rising `degrees` toward `toward`.
#[derive(Clone, Copy, Debug)]
struct Plane {
    rise: Vec3,
}

impl Plane {
    fn new(degrees: f32, toward: Vec3) -> Self {
        Self {
            rise: toward * degrees.to_radians().tan(),
        }
    }

    fn height(self, at: Vec3) -> f32 {
        self.rise.x * at.x + self.rise.z * at.z
    }

    fn ground(self) -> impl Fn(Vec3) -> Option<Ground> + Copy {
        move |at: Vec3| {
            Some(Ground {
                position: Vec3::new(at.x, self.height(at), at.z),
                normal: Vec3::new(-self.rise.x, 1.0, -self.rise.z).normalize(),
            })
        }
    }
}

/// How far over the plane the lowest drawn point of the body's skin - its
/// soles, toes and membrane with it - stands, in metres.
fn clearance(avatar: &Avatar, pose: &Pose, plane: Plane) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    avatar
        .meshes
        .iter()
        .filter(|mesh| mesh.kind == MeshKind::Skin)
        .flat_map(|mesh| posed.deform_mesh(rig, &mesh.mesh).positions)
        .map(|at| at.y - plane.height(at))
        .fold(f32::MAX, f32::min)
}

/// The most any joint turned from `before` to `now`, in radians.
fn turned(before: &[Quat], now: &[Quat]) -> f32 {
    now.iter()
        .zip(before)
        .map(|(a, b)| a.angle_between(*b))
        .fold(0.0, f32::max)
}

/// One driven frame.
struct Shot {
    joints: Vec<Vec3>,
    turns: Vec<Quat>,
    source: Source,
    pose: Pose,
    flown: Option<Flown>,
}

/// A host driving one body over `ground`, `delta` a frame, under `carriage`.
struct Host<'a, G> {
    rig: &'a Rig,
    driver: Driver,
    delta: f32,
    ground: G,
    shots: Vec<Shot>,
}

impl<'a, G: Fn(Vec3) -> Option<Ground> + Copy> Host<'a, G> {
    fn new(rig: &'a Rig, delta: f32, ground: G, carriage: Carriage) -> Self {
        Self {
            rig,
            driver: Driver::new(
                DriverConfig {
                    carriage,
                    ..DriverConfig::default()
                },
                7,
            ),
            delta,
            ground,
            shots: Vec::new(),
        }
    }

    fn frames(&self, seconds: f32) -> usize {
        ((seconds / self.delta).round() as usize).max(1)
    }

    fn frame(
        &mut self,
        flight: Option<FlightMode>,
        speed: f32,
        contact: Option<f32>,
    ) -> Option<Flown> {
        let flight = flight.map(|mode| Flight {
            contact,
            ..Flight::new(mode)
        });
        let driven = self
            .driver
            .drive(
                self.rig,
                &Inputs {
                    delta: self.delta,
                    velocity: Vec3::Z * speed,
                    flight,
                    ..Inputs::default()
                },
                self.ground,
            )
            .expect("an unheld body is posed");
        let posed = driven.pose.forward(self.rig);
        let flown = self.driver.flight(self.rig);
        self.shots.push(Shot {
            joints: posed.positions,
            turns: posed.rotations,
            source: driven.source,
            pose: driven.pose,
            flown,
        });
        flown
    }

    fn stand(&mut self, seconds: f32) {
        for _ in 0..self.frames(seconds) {
            self.frame(None, 0.0, None);
        }
    }

    /// Takes off and is handed on to flapping two beats after it lifts,
    /// flaps, flares by time to contact and lands, the landing held until it
    /// has settled: the frame it is then handed back to the ground.
    fn fly_and_land(&mut self) -> usize {
        let mut lifted = None;
        for _ in 0..self.frames(5.0) {
            let flown = self
                .frame(Some(FlightMode::Takeoff), 2.0, None)
                .expect("flying");
            if flown.lifted && lifted.is_none() {
                lifted = Some(flown.phase);
            }
            if lifted.is_some_and(|at| (flown.phase - at).rem_euclid(64.0) >= 2.0) {
                break;
            }
        }
        for _ in 0..self.frames(0.5) {
            self.frame(Some(FlightMode::Flap), 4.0, None);
        }
        let flare = self.frames(0.3);
        for at in 0..flare {
            let contact = 0.45 - 0.35 * at as f32 / (flare - 1).max(1) as f32;
            self.frame(Some(FlightMode::Flare), 2.0, Some(contact));
        }
        for _ in 0..self.frames(5.0) {
            let flown = self
                .frame(Some(FlightMode::Land), 0.0, None)
                .expect("flying");
            if flown.settled {
                break;
            }
        }
        self.shots.len()
    }

    /// How far the joints moved and turned into each frame of `frames`, the
    /// most of them, each read as a sixtieth's at the speed it went.
    fn steps(&self, frames: std::ops::Range<usize>) -> Vec<(f32, f32)> {
        self.steps_of(frames, &(0..self.rig.len()).collect::<Vec<_>>())
    }

    /// [`Self::steps`] of `joints` alone.
    fn steps_of(&self, frames: std::ops::Range<usize>, joints: &[usize]) -> Vec<(f32, f32)> {
        let scale = REFERENCE / self.delta;
        frames
            .map(|at| {
                let (before, now) = (&self.shots[at - 1], &self.shots[at]);
                let pick = |shot: &Shot| -> (Vec<Vec3>, Vec<Quat>) {
                    joints
                        .iter()
                        .map(|&joint| (shot.joints[joint], shot.turns[joint]))
                        .unzip()
                };
                let ((was, was_turned), (is, is_turned)) = (pick(before), pick(now));
                (
                    furthest(&was, &is) * scale,
                    turned(&was_turned, &is_turned) * scale,
                )
            })
            .collect()
    }
}

/// How long after the hand-back its steps are held to the settled landing's,
/// in seconds: past the source switch's blend (0.15 s).
const HANDED_BACK: f32 = 0.25;

/// The most any joint may turn in a sixtieth of a second on the ground, in
/// radians: `winged_life`'s bound.
const GROUND_TURN: f32 = 1.25;

/// How much more than twice the most it moved in the settled landing's last
/// frames a joint may move in a sixtieth across the hand back, as a share of
/// the trunk: three times the pop bound's hundredth, for what the idle draws
/// that no landing does - its breath, its sway, its stance's own sink - which
/// its blend carries in over the first frames, a bat x glider's arm 2.7 mm a
/// frame at 144 frames a second on a 20 degree slope. Before, the hand back
/// moved bodies 20 to 90 mm a frame at 60 frames a second, 40 to 300 times
/// what they moved settled.
const HAND_BACK_SLACK: f32 = 0.03;

/// Every joint but those of the feet it stands on - pads, toes and the rest
/// of each foot - which the ground sources set on the ground after their
/// blend (`winged::ground::keep_toes_up`): under any blend on a slope it
/// lifts a pad or lays a toe down for a frame or two and back, a bat's hind
/// toe 12 mm, in a walk stopping on a 20 degree slope as in a landing's hand
/// back (a walk's stop moves the rest of the body 30 to 60 mm a frame around
/// it), so the ground's own, not the hand back's. The feet are held to the
/// slope by [`clearance`], which reads them drawn, and their turns to
/// [`GROUND_TURN`] with every joint's.
fn joints_but_feet(rig: &Rig) -> Vec<usize> {
    let contacts = rig.ground_contacts();
    (0..rig.len())
        .filter(|&joint| {
            let foot = matches!(rig.joints[joint].zone, Zone::Extremity(limb) if contacts.contains(&limb))
                && !rig.wings.as_ref().is_some_and(|wings| wings.owns(joint));
            !foot
        })
        .collect()
}

/// **A landing on a slope hands back to the ground without a pop** (#435):
/// a bird, a bat, a glider and two crosses carried as Overlands carries them
/// stand on a slope of 10 and 20 degrees - facing up it, down it and across
/// it - take off, fly, and land on it, the landing held until it has settled
/// and then handed back to the ground, at 30, 60 and 144 frames a second:
///
/// - **across the hand-back no joint moves in a frame more than twice the
///   most it moved in the four frames before it and [`HAND_BACK_SLACK`] of
///   the trunk** - the pop bound (`winged_life`) with the settled landing
///   for its neighbours - every joint but its feet ([`joints_but_feet`]), **nor
///   turns more than [`GROUND_TURN`]**, every joint, every frame read as a
///   sixtieth at the speed it moved, for the first [`HANDED_BACK`] of the
///   ground source;
/// - **nothing drawn goes into the slope** - soles, toes and membrane - from
///   the settled landing to a second after the hand-back, past
///   [`UNDER_GROUND`] below where it stood on the slope before it took off.
///
/// Before: the settled landing stood its trunk level and its feet as no
/// ground source stands them, so handed back the idle carried it up to
/// 0.31 m to its own stance over the blend - a bat x glider moving 88 mm in a
/// frame at 60 frames a second where it moved 4.7 settled, a glider's tail 60
/// against 1.7 - and a settled glider stood 21 mm into a 20 degree slope
/// facing up it and 78 mm facing down it.
#[test]
fn a_landing_on_a_slope_hands_back_without_a_pop() {
    let bodies = bodies();
    let mut slopes = Vec::new();
    for degrees in [10.0, 20.0] {
        for (way, toward) in [("up", Vec3::Z), ("down", -Vec3::Z), ("across", Vec3::X)] {
            slopes.push((format!("{degrees} {way}"), Plane::new(degrees, toward)));
        }
    }
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let (failures, slopes) = (&failures, &slopes);
            scope.spawn(move || {
                for (slope, plane) in slopes {
                    for rate in RATES {
                        for line in land_on(avatar, *plane, 1.0 / rate) {
                            failures
                                .lock()
                                .expect("a lock")
                                .push(format!("{name} on {slope} at {rate}: {line}"));
                        }
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// [`a_landing_on_a_slope_hands_back_without_a_pop`] for one body on one
/// slope at one rate: every bound it breaks.
fn land_on(avatar: &Avatar, plane: Plane, delta: f32) -> Vec<String> {
    let rig = &avatar.rig;
    let mut host = Host::new(rig, delta, plane.ground(), Carriage::Chassis);
    host.stand(0.5);
    let standing = (0..host.shots.len())
        .step_by(host.frames(0.1))
        .map(|at| clearance(avatar, &host.shots[at].pose, plane))
        .fold(f32::MAX, f32::min)
        .min(0.0);
    let back = host.fly_and_land();
    host.stand(1.0);
    let mut failures = Vec::new();
    if host.shots[back - 1]
        .flown
        .is_none_or(|flown| !flown.settled)
        || host.shots[back].source == Source::Fly
    {
        failures.push("never settled and handed back".to_string());
        return failures;
    }
    let held = joints_but_feet(rig);
    let moved = host
        .steps_of(back - 4..back, &held)
        .into_iter()
        .fold(0.0f32, |most, step| most.max(step.0));
    let slack = HAND_BACK_SLACK * trunk(rig);
    let turns = host.steps(back..back + host.frames(HANDED_BACK));
    for (at, (step, _)) in host
        .steps_of(back..back + host.frames(HANDED_BACK), &held)
        .into_iter()
        .enumerate()
    {
        let turning = turns[at].1;
        if step > 2.0 * moved + slack {
            let frame = back + at;
            let joint = held
                .iter()
                .copied()
                .max_by(|&a, &b| {
                    let far = |joint: usize| {
                        host.shots[frame].joints[joint]
                            .distance(host.shots[frame - 1].joints[joint])
                    };
                    far(a).total_cmp(&far(b))
                })
                .unwrap_or(0);
            failures.push(format!(
                "frame {at} of the ground moved {:.1} mm a sixtieth, settled {:.1} (joint {joint}, {:?} {:?})",
                step * 1000.0,
                moved * 1000.0,
                rig.joints[joint].zone,
                rig.joints[joint].role
            ));
        }
        if turning > GROUND_TURN {
            failures.push(format!(
                "frame {at} of the ground turned a joint {turning:.2} rad a sixtieth"
            ));
        }
    }
    let (lowest, at) = (back - 1..host.shots.len())
        .map(|at| (clearance(avatar, &host.shots[at].pose, plane), at))
        .fold(
            (f32::MAX, 0),
            |low, this| if this.0 < low.0 { this } else { low },
        );
    if lowest < standing - UNDER_GROUND {
        failures.push(format!(
            "{:.1} mm into the slope at frame {} of the ground, standing {:.1}",
            -lowest * 1000.0,
            at as i64 - back as i64,
            -standing * 1000.0
        ));
    }
    failures
}

/// The pelvis's height in a shot.
fn hip(rig: &Rig, shot: &Shot) -> f32 {
    shot.joints[rig.in_zone(Zone::Pelvis)[0]].y
}

/// How long into a takeoff its crouch and push are read, in seconds.
const LEAP: f32 = 0.4;

/// **A takeoff asked of a landing down on its feet crouches and pushes**
/// (#435): a bird, a bat, a glider and two crosses standing on the level,
/// asked to land on it - a landing from no height, which a host holds until
/// it has settled - and asked to take off a tenth of a second in, a quarter,
/// and once it has settled, under either carriage and at 30, 60 and 144
/// frames a second:
///
/// - **its root dips at least as far as a takeoff from standing dips** in its
///   first [`LEAP`] seconds, under the same carriage at the same rate;
/// - **and pushes**: it lifts (`Flown::lifted`) within those [`LEAP`]
///   seconds, its root risen from its lowest by at least half the dip by then;
/// - **without a pop**: from the landing to [`LEAP`] into the takeoff no
///   joint moves in a frame more than twice the most it moved in the four
///   frames either side and a hundredth of the trunk (`winged_life`'s
///   bound), nor turns more than 1.25 radians in a sixtieth, each frame read
///   as a sixtieth at the speed it moved.
///
/// Before: `pushing` was set only as a flight began, so a takeoff asked of a
/// landing never crouched or pushed: a bird's root dipped nothing in 0.4 s
/// (it rose 2.6 mm) where one from standing dips 18, and lifted at once.
#[test]
fn a_takeoff_asked_of_a_held_landing_crouches_and_pushes() {
    let bodies = bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let failures = &failures;
            scope.spawn(move || {
                for carriage in [Carriage::Chassis, Carriage::Own] {
                    for rate in RATES {
                        for line in take_off_held(avatar, carriage, 1.0 / rate) {
                            failures
                                .lock()
                                .expect("a lock")
                                .push(format!("{name} {carriage:?} at {rate}: {line}"));
                        }
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A takeoff asked now, for [`LEAP`] seconds: how far its root dips under
/// where it stood, whether it lifts, and how far its root has risen from its
/// lowest when it does.
fn leap<G: Fn(Vec3) -> Option<Ground> + Copy>(
    host: &mut Host<'_, G>,
    standing: f32,
) -> (f32, bool, f32) {
    let from = host.shots.len();
    let mut lifted = None;
    for _ in 0..host.frames(LEAP) {
        let flown = host
            .frame(Some(FlightMode::Takeoff), 2.0, None)
            .expect("flying");
        if flown.lifted && lifted.is_none() {
            lifted = Some(host.shots.len() - 1);
        }
    }
    let lowest = host.shots[from..]
        .iter()
        .map(|shot| hip(host.rig, shot))
        .fold(f32::MAX, f32::min);
    let risen = lifted.map_or(0.0, |at| hip(host.rig, &host.shots[at]) - lowest);
    (lowest - standing, lifted.is_some(), risen)
}

/// [`a_takeoff_asked_of_a_held_landing_crouches_and_pushes`] for one body
/// under one carriage at one rate: every bound it breaks.
fn take_off_held(avatar: &Avatar, carriage: Carriage, delta: f32) -> Vec<String> {
    let rig = &avatar.rig;
    let mut failures = Vec::new();
    let fresh = {
        let mut host = Host::new(rig, delta, level_ground, carriage);
        host.stand(0.5);
        let standing = hip(rig, host.shots.last().expect("a frame"));
        leap(&mut host, standing).0
    };
    let slack = 0.01 * trunk(rig);
    for held in [Some(0.1), Some(0.25), None] {
        let mut host = Host::new(rig, delta, level_ground, carriage);
        host.stand(0.5);
        let standing = hip(rig, host.shots.last().expect("a frame"));
        let start = host.shots.len();
        for _ in 0..held.map_or(host.frames(3.0), |seconds| host.frames(seconds)) {
            let flown = host
                .frame(Some(FlightMode::Land), 0.0, None)
                .expect("flying");
            if held.is_none() && flown.settled {
                break;
            }
        }
        let asked = host.shots.len();
        let (depth, lifted, risen) = leap(&mut host, standing);
        let when = held.map_or("settled".to_string(), |seconds| format!("{seconds} s in"));
        if depth > fresh {
            failures.push(format!(
                "asked {when}: its root dipped {:.1} mm, from standing {:.1}",
                -depth * 1000.0,
                -fresh * 1000.0
            ));
        }
        if !lifted || risen < -0.5 * depth {
            failures.push(format!(
                "asked {when}: lifted {lifted}, its root risen {:.1} mm off a dip of {:.1}",
                risen * 1000.0,
                -depth * 1000.0
            ));
        }
        let steps = host.steps(start + 1..host.shots.len());
        for at in 4..steps.len().saturating_sub(4) {
            let around = (at - 4..at)
                .chain(at + 1..=at + 4)
                .map(|frame| steps[frame].0)
                .fold(0.0f32, f32::max);
            let frame = (start + 1 + at) as i64 - asked as i64;
            if steps[at].0 > 2.0 * around + slack {
                failures.push(format!(
                    "asked {when}: frame {frame} of the takeoff moved {:.1} mm a sixtieth \
                     against {:.1}",
                    steps[at].0 * 1000.0,
                    around * 1000.0
                ));
            }
            if steps[at].1 > GROUND_TURN {
                failures.push(format!(
                    "asked {when}: frame {frame} of the takeoff turned a joint {:.2} rad a \
                     sixtieth",
                    steps[at].1
                ));
            }
        }
    }
    failures
}

/// How long a body's fast gait is driven to read how far a frame of it moves
/// its joints, and how much of its start is let go: the walk off and the
/// blend into it.
const GAIT_READ: (f32, f32) = (1.5, 0.5);

/// **A landing folds its wings no faster than its gaits move it** (#435):
/// a bird, a bat, a glider and crosses - a cross a bat leads among them -
/// carried as Overlands carries them, flown over the level, flared by time
/// to contact and landed, the landing held until it has settled, at 30, 60
/// and 144 frames a second: from the landing's first frame to its settling
/// **no joint moves in a frame more than six times what it moves in a
/// median frame of its own fast gait** - its run, hop or bound at a Froude
/// number of one on its hip, as `winged_life` drives it - **and a hundredth
/// of its trunk**,
/// the pop bound with the gait for its neighbours, every frame read as a
/// sixtieth at the speed it moved.
///
/// Before: a landing eased its beat's width to nothing with the clock still
/// beating, so a bat's last braking strokes went on through its landing's
/// first frames, its wingtip 353, 336 and 257 mm in a sixtieth at 60 frames
/// a second where a frame of its bound moves a joint 118 at the most, and
/// 343 at 30 frames a second, a bird x bat cross's 223.
#[test]
fn a_landing_folds_no_faster_than_its_gaits_move() {
    let bodies = bodies();
    let failures = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (name, avatar) in &bodies {
            let failures = &failures;
            scope.spawn(move || {
                for rate in RATES {
                    for line in fold(avatar, 1.0 / rate) {
                        failures
                            .lock()
                            .expect("a lock")
                            .push(format!("{name} at {rate}: {line}"));
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().expect("a lock");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// [`a_landing_folds_no_faster_than_its_gaits_move`] for one body at one
/// rate: every bound it breaks.
fn fold(avatar: &Avatar, delta: f32) -> Vec<String> {
    let rig = &avatar.rig;
    let mut failures = Vec::new();
    let gait = {
        let hip = rig
            .limb_chain(symbios_avatar::Limb::HindLeft)
            .map_or(0.1, |chain| rig.joints[chain[0]].position.y);
        let speed = (9.81 * hip).sqrt().max(0.4);
        let mut host = Host::new(rig, delta, level_ground, Carriage::Chassis);
        for _ in 0..host.frames(GAIT_READ.0) {
            host.frame(None, speed, None);
        }
        // A median frame, not the most: a bat's bound carries a single-frame
        // arm pop of about 115 mm at every rate (the 0.23.1 critic), which read
        // as a sixtieth at 144 frames a second loosened the bound past 550 mm.
        let mut steps: Vec<f32> = host
            .steps(host.frames(GAIT_READ.1)..host.shots.len())
            .into_iter()
            .map(|step| step.0)
            .collect();
        steps.sort_by(f32::total_cmp);
        3.0 * steps[steps.len() / 2]
    };
    let mut host = Host::new(rig, delta, level_ground, Carriage::Chassis);
    host.stand(0.5);
    let back = host.fly_and_land();
    let landed = (0..back)
        .rev()
        .take_while(|&at| {
            host.shots[at]
                .flown
                .is_some_and(|flown| flown.mode == FlightMode::Land)
        })
        .last()
        .expect("a landing");
    if host.shots[back - 1]
        .flown
        .is_none_or(|flown| !flown.settled)
    {
        failures.push("never settled".to_string());
    }
    let bound = 2.0 * gait + 0.01 * trunk(rig);
    for (at, (step, _)) in host.steps(landed..back).into_iter().enumerate() {
        if step > bound {
            failures.push(format!(
                "frame {at} of the landing moved {:.1} mm a sixtieth, its gait {:.1}",
                step * 1000.0,
                gait * 1000.0
            ));
        }
    }
    failures
}

/// **The driver and the avatar cross threads** (the 0.23.1 critic): a
/// consumer keeps a `Driver` in a Bevy component and an `Avatar` in an asset,
/// both `Send + Sync + 'static`; a `Cell` in the flight's state once made the
/// `Driver` lose `Sync` in a patch release, which would have broken every
/// consumer's build. Checked when this compiles.
#[test]
fn the_driver_and_the_avatar_can_cross_threads() {
    fn needs<T: Send + Sync + 'static>() {}
    needs::<Driver>();
    needs::<Avatar>();
}
