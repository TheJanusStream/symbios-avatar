# symbios-avatar — Project Plan

Parametric 3D avatars whose source of truth is a small AT Protocol record —
rpg.actor's idea carried into 3D, at AAA-adjacent render and animation quality.
Two crates in the symbios pattern: **symbios-avatar** (engine-agnostic, this
repo) and **bevy_symbios_avatar** (Bevy integration, sibling repo). Symbios
Overlands becomes the first consuming application once the engine stands.

The plan was synthesized from six web-research dossiers in
[docs/research/](research/), which stay in the repository as the record of what
was found and why the decisions below were made.

## Locked decisions

| Decision | Choice |
| --- | --- |
| Visual target | Stylized semi-realistic (Fortnite / Overwatch / Sea-of-Thieves class) |
| Geometry | Fully procedural — no shipped mesh assets, records stay tiny |
| Scope | Humanoid + creatures from day one (goal-space animation mandatory) |
| Interop | Native atproto records are canonical; **GLB export only** — VRM 1.0 was considered and dropped; research-05 stays as the record of what conformance would have cost |
| Lexicon root | `network.symbios.avatar.*` (symbios ecosystem, outside overlands) |
| Multiplicity | Wardrobe of named avatars per identity, one marked default |
| Baked artifacts | Stored on PDS within reason; degree decided along the way by content size |
| Overlands chassis | All four existing families — vehicles included — eventually migrate onto this system |
| Face parameter space | ARKit-52 naming + Oculus-15 visemes |
| How the face animates | **Bone-driven macro rig, ARKit-52 naming-only**, with generated pose-space correctives held in reserve for where bones are *measured* to fail |
| Runtime motion | The procedural layer (IK, gait, footing, goal-space clips) is the only runtime motion source; the baked clip set is a development reference (see [clips.md](clips.md)) |

Dropping VRM retired the T-pose conflict, freed dual-quaternion skinning, and
removed VRM as a constraint on mesh decomposition and on how the face animates.
Creatures staying day one means humanoid-only assumptions are a live
architectural problem wherever they appear; the pattern to design against is a
rule stated about a body *part* where it should be stated about what that part
*does*.

Why bones and not morph targets, in brief: Bevy stores morph targets as an
R32Float 3D texture at nine floats per vertex per target, unconditionally and
sized by the *whole* mesh — ARKit-52 over the merged skin mesh costs megabytes,
or a dedicated head mesh costs a draw call on the half of the budget that is
hardest to hold. Bones cost neither, and the eyelids become a pose rather than
geometry. This is pose space deformation (Lewis et al., 2000) as shipped by
Maya, Houdini and Unreal's MetaHuman: a facial joint hierarchy first, corrective
shapes only where measurement demands them. This crate is better placed than the
studios that invented the technique, because `skull::reshape` is an analytic
parametric head deformation — a corrective is that function evaluated at another
parameter and subtracted, not a hand sculpt.

## Headline research findings

1. **First-of-kind.** No parametric 3D avatar lexicon exists on atproto;
   rpg.actor is the only shipped parametric-avatar-on-PDS system and it is 2D
   sprite layers. Conventions to lift from it: singleton pointer records, a
   `source` AT-URI on baked artifacts pointing back at the parametric record, an
   `isCustom` flag protecting hand-edits from recomposition, cross-PDS give/item
   attestation pairs for granted equipment, lexicon JSONs at predictable URLs,
   and granular OAuth `repo:<nsid>` scopes (live in their production).
   Constraints: 1 MB hard record limit (we keep the 100 KiB discipline), 50 MB
   blobs, blob GC on last-reference deletion, and **no CDN for arbitrary binary
   blobs** — serving baked GLBs needs our own caching layer. (research-01)

2. **Convergent body recipe, license-clean.** SMPL, MakeHuman, and runevision's
   creature project independently converge on: small semantic parameter vector →
   skeleton *derived from* the shaped geometry → mesh generated over it → skin
   weights derived analytically at generation time → additive
   identity/pose/corrective layers before one standard LBS pass. SMPL-family and
   SMAL are MPI non-commercial (design reference only); every technique we
   actually need (B-Mesh, Spore, Pinocchio, sphere-meshes, implicit skinning,
   delta mush) is a published paper. Parameter-space law confirmed three ways:
   hand-authored semantic macro axes with encoded correlations — never PCA.
   (research-02)

