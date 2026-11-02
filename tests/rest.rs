//! A beast at rest on the ground (#377): the drawn skin of a sitting and a
//! lying body against the ground it rests on.
//!
//! **The skin as drawn, not the joints.** A pose that never asks where the
//! skin is put the owner's beasts partly underground: P5's sit sank a dog's
//! thigh 165 mm and its tail 82, its lying down a cat's chest 142 and its
//! belly 153, and floated a horse's chest 194 mm off the ground on legs
//! sunk 57. So every reading here is the lowest vertex of the body's own
//! skin, posed through its own skin weights, sorted by the joint that holds
//! it most: the haunches (the pelvis, the tail's root and the thighs), the
//! hind feet (hock to toe), the forelegs, the forefeet, the chest - and
//! everything.
//!
//! The bodies are the roll's whole range: each family and a cross of each,
//! `appeal` from -1.5 to 2, and the composites at both ends of `mass` and
//! `bodyFat` at once.

use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
use symbios_avatar::anim::{Clip, Ground, Pose, Posed, emote, gesture};
use symbios_avatar::plan::BODY_FAT_RANGE;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Family, FootKind, QuadrupedParams, Rig, Vec3, Zone,
};

/// How far anything may reach below the ground, in withers heights, and in
/// metres at least: the skin is drawn in floats.
const SINK: f32 = 0.005;
const SINK_FLOOR: f32 = 0.002;

/// How near the ground a support must come to rest on it, in withers
/// heights, sitting and lying: a dog of 0.9 m touching within 32 mm, a cat
/// of 0.44 within 15. Lying a little looser: the skin of the brisket between
/// the forelegs is bound to the upper arms, and folding the forelegs carries
/// it down under the chest, so the body rests on it a few millimetres above
/// its sternum - on a slight cat, 4.0% of its withers height, the worst of
/// every body here (#377).
const TOUCH: f32 = 0.035;
const TOUCH_LYING: f32 = 0.042;

/// How far off the ground a lying horse's chest stands, at most, in withers
/// heights: it lies on its folded forelegs and its hind legs folded beside
/// it, and its sternum comes down between its knees without touching -
/// measured (#377), 1.0 to 10.2% across every horse here, the default
/// horse's 3.8%. The hind junctions drawn under the trunk (#387) carry the
/// heavy foal's to 11.0%.
const HORSE_CHEST: f32 = 0.115;

/// How far off the ground a horse's croup may stand in its dog-sit, in
/// withers heights: the research's "on the ground or just off it" (09,
/// 2.3.4).
const JUST_OFF: f32 = 0.05;

/// How fast any joint of a body going into, holding or getting up out of a
/// rest may move, in the speed `sqrt(g WH)` of a body its withers height
/// high: about as fast as a galloping dog's paw swings forward past its
/// body. A gallop at a Froude number of three on a leg of 0.55 WH carries
/// the body at 1.3 `sqrt(g WH)`, and a paw in its swing passes the body at
/// three times that. Nothing in a rest has business that fast: the fastest
/// part of any of them, a rising horse's tail tip sweeping along the ground,
/// reads 3.6; a joint past the bound has jumped (#377). Dogs going from their
/// sit into lying down jumped at 6 to 8, horses getting up at 19 to 48.
pub(crate) const SWIFT: f32 = 4.0;

/// Gravity, for [`SWIFT`].
const GRAVITY: f32 = 9.81;

/// How finely a motion is swept, in steps of its own length: at the bound, a
/// 6 mm step on a default dog's lying down.
const STEPS: usize = 300;

/// The furthest any joint but a marker moved between two poses of `rig`, and
/// which joint it was.
pub(crate) fn fastest(rig: &Rig, before: &Posed, after: &Posed) -> (f32, usize) {
    before
        .positions
        .iter()
        .zip(&after.positions)
        .enumerate()
        .filter(|(joint, _)| !rig.joints[*joint].marker)
        .map(|(joint, (from, to))| (from.distance(*to), joint))
        .fold(
            (0.0, 0),
            |most, step| if step.0 > most.0 { step } else { most },
        )
}

