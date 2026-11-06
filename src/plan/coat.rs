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
//!
//! **And the look, where a creator asks for one** (#401). The loci make a preset:
//! beside them [`CoatColours`] holds a colour per channel a creator has picked,
//! the way eyes hold theirs, each absent until picked and absent meaning exactly
//! what the loci derive. What each one reaches is stated on [`CoatColours`]. A rare
//! roll picks them too, as a creator would, a pair off a short list (#403).

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
    /// The colours a creator has picked over what all of the above derive
    /// (#401), or a rare roll has (#403). Left off the wire while none is.
    #[serde(skip_serializing_if = "CoatColours::is_empty")]
    pub colours: CoatColours,
}

/// One of the colours a creator can pick for a coat (#401): see [`CoatColours`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoatChannel {
    /// The dark pigment.
    Dark,
    /// The light pigment.
    Light,
    /// White markings.
    White,
    /// The nose and the pads.
    Leather,
    /// The claws and the hooves.
    Horn,
    /// The mane and the forelock.
    Mane,
    /// The tail.
    Tail,
}

impl CoatChannel {
    /// Every channel, in the order the wire's mask counts them.
    pub const ALL: [Self; 7] = [
        Self::Dark,
        Self::Light,
        Self::White,
        Self::Leather,
        Self::Horn,
        Self::Mane,
        Self::Tail,
    ];

    /// Its name on a record.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::White => "white",
            Self::Leather => "leather",
            Self::Horn => "horn",
            Self::Mane => "mane",
            Self::Tail => "tail",
        }
    }
}

/// The colours a creator has picked for a coat (#401), or a rare roll has for it
/// (#403), in sRGB, each absent until picked. The loci stay the presets that fill in whatever is absent; marks keep
/// their layout, since `marks`, the pattern and the white pattern still say where
/// everything goes, and only what they are drawn in changes.
///
/// **A picked colour wins outright over what the loci derive**: `dilute`,
/// `intensity` and `fantasy` do not touch it. A channel derived from another stays
/// derived until it is picked itself: a red coat's dark is its light deepened,
/// whichever light that is, and a fantasy coat's leather its dark's.
///
/// What each one reaches, settled by render:
///
/// - **`dark`** every mark, tip and point drawn in the dark, ear tufts, a dark
///   coat's whole ground, and a bay's or a dun's long hair unless that is picked.
/// - **`light`** a tan, a bay's body, a tabby's ground and a red coat's whole
///   ground. Past that it takes over the places the loci only lean toward a pale
///   of it: a colourpoint's paled body, a dun's diluted body, and a dark merle's
///   diluted ground.
/// - **`white`** white spotting and a roan's white hairs, both of which are white
///   hair. Not a colourpoint's paled body, which is `light`'s, and not a dapple's
///   lighter rounds, which are no marking.
/// - **`leather`** the nose and the pads, under white as well: a pad goes pink
///   under a white foot only while it is unpicked.
/// - **`horn`** claws and hooves, likewise under a white sock. A unicorn's horn
///   stays ivory: it is drawn with the whiskers.
/// - **`mane`** the mane, forelock and all, exactly, and the skin under the crest's
///   line so that no seam shows. A beast with no mane draws nothing of it. Unpicked,
///   a mane is the coat's own long hair in one colour (#407).
/// - **`tail`** the tail, skin and cards, fading in from the rump.
///
/// A dapple is not greyed where `dark` or `light` is picked, only given its rounds:
/// the grey would otherwise be most of what shows. Countershading goes on applying
/// to `dark` and `light`, at most 15 % toward a pale under the belly, because it
/// is what shapes a coat in one colour; the others are not shaded at all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CoatColours {
    /// The dark pigment.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub dark: Option<[f32; 3]>,
    /// The light pigment.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub light: Option<[f32; 3]>,
    /// White markings.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub white: Option<[f32; 3]>,
    /// The nose and the pads.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub leather: Option<[f32; 3]>,
    /// The claws and the hooves.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub horn: Option<[f32; 3]>,
    /// The mane and the forelock.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub mane: Option<[f32; 3]>,
    /// The tail.
    #[serde(
        with = "super::scaled::chosen",
        skip_serializing_if = "Option::is_none"
    )]
    pub tail: Option<[f32; 3]>,
}

