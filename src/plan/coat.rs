//! A beast's coat (#368): what colour its fur is and how it is marked,
//! stored as coat-genetics loci.
//!
//! **The loci, not the look.** A coat is recorded the way a breeder would
//! describe it - the pigment, whether it is diluted, how the agouti gene
//! spreads the dark and the light, a pattern, how much white - and
//! `texture::coat` derives the colours from them. So a coat stays a few small
//! integers on the wire, a cross can take its loci from either parent, and a
//! creator changes a coat the way it varies in life: a liver dog is the black
//! dog's coat with one locus turned, not a new colour picked by eye.
//!
//! The genetics are a game's, not a geneticist's: each locus has the handful
//! of states a coat needs to read, and loci that only one family carries in
//! life - merle, roan, dun, tortie, the cat's points - are open to every
//! family, which is what a cross wants. Every family rolls its own
//! frequencies; the unnatural palettes (`Fantasy`) are rolled as a rarity,
//! the owner's call (#364).

use serde::{Deserialize, Serialize};

use super::{Family, PlanDecodeError, Rolls};

/// The pigment a coat is built on: black or liver eumelanin, or red
/// phaeomelanin alone (a ginger cat, a chestnut horse, a red setter).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pigment {
    /// Black eumelanin.
    Black,
    /// Brown eumelanin: liver, chocolate.
    Liver,
    /// No eumelanin in the coat: red, ginger, chestnut, cream.
    #[default]
    Red,
}

named!(Pigment: Black => "black", Liver => "liver", Red => "red");

/// How the dark and the light are spread over the body: the agouti locus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Agouti {
    /// One colour all over.
    #[default]
    Solid,
    /// A light coat tipped dark along the back: sable.
    Sable,
    /// Every hair banded dark and light, a grizzle: the wild type, and the
    /// ground a tabby's marks are drawn on.
    Wild,
    /// A dark coat with light points: the muzzle, brows, chest, legs and the
    /// underside of the tail - a black-and-tan.
    TanPoints,
    /// A light coat with dark points: the legs, ear rims and the edge of the
    /// muzzle - a bay.
    Bay,
}

named!(Agouti: Solid => "solid", Sable => "sable", Wild => "wild", TanPoints => "tanPoints", Bay => "bay");

/// Marks drawn over the coat in its dark pigment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// None.
    #[default]
    None,
    /// A tabby's narrow stripes round the body, rings on the legs and tail.
    Mackerel,
    /// A tabby's swirls, a bullseye on each flank.
    Blotched,
    /// A tabby's ticked coat: the grizzle alone, faint bars on the legs.
    Ticked,
    /// Round spots: a spotted tabby, a dalmatian, an appaloosa.
    Spotted,
    /// Rings round a lighter centre: a leopard's, a Bengal's.
    Rosettes,
    /// Thin dark stripes, broken and wavering: a brindle.
    Brindle,
    /// Lighter rings netted over a grey: a dapple.
    Dapple,
}

named!(Pattern: None => "none", Mackerel => "mackerel", Blotched => "blotched", Ticked => "ticked", Spotted => "spotted", Rosettes => "rosettes", Brindle => "brindle", Dapple => "dapple");

/// How much white the coat carries: the white spotting locus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum White {
    /// None.
    #[default]
    None,
    /// White feet.
    Socks,
    /// White feet, chest, a blaze down the face and a white tail tip: the
    /// Irish pattern, a tuxedo cat's.
    Irish,
    /// Large white patches, most of the underside: a piebald, a paint.
    Piebald,
    /// White nearly all over, colour left on the head and the base of the
    /// tail.
    Extreme,
}

named!(White: None => "none", Socks => "socks", Irish => "irish", Piebald => "piebald", Extreme => "extreme");

/// An unnatural palette (#364: the owner wants them): the coat's dark and
/// light drawn in a pair of colours no animal grows. None on a natural coat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Fantasy {
    /// A natural coat.
    #[default]
    None,
    /// Blues.
    Sky,
    /// Greens.
    Mint,
    /// Lilacs.
    Lilac,
    /// Pinks.
    Rose,
    /// Golds and yellows.
    Sunny,
    /// Deep teals.
    Ocean,
}

