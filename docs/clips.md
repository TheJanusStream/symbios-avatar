# The baked clip set

`assets/clips.bin` is the only asset this repository ships. It holds twelve
retargeted animations, and every one of them is somebody else's work released
into the public domain. This file is the record of whose, from where, and under
what.

## What this is now

**A development reference, and not a runtime motion source.** It was the second
of those when it was imported. Epic #237 replaced runtime clip locomotion with
the procedural layer, and #248 re-authored the expressive roster as goal-space
clips that cost the bytes of their own description; what the artifact is for now
is comparison — a second opinion when a procedural motion looks wrong, and the
material `examples/locomotion` measures the gait against.

**It is a second opinion and never a gold standard**, and the reason is
measured rather than asserted. The owner's report at #237 was that the imported
clips do not loop cleanly and that on some of them the body teleports between
frames, as if a reference frame had changed under it. Both are real, both are
now printed by the instruments, and both are properties of the source rather
than of the transfer — the collapse rate below says the retarget invented no
motion of its own.

### The two defects, as numbers

Every figure is a ratio to that clip's **own** median frame-to-frame travel,
because an absolute distance means nothing across clips that move at very
different speeds. One is in family. Far from one, **either way**, is not.

```text
cargo run --release --example locomotion
```

| clip | frames | step | jump | jump × | at frame | seam × |
| --- | --- | --- | --- | --- | --- | --- |
| `Idle_A` | 94 | 2.6 mm | 3.0 mm | 1.1 | 8/94 | 1.2 |
| `Idle_Talking` | 110 | 8.9 | 22.8 | 2.6 | 48/110 | **0.3** |
| `Idle Listening` | 51 | 1.9 | 2.6 | 1.4 | 14/51 | **0.2** |
| `Walk` | 50 | 42.0 | 96.5 | 2.3 | 41/50 | 0.8 |
| `Jog` | 35 | 113.5 | 257.5 | 2.3 | 6/35 | **0.4** |
| `Sprint` | 25 | 163.5 | 238.2 | 1.5 | 5/25 | **0.4** |
| `Sitting_Idle` | 62 | 0.6 | 0.9 | 1.6 | 51/62 | **0.3** |
| `Greeting` | 145 | 19.9 | 50.5 | 2.5 | 19/145 | — |
| `Head Nod` | 26 | 4.3 | 6.3 | 1.5 | 21/26 | — |
| `Reject` | 114 | 11.9 | 71.8 | **6.0** | 79/114 | — |
| `Bow` | 113 | 2.0 | 50.1 | **24.5** | 71/113 | — |
| `Sleeping` | 50 | 0.2 | 0.3 | 1.5 | 13/50 | **0.1** |

**The loops do not jerk at the wrap. They pause at it.** That is the half of
the report the measurement corrected: not one of the eight looping clips steps
too far across its wrap, and six of them step far too little — `Sleeping` moves
a tenth of a normal frame's distance there and `Jog` four tenths. A wrap that
costs nothing is a body holding still for a frame every cycle, which is what a
last frame duplicating the first produces, and it is the shape of "does not
loop cleanly" that this set actually has. `Idle_A` and `Walk`, at 1.2 and 0.8,
are the two that close.

**The teleports are real, and they are two clips.** `Bow` moves a body
twenty-four times its own median step between frames 70 and 71, and `Reject` six
times between 78 and 79. Nothing else in the set is past 2.6, which is about
what a gait's fastest moment costs. Those two frames are the report, located.

Neither reading is available by watching: a pause of one frame at 30 fps and a
50 mm jump in a 2 mm clip both pass a render. They are `Continuity`, on
`PoseClip`, so the readings belong to the clip rather than to whichever
instrument remembered to take them — and `examples/bakeclips` prints them as it
bakes, so a re-bake cannot quietly lose the caveat.

### Where the artifact is used

The `builtin-clips` feature embeds it, and is **off by default and meant for
development**: the viewer's `--clip` picker uses it to put an imported motion
beside a procedural one, and `examples/locomotion` uses it to measure them
against each other. Nothing that ships to a person needs it. `ClipLibrary::read`
stays for a consumer that would rather fetch the file at run time, and
`src/retarget.rs` and the glTF reader stay because they are the bake path.

## Provenance

