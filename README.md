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

## Status

Early. Records, body plans, and the mesher are in place and tested. Skinning,
animation, hair and outfits, and glTF/VRM export are still ahead — see
[`docs/plan.md`](docs/plan.md).

## Licence

MIT.

[`bevy_symbios_avatar`]: https://github.com/TheJanusStream/bevy_symbios_avatar
