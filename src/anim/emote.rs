//! A beast's roster (#370): what a dog, a cat or a horse does with itself that
//! a person's gestures cannot say - sit on its haunches, lie down, bow to
//! play, shake itself dry, wag, flick its ears, cock its head, yawn, pant,
//! sniff the ground.
//!
//! Written in the same goal-space [`Clip`]s as a person's roster, with the
//! parts a beast moves: [`Target::Pitch`] and [`Target::Roll`] for its trunk,
//! [`Target::Tail`], [`Target::Ears`] and [`Target::Jaw`], each doing nothing
//! on a body without the part, and [`Target::Cant`], which any head can make.
//!
//! **A person is never offered one.** [`super::gesture::for_body`] answers
//! these names only on four legs, and the [`super::Driver`] asks it, because
//! the parts of an emote any body has - where it looks, how low it sits,
//! where its feet go - move a person as readily as a beast: a sit asked of a
//! person folded their legs under them and laid their feet flat.
//!
//! **Two kinds, as the person's roster has.** [`sit`] and [`lie_down`] end in
//! the pose they are for, which a caller holds; every other one starts and
//! ends at rest. [`super::gesture::returns_to_rest`] tells them apart.

use glam::Vec3;

use crate::det::DetMath;

use super::clip::{Clip, Key, Target, Track};
use crate::plan::Limb;

/// Every beast's emote this module can build, by name.
pub const ROSTER: &[&str] = &[
    "Sit",
    "Lie Down",
    "Play Bow",
    "Shake",
    "Wag",
    "Ear Flick",
    "Head Tilt",
    "Yawn",
    "Pant",
    "Sniff",
];

/// The emote called `name`, if this roster has one.
#[must_use]
pub fn by_name(name: &str) -> Option<Clip> {
    match name {
        "Sit" => Some(sit()),
        "Lie Down" => Some(lie_down()),
        "Play Bow" => Some(play_bow()),
        "Shake" => Some(shake()),
        "Wag" => Some(wag()),
        "Ear Flick" => Some(ear_flick()),
        "Head Tilt" => Some(head_tilt()),
        "Yawn" => Some(yawn()),
        "Pant" => Some(pant()),
        "Sniff" => Some(sniff()),
        _ => None,
    }
}

/// How far a sitting beast's trunk turns nose up about its withers, in
/// quarter turns: its hindquarters down onto the ground behind its fores.
/// Provenance: **tuned by render** (#370).
const SIT_PITCH: f32 = 0.35;

/// How far forward a sitting beast's hind feet come, under its haunches, in
/// the leg's reach.
/// Provenance: **tuned by render** (#370).
const SIT_TUCK: f32 = 0.3;

/// How far a lying beast's body sinks, in its standing leg's reach, and how
/// far forward its fore feet stretch and its hind feet tuck, in the leg's
/// reach - every foot laid flat ([`Track::laid`]), so its legs fold under a
/// body that comes down to the ground.
/// Provenance: **tuned by render** (#370); every paw lands where it is sent
/// down to 1.1, hips 312 mm down on the default dog.
const LIE_DROP: f32 = 0.9;
const LIE_REACH: f32 = 0.8;
const LIE_TUCK: f32 = 0.3;

/// How far a bowing beast's trunk turns nose down about its hips, in quarter
/// turns, and how far its fore feet stretch out ahead, in the leg's reach.
/// Provenance: **tuned by render** (#370).
const BOW_PITCH: f32 = 0.22;
const BOW_REACH: f32 = 0.5;

/// How far a shaking beast braces down, in its standing leg's reach.
/// Provenance: **measured** (#370), by `examples/gestureaudit --quadruped`:
/// the least that keeps every audited body's paws on the ground through the
/// roll - the default dog needs 0.05, a horse 0.15.
const SHAKE_BRACE: f32 = 0.15;

/// How far a tail swings either way as it wags, in quarter turns.
/// Provenance: **tuned by render** (#370).
const WAG: f32 = 0.3;

