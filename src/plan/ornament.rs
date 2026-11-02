//! What a beast wears on its head (#402): a single horn, paired horns, fangs,
//! tusks and antlers, each one optional and shaped by the same few numbers.
//!
//! **In the head's own proportions.** A length and a thickness are shares of the
//! head's radius, so a piece scales with the head and a slider means the same on a
//! cat and a horse, and nothing here names a family: where a piece stands is the
//! head's declaration (`plan::derive::quadruped`), and the sweep that draws it
//! `face::beast::adorn`.
//!
//! **A creator's, and rarely a roll's** (#403, the owner's call). A creator gives a
//! beast whatever it likes; a roll gives one beast in [`ORNAMENT_SHARE`]'s few one
//! look off a short list, under the head's lock. A re-roll of the head puts what its
//! seed draws in place of what was there, a creator's own included, which on most
//! seeds is nothing: a creator who wants to keep them locks the head.
//!
//! **A published `horn: "unicorn"` goes on meaning what it meant**, and a body
//! wearing exactly that goes on writing it: [`Ornament::UNICORN`] is that horn
//! written out, and [`on_record`] and the share code's `horn` byte are the two
//! places it is told apart from every other horn, which only this block can say.

use serde::{Deserialize, Serialize};

use super::{PlanDecodeError, Rolls};

/// The share of re-rolled beasts that wear an ornament.
/// Provenance: **the owner's call** (#403), for the owner to judge from a sheet: about
/// one roll in twelve. Drawn on a stream of its own, `quadruped.ornament`, so every
/// beast it misses is what it was to the byte.
const ORNAMENT_SHARE: f32 = 1.0 / 12.0;

/// The looks a roll chooses among, on `quadruped.ornamentLook`, each with how often
/// it is drawn. One piece at a time: each of these reads as one animal, and two of
/// them together read as a costume.
///
/// Provenance: **chosen, and judged by render** (#403). These are the shapes a creator
/// starts from rather than random numbers, so a rolled piece is one somebody looked
/// at. Even, but for [`SABRE_LONGEST_MUZZLE`].
const ROLLED_LOOKS: [(f32, fn() -> Ornaments); 6] = [
    (1.0, || Ornaments::wearing(OrnamentKind::Horn)),
    (1.0, || Ornaments::wearing(OrnamentKind::PairedHorns)),
    (1.0, || Ornaments {
        paired_horns: Some(Ornament::RAM),
        ..Ornaments::default()
    }),
    (1.0, || Ornaments::wearing(OrnamentKind::UpperFangs)),
    (1.0, || Ornaments::wearing(OrnamentKind::Tusks)),
    (1.0, || Ornaments::wearing(OrnamentKind::Antlers)),
];

/// How long a piece can be, in head radii.
pub const LENGTH_RANGE: (f32, f32) = (0.05, 4.0);

/// A piece's radius at its base, in head radii.
pub const THICKNESS_RANGE: (f32, f32) = (0.01, 0.5);

/// How much of its radius a piece has lost at its tip. Short of all of it, so that
/// a tip is always a ring with a direction rather than a point with none.
pub const TAPER_RANGE: (f32, f32) = (0.0, 0.98);

/// How far a piece's line turns from base to tip, in whole turns. The sign says
/// which way, and what that means is each kind's: see [`Ornament::curve`].
pub const CURVE_RANGE: (f32, f32) = (-1.5, 1.5);

/// How far a piece's flutes turn about it from base to tip, in whole turns.
pub const TWIST_RANGE: (f32, f32) = (-4.0, 4.0);

/// How deep a piece's flutes are, as a share of its radius at each point.
pub const FLUTES_RANGE: (f32, f32) = (0.0, 0.5);

/// How many points an antler can end in, its own tip among them.
pub const POINTS_RANGE: (u32, u32) = (1, 8);

/// How wide an antler's points fork from its beam: from almost along it to almost
/// square to it.
pub const FORK_RANGE: (f32, f32) = (0.0, 1.0);

/// Ivory: what a horn, a fang and a tusk are unless a creator says otherwise. A
/// unicorn's, as it has been since #367.
const IVORY: [f32; 3] = [0.96, 0.91, 0.80];

/// Where in [`ROLLED_LOOKS`] the sabre fangs are.
const SABRE_LOOK: usize = 3;

/// The longest muzzle a roll gives sabre fangs to, on `muzzleLength`: past it the look
/// is not drawn and the others share its weight.
/// Provenance: **tuned by render** (#403). A fang hangs from the upper lip, so on a long
/// face it is a thorn halfway down the muzzle rather than a sabre, and from the front it
/// hardly shows. A head's own proportion, so a cross goes by the face it grew.
const SABRE_LONGEST_MUZZLE: f32 = 0.5;

/// Old ivory, for the root of a tusk.
const OLD_IVORY: [f32; 3] = [0.86, 0.76, 0.56];

/// A pale fang's root.
const ENAMEL: [f32; 3] = [0.97, 0.95, 0.88];

/// A horn's pale root, and its dark tip.
const HORN_ROOT: [f32; 3] = [0.74, 0.68, 0.56];
const HORN_TIP: [f32; 3] = [0.15, 0.12, 0.10];

/// An antler's dark beam, and the pale of its worn tips.
const ANTLER_ROOT: [f32; 3] = [0.34, 0.25, 0.17];
const ANTLER_TIP: [f32; 3] = [0.88, 0.82, 0.68];

