# Dossier: Parametric 3D Body/Character Generation — Techniques, Architectures, Licensing

*A point-in-time research dossier gathered while this crate was being designed. Kept as the record of what was found and why the decisions in [../plan.md](../plan.md) were made — external version numbers, prices and library gaps in it were true when written and are not maintained.*


Scope: parameter-space design + mesh-generation technique survey for a fully-procedural (no shipped base mesh) Rust generator covering humanoids AND creatures at Fortnite/Sea-of-Thieves fidelity. All third-party model data below is reference-only; licensing analysis included to confirm what must NOT be shipped.

---

## 1. Academic parametric body models (SMPL family, GHUM)

### 1.1 How the shape spaces work

**SMPL** (Loper et al., SIGGRAPH Asia 2015) is the canonical design. Core recipe:
- A fixed artist-made **template mesh** (6,890 verts, quad-dominant) with fixed topology and a fixed 24-joint kinematic tree.
- **Shape space**: PCA over ~4,000 registered body scans (CAESAR). Identity = vector of **shape betas** (10–300 coefficients); mesh = template + Σ βᵢ·Bᵢ where Bᵢ are per-vertex displacement basis vectors ("blend shapes as vertex offsets").
- **Joint regressor**: joint locations are a learned sparse linear function of the *shaped* vertices — joints move automatically as betas change. This is the key trick your crate should copy: derive the skeleton from the shaped surface, never store it independently.
- **Pose correctives**: additional blend shapes driven by elements of the (rotation-matrix-encoded) pose, linearly added *before* LBS, so LBS artifacts (candy-wrapper, joint collapse) are pre-corrected. Pose is factored out of shape by training.
- Everything is linear → runs as vertex shader–friendly math; SMPL was explicitly designed to be compatible with standard game LBS pipelines.
- SMPL page: https://smpl.is.tue.mpg.de/

**Variants**:
- **SMPL-X**: adds articulated hands (MANO) + face (FLAME expression space) in one model — https://smpl-x.is.tue.mpg.de/
- **STAR** (Osman et al., ECCV 2020): replaces SMPL's *global* pose-corrective blendshapes with **sparse, spatially-local per-joint correctives** (each joint only influences nearby vertices), 80% fewer parameters, better generalization; drop-in SMPL replacement. https://www.researchgate.net/publication/346782152_STAR_Sparse_Trained_Articulated_Human_Body_Regressor
- **SUPR** (ECCV 2022): trained as a unified full-body model that can be **factorized into separate part models** (head, hands, novel foot model with contact deformations). Lesson: train/design whole-body but keep part-wise separability. https://arxiv.org/pdf/2210.13861
- **GHUM/GHUML** (Xu et al., Google, CVPR 2020): 10,168-vert full-body model; **16-D body shape latent space offered as both linear PCA and a nonlinear VAE**, 20-D facial expression space, minimal 63-joint/124-DOF skeleton with anatomical joint-limit constraints, normalizing-flow pose priors. Distribution is gated behind a request form. https://github.com/google-research/google-research/blob/master/ghum/README.md

### 1.2 Licenses — confirmed

