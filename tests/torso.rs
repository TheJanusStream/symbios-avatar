//! The trunk's silhouette: a lean, massive body tapers from the armpit to the
//! waist, and a body that is neither is left alone.
//!
//! **Guard fitted AFTER the geometry was agreed by render** — milestone #10's
//! standing method (#317). The owner's report: a muscular masculine body had
//! its lower ribcage standing out past the chest above it, so no V appeared.
//! Measured with `bodyaudit` (which until #317 read the bare cage and had
//! never seen a carve): the band at the ribcage's equator stood 42% over the
//! male reference while the band under the armpit stood 19%, a bulge below a
//! dip. `torso::carve_taper` pulls the flank in about that equator and lets
//! it out under the armpit, by lean mass. This reads the same two bands off
//! the body `Avatar::build` ships.
use symbios_avatar::plan::Composites;
use symbios_avatar::torso::ChestTraits;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Zone};

/// A record at named composites, sanitized the way a shipped one is.
fn body(femininity: f32, mass: f32, fat: f32) -> Avatar {
    let mut record = AvatarRecord::new("Trunk", Archetype::default());
    record.composites.femininity = femininity;
    record.composites.mass = mass;
    record.composites.body_fat = fat;
    record.sanitize();
    Avatar::build(&record).expect("a biped builds")
}

/// The trunk's half-width in a band of the waist-to-girdle span, arms out.
///
/// Only skin the trunk's own bones hold: a vertex whose nearest bone is a
/// limb's is the arm, however close to the flank it hangs.
fn half_width(avatar: &Avatar, from: f32, to: f32) -> f32 {
    let waist = avatar.rig.in_zone(Zone::Abdomen)[0];
    let chests = avatar.rig.in_zone(Zone::Chest);
    let (waist, girdle) = (
        avatar.rig.joints[waist].position.y,
        avatar.rig.joints[chests[1]].position.y,
    );
    let span = girdle - waist;
    avatar
        .parts
        .body
        .positions
        .iter()
        .filter(|at| {
            let t = (at.y - waist) / span;
            t >= from
                && t <= to
                && matches!(
                    avatar.rig.joints[avatar.rig.nearest_bone(**at).joint].zone,
                    Zone::Chest | Zone::Abdomen
                )
        })
        .map(|at| at.x.abs())
        .fold(0.0f32, f32::max)
}

#[test]
fn a_lean_massive_trunk_tapers_from_the_armpit_to_the_waist() {
    // The ribcage's equator against the band under the armpit, on the
    // muscular masculine corner. Before: 0.848 — the equator nearly as wide
    // as the armpit, which is the egg; now about 0.70.
    let avatar = body(-1.0, 1.0, 0.06);
    let ribs = half_width(&avatar, 0.30, 0.42);
    let armpit = half_width(&avatar, 0.66, 0.78);
    let ratio = ribs / armpit;
    assert!(
        ratio <= 0.80,
        "the lower ribcage is {ratio:.3} of the armpit's half-width ({:.0} mm against {:.0})",
        ribs * 1000.0,
        armpit * 1000.0
    );
}

#[test]
fn a_body_with_no_definition_is_not_tapered() {
    // The neutral body carries 22% fat, past the fraction where definition
    // shows on any frame, so the carve has nothing to say to it — which is
    // what keeps the goldens still. Read as the trait itself, which is the
    // one place the strength is decided.
    let traits = ChestTraits::of(&Composites::default());
    assert!(
        traits.taper == 0.0,
        "the default body carries a taper of {:.3}",
        traits.taper
    );
    let heavy = Composites {
        mass: 1.0,
        femininity: -1.0,
        body_fat: 0.30,
        ..Default::default()
    };
    assert!(
        ChestTraits::of(&heavy).taper == 0.0,
        "a heavy body that is not lean carries a taper"
    );
}
