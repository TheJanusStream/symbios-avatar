//! A beast's mane: one closed solid from the withers to the forelock (#407).
//!
//! **Not cards.** A mane of bands of cards hung as a flat curtain from behind, read
//! as a fin side on, and showed slits between the bands; and its forelock was a
//! second zone of cards between the ears, apart from it. A real mane runs on from
//! the back, over the neck and the poll between the ears, to the forehead. So the
//! mane zone and the forelock zone are drawn together as one solid, as a person's
//! helmet-family hair is ([`crate::hair::shell`]), in the hair's own draw, cut from
//! the same strand mask.
//!
//! **One solid, rooted on one line.** The line runs up the crest the rig draws
//! ([`Crest`], #389) to the poll, and on from there over the head's declared surface
//! ([`BeastHead::drawn_distance`], the head with the neck's upper run joined on) to
//! the whorl between the ears. Columns stand along it in locks, a lock's edge
//! columns shared with its neighbours, so the two surfaces run unbroken from end to
//! end and close on themselves at the root, at the hem and at either end:
//!
//! - **Along the neck** a column lies on the neck's off side (the beast's right), on
//!   the skin, and as the mane stiffens it hands over to the curve a stiff mane's
//!   card stands on ([`Lock`]).
//! - **Over the poll** the columns fan: each one's heading turns from the neck's,
//!   to the off side and back, round through forward to a little toward the near
//!   side, while its root climbs to the whorl. The first of them still follow the
//!   neck's curve, handing over to the head's within [`HANDOVER`] of the fan, and
//!   the hair is shortest where it turns past the off ear.
//! - **The forelock** is the end of that fan: limp, its columns are walked over the
//!   declared head toward the nose and down it, lying on it, stopping at the eyes;
//!   stiff, they stand off it as a short upright brush.
//!
//! **The inner surface lies on the skin**: where a column runs close over it, the
//! inner surface is laid on the skin and the lower part of a lock comes down onto it,
//! so no hem stands off a shadow; over its frayed end a lying lock goes flat on the
//! skin and its hem tucks a hair under it, so its strands' tips cast no dark line
//! apart from themselves. The skin is never a ray cast at the mesh as the mane is
//! grown: it is the crest's section, measured once along its length
//! ([`Crested::skin_at`]); the neck's declared upper run where the build has shaped
//! the neck onto it ([`BeastHead::nape_skin`]); and the declared head over the poll.
//! **Locks are irregular at the hem and one strip above it**: each its own width,
//! length and point, off its middle, and a shallow notch between two; but every
//! column is as thick as the others at the same distance down ([`SWELL`]), so a
//! lock's point cuts the hem without shaping the strip above it into a lump, and a
//! lock's ridge is gentle, thicker in its middle only toward the hem. A standing
//! mane evens its points into one brush, roached ([`BRUSH`]). A few short cards fray
//! the hem of a mane that stands clear of the skin.
//!
//! **Shaded as one strip, streaked along the hair**: by its own faces a lock is a
//! broad smooth stretch, and the hair's highlight lit a whole one at once, a grey
//! sheet on a black mane; turned round across each lock, the locks read as a braid
//! of bundles. So the solid's normals are read off its whole grid, alike on either
//! side of a column two locks share, and each column's is turned about the way its
//! hair runs, every other one the other way ([`STREAK`]): the highlight breaks into
//! streaks down the strip, a span apart. The fur carries them, and the draw keeps
//! the normals of fur with a mane (`super::grow`).
//!
//! **Bound as the skin under it**: every point of the solid takes the binding of the
//! skin it lies over, blended from the body's nearest vertices of the neck, the
//! chest and the head, so a nod or a turn bends it at the poll as it bends the skin,
//! and two locks, or its two surfaces, that meet at rest meet in every pose.
//!
//! **Kept out of what the head wears and its ears**: every vertex is pushed out of a
//! capsule round each ear and each piece the head wears, a little wider than it.
//!
//! **A fixed cost.** The grid is the mane's and not a record's: the solid costs the
//! same at every length and stiffness ([`NEAR`]), so the fur's ceiling cannot thin it
//! and thins the zones of cards round it instead (`super::grow`). A beast far off
//! draws the same solid on a coarser grid with no fringe ([`FAR`]), beside the near
//! tier's cards of every other zone.
//!
//! **One colour** (`CoatSampler::mane`): a picked colour exactly, or the coat's own
//! long hair whole, a shade darker where it is the coat's own colour. Faded a shade
//! lighter toward the hem as a card is, and its underside a shade darker.
//!
//! **The same bits on every target**: every sine, power, arc tangent and turn here is
//! [`crate::det`]'s, not the platform's. Several decide an outcome - which way the
//! sheet faces, how a lock is wound, where a walk lets go of the head or stops, where
//! the poll's line ends - and a rounding apart there would build another mane in a
//! browser's worker than on a desktop from the same record.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Vec2, Vec3};
use rand::Rng;
use rand_pcg::Pcg64Mcg;

use super::zone::Lock;
use super::{Bed, Crest, Crested, PARTING};
use crate::det::{DetMath, Rot, from_rotation_arc, sin_cos};
use crate::face::step;
use crate::hair::clump::loft::loft;
use crate::hair::clump::{Root, Shape};
use crate::hair::mask::{self, StrandMask};
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::Fur;
use crate::plan::Zone;
use crate::plan::beast_head::{BeastHead, march};
use crate::rig::Influence;
use crate::texture::coat::CoatSampler;

/// The grid a mane is lofted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Grid {
    /// How many column spans a lock is lofted across: one more column than this,
    /// the two at its edges shared with its neighbours.
    pub per_lock: usize,
    /// How many rows each column is lofted with, root to hem.
    pub rows: usize,
    /// Whether the hem grows its fringe of short cards.
    pub fringe: bool,
}

impl Grid {
    /// What the solid costs on this grid, in triangles, its fringe left out: two
    /// surfaces of quads.
    #[cfg(test)]
    pub(super) fn triangles(self) -> usize {
        LOCKS * self.per_lock * (self.rows - 1) * 4
    }
}

/// The grid a mane is drawn on near.
///
/// **About what the cards it replaces cost** (#407): 2,004 to 2,488 triangles of
/// mane and forelock at the default stiffness over the round's horses and crosses.
/// Fourteen locks of four spans over two surfaces is 112 triangles a row; ten rows
/// is 2,016, and the fringe a standing mane grows adds 220 to 250 more. Four spans
/// rather than three because a lock's point and its ridge are what a hem of locks
/// is drawn with, and three spans put the point on a column; ten rows rather than
/// twelve because the curve down the neck reads the same at either.
///
/// Provenance: **derived** from the cards' cost, **judged by render** at both ends
/// of the slider.
pub(super) const NEAR: Grid = Grid {
    per_lock: 4,
    rows: 10,
    fringe: true,
};

/// The grid a mane is drawn on far off (#407), in the far tier's hair: two spans a
/// lock, six rows, and no fringe. 560 triangles, a little over a quarter of the
/// near one, as a scalp's far shell is of its near one.
///
/// Provenance: **judged by render** at twelve metres, where a lock is a few
/// pixels wide.
pub(super) const FAR: Grid = Grid {
    per_lock: 2,
    rows: 6,
    fringe: false,
};