/// The colours an editor offers a piece before its free picker (#402), named, in sRGB:
/// what the presets are made of, and a few beyond them.
pub const ORNAMENT_SWATCHES: [(&str, [f32; 3]); 10] = [
    ("ivory", IVORY),
    ("enamel", ENAMEL),
    ("old ivory", OLD_IVORY),
    ("pale horn", HORN_ROOT),
    ("dark horn", HORN_TIP),
    ("antler", ANTLER_ROOT),
    ("worn antler", ANTLER_TIP),
    ("gold", [0.90, 0.72, 0.28]),
    ("obsidian", [0.06, 0.06, 0.08]),
    ("blood", [0.55, 0.06, 0.08]),
];

/// One piece's shape and colours (#402), in the head's own proportions.
///
/// Every number is clamped and put on the wire's grid by `sanitize`, and a value a
/// reader cannot use reads as this default's, which is a unicorn's horn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ornament {
    /// Base to tip along its line, in head radii ([`LENGTH_RANGE`]).
    #[serde(with = "super::scaled")]
    pub length: f32,
    /// Its radius at the base, in head radii ([`THICKNESS_RANGE`]).
    #[serde(with = "super::scaled")]
    pub thickness: f32,
    /// How much of its radius it has lost at the tip ([`TAPER_RANGE`]).
    #[serde(with = "super::scaled")]
    pub taper: f32,
    /// How far its line turns from base to tip, in whole turns ([`CURVE_RANGE`]),
    /// the turn tightening toward the tip past a little of one and the line
    /// stepping aside so that a coil clears itself.
    ///
    /// Which way is each kind's. Paired horns' positive turns up and forward, a
    /// bull's crescent, and their negative back and down round the ear, a ram's curl.
    /// An antler's positive turns its beam forward over the face. A single horn's
    /// positive turns forward over the nose and its negative back over the skull. A
    /// fang's and a tusk's positive turns its tip back toward the throat.
    #[serde(with = "super::scaled")]
    pub curve: f32,
    /// How far its flutes turn about it from base to tip, in whole turns
    /// ([`TWIST_RANGE`]). Nothing shows without [`Self::flutes`].
    #[serde(with = "super::scaled")]
    pub twist: f32,
    /// How deep its flutes are, as a share of its radius ([`FLUTES_RANGE`]): none
    /// at zero, a smooth tube.
    #[serde(with = "super::scaled")]
    pub flutes: f32,
    /// Its colour at the base, in sRGB.
    #[serde(with = "super::scaled::triple")]
    pub base: [f32; 3],
    /// Its colour at the tip, in sRGB.
    #[serde(with = "super::scaled::triple")]
    pub tip: [f32; 3],
}

impl Default for Ornament {
    fn default() -> Self {
        Self::UNICORN
    }
}

impl Ornament {
    /// A unicorn's horn, as `horn: "unicorn"` has meant since #367: straight,
    /// fluted, ivory. Provenance: **the constants that horn was drawn with**.
    pub const UNICORN: Self = Self {
        length: 1.3,
        thickness: 0.15,
        taper: 0.94,
        curve: 0.0,
        twist: 1.5,
        flutes: 0.12,
        base: IVORY,
        tip: IVORY,
    };

    /// A ram's paired horns: thick at the root and curling back round the ear, where
    /// [`OrnamentKind::preset`]'s are a bull's crescent. One of the looks a roll draws,
    /// and there for any creator to start from.
    /// Provenance: **tuned by render** (#403).
    pub const RAM: Self = Self {
        length: 2.6,
        thickness: 0.2,
        taper: 0.85,
        curve: -0.8,
        twist: 0.0,
        flutes: 0.0,
        base: HORN_ROOT,
        tip: OLD_IVORY,
    };

    /// Clamps every number to its envelope and puts it on the wire's grid.
    pub fn sanitize(&mut self) {
        let default = Self::default();
        for (value, fallback, range) in [
            (&mut self.length, default.length, LENGTH_RANGE),
            (&mut self.thickness, default.thickness, THICKNESS_RANGE),
            (&mut self.taper, default.taper, TAPER_RANGE),
            (&mut self.curve, default.curve, CURVE_RANGE),
            (&mut self.twist, default.twist, TWIST_RANGE),
            (&mut self.flutes, default.flutes, FLUTES_RANGE),
        ] {
            *value = super::sanitize_axis(*value, fallback, range);
        }
        for colour in [&mut self.base, &mut self.tip] {
            *colour = colour.map(|part| super::sanitize_axis(part, 0.0, (0.0, 1.0)));
        }
    }

    /// Its bytes on a share code: a span an axis, a byte a colour's part.
    fn encode(&self, out: &mut Vec<u8>) {
        super::put_span(out, self.length, LENGTH_RANGE);
        super::put_span(out, self.thickness, THICKNESS_RANGE);
        super::put_span(out, self.taper, TAPER_RANGE);
        super::put_span(out, self.curve, CURVE_RANGE);
        super::put_span(out, self.twist, TWIST_RANGE);
        super::put_span(out, self.flutes, FLUTES_RANGE);
        for part in self.base.into_iter().chain(self.tip) {
            super::put_unit(out, part);
        }
    }

    /// It back off a share code.
    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut read = Self {
            length: super::take_span(bytes, LENGTH_RANGE)?,
            thickness: super::take_span(bytes, THICKNESS_RANGE)?,
            taper: super::take_span(bytes, TAPER_RANGE)?,
            curve: super::take_span(bytes, CURVE_RANGE)?,
            twist: super::take_span(bytes, TWIST_RANGE)?,
            flutes: super::take_span(bytes, FLUTES_RANGE)?,
            ..Self::default()
        };
        for part in read.base.iter_mut().chain(read.tip.iter_mut()) {
            *part = super::take_unit(bytes)?;
        }
        Ok(read)
    }
}

