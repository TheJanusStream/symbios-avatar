# The triangle economy

What one avatar may cost, where the cost actually sits, and how to measure a
proposal against it. **Do not quote a figure from this file in an argument —
re-measure it.** The instruments below print every number this file names, and
budget figures have a history of being true on the day and wrong within a week.

## The numbers and where they live

| constant | value | where |
| --- | --- | --- |
| `TRIANGLE_TARGET` | 32,200 | `tests/budget.rs` - the number the engine is judged by (WebGL2 tier) |
| `TRIANGLE_CEILING` | 30,200 | `tests/budget.rs` - the ratchet: today's high-water mark, not a goal |
| `MESH_TARGET` / `MESH_CEILING` | 4 draws | `tests/budget.rs` - skin, hair, cloth, eye; each justified by a material the others cannot provide |
| `hair::clump::MAX_TRIANGLES` | 3,300 | `src/hair/clump/mod.rs` - what is left for hair once everything else is paid for, measured at the *dearest* body, not the default, and re-measured by a test rather than quoted; it also has a *floor*, the dearest head a re-roll can grow, which the tier must never trim |

The figures as reconciled at the 0.9.0 release slice (#351, 2026-09-16)  - 
stale the moment anything lands, re-run:

- default body: **28,444**
- dearest head-axis corner: **30,116** (seed 1, long broad)
- dearest bald body: **28,708** (seed 1, long broad) - leaving 3,492 under the
  target, which is what the hair ceiling is the leftover of
- greedy-hair body: **30,314**
- dearest *product* corner - greediest legal hair on the dearest head:
  **31,962** (seed 1, long broad)
- dearest legal hair, untiered: **3,474** (seed 29, long broad); the test prints
  seed 42's, 3,390
- the hair ceiling's floor: a helmet on a rolled face, **3,276** (seed 102 as
  rolled, a bun); on the tier test's fixed beard, 2,832
- the far hair tier (`AvatarConfig::far_hair`, #350): at most **664** on a
  rolled face, **2,232** under the greediest face and cut; never more than the
  near tier it stands for, and not counted against `MAX_TRIANGLES`

**What these figures do not cover** (#353, open): every sweep in
`tests/budget.rs` visits six seeds, and they are not the dearest bodies a
re-roll makes. Over 300 rolled seeds, bald bodies reach **31,722** as rolled
(seed 175) and **32,408** on a long broad head (seed 237); built as rolled, 14
of 300 exceed the ceiling and 3 the target (65 and 6 at long broad), the
dearest **33,224** and **34,112**. It is the body, not the hair. Until #353
lands, the constants above hold against the corners the suite measures.

## How to measure

```text
cargo test --release --test integration budget:: -- --nocapture
    prints: the default body, the dearest sweep corner, the greedy-hair body,
    and the dearest product corner — the four numbers any proposal is argued against.

cargo run --release --example garmentaudit [-- SEEDS] [--cuts]
    what the clothes cover and what the body therefore stops drawing: the claim,
    the row given back along the hem, and the net — beside the hem's own
    coarseness, which is what that give-back buys.

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
same was never done for the rest of a body: #351 rolled 300 seeds and found
bald bodies 3,700 triangles past the dearest the six reach (#353). Before a
ratchet is trusted, roll a population and compare its tail with the sweep's
corner.

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

**A carve is not free just because it adds no vertices.** The skin is
bit-identical under a displacement-only change, but the hair's scalp is
measured off the *built* skull — a deeper nose resampled hair at +40 triangles
on the default and +96 on the greedy record.

**A garment is paid for twice unless the body stops drawing what it covers.**
Cloth is about 3,200 triangles on a default outfit and the skin
beneath it was another 1,490 that no camera could reach. Suppression is whole
faces — the cut takes whole faces — less the row the hem runs through, which has
to stay because the hem is smoothed off the boundaries it was cut along. The
saving follows the cut: 664 claimed for bare sleeves and shorts, 1,618 for
wrists and ankles, so the *dearest* outfit is also the one that gives most back.

**Where the levers are** (in order of size):

1. **Anisotropic refinement.** The face's quads are 2:1 tall and the two
   finest passes cost 8,120; a directional split would halve the direction the
   mouth needs without paying for the one it does not.
2. **The jaw-flank cut** — 1,284 triangles, but it undoes the refinement that
   gave the gonion its corner; a render judgement, not a number.

Spent, and listed so it is not proposed again: **the skin under the clothes**,
1,216 net on the default body.
