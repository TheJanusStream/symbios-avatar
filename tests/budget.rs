//! What one avatar is allowed to cost.
//!
//! The first consuming application is wasm32 on WebGL2, where the ceiling is
//! one draw call per skinned mesh and there is no batching to hide behind. That
//! makes the budget a property of the engine rather than of the app: a body that
//! costs too much cannot be made cheaper by whoever renders it.
//!
//! Two tests, deliberately. One is a **ratchet** at the figure the crate has
//! actually reached, so nothing silently gets worse; the other is the
//! **target**, which was ignored until it could pass and passes now. Writing
//! only the target would leave a red test in CI and teach everyone to ignore
//! it; writing only the ratchet would quietly bless whatever today happens to
//! cost.

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use symbios_avatar::face::{Canon, Skull};
use symbios_avatar::hair::clump::{Bed, Sowing};
use symbios_avatar::hair::{
    BrowStyle, ChinStyle, Cut, FlankStyle, Follicle, Growth, HairRecord, MoustacheStyle,
    ScalpStyle, Tress,
};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3};

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
/// What follows is that ledger, oldest first: each layer's figures were true
/// when written and are corrected by the layers under it, so only the last
/// speaks for the constant.
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
/// the ceiling is [`symbios_avatar::hair::clump::MAX_TRIANGLES`], which tiers
/// the clump counts down when the rest of the axes are expensive. (That constant
/// went with the shell at #202 and came back at #209, for the same reason it
/// existed: a record can ask for hair the budget does not hold.)
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
///
/// **What is left of it, measured 2026-08-11 rather than quoted: 82.** The
/// dearest corner is still seed 42 long broad and it stands at 29,818, of which
/// #181's dorsum band is the last 548 — the figure on this constant's own
/// docstring was 29,886 and the one on #115 was 29,092, and neither was within
/// two hundred of the truth by the time it was next needed. The test below
/// prints it now instead of computing it and throwing it away, so the next
/// proposal can start from a number rather than from a docstring.
///
/// **Down 1,200 because a dressed body stopped drawing the skin under its
/// clothes** (#46/#117). The saving is the whole claim of both garments less
/// the row of faces their hems run through, which has to stay drawn now that
/// the hems are smoothed off the face boundaries they were cut along: 1,490
/// triangles claimed, 274 given back, 1,216 net on the default body and 1,236
/// at this corner. It is a ratchet rather than a target, so it moves down with
/// the measurement and not to the nearest round number; `garmentaudit` prints
/// the claim, the give-back and the net.
///
/// **Up from 28,700 for the jaw-band refinement pass, by owner decision**
/// (#196, 2026-08-12): the tenth `FACE_PASSES` entry costs 2,848 at the
/// dearest sweep corner — 31,430 measured at seed 42 long broad — and the
/// owner chose the smooth crease on the A/B sheet over the arithmetic.
///
/// **And back DOWN to 28,900 when hair became clumps** (#202). A ratchet moves
/// down with the measurement, which is the whole of what makes it one: the
/// dearest sweep corner reads 28,722 now against 31,430, because the shell and
/// its locks cost about 3,000 triangles that 150 scalp clumps do not. The jaw
/// pass has not been given back — it is still in every one of these figures —
/// it is simply no longer sitting on top of the dearest hair the old system
/// could grow.
///
/// **And down again to 27,800, which is fifty over the measurement** (#209). The
/// dearest corner this sweep reaches is 27,750 at seed 1 long broad and has been
/// since #204; the constant was left round while the hair catalogues landed one
/// after another, and a ratchet that is left round is a ratchet with 250
/// triangles of drift in it. Fifty is thin on purpose — a ratchet moves down
/// with the measurement and not to the nearest round number.
///
/// **And UP 50 to 27,850 when a re-roll started drawing the whole scalp
/// catalogue** (#203), which is the direction this constant's own first
/// paragraph says needs a reason written beside it. The reason is that the space
/// being measured grew rather than that a body got dearer: every rolled body
/// wore a `Crop` from #204 until #203, and a crop is the cheapest of the five
/// styles. Rolling all five moved the dearest corner in this sweep from 27,750
/// to 27,786.
///
/// **Thirty-six triangles is the whole cost of it, and that is the interesting
/// number.** The dearest scalp style costs nearly three times a crop per card,
/// so this could as easily have been two thousand; it is thirty-six because
/// #209 sized each style's clump count by what a card of it actually costs. The
/// sizing was done for the greedy corner and this is what it bought at the
/// rolled one.
///
/// Note what this does NOT guard, because it is the corner that mattered at
/// #209 and it is guarded somewhere else now: a record off the network choosing
/// its own hair rather than rolling it. That product is bounded by
/// construction — the dearest bald body plus `hair::clump::MAX_TRIANGLES` — and
/// `the_hair_ceiling_is_what_the_budget_actually_leaves` re-measures both halves
/// rather than ratcheting the product.
const TRIANGLE_CEILING: usize = 27_850;