/// How many locks along the neck, and over the poll and the forelock.
const NECK_LOCKS: usize = 9;
const HEAD_LOCKS: usize = 5;
const LOCKS: usize = NECK_LOCKS + HEAD_LOCKS;

/// Where along the crest the first column stands.
const FIRST: f32 = 0.03;

/// Over what stiffness a mane's hem goes from its locks' points to one brush, and
/// how much of their unevenness it loses there, toward what length: at full
/// stiffness the points stood as a row of separate spikes (cross 73), where a stiff
/// mane is one brush, roached.
/// Provenance: **tuned by render** (#407), from the default stiffness up, so a
/// lying mane keeps every point.
const BRUSH: [f32; 2] = [0.45, 1.0];
const BRUSH_EVENS: f32 = 0.8;
const EVEN: f32 = 0.85;

/// How wide a lock is against the others, drawn per lock.
const WIDTHS: [f32; 2] = [0.6, 1.45];

/// How long a lock's point runs, as a share of its column's whole curve.
const LENGTHS: [f32; 2] = [0.68, 1.15];

/// Where across a lock its point is.
const POINT_AT: [f32; 2] = [0.25, 0.75];

/// How far short of the shorter neighbour a lock's edge stops: 0.8 first, which cut
/// the strip into tongues, and from above, a lock apiece, into a braid.
/// Provenance: **tuned by render** (#407).
const NOTCH: f32 = 0.92;

/// The same over the head, where a fan opens its locks apart as they go.
const HEAD_NOTCH: f32 = 0.97;

/// How thick the volume is at its thickest, as a share of the back's height, at a
/// mane's full length, and how much of that the forelock has.
const THICK: f32 = 0.03;
const FORELOCK_THICK: f32 = 1.0;

/// Where along a column its thickness peaks, and the power it falls to the hem by,
/// as shares of a mane's full length rather than of the column's own: a short
/// column beside a long one is as thick as it is at the same distance down,
/// rather than thinner by the whole of its shortfall, so a lock's point and the
/// notch beside it cut the hem and do not shape the strip above it into a lump.
/// And over how much of a mane's full length up from its own hem a column closes.
const SWELL: [f32; 2] = [0.06, 0.28];
const FALL_POW: f32 = 0.85;
const CLOSES: f32 = 0.3;

/// What share of a lock's middle thickness its edges keep at the hem, from how far
/// down.
const GROOVE: f32 = 0.78;
const GROOVE_FROM: f32 = 0.5;

/// How much of the thickness stands out of the sheet, limp and stiff.
const OUT_LIMP: f32 = 0.8;
const OUT_STIFF: f32 = 0.5;

/// How long the forelock is against the mane: the forelock card's reach over the
/// mane card's.
const FORELOCK: f32 = 0.85;

/// How short the hair gets where the fan turns past the off ear, against the mane.
const POLL_DIP: f32 = 0.4;

/// The heading of the fan's first column, its turn, and its last, in radians from
/// forward toward the off side: the neck's, to the off side and a little back,
/// round to a little toward the near side.
const HEADING: [f32; 3] = [1.95, 0.35, -0.35];

/// How far across the poll a forelock's roots spread, as a share of the room between
/// the ears' bases.
const FORELOCK_SPREAD: f32 = 0.9;

/// How short a standing crest is over the poll against the mane, and how much of
/// the mane's length it loses up the last of the crest to get there.
const STIFF_DIP: f32 = 0.2;
const POLL_TAPER: f32 = 0.8;

/// From how far up the crest a standing mane starts to fall away toward the poll.
const TAPER_FROM: f32 = 0.3;

/// How far through the fan its heading has come round to the forelock's first.
const TURN: f32 = 0.45;

/// Over what stiffness a forelock goes from lying on the forehead to standing as a
/// brush, and how much shorter that brush is than the forelock lying.
const FORELOCK_STANDS: [f32; 2] = [0.4, 1.0];
const FORELOCK_BRUSH: f32 = 0.4;

/// How much of the fan has its root climbed to the whorl by.
const ROOTS_REACH: f32 = 0.55;

/// Over how much of the fan the neck's curve hands over to the head's.
const HANDOVER: f32 = 0.3;

/// How far a forelock lies off the head, as a share of its length, at its root and
/// its tip.
const FORELOCK_LIE: [f32; 2] = [0.02, 0.06];

/// How far under the skin a column starts, as a share of its thickness.
const SINK: f32 = 0.6;

/// How many steps a walk over the skin takes, root to tip, whatever the grid: the
/// far tier reads the near tier's curves.
const WALK: usize = 33;

/// How far below level a head's surface may face before a walk lets go of it and
/// hangs.
const LETS_GO: f32 = -0.15;

/// How far a forelock's tip reaches past the eyes, in eye radii.
const PAST_EYES: f32 = 2.0;

/// How far over its gap above the skin a column is laid on the skin: whole under
/// the first, none past the second, in its thicknesses.
const ON_SKIN: [f32; 2] = [1.6, 3.2];

/// How far off the skin the inner surface lies, as a share of the back's height.
const OFF_SKIN: f32 = 0.0015;

/// Over what stiffness a neck column goes from lying down the neck to standing off
/// the crest on a mane card's curve.
const STANDS: [f32; 2] = [0.2, 0.9];

/// How far off the neck its column lies, as a share of its length, at its root and
/// its tip.
const NECK_LIE: [f32; 2] = [0.03, 0.05];

/// How much of the drawn neck has to be the declared run's before a column lies on
/// the run rather than on the section the crest read off the body: below it the
/// build blends the cage's neck into the run, which only the section knows.
const NAPE_FROM: f32 = 0.8;

/// How far off the neck's run toward its side a lying column combs, in radians:
/// further than a card's, which lies under the one ahead of it.
pub(super) const SWEEP: f32 = 1.2;

/// Over how much of a column, up to where its strands fray, a lock lying on the skin
/// goes flat on it.
const FLATTENS: f32 = 0.2;

/// Over what share of a column, down to its hem, a lock lying on the skin goes from
/// lying [`OFF_SKIN`] off it to as far under it.
const TUCK: [f32; 2] = [0.8, 1.0];

/// From how far up its hem a lock lying on the skin comes down onto it, as a share
/// of a mane's full length, as [`SWELL`] is.
const HEM_DOWN: f32 = 0.25;

/// How far past an ear, or a piece the head wears, a vertex is kept, as a share of
/// the back's height.
pub(super) const CLEAR: f32 = 0.006;

/// How much wider than an ear's half width its capsule is at its base.
const EAR_WIDTH: f32 = 0.55;

/// The mask row the whole part ends on, and how far down a column that is.
const FRAY_ROW: f32 = 0.55;
const FRAY_FROM: f32 = 0.72;

/// How far in from each side of its lane of the strand mask a lock is drawn.
const LANE_INSET: f32 = 0.08;

/// How far a column's normal is turned about the way its hair runs, at most, in
/// radians, and the least share of that any column is turned: every other column
/// the other way, so the highlight breaks into streaks a span wide down the strip.
/// Provenance: **tuned by render** (#407), on black manes under the key light. A
/// lock's normals fanned 0.8 from its middle to either edge first, which shaded
/// each lock as a round bundle and the mane as a braid of them.
const STREAK: f32 = 0.4;
const STREAK_LEAST: f32 = 0.4;