/// The keys of a sway between `from` and `to`, `swings` times each way
/// about nothing, `size` at its widest: eased in and out so it starts and
/// ends at rest, and laid out as `along` is.
fn sway(from: f32, to: f32, swings: usize, size: f32, along: Vec3) -> Vec<Key> {
    // Two keys a swing, and the rest at both ends.
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

/// A track that eases in to `toward` by `arrive`, holds it until `leave`,
/// and eases back out by `gone`.
fn held(target: Target, toward: Vec3, arrive: f32, leave: f32, gone: f32) -> Track {
    Track::new(
        target,
        [
            Key::new(0.0, Vec3::ZERO),
            Key::new(arrive, toward),
            Key::new(leave, toward),
            Key::new(gone, Vec3::ZERO),
        ],
    )
}

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

/// Every foot held where it stands, in the world, while the body moves over
/// it.
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

/// Sitting on its haunches: the hindquarters settle onto the ground behind
/// fores that stay straight, the hind feet come forward under them, and the
/// head stays level. **A state**: it ends sitting.
#[must_use]
pub fn sit() -> Clip {
    let down = 0.6;
    let mut tracks = vec![settled(Target::Pitch, Vec3::Z * SIT_PITCH, down)];
    tracks.extend(planted(&[Limb::ForeLeft, Limb::ForeRight]));
    for limb in [Limb::HindLeft, Limb::HindRight] {
        tracks.push(
            settled(Target::Just(limb), Vec3::Z * SIT_TUCK, down)
                .in_world()
                .laid(),
        );
    }
    tracks.push(Track::new(Target::Gaze, [Key::new(0.0, Vec3::ZERO)]).in_world());
    Clip::new(tracks)
}

/// Lying down: the body sinks to the ground with its fore feet stretched out
/// ahead and its hind feet tucked under it, every foot laid flat, its head
/// up. **A state**: it ends lying.
#[must_use]
pub fn lie_down() -> Clip {
    let down = 0.7;
    let mut tracks = vec![settled(Target::Root, Vec3::NEG_Y * LIE_DROP, down)];
    for (limbs, reach) in [
        ([Limb::ForeLeft, Limb::ForeRight], LIE_REACH),
        ([Limb::HindLeft, Limb::HindRight], LIE_TUCK),
    ] {
        for limb in limbs {
            tracks.push(
                settled(Target::Just(limb), Vec3::Z * reach, down)
                    .in_world()
                    .laid(),
            );
        }
    }
    tracks.push(Track::new(Target::Gaze, [Key::new(0.0, Vec3::ZERO)]).in_world());
    Clip::new(tracks)
}

/// The bow a dog asks another to play with: its forehand down onto its
/// elbows, fores stretched out ahead, rump up and tail wagging high, looking
/// up at whoever it is asking.
#[must_use]
pub fn play_bow() -> Clip {
    let (down, up, gone) = (0.25, 0.75, 1.0);
    let mut tracks = vec![held(Target::Pitch, Vec3::NEG_Z * BOW_PITCH, down, up, gone)];
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        tracks.push(held(Target::Just(limb), Vec3::Z * BOW_REACH, down, up, gone).in_world());
    }
    tracks.extend(planted(&[Limb::HindLeft, Limb::HindRight]));
    tracks.push(held(Target::Gaze, Vec3::Y * 0.3, down, up, gone));
    let mut tail = sway(0.1, 0.9, 4, WAG, Vec3::X);
    for key in tail
        .iter_mut()
        .filter(|key| key.time > 0.0 && key.time < 1.0)
    {
        // High while it wags.
        key.offset.y = 0.25 * (key.time * std::f32::consts::PI).det_sin();
    }
    tracks.push(Track::new(Target::Tail, tail));
    Clip::new(tracks)
}

/// Shaking itself dry: braced a little lower, the trunk rolls back and forth
/// about its length, the head first and the tail last, with every foot held
/// where it stands. The brace is what leaves a leg under the shoulder the
/// roll lifts something to reach with: standing tall, a fore came up 17 mm
/// short of the ground at the roll's height.
#[must_use]
pub fn shake() -> Clip {
    let mut tracks = vec![
        held(Target::Root, Vec3::NEG_Y * SHAKE_BRACE, 0.1, 0.85, 0.95),
        Track::new(Target::Cant, sway(0.05, 0.75, 4, 0.25, Vec3::Z)),
        Track::new(Target::Roll, sway(0.1, 0.85, 4, 0.12, Vec3::Z)),
        Track::new(Target::Tail, sway(0.15, 0.95, 4, WAG, Vec3::X)),
    ];
    tracks.extend(planted(&Limb::ALL));
    Clip::new(tracks)
}

/// A wag, the tail a little raised.
#[must_use]
pub fn wag() -> Clip {
    let mut keys = sway(0.05, 0.95, 5, WAG, Vec3::X);
    for key in keys
        .iter_mut()
        .filter(|key| key.time > 0.0 && key.time < 1.0)
    {
        key.offset.y = 0.1 * (key.time * std::f32::consts::PI).det_sin();
    }
    Clip::new([Track::new(Target::Tail, keys)])
}

/// Both ears flicked back and forward again.
#[must_use]
pub fn ear_flick() -> Clip {
    Clip::new([Track::new(
        Target::Ears,
        [
            Key::new(0.0, Vec3::ZERO),
            Key::new(0.12, Vec3::Z * 0.3),
            Key::new(0.28, Vec3::Z * -0.05),
            Key::new(0.45, Vec3::ZERO),
        ],
    )])
}