3. **Mesh path: capsule node-graph → B-Mesh quad cage → Catmull-Clark.**
   Quad-dominant cages with skeleton-aligned edge flow give the clean deforming
   silhouettes the fidelity target needs. Humanoids and quadrupeds are just
   different node graphs on the same engine. The acknowledged hard 20% is the
   3+-branch joint-merge topology — prototyped first, before anything else
   depended on it, and it held. (research-02)

4. **Animation is goal-space from day one.** The only proven
   one-animation-set-many-morphologies system is Spore's (Hecker et al.,
   SIGGRAPH 2008): motion stored as semantic part queries plus goals in
   normalized body space, reconstructed per body by a fast IK solver —
   corroborated by Unreal's IK Rig and Bereznyak's GDC 2016 IK Rig. We are
   better-positioned than Spore because our generator names its own parts. Bevy
   ships a solid low-level AnimationGraph but **no retargeting, no IK, no
   inertialization, no foot placement** — exactly the layer these crates occupy;
   no incumbent crate exists. Motion matching is rejected as structurally
   incompatible with parametric bodies. Retrofitting goal-space later would
   invalidate authored clips, hence day one. (research-03)

5. **The look is ~70% textures, not shaders.** All three reference games are
   plain PBR with painterly inputs: painted bevels + baked AO in albedo,
   micro-detail confined to roughness/spec, subdermal "blood map" faked SSS,
   solid clumped mesh hair (no alpha sorting), deterministic non-physical eye
   highlights, no outlines. Most of the win lands in the symbios texture stack.
   Custom shaders are a short list: skin ExtendedMaterial and eye parallax.
   WebGL2 shapes the budget: one draw per skinned mesh → a handful of skinned
   meshes per avatar, 15–30k tris, quality tiers that degrade by omission.
   (research-04)

