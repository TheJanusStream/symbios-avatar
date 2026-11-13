//! A winged body's rests and emotes (#424): what a bird, a bat and a sugar
//! glider do with themselves when they are not going anywhere.
//!
//! **Rests**, each the leading family's and each a state - gone into at the
//! body's own pace, held, and got up from as [`crate::anim::emote`]'s sit is:
//!
//! - a bird's **Perch**: crouched on its toes, the hocks and knees folded so
//!   the belly settles over its feet, the toes curled into a grip
//!   ([`Target::Grip`]), the wings folded and the head level (research-10
//!   6.2: "toes wrap the perch ... ankle and knee flex into a crouch so the
//!   belly feathers settle over the feet");
//! - its **Roost**: lower still, the tarsi folded under it, and the head
//!   turned back and laid into the scapulars over its left shoulder
//!   ([`Target::Tuck`]; 6.2: "head turned back and tucked into the
//!   scapulars (not under the wing)");
//! - a bat's **Rest** on the ground and a glider's **Curl**, one motion
//!   ([`Target::Curl`]): down on the belly - as low as its legs, its tail
//!   and its folded membrane let it, every drawn point kept out of the
//!   ground - with the legs folded under, a bat's wrists drawn in close to
//!   its shoulders and its head down on them, a glider curled round with its
//!   tail wrapped about it (6.2: "the curled pose (tail wrapped round)").
//!   **A cross between the two lies between them**, by its glider share:
//!   the families share the rest, and the curl and the drawn-in wrists are
//!   each the share of the family they are ([`crate::Kinship`]).
//!
//! A bird-led cross perches and roosts as a bird; a bat- or glider-led one
//! rests as its family. The bat's hang and the glider's cling are the
//! owner's later phase ("landing on the tops of things first").
//!
//! **Emotes**, which start and end at rest: a **Preen** (a bird's) or a
//! **Groom** (a bat's or a glider's), the head reaching back to the wing, a
//! bird's lifted to meet it; a **Wing Stretch** - a bird's one wing and the
//! leg on its side stretched down and back with the tail fanned, then both
//! wings up (6.2, S16), a bat's each wing out in turn - and a glider's
//! **Stretch**, a bow; a **Shake**, a bird's ruffle of its trunk and a flick
//! of its wings and tail, a bat's or a glider's as a dog shakes, all four
//! feet held. A bat's and a glider's wings are the forelegs it stands on,
//! and stay down but in its wing stretch; and a **Head Tilt**, a bird's head
//! canted to look up with one eye and swayed side to side as an owl judges
//! depth (6.2), a bat's or a glider's as a beast's.
//!
//! **None is played in the air** ([`super`]'s order): a rest or an emote
//! asked mid-flight is let go, and one playing ends when a flight begins.
//!
//! **The roster is the body's** ([`roster`]), its leading family's names, so
//! a host offers a bird its own and never a person's laid over its folded
//! wings; [`crate::anim::gesture::for_body`] answers only these names on a
//! winged body.
//!
//! **And a bird looks with its head** (research-10 fact 5): its eyes turn
//! their [`crate::face::eye::winged_range`] - an owl's three degrees - and
//! the driver's glance turns its head further and in a quick saccade
//! ([`glance_config`], [`head_saccade`]).
//!
//! Nothing here moves a person or a beast: every target this adds does
//! nothing on a body with no wings, and a winged body's names are answered
//! only on one.

use glam::{Quat, Vec3};

use crate::anim::clip::{Clip, Key, Target, Track};
use crate::anim::emote::GET_UP;
use crate::anim::gaze::GazeConfig;
use crate::anim::ground::{Foot, Ground, solve_contact_laid_by, solve_contact_with};
use crate::anim::pose::Pose;
use crate::det::{self, DetMath, Rot};
use crate::plan::{Limb, WingedFamily, Zone};
use crate::rig::{Rig, Role};
use crate::wing::Kinship;

/// A bird's roster: its two rests, the way up, and its emotes.
pub const BIRD: &[&str] = &[
    "Perch",
    "Roost",
    GET_UP,
    "Preen",
    "Wing Stretch",
    "Shake",
    "Head Tilt",
];

/// A bat's roster.
pub const BAT: &[&str] = &[
    "Rest",
    GET_UP,
    "Groom",
    "Wing Stretch",
    "Shake",
    "Head Tilt",
];

/// A sugar glider's roster.
pub const GLIDER: &[&str] = &["Curl", GET_UP, "Groom", "Stretch", "Shake", "Head Tilt"];

/// Every name any winged body's roster holds, each once.
pub const ROSTER: &[&str] = &[
    "Perch",
    "Roost",
    "Rest",
    "Curl",
    GET_UP,
    "Preen",
    "Groom",
    "Wing Stretch",
    "Stretch",
    "Shake",
    "Head Tilt",
];

/// The names `rig` can be asked for: its leading family's roster, or none on
/// a body with no wings.
#[must_use]
pub fn roster(rig: &Rig) -> &'static [&'static str] {
    match super::kinship(rig).map(|kin| kin.leading) {
        Some(WingedFamily::Bird) => BIRD,
        Some(WingedFamily::Bat) => BAT,
        Some(WingedFamily::Glider) => GLIDER,
        None => &[],
    }
}

/// The emote called `name` on `rig`, if its roster has one: see the module's
/// notes. [`GET_UP`] is an empty clip, as a beast's is.
#[must_use]
pub fn for_body(rig: &Rig, name: &str) -> Option<Clip> {
    let kin = super::kinship(rig)?;
    if !roster(rig).contains(&name) {
        return None;
    }
    Some(match name {
        GET_UP => Clip::new(Vec::new()),
        "Perch" => perch(drop(rig, PERCH_DROP)),
        "Roost" => roost(drop(rig, ROOST_DROP)),
        "Rest" | "Curl" => curled(),
        "Preen" | "Groom" => preen(kin),
        "Wing Stretch" if kin.leading == WingedFamily::Bird => bird_stretch(rig),
        "Wing Stretch" => bat_stretch(),
        "Stretch" => glider_stretch(rig),
        "Shake" => shake(kin),
        "Head Tilt" => head_tilt(kin),
        _ => return None,
    })
}

/// Whether `name` is one of a winged body's rests, which keep their own
/// time and are held until the body gets up.
fn is_state(name: &str) -> bool {
    matches!(name, "Perch" | "Roost" | "Rest" | "Curl")
}

/// How long the rest `name` takes `rig` to go into and to get up from, in
/// seconds, or `None` for an emote, a name its roster does not have, or a
/// body with no wings.
fn timing(rig: &Rig, name: &str) -> Option<(f32, f32)> {
    let kin = super::kinship(rig)?;
    if !is_state(name) || !roster(rig).contains(&name) {
        return None;
    }
    let [down, up] = match name {
        "Perch" => PERCH_TIME,
        "Roost" => ROOST_TIME,
        // A bat's rest and a glider's curl are one motion, timed by the
        // glider's share as it is curled by it.
        _ => {
            let share = glider_share(kin);
            [
                REST_TIME[0] + (CURL_TIME[0] - REST_TIME[0]) * share,
                REST_TIME[1] + (CURL_TIME[1] - REST_TIME[1]) * share,
            ]
        }
    };
    let scale = (crate::anim::speed::leg_of(rig) / TIME_LEG).max(0.0).sqrt();
    Some((down * scale, up * scale))
}

/// How long the rest `name` takes to go into on `rig`: see
/// [`crate::anim::emote::seconds`], which answers with this on a winged body.
#[must_use]
pub fn seconds(rig: &Rig, name: &str) -> Option<f32> {
    Some(timing(rig, name)?.0)
}

/// Getting up out of the rest `name` on `rig`, and how long it takes: the
/// way down played back, which is how a bird rises off its hocks and a
/// glider uncurls. See [`crate::anim::emote::getting_up`], which answers with this
/// on a winged body.
#[must_use]
pub fn getting_up(rig: &Rig, name: &str) -> Option<(Clip, f32)> {
    let (_, up) = timing(rig, name)?;
    let clip = for_body(rig, name)?;
    Some((backward(&clip), up))
}

/// `clip` played from its end to its start.
fn backward(clip: &Clip) -> Clip {
    let tracks: Vec<Track> = clip
        .tracks
        .iter()
        .map(|track| {
            let mut track = track.clone();
            for key in &mut track.keys {
                key.time = 1.0 - key.time;
            }
            track.keys.reverse();
            track
        })
        .collect();
    Clip::new(tracks)
}

/// How long a bird takes to settle onto its perch and to rise off it, and to
/// roost and to wake from it; a bat to lie down to rest and to get up, and a
/// glider to curl and to uncurl, in seconds, for a body with a leg of
/// [`TIME_LEG`].
/// Provenance: **chosen** (#424): research-10 and research-11 time none of
/// them; a perch's crouch is a bird's quick settle, a roost a slow one, and a
/// glider curls more slowly than a bat lies down because it turns its whole
/// body round to do it.
const PERCH_TIME: [f32; 2] = [0.6, 0.4];
const ROOST_TIME: [f32; 2] = [1.6, 0.9];
const REST_TIME: [f32; 2] = [1.2, 0.8];
const CURL_TIME: [f32; 2] = [2.2, 1.6];

/// The leg those times are stated for, in metres; they scale with the square
/// root of a body's own, as a beast's do (research-09 note 12).
const TIME_LEG: f32 = 0.25;

/// How far a perching bird's body comes down, in its standing leg's reach,
/// and a roosting one's.
/// Provenance: **tuned by render** (#424): the perch until the belly hangs
/// over the toes, the roost until it rests on them and the bird looks
/// legless (10 6.2).
const PERCH_DROP: f32 = 0.22;
const ROOST_DROP: f32 = 0.36;

/// How far a perching bird's toes close, `0..1`.
/// Provenance: **tuned by render** (#424).
const PERCH_GRIP: f32 = 1.0;

/// A track that eases in to `toward` by `arrive` and stays.
fn settled(target: Target, toward: Vec3, arrive: f32) -> Track {
    Track::new(
        target,
        [
            Key::new(0.0, Vec3::ZERO),
            Key::new(arrive, toward),
            Key::new(1.0, toward),
        ],
    )
}

