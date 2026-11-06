# The triangle economy

What one avatar may cost, where the cost actually sits, and how to measure a
proposal against it. **Do not quote a figure from this file in an argument —
re-measure it.** The instruments below print every number this file names, and
budget figures have a history of being true on the day and wrong within a week.

## The numbers and where they live

| constant | value | where |
| --- | --- | --- |
| `TRIANGLE_TARGET` | 33,500 | `tests/budget.rs` - the number the engine is judged by (WebGL2 tier) |
| `TRIANGLE_CEILING` | 32,200 | `tests/budget.rs` - the ratchet: today's high-water mark (32,012, 32,056 under the libm nudge), not a goal |
| `MESH_TARGET` / `MESH_CEILING` | 4 draws | `tests/budget.rs` - skin, hair, cloth, eye; each justified by a material the others cannot provide |
| `hair::clump::MAX_TRIANGLES` | 3,300 | `src/hair/clump/mod.rs` - what is left for hair once everything else is paid for, measured at the *dearest* body, not the default, and re-measured by a test rather than quoted; it also has a *floor*, the dearest head a re-roll can grow, which the tier must never trim |

The figures as reconciled at #353 (2026-11-06) - stale the moment anything
lands, re-run:

- default body: **28,380**
- dearest sweep corner (built, rolled hair): **32,012** (seed 2 long broad,
  lean thin tall at frame 0); the six seeds alone reach 29,988 (seed 1 long
  broad)
- dearest bald body: **30,142** (the same body) - leaving **3,358** under the
  target, which is what the hair ceiling is the leftover of
- greedy-hair body: **30,250**
- dearest *product* corner - greediest legal hair on the dearest body:
  **33,440** (the same body), 60 under the target
- dearest legal hair, untiered: the test prints seed 42 long broad's, **3,390**
- the hair ceiling's floor: a helmet on a rolled face, **3,276** (seed 102 as
  rolled, a bun, #351)
