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
/// Measured over 64 seeds: 43,236 to 43,388, with the default at 43,308. That
/// range is worth noticing on its own. A body's cost is almost **independent of
/// its parameters**, because hair dominates the count and hair group count is a
/// fixed default rather than something the axes move. Nothing a creator does
/// makes an avatar cheaper, which is why the cut has to come from the generator.
const TRIANGLE_CEILING: usize = 43_500;

/// Draw calls the crate currently costs.
///
/// Five: skin, hair, cloth, eyes, lids. Two of the excess are the eye pair —
/// globes need a glossy material and lids move without a joint to move them
/// (#35) — and the third is the attached-part draw that charting parts into the
/// skin atlas absorbs (#58).
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
fn hair_is_where_the_triangles_are() {
    // Measured rather than assumed, because it decides where the next cut goes:
    // halving everything else would not reach the target, and halving hair
    // nearly does.
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
        share > 0.6,
        "hair is {:.0}% of the budget; if this has fallen, the cut worked and \
         the ceiling above should come down with it",
        share * 100.0
    );
}

#[test]
#[ignore = "the target, not the state: needs the hair cut (#40) and parts charted into the atlas (#58)"]
fn a_default_avatar_fits_the_webgl2_budget() {
    // Turn this on — and delete the ceilings above — when it passes. It is the
    // number the engine is actually judged by; everything else here is a guard
    // against drifting further from it.
    let avatar = built(None);
    assert!(
        avatar.budget.tris <= TRIANGLE_TARGET,
        "{} triangles against a budget of {TRIANGLE_TARGET}",
        avatar.budget.tris
    );
    assert!(
        avatar.budget.meshes <= MESH_TARGET,
        "{} draws against a budget of {MESH_TARGET}",
        avatar.budget.meshes
    );
}
