//! The mouth shapes speech is drawn with, under their industry names.
//!
//! ARKit-52 and Oculus-15 are adopted as NAMING ONLY. An
//! integration driving lipsync — an audio pipeline, a network peer — sends
//! the standard vocabulary, and this crate renders the nearest shape its
//! macro rig can make. The wire format stays stable while the rig grows: a
//! viseme that today aliases another starts reading the day the bone it
//! wants exists, and no caller changes.
//!
//! **What "nearest shape" honestly means here.** The mouth set is two
//! channels — the jaw's parting and the corners' spread — because that is
//! all the macro rig carries. There is no lip-rounding bone and no
//! lip-closure bone, so the fifteen names collapse onto about five visibly
//! distinct shapes, and the table below records every alias rather than
//! letting a render discover it: the bilabials (`PP`) sit AT silence, the
//! rounded vowels (`oh`, `ou`) approximate rounding by narrowing the
//! corners, and the labiodental (`FF`) is a near-shut parting with nothing
//! bitten. An integration should not expect bilabials to read until a lip
//! bone exists.
//!
//! **Precedence, stated once:** while speech runs, the viseme owns the
//! MOUTH — [`Viseme::apply`] writes the jaw and the corners outright, over
//! whatever an [`super::Expression`] rested there — and the expression keeps
//! the brows and the lids. A body that talks while happy holds its brow and
//! its eyes; its mouth is busy saying things. Blending a smile INTO speech
//! is an upstream, expression-space question (scale the corner spread before
//! applying), not a pose-space fight between two writers.
//!
//! The jaw is counted in [`super::talk::TalkConfig::open`]'s own ruler — a
//! viseme is speech at full articulation, not a resting bias, and the
//! expression layer's 4° jaw gain under-articulates every open vowel. `aa`
//! at 1.0 is the full 12° conversational open.

use glam::Quat;

use crate::anim::Pose;
use crate::rig::Rig;
use crate::rig::skin::{jaw_pivot, mouth_corners};

/// Degrees of corner spread at `spread = 1.0`, and of narrowing at `-1.0`.
///
/// The same number as the expression layer's corner gain, so a viseme's
/// spread and a smile's are the same currency — `SS` at 0.35 is about the
/// corner motion of a mild smile, which is what saying an S does to a face.
const SPREAD_GAIN: f32 = 22.0;

/// The Oculus-15 viseme set, by its own names.
///
/// The casing follows the Oculus reference (`sil`, `PP`, `aa`) through
/// [`Viseme::NAMES`]; the variants are Rust-cased.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viseme {
    /// Silence: the mouth at rest.
    Sil,
    /// `p`, `b`, `m` — lips pressed. **Aliases [`Self::Sil`]** until a lip
    /// bone exists: nothing in the rig can press what it cannot hold.
    Pp,
    /// `f`, `v` — a near-shut parting; the bite is not expressible.
    Ff,
    /// `th` — tongue between the teeth, shown as a narrow parting.
    Th,
    /// `t`, `d` — a small parting.
    Dd,
    /// `k`, `g` — a middling parting, back of the mouth.
    Kk,
    /// `ch`, `j`, `sh` — a small parting, slightly spread.
    Ch,
    /// `s`, `z` — teeth close, corners spread.
    Ss,
    /// `n`, `l` — a slight parting.
    Nn,
    /// `r` — a middling parting, corners a touch narrowed.
    Rr,
    /// `aa` — the full conversational open.
    Aa,
    /// `e` — open and spread.
    E,
    /// `ih` — a middling open, slightly spread.
    Ih,
    /// `oh` — tall open, corners narrowed for the rounding the lips cannot
    /// make.
    Oh,
    /// `ou` — small open, corners drawn well in; the nearest thing to a
    /// purse this rig has.
    Ou,
}

impl Viseme {
    /// Every viseme beside its wire name, in the Oculus reference order.
    pub const NAMES: [(&str, Self); 15] = [
        ("sil", Self::Sil),
        ("PP", Self::Pp),
        ("FF", Self::Ff),
        ("TH", Self::Th),
        ("DD", Self::Dd),
        ("kk", Self::Kk),
        ("CH", Self::Ch),
        ("SS", Self::Ss),
        ("nn", Self::Nn),
        ("RR", Self::Rr),
        ("aa", Self::Aa),
        ("E", Self::E),
        ("ih", Self::Ih),
        ("oh", Self::Oh),
        ("ou", Self::Ou),
    ];

