//! The shell generator: hair as a closed sculpted solid on the follicle
//! envelope, with a broken rim of cards.
//!
//! **A sibling of [`clump`](super::clump), not a [`Shape`]**: a shell is a
//! lofted solid rather than a ribbon per clump, so it is built here and drawn
//! once beside a region's cards. A style asks for one through
//! [`Shape::shell`] exactly as a tied-back head asks for its knot lump, and
//! [`Growth::grow`](super::clump::Growth::grow) emits it inside the region's
//! own accounting - so the ledger, the tier and `tests/budget.rs` pay for it
//! as they pay for a card.
//!
//! # Why a solid, when flat cards are what this crate draws
//!
//! Because flat cards cannot lie UNDER flat cards (#339). A card is a tangent
//! plane, so on a curved head its edges stand off the surface by about
//! `w^2 / 2R`: at a long style's crest a top card's edges sit 4 mm out and a
//! double-width bed card's 16 mm, which is outside the hair it was meant to
//! back. The sound under-layer is a CLOSED CURVED SURFACE, and that is what
//! this builds - the foundation the helmet family is cut from.
//!
//! A sculpted shell existed before (2026-08-03) and was discarded at #198 for
//! reading as a helmet while trying to be realistic hair. A DELIBERATE helmet
//! family is a different target, so its loft is revived here on the CURRENT
//! follicle envelope: columns by azimuth, rows down the meridian, a closed
//! solid of two surfaces, and locks at the rim - which is the one thing the
//! discarded shell got right, since the edge is what makes a shell read as
//! hair and not as a swim cap.
//!
//! # What is measured rather than assumed
//!
//! - **A column DRAPES.** It walks the measured skull down its own meridian and
//!   keeps the widest radius it has passed, exactly as a scalp card does: hair
//!   is held out by whatever it has draped over. Walked without that rule, the
//!   straight-back column of a default head ends 40 mm inside the neck
//!   (measured, #345).
//! - **Rows are placed where the columns BEND**, not evenly along them, and on
//!   one schedule shared by every column (this module's own `schedule`, and the
//!   chevrons a per-column schedule drew). The back of a head is twice the arc of the front - 213 mm
//!   against 112 on a default body - and an even schedule leaves the back's row
//!   polyline straying up to 3.1 mm inside the head while the front has rows to
//!   spare. The crown's cap is NOT where that stray lives, which was worth
//!   measuring: a schedule that spent three rows on it was worse at the back
//!   than an even one.
//! - **The rim is where the mask says the hair stops**, azimuth by azimuth, and
//!   never a height: a hairline is 100 mm of forehead above the brow and 20 mm
//!   below the occiput at the nape.
//!
//! # What it costs
//!
//! [`COLUMNS`] by [`ROWS`] over two surfaces, plus a fan at the crown and a
//! bevelled band at the rim. The count is the head's own and not a record's: a
//! shell costs what the head's size dictates, which is what made the discarded
//! one affordable and is why nothing here is a wire field.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::clump::{Root, Seating, Shape};
use super::follicle::{Follicle, Follicles};
use super::mask::{self, StrandMask};
use super::style::{Cut, ScalpStyle, Sown, Tress};
use crate::mesh::{PolyMesh, VertexSkin};

/// How many columns a shell is swept with, round the whole head.
///
/// **The head's own girth decides this, not a record.** At the widest parallel
/// a measured head reaches - about 98 mm of radius at the occiput - thirty-six
/// columns is a step of 17 mm, which is under what a facet reads as at the
/// framing a head is judged from, and half what the scalp's coarsest card is
/// wide.
///
/// Provenance: **derived** from the measured parallel and the card it sits
/// beside.
pub const COLUMNS: usize = 36;

/// How many rows each column is lofted with, crown to rim.
///
/// **Placed by where the columns bend** (this module's own `schedule`), so this is a budget
/// rather than a resolution: twelve rows over two surfaces is 1,584 triangles
/// of quads on thirty-six columns, which is what the issue's 1,000 to 2,300
/// allows with the crown's fan and the rim's band on top.
///
/// Provenance: **derived** from the budget, **checked by the stray** a column
/// of this many rows leaves (#345).
pub const ROWS: usize = 12;

