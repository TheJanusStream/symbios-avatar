//! A body's face.
//!
//! The head arrives from the body plan as a smooth egg; everything a face is
//! made of happens here, in stages that measure the built surface rather than
//! trust the plan:
//!
//! - [`skull`] refines the front of the head ([`refine_face`]) and maps it onto
//!   a skull — jaw, chin, cheekbones, occiput ([`shape_skull`]) — driven by
//!   [`HeadTraits`] resolved from the record.
//! - [`relief`] carves the features — nose, brows, lips — as displacements of
//!   the head's own surface ([`carve_face`]), so a face has no seams (#59).
//! - [`mouth`] splits the skin along the parting line and sews a cavity in
//!   behind it, so the jaw can open.
//! - [`neck`] refines, fairs and narrows the column under the skull it carries.
//! - [`features`] builds the one thing that cannot be a displacement: the ears,
//!   conformed to the measured surface.
//! - [`eye`] seats the globes against that surface, with the lids as four rig
//!   joints; [`Blink`] and [`Talk`] decide when the lids and the jaw move, and
//!   both write poses rather than geometry.
//! - [`canon`] is the ruler the rest are authored against — measured landmark
//!   spans rather than plan numbers.

pub mod blink;
pub mod canon;
mod curve;
pub mod expression;
pub mod eye;
pub mod features;
pub mod mouth;
pub mod neck;
pub mod relief;
pub mod skull;
pub mod talk;
pub mod viseme;

pub use blink::Blink;
pub use canon::Canon;
pub use expression::Expression;
pub use eye::{Aperture, Eye, EyeParams, Eyes};
pub use features::{FaceParams, Features};
pub use mouth::Mouth;
pub use neck::{fair as fair_neck, refine as refine_neck, shape as shape_neck};
pub use relief::carve as carve_face;
pub use skull::{HeadTraits, Skull, band_at, refine_face, shape as shape_skull};
pub use talk::{Talk, TalkConfig};
pub use viseme::Viseme;

/// Smoothstep, clamped, for fading a field in and out without leaving a crease
/// where it starts or ends.
///
/// Shared by [`skull`] and [`relief`] rather than written twice: both fade terms
/// against a normalised distance, and two copies of one curve is two things to
/// keep in step for no gain.
pub(crate) fn smooth(at: f32) -> f32 {
    let at = at.clamp(0.0, 1.0);
    at * at * (3.0 - 2.0 * at)
}