impl CoatColours {
    /// Whether nothing is picked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The colour picked for `channel`, if one is.
    #[must_use]
    pub fn get(&self, channel: CoatChannel) -> Option<[f32; 3]> {
        match channel {
            CoatChannel::Dark => self.dark,
            CoatChannel::Light => self.light,
            CoatChannel::White => self.white,
            CoatChannel::Leather => self.leather,
            CoatChannel::Horn => self.horn,
            CoatChannel::Mane => self.mane,
            CoatChannel::Tail => self.tail,
        }
    }

    /// Where `channel`'s colour is kept.
    pub fn get_mut(&mut self, channel: CoatChannel) -> &mut Option<[f32; 3]> {
        match channel {
            CoatChannel::Dark => &mut self.dark,
            CoatChannel::Light => &mut self.light,
            CoatChannel::White => &mut self.white,
            CoatChannel::Leather => &mut self.leather,
            CoatChannel::Horn => &mut self.horn,
            CoatChannel::Mane => &mut self.mane,
            CoatChannel::Tail => &mut self.tail,
        }
    }

    /// What a roll picks for a coat (#403): nothing, on most seeds, and otherwise a
    /// pair off [`ROLLED_COLOURS`] with its accent, sometimes, on the long hair. Three
    /// draws on streams of their own, even among the pairs.
    #[must_use]
    pub(crate) fn rolled(rolls: &Rolls) -> Self {
        if rolls.range("coat.colours", 0.0, 1.0) >= FREE_COLOUR_SHARE {
            return Self::default();
        }
        let pair = ROLLED_COLOURS[rolls.pick("coat.coloursPair", &[1.0; ROLLED_COLOURS.len()])];
        let accent = Some(pair.accent);
        let (mane, tail) = match rolls.pick("coat.coloursAccent", &ACCENT_ODDS) {
            0 => (None, None),
            1 => (accent, accent),
            2 => (accent, None),
            _ => (None, accent),
        };
        Self {
            dark: Some(pair.dark),
            light: Some(pair.light),
            mane,
            tail,
            ..Self::default()
        }
    }

    /// Clamps every channel to `0..=1` and puts it on the wire's grid. A channel
    /// that is not a number reads as black rather than as unpicked: what was asked
    /// for is the record's, and the coat a reader would otherwise fall back to is
    /// not what it asked for either.
    pub fn sanitize(&mut self) {
        for channel in CoatChannel::ALL {
            if let Some(colour) = self.get_mut(channel) {
                *colour = colour.map(|part| super::sanitize_axis(part, 0.0, (0.0, 1.0)));
            }
        }
    }

    /// Its bytes on a share code: a mask of which channels are picked, in
    /// [`CoatChannel::ALL`]'s order, then a byte a part for each that is.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        let mask = CoatChannel::ALL
            .iter()
            .enumerate()
            .filter(|(_, channel)| self.get(**channel).is_some())
            .fold(0u8, |mask, (bit, _)| mask | 1 << bit);
        out.push(mask);
        for channel in CoatChannel::ALL {
            for part in self.get(channel).into_iter().flatten() {
                super::put_unit(out, part);
            }
        }
    }

    /// Them back off a share code. The mask's top bit is no channel yet, and
    /// reads as nothing.
    pub(crate) fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mask = take(bytes)?;
        let mut colours = Self::default();
        for (bit, channel) in CoatChannel::ALL.into_iter().enumerate() {
            if mask & 1 << bit != 0 {
                *colours.get_mut(channel) = Some([
                    super::take_unit(bytes)?,
                    super::take_unit(bytes)?,
                    super::take_unit(bytes)?,
                ]);
            }
        }
        colours.sanitize();
        Ok(colours)
    }
}

/// The share of re-rolled coats that wear free colours.
/// Provenance: **the owner's call** (#403), for the owner to judge from a sheet: about
/// one roll in twenty. Drawn on a stream of its own, `coat.colours`, so every coat it
/// misses is what it was to the byte.
const FREE_COLOUR_SHARE: f32 = 1.0 / 20.0;

