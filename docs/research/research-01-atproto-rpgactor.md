# Dossier: rpg.actor & the state of avatars/games/3D on AT Protocol

*A point-in-time research dossier gathered while this crate was being designed. Kept as the record of what was found and why the decisions in [../plan.md](../plan.md) were made — external version numbers, prices and library gaps in it were true when written and are not maintained.*


## 1. rpg.actor — what it is

"RPG Actor Registry — a universal character compendium" at https://rpg.actor. Users log in with any atproto account and store character sheets, pixel-art sprites, and equipment **as records in their own repo**; the registry indexes and displays them; "any ATproto-enabled service can read or write the same data." (https://rpg.actor/dev-guide)

Built anonymously (only dev@rpg.actor, Canadian servers; Bluesky `@rpg.actor`, did:plc:kwgllf365cwmxbnxitx4pjdj, created 2026-02-28, launch ≈ March 2026). Godot integration by TechTastic (github.com/TechTastic/godot-rpg-actor). Freemium "Creator" tiers provision **real PDS accounts** with `name.rpg.actor` handles per character — each NPC is a full atproto identity — with claim-link handoff to players. Custom domains via wildcard CNAME + Caddy TLS.

### Lexicons (public at https://rpg.actor/lexicons/<nsid>.json)

| NSID | rkey | Purpose |
|---|---|---|
| actor.rpg.stats | per-system key (`dnd`, `rmmz`…) | Stats, one record per game system |
| actor.rpg.sprite | `self` singleton | Baked 144×192 PNG sprite sheet + anim metadata |
| actor.rpg.generator | `self` | Decomposed sprite **layers + parameters** to rebuild |
| actor.rpg.master | TID | GM attestation of player stats |
| equipment.rpg.give | any | Provider-side attestation of item grant (provider's PDS) |
| equipment.rpg.item | any | Player-held item w/ asset blobs (player's PDS, links give AT-URI) |

Two authority domains (actor.rpg.* + equipment.rpg.* = rpg.actor + rpg.equipment reversed).

**actor.rpg.sprite**: required spriteSheet (PNG blob, max 10MB) + createdAt; rows/columns (1–16), frames (1–64), frameWidth/Height (≤512), animationSpeed (50–2000ms), `source` (AT-URI of generator record that baked it — enables safe recomposition), `isCustom` (blocks recomposition). Baseline: 144×192, 3×4 grid of 48×48, rows=down/left/right/up, walk cycle [0,1,2,1].

**actor.rpg.generator — the parametric avatar record**: required bodyType, skin + eyes (color-config: tone 0–100, mode, hex), layers[] (≤32, back-to-front), createdAt; optional items[] (≤20 equipment IDs). Each layer: id (category) + blob (**recolored** PNG ≤1MB); colors (4 recolor channels), subtractMask (≤50KB, punches holes), behindRows, zSlot/zSlotByDir, layerClass, suppress/suppressAs/suppressSpecific (helmet hides hair), disabled. Canonical layer order: body, undies, feet, bottoms, tops, clothHands, necklaces, hair, bangs, beard, hind, facial, glasses, ears, headwear, lefthand, righthand, costume. **Colorway system**: sentinel-color RGB channels in source art (#0000FF main / #00FF00 sub1 / #FF0000 sub2; skin #F9C19D; hair #FCCB0A; black = subtract) recolored at bake time. Key design point: the record stores **already-recolored layer blobs** (baked intermediates), not just parameters — consumers composite without owning the asset library; equipment items ship their own layer art.

**actor.rpg.stats**: "typed core + `unknown` escape hatch" — first-party typed refs (dnd, pathfinder, cyberpunk2020, rmmz…) + third-party systems under any lowercase key with untyped `data`; registry renders untyped stats by value-shape conventions (number → stat box, {current,max} → bar, {_heading:true} → divider).

**equipment.rpg.item**: dual-record trade model — provider writes `give` on THEIR PDS, player writes `item` on THEIRS referencing it; colorway blob + channels[] (≤8: name, sentinel color, defaultColor), fitClass/suppressAs/fallback machinery.

## 2. Rendering/serving & consumers

- Generator UI composites client-side, **bakes** final sprite into the sprite record blob on user's PDS. Registry serves derived images server-side (GET /api/sprite/normalized?did=…, OG cards) + API-key-gated compose/recolor/dress/undress/roll endpoints. Plain consumers fetch the baked blob straight from the player's PDS (getRecord + getBlob). Public API rate limit: 150 req/min/IP.
- Consumers: RPG Maker MZ plugin suite (OAuth PKCE+DPoP with **granular scopes**: repo:actor.rpg.stats, repo:actor.rpg.sprite, blob:image/*), Godot addon, April 2026 game jam ("read or write at least one rpg.actor lexicon"), first-party minigames, partner systems (Reverie House, Playtopia).
- HN noted the `self`-rkey one-character-per-identity constraint as a limitation for transient-character games.

## 3. Other atproto game/3D prior art

- games.gamesgamesgamesgames lexicon (Trezy/Birbhouse, MIT, RFC governance) — game metadata, consumer: Pentaract. Trezy's "Lexicon Generics" blog post relevant to per-app profile/avatar records.
- Kevin Hoffman "Decentralized Gaming with ATproto" design essay.
- **No credible atproto-native 3D-world/metaverse/VRM-avatar project found** beyond Symbios Overlands. lexicon.community catalogue has nothing 3D. **A parametric 3D avatar lexicon would be first-of-kind; rpg.actor is the only shipped parametric-avatar-on-PDS system, and it's 2D sprites.**

## 4. atproto constraints for parametric 3D avatars

- **Record size**: hard protocol limit 1,000,000 bytes/record block; commit blocks ≤2MB; ≤200 ops/commit; firehose frames ≤5MB (atproto.com/specs/sync). Our 100 KiB budget is self-imposed, well under.
- **Blobs**: bsky.social PDS max upload 50MB. Lexicon maxSize/accept enforced at record-creation, not upload.
- **Blob serving/CORS**: spec discourages direct-from-PDS media serving (mandates CSP sandbox on getBlob) but in practice reference PDS is CORS-open and ecosystem apps fetch cross-origin directly. **cdn.bsky.app only serves transcoded images at feature-specific URLs** — arbitrary binary (glTF, mesh params) will NOT get Bluesky CDN treatment; serve from PDS getBlob or run own caching CDN (as rpg.actor does).
- **Blob GC**: blobs temporary until referenced by a record (grace ≥1h); deleting last referencing record deletes the blob.
- **Write rate limits (bsky.social)**: 5,000 pts/hr, 35,000/day per DID (create=3 → 1,666 creates/hr); applyWrites ops count individually; ~3,000 req/5min/IP.
- **Storage quotas**: currently NO per-account total-storage quota on reference PDS (open issue #3392).
- **Lexicon evolution**: additive-only — new fields optional; no type changes/renames; breaking ⇒ new NSID; unknown fields ignored; invalid = wholly invalid.
- **Lexicon resolution (live since 2025)**: publish schemas as com.atproto.lexicon.schema records keyed by NSID; `_lexicon.<reversed-authority>` DNS TXT → DID; com.atproto.lexicon.resolveLexicon XRPC + @atproto/lexicon-resolver npm.
- **Granular OAuth scopes shipping** and used by rpg.actor in production: repo:<nsid> + blob:image/* (matches our OAuth-narrowing notes) — plan repo: scopes on our NSIDs.

## 5. Community lexicon-publishing conventions

- Namespace = reversed domain you control; records = singular nouns.
- lexicon.community governs **shared** cross-app types under community.lexicon.* (Smoke Signal donated events/location types). Pattern: app-specific types stay in app's namespace; genuinely shared vocabulary migrates to community namespace.
- Publication: raw lexicon JSONs at predictable URLs AND/OR schema records + _lexicon DNS; dev-guide page documenting rkey conventions (rpg.actor's is a good template).
- **rpg.actor conventions worth copying**: singleton `self` rkey for "the one avatar"; `source` AT-URI on baked artifacts pointing at the parametric record; `isCustom` flag protecting hand-edited artifacts from recomposition; cross-PDS attestation pairs (give/item) for third-party-granted content; reserved-key registry + regex for third-party extension keys.

## Key unknowns
- rpg.actor authorship; no public source repo for the registry itself.
- actor.rpg.master / generator full JSON not captured verbatim.
- Jam entry count; ecosystem scale beyond RMMZ/Godot toolchains.
- Whether rpg.actor publishes schema records / _lexicon DNS (vs only HTTP JSONs).
- No normative CORS guarantee on getBlob for large non-image blobs — plan a caching layer.
