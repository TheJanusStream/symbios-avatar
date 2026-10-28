# Quadruped postures and transitions: sit, lie down, stand up (dog, cat, horse)

*A point-in-time research dossier gathered for the quadruped refinement (#372, R0 #373, 2026-10-28). Kept as the record of what was found and why the refinement's decisions were made. Every figure carries its own confidence; a value recalled from a paywalled paper, or derived here, is marked so and needs checking before anything hard-codes it.*

Scope: postures and transitions only (sit, sternal and lateral lying, getting down and up).
Morphology at rest and gaits are covered by the other two reports.

Conventions used throughout:

- **Joint angle = included angle** between the two segments, in degrees, as veterinary
  goniometry measures it: 180 deg = segment line straight, smaller = more flexed. (The
  carpus can exceed 180 deg: dorsiflexion.) Where a source uses a different convention
  (flexion from neutral, "+ extension / - flexion"), it is said so.
- **RoM** = range of motion (degrees travelled) during a movement, not a joint limit.
- **H_w** = standing withers height of the same animal. Heights are fractions of H_w.
- **BW** = body weight. **GRF** = ground reaction force.
- Every figure carries a tag: **[M]** measured in a cited study, **[R]** rule of thumb or
  description from a veterinary/ethology text without numbers, **[D]** derived here by
  geometry or arithmetic from other figures (the derivation is shown), plus a confidence
  **(high / med / low)**.
- Source keys in brackets, e.g. [Ellis2018], resolve in section 4.

What could and could not be read: the greyhound sit-to-stand paper [Ellis2018] was read in
full text. The beagle sit-to-stand/stand-to-sit kinematics [Yoshikawa2023], its EMG
sequel [Yoshikawa2025], the Labrador sit-to-stand validation [Feeney2007] and both
goniometry papers [Jaegger2002, Jaeger2007] are paywalled; only their abstracts were read,
and the Labrador RoMs are quoted second-hand from [Ellis2018]. No measured timing of a
horse lying down or getting up, and no kinematic study of a cat sitting or lying, was
found; those figures are rules of thumb and are marked so.

---

## 1. Summary: the ten facts that most change whether a sit or a lie-down reads as natural

1. **A dog's sit is made at the stifle and hock, not the hip.** Sit-to-stand hip RoM is about
   half of walking hip RoM (beagles), while stifle and tarsus RoMs exceed walking; Labradors:
   hip 35 deg, stifle 62 deg, tarsus 66 deg; greyhounds: tarsus 89 deg. [M, high] [Yoshikawa2023, Ellis2018 citing Feeney2007]
   -> the pelvis pitches down WITH the femur; the hip angle changes only 20-35 deg.
2. **A square sit needs near-full stifle flexion**; a dog that cannot fold the stifle sits
   with the leg kicked out sideways. [R, high] [Lafuente2024]
3. **The sit's weight sits on four things only:** both ischial tuberosities (buttock), both
   plantar metatarsi lying flat from hock point to toe, and the two forepaws. Nothing else
   touches. [R, high] [WikiVet; anatomy]
4. **The feet do not move during a sit-to-stand.** Greyhound trials were valid only when no
   foot stepped; the whole 0.64-1.94 s movement (mean 1.14 +/- 0.16 s) rotates the body over
   planted feet. [M, high] [Ellis2018]
5. **Rising unfolds proximal to distal:** hip extends at 0-20 % of the movement, stifle at
   10-50 %, hock at 25-60 %; hindlimb vertical force reaches about 0.75 BW by 25 %, forelimbs
   drop to about 0.25 BW. [M, high] [Ellis2018]
6. **Dogs and cats lie down through a sit, and stand up by reversing it;** carnivores
   typically extend the forelimbs first when rising (horses too), ruminants the hindlimbs.
   [R, med-high] [WikiVet; Nickel1986 via Ellis2018]
7. **A horse's sternal lying is asymmetric:** sternum and abdomen rest on the ground to one
   side of the midline, on the LATERAL surfaces of the flexed underneath limbs; the
   hindquarters roll onto one hip while the forehand stays upright; muzzle on the ground, on a
   forelimb, or held up. [R (ethogram definition), high] [Pedersen2004 via Burla2017]
8. **In horse lateral lying the upper forelimb lies in front of the lower one, the hind limbs
   are extended and the head rests on the ground.** [R (ethogram definition), high] [Pedersen2004 via Burla2017]
9. **A horse rises front first:** roll to sternal, gather all four limbs, extend both forelimbs
   forward in front of the chest, push the forehand up into a brief dog-sit, then throw the
   head and neck forward and down (the front end is about 20 % of BW) to unload and lift the
   hindquarters. [R, med-high] [WikiVet; EQUUS1999; Fuhs2021]
10. **Horses never rest sitting.** Dog-sitting is a transitional instant inside rising; held,
    it is a sign of hind-end weakness or spinal disease. A horse's lying-down is a collapse
    after gathering the four feet together: "flex front and hind legs and let gravity pull him
    to earth with a thump", carpi (dorsal surface) touching first. [R, med] [EQUUS1999; WikiVet; Kelemen2021; TheHorseDown]

---

## 2. Postures and transitions, per family

### 2.0 Reference joint limits (passive, goniometric)

Used to bound folded poses: a folded joint in a resting posture is at or near these limits.

| Joint | Dog (Labrador) flex / ext | Cat flex / ext |
|---|---|---|
| Carpus | 32 / 196 | 22 / 198 |
| Elbow | 36 / 165 | 22 / 163 |
| Shoulder | 57 / 165 | 32 / 163 |
| Hip | 50 / 162 | 33 / 164 |
| Stifle | 42 / 162 | 24 / 164 |
| Tarsus | 39 / 164 | 21 / 167 |

Included angles, degrees, means (SD about 2 deg). Dog [Jaegger2002], cat [Jaeger2007].
**[M, med]**: these are the commonly cited values of those papers, recalled, not re-read
here (paywalled); verify before hard-coding. Note cats fold every joint 10-20 deg further than
dogs, which is why the loaf and the tight sit are cat postures.

Horse: carpal flexion is shared by the radiocarpal joint (about 90 deg) and the midcarpal
joint (about 70 deg), the carpometacarpal joint barely moves [R, med] [HorseCarpus]; i.e. the
carpus can fold to roughly 20-30 deg included, cannon bone against the back of the forearm,
which is what a kneeling or sternal horse uses. In the horse hindlimb the reciprocal apparatus
couples stifle and hock: they flex and extend together, one cannot fold without the other
[R, high] [standard equine anatomy, not re-read here]. A rig must not fold a horse hock with
a straight stifle or vice versa.

---

### 2.1 DOG

#### 2.1.1 Sit (square sit)

**Ground contacts** [R, high] [WikiVet; Lafuente2024; anatomy]

- Both ischial tuberosities and the caudal thigh soft tissue (the buttock) on the ground; the
  tail base just above.
- Both hind metatarsi lie flat on their plantar surface, from the point of the hock (calcaneal
  tuber) to the digital pads; the hock points backward, the toes forward, roughly under or
  slightly behind the stifle.
- Both forepaws, planted, under or slightly in front of the shoulders.
- Nothing else: the abdomen and sternum are clear of the ground.

**Weight split** [M, med]: in the first quarter of rising, hindlimbs carry about 0.75 BW and
forelimbs about 0.25 BW [Ellis2018]; at rest in the sit a large share goes through the ischia,
so the hindfeet carry less than in stance. No measured static sit split was found [low].

**Joint angles** (included):

| Joint | Sit | Basis |
|---|---|---|
| Stifle | 45-75 deg, near full flexion; femur near horizontal, pointing forward from hip to stifle | [D, med]: standing stifle about 130-140 deg minus Labrador StS RoM 62 deg [Feeney2007 via Ellis2018] gives about 70 deg; the clinical "full flexion" [Lafuente2024] and goniometric limit 42 deg bound it below. The patella reaches its deepest flexion (92 deg patellar flexion) in the sit, deeper than any walk or trot pose [M, high] [Moore2016] |
| Tarsus | 45-70 deg; metatarsus flat (0 deg to the ground) | [D, med]: Labrador RoM 66 deg, greyhound RoM 89 deg from standing [Ellis2018]; limit 39 deg |
| Hip (pelvis-femur) | 65-90 deg, only moderately flexed | [D, med]: hip RoM 35 deg Labrador [Feeney2007 via Ellis2018]; half of walking RoM in beagles [Yoshikawa2023] |
| Hip rotation | femur externally rotated, stifles point forward and slightly outward | [M, med]: hindlimb external/internal rotation RoM in StS significantly larger than in walking [Yoshikawa2023]; greyhound ankle internal rotation up to 51 deg, partly marker artefact [Ellis2018] |
| Lumbar / lumbosacral | flexed (rounded loin), pelvis pitched caudoventrally | [R, high] sitting needs a "vertebral column flexible enough"; in the sitting cat the bend sits "predominantly near the thoracolumbar junction" so the thorax keeps its shape [WikiVet] |
| Forelimb | near vertical, joints close to standing | [M, med]: StS RoMs in Labradors shoulder 27, elbow 37, carpus 70 deg; greyhounds carpus only 32 deg [Ellis2018]: the front leg changes inclination during the rise, mostly at the carpus |

**Trunk inclination**: dorsal line (withers to sacrum) about 35-55 deg above horizontal,
breed dependent (long-backed, short-legged breeds lower) [D, low]: withers stays at about
0.9-1.0 H_w because the forelimbs stay straight [R, med], the sacrum drops to about 0.25-0.35
H_w [D, below], over a back length of about 1.1-1.3 H_w. No measured trunk angle was found.

**Heights above ground** [D, med unless noted]

- Ischium: 0. Hip joint: about tibia length x sin(tarsal angle) + hock pad, i.e. about
  0.25-0.35 H_w (Labrador tibia about 0.37 H_w, tarsus 45-70 deg).
- Stifle: about the same height as the hip or slightly lower, beside the caudal abdomen.
- Sternum: about elbow height, 0.45-0.55 H_w (dog elbow is near 0.5 H_w [R]).
- Underside: rises from 0 at the ischia, past the stifles, to sternum height; the inguinal
  region is clear of the ground by roughly the hock-to-stifle stack.

**Variants that read as wrong or unwell**: one hindlimb abducted and extended forward
("sloppy/puppy sit"): normal in puppies, in an adult a sign of stifle (or hip, tarsus) pain
[R, high] [Lafuente2024; Triviño2024]. Dogs with hip dysplasia usually sit normally but rise
slowly [R, med] (search-result summary of dvm360, not read in full).

#### 2.1.2 Stand to sit

Sequence [R, med unless noted]:

1. Feet stay planted (fore and hind) [M-protocol, high: dogs do StS without stepping, Ellis2018].
   A dog may shuffle the hindfeet a few cm forward before or during the sit.
2. Hindquarters lower first and mostly alone: the pelvis drops caudoventrally (backward and
   down), stifles and hocks fold together; the hocks swing down until the metatarsi lie flat.
3. The ischia touch down last in the hind chain; the loin rounds as the pelvis pitches.
4. The forelimbs stay straight; the forehand moves slightly back and up relative to the
   forepaws; the head stays level or rises (counterbalance).
5. Muscle pattern: in stand-to-sit, vastus lateralis, gluteus medius and gastrocnemius fire
   together (coordinated eccentric control); gastrocnemius peaks higher in stand-to-sit
   (206 +/- 46 % of walking peak) than in sit-to-stand (125 +/- 29 %) [M, high] [Yoshikawa2025].
   I.e. the hock is actively braked on the way down: the drop is controlled, not a fall.
6. The motion is continuous: neither sit-to-stand nor stand-to-sit "could be divided into
   multiple phases" from the joint RoM traces [M, high] [Yoshikawa2023]. No hold, no stepped
   keyframes inside it.

Duration: about 0.8-1.5 s for a relaxed medium dog, 0.4-0.6 s for a trained quick sit
[R, low]; no measured value found. Mirror of the measured 1.14 s rise is the best anchor.

#### 2.1.3 Sit to stand

Measured on greyhounds rising from a symmetric crouch with all four feet on two force plates
[M, high] [Ellis2018]:

- Duration 1.14 +/- 0.16 s (trial range 0.64-1.94 s).
- Hindlimb joints extend proximal to distal: hip mostly 0-20 % of the movement, knee 10-50 %,
  ankle 25-60 %. Hindlimb extension about 53-89 deg per joint, more than the forelimbs.
- Hindlimb vertical GRF rises quickly to about 0.75 BW by about 25 %; forelimb GRF falls to
  about 0.25 BW at the same time: the rise is driven from behind with the weight shifted back
  onto the hindlimbs.
- Peak net extensor moments at about 15 % (hip and ankle) while the limb is most flexed.
- Same sequence (hip then knee then ankle) as Labradors [Feeney2007] and humans.
- The vastus lateralis (stifle extensor) peaks at 337 +/- 165 % of walking during
  sit-to-stand vs 215 +/- 71 % in stand-to-sit [M, high] [Yoshikawa2025]: the stifle
  extension early in the rise is the effort.

Order of the ends: in a sit the forelimbs are already straight, so the hind end does the work;
the front end lifts passively as the pelvis rises. Head and chest may lean slightly forward
at the start [R, low].

#### 2.1.4 Sternal down ("sphinx" and the relaxed hip-rolled down)

Two forms [R, high] [WikiVet: "ventral recumbency with all four limbs tucked underneath or
the hindlimbs placed to one side"]:

A. **Symmetric sphinx (alert down)**

- Contacts: sternum; both elbows (olecranon) and the caudal forearms lying flat on the ground
  from elbow to carpus; the forepaws flat (palmar pads down) pointing forward; both hindlimbs
  folded under or beside the abdomen with the metatarsi flat on their plantar surface, hocks
  pointing back; the ventral thighs and caudal abdomen resting on or just above the folded
  hindlimbs.
- Forelimb angles [D, med]: elbow strongly flexed, about 40-60 deg (forearm horizontal,
  humerus sloping up-back to the shoulder); carpus near straight, 170-190 deg; shoulder
  flexed, about 70-100 deg.
- Hindlimb angles [D, med]: as in the sit at the stifle and hock (45-70 deg, near full
  flexion), but the hip is now strongly flexed (about 50-70 deg) because the pelvis is level
  again, not pitched.
- Heights [D, med]: withers about chest depth, 0.45-0.55 H_w; spine roughly horizontal; head
  carried up (neck raised) or chin on the paws; caudal abdomen gap 0 to about 0.05 H_w.

B. **Hip-rolled down (relaxed, the commonest settled down)**

- The forehand stays as in A (sternum and both forearms down). The pelvis rolls onto one side:
  the down-side lateral thigh (greater trochanter region) and the lateral surface of the
  down-side flexed hindlimb carry the hind weight; the up-side hindlimb lies folded on top or
  beside, its paw on the ground ahead of the stifle. The spine twists between thorax and
  pelvis. [R, high] [WikiVet; same asymmetric principle as the horse definition, Pedersen2004]
- Variant "sploot" (both hindlimbs extended straight back, ventral pelvis down) is a breed and
  individual habit, not the default [R, med].

#### 2.1.5 Lateral recumbency (lying on the side)

- Contacts: the whole down-side lateral surface: lateral thorax over the ribs, shoulder,
  lateral hip and thigh, lateral surfaces of the down-side limbs; the head on its cheek or
  with the neck extended along the ground [R, high].
- Limbs: loosely extended or mildly flexed, roughly parallel; the up-side limbs lie on the
  down-side ones or slightly in front of them [R, med].
- Heights [D, low]: the top of the up-side ribcage is at about the trunk's width (a dog's
  thorax is about 0.3-0.4 H_w wide at most breeds); the up-side limbs lie at about that height
  or sag onto the lower limbs.
- Getting into it: from the hip-rolled down, the dog lets the forehand roll onto the same side
  as the pelvis and lays the head down [R, med].

#### 2.1.6 Lying down (from sit or from standing) and getting up

Lying down [R, med] [WikiVet: "from the standing position, the dog, cat and pig adopt a
sitting position first"]:

1. Stand -> sit (2.1.2), about 1 s.
2. From the sit, the forepaws walk or slide forward, one or two short steps, while the
   elbows bend; the chest lowers ahead of and in front of the hind end, which stays on the
   ischia and the metatarsi. [R, med]
3. Sternum and elbows touch down; the pelvis rotates from pitched (sit) to level, the hip
   flexes further, the hindlimbs end tucked (sphinx) or the pelvis rolls to one side
   (hip-rolled). [R, med]
4. Optional: circling one or more turns before settling, and pawing/scratching the bedding
   (nesting) [R, med; common in dogs, no figure found].
5. Duration for steps 2-3 about 1-2 s; the hip roll at the end about 0.3-0.6 s [R, low].

Variants: some dogs go down front first from standing (a play-bow like lowering of the
elbows, hind end following); fast "drops" fold both ends at once [R, low].

Getting up [R, med]:

1. From lateral: head and neck lift first, then the thorax rolls to sternal while the limbs are
   drawn under (about 0.3-0.8 s) [R, low].
2. From sternal: forelimbs extend first (carnivore pattern [Nickel1986 via Ellis2018]),
   raising the forehand into a sit-like posture, then the hindlimbs extend proximal to distal
   as in 2.1.3. Duration about 1-1.5 s [R, low].
3. Small, fit animals can spring up all four limbs at once ("spring to its feet with a sudden
   back movement") [R, high] [WikiVet].
4. Often followed by a stretch: forelimbs extended forward with the chest low and hips high,
   then a hindlimb stretch; then a shake [R, med].
5. Measurement caveat: in greyhounds rising from a crouch the hindlimbs extended first "for
   unclear reasons" [M, high] [Ellis2018]: the forelimbs-first rule is a tendency, not a law.

---

### 2.2 CAT

No kinematic study of cat sitting or lying was found. Joint limits in 2.0 and the
descriptions below are what exists; angles are derived from those limits.

#### 2.2.1 Sit

- Contacts [R, high]: ischial region and caudal thighs; both hind metatarsi flat on their
  plantar surface (hock points touching behind); both forepaws close together, usually
  directly under or slightly in front of the shoulders, often close to the hindpaws; tail
  laid along the ground and often wrapped around the feet. Relaxed sitting is listed as a
  low-stress posture ("lying on side, on belly or sitting ... legs bent, hind legs may be laid
  out ... tail extended or loosely wrapped") [Kessler1997 via OSU and dvm360 adaptations].
- Joint angles [D, med]: stifle and tarsus at or near full flexion, about 25-45 deg (cat
  limits 24 and 21 deg); hip moderately flexed; forelimbs straight and near vertical.
- Trunk: more upright than a dog, dorsal line about 50-70 deg above horizontal [D, low]; the
  spine bends "predominantly near the thoracolumbar junction" so the ribcage keeps its shape
  [R, high] [WikiVet Fig 10.4].
- A fearful cat sits or crouches with the body "lower behind than in front", legs bent, feet
  close to the body, tail curled around or down [R, high] [Kessler1997 via dvm360]. A content
  sit is the opposite: chest lifted, tail loose.

#### 2.2.2 Loaf (forepaws tucked)

- Contacts [R, high]: the folded forelimbs and the sternum: the forearms lie along the
  ground under the chest, the carpi are flexed so the forepaws turn back under the chest
  (the dorsal surface of the paws and the carpi on the ground, no pads visible); the hindlimbs
  are folded under the body with stifles and hocks fully flexed and the metatarsi flat, feet
  hidden beside or under the belly; the tail along the side.
- Joint angles [D, med]: carpus fully flexed (palmar flexion), about 20-40 deg; elbow fully
  flexed, about 25-40 deg; hindlimb stifle and tarsus 25-40 deg.
- Heights [D, low]: the body sits on the stacked folded limbs, so the trunk underside is
  about a forearm's thickness above the ground (roughly 0.05-0.1 H_w) and the withers about
  0.5-0.6 H_w; the outline is a smooth box with no limbs visible.
- Reads as: relaxed but not deeply asleep; paws hidden = warm and safe [R, med].

#### 2.2.3 Sphinx lying

- As the dog sphinx (2.1.4 A): sternum down, forearms flat on the ground from elbow to
  forepaw, forepaws pointing forward with pads down, hindlimbs folded beside the abdomen with
  metatarsi flat, head up [R, high].
- Angles [D, med]: elbow 30-50 deg, carpus near straight; hindlimb as in the loaf.
- Difference from the loaf is only the forelimb: extended forward (sphinx, ready to rise)
  versus folded under (loaf).
- Further relaxed: the pelvis rolls to one side with the hindlimbs to one side ("half on
  side" in the Cat Stress Score), then fully lateral, or curled with the tail over the nose
  in the cold [R, med] [Kessler1997 via secondary].

#### 2.2.4 Transitions

- Sitting down [R, med]: as the dog (hind end lowers over planted paws, forelimbs stay
  straight), faster and tidier; the tail sweeps round as the rump lands.
- Lying down [R, med]: through a sit or a low crouch ("from the standing position, the dog,
  cat and pig adopt a sitting position first" [WikiVet]); from the sit the forepaws step or
  slide forward and the chest lowers (sphinx); to loaf, the cat then tucks one forepaw and
  the other with small shuffles of the chest. Often a turn or two and kneading before lying.
- Getting up [R, med]: forelimbs extend first, then the hind end rises (carnivore pattern);
  frequently a two-part stretch: forelimbs forward with the chest low and the rump high, then
  the body moves forward over the forelimbs while one hindlimb at a time stretches back.
- Durations [R, low]: sit about 0.5-1 s; sit-to-stand about 0.5-0.8 s; settling into a loaf
  1-3 s including paw tucks. Scaling the greyhound's 1.14 s rise by dynamic similarity
  (time proportional to sqrt(leg length)) with a cat leg about 0.4 of a greyhound's gives
  about 0.7 s [D, low].

---

### 2.3 HORSE

#### 2.3.1 Sternal recumbency

Ethogram definition used in horse lying studies [R, high] [Pedersen2004, quoted in Burla2017]:
"The asymmetrical sterno-abdominal posture in which the lateral surfaces of the flexed
underneath limbs are applied to the ground in such a way that the sternum and abdomen rest on
the ground to the right or left of the midline, with the muzzle resting on the ground, a
forelimb, or not at all."

Ground contacts [R, high unless noted]:

- Sternum and the abdomen, off the midline to one side.
- Forelimbs folded under the chest: carpi fully flexed, cannons folded against the back of
  the forearms, fetlocks flexed, hooves under the chest near the elbows; contact on the
  forearm and the dorsal-lateral carpus. Often one forelimb is instead extended forward with
  the hoof planted in front of the chest, especially just before rising [R, med].
- Hindquarters rolled onto one hip: the down-side hindlimb flexed beneath the belly lying on
  its lateral surface; the lateral thigh and hip of that side carry the hind weight; the
  up-side hindlimb flexed, lying beside or over it with its hoof on the ground near the
  down-side stifle or forward toward the elbow.
- Head held up, or muzzle resting on the ground; REM sleep is possible in sternal only with
  the muzzle resting on the ground (or when leaning on something) [M-review, high] [Burla2017;
  Diehl2020].

Joint angles [D, low-med]:

- Forelimb: carpus 20-40 deg (near its limit, [HorseCarpus]); elbow about 60-90 deg;
  shoulder flexed.
- Hindlimb: stifle and hock flexed together (reciprocal apparatus), about 40-70 deg each; hip
  strongly flexed.
- Spine: twisted between a nearly upright thorax and a pelvis tilted onto its side, by
  roughly 30-60 deg of roll [D, low].

Heights [D, low]: withers at about chest depth plus the sternal soft tissue, 0.45-0.55 H_w
(a horse's chest depth is a little under half its height); the forehand reads upright and the
croup lower and tilted.

#### 2.3.2 Lateral recumbency

Ethogram definition [R, high] [Pedersen2004 via Burla2017]: "A right or left posture in which
the upper forelimb is anterior to the lower forelimb, the hind limbs are extended, and the
head is resting on the ground."

- Contacts: whole down-side lateral surface of thorax, shoulder, hip and thigh; down-side
  limbs on their lateral surfaces; the side of the head and neck flat on the ground.
- Limbs extended; the upper forelimb lies ahead of the lower; the up-side limbs are held at
  about the height of the trunk's half width, often slightly flexed and moving in REM sleep.
- Time budget: lateral lying is about 15 % of recumbency (about 20 min/day) in domestic
  adults [M-review, med] [Kelemen2021], 15-30 % in feral horses, 4-30 % group-housed
  [M-review, med] [Burla2017]. Total recumbency 3-15 % of the day (43-216 min) [Kelemen2021];
  about 25 min/day in one riding-school sample [M, med] [Gobbo2025]; under 3 % of the day in
  another source [R, med] [EdinburghPain]. Horses fall asleep soon after lying down
  [Burla2017].
- Lame or painful horses lie down MORE (13 +/- 11 % vs 3 +/- 6 % of 11 overnight hours)
  [M, med] [Uellendahl2024], while severe carpal osteoarthritis can stop a horse lying down at
  all [M-case, med] [Kelemen2021].

#### 2.3.3 Lying down (standing -> sternal -> lateral)

Sequence [R, med-high]:

1. Preparation: sniff the ground with the head low, paw, and circle; "rotating behaviour
   just before lying down" is a scored item in box-size studies [M-item, high]
   [Raabymagle2006]. Horses in small boxes ((1.5 x height)^2 vs (2.5 x height)^2) lay in
   sternal for significantly shorter times [M, high] [Raabymagle2006].
2. Gather: "the horse arches its back, brings its four limbs underneath the trunk, lowers its
   head" [R, high] [WikiVet]: a very narrow base, all four hooves close together under the
   belly, back roached, head and neck low.
3. Drop: "flexes its limbs and drops heavily to one side" [WikiVet]; "all a horse needs to do
   is flex his front and hind legs and let gravity pull him to earth with a thump"
   [EQUUS1999]. The forehand usually goes first: the forelimbs buckle and the dorsal carpi
   touch down first [R, med]; the dorsal carpi and fore fetlocks are where horses that
   collapse when sleep-deprived get their pathognomonic skin lesions, which marks that
   surface as the kneeling contact [M-clinical, high] [Kelemen2021]. The hindlimbs then flex
   (stifle and hock together) and the hindquarters fall onto one hip. Sources disagree on how
   distinct the kneeling stage is; in a relaxed horse fore and hind fold within about a
   second of each other [R, low].
4. Settle in sternal (2.3.1): the croup rolls onto the hip, the forelimbs tuck.
5. Optionally to lateral: the forehand rolls onto the same side, the limbs extend, the head and
   neck are laid down. Rolling (onto the back and over) is a separate behaviour, defined as
   lateral lying followed by moving up onto the back [R, high] [Raabymagle2006].

Durations [R, low]: preparation 5-30 s or none; gather 1-3 s; drop from gathered stance to
sternal 1-3 s; sternal to lateral 1-3 s. No measured value for unsedated horses was found.
(A sedated horse took 53.8 +/- 9.1 s to reach lateral recumbency under detomidine and
tiletamine-zolazepam: irrelevant to voluntary lying, noted only so it is not misused
[M, high] [search result, scielo].)

#### 2.3.4 Getting up (lateral -> sternal -> standing)

Sequence [R, med-high] [WikiVet; EQUUS1999; Fuhs2021; Huebener2006]:

1. Head and neck lift; the horse "rolls its trunk to sternal recumbency, collects its four
   legs underneath" [WikiVet]. If the limbs face a wall or lack room, it may roll over its
   back to the other side first; the frequency of rolling before getting up depends on box
   size [M, high] [Raabymagle2006].
2. "Raises his head high, which lifts his chest slightly and allows him to begin unfolding
   his legs" [EQUUS1999].
3. Forelimbs extend forward, one then the other, hooves planted in front of the chest
   ("Strecken der Vordergliedmaßen vor die Brust" is a coded phase in a Vienna study of rising
   [Fuhs2021]); the horse "protracts and extends its forelimbs, raising its cranial trunk
   before its caudal trunk" [WikiVet].
4. Brief dog-sit: "the horse lifts himself into a sitting position, almost like a dog. He may
   remain in this position only briefly, or he may pause to collect himself" [EQUUS1999].
   Hindlimbs still folded under, croup on the ground or just off it.
5. Heave: "throws his head down and forward: the extreme weight of his front end (20 percent
   of his entire body weight) acts like a counterbalance to lift his hindquarters" [EQUUS1999],
   while "pushing its weight over its forelimbs with a thrust from the hindlimbs" [WikiVet].
   The head and neck work as a balancing rod throughout lying down, rolling and rising
   [R, high] [Huebener2006].
6. Often shakes after standing [R, med].

Durations [R, low]: steps 1-2 about 1 s; forelimb extension about 0.5-1 s per limb; dog-sit
0 to several s; heave about 1 s; whole rise from sternal about 2-5 s. Not measured in any
source found. A healthy horse rises in one attempt; repeated attempts, or a held dog-sit,
read as illness [R, high] [EQUUS1999].

#### 2.3.5 Do horses sit?

- Not as a resting posture. The dog-sit exists as the transitional stage 4 of rising
  [R, high] [EQUUS1999].
- A horse that stays dog-sitting is a clinical sign: thoracolumbar spinal lesions leave the
  hindlimbs weak while the forelimbs push the front up [R, med] [TheHorseDown, paywalled
  beyond the first paragraph]; "dog-sitting" in lambs and calves is a pain posture
  [R, high] [EdinburghPain]; in equine obstetrics "dog-sitting" names a foal malpresentation
  (0.7 % incidence) [M, med] [search result, Acta Vet Scand figure caption]. Contrast:
  "sitting down ... is a usual position for the dog and cat, an occasional one for the pig, and
  a possible although a very unusual one for cattle" [WikiVet]; horses are not on the list.
- Character animation: cartoon and toy horses are routinely given a dog-style sit (forelegs
  straight, haunches down, hind cannons flat) because the pose reads as friendly and
  attentive; it is a stylisation that audiences accept [R, low; convention, no source read].
  For a horse-like avatar the anatomically faithful choices are: (a) a held stage 4 of rising
  (forelegs straight in front, hindlimbs folded under, croup on the ground), which is the real
  horse "sit"; or (b) a sternal rest with the head up. A dog-like sit with the hind
  metatarsi flat on the ground is NOT how a horse's folded hindlimb lies: in the real dog-sit
  stage the hind limbs are folded under the body with the hocks flexed beside the belly
  [R, med].

---

## 3. Notes: what the numbers do not capture

1. **Solve the pose from the contacts, then fill in angles.** Every posture above is defined
   first by which surfaces touch (ischia, plantar metatarsi, sternum, forearms, lateral thigh,
   dorsal carpus) at height 0 and which do not. Angles applied from a standing root push the
   ischia, hocks and elbows through the floor because the hip must drop by roughly 0.6-0.7 of
   its standing height in a dog sit [D, med] while the withers barely moves.
2. **The sit is a pelvic pitch plus a stifle-hock fold.** The hip angle changes only 20-35 deg
   (fact 1). If a rig flexes the hip deeply instead, the femur swings down into the ground and
   the thigh reads as broken. The lumbar spine must round as the pelvis pitches.
3. **Planted feet.** Real dogs sit and rise without moving their feet [Ellis2018]; a foot that
   slides during a sit reads as slipping. Small hindfoot shuffles before the sit are fine.
4. **Continuous, braked descent.** Stand-to-sit is a controlled eccentric movement with
   coordinated quadriceps, gluteal and gastrocnemius activity [Yoshikawa2025] and no internal
   phases [Yoshikawa2023]: ease in and out, no free fall, no hold mid-way. The opposite is true
   of a horse lying down, which does drop the last part.
5. **Proximal-to-distal timing on the rise** (hip 0-20 %, stifle 10-50 %, hock 25-60 %
   [Ellis2018]) and the weight shift to the hindlimbs early (0.75 BW at 25 %). A rig that
   extends all joints in lockstep reads as robotic.
6. **Asymmetry is the relaxed look.** Settled dogs, cats and horses lie with the pelvis rolled
   onto one hip and the thorax upright; a perfectly symmetric sphinx reads as alert or posed.
   Pick a side and keep it for the duration of the rest; the spine twists between thorax and
   pelvis.
7. **Head and neck lead.** Horses lower the head before lying and use it as a balancing rod
   to rise (thrown forward-down to unload the hind end) [Huebener2006; EQUUS1999]; dogs and
   cats raise the head first when rolling from lateral to sternal. The head moves before the
   body.
8. **Cross-breeds.** Dog- and cat-like bodies use the carnivore sequence (sit first, front
   extends first on rising, hind metatarsi flat in the sit). Horse-like bodies must not sit
   on flat hind cannons as a rest; they gather, drop, and rise front first through a brief
   dog-sit. A cross should inherit its lying strategy from its leg design: a horse-like distal
   limb (long cannon, hoof, coupled stifle-hock) cannot fold flat under like a cat's.
9. **Do not borrow cattle reference.** Cattle go down carpi first in a deliberate kneel and
   rise HIND first, then up onto the carpi, then the forelimbs [WikiVet]; horses rise front
   first. Cow footage is the commonest wrong reference for horse rising.
10. **Space.** Horses need room in front of the chest to extend both forelimbs and room to
    roll; with too little space they lie in sternal for shorter bouts and roll over before
    rising [Raabymagle2006; Burla2017]. In a world, a horse avatar lying close to a wall
    should rise away from it.
11. **Readiness reads.** Sternal with the forelimbs extended forward (sphinx) is
    ready-to-rise; folded under (loaf, horse forelimbs tucked) is settled; lateral is deep
    rest. A healthy animal rises in one fluid attempt; hesitation, repeated attempts or a
    leg held out sideways read as pain to any vet [Lafuente2024; EQUUS1999].
12. **Timing scale.** For stylised sizes, scale transition times by sqrt(leg length) from the
    greyhound's 1.14 s rise (dynamic similarity) [D, low]; cute styling often wants a slightly
    slower settle and a quicker rise.
13. **Gaps in the literature.** No measured joint angles of a dog in a held sit, no trunk
    inclination, no timed horse lying-down or rising for unsedated horses, and no cat posture
    kinematics were found. The paywalled [Yoshikawa2023] (beagle sit and stand angles over time)
    and [Feeney2007] (Labrador sit-to-stand angle histories) are the two papers that would
    replace most [D] figures in 2.1 if the owner can obtain them.

---

## 4. Sources

- **[Ellis2018]** Ellis RG, Rankin JW, Hutchinson JR (2018). Limb kinematics, kinetics and
  muscle dynamics during the sit-to-stand transition in greyhounds. Front Bioeng Biotechnol
  6:162. https://www.frontiersin.org/journals/bioengineering-and-biotechnology/articles/10.3389/fbioe.2018.00162/full
  (full text read; Table 3 values are an image and were not readable, the text gives the
  ranges quoted; it also cites Feeney et al. 2007 Labrador RoMs and Nickel et al. 1986.)
- **[Yoshikawa2023]** Yoshikawa K, Kitazawa T, Sano T, Ino T, Miyasaka T (2023). Kinematic
  characteristics of canine hindlimb movement during sit-to-stand and stand-to-sit motions.
  Res Vet Sci 162:104944. https://doi.org/10.1016/j.rvsc.2023.104944 (abstract via Europe PMC,
  PMID 37423012).
- **[Yoshikawa2025]** Yoshikawa K, Kitazawa T, Sano T, Ino T, Miyasaka T, Edamura K (2025).
  Surface electromyography of the gluteus medius, vastus lateralis, and gastrocnemius muscles
  during sit-to-stand and stand-to-sit transitions in normal Beagles. Res Vet Sci 193:105783.
  https://doi.org/10.1016/j.rvsc.2025.105783 (abstract via Europe PMC, PMID 40616965).
- **[Feeney2007]** Feeney LC, Lin CF, Marcellin-Little DJ, Tate AR, Queen RM, Yu B (2007).
  Validation of two-dimensional kinematic analysis of walk and sit-to-stand motions in dogs.
  Am J Vet Res 68:277-282. https://doi.org/10.2460/ajvr.68.3.277 (abstract read; RoMs quoted
  from Ellis2018.)
- **[Moore2016]** Moore EJ, Kim SE, Banks SA, Pozzi A, Coggeshall JD, Jones SC (2016). Normal
  patellofemoral kinematic patterns during daily activities in dogs. BMC Vet Res 12:262.
  https://doi.org/10.1186/s12917-016-0889-z
- **[Lafuente2024]** Lafuente P (2024). Sit to stand test to identify cranial cruciate
  ligament disease (commentary). Clinician's Brief Global, Fall 2024.
  https://www.qgdigitalpublishing.com/publication/?i=827714&article_id=4822999&view=articleBrowser
- **[Triviño2024]** Triviño A, Davidson C, Clements DN, Ryan JM (2024). Objective comparison
  of a sit to stand test to the walk test for the identification of unilateral lameness caused
  by cranial cruciate ligament disease in dogs. J Small Anim Pract 65:24-29.
  https://doi.org/10.1111/jsap.13679 (abstract via Europe PMC.)
- **[Jaegger2002]** Jaegger G, Marcellin-Little DJ, Levine D (2002). Reliability of goniometry
  in Labrador Retrievers. Am J Vet Res 63:979-986. https://doi.org/10.2460/ajvr.2002.63.979
  (abstract read; joint values recalled from the literature, not re-read.)
- **[Jaeger2007]** Jaeger GH, Marcellin-Little DJ, Depuy V, Lascelles BDX (2007). Validity of
  goniometric joint measurements in cats. Am J Vet Res 68:822-826.
  https://doi.org/10.2460/ajvr.68.8.822 (abstract read; values recalled, not re-read.)
- **[Nickel1986]** Nickel R, Schummer A, Seiferle E et al. (1986). The Locomotor System of the
  Domestic Mammals. Cited via Ellis2018 for "carnivores extend their forelimbs first whereas
  ruminant herbivores extend their hindlimbs first". Not read.
- **[WikiVet]** WikiVet, Quadrupedal Mechanics - Anatomy & Physiology (revision of
  29 Nov 2013), sections "Sitting down", "Lying down", "Standing up", Fig 10.4.
  https://en.wikivet.net/Special:MobileDiff/159422
- **[Pedersen2004]** Pedersen GR, Søndergaard E, Ladewig J (2004). The influence of bedding on
  the time horses spend recumbent. J Equine Vet Sci 24:153-158.
  https://doi.org/10.1016/j.jevs.2004.03.013 (definitions quoted via Burla2017.)
- **[Burla2017]** Burla JB, Rufener C, Bachmann I, Gygax L, Patt A, Hillmann E (2017). Space allowance
  of the littered area affects lying behavior in group-housed horses. Front Vet Sci 4:23.
  https://www.frontiersin.org/journals/veterinary-science/articles/10.3389/fvets.2017.00023/full
- **[Kelemen2021]** Kelemen Z, Grimm H, Long M, Auer U, Jenner F (2021). Recumbency as an
  equine welfare indicator in geriatric horses and horses with chronic orthopaedic disease.
  Animals 11:3189. https://doi.org/10.3390/ani11113189 (full text via Europe PMC PMC8614510.)
- **[Raabymagle2006]** Raabymagle P, Ladewig J (2006). Lying behavior in horses in relation to
  box size. J Equine Vet Sci 26:11-17. https://doi.org/10.1016/j.jevs.2005.11.015 (abstract.)
- **[Gobbo2025]** Gobbo E, Maccario C, Zupan Šemrov M, Bovo M, Atallah E, Minero M,
  Dalla Costa E (2025). Exploring the impact of housing routine on lying behavior in horses
  measured with triaxial accelerometer. Front Vet Sci 12:1572051.
  https://doi.org/10.3389/fvets.2025.1572051 (abstract.)
- **[Uellendahl2024]** Uellendahl A, Schramel JP, Tichy A, Peham C (2024). Monitoring of
  non-lame horses and horses with unilateral hindlimb lameness at rest with the aid of
  accelerometers. Sensors 24:7203. https://doi.org/10.3390/s24227203 (abstract.)
- **[EQUUS1999]** EQUUS magazine, "Rolling and Rising" (July 1999; republished EquiSearch
  20 Jan 2003). https://equisearch.com/articles/eqrolling3410/ (popular veterinary-advised
  article; the only source found with the full rising sequence and the 20 % front-end figure,
  which is unsourced there: confidence med.)
- **[Huebener2006]** Huebener E (2006). The horse's movement cycles while lying down, rolling
  and jumping up. Tierärztliche Umschau 7:347 (German; abstract only).
  https://references.equine-behaviour.de/show.php?record=431
- **[Fuhs2021]** Fuhs A-M (2021). Unterschiede im Liege- und Aufstehverhalten von Pferden mit
  Störungen des Bewegungsablaufes. Diploma thesis, Vetmeduni Vienna (supervisor T. Licka).
  https://phaidra.vetmeduni.ac.at/api/object/o:1828/download (only the title and the coded
  phase names were readable.)
- **[Diehl2020]** Diehl N (2020). Why does a horse lie down? The Horse, 2 Jan 2020.
  https://thehorse.com/149645/why-does-a-horse-lie-down/
- **[TheHorseDown]** "The Down Horse", The Horse. https://thehorse.com/16468/the-down-horse/
  (first paragraph only, via search summary.)
- **[EdinburghPain]** University of Edinburgh, Animal Pain: Postural changes in response to
  pain. https://www.link.vet.ed.ac.uk/animalpain/Pages/posture.htm
- **[Kessler1997]** Kessler MR, Turner DC (1997). Stress and adaptation of cats housed singly,
  in pairs and in groups in boarding catteries. Anim Welf 6:243-254 (Cat Stress Score; posture
  wording read via adaptations: dvm360 "Feline posture visual dictionary",
  https://www.dvm360.com/view/feline-posture-visual-dictionary , and an OSU Indoor Pet
  handout, search summary only.)
- **[HorseCarpus]** "Introduction to the equine carpus (knee): anatomy, biomechanics, and
  common problems". https://www.horseeducationonline.com/post/introduction-to-the-equine-carpus-knee-anatomy-biomechanics-and-common-problems
  (search-result excerpt only: radiocarpal about 90 deg, midcarpal about 70 deg.)
- **[MANN2018]** Zhang H, Starke S, Komura T, Saito J (2018). Mode-adaptive neural networks
  for quadruped motion control. ACM Trans Graph 37(4) (SIGGRAPH). Dog motion-capture set
  including sitting and idling; a possible source of real dog sit/lie clips for timing.
  https://history.siggraph.org/?p=100804 (search summary only.)
- Sedated-horse lying time (53.8 +/- 9.1 s): scielo.br record "https://scielo.br/j/cr/a/g6rJmyZNNLtDD7qCYCbjY8t/abstract/?lang=en"
  (search summary only; cited only to warn against using it.)
- Foal "dog-sitting" malpresentation 0.7 %: Acta Vet Scand figure,
  https://actavetscand.biomedcentral.com/articles/10.1186/s13028-024-00733-1/figures/8
  (search summary only.)