- the far hair tier (`AvatarConfig::far_hair`, #350): at most **660** on a
  rolled face; never more than the near tier it stands for, and not counted
  against `MAX_TRIANGLES`
- the dearest beast: **28,960** built (a horse, every axis low at 0.05 m),
  **20,452** bald (seed 18)

**What the sweeps visit** (#353). Every person sweep in `tests/budget.rs` -
the ratchet, the leftover and the product - builds one list, `swept`: six
seeds (1, 7, 23, 29, 42, 99) as rolled at the head corners (as rolled, long
broad at (1, 1), long narrow, short broad; the product takes the first two),
and twenty seeds (0-19, by the owner's call) at five composite corners - lean
(body fat at the bottom of its range), thin (mass -2) and tall (2.2 m) at
femininity -2, -1, 0, +1 and +2 - each as rolled and long broad. The three
person sweeps build 224, 224 and 212 bodies on up to eight threads, about 7
CPU-minutes between them (53 s on this machine's eight threads).
The corners were chosen from what the rolled population's tail turned out to
be: over 300 seeds its dearest bodies were all lean and thin, across the frame
axis, and six seeds drew none of them. The beast sweep pins every axis of each
family at both ends at the shortest and tallest back, beside 24 seeds.

**What the tail was** (#353 step 1). Every refinement pass cost the dearest
bodies within a few hundred triangles of a median body's (seed 68, 27,093).
What differed was the top: its raw claim was about 1,500 triangles on every
body and took no head skin, and `garment::close`'s notch fill, run to a fixed
point, climbed from the collar up the column and over the jaw on lean, thin
bodies - a garment over the chin, cloth at 7,000 to 17,000 where a median body
wears 4,400 (bald 31,338 at seed 175 as rolled, 32,176 at seed 237 long broad,
35,096 at seed 42's lean thin corner). Now, when the fill reaches a head face
above the raw claim's top, it is run again from the raw claim under a level
line square to the neck bone, at the lowest height from the raw claim's top
up that leaves the fewest notches: the collar stands -1 to 16 mm over the raw
claim, against about 7 on a body whose fill never reached the head, and at
most one mirrored pair of notches stays open, where the neck's refined band
meets the coarse surface and the grid's rows step. (Stopped at the head alone,
the fill climbed the whole neck band to the head's ragged boundary: points of
cloth above the collar, 20 to 36 mm over the raw claim, and up to eight open
notches.) `no_top_climbs_onto_the_head` holds all three. Only bodies whose
fill reached the head are cut differently; of the sixteen people
`scripts/compare-published.sh` reads, that is seed 8.

**What they do not visit, printed by `probe_the_rolled_population` so it stays
visible** (owner's call, 2026-11-06):

- the rolled population, bald: as rolled p50 27,074 / p90 28,002 / p99 29,358,
  dearest 29,772 (seed 56); long broad 28,184 / 29,124 / 29,772, dearest
  29,872 (seed 96) - under the swept corner
- the head envelope's ends past the (1, 1) pin: at (2, 2) p99 30,592, dearest
  31,436; at (3, 3) p99 31,370, dearest 31,954 (both seed 96). A roll reaches
  about +-2.2 and a record +-3
- the heavy, fat, feminine, tall corner (mass +2, body fat at the top of its
  range): the swelled chest's own refinement pass and the cloth cut over it,
  31,578 bald at seed 7 long broad, frame +2, on the six seeds - past the
  population's dearest heavy body (seed 96, 29,872), and pinned it would leave
  hair less than its floor under the target; over eighty seeds with each
  seed's own hair, below
- beasts per family, built p99 23,580 / 23,986 / 23,418 (dog, cat, horse),
  dearest 24,186; bald dearest 20,608 (cat seed 289), 156 past the sweep's,
  which leaves fur's 9,000 inside the target with 2,592 to spare
- **the swept corners over eighty seeds rather than six**: the lean, thin,
  tall corners at both of the product's heads, bald, p50 27,948 / p90 29,068 /
  p99 29,804 of 800, dearest **30,142** (seed 2 long broad, frame 0) - the
  body the twenty-seed sweep pins, so the eighty agree with it - and the
  greediest legal hair on it **33,440**
- **the heavy, fat, feminine, tall corner, a documented tail** (owner's call,
  2026-11-06: reported, not pinned). With each seed's OWN rolled hair over
  seeds 0-79: p50 31,888 / p90 32,982 / p99 34,160 of 320, dearest **34,664**
  (seed 2 long broad, frame +1), **10 of the 320 past the 33,500 target**;
  the probe prints that count each time it runs. Pinned, it would need a target past 34,664 before any
  hair is chosen, and the leftover under any target near this one would be
  less than the hair ceiling's floor.

## How to measure

```text
cargo test --release --test integration budget:: -- --nocapture
    prints: the default body, the dearest sweep corner, the greedy-hair body,
    and the dearest product corner — the four numbers any proposal is argued against.

cargo run --release --example garmentaudit [-- SEEDS] [--cuts]
    what the clothes cover and what the body therefore stops drawing: the claim,
    the row given back along the hem, and the net — beside the hem's own
    coarseness, which is what that give-back buys.

cargo test --release --test integration probe_the_rolled_population -- --ignored --nocapture
    the population the sweeps' corners stand for: 300 rolled people bald (as
    rolled, long broad, and the head envelope's ends at (2, 2) and (3, 3)),
    the swept lean corners over seeds 0-79 bald and the greediest hair on the
    dearest of them, the heavy corner the sweeps do not pin over the same
    seeds with its own hair, and 300 rolled beasts per family, built and bald,
    each as p50 / p90 / p99 and the dearest; for people, how many wear a
    garment on head-owned skin (none, since #353). About four minutes.

cargo run --release --example refinecost [-- SEEDS]
    per-pass cost of face refinement beside the cell it buys in each feature
    band, reported ACROSS and DOWN separately (the faces are 2:1; one median
    lies).
```

## The rules, each learned the expensive way

**The cost is quantised by ring.** A band edge lands ON a row of faces or
between two, and the difference is a whole row of quads — hundreds of
triangles. Consequences: nothing may be costed by interpolation or scaling
(measured: a band change costed 188 where the "same" change elsewhere cost 534;
moving a ceiling by 0.011 profile heights cost 852 and by 0.005 cost 950 — not
even in cost order). Build the candidate and measure it.

**A band's cost lives in its azimuth.** The same dorsum band cost 6,196
triangles at a cosine of 0.55, 884 at 0.92 and 548 at 0.97 — all landing on
the same feature. Reach exactly as far round the head as the feature needs and
no further.

**A guard per axis does not guard the product.** The budget once held one
test that pinned the head's axes and one that pinned the hair, and the corner
where both are dear was over target on *every* seed while both tests passed.
`the_budget_holds_for_the_dearest_hair_on_the_dearest_head` is the product
written as a product; any new expensive axis needs the same treatment.

**"What is left over" must be computed at the dearest body.** Leftover-defined
ceilings (`hair::clump::MAX_TRIANGLES`) go stale whenever anything else moves,
and the default body's leftover is ~800 triangles more generous than a seeded
long-broad one's. The last leftover-defined ceiling went stale at three times
the room that actually existed, which is why
`the_hair_ceiling_is_what_the_budget_actually_leaves` re-measures the
leftover in the suite instead of anybody quoting it.

**A budget test wears ONE style out of each catalogue, and it is not the
dearest one unless somebody made it so.** Every hair region carries a
catalogue and the styles inside one do not cost the same: measured at the
greediest legal cut, one scalp card is 15 triangles as a crop, 42 as a tied-back
tail and 65 as a ringlet, because both of the dear ones spend their cost on path
and curvature rather than on count. The greedy record once wore a crop — the
cheapest of the five — so the dearest legal record was **32,448 against a 30,000
target while every budget test in this file passed**. The fix is that
`the_dearest_variant_of_each_region_is_the_one_the_greedy_record_wears` costs
every style in every catalogue on two bodies and fails naming the winner, so the
corner is derived rather than picked. Any new catalogue anywhere owes the same.

**A sweep of six seeds is six bodies, not the population.** The head's axes
are pinned to their dear ends because a seed draws them timidly (#61), and the
same was not done for the composites until #353: #351 rolled 300 seeds and
found bald bodies 3,700 triangles past the dearest the six reached. Before a
ratchet is trusted, roll a population and compare its tail with the sweep's
corner - `probe_the_rolled_population` does - and before pinning a corner,
find out WHAT the tail is: #353's was a cloth defect, which a corner pinned on
it would have blessed.

**A tier cannot squeeze a fixed grid.** The hair ceiling trims a region by
rooting fewer clumps, and a helmet's shell is `COLUMNS x ROWS` whatever it is
asked for (#347). So once a helmet can be worn, the floor under
`MAX_TRIANGLES` is the shell plus everything a re-roll can put beside it, and a
tier test that grows one fixed beard under it measures a smaller floor than a
rolled face does (2,832 against 3,276, #351).

**A count is set in cards and paid for in triangles.** Four of the five hair
regions have had their counts re-set for exactly this reason, and the scalp's
was the expensive one because it is the largest region: its counts were sized
against a crop and spent by styles costing four times as much. `CROWD` grants
each style the count that spends what a crop spends.

**Coverage is area, and width is free.** A card is four triangles a
segment however wide it is, so a region that reads thin wants bigger cards before
more of them — but only where the cards TILE something. Cutting the scalp counts
left a crop and a tail unchanged on the render, because a card that lies on the
skull covers it several times over either way; it left a bob, a curtain and a
coil visibly thin, because a hanging card covers nothing but itself and the count
IS the density of the curtain. Do the arithmetic, then look at it — the
arithmetic was right for two styles out of five.

**A beast's head ornaments come out of its fur's ceiling** (#402). A piece costs
what its section and its stations do: a unicorn's horn is 140, a preset pair of
horns 312, fangs 216 to 248, tusks 352 and a stag's antlers 696 (the
`a_piece_costs_little_against_the_budget` test prints these and each at its
tightest curl, where the dearest, antlers with eight points, is 1,544). Six kinds
at their dearest at once are about 5,500, and on a body with the longest fur that
is more than was left, so `hair::fur::grow`'s ceiling gives up whatever a body wears
past a unicorn's 140: such a body is under the target as a unicorn's always was,
and a body wearing no more than that grows the fur it always grew.

**A carve is not free just because it adds no vertices.** The skin is
bit-identical under a displacement-only change, but the hair's scalp is
measured off the *built* skull — a deeper nose resampled hair at +40 triangles
on the default and +96 on the greedy record.

**A garment is paid for twice unless the body stops drawing what it covers.**
Cloth is about 4,700 triangles on a default outfit and the skin beneath it
another 1,860 that no camera could reach (seed 1, `garmentaudit`, re-measured
for #356). Suppression is whole faces — the cut takes whole faces — less the
row the hem runs through, which has to stay because the hem is smoothed and
placed off the boundaries it was cut along. The saving follows the length: cloth
runs from 3,944 triangles sleeveless in the shortest shorts to 5,096 at the
wrists and ankles, and the skin it hides from 1,480 to 2,052, so the *dearest*
outfit is also the one that gives most back. A texture costs no triangles: it
is baked into the cloth atlas, which is texture bytes, not geometry.

**Where the levers are** (in order of size):

1. **Anisotropic refinement.** The face's quads are 2:1 tall and the two
   finest passes cost 8,120; a directional split would halve the direction the
   mouth needs without paying for the one it does not.
2. **The jaw-flank cut** — 1,284 triangles, but it undoes the refinement that
   gave the gonion its corner; a render judgement, not a number.

Spent, and listed so it is not proposed again: **the skin under the clothes**,
1,216 net on the default body.
