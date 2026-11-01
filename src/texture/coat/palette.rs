//! A coat's colours, from its loci (#368).
//!
//! Every colour a coat is painted in comes out of one [`Palette`]: the dark
//! and the light pigment the agouti locus spreads, the ground a solid coat of
//! the pigment shows, and the bare parts - nose leather and pads, claws and
//! hooves - which follow the pigment the way a breeder expects them to: a
//! liver dog has a liver nose, a blue cat a grey one.
//!
//! Authored in sRGB, as the skin painter's colours are.
//!
//! **A colour a creator picked replaces the channel here** (#401), so that every
//! mark drawn in it follows without knowing; the few places a locus mixes in a
//! colour of its own have a field each below, which a picked colour takes over.
//! [`crate::plan::CoatColours`] states what each one reaches.

use glam::Vec3;

use crate::plan::{Coat, CoatChannel, Fantasy, Pigment};

/// White fur.
pub(super) const WHITE: Vec3 = Vec3::new(0.94, 0.93, 0.90);

/// The leather a white face wears: pink, where no pigment reaches it.
const PINK: Vec3 = Vec3::new(0.86, 0.58, 0.56);

/// A claw or a hoof below white hair: horn with no pigment in it.
const PALE_HORN: Vec3 = Vec3::new(0.86, 0.80, 0.68);

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

/// What a merle's diluted ground leans toward.
/// Provenance: **tuned by render** (#368).
const MERLE_GREY: Vec3 = Vec3::new(0.56, 0.57, 0.60);

/// Red's leather, as [`leather`] gives it, and diluted.
const RED_LEATHER: Vec3 = Vec3::new(0.20, 0.14, 0.13);
const CREAM_LEATHER: Vec3 = Vec3::new(0.62, 0.46, 0.44);

/// Horn, and diluted.
const HORN: Vec3 = Vec3::new(0.21, 0.19, 0.19);
const DILUTE_HORN: Vec3 = Vec3::new(0.36, 0.33, 0.32);

/// The six unnatural palettes, for the swatches.
const FANTASIES: [Fantasy; 6] = [
    Fantasy::Sky,
    Fantasy::Mint,
    Fantasy::Lilac,
    Fantasy::Rose,
    Fantasy::Sunny,
    Fantasy::Ocean,
];

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
    /// White hair.
    pub white: Vec3,
    /// Leather where white reaches it.
    pub bare_leather: Vec3,
    /// Horn under white hair.
    pub bare_horn: Vec3,
    /// What a colourpoint's body pales toward.
    pub pale: Vec3,
    /// What a merle's diluted ground leans toward.
    pub merle: Vec3,
    /// What a dun's body is diluted to, if a creator says so outright.
    pub dun: Option<Vec3>,
    /// Whether a creator picked the coat's own colour, which a dapple then leaves
    /// ungreyed.
    pub picked: bool,
    /// A mane's colour, if a creator picked one.
    pub mane: Option<Vec3>,
    /// A tail's colour, if a creator picked one.
    pub tail: Option<Vec3>,
}

impl Palette {
    /// The palette a coat's loci make.
    pub(super) fn of(coat: &Coat) -> Self {
        let picked = coat.colours;
        let chosen = |colour: Option<[f32; 3]>| colour.map(Vec3::from_array);
        let (dark, light) = match coat.fantasy {
            Fantasy::None => (eumelanin(coat), phaeomelanin(coat)),
            palette => fantasy(palette),
        };
        let light = chosen(picked.light).unwrap_or(light);
        // A red coat has no eumelanin to draw its marks in: they are its own
        // red, deepened, as a ginger tabby's stripes are.
        let (ground, dark) = match coat.pigment {
            Pigment::Red => (light, chosen(picked.dark).unwrap_or(light * 0.62)),
            _ => {
                let dark = chosen(picked.dark).unwrap_or(dark);
                (dark, dark)
            }
        };
        let (leather, horn) = (chosen(picked.leather), chosen(picked.horn));
        Self {
            dark,
            light,
            ground,
            leather: leather.unwrap_or_else(|| self::leather(coat, dark)),
            horn: horn.unwrap_or(if coat.dilute { DILUTE_HORN } else { HORN }),
            white: chosen(picked.white).unwrap_or(WHITE),
            bare_leather: leather.unwrap_or(PINK),
            bare_horn: horn.unwrap_or(PALE_HORN),
            pale: chosen(picked.light).unwrap_or_else(|| WHITE.lerp(light, 0.15)),
            // On a coat whose ground is the light already, a merle's patches
            // would have nothing to lean toward.
            merle: match (picked.light, picked.dark, coat.pigment) {
                (Some(light), _, Pigment::Black | Pigment::Liver) => Vec3::from_array(light),
                (_, Some(dark), Pigment::Red) => Vec3::from_array(dark),
                _ => MERLE_GREY,
            },
            dun: chosen(picked.light),
            picked: picked.dark.is_some() || picked.light.is_some(),
            mane: chosen(picked.mane),
            tail: chosen(picked.tail),
        }
    }
}