- **SMPL model license** (verified at https://smpl.is.tue.mpg.de/modellicense.html): non-commercial scientific research/education/artistic projects ONLY; explicitly prohibits commercial products/services; prohibits redistribution and sublicensing; prohibits training networks for commercial deployment. Same license family applies to SMPL-X, STAR, SUPR, SMAL (all MPI Perceiving Systems releases).
- **Commercial route exists**: Meshcapade holds a non-exclusive sublicensing agreement with Max-Planck-Innovation; commercial SMPL licensing covers adult variants. https://meshcapade.com/smpl/
- **GHUM**: request-form gated; not freely commercially usable.
- **Conclusion for the crate**: cannot ship any of these meshes/bases; the non-commercial clause plausibly taints *outputs* ("production of other artefacts"). Treat as design reference only.

### 1.3 Transferable parameter-space lessons
1. Small identity vector (10–16 floats) is enough for convincing body-shape variation when the basis is good.
2. Derive joints from the shaped mesh (joint-regressor idea) — skeleton always fits, no re-rigging step.
3. Separate identity (shape), pose, and pose-dependent correction into additive layers applied before a single standard LBS pass.
4. Local/sparse correctives (STAR) beat global ones for generalization and memory.
5. PCA axes are *not human-meaningful*; runevision independently found PCA parameters useless for hand-tuning. For an authored (non-scanned) system, hand-designed semantic axes with encoded correlations are the right analogue to betas.

---

## 2. MakeHuman / MPFB2

- **Architecture**: one fixed base mesh ("hm08", ~13k verts + "helper" geometry) deformed by **targets** — sparse per-vertex delta files that morph the base without changing topology. https://static.makehumancommunity.org/about/concepts/basemesh.html
- **Macro vs micro targets**: "macro" axes (gender, age, weight, muscle, height, proportions, ethnicity) are *interpolating networks of targets* — the engine blends among many pre-authored extreme targets (e.g., young-female-heavy-muscular) so macro sliders traverse a curated multi-dim grid; "micro"/detail targets are hundreds of local deltas (nose width, ear lobe, etc.) applied additively on top. Broken combinations avoided because macro extremes were authored as *complete coherent bodies*, and micro targets are small and local.
- **Skeleton fitting**: the base mesh carries **joint-cube helpers** — tiny 8-vertex cubes embedded in the body whose median defines each bone head/tail. Targets deform the helpers along with the skin, so **joint positions re-derive automatically after any morph** — same spirit as SMPL's joint regressor, implemented as authored geometry.
- **Skinning**: hand-authored weights on the fixed-topology base (LBS); topology never changes, so weights survive all morphs unchanged.
- **License — confirmed CC0 for assets**: program code AGPL/GPL but **all core assets (base mesh, targets, system assets) are CC0**, and outputs are explicitly CC0 — commercially usable. https://static.makehumancommunity.org/about/license.html
- **Implication**: MakeHuman is the ONE system whose mesh data we *could* legally ship — but it's humanoid-only and violates the "no shipped base mesh" constraint. Its joint-cube trick and macro-target-grid design are directly stealable ideas.

---

## 3. Fully procedural mesh generation (core section)

### 3.1 Spore (Maxis / Chris Hecker)

**Skin mesh (editor-time, real-time regeneration)** — primary source: Hecker's "My Liner Notes for Spore," https://chrishecker.com/My_Liner_Notes_for_Spore
- Representation: **blobby implicit surface (metaballs)** over the user-built spine/limb graph. Chosen for **topological robustness** (arbitrary limb attach/detach, no seams) at the cost of local control.
- Kernel: 4th-order polynomial in squared distance, `f_i(p) = s_i[(|p−c_i|²/R_i²) − 1]⁴` — squared *again* vs. prior 2nd-order work for more continuous derivatives, avoiding lighting discontinuities.
- Placement: spherical metaballs along limbs/torso with spacing computed analytically so consecutive balls fuse into a smooth tube. Ellipsoidal metaballs rejected as too slow.
- Tessellation: polygonized then triangulated via **ear clipping** (dodging the then-active Marching Cubes patent); mesh quality via Moore & Warren "Mesh Displacement" (Graphics Gems III).
- Known failure modes (instructive): no metaball grouping → single global surface → **webbing between close limbs** (shipped as "bat wings" feature); skin weights derived from "which body part generated which metaball" → smooth on limbs but **big spine segments give non-smooth weights; fat torsos shear**. A better weighting prototype existed but never shipped.
- **Procedural texturing**: auto UV-charting in ~10 ms (flood-fill triangles by dominant cardinal direction → planar project per chart → bounding-box pack); GPU 3D-paint with multi-channel brushes; skirt polygons + dilation to kill seams; later a particle-script system painting in barycentric surface coordinates with limb-relative frames.

**Animation — SIGGRAPH 2008, Hecker et al., "Real-time Motion Retargeting to Highly Varied User-Created Morphologies"** (PDF: https://www.chrishecker.com/images/c/cb/Sporeanim-siggraph08.pdf):
- Creatures are **generalized body plans**: the editor's spine-and-parts graph is analyzed into semantically labeled capability structures ("spines," graspers, mouths, feet), not a fixed skeleton.
- Animations authored on example characters but recorded **morphology-independently**: each channel stores (a) a **semantic query** selecting parts ("all feet," "front-most grasper," "spine segment N% along") and (b) goals in normalized **goal-space** (positions relative to body frames, fractions of limb length, ground-relative targets) rather than joint-space rotations.
- Runtime **specializes** the animation to the actual creature: queries bind to whatever parts exist, goals scaled by proportions, fast **particle/IK solver** solves per-frame poses. Style survives radically different morphologies.
- Strongest prior art for "one animation set, arbitrary humanoid+creature bodies." Cost: bespoke authoring tool + solver; benefit: no per-morphology animation work.

### 3.2 Implicit / SDF / convolution / sphere-mesh line

- **Convolution surfaces** (Bloomenthal & Shoemake 1991): convolve a kernel along skeletal curves → smooth bulge-free unions along limbs; the classic fix for metaball "lumpiness" along a chain.
- **Sphere-meshes** (Thiery et al. 2013; Tkach/Pauly/Tagliasacchi SIGGRAPH Asia 2016 hand tracking): geometry = volume swept by spheres with **linearly interpolated centers and radii** over a simplicial complex — capsules and "slabs" with per-vertex radii. A whole hand is ~30 spheres. Effectively an *analytic SDF rig*: the parameter record IS the model. https://dl.acm.org/doi/10.1145/2980179.2980226
- **ZBrush ZSpheres**: artist-facing version — tree of spheres (position+radius) converted by "Adaptive Skin" into a quad-dominant subdivision base mesh. Concept target for the authoring layer.
- **Implicit Skinning** (Vaillant et al., SIGGRAPH 2013): approximate each mesh *part* by an HRBF implicit surface; after ordinary LBS, vertices projected/adjusted against the composed implicit field → real-time **skin contact and organic bulges at joints, no self-intersection**, purely geometric post-process. Quality upgrade over raw LBS. http://rodolphe-vaillant.fr/entry/31/implicit-skinning-real-time-skin-deformation-with-contact-modeli

### 3.3 Subdivision / control-cage generation — the quad-topology route

- **B-Mesh** (Ji, Liu, Wang, CGF 2010) — the key paper: **1-D skeleton with key balls (position+radius) at nodes**; sweeps square cross-sections along bones, merges at joints, emits a **quad-dominant base mesh with edge flow aligned to the skeleton**, refines via subdivision + fairing toward the implicit union of the balls. Output suits **sculpting and skeleton-based animation**; Blender's Skin Modifier is the direct descendant. Implementation walk-through: https://khanhha.github.io/portfolio/skeleton-to-mesh-conversion/
- Related: "Interactive shape modeling using a skeleton-mesh co-representation" (Bærentzen et al., SIGGRAPH 2014) — bidirectional skeleton↔mesh editing, polar-annular mesh structure.
- Pipeline implication: **parameters → sphere/capsule skeleton → B-Mesh-style quad cage → 1–2 Catmull-Clark levels** yields game-ready quad topology with animation-friendly edge loops, entirely from code. This is how you get clean stylized silhouettes *without* marching-cubes triangle soup.

### 3.4 Modern/indie writeups

- **runevision (Rune Skovbo Johansen), "Procedural creature progress 2021–2024"** — closest living indie project: creatures from **multi-segmented extruded rectangles** (≈ swept quad boxes ≈ degenerate B-Mesh). Parameter architecture: **503 low-level parameters** (bone lengths, rotations, skin distance from bone) controlled by **106 hand-designed high-level parameters** ("bulkiness," "head length relative to spine"). **PCA tried and rejected** — axes not interpretable; correlations between parameters manually encoded instead. Animation fully kinematic (custom IK w/ foot roll; stride/footfall timing empirically fit). Honest lesson: unconstrained random sampling still mostly produces failures — the constraint/correlation layer is the hard part. https://blog.runevision.com/2025/01/procedural-creature-progress-2021-2024.html
- Survey: "A Primer on Procedural Character Generation for Games and Real-Time Applications" — **ontogenetic** methods (deform a base + combine modular parts) fit games better than simulation-of-growth. https://link.springer.com/chapter/10.1007/978-3-319-53088-8_7
- ML mesh generators (QuadGPT etc.) are offline, heavyweight, license-encumbered — not relevant for a runtime Rust crate.

---

## 4. Auto-skinning / auto-rigging for generated meshes

- **Pinocchio — Baran & Popović SIGGRAPH 2007**: embeds a template skeleton into an arbitrary watertight mesh, computes weights by **bone-heat diffusion** (one sparse linear solve per bone, Laplacian + visibility-based heat term). <1 min on 2007 hardware → trivially fast today. https://people.csail.mit.edu/ibaran/papers/2007-SIGGRAPH-Pinocchio.pdf
- **Geodesic voxel binding** (Dionne & de Lasa, SCA 2013; Maya's "geodesic voxel bind"): voxelize the character (works on **non-watertight, degenerate** meshes), geodesic distance from each bone through the voxel interior → falloff weights. Robust industry default.
- **Delta Mush** (Mancewicz et al., DigiPro 2014): Laplacian-smooth the LBS result, re-add rest-pose detail deltas in local tangent frames → cheap crude weights become production-quality deformation. Ideal companion for procedurally-guessed weights.
- **RigNet** (SIGGRAPH 2020): GNN predicts skeleton + weights from mesh. **GPLv3 or commercial**; weights trained on non-commercial dataset — unusable, and unnecessary when we generate the skeleton ourselves.
- **Practicality ranking for Rust runtime**: since the generator *creates* skeleton and mesh together, derive weights **analytically during generation** (capsule-distance falloff / which-part-emitted-which-region), then optionally (a) Laplacian smoothing pass over weights, (b) delta-mush at runtime. Bone-heat needs sparse solver (e.g. `faer`) + closed mesh — feasible but overkill; geodesic-voxel is the fallback for arbitrary ingested meshes.

---

## 5. Character-creator parameter design in shipped games

- **The Sims 4** — GDC 2015: no visible sliders — **direct manipulation** (grab & drag body/face). Under the hood: data-driven **selectors** (screen region → modifier set) and **modifiers** = mix of morph targets, **bone-driven deformations**, and **deformation maps**; surface partitioned into **regions + layers**, per region highest-layer part wins. Lesson: decouple UI gesture space from internal parameter space; one "slider" = bundle of heterogeneous deformers. https://www.gdcvault.com/play/1022085/Innovations-in-The-Sims-4
- **Nintendo Mii**: deliberately tiny space — discrete part libraries + few continuous per-part transforms. Users need *recognizable caricature*, not accuracy; constraining the space guarantees every combination reads as "a Mii" — constraints via curation, not validation. Wii Mii record ≈ 74 bytes — benchmark for "small parameter records."
- **Elden Ring / FromSoft**: ~50–100 sliders, hierarchical. Community-documented pain: sliders are **coupled** ("everything affects everything else") — cautionary example of *ratio-based* parameterization without orthogonalization.
- **Soul Calibur VI**: **discrete part recombination** + per-region scaling sliders; broken combos avoided by authoring parts to shared sockets and clamped scale ranges (curation strategy at higher fidelity).
- **Cross-game constraint patterns**: (1) curate extremes (author every slider endpoint as a complete valid shape); (2) clamp + couple (sliders drive multiple low-level params with encoded correlations); (3) region/layer arbitration for part conflicts; (4) avoid raw ratio stacks.

---

## 6. Creature/quadruped prior art beyond Spore

- **SMAL — "3D Menagerie"** (Zuffi et al., CVPR 2017): SMPL-formula model for quadrupeds — one template, PCA shape space **trained on scans of 41 toy figurines** (lions, cats, dogs, horses, cows, hippos). Key finding: one *shared* quadruped shape space generalizes to unseen species — strong evidence a single parameterized quadruped chassis with per-family betas can cover many creatures. License: MPI non-commercial. https://arxiv.org/abs/1611.07700
- **No Man's Sky** (GDC 2015/2017): creatures are **part recombination over a small set of archetype skeletons ("blueprints")** — "surprisingly few skeleton types exist on Earth"; authored base rigs per archetype, code swaps/deforms part descriptors from pools with weighted chances + rule filters. NOT per-vertex procedural meshing — authored parts, procedural *assembly + deformation + texture/color*.
- **Impossible Creatures** (Relic 2003): combine two of ~76 animals via authored body parts at fixed sockets; earliest large-scale demonstration that socketed part swaps stay art-directable.

---

## Synthesis: candidate architectures for a fully-procedural humanoid+creature generator

All three candidates share components validated repeatedly above: **(i)** small semantic parameter record → **(ii)** derived internal skeleton ("generalized body plan," typed parts: spine chain, limb chains, head, tail, digits) → **(iii)** mesh generated from the skeleton → **(iv)** weights derived analytically during generation (+ optional smoothing/delta-mush) → **(v)** goal-space/IK-first animation (Spore SIGGRAPH 2008) since one authored clip set must serve all morphologies.

### Architecture A — Sphere/capsule skeleton → quad control cage → Catmull-Clark (B-Mesh line) — RECOMMENDED FIRST
Parameters define a node graph of spheres (position, radius, optional ellipse ratio) per body part; B-Mesh-style sweep produces a quad-dominant cage with skeleton-aligned edge flow; 1–2 subdivision levels final; weights fall out of the sweep.
- **Pros**: clean quad topology + edge loops at joints (best deformation per triangle — matches stylized fidelity); fully analytic (fast, deterministic, seed-stable); UVs tractable (per-part cylindrical/box charts along sweep parameter); tiny records; humanoids and quadrupeds are just different graphs; proven by B-Mesh/Blender Skin Modifier/ZSpheres/runevision.
- **Cons**: joint-merge topology (3+ branches) is the hard 20%; no automatic limb fusion/webbing; local detail (muscle bulges) needs a displacement/"target"-like layer on top.

### Architecture B — Implicit/SDF body (metaballs/convolution over skeleton) + polygonization (Spore line)
- **Pros**: topological robustness — arbitrary attachment, fused/webbed anatomy free; most "creature-capable"; SDF also usable for collision.
- **Cons**: polygonizer output is triangle soup — deforms and UV-charts poorly at stylized fidelity; skin-weight derivation on merged regions was Spore's unsolved wart; implicits round everything (hard to hit crisp silhouettes); mesh not seed-stable vertex-for-vertex under parameter changes.
- Verdict: superb *editor* tech, wrong final-mesh representation; keep SDF as intermediate/fairing.

### Architecture C — Hybrid: layered semantic params → capsule/implicit interior → quad cage extraction, MakeHuman-style macro/micro layering
~10–20 **macro axes** with authored-coherent extremes + encoded cross-correlations (explicitly instead of PCA), plus optional **micro offsets** as sparse displacement layers on the generated cage; body-plan variation (limb counts, tail, digitigrade) as discrete curated part choices. Mesh path = A's cage sweep, implicit-union pass only to *fair* joint regions.
- **Pros**: best final quality; parameter UX proven by four independent systems; degrades gracefully (macro-only records stay tiny); micro layer = artist escape hatch without shipping any base mesh (deltas relative to *generated* geometry).
- **Cons**: most engineering surface; micro-offset layer must be defined in generator-stable coordinates (sweep-parameter space, not vertex indices) or it breaks under macro changes — the main novel design problem.

**Ranking**: C > A > B. Ship A first (C minus micro layer + correlation-rich macros), grow into C. Use B's implicit math for joint-fairing and optional fused anatomy. Adopt the Spore goal-space animation model from day one — retrofitting it later would invalidate authored clips.

**Licensing bottom line**: SMPL/SMPL-X/STAR/SUPR/SMAL = MPI non-commercial (commercial only via Meshcapade) — reference only. GHUM = request-gated. RigNet = GPLv3 + dataset-tainted — avoid. MakeHuman assets = CC0 (legally shippable, architecturally rejected). Spore/B-Mesh/Pinocchio/sphere-meshes/implicit-skinning *techniques* are published papers — freely implementable (Marching Cubes patent long expired).

## Allometry — how a circumference tracks body size

Two sources, both used directly by `plan::derive::humanoid::Girth`, whose
docstring carries the numbers at the sites that read them.

**Heymsfield et al., *Body circumferences: clinical implications emerging from a
new geometric model*, Nutrition & Metabolism 2008
([PMC2569934](https://pmc.ncbi.nlm.nih.gov/articles/PMC2569934/)).** Fits five
circumferences against volume-over-height and reports the powers by sex:

```text
           male   female
  arm      0.61    0.62
  waist    0.62    0.61
  hip      0.42    0.49
  thigh    0.50    0.54
  calf     0.38    0.33
```

Its own summary of the sex split — "males 'enlarging' at a greater rate around
the waist and calf and females around the hips, thighs" — is the android/gynoid
distribution as a measurement rather than an authored blend, and is what the
`femininity` composite interpolates between. At a fixed height a body that grows
only sideways has radius ∝ volume^0.5, so **0.5 is geometric similarity here**,
not the 0.333 that applies when height varies too.

**Nevill et al., *Are adult physiques geometrically similar? The dangers of
allometric scaling using body mass power laws*, Am J Phys Anthropol 2004
([PMID 15160370](https://pubmed.ncbi.nlm.nih.gov/15160370/)).** 478 subjects,
thirteen girths against body mass. Fleshy sites containing both muscle and fat
(upper arms, legs) develop faster than geometric similarity and bony ones (head,
wrists, ankles) slower, with head girths "almost constant, irrespective of
subjects' body size/mass". Thigh muscle girth scales as M^0.439 in athletes and
M^0.377 in controls. This is the source for the wrist and ankle exponents and
for the head carrying no girth factor at all.

**What neither measures**, and what the plan therefore interpolates and labels as
such: the chest, the shoulder girdle, the forearm and the neck.
