//! Hair, grown as strand groups over a measured scalp.
//!
//! Hair is the single strongest signal of who a character is — more than the
//! face, at the distances a game is actually played from, because it is the
//! outline you read before any feature resolves. It is also the part that most
//! reliably gives away procedural characters, since a body plan produces a bald
//! skull and a bald skull reads as a mannequin however well it is proportioned.
//!
//! The construction follows the one the tools converged on: a modest number of
//! **strand groups**, each rooted on the scalp, each following the skull down to
//! the hairline before falling free. Layering rows of them from crown to
//! hairline gives coverage the way overlapping cards do, without any single
//! group having to be shaped by hand.
//!
//! Two decisions are worth stating, because both are the opposite of the obvious
//! one:
//!
//! - The scalp is **measured from the built mesh**, never derived from the body
//!   plan's numbers. See [`Scalp`] for what that costs and why it is necessary.
//! - Fall length **varies with azimuth**. Uniform-length hair falls off the brow
//!   straight down the face, which is a curtain, not a hairstyle. Real long hair
//!   is long at the back and a fringe at the front, and that difference is most
//!   of what makes a head of hair look like one.
//!
//! Everything is built in head-local space, as the eyes are, so a renderer
//! parents it to the head joint and the hair follows the body for free.

pub mod scalp;
pub mod strand;

use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::rig::{Rig, Surface};

pub use scalp::Scalp;
pub use strand::Strand;

/// How many rows of roots run from the crown down to the hairline.
///
/// Layering matters more than raw count: one row of very wide ribbons reads as a
/// helmet, because a helmet is exactly what a single shell is.
const ROWS: usize = 4;

/// The fewest columns of strand groups worth building.
const MIN_COLUMNS: usize = 8;

/// How far a strand group's root is pushed away from the parting, in radians.
///
/// A parting is a line, not a clearing. At three times this the front view had a
/// bald wedge running from the crown to the brow.
const PART_PUSH: f32 = 0.12;

/// How far either side of the parting that push reaches, in radians.
const PART_SPREAD: f32 = 0.75;

/// How many points sample a strand's cap and its fall.
const CAP_STEPS: usize = 5;
const FALL_STEPS: usize = 9;

/// How shape parameters describe a head of hair.
///
/// Axes run `-1` to `+1` where a direction is meant and `0` to `1` where an
/// amount is, matching the body plan's convention.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HairParams {
    /// How far the hair falls, `0` cropped and `1` past the shoulder blades.
    #[serde(default, with = "crate::plan::scaled")]
    pub length: f32,
    /// How far the hair stands off the scalp, `-1` flat and `+1` full.
    #[serde(default, with = "crate::plan::scaled")]
    pub volume: f32,
    /// Where the hairline sits, `-1` receding and `+1` low on the brow.
    #[serde(default, with = "crate::plan::scaled")]
    pub coverage: f32,
    /// Where the parting runs, `-1` over the left ear and `+1` over the right.
    #[serde(default, with = "crate::plan::scaled")]
    pub part: f32,
    /// How much the hair waves as it falls, `0` straight and `1` curling.
    #[serde(default, with = "crate::plan::scaled")]
    pub wave: f32,
    /// Colour along a melanin ramp, `0` black and `1` pale blonde.
    #[serde(default, with = "crate::plan::scaled")]
    pub shade: f32,
    /// How many strand groups to grow.
    #[serde(default)]
    pub groups: u32,
}

impl Default for HairParams {
    fn default() -> Self {
        Self {
            length: 0.45,
            volume: 0.0,
            coverage: 0.0,
            part: 0.0,
            wave: 0.25,
            shade: 0.3,
            groups: 96,
        }
    }
}

/// The fewest and most strand groups a record may ask for.
///
/// A floor because a handful of ribbons is a wig, not hair; a ceiling because
/// group count is the one axis that costs geometry, and a record read off the
/// network is not to be trusted with that.
pub const MIN_GROUPS: u32 = 24;
/// See [`MIN_GROUPS`].
pub const MAX_GROUPS: u32 = 256;