/// How far a shell's inner surface stands off the walked envelope, in metres.
///
/// **Bigger than a card's [`LIFT`](super::clump::LIFT), and measured rather
/// than carried.** The envelope a column walks sits between 0.6 and 4.2 mm
/// outside the built skin over the scalp, but at the nape's rim it dips 2.0 mm
/// INSIDE it - and a row polyline strays further inside still than the walk it
/// stands for. A card at `LIFT` can be a little under the skin at one station
/// and nobody sees it; a closed solid with a vertex under the skin is a shell
/// with the head poking through it.
///
/// Provenance: **derived** from the measured envelope depth and the row stray
/// (#345).
const STAND: f32 = 0.004;

/// How thick the shell is at the crown and at the rim, as shares of the head's
/// own half-width.
///
/// **Thicker than the discarded shell's 0.035 head radii, and deliberately
/// so.** That figure was the most a shell could carry while trying to be
/// realistic hair - past it, its own author's note says, it starts to read as a
/// helmet. This family IS the helmet, so the crown carries real volume and the
/// rim thins to an edge: a shell of even thickness ends in a slab, and a slab
/// across the bottom of a head of hair is most of what makes it a bonnet.
///
/// Provenance: **carried** from the discarded shell's own measurement,
/// **thickened by render** for a family that means to read as sculpted.
const THICKNESS: [f32; 2] = [0.055, 0.016];

/// The power the thickness falls off from crown to rim with.
///
/// Squared, so the shell keeps its volume over the vault and gives it up over
/// the last part of the descent rather than thinning steadily all the way down,
/// which reads as a cone.
///
/// Provenance: **tuned by render**.
const THICKNESS_POW: f32 = 2.0;

/// How far the rim's bevel ring is carried past the surfaces it joins, as a
/// share of the thickness there.
///
/// A rim closed by one quad from the outer surface to the inner is a cut edge,
/// and a cut edge catches the light as a bright line all round the head. A
/// middle ring carried a little further down the meridian turns it into a
/// rounded lip.
///
/// Provenance: **tuned by render**.
const BEVEL: f32 = 0.6;

/// The mask weight a shell's rim sits at.
///
/// The scalp styles' own [`EDGE`](super::style::scalp::EDGE): where a card
/// stops lying on the scalp and starts hanging is where the hair ends, and a
/// shell that ended anywhere else would end somewhere no card does.
const EDGE: f32 = super::style::scalp::EDGE;

/// How finely a column's meridian is walked, in metres.
///
/// A millimetre and a half: under the loft's own flatness tolerance, so the
/// rows placed along it are choosing from a curve rather than from a polyline
/// of their own.
///
/// Provenance: **derived** from the loft's tolerance.
const WALK: f32 = 0.0015;

/// How much of the roots' colour the shell's underside keeps.
///
/// The inside of a mass of hair faces the head and almost nothing reaches it,
/// which is what says the shell is a volume rather than a cut-out.
///
/// Provenance: **carried** from the discarded shell, **tuned by render**.
const UNDER_SHADE: f32 = 0.6;

/// Which row of the strand mask a shell's texture coordinates sit on.
///
/// The same row a lump takes, and for the same reason: inside the part of every
/// lane the mask keeps whole, so the cut-out material a consumer binds draws the
/// shell solid rather than cutting holes in it.
const SHELL_ROW: f32 = super::clump::loft::LUMP_ROW;

/// What one style asks the generator for.
///
/// **A description rather than a curve**, so a catalogue entry is a handful of
/// numbers and the geometry is decided in one place. What a style says is how
/// thick it is, how its rim is cut, and whether it is faceted; everything else -
/// the columns, the rows, the drape, the crown's pole - is this module's
/// business.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shell {
    /// How thick it is at the crown, as a share of the head's half-width.
    pub crown: f32,
    /// How thick at the rim, likewise.
    pub rim: f32,
    /// How far the rim is cut back from the mask's own hairline at the front,
    /// in metres: a fringe notch. Negative carries the rim past the hairline.
    pub fringe: f32,
    /// The same at the nape.
    pub nape: f32,
    /// Whether its normals are faceted rather than smooth.
    ///
    /// A catalogue constant and never a record axis (the owner's decision):
    /// faceted, painterly or smooth is what a named style IS.
    pub faceted: bool,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            crown: THICKNESS[0],
            rim: THICKNESS[1],
            fringe: 0.0,
            nape: 0.0,
            faceted: false,
        }
    }
}

