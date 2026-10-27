//! A coat's colours, from its loci (#368).
//!
//! Every colour a coat is painted in comes out of one [`Palette`]: the dark
//! and the light pigment the agouti locus spreads, the ground a solid coat of
//! the pigment shows, and the bare parts - nose leather and pads, claws and
//! hooves - which follow the pigment the way a breeder expects them to: a
//! liver dog has a liver nose, a blue cat a grey one.
//!
//! Authored in sRGB, as the skin painter's colours are.

use glam::Vec3;

use crate::plan::{Coat, Fantasy, Pigment};

/// White fur.
pub(super) const WHITE: Vec3 = Vec3::new(0.94, 0.93, 0.90);

/// The leather a white face wears: pink, where no pigment reaches it.
pub(super) const PINK: Vec3 = Vec3::new(0.86, 0.58, 0.56);

/// A claw or a hoof below white hair: horn with no pigment in it.
pub(super) const PALE_HORN: Vec3 = Vec3::new(0.86, 0.80, 0.68);

/// Black eumelanin.
const BLACK: Vec3 = Vec3::new(0.08, 0.075, 0.075);

/// Brown eumelanin: liver, chocolate.
const LIVER: Vec3 = Vec3::new(0.34, 0.19, 0.12);

/// Black, diluted: blue-grey.
const BLUE: Vec3 = Vec3::new(0.44, 0.46, 0.50);

/// Liver, diluted: lilac, isabella.
const LILAC: Vec3 = Vec3::new(0.64, 0.56, 0.50);

/// Phaeomelanin at the bottom of its classic range: cream.
const CREAM: Vec3 = Vec3::new(0.96, 0.88, 0.72);

/// Phaeomelanin past the classic range at the pale end, as far as
/// `intensity` reaches: ivory.
const IVORY: Vec3 = Vec3::new(0.98, 0.95, 0.88);

/// Phaeomelanin at the top of its classic range: a red setter's.
const RED: Vec3 = Vec3::new(0.74, 0.36, 0.13);

/// Phaeomelanin past the classic range at the deep end, as far as
/// `intensity` reaches: mahogany.
const MAHOGANY: Vec3 = Vec3::new(0.52, 0.18, 0.07);

/// The colours a coat is painted in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Palette {
    /// The dark pigment: marks, tipping, points.
    pub dark: Vec3,
    /// The light pigment: tan, a bay's body, a tabby's ground.
    pub light: Vec3,
    /// What a solid coat of the pigment is.
    pub ground: Vec3,
    /// Nose leather and pads.
    pub leather: Vec3,
    /// Claws and hooves.
    pub horn: Vec3,
}

impl Palette {
    /// The palette a coat's loci make.
    pub(super) fn of(coat: &Coat) -> Self {
        let (dark, light) = match coat.fantasy {
            Fantasy::None => (eumelanin(coat), phaeomelanin(coat)),
            palette => fantasy(palette),
        };
        // A red coat has no eumelanin to draw its marks in: they are its own
        // red, deepened, as a ginger tabby's stripes are.
        let (ground, dark) = match coat.pigment {
            Pigment::Red => (light, light * 0.62),
            _ => (dark, dark),
        };
        Self {
            dark,
            light,
            ground,
            leather: leather(coat, dark),
            horn: if coat.dilute {
                Vec3::new(0.36, 0.33, 0.32)
            } else {
                Vec3::new(0.21, 0.19, 0.19)
            },
        }
    }
}

/// The coat's eumelanin, diluted if it is.
fn eumelanin(coat: &Coat) -> Vec3 {
    match (coat.pigment, coat.dilute) {
        (Pigment::Liver, false) => LIVER,
        (Pigment::Liver, true) => LILAC,
        (_, false) => BLACK,
        (_, true) => BLUE,
    }
}

