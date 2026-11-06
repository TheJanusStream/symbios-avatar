//! A bird's plumage (#423, W4): what colour its feathers are, tract by tract,
//! and how they are marked.
//!
//! **Colours by tract, not loci.** A beast's coat is recorded as the genetics a
//! breeder would name (`plan::coat`), because a coat varies in life the way its
//! loci do. A bird's does not: what separates an owl from a crow, a parrot or a
//! songbird is which colour each feather tract wears, and a field guide
//! describes a bird that way - its crown, its back, its breast, its flight
//! feathers, its tail (research-10 5.1). So a plumage holds a colour per tract
//! a bird reads by, a pattern drawn over them in a colour of its own, an owl's
//! disc and ruff, and how glossy it is: every colour free, as a beast's picked
//! colours are (#401), and a roll picks a whole set off a short list per kind of
//! bird, as a beast's roll picks its free colours off one (#403).
//!
//! **The kind is not stored.** A bird is an owl, a crow, a parrot or a songbird
//! only by where its head's axes stand (#422), so a roll reads the kind back from
//! them (`WingedParams::bird_kind`) - an owl's if the head has a disc, which only
//! an owl's roll draws, else the disc-less preset nearest it - and draws the
//! colours of that kind: a parrot-beaked bird wears a parrot's colours. A creator who
//! re-shapes the beak keeps the plumage they have; a plumage is the record's.
//!
//! Authored in sRGB, as every colour a record carries is, on the wire's
//! thousandths so a preset is exactly what it round-trips to.

use serde::{Deserialize, Serialize};

use super::{PlanDecodeError, Rolls};

/// How a plumage is marked: what its [`Plumage::mark`] colour draws.
///
/// **Drawn where it shows**: a mark lighter than a tract shows on it, one no
/// lighter does not, so a little owl's pale spots land on its brown back and
/// crown and not its buff breast, a thrush's dark ones on its pale breast and a
/// snowy owl's dark bars everywhere (`texture::coat::plumage`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlumagePattern {
    /// None: every tract its own colour.
    #[default]
    Plain,
    /// Bars across the body and the flight feathers: a snowy owl's, a cuckoo's,
    /// a budgerigar's back.
    Barred,
    /// Round spots: a little owl's pale ones, a thrush's dark ones.
    Spotted,
    /// Narrow streaks running down the body: a tawny owl's breast, a sparrow's
    /// back.
    Streaked,
}

named!(PlumagePattern: Plain => "plain", Barred => "barred", Spotted => "spotted", Streaked => "streaked");

/// A bird's plumage (#423): a colour per feather tract, a pattern, an owl's
/// disc and ruff, a gloss, and the seed its marks are laid out from.
///
/// **What each colour reaches**, on the skin's atlas and on every feather card
/// (`texture::coat::PlumageSampler`):
///
/// - **`head`** the crown, the nape, the cheeks and the throat: the capital
///   tract, outside an owl's disc.
/// - **`upper`** the back, the mantle and the rump, and the wing's coverts,
///   humerals and alula: the dorsal tracts and the alar coverts.
/// - **`under`** the breast, the belly, the flanks and the feathered legs: the
///   ventral tract, which the upper turns into round the flank.
/// - **`flight`** the remiges: primaries, secondaries and tertials.
/// - **`tail`** the rectrices; the upper tail coverts are `upper`'s.
/// - **`mark`** what [`Self::pattern`] draws in.
/// - **`disc`** an owl's face inside its ruff (#422's dish), and **`ruff`** the
///   rim round it: a pale face ringed darker is what reads as a facial disc at
///   a distance. Neither shows on a head with no disc.
///
/// The beak's horn and the bare toes are not plumage: the beak is drawn in the
/// horn its length gives it (#422) and the toes in the bird's scaled leather.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Plumage {
    /// The head, outside a disc.
    #[serde(with = "super::scaled::triple")]
    pub head: [f32; 3],
    /// The back and the wing's coverts.
    #[serde(with = "super::scaled::triple")]
    pub upper: [f32; 3],
    /// The breast, the belly and the legs.
    #[serde(with = "super::scaled::triple")]
    pub under: [f32; 3],
    /// The wing's flight feathers.
    #[serde(with = "super::scaled::triple")]
    pub flight: [f32; 3],
    /// The tail's feathers.
    #[serde(with = "super::scaled::triple")]
    pub tail: [f32; 3],
    /// What the pattern is drawn in.
    #[serde(with = "super::scaled::triple")]
    pub mark: [f32; 3],
    /// An owl's face, inside its ruff.
    #[serde(with = "super::scaled::triple")]
    pub disc: [f32; 3],
    /// An owl's ruff, round its disc.
    #[serde(with = "super::scaled::triple")]
    pub ruff: [f32; 3],
    /// How it is marked.
    pub pattern: PlumagePattern,
    /// How glossy the back, the head and the wings are, `0..=1`: none at
    /// nought, a raven's at one. Lowers the skin's roughness there, so its
    /// highlight tightens, and lays a cool sheen over its colour.
    #[serde(with = "super::scaled")]
    pub gloss: f32,
    /// The seed its spots, bars and streaks are laid out from.
    pub marks: u16,
}

