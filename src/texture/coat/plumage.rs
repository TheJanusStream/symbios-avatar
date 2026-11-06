//! Painting a bird's plumage (#423, W4): its skin's atlas and its feather
//! cards, from one sampler.
//!
//! **One sampler for every surface a feather colours** ([`PlumageSampler`]):
//! the atlas under the body's feathered outline is painted texel by texel with
//! [`PlumageSampler::colour`], and every feather card - W2's flight feathers,
//! coverts and tail fan, and the contour feathers a later part grows - takes
//! its root and tip from [`PlumageSampler::card`] at the point it roots, so a
//! card and the skin under it are one plumage and cannot drift apart.
//!
//! **Tracts by the body's own frame, not by the zone map** (the beast's rule,
//! #376, #395): a perching bird's trunk is raised most of a right angle, so
//! "up" is not its back. The back is the side the trunk's dorsal axis faces -
//! square to the spine from the hips to the withers, toward the sky - and the
//! upper tract turns into the under round the flank over a band, never on a
//! line. The head's colour fades in by how far a texel lies off the declared
//! head, the feathered legs' by how much of a leg it is, so no tract stops on
//! a chart's seam or a weight's line.
//!
//! **An owl's disc is painted, not tinted** (#423 folds #422's `ruff_shade` in):
//! the face inside the ruff in the plumage's `disc` and the ruff in its `ruff`,
//! both read off the declared head's carved dish, so a creator's pale face and
//! dark rim are exactly what shows, where a vertex tint could only darken the
//! coat under it.
//!
//! **A crow's gloss is a roughness and a sheen**: the atlas carries a roughness
//! channel (the ORM map's green), so a glossy plumage lowers it over the back,
//! the wings' coverts and the head and its highlight tightens; and since the
//! draw has no view-dependent channel, a cool blue-violet to blue-green sheen
//! is laid into the colour there, drifting over the body. A true iridescence -
//! the hue turning with the angle it is seen at - would need a thin-film or
//! iridescence term in the skin's material (Bevy's `StandardMaterial` has
//! none), or a material extension reading a hue-shift map; feather cards, in
//! the hair's draw, share its material and take only the sheen.

use glam::Vec3;

use super::body::{CoatBody, underside};
use super::marks::{Fields, step};
use crate::det::DetMath;
use crate::plan::{Limb, Plumage, PlumagePattern, Zone};

/// How rough a feather's surface is to the light, without gloss: a little
/// smoother than fur's (`FUR_ROUGHNESS`, 0.78). Provenance: **chosen by
/// render** (#423).
pub(super) const FEATHER_ROUGHNESS: f32 = 0.72;

/// How much a full gloss takes off [`FEATHER_ROUGHNESS`] where it reaches: a
/// raven's back at 0.32. Provenance: **chosen by render** (#423).
const GLOSS_SMOOTHING: f32 = 0.40;

/// The sheen a gloss lays into the colour, the two hues it drifts between:
/// a corvid's blue-violet and its blue-green. Provenance: **chosen by render**
/// (#423).
const SHEEN: (Vec3, Vec3) = (Vec3::new(0.26, 0.20, 0.52), Vec3::new(0.10, 0.34, 0.38));

/// How much of [`SHEEN`] a full gloss adds to the colour where it reaches.
/// Provenance: **chosen by render** (#423): enough that a black crow reads as
/// glossed at play size, not so much that it reads as blue.
const SHEEN_SHARE: f32 = 0.22;

/// The share of the trunk's dorsal facing over which the upper tract turns into
/// the under, as `(from, to)` on the cosine of the angle off the back: the
/// flank's band. Provenance: **chosen by render** (#423).
const FLANK: (f32, f32) = (-0.45, 0.25);

/// How much of the under colour the throat takes, on the head's undersides,
/// and over which share of facing the ground it comes in: narrow, since what
/// lies between two tracts' colours is a mixture of them, a muddy band
/// between a bluebird's blue and its rust.
const THROAT: (f32, (f32, f32)) = (1.0, (0.5, 0.85));

