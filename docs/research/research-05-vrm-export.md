# Dossier: Exporting procedurally-generated rigged characters from Rust to VRM / glTF

*A point-in-time research dossier gathered while this crate was being designed. Kept as the record of what was found and why the decisions in [../plan.md](../plan.md) were made — external version numbers, prices and library gaps in it were true when written and are not maintained.*


## 1. VRM spec landscape (2025–2026)

- Stewarded by the **VRM Consortium** (Dwango, Cluster, REALITY/pixiv…); spec at github.com/vrm-c/vrm-specification; docs vrm.dev. Khronos collaboration announced to ratify VRM as official glTF extensions → ISO path.
- A VRM = standard glTF 2.0 **GLB** renamed `.vrm`, VRM data in root-level extensions (`VRMC_vrm` in 1.0; `VRM` in 0.x).
- **Adoption (mid-2026)**: VSeeFace = **0.x only, frozen**; Cluster = both (1.0 since Sep 2024); three-vrm = both (rotateVRM0 bridges facing change); UniVRM = both import+export; Godot (V-Sekai) imports both, exports 1.0; Warudo/VNyan accept VRM (1.0 mocap quirks reported).
- **Humanoid-only: CONFIRMED.** `VRMC_vrm.humanoid` is required; arm/leg/spine/head bones mandated. Creatures ⇒ plain glTF is correct.

## 2. VRM 1.0 requirements

Root extension `VRMC_vrm`, specVersion "1.0". Required: **meta**, **humanoid**. Optional: expressions, lookAt, firstPerson.

**meta**: required `name`, `authors` (≥1), `licenseUrl` **MUST** be https://vrm.dev/licenses/1.0/. Permission flags with defaults (avatarPermission=onlyAuthor, commercialUsage=personalNonProfit, allowRedistribution=false, modification=prohibited…). thumbnailImage: square PNG/JPG, 1024² recommended.

**humanoid** — **15 REQUIRED bones**: hips, spine, head, left/rightUpperLeg, left/rightLowerLeg, left/rightFoot, left/rightUpperArm, left/rightLowerArm, left/rightHand. Optional: chest, upperChest, neck, jaw, left/rightEye, toes, shoulders, 30 finger bones. Parent hierarchy **fixed** (upperChest only if chest exists; shoulder parents upperArm). Bone scales MUST be positive non-zero.

**expressions** (optional): presets happy/angry/sad/relaxed/surprised; visemes aa/ih/ou/ee/oh; blink/blinkLeft/blinkRight; lookUp/Down/Left/Right; neutral; + custom. Bind types: MorphTargetBind {node, index, weight}, MaterialColorBind {material, type, targetValue}, TextureTransformBind {material, scale, offset}. Flags: isBinary, overrideMouth/Blink/LookAt.

**lookAt**: type bone (rotates eye bones) or expression; offsetFromHeadBone; rangeMap {inputMaxValue, outputScale}.

**firstPerson**: per-mesh meshAnnotations (auto/both/thirdPersonOnly/firstPersonOnly).

**VRMC_springBone**: colliders (sphere/capsule on nodes), colliderGroups, springs = joint chains; joint params hitRadius, stiffness, gravityPower, gravityDir, dragForce; optional `center` node for inertial space.

**VRMC_materials_mtoon**: layered ON TOP of pbrMetallicRoughness; fallback KHR_materials_unlit. **MToon NOT required — plain KHR PBR metallic-roughness is legal VRM 1.0** and renders in three-vrm/UniVRM. For semi-realistic style, pbrMetallicRoughness + normal/emissive is fine.

**VRMC_node_constraint**: roll (twist bones), aim, rotation constraints.

**Runtime evaluation order** (spec-mandated): LookAt → Expressions → Constraints → SpringBone.

**Validators**: mrxz/vrm-validator (Khronos glTF-Validator fork extended to VRM 1.0, CLI JSON reports, browser at vrm-validator.fern.solutions) ← best CI candidate. UniVRM export dialog validates. Official samples in spec repo; three-vrm = de-facto behavioral reference.

## 3. Conventions that trip exporters

- glTF: right-handed, +Y up, meters. **VRM 0.x faces −Z; VRM 1.0 faces +Z.** (Our parts author front-+Z = matches 1.0; a 0.x bake must yaw 180°.)
- **T-pose**: 0.x requires fully normalized rest pose (identity rotations); 1.0 relaxes normalization but rest pose MUST still be T-pose. Safe play: bake normalized anyway (identity rotations, translations only) — same skeleton then serializes to 0.x with just the yaw flip.
- No negative scale (no mirroring). Emit TRIANGLES primitives. Tangents may be omitted.
- Textures: core glTF images PNG/JPEG only; thumbnail PNG/JPG square.
- Morph target names: `mesh.extras.targetNames: [string]` (universal convention; binds reference by index — names for DCC round-trips).

## 4. Rust glTF writing