/// A pair of antlers (#402): a beam shaped as any other piece is, and what its
/// branching needs of its own. A writer sets every field; a beam's field left out of
/// a hand-written record reads as [`Ornament`]'s default rather than a stag's.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Antlers {
    /// The beam, from the skull to its own tip.
    #[serde(flatten)]
    pub beam: Ornament,
    /// How many points each ends in, the beam's own tip among them
    /// ([`POINTS_RANGE`]): the rest are tines, spaced up the beam from the brow.
    #[serde(
        serialize_with = "super::scaled::serialize_count",
        deserialize_with = "super::scaled::deserialize_count"
    )]
    pub points: u32,
    /// How wide the tines fork from the beam ([`FORK_RANGE`]).
    #[serde(with = "super::scaled")]
    pub fork: f32,
}

impl Default for Antlers {
    /// A stag's. Provenance: **tuned by render** (#402).
    fn default() -> Self {
        Self {
            beam: Ornament {
                length: 2.3,
                thickness: 0.11,
                taper: 0.65,
                curve: 0.14,
                twist: 0.0,
                flutes: 0.0,
                base: ANTLER_ROOT,
                tip: ANTLER_TIP,
            },
            points: 5,
            fork: 0.45,
        }
    }
}

impl Antlers {
    /// Clamps its numbers to their envelopes. A count out of range lands on its
    /// nearer end.
    pub fn sanitize(&mut self) {
        self.beam.sanitize();
        self.points = self.points.clamp(POINTS_RANGE.0, POINTS_RANGE.1);
        self.fork = super::sanitize_axis(self.fork, Self::default().fork, FORK_RANGE);
    }
}

/// The kinds of piece a head can wear (#402).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrnamentKind {
    /// One horn on the middle of the forehead: a unicorn's.
    Horn,
    /// A horn either side of the crown: a bull's, a ram's.
    PairedHorns,
    /// A fang either side of the upper lip, outside the shut jaw: a sabre-tooth's.
    UpperFangs,
    /// A small fang either side of the lower jaw, opening with it.
    LowerFangs,
    /// A tusk either side of the lower jaw, opening with it and rising outside
    /// the upper lip: a boar's.
    Tusks,
    /// A branching antler either side of the crown: a stag's.
    Antlers,
}

impl OrnamentKind {
    /// Every kind, in the order the share code's mask counts them.
    pub const ALL: [Self; 6] = [
        Self::Horn,
        Self::PairedHorns,
        Self::UpperFangs,
        Self::LowerFangs,
        Self::Tusks,
        Self::Antlers,
    ];

    /// Its key on a record.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Horn => "horn",
            Self::PairedHorns => "pairedHorns",
            Self::UpperFangs => "upperFangs",
            Self::LowerFangs => "lowerFangs",
            Self::Tusks => "tusks",
            Self::Antlers => "antlers",
        }
    }

    /// Whether it rides the jaw and opens with the mouth, rather than the head.
    #[must_use]
    pub fn rides_the_jaw(self) -> bool {
        matches!(self, Self::LowerFangs | Self::Tusks)
    }

    /// The shape and colours it starts from when a creator turns it on: a good one
    /// of its kind before anyone has picked anything.
    /// Provenance: **tuned by render** (#402).
    #[must_use]
    pub fn preset(self) -> Ornament {
        match self {
            Self::Horn => Ornament::UNICORN,
            Self::PairedHorns => Ornament {
                length: 1.7,
                thickness: 0.17,
                taper: 0.92,
                curve: 0.14,
                twist: 0.0,
                flutes: 0.0,
                base: HORN_ROOT,
                tip: HORN_TIP,
            },
            Self::UpperFangs => Ornament {
                length: 0.62,
                thickness: 0.075,
                taper: 0.93,
                curve: 0.05,
                twist: 0.0,
                flutes: 0.0,
                base: ENAMEL,
                tip: IVORY,
            },
            Self::LowerFangs => Ornament {
                length: 0.42,
                thickness: 0.06,
                taper: 0.92,
                curve: 0.10,
                twist: 0.0,
                flutes: 0.0,
                base: ENAMEL,
                tip: IVORY,
            },
            Self::Tusks => Ornament {
                length: 1.05,
                thickness: 0.11,
                taper: 0.90,
                curve: 0.20,
                twist: 0.0,
                flutes: 0.0,
                base: OLD_IVORY,
                tip: IVORY,
            },
            Self::Antlers => Antlers::default().beam,
        }
    }
}

/// What a beast wears on its head (#402): one optional slot a kind, all empty until
/// a creator or a rare roll fills one (#403). See the module's own header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ornaments {
    /// One horn on the middle of the forehead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horn: Option<Ornament>,
    /// A horn either side of the crown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paired_horns: Option<Ornament>,
    /// A fang either side of the upper lip, riding the head.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upper_fangs: Option<Ornament>,
    /// A small fang either side of the lower jaw, riding it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lower_fangs: Option<Ornament>,
    /// A tusk either side of the lower jaw, riding it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tusks: Option<Ornament>,
    /// A branching antler either side of the crown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub antlers: Option<Antlers>,
}

impl Ornaments {
    /// Wearing `kind` at its [`OrnamentKind::preset`], and nothing else.
    fn wearing(kind: OrnamentKind) -> Self {
        let mut worn = Self::default();
        worn.wear(kind, true);
        worn
    }

