# The instruments, and the rules for trusting one

This crate is developed measurement-first, and its history's most repeated
event — more than twenty-five times as of 2026-08-12 — is an instrument caught
answering a different question than its name asks. Every one had been passing;
every one was caught because **a number moved when the geometry had not**, or
because two instruments disagreed. This file is the fleet and the distilled
rules. Collected from the tracker by #188; the case histories live in the
closed issues and are not repeated here.

## The fleet

All under `examples/`, all release-built (`cargo run --release --example …`).

| instrument | what it measures |
| --- | --- |
| `render` | the software renderer: four-view contact sheets, `--head` close-ups, `--pass normal`, clip playback through the real retarget path |
| `headaudit` | head proportions vs the canon, `--sweep` over the guarded seeds, `--axis` walks one record axis |
| `facesection` | nose and mouth in CROSS-SECTION off the built surface — delivered relief, slopes, cells; the instrument for "does this feature exist on the polygons" |
| `refinecost` | what each face-refinement pass costs and the cell it buys per feature band, across/down |
| `chinprofile` | where the surface turns (`--ring`): curvature per azimuth, the flat-facet detector |
| `garmentaudit` | what the clothes cover and what the body stops drawing for it; the hem as cut against the hem as worn — step, turn, and distance from a smooth ring |
| `follicleaudit` | the five follicle regions on a built head: where each mask lands and what the grown hair occupies against it, `--sweep` over the population; `render -- --follicles` is its visual half |
| `bodyaudit`, `footaudit`, `walkaudit`, `neckaudit`, `column`, `jawprobe`, `envelope` | body regions: proportions, sole contact, gait excursions, neck spans, midline profiles, jaw shelf, exploration-range envelopes |
| `locomotion` | procedural gait vs baked clips: feet-to-ground across body scales and grades |
| `measure`, `dump`, `headref`, `reference`, `retargetaudit`, `bakeclips` | scalar dumps, mesh export, reference comparisons, clip retarget checks, artifact baking |

The second instrument is the Bevy viewer (`../bevy_symbios_avatar`,
`--example viewer`): `--face` framing, `--still`, `--shot`, `--gait/--cadence/
--pace/--phase`, `--clip`. **The two-renderer rule:** a defect visible in one
renderer is the renderer's; a defect visible in both is the body's. No
judgement is final on one instrument.

## The rules

1. **When a number moves and nothing physical did, suspect the measurement
   first.** The converse too: when a real change reads as no change, the
   instrument may be blind to it (a median over the face's 2:1 quads read a
   halving of every edge as `3.53 → 3.58`; #185).

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
   a crest the carve moves (#133, accepted); `Canon` is the ruler features are
   authored against. An instrument anchored on a crest follows the surface it
   is supposed to judge.

7. **A guard per axis does not guard the product** — see `docs/budget.md`
   (#187). If two tests each pin one axis at its worst, write a third that
   pins both.

8. **Know what each renderer cannot see.** The software renderer samples only
   the skin atlas's albedo (#45 open): every relief change is invisible there
   *by construction*. Judge relief in the viewer or with numbers. The viewer's
   vertex colours were sRGB-in-a-linear-channel until 2026-08-11 (viewer#14):
   every in-app colour judgement recorded before that date is void.

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