/// A questioning tilt of the head, the ears perked forward.
#[must_use]
pub fn head_tilt() -> Clip {
    Clip::new([
        held(Target::Cant, Vec3::Z * 0.17, 0.3, 0.7, 1.0),
        held(Target::Ears, Vec3::Z * -0.08, 0.3, 0.7, 1.0),
    ])
}

/// A yawn: the jaw wide, the head up, the ears back.
#[must_use]
pub fn yawn() -> Clip {
    Clip::new([
        held(Target::Jaw, Vec3::Y, 0.45, 0.6, 0.85),
        held(Target::Gaze, Vec3::Y * 0.3, 0.45, 0.6, 0.9),
        held(Target::Ears, Vec3::Z * 0.2, 0.45, 0.6, 0.9),
    ])
}

/// Panting: the mouth open and the tongue out, the jaw working with each
/// breath.
#[must_use]
pub fn pant() -> Clip {
    let mut jaw = sway(0.1, 0.9, 5, 0.08, Vec3::Y);
    for key in &mut jaw {
        let open = if key.time <= 0.0 || key.time >= 1.0 {
            0.0
        } else {
            0.35 * (key.time * std::f32::consts::PI).det_sin().sqrt()
        };
        key.offset.y += open;
        key.offset.z = open * 2.5;
    }
    Clip::new([Track::new(Target::Jaw, jaw)])
}