impl Default for Plumage {
    /// The plan's default bird's: a little owl's (the owner's call, #418).
    fn default() -> Self {
        Self::OWL
    }
}

/// An sRGB triple, written short.
const fn rgb(r: f32, g: f32, b: f32) -> [f32; 3] {
    [r, g, b]
}

impl Plumage {
    /// A little owl (research-10 7.2, the plan's default bird): brown above,
    /// spotted with cream on the back and the crown, buff below, a pale face
    /// ringed dark brown. Its brown is W2's stand-in feathers'.
    pub const OWL: Self = Self {
        head: rgb(0.44, 0.33, 0.22),
        upper: rgb(0.42, 0.31, 0.21),
        under: rgb(0.76, 0.66, 0.52),
        flight: rgb(0.36, 0.27, 0.18),
        tail: rgb(0.38, 0.28, 0.19),
        mark: rgb(0.92, 0.88, 0.78),
        disc: rgb(0.84, 0.78, 0.66),
        ruff: rgb(0.28, 0.20, 0.13),
        pattern: PlumagePattern::Spotted,
        gloss: 0.0,
        marks: 0,
    };

    /// A carrion crow: black all over, glossed blue and violet on its back and
    /// wings (research-10 5.1: a corvid's).
    pub const CROW: Self = Self {
        head: rgb(0.07, 0.07, 0.08),
        upper: rgb(0.07, 0.07, 0.09),
        under: rgb(0.09, 0.09, 0.10),
        flight: rgb(0.06, 0.06, 0.08),
        tail: rgb(0.06, 0.06, 0.08),
        mark: rgb(0.07, 0.07, 0.09),
        disc: rgb(0.07, 0.07, 0.08),
        ruff: rgb(0.07, 0.07, 0.08),
        pattern: PlumagePattern::Plain,
        gloss: 0.8,
        marks: 0,
    };

    /// A scarlet macaw: red, its flight feathers blue, its long tail red.
    pub const PARROT: Self = Self {
        head: rgb(0.80, 0.10, 0.08),
        upper: rgb(0.78, 0.12, 0.08),
        under: rgb(0.74, 0.10, 0.08),
        flight: rgb(0.10, 0.28, 0.74),
        tail: rgb(0.76, 0.14, 0.10),
        mark: rgb(0.96, 0.80, 0.10),
        disc: rgb(0.80, 0.10, 0.08),
        ruff: rgb(0.80, 0.10, 0.08),
        pattern: PlumagePattern::Plain,
        gloss: 0.2,
        marks: 0,
    };

    /// An eastern bluebird: blue above, rust below.
    pub const SONGBIRD: Self = Self {
        head: rgb(0.20, 0.36, 0.74),
        upper: rgb(0.22, 0.38, 0.76),
        under: rgb(0.80, 0.46, 0.22),
        flight: rgb(0.16, 0.28, 0.62),
        tail: rgb(0.18, 0.30, 0.66),
        mark: rgb(0.92, 0.90, 0.86),
        disc: rgb(0.20, 0.36, 0.74),
        ruff: rgb(0.20, 0.36, 0.74),
        pattern: PlumagePattern::Plain,
        gloss: 0.1,
        marks: 0,
    };

    /// Every colour, in the order a share code writes them.
    fn colours(&self) -> [&[f32; 3]; 8] {
        [
            &self.head,
            &self.upper,
            &self.under,
            &self.flight,
            &self.tail,
            &self.mark,
            &self.disc,
            &self.ruff,
        ]
    }

