//! What a re-roll dresses a body in (#358).
//!
//! An outfit used to be the one part of a record a re-roll never touched, so
//! every body a seed described — every identity's default avatar — wore the
//! same pink top over the same green trousers. The owner asked for the outfit
//! to be part of the seeded draw. It is its own lockable
//! [`Category::Outfit`](crate::Category::Outfit), drawn from streams named for
//! itself, so no other axis moved on any seed.
//!
//! **Culture, not anatomy, so uncoupled** — the rule the hair roll states for
//! a haircut. Nothing here reads the frame axis or the age: which cut
//! somebody wears is not a property of their body. The two things it does
//! read are legibility rather than taste: a top the colour of the skin under it
//! reads as bare skin, and a top the colour of the trousers reads as one
//! bodysuit, which is what the old defaults were chosen far apart on the
//! wheel to avoid.
//!
//! Distributions are weighted toward what a street looks like — trousers
//! mostly denim, black and neutrals, tops a third neutral — because a
//! uniform draw over the colour wheel reads as a costume department. Every
//! weight here is **a judgement, not a measurement**: none is looked up, and
//! the tables were checked only against what seeds 1 to 12 draw, two of them
//! rendered. They are the place to tune against a sheet of rolled bodies.

use rand::Rng as _;
use serde_json::Value;

use super::garment::dye;
use super::params::{GarmentParams, OutfitParams, SHORTS};
use super::surface::{GarmentTexture, SurfaceConfig};
use crate::plan::Rolls;
use crate::texture::SkinParams;

/// Trouser colours, sRGB, and how often each is drawn relative to the rest.
///
/// Denim first, then the darks and the neutrals. The share left over for a
/// dyed pair is [`DYED_TROUSERS`].
const TROUSER_COLOURS: [([f32; 3], f32); 9] = [
    ([0.23, 0.32, 0.47], 20.0), // denim
    ([0.12, 0.15, 0.25], 12.0), // indigo
    ([0.07, 0.07, 0.08], 15.0), // black
    ([0.22, 0.22, 0.24], 12.0), // charcoal
    ([0.60, 0.54, 0.40], 10.0), // khaki
    ([0.35, 0.25, 0.17], 8.0),  // brown
    ([0.32, 0.34, 0.21], 7.0),  // olive
    ([0.47, 0.47, 0.49], 6.0),  // grey
    ([0.74, 0.70, 0.60], 4.0),  // stone
];

/// The weight of a dyed pair of trousers against [`TROUSER_COLOURS`].
const DYED_TROUSERS: f32 = 6.0;

/// Neutral tops, sRGB.
const NEUTRAL_TOPS: [[f32; 3]; 7] = [
    [0.90, 0.90, 0.88], // white
    [0.86, 0.82, 0.72], // cream
    [0.70, 0.70, 0.72], // light grey
    [0.52, 0.52, 0.54], // heather
    [0.22, 0.22, 0.24], // charcoal
    [0.07, 0.07, 0.08], // black
    [0.13, 0.16, 0.28], // navy
];

/// How often a rolled top is a neutral rather than a dyed colour.
const NEUTRAL_TOP: f64 = 0.35;

/// Sleeve cuts: sleeveless, short, to the elbow, three-quarter, long — each
/// a span of lengths and a weight.
const SLEEVE_CUTS: [((f32, f32), f32); 5] = [
    ((0.0, 0.0), 12.0),
    ((0.18, 0.32), 38.0),
    ((0.45, 0.55), 8.0),
    ((0.65, 0.80), 12.0),
    ((1.0, 1.0), 30.0),
];

/// Trouser cuts: shorts, to the knee, to the calf, full length.
const TROUSER_CUTS: [((f32, f32), f32); 4] = [
    ((SHORTS, 0.30), 18.0),
    ((0.45, 0.55), 8.0),
    ((0.70, 0.85), 10.0),
    ((1.0, 1.0), 64.0),
];

/// How often a rolled garment is a visible weave rather than plain cloth.
///
/// A fifth each. With [`COSTUME`] that textures 22% of rolled garments and
/// puts a cloth atlas on two rolled bodies in five (7,932 of seeds 0 to
/// 19,999) — the price of a texture is an atlas per body, and a room of
/// defaults should not pay it for every one of them.
const WOVEN: f64 = 0.2;

