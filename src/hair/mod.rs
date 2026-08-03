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
//! Hair is also the most expensive thing on a body by some way — it was 70% of
//! the whole triangle budget — so two things here are about cost. Each lock is
//! **sampled by how far it actually travels** rather than by a fixed count,
//! which follows from the point above: if a fringe is a tenth the length of the
//! hair behind it, it does not need the same number of points. And the group
//! count a record asks for is a **request**, tiered down when the rest of the
//! axes are expensive enough that granting it would not fit; see
//! [`MAX_TRIANGLES`]. Neither touches the cross-section or the group count of a
//! default head, because both of those are what the two decisions above
//! protect: cheapening the ribbon turns locks into rope or into one shell, and
//! growing fewer of them shows scalp.
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

/// How many points sample the cap of the row rooted at the crown.
///
/// That row is sampled densely because the scalp's curvature is worst exactly
/// where its cap begins. At five steps the chord from the crown to the next
/// sample cut *under* the skull, and the close-up showed a bald disc at the
/// whorl — invisible at body framing.
const CROWN_STEPS: usize = 10;

/// How much scalp one cap sample covers, in head radii.
///
/// Every lock used to get the crown row's sample count whatever it had to hug,
/// and a lock's cap is not a fixed length: at the front it runs from the crown
/// to the brow, half a head radius, and at the back it runs on to the nape,
/// nearly three times as far. Sampling by how much scalp a lock actually
/// crosses spends the same total on a much better distribution.
///
/// Measured on built hair rather than on a reconstruction of it, because a
/// synthetic meridian at one azimuth said the opposite and was wrong twice
/// over — it made a coarser cap look *better* than the one it replaced, and it
/// never saw the back of the head, which is where the deepest chord is. The
/// figure the value is set by is how far a centre-line falls inside the
/// measured profile, worst case over five bodies and all three volumes: a lock
/// is `0.024` head radii thick, and no chord sags as much as half of that, so
/// every lock's outer surface still stands proud of the skull.
const CAP_PER_STEP: f32 = 0.15;

/// The most points that sample a lock's fall.
const FALL_STEPS: usize = 9;

/// The fewest points either phase of a lock is sampled with.
const MIN_STEPS: usize = 2;

/// How much fall one sample of it covers, in head radii.
///
/// Sampling a fringe at a back lock's resolution was most of what hair cost.
/// At the default length the locks over the face fall a tenth as far as the
/// ones behind them and were given the same nine points; a lock's samples
/// should follow how far it actually travels, which is the same principle that
/// sizes the cap by how much scalp its row crosses. Waving counts toward the
/// distance, because a wave is path the lock goes round.
const FALL_PER_STEP: f32 = 0.21;

/// How far a lock's tip is drawn back toward the middle of its clump.
///
/// Locks that fall exactly as they were rooted stay parallel all the way down
/// and read as a comb. Real hair gathers: the cards of a clump spread at the
/// scalp and converge at the tips, and that convergence is most of what makes a
/// head of hair read as hair rather than as fringing.
const CLUMP: f32 = 0.34;

/// How much lock lengths vary either side of their nominal.
///
/// Without it every tip lands on the same line and the hair ends in a clean
/// edge, which nothing on a head does.
const RAGGED: f32 = 0.16;

/// How far a falling strand may move outward per unit it drops.
///
/// Draping hair leans out; it does not jut. Taking a clearance correction whole
/// at one step turned strands into horizontal shelves at the shoulders.
const MAX_LEAN: f32 = 0.55;

/// How shape parameters describe a head of hair.
///
/// Axes run `-1` to `+1` where a direction is meant and `0` to `1` where an
/// amount is, matching the body plan's convention.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HairParams {
    /// How far the hair falls, `0` cropped and `1` past the shoulder blades.
    #[serde(with = "crate::plan::scaled")]
    pub length: f32,
    /// How far the hair stands off the scalp, `-1` flat and `+1` full.
    #[serde(with = "crate::plan::scaled")]
    pub volume: f32,
    /// Where the hairline sits, `-1` receding and `+1` low on the brow.
    #[serde(with = "crate::plan::scaled")]
    pub coverage: f32,
    /// Where the parting runs, `-1` over the left ear and `+1` over the right.
    #[serde(with = "crate::plan::scaled")]
    pub part: f32,
    /// How much the hair waves as it falls, `0` straight and `1` curling.
    #[serde(with = "crate::plan::scaled")]
    pub wave: f32,
    /// Colour along a melanin ramp, `0` black and `1` pale blonde.
    #[serde(with = "crate::plan::scaled")]
    pub shade: f32,
    /// How many strand groups to grow.
    #[serde(
        deserialize_with = "crate::plan::scaled::deserialize_count",
        serialize_with = "crate::plan::scaled::serialize_count"
    )]
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
            groups: 128,
        }
    }
}

