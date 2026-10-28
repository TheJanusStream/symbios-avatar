# Quadruped anatomy: what the beast is built to, and where it departs

The quadruped's body (#372's R1 and R2, issues #374 and #375) is built to
measured animals. This file holds the targets, family by family, with the
dossier rows they come from; the owner's decisions that shape them; what the
rig honours today, measured; and every place it departs from the animal, each
with its reason. The dossiers are `docs/research/research-07-quadruped-morphology.md`
(the body at rest, cited here as **07** with its source keys, S1 to S33),
`research-08-quadruped-gait.md` (**08**) and `research-09-quadruped-postures.md`
(**09**). A figure's confidence is the dossier's: H measured and read, M
measured with a caveat or a breed standard, L a rule of thumb, derived or
recalled.

**WH** is the withers height throughout: the ground to the top of the
shoulder blades, standing square. Every length is a share of it.

Measure a body against this file with `cargo run --release --example
quadprobe` (the three families at their anchors; `--appeal 1` at the roll's
default, `--seed N` a rolled record, `--sweep N` how many rolls mesh). Judge it
with `cargo run --release --example render -- --quadruped --gallery 6 --family
dog` (or `cat`, `horse`), and in motion with `--seed 1 --strip 8 --speed 2.5`
(seed 1 is a dog, 2 a cat, 6 a horse).

## The owner's decisions (2026-10-28, on #372)

1. **The body is one slice**, R1 and R2 together: the skeleton and the trunk.
2. **Cute follows the family's young**, not a toy's proportions. A dog's and a
   cat's `appeal` draws a puppy: a head as much bigger as a puppy's, a barrel
   that grows **rounder rather than longer**, under a shorter, thicker neck, on
   legs at the adult's share. A horse's draws a **foal**: a bigger head, a
   **shorter body on longer legs**. A cross blends its two families' rules by
   its blend. The second half of this decision (rounder, not longer; the foal)
   was taken mid-slice, after the probe's rolls read as dachshunds and ferrets.
