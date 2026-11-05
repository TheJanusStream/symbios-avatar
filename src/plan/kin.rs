//! A body plan's families, and what blends a body between them (#366, #419).
//!
//! A plan with families - the quadruped's dog, cat and horse - does not build a
//! family's body. A family is an **anchor**: where a roll centres each of the
//! plan's axes and how widely it spreads about it (`Draw`). A body is its
//! axes, rolled around its family's anchor, and a **cross** is rolled around
//! an anchor drawn between its two families' by its blend ([`Lineage`]). The
//! plan's own frames - bone lengths, trunk sections, a young animal's
//! proportions - blend the same way, by the same blend.
//!
//! **Everything about that is the same for every plan with families**, and it
//! lives here so the second such plan (the winged, #418) blends its bird, bat
//! and glider exactly as the quadruped blends its three:
//!
//! * `Lineage::blended` draws any per-family table between a cross's two
//!   families, clamping its blend, and hands a body of one family its own.
//! * `Draw::carry` moves an axis across a change of family: as far from its
//!   new centre as it sat from its old, in the anchor's own spread, so a
//!   long-legged dog becomes a long-legged cat and not the mean cat.
//! * `change_kind` is the whole change: carry every axis, hand every part
//!   over (`Lineage::handed_over`), sanitise.
//! * `Lineage::rolled` draws a re-roll's family and cross, `HYBRID_SHARE`
//!   of them crosses, and `Lineage::drawn_part` a part off one parent or the
//!   other as often as the blend leans.
//!
//! What a plan brings is its family type ([`Kin`]), its anchors
//! (`Anchored`) and the axes and parts a change of family moves (`Bred`).
//! The public face is [`Kin`], [`Lineage`] and [`Blended`], so a tool can list
//! a plan's families and set one without naming the plan's own type; the rest
//! is the crate's own.
//!
//! **Moving this here moved no roll** (#419): the streams, the order they are
//! drawn in and every expression are the quadruped's as they were.

use std::fmt::Debug;

use super::{BodyPlan, Rolls};

/// The share of re-rolled bodies that come out a cross of two families (#366:
/// hybrids are in, the owner's call). Drawn on its own stream.
pub(crate) const HYBRID_SHARE: f32 = 0.15;

/// A plan's family: one of the kinds of animal its bodies are blended from.
///
/// [`crate::Family`] is the quadruped's.
pub trait Kin: Copy + Eq + Default + Debug + 'static {
    /// Every family, in the order a re-roll picks among them.
    const ALL: &'static [Self];

    /// Its name, as a record writes it.
    fn name(self) -> &'static str;

    /// The family `name` names, or `None` for one this plan does not have.
    ///
    /// Strict, for a tool's argument; a record's own read of a name it does
    /// not know is the plan's default family, which is its type's business.
    fn parse(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kin| kin.name() == name)
    }
}

/// Which families a body is blended from: its own, the second of a cross, and
/// how far toward that second it is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lineage<K> {
    /// The body's family.
    pub family: K,
    /// The second family of a cross, if it is one.
    pub hybrid: Option<K>,
    /// How far a cross is drawn toward [`Self::hybrid`], `0..=1`.
    pub blend: f32,
}

impl<K: Kin> Lineage<K> {
    /// A body of one family, and no cross.
    #[must_use]
    pub fn pure(family: K) -> Self {
        Self {
            family,
            hybrid: None,
            blend: 0.0,
        }
    }

    /// A cross of `family` and `other`, `blend` of the way to `other`.
    #[must_use]
    pub fn cross(family: K, other: K, blend: f32) -> Self {
        Self {
            family,
            hybrid: Some(other),
            blend,
        }
    }

    /// The second family, if this is a cross of two different ones.
    #[must_use]
    pub fn crossed(self) -> Option<K> {
        self.hybrid.filter(|&other| other != self.family)
    }

    /// The family a cross leans toward, or the family of a body that is none.
    #[must_use]
    pub fn leading(self) -> K {
        match self.hybrid {
            Some(other) if self.blend >= 0.5 => other,
            _ => self.family,
        }
    }