impl HairParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.length = quantize(finite(self.length, 0.45).clamp(0.0, 1.0));
        self.wave = quantize(finite(self.wave, 0.25).clamp(0.0, 1.0));
        self.shade = quantize(finite(self.shade, 0.3).clamp(0.0, 1.0));
        self.volume = quantize(finite(self.volume, 0.0).clamp(-1.0, 1.0));
        self.coverage = quantize(finite(self.coverage, 0.0).clamp(-1.0, 1.0));
        self.part = quantize(finite(self.part, 0.0).clamp(-1.0, 1.0));
        self.groups = self.groups.clamp(MIN_GROUPS, MAX_GROUPS);
    }

    /// The hair's colour, along a melanin ramp.
    ///
    /// Dark hair is very dark — the common mistake is a mid-brown that reads as
    /// dusty — and the ramp reddens through the middle before it lightens,
    /// because that is the order melanin actually gives up.
    #[must_use]
    pub fn colour(&self) -> [f32; 3] {
        const RAMP: [[f32; 3]; 5] = [
            [0.021, 0.017, 0.014],
            [0.098, 0.055, 0.036],
            [0.275, 0.148, 0.070],
            [0.545, 0.355, 0.140],
            [0.820, 0.660, 0.350],
        ];
        let along = self.shade.clamp(0.0, 1.0) * (RAMP.len() - 1) as f32;
        let stop = (along.floor() as usize).min(RAMP.len() - 2);
        let blend = along - stop as f32;
        let (low, high) = (RAMP[stop], RAMP[stop + 1]);
        [
            low[0] + (high[0] - low[0]) * blend,
            low[1] + (high[1] - low[1]) * blend,
            low[2] + (high[2] - low[2]) * blend,
        ]
    }
}

/// Substitutes `fallback` for a non-finite value.
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// A head of hair, in head-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Hair {
    /// The groups it is made of.
    pub groups: Vec<Strand>,
    /// The head joint the hair is parented to.
    pub head: usize,
    /// The colour it should be shaded.
    pub colour: [f32; 3],
}

impl Hair {
    /// Grows hair over a built body.
    ///
    /// Needs the mesh, not just the rig: the scalp is measured rather than
    /// assumed, which is what keeps the hair on the head.
    #[must_use]
    pub fn build(mesh: &PolyMesh, rig: &Rig, params: &HairParams) -> Option<Self> {
        let scalp = Scalp::measure(mesh, rig)?;
        let surface = Surface::measure(mesh, rig);
        Some(Self::over(&scalp, Some((rig, &surface)), params))
    }

    /// Grows hair over an already-measured scalp.
    ///
    /// Given the body's rig, falling hair is pushed clear of whatever it lands
    /// on. Without that, anything past a bob hangs *inside* the chest and reads
    /// as dark stripes painted on the skin.
    #[must_use]
    pub fn over(scalp: &Scalp, body: Option<(&Rig, &Surface)>, params: &HairParams) -> Self {
        let radius = scalp.radius();
        let columns = (params.groups as usize / ROWS).max(MIN_COLUMNS);
        let step = TAU / columns as f32;

        let thickness = radius * 0.035;

        // Rooted a hair's breadth below the crown rather than at it: the crown
        // is a pole, so every column's top root converges there, which is the
        // whorl real hair has.
        let crown = scalp.top() - 0.04;
        let stand = radius * (0.05 + 0.05 * params.volume.clamp(-1.0, 1.0));
        let reach = radius * (0.15 + 3.8 * params.length.clamp(0.0, 1.0));
        // A fringe is a fringe whatever the rest is doing: it stops above the
        // eyes or it is not a fringe, so it barely tracks the length axis.
        let fringe = radius * (0.10 + 0.22 * params.length.clamp(0.0, 1.0));
        let waving = radius * 0.28 * params.wave.clamp(0.0, 1.0);

        // Sized at the radius a strand actually sits at — the skull plus the
        // stand-off plus the layers stacked on it — rather than at the skull.
        //
        // The factor is what makes a lock read as a lock. Rows are staggered
        // across the column gap, so coverage is four times finer than this and
        // was never the thing leaving gaps; widening ribbons to close them
        // produced slabs nearly as broad as the head. The gaps were the wave
        // pulling neighbours apart, and that is fixed where the wave is.
        let outermost = scalp.width_at(0.0) * radius + stand + (ROWS - 1) as f32 * thickness * 1.2;
        let width = outermost * step * 0.62;

        let mut groups = Vec::with_capacity(columns * ROWS);
        for column in 0..columns {
            for row in 0..ROWS {
                // Rows are staggered *across* the gap between columns as well as
                // stacked down it. Without the stagger, all four rows of a
                // column reach the hairline at one point and fall down one line
                // — four ribbons in the same place and bare gaps between them,
                // which is exactly how it first rendered.
                let spoke = column as f32 + (row as f32 + 0.5) / ROWS as f32;
                let azimuth = part_aside(step * spoke, params.part);
                let hairline = hairline(scalp, azimuth, params.coverage);
                if hairline >= crown {
                    continue;
                }

                // The top row roots at the whorl itself. Starting it even an
                // eighth of the way down left a bare patch at the crown, since
                // every strand travels downward and nothing covers what is
                // above the highest root.
                let down = row as f32 / ROWS as f32;
                let root = crown + (hairline - crown) * down;

                // Layering. The strand rooted at the crown travels over all the
                // others to reach the hairline, so it lies furthest out and
                // hangs longest; the ones rooted low are the underlayer.
                let layer = (ROWS - 1 - row) as f32;
                let lift = stand + layer * thickness * 1.2;
                let front = frontness(azimuth);
                let fall = (reach + (fringe - reach) * front) * (1.0 - 0.15 * row as f32);

                let path = sweep(
                    scalp,
                    body,
                    Fall {
                        azimuth,
                        root,
                        hairline,
                        stand: lift,
                        fall,
                        waving,
                        // Phase follows the azimuth, so neighbours wave
                        // together the way a lock of hair does. A phase per
                        // strand looks like corrugated iron.
                        phase: azimuth * 3.0,
                        clearance: thickness * 1.5,
                    },
                );
                if path.len() < 2 {
                    continue;
                }
                groups.push(Strand {
                    path,
                    width,
                    thickness,
                    across: Vec3::new(azimuth.cos(), 0.0, -azimuth.sin()),
                });
            }
        }

        Self {
            groups,
            head: scalp.head,
            colour: params.colour(),
        }
    }