/// The helmet family's prototype: a plain cap of hair.
///
/// **Not a style on the wire.** #345 is the generator and the catalogue is
/// #346, so this is asked for through `AvatarConfig::helmet` and nothing a
/// record can say reaches it. It is what the generator is sheeted and judged
/// on, in both renderers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cap {
    /// The solid it wears.
    pub shell: Shell,
    /// How many meridians its rim cards are seated on, of which the fringe's
    /// and the nape's grow. See [`Helmet::length`].
    pub rim_cards: usize,
}

impl Default for Cap {
    fn default() -> Self {
        Self {
            shell: Shell::default(),
            rim_cards: RIM_CARDS,
        }
    }
}

/// How many meridians a rim's cards are seated on.
///
/// Sixteen round the head, of which the fringe's and the nape's grow: the
/// issue's own 8 to 16 locks, seated the way a scalp style seats its cards so
/// the rim's breaks are spread round the head rather than scattered by area
/// (#316's lesson).
///
/// Provenance: **from the issue**.
const RIM_CARDS: usize = 16;

/// How wide a rim card is, in metres.
///
/// **Costed against the shell before it was drawn** (#345, and #339's rule): a
/// card laid over a curved shell stands its own `w^2 / 2R` off it, and the
/// tightest parallel a rim crosses is the temple's, measured at 28 to 44 mm. At
/// twenty millimetres a card stands at most 1.9 mm proud, which is inside the
/// hair's own thickness there; at the scalp's coarsest width of seventy it
/// would stand twenty millimetres off and read as a plate stuck to the side of
/// the head.
///
/// Provenance: **derived** from the measured parallel at the rim.
const RIM_WIDTH: f32 = 0.020;

/// What share of its width a rim card keeps at its tip.
///
/// The scalp's own taper, since the strand mask cuts the end either way.
///
/// Provenance: **carried** from the scalp's `TAPER`.
const RIM_TAPER: f32 = 0.5;

/// How far a rim card hangs past the shell's rim at a middling cut, in metres.
///
/// Provenance: **tuned by render**.
const RIM_REACH: f32 = 0.022;

/// How far up the shell a rim card starts, in metres of its own travel.
///
/// Enough that the card overlaps the solid it breaks rather than starting at
/// its edge, which would draw a line round the head exactly where the rim
/// already is - and no more, because **what lies on the shell is what reads as
/// a plate stuck to it** (#345). At eighteen millimetres the lying part drew a
/// pale tab on the helmet's forehead in both renderers; at eight it is a tuft
/// emerging from under the rim, and the line the overlap exists to prevent
/// still does not appear.
///
/// Provenance: **tuned by render**, **cut by the defect it caused**.
const RIM_ROOT: f32 = 0.008;

/// Where a rim card grows, as a cosine of its azimuth: the fringe forward of
/// the first, the nape behind the second.
///
/// **The rim is BROKEN, not fringed all round.** A lock at every meridian is a
/// hem, which is the picket fence this whole overhaul is about; the two places
/// a helmet's edge wants breaking are the ones hair actually parts over, the
/// brow and the nape.
///
/// Provenance: **from the issue** (fringe and nape), **tuned by render**.
const RIM_AT: [f32; 2] = [0.55, -0.45];

/// How far a rim card stands off the shell it lies on, in metres.
///
/// Its own lift, over the shell's outer surface rather than over the skin: a
/// card whose chords sag into the shell draws the shell's colour through
/// itself.
///
/// **Bigger than a card's [`LIFT`](super::clump::LIFT), because it is lifted
/// off a lofted surface rather than off a measured one.** At the loft's own
/// tolerance, four of a rim's 294 vertices read 0.53 mm INSIDE the shell
/// (measured, #345): the shell's rows are a polyline through a walk, so its
/// surface bulges between them by as much as a card's chords sag, and the two
/// tolerances add rather than cancel.
///
/// Measured again at three millimetres: seed 42 and seed 7 read clean and the
/// default head still had two of its 294 rim vertices 0.34 mm inside, so this
/// is the step that takes every head to none.
///
/// Provenance: **derived** from the loft's tolerance and the shell's own row
/// stray, **measured** against the built rim.
const RIM_LIFT: f32 = 0.004;

