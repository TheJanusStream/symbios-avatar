# Dossier: AAA-feel animation for parametric humanoid + creature bodies, and the Bevy animation landscape

## 1. Bevy animation, current state (0.18 shipped Jan 2026; 0.19 shipped June 19, 2026)

**What exists in-engine (`bevy_animation`):**
- **AnimationGraph** — DAG of nodes evaluated onto an `AnimationPlayer`. Clip nodes, **blend nodes** (weights normalized), **add nodes** (additive layering, NOT normalized — breathing-over-walk etc.). Additive-blend weight fix in PR #16279.
- **Masks / groups** — every graph node can carry an `AnimationMask` (64 mask groups, bitmask). Native upper-body/lower-body layering.
- **Animation events** — clips embed events fired on the player or a specific `AnimationTargetId`, delivered through observers. Churn risk: Event Rearchitecture PR #20731 + issue #21473 — API still moving.
- **Morph targets** — since PR #8158; **max 64 per mesh**, weights animatable. Implementation uses 2D texture per target to survive WebGL2 row-size limits.
- **Skinning limits** — historically MAX_JOINTS = 256 (64 KB uniform binding for WebGL2). PR #21256 made MAX_JOINTS/MAX_MORPH_WEIGHTS **user-configurable**. PR #9351 clamps rather than crashes. 0.19 added **per-joint-bounds frustum culling for skinned meshes** (PR #21837).
- **0.18** — glTF extension handler that builds an AnimationGraph during asset load.

