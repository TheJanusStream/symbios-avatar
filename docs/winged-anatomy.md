# Winged anatomy: what the bird, the bat and the glider are built to, and where they depart

The winged plan (epic #418, its first visible phase #420) is built to measured
animals, as the quadruped is (docs/quadruped-anatomy.md). This file holds the
targets, family by family, with the dossier rows they come from; the owner's
decisions that shape them; what the body honours today, measured; and every
place it departs from the animal, each with its reason. The dossier is
`docs/research/research-10-winged-morphology.md`, cited here as **10** with
its source keys (S1 to S47); its companion on locomotion and flight,
research-11, is the motion phase's (#424). A figure's confidence is the
dossier's: H measured and read, M measured with a caveat, L a rule of thumb,
derived or recalled ("bg").

**T** is the trunk throughout: the shoulder joint to the hip joint, which no
posture changes. Every length the derive builds is a share of it. **HUM** is
the humerus, against which 10 gives a wing's bones.

Measure a body against this file with `cargo run --release --example
wingprobe` (the three families at their anchors and four crosses; `--appeal 1`
at a roll's default, `--sweep N` how many rolls mesh as asked, `--crosses`
every cross at blends a twentieth apart, `--dump` every node). Judge it with
`cargo run --release --example render -- --winged --gallery 9 --family bird`
(and `--winged --kin bat --wings` for one body's wings folded, half and spread,
`--wings-crosses` for every cross)
(or `bat`, `glider`, `cross`), `--pass silhouette` for the outline, and one
body with `--winged --kin bat` (`--kin bird+bat` a cross at half).

## The owner's decisions (2026-11-05, on #418)

1. **Avatar-sized, not real-sized**, with a size axis running about a cat to
   a dog: `size` is a head-and-body length, 0.35 to 1.2 m in its conservative
   range, the families centred at 0.60 (bird), 0.70 (bat) and 0.65 (glider),
   about the game's cat and dog.
2. **The bird is a small owl by default**, its axes reaching a crow, a parrot
   and a songbird; **its wing proportions are a crow's** (10 section 3.1).
3. **The bat is a fox-faced fruit bat in a flying fox's ratios** (10 section
   3.2).
4. **The glider is the sugar glider**, whose membrane will start at digit V as
   the bat's does (10 fact 7; the membrane is W2's).
5. **Crosses are allowed across two- and four-legged bodies**, the ground
   posture the dominant parent's: a cross under a blend of 0.5 stands as its
   own family, from 0.5 as its hybrid's (`Lineage::leading`).
6. **Cute by default**: `appeal` follows the young - an owlet or a fledgling,
   a pup, a joey - as a beast's follows a puppy or a foal.

At the look at W1's galleries (2026-11-05, on #420) the owner took every
recommendation: the bird's folded wing stays markers until W2's wing part
draws it; **a bird's bony tail is a short stub of markers under the rump**,
its visible tail the fan W2's tail part draws (the first build hung a thin
tube from the rump to the ground); the bat keeps its crawl, its wrists
ahead and out and its elbows over the back, to be judged again with its
membrane; and a cross's posture and marker pattern switch at the halfway
blend, as decision 5 has it.

## How the body is built

- **The beast's graph, node for node** (`plan::quadruped::hang`): six trunk
  rings from the pelvis forward, a neck, a head chain of four, a tail of five,
  two hind legs of five nodes and two fore limbs of six. The cage, the rig,
  the beast's head and feet and the gait that carry a beast carry this body.
  Nothing new crosses a worker: the anatomy's kinds are the beast's
  (`HeadKind::Beast`, `TrunkKind::Beast`, `FootKind::Beast`) and the legs are
  declared on `Anatomy::legs`, which W0 made.
- **The fore limb is a wing's arm**: the shoulder blade's top and the shoulder
  inside the trunk, then the humerus, the forearm and the wrist, and a
  placeholder hand of two nodes. **The digits are the wing part's** (W2, #421),
  as `extremity::hand` brings a person's fingers: the fan of feathers or the
  membrane needs a chain of its own per digit, which the graph's two
  extremity nodes cannot be, and a part grown off the wrist's joint can be
  rebuilt there without moving one node of the body.
