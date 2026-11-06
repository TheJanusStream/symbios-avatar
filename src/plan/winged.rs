//! The winged body plan (#418, #420): a bird, a bat and a sugar glider, and
//! every cross between them.
//!
//! **The quadruped's graph, with its fore limb a wing arm.** Six trunk rings,
//! a neck, a head chain, a tail and four limbs, node for node the beast's
//! (`plan::quadruped::hang`), so the cage, the rig, the head, the feet and the
//! gait that already carry a beast carry this one. The fore limb is the arm a
//! wing hangs from: the shoulder blade and the shoulder inside the trunk, the
//! humerus, the forearm, the wrist and a placeholder hand. The digits that
//! will carry a membrane or a fan of feathers are the wing part's own (#421),
//! as a person's hand brings its own fingers (`extremity::hand`).
//!
//! **A family moves lengths and radii; the posture is the leading family's.**
//! Every bone and ring is a share of the trunk, blended between a cross's two
//! families by its blend (`derive::winged`), so a bird-bat cross blends as a
//! dog-cat one does. What cannot blend is how the body stands, and it is
//! taken from the family a cross leans toward ([`Lineage::leading`], the
//! owner's call of 2026-11-05): a bird perches on its hind pair, a bat crawls
//! on its wrists and its feet, a glider stands on all four. The legs the plan
//! declares follow it ([`BodyPlan::anatomy`]).
//!
//! docs/winged-anatomy.md has the targets, where they come from in
//! research-10 and every departure from the animal.

use std::cell::RefCell;
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::kin::{self, Anchored, Blended, Bred, Draw, Kin, Lineage, Streams};
use super::{Anatomy, CompositePriors, Composites, DEFAULT_BODY_FAT, FootKind, Legs};
use super::{BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_span, take_length};
use super::{take_byte, take_span};
use crate::skeleton::Skeleton;

/// Which kind of winged animal a body is drawn as (#420): the anchor its axes
/// are rolled around and the frame its body is built in.
///
/// **The body reads it**, as a quadruped's reads its [`super::Family`]: a
/// family is a frame of bone lengths and trunk sections in shares of the
/// trunk, and a cross is built in its two families' frames blended by its
/// blend. How it stands is its leading family's.
///
/// Written as its lowercase name. A name this build does not know - a family
/// a newer one added - reads as a bird rather than failing the record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WingedFamily {
    /// A bird: the plan's neutral, a small owl by default (the owner's call),
    /// its wing bones a crow's.
    #[default]
    Bird,
    /// A fox-faced fruit bat, in a flying fox's proportions.
    Bat,
    /// A sugar glider.
    Glider,
}

named!(WingedFamily: Bird => "bird", Bat => "bat", Glider => "glider");

/// How a winged body stands at rest: its leading family's (#420).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Posture {
    /// On its hind pair, the trunk raised: a bird's.
    Perch,
    /// On its wrists and its feet, low, the elbows up: a bat's.
    Crawl,
    /// On all four, as a small mammal stands: a glider's.
    Stand,
}

impl Posture {
    /// A family's own.
    pub(crate) fn of(family: WingedFamily) -> Self {
        match family {
            WingedFamily::Bird => Posture::Perch,
            WingedFamily::Bat => Posture::Crawl,
            WingedFamily::Glider => Posture::Stand,
        }
    }

    /// The limbs a body standing so bears its weight on.
    pub(crate) fn legs(self) -> Legs {
        match self {
            Posture::Perch => Legs::HIND,
            Posture::Crawl | Posture::Stand => Legs::ALL,
        }
    }
}

/// Smallest and largest head-and-body length the plan's conservative range
/// spans, in metres: a cat's to a dog's at a game's scale (the owner's call,
/// 2026-11-05), as the quadruped's cat and dog stand.
pub const SIZE_RANGE: (f32, f32) = (0.35, 1.2);

/// The byte a share code writes for "no hybrid".
const NO_HYBRID: u8 = 255;

/// Where a re-roll centres `appeal`: toward the young, as on a beast (#364).
/// A fledgling or an owlet, a pup, a joey (the owner's call, 2026-11-05).
const CUTE: f32 = 1.0;

