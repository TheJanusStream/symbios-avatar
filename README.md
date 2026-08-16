# symbios-avatar

Parametric humanoid and creature bodies, generated entirely from code — no base
mesh and no third-party model licences. The engine-agnostic half of the pair;
[`bevy_symbios_avatar`] binds it to Bevy.

One asset is shipped, and it is motion rather than geometry: `assets/clips.bin`
holds twelve animations retargeted from a CC0 reference library. It is a
development reference rather than a runtime motion source — runtime motion is
procedural — and it is behind an off-by-default feature. Every body is
generated. See [`docs/clips.md`](docs/clips.md) for which clip came from where.

```text
Record   ──►  Skeleton  ──►  control cage  ──►  Catmull-Clark  ──►  render mesh
(~2 KiB)      (capsules)     (quad-dominant)    (smooth, all-quad)
```

```rust
use symbios_avatar::{Avatar, AvatarRecord};

// An avatar is a small parametric record, not a mesh.
let mut record = AvatarRecord::default();
record.reroll(42);

// One call from record to something drawable: merged skinned meshes grouped by
// material, a painted skin atlas, the rig they are bound to, and the bill.
let avatar = Avatar::build(&record).expect("a default body builds");
println!("{} tris across {} meshes", avatar.budget.tris, avatar.budget.meshes);

// Merged skinned meshes plus the eye globes, ready to hand to a renderer.
for drawn in avatar.drawn(0.0) {
    assert_eq!(drawn.mesh.skin.len(), drawn.mesh.vertex_count());
}
```

The stages are all public — `build_cage`, `catmull_clark`, `skin::bind`,
`unwrap`, `paint_skin` — but `Avatar::build` is the one recipe, and it is what
the examples consume.

## What it does

- **A body is a record, not a mesh.** Nine plan axes for a biped (eight for a
  quadruped) plus four composites — femininity, mass, body fat, age — with eyes,
  face, skin, hair and outfit in blocks of their own, about two kilobytes of
  JSON all told. Geometry is derived on demand, so the avatar belongs to the
  identity rather than to whichever app rendered it.
- **Every point of the space is a body.** Procedural character systems classically
  fail when sliders reach shapes that cannot be built. The constraints live in the
  body plans, and a sweep over 3,000 random bodies — 1,500 per plan — plus every
  axis extreme holds them honest.
- **Humanoids and creatures on one engine.** Only the graph differs; a pelvis
  carrying two legs and a quadruped girdle carrying four are the same code path.
- **Deterministic.** The same record always yields the same vertex layout, so
  geometry caches stay valid and a look reproduces exactly.
- **Honest failures.** When a joint is too crowded to mesh, the error names the
  limbs and the distance shortfall, because the fix is nearly always to widen a
  joint or lengthen a bone rather than to retune the mesher.

## Records

Avatars live in the owner's AT Protocol repository under
`network.symbios.avatar.*`, with the published schemas in [`lexicons/`](lexicons).
Each avatar is its own record — a wardrobe, not a single pinned identity — and
`network.symbios.avatar.profile` names the default.

Three things shape the record and are worth knowing before adding fields:

- **There is no float type.** The data model omits floating point so records have
  one canonical encoding, so axes are stored as thousandths and lengths as
  millimetres. Sanitising quantises to match, which is what makes a record equal
  itself after a round trip.
- **Records only ever grow.** `#[serde(default)]` goes on the *container*, not
  on the field: a field-level default yields the field type's zero rather than
  the struct's default, so one specified axis dragged every sibling to zero.
  Unknown fields are kept rather than ignored, so an older client rewriting a
  newer client's record cannot delete what it did not understand, and an
  unrecognised `$type` or token degrades to a stand-in instead of failing the
  whole avatar.
- **A seed reproduces a look.** Each axis draws from its own stream keyed on its
  name, so adding an axis cannot shift the others, and `generator` records which
  generation of the draw produced the parameters.

Re-rolling draws each category — stature, build, frame, proportions, head,
colouring, hair, age — from its own seed stream, so locking one category never
reshuffles another. A look also renders as a short share code:

```text
0W0Y2-1BBFX-V812W-CH1YC-A5G7P-7BPH6-R0K0
```

Codes are deliberately lossy — quantised to a byte per axis (a length gets
two), checksummed, and written in Crockford base32, whose alphabet drops
`I`, `L`, `O` and `U` so a code survives being read aloud or copied by hand.
The record remains the source of truth.

## How joints work

Joints are the hard part of skeleton-to-mesh conversion, and the construction
here follows B-Mesh (Ji, Liu & Wang, 2010): each limb gets a *socket ring* near
the joint, the sockets are hulled, and the facets that *are* sockets are deleted
to leave openings the limb tubes plug into. The rings are shared between hull
and tube, so the surface is watertight by construction rather than by stitching.

Two degeneracies have to be handled rather than wished away:

- **Flat socket fans.** A pelvis whose spine and legs all lie in the sagittal
  plane has no hull at all. The joint's own ball supplies the missing thickness
  as two apex points, which also read correctly as the joint's depth.
- **Buried sockets.** A socket is only a hull facet if its *plane* supports every
  other point in the joint. Angular reasoning fails here — a socket can subtend a
  comfortable 34° inside an 86° gap and still be buried, because a fat sibling
  ring's corner pokes past its plane sideways. Sockets are therefore slid along
  their own limb until the exact plane condition holds, which also thins them
  where the limb tapers.

## Debug tooling

```text
cargo run --example dump               # every demo body, cage + smoothed mesh
cargo run --example dump -- humanoid   # just one
cargo run --example dump -- --rolls 8  # eight rerolled avatar records
```

Writes OBJ files to `target/dump` (override with `SYMBIOS_AVATAR_DUMP_DIR`) and
prints a topology audit per body:

```text
humanoid     cage       264 verts    279 faces   78.9% quads  1.44x1.63x0.26 m  closed
humanoid     smooth    1084 verts   1082 faces  100.0% quads  1.44x1.63x0.24 m  closed
```

Open the `.obj` in any DCC tool to read the edge flow. `PolyMesh::manifold_report`
is the same audit as a value, and every test asserts against it: a hole means a
joint failed to weld to a limb, a winding conflict means a face was emitted
backwards, and both are silent in a renderer but fatal to normals, skinning, and
glTF export.

## Rigging and dressing

A meshed body is also a posable one. `Rig` roots the capsule graph into a
hierarchy ordered parent-before-child — the order glTF wants joints
written in — and `rig::skin::bind` attaches a mesh to it. Weights are derived
analytically rather than solved for, because the same code generated both the
skeleton and the surface, then smoothed across the surface so a torso does not
crease where two bones' influence meets.

Every node carries a **zone** saying what part of the body it is, which is what
lets the rest of the system address a body without knowing which plan built it:

```rust
use symbios_avatar::{AvatarRecord, Landmark, Limb, Rig, Zone};

let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton()).expect("a default body rigs");

// Semantic queries instead of bone names — the same call works on a quadruped.
// A hind extremity is a graph of its own — heel, the stub that closes it, ball
// and toe — so this finds every joint of both feet, four apiece.
let feet = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
assert_eq!(feet.len(), 8);

// Named anchors for fitting hats, garments, and other attachments.
let marks = rig.landmarks();
let hat = marks.get(Landmark::Crown).expect("every body has a crown");
let shoulders = marks.span(
    Landmark::LimbRoot(Limb::ForeLeft),
    Landmark::LimbRoot(Limb::ForeRight),
);
```

Garments declare the zones they cover as a `ZoneSet`, and the body suppresses
those zones underneath — poke-through is avoided by not emitting the geometry
rather than by hiding it.

## Texture atlas

A body unwraps into charts that follow its own zones, so a chart is *named*: a
procedural painter addresses "the chest" rather than "island 7". Two conventions
match what character artists do by hand — the seam runs up the back, so a face is
one unbroken island paintable as plain 2-D maths; and chart area is weighted by
importance, so the face and hands get more texels than an equal area of forearm.

```text
record_biped     unwrap      38 charts    1950 split verts  64% atlas used
```