/// How often a rolled garment is a costume surface instead: the rare
/// surprise, the hair roll's dyed head for clothes.
const COSTUME: f64 = 0.03;

/// The surfaces a costume draws from: the ones that read as something worn.
const COSTUMES: [&str; 5] = ["Metal", "Chitin", "Enamel", "Truchet", "Encaustic"];

/// How far apart in lightness a top and its trousers must sit before they
/// read as two garments.
const PAIR_CONTRAST: f32 = 0.12;

/// How near a garment's colour may come to the skin under it before it reads
/// as bare skin.
const SKIN_CONTRAST: f32 = 0.14;

/// Draws a fresh outfit.
///
/// Every stream is drawn on every roll whatever the branches decide, for the
/// reason the hair roll gives: a draw that is skipped costs nothing, and one
/// that is added moves no other axis on any seed.
pub(crate) fn reroll_outfit(outfit: &mut OutfitParams, rolls: &Rolls, skin: &SkinParams) {
    // The trousers keep clear of the skin; the top keeps clear of both, and
    // is settled against the two TOGETHER — moved off the skin alone, a beige
    // top pushed lighter to part from grey trousers came back darker to part
    // from the skin, and landed on the trousers again.
    let tone = skin.base_tone().to_array();
    let trousers_colour = settle(trouser_colour(rolls), |colour| clear_of(colour, tone));
    let top_colour = settle(top_colour(rolls), |colour| {
        clear_of(colour, tone)
            && (lightness(colour) - lightness(trousers_colour)).abs() >= PAIR_CONTRAST
    });

    outfit.top = GarmentParams {
        colour: top_colour,
        length: length(rolls, "outfit.top", &SLEEVE_CUTS),
        texture: texture(rolls, "outfit.top"),
    };
    outfit.trousers = GarmentParams {
        colour: trousers_colour,
        length: length(rolls, "outfit.trousers", &TROUSER_CUTS),
        texture: texture(rolls, "outfit.trousers"),
    };
}

/// A pair of trousers' colour: a weighted pick from the street, each shade a
/// little lighter or darker than the next pair's.
fn trouser_colour(rolls: &Rolls) -> [f32; 3] {
    let weights: Vec<f32> = TROUSER_COLOURS
        .iter()
        .map(|(_, weight)| *weight)
        .chain([DYED_TROUSERS])
        .collect();
    let dyed = dye(
        rolls.range("outfit.trousers.hue", 0.0, 1.0),
        rolls.range("outfit.trousers.shade", 0.1, 0.6),
    );
    let value = rolls.range("outfit.trousers.value", 0.85, 1.15);
    let picked = TROUSER_COLOURS
        .get(rolls.pick("outfit.trousers.colour", &weights))
        .map_or(dyed, |(colour, _)| *colour);
    picked.map(|channel| (channel * value).clamp(0.0, 1.0))
}

/// A top's colour: a neutral about a third of the time, and otherwise a dye
/// anywhere on the wheel, leaning light.
fn top_colour(rolls: &Rolls) -> [f32; 3] {
    let neutral = NEUTRAL_TOPS[rolls.pick("outfit.top.tone", &[1.0; NEUTRAL_TOPS.len()])];
    // A square root leans the shade light: a top is more often pale than
    // dark, where trousers are the other way about.
    let dyed = dye(
        rolls.range("outfit.top.hue", 0.0, 1.0),
        0.25 + 0.7 * rolls.range("outfit.top.shade", 0.0, 1.0).sqrt(),
    );
    if rolls.chance("outfit.top.neutral", NEUTRAL_TOP) {
        neutral
    } else {
        dyed
    }
}

/// A garment's length: a cut from `cuts`, then a length inside its span.
fn length(rolls: &Rolls, garment: &str, cuts: &[((f32, f32), f32)]) -> f32 {
    let weights: Vec<f32> = cuts.iter().map(|(_, weight)| *weight).collect();
    let within = rolls.range(&format!("{garment}.length"), 0.0, 1.0);
    let ((low, high), _) = cuts[rolls.pick(&format!("{garment}.cut"), &weights)];
    low + (high - low) * within
}

