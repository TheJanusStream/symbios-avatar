# The instruments, and the rules for trusting one

This crate is developed measurement-first, and its most repeated failure —
caught more than twenty-five times — is an instrument answering a different
question than its name asks. Every one had been passing; every one was caught
because **a number moved when the geometry had not**, or because two
instruments disagreed. This file is the fleet and the distilled rules.

## The fleet

All under `examples/`, all release-built (`cargo run --release --example …`).

| instrument | what it measures |
| --- | --- |
| `render` | the software renderer: four-view contact sheets, `--head` close-ups, `--pass normal`, clip playback through the real retarget path |
| `headaudit` | head proportions vs the canon, `--sweep` over the guarded seeds, `--axis` walks one record axis |
| `facesection` | nose and mouth in CROSS-SECTION off the built surface — delivered relief, slopes, cells; the instrument for "does this feature exist on the polygons" |
| `chestsection` | the TRUNK in cross-section off the built surface — how far the front stands proud of its own back, how many sides the section shows and how far apart, and the cells there; `--lobe` displaces a synthetic pair onto the mesh as its control |
| `refinecost` | what each face-refinement pass costs and the cell it buys per feature band, across/down |
| `chinprofile` | where the surface turns (`--ring`): curvature per azimuth, the flat-facet detector |
| `garmentaudit` | what the clothes cover and what the body stops drawing for it; the hem as cut against the hem as worn — step, turn, and distance from a smooth ring |
| `follicleaudit` | the five follicle regions on a built head: where each mask lands and what the grown hair occupies against it, `--sweep` over the population; `render -- --follicles` is its visual half |
| `bodyaudit`, `footaudit`, `walkaudit`, `neckaudit`, `column`, `jawprobe`, `envelope` | body regions: proportions, sole contact, gait excursions, neck spans and the junction (collar rim, nape ledge, shoulder slope — `tests/throat.rs` asserts them), midline profiles, jaw shelf, exploration-range envelopes |
| `render -- --close throat` | the collar and the trapezius: front, three-quarter, side and REAR three-quarter; `--head-size` and `--mass` reach the corners the junction was judged at |
| `locomotion` | procedural gait vs baked clips: feet-to-ground across body scales and grades, and what the imported set does between its own frames |
| `measure`, `dump`, `headref`, `reference`, `retargetaudit`, `bakeclips` | scalar dumps, mesh export, reference comparisons, clip retarget checks, artifact baking with its loop-seam and teleport readings |

The second instrument is the Bevy viewer (`../bevy_symbios_avatar`,
`--example viewer`): `--face` framing, `--still`, `--shot`, `--gait/--cadence/
--pace/--phase`, `--clip`. **The two-renderer rule:** a defect visible in one
renderer is the renderer's; a defect visible in both is the body's. No
judgement is final on one instrument.

## The rules

1. **When a number moves and nothing physical did, suspect the measurement
   first.** The converse too: when a real change reads as no change, the
   instrument may be blind to it (a median over the face's 2:1 quads once read
   a halving of every edge as `3.53 → 3.58`).

2. **Ask the surface, not the vertex list.** Bisect against
   `PolyMesh::contains`; never bin vertices in a height window (row spacing
   changes under you and the window comes back empty — and `f32::MIN - f32::MIN`
   is a tidy zero). Share of *area*, never share of *vertices*.

3. **Ask the crate for its constants.** A tool carrying its own copy of a
   fraction, a landmark or a build path measures the crate that existed when it
   was written. Any ruler that builds a body must read the composites:
   `HeadTraits::of(&record.composites)` before `build_body`,
   `FaceParams::on(&traits)` before a carve.

4. **Report a quantity in the units its subject is designed in**, and check
   the units of the number in a failure message before explaining it. Compare
   at equal stature or not at all; before comparing a rate against a reference,
   divide out the reference's size; prefer the quantity itself to a difference
   of it. A reference encodes a style as often as an anatomy — it is not an
   authority on size.

5. **Distrust single-number summaries.** If a population can be bimodal, look
   at the distribution; if a quantity has a direction, report it per direction;
   if a test bound is handed out by step index instead of by anatomy, it is
   true of the span and of no particular body.

6. **Anchor on constructed landmarks, not measured crests.** `Skull::chin` is
   a crest the carve moves — accepted, by design; `Canon` is the ruler features
   are authored against. An instrument anchored on a crest follows the surface
   it is supposed to judge.

7. **A guard per axis does not guard the product** — see `docs/budget.md`.
   If two tests each pin one axis at its worst, write a third that pins both.

8. **Know what each renderer cannot see, and re-ask when it changes.** This
   rule used to read "the software renderer samples only the skin atlas's
   albedo, so every relief change is invisible there by construction" — and
   that was true until #225 and #226 gave it per-texel roughness off `ORM.g`
   and a per-triangle tangent frame off the UVs. Pores and stubble draw now.
   What it still cannot see is anything outside the atlas it is handed, and a
   relief term is only as visible as the light angle chosen for the shot.
   **The rule that survives its own correction is the general one:** a
   renderer's blind spots are a fact about today's code, so before concluding
   that a change is invisible, check whether the thing that made it invisible
   is still there.

9. **Print the number the test computes.** A test that computes its worst case
   and asserts silently makes the figure obtainable only by breaking the
   build; the budget tests print theirs under `--nocapture` for exactly this
   reason. When a bound must tolerate slack, write the measured reason for the
   tolerance beside it.

10. **Read the construction before modelling it.** Issues filed from
    arithmetic about unread code have a perfect record of dying in their own
    first measurement. Everything that survives comes from an instrument.

11. **Render before believing.** Quality defects have passed every test this
    crate owns; when a test disagrees with the render, suspect the test. The
    two-renderer rule above decides whose defect it is.

## Goldens: the render tool's own gate, and why it is local

`examples/render` is the instrument every other judgement leans on, and it has
twice produced a false diagnosis. `--golden` is its insurance: two fixed
questions — the DEFAULT record's standing sheet and its head close-up —
rendered through the very path every other run uses and resolved down one
factor.

```sh
cargo run --release --example render -- --golden bless   # write the fixtures
cargo run --release --example render -- --golden check   # compare against them
```

Run it bare. `--golden` composes with no other flag, because a fixture that
moves with a flag set is not a fixture. `check` exits nonzero on drift and
names the worst channel and the share of pixels past
`GOLDEN_PIXEL_SHARE`; the tolerances are two steps of sRGB per channel and half
a percent of pixels, sized to sit under what an eye can judge and over the
float wobble two machines disagree by.

**The fixtures are deliberately NOT committed** (owner decision, 2026-08-18),
so `tests/golden/` is empty on a fresh clone and there is no `cargo test` gate
behind this. That makes it a *session-local* instrument, and the workflow it
needs is different from a committed golden's:

1. **Bless before you change anything**, on the tree as you found it. A golden
   blessed after a change is a photograph of the change, not a baseline.
2. **Check after**, and read the number rather than the verdict. A shading
   change should move a large share of pixels; a geometry change should move a
   small share in one place. A drift you cannot explain is the finding.
3. **Re-bless deliberately**, having eyeballed the sheets, when the drift is
   the change you meant to make.
4. **Do not trust a green check across a rebuild of the fixtures.** The
   comparison only means something while both sides came from the same bless.

What this costs, stated plainly so nobody rediscovers it as a surprise: a
shading change that lands without a bless-and-check has no baseline at all,
because the previous session's fixtures are gone. The insurance is real but it
is only as good as the habit, which is the trade the owner took to keep binary
fixtures out of the repository.
