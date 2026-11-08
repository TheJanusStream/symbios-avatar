//! What a winged body's feet are (#422): a bird's toes, a bat's, a sugar
//! glider's paws.
//!
//! **A bird's and a bat's toes are drawn as a part; a glider's paws are a
//! beast's.** A bird stands on four toes radiating from the end of its
//! tarsometatarsus, a bat hangs from five hooked ones, and neither is a pad
//! with bumps on it: each toe is a chain of bones a grip will one day curl,
//! and a talon curves away from the toe it ends. So each is laid out here as
//! a chain of points in the foot's own frame (`FootShape`'s: the origin on the
//! ground under the pad, `+Z` toward the toes, `+X` across), from which
//! `extremity::toes` attaches a digit joint at every point and draws a tube
//! along them, and from which the coat painter tells a talon from a scaled
//! toe. The body's own foot is shaped under them into what they grow from: a
//! bird's ball, a bat's sole. A glider's paws are the beast's analytic foot
//! with a glider's digits - five clawed ones on the fore paw; on the hind, an
//! opposable, clawless hallux and the second and third toes fused (research-10
//! 3.3, S31) - since what a glider grips with is its whole paw.
//!
//! **The kind is the leading family's**, as the posture is (the owner's call
//! of 2026-11-05): a cross stands on its leading family's feet, the toes as
//! long as its blended frame makes them.
//!
//! docs/winged-anatomy.md, "The feet", has the targets, their sources and
//! every departure.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::det;

/// How a bird's four toes are arranged (#422).
///
/// Written as its lowercase name. Not stored: a body's arrangement is derived
/// from what it is (`WingedParams::toes`), so nothing a record carries reads
/// as one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Toes {
    /// Three toes forward and the hallux back: about nine birds in ten,
    /// a crow's and a songbird's (research-10 3.1, S17).
    #[default]
    Anisodactyl,
    /// The second and third toes forward, the hallux and the fourth back: a
    /// parrot's, and an owl's when it perches (S17: an owl's fourth toe
    /// turns back at will).
    Zygodactyl,
}

impl Toes {
    /// Every arrangement.
    pub const ALL: [Toes; 2] = [Toes::Anisodactyl, Toes::Zygodactyl];

    /// Its name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Toes::Anisodactyl => "anisodactyl",
            Toes::Zygodactyl => "zygodactyl",
        }
    }
}

/// What a winged body's feet carry beyond a beast's paw (#422): the leading
/// family's, declared on its skeleton (`BeastFeet::winged`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WingedFoot {
    /// A bird's: four toes on the tarsometatarsus, arranged so, drawn as a
    /// part on a ball the body's foot is shaped into.
    Bird(Toes),
    /// A bat's: five near-equal toes with hooked claws, drawn as a part on a
    /// sole the body's foot is shaped into.
    Bat,
    /// A sugar glider's: a beast's paws, five clawed digits on the fore
    /// paws, and on the hind an opposable, clawless hallux and the second and
    /// third toes fused.
    Glider,
}

impl WingedFoot {
    /// Whether its toes are a drawn part rather than the paw's own shape.
    #[must_use]
    pub fn drawn(self) -> bool {
        !matches!(self, WingedFoot::Glider)
    }

    /// How far ahead of the pad the plan sets the foot's second node, as a
    /// share of the frame's toe measure (`plan::derive::winged`): a bird's
    /// ball where its toes meet, a bat's sole. The toes drawn on it reach the
    /// rest. A glider's paw keeps the beast's node, the whole measure.
    pub(crate) fn sole_share(self) -> f32 {
        match self {
            WingedFoot::Bird(_) => BIRD_BALL,
            WingedFoot::Bat => BAT_SOLE,
            WingedFoot::Glider => 1.0,
        }
    }

    /// How long its toes are, in metres, a foot whose second node stands
    /// `reach` ahead of its pad: a bird's middle toe the frame's whole toe
    /// measure, from its root in the ball; a bat's toes the rest of its foot
    /// past the sole. None on a glider's paw.
    fn toe_unit(self, reach: f32) -> f32 {
        let foot = reach / self.sole_share();
        match self {
            WingedFoot::Bird(_) => foot,
            WingedFoot::Bat => foot * (1.0 - BAT_SOLE),
            WingedFoot::Glider => 0.0,
        }
    }
}

/// How much of a bird's toe measure its ball takes, pad to the foot's second
/// node: the toes' roots sit in it, and the measure is the middle toe's
/// length from there, its claw counted. Set so the node stands where a dog's does, 1.6 pad
/// radii ahead, which the cage meshes cleanly.
/// Provenance: **chosen by render** (#422).
const BIRD_BALL: f32 = 0.42;