All twelve come from [mesh2motion](https://github.com/scottpetrovic/mesh2motion-app),
whose `LICENSE-CC0.MD` covers **all 3d models, blend files, rigs and
animations** in that project under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
CC0 is a dedication to the public domain: it asks for nothing, not even
attribution. This table exists anyway, because a dedication that asks for
nothing still deserves an honest record of what was taken.

The source files are **not vendored here**. They are eleven megabytes of GLB and
this crate ships generated geometry, so `examples/bakeclips` reads them from a
sibling checkout at `../mesh2motion-app/static/animations/` and refuses with
that path in the message when they are absent. Nothing in the crate needs them:
the artifact is already baked, and only re-baking wants the sources.

| clip | source file | licence |
| --- | --- | --- |
| `Idle_A` | `human-base-animations.glb` | CC0 1.0 |
| `Idle_Talking` | `human-base-animations.glb` | CC0 1.0 |
| `Idle Listening` | `human-addon-animations.glb` | CC0 1.0 |
| `Walk` | `human-base-animations.glb` | CC0 1.0 |
| `Jog` | `human-base-animations.glb` | CC0 1.0 |
| `Sprint` | `human-base-animations.glb` | CC0 1.0 |
| `Sitting_Idle` | `human-base-animations.glb` | CC0 1.0 |
| `Greeting` | `human-addon-animations.glb` | CC0 1.0 |
| `Head Nod` | `human-addon-animations.glb` | CC0 1.0 |
| `Reject` | `human-addon-animations.glb` | CC0 1.0 |
| `Bow` | `human-addon-animations.glb` | CC0 1.0 |
| `Sleeping` | `human-addon-animations.glb` | CC0 1.0 |

The names are the source's own, kept verbatim so that a row here can be checked
against the file it came from without a translation table in between.

What is *not* taken: mesh2motion's retargeting code, which is MIT and would
need attribution. It was read while designing ours (#139) and none of it was
copied; the transfer here is a different formulation, recorded at
`src/retarget.rs`.

## Why these twelve

The principle, stated so that the list can be argued with rather than merely
disagreed with: **a clip earns a slot if it is something an avatar in a social
space actually does.** Not if it is fun to watch once.

That resolves into three ways in:

- **It is addressed to somebody.** A social space is other people. A greeting, a
  yes, a no and a bow carry meaning to a viewer without a shared language or a
  chat box.
- **It is what a body does between things.** An avatar is idle for most of the
  time it exists, and the idle is the state a viewer sees longest.
- **It is how a body gets somewhere.** Locomotion is the only motion the system
  drives rather than the user, so it has to be there whatever anybody picks.

And four ways out:

- **It needs a prop or a world we do not have.** Every sword, pistol, bow, axe,
  ladder, shield and steering wheel in the library is a mime without the object.
  This is the largest single family in the 162 and all of it is out.
- **It is a one-time spectacle.** A backflip is delightful once and then it is
  the thing your avatar does.
- **It needs state we do not model.** There is no damage model, so a death and a
  flinch are poses without a cause.
- **It is a transition into or out of a state.** `Sitting_Enter` is worth
  carrying the moment something owns a state machine, and nothing does yet, so
  `Sitting_Idle` goes in alone and the entry pops. A pop into a real pose beats
  a transition into nothing.

### What was left out that a reasonable person would keep

- **A victory.** It was in the issue's own first cut and this list drops it for
  `Idle Listening`, on the grounds that a talking idle without a listening one
  gives a room where everybody talks at once. Victory is also a spectacle by the
  rule above, and it needs a thing to have won.
- **`Idle_FoldArms`** — a genuine second standing posture, and the strongest of
  the near-misses at 8.9 KiB. Out only because one idle plus two conversational
  ones is already three of twelve.
- **`Dance_Simple`, `Jumping Jacks`, `Backflip`** — spectacle.
- **`Angry`, `Confused`, `Dizzy`, `Shivering`** — emotes, and a good future set,
  but they read as reactions to something and nothing yet produces the something.
- **`Meditate`, `Kneeling Tired`** — both are rests, and `Sleeping` covers the
  away state more legibly than either.
- **The whole `_RM` family** — root-motion variants of clips already here. They
  belong with a locomotion system that consumes root motion, which is #141's
  question.

## What it costs

Measured by `cargo run --release --example bakeclips`:

| | |
| --- | --- |
| twelve clips | 204,462 bytes, 199.7 KiB |
| average per clip | 16.6 KiB |
| smallest | `Head Nod`, 920 bytes |
| largest | `Greeting`, 54,100 bytes |
| hands and feet | 97,840 bytes — **48% of the artifact** |
| as JSON instead | 636.8 KiB, 3.2× |
| gzipped | 117.9 KiB, 59% |

**Fingers are nearly half of it, and they stay.** Including them was the owner's
decision (2026-08-07) and the price is stated here rather than quietly taken out
of it: a hand is twenty-one of our rig's seventy-seven joints and forty of the
reference's sixty-six, so a clip that moves the fingers spends most of its bytes
on them. `Greeting` is a wave with an open hand and is fifty-nine times the size
of `Head Nod`, which moves two joints.

**This is not measured against the record budget and should not be.** The 100
KiB in `record::mod` is a ceiling on one avatar's atproto record, which travels
per person and over the network. The artifact is a build-time asset shared by
every body the crate ever makes, and it is carried once. The comparison worth
making is against the sources it replaces: 10.9 MB of GLB, of which this is
1.8%.

## Re-baking

```text
cargo run --release --example bakeclips -- --dry   # report only
cargo run --release --example bakeclips            # and write assets/clips.bin
```

The report's important column is not the size. It is **the collapse rate**: how
many of our tracks move against how many joints the reference itself moves
through the same clip, under the same tolerance. Those two are related by an
identity, not by a tolerance —

> our moving tracks + the source's silent movers = the source's moving joints

— where a *silent mover* is a joint the retargeter holds still whatever the
source does: a leaf, whose rotation deforms no surface, and the reference's own
`root`, which arrives as translation rather than as a joint. It holds exactly on
all twelve. Anything else is a finding: above means the transfer introduced
motion of its own, which is what a first draft of the retargeter did at #139
while every visible check passed at 0.028 degrees; below means it dropped some.
Neither shows in a render.

To look at a clip rather than measure it:

```text
cargo run --release --example render -- --clip Greeting --clipframes 8
```