/// The bound [`SWIFT`] puts on one step of `seconds` for a body `withers`
/// high, in metres.
pub(crate) fn swift(withers: f32, seconds: f32) -> f32 {
    SWIFT * (GRAVITY * withers).sqrt() * seconds
}

/// A built beast and what it is.
struct Beast {
    what: String,
    avatar: Avatar,
    /// Its withers height, in metres.
    withers: f32,
    hooves: bool,
}

/// The lowest drawn skin of each part a rest is judged by, in metres.
#[derive(Debug)]
struct Lows {
    haunch: f32,
    hind_feet: f32,
    fore_feet: f32,
    fore_legs: f32,
    chest: f32,
    all: f32,
}

fn beasts() -> Vec<Beast> {
    // Built side by side: there are thirty-three.
    let mut records: Vec<(String, AvatarRecord)> = Vec::new();
    let corners = [
        (0.0, None),
        (3.0, Some(BODY_FAT_RANGE.1)),
        (-3.0, Some(BODY_FAT_RANGE.0)),
    ];
    for family in Family::ALL {
        for appeal in [-1.5f32, 1.0, 2.0] {
            for (mass, fat) in corners {
                let mut params = QuadrupedParams::default();
                params.set_kind(family, None, 0.0);
                params.appeal = appeal;
                let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
                record.composites.mass = mass;
                if let Some(fat) = fat {
                    record.composites.body_fat = fat;
                }
                records.push((
                    format!("{family:?} appeal {appeal} mass {mass} fat {fat:?}"),
                    record,
                ));
            }
        }
    }
    for (family, other) in [
        (Family::Dog, Family::Horse),
        (Family::Horse, Family::Dog),
        (Family::Cat, Family::Dog),
    ] {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, Some(other), 0.5);
        records.push((
            format!("{family:?} crossed with {other:?}"),
            AvatarRecord::new("Beast", Archetype::Quadruped(params)),
        ));
    }
    for seed in [1i64, 2, 6] {
        records.push((
            format!("rolled seed {seed}"),
            AvatarRecord::rolled(
                "Beast",
                Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            ),
        ));
    }
    let build = |(what, mut record): (String, AvatarRecord)| {
        record.sanitize();
        let Archetype::Quadruped(params) = &record.archetype else {
            unreachable!("every record here is a beast")
        };
        // The withers height the plan builds a back height to.
        let withers = params.height * 1.2;
        let avatar = Avatar::build_for_shape(&record).unwrap_or_else(|| panic!("{what} builds"));
        let hooves = avatar.rig.anatomy.feet == FootKind::Hooves;
        Beast {
            what,
            avatar,
            withers,
            hooves,
        }
    };
    in_parallel(records, build)
}

/// `work` over every item, eight at a time, in the items' order.
fn in_parallel<T: Send, R: Send>(items: Vec<T>, work: impl Fn(T) -> R + Sync) -> Vec<R> {
    let count = items.len();
    let queue = std::sync::Mutex::new(items.into_iter().enumerate());
    let done = std::sync::Mutex::new(Vec::with_capacity(count));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().expect("the queue").next();
                    let Some((index, item)) = next else {
                        break;
                    };
                    let result = work(item);
                    done.lock().expect("the results").push((index, result));
                }
            });
        }
    });
    let mut done = done.into_inner().expect("the results");
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, result)| result).collect()
}

/// The lowest drawn skin of `beast` with `clip` held at `time`, from rest.
fn lows(beast: &Beast, clip: &Clip, time: f32) -> Lows {
    let rig = &beast.avatar.rig;
    let mut pose = Pose::rest(rig);
    clip.apply(rig, &mut pose, time);
    lows_over(beast, &pose, 0.0, 0.0)
}