/// How much of a bat's foot its sole takes, heel to the toes' roots: the hind
/// foot is 0.4 of a trunk to its claws' tips (S21), and its toes a little
/// under half of it (bg, L).
/// Provenance: **bg**, chosen by render (#422).
const BAT_SOLE: f32 = 0.62;

/// One toe as drawn, in its foot's frame (#422).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawnToe {
    /// Which digit, `1` the hallux to `5`.
    pub(crate) digit: u8,
    /// Its joints from the root to the claw's tip: one at the start of every
    /// bone and a leaf at the tip, so `bones() == joints.len() - 1`. The last
    /// bone is the claw.
    pub(crate) joints: Vec<Vec3>,
    /// The claw's curve, from its base (the last joint but one) to its tip
    /// (the last), sampled.
    pub(crate) claw: Vec<Vec3>,
    /// Its radius at the root, the phalanges' tapering from there.
    pub(crate) radius: f32,
    /// Its radius at each of its phalanges' joints, the root to the claw's
    /// base: what stood each joint at its height.
    pub(crate) radii: Vec<f32>,
    /// How much of its radius its pads bulge below it.
    pub(crate) pads: f32,
}

impl DrawnToe {
    /// How many bones it has, the claw counted: research-10's phalanges.
    pub(crate) fn bones(&self) -> usize {
        self.joints.len() - 1
    }

    /// Its radius at `share` of the way along its phalanges, `0` at the root
    /// and `1` at the claw's base.
    fn radius_at(&self, share: f32) -> f32 {
        self.radius * (1.0 - TAPER * share.clamp(0.0, 1.0))
    }

    /// The claw's radius at `share` of the way from its base to its tip.
    pub(crate) fn claw_radius_at(&self, share: f32) -> f32 {
        self.radius_at(1.0) * CLAW_BASE * (1.0 - share.clamp(0.0, 1.0)).max(0.0)
    }
}

/// How much a toe's radius falls from its root to its claw's base.
const TAPER: f32 = 0.35;

/// A claw's radius at its base, as a share of the toe's there.
const CLAW_BASE: f32 = 0.9;

/// How high above the ground a claw's tip rests, as a share of the toe's
/// root radius: a talon resting on the ground, not driven into it.
const TIP_CLEARANCE: f32 = 0.2;

/// One toe's recipe: which digit, which way it points (radians from straight
/// ahead, toward the foot's outside), how long it is as a share of the foot's
/// toe unit, its bones as shares of that (the claw last), its root radius in
/// pad radii and how far its claw curls, in radians.
struct Recipe {
    digit: u8,
    yaw: f32,
    length: f32,
    bones: &'static [f32],
    radius: f32,
    curl: f32,
}

/// A bird's phalanges, the claw counted: hallux 2, II 3, III 4, IV 5
/// (research-10 3.1, S17, H). The shares of each toe are **bg** - a toe's
/// proximal phalanges longest but for the fourth's, which are many and
/// short - and the owl's talons long (chosen by render).
const BIRD_I: &[f32] = &[0.50, 0.50];
/// See [`BIRD_I`].
const BIRD_II: &[f32] = &[0.34, 0.28, 0.38];
/// See [`BIRD_I`].
const BIRD_III: &[f32] = &[0.27, 0.22, 0.18, 0.33];
/// See [`BIRD_I`].
const BIRD_IV: &[f32] = &[0.19, 0.15, 0.13, 0.13, 0.40];

/// A bat's phalanges, the claw counted: hallux 2, the others 3, the
/// mammal's formula (bg, L; research-10 measures none).
const BAT_I: &[f32] = &[0.50, 0.50];
/// See [`BAT_I`].
const BAT_OTHERS: &[f32] = &[0.34, 0.26, 0.40];

