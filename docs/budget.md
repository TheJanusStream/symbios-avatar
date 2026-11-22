# The triangle economy

What one avatar may cost, where the cost actually sits, and how to measure a
proposal against it. **Do not quote a figure from this file in an argument —
re-measure it.** The instruments below print every number this file names, and
budget figures have a history of being true on the day and wrong within a week.

## The numbers and where they live

| constant | value | where |
| --- | --- | --- |
| `TRIANGLE_TARGET` | 25,000 | `tests/budget.rs` - the number a person is judged by (WebGL2 tier); 33,500 until milestone #16 banked Round B's savings (#308) |
| `BEAST_TRIANGLE_TARGET` | 29,800 | `tests/budget.rs` - a beast's own since milestone #16: the dearest bald beast with fur's 9,000 ceiling on it (29,452 swept, 29,608 over 300 rolls) and a cushion |
| `TRIANGLE_CEILING` | 23,600 | `tests/budget.rs` - the ratchet: today's high-water mark (23,428 at milestone #16; 32,452 at generation 12, #486), not a goal |
| `MESH_TARGET` / `MESH_CEILING` | 4 draws | `tests/budget.rs` - skin, hair, cloth, eye; each justified by a material the others cannot provide |
| `hair::clump::MAX_TRIANGLES` | 3,300 | `src/hair/clump/mod.rs` - what is left for hair once everything else is paid for, measured at the *dearest* body, not the default, and re-measured by a test rather than quoted; it also has a *floor*, the dearest head a re-roll can grow, which the tier must never trim; and since the tier holds every legal head of hair to it, the product corner is the dearest bald body plus it (#489) |

The figures as re-measured at milestone #16's Round B (2026-11-22: #189 with
the lip band halved too, #493, #494 and #498 in), with the constants above
re-based on them and #489's readings in brackets - stale the moment anything
lands, re-run:

- default body: **20,878** [28,348]
- dearest sweep corner (built, rolled hair): **23,428** (seed 2 as rolled,
  lean thin tall at frame 0) [32,452 on the same seed's long broad head, which
  the face's refinement cost most and which gave most back]
- dearest bald body: **21,124** (the same body) [30,126] - leaving **3,876**
  under the target, which is what the hair ceiling is the leftover of
- greedy-hair body: **22,706** [30,216], the default body in the dearest hair
  derived on its own head (a bob with no fringe at droop 0, Natural brows, a
  Handlebar, a Full chin and FullConnect flanks)
- dearest *product* corner: **24,424**, 576 under the target - the dearest
  bald body plus the hair ceiling (21,124 + 3,300), which the tier holds every
  legal head of hair to within `TIER_SLACK` (#489) [33,426]. Built on that
  body, the dearest hair derived there asks **4,486** untiered [4,738], is
  tiered to 3,264 and reads 24,388 (a crop at droop 0.5, Natural brows, a
  Handlebar, a Full chin and FullConnect flanks); the most of a head the tier
  cannot thin, every region in its solid style, is **2,934** (seed 9 long
  broad, lean thin tall at frame 0), so the tier never has to
- the hair ceiling's floor: a helmet on a rolled face, **3,276** (seed 114 as
  rolled, a bun, at generation 12, #486; seed 102 at #351). Not re-measured in
  Round B: it is measured at each seed's own cut on 300 rolled faces
- the far hair tier (`AvatarConfig::far_hair`, #350): at most **812** over the
  rail's three heads (a tied-back tail on seed 7 as rolled, at its own cut);
  never more than the near tier it stands for, and not counted against
  `MAX_TRIANGLES`
- the dearest beast: **28,960** built (a horse, every axis low at 0.05 m),
  **20,452** bald (seed 18), unchanged

How Round B got there, on the default body and the dearest corner: #189's
variant C 22,862 and 25,498; #493 22,130 and 24,710; the lip band halved too
(variant A, the owner's pick of 2026-11-22) 20,878 and 23,428; #494 and #498
move no triangle. Beasts and winged bodies did not change. The owner's call
on how to spend it (#308, 2026-11-22): bank it, keep the hair ceiling at
3,300, and lower the target into the window the leftover test allows - the
dearest bald body plus 3,333 (the product bound, with `TIER_SLACK`) to plus
4,125 (the 80 percent line), 24,457 to 25,249 - and give beasts their own.

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

Re-measured at milestone #16's Round B (2026-11-22), with #489's readings
(generation 12, 2026-11-21) in brackets:

- the rolled population, bald: as rolled p50 19,342 / p90 20,256 / p99 20,706,
  dearest 20,950 (seed 207) [29,662]; long broad 20,270 / 20,726 / 21,212,
  dearest **22,084** (seed 207) [30,892], **960 past the swept corner** [766].
  Seed 207 is the heavy, fat, feminine tail reached by a natural roll (#491:
  femininity +1.05, mass +1.47, body fat 0.477), and the whole of its excess
  is the top over its swelled chest's refinement, which the top's closed
  shell drew twice until #493 thinned the inner wall. With the hair ceiling
  on it, it would read 25,384, 384 past the target
- the head envelope's ends past the (1, 1) pin: at (2, 2) p99 21,712, dearest
  22,364 (seed 207) [32,232]; at (3, 3) p99 22,042, dearest 22,440 (seed 264)
  [32,476]. A roll reaches about +-2.2 and a record +-3
- beasts per family, built p99 23,580 / 23,986 / 23,418 (dog, cat, horse),
  dearest 24,186; bald dearest 20,608 (cat seed 289), 156 past the sweep's,
  29,608 with fur's 9,000 on it: inside the beasts' own target with 192 to
  spare. No beast changed in Round B
- **the swept corners over eighty seeds rather than twenty**: the lean, thin,
  tall corners at both of the product's heads, bald, p50 20,130 / p90 20,700 /
  p99 21,006 of 800, dearest **21,172** (seed 31 long broad, frame +2), 48
  past the twenty-seed sweep's [30,126, the body the sweep pins]; bounded at
  **24,472** with the hair ceiling on it, inside the target with the slack,
  and the dearest hair derived on each of the eight dearest builds at most
  24,434 (the same body)
- **the heavy, fat, feminine, tall corner, a documented tail** (owner's call,
  2026-11-06: reported, not pinned). With each seed's OWN rolled hair over
  seeds 0-79: p50 22,806 / p90 23,686 / p99 24,824 of 320, dearest **25,150**
  (seed 64 long broad, frame +1), **2 of the 320 past the 25,000 target** [11
  past 33,500; 10 at #353]; the probe prints that count each time it runs.
  The 80 percent line is read on the swept corners, so the target cannot rise
  to cover this tail without the hair ceiling rising with it.

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
    the swept lean corners over seeds 0-79 bald, the product bound on the
    dearest of them and the dearest hair derived on each of the eight dearest,
    built, the heavy corner the sweeps do not pin over the same
    seeds with its own hair, and 300 rolled beasts per family, built and bald,
    each as p50 / p90 / p99 and the dearest; for people, how many wear a
    garment on head-owned skin (none, since #353). About four minutes.

cargo run --release --example refinecost [-- SEEDS] [--skull B,L] [--corner F]
    per-pass cost of face refinement beside the cell it buys in each feature
    band, reported ACROSS and DOWN separately (the faces are 2:1; one median
    lies). `--skull 1,1` pins the long broad head the sweeps pin, `--corner F`
    the lean, thin, tall composites at frame F, so `2 --skull 1,1 --corner 0`
    is the sweep's dearest corner.
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
target while every budget test in this file passed**. The fix was a test that
costed every style in every catalogue on two bodies and failed naming the
winner, so the corner was derived rather than picked. **And a record derived on
two bodies is picked again on the third** (#489): over seeds 0-39 as rolled, 24
heads have a scalp style more than 3 % dearer than the bun that record wore (a
crop 2,354 on seed 2, against 1,748), and the cut was picked too, since droop is
not monotone (a crop on the dearest body costs 3,066 at droop 1 and 3,194 at 0).
So `Head::dearest_hair` derives the dearest legal hair on the head a figure is
argued for - every style of every catalogue at droop 0, 0.5 and 1, region by
region - and the product corner stops depending on any one record: the tier
holds every legal head to `MAX_TRIANGLES`, so the corner is the dearest bald
body plus that ceiling, and the derived hair is what shows the ceiling is
reached and held there. Any new catalogue anywhere owes the same.

That the tier holds every legal head is sampled, not proven: `tiered` stops
after `TIER_PASSES` scaling passes with no final clamp, and a shell it cannot
thin slows it. #489's critic grew 2,080 random legal records on the dearest
bodies - every helmet, card beards, droop 0 to 1, every region parameter at
-1 to 1, asks up to 3,892 - and none grew past 3,300; several grew exactly
3,300. There is no headroom, so a new style or region owes that sample again.
And under CI's libm (the gate's `--libm` nudge, down one place) the hair
derived on the dearest body grew 3,306, six over: the tier holds a head to the
ceiling within `TIER_SLACK` (33, one percent, in `tests/budget.rs`), and the
product bound must fit under the target with that slack on it - 33,459 at
#489, 24,457 since milestone #16 re-based the target.

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
rolled face does (2,832 against 3,276, #351). The same fact bounds the product
from the other side (#489): the tier holds a head to the ceiling only if what it
cannot thin fits under it, so the product test grows every region in its solid
style - a bun's shell and the sculpted brow, lip, chin and flank solids - on
every swept body, and it has to fit: 2,934 at most, against 3,300.

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

**And the cloth itself was paid for twice over a refined trunk**
(#491 and #493). A garment is a closed solid, and its inner wall was cut face for face
from the same body faces as its outer shell, so every refinement pass under a
top - `torso::refine_chest` on every body, `torso::refine_lobe` on a swelled
chest - was drawn on both shells, though no camera reaches the inner one but
at a hem. `Garment::thin_inner` merges the passes' quartered faces back on the
inner wall wherever the skin over it is not drawn, keeping the hem's row, the
rims and the outer shell exactly as sewn, and refuses any merge that would put
the wall further out than the skin or through the outer shell. Measured with
`garmentaudit` on seed 1, cloth now runs from 3,178 triangles sleeveless in the
shortest shorts to 4,322 at the wrists and ankles (3,896 to 5,048 before it),
and the skin it hides is unchanged. The top's inner wall on the default body
went from 1,622 triangles to 890, on seed 207 at the long broad pin from 2,946
to 1,240; the trousers have no refinement under them and do not change. The
chest pass now costs 610 on the dressed default body, where it cost 1,138.

**Where the levers are**:

1. **The jaw-flank cut** - 1,284 triangles, but it undoes the refinement that
   gave the gonion its corner; a render judgement, not a number.

Spent, and listed so it is not proposed again: **the skin under the clothes**,
1,216 net on the default body; and **anisotropic refinement** (#189), the
second nose-base pass halving its band DOWN only instead of quartering it:
on the face refinement alone 5,014 on the default body and 6,432 at the
dearest corner (`refinecost` row 10, 18,972 to 13,958 and 21,638 to 15,206),
and on the built bodies 4,596 to 6,986 a person over 122 sampled (median
5,638); then the lip band halved DOWN as well, the owner's pick by render
(2026-11-22): another 1,052 on the default body and 1,072 at the dearest
corner, the lips 1.42 by 1.13 mm. It was first measured tripping two guards,
on a sawn lip parting; with the parting on its curve (#494) the moustache
reads 1.16 against its guard's 1.25 and the mouth guard passes it at 2.05
rows. See `face::skull`'s `Split::Down`. Beasts and winged bodies do not
change. And **the inner wall's share
of the trunk's refinement** (#493): 732 on the default body, 788 at the swept
corner, 1,706 on seed 207 at the long broad pin and about 2,000 on the heavy
corner; what is left on the inner wall is what the guards above would not
merge, mostly round the swelled chest's fold and the armpit.