/// Draw calls the crate currently costs.
///
/// **Four, which is [`MESH_TARGET`], so this constant no longer has a job.** It
/// was five — skin, hair, cloth, eye globes, lids — and #118 gave the four lids
/// joints on the rig and folded their shells into the skin's own mesh, so a
/// blink is a pose rather than a rebuild and the draw went with it. It is kept
/// only so that the ratchet below still names a number of its own; the test
/// that matters is now `a_default_avatar_fits_the_webgl2_draw_budget`, which is
/// no longer ignored.
const MESH_CEILING: usize = 4;

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
    // Said out loud for the same reason the sweep's dearest corner is (#185):
    // the default body's own figure is quoted in half the docstrings in this
    // crate and every one of them goes stale silently.
    println!("the default body: {} triangles", avatar.budget.tris);
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
                avatar.budget.meshes <= MESH_CEILING,
                "{at} costs {} draws",
                avatar.budget.meshes
            );
        }
    }
    // **The triangle assertion is made AFTER the sweep, and printed either way,
    // because computing the dearest corner and then throwing it away is what
    // this test did until #185.** Every proposal that spends triangles on the
    // face has to be argued against the dearest body rather than the default
    // one — the default is 27,788 and the corner it does not visit runs two
    // thousand past it — and the last figure on record was five days and
    // three body changes stale before anyone noticed, because the only way to
    // get the number was to make this test fail. Asserting inside the loop also
    // reported the FIRST corner over the line rather than the dearest, which is
    // a different body on most changes. `cargo test --release --test budget --
    // --nocapture` now says it whether it passes or not.
    println!("dearest corner: {} at {}", worst.1, worst.0);
    assert!(
        worst.1 <= TRIANGLE_CEILING,
        "{} costs {} triangles",
        worst.0,
        worst.1
    );
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
    // room that existed (the shell era's `hair::MAX_TRIANGLES`, 16,500 -> 6,000).
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
    // **0.75 -> 0.80 -> 0.84 because the OTHER parts got cheaper, twice** (#202,
    // #204), which is the one way a share can move that says nothing about the
    // part it names. Skin is 20,384 triangles here and was 20,384 before either
    // hair rewrite, bit for bit. What changed both times is the denominator: the
    // shell and its locks left and took about 3,000 with them, and then a hair
    // element stopped being a swept volume and became one flat card, which took
    // most of what was left — the whole hair layer is now about 1,600 triangles on
    // this body against 2,500 for a scalp of cards and 3,000 for the shell era.
    //
    // So skin's share has risen from 73% to 82% while skin has not moved at all. A
    // ratio test that fires on an improvement elsewhere is measuring something
    // other than its own name. What it still says is worth keeping: if one part
    // ever reaches this share by GROWING, that is where the next cut goes.
    assert!(
        share < 0.84,
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

/// One body, built once, whose hair can be regrown as often as a sweep needs.
///
/// **Because a catalogue sweep is thirty-five haircuts and a body build is half
/// a second** (#209). `greediest` used to hand-pick the dearest variant of each
/// region and say so in its own comment; deriving it instead means costing every
/// style in every catalogue, and doing that through `Avatar::build` would put
/// twenty seconds into a test suite to answer a question about hair.
///
/// So the body is built once and the hair is regrown on it. That is only worth
/// anything if the regrown hair is the hair the body ships, which is not
/// something to assume — see `the_regrown_hair_is_the_hair_the_body_ships`,
/// which asserts it region by region before any sweep reads a number from here.
struct Head {
    avatar: Avatar,
    /// The record it was built from, hair and all.
    ///
    /// Kept whole rather than as a seed, because the body a sweep costs against
    /// has to be reproducible field for field: a `Head` that remembered only its
    /// seed could not say which skull axes were pinned on it.
    record: AvatarRecord,
    skull: Skull,
    canon: Canon,
    /// What to call this body in a report.
    at: String,
}

impl Head {
    /// Builds one body and measures the head hair is grown on.
    fn of(seed: i64, skull: Option<(f32, f32)>) -> Self {
        let mut record = AvatarRecord::new("Ledger", Archetype::default());
        record.reroll(seed);
        let at = match skull {
            Some(_) => format!("seed {seed} long broad"),
            None => format!("seed {seed} as rolled"),
        };
        if let (Archetype::Humanoid(params), Some((breadth, length))) =
            (&mut record.archetype, skull)
        {
            params.head_breadth = breadth;
            params.face_length = length;
        }
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a biped has a head");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        Self {
            avatar,
            record,
            skull,
            canon,
            at,
        }
    }

    /// Grows one head of hair on it, through the same call `Avatar::build` makes.
    ///
    /// `grow_head` rather than a copy of its loop: the five regions come off one
    /// stream in `Follicle::ALL` order and are tiered together to fit the hair
    /// ceiling, and a sweep that reproduced that by hand would be a second
    /// opinion about the one thing that has to match (#89).
    fn regrow(&self, hair: &HairRecord) -> Growth {
        self.regrow_under(hair, symbios_avatar::hair::clump::MAX_TRIANGLES)
    }

    /// The same, under a ceiling of the caller's choosing.
    ///
    /// `usize::MAX` is what a catalogue is ranked under: the tier is a property
    /// of the whole head, and ranking styles through it would rank them by how
    /// much of one ceiling each happened to be granted rather than by what each
    /// costs.
    fn regrow_under(&self, hair: &HairRecord, ceiling: usize) -> Growth {
        let follicles = symbios_avatar::Follicles::of(
            &self.avatar.rig,
            &self.skull,
            &self.canon,
            &hair.regions,
        );
        let bed = Bed {
            body: &self.avatar.parts.body,
            rig: &self.avatar.rig,
            weights: &self.avatar.parts.weights,
            follicles: &follicles,
        };
        let sown: Vec<_> = Follicle::ALL
            .into_iter()
            .filter_map(|follicle| {
                hair.sowing(follicle, &follicles)
                    .map(|sown| (follicle, sown))
            })
            .collect();
        let sowings: Vec<_> = sown
            .iter()
            .map(|(follicle, sown)| Sowing {
                follicle: *follicle,
                count: sown.clumps,
                shape: sown.shape.as_ref(),
                roots: Vec3::from_array(sown.roots),
                tips: Vec3::from_array(sown.tips),
            })
            .collect();
        symbios_avatar::hair::clump::grow_head(&bed, &sowings, self.record.seed, ceiling)
    }

    /// What one region of a head of hair costs on this body.
    ///
    /// **Only the region asked for is lofted, and the regions before it are
    /// scattered and thrown away** (#209). A sweep is seventy haircuts and the
    /// scalp is by far the dearest thing on a head to grow, so costing a
    /// moustache by growing a whole head of hair five times over put seventy-odd
    /// seconds into this file to answer a question about a lip.
    ///
    /// It is exact rather than an approximation, and the reason is narrow enough
    /// to write down: the only thing the regions share is the root stream, the
    /// only thing that draws from it is `scatter`, and `scatter` is handed a
    /// COUNT and a mask — never a style. So advancing the stream by scattering
    /// the earlier regions and discarding their roots leaves this region's own
    /// roots bit-identical, and the regions AFTER it cannot reach backward at
    /// all. `the_cheap_way_to_cost_a_region_agrees_with_the_dear_one` holds it.
    fn region(&self, hair: &HairRecord, follicle: Follicle) -> usize {
        let follicles = symbios_avatar::Follicles::of(
            &self.avatar.rig,
            &self.skull,
            &self.canon,
            &hair.regions,
        );
        let mut stream = Pcg64Mcg::seed_from_u64(self.record.seed as u64);
        for earlier in Follicle::ALL {
            let Some(sown) = hair.sowing(earlier, &follicles) else {
                continue;
            };
            if earlier != follicle {
                // Discarded on purpose: the roots are not wanted, only the
                // stream state they leave behind.
                let _ = symbios_avatar::hair::clump::scatter::scatter(
                    &self.avatar.parts.body,
                    &self.avatar.rig,
                    &self.avatar.parts.weights,
                    &follicles,
                    earlier,
                    sown.clumps,
                    &mut stream,
                );
                continue;
            }
            let bed = Bed {
                body: &self.avatar.parts.body,
                rig: &self.avatar.rig,
                weights: &self.avatar.parts.weights,
                follicles: &follicles,
            };
            let mut growth = Growth::on(follicles.head);
            growth.grow(
                &bed,
                &Sowing {
                    follicle,
                    count: sown.clumps,
                    shape: sown.shape.as_ref(),
                    roots: Vec3::from_array(sown.roots),
                    tips: Vec3::from_array(sown.tips),
                },
                &mut stream,
            );
            // UNTIERED, deliberately: what this answers is what a style ASKS
            // for, which is what a catalogue is ranked by. See `regrow_under`.
            return growth.tris();
        }
        0
    }
}