/// Nose to the ground ahead, in short quick sniffs.
#[must_use]
pub fn sniff() -> Clip {
    let mut keys = sway(0.25, 0.8, 4, 0.06, Vec3::Y);
    for key in &mut keys {
        if key.time > 0.0 && key.time < 1.0 {
            key.offset.y -= 0.7 * (key.time * std::f32::consts::PI).det_sin().sqrt();
        }
    }
    Clip::new([Track::new(Target::Gaze, keys).in_world()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::Pose;
    use crate::anim::gesture::returns_to_rest;
    use crate::plan::{BodyPlan, Composites, HumanoidParams, QuadrupedParams, Zone};
    use crate::rig::Rig;

    fn beast() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton(&Composites::default()))
            .expect("rigs")
    }

    /// How far each named joint has moved from rest with `clip` held at
    /// `time`: the hips, the withers, and the four paws, in metres.
    fn moved(rig: &Rig, clip: &Clip, time: f32) -> (Vec3, Vec3, Vec<Vec3>) {
        let mut pose = Pose::rest(rig);
        clip.apply(rig, &mut pose, time);
        let posed = pose.forward(rig).positions;
        let joint = |zone: Zone| *rig.in_zone(zone).first().expect("a beast's joint");
        let from = |joint: usize| posed[joint] - rig.joints[joint].position;
        let paws = Limb::ALL
            .into_iter()
            .map(|limb| from(joint(Zone::Extremity(limb))))
            .collect();
        (from(joint(Zone::Pelvis)), from(joint(Zone::Chest)), paws)
    }

    #[test]
    fn a_sitting_beast_lowers_its_hindquarters_over_straight_fores() {
        let rig = beast();
        let (hips, withers, paws) = moved(&rig, &sit(), 1.0);
        assert!(hips.y < -0.15, "the hips sank {:.0} mm", -hips.y * 1000.0);
        // Sitting back about its shoulders, the chest above them settles a
        // little; the hindquarters go all the way down.
        assert!(
            withers.y > hips.y * 0.4,
            "the withers sank {:.0} mm against the hips' {:.0}",
            -withers.y * 1000.0,
            -hips.y * 1000.0
        );
        for (limb, paw) in Limb::ALL.into_iter().zip(&paws) {
            if limb.is_fore() {
                assert!(
                    paw.length() < 0.005,
                    "{limb:?} moved {:.0} mm",
                    paw.length() * 1000.0
                );
            } else {
                assert!(
                    paw.z > 0.03,
                    "{limb:?} came {:.0} mm forward",
                    paw.z * 1000.0
                );
            }
        }
    }

    #[test]
    fn a_pitch_alone_leaves_the_feet_under_the_staying_end_where_they_stood() {
        // Nose up turns about the shoulders and nose down about the hips, and
        // the legs there keep pointing where they did, so with nothing
        // holding them their feet do not move.
        let rig = beast();
        for (pitch, fores) in [(0.3, true), (-0.2, false)] {
            let clip = Clip::new([Track::new(Target::Pitch, [Key::new(0.0, Vec3::Z * pitch)])]);
            let (_, _, paws) = moved(&rig, &clip, 0.5);
            for (limb, paw) in Limb::ALL.into_iter().zip(&paws) {
                if limb.is_fore() == fores {
                    assert!(
                        paw.length() < 1.0e-3,
                        "pitched {pitch}, {limb:?} moved {:.1} mm",
                        paw.length() * 1000.0
                    );
                }
            }
        }
    }

    #[test]
    fn a_lying_beast_sinks_with_its_fores_out_and_its_feet_on_the_ground() {
        let rig = beast();
        let (hips, withers, paws) = moved(&rig, &lie_down(), 1.0);
        assert!(
            hips.y < -0.2 && withers.y < -0.2,
            "sank {hips:?} and {withers:?}"
        );
        for (limb, paw) in Limb::ALL.into_iter().zip(&paws) {
            assert!(
                paw.y.abs() < 0.01,
                "{limb:?} left the ground by {:.0} mm",
                paw.y * 1000.0
            );
            if limb.is_fore() {
                assert!(paw.z > 0.1, "{limb:?} reached {:.0} mm out", paw.z * 1000.0);
            }
        }
    }

    #[test]
    fn a_bowing_beast_lowers_its_forehand_and_keeps_its_rump_up() {
        let rig = beast();
        let (hips, withers, paws) = moved(&rig, &play_bow(), 0.5);
        assert!(
            withers.y < -0.1,
            "the withers sank {:.0} mm",
            -withers.y * 1000.0
        );
        assert!(hips.y > -0.03, "the hips sank {:.0} mm", -hips.y * 1000.0);
        for (limb, paw) in Limb::ALL.into_iter().zip(&paws) {
            assert!(
                paw.y.abs() < 0.01,
                "{limb:?} left the ground by {:.0} mm",
                paw.y * 1000.0
            );
            if !limb.is_fore() {
                assert!(
                    paw.length() < 0.005,
                    "{limb:?} moved {:.0} mm",
                    paw.length() * 1000.0
                );
            }
        }
        // And it ends standing.
        let (hips, withers, _) = moved(&rig, &play_bow(), 1.0);
        assert!(hips.length() < 1.0e-3 && withers.length() < 1.0e-3);
    }

    #[test]
    fn a_shaking_beast_rolls_its_back_and_keeps_its_feet() {
        let rig = beast();
        let loin = *rig.in_zone(Zone::Abdomen).first().expect("a loin");
        let rolled = (0..=40)
            .map(|step| {
                let mut pose = Pose::rest(&rig);
                shake().apply(&rig, &mut pose, step as f32 / 40.0);
                let (_, _, paws) = moved(&rig, &shake(), step as f32 / 40.0);
                let slid = paws.iter().map(|paw| paw.length()).fold(0.0f32, f32::max);
                // Within what the contact solve itself calls reached: a foot
                // the forehand's roll swings out is put back to the solve's
                // own verdict, not to the millimetre.
                assert!(slid < 0.007, "a foot slid {:.1} mm", slid * 1000.0);
                pose.rotations[loin].to_axis_angle().1
            })
            .fold(0.0f32, f32::max);
        assert!(rolled > 0.12, "the loin turned {rolled:.2} rad at most");
    }

    #[test]
    fn every_name_builds_and_only_the_states_stay_put() {
        for name in ROSTER {
            let clip = by_name(name).unwrap_or_else(|| panic!("{name} is in the roster"));
            let state = matches!(*name, "Sit" | "Lie Down");
            assert_eq!(
                returns_to_rest(&clip),
                !state,
                "{name} {} at rest",
                if state { "must not end" } else { "must end" }
            );
        }
    }

    #[test]
    fn a_person_asked_for_a_beasts_parts_does_nothing() {
        // The refusal: a person has no tail, no beast's ears or jaw, and no
        // trunk that lies along the ground to pitch or roll.
        let rig = Rig::from_skeleton(&HumanoidParams::default().skeleton(&Composites::default()))
            .expect("rigs");
        let rest = Pose::rest(&rig);
        let only = |target: Target| {
            Clip::new([Track::new(
                target,
                [Key::new(0.0, Vec3::ZERO), Key::new(1.0, Vec3::ONE * 0.3)],
            )])
        };
        let beasts = [
            ("Wag", wag()),
            ("Ear Flick", ear_flick()),
            ("Jaw", only(Target::Jaw)),
            ("Pant", pant()),
            ("Pitch", only(Target::Pitch)),
            ("Roll", only(Target::Roll)),
        ];
        for (name, clip) in beasts {
            for step in 0..=10 {
                let mut pose = Pose::rest(&rig);
                clip.apply(&rig, &mut pose, step as f32 / 10.0);
                let (a, b) = (rest.forward(&rig).positions, pose.forward(&rig).positions);
                let moved = a
                    .iter()
                    .zip(&b)
                    .map(|(a, b)| a.distance(*b))
                    .fold(0.0f32, f32::max);
                assert!(moved < 2.0e-3, "{name} moved a person {moved} m");
            }
        }
    }
}
