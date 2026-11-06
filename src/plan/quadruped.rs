//! The four-legged body plan.
//!
//! Structurally this is the humanoid's problem twice over: two girdles, each a
//! four-way joint - the pelvis carrying the loin, the tail and two hind legs,
//! the forechest the ribs, the neck and two fore legs - with a run of four
//! trunk rings between them (#375). Each leg's bones inside the trunk, the
//! shoulder blade and the shoulder on a fore leg and the hip joint on a hind
//! one, are markers the rig keeps and the cage sweeps past (#374), so a leg
//! meshes from its girdle straight to the elbow or the stifle under the body.
//! docs/quadruped-anatomy.md has the targets and what the body departs from.
//!
//! **What is left in this file is the graph, not the geometry.** The axes, how
//! they are clamped, rolled and encoded are here; every number a node is built
//! from comes out of [`Dimensions`], including the correlation this plan is
//! most worth reading for — legginess slims the barrel rather than changing the
//! back height, because at a given height a leggy animal is a slender one and a
//! squat animal is a barrel. One thing here asks the cage: a body it cannot
//! hull is eased toward its family's before it is handed out (#391, see
//! `QuadrupedParams::skeleton`).

use std::cell::RefCell;
use std::collections::VecDeque;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::derive::quadruped::{
    AS_ASKED, Dimensions, EASED_AXES, Ease, FORE_INSIDE, HIND_INSIDE, TRUNK_NODES,
};

/// One leg node as the derive gives it: where it stands and its radius.
type LegNode = (Vec3, f32);
use super::kin::{self, Anchored, Blended, Bred, Draw, Kin, Lineage, Streams};
use super::ornament::Ornaments;
use super::{Anatomy, Coat, CompositePriors, Composites, DEFAULT_BODY_FAT, Fur, Limb, Zone};
use super::{
    BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_span, take_length, take_signed,
    take_span, take_unit,
};
use crate::cage::limb::HALF_SEGMENT;
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest back height this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (0.25, 1.8);

/// Which kind of animal a quadruped is drawn as (#366): the anchor its axes
/// are rolled around, where a roll draws its parts (#367) and its coat and
/// eyes (#368) from, and the anatomy its body is built in (#374).
///
/// **The body reads it.** A family is a frame - a dog's, a cat's or a
/// horse's bone lengths, trunk sections and stance, measured in withers
/// heights (docs/quadruped-anatomy.md) - and what `appeal` makes of it: a
/// puppy's, a kitten's or a foal's proportions. The body's axes move that
/// frame by how far each sits from the family's own centre, so a roll at its
/// family's centre is the family's frame, and a creator who drags a cat's
/// axes into a horse's range has a cat stretched toward a horse. A hybrid is
/// both: rolled between its two families, and built in their frames blended
/// by its blend. The head still reads the axes alone, as P2 built it.
///
/// Written as its lowercase name. A name this build does not know - a family a
/// newer one added - reads as a dog rather than failing the record: the body
/// is in its axes, which come through whole, and only the anchor its next roll
/// centres on is lost.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Family {
    /// Dog-like: the plan's neutral.
    #[default]
    Dog,
    /// Cat-like.
    Cat,
    /// Horse-like.
    Horse,
}

named!(Family: Dog => "dog", Cat => "cat", Horse => "horse");

/// The shape of a quadruped's pupils (#367): round on a dog, a vertical slit
/// on a cat, a horizontal bar on a horse.
///
/// **Stored rather than read off the family**, as a part is: a record keeps
/// its eyes whatever a reader makes of its family, and a cross can take
/// either parent's. A re-roll gives a body its family's, and a cross the
/// family it is blended toward as often as its blend says.
///
/// Written as its lowercase name. A name or share-code byte this build does
/// not know reads as round, for [`Family`]'s reason.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pupil {
    /// Round: a dog's, and a person's.
    #[default]
    Round,
    /// A vertical slit: a cat's.
    Slit,
    /// A horizontal bar, held level: a horse's.
    Bar,
}

named!(Pupil: Round => "round", Slit => "slit", Bar => "bar");

impl Pupil {
    /// A family's own.
    #[must_use]
    pub fn of(family: Family) -> Self {
        match family {
            Family::Dog => Pupil::Round,
            Family::Cat => Pupil::Slit,
            Family::Horse => Pupil::Bar,
        }
    }
}

/// What a quadruped's nose is (#367): a bare leather pad, a dog's or a
/// cat's, or the soft muzzle a horse breathes through open nostrils.
///
/// A stored part, for [`Pupil`]'s reason. Written as its lowercase name; a
/// name or share-code byte this build does not know reads as leather.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Nose {
    /// A leather pad at the muzzle's tip, its nostrils cut into its front.
    #[default]
    Leather,
    /// No pad: the muzzle's own soft end, with nostrils open on its front.
    Soft,
}

named!(Nose: Leather => "leather", Soft => "soft");

impl Nose {
    /// A family's own.
    #[must_use]
    pub fn of(family: Family) -> Self {
        match family {
            Family::Dog | Family::Cat => Nose::Leather,
            Family::Horse => Nose::Soft,
        }
    }
}

/// What a quadruped stands on (#367): paws with their claws showing, a
/// dog's; paws with them sheathed, a cat's; or hooves, a horse's.
///
/// A stored part, for [`Pupil`]'s reason, and not read off `stance`, which
/// is how the leg stands rather than what it stands on: a cross may walk on
/// its toes in hooves. Written as its lowercase name; a name or share-code
/// byte this build does not know reads as a clawed paw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Feet {
    /// Paws on four toes, the claws showing at their tips.
    #[default]
    Claws,
    /// Paws on four rounder toes, the claws sheathed.
    Pads,
    /// Hooves: a wall of horn round a flat sole.
    Hooves,
}

named!(Feet: Claws => "claws", Pads => "pads", Hooves => "hooves");

impl Feet {
    /// A family's own.
    #[must_use]
    pub fn of(family: Family) -> Self {
        match family {
            Family::Dog => Feet::Claws,
            Family::Cat => Feet::Pads,
            Family::Horse => Feet::Hooves,
        }
    }
}

/// The byte a share code writes for "no hybrid".
const NO_HYBRID: u8 = 255;

/// Where a re-roll centres `appeal` on every family: well toward cute, the
/// owner's call for the epic (#364), with room either side of it.
///
/// Provenance: **tuned by render** (#366), over family galleries at 0.6, 1.0
/// and 1.2. At 0.6 a roll read as the plan's neutral with a larger head; at
/// 1.2 the dogs were puppies, the cats round-headed and the horses ponies,
/// every family still itself. One, to leave the eyes and ears of #367 room
/// to carry the rest.
const CUTE: f32 = 1.0;

/// Parameters describing one quadruped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct QuadrupedParams {
    /// The body's size, in metres, within [`super::quadruped_height_range`]:
    /// the back height P1 measured it as. The body stands at a withers height
    /// 1.2 times this (#374), the top of the shoulder blades over the ground,
    /// which keeps every record the size it was in a world while the body
    /// under it was rebuilt; every proportion is a share of that withers
    /// height.
    #[serde(with = "super::scaled")]
    pub height: f32,
    /// Body length from shoulder to hip.
    #[serde(with = "super::scaled")]
    pub body_length: f32,
    // `build` and `muscle` were here and are gone (#178): they retired into the
    // `mass` and `bodyFat` composites, as the humanoid's did (#164), which say
    // how much body there is and how it is spent without the two of them
    // saying it twice.
    /// Legginess. Longer legs slim the body at a fixed back height.
    #[serde(with = "super::scaled")]
    pub leg_length: f32,
    /// Neck length.
    #[serde(with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(with = "super::scaled")]
    pub head_size: f32,
    /// Tail length.
    #[serde(with = "super::scaled")]
    pub tail_length: f32,
    /// Which kind of animal this is rolled as (#366). See [`Family`].
    pub family: Family,
    /// The second family of a cross, if this is one (#366).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<Family>,
    /// How far a cross is rolled toward its [`Self::hybrid`] family, `0..=1`:
    /// none of it at zero, all of it at one.
    #[serde(with = "super::scaled")]
    pub blend: f32,
    /// How long the muzzle is: a snub cat's face at the bottom, a horse's
    /// long head at the top (#366).
    #[serde(with = "super::scaled")]
    pub muzzle_length: f32,
    /// What the legs stand on: flat feet at the bottom, toes (a dog's, a
    /// cat's) in the middle, hooves at the top - which also raises the hock and
    /// the carpus up longer bones of the foot (#366).
    #[serde(with = "super::scaled")]
    pub stance: f32,
    /// How the neck is carried: low and forward at the bottom, high and arched
    /// at the top (#366).
    #[serde(with = "super::scaled")]
    pub neck_carriage: f32,
    /// How the tail is carried: hanging at the bottom, a sabre curve in the
    /// middle, raised and curled at the top (#366).
    #[serde(with = "super::scaled")]
    pub tail_carriage: f32,
    /// How deep the chest is, against the loin behind it (#366).
    #[serde(with = "super::scaled")]
    pub chest_depth: f32,
    /// How the whole body is drawn, from naturalistic at the bottom to cute
    /// at the top (#366). Applied over the anatomy, it grows the head and
    /// shortens the muzzle on every family, and draws the body its family's
    /// young has (#372, the owner's call): a dog or a cat a puppy's, its
    /// barrel rounder rather than longer under a shorter, thicker neck, on
    /// legs at their adult share with bigger paws; a horse a foal's, a short
    /// body on long legs. A cross blends its two families' by its blend. A
    /// re-roll leans it cute, the owner's call.
    #[serde(with = "super::scaled")]
    pub appeal: f32,
    /// How broad the muzzle is: a borzoi's wedge at the bottom, a boxer's
    /// box at the top (#367). Wider at the nose than at the stop, and
    /// squarer in section as it widens.
    #[serde(with = "super::scaled")]
    pub muzzle_breadth: f32,
    /// How the muzzle is set into the skull: a Roman nose, the bridge
    /// raised to the forehead's line and bowed, at the bottom; a straight
    /// profile in the middle; a deep stop under a high brow at the top - a
    /// horse's face, a hound's, a cat's (#367).
    #[serde(with = "super::scaled")]
    pub stop: f32,
    /// How the eyes are set: to the front of a flat face at the bottom, a
    /// cat's, which look where it is going; on the sides of the skull at the
    /// top, a horse's, which watch around it (#367).
    #[serde(with = "super::scaled")]
    pub eye_set: f32,
    /// The pupils' shape (#367). See [`Pupil`].
    pub pupil: Pupil,
    /// How large the ears are against the head (#367).
    #[serde(with = "super::scaled")]
    pub ear_size: f32,
    /// How the ears end: round-tipped leaves at the bottom, pointed
    /// triangles at the top (#367).
    #[serde(with = "super::scaled")]
    pub ear_point: f32,
    /// How the ears are carried: dropped and hanging beside the face at the
    /// bottom, folded back in a rose in the middle, pricked up at the top
    /// (#367).
    #[serde(with = "super::scaled")]
    pub ear_carriage: f32,
    /// What the nose is (#367). See [`Nose`].
    pub nose: Nose,
    /// What the legs stand on (#367). See [`Feet`].
    pub feet: Feet,
    /// How long the whiskers are: none at the bottom, a cat's long fan at
    /// the top (#367).
    #[serde(with = "super::scaled")]
    pub whiskers: f32,
    /// What it wears on its head (#402), which a rare roll draws too, under the head's
    /// lock (#403). See [`Ornaments`]. On a record, a unicorn's horn goes on being
    /// `horn` and anything else is `ornaments`: `on_record`.
    #[serde(flatten, with = "super::ornament::on_record")]
    pub ornaments: Ornaments,
    /// What colour its fur is and how it is marked (#368). See [`Coat`].
    pub coat: Coat,
    /// How long its fur grows where a beast grows long hair (#369). See
    /// [`Fur`].
    pub fur: Fur,
}