/// Parameters describing one winged body.
///
/// Axes run `-1..=1` unless noted, with `0` the family's own, and every one is
/// read as a step from it: a cross of two families is built in their frames
/// blended, and its axes move that. A new field here is `serde(default)` like
/// every other, so an older build that knows the plan reads a newer record.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WingedParams {
    /// How big the body is, in metres: its head-and-body length, the bill or
    /// the nose to the root of the tail, as a field guide measures it, at its
    /// family's own proportions. Within [`super::winged_size_range`], explored past it.
    #[serde(with = "super::scaled")]
    pub size: f32,
    /// How long the wing's arm is: the humerus and the forearm, and the hand
    /// a wing part hangs from it (#421).
    #[serde(with = "super::scaled")]
    pub wing_length: f32,
    /// How long the neck is: an owl's hidden one at the bottom, a crow's at
    /// the top.
    #[serde(with = "super::scaled")]
    pub neck_length: f32,
    /// How long the hind legs are below the trunk.
    #[serde(with = "super::scaled")]
    pub leg_length: f32,
    /// How long the tail is: a bat's nub at the bottom of its family's
    /// range, a parrot's or a glider's long tail at the top.
    #[serde(with = "super::scaled")]
    pub tail_length: f32,
    /// How big the head is against the body.
    #[serde(with = "super::scaled")]
    pub head_size: f32,
    /// How the body is drawn, from naturalistic at the bottom to its family's
    /// young at the top: a bigger head and a shorter face, a rounder body, a
    /// shorter neck and tail; a pup's arm a little shorter. A re-roll leans
    /// it cute, the owner's call.
    #[serde(with = "super::scaled")]
    pub appeal: f32,
    /// Which kind of animal this is rolled as. See [`WingedFamily`].
    pub family: WingedFamily,
    /// The second family of a cross, if this is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<WingedFamily>,
    /// How far a cross is rolled toward its [`Self::hybrid`] family, `0..=1`.
    #[serde(with = "super::scaled")]
    pub blend: f32,
}

/// Neutral size, used when a record omits the field: the bird's.
fn default_size() -> f32 {
    0.60
}

/// The size envelope: [`SIZE_RANGE`] stretched about the default, floored at
/// five centimetres for the quadruped's reason.
fn size_envelope() -> (f32, f32) {
    let raw = super::explore_range(default_size(), SIZE_RANGE);
    (raw.0.max(0.05), raw.1)
}

/// The envelope every signed axis clamps and encodes against.
fn signed_envelope() -> (f32, f32) {
    super::explore_range(0.0, (-1.0, 1.0))
}

/// The `blend` range: a share.
const BLEND_RANGE: (f32, f32) = (0.0, 1.0);

/// How far a roll's size may stray from its family's, as factors on the
/// anchor's mean: the quadruped's band.
const SIZE_BAND: (f32, f32) = (0.5, 1.6);

impl Default for WingedParams {
    fn default() -> Self {
        Self {
            size: default_size(),
            wing_length: 0.0,
            neck_length: 0.0,
            leg_length: 0.0,
            tail_length: 0.0,
            head_size: 0.0,
            appeal: 0.0,
            family: WingedFamily::Bird,
            hybrid: None,
            blend: 0.0,
        }
    }
}

/// Where each family's rolls centre every axis.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anchor {
    size: Draw,
    wing_length: Draw,
    neck_length: Draw,
    leg_length: Draw,
    tail_length: Draw,
    head_size: Draw,
    appeal: Draw,
    /// Where the family's [`Composites::mass`] centres.
    mass: Draw,
    /// Where its [`Composites::age`] centres, in years.
    age: Draw,
}

impl Anchored for Anchor {
    type Kin = WingedFamily;

