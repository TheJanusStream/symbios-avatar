//! What one avatar is allowed to cost.
//!
//! The first consuming application is wasm32 on WebGL2, where the ceiling is
//! one draw call per skinned mesh and there is no batching to hide behind. That
//! makes the budget a property of the engine rather than of the app: a body that
//! costs too much cannot be made cheaper by whoever renders it.
//!
//! Two tests, deliberately. One is a **ratchet** at the figure the crate has
//! actually reached, so nothing silently gets worse; the other is the **target**,
//! ignored until it can pass. Writing only the target would leave a red test in
//! CI and teach everyone to ignore it; writing only the ratchet would quietly
//! bless whatever today happens to cost.

use symbios_avatar::{Archetype, Avatar, AvatarRecord};

/// Triangles a WebGL2-tier avatar may draw.
///
/// The upper end of the 15–30k band the reference games sit in.
const TRIANGLE_TARGET: usize = 30_000;

/// Draw calls a WebGL2-tier avatar may cost.
const MESH_TARGET: usize = 3;

/// Triangles the crate currently draws, across the whole parameter space.
///
/// Not a target — a high-water mark. Lowering it is the work; raising it needs
/// a reason written down beside the change.
///
/// It rose from 29,600 when the front of the head was refined (#59): the head
/// arrives from the cage with a mean mesh edge of 24 mm and every feature a
/// face needs is smaller than that, so there was nothing there to shape. The
/// refinement is graded and confined to the face, and a third pass went in with
/// #59 because a nostril crease is 5 mm wide and 7 mm cells cannot hold one.
/// Three passes cost about 4,800 triangles against the 12,000 two more whole
/// subdivision levels would, and take the face to a 3.6 mm cell.
///
/// Measured after that, the dearest legal record — maximum hair, every axis at
/// its most expensive — is 25,660. This ceiling has four thousand of slack in
/// it and is due a ratchet down; it is left where it is only because lowering a
/// ceiling in the same change that raises a cost hides which did what.
///
/// It was 43,500 before the hair cut (#40), of which hair alone was 30,208.
/// Sampling each lock by how far it actually travels, rather than giving every
/// lock the count the crown row needs, took hair to 15,976 without touching the
/// cross-section, the group count, or any axis a creator sets.
///
/// One thing that used to be true here no longer is. A body's cost used to be
/// almost independent of its parameters, because every lock cost the same and
/// the group count was fixed; now a lock's price follows its length and its
/// wave, and a head of hair ranges over more than a factor of five. What keeps
/// the ceiling is [`symbios_avatar::hair::MAX_TRIANGLES`], which tiers the
/// group count down when the rest of the axes are expensive.
const TRIANGLE_CEILING: usize = 29_800;

/// Draw calls the crate currently costs.
///
/// Five: skin, hair, cloth, eye globes, lids. The comment here used to blame a
/// third of the excess on attached parts having no atlas region; charting them
/// (#58) absorbed that draw and the count stayed at five, so it was never the
/// third. Both of the two over target are the eyes — globes want a glossy
/// material of their own, and the lids are geometry rather than a pose because
/// nothing rigs a lid. Which of those survives is #35's to decide.
const MESH_CEILING: usize = 5;