/// A track that eases in to `toward` from `from` by `arrive`, holds it until
/// `leave`, and eases back out by `gone`.
fn held(target: Target, toward: Vec3, [from, arrive, leave, gone]: [f32; 4]) -> Track {
    let mut keys = vec![Key::new(0.0, Vec3::ZERO)];
    if from > 0.0 {
        keys.push(Key::new(from, Vec3::ZERO));
    }
    keys.extend([
        Key::new(arrive, toward),
        Key::new(leave, toward),
        Key::new(gone, Vec3::ZERO),
    ]);
    if gone < 1.0 {
        keys.push(Key::new(1.0, Vec3::ZERO));
    }
    Track::new(target, keys)
}

/// The keys of a sway `swings` times each way between `from` and `to`,
/// `size` at its widest along `along`, eased in and out from rest.
fn sway(from: f32, to: f32, swings: usize, size: f32, along: Vec3) -> Vec<Key> {
    let steps = swings * 2;
    #[expect(clippy::cast_precision_loss, reason = "a handful of keys")]
    let keys = (0..=steps).map(|step| {
        let share = step as f32 / steps as f32;
        let side = if step % 2 == 0 { -1.0 } else { 1.0 };
        let ease = (share * std::f32::consts::PI).det_sin();
        let size = if step == 0 || step == steps {
            0.0
        } else {
            size * ease
        };
        Key::new(from + (to - from) * share, along * (side * size))
    });
    std::iter::once(Key::new(0.0, Vec3::ZERO))
        .chain(keys)
        .chain(std::iter::once(Key::new(1.0, Vec3::ZERO)))
        .collect()
}

/// Both hind feet held where they stand, in the world, while the body moves
/// over them.
fn planted(limbs: &[Limb]) -> Vec<Track> {
    limbs
        .iter()
        .map(|&limb| {
            Track::new(Target::Just(limb), [Key::new(0.0, Vec3::ZERO)])
                .in_world()
                .standing()
        })
        .collect()
}

/// How far a perching body of `rig` comes down, in its standing leg's reach:
/// `asked`, but no further than its folded wing stands off the ground
/// ([`crate::Wings::low`]), so a bird-led cross whose long bat's wing hangs
/// low does not carry it into the ground - at the full roost, 197 mm in.
fn drop(rig: &Rig, asked: f32) -> f32 {
    let leg = crate::anim::speed::leg_of(rig);
    let Some(low) = rig.wings.as_ref().and_then(|wings| wings.low) else {
        return asked;
    };
    if leg <= f32::EPSILON {
        return asked;
    }
    asked.min(low.max(0.0) / leg)
}

/// A bird's perch, `drop` of its leg's reach down: see the module's notes.
/// **A state**.
///
/// **No gaze of its own**: the crouch lowers the body without turning it, so
/// the head stays level, and a perched bird goes on looking about in its
/// head saccades as it stands.
fn perch(drop: f32) -> Clip {
    let mut tracks = vec![
        settled(Target::Root, Vec3::NEG_Y * drop, 1.0),
        settled(Target::Grip, Vec3::Z * PERCH_GRIP, 0.7),
    ];
    tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    Clip::new(tracks)
}

/// A bird's roost, `drop` of its leg's reach down: see the module's notes.
/// **A state**. The head goes back
/// once the body has come most of the way down.
fn roost(drop: f32) -> Clip {
    let mut tracks = vec![
        settled(Target::Root, Vec3::NEG_Y * drop, 0.6),
        settled(Target::Grip, Vec3::Z * PERCH_GRIP, 0.5),
        Track::new(Target::Gaze, [Key::new(0.0, Vec3::ZERO)]).in_world(),
        Track::new(
            Target::Tuck,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(0.35, Vec3::ZERO),
                Key::new(1.0, Vec3::new(1.0, 0.0, 1.0)),
            ],
        ),
    ];
    tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    Clip::new(tracks)
}

/// A bat's rest and a glider's curl: see [`Target::Curl`]. **A state**.
fn curled() -> Clip {
    Clip::new([settled(Target::Curl, Vec3::Z, 1.0)])
}

/// Preening (a bird) or grooming (a bat, a glider): the head reaches back
/// round the left shoulder to the wing, which lifts a little away from the
/// body to meet it, and works along it in short nibbles.
fn preen(kin: Kinship) -> Clip {
    let bird = kin.leading == WingedFamily::Bird;
    // A bird turns its head further round to its wing than a bat or a
    // glider, whose wing is the leg beside it.
    let reach = if bird { PREEN_REACH[0] } else { PREEN_REACH[1] };
    // Round to the wing by a quarter of the way through, nibbling along it
    // until four fifths, and back.
    let mut tuck = vec![Key::new(0.0, Vec3::ZERO)];
    let nibbles = sway(0.25, 0.8, 4, PREEN_NIBBLE, Vec3::Y);
    for key in nibbles
        .iter()
        .filter(|key| key.time > 0.0 && key.time < 1.0)
    {
        // A bat or a glider grooms its flank level: its head, carried low,
        // dipped further went into the ground.
        let dip = if bird { PREEN_DIP } else { 0.0 };
        tuck.push(Key::new(
            key.time,
            Vec3::new(1.0, dip + key.offset.y, reach),
        ));
    }
    tuck.push(Key::new(1.0, Vec3::ZERO));
    let mut tracks = vec![Track::new(Target::Tuck, tuck)];
    if bird {
        tracks.push(held(
            Target::Wing(Limb::ForeLeft),
            PREEN_WING,
            [0.0, 0.25, 0.78, 0.95],
        ));
        tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    } else {
        // A bat's or a glider's wing is the foreleg it stands on: it stays
        // down, and the head comes round to it there.
        tracks.extend(planted(&Limb::ALL));
    }
    Clip::new(tracks)
}

/// How far round a preening head turns, a share of [`TUCK_TURN`]: a bird's,
/// and a bat's or a glider's to the wing beside it; how far it dips, in
/// quarter turns; and how far it nibbles up and down, likewise.
/// Provenance: **tuned by render** (#424).
const PREEN_REACH: [f32; 2] = [0.8, 0.62];
const PREEN_DIP: f32 = 0.18;
const PREEN_NIBBLE: f32 = 0.06;

/// How a preened wing is held: a little spread, lifted, swept back.
/// Provenance: **tuned by render** (#424).
const PREEN_WING: Vec3 = Vec3::new(0.12, 0.12, 0.0);

/// A bird's wing-and-leg stretch, then both wings up (10 6.2, S16): the left
/// wing spread down and back with the left leg stretched back under it and
/// the tail fanned, the bird standing on its right; then both wings raised
/// over the back.
///
/// **Laid down only as far as the folded wing stands clear**: a bird-led
/// cross whose long bat's wing hangs low stretches it lifted instead, which
/// laid down went 23 mm into the ground.
///
/// **And only as far as its feathers and its claws clear the ground**
/// (#440), on this body: the wing is raised past that until no flight
/// feather of it passes into the ground on its way out or held
/// ([`stretch_raise`]), and the stretched leg lifted by as far as its foot
/// stands in it ([`stretch_lift`]). How low the folded wing stands says
/// little of how far the spread one reaches: a Bird x Bat cross at 0.4, its
/// fold 86 mm clear, put its primaries 19 mm into the ground held level and
/// 62 laid down as a bird's is; and its foot, carried on a shank stretched
/// back, pointed its claws down into it, 11 mm there and 14 at 0.49, where
/// a bird's stand 80 mm over it.
fn bird_stretch(rig: &Rig) -> Clip {
    let clear = rig
        .wings
        .as_ref()
        .and_then(|wings| wings.low)
        .map_or(1.0, |low| (low / STRETCH_CLEAR).clamp(0.0, 1.0));
    let raise = STRETCH_DOWN.y * clear + STRETCH_LIFT * (1.0 - clear);
    let raise = stretch_raise(rig, raise);
    let down = Vec3::new(STRETCH_DOWN.x, raise, STRETCH_DOWN.z);
    // Each phase eased over a fifth of the gesture, so no joint runs faster
    // than a wing can be carried.
    let one = [0.0, 0.2, 0.32, 0.5];
    let both = [0.5, 0.68, 0.78, 0.98];
    let mut left = held(Target::Wing(Limb::ForeLeft), down, one);
    let up = held(Target::Wing(Limb::ForeLeft), STRETCH_UP, both);
    // One track, both stretches: the second's keys after the first's.
    left.keys.retain(|key| key.time <= one[3]);
    left.keys
        .extend(up.keys.into_iter().filter(|key| key.time > one[3]));
    // The same key for the right wing: [`wing`] mirrors a raise and a sweep.
    let right = held(Target::Wing(Limb::ForeRight), STRETCH_UP, both);
    // The knee kept forward and down, as a bird's is, so the solve never
    // turns the leg over as the foot passes behind the hip.
    // Lifted clear before it goes back, and set down from above as it
    // comes home: carried back along the ground, its toes dragged 28 mm
    // through it.
    let stretched = |leg: Vec3| {
        Track::new(
            Target::Just(Limb::HindLeft),
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(one[1] * 0.45, Vec3::Y * leg.y),
                Key::new(one[1], leg),
                Key::new(one[2], leg),
                Key::new(one[3] - (one[3] - one[2]) * 0.4, Vec3::Y * leg.y),
                Key::new(one[3], Vec3::ZERO),
                Key::new(1.0, Vec3::ZERO),
            ],
        )
    };
    let lift = stretch_lift(
        rig,
        |lift| Clip::new([stretched(STRETCH_LEG + Vec3::Y * lift)]),
        one[1],
    );
    Clip::new([
        left,
        right,
        stretched(STRETCH_LEG + Vec3::Y * lift),
        held(Target::Fan, Vec3::Z * STRETCH_FAN, one),
        Track::new(Target::Just(Limb::HindRight), [Key::new(0.0, Vec3::ZERO)])
            .in_world()
            .standing(),
        held(Target::Gaze, Vec3::Y * 0.15, both),
    ])
}