/// The fewest and most strand groups a record may ask for.
///
/// A floor because a handful of ribbons is a wig, not hair; a ceiling because
/// group count is the one axis that costs geometry, and a record read off the
/// network is not to be trusted with that.
///
/// The ceiling used to be 256, which admitted more triangles of hair than the
/// whole avatar is allowed to cost — so it was not doing the job its own
/// comment claimed. It is now the count the default asks for, because that is
/// the most the budget can pay for even at the cheap end of the look axes.
/// Asking is still not getting: see [`MAX_TRIANGLES`].
pub const MIN_GROUPS: u32 = 24;
/// See [`MIN_GROUPS`].
pub const MAX_GROUPS: u32 = 128;

/// The most triangles a head of hair may cost, whatever a record asks for.
///
/// The whole avatar is judged against 30,000 on a WebGL2 tier, and this is
/// whatever is left once everything that is not hair has been paid for. It is
/// DERIVED, so it moves when they do: giving the face enough surface to carry
/// features (#59) took everything that is not hair from about 13,100 triangles
/// to about 13,400, and the worst legal head of hair now brings a body to
/// 29,900 against the 30,000 it is allowed. That is most of the slack, and the
/// next thing to want triangles has to take them from here.
///
/// This is the number that makes group count a *request*. A record may ask for
/// [`MAX_GROUPS`]; whether it gets them depends on what the rest of its axes
/// cost, because a lock's price is no longer fixed — it follows how far the
/// lock actually travels. Longest and waviest, a lock costs nearly twice what
/// the default's does.
pub const MAX_TRIANGLES: usize = 16_500;