3. **A horse's Sit** is the dog-sit a rising horse passes through, held, its
   hind legs folded under (R4, #377).
4. **Lie Down** is the sphinx for dogs and cats and the lopsided sternal rest
   for a horse; a dog's relaxed down, rolled onto one hip, is a later variant
   (R4).

And a correction the owner made to the plan: `GENERATOR_VERSION` tracks what a
seed rolls, not how the record's axes become a body, so a body rebuilt in the
derive leaves it at 8. Nothing on the wire changed: `QuadrupedParams` already
carries `family`, `hybrid` and `blend`, which is all the per-family frame
needs.

## How the body is built

- **A frame per family** (`plan::derive::quadruped::Frame`): bone lengths,
  six trunk sections (pelvis, loin, last rib, mid-ribcage, heart girth,
  forechest, each a top, an underline and a half-width), the shoulder blade's
  slope and top, the hip's height, how far the hind paw stands behind it, the
  legs' and the thigh's girth, the neck's. A cross blends two frames in these
  normalised units, as 07's note 11 says to.
- **The stance is solved, not authored** (07 note 1). Fore: the blade hangs
  from its top at its slope, the forearm stands vertical under the withers
  (S11), the upper arm closes the gap. Hind: the paw stands its distance
  behind the hip, the metatarsus rises straight from it (S12), and the stifle
  closes femur and tibia between hip and hock, bent forward. The standing
  angles that come out are checks against the dossier, never inputs.
- **The body's axes are steps from the family's centre**, so a roll at its
  family's centre is the family's frame. Legginess lengthens only the leg
  below the trunk.
- **So are the composites, on the trunk**: the frame's rings are the family's
  at the `mass` and `bodyFat` its rolls centre on, and a body heavier than
  that is wider at every ring and deeper through the barrel under the same
  back; a fatter one most at the belly, whose underline drops. Measured at
  `appeal` 1, a dog at `mass` 1 and `bodyFat` 0.45 has a ribcage girth of 1.64
  against 1.38 and its underline at 0.40 WH against 0.47.
- **The bones inside the trunk are markers** (`Node::marker`): the blade's top
  and the shoulder on a fore leg, the hip joint on a hind one. The rig keeps
  them as joints; the cage sweeps each leg from its girdle straight to the
  elbow or the stifle (a **bridge**), its socket aimed at the shoulder or the
  hip. The skin binds to them as to any bone.

## Targets per family, and what the body measures

`quadprobe`, each family at its anchor and `appeal` 0; fractions of WH as the
built surface measures them, bones joint to joint.

### Dog (07 section 2.1; reference breeds Labrador, Malinois, GSD)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Chest depth | 0.45 to 0.50 | S11, S12 (M, H) | 0.50 |
| Brisket and elbow | the elbow at 0.50, the brisket reaching it | S11 (M) | brisket 0.50; elbow joint 0.44, its point about 0.50 |
| Heart girth | 1.07 to 1.15 | S9, S7 (H) | 1.10 |
| Depth to width at the heart | 1.3 to 1.5:1 (Labrador) | 07 fact 4 (derived) | 1.9:1, see departures |
| Loin against ribcage girth | 0.85 to 0.9 (Labrador), 0.72 (sighthound) | S9, S11 (L-M) | 0.84 |
| Body length, shoulder to buttock | 1.00 to 1.05 | S11, S12 (H/M) | 1.07 |
| Scapula, its slope | 0.25, 55 to 65 deg | S5 (M), bg (L) | 0.26 at 60 deg |
| Humerus, radius | 0.30, 0.31 | S5 (M) | 0.29, 0.30 |
| Femur, tibia | 0.33, 0.34 | S5 (M) | 0.32, 0.33 |
| Metatarsus | 0.15 to 0.18 | derived (L) | 0.16 |
| Shoulder angle | 107 to 120 | S13 (H) | 103 |
| Elbow angle | 130 to 145 | S18 (M) | 138 |
| Stifle angle | 145 +/- 8 | S6 (H) | 139 |
| Hock angle | 134 +/- 9 | S6 (H) | 141 |
| Neck girth | 0.45 to 0.5 of the chest's | S9 (one dog, 07 note 4) | 0.44 |

### Cat (07 section 2.2; domestic shorthair, WH about 26 cm)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Heart girth | 1.3 to 1.5 | S14 (M), derived | 1.33 |
| Depth to width | 1.1 to 1.3:1 | derived (L) | 1.23:1 |
| Loin against chest girth | 1.04 to 1.13, a pouch under it | S14 (M) | 1.00 |
| Chest depth, brisket | about 0.5 | (dog and horse rows) | 0.49, 0.51 |
| Humerus, radius | 0.387, 0.375 | S2 (H) | 0.38, 0.37 |
| Femur, tibia | 0.416, 0.438 | S2 (H) | 0.39, 0.41 |
| Metatarsus | 0.20 to 0.23 | bg (L) | 0.22 |
| Hip height | 0.88 | S21 (M) | 0.84 |
| Stifle, hock, elbow | 110 to 125, 110 to 130, 120 to 140 | bg (L) | 96, 132, 116 |
| Body length | 1.3 to 1.5 | S25 (L, definition unconfirmed) | 1.14, see departures |

### Horse (07 section 2.3; Matsuura's 35 horses, S4)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Chest depth | 0.48 | S4 (H) | 0.50 |
| Heart girth | 1.14 to 1.22 | S4, S3 (H) | 1.08 |
| Depth to width at the girth | about 2:1 | S4, derived | 1.75:1 |
| Barrel | widest behind the girth, no tuck-up | bg (L-M) | ribcage 0.31 wide against the girth's 0.25; loin 0.93 of the ribcage's girth |
| Body length | 1.015 to 1.048 | S4 (H) | 1.05 |
| Scapula, its slope | 0.31 to 0.33, 58 to 61 deg | S4 (H) | 0.31 at 60 deg |
| Humerus, radius | 0.19, 0.24 | S4 (H) | 0.18, 0.22 |
| Femur, tibia | 0.22, 0.32 to 0.34 | S4 (M, H) | 0.20, 0.30 |
| Shoulder, elbow angles | 100 to 105, 133 to 137 | S4 (H) | 100, 135 |
| Stifle, hock angles | 117 to 126, 141 to 159 | S4, S20 (H) | 122, 141 |
| Neck length | 0.53 (pony, withers to poll) | S3 (M) | 0.46 to the occiput joint |
| Neck girth | (no figure) | | 0.51 of the heart's |

### The thigh

07 note 3: a thin-looking body comes from a missing forechest, triceps and
**thigh**; S8 puts a GSD's thigh at 4.5% of its body mass against 2.4% for
the upper arm. No dossier measures a thigh's outline, so its thickness is
**chosen by render**, the horse thickest: from the side, three quarters of the
way down the femur, the dog's thigh is 0.22 WH fore and aft (0.13 across), the
horse's 0.26 (0.15), and the gaskins 0.18 and 0.21. (The cat's thigh is cut
inside its belly at that height, so its row reads the trunk.)

