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
///
/// **Four, by the owner's decision of 2026-08-02, and it was three.** Three
/// would mean folding the eye globes into the skin material and giving up the
/// specular eye at exactly the framing a face is judged from — a quality loss
/// taken to satisfy a number. So the bar is four draws with each one justified
/// by a material the others cannot provide: skin takes the atlas, hair is
/// alpha-tested, cloth has its own roughness, an eye is glossy. See #6, whose
/// criterion 1 this is.
const MESH_TARGET: usize = 4;

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
///
/// **Up 100 while #61 moved both sides of it, which is worth writing out because
/// the two nearly cancelled and neither is small.**
///
/// The face's refinement bands became fractions of each head's own lower face
/// rather than raw skull radii, because a face-length axis slides the whole
/// feature stack through bands measured in radii and the lip line walks out of
/// its own refinement at both ends. On the default body that re-basing is
/// bit-identical — it was derived from that body — and off it, it costs: a band
/// edge lands ON a ring of faces rather than between them, so seed 42's stretch
/// differing from the default's by 1.4% moved a whole row of quads inside and
/// cost 976 triangles.
///
/// It was paid for twice over. The finest pass came in from ±0.85 of a lip stack
/// to ±0.7, where the groove it exists for is seven parts in ten thousand of its
/// peak — 390 triangles on the default and 750 on the dearest. And `refine_face`
/// now asks its azimuth of the UNSECTIONED head, so a narrow skull no longer
/// passes more of its own circumference into every band: that alone was worth
/// 2,700 triangles between the two ends of the breadth axis, a tenth of the
/// whole avatar decided by which way a slider was pushed.
///
/// Net, measured: the default fell from 27,464 to 27,078, every seed the test
/// below rolls fell too, and the 100 is bought entirely by the corners the test
/// now visits — the dearest body reachable anywhere in the space is seed 42 with
/// a long broad skull at 29,886, where six seeds as rolled reach only 29,352.
/// The high-water mark rose because the space being measured grew, not because
/// a body got dearer.
const TRIANGLE_CEILING: usize = 29_900;