/// How a bird stretches one wing - spread most of the way, laid down and
/// swept back - and how far up it raises both, in [`Target::Wing`]'s units;
/// where its stretched leg goes, in the leg's reach, back and a little up;
/// and how far its tail fans.
/// Provenance: **tuned by render** (#424), after 10 6.2: "extended down and
/// back together", "a both-wings-up stretch to about 90 deg".
const STRETCH_DOWN: Vec3 = Vec3::new(0.75, -0.1, -0.2);
const STRETCH_UP: Vec3 = Vec3::new(0.5, 0.75, 0.0);
const STRETCH_LEG: Vec3 = Vec3::new(0.15, 0.25, -0.6);
const STRETCH_FAN: f32 = 0.9;

/// How far off the ground a folded wing must stand to be laid all the way
/// down in a stretch, in metres; one nearer is laid down by its share of it.
const STRETCH_CLEAR: f32 = 0.12;

/// How far a stretched wing that hangs to the ground folded is lifted
/// instead, in quarter turns: the most any stretch is raised.
const STRETCH_LIFT: f32 = 0.4;

/// How much further a stretched wing is raised at a time until its feathers
/// clear the ground, in quarter turns, and where along its way out it is
/// read, as shares of the stretch.
const STRETCH_STEP: f32 = 0.02;
const STRETCH_PATH: [f32; 5] = [0.5, 0.625, 0.75, 0.875, 1.0];

/// How far a bird's stretched left wing is raised, in quarter turns: `raise`,
/// and past it by [`STRETCH_STEP`] at a time, to [`STRETCH_LIFT`] at the
/// most, until no corner of a flight feather of it ([`crate::wing::vanes`])
/// stands under the level ground along [`STRETCH_PATH`]. `raise` itself on a
/// wing whose feathers clear it there, and on a body with none.
fn stretch_raise(rig: &Rig, raise: f32) -> f32 {
    let limb = Limb::ForeLeft;
    let vanes: Vec<crate::wing::Vane> = crate::wing::vanes(rig)
        .into_iter()
        .filter(|vane| rig.joints[vane.joint].zone == Zone::Extremity(limb))
        .collect();
    if vanes.is_empty() {
        return raise;
    }
    let under = |raise: f32| {
        STRETCH_PATH.iter().any(|&share| {
            let mut pose = Pose::rest(rig);
            let out = Vec3::new(STRETCH_DOWN.x, raise, STRETCH_DOWN.z) * share;
            wing(rig, &mut pose, limb, out);
            let posed = pose.forward(rig);
            vanes.iter().any(|vane| {
                vane.posed(&posed.positions, &posed.rotations)
                    .any(|corner| corner.y < 0.0)
            })
        })
    };
    let mut raise = raise;
    while raise < STRETCH_LIFT && under(raise) {
        raise = (raise + STRETCH_STEP).min(STRETCH_LIFT);
    }
    raise
}

/// How far a bird's stretched leg is lifted over where [`STRETCH_LEG`] puts
/// it, in the leg's reach: by as far as any joint of its foot - its pad, its
/// toes and its claws' tips - stands under the height it rests at, `held`
/// through the clip `stretched` makes of a lift, sought three times over.
/// Nothing on a foot that stands clear, as a bird's does.
fn stretch_lift(rig: &Rig, stretched: impl Fn(f32) -> Clip, held: f32) -> f32 {
    let limb = Limb::HindLeft;
    let Some(reach) = rig.limb_reach(limb).filter(|reach| *reach > f32::EPSILON) else {
        return 0.0;
    };
    let foot = rig.extremity_joints(limb);
    let mut lift = 0.0f32;
    for _ in 0..3 {
        let mut pose = Pose::rest(rig);
        stretched(lift).apply(rig, &mut pose, held);
        let posed = pose.forward(rig);
        let short = foot
            .iter()
            .map(|&joint| rig.joints[joint].position.y - posed.positions[joint].y)
            .fold(0.0f32, f32::max);
        if short <= 0.0 {
            break;
        }
        lift += short / reach;
    }
    lift
}

/// A bat's stretch: each wing opened out to the side and a little up in
/// turn, the body holding on the other three.
fn bat_stretch() -> Clip {
    // Lifted as it opens, so the membrane between the wrist and the ankle
    // clears the ground.
    let wing = Vec3::new(0.45, 0.3, 0.0);
    let mut tracks = vec![
        held(Target::Wing(Limb::ForeLeft), wing, [0.0, 0.22, 0.3, 0.5]),
        held(Target::Wing(Limb::ForeRight), wing, [0.5, 0.72, 0.8, 1.0]),
    ];
    tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    Clip::new(tracks)
}

/// A glider's stretch: a bow, its forepaws walked out ahead on the ground
/// and its rump up, its forehand sunk as far as its forelegs fold
/// ([`bow_sink`]). Its patagium stays folded: opening it turns the foreleg it
/// hangs from off the ground.
fn glider_stretch(rig: &Rig) -> Clip {
    bowed(bow_sink(rig))
}

/// [`glider_stretch`] with its trunk turned `sink` quarter turns nose down.
fn bowed(sink: f32) -> Clip {
    let span = [0.0, 0.25, 0.7, 0.95];
    let mut tracks = vec![
        held(Target::Pitch, Vec3::NEG_Z * sink, span),
        held(Target::Gaze, Vec3::Y * 0.35, span),
        held(Target::Tail, Vec3::Y * 0.4, span),
    ];
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        tracks.push(
            held(Target::Just(limb), Vec3::Z * GLIDER_BOW[1], span)
                .in_world()
                .standing(),
        );
    }
    tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    Clip::new(tracks)
}

/// How far a glider-led body's bow turns its trunk nose down, in quarter
/// turns: [`GLIDER_BOW`]'s, but no further than leaves its foreleg folded to
/// [`BOW_FOLD`] of its reach at the bow's depth on the level, sought by
/// halving - as [`drop`] keeps a perch over its folded wing.
///
/// **A foreleg folded nearly shut has no elbow to speak of** (#451's
/// critic). A glider's own folds to 0.29 of its reach; a cross with a bird's
/// shoulder stands higher on a shorter foreleg, already folded to 0.37 of it
/// standing, and its full bow folded it to 0.17 (Glider x Bird 0.35) and,
/// on the downhill side of a 20 degree slope, to 0.08. There the line from
/// its shoulder to its wrist is short enough to swing round the shoulder as
/// the paw walks out, and turns to stand opposite the way the elbow bends
/// (`Rig::bend_pole`), where the solve's bend plane is undetermined: the
/// elbow went from bent back to bent out in one frame as the bow arrived or
/// left, a fore upper limb turning 1.15 rad a sixtieth of a second at 60
/// frames a second (Glider x Bird 0.3) and 1.71 at 144 (0.35), across the
/// game's slope, against the glider's 0.07. Sunk only as far as leaves the
/// fold open, no cross's bow turns a joint faster than the glider's on the
/// same ground by more than a tenth of a radian. A glider bows as before; a
/// Glider x Bird 0.2 sinks 0.072, one at 0.3 half as far as a glider (0.05)
/// and one at 0.45 a fifth (0.019), its bow the paws walked out, the head
/// raised and the tail up.
fn bow_sink(rig: &Rig) -> f32 {
    let limb = Limb::ForeLeft;
    let (Some(chain), Some(reach)) = (rig.limb_chain(limb), rig.limb_reach(limb)) else {
        return GLIDER_BOW[0];
    };
    if reach <= f32::EPSILON {
        return GLIDER_BOW[0];
    }
    // The fold at the bow's depth, of the trunk's turn and that foreleg's
    // walk alone: nothing else in the clip moves the foreleg.
    let open = |sink: f32| {
        let mut clip = bowed(sink);
        clip.tracks
            .retain(|track| track.target == Target::Pitch || track.target == Target::Just(limb));
        let mut pose = Pose::rest(rig);
        clip.apply(rig, &mut pose, 0.5);
        let posed = pose.forward(rig);
        posed.positions[chain[0]].distance(posed.positions[chain[2]]) / reach >= BOW_FOLD
    };
    if open(GLIDER_BOW[0]) {
        return GLIDER_BOW[0];
    }
    let (mut deep, mut shallow) = (GLIDER_BOW[0], 0.0f32);
    for _ in 0..BOW_HALVINGS {
        let between = 0.5 * (deep + shallow);
        if open(between) {
            shallow = between;
        } else {
            deep = between;
        }
    }
    shallow
}

/// How far a glider's bow turns its trunk nose down at most, in quarter
/// turns ([`bow_sink`]), and how far its forepaws walk out ahead, in the
/// leg's reach, stood on the ground.
/// Provenance: **tuned** (#424) to the glider's short forelimb: at a beast's
/// play bow's sink and reach, its paws went 62 to 110 mm into the ground.
/// Raised to **a bow that reads** at the look review, where 0.05 and 0.15
/// moved the body 8% of its height: its paws stood on the ground, the
/// forehand goes further down.
const GLIDER_BOW: [f32; 2] = [0.1, 0.22];

/// How far in a bow on the level a glider-led body's foreleg may fold at
/// most: its wrist's distance from its shoulder, in the leg's reach.
/// Provenance: **measured** (#451's critic), over Glider x Bird crosses at
/// 0.2 to 0.45 named both ways on a 20 degree slope - up, down and across
/// both ways, as a plane and as the game hands it - at 30, 60 and 144 frames
/// a second: at 0.22 a Glider x Bird 0.2 going down the game's slope still
/// turned a fore upper limb 0.24 rad a sixtieth, at 0.25 none is past the
/// glider's own by a tenth of a radian, from 15 to 22.5 degrees. Under the
/// glider's own fold (0.29), so a glider bows as it did.
const BOW_FOLD: f32 = 0.25;

/// How many times [`bow_sink`] halves the sink it seeks: to a sixty-fourth
/// of [`GLIDER_BOW`]'s.
const BOW_HALVINGS: usize = 6;

