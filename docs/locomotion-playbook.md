# Locomotion playbook

**Read this before touching any gait term.** It has the same standing the
L-system playbook has in overlands: every posture-arc issue (milestone #11)
cites the section it is acting on, and if the section it needs is missing, the
playbook is incomplete — fix that first (#326's rule). The goal is never "the
walk looks better today"; it is **the moving body reads as a person**, judged
on strips against the CC0 references, at natural pace AND at the pace the
consuming app actually travels.

The one lesson that shaped this milestone's method is this repo's own: the
head overhaul shipped five measurable, literature-grounded fixes that moved
the render by nothing, and the gait arrived in the same posture — every term
individually verified by walkaudit and the sum reading wrong. The diagnosis
(#325) found the same shape again: **every constant was in family and the
distribution was wrong** — where the lean lives, where the head's counter
lives, what feeds the pace. So: renders convict, instruments explain, and no
constant moves before a strip shows why.

---

## 1. The method: reference-guided, strips-first

Owner's decision, 2026-08-31, recorded here so it outlives the session that
made it:

- **Reference-guided, not reference-fitted.** Renders are judged side by side
  against the CC0 clips (`assets/clips.bin`, provenance in
  [clips.md](clips.md); the mesh2motion sibling holds the GLB sources). Where
  a literature constant disagrees with the reference and the eye, **the
  reference wins**. Fitting stays offline; the runtime stays procedural and
  deterministic — the architecture is load-bearing for P2P (#237 removed
  runtime clips on purpose; overlands #1067 gates on parity). Full numeric
  curve-fitting was considered and deferred; revisit if the guided pass
  plateaus.
- **Strips are the unit of review.** `bevy_symbios_avatar`'s viewer:
  `cargo run --release -F builtin-clips --example viewer -- --strip out.png`
  — phase-matched rows, `--norelevel` for the head-level ablation,
  `--accel from:to` for transients, `--align` for a clip's arbitrary frame-0
  origin (**`Walk` wants 0.5**). Iterations are judged on strips; only passes
  worth in-world time go to the owner, whose eye is the gate for done.
- **Both paces, always.** Overlands' default `walk_speed` is 4.0 m/s ≈ viewer
  pace 1.5 ≈ Froude 1.81 — a run, held constantly while a player travels
  (walkaudit: pace 1.4 → 3.57 m/s, 1.8 → 5.49). A term that only looks right
  at pace 1.0 is not done.
- **Instruments stay convergence-shaped** (#1183): the same source reads
  25–50 mm across build environments through `f32` transcendentals, so a
  guard asserts "the step halves at 2× sampling" or compares two runs of the
  SAME build — never a millimetre threshold across builds.

## 2. What a real walker measurably does

### 2.1 The speed axis

One dimensionless number — the Froude number `v²/(gL)` — sets the family:
stride, cadence, duty, and the walk-run transition near **Fr ≈ 0.5**
(dynamic-similarity ceiling for walking; the engine's `FROUDE_TRANSITION`).
This crate already lives on that axis (`anim/speed.rs`, Grieve's relation);
nothing in the arc should introduce a second speed vocabulary beside it.

### 2.2 The cycle's proportions

At natural pace: stance ≈ 60% of the cycle, swing ≈ 40%, double support
≈ 10% twice per cycle — and double support is what a run gives up: it shrinks
with speed to zero at the transition, replaced by flight. The classic "six
determinants" (Saunders/Inman 1953) name the pelvic mechanisms below; the
modern reading (Kuo and others) treats them less as prescriptions than as the
observed smoothing of the centre of mass — useful here as target magnitudes,
not as mechanisms to copy blindly.

### 2.3 The pelvis and the centre of mass

Target magnitudes on an adult at natural pace: pelvic rotation ≈ 8–14° total
about the vertical; pelvic list (Trendelenburg dip toward the swing side)
≈ 5°; vertical centre-of-mass excursion ≈ 3–5 cm (double-bump, two per
cycle); hip sagittal excursion ≈ 40° per cycle, reaching 10–20° of
**extension** at terminal stance. That extension figure matters beyond the
legs — see §2.4.

### 2.4 Where the lean lives — the trunk against speed *(cited by #329)*

How much: self-selected trunk inclination while walking is a few degrees
(2–7° band; this crate's `TRUNK_LEAN` = 5.5° at natural pace sits mid-band,
verified). Recreational runners hold **5–7.5°** across 12–20 km/h; elites
hold less, and hold it constant. Our 8.25° at travel pace is *in family* —
#325 acquitted the magnitude — with one caveat: the literature's lean does
not grow without bound, and records allow `walk_speed` up to 50. A ceiling
belongs in the arc.

Where: **a real lean is a whole-body line, not a waist fold.** A runner's
forward inclination is carried substantially by the ankle and by *hip
extension* — the body falls forward as one unit and the trailing hip opens —
while flexion of the trunk *on* the pelvis stays small, and what there is
distributes through a lumbar curve. The running-form literature frames this
as "lean from the ankles, not the waist"; the reference clips draw it as one
continuous ankle-to-shoulder line. This crate's `lean` pitches rigidly from
the lowest spine joint — deliberately, and the deliberateness is exactly what
#325 convicted: same magnitude as the reference, opposite silhouette (the
waist fold plus the collar kink). The arc's job (#329) is to move the lean's
*anatomy*: hip extension first, a spread counter-curve above, the waist crease
last and least.

### 2.5 Head stabilization in space *(cited by #330)*

Humans stabilize the **head in space**, not the neck angle: across walking,
running and hopping the head's pitch is held within a few degrees *of the
world* (Pozzo, Berthoz & Lefort 1990), and its small residual pitch
oscillation counter-rotates against the head's vertical *translation* — an
inertial gaze platform, two cycles per stride, compensating bob rather than
trunk posture. Three consequences for the arc:

1. The right target is "head level in space", which the current bargain does
   reach — the conviction is not the goal but the delivery: a **15.2°
   counter over an 8.25° visible lean, all at one neck joint** (measured by
   #328's guard; the hinge over-rotates to incline the chord, and the neck
   pays the hinge's angle, not the chord's).
2. A real cervical spine distributes its counter along seven vertebrae; ours
   kinks at one joint, which is the collar crease the strips show.
3. Stabilization is not 100% cancellation of posture: a runner's head rides a
   leaning body whose *face* stays level — the chin does not jut forward off
   a vertical neck. When #329 moves the lean into the hips, the counter the
   neck owes shrinks toward the chord angle, which is half of #330 for free.

### 2.6 The arms *(cited by #331)*

Arm swing is substantially **passive** — the arms ride the trunk's yaw as
damped pendulums (shoulders as elastic linkages; Collins, Adamczyk & Kuo
2009), anti-phase with the legs, cancelling the pelvis's angular momentum.
Amplitude grows with speed. The gait-specific fact the reference row made
unmissable is measured: humans walk with **straight arms (elbow ≈ 35°)** and
run with **bent arms (elbow ≈ 90°)** — and bent-arm *walking* costs about 11%
more oxygen, so the straight arm at a walk is not laziness, it is optimal
(Callens et al., J Exp Biol 2019). At overlands' travel Froude the body is
running, so the elbows must flex and the swing must become a pump at waist
height, with kicked-up swing-leg heels to match. Both should ride the speed
axis continuously (elbow flexion and heel lift as functions of Froude), not
switch with a gait label.

## 3. What makes a walk READ

Not the same question as what a walker does. The craft vocabulary for judging
strips, from the animation canon (Williams, *The Animator's Survival Kit*;
the Disney principles):

- **The four positions**: *contact* (legs split, both toward the ground, arms
  opposite — the pose that says "walking"), *down* (weight taken, the body's
  lowest point — where weight lives), *passing* (one leg vertical under the
  body), *up* (push-off carrying the body to its highest). A strip of 8
  frames crosses each twice; when a strip reads wrong, name WHICH position
  reads wrong before naming a term.
- **Weight is the perceived quantity.** An audience reads the pelvis's drop
  into *down* and the compression out of it; a gait with correct joint angles
  and no down feels floaty, one with exaggerated down feels heavy. The
  engine's pelvis bob and crouch terms are this section's levers.
- **Overlap and follow-through**: parts arrive at different times; an arm
  that reverses exactly with the legs reads mechanical. ARM_LAG 0.07 exists
  for this; the craft question is whether the lag reads at both paces.
- **A run must read airborne**: visible flight frames, heels kicked toward
  the glutes, elbows pumping. A run with a walk's silhouette at a run's speed
  reads as skating — which is precisely how travel pace read in #325's
  sheets.
- Williams' own caveat applies to us doubly: he moves the arm's widest swing
  from *down* to *contact* because it **reads** better, against observation —
  the license the reference-guided method grants. When the literature, the
  reference and the eye disagree, the eye is reading FOR the audience.

## 4. How others made procedural motion not read as a mechanism

- **Very few poses, physical interpolation** (Rosen, "An Indie Approach to
  Procedural Animation", GDC 2014 — Overgrowth ran on ~13 keyframes total):
  the life is in the interpolation and the physics-flavoured overlays, not in
  pose count. Our gait is already this shape; the lesson is that when
  something reads dead, suspect the *transitions and overlays* before adding
  pose vocabulary — the head-overhaul lesson in animation clothing.
- **Layered IK over a simple core** ("IK Rig", Bereznyak, GDC 2016): keep the
  generator simple and re-target/correct at the end. Ours: gait core →
  posture layer → gaze → footing tail. The arc adds terms to layers; it does
  not add layers.
- **Springs and inertialization for every discontinuity** (critically damped
  springs; Bollo's inertialization, GDC 2018): any input that can step —
  source switches, pace, heading — goes through a decaying offset or a
  spring, never raw. This is the frame for the chassis conviction (overlands
  #1192): the engine's postural terms are pure functions of pace, so they are
  exactly as continuous as the pace fed in; overlands assigns velocity at
  39–50 m/s² effective on stops. The smoothing belongs at the consumer (or as
  a rate limit on postural terms — the arc may decide), with a time constant
  around a human load-response: **of the order of one step, ~0.3 s**, not one
  frame.
- **Motion matching** (Clavet, GDC 2016) is what we are deliberately not
  buying: it solves transition quality by searching a mocap database at
  runtime — data-heavy, non-procedural, and unfit for a P2P architecture
  where every peer must derive the identical pose from a record and a clock
  (#237, overlands #1067). Its lesson survives without its database: match
  the *future trajectory*, not the instant — a body should commit to where it
  is going, which is why pace smoothing (§ above) reads as intent rather than
  lag.

## 5. Section map for the arc

| Issue | Acts on | Sections |
|---|---|---|
| #329 lean distribution | hip extension, spread counter-curve, ceiling | §2.4, §2.3, §3 (weight) |
| #330 head-level bargain | partial counter along the cervical chain | §2.5 |
| #331 run character | elbow pump, heel lift, on the speed axis | §2.6, §2.2, §3 (airborne) |
| overlands #1192 | pace smoothing at the consumer | §4 (springs) |

## 6. Traps already paid for

- **The reference clips pause at the wrap** — six of eight looping clips hold
  a frame there ([clips.md](clips.md)); a strip column near phase 1.0 of a
  clip row may show the source's pause, not a gait defect. `Walk` and
  `Idle_A` close cleanly; prefer `Walk` as the walking reference.
- **A clip's frame 0 is an arbitrary export moment** — align it (`--align`,
  `Walk` = 0.5) or every column compares two different stride moments.
- **The neck counter is larger than the lean** (§2.5, #328's measurement):
  any term that pitches the trunk hinge and "takes it back off" pays the
  hinge angle, not the chord angle. Budget accordingly.
- **Never compare rigs by bone name** (retarget lesson); never trust a
  millimetre threshold across build environments (#1183, §1).
- **Judge a carve POSED and judge a walk MOVING**: idle was fine and the
  hunch appeared only in motion — a term verified at rest says nothing about
  what the gait adds.

## Sources

Biomechanics: [Trunk lean metabolic optimum (Proc B)](https://doi.org/10.1098/rspb.2025.2717);
[running trunk inclination 5–7.5° recreational](https://www.tandfonline.com/doi/full/10.1080/14763141.2021.1873411);
[trunk flexion effects on limb mechanics](https://www.sciencedirect.com/science/article/pii/S0167945721000658);
[normal gait parameters (AAPM&R)](https://now.aapmr.org/biomechanics-normal-gait/);
[six determinants overview](https://whatispodiatry.com/the-six-determinants-of-gait-foundations-of-efficient-human-locomotion/);
[Pozzo, Berthoz & Lefort 1990, head stabilization](https://link.springer.com/article/10.1007/BF00230002);
[vestibulocollic head control review](https://pmc.ncbi.nlm.nih.gov/articles/PMC4157641/);
[Collins, Adamczyk & Kuo 2009, arm swing](https://journals.biologists.com/jeb/article/212/4/523/18953/Control-and-function-of-arm-swing-in-human-walking);
[straight-arm walking, bent-arm running (JEB 2019)](https://journals.biologists.com/jeb/article/222/13/jeb197228/2704/Straight-arm-walking-bent-arm-running-gait).
Craft: Williams, *The Animator's Survival Kit* ([walk-cycle positions](https://courses.cs.washington.edu/courses/cse459/19au/assignments/assignment_5/index.html)).
Practice: [Rosen, GDC 2014](https://www.gdcvault.com/play/1020583/Animation-Bootcamp-An-Indie-Approach);
[Clavet, GDC 2016 motion matching](https://archive.org/details/GDC2016Clavet);
Bollo, "Inertialization" GDC 2018.