    /// What a roll gives a head with this `muzzle_length` (#403): nothing, on most
    /// seeds, and otherwise one look off [`ROLLED_LOOKS`], each draw on a stream of its
    /// own.
    #[must_use]
    pub(crate) fn rolled(rolls: &Rolls, muzzle_length: f32) -> Self {
        if rolls.range("quadruped.ornament", 0.0, 1.0) >= ORNAMENT_SHARE {
            return Self::default();
        }
        let mut weights = ROLLED_LOOKS.map(|look| look.0);
        if muzzle_length > SABRE_LONGEST_MUZZLE {
            weights[SABRE_LOOK] = 0.0;
        }
        let look = rolls.pick("quadruped.ornamentLook", &weights);
        ROLLED_LOOKS[look].1()
    }

    /// Whether it wears nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The shape and colours of what it wears as `kind`, if it wears one: an
    /// antler's beam, for antlers.
    #[must_use]
    pub fn get(&self, kind: OrnamentKind) -> Option<Ornament> {
        match kind {
            OrnamentKind::Horn => self.horn,
            OrnamentKind::PairedHorns => self.paired_horns,
            OrnamentKind::UpperFangs => self.upper_fangs,
            OrnamentKind::LowerFangs => self.lower_fangs,
            OrnamentKind::Tusks => self.tusks,
            OrnamentKind::Antlers => self.antlers.map(|antlers| antlers.beam),
        }
    }

    /// Where the shape and colours of what it wears as `kind` are kept, if it wears
    /// one.
    pub fn get_mut(&mut self, kind: OrnamentKind) -> Option<&mut Ornament> {
        match kind {
            OrnamentKind::Horn => self.horn.as_mut(),
            OrnamentKind::PairedHorns => self.paired_horns.as_mut(),
            OrnamentKind::UpperFangs => self.upper_fangs.as_mut(),
            OrnamentKind::LowerFangs => self.lower_fangs.as_mut(),
            OrnamentKind::Tusks => self.tusks.as_mut(),
            OrnamentKind::Antlers => self.antlers.as_mut().map(|antlers| &mut antlers.beam),
        }
    }

    /// Puts `kind` on, at its [`OrnamentKind::preset`], or takes it off. Putting on
    /// what is already worn leaves it as it is.
    pub fn wear(&mut self, kind: OrnamentKind, worn: bool) {
        if worn == self.get(kind).is_some() {
            return;
        }
        let slot = worn.then(|| kind.preset());
        match kind {
            OrnamentKind::Horn => self.horn = slot,
            OrnamentKind::PairedHorns => self.paired_horns = slot,
            OrnamentKind::UpperFangs => self.upper_fangs = slot,
            OrnamentKind::LowerFangs => self.lower_fangs = slot,
            OrnamentKind::Tusks => self.tusks = slot,
            OrnamentKind::Antlers => self.antlers = worn.then(Antlers::default),
        }
    }

    /// Clamps everything it wears.
    pub fn sanitize(&mut self) {
        for piece in [
            &mut self.horn,
            &mut self.paired_horns,
            &mut self.upper_fangs,
            &mut self.lower_fangs,
            &mut self.tusks,
        ]
        .into_iter()
        .flatten()
        {
            piece.sanitize();
        }
        if let Some(antlers) = &mut self.antlers {
            antlers.sanitize();
        }
    }

    /// What the older part of a share code says of it: the `horn` byte, a unicorn
    /// only for exactly a unicorn's horn.
    #[must_use]
    pub(crate) fn horn_byte(&self) -> u8 {
        Horn::from(self).code()
    }

    /// Reads the `horn` byte: a unicorn's horn, or nothing. A byte this build does
    /// not know reads as nothing.
    #[must_use]
    pub(crate) fn from_horn_byte(byte: u8) -> Self {
        Self {
            horn: (Horn::from_code(byte) == Horn::Unicorn).then_some(Ornament::UNICORN),
            ..Self::default()
        }
    }

    /// Whether a share code needs this block's bytes to say what it wears: whether
    /// it wears anything but exactly a unicorn's horn, which the `horn` byte says.
    #[must_use]
    pub(crate) fn needs_bytes(&self) -> bool {
        self.mask() != 0
    }

    /// Which kinds go on a share code as bytes, a bit each in
    /// [`OrnamentKind::ALL`]'s order.
    fn mask(&self) -> u8 {
        OrnamentKind::ALL
            .into_iter()
            .enumerate()
            .filter(|&(_, kind)| match self.get(kind) {
                Some(Ornament::UNICORN) => kind != OrnamentKind::Horn,
                Some(_) => true,
                None => false,
            })
            .fold(0, |mask, (bit, _)| mask | 1 << bit)
    }