fn built(seed: Option<i64>) -> Avatar {
    let mut record = AvatarRecord::new("Budget", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    Avatar::build(&record).expect("a biped builds")
}

#[test]
fn the_default_avatar_does_not_get_more_expensive() {
    let avatar = built(None);
    assert!(
        avatar.budget.tris <= TRIANGLE_CEILING,
        "the default body grew to {} triangles, past the {TRIANGLE_CEILING} it had reached",
        avatar.budget.tris
    );
    assert!(
        avatar.budget.meshes <= MESH_CEILING,
        "the default body grew to {} draws, past the {MESH_CEILING} it had reached",
        avatar.budget.meshes
    );
    // A ceiling nobody is near stops being a ceiling. If this fires, the body
    // got much cheaper and the constant should follow it down.
    assert!(
        avatar.budget.tris > TRIANGLE_CEILING / 2,
        "the body now costs {} triangles; lower TRIANGLE_CEILING to match",
        avatar.budget.tris
    );
}

#[test]
fn the_ceiling_holds_across_the_parameter_space() {
    // The default is one point in the space, and a budget that only holds there
    // is not a budget — the first version of this test asserted against the
    // default's own figure and seed 23 came in forty triangles above it.
    let mut worst = 0;
    for seed in [1, 7, 23, 29, 42, 99] {
        let avatar = built(Some(seed));
        worst = worst.max(avatar.budget.tris);
        assert!(
            avatar.budget.tris <= TRIANGLE_CEILING,
            "seed {seed} costs {} triangles",
            avatar.budget.tris
        );
        assert!(
            avatar.budget.meshes <= MESH_CEILING,
            "seed {seed} costs {} draws",
            avatar.budget.meshes
        );
    }
    // And the spread really is as narrow as the constant's note claims. If a
    // parameter ever does buy real geometry, this is where that shows up.
    assert!(
        worst < TRIANGLE_CEILING,
        "a seed reached the ceiling exactly at {worst}"
    );
}

#[test]
fn a_creature_is_not_more_expensive_than_a_person() {
    // Creatures are day-one scope, and a quadruped wears nothing, so it should
    // be comfortably the cheaper of the two. If that ever inverts, something is
    // being built for it that should not be.
    let beast = Avatar::build(&AvatarRecord::new(
        "Beast",
        Archetype::Quadruped(symbios_avatar::QuadrupedParams::default()),
    ))
    .expect("a quadruped builds");
    assert!(
        beast.budget.tris < built(None).budget.tris,
        "the quadruped costs {} triangles",
        beast.budget.tris
    );
}

#[test]
fn no_one_part_of_a_body_dominates_its_budget() {
    // This test has now been rewritten twice by its own failure, which is what
    // it is for. It began asserting hair was over 60% of the budget. The hair
    // cut (#40) took that to 54.9% and it was inverted to catch the share
    // climbing back. Rebuilding hair as a sculpted shell (#68) took it to 19%,
    // and the lower bound fired with the message that the next cut belongs
    // elsewhere. It does: skin is now the largest single part.
    //
    // So it asks the question that outlives any particular answer — is any one
    // part eating the body? — rather than naming a part.
    //
    // **Raised from 0.55 to 0.60 by the mouth's refinement pass (#85), and that
    // is a decision rather than an accommodation.** Skin was already 53% of the
    // default body before it; the pass took it to 57%, spending 2,154 triangles
    // to halve the cell size in the band from the nose base to below the chin.
    // The absolute budget is not what this guards — the body is 23,182 against
    // a 30,000 target, the ceiling holds on every seed, and the greedy-hair
    // record still fits. What it guards is balance, and the honest answer for
    // this crate is that a character generator whose deliverable is a FACE
    // should spend its triangles on one: at 3.6 mm cells every term in the lip
    // field was about one cell wide and the mouth rendered as a stack of bars.
    //
    // The number to watch is now hair (3,416) and cloth (5,936). If the total
    // ever presses the ceiling, those are where the room is, not the face.
    //
    // **The jaw's two passes took skin from 57.3% to 57.9% for 652 triangles
    // (#80), and the guard is what stopped a third.** The jaw's border out at
    // the gonion is carved across three quarters of a cell; a third pass over
    // the knee's own band would take it to 1.9 and cost 664 more, which lands
    // at 59.0% — one percent of guard left. A guard with one percent left in it
    // is not a guard, so the pass was not taken and the softness is recorded in
    // `FACE_PASSES` instead. Note also that the default body is NOT the worst
    // case here: the refinement bands are fixed heights in head radii, so a
    // record with a small head refines more (#86).
    let avatar = built(None);
    let mut largest = ("nothing", 0usize);
    let mut by_kind: Vec<(&str, usize)> = Vec::new();
    for drawn in avatar.drawn(0.0) {
        let tris = drawn.mesh.triangulated().len();
        let name = drawn.kind.name();
        if let Some(entry) = by_kind.iter_mut().find(|(kind, _)| *kind == name) {
            entry.1 += tris;
        } else {
            by_kind.push((name, tris));
        }
    }
    for (name, tris) in &by_kind {
        if *tris > largest.1 {
            largest = (name, *tris);
        }
    }
    let share = largest.1 as f32 / avatar.budget.tris as f32;
    assert!(
        share < 0.60,
        "{} is {:.0}% of the budget, and a body that is mostly one thing is \
         where the next cut goes: {by_kind:?}",
        largest.0,
        share * 100.0
    );
}

#[test]
fn a_default_avatar_fits_the_webgl2_triangle_budget() {
    // The number the engine is actually judged by. It was ignored until the
    // hair cut (#40) landed; the ceilings above are now only a guard against
    // drifting back toward it.
    let avatar = built(None);
    assert!(
        avatar.budget.tris <= TRIANGLE_TARGET,
        "{} triangles against a budget of {TRIANGLE_TARGET}",
        avatar.budget.tris
    );
}

#[test]
fn the_budget_holds_for_a_record_that_asks_for_the_most_expensive_hair() {
    // The target has to survive a record off the network, not just the default
    // one. Hair is the only part whose cost a record can move, and until #40 it
    // could move it to twice the whole avatar's budget.
    let mut record = AvatarRecord::new("Greedy", Archetype::default());
    record.hair = symbios_avatar::HairParams {
        length: 1.0,
        wave: 1.0,
        volume: 1.0,
        locks: u32::MAX,
        curl: 1.0,
        ..record.hair
    };
    record.sanitize();
    let avatar = Avatar::build(&record).expect("a biped builds");
    assert!(
        avatar.budget.tris <= TRIANGLE_TARGET,
        "the dearest legal hair brought the body to {} triangles",
        avatar.budget.tris
    );
}

#[test]
#[ignore = "the target, not the state: eye globes need a glossy material and lids move without a joint (#35)"]
fn a_default_avatar_fits_the_webgl2_draw_budget() {
    // The triangle half of the WebGL2 target passes; this half does not, and
    // will not until the eyes stop needing draws of their own. Turn it on — and
    // delete MESH_CEILING — when it does.
    let avatar = built(None);
    assert!(
        avatar.budget.meshes <= MESH_TARGET,
        "{} draws against a budget of {MESH_TARGET}",
        avatar.budget.meshes
    );
}