/// How far off the middle of an owl's ruff its colour reaches, in the ruff's
/// radii (`BeastHead::on_ruff`): all of it within the first, none past the
/// second. Provenance: **chosen by render** (#423): its surface stands one
/// radius off, and `0.4..1.0`, #422's shade's reach, painted almost nothing.
const RUFF_REACH: (f32, f32) = (1.15, 1.8);

/// How far apart, in luminance, a mark and a tract's colour must be for the
/// mark to show on it, from none to all: a little owl's pale spots on its
/// brown back and not on its buff breast.
const CONTRAST: (f32, f32) = (0.12, 0.30);

/// How many spots, bars and streaks there are per back height. Provenance:
/// **chosen by render** (#423).
const SPOTS: f32 = 16.0;
/// See [`SPOTS`].
const BARS: f32 = 13.0;
/// See [`SPOTS`].
const STREAKS: f32 = 20.0;

/// How far a streak's cells are drawn out along the spine: a streak is a spot
/// this many times as long as it is wide.
const STREAK_LENGTH: f32 = 5.0;

/// What kind of feather a card is (#423): what [`PlumageSampler::card`] colours
/// it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Feather {
    /// A flight feather of the wing: a primary, a secondary or a tertial.
    Remex,
    /// A covert of the wing, a humeral or a feather of the alula.
    Covert,
    /// A tail feather.
    Rectrix,
    /// An upper tail covert.
    TailCovert,
}

/// A feather card's colour along its length (#423): its root's and its tip's,
/// and the bars a barred plumage draws across it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FeatherTone {
    /// At the quill.
    pub root: Vec3,
    /// At the tip.
    pub tip: Vec3,
    /// What a bar is drawn in.
    pub bar: Vec3,
    /// How strongly its bars show, `0..=1`: none on an unbarred feather.
    pub barred: f32,
}

impl FeatherTone {
    /// The colour `t` of the way from the quill to the tip.
    ///
    /// **Two bars across a barred feather**, dark about a fifth and four fifths
    /// of the way along and light between and at the tip: W2's cards are
    /// lofted in six rows (`wing::feather`'s `VANE`), so two bars - their rows
    /// at 0.2 and 0.8 - are what a card can draw, an owl's barred remex at a
    /// distance.
    pub(crate) fn at(&self, t: f32) -> Vec3 {
        let colour = self.root.lerp(self.tip, t);
        if self.barred <= 0.0 {
            return colour;
        }
        let bar = 0.5 + 0.5 * (std::f32::consts::TAU * (t - 0.2) / 0.6).det_cos();
        colour.lerp(self.bar, self.barred * step(0.35, 0.9, bar))
    }
}

/// A bird's plumage asked at a point of its body (#423): the colour of the skin
/// there, how glossy it is, and the colour of any feather card rooted there.
/// What [`paint_plumage`](super::paint_plumage) paints the atlas with and every feather card is
/// coloured by.
pub(crate) struct PlumageSampler<'a> {
    plumage: Plumage,
    body: &'a CoatBody,
    fields: Fields,
    /// The spine's direction, from the hips toward the withers.
    spine: Vec3,
    /// The trunk's dorsal axis: square to the spine, toward the sky.
    dorsal: Vec3,
}

impl<'a> PlumageSampler<'a> {
    /// `plumage` on the body `body` reads, ready to be asked.
    pub(crate) fn new(plumage: Plumage, body: &'a CoatBody) -> Self {
        let spine = (body.withers - body.hips).normalize_or(Vec3::Z);
        let dorsal = (Vec3::Y - spine * spine.y).normalize_or(Vec3::Y);
        Self {
            plumage,
            body,
            fields: Fields::new(plumage.marks, body.height),
            spine,
            dorsal,
        }
    }

