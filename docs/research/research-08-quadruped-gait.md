# Quadruped locomotion kinematics for a procedural rig (dog, cat, horse)

*A point-in-time research dossier gathered for the quadruped refinement (#372, R0 #373, 2026-10-28). Kept as the record of what was found and why the refinement's decisions were made. Every figure carries its own confidence; a value recalled from a paywalled paper, or derived here, is marked so and needs checking before anything hard-codes it.*

Scope: how the body moves in walk, trot, pace, canter, gallop. Morphology at rest and sit/lie postures are out of scope.

Legend used in every table:
- **Type**: M = measured (number from a study), D = derived here from measured numbers (arithmetic shown), R = rule of thumb (recalled textbook/craft figure, or observation without a number found in a readable source).
- **Conf**: H = high, M = medium, L = low.
- **Conventions** (each row names one):
  - **INC** - included joint angle, measured on the flexion side between the two segments; 180° = straight; smaller = more flexed; above 180° = hyperextension (dorsiflexion of carpus, fetlock, MCP/MTP).
  - **SEG-V** - segment angle to the vertical (Fischer et al. 2018): 0° = segment vertical, positive = protracted (distal end forward), negative = retracted.
  - **LEG-T** - effective leg angle to the trunk (Andrada et al. 2023): angle between the trunk vector (hip to scapular pivot, pointing cranially) and the line from the limb's proximal pivot (hip joint, or the scapula's pivot) to the paw. 90° = leg perpendicular to the trunk; below 90° = paw ahead of the pivot (protracted); above 90° = paw behind (retracted).
  - **LEG-H** - limb orientation of the cat forelimb (Rahmati et al. 2025): line from shoulder joint centre to the most distal carpal point, angle to horizontal; around 90° = vertical; small = protracted, large = retracted.
  - **ROM** - range of motion, max minus min over the stated phase.
  - **Phase** - percent of the stride from touch-down (TD) of the reference limb unless the row says "% of stance"; TO = toe-off / lift-off.
- Breeds: medium dog = Beagle, Malinois, Labrador, German Shepherd (GSD); gallop dog = Greyhound; cat = domestic short-hair; horse = Warmblood / riding horse unless stated.

---

## 1. Summary: the ten facts that most change whether a gait reads as natural

1. **The forelimb swings from the shoulder blade, not the shoulder joint.** In dogs the gleno-humeral joint "stays nearly immobile" and the scapula is the centre of rotation (Fischer & Lilje 2011). Across small mammals, scapula plus femur supply **more than 50%** of the step (Fischer et al. 2002). A pony's scapula rotates **27.6° at walk and 23.3° at trot** (fluoroscopy, 2026).
2. **The legs work as a pantograph.** The scapula stays parallel to the forearm, and the femur to the metatarsus, through the whole stride ("if the forearm is vertical, the shoulder blade will be in the same position"). The rig should drive the scapula angle from the forearm angle, not on its own.
3. **Each stance sweeps about 40-48° of effective leg angle.** A dog's leg lands at **LEG-T ≈ 70°** at walk (**≈ 74°** at trot), so it is about 16-20° protracted. It leaves at **107-123°** (walk) or **104-118°** (trot), so it is 14-33° retracted. Retraction is larger than protraction; the sweep is not symmetric about vertical.
4. **Duty factor per gait.** Dog walk **0.56-0.64**, dog trot **0.39-0.47**, cat walk **0.64**, horse gallop hindlimb **0.28-0.32**. A trot with duty factor ≥ 0.5 reads as a shuffle, except in cats, which really do trot near 0.5.
5. **The front paw folds hard in swing and over-straightens in stance.** In a trotting GSD the carpus has a **swing ROM of 89°** and the MCP joint **105°**. Both hyperextend past 180° through almost the whole stance. A horse's carpus flexes to an included angle of **≈ 92°** in trot swing. The hind paw breaks far less.
6. **Leg length barely changes in stance, then shortens a lot in swing.** The dog effective leg *lengthens* **≈ 4%** during walking stance and **1-3.5%** during trotting stance; it does not compress like a pogo stick. The cat forelimb shortens by **33%** from end-stance (0.204 m) to mid-swing (0.137 m).
7. **Vertical body motion, horse (in hand).** Walk: head **80-117 mm**, withers **27-36 mm**, pelvis **66-79 mm**. Trot: head **61-78 mm**, withers **47-92 mm**, pelvis **56-89 mm**. That is roughly 2-6% of withers height. Withers bob is small at walk and large at trot.
8. **Bob phase.**
   - Trot: head, withers and pelvis are lowest near **mid-stance (≈ 40-45% of stance)** and highest at the **start of suspension (≈ 92-100% of stance)**.
   - Walk: **head and pelvis move in phase, and the withers run ≈ 25% of a stride out of phase.**
   - Walk head: two nods per stride, lowest at **≈ 47% of each forelimb's stance**.
9. **The spine is quiet in horses and loud in carnivores.**
   - Horse thoracolumbar flexion-extension: ≈ **4°** at walk, smallest at trot, ≈ **9°** at canter. Lumbosacral: **6-7°** at canter.
   - Dog at walk and trot: lateral bending is ≥ sagittal bending. It is one wave per stride, peaks at the thoracolumbar junction, travels head to tail at walk and is synchronous at trot.
   - Small-mammal gallop: **7 ± 1** intervertebral joints bend, and lumbar flexion supplies **≈ half** of the hindlimb step.
10. **Scale stride by HIP height, not withers height.** Use λ/h ≈ 2.3·Fr^0.3 with h = hip-joint height, which is ≈ 0.65 × withers height in a horse.
    - Check: a horse trotting at 4 m/s with h = 1.05 m gives λ ≈ 2.75 m at 1.45 Hz, which matches measured horses. Withers height overestimates λ by about 30%.
    - Greyhounds keep **≈ 3.5 strides/s** across their whole gallop speed range and gain speed by stride length.

---

## 2. Tables

