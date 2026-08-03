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
pub mod eye;
pub mod features;
pub mod relief;
pub mod skull;

pub use blink::Blink;
pub use eye::{Eye, EyeParams, Eyes};
pub use features::{FaceParams, Features};
pub use relief::carve as carve_face;
pub use skull::{Skull, refine_face, shape as shape_skull};
