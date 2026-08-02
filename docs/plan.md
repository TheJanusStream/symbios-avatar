# symbios-avatar — Project Plan

Parametric 3D avatars whose source of truth is a small AT Protocol record — rpg.actor's idea carried into 3D, at AAA-adjacent render and animation quality. Two crates in the symbios pattern: **symbios-avatar** (engine-agnostic, this repo) and **bevy_symbios_avatar** (Bevy integration, sibling repo). Symbios Overlands becomes the first consuming application once the engine stands.

Synthesized 2026-08-01 from six web-research dossiers in [docs/research/](research/). Tracked as epic #1 (workstreams #2–#11, milestone "v0.1 vertical slice").

## Locked decisions

| Decision | Choice |
|---|---|
| Visual target | Stylized semi-realistic (Fortnite / Overwatch / Sea-of-Thieves class) |
| Geometry | Fully procedural — no shipped mesh assets, records stay tiny |
| Scope | Humanoid + creatures from day one (goal-space animation mandatory) |
| Interop | Native atproto records are canonical; **GLB export only — VRM 1.0 was dropped 2026-08-02** (see below) |
| Lexicon root | `network.symbios.avatar.*` (symbios ecosystem, outside overlands) |
| Multiplicity | Wardrobe of named avatars per identity, one marked default |
| Baked artifacts | Stored on PDS within reason; degree decided along the way by content size |
| Repos | `~/Workspace/symbios-avatar` + `~/Workspace/bevy_symbios_avatar` (each with own chainlink + docs) |
| Overlands chassis | All four existing families — vehicles included — eventually migrate onto this system |
| Face parameter space | ARKit-52 naming + Oculus-15 visemes |
| How the face animates | **Bone-driven macro rig, ARKit-52 naming-only**, with generated pose-space correctives held in reserve for where bones are *measured* to fail. Decided 2026-08-02 (#35) — see §0 |

## 0. Revisions

### 2026-08-02, the adversarial review

**A sixteen-agent adversarial review** (seven lenses, each attacked by an independent skeptic, plus
a completeness critic). Verdict: **sound with corrections**. The foundations were examined and
defended — shared-socket-ring joint construction, goal-space clips over joint-space, `rig::Surface`
as the rule that you measure the built mesh and never trust the plan, `Zone` + `ground_contacts()`
as a generalised body plan, integer-only wire encoding with a test that checks it, and the
two-bone/FABRIK/inertializer trio that has no equivalent in the Bevy ecosystem. None of those
change.

What was missing was a **product**: `PolyMesh` carried no vertex attributes and there was no
`Avatar` type, so the record-to-renderable recipe lived in the examples and had already diverged
between two of them. Everything else the review found was downstream of that. Filed as #27 with the
work in #28–#57.

Three locked decisions changed by the owner in the same session:

- **VRM 1.0 export dropped.** Native records plus our own GLB. This retires the T-pose conflict
  that #19 introduced, frees dual-quaternion skinning, and removes VRM as a constraint on mesh
  decomposition and on how the face animates. Research-05 stays in the repository as a record of
  what conformance would have cost.
- **Horizon: build it properly, no deadline.** Foundations get fixed even where that discards work.
- **Creatures stay day one.** Humanoid-only assumptions are therefore a live architectural problem.
  Three were found and fixed in a single session (#26); the underlying pattern — a rule stated
  about a body *part* where it should be stated about what that part *does* — is the thing to
  design against, not the three instances.

### 2026-08-02, what has since closed

**The headline finding is resolved.** `PolyMesh` carries positions, UVs, normals, skin weights and
vertex colours; `Avatar::build` is the single record-to-renderable recipe; and both examples now
consume it, which turns them from parallel implementations into conformance tests (#28, #29). The
record-evolution defects that become unfixable once the lexicon is published are fixed and tested
(#30, #31, #32, #57), as are the anatomy and IK faults the review found (#41, #42, #43) and the
atlas regions for attached parts (#58).

Three more closed in a second pass, and each changed something the plan asserts:

- **The body now fits the WebGL2 triangle budget** (#40). 43,308 → 29,076 triangles, of which hair
  fell 30,208 → 15,976. The `tests/budget.rs` target test is on and passing, so §1's finding 5
  ("15–30k tris") is met rather than aspired to — but the way it was met matters: each lock of hair
  is sampled by how far it *travels* rather than at a fixed resolution, and the cheaper
  cross-sections were rendered and rejected (a four-sided lock reads as rope; a flat card reads as a
  helmet). The draw-call half of the target is **not** met: five draws against a target of three,
  and both of the extra two are the eyes — the globes, which want a glossy material of their own,
  and the lids, which are geometry rather than a pose because nothing rigs a lid. The lids are
  answered by the face decision below; the globes are not, and a body that draws four is the honest
  expectation until something merges them.
- **A rig can carry joints the body is not made of** (#34). `Role { Deform, Helper, Spring, Facial }`
  on `Joint`, with `Rig::attach` and a filter in both `skin::bind` and `Rig::nearest_bone`. This is
  the prerequisite §3 assumed for both spring chains (#38) and a bone-driven face rig (WS4/#7), and
  it unblocks whichever way #35 goes.
- **`bevy_symbios_avatar` exists** (#37). Library, plugin and a viewer example that draws a body
  through a real GPU, with `--shot <path>` for headless capture. This is the "and in-app" half of
  the gate, which had never once been executed. It found a cross-instrument difference on its first
  frame (hands and feet shade differently in the two renderers), which is precisely the class of
  thing §5's "unproven Bevy quality ceiling" risk needed an instrument to see.

### 2026-08-02, how the face animates (#35, closed)

**Decided: a bone-driven macro rig, ARKit-52 naming-only, with generated pose-space correctives
held in reserve for where bones are *measured* to fail.** VRM forced morph targets; with VRM gone
this was a real choice, and it was made on measurement rather than on preference.

The arithmetic. Bevy stores morph targets as an R32Float 3D texture at nine floats per vertex per
target — position, normal *and* tangent deltas, unconditionally, with no sparse form. Over the
merged 3,643-vertex skin mesh that is 128 KiB per target: **6.5 MiB for ARKit-52**, 8.4 MiB with
Oculus-15 as well, on top of a 12 MiB atlas. Bones cost nothing comparable — 25 joints are in use
of Bevy's 256, the skinning uniform is 16 KiB whether or not it is filled, and a face rig of about
thirty adds no per-vertex data at all. Neither costs record bytes; both are generated.

What settled it was not the ratio but a structural trap: **a morph target image is sized by the
whole mesh**, so face targets pay for all 3,643 vertices including the feet, and confining them to
the 189 head-zone vertices means giving the head its own mesh — a **draw call**, on the half of the
budget that is already failing (five against three). Bones move that number the right way instead,
because the eyelids become a pose rather than geometry.

This is not an improvisation. It is **pose space deformation** (Lewis et al., 2000) — corrective
displacements interpolated in a pose space of joint angles and added to the skinned surface —
shipped by Maya, Houdini and Unreal. MetaHuman, Epic's flagship real-time human, is the same shape:
Rig Logic drives a facial *joint* hierarchy with RBF solvers mapping joint rotations onto corrective
bones and shapes. It is not blendshape-primary. §1's finding 2 already described this pattern
without naming it — "additive identity/pose/**corrective** layers before one standard LBS pass" is
PSD — so the decision follows the research the plan was built on rather than departing from it.

One way this repository is better placed than the studios that invented the technique: in
production a corrective is a hand sculpt, and that labour is why they are rationed. Here
`skull::reshape` is already an analytic parametric head deformation, so a corrective is that
function evaluated at another parameter and subtracted. The usual reason to keep the set small does
not apply; memory is the only one left, and memory agrees.

Two prerequisites came out of the decision, neither previously costed:

- **#59 — weld the face into a continuous surface.** `Features::build` appends nose, brows, lips
  and ears as detached rigid solids. Nothing can deform across a boundary that does not exist, so
  this is true whichever way the decision had gone, and WS4 cannot start without it.
- **#60 — `anim/` has no pose-space corrective driver.** Bevy blends morph weights but nothing
  reads a joint angle and produces one. That solver is ours, belongs beside the inertializer, and
  must stay engine-agnostic so the software renderer can use it too. **Not to be built
  speculatively** — the decision was bones first, correctives where measurement demands them.

`Role::Facial` (#34) already exists and the body skin already ignores it, so nothing blocks WS4.

*Sources for the technique, since the research dossiers predate the decision:* [Lewis et al., pose
space deformation](https://en.wikipedia.org/wiki/Pose_space_deformation) and its shipped forms in
[Maya](https://help.autodesk.com/cloudhelp/2018/ENU/Maya-CharacterAnimation/files/GUID-45D389D6-B8E4-4225-B27B-9927BB61C28D.htm)
and [Houdini](https://www.sidefx.com/docs/houdini/nodes/sop/posespacedeform.html);
[Rig Logic for MetaHumans](https://kalyansthupili.wordpress.com/2025/04/14/demystifying-rig-logic-for-metahumans/)
and its [RBF corrective layer](https://www.cgchannel.com/2026/03/metahuman-dna-add-on-for-blender-gets-new-rbf-editor/);
practitioner accounts of the joint-primary hybrid at
[Tech-Artists.Org](https://www.tech-artists.org/t/blendshapes-vs-joint-driven-facial-set-up/1127)
and [Polycount](https://polycount.com/discussion/217908/cost-of-morphs-blendshapes-vs-bones).

### 2026-08-02, the complexion (#39, half closed)

The melanin ramp was two stops interpolated between a pale colour and a deep one, and measuring it
against the **Monk Skin Tone Scale** — Ellis Monk's ten published shades, developed with Google —
found three faults rather than the one the issue reported. Hue was flat at 18–21° from end to end
where real skin runs 30–40° pale and falls through the high teens; the ramp did not reach far
enough into the dark; and, worst, **saturation climbed monotonically into the deepest tones** where
the reference peaks in the deep middle and falls away. A saturated colour at a dark value is a
garish orange, and that is what full melanin produced. The undertone axis was an absolute offset,
so it moved 15% of the blue in the palest complexion and **100%** of it in the deepest.

The ramp is now ten stops fitted to that reference, undertone is a hue *rotation* that preserves
value and saturation, and blush is scaled down by melanin because pigment sits above blood and
absorbs what would have shown through it. `AvatarConfig::complexion` and `render -- --skin` were
added so the axes can be walked by eye, symmetric with `--hair`.

Two things worth carrying forward. **The reference is a set of colour chips, not a ramp** — its
outermost shades are nearly neutral, which is right under flat neutral light and rendered as a
colourless mannequin at one end and charcoal grey at the other. Saturation is held up at both ends
against the reference, deliberately, toward the stylised target in the table above. And **fitting
it verbatim measured correct and looked wrong**, which is the second time in this file that a
number has been right and a body has not; the deviation was found by sampling rendered pixels, not
by reasoning about albedo.

The **geometry** half of #39 — `FaceParams` is four prominence scalars over one fixed skull, and
`skull.rs` ships six const tables no record touches — is split out as **#61**, deliberately
sequenced *after* #59: widening the face axes means tuning feature shapes by eye against a topology
#59 is about to replace.

### Where this leaves the gate

**Open and blocking:** #38 (spring chains, now unblocked by #34) is the last of the P1 band. **Gate
#6 (#36) is closed to re-judging until #34–#39 clear** — which, with #39 closed, means #38 alone.
Behind it and not gating: #59 before WS4 can start, then #61, then #60 if measurement asks for it.

## 1. Headline research findings

1. **First-of-kind.** No parametric 3D avatar lexicon exists on atproto; rpg.actor is the only shipped parametric-avatar-on-PDS system and it is 2D sprite layers. Conventions to lift from it: singleton pointer records, a `source` AT-URI on baked artifacts pointing back at the parametric record, an `isCustom` flag protecting hand-edits from recomposition, cross-PDS give/item attestation pairs for granted equipment, lexicon JSONs at predictable URLs, and granular OAuth `repo:<nsid>` scopes (live in their production). Constraints: 1 MB hard record limit (we keep the 100 KiB discipline), 50 MB blobs, blob GC on last-reference deletion, and **no CDN for arbitrary binary blobs** — serving baked GLBs needs our own caching layer. (research-01)

2. **Convergent body recipe, license-clean.** SMPL, MakeHuman, and runevision's creature project independently converge on: small semantic parameter vector → skeleton *derived from* the shaped geometry (joint regressor / joint-cube idea) → mesh generated over it → skin weights derived analytically at generation time → additive identity/pose/corrective layers before one standard LBS pass. SMPL-family and SMAL are MPI non-commercial (design reference only); every technique we actually need (B-Mesh, Spore, Pinocchio, sphere-meshes, implicit skinning, delta mush) is a published paper. Parameter-space law confirmed three ways: hand-authored semantic macro axes with encoded correlations — never PCA. (research-02)

3. **Mesh path: capsule node-graph → B-Mesh quad cage → Catmull-Clark.** Quad-dominant cages with skeleton-aligned edge flow give the clean deforming silhouettes the fidelity target needs; implicit/SDF math is used only to fair joint regions. Humanoids and quadrupeds are just different node graphs on the same engine. The acknowledged hard 20% is the 3+-branch joint-merge topology — prototyped first, before anything else depends on it. (research-02 synthesis, Architecture A growing into C)

4. **Animation is goal-space from day one.** The only proven one-animation-set-many-morphologies system is Spore's (Hecker et al., SIGGRAPH 2008): motion stored as semantic part queries plus goals in normalized body space, reconstructed per body by a fast IK solver — corroborated by Unreal's IK Rig (named chains + goals) and Bereznyak's GDC 2016 IK Rig. We are better-positioned than Spore because our generator names its own parts. Bevy ships a solid low-level AnimationGraph but **no retargeting, no IK, no inertialization, no foot placement** — exactly the layer these crates occupy; no incumbent crate exists. Shippable data: 100STYLE (CC BY 4.0) + code-authored Rosen-style sparse poses; Mixamo and LaFAN1 can never live in the crate. Motion matching is rejected as structurally incompatible with parametric bodies. Retrofitting goal-space later would invalidate authored clips, hence day one. (research-03)

5. **The look is ~70% textures, not shaders.** All three reference games are plain PBR with painterly inputs: painted bevels + baked AO in albedo, micro-detail confined to roughness/spec, subdermal "blood map" faked SSS, solid clumped mesh hair (no alpha sorting), deterministic non-physical eye highlights, no outlines. Most of the win lands in the symbios texture stack. Custom shaders are a short list: skin ExtendedMaterial (wrap lighting → Penner pre-integrated LUT with curvature baked at mesh-gen time — cheap because we own the mesher) and eye parallax v2. WebGL2 shapes the budget: one draw per skinned mesh → **1–3 skinned meshes per avatar**, 15–30k tris, quality tiers that degrade by omission (SSAO/TAA/OIT/batching are native/WebGPU-only). (research-04)

6. **Dress needs no cages and no cloth sim.** Because the body is procedural, hair/outfit/accessory generators evaluate against its analytic surface. Substrate: a **landmark/measurement API** (named anchors + scalar measurements from the record) and a **≤32-zone coverage bitmask**. Hair = VRoid's model (parameter records generating strand-tube groups on a scalp offset surface; spring physics per group). Tight clothing = the body surface re-evaluated at +ε with covered zones suppressed at generation — intersection-proof by construction. Loose garments = GarmentCode's component taxonomy evaluated directly in 3D as swept panels from body rings. Physics = VRMC_springBone semantics verbatim, shared by hair, tails, hems, accessories. Rejected: Roblox cage deformation, RPM body-replacing outfits, full cloth simulation. (research-06)

7. **VRM export is buildable, not adoptable.** No Rust VRM writer exists. Assemble: `gltf-json` (serde glTF 2.0 schema) + pixiv's `vrm-spec` crate (Apache-2.0 serde structs for all VRMC_* extensions) + a ~50-line GLB writer. VRM 1.0 canonical: 15 required humanoid bones, normalized T-pose rest (identity rotations — also required by 0.x and makes inverse bind matrices trivial), +Z facing (matches our authoring convention); optional 0.x bake for VSeeFace is a yaw-180 + different extension JSON. Creatures = plain GLB (VRM is confirmed humanoid-only; GLB is also the Resonite path). PNG textures only (no KTX2 in the VRM ecosystem). CI: mrxz/vrm-validator. Expectation check: VRChat does **not** consume VRM; conformance buys Cluster, three-vrm/web, Godot, Unity, Blender, VTuber apps. (research-05)

## 2. Lexicon sketch (`network.symbios.avatar.*`)

Design work happens in WS0 (#2); this is the starting shape:

- **`network.symbios.avatar.avatar`** — the parametric record, one per avatar, **named/TID rkeys** (wardrobe model). Contains the full parameter tree: body-plan macro axes, face, hair groups, outfit, accessories, palette, seed + per-category locks. Additive-evolution discipline throughout (new fields optional, field-level serde defaults — the overlands record-scaling lessons apply verbatim). Self-imposed budget: 100 KiB (protocol limit 1 MB; small records drive good parameterization — a Wii Mii was 74 bytes, Spore kept recipes deliberately tiny for sharing).
- **`network.symbios.avatar.profile`** (singleton `self`) — points at the default avatar's rkey; room for future per-app preferences.
- **`network.symbios.avatar.bake`** — optional baked artifacts keyed to a source avatar: thumbnail PNG blob always; optionally a baked GLB blob (≤50 MB, "within reason" — actual policy decided by observed content size). Carries `source` AT-URI + `isCustom` (rpg.actor conventions). Non-symbios consumers get renderable output without implementing the generator.
- Publication: lexicon JSONs at predictable URLs **and** `com.atproto.lexicon.schema` records + `_lexicon` DNS; clients request granular OAuth `repo:network.symbios.avatar.*` + `blob:*` scopes.
- Share codes: record → base32 compact code (MH-Wilds-style), QR-friendly.

## 3. Crate architecture

**symbios-avatar** (engine-agnostic):
- `record/` — lexicon types + serde, seed + category-lock model, share-code codec.
- `plan/` — body-plan graph (typed parts: spine chain, limb chains, head, tail, digits, sockets), macro-axis → graph resolution, constraint/correlation layer, semantic capability tags (ground contacts, graspers, gaze) feeding both meshing and animation.
- `mesh/` — capsule graph → B-Mesh quad cage → subdivision; skeleton derivation; analytic weights + smoothing; landmark/measurement API; zone bitmask; canonical UV charting (fixed face layout).
- `dress/` — hair groups, tight-garment re-evaluation, swept-panel loose garments, accessory generators + sockets, region/layer arbitration.
- `face/` — a **bone-driven** macro rig on `Role::Facial` joints, with ARKit-52 naming and
  Oculus-15 visemes over it, and generated pose-space correctives only where bones are measured to
  fail (#35). The face must be welded into a continuous surface first (#59); the driver that turns
  a joint angle into a corrective weight lives in `anim/`, not here (#60).
- `anim/` — goal-space clip format, two-bone + FABRIK IK, gait engine (phase + duty-cycle over N contacts), inertialization, spring chains (VRMC_springBone semantics, #38), look-at, and the pose-space driver that turns joint angles into corrective weights (#60). Pure math, no engine deps — the software renderer needs all of it too.
- `texture/` — character generators atop symbios-texture: skin stack (melanin/subdermal/freckle/stubble/AO-bevel), iris, hair strand, fabric weave/knit/print, gradient-map palette baking.
- `export/` — glTF/GLB writer. (VRM dropped; see §0.)

**bevy_symbios_avatar**: spawn path (record → entities, 1–3 skinned meshes per avatar), skin/eye ExtendedMaterials, AnimationGraph bridge + IK/inertialization/spring/foot-placement/look-at systems, LOD hooks, headless render-tool integration (contact-sheet review loop), creator-UI-agnostic parameter surface. Bevy animation events API is still churning (0.19 event rearchitecture) — keep this bridge thin.

## 4. Workstreams (chainlink issues)

| # | Workstream | Notes |
|---|---|---|
| #2 | WS0 Lexicon + records + share codes | high |
| #3 | WS1 Body engine | high — **joint-merge topology prototype first** |
| #4 | WS2 Look (textures, eyes, hair, skin material) | |
| #5 | WS3 Motion (IK, foot placement, inertialization, gait, goal-space) | high |
| #6 | Vertical slice checkpoint | gate for everything after |
| #7 | WS4 Face | bone-driven, per #35. Blocked by #59 (the face is currently detached solids) |
| #8 | WS5 Dress | |
| #9 | WS6 Creatures | |
| #10 | WS7 Export (GLB) | **split**: the assembly half is pre-gate and **done** (#28, #29); the writer half moves to immediately post-gate. VRM dropped. |
| #11 | WS8 Creator UX + Overlands adoption (incl. eventual vehicle-chassis migration) | spans sibling repos. **Started at the gate, not last**: #37 stood the Bevy crate up, so the gate's "and in-app" clause is now executable. The creator UI itself is still post-gate. |

Milestone **"v0.1 vertical slice"** = #2–#6: one parametric humanoid walking/idling/running on terrain with blink + look-at, one outfit, one hair style, judged against the Fortnite/Overwatch bar via render contact sheets and in-app. AAA feel is won by iteration; the slice is where we find out early.

## 5. Risks

1. **B-Mesh joint-merge topology** — the hard 20% of meshing; prototyped before dependence. *Retired
   as a risk: it was built, and the review defended it.*
2. **Goal-space encoding/reconstruction subtlety** — highest-risk subsystem; mitigated by landing IK/inertialization/springs first (they deliver feel even with a tiny pose set) and by Spore's shipped precedent. Springs are the piece still missing (#38).
3. **Unproven Bevy quality ceiling** — no published Bevy project shows this character fidelity; nothing engine-side blocks it, but the gap is art-direction iteration. *There is now an instrument for it (#37), and the first thing it showed was a difference the software renderer alone could not have attributed.*
4. **WebGL2 constraints** — draw-per-skinned-mesh, no SSAO/TAA; tiers degrade by omission; WebGPU improves everything later. *Triangles are inside the budget (#40); draws are not — five against three, and the excess is the eye pair, so #35 decides it.*
5. **Bevy animation API churn** — thin bridge, engine-agnostic core.
6. **A budget that only holds for the default record.** Until #40, a body's cost was nearly
   independent of its parameters, so one measurement covered the space. It is not any more: a lock
   of hair is priced by how far it travels, and a head of hair now ranges over more than a factor of
   five. Anything else whose cost a record can move needs the same treatment — a ceiling enforced in
   the generator, not a number asserted about one body.

*(The former risk 6, VRM conformance fiddliness, went away with VRM.)*
