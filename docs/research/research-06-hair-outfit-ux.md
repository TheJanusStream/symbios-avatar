# Dossier: Parametric Hair, Clothing, Accessories & Character-Creator UX

## 1. Procedural hair

### VRoid Studio model (best-documented parametric hair authoring)
- All hair generated on a **guide mesh** — invisible offset surface inflated around the scalp; guide has height/offset so layers (base coat, main bundles, stray hairs) live at different radii.
- Two modes: freehand strokes on the guide, or **procedural hair groups** generated entirely from parameters — the model to copy: one small record → a full hair section.
- Per-group parameters: hair count + spread across scalp region; cross-section profile (diamond/triangle/flat/rounded); 2D curve editor for root-to-tip path; thickness taper curve; width; smoothness; twist (amplitude + interval → curls); per-strand X/Y nudge; texture params (highlight position, offset/rotation, per-channel colors: base/shade/highlight/outline).
- Physics per **bone group**: select strands → one strand is the axis, whole group follows a single spring chain. Params: bone count (few=stiff, many=fluid), fixation point (0..1 along strand), stiffness (0.01–0.99), collision radius (head/body collider keeps hair out of face).
- Takeaways: (a) hair = groups, each a parameter record generating N strand tubes on a scalp offset surface; (b) physics per-group not per-strand; (c) auto-generate bone groups with manual override.

### Stylized clumped hair structure
- Stylized hair = **solid clump meshes**: curve-strips or beveled tubes forming locks — low poly, no alpha, avoids sorting problems. Overwatch: main mass from strips, bangs/buns as tubes.
- For our system: sweep a profile along a spline = exactly the existing prim-sweep mesher — VRoid-style tubes natural fit; cards unnecessary without strand textures.
- Card/clump generation research if later needed: Strands2Cards (SIGGRAPH Asia 2025), DiffHairCard, UE Hair Card Generator, Houdini haircardgen.

### VRMC_springBone 1.0 = canonical runtime spring-chain model
- Per-joint: stiffness (restore to rest), gravityPower + gravityDir, dragForce (0–1), hitRadius (m).
- Update: Verlet — next tail = inertia·(1−drag) + stiffness-toward-rest + gravity, then length-constrain to rest bone length; collide sphere/capsule colliders (push out, re-constrain); derive rotation from tail via quaternion.
- Colliders grouped; springs reference groups. **Center node** evaluates inertia in local space so locomotion doesn't whip hair (gravity stays world) — important for avatars on vehicles.
- Maps ~1:1 onto our per-node record style; trivially implementable in Bevy.

## 2. Layered clothing over varying bodies

### Roblox layered clothing — cage deformation
- Every body has an **outer cage**; every item an **inner cage** (body it was modeled on) + own outer cage for the next layer. Deformation transfers delta between item's inner cage and body's outer cage; up to 6 layers stack. Engine also does Hidden Surface Removal (covered geometry removed to stop poke-through). Cage authoring is the labor cost.
- Relevance: cages solve "artist mesh onto arbitrary body." Since our clothing is procedural, skip cages entirely — generate garments from the body's own parametric surface. Strictly easier.

### MakeHuman proxy/clothes — most reusable idea
- .mhclo binds every clothing vertex to a **triple of base-mesh vertices + offset in the local normal space** of that triangle — clothes re-fit automatically as the body morphs. Extras: per-axis scale factors from reference vertex pairs; **delete-groups** listing body vertices hidden under the garment. = Roblox cages without the cage: bind garment control points to body-surface barycentric anchors + normal offsets.

### Skin-tight clothing as body displacement / texture
- WoW-style: most clothing = texture regions composited on the body mesh + small set of swap submeshes (~90 toggleable).
- Second-skin: duplicate body patch, offset along normals, retexture. For procedural: tight layer = **body mesh re-evaluated with +ε radial offset and garment material**, trim curves for hems. Zero intersection risk by construction.

### Body hiding under clothes
- **Zone culling**: partition body + garments into bitmask zones (≤32/material); garment declares covered zones; vertex shader kills covered vertices (NaN position). Star Citizen works this way. Character Creator 3 stores hidden-vertex info in the clothing asset.
- Sims 4 generalizes: **regions + layers** — every asset declares regions + layer value; highest layer wins per region — resolves boots-vs-pants, hat-vs-hair without bespoke pairs.