/// Shaking: a bird ruffles its trunk side to side about its own line, its
/// head going first, then flicks its wings and its tail; a bat or a glider
/// braces and shakes as a dog does, every foot held where it stands.
fn shake(kin: Kinship) -> Clip {
    let bird = kin.leading == WingedFamily::Bird;
    let roll = if bird { SHAKE_ROLL[0] } else { SHAKE_ROLL[1] };
    // A bat or a glider shakes in three slower swings, so a roll wide
    // enough to read carries no joint faster than a frame allows.
    let (swings, end) = if bird { (4, 0.62) } else { (3, 0.8) };
    let mut tracks = vec![
        Track::new(
            Target::Cant,
            sway(0.05, end - 0.02, swings, SHAKE_CANT, Vec3::Z),
        ),
        Track::new(Target::Roll, sway(0.08, end, swings, roll, Vec3::Z)),
    ];
    let flick = sway(0.6, 0.9, 2, 0.12, Vec3::X);
    // Spread a little and flicked, never folded past the fold.
    let open = |keys: &[Key]| -> Vec<Key> {
        keys.iter()
            .map(|key| {
                let on = key.time > 0.6 && key.time < 0.9;
                let base = if on { 0.14 } else { 0.0 };
                // Lifted as it opens, so a long wing's hand clears the ground.
                Key::new(key.time, Vec3::new(base + key.offset.x, base, 0.0))
            })
            .collect()
    };
    if bird {
        tracks.push(Track::new(Target::Wing(Limb::ForeLeft), open(&flick)));
        tracks.push(Track::new(Target::Wing(Limb::ForeRight), open(&flick)));
        tracks.push(held(Target::Fan, Vec3::Z * 0.7, [0.62, 0.7, 0.78, 0.9]));
        tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    } else {
        // **Every foot held where it stands**, as a beast's shake holds them:
        // a bat's and a glider's wings are the forelegs it stands on, so they
        // stay planted and do not flick - braced and lowered with its hind
        // feet alone, a bat's wrist went 75 mm into the ground.
        tracks.push(held(
            Target::Root,
            Vec3::Y * SHAKE_BRACE,
            [0.0, 0.1, 0.85, 0.95],
        ));
        tracks.push(Track::new(Target::Tail, sway(0.15, 0.95, 4, 0.3, Vec3::X)));
        // The head whipping side to side ahead of the trunk, as a dog's
        // shake starts at its head: its feet held, a glider's forehand rolls
        // little, and the shake read only in its head.
        tracks.push(Track::new(
            Target::Gaze,
            sway(0.03, end - 0.04, swings, SHAKE_HEAD, Vec3::X),
        ));
        tracks.extend(planted(&Limb::ALL));
    }
    Clip::new(tracks)
}

/// How far a shaking body rolls its trunk each way at the widest, in quarter
/// turns - a bird's, and a bat's or a glider's - and cants its head.
/// Provenance: **tuned by render** (#424, the look review): at a tenth of a
/// quarter turn a glider's shake moved its body 5.6% of its height and read
/// as standing still; a dog shaking dry rolls its whole forehand.
const SHAKE_ROLL: [f32; 2] = [0.18, 0.2];
const SHAKE_CANT: f32 = 0.32;

/// How far a shaking bat's or glider's head whips each way, as a gaze
/// tangent.
const SHAKE_HEAD: f32 = 0.45;

/// How far a shaking bat or glider rises on its legs, in its standing leg's
/// reach: braced up rather than down as a beast braces, since its folded
/// membrane hangs near the ground and the roll swings it lower on one side
/// (`tests/winged_rests.rs` holds every drawn point to the ground).
/// Provenance: **tuned** (#424).
const SHAKE_BRACE: f32 = 0.05;

/// A head tilt: a bird's head canted to look up with one eye and swayed
/// side to side, as an owl judges depth (10 6.2); a bat's or a glider's
/// canted with its ears perked, as a beast's.
fn head_tilt(kin: Kinship) -> Clip {
    let span = [0.0, 0.25, 0.75, 1.0];
    if kin.leading == WingedFamily::Bird {
        let mut gaze = sway(0.3, 0.75, 3, TILT_SWAY, Vec3::X);
        for key in gaze
            .iter_mut()
            .filter(|key| key.time > 0.0 && key.time < 1.0)
        {
            key.offset.y = TILT_UP * (key.time * std::f32::consts::PI).det_sin();
        }
        Clip::new([
            held(Target::Cant, Vec3::Z * TILT_CANT, span),
            Track::new(Target::Gaze, gaze),
        ])
    } else {
        // Cocked and turned to look up and aside with one eye, then swayed
        // as the bird's: canted alone, a bat's or a glider's head turned
        // about its own line and moved its body 1.5% of its height.
        let mut gaze = sway(0.3, 0.75, 2, TILT_SWAY * 0.5, Vec3::X);
        for key in gaze
            .iter_mut()
            .filter(|key| key.time > 0.0 && key.time < 1.0)
        {
            let along = (key.time * std::f32::consts::PI).det_sin();
            key.offset.x += TILT_SWAY * along;
            key.offset.y = TILT_UP * along;
        }
        Clip::new([
            held(Target::Cant, Vec3::Z * TILT_CANT, span),
            held(Target::Ears, Vec3::Z * -0.08, span),
            Track::new(Target::Gaze, gaze),
        ])
    }
}

/// How far a head tilt cants the head, in quarter turns; how far it sways
/// it side to side and lifts it to look up, as gaze tangents.
/// Provenance: **tuned by render** (#424, the look review): a cant of a sixth
/// of a quarter turn, alone, moved a bat's body 1.4% of its height.
const TILT_CANT: f32 = 0.3;
const TILT_SWAY: f32 = 0.3;
const TILT_UP: f32 = 0.3;

/// The glider's share of a four-legged winged body, against the bat's: what
/// curls it, `0` a bat's rest to `1` a glider's curl. A bird's share of a
/// four-legged cross rests as the bat's does.
fn glider_share(kin: Kinship) -> f32 {
    kin.glider.clamp(0.0, 1.0)
}

/// Turns `joint` by `turn`, a rotation in the body's frame as the root is
/// posed, about the joint itself: everything it carries goes with it.
fn turn_in_body(rig: &Rig, pose: &mut Pose, joint: usize, turn: Quat) {
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let world = pose.forward(rig).rotations;
    let body = world[root];
    let turn = body * turn * body.inverse();
    let parent = rig.joints[joint]
        .parent
        .map_or(Quat::IDENTITY, |parent| world[parent]);
    pose.rotations[joint] = (parent.inverse() * turn * world[joint]).normalize();
}

/// Lays out the wing that is `limb` as a [`Target::Wing`] key asks: its
/// extension `offset.x`, raised by `offset.y` and swept forward by
/// `offset.z` in quarter turns at its shoulder. A right wing's raise and
/// sweep are the left's mirrored. Over the level floor: see [`wing_on`].
///
/// **Out of whatever the pose held, not out of the fold**: a bat's wing is
/// the leg it walks on, which the plant has already put down, so over the
/// first [`WING_TAKE`] of the extension each joint the wing drives turns
/// from where the pose had it to where the wing's own extension puts it.
fn wing(rig: &Rig, pose: &mut Pose, limb: Limb, offset: Vec3) {
    wing_on(rig, pose, limb, offset, &crate::anim::driver::level_ground);
}

/// [`wing`] over `ground`, as [`crate::anim::Driver::drive`] plays a
/// [`Target::Wing`] track (#457).
///
/// **A bat's wing is laid out as it was authored, over the level**: its
/// shoulder and its hip, which the wing's membrane spreads with it, are
/// turned to where the extension puts them in the body, whatever the ground
/// under it. On a slope its stance had stood the legs it lifts for the
/// slope, the wrist upright at the end of a foreleg reaching up it and the
/// hind toes laid along it, and carried into the wing's spread as they stood, they
/// went into the ground as it opened: a bat's hind toes 56.6 mm into a 20
/// degree slope it faced up, the thumb on the side higher across one 27 mm.
/// So on a slope, **as the wing takes the limbs it lifts, what they carry
/// that the wing does not drive** - the wrist and the ankle, the pads and
/// the toes - **turns back to how the wing was authored over** (their rest),
/// by as far as the ground under the trunk is off level (`Under::tilted`),
/// and **every foot of those limbs is kept out of the ground under it**
/// (`ground::keep_feet_up`), lifted in the plane its leg is drawn in. On the
/// level both leave the wing as it was: nothing there is in the ground.
pub(crate) fn wing_on(
    rig: &Rig,
    pose: &mut Pose,
    limb: Limb,
    offset: Vec3,
    ground: &dyn Fn(Vec3) -> Option<Ground>,
) {
    let Some(wings) = &rig.wings else {
        return;
    };
    let Some(side) = wings.sides.iter().find(|side| side.limb == limb) else {
        return;
    };
    let extension = offset.x.clamp(0.0, 1.0);
    if extension <= 0.0 && offset.y == 0.0 && offset.z == 0.0 {
        return;
    }
    let drives: Vec<usize> = side.drives.iter().map(|drive| drive.joint).collect();
    let before: Vec<Quat> = drives.iter().map(|&joint| pose.rotations[joint]).collect();
    // The legs the wing lifts: a bat's foreleg, and the hind leg its
    // membrane spreads.
    let legs: Vec<Limb> = rig
        .ground_contacts()
        .into_iter()
        .filter(|&leg| {
            rig.limb_chain(leg)
                .is_some_and(|chain| chain.iter().any(|joint| drives.contains(joint)))
        })
        .collect();
    let tilted = Under::of(rig, pose, ground).map_or(0.0, |under| under.tilted);
    wings.pose_side(pose, limb, extension);
    let take = smooth((extension / WING_TAKE).clamp(0.0, 1.0));
    for (&joint, was) in drives.iter().zip(before) {
        let written = pose.rotations[joint];
        pose.rotations[joint] = det::slerp(was, written, take);
    }
    if tilted > 0.0 {
        for &leg in &legs {
            let Some(chain) = rig.limb_chain(leg) else {
                continue;
            };
            let carried = std::iter::once(chain[2]).chain(rig.in_zone(Zone::Extremity(leg)));
            for joint in carried.filter(|joint| !drives.contains(joint)) {
                pose.rotations[joint] =
                    det::slerp(pose.rotations[joint], Quat::IDENTITY, take * tilted);
            }
        }
    }
    let quarter = std::f32::consts::FRAC_PI_2;
    if (offset.y != 0.0 || offset.z != 0.0)
        && let Some(&shoulder) = drives.first()
    {
        let mirror = if limb.is_left() { 1.0 } else { -1.0 };
        let turn = Rot::y(-mirror * offset.z * quarter) * Rot::z(mirror * offset.y * quarter);
        turn_in_body(rig, pose, shoulder, turn);
    }
    super::ground::keep_feet_up(rig, pose, ground, &legs);
}