/// The fewest groups the budget will tier a request down to.
///
/// Tiering is the renderer overruling the creator, and it has a floor because
/// the thing it protects is a triangle count, which knows nothing about
/// whether the result still reads as hair. At 96 groups the locks are visibly
/// broader than the default's and the crown starts to read as one shell; at 64
/// they are slabs with daylight between them. So: 96, and never lower.
///
/// It costs nothing to hold that line. Measured across the whole parameter
/// space, no legal record exceeds [`MAX_TRIANGLES`] at 96 groups.
const TIER_FLOOR: u32 = 96;

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
    ///
    /// The group count in `params` is a **request**. If the hair it grows costs
    /// more than [`MAX_TRIANGLES`], fewer groups are grown until it fits or the
    /// tier reaches its floor — which is what stops a record read off the
    /// network from spending the whole avatar's budget on its hair. Every other
    /// axis is the creator's and is never touched: length, wave and volume are
    /// what the hair *looks like*, and quietly shortening someone's hair to
    /// save triangles is a different thing from drawing fewer locks of it.
    ///
    /// Cost is close to linear in the group count, so this settles in one
    /// regrow in practice; it is bounded by the strictly decreasing count.
    #[must_use]
    pub fn over(scalp: &Scalp, body: Option<(&Rig, &Surface)>, params: &HairParams) -> Self {
        let mut asked = params.groups;
        let mut grown = Self::grow(scalp, body, params, asked);
        while grown.tris() > MAX_TRIANGLES && asked > TIER_FLOOR {
            let afford = asked as usize * MAX_TRIANGLES / grown.tris();
            asked = (afford as u32).clamp(TIER_FLOOR, asked - 1);
            grown = Self::grow(scalp, body, params, asked);
        }
        grown
    }

    /// What a head of hair will cost to sweep, in triangles.
    #[must_use]
    pub fn tris(&self) -> usize {
        self.groups.iter().map(Strand::tris).sum()
    }

    /// Grows exactly `groups` strand groups, without regard to the budget.
    #[must_use]
    fn grow(
        scalp: &Scalp,
        body: Option<(&Rig, &Surface)>,
        params: &HairParams,
        groups: u32,
    ) -> Self {
        let radius = scalp.radius();
        let columns = (groups as usize / ROWS).max(MIN_COLUMNS);
        let step = TAU / columns as f32;

        let thickness = radius * 0.024;

        // Rooted AT the pole, so every column's top row converges on one point
        // and radiates outward — which is the whorl real hair has.
        //
        // Rooting a hair's breadth below it does not work: the profile closes
        // very fast near the crown, so a mere 0.04 of a head radius down still
        // leaves a circle 0.038 m across with nothing inside it. That is half
        // the skull's own width, and the close-up showed it as a clean bald
        // disc — entirely invisible at body framing.
        let crown = scalp.top();
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
        let width = outermost * step * 0.80;

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
                // Where this lock's clump gathers: the column's own line, which
                // the rows are staggered either side of.
                let gathers = part_aside(step * (column as f32 + 0.5), params.part);
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
                let ragged = 1.0 + RAGGED * (jitter(column, row) * 2.0 - 1.0);
                let fall = (reach + (fringe - reach) * front) * (1.0 - 0.15 * row as f32) * ragged;

                let path = sweep(
                    scalp,
                    body,
                    Fall {
                        azimuth,
                        gathers,
                        root,
                        hairline,
                        stand: lift,
                        fall,
                        waving,
                        // The crown row keeps its floor whatever it crosses: it
                        // is the only one that goes over the pole, where the
                        // profile turns through a right angle inside a couple
                        // of bands, and a chord across that is the bald disc at
                        // the whorl.
                        cap_steps: (((root - hairline) / CAP_PER_STEP).round() as usize)
                            .max(if row == 0 { CROWN_STEPS } else { MIN_STEPS }),
                        fall_steps: (((fall + 3.0 * waving) / (FALL_PER_STEP * radius)).round()
                            as usize)
                            .clamp(MIN_STEPS, FALL_STEPS),
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
    /// The line its clump gathers toward as it falls.
    gathers: f32,
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
    /// How many points sample its cap.
    cap_steps: usize,
    /// How many points sample its fall.
    fall_steps: usize,
}

/// The centre-line of one strand group.
///
/// Two phases: down the scalp to the hairline, then falling free. The first is
/// what makes hair look attached, and skipping it — rooting every group at the
/// crown and letting it fall — is what makes procedural hair look like a mop
/// dropped on a head.
fn sweep(scalp: &Scalp, body: Option<(&Rig, &Surface)>, fall: Fall) -> Vec<Vec3> {
    let Fall { azimuth, .. } = fall;
    let mut path = Vec::with_capacity(fall.cap_steps + fall.fall_steps + 1);

    for step in 0..=fall.cap_steps {
        let along = step as f32 / fall.cap_steps as f32;
        let height = fall.root + (fall.hairline - fall.root) * along;
        path.push(scalp.point(azimuth, height) + scalp.normal(azimuth, height) * fall.stand);
    }

    let from = *path.last().expect("the cap always has points");
    let radial = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
    // Where the tip would hang if the lock belonged to its clump's centre line
    // rather than to its own spoke.
    let gathered = Vec3::new(fall.gathers.sin(), 0.0, fall.gathers.cos());
    let toward = (gathered - radial) * Vec3::new(from.x, 0.0, from.z).length();
    // How far the strand has already leaned out to clear the body. It may grow
    // with the drop but never jump, which is what keeps a drape a drape.
    let mut leaned = 0.0f32;
    let mut previous = from;
    for step in 1..=fall.fall_steps {
        let along = step as f32 / fall.fall_steps as f32;
        // Widening as it falls, so long hair drapes over the shoulders instead
        // of hanging inside them.
        let flare = 1.0 + 0.22 * along;
        let wave = (along * TAU * 1.6 + fall.phase).sin() * fall.waving * along;
        // Gathering grows with the fall, so the lock leaves the scalp on its own
        // spoke and arrives at the clump's.
        let gather = toward * (CLUMP * along * along);
        let point = Vec3::new(
            from.x * flare + radial.x * wave + gather.x,
            from.y - fall.fall * along,
            from.z * flare + radial.z * wave + gather.z,
        );

        let placed = match body {
            Some((rig, surface)) => {
                let push = surface.clearance(rig, point + scalp.origin(), fall.clearance);
                let drop = (previous.y - point.y).max(0.0);
                leaned = push.length().min(leaned + drop * MAX_LEAN);
                point + push.normalize_or_zero() * leaned
            }
            None => point,
        };
        previous = placed;
        path.push(placed);
    }

    path.dedup_by(|a, b| a.distance_squared(*b) < 1e-10);
    path
}

/// A settled pseudo-random value in `0..1` for one lock.
///
/// Hashed rather than drawn from a generator, so a head of hair is the same
/// every time it is grown without anything having to carry a seed around.
fn jitter(column: usize, row: usize) -> f32 {
    let mixed = (column as u32)
        .wrapping_mul(73_856_093)
        .wrapping_add((row as u32).wrapping_mul(19_349_663));
    f32::from(((mixed >> 7) % 1024) as u16) / 1024.0
}

/// How much a direction faces the front, from `0` at the back to `1` at the face.
///
/// Deliberately a gentle falloff. Squaring it made the window too narrow: the
/// strands a third of the way round still counted as side hair, fell full
/// length, and passed straight down the eye line — the close-up showed a face
/// behind a curtain. A forehead is broad, so the fringe window is broad.
fn frontness(azimuth: f32) -> f32 {
    0.5 + 0.5 * azimuth.cos()
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
    fn no_lock_sinks_into_the_skull() {
        // What the sampling rules are actually for. A centre-line runs between
        // its samples as a chord, and a chord across a curve falls inside it;
        // if it falls further in than the ribbon is thick, the lock's outer
        // surface is inside the skull and there is a bald patch where it should
        // have been.
        //
        // Measured, not assumed, and the measurement had to be built twice: a
        // synthetic meridian at one azimuth reported the opposite ordering of
        // two sampling rules, because it never looked at the back of the head —
        // where the cap runs nearly three times as far as it does at the brow,
        // and where the deepest chord always is.
        //
        // The worst case over these five bodies is 0.80 of a half-thickness,
        // and it is on the row rooted lowest: that row is the underlayer, with
        // the other three stacked outside it, so it is also the row nothing can
        // see. The row that is on top is the one rooted at the crown, and it
        // keeps a sample count of its own for exactly that reason.
        for seed in [0, 1, 7, 23, 42] {
            let (scalp, rig, surface) = scalp(seed);
            let thickness = scalp.radius() * 0.024;
            for volume in [-1.0f32, 0.0, 1.0] {
                let params = HairParams {
                    volume,
                    ..Default::default()
                };
                let hair = Hair::over(&scalp, Some((&rig, &surface)), &params);
                for group in &hair.groups {
                    for step in group.path.windows(2) {
                        for sub in 0..=8 {
                            let point = step[0].lerp(step[1], sub as f32 / 8.0);
                            let height = point.y / scalp.radius();
                            if height < scalp.bottom() || height > scalp.top() {
                                continue;
                            }
                            let skull = scalp.width_at(height) * scalp.radius();
                            let across = Vec3::new(point.x, 0.0, point.z).length();
                            assert!(
                                skull - across < thickness,
                                "seed {seed} at volume {volume}: a lock sank {} of a \
                                 half-thickness into the skull at height {height}",
                                (skull - across) / thickness
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn an_expensive_head_of_hair_is_tiered_down_rather_than_grown() {
        // The half of this that a record can attack. Group count is what costs
        // geometry, so it is what gets taken away — and only it: the axes that
        // say what the hair looks like are the creator's.
        let (scalp, rig, surface) = scalp(1);
        let dear = HairParams {
            length: 1.0,
            wave: 1.0,
            groups: MAX_GROUPS,
            ..Default::default()
        };
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &dear);
        assert!(
            hair.tris() <= MAX_TRIANGLES,
            "the dearest legal hair cost {}",
            hair.tris()
        );
        assert!(
            hair.groups.len() < MAX_GROUPS as usize,
            "it asked for {MAX_GROUPS} groups at the dearest look and got them all"
        );
        assert!(
            hair.groups.len() >= TIER_FLOOR as usize - ROWS,
            "tiering went past its floor, to {} groups",
            hair.groups.len()
        );
        // The look survives the tier: it is still long hair, not cropped.
        let cheap = Hair::over(&scalp, Some((&rig, &surface)), &HairParams::default());
        assert!(
            hair.drop() > cheap.drop() * 1.5,
            "the tier shortened the hair, from {} to {}",
            cheap.drop(),
            hair.drop()
        );
    }

    #[test]
    fn the_default_head_of_hair_is_not_tiered() {
        // A budget that bites on the default is a budget set too low: the
        // number a creator sees when they change nothing has to be the number
        // they asked for.
        let (scalp, rig, surface) = scalp(1);
        let params = HairParams::default();
        let hair = Hair::over(&scalp, Some((&rig, &surface)), &params);
        let ungoverned = Hair::grow(&scalp, Some((&rig, &surface)), &params, params.groups);
        assert_eq!(
            hair.groups.len(),
            ungoverned.groups.len(),
            "the default asked for {} groups and was tiered",
            params.groups
        );
    }

    #[test]
    fn no_record_can_spend_more_than_the_budget_on_hair() {
        // The whole point of the ceiling. Swept rather than argued, because the
        // cost of a lock is no longer fixed — it follows how far the lock
        // travels — so which corner of the space is dearest is not obvious and
        // was not the one guessed at.
        let (scalp, rig, surface) = scalp(1);
        for length in [0.0, 0.5, 1.0] {
            for volume in [-1.0, 0.0, 1.0] {
                for coverage in [-1.0, 0.0, 1.0] {
                    for wave in [0.0, 0.5, 1.0] {
                        // Above the ceiling too: sanitize clamps a record, but
                        // nothing makes a caller sanitize one.
                        for groups in [MIN_GROUPS, 96, MAX_GROUPS, 1024] {
                            let mut params = HairParams {
                                length,
                                volume,
                                coverage,
                                wave,
                                groups,
                                ..Default::default()
                            };
                            let hair = Hair::over(&scalp, Some((&rig, &surface)), &params);
                            assert!(
                                hair.tris() <= MAX_TRIANGLES,
                                "{params:?} cost {} triangles",
                                hair.tris()
                            );
                            params.sanitize();
                            assert!(params.groups <= MAX_GROUPS);
                        }
                    }
                }
            }
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
