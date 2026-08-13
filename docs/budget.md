# The triangle economy

What one avatar may cost, where the cost actually sits, and how to measure a
proposal against it. Collected from the tracker by #188 (2026-08-12); every
figure here was measured on 2026-08-11/12 against the code as it stands. **Do
not quote a figure from this file in an argument — re-measure it.** The
instruments below print every number this file names, and the tracker's history
is a museum of budget figures that were true on the day and wrong within a week.

## The numbers and where they live

| constant | value | where |
| --- | --- | --- |
| `TRIANGLE_TARGET` | 30,000 | `tests/budget.rs` — the number the engine is judged by (WebGL2 tier) |
| `TRIANGLE_CEILING` | 27,850 | `tests/budget.rs` — the ratchet: today's high-water mark, not a goal |
| `MESH_TARGET` / `MESH_CEILING` | 4 draws | `tests/budget.rs` — skin, hair, cloth, eye; each justified by a material the others cannot provide |
| `hair::clump::MAX_TRIANGLES` | 3,200 | `src/hair/clump/mod.rs` — what is left for hair once everything else is paid for, measured at the *dearest* body, not the default (#187). Re-measured by a test now rather than quoted (#209) |

Measured on 2026-08-13 after the hair makeover (stale the moment anything lands
— re-run):

- default body: **25,726**
- dearest head-axis corner: **27,786** (seed 42, long broad)
- dearest bald body: **26,670** (seed 42, long broad) — what the hair ceiling is
  the leftover of
- dearest *product* corner — greediest legal hair on the dearest head: **29,856**

So the working headroom is about 144 against the target at the product corner and
64 against the ratchet at the sweep corner. The product corner is now bounded by
construction rather than by a ratchet: the dearest bald body plus the hair
ceiling, both of which a test re-measures. Anything that spends geometry must
still name what pays for it.

## How to measure

```text
cargo test --release --test budget -- --nocapture
    prints: the default body, the dearest sweep corner, the greedy-hair body,
    and the dearest product corner — the four numbers any proposal is argued against.

cargo run --release --example garmentaudit [-- SEEDS] [--cuts]
    what the clothes cover and what the body therefore stops drawing: the claim,
    the row given back along the hem, and the net — beside the hem's own
    coarseness, which is what that give-back buys.

cargo run --release --example refinecost [-- SEEDS]
    per-pass cost of face refinement beside the cell it buys in each feature
    band, reported ACROSS and DOWN separately (the faces are 2:1; one median
    lies — #185).
```

## The rules, each learned the expensive way

**The cost is quantised by ring.** A band edge lands ON a row of faces or
between two, and the difference is a whole row of quads — hundreds of
triangles. Consequences: nothing may be costed by interpolation or scaling
(#79: a band change costed 188 where the "same" change elsewhere cost 534;
and #185: moving a ceiling by 0.011 profile heights cost 852 and by 0.005
cost 950 — not even in cost order). Build the candidate and measure it.

**A band's cost lives in its azimuth.** The same dorsum band cost 6,196
triangles at a cosine of 0.55, 884 at 0.92 and 548 at 0.97 — all landing on
the same feature (#181). Reach exactly as far round the head as the feature
needs and no further.

**A guard per axis does not guard the product** (#187). The budget held one
test that pinned the head's axes and one that pinned the hair, and the corner
where both are dear was over target on *every* seed while both tests passed.
`the_budget_holds_for_the_dearest_hair_on_the_dearest_head` is the product
written as a product; any new expensive axis needs the same treatment.

**"What is left over" must be computed at the dearest body.** Leftover-defined
ceilings (`hair::clump::MAX_TRIANGLES`) go stale whenever anything else moves,
and the default body's leftover is ~800 triangles more generous than a seeded
long-broad one's. The last leftover-defined ceiling went stale at three times
the room that actually existed, which is why
`the_hair_ceiling_is_what_the_budget_actually_leaves` now re-measures the
leftover in the suite instead of anybody quoting it (#209).

**A budget test wears ONE style out of each catalogue, and it is not the
dearest one unless somebody made it so** (#209). Every hair region carries a
catalogue and the styles inside one do not cost the same: measured at the
greediest legal cut, one scalp card is 15 triangles as a crop, 42 as a tied-back
tail and 65 as a ringlet, because both of the dear ones spend their cost on path
and curvature rather than on count. The greedy record wore a crop — the cheapest
of the five — so the dearest legal record was **32,448 against a 30,000 target
while every budget test in this file passed**. The fix is that
`the_dearest_variant_of_each_region_is_the_one_the_greedy_record_wears` costs
every style in every catalogue on two bodies and fails naming the winner, so the
corner is derived rather than picked. Any new catalogue anywhere owes the same.

**A count is set in cards and paid for in triangles** (#206, #207, #208, #209).
Four of the five hair regions have now had their counts re-set for exactly this
reason, and the scalp's was the expensive one because it is the largest region:
its counts were sized against a crop and spent by styles costing four times as
much. `CROWD` grants each style the count that spends what a crop spends.

**Coverage is area, and width is free** (#208, #209). A card is four triangles a
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
on the default and +96 on the greedy record (#183/#187).

**A garment is paid for twice unless the body stops drawing what it covers**
(#46/#117). Cloth is about 3,200 triangles on a default outfit and the skin
beneath it was another 1,490 that no camera could reach. Suppression is whole
faces — the cut takes whole faces — less the row the hem runs through, which has
to stay because the hem is smoothed off the boundaries it was cut along. The
saving follows the cut: 664 claimed for bare sleeves and shorts, 1,618 for
wrists and ankles, so the *dearest* outfit is also the one that gives most back.

**Where the levers are** (as of 2026-08-12, in order of size):

1. **Anisotropic refinement** — #189. The face's quads are 2:1 tall and the
   two finest passes cost 8,120; a directional split halves the direction the
   mouth needs without paying for the one it does not.
2. **The jaw-flank cut** — 1,284 triangles, but it undoes #80's gonion work;
   a render judgement, not a number.
3. ~~The hair replacement~~ — landed (milestone #6). It redrew the map: hair is
   about 1,600 triangles on a default body against the shell era's 3,000, and
   the whole five-region catalogue at its greediest fits in 3,200.

Spent, and listed so it is not proposed again: **the skin under the clothes**,
1,216 net on the default body (#117, 2026-08-12).

## History in one paragraph

Hair was once 30,208 triangles of a 43,500 body (#40 cut it by sampling each
lock by distance travelled); the sculpted shell took it to ~2,700 (#68); the
two-layer follicle system replaced the shell with five regions of flat cards
(milestone #6), which cost about 1,600 on a default body and are held under
3,200 at the dearest legal record by a tier restored at #209; the
eight-point cage halved the base body and cloth with it while the face bought
a coarser head back with a broad first pass (#107); the face's refinement
bands moved from raw radii to profile heights below the joint (#61), gained a
conversion for landmarks (`face::band_at`, #185) and a ninth pass for the
nose's dorsum (#181). The full ledgers live in the closed issues named above
and in `FACE_PASSES`' own docstrings — the tracker is the archive; this file
is only the map.