/// Prints what each region of one head of hair cost.
///
/// **The per-zone ledger** (#209). Every figure in this file until now was a
/// whole body, and a whole body is the wrong unit for the only part of it a
/// record can move: when the greedy corner moved 424 triangles at #207 there was
/// no way to read from any test which region had spent them. `Grown` has carried
/// a per-region breakdown since #202 and nothing printed it.
fn ledger(label: &str, growth: &Growth) {
    println!("{label}: {} triangles of hair", growth.tris());
    for follicle in Follicle::ALL {
        let grown = growth.grown.iter().find(|grown| grown.follicle == follicle);
        let (clumps, tris) = grown.map_or((0, 0), |grown| (grown.clumps, grown.tris));
        println!(
            "  {:<10} {clumps:>4} clumps {tris:>6} triangles",
            follicle.name()
        );
    }
}

/// How far along its own axis each parametric style is costed.
///
/// Ends and middle. An axis does not have to be monotone in what it costs — a
/// bob's fringe at zero is an even curtain, which is the LONGER of its two ends —
/// so sampling one end would be picking a corner again with more steps.
const AXES: [f32; 3] = [0.0, 0.5, 1.0];

/// Every scalp style a record may ask for, costed one at a time.
fn scalp_catalogue() -> Vec<(String, ScalpStyle)> {
    let mut all = vec![("crop".to_string(), ScalpStyle::Crop)];
    for axis in AXES {
        all.push((format!("bob {axis}"), ScalpStyle::Bob { fringe: axis }));
        all.push((format!("long {axis}"), ScalpStyle::Long { weight: axis }));
        all.push((format!("tied {axis}"), ScalpStyle::TiedBack { tail: axis }));
        all.push((format!("curly {axis}"), ScalpStyle::Curly { curl: axis }));
    }
    all
}