    /// Its bytes on a share code: the [`Self::mask`], then each piece it names.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        let mask = self.mask();
        out.push(mask);
        for (bit, kind) in OrnamentKind::ALL.into_iter().enumerate() {
            if mask & 1 << bit == 0 {
                continue;
            }
            if let Some(piece) = self.get(kind) {
                piece.encode(out);
            }
            if let (OrnamentKind::Antlers, Some(antlers)) = (kind, self.antlers) {
                out.push(antlers.points.min(u32::from(u8::MAX)) as u8);
                super::put_span(out, antlers.fork, FORK_RANGE);
            }
        }
    }

    /// What a share code's block says, over what its `horn` byte said. A code that
    /// stops where the block would start names nothing further. A mask's top bits
    /// are no kind yet and read as nothing, and a code cut off inside a piece is
    /// [`PlanDecodeError::Truncated`].
    pub(crate) fn decode_over(&mut self, bytes: &mut &[u8]) -> Result<(), PlanDecodeError> {
        let Some((&mask, rest)) = bytes.split_first() else {
            return Ok(());
        };
        *bytes = rest;
        for (bit, kind) in OrnamentKind::ALL.into_iter().enumerate() {
            if mask & 1 << bit == 0 {
                continue;
            }
            let piece = Ornament::decode(bytes)?;
            match kind {
                OrnamentKind::Horn => self.horn = Some(piece),
                OrnamentKind::PairedHorns => self.paired_horns = Some(piece),
                OrnamentKind::UpperFangs => self.upper_fangs = Some(piece),
                OrnamentKind::LowerFangs => self.lower_fangs = Some(piece),
                OrnamentKind::Tusks => self.tusks = Some(piece),
                OrnamentKind::Antlers => {
                    let points = super::take_byte(bytes)?;
                    self.antlers = Some(Antlers {
                        beam: piece,
                        points: u32::from(points),
                        fork: super::take_span(bytes, FORK_RANGE)?,
                    });
                }
            }
        }
        self.sanitize();
        Ok(())
    }
}

/// What a record's older key says (#367): none, or a unicorn's horn. Read and
/// written for [`Ornament::UNICORN`] only, whoever gave it, a creator or a rare roll
/// (#403); a name or a share-code byte this build does not know reads as none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Horn {
    /// No horn.
    #[default]
    None,
    /// One spiralled horn, rising forward from the forehead: [`Ornament::UNICORN`].
    Unicorn,
}

named!(Horn: None => "none", Unicorn => "unicorn");

impl From<&Ornaments> for Horn {
    fn from(ornaments: &Ornaments) -> Self {
        if ornaments.horn == Some(Ornament::UNICORN) {
            Self::Unicorn
        } else {
            Self::None
        }
    }
}

/// How [`Ornaments`] sit on a record (#402), beside the plan's other keys: `horn`,
/// the older key, for exactly a unicorn's horn, and `ornaments` for everything else.
///
/// **So a body wearing what a published record could already say writes what that
/// record wrote**, and a reader of it loses nothing. A record's `ornaments.horn`
/// wins over its `horn`, which is what an older writer that kept the key it did not
/// know would leave behind it.
pub(crate) mod on_record {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{Horn, Ornament, Ornaments};

    #[derive(Default, Serialize, Deserialize)]
    #[serde(default)]
    struct OnRecord {
        #[serde(skip_serializing_if = "is_hornless")]
        horn: Horn,
        #[serde(skip_serializing_if = "Ornaments::is_empty")]
        ornaments: Ornaments,
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_hornless(horn: &Horn) -> bool {
        *horn == Horn::None
    }