- **A frame per family** (`plan::derive::winged::Frame`): every bone, ring,
  girth and the head's proportions in trunks. A cross blends two frames by its
  blend, as a quadruped's does.
- **The posture is the leading family's** (`plan::winged::Posture`), and with
  it three things that cannot blend: how the trunk is pitched and stood, which
  of a limb's nodes lie inside the body as markers, and which limbs the plan
  declares as legs.
  - **Perch** (a bird): the trunk raised 0.95 rad, stood on the hind pair;
    `Legs::HIND`. Every node of the wing arm is a marker inside the feathered
    outline the rings draw, and the femur and the knee are markers too: the
    visible leg is a bridge from the pelvis to the intertarsal joint, as a
    bird's visible leg is the tibiotarsus's lower half and the
    tarsometatarsus (10 section 3.1, "femur posture", and note 2).
  - **Crawl** (a bat): the trunk nearly level, the belly 0.10 T off the
    ground, each wrist planted ahead and out with the elbow folded up over the
    back, the hind feet out and back with the knees up and out (S22: the knee
    faces dorsolaterally); `Legs::ALL`. The bat's fore "foot" is at its
    wrist: the hand's first node is the wrist's pad on the ground, its second
    the thumb ahead of it.
  - **Stand** (a glider): level, crouched, an ordinary small mammal's legs;
    `Legs::ALL`.
- **The stance is set from the ground up**: a bird's hip stands where its
  leg's angles put it and its feet go under its trunk's mass; a bat's trunk is
  lowered to its belly's clearance and each limb solved to the ground; a
  glider's hip stands at a crouch's share of its leg.
- **A body its cage cannot hull is eased toward its leading family's frame**
  until it can (`WingedParams::skeleton`), as a beast's is (#391): the cross's
  blend toward its leading end (so the posture never changes), each shape axis
  toward nought, and the girth toward neutral, each by as little as the search
  finds. Every family's own frame meshes; of 2,000 rolls, 6 are eased, each a
  heavy body past `mass` 1.5 or a cross at the middle of its blend. A share
  code and a record keep what was asked.

## Targets per family, and what the body measures

`wingprobe`, each family at its anchor and `appeal` 0; bones joint to joint.

