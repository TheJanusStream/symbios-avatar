# Dossier: Stylized Semi-Realistic Character Rendering in Bevy 0.18 (native + wasm)

*A point-in-time research dossier gathered while this crate was being designed. Kept as the record of what was found and why the decisions in [../plan.md](../plan.md) were made — external version numbers, prices and library gaps in it were true when written and are not maintained.*


## 1. What makes the look (Fortnite / Overwatch / Sea of Thieves / Arcane)

**Common core: it is PBR underneath, stylized in the *inputs*, not the shading model.** All three games run standard physically-based shading; the style comes from hand-authored/simplified textures, controlled roughness, and lighting discipline. None uses outlines.

**Overwatch** (frame analysis + 80.lv technical breakdown):
- Deferred-ish PBR: prepass captures view-space normals, albedo, metalness, roughness, emissive, depth; per-light diffuse passes + IBL specular from skybox (https://alain.xyz/blog/frame-analysis-overwatch).
- Texture packing: Color(RGB)+AO(A) in one 32-bit map; normal map plus specular(gray)+roughness pair. **Skin gets a dedicated "blood map" (24-bit RGB) that the skin shader uses to fake subsurface scattering** — faked SSS via a subdermal color mask, not screen-space blur.
- **Painted bevels**: large soft bevels painted into textures/normals make objects read softer and give "a dramatic rim light feeling for completely free" — rim is baked into the material response, not a fresnel term (https://80.lv/articles/overwatch-technical-overview).
- **Specular-only detail philosophy**: micro-detail lives only in the specular/roughness channels, so materials stay flat and readable at distance but sparkle up close.
- Geometry: polygon density weighted to face/hands/weapon; bone-based facial animation (no blendshapes); rectangular 1024×2048 textures to keep long UV islands straight; nearly every hero has dynamic hair/dangly bits for secondary motion.

**Sea of Thieves** (SIGGRAPH 2018 talk): stylized game fighting a photoreal engine (UE4). Character/asset lessons: hand-paint textures *from* photo reference (don't filter photos), simplified geometry composed of simple objects, "wonky but physically plausible" deformation, wear/patches/repairs that tell history, mood via lighting not texture.

**Fortnite**: "stylized PBR" — real roughness/metal/normal channels treated stylistically: exaggerated edge wear, simplified color palettes, brush-stroke detail in normal maps; hand-painting beat photo simplification; story told by silhouette and broad shape. Skin reads as smooth albedo + narrow roughness range + engine SSS.

**Arcane/Fortiche** (offline, style target): explicitly **not** PBR — painted light/shadow in textures with visible brushstrokes, dual complementary rim lights (blue/orange) as *light rig* not shader fresnel, color-scripted non-physical bounce lights. Real-time approximation = painterly albedo + rim light rig + hard color grading.

**Extracted ingredient list**: (1) PBR base, (2) painterly albedo with baked AO + painted bevels, (3) restrained roughness (broad smooth zones, detail only in spec), (4) subdermal-color faked SSS for skin, (5) rim via painted bevels and/or a fill/rim light rig — not a shader outline, (6) chunky sculpted hair with anisotropic-ish highlight, (7) big readable eyes with a deterministic highlight, (8) bloom + filmic tonemap + saturated controlled palette, (9) no outlines in any of the three games.

## 2. Skin shading, ranked by cost (cheap → expensive)

1. **Plain PBR + good albedo** (free): warm albedo, red-shifted AO, painted subdermal tones. Most of the Overwatch look already.
2. **Wrap / ramp lighting** (near-free, custom shader): shift NdotL falloff toward warm red at the terminator; classic stylized-skin hack (https://therealmjp.github.io/posts/sss-intro/).
3. **Pre-integrated skin (Penner, SIGGRAPH 2011)** (cheap, single-pass): LUT indexed by NdotL × curvature, no blur passes. Deferred variant ran 1650×1080 in 2.2 ms on a GTX 570. **Best fit for our case**: curvature can be baked at procgen time since we generate the mesh.
4. **Screen-space SSS** (expensive): screen-space blur of diffuse irradiance; high fixed cost, memory-heavy, halo artifacts, wants deferred G-buffer. Worst fit for WebGL2.

**Bevy 0.18 native support**:
- `StandardMaterial` has **no SSS / pre-integrated skin**. It *does* have `diffuse_transmission` (cheap reversed Lambertian lobe — right for **backlit ears/nostrils**, pair with `TransmittedShadowReceiver`) and `specular_transmission` (screen-space refraction, expensive — skip for characters). Both work on WebGL2; `pbr_transmission_textures` feature-gated off by default due to texture-binding pressure.
- Wrap/ramp or Penner LUT = custom shader: cleanest as `ExtendedMaterial<StandardMaterial, SkinExtension>` overriding the diffuse lobe.
- Bevy 0.18 fixed over-bright point-light specular and env-map fresnel — materials render noticeably better than 0.17.
- **WebGL2 constraints**: no compute shaders, no storage buffers, tight texture-binding limits, SSAO plugin not loaded, TAA disabled. Ranks 1–3 work fine on WebGL2; rank 4 effectively doesn't.

## 3. Hair

**What stylized games ship**: sculpted **mesh hair** (solid chunky clumps, alpha-free) — Overwatch, Fortnite, SoT all do this, with painted flow + gloss ramp; alpha cards only for wisps/edges. Mesh hair sidesteps the whole transparency problem — recommended primary approach.

**Bevy capabilities**:
- **Anisotropy: yes.** `StandardMaterial::anisotropy_strength` / `anisotropy_rotation` + optional `anisotropy_texture` (KHR_materials_anisotropy), since 0.14. Requires correct tangents along hair flow — we generate the mesh, so author tangents down the strand direction. Watch texture-binding count on WebGL2.
- **Alpha-to-coverage: yes** — `AlphaMode::AlphaToCoverage` since 0.14; needs MSAA; order-independent, cheap, works on WebGL2 — the pragmatic wisp-card path.
- **OIT: exists but avoid** — 0.15, opt-in, "uses a lot of GPU memory", WIP, **does not work with custom materials** (issue #20297), rules out WebGL2.
- **Procedural generation reference**: VRoid's hair = procedural tube/guide system (hair count per guide, cross-section profile, twist, thickness). For our crates: sweep tapered ribbon/tube cross-sections along guide splines (our prim/sweep mesher is already most of this), UV = (around, along-strand), strand texture = procedural streak noise gradient-mapped by palette, tips optionally A2C-faded.

## 4. Eyes

**Standard AAA construction**: two surfaces — opaque sclera/iris ball + transparent glossy **cornea shell** (IOR ≈ 1.38); iris as a plane ~2.2 mm behind cornea apex; **iris parallax** = view-dependent UV shift from view dir in tangent space × iris depth (cheap 1-sample POM); refraction via `refract(view, normal, 1/IOR)`.
- **Stylized games use a deterministic, non-physical highlight**: position controlled relative to the iris, decoupled from scene lighting — texture sprite on a top layer or shader-placed disc. The single biggest "alive vs dead eyes" lever.
- **Minimal viable stylized eye**: one sphere, procedural iris albedo, roughness ≈ 0.05 (wet), painted or shader-fixed highlight, dark limbal ring, slight top-lid AO band painted onto the ball. Parallax and cornea shell are tier-2.
- **Procedural iris**: polar-coordinate radial fiber noise (stretched voronoi/streaks in (r,θ)) + radial gradient (dark limbal ring → colored mid → brighter around pupil) + gradient-map by palette; pupil = smoothstepped disc.

## 5. Bevy 0.18 character-relevant feature inventory

| Feature | Status | wasm notes |
|---|---|---|
| Skinning + morph targets on same mesh | Yes since 0.11 (PR #8158); motion vectors for skinned+morphed (PR #13075) | Works on WebGL2 (uniform-buffer joint path) |
| ExtendedMaterial | Stable pattern | Works; already used for wind sway |
| Anisotropy, clearcoat | 0.14+ | Binding-pressure caveats on WebGL2 |
| Diffuse/specular transmission | 0.12+ | Diffuse: all platforms; specular: expensive |
| TAA | Still experimental as of 0.17/0.18 | **Disabled on WebGL2** |
| SSAO | Native yes | **Not loaded on WebGL2** (issue #8888) |
| Bloom, tonemapping (TonyMcMapface), color grading | Solid | Works on WebGL2 |
| Shadows | Cascades, PCF; soft-shadow options | Works; quality via cascade config + map size |
| EnvironmentMapLight | Yes (0.12+) | Works on WebGL2 — **the consistent-character-grading tool** |
| Reflection/light probes | 0.13+, "WebGL2 support effectively non-existent" | Native only |
| OIT | 0.15, WIP, memory-heavy, no custom materials | Not WebGL2 |
| Skinned-mesh batching | PR #16599: joint matrices in storage buffers → batching "on platforms other than WebGL2" | **WebGL2: one draw per skinned mesh** |
| 0.18 PBR fixes | Specular over-bright + env-map fresnel fixes — free quality | — |
| Community ceiling | bevy_toon_shader etc. exist; no published AAA-quality character showcase found — ceiling unproven, gap is ours to close, not an engine hard limit | — |

## 6. Procedural character textures (character-specific)

- **Skin albedo layer stack** (refs: MB-Lab skin editor, Universal Human): base tone driven by a **melanin parameter** → subdermal red **blood/vascular zone map** (cheeks, nose, ears, lips, knuckles — Overwatch blood-map precedent) → thresholded-noise **freckle mask** gated by region mask → **stubble mask** → painted AO/bevel layer (brow, nose sides, under-lip) → optional makeup/warpaint layers. All mask×noise×gradient-map compositions our texture crate already speaks.
- **Face UV conventions**: unwrap one symmetric master half, **mirror geometry not UVs**; seam up the back of the head; keep face one continuous front-projection-friendly island (procedural painting becomes 2D face-space math); highest texel density on face and hands; Overwatch used 1024×2048 rectangles to keep long islands axis-straight. For procgen: a *fixed canonical face UV layout* across all generated heads means one parametric painter serves every character.
- **Stylized fabric/leather** (character deltas vs environment textures): exaggerated **edge wear on seams/borders**, painted bevel/normal at panel boundaries, stitching rows as normal+albedo features, micro-detail **only in roughness/spec channel**, broad flat albedo zones per garment panel.
- **Gradient mapping for palette customization**: one grayscale value/detail map per region + N-region ID mask; index into per-region color ramps. Since our textures are CPU/worker-generated anyway, **bake the ramp at generation time** — zero shader cost, batching-friendly, re-rolls are just re-bakes.

## 7. Performance budgets

- **VRChat PC ranks** (best public reference): **Excellent ≤ 32k triangles, Good ≤ 70k**; Quest/mobile <20k. Texture memory: Excellent ≈ ≤30 MB VRAM.
- **General modern budgets**: 15k–50k polys typical for console/PC main characters. Fortnite hero outfits community-estimated 30–80k.
- **Recommended target: 15–30k tris, distributed Overwatch-style (face/hands heavy)**, textures 1024² face + 1024² body + 512² hair/eyes ≈ well under 30 MB.
- **Draw calls / many avatars in Bevy**: batching requires same mesh+material handle; skinned meshes batch only where storage buffers exist — **on WebGL2 every skinned mesh is its own draw**, so the lever is *fewer meshes/materials per avatar*: merge each avatar to 1–3 skinned meshes (body / hair / cutout bits), one atlas texture set per avatar, bake palette colors into textures instead of per-instance material params. 10 avatars × 3 draws is nothing; 10 avatars × 40 part-meshes is a WebGL2 problem.

## Synthesis: recommended Bevy stack + custom-shader shopping list (max perceived quality per effort)

**Base stack (no custom shaders, works WebGL2 today):** `StandardMaterial` + procedural textures; `EnvironmentMapLight` on the camera with a *fixed authored cubemap* for consistent character grading across biomes; 3-point-ish light rig sensibility (key sun + cool fill + warm/cool rim light attached to camera or character); bloom + TonyMcMapface tonemap; MSAA 4x; shadow cascade tuned tight around characters.

Ranked shopping list:
1. **Texture generators, not shaders** (highest ROI, zero platform risk): skin layer stack, painted-bevel treatment on clothing seams, specular-only micro-detail, gradient-map palette baking on a canonical face/body UV layout. ~70% of the Overwatch/Fortnite look; lives entirely in the existing texture crate.
2. **Eyes v1** (small, disproportionate payoff): sphere + procedural polar iris + limbal ring + fixed painted highlight, roughness ~0.05. Eyes v2 later: cornea shell + 1-sample iris parallax in ExtendedMaterial.
3. **Mesh hair via sweep/tube mesher** (VRoid-style guides → tapered ribbons), opaque, anisotropy + strand-direction tangents, gradient-mapped strand texture; AlphaToCoverage for wisp cards only. Avoid OIT entirely.
4. **Skin ExtendedMaterial**: wrap/ramp warm terminator first (an afternoon), upgrade to Penner pre-integrated LUT using curvature baked at mesh-generation time (we own the mesher — unusually cheap for us). Add diffuse_transmission + TransmittedShadowReceiver on ears/nostril region.
5. **Deterministic eye highlight + rim light rig** (Arcane flavor): camera-space rim light entity following each avatar; shader-fixed eye glint. Cheap, big "alive" factor.
6. **Native-only garnish, feature-gated**: SSAO on characters, TAA/DLSS, soft shadows — never load-bearing; WebGL2 stays first-class.
7. **Skip/defer**: screen-space SSS, OIT, strand hair, screen-space specular transmission, outlines (the reference games don't use them).

Key platform trap: anything storage-buffer/compute-based (SSAO, TAA, OIT, skinned batching) is native/WebGPU-only; design each quality tier so WebGL2 degrades by *omission*, not by different art.