/// The legs the wing that is `limb` lifts as it lays out ([`wing_on`]): a
/// bat's foreleg, and the hind leg its membrane spreads.
fn lifted_by(rig: &Rig, limb: Limb) -> Vec<Limb> {
    let Some(side) = rig
        .wings
        .as_ref()
        .and_then(|wings| wings.sides.iter().find(|side| side.limb == limb))
    else {
        return Vec::new();
    };
    let drives: Vec<usize> = side.drives.iter().map(|drive| drive.joint).collect();
    rig.ground_contacts()
        .into_iter()
        .filter(|&leg| {
            rig.limb_chain(leg)
                .is_some_and(|chain| chain.iter().any(|joint| drives.contains(joint)))
        })
        .collect()
}

/// The legs a clip's wings have taken any of (`taken`, as [`wings_taken`]
/// reads it): what a stride's settle leaves off the ground (#460).
pub(crate) fn legs_taken(rig: &Rig, taken: &[(Limb, f32)]) -> Vec<Limb> {
    let mut legs = Vec::new();
    for &(limb, share) in taken {
        if share > 0.0 {
            for leg in lifted_by(rig, limb) {
                if !legs.contains(&leg) {
                    legs.push(leg);
                }
            }
        }
    }
    legs
}

/// Every joint of the legs a clip's wings have taken (`taken`, as
/// [`wings_taken`] reads it), with its turn in `pose` and how far the wing
/// has it: what [`give_back`] hands back to the wing after a stride's settle
/// (#460). **And on a body on two, the wing's own root** (#460's fixer),
/// which the stride's `ground::hold_wings` turns up for any feather under
/// the height it stands at folded - a wing laid out by the clip is the
/// clip's, as it is standing, where nothing holds it. Empty where no wing
/// has taken a leg or been laid out.
pub(crate) fn wing_held(rig: &Rig, pose: &Pose, taken: &[(Limb, f32)]) -> Vec<(usize, Quat, f32)> {
    let mut held: Vec<(usize, Quat, f32)> = Vec::new();
    for &(limb, share) in taken {
        let share = share.clamp(0.0, 1.0);
        if share <= 0.0 {
            continue;
        }
        let mut joints = Vec::new();
        for leg in lifted_by(rig, limb) {
            let Some(chain) = rig.limb_chain(leg) else {
                continue;
            };
            joints.extend(chain.into_iter().chain(rig.in_zone(Zone::Extremity(leg))));
        }
        if !rig.four_legged() {
            joints.extend(rig.in_zone(Zone::UpperLimb(limb)).first().copied());
        }
        for joint in joints {
            match held.iter_mut().find(|(at, ..)| *at == joint) {
                Some(entry) => entry.2 = entry.2.max(share),
                None => held.push((joint, pose.rotations[joint], share)),
            }
        }
    }
    held
}

/// Every joint of the legs a winged body on two walks on, with its turn in
/// `pose`, all of it: what [`give_back`] hands back to the stride after an
/// emote is laid over a walk (#460's fixer). Empty on a body on four.
pub(crate) fn walked_legs(rig: &Rig, pose: &Pose) -> Vec<(usize, Quat, f32)> {
    if rig.wings.is_none() || rig.four_legged() {
        return Vec::new();
    }
    let mut held: Vec<(usize, Quat, f32)> = Vec::new();
    for leg in rig.ground_contacts() {
        let Some(chain) = rig.limb_chain(leg) else {
            continue;
        };
        for joint in chain.into_iter().chain(rig.in_zone(Zone::Extremity(leg))) {
            if !held.iter().any(|&(at, ..)| at == joint) {
                held.push((joint, pose.rotations[joint], 1.0));
            }
        }
    }
    held
}

/// Turns each joint [`wing_held`] read back toward the turn it had, by as
/// far as the wing has its leg (#460).
pub(crate) fn give_back(pose: &mut Pose, held: &[(usize, Quat, f32)]) {
    for &(joint, turn, share) in held {
        if let Some(now) = pose.rotations.get_mut(joint) {
            *now = det::slerp(*now, turn, share).normalize();
        }
    }
}

/// How far each wing `clip` lays out at `time` has taken the leg it is
/// ([`wing_on`], over the first [`WING_TAKE`] of its extension), `0..=1`, on
/// a winged body: what `ground::keep_clear` lets go of a crawling hand by
/// (#457). None on a body with no wings.
pub(crate) fn wings_taken(rig: &Rig, clip: &Clip, time: f32) -> Vec<(Limb, f32)> {
    if rig.wings.is_none() {
        return Vec::new();
    }
    clip.tracks
        .iter()
        .filter_map(|track| match track.target {
            Target::Wing(limb) => track.offset_at(time, clip.looping).map(|offset| {
                (
                    limb,
                    smooth((offset.x.clamp(0.0, 1.0) / WING_TAKE).clamp(0.0, 1.0)),
                )
            }),
            _ => None,
        })
        .collect()
}

/// How much of a wing's extension it takes to turn from the pose it was in
/// to the wing's own.
/// Provenance: **chosen** (#424): a tenth, so a wing barely off its fold is
/// still the leg the plant put down.
const WING_TAKE: f32 = 0.1;