/// Neutral back height, used when a record omits the field.
fn default_height() -> f32 {
    0.58
}

/// The stature envelope: [`HEIGHT_RANGE`] stretched about the default.
///
/// The raw stretch runs to −0.41 m — the conservative floor sits closer to
/// the default than the ceiling does, and tripling the short side crosses
/// zero. Every dimension of the plan scales with height, so any positive
/// value meshes (the sweep bisected the wall to +0.001); the floor is five
/// centimetres because a creature is a thing in a world, not a float.
fn height_envelope() -> (f32, f32) {
    let raw = super::explore_range(default_height(), HEIGHT_RANGE);
    (raw.0.max(0.05), raw.1)
}

/// The envelope every signed axis clamps and encodes against.
fn signed_envelope() -> (f32, f32) {
    super::explore_range(0.0, (-1.0, 1.0))
}

/// The `muscle` envelope its retired bytes span on an older share code: its
/// default sat at the bottom of its range.
fn muscle_envelope() -> (f32, f32) {
    super::explore_range(0.0, (0.0, 1.0))
}

/// The `blend` range: a share, so it is not explored past its ends.
const BLEND_RANGE: (f32, f32) = (0.0, 1.0);

/// How many halvings each search for an eased body takes (#391, see
/// `QuadrupedParams::skeleton`): a 64th of an axis's step from its family's
/// centre, under a tenth of an axis unit from the envelope's far end.
const EASE_HALVINGS: usize = 6;

thread_local! {
    /// The last [`EASES_KEPT`] eases found on this thread, each with the record
    /// and composites it was found for (#391): see `QuadrupedParams::ease`.
    static EASES: RefCell<VecDeque<(QuadrupedParams, Composites, Ease)>> =
        const { RefCell::new(VecDeque::new()) };
}

/// How many eases [`EASES`] remembers: a viewer's own body, the one it is
/// editing and a few peers' are asked for over and over, every frame alike.
const EASES_KEPT: usize = 4;

#[cfg(test)]
thread_local! {
    /// How many eases this thread has searched for, which
    /// `a_record_asked_again_is_not_searched_again` reads.
    static SEARCHED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Whether the cage hulls `skeleton` and the rig roots it: what
/// [`crate::Avatar::build`] asks of a body before anything else.
fn meshes(skeleton: &Skeleton) -> bool {
    crate::cage::build_cage(skeleton, &crate::cage::CageConfig::default()).is_ok()
        && crate::rig::Rig::from_skeleton(skeleton).is_ok()
}

/// How far a roll's back height may stray from its family's, as factors on
/// the anchor's mean. [`Rolls::shape`]'s rare wildcard spans this rather than
/// the whole envelope, because a family is told apart by its size before
/// anything else: drawn anywhere in the envelope, one cat in thirty would
/// stand taller than a horse.
const HEIGHT_BAND: (f32, f32) = (0.5, 1.6);

/// Where each family's rolls centre every axis (#366): the anchors the
/// families are told apart by. Read [`Family`] for why these are a roll's
/// centre and not a fixed body, and [`kin`] for how a cross blends them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anchor {
    height: Draw,
    body_length: Draw,
    leg_length: Draw,
    neck_length: Draw,
    head_size: Draw,
    tail_length: Draw,
    muzzle_length: Draw,
    stance: Draw,
    neck_carriage: Draw,
    tail_carriage: Draw,
    chest_depth: Draw,
    appeal: Draw,
    muzzle_breadth: Draw,
    stop: Draw,
    eye_set: Draw,
    ear_size: Draw,
    ear_point: Draw,
    ear_carriage: Draw,
    whiskers: Draw,
    /// Where the family's [`Composites::mass`] centres (#178).
    mass: Draw,
    /// The family's body fat at neutral mass and age.
    body_fat: f32,
    /// Where the family's [`Composites::age`] centres, in years.
    age: Draw,
}

impl Anchor {
    /// The anchor a roll of `params` centres on: its family's, drawn toward
    /// its hybrid's by its blend.
    fn for_params(params: &QuadrupedParams) -> Self {
        Self::of_lineage(params.lineage())
    }
}

impl Anchored for Anchor {
    type Kin = Family;

    /// The anchor of one family.
    ///
    /// Heights are a back's, in metres, at a game's scale rather than a
    /// zoo's: a cat a little over knee-high to the person it walks beside, a
    /// horse a head shorter than a tall one. The rest are axis units.
    fn of(family: Family) -> Self {
        match family {
            Family::Dog => Self {
                height: Draw::at(0.58, 0.16),
                body_length: Draw::at(0.0, 0.4),
                leg_length: Draw::at(0.0, 0.45),
                neck_length: Draw::at(0.0, 0.4),
                head_size: Draw::at(0.0, 0.35),
                tail_length: Draw::at(0.0, 0.45),
                muzzle_length: Draw::at(0.0, 0.45),
                stance: Draw::at(0.0, 0.1),
                neck_carriage: Draw::at(0.0, 0.35),
                tail_carriage: Draw::at(0.3, 0.45),
                chest_depth: Draw::at(0.1, 0.35),
                appeal: Draw::at(CUTE, 0.3),
                muzzle_breadth: Draw::at(0.0, 0.55),
                stop: Draw::at(0.1, 0.5),
                eye_set: Draw::at(-0.1, 0.3),
                // A dog's ears are anything: every carriage, size and tip.
                ear_size: Draw::at(0.0, 0.5),
                ear_point: Draw::at(-0.1, 0.55),
                ear_carriage: Draw::at(0.0, 0.7),
                // Fewer and shorter than a cat's: a fan is half of what
                // tells a cat's face from a dog's.
                whiskers: Draw::at(-0.8, 0.3),
                mass: Draw::at(0.0, 0.6),
                body_fat: DEFAULT_BODY_FAT,
                age: Draw::at(32.0, 12.0),
            },
            Family::Cat => Self {
                height: Draw::at(0.40, 0.08),
                body_length: Draw::at(0.2, 0.25),
                leg_length: Draw::at(-0.15, 0.25),
                neck_length: Draw::at(-0.8, 0.2),
                head_size: Draw::at(0.6, 0.25),
                tail_length: Draw::at(0.9, 0.3),
                muzzle_length: Draw::at(-0.85, 0.15),
                stance: Draw::at(0.0, 0.05),
                neck_carriage: Draw::at(-0.5, 0.2),
                tail_carriage: Draw::at(0.55, 0.35),
                chest_depth: Draw::at(-0.2, 0.2),
                appeal: Draw::at(CUTE, 0.3),
                muzzle_breadth: Draw::at(0.3, 0.25),
                stop: Draw::at(0.7, 0.25),
                eye_set: Draw::at(-0.8, 0.15),
                ear_size: Draw::at(0.3, 0.25),
                ear_point: Draw::at(0.6, 0.2),
                ear_carriage: Draw::at(0.9, 0.1),
                whiskers: Draw::at(0.6, 0.25),
                mass: Draw::at(-0.4, 0.4),
                body_fat: DEFAULT_BODY_FAT + 0.02,
                age: Draw::at(32.0, 12.0),
            },
            Family::Horse => Self {
                height: Draw::at(1.45, 0.15),
                body_length: Draw::at(-1.0, 0.25),
                leg_length: Draw::at(0.0, 0.2),
                neck_length: Draw::at(0.85, 0.2),
                head_size: Draw::at(-0.1, 0.2),
                tail_length: Draw::at(-0.5, 0.25),
                muzzle_length: Draw::at(0.75, 0.15),
                stance: Draw::at(1.0, 0.05),
                neck_carriage: Draw::at(0.55, 0.2),
                tail_carriage: Draw::at(-0.9, 0.1),
                chest_depth: Draw::at(1.2, 0.25),
                appeal: Draw::at(CUTE, 0.3),
                muzzle_breadth: Draw::at(-0.2, 0.25),
                stop: Draw::at(-0.5, 0.35),
                eye_set: Draw::at(0.85, 0.15),
                ear_size: Draw::at(-0.2, 0.2),
                ear_point: Draw::at(0.8, 0.15),
                ear_carriage: Draw::at(0.95, 0.05),
                whiskers: Draw::at(-1.0, 0.1),
                mass: Draw::at(0.6, 0.35),
                body_fat: DEFAULT_BODY_FAT - 0.06,
                age: Draw::at(32.0, 12.0),
            },
        }
    }