    /// The skin's colour at `at`, on a surface facing along `normal` in
    /// `zone`: every tract, the pattern, the disc and the sheen, as the atlas
    /// is painted.
    pub(crate) fn colour(&self, at: Vec3, normal: Vec3, zone: Zone) -> Vec3 {
        let plumage = &self.plumage;
        let (head, disc, ruff) = self.face(at, zone);
        let upper = self.upper(normal);
        let legs = self.legs(at);
        let body = rgb(plumage.under)
            .lerp(rgb(plumage.upper), upper)
            .lerp(rgb(plumage.under), legs);
        let (throat, (from, to)) = THROAT;
        let crown = rgb(plumage.head).lerp(
            rgb(plumage.under),
            throat * step(from, to, underside(normal)),
        );
        let ground = body.lerp(crown, head);
        let marked = ground.lerp(
            rgb(plumage.mark),
            self.pattern(at) * self.shows(ground) * (1.0 - disc.max(ruff)),
        );
        let faced = marked
            .lerp(rgb(plumage.disc), disc)
            .lerp(rgb(plumage.ruff), ruff);
        self.sheened(
            faced,
            at,
            self.glossy(upper, head, legs) * (1.0 - disc.max(ruff)),
        )
    }

    /// How rough the skin is at `at`: [`FEATHER_ROUGHNESS`], smoothed by the
    /// gloss where it reaches - the back, the head, not the breast or a disc.
    pub(crate) fn roughness(&self, at: Vec3, normal: Vec3, zone: Zone) -> f32 {
        let (head, disc, ruff) = self.face(at, zone);
        let glossy = self.glossy(self.upper(normal), head, self.legs(at)) * (1.0 - disc.max(ruff));
        FEATHER_ROUGHNESS - GLOSS_SMOOTHING * self.plumage.gloss * glossy
    }

    /// **The colour of a feather card rooted at `root`** (#423), on a surface
    /// facing along `normal` in `zone`, by the kind of feather it is: what
    /// every feather a bird grows is coloured by, W2's flight feathers and
    /// tail fan here and a contour feather on the body or the head wherever
    /// it roots. Each root darker than its tip, as a feather's base is under
    /// the ones over it; a flight feather or a tail feather barred on a barred
    /// plumage, a covert's tip spotted pale on a spotted one, every card of
    /// the back and the wings glossed as the skin is.
    ///
    /// - a remex ([`Feather::Remex`]) takes `flight`;
    /// - a covert and an upper tail covert ([`Feather::Covert`],
    ///   [`Feather::TailCovert`]) take `upper`, the tracts they lie over;
    /// - a rectrix ([`Feather::Rectrix`]) takes `tail`.
    ///
    /// A body's contour feathers are not asked here: each takes the skin's
    /// own colour at its root ([`Self::colour`]) and shades itself
    /// (`hair::fur::contour`).
    pub(crate) fn card(&self, root: Vec3, feather: Feather) -> FeatherTone {
        let plumage = &self.plumage;
        let gloss = 1.0;
        let main = match feather {
            Feather::Remex => rgb(plumage.flight),
            Feather::Covert | Feather::TailCovert => rgb(plumage.upper),
            Feather::Rectrix => rgb(plumage.tail),
        };
        let main = self.sheened(main, root, gloss);
        let shows = self.shows(main);
        let mark = rgb(plumage.mark);
        let (root_share, tip, barred) = match (feather, plumage.pattern) {
            (Feather::Remex | Feather::Rectrix, PlumagePattern::Barred) => (0.72, main, shows),
            (Feather::Covert | Feather::TailCovert, PlumagePattern::Barred) => {
                (0.85, main, 0.6 * shows)
            }
            // A pale or a dark spot at the tip of about every other covert:
            // which ones by where it roots, so the two wings agree.
            (Feather::Covert, PlumagePattern::Spotted) => {
                let spotted = step(0.0, 0.3, self.fields.grain(root.abs(), 30.0));
                (0.85, main.lerp(mark, 0.6 * shows * spotted), 0.0)
            }
            (Feather::Remex | Feather::Rectrix, _) => (0.72, main, 0.0),
            _ => (0.85, main, 0.0),
        };
        FeatherTone {
            root: main * root_share,
            tip,
            bar: mark,
            barred,
        }
    }

    /// How much of the upper tract a surface facing along `normal` is, `0` on
    /// the breast to `1` on the back.
    fn upper(&self, normal: Vec3) -> f32 {
        step(FLANK.0, FLANK.1, normal.dot(self.dorsal))
    }