named!(Fantasy: None => "none", Sky => "sky", Mint => "mint", Lilac => "lilac", Rose => "rose", Sunny => "sunny", Ocean => "ocean");

/// A beast's coat: its loci, how rich its red runs, an unnatural palette if
/// it wears one, and the seed its marks are laid out from.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Coat {
    /// The pigment the coat is built on.
    pub pigment: Pigment,
    /// Whether it is diluted: black to blue-grey, liver to lilac, red to
    /// cream.
    pub dilute: bool,
    /// How its dark and light are spread.
    pub agouti: Agouti,
    /// Marks drawn over it.
    pub pattern: Pattern,
    /// Whether its points - face, ears, legs, tail - are darker than its
    /// body: a colourpoint, a Siamese.
    pub points: bool,
    /// How much white it carries.
    pub white: White,
    /// Whether its dark is broken into patches over a diluted ground: merle.
    pub merle: bool,
    /// Whether white hairs are scattered through its body: roan.
    pub roan: bool,
    /// Whether its body is diluted with a dark stripe down the back and
    /// barred legs: dun.
    pub dun: bool,
    /// Whether red and dark are mixed in patches: a tortoiseshell.
    pub tortie: bool,
    /// How rich its red runs: cream at -1, a setter's red at 1, and past
    /// them ivory at -3 and mahogany at 3.
    #[serde(with = "super::scaled")]
    pub intensity: f32,
    /// An unnatural palette, if it wears one.
    pub fantasy: Fantasy,
    /// The seed its spots, stripes and patches are laid out from, so a coat's
    /// marks survive the wire and can be re-thrown on their own.
    pub marks: u16,
}

/// A warm fawn, unmarked: the neutral beast.
impl Default for Coat {
    fn default() -> Self {
        Self {
            pigment: Pigment::Red,
            dilute: false,
            agouti: Agouti::Solid,
            pattern: Pattern::None,
            points: false,
            white: White::None,
            merle: false,
            roan: false,
            dun: false,
            tortie: false,
            intensity: 0.0,
            fantasy: Fantasy::None,
            marks: 0,
        }
    }
}

/// The share of re-rolled coats that wear an unnatural palette: a rarity,
/// the owner's call (#364).
const FANTASY_SHARE: f32 = 0.04;

/// How often a family rolls each state of each locus.
#[derive(Clone, Copy, Debug)]
struct Frequencies {
    pigment: [f32; 3],
    dilute: f32,
    agouti: [f32; 5],
    pattern: [f32; 8],
    points: f32,
    white: [f32; 5],
    merle: f32,
    roan: f32,
    dun: f32,
    tortie: f32,
    /// Where `intensity` centres, and how widely it spreads.
    intensity: (f32, f32),
}

impl Frequencies {
    /// A family's own. Every row sums to one.
    /// Provenance: **chosen** (#368), by the coats each family is known
    /// by: a dog's are anything, a cat's mostly tabby, a horse's mostly bay
    /// and chestnut with white on the legs.
    fn of(family: Family) -> Self {
        match family {
            Family::Dog => Self {
                pigment: [0.40, 0.20, 0.40],
                dilute: 0.10,
                agouti: [0.35, 0.25, 0.05, 0.30, 0.05],
                pattern: [0.80, 0.0, 0.0, 0.0, 0.08, 0.0, 0.12, 0.0],
                points: 0.0,
                white: [0.35, 0.20, 0.25, 0.15, 0.05],
                merle: 0.08,
                roan: 0.0,
                dun: 0.0,
                tortie: 0.0,
                intensity: (0.0, 0.6),
            },
            Family::Cat => Self {
                pigment: [0.65, 0.05, 0.30],
                dilute: 0.20,
                agouti: [0.35, 0.0, 0.65, 0.0, 0.0],
                pattern: [0.30, 0.30, 0.20, 0.07, 0.08, 0.05, 0.0, 0.0],
                points: 0.12,
                white: [0.50, 0.15, 0.15, 0.15, 0.05],
                merle: 0.0,
                roan: 0.0,
                dun: 0.0,
                tortie: 0.15,
                intensity: (0.2, 0.5),
            },
            Family::Horse => Self {
                pigment: [0.45, 0.0, 0.55],
                dilute: 0.15,
                agouti: [0.40, 0.0, 0.0, 0.0, 0.60],
                pattern: [0.75, 0.0, 0.0, 0.0, 0.10, 0.0, 0.0, 0.15],
                points: 0.0,
                white: [0.35, 0.35, 0.20, 0.08, 0.02],
                merle: 0.0,
                roan: 0.08,
                dun: 0.10,
                tortie: 0.0,
                intensity: (0.2, 0.5),
            },
        }
    }