    /// The viseme called `name` on the wire, or `None`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::NAMES
            .iter()
            .find(|(wire, _)| *wire == name)
            .map(|(_, viseme)| *viseme)
    }

    /// The mouth this viseme is drawn with: `(parting, spread)`.
    ///
    /// The parting is a share of [`super::talk::TalkConfig::open`]'s 12°
    /// conversational swing, `0..=1`; the spread is the corners' channel,
    /// `-1..=1`, in the same currency as [`super::Expression::corners`].
    /// Aliases are deliberate and named in the enum docs — a table that made
    /// every row unique would be promising distinctions the rig cannot draw.
    #[must_use]
    pub const fn mouth(self) -> (f32, f32) {
        match self {
            Self::Sil | Self::Pp => (0.0, 0.0),
            Self::Ff => (0.08, 0.0),
            Self::Th => (0.15, 0.0),
            Self::Dd => (0.20, 0.0),
            Self::Kk => (0.25, 0.0),
            Self::Ch => (0.20, 0.15),
            Self::Ss => (0.12, 0.35),
            Self::Nn => (0.10, 0.0),
            Self::Rr => (0.20, -0.10),
            Self::Aa => (1.00, 0.0),
            Self::E => (0.45, 0.45),
            Self::Ih => (0.30, 0.25),
            Self::Oh => (0.70, -0.30),
            Self::Ou => (0.35, -0.50),
        }
    }

    /// Writes this viseme's mouth onto `pose`: the jaw and both corners,
    /// outright.
    ///
    /// Write-over is the precedence in the module docs — speech owns the
    /// mouth while it runs, so a caller applies its [`super::Expression`]
    /// first and this after, and the brows and lids come through untouched.
    pub fn apply(&self, rig: &Rig, pose: &mut Pose) {
        let (parting, spread) = self.mouth();
        let open = super::talk::TalkConfig::default().open;
        if let Some(pivot) = jaw_pivot(rig)
            && let Some(rotation) = pose.rotations.get_mut(pivot)
        {
            *rotation = Quat::from_rotation_x(parting * open);
        }
        for (corner, sign, _) in mouth_corners(rig) {
            if let Some(rotation) = pose.rotations.get_mut(corner) {
                *rotation = Quat::from_rotation_z(sign * (spread * SPREAD_GAIN).to_radians());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_viseme_owns_the_mouth_and_only_the_mouth() {
        // #218's P4: the vocabulary is a pose layer over exactly three
        // joints — the jaw's pivot and the two corners. A viseme that moved
        // a brow would be speech raising eyebrows on every syllable.
        let record = crate::AvatarRecord::new("Spoken", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let rest = Pose::rest(rig);
        // `ou`, because both of its channels are nonzero: `aa` spreads by
        // zero and its corners legitimately stay at rest, which this guard's
        // first draft asserted against and promptly learned from.
        let mut posed = rest.clone();
        Viseme::Ou.apply(rig, &mut posed);

        let mut expected: Vec<usize> = crate::rig::skin::mouth_corners(rig)
            .into_iter()
            .map(|(joint, _, _)| joint)
            .chain(crate::rig::skin::jaw_pivot(rig))
            .collect();
        expected.sort_unstable();
        let moved: Vec<usize> = (0..rig.len())
            .filter(|&joint| posed.rotations[joint] != rest.rotations[joint])
            .collect();
        assert_eq!(moved, expected, "ou moved joints outside the mouth set");
    }

    #[test]
    fn silence_is_exactly_rest_and_every_shape_is_in_range() {
        let record = crate::AvatarRecord::new("Silent", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let rest = Pose::rest(rig);
        let mut posed = rest.clone();
        Viseme::Sil.apply(rig, &mut posed);
        assert_eq!(
            posed.rotations, rest.rotations,
            "silence posed something — sil has to be the rest mouth exactly"
        );

        for (name, viseme) in Viseme::NAMES {
            let (parting, spread) = viseme.mouth();
            assert!(
                (0.0..=1.0).contains(&parting) && (-1.0..=1.0).contains(&spread),
                "{name}: mouth ({parting}, {spread}) is outside the channels' ranges"
            );
        }
        // The one alias promised by name: bilabials sit at silence until a
        // lip bone exists, DECLARED rather than discovered.
        assert_eq!(Viseme::Pp.mouth(), Viseme::Sil.mouth());
    }
}
