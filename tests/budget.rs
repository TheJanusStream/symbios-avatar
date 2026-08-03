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

use symbios_avatar::{Archetype, Avatar, AvatarRecord, MeshKind};

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
/// refinement is graded and confined to the face, which is why it cost about
/// 300 triangles rather than the 2,700 another whole subdivision level would.
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
fn hair_is_no_longer_where_all_the_triangles_are() {
    // This test used to assert that hair was over 60% of the budget, and say
    // that if the share ever fell, the cut had worked. It fell: 69.8% to 54.9%.
    //
    // It is kept, inverted, because the share is still the number that decides
    // where any further cut goes — and because hair is the one part whose cost
    // a record can move, so a share that climbs back is a sign the tier stopped
    // biting rather than that hair got dearer.
    let avatar = built(None);
    let hair = avatar
        .drawn(0.0)
        .into_iter()
        .find(|mesh| mesh.kind == MeshKind::Hair)
        .expect("a biped has hair")
        .mesh
        .triangulated()
        .len();
    let share = hair as f32 / avatar.budget.tris as f32;
    assert!(
        share < 0.6,
        "hair is back to {:.0}% of the budget",
        share * 100.0
    );
    // And it is still the largest single part, so it is still where to look.
    assert!(
        share > 0.4,
        "hair is down to {:.0}% of the budget; the next cut belongs elsewhere \
         and this test should say where",
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
        groups: u32::MAX,
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