    /// A per-family table drawn for this body: its family's `of`, and a
    /// cross's `mix`ed between its two families' by its blend, clamped into
    /// `0..=1`. A cross of a family with itself is that family.
    pub(crate) fn blended<A>(self, of: impl Fn(K) -> A, mix: impl Fn(A, A, f32) -> A) -> A {
        let own = of(self.family);
        let Some(other) = self.crossed() else {
            return own;
        };
        mix(own, of(other), self.blend.clamp(0.0, 1.0))
    }

    /// What becomes of a part `kept` - a family's own, by `of` - when the
    /// body changes kind: kept while one of its families has it, its
    /// leading family's otherwise. So dragging a cross's blend does not flip
    /// its eyes or its nose back and forth at the middle.
    pub(crate) fn handed_over<T: Copy + PartialEq>(self, kept: T, of: impl Fn(K) -> T) -> T {
        let parents = [Some(self.family), self.hybrid];
        if parents
            .into_iter()
            .flatten()
            .any(|family| of(family) == kept)
        {
            kept
        } else {
            of(self.leading())
        }
    }

    /// A part drawn for a re-roll: its family's, and a cross its hybrid's as
    /// often as its blend leans that way, each part on its own stream.
    pub(crate) fn drawn_part<T>(self, rolls: &Rolls, stream: &str, of: impl Fn(K) -> T) -> T {
        match self.hybrid {
            Some(other) if rolls.range(stream, 0.0, 1.0) < self.blend => of(other),
            _ => of(self.family),
        }
    }

    /// Draws a re-roll's family, and whether it is a cross of two: each on
    /// its own stream, the family evenly among [`Kin::ALL`], [`HYBRID_SHARE`]
    /// of the rolls a cross, blended a quarter to three quarters of the way.
    pub(crate) fn rolled(rolls: &Rolls, streams: &Streams) -> Self {
        let pick = |share: f32, among: &[K]| {
            among[((share * among.len() as f32) as usize).min(among.len() - 1)]
        };
        let family = pick(rolls.range(streams.family, 0.0, 1.0), K::ALL);
        let others: Vec<K> = K::ALL
            .iter()
            .copied()
            .filter(|&other| other != family)
            .collect();
        let second = pick(rolls.range(streams.hybrid_family, 0.0, 1.0), &others);
        let crossed = rolls.range(streams.hybrid, 0.0, 1.0) < HYBRID_SHARE;
        let blend = rolls.range(streams.blend, 0.25, 0.75);
        Self {
            family,
            hybrid: crossed.then_some(second),
            blend: if crossed { blend } else { 0.0 },
        }
    }
}

/// The streams [`Lineage::rolled`] draws on: a plan's own names, which are a
/// record's reproducibility and never change.
pub(crate) struct Streams {
    /// The family.
    pub family: &'static str,
    /// A cross's second family.
    pub hybrid_family: &'static str,
    /// Whether it is a cross.
    pub hybrid: &'static str,
    /// A cross's blend.
    pub blend: &'static str,
}

/// Where a roll centres one axis, and how widely it spreads about it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Draw {
    /// The centre.
    pub mean: f32,
    /// The spread.
    pub sigma: f32,
}

impl Draw {
    /// A draw centred on `mean`, spread by `sigma`.
    pub(crate) const fn at(mean: f32, sigma: f32) -> Self {
        Self { mean, sigma }
    }

    /// The draw `share` of the way to `other`: a cross centres between its
    /// families and spreads as widely as the wider of the two.
    pub(crate) fn toward(self, other: Self, share: f32) -> Self {
        Self {
            mean: self.mean + (other.mean - self.mean) * share,
            sigma: self.sigma.max(other.sigma),
        }
    }

    /// Carries an axis `value` centred by `from` across to this draw: as far
    /// from this centre as it sat from that one, in each one's own spread.
    pub(crate) fn carry(self, value: &mut f32, from: Self) {
        *value = self.mean + (*value - from.mean) * (self.sigma / from.sigma);
    }
}

/// A plan's anchor: the [`Draw`] of every axis its family centres (#366).
pub(crate) trait Anchored: Sized {
    /// The plan's family.
    type Kin: Kin;

    /// One family's anchor.
    fn of(kin: Self::Kin) -> Self;