/// A garment's texture, if the roll gives it one: a weave a fifth of the
/// time, a costume surface rarely, plain cloth otherwise.
fn texture(rolls: &Rolls, garment: &str) -> Option<GarmentTexture> {
    let draw = |axis: &str| format!("{garment}.{axis}");
    let woven = rolls.chance(&draw("woven"), WOVEN);
    let costume = rolls.chance(&draw("costume"), COSTUME);
    let weave = weave(rolls, garment);
    let dressing = COSTUMES[rolls.pick(&draw("costumeSurface"), &[1.0; COSTUMES.len()])];
    let seed = rolls.stream(&draw("surfaceSeed")).random::<u32>();
    let scale = rolls.range(&draw("tile"), 3.0, 7.0);
    let surface = if costume {
        reseeded(SurfaceConfig::named(dressing)?, seed)
    } else if woven {
        weave
    } else {
        return None;
    };
    Some(GarmentTexture {
        surface,
        scale,
        rotation: 0.0,
    })
}

/// A woven fabric in near-white threads, so the garment's colour is the
/// cloth's and the weave is what the texture adds.
fn weave(rolls: &Rolls, garment: &str) -> SurfaceConfig {
    use symbios_texture::fabric::{FabricConfig, WeaveKind};
    let draw = |axis: &str| format!("{garment}.{axis}");
    let weave = match rolls.pick(&draw("weave"), &[40.0, 35.0, 10.0, 15.0]) {
        1 => WeaveKind::Twill,
        2 => WeaveKind::Satin,
        3 => WeaveKind::Basket,
        _ => WeaveKind::Plain,
    };
    // The fabric's own colours are linear. A weft a little darker than the
    // warp is what shows the weave under a dye; the same colour both ways
    // reads as felt.
    let warp = 0.92;
    let weft = warp - rolls.range(&draw("twoTone"), 0.02, 0.25);
    SurfaceConfig::Fabric(FabricConfig {
        seed: rolls.stream(&draw("fabricSeed")).random::<u32>(),
        weave,
        thread_count: rolls.range(&draw("threads"), 14.0, 36.0).round().into(),
        weave_contrast: rolls.range(&draw("contrast"), 0.3, 0.8).into(),
        fuzz: rolls.range(&draw("fuzz"), 0.15, 0.5).into(),
        color_warp: [warp; 3],
        color_weft: [weft; 3],
        ..FabricConfig::default()
    })
}

/// A surface with its seed replaced, for the generators that have one.
///
/// Through the wire rather than per generator, because the costume list is
/// names and every surface calls its seed `seed`.
fn reseeded(surface: SurfaceConfig, seed: u32) -> SurfaceConfig {
    let mut wire = surface.to_wire();
    if wire.contains_key("seed") {
        wire.insert("seed".into(), Value::from(seed));
    }
    SurfaceConfig::from_wire(&wire)
}

/// How light a colour reads, as sRGB luma.
fn lightness(colour: [f32; 3]) -> f32 {
    0.2126 * colour[0] + 0.7152 * colour[1] + 0.0722 * colour[2]
}

/// `colour`, or the nearest of its lighter and darker versions that reads
/// as it should.
///
/// Out in thirds toward white and toward black, lighter first at each step;
/// the colour itself if nothing within three steps either way will do, which
/// neither garment of seeds 0 to 4,999 needed at any of eleven even steps
/// along the melanin ramp.
fn settle(colour: [f32; 3], reads: impl Fn([f32; 3]) -> bool) -> [f32; 3] {
    let lighter = |c: [f32; 3]| c.map(|channel| channel + (1.0 - channel) / 3.0);
    let darker = |c: [f32; 3]| c.map(|channel| channel * 2.0 / 3.0);
    let (mut up, mut down) = (colour, colour);
    let mut candidates = vec![colour];
    for _ in 0..3 {
        up = lighter(up);
        down = darker(down);
        candidates.extend([up, down]);
    }
    candidates
        .into_iter()
        .find(|&candidate| reads(candidate))
        .unwrap_or(colour)
}