### 2.1 Gait timing and scaling (all families)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Froude number | Fr = v²/(g·h) | h = hip-joint height standing (Alexander & Jayes); Andrada uses hip-to-ground effective leg | - | M (definition) | Alexander & Jayes 1983; Andrada et al. 2023 | H |
| Dog preferred trot range | 0.5 to 3 Fr | Fr as above | walk below ≈ 0.5 | M (cited) | Andrada et al. 2023, citing Jayes & Alexander 1978 | M-H |
| Horse walk-trot transition | Fr ≈ 0.35 | Fr as above | - | M (review, via search summary) | 2016 review of horse gait transitions, as reported in search; White Rose eprint 201823 | M-L |
| Trot-gallop transition | Fr ≈ 2-3 | - | - | R (Alexander's classic figure, not re-verified here) | Alexander 1984/1989 | M |
| Relative stride length | λ/h ≈ 2.3·Fr^0.3 | h = hip height | stride = full cycle of one foot | R (recalled fit, not re-verified here) | Alexander 1984 "Gaits of bipedal and quadrupedal animals"; Alexander & Jayes 1983 | M |
| Check: Labrador trot | h 0.50 m, 2 m/s → Fr 0.82 → λ ≈ 1.08 m, f ≈ 1.85 Hz | - | - | D | from row above | M |
| Check: horse trot | h 1.05 m (hip), 4 m/s → Fr 1.55 → λ ≈ 2.75 m, f ≈ 1.45 Hz; using withers 1.6 m gives λ ≈ 3.5 m (too long) | - | - | D | from row above; horse walk table below | M |
| Check: cat walk | h 0.22 m, 0.6 m/s → Fr 0.17 → λ ≈ 0.30 m, f ≈ 2 Hz | - | - | D | from row above; measured cat cycle 0.58-0.87 s | M-L |
| Dog walk duty factor | 0.56-0.64 (Malinois 0.58-0.64, Whippet 0.56-0.60, Beagle 0.60-0.61, French Bulldog 0.56-0.63) | stance / stride, per limb | speeds 0.7-1.3 m/s | M | Fischer, Lehmann, Andrada 2018 | H |
| Dog trot duty factor | 0.39-0.47 (Malinois 0.42, Whippet 0.41-0.42, Beagle 0.39-0.42, French Bulldog 0.41-0.47) | per limb | speeds 1.4-2.5 m/s | M | Fischer et al. 2018 | H |
| Dog trot speed, comfortable | 1.9 ± 0.1 m/s (mixed medium dogs, treadmill) | - | - | M | Bergh et al. 2018 | H |
| Cat walk duty factor | 0.64 ± 0.01 (forelimb) | per limb | cycle 0.69 ± 0.09 s, overground, self-selected | M | Rahmati et al. 2025 (7 cats, 3.2 kg) | H |
| Cat walk / trot speed | walk 1-4 km/h (0.3-1.1 m/s); trot 3.5-8 km/h (1.0-2.2 m/s), overlap in 1.0-1.1 m/s | - | - | M (via search summary) | Polish cat gait study, ane.pl | M |
| Cat trot duty factor | can stay near or above 0.5 ("cats sometimes trot at a higher duty factor") | - | - | R | UCSD CSE169 lecture | L-M |
| Cat step phases | F and E1 = swing, E2 = yield, E3 = propulsion. With speed, E3 shortens most; F is nearly constant in absolute time, so F's share of the cycle grows | - | hindlimb | M (abstract) | Goslow, Reinking & Stuart 1973 | H (qualitative) |
| Horse walk | 0.9-2.1 m/s, 0.7-1.1 Hz, stride 1.3-1.9 m; hind duty factor 0.55-0.75 | - | - | M (review table, via search summary) | equine gait review (IntechOpen chapter 83285) | M |
| Horse trot | stride rate and stride length both rise from 3.25 to 5.5 m/s; duty factor < 0.5 (suspension) | - | fore-first diagonal dissociation is normal | M | Gmel et al. 2024 (via search); Rhodin et al. 2022 | M |
| Horse trot suspension | present each half-stride; shorter when ridden | % of stride | - | M | Rhodin et al. 2022 | H |
| Horse canter | λ ≈ 3-3.5 m at 6-7 m/s, ≈ 1.8-2 Hz; duty factor ≈ 0.4-0.5 | - | 3-beat: trailing hind, diagonal pair, leading fore, suspension | R | Back & Clayton, Equine Locomotion (general) | L-M |
| Horse gallop | hind duty factor 0.31-0.32 at 9.5 m/s and 0.28-0.29 at 12.5 m/s; 2.0-2.2 strides/s; fore stance 131 ms at 9 m/s down to 77 ms at 17 m/s | - | transverse gallop (4-beat, one suspension) | M (via search summaries) | Thoroughbred incline-gallop and elite-gallop studies (RVC) | M |
| Greyhound gallop | ≈ 3.5 strides/s, roughly constant over the speed range; stride up to ≈ 3.5 m at 13 m/s; swing ≈ 0.2 s | - | rotary gallop, 2 suspensions | M | Hudson, Corr & Wilson 2012; SICB abstract | M-H |
| Greyhound gallop duty factor | ≈ 0.3 (stance ≈ 0.29 s stride - 0.2 s swing ≈ 0.09 s) | - | - | D | from row above | M-L |
| Cheetah (for exaggeration) | 2.4 Hz at 9 m/s up to 3.2 Hz at 17 m/s; stride up to 5.3 m; stride relative to height ≈ 2× a horse's | - | rotary | M | Hudson et al. 2012; UMN gait site | M |
| Rotary gallop footfalls | e.g. RH, LH, extended suspension, LF, RF, gathered suspension | - | lead = limb lifted just before suspension | M (descriptive) | UMN Veterinary Anatomy gaits site | H |

### 2.2 Dog - limb segments and effective leg (walk, trot)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Forelimb effective leg at TD, walk | 67.8-71.0° (Beagle, Malinois, Whippet, French Bulldog) | - | LEG-T, leg from scapular pivot to paw; 0% stance | M | Andrada et al. 2023 | H |
| Forelimb at 25 / 50 / 75 / 100% stance, walk | ≈ 78-81 / 92-97 / 103-109 / 107-115° | - | LEG-T | M | Andrada et al. 2023 | H |
| Forelimb at TD, trot | 68-74° | - | LEG-T | M | Andrada et al. 2023 | H |
| Forelimb at TO, trot | 109-117° | - | LEG-T | M | Andrada et al. 2023 | H |
| Hindlimb effective leg at TD, walk | ≈ 70° (68.9-72.6) | - | LEG-T, hip joint to metatarsal marker | M | Andrada et al. 2023 | H |
| Hindlimb at TO, walk | 112.5° (French Bulldog) to 123° (Malinois) | - | LEG-T | M | Andrada et al. 2023 | H |
| Hindlimb at TD / TO, trot | ≈ 73-75° / 104-118°; 3-8° less retraction than at walk | - | LEG-T | M | Andrada et al. 2023 | H |
| Paw landing position, walk | ≈ 0.34 L ahead of the proximal pivot (L·cos 70°) | L = effective leg length | TD | D | from rows above | M |
| Paw lift-off position, walk | ≈ 0.29-0.54 L behind the pivot (L·cos 107-123°) | - | TO | D | from rows above | M |
| Paw landing position, trot | ≈ 0.28 L ahead (cos 74°) | - | TD | D | from rows above | M |
| Effective leg length change, stance | walk: +4% (Beagle, Malinois); trot: +1.1% (Whippet), +2.7% (French Bulldog), ≈ +3.5% (Beagle, Malinois); French Bulldog at walk *shortens* ≈ 4% | % of length at TD | TD to TO | M | Andrada et al. 2023 | H |
| Scapular pivot location | modelled at 2/3 of the distance between the most dorsal and most ventral scapular markers (the "scapulothoracic joint") | scapula height | - | M (model assumption from earlier fluoroscopy) | Andrada et al. 2023, citing Fischer 1999 and Andrada et al. 2017 | M. The wording does not say from which end; read literally from dorsal, the pivot is ≈ 1/3 of the blade's height above the shoulder joint. Fischer's group describes it as an *instantaneous* centre that moves. |
| Virtual pivot point of forelimb forces | above and caudal to the scapular pivot; closer to it at trot than at walk | % of leg length | - | M | Andrada et al. 2023 | H |
| Hindlimb virtual pivot point | walk: ≈ 0.5-1.0 L above the hip; trot: ≈ 0.2 L above it (Whippet ≈ 0.03 L below) | L at TD | - | M | Andrada et al. 2023 | H |
| Hip effective angular movement, TD to TO | walk / trot: Malinois 47 / 48°, Whippet 41 / 43°, Beagle 41 / 40°, French Bulldog 36 / 41° | - | joint flexion-extension | M (fluoroscopy) | Fischer et al. 2018 | H |
| Stifle movement in stance | walk / trot: Whippet 5 / 4°, Beagle 4 / 2°, French Bulldog 14 / 1° | - | TD minus TO | M (fluoroscopy) | Fischer et al. 2018 | H |
| Hindlimb retraction driver | the femur's retraction provides the limb's retraction; the stifle stays near-constant in stance | - | stance | M | Fischer et al. 2018 | H |
| Femur retraction, French Bulldog vs others | up to 20° smaller; steeper TD at trot | - | SEG-V | M | Fischer et al. 2018 | H |
| Pelvic pitch | strongly biphasic (two cycles per stride) | - | SEG-V (retroversion +) | M | Fischer et al. 2018 | H |
| Pelvic roll | Whippet < 3°; French Bulldog 16° walk, 19.5° trot | - | about craniocaudal axis, biphasic | M | Fischer et al. 2018 | H |
| Femoral abduction in stance | Whippet 6 / 3°, Beagle 2 / 8°, Malinois 14 / 7° (walk / trot); French Bulldog ≈ 18 / 27° | - | about craniocaudal axis | M | Fischer et al. 2018 | H |
| Long-axis rotation, scapula and humerus | ≈ 10° | - | 3D, Beagle | M (cited) | Fischer et al. 2018, citing Andrada et al. 2017 | M |
| Hock in stance | short flexion (yield) just after TD, then extension to TO | - | phase | M | Fischer et al. 2018 | H |
| Pantograph coupling | scapula ∥ forearm; femur ∥ metatarsus. Functional homologues: scapula = thigh, upper arm = shank, forearm = metatarsus | - | whole stride | M (X-ray, 327 dogs, 32 breeds) | Fischer & Lilje 2011, ScienceDaily | H (qualitative) |
| Forelimb proportions | scapula 24-34% of foreleg length (long in short-legged dogs, short in Greyhounds); upper arm a constant share | of foreleg length | - | M | Fischer & Lilje 2011 | M-H |

### 2.3 Dog - joint angles, trot (GSD; walk contrasted where known)

| Joint | Swing ROM | Stance ROM | Convention | When it flexes / extends most | Type | Source | Conf |
|---|---|---|---|---|---|---|---|
| Shoulder | 38.2 ± 4.3° | 24.6 ± 3.6° | INC | Extends through swing to a maximum just before TD; flexes during stance | M | Miao et al. 2026 (4 GSD, preliminary) | M |
| Elbow | 56.0 ± 7.3° | 29.7 ± 5.1° | INC | Swing: flexes then extends (maximum flexion early-mid swing); stance: small extension, then flexion | M | Miao et al. 2026 | M |
| Carpus | 88.7 ± 6.4° | 29.4 ± 4.2° | INC; > 180 = hyperextension | Hyperextends past 180° just before TD and **stays hyperextended through almost all of stance**; releases at end-stance; maximum flexion near mid-swing | M | Miao et al. 2026 | M-H |
| MCP (front toes) | 104.8 ± 9.7° | 38.8 ± 5.9° | INC | Hyperextended through stance; folds deeply in swing (the paw "peels") | M | Miao et al. 2026 | M |
| Hip | 45.3 ± 5.3° | 43.9 ± 5.1° | INC | Flexes in swing (minimum just before TD); extends through stance | M | Miao et al. 2026; Fischer et al. 2018 agrees (40-48°) | H |
| Stifle | 63.5 ± 6.3° | 27.9 ± 4.6° | INC | Swing: rapid flexion then extension; stance: gradual flexion (marker data - fluoroscopy shows only 1-14° stance change) | M | Miao et al. 2026; Fischer et al. 2018 | M (markers overstate the stifle) |
| Tarsus (hock) | 56.5 ± 4.7° | 43.3 ± 6.1° | INC; never > 180 | Flexes after TD (yield), extends to TO, flexes in early swing, extends before TD | M | Miao et al. 2026 | M |
| MTP (hind toes) | 40.6 ± 7.3° | 55.0 ± 8.6° | INC | Hyperextends then extends in stance; no strong swing pattern; hind paw stays flatter than the fore paw | M | Miao et al. 2026 | M |
| Typical absolute ranges, trot, medium dog | shoulder ≈ 105-140, elbow ≈ 85-150, carpus ≈ 95-200, hip ≈ 100-140, stifle ≈ 80-140, tarsus ≈ 110-150 | - | INC | consistent with the ROMs above | R (recalled from marker-based clinical studies of the DeCamp type) | - | L-M |
| Walk vs trot | Joint patterns similar; amplitudes larger at trot; forelimb control the same in both gaits | - | - | - | M (abstract) | Andrada et al. 2017 (Beagle forelimb inverse dynamics) | M-H |
| Labrador vs GSD at trot | Labrador hips stay slightly flexed through the whole stride; Labrador thoracolumbar joint stays extended, GSD's is flexed; lumbosacral flexed in both, more in GSD | - | Humphries flexion-from-neutral convention | breed posture | M | Humphries et al. 2020 | H (qualitative) |
| Marker caveat | Skin markers give near-useless pelvis data, wrong femur amplitude (37° error in French Bulldog) and wrong timing (12%); reliable distal to the stifle | - | - | - | M | Fischer et al. 2018 | H |
| Shoulder caveat | Fluoroscopy shows the gleno-humeral joint "nearly immobile". Marker shoulder ROMs (25-38°) partly reflect scapula and skin motion | - | - | - | M vs M | Fischer & Lilje 2011 vs Miao 2026 | M |

### 2.4 Dog - gallop (Greyhound for exaggeration)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Gait type at speed | rotary gallop, 2 suspensions (gathered after forelimb TO, extended after hindlimb TO) | - | - | M | UMN gaits; Hudson et al. 2012 | H |
| Spine joints that bend | 7 ± 1 presacral intervertebral joints, regardless of thoracic or lumbar label | - | sagittal | M (cineradiography, small mammals) | Schilling & Hackert 2006 | M (not dog) |
| Spine maximum flexion | from the last third of hindlimb swing to TD | - | phase | M | Schilling & Hackert 2006 | M-H |
| Spine maximum extension | in the first third of swing, just after lift-off | - | phase | M | Schilling & Hackert 2006 | M-H |
| Lumbar share of the hind step | ≈ 50% in in-phase gaits (half-bound, bound) | of step length | - | M (small mammals) | Fischer et al. 2002 | M |
| Greyhound lumbar sagittal ROM | no measured dog figure found. Estimate 20-40° total across the lumbar region; cheetahs bend more than greyhounds | - | sagittal | R | inference from cat (below) and small mammals | L |
| Cat gallop spinal segment angle | 118.9-168.0° (≈ 49° excursion) in hindlimb stance; pelvic segment 46-91°; spine and pelvis synchronise as speed rises (3.2-4.3 m/s) | - | angle definition not stated in the abstract | M (abstract, via search) | KAIST, J Bionic Eng 2014 | L-M |
| Role of flexion | flexion brings the hind paws far ahead of the fore paws' prints; extension in hind propulsion produces the leap; abdominals bring the pelvis forward | - | - | M (descriptive) | UMN gaits site | H (qualitative) |
| Tail at gallop | all tail muscles co-contract, holding the tail stable as a counter-torque to the limbs | - | - | M (EMG + film, 7 dogs) | Wada et al. 1993 | M-H |

### 2.5 Cat - walk (forelimb measured; hindlimb mostly qualitative)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Shoulder angle | minimum 86.8 ± 6.6° in early-mid swing (maximum retraction); maximum 124.3 ± 4.5° at end of swing | - | INC (scapula-humerus) | M | Rahmati et al. 2025 (7 cats, 211 cycles) | H |
| Elbow angle | minimum 78.5 ± 7.3° early-mid swing; maximum 113.5 ± 7.5° at end of swing; keeps extending into stance | - | INC | M | Rahmati et al. 2025 | H |
| Note on the elbow | **the cat elbow never straightens at walk** (maximum ≈ 114°): a crouched forelimb, unlike dog or horse | - | - | D | from row above | H |
| Wrist | maximum 232.7 ± 15.8° in early-mid swing ("dorsiflexion" in their terms = paw folded); slight dorsiflexion in stance, then plantarflexion at end-stance | - | Rahmati wrist angle (>180 = folded in swing) | M | Rahmati et al. 2025 | M (convention differs from INC) |
| Forelimb length (shoulder to distal carpals) | 0.137 ± 0.009 m mid-swing to 0.204 ± 0.006 m end of stance (33% shortening); follows the elbow angle | absolute; cat 3.2 kg | - | M | Rahmati et al. 2025 | H |
| Forelimb orientation | 136.4 ± 2.8° at swing onset (most retracted) to 64.2 ± 5.0° at end of swing (most protracted): ≈ 72° total | - | LEG-H | M | Rahmati et al. 2025 | H |
| Segment lengths | scapula 5.8 cm, upper arm 9.3 cm, forearm 9.5 cm, carpals + metacarpals 2.6 cm | absolute, 3.2 kg cat | - | M | Rahmati et al. 2025 | H |
| Peak vertical force, forelimb | 58% body weight, near the end of stance; ≈ 20% higher than the hindlimb's | - | - | M | Rahmati et al. 2025 | H |
| Scapula in posture | vertebral border **above** the thoracic spinous processes; gleno-humeral joint just below the vertebral bodies of T3-T5 | - | standing | M (via search summary) | Macpherson & Ye 1998 | M-H |
| Scapula role | dominant forelimb element; forelimb kinematics largely independent of speed and gait | - | - | M (small therians incl. cineradiography) | Fischer et al. 2002 | M-H |
| Hindlimb joint ranges at walk | hip ≈ 75-120, knee ≈ 70-135, ankle ≈ 70-150 | - | INC | R (recalled; Goslow 1973 full text not read) | Goslow et al. 1973; Halbertsma 1983 | L |
| Hindlimb yield | ankle and knee flex briefly after TD (E2), then extend (E3) | - | phase | M (abstract) | Goslow et al. 1973 | H (qualitative) |

### 2.6 Horse - limbs (walk, trot, canter)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Scapula ROM | 27.6 ± 0.9° walk (n = 2); 23.3 ± 3.7° trot (n = 9) | - | sagittal rotation, fluoroscopy; rotates about an *instantaneous* centre, not a fixed pivot | M (one Mini Shetland pony) | J Equine Vet Sci 2026 (Mad Barn abstract) | M |
| Hip ROM | 29.5 ± 2.3° walk; 33.2 ± 3.8° trot | - | fluoroscopy | M (one pony) | same | M |
| Marker error | skin markers understate elbow and stifle ROM by up to 33° and 28° | - | - | M | same | M |
| Fore fetlock (MCP) ROM | 62 ± 7° walk; 77 ± 5° trot | - | 3D flexion-extension | M (via search summary) | Clayton et al. 2007, VCOT 20:86 | M-H |
| Fore fetlock at mid-stance, ridden trot | 111.2 ± 7.3° | - | INC on the dorsal side (≈ 69° past straight) | M | MacKechnie-Guire et al. 2019 (13 horses) | M |
| Hind fetlock at mid-stance, ridden trot | 113.7 ± 6.0° | - | INC dorsal | M | MacKechnie-Guire et al. 2019 | M |
| Carpus, maximum swing flexion, ridden trot | 92.3 ± 7.9° (≈ 88° from straight) | - | INC | M | MacKechnie-Guire et al. 2019 | M-H |
| Elbow, swing flexion, ridden trot | 103.9 ± 10.8° | - | INC | M | MacKechnie-Guire et al. 2019 | M |
| Hip, maximum swing flexion, ridden trot | 99.8 ± 12.9° | - | INC | M | MacKechnie-Guire et al. 2019 | M |
| Tarsus, maximum swing flexion, ridden trot | 119.9 ± 10.4° | - | INC | M | MacKechnie-Guire et al. 2019 | M |
| Carpus in stance | locked near straight (≈ 180°) through stance; flexes only in swing | - | INC | R (well established) | Back & Clayton | H |
| Walk vs trot | At trot, shoulder, stifle and tarsus are more flexed through stance, and carpus and fetlock more extended at maximum; in swing, elbow, carpus, stifle and tarsus flex more. Curve shapes are the same | - | - | M (via search summary) | Back, Schamhardt & Barneveld 1996 | M-H |
| Flat-walk ROM (gaited yearlings) | carpus 54°, tarsus 40°, fore fetlock 45°, hind fetlock 60° | - | sagittal | M (Tennessee Walking Horse, so exaggerated) | Cambridge, ECEP | L-M for a riding horse |
| Limb protraction / retraction, trot | fore: protraction 22.6°, retraction 6.8°; hind: protraction 9.1°, retraction 22.0° | - | coronet-to-withers (fore) or coronet-to-sacrum (hind) line vs vertical | M (2D video, Criollo, grass) | Kinematic aspects of the Criollo Argentino horse at trot | L-M |
| Leading vs trailing limb, canter | leading limb more protracted, via more elbow, carpal, hip and tarsal flexion; scapula more rotated in the trailing forelimb | - | - | M (abstract) | Utrecht canter leading/trailing study | M |
| Hoof swing height, hind, trot | 5.4 ± 1.4 cm peak (unshod, no devices); 14-24 cm with stimulators or weights | absolute; ≈ 3-4% of withers height | mid-swing | M | 2011 hind-foot stimulation device study (8 horses) | M |
| Fore hoof swing height, trot | higher than the hind (carpus folds ≈ 88°): roughly 10-20 cm in a warmblood | - | mid-swing | R | general | L |
| Hind hoof placement, walk | hind hoof lands in or past the fore print of the same side ("tracking up", "overtrack" in a free walk) | - | - | R (craft and judging) | general / Muybridge | M |

### 2.7 Trunk motion (all families)

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Horse walk vertical ROM (in hand) | head 104.7 (Iberian), 79.3 (Icelandic), 117.1 mm (Warmblood); withers 26.8 / 36.0 / 33.4 mm; pelvis 68.0 / 65.8 / 78.8 mm | EMM, mm | - | M | Rhodin et al. 2022 (68 horses) | H |
| Horse trot vertical ROM (in hand) | head 63.7 / 60.7 / 78.1 mm; withers 66.2 / 46.8 / 92.2 mm; pelvis 70.8 / 56.0 / 89.1 mm | mm | - | M | Rhodin et al. 2022 | H |
| Same, normalised | walk withers ≈ 2%, pelvis ≈ 4-5% of withers height; trot withers ≈ 3-6% | % of withers height (≈ 1.45-1.65 m) | - | D | from rows above | M |
| Speed effect | head, withers and pelvis ROM *fall* as speed rises within a gait | - | - | M (cited) | Rhodin et al. 2022 | H |
| Horse trot timing | withers minimum ≈ 39-42% of forelimb stance, maximum ≈ 91-98%; pelvis minimum ≈ 37-45% of hindlimb stance, maximum ≈ 92-110%; head minimum 39-46%, maximum 92-97% | % of the limb's stance | minimum ≈ mid-stance, maximum ≈ start of suspension | M | Rhodin et al. 2022, Table 4 | H |
| Horse walk timing | head minimum ≈ 46-48% of each forelimb stance, maximum ≈ 86-88%; withers minimum ≈ 4-13% of forelimb stance (both forelimbs on the ground, spread), maximum ≈ 42-48% (mid-stance); pelvis minimum ≈ 8-11% of hindlimb stance (both hindlimbs on the ground), maximum ≈ 51-57% (mid-stance) | % of stance | 2 cycles per stride | M | Rhodin et al. 2022 | H |
| Horse walk relative phase | head and pelvis in phase; withers ≈ 25% of a stride out of phase with both | - | - | M | Rhodin et al. 2022 | H |
| Horse body centre | ≈ 69-71 cm below the sacral marker at walk and trot (Haflinger) | - | - | M (via search) | Croatian treadmill study (hrcak) | M |
| Dog trot vertical (withers or pelvis) | ≈ 3-5% of withers height (Labrador ≈ 17-30 mm), by dynamic similarity with the horse trot at a similar Fr and duty factor | - | min mid-stance, max start of suspension | D | horse rows and Andrada 2023 Fr range | L-M |
| Dog walk vertical | withers small (≈ 1-2% of height), pelvis larger, by analogy | - | - | D / R | horse rows | L |
| Horse thoracolumbar flexion-extension | walk ≈ 4°, canter ≈ 9° (unridden, on a circle); trot smallest of the three in most segments | - | sagittal ROM | M (via search summaries) | 2023 Vet Res Commun study; review of Faber 2000/2001 | M |
| Horse lumbosacral flexion-extension, canter | 6.1° (6 m/s), 6.3° (6.5), 6.6° (7.0), 7.2° (8.0 m/s), linear with speed | - | sagittal ROM | M | Hartpury Thoroughbred treadmill abstract | M-H |
| Horse lateral bending | similar across gaits; treadmill reduces it by ≈ 1.7-1.8° at two lumbar angles | - | ROM | M (via search) | Faber review; Gómez Álvarez 2009 | M |
| Horse sagittal back motion | two cycles per stride at walk and trot | - | - | M | Layer 2012 (cites the horse pattern) | H |
| Dog lateral bending | **one cycle per stride**; rises from cranial to the thoracolumbar junction, falls toward the sacrum; at walk the peak travels cranial to caudal; at trot it is synchronous | - | transverse plane | M | Layer 2012 (Labrador n = 15, Dachshund n = 13) | H (qualitative) |
| Dog sagittal bending | two cycles per stride; at walk the peak travels caudal to cranial; largest ROM at the sacrum; mid-thoracic ROM rises from walk to trot | - | sagittal | M | Layer 2012 | H (qualitative) |
| Dog lateral vs sagittal | lateral ROM ≥ sagittal ROM at walk and trot | - | - | M | Layer 2012 | H |
| Dog walk-trot change | Dachshund lateral ROM falls at trot; Labrador keeps its walking lateral ROM | - | - | M | Layer 2012 | H |
| Dog trunk bending wave | sequential back-muscle activity at walk and gallop, synchronous at trot (travelling wave at walk) | - | - | M (EMG, cited) | Schilling & Carrier (cited in JEB 2010) | M-H |
| Dog hip mediolateral sway | larger in the Labrador than in the Dachshund | - | - | M | Layer 2012 | M |
| Small-mammal and cat gallop | see 2.4 | - | - | - | - | - |

### 2.8 Head, neck and tail

| Quantity | Value / range | Normalisation | Convention / phase | Type | Source | Conf |
|---|---|---|---|---|---|---|
| Horse head vertical ROM | walk 79-117 mm; trot 61-78 mm | mm | - | M | Rhodin et al. 2022 | H |
| Horse walk nod | two per stride; head lowest near mid-stance of each forelimb (≈ 47% of its stance), highest ≈ 87% | % of forelimb stance | - | M | Rhodin et al. 2022 | H |
| Horse nod and energy | timing consistent with minimum metabolic cost in inverted-pendulum walking; shifting it raises head-neck cost by up to 63%; nod raises forelimb load in single support and lowers it in double support | - | - | M (model + kinematics) | Loscher et al. 2016, Proc R Soc B | H (qualitative) |
| Horse nod vs withers | head excursion exceeds the withers' (walk: ≈ 3× in mm) | - | - | M | Loscher et al. 2016; Rhodin 2022 | H |
| Horse head angular stabilisation | head pitch and angular velocity held small (smaller than in primates) at walk, trot and canter | - | - | M (abstract) | Dunbar et al. 2008, JEB 211:3889 | M |
| Horse canter head-neck | head and neck rise and fall once per stride, roughly counter to trunk pitch | - | phase | R | rider literature | L |
| Horse canter trunk | sagittal trunk oscillation amplitude rises with diagonal dissociation | - | - | M (abstract) | canter-transition study (via search) | L-M |
| Dog head at trot | head lowest at mid-stance of the diagonal forelimb (the clinical "head nod" depends on it); sound asymmetry ≈ 1-3 mm | - | - | M (asymmetry only) | Bergh et al. 2018 | M |
| Dog head carriage at trot | head lowered toward withers level, neck more horizontal than at walk; trotting ROM about 2-4 cm | - | - | R | observation | L |
| Dog tail, walk and trot | left and right tail muscles fire reciprocally, giving lateral tail swing | - | once per stride (follows the lateral bending cycle) | M | Wada et al. 1993 | H (qualitative) |
| Dog tail base | moves **opposite** to the rest of the spine in the transverse plane (balancing) | - | anti-phase | M | Layer 2012 | H (qualitative) |
| Dog tail heterogeneity | very variable even within a breed; it clearly couples to back motion | - | - | M | Layer 2012 | H |
| Cheetah tail | forward tail rotation counters pitch in braking; a conical swing gives continuous roll torque in turns | - | - | M (motion capture) | Patel et al. (UCT); "Tails, Flails, and Sails" review | M |

---

## 3. Notes: what the numbers do not capture

**Rig topology**
- **The forelimb has no hip.** There is no clavicle in dogs or horses and only a vestige in cats. The scapula hangs in a muscular sling (serratus ventralis, pectorals) and both rotates and slides on the rib cage. Its pivot is an instantaneous centre that moves (pony fluoroscopy; Fischer's group).
  - The dog forelimb works as a near-strut at the shoulder: retractors are quiet in stance, and the ground force passes close to the shoulder's fulcrum (Carrier et al. 2006, 2008).
  - The forelimb therefore needs a scapula bone with rotation, plus dorsal-ventral and cranial-caudal translation, about a pivot high on the blade. The gleno-humeral angle should change little in stance.
  - Pinning the forelimb at a fixed shoulder joint and swinging it like a human arm is the most likely single cause of the owner's "unnatural" verdict.
- **The withers are made of scapulae.** In cats the vertebral borders ride above the spinous processes (Macpherson & Ye 1998). In a slow cat walk the weight-bearing side's blade visibly rises and the swing side's drops: the stalking "shoulder roll".
- **Scale stride by hip height.** Hip height is the length Alexander's relations use. A horse's withers stand far above its hip joint and well above its shoulder joint, so feeding withers height into Fr and λ inflates a horse's stride by about 30%.

**Biomechanist checks**
- **Retraction exceeds protraction.** Dogs: TD ≈ 16-22° ahead of perpendicular, TO ≈ 14-33° behind. The leg lands steeper than it leaves, and steeper again at trot than at walk.
- **The hind paw overtakes the fore print at walk.** Lateral-sequence walk: LH, LF, RH, RF, with each hind landing about a quarter-stride before its same-side fore. The craft description is that the back leg "kicks" the front leg forward.
- **Trot diagonals are not perfectly simultaneous.** Normal horses land the fore slightly first (fore-first dissociation, Rhodin 2022). In warmbloods the hind may land first, which is judged a quality trot. A dissociation of a few percent of the stride removes the mechanical look.
- **Pace** (lateral pairs) is used by some dogs when tired or long-legged, and by camels and gaited horses. Its vertical minima come early, at ≈ 27% of stance (Icelandic pace, Rhodin 2022), not at mid-stance as in trot.
- **Canter is asymmetric.** Leading and trailing limbs differ in joint flexion, and in scapula rotation (more in the trailing fore). A mirrored "symmetric canter" reads wrong.
- **Breed-shape effects are real and directional.**
  - French Bulldog-like (wide, short): abducted hindlimbs, large pelvic roll (16-19°), femoral rotation above 30°, shorter retraction.
  - Whippet-like: near-parasagittal, pelvic roll under 3°.
  - Dachshund-like (long-backed): lateral bending drops at trot.
  - For crosses, interpolating these parameters is better than averaging joint curves.
- **Cats are crouched.** The elbow stays at or below ≈ 114° even at end-swing extension. Cats walk with a high duty factor (0.64) and long, slow cycles relative to their size.

**Animator rules** (Muybridge's plates; Williams's "Animator's Survival Kit" animal-walk, running-dog and horse sections; Don Kim / Animation Mentor 2025; Frey, GDC 2016)
- **Down and up positions.** In a walk the body is lowest when a limb pair is spread (double support) and highest over the supporting leg (mid-stance). The withers and pelvis do this out of phase (≈ 25% at walk, horse data); at trot they are synchronous. One sine on the root node, shared by front and back, is the classic robotic tell.
- **Weight shift.** Hips and shoulders tilt toward the weight-bearing side, and the clavicle or scapula on that side rises in the pass position (Kim). The lateral bend is one wave per stride.
- **Overlap and follow-through.**
  - The head lags and leads in a staggered way.
  - The tail is offset joint by joint, so a wave travels down it, with its base anti-phase to the spine.
  - Ears and jowls follow on springs.
  - All of these are timed to the hips (Kim) and to the forelimb stance (head nod).
- **Paw articulation.** The front paw peels off with a strong carpal and toe break (carpus ≈ 90° of flexion, MCP ≈ 105° of swing ROM). The hind paw plants flatter, with less break and some toe drag (Kim; Miao 2026).
- **Separate front and back IK chains.**
  - Pelvis and shoulder girdle correct independently on uneven ground, and the spine between them depends on all four feet (Town 2025).
  - One rigid chassis with four IK legs looks like a table.
  - A rigid-spine and a procedural-spine character can each blend gaits without transition clips if the spine layer is driven by gait phase (Frey, GDC 2016).
- **No limb-length pumping in stance.** Real effective legs lengthen slightly (1-4%) during stance and shorten about a third in swing. An IK that compresses the stance leg like a spring makes a bouncy cartoon trot. Keep stance near full length and put the shortening in swing via the elbow, carpus and hock.

**Gaps in this research**
- No measured paw swing height for dogs or cats in mm. The 33% cat forelimb shortening in swing is the best proxy.
- No measured dog scapula rotation in degrees (only "dominant").
- No Greyhound spine ROM.
- No horse head-neck pitch amplitude at canter.
- No cat hindlimb joint table (Goslow 1973 full text not read).

---

## 4. Sources

Read in full or in large part (via Europe PMC full-text XML or the publisher page):
- Fischer MS, Lehmann SV, Andrada E (2018). Three-dimensional kinematics of canine hind limbs: in vivo, biplanar, high-frequency fluoroscopic analysis of four breeds during walking and trotting. Sci Rep 8:16982. https://www.nature.com/articles/s41598-018-34310-0
- Andrada E, Hildebrandt G, Witte H, Fischer MS (2023). Positioning of pivot points in quadrupedal locomotion: limbs global dynamics in four different dog breeds. Front Bioeng Biotechnol 11:1193177. https://doi.org/10.3389/fbioe.2023.1193177 (PMC10360120)
- Miao H, Zhu L, Zheng L, Qian Z, Ren L (2026). Comparative kinematic analysis of forelimb and hindlimb cushioning strategies in German Shepherd dogs. Front Vet Sci 13:1878605. https://doi.org/10.3389/fvets.2026.1878605 (PMC13384870)
- Rhodin M, Smit IH, Persson-Sjodin E, et al. (2022). Timing of vertical head, withers and pelvis movements relative to the footfalls in different equine gaits and breeds. Animals 12:3053. https://doi.org/10.3390/ani12213053 (PMC9657284)
- Rahmati SM, Klishko AN, et al., Prilutsky BI (2025). Role of forelimb morphology in muscle sensorimotor functions during locomotion in the cat. J Physiol 603:447-487. https://doi.org/10.1113/jp287448 (PMC11737544)
- MacKechnie-Guire R, et al. (2019). The effect of tree width on thoracolumbar and limb kinematics... in sports horses in trot and canter. Animals 9:842. https://doi.org/10.3390/ani9100842 (PMC6827167)
- Humphries A, Shaheen AF, Gómez Álvarez CB (2020). Biomechanical comparison of standing posture and during trot between German shepherd and Labrador retriever dogs. PLoS One 15:e0239832. https://doi.org/10.1371/journal.pone.0239832
- Bergh A, Gómez Álvarez CB, Rhodin M, Gustås P (2018). Head and pelvic vertical displacement in dogs with induced swinging limb lameness. Acta Vet Scand 60:81. https://doi.org/10.1186/s13028-018-0435-z (PMC6311055)
- Layer AF (2012). Ganganalytische Untersuchung der Rückenbewegung von gesunden Hunden der Rassen Dackel und Labrador Retriever. Dissertation, LMU München. https://edoc.ub.uni-muenchen.de/15120/
- Friedrich Schiller University Jena / Fischer MS, Lilje KE (2011). Insights into the motion of dogs (press release on "Hunde in Bewegung", Kosmos 2011). https://www.sciencedaily.com/releases/2011/05/110527080325.htm
- Equine upper limb kinematics: biplanar high-speed fluoroscopic kinematography vs optoelectronic motion capture in one Mini Shetland Pony (2026). J Equine Vet Sci 106094. Abstract: https://madbarn.com/research/equine-upper-limb-kinematics-comparison-between-biplanar-high-speed-fluoroscopic-kinematography-and-optoelectronic-motion-capture-in-one-mini-shetland-pony/
- University of Minnesota Veterinary Anatomy, Gaits: Rotatory Gallop. https://vanat.ahc.umn.edu/gaits/rotGallop.html
- Kim D / Animation Mentor (2025). Tutorial: How to Animate a Quadruped Walk Cycle. https://www.animationmentor.com/?p=45178
- Town B (2025). Deep Dive: Rigging and animating a quadruped (part 1). https://brycetown.substack.com/p/deep-dive-rigging-and-animating

Abstract-level or search-summary only (lower confidence; numbers as quoted in summaries):
- Fischer MS, Schilling N, Schmidt M, Haarhaus D, Witte H (2002). Basic limb kinematics of small therian mammals. J Exp Biol 205:1315-1338. https://journals.biologists.com/jeb/article/205/9/1315/8896/Basic-limb-kinematics-of-small-therian-mammals
- Schilling N, Hackert R (2006). Sagittal spine movements of small therian mammals during asymmetrical gaits. J Exp Biol 209:3925-3939. https://doi.org/10.1242/jeb.02400
- Goslow GE, Reinking RM, Stuart DG (1973). The cat step cycle: hind limb joint angles and muscle lengths during unrestrained locomotion. J Morphol 141:1-41.
- Andrada E, et al. (2017). Three-dimensional inverse dynamics of the forelimb of Beagles at a walk and trot. Am J Vet Res. https://agris.fao.org/search/en/records/65dff24c63b8185d9cb0352b
- Carrier DR, Deban SM, Fischbein T (2006). Locomotor function of the pectoral girdle "muscular sling" in trotting dogs. J Exp Biol 209:2224-2237. https://carrier.biology.utah.edu/Extrinsic.html
- Hudson PE, Corr SA, Wilson AM (2012). High speed galloping in the cheetah and the racing greyhound: a spatio-temporal analysis. J Exp Biol 215:2425-2434. https://doi.org/10.1242/jeb.066720 ; SICB abstract https://sicb.org/?p=27128 ; https://www.sciencedaily.com/releases/2012/06/120621070913.htm
- Alexander RMcN, Jayes AS (1983). A dynamic similarity hypothesis for the gaits of quadrupedal mammals. J Zool 201:135-152. https://doi.org/10.1111/j.1469-7998.1983.tb04266.x ; Alexander RMcN (1984). The gaits of bipedal and quadrupedal animals. Int J Robotics Res 3:49-59 (λ/h formula recalled, not re-read).
- Clayton HM, et al. (2007). 3D kinematics of the equine metacarpophalangeal joint at walk and trot. Vet Comp Orthop Traumatol 20:86-91. https://thieme-connect.com/products/ejournals/abstract/10.1160/VCOT-07-01-0011
- Back W, Schamhardt HC, Barneveld A (1996). Kinematic comparison of walk and trot (warmblood). Via search summary; see also https://research-portal.uu.nl/en/publications/kinematic-differences-between-the-distal-portions-of-the-forelimb/
- Faber M, et al. (2000, 2001). Basic three-dimensional kinematics of the vertebral column of horses walking / trotting; equine spine during canter. Am J Vet Res 61:399, 62:757; Equine Vet J 33:145. https://research-portal.uu.nl/en/publications/basic-three-dimensional-kinematics-of-the-vertebral-column-of-hor/
- Unridden horses on a circle, thoracolumbar ROM (2023). Vet Res Commun. https://link.springer.com/article/10.1007/s11259-023-10132-y
- Lumbosacral flexion-extension ROM vs canter speed, Thoroughbreds (Hartpury). https://pure.hartpury.ac.uk/en/publications/the-relationship-between-range-of-motion-of-lumbosacral-flexion-e/
- Loscher DM, et al. (2016). Timing of head movements is consistent with energy minimization in walking ungulates. Proc R Soc B 283:20161908. https://pmc.ncbi.nlm.nih.gov/articles/PMC5136594
- Dunbar DC, et al. (2008). Stabilization and mobility of the head, neck and trunk in horses during overground locomotion. J Exp Biol 211:3889. https://cob.silverchair.com/jeb/article-split/211/24/3889/18018/Stabilization-and-mobility-of-the-head-neck-and
- Wada N, Hori H, Tokuriki M (1993). EMG and kinematic studies of tail movements in dogs during treadmill locomotion. J Morphol. https://pubmed.ncbi.nlm.nih.gov/8411184
- Macpherson JM, Ye Y (1998). The cat vertebral column: stance configuration and range of motion. https://gwern.net/doc/cat/biology/1998-macpherson.pdf
- KAIST group (2014). The role of relative spinal motion during feline galloping for speed performance. J Bionic Eng. https://koasas.kaist.ac.kr/handle/10203/194741
- Cat gait study (walk/trot speeds). Acta Neurobiol Exp. https://ane.pl/index.php/ane/article/download/266/266/288
- Kinematic aspects of the Criollo Argentino horse at trot. https://cerac.unlpam.edu.ar/index.php/veterinaria/en/article/view/6373
- Two-dimensional kinematics of the flat-walking Tennessee Walking Horse yearling. https://resolve.cambridge.org/core/journals/equine-and-comparative-exercise-physiology/article/twodimensional-kinematics-of-the-flatwalking-tennessee-walking-horse-yearling/2D5F507DA23E3239A4E213E3D2E6A0AC
- Evaluation of biomechanical effects of four stimulation devices placed on the hind feet of trotting horses (2011). https://agris.fao.org/search/en/records/65df530bb766d82b180130a7
- Thoroughbred gallop duty factor and stance times (RVC). https://rvc-repository.worktribe.com/OutputFile/1381163 ; https://pmc.ncbi.nlm.nih.gov/articles/PMC6680649
- Horse walk review table. https://intechopen.com/chapters/83285
- Horse walk-trot transition Fr ≈ 0.35 (2016 review, via search). https://eprints.whiterose.ac.uk/201823
- Patel A, et al. Cheetah tail (UCT). https://news.uct.ac.za/article/-2015-08-31-using-cheetah-movements-to-develop-search-and-rescue-robot-uct-study ; Tails, Flails, and Sails (review). https://www.ncbi.nlm.nih.gov/pmc/articles/PMC8633431/
- Frey G (2016). Animating Quadruped Characters in The Flame in The Flood. GDC. https://gdcvault.com/play/1023209/Animating-Quadruped-Characters-in-The
- Williams R (2001/2009). The Animator's Survival Kit, sections "Basic animal walk pattern", "Action on a running dog", "How does a horse really walk?" (contents only; text not read).
- Muybridge E (1899/1957). Animals in Motion (plates; cited for craft observations only).
- UCSD CSE169 lecture 13 (duty factor notes). https://cseweb.ucsd.edu/classes/wi17/cse169-a/slides/CSE169_13.pdf