/// How much darker the inner surface is at its root, and over what share of a
/// column it comes up to the outer's colour.
pub(super) const UNDER: f32 = 0.88;
const HEM_LIT: [f32; 2] = [0.35, 0.72];

/// How much lighter than its root a column's hem is, toward white: a card's.
pub(super) const TIPPED: f32 = 0.08;

/// The fringe: cards per lock, how far down they are rooted, how long they are as a
/// share of the column, and how much of that they lose at full stiffness.
const FRINGE_PER_LOCK: usize = 3;
const FRINGE_ROOT: f32 = 0.6;
const FRINGE_REACH: f32 = 0.5;
const FRINGE_STIFF: f32 = 0.75;

/// How far a mane has to stand off the skin (`STANDS`) before its hem grows a fringe.
const FRINGE_FROM: f32 = 0.3;

/// Where a column grows: on the neck, or over the head a share of the fan.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Place {
    Neck,
    Head(f32),
}

/// One column's root and how it is cut.
struct Column {
    root: Root,
    place: Place,
    /// How much of its curve it runs.
    reach: f32,
    /// The same, before a standing crest tapers it toward the poll: what its
    /// thickness is measured on.
    span: f32,
    /// Where across its lock it stands, `0..=1`; an edge shared by two locks is `0`.
    across: f32,
    /// How thick, before the swell and the groove.
    thick: f32,
    /// What it hangs off: the bone under the crest, or the braincase's centre.
    core: Vec3,
    /// Along the crest toward the withers, at its root.
    run: Vec3,
}

/// A mane, grown: what it cost and where each column of it went, for its tests.
pub(super) struct Grown {
    /// How many locks it drew.
    pub locks: usize,
    /// What it cost, in triangles, its fringe included.
    pub tris: usize,
    /// How many of its mesh's first vertices are shaded by normals of their own: up
    /// to the end of the solid's two surfaces, before its fringe.
    pub shaded: usize,
}

/// The head, as far as the mane asks of it: its declared surface in body space.
struct Head<'a> {
    beast: &'a BeastHead,
    centre: Vec3,
}

impl Head<'_> {
    /// Where a ray from the braincase's centre through `at` leaves the drawn
    /// surface, and the surface's outward normal there, in body space.
    fn surface(&self, at: Vec3) -> Option<(Vec3, Vec3)> {
        let local = self.beast.local(at);
        let direction = (local - self.centre).try_normalize()?;
        let hit = march(
            |point| self.beast.drawn_distance(point),
            self.centre,
            direction,
            self.beast.radius,
        )?;
        Some((
            self.beast.body(hit),
            (self.beast.turn * self.beast.drawn_normal(hit)).normalize_or(Vec3::Y),
        ))
    }
}

