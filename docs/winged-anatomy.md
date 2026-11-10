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

At the look at W3's heads and feet (2026-11-06, on #422) the owner took every
recommendation: the owl's disc rounded at its edge and its ruff a little
lighter, so the face stops reading as a hood in profile; a rolled bird drawing
a kind of head first, so about one bird in four has a crow's or a songbird's
straight beak; a straighter-beaked head's cheeks drawn in further; and as they
are, the glider's large eyes, the fruit bat's fox face, the bat's foot as W1
set it and the glider's hallux. The eyes' colours wait for W4.

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

Seven for the body, as the quadruped's has six beside its head's, and three
for a bird's head (W3, #422): research-10 names the differences that change
the rig, and these are the ones its rows support.

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
- **`beakLength`** (#422) - an owl's short beak, mostly hidden in its disc,
  at nought; a crow's, "about as long as the rest of its head" (10 6.1; a
  raven's culmen 0.07 of its total length, S2), at one; shorter below. The
  beak's visible run is 0.45 cranium radii at nought, 1.3 at one.
- **`beakHook`** (#422) - how far the upper mandible's line turns down from
  base to tip: 1.2 rad (an owl's hook) at nought, most of a right angle and a
  half (a parrot's "deep hooked bill", 10 3.1) at one, a crow's slight curve
  at minus one and a songbird's straight bill below. **The beak's depth
  follows both**: its base is 0.55 of its length deep at nought, 0.22 more
  per unit of hook, never under 0.36 - a parrot's deep bill, a songbird's
  thin one.
- **`facialDisc`** (#422) - an owl's facial disc (10 3.1, 6.1: "a ring of
  stiff feathers", "eyes forward and close together") full at nought, none
  from minus one. The eyes follow it: on the front of the face with a full
  disc, round the sides of the head (a crow's, a songbird's) with none.

The bird's reach by the three: **owl** `0, 0, 0`; **crow** `1, -1, -1`;
**parrot** `0.3, 1, -1`; **songbird** `0.2, -1.3, -1`. **A roll draws a kind
of bird first, then each axis about it** (the owner's call at W3's review): an
owl six rolls in ten, a parrot three in twenty, a crow 0.13 and a songbird 0.12
(`winged.beakKind`), so a straight beak comes up about one bird in four; then
the beak's length and hook about that kind's with a spread of 0.25, and a disc
only for an owl, about its own (`winged.beakLength`, `winged.beakHook`,
`winged.facialDisc`). Drawn alone about the owl with a spread of 0.45, a
straight beak came up one roll in eight and landed in an owl's disc. Only a
bird-led body draws them: a bat's or a glider's anchor centres them at an
owl's and spreads them as a bird's, so a change of family carries a bird's
beak across and back, and a bat's or a glider's roll keeps what it carries
(#429), since the axes shape nothing it has. `appeal` shortens the beak, by 0.3 of it at the young's end (10 fact 10:
"shorten the face and bill").

Not axes, and why: the trunk's length and depth, which 10 gives as
family-level (a bird's "compact rigid trunk", a glider's elongated one) and
`mass` and `bodyFat` already move; **plumage volume** (10 note 8), which is
W4's; the bat's and the glider's faces, which research-10 describes one way
each (a fruit bat's fox face, a sugar glider's) and which are their families'
frames, blended across a cross.

## The head

The head is **the beast head** (`plan::beast_head`, #367): a braincase, a
muzzle, the jaw, the cheeks and the seats of the eyes and ears, declared on
the skeleton and the head's surface projected onto it. Each family brings the
beast axes it stands in for (`derive::winged`'s `Face`, blended across a
cross), and `derive::winged_head` then dresses what the construction made, in
an order that matters: the disc is carved first, because the eyes and the beak
are seated on it.

| Family | Mouth | Eyes | Ears | Nose, whiskers |
|---|---|---|---|---|
| Bird (owl) | a beak of two mandibles | 1.45 a beast's globe, frontal and close-set with a disc, lateral without | none | no nose cut, none |
| Bat (fruit bat) | a long, narrow fox muzzle (`muzzleLength` 0.55, `muzzleBreadth` -0.7) | 1.7 a beast's | large, plain, pricked, a round-pointed leaf (1.25 a beast's at `earSize` 1.4) | leather, none |
| Glider | a short snout (`muzzleLength` -1.0) | 1.9 a beast's, set wide, 0.62 of the globe standing proud | large, rounded (`earSize` 1.5, `earPoint` -0.8) | leather, whiskers |

- **The beak** (`BeakSeat`, swept by `face::beast::beak`) is two closed solids
  of horn along the line of the gape, seated on the face between and under
  the eyes, sunk into it by 0.45 of its depth. The upper mandible turns down
  along its line by its hook (the turn grows as the square of the way along,
  so the hook gathers at the tip), its section an arch narrower at the culmen
  than at the cutting edges; the lower is straighter, shorter as the upper
  hooks (0.95 of its line less 0.18 per radian), its top tucked inside the
  upper. **The upper rides the head and the lower the jaw**: both are drawn in
  the eyes' draw (`Adornments::on_head`, `on_jaw`), which carries a part's
  own colour on any plumage and crosses a worker already, and the lower is
  bound to the jaw's joint, so `Jaw::pose` - the beast's pant and yawn -
  parts the beak. A beaked head builds no jaw of skin and no tongue (the jaw
  is rigged all the same), cuts no nostril, grows no whiskers, and its
  muzzle's lip lobes sink into the braincase and its cheeks draw in: as built
  they drew a mammal's mouth line under the beak. The horn is an owl's pale
  horn darkening toward a crow's black as the beak lengthens, a stand-in until
  W4 stores a plumage's colours.
- **The disc** (`Disc`, part of `BeastHead::distance`): a dish carved out of
  the face as the inside of a ball standing ahead of it, its rim 0.8 cranium
  radii across from a middle 0.12 below the braincase's centre and its middle
  0.08 under the rim at full strength, and a ruff - a tube 0.06 radii round
  the line where the ball crossed the head - smoothly joined on, so the ruff
  follows the edge wherever the face's own parts put it. The ruff is darker
  than the dish it rings, which is what outlines a disc at a distance: a tint
  by vertex until W4 painted both in the plumage's colours ("Surfaces"). The eyes are
  seated in the dish and their gaze turned 0.7 of the way from its normals,
  which lean in toward its middle, to straight ahead. **An owl's head is
  charted round its crown's line** (`uv::BeastCharts`): wrapped round the
  line from the back of the skull to the face, as a beast's is, the dish came
  out all one height across, in streaks. The surface's projection axis is
  drawn back inside the head, which the dish carves its front end out of.
- **The ears** are the beast's leather cards (`face::beast::ears`), sized by
  the family's share: none on a bird, 0.3 of a bat's on a bird+bat cross at
  0.3, and an ear smaller than 0.15 cranium radii none at all. A head with
  none builds each ear empty, which is no part and rides no joint.
- **A cross**: the kind of mouth - a beak, or a muzzle with its nose and its
  jaw - is the leading family's, switching at the halfway blend as the
  posture does. The eyes' size, set and standing-out, the ears' size and the
  snout's length blend across the cross and move continuously through the
  switch; **the disc's turn of the eyes is the bird's share of them**, so it
  reaches a bat- or glider-led cross by its bird share, as the bird's own eyes
  do. The disc itself is a part of a bird's face, carved only into a bird-led
  head, as deep as the bird's share of it.
- **The young** (`appeal`): the beast's larger globe (`eyeSet`'s seat adds a
  twentieth of a radius per unit of appeal) and, on a bird, a beak 0.3
  shorter at the young's end.
- **Measured** (`tests/winged_heads.rs`): each family's head has its parts
  and no others; the beak reaches further with `beakLength` and its tip curls
  further under the lower mandible with `beakHook`; the disc comes with its
  axis, flattens the face and sets the eyes closer; the lower mandible, and
  nothing else of the eyes' draw, opens with `Jaw::pose`; a 0.20.0 record and
  its pinned code read as an owl's head; every family and cross builds finite,
  its mandibles closed, at the axes' ends. Triangles (`render --budget`): owl
  15,626 (the beak 520 of the eyes' draw; no ears, no jaw or tongue of skin),
  crow and parrot 17,300, bat 24,428, glider 22,400; the dearest of
  `a_winged_body_fits_its_budget`'s families, crosses and rolls 25,130.
- A perching bird's head sits on its breast, so its neck's upper run is as
  wide as the skull it meets (`Nape::throat` set to its seat): a beast's
  throat stood an owl's head on a collar.

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
- **The head's axes** (#422): `beakLength`, `beakHook` and `facialDisc` on the
  record, thousandths in the signed envelope, optional and defaulted, so a
  0.20.0 record reads as an owl's head; and in a share code **version 13**,
  three spans appended after the complexion, written only by a winged body
  whose head is not an owl's - asked of the three bytes it would write (#429),
  so a head whose bytes are the owl's (an axis from 0 to 0.023) writes 10 and
  reads back as the owl's, and a head read from a code this build writes
  writes that code again (a 13 an older build wrote with the owl's three bytes
  reads 0.012 an axis and writes 10). Every other code - a person's, a beast's,
  an owl-headed winged body's - is written at the version it was, byte for byte,
  so a 0.20 reader takes it; a 0.20 reader refuses a 13 as a version from the
  future. A 0.20 winged code (version 10) reads as an owl's head.
- **The head's new parts ride what already crosses a worker**: the beak in
  the eyes' draw's `Adornments::on_head` and `on_jaw`, the ruff in the skin's
  vertex tint; the `BeastHead`'s `beak` and `disc` are skipped when absent, so
  a beast's skeleton serialises byte for byte as it did.
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
  hands. Nor does it breathe or roll in a fidget (`anim::idle`, #424): the
  idle's shoulders are an arm's too. The bird's wing is still on the ground.
- **A coat stood in for a surface** until W4 (#423) stored a plumage and a
  coat: see "Surfaces".

### Where the head departs, and why (W3, #422)

- **An owl's disc is a carved dish, not feathers**: research-10 describes "a
  ring of stiff feathers"; built as relief on the head's surface with a
  shaded ruff, it reads at a distance and costs no cards. Its paler face and
  darker ruff are its plumage's since W4 ("Surfaces").
- **No ear tufts**: the default owl is a little owl (10 7.2), which has none;
  a scops or an eagle owl's tufts would be a part of their own.
- **A bird's eyes turn as a beast's do**: research-10 fact 5 has an owl's
  eyes move 1 to 4 degrees and the head do the looking. The eyes' range is
  the eye rig's, shared with every body, and a range of the plan's own would
  be a new field crossing a worker; the head saccades that stand in for it are
  the motion phase's (W5, #424).
- **The lower mandible hinges at the beast jaw's hinge**, under the ear and
  behind the eye - where a bird's quadrate is, so an open beak drops as a
  bird's does, about a point far behind it.
- **The beak is horn-coloured by its length, not by species**: an owl's pale
  horn toward a crow's black. A parrot's two-tone bill, a songbird's yellow or
  orange, a cere, are W4's colours.
- **The disc's turn of the eyes reaches a bat- or glider-led cross** by its
  bird share, though the head's axes otherwise shape only a bird-led head:
  held out of a cross's blend, the eyes would step where the mouth's kind
  switches, and research-10 asks that the young and the crosses push eye size
  continuously.
- **The fruit bat's ears are a beast's cards** - plain leather, no tragus, as
  a fruit bat's are (10 6.1) - so a microbat's tragus and noseleaf are not
  built; nor is a barn owl's asymmetric ear opening.
- **A sugar glider's eyes stand out by 0.62 of the globe**, against a beast's
  0.40: research-10's "large, protruding" eyes (S31). Their dark iris is the
  plan's, the horse's ramp.

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
joint they part at, are scaled back a tenth at a time, never below a fifth
(held at it, #427: a tenth less from above it reached 0.185), and a pair whose
branches are all at the floor is passed over for good, so it costs the loop
none of its sixty trims. The test is reckoned in a fixed order (`det::dot4`),
so a browser trims what a native build does; and an extension that is not a
number poses the fold, where it posed every driven joint NaN.
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
| Bat fold: digits back along the forearm, phalanges folded back (S46) | 10 4.2 | metacarpals 171-176 deg back along the forearm, within 5 deg of one another; P1 folded back 152, P2 forward 169 (149 and 143 before W5's knuckle helpers), each layer leaning out a little |
| Dactylopatagium II-V, plagiopatagium V to the ankle, propatagium shoulder to thumb | 10 5.2, S22, S25 | all three, closed thin shells, the trailing edges scalloped (one sweep V-to-ankle, one per pair of fingertips) |
| Membrane 130-300 um | S23, S24 | 0.4% of the trunk either side (about 1.2 mm), see departures |
| Glider patagium digit V to the ankle | S30, S31 | the forefoot (palm) to the ankle, one shell, on ten spars from a seam along the flank, reaching round the foreleg, the free edge and the leg spread (#434; three, riding the legs, at W2) |
| Glider at rest reads as a small mammal, a fold at the flank | 10 fact 9, 4.3 | a band under a seam along the flank between the upper arm and the thigh, under a centimetre deep and about 4 mm off the skin, its outer face the coat (#434; at W2 a plate standing off the flank and under the belly) |
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
between neighbours turns by much: **a bat's three are its arm and its leg
blended**, each bone resting where the arm's and the leg's rest blended and
turning by their two turns slerped, since a flank spar of its own two bones
had to fold sharply to fit a crawl and stood out of the fold as a spike.
(Blended on a glider, every bone of them turned near a half turn and the
guard trimmed the patagium to a strip.) **A glider's patagium folds onto a
seam along its flank** (#434): ten spars root on the skin along a line a
tenth of the trunk above the shoulder and hip joints (about the flank's
widest), from where the trunk's own skin begins behind the upper arm to
where it ends ahead of the thigh - at least three tenths of the trunk in
from each joint, further on a long-limbed cross, until the skin across is
95% the trunk's and the fold's end stands clear of the limb's bones; spread,
they reach round the patagium's outline - the shoulder, the elbow, the wrist,
the forefoot, three points on the free edge, the ankle, the knee, the hip - so
the spread wing is W2's, taut along the arm and the leg and scalloped
between the forefoot and the ankle alone; folded, each is a line from its
root down the flank, the whole membrane a band 3% of the trunk deep (about 8
mm) lying 4 mm off the skin, each point laid off the skin across from it
where a line out from the midline first leaves the body, so it lies on the
trunk, never on a limb over it. A pose turns joints and never shortens a
bone, so a spar is carried out by a **carrier**: hung from a pin at its root
that turns by the hinge's turn and a quarter turn of a crank, it turns back
to the hinge's turn alone, so it holds what it carries moved by the crank's
chord. The rest of every point is held by **hinges**, one hung from each
joint holding the skin along the seam, each turning the fold onto the spread
plane alike about one line; a point's hinge share is split as the skin under
it is held (less what its carriers' own joints hold already), so folded it
goes as that skin goes, and the carriers' chords are twice the spars' reach
so they hold at most half of any point. Every joint the membrane hangs from
turns alike at every extension, so the binding blends only how far each
moved, which blends straight, and nothing the guard measures can tear; the
arm and the leg are held short first, on the body's own blends, and the
spars laid out to where they then reach. The first spar runs along the
flank into the shoulder and the last into the hip, rooted as an arm's or a
leg's edge is. Half spread, the band has swung halfway out and down and
stretched about halfway, a drape between the legs. W2's glider had three
two-bone spars from the flank to the free edge, folded out and down at the
middle beside the legs, and the patagium rode the legs themselves: at rest a
flat plate standing off the flank and under the belly (#434). #434's first
round pleated nine spars back and forth along the flank (a joint at each
pleat and a helper at each fold, 230 joints a glider), and the pleats stood a
centimetre or two off the skin and read as stacked strips. Hung from one
joint each, as both rounds' spars were, a fold on the skin went up to 13 mm
into it where a curl or a shake bent the spine under it: the skin there is
two joints' blend.
**Left for later**: a glider-led cross with a bat in it moves its wing as a
bat's in its gaits and rests (`rests::wing`), so its patagium opens partway
there, as W2's did.
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
mirrored. **And never onto the ground** (#440): the carry takes a tip
along the normal of the nearest point of the body, which for a feather
longer than the body it folds on is a leg or a foot, so a Bird x Bat
cross's secondaries, sized to a bat's forearm (0.38 to 0.41 m on a trunk
of 0.28), were laid on the ground and ran on through it, 83 mm under at
0.49, 64 at 0.45, 36 at 0.4 and 10 at 0.35, their tips anywhere from under
the far foot to ahead of the quill. **A wing the hug lays lower than 0.02
of the trunk over the ground goes over as one** (`feather::going_over`,
`Card::settle`): every card turned the same share of the way from where the
hug laid it to where the fold points it - back along the body and in over
the rump, its tip not carried - by how far under that floor the hug laid
the wing's lowest card, the whole way at a tenth of the trunk under it, and
tucked up in its own plane to a floor rising with the share from 0.02 to
0.15 of the trunk. So a bird, the rolls and a Bird x Bat cross to 0.32 are
laid as they always were (that cross's lowest card 0.024 of its trunk over
the ground at 0.3); from 0.33 to 0.37 its wing goes over, its rearmost
point moving at most a fifth of the trunk a hundredth of the blend; and
from there it lies back along the body, 60 to 94 mm over the ground. Laid
card by card instead (the round before: each card under 0.15 of the trunk
left where the fold points it, the rest as laid), from 0.22 one card after
another swung out to point straight back past the tail while the rest hung
down the flank, two layers of one wing as a creator dragged the blend; and
a floor that low with the share alone, the cards going over stood 6 mm off
the ground and sank 7 to 21 mm into it in a wing stretch, a preen and a
shake at 0.4 and 0.49. A covert
is held between the two feathers it lies over, graded by its place between
them. **Sized by the bone it grows on**: primaries by the
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
over 200 steps of the extension no drawn vertex steps more than three times
its own mean step (#428: the largest of all 1.86; it was six times the mean of
every step's largest, which a twelve-degree flip 80 mm from a bat's hand
joint passed at 3.45 and a 56-degree one within 20 mm at 1.93, and which the
per-vertex measure fails at 6.9 and 3.8), on every family and every cross at blends 0.3, 0.5 and
0.6 either way round, each as rolled, with the longest wings on the shortest
legs, with the longest wings, and with the heaviest composites; the wings are mirror images to a millimetre, and spread
at least 1.6 times as wide as folded. Triangles: bird 17,866 (feathers 1,470
in the hair's draw), bat 24,490 (membrane and digits 5,392), glider 22,060
(patagium 2,576); the dearest of the families, six crosses and twelve rolls
25,142. After the owner's look (v2): bird 19,426 (feathers 3,030 - 303
cards); bat, glider and crosses unchanged. Joints: bird 106, bat 130,
glider 68. After #434 the glider's patagium is 2,880 triangles (eighteen
columns across nine strips, against sixteen; the whole glider 23,364, the
dearest glider-led body of forty rolls 24,004) on 100 joints at the most
(a pin and a carrier for each of ten spars a side and a hinge for each joint
holding the seam's skin); `tests/winged_fold.rs` holds the fold to the skin,
its coat, its frames and its joints, and the spread edges to the legs.

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
- **A bat's hand folds a Z at 152 and 169 degrees, not quite flat** (W5,
  #424; W2 stopped at 149 and 143): flatter, the turns of a knuckle's two
  bones come a half turn apart spread, and dual-quaternion skinning blending
  them directly made the membrane jump. W5 adds a **helper joint at each
  joint of the hand** - the digit's root at the wrist, its knuckle and its
  middle joint - turning halfway between the bones either side
  (`wing::Helper`); the skin across the joint is shared with it rather than
  with the bone beyond (`wing::on_chain`), so no vertex blends two turns more
  than half the fold apart. 24 joints on a bat (192 in all, under a
  skinned mesh's 256). The first phalanx stops at 152: at 158 the guard
  (`wing::fit`) trims digits IV and V half shut spread on some bodies,
  their first phalanges' near half turns parting on opposite arcs; without
  the helpers it trims them 27 degrees shut at 152 too.
- **A bat's cape is drawn 60% of the way in toward its flank** at rest; the
  spread membrane near the body carries a slight ripple for it.
- **The bird's tail is rooted in its rump, not on W1's stub**, which ends
  about five centimetres behind the body.
- **Its rump is the trunk's, not the tail's** (#426): the binding's zone
  map hands the skin past a tail's root over to the tail
  (`SkinWeights::zone_map`), never where the dock is a marker - a bird's
  stub draws nothing, and a stored coat would paint the tail's colour
  across the rump. As built no bird's rump is held by its pelvis, so
  nothing drawn moved; held by it, a bird-led cross's rump gave 20 vertices
  to the tail.
- **A bird's humerals rise over its back when spread** (0.6 of the way out
  of the wing's plane, the tertials half that): the plane through the
  shoulder runs under the back, and in it they passed through the body.
- **The feathers take the plumage** (W4, #423): see "Surfaces". The
  membrane is the coat's, in its own atlas region.

## The feet (W3, #422)

**The feet are the leading family's**, as the posture and the wings are: a
bird-led body stands on a bird's toes, a bat-led one on a bat's, a
glider-led one on a glider's paws, each sized by its blended frame, and a
cross switches at the halfway blend. A bird's and a bat's toes are **drawn
as a part** on `extremity::hand`'s pattern (`extremity::toes`): grown on the
rig after the body is bound, each toe a chain of digit joints - one at the
start of every bone and a leaf at the claw's tip - `Role::Digit` in the
hind leg's extremity zone, under the foot's pad, so `Rig::extremity_joints`
holds them and `Rig::extremity_extent` spans them, while the contact stays
the pad (the plan's own joint, first in the zone) and `Rig::limb_chain`,
three joints above the extremity on every winged leg, never reaches them.
Grown after the wings, whose joints keep the numbers they had. **The foot's
tip is not a toe's** (#430): the `LimbTip` landmark, where a prop at the foot
sits (`Socket::LeftFoot`'s anchor and seat), is the last joint the plan hung,
the toe node with its real radius, as it was before the toes were drawn. Up
to 0.23.1 it was the last drawn joint, the middle toe's claw point - radius
nought, on the ground, and on a bat turned about 23 degrees outward. The
socket's joint, the ankle the foot hangs from, never moved. The body's
own foot is shaped under them into what they grow from - a bird's ball, a
bat's sole - by the analytic foot a beast's paw is shaped by
(`plan::beast_foot`), and the plan sets its second node only that far ahead
(`WingedFoot::sole_share`). The layout is one function
(`plan::winged_foot::drawn`): the part's tubes, its joints and the coat
painter's talons and bare toes all read it. A glider's paws are a beast's
analytic paw with a glider's digits; its toes grow no joints, since what a
glider grips with is its whole paw.

**The arrangement is derived, not stored** (`WingedParams::toes`): from the
beak's hook, so a bird with a hooked beak grips as an owl and a parrot do,
zygodactyl, and one with a straighter beak stands anisodactyl, the cut
halfway between the crow's hook and the owl's (`beakHook` -0.5). The default
bird, an owl, is zygodactyl. A review shows either through the beak:
`examples/render -- --winged --close foot --beak 1,-1,-1` (a crow's).

**No new `FootKind`.** Every engine pass that reads the kind asks whether a
foot is a person's or a hoof (`anim::ground`, `gait`, `rest`, `speed`,
`emote`); a toed foot stands on its toes as a paw does, so a bird's and a
bat's are `FootKind::Beast`, and what they carry is declared on the
skeleton (`BeastFeet::winged`, absent on a beast, whose skeleton is written
as before). The viewer matches no `FootKind` and would have had nothing
forced; an older worker would have read a new kind as `Beast` anyway.

| Target | Source | Built |
|---|---|---|
| Bird: four toes, phalanges 2-3-4-5 for I to IV, the claw counted | 10 3.1, S17 (H) | as the target, a closed tube a toe |
| Anisodactyl (I back; II, III, IV forward), about 90% of birds | 10 3.1, S17 (H) | the default: III straight ahead, II and IV 24 and 23 degrees out, I back 7 degrees inside |
| Zygodactyl (I and IV back; II and III forward): a parrot's, an owl's at will | 10 3.1, S17 | II 15 degrees in, III 6 out, I and IV back 22 and 27 degrees off straight back |
| Toe lengths | none in 10 (bg, L) | III the frame's toe measure (0.28 T on a bird) from its root; I 0.55, II 0.78, IV 0.86 of it; root radii 0.40-0.44 of the pad's; talons a third to a half of a toe, curling 80-86 degrees |
| Pads under the toes, hallux opposing the front three (perching) | 10 6.2, S13 | a pad under every joint (the tube's underside 35% fuller), the hallux back |
| Bat: five near-equal toes, strong hooked claws | the brief; 10 6.2 (hanging) | hallux 2 bones, II-V 3 (the mammal's formula, bg, L); lengths 0.90-1.0 of the longest; claws curling 126 degrees; a fan 28 degrees wide |
| Bat hind foot 0.4 T to its claws' tips | S21 (M) | sole 0.62 of it, toes the rest |
| Bat hind limb rotated, knee dorsolateral | S22 (H) | W1's leg (the knee up, out and back) unchanged; since W5 (#424) the foot trails out and back from its pad, its toes behind it |
| Glider hind foot: opposable, clawless hallux; II and III fused | 10 3.3, S31 (M) | a hallux of two balls on the inside, turned out and back as a thumb, no claw; II and III two narrow balls run together; IV the longest |
| Glider fore paw: five clawed digits | the brief | five, the thumb and fifth splayed, every claw horn |
| At rest the toes on the ground, not through it | the brief | every drawn foot's lowest point within 1 mm of the ground, every claw's tip 0.2 toe radii off it |

**Measured** (`tests/winged_feet.rs`): toe counts, phalanges and
arrangement per family and both bird arrangements; contacts and leg chains
those of the rig the skeleton roots, on every family and cross; every toe
in `extremity_joints`, no wing joint; the foot's extent along its toes at
least 1.3 times what it is without them; the toes at rest within a
millimetre of the ground; closed tubes, at most four influences summing to
one, all the foot's own digit joints, the feet mirror images to 0.1 mm;
and no person or beast grows a toe. Triangles: bird 20,578 (toes 1,152),
bat 25,186 (toes 696), glider 22,060 (unchanged); the dearest of the
families, six crosses and twelve rolls 25,838, a bat-led roll, under the
26,000 ceiling. Joints: bird 142 (36 toe joints), bat 168 (38), glider 68.

**Walked** (a scratch probe through `Driver`, standing two seconds, walking
three at 1.2 m/s, stopping): at rest no drawn toe is below the ground. In a
stride the gait read the toes as sole points and peeled a foot about the
foremost of them, carrying the rest rigidly, so the toes dipped at toe-off:
a bird's 3 mm, a bat's 14 mm, a bat-glider cross's 16 mm, and a bird-led
bird-bat cross's 16 mm in its stride and 42 mm in the one frame it set off
in. W5 lays them on the ground (#424): see "Motion", "On the ground".

### Where the feet depart, and why

- **Research-10 gives no toe lengths**: every length and radius above is
  bg and chosen by render, an owl's thick toes and long talons for the
  default bird.
- **A bat's foot trails out and back since W5** (#424; `CRAWL_TOES` in
  the derive), S22's rotated limb carried through to the foot. W1 and W3
  pointed it forward and out because the gait's toe-off, which pitches every
  four-legged foot's pad about its side, rolled trailing toes 49 to 58 mm
  into the ground; W5 lays the toes on the ground through every stride
  instead (`anim::winged::ground::lay_toes`). The membrane, anchored at the ankle,
  was never the limit.
- **A bat's toes are drawn coarser than a bird's** (six sides a ring, no
  ring between joints): ten toes on a body already within 1,600
  triangles of its ceiling.
- **A claw is lifted off its toe's line at its base** just enough that its
  tip comes down to rest 0.2 toe radii off the ground: an owl's talon
  continuing the toe's line would end below it. The distal phalanx raised,
  as a standing bird's is, but drawn in the claw rather than the toe.
- **A glider's digits melt into its paw** at one refinement pass: its pad
  is smaller than a dog's, and the hallux reads as a lump on the inside
  rather than a thumb. A second pass on its paws alone cost 8,000
  triangles.
- **A bird's leg is feathered to the ball**: the ball and the toes are
  bare, scaled skin (the coat's leather, a stand-in until W4), the tarsus
  above it the coat.
- **The toes do not grip or curl**: perching, hanging and clinging are the
  owner's later phase, and the chains are what a grip will turn. On the
  ground they are turned together about the pad to lie on it (#424).

## Surfaces (W4, #423)

**The surface is the leading family's**, as the posture, the wings and the
feet are: a bird-led body is painted from its plumage, a bat- or glider-led
one from its coat, and a cross switches at the halfway blend. Both are stored
on `WingedParams`, and each is kept across a change of family (below). Judge
them with `cargo run --release --example render -- --winged --beak 1,-1,-1
--plumage crow` (or `owl`, `parrot`, `songbird`, or `beak` for the preset its
head is nearest), `--winged --head` for an owl's disc, and `--winged --gallery 9
--family bird` (or `bat`, `glider`), whose tiles print each body's surfaces.

### Plumage (`plan::Plumage`)

**Colours by tract, not loci.** A beast's coat is the genetics a breeder would
name; a bird varies by which colour each feather tract wears, and a field
guide describes one that way (10 5.1, 5.3: "patterns follow feather tracts").
So a plumage is a colour per tract, a pattern, an owl's disc, a gloss and the
seed its marks are laid out from, every colour free:

| Field | Reaches |
|---|---|
| `head` | the crown, nape, cheeks and throat, outside a disc; the throat's undersides take `under` |
| `upper` | the back, mantle and rump, and the wing's coverts, humerals and alula |
| `under` | the breast, belly, flanks and the feathered legs |
| `flight` | the remiges: primaries, secondaries, tertials |
| `tail` | the rectrices (the upper tail coverts are `upper`'s) |
| `mark` | what the pattern is drawn in |
| `disc`, `ruff` | an owl's face inside its ruff, and the rim: shown only on a head with a disc |
| `pattern` | `plain`, `barred`, `spotted`, `streaked` |
| `gloss` | `0..1`: none to a raven's |
| `marks` | the layout seed |

**Presets per kind of bird** (`Plumage::OWL`, `CROW`, `PARROT`, `SONGBIRD`),
each the first of a short list of palettes after birds a field guide shows:
owls (a little owl, the default; a tawny, a barn owl, a snowy, a great grey),
crows (a carrion crow; a raven, a jackdaw, a hooded crow, a magpie), parrots (a
scarlet macaw; a blue-and-yellow macaw, a rainbow lorikeet, a budgerigar, an
African grey, a cockatoo) and songbirds (an eastern bluebird; a robin, a house
sparrow, a cardinal, a blue tit, a song thrush). Every colour is chosen by
render. **A roll draws the kind its head is drawn as**: the kind is not
stored, so the plumage roll asks the head roll's streams again for the kind
they drew (`winged.beakKind`) while the head is the one they drew, and
otherwise - a creator's head, one a lock kept - reads it back off the head's
three axes (`WingedParams::bird_kind`): **an owl if it has a disc**
(`facialDisc` above -0.75, a quarter of a full one; only an owl's roll draws
one), else the disc-less preset nearest it. Read as the nearest of all four,
an owl whose hook rolled wild stood nearer a parrot and wore a macaw's red
with no disc (49 of 1,190 owls), and a crow's and a songbird's beaks, a
quarter's spread about presets 0.85 apart, wore each other's colours about
one roll in fifty: 1,898 of the 1,987 bird-led rolls in 6,000 seeds wore their
head roll's kind, 1,947 with the disc read first, and now every one does. A roll picks one of that kind's palettes
evenly (`winged.plumage`), lighter or darker by up to a tenth all over
(`winged.plumageShade`), with a fresh layout (`winged.plumageMarks`): a
parrot-beaked bird wears a parrot's colours, as a beast's roll picks its free
colours off a list (#403). `WingedParams::plumage_palettes` and
`plumage_for_head` hand a creator the same rows to pick a whole plumage from.

**The painter** (`texture::coat::paint_plumage`, `PlumageSampler`):

- **Tracts by the body's own frame.** A perching bird's trunk is raised
  0.95 rad, so "up" is not its back: the back is the side its dorsal axis faces
  (square to the spine from the hips to the withers, toward the sky), and the
  upper tract turns into the under round the flank over a band (the cosine
  -0.45 to 0.25). The head's colour fades in by how far a texel lies off the
  declared head and a feathered leg's by how much of a hind leg it is (the
  beast's junctions, its wing arms excluded): no tract stops on a zone's line.
- **A mark shows where it stands out**: the pattern is drawn on a tract only
  as far as their luminances differ (0.12 to 0.30), so a little owl's cream
  spots land on its brown back and crown and not its buff breast, a thrush's
  dark ones on its pale breast, a snowy owl's dark bars everywhere. Spots at 16
  per back height; bars 13, narrow, wavering and broken into lengths (drawn
  whole they read as a striped jersey); streaks 20 cells drawn out five times
  along the spine.
- **An owl's disc is painted, not tinted**: #422's `ruff_shade` vertex tint
  is folded in and gone. The face inside the ruff is `disc` and the rim
  `ruff`, read off the declared head's carved dish (`BeastHead::in_dish`,
  `on_ruff`), so a pale face ringed dark is exactly what a record says. The
  ruff's colour reaches 1.15 to 1.8 of its radii off its middle line: its
  surface stands one radius off, and #422's reach, 0.4 to 1, painted nearly
  nothing in the atlas.
- **A crow's gloss is a roughness and a sheen.** The skin's ORM map carries
  roughness, so a gloss lowers it over the back, the coverts and the head
  (0.72 to 0.40 on a crow's back, 0.32 a raven's) and its highlight tightens;
  the draw has no view-dependent channel, so a cool sheen, drifting between a
  corvid's blue-violet and blue-green, is laid into the colour there (0.22 of it
  at a full gloss). **A true iridescence** - the hue turning with the angle it
  is seen at - would need a thin-film or iridescence term in the skin's material
  (Bevy's `StandardMaterial` has none) or a material extension reading a
  hue-shift map; the feather cards, in the hair's draw, share its material and
  take only the sheen.
- **The beak and the toes are not plumage**: the beak keeps the horn its
  length gives it (#422) and the bare toes and talons the leather and horn W3
  painted them in, from the bird's own coat (`WingedFamily::default_coat`).

**One sampler for every feather** (`PlumageSampler::card(root, normal, zone,
feather) -> FeatherTone`): W2's stand-in tones (`FLIGHT_TONE`,
`COVERT_TONE`) are gone, and every card takes its colour root to tip from
where it roots and what it is - a remex `flight`, a covert, humeral or alula
`upper`, a rectrix `tail`, an upper tail covert `upper`, a contour feather the
skin's own colour at its root - each root darker than its tip, barred across
on a barred plumage (two bars, at the rows a six-row card has), every other
covert tipped with the mark on a spotted one, and glossed as the skin is. The
contour feathers a later part grows call the same sampler.

### Coats (`WingedParams::coat`)

A bat's or a glider's fur and membrane are **the beast's `Coat`**, its loci
and its picked colours, stored as `coat: Option<Coat>`: **absent until a roll
or a creator stores one, absent reading as the leading family's own**
(`WingedFamily::default_coat`): a fruit bat's near-black (black, solid: a flying fox's dark body, its golden mantle a mark the painter lays over it), a
sugar glider's mid grey (a picked grey, 0.30, over the diluted black, whose
blue-grey read nearly white: the owner's call at W4's review), each lightening
toward its belly as every coat does. `WingedParams::coat()` is what every reader asks: the stored
coat on a bat- or glider-led body, its family's while none is, and on a
bird-led body a bird's own, whose leather paints its toes. A roll draws a coat
for a bat- or glider-led body from its families' loci on the beast's coat
streams (`Coat::reroll_winged`): a bat mostly dark brown or black, one in seven
sable (a russet tipped dark along the back); a glider mostly blue-grey, a quarter
grizzled, now and then a mosaic's white patches or a leucistic glider's white;
and as a beast's, one in twenty-five an unnatural palette and one in twenty
free colours. A cross with a bird draws its mammal family's loci alone. Every
frequency is chosen by render: research-10 5.3 gives the colours, not how
often. The glider's dorsal stripe and the fruit bat's mantle are the family
marks, painted with the fur (#423's other half).

### The wire

- **The record**: `plumage` (`#wingedPlumage`) and `coat` (`#quadrupedCoat`)
  on the `winged` definition, optional and defaulted, each left off while it
  is the default owl's and absent, so a body in its family's surfaces writes
  the record 0.21 wrote. A 0.21 record, which stores neither, reads as its
  family's defaults: the default owl's plumage and its family's own coat.
  **Those are not all the stand-ins 0.21 showed**: a bat's default is the
  near-black 0.21 painted (black, solid), but a glider's is now blue-grey
  (black, diluted) where 0.21's stand-in was a dark grizzle (black, wild
  agouti), and a bird is painted in its plumage where 0.21 painted a brown
  stand-in coat (liver, wild agouti), so a 0.21 glider or bird does not look
  as it did. Colours are thousandths; the gloss is thousandths of a share.
- **The share code, version 14**: after the head's three spans, which 14
  always carries, a byte of which surfaces follow, then the plumage (eight
  colours a byte a channel, the pattern, the gloss, the marks' two bytes: 28)
  and the coat (the beast's six bytes and its picked colours' block). **Written
  only by a body that needs it**: a winged body whose plumage is the default
  owl's and which stores no coat goes on writing 13 or 10, byte for byte, so a
  0.21 reader takes it; a 0.21 reader refuses a 14 as a version from the
  future. **The default owl's plumage is asked of the bytes it would write**
  (#438), as the head is (#429): a plumage within half a byte of the owl's on
  every channel writes the owl's code and reads back as the owl's, where up to
  0.24.1 it wrote a 14 carrying exactly the owl's bytes; such a 14 still reads,
  and writes the owl's code after. **A stored coat rides whatever it is** (#431): up to 0.23.1 one equal
  to its leading family's own was left off, and the body came back storing
  none - the same look while that family led, but made another family it
  wore that family's coat where the body it was read from kept its own.
  Codes up to 13 read as the default plumage
  and the family's coat, which a glider and a bird wear in other colours than
  0.21 painted them (above).
- **Sanitised and quantised**: every colour clamped to `0..1` on the wire's
  grid, a channel that is not a number reading as black (a coat's picked
  colours' rule), the gloss to a share; every preset is authored on the grid,
  so it is what it reads back as.
- **A change of family keeps both surfaces** (`set_kind`, after the beast,
  whose coat goes with the body across a change of family, #368): the plumage
  paints only a bird-led body, so it rides a bat or a glider unseen and a bird
  made a bat and back comes back in its own feathers, as its beak does; a
  stored coat is a creator's or a roll's and goes with the body, while an
  absent one reads as whichever family leads, so a bird made a bat wears a
  fruit bat's brown and made a glider a glider's grey. Nothing is handed over.
- **`GENERATOR_VERSION` 11**: a winged roll draws its surface on new streams
  (`winged.plumage*`) and the beast's coat streams, which no winged roll drew
  before; no person or beast moved. No migration: a saved winged body stores
  no surface and reads as its family's defaults, above.

### The eyes

**An unrolled winged body wears its family's eyes**: `AvatarRecord::new` gives
a winged record the plan's own iris at 0.33 of its ramp (about a roll's mean;
the rim darkened 0.22) - an owl's gold, a bat's and a glider's dark brown -
where it had a person's blue until rolled. Persons and beasts keep theirs, to
the byte. **Each kind of bird has its own** (the owner's call at W4's
review), read off the head as its plumage is: an owl's gold (the cat's ramp),
a parrot's pale cream to straw, a crow's and a songbird's dark brown (the
horse's ramp, short of its wall eye).

### Where the surfaces depart, and why

- **A plumage is colours, not feathers' genetics**: no locus says why a bird
  is the colour it is, so a cross between two birds cannot take a colour from
  each parent the way a beast's coat can; every bird is a whole palette.
- **The kind is the head's, read back** where a roll did not draw the head:
  a creator who reshapes a beak keeps the plumage they have, and the rows
  they are handed are an owl's on a head with a disc and otherwise the
  nearer kind's.
- **One pattern a plumage**, drawn wherever it stands out: a little owl's
  spotted back and streaked breast are one or the other, and a pattern on the
  breast of a pale-bellied owl does not show.
- **No wing bars, rump patches, eye stripes or two-tone bills** (10 5.3's
  pattern rules): a parrot's yellow coverts, a robin's orange face against its
  olive crown, a magpie's white scapulars are tracts the plumage does not
  separate. Each would be a colour more on the wire.
- **The gloss is static**: a sheen in the colour where a real one turns with
  the view (above); the feather cards cannot take a lower roughness, since
  they share the hair's material.
- **A barred feather has two bars**: a card is lofted in six rows.
- **Countershading on a coat, none on a plumage**: a coat lightens toward the
  belly by its own rule (#368); a plumage's belly is its `under`.

## Cards, marks and the membrane (W4, #423)

Three surfaces, each the leading family's, as the wings are: a bird-led body
is feathered, a bat- or glider-led body furred, and a bat's membrane is naked
skin with a material of its own. A bird's feathers take its plumage
(`texture::coat::PlumageSampler`, see "Surfaces"); a bat's or a glider's fur,
marks and membrane its coat (`WingedParams::coat()`, stored or its family's).

**A bird's contour feathers** (`hair::fur::contour`, 10 5.1). Up to 760
cards, each a small feather of seven triangles - narrow at the quill,
broadest past the middle, rounded to a point - scattered over the body's own
faces by area (the head's a little denser, its cards smaller, the legs'
smaller still) and laid along the body's flow: from the beak back over the
crown and down the nape, down and back along the body, down a leg. **Back
over front**: each card goes in under the skin and lies along it, its tip
laid on the skin where it lands and lifted 0.07 of its length, bowed out over
the skin where the body bulges under it, so the card ahead covers where the
next comes out. They leave clear an owl's dish (inside the ball whose inside
the dish is, as strongly as the disc is carved), a margin round the beak and
round each eye - all read off the parts the head declares, in its frame,
never off whether a point is inside the head - the bare scaled ball and toes
(the foot's zone), and the flank the folded wing lies over (`Clear`: the body
within 0.03 of the back's height of any folded feather grows none, out to
0.06 fewer - **until W5** (#424): spread, a wing bared painted skin under it,
so the flank keeps its cards' weight and grows them shortened, to two
fifths within 0.03 and out to 0.15, where a card rooted beside the fold
reaches under it: `Clear::thinned_to`, `UNDER_WING`). **And a card that still passes through a folded feather is left
out whole** (`Sheet`, `leave_out`): `Clear` reads the feathers' vertices,
which are sparse along a long feather, and the default owl had 20 card faces
crossing its folded wing, a bird-bat cross at appeal -1 195. A card whose tip would have to be carried more than a third of
its length to land on the skin - over the crease where the head sits on the
breast - is left out. Each card is coloured at its root by **one call**, the
`colour` closure `Avatar::grow_winged_cards` hands `contour::grow`: the bird's
plumage sampler (`PlumageSampler::colour`), so a card at its root is the
colour of the skin it grows from. A card is a shade deeper at its quill
than its tip and a little lighter or darker than its neighbours.

**A bat's and a glider's fur** (`hair::fur`'s zones, 10 5.3) from a fur
derived on the plan, not stored (`WingedParams::fur()`): a fruit bat a ruff
of 1.6 (its mantle's long fur, which on a crawling bat's low back reaches
about a fifth of the back's height), no tail (*Pteropus* has none, S21) and
no tufts on its plain ears; a sugar glider a brush along its tail of 1.0 and
nothing else; each zone blended across a cross by its blend, and none on a
bird-led body. The ruff reaches the cheeks and the throat behind the jaw,
never the muzzle, so the fruit bat keeps the fox face 10 6.1 gives it.
Coloured as a beast's fur is, so a bat's ruff takes its mantle. **None on
the membrane**: no card roots within 0.04 of the back's height of a membrane
at rest (0.08 fewer), and a card that still reaches through one is left out
whole (`Sheet`, `leave_out`) - **at rest, half spread and spread**, each card
moved by its own skin as the pose moves the arm under it: a ruff card on the
fore limb or the chest rides the arm out through the propatagium as the wing
opens, which the rest alone does not show (the default bat had none crossing
at rest, 18 edges half spread and 25 spread). **None on an eye**: no card of a
winged body roots within 1.25 of the declared globe's radius of an eye (as
the contour feathers' face term), and a card one of whose edges comes within
1.15 of the globe's own radius of it is left out whole (`mark_near`); a ruff,
1.6 long and reaching the cheeks, lay across a rolled bat's eye up to 1.7 cm
deep in a 5.7 cm globe, and kept from rooting alone, still reached seeds 6,
32 and 71's. The ruff stays 1.6: what is left out keeps it off the eye.

**The marks** (`texture::coat`'s `winged`, 10 5.3), derived and not stored,
each as strong as its family's share of the body, on bat- and glider-led
bodies only: a fruit bat's **mantle**, a golden brown (leaning a quarter of
the way to the coat's own light) over the neck and the shoulders, over the
top and down the sides, faint under the throat, gone down the chest, the
fore limbs and before the head; a sugar glider's **dorsal stripe**, its coat's
dark darker still, a share of the spine wide, from the forehead (ending
before the snout) down the back and fading over the rump, and its **face
stripes**, a ring round each eye and on through it to the ear's root. Every
mark is placed by anatomy - the spine, the junctions, the head's frame - so
none stops on a chart's line, and laid after the pattern and before the
belly lightens.

**The membrane's material** (10 5.2). A membrane has a region of the atlas to
itself, and each texel there says what it is by where it lies
(`wing::charted`): each piece a column, its rows from root to free edge (a
propatagium across its column). A bat's sheet is dark brown, near black;
from halfway out it warms and pales toward its free edge, as if lit through,
half the way to a reddish brown at the edge; faint darker fibres run along its
rows; its roughness is leather's, a little duller; its digits are the bone
under their skin, darker. A glider's patagium is furred, so it is its coat,
paling a little toward its edge.

**Measured** (`tests/winged_cards.rs`): every bird-led body - owl, crow,
parrot and songbird heads, crosses either side of the halfway blend, rolls -
grows at least 500 contour feathers, none rooted in an owl's dish, none
within a twentieth of a head radius (half a centimetre on the owl) of a
mandible, none bound to a foot, no edge of one through its folded wing;
every bat- and glider-led body - families, crosses either way round at 0.3,
0.45, 0.55 and 0.7, rolls - grows fur, a bat its ruff and a glider its
brush, and no edge of a card passes through its membrane at rest, half
spread or spread; no winged body's card has a vertex inside or within a
tenth of a radius of an eye's globe (families, crosses, the young end of
bat-glider crosses, rolls); a bat's shoulders are paler and
warmer than its back, a glider's spine darker than its flank and the line
behind its eye darker than its cheek; a bat's membrane is paler and warmer at
its free edge than in its middle, and dark there, and a glider's patagium paler
at its edge; and every body, cards and all, makes its draws and costs no more
than `wing::MAX_TRIANGLES`. Each test was run against the code with its guard
taken out and failed.

### The budget

A winged body's cards are **fitted to what its other draws leave of the
27,200** (`wing::MAX_TRIANGLES`, raised from 26,000 at the owner's call at
W4's review), as a beast's fur gives up what its head
wears: the body is built and measured without them, and they are grown to
the remainder, thinned in an order that spreads over the body (`Thinned`).
A card left out - through a wing part, at rest or spread, or near an eye -
gives its share back, and the cards are asked for once more with it.
Triangles, each body as `build_for_shape` builds it:

| Body | Whole | Cards | Of what they ask |
|---|---|---|---|
| Bird (owl) | 21,594 | 4,816 (688 contour feathers) | all |
| Bat | 25,888 | 764 (ruff) | all |
| Glider | 23,060 | 660 (tail's brush) | all |
| Bird x bat 0.3 / 0.7 | 23,152 / 25,994 | 4,690 / 804 | all / 872 |
| Bat x glider 0.3 / 0.7 | 25,996 / 23,696 | 308 / 1,596 | 1,308 / all |
| Glider x bird 0.3 / 0.7 | 22,946 / 24,848 | 654 / 4,900 | all |

The dearest of the families, crosses (0.3, 0.45, 0.55, 0.7 either way
round) at appeals -1, 0 and 1.5, and 100 rolls is 26,000; the thinnest fur
any bat-led one wears is seed 2's 156 triangles, of the 1,332 it asks.

**At 26,000 a bat-led cross could not wear its whole fur, so the ceiling is
27,200.** The table above and the paragraph before it are the 26,000
measurements that decided it. A bat's body
without cards is 25,124 (the membrane and digits 5,392 of it, the toes 696),
which leaves its own ruff room, just; a bat-glider cross asks for a ruff and
a brush, about 1,320, on a body as dear, and the dearest such roll keeps a
few dozen cards. Built untiered, the dearest of the same 181 bodies is
27,158 (seed 2, a bat-glider cross), and 49 of them pass 26,000 - 47
bat-led, and two bird-glider crosses at appeal 1.5 at 26,285; **a ceiling of
27,158 lets every one of them wear its cards whole**, still well under the
engine's 33,500 target for a person (docs/budget.md). W5 (#424) thins the
contour cards under a folded wing rather than clearing them, so a spread
wing shows no bare skin; the dearest is then 27,164, the same seed 2. The other levers are the membrane's grid (a pair of
plagiopatagia alone is 2,576), which W2 tuned for a continuous extension,
and are not this phase's.

### Where the cards, marks and membrane depart, and why

- **Hundreds of contour feathers, not thousands**: 10 counts 1,100 to 4,600 on
  a songbird. Each card stands for several, and the coat painted under them
  fills what a few hundred leave.
- **A contour feather is a solid card, its outline the geometry's**, cut from
  the strand mask's solid run as a flight feather is: the mask's frayed end
  read as hair.
- **No feather tracts or apteria**: research-10 names the tracts but gives
  their layout for no species; the cards cover the body evenly, and a bird's
  plumage colours (a stored plumage, builder A's) are where tracts will show.
- **No plumage volume**: 10 note 8's "fluffing and sleeking" axis is not
  built; the cards lie at one lift.
- **An owl's dish is bare of cards**: its stiff feathers are W3's carved
  relief and shading.
- **A bat's fur is a ruff, not a pelt**: a beast's zones grow long hair, and
  the rest of a beast is painted; so is a bat's. Its dense body fur (10 5.3)
  is the coat's grain.
- **The light through a membrane is faked in colour**: the skin's draw is one
  opaque material shared with the body. A real one needs the renderer to give
  the membrane a draw of its own with a two-sided material that transmits -
  diffuse transmission through a thickness, or a thin-surface subsurface
  term, lit from behind - and a map of how thin it is where; the colour here is
  what such a membrane looks like with the sun behind it at a game's distance.
- **The fibres barely read**: research-10's elastin bundles are drawn as faint
  lines along the sheet's rows, visible close up and gone at play size. No
  veins.
- **Both sides of a membrane are one texture**: its two sheets share their
  texture coordinates, so a glider's patagium cannot be grey above and cream
  below (S31) without charting the underside apart.
- **The marks key on the family, not the coat**: a recoloured bat keeps its
  mantle (leaning toward its coat's light), a glider its stripes. A record
  cannot turn them off.

## Motion (W5, #424)

A winged body moves through the driver that moves a person and a beast,
and everything it adds lives in one place, `anim::winged`, a file for each
concern: **on the ground** (`ground`: the bird's walk, hop and run, the bat's
crawl and bound, the glider's bound, the drawn toes on the ground), **in the
air** (`flight`: a sustained source its host flies) and **at rest** (`rests`:
the family's rests and emotes and its roster). Every entry asks the rig's
wings first, so no person or beast is touched.

**What the body is, stored once.** `Wings::kinship` (a `Kinship`) holds each
family's share of the lineage and the leading family, written when the wings
are grown and `serde(default)` across the worker. Every choice of one
family (which gait, whether the wings beat and how a takeoff opens them,
which roster) is the leading family's (`Lineage::leading`), so a cross at
exactly 0.5 takes the one answer its surface and its posture wear; every
blend (a beat's rate, width and fold, the dihedral, a rest's curl and its
times) is the shares'. **A rig from an older build carries none**, and every reader
alike takes the family its stance belongs to: wholly a bird on two legs,
wholly a bat on four (a glider from before it was written goes as a bat,
which no rig's joints tell apart). Beside it each concern keeps its own: a
bird's `Gaits` (`bob`, `hops_from`) and `Wings::low` (how low the folded wing
hangs), both written at the build, and the flight's numbers, read off the
rig as it flies. None of it is on `mesh_serde::Wire` or the rig's outline.

**The driver's order**, highest first (`anim::winged`'s header states it
whole, `driver.rs` beside the code):

1. **The water and a shown motion** outrank a flight.
2. **A flight** (`Inputs::flight`, on a body with wings) holds the body in
   the air for as long as the host gives one, and plays **no rest and no
   emote**: one asked mid-flight is let go, not kept for the landing, and
   one playing when a flight is given - a flight given mid-perch - ends that
   frame, the change of source to `Source::Fly` blending the body out of it.
   The flight starts from the ground posture's values, the wings folded.
3. **The ground gait or the idle** carries every other frame. A landing
   hands back when the host gives no flight once `Land` has absorbed and
   folded: the gait or the idle takes the next frame, blended from the
   landing's last pose.
4. **A rest or an emote** rides over the gait or the idle, as a beast's
   does, and whatever stirs the body gets it up from a rest in a hurry.

The drawn toes are lifted out of the ground after the blend
(`ground::keep_toes_up`) on the gait's and the idle's frames only, the
blends into them included: a flight's and a rest's or emote's poses own their
feet - a landing plants them, a perch grips them, a curl folds them under.

**And a body on four legs is kept out of the ground it is handed**
(`ground::keep_clear`, #441): on the pose handed back, after the blend and
never on the pose kept to blend from, whatever carries the body but the
water and a rest - its belly lifted off the ground, a crawling body's hands
laid along it, its tail lifted off it. **Eased, never switched**: nothing
kept to blend from carries it, so it is eased out going into a swim or a
rest and in coming out over two blends (`KEEP_EASE`); switched with the
source, a bat's whole hand turn and belly lift went in one frame at either
edge of every emote, rest and swim, a joint 115 to 118 mm a sixtieth further
than without it at 60 frames a second on a 20 degree slope it faced up (290
at 144), the slope's whole 20 degrees. An emote keeps the stance, and the
hold with it. See "On the ground". `Driver::posed` is the pose kept to blend
from, before it.

**Persons and beasts do not move.** `scripts/compare-motion.sh` drives four
people and ten beasts through a stand, a walk, a trot or run, a gallop or a
sprint, a curve, a stop, a pivot and an idle (`examples/motiondump`) on this
tree and on HEAD's, and compares every frame's pose bit for bit: identical
at 60 frames a second; at 144, people identical and beasts identical up to
the first frame they pivot (#415, below). A person or a beast handed a
flight moves exactly as without one (`tests/winged_flight.rs`), and none is
offered or moved by a rest's or an emote's target (`tests/winged_rests.rs`).

**The whole life** (`tests/winged_life.rs`): the three families, each pair
crossed at 0.49, 0.5 and 0.51, and three rolls, driven through standing, a
walk, the fast gait, a stop, a takeoff, flapping, a glide, a flare, a
landing, a walk, the family's rest and up, an emote and standing again, at
60 frames a second and at steps of a fortieth and a hundred and twentieth in
turn: no joint moves in a frame more than twice the most it moved in the four
frames either side and a hundredth of the trunk (each frame's travel read at
a sixtieth's length), and on the ground no drawn point goes more than 6 mm
under where the body stood. Without the blend out of a landing, a bird's
pad moved 88 mm a sixtieth against 17 around it. Two things are let off, and
are the owner's: through a change of pace on the ground a glider's forepaw
rolls 6 to 10 mm into the ground, as a beast's paw does (a dog's 44 mm and a
horse's 120 in the same stretches), and is held to the toes' 1.5 mm there;
and over the uneven steps a bird's and a glider-led cross's thigh jumps 50 to
80 mm in one frame of a stride, whatever the frame's length (a person and a
dog step evenly), so the ground gait's own frames are held to the pop bound
at 60 only.

### On the ground (`anim::winged::ground`)

Every winged body is driven by the gait that drives a person and a beast - a
bird on two legs, a bat and a glider on four - and `anim::winged::ground`
adds what research-11 says each family does that the others do not. **The
gaits are the leading family's** (`Kinship::leading`), as the posture is; a
bird's two habits are read off its plan when its wings are grown
(`wing::Gaits::of`) and carried on the rig (`Wings::gaits`: its `bob` and
`hops_from`), so they cross a worker boundary with the rig. A rig from an
older build carries none, and its bird walks and runs with neither.

| Target | Source | Built |
|---|---|---|
| Bird stride on hip height, `lambda/h = 2.3 Fr^0.3`, Froude on hip height | 11 2.1 (M-L) | `BIRD_STRIDE` 2.3; a person's step off the leg's reach was half as long again on a bird's short leg, and its crouch for each long step dropped the body 22 mm in two frames as each foot landed |
| Walk to run near Fr 0.5 without a jump in duty, grounded running | 11 2.1, fact 6 (M) | a person's walk duty to Fr 0.5 (0.65 to 0.55), then falling to 0.5 at Fr 1.2 (a grounded run) and on to a sprint's 0.22 (`bird_duty`) |
| Head-bob: one a step, hold still in the world, thrust about half the step, the hold just after a foot lifts | 11 2.2 (H, M) | `bob_offset`: held from 0.05 of a step after the trailing foot lifts, thrust over half the step on a smoothstep, centred on where the head rode; the neck turns so the head moves exactly that far along the walk, the head turned back to keep its bearing; held along the ground, not in height (about 10 mm of the walk's bounce and the neck's dip over a hold) |
| Bob fades past Fr 0.5 | 11 2.2 (M) | `bob_share`: whole to Fr 0.3, none from 0.5 |
| Who bobs: pigeons, fowl, corvids; not raptors, owls, parrots | 11 2.1 (M-H) | by the beak: none on a hooked one (`beakHook` from -0.5 up: the owl, the parrot), all of it on a straight one (from -1: the crow, the songbird) - the toes' own cut |
| Who hops: small passerines; a corvid walks below about 1 m/s, then hops | 11 2.1 (M-H) | `hops_from`: at every pace on a bird of `size` 0.45 m or less (`HOP_SIZE`); from Fr 0.5 on a straight-beaked one, back to a walk below four fifths of it; never on a hooked one, which runs |
| Hop: two feet together, a run's step and duty | 11 2.1 (H, qualitative) | both feet down together, a run's duty (0.35 to 0.22), a step 1.6 times a walking step (`HOP_STRIDE`, chosen by render: at a run's step a songbird cleared a quarter of its hip and read as bouncing on the spot, now half); risen as a body thrown for the time it is in the air, `g t^2 / 8` (`hop_rise`) |
| Hop body: head retracts as the legs fold, extends as it leaves, retracts before landing | 11 2.1 (M) | the head 0.2 of the leg ahead at take-off and behind it half a cycle later (0.08 did not show) |
| Bat: lateral-sequence walk on wrists and thumbs, the feet behind | 11 2.3 (M) | the beast's walk on the wrist's pad (the plan's fore contact since W1), the hind foot trailing out and back (`CRAWL_TOES`) |
| Bat bound at 0.56 to 1.14 m/s, forelimb-driven | 11 2.3 (M, H) | `Pattern::Bound` from a beast's change to a trot (Fr 0.5): the hinds together, the fores at 0.3 and 0.36 of the cycle, the flight after them; duty 0.32 to 0.24, a stride 1.4 times a beast's, thrown three times its flight's height and the back folding 1.8 times a beast's (all chosen by render at the look review, where the bound read as a fast walk: every foot now off the ground by 36% of a bat's hip and 29% of a glider's, the back through 0.61 and 0.63 rad, from 0.38 and 0.40) |
| Glider: half-bound and bound, hinds together | 11 2.4 (L) | the same bound after the same walk |

**A crawling bat's legs are its own** (`winged::crawls`): the pantograph that
stands a beast's paw and the pole a beast's stride swings a leg about both
work in the body's side plane, where a bat's leg is folded out of it, and its
elbow jumped 100 to 330 mm between frames in a pivot and a trot before W5 (the
0.22 tree, measured). Its legs are solved about the bend they were built
with, each foot stood upright; its strides are centred on its own feet, not
under its blades and hips as a beast's are, which drew its wrists 23 cm back
and folded its elbows in over its spine within four frames of setting off;
and neither its wrist nor its trailing foot is folded in the swing, where a
paw's fold flicked the foot's toes up through 55 mm a frame. So its elbows
stay high and out over its wrists, and no joint of its body moves more than
45 mm a frame setting off into its crawl or its bound. A glider's legs are
a beast's, and go as a beast's do.

**The drawn toes stay on the ground** (`ground::lay_toes`): through a stance
the foot is laid square to the ground under it and its toes and second node
turned together about the pad by the least turn that brings the lowest of
them to its height at rest, so they lie on the ground as the pad peels off
about them; the stance's turn is let go of over the first 0.3 of the swing,
and in the air no toe passes into the ground. After the driver's blend
(`keep_toes_up`), a pad a blend left in the ground is lifted out by the leg
and its toes laid out of it: a bird setting off sank its pads 28 mm in the
first frame, each joint turned on its own between two poses that stand on the
ground.

**Measured on the ground** (`tests/winged_motion.rs`, through `Driver` at 60
frames a second):

- A crow walking at 0.6 m/s (Fr 0.19) holds its head within 30 mm/s for 7 of
  a step's 13 frames and thrusts it at up to 1.8 m/s, one hold a step; the
  head dips 18 mm as the neck reaches. The default owl never holds.
- A songbird-sized bird hops at 0.4 m/s at 5 Hz, its body rising 24 mm and
  its feet 31 mm; a crow walks at 0.6 m/s and hops at 1.6.
- A bat walks in a lateral sequence at 0.4 m/s on two feet or more, and
  bounds at 1.0; a glider bounds at 1.2, where a dog at the same Froude
  number trots.
- The lowest drawn toe, standing, setting off, walking and running at 0.5
  and 1.2 m/s and stopping, on a bird, a crow, a songbird, a bat and two
  crosses: never more than 0.1 mm under the ground (before: bird 6 mm, bat
  14, bat-glider 16, bird-led bird-bat 10 in stride and 42 setting off).

**Fixed beside it:**

- **The idle's breath and fidget leave a wing alone**: a limb that bears no
  weight is an arm only on a body with hands, the rule `hang_arms` keeps
  since W1; the breath still turned a bird's marker shoulders.
- **A bird's tail is one chain** (#425): `Rig::tail` is the tail zone's own
  joints, never the wings' fan edges attached after the stub, and every
  reader of a tail as a chain reads it - the paddle and the swish, a clip's
  shared turn, the rest, the idle's wag and the coat painter's tip.
- **A bird steps as evenly over uneven frames as over even ones**: its
  foot's peel was solved in a beast's pantograph (`gait::striding_of`)
  while the rest of its stride stood the foot, so its knee swung 52 mm out
  of its own bend and back in one frame as each heel lifted; and a blend
  read the velocity of the last two drawn poses over this frame rather than
  the one between them (`Driver::drawn_over`), three times too fast where a
  short frame followed a long one. Both showed only over uneven steps
  (`tests/winged_life.rs`, `winged_motion`).
- **A glider's paw stays on the ground as its pace changes**: a blend
  rolled its forepaw and put its sole 6 to 13 mm under with its pad on the
  ground, as a beast's paw (a dog's 44 mm, left to it). After the blend
  (`winged::ground::keep_toes_up`) a winged foot coming down is lifted by
  as far as its pad, its sole's ring or its second node stands under, past
  2 mm, in the plane its leg is bent in, taken in over its last 2 cm. A
  toe's turn is sought from where it was carried and eases to its reach as
  its root runs out, never jumping across the pad.
- **A small four-legged body's paws turn no faster than a beast's**: a
  glider folds its paws in the swing as a dog does, over a swing a third
  as long, and bounding at 3 m/s turned a forepaw 2.8 rad in a frame (a
  dog 1.3). A winged body's swing fold is now the share of a beast's its
  swing's length is of 0.4 s (`winged::ground::fold_share`); the ground
  gaits turn no joint more than 0.8 rad a sixtieth at 60 frames a second
  and 0.95 over uneven steps, held to 1.25 in `tests/winged_life.rs`.
- **A bird's folded wings ride no lower than they stand**: a walk's or a
  run's crouch and lean carried them down, a Bird x Bat cross at 0.49 by
  14 mm at 3 m/s. Each wing is turned up about its root by as much as its
  outer joints stand under their height at rest (`hold_wings`); 5.6 mm of
  the cards' drop is left, under the ground bound. (Until #440 that cross
  stood its folded secondaries 83 mm under the ground at rest, its build's,
  and every bound read its wing from where it stood: see "How a feather is
  built" and the next two.)
- **Nothing drawn passes into the ground** (#433, #440): a ground bound
  reads everything drawn, not the toes alone (`tests/winged_motion.rs`),
  standing and walking, at the fast gait and stopping, nothing under 6 mm,
  measured from the ground itself, on every family, a crow, a songbird, each
  pair crossed at 0.49, 0.5 and 0.51 with the neighbours of the two that
  failed, and three rolls. It found a bat's thumb, aimed from the wrist at
  the hand's node on the ground and longer than that reach, running on
  14.5 mm into it (a bat-led roll's 13): the thumb now roots and runs no
  lower than its own tube over the floor (`wing::membrane`). And the two
  crosses #433 left out (#440): a Bird x Bat cross at 0.49's folded wing,
  83 mm under, was its cards' laying (above), and a Bat x Glider cross at
  0.49's tail, 21.6 mm into the ground as its back first gathered into a
  bound, had nothing to hold it up (`keep_clear`, below; 4.0 mm now, 2.9 of
  them the membrane's edge by its thumb as it is built). A bird's leg skin,
  which W5's critic read 5 to 16 mm under at rest, stands on the floor on
  0.23.1.
- **A folded wing off the ground showed what its motion did** (#440): with
  that cross's cards 83 mm under at rest, the flight's, the rests' and the
  whole life's bounds, which read from where a body stood, had passed three
  things. A takeoff's first beats put its primaries 25 mm into the ground
  (a cross at 0.4's 12): the flight's raise reckons a feather outward from
  the shoulder, where a wing bent at the wrist trails them lower, and now
  reads the cards themselves (`wing::vanes`, each flight feather read back
  off the rig from its joint's drive and the feather tables, to half a
  millimetre of the mesh). A wing stretch laid its primaries 19 to 86 mm
  into the ground wherever the fold's height let it lie down, and is raised
  until none passes into it; and the stretch's leg, its foot carried on a
  shank stretched back, pointed its claws 14 mm into it (11 at 0.4, 7 at
  0.3, none on a bird), and is lifted by as far as they stand in it
  (`rests::stretch_raise`, `stretch_lift`).
- **A body on four legs is kept out of the ground it is handed** (#441):
  three things a slope showed, each kept out of it by `ground::keep_clear`
  on the pose handed back - a function of that pose and the ground under
  the part, nothing kept between frames, so on ground all but level it does
  all but nothing (at one part in ten thousand it moves no joint a hundredth
  of a millimetre, where the gaits' own reading of that ground moves one up
  to 0.5 mm) and on the level a hand is left to the bit.
  **A crawling body's hands are laid along the ground**: a bat's wrist
  stands upright whatever it stands on and its thumb is built along a level
  floor ahead of it, so on a 20 degree slope faced up it ran 37 mm into the
  ground, and the membrane's edge beside the wrist 17 mm into ground rising
  to either side. Each wrist is turned by the ground's own tilt about its
  pad, which stays where and as the footing laid it, and carries the thumb,
  the membrane's edge and the folded digits with it; whole within a leg's
  reach of the ground and let go of over the next, so a wing in the air is
  the flight's. **The belly is lifted off it**: a stride sinks the trunk
  for its legs' reach, up a slope the more, and a bat's belly stands 32 mm
  over the ground where a beast's stands a leg's length: bounding up the
  slope it went 19 mm in, a bat-led cross with a bird's keel 22. The body is
  lifted by as far as its trunk's skin (the rig's outline) stands in the
  ground and every leg solved back to its foot; a trunk whose every joint
  stands more than its ring over the ground is not read (a bat standing on
  the level paid 25 microseconds a frame for nothing). **On the ground a
  game hands it**: the hand's slope and the belly's are each the plane
  through the ground round it (`plane_under`: one and a half hips either
  side of a pad, half the trunk either side of its middle), not the normal
  at one point, and each axis of it takes the gentler of its two sides:
  read across both at once, a hand standing on the flat beside a step -
  which a grid hands over as a ramp one cell wide - read half the ramp, and
  a bat beside a 20 cm step laid its hand's edge 14.8 mm into the flat it
  cleared without this (`keep_clear_lays_no_hand_into_the_flat_beside_a_step`).
  Overlands hands a body on four heights a quarter metre
  apart, bilinear between them, whose tilt jumps at every cell's edge, and
  read at the pad a bat's hand turned 7.3, 1.5 and 7.0 degrees in single
  frames as its pad crossed one, everything it carries swinging 45 mm and
  back, every stride: over that ground with centimetre bumps and a 10 cm
  crest, a joint moved up to 121 mm a sixtieth further walking than without
  it, and now no more than 10.7 (`winged::keep_clear_tests`). **And the
  tail**: turned up
  about its root by as much as its lowest joint stands in the ground, its
  ring's depth kept over it - a Bat x Glider cross's went 53 mm in bounding
  up the slope. Up, down and across it both ways, at 30, 60 and 144 frames
  a second, a bat, a glider and every cross one of them leads now stand and
  go within 5.0 mm of it (a bat 0.8). **Turned on the pose handed back
  alone**: a hand turned on the pose kept to blend from was turned twice
  through every change of pace, 38 degrees on a slope of 20. **A bird on a
  slope is not held to it** - it stands on a level floor in Overlands - and
  walking down one still draws 11 to 13 mm into it (#443, #444).
- **A bound steps without a pop wherever its chassis is** (#437): a host
  that holds its chassis still while its velocity still asks for a pace - a
  wall in the way, or a host that hands the driver no position - leaves each
  planted foot at the foothold ledger's yield, 0.15 of its reach ahead of
  the stride's answer, and the ledger let it go at once as it lifted: a
  bat's wrist jumped 98 mm in one frame at every rate and its wing arm 115,
  its hind feet 65, a glider's 40 at 144 frames a second. On a winged body
  the gap a foot lifts with is carried into its swing: the swing sets off
  from where the foot stood and lets the gap go by the flight's own
  smoothstep (`gait::seated_offset`), so it lands where the stride puts it; a
  person and a beast let go at once, as they always have. **The take-off
  end is moved, not the path**, so the swing's height is read off the
  ground it really crosses: carried along the level alone, its height still
  came off the ground under the path the stride would have swung, and on a
  20 degree slope a held foot dropped or rose by the gap times the grade as
  it lifted - a held bat facing down the slope 112.7 mm a sixtieth at 144
  frames a second, eight frames popping, a bat x glider 266.8 - where now
  they move 35.7 and 69.3 and nothing pops, up, down or across it. Carried
  along, a bound was smooth before and is unchanged. **What else moved**:
  a turn on the spot holds its feet as a held chassis does, so a pivot
  carries the gap too, and moves 12 to 36 mm off where it went for every
  winged body - stiller, a bird's at 144 frames a second 26.1 mm a
  sixtieth at the most where it moved 52.5 and popped twice.
- **A foot stopping on a slope lays its toes on it, and does not flip
  them** (#436): after the blend (`keep_toes_up`) a foot whose sole a blend
  left in the ground had its toes turned about the pad from the blend's
  attitude, and where no turn could bring the sole out - one side of its
  ring rises as the other sinks - laid square to the ground instead, so a
  foot stopping on a slope went from one to the other and back, a bat's
  hind toe 24 mm a sixtieth facing up a 20 degree slope at 144 frames a
  second, a bird x bat cross's 43 across it. That pass has no state and no
  time, so it cannot choose between the two smoothly: a sole is now let
  stand in the ground after a blend as far as the pad pass leaves it (2 mm),
  so it turns a toe only when a toe itself is in the ground, and **the idle
  lays its own feet on the ground** as a stance does, square to it and the
  lowest point on it (`stand_toes`), each by as much as it is down, a foot
  the idle steps let go of as it lifts. Laid in the idle's own pose, before
  the blend, a stop or a landing's hand back eases the toes onto the slope
  with every other joint, and they lie on it for as long as the body
  stands: within 0.5 mm of their height over a 20 degree slope a second and
  a half after a stop, up, down or across, where letting the sole stand in
  without laying them left them level with the body, 6 to 38 mm off it.
  The feet are held to the landing's hand back pop bound with every other
  joint. **What else moved**: on the level a body standing from rest moves
  nothing (0.01 mm), and one stopping moves its toes up to 5 mm while the
  idle steps its feet home over the second after the stop, laid as they
  set down, and stands as it stood before.
- **A pivot's clock goes the same at any frame rate** (#415): a pivot's
  speed is a standstill's turned, which knows no pattern, so its bare duty
  was a person's while a four-legged gait stepped at its pattern's, and
  mapped from one onto the other every frame the turn eased, the clock
  gained a twentieth of its stance a frame - two and a half times as much a
  second at 144 frames a second as at 60, a horse's feet 605 mm apart
  between the two. The gain is now held to what it is a second at 60
  (`driver::REFERENCE_RATE`), phase by phase - a stance's progress and a
  swing's remainder each scaled by its own share (`driver::carry_scaled`) -
  so a pivot at 60 is the one it was to the bit, and at 90, 144 and 240
  frames a second and at uneven steps a turning body's clock steps within a
  twentieth of a cycle of 60's while its turn eases. Mapping from the duty
  the gait steps at instead drops the gain altogether, and moves every
  beast's pivot at 60 too; that is the owner's call.
- **A wrist is eased short of its leg's fold** (#445): a two-bone leg has
  a square-root singularity folded shut as it has at full reach - its wrist
  no nearer its shoulder than the difference of its bones, where the elbow
  swings through the last of its angle in a millimetre of the wrist's - and
  `ik::two_bone` keeps that end hard, a beast's leg standing nearly straight
  and never coming there. A glider-led cross with a bat's share stands a
  glider's stance on a bat's long arm bones: a Bat x Glider at 0.5 has an
  upper arm of 181 mm and a forearm of 253, its wrist 124 mm from its
  shoulder at rest and shut at 72. On a 20 degree slope its stance ends at
  104 and the swing's lift took the wrist under 80 and past shut, where the
  stride's solve, the swing's re-aim and the fold each landed somewhere
  else: walking down the slope carried, its elbow moved 84 mm a sixtieth at
  144 frames a second and went 47 back the next frame (the same joint the
  other way), and held walking up it 83 and 72 back; no pole flipped. On a
  winged body the stride's wrist is now eased over the last two fifths of
  that difference (`quadruped::clear_of_fold`, `ik`'s soft reach turned
  round, C1), in the pantograph's own reckoning (`stand`), the leg's solve
  (`ground::solve_leg`, a stride's only) and the swing's fold
  (`articulate`) alike: the same walks move a joint 60 and 35 mm a sixtieth
  at the most and go 10 and 6 back. **What else moved**: only bodies whose
  stride comes within the zone - that cross on a slope (its elbow up to 58
  mm from where it was thrown, a planted pad 0.3 mm, nothing lower) and by
  0.3 mm carried on the level (1.5 held); a glider, a Glider x Bird cross
  and every bat-led body stride as they did, to the bit or within 0.05 mm.
  Held to it by `tests/winged_steps.rs`, which also bounds a joint going
  back along the way it had just come (a zig-zag): the pop bound did not
  see one.
- **A turn on the spot steps without a pop** (#446): three things jumped,
  each its own (`tests/winged_steps.rs`, a pivot at a radian a second and a
  spin at two and a half, left and right, at 30, 60 and 144 frames a
  second). **A bat-led body's spin changed gait between two frames**: a
  standstill turned knows no pattern, so its gait was the one its working
  foot's speed picks, and a bat's wrists stand so far out that a spin takes
  them past the speed it bounds from - a walk to a bound as the turn eased
  up, unblended, a bat's head 44 mm in the frame, and back as it eased down,
  a forefoot 95 (106.8 and 227.1 mm a sixtieth at 144 frames a second). One
  foot going faster does not change the gait a body is in (#370): from
  where its working foot would bound, a winged body's standstill is turned
  in its walk's pattern (`Driver::drive`), and below that it is the pivot it
  was, to the bit. **A glider-led body's paws folded whole in every
  pivot**: a stride that only turns has no length to read a swing's time
  off, so `fold_share` gave a beast's whole fold to a swing of a twelfth of
  a second, a forepaw turning 1.8 to 2.0 rad a sixtieth at 144 frames a
  second and 1.4 to 1.7 at 60. A turning stride's swing is now timed by its
  working foot, as its cadence is, and folds half of that for as much of it
  as is its turn (`PIVOT_FOLD`: a pivot's clock gains on its cadence while
  its turn eases, #415, and its swing is over in half the time): 0.2 to 0.42
  rad a sixtieth. **A bat's hind toes were thrown out of the ground in one
  frame** as a turn died away: `lift_from` answered with no turn until one
  first brought the toes out, then with that turn, an eighth of a radian
  off - a rolled bat's toe tips 28 mm a sixtieth against 10 at 144 frames a
  second; it now takes up the turn that brings them nearest out as that
  comes within 2 mm of out. **What else moved**: a glider-led body's
  forepaw folds less in every pivot (its toes up to 72 mm from where they
  folded to) and by 2 mm through a curve walked at 0.8 rad/s; a bat-led
  body's spin past the bound's speed is a walk; a bat-led body's hind toe
  moves 1 to 2 mm in a frame or a few of a stride, and up to 15 as a pivot
  ends; a bird and a bird-led cross move nothing.
- **A turn on a slope seats its swing for the ground it crosses** (#446,
  after it). A swing is seated over the highest ground within reach of its
  sole along the flight (`gait::seated_offset`'s roof), and that reach was
  read along the stride's direction, which a pivot's feet swing across: a
  pivot's every probe lay beside the sole, so the landing's ground held the
  foot up from its take-off. On a 20 degree slope across the turn a
  downhill forepaw sweeping in under the chest uphill rose some 35 mm over
  its ramp; a Glider x Bird cross's wrist came within 6.5 mm of its
  shoulder, where its arm (bones 100 and 113 mm, folded to 12.6) has its
  elbow pointing back against the wrist's own direction, and as the wrist
  passed the shoulder from outside to inside the elbow was thrown 116 mm
  out in one frame: spinning, 337.8 mm a sixtieth at 144 frames a second, a
  joint turned 4.12 rad (108.6 mm and 1.48 rad at 60). A Bat x Glider
  cross pivoting popped its elbow at 60 (49.3 mm a sixtieth), and a bat
  spinning went back 0.13 to 0.14 of its trunk at 30 and 60. For as much of
  a winged body's stride as is its turn (`winged::ground::turned_share`)
  the roof now looks along the swing's own flight, take-off to landing; a
  stride that does not turn, and every person's and beast's, is seated as
  it was, to the bit, and on the level nothing changes at all. The same
  spin moves 37.1 mm a sixtieth at 144 and turns no joint more than 0.42
  rad, as on the level; no body of four pops turning up, down or across
  the slope at 30, 60 or 144 frames a second (`tests/winged_steps.rs`).
  **What else moved**: on a slope only, a winged body's curve and pivot -
  motiondump's slope scenario (15 degrees ahead, 10 across) moves a bat's,
  a glider's and a Bat x Glider's joints up to 16, 9 and 11 mm through its
  curve and pivot, a bird's 5 and a Bird x Bat's 2 in its pivot.


### In the air (`anim::winged::flight`)

A winged body in the air is its own source, `Source::Fly`, driven by its
host frame by frame: a leap's flight is ballistic and over
in a second, a flier stays up for as long as its host flies it. Everything
here is on a body with wings (`Rig::wings`); a person or a beast handed a
flight moves exactly as without one.

Two strips show it. `render --winged --kin K --flight` drives a body through
the modes here, in this crate. `render --winged --kin K --poses FILE` (#448)
draws a host's own frames instead - what a flight in Overlands made of the
body, chassis and all - from the file Overlands' ignored probe
`probe_dump_a_flown_life` (src/player/rigged/beast_tests/flight.rs) writes:
one line per frame, `row col lift qx qy qz qw tx ty tz` and a rotation per
joint. Render a motion phase as strips, side on and ahead, before calling
it done: the winged epic's green tests let a downstroke through the ground,
takeoffs that did not read and a touchdown snap reach the renders.

#### What the host gives

`Inputs::flight: Option<Flight>`, `None` on the ground (the default, so every
caller from before is unchanged). A `Flight` is:

| Field | Type | What it is |
|---|---|---|
| `mode` | `FlightMode` | `Takeoff`, `Flap`, `Glide`, `Soar`, `Dive`, `Flare`, `Land` |
| `air` | `Option<Vec3>` | the air's velocity past the body, m/s; `None` reads `Inputs::velocity` |
| `phase` | `Option<f32>` | where the wings are in their beat, in beats (`0` the top of a downstroke); `None` lets the engine keep its own clock |
| `contact` | `Option<f32>` | seconds to what the body lands on; under `FLARE_TAU` (0.45 s) it flares |

`Inputs::turn` (the yaw rate) steers the tail as a rudder and lifts the outer
wing; a glider eases its inner membrane instead. `Driver::flight(rig)`
answers what the body was posed doing (`Flown`, `#[non_exhaustive]`: the
mode in effect, the beat's clock and rate, the drawn extension, the beat's
width, the flare's depth, how far the trunk is levelled, and three a host
hands on by: `lifted: bool`, the takeoff's push over and the feet off;
`settled: bool`, a landing down, absorbed, folded and, carrying itself, at
the ground; `height: f32`, the hover's height carrying itself, 0 carried).

**Handing on, with no durations guessed** (`anim::winged::flight`, "Handing
on"): a takeoff into flapping **two beats after `lifted`** - when `phase` has
grown by two from its value that frame, the takeoff's two big beats; in beats
it reads alike at every size, where a bird at size 2.4 lifts off at about
0.45 s and beats about once a second and a songbird at 0.1 beats ten times in
0.6 s. A landing back to no flight **once `settled`**. A landing asked of a
body on the ground is a landing from no height: its feet put down, its wings
folded, no absorb.

**The host carries the attitude.** A host writes the body's heading, pitch
and bank itself, so the pose is the body riding that attitude: its trunk
levelled along its own `+Z` (a perched bird stands 1.05 rad nose up and flies
level), its head held as it was by its neck, only the beat's small pitch and
a flare's last lift of the nose added. A body that carries itself
(`Carriage::Own`) hovers instead, risen by its takeoff and brought down by
its landing, on a critically damped spring stepped in closed form, so a frame
of any length lands where it should: to 0.8 of its height, or, where its
wings reach further, high enough that the lowest of a takeoff's beats clears
the ground by 0.15 of the wing's reach and 0.08 of its height (a bat, whose
wing reaches 0.75 m on a body 0.36 m tall, hovers at 0.82 m, not 0.29).

**No wing into the ground** (W5's look review): near the ground - a
takeoff's first beats, a landing's last, a host that carries a body low -
each wing's stroke is raised as far as keeps every joint of it (to the
digits' tips, a feather reckoned 2.6 humeri past its joint along the wing)
0.06 of the body's height over the ground, and every corner of every flight
feather's card out of it (`wing::vanes`, #440), posed again, and never raised
further than helps: a partial, raised stroke. A folded wing is the ground
posture's and is not raised, nor a wing the ground holds as a leg. Through a
takeoff and a landing the root is held up by whatever a foot would sink,
its sole reckoned under each joint and further as it tilts, and a hind foot
flattens to meet the ground as it nears.

#### From Overlands' `FlightMode`

Overlands (#1861, adopted in #1862) flies an arcade model whose modes the
engine mirrors:

| Overlands | `Inputs::flight` |
|---|---|
| `Ground` | `None` |
| `Takeoff` | `Takeoff` |
| `Flap` | `Flap`, `phase: beats + (since_beat / beat_period).min(1)` |
| `Glide` | `Glide` (`Soar` on rising air, which it does not yet fly) |
| `Dive` | `Dive` |
| `Flare` | `Flare`, `contact` from its approach's tau |
| `Land` | `Land` |

`air` is its chassis velocity in still air; the phase may jump by half a beat
where a press beats early, and the engine follows it rather than copying it:
its clock runs between half and twice its own rate to catch the host's, and
never backward.
On a peer, which knows only the mode, the phase and contact are `None` and
the engine keeps its own clock and flares on its own time.

#### How each mode looks, and what scales

Every number the pose reads - the wings' extension, the beat's width, the
dihedral and sweep, the tail's fan, the legs' tuck, the trunk's levelling,
the flare's depth - is eased toward its mode's target through a first-order
lag of its own, so no change of mode pops; the beat's clock never jumps.
The wings are posed through `Rig::stroke_wings(pose, left, right)` with a
`wing::Stroke` each - an extension, an elevation about the trunk's long axis,
a sweep about its up, and a reach that swings every limb a wing drives - and
the tail through `Rig::steer_tail(pose, fan, yaw, pitch)`. A stroke turns the
arm wholly, a membrane's spars by their root's place from the hip to the
shoulder, and the hind leg not at all.

- **Takeoff** (research-11 3): from the ground a crouch and a push, the
  legs a spring released over a quarter turn; the wings' first downstroke
  begins at 0.8 of the push, so the legs give `LEG_SHARE` (0.88) of the
  lift-off speed and the wings the rest; then beats a fifth faster and 1.3
  times as wide as a cruise's. **Each part long and deep enough to be seen**
  (the look review): a crouch of at least 0.18 s (outlasting the blend out of
  the ground) and 0.2 of the leg, a push of at least 0.1 s, so a body carrying
  itself lifts off at about 0.27 s. The wings stay folded through the crouch
  and open in the push - a bird's from its start, leaning a third of the way
  level into it so its half-open wings are carried clear; a bat's and a
  glider's, whose wings are the legs they push with, over its last 0.4. A
  host that carries the body has already launched it, so its takeoff starts
  crouched; one that carries itself crouches first. The crouch takes at most
  half of what each leg it stands on can still fold (a glider, which stands
  crouched, never turns a knee over its pole) and 1.2 times its belly's
  height over the ground, reckoned at its trunk's joints; and, on a body whose
  wings are no legs, no more than its folded wing's lowest drawn point stands
  off it (`Wings::low`, the whole of it and not 1.2 times since #440: a Bird
  x Bat cross at 0.45 crouched its feathers 8.5 mm into the ground). A
  landing asked mid-push absorbs from the crouch it finds. Off the ground the
  trunk levels on a critically damped spring, from rest.
- **Flap** (research-11 4): the beat's rate `m^-0.28`, its trunk cubed - 3.0 a
  second at the default bird's trunk (0.311 m), 2.6 at the bat's (0.269), as
  the trunk to the -0.84 from there, the consuming application's own law; a
  half-width of 0.80 rad (bird) or 0.62 (bat); a downstroke of 0.56 or 0.50
  of the beat; the upstroke folding the wing by 0.25 or 0.38 of its extension
  at its middle - a bird's flexed wing, a bat's strong flex; the tip swept
  forward on the downstroke and back on the upstroke. The body heaves up
  through the downstroke and pitches nose up, by `1 - speed / cruise` (cruise
  6.5 m/s at the reference): a slow beat bobs, a cruise does not. A glider
  never flaps, nor a cross a glider leads (`Kinship::flaps`): asked to, it
  glides.
- **Glide**: spread, still, the dihedral 0.10 (bird), 0.04 (bat), 0.0 (the
  glider's flat X); a bird's tail fanned a third. **Soar**: a little more
  dihedral, the tail fanned to circle.
- **Dive**: the wings half folded and swept back, a stoop - 0.3 of the
  extension on a bird, 0.5 on a bat and 0.4 on a glider, whose halfway pose
  is partway to the crawl's fold.
- **Flare** (research-11 6.1, 8.1): by time to contact - from 0.45 s, full by
  0.1 - whatever mode the host is in, and deeper the nearer: the wings
  forward and raised, braking beats 1.25 times as wide, the tail fanned and
  depressed, a bird's legs swung forward and its trunk raised back toward its
  perch by 0.45 of its rest pitch over the host's own (near vertical), a bat's
  and a glider's nose lifted 0.25 rad. Without a contact, over 0.35 s.
- **Land**: the legs absorb, a dip of a quarter of the leg over its half
  period - held, as a takeoff's crouch is, to what the legs fold and the
  belly clears, and on a body standing on its wings to as far as they are
  folded - the feet put on the ground over 0.2 s, a wing's own paw levelled
  as far as the plant does not yet hold it; the
  wings fold, the trunk returns to its ground posture, the tail closes. The
  absorb starts at touchdown: at once on a carried body, and on one carrying
  itself once its hover has brought it down (aiming 0.05 of its height under
  the ground, so it arrives), held spread with its legs reaching until then.
  A wing's own limb - a bat's wrist, a glider's every leg - is put down from
  its fold rather than solved where it stands half spread, and a wing opens
  or folds in no less than a sixth of a second - **in a landing, no less
  than half a second**, the feet put down over a fifth (the look review: a
  glider's spread legs swung down and planted in two frames, its patagium
  flipping from a sheet to a hanging panel with them); a body standing on
  its wings gives up their raise off the ground as its feet go down. The ground gait or the
  idle takes over, blended, the frame the host gives no flight (the order
  above).
  **A landing lets its beat go** (#435): what the beat adds as the landing
  begins - its elevation, sweep and pitch, and its upstroke's fold - is held
  and let go of from rest on a critically damped spring (10 a second), kept
  off the ground for as long as it lasts, its heave at the rate the beat's
  width used to ease out at, and the beat's width set to nothing; eased out
  with the clock still running, a bat's braking strokes went on through its
  landing's first frames, its wingtip 150 to 300 mm a frame at 60 a second
  (Overlands' end review). And a raise that holds a wing off the ground is
  let go of at no more than 4 radians a second as the ground stops needing
  it, the wing leaving the ground a little ahead of its stroke: let go in a
  frame, a cross's wingtip went from 40 mm a frame to 203 as a flare's last
  upstroke left the ground. **On sloped ground
  it settles into the ground's own stance** (#435): its trunk rides the
  ground under its girdles as the ground sources do, coming onto it over
  0.25 s as its feet go down (and off it as a push leaves it), each foot is
  stood back over where it rests and its pelvis let drop as the idle's is,
  its toes are laid on the slope, and it is held up by its pads along the
  ground's up rather than by its whole foot reckoned against the level.
  Settled level, a landing on a 20 degree slope stood up to 0.31 m off what
  the idle draws there, and the hand back moved a bat 82 mm in a frame
  (Overlands W6.4). On the level all of it is as it was, to the bit. A
  landing has settled once its beat has gone as well as its wings folded.
- **Takeoff from a landing** (#435): a takeoff asked of a landing down on
  its feet crouches and pushes as one from standing does, crouching first
  (as a body carrying itself does) from wherever the landing has its root;
  before, it never crouched or pushed, its root rising a few millimetres
  where a takeoff from standing dips 17 to 22.

The beat's rate, width, downstroke, upstroke fold, sweep, dihedral, heave and
pitch blend across a cross by its families' shares (`Kinship`); whether it
beats, and whether its wings rise in its push, do not: they are its leading
family's (`Kinship::leading`), as its surface is, so a cross at an even blend
beats exactly when it wears a beater's surface.

### At rest: rests and emotes (`anim::winged::rests`)

The owner's order holds: "landing on the tops of things first, hanging and
clinging later", so a bird perches and roosts on whatever it stands on, a
bat rests on the ground and a glider curls there; the bat's hang and the
glider's cling wait for their phase. Everything is in `anim::winged::rests`, written
as goal-space clips as the person's and the beast's rosters are, with five
targets of a winged body's own, each doing nothing on a body without wings:

| Target | Key | What it moves |
|---|---|---|
| `Wing(limb)` | `x` extension 0-1, `y` raise, `z` sweep (quarter turns) | one wing off its fold (`Wings::pose_side`), then turned whole at the shoulder; out of what the pose held over its first tenth, since a bat's wing is the leg the plant put down |
| `Fan` | `z` 0-1 | a bird's tail fan (`Rig::spread_tail`) |
| `Grip` | `z` 0-1 | every drawn toe: the joints past its root bent 2 rad in all toward the sole, and the root lifted as a knuckle just as far as brings the claw's tip back to the height it stood at (found by halving, at most 1.4 rad), so the claws close on a flat top without going through it |
| `Tuck` | `z` share of 0.8 of a half turn, `x` side, `y` extra dip | the head turned back about the body's up, half at the neck and half at the head, a bird's neck leaned back 0.7 rad first and the head dipped 0.8 rad past the lean |
| `Curl` | `z` 0-1 | a four-legged winged body down on its belly (its trunk's skin, read off the rig's outline as a beast's lying down reads it, 4 mm off the ground), legs folded under - hind feet laid, forefeet stood on their pads, each foot raised by whatever its shin and pad sink - a bat's wrists drawn in by its bat share, the spine bent 2.3 rad round, the tail laid along the ground at its ring's depth wrapped 3.4 rad round the outside and the head turned on and dipped, by its glider share; then, in four passes always taken, lifted by whatever of its skin is still under the ground: the outline but the feet, shins and tail, every upper limb's ring, and a membrane's trunk-rooted spars read 30 mm deep |

`Roll` now rolls a winged body on two legs too, about its trunk's own line
(a bird carries its trunk raised); a person is refused it as before.

**Rests** are states, gone into at the body's own pace, held, and got up
from by the way down played back: the driver plays them as it plays a
beast's sit, since `emote::seconds` and `emote::getting_up` answer a winged
body's rests.

| Family | Rest | Pose (10 6.2) | Down, up (s, leg 0.25 m) |
|---|---|---|---|
| Bird | **Perch** | crouched 0.22 of its leg's reach on planted feet (no further than its folded wing stands off the ground, `Wings::low`), the toes gripping, the head level and still glancing about: "the belly feathers settle over the feet" | 0.6, 0.4 |
| Bird | **Roost** | down 0.36 (limited as the perch) so the tarsi fold under and it "looks legless", gripping, the head turned back over the left shoulder and laid into the scapulars, the lids shut | 1.6, 0.9 |
| Bat | **Rest** | the curl with no curl: on its belly, wrists drawn 45% in toward the shoulders, head down 0.5 rad, the lids shut | 1.2, 0.8 |
| Glider | **Curl** | curled round as low as its folded patagium lets it, the tail wrapped about it, the lids shut | 2.2, 1.6 |

A bat-glider cross offers its leading family's name for the one motion, and
lies between the two by its glider share: the curl, the drawn-in wrists and
the times each the share of the family they are (`tests/winged_rests.rs`
measures the spine's bend rising with the share). A bird-led cross perches
and roosts as a bird; a bat- or glider-led one rests as its family. **A
sleeping body shuts its eyes**: the driver closes the lids by how far into a
roost, a rest or a curl the body is, over the blink.

**Emotes** start and end at rest: **Preen** (a bird's) and **Groom** (a
bat's or a glider's), the head turned 0.8 of the tuck (a bat's or a
glider's 0.62, level, all four feet down) round to the left wing, a bird's
lifted a little to meet it, in four nibbles; a bird's **Wing Stretch**, the
left wing spread 0.75 and laid down and back (lifted instead where the
folded wing stands within 12 cm of the ground, and raised past that until
none of its flight feathers passes into it, #440) with the left leg lifted
and stretched back under it, inside its reach (and lifted further by as far
as its claws would stand in the ground), and the tail fanned, then both
wings raised 0.75 of a quarter turn over the back (S16); a bat's **Wing
Stretch**, each wing opened 0.45 out and lifted in turn; a glider's
**Stretch**, a bow of 0.1 with its forepaws walked 0.22 out and stood on the
ground, its head up and its tail raised high; **Shake**, a bird's trunk
rolled 0.18 of a quarter turn side to side with its head canting first, then
its wings, lifted as they open, and fan flicked, a bat's or a glider's risen
0.05 on its legs and shaken as a dog's in three slower swings - the trunk
rolled 0.2, the head whipped side to side ahead of it - with all four feet
held, its wings - the forelegs it stands on - still; and **Head Tilt**, the
head canted 0.3 and swayed side to side and up to look with one eye (an owl
judging depth, 10 6.2), a bat's and a glider's ears perked besides.

**Each emote reads at a glance** (the look review): at its widest some
joint of the body's own - not a tail, an ear or a feather - moves at least a
tenth of the body's height from where it stood (`tests/winged_rests.rs`).
Before, a glider's head tilt moved its body 1.6% of its height, its shake
5.6% and its stretch 8.2%, a bat's head tilt 1.4%, a bird's 5.2%; now
16.8%, 16.1%, 16.7%, 26.5% and 10.3%.

**The roster is the body's**: `gesture::roster(rig)` is what a host offers
- a bird's, a bat's or a glider's by its leading family, a beast's on four
legs, a person's otherwise - and `gesture::for_body` answers a winged body
only its own names, so a bird is never laid a person's wave over its folded
wings and a bat never asked a beast's sit. The roster, the curl's share and
the times read the body's `Kinship` (above); how low a perch may crouch
reads `Wings::low`, how far the folded wing stands off the ground, written
when the wings are grown - `None` on a rig from an older build, whose rests
then go unlimited.

**A bird looks with its head** (10 fact 5, 6.1, S12). Its eyes turn
`face::eye::winged_range`: an owl's 3 degrees with its full disc, a bird's
with no disc 10 (a peafowl's habit between saccades, not a starling's 32 at
most), toward a beast's 30 by how little of a cross is bird; a bat's and a
glider's are a beast's. The range rides on `Eyes::range`, `None` and off the
wire on every person and beast, set at the build from the plan. The driver's
idle glance turns a bird's head up to 1.5 rad on its neck and head, not its
chest, and makes the turn in the first 30% of the glance's ramp and holds
it: "a series of quick head saccades, each one a held pose".

**No person or beast is offered or moved by any of it.** Every new target is refused on a body
without wings, every new name answered only on one, and the eye range and
the bird's glance apply to a winged body alone. A scratch probe drove four
persons and six beasts through every name of both rosters - each clip over
21 times, its getting up, and 420 driver frames asked and told to get up -
and through two gaze configurations at four targets with their eyes, and
twenty seconds of idle glances, on the 0.22.0 tree and on this one: every
line identical, to the bit.

**Held to the ground and to a frame's travel** (#424, the critic's
review): `tests/winged_rests.rs` drives every name of every family's
roster, on the three families, each pair crossed at 0.49, 0.5 and 0.51 and
ten rolls, at 25 moments of its clip and through the driver from asking to
standing again: the whole drawn body - skin, toes, membrane and feathers -
never more than 6 mm under the ground beyond what stood under it before,
and no joint travelling more than 67 mm in a sixtieth of a second. Before
the fixes it found a bat's shake driving its wrists 116 mm in, a glider's
bow its paws 78 mm, a glider's curl its thighs 51 mm and its patagium 20 mm,
a bird-bat cross's roost its wing 197 mm, and a bird's stretched leg
turning over, 283 mm in one frame.

### Where the motion departs, and why

**On the ground**

- **The default owl does not bob**: research-11 lists owls and parrots
  among the birds that walk without it, so the bob follows the beak. One
  constant (`BOB_HOOK`) gives every bird some.
- **A hop is low**: thrown for its time in the air, a 0.4 m bird's hop
  rises a few centimetres, where the dossier's rule of thumb (L, no measured
  figure) has half a body length.
- **The bound's duty and footfalls are chosen**: research-11 has none.
- **A bat's hind feet lift little** (5 mm at 1 m/s): the beast's hind lift
  on a short, splayed leg. A crawl reads as one.
- **The toes do not curl in the air**: a swing carries them as the pad
  holds them.

**In the air**

- **A bird's legs trail back**, drawn up toward the hip, rather than tucked
  forward under the belly as an owl carries them: levelled from its perch,
  the leg hangs back, and a tuck forward would need a hip turn the bridge leg
  does not have.
- **The upstroke folds along the extension's own arcs**: a wing at 0.7 of
  its extension is partway back toward its fold, not a flexed wing held out,
  so a deep fold (a bat's at 0.6 read crumpled) is held to 0.38.
- **No tip reversal, no backward flick, no flap-bounding**: one stroke shape,
  its width and fold blended; the bat's flick and a small bird's bound are
  left for a later pass.
- **A takeoff's leg share is kinematic**: the share of the lift-off speed
  gained before the first downstroke begins, which is what a pose can show;
  research-11's figure is a share of the impulse.
- **The tail steers by yaw only**, and the rudder turns the fan's edges
  alone, not the stub: written before #425 made the stub one chain
  (`Rig::tail`), which a later pass can pitch.

**At rest**

- **A glider lies no lower than its folded patagium**, which the curl reads
  off the joints rooted in its trunk, 30 mm under each (`SPAR_DEPTH`, a
  bat's too). Since #434 the fold is a band under a seam along the flank, so
  that read no longer holds the curl up as W2's fold to within a few
  centimetres of the ground did; the trunk's outline and its folded thighs
  hold it. How low a glider should lie is the owner's to say.
- **A bat's and a glider's wings stay down in their emotes**: they are the
  forelegs the body stands on, so the shake does not flick them, the groom
  does not lift them, and the glider's bow does not open its patagium.
- **No fluffing.** Research-10's fluffed outline wants the contour feathers
  raised off the body, which no pose can do: they are cards bound to the
  trunk's joints. The shake flicks the wings and fan instead.
- **On level ground**: a winged rest is posed against the level its body
  stands on, as a beast's play bow is, not fitted to the ground under each
  support as a beast's sit is (#381); on a slope its feet, belly and tail
  rest on the level through it, and what keeps a standing or a going body
  out of a slope (`keep_clear`, #441) leaves a rest's frames alone, eased
  out as it goes down and in as it gets up; an emote, which keeps the
  stance, keeps it too. A bat's forearm reaches into a 20 degree slope it
  rests facing up, on the tree #441 landed on and since.
- **The roost's tuck is over the left shoulder always**, and the preen
  reaches the left wing: one side reads, and a mirrored clip is one sign.
- **A perch's grip is on a flat top**: the toes close as a fist on a table
  does, the knuckles up, every claw off it. A branch's grip, which wraps
  the toes round something under them, wants the perch's own shape, which
  the engine is not given.
- **Times are chosen**: research-10 and research-11 time none of the rests.
- **A bird's head turns no further than 1.5 rad on a glance**: an owl's
  "up to 270 degrees" is the neck's reach, not a glance's, and `look_at`
  clamps about the rest facing.

## Left for later phases, and for the owner

- **W2 (#421)**: built - see "The wings". For the owner: the bat's cape
  from its raised elbow to the leg at rest, the bird's folded plate and how
  far its primaries reach, the feather count, the glider's resting fold.
- **W3 (#422)**: the head and the feet built - see "The head" and "The
  feet". For the owner: the reach of the beak's axes, the disc's depth and
  its ruff's shade, the beak's horn, the bat's and the glider's eyes and
  ears; both bird arrangements, the bat's foot turned with its leg, the
  glider's hallux.
- **W4 (#423)**: plumage, coats, the wire and the eyes built - see
  "Surfaces" - and the cards, the marks and the membrane - see "Cards, marks
  and the membrane". For the owner: each kind's palettes and how often a roll
  picks each, the disc's face and ruff, the crow's sheen, the bat's and the
  glider's default coats; the contour feathers' size and lift, the mantle's
  gold, the glider's stripes, the membrane's lit edge. At the review the
  owner raised the ceiling to 27,200, chose a mid grey glider and eyes by
  the kind of bird.
- **W5 (#424)**: the motion built - see "Motion": the ground gaits per
  posture, the flight source, the rests and emotes. For the owner: whether
  the default owl should bob after all, the hop's height, the bat's crawl
  and bound, the glider's bound; each flight mode's look per family (the
  strips `render --winged --kin K --flight`), the beat's width and upstroke
  fold, the bird's trailing legs, the dive's half fold; the perch's crouch
  and grip, the roost's tuck, the bat's drawn-in wrists, the glider's curl,
  each emote's reach; the hang and the cling (the later phase); a glider
  from an older build read as a bat; the glider's forepaw at a change of
  pace and the bird's and glider-led cross's thigh over uneven steps; how
  little a glider's paws now fold stepping round on the spot (#446).
