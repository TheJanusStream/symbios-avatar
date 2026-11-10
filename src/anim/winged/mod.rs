//! A winged body's own motion (W5, #424): on the ground, in the air, and at
//! rest - what a bird, a bat and a sugar glider do that a person and a beast
//! do not.
//!
//! **One body, driven as every body is.** A winged body walks through the
//! gait that walks a person and a beast, rests and emotes through the clips
//! and states a beast's roster plays, and stands through the idle; what this
//! module adds rides on those, one file per concern:
//!
//! - [`ground`]: the bird's walk with its head-bob, its hop and its grounded
//!   run; the bat's crawl and bound; the glider's bound; the drawn toes laid
//!   on the ground through every stride.
//! - [`flight`]: a sustained airborne source its host flies, beating,
//!   gliding, diving, flaring and landing ([`crate::anim::Source::Fly`]).
//! - [`rests`]: a bird's perch and roost, a bat's rest and a glider's curl,
//!   and each family's emotes - its roster, offered by
//!   [`crate::anim::gesture::roster`].
//!
//! **What the body is, stored once** ([`crate::Kinship`] on
//! [`crate::Wings::kinship`]): each family's share and the leading family,
//! written when the wings are grown. Every choice of one family - which gait,
//! whether the wings beat, which roster - is the leading family's, so a cross
//! at exactly 0.5 takes the one answer its surface wears; every blend - a
//! beat's rate and width, a rest's curl and times - is the shares'. A rig
//! from an older build carries none, and every reader alike takes the family
//! its stance belongs to: a bird on two legs, a bat on four. Each concern
//! keeps its own numbers beside it: a bird's [`crate::Gaits`] (its bob and
//! its hop), how low its folded wing hangs ([`crate::Wings::low`]), and the
//! flight's, which it reads off the rig as it flies.
//!
//! **The driver's order** ([`crate::anim::Driver::drive`]), highest first:
//!
//! 1. **The water and a shown motion** outrank a flight: a winged body asked
//!    to swim swims, and a leap or a swim shown is played, whatever its host
//!    says of the air.
//! 2. **A flight** ([`crate::anim::Inputs::flight`]), on a body with wings,
//!    holds it in the air for as long as the host gives one: no leap starts
//!    under it, and it plays **no rest and no emote** - one asked mid-flight
//!    is let go, not kept for the landing, and one playing when a flight is
//!    given (a flight given mid-perch) ends that frame, the change of source
//!    blended out of it as any change is. The flight's own first frame is
//!    the ground posture's values, the wings folded.
//! 3. **The ground gait or the idle** carries every frame no flight holds. A
//!    landing hands back by its host giving no flight once the body is down
//!    ([`flight::FlightMode::Land`] having absorbed it and folded its wings):
//!    the gait or the idle takes over the next frame, blended from the
//!    landing's last pose.
//! 4. **A rest or an emote** is laid over the ground gait or the idle, as a
//!    beast's is, and a rest is got up from in a hurry by whatever stirs the
//!    body - being carried, swimming, a leap.
//!
//! **And the drawn toes after the blend** (`ground::keep_toes_up`) on the
//! gait's and the idle's frames only, the blends into them included: a
//! flight's and a rest's or emote's poses own their feet - a landing plants
//! them, a perch grips them, a curl folds them under.
//!
//! **And a body on four legs kept out of the ground it is handed**
//! (`ground::keep_clear`, #441), last of all, on the pose handed back and
//! never the one kept to blend from: its belly, its hands with their thumbs
//! and its tail, on every frame but a swim's, which the water floats, and a
//! rest's, which is posed against a level - eased out into those and in out
//! of them over two blends, never switched in a frame.
//!
//! **Nothing here moves a person or a beast**: every entry asks the rig's
//! wings first ([`crate::Rig::wings`]), which only a winged body has.
//!
//! docs/winged-anatomy.md, "Motion", has the targets, their sources in
//! research-10 and research-11, what was measured and every departure;
//! `tests/winged_life.rs` drives each family through all three and every
//! hand-off between them.

use crate::plan::WingedFamily;
use crate::rig::Rig;
use crate::wing::{Gaits, Kinship};

pub mod flight;
pub mod ground;
pub mod rests;

#[cfg(test)]
mod keep_clear_tests;

/// Which families `rig` is, or `None` for a body with no wings: see
/// [`crate::Wings::kinship`].
pub(crate) fn kinship(rig: &Rig) -> Option<Kinship> {
    rig.wings.as_ref().map(|wings| wings.kinship(rig))
}

/// The family that leads `rig`, if it has wings.
fn leading(rig: &Rig) -> Option<WingedFamily> {
    kinship(rig).map(|kinship| kinship.leading)
}

/// A bird's gaits, on a body a bird leads - which stands on two legs, as its
/// posture is its leading family's - or `None`.
pub(crate) fn bird(rig: &Rig) -> Option<Gaits> {
    (leading(rig) == Some(WingedFamily::Bird) && !rig.four_legged())
        .then(|| rig.wings.as_ref().map(|wings| wings.gaits))
        .flatten()
}

/// Whether `rig` bounds where a beast would trot: a bat's or a glider's,
/// standing on four legs.
pub(crate) fn bounds(rig: &Rig) -> bool {
    rig.four_legged() && matches!(leading(rig), Some(WingedFamily::Bat | WingedFamily::Glider))
}

/// Whether `rig` crawls: a bat's, on its wrists and its feet, every leg of it
/// splayed - the elbows up over the back, the knees up and out.
///
/// **Such a leg is not a beast's**, and the beast's stride does not hold it:
/// the pantograph that stands a beast's paw and the pole it swings its leg
/// about both work in the body's side plane, where a bat's leg is folded out
/// of it, and its elbow popped a third of a metre between frames in a pivot
/// and a bound (#424). Its legs are solved about the bend it was built with,
/// each foot stood upright, and it does not fold a fore paw in its swing.
pub(crate) fn crawls(rig: &Rig) -> bool {
    rig.four_legged() && leading(rig) == Some(WingedFamily::Bat)
}
