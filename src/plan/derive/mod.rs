//! Turning a record's axes into the numbers a cage is built from.
//!
//! A record carries semantic axes — `height`, `build`, `shoulder_width` — and a
//! cage wants radii, heights and spans. These modules are the whole of that
//! translation, one per body plan, and the plans themselves do nothing but hang
//! nodes on what comes out.
//!
//! ## Why the translation is its own layer
//!
//! **Every axis already drives several quantities, and until this split each
//! one said so at a dozen scattered sites.** The biped's `girth` alone is read
//! by twenty expressions across the torso, the limbs and the neck. That is
//! survivable while an axis is one-to-one with a body part, and it stops being
//! survivable the moment an axis is a *composite* — a body-fat fraction that
//! moves the waist one way, the thigh another and the wrist not at all, or a
//! frame axis that has to reach the shoulders and the hips together.
//! Adding one of those to the old arrangement meant an edit at every site and a
//! hope that none had been missed.
//!
//! So the seam is drawn where it can be checked: these modules own *what the
//! numbers are*, [`super::humanoid`] and [`super::quadruped`] own *what the
//! graph is*, and a new composite is a term in one function rather than a sweep
//! through a thousand-line one.
//!
//! ## Saturation is part of a derivation, not a detail of it
//!
//! Several expressions here clamp, and they are not styling. Each is a wall the
//! mesher puts up, derived and swept in the note beside it, and **anything new
//! that fans into a radius inherits all of them**. A composite that reaches the
//! hips through `hip_x` cannot narrow them past the point where the two hip
//! sockets interpenetrate, however the slider is labelled. Each plan's floors
//! are listed on its own `Dimensions`.
//!
//! ## What a refactor here has to prove
//!
//! `tests/plan.rs` fingerprints every number both plans produce, over the
//! default bodies, the corners of the humanoid space and the first rolls of
//! each. Moving arithmetic between these files is only correct when those
//! fingerprints do not move; changing it on purpose means judging the new
//! bodies by render and re-basing them.
//!
//! ## Provenance
//!
//! Unchanged, and it travelled with the arithmetic: every coefficient carries
//! the note it carried in its plan, including the ones whose note says they are
//! unsourced guesses from the first commit. See the crate docs for what the
//! four provenance tags mean.

pub(crate) mod humanoid;
pub(super) mod quadruped;