## The young: what `appeal` draws (`plan::derive::quadruped::Schema`)

`appeal` runs 0 at naturalistic to 1.5 at all the way to the young animal
(`young` = `appeal / 1.5`, held to -0.5 below zero, so the other way goes
half as far at most, reached at `appeal` -0.75); a roll centres it at 1.
Below zero the ribcage does not narrow by its column here but by a tenth of
`young`, 5% at most.

| Family | Head | Body length | Ribcage width toward round | Underline | Neck reach, girth | Upper arm, thigh | Cannons | Leg girth, paws |
|---|---|---|---|---|---|---|---|---|
| Dog, cat (puppy) | x1.56 | x1.10 | x1.90, capped at round | sinks 0.05 WH | -30%, +30% | adult share | adult share | +18%, +37.5% |
| Horse (foal) | x1.40 | x0.88 | none | none | -20%, +10% | x0.80 | x1.35 | unchanged |

- **The puppy** (07 fact 10, S7, a 45-day Kars shepherd against WH): head 0.61
  against an adult's 0.39, chest girth 1.58 against 1.15, body 1.44 against
  1.04, legs 0.61 both. The head takes the whole ratio; the girth most of
  it; the length a quarter of it, on the owner's call. Measured at the roll's
  default (`appeal` 1): the dog's heart girth 1.34 and body 1.12, the cat's
  1.61 and 1.21.
- **The foal** (07 foal row, L-M): about 60% of mature WH at birth on cannons
  only about 20% short of their adult length, the humerus and femur about
  half. So against its own WH a foal's cannons are about 1.4 times an
  adult's share and its upper bones about 0.8, and its body is short: the
  cannons lengthen, the upper arm and the thigh shorten, the trunk shortens
  and does not round. How far is chosen by render; the dossier has no foal
  trunk figure. At `appeal` 1 the horse's elbow stands at 0.49 WH against
  0.445, its body is 0.95 long and its heart girth is 0.98, the front of its
  chest 0.05 ahead of its shoulder joints; its stifle folds to 108 degrees against
  the adult's 122, and its shoulder to 87 against 100 - a foal standing on
  legs too long for the hip the frame keeps (a croup-high foal would open
  them; the dossier gives no figure for it).
- 07 note 10's envelope for a cute but believable body (head 0.45 to 0.6, body
  1.1 to 1.4, girth 1.3 to 1.6) holds at the roll's default for a dog and a
  cat. Its "shorten the lower legs" policy is the toy style the owner did not
  take.

## What the rig honours, beyond the body at rest