    /// `own` drawn `share` of the way toward `other`: a cross's.
    fn toward(own: Self, other: Self, share: f32) -> Self;

    /// The anchor a body of `lineage` is rolled around.
    fn of_lineage(lineage: Lineage<Self::Kin>) -> Self {
        lineage.blended(Self::of, Self::toward)
    }
}

/// A plan whose bodies are blended from families, as a tool sees it.
///
/// [`crate::QuadrupedParams`] is one: a tool lists its families through
/// [`Kin::ALL`] and moves a body between them with [`Self::set_lineage`]
/// without naming the plan.
pub trait Blended {
    /// The plan's family.
    type Kin: Kin;

    /// Which families this body is blended from.
    fn lineage(&self) -> Lineage<Self::Kin>;

    /// Makes this body another family, or another cross, carrying its own
    /// character across: what a creator's family picker does. See
    /// [`crate::QuadrupedParams::set_kind`].
    fn set_lineage(&mut self, lineage: Lineage<Self::Kin>);
}

/// What [`change_kind`] asks of a plan beyond its lineage.
pub(crate) trait Bred: BodyPlan + Blended {
    /// The plan's anchor.
    type Anchor: Anchored<Kin = Self::Kin>;

    /// Writes `lineage` into the body's fields, and nothing else.
    fn put_lineage(&mut self, lineage: Lineage<Self::Kin>);

    /// Moves every axis a family centres from `from` to `to` ([`Draw::carry`]).
    fn carry(&mut self, from: &Self::Anchor, to: &Self::Anchor);

    /// Hands every part a family owns over to the new lineage
    /// ([`Lineage::handed_over`]).
    fn hand_over(&mut self);
}

/// Makes `params` a body of `lineage`, carrying its own character across.
///
/// Setting the family alone changes nothing a creator can see, because the
/// geometry reads only the axes and a family is where a roll centres them. So
/// every axis moves as far from its new anchor as it sat from its old one, in
/// the anchor's own spread, and dragging a cross's blend morphs the body
/// between its two families as the blend moves. Sanitised before the new
/// anchor is read and again at the end, so a cross of a family with itself
/// comes out no cross.
pub(crate) fn change_kind<P: Bred>(params: &mut P, lineage: Lineage<P::Kin>) {
    let from = P::Anchor::of_lineage(params.lineage());
    params.put_lineage(lineage);
    params.sanitize();
    let to = P::Anchor::of_lineage(params.lineage());
    params.carry(&from, &to);
    params.hand_over();
    params.sanitize();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Family;

    #[test]
    fn a_family_parses_by_its_name_and_only_by_it() {
        for &family in Family::ALL.iter() {
            assert_eq!(<Family as Kin>::parse(family.name()), Some(family));
        }
        assert_eq!(<Family as Kin>::parse("griffin"), None);
        assert_eq!(<Family as Kin>::ALL, &Family::ALL);
    }

    #[test]
    fn a_cross_of_a_family_with_itself_blends_nothing() {
        let blend = |lineage: Lineage<Family>| {
            lineage.blended(|family| family as u8 as f32, |a, b, t| a + (b - a) * t)
        };
        assert_eq!(blend(Lineage::pure(Family::Cat)), 1.0);
        assert_eq!(blend(Lineage::cross(Family::Cat, Family::Cat, 0.5)), 1.0);
        assert_eq!(blend(Lineage::cross(Family::Dog, Family::Horse, 0.25)), 0.5);
        // Clamped: a blend past either end is that end.
        assert_eq!(blend(Lineage::cross(Family::Dog, Family::Horse, 3.0)), 2.0);
        assert_eq!(
            Lineage::cross(Family::Dog, Family::Horse, 0.5).leading(),
            Family::Horse
        );
        assert_eq!(
            Lineage::cross(Family::Dog, Family::Horse, 0.4).leading(),
            Family::Dog
        );
    }

    #[test]
    fn a_carried_axis_keeps_its_place_in_the_spread() {
        let mut value = 0.9;
        Draw::at(0.0, 0.2).carry(&mut value, Draw::at(0.5, 0.4));
        assert!((value - 0.2).abs() < 1e-6, "{value}");
    }
}