    /// `share` of the way from `self` to `other`, state by state: a cross's.
    fn toward(self, other: Self, share: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * share;
        let rows = |a: &[f32], b: &[f32], out: &mut [f32]| {
            for ((out, a), b) in out.iter_mut().zip(a).zip(b) {
                *out = mix(*a, *b);
            }
        };
        let mut mixed = self;
        rows(&self.pigment, &other.pigment, &mut mixed.pigment);
        rows(&self.agouti, &other.agouti, &mut mixed.agouti);
        rows(&self.pattern, &other.pattern, &mut mixed.pattern);
        rows(&self.white, &other.white, &mut mixed.white);
        mixed.dilute = mix(self.dilute, other.dilute);
        mixed.points = mix(self.points, other.points);
        mixed.merle = mix(self.merle, other.merle);
        mixed.roan = mix(self.roan, other.roan);
        mixed.dun = mix(self.dun, other.dun);
        mixed.tortie = mix(self.tortie, other.tortie);
        mixed.intensity = (
            mix(self.intensity.0, other.intensity.0),
            self.intensity.1.max(other.intensity.1),
        );
        mixed
    }
}

/// The state a draw in `0..1` lands on, `weights` being each state's share.
fn pick<T: Copy>(draw: f32, weights: &[f32], states: &[T]) -> T {
    let total: f32 = weights.iter().sum();
    let mut left = draw * total;
    for (&weight, &state) in weights.iter().zip(states) {
        if left < weight {
            return state;
        }
        left -= weight;
    }
    states[states.len() - 1]
}

impl Coat {
    /// Clamps `intensity` to its envelope; every other field is a state.
    pub fn sanitize(&mut self) {
        self.intensity =
            super::sanitize_axis(self.intensity, 0.0, super::explore_range(0.0, (-1.0, 1.0)));
    }

    /// Draws a fresh coat for a body of `family`, crossed with `hybrid`
    /// `blend` of the way, each locus on a stream of its own.
    pub(crate) fn reroll(
        &mut self,
        rolls: &Rolls,
        family: Family,
        hybrid: Option<Family>,
        blend: f32,
    ) {
        let own = Frequencies::of(family);
        let odds = match hybrid {
            Some(other) => own.toward(Frequencies::of(other), blend.clamp(0.0, 1.0)),
            None => own,
        };
        let chance = |stream: &str, share: f32| rolls.range(stream, 0.0, 1.0) < share;
        let state = |stream: &str| rolls.range(stream, 0.0, 1.0);
        self.pigment = pick(state("coat.pigment"), &odds.pigment, &Pigment::ALL);
        self.dilute = chance("coat.dilute", odds.dilute);
        self.agouti = pick(state("coat.agouti"), &odds.agouti, &Agouti::ALL);
        self.pattern = pick(state("coat.pattern"), &odds.pattern, &Pattern::ALL);
        self.points = chance("coat.points", odds.points);
        self.white = pick(state("coat.white"), &odds.white, &White::ALL);
        self.merle = chance("coat.merle", odds.merle);
        self.roan = chance("coat.roan", odds.roan);
        self.dun = chance("coat.dun", odds.dun);
        self.tortie = chance("coat.tortie", odds.tortie);
        let (mean, sigma) = odds.intensity;
        self.intensity = rolls.shape("coat.intensity", mean, sigma, (-1.0, 1.0));
        self.fantasy = if chance("coat.fantasy", FANTASY_SHARE) {
            let palettes = &Fantasy::ALL[1..];
            pick(state("coat.palette"), &[1.0; 6], palettes)
        } else {
            Fantasy::None
        };
        self.marks = (state("coat.marks") * f32::from(u16::MAX)) as u16;
    }