impl Cap {
    /// What this prototype grows on one head, ready for the clump engine.
    ///
    /// **One place, because two callers need it**: `Avatar::build_with`, which
    /// wears it in place of the record's scalp style, and `tests/budget.rs`,
    /// which costs it. Two copies would be two opinions about what the body
    /// draws - the trap `the_regrown_hair_is_the_hair_the_body_ships` exists
    /// for.
    #[must_use]
    pub fn sowing(&self, scalp: &Tress<ScalpStyle>, head: &Follicles) -> Sown {
        Sown {
            shape: Box::new(Helmet {
                regions: head.clone(),
                shell: self.shell,
                cut: scalp.cut,
            }),
            clumps: self.rim_cards,
            roots: scalp.roots,
            tips: scalp.tips,
        }
    }
}

/// One head's worth of helmet: the shell, and the cards that break its rim.
///
/// A [`Shape`] like any other - its clumps are the rim's locks - which also
/// answers [`Shape::shell`], so the solid is drawn beside them and counted with
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct Helmet {
    /// The measured head this is fitted to, and where hair may grow on it.
    regions: Follicles,
    /// The solid.
    shell: Shell,
    /// How the rim's cards are cut.
    cut: Cut,
}

impl Helmet {
    /// Which way round the head a root sits, from dead ahead.
    fn azimuth(root: &Root) -> f32 {
        root.at.x.atan2(root.at.z)
    }

    /// Whether the rim is broken at this azimuth: the fringe, or the nape.
    fn breaks(azimuth: f32) -> bool {
        !(RIM_AT[1]..=RIM_AT[0]).contains(&azimuth.cos())
    }
}

impl Shape for Helmet {
    fn length(&self, root: &Root) -> f32 {
        // A card only where the rim is broken: everywhere else the shell's own
        // edge is the hair's edge, and a lock at every meridian is a hem.
        if !Self::breaks(Self::azimuth(root)) {
            return 0.0;
        }
        RIM_ROOT + RIM_REACH * (0.55 + 0.45 * self.cut.length.clamp(0.0, 1.0))
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        let azimuth = Self::azimuth(root);
        let walked = walk(&self.regions, azimuth, &self.shell);
        let last = walked.len() - 1;
        let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
        let normal = facing(&walked, last, azimuth);
        let thick = head_radius(&self.regions) * self.shell.rim;
        // Clear of the solid it breaks: past the shell's own outer surface at
        // the rim, by the same tolerance a card keeps off a skull.
        let lift = normal * (STAND + thick + RIM_LIFT);
        let travel = self.length(root) * along.clamp(0.0, 1.0);
        if travel <= RIM_ROOT {
            // Lying up the shell toward the crown, on the surface itself.
            return back_along(&walked, RIM_ROOT - travel) + lift;
        }
        // Then hanging, leaning out as it goes: hair leaving a mass does not
        // fall dead against it.
        let hang = travel - RIM_ROOT;
        walked[last] + lift + Vec3::NEG_Y * hang + out * (hang * RIM_LEAN)
    }

    fn width(&self, root: &Root) -> (f32, f32) {
        let coarse = 0.7 + 0.6 * self.cut.thickness.clamp(0.0, 1.0);
        let half = RIM_WIDTH * 0.5 * coarse * root.weight.clamp(0.0, 1.0).sqrt().max(0.4);
        (half, half * RIM_TAPER)
    }

    fn width_at(&self, root: &Root, along: f32) -> f32 {
        // **Feathered where it lies on the shell and tapered where it hangs**,
        // which are two different jobs on one card (the scalp's `width_at` says
        // the same about its fan and its taper). The lying part is what drew a
        // plate on the helmet at full width; the hanging part is the lock.
        let (base, tip) = self.width(root);
        let length = self.length(root);
        let travel = length * along.clamp(0.0, 1.0);
        let out = (travel / RIM_ROOT.max(f32::EPSILON)).clamp(0.0, 1.0);
        let emerging = RIM_FEATHER + (1.0 - RIM_FEATHER) * crate::face::smooth(out);
        let hang = ((travel - RIM_ROOT) / (length - RIM_ROOT).max(f32::EPSILON)).clamp(0.0, 1.0);
        (base + (tip - base) * hang) * emerging
    }

    fn across(&self, root: &Root) -> Vec3 {
        // Round the head, as every scalp card's width lies: a card running down
        // a meridian is wide along the parallel.
        let azimuth = Self::azimuth(root);
        Vec3::new(azimuth.cos(), 0.0, -azimuth.sin())
    }