    /// Every group swept into one mesh.
    ///
    /// The result is not a manifold — strand groups overlap, which is the point
    /// of them — though each group on its own is a closed solid.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for group in &self.groups {
            mesh.append(&group.mesh());
        }
        mesh
    }

    /// How far the lowest tip hangs below the head joint, in metres.
    #[must_use]
    pub fn drop(&self) -> f32 {
        -self
            .groups
            .iter()
            .map(Strand::tip)
            .fold(0.0f32, |low, tip| low.min(tip.y))
    }
}

/// How one strand group falls.
struct Fall {
    /// Which way round the head it is rooted.
    azimuth: f32,
    /// The root's height, in head radii.
    root: f32,
    /// The height its cap phase ends at, in head radii.
    hairline: f32,
    /// How far it stands off the scalp, in metres.
    stand: f32,
    /// How far it falls once free of the scalp, in metres.
    fall: f32,
    /// How far it waves as it falls, in metres.
    waving: f32,
    /// Where in the wave it starts.
    phase: f32,
    /// How far clear of the body it is kept, in metres.
    clearance: f32,
}

/// The centre-line of one strand group.
///
/// Two phases: down the scalp to the hairline, then falling free. The first is
/// what makes hair look attached, and skipping it — rooting every group at the
/// crown and letting it fall — is what makes procedural hair look like a mop
/// dropped on a head.
fn sweep(scalp: &Scalp, body: Option<(&Rig, &Surface)>, fall: Fall) -> Vec<Vec3> {
    let Fall { azimuth, .. } = fall;
    let mut path = Vec::with_capacity(CAP_STEPS + FALL_STEPS);

    for step in 0..=CAP_STEPS {
        let along = step as f32 / CAP_STEPS as f32;
        let height = fall.root + (fall.hairline - fall.root) * along;
        path.push(scalp.point(azimuth, height) + scalp.normal(azimuth, height) * fall.stand);
    }

    let from = *path.last().expect("the cap always has points");
    let radial = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
    for step in 1..=FALL_STEPS {
        let along = step as f32 / FALL_STEPS as f32;
        // Widening as it falls, so long hair drapes over the shoulders instead
        // of hanging inside them.
        let flare = 1.0 + 0.22 * along;
        let wave = (along * TAU * 1.6 + fall.phase).sin() * fall.waving * along;
        let point = Vec3::new(
            from.x * flare + radial.x * wave,
            from.y - fall.fall * along,
            from.z * flare + radial.z * wave,
        );
        path.push(match body {
            Some((rig, surface)) => {
                surface.clear(rig, point + scalp.origin(), fall.clearance) - scalp.origin()
            }
            None => point,
        });
    }

    path.dedup_by(|a, b| a.distance_squared(*b) < 1e-10);
    path
}

/// How much a direction faces the front, from `0` at the back to `1` at the face.
fn frontness(azimuth: f32) -> f32 {
    let facing = 0.5 + 0.5 * azimuth.cos();
    facing * facing
}

