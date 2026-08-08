//! The artifact this repository ships.
//!
//! `assets/clips.bin` is checked in and `examples/bakeclips` writes it, which
//! means nothing in the crate reads it unless something here does — and a file
//! nothing reads is a file that rots. These are what keep it honest: it parses,
//! it holds what `docs/clips.md` says it holds, and every clip in it plays on a
//! body built today rather than on the one it was baked against.

use symbios_avatar::anim::{contacts_during, contacts_in};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, ClipLibrary, Play, Slot, Zone};

/// The artifact, read from the repository rather than from a build directory.
fn library() -> ClipLibrary {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/clips.bin"))
        .expect("assets/clips.bin is checked in; run examples/bakeclips if it is not");
    ClipLibrary::read(&bytes).expect("the artifact parses")
}

/// The curated set, in the order `examples/bakeclips` bakes it.
const CURATED: [&str; 12] = [
    "Idle_A",
    "Idle_Talking",
    "Idle Listening",
    "Walk",
    "Jog",
    "Sprint",
    "Sitting_Idle",
    "Greeting",
    "Head Nod",
    "Reject",
    "Bow",
    "Sleeping",
];

#[test]
fn the_shipped_artifact_holds_the_curated_set() {
    let library = library();
    assert_eq!(
        library.names(),
        CURATED,
        "the artifact and the curated list have drifted; re-bake or fix docs/clips.md"
    );
    // The figure docs/clips.md states, to the byte. A silent change of size is a
    // silent change of contents.
    assert_eq!(library.bytes(), 204_462);
}

#[test]
fn every_shipped_clip_plays_on_a_body_built_today() {
    // **The point of a Slot.** A clip is baked against one body and has to play
    // on every other, so this asks a body built now — not the one the bake ran
    // against — to resolve every track the artifact carries. A track that
    // resolves to nothing is a limb that would not move and no error.
    let record = AvatarRecord::new("Plays", Archetype::default());
    let avatar = Avatar::build(&record).expect("a biped builds");

    for clip in &library().clips {
        assert!(clip.frames > 0, "{} has no frames", clip.name);
        assert!(clip.rate > 0.0, "{} has no rate", clip.name);
        for track in &clip.tracks {
            assert!(
                track.slot.resolve(&avatar.rig).is_some(),
                "{}: {:?} lands on no joint of a body built today",
                clip.name,
                track.slot
            );
        }

        // And it poses a body, at both ends and in the middle, without panicking
        // or producing a rotation that is not one.
        let mut play = Play::new();
        for _ in 0..3 {
            let pose = play.pose(clip, &avatar.rig);
            assert!(
                pose.rotations.iter().all(|q| q.is_finite()),
                "{} produced a rotation that is not finite",
                clip.name
            );
            play.advance(clip, clip.duration() / 2.0);
        }
    }
}

#[test]
fn a_run_leaves_the_ground_and_a_walk_does_not() {
    // **The property `anim::contacts_in` exists for, asked of real motion.**
    // `Rig::ground_contacts` answers about the body's shape and says two feet
    // whatever it is doing; handing that to a footing solve drags an airborne
    // foot onto the floor. A run is where that shows and a walk is where it
    // hides, so both are checked — a test that only saw the run could pass on a
    // rule that called every foot airborne.
    let record = AvatarRecord::new("Running", Archetype::default());
    let avatar = Avatar::build(&record).expect("a biped builds");
    let library = library();

    let over = |name: &str| -> Vec<usize> {
        let clip = library
            .get(name)
            .unwrap_or_else(|| panic!("{name} is shipped"));
        (0..16)
            .map(|frame| {
                let time = clip.duration() * frame as f32 / 16.0;
                contacts_in(&avatar.rig, &clip.pose(&avatar.rig, time)).len()
            })
            .collect()
    };

    let feet = avatar.rig.ground_contacts().len();
    let sprint = over("Sprint");
    assert!(
        sprint.iter().any(|down| *down < feet),
        "Sprint never left the ground: {sprint:?}"
    );
    let walk = over("Walk");
    assert!(
        walk.iter().all(|down| *down == feet),
        "a walk has a foot down at all times, and this one did not: {walk:?}"
    );
}

#[test]
fn a_walks_swinging_foot_is_not_called_planted() {
    // **Why `contacts_during` exists and `contacts_in` was not enough.** A
    // walking foot lifts about 150 mm at its highest (#139), so for much of its
    // swing it is within `CONTACT_SLACK` of the floor — measured, the height
    // test alone calls both feet planted at every phase of Walk. Handing that to
    // a footing solve drags the swinging foot down and ruins the walk.
    //
    // Speed is what separates them, and it has to be read with the clip's root
    // motion in: a planted foot is stationary in the world while the body passes
    // over it.
    let record = AvatarRecord::new("Walking", Archetype::default());
    let avatar = Avatar::build(&record).expect("a biped builds");
    let library = library();
    let walk = library.get("Walk").expect("Walk is shipped");

    let feet = avatar.rig.ground_contacts().len();
    let by_height: Vec<usize> = (0..24)
        .map(|frame| {
            let time = walk.duration() * frame as f32 / 24.0;
            contacts_in(&avatar.rig, &walk.pose(&avatar.rig, time)).len()
        })
        .collect();
    assert!(
        by_height.iter().all(|down| *down == feet),
        "height alone was expected to call every foot planted through a walk: {by_height:?}"
    );

    let by_speed: Vec<usize> = (0..24)
        .map(|frame| {
            let time = walk.duration() * frame as f32 / 24.0;
            contacts_during(&avatar.rig, walk, time).len()
        })
        .collect();
    assert!(
        by_speed.iter().any(|down| *down < feet),
        "no foot was ever swinging through a whole walk cycle: {by_speed:?}"
    );
    assert!(
        by_speed.iter().any(|down| *down > 0),
        "a walk always has a foot down and this one never did: {by_speed:?}"
    );
}

#[test]
fn the_clips_move_the_parts_their_names_promise() {
    // Cheap, and it would catch the worst thing a re-bake could do quietly:
    // write the right names over the wrong motion. A nod is the head; a walk is
    // the legs; a greeting is an arm.
    let library = library();
    let moves = |name: &str, zone: Zone, index: u8| {
        let clip = library
            .get(name)
            .unwrap_or_else(|| panic!("{name} is shipped"));
        let slot = Slot::new(zone, index);
        clip.tracks
            .iter()
            .find(|track| track.slot == slot)
            .map(|track| matches!(track.rotation, symbios_avatar::Curve::Sampled(_)))
            .unwrap_or(false)
    };

    assert!(moves("Head Nod", Zone::Head, 0), "a nod moves the head");
    assert!(
        moves("Walk", Zone::UpperLimb(symbios_avatar::Limb::HindLeft), 0),
        "a walk moves the legs"
    );
    assert!(
        moves(
            "Greeting",
            Zone::UpperLimb(symbios_avatar::Limb::ForeLeft),
            0
        ),
        "a greeting moves an arm"
    );
    assert!(
        !moves(
            "Head Nod",
            Zone::UpperLimb(symbios_avatar::Limb::HindLeft),
            0
        ),
        "a nod does not move a leg"
    );
}
