//! What a real build keeps in its [`Footprints`] (#477), against the owner's
//! [`CEILING`].
//!
//! The ceiling was held on the bare default skeleton alone, which has no face
//! and no parts and is a person of middling build, so it never saw what real
//! builds kept: at 0.24.7 persons up to 893 tiles, 5.5 MB, at the budget
//! sweep's dear corners, winged bodies up to 2,291, 14 MB, and beasts up to
//! 1,511, 9 MB - in a web worker whose memory, once grown, is never handed
//! back. Read here on the bodies that came nearest, built whole.

use super::*;
use crate::plan::{Archetype, QuadrupedParams, WingedParams};
use crate::{Avatar, AvatarRecord};

/// A person rolled from `seed`, its head at the budget sweep's dear end -
/// long and broad - and, if `lean` is given, at the sweep's composite corner
/// lean, thin and tall at that frame (`tests/budget.rs`, `corners`).
fn person(seed: i64, lean: Option<f32>) -> (String, AvatarRecord) {
    let mut record = AvatarRecord::new("Footprints", Archetype::default());
    record.reroll(seed);
    if let Some(frame) = lean {
        record.composites.femininity = frame;
        record.composites.mass = -2.0;
        record.composites.body_fat = crate::plan::BODY_FAT_RANGE.0;
    }
    if let Archetype::Humanoid(params) = &mut record.archetype {
        params.head_breadth = 1.0;
        params.face_length = 1.0;
        if lean.is_some() {
            params.height = crate::plan::humanoid_height_range().1;
        }
    }
    record.sanitize();
    let corner = lean.map_or(String::new(), |frame| {
        format!(" lean thin tall at frame {frame:+}")
    });
    (format!("person {seed} long broad{corner}"), record)
}

/// A body of `archetype` rolled from `seed`.
fn rolled(name: &str, archetype: Archetype, seed: i64) -> (String, AvatarRecord) {
    let mut record = AvatarRecord::new("Footprints", archetype);
    record.reroll(seed);
    (format!("{name} {seed}"), record)
}

/// What building `record` keeps in its footprints, in bytes.
fn kept(record: &AvatarRecord) -> usize {
    kept_since();
    Avatar::build(record).expect("a body builds");
    kept_since().expect("a build bakes")
}

#[test]
fn a_real_body_keeps_its_footprints_under_the_ceiling() {
    // **Persons**: the budget sweep's six seeds at its dear head, and the
    // dearest of 160 persons read at #477 - those six at the sweep's four
    // heads, its twenty corner seeds lean, thin and tall at frames -2, 0 and
    // +2 at both its dear heads, and the sixteen seeds `compare-published`
    // builds: seed 2 at the corner, 893 tiles at frame -2 at 0.24.7, seed 13,
    // the dearest of every other seed, 655; and seed 2 as rolled, the
    // dearest of the sixteen, 699.
    let mut persons: Vec<(String, AvatarRecord)> = [1, 7, 23, 29, 42, 99]
        .into_iter()
        .map(|seed| person(seed, None))
        .collect();
    for seed in [2, 13] {
        for frame in [-2.0, 0.0] {
            persons.push(person(seed, Some(frame)));
        }
    }
    persons.push(rolled("person", Archetype::default(), 2));
    // **Coated bodies**, which read no footprint: the rolls that kept the
    // most at 0.24.7 of 24 winged rolls and of 12 beasts - winged 14,
    // 2,291 tiles, half of them its ears', jaw's and lids'; 6 and 18, whose
    // heads' own skin kept the most, 1,537 and 1,659; and beasts 8 and 3,
    // 1,511 and 977 of their heads' own.
    let mut coated: Vec<(String, AvatarRecord)> = [14, 6, 18]
        .into_iter()
        .map(|seed| rolled("winged", Archetype::Winged(WingedParams::default()), seed))
        .collect();
    for seed in [8, 3] {
        coated.push(rolled(
            "beast",
            Archetype::Quadruped(QuadrupedParams::default()),
            seed,
        ));
    }

    let all: Vec<(String, AvatarRecord, bool)> = persons
        .into_iter()
        .map(|(name, record)| (name, record, true))
        .chain(
            coated
                .into_iter()
                .map(|(name, record)| (name, record, false)),
        )
        .collect();
    let threads = 4;
    let read: Vec<(&str, bool, usize)> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|first| {
                let all = &all;
                scope.spawn(move || {
                    all.iter()
                        .skip(first)
                        .step_by(threads)
                        .map(|(name, record, skin)| (name.as_str(), *skin, kept(record)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a build"))
            .collect()
    });

    let dearest = read
        .iter()
        .filter(|(_, skin, _)| *skin)
        .max_by_key(|(_, _, bytes)| *bytes)
        .expect("persons");
    println!(
        "the dearest person, {}, keeps {} bytes of footprints against the {CEILING} of the ceiling",
        dearest.0, dearest.2
    );
    let mut failures = Vec::new();
    for &(name, skin, bytes) in &read {
        if skin && bytes == 0 {
            // Measured, or the ceiling holds over nothing.
            failures.push(format!("{name} keeps no footprints"));
        } else if skin && bytes > CEILING {
            failures.push(format!("{name} keeps {bytes} bytes"));
        } else if !skin && bytes > 0 {
            failures.push(format!(
                "{name} keeps {bytes} bytes, which its coat never reads"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "footprints past the {CEILING} bytes of the ceiling, or where nothing reads them:\n{}",
        failures.join("\n")
    );
}