/// Grows the whole mane along `line` on `grid`, as a mesh of its own: `None` on a
/// body with no declared head to run it over.
pub(super) fn grow(
    bed: &Bed,
    line: &Crest,
    lock: &Lock,
    sampler: &CoatSampler,
    fur: &Fur,
    grid: Grid,
    stream: &mut Pcg64Mcg,
) -> Option<(PolyMesh, Grown)> {
    let beast = bed.anatomy.head.as_ref()?;
    let crest_end = line.line.last()?.at;
    if line.line.len() < 2 {
        return None;
    }
    let head = Head {
        beast,
        centre: beast.cranium.centre,
    };
    let stiffness = fur.mane_stiffness.clamp(0.0, 1.0);
    let height = bed.anatomy.height;
    let normals = bed.body.shading_normals();
    let thick = THICK * height * fur.mane.min(1.0).sqrt();
    let per_lock = grid.per_lock;
    let rows_count = grid.rows;

    // Each lock its own width, length and point.
    let widths: Vec<f32> = (0..LOCKS)
        .map(|_| stream.random_range(WIDTHS[0]..WIDTHS[1]))
        .collect();
    let lengths: Vec<f32> = (0..LOCKS)
        .map(|_| stream.random_range(LENGTHS[0]..LENGTHS[1]))
        .collect();
    let points: Vec<f32> = (0..LOCKS)
        .map(|_| stream.random_range(POINT_AT[0]..POINT_AT[1]))
        .collect();
    let edges = |from: usize, count: usize, span: [f32; 2]| -> Vec<f32> {
        let total: f32 = widths[from..from + count].iter().sum();
        let mut at = span[0];
        let mut edges = vec![at];
        for width in &widths[from..from + count] {
            at += (span[1] - span[0]) * width / total;
            edges.push(at);
        }
        edges
    };
    let neck_edges = edges(0, NECK_LOCKS, [FIRST, 1.0]);
    let brush = BRUSH_EVENS * step(BRUSH[0], BRUSH[1], stiffness);
    let head_edges = edges(NECK_LOCKS, HEAD_LOCKS, [0.0, 1.0]);

    // The head's line: from the crest's end over the poll to the whorl between the
    // ears, read off the declared head.
    let path = head_path(&head, crest_end);
    let end_root = neck_root(bed, line, &normals, 1.0)?;

    // Every column, lock by lock, the edges shared.
    let mut columns: Vec<Column> = Vec::new();
    for index in 0..LOCKS {
        let on_neck = index < NECK_LOCKS;
        let (from, to) = if on_neck {
            (neck_edges[index], neck_edges[index + 1])
        } else {
            let at = index - NECK_LOCKS;
            (head_edges[at], head_edges[at + 1])
        };
        let edge_reach = |other: Option<usize>| {
            other.map_or(lengths[index], |other| lengths[index].min(lengths[other]))
                * if on_neck { NOTCH } else { HEAD_NOTCH }
        };
        let first = usize::from(index > 0);
        for place in first..=per_lock {
            let across = place as f32 / per_lock as f32;
            let share = from + (to - from) * across;
            // Its length: the lock's point off its middle, down to its edges.
            let reach = if place == 0 {
                edge_reach(index.checked_sub(1))
            } else if place == per_lock {
                edge_reach((index + 1 < LOCKS).then_some(index + 1))
            } else {
                let edge = edge_reach(None);
                let point = points[index];
                let off = (across - point).abs() / point.max(1.0 - point);
                edge + (lengths[index] - edge) * (1.0 - off).max(0.0).det_powf(0.8)
            };
            // A standing mane is cut as one brush, its locks' points and notches
            // evened toward their mean, as a roached mane is.
            let reach = reach + (EVEN - reach) * brush;
            let span = reach;
            // A standing crest falls away toward the poll as the neck rises to it, so
            // its outline follows the neck's rather than the head's.
            let reach = if on_neck {
                reach * (1.0 - POLL_TAPER * stiffness * stiffness * step(TAPER_FROM, 1.0, share))
            } else {
                reach
            };
            let mut column = if on_neck {
                neck_column(bed, line, &normals, share, reach, thick)?
            } else {
                head_column(&head, &path, &end_root, share, (reach, thick))?
            };
            column.across = if place == per_lock { 0.0 } else { across };
            column.span = span;
            columns.push(column);
        }
    }
    let count = columns.len();
    // The two ends close the solid.
    columns[0].thick = 0.0;
    columns[count - 1].thick = 0.0;

    // Each column's middle curve, row by row, and the skin under it.
    let rows: Vec<f32> = (0..rows_count)
        .map(|row| (row as f32 / (rows_count - 1) as f32).det_powf(1.0 + 0.35 * (1.0 - stiffness)))
        .collect();
    let at = |column: usize, row: usize| column * rows_count + row;
    let ground = Ground {
        head: &head,
        crest: line,
    };
    let sink = thick * SINK;
    let crest_top = crested(line, 1.0);
    let end = (end_root, crest_top.run, crest_top.up);
    let mut middle: Vec<Vec3> = Vec::with_capacity(count * rows_count);
    for column in &columns {
        let mut curve = curve(&ground, lock, column, &end, &rows, stiffness, sink);
        // Every root starts under the skin, as a limp one's walk does: a stiff
        // card's curve comes out of the skin by its own reckoning, and on a crest
        // evened over a hollow that is not always under it.
        curve[0] = column.root.at - column.root.out * sink;
        middle.extend(curve);
    }
    let cores: Vec<Vec3> = (0..count * rows_count)
        .map(|here| {
            let column = &columns[here / rows_count];
            let on_bone = ground.bone(middle[here]);
            match column.place {
                Place::Neck => on_bone,
                Place::Head(fan) => on_bone.lerp(column.core, step(0.0, HANDOVER, fan)),
            }
        })
        .collect();
    let skin: Vec<Option<Vec3>> = (0..count * rows_count)
        .map(|here| {
            let point = middle[here];
            let neck = match columns[here / rows_count].place {
                Place::Neck => 1.0,
                Place::Head(fan) => 1.0 - step(0.0, HANDOVER, fan),
            };
            let on_body = || ground.body(point).map(|(at, _)| at);
            let on_head = || head.surface(point).map(|(at, _)| at);
            match (neck > 0.999, neck < 0.001) {
                (true, _) => on_body(),
                (_, true) => on_head(),
                _ => match (on_body(), on_head()) {
                    (Some(body), Some(head)) => Some(head.lerp(body, neck)),
                    (body, head) => body.or(head),
                },
            }
        })
        .collect();
    let off_skin = OFF_SKIN * height;
    // Each point bound as the skin under it.
    let binder = Binder::new(bed);
    let bindings: Vec<VertexSkin> = (0..count * rows_count)
        .map(|here| binder.at(skin[here].unwrap_or(middle[here])))
        .collect();

    // The sheet's normal, by its own grid, turned out of the body by the neck's vote.
    let mut out: Vec<Vec3> = (0..count * rows_count)
        .map(|here| {
            let (column, row) = (here / rows_count, here % rows_count);
            let across = middle[at((column + 1).min(count - 1), row)]
                - middle[at(column.saturating_sub(1), row)];
            let down = middle[at(column, (row + 1).min(rows_count - 1))]
                - middle[at(column, row.saturating_sub(1))];
            across.cross(down).normalize_or(Vec3::Y)
        })
        .collect();
    let vote: f32 = (0..count * rows_count)
        .filter(|here| columns[here / rows_count].place == Place::Neck && here % rows_count > 0)
        .map(|here| out[here].dot(middle[here] - cores[here]) - out[here].x * height * 0.01)
        .sum();
    if vote < 0.0 {
        for normal in &mut out {
            *normal = -*normal;
        }
    }
    for column in 0..count {
        out[at(column, 0)] = out[at(column, 1)];
    }

    // The two surfaces.
    let stands_out = OUT_LIMP + (OUT_STIFF - OUT_LIMP) * stiffness;
    let mut outer = Vec::with_capacity(count * rows_count);
    let mut inner = Vec::with_capacity(count * rows_count);
    for (index, column) in columns.iter().enumerate() {
        let ridge = GROOVE + (1.0 - GROOVE) * (PI * column.across).det_sin();
        for (row, share) in rows.iter().enumerate() {
            let here = at(index, row);
            // How far down it is and how far it has left, in a full length's shares.
            let (down, left) = (share * column.span, (1.0 - share) * column.span);
            let swell = step(SWELL[0], SWELL[1], down)
                * (1.0 - ((down - SWELL[1]) / (1.0 - SWELL[1])).max(0.0))
                    .max(0.0)
                    .det_powf(FALL_POW)
                * step(0.0, CLOSES, left);
            let across = 1.0 - (1.0 - ridge) * step(GROOVE_FROM, 1.0, *share);
            let thickness = column.thick * swell * across;
            let mut sheet = middle[here];
            let mut lower = sheet - out[here] * (thickness * (1.0 - stands_out));
            // How far a lock lying on the skin has gone flat on it: over its frayed
            // end, so that its strands' tips lie on the skin and cast no shadow apart
            // from themselves - a dark line along the hem, which on a mane of the
            // coat's colour read as a seam (the forelock's front edge, horses 28
            // and 73).
            let mut flat = 0.0;
            if let Some(skin) = skin[here] {
                let up = (skin - cores[here]).normalize_or(out[here]);
                let gap = (sheet - skin).dot(up);
                let near = column.thick.max(off_skin);
                // On the skin while it runs close over it, and not while it is
                // still coming out of it.
                let on = step(ON_SKIN[1] * near, ON_SKIN[0] * near, gap)
                    * step(-0.4 * near, 0.1 * near, gap);
                flat = on * step(FRAY_FROM - FLATTENS, FRAY_FROM, *share);
                // Off the skin, and at its very hem a hair under it.
                let laid = skin + up * (off_skin * (1.0 - 2.0 * step(TUCK[0], TUCK[1], *share)));
                sheet = sheet.lerp(laid, on * (1.0 - step(0.0, HEM_DOWN, left)).max(flat));
                lower = lower.lerp(laid, on);
            }
            let mut upper = sheet + out[here] * (thickness * stands_out);
            // Never under the inner surface, and on it where the solid has no
            // thickness: at either end, its root and its hem, the two surfaces meet
            // and close it. Left apart there, the outer stood off the inner laid on
            // the skin, and the slit between them read as a dark seam.
            let apart = (upper - lower).dot(out[here]);
            if apart < 0.0 {
                upper -= out[here] * apart;
            }
            upper = upper.lerp(lower, flat);
            // Exactly, where they meet: a lerp is a rounding off its ends.
            if thickness <= 0.0 || flat >= 1.0 {
                upper = lower;
            }
            outer.push(upper);
            inner.push(lower);
        }
    }
    // Out of the ears and whatever the head wears.
    let capsules = capsules(bed, beast);
    let clear = CLEAR * height;
    for point in outer.iter_mut().chain(inner.iter_mut()) {
        for capsule in &capsules {
            *point = capsule.pushed(*point, clear);
        }
    }

    // Shaded as one strip, streaked along the hair (`streaked`).
    let outer_normals = streaked(&outer, &out, rows_count, 1.0);
    let inner_normals = streaked(&inner, &out, rows_count, -1.0);

    // Lock by lock, each across one lane of the strand mask: one colour, a shade
    // lighter toward the hem as a card is, and the underside darker at its root.
    let roots = sampler.mane();
    let tips = roots.lerp(Vec3::ONE, TIPPED);
    let mut solid = PolyMesh::new();
    // One surface of one lock: its columns, root to hem.
    let run = (per_lock + 1) * rows_count;
    let mut normals = Vec::with_capacity(LOCKS * 2 * run);
    for index in 0..LOCKS {
        let span = index * per_lock..=index * per_lock + per_lock;
        let middle_column = index * per_lock + per_lock / 2;
        let (lane_from, lane_to) =
            StrandMask::lane_span(mask::lane_of(columns[middle_column].root.at));
        for (surface, shading, facing) in [
            (&outer, &outer_normals, 1.0),
            (&inner, &inner_normals, -1.0),
        ] {
            let first = solid.positions.len() as u32;
            for (place, column) in span.clone().enumerate() {
                let x = place as f32 / per_lock as f32;
                for (row, share) in rows.iter().enumerate() {
                    let dark = if facing < 0.0 {
                        UNDER + (1.0 - UNDER) * step(HEM_LIT[0], HEM_LIT[1], *share)
                    } else {
                        1.0
                    };
                    solid.positions.push(surface[at(column, row)]);
                    normals.push(shading[at(column, row)]);
                    solid.uvs.push(Vec2::new(
                        lane_from
                            + (lane_to - lane_from) * (LANE_INSET + (1.0 - 2.0 * LANE_INSET) * x),
                        mask_row(*share),
                    ));
                    solid.colours.push(roots.lerp(tips, *share) * dark);
                    solid.skin.push(bindings[at(column, row)]);
                }
            }
            let vertex = |place: usize, row: usize| first + (place * rows_count + row) as u32;
            // Wound one way over the whole lock, by its quads' vote: a quad wound
            // apart from its neighbours would cancel their normals where it meets them.
            let mut wound = 0.0;
            for place in 0..per_lock {
                for row in 0..rows_count - 1 {
                    let corner = |at: u32| solid.positions[at as usize];
                    let (a, b, d) = (
                        corner(vertex(place, row)),
                        corner(vertex(place + 1, row)),
                        corner(vertex(place, row + 1)),
                    );
                    let away = out[at(index * per_lock + place, row + 1)] * facing;
                    wound += (b - a).cross(d - a).dot(away);
                }
            }
            for place in 0..per_lock {
                for row in 0..rows_count - 1 {
                    let quad = [
                        vertex(place, row),
                        vertex(place + 1, row),
                        vertex(place + 1, row + 1),
                        vertex(place, row + 1),
                    ];
                    solid.faces.push(if wound >= 0.0 {
                        quad.to_vec()
                    } else {
                        vec![quad[0], quad[3], quad[2], quad[1]]
                    });
                }
            }
        }
    }
    solid.set_normals(normals);
    // A few short cards out of the hem of a mane standing clear of the skin.
    if grid.fringe {
        for index in 0..LOCKS {
            for card in 0..FRINGE_PER_LOCK {
                let place = 1 + card * (per_lock - 2) / (FRINGE_PER_LOCK - 1).max(1);
                let column = index * per_lock + place.min(per_lock - 1);
                let Some(fringe) = Fringe::new(&middle, &out, &columns, &rows, column, stiffness)
                else {
                    continue;
                };
                let seat = Root {
                    at: fringe.start,
                    out: fringe.out,
                    weight: 1.0,
                    skin: bindings[at(column, fringe.row)],
                };
                loft(&mut solid, &seat, &fringe, None, roots, tips);
            }
        }
    }
    let tris = solid
        .faces
        .iter()
        .map(|face| face.len().saturating_sub(2))
        .sum();
    Some((
        solid,
        Grown {
            locks: LOCKS,
            tris,
            shaded: LOCKS * 2 * run,
        },
    ))
}