    /// Every colour, to be changed.
    fn colours_mut(&mut self) -> [&mut [f32; 3]; 8] {
        [
            &mut self.head,
            &mut self.upper,
            &mut self.under,
            &mut self.flight,
            &mut self.tail,
            &mut self.mark,
            &mut self.disc,
            &mut self.ruff,
        ]
    }

    /// Clamps every colour to `0..=1` and the gloss to a share, on the wire's
    /// grid. A channel that is not a number reads as black, as a coat's picked
    /// colour does. Idempotent.
    pub fn sanitize(&mut self) {
        for colour in self.colours_mut() {
            *colour = colour.map(|part| super::sanitize_axis(part, 0.0, (0.0, 1.0)));
        }
        self.gloss = super::sanitize_axis(self.gloss, 0.0, (0.0, 1.0));
    }

    /// The preset of `kind`: the first of its palettes.
    pub(crate) fn of(kind: BirdKind) -> Self {
        palettes(kind)[0]
    }

    /// What a roll draws for a bird of `kind` (#423): one of its palettes,
    /// evenly, a little lighter or darker all over, and a fresh layout for its
    /// marks. Three draws on streams of their own.
    pub(crate) fn rolled(rolls: &Rolls, kind: BirdKind) -> Self {
        let palettes = palettes(kind);
        let mut plumage = palettes[rolls.pick("winged.plumage", &vec![1.0; palettes.len()])];
        let shade = rolls.range(
            "winged.plumageShade",
            1.0 - SHADE_SPREAD,
            1.0 + SHADE_SPREAD,
        );
        for colour in plumage.colours_mut() {
            *colour = colour.map(|part| part * shade);
        }
        plumage.marks = (rolls.range("winged.plumageMarks", 0.0, 1.0) * f32::from(u16::MAX)) as u16;
        plumage.sanitize();
        plumage
    }

    /// Its bytes on a share code: a byte a channel of each colour, the
    /// pattern, the gloss and the marks' seed.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        for colour in self.colours() {
            for &part in colour {
                super::put_unit(out, part);
            }
        }
        out.push(self.pattern.code());
        super::put_unit(out, self.gloss);
        out.extend_from_slice(&self.marks.to_le_bytes());
    }

    /// It back off a share code, sanitised.
    ///
    /// # Errors
    ///
    /// [`PlanDecodeError::Truncated`] when the code ends before it does.
    pub(crate) fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut plumage = Self::default();
        for colour in plumage.colours_mut() {
            for part in colour.iter_mut() {
                *part = super::take_unit(bytes)?;
            }
        }
        plumage.pattern = PlumagePattern::from_code(super::take_byte(bytes)?);
        plumage.gloss = super::take_unit(bytes)?;
        plumage.marks = u16::from_le_bytes([super::take_byte(bytes)?, super::take_byte(bytes)?]);
        plumage.sanitize();
        Ok(plumage)
    }
}

/// How much lighter or darker a roll draws a palette, at most, as a factor
/// on every colour. Provenance: **chosen by render** (#423): enough that two
/// owls of one palette are not twins, not so much a crow greys.
const SHADE_SPREAD: f32 = 0.10;

/// The kinds of bird a plumage is drawn for (#422, #423): the plan's default,
/// an owl, and the three its head's axes reach. Not stored: read back from the
/// head (`WingedParams::bird_kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BirdKind {
    /// An owl: a short hooked beak in a full disc.
    Owl,
    /// A parrot: a deep hooked bill, no disc.
    Parrot,
    /// A crow: a long, slightly curved bill.
    Crow,
    /// A songbird: a short straight bill.
    Songbird,
}

impl BirdKind {
    /// Every kind, in the order a roll's weights count them.
    pub(crate) const ALL: [Self; 4] = [Self::Owl, Self::Parrot, Self::Crow, Self::Songbird];

    /// Its head's `beakLength`, `beakHook` and `facialDisc` (docs/winged-anatomy.md
    /// "The axes").
    pub(crate) const fn axes(self) -> [f32; 3] {
        match self {
            Self::Owl => [0.0, 0.0, 0.0],
            Self::Parrot => [0.3, 1.0, -1.0],
            Self::Crow => [1.0, -1.0, -1.0],
            Self::Songbird => [0.2, -1.3, -1.0],
        }
    }