### Cloth sim vs rigged
- Majority of games: clothes skinned like skin — cheap, robust, no penetration, no secondary motion. Full sim: expensive, collision proxies needed, tunneling/jitter.
- Sweet spot: **hybrid** — skinning blended 5–20% into sim for stability, or spring-bone chains on skirt/coat hem bones (same machinery as hair) + normal-map wrinkle blending (Uncharted 4).
- Rust/Bevy: bevy_silk (CPU Verlet cloth, only maintained option; check Bevy-version currency), bevy_verlet older. Neither does body-collision proxies. **VRM-style spring bones cover 90% of avatar need at a fraction of the cost.**

### Ready Player Me (anti-pattern for us)
- Outfits are full-body assets that INCLUDE the body mesh — outfit replaces the body, sidestepping layering. Incompatible with per-user body-proportion variation.

## 3. Accessory attachment

- **Named bone-relative sockets** universal: Roblox fixed vocabulary (HatAttachment, HairAttachment, FaceFrontAttachment, NeckAttachment, collar/shoulder, WaistFront/Center/Back, BodyBack/Front) — accessories carry their matching attachment so offsets compose; Accessory Fitting Tool previews across body types.
- VRoid accessories are themselves **parametric** (glasses: frame shape/thickness/size/texture; ears: placement/roundness/fur/size), positioned relative to a default anchor. VRM export just bone-parents the mesh — no dedicated accessory extension.
- **Fitting to parametric heads**: eyewear research parameterizes frames against facial landmarks (temple width, nose-bridge height/width, ear-tragus position, pupillary distance). For us: heads are generated from our own parameters, so **landmarks are computable analytically** — accessory generators take (interocular distance, temple width, ear anchors, crown circumference) as inputs derived from the record, not fitted after the fact.

## 4. Parametric garment description

- **GarmentCode** (ETH, SIGGRAPH Asia 2023): DSL treating garments as **hierarchical parametric component compositions** (skirt: flare/godet/pencil/gather; bodices; pants; sleeves w/ cuffs; collars), parameterized by **body measurements** + **style parameters**; low-level tailoring automated inside components. GarmentCodeData: same programs re-evaluated across body shapes = made-to-measure free. Strongest conceptual template: `component tree + body measurements + style params → geometry`.
- Game-practical simplification: no sewing/flattening — evaluate panels **directly in 3D** as offset surfaces over body regions (bodice = torso surface + offset + looseness falloff; skirt/cape = swept surface from waist/shoulder curve with flare + length; sleeve = tube around arm capsule). Keep GarmentCode's component taxonomy + measurement-driven parameterization, not the 2D patterns.
- **Procedural fabric**: Substance Weave Generator parameter set (weave tightness, thread thickness, twill pattern, disruption) + knits + prints — maps onto our sovereign-texture generator system. Mind feature-size vs garment footprint + UV conventions (plank-stagger/brick-UV memory lessons).

## 5. Character-creator UX