**What does NOT exist (we'd build it):**
- **No retargeting.** Issue #15612 "Animation Retargeting Woes": `AnimationTargetId` is a UUID from bone-name path → clips only apply to identical hierarchies. Nothing implemented.
- **No state machine / transition layer** — `AnimationTransitions` is linear crossfade only. No inertialization, no blend trees beyond the raw graph.
- **No IK**, no foot planting, no look-at, no physics secondary bones.
- **No GPU animation** (issue #17964) — a few thousand CPU-evaluated skeletal animations practical.
- **No runtime clip mutation ergonomics** (issue #13052) — `AnimationClip`s CAN be built in code (curves are `AnimatableCurve` over the `Curve` trait) but no tooling.
- No committed animation workstream in 0.19. **The low-level graph/evaluation layer is solid; everything "character animation" above it is ecosystem/DIY.**

## 2. Bevy ecosystem crates

| Crate | What | Compat | Maintenance |
|---|---|---|---|
| bevy-tnua | Floating character controller (walk/jump/crouch/coyote/platforms) | 0.32.0 (June 2026), **Bevy 0.19**; avian3d integration | Very active. Exposes basis state (speed, airborne) useful as animation-controller input. |
| bevy_animation_graph (mbrea-c) | Asset-driven full animation system: graphs-as-assets, **FSMs as graph nodes**, visual editor, **basic retargeting** (bone-path overrides), **partial-ragdoll via physics_avian**, root-motion extraction | 0.11.0 (July 2026), **Bevy 0.19** | Active, single-maintainer, 10 breaking releases. *Replaces* bevy_animation rather than extending it — adoption fork-in-the-road. |
| bevy_mod_inverse_kinematics | Cyclic-descent IK, positional + pole targets | 0.11.0 (Mar 2026), **Bevy 0.18** | Maintained. **Tiny (~141 LoC)** — two-bone limb solver reference, not full-body. |
| bevy_fabrik | FABRIK chains | — | Single-chain only. bevy_ik abandoned. |
| bevy_vrm (unavi-xyz) | VRM 0.x/1.0 loading, MToon, humanoid bone map | 0.3.0 (Apr 2026), Bevy 0.18.1 | Active again; UNAVI project. |
| bevy_vrm1 (not-elm) | VRM 1.0 + VRMA; **spring bones, LookAt, node constraints** | 0.3.x | Active; most complete anime-avatar runtime in Bevy. Spring-bone + LookAt implementations map directly to our secondary-motion needs — read even if not adopted. |
| bevy_verlet | Verlet points/sticks | 0.5 | Not bone-aware; likely write ~200 lines of damped-spring bone chains ourselves (see naelstrof/JigglePhysics for the 0..1-param design). |
| Ragdoll | No standalone crate; **Avian 0.4 joints** (JointDamping, JointForces) adequate to build one | — | bevy_animation_graph's partial-ragdoll is the only shipped integration. |
| Foot placement | **Nothing exists.** No Bevy foot-IK/stride-warp crate — build foot-raycast + two-bone IK + pelvis drop ourselves. | — | — |

Gap summary: Bevy has *pieces* but **no retargeting, no full-body IK, no inertialization, no foot planting** — exactly the layer our crates would occupy; no incumbent.

## 3. Retargeting theory & practice

**Unity Humanoid/Mecanim "muscle space"**: each humanoid joint DOF is a **muscle** — normalized scalar in [−1,1] mapping to per-character anatomical [min,max] rotation range. Clips stored as muscle curves, not bone transforms; playback maps through the target's Avatar back to bones. Hands/feet also stored **normalized relative to "Humanoid Root"** (center of mass, humanoid scale); optional IK pass pulls extremities to match. Lesson: normalization is the whole trick — rotations as fraction-of-range, positions as fraction-of-character-scale in a body frame. Only works within one topology class.

**Unreal IK Rig / IK Retargeter**: per-skeleton **IK Rig asset**: retarget root + named **chains** (spine, legs, arms) + Full-Body-IK **goals** on extremities. Retargeter maps source→target chains; **chains abstract over bone count** (3-bone arm → 5-bone arm); FK copied per-chain in normalized parameter space, IK goals fix contacts (speed/foot planting, stride warping). Lesson: *named chains + goals* is the topology-flexible unit, not bones. Generalizes to creatures better than Unity's fixed muscle list.

**Bereznyak, GDC 2016, "IK Rig: Procedural Pose Animation"** (gdcvault.com/play/1023279; open-source partial impl github.com/gustavoeb/ikrig): encode a pose as **proportion- and orientation-invariant IK descriptors** (per limb: direction vectors, normalized reach = distance/limb-length, hip/chest frames), then **reconstruct** on any target rig at runtime with IK. Animation becomes a stream of goals, not rotations. Demonstrated: human→octopus transfer, terrain adaptation, runtime with artist modifiers. Closest published fit for the humanoid-ish subset.

**Hecker et al., SIGGRAPH 2008 (Spore)**: the only shipped system where **morphology is unknown at author time**. Animators pose in a custom tool (*Spasm*) with normal keyframing, but targets are **semantic queries** ("all feet", "front-most grasper", "spine") rather than bones; motion recorded **body-independently** (positions in normalized character frames). Runtime *specializes* against the actual creature — queries resolve to concrete parts → per-frame **pose constraints fed to a fast IK solver**. Separate **gait system** generates foot timing for arbitrary leg counts; **"jiggles"** secondary system adds wobble.
- Lesson: our parametric records already know their own semantics (assemblers name parts) — we're in a *better* position than Spore, which had to infer semantics from arbitrary user creatures. Semantic queries ("all ground-contact points", "primary manipulators", "gaze part") would unify creatures and vehicles under one animation vocabulary.

## 4. Procedural techniques that reduce clip authoring

- **David Rosen, GDC 2014 (Overgrowth)**: entire character moveset ≈ **13 keyframes**. Interpolate 1–2 extreme poses per motion with **sinusoidal/spring curves** driven by gameplay state (walk = 2 poses + sine phase from speed; jump = 1 pose + physics); **mirroring** halves authoring; physics springs on interpolation give overshoot/settle; IK pins feet/hands; lean from acceleration. Highest-leverage philosophy for code-authored clips on parametric bodies: poses scale with the body automatically if stored as IK-goal poses.
- **Procedural gait for N legs**: robotics CPG (coupled-oscillator) generators — one parameter morphs tripod/quadruped/wave gaits with smooth transitions. Game-side simpler pattern: per-foot phase offsets + "step when home-point drifts past threshold" raycast stepping (Spore's shipped precedent). **Phase-offset + duty-cycle (stance fraction) is a 2-parameter gait space** covering walk/trot/gallop/wave for arbitrary leg counts.
- **Foot placement / terrain**: raycast under each planted foot, two-bone IK to hit, rotate foot to surface normal, **lower pelvis by min(foot corrections)**; stride warping scales step length with slope/speed. ~1–2 weeks on top of a two-bone solver.
- **Look-at chains**: distribute gaze rotation across eyes→head→neck→chest with per-joint weight falloff and angular clamps; VRM LookAt is the reference model.
- **Secondary motion**: damped spring / verlet bone chains (ears, tails, hair, antennae). Refs: VRM springBone spec, naelstrof JigglePhysics. Cheap, body-agnostic, huge perceived-quality win — Spore shipped it as "jiggles."
- **Inertialization** — Bollo, GDC 2018 (Gears of War): replace crossfade with a **post-process**: at transition capture pose+velocity offset from new target, decay offset (quintic, per-channel, vectors + quaternions) to zero. Evaluates ONE pose during transitions and preserves momentum; industry standard. *Exactly* what a procedural/goal-driven system wants — works on any pose source including IK outputs.
- **Motion matching** — Clavet GDC 2016. **Verdict: skip.** Requires large per-skeleton mocap DB; fundamentally at odds with parametric bodies (DB authored on ONE skeleton; every proportion change degrades match quality). Inertialization + small goal-space clip set captures most of the perceived fluidity.

## 5. Facial animation standards

- **ARKit 52 blendshapes** (FACS-derived, 0..1) = de-facto interchange standard — "Perfect Sync" in VTuber world means exactly these 52 on VRM. Adopting the 52 as the face parameter space buys free compat with face-tracking sources and tooling. (arkit-face-blendshapes.com)
- **Oculus/Meta 15 visemes** for lipsync: sil, PP, FF, TH, DD, kk, CH, SS, nn, RR, aa, E, ih, oh, ou — language-agnostic; the set remains standard (VRChat, RPM) even though OVRLipSync itself is EOL.
- **VRM 1.0 expression presets** — minimal stylized set: emotions happy/angry/sad/relaxed/surprised/neutral; vowels aa/ih/ou/ee/oh; blink, blinkLeft/Right; lookUp/Down/Left/Right; with **override modes** (happy can suppress blink/lookAt/mouth). Crucially an **abstraction layer**: each preset binds to morph targets *or* bone transforms *or* material/UV changes — ideal for procedural heads where `happy` can be implemented per head-archetype however geometry allows.
- **Blendshape vs bone-based for procedural heads**: blendshapes need per-mesh authored deltas — hostile to generated geometry. Bone/joint facial rigs (jaw, eyes, brows, lids, mouth corners as small bone clusters) are proportion-independent and generate naturally from part composition; less subtle skin deformation — acceptable stylized. **Recommended hybrid: bone-driven macro face + optional generated morphs for a few key shapes**, exposed through a VRM-style preset layer; Bevy's 64-morph ceiling as hard bound. Eyes standard regardless: look-at with clamps, saccades, stochastic blink (period ~2–6 s, double-blink probability) layered as an add-node.

## 6. Animation data sources & licenses

| Source | License | Redistributable in an OSS crate? |
|---|---|---|
| Mixamo (Adobe) | Royalty-free *within* games/projects | **NO** — cannot redistribute as standalone assets/engine packs; problematic for MIT public repos. Fine inside a shipped game binary; not the crate. |
| CMU mocap | "Free for use in research… may include in commercially-sold products, may not resell directly" — informal | **Gray but customary yes** for a small curated subset as retarget sources. 2000s-era quality, inconsistent skeletons. |
| Ubisoft LaFAN1 | CC BY-NC-ND 4.0 | **NO** — NC + ND kills both. |
| **100STYLE** (Mason et al., 100 locomotion styles) | **CC BY 4.0** (Zenodo 8127870) | **YES** — commercial + derivatives with attribution. Best permissive locomotion corpus; Daniel Holden published a common-skeleton retarget pipeline (orangeduck/100style-retarget). |
| Hand-keyed in code | Ours | **YES** — uniquely suited: Rosen-style pose-sparse clips as Rust data (IK-goal keyframes + easing) are proportion-independent by construction, versionable, license-clean. |

Posture: **100STYLE (CC BY) + code-authored poses as redistributable core; CMU optional; Mixamo/LaFAN1 never inside the crate.**

## Synthesis: recommended animation architecture

Common foundation (build regardless):
- Engine-agnostic core: **semantic body description** (named chains + goals + capabilities, Spore/UE hybrid: "spine chain", "ground contacts[]", "graspers[]", "gaze") generated from parametric records — we own the generator, so semantics are free.
- **Two-bone + FABRIK IK solvers** (no adequate crate), **inertialization transitions** (one implementation serves clips AND procedural output), **spring-bone secondary motion**, **foot raycast + pelvis adjust**, **look-at chain**, **VRM-style expression preset layer** over bone-driven faces with ARKit-52 naming and Oculus-15 visemes.

**Candidate A — Goal-space procedural core ("Spore/IK-Rig native"), clips as sparse pose data.**
All motion authored/stored as **body-invariant goal streams** (normalized reach, contact timing, body-frame trajectories) — hand-keyed in code (Rosen-style, ~tens of poses per moveset) or extracted offline from 100STYLE into goal space. Runtime = gait generator (phase/duty-cycle, CPG-style across leg counts) + IK reconstruction + inertialization + springs.
- Pros: ONLY approach natively covering humanoids AND arbitrary creatures AND vehicles with one pipeline; zero per-body authoring; tiny data; wasm-friendly; matches procedural-everything ethos; shipped precedent (Spore).
- Cons: highest engineering risk (goal encoding + robust reconstruction is subtle); mocap richness hard to reach purely procedurally — expressive one-offs (emotes, dances) laborious as goal streams; debugging needs bespoke tooling.

**Candidate B — Canonical-skeleton clip library + runtime retarget + IK fix-up ("Unity/UE-shaped").**
One canonical humanoid rig (+ one per creature family); small clip library (100STYLE-derived + hand-keyed) on canonical rigs; runtime chain-normalized FK copy retarget; IK fixes contacts; Bevy AnimationGraph layers; inertialization transitions.
- Pros: proven quality ceiling (mocap nuance survives); leverages Bevy's graph; per-family clip sets tractable; easier future artist content.
- Cons: retarget layer built from scratch; creatures outside authored families need new clip sets (breaks "arbitrary proportions" at topology level); clip data inflates wasm payload; UUID-path clip binding needs a maintained indirection layer.

**Candidate C — Adopt bevy_animation_graph + procedural layers on top.**
- Pros: fastest to first result; editor + FSM + ragdoll free; 0.19-current.
- Cons: single-maintainer dependency with 10 breaking releases *replacing* bevy_animation (bus-factor + upgrade tax); retargeting is path-remap only (insufficient for proportions); weds engine-agnostic core to a Bevy-specific third-party graph — inverts our layering.

**Recommendation:** A as the architecture, borrowing B's data strategy where cheap — **goal-space core with a small library of goal-space "style assets"**, some hand-keyed in code, some batch-extracted offline from 100STYLE (CC BY, attribution in crate). Treat C as reference reading, not a dependency. Sequence: IK solvers + foot placement + inertialization first (biggest feel-per-effort), gait engine second, goal-space clip format third, face layer last (bone-driven + VRM presets + ARKit naming). Motion matching explicitly out of scope.