/// Whether a colour sits far enough from a skin tone not to read as it.
fn clear_of(colour: [f32; 3], tone: [f32; 3]) -> bool {
    colour
        .iter()
        .zip(tone)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
        >= SKIN_CONTRAST
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolled(seed: i64) -> OutfitParams {
        let mut outfit = OutfitParams::default();
        reroll_outfit(&mut outfit, &Rolls::new(seed), &SkinParams::default());
        outfit.sanitize();
        outfit
    }

    #[test]
    fn a_seed_always_dresses_the_same_way() {
        assert_eq!(rolled(42), rolled(42));
        assert_ne!(rolled(42), rolled(43));
    }

    #[test]
    fn a_rolled_population_varies_in_every_way_an_outfit_can() {
        // What the owner asked for: seeded bodies that do not all wear one
        // outfit. Over two hundred seeds every axis has to move, and the
        // distributions have to land where the tables put them.
        let outfits: Vec<OutfitParams> = (0..200).map(rolled).collect();
        let distinct = |read: &dyn Fn(&OutfitParams) -> [i64; 3]| {
            let mut seen: Vec<[i64; 3]> = outfits.iter().map(read).collect();
            seen.sort_unstable();
            seen.dedup();
            seen.len()
        };
        let quantised = |colour: [f32; 3]| colour.map(|channel| (channel * 1000.0).round() as i64);
        assert!(distinct(&|o| quantised(o.top.colour)) > 100);
        assert!(distinct(&|o| quantised(o.trousers.colour)) > 100);

        let sleeveless = outfits.iter().filter(|o| o.top.length == 0.0).count();
        let long = outfits.iter().filter(|o| o.top.length == 1.0).count();
        let shorts = outfits.iter().filter(|o| o.trousers.length < 0.31).count();
        let full = outfits.iter().filter(|o| o.trousers.length == 1.0).count();
        assert!((10..=45).contains(&sleeveless), "{sleeveless} sleeveless");
        assert!((35..=90).contains(&long), "{long} long-sleeved");
        assert!((20..=60).contains(&shorts), "{shorts} in shorts");
        assert!((100..=160).contains(&full), "{full} full-length");

        let textured = outfits.iter().filter(|o| o.is_textured()).count();
        assert!(
            (40..=110).contains(&textured),
            "{textured} of 200 rolled bodies carry a texture"
        );
    }

    #[test]
    fn a_rolled_outfit_is_already_in_range() {
        for seed in 0..100 {
            let outfit = rolled(seed);
            let mut again = outfit.clone();
            again.sanitize();
            assert_eq!(again, outfit, "seed {seed} rolled off the wire's grid");
            assert!(outfit.trousers.length >= SHORTS);
        }
    }

    #[test]
    fn a_top_and_its_trousers_never_read_as_one_garment() {
        for seed in 0..300 {
            let outfit = rolled(seed);
            let apart = (lightness(outfit.top.colour) - lightness(outfit.trousers.colour)).abs();
            let distance = outfit
                .top
                .colour
                .iter()
                .zip(outfit.trousers.colour)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(
                apart >= PAIR_CONTRAST * 0.5 || distance > 0.15,
                "seed {seed}: {:?} over {:?}",
                outfit.top.colour,
                outfit.trousers.colour
            );
        }
    }

    #[test]
    fn a_garment_is_never_the_colour_of_the_skin_under_it() {
        // Across the complexion ramp, since the rule reads the skin.
        for melanin in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let skin = SkinParams {
                melanin,
                ..SkinParams::default()
            };
            let tone = skin.base_tone().to_array();
            for seed in 0..200 {
                let mut outfit = OutfitParams::default();
                reroll_outfit(&mut outfit, &Rolls::new(seed), &skin);
                for colour in [outfit.top.colour, outfit.trousers.colour] {
                    let distance = colour
                        .iter()
                        .zip(tone)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f32>()
                        .sqrt();
                    assert!(
                        distance >= SKIN_CONTRAST * 0.5,
                        "seed {seed} at melanin {melanin}: {colour:?} on skin {tone:?}"
                    );
                }
            }
        }
    }
}