/// Likewise the brows, which carry no axis of their own.
fn brow_catalogue() -> Vec<(String, BrowStyle)> {
    vec![
        ("natural".to_string(), BrowStyle::Natural),
        ("thick".to_string(), BrowStyle::Thick),
    ]
}

/// Likewise the upper lip.
fn moustache_catalogue() -> Vec<(String, MoustacheStyle)> {
    let mut all = vec![("chevron".to_string(), MoustacheStyle::Chevron)];
    for axis in AXES {
        all.push((
            format!("handlebar {axis}"),
            MoustacheStyle::Handlebar { sweep: axis },
        ));
        all.push((
            format!("pencil {axis}"),
            MoustacheStyle::Pencil { ride: axis },
        ));
    }
    all
}

/// Likewise the chin.
fn chin_catalogue() -> Vec<(String, ChinStyle)> {
    let mut all = vec![("full".to_string(), ChinStyle::Full)];
    for axis in AXES {
        all.push((format!("goatee {axis}"), ChinStyle::Goatee { point: axis }));
        all.push((
            format!("braided {axis}"),
            ChinStyle::Braided { twist: axis },
        ));
    }
    all
}

/// Likewise the jaw's flanks.
fn flank_catalogue() -> Vec<(String, FlankStyle)> {
    let mut all = Vec::new();
    for axis in AXES {
        all.push((
            format!("sideburns {axis}"),
            FlankStyle::Sideburns { drop: axis },
        ));
        all.push((
            format!("full-connect {axis}"),
            FlankStyle::FullConnect { reach: axis },
        ));
    }
    all
}

/// The cut every catalogue is costed at: the greediest a record may ask for.
const GREEDY: Cut = Cut {
    length: 1.0,
    thickness: 1.0,
    density: 1.0,
    droop: 1.0,
};