    /// What a bird with the head `axes` is drawn as: **an owl if it has a
    /// disc** - its `facialDisc` above [`OWL_DISC`] - and otherwise the kind
    /// of the three disc-less ones whose head's axes stand nearest. Ties go
    /// to the earlier.
    ///
    /// **The disc decides first** (#423, the critic): only an owl's roll
    /// draws a disc, so a head that has one was drawn as an owl however its
    /// beak rolled; read as the nearest of all four, an owl whose hook rolled
    /// wild - `[0.01, 2.79, -0.03]`, a full disc - stood nearer the parrot and
    /// was painted a macaw's red with no disc to show.
    pub(crate) fn of_head(axes: [f32; 3]) -> Self {
        if axes[2] > OWL_DISC {
            return Self::Owl;
        }
        let far = |kind: Self| {
            kind.axes()
                .iter()
                .zip(axes)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f32>()
        };
        [Self::Parrot, Self::Crow, Self::Songbird]
            .into_iter()
            .fold((Self::Parrot, f32::INFINITY), |(best, near), kind| {
                let distance = far(kind);
                if distance < near {
                    (kind, distance)
                } else {
                    (best, near)
                }
            })
            .0
    }
}

/// The `facialDisc` above which a head has a disc and is read as an owl's
/// ([`BirdKind::of_head`]): a quarter of a full disc. Only an owl's roll draws
/// a disc, about `0` at a quarter's spread, and every other kind's head is
/// `-1`, none; so the line is three spreads under an owl's and a quarter
/// over the rest. Provenance: **derived** from the head roll's spread (#423).
pub(crate) const OWL_DISC: f32 = -0.75;

/// The palettes a roll picks among for a kind of bird, its preset first.
/// Provenance: **chosen by render** (#423), each after a bird a field guide
/// shows, so a roll reads as a kind of bird rather than as a random colouring.
pub(crate) fn palettes(kind: BirdKind) -> &'static [Plumage] {
    match kind {
        BirdKind::Owl => &OWLS,
        BirdKind::Parrot => &PARROTS,
        BirdKind::Crow => &CROWS,
        BirdKind::Songbird => &SONGBIRDS,
    }
}

/// A plumage of one colour on every tract but those given: what most
/// palettes below differ from.
const fn plain(colour: [f32; 3]) -> Plumage {
    Plumage {
        head: colour,
        upper: colour,
        under: colour,
        flight: colour,
        tail: colour,
        mark: colour,
        disc: colour,
        ruff: colour,
        pattern: PlumagePattern::Plain,
        gloss: 0.0,
        marks: 0,
    }
}

const OWLS: [Plumage; 5] = [
    Plumage::OWL,
    // A tawny owl: rufous brown, its breast streaked dark.
    Plumage {
        head: rgb(0.50, 0.32, 0.18),
        upper: rgb(0.48, 0.30, 0.17),
        under: rgb(0.78, 0.62, 0.44),
        flight: rgb(0.40, 0.27, 0.16),
        tail: rgb(0.42, 0.28, 0.17),
        mark: rgb(0.26, 0.16, 0.10),
        disc: rgb(0.68, 0.50, 0.33),
        ruff: rgb(0.26, 0.16, 0.10),
        pattern: PlumagePattern::Streaked,
        gloss: 0.0,
        marks: 0,
    },
    // A barn owl: golden buff above, white below, a white face ringed buff.
    Plumage {
        head: rgb(0.80, 0.64, 0.40),
        upper: rgb(0.78, 0.62, 0.40),
        under: rgb(0.95, 0.93, 0.88),
        flight: rgb(0.82, 0.68, 0.46),
        tail: rgb(0.82, 0.68, 0.46),
        mark: rgb(0.46, 0.44, 0.42),
        disc: rgb(0.97, 0.95, 0.92),
        ruff: rgb(0.70, 0.52, 0.30),
        pattern: PlumagePattern::Spotted,
        gloss: 0.0,
        marks: 0,
    },
    // A snowy owl: white, barred dark.
    Plumage {
        mark: rgb(0.22, 0.20, 0.18),
        ruff: rgb(0.80, 0.80, 0.78),
        pattern: PlumagePattern::Barred,
        ..plain(rgb(0.95, 0.95, 0.93))
    },
    // A great grey owl: grey, streaked darker, a pale face ringed dark.
    Plumage {
        head: rgb(0.54, 0.53, 0.51),
        upper: rgb(0.50, 0.49, 0.47),
        under: rgb(0.66, 0.65, 0.62),
        flight: rgb(0.42, 0.41, 0.40),
        tail: rgb(0.44, 0.43, 0.42),
        mark: rgb(0.24, 0.23, 0.22),
        disc: rgb(0.78, 0.77, 0.74),
        ruff: rgb(0.20, 0.19, 0.18),
        pattern: PlumagePattern::Streaked,
        gloss: 0.0,
        marks: 0,
    },
];