    fn seating(&self) -> Seating {
        // A rim card's root is a MERIDIAN, as a scalp card's is: scattered by
        // area, the breaks land wherever the faces happen to be (#316).
        Seating::Meridians
    }

    fn shell(&self) -> Option<Shell> {
        Some(self.shell)
    }
}

/// How far a hanging rim card leans out per metre it drops.
///
/// Provenance: **tuned by render**.
const RIM_LEAN: f32 = 0.25;

/// What share of its width a rim card has where it lies on the shell, before it
/// reaches the rim.
///
/// **A card lying on a solid at full width is a plate stuck to it** (#345,
/// measured by lever): the first shell drew its rim cards from 18 mm above the
/// rim at full width, and that lying part read as a flat pale tab on the
/// helmet's forehead in BOTH renderers - the software one and Bevy - while only
/// the part hanging past the rim read as hair. Declining every rim card removed
/// the tabs and nothing else, which is what named them.
///
/// Feathered in instead, the card emerges from under the rim rather than lying
/// across it. It is the sideburn's lesson from #344 the other way up: a strip
/// that starts ON its line draws the line, and one that fades in from under it
/// does not.
///
/// Provenance: **derived** from the defect, **tuned by render**.
const RIM_FEATHER: f32 = 0.15;

// A shade on the lying part was tried here and REFUTED (#345). The premise was
// the scalp's own `SHADOW` (#339): that the part of a card lying against the
// mass reads light because it is lit, so darkening it would settle it into the
// rim. Isolated on a sheet at a quarter and at a half, it changed nothing a
// viewer sees - the pale tabs were unmoved at both - because the defect is
// geometric and not tonal: a flat card standing off a curved solid is a plate
// whatever shade it is painted. What removed it was cutting how far the card
// lies on the shell at all (see `RIM_ROOT`), so the constant is gone rather
// than kept at a value that buys nothing.

/// The head's own half-width, in metres: what everything sized in head radii is
/// a share of.
///
/// Measured at the middle of the vault rather than taken from the rig's node
/// radius, for the reason the whole hair system is measured: subdivision pulls
/// a head well inside its node radius, and a thickness taken from the plan is a
/// thickness that suits no built head.
fn head_radius(head: &Follicles) -> f32 {
    let skull = head.skull();
    let (throat, crown) = skull.throat_and_crown();
    skull.half_width(throat + (crown - throat) * 0.72)
}

/// Which way the walked surface faces at one of its points.
///
/// Across the meridian's own heading and the parallel, turned to face away from
/// the head's axis. At the crown the heading is already horizontal, so this is
/// the crown's up - which is what a card lying there needs (#316's lesson about
/// lifting along the surface a card is on rather than along its root's normal).
fn facing(walked: &[Vec3], at: usize, azimuth: f32) -> Vec3 {
    if at == 0 {
        return Vec3::Y;
    }
    let before = walked[at.saturating_sub(4)];
    let after = walked[(at + 4).min(walked.len() - 1)];
    let down = (after - before).normalize_or(Vec3::NEG_Y);
    let side = Vec3::new(azimuth.cos(), 0.0, -azimuth.sin());
    let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
    let normal = side.cross(down).normalize_or(out);
    if normal.dot(out) < 0.0 {
        -normal
    } else {
        normal
    }
}