/// A surface's normals, one per column and row, shaded as one strip across every
/// lock and streaked along the hair: each read off the whole grid, so a column two
/// locks share is shaded alike in both and the strip has no crease between locks,
/// and turned about the way its column runs by its own share of [`STREAK`]. Turned
/// out of the body by `out` times `facing`.
fn streaked(surface: &[Vec3], out: &[Vec3], rows: usize, facing: f32) -> Vec<Vec3> {
    let count = surface.len() / rows;
    let at = |column: usize, row: usize| column * rows + row;
    (0..surface.len())
        .map(|here| {
            let (column, row) = (here / rows, here % rows);
            let away = out[here] * facing;
            let across = surface[at((column + 1).min(count - 1), row)]
                - surface[at(column.saturating_sub(1), row)];
            let down = surface[at(column, (row + 1).min(rows - 1))]
                - surface[at(column, row.saturating_sub(1))];
            let normal = across.cross(down).normalize_or(away);
            let normal = if normal.dot(away) < 0.0 {
                -normal
            } else {
                normal
            };
            let Some(along) = down.try_normalize() else {
                return normal;
            };
            Rot::axis_angle(along, STREAK * streak(column))
                .mul_vec3(normal)
                .normalize_or(normal)
        })
        .collect()
}

/// How far a column's normal turns, as a share of [`STREAK`]: every other column
/// the other way, each by its own amount, so no two streaks run alike and none
/// gathers a lock into a lump.
fn streak(column: usize) -> f32 {
    let side = if column.is_multiple_of(2) { 1.0 } else { -1.0 };
    let drawn = (column as u32).wrapping_mul(0x9E37_79B9) >> 24;
    side * (STREAK_LEAST + (1.0 - STREAK_LEAST) * drawn as f32 / 255.0)
}

/// The mask row a share down a column is drawn on.
fn mask_row(share: f32) -> f32 {
    if share < FRAY_FROM {
        share / FRAY_FROM * FRAY_ROW
    } else {
        FRAY_ROW + (share - FRAY_FROM) / (1.0 - FRAY_FROM) * (1.0 - FRAY_ROW)
    }
}

/// The skin a point of the mane is bound as: the skin's under it, read off the body's
/// nearest few vertices of the neck, the chest and the head by how near each is, so
/// that the solid bends where the skin it lies on bends and tears nowhere it does not.
struct Binder<'a> {
    bed: &'a Bed<'a>,
    /// The body's vertices a mane can lie on.
    under: Vec<usize>,
}

impl<'a> Binder<'a> {
    /// How many of the nearest vertices a binding is blended from.
    const NEAREST: usize = 3;

    fn new(bed: &'a Bed<'a>) -> Self {
        let under = (0..bed.body.vertex_count())
            .filter(|&vertex| matches!(bed.zones[vertex], Zone::Neck | Zone::Chest | Zone::Head))
            .collect();
        Self { bed, under }
    }

    fn at(&self, point: Vec3) -> VertexSkin {
        let mut nearest: Vec<(f32, usize)> = Vec::with_capacity(Self::NEAREST + 1);
        for &vertex in &self.under {
            let off = self.bed.body.positions[vertex].distance_squared(point);
            if nearest.len() < Self::NEAREST || off < nearest[nearest.len() - 1].0 {
                let at = nearest.partition_point(|(other, _)| *other <= off);
                nearest.insert(at, (off, vertex));
                nearest.truncate(Self::NEAREST);
            }
        }
        let Some(&(_, first)) = nearest.first() else {
            return VertexSkin::default();
        };
        let mut skin = self.bed.weights.vertices[first];
        let mut held = 1.0 / nearest[0].0.sqrt().max(1.0e-5);
        for &(off, vertex) in &nearest[1..] {
            let weight = 1.0 / off.sqrt().max(1.0e-5);
            held += weight;
            skin = blended(&skin, &self.bed.weights.vertices[vertex], weight / held);
        }
        skin
    }
}

/// The crest a share of the way up it.
fn crested(line: &Crest, share: f32) -> Crested {
    let along = share.clamp(0.0, 1.0) * (line.line.len() - 1) as f32;
    let piece = (along as usize).min(line.line.len() - 2);
    line.between(piece, along - piece as f32)
}