Unwrapping duplicates vertices at chart boundaries and seams, so `UvUnwrap`
carries its own vertex list plus a `source` index back into the mesh — `gather`
pulls positions, normals, and skin weights through it. `to_obj` writes the
unwrapped body with texture coordinates, which is the only real way to judge
whether a chart reads as the body part it covers.

One thing to know before writing a painter: the zones charted are the same for
every body of a plan, but the *number* of charts is not. A zone can be genuinely
disconnected — each clavicle is cut off from the torso by the shoulder's own zone
— so key on `Chart::zone` and expect more than one chart per zone.

## Painting

Painting a body needs the inverse of rendering: for each texel, *where on the
body is this?* `bake_geometry` answers that once by rasterising the charts into a
buffer of position, normal, and zone per texel. `paint_skin` is then a pure
function of one texel's sample, which is what makes a procedural complexion
tractable — a freckle becomes arithmetic on a position rather than a search
through geometry.

```rust
use symbios_avatar::{
    AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Rig, SkinConfig, UvConfig,
    build_cage, catmull_clark, rig::skin, texture, unwrap,
};

// The stages `Avatar::build` runs, spelled out.
let record = AvatarRecord::default();
let skeleton = record.skeleton();
let cage = build_cage(&skeleton, &CageConfig::default()).expect("a default body meshes");
let mesh = catmull_clark(&cage, BODY_SUBDIVISIONS);
let rig = Rig::from_skeleton(&skeleton).expect("a meshable skeleton rigs");
let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());

let geometry = texture::bake_geometry(&mesh, &uv, 1024);
let condition = texture::Condition::of(&record.composites);
let map = texture::paint_skin(&geometry, &rig, &record.skin, &condition, None);
```

The output is a `symbios_texture::generator::TextureMap` — the same container
every other symbios generator produces — so an avatar's skin travels through the
ecosystem's existing image-conversion and upload path unchanged.

Two rules the layers follow, both learned the hard way:

- **Sample in body space, never atlas space.** Atlas space is discontinuous
  across chart boundaries, so a freckle field sampled there breaks at every seam.
  Body space is continuous by construction.
- **Shade from smooth geometry, not from zones.** A per-zone weight steps
  abruptly where two zones meet, drawing a visible line across a jaw or a wrist.
  Subdermal colour therefore comes from how thin the flesh is (the local radius),
  and cavity shading from how much the surface folds back on itself — both
  smooth everywhere. Zones stay for masking discrete things, where a step is
  invisible.

`cargo run --example dump` writes the albedo, normal, and ORM atlases as PNGs
alongside the OBJs, which is the only real way to judge whether skin reads as
skin.

## Hair

Hair is five regions of the head — scalp, brows, moustache, chin, and the
flanks of the jaw — in two layers that agree about where hair may grow: a
painted layer in the skin atlas, and low-poly card geometry standing off it.
The follicle masks are measured from the built surface rather than the plan,
so a beard's boundary lands on the jaw that was actually meshed.

Each region wears a style from its own catalogue — crop, bob, long, tied-back
or curly on a scalp; goatee, full or braided on a chin — with a cut (length,
thickness, density, droop) and a root-to-tip sRGB pair faded along each card
as vertex colour, so grey and dyed hair cost no texture and no draw. The whole
catalogue at its greediest fits under a triangle ceiling the budget suite
re-measures rather than quotes; a record that asks for more hair than the
budget holds is thinned to fit, and everything under the ceiling builds bit
for bit as asked.

## Motion

Motion is described by **goals** rather than joint angles, because a joint angle
bakes in the skeleton it was authored on and a body's proportions come from its
record. A goal — this foot is on the ground here, this hand is on that handle —
survives being replayed on a body that did not exist when it was written.