    /// A cross's: every draw `share` of the way to `other`'s, and the body
    /// fat between the two.
    fn toward(own: Self, other: Self, share: f32) -> Self {
        let mix = |a: Draw, b: Draw| a.toward(b, share);
        Self {
            height: mix(own.height, other.height),
            body_length: mix(own.body_length, other.body_length),
            leg_length: mix(own.leg_length, other.leg_length),
            neck_length: mix(own.neck_length, other.neck_length),
            head_size: mix(own.head_size, other.head_size),
            tail_length: mix(own.tail_length, other.tail_length),
            muzzle_length: mix(own.muzzle_length, other.muzzle_length),
            stance: mix(own.stance, other.stance),
            neck_carriage: mix(own.neck_carriage, other.neck_carriage),
            tail_carriage: mix(own.tail_carriage, other.tail_carriage),
            chest_depth: mix(own.chest_depth, other.chest_depth),
            appeal: mix(own.appeal, other.appeal),
            muzzle_breadth: mix(own.muzzle_breadth, other.muzzle_breadth),
            stop: mix(own.stop, other.stop),
            eye_set: mix(own.eye_set, other.eye_set),
            ear_size: mix(own.ear_size, other.ear_size),
            ear_point: mix(own.ear_point, other.ear_point),
            ear_carriage: mix(own.ear_carriage, other.ear_carriage),
            whiskers: mix(own.whiskers, other.whiskers),
            mass: mix(own.mass, other.mass),
            body_fat: own.body_fat + (other.body_fat - own.body_fat) * share,
            age: mix(own.age, other.age),
        }
    }
}

/// Where a roll of a body centres the axes its body is built from (#374): its
/// family's, a cross's blended. The derive reads each body axis as a step
/// from here, so a roll at its family's centre builds the family's own frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AxisCentres {
    /// `legLength`'s centre.
    pub leg_length: f32,
    /// `bodyLength`'s centre.
    pub body_length: f32,
    /// `chestDepth`'s centre.
    pub chest_depth: f32,
    /// `stance`'s centre.
    pub stance: f32,
    /// [`Composites::mass`]'s centre: the trunk's depth reads mass and body
    /// fat as steps from the family's own body, as its frame was measured
    /// on.
    pub mass: f32,
    /// The family's body fat at that mass.
    pub body_fat: f32,
    /// `appeal`'s centre, which a body eased toward its family's
    /// (`Dimensions::eased`, #391) reads with the four above.
    pub appeal: f32,
    /// `neckLength`'s centre.
    pub neck_length: f32,
    /// `neckCarriage`'s centre.
    pub neck_carriage: f32,
    /// [`Composites::age`]'s centre, in years.
    pub age: f32,
}

/// See [`AxisCentres`].
pub(crate) fn axis_centres(params: &QuadrupedParams) -> AxisCentres {
    let anchor = Anchor::for_params(params);
    AxisCentres {
        leg_length: anchor.leg_length.mean,
        body_length: anchor.body_length.mean,
        chest_depth: anchor.chest_depth.mean,
        stance: anchor.stance.mean,
        mass: anchor.mass.mean,
        body_fat: anchor.body_fat,
        appeal: anchor.appeal.mean,
        neck_length: anchor.neck_length.mean,
        neck_carriage: anchor.neck_carriage.mean,
        age: anchor.age.mean,
    }
}

impl Default for QuadrupedParams {
    fn default() -> Self {
        Self {
            height: default_height(),
            body_length: 0.0,
            leg_length: 0.0,
            neck_length: 0.0,
            head_size: 0.0,
            tail_length: 0.0,
            family: Family::Dog,
            hybrid: None,
            blend: 0.0,
            muzzle_length: 0.0,
            stance: 0.0,
            neck_carriage: 0.0,
            tail_carriage: 0.0,
            chest_depth: 0.0,
            appeal: 0.0,
            muzzle_breadth: 0.0,
            stop: 0.0,
            eye_set: 0.0,
            pupil: Pupil::Round,
            ear_size: 0.0,
            ear_point: 0.0,
            ear_carriage: 0.0,
            nose: Nose::Leather,
            feet: Feet::Claws,
            whiskers: 0.0,
            ornaments: Ornaments::default(),
            coat: Coat::default(),
            fur: Fur::default(),
        }
    }
}

impl QuadrupedParams {
    /// The stature envelope `sanitize` clamps to, in metres.
    ///
    /// Public so an editor's slider and the clamp cannot disagree about
    /// where the axis ends.
    #[must_use]
    pub fn height_envelope() -> (f32, f32) {
        height_envelope()
    }

    /// Makes this body a different kind of animal, carrying its own
    /// character across: what a creator's family picker calls (#366).
    ///
    /// Setting [`Self::family`] alone changes nothing a creator can see,
    /// because the geometry reads only the axes and a family is where a roll
    /// centres them. So every axis moves as far from its new anchor as it sat
    /// from its old one, in the anchor's own spread: a long-legged dog becomes
    /// a long-legged cat rather than the mean cat, and dragging a cross's blend
    /// morphs the body between its two families as the blend moves. Sanitised
    /// afterwards, so a cross of a family with itself comes out no cross. The
    /// same as [`Blended::set_lineage`], in the plan's own words.
    pub fn set_kind(&mut self, family: Family, hybrid: Option<Family>, blend: f32) {
        kin::change_kind(
            self,
            Lineage {
                family,
                hybrid,
                blend,
            },
        );
    }
}

/// The streams a quadruped's family and cross are rolled on (#366).
const STREAMS: Streams = Streams {
    family: "quadruped.family",
    hybrid_family: "quadruped.hybridFamily",
    hybrid: "quadruped.hybrid",
    blend: "quadruped.blend",
};

impl Kin for Family {
    const ALL: &'static [Self] = &Family::ALL;

    fn name(self) -> &'static str {
        Family::name(self)
    }
}

impl Blended for QuadrupedParams {
    type Kin = Family;

    fn lineage(&self) -> Lineage<Family> {
        Lineage {
            family: self.family,
            hybrid: self.hybrid,
            blend: self.blend,
        }
    }

    fn set_lineage(&mut self, lineage: Lineage<Family>) {
        kin::change_kind(self, lineage);
    }
}

impl Bred for QuadrupedParams {
    type Anchor = Anchor;

    fn put_lineage(&mut self, lineage: Lineage<Family>) {
        self.family = lineage.family;
        self.hybrid = lineage.hybrid;
        self.blend = lineage.blend;
    }

    fn carry(&mut self, from: &Anchor, to: &Anchor) {
        to.height.carry(&mut self.height, from.height);
        to.body_length
            .carry(&mut self.body_length, from.body_length);
        to.leg_length.carry(&mut self.leg_length, from.leg_length);
        to.neck_length
            .carry(&mut self.neck_length, from.neck_length);
        to.head_size.carry(&mut self.head_size, from.head_size);
        to.tail_length
            .carry(&mut self.tail_length, from.tail_length);
        to.muzzle_length
            .carry(&mut self.muzzle_length, from.muzzle_length);
        to.stance.carry(&mut self.stance, from.stance);
        to.neck_carriage
            .carry(&mut self.neck_carriage, from.neck_carriage);
        to.tail_carriage
            .carry(&mut self.tail_carriage, from.tail_carriage);
        to.chest_depth
            .carry(&mut self.chest_depth, from.chest_depth);
        to.appeal.carry(&mut self.appeal, from.appeal);
        to.muzzle_breadth
            .carry(&mut self.muzzle_breadth, from.muzzle_breadth);
        to.stop.carry(&mut self.stop, from.stop);
        to.eye_set.carry(&mut self.eye_set, from.eye_set);
        to.ear_size.carry(&mut self.ear_size, from.ear_size);
        to.ear_point.carry(&mut self.ear_point, from.ear_point);
        to.ear_carriage
            .carry(&mut self.ear_carriage, from.ear_carriage);
        to.whiskers.carry(&mut self.whiskers, from.whiskers);
    }

    /// A part goes with its family, but a cross keeps whichever of its two
    /// families' parts it has, so dragging the blend does not flip the eyes
    /// or the nose back and forth at the middle.
    fn hand_over(&mut self) {
        let lineage = self.lineage();
        self.pupil = lineage.handed_over(self.pupil, Pupil::of);
        self.nose = lineage.handed_over(self.nose, Nose::of);
        self.feet = lineage.handed_over(self.feet, Feet::of);
    }
}

impl BodyPlan for QuadrupedParams {
    /// A beast's, its feet paws or hooves as its own feet are (#367, #370).
    fn anatomy(&self) -> Anatomy {
        Anatomy {
            feet: if self.feet == Feet::Hooves {
                super::FootKind::Hooves
            } else {
                super::FootKind::Beast
            },
            ..Anatomy::BEAST
        }
    }

    /// Its family's (#178, #366), and no frame axis: the sexual dimorphism of
    /// a cat and of a horse are not one axis, and `femininity` is a measured
    /// person's. A cross is centred between its two families, as its axes are.
    fn composite_priors(&self) -> CompositePriors {
        let anchor = Anchor::for_params(self);
        CompositePriors {
            frame: false,
            mass: (anchor.mass.mean, anchor.mass.sigma),
            body_fat: anchor.body_fat,
            age: (anchor.age.mean, anchor.age.sigma),
        }
    }

    /// Its family's eyes (#368): a cross's leading family's.
    fn iris(&self, shade: f32) -> [f32; 3] {
        super::coat::iris(self.lineage().leading(), shade)
    }

