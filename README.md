# symbios-avatar

Parametric humanoid and creature bodies, generated entirely from code — no base
mesh, no shipped assets, no third-party model licences. The engine-agnostic half
of the pair; [`bevy_symbios_avatar`] binds it to Bevy.

```text
Record  ──►  Skeleton  ──►  control cage  ──►  Catmull-Clark  ──►  render mesh
(9 axes)     (capsules)     (quad-dominant)    (smooth, all-quad)
```

```rust
use symbios_avatar::{AvatarRecord, CageConfig, build_cage, catmull_clark};

// An avatar is a small parametric record, not a mesh.
let mut record = AvatarRecord::default();
record.reroll(42);

let cage = build_cage(&record.skeleton(), &CageConfig::default())?;
let body = catmull_clark(&cage, 2);

assert!(body.is_closed_manifold());
assert_eq!(body.quad_fraction(), 1.0);
# Ok::<(), symbios_avatar::CageError>(())
```

## What it does

- **A body is a record, not a mesh.** Nine semantic axes for a biped, eight for a
  quadruped, in a few hundred bytes. Geometry is derived on demand, so the avatar
  belongs to the identity rather than to whichever app rendered it.
- **Every point of the space is a body.** Procedural character systems classically
  fail when sliders reach shapes that cannot be built. The constraints live in the
  body plans, and a sweep over 3,000 random bodies plus every axis extreme holds
  them honest.
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

Two consequences of the protocol shape the record and are worth knowing before
adding fields:

- **There is no float type.** The data model omits floating point so records have
  one canonical encoding, so axes are stored as thousandths and lengths as
  millimetres. Sanitising quantises to match, which is what makes a record equal
  itself after a round trip.
- **Records only ever grow.** New fields are optional with `#[serde(default)]`
  *on the field* — a container-level default silently resets sibling fields when
  one is missing — and unknown fields are ignored on read.

Re-rolling draws each category (stature, build, frame, proportions, features)
from its own seed stream, so locking one category never reshuffles another.
A look also renders as a short share code:

```text
040S4-27HTV-7YXXG-AT5TY-0
```

Codes are deliberately lossy — quantised to a byte per axis, checksummed, and
written in Crockford base32 so `I`/`L`/`O` survive being copied by hand. The
record remains the source of truth.

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
cargo run --example dump               # every demo body, cage + 2 subdivisions
cargo run --example dump -- humanoid   # just one
cargo run --example dump -- --rolls 8  # eight rerolled avatar records
```

Writes OBJ files to `target/dump` (override with `SYMBIOS_AVATAR_DUMP_DIR`) and
prints a topology audit per body:

```text
humanoid     cage       132 verts    145 faces   79.3% quads  1.45x1.65x0.26 m  closed
humanoid     smooth    2202 verts   2200 faces  100.0% quads  1.44x1.63x0.22 m  closed
```

Open the `.obj` in any DCC tool to read the edge flow. `PolyMesh::manifold_report`
is the same audit as a value, and every test asserts against it: a hole means a
joint failed to weld to a limb, a winding conflict means a face was emitted
backwards, and both are silent in a renderer but fatal to normals, skinning, and
glTF export.

## Rigging and dressing

A meshed body is also a posable one. [`Rig`] roots the capsule graph into a
hierarchy ordered parent-before-child — the order glTF and VRM want joints
written in — and `rig::skin::bind` attaches a mesh to it. Weights are derived
analytically rather than solved for, because the same code generated both the
skeleton and the surface, then smoothed across the surface so a torso does not
crease where two bones' influence meets.

Every node carries a **zone** saying what part of the body it is, which is what
lets the rest of the system address a body without knowing which plan built it:

```rust
use symbios_avatar::{AvatarRecord, Landmark, Limb, Rig, Zone};

let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;

// Semantic queries instead of bone names — the same call works on a quadruped.
let feet = rig.query(|zone| matches!(zone, Zone::Extremity(limb) if !limb.is_fore()));
assert_eq!(feet.len(), 2);

// Named anchors for fitting hair, hats, and garments.
let marks = rig.landmarks();
let hat = marks.get(Landmark::Crown).expect("every body has a crown");
let shoulders = marks.span(
    Landmark::LimbRoot(Limb::ForeLeft),
    Landmark::LimbRoot(Limb::ForeRight),
);
# Ok::<(), symbios_avatar::RigError>(())
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
record_biped     rig         23 joints     29 charts     568 split verts  61% atlas used
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
use symbios_avatar::{AvatarRecord, texture};
# use symbios_avatar::{CageConfig, Rig, SkinConfig, UvConfig, build_cage, catmull_clark, rig::skin, unwrap};

let record = AvatarRecord::default();
# let skeleton = record.skeleton();
# let mesh = catmull_clark(&build_cage(&skeleton, &CageConfig::default())?, 2);
# let rig = Rig::from_skeleton(&skeleton)?;
# let zones = skin::bind(&mesh, &rig, &SkinConfig::default()).zone_map(&mesh, &rig);
# let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());
let geometry = texture::bake_geometry(&mesh, &uv, 1024);
let map = texture::paint_skin(&geometry, &rig, &record.skin);
# Ok::<(), Box<dyn std::error::Error>>(())
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

## Motion

Motion is described by **goals** rather than joint angles, because a joint angle
bakes in the skeleton it was authored on and a body's proportions come from its
record. A goal — this foot is on the ground here, this hand is on that handle —
survives being replayed on a body that did not exist when it was written.

```rust
use symbios_avatar::{AvatarRecord, Ground, Rig, Vec3, anim::{Pose, plant_feet}, FootingConfig};

let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton())?;
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
# Ok::<(), symbios_avatar::RigError>(())
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

Which limbs carry a body is read off the rig rather than declared: a biped stands
on two of its four, a quadruped on all four, and a six-legged plan would work
without changing any of this.

## Status

Early. Records, body plans, the mesher, rigging, skinning, zones, landmarks, UV
charting, procedural skin, and the motion foundation (pose, IK, foot placement,
inertialization) are in place and tested. Gait, goal-space clips, eyes and hair,
outfits, and glTF/VRM export are still ahead — see [`docs/plan.md`](docs/plan.md).

## Licence

MIT.

[`bevy_symbios_avatar`]: https://github.com/TheJanusStream/bevy_symbios_avatar