/// One column of the shell: the meridian at `azimuth`, walked densely from the
/// crown to the rim, draped.
///
/// **Draped, which is the scalp walk's own rule**: below the widest band a
/// meridian passes, the head falls away and hair does not follow it in. Without
/// it the straight-back column of a default head ends 40 mm inside the neck.
///
/// Dense - a point every [`WALK`] metres - because the rows are chosen from it
/// by curvature rather than by index.
fn walk(head: &Follicles, azimuth: f32, shell: &Shell) -> Vec<Vec3> {
    let skull = head.skull();
    let (throat, crown) = skull.throat_and_crown();
    let radius = |height: f32| {
        let at = skull.surface_at(height, azimuth);
        (at.x * at.x + at.z * at.z).sqrt()
    };
    // How far this azimuth's rim is cut back from the mask's own hairline: the
    // fringe notch at the front and the nape line behind, carried between by
    // the azimuth's own cosine, as every landmark on a head is.
    let facing = azimuth.cos();
    let cut = shell.fringe * facing.max(0.0) + shell.nape * (-facing).max(0.0);
    let mut walked = vec![skull.surface_at(crown, azimuth)];
    let mut crest = crown;
    let mut height = crown;
    let mut begun = false;
    let mut past = 0.0f32;
    while height > throat {
        height -= WALK;
        // Held out by whatever it has draped over: the widest radius this
        // meridian has passed, re-read at the height it was passed.
        let here = radius(height);
        let over = radius(crest);
        if here >= over {
            crest = height;
        }
        let held = here.max(over);
        let point = Vec3::new(held * azimuth.sin(), height, held * azimuth.cos());
        let grows = head.weight(Follicle::Scalp, skull.surface_at(height, azimuth)) >= EDGE;
        begun |= grows;
        if begun && !grows {
            // Past the hairline the walk runs on only as far as a rim hanging
            // below it asks for.
            past += WALK;
            if past > (-cut).max(0.0) {
                break;
            }
        } else if begun && cut > 0.0 {
            // A rim cut back from the hairline stops that far short of it.
            let ahead = head.weight(Follicle::Scalp, skull.surface_at(height - cut, azimuth));
            if ahead < EDGE {
                walked.push(point);
                break;
            }
        }
        walked.push(point);
    }
    walked
}

/// Where a dense walk is `back` metres before its end.
fn back_along(walk: &[Vec3], back: f32) -> Vec3 {
    let mut left = back;
    for pair in walk.windows(2).rev() {
        let leg = pair[0].distance(pair[1]);
        if leg >= left {
            let share = if leg > f32::EPSILON { left / leg } else { 0.0 };
            return pair[1].lerp(pair[0], share);
        }
        left -= leg;
    }
    walk[0]
}

