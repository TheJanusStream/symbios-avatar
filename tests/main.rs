//! The single integration-test harness (#332).
//!
//! Every `tests/*.rs` beside this file is a `mod` of it rather than a
//! `[[test]]` target of its own. Nothing about the tests changed; what changed
//! is how many times the suite is compiled and linked. There were fifteen
//! targets, and each one was a separate codegen of everything it reaches in
//! the crate plus a link — at opt-level 3, since the test profile went to
//! release codegen, fifteen of those is real time on a 4-vCPU runner. One
//! target compiles once.
//!
//! ## What this changes about running them
//!
//! Per-file targeting moves from the target flag to the name filter, because
//! there is now one target:
//!
//! ```text
//! cargo test --test budget                  →  cargo test --test integration budget::
//! cargo nextest run -E 'binary(budget)'     →  cargo nextest run -E 'test(budget::)'
//! ```
//!
//! The module path is part of every test's name here (`budget::the_ceiling_…`),
//! so a filter on the module name with its `::` still selects exactly one
//! file's tests. `tests/golden/` is a directory of fixtures, never a target,
//! and the instrument that writes it (`examples/render --golden`) addresses it
//! through `CARGO_MANIFEST_DIR` — nothing here moved it.
//!
//! ## Why one process is safe here, and where the line is
//!
//! Sharing a target means sharing a process under `cargo test` (nextest still
//! forks per test). That is only safe for tests that do not share mutable
//! process-global state, and the #332 audit is what says these do: every file
//! here builds bodies from seeded records, reads a checked-in artifact, or
//! measures a mesh — nothing installs a hook, arms a `OnceLock` with a
//! test-specific value, or asserts on a process-wide counter. The crate has no
//! process-global to share; every draw comes from a `Pcg64Mcg` the test seeds.
//!
//! The line is worth stating for whatever is added next. A test that needs a
//! private copy of a process-global — one that installs a panic hook, sets an
//! environment variable it then reads back, or asserts on a global the crate
//! might one day grow — may NOT live here. It was isolated by having its own
//! binary, and that isolation is what this file spends. Such a test belongs in
//! the lib's unit tests where the fixture can be injected, or in a `[[test]]`
//! target of its own with the reason written down beside the declaration in
//! `Cargo.toml`. Overlands' #1194 is what that failure looks like when it is
//! not caught: one measurement reading 58-82 mm depending on which tests share
//! its process.

mod budget;
mod clips;
mod driver;
mod feet;
mod hair;
mod hands;
mod lexicon;
mod motion;
mod parts;
mod plan;
mod rig;
mod texture;
mod throat;
mod topology;
mod torso;
mod uv;