/// The lowest drawn skin of `beast` posed as `pose`, measured square off the
/// plane rising `grade` per metre ahead and `camber` across.
fn lows_over(beast: &Beast, pose: &Pose, grade: f32, camber: f32) -> Lows {
    let rig = &beast.avatar.rig;
    let up = Vec3::new(-camber, 1.0, -grade).normalize().y;
    let body = &beast.avatar.parts.body;
    let weights = &beast.avatar.parts.weights;
    let drawn = pose.forward(rig).deform(rig, &body.positions, weights);
    let off = |at: Vec3| (at.y - at.z * grade - at.x * camber) * up;
    let mut lows = Lows {
        haunch: f32::MAX,
        hind_feet: f32::MAX,
        fore_feet: f32::MAX,
        fore_legs: f32::MAX,
        chest: f32::MAX,
        all: f32::MAX,
    };
    for (vertex, at) in drawn.iter().enumerate() {
        let joint = &rig.joints[usize::from(weights.vertices[vertex][0].joint)];
        // The root of a tail set on the pelvis is the rump's, under which
        // the ischia sit: a slight cat sitting bolt upright rests on it.
        let rump = joint.zone == Zone::Tail
            && joint
                .parent
                .is_some_and(|parent| rig.joints[parent].zone == Zone::Pelvis);
        let zone = if rump { Zone::Pelvis } else { joint.zone };
        let low = |slot: &mut f32| *slot = slot.min(off(*at));
        low(&mut lows.all);
        match zone {
            Zone::Pelvis => low(&mut lows.haunch),
            Zone::UpperLimb(limb) if !limb.is_fore() => low(&mut lows.haunch),
            Zone::LowerLimb(limb) | Zone::Extremity(limb) if !limb.is_fore() => {
                low(&mut lows.hind_feet);
            }
            Zone::Extremity(_) => low(&mut lows.fore_feet),
            Zone::UpperLimb(_) | Zone::LowerLimb(_) => low(&mut lows.fore_legs),
            Zone::Chest => low(&mut lows.chest),
            _ => {}
        }
    }
    lows
}

/// Every way of going into and out of a rest a beast has, by name: the two
/// states down, and getting up from each.
fn motions(beast: &Beast) -> Vec<(String, Clip)> {
    let rig = &beast.avatar.rig;
    let mut motions = Vec::new();
    for name in ["Sit", "Lie Down"] {
        let down = gesture::for_body(rig, name).expect("a beast's state");
        motions.push((name.to_string(), down));
        let (up, _) = emote::getting_up(rig, name).expect("a beast gets up");
        motions.push((format!("getting up from {name}"), up));
    }
    motions
}

#[test]
fn nothing_of_a_resting_beast_goes_below_the_ground() {
    // Through the whole of each motion, both ways, at twenty moments.
    let mut worst: Vec<String> = Vec::new();
    for beast in beasts() {
        let allowed = (SINK * beast.withers).max(SINK_FLOOR);
        for (name, clip) in motions(&beast) {
            for step in 0..=20 {
                #[expect(clippy::cast_precision_loss, reason = "twenty moments")]
                let time = step as f32 / 20.0;
                let low = lows(&beast, &clip, time).all;
                if low < -allowed {
                    worst.push(format!(
                        "{}: {name} at {time:.2} reaches {:.0} mm below the ground",
                        beast.what,
                        -low * 1000.0
                    ));
                }
            }
        }
    }
    assert!(worst.is_empty(), "{}", worst.join("\n"));
}