/// What `channel` paints on `coat` right now, in sRGB: the colour picked, or the
/// one the loci derive (#401). What a creator's picker opens on. A mane's and a
/// tail's are the coat's long hair, which on a marked coat varies card to card, so
/// this is its main colour.
#[must_use]
pub fn coat_colour(coat: &Coat, channel: CoatChannel) -> [f32; 3] {
    let palette = Palette::of(coat);
    let long_hair = if super::darkens_points(coat) {
        palette.dark
    } else {
        palette.ground
    };
    match channel {
        CoatChannel::Dark => palette.dark,
        CoatChannel::Light => palette.light,
        CoatChannel::White => palette.white,
        CoatChannel::Leather => palette.leather,
        CoatChannel::Horn => palette.horn,
        CoatChannel::Mane => palette.mane.unwrap_or(long_hair),
        CoatChannel::Tail => palette.tail.unwrap_or(long_hair),
    }
    .to_array()
}

/// The presets a creator is offered for `channel`, named: the colours the loci and
/// the unnatural palettes paint it, so that a swatch cannot drift from a coat (#401).
#[must_use]
pub fn coat_swatches(channel: CoatChannel) -> Vec<(&'static str, [f32; 3])> {
    let named = |names: &[(&'static str, Vec3)]| {
        names
            .iter()
            .map(|(name, colour)| (*name, colour.to_array()))
            .collect::<Vec<_>>()
    };
    let palettes = |pick: fn((Vec3, Vec3)) -> Vec3| {
        FANTASIES
            .iter()
            .map(|palette| (palette.name(), pick(fantasy(*palette)).to_array()))
            .collect::<Vec<_>>()
    };
    let darks = || {
        let mut all = named(&[
            ("black", BLACK),
            ("liver", LIVER),
            ("blue", BLUE),
            ("lilac", LILAC),
        ]);
        all.extend(palettes(|pair| pair.0));
        all
    };
    let lights = || {
        let mut all = named(&[
            ("ivory", IVORY),
            ("cream", CREAM),
            ("red", RED),
            ("mahogany", MAHOGANY),
        ]);
        all.extend(palettes(|pair| pair.1));
        all
    };
    match channel {
        CoatChannel::Dark => darks(),
        CoatChannel::Light => lights(),
        CoatChannel::White => named(&[("white", WHITE), ("ivory", IVORY), ("cream", CREAM)]),
        CoatChannel::Leather => named(&[
            ("black", BLACK * 1.15),
            ("liver", LIVER * 1.15),
            ("blue", BLUE * 1.15),
            ("lilac", LILAC * 1.15),
            ("red", RED_LEATHER),
            ("cream", CREAM_LEATHER),
            ("pink", PINK),
        ]),
        CoatChannel::Horn => named(&[
            ("horn", HORN),
            ("pale grey", DILUTE_HORN),
            ("pale", PALE_HORN),
        ]),
        CoatChannel::Mane | CoatChannel::Tail => {
            let mut all = darks();
            all.extend(lights());
            all.push(("white", WHITE.to_array()));
            all
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
        (Fantasy::None, Pigment::Red, false) => RED_LEATHER,
        (Fantasy::None, Pigment::Red, true) => CREAM_LEATHER,
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