    fn sanitize(&mut self) {
        // See [`super::humanoid::HumanoidParams::sanitize`]: same shape, same
        // reason, and it carried the same bug (#55).
        let default = Self::default();
        self.height = super::sanitize_axis(self.height, default.height, height_envelope());
        for (axis, fallback) in [
            (&mut self.body_length, default.body_length),
            (&mut self.leg_length, default.leg_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.head_size, default.head_size),
            (&mut self.tail_length, default.tail_length),
            (&mut self.muzzle_length, default.muzzle_length),
            (&mut self.stance, default.stance),
            (&mut self.neck_carriage, default.neck_carriage),
            (&mut self.tail_carriage, default.tail_carriage),
            (&mut self.chest_depth, default.chest_depth),
            (&mut self.appeal, default.appeal),
            (&mut self.muzzle_breadth, default.muzzle_breadth),
            (&mut self.stop, default.stop),
            (&mut self.eye_set, default.eye_set),
            (&mut self.ear_size, default.ear_size),
            (&mut self.ear_point, default.ear_point),
            (&mut self.ear_carriage, default.ear_carriage),
            (&mut self.whiskers, default.whiskers),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, signed_envelope());
        }
        // A cross of a family with itself is no cross, and a body that is no
        // cross is blended toward nothing. (A cross at either end of its blend
        // stays a cross: it is centred on one family with the wider spread of
        // two, which is a state an editor's slider passes through.)
        self.coat.sanitize();
        self.fur.sanitize();
        self.ornaments.sanitize();
        if self.hybrid == Some(self.family) {
            self.hybrid = None;
        }
        self.blend = if self.hybrid.is_some() {
            super::sanitize_axis(self.blend, 0.5, BLEND_RANGE)
        } else {
            0.0
        };
    }

    /// **A body its cage cannot hull is eased toward its family's until it
    /// can** (#391). Every slider runs to the exploration envelope's ends,
    /// where no roll reaches, and about one corner in thirteen crowds a
    /// girdle's sockets off its hull: a chest hung below the elbows on short
    /// legs and a heavy body, the elbows' sockets buried under the ribs'; a
    /// forechest deeper than the run back to the ribs; a foal's short thigh
    /// under a heavy croup; a heavy cat's neck carried low. Which bodies is
    /// the cage's plane test, and no ratio of the derived body tells them from
    /// the ones that mesh (`examples/cornercensus`), so the cage is asked.
    ///
    /// The body as asked is built first and returned whenever it meshes,
    /// which is every body a roll draws, bit for bit. One that does not is
    /// eased (`Dimensions::eased`): every eased axis by the largest common
    /// share of its step from its family's centre that meshes, then each
    /// axis back to what was asked where the body still meshes with it, then
    /// the axes still eased together again as far as they go - so only the
    /// axes crowding a girdle give anything up, and each by as little as
    /// `EASE_HALVINGS` can find. The record keeps what its creator asked;
    /// only the body built from it has less of it. A body whose family's own
    /// frame does not mesh either is returned as asked, and refuses as it
    /// did: none of 70,000 sampled corners is one.
    ///
    /// Two things follow from asking the cage. Its test is the cage's own
    /// arithmetic, whose trigonometry is the platform's, so a body at the very
    /// edge of meshing may ease a step more or less in a browser than natively,
    /// as it could already build on one and refuse on the other. And it asks
    /// with the default `CageConfig`: a caller building with another cage may
    /// get an eased body that still refuses. The answer is remembered for the
    /// last few records asked on a thread, so an editor that asks every frame
    /// pays once per change.
    fn skeleton(&self, composites: &Composites) -> Skeleton {
        self.assemble(&Dimensions::eased(self, composites, &self.ease(composites)))
    }

    /// The composites are drawn first and centred on the family's own
    /// ([`Self::composite_priors`], #178), so nothing here reads them: the
    /// family's girth is in them, and the axes left here are proportions.
    fn reroll(&mut self, category: Category, rolls: &Rolls, _composites: &Composites) {
        // See [`super::humanoid::HumanoidParams::reroll`]: same shape draws,
        // same reasons, same stream names as before (#160) - centred, since
        // #366, on the family's anchor ([`Anchor::for_params`]), which
        // [`Self::reroll_family`] drew before any axis.
        let signed = signed_envelope();
        let anchor = Anchor::for_params(self);
        let draw =
            |stream: &str, at: Draw, envelope| rolls.shape(stream, at.mean, at.sigma, envelope);
        match category {
            Category::Stature => {
                let (low, high) = height_envelope();
                let band = (
                    (anchor.height.mean * HEIGHT_BAND.0).max(low),
                    (anchor.height.mean * HEIGHT_BAND.1).min(high),
                );
                self.height = draw("quadruped.height", anchor.height, band);
            }
            Category::Build => {
                self.chest_depth = draw("quadruped.chestDepth", anchor.chest_depth, signed);
            }
            Category::Frame => {
                self.body_length = draw("quadruped.bodyLength", anchor.body_length, signed);
            }
            Category::Proportions => {
                self.leg_length = draw("quadruped.legLength", anchor.leg_length, signed);
                self.neck_length = draw("quadruped.neckLength", anchor.neck_length, signed);
                self.tail_length = draw("quadruped.tailLength", anchor.tail_length, signed);
                self.stance = draw("quadruped.stance", anchor.stance, signed);
                self.neck_carriage = draw("quadruped.neckCarriage", anchor.neck_carriage, signed);
                self.tail_carriage = draw("quadruped.tailCarriage", anchor.tail_carriage, signed);
                self.appeal = draw("quadruped.appeal", anchor.appeal, signed);
                self.feet = self.lineage().drawn_part(rolls, "quadruped.feet", Feet::of);
            }
            Category::Head => {
                self.head_size = draw("quadruped.headSize", anchor.head_size, signed);
                self.muzzle_length = draw("quadruped.muzzleLength", anchor.muzzle_length, signed);
                self.muzzle_breadth =
                    draw("quadruped.muzzleBreadth", anchor.muzzle_breadth, signed);
                self.stop = draw("quadruped.stop", anchor.stop, signed);
                self.eye_set = draw("quadruped.eyeSet", anchor.eye_set, signed);
                self.ear_size = draw("quadruped.earSize", anchor.ear_size, signed);
                self.ear_point = draw("quadruped.earPoint", anchor.ear_point, signed);
                self.ear_carriage = draw("quadruped.earCarriage", anchor.ear_carriage, signed);
                self.nose = self.lineage().drawn_part(rolls, "quadruped.nose", Nose::of);
                self.whiskers = draw("quadruped.whiskers", anchor.whiskers, signed);
                // Rarely, off a short list, and over a creator's: see [`Ornaments`].
                self.ornaments = Ornaments::rolled(rolls, self.muzzle_length);
                self.pupil = self
                    .lineage()
                    .drawn_part(rolls, "quadruped.pupil", Pupil::of);
            }
            // Its coat (#368), drawn from its family's loci - a cross's mixed
            // by its blend.
            Category::Colouring => {
                self.coat
                    .reroll(rolls, self.family, self.hybrid, self.blend);
            }
            // Its fur (#369): on a beast the hair lock is the fur lock, and
            // a cross's odds are mixed by its blend as its coat's are.
            Category::Hair => {
                self.fur.reroll(rolls, self.family, self.hybrid, self.blend);
            }
            // Nothing on this plan is an age or a garment (#53, #358).
            Category::Age | Category::Outfit => {}
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        // Version-4 bytes span the exploration envelope; see
        // [`super::humanoid::HumanoidParams::encode`] (#160).
        put_length(out, self.height);
        put_span(out, self.body_length, signed_envelope());
        put_span(out, self.leg_length, signed_envelope());
        put_span(out, self.neck_length, signed_envelope());
        put_span(out, self.head_size, signed_envelope());
        put_span(out, self.tail_length, signed_envelope());
        // Version 10 (#366, #178): `build` and `muscle` gone from between the
        // two above, and the family, the cross and the six new axes after.
        out.push(self.family.code());
        out.push(self.hybrid.map_or(NO_HYBRID, Family::code));
        put_span(out, self.blend, BLEND_RANGE);
        put_span(out, self.muzzle_length, signed_envelope());
        put_span(out, self.stance, signed_envelope());
        put_span(out, self.neck_carriage, signed_envelope());
        put_span(out, self.tail_carriage, signed_envelope());
        put_span(out, self.chest_depth, signed_envelope());
        put_span(out, self.appeal, signed_envelope());
        // And the head's (#367), folded into version 10 while it is
        // unpublished.
        put_span(out, self.muzzle_breadth, signed_envelope());
        put_span(out, self.stop, signed_envelope());
        put_span(out, self.eye_set, signed_envelope());
        out.push(self.pupil.code());
        put_span(out, self.ear_size, signed_envelope());
        put_span(out, self.ear_point, signed_envelope());
        put_span(out, self.ear_carriage, signed_envelope());
        out.push(self.nose.code());
        out.push(self.feet.code());
        put_span(out, self.whiskers, signed_envelope());
        out.push(self.ornaments.horn_byte());
        self.coat.encode(out);
        self.fur.encode(out);
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            body_length: take_span(bytes, signed_envelope())?,
            leg_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            tail_length: take_span(bytes, signed_envelope())?,
            ..Self::default()
        };
        params.family = Family::from_code(take_byte(bytes)?);
        let hybrid = take_byte(bytes)?;
        params.hybrid = (hybrid != NO_HYBRID).then(|| Family::from_code(hybrid));
        params.blend = take_span(bytes, BLEND_RANGE)?;
        params.muzzle_length = take_span(bytes, signed_envelope())?;
        params.stance = take_span(bytes, signed_envelope())?;
        params.neck_carriage = take_span(bytes, signed_envelope())?;
        params.tail_carriage = take_span(bytes, signed_envelope())?;
        params.chest_depth = take_span(bytes, signed_envelope())?;
        params.appeal = take_span(bytes, signed_envelope())?;
        params.muzzle_breadth = take_span(bytes, signed_envelope())?;
        params.stop = take_span(bytes, signed_envelope())?;
        params.eye_set = take_span(bytes, signed_envelope())?;
        params.pupil = Pupil::from_code(take_byte(bytes)?);
        params.ear_size = take_span(bytes, signed_envelope())?;
        params.ear_point = take_span(bytes, signed_envelope())?;
        params.ear_carriage = take_span(bytes, signed_envelope())?;
        params.nose = Nose::from_code(take_byte(bytes)?);
        params.feet = Feet::from_code(take_byte(bytes)?);
        params.whiskers = take_span(bytes, signed_envelope())?;
        params.ornaments = Ornaments::from_horn_byte(take_byte(bytes)?);
        params.coat = Coat::decode(bytes)?;
        params.fur = Fur::decode(bytes)?;
        params.sanitize();
        Ok(params)
    }

    fn decode_pre_families(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // The version-9 payload: the eight axes alone. A code minted before
        // the families meant the plan's neutral dog with its axes, which is
        // what the defaults of everything version 10 appends say (#366), and
        // its `build` and `muscle` are taken off the payload and dropped, as
        // the humanoid's were at version 6 (#178).
        let mut params = Self::decode_axes(bytes)?;
        params.sanitize();
        Ok(params)
    }

    fn decode_pre_distribution(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Version 9's payload, one version on again: the two axes version 9
        // appends say where a HUMANOID chest's volume sits, across and up. Same
        // payload since version 4 (#289).
        Self::decode_pre_families(bytes)
    }

    fn decode_pre_chest(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Version 9's payload, for `decode_reserved`'s reason one version on:
        // the three axes version 8 appends are the HUMANOID's chest, and this
        // plan's trunk is not carved. Its payload has not moved since version 4.
        Self::decode_pre_families(bytes)
    }

    fn decode_reserved(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // Version 9's payload, and that is the fact worth writing down: the
        // slots version 6 removes are the HUMANOID's retired `build` and
        // `muscle`. This plan's two axes of the same name stayed on its
        // payload until version 10 retired them in turn (#178), so its bytes
        // did not move from version 4 to 9 (#169).
        Self::decode_pre_families(bytes)
    }

    fn decode_legacy(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        // The version-3 byte layout, kept exactly as it was (#160), its
        // `build` and `muscle` consumed and dropped (#178).
        let height = take_length(bytes)?;
        let body_length = take_signed(bytes)?;
        let _retired_build = take_signed(bytes)?;
        let _retired_muscle = take_unit(bytes)?;
        let mut params = Self {
            height,
            body_length,
            leg_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            tail_length: take_signed(bytes)?,
            ..Self::default()
        };
        params.sanitize();
        Ok(params)
    }
}