- **gltf-json** (gltf-rs): full serde model of the glTF 2.0 schema, the standard way to WRITE glTF from Rust; official export example builds Root by hand + GLB writer. No high-level builder — accessor/bufferView bookkeeping is yours. `extensions` cargo feature exposes raw-JSON extension maps — hook for VRMC_*.
- **pixiv `vrm-spec` (vrm-utils-rs)** — key find: **pixiv-maintained serde structs for VRM 0.0, VRMC_vrm 1.0, MToon 1.0, springBone 1.0, vrm_animation 1.0**, serialize AND deserialize; v0.1.1 June 2026, Apache-2.0, generated from official JSON Schemas. VRM extension JSON does not need hand-rolling.
- mesh-tools (higher-level GLB builder, skin/morph unverified); easy-gltf/goth-gltf read-only; asset-importer-rs-gltf (young, claims full export); awsm-renderer-glb-export (GLB writer with skins + morph deltas — good reference code); gltf_kun (unavi, petgraph document graph, GLB export, extension hooks; backs bevy_vrm).
- bevy_vrm = import only; bevy_vrm1 = runtime only. **No existing Rust VRM writer anywhere.**
- Verdict: glTF 2.0 with skins + morphs + embedded textures from Rust is entirely realistic via gltf-json (or ~1–2k lines own serde structs); GLB container trivial; real work = accessor/buffer bookkeeping (mechanical) + VRM semantic layer (vrm-spec removes the schema half).

## 5. glTF technical details

- **GLB**: 12-byte header (magic "glTF", version 2, total length), chunks {length, type, payload}: JSON chunk space-padded to 4B, BIN chunk zero-padded. Little-endian. `.vrm` = this with different extension.
- **Skins**: node.skin → {joints: [node indices], inverseBindMatrices: MAT4/f32 accessor}. Primitives need JOINTS_0 (VEC4 u8/u16) + WEIGHTS_0 (sum 1). IBMs = inverse of joint world transform at bind (T-pose) — normalized rig ⇒ pure inverse translations.
- **Morph targets**: primitives[].targets[] = [{POSITION, NORMAL?, TANGENT?}] **displacement deltas**; same target count across primitives of a mesh; POSITION deltas need min/max. **Sparse accessors legal, ideal for face morphs, but importer support uneven — emit dense first**, sparse later after validation.
- **Textures**: PNG/JPEG only in core. **KHR_texture_basisu (KTX2) NOT supported by the VRM consumer ecosystem** — ship PNG (JPEG for large albedo).
- KHR extensions in VRM world: KHR_materials_unlit, KHR_texture_transform, KHR_materials_emissive_strength. Assume nothing else — VRM apps are the lowest common denominator.

## 6. Interop targets

| Consumer | VRM support |
|---|---|
| VSeeFace | 0.x only (frozen) |
| VNyan / Warudo | 0.x + 1.0 (1.0 quirks in Warudo mocap) |
| Cluster | 0.x + 1.0 |
| VRChat | **NO VRM** — Unity SDK3 upload only; community converters bridge |
| Resonite | No .vrm; rename to .glb imports geometry/rig, VRM extensions ignored |
| three.js | three-vrm 0.x + 1.0 |
| Unity | UniVRM both, import+export |
| Godot | V-Sekai imports both, exports 1.0, full MToon |
| Blender | saturday06 VRM Add-on imports/exports both |

What conformance buys: VRM 1.0 → Cluster + web + Godot + Unity + Blender + mostly Warudo/VNyan; 0.x variant additionally → VSeeFace. VRChat/Resonite unreachable via VRM regardless; our plain-glTF creature path IS the Resonite path.

## Synthesis: recommended export pipeline

- **Target VRM 1.0 canonical humanoid bake** + optional **0.x compat bake** behind a toggle (VSeeFace). Dual-version output is cheap from a parametric source: same skeleton/mesh bake, different extension JSON + 180° yaw + 0.x flat bone list. Don't make one file serve both.
- **Build the writer**: gltf-json schema model + pixiv vrm-spec extension structs + ~50-line GLB writer; crib from awsm-renderer-glb-export/gltf_kun. CI gate: mrxz/vrm-validator on humanoid bakes + Khronos validator on creature bakes; manual round-trips in three-vrm and UniVRM.
- **Bone map**: emit exactly the 15 required bones from parametric joints; add optional bones (neck, chest→upperChest, shoulders, eyes, toes, fingers) only when the record actually articulates them; never violate the fixed parent table. Bake **normalized T-pose rest** (identity rotations, translation-only nodes, scale 1): satisfies 1.0, required for 0.x, makes IBMs trivial, sidesteps retargeting quirks.
- Expressions: wire morphs/bones to the 18 presets where semantics match (blink/visemes minimum — what mocap apps drive); dense morph accessors + mesh.extras.targetNames. Materials: plain pbrMetallicRoughness + embedded PNGs; MToon only if toon look wanted. SpringBone/constraints later for hair/tails.
- **Creatures**: same writer, plain GLB, no VRMC_* — directly consumable by Blender/Unity/Godot/three.js and the correct Resonite format.