/// Draw calls the crate currently costs.
///
/// Five: skin, hair, cloth, eye globes, lids. The comment here used to blame a
/// third of the excess on attached parts having no atlas region; charting them
/// (#58) absorbed that draw and the count stayed at five, so it was never the
/// third.
///
/// **The one over the target of four is the LIDS, and only the lids.** The eye
/// globes keep a draw of their own by decision — see [`MESH_TARGET`] — and the
/// lids are geometry rather than a pose only because nothing rigs a lid yet.
/// #118 retires them into a slice of the skin and takes this to four.
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
    //
    // **A seed is not the whole space, and the axes that cost geometry are the
    // ones a seed draws timidly** (#61). Re-rolls hold head breadth and face
    // length inside ±0.7, so six seeds sampled a body a record can ask for and
    // never reached it: the dearest seed came in at 29,352 while the same seed
    // with a long broad skull costs 29,886, and the whole of that 534 is a
    // record's to spend. The head's own two axes are pinned to their ends here
    // for the same reason `tests/plan.rs` sweeps corners rather than samples.
    let mut worst = (String::new(), 0);
    for seed in [1, 7, 23, 29, 42, 99] {
        for (name, breadth, length) in [
            ("as rolled", None, None),
            ("long broad", Some(1.0f32), Some(1.0f32)),
            ("long narrow", Some(-1.0), Some(1.0)),
            ("short broad", Some(1.0), Some(-1.0)),
        ] {
            let mut record = AvatarRecord::new("Budget", Archetype::default());
            record.reroll(seed);
            if let Archetype::Humanoid(params) = &mut record.archetype {
                params.head_breadth = breadth.unwrap_or(params.head_breadth);
                params.face_length = length.unwrap_or(params.face_length);
            }
            let avatar = Avatar::build(&record).expect("a biped builds");
            let at = format!("seed {seed} {name}");
            if avatar.budget.tris > worst.1 {
                worst = (at.clone(), avatar.budget.tris);
            }
            assert!(
                avatar.budget.tris <= TRIANGLE_CEILING,
                "{at} costs {} triangles",
                avatar.budget.tris
            );
            assert!(
                avatar.budget.meshes <= MESH_CEILING,
                "{at} costs {} draws",
                avatar.budget.meshes
            );
        }
    }
    let worst = worst.1;
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
    // **Raised again from 0.60 to 0.63 by the foot (#111), and it is the same
    // kind of decision.** The heel took skin from 14,440 to 16,320 — 1,880
    // triangles — and with it skin's share from 58% to 61%. What this guard is
    // for is an *ornament* eating the body: hair that costs more than the
    // person wearing it, which is the state #40 found and fixed. A foot is not
    // an ornament, it is the body, and the alternative to paying for it is a
    // biped with no heel. The total did press the ceiling, and the prediction
    // in the paragraph above turned out to be right — the room came out of
    // hair, whose leftover-defined ceiling had gone stale at three times the
    // room that existed (`hair::MAX_TRIANGLES`, 16,500 -> 6,000).
    //
    // The room left is now cloth (6,248), and after that the face's refinement
    // passes. There is not a third raise in this number.
    //
    // **Re-based from 0.63 to 0.75 by the eight-point cage (#107), and it is
    // NOT the third raise that paragraph rules out.** Every raise above spent
    // more on the face against a fixed body. This is the opposite: the body got
    // cheaper and took the counterweight with it. Decomposed on the default
    // avatar, before and after:
    //
    // ```text
    //            before    after
    //   skin     16,320   19,896     of which base body 5,092, face refinement 14,804
    //   cloth     6,248    2,936     cut from body faces, and there are half as many
    //   hair      3,416    3,816
    //   eye         768      768
    //   total    ~26,750   27,416     the ceiling is 30,000
    //   share       61%     72.6%
    // ```
    //
    // Two of the three moved the same way. Eight-point rings at one subdivision
    // more than halved the body's own mesh — skin without any face refinement is
    // 5,092 triangles — and cloth is cut from body faces, so it halved with it,
    // for free. Against that the head arrived a whole subdivision level coarser
    // and `FACE_PASSES` had to buy that level back, which is the broad first
    // pass. The total barely moved and every ceiling test still passes.
    //
    // **The floor under this is the mouth, and it was measured rather than
    // assumed.** Three cheaper pass sets were built and costed. Dropping either
    // the second broad-front pass or one of the two nose-base passes takes skin
    // to 10,506 and 11,734 — comfortably under the old 0.63 — and both fail
    // `the_mouth_is_wider_than_the_mesh_under_it` at 1.41 cells, which is the
    // terraced lower face of #85 coming straight back. The mouth's band needs
    // six splits under this cage and there is no arrangement of passes that
    // gives it six and lands under 0.63. Dropping one of the two jaw-flank
    // passes is the only cut that holds every face test, and it is worth 1,184
    // triangles and undoes #80's gonion work to get them.
    //
    // So this number has 2.4 points of room, which is thinner than a guard
    // should carry, and the reason it is left thin rather than rounded up is
    // that the next cut is now identified: face refinement is 54% of the whole
    // avatar and 74% of skin. That is where the room is, and it is filed.
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
        share < 0.75,
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
#[ignore = "the target, not the state: the lids are geometry because nothing rigs a lid (#118)"]
fn a_default_avatar_fits_the_webgl2_draw_budget() {
    // The triangle half of the WebGL2 target passes; this half does not, and
    // will not until #118 turns the lid shells into a slice of the skin. It is
    // one draw short, not two: the eye globes are not a defect to be fixed but
    // a material the other three cannot provide. Turn it on — and delete
    // MESH_CEILING — when the lids go.
    let avatar = built(None);
    assert!(
        avatar.budget.meshes <= MESH_TARGET,
        "{} draws against a budget of {MESH_TARGET}",
        avatar.budget.meshes
    );
}
