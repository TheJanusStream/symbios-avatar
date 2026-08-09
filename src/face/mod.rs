//! A body's face.
//!
//! Currently the parts that move: [`eye::Eyes`] and the lids that blink over
//! them, plus [`Blink`], which decides when.
//!
//! Faces here are built from separate small solids parented to the head rather
//! than deformed out of it. A head from the body plan is a smooth blob with no
//! eyelid, brow, or lip to deform, and inventing a facial rig over geometry that
//! has no features would be pretending. Rigid parts that rotate are both honest
//! about what exists and — for eyes at least — very nearly what anatomy does.

pub mod blink;
pub mod canon;
pub mod eye;
pub mod features;
pub mod mouth;
pub mod relief;
pub mod skull;
pub mod talk;

pub use blink::Blink;
pub use canon::Canon;
pub use eye::{Aperture, Eye, EyeParams, Eyes};
pub use features::{FaceParams, Features};
pub use mouth::Mouth;
pub use relief::carve as carve_face;
pub use skull::{Skull, refine_face, shape as shape_skull};
pub use talk::{Talk, TalkConfig};

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