impl QuadrupedParams {
    /// [`Self::search_ease`]'s answer, remembered for the last few records
    /// asked on this thread ([`EASES`]): the search asks the cage, at ten
    /// times the skeleton's own cost, and a viewer's header or a game's slider
    /// asks for the same record's skeleton every frame it is open or held.
    fn ease(&self, composites: &Composites) -> Ease {
        let remembered = EASES.with_borrow(|eases| {
            eases
                .iter()
                .find(|(params, asked, _)| params == self && asked == composites)
                .map(|&(_, _, ease)| ease)
        });
        remembered.unwrap_or_else(|| {
            let ease = self.search_ease(composites);
            EASES.with_borrow_mut(|eases| {
                if eases.len() == EASES_KEPT {
                    eases.pop_front();
                }
                eases.push_back((*self, *composites, ease));
            });
            ease
        })
    }

    /// How far each axis [`Ease`] names is built toward what this record asks
    /// (#391): [`AS_ASKED`] for a body that meshes as asked, and for one whose
    /// family's own frame does not mesh either. See [`BodyPlan::skeleton`].
    fn search_ease(&self, composites: &Composites) -> Ease {
        #[cfg(test)]
        SEARCHED.with(|count| count.set(count.get() + 1));
        let meshes_at =
            |ease: &Ease| meshes(&self.assemble(&Dimensions::eased(self, composites, ease)));
        if meshes(&self.assemble(&Dimensions::of(self, composites)))
            || !meshes_at(&[0.0; EASED_AXES])
        {
            return AS_ASKED;
        }
        // Every axis together first: the largest common share that meshes.
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..EASE_HALVINGS {
            let middle = 0.5 * (low + high);
            if meshes_at(&[middle; EASED_AXES]) {
                low = middle;
            } else {
                high = middle;
            }
        }
        let mut ease = [low; EASED_AXES];
        // Then each axis back to what was asked where the body still meshes
        // with it: an axis that crowds no girdle goes back all the way.
        for axis in 0..EASED_AXES {
            let mut trial = ease;
            trial[axis] = 1.0;
            if meshes_at(&trial) {
                ease = trial;
            }
        }
        // And the ones that do, together again, as far as they will go now
        // that the rest are as asked.
        let eased = |share: f32| ease.map(|kept| if kept < 1.0 { share } else { kept });
        let mut high = 1.0f32;
        for _ in 0..EASE_HALVINGS {
            let middle = 0.5 * (low + high);
            if meshes_at(&eased(middle)) {
                low = middle;
            } else {
                high = middle;
            }
        }
        eased(low)
    }

    /// Hangs the nodes of one derived body on a skeleton.
    fn assemble(&self, d: &Dimensions) -> Skeleton {
        hang(d, self.anatomy(), self.feet, HIND_INSIDE, FORE_INSIDE, 0)
    }
}

/// Hangs the nodes of one derived body on a skeleton: the graph every plan on
/// the beast's graph class shares (#420), the quadruped's and the winged's.
/// `hind_inside` and `fore_inside` say how many of each leg's nodes, from the
/// body down, are markers inside it; a quadruped's are [`HIND_INSIDE`] and
/// [`FORE_INSIDE`], and a limb whose nodes are all markers meshes nothing.
/// `tail_inside` says the same of the tail from its root: none of a
/// quadruped's, all of a bird's, whose visible tail is the fan its tail part
/// draws (#421).
pub(crate) fn hang(
    d: &Dimensions,
    anatomy: Anatomy,
    feet: Feet,
    hind_inside: usize,
    fore_inside: usize,
    tail_inside: usize,
) -> Skeleton {
    let mut skeleton = Skeleton::new();
    skeleton.anatomy = anatomy;
    // The head the cage's head chain is shaped into, and what its feet
    // are shaped into (#367).
    skeleton.beast_head = Some(d.head);
    skeleton.beast_feet = Some(super::BeastFeet { kind: feet });
    // The trunk, pelvis forward (#375): six rings, each its own depth and
    // width. The pelvis is the root, as it always was.
    let zones = [
        Zone::Pelvis,
        Zone::Abdomen,
        Zone::Abdomen,
        Zone::Chest,
        Zone::Chest,
        Zone::Chest,
    ];
    let mut ring = None;
    let mut trunk = [0u32; TRUNK_NODES];
    for (index, (&(at, radius, section), zone)) in d.trunk.iter().zip(zones).enumerate() {
        let node = Node::new(at, radius).with_scale(section).in_zone(zone);
        let added = match ring {
            None => skeleton.add_node(node),
            Some(previous) => skeleton.extend_from(previous, node),
        };
        trunk[index] = added;
        ring = Some(added);
    }
    let (hips, withers) = (trunk[0], trunk[TRUNK_NODES - 1]);
    let neck = skeleton.extend_from(
        withers,
        Node::new(d.neck_at, d.neck_r)
            .with_scale(d.neck_section)
            .in_zone(Zone::Neck),
    );
    // The head is a chain out of the neck: the back of the skull, the
    // cranium, the muzzle, the nose. All of it is the head's zone,
    // because all of it is what a head is.
    let occiput = skeleton.extend_from(
        neck,
        Node::new(d.occiput_at, d.occiput_r).in_zone(Zone::Head),
    );
    let cranium = skeleton.extend_from(occiput, Node::new(d.head_at, d.head_r).in_zone(Zone::Head));
    let muzzle = skeleton.extend_from(
        cranium,
        Node::new(d.muzzle_at, d.muzzle_r).in_zone(Zone::Head),
    );
    skeleton.extend_from(muzzle, Node::new(d.nose_at, d.nose_r).in_zone(Zone::Head));

    let mut tail = hips;
    for (index, &(at, radius)) in d.tail.iter().enumerate() {
        let mut node = Node::new(at, radius).in_zone(Zone::Tail);
        if index < tail_inside {
            node = node.as_marker();
        }
        tail = skeleton.extend_from(tail, node);
    }

    // Left is `+X`, as it is on every plan - see `plan::humanoid` for the
    // convention and for why the `−X` side is built first. This body faces
    // `+Z` too: its head sits ahead of its tail.
    for (side, fore, hind) in [
        (-1.0f32, Limb::ForeRight, Limb::HindRight),
        (1.0, Limb::ForeLeft, Limb::HindLeft),
    ] {
        // [`Dimensions`] gives every leg on the `+X` side; this is the
        // mirror, and it is the only place a side is decided.
        let at = |node: Vec3| Vec3::new(side * node.x, node.y, node.z);
        // Zoned as the humanoid's limbs are (#366): two nodes in the upper
        // limb, the ankle - the hock, the carpus - in the lower, the foot
        // in the extremity. `Rig::limb_chain` then hands the leg IK the
        // same three joints it gets on a person, and the pad, the first
        // extremity node, is the contact the gait plants.
        // Zoned as the humanoid's limbs are (#366): the upper limb, the
        // ankle - the hock, the carpus - in the lower, the foot in the
        // extremity. The bones inside the trunk are upper-limb MARKERS
        // (#374): the rig keeps them, the cage sweeps the leg straight
        // from the girdle to the first joint outside the body. Then
        // `Rig::limb_chain`, which takes the last three of the upper and
        // lower limb, hands the leg IK the shoulder, the elbow and the
        // carpus - or the hip, the stifle and the hock - and the pad, the
        // first extremity node, is the contact the gait plants.
        let zone = |limb: Limb, index: usize, count: usize| match count - index {
            1 | 2 => Zone::Extremity(limb),
            3 => Zone::LowerLimb(limb),
            _ => Zone::UpperLimb(limb),
        };
        let legs: [(u32, Limb, &[LegNode], usize); 2] = [
            (hips, hind, &d.hind, hind_inside),
            (withers, fore, &d.fore, fore_inside),
        ];
        for (girdle, limb, nodes, inside) in legs {
            let mut parent = girdle;
            for (index, &(position, radius)) in nodes.iter().enumerate() {
                let zone = zone(limb, index, nodes.len());
                let mut node = Node::new(at(position), radius).in_zone(zone);
                if index < inside {
                    node = node.as_marker();
                }
                // The thigh's muscle rides the stifle's ring, the first
                // the cage sweeps below the croup (#374).
                if index == inside && limb == hind {
                    node = node.with_scale(d.stifle_section);
                }
                // The foot's two nodes stand on a flat edge rather than a
                // vertex, for the reason `plan::humanoid` gives its sole.
                if matches!(zone, Zone::Extremity(_)) {
                    node = node.with_roll(HALF_SEGMENT);
                }
                parent = skeleton.extend_from(parent, node);
            }
        }
    }

    skeleton
}

impl QuadrupedParams {
    /// The eight axes every payload from version 4 to 9 carries, the rest at
    /// their defaults. The retired `build` and `muscle` are CONSUMED and
    /// dropped rather than skipped: a payload is a byte stream, and every axis
    /// after them reads at the wrong offset if they are not taken off it
    /// (#164's lesson, #178).
    fn decode_axes(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let height = take_length(bytes)?;
        let body_length = take_span(bytes, signed_envelope())?;
        let _retired_build = take_span(bytes, signed_envelope())?;
        let _retired_muscle = take_span(bytes, muscle_envelope())?;
        Ok(Self {
            height,
            body_length,
            leg_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            tail_length: take_span(bytes, signed_envelope())?,
            ..Self::default()
        })
    }

    /// Draws which family this quadruped is, and whether it is a cross of two
    /// (#366). Its own streams, drawn before any axis - by
    /// [`crate::AvatarRecord::reroll`], under the frame's lock - so every axis
    /// is centred on the result.
    pub(crate) fn reroll_family(&mut self, rolls: &Rolls) {
        self.put_lineage(Lineage::rolled(rolls, &STREAMS));
    }
}