```rust
use symbios_avatar::{AvatarRecord, Ground, Rig, Vec3, anim::{Pose, plant_feet}, FootingConfig};

let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton()).expect("a default body rigs");
let mut pose = Pose::rest(&rig);

// Stand the body on whatever is beneath it. The closure is the only thing that
// knows about the world, so the same code works against a physics engine, a
// heightmap, or a flat plane.
let footing = plant_feet(
    &rig,
    &mut pose,
    |foot| Some(Ground::level(Vec3::new(foot.x, foot.x * 0.2, foot.z))),
    &FootingConfig::default(),
);
assert!(footing.is_settled());
```

- `anim::ik::two_bone` solves a limb analytically; `anim::ik::fabrik` iterates a
  spine or a tail. Both preserve bone lengths and the twist a limb was holding.
- `plant_feet` probes beneath each contact, drops the pelvis by the largest
  downward correction so the most stretched leg can still reach, then solves each
  leg to its own ground. Skipping the pelvis drop is the classic mistake: legs
  hyperextend and the body hovers on tiptoe.
- `Inertializer` transitions between poses by decaying the *offset* between them
  rather than crossfading, so a limb that was moving keeps moving. Only the
  incoming pose is ever evaluated, and it composes with anything — a clip, a
  solver, a gait.
- `anim::gait` walks a body with two numbers: a phase offset per contact and a
  duty factor. A biped's walk, a horse's trot, and a wave gait rippling down a
  centipede all fall out of those, which is what no library of authored walk
  cycles can do. Stride length scales with the legs' own reach, and the body
  sinks exactly as far as it must for its feet to stay within it — a leg standing
  straight has no slack to step with.
- `anim::clip` describes authored motion the same way: a **semantic query**
  naming which parts it moves — every ground contact, every grasper — and goals
  measured in fractions of the limb's own reach. "Raise both graspers" waves a
  biped's arms and does nothing to a quadruped, which has none free, and neither
  case needs a special path.
- `look_at` turns a body toward something, sharing the rotation down the chain
  from the torso outward. A body that swivels only its skull reads as a doll.

Which limbs carry a body is read off the rig rather than declared: a biped stands
on two of its four, a quadruped on all four, and a six-legged plan would work
without changing any of this.

```text
cargo run --example dump -- --walk 12   # a walk cycle over a slope, frame by frame
```

## Cargo features

Both are off by default:

- **`builtin-clips`** embeds `assets/clips.bin` (~200 KiB) and turns on
  `ClipLibrary::builtin`. Meant for development: the baked set is a comparison
  reference rather than a runtime motion source, and a consumer that only
  builds bodies should not carry it — least of all a wasm one. A consumer that
  wants the clips at run time can fetch the file and read it with
  `ClipLibrary::read` without this feature.
- **`serde-avatar`** makes a built `Avatar` serialisable, so one can cross a
  process or worker boundary. A round-tripped avatar is drawable, not
  rebuildable: it keeps its measured surface, eyes and handedness, and drops
  the intermediates the build was made from.

## Status

Early, but a body can now be described, built, given a face, eyes and hair,
dressed in its own skin and clothes, rigged, and walked. Records, body plans,
the mesher, rigging, skinning, zones, landmarks, UV charting, procedural skin,
hair, tight garments, the bone-driven face layer (expressions, blink, talk and
visemes), and motion (pose, IK, foot placement, inertialization, gait,
goal-space clips, baked-clip playback, gaze) are in place and tested.

Still ahead: a GLB writer (the glTF module only *reads*, for clip import),
loose garments and accessories, and creature variety —
see [`docs/plan.md`](docs/plan.md). [`docs/budget.md`](docs/budget.md) is the
triangle economy and how to measure a proposal against it;
[`docs/instruments.md`](docs/instruments.md) is the measurement fleet and the
rules for trusting an instrument.

## Licence

MIT, for the code.

`assets/clips.bin` is derived from [mesh2motion](https://github.com/scottpetrovic/mesh2motion-app)'s
animations, which are CC0 1.0 — a public-domain dedication that asks for nothing,
including attribution. [`docs/clips.md`](docs/clips.md) records what was taken
anyway. mesh2motion's retargeting *code* is MIT and none of it was copied; it was
read while ours was designed, and ours is a different formulation.

[`bevy_symbios_avatar`]: https://github.com/TheJanusStream/bevy_symbios_avatar