### Bird (10 section 3.1; reference a carrion crow's wing, a small owl's body)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Ulna (forearm) / HUM | 1.22 | S1 (M) | 1.21 |
| Carpometacarpus / HUM | 0.74 | S1, S2 (M/H) | 0.74 |
| Digits / HUM | 0.30 to 0.37 (0.4 to 0.5 of the CMC) | bg (L) | 0.34 |
| Humerus / T | about 0.77 (T about 1.3 HUM) | derived (L) | 0.50, see departures (#421) |
| Femur / HUM | 0.6 to 0.7 | bg (L) | 0.60, see departures |
| Tibiotarsus / HUM | 1.1 to 1.2 | bg (L) | 0.81, see departures |
| Tarsometatarsus / HUM | 0.72 | S2 (H) | 0.39, see departures |
| Folded wing | humerus back along the flank, forearm forward, hand back (a Z); elbow 31, wrist 52 deg interior | S5, S45 (H, M) | the Z, all markers, elbow 31 and wrist 45 deg, the forearm and hand in a plane just outside the widest ring (#421) |
| Femur posture | near level inside the body wall, the knee hidden | bg (L) | forward and 0.25 rad down, a marker |
| Neck | an owl's: no visible neck | S10, bg | the head on the breast |
| Tail | the pygostyle, a stub; what shows is the tail feathers | bg (L) | a stub of 0.12 T, all markers, near the rump: the fan (W2) is the tail |

### Bat (10 section 3.2; a large flying fox)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Forearm / HUM | about 1.5 | bg (L) | 1.50 |
| Humerus / T | about 1 (HUM 0.6 to 0.7 FA; FA 179 to 220 mm on HB 225 to 320) | S20, S21 (M), derived | 0.92 of the trunk as stretched (1.0 of the frame's) |
| Hind foot / T | 52 to 68 mm on a trunk of about 130: about 0.4 | S21 (M), derived | 0.40 |
| Tail | none in *Pteropus* | S21, S22 (M) | a nub of 0.04 T, tapering |
| Hind limb | rotated, the knee dorsolateral | S22 (H) | the knee up, out and back |
| Crawl | on thumbs and feet, wings folded | S29 (M) | wrists planted ahead and out, elbows over the back |

### Glider (10 section 3.3; the sugar glider)

| Quantity | Target | Source | Built |
|---|---|---|---|
| Tail / HB | 1.0 to 1.1 | S30 (M), derived | 0.94 (a tail of 2.3 T, laid back over the ground) |
| Limbs | long and gracile against arboreal kin | S36 (H, abstract) | humerus 0.32 T, femur 0.38 T: chosen by render |
| At rest | reads as a small quadruped, the patagium folded at the flank | S31, fact 9 | as the target; the fold is W2's |
| Face | big, wide-set eyes, short snout, big ears | S31 (M) | the beast head's axes, see "The head" |

## The young: what `appeal` draws (`plan::derive::winged::Schema`)

`appeal` runs 0 at naturalistic to 1.5 all the way to the young (`young` =
`appeal / 1.5`, held to -0.5 below zero), as a beast's does, and a roll
centres it at 1. 10 fact 10: the young are the baby schema - a large head, a
short face, a round outline - and a bat pup is big-headed and short-winged
(S26); "a cute default should push head and eye size and shorten the face
and bill, not shrink the wings".

| Family | Head | Rings toward round | Neck | Tail | Arm | Legs' girth |
|---|---|---|---|---|---|---|
| Bird (owlet) | x1.40 | x1.12 | -30% | -35% | adult | +20% |
| Bat (pup) | x1.45 | x1.10 | -25% | adult | -15% | +15% |
| Glider (joey) | x1.50 | x1.12 | -25% | -30% | adult | +15% |

Every share is **chosen by render**: 10 has no figure for a fledgling's or a
joey's trunk, and the head's ratios follow the quadruped's puppy (x1.56) a
little short of it, since an owl's head is already large.

## The axes, and why these

Seven, as the quadruped's body has six beside its head's: research-10 names
the differences that change the rig, and these are the ones its rows support.

- **`size`** - the owner's: a cat's to a dog's at a game's scale.
- **`wingLength`** - span against body is the largest difference inside each
  corner (span over length about 2.1 on a crow, 5 on a flying fox, 1.6 to 2
  on a glider; 10 sections 3.1 to 3.3). It scales the humerus, the forearm and
  the placeholder hand now, and the wing part's own bones in W2. **Aspect** -
  long and narrow against broad and rounded - is the hand's and the feathers'
  shape, which W1 does not build, so it is left for W2 to add if the owner
  wants it as its own axis.
- **`neckLength`** - 10 section 3.1: neck counts run from about 10 to 26, an
  owl's hidden in an S, a crow's showing. The bird's axis reaches the crow.
- **`legLength`** - 10's songbird "small enough that the feet show more than
  the legs" against a crow's long tarsus; the owl's legs are feathered short.
- **`tailLength`** - the widest single difference between the corners: a
  flying fox has none (S21), an owl a short one, a parrot "a long, graduated
  tail" and a sugar glider one as long as its body (S30).
- **`headSize`** - an owl's large round head against a crow's, a parrot's
  "big head against the body" (10 section 3.1).
- **`appeal`** - the owner's: the young.

Not axes, and why: the trunk's length and depth, which 10 gives as
family-level (a bird's "compact rigid trunk", a glider's elongated one) and
`mass` and `bodyFat` already move; **plumage volume** (10 note 8), which is
W4's; the head's own shape (a beak, a disc, ears), which is W3's.

## The head

Until W3 builds a beak, an owl's disc and a bat's fox face (#422), each family
wears **the beast head** (`plan::beast_head`) with the axes a family stands
in for: a bird a flat face (`muzzleLength` -1.5, a high brow, eyes to the
front, small pricked ears that read as tufts, no whiskers, a soft nose); a
bat a fox's muzzle (`muzzleLength` +0.35, narrow, a pricked ear); a glider a
short snout under big ears and wide-set eyes. They are the leading family's
blend and are not stored: a record carries no head axis of the plan's until
W3 says which it needs. A perching bird's head sits on its breast, so its
neck's upper run is as wide as the skull it meets (`Nape::throat` set to its
seat): a beast's throat stood an owl's head on a collar.

## The wire

- **The record**: `network.symbios.avatar.defs#winged`, the lexicon's own
  definition and the archetype union's third ref; `size` in millimetres and
  six signed axes in thousandths, `family`, `hybrid` and `blend` as a beast's.
  Every field is `serde(default)`.
- **A build from before the plan reads the record as an unknown body**, the
  honest stand-in: read with the published 0.19.7 itself, each family's record
  and a cross's read as `Archetype::Unknown` under the winged `$type`, build the
  stand-in person, and write the archetype back unchanged; and a built winged
  body, sent as the worker sends one (named msgpack), decodes whole in 0.19.7 -
  its anatomy, legs and parts are kinds 0.19.7 has. `tests` hold the same over
  a name this build does not know
  (`a_winged_record_is_an_older_build_s_honest_stand_in`).
- **The share code**: tag 3, written at version 10, the oldest anything still
  writes; nothing it carries is new to the layout but the tag. A 0.19.7 reader
  refuses it as an unknown tag; so does this build at any version before 10,
  which no winged code was written at (the five legacy readers are refusals).
- **`GENERATOR_VERSION` does not move**: a winged roll draws on streams of its
  own (`winged.*`), so no seed of a person or a beast moved.
- **Nothing was added to `mesh_serde::Wire` or `rig::outline`** (the epic's
  rule).

## Where the body departs, and why

### Cage walls (each commented "a cage wall, not anatomy" in the derive)

- **The trunk stretches along its line** until the pelvis's bone to the loin
  is 1.1 of its cage radius and, on four legs, the forechest's to the ribs 0.75
  of its own and the run between the girdles the beast's 2.85 of the largest
  ring (`SPAN_FLOOR`). A round body on a short trunk grows a little longer
  instead of refusing.
- **A bridged leg's first joint stands 1.7 cage radii from its girdle**: a
  bird stands as much taller, a mammal's knee or elbow is carried on along
  its bone. Short legs under a heavy rump stand a little taller.
- **The neck's bone off a four-legged body's forechest is 1.2 of its radius**,
  so its socket clears the fore limbs'.
- **A joint inside the body a leg is aimed at stands out from its girdle 0.65
  of its offset**, the beast's rule; a bird's knee stands out against the body
  wall by 0.45 of its femur besides, so its socket leaves the loin's.
- **The tail is set off the pelvis by 1.4 back and 0.9 up of its radius**, so
  its socket clears the legs'. A bird's, all markers and so no socket, is set
  0.8 back and 0.3 up: a stub under the rump where its fan roots.
- **A four-legged body's rings stand where a beast's do** (0.06 to 0.95 of its
  trunk, the forechest behind the shoulders), whichever families it blends:
  hung from a ring ahead of them, a cross's arm sockets opened back into the
  ribs'.

### Choices against a dossier figure

- **The owl's legs are shorter than a crow's** (femur, tibiotarsus and
  tarsometatarsus 0.42, 0.62 and 0.30 T against the crow's 0.50, 0.89 and
  0.55), and crouched: at a crow's lengths the default owl stood its body
  half its height off the ground. `legLength` reaches the crow's.
- **A bat's arm is one tube, the humerus and the forearm folded**: the
  digits folded along the forearm and the membrane bunched over them, which
  are what a resting flying fox's arm reads as (S46), are W2's.
- **The bird's humerus is half a trunk, not 0.77** (#421): a crow's 68 mm
  humerus (S1) on a trunk of about 145 mm (bg) is 0.47 of it, and at W1's
  0.77 an owl's bones alone spread wider than a crow's feathers reach. The
  forearm, hand and digit keep their ratios to it.
- **The bird's body is its feathered outline, not its skeleton**: 10 fact 1,
  a bird's wing is mostly feather, and so is its trunk's outline. The wing
  arm is inside it, and shows nothing until W2's wing part.

### Junctions and parts

- **A leg's junction wraps under the trunk only if the leg hangs from its
  girdle** (`cage::leg_root`): a crawling bat's arm rises from its chest to an
  elbow over its back, and wrapped, its junction drew a web from the keel up
  the arm. Every beast's elbow and stifle stand below their girdles, so no
  beast moved.
- **A limb that meshes nothing has no junction** (`rig::junction`) and **ends
  in no paw** (`extremity::beast`, which shapes only the declared legs): a
  bird's wing arm is markers inside its flank, and a paw pressed onto it there
  would dent its side.
- **A wing does not swing as an arm** (`anim::gait::hang_arms`,
  `swing_arms`): a limb that bears no weight is an arm only on a body with
  hands. The bird's wing is still until W5 moves it.
- **A coat stands in for a surface**: until W4 (#423) stores a plumage, a pelt
  or a membrane, each family is painted with a coat of its leading family's -
  a brown bird, a dark bat, a grey glider - not stored.

## The wings (W2, #421)

The wing is an attached part on `extremity::hand`'s pattern: grown on the rig
after the body is bound (`wing::grow`), its own joints `Role::Digit` in the
fore limb's extremity zone (the tail fan's in the tail's) and listed on the
rig (`Wings::joints`), so the body's binding skips them, `limb_chain` never
reaches them, and `Rig::extremity_joints` - the foot every gait, ground
contact, socket and `LimbTip` landmark reads - leaves them out. Before it
did (the end review), a bat's digits folded back along its forearm read to
the gait as a heel half a metre long, and its hand's tip was its last digit.
The
**rest is the fold**, as the plan stands the body; `Rig::spread_wings(pose,
left, right)` writes every wing joint's rotation for an extension from `0`
folded to `1` spread, and `Rig::spread_tail(pose, t)` the fan, from data on
the rig (`Rig::wings`, `serde(default)`, absent on every other body) that
crosses a worker boundary. **Each joint turns along its own arc in the
body's frame**, eased over its share of the range, and its rotation in the
pose is that turn reckoned against the nearest driven joint above it (its
anchor): a bird's elbow and wrist on one schedule, the linkage (S6); its
feathers from 0.1; everything a membrane blends - arm, digits, spars, hind
leg - on one schedule, the whole range; each turn the shortest arc. And
**the extension is held short of tearing a blend** (`wing::fit`): at build,
every distinct binding a drawn vertex carries - the body's own, the
membrane's, the feathers' - is accumulated across the extension as
dual-quaternion skinning accumulates it, and wherever a part would come
within 174 degrees of a half turn from the running sum (where skinning
flips it), the turns on the two branches that part below the pair, and the
joint they part at, are scaled back a tenth at a time, never below a fifth.
It never touches a bird or a glider as rolled; on a bat it trims three hand
bones a side to 90% (the knuckle blended across digits IV and V reaches 160
degrees); on the far corners - the longest wings on the shortest legs of a
glider-led cross - a patagium opens partway. Composed joint by joint in
their parents' frames, chains of large turns about different axes brought
blended joints a half turn apart partway through, and a glider-led cross's
membrane jumped 0.76 m in one step (the critic). Signing turns to agree
with a mean, or putting one on its long arc, mended some pairs and broke
others (two spars' near-half turns leaving the fold on opposite arcs; a
membrane swung through the trunk on the way).

**The surface is the leading family's**, as the posture is: feathers on a
bird-led body, a bat's membrane on a bat-led one, a glider's patagium on a
glider-led one. Proportions are the blended body's, read off the arm the
part hangs from. Simple and cheap; a bird-bat cross switches at the halfway
blend with its posture, so a cross never wears a surface its stance does not
match.

| Target | Source | Built |
|---|---|---|
| Bat digits II-V off the wrist, thumb clawed | 10 3.2, S25 | II-V of 3 bones each, digit III 0.67 + 0.50 + 0.58 FA (S20, bg), V 1.28 FA; thumb 0.26 FA, free, a claw point |
| Bat spread: III leads the tip, V about 90 deg behind it, elbow 120-150 | 10 4.2 (bg) | III 28 deg, V 100 deg behind the forearm; elbow 134 deg |
| Bat fold: digits back along the forearm, phalanges folded back (S46) | 10 4.2 | metacarpals 171-176 deg back along the forearm, within 5 deg of one another; P1 folded back 149, P2 forward 143, each layer leaning out a little |
| Dactylopatagium II-V, plagiopatagium V to the ankle, propatagium shoulder to thumb | 10 5.2, S22, S25 | all three, closed thin shells, the trailing edges scalloped (one sweep V-to-ankle, one per pair of fingertips) |
| Membrane 130-300 um | S23, S24 | 0.4% of the trunk either side (about 1.2 mm), see departures |
| Glider patagium digit V to the ankle | S30, S31 | the forefoot (palm) to the ankle, one shell, on three spars |
| Glider at rest reads as a small mammal, a fold at the flank | 10 fact 9, 4.3 | a narrow fold along the flank between the legs |
| Bird: 10 primaries, 10 secondaries, 3 tertials, coverts, alula | 10 5.1 | 10 (6 on the carpometacarpus, 4 on the digit), 10, 3, three humerals; greater, median, lesser and marginal coverts over the secondaries, greater and lesser primary coverts, a row over the tertials, two to a gap (114 a wing); a 3-card alula |
| Overlap: each remex over the next toward the tip, tertials on top | S45, 5.1 | each card a lift (0.2% T) above the one it overlies, folded and spread |
| Folded: stacked along the flank; spread: the fan opens | 10 4.1, 5.1 | primaries along the hand when folded, fanned from 22 to 93 deg behind straight out when spread |
| 12 rectrices on the pygostyle, fanned by its muscles | 10 5.1 | 12, rooted just inside the rump, closed +-11 deg and roofed (outer ones turned 37 deg down about their shafts), fanned to +-49 deg on two edge joints, graded |
| Folded wing on the flank and back, primaries crossing over the rump toward the tail | S45, 10 4.1 | the fold's plane leans 32 deg in over the back at the trunk's support distance; primaries turned 45 deg from straight back toward the back, secondaries 17 |

**How a membrane is built.** Each piece is a ruled grid between its edges -
spars, the arm, the leg - where they **rest**, each vertex held by the two
edges beside it, two influences each, blended across the strip: four in all,
never truncated, so the binding is continuous. The fold is therefore what
was built; the spread is the binding's. A plagiopatagium or a patagium needs
spars of its own (research-11 7: "an extra spar per digit") so no strip
between neighbours turns by much: a glider's three are two-bone chains from
the flank to the free edge; **a bat's three are its arm and its leg
blended**, each bone resting where the arm's and the leg's rest blended and
turning by their two turns slerped, since a flank spar of its own two bones
had to fold sharply to fit a crawl and stood out of the fold as a spike.
(Blended on a glider, every bone of them turned near a half turn and the
guard trimmed the patagium to a strip.)
Every edge is sampled bone by bone, so a strip never blends one digit's
phalanx with its neighbour's metacarpal, whose turns lie more than a half
turn apart and made dual-quaternion skinning jump. The two sheets stand
either side of the spread sheet's normal carried back to the fold, and each
vertex keeps that normal, so a spread membrane shades taut whatever its fold
did. A free point of the fold that would rest inside the body is carried out
across it (asked on the left and mirrored, since the inside test's ray is not
symmetric), and every quad is split on the diagonal that keeps both halves
facing the sheet's way spread.

**How a feather is built.** A card of six rows lofted along its rachis,
narrow at the quill, broadest past the middle, rounded at the tip, cambered
toward the body; cut from the strand mask's solid run (its first three
fifths, the middle of a lane), so it draws whole. **Folded, each card lies
on the body straight**: turned about its quill until its tip stands a few
millimetres off the surface, backed off until no row lies inside, and its
plane turned halfway to the body's - every flight feather rides a joint of
its own, which carries any fold onto its spread place, so a card can lie on
an egg and still spread flat (bent to lie there, it spread as a banana; kept
in one plane, the fold stood off the flank as a plate). Laid on the left and
mirrored. A covert is held between the two feathers it lies over, graded by
its place between them. **Sized by the bone it grows on**: primaries by the
hand, secondaries by the forearm, humerals by the humerus, each against a
crow's ratios, so a bat-blended cross's longer forearm grows secondaries to
fill it.

**Measured** (`tests/wings.rs`): membranes are closed manifolds; every wing
vertex is held by at most four joints summing to one; of a wing's free
vertices (past the first fifth of each piece, off the columns that ride
inside a limb), folded, at 0.3, 0.6 and spread, on every family and cross,
every axis at each end and twelve rolls, at most 1.03% stand inside the
trunk's own skin (four vertices a wing, a glider-led cross's patagium corner
under a bird-blended belly; a wing mutated to fold into the trunk reads 61%);
over 200 steps of the extension no step moves any drawn vertex more than six
times the mean step, on every family and every cross at blends 0.3, 0.5 and
0.6 either way round, each as rolled, with the longest wings on the shortest
legs, with the longest wings, and with the heaviest composites; the wings are mirror images to a millimetre, and spread
at least 1.6 times as wide as folded. Triangles: bird 17,866 (feathers 1,470
in the hair's draw), bat 24,490 (membrane and digits 5,392), glider 22,060
(patagium 2,576); the dearest of the families, six crosses and twelve rolls
25,142. After the owner's look (v2): bird 19,426 (feathers 3,030 - 303
cards); bat, glider and crosses unchanged. Joints: bird 106, bat 130,
glider 68.

### Where the wings depart, and why

- **The membrane is thicker than a bat's**: about 1.2 mm a side on an avatar
  where research-10 measures 130-300 um, which no depth buffer at a game's
  distance would separate from its other side.
- **A membrane's spread is bound, not built**: the spars are close enough
  that what blending two spars' motions leaves out of a taut sheet is a
  slight billow near the body. Building the spread and resting it by the
  binding's inverse gave an exact spread and a fold that bulged in a dome
  over the back.
- **The bat's wrist rides its forearm when spread**: its own bone is the pad
  the crawl stands on, below the arm's plane; turned onto the plane, it
  carried the digits' turns out of it and some past a half turn.
- **A membrane body's extension carries its hind legs out**, behind the arm:
  the trailing edge is anchored at the ankle (S22). W5 may want the legs on
  their own value.
- **A bat's hand folds a Z at 149 and 143 degrees, not flat**: flatter, the
  turns of neighbouring bones - at a knuckle blended across digits IV and V,
  or between V and the spar beside it - came more than a half turn apart
  partway through the extension, and dual-quaternion skinning made the
  membrane jump. A few short shards of the fold still show.
- **A bat's cape is drawn 60% of the way in toward its flank** at rest; the
  spread membrane near the body carries a slight ripple for it.
- **The bird's tail is rooted in its rump, not on W1's stub**, which ends
  about five centimetres behind the body.
- **A bird's humerals rise over its back when spread** (0.6 of the way out
  of the wing's plane, the tertials half that): the plane through the
  shoulder runs under the back, and in it they passed through the body.
- **Plumage and membrane colour are stand-ins** (W4, #423): the feathers a
  brown owl's in vertex colour, the membrane the coat's in its own atlas
  region.

## Left for later phases, and for the owner

- **W2 (#421)**: built - see "The wings". For the owner: the bat's cape
  from its raised elbow to the leg at rest, the bird's folded plate and how
  far its primaries reach, the feather count, the glider's resting fold.
- **W3 (#422)**: the beak, the owl's disc and eyes, the bat's ears and fox
  face, the glider's face; toes, talons and claws.
- **W4 (#423)**: plumage, contour feathers, the membrane's material, coats.
- **W5 (#424)**: the ground gaits per posture, the flight source, rests. A
  winged rig is driven today without a panic (`tests/driver.rs`): a bat and a
  glider walk as beasts, a bird as a biped beast.