    /// Its bytes on a share code: the loci packed into three, then the
    /// intensity's span and the marks' two.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        let bits = u32::from(self.pigment.code())
            | u32::from(self.dilute) << 2
            | u32::from(self.agouti.code()) << 3
            | u32::from(self.pattern.code()) << 6
            | u32::from(self.points) << 9
            | u32::from(self.white.code()) << 10
            | u32::from(self.merle) << 13
            | u32::from(self.roan) << 14
            | u32::from(self.dun) << 15
            | u32::from(self.tortie) << 16
            | u32::from(self.fantasy.code()) << 17;
        out.extend_from_slice(&bits.to_le_bytes()[..3]);
        super::put_span(out, self.intensity, super::explore_range(0.0, (-1.0, 1.0)));
        out.extend_from_slice(&self.marks.to_le_bytes());
    }

    /// A coat back off a share code, a state this build does not know reading
    /// as its default.
    pub(crate) fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let bits = u32::from_le_bytes([take(bytes)?, take(bytes)?, take(bytes)?, 0]);
        let field = |shift: u32, width: u32| ((bits >> shift) & ((1 << width) - 1)) as u8;
        let flag = |shift: u32| field(shift, 1) == 1;
        let mut coat = Self {
            pigment: Pigment::from_code(field(0, 2)),
            dilute: flag(2),
            agouti: Agouti::from_code(field(3, 3)),
            pattern: Pattern::from_code(field(6, 3)),
            points: flag(9),
            white: White::from_code(field(10, 3)),
            merle: flag(13),
            roan: flag(14),
            dun: flag(15),
            tortie: flag(16),
            fantasy: Fantasy::from_code(field(17, 3)),
            ..Self::default()
        };
        coat.intensity = super::take_span(bytes, super::explore_range(0.0, (-1.0, 1.0)))?;
        coat.marks = u16::from_le_bytes([take(bytes)?, take(bytes)?]);
        coat.sanitize();
        Ok(coat)
    }
}

/// One raw byte off a share code.
fn take(bytes: &mut &[u8]) -> Result<u8, PlanDecodeError> {
    let (&byte, rest) = bytes.split_first().ok_or(PlanDecodeError::Truncated)?;
    *bytes = rest;
    Ok(byte)
}