    /// The anchor of one family. Every shape axis centres on the family's own
    /// frame, nought, because the body reads each as a step from it; how far
    /// a roll strays is the family's. The bird strays furthest, toward the
    /// crow, the parrot and the songbird its corner holds (the owner's call):
    /// a longer neck and legs and a smaller head toward a crow, a long tail
    /// toward a parrot, a small one toward a songbird.
    fn of(family: WingedFamily) -> Self {
        match family {
            WingedFamily::Bird => Self {
                size: Draw::at(0.60, 0.10),
                wing_length: Draw::at(0.0, 0.35),
                neck_length: Draw::at(0.0, 0.5),
                leg_length: Draw::at(0.0, 0.45),
                tail_length: Draw::at(0.0, 0.55),
                head_size: Draw::at(0.0, 0.4),
                appeal: Draw::at(CUTE, 0.3),
                mass: Draw::at(0.0, 0.5),
                age: Draw::at(32.0, 12.0),
            },
            WingedFamily::Bat => Self {
                size: Draw::at(0.70, 0.09),
                wing_length: Draw::at(0.0, 0.25),
                neck_length: Draw::at(0.0, 0.25),
                leg_length: Draw::at(0.0, 0.25),
                tail_length: Draw::at(0.0, 0.3),
                head_size: Draw::at(0.0, 0.3),
                appeal: Draw::at(CUTE, 0.3),
                mass: Draw::at(0.0, 0.4),
                age: Draw::at(32.0, 12.0),
            },
            WingedFamily::Glider => Self {
                size: Draw::at(0.65, 0.08),
                wing_length: Draw::at(0.0, 0.25),
                neck_length: Draw::at(0.0, 0.25),
                leg_length: Draw::at(0.0, 0.25),
                tail_length: Draw::at(0.0, 0.3),
                head_size: Draw::at(0.0, 0.3),
                appeal: Draw::at(CUTE, 0.3),
                mass: Draw::at(0.0, 0.4),
                age: Draw::at(32.0, 12.0),
            },
        }
    }

    fn toward(own: Self, other: Self, share: f32) -> Self {
        let mix = |a: Draw, b: Draw| a.toward(b, share);
        Self {
            size: mix(own.size, other.size),
            wing_length: mix(own.wing_length, other.wing_length),
            neck_length: mix(own.neck_length, other.neck_length),
            leg_length: mix(own.leg_length, other.leg_length),
            tail_length: mix(own.tail_length, other.tail_length),
            head_size: mix(own.head_size, other.head_size),
            appeal: mix(own.appeal, other.appeal),
            mass: mix(own.mass, other.mass),
            age: mix(own.age, other.age),
        }
    }
}

/// The streams a winged body's family and cross are rolled on: its own, so
/// nothing a beast or a person draws moves.
const STREAMS: Streams = Streams {
    family: "winged.family",
    hybrid_family: "winged.hybridFamily",
    hybrid: "winged.hybrid",
    blend: "winged.blend",
};

impl Kin for WingedFamily {
    const ALL: &'static [Self] = &WingedFamily::ALL;

    fn name(self) -> &'static str {
        WingedFamily::name(self)
    }
}

impl Blended for WingedParams {
    type Kin = WingedFamily;

    fn lineage(&self) -> Lineage<WingedFamily> {
        Lineage {
            family: self.family,
            hybrid: self.hybrid,
            blend: self.blend,
        }
    }

    fn set_lineage(&mut self, lineage: Lineage<WingedFamily>) {
        kin::change_kind(self, lineage);
    }
}

impl Bred for WingedParams {
    type Anchor = Anchor;

    fn put_lineage(&mut self, lineage: Lineage<WingedFamily>) {
        self.family = lineage.family;
        self.hybrid = lineage.hybrid;
        self.blend = lineage.blend;
    }

    fn carry(&mut self, from: &Anchor, to: &Anchor) {
        to.size.carry(&mut self.size, from.size);
        to.wing_length
            .carry(&mut self.wing_length, from.wing_length);
        to.neck_length
            .carry(&mut self.neck_length, from.neck_length);
        to.leg_length.carry(&mut self.leg_length, from.leg_length);
        to.tail_length
            .carry(&mut self.tail_length, from.tail_length);
        to.head_size.carry(&mut self.head_size, from.head_size);
        to.appeal.carry(&mut self.appeal, from.appeal);
    }

    /// No part is stored yet: the head, the feet and the coat a winged body
    /// is built with are its leading family's until W3 and W4 store them.
    fn hand_over(&mut self) {}
}

impl WingedParams {
    /// The size envelope `sanitize` clamps to, in metres.
    #[must_use]
    pub fn size_envelope() -> (f32, f32) {
        size_envelope()
    }