/// Smoothstep: eased in and out over `0..1`.
fn smooth(share: f32) -> f32 {
    let share = share.clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// Every chain of drawn toes on `rig`'s feet, each from the joint at its
/// root on the pad to its claw's tip.
fn toe_chains(rig: &Rig) -> Vec<Vec<usize>> {
    let digit = |joint: usize| rig.joints[joint].role == Role::Digit;
    let child = |parent: usize| {
        (parent + 1..rig.len())
            .find(|&joint| rig.joints[joint].parent == Some(parent) && digit(joint))
    };
    let mut chains = Vec::new();
    for limb in rig
        .ground_contacts()
        .into_iter()
        .filter(|limb| !limb.is_fore())
    {
        let Some(&pad) = rig.in_zone(Zone::Extremity(limb)).first() else {
            continue;
        };
        for root in (pad + 1..rig.len()).filter(|&joint| rig.joints[joint].parent == Some(pad)) {
            if !digit(root) || rig.wings.as_ref().is_some_and(|wings| wings.owns(root)) {
                continue;
            }
            let mut chain = vec![root];
            while let Some(next) = child(*chain.last().unwrap_or(&root)) {
                chain.push(next);
            }
            chains.push(chain);
        }
    }
    chains
}

/// Curls every drawn toe `share` of the way into a grip (#424), `0` flat to
/// `1` closed: each joint past the toe's root bent down toward the sole, and
/// the root lifted off the ground as a knuckle rises, just as far as brings
/// the claw's tip back to the height it stood at - so the toes close about
/// whatever is under them, and on a flat top every claw stays off it.
pub(crate) fn grip(rig: &Rig, pose: &mut Pose, share: f32) {
    let share = share.clamp(0.0, 1.0);
    if share <= 0.0 || rig.wings.is_none() {
        return;
    }
    for chain in toe_chains(rig) {
        let (Some(&first), Some(&last)) = (chain.first(), chain.last()) else {
            continue;
        };
        let run = rig.joints[last].position - rig.joints[first].position;
        let along = Vec3::new(run.x, 0.0, run.z).normalize_or(Vec3::Z);
        // The axis that carries the toe's line down toward the sole.
        let axis = along.cross(Vec3::NEG_Y).normalize_or(Vec3::X);
        // The leaf at the claw's tip carries nothing to turn.
        // A toe of one bone has nothing past its root to close, and turned
        // down whole it would go into what it stands on: it stays flat.
        let bending = chain.len().saturating_sub(1);
        if bending < 2 {
            continue;
        }
        #[expect(clippy::cast_precision_loss, reason = "a toe is a handful of joints")]
        let each = GRIP_CURL * share / (bending - 1).max(1) as f32;
        // The toe in its own upright plane: each bone along it and up.
        let bones: Vec<(f32, f32)> = chain
            .windows(2)
            .map(|pair| {
                let bone = rig.joints[pair[1]].position - rig.joints[pair[0]].position;
                (bone.dot(along), bone.y)
            })
            .collect();
        let standing: f32 = bones.iter().map(|&(_, up)| up).sum();
        // Where the tip stands with the root lifted `knuckle`, against the
        // root: every bone turned down by all the bends above it.
        let tip = |knuckle: f32| {
            let mut down = 0.0f32;
            let mut height = 0.0f32;
            for (index, &(ahead, up)) in bones.iter().enumerate() {
                down += if index == 0 { -knuckle } else { each };
                let (sin, cos) = (down.det_sin(), down.det_cos());
                height += -ahead * sin + up * cos;
            }
            height
        };
        // The least lift that brings the tip back up, by halving.
        let (mut low, mut high) = (0.0f32, GRIP_KNUCKLE);
        if tip(low) < standing {
            for _ in 0..GRIP_STEPS {
                let middle = 0.5 * (low + high);
                if tip(middle) < standing {
                    low = middle;
                } else {
                    high = middle;
                }
            }
            low = high;
        }
        let knuckle = low;
        for (index, &joint) in chain.iter().take(bending).enumerate() {
            let angle = if index == 0 { -knuckle } else { each };
            pose.rotations[joint] *= Rot::axis_angle(axis, angle);
        }
    }
}

/// How far the joints past a gripping toe's root bend in all, in radians,
/// and the most its root may lift to keep the claw's tip up.
/// Provenance: **tuned by render** (#424): toes closed two radians, a
/// talon's grip; a lift up to most of a quarter turn, which no toe of the
/// families' own feet needs.
const GRIP_CURL: f32 = 2.0;
const GRIP_KNUCKLE: f32 = 1.4;

/// How many halvings find a toe's lift: to a ten-thousandth of a radian.
const GRIP_STEPS: usize = 14;

/// Turns a winged body's head back over its shoulder as a [`Target::Tuck`]
/// key asks: `offset.z` of [`TUCK_TURN`], toward the left for a positive
/// `offset.x`, and dipped by `offset.y` quarter turns besides its own dip.
///
/// **Turned about the body's up at the neck and the head**, each a share,
/// and a bird's neck leaned back first so the head comes down over its
/// shoulders: research-10 6.2, "head turned back and tucked into the
/// scapulars". Past any gaze's limit, as a bird's neck goes (an owl's up to
/// 270 degrees, fact 5).
pub(crate) fn tuck(rig: &Rig, pose: &mut Pose, offset: Vec3) {
    let share = offset.z.clamp(0.0, 1.0);
    if (share <= 0.0 && offset.y == 0.0) || rig.wings.is_none() {
        return;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let neck = rig.in_zone(Zone::Neck);
    let side = if offset.x < 0.0 { -1.0 } else { 1.0 };
    let quarter = std::f32::consts::FRAC_PI_2;
    let upright = !rig.four_legged();
    let turn = TUCK_TURN * share * side;
    // The neck's share, spread down it; the head takes the rest.
    #[expect(clippy::cast_precision_loss, reason = "a neck is a handful of joints")]
    let joints = neck.len().max(1) as f32;
    let each = TUCK_NECK / joints;
    let leaned = if upright { TUCK_LEAN * share } else { 0.0 };
    let lean = Rot::x(-leaned / joints);
    for &joint in &neck {
        turn_in_body(rig, pose, joint, Rot::y(turn * each) * lean);
    }
    let headed = if neck.is_empty() {
        1.0
    } else {
        1.0 - TUCK_NECK
    };
    turn_in_body(rig, pose, head, Rot::y(turn * headed));
    // Dipped from where the neck's lean left it, so the lean carries the
    // head back without lifting the bill - a bird's into its scapulars; a
    // four-legged body's head, already carried near the ground, is not.
    let own = if upright { TUCK_DIP } else { 0.0 };
    let dip = own * share + leaned + offset.y * quarter;
    pose.rotations[head] *= Rot::x(dip);
}

/// How far a tucked head turns about the body's up, in radians; the share of
/// it the neck takes; how far a bird's neck leans back to bring the head down
/// over the shoulders, and how far the head dips into them.
/// Provenance: **tuned by render** (#424): most of a half turn, the bill
/// laid back over the left scapulars.
const TUCK_TURN: f32 = 0.8 * std::f32::consts::PI;
const TUCK_NECK: f32 = 0.5;
const TUCK_LEAN: f32 = 0.7;
const TUCK_DIP: f32 = 0.8;

/// Rolls a winged body's trunk `angle` radians about its own line at the
/// loin (#424), positive rolling its back toward its left: see
/// [`Target::Roll`].
pub(crate) fn roll(rig: &Rig, pose: &mut Pose, loin: usize, angle: f32) {
    let Some(withers) = rig
        .in_zone(Zone::Neck)
        .first()
        .and_then(|&neck| rig.joints[neck].parent)
    else {
        return;
    };
    let line = (rig.joints[withers].position - rig.joints[loin].position).normalize_or(Vec3::Z);
    pose.rotations[loin] *= Rot::axis_angle(line, -angle);
}

/// Lays a four-legged winged body down to rest `down` of the way (#424):
/// see [`Target::Curl`].
///
/// 1. **Down on its belly**: the body lowered until its trunk's lowest ring
///    rests on the ground under it ([`CURL_CLEARANCE`] above it).
/// 2. **Curled round** by the glider's share: [`CURL_SPINE`] down the trunk
///    and the neck toward its left, the head turned on round and dipped
///    into the curl.
/// 3. **Its legs folded under**, each foot sent to the ground below where
///    its girdle has gone and drawn in toward it - a bat's wrists by its
///    bat share, close to its shoulders, the wings folded with them - and
///    laid flat as the body sinks over it.
/// 4. **Its tail wrapped round** the outside of the curl along the ground,
///    by the glider's share.
/// 5. **Its head down**, onto its wrists or into the curl.
///
/// **On `ground`** (#450): the body laid down on the plane the ground makes
/// under its trunk, and each foot and the tail on the ground under it.
///
/// **Laid on the plane under it, then curled as on the level, and laid
/// back.** The body is turned, about the ground under its trunk's middle,
/// from that plane onto the level the curl is solved against
/// ([`Under::level`]), curled there with each foot sent to the ground under
/// it and the tail kept over it, as far over it as it would be over the
/// level floor ([`Under::floor`]), and turned back onto the plane
/// ([`Under::lay`]). Solved against the level floor alone, a bat resting
/// facing up a 20 degree slope put its forearm 222 mm into it, facing down
/// it 70 mm, and across it 93: the legs were folded to the floor's height
/// wherever the ground under them stood.
///
/// **The plane is [`super::ground::plane_under`]'s**, over half the trunk's
/// length either side of its middle, as `keep_clear` lifts the belly over
/// it: a plane's own tilt on a plane, and on a host's height grid - whose
/// tilt jumps at every cell's edge - continuous, and level beside a step.
/// A level ground is the level curl to the bit; ground answering nothing
/// under the trunk is taken as the level.
pub(crate) fn curl_on(
    rig: &Rig,
    pose: &mut Pose,
    down: f32,
    ground: &dyn Fn(Vec3) -> Option<Ground>,
) {
    if down <= 0.0 || !rig.four_legged() || !pose.fits(rig) {
        return;
    }
    let Some(under) = Under::of(rig, pose, ground) else {
        curl_over(rig, pose, down, None);
        return;
    };
    under.level(rig, pose);
    let floor = |at: Vec3| under.floor(ground, at);
    curl_over(
        rig,
        pose,
        down,
        Some(Floor {
            under: &floor,
            tilted: under.tilted,
        }),
    );
    under.lay(rig, pose);
    if super::crawls(rig) {
        stand_wrists(rig, pose);
    }
}

/// Stands a crawling body's wrists upright again where the curl laid its
/// pads (#450), as its idle stands them on any ground: `keep_clear` lays a
/// crawling wrist along the slope under it on every frame on the ground, a
/// rest's included, from upright (`ground::lay_hands`). Left turned with the
/// plane the curl was laid on, it was turned by the slope twice while that
/// laying went on, a bat's folded digits 24 mm into a 20 degree slope it
/// faced down as it lay down.
fn stand_wrists(rig: &Rig, pose: &mut Pose) {
    for limb in rig.ground_contacts() {
        if !limb.is_fore() {
            continue;
        }
        let (Some(chain), Some(&pad)) = (
            rig.limb_chain(limb),
            rig.in_zone(Zone::Extremity(limb)).first(),
        ) else {
            continue;
        };
        let posed = pose.forward(rig);
        let [top, mid, wrist] = chain.map(|joint| posed.positions[joint]);
        let line = (wrist - top).normalize_or_zero();
        let bend = (mid - top) - line * (mid - top).dot(line);
        let pole = match bend.try_normalize() {
            Some(bend) => mid + bend * rig.extent(),
            None => rig.bend_pole(limb).unwrap_or(mid),
        };
        solve_contact_with(rig, pose, limb, posed.positions[pad], pole, Foot::Stood);
    }
}

/// The plane a resting body is laid on (#450): where the ground under its
/// trunk's middle is, and the turn from the level to the plane there.
#[derive(Clone, Copy, Debug)]
struct Under {
    /// The ground under the trunk's middle.
    at: Vec3,
    /// From up to the plane's up.
    turn: Quat,
    /// How far from level the plane is, `0..=1`: see [`LAID_TILT`].
    tilted: f32,
}

impl Under {
    /// The plane under `pose`'s trunk on `ground`, or `None` where that is
    /// the level floor itself, or nothing answers under the trunk.
    fn of(rig: &Rig, pose: &Pose, ground: &dyn Fn(Vec3) -> Option<Ground>) -> Option<Self> {
        let (Some(&pelvis), Some(&withers)) = (
            rig.in_zone(Zone::Pelvis).first(),
            rig.in_zone(Zone::Chest).last(),
        ) else {
            return None;
        };
        let posed = pose.forward(rig);
        let (hips, chest) = (posed.positions[pelvis], posed.positions[withers]);
        let reach = super::ground::BELLY_PLANE * hips.distance(chest);
        let below = super::ground::plane_under(ground, 0.5 * (hips + chest), reach)?;
        let up = below.normal.normalize_or(Vec3::Y);
        if up == Vec3::Y && below.position.y == 0.0 {
            return None;
        }
        let rise = Vec3::new(up.x, 0.0, up.z).length() / up.y.max(f32::EPSILON);
        Some(Self {
            at: below.position,
            turn: det::from_rotation_arc(Vec3::Y, up),
            tilted: smooth((rise / LAID_TILT).clamp(0.0, 1.0)),
        })
    }

    /// The ground under the trunk's middle, brought to the level floor.
    fn flat(self) -> Vec3 {
        Vec3::new(self.at.x, 0.0, self.at.z)
    }

    /// Where `at`, in the frame the body is posed in, lies once the plane
    /// is turned onto the level floor.
    fn levelled(self, at: Vec3) -> Vec3 {
        self.flat() + self.turn.inverse() * (at - self.at)
    }

    /// Turns the body in `pose` from the plane onto the level floor.
    fn level(self, rig: &Rig, pose: &mut Pose) {
        turn_whole(rig, pose, self.turn.inverse(), self.at, self.flat());
    }

    /// Turns the body in `pose` from the level floor back onto the plane.
    fn lay(self, rig: &Rig, pose: &mut Pose) {
        turn_whole(rig, pose, self.turn, self.flat(), self.at);
    }

    /// How high the ground under `at` stands over the level floor, both
    /// in the levelled frame: nought over the plane itself, and over ground
    /// answering nothing.
    fn floor(self, ground: &dyn Fn(Vec3) -> Option<Ground>, at: Vec3) -> f32 {
        let posed = self.at + self.turn * (at - self.flat());
        ground(posed).map_or(0.0, |below| self.levelled(below.position).y)
    }
}

/// How far the plane a body rests on must rise a metre before its legs fold
/// from where they stood and about the bend they stood in, wholly
/// ([`fold_legs`]): short of it they fold as on the level, as far as it is
/// short, so ground a hair off level folds them as the level does.
/// Provenance: **chosen** (#450), the rise a beast's rest takes for its
/// slope (`rest::TILTED`, #413): a hundredth, about half a degree.
const LAID_TILT: f32 = 0.01;

/// What a resting body is curled over off the level ([`curl_on`]): how high
/// the ground under a point stands over the level floor, and how far from
/// level the plane it is laid on is ([`Under::tilted`]).
#[derive(Clone, Copy)]
struct Floor<'a> {
    under: &'a dyn Fn(Vec3) -> f32,
    tilted: f32,
}

/// Turns every joint of `pose` by `turn` about `from`, and carries `from`
/// to `to`: the whole body moved as one.
fn turn_whole(rig: &Rig, pose: &mut Pose, turn: Quat, from: Vec3, to: Vec3) {
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let rest = rig.joints[root].position;
    let at = to + turn * (rest + pose.translation - from);
    pose.translation = at - rest;
    pose.rotations[root] = turn * pose.rotations[root];
}

/// [`curl_on`] over `ground`, or the level floor itself at none.
fn curl_over(rig: &Rig, pose: &mut Pose, down: f32, ground: Option<Floor>) {
    let floor = ground.map(|ground| ground.under);
    let down = down.clamp(0.0, 1.0);
    if down <= 0.0 || !rig.four_legged() {
        return;
    }
    let Some(kin) = super::kinship(rig) else {
        return;
    };
    let eased = smooth(down);
    let curled = glider_share(kin) * eased;
    let drawn = (1.0 - glider_share(kin)) * eased;
    // Whatever of the skin stood below the ground before, which the body
    // is lifted to no higher than.
    let lowest_skin_before = lowest_skin(rig, pose).min(0.0);
    // And where the feet stood, which on the ground they are sent from.
    let stood = floor.map(|_| pose.forward(rig).positions);

    // 1. Down: the trunk's own skin, read off the rig's outline of it where
    // it has one as a beast's lying down reads it, off its rings where not.
    let trunk = |zone: Zone| matches!(zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest);
    let lowest = crate::anim::rest::lowest_in(rig, &pose.forward(rig), trunk);
    if lowest < f32::MAX {
        pose.translation.y -= (lowest - CURL_CLEARANCE).max(0.0) * eased;
    }

    // 2. Curled round: the trunk from the root's first joint up to the
    // withers, then the neck.
    let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
        return;
    };
    let mut spine = Vec::new();
    let mut cursor = rig.joints[neck].parent;
    while let Some(joint) = cursor {
        if rig.joints[joint].parent.is_none() {
            break;
        }
        spine.push(joint);
        cursor = rig.joints[joint].parent;
    }
    spine.reverse();
    let necks = rig.in_zone(Zone::Neck);
    if curled > 0.0 {
        #[expect(clippy::cast_precision_loss, reason = "a spine is a handful of joints")]
        let each = CURL_SPINE * curled / (spine.len() + necks.len()).max(1) as f32;
        for &joint in spine.iter().chain(&necks) {
            pose.rotations[joint] *= Rot::y(each);
        }
    }

    // 3 to 5, then lifted by whatever of its skin they left under the
    // ground - a folded thigh is lower and fatter than the trunk it hangs
    // from, and laid under a glider at the trunk's height it went 51 mm in -
    // and done again from there, the legs reaching down the further.
    //
    // **Every pass, always**, each adding only what is still under, so the
    // lift is a continuous function of how far down the body is: stopping
    // once it cleared made the passes taken, and the lift, jump.
    let curled_pose = pose.clone();
    let mut lift = 0.0f32;
    for pass in 0..=CURL_PASSES {
        *pose = curled_pose.clone();
        pose.translation.y += lift;
        fold_legs(
            rig,
            pose,
            eased,
            curled,
            drawn,
            ground.zip(stood.as_deref()),
        );
        if curled > 0.0 {
            wrap_tail(rig, pose, curled, ground);
        }
        if let Some(&head) = rig.in_zone(Zone::Head).first() {
            pose.rotations[head] *=
                Rot::y(CURL_HEAD_TURN * curled) * Rot::x(CURL_HEAD_DOWN * eased);
        }
        if pass < CURL_PASSES {
            lift += (lowest_skin_before + CURL_CLEARANCE - lowest_skin(rig, pose)).max(0.0);
        }
    }
}