    /// How much of a hind leg `at` is: a bird's feathered leg, which wears the
    /// under colour. Its wings' arms lie inside the body and are no leg.
    fn legs(&self, at: Vec3) -> f32 {
        self.body
            .junctions
            .leg(at)
            .filter(|leg| matches!(leg.limb, Limb::HindLeft | Limb::HindRight))
            .map_or(0.0, |leg| leg.share)
    }

    /// How much of the head `at` is, and of an owl's disc and its ruff: all of
    /// the head on its zone, fading out down the neck by how far off it a
    /// texel lies (#395's rule); the disc's face on its carved dish, the ruff
    /// on its rim (#422).
    fn face(&self, at: Vec3, zone: Zone) -> (f32, f32, f32) {
        let Some(head) = self.body.head.as_ref() else {
            return (0.0, 0.0, 0.0);
        };
        let local = head.local(at);
        let share = if zone == Zone::Head {
            1.0
        } else {
            1.0 - step(0.3, 1.0, head.distance(local) / head.radius)
        };
        if share <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        // The ruff's surface stands about one of its radii off its middle
        // line, so its colour reaches a little past that: within it, on the
        // tube's crown, and fading out over the face and the crown either
        // side, where #422's shade by vertex had spread it over its facets.
        let ruff = head
            .on_ruff(local)
            .map_or(0.0, |off| step(RUFF_REACH.1, RUFF_REACH.0, off));
        let disc = head.in_dish(local).unwrap_or(0.0);
        (share, disc * share, ruff * share)
    }

    /// How strongly the pattern's mark lies at `at`, `0..=1`, before the tract
    /// decides whether it shows.
    fn pattern(&self, at: Vec3) -> f32 {
        let fields = &self.fields;
        let along = at.dot(self.spine);
        match self.plumage.pattern {
            PlumagePattern::Plain => 0.0,
            PlumagePattern::Spotted => step(0.24, 0.15, fields.cells(at, SPOTS).0),
            PlumagePattern::Barred => {
                // Narrow, wavering and broken, as a snowy owl's are: drawn
                // whole round the body they read as a striped jersey.
                let phase = along / self.body.height * BARS + 0.15 * fields.warp(at, 3.0);
                let broken = step(-0.35, 0.05, fields.patches(at, 9.0));
                step(0.6, 0.88, (std::f32::consts::TAU * phase).det_sin()) * broken
            }
            PlumagePattern::Streaked => {
                // Cells drawn out along the spine: each a narrow streak.
                let drawn = at - self.spine * (along * (1.0 - 1.0 / STREAK_LENGTH));
                step(0.22, 0.12, fields.cells(drawn, STREAKS).0)
            }
        }
    }

    /// How much a mark shows on a ground of `colour`: by how far apart their
    /// luminances lie.
    fn shows(&self, colour: Vec3) -> f32 {
        let gap = (luminance(rgb(self.plumage.mark)) - luminance(colour)).abs();
        step(CONTRAST.0, CONTRAST.1, gap)
    }

    /// How much of the gloss reaches a texel this much of the upper tract, the
    /// head and a leg: the back and the wings' coverts, most of the head, none
    /// of the breast or the legs.
    fn glossy(&self, upper: f32, head: f32, legs: f32) -> f32 {
        (upper * (1.0 - legs)).max(0.8 * head)
    }

    /// `colour` with the gloss's sheen laid into it, `share` of the way.
    fn sheened(&self, colour: Vec3, at: Vec3, share: f32) -> Vec3 {
        let amount = self.plumage.gloss * share * SHEEN_SHARE;
        if amount <= 0.0 {
            return colour;
        }
        let hue = SHEEN.0.lerp(SHEEN.1, 0.5 + 0.5 * self.fields.warp(at, 2.5));
        (colour + hue * amount).min(Vec3::ONE)
    }
}

/// An sRGB triple as a colour.
fn rgb(colour: [f32; 3]) -> Vec3 {
    Vec3::from_array(colour)
}

/// How light a colour reads, as a person's eye weighs its channels.
fn luminance(colour: Vec3) -> f32 {
    colour.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}