    /// Makes this body another family, or another cross, carrying its own
    /// character across, as [`crate::QuadrupedParams::set_kind`] does.
    pub fn set_kind(&mut self, family: WingedFamily, hybrid: Option<WingedFamily>, blend: f32) {
        kin::change_kind(
            self,
            Lineage {
                family,
                hybrid,
                blend,
            },
        );
    }

    /// How this body stands: its leading family's.
    pub(crate) fn posture(&self) -> Posture {
        Posture::of(self.lineage().leading())
    }

    /// Draws which family this body is, and whether it is a cross, on the
    /// plan's own streams, before any axis.
    pub(crate) fn reroll_family(&mut self, rolls: &Rolls) {
        self.put_lineage(Lineage::rolled(rolls, &STREAMS));
    }

    /// The coat a winged body is painted in until it stores one (W4, #423):
    /// its leading family's, a stand-in rather than a roll. A brown bird, a
    /// dark bat and a blue-grey glider with the dark stripe down its back.
    pub(crate) fn coat(&self) -> super::Coat {
        use super::{Agouti, Coat, Pigment};
        match self.lineage().leading() {
            WingedFamily::Bird => Coat {
                pigment: Pigment::Liver,
                agouti: Agouti::Wild,
                ..Coat::default()
            },
            WingedFamily::Bat => Coat {
                pigment: Pigment::Black,
                ..Coat::default()
            },
            WingedFamily::Glider => Coat {
                pigment: Pigment::Black,
                agouti: Agouti::Wild,
                ..Coat::default()
            },
        }
    }

    /// The feet its leading family stands on, until W3 builds a bird's.
    pub(crate) fn feet(&self) -> super::Feet {
        match self.lineage().leading() {
            WingedFamily::Bird | WingedFamily::Bat => super::Feet::Claws,
            WingedFamily::Glider => super::Feet::Pads,
        }
    }
}