/// The lowest skin of the body but its feet and shins in `pose`, in metres
/// above the ground: the rig's outline of its skin, as a beast's lying down
/// reads it, and every upper limb's ring besides, which reads a folded thigh
/// lower than the outline does. Nought for a body with neither.
fn lowest_skin(rig: &Rig, pose: &Pose) -> f32 {
    let posed = pose.forward(rig);
    // Not the feet and shins, which the legs lay on the ground whatever the
    // body does - [`fold_legs`] holds those up itself - nor the tail, which
    // [`wrap_tail`] lays along it.
    let outline = crate::anim::rest::lowest_in(rig, &posed, |zone| {
        !matches!(zone, Zone::Extremity(_) | Zone::LowerLimb(_) | Zone::Tail)
    });
    let limbs = (0..rig.len())
        .filter(|&joint| matches!(rig.joints[joint].zone, Zone::UpperLimb(_)))
        .filter_map(|joint| crate::anim::rest::ring_low(rig, &posed, joint))
        .fold(f32::MAX, f32::min);
    // And a membrane's spars rooted in the trunk, which the trunk lowers:
    // each read as the skin it carries, [`SPAR_DEPTH`] below it, and no
    // lower than it stood.
    let trunk = |zone: Zone| matches!(zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest);
    let spars = rig.wings.as_ref().map_or(f32::MAX, |wings| {
        wings
            .joints
            .iter()
            .filter(|&&joint| {
                // Rooted, past every joint of the wing's own, in the trunk.
                let mut root = rig.joints[joint].parent;
                while let Some(at) = root.filter(|&at| wings.owns(at)) {
                    root = rig.joints[at].parent;
                }
                root.is_some_and(|root| trunk(rig.joints[root].zone))
            })
            .map(|&joint| {
                let stood = rig.joints[joint].position.y;
                let under = (SPAR_DEPTH - stood).max(0.0);
                // One that stood nearer the ground than that reads as the
                // ground's clearance where it stood.
                posed.positions[joint].y - SPAR_DEPTH + under + CURL_CLEARANCE
            })
            .fold(f32::MAX, f32::min)
    });
    let low = outline.min(limbs).min(spars);
    if low == f32::MAX { 0.0 } else { low }
}

/// How far below a membrane's spar its skin may reach, in metres: a glider's
/// patagium sags 24 mm under the spar that roots it in the chest.
const SPAR_DEPTH: f32 = 0.03;

/// Step 3 of [`curl_on`]: a resting body's legs folded under it, each foot sent
/// to the ground below where its girdle has gone and drawn in toward it, and
/// laid flat as the body sinks over it.
///
/// On the ground ([`curl_on`]), `ground` says how high the ground under a
/// point stands over the level floor - each foot is sent that much higher,
/// and its shin and pad held that much higher too - and where each joint
/// stood before the body went down: off the level ([`LAID_TILT`]) a foot
/// goes from there `eased` of the way. On a slope a body's idle stands its
/// feet elsewhere than the level's goals, and sent there at once a bat's
/// forefoot moved 20 mm in the frame its getting up handed back to the idle
/// on a slope of 20 degrees it faced up.
fn fold_legs(
    rig: &Rig,
    pose: &mut Pose,
    eased: f32,
    curled: f32,
    drawn: f32,
    ground: Option<(Floor, &[Vec3])>,
) {
    let floor = ground.map(|(floor, _)| floor.under);
    // How far from where they stood the feet are sent, and about the bend
    // they stood in: `eased`, on ground [`LAID_TILT`] off level or more.
    let sent = |tilted: f32| 1.0 - tilted * (1.0 - eased);
    let posed = pose.forward(rig);
    for limb in rig.ground_contacts() {
        let (Some(chain), Some(&pad)) = (
            rig.limb_chain(limb),
            rig.in_zone(Zone::Extremity(limb)).first(),
        ) else {
            continue;
        };
        let top = chain[0];
        let Some(girdle) = rig.joints[top].parent else {
            continue;
        };
        let carry = |rest: Vec3| {
            posed.positions[girdle] + posed.rotations[girdle] * (rest - rig.joints[girdle].position)
        };
        let reach = rig.joints[pad].position - rig.joints[top].position;
        let draw = if limb.is_fore() {
            1.0 - CURL_DRAW[0] * drawn - CURL_DRAW[2] * curled
        } else {
            1.0 - CURL_DRAW[1] * eased
        };
        let under = Vec3::new(reach.x * draw, reach.y, reach.z * draw);
        let mut goal = carry(rig.joints[top].position + under);
        goal = match ground {
            Some((floor, stood)) => {
                goal.y = rig.joints[pad].position.y + (floor.under)(goal);
                stood[pad].lerp(goal, sent(floor.tilted))
            }
            None => Vec3::new(goal.x, rig.joints[pad].position.y, goal.z),
        };
        let pole = rig.bend_pole(limb).map_or(goal + Vec3::Y, carry);
        // On the ground, folded from the bend the leg stood in, `eased` of
        // the way to the one carried with its girdle: on a slope a body's
        // idle stands its legs bent about poles its girdles do not carry,
        // and folded about the carried one from the first frame, a bat's
        // elbows went 90 mm round between its getting up and its idle across
        // a slope of 20 degrees.
        let pole = match ground {
            Some((floor, stood)) => {
                let [root, mid, tip] = chain.map(|joint| stood[joint]);
                let line = (tip - root).normalize_or_zero();
                let bend = (mid - root) - line * (mid - root).dot(line);
                match bend.try_normalize() {
                    Some(bend) => {
                        let drawn = posed.positions[top] + (mid - root) + bend * rig.extent();
                        drawn.lerp(pole, sent(floor.tilted))
                    }
                    None => pole,
                }
            }
            None => pole,
        };
        // The hind feet laid flat under it as a beast's lying down lays them;
        // the forefeet stood on their pads, as a bat crawls on its wrists -
        // a glider's laid flat turned its patagium's root at the wrist 20 mm
        // into the ground.
        // **And the foot raised by whatever its shin, laid on the ground,
        // sinks into it** - a glider's forearm under its chest went 10 mm in.
        // The shin and the pad: each may come down to the ground, or to where
        // its ring stood if that was lower - a pad's skin stands a little in.
        let standing = Pose::rest(rig).forward(rig);
        let held: Vec<(usize, f32)> = rig
            .in_zone(Zone::LowerLimb(limb))
            .into_iter()
            .chain(std::iter::once(pad))
            .filter_map(|joint| {
                let stood = crate::anim::rest::ring_low(rig, &standing, joint)?;
                Some((joint, stood.min(CURL_CLEARANCE)))
            })
            .collect();
        for _ in 0..SHIN_PASSES {
            if limb.is_fore() {
                solve_contact_with(rig, pose, limb, goal, pole, Foot::Stood);
            } else {
                solve_contact_laid_by(rig, pose, limb, goal, pole, eased);
            }
            let laid = pose.forward(rig);
            let short = held
                .iter()
                .filter_map(|&(joint, low)| {
                    let ring = crate::anim::rest::ring_low(rig, &laid, joint)?;
                    Some(match floor {
                        Some(floor) => low + floor(laid.positions[joint]) - ring,
                        None => low - ring,
                    })
                })
                .fold(0.0f32, f32::max);
            if short <= 0.0 {
                break;
            }
            goal.y += short;
        }
    }
}