/// An iris colour for a beast of `family` at `shade`, a draw in `0..=1`
/// (#368, from #367): a cat's amber, gold and green, a dog's browns and
/// ambers, a horse's dark browns - and the top of every family's range blue,
/// as a husky's and a Siamese's are.
/// Provenance: **chosen** (#368), by render.
#[must_use]
pub fn iris(family: Family, shade: f32) -> [f32; 3] {
    let stops: &[(f32, [f32; 3])] = match family {
        Family::Dog => &[
            (0.0, [0.18, 0.11, 0.06]),
            (0.45, [0.34, 0.21, 0.09]),
            (0.88, [0.62, 0.42, 0.14]),
            (0.92, [0.46, 0.62, 0.78]),
            (1.0, [0.52, 0.68, 0.84]),
        ],
        Family::Cat => &[
            (0.0, [0.72, 0.38, 0.12]),
            (0.35, [0.86, 0.66, 0.18]),
            (0.65, [0.66, 0.70, 0.22]),
            (0.88, [0.30, 0.60, 0.28]),
            (0.92, [0.42, 0.64, 0.90]),
            (1.0, [0.48, 0.70, 0.94]),
        ],
        Family::Horse => &[
            (0.0, [0.14, 0.09, 0.05]),
            (0.9, [0.30, 0.18, 0.09]),
            (0.95, [0.50, 0.64, 0.80]),
            (1.0, [0.54, 0.68, 0.84]),
        ],
    };
    let shade = shade.clamp(0.0, 1.0);
    for pair in stops.windows(2) {
        let ((from, a), (to, b)) = (pair[0], pair[1]);
        if shade <= to {
            let t = ((shade - from) / (to - from).max(f32::EPSILON)).clamp(0.0, 1.0);
            return [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ];
        }
    }
    stops[stops.len() - 1].1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolled(seed: i64, family: Family) -> Coat {
        let mut coat = Coat::default();
        coat.reroll(&Rolls::new(seed), family, None, 0.0);
        coat
    }

    #[test]
    fn a_coat_survives_its_bytes_and_unknown_states_read_as_defaults() {
        for seed in 0..200 {
            for family in Family::ALL {
                let coat = rolled(seed, family);
                let mut out = Vec::new();
                coat.encode(&mut out);
                assert_eq!(out.len(), 6, "three loci bytes, a span and the marks");
                let back = Coat::decode(&mut out.as_slice()).expect("decodes");
                assert!((back.intensity - coat.intensity).abs() < 0.013);
                assert_eq!(
                    Coat {
                        intensity: coat.intensity,
                        ..back
                    },
                    coat
                );
            }
        }
        // Every state field at its all-ones code: pigment 3, and agouti,
        // white and fantasy 7, none of which a build knows yet - and pattern
        // 7, which is a dapple's.
        let unknown = [0xFF, 0xFF, 0x0F, 128, 0, 0];
        let back = Coat::decode(&mut unknown.as_slice()).expect("decodes");
        assert_eq!(back.pigment, Pigment::default());
        assert_eq!(back.agouti, Agouti::default());
        assert_eq!(back.white, White::default());
        assert_eq!(back.fantasy, Fantasy::None);
        assert_eq!(back.pattern, Pattern::Dapple);
    }

    #[test]
    fn each_family_rolls_the_coats_it_is_known_by() {
        let count = |family: Family, test: &dyn Fn(&Coat) -> bool| {
            (0..400).filter(|&seed| test(&rolled(seed, family))).count()
        };
        // A horse is bay or solid, never sable or tabby-grounded.
        assert_eq!(
            count(Family::Horse, &|coat| matches!(
                coat.agouti,
                Agouti::Sable | Agouti::Wild
            )),
            0
        );
        assert!(count(Family::Horse, &|coat| coat.agouti == Agouti::Bay) > 150);
        // A cat is mostly tabby; merle and roan are not a cat's.
        assert!(count(Family::Cat, &|coat| coat.pattern != Pattern::None) > 200);
        assert_eq!(count(Family::Cat, &|coat| coat.merle || coat.roan), 0);
        // Unnatural palettes are rare in every family, and appear.
        let fantasies = Family::ALL
            .into_iter()
            .map(|family| count(family, &|coat| coat.fantasy != Fantasy::None))
            .sum::<usize>();
        assert!(
            (10..=100).contains(&fantasies),
            "{fantasies} fantasies in 1,200 rolls"
        );
    }

    #[test]
    fn a_cross_mixes_its_parents_odds() {
        let odds = Frequencies::of(Family::Cat).toward(Frequencies::of(Family::Dog), 0.5);
        assert!((odds.merle - 0.04).abs() < 1.0e-6);
        assert!((odds.tortie - 0.075).abs() < 1.0e-6);
        assert!((odds.pigment.iter().sum::<f32>() - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn every_family_has_its_own_eyes() {
        // A horse's middle is a dark brown, a cat's a gold, and blue sits at
        // the top of each range.
        let [r, g, b] = iris(Family::Cat, 0.4);
        assert!(r > 0.7 && g > 0.6 && b < 0.3, "a cat's gold: {r} {g} {b}");
        let [r, _, b] = iris(Family::Horse, 0.5);
        assert!(r < 0.3 && b < 0.1, "a horse's brown");
        for family in Family::ALL {
            let [r, _, b] = iris(family, 1.0);
            assert!(b > r, "{family:?}: the top of the range is blue");
        }
    }
}
