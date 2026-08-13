//! Hair, in two layers over five follicle regions.
//!
//! Hair is the single strongest signal of who a character is — more than the
//! face, at the distances a game is actually played from, because it is the
//! outline you read before any feature resolves. It is also the part that most
//! reliably gives away procedural characters, since a body plan produces a bald
//! skull and a bald skull reads as a mannequin however well it is proportioned.
//!
//! # The five regions and the two layers
//!
//! A head grows hair in five places — the scalp, the brows, the upper lip, the
//! chin and the jaw's flanks — and each of them is cut from the built mesh as a
//! [`Follicle`] mask. Every mask is **measured from the geometry the body
//! actually has**, never derived from the plan's numbers, which is the lesson
//! the shell era paid for twice: a hairline computed from a head radius sits on
//! a different part of every skull.
//!
//! Over each mask, two layers:
//!
//! - **Painted** ([`painted`]) — hair drawn into the skin's own albedo, one
//!   colour and one density per region. It is what stops a thin style reading
//!   as a bald one, and it is the whole of a region a record asks for without
//!   geometry: a shaved jaw, a drawn-on brow.
//! - **Grown** ([`clump`]) — low-poly cards rooted on the mask and lofted along
//!   a guide curve. A region's [`Style`] chooses the curve; the record's
//!   [`Cut`] says how long, how thick, how many and how far they hang.
//!
//! Both layers read the same mask, so the paint and the cards agree about where
//! hair is by construction rather than by two sets of numbers being kept in
//! step.
//!
//! # Every region has its own curve
//!
//! There is no shared fall. A crop, a brow, a moustache and a beard are not one
//! curve at four lengths, and each said so the first time it was rendered: the
//! engine's own downhill points into the eye on a brow ridge and into the throat
//! under a jaw. So each region carries its own catalogue in its own file under
//! [`style`], reading its own measured landmark off [`Follicles`] — the brow
//! ridge, the lip, the chin's pad, the jawline. A change to how one region reads
//! its landmark moves that region's paint and geometry together, and nothing
//! else.
//!
//! # Colour is two colours, and it is free
//!
//! Each region stores an sRGB pair, root and tip, faded along the clump as
//! **vertex colour** — so a gradient costs no texture and no draw call, and grey
//! and fantasy colours come free rather than needing a point on a melanin ramp.
//! [`melanin`](style::melanin) survives as one convenient way to pick a
//! plausible natural pair; nothing is confined to it.
//!
//! # Cost
//!
//! Hair was once 70% of the whole triangle budget. An element is now **one flat
//! card**, four triangles a segment. The per-region counts (`FULL` in
//! [`style`]) are derived from the budget and the measured cost of a card, each
//! style's share of them is granted so that a dear style spends what a crop
//! spends (`CROWD`, per region), and the whole catalogue at its greediest is
//! held under [`clump::MAX_TRIANGLES`] — a ceiling `tests/budget.rs`
//! re-measures rather than quotes. Coverage is bought with **width** before count,
//! because width is free and a card is four triangles: the flanks hold 7.5% of
//! a head's surface against the moustache's 0.5, and that arithmetic is what
//! says whether a region wants bigger cards or more of them.
//!
//! Everything is built in head-local space, as the eyes are, so a renderer
//! parents it to the head joint and the hair follows the body for free — except
//! where a beard hangs, which hands its binding over as it leaves the skin.

pub mod clump;
pub mod follicle;
pub mod painted;
pub mod style;

pub use clump::{Grown, Growth, Root, Shape};
pub use follicle::{Follicle, FollicleParams, Follicles};
pub use painted::{Paint, PaintedHair};
pub use style::{
    BrowStyle, ChinStyle, Cut, FlankStyle, HairRecord, MoustacheStyle, ScalpStyle, Sown, Style,
    Tress,
};