/// The dearest hair a record can legally ask for.
///
/// Every region grown, at full length and full density: the corner a record off
/// the network can put a body in, which is what the budget has to survive
/// rather than the default one.
///
/// **The variants here are DERIVED rather than chosen** (#209), and
/// `the_dearest_variant_of_each_region_is_the_one_the_greedy_record_wears`
/// re-derives them by costing every style in every catalogue against this body.
/// Every region carries a catalogue now and the styles inside one do not cost the
/// same — a braid is a third as many clumps however many stations its twist earns
/// — so a budget test that picked whichever variant was written first would be
/// measuring the order of an enum. #208 hand-picked them from four measurements;
/// this is the sweep that issue said it owed.
fn greediest() -> HairRecord {
    HairRecord {
        scalp: Tress {
            style: ScalpStyle::Curly { curl: 1.0 },
            cut: GREEDY,
            ..Default::default()
        },
        brows: Tress {
            style: BrowStyle::Natural,
            cut: GREEDY,
            ..Default::default()
        },
        moustache: Tress {
            style: MoustacheStyle::Handlebar { sweep: 1.0 },
            cut: GREEDY,
            ..Default::default()
        },
        chin: Tress {
            style: ChinStyle::Full,
            cut: GREEDY,
            ..Default::default()
        },
        flanks: Tress {
            style: FlankStyle::FullConnect { reach: 1.0 },
            cut: GREEDY,
            ..Default::default()
        },
        ..HairRecord::default()
    }
}

#[test]
fn the_budget_holds_for_a_record_that_asks_for_the_most_expensive_hair() {
    // The target has to survive a record off the network, not just the default
    // one. Hair is the only part whose cost a record can move, and until #40 it
    // could move it to twice the whole avatar's budget.
    let mut record = AvatarRecord::new("Greedy", Archetype::default());
    record.hair = greediest();
    record.sanitize();
    let avatar = Avatar::build(&record).expect("a biped builds");
    println!("the greedy-hair body: {} triangles", avatar.budget.tris);
    assert!(
        avatar.budget.tris <= TRIANGLE_TARGET,
        "the dearest legal hair brought the body to {} triangles",
        avatar.budget.tris
    );
}

#[test]
fn the_budget_holds_for_the_dearest_hair_on_the_dearest_head() {
    // **NO LONGER IGNORED, as of #204**, which is what this whole hair makeover
    // was for: the epic said it would end with this test un-ignored and passing.
    // The history, since a target that was ignored for four issues is worth being
    // able to read:
    //
    // - 31,482 under the shell era, when hair was a sculpted mass plus locks.
    // - 31,994 at #202, when hair became clumps — worse, and honestly so: the
    //   clumps cost less than the shell and the ratchet came down 2,600, but this
    //   corner is a PRODUCT of two worst cases and it moved the other way.
    // - 31,952 at #205, then 30,512 and 30,028 as a scalp clump became a wide card
    //   that walks the skull rather than a bristle standing off it.
    // - 29,268 here, once an element stopped being a swept volume and became ONE
    //   FLAT CARD (owner call). A swept tube pays `sides x 2` triangles a segment
    //   plus two caps; a card pays two. That is the whole of the last 800.
    //
    // The jaw pass has not been given back, and none of the three givebacks this
    // reason used to name were needed: not the greedy cut's cap, not the tenth
    // FACE_PASSES band, not thinner clumps on the dearest heads.
    //
    // **The corner neither of the two tests above visits, and it was 1,050
    // triangles over target the whole time** (#187). One pins the head's axes
    // at their dearest and rolls whatever hair a seed gives it; the other pins
    // the hair at the dearest a record may legally ask for and builds a DEFAULT
    // head under it. A record off the network is under no obligation to pick
    // one. Measured, every seed in the sweep was over 30,000 with greedy hair
    // on it and the worst was seed 42 long broad at 31,050 — while both tests
    // passed, because a body is not the maximum of its parts.
    //
    // It is a product and it is written as one: the same head axes, the same
    // hair, and the two together. What it caught is that
    // `hair::MAX_TRIANGLES` is defined as what is left over and had been
    // computed on the default body, where the leftover is eight hundred
    // triangles larger than on a long broad seeded one. Its replacement is
    // measured rather than quoted, by
    // `the_hair_ceiling_is_what_the_budget_actually_leaves` (#209).
    let mut worst = (String::new(), 0);
    for seed in [1, 7, 23, 29, 42, 99] {
        for (name, breadth, length) in [
            ("as rolled", None, None),
            ("long broad", Some(1.0f32), Some(1.0f32)),
        ] {
            let mut record = AvatarRecord::new("Greedy", Archetype::default());
            record.reroll(seed);
            if let Archetype::Humanoid(params) = &mut record.archetype {
                params.head_breadth = breadth.unwrap_or(params.head_breadth);
                params.face_length = length.unwrap_or(params.face_length);
            }
            record.hair = greediest();
            record.sanitize();
            let avatar = Avatar::build(&record).expect("a biped builds");
            if avatar.budget.tris > worst.1 {
                worst = (format!("seed {seed} {name}"), avatar.budget.tris);
            }
        }
    }
    println!("dearest product corner: {} at {}", worst.1, worst.0);
    assert!(
        worst.1 <= TRIANGLE_TARGET,
        "{} costs {} triangles with the greediest legal hair on it",
        worst.0,
        worst.1
    );
}