/// A root on the crest a share of the way up, parted to the off side, thinning only
/// into the withers: past the poll the head's columns go on.
fn neck_root(bed: &Bed, line: &Crest, normals: &[Vec3], share: f32) -> Option<Root> {
    let here = crested(line, share);
    let nearest = (0..bed.body.vertex_count()).min_by(|&a, &b| {
        bed.body.positions[a]
            .distance_squared(here.at)
            .total_cmp(&bed.body.positions[b].distance_squared(here.at))
    })?;
    let faces = normals[nearest] - Vec3::X * normals[nearest].x;
    Some(Root {
        at: here.at,
        out: ((faces + here.up).normalize_or(here.up) - Vec3::X * PARTING).normalize_or(here.up),
        weight: step(0.0, 0.3, share).max(0.02),
        skin: bed.weights.vertices[nearest],
    })
}

fn neck_column(
    bed: &Bed,
    line: &Crest,
    normals: &[Vec3],
    share: f32,
    reach: f32,
    thick: f32,
) -> Option<Column> {
    let root = neck_root(bed, line, normals, share)?;
    let here = crested(line, share);
    Some(Column {
        place: Place::Neck,
        reach,
        across: 0.0,
        span: reach,
        thick: thick * root.weight.sqrt(),
        core: here.at - here.up * here.skin[0],
        run: here.run,
        root,
    })
}

/// The line over the head: from the crest's end to the whorl between the ears, read
/// off the declared head as rays from the braincase's centre in its middle plane.
/// Body space, a point a step, the crest's end first.
fn head_path(head: &Head, crest_end: Vec3) -> Vec<Vec3> {
    const STEPS: usize = 24;
    let beast = head.beast;
    let centre = head.centre;
    let end = beast.local(crest_end) - centre;
    let back = end.z.det_atan2(end.y);
    let surface = |angle: f32| {
        let (sin, cos) = sin_cos(angle);
        let direction = Vec3::new(0.0, cos, sin);
        march(
            |point| beast.drawn_distance(point),
            centre,
            direction,
            beast.radius,
        )
    };
    // The whorl: on the middle line, level with the ears' bases.
    let whorl_z = beast.ear.base.z;
    let (mut low, mut high) = (back, 1.2f32);
    for _ in 0..24 {
        let middle = 0.5 * (low + high);
        match surface(middle) {
            Some(at) if at.z < whorl_z => low = middle,
            _ => high = middle,
        }
    }
    let first = surface(back).unwrap_or(end + centre);
    let miss = (end + centre) - first;
    (0..=STEPS)
        .map(|step_at| {
            let share = step_at as f32 / STEPS as f32;
            let angle = back + (low - back) * share;
            let at = surface(angle).unwrap_or(end + centre) + miss * (1.0 - share);
            beast.body(at)
        })
        .collect()
}

/// Read along `path` a share of the way, by its length.
fn along(path: &[Vec3], share: f32) -> Vec3 {
    let lengths: Vec<f32> = path
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .collect();
    let total: f32 = lengths.iter().sum();
    let mut left = share.clamp(0.0, 1.0) * total;
    for (piece, length) in lengths.iter().enumerate() {
        if left <= *length {
            return path[piece].lerp(path[piece + 1], left / length.max(f32::EPSILON));
        }
        left -= length;
    }
    *path.last().unwrap_or(&Vec3::ZERO)
}

/// Mixed bindings: each joint's weight in either, by `share` of the second.
fn blended(first: &VertexSkin, second: &VertexSkin, share: f32) -> VertexSkin {
    let mut weights: Vec<(u16, f32)> = Vec::new();
    for (skin, scale) in [(first, 1.0 - share), (second, share)] {
        for influence in skin {
            if influence.weight <= 0.0 {
                continue;
            }
            match weights
                .iter_mut()
                .find(|(joint, _)| *joint == influence.joint)
            {
                Some((_, weight)) => *weight += influence.weight * scale,
                None => weights.push((influence.joint, influence.weight * scale)),
            }
        }
    }
    weights.retain(|(_, weight)| *weight > 0.0);
    weights.sort_by(|a, b| b.1.total_cmp(&a.1));
    weights.truncate(4);
    let total: f32 = weights.iter().map(|(_, weight)| weight).sum();
    let mut skin = VertexSkin::default();
    for (slot, (joint, weight)) in weights.into_iter().enumerate() {
        skin[slot] = Influence {
            joint,
            weight: weight / total.max(f32::EPSILON),
        };
    }
    skin
}

fn head_column(
    head: &Head,
    path: &[Vec3],
    end_root: &Root,
    fan: f32,
    (reach, thick): (f32, f32),
) -> Option<Column> {
    // A forelock's roots spread across the poll between the ears, each to the side
    // it falls toward, rather than fanning from one point.
    let beast = head.beast;
    let spread = (beast.ear.base.x - beast.ear.width * 0.5).max(0.0) * FORELOCK_SPREAD;
    let aside =
        spread * (fan_heading(fan) / HEADING[1]).clamp(-1.0, 1.0) * step(TURN - 0.15, TURN, fan);
    let at = along(path, step(0.0, ROOTS_REACH, fan)) + beast.turn * Vec3::NEG_X * aside;
    let (on, out) = head.surface(at)?;
    Some(Column {
        place: Place::Head(fan),
        reach,
        across: 0.0,
        span: reach,
        thick: thick * (1.0 + (FORELOCK_THICK - 1.0) * step(0.3, 0.7, fan)),
        core: beast.body(head.centre),
        run: Vec3::ZERO,
        root: Root {
            at: on,
            out,
            weight: 1.0,
            skin: end_root.skin,
        },
    })
}

/// How long the hair over the head is against the mane's, a share of the way
/// through the fan: the mane's at the crest's end, shortest where it turns past the
/// off ear, a forelock's from there on.
fn fan_length(fan: f32, stiffness: f32) -> f32 {
    // A standing crest has no ear to clear, but goes on falling away over the poll.
    let dip = POLL_DIP + (STIFF_DIP - POLL_DIP) * stiffness * stiffness;
    // From where the crest's last column left off.
    let start = 1.0 - POLL_TAPER * stiffness * stiffness;
    let dipped = start + (dip - start) * step(0.0, 0.25, fan);
    dipped + (FORELOCK - dipped) * step(0.35, 0.65, fan)
}

/// How stiff the hair over the head stands, a share of the way through the fan: the
/// mane's own stiffness at the crest's end, and a forelock's past the turn, which
/// lies on the forehead until the mane is well past its default and stands as a
/// brush only toward the slider's end.
fn fan_stiffness(stiffness: f32, fan: f32) -> f32 {
    let forelock = step(FORELOCK_STANDS[0], FORELOCK_STANDS[1], stiffness);
    stiffness + (forelock - stiffness) * step(0.0, 0.35, fan)
}

/// Which way a column over the head heads, in radians from forward toward the off
/// side: from the neck's round to straight ahead by the turn, and on across the
/// forelock to a little toward the near side.
fn fan_heading(fan: f32) -> f32 {
    if fan < TURN {
        HEADING[0] + (HEADING[1] - HEADING[0]) * (fan / TURN)
    } else {
        HEADING[1] + (HEADING[2] - HEADING[1]) * ((fan - TURN) / (1.0 - TURN))
    }
}