impl BodyPlan for WingedParams {
    fn sanitize(&mut self) {
        let default = Self::default();
        self.size = super::sanitize_axis(self.size, default.size, size_envelope());
        for (axis, fallback) in [
            (&mut self.wing_length, default.wing_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.leg_length, default.leg_length),
            (&mut self.tail_length, default.tail_length),
            (&mut self.head_size, default.head_size),
            (&mut self.appeal, default.appeal),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, signed_envelope());
        }
        if self.hybrid == Some(self.family) {
            self.hybrid = None;
        }
        self.blend = if self.hybrid.is_some() {
            super::sanitize_axis(self.blend, 0.5, BLEND_RANGE)
        } else {
            0.0
        };
    }

    /// A beast's head, trunk and feet (W3 builds a beak and a bird's feet), no
    /// hands, and the legs its posture stands on: a bird its hind pair, a bat
    /// and a glider all four.
    fn anatomy(&self) -> Anatomy {
        Anatomy {
            head: super::HeadKind::Beast,
            trunk: super::TrunkKind::Beast,
            hands: false,
            feet: FootKind::Beast,
            legs: Some(self.posture().legs()),
        }
    }

    /// Its family's, and no frame axis, as a beast's.
    fn composite_priors(&self) -> CompositePriors {
        let anchor = Anchor::of_lineage(self.lineage());
        CompositePriors {
            frame: false,
            mass: (anchor.mass.mean, anchor.mass.sigma),
            body_fat: DEFAULT_BODY_FAT,
            age: (anchor.age.mean, anchor.age.sigma),
        }
    }

    /// A bird's eyes are an owl's amber and gold, a bat's and a glider's dark,
    /// the leading family's: the cat's ramp and the horse's.
    fn iris(&self, shade: f32) -> [f32; 3] {
        let like = match self.lineage().leading() {
            WingedFamily::Bird => super::Family::Cat,
            WingedFamily::Bat | WingedFamily::Glider => super::Family::Horse,
        };
        super::coat::iris(like, shade)
    }

    /// **A body its cage cannot hull is eased toward its leading family's
    /// own until it can**, as a beast's is (#391): see `Ease`. The body as
    /// asked is built first and returned whenever it meshes, which is every
    /// body a roll is likely to draw; one that does not keeps as much of what
    /// was asked as the cage allows. The record keeps what was asked.
    fn skeleton(&self, composites: &Composites) -> Skeleton {
        let (params, composites) = self.ease(composites).apply(self, composites);
        params.assemble(&composites)
    }

    fn reroll(&mut self, category: Category, rolls: &Rolls, _composites: &Composites) {
        let signed = signed_envelope();
        let anchor = Anchor::of_lineage(self.lineage());
        let draw =
            |stream: &str, at: Draw, envelope| rolls.shape(stream, at.mean, at.sigma, envelope);
        match category {
            Category::Stature => {
                let (low, high) = size_envelope();
                let band = (
                    (anchor.size.mean * SIZE_BAND.0).max(low),
                    (anchor.size.mean * SIZE_BAND.1).min(high),
                );
                self.size = draw("winged.size", anchor.size, band);
            }
            Category::Proportions => {
                self.wing_length = draw("winged.wingLength", anchor.wing_length, signed);
                self.neck_length = draw("winged.neckLength", anchor.neck_length, signed);
                self.leg_length = draw("winged.legLength", anchor.leg_length, signed);
                self.tail_length = draw("winged.tailLength", anchor.tail_length, signed);
                self.appeal = draw("winged.appeal", anchor.appeal, signed);
            }
            Category::Head => {
                self.head_size = draw("winged.headSize", anchor.head_size, signed);
            }
            // The family is drawn by `reroll_family`, under the frame's lock;
            // the girth is the composites'; nothing here is a colour, a coat,
            // an age or a garment yet.
            Category::Build
            | Category::Frame
            | Category::Colouring
            | Category::Hair
            | Category::Age
            | Category::Outfit => {}
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        put_length(out, self.size);
        for axis in [
            self.wing_length,
            self.neck_length,
            self.leg_length,
            self.tail_length,
            self.head_size,
            self.appeal,
        ] {
            put_span(out, axis, signed_envelope());
        }
        out.push(self.family.code());
        out.push(self.hybrid.map_or(NO_HYBRID, WingedFamily::code));
        put_span(out, self.blend, BLEND_RANGE);
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            size: take_length(bytes)?,
            wing_length: take_span(bytes, signed_envelope())?,
            neck_length: take_span(bytes, signed_envelope())?,
            leg_length: take_span(bytes, signed_envelope())?,
            tail_length: take_span(bytes, signed_envelope())?,
            head_size: take_span(bytes, signed_envelope())?,
            appeal: take_span(bytes, signed_envelope())?,
            ..Self::default()
        };
        params.family = WingedFamily::from_code(take_byte(bytes)?);
        let hybrid = take_byte(bytes)?;
        params.hybrid = (hybrid != NO_HYBRID).then(|| WingedFamily::from_code(hybrid));
        params.blend = take_span(bytes, BLEND_RANGE)?;
        params.sanitize();
        Ok(params)
    }

    /// **No code older than version 10 carries a winged body**: the plan came
    /// after every layout these readers exist for, so a tag this plan's read
    /// at one of those versions was never written, and is refused as the
    /// unknown tag it was then.
    fn decode_pre_families(_bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
    }

    /// See [`Self::decode_pre_families`].
    fn decode_pre_distribution(_bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
    }

    /// See [`Self::decode_pre_families`].
    fn decode_pre_chest(_bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
    }

    /// See [`Self::decode_pre_families`].
    fn decode_legacy(_bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
    }

    /// See [`Self::decode_pre_families`].
    fn decode_reserved(_bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
    }
}

/// The winged plan's share-code tag.
pub(crate) const WINGED_TAG: u8 = 3;

/// How far each group of a body's axes goes from its leading family's own
/// frame toward what the record asks, `1` all the way and `0` none of it:
/// the cross's blend (toward the leading family's end, so the posture never
/// changes), each of the six shape axes, and the composites `mass` and
/// `bodyFat` together. Easing a group moves the body along the line its own
/// slider runs on, so an eased body is one a creator could have asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ease([f32; EASED]);

/// How many groups an [`Ease`] carries.
const EASED: usize = 8;

/// How many halvings each search for an eased body takes: the beast's.
const EASE_HALVINGS: usize = 6;

/// How many eases a thread remembers, with the record and the composites
/// each was found for: the beast's reason, an editor asking every frame.
const EASES_KEPT: usize = 4;