#[test]
fn a_default_avatar_fits_the_webgl2_draw_budget() {
    // **No longer ignored, as of #118.** This was written as the target rather
    // than the state, against a body that drew five, and it stayed ignored for
    // nine days on one draw: the lid shells, which moved without a joint to move
    // them. They have joints now. The four that remain are the four the owner's
    // 2026-08-02 decision named, each justified by a material the others cannot
    // provide — skin takes the atlas, hair is alpha-tested, cloth has its own
    // roughness, an eye is glossy.
    let avatar = built(None);
    assert!(
        avatar.budget.meshes <= MESH_TARGET,
        "{} draws against a budget of {MESH_TARGET}",
        avatar.budget.meshes
    );
}

#[test]
fn the_regrown_hair_is_the_hair_the_body_ships() {
    // **The instrument check the sweep below rests on, and it is not a
    // formality** (#209). `Head::regrow` is a second copy of the loop in
    // `Avatar::build`, written so that costing thirty-five haircuts takes one
    // body build rather than thirty-five. A second copy of a loop is a second
    // opinion about what the body does, and the way that goes wrong is silent:
    // the roots come off one shared stream in `Follicle::ALL` order, so a copy
    // that seeds its own stream per region, or visits the regions in a different
    // order, or reads the follicle masks from `FollicleParams::default()` rather
    // than from the record, still grows a plausible head of hair — of the wrong
    // cost, on every row of a ledger that looks right.
    //
    // So it is checked against the shipped article, region by region rather than
    // in total, on two bodies and three haircuts. A total can agree while two
    // regions are swapped.
    for head in [Head::of(0, None), Head::of(42, Some((1.0, 1.0)))] {
        for (what, hair) in [
            ("the default", HairRecord::default()),
            ("the greediest", greediest()),
            ("a bald head", HairRecord::bald()),
            // **A fourth, because the three above all carry the DEFAULT
            // regions** and a copy of the loop that read the masks from
            // `FollicleParams::default()` instead of from the record would agree
            // with the body on every one of them. The masks are what decide
            // where a root may land and how many find a seat, so reading the
            // wrong ones is the mutation this test most needs to catch and was
            // the one it could not.
            ("a moved hairline", {
                let mut hair = greediest();
                hair.regions.scalp.line = -0.8;
                hair.regions.scalp.temples = 1.0;
                hair.regions.flanks.cheek = 1.0;
                hair.regions.chin.under = 1.0;
                hair
            }),
        ] {
            let mut record = head.record.clone();
            record.hair = hair;
            record.sanitize();
            let shipped = Avatar::build(&record).expect("a biped builds");
            let regrown = head.regrow(&record.hair);
            let shipped = shipped
                .parts
                .hair
                .map(|growth| growth.grown)
                .unwrap_or_default();
            let regrown: Vec<_> = regrown
                .grown
                .into_iter()
                .filter(|grown| grown.tris > 0)
                .collect();
            let shipped: Vec<_> = shipped.into_iter().filter(|grown| grown.tris > 0).collect();
            assert_eq!(
                regrown, shipped,
                "regrowing {what} on {} is not what the body ships",
                head.at
            );
        }
    }
}

