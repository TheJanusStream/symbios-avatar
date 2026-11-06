//! What a winged body's surface carries beside its coat (W4, #423): the fur a
//! bat and a glider grow, and the marks their families are known by.
//!
//! **Derived, not stored.** Both come from the body's families and its blend,
//! as its posture and its feet do, so nothing here crosses the wire and a
//! record moves nothing by being read again: a change of family changes them
//! with it. What a winged body stores of its colour is its coat
//! ([`WingedParams::coat`]); these lie over it.
//!
//! A bird grows neither: its body is feathered (`hair::fur::contour`), and its
//! plumage's own marks are its coat's.

use super::Fur;
use super::kin::Blended;
use super::winged::{WingedFamily, WingedParams};

/// The fur each family grows in a beast's zones (`hair::fur`), at its own:
/// research-10 5.3, chosen by render.
///
/// - **A fruit bat** grows a ruff, the long fur of the mantle a flying fox
///   wears over its neck and shoulders (10 5.3: "a tan to golden mantle over
///   the shoulders and neck"), whose colour the painter lays under it
///   ([`WingedMarks::mantle`]); no tail, which *Pteropus* has none of (S21),
///   and no tufts on its plain ears (10 6.1). Its face is the fox face 10 6.1
///   gives it: a ruff reaches up behind the jaw onto the cheeks and the
///   throat, never the muzzle.
/// - **A sugar glider** grows a brush along its tail, as long as its body and
///   furred to the tip (S30, S31), and nothing else long: its ears are bare,
///   its face short-furred.
/// - **A bird** none: its contour feathers are cards of their own.
fn family_fur(family: WingedFamily) -> Fur {
    match family {
        WingedFamily::Bird => Fur::default(),
        WingedFamily::Bat => Fur {
            ruff: 1.6,
            ..Fur::default()
        },
        WingedFamily::Glider => Fur {
            tail: 1.0,
            ..Fur::default()
        },
    }
}

/// How much of each family's own mark a body draws, `0..=1` each: what the
/// coat painter lays over a bat's or a glider's coat (`texture::coat`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct WingedMarks {
    /// A fruit bat's mantle: a paler, golden-brown collar over the neck and
    /// the shoulders (10 5.3).
    pub mantle: f32,
    /// A sugar glider's stripes: a dark stripe from the forehead down the
    /// spine, and one through each eye to the ear (10 5.3, S31).
    pub stripes: f32,
}

impl WingedParams {
    /// **The fur this body grows** (#423), derived and not stored: its
    /// families' own ([`family_fur`]), each zone blended across a cross by its
    /// blend - a bat-glider cross wears a shorter ruff and a thinner brush -
    /// and none on a bird-led body, whose surface is feathers whatever it is
    /// crossed with, as its wings are.
    pub(crate) fn fur(&self) -> Fur {
        let lineage = self.lineage();
        if lineage.leading() == WingedFamily::Bird {
            return Fur::default();
        }
        lineage.blended(family_fur, |own, other, blend| {
            let mix = |a: f32, b: f32| a + (b - a) * blend;
            Fur {
                mane: mix(own.mane, other.mane),
                ruff: mix(own.ruff, other.ruff),
                tail: mix(own.tail, other.tail),
                feathering: mix(own.feathering, other.feathering),
                tufts: mix(own.tufts, other.tufts),
                mane_stiffness: own.mane_stiffness,
            }
        })
    }

    /// Whether this body's membrane is a bat's, naked skin, rather than a
    /// glider's furred patagium (#423): its leading family's, as its wings are.
    pub(crate) fn naked_membrane(&self) -> bool {
        self.lineage().leading() == WingedFamily::Bat
    }

    /// **The marks this body's families draw over its coat** (#423), derived
    /// and not stored: each family's mark as strong as its share of the body,
    /// so a bat-glider cross carries both, each faded; and none on a
    /// bird-led body, whose plumage is its coat's own.
    pub(crate) fn marks(&self) -> WingedMarks {
        let lineage = self.lineage();
        if lineage.leading() == WingedFamily::Bird {
            return WingedMarks::default();
        }
        let share = |family: WingedFamily| {
            let blend = lineage.blend.clamp(0.0, 1.0);
            match lineage.crossed() {
                Some(other) if other == family => blend,
                Some(_) if lineage.family == family => 1.0 - blend,
                None if lineage.family == family => 1.0,
                _ => 0.0,
            }
        };
        WingedMarks {
            mantle: share(WingedFamily::Bat),
            stripes: share(WingedFamily::Glider),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::kin::Lineage;

    fn of(lineage: Lineage<WingedFamily>) -> WingedParams {
        let mut params = WingedParams::default();
        params.set_lineage(lineage);
        params
    }

    #[test]
    fn a_bird_grows_no_fur_and_draws_no_marks_whatever_it_is_crossed_with() {
        use WingedFamily::{Bat, Bird, Glider};
        for lineage in [
            Lineage::pure(Bird),
            Lineage::cross(Bird, Bat, 0.3),
            Lineage::cross(Glider, Bird, 0.7),
        ] {
            let params = of(lineage);
            assert!(!params.fur().any(), "{lineage:?} grows fur");
            assert_eq!(params.marks(), WingedMarks::default(), "{lineage:?}");
        }
    }

    #[test]
    fn a_bat_wears_a_ruff_a_glider_a_brush_and_a_cross_some_of_each() {
        use WingedFamily::{Bat, Glider};
        let bat = of(Lineage::pure(Bat));
        assert!(bat.fur().ruff > 0.0 && bat.fur().tail == 0.0);
        assert_eq!(bat.marks().mantle, 1.0);
        let glider = of(Lineage::pure(Glider));
        assert!(glider.fur().tail > 0.0 && glider.fur().ruff == 0.0);
        assert_eq!(glider.marks().stripes, 1.0);
        let cross = of(Lineage::cross(Bat, Glider, 0.3));
        let (fur, marks) = (cross.fur(), cross.marks());
        assert!(fur.ruff < bat.fur().ruff && fur.ruff > 0.0);
        assert!(fur.tail < glider.fur().tail && fur.tail > 0.0);
        assert!((marks.mantle - 0.7).abs() < 1e-6 && (marks.stripes - 0.3).abs() < 1e-6);
    }
}