/// Where the shell's rows sit, as shares of each column's own arc: exactly
/// `rows` of them, the same schedule for every column.
///
/// **Placed where the columns BEND, and SHARED between them, and the second
/// half is as hard-won as the first** (#345). Placed evenly, the back of a
/// head, which is twice the arc of the front, leaves its row polyline straying
/// up to 3.1 mm inside the head while the front has rows to spare. Placed per column, each
/// column puts its rows at its own heights, so a quad spans two rows that are
/// not level with each other and the interpolated normals zig-zag: the sheet
/// showed chevron bands down the flank of the first shell built this way.
///
/// So the schedule is chosen once, against the WORST column at every step: the
/// gap whose split buys the most for whichever column is straying furthest.
/// Every column is then sampled at those same shares of its own arc, which
/// keeps the grid level and still spends the rows where the heads bend.
fn schedule(walks: &[Vec<Vec3>], rows: usize) -> Vec<f32> {
    let arcs: Vec<Vec<f32>> = walks.iter().map(|walk| along(walk)).collect();
    let mut chosen = vec![0.0f32, 1.0];
    while chosen.len() < rows {
        let mut worst = (0.0f32, 0usize, 0.5f32);
        for gap in 0..chosen.len() - 1 {
            let (from, to) = (chosen[gap], chosen[gap + 1]);
            for (walk, arc) in walks.iter().zip(&arcs) {
                let (one, two) = (at_share(walk, arc, from), at_share(walk, arc, to));
                let run = two - one;
                let length = run.length_squared();
                // Where this chord leaves its own column furthest behind, as a
                // share: the point a row would be worth spending here.
                let steps = 24;
                for step in 1..steps {
                    let share = from + (to - from) * step as f32 / steps as f32;
                    let point = at_share(walk, arc, share);
                    let along = if length > f32::EPSILON {
                        ((point - one).dot(run) / length).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let stray = point.distance(one + run * along);
                    if stray > worst.0 {
                        worst = (stray, gap, share);
                    }
                }
            }
        }
        if worst.0 <= 0.0 {
            break;
        }
        chosen.insert(worst.1 + 1, worst.2);
    }
    chosen
}

/// The arc length to each point of a walk.
fn along(walk: &[Vec3]) -> Vec<f32> {
    let mut arc = Vec::with_capacity(walk.len());
    let mut total = 0.0;
    arc.push(0.0);
    for step in walk.windows(2) {
        total += step[0].distance(step[1]);
        arc.push(total);
    }
    arc
}

/// Where a walk is a `share` of the way along its own arc.
fn at_share(walk: &[Vec3], arc: &[f32], share: f32) -> Vec3 {
    let total = *arc.last().unwrap_or(&0.0);
    if total <= f32::EPSILON {
        return walk[0];
    }
    let want = total * share.clamp(0.0, 1.0);
    let at = arc.partition_point(|reached| *reached < want).max(1);
    let (before, after) = (at - 1, at.min(walk.len() - 1));
    let span = arc[after] - arc[before];
    let blend = if span > f32::EPSILON {
        (want - arc[before]) / span
    } else {
        0.0
    };
    walk[before].lerp(walk[after], blend)
}

/// The shell's grid: the inner and outer point of every row of every column,
/// and the way the surface faces there.
struct Grid {
    /// Column-major: `COLUMNS * ROWS` of each.
    inner: Vec<Vec3>,
    outer: Vec<Vec3>,
    normals: Vec<Vec3>,
}

impl Grid {
    /// Where one row of one column sits in the grid, wrapping round the head.
    fn at(column: usize, row: usize) -> usize {
        (column % COLUMNS) * ROWS + row
    }

    /// Lofts the grid over a measured head.
    fn of(head: &Follicles, shell: &Shell) -> Self {
        let radius = head_radius(head);
        let (crown, rim) = (radius * shell.crown, radius * shell.rim);
        // Every column walked first, because the rows are scheduled against all
        // of them at once: see [`schedule`] for the chevrons a per-column
        // schedule draws.
        let walks: Vec<Vec<Vec3>> = (0..COLUMNS)
            .map(|column| walk(head, TAU * column as f32 / COLUMNS as f32, shell))
            .collect();
        let rows = schedule(&walks, ROWS);
        let walked: Vec<Vec3> = walks
            .iter()
            .flat_map(|walk| {
                let arc = along(walk);
                rows.iter()
                    .map(move |share| at_share(walk, &arc, *share))
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut grid = Self {
            inner: Vec::with_capacity(COLUMNS * ROWS),
            outer: Vec::with_capacity(COLUMNS * ROWS),
            normals: Vec::with_capacity(COLUMNS * ROWS),
        };
        for column in 0..COLUMNS {
            for row in 0..ROWS {
                let point = walked[Self::at(column, row)];
                // **The surface's normal read off the GRID, not off one
                // column's own walk.** A normal taken from the walk's
                // neighbours a few millimetres away follows every wobble the
                // drape puts in a single meridian; the surface the shell
                // actually has is the one its own rows and columns describe,
                // and that is what a quad is shaded by.
                let up = walked[Self::at(column, row.saturating_sub(1))];
                let down = walked[Self::at(column, (row + 1).min(ROWS - 1))];
                let left = walked[Self::at((column + COLUMNS - 1) % COLUMNS, row)];
                let right = walked[Self::at(column + 1, row)];
                let azimuth = TAU * column as f32 / COLUMNS as f32;
                let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
                let mut normal = (right - left).cross(down - up).normalize_or(Vec3::Y);
                if normal.dot(out) < 0.0 && row > 0 {
                    normal = -normal;
                }
                // The crown is a pole: every column meets there and the surface
                // faces the sky, whatever the columns' own directions say.
                if row == 0 {
                    normal = Vec3::Y;
                }
                let share = row as f32 / (ROWS - 1) as f32;
                let thick = rim + (crown - rim) * (1.0 - share).powf(THICKNESS_POW);
                grid.inner.push(point + normal * STAND);
                grid.outer.push(point + normal * (STAND + thick));
                grid.normals.push(normal);
            }
        }
        grid
    }
}

/// Draws one shell into `into`, bound rigidly to `head`, and returns what it
/// cost in triangles.
///
/// **Rigid to the head joint**: a scalp shell moves with nothing else, which is
/// what every scalp card's tip already does. A facial shell hands its binding
/// over as a beard's cards do, and that is #349's.
pub(super) fn loft(
    into: &mut PolyMesh,
    regions: &Follicles,
    shell: &Shell,
    head: u16,
    roots: Vec3,
    tips: Vec3,
) -> usize {
    let grid = Grid::of(regions, shell);
    let before = into.faces.len();
    let first = into.positions.len() as u32;
    let (lane_from, lane_to) = StrandMask::lane_span(mask::lane_of(grid.outer[0]));
    let uv = Vec2::new((lane_from + lane_to) * 0.5, SHELL_ROW);
    let mut skin = VertexSkin::default();
    skin[0] = crate::rig::Influence {
        joint: head,
        weight: 1.0,
    };
    let under = (roots * UNDER_SHADE).clamp(Vec3::ZERO, Vec3::ONE);
    // **The crown is a POLE and it is welded** (the discarded shell's own
    // lesson, #69): every column's first row is the same walked point, and left
    // as a ring of coincident vertices the top of the shell is a ring of edges
    // belonging to one face each - a solid that is not closed, which is 72 of
    // its 1,764 edges on a default head, measured. One apex a surface, and the
    // rows below it are a fan.
    let apex = |points: &[Vec3]| {
        let sum: Vec3 = (0..COLUMNS).map(|column| points[Grid::at(column, 0)]).sum();
        sum / COLUMNS as f32
    };
    let (apex_out, apex_in) = (apex(&grid.outer), apex(&grid.inner));
    // Four blocks - the outer apex and its rows, the inner apex and its rows,
    // the rim's bevel ring - so a face's corners are arithmetic rather than a
    // lookup.
    {
        let mut push = |at: Vec3, normal: Vec3, colour: Vec3| {
            into.positions.push(at);
            into.normals.push(normal);
            into.uvs.push(uv);
            into.colours.push(colour);
            into.skin.push(skin);
        };
        push(apex_out, Vec3::Y, tips);
        for column in 0..COLUMNS {
            for row in 1..ROWS {
                let at = Grid::at(column, row);
                // The tips' colour at the crown falling to the roots' at the
                // rim, which is the way round a mass of hair is lit: the rim is
                // the part in its own shadow.
                let down = row as f32 / (ROWS - 1) as f32;
                push(grid.outer[at], grid.normals[at], tips.lerp(roots, down));
            }
        }
        push(apex_in, Vec3::NEG_Y, under);
        for column in 0..COLUMNS {
            for row in 1..ROWS {
                let at = Grid::at(column, row);
                push(grid.inner[at], -grid.normals[at], under);
            }
        }
        for column in 0..COLUMNS {
            let at = Grid::at(column, ROWS - 1);
            let (outer, inner) = (grid.outer[at], grid.inner[at]);
            let above = grid.outer[Grid::at(column, ROWS - 2)];
            let heading = (outer - above).normalize_or(Vec3::NEG_Y);
            let across = (outer - inner).normalize_or(grid.normals[at]);
            let thick = outer.distance(inner);
            push(
                (outer + inner) * 0.5 + heading * (thick * BEVEL),
                (across + heading).normalize_or(grid.normals[at]),
                roots,
            );
        }
    }
    let rows = (ROWS - 1) as u32;
    let outer_apex = first;
    let inner_apex = first + 1 + COLUMNS as u32 * rows;
    let outer =
        |column: usize, row: usize| first + 1 + (column % COLUMNS) as u32 * rows + (row as u32 - 1);
    let inner = |column: usize, row: usize| {
        inner_apex + 1 + (column % COLUMNS) as u32 * rows + (row as u32 - 1)
    };
    let bevel = |column: usize| inner_apex + 1 + COLUMNS as u32 * rows + (column % COLUMNS) as u32;
    for column in 0..COLUMNS {
        let next = column + 1;
        // The crown's fan, wound as the quads below it are.
        into.faces
            .push(vec![outer_apex, outer(column, 1), outer(next, 1)]);
        into.faces
            .push(vec![inner_apex, inner(next, 1), inner(column, 1)]);
        for row in 1..ROWS - 1 {
            into.faces.push(vec![
                outer(column, row),
                outer(column, row + 1),
                outer(next, row + 1),
                outer(next, row),
            ]);
            // Wound the other way: the inner surface faces the head.
            into.faces.push(vec![
                inner(column, row),
                inner(next, row),
                inner(next, row + 1),
                inner(column, row + 1),
            ]);
        }
        // The rim, closed through the bevel ring and wound out of the solid.
        into.faces.push(vec![
            outer(column, ROWS - 1),
            bevel(column),
            bevel(next),
            outer(next, ROWS - 1),
        ]);
        into.faces.push(vec![
            bevel(column),
            inner(column, ROWS - 1),
            inner(next, ROWS - 1),
            bevel(next),
        ]);
    }
    into.faces[before..].iter().map(|face| face.len() - 2).sum()
}