#[test]
fn the_dearest_variant_of_each_region_is_the_one_the_greedy_record_wears() {
    // **The sweep #208 said it owed** (#209). Every region carries a catalogue
    // and the styles inside one do not cost the same: a braid is a third as many
    // clumps however many stations its twist earns, a pencil line is a fraction
    // of a chevron, and a bob's fringe is LONGER at zero than at one. `greediest`
    // named four of them by hand off four measurements, which is a corner picked
    // rather than found — and the corner it picks is what both target tests are
    // argued from.
    //
    // So every style in every catalogue is costed, at the greediest cut a record
    // may legally ask for, and `greediest` has to be wearing the winner of each.
    // On two bodies, because a region's ranking is a property of the head it
    // grows on and the dearest body is not the default one: a long broad skull
    // has more scalp to cover and a longer jawline to run a beard down.
    //
    // The failure message names the variant that beat the one in `greediest`, so
    // the fix is to paste it in rather than to go and measure again.
    let wearing = greediest();
    let mut beaten: Vec<String> = Vec::new();
    for head in [Head::of(0, None), Head::of(42, Some((1.0, 1.0)))] {
        println!("\n=== the catalogue on {} ===", head.at);
        let mut dearest: Vec<(Follicle, String, usize)> = Vec::new();

        macro_rules! sweep {
            ($follicle:expr, $field:ident, $catalogue:expr) => {{
                let follicle = $follicle;
                let mut best = (String::new(), 0usize);
                for (name, style) in $catalogue {
                    let mut hair = wearing;
                    hair.$field.style = style;
                    hair.sanitize();
                    let tris = head.region(&hair, follicle);
                    println!("  {:<10} {name:<16} {tris:>6} triangles", follicle.name());
                    if tris > best.1 {
                        best = (name, tris);
                    }
                }
                dearest.push((follicle, best.0, best.1));
            }};
        }

        sweep!(Follicle::Scalp, scalp, scalp_catalogue());
        sweep!(Follicle::Brows, brows, brow_catalogue());
        sweep!(Follicle::Moustache, moustache, moustache_catalogue());
        sweep!(Follicle::Chin, chin, chin_catalogue());
        sweep!(Follicle::Flanks, flanks, flank_catalogue());

        let worn = head.regrow_under(&wearing, usize::MAX);
        ledger(&format!("\nwhat `greediest` wears on {}", head.at), &worn);
        for (follicle, name, tris) in dearest {
            let wears = worn
                .grown
                .iter()
                .find(|grown| grown.follicle == follicle)
                .map_or(0, |grown| grown.tris);
            if wears < tris {
                beaten.push(format!(
                    "on {} the {} catalogue's dearest style is `{name}` at {tris} triangles \
                     and `greediest` wears one costing {wears}: put `{name}` in it",
                    head.at,
                    follicle.name()
                ));
            }
        }
    }
    // Collected over both bodies and asserted at the end rather than inside the
    // loop, for the reason `the_ceiling_holds_across_the_parameter_space` gives
    // for the same shape: asserting as it goes reports the FIRST region that is
    // wrong and hides every other, and the whole point of a sweep is the table.
    assert!(beaten.is_empty(), "{}", beaten.join("\n"));
}

#[test]
fn the_cheap_way_to_cost_a_region_agrees_with_the_dear_one() {
    // `Head::region` grows one region and scatters the rest away, which is the
    // whole reason the catalogue sweep finishes. It rests on a claim about the
    // root stream — that only `scatter` draws from it, and that it is handed a
    // count rather than a style — and a claim of that shape is exactly the kind
    // that stays true until somebody adds a draw somewhere else.
    //
    // So it is checked against growing the lot, on the two bodies the sweep
    // uses, for every region, on a record where every region is grown and the
    // counts differ region to region.
    for head in [Head::of(0, None), Head::of(42, Some((1.0, 1.0)))] {
        let hair = greediest();
        let whole = head.regrow_under(&hair, usize::MAX);
        for follicle in Follicle::ALL {
            let dear = whole
                .grown
                .iter()
                .find(|grown| grown.follicle == follicle)
                .map_or(0, |grown| grown.tris);
            assert_eq!(
                head.region(&hair, follicle),
                dear,
                "costing the {} alone on {} disagrees with growing the whole head",
                follicle.name(),
                head.at
            );
        }
    }
}