const CROWS: [Plumage; 5] = [
    Plumage::CROW,
    // A raven: blacker and glossier.
    Plumage {
        gloss: 1.0,
        ..plain(rgb(0.05, 0.05, 0.07))
    },
    // A jackdaw: a grey nape over a black back.
    Plumage {
        head: rgb(0.40, 0.41, 0.44),
        under: rgb(0.16, 0.16, 0.18),
        gloss: 0.5,
        ..plain(rgb(0.09, 0.09, 0.11))
    },
    // A hooded crow: a grey body, the head, wings and tail black.
    Plumage {
        upper: rgb(0.56, 0.56, 0.57),
        under: rgb(0.60, 0.60, 0.61),
        gloss: 0.5,
        ..plain(rgb(0.07, 0.07, 0.08))
    },
    // A magpie: black and white, its wings blue-black and its tail green.
    Plumage {
        under: rgb(0.94, 0.94, 0.92),
        flight: rgb(0.08, 0.10, 0.22),
        tail: rgb(0.06, 0.12, 0.10),
        gloss: 0.8,
        ..plain(rgb(0.06, 0.06, 0.07))
    },
];

const PARROTS: [Plumage; 6] = [
    Plumage::PARROT,
    // A blue-and-yellow macaw.
    Plumage {
        head: rgb(0.18, 0.52, 0.34),
        upper: rgb(0.10, 0.40, 0.80),
        under: rgb(0.98, 0.76, 0.10),
        flight: rgb(0.08, 0.30, 0.72),
        tail: rgb(0.10, 0.38, 0.78),
        gloss: 0.2,
        ..plain(rgb(0.10, 0.40, 0.80))
    },
    // A rainbow lorikeet: a blue head, a green back, an orange breast.
    Plumage {
        head: rgb(0.14, 0.24, 0.74),
        under: rgb(0.98, 0.46, 0.08),
        gloss: 0.2,
        ..plain(rgb(0.14, 0.58, 0.20))
    },
    // A budgerigar: a yellow face, a green body, its back barred black.
    Plumage {
        head: rgb(0.95, 0.88, 0.22),
        upper: rgb(0.42, 0.74, 0.20),
        under: rgb(0.40, 0.80, 0.22),
        flight: rgb(0.30, 0.54, 0.18),
        tail: rgb(0.14, 0.30, 0.64),
        mark: rgb(0.08, 0.08, 0.06),
        pattern: PlumagePattern::Barred,
        gloss: 0.1,
        ..plain(rgb(0.40, 0.76, 0.22))
    },
    // An African grey: grey, its tail red.
    Plumage {
        head: rgb(0.56, 0.57, 0.58),
        upper: rgb(0.50, 0.51, 0.53),
        under: rgb(0.62, 0.63, 0.64),
        flight: rgb(0.30, 0.31, 0.33),
        tail: rgb(0.82, 0.10, 0.10),
        gloss: 0.1,
        ..plain(rgb(0.56, 0.57, 0.58))
    },
    // A cockatoo: white, its tail washed yellow.
    Plumage {
        tail: rgb(0.95, 0.90, 0.66),
        flight: rgb(0.93, 0.92, 0.82),
        gloss: 0.1,
        ..plain(rgb(0.95, 0.95, 0.93))
    },
];