/// The coat's phaeomelanin by its intensity - cream at -1 through red at 1,
/// on to ivory at -3 and mahogany at 3, the envelope's ends - and toward
/// cream again if it is diluted.
fn phaeomelanin(coat: &Coat) -> Vec3 {
    let intensity = coat.intensity;
    let red = if intensity < -1.0 {
        CREAM.lerp(IVORY, ((-1.0 - intensity) * 0.5).min(1.0))
    } else if intensity <= 1.0 {
        CREAM.lerp(RED, (intensity + 1.0) * 0.5)
    } else {
        RED.lerp(MAHOGANY, ((intensity - 1.0) * 0.5).min(1.0))
    };
    if coat.dilute {
        red.lerp(CREAM, 0.6)
    } else {
        red
    }
}

/// The nose and the pads: the eumelanin, whatever the coat shows, as a
/// yellow dog's nose is black - deepened on a red coat, which has none in
/// its fur to say so.
fn leather(coat: &Coat, dark: Vec3) -> Vec3 {
    match (coat.fantasy, coat.pigment, coat.dilute) {
        (Fantasy::None, Pigment::Red, false) => Vec3::new(0.20, 0.14, 0.13),
        (Fantasy::None, Pigment::Red, true) => Vec3::new(0.62, 0.46, 0.44),
        (Fantasy::None, _, _) => eumelanin(coat) * 1.15,
        _ => dark * 0.6,
    }
}

/// An unnatural palette's dark and light.
/// Provenance: **chosen** (#368), as pastels over deep tones of one hue.
fn fantasy(palette: Fantasy) -> (Vec3, Vec3) {
    match palette {
        Fantasy::Sky => (Vec3::new(0.20, 0.34, 0.70), Vec3::new(0.70, 0.85, 0.97)),
        Fantasy::Mint => (Vec3::new(0.14, 0.48, 0.38), Vec3::new(0.72, 0.95, 0.82)),
        Fantasy::Lilac => (Vec3::new(0.40, 0.27, 0.60), Vec3::new(0.86, 0.76, 0.96)),
        Fantasy::Rose => (Vec3::new(0.62, 0.22, 0.40), Vec3::new(0.98, 0.78, 0.86)),
        Fantasy::Sunny => (Vec3::new(0.76, 0.44, 0.06), Vec3::new(1.00, 0.92, 0.46)),
        Fantasy::Ocean | Fantasy::None => {
            (Vec3::new(0.05, 0.30, 0.38), Vec3::new(0.46, 0.80, 0.80))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_coat_s_parts_follow_its_pigment() {
        let coat = |pigment, dilute| Coat {
            pigment,
            dilute,
            ..Coat::default()
        };
        let black = Palette::of(&coat(Pigment::Black, false));
        let blue = Palette::of(&coat(Pigment::Black, true));
        let liver = Palette::of(&coat(Pigment::Liver, false));
        // A black coat's nose is black, a blue's lighter and greyer, a
        // liver's brown.
        assert!(black.leather.x < 0.15);
        assert!(blue.leather.x > black.leather.x && blue.leather.z > blue.leather.x);
        assert!(liver.leather.x > liver.leather.z * 1.5);
        // A red coat's marks are its own red, deepened.
        let red = Palette::of(&coat(Pigment::Red, false));
        assert!(red.dark.x < red.ground.x && red.dark.x > red.dark.z);
    }

    #[test]
    fn intensity_runs_cream_to_red_and_an_unnatural_coat_is_unnatural() {
        let red = |intensity| {
            Palette::of(&Coat {
                intensity,
                ..Coat::default()
            })
            .ground
        };
        assert!(
            red(-1.0).z > red(0.0).z && red(0.0).z > red(1.0).z,
            "paler is creamier"
        );
        assert!(red(2.0).x < red(1.0).x, "and past red, mahogany");
        // The envelope's whole length moves the colour, to its ends.
        assert!(
            red(-3.0).z > red(-2.0).z && red(-2.0).z > red(-1.0).z,
            "ivory"
        );
        assert_eq!(red(3.0), MAHOGANY);
        assert_eq!(red(-3.0), IVORY);
        let sky = Palette::of(&Coat {
            fantasy: Fantasy::Sky,
            ..Coat::default()
        });
        assert!(sky.ground.z > sky.ground.x, "a sky coat is blue");
    }
}