/// One raw byte off a share code.
fn take_byte(bytes: &mut &[u8]) -> Result<u8, PlanDecodeError> {
    let (&byte, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
    *bytes = rest;
    Ok(byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::derive::quadruped::SKIN;
    use crate::plan::kin::HYBRID_SHARE;
    use crate::skeleton::NodeKind;

    #[test]
    fn the_neutral_body_has_two_four_way_girdles() {
        // #375: the trunk is a run of rings from the pelvis to the forechest,
        // and only its two ends are joints. Each girdle still carries four
        // bones; a leg's first is the marker its bridge starts with (#374).
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        skeleton.validate().expect("valid skeleton");
        let withers = TRUNK_NODES as u32 - 1;
        assert_eq!(skeleton.degree(0), 4, "pelvis: loin, tail, two hips");
        assert_eq!(
            skeleton.degree(withers),
            4,
            "forechest: ribs, neck, two blades"
        );
        assert_eq!(skeleton.kind(0), NodeKind::Joint);
        assert_eq!(skeleton.kind(withers), NodeKind::Joint);
        for ring in 1..withers {
            assert_eq!(
                skeleton.kind(ring),
                NodeKind::Connector,
                "ring {ring} of the trunk is not one of a run"
            );
        }

        // Head, tail tip, four feet: the markers inside the legs mesh
        // nothing, and are no part of the cage's count.
        let leaves = (0..skeleton.nodes.len() as u32)
            .filter(|&node| {
                !skeleton.nodes[node as usize].marker && skeleton.kind(node) == NodeKind::Leaf
            })
            .count();
        assert_eq!(leaves, 6);
    }

    #[test]
    fn legginess_slims_the_body() {
        let leggy = QuadrupedParams {
            leg_length: 1.0,
            ..Default::default()
        };
        let squat = QuadrupedParams {
            leg_length: -1.0,
            ..Default::default()
        };
        assert!(
            leggy.skeleton(&crate::Composites::default()).nodes[1].radius
                < squat.skeleton(&crate::Composites::default()).nodes[1].radius,
            "a leggy animal is a slender one at the same back height"
        );
    }

    #[test]
    fn a_short_heavy_body_stretches_instead_of_folding() {
        let params = QuadrupedParams {
            body_length: -1.0,
            leg_length: -1.0,
            ..Default::default()
        };
        let heavy = Composites {
            mass: 1.0,
            body_fat: 0.35,
            ..Default::default()
        };
        // #374: the run from the pelvis to the forechest is floored at a
        // multiple of the trunk's largest ring, where P1 floored one spine
        // bone against its girdle: a short deep body stretches rather than
        // crowd its ribs out of their girdles' sockets, and still meshes.
        let skeleton = params.skeleton(&heavy);
        let withers = TRUNK_NODES - 1;
        let run = skeleton.nodes[withers].position.z - skeleton.nodes[0].position.z;
        let largest = skeleton.nodes[..TRUNK_NODES]
            .iter()
            .map(|node| node.radius)
            .fold(0.0, f32::max);
        let floor = crate::plan::derive::quadruped::SPAN_FLOOR;
        assert!(
            run >= floor * largest * 0.999,
            "the trunk runs {run} under its floor {}",
            floor * largest
        );
        let cage = crate::cage::build_cage(&skeleton, &crate::CageConfig::default())
            .expect("the stretched body meshes");
        assert!(cage.manifold_report().is_clean());
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = QuadrupedParams {
            height: -5.0,
            tail_length: f32::INFINITY,
            stance: 9.0,
            ..Default::default()
        };
        params.sanitize();
        // Bounds are the exploration envelope (#160).
        assert_eq!(params.height, height_envelope().0);
        assert_eq!(params.tail_length, 0.0);
        assert_eq!(params.stance, 3.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }
    /// One seed's kind and every axis, drawn as a record's re-roll draws them.
    fn rolled(seed: i64) -> QuadrupedParams {
        let rolls = Rolls::new(seed);
        let mut params = QuadrupedParams::default();
        params.reroll_family(&rolls);
        for category in Category::ALL {
            params.reroll(category, &rolls, &Composites::default());
        }
        params.sanitize();
        params
    }

    #[test]
    fn the_families_roll_apart_and_about_evenly() {
        const SEEDS: usize = 3000;
        let mut count = [0usize; 3];
        let mut sums = [[0.0f32; 3]; 3];
        let mut crosses = 0;
        for seed in 0..SEEDS as i64 {
            let params = rolled(seed);
            if params.hybrid.is_some() {
                crosses += 1;
                continue;
            }
            let at = params.family.code() as usize;
            count[at] += 1;
            for (sum, axis) in
                sums[at]
                    .iter_mut()
                    .zip([params.height, params.muzzle_length, params.stance])
            {
                *sum += axis;
            }
        }
        let share = crosses as f32 / SEEDS as f32;
        assert!(
            (share - HYBRID_SHARE).abs() < 0.02,
            "{share} of rolls are crosses"
        );
        for n in count {
            let even = n as f32 / (SEEDS - crosses) as f32;
            assert!((even - 1.0 / 3.0).abs() < 0.03, "families drawn {count:?}");
        }
        let mean = |family: Family, axis: usize| {
            let at = family.code() as usize;
            sums[at][axis] / count[at] as f32
        };
        // A cat is the smallest with the shortest face, a horse the tallest
        // with the longest, and only a horse stands on its hooves.
        for axis in [0, 1] {
            assert!(
                mean(Family::Cat, axis) < mean(Family::Dog, axis)
                    && mean(Family::Dog, axis) < mean(Family::Horse, axis),
                "axis {axis}: {:?}",
                Family::ALL.map(|family| mean(family, axis))
            );
        }
        assert!(mean(Family::Horse, 2) > 0.8);
        assert!(mean(Family::Dog, 2).abs() < 0.2 && mean(Family::Cat, 2).abs() < 0.2);
    }

    #[test]
    fn a_cross_is_rolled_between_its_two_families() {
        let cross = QuadrupedParams {
            family: Family::Cat,
            hybrid: Some(Family::Horse),
            blend: 0.5,
            ..Default::default()
        };
        let (cat, horse) = (Anchor::of(Family::Cat), Anchor::of(Family::Horse));
        let between = Anchor::for_params(&cross);
        assert!((between.height.mean - (cat.height.mean + horse.height.mean) / 2.0).abs() < 1e-6);
        assert_eq!(
            between.height.sigma,
            cat.height.sigma.max(horse.height.sigma)
        );
        for seed in 0..3000 {
            let params = rolled(seed);
            match params.hybrid {
                Some(other) => {
                    assert_ne!(other, params.family, "seed {seed} is crossed with itself");
                    assert!((0.25..=0.75).contains(&params.blend), "seed {seed}");
                }
                None => assert_eq!(params.blend, 0.0, "seed {seed}"),
            }
        }
    }

    #[test]
    fn a_cross_has_one_canonical_form() {
        let sanitized = |params: QuadrupedParams| {
            let mut params = params;
            params.sanitize();
            (params.hybrid, params.blend)
        };
        let horse = QuadrupedParams {
            family: Family::Horse,
            ..Default::default()
        };
        // A cross with itself is no cross, and no cross is blended.
        assert_eq!(
            sanitized(QuadrupedParams {
                hybrid: Some(Family::Horse),
                blend: 0.6,
                ..horse
            }),
            (None, 0.0)
        );
        assert_eq!(
            sanitized(QuadrupedParams {
                blend: 0.6,
                ..horse
            }),
            (None, 0.0)
        );
        // A cross's blend is a share, clamped and never NaN.
        let cross = QuadrupedParams {
            hybrid: Some(Family::Cat),
            ..horse
        };
        assert_eq!(
            sanitized(QuadrupedParams {
                blend: 7.0,
                ..cross
            }),
            (Some(Family::Cat), 1.0)
        );
        assert_eq!(
            sanitized(QuadrupedParams {
                blend: f32::NAN,
                ..cross
            }),
            (Some(Family::Cat), 0.5)
        );
    }

    #[test]
    fn a_family_this_build_does_not_know_reads_as_a_dog_and_keeps_its_body() {
        let params: QuadrupedParams = serde_json::from_str(
            r#"{"height":900,"family":"rabbit","hybrid":"horse","blend":400,"legLength":-500}"#,
        )
        .expect("parses");
        assert_eq!(params.family, Family::Dog);
        assert_eq!(params.hybrid, Some(Family::Horse));
        assert!(
            (params.leg_length + 0.5).abs() < 1e-6,
            "the body comes through whole"
        );
        assert_eq!(Family::from_code(9), Family::Dog);
        for family in Family::ALL {
            assert_eq!(Family::named(family.name()), family);
            assert_eq!(Family::from_code(family.code()), family);
            let json = serde_json::to_string(&family).expect("serialises");
            assert_eq!(json, format!("\"{}\"", family.name()));
        }
    }

    #[test]
    fn a_family_s_pupils_come_with_its_rolls() {
        let rolled = |seed: i64| {
            let mut record = crate::AvatarRecord::new(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
            );
            record.reroll(seed);
            match record.archetype {
                crate::Archetype::Quadruped(params) => params,
                _ => unreachable!("a quadruped rolls a quadruped"),
            }
        };
        let mut crosses = 0;
        for seed in 0..300 {
            let params = rolled(seed);
            match params.hybrid {
                None => assert_eq!(params.pupil, Pupil::of(params.family), "seed {seed}"),
                Some(other) => {
                    crosses += 1;
                    assert!(
                        [Pupil::of(params.family), Pupil::of(other)].contains(&params.pupil),
                        "seed {seed}: a cross took a pupil neither parent has"
                    );
                }
            }
        }
        assert!(crosses > 10, "only {crosses} crosses in 300 rolls");
    }

    #[test]
    fn a_family_change_hands_the_pupils_over_and_a_cross_keeps_either() {
        let mut params = QuadrupedParams::default();
        params.set_kind(Family::Cat, None, 0.0);
        assert_eq!(params.pupil, Pupil::Slit);
        // A cat crossed with a dog keeps the slit however the blend is
        // dragged, since the slit is one of its families'.
        for blend in [0.2, 0.5, 0.8] {
            params.set_kind(Family::Cat, Some(Family::Dog), blend);
            assert_eq!(params.pupil, Pupil::Slit, "at a blend of {blend}");
        }
        // A cross the slit belongs to neither side of takes the side it
        // leans to.
        params.set_kind(Family::Dog, Some(Family::Horse), 0.7);
        assert_eq!(params.pupil, Pupil::Bar);
        // The nose goes the same way: a cat's leather is a dog's too, so the
        // dog-horse cross kept it; a horse alone has a soft muzzle.
        assert_eq!(params.nose, Nose::Leather);
        params.set_kind(Family::Horse, None, 0.0);
        assert_eq!(params.nose, Nose::Soft);
        for nose in Nose::ALL {
            assert_eq!(Nose::named(nose.name()), nose);
            assert_eq!(Nose::from_code(nose.code()), nose);
        }
        for pupil in Pupil::ALL {
            assert_eq!(Pupil::named(pupil.name()), pupil);
            assert_eq!(Pupil::from_code(pupil.code()), pupil);
        }
        assert_eq!(Pupil::named("star"), Pupil::Round);
        assert_eq!(Pupil::from_code(200), Pupil::Round);
    }

    #[test]
    fn a_family_change_carries_the_body_s_character_across() {
        // A dog a spread taller and a spread leggier than its family's centre
        // becomes a cat as much taller and leggier than a cat's - and back.
        let dog = Anchor::of(Family::Dog);
        let mut params = QuadrupedParams {
            height: dog.height.mean + dog.height.sigma,
            leg_length: dog.leg_length.mean + dog.leg_length.sigma,
            ..Default::default()
        };
        let before = params;
        params.set_kind(Family::Cat, None, 0.0);
        let cat = Anchor::of(Family::Cat);
        assert_eq!(params.family, Family::Cat);
        assert!((params.height - (cat.height.mean + cat.height.sigma)).abs() < 1e-5);
        assert!((params.leg_length - (cat.leg_length.mean + cat.leg_length.sigma)).abs() < 1e-5);
        params.set_kind(Family::Dog, None, 0.0);
        assert!((params.height - before.height).abs() < 1e-5);
        assert!((params.leg_length - before.leg_length).abs() < 1e-5);
    }
    /// The skeleton of the default quadruped under `composites`.
    fn built(composites: Composites) -> Skeleton {
        QuadrupedParams::default().skeleton(&composites)
    }

    /// The radius and the cross-section of trunk node `index`.
    fn skeleton_trunk(skeleton: &Skeleton, index: usize) -> (f32, glam::Vec2) {
        let node = skeleton.nodes[index];
        (node.radius, node.scale)
    }

    /// The radius of the first node in `zone`.
    fn radius(skeleton: &Skeleton, zone: Zone) -> f32 {
        skeleton
            .nodes
            .iter()
            .find(|node| node.zone == zone)
            .map(|node| node.radius)
            .expect("the zone is built")
    }

    #[test]
    fn the_composites_shape_a_quadruped_where_its_sites_say() {
        // #178: mass reaches every girth and the fleshy sites fastest, body
        // fat fills the belly and leaves the cannon alone, and age wastes the
        // legs and settles the back. The frame axis reaches nothing.
        // On the trunk (#374, #375) a heavier body is wider at every ring and
        // deeper through the barrel under the same back, and a fatter one
        // most at the belly, whose underline drops - on a dog and a cat, at
        // a naturalistic `appeal` and at the roll's centre, where a puppy's
        // or a kitten's ribcage is already as round as it may be. The walls
        // the rings are held by (round, the loin's width) hold the shape and
        // never swallow the amount of body.
        for family in [Family::Dog, Family::Cat] {
            for appeal in [0.0, 1.0] {
                let mut params = QuadrupedParams::default();
                params.set_kind(family, None, 0.0);
                params.appeal = appeal;
                let at = |mass: f32, body_fat: f32| {
                    Dimensions::of(
                        &params,
                        &Composites {
                            mass,
                            body_fat,
                            ..Default::default()
                        },
                    )
                };
                // A ring's half-extents as the skin should measure them.
                let width = |d: &Dimensions, i: usize| d.trunk[i].1 * d.trunk[i].2.x * SKIN;
                let depth = |d: &Dimensions, i: usize| d.trunk[i].1 * d.trunk[i].2.y * SKIN;
                let underline = |d: &Dimensions, i: usize| d.trunk[i].0.y - depth(d, i);
                let back = |d: &Dimensions, i: usize| d.trunk[i].0.y + depth(d, i);
                let what = format!("{family:?} at appeal {appeal}");
                let (neutral, heavy, slight) = (
                    at(0.0, DEFAULT_BODY_FAT),
                    at(1.0, DEFAULT_BODY_FAT),
                    at(-1.0, DEFAULT_BODY_FAT),
                );
                for i in 0..TRUNK_NODES {
                    assert!(
                        width(&heavy, i) > width(&slight, i) * 1.15,
                        "{what}: mass widens ring {i} ({} against {})",
                        width(&heavy, i),
                        width(&slight, i)
                    );
                }
                for i in 1..=4 {
                    assert!(
                        depth(&heavy, i) > depth(&slight, i) * 1.05,
                        "{what}: mass deepens ring {i} ({} against {})",
                        depth(&heavy, i),
                        depth(&slight, i)
                    );
                    assert!(
                        underline(&heavy, i) < underline(&neutral, i),
                        "{what}: under the same back, so ring {i}'s underline drops"
                    );
                    assert!(
                        (back(&neutral, i) - back(&heavy, i)).abs() < 1e-5,
                        "{what}: ring {i}'s back stays where it was"
                    );
                }
                let ratio = |of: &dyn Fn(&Dimensions, usize) -> f32, d: &Dimensions, i: usize| {
                    of(d, i) / of(&neutral, i)
                };
                assert!(
                    ratio(&width, &heavy, 2) > ratio(&width, &heavy, 3) * 1.03,
                    "{what}: the belly gains faster than the ribcage"
                );
                let fat = at(0.0, 0.45);
                assert!(
                    ratio(&width, &fat, 1) > 1.05 && ratio(&width, &fat, 2) > 1.05,
                    "{what}: a fat animal is round"
                );
                assert!(
                    underline(&neutral, 2) - underline(&fat, 2) > 0.015 * params.height
                        && underline(&fat, 1) < underline(&neutral, 1)
                        && ratio(&depth, &fat, 2) > ratio(&depth, &fat, 3),
                    "{what}: and carries it at the belly, whose underline drops"
                );
            }
        }

        let neutral = built(Composites::default());
        let fat = built(Composites {
            body_fat: 0.45,
            ..Default::default()
        });
        let hock = Zone::LowerLimb(Limb::HindLeft);
        assert_eq!(
            radius(&fat, hock),
            radius(&neutral, hock),
            "a cannon is bone"
        );

        let old = built(Composites {
            age: 80,
            ..Default::default()
        });
        let stifle = Zone::UpperLimb(Limb::HindLeft);
        let thigh = |skeleton: &Skeleton| {
            skeleton
                .nodes
                .iter()
                .filter(|node| node.zone == stifle)
                .map(|node| node.radius)
                .nth(1)
                .expect("the stifle is built")
        };
        assert!(thigh(&old) < thigh(&neutral), "age wastes the legs");
        let loin = |skeleton: &Skeleton| skeleton.nodes[1].position.y;
        assert!(loin(&old) < loin(&neutral), "and settles the back");
        assert_eq!(
            skeleton_trunk(&old, 0),
            skeleton_trunk(&neutral, 0),
            "and leaves the pelvis as it was"
        );

        for femininity in [-3.0, 3.0] {
            let framed = built(Composites {
                femininity,
                ..Default::default()
            });
            assert_eq!(framed, neutral, "femininity {femininity} reached a beast");
        }
    }

    #[test]
    fn a_family_centres_the_composites_too_and_draws_no_frame() {
        // A horse is heavier than a dog and a dog than a cat before anything
        // else is said about them, and a quadruped draws no frame axis: its
        // stored value is kept, as a beast keeps a face it never builds.
        let mut mass = [0.0f32; 3];
        let mut count = [0usize; 3];
        for seed in 0..600 {
            let mut record = crate::AvatarRecord::new(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
            );
            record.composites.femininity = 0.7;
            record.reroll(seed);
            assert_eq!(
                record.composites.femininity, 0.7,
                "seed {seed} drew a frame"
            );
            let crate::Archetype::Quadruped(params) = record.archetype else {
                panic!("archetype changed");
            };
            if params.hybrid.is_none() {
                let at = params.family.code() as usize;
                mass[at] += record.composites.mass;
                count[at] += 1;
            }
        }
        let mean =
            |family: Family| mass[family.code() as usize] / count[family.code() as usize] as f32;
        assert!(
            mean(Family::Cat) < mean(Family::Dog) && mean(Family::Dog) < mean(Family::Horse),
            "{:?}",
            Family::ALL.map(mean)
        );
        let person = crate::AvatarRecord::rolled("Person", crate::Archetype::default(), 3);
        assert_ne!(
            person.composites.femininity, 0.0,
            "a person still draws one"
        );
    }
    #[test]
    fn appeal_draws_the_baby_schema_over_any_family() {
        // #366, and #372's owner calls: a bigger head and a shorter face on
        // every family, and the body each family's young has. A puppy's or a
        // kitten's barrel grows round rather than long, under a shorter,
        // thicker neck, on legs at the adult's share (S7: head 0.61 against
        // 0.39, girth 1.58 against 1.15, legs 0.61 both). A foal's body is
        // short on long legs and no rounder. A cute horse is still a horse.
        let at = |appeal: f32, family: Family, hybrid: Option<Family>| {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, hybrid, 0.5);
            params.appeal = appeal;
            Dimensions::of(&params, &Composites::default())
        };
        // The heart girth's ring, as an ellipse's girth.
        let girth = |d: &Dimensions| {
            let (_, radius, section) = d.trunk[TRUNK_NODES - 2];
            let (a, b) = (radius * section.x, radius * section.y);
            std::f32::consts::PI * (3.0 * (a + b) - ((3.0 * a + b) * (a + 3.0 * b)).sqrt())
        };
        let length = |d: &Dimensions| d.trunk[TRUNK_NODES - 1].0.z - d.trunk[0].0.z;
        // The elbow over the pad's centre: the leg, without the bigger paw
        // a puppy stands on.
        let elbow = |d: &Dimensions| d.fore[2].0.y - d.fore[4].0.y;
        let neck = |d: &Dimensions| d.neck_at.distance(d.trunk[TRUNK_NODES - 1].0);
        let face = |d: &Dimensions| d.nose_at.distance(d.head_at) / d.head_r;
        let grew = |of: &dyn Fn(&Dimensions) -> f32, plain: &Dimensions, cute: &Dimensions| {
            of(cute) / of(plain)
        };
        for family in Family::ALL {
            let (plain, cute) = (at(0.0, family, None), at(1.5, family, None));
            assert!(
                cute.head_r > plain.head_r * 1.35,
                "{family:?}: the head grows"
            );
            assert!(face(&cute) < face(&plain), "{family:?}: the face shortens");
            let (round, long, legs) = (
                grew(&girth, &plain, &cute),
                grew(&length, &plain, &cute),
                grew(&elbow, &plain, &cute),
            );
            match family {
                Family::Dog | Family::Cat => {
                    assert!(round > 1.2, "{family:?}: the barrel rounds ({round})");
                    assert!(
                        long < 1.15 && long - 1.0 < (round - 1.0) * 0.5,
                        "{family:?}: rounder rather than longer ({long} against {round})"
                    );
                    assert!(
                        (legs - 1.0).abs() < 0.01,
                        "{family:?}: the legs keep their share ({legs})"
                    );
                    assert!(
                        neck(&cute) < neck(&plain) && cute.neck_r > plain.neck_r * 1.2,
                        "{family:?}: a shorter, thicker neck"
                    );
                    assert!(
                        cute.hind[2].1 > plain.hind[2].1 * 1.1,
                        "{family:?}: the hock thickens"
                    );
                }
                Family::Horse => {
                    assert!(long < 0.95, "a foal's body is short ({long})");
                    assert!(legs > 1.1, "a foal stands on long legs ({legs})");
                    let arm = |d: &Dimensions| d.fore[1].0.distance(d.fore[2].0);
                    assert!(
                        arm(&cute) < arm(&plain) * 0.9,
                        "its long legs are its cannons, under a short upper arm"
                    );
                    assert!(round <= 1.0, "and no rounder ({round})");
                }
            }
        }
        // A cross is drawn by both its families' rules, by its blend.
        let (plain, cute) = (
            at(0.0, Family::Dog, Some(Family::Horse)),
            at(1.5, Family::Dog, Some(Family::Horse)),
        );
        let legs = grew(&elbow, &plain, &cute);
        let foal = grew(
            &elbow,
            &at(0.0, Family::Horse, None),
            &at(1.5, Family::Horse, None),
        );
        assert!(
            legs > 1.01 && legs < foal,
            "a dog-horse's legs lengthen by part of a foal's ({legs} against {foal})"
        );
        let face = |d: &Dimensions| d.nose_at.distance(d.head_at) / d.head_r;
        assert!(
            face(&at(1.0, Family::Horse, None)) > face(&at(1.0, Family::Dog, None))
                && face(&at(1.0, Family::Dog, None)) > face(&at(1.0, Family::Cat, None)),
            "the families keep their faces' order"
        );
    }
    #[test]
    fn a_thigh_is_deeper_from_the_side_than_across() {
        // #374: the thigh's muscle rides the stifle's ring, drawn deeper fore
        // and aft than across - but a leg's ring frame was transported from
        // world up, and on a leg dropping from the midline with a slight lean
        // out and forward that turned its first axis forty-five degrees
        // round: a thigh drawn deep from the side came out deep on the
        // diagonal. A bridged leg's frame starts across the body.
        for seed in 0..40 {
            let mut record = crate::AvatarRecord::new(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
            );
            record.reroll(seed);
            let skeleton = record.skeleton();
            let cage = crate::cage::build_cage(&skeleton, &crate::CageConfig::default())
                .expect("the roll meshes");
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let stifle = skeleton
                    .nodes
                    .iter()
                    .find(|node| node.zone == Zone::UpperLimb(limb) && !node.marker)
                    .expect("a stifle");
                // The stifle's own ring: the cage's corners in the plane
                // square to the leg there, about as far out as its section.
                let index = skeleton
                    .nodes
                    .iter()
                    .position(|node| std::ptr::eq(node, stifle))
                    .expect("a node") as u32;
                let [into, out] =
                    [skeleton.neighbors(index)[0], skeleton.neighbors(index)[1]].map(|other| {
                        (stifle.position - skeleton.nodes[other as usize].position).normalize()
                    });
                let along = (into - out).normalize();
                let reach = stifle.radius * stifle.scale.max_element();
                let ring: Vec<Vec3> = cage
                    .positions
                    .iter()
                    .map(|&p| p - stifle.position)
                    .filter(|offset| {
                        offset.length() < reach * 1.05 && offset.dot(along).abs() < 0.3 * reach
                    })
                    .collect();
                assert!(ring.len() >= 6, "seed {seed}: no ring at the stifle");
                let extent = |of: fn(Vec3) -> f32| {
                    ring.iter().map(|&p| of(p)).fold(f32::MIN, f32::max)
                        - ring.iter().map(|&p| of(p)).fold(f32::MAX, f32::min)
                };
                let (deep, wide) = (extent(|p| p.z), extent(|p| p.x));
                assert!(
                    deep > wide * 1.15,
                    "seed {seed} {limb:?}: the thigh is {deep} deep and {wide} wide"
                );
            }
        }
    }

    #[test]
    fn a_thigh_s_muscle_does_not_crowd_the_croup() {
        // #374: a bridged bone's socket is sized by the joint it opens toward,
        // the hip, not by the stifle it sweeps to; blended from a thick
        // thigh's stifle ring, the thighs' sockets on the pelvis crowded the
        // spine's off its bone on 57 rolls in 400.
        for seed in 0..400 {
            let mut record = crate::AvatarRecord::new(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
            );
            record.reroll(seed);
            if let Err(error) =
                crate::cage::build_cage(&record.skeleton(), &crate::CageConfig::default())
            {
                panic!("seed {seed} does not mesh: {error}");
            }
        }
    }

    #[test]
    fn a_record_asked_again_is_not_searched_again() {
        // #391: the search for an ease asks the cage, at ten times the
        // skeleton's cost, and an open editor asks for the same record's
        // skeleton every frame. A body that refuses as asked is the dear
        // one: the slight, fat dog below, about twenty cages a search.
        let mut params = QuadrupedParams {
            height: 2.0,
            neck_length: 3.0,
            neck_carriage: 3.0,
            stance: 3.0,
            chest_depth: 3.0,
            ..QuadrupedParams::default()
        };
        params.sanitize();
        let composites = Composites {
            mass: -3.0,
            body_fat: crate::plan::BODY_FAT_RANGE.1,
            ..Composites::default()
        };
        let first = params.skeleton(&composites);
        let searched = SEARCHED.with(std::cell::Cell::get);
        let again = params.skeleton(&composites);
        assert_eq!(
            SEARCHED.with(std::cell::Cell::get),
            searched,
            "the same record was searched again"
        );
        assert_eq!(first.nodes.len(), again.nodes.len());
        for (a, b) in first.nodes.iter().zip(&again.nodes) {
            assert_eq!(a.position, b.position);
            assert_eq!(a.radius, b.radius);
        }
        // And a record one step away is its own question.
        params.leg_length = f32::from_bits(params.leg_length.to_bits() + 1);
        let _ = params.skeleton(&composites);
        assert_eq!(SEARCHED.with(std::cell::Cell::get), searched + 1);
    }

    #[test]
    fn an_eased_body_keeps_the_head_its_record_asks_for() {
        // #391: a body its cage cannot hull is eased toward its family's on
        // the axes its girdles are built from, and on nothing else. The head
        // hangs off the eased neck, so it may stand elsewhere, but its nodes'
        // sizes and the runs between them are the record's: a slight, fat dog
        // whose forechest is deeper than the run back to its ribs, and whose
        // `appeal` and `mass` are among the axes eased - both of which also
        // grow a head.
        let mut params = QuadrupedParams {
            height: 2.0,
            neck_length: 3.0,
            neck_carriage: 3.0,
            stance: 3.0,
            chest_depth: 3.0,
            ..QuadrupedParams::default()
        };
        params.sanitize();
        let composites = Composites {
            mass: -3.0,
            body_fat: crate::plan::BODY_FAT_RANGE.1,
            ..Composites::default()
        };
        let ease = params.ease(&composites);
        assert!(
            ease[6] < 1.0 && ease[7] < 1.0,
            "appeal and mass eased: {ease:?}"
        );
        let asked = params.assemble(&Dimensions::of(&params, &composites));
        let built = params.skeleton(&composites);
        assert!(meshes(&built), "the body built meshes");
        assert!(!meshes(&asked), "the body as asked does not");
        let head = [7, 8, 9, 10];
        for node in head {
            assert_eq!(
                built.nodes[node].radius, asked.nodes[node].radius,
                "head node {node}"
            );
        }
        for pair in head.windows(2) {
            let run = |skeleton: &Skeleton| {
                skeleton.nodes[pair[1]].position - skeleton.nodes[pair[0]].position
            };
            assert!(
                run(&built).abs_diff_eq(run(&asked), 1e-5 * params.height),
                "the run from {} to {}: {} against {}",
                pair[0],
                pair[1],
                run(&built),
                run(&asked)
            );
        }
        // And a body that meshes as asked - every roll - is not eased at all.
        for seed in 0..200 {
            let record = crate::AvatarRecord::rolled(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            );
            let crate::Archetype::Quadruped(params) = &record.archetype else {
                unreachable!("a beast rolls a beast");
            };
            assert_eq!(params.ease(&record.composites), AS_ASKED, "seed {seed}");
            assert_eq!(
                params.skeleton(&record.composites),
                params.assemble(&Dimensions::of(params, &record.composites)),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn no_tail_reaches_the_ground() {
        // A long tail hanging straight down ran a tenth of the body's height
        // into the ground at three units long (the P1 critic), and one roll in
        // three thousand did. Its step is shortened to clear it instead.
        let clears = |params: &QuadrupedParams, composites: &Composites, what: &str| {
            let d = Dimensions::of(params, composites);
            for (at, radius) in d.tail {
                assert!(
                    at.y - radius > 0.0,
                    "{what}: a tail node at {} r {radius}",
                    at.y
                );
            }
        };
        for tail_length in [1.0, 2.0, 2.5, 3.0] {
            for tail_carriage in [-1.0, -0.9, -0.5] {
                for height in [0.25, 0.58, 1.8] {
                    let mut params = QuadrupedParams {
                        height,
                        tail_length,
                        tail_carriage,
                        ..QuadrupedParams::default()
                    };
                    params.sanitize();
                    clears(&params, &Composites::default(), &format!("{params:?}"));
                }
            }
        }
        for seed in 0..3000 {
            let mut record = crate::AvatarRecord::new(
                "Beast",
                crate::Archetype::Quadruped(QuadrupedParams::default()),
            );
            record.reroll(seed);
            let crate::Archetype::Quadruped(params) = record.archetype else {
                panic!("archetype changed");
            };
            clears(&params, &record.composites, &format!("seed {seed}"));
        }
    }
}