thread_local! {
    static EASES: RefCell<VecDeque<(WingedParams, Composites, Ease)>> =
        const { RefCell::new(VecDeque::new()) };
}

impl Ease {
    /// The body as asked.
    const AS_ASKED: Self = Self([1.0; EASED]);

    /// The record and composites this ease builds.
    fn apply(self, asked: &WingedParams, composites: &Composites) -> (WingedParams, Composites) {
        let share = |group: usize, value: f32, centre: f32| {
            if self.0[group] >= 1.0 {
                value
            } else {
                centre + (value - centre) * self.0[group]
            }
        };
        let lead_end = if asked.blend >= 0.5 { 1.0 } else { 0.0 };
        let params = WingedParams {
            blend: share(0, asked.blend, lead_end),
            wing_length: share(1, asked.wing_length, 0.0),
            neck_length: share(2, asked.neck_length, 0.0),
            leg_length: share(3, asked.leg_length, 0.0),
            tail_length: share(4, asked.tail_length, 0.0),
            head_size: share(5, asked.head_size, 0.0),
            appeal: share(6, asked.appeal, 0.0),
            ..*asked
        };
        let composites = Composites {
            mass: share(7, composites.mass, 0.0),
            body_fat: share(7, composites.body_fat, DEFAULT_BODY_FAT),
            ..*composites
        };
        (params, composites)
    }
}

impl WingedParams {
    /// Hangs this body's nodes, as its axes ask, on a skeleton.
    fn assemble(&self, composites: &Composites) -> Skeleton {
        let built = super::derive::winged::derive(self, composites);
        super::quadruped::hang(
            &built.dims,
            self.anatomy(),
            self.feet(),
            built.hind_inside,
            built.fore_inside,
            built.tail_inside,
        )
    }

    /// [`Self::search_ease`]'s answer, remembered on this thread.
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

    /// How far each group is built toward what this record asks: as asked
    /// when it meshes, and when even its leading family's own frame does not
    /// (none does, `tests/plan.rs`); otherwise the beast's search - every group
    /// together as far as it meshes, each back to what was asked where the
    /// body still meshes with it, then the rest together again.
    fn search_ease(&self, composites: &Composites) -> Ease {
        let meshes_at = |ease: Ease| {
            let (params, composites) = ease.apply(self, composites);
            meshes(&params.assemble(&composites))
        };
        if meshes_at(Ease::AS_ASKED) || !meshes_at(Ease([0.0; EASED])) {
            return Ease::AS_ASKED;
        }
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..EASE_HALVINGS {
            let middle = 0.5 * (low + high);
            if meshes_at(Ease([middle; EASED])) {
                low = middle;
            } else {
                high = middle;
            }
        }
        let mut ease = [low; EASED];
        for group in 0..EASED {
            let mut trial = ease;
            trial[group] = 1.0;
            if meshes_at(Ease(trial)) {
                ease = trial;
            }
        }
        let eased = |share: f32| Ease(ease.map(|kept| if kept < 1.0 { share } else { kept }));
        let mut high = 1.0f32;
        for _ in 0..EASE_HALVINGS {
            let middle = 0.5 * (low + high);
            if meshes_at(eased(middle)) {
                low = middle;
            } else {
                high = middle;
            }
        }
        eased(low)
    }
}