/// A bird's toes, each a share of its middle toe's length: the hallux about
/// half, II three quarters, IV between (bg, L; research-10 gives no toe
/// lengths). Thick and short, with long curved talons: an owl's, the
/// default bird (chosen by render).
fn bird(toes: Toes) -> [Recipe; 4] {
    let back = std::f32::consts::PI;
    let (hallux, second, third, fourth) = match toes {
        Toes::Anisodactyl => (-(back - 0.12), -0.42, 0.0, 0.40),
        Toes::Zygodactyl => (-(back - 0.38), -0.26, 0.10, back - 0.48),
    };
    [
        Recipe {
            digit: 1,
            yaw: hallux,
            length: 0.55,
            bones: BIRD_I,
            radius: 0.44,
            curl: 1.5,
        },
        Recipe {
            digit: 2,
            yaw: second,
            length: 0.78,
            bones: BIRD_II,
            radius: 0.42,
            curl: 1.4,
        },
        Recipe {
            digit: 4,
            yaw: fourth,
            length: 0.86,
            bones: BIRD_IV,
            radius: 0.40,
            curl: 1.4,
        },
        // The middle toe last, as W3 laid it out. The foot's tip
        // (`Landmark::LimbTip`) is not its claw: it is the plan's toe node
        // (#430), whatever order the toes are attached in.
        Recipe {
            digit: 3,
            yaw: third,
            length: 1.0,
            bones: BIRD_III,
            radius: 0.42,
            curl: 1.4,
        },
    ]
}

/// A bat's five near-equal toes in a narrow fan, the hallux no shorter than
/// the rest and not opposed (bg, L), each with a strongly hooked claw: what a
/// bat hangs from (research-10 6.2).
fn bat() -> [Recipe; 5] {
    let toe = |digit: u8, yaw: f32, length: f32| Recipe {
        digit,
        yaw,
        length,
        bones: if digit == 1 { BAT_I } else { BAT_OTHERS },
        radius: 0.46,
        curl: 2.2,
    };
    [
        toe(1, -0.24, 0.90),
        toe(2, -0.12, 0.98),
        toe(4, 0.12, 0.98),
        toe(5, 0.24, 0.92),
        // The middle toe last, for the bird's reason.
        toe(3, 0.0, 1.0),
    ]
}

/// Every toe a drawn foot carries, in its frame: `pad` its pad's radius and
/// `reach` how far ahead of the pad its second node stands, both in metres;
/// `lateral` `+1` where the frame's `+X` is the foot's outside (a left foot)
/// and `-1` where it is the inside. Empty for a foot whose toes are not
/// drawn.
pub(crate) fn drawn(foot: WingedFoot, pad: f32, reach: f32, lateral: f32) -> Vec<DrawnToe> {
    let unit = foot.toe_unit(reach);
    match foot {
        WingedFoot::Bird(toes) => {
            // Rooted round the middle of the ball, each a little out along
            // its own heading: the trochleae at the tarsometatarsus's end.
            let centre = Vec3::new(0.0, 0.0, 0.5 * reach);
            bird(toes)
                .iter()
                .map(|recipe| {
                    lay(recipe, unit, pad, lateral, |heading| {
                        centre + heading * 0.3 * pad
                    })
                })
                .collect()
        }
        WingedFoot::Bat => bat()
            .iter()
            .map(|recipe| {
                // Rooted across the sole's front, the hallux on its inside.
                let across = (f32::from(recipe.digit) - 3.0) * 0.42 * pad * lateral;
                let root = Vec3::new(across, 0.0, reach - 0.25 * pad);
                lay(recipe, unit, pad, lateral, |_| root)
            })
            .collect(),
        WingedFoot::Glider => Vec::new(),
    }
}

/// Lays one toe out from its root along its heading, its phalanges on the
/// ground - the pads under them touching it - and its claw curving down from
/// the last to rest its tip just off the ground.
fn lay(
    recipe: &Recipe,
    unit: f32,
    pad: f32,
    lateral: f32,
    root: impl Fn(Vec3) -> Vec3,
) -> DrawnToe {
    let (sin, cos) = det::sin_cos(recipe.yaw);
    let heading = Vec3::new(lateral * sin, 0.0, cos);
    let length = recipe.length * unit;
    let radius = recipe.radius * pad;
    let pads = PAD_BULGE;
    let mut toe = DrawnToe {
        digit: recipe.digit,
        joints: Vec::with_capacity(recipe.bones.len() + 1),
        claw: Vec::new(),
        radius,
        radii: Vec::with_capacity(recipe.bones.len()),
        pads,
    };
    // The phalanges: each joint at the height its own radius and the pad
    // beneath it stand it at.
    let phalanges = &recipe.bones[..recipe.bones.len() - 1];
    let span: f32 = phalanges.iter().sum::<f32>().max(f32::EPSILON);
    let start = root(heading);
    let mut run = 0.0f32;
    for share in std::iter::once(0.0).chain(phalanges.iter().copied()) {
        run += share;
        let along = start + heading * (run * length);
        let radius = toe.radius_at(run / span);
        toe.radii.push(radius);
        toe.joints
            .push(Vec3::new(along.x, radius * (1.0 + pads), along.z));
    }
    // The claw: an arc of `curl` radians, lifted at its base just enough that
    // its tip comes down to rest off the ground rather than in it.
    let base = *toe.joints.last().unwrap_or(&start);
    let claw = recipe.bones[recipe.bones.len() - 1] * length;
    let curl = recipe.curl.max(1.0e-3);
    let bend = claw / curl;
    let drop = (base.y - TIP_CLEARANCE * radius).max(0.0);
    let lift = lift_for(bend, curl, drop);
    let at = |phi: f32| {
        let (sin_a, cos_a) = det::sin_cos(lift);
        let (sin_b, cos_b) = det::sin_cos(lift - phi);
        base + heading * (bend * (sin_a - sin_b)) + Vec3::Y * (bend * (cos_b - cos_a))
    };
    toe.claw = (0..=CLAW_SAMPLES)
        .map(|step| at(curl * step as f32 / CLAW_SAMPLES as f32))
        .collect();
    toe.joints.push(at(curl));
    toe
}