- **Froude by hip height** (08 fact 10): a leg's reach is now measured from
  the hip and the shoulder joints inside the body, 0.466 m on the default dog
  against P5's 0.284, which put the hip at the trunk's underside. Strides,
  cadences and the gait's transitions follow it. The bob of the trunk at a
  trot is 2.8% of WH on the default dog, 5.3% on seed 2's cat and 2.9% on seed
  6's horse, inside 08's 2 to 6% (fact 7, the horse's); on P5's body it was
  1.0 to 1.5%.
- **A side-pass** (#374): four feet in two pairs cross over to strafe, as a
  horse's legs do, rather than shuffling on a stance too narrow to step in.
- **The scapula is its own joint** (the first upper-limb joint of a fore leg),
  rigid with the girdle in every motion today. R5 (#378) drives it parallel
  to the forearm, the pantograph of 08 fact 2; it already carries the skin
  over it.
- **Sit and Lie Down rest on the ground** (#377): see the next section.

## Resting on the ground (R4, #377)

The owner's verdict on P5's clips: "With Sit and Lie Down, the animal ends up
partially underground and the transition also looks unnatural." They pitched
the trunk a fixed share and dropped it a fixed share of the leg, and nothing
asked where the skin met the ground. Measured on the drawn skin over a sweep
of 33 bodies (each family and a cross, `appeal` -1.5 to 2, `mass` and
`bodyFat` at both ends; `tests/rest.rs`), the skin went under by up to 225 mm
on a dog's sit and 229 lying, a cat's 132 and 240, a horse's 413 and 266;
seed 1's dog 166 (18% of its withers), seed 2's cat 120 lying (27%), and a
lying horse's chest stood 158 to 194 mm off the ground on forelegs sunk 57.

**Posed from the skin, not the joints.** A rig is joints and radii, and the
skin over them misses the radii by a tenth on a dog's thigh and by
centimetres where a folded joint's blended skin bulges or collapses. So a
built beast's rig keeps an outline of its skin (`rig::Outline`): the extremes
of each joint's skin along a fan of directions, and every vertex held near
half and half between two joints, each posed through the skin's own
transforms - about 2,000 points, which read a folded body's lowest skin as
drawn (`rig::outline`'s test). A rig rooted straight from a
skeleton has none, and a rest reads the joints' rings instead, as it does
for an outline with no points in it. Under `serde-avatar` the outline
crosses a worker boundary as one blob of 36 bytes a point: 77 to 83 KB of a
beast's rig in named msgpack, where field by field it was 217 to 233 KB; a
person's rig, which has none, is written byte for byte as before.

**The sit** is solved from the forehand back (`anim::rest`). The forelegs
stand straight over the paws that stay, leaning back 0.12 rad (09: the
forehand moves slightly back and up); a horse's lean 0.4, its forehooves out
ahead of its chest as a rising horse's are. The chest pitches until the
haunches touch the ground, the loin rounding as far again (a horse's 0.3 as
far, its back stiff), and each hind leg folds down from the hip onto a
metatarsus laid flat, the femur sloping 0.1 rad to the stifle and the hock
closing as far as the hip is high - no further than 0.6 rad on paws (a dog's
tarsus to 39 degrees) or 1.0 on hooves, where a horse's hock closed to 15
degrees sank the back of its cannon 38 mm; beyond, the stifle rises toward
the hip instead. **The hind paws step forward under the body** by however far
that leaves them, lifted, before the body moves: 38% of its withers height
on seed 1's dog, 12% on seed 2's cat, 16% on seed 6's horse, the forelegs
leaning 7, 7 and 22 degrees. 09 says the feet stay planted, which is of rising
from a sit (Ellis's protocol, from a crouch); from a natural stand with the
hind paws planted, the first try laid the metatarsi flat behind them, carried
the hips back past the paws and strained the forelegs forward. At the end a dog's or a cat's
haunches, metatarsi and forepaws are on the ground (within 1.1% of its
withers height over the sweep; a slight cat sitting bolt upright rests on
its tail's root, which the haunches count) and nothing is under it; a horse
rests on its hind cannons and forehooves, its croup within 2% of its withers
height of the ground - 09's "on the ground or just off it".

**Lying down**, for a dog or a cat, goes through the sit (09 fact 6): the
first 0.4 of it is the sit, and then the chest comes down between forepaws
that walk out ahead and lay flat, the pelvis levelling last. Its end is solved
for the sternum and the back to touch together, the chest pitching no more
than 0.3 rad; each foreleg's elbow point and paw are fitted to their own skin
on the ground, the forearm sloping from one to the other; and the trunk is
lowered again by what folding the forelegs lifts the brisket. The sternum,
belly, forearms, forepaws and folded hind legs end within 4.0% of the
withers height of the ground (a slight cat's chest at 4.0: the brisket's
skin is bound to the upper arms, and rides a little under the sternum),
nothing under it. **The lying down goes on from the sit exactly**: its trunk
turns from the sit's attitude about hips held where the sit left them and
comes down onto the sphinx's as the pelvis levels, its hind legs fold on from
the sit's own fold; placed by the sphinx's own rule from its first moment, a
dog's hips stood 161.5 mm off where the sit had left them, and the body
jumped there in one frame, going down and getting up.

**A horse lies down its own way** (09, 2.3.3): it gathers its hind feet
under it, drops its forehand onto its knees first - the forearm sloping from
the elbow to a knee on the ground, the cannon laid back under it and the
hoof turned over, sole up - and folds its hindquarters after, rolling its
pelvis 0.3 rad onto its left hip, its chest keeping 15% of the roll. Its
hindquarters end where its dog-sit has them, which it rises through, its
hind legs folded out beside its belly by the length its tibia exceeds its
femur: a horse's hind leg folds no tighter than that, more than a lying
horse's hip stands above a hock beneath it. Its chest comes down between its
knees, short of the ground: lowered as far as its sternum lets it, the trunk
sank a heavy horse's folded hind legs 229 mm into the ground, so it is
raised again until its thighs clear, its forelegs where they were fitted.
The lowest skin of its chest ends 1.0 to 10.2% of its withers height off the
ground over the sweep, the default horse's 3.8% and with its legs at +1 2.1%
(`tests/rest.rs` holds a horse's under 11%).

**Getting up** follows the animal (`emote::getting_up`): out of a sit the hip
opens first, then the stifle, then the hock (09 fact 5: 0 to 20%, 10 to 50%,
25 to 60% of the rise), the hips staying over the hind feet until they step
back under them at the end; out of lying down a dog or a cat puts its
forelegs out first and rises into its sit and up from it; a horse extends one
foreleg ahead of its chest and then the other, rises into its dog-sit and
heaves up from it. Tests hold the order (`anim::rest`).

**Every motion is a continuous function of its time** (#377), and
`tests/rest.rs` sweeps each at 300 steps over the 33 bodies, and the
`Driver` at 60 frames a second, holding every joint to four times
`sqrt(g WH)`, about as fast as a galloping paw swings past its body; the
fastest part of any rest, a rising horse's tail tip sweeping along the
ground, reads 3.6 at that sweep and 4.2 at a 1,200-step one. What had
jumped: the lying down taking over from the sit (above); a horse's loin
finding its bend at the end of its range, and again where its shoulders came
back within its forelegs' reach (its chest now bends as far as a straight
foreleg's length costs against bending, smoothly); a hind leg's pole, which
flipped when a lying horse's hip passed its hock's height and spun about a
rising one's leg (it now turns over through the leg's own side); a tail
tucked under a croup pitched far up, which first swung down through hanging
before it came out of the ground (it hangs no further under than at rest,
or 75 degrees); and a foreleg put back at rest that a contact solve took for
one nothing had posed. A rest poses the joints it owns from rest, every
frame, so two ways to one moment pose it the same.

**Seconds**, a dog's at a hip-to-hock leg of 0.5 m and every body's by the
square root of its own (09 note 12): to sit 1.0 and up 1.1 (Ellis: 1.14),
to lie down 2.4 and up 2.0; a horse's 1.5 and 1.4, 3.0 and 3.0.

**The `Driver` holds a state until something gets the beast up** (#377, the
owner's default, for judging in the viewer). It goes down at its own pace
and stays down. Moved (faster than 0.1 m/s), off the ground or in the water,
the beast gets up in a hurry, in two fifths of its own time (`emote::HURRY`,
09's quick sit against its relaxed one), and locomotion takes over through
the blend; asked for another gesture, it gets up and then plays it, the last
one asked for; asked for `emote::GET_UP`, a name in the beast's roster, it
gets up where it is. Caught on the way down, it comes back up the way it was
going. A state is not started under a body that is moving, in the air or
swimming, and a request while one lasts is never rate-limited. Held for
`gesture_secs` and then replaced by a second gesture, P5's and the first
cut's states stood the body up in one frame - a dog 644 mm, a horse 1535 -
and walked off while seated a dog dragged its pads 115 mm through the
ground: the walk's own foot settling ran after the sit, and no longer runs
under a beast in a state. A person's gestures are as they were.

**Not done here**: the scapula still does not move - a dog's or a cat's chest
rests on its sternum with its elbows on the ground without it. A dog's
relaxed down, rolled onto one hip, is a later variant (the owner's call). A
lying horse rests on its folded legs, its chest 1 to 10% of its withers
height off the ground (above). A rest ignores a ground that is not level.

**Cost**, native, per `Driver` frame (#377): what a rest solves from the rig
alone - the sit's end, the lying end, the pose each holds - is solved once
and kept while the rig is the same, and every reading of the skin takes all
it needs of one pose in one pass. A held state costs what an idle frame
does, 0.04 to 0.05 ms; going down or getting up 0.12 to 0.21 ms on average
and at worst 0.8; the one frame that first needs a lying end pays its solve,
about 2 ms. A clip applied on its own solves everything afresh, a little
dearer than the first cut's: 0.4 ms held in a sit, 1.7 to 2.2 lying down.

## Where the body departs, and why

### Cage walls (each commented "a cage wall, not anatomy" in the derive)

The joint-hull mesher opens one socket per bone on a girdle's hull, and the
anatomy is held back wherever two sockets would not fit.

- **The loin is at most 1.15 times the pelvis's width.** The spine's socket on
  the pelvis's hull carries the loin's width, and the thighs' sockets beside
  it have to clear it within their bones.
- **The forechest to the pelvis is at least 2.85 times the largest ring.** The
  ribs between the girdles need bones long enough to open their sockets on
  both hulls; a short, deep body stretches instead. It holds the cat's trunk
  near 1.14 WH whatever its frame asks.
- **A shoulder or hip joint stands out from its girdle at least 0.65 of its
  offset from it.** The cage aims each leg's socket at that joint, and two
  sockets open apart on the hull only if they leave it at least a third of
  the way sideways. It moves only rare bodies.
- **The front girdle stands a third of the way from the blades' tops toward
  the points of the shoulders**, so the hull it closes reaches past them as
  the pectorals do; under the blades' tops a horse's steep upper arm left the
  front of its chest behind its own shoulders. **And further on where that
  falls short**: the forechest's hull is held to reach 0.07 WH past the
  shoulder joints at their height (`PECTORAL`), which moves every horse's on
  by about 0.06 WH. Measured on the skin, the front stands at least 0.02 WH
  ahead of the joints on every roll (`tests/plan.rs`): 0.06 on the anchor
  horse, 0.10 on the dog, 0.09 on the cat. Before it a foal's chest ended
  up to 0.13 WH behind its own shoulders, so the leg's IK root and the
  shoulder's socket stood outside the skin.
- **The chest's underline at the heart girth and the forechest stays at least
  0.04 WH under the shoulder joints** (`BRISKET`), so the forechest's hull is
  there at their height for the rule above to close. A foal's long cannons
  raise its underline under shoulders that stay where the blades put them,
  and so do a slight horse's; under every dog's and cat's own rings, it
  deepens the anchor horse's forechest a little and a foal's most.
- **The composites grow a ring's width fully, the ribcage's depth by the
  square root of it, the loin's by its fourth root, and the girdles' depth not
  at all**, and the ribcage stays no wider than 1.05 of its depth after them as
  before. Depth grown as fully as width hung a heavy dog's chest under its
  elbows and crowded the legs' sockets off the girdles' hulls on 28 rolls in
  3,000; the loin's socket on the pelvis's hull and a heavy kitten's ribs
  crowded the envelope's high corner (`tests/budget.rs`).
- **The tail is set off the pelvis by 0.75 up and 1.15 back of its ring's
  radius**, so its socket clears the thighs' on the hull.
- **Legginess does not lengthen the blade, the humerus or the femur**: a blade
  lengthened with the leg hung its shoulder below its own elbow. A foal's
  long legs are its cannons.
- **A bridged leg's socket is sized by the joint it opens toward**, the hip or
  the shoulder, not by the node it sweeps to: the thigh's muscle rides the
  stifle's ring, and blended into the socket on the pelvis it crowded the
  spine's socket off its bone on 57 rolls in 400.
- **A bridged leg drawn out of round starts its ring frame across the body**,
  so a section says across and fore and aft whatever the leg's lean;
  transported from world up, the thigh came out deep on the diagonal. A round
  leg (every fore leg) keeps the frame it had: turned, its sockets crowded the
  spine's off the forechest at the envelope's low corner.

### Choices against a dossier figure

- **The dog's chest is about 1.9:1 deep to wide, not 1.3 to 1.5:1.** The
  dossier's own rows disagree: a chest 0.45 to 0.50 deep with a heart girth of
  1.07 to 1.15 forces nearly 2:1. The girth was kept.
- **The elbow joint stands at 0.44 to 0.46 under a 0.50 brisket**, reading the
  dossier's 0.50 to 0.52 as the point of the elbow, which stands above the
  joint.
- **The dog's shoulder angle reads 103 against 107 to 120**: the blade is
  measured in three dimensions here, and includes the joint set out from the
  ribs that its socket needs.
- **The cat is compact.** Its 1.3 to 1.5 body length (S25, L, its definition
  unconfirmed) read as a ferret; the paw 0.22 behind the hip that the
  radiographs' bones need at a 0.86 hip trailed the hind leg out behind the
  croup. The paw stands near under the hip, as on a dog's radiograph table
  (S6), the thigh bones trimmed a little toward joint centres; that folds the
  stifle to about 96 degrees against the background's 110 to 125 (L). Shorter
  thigh bones opened it, and folded the stifle of the envelope's low corner
  too close under the croup for its socket.
- **The horse's heart girth and depth to width are a little low** (1.08 and
  1.75:1): its forechest ring narrows ahead of the girth.
- **The horse's hip angle is not measured**: the rig has no pelvis bone, only
  the girdle.
- **The size is P5's.** A record's `height` builds a withers height 1.2 times
  it, so every record stands as tall in a world as the one the owner judged.

### Skin and zones on the new body

- **A leg's skin below the trunk is the leg's.** The trunk's bones are as wide
  as the body and reached the whole upper leg; the trunk lets go of a leg's
  skin where it leaves the trunk's own section, over a band (`rig::skin`).
- **The croup is not the tail.** The bone from the pelvis to the tail's root
  lies inside the croup; the tail's zone starts at its root, so the coat no
  longer paints a tail's rings on the rump and the brush no longer grows out
  of it.
- **The withers are the girdle the neck hangs from**, for every reader that
  measures along the spine (the coat, the wearable sockets, the instruments):
  the chest's first joint is now the middle of the ribcage.

### Where a leg leaves the trunk (R3, #376)

The visible junction is the elbow and the stifle, the first joint of each leg
outside the trunk. The zone map draws its line between trunk and leg where the
skin weights cross, which is nowhere an animal's coat changes; the renders
showed four things changing on it: the stripes' frame (bands round the barrel,
rings round the leg, meeting on a hard line), every mark gated on a leg's zone
(points, socks, a bay's black, a roan's and a dun's thinning, a bib, a
piebald's lean, a ruff's cards), the charts (the leg's zone ran up over the
shoulder and the flank, wrapped round a bone inside the trunk: its thinnest
tenth at a third to a half of the barrel's texel density, its texels stretched
two to three to one, the streaked crescent over the elbow), and the fur's
grain switching to the leg's bone on a line.

- **The junction is a share, not a zone** (`rig::junction`): a point's
  distance from a leg's bones over its distance from the spine, each in its
  own section (half-extents across and deep), runs from the barrel (all
  trunk) through the crease to the leg (all leg). The leg is the nearest one:
  by share, a paw far below the trunk was as much the other leg of its
  pair's. A section and not a radius, because a horse's thigh is half again
  as deep at the stifle as it is across: measured in its depth all round, the
  stifle reached in past the midline into the groin. And the spine runs on
  past the croup: measured from its last joint there, a tail was many of the
  spine's radii out and only a few of a hind leg's, and came out part leg,
  wearing a piebald's lean, a ticked coat's bars and the leg's grain. No
  tail and no back is any leg's now, over two dozen rolls and the shortest
  legs of a dog and a horse.
- **The charts follow it** (`uv`): skin the zone map gave a leg but more trunk
  than leg is charted with the trunk and wraps round the spine; the leg's
  chart is the skin below its junction, wrapped round the bone leaving the
  elbow or the stifle. Only below it: the fold over a horse's stifle is as
  much leg as trunk, but wrapped round the bone below it stretched a tenth of
  the hind leg's texels up to twelve to one, so it stays with the trunk. Over
  sixty rolls, fore and hind apart, the thinnest tenth of an upper leg's
  chart is 0.69 to 0.93 of the barrel's density on the forelegs and 0.67 to
  0.91 on the hind (before the slice 0.25 to 0.50 and 0.25 to 0.70), its
  median stretch 1.2 to 1.5 to one and 1.1 to 1.4, and its worst tenth
  stretched at most 3.0 and 3.6 to one (before, up to 22 and 32). Every roll
  is better on both legs than before. The barrel pays for the skin it takes
  over: its worst tenth stretched 3.13 to one on average over the sixty
  against 3.06, at most 4.3 against 5.5.
- **The coat blends through it** (`texture::coat`): every mark that differs
  between barrel and leg asks the share, so below the band a leg is painted
  as its zone used to paint it and nothing is cut on the zone line. A band
  turns into a ring down the top of the leg as a tiger's do, counted from the
  band that crosses the junction, the turn taking 1.5 of the junction's radii
  so the bands crossing a thick thigh close into rings no tighter than the
  leg's own. Spots, rosettes and dapples are cut 1.6 times finer down a leg,
  blended over the same turn. The grain blends the barrel's and the leg's.
- **Persons are untouched**: a person's limbs carry no marker inside the
  trunk, so it has no junction.
- **Not the coat's**: the fold the leg's tube makes against the barrel at the
  elbow still shades as a crease (geometry), and a chest's front still wears
  its bands broad, where the spine's frame lies square to the surface.

## Left for later slices

- **R5 (#378)**: the scapula driven parallel to the forearm, duty factors, the
  stance sweep, the carpus's fold, the spine's wave, the head's nod (08).