const SONGBIRDS: [Plumage; 6] = [
    Plumage::SONGBIRD,
    // A robin: olive brown above, an orange face and breast.
    Plumage {
        head: rgb(0.86, 0.46, 0.20),
        upper: rgb(0.44, 0.38, 0.27),
        under: rgb(0.88, 0.50, 0.24),
        flight: rgb(0.38, 0.32, 0.23),
        tail: rgb(0.40, 0.34, 0.24),
        ..plain(rgb(0.44, 0.38, 0.27))
    },
    // A house sparrow: a streaked brown back, a grey breast, a chestnut head.
    Plumage {
        head: rgb(0.52, 0.34, 0.20),
        upper: rgb(0.56, 0.40, 0.26),
        under: rgb(0.70, 0.69, 0.66),
        flight: rgb(0.42, 0.30, 0.20),
        tail: rgb(0.40, 0.30, 0.22),
        mark: rgb(0.16, 0.11, 0.08),
        pattern: PlumagePattern::Streaked,
        ..plain(rgb(0.52, 0.34, 0.20))
    },
    // A cardinal: red all over, its wings a deeper red.
    Plumage {
        flight: rgb(0.60, 0.10, 0.08),
        tail: rgb(0.62, 0.10, 0.08),
        gloss: 0.1,
        ..plain(rgb(0.82, 0.12, 0.10))
    },
    // A blue tit: a blue cap, a green back, a yellow breast, blue wings.
    Plumage {
        head: rgb(0.22, 0.44, 0.80),
        upper: rgb(0.52, 0.62, 0.28),
        under: rgb(0.96, 0.84, 0.22),
        flight: rgb(0.24, 0.40, 0.72),
        tail: rgb(0.24, 0.40, 0.72),
        ..plain(rgb(0.22, 0.44, 0.80))
    },
    // A song thrush: brown above, a cream breast spotted dark.
    Plumage {
        head: rgb(0.48, 0.38, 0.26),
        upper: rgb(0.48, 0.38, 0.26),
        under: rgb(0.92, 0.86, 0.72),
        flight: rgb(0.42, 0.33, 0.22),
        tail: rgb(0.44, 0.34, 0.23),
        mark: rgb(0.24, 0.17, 0.10),
        pattern: PlumagePattern::Spotted,
        ..plain(rgb(0.48, 0.38, 0.26))
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_is_on_the_wire_s_grid_and_survives_its_bytes() {
        for kind in BirdKind::ALL {
            for palette in palettes(kind) {
                let mut settled = *palette;
                settled.sanitize();
                assert_eq!(
                    settled, *palette,
                    "{kind:?}: a preset is what it reads back as"
                );
                let mut out = Vec::new();
                palette.encode(&mut out);
                assert_eq!(out.len(), 8 * 3 + 4);
                let back = Plumage::decode(&mut out.as_slice()).expect("decodes");
                assert_eq!(back.pattern, palette.pattern);
                assert_eq!(back.marks, palette.marks);
                for (a, b) in back.colours().into_iter().zip(palette.colours()) {
                    for (a, b) in a.iter().zip(b) {
                        assert!((a - b).abs() < 0.003, "{kind:?}: {a} came back {b}");
                    }
                }
                let mut again = Vec::new();
                back.encode(&mut again);
                assert_eq!(again, out, "a decoded plumage writes the same bytes");
            }
        }
        let short = [0u8; 10];
        assert!(Plumage::decode(&mut short.as_slice()).is_err());
    }

    #[test]
    fn each_kind_is_read_off_its_own_head_and_a_disc_is_an_owl_s() {
        for kind in BirdKind::ALL {
            assert_eq!(BirdKind::of_head(kind.axes()), kind);
            assert_eq!(Plumage::of(kind), palettes(kind)[0]);
        }
        // Halfway along a crow's beak toward an owl's is still a crow while
        // the disc is under a quarter, an owl once it is past.
        assert_eq!(BirdKind::of_head([0.7, -0.7, -0.8]), BirdKind::Crow);
        assert_eq!(BirdKind::of_head([0.2, -0.2, -0.2]), BirdKind::Owl);
        // An owl whose hook rolled wild keeps its disc and is an owl.
        assert_eq!(BirdKind::of_head([0.01, 2.79, -0.03]), BirdKind::Owl);
        assert_eq!(BirdKind::of_head([0.3, 1.0, -0.7]), BirdKind::Owl);
        assert_eq!(BirdKind::of_head([0.3, 1.0, -0.8]), BirdKind::Parrot);
    }

    #[test]
    fn a_plumage_reads_from_a_record_that_has_none_as_an_owl_s() {
        let json = serde_json::to_string(&Plumage::CROW).expect("writes");
        let back: Plumage = serde_json::from_str(&json).expect("reads");
        assert_eq!(back, Plumage::CROW);
        let empty: Plumage = serde_json::from_str("{}").expect("reads");
        assert_eq!(empty, Plumage::OWL);
        let mut wild = Plumage {
            head: [2.0, -1.0, f32::NAN],
            gloss: 4.0,
            ..Plumage::OWL
        };
        wild.sanitize();
        assert_eq!(wild.head, [1.0, 0.0, 0.0]);
        assert_eq!(wild.gloss, 1.0);
    }
}