/// How many times a resting body is lifted off what it sank, and a resting
/// foot off what its shin sank.
const CURL_PASSES: usize = 4;
const SHIN_PASSES: usize = 3;

/// How far a resting body's trunk stands off the ground, in metres: its
/// lowest ring just touching.
const CURL_CLEARANCE: f32 = 0.004;

/// How far a curled glider's trunk and neck bend round in all, in radians,
/// and its head turns on past them.
/// Provenance: **tuned by render** (#424): about a half turn and a third,
/// the nose brought round toward the hips, as a curled cat's.
const CURL_SPINE: f32 = 2.3;
const CURL_HEAD_TURN: f32 = 0.6;

/// How far a resting head dips, in radians: onto a bat's wrists, into a
/// glider's curl.
const CURL_HEAD_DOWN: f32 = 0.5;

/// How far a resting bat's wrists are drawn in toward its shoulders, a share
/// of where they stood; a hind foot toward its hip; and a curled glider's
/// forefoot, which drawn in further turned its patagium's root at the wrist
/// into the ground.
/// Provenance: **tuned by render** (#424): a bat's wrists folded close (10
/// 6.2: "wings folded"), a glider's paws gathered under it.
const CURL_DRAW: [f32; 3] = [0.45, 0.3, 0.1];

/// How far round a curled glider's tail wraps along the ground, in radians.
const TAIL_WRAP: f32 = 3.4;

/// Lays a tail along the ground round the outside of the curl, `share` of
/// the way from where the pose had it: its first joint where it is, each
/// further one on an arc at its own radius above the ground, turning toward
/// the body's left as the trunk does.
///
/// On the ground ([`curl_on`]), `ground` says how high the ground under a
/// point stands over the level floor: each joint is kept that much higher,
/// and one the aim leaves under it - each joint is aimed along the line to
/// its target, and a bone longer than that line carries the next past it -
/// is aimed again over it, as far as the plane it rests on is off level
/// ([`LAID_TILT`]). A body standing on a slope stands lower over it than on
/// the level, and its tail, carried on its pelvis, hangs the lower: wrapped
/// from there a Glider x Bat cross's tip was aimed 18 mm under its target and
/// went 12.7 mm into a slope of 20 degrees it faced up, 6.5 at the most on
/// the level.
fn wrap_tail(rig: &Rig, pose: &mut Pose, share: f32, ground: Option<Floor>) {
    let floor = ground.map(|ground| ground.under);
    let tail = rig.tail();
    if tail.len() < 2 {
        return;
    }
    let posed = pose.forward(rig);
    let lengths: Vec<f32> = tail
        .windows(2)
        .map(|pair| {
            rig.joints[pair[0]]
                .position
                .distance(rig.joints[pair[1]].position)
        })
        .collect();
    let total: f32 = lengths.iter().sum::<f32>().max(f32::EPSILON);
    let start = posed.positions[tail[0]];
    let out = posed.positions[tail[1]] - start;
    let mut heading = Vec3::new(out.x, 0.0, out.z).normalize_or(Vec3::NEG_Z);
    let mut targets = vec![start];
    let mut at = start;
    for (index, &length) in lengths.iter().enumerate() {
        let joint = tail[index + 1];
        // On the ground, which is where the body is posed over: each joint
        // its ring's own depth above it - its radius by its section's
        // larger half, which on a bat-glider cross's tail is twice its
        // radius - and a little more.
        let it = &rig.joints[joint];
        let ring = it.radius * it.scale.x.max(it.scale.y).max(1.0) + CURL_CLEARANCE;
        // Turned by the share of the wrap this bone covers, toward the left
        // (+X) for a tail running back.
        let turn = -TAIL_WRAP * share * length / total;
        heading = (Rot::y(turn) * heading).normalize_or(heading);
        let low = match floor {
            // Over the ground about where this bone lays it.
            Some(floor) => {
                let over = floor(at + heading * length);
                (posed.positions[joint].y - over).min(ring) + over
            }
            None => posed.positions[joint].y.min(ring),
        };
        let drop = (at.y - low).max(0.0).min(length);
        let run = (length * length - drop * drop).max(0.0).sqrt();
        at = Vec3::new(at.x, at.y - drop, at.z) + heading * run;
        targets.push(at);
    }
    // Partway, from where the pose had it - and never under the ground, which
    // a long tail lowered with the body went, half wrapped, 34 mm into.
    for (index, target) in targets.iter_mut().enumerate().skip(1) {
        let it = &rig.joints[tail[index]];
        let ring = it.radius * it.scale.x.max(it.scale.y).max(1.0) + CURL_CLEARANCE;
        *target = posed.positions[tail[index]].lerp(*target, share);
        target.y = match floor {
            Some(floor) => target.y.max(ring + floor(*target)),
            None => target.y.max(ring),
        };
    }
    aim_chain(rig, pose, &tail, &targets);
    let Some(ground) = ground else {
        return;
    };
    for _ in 0..TAIL_PASSES {
        let posed = pose.forward(rig);
        let mut under = false;
        let lifted: Vec<Vec3> = tail
            .iter()
            .enumerate()
            .map(|(index, &joint)| {
                let mut at = posed.positions[joint];
                let it = &rig.joints[joint];
                let ring = it.radius * it.scale.x.max(it.scale.y).max(1.0) + CURL_CLEARANCE;
                let short = ring + (ground.under)(at) - at.y;
                if index > 0 && short > 0.0 {
                    at.y += ground.tilted * short;
                    under = true;
                }
                at
            })
            .collect();
        if !under {
            break;
        }
        aim_chain(rig, pose, &tail, &lifted);
    }
}

/// How many times [`wrap_tail`] aims a tail again over the ground it was
/// left under.
const TAIL_PASSES: usize = 2;

/// Turns each joint of `chain` in turn so the next lies along the line to
/// its target, from the root of the chain outward: the bones keep their
/// lengths, and each joint lands as near its target as the one before it
/// leaves possible.
fn aim_chain(rig: &Rig, pose: &mut Pose, chain: &[usize], targets: &[Vec3]) {
    for index in 0..chain.len().saturating_sub(1) {
        let posed = pose.forward(rig);
        let (joint, next) = (chain[index], chain[index + 1]);
        let now = posed.positions[next] - posed.positions[joint];
        let wanted = targets[index + 1] - posed.positions[joint];
        let (Some(now), Some(wanted)) = (now.try_normalize(), wanted.try_normalize()) else {
            continue;
        };
        let turn = det::from_rotation_arc(now, wanted);
        let parent = rig.joints[joint]
            .parent
            .map_or(Quat::IDENTITY, |parent| posed.rotations[parent]);
        pose.rotations[joint] = (parent.inverse() * turn * posed.rotations[joint]).normalize();
    }
}

/// How far the rest `name` shuts `rig`'s eyes when it is all the way down,
/// `0..=1`, or `None` for a body with no wings or a name that is no rest of
/// its: a roost, a bat's rest and a glider's curl are sleep (10 6.2: "sleep
/// is about 80% of roost time"), a perch is not.
#[must_use]
pub fn sleeps(rig: &Rig, name: &str) -> Option<f32> {
    timing(rig, name)?;
    Some(match name {
        "Roost" | "Rest" | "Curl" => 1.0,
        _ => 0.0,
    })
}

/// The gaze a bird's glance turns its head with (#424): its eyes barely
/// move, so its head goes further than a beast's and its neck carries the
/// head without the chest - an owl's turns "up to 270 degrees" (10 fact 5),
/// a glance a good deal less.
/// Provenance: **chosen** (#424): the person's glance limit doubled at most,
/// within what [`crate::anim::look_at`]'s clamp about the rest facing can say.
#[must_use]
pub fn glance_config() -> GazeConfig {
    GazeConfig {
        limit: 1.5,
        shares: [0.0, 0.3, 1.0],
    }
}

/// How far into its turn a bird's head is when a glance is `share` of the
/// way through its ramp (#424): **a head saccade**, the turn made in the
/// first 30 percent of the ramp and held - 10 fact 5: "an idle bird
/// animates as a series of quick head saccades, each one a held pose".
#[must_use]
pub fn head_saccade(share: f32) -> f32 {
    smooth(share / HEAD_SNAP)
}

/// The share of a glance's ramp a bird's head takes to turn.
/// Provenance: **chosen** (#424): under a seventh of a second over the
/// idle's 0.45 s ramp, a snap rather than a sweep.
const HEAD_SNAP: f32 = 0.3;

/// Whether `rig` is a bird for its gaze: winged, and standing on two legs.
#[must_use]
pub fn looks_with_head(rig: &Rig) -> bool {
    super::bird(rig).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::gesture::returns_to_rest;

    #[test]
    fn the_union_holds_every_family_roster() {
        for name in BIRD.iter().chain(BAT).chain(GLIDER) {
            assert!(ROSTER.contains(name), "{name} is not in the union");
        }
    }

    #[test]
    fn a_reversed_clip_ends_where_the_clip_began() {
        let clip = perch(PERCH_DROP);
        let back = backward(&clip);
        for (track, reversed) in clip.tracks.iter().zip(&back.tracks) {
            assert_eq!(
                track.keys.first().map(|key| key.offset),
                reversed.keys.last().map(|key| key.offset)
            );
        }
        assert!(!returns_to_rest(&back));
    }
}