#[test]
fn the_hair_ceiling_is_what_the_budget_actually_leaves() {
    // **A leftover-defined ceiling goes stale silently, and the last one did**
    // (#187: `hair::MAX_TRIANGLES` was computed on the default body, where the
    // leftover is eight hundred triangles larger than on a long broad seeded
    // one, and nothing noticed for weeks). So the leftover is re-measured here
    // rather than quoted, on the dearest body the sweep reaches, and the
    // constant has to be under it.
    //
    // Measured as the body WITHOUT hair rather than as a subtraction from a
    // haired one, because a tiered head of hair is not the hair the sum was
    // taken over.
    let mut worst = (String::new(), 0);
    for seed in [1, 7, 23, 29, 42, 99] {
        for (name, breadth, length) in [
            ("as rolled", None, None),
            ("long broad", Some(1.0f32), Some(1.0f32)),
            ("long narrow", Some(-1.0), Some(1.0)),
            ("short broad", Some(1.0), Some(-1.0)),
        ] {
            let mut record = AvatarRecord::new("Bald", Archetype::default());
            record.reroll(seed);
            if let Archetype::Humanoid(params) = &mut record.archetype {
                params.head_breadth = breadth.unwrap_or(params.head_breadth);
                params.face_length = length.unwrap_or(params.face_length);
            }
            record.hair = HairRecord::bald();
            record.sanitize();
            let avatar = Avatar::build(&record).expect("a biped builds");
            if avatar.budget.tris > worst.1 {
                worst = (format!("seed {seed} {name}"), avatar.budget.tris);
            }
        }
    }
    let leftover = TRIANGLE_TARGET - worst.1;
    println!(
        "the dearest bald body: {} at {}, leaving {leftover} for hair",
        worst.1, worst.0
    );
    assert!(
        symbios_avatar::hair::clump::MAX_TRIANGLES <= leftover,
        "the hair ceiling is {} against a body that leaves {leftover} at {}",
        symbios_avatar::hair::clump::MAX_TRIANGLES,
        worst.0
    );
    // And it is not so far under the leftover that hair is being starved of room
    // the body is not using. If this fires, the body got cheaper and the ceiling
    // should follow it up — which is a decision, not an automatic one, since
    // every triangle it takes back is one the face cannot have later.
    assert!(
        symbios_avatar::hair::clump::MAX_TRIANGLES * 5 >= leftover * 4,
        "the body leaves {leftover} triangles for hair and the ceiling only \
         spends {}",
        symbios_avatar::hair::clump::MAX_TRIANGLES
    );
}

#[test]
fn the_tier_bites_only_where_a_record_asks_for_more_than_the_budget_holds() {
    // **The whole claim the tier rests on** (#209). A ceiling that trimmed
    // ordinary heads would be the smaller-counts answer wearing a ceiling's
    // name: every scalp style's count and width were tuned by render at #204 and
    // #210, and a tier that shaved a rolled body would pay for one unreachable
    // record with every reachable one.
    //
    // So: nothing a re-roll can produce is trimmed, on any of the styles, at any
    // cut a re-roll can reach. Measured as the tiered growth against the same
    // growth with no ceiling at all — the same head, region for region — because
    // a total that matches can still be two regions trading.
    for head in [Head::of(0, None), Head::of(42, Some((1.0, 1.0)))] {
        for (name, style) in scalp_catalogue() {
            for density in [0.0, 0.5, 1.0] {
                let mut hair = HairRecord::default();
                hair.scalp.style = style;
                hair.scalp.cut.density = density;
                // Everything a re-roll can put on a face at once, so this is not
                // a scalp on a bare head.
                hair.moustache.style = MoustacheStyle::Chevron;
                hair.chin.style = ChinStyle::Full;
                hair.flanks.style = FlankStyle::FullConnect { reach: 0.5 };
                hair.sanitize();
                let free = head.regrow_under(&hair, usize::MAX);
                let tiered = head.regrow(&hair);
                assert_eq!(
                    tiered.grown,
                    free.grown,
                    "the tier trimmed a `{name}` at density {density} on {}, which costs \
                     {} against a ceiling of {}",
                    head.at,
                    free.tris(),
                    symbios_avatar::hair::clump::MAX_TRIANGLES
                );
            }
        }
    }

    // **And it is a backstop with nothing to reach it, which is the state it is
    // meant to be in.** The dearest legal record costs about 3,050 triangles of
    // hair against a 3,200 ceiling, because #209 sized the scalp's counts by
    // what a card of each style actually costs rather than leaving a tier to
    // clean up after them. That is the right order: a ceiling that bites in
    // ordinary use is smaller counts wearing a ceiling's name, and it takes them
    // out of whichever region the scaling happens to reach — a dear haircut
    // thinning somebody's beard.
    //
    // So what is checked is that it still WORKS, against a ceiling set low
    // enough to bite, and that every region shrinks together rather than one
    // being shaved out.
    let head = Head::of(42, Some((1.0, 1.0)));
    let free = head.regrow_under(&greediest(), usize::MAX);
    let squeezed = head.regrow_under(&greediest(), free.tris() / 2);
    assert!(
        squeezed.tris() <= free.tris() / 2,
        "squeezed to half of {}, a head of hair still costs {}",
        free.tris(),
        squeezed.tris()
    );
    assert_eq!(
        squeezed.grown.len(),
        free.grown.len(),
        "the tier shaved a whole region out of a head of hair: {:?} against {:?}",
        squeezed.grown,
        free.grown
    );
    ledger("the dearest legal hair", &free);
    ledger("the same, squeezed to half", &squeezed);
}