/// What a column lies on: the neck's skin round the crest's bone, or the declared
/// head round the braincase's centre.
struct Ground<'a> {
    head: &'a Head<'a>,
    crest: &'a Crest,
}

impl Ground<'_> {
    /// The point of the bone under the crest nearest `at`.
    fn bone(&self, at: Vec3) -> Vec3 {
        let (piece, t) = self.crest.place(at);
        let here = self.crest.between(piece, t);
        here.at - here.up * here.skin[0]
    }

    /// Where the neck's skin is round the bone from `at`, and the way out to it from
    /// the bone: round the crest's measured section, or on the neck's declared upper
    /// run where the head declares one and the drawn neck is shaped onto it.
    fn body(&self, at: Vec3) -> Option<(Vec3, Vec3)> {
        let (piece, t) = self.crest.place(at);
        let here = self.crest.between(piece, t);
        let bone = here.at - here.up * here.skin[0];
        let off = at - bone;
        let flat = off - here.run * off.dot(here.run);
        let side = if flat.x < 0.0 { -1.0 } else { 1.0 };
        let turned = flat.x.abs().det_atan2(flat.dot(here.up));
        let (sin, cos) = sin_cos(turned);
        let way = (here.up * cos + Vec3::X * (side * sin)).normalize_or(here.up);
        let mut skin = bone + way * here.skin_at(turned);
        // The section was read off the body as built, which is the declared run only
        // where the build has shaped the neck wholly onto it: there the run, past
        // where the section's rays reach round as well.
        let beast = self.head.beast;
        if let Some((on, share)) = beast.nape_skin(beast.local(at)) {
            skin = skin.lerp(beast.body(on), step(NAPE_FROM, 1.0, share));
        }
        Some((skin, (skin - bone).normalize_or(way)))
    }
}

/// A neck column's curve: walked down the neck's side lying on it, and as the mane
/// stiffens, a stiff mane card's curve standing off the crest (`Lock`).
#[allow(clippy::too_many_arguments)]
fn neck_curve(
    ground: &Ground,
    lock: &Lock,
    root: &Root,
    run: Vec3,
    reach: f32,
    rows: &[f32],
    stiffness: f32,
    sink: f32,
) -> Vec<Vec3> {
    let (sin, cos) = sin_cos(SWEEP);
    let way = Vec3::NEG_X * sin + run * cos;
    let length = lock.length(root) * reach;
    let walked = walk(
        &|at| ground.body(at),
        root,
        way,
        length,
        &|_| false,
        sink,
        NECK_LIE,
    );
    let stands = step(STANDS[0], STANDS[1], stiffness);
    rows.iter()
        .map(|share| {
            let lying = sample(&walked, *share);
            if stands <= 0.0 {
                lying
            } else {
                lying.lerp(Shape::at(lock, root, share * reach), stands)
            }
        })
        .collect()
}

/// A column's middle curve at each row's share.
fn curve(
    ground: &Ground,
    lock: &Lock,
    column: &Column,
    end: &(Root, Vec3, Vec3),
    rows: &[f32],
    stiffness: f32,
    sink: f32,
) -> Vec<Vec3> {
    let fan = match column.place {
        Place::Neck => {
            return neck_curve(
                ground,
                lock,
                &column.root,
                column.run,
                column.reach,
                rows,
                stiffness,
                sink,
            );
        }
        Place::Head(fan) => fan,
    };
    let head = ground.head;
    let stiff = fan_stiffness(stiffness, fan);
    let forelock = step(0.35, 0.65, fan);
    let length = lock.length(&Root {
        weight: 1.0,
        ..column.root
    }) * fan_length(fan, stiff)
        * column.reach
        * (1.0 - FORELOCK_BRUSH * forelock * stiff * stiff);
    // A forelock stops at the eyes, each lock a little short of or past them as it is
    // shorter or longer: how far past them a point is, down the head's axis.
    let beast = head.beast;
    let stop = beast.eye.radius * (PAST_EYES + 3.0 * (column.reach - 0.9));
    let [back, nose] = beast.axis;
    let axis = (nose - back).normalize_or(Vec3::Z);
    let eye_line = beast.eye.centre * Vec3::new(0.0, 1.0, 1.0);
    let past_eyes = |at: Vec3| (beast.local(at) - eye_line).dot(axis) > stop;
    let heading = fan_heading(fan);
    let (sin, cos) = sin_cos(heading);
    let way = beast.turn * Vec3::new(-sin, 0.0, cos);
    let limp = walk(
        &|at| head.surface(at),
        &column.root,
        way,
        length,
        &past_eyes,
        sink,
        FORELOCK_LIE,
    );
    let stood = step(0.0, 0.2, stiff);
    let stiff = (stood > 0.0).then(|| standing(&column.root, way, length, stiff, sink));
    let handed = step(0.0, HANDOVER, fan);
    // The neck's last column, turned onto this root's own facing.
    let from_neck = (handed < 1.0).then(|| {
        let (end, run, up) = end;
        let turn = from_rotation_arc(*up, column.root.out);
        // As short as the crest's last column, which a standing crest has tapered
        // toward the poll.
        let reach = column.reach * (1.0 - POLL_TAPER * stiffness * stiffness);
        neck_curve(ground, lock, end, *run, reach, rows, stiffness, sink)
            .into_iter()
            .map(|at| turn * (at - end.at))
            .collect::<Vec<_>>()
    });
    rows.iter()
        .enumerate()
        .map(|(row, share)| {
            let limp = sample(&limp, *share);
            let on_head = stiff
                .as_ref()
                .map_or(limp, |stiff| limp.lerp(sample(stiff, *share), stood));
            match &from_neck {
                Some(neck) => (neck[row] + column.root.at).lerp(on_head, handed),
                None => on_head,
            }
        })
        .collect()
}

/// A walked curve read a share of the way along it.
fn sample(points: &[Vec3], share: f32) -> Vec3 {
    let at = share.clamp(0.0, 1.0) * (points.len() - 1) as f32;
    let below = (at as usize).min(points.len() - 2);
    points[below].lerp(points[below + 1], at - below as f32)
}

/// A limp column, walked from its root along `way` for `length` over what `land`
/// says is under a point (where the surface is and which way it faces), lying
/// `lie` of its length off it at its root and its tip and coming out from `sink`
/// under it; letting go and hanging where the surface turns under, and holding
/// still once `stop` says it has gone far enough.
fn walk(
    land: &dyn Fn(Vec3) -> Option<(Vec3, Vec3)>,
    root: &Root,
    way: Vec3,
    length: f32,
    stop: &dyn Fn(Vec3) -> bool,
    sink: f32,
    lie: [f32; 2],
) -> Vec<Vec3> {
    let down = Vec3::NEG_Y;
    let tangent = |way: Vec3, normal: Vec3| (way - normal * way.dot(normal)).normalize_or(way);
    let ds = length / WALK as f32;
    let bends = ds / length.max(1e-6);
    let mut walked = Vec::with_capacity(WALK + 1);
    let mut on = root.at;
    let mut normal = root.out;
    let mut way = tangent(way, normal);
    let mut hanging = false;
    for step_at in 0..=WALK {
        let share = step_at as f32 / WALK as f32;
        let off = length * (lie[0] + (lie[1] - lie[0]) * share);
        let rising = step(0.0, 0.12, share);
        walked.push(on + normal * (off * rising - sink * (1.0 - rising)));
        if step_at == WALK {
            break;
        }
        let next = on + way * ds;
        match (!hanging).then(|| land(next)).flatten() {
            Some((at, facing)) if facing.y > LETS_GO => {
                // Stopped short, a column is as long as it went: its rows spread over
                // that, rather than piling up where it stopped.
                if stop(at) {
                    break;
                }
                way = tangent((at - on).normalize_or(way) + down * (0.15 * bends), facing);
                on = at;
                normal = facing;
            }
            _ => {
                hanging = true;
                way = (way + down * (2.0 * bends)).normalize_or(down);
                on = next;
            }
        }
    }
    if walked.len() < 2 {
        walked.push(walked[0] + way * ds.max(f32::EPSILON));
    }
    walked
}