/// How often a rolled pair's accent shows: not at all, on the mane and the tail, on
/// the mane alone, on the tail alone.
/// Provenance: **chosen** (#403): half plain, as a coat in two colours is look enough.
const ACCENT_ODDS: [f32; 4] = [0.5, 0.3, 0.1, 0.1];

/// One combination a roll can pick, as a creator would on purpose.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RolledPair {
    /// The dark pigment: a dark coat's ground and everybody's marks.
    dark: [f32; 3],
    /// The light pigment: a red coat's ground, a tan, a bay's body, a belly.
    light: [f32; 3],
    /// What its long hair can be drawn in instead.
    accent: [f32; 3],
}

/// The combinations a roll picks among (#403).
///
/// **A pair rather than a colour, as the loci say which of the two a body is mostly
/// drawn in**: a black or a liver coat shows `dark` with `light` on its belly and its
/// tan, a red coat `light` with `dark` in its marks, and a bay `light` with `dark` on
/// its points. So both of a pair are one harmony, the deep and the pale of one hue or a
/// neutral beside it, and either way round reads as meant. An accent is a contrast
/// chosen for the pair. Leather and horn stay the loci's, which is what keeps a blue
/// beast an animal, and white stays white.
///
/// Provenance: **chosen, and judged by render** (#403). `dark` is the darker of each.
const ROLLED_COLOURS: [RolledPair; 12] = [
    // A black coat with a white mane.
    RolledPair {
        dark: [0.07, 0.07, 0.08],
        light: [0.24, 0.24, 0.26],
        accent: [0.93, 0.92, 0.89],
    },
    // A deep blue with a pale belly.
    RolledPair {
        dark: [0.09, 0.14, 0.36],
        light: [0.70, 0.78, 0.90],
        accent: [0.86, 0.90, 0.96],
    },
    // Silver and white, with a charcoal mane.
    RolledPair {
        dark: [0.56, 0.58, 0.62],
        light: [0.90, 0.90, 0.88],
        accent: [0.12, 0.12, 0.14],
    },
    // Burgundy over rose.
    RolledPair {
        dark: [0.30, 0.06, 0.10],
        light: [0.72, 0.42, 0.38],
        accent: [0.12, 0.05, 0.06],
    },
    // Forest and moss, with a gold mane.
    RolledPair {
        dark: [0.08, 0.22, 0.14],
        light: [0.62, 0.70, 0.46],
        accent: [0.85, 0.76, 0.42],
    },
    // Copper and ink.
    RolledPair {
        dark: [0.16, 0.12, 0.20],
        light: [0.72, 0.38, 0.16],
        accent: [0.10, 0.08, 0.14],
    },
    // Slate teal and sea, with a cream mane.
    RolledPair {
        dark: [0.10, 0.26, 0.32],
        light: [0.62, 0.80, 0.78],
        accent: [0.93, 0.86, 0.66],
    },
    // Plum and lilac.
    RolledPair {
        dark: [0.24, 0.12, 0.36],
        light: [0.76, 0.66, 0.86],
        accent: [0.93, 0.84, 0.93],
    },
    // Chocolate and gold, with a flaxen mane: a palomino's, richer.
    RolledPair {
        dark: [0.26, 0.15, 0.08],
        light: [0.90, 0.70, 0.30],
        accent: [0.97, 0.92, 0.76],
    },
    // Steel blue and pearl, with a midnight mane.
    RolledPair {
        dark: [0.34, 0.42, 0.52],
        light: [0.78, 0.82, 0.86],
        accent: [0.12, 0.16, 0.24],
    },
    // Rose and blush.
    RolledPair {
        dark: [0.52, 0.20, 0.30],
        light: [0.96, 0.80, 0.76],
        accent: [0.99, 0.93, 0.85],
    },
    // Ash and charcoal, with an ember mane.
    RolledPair {
        dark: [0.14, 0.14, 0.15],
        light: [0.50, 0.50, 0.50],
        accent: [0.85, 0.30, 0.10],
    },
];

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
            colours: CoatColours::default(),
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

    /// A fruit bat's (#423): a dark brown or a black, now and then sable
    /// (darker along the back over a rich russet, which on a crawling bat whose
    /// back its folded wings cover reads as a golden bat, so one in seven),
    /// and almost never white. Provenance: **chosen by render**; research-10 5.3
    /// gives a flying fox's colours, not their frequencies.
    const BAT: Self = Self {
        pigment: [0.38, 0.57, 0.05],
        dilute: 0.05,
        agouti: [0.75, 0.15, 0.10, 0.0, 0.0],
        pattern: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        points: 0.0,
        white: [0.97, 0.0, 0.0, 0.02, 0.01],
        merle: 0.0,
        roan: 0.0,
        dun: 0.0,
        tortie: 0.0,
        intensity: (0.6, 0.4),
    };

    /// A sugar glider's (#423): mostly blue-grey, a black's dilution, some
    /// grizzled; now and then a mosaic's white patches or a leucistic
    /// glider's white all over. Provenance: **chosen by render** after
    /// research-10 5.3 (S31: grey above, cream below).
    const GLIDER: Self = Self {
        pigment: [0.85, 0.05, 0.10],
        dilute: 0.80,
        agouti: [0.75, 0.0, 0.25, 0.0, 0.0],
        pattern: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        points: 0.0,
        white: [0.88, 0.0, 0.0, 0.08, 0.04],
        merle: 0.0,
        roan: 0.0,
        dun: 0.0,
        tortie: 0.0,
        intensity: (-0.5, 0.4),
    };

    /// A winged family's loci, and none for a bird, which grows feathers.
    fn of_winged(family: super::WingedFamily) -> Option<Self> {
        match family {
            super::WingedFamily::Bird => None,
            super::WingedFamily::Bat => Some(Self::BAT),
            super::WingedFamily::Glider => Some(Self::GLIDER),
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
    /// Clamps `intensity` to its envelope and the picked colours to theirs; every
    /// other field is a state.
    pub fn sanitize(&mut self) {
        self.intensity =
            super::sanitize_axis(self.intensity, 0.0, super::explore_range(0.0, (-1.0, 1.0)));
        self.colours.sanitize();
    }

    /// Draws a fresh coat for a body of `family`, crossed with `hybrid`
    /// `blend` of the way, each locus on a stream of its own.
    ///
    /// Picked colours go with the coat they were picked over (#401), as a roll of
    /// the colouring draws fresh eyes over the ones picked before it, and what the
    /// seed draws takes their place, on most seeds nothing (#403): a creator who wants
    /// to keep them locks the colouring. Nothing else a roll does touches them.
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
        self.draw(rolls, &odds);
    }

    /// Draws a fresh coat for a winged body (#423) as [`Self::reroll`] draws a
    /// beast's, on the same streams, from a fruit bat's and a sugar glider's
    /// loci: a bat mostly a dark brown or a black, often tipped darker along
    /// the back, a glider mostly blue-grey, sometimes grizzled, now and then a
    /// mosaic's white patches or a leucistic glider's white - and, as a beast
    /// does, a rare unnatural palette or free colours. A bird has no coat to
    /// draw from, so a cross with one draws its other family's loci alone.
    pub(crate) fn reroll_winged(
        &mut self,
        rolls: &Rolls,
        family: super::WingedFamily,
        hybrid: Option<super::WingedFamily>,
        blend: f32,
    ) {
        let blend = blend.clamp(0.0, 1.0);
        let odds = match (
            Frequencies::of_winged(family),
            hybrid.and_then(Frequencies::of_winged),
        ) {
            (Some(own), Some(other)) => own.toward(other, blend),
            (Some(only), None) | (None, Some(only)) => only,
            (None, None) => Frequencies::BAT,
        };
        self.draw(rolls, &odds);
    }

    /// Draws every locus against `odds`, each on a stream of its own.
    fn draw(&mut self, rolls: &Rolls, odds: &Frequencies) {
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
        self.colours = CoatColours::rolled(rolls);
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
                    // Picked colours go in a block of their own, at the code's end.
                    Coat {
                        intensity: coat.intensity,
                        colours: coat.colours,
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

    /// Every channel picked, each its own colour, off the wire's grid.
    fn every_channel() -> CoatColours {
        let mut colours = CoatColours::default();
        for (index, channel) in CoatChannel::ALL.into_iter().enumerate() {
            let part = |at: usize| ((index * 37 + at * 91 + 5) % 255) as f32 / 255.0;
            *colours.get_mut(channel) = Some([part(0), part(1), part(2)]);
        }
        colours
    }

    #[test]
    fn a_coat_that_picks_nothing_writes_no_colours_and_reads_as_none() {
        let coat = Coat::default();
        let json = serde_json::to_string(&coat).expect("writes");
        assert!(!json.contains("colours"), "{json}");
        let back: Coat = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, coat);
        // A key a coat never had is a coat that picked nothing.
        let old: Coat = serde_json::from_str(r#"{"pigment":"black"}"#).expect("reads");
        assert!(old.colours.is_empty());
    }

    #[test]
    fn picked_colours_are_thousandths_on_the_wire_and_settle_at_once() {
        let mut coat = Coat {
            colours: every_channel(),
            ..Coat::default()
        };
        coat.sanitize();
        let json = serde_json::to_string(&coat).expect("writes");
        let wire: serde_json::Value = serde_json::from_str(&json).expect("parses");
        for channel in CoatChannel::ALL {
            let parts = wire["colours"][channel.name()].as_array().expect("written");
            assert_eq!(parts.len(), 3);
            assert!(parts.iter().all(serde_json::Value::is_i64), "{json}");
        }
        let mut back: Coat = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, coat, "one round trip is stable");
        back.sanitize();
        assert_eq!(back, coat, "and sanitizing moves nothing");
        assert_eq!(serde_json::to_string(&back).expect("writes"), json);
        // Only what was picked.
        let one = Coat {
            colours: CoatColours {
                mane: Some([0.25, 0.5, 0.75]),
                ..CoatColours::default()
            },
            ..Coat::default()
        };
        let json = serde_json::to_string(&one).expect("writes");
        assert!(
            json.contains(r#""colours":{"mane":[250,500,750]}"#),
            "{json}"
        );
    }

    #[test]
    fn a_picked_colour_out_of_range_is_brought_into_it() {
        let mut colours = CoatColours {
            dark: Some([1.7, -0.2, f32::NAN]),
            tail: Some([0.123_456, 0.5, 1.0]),
            ..CoatColours::default()
        };
        colours.sanitize();
        assert_eq!(colours.dark, Some([1.0, 0.0, 0.0]));
        assert_eq!(colours.tail, Some([0.123, 0.5, 1.0]));
        assert_eq!(colours.light, None, "and an absent one stays absent");
    }

    #[test]
    fn picked_colours_survive_their_bytes_and_their_bytes_are_stable() {
        for mask in 0..128u8 {
            let mut colours = every_channel();
            for (bit, channel) in CoatChannel::ALL.into_iter().enumerate() {
                if mask & 1 << bit == 0 {
                    *colours.get_mut(channel) = None;
                }
            }
            let mut out = Vec::new();
            colours.encode(&mut out);
            assert_eq!(out.len(), 1 + 3 * mask.count_ones() as usize);
            assert_eq!(out[0], mask);
            let back = CoatColours::decode(&mut out.as_slice()).expect("decodes");
            for channel in CoatChannel::ALL {
                match (colours.get(channel), back.get(channel)) {
                    (None, None) => {}
                    (Some(a), Some(b)) => {
                        for (a, b) in a.iter().zip(b) {
                            assert!((a - b).abs() < 0.001, "{channel:?}: {a} came back {b}");
                        }
                    }
                    other => panic!("{channel:?}: {other:?}"),
                }
            }
            // A byte, read and sanitized onto the record's grid, is the same byte.
            let mut again = Vec::new();
            back.encode(&mut again);
            assert_eq!(again, out);
        }
        let short = [0b0000_0001, 1, 2];
        assert!(CoatColours::decode(&mut short.as_slice()).is_err());
        // A channel this build does not know reads as nothing.
        let future = [0b1000_0000];
        assert!(
            CoatColours::decode(&mut future.as_slice())
                .expect("decodes")
                .is_empty()
        );
    }

    /// The first seed from 0 that does or does not draw free colours.
    fn seed_drawing(picked: bool) -> i64 {
        (0..)
            .find(|&seed| CoatColours::rolled(&Rolls::new(seed)).is_empty() != picked)
            .expect("in reach")
    }

    #[test]
    fn a_roll_of_the_coat_puts_what_its_seed_draws_over_picked_colours() {
        let reroll = |seed: i64| {
            let mut coat = Coat {
                colours: every_channel(),
                ..Coat::default()
            };
            coat.reroll(&Rolls::new(seed), Family::Dog, None, 0.0);
            coat.colours
        };
        assert!(reroll(seed_drawing(false)).is_empty());
        let seed = seed_drawing(true);
        assert!(!reroll(seed).is_empty());
        assert_eq!(reroll(seed), CoatColours::rolled(&Rolls::new(seed)));
    }

    #[test]
    fn a_small_share_of_rolls_pick_a_pair_off_the_list_with_an_accent_sometimes() {
        const SEEDS: i64 = 16_000;
        let mut pairs = [0usize; ROLLED_COLOURS.len()];
        let mut shown = [0usize; 4];
        for seed in 0..SEEDS {
            let colours = CoatColours::rolled(&Rolls::new(seed));
            if colours.is_empty() {
                continue;
            }
            let at = ROLLED_COLOURS
                .iter()
                .position(|pair| {
                    colours.dark == Some(pair.dark) && colours.light == Some(pair.light)
                })
                .unwrap_or_else(|| panic!("{seed} picked no pair on the list: {colours:?}"));
            pairs[at] += 1;
            for long_hair in [colours.mane, colours.tail].into_iter().flatten() {
                assert_eq!(long_hair, ROLLED_COLOURS[at].accent, "{seed}");
            }
            shown[match (colours.mane, colours.tail) {
                (None, None) => 0,
                (Some(_), Some(_)) => 1,
                (Some(_), None) => 2,
                (None, Some(_)) => 3,
            }] += 1;
            assert_eq!(colours.white, None, "white stays white");
            assert_eq!(colours.leather, None, "and leather the loci's");
            assert_eq!(colours.horn, None, "and horn");
        }
        let picked: usize = pairs.iter().sum();
        let share = picked as f32 / SEEDS as f32;
        // Sampling error at this count is 0.0017; the margin is a quarter of the share.
        assert!(
            (share - FREE_COLOUR_SHARE).abs() < FREE_COLOUR_SHARE * 0.25,
            "{share} of rolls pick colours, for a share of {FREE_COLOUR_SHARE}"
        );
        let even = picked as f32 / pairs.len() as f32;
        assert!(
            pairs.iter().all(|&n| (n as f32 - even).abs() < even * 0.6),
            "pairs drawn {pairs:?}"
        );
        let total: f32 = ACCENT_ODDS.iter().sum();
        for (count, odds) in shown.iter().zip(ACCENT_ODDS) {
            let at = *count as f32 / picked as f32;
            assert!((at - odds / total).abs() < 0.08, "accents {shown:?}");
        }
    }

    #[test]
    fn every_rolled_pair_is_on_the_grid_dark_over_light_and_survives_its_round_trips() {
        // The perceived weight sRGB carries, near enough to say which is the deeper.
        let weight = |colour: [f32; 3]| 0.3 * colour[0] + 0.59 * colour[1] + 0.11 * colour[2];
        for (index, pair) in ROLLED_COLOURS.iter().enumerate() {
            assert!(
                weight(pair.dark) < weight(pair.light),
                "pair {index} is the wrong way round"
            );
            for mane in [None, Some(pair.accent)] {
                let colours = CoatColours {
                    dark: Some(pair.dark),
                    light: Some(pair.light),
                    mane,
                    tail: Some(pair.accent),
                    ..CoatColours::default()
                };
                let mut settled = colours;
                settled.sanitize();
                assert_eq!(settled, colours, "pair {index} is off the grid");
                let text = serde_json::to_string(&colours).expect("writes");
                assert_eq!(
                    serde_json::from_str::<CoatColours>(&text).expect("reads"),
                    colours
                );
                let mut bytes = Vec::new();
                colours.encode(&mut bytes);
                let back = CoatColours::decode(&mut bytes.as_slice()).expect("reads");
                let mut again = Vec::new();
                back.encode(&mut again);
                assert_eq!(again, bytes, "pair {index} on a code");
            }
        }
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