#[test]
fn a_sitting_beast_rests_on_its_haunches_its_hind_feet_and_its_forefeet() {
    // 09 fact 3: the ischia, the flat metatarsi and the forepaws, and
    // nothing else; a horse's held dog-sit on its folded hind legs and its
    // forehooves, its croup on the ground or just off it.
    let mut wrong: Vec<String> = Vec::new();
    for beast in beasts() {
        let sat = gesture::for_body(&beast.avatar.rig, "Sit").expect("a beast sits");
        let lows = lows(&beast, &sat, 1.0);
        let touch = TOUCH * beast.withers;
        let haunch = if beast.hooves {
            JUST_OFF * beast.withers
        } else {
            touch
        };
        let sunk = (SINK * beast.withers).max(SINK_FLOOR);
        for (part, low, most) in [
            ("haunches", lows.haunch, haunch),
            ("hind feet", lows.hind_feet, touch),
            ("forefeet", lows.fore_feet, touch),
        ] {
            // On the ground: neither off it nor in it.
            if low > most || low < -sunk {
                wrong.push(format!(
                    "{}: its {part} rest {:+.0} mm from the ground",
                    beast.what,
                    low * 1000.0
                ));
            }
        }
        // And sitting, not lying: the chest stands clear.
        if lows.chest < touch {
            wrong.push(format!("{}: it sits on its chest", beast.what));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_lying_beast_rests_on_its_chest_its_forelegs_and_its_folded_hind_legs() {
    // 09, 2.1.4 and 2.2.3: a dog's and a cat's sphinx, sternum, forearms
    // and folded hind legs on the ground; 2.3.1: a horse on its folded legs,
    // knees and hind legs on the ground, its chest just up off it.
    let mut wrong: Vec<String> = Vec::new();
    for beast in beasts() {
        let lay = gesture::for_body(&beast.avatar.rig, "Lie Down").expect("a beast lies down");
        let lows = lows(&beast, &lay, 1.0);
        let touch = TOUCH_LYING * beast.withers;
        let chest = if beast.hooves {
            HORSE_CHEST
        } else {
            TOUCH_LYING
        } * beast.withers;
        let parts = [
            ("forelegs", lows.fore_legs, touch),
            ("hind feet", lows.hind_feet, touch),
            ("chest", lows.chest, chest),
        ];
        let sunk = (SINK * beast.withers).max(SINK_FLOOR);
        for (part, low, most) in parts {
            // On the ground, or for a horse's chest near it: neither off it
            // nor in it.
            if low > most || low < -sunk {
                wrong.push(format!(
                    "{}: its {part} rest {:+.0} mm from the ground",
                    beast.what,
                    low * 1000.0
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_resting_beast_moves_into_and_out_of_it_without_a_jump() {
    // #377: every motion a continuous function of its time, and the driver
    // that plays them continuous frame to frame. The lying down took over
    // from the sit by placing the trunk again by its own rule, and a dog's
    // whole body jumped 161.5 mm one way going down and back getting up; a
    // horse getting up found its loin's bend at the end of its range one
    // frame and not the next, and its head jumped 906 mm.
    let wrong: Vec<String> = in_parallel(beasts(), |beast| {
        let rig = &beast.avatar.rig;
        let mut wrong = Vec::new();
        for name in ["Sit", "Lie Down"] {
            let down = gesture::for_body(rig, name).expect("a beast's state");
            let (up, rising) = emote::getting_up(rig, name).expect("a beast gets up");
            let going = emote::seconds(rig, name).expect("a state keeps its own time");
            for (way, clip, seconds) in [("going down", &down, going), ("getting up", &up, rising)] {
                #[expect(clippy::cast_precision_loss, reason = "a few hundred steps")]
                let at = |step: usize| clip.pose(rig, step as f32 / STEPS as f32).forward(rig);
                #[expect(clippy::cast_precision_loss, reason = "a few hundred steps")]
                let most = swift(beast.withers, seconds / STEPS as f32);
                let mut before = at(0);
                for step in 1..=STEPS {
                    let after = at(step);
                    let (moved, joint) = fastest(rig, &before, &after);
                    if moved > most {
                        #[expect(clippy::cast_precision_loss, reason = "a few hundred steps")]
                        let time = step as f32 / STEPS as f32;
                        wrong.push(format!(
                            "{}: {name} {way}: joint {joint} ({:?}) moved {:.1} mm at {time:.3}, past {:.1}",
                            beast.what,
                            rig.joints[joint].zone,
                            moved * 1000.0,
                            most * 1000.0
                        ));
                        break;
                    }
                    before = after;
                }
            }
            // And the driver at sixty frames a second: down, held half a
            // second, and up again where it lies, at the unhurried pace.
            let frame = 1.0 / 60.0;
            let mut driver = Driver::seeded(377);
            let mut drive = |gesture: Option<&str>| {
                driver
                    .drive(
                        rig,
                        &Inputs {
                            delta: frame,
                            gesture,
                            ..Inputs::default()
                        },
                        level_ground,
                    )
                    .expect("an unheld body is posed every frame")
                    .pose
                    .forward(rig)
            };
            #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
            let held = 30 + ((going + 0.5) / frame) as usize;
            #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
            let frames = held + ((rising + 1.0) / frame) as usize;
            let most = swift(beast.withers, frame);
            let mut before = drive(None);
            for at in 1..frames {
                let asked = match at {
                    30 => Some(name),
                    at if at == held => Some(emote::GET_UP),
                    _ => None,
                };
                let after = drive(asked);
                let (moved, joint) = fastest(rig, &before, &after);
                if moved > most {
                    wrong.push(format!(
                        "{}: {name} driven: joint {joint} ({:?}) moved {:.1} mm in frame {at}, past {:.1}",
                        beast.what,
                        rig.joints[joint].zone,
                        moved * 1000.0,
                        most * 1000.0
                    ));
                    break;
                }
                before = after;
            }
        }
        wrong
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The plane rising `grade` per metre ahead and `camber` across.
fn plane(grade: f32, camber: f32) -> impl Fn(Vec3) -> Option<Ground> + Copy {
    let normal = Vec3::new(-camber, 1.0, -grade).normalize();
    move |at: Vec3| {
        Some(Ground {
            position: Vec3::new(at.x, at.x * camber + at.z * grade, at.z),
            normal,
        })
    }
}

/// One frame of a body seated `through` of the way down `name`, on `ground`.
fn seated<F>(beast: &Beast, name: &str, through: f32, ground: F) -> Pose
where
    F: Fn(Vec3) -> Option<Ground> + Copy,
{
    let rig = &beast.avatar.rig;
    let mut driver = Driver::seeded(381);
    assert!(driver.seat(rig, name, through), "{name} is a beast's state");
    let inputs = Inputs {
        delta: 1.0e-4,
        ..Inputs::default()
    };
    driver
        .drive(rig, &inputs, ground)
        .expect("an unheld body is posed")
        .pose
}

#[test]
fn a_beast_rests_on_a_plane_as_it_rests_on_level_ground() {
    // #381: on a 20% grade a sitting dog's hind pads sank 19 mm and a horse's 81
    // to 93 while its forepads floated 49 and 192. Off the plane, square to it,
    // every part reads what it reads on the level, which the guards above
    // judge; both ways up a grade, across one and on a corner of both.
    let wrong: Vec<String> = in_parallel(beasts(), |beast| {
        let mut wrong = Vec::new();
        for name in ["Sit", "Lie Down"] {
            let level = lows_over(&beast, &seated(&beast, name, 1.0, level_ground), 0.0, 0.0);
            for (grade, camber) in [(0.2, 0.0), (-0.2, 0.0), (0.0, 0.2), (0.3, 0.3), (-0.4, 0.0)] {
                let pose = seated(&beast, name, 1.0, plane(grade, camber));
                let sloped = lows_over(&beast, &pose, grade, camber);
                for (part, on, off) in [
                    ("haunches", level.haunch, sloped.haunch),
                    ("hind feet", level.hind_feet, sloped.hind_feet),
                    ("forefeet", level.fore_feet, sloped.fore_feet),
                    ("forelegs", level.fore_legs, sloped.fore_legs),
                    ("chest", level.chest, sloped.chest),
                    ("whole body", level.all, sloped.all),
                ] {
                    if (on - off).abs() > ON_A_PLANE {
                        wrong.push(format!(
                            "{}: {name} on {grade} by {camber}: its {part} read {:+.1} mm off \
                             the plane against {:+.1} on the level",
                            beast.what,
                            off * 1000.0,
                            on * 1000.0
                        ));
                    }
                }
            }
        }
        wrong
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far a part of a held rest may read off the plane from what it reads on
/// the level, in metres. Provenance: **measured** (#381): the rest is turned
/// whole, so what is left is float rounding, 0.1 mm at most.
const ON_A_PLANE: f32 = 1.0e-3;

/// The families and a cross of two, for what is too dear to sweep over every
/// body: built, and their withers heights.
fn few() -> Vec<Beast> {
    beasts()
        .into_iter()
        .filter(|beast| {
            beast.what.ends_with("appeal 1 mass 0 fat None") || beast.what.contains("crossed")
        })
        .collect()
}

#[test]
fn a_beast_goes_down_and_gets_up_on_a_plane_out_of_it_and_without_a_jump() {
    // #381, through the driver at sixty frames a second, from when the blend
    // from its idle is done until the beast is up: the idle's own feet on a slope
    // are not a rest's to answer for.
    let wrong: Vec<String> = in_parallel(few(), |beast| {
        let rig = &beast.avatar.rig;
        let mut wrong = Vec::new();
        let frame = 1.0 / 60.0;
        let allowed = (SINK * beast.withers).max(SINK_FLOOR);
        let most = swift(beast.withers, frame);
        for name in ["Sit", "Lie Down"] {
            let going = emote::seconds(rig, name).expect("a state keeps its own time");
            for (grade, camber) in [(0.2, 0.0), (-0.2, 0.0), (0.0, 0.2)] {
                let ground = plane(grade, camber);
                let mut driver = Driver::seeded(381);
                let drive = |gesture: Option<&str>, driver: &mut Driver| {
                    driver
                        .drive(
                            rig,
                            &Inputs {
                                delta: frame,
                                gesture,
                                ..Inputs::default()
                            },
                            ground,
                        )
                        .expect("an unheld body is posed")
                        .pose
                };
                for _ in 0..30 {
                    drive(None, &mut driver);
                }
                let mut before = drive(Some(name), &mut driver).forward(rig);
                #[expect(clippy::cast_possible_truncation, reason = "a few hundred frames")]
                let held = ((going + 0.5) / frame) as usize;
                let mut at = 0usize;
                while driver.gesture().is_some() && at < 3000 {
                    at += 1;
                    let pose = drive((at == held).then_some(emote::GET_UP), &mut driver);
                    let after = pose.forward(rig);
                    let (moved, joint) = fastest(rig, &before, &after);
                    if moved > most {
                        wrong.push(format!(
                            "{}: {name} on {grade} by {camber}: joint {joint} moved {:.0} mm in \
                             frame {at}, past {:.0}",
                            beast.what,
                            moved * 1000.0,
                            most * 1000.0
                        ));
                    }
                    before = after;
                    // Past the blend in, and on all of the rest of it.
                    if at > 20 && driver.gesture().is_some() {
                        let low = lows_over(&beast, &pose, grade, camber).all;
                        if low < -allowed {
                            wrong.push(format!(
                                "{}: {name} on {grade} by {camber}: {:.0} mm into it in frame \
                                 {at}, past {:.0}",
                                beast.what,
                                -low * 1000.0,
                                allowed * 1000.0
                            ));
                        }
                    }
                }
                assert!(at < 3000, "{}: {name} never got up", beast.what);
            }
        }
        wrong
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_grade_taken_toward_nothing_comes_to_the_rest_on_the_level() {
    // No jump at the slider's middle: level ground is left alone to the bit,
    // and the least of a grade must be as near that as it is near level.
    for beast in few() {
        let rig = &beast.avatar.rig;
        for name in ["Sit", "Lie Down"] {
            for through in [0.3, 1.0] {
                let level = seated(&beast, name, through, level_ground).forward(rig);
                let nearly = seated(&beast, name, through, plane(1.0e-4, 0.0)).forward(rig);
                let (moved, joint) = fastest(rig, &level, &nearly);
                assert!(
                    moved < 1.0e-3,
                    "{}: {name} at {through}: joint {joint} stood {:.1} mm off",
                    beast.what,
                    moved * 1000.0
                );
            }
        }
    }
}

#[test]
fn a_head_stays_level_when_a_rest_turns_onto_a_slope() {
    // The trunk takes the ground's slope and the neck takes it back off.
    for beast in few() {
        let rig = &beast.avatar.rig;
        let head = rig.in_zone(Zone::Head)[0];
        for name in ["Sit", "Lie Down"] {
            let level = seated(&beast, name, 1.0, level_ground).forward(rig);
            let turned = seated(&beast, name, 1.0, plane(0.3, 0.2)).forward(rig);
            let off = level.rotations[head].angle_between(turned.rotations[head]);
            assert!(
                off < 1.0e-2,
                "{}: {name}: the head turned {off:.3} rad with the ground",
                beast.what
            );
        }
    }
}

#[test]
fn each_foot_of_a_rest_reads_the_ground_under_itself() {
    // Ground that is no plane: the body fits what it can, and each foot is
    // sent to its own height, as far off it as it stands off the level.
    let height = |x: f32, z: f32| 0.05 * (2.0 * z).sin() + 0.03 * (3.0 * x).sin();
    let ground = move |at: Vec3| {
        let normal = Vec3::new(-0.09 * (3.0 * at.x).cos(), 1.0, -0.1 * (2.0 * at.z).cos());
        Some(Ground {
            position: Vec3::new(at.x, height(at.x, at.z), at.z),
            normal: normal.normalize(),
        })
    };
    for beast in few() {
        let rig = &beast.avatar.rig;
        for name in ["Sit", "Lie Down"] {
            let level = seated(&beast, name, 1.0, level_ground).forward(rig);
            let bumpy = seated(&beast, name, 1.0, ground).forward(rig);
            for limb in rig.ground_contacts() {
                let pad = rig.in_zone(Zone::Extremity(limb))[0];
                let at = bumpy.positions[pad];
                let off = at.y - height(at.x, at.z) - level.positions[pad].y;
                assert!(
                    off.abs() < 2.0e-3,
                    "{}: {name}: {limb:?} stood {:+.1} mm off its own ground",
                    beast.what,
                    off * 1000.0
                );
            }
        }
    }
}

/// The body #390 was found on and its neighbours: short legs under a long or a
/// middling trunk, a horse's, whose tibia folds least far under its femur.
fn short_legged() -> Vec<Beast> {
    let mut records = Vec::new();
    for leg in [-3.0f32, -2.0] {
        for trunk in [0.0f32, 3.0] {
            let mut params = QuadrupedParams::default();
            params.set_kind(Family::Horse, None, 0.0);
            params.leg_length = leg;
            params.body_length = trunk;
            records.push((format!("horse leg {leg} trunk {trunk}"), params));
        }
    }
    in_parallel(records, |(what, params)| {
        let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
        record.sanitize();
        let Archetype::Quadruped(params) = &record.archetype else {
            unreachable!("every record here is a beast")
        };
        let withers = params.height * 1.2;
        let avatar = Avatar::build_for_shape(&record).unwrap_or_else(|| panic!("{what} builds"));
        Beast {
            what,
            avatar,
            withers,
            hooves: true,
        }
    })
}

#[test]
fn a_short_legged_beast_gets_up_from_a_sit_with_its_thigh_out_of_the_ground() {
    // #390: hips rising over a hind leg folded this tight fall the stifle four
    // times as far, so each pass of the settle sank the wrapped thigh further:
    // 20.5 mm on legLength -3 and 23.7 on -2 at a long trunk, 2.4 and 2.7 times
    // what is allowed, the trunk climbing 100 mm in the frame it gave up.
    let wrong: Vec<String> = in_parallel(short_legged(), |beast| {
        let rig = &beast.avatar.rig;
        let (up, _) = emote::getting_up(rig, "Sit").expect("a beast gets up");
        let allowed = (SINK * beast.withers).max(SINK_FLOOR);
        let mut wrong = Vec::new();
        let mut before: Option<Posed> = None;
        for step in 0..=100 {
            // The first third of it, where the hips open over the closed leg.
            #[expect(clippy::cast_precision_loss, reason = "a hundred steps")]
            let time = step as f32 / 300.0;
            let low = lows(&beast, &up, time).all;
            if low < -allowed {
                wrong.push(format!(
                    "{}: at {time:.3} reaches {:.1} mm below the ground, past {:.1}",
                    beast.what,
                    -low * 1000.0,
                    allowed * 1000.0
                ));
            }
            let now = up.pose(rig, time).forward(rig);
            if let Some(before) = &before {
                let (moved, joint) = fastest(rig, before, &now);
                if moved > FOLDED_STEP {
                    wrong.push(format!(
                        "{}: at {time:.3} joint {joint} moved {:.1} mm, past {:.1}",
                        beast.what,
                        moved * 1000.0,
                        FOLDED_STEP * 1000.0
                    ));
                }
            }
            before = Some(now);
        }
        wrong
    })
    .into_iter()
    .flatten()
    .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// How far any joint of a short-legged horse may move in a three-hundredth of
/// its rise from a sit, in metres, over the first third of it. Provenance:
/// **measured** (#390): 89 mm where the trunk climbed in vain, 62 since, a neck
/// joint carried on as the trunk swings up. A last-place change in libm alone reads
/// up to 75 on this body, as the rise's own threshold falls the other way at a
/// sample, so the bound stands between that and the fault.
const FOLDED_STEP: f32 = 0.08;