6. **Dress needs no cages and no cloth sim.** Because the body is procedural,
   hair/outfit/accessory generators evaluate against its analytic surface.
   Substrate: a landmark/measurement API (named anchors + scalar measurements
   from the record) and a zone coverage set. Tight clothing is the body surface
   re-evaluated at +ε with covered zones suppressed at generation —
   intersection-proof by construction. Loose garments are swept panels from body
   rings (GarmentCode's component taxonomy evaluated directly in 3D). Physics is
   VRMC_springBone semantics, shared by hair, tails, hems, accessories.
   Rejected: Roblox cage deformation, RPM body-replacing outfits, full cloth
   simulation. (research-06)

7. **GLB is the export target.** No Rust VRM writer exists, and VRM is
   confirmed humanoid-only — creatures were always plain GLB. Native records
   plus our own GLB writer serve every consumer we care about; research-05
   records the assembly a VRM path would have needed had it been kept.

## Lexicon (`network.symbios.avatar.*`)

The published schemas live in [lexicons/](../lexicons/).

- **`network.symbios.avatar.avatar`** — the parametric record, one per avatar,
  named/TID rkeys (wardrobe model). Contains the full parameter tree: body-plan
  macro axes, composites, face, hair regions, outfit, palette, seed +
  per-category locks. Additive-evolution discipline throughout: new fields
  optional, container-level serde defaults, unknown fields preserved.
  Self-imposed budget: 100 KiB (protocol limit 1 MB; small records drive good
  parameterization — a Wii Mii was 74 bytes, Spore kept recipes deliberately
  tiny for sharing).
- **`network.symbios.avatar.profile`** (singleton `self`) — points at the
  default avatar's rkey; room for future per-app preferences.
- **`network.symbios.avatar.bake`** (future) — optional baked artifacts keyed to
  a source avatar: thumbnail PNG blob always; optionally a baked GLB blob
  (≤50 MB, "within reason" — actual policy decided by observed content size).
  Carries `source` AT-URI + `isCustom` (rpg.actor conventions). Non-symbios
  consumers get renderable output without implementing the generator.
- Publication: lexicon JSONs at predictable URLs **and**
  `com.atproto.lexicon.schema` records + `_lexicon` DNS; clients request
  granular OAuth `repo:network.symbios.avatar.*` + `blob:*` scopes.
- Share codes: record → base32 compact code (MH-Wilds-style), QR-friendly.

## Crate architecture

**symbios-avatar** (engine-agnostic), as built:

- `record/` — lexicon types + serde, seed + category-lock model, share-code
  codec.
- `plan/` — body plans (humanoid, quadruped), the composite axes and the
  derivation layer that turns them into skeleton parameters, zones and zone
  sets.
- `skeleton`, `cage/`, `subdiv`, `hull`, `mesh`, `prim` — the mesher: capsule
  graph → B-Mesh quad cage → Catmull-Clark, with `PolyMesh` carrying positions,
  normals, UVs, skin weights and vertex colours.
- `rig/` — joint hierarchy with roles and zones, analytic skin binding,
  landmarks, the measured `Surface`, named prop sockets, patches.
- `face/` — skull shaping and refinement, features, eyes, the bone-driven
  expression layer, blink, talk, visemes.
- `extremity/` — hands and feet.
- `hair/` — five follicle regions measured from the built surface, style
  catalogues, card geometry, the painted layer.
- `dress/` — tight garments as re-evaluated body surface with zone suppression.
- `anim/` — pose, two-bone + FABRIK IK, foot placement, gait, speed/heading/
  turn, idle, swim, leap, inertialization, spring chains, goal-space clips,
  baked-clip playback, look-at, blink/gaze drivers. Pure math, no engine deps.
- `texture/` — geometry bake (position/normal/zone per texel) and the
  procedural skin painter, atop symbios-texture.
- `uv/` — zone-driven unwrap into named charts, importance-weighted packing.
- `avatar` — `Avatar::build`, the one record-to-renderable recipe.
- `gltf` + `retarget` — the clip *import* path (reader only; the GLB writer is
  still ahead).

**bevy_symbios_avatar**: spawn path (record → entities), skin/eye
ExtendedMaterials, AnimationGraph bridge + IK/inertialization/spring/
foot-placement/look-at systems, LOD hooks, headless render-tool integration,
creator-UI-agnostic parameter surface. Bevy's animation API still churns — keep
this bridge thin.

## Status, and what remains

A body can be described, built, given a face, eyes and hair, dressed in its own
skin and clothes, rigged, and walked — see the README's status section for the
full list. The instruments that judge it are catalogued in
[instruments.md](instruments.md), the triangle economy in
[budget.md](budget.md).

The milestone the work is judged against is **one parametric humanoid
walking/idling/running on terrain with blink + look-at, one outfit, one hair
style**, held to the Fortnite/Overwatch bar via render contact sheets *and*
in-app through `bevy_symbios_avatar`. Two instruments, one rule: a defect
visible in one renderer is the renderer's; a defect visible in both is the
body's.

Still ahead, in rough order:

- **GLB export** — a writer for baked avatar artifacts; the glTF module only
  reads today.
- **Loose garments and accessories** — swept panels and sockets; tight garments
  ship, loose ones do not.
- **Creature variety** — part generators, patterns, fur, and the constraint
  system over them; the quadruped proves the engine, the variety is unbuilt.
- **Creator UX** — seed-lock editor, share-code flow, staged preview, spanning
  both crates.

## Risks

1. **Goal-space encoding/reconstruction subtlety** — the highest-risk
   subsystem; mitigated by having landed IK, inertialization and springs first
   (they deliver feel even with a tiny pose set) and by Spore's shipped
   precedent.
2. **Unproven Bevy quality ceiling** — no published Bevy project shows this
   character fidelity; nothing engine-side blocks it, but the gap is
   art-direction iteration. The two-renderer rule exists to attribute defects
   while closing it.
3. **WebGL2 constraints** — draw-per-skinned-mesh, no SSAO/TAA; tiers degrade
   by omission; WebGPU improves everything later. Both halves are currently met
   and held by `tests/budget.rs`: triangles inside the 30,000 target, and four
   draws — skin, hair, cloth, eye — each justified by a material the others
   cannot provide.
4. **Bevy animation API churn** — thin bridge, engine-agnostic core.
5. **A budget that only holds for the default record.** A record can move a
   body's cost — a head of hair ranges over more than a factor of five — so any
   axis whose cost a record can move needs a ceiling enforced in the generator,
   not a number asserted about one body. The hair catalogue has this treatment;
   anything new and expensive owes the same.