/// How much a toe's pads bulge below it, as a share of its radius.
const PAD_BULGE: f32 = 0.35;

/// How many pieces a claw's curve is drawn in.
pub(crate) const CLAW_SAMPLES: usize = 4;

/// How far up from level a claw of radius `bend` curling `curl` radians must
/// leave its toe for its tip to come down `drop` and no further: none if a
/// claw leaving level drops less, and never past the half curl, at which it
/// comes back to the height it left.
fn lift_for(bend: f32, curl: f32, drop: f32) -> f32 {
    use crate::det::DetMath;
    let fall = |lift: f32| bend * (lift.det_cos() - (lift - curl).det_cos());
    if fall(0.0) <= drop {
        return 0.0;
    }
    let (mut low, mut high) = (0.0f32, 0.5 * curl);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        if fall(middle) > drop {
            low = middle;
        } else {
            high = middle;
        }
    }
    high
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAD: f32 = 0.02;

    fn laid(foot: WingedFoot) -> Vec<DrawnToe> {
        drawn(foot, PAD, 1.6 * PAD, 1.0)
    }

    #[test]
    fn a_bird_has_four_toes_on_the_bird_formula() {
        for toes in Toes::ALL {
            let mut formula: Vec<(u8, usize)> = laid(WingedFoot::Bird(toes))
                .iter()
                .map(|toe| (toe.digit, toe.bones()))
                .collect();
            formula.sort_unstable();
            assert_eq!(formula, vec![(1, 2), (2, 3), (3, 4), (4, 5)], "{toes:?}");
        }
    }

    #[test]
    fn every_toe_rests_on_the_ground_and_no_claw_passes_into_it() {
        for foot in [
            WingedFoot::Bird(Toes::Anisodactyl),
            WingedFoot::Bird(Toes::Zygodactyl),
            WingedFoot::Bat,
        ] {
            for toe in laid(foot) {
                let tip = *toe.joints.last().expect("a toe has joints");
                assert!(
                    tip.y >= 0.0 && tip.y < 0.5 * toe.radius,
                    "{foot:?} toe {}: its claw's tip at {} m",
                    toe.digit,
                    tip.y
                );
                for point in &toe.claw {
                    assert!(
                        point.y >= tip.y - 1.0e-6,
                        "{foot:?}: a claw dips below its tip"
                    );
                }
            }
        }
    }

    #[test]
    fn a_bat_s_claws_hook_and_a_bird_s_talons_curve() {
        // How far each claw turns from its first piece to its last.
        let turn = |toe: &DrawnToe| {
            let first = (toe.claw[1] - toe.claw[0]).normalize();
            let last = (toe.claw[CLAW_SAMPLES] - toe.claw[CLAW_SAMPLES - 1]).normalize();
            first.dot(last).clamp(-1.0, 1.0).acos().to_degrees()
        };
        for toe in laid(WingedFoot::Bat) {
            assert!(
                turn(&toe) > 85.0,
                "a bat's claw turns {:.0} deg",
                turn(&toe)
            );
        }
        for toe in laid(WingedFoot::Bird(Toes::Anisodactyl)) {
            assert!(
                turn(&toe) > 55.0,
                "a bird's talon turns {:.0} deg",
                turn(&toe)
            );
        }
    }

    #[test]
    fn a_curl_that_cannot_reach_the_ground_leaves_level() {
        assert_eq!(lift_for(1.0, 0.5, 10.0), 0.0);
        let lift = lift_for(1.0, 2.0, 0.1);
        let fall = 1.0 * (det::sin_cos(lift).1 - det::sin_cos(lift - 2.0).1);
        assert!((fall - 0.1).abs() < 1.0e-4, "the claw falls {fall}");
    }
}
