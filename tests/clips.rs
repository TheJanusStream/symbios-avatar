//! The artifact this repository ships.
//!
//! `assets/clips.bin` is checked in and `examples/bakeclips` writes it, which
//! means nothing in the crate reads it unless something here does — and a file
//! nothing reads is a file that rots. These are what keep it honest: it parses,
//! it holds what `docs/clips.md` says it holds, and every clip in it plays on a
//! body built today rather than on the one it was baked against.

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