/// The lowest a strand group may root at this azimuth, in head radii.
fn hairline(scalp: &Scalp, azimuth: f32, coverage: f32) -> f32 {
    let brow = 0.36 - 0.20 * coverage.clamp(-1.0, 1.0);
    let nape = scalp.bottom() + 0.05;
    nape + (brow - nape) * frontness(azimuth)
}

/// Pushes a root away from the parting.
///
/// A parting is a gap, and a gap is made by moving hair out of it rather than by
/// leaving roots out — leaving them out shows scalp.
fn part_aside(azimuth: f32, part: f32) -> f32 {
    let parting = part.clamp(-1.0, 1.0) * 1.1;
    let mut offset = azimuth - parting;
    while offset > std::f32::consts::PI {
        offset -= TAU;
    }
    while offset < -std::f32::consts::PI {
        offset += TAU;
    }
    let nearness = (-(offset / PART_SPREAD).powi(2)).exp();
    azimuth + PART_PUSH * offset.signum() * nearness
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn scalp(seed: i64) -> (Scalp, Rig, Surface) {
        let mut record = AvatarRecord::new("Haired", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        let surface = Surface::measure(&mesh, &rig);
        (scalp, rig, surface)
    }

    #[test]
    fn a_head_grows_hair() {
        let (scalp, rig, surface) = scalp(1);
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        assert!(hair.groups.len() > 40, "only {} groups", hair.groups.len());
        assert!(hair.mesh().face_count() > 1000);
    }

    #[test]
    fn every_group_hangs_downward() {
        let (scalp, rig, surface) = scalp(7);
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        for group in &hair.groups {
            for step in group.path.windows(2) {
                assert!(
                    step[1].y <= step[0].y + 1e-4,
                    "a strand rose from {:?} to {:?}",
                    step[0],
                    step[1]
                );
            }
        }
    }

    #[test]
    fn roots_sit_outside_the_scalp_but_not_far_outside() {
        // Hair inside the skull disappears; hair far outside it floats. The
        // margin either way is small, which is why the scalp is measured.
        let (scalp, rig, surface) = scalp(23);
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        for group in &hair.groups {
            let root = group.root();
            let height = root.y / scalp.radius();
            let across = Vec3::new(root.x, 0.0, root.z).length() / scalp.radius();
            // Outside-ness has to be judged against the profile, not against the
            // width at the root's own height: standing a root off the crown
            // carries it *above* the crown, where the profile has closed to
            // nothing and any comparison there is meaningless.
            assert!(
                across >= scalp.width_at(height) - 0.02 || height >= scalp.top() - 0.02,
                "a root sank inside the skull at height {height}, {across} across"
            );
            let out = Vec3::new(across, height, 0.0).length();
            assert!(
                out <= scalp.top() + 0.45,
                "a root floated {out} from the head's centre"
            );
        }
    }

    #[test]
    fn nothing_roots_on_the_face() {
        // The forehead is the boundary; roots below it are eyebrows at best.
        let (scalp, rig, surface) = scalp(3);
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        for group in &hair.groups {
            let root = group.root();
            if root.z <= 0.0 {
                continue;
            }
            let azimuth = root.x.atan2(root.z);
            let lowest = hairline(&scalp, azimuth, 0.0);
            assert!(
                root.y / scalp.radius() >= lowest - 0.05,
                "a root sat at {} with a hairline of {lowest}",
                root.y / scalp.radius()
            );
        }
    }

    #[test]
    fn the_front_is_a_fringe_and_the_back_is_not() {
        // Uniform length is a curtain over the face, and this is the test that
        // says so.
        let (scalp, rig, surface) = scalp(11);
        let hair = Hair::over(
            &scalp,
            Some((&rig, &surface)),
            &HairParams {
                length: 1.0,
                ..Default::default()
            },
        );
        // Only what is genuinely over the face counts as fringe. Hair at the
        // sides hangs full length, as it should — the short part is the narrow
        // window in front, not everything forward of the ears.
        let reach = |window: fn(f32) -> bool| {
            hair.groups
                .iter()
                .filter(|group| {
                    let root = group.root();
                    window(root.x.atan2(root.z).abs())
                })
                .map(|group| -group.tip().y)
                .fold(0.0f32, f32::max)
        };
        let fringe = reach(|azimuth| azimuth < 0.6);
        let behind = reach(|azimuth| azimuth > 2.5);
        assert!(
            behind > fringe * 3.0,
            "the back reached {behind} and the fringe {fringe}"
        );
    }

    #[test]
    fn longer_hair_hangs_lower() {
        let (scalp, rig, surface) = scalp(5);
        let short = Hair::over(
            &scalp,
            Some((&rig, &surface)),
            &HairParams {
                length: 0.0,
                ..Default::default()
            },
        );
        let long = Hair::over(
            &scalp,
            Some((&rig, &surface)),
            &HairParams {
                length: 1.0,
                ..Default::default()
            },
        );
        assert!(long.drop() > short.drop() * 2.0);
        assert!(short.drop() > 0.0, "cropped hair still has to exist");
    }

    #[test]
    fn a_lower_hairline_covers_more_forehead() {
        let (scalp, rig, surface) = scalp(9);
        let lowest = |coverage: f32| {
            Hair::over(
                &scalp,
                Some((&rig, &surface)),
                &HairParams {
                    coverage,
                    ..Default::default()
                },
            )
            .groups
            .iter()
            .filter(|group| group.root().z > 0.4 * scalp.radius())
            .map(|group| group.root().y)
            .fold(f32::MAX, f32::min)
        };
        assert!(lowest(1.0) < lowest(-1.0));
    }

    #[test]
    fn the_parting_moves_with_its_parameter() {
        // Roots are pushed out of the parting, so the widest gap in azimuth is
        // where the parting is.
        for (part, expected) in [(-1.0f32, -1.1f32), (0.0, 0.0), (1.0, 1.1)] {
            let moved = part_aside(expected + 0.001, part) - part_aside(expected - 0.001, part);
            // Roots either side are pushed apart by a full push each way.
            assert!(
                (moved - 2.0 * PART_PUSH).abs() < 0.01,
                "a parting at {part} opened by {moved}, not {}",
                2.0 * PART_PUSH
            );
        }
    }

    #[test]
    fn hair_is_reproducible() {
        let (scalp, rig, surface) = scalp(13);
        let params = HairParams::default();
        assert_eq!(
            Hair::over(&scalp, Some((&rig, &surface)), &params),
            Hair::over(&scalp, Some((&rig, &surface)), &params)
        );
    }

    #[test]
    fn every_group_sweeps_a_closed_solid() {
        let (scalp, rig, surface) = scalp(17);
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        for group in &hair.groups {
            let mesh = group.mesh();
            assert!(
                mesh.is_closed_manifold(),
                "a group swept {:?}",
                mesh.manifold_report()
            );
        }
    }

    #[test]
    fn the_extremes_of_every_axis_still_grow_hair() {
        let (scalp, rig, surface) = scalp(19);
        for length in [0.0, 1.0] {
            for volume in [-1.0, 1.0] {
                for coverage in [-1.0, 1.0] {
                    for part in [-1.0, 0.0, 1.0] {
                        for wave in [0.0, 1.0] {
                            let params = HairParams {
                                length,
                                volume,
                                coverage,
                                part,
                                wave,
                                ..Default::default()
                            };
                            let hair = Hair::over(&scalp, Some((&rig, &surface)), &params);
                            assert!(
                                hair.groups.len() > 20,
                                "{params:?} grew {} groups",
                                hair.groups.len()
                            );
                            assert!(hair.mesh().face_count() > 500, "{params:?} swept nothing");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = HairParams {
            length: 9.0,
            volume: -8.0,
            coverage: f32::NAN,
            part: 3.0,
            wave: -1.0,
            shade: f32::INFINITY,
            groups: 4,
        };
        params.sanitize();
        assert_eq!(params.length, 1.0);
        assert_eq!(params.volume, -1.0);
        assert_eq!(params.coverage, 0.0);
        assert_eq!(params.part, 1.0);
        assert_eq!(params.wave, 0.0);
        assert_eq!(params.shade, 0.3);
        assert_eq!(params.groups, MIN_GROUPS);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn a_default_head_of_hair_survives_a_round_trip_through_json() {
        let params = HairParams::default();
        let text = serde_json::to_string(&params).expect("serialises");
        let back: HairParams = serde_json::from_str(&text).expect("deserialises");
        assert_eq!(params, back);
    }

    #[test]
    fn the_colour_ramp_runs_dark_to_pale() {
        let dark = HairParams {
            shade: 0.0,
            ..Default::default()
        }
        .colour();
        let pale = HairParams {
            shade: 1.0,
            ..Default::default()
        }
        .colour();
        assert!(dark.iter().all(|channel| *channel < 0.05));
        assert!(pale[0] > 0.7);
        for shade in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let colour = HairParams {
                shade,
                ..Default::default()
            }
            .colour();
            assert!(colour.iter().all(|channel| (0.0..=1.0).contains(channel)));
            // Hair is warm at every point on the ramp; a grey ramp reads as ash.
            assert!(colour[0] >= colour[2], "shade {shade} came out cold");
        }
    }
}