- **Sims 4** (GDC): no sliders — **color-ID hotspot picking** (regions rendered with unique color, shader mouse pick), drag-to-sculpt at three granularities (Top/Macro/Micro), symmetric editing, **resistance + red highlight at extremes** (soft-limit feedback), morphs = blend targets + bone deltas + **deformation maps** (deltas in bitmaps, topology-independent). **Smart randomization** via asset tags (age/gender/archetype/style/color palette) with graceful tag-pruning.
- **BG3**: curated discrete options — zero broken outputs, fast good results.
- **Mii**: parts-based caricature; super-deformed models make tiny discrete changes strongly distinct — likeness through exaggeration. **Coarse discrete parameters beat fine continuous sliders** for likeness and shareability in stylized settings.
- **Preset-blend vs sliders**: presets = saved slider vectors over the same parameter space, blendable. Taxonomy: presets (archetype) → macro sliders (region) → micro sliders (feature) = Sims Top/Macro/Micro.
- **Randomize + locks**: Ship of Heroes ships master randomize + checkbox locks per option + hierarchical randomizers (3 major: look/costume/colors; ~10 secondary). Our seed-hunt + pin_axis_row 🔒 mechanic (#1005) is directly transplantable and MORE principled (deterministic seed hunt vs naive re-roll).
- **Save/share**: MH Wilds short alphanumeric preset codes (hunter + Palico); DD2 slider formulas on boards; Elden Ring community JSON tools; Mii QR. **Native compact codes win** — small-record + seed architecture makes this nearly free (record → base32/QR).
- **Colorways**: curated swatch ramps by default, full picker behind "advanced" (consistent with #900 realistic palettes).
- **Nikke/Stellar Blade**: contribution is **presentation** — invest in the preview stage (turntable, pose, lighting, backdrop); it sells procedural outfits.

## 6. Creature customization

- **Spore**: body = metaball surface along editable spine (pull to extend, scroll to fatten); details = **rigblocks** — hand-authored parts with parameterized deformation handles snapped onto the body — the hybrid that **guarantees animation compatibility** (anti-brokenness safeguard). Auto-texturing 3 layers (base/pattern/detail). **Creature recipe kept deliberately small for network sharing** — same constraint as our 100 KiB budget; it DROVE the high-level parameterization. Editor lessons: keep manipulable primitives simple (spheres/capsules — non-spherical manipulation UI was "clunky"); "Bugs + Player Creativity = Features."
- **Palico creator (MH Wilds)**: huge perceived variety from a small space — fur pattern type + colors, ear type, tail type, fur length, voice. **Pattern × palette × part-type-enum on a fixed chassis = variety with zero brokenness** — aligns with BlobGroup constraint system.

---

## Synthesis: recommended architecture

Principle: **small records + deterministic generators evaluated against the body's own parametric surface.** The body is procedural, so its analytic surface and landmarks are always known — no cages, no mesh bindings; just re-evaluate generators when body params change.

**Shared substrate (build first)**
1. **Body landmark/measurement API**: from an avatar record, compute named anchors (crown, temples, ear points, eye line, neck ring, shoulder sockets, chest/waist/hip rings, wrist/ankle rings, tail-base) + scalar measurements (head circumference, interocular distance, torso lengths, limb radii). The single contract hair/outfit/accessory generators consume.
2. **Zone/coverage enum** on the body surface (bitmask ≤32: scalp, face, neck, upper-arm L/R, torso front/back, pelvis, thigh, calf, foot, tail…) for garment targeting AND under-clothing culling.

**v1 — minimum shippable**
3. **Hair = VRoid model**: scalp offset surface; N hair-group records {region, strand_count, profile, taper curve, length, curl, spread, part-line, color} → strand tubes via existing sweep mesher. Solid clumped tubes, no alpha cards. ~6–10 archetype presets the seed hunts across.
4. **Tight clothing = body re-evaluation**: garment record {covered zones, ε offset, hem/neck/sleeve trims, material+pattern} → re-emit covered patches at +ε with garment material; covered-zone bits suppress base skin (culling by construction — we generate the mesh). Gives shirts/pants/suits/gloves/boots across any proportions, zero intersection.
5. **Accessories = parametric generators + named sockets**: record {socket, generator params, offset}; generators take measurements from (1). Reuse prop-assembly conventions (nest/assemble transform gotchas).
6. **Creator UX v1**: category tabs (body/head/hair/outfit/accessories/colors) backed by presets = named parameter records + **seeded re-roll with per-category locks** (keep deterministic seed-hunt semantics); curated palette ramps per theme + advanced picker; **share = compact record code** (base32/QR). Randomize hierarchy: master + per-category.

**v2 — motion & looseness**
7. **Spring-bone chains (VRMC_springBone semantics verbatim)** as a Bevy system; auto-generate bone groups per hair group (user-facing knobs: bone count, fixation point, stiffness, collision radius). Reuse for tails, skirt hems, coat tails, dangling accessories. Skip bevy_silk/full cloth.
8. **Loose garments = swept-panel generators** (GarmentCode taxonomy, 3D-direct): skirt/cape/coat swept from body rings (+ flare, length, gather), skinned to ring bones, hems on spring chains. Sims-4 **region+layer arbitration** for loose-over-tight.
9. **Fabric pattern generators**: weave/knit/print + colorway layer in the texture family.

**v3 — creatures & polish**
10. **Creature creator = Spore-lite on capsule chains**: pull/scale spine/limbs (spherical manipulanda), parts as parameterized generator "rigblocks" on sockets, constraint system as anti-brokenness guarantee; variety via pattern × palette × part-enum (Palico). Hair/outfits apply via the same landmark API (fur = hair groups on body zones; harnesses = ring-swept garments).
11. **Presentation polish**: staged preview (pose/lighting/turntable); later direct-manipulation face editing via color-ID hotspot picking + soft-limit resistance, mapping drags onto the same record parameters.

Rejected: mesh-asset clothing + cages (Roblox) — wrong for no-shipped-assets; body-replacing outfits (RPM) — incompatible with proportion variation; full cloth sim — cost/instability; 2D sewing-pattern flattening — keep only the taxonomy.
