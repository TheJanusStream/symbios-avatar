# Locomotion and flight of winged animals for a procedural rig (bird, bat, glider)

*A point-in-time research dossier gathered for the winged-family proposal (Overlands #1859, 2026-11-05; its companion on morphology, surfaces and rest postures is research-10). Kept as the record of what was found and why the decisions it recommends were made. Every figure carries its own confidence; a value recalled from a paywalled paper, or derived here, is marked so and needs checking before anything hard-codes it.*

Scope: how a bird (crow or pigeon), a bat (fruit bat and microbat) and a gliding mammal (sugar glider, flying squirrel, colugo as the long-glide end) move on the ground, take off, flap, glide, land and rest. Morphology at rest (bone lengths, wing planform) is out of scope beyond the masses and spans needed by the formulas.

Conventions used throughout:
- **Type**: M = measured (a number from a study), D = derived here from measured numbers (the arithmetic is shown), R = rule of thumb or a recalled textbook or craft figure.
- **Confidence** (as research-07 defines it): **H** means a measured value from a peer-reviewed study read in this session (full text or a quoted abstract). **M** means measured, but with a caveat about method or sample, or read only through a secondary summary of the study. **L** means a rule of thumb, a figure derived here (marked "D"), or background textbook knowledge not verified in this session (marked "bg").
- **Source keys** (S1, S2, ...) point to section 9.
- **Fr** = v^2 / (g h), with h = hip-joint height (birds: hip to ground standing, as research-08 uses).
- **Stroke amplitude** = the angle the shoulder-to-wrist (or shoulder-to-tip) line sweeps from the top to the bottom of the stroke, in the stroke plane.
- **Stroke-plane angle** = the tilt of the plane the wingtip sweeps through, from the horizontal (birds) or from the body's long axis where the row says so.
- **Downstroke ratio (DR)** = downstroke time / whole wingbeat time.
- **Glide ratio** = horizontal distance / height lost (= 1 / tan(glide angle)).
- **BW** = body weights (a force divided by m g).
- Reference animals and the masses and spans used in the arithmetic (bg, L): carrion or American crow 0.45-0.6 kg, span 0.9-1.0 m, wing area 0.15 m^2; feral pigeon 0.3-0.45 kg, span 0.65 m, area 0.065 m^2; short-nosed fruit bat *Cynopterus brachyotis* 30-40 g, span 0.35-0.4 m; Seba's short-tailed bat *Carollia* 15-20 g; microbat *Myotis* or *Pipistrellus* 5-10 g, span 0.2-0.25 m; large flying fox *Pteropus* 0.5-1 kg; sugar glider 0.1-0.16 kg; northern flying squirrel 0.11-0.23 kg; colugo 1-2 kg.

---

## 1. Summary: the twelve facts that most change whether a winged avatar reads as natural

1. **Wingbeat frequency falls with size at about the -1/4 to -1/3 power of mass, and birds beat faster than bats of the same mass.** Bats: f ∝ M^-0.26 across 65 species from 2 to 870 g, and M^-0.30 in free flight; birds about M^-0.27 (S4, M). Pennycuick's fitted bird formula f = m^(3/8) g^(1/2) b^(-23/24) S^(-1/3) ρ^(-3/8) gives **≈ 4.6 Hz for a crow and ≈ 7.7 Hz for a pigeon** (S1, D). The bat regression fw = 5.54 - 3.068 log10 m - 2.857 log10 V (m in kg, V in m/s) gives **≈ 8 Hz for a 35 g fruit bat and ≈ 10 Hz for an 8 g microbat at 5 m/s, ≈ 3.6 Hz for a 0.6 kg flying fox** (S4, D).
2. **In a bird's ground takeoff the legs supply most of the speed at lift-off: 80-90%** in starling and quail (S10, M), **94-95%** in zebra finch and dove (S11, M), but only 59% in hummingbirds. Over the whole takeoff, wings take over fast: in pigeons the legs gave only **about a quarter of the acceleration** across takeoff, and the **second wingbeat** was the strongest (S13, M). Animate a crouch-and-spring with the wings already rising, then two or three big, fast, near-full-amplitude beats.
3. **A vampire bat jumps with its arms, not its legs.** The forelimbs and thumbs push for about 30 ms to a peak of **9.5 BW**, giving a takeoff speed of **2.38 ± 0.24 m/s** (S9, M). That is a ballistic rise of about 0.29 m (D). Its ground gait is a forelimb-driven bound at **0.56-1.14 m/s**, with a lateral-sequence walk at **0.12-0.28 m/s** (S8, M).
4. **Most bats crawl on the wrists and thumbs with the wings folded, in a lateral-sequence walk** (S8, M). The vampire bat and the New Zealand short-tailed bat *Mystacina* are the agile ground specialists. Most bats drop from a hanging roost into flight. Free-tailed bats can take off from flat ground, though: 98.7% of 600 managed it directly (S30, M).
5. **Head-bob: one cycle per step, two per stride.** The head holds still in space (it drifts about **3 mm/s**) while the body walks under it, then thrusts at **500-800 mm/s** (S17, M). The thrust takes about half of each step (S17, M via review). The hold starts just after one foot lifts and the thrust starts just before the next foot lands (Fujita, S17, M). The hold is visually triggered and **vanishes at a fast run and on landing**. Pigeons, fowl, cranes, herons and corvids bob. Ducks, geese, gulls, raptors and parrots do not, and small passerines hop.
6. **A corvid walks below about 1 m/s and above that either runs or, by preference, hops** (magpie, S18, M). A hop is a phase-shifted run: its stride frequency barely changes with speed, while its step length and duty factor match the run. Swing time is independent of speed. Birds change from walk to run smoothly, without a jump in duty factor, near **Fr ≈ 0.5**, often into a "grounded run" with no aerial phase (S19, M).
7. **Upstroke style is a speed-and-size dial.** At slow speeds long-winged birds (pigeon) use a **tip-reversal** upstroke. At cruise the wing is **flexed** at wrist and elbow. Small round-winged birds flex at all speeds (S14, S15, M-L). Bats flex strongly in a slow-speed upstroke with a **"backward flick"** that disappears above about **3.2 m/s** (S5, M). Their kinematics change gradually with speed, with no sudden gait switch.
8. **Bats steer with wing inertia.** Their wings are heavy: the wing-to-body moment-of-inertia ratio is about **5** in *Carollia*, against 0.02 in a fruit fly. Folding one wing in the upstroke rolls the body **about 40 deg per wingbeat**. Asymmetric fore-aft sweep between up- and downstroke pitches it **about 60 deg per wingbeat** (S6, H, model fitted to measured flights). A ceiling landing takes about **4-5 wingbeats**: about 0.25 s of steady climb at body pitch ≈ 60 deg, then about 0.25 s of manoeuvre (S6, H).
9. **Ceiling landings come in two kinds** (S7, H): a **four-point** landing (thumbs and feet hit together; *Cynopterus*; **3.6 BW**) and a **two-point** landing (feet only, legs swung up beside the body; *Carollia*, *Glossophaga*; **0.74 BW**). The roll-and-pitch flip is the animation the player will remember.
10. **Glide ratio is only about 2 in sugar gliders and flying squirrels.** Northern flying squirrel, field: **1.98** (glide angle 26.8 deg) (S23, M). Inland sugar glider, field: **2.2** (26.4 deg) over 40 glides averaging 18.1 m (S24, H); 1.8 (29.7 deg) in an older review (S24, M). Colugo typical glides: about **30 m for 8 m of height, ≈ 3.75** (D from S22), up to about 150 m. A soaring bird is 10-20 (bg, L). Flying squirrel glide speeds are **6.3-8.1 m/s** (S23, M). Glides are not in equilibrium: speed and force coefficients change throughout (S23, H).
11. **Every glider stalls into its landing, and how well it stalls sets the impact.** Flying squirrel landing forces run **3-10 BW** and rise with glide distance. At approach angles near **45 deg** the animal cannot pitch up enough and lands **forelimbs first** at higher force (S20, M). Colugos cut speed by **about 60%** by changing posture just before touchdown, and land more softly after longer glides (S21, M).
12. **A bird's landing is braked by the wings, not the legs.** The wings do **3-10 times** more work in the last beats than the legs absorb on contact. Peak leg force on landing is about **50%** of that on takeoff (S12, M). In a pigeon, body and tail rotate from near-horizontal at takeoff to **near-vertical** at landing (body pitch rises about **80 deg**). The stroke plane tilts down for takeoff and up for landing, and amplitude is larger in both than in cruise (S13, M).

---

## 2. Ground locomotion

### 2.1 Bird: walk, hop, run

| Quantity | Value / range | Convention | Type | Source | Conf |
|---|---|---|---|---|---|
| Who hops, who walks | Small passerines usually hop. Larger passerines (magpie, crows) both hop and walk with a head-bob. Columbiformes, Galliformes, Gruiformes and Ciconiiformes walk and bob. Anseriformes, raptors, owls, parrots, flamingos and pelicans walk without bobbing | - | M (review of 322 spp, Jiménez Ortega 2005, via Necker) | S17 | M-H |
| Why a bird hops | Short legs relative to body and a perching (arboreal) habit; a hop is two synchronous legs, a walk two alternating legs. Song sparrows use every gait, chosen by substrate and speed | - | R + M (field note) | S17, Wilson Bull. note | L |
| Magpie gait choice | Walk below about 1 m/s; above that, run or hop, with hopping preferred; speeds studied 0.4-3 m/s | - | M (abstract) | S18 | H |
| Magpie hop vs run | Hop stride frequency barely changes with speed; step length and duty factor match the run's; swing duration independent of speed; joint flexion and extension patterns comparable across gaits; "a simple phase shift" turns a run into a hop | - | M (abstract) | S18 | H (qualitative) |
| Hop body sequence | Phase 1: legs fold, head retracts. Phase 2: head and legs extend together and the bird leaves the ground. Before touchdown the head retracts again | - | M (Daanje 1951, via review) | S17 | M |
| Hop height and length, small passerine | Height about 0.5-1 body length, length about 1-2 body lengths per hop at a forage pace | - | R (no measured figure found) | bg | L |
| Walk-run transition | Smooth in birds (no abrupt fall in duty factor or step length, unlike humans), near Fr ≈ 0.5; walking and grounded running both seen at Fr ≈ 0.5-0.6 | Fr on hip height | M (via later citations) | S19 | M |
| Grounded running | Running mechanics with duty factor > 0.5 and no aerial phase; Japanese quail bounce across 0.35-1.4 m/s; quail leg stiffness normalised about 6 in walk, grounded run and run | - | M | S19 | M |
| Ground-bird run | Helmeted guinea fowl modelled and measured 1.3-3.1 m/s; with a hip height of about 0.3 m (bg), that is Fr ≈ 0.6-3.3 | Fr | D | S19 | L-M |
| Walk duty factor | Similar to or a little below human walking (human about 0.6); larger than human in running | - | M (via review) | S19 | M |
| Stride length vs leg | Use Alexander's λ/h ≈ 2.3 Fr^0.3 with h = hip height, as research-08 does for quadrupeds; Gatesy & Biewener found that Froude normalisation removes most size effects, leaving posture and segment ratios | - | R | S19; research-08 | M-L |
| Trunk carriage | Trunk near horizontal (pronograde), centre of mass ahead of the hip; femur held sub-horizontal and swings little, so the step comes mostly from the knee and the tarsometatarsus | - | R (Gatesy 1990, bg) | bg | L-M |
| Running head | Fast-running birds hold the neck stretched forward continuously; no hold phase | - | M (review) | S17 | M |

### 2.2 Bird: head-bob

| Quantity | Value | Phase | Type | Source | Conf |
|---|---|---|---|---|---|
| Cycles | 1 head cycle per step, 2 per stride | - | M | S17 | H |
| Hold-phase head speed | ≈ 3 mm/s forward drift (a small slip, the visual error signal) | hold | M (Frost 1978; Troje & Frost 2000) | S17 | H |
| Thrust-phase head speed | ≈ 500-800 mm/s | thrust | M | S17 | H |
| Thrust share | ≈ 50% of each step | - | M (via 2017 review) | S17 | M |
| Bob frequency range | ≈ 2-10 Hz across species | - | M (via review) | S17 | M |
| Timing against the feet | Hold begins slightly after a foot lifts off; thrust begins slightly before the next touchdown; timing varies a lot from step to step | - | M (Fujita 2002, 2003) | S17 | M |
| Mechanism | Hold: the head keeps a fixed world position while the body moves under it, so it moves backward relative to the body. Thrust: the neck extends quickly to a new hold point. Head excursion scales with stride length; birds that take short steps do not bob | - | M | S17 | H |
| Trigger | Visual: no bob on a treadmill, none when blindfolded, a bob when carried forward by hand | - | M | S17 | H |
| Speed limit | The hold shortens with speed and disappears at a fast run (whooping crane) and during landing (pigeon) | - | M | S17 | H |
| Rig rule | Drive head world-position with a hold-and-thrust curve keyed to the foot phase; the thrust ends where the head would be at mid-stance of the next step. Fade the hold to zero as Fr rises past about 0.5 | - | D | from rows above | M |

### 2.3 Bat on the ground

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Contact points | Folded wing carried on the wrist and thumb (pollex claw) in front, foot behind; hindlimbs rotated so the knees point up and back | R (bg) | bg | M |
| Gait pattern | Symmetrical lateral-sequence walk in *Desmodus* and *Mystacina* | M (via later citation) | S8 | M |
| Vampire walk speed | 0.12-0.28 m/s | M (conference abstract) | S8 | M |
| Vampire bound speed | 0.56-1.14 m/s, both gaits used at intermediate speed; at least twice as fast possible on a larger treadmill (Riskin) | M | S8 | M |
| Vampire bound mechanics | Asymmetrical; the forelimbs, not the hindlimbs, produce the propulsive force | M | S8 | H (qualitative) |
| *Mystacina* | One gait, kinematically a walk but kinetically run-like at all speeds | M | S8 | M |
| Other bats | Most bats are slow and clumsy on the ground; they climb up a surface with thumb claws and feet, then drop into flight | R + M | S30, bg | L-M |
| Ground takeoff, free-tailed bat | 592 of 600 (98.7%) took off directly from the ground; the rest after climbing up to 30 cm of trunk | M | S30 | M |

### 2.4 Glider on the ground and on a trunk

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Ground gait | A squirrel-type half-bound and bound: the hindlimbs land together ahead of the forelimbs; the patagium bunches along the flank | R | bg | L |
| Climbing a trunk | Alternating or bounding climb with claws; the head-up gallop on a vertical trunk; head-down descent with the hind feet reversed (squirrels) | R | bg | L |
| Sugar glider | Arboreal quadruped; a crouched, sprawled posture; the patagium runs from the fifth finger to the first toe | R | bg | L |
| Flying squirrel | Patagium from the wrist (styliform cartilage extends the leading edge) to the ankle; flattened tail | R | bg | M |

---

## 3. Takeoff

### 3.1 Bird: leap and flap

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Leg share of lift-off speed, starling and quail | 80-90% | M (abstract and secondary) | S10 | M-H |
| Leg share, zebra finch and diamond dove | 94-95% at lift-off | M (via citing paper) | S11 | M |
| Leg share, hummingbird | 59% | M | S10 (cited in hummingbird study) | M |
| Leg share of acceleration over the whole takeoff, pigeon | About 1/4; the wings supply the rest, with the greatest acceleration in the **second wingbeat** | M (abstract) | S13 | H |
| Wingbeat frequency at takeoff | About 20% higher than in level flight; landing similar to level flight (Dial 1992) | M (via citing paper) | S13 | M |
| Amplitude at takeoff | Higher than in level flight, and highest in takeoff and landing | M | S13, S14 | H |
| Stroke plane at takeoff | Tilted steeply downward relative to horizontal; body more horizontal than at landing | M | S13 | H |
| Body sequence | Crouch with the tail down and the head drawn in. The wings begin to rise while the legs extend. The legs extend through lift-off, so the first downstroke starts near lift-off. Then two or three full-amplitude beats with tip-reversal upstrokes before the transition to cruise kinematics | R, from rows above plus Provini 2018 (legs propel the trunk) | S11, S13 | M-L |
| Leg force at takeoff | Several BW peak; landing peaks about half of that | M | S12 | M |

### 3.2 Bat

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Roost drop | Release the feet, fall head-down, open the wings, pull out of the fall; no measured drop height found. 0.3-1 m of fall before level flight looks right | R | bg; S30 | L |
| Vampire jump: thrust source | Forelimbs; the hindlimbs stabilise and orient the body over the forelimbs; the thumbs lengthen the push | M | S9 | H |
| Vampire jump: force | Peak 9.5 BW reached in about 30 ms (6.51 BW also quoted, probably a mean) | M | S9 | M-H |
| Vampire jump: takeoff speed | 2.38 ± 0.24 m/s (impulse 0.058 N s, N = 12) | M | S9 | H |
| Vampire jump: height | ≈ 0.29 m if launched vertically (v^2 / 2g) | D | from S9 | M |
| Sequence | Crouch on the wrists, thumbs and feet; a forelimb push (the body tips up); the wings open at the top of the jump; the first downstroke starts there | R + M | S9 | M-L |

### 3.3 Glider

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Launch force | 1-10 BW (northern flying squirrel); colugos push harder for longer glides | M | S20, S21 | M-H |
| Launch kinematics | The hindlimbs propel; their kinematics are the same in leaping chipmunks, parachuting red squirrels and gliding flying squirrels. Only two forelimb variables separate gliders from non-gliders | M | S26 | H (qualitative) |
| Launch velocity | Not related to body mass (northern flying squirrel) | M | S26 | M |
| First phase | A ballistic dive to gain speed, then the membrane spreads and the path flattens; the colugo reaches terminal speed soon after spreading | M (qualitative) | S21, S23 | M |
| Sequence | Crouch head-down or head-out on the trunk; a hindlimb push; the limbs spread into the X within the first metre; an early steep descent; a shallowing glide | R | S23, S26 | L-M |

---

## 4. Flapping flight

### 4.1 Wingbeat frequency

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Pennycuick 1996 (birds, cruising) | f = m^(3/8) g^(1/2) b^(-23/24) S^(-1/3) ρ^(-3/8) (m kg, b span m, S wing area m^2, ρ air kg/m^3) | M (fitted formula) | S1 | M |
| Pennycuick 1990 (birds) | f = 1.08 m^(1/3) g^(1/2) b^-1 S^(-1/4) ρ^(-1/3) | M | S2 | M |
| Crow (0.55 kg, b 0.95, S 0.15) | 4.6 Hz (1996), 4.4 Hz (1990) | D | S1, S2 | M |
| Pigeon (0.4 kg, b 0.65, S 0.065) | 7.7 Hz (1996), 7.1 Hz (1990); measured cruise in the 5-8 Hz range (bg) | D | S1, S2 | M |
| Observed against benchmark | Migrating birds: observed 0.81-1.05 of the calculated benchmark | M | S1 (2001 paper) | M |
| Dimensional rule, geometrically similar birds | f ∝ m^-1/6; measured interspecific about m^-0.27 because big birds have relatively longer wings | M | S4, S39 | M |
| Bats, interspecific | f ∝ M^-0.26 (65 spp, 2-870 g); M^-0.30 in free flight; birds faster at a given mass | M | S4 | M-H |
| Bat regression with speed | fw = 5.54 - 3.068 log10 m - 2.857 log10 V; units inferred as kg and m/s (the arithmetic only gives sensible values in kg); max fw 4-13 Hz in 23 Australian species | M (abstract) + D (units) | S4 | M |
| Fruit bat (35 g, 5 m/s) | ≈ 8.0 Hz | D | S4 | M |
| *Carollia* (≈ 18 g) | ≈ 5.7-8 Hz: 125-175 frames per wingbeat at 1000 fps during landing manoeuvres | D | S6 | M |
| Microbat (8 g, 5 m/s) | ≈ 10 Hz | D | S4 | M |
| Flying fox (0.6 kg, 8 m/s) | ≈ 3.6 Hz | D | S4 | M |
| Hovering, universal | f ≈ (1/2π) sqrt(m g / (ρ C A)), i.e. f ∝ sqrt(m)/A | M (2024 fit, 414 points) | S39 | M |
| Frequency vs speed | U-shaped in many species (lowest at mid speeds); flat in pigeons and magpies; rising roughly linearly in starlings; bats rise in frequency as speed falls | M | S14 | M-H |

### 4.2 Stroke shape

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Bird stroke amplitude, cruise | ≈ 90-130 deg (pigeon, crow); not systematically speed-dependent in several species (Rosén 2004) | R (bg) + M (no speed effect) | S14 | L-M |
| Bird amplitude, takeoff and landing | ≈ 140-170 deg, with a dorsal near-clap at the top in pigeons | R (bg) + M (larger than cruise) | S13 | M-L |
| Stroke plane, bird | Tilted forward-down at takeoff, more vertical in cruise, tilted upward in landing; changes come mostly from body pitch, not from the shoulder | M | S13 | H |
| Body angle vs speed | Body angle falls as speed rises (birds) | M | S14 | M |
| Downstroke ratio, bird | ≈ 0.5-0.6 cruise, higher at slow speed | R | bg | L |
| Downstroke ratio, bat | DR ∝ f^-0.40 in *Glossophaga* (DR falls as frequency rises) | M | S5 | M |
| Upstroke: tip reversal | Slow flight and takeoff in long-winged birds (pigeon): the primaries flip and the hand sweeps back, giving thrust; on a revolving pigeon wing the downstroke supports ≈ 115% BW and the tip-reversal upstroke gives ≈ 36% BW of thrust | M | S15 | M |
| Upstroke: flexed wing | Cruise in most birds; at all speeds in small, round-winged birds (passerines); wrist and elbow fold, the hand comes in close | M-R | S14 | M |
| Upstroke: feathered | Fast flight: the extended wing rotates nose-down to cut drag | R | bg | L |
| Pigeon upstroke lift in a slow turn | ≈ 50% of downstroke lift in the paper; the lab summary gives ≈ 27% | M | S16 | M |
| Wingtip path, bird (body frame) | Slow: a figure eight or a forward-tilted loop (tip forward and down on the downstroke, back and up on the upstroke). Cruise: a narrow oval tilted forward. Relative to still air it is a sinusoid | R | bg | L |
| Wingtip path, bat | A backward flick at slow speed disappears above about 3.2 m/s; above that the tip path is more vertical or directed up and forward relative to the air | M | S5 | H |
| Bat kinematics with speed | Change gradually; no sudden gait change (*Glossophaga* 1.2-7.5 m/s); 20 joint angles from 17 markers vary with speed but their dimensional complexity does not (*Cynopterus*, nine speeds) | M | S5, S38 | H |
| Bat Strouhal number | Favourable 0.17-0.22; 0.25-0.4 at 3.4-4 m/s; 0.5-0.68 below about 3 m/s | M | S5 | H |
| Bat upstroke lift at cruise | Vortices trail the wingtips through the whole beat, so the upstroke makes lift at cruise speed | M | S40 | H |
| Bat wing mass | The heaviest wings among flyers, normalised by body mass; wing-to-body roll inertia ratio I* ≈ 5 (*Carollia*) | M | S6 | H |
| Bat camber and trailing edge | Camber set by the fingers (digits 3-5) and membrane tension; the leg (knee flexion) pulls the trailing edge ventrally; under load bats raise frequency and cut amplitude, and adjust angle of attack, camber and area | M | S38 | M-H |
| Bat amplitude, model | ±20 to ±50 deg in the simulation sweep; ±30 deg used as representative | M (model input) | S6 | M |

### 4.3 Body motion through a beat

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Heave | The body rises during the downstroke and sinks during the upstroke; a few percent of span in a pigeon's slow flight, near zero in cruise | R | bg | L |
| Pitch | The body pitches nose-up during the downstroke and down at the upstroke; a few degrees in cruise, larger in slow flight | R | bg | L |
| Bat body | Heavy wings make the body counter-rotate more visibly than in birds (follows from I* ≈ 5) | D | S6 | M |
| Rig rule | Drive body heave and pitch as a phase-locked sinusoid at the wingbeat frequency, with the peak upward velocity at mid-downstroke; scale amplitude by (1 - speed / cruise speed) so that slow flight bobs and cruise is smooth | D | rows above | L |

### 4.4 Speeds

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Bird cruise (radar, 138 spp) | Ue = 15.9 m^0.13 (m kg); 8-23 m/s across species; pigeons 15-20 m/s; birds of prey and herons slow for their mass, songbirds fast | M | S3 | H |
| Crow, cruise | ≈ 14.7 m/s from the fit (0.55 kg); crows sit in the falcon, crow and songbird group, whose exponent is a flatter 0.08 | D | S3 | M |
| Pigeon Vmp / Vmr | ≈ 9-10 m/s / ≈ 12-14 m/s (Pennycuick 1968, recalled) | R | bg | L |
| Pigeon cruise / racing | ≈ 14 m/s from the fit; racing homers about 18-20 m/s | D / R | S3, bg | M / L |
| Small passerine cruise | ≈ 9.6 m/s from the fit (20 g) | D | S3 | M |
| Fruit bat speeds | Wind-tunnel range about 3-7 m/s (*Cynopterus*, nine speeds; *Glossophaga* 1.2-7.5 m/s) | M | S5, S38 | M |
| Microbat | Most 4-8 m/s foraging; the free-tailed bat is far faster (above 40 m/s reported, bg) | R | bg | L |
| Wind and power-curve speeds | Vmp does not change with wind; Vmr rises in a headwind and falls in a tailwind | M | S3 (context) | M |

### 4.5 Intermittent flight, hovering, turning

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Flap-bounding | Most passerines, woodpeckers, some small owls: a burst of flaps, then the wings fold flat to the body in a ballistic bound | M | S28 | H |
| Zebra finch | Flap-bounds at all speeds; the body gives ≈ 8% of weight support in the bound; time spent bounding rises with practice | M | S28 | H |
| Flap-gliding | Larger birds (crows, gulls, raptors) alternate flaps with glides on extended wings; some species flap-glide when slow and flap-bound when fast | M | S14 | M |
| Bound phase share | ≈ 30-50% of time, rising with speed (zebra finch) | R | bg | L |
| Hovering | Only brief in small birds and pigeons, with tip-reversal or backward-flick upstrokes; nectar bats (*Glossophaga*) hover-feed; Strouhal number high below 3 m/s | M | S5, S15 | M |
| Turning, pigeon | "Like a helicopter": the aerodynamic force stays roughly fixed to the body, so the bird banks (rolls) to aim it into the turn; the roll torque comes from the outside wing sweeping further forward and the inside wing more laterally | M | S16 | H |
| Turning, bat | More complex than a bank turn; much of the roll and pitch comes from inertial asymmetric wing folding (fact 8) | M | S6, S38 | H |
| Tail, bird | Spread at low speed, in landing and in turns for lift and pitch; folded in cruise. Pigeon tail angle moves with body angle from takeoff to landing | M (tail angle) + R | S13 | M-L |

---

## 5. Gliding and soaring

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Glide ratio, northern flying squirrel (field) | 1.98; glide angle 26.8 deg | M | S23 | M |
| Glide ratio, inland sugar glider (field) | 2.2; 26.4 deg; 40 glides, mean 18.1 m (8-41 m) | M | S24 | H |
| Glide ratio, *Petaurus* (review) | 1.8; 29.7 deg; 20.4 m from 12 m height | M | S24 | M |
| Sugar glider in a short lab glide | Accelerating slightly downward (≈ 1.0 m/s^2) and forward (≈ 2.1 m/s^2), so not at equilibrium; lower lift and higher drag coefficients than flying squirrels, yet glides only slightly steeper | M | S25 | M |
| Glide speed, flying squirrel | 6.3-8.1 m/s | M (via search) | S23 | M |
| Equilibrium? | No: speed, forces and coefficients change through the glide; squirrels make more lift than weight late in the glide; they reach the steady-state glide ratio at higher speed | M | S23 | H |
| Sink rate, flying squirrel | ≈ 3-3.5 m/s (7 m/s at 27 deg: 7 × sin 27°) | D | S23 | M |
| Colugo | Typical 30 m glide for 8 m of drop, ratio ≈ 3.75 (D); up to ≈ 150 m; turns of 90 deg; collapses the membrane to punch through foliage, then reopens it | M (press) + D | S21, S22 | M-L |
| Colugo cost | Gliding costs ≈ 1.5× the energy of climbing the same distance but is ≈ 10× faster | M | S22 | M |
| Glide pose (X) | Forelimbs forward and out, hindlimbs back and out, wrists and ankles at the membrane corners; head up; the tail trails as a stabiliser and rudder (flying squirrel's tail flattened) | R | bg | M |
| Steering | Asymmetric limb (wrist and ankle) movement that changes membrane tension, angle of attack and area on one side; flying squirrels deploy their surfaces symmetrically during landing | R + M | S25, Byrnes abstract | L-M |
| Soaring birds: circling | Radius 12-18 m (frigatebird, black vulture, pelican; grows with wing loading); 20-26 m for falcon and stork | M | S29 | M-H |
| Soaring birds: bank | Pennycuick's optimum for vultures ≈ 24 deg; vultures measured at 25-35 deg | M | S29 | M |
| Soaring birds: glide ratio and sink | Vulture best L/D ≈ 15, minimum sink ≈ 0.7-1 m/s; albatross L/D above 20; a crow's glide about 8-10 | R | bg (Pennycuick 1971) | L |
| Slope soaring | Ride the updraft along a ridge or cliff face at minimum-sink speed; the wings are held extended and barely beat | R | bg | L |

---

## 6. Landing

### 6.1 Bird

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Wings vs legs | The wings do 3-10 times more work in the last beats than the body kinetic energy the legs absorb on contact; peak landing force about 50% of the takeoff peak (zebra finch, diamond dove) | M | S12 | M-H |
| Sequence | Aerial braking with the wings, then the legs absorb the remaining momentum on impact ("aerial" then "absorption") | M | S27 | H |
| Body pitch | Near-vertical at landing; body pitch about 80 deg higher than at takeoff (pigeon) | M | S13 | H |
| Stroke plane | Tilted upward; larger amplitude than cruise; frequency similar to level flight | M | S13 | H |
| Flare posture | Wings forward and fully spread, alula up, tail fanned and depressed, legs swung forward under or ahead of the body, head held level (visual tau control of the approach, Lee 1993) | R + M (tau) | S27, bg | M-L |
| Approach speed | A few m/s at the perch (pigeon about 2-4 m/s at contact) | R | bg | L |
| Perch grasp | Toes curl around the perch in under 100 ms overall; single claw corrections take 1-2 ms (2-4% of a wingbeat); squeeze up to ≈ 3 BW (30 g parrotlet, 20 Hz wingbeat) | M | S27 | H |
| Perch vs ground | On a perch the feet must grasp and the body pitches forward over the perch after contact; on the ground the legs flex more and the bird may run on for a step or two | R | bg | L |
| Compliant perch | Doves do not recover the elastic energy of a flexible perch; their takeoff speed suffers | M | S12 | M |

### 6.2 Bat on a ceiling

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Four-point landing | Thumbs and feet strike together; body pitch rises throughout until the belly faces the ceiling (*Cynopterus*); 3.6 BW mean | M | S7 | H |
| Two-point landing | Legs brought up along the side of the body; only the feet touch (*Carollia*, *Glossophaga*); 0.74 BW mean | M | S7 | H |
| Flip timing (*Carollia*) | ≈ 0.25 s steady climb toward the ceiling at pitch ≈ 60 deg, symmetric beats; then ≈ 0.25-0.5 s of manoeuvre with roll, pitch and yaw changing at once; 4-5 wingbeats in all | M | S6 | H |
| Flip mechanism | Wingbeat frequency drops; one wing stays more extended than the other (roll and yaw); the wings retract in the upstroke and protract in the downstroke (pitch) | M | S6 | H |
| Inertial yield | ≈ 40 deg of roll per wingbeat with one wing fully folded in the upstroke; ≈ 60 deg of pitch per wingbeat from a ±22.5 deg sweep; aerodynamics needed only to complete the full landing | M (model fitted to data, R^2 0.69-0.94) | S6 | H |

### 6.3 Glider on a trunk

| Quantity | Value | Type | Source | Conf |
|---|---|---|---|---|
| Pitch-up | Just before landing, the glider pitches up (rising angle of attack, an active stall) to brake and spread the impact over all four limbs; the angle of attack at landing grows with glide distance | M | S20, S26 | M-H |
| Landing force | 3-10 BW (northern flying squirrel), rising with glide distance | M | S20 | H |
| Steep approach | At about 45 deg the squirrel cannot pitch up enough and lands forelimbs first at higher force | M | S20 | H |
| Colugo braking | ≈ 60% speed reduction on average just before landing; forces fall rapidly as glide distance rises from the shortest glides, then level off; short leaps land on two limbs | M | S21 | M-H |
| Sequence | Swoop low under the target, pull up, the body goes near-vertical (head up), all four feet reach forward, contact on all four at once, the claws grip, the body settles against the trunk | R + M | S20, S21 | M-L |

---

## 7. Animation and rigging references

| Reference | What it gives | Type | Source | Conf |
|---|---|---|---|---|
| Wu & Popović 2003 (SIGGRAPH) | Bird as an articulated skeleton with elastically deformable feathers; forward dynamics with aerodynamic forces; each wingbeat optimised separately, then concatenated; takeoff, cruise, rapid descent, turn and landing (raven, eagle); feather elasticity strongly shapes the wingbeat | M (abstract) | S31 | H |
| Ju et al. 2013 (TOG) | Dove captured with markers plus high-speed cameras; a physics controller that is interactively steerable and robust to disturbance | M | S32 | H |
| Won et al. 2017 (SIGGRAPH Asia), "How to Train Your Dragon" | Example-guided (keyframe-shaped) learned control of flapping flight | M | S32 | H |
| Falcon Age (2019) design write-up | The perched falcon is pose-blended, not IK: the fist is a ball, the feet rotate around it through 30 blend poses, with a shuffle clip to recentre; 27 body poses keep the bird natural; a counter-motion blend holds the bird still in world space; 3D navigation on a mostly auto-generated graph with hand-placed links through windows | M (dev write-up) | S33 | M |
| Keeper (Double Fine) GDC 2026 | "Fixed-wing flight": pathfinding, locomotion and animation of a bird companion, aimed at controllable, not floaty, flight | M (session listing) | S34 | M |
| Pixar "Hoppers" SIGGRAPH 2026 talk | Feather surfaces from skin and silhouette, de-intersected with a vellum sim; tiered animation that simulates all body feathers, only hero wing feathers or a blend; a simpler sim plus procedural flutter controls | M | S35 | M |
| DreamWorks "The Wild Robot" (2025) | A surface-constraint wing-fold system that "significantly reduces the need for counter animation" | M (talk summary) | S35 | M |
| Animal Logic Quill | Feather twist, fold and curl as groom parameters; feathers driven mainly by rig deformation of the surface mesh and auxiliary meshes such as wing membranes | M | S35 | M |
| Feather spread rig, craft practice | Drive the primaries' fan from the forearm's rotation and from the wrist-to-shoulder distance; per-feather rotation constraints with graded influence so feathers fan open as the wing extends and stack as it folds; a manual layer on top | R (forums, course outlines) | S36 | L |
| Membranes | Skinned to finger bones with an extra spar per digit; blendshapes or a cloth or position-based solver add billow and trailing-edge flutter; the leg drives the trailing edge (bats) | R + M (leg) | S38, bg | L |
| Raph Koster, "Andean Bird" prototype (2006) | One flap control per wing; constant gentle gravity; wings held back put the bird into a steep dive; climb comes from flapping on tempo; holding one wing down to turn makes it sink. Lesson: it became "too hard and tiring" | M (design blog) | S37 | M |
| Indie bird-game devlogs | A flap is a discrete press that costs stamina and gives a burst; gliding recovers stamina; control feel is make-or-break; damp the camera against flap bob | R | S37 (devlog search) | L |
| Aven Colony, "Flight of Nature", "Feather", "Glide" | No design write-up on their flight found. Aven Colony is a city builder, not a bird game (bg) | - | - | - |
| Open data | Bergou et al. bat landing and falling kinematics (Euler angles and model files) on Dryad; Cheney et al. 2014 bat hindlimb dataset (S1 file); `afpt` R package implements Pennycuick's flight models; Pennycuick's "Flight" program (bg); Brown's XMAPortal hosts XROMM data (bg, not checked) | M / R | S6, S38, S41 | M |

---

## 8. Recommendation: flight model and animation states

### 8.1 State machine

States common to all three, with what each family does in it:

| State | Bird | Bat | Glider | Enter | Leave |
|---|---|---|---|---|---|
| **Ground** | Walk (bob), hop, run | Crawl (wrist and thumb), bound | Bound, climb | Grounded | Jump input, or a ledge |
| **Perch / roost** | Grip, settle, preen idle | Hang head-down (ceiling) | Cling head-up or head-down to a trunk | Landed on a perch surface | Takeoff |
| **Takeoff** | Crouch and leap: legs give 80-95% of lift-off speed, then 2-3 big beats | Roost drop (fall 0.3-1 m and open), or the vampire forelimb jump (2.4 m/s) | Hindlimb leap with the limbs spreading, and a steep dive | Takeoff input | Airspeed above a threshold, or about 0.4 s |
| **Flap** | Continuous flapping; flap-bound for small birds, flap-glide for large | Continuous flapping, high amplitude, upstroke flex | Not available | Thrust input and powered | No thrust input: Glide |
| **Glide** | Wings spread, slight dihedral, tail as rudder | Wings spread (bats glide poorly; short) | The X spread | No thrust input in air | Thrust (powered), landing zone, or speed too low (stall: dive) |
| **Soar** (sub-state of Glide) | Circle in a thermal (bank 25-35 deg, radius 15-25 m) or ride a slope | Not used | Not used | Updraft volume | Leaving the volume |
| **Flare** | Pitch up to near-vertical, wings forward, tail fanned, legs forward | Ceiling flip (two-point or four-point) | Pitch up into a stall, all four limbs forward | Within a distance or time-to-contact of the target | Contact |
| **Land** | Legs absorb; grip if a perch | Feet hook (and thumbs if four-point) | Four limbs strike, claws grip | Contact | To Ground or Perch after the settle |

Trigger the flare on time-to-contact (tau = distance / closing speed, as pigeons do) rather than distance. A tau of about 0.3-0.5 s fits a 4-5-beat bat flip at 8-10 Hz and a bird's 2-3-beat flare (D, L).

### 8.2 What scales with size

| Quantity | Scaling | Use |
|---|---|---|
| Wingbeat frequency | ∝ m^-0.26 to m^-0.30 (both families); Pennycuick's formula when span and area are known | The flap clip's playback rate |
| Cruise speed | ∝ m^0.13 (birds, measured), against a theoretical m^1/6 | The flight model's trim speed |
| Ground stride | Fr on hip height, λ/h ≈ 2.3 Fr^0.3 (as research-08) | Walk, hop and crawl rates |
| Head-bob | Excursion ∝ stride length; off at short steps and above Fr ≈ 0.5 | Walk only |
| Glide ratio | Barely scales with size in mammals (≈ 2 for 0.1-0.2 kg animals, about 2-4 for a colugo); in birds set by aspect ratio | The glide polar |
| Landing force | Rises with approach speed; the stall cuts it | Animation intensity, a "hard landing" reaction |
| Takeoff | Leg share falls in very small flyers (hummingbird 59%) | Leap height |

### 8.3 Continuous blends and discrete switches

Continuous across bird, bat and glider (blend with weights, like the dog-cat-horse blend):
- Wingbeat frequency and amplitude, downstroke ratio, and the degree of upstroke flex (bird flexed-wing → bat strong flex with backward flick; it maps onto one "upstroke fold" parameter from 0 to 1).
- The surface's fold geometry along the arm: feathers fanning against membrane spars. Both are a hand-span parameter driven by the wrist and digit spread, so one "extension" scalar can drive both.
- The glide pose: wing span, dihedral, limb sweep, tail spread.
- Flare pitch angle, body heave and pitch amplitude, and turn bank.
- Glide polar parameters (L/D, minimum-sink speed).

Discrete (switch on the family weight's dominant member, or keep separate clip sets):
- **Ground posture**: biped (bird) against quadruped (bat on the wrists, glider on all fours). Do not blend a biped into a quadruped. Pick by the dominant weight and cross-fade the clip.
- **Head-bob**: bird only.
- **Powered against glide-only**: the glider has no Flap state.
- **Rest orientation**: bird upright on a perch, bat inverted from a ceiling, glider clinging to a vertical trunk.
- **Landing topology**: bird feet-first on top of a surface, bat flip onto an underside, glider four-limb strike on a vertical face.
- **Takeoff mode**: leg leap, roost drop or forelimb jump, and limb-spread leap.

### 8.4 Is a "flight power" axis (glider 0, bat, bird 1) a sensible blend parameter?

Yes, but only as **one** of two axes, and not for the ground.
- **What it should control**: the existence and strength of the Flap state, meaning the thrust available, the climb rate, the sustainable flap duration, how far the flight model lets the avatar climb, and the glide ratio's improvement with aspect ratio. At 0 the avatar can only trade height for distance (ratio about 2). At values above roughly 0.3-0.5 the Flap state unlocks with thrust scaled by the axis.
- **Why the bat is not halfway in between**: the bat is not a weaker bird. Its kinematics differ in kind: heavy inertial wings, strong upstroke flex, slow-speed manoeuvrability, a gradual speed response and a poor glide. Use a second axis, **"surface: feather ↔ membrane"** (or simply the family weights), to drive wing fold style, camber, glide quality and manoeuvre style. The flight-power axis then sets how much flapping the avatar can do, and the surface axis sets how the flapping looks.
- **Recommended parameters**: family weights (bird, bat, glider), as the quadrupeds use, and two derived scalars: `flight_power` (0 glider, about 0.7 bat, 1 bird) and `membrane` (0 bird, 1 bat and glider). Ground posture and rest orientation stay discrete, keyed to the dominant family.

### 8.5 Flight model for the game (arcade, not aerodynamic)

- **Lift and drag from a glide polar**: L/D as above; sink rate at trim; the player pitches to trade speed for height. Stall below a minimum speed: the nose drops and the avatar dives to regain speed (it never just falls).
- **A flap is a discrete impulse** on press (a beat or a short burst), tied to the clip's phase so the impulse lands at mid-downstroke. It drains stamina, scaled by `flight_power`. Gliding and soaring recover stamina. This matches what indie bird games converged on (S37), and it avoids the Koster prototype's failure mode of per-wing realism that was "too hard and tiring".
- **Turn by bank**: roll first, then the lift turns the path (pigeon "helicopter" steering, S16). Cap bank at 45-60 deg for the bird, and allow the bat a faster roll rate (≈ 40 deg per wingbeat inertially, S6).
- **Takeoff assist**: give an initial velocity from the family's takeoff (bird leap: about 2-3 m/s up and forward, L; vampire 2.4 m/s; glider leap plus a dive), and then let 2-3 strong beats carry it to trim.
- **Landing assist**: snap to a valid perch, ceiling or trunk within a capture radius once the flare starts; drive the flare from tau; grade the landing as soft or hard by the remaining speed (gliders: steep approaches above about 45 deg land hard, as in S20).

---

## 9. Sources

- S1. Pennycuick CJ (1996). Wingbeat frequency of birds in steady cruising flight: new data and improved predictions. J Exp Biol 199:1613-1618. https://tethys.pnnl.gov/publications/wingbeat-frequency-birds-steady-cruising-flight-new-data-improved-predictions ; Pennycuick CJ (2001). Speeds and wingbeat frequencies of migrating birds compared with calculated benchmarks. J Exp Biol 204:3283. https://tethys.pnnl.gov/publications/speeds-wingbeat-frequencies-migrating-birds-compared-calculated-benchmarks
- S2. Pennycuick CJ (1990). Predicting wingbeat frequency and wavelength of birds. J Exp Biol 150:171. https://cob.silverchair.com/jeb/article/150/1/171/5695/Predicting-Wingbeat-Frequency-and-Wavelength-of
- S3. Alerstam T, Rosén M, Bäckman J, Ericson PGP, Hellgren O (2007). Flight speeds among bird species: allometric and phylogenetic effects. PLoS Biol 5:e197. https://europepmc.org/article/PMC/PMC1914071
- S4. Bullen RD, McKenzie NL (2002). Scaling bat wingbeat frequency and amplitude. J Exp Biol 205:2615. https://cob.silverchair.com/jeb/article/205/17/2615/9081/Scaling-bat-wingbeat-frequency-and-amplitude ; Norberg UML, Norberg RÅ (2012). Scaling of wingbeat frequency with body mass in bats and limits to maximum bat size. J Exp Biol 215:711. https://tethys.pnnl.gov/publications/scaling-wingbeat-frequency-body-mass-bats-limits-maximum-bat-size
- S5. Norberg UML, Winter Y (2006). Wing beat kinematics of a nectar-feeding bat, Glossophaga soricina, flying at different flight speeds and Strouhal numbers. J Exp Biol 209:3887. https://cob.silverchair.com/jeb/article-split/209/19/3887/16372/Wing-beat-kinematics-of-a-nectar-feeding-bat
- S6. Bergou AJ, Swartz SM, Vejdani H, Riskin DK, Reimnitz L, Taubin G, Breuer KS (2015). Falling with style: bats perform complex aerial rotations by adjusting wing inertia. PLoS Biol 13:e1002297. https://europepmc.org/article/PMC/PMC4646499 ; data: https://datadryad.org/dataset/doi:10.5061/dryad.21qs5
- S7. Riskin DK, Bahlman JW, Hubel TY, Ratcliffe JM, Kunz TH, Swartz SM (2009). Bats go head-under-heels: the biomechanics of landing on a ceiling. J Exp Biol 212:945. https://www.sciencedaily.com/releases/2009/03/090320102136.htm ; https://sicb.org/?p=28269
- S8. Riskin DK, Hermanson JW (2005). Independent evolution of running in vampire bats. Nature 434:292. https://news.cornell.edu/node/282646 ; Riskin DK, Parsons S, Schutt WA, Carter GG, Hermanson JW (2006). Terrestrial locomotion of the New Zealand short-tailed bat Mystacina tuberculata and the common vampire bat Desmodus rotundus. J Exp Biol 209:1725. https://cob.silverchair.com/jeb/article-pdf/209/9/1725/1258951/1725.pdf ; https://sicb.org/?p=32714
- S9. Schutt WA, Altenbach JS, Chang YH, Cullinane DM, Hermanson JW, Muradali F, Bertram JEA (1997). The dynamics of flight-initiating jumps in the common vampire bat Desmodus rotundus. J Exp Biol 200:3003. https://www.sciencegate.app/document/10.1242/jeb.200.23.3003
- S10. Earls KD (2000). Kinematics and mechanics of ground take-off in the starling Sturnus vulgaris and the quail Coturnix coturnix. J Exp Biol 203:725. https://www.sciencegate.app/document/10.1242/jeb.203.4.725 ; hummingbird comparison: Tobalske BW et al. (2004). Take-off mechanics in hummingbirds. J Exp Biol 207:1345. https://cob.silverchair.com/jeb/article/207/8/1345/15063/Take-off-mechanics-in-hummingbirds-Trochilidae
- S11. Provini P, Tobalske BW, Crandell KE, Abourachid A (2012). Transition from leg to wing forces during take-off in birds. J Exp Biol 215:4115. https://umimpact.umt.edu/en/publications/transition-from-leg-to-wing-forces-during-take-off-in-birds/ ; Provini P, Abourachid A (2018). Whole-body 3D kinematics of bird take-off. Sci Nat 105:12.
- S12. Provini P, Tobalske BW, Crandell KE, Abourachid A (2014). Transition from wing to leg forces during landing in birds. J Exp Biol 217:2659. https://research.bangor.ac.uk/en/publications/transition-from-wing-to-leg-forces-during-landing-in-birds/ ; https://researchnow.flinders.edu.au/en/publications/coping-with-compliance-during-take-off-and-landing-in-the-diamond/
- S13. Berg AM, Biewener AA (2010). Wing and body kinematics of takeoff and landing flight in the pigeon (Columba livia). J Exp Biol 213:1651. https://biewenerlab.oeb.harvard.edu/publications/wing-and-body-kinematics-takeoff-and-landing-flight-pigeon-columba-livia ; Robertson AMB, Biewener AA (2012), body pitch, via search summary.
- S14. Tobalske BW, Dial KP (1996). Flight kinematics of black-billed magpies and pigeons over a wide range of speeds. J Exp Biol 199:263. https://typeset.io/pdf/flight-kinematics-of-black-billed-magpies-and-pigeons-over-a-3znrg2t2m9.pdf ; Tobalske BW (2007). Biomechanics of bird flight. J Exp Biol 210:3135. https://www.scilit.net/publications/29035c3d384193f9d934326f9e5c095c
- S15. Crandell KE, Tobalske BW (2011). Aerodynamics of tip-reversal upstroke in a revolving pigeon wing. J Exp Biol 214:1867. https://typeset.io/pdf/aerodynamics-of-tip-reversal-upstroke-in-a-revolving-pigeon-3d13rw0x24.pdf ; Crandell & Tobalske (2015). Kinematics and aerodynamics of avian upstrokes during slow flight. https://research.bangor.ac.uk/portal/en/researchoutputs/kinematics-and-aerodynamics-of-avian-upstrokes-during-slow-flight(1439760d-e111-4410-a61e-6ac1be19832a)/export.html
- S16. Ros IG, Bassman LC, Badger MA, Pierson AN, Biewener AA (2011). Pigeons steer like helicopters and generate down- and upstroke lift during low speed turns. PNAS 108:19990. https://europepmc.org/article/PMC/PMC3250151 ; Biewener lab, pigeon turning: https://biewenerlab.oeb.harvard.edu/pigeon-turning
- S17. Necker R (2007). Head-bobbing of walking birds. J Comp Physiol A 193:1177. https://reinhold-necker.de/seite10.html ; Troje NF, Frost BJ (2000). Head-bobbing in pigeons: how stable is the hold phase? J Exp Biol 203:935. https://acnpsearch.tweb-dev.unibo.it/singlejournalindex/70646995 ; Fujita M (2002). Head bobbing and the movement of the centre of gravity in walking pigeons. J Zool 257:373. Hancock JA et al. (2014), tinamous: https://people.ohio.edu/stevensn/documents/Hancock%20et%20al%202014.pdf
- S18. Verstappen M, Aerts P, Van Damme R (2000). Terrestrial locomotion in the black-billed magpie: kinematic analysis of walking, running and out-of-phase hopping. J Exp Biol 203:2159. https://acnpsearch.tweb-dev.unibo.it/singlejournalindex/74170395 ; Verstappen M, Aerts P (2000). Motor Control 4:150. https://repository.uantwerpen.be/link/irua/28121
- S19. Gatesy SM, Biewener AA (1991). Bipedal locomotion: effects of speed, size and limb posture in birds and humans. J Zool 224:127. https://data.zhenzhi.com/details/1/1008834282 ; grounded running in birds (quail, guinea fowl, ptarmigan): https://pmc.ncbi.nlm.nih.gov/articles/PMC5821450 ; https://people.ohio.edu/stevensn/documents/Hancock%20et%20al%202007%20Tinamous%20early%20view.pdf
- S20. Paskins KE, Bowyer A, Megill WM, Scheibe JS (2007). Take-off and landing forces and the evolution of controlled gliding in northern flying squirrels Glaucomys sabrinus. J Exp Biol 210:1413. https://researchportal.bath.ac.uk/en/publications/take-off-and-landing-forces-and-the-evolution-of-controlled-glidi/ ; https://www.pubmed.gov/17401124/
- S21. Byrnes G, Lim NT-L, Spence AJ (2008). Take-off and landing kinetics of a free-ranging gliding mammal, the Malayan colugo. Proc R Soc B 275:1007. https://newsarchive.berkeley.edu/news/media/releases/2008/02/07_colugo.shtml ; https://sicb.org/?p=30075
- S22. Byrnes G, Libby T, Lim NT-L, Spence AJ (2011). Gliding saves time but not energy in Malayan colugos. J Exp Biol 214:2690. https://www.sciencedaily.com/releases/2011/07/110728082546.htm ; https://rvc-repository.worktribe.com/output/1417591/gliding-saves-time-but-not-energy-in-malayan-colugos
- S23. Bahlman JW, Swartz SM, Riskin DK, Breuer KS (2013). Glide performance and aerodynamics of non-equilibrium glides in northern flying squirrels. J R Soc Interface 10:20120794. https://pmc.ncbi.nlm.nih.gov/articles/PMC3565731 ; field glide ratio 1.98: https://researchers.une.edu.au/en/publications/gliding-performance-of-the-northern-flying-squirrel-glaucomys-sab-6/
- S24. Goldingay RL et al. (2024). Gliding performance of the inland sugar glider. Aust Mammal AM24004. https://www.publish.csiro.au/AM/AM24004 ; Jackson SM (2000). Glide angle in the genus Petaurus and a review of gliding in mammals. Mammal Rev 30:9 (via table at https://connectsci.au/view-large/9944216).
- S25. Bishop KL (2007). Aerodynamic force generation, performance and control of body orientation during gliding in sugar gliders (Petaurus breviceps). J Exp Biol 210:2593. https://cob.silverchair.com/jeb/article/210/15/2593/16981/Aerodynamic-force-generation-performance-and ; https://sicb.org/?p=31524
- S26. Essner RL (2002). Three-dimensional launch kinematics in leaping, parachuting and gliding squirrels. J Exp Biol 205:2469. https://cob.silverchair.com/jeb/article/205/16/2469/9129/Three-dimensional-launch-kinematics-in-leaping ; Scheibe JS, Paskins KE, Ferdous S, Birdsill D (2007). Kinematics and functional morphology of leaping, landing and branch use in Glaucomys sabrinus. J Mammal 88:850. https://bioone.org/journals/journal-of-mammalogy/volume-88/issue-4/06-MAMM-S-331R1.1/Kinematics-and-Functional-Morphology-of-Leaping-Landing-and-Branch-Use/10.1644/06-MAMM-S-331R1.1.full
- S27. Roderick WRT, Chin DD, Cutkosky MR, Lentink D (2019). Birds land reliably on complex surfaces by adapting their foot-surface interactions upon contact. eLife 8:e46415. https://elifesciences.org/articles/46415
- S28. Zebra finch flap-bounding: body lift (Tobalske, Hearn, Warrick). https://sicb.org/?p=30904 ; Tobalske BW et al. (1999). Kinematics of flap-bounding flight in the zebra finch. J Exp Biol 202:1725. https://cob.silverchair.com/jeb/article-pdf/2604006/jexbio_202_13_1725.pdf ; https://pmc.ncbi.nlm.nih.gov/articles/PMC6602328
- S29. Ákos Z, Nagy M, Vicsek T (2008/2010). Comparing bird and human soaring strategies; Thermal soaring flight of birds and unmanned aerial vehicles. https://arxiv.org/pdf/0902.0312 ; https://arxiv.org/pdf/1012.0434 ; Pennycuick CJ (1971). Gliding flight of the white-backed vulture. J Exp Biol 55:13. https://openpolar.no/Record/fthighwire:oai:open-archive.highwire.org:jexbio:55%2F1%2F13 ; vultures near the ground: https://hal.umontpellier.fr/hal-02393999/document
- S30. Initiating flight directly from the ground by Tadarida brasiliensis. Chiroptera Neotropical 13(2), 2007. https://bvs-vet.org.br/vetindex/periodicos/chiroptera-neotropical/13-(2007)-2/initiating-flight-directly-from-the-ground-by-tadarida-brasiliensis-ch
- S31. Wu J-c, Popović Z (2003). Realistic modeling of bird flight animations. ACM TOG 22(3):888. https://grail.cs.washington.edu/projects/flight/
- S32. Ju E, Won J, Lee J, Choi B, Noh J, Choi M-G (2013). Data-driven control of flapping flight. ACM TOG 32. https://mrl.snu.ac.kr/research/ProjectBird/birdFlight.htm ; Won J, Park J, Kim K, Lee J (2017). How to train your dragon: example-guided control of flapping flight. ACM TOG 36(6). https://mrl.snu.ac.kr/research/ProjectDragon/Dragon.htm
- S33. Game Design Deep Dive: Creating Falcon Age's feathered companion. Game Developer. https://gamedeveloper.com/design/game-design-deep-dive-creating-i-falcon-age-s-i-feathered-companion
- S34. Collins J (2026). Fixed-wing flight in Keeper: pathfinding, locomotion and animation of a bird companion. GDC. https://gamedev.net/news/fixed-wing-flight-in-keeper-r5220/
- S35. Pixar (2026). Crafting feathers for the bird kingdom in Hoppers. SIGGRAPH Talks. https://research.pixar.com/docs/2026.SiggraphTalks.PCB.pdf ; Animal Logic, Quill: https://animallogic.com/wp-content/uploads/2023/06/Quill-Birds-of-a-Feather-Tool.pdf ; DreamWorks talk library: https://research.dreamworks.com/research_library/page/2
- S36. Feather-spread rigging practice (forums and course outlines). https://forums.autodesk.com/t5/maya-animation-and-rigging/what-is-the-best-way-to-approach-rig-these-types-of-wings/m-p/12796121 ; https://blenderartists.org/t/how-would-you-rig-this-owl-and-what-do-you-think-of-him/538513 ; https://pluralsight.com/courses/rigging-wings-3ds-max-643
- S37. Koster R (2006). Andean Bird 0.4. https://www.raphkoster.com/?p=650 ; indie flight devlogs: https://gamesandwutnot.itch.io/flyinghome/devlog/804788/added-glide ; https://ethangodwin.itch.io/bird/devlog/320302/bird-postmortem
- S38. Riskin DK, Willis DJ, Iriarte-Díaz J, Hedrick TL, Kostandov M, Chen J, Laidlaw DH, Breuer KS, Swartz SM (2008). Quantifying the complexity of bat wing kinematics. J Theor Biol 254:604. https://vis.cs.brown.edu/results/bibtex/Riskin-2008-QCB.bib ; Iriarte-Díaz J et al. (2012). Kinematic plasticity during flight in fruit bats. PLoS ONE. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC3352941/ ; Cheney JA et al. (2014). Wing membrane and hindlimb kinematics. PLoS ONE. https://pmc.ncbi.nlm.nih.gov/articles/PMC4032239
- S39. Jensen KH et al. (2024). Universal wing- and fin-beat frequency scaling. https://www.ncbi.nlm.nih.gov/pmc/articles/PMC11152310/
- S40. Hedenström A et al. (2007). Bat flight generates complex aerodynamic tracks. Science 316:894. https://sites.usc.edu/drydenwt/files/2022/06/AHetal_Science_07.pdf
- S41. afpt: Animal Flight Performance Tools (R package implementing Pennycuick's models). https://search.r-project.org/CRAN/refmans/afpt/html/Bird.html

### Reading notes

- Many Journal of Experimental Biology full texts (silverchair) and PMC pages were unavailable (maintenance page or 403). Europe PMC's REST full text worked for S3 and S6. Most JEB figures here come from abstracts or from secondary citations and are graded M accordingly.
- No primary measurement was found for: bird hop height, a bat's roost-drop distance, a bird's approach speed at the perch, the flap-bound duty fraction, or glider ground gaits. These rows are L and need a measurement or a reference before anything hard-codes them.
- The leg share of takeoff depends on the definition. "Share of lift-off velocity" (80-95%, S10, S11) and "share of acceleration over the takeoff" (≈ 25% in the pigeon, S13) are both right. The first is the instant the feet leave; the second covers the first wingbeats too.