/// Whether the cage hulls `skeleton` and the rig roots it.
fn meshes(skeleton: &Skeleton) -> bool {
    crate::cage::build_cage(skeleton, &crate::cage::CageConfig::default()).is_ok()
        && crate::rig::Rig::from_skeleton(skeleton).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Zone;
    use crate::plan::kin::HYBRID_SHARE;

    /// One seed's kind and every axis, drawn as a record's re-roll draws them.
    pub(crate) fn rolled(seed: i64) -> WingedParams {
        let rolls = Rolls::new(seed);
        let mut params = WingedParams::default();
        params.reroll_family(&rolls);
        for category in Category::ALL {
            params.reroll(category, &rolls, &Composites::default());
        }
        params.sanitize();
        params
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = WingedParams {
            size: -5.0,
            tail_length: f32::INFINITY,
            wing_length: 9.0,
            hybrid: Some(WingedFamily::Bird),
            blend: 0.4,
            ..Default::default()
        };
        params.sanitize();
        assert_eq!(params.size, size_envelope().0);
        assert_eq!(params.tail_length, 0.0);
        assert_eq!(params.wing_length, 3.0);
        assert_eq!((params.hybrid, params.blend), (None, 0.0));
        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }

    #[test]
    fn the_families_roll_about_evenly_and_a_share_of_them_cross() {
        const SEEDS: usize = 3000;
        let mut count = [0usize; 3];
        let mut crosses = 0;
        let mut sizes = [0.0f32; 3];
        for seed in 0..SEEDS as i64 {
            let params = rolled(seed);
            if let Some(other) = params.hybrid {
                assert_ne!(other, params.family);
                assert!((0.25..=0.75).contains(&params.blend), "seed {seed}");
                crosses += 1;
                continue;
            }
            count[params.family.code() as usize] += 1;
            sizes[params.family.code() as usize] += params.size;
        }
        let share = crosses as f32 / SEEDS as f32;
        assert!((share - HYBRID_SHARE).abs() < 0.02, "{share} crosses");
        for n in count {
            let even = n as f32 / (SEEDS - crosses) as f32;
            assert!((even - 1.0 / 3.0).abs() < 0.03, "families {count:?}");
        }
        // About a cat's size to a dog's, the bird the smallest.
        let mean = |at: usize| sizes[at] / count[at] as f32;
        assert!(mean(0) < mean(2) && mean(2) < mean(1), "{sizes:?}");
    }

    #[test]
    fn a_roll_is_its_seed_s_and_the_frame_s_lock_keeps_the_family() {
        let rolled_record = |seed: i64, locks: crate::LockSet| {
            let mut record =
                crate::AvatarRecord::new("Roll", crate::Archetype::Winged(WingedParams::default()));
            record.locks = locks;
            record.reroll(seed);
            record
        };
        for seed in 0..40 {
            let once = rolled_record(seed, crate::LockSet::default());
            assert_eq!(
                once.archetype,
                rolled_record(seed, crate::LockSet::default()).archetype
            );
            // Locked, the family and the cross are the ones it had: here the
            // plan's default, a bird and no cross.
            let kept = rolled_record(seed, crate::LockSet::default().with(Category::Frame));
            let crate::Archetype::Winged(kept) = kept.archetype else {
                panic!("still winged");
            };
            assert_eq!((kept.family, kept.hybrid), (WingedFamily::Bird, None));
        }
    }

    #[test]
    fn the_posture_and_the_legs_are_the_leading_family_s() {
        let cross = |family, other, blend| WingedParams {
            family,
            hybrid: Some(other),
            blend,
            ..Default::default()
        };
        use WingedFamily::{Bat, Bird, Glider};
        assert_eq!(WingedParams::default().posture(), Posture::Perch);
        assert_eq!(cross(Bird, Bat, 0.49).posture(), Posture::Perch);
        assert_eq!(cross(Bird, Bat, 0.5).posture(), Posture::Crawl);
        assert_eq!(cross(Glider, Bird, 0.7).posture(), Posture::Perch);
        assert_eq!(cross(Bat, Glider, 0.2).posture(), Posture::Crawl);
        assert_eq!(
            cross(Bird, Glider, 0.8).anatomy().legs,
            Some(Legs::ALL),
            "a glider-led cross stands on four"
        );
        assert_eq!(
            cross(Bat, Bird, 0.8).anatomy().legs,
            Some(Legs::HIND),
            "a bird-led cross stands on two"
        );
    }

    /// **A bird's bony tail draws nothing** (#420, the owner, 2026-11-05): it is a
    /// stub of markers under the rump, and its visible tail is the fan its tail
    /// part draws (#421) - never the tube to the ground the first build hung
    /// there. A bat's nub and a glider's long tail are meshed, as are a
    /// four-legged cross's; a bird-led cross's is markers like a bird's.
    #[test]
    fn a_bird_s_tail_is_markers_and_the_others_are_meshed() {
        use WingedFamily::{Bat, Bird, Glider};
        let tail_markers = |params: WingedParams| -> (usize, usize) {
            let skeleton = params.skeleton(&Composites::default());
            let tail: Vec<_> = skeleton
                .nodes
                .iter()
                .filter(|node| node.zone == Zone::Tail)
                .collect();
            (tail.iter().filter(|node| node.marker).count(), tail.len())
        };
        let family = |family| WingedParams {
            family,
            ..Default::default()
        };
        let (markers, nodes) = tail_markers(family(Bird));
        assert!(
            nodes > 0,
            "the bird has a tail chain for its fan to hang from"
        );
        assert_eq!(markers, nodes, "a bird's tail draws nothing of its own");
        for other in [Bat, Glider] {
            assert_eq!(
                tail_markers(family(other)).0,
                0,
                "{other:?}'s tail is meshed"
            );
        }
        let cross = |family, other, blend| WingedParams {
            family,
            hybrid: Some(other),
            blend,
            ..Default::default()
        };
        let (markers, nodes) = tail_markers(cross(Glider, Bird, 0.7));
        assert_eq!(markers, nodes, "a bird-led cross perches, its tail a stub");
        assert_eq!(
            tail_markers(cross(Bird, Glider, 0.7)).0,
            0,
            "a glider-led cross's is meshed"
        );
    }

    #[test]
    fn a_change_of_family_carries_the_body_s_character() {
        let mut params = WingedParams {
            tail_length: 0.7,
            size: 0.70,
            ..Default::default()
        };
        params.set_kind(WingedFamily::Glider, None, 0.0);
        assert_eq!(params.family, WingedFamily::Glider);
        // As far from the glider's centre, in its spread, as it was from the
        // bird's: a long-tailed bird is a long-tailed glider.
        assert!(params.tail_length > 0.0, "{params:?}");
        assert!(params.size > 0.65, "{params:?}");
    }

    #[test]
    fn every_family_meshes_as_asked_and_a_roll_is_rarely_eased() {
        // The ease is the cage's last word (#391's), not the plan's: every
        // family's own frame meshes as it is, and nearly every roll as it was
        // drawn. Measured on 2,000 rolls: 6 eased, every one a heavy body
        // past `mass` 1.5 or a cross at the middle of its blend.
        for &family in WingedFamily::ALL.iter() {
            let params = WingedParams {
                family,
                ..Default::default()
            };
            assert_eq!(
                params.ease(&Composites::default()),
                Ease::AS_ASKED,
                "{family:?}"
            );
            let young = WingedParams {
                appeal: CUTE,
                ..params
            };
            assert_eq!(
                young.ease(&Composites::default()),
                Ease::AS_ASKED,
                "young {family:?}"
            );
        }
        let mut eased = 0;
        for seed in 0..500 {
            let params = rolled(seed);
            let rolls = Rolls::new(seed);
            let mut composites = Composites::default();
            for category in [Category::Frame, Category::Age, Category::Build] {
                composites.reroll(category, &rolls, &params.composite_priors());
            }
            if params.ease(&composites) != Ease::AS_ASKED {
                eased += 1;
            }
        }
        assert!(eased <= 5, "{eased} of 500 rolls eased");
    }

    #[test]
    fn a_share_code_payload_round_trips() {
        let params = WingedParams {
            size: 0.73,
            wing_length: 0.5,
            neck_length: -0.4,
            leg_length: 1.2,
            tail_length: -2.0,
            head_size: 0.3,
            appeal: 1.1,
            family: WingedFamily::Bat,
            hybrid: Some(WingedFamily::Glider),
            blend: 0.35,
        };
        let mut out = Vec::new();
        params.encode(&mut out);
        let mut read = out.as_slice();
        let back = WingedParams::decode(&mut read).expect("decodes");
        assert!(read.is_empty());
        assert_eq!(back.family, params.family);
        assert_eq!(back.hybrid, params.hybrid);
        assert!((back.size - params.size).abs() < 0.001);
        assert!((back.blend - params.blend).abs() < 0.01);
        assert!((back.tail_length - params.tail_length).abs() < 0.03);
        // And a layout from before the plan existed is refused, never read.
        let mut old = out.as_slice();
        assert_eq!(
            WingedParams::decode_pre_families(&mut old),
            Err(PlanDecodeError::UnknownArchetype(WINGED_TAG))
        );
    }
}