/// A stiff column over the head: off it, leaning the way it heads as a stiff mane's
/// card does (`Lock`), bending over past as much of it as its stiffness stands.
fn standing(root: &Root, way: Vec3, length: f32, stiffness: f32, sink: f32) -> Vec<Vec3> {
    let ds = length / WALK as f32;
    let normal = root.out;
    let start = (way - normal * way.dot(normal)).normalize_or(way);
    let leans = FRAC_PI_2 * (1.0 - stiffness.det_powf(0.4));
    let (sin, cos) = sin_cos(leans);
    let rise = (start * sin + normal * cos).normalize_or(normal);
    let over = (start + Vec3::NEG_Y * 0.6).normalize_or(Vec3::NEG_Y);
    let stands = stiffness.det_powf(1.35) * length;
    let mut stiff = Vec::with_capacity(WALK + 1);
    let mut at = root.at - normal * sink;
    let mut going = rise;
    for step_at in 0..=WALK {
        stiff.push(at);
        if step_at as f32 * ds > stands {
            going = going
                .lerp(over, (4.0 * ds / length.max(1e-6)).min(1.0))
                .normalize_or(over);
        }
        at += going * ds;
    }
    stiff
}

/// A capsule a mane is kept out of: a line with a radius at each end.
pub(super) struct Capsule {
    pub from: Vec3,
    pub to: Vec3,
    pub radius: [f32; 2],
}

impl Capsule {
    /// How far `point` is inside the capsule widened by `clear`: positive inside.
    #[cfg(test)]
    pub(super) fn inside(&self, point: Vec3, clear: f32) -> f32 {
        let (closest, radius) = self.nearest(point, clear);
        radius - point.distance(closest)
    }

    /// The point of its line nearest `point`, and its radius there widened by
    /// `clear`.
    fn nearest(&self, point: Vec3, clear: f32) -> (Vec3, f32) {
        let run = self.to - self.from;
        let t =
            ((point - self.from).dot(run) / run.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
        (
            self.from + run * t,
            self.radius[0] + (self.radius[1] - self.radius[0]) * t + clear,
        )
    }

    fn pushed(&self, point: Vec3, clear: f32) -> Vec3 {
        let (closest, radius) = self.nearest(point, clear);
        let off = point - closest;
        if off.length() >= radius {
            return point;
        }
        let away = off
            .try_normalize()
            .unwrap_or_else(|| (self.to - self.from).cross(Vec3::Y).normalize_or(Vec3::X));
        closest + away * radius
    }
}

/// Round each ear and each piece the head wears.
pub(super) fn capsules(bed: &Bed, beast: &BeastHead) -> Vec<Capsule> {
    let mut capsules = Vec::new();
    if let Some(ears) = bed.ears {
        let at = bed.rig.joints[ears.head].position;
        let half = beast.ear.width * EAR_WIDTH;
        for ear in [&ears.left, &ears.right] {
            capsules.push(Capsule {
                from: at + ear.base,
                to: at + ear.tip,
                radius: [half, half * 0.3],
            });
        }
    }
    for seat in beast.ornaments.iter().flatten() {
        let sides = if seat.pair {
            vec![*seat, seat.mirrored()]
        } else {
            vec![*seat]
        };
        for side in sides {
            let line = side.line();
            let last = (line.len() - 1).max(1) as f32;
            for (index, pair) in line.windows(2).enumerate() {
                capsules.push(Capsule {
                    from: beast.body(pair[0]),
                    to: beast.body(pair[1]),
                    radius: [
                        side.radius_at(index as f32 / last) * 1.1,
                        side.radius_at((index + 1) as f32 / last) * 1.1,
                    ],
                });
            }
        }
    }
    capsules
}

/// One short card out of a lock's hem.
struct Fringe {
    /// The row it is rooted on.
    row: usize,
    start: Vec3,
    way: Vec3,
    across: Vec3,
    out: Vec3,
    length: f32,
    width: f32,
}

impl Fringe {
    fn new(
        middle: &[Vec3],
        out: &[Vec3],
        columns: &[Column],
        rows: &[f32],
        column: usize,
        stiffness: f32,
    ) -> Option<Self> {
        let count = columns.len();
        let row_count = rows.len();
        if column == 0 || column + 1 >= count {
            return None;
        }
        // Only off a hem that stands clear of the skin: a lock lying on the neck ends
        // in its own frayed points, and cards lying flat beside them read as flakes.
        let standing = step(STANDS[0], STANDS[1], stiffness);
        if standing < FRINGE_FROM || columns[column].thick <= 0.0 {
            return None;
        }
        let at = |column: usize, row: usize| middle[column * row_count + row];
        let row = rows
            .iter()
            .position(|share| *share >= FRINGE_ROOT)
            .unwrap_or(row_count - 2);
        let start = at(column, row);
        let hem = at(column, row_count - 1);
        let way = (hem - at(column, row_count - 2)).normalize_or(Vec3::NEG_Y);
        let column_length: f32 = (1..row_count)
            .map(|row| at(column, row).distance(at(column, row - 1)))
            .sum();
        let forelock = match columns[column].place {
            Place::Head(fan) => step(0.3, 0.7, fan),
            Place::Neck => 0.0,
        };
        let reach =
            FRINGE_REACH * standing * (1.0 - FRINGE_STIFF * stiffness) * (1.0 - 0.4 * forelock);
        let length = start.distance(hem) + column_length * reach;
        if length <= f32::EPSILON {
            return None;
        }
        let across = (at(column + 1, row) - at(column - 1, row)).normalize_or(Vec3::Z);
        let width = at(column - 1, row).distance(at(column + 1, row)) * 0.5;
        Some(Self {
            row,
            start,
            way,
            across,
            out: out[column * row_count + row],
            length,
            width: width.max(1e-4),
        })
    }
}

impl Shape for Fringe {
    fn length(&self, _: &Root) -> f32 {
        self.length
    }

    fn at(&self, _: &Root, along: f32) -> Vec3 {
        let heading = (self.way + Vec3::NEG_Y * (0.6 * along)).normalize_or(self.way);
        let bend = (self.way + heading).normalize_or(self.way);
        self.start + bend * (self.length * along)
    }

    fn width(&self, _: &Root) -> (f32, f32) {
        (self.width, self.width * 0.6)
    }

    fn across(&self, _: &Root) -> Vec3 {
        self.across
    }

    fn flatness(&self) -> f32 {
        self.length * 0.02
    }

    fn shade_at(&self, _: &Root, along: f32) -> f32 {
        0.85 + 0.15 * (along * FRAC_PI_2).det_sin()
    }
}

#[cfg(test)]
mod tests;