    pub(crate) fn serialize<S: Serializer>(
        value: &Ornaments,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let horn = Horn::from(value);
        let mut ornaments = *value;
        if horn == Horn::Unicorn {
            ornaments.horn = None;
        }
        OnRecord { horn, ornaments }.serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Ornaments, D::Error> {
        let OnRecord {
            horn,
            mut ornaments,
        } = OnRecord::deserialize(deserializer)?;
        if horn == Horn::Unicorn && ornaments.horn.is_none() {
            ornaments.horn = Some(Ornament::UNICORN);
        }
        Ok(ornaments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{Archetype, QuadrupedParams};

    /// A body wearing every kind, each away from anything a unicorn is.
    fn all_worn() -> Ornaments {
        let mut ornaments = Ornaments::default();
        for kind in OrnamentKind::ALL {
            ornaments.wear(kind, true);
        }
        ornaments.horn = Some(Ornament {
            length: 2.0,
            ..Ornament::UNICORN
        });
        ornaments
    }

    #[test]
    fn every_preset_is_on_the_grid_and_inside_its_envelope() {
        let mut worn = all_worn();
        let before = worn;
        worn.sanitize();
        assert_eq!(worn, before, "sanitize moved a preset");
        let mut unicorn = Ornament::UNICORN;
        unicorn.sanitize();
        assert_eq!(unicorn, Ornament::UNICORN, "bit for bit: see `on_record`");
    }

    #[test]
    fn sanitize_is_safe_against_anything() {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0e30, -1.0e30] {
            let hostile = Ornament {
                length: value,
                thickness: value,
                taper: value,
                curve: value,
                twist: value,
                flutes: value,
                base: [value; 3],
                tip: [value; 3],
            };
            let mut worn = Ornaments {
                horn: Some(hostile),
                paired_horns: Some(hostile),
                upper_fangs: Some(hostile),
                lower_fangs: Some(hostile),
                tusks: Some(hostile),
                antlers: Some(Antlers {
                    beam: hostile,
                    points: u32::MAX,
                    fork: value,
                }),
            };
            worn.sanitize();
            for kind in OrnamentKind::ALL {
                let piece = worn.get(kind).expect("kept");
                for (read, (low, high)) in [
                    (piece.length, LENGTH_RANGE),
                    (piece.thickness, THICKNESS_RANGE),
                    (piece.taper, TAPER_RANGE),
                    (piece.curve, CURVE_RANGE),
                    (piece.twist, TWIST_RANGE),
                    (piece.flutes, FLUTES_RANGE),
                ] {
                    assert!((low..=high).contains(&read), "{value} read {read}");
                }
                for part in piece.base.into_iter().chain(piece.tip) {
                    assert!((0.0..=1.0).contains(&part), "{value} read {part}");
                }
            }
            let antlers = worn.antlers.expect("kept");
            assert_eq!(antlers.points, POINTS_RANGE.1);
            assert!((FORK_RANGE.0..=FORK_RANGE.1).contains(&antlers.fork));
            let again = worn;
            worn.sanitize();
            assert_eq!(worn, again, "sanitize is idempotent");
        }
    }

    #[test]
    fn a_share_code_block_round_trips_and_is_stable() {
        let mut worn = all_worn();
        worn.sanitize();
        let mut bytes = Vec::new();
        worn.encode(&mut bytes);
        let mut read = Ornaments::default();
        let mut cursor = bytes.as_slice();
        read.decode_over(&mut cursor).expect("reads");
        assert!(cursor.is_empty(), "every byte was taken");
        for kind in OrnamentKind::ALL {
            let (was, now) = (worn.get(kind).unwrap(), read.get(kind).unwrap());
            assert!((was.length - now.length).abs() < 0.01, "{kind:?}");
            assert!((was.base[0] - now.base[0]).abs() < 0.003, "{kind:?}");
        }
        let mut again = Vec::new();
        read.encode(&mut again);
        assert_eq!(again, bytes, "a block read and written is the same block");
    }

    #[test]
    fn a_block_cut_short_is_an_error_and_unknown_bits_are_nothing() {
        let mut bytes = Vec::new();
        all_worn().encode(&mut bytes);
        for cut in 1..bytes.len() {
            let mut cursor = &bytes[..cut];
            assert_eq!(
                Ornaments::default().decode_over(&mut cursor),
                Err(PlanDecodeError::Truncated),
                "cut at {cut}"
            );
        }
        let mut cursor: &[u8] = &[];
        let mut read = Ornaments::default();
        read.decode_over(&mut cursor).expect("a code with no block");
        assert!(read.is_empty());
        let mut cursor: &[u8] = &[0b1100_0000];
        read.decode_over(&mut cursor).expect("bits no kind has yet");
        assert!(read.is_empty() && cursor.is_empty());
    }

    #[test]
    fn exactly_a_unicorn_is_what_the_older_key_and_byte_say() {
        let unicorn = Ornaments {
            horn: Some(Ornament::UNICORN),
            ..Ornaments::default()
        };
        assert_eq!(Ornaments::from_horn_byte(unicorn.horn_byte()), unicorn);
        assert!(!unicorn.needs_bytes());
        let longer = Ornaments {
            horn: Some(Ornament {
                length: 1.301,
                ..Ornament::UNICORN
            }),
            ..Ornaments::default()
        };
        assert_eq!(longer.horn_byte(), 0, "no reader of the byte draws this");
        assert!(longer.needs_bytes());
        assert!(Ornaments::from_horn_byte(200).is_empty(), "an unknown byte");
        // Beside something else, the horn still goes by its byte.
        let both = Ornaments {
            tusks: Some(OrnamentKind::Tusks.preset()),
            ..unicorn
        };
        assert_eq!(both.horn_byte(), 1);
        assert_eq!(both.mask(), 1 << 4);
    }

    #[test]
    fn wearing_keeps_what_is_worn_and_taking_off_clears() {
        let mut ornaments = Ornaments::default();
        ornaments.wear(OrnamentKind::Tusks, true);
        assert_eq!(ornaments.tusks, Some(OrnamentKind::Tusks.preset()));
        ornaments.get_mut(OrnamentKind::Tusks).unwrap().length = 2.0;
        ornaments.wear(OrnamentKind::Tusks, true);
        assert_eq!(ornaments.tusks.unwrap().length, 2.0);
        ornaments.wear(OrnamentKind::Tusks, false);
        assert!(ornaments.is_empty());
    }

    /// A record's body, as JSON, with `extra` keys on it.
    fn read_body(extra: &str) -> QuadrupedParams {
        let json = format!(r#"{{"$type":"network.symbios.avatar.defs#quadruped"{extra}}}"#);
        let archetype: Archetype = serde_json::from_str(&json).expect("a record reads");
        let Archetype::Quadruped(mut params) = archetype else {
            panic!("a quadruped");
        };
        let mut record = crate::AvatarRecord::new("Read", Archetype::Quadruped(params));
        record.sanitize();
        let Archetype::Quadruped(read) = record.archetype else {
            panic!("a quadruped");
        };
        params = read;
        params
    }

    fn written(params: QuadrupedParams) -> serde_json::Value {
        serde_json::to_value(Archetype::Quadruped(params)).expect("writes")
    }

    #[test]
    fn a_published_unicorn_reads_as_one_horn_and_writes_what_it_wrote() {
        let params = read_body(r#","horn":"unicorn""#);
        assert_eq!(params.ornaments.horn, Some(Ornament::UNICORN));
        assert_eq!(params.ornaments.paired_horns, None);
        let json = written(params);
        assert_eq!(json["horn"], "unicorn");
        assert!(json.get("ornaments").is_none(), "{json}");
        // An unknown name, and none at all, read as none and write nothing.
        for extra in [r#","horn":"narwhal""#, r#","horn":"none""#, ""] {
            let params = read_body(extra);
            assert!(params.ornaments.is_empty(), "{extra}");
            let json = written(params);
            assert!(json.get("horn").is_none() && json.get("ornaments").is_none());
        }
    }

    #[test]
    fn every_kind_round_trips_through_a_record_and_a_block_wins_over_the_older_key() {
        let mut params = QuadrupedParams {
            ornaments: all_worn(),
            ..QuadrupedParams::default()
        };
        params.ornaments.antlers.as_mut().expect("worn").points = 7;
        let json = written(params);
        assert!(json.get("horn").is_none(), "a longer horn is no unicorn");
        for kind in OrnamentKind::ALL {
            assert!(json["ornaments"].get(kind.name()).is_some(), "{kind:?}");
        }
        assert_eq!(json["ornaments"]["antlers"]["points"], 7);
        assert_eq!(json["ornaments"]["tusks"]["length"], 1050);
        let back: Archetype = serde_json::from_value(json.clone()).expect("reads");
        assert_eq!(back, Archetype::Quadruped(params));
        assert_eq!(written(params), json, "stable");

        let both = read_body(r#","horn":"unicorn","ornaments":{"horn":{"length":2000}}"#);
        assert_eq!(both.ornaments.horn.expect("worn").length, 2.0);
        // A unicorn beside anything else keeps the key a reader of it can read.
        let mut beside = QuadrupedParams::default();
        beside.ornaments.horn = Some(Ornament::UNICORN);
        beside.ornaments.wear(OrnamentKind::Tusks, true);
        let json = written(beside);
        assert_eq!(json["horn"], "unicorn");
        assert!(json["ornaments"].get("horn").is_none());
        assert_eq!(
            serde_json::from_value::<Archetype>(json).expect("reads"),
            Archetype::Quadruped(beside)
        );
    }

    #[test]
    fn hostile_numbers_on_a_record_are_clamped_and_still_build() {
        let params = read_body(
            r#","ornaments":{"horn":{"length":99999999,"thickness":-5,"taper":99999,"curve":-99999,"twist":99999,"flutes":-3,"base":[-1,99999,5],"tip":[0,0,0]},
                "antlers":{"points":4000000000,"fork":99999,"length":-99},
                "tusks":{},"unknownKind":{"length":1},"upperFangs":{"unknownKey":1}}"#,
        );
        let horn = params.ornaments.horn.expect("worn");
        assert_eq!(horn.length, LENGTH_RANGE.1);
        assert_eq!(horn.thickness, THICKNESS_RANGE.0);
        assert_eq!(horn.taper, TAPER_RANGE.1);
        assert_eq!(horn.curve, CURVE_RANGE.0);
        assert_eq!(horn.twist, TWIST_RANGE.1);
        assert_eq!(horn.flutes, FLUTES_RANGE.0);
        assert_eq!(horn.base, [0.0, 1.0, 0.005]);
        let antlers = params.ornaments.antlers.expect("worn");
        assert_eq!(antlers.points, POINTS_RANGE.1);
        assert_eq!(antlers.fork, FORK_RANGE.1);
        assert_eq!(antlers.beam.length, LENGTH_RANGE.0);
        assert_eq!(params.ornaments.tusks, Some(Ornament::default()));
        let record = crate::AvatarRecord::new("Hostile", Archetype::Quadruped(params));
        assert!(crate::Avatar::build_for_shape(&record).is_some());
    }

    /// The first plain roll of each family and the first cross, which between them
    /// grow the heads a piece has to sit on, each with nothing on it.
    fn rolled_heads() -> Vec<QuadrupedParams> {
        let first = |wanted: &dyn Fn(&QuadrupedParams) -> bool| {
            let params = (0..)
                .map(rolled_body)
                .find(|params| wanted(params))
                .expect("in reach");
            QuadrupedParams {
                ornaments: Ornaments::default(),
                ..params
            }
        };
        let mut heads: Vec<_> = crate::plan::Family::ALL
            .into_iter()
            .map(|family| first(&|params| params.family == family && params.hybrid.is_none()))
            .collect();
        heads.push(first(&|params| params.hybrid.is_some()));
        heads
    }

    /// A rolled beast's body.
    fn rolled_body(seed: i64) -> QuadrupedParams {
        let record = crate::AvatarRecord::rolled(
            "Rolled",
            Archetype::Quadruped(QuadrupedParams::default()),
            seed,
        );
        let Archetype::Quadruped(params) = record.archetype else {
            panic!("a quadruped");
        };
        params
    }

    /// The first seed from 0 that does or does not draw a piece.
    fn seed_drawing(worn: bool) -> i64 {
        (0..)
            .find(|&seed| Ornaments::rolled(&Rolls::new(seed), 0.0).is_empty() != worn)
            .expect("in reach")
    }

    #[test]
    fn a_small_share_of_rolls_wear_one_look_off_the_list() {
        const SEEDS: i64 = 12_000;
        let looks = ROLLED_LOOKS.map(|look| look.1());
        let mut drawn = [0usize; ROLLED_LOOKS.len()];
        for seed in 0..SEEDS {
            let worn = Ornaments::rolled(&Rolls::new(seed), 0.0);
            if worn.is_empty() {
                continue;
            }
            let at = looks.iter().position(|look| *look == worn);
            drawn[at.unwrap_or_else(|| panic!("{seed} wears no look on the list: {worn:?}"))] += 1;
        }
        let worn: usize = drawn.iter().sum();
        let share = worn as f32 / SEEDS as f32;
        // Sampling error at this count is 0.0025; the margin is a quarter of the share.
        assert!(
            (share - ORNAMENT_SHARE).abs() < ORNAMENT_SHARE * 0.25,
            "{share} of rolls wear something, for a share of {ORNAMENT_SHARE}"
        );
        for (index, count) in drawn.iter().enumerate() {
            let even = *count as f32 / worn as f32;
            let weight = ROLLED_LOOKS[index].0 / ROLLED_LOOKS.iter().map(|l| l.0).sum::<f32>();
            assert!(
                (even - weight).abs() < weight * 0.35,
                "look {index} drawn {drawn:?}"
            );
        }
    }

    #[test]
    fn a_long_face_draws_no_sabres_and_the_coin_does_not_read_the_face() {
        assert!(ROLLED_LOOKS[SABRE_LOOK].1().upper_fangs.is_some());
        let at = |seed: i64, muzzle: f32| Ornaments::rolled(&Rolls::new(seed), muzzle);
        let (mut short, mut long) = (0, 0);
        for seed in 0..6000 {
            let (snub, shoulder, past) = (
                at(seed, -0.85),
                at(seed, SABRE_LONGEST_MUZZLE),
                at(seed, SABRE_LONGEST_MUZZLE + 0.01),
            );
            assert_eq!(snub.is_empty(), past.is_empty(), "seed {seed}");
            assert_eq!(snub, shoulder, "seed {seed}");
            short += usize::from(snub.upper_fangs.is_some());
            long += usize::from(past.upper_fangs.is_some());
        }
        assert!(short > 40, "a short face draws them: {short}");
        assert_eq!(long, 0);
    }

    #[test]
    fn every_rolled_look_is_one_piece_on_the_grid() {
        let unicorn = ROLLED_LOOKS[0].1();
        assert_eq!(
            unicorn.horn,
            Some(Ornament::UNICORN),
            "goes by the older key"
        );
        for (index, (_, look)) in ROLLED_LOOKS.iter().enumerate() {
            let worn = look();
            let pieces = OrnamentKind::ALL
                .iter()
                .filter(|kind| worn.get(**kind).is_some())
                .count();
            assert_eq!(pieces, 1, "look {index}");
            let mut settled = worn;
            settled.sanitize();
            assert_eq!(settled, worn, "look {index} is off the grid");
        }
    }

    #[test]
    fn every_rolled_look_builds_is_drawn_and_survives_its_round_trips() {
        // Every look on the heads a roll grows: what a piece costs and where it sits
        // read the head.
        let tris = |params: QuadrupedParams| {
            let record = crate::AvatarRecord::new("Worn", Archetype::Quadruped(params));
            crate::Avatar::build_for_shape(&record)
                .expect("builds")
                .drawn(0.0)
                .iter()
                .map(|mesh| mesh.mesh.faces.len())
                .sum::<usize>()
        };
        for bare in rolled_heads() {
            let plain = tris(bare);
            for (index, (_, look)) in ROLLED_LOOKS.iter().enumerate() {
                let params = QuadrupedParams {
                    ornaments: look(),
                    ..bare
                };
                assert!(
                    tris(params) > plain,
                    "look {index} on a {:?} draws nothing",
                    bare.family
                );
                let record = crate::AvatarRecord::new("Worn", Archetype::Quadruped(params));
                let text = serde_json::to_string(&record).expect("writes");
                let read: crate::AvatarRecord = serde_json::from_str(&text).expect("reads");
                assert_eq!(read, record, "look {index} on the record");
                let code = record.share_code();
                let back = crate::AvatarRecord::from_share_code("Back", &code).expect("reads");
                assert_eq!(back.share_code(), code, "look {index} on a code is stable");
                let Archetype::Quadruped(back) = back.archetype else {
                    panic!("a quadruped");
                };
                for kind in OrnamentKind::ALL {
                    let (read, wrote) = (back.ornaments.get(kind), look().get(kind));
                    assert_eq!(read.is_some(), wrote.is_some(), "look {index}: {kind:?}");
                    if let (Some(read), Some(wrote)) = (read, wrote) {
                        assert!(
                            (read.length - wrote.length).abs() < 0.02
                                && (read.curve - wrote.curve).abs() < 0.02
                                && (read.base[0] - wrote.base[0]).abs() < 0.003,
                            "look {index}: {kind:?} came back {read:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_beast_a_sweep_rolls_wearing_something_builds() {
        for seed in 0..360 {
            let params = rolled_body(seed);
            if params.ornaments.is_empty() {
                continue;
            }
            let record = crate::AvatarRecord::new("Worn", Archetype::Quadruped(params));
            assert!(
                crate::Avatar::build_for_shape(&record).is_some(),
                "seed {seed}"
            );
        }
    }

    #[test]
    fn a_re_roll_of_the_head_puts_what_its_seed_draws_over_a_creator_s() {
        let horned = QuadrupedParams {
            ornaments: all_worn(),
            ..QuadrupedParams::default()
        };
        let reroll = |seed: i64, locks: crate::LockSet| {
            let mut record = crate::AvatarRecord::new("Horned", Archetype::Quadruped(horned));
            record.locks = locks;
            record.reroll(seed);
            let Archetype::Quadruped(params) = record.archetype else {
                panic!("a quadruped");
            };
            params.ornaments
        };
        // Most seeds draw nothing, which takes a creator's off.
        let (bare, worn) = (seed_drawing(false), seed_drawing(true));
        assert!(reroll(bare, crate::LockSet::NONE).is_empty());
        let looks = ROLLED_LOOKS.map(|look| look.1());
        assert!(looks.contains(&reroll(worn, crate::LockSet::NONE)));
        // And the head's lock keeps them, whichever the seed draws.
        let locks = crate::LockSet::NONE.with(crate::Category::Head);
        let mut kept = all_worn();
        kept.sanitize();
        assert_eq!(reroll(bare, locks), kept);
        assert_eq!(reroll(worn, locks), kept);
        // No other lock does.
        for category in crate::Category::ALL {
            if category == crate::Category::Head {
                continue;
            }
            let drawn = reroll(worn, crate::LockSet::NONE.with(category));
            assert!(looks.contains(&drawn), "{category:?}: {drawn:?}");
        }
    }
}
