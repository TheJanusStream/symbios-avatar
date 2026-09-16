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
//! bevelled band at the rim. The count is the grid's and not a record's: a
//! shell costs the same whatever head or cut it is grown for, which is what
//! made the discarded one affordable, why nothing here is a wire field, and why
//! the hair ceiling's tier cannot squeeze one - it roots fewer CARDS (#347). A
//! faceted solid splits its corners and costs vertices for it, and ships none
//! that no face references (#351).
//!
//! A head twelve metres off draws the same solid on [`FAR_COLUMNS`] by
//! [`FAR_ROWS`] ([`Shell::far`], #350): 468 triangles where the near grid is
//! 1,656, which is what lets a card style's helmet twin cost less than its
//! cards - on the near grid it costs more.

pub mod face;

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::clump::{Lump, Root, Seating, Shape};
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

/// How many columns the far tier's shell is swept with (#350).
///
/// **The owner's decision, on the cost table**: as committed, a helmet twin
/// costs more than the cards it stands for at an ordinary cut (196 to 1,346
/// triangles more over three heads), and the grid is what a far tier has to
/// spend less of. At twelve metres a head is about thirty pixels tall and a
/// column of the near grid is under half of one.
///
/// Eighteen by seven is 468 triangles and about 250 vertices of shell, against
/// 1,656 and 830. **What a coarse grid breaks is not the vertices but the
/// chords between them**: every vertex of every far solid stays outside the
/// skin at every grid tried, but a quad spanning two rows sags into the head
/// between them, and where two neighbouring columns differ in length - the
/// face notch - it sags furthest. Read on points spread over every
/// OUTWARD-facing face, on three heads and every scalp style: 36 x 12 dips at
/// most 0.15 mm under the skin, 24 x 8 1.0, 18 x 8 2.1, 18 x 7 2.8, 18 x 6
/// 7.1, 20 x 7 6.9 and 16 x 7 8.6 - so the column count matters as much as the
/// row count, by where the columns fall against the notch. At the far framing
/// the three 18-column grids render alike; at the near one 18 x 6 shows a fleck
/// of skin at the temple on seed 7 and 18 x 7 does not.
///
/// Provenance: **measured** (the chord sag above), **judged by render** at the
/// far framing and at the near one.
pub const FAR_COLUMNS: usize = 18;

/// How many rows the far tier's shell is lofted with, crown to rim. See
/// [`FAR_COLUMNS`] for why seven.
///
/// Provenance: **measured**, **judged by render**.
pub const FAR_ROWS: usize = 7;

/// How far off the sagittal line a band's cut has not begun at all, in the sine
/// of the azimuth: the strip's own half-width.
///
/// **A ramp that starts at the line itself leaves a strip ONE COLUMN wide**
/// (#347, rendered and then measured). The sine is zero dead ahead, so the
/// midline column is uncut and its neighbour ten degrees away already takes 50
/// of the 80 mm - the default head's rim goes from 110 mm of arc to 47 over one
/// column, and the shell comes out as a triangle from the forehead to the pole
/// rather than as a strip running front to back. A crest's strip is a few
/// columns wide and every one of them is FULL length.
///
/// Nought point two six is a sine of 0.26, which is 15 degrees off the line:
/// with [`BAND_OVER`] at 0.42 the columns at the midline and either side of it
/// keep their whole column, the next pair is part cut, and everything past 25
/// degrees is shaved.
///
/// Provenance: **derived** from the measured column, **tuned by render**.
const BAND_INSIDE: f32 = 0.26;

/// How far off the sagittal line a band's cut comes fully down, in the sine of
/// the azimuth.
///
/// **A bare sine is an HOURGLASS and not a strip** (#347, rendered). The sine
/// is zero dead ahead AND dead behind, so a cut weighted by it alone leaves the
/// front and the nape columns their whole length and pinches only the sides:
/// the shell came out as a swept mass over the crown running down to a point
/// over the forehead, a quiff rather than a crest, in both renderers. A crest
/// is a strip of a few degrees either side of the midline with everything else
/// shaved, so the cut has to be FULL a little way off the line rather than
/// creeping up to it.
///
/// Nought point four two is a sine of 0.42, 25 degrees off the midline, which
/// with [`BAND_INSIDE`] at 0.26 gives a strip of three columns at full length
/// with one part-cut column either side of it.
///
/// Provenance: **derived** from the defect, **tuned by render**.
const BAND_OVER: f32 = 0.42;

/// How sharply a fin's thickness falls away from the sagittal line, as the
/// power its cosine is raised to.
///
/// **Two, as [`RISE_ROUND`] is, and for the same reason**: at one the cosine
/// still carries a third of the fin at the temple, which is a thick shell with
/// a bulge rather than a fin on a shaved head. Squared it is down to a ninth
/// there and the mass is on the strip the band left.
///
/// Provenance: **derived** from the cosine, with [`RISE_ROUND`].
const FIN_ROUND: f32 = 2.0;

/// How sharply a fin falls away DOWN its own column, likewise.
///
/// A fin is tallest at the crown and gone by the rim, which is where the band
/// has already taken the shell away - so this only has to reach the few rows a
/// banded shell has left. One and a half rather than two: squared, the fin was
/// already half gone by the second row and read as a bump at the pole.
///
/// Provenance: **tuned by render**.
const FIN_POW: f32 = 1.5;

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

/// Where down a front column a pompadour's rise peaks, as a share of the arc.
///
/// **Not at the pole.** A rise carried at the crown draws a cone; the mass a
/// slicked-back head has is the one it sweeps back off the forehead, which sits
/// between the hairline and the vault. Three fifths of the way down from the
/// crown is over the front of the skull on all three measured heads.
///
/// Provenance: **tuned by render**.
const RISE_AT: f32 = 0.6;

/// How much of the column the rise is spread over either side of [`RISE_AT`].
///
/// Wide enough that the front rim itself still carries some of it - a rise that
/// has fallen to nothing by the hairline leaves a step where the two meet.
///
/// Provenance: **tuned by render**.
const RISE_OVER: f32 = 0.55;

/// How fast the rise falls away round the head from dead ahead.
///
/// Squared rather than linear, so it is a FRONT and not a whole hemisphere: at
/// the temple's azimuth a linear fall still carries two thirds of it, which
/// reads as a head one size too big rather than as a sweep.
///
/// Provenance: **tuned by render**.
const RISE_ROUND: f32 = 2.0;

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

/// Over how many radians past the face's own corner a hem comes down.
///
/// A fifth of a radian is about one lock of the rim's sixteen, so the hem
/// reaches its full drop within a card of leaving the face rather than plunging
/// at one meridian - which is a parting, and a cliff is a cut.
///
/// Provenance: **derived** from the rim's own card spacing, **tuned by render**.
const NOTCH_OVER: f32 = 0.35;

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
    /// How far the rim is carried past the mask's own hairline at the SIDES, in
    /// metres, negative like [`Shell::fringe`]'s other sign.
    ///
    /// **The third azimuth, and a bell needs it** (#346). `fringe` and `nape`
    /// reach the front and the back, weighted by the azimuth's own cosine, so
    /// between them they leave the sides alone - and the sides are exactly where
    /// a bob hangs. Measured on three heads: the side of a default head has to
    /// run 83 mm past its hairline to reach the gonion, and the drape holds it
    /// out at the widest radius the whole way, so the shell stands 3.7 mm off
    /// the skin at the hairline and 10.0 mm off at 60 mm past it. That standing
    /// off IS the bell.
    pub side: f32,
    /// How much thickness the front of the crown carries over the rest, as a
    /// share of the head's own half-width: a pompadour.
    ///
    /// **A thickness that varies by AZIMUTH, which the prototype had no way to
    /// ask for** (#346). [`Shell::crown`] and [`Shell::rim`] describe the
    /// thickness down a column, the same for every column; a slicked-back head
    /// is the one shape in this first catalogue whose mass is not even round the
    /// head, so it needs a term that is.
    ///
    /// Safe against the face box by construction rather than by tolerance: the
    /// rise thickens the shell along the surface's own normal, and at the front
    /// that normal is above the brow by 16 to 58 mm on the three measured heads
    /// (and further still once the rim is swept back), so a front that grows
    /// forward grows forward over the forehead and never into
    /// [`Follicles::clearance`](super::follicle::Follicles::clearance).
    pub rise: f32,
    /// How far the row schedule is pulled back toward EVEN, `0` placing rows
    /// where the columns bend and `1` at equal shares of every column's arc.
    ///
    /// **A faceted shell shows the schedule, and a smooth one hides it**
    /// (#346, measured). #345 places rows by curvature, against the worst
    /// column, because an even schedule leaves the back of a head straying 3.1 mm
    /// inside itself - and on a smooth shell that spacing is invisible, since
    /// the normals are read off the grid and averaged across the rows. Give
    /// every face its own normal and it is the only thing you can see: the
    /// default head's ten outer rows come out 1.7, 1.0, 1.4, 2.7, 1.5, 2.3,
    /// 3.1, 2.8, 9.9 and 18.2 mm tall, so the shading step between them runs
    /// 10.3, 8.9, 7.7, 10.0, 6.8, 7.1, 8.2, 8.8 and 7.9 degrees - up and down
    /// rather than along - and the cap reads as a QUILT rather than as a crop.
    /// The quads' own warp is 0.09 to 0.35 mm, so it is the spacing and not the
    /// flatness.
    ///
    /// So a faceted style buys its even facets back at the price of some stray,
    /// and says how much. A smooth style leaves this at zero and is what #345
    /// built, point for point.
    pub even: f32,
    /// How far the rim is cut back weighted by the azimuth's own SINE, with no
    /// ramp, in metres: what shrinks the shell to a band along the sagittal
    /// line.
    ///
    /// **[`Shell::side`] cannot do this, and its own numbers are why** (#347,
    /// measured on three heads). That term is held off by the notch ramp until
    /// the column is clear of the face box's temple corner - 0.88 rad on all
    /// three - so at every value from 20 mm to 200 the default head's columns
    /// from dead ahead to 50 degrees off it are untouched: 110, 110, 112, 113,
    /// 112, 111 mm of arc, the same to the millimetre at 200 mm as at zero. A
    /// shell cut by `side` as far as it will go is a bowl over the whole front
    /// quadrant with its back and sides shaved - a monk's fringe, not a crest.
    /// The ramp cannot be taken out of `side`, because keeping a hem off the
    /// face is the thing it is there for.
    ///
    /// So this is the same weighting without it. Safe against the face by
    /// construction rather than by tolerance, for a reason `side` does not
    /// have: this cut only ever moves a rim UP the column, and the front rim of
    /// the three measured heads already sits 16 to 58 mm above the brow (#346)
    /// - the walk's own face-box break is still there and is still never asked.
    pub band: f32,
    /// How much thickness the MIDLINE carries over the rest, as a share of the
    /// head's own half-width, peaked at the crown: a fin.
    ///
    /// **Neither [`Shell::crown`] nor [`Shell::rise`] is this** (#347). `crown`
    /// is the same thickness for every column, and every column's first row is
    /// the welded pole, so raising it raises the whole apex: a thicker dome.
    /// `rise` does vary by azimuth, but it is weighted `cos(az).max(0)` and
    /// peaked 0.6 of the way down the FRONT column, which is a pompadour over
    /// the forehead - a fin runs front to back and is tallest at the crown.
    ///
    /// Measured on the default head: 0.20 head radii is 14.4 mm of fin over the
    /// crown, 0.35 is 25.2 and 0.50 is 36.0.
    pub fin: f32,
    /// How long the outer surface's roll IN to the rim is, as a multiple of the
    /// gap it closes there: `1` a quarter circle from the thickness the column
    /// carries down to the catalogue's thin edge at the rim itself, more a
    /// gentler slope, `0` no roll.
    ///
    /// **What an afro needs and neither `crown` nor `rim` can give** (#348).
    /// Squared from a thick crown to a thin rim is a cone with thin sides; a
    /// thick rim is a slab, and the slab the bevel ring closes is the edge that
    /// reads as a helmet (#345's veto point (a)). A mass of hair that comes down
    /// the head at full thickness and then turns in to meet it has no edge to
    /// see from any side.
    ///
    /// Down to the thin edge and not to nothing, because a rim of no thickness
    /// puts the outer row, the bevel ring and the inner row at one position, and
    /// a solid welded by position there is not closed. Measured, the roll has at
    /// least 3.0 mm of room past the shell's inner surface on all three heads
    /// (seed 7 dead ahead), 5.0 and 5.75 on the other two, and 9 to 18 mm at the
    /// nape, where the drape holds the column off the neck - so a roll that
    /// ends at the shell's own inner surface cannot reach the skin.
    pub roll: f32,
    /// How far the outer surface is placed on a soft ELLIPSOID fitted round the
    /// head rather than stood off along the walk's own normal, `0` not at all
    /// and `1` wholly: an afro's round mass.
    ///
    /// **What a thick mass needs and a thickness cannot give** (#348,
    /// measured). Stood off along the walk's normal, an afro 0.8 head radii
    /// thick FOLDED where its sides come down: 16 to 26 edges of the built
    /// solid creased past 107 degrees, up to 175, all at 0.9 to 1.3 radians
    /// round the head - the notch ramp, where one column is tens of millimetres
    /// longer than its neighbour, so a row at the same share of each sits at a
    /// different height and a 58 mm offset turns that shear into a fold. A bell
    /// at the same ramp, 1.2 mm thick, has none. Along the ray from the
    /// ellipsoid's own middle nothing can fold however the rows are sheared,
    /// and the surface is round, which is what the issue asks an afro to be.
    ///
    /// The ellipsoid is stood off the head by [`Shell::crown`]'s thickness.
    pub round: f32,
    /// How many cornrow ridges run down the head, round the whole of it: `0`
    /// draws none.
    ///
    /// **The same mechanism as [`Shell::rise`] and [`Shell::fin`]** - a
    /// thickness that varies by azimuth - with a term that is PERIODIC, which
    /// neither of those is: rise is weighted `cos.max(0)` and fin `cos^2`
    /// (#348). See [`Shell::ridge`] for how tall.
    pub ridges: usize,
    /// How tall a cornrow ridge stands over its partings, as a share of the
    /// head's own half-width.
    pub ridge: f32,
    /// How the solid is coloured from the record's roots and tips.
    ///
    /// A catalogue constant, like [`Shell::facets`]: what a named style IS.
    pub tone: Tone,
    /// How its normals are read: one continuous surface, one per face, or
    /// smooth over the vault and faceted over the rim band.
    ///
    /// A catalogue constant and never a record axis (the owner's decision):
    /// faceted, painterly or smooth is what a named style IS.
    pub facets: Facets,
    /// How many columns the solid is swept with, round the whole head:
    /// [`COLUMNS`] for every style in the catalogue.
    ///
    /// **The far tier's knob, and nothing else's** (#350, the owner's
    /// decision). A shell is the same grid whatever it is asked, which is the
    /// whole of why the helmet twin of a card style costs MORE than its cards
    /// at an ordinary cut - measured, 196 to 1,346 triangles more on three
    /// heads. [`Shell::far`] is what a head twelve metres off draws, and the
    /// grid is what it spends less of. A coarser grid for a NEAR shell was
    /// refuted at #346 and is not what this is for.
    pub columns: usize,
    /// How many rows each column is lofted with, crown to rim: [`ROWS`] for
    /// every style in the catalogue. See [`Shell::columns`].
    pub rows: usize,
}

/// How a shell is coloured from the record's roots and tips.
///
/// **A style's own and not the record's** (#348). Every shell was drawn with
/// the tips' colour at the crown falling to the roots' at the rim, baked into
/// the loft; an afro wants none of that, since a gradient over one round mass
/// of coils reads as a highlight painted on. A default on a published record
/// field would move nothing an owner sees, and the loft is handed a [`Shell`]
/// and nothing else, so the choice lives here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    /// The tips' colour at the crown falling to the roots' at the rim.
    #[default]
    Crown,
    /// One tone, the tips' - the outside of a mass of coils is its ends - with
    /// only the underside darker.
    One,
}

/// How a shell's normals are read.
///
/// **What a named style IS, rather than a number on the wire** (#338's owner
/// decision). The three are the three art targets the helmet family was briefed
/// with: smooth, faceted, and painterly - which turned out to mean a smooth
/// vault with a faceted band at the hem, the way a painted bob reads as one
/// mass with a cut edge.
///
/// Measured before it was built (#346): at [`COLUMNS`] the surface turns ten
/// degrees a column, and the crown's fan turns 1.3 degrees a face on a default
/// head and 3.3 on seed 7 - so the break a facet shows is a shading step of ten
/// degrees round the head, and the silhouette barely moves (the sagitta a facet
/// cuts off the widest parallel is 0.2 to 0.3 mm). A facet here is a SHADE, not
/// an outline, which is what a low-poly crop reads as at the distance a body is
/// judged from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Facets {
    /// Normals read off the grid: one continuous surface.
    #[default]
    Smooth,
    /// Every face carries its own normal.
    All,
    /// One continuous surface, with the OUTER surface's normals read off the
    /// outer surface's own grid rather than copied from the walk's.
    ///
    /// **A thickness that varies is invisible to [`Facets::Smooth`]** (#348).
    /// That mode reads a normal off the walked grid and gives it to both
    /// surfaces, so the outer surface is shaded as the head is: a rolled rim
    /// shades as if it did not roll and a cornrow ridge does not shade at all.
    /// A faceted shell never had the problem, since its normals are taken off
    /// the positions (which is why the crest's fin reads). Kept separate from
    /// `Smooth` so every style built before it draws point for point as it
    /// did.
    Relief,
}

// A third mode - smooth over the vault and FACETED OVER THE RIM BAND, the
// "painterly" art target the bell was briefed with - was built and REFUTED
// (#346). Isolated on its own sheet over the two rows above the hem and the
// bevel ring that closes them, it drew a pale flat plate across the forehead at
// the face notch and a banded ring round the hem, in both renderers: the same
// defect as #345's pale tabs and for the same reason, which is that a flat band
// lying across a curved solid reads as a plate stuck to it however it is shaded.
// The bell is smooth, and the mode is gone rather than kept at a value that
// buys nothing - which is #345's veto point (f) the other way round.

impl Shell {
    /// The same solid as a head twelve metres off draws it (#350): on the far
    /// grid, smooth, with no cornrow ridges.
    ///
    /// **Smooth, because a facet is below a pixel there and costs a vertex
    /// per corner of every face** - a faceted cap is 4,280 vertices where a
    /// smooth one of the same grid is under a thousand (measured, #350; 3,450
    /// since #351 stopped shipping the 830 the split left behind). **No
    /// ridges**, because ten ridges need twenty columns to sample at all (see
    /// `ridged`) and a cornrow is seven millimetres tall, half a pixel at that
    /// distance. Everything else - the thickness, the cuts, the roll, the round
    /// mass, the fin - is the style's own, so the far solid is the near one's
    /// silhouette on fewer points.
    #[must_use]
    pub fn far(self) -> Self {
        Self {
            columns: FAR_COLUMNS,
            rows: FAR_ROWS,
            facets: match self.facets {
                Facets::All => Facets::Smooth,
                smooth => smooth,
            },
            ridges: 0,
            ridge: 0.0,
            ..self
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            crown: THICKNESS[0],
            rim: THICKNESS[1],
            fringe: 0.0,
            nape: 0.0,
            side: 0.0,
            rise: 0.0,
            band: 0.0,
            fin: 0.0,
            even: 0.0,
            roll: 0.0,
            round: 0.0,
            ridges: 0,
            ridge: 0.0,
            tone: Tone::Crown,
            facets: Facets::Smooth,
            columns: COLUMNS,
            rows: ROWS,
        }
    }
}

/// One helmet style's whole description: the solid, the cards that break its
/// rim, and the ball it hangs off the shell if it has one.
///
/// **Not a thing on the wire.** A record asks for a named
/// [`ScalpStyle`], and `ScalpStyle::helmet` is the one place that turns a name
/// and its axis into one of these (#346 removed the `AvatarConfig::helmet` door
/// #345 sheeted the prototype through). The catalogue's constructors -
/// [`Cap::crop`], [`Cap::slicked`], [`Cap::bell`], [`Cap::bun`], [`Cap::crest`],
/// [`Cap::afro`] and [`Cap::braids`] - are what the styles are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cap {
    /// The solid it wears.
    pub shell: Shell,
    /// How many meridians its rim cards are seated on, of which the ones inside
    /// [`Cap::breaks`] grow. Zero grows none, and the solid is still the hair.
    /// See [`Helmet::length`].
    pub rim_cards: usize,
    /// Where the rim is BROKEN by cards, as the cosine window its azimuth must
    /// fall OUTSIDE: `[ahead, behind]`.
    ///
    /// **A style's own, because the two shapes want opposite windows** (#346). A
    /// crop breaks at the brow and the nape and keeps a clean arc over the ear
    /// (`RIM_AT`); a bell is a hem all the way round with a notch over the
    /// face, which is the same window turned inside out.
    pub breaks: [f32; 2],
    /// A closed solid this style hangs off the shell, if it has one: a bun.
    ///
    /// **The tied-back's own lump, moved and resized** (#342's `hair::Lump` and
    /// `Shape::lump`, 48 triangles, drawn in the roots' colour and counted with
    /// the region), rather than a second kind of solid - a bun IS that
    /// ellipsoid on a different seat. Where the seat is, and why it is not the
    /// knot's own, is [`Cap::bun`].
    pub lump: Option<Lump>,
}

impl Default for Cap {
    fn default() -> Self {
        Self {
            shell: Shell::default(),
            rim_cards: RIM_CARDS,
            breaks: RIM_AT,
            lump: None,
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

/// How big a bun is, as a share of the head's own half-width.
///
/// **In head radii and not in millimetres**, for [`BELL_DROP`]'s reason: the
/// three measured heads' half-widths are 72.1, 57.9 and 83.3 mm, so a bun cut
/// in millimetres is a knot on one head and a melon on another. Nought point
/// three five is 25 mm of radius on a default head, a 50 mm ball, which is what
/// a gathered bun of hair is; measured, every head clears its own body by 3.9
/// to 13.8 mm at the nape and 5.4 at the crown at this size.
///
/// Provenance: **derived** from the measured clearance, **tuned by render**.
const BUN_RADIUS: f32 = 0.35;

/// How much wider than tall a bun is.
///
/// A gathered bun is a little flattened against the head it is pinned to, which
/// is the axis the sphere is squashed on. The tied-back's own knot is
/// 14 x 11 x 9, which is the same idea at a sixth the size.
///
/// Provenance: **tuned by render**, **carried** from the knot's own proportions.
const BUN_SQUASH: [f32; 3] = [1.0, 0.86, 0.78];

/// How much of the roots' colour a bun keeps.
///
/// The knot's own [`super::style::scalp::LUMP_SHADE`] taken from there rather
/// than chosen again: a gathered mass of the same hair is the same colour, and
/// two shades chosen separately are two chances to disagree on one head.
const BUN_SHADE: f32 = super::style::scalp::LUMP_SHADE;

/// How far a bun sinks INTO the shell it is seated on, as a share of its own
/// radius.
///
/// **The seat is a contact, not a standoff, and the first cut got that the
/// wrong way round** (#347). The issue asks for the tied-back's own seating at
/// [`super::style::scalp::KNOT_STANDOFF`], and that constant is a share of a
/// DEPTH: `depth_behind(height) x 1.15`, a point pushed out past the back of
/// the head. Read off the built body it does not work for a bun at all - it
/// puts the axis's low end 0.41 mm off the default head's skin, 3.5 mm INSIDE
/// seed 42's and 24.7 mm inside seed 7's, and 22 to 44 mm inside the shell the
/// ball hangs off, because `depth_behind` is the SKULL's profile and the skull
/// stops describing the body at the throat, which is where a nape bun sits. The
/// knot gets away with it at 14 mm across with half of itself buried in a
/// gather. A bun is the style.
///
/// Applied as a standoff from the shell's own outer surface instead, the ball
/// came out DETACHED - a sphere hanging in the air behind the head at the nape
/// and floating over the crown at the top knot, in both renderers, which is
/// what named this. A bun is pinned INTO the hair it gathers, so the seat is
/// the shell's outer surface and the sphere sinks this share of itself into it.
///
/// What bounds it is the skin, not the shell: a ball resting on the middle of
/// the occiput swings its lower quadrant toward a head that is curving away
/// under it, so the worst point of the sphere is not the one it is seated at.
/// Measured on the BUILT ball rather than on an ideal one, and the guard
/// `a_bun_is_a_closed_ball_that_sits_on_its_shell` is what holds it.
///
/// Provenance: **derived** from the defect, **bounded by the measured skin**.
const BUN_EMBED: f32 = 0.20;

/// How far the crest cuts its rim back at the sides, in metres.
///
/// **What makes the shell a band**, measured against the column it eats
/// (#347): at 80 mm the default head's side column is down from 134 mm of arc
/// to 17, which is one row and is the floor a column cannot go below; at 60 mm
/// it keeps 65 mm and reads as short sides rather than as shaved ones. Eighty
/// is where all three heads have taken their sides to the floor while the
/// midline still has its whole column: 110 mm on the default head, 102 on seed
/// 42, 95 on seed 7 - untouched, because the band's weighting is the sine and
/// the sine is zero dead ahead.
///
/// Provenance: **derived** from the measured column, **tuned by render**.
const CREST_BAND: f32 = 0.080;

/// How tall the crest's fin is at its axis's top, as a share of the head's own
/// half-width.
///
/// Measured on the default head: 0.20 head radii is 14.4 mm over the crown,
/// 0.35 is 25.2 and 0.50 is 36.0 - and a fin is read against the head it is on,
/// which is why this is in radii like everything else the catalogue sizes.
///
/// Provenance: **tuned by render** against the measured height.
const CREST_FIN: [f32; 2] = [0.10, 0.50];

/// How thick an afro is at the crown at each end of its axis, as shares of the
/// head's own half-width.
///
/// Measured before it was built (#348): no even thickness from 10 to 80 mm put
/// a single outer point inside `Follicles::clearance` on any of three heads,
/// because at the front the surface faces up and forward and a thicker mass is
/// a higher one. So what bounds an afro is the look and not the face.
///
/// Provenance: **tuned by render**.
const AFRO_THICK: [f32; 2] = [0.15, 0.80];

/// How much of the crown's thickness an afro carries at its sides, before the
/// roll takes it in.
///
/// Provenance: **tuned by render**.
const AFRO_SIDES: f32 = 0.8;

/// How far an afro carries its rim past the hairline at the sides and the back,
/// at each end of its axis, as shares of the head's own half-width.
///
/// **A mass whose bottom is the hairline is a mushroom** (#348, rendered): the
/// first afro kept the scalp mask's own rim, which runs level across the front
/// and above the ear, so the rolled underside drew a flat shelf round the head
/// at the top of the ear in both the front and the side views. Close-cropped
/// coils stay at the hairline; a full crown comes down over the ears and to the
/// nape.
///
/// Provenance: **carried** from `BELL_DROP`'s measured drop to the gonion,
/// **tuned by render**.
const AFRO_DROP: [f32; 2] = [0.0, 0.35];

/// How long an afro's roll is against the gap it closes: `1` a quarter circle.
/// See [`Shell::roll`].
///
/// Provenance: **derived** (a quarter circle), **tuned by render**.
const AFRO_ROLL: f32 = 2.0;

/// How far an afro is placed on its ellipsoid at each end of its axis. See
/// [`Shell::round`].
///
/// **Close-cropped coils follow the head** (#348, rendered): wholly round at
/// the axis's low end, the ellipsoid fitted through a head's own bounds stood a
/// short crop far off the forehead and the temples, and the style read as a
/// bowler hat.
///
/// Provenance: **tuned by render**.
const AFRO_ROUND: [f32; 2] = [0.0, 1.0];

/// How many cornrow ridges braids have at each end of their axis.
///
/// **From the issue**: four to ten. Measured, that is one ridge every 9 to 3.6
/// columns, 108 to 43 mm apart at the middle of a default head's column.
const BRAID_RIDGES: [usize; 2] = [4, 10];

/// How thick a cornrow shell is under its ridges, crown and rim alike, as a share
/// of the head's own half-width.
///
/// **Cornrows lie ON the scalp** (#348, rendered): on the Cap's own crown of
/// 0.055 the ridges stood on a bowl and the style read as a pleated helmet. The
/// catalogue's thin rim edge all over, so the partings are the scalp's own
/// shape.
///
/// Provenance: **carried** from the thin edge, **tuned by render**.
const BRAID_THICK: f32 = THICKNESS[1];

/// How tall a cornrow ridge stands over its partings, as a share of the head's
/// own half-width: seven millimetres on a default head.
///
/// Provenance: **tuned by render**.
const BRAID_DEPTH: f32 = 0.06;

/// How far a [`ScalpStyle::Cap`]'s fringe notch
/// cuts the rim back over the brow at its axis's top, in metres.
///
/// **Measured against the column it eats** (#346): a front cut of 20 mm takes
/// the default head's rim from 34 to 53 mm above the brow and leaves 86 mm of
/// the 110 mm column; at 40 mm seed 7's front column collapses from 95 mm of arc
/// to 20, which is a cap with no front left rather than a deeper notch. Twenty
/// two millimetres is the most all three measured heads carry.
///
/// Provenance: **derived** from the measured column, **tuned by render**.
const CAP_NOTCH: f32 = 0.022;

/// How far a faceted crop pulls its row schedule back toward even.
///
/// **What the facets cost, and all of it** (#346): a faceted shell reads as a
/// quilt at zero, because every uneven row is a shading step of its own - and at
/// one the rows are level and the facets read as a crop. What it buys with is
/// stray: the curvature schedule is what keeps the long back columns of a head
/// off the skull, so an even one has to be paid for out of the shell's 4 mm
/// [`STAND`], and `a_shell_is_a_closed_solid_the_head_cannot_come_through` is
/// what says whether it can be.
///
/// Provenance: **tuned by render**, **bounded by the closed-solid guard**.
const CAP_EVEN: f32 = 1.0;

/// How far a [`ScalpStyle::SlickBack`]'s
/// rim is swept back off the brow, in metres.
///
/// Fixed rather than the style's axis, because what a slicked head varies is its
/// VOLUME and not its hairline: fourteen millimetres takes the default head's
/// rim from 34 to 47 mm above the brow, which reads as combed back rather than
/// as receding.
///
/// Provenance: **derived** from the measured column, **tuned by render**.
const SLICK_SWEPT: f32 = 0.014;

/// How much thickness a slicked-back head's front carries at volume one, as a
/// share of the head's own half-width.
///
/// Against the crown's own 0.055: a pompadour is about three times the mass of
/// the vault it rises out of, which on a default head's 72 mm half-width is
/// twelve millimetres of lift over the forehead.
///
/// Provenance: **tuned by render**.
const SLICK_RISE: f32 = 0.165;

/// How far a [`ScalpStyle::Bell`] carries its
/// rim past the hairline at the sides and the back, at each end of its axis, as
/// shares of the head's own half-width.
///
/// **In head radii and not in metres, because the three measured heads disagree
/// by half** (#346): the side of a default head has to run 83 mm past its
/// hairline to reach the gonion, seed 42's 57 and seed 7's 112 - which are 1.15,
/// 0.98 and 1.33 of each head's own half-width. A bell cut in millimetres is a
/// bob on one head and a curtain on another.
///
/// Provenance: **derived** from the measured drop to the gonion.
const BELL_DROP: [f32; 2] = [0.30, 1.20];

/// Where a bell's rim is broken by hem cards, as the cosine window its azimuth
/// must fall outside.
///
/// The window is the FACE, so the hem breaks all the way round and the notch
/// over the face does not: a bob's wisps are at its hem and never over its
/// cheek. Measured: at the temple's own azimuth a carried rim would sit below
/// the brow and inside the temple walls, which is the box the walk already stops
/// at - so the two agree by construction rather than by tuning.
///
/// Provenance: **derived** from the face box's own reach round the head.
const BELL_AT: [f32; 2] = [1.0, 0.65];

impl Cap {
    /// The catalogue's low-poly crop: a bowl to the hairline with a fringe notch
    /// over the brow, faceted, its rim broken at the brow and the nape.
    ///
    /// `fringe` cuts the notch, `0` at the hairline - where it is #345's own
    /// prototype but for the facets - to `CAP_NOTCH` back.
    #[must_use]
    pub fn crop(fringe: f32) -> Self {
        Self {
            shell: Shell {
                fringe: CAP_NOTCH * fringe.clamp(0.0, 1.0),
                even: CAP_EVEN,
                facets: Facets::All,
                ..Shell::default()
            },
            ..Self::default()
        }
    }

    /// The catalogue's slick: swept back off the brow, smooth, a pompadour rise
    /// at the front as `volume` grows, and no cards at all - the one style in
    /// this first catalogue whose edge is the solid's own.
    #[must_use]
    pub fn slicked(volume: f32) -> Self {
        Self {
            shell: Shell {
                fringe: SLICK_SWEPT,
                rise: SLICK_RISE * volume.clamp(0.0, 1.0),
                facets: Facets::Smooth,
                ..Shell::default()
            },
            // **No rim of cards, and the solid is still the hair.** A slicked
            // head's whole point is an unbroken edge; the shell is drawn because
            // the region was asked for, not because a card grew (#345's rule,
            // which this is the first style to need).
            rim_cards: 0,
            ..Self::default()
        }
    }

    /// The catalogue's bob: one bell to the jaw with a notch over the face, its
    /// vault smooth, and its rim broken by wisps everywhere the notch is not.
    ///
    /// Smooth rather than the "painterly" faceted hem band the issue briefed:
    /// that was built, isolated and refuted - see [`Facets`].
    ///
    /// `length` runs the hem from just past the hairline to the jawline, in the
    /// head's own radii (`BELL_DROP`).
    #[must_use]
    pub fn bell(length: f32, head: &Follicles) -> Self {
        let drop = head_radius(head)
            * (BELL_DROP[0] + (BELL_DROP[1] - BELL_DROP[0]) * length.clamp(0.0, 1.0));
        Self {
            shell: Shell {
                // Past the hairline, so negative: the front is left to the face
                // box, which cuts a truer notch than a number can.
                nape: -drop,
                side: -drop,
                ..Shell::default()
            },
            breaks: BELL_AT,
            ..Self::default()
        }
    }

    /// The catalogue's bun: a Cap-like shell with a closed sphere seated on its
    /// own back column, `height` running `0` a nape bun to `1` a top knot.
    ///
    /// Faceted like the crop, since the sphere is 48 triangles and a smooth
    /// shell beside a faceted ball reads as two materials.
    ///
    /// **The seat is the SHELL'S back column, not the knot's own arithmetic**,
    /// and the module constant BUN_EMBED is where the measurement that says so is written
    /// down. The axis is the share of that column, walked from its crown to its
    /// nape rim, so `0` puts the sphere at the rim and `1` at the pole and
    /// every head gets its own two ends rather than a height in millimetres.
    #[must_use]
    pub fn bun(height: f32, head: &Follicles) -> Self {
        let shell = Shell {
            even: CAP_EVEN,
            facets: Facets::All,
            ..Shell::default()
        };
        let radius = head_radius(head) * BUN_RADIUS;
        // The back column the sphere is seated on: the shell's own, so the two
        // cannot disagree about where the back of this head is.
        let back = walk(head, std::f32::consts::PI, &shell);
        let arc = along(&back);
        // `0` at the rim and `1` at the pole, which is the axis's own two ends.
        let on = at_share(&back, &arc, 1.0 - height.clamp(0.0, 1.0));
        let last = back.len() - 1;
        let at = ((1.0 - height.clamp(0.0, 1.0)) * last as f32).round() as usize;
        let out = facing(&back, at.max(1), std::f32::consts::PI);
        // Out of the shell's OUTER surface by the standoff's share of the
        // sphere, so the ball rests on the solid rather than in it.
        let thick = head_radius(head) * shell.rim;
        let centre = on + out * (STAND + thick + radius * (1.0 - BUN_EMBED));
        Self {
            shell,
            lump: Some(Lump {
                centre,
                radii: Vec3::from_array(BUN_SQUASH) * radius,
                shade: BUN_SHADE,
            }),
            ..Self::default()
        }
    }

    /// The catalogue's crest: the shell cut back to a band along the sagittal
    /// line, with a faceted fin along it whose height is `height`.
    ///
    /// The sides are SHAVED rather than bare - the painted layer at the density
    /// [`ScalpStyle::shaved`] guarantees, which is what makes the paint and the
    /// shell agree by construction rather than by a record's own choice (#347).
    ///
    /// No rim cards: a fin's edge is the solid's own, as a slicked head's is.
    /// The band has taken the rim off the sides altogether, and a hem card at
    /// the two ends of a strip is two wisps and not a fringe.
    #[must_use]
    pub fn crest(height: f32) -> Self {
        Self {
            shell: Shell {
                band: CREST_BAND,
                fin: CREST_FIN[0] + (CREST_FIN[1] - CREST_FIN[0]) * height.clamp(0.0, 1.0),
                even: CAP_EVEN,
                facets: Facets::All,
                ..Shell::default()
            },
            rim_cards: 0,
            ..Self::default()
        }
    }

    /// The catalogue's afro: one round mass standing off the whole scalp mask,
    /// smooth, in one tone, with its rim rolled in so there is no edge to see.
    ///
    /// `size` runs the mass from close-cropped coils to a full crown, in the
    /// head's own radii (`AFRO_THICK`) for `BELL_DROP`'s reason, and carries
    /// the rim down past the hairline at the sides and the back as it grows
    /// (`AFRO_DROP`), by the bell's own two terms and so past the face box's own
    /// notch.
    ///
    /// No rim cards: a card is an edge, and the whole acceptance is that there
    /// is none.
    #[must_use]
    pub fn afro(size: f32, head: &Follicles) -> Self {
        let size = size.clamp(0.0, 1.0);
        let crown = AFRO_THICK[0] + (AFRO_THICK[1] - AFRO_THICK[0]) * size;
        let drop = head_radius(head) * (AFRO_DROP[0] + (AFRO_DROP[1] - AFRO_DROP[0]) * size);
        Self {
            shell: Shell {
                crown,
                rim: crown * AFRO_SIDES,
                nape: -drop,
                side: -drop,
                roll: AFRO_ROLL,
                round: AFRO_ROUND[0] + (AFRO_ROUND[1] - AFRO_ROUND[0]) * size,
                tone: Tone::One,
                facets: Facets::Relief,
                ..Shell::default()
            },
            rim_cards: 0,
            ..Self::default()
        }
    }

    /// The catalogue's cornrows: a close shell with braided ridges running down
    /// the meridians and darker partings between them, `rows` running four
    /// ridges to ten (`BRAID_RIDGES`).
    ///
    /// No rim cards: a cornrow's edge is neat, and a loose wisp at the fringe
    /// is a different head of hair.
    #[must_use]
    pub fn braids(rows: f32) -> Self {
        let span = (BRAID_RIDGES[1] - BRAID_RIDGES[0]) as f32;
        Self {
            shell: Shell {
                crown: BRAID_THICK,
                rim: BRAID_THICK,
                ridges: BRAID_RIDGES[0] + (span * rows.clamp(0.0, 1.0)).round() as usize,
                ridge: BRAID_DEPTH,
                facets: Facets::Relief,
                ..Shell::default()
            },
            rim_cards: 0,
            ..Self::default()
        }
    }

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
            shape: self.shape(&scalp.cut, head),
            clumps: self.rim_cards,
            roots: scalp.roots,
            tips: scalp.tips,
        }
    }

    /// The same helmet as a head twelve metres off draws it (#350): its solid
    /// on the far grid ([`Shell::far`]), its ball if it has one, and no rim
    /// cards - a card is a sub-pixel sliver at that distance, and the rim is
    /// what the far tier is for not drawing.
    #[must_use]
    pub fn far(self) -> Self {
        Self {
            shell: self.shell.far(),
            rim_cards: 0,
            ..self
        }
    }

    /// The one head's worth of helmet this description grows, as a [`Shape`].
    ///
    /// Split out of [`Cap::sowing`] because `ScalpStyle::shape` wants the shape
    /// without a `Tress` to hand, and two ways of building one would be two
    /// opinions about what the body draws.
    #[must_use]
    pub fn shape(&self, cut: &Cut, head: &Follicles) -> Box<dyn Shape> {
        Box::new(Helmet {
            regions: head.clone(),
            shell: self.shell,
            cut: *cut,
            breaks: self.breaks,
            lump: self.lump,
        })
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
    /// Where its rim is broken by cards. See [`Cap::breaks`].
    breaks: [f32; 2],
    /// The solid it hangs off the shell, if it has one. See [`Cap::lump`].
    lump: Option<Lump>,
}

impl Helmet {
    /// Which way round the head a root sits, from dead ahead.
    fn azimuth(root: &Root) -> f32 {
        root.at.x.atan2(root.at.z)
    }

    /// Whether the rim is broken at this azimuth: outside this style's own
    /// window, which is the fringe and the nape for a crop and everything but
    /// the face notch for a bell.
    fn breaks(&self, azimuth: f32) -> bool {
        !(self.breaks[1]..=self.breaks[0]).contains(&azimuth.cos())
    }

    /// How far a rim card would run if nothing stopped it.
    fn reach(&self) -> f32 {
        RIM_ROOT + RIM_REACH * (0.55 + 0.45 * self.cut.length.clamp(0.0, 1.0))
    }

    /// Where one rim card is at `travel` metres along itself.
    ///
    /// Split out of [`Shape::at`] so [`Shape::length`] can walk the card AS
    /// DRAWN without asking for its length and recurring: the floor below is
    /// #341's construction, and #341's own lesson is that a clearance read off
    /// anything but the drawn lock is read off the wrong thing.
    fn drawn(&self, root: &Root, travel: f32) -> Vec3 {
        let azimuth = Self::azimuth(root);
        let walked = walk(&self.regions, azimuth, &self.shell);
        let last = walked.len() - 1;
        let out = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
        let normal = facing(&walked, last, azimuth);
        let thick = head_radius(&self.regions) * self.shell.rim;
        // Clear of the solid it breaks: past the shell's own outer surface at
        // the rim, by the same tolerance a card keeps off a skull.
        let lift = normal * (STAND + thick + RIM_LIFT);
        if travel <= RIM_ROOT {
            // Lying up the shell toward the crown, on the surface itself.
            return back_along(&walked, RIM_ROOT - travel) + lift;
        }
        // Then hanging, leaning out as it goes: hair leaving a mass does not
        // fall dead against it.
        let hang = travel - RIM_ROOT;
        walked[last] + lift + Vec3::NEG_Y * hang + out * (hang * RIM_LEAN)
    }
}

impl Shape for Helmet {
    fn length(&self, root: &Root) -> f32 {
        // A card only where the rim is broken: everywhere else the shell's own
        // edge is the hair's edge, and a lock at every meridian is a hem.
        if !self.breaks(Self::azimuth(root)) {
            return 0.0;
        }
        let reach = self.reach();
        // **And it stops before the face, as drawn** (#341's construction, and
        // #346's need for it): the shell's own walk already stops at the box, so
        // a bell's rim is swept up over the brow - but a hem card hangs BELOW
        // that rim, and 22 mm below the front rim of a default head 20 mm past
        // its hairline is inside the box (measured). Walked in chords rather
        // than checked at its ends, since a card that entered and left between
        // them would pass.
        let mut before = self.drawn(root, RIM_ROOT);
        let pieces = ((reach - RIM_ROOT) / CHECK).ceil().max(1.0) as usize;
        for piece in 1..=pieces {
            let share = piece as f32 / pieces as f32;
            let after = self.drawn(root, RIM_ROOT + (reach - RIM_ROOT) * share);
            if let Some(into) = self.regions.clearance().entry(before, after, STRAY) {
                let gone = (piece - 1) as f32 + into;
                return RIM_ROOT + (reach - RIM_ROOT) * gone / pieces as f32;
            }
            before = after;
        }
        reach
    }

    fn at(&self, root: &Root, along: f32) -> Vec3 {
        self.drawn(root, self.length(root) * along.clamp(0.0, 1.0))
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

    fn lump(&self) -> Option<Lump> {
        // A bun's sphere, which `Growth::grow` draws beside the shell and
        // counts with it - the tied-back's own lump on a different seat, and
        // drawn by the same loft (#347, and #342's `Shape::lump`).
        self.lump
    }
}

/// How far a rim card AS DRAWN can be from the spine the floor is read along,
/// in metres.
///
/// **Because the floor stops a spine and the box has to be clear of a CARD**
/// (#346, and `Sheet::stray`'s own reason). A rim card is drawn with its width
/// across the parallel and its own lift off the shell, so its corners sit up to
/// half a width and a lift away from the station the clearance was asked about,
/// and a floor read at no margin left four of seed 42's front vertices a
/// rounding inside the box, measured. Half a [`RIM_WIDTH`] plus the stand and
/// the lift is the most any of them can be.
///
/// Provenance: **derived** from the card's own width and lift.
const STRAY: f32 = RIM_WIDTH * 0.5 + STAND + RIM_LIFT;

/// How long a chord the face box is asked about a rim card in, in metres.
///
/// Three millimetres: a rim card is 22 to 30 mm long and leans as it drops, so a
/// tenth of it is finer than the box's own walls are near, and asking at its two
/// ends alone would pass a card that cut the box's corner (#341's lesson).
///
/// Provenance: **derived** from the card's own length and the box's walls.
const CHECK: f32 = 0.003;

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
    // fringe notch at the front, the nape line behind and the hem at the sides,
    // carried between by the azimuth's own cosine and sine, as every landmark on
    // a head is. Three terms rather than two because the sides are where a bob
    // hangs and the cosine leaves them alone (#346).
    let face = head.clearance();
    let facing = azimuth.cos();
    // **And the side term is held off until the column is clear of the FACE**
    // (#346). A hem weighted by the sine alone still carries a third of itself
    // at the temple, which on a default head puts the rim 8 mm below the brow
    // and inside the box - so the walk's floor below stops it dead at the brow
    // level and the notch comes out as a square visor across the forehead
    // (rendered, and that is what named this). Where the face ENDS is measured
    // rather than chosen: the box's own temple corner is at
    // `atan2(side, front)`, 0.88 radians off dead ahead on the default head, and
    // the hem comes down over the [`NOTCH_OVER`] radians past it - which is a
    // parting, and is what the floor is then never asked about.
    let off = facing.clamp(-1.0, 1.0).acos();
    let notch =
        crate::face::smooth(((off - face.side.atan2(face.front)) / NOTCH_OVER).clamp(0.0, 1.0));
    // **And the band term carries the same sine with NO ramp** (#347): a crest
    // shrinks the shell to a strip along the sagittal line, which means cutting
    // the rim back dead ahead as much as at the temple - the one place the
    // ramp above holds `side` at zero, measured, at every value it was asked.
    // Safe without it for a reason `side` does not have: this cut only ever
    // moves a rim UP its own column, and the front rim of the three measured
    // heads sits 16 to 58 mm above the brow before it is cut at all.
    let cut = shell.fringe * facing.max(0.0)
        + shell.nape * (-facing).max(0.0)
        + shell.side * azimuth.sin().abs() * notch
        + shell.band
            * crate::face::smooth(
                ((azimuth.sin().abs() - BAND_INSIDE) / (BAND_OVER - BAND_INSIDE)).clamp(0.0, 1.0),
            );
    // **The face notch is the face box, not a number** (#346). A rim carried
    // past the hairline descends, and at the front quarter of a default head a
    // rim 40 mm past it sits 8 mm BELOW the brow and 59 mm off the midline -
    // inside `Follicles::clearance`, which
    // `long_hair_does_not_hang_over_the_face` holds at ZERO (#341, measured
    // again here). So the walk stops where it would enter the box, which draws
    // the notch as the box's own top edge: swept up over the brow at the front,
    // falling away again past the temple where the box's side wall ends. A
    // notch tuned as a cut instead would be a tolerance, and #341 is the issue
    // that says a clearance is a construction.
    //
    // Inert for a rim at the hairline: the front rim of the three measured heads
    // sits 16 to 58 mm ABOVE the brow, so a cap's walk never reaches the box and
    // is point for point what it was.
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
        if face.contains(point) {
            break;
        }
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
fn schedule(walks: &[Vec<Vec3>], rows: usize, even: f32) -> Vec<f32> {
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
    // **And then pulled back toward even by as much as the style asks for**
    // (#346, and see [`Shell::even`]): the curvature schedule is what keeps a
    // smooth shell off the skull, and the uneven rows it leaves are what a
    // faceted one reads as a quilt. Blended rather than switched, so a style can
    // buy exactly the evenness it needs and pay for exactly that much stray.
    let even = even.clamp(0.0, 1.0);
    let last = (chosen.len() - 1).max(1) as f32;
    for (row, share) in chosen.iter_mut().enumerate() {
        *share += (row as f32 / last - *share) * even;
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
    /// Column-major: [`Shell::columns`] times [`Shell::rows`] of each.
    inner: Vec<Vec3>,
    outer: Vec<Vec3>,
    normals: Vec<Vec3>,
    /// The outer surface's own: the walk's normals copied, unless the style
    /// reads them off the outer surface (see [`Facets::Relief`]).
    outer_normals: Vec<Vec3>,
    /// Where each row sits, as a share of every column's own arc: the one
    /// schedule the grid was lofted on.
    shares: Vec<f32>,
    /// How many columns and rows it has: the shell's own.
    columns: usize,
    rows: usize,
}

impl Grid {
    /// Where one row of one column sits in the grid, wrapping round the head.
    fn at(&self, column: usize, row: usize) -> usize {
        cell(self.columns, self.rows, column, row)
    }

    /// Lofts the grid over a measured head.
    fn of(head: &Follicles, shell: &Shell) -> Self {
        let (columns, rows_down) = (shell.columns, shell.rows);
        let at = |column: usize, row: usize| cell(columns, rows_down, column, row);
        let radius = head_radius(head);
        let (crown, rim) = (radius * shell.crown, radius * shell.rim);
        // Every column walked first, because the rows are scheduled against all
        // of them at once: see [`schedule`] for the chevrons a per-column
        // schedule draws.
        let walks: Vec<Vec<Vec3>> = (0..columns)
            .map(|column| walk(head, TAU * column as f32 / columns as f32, shell))
            .collect();
        let rows = schedule(&walks, rows_down, shell.even);
        // A row schedule measured in ARC rather than in shares was tried here
        // and REFUTED (#347), and it is the lever of the slice for what it cost
        // to find out. The premise was sound and is still true: the schedule is
        // shared but it is in shares of each column's OWN arc, so row five of a
        // banded shell's 110 mm midline column sits 55 mm down the head while
        // row five of its 17 mm side column sits 8 mm down, and the quads
        // spanning the two are sheared.
        //
        // Measured in arc and clamped at each column's end, the shear does go -
        // and every one of a cut column's rows then lands ON its rim, at the
        // same position. That is not a shading defect, it is a broken solid:
        // the faces between two coincident rows are degenerate, and welded by
        // position the shell came out with 630 distinct vertices where a cap
        // has 830, 63 of 1,339 edges belonging to one face, and 14 vertices
        // 1.3 mm under the skin. `a_shell_is_a_closed_solid_the_head_cannot
        // _come_through` is what said so. It also looked worse (sheets347/level
        // - the fin drew as a blocky wedge with a notch out of it), but the
        // guard is why the knob is gone rather than tuned.
        let walked: Vec<Vec3> = walks
            .iter()
            .flat_map(|walk| {
                let arc = along(walk);
                rows.iter()
                    .map(move |share| at_share(walk, &arc, *share))
                    .collect::<Vec<_>>()
            })
            .collect();
        // The ellipsoid a round mass is placed on, fitted to every point the
        // grid is lofted through and stood off by the crown's thickness.
        let round = (shell.round > 0.0).then(|| Round::fitted(&walked, radius * shell.crown));
        // The catalogue's thin edge, which a rolled rim comes down to.
        let edge = radius * THICKNESS[1];
        let mut grid = Self {
            inner: Vec::with_capacity(columns * rows_down),
            outer: Vec::with_capacity(columns * rows_down),
            normals: Vec::with_capacity(columns * rows_down),
            outer_normals: Vec::new(),
            shares: rows.clone(),
            columns,
            rows: rows_down,
        };
        for column in 0..columns {
            for row in 0..rows_down {
                let point = walked[at(column, row)];
                // **The surface's normal read off the GRID, not off one
                // column's own walk.** A normal taken from the walk's
                // neighbours a few millimetres away follows every wobble the
                // drape puts in a single meridian; the surface the shell
                // actually has is the one its own rows and columns describe,
                // and that is what a quad is shaded by.
                let up = walked[at(column, row.saturating_sub(1))];
                let down = walked[at(column, (row + 1).min(rows_down - 1))];
                let left = walked[at((column + columns - 1) % columns, row)];
                let right = walked[at(column + 1, row)];
                let azimuth = TAU * column as f32 / columns as f32;
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
                let share = row as f32 / (rows_down - 1) as f32;
                // **A pompadour is a thickness that varies by azimuth** (#346):
                // the front of the crown carries the rise over the rest, peaked
                // part-way down the front column rather than at the pole,
                // because a slicked-back head's mass sits ABOVE THE FOREHEAD and
                // a rise at the pole is a cone.
                let front = azimuth.cos().max(0.0).powf(RISE_ROUND);
                let along = 1.0 - ((share - RISE_AT) / RISE_OVER).clamp(-1.0, 1.0).abs();
                let rise = radius * shell.rise * front * crate::face::smooth(along);
                // **And a FIN is the same idea along the midline** (#347):
                // thickness carried by the azimuth's cosine either way round,
                // front AND back, and peaked at the crown rather than part-way
                // down a column - which is what makes it a crest and not a
                // pompadour. Falls over the column by its own power, so the
                // fin is tallest at the pole and gone by the rim, which is
                // where the band has already taken the shell away.
                let midline = azimuth.cos().abs().powf(FIN_ROUND);
                let fin = radius * shell.fin * midline * (1.0 - share).powf(FIN_POW);
                // **And a cornrow is the same idea again, periodic** (#348):
                // the ridges' own profile, which melts into the crown's mean
                // rather than surviving to a pole every column shares.
                let ridge = radius * shell.ridge * ridged(shell, azimuth, rows[row]);
                let thick =
                    rim + (crown - rim) * (1.0 - share).powf(THICKNESS_POW) + rise + fin + ridge;
                let outer = if shell.roll > 0.0 || shell.round > 0.0 {
                    let mut full = point + normal * (STAND + thick);
                    // **And a ROUND mass is placed on its ellipsoid rather than
                    // stood off along the walk's normal** (#348, and see
                    // [`Shell::round`]): along the ray from the ellipsoid's own
                    // middle, which cannot fold however the rows are sheared.
                    if let Some(round) = &round {
                        full = full.lerp(round.through(point, STAND + edge), shell.round);
                    }
                    full
                } else {
                    point + normal * (STAND + thick)
                };
                grid.inner.push(point + normal * STAND);
                grid.outer.push(outer);
                grid.normals.push(normal);
            }
            // **An afro's rim rolls IN** (#348, and see [`Shell::roll`]): the
            // outer surface comes down to the thin edge over a length of the
            // column's own ARC that is a multiple of the gap it closes, so at
            // one it is a quarter circle whatever the mass stands off by - a
            // roll shorter than its gap is a shelf, and a share of the arc was
            // (a 34 mm roll closing 45 mm at the front drew a brim over the
            // brow). By arc rather than by row, because the rows are scheduled
            // where the head bends and the roll is a shape of its own.
            if shell.roll > 0.0 {
                let arc = *along(&walks[column]).last().unwrap_or(&0.0);
                let rim_at = at(column, rows_down - 1);
                let rim_edge = grid.inner[rim_at] + grid.normals[rim_at] * edge;
                let gap = grid.outer[rim_at].distance(rim_edge);
                let over =
                    (shell.roll * gap / arc.max(f32::EPSILON)).clamp(f32::EPSILON, ROLL_MOST);
                for (row, share) in rows.iter().enumerate() {
                    let at = at(column, row);
                    let edge_point = grid.inner[at] + grid.normals[at] * edge;
                    let into = ((share - (1.0 - over)) / over).clamp(0.0, 1.0);
                    let kept = (1.0 - into * into).max(0.0).sqrt();
                    grid.outer[at] = edge_point + (grid.outer[at] - edge_point) * kept;
                }
            }
        }
        // **Relief reads the OUTER surface's normals off the outer surface**
        // (#348, and see [`Facets::Relief`]), the same way the walk's are read
        // off its own grid above: the neighbours round and down, turned to face
        // the way the head's own normal does.
        grid.outer_normals = match shell.facets {
            Facets::Relief => (0..columns)
                .flat_map(|column| (0..rows_down).map(move |row| (column, row)))
                .map(|(column, row)| {
                    let here = at(column, row);
                    if row == 0 {
                        return Vec3::Y;
                    }
                    let up = grid.outer[at(column, row - 1)];
                    let down = grid.outer[at(column, (row + 1).min(rows_down - 1))];
                    let left = grid.outer[at((column + columns - 1) % columns, row)];
                    let right = grid.outer[at(column + 1, row)];
                    let normal = (right - left)
                        .cross(down - up)
                        .normalize_or(grid.normals[here]);
                    if normal.dot(grid.normals[here]) < 0.0 {
                        -normal
                    } else {
                        normal
                    }
                })
                .collect(),
            _ => grid.normals.clone(),
        };
        grid
    }
}

/// Where one row of one column sits in a grid of `columns` by `rows`,
/// column-major, wrapping round the head.
fn cell(columns: usize, rows: usize, column: usize, row: usize) -> usize {
    (column % columns) * rows + row
}

/// The soft ellipsoid a round mass of hair is placed on (#348).
///
/// **Fitted to the head it covers and not chosen**: its middle and its six
/// half-axes - front and back, up and down, and one across, since a head is
/// neither symmetric front to back nor top to bottom - are the bounds of every
/// point the grid is lofted through, scaled until all of them are inside it,
/// and then stood off by the style's own thickness. So the smallest mass is a
/// close round cap and the largest a ball, on every head.
struct Round {
    middle: Vec3,
    /// `[across, up, down, front, back]`.
    axes: [f32; 5],
}

impl Round {
    fn fitted(points: &[Vec3], standoff: f32) -> Self {
        let (lo, hi) = points.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(lo, hi), at| (lo.min(*at), hi.max(*at)),
        );
        let middle = Vec3::new(
            0.0,
            hi.y - (hi.y - lo.y) * ROUND_MIDDLE,
            (hi.z + lo.z) * 0.5,
        );
        let axes = [
            lo.x.abs().max(hi.x.abs()),
            hi.y - middle.y,
            middle.y - lo.y,
            hi.z - middle.z,
            middle.z - lo.z,
        ]
        .map(|axis| axis.max(0.01));
        let mut fitted = Self { middle, axes };
        // Scaled until every point is inside: an ellipsoid through the bounds
        // alone cuts the corners of a head between them.
        let widest = points
            .iter()
            .map(|at| fitted.norm(*at - middle))
            .fold(1.0f32, f32::max);
        fitted.axes = fitted.axes.map(|axis| axis * widest + standoff);
        // **But the underside comes in to the lowest rim and no further**
        // (#348, measured): scaled and stood off like the rest, the lower
        // half-axis put the mass's bottom far below the nape rim, so at the back
        // the full surface hung BELOW the rim it had to roll up into - 24 to 32
        // edges creased past 107 degrees at the nape and the low sides, up to
        // 174 - and from the side the ball's underside read as a shelf.
        fitted.axes[2] = (middle.y - lo.y) * ROUND_DOWN;
        fitted
    }

    /// How far out along its own direction a point is, `1` on the surface.
    fn norm(&self, from_middle: Vec3) -> f32 {
        let [across, up, down, front, back] = self.axes;
        let y = if from_middle.y >= 0.0 { up } else { down };
        let z = if from_middle.z >= 0.0 { front } else { back };
        ((from_middle.x / across).powi(2)
            + (from_middle.y / y).powi(2)
            + (from_middle.z / z).powi(2))
        .sqrt()
    }

    /// Where the ray from the middle through `point` meets the surface - or
    /// `least` metres past `point` along it, if that is further out.
    fn through(&self, point: Vec3, least: f32) -> Vec3 {
        let from = point - self.middle;
        let reach = from.length();
        let direction = from.normalize_or(Vec3::Y);
        let on = 1.0 / self.norm(direction).max(f32::EPSILON);
        self.middle + direction * on.max(reach + least)
    }
}

/// The most of a column's arc a roll may take, however far the mass stands off.
///
/// A roll longer than the column leaves no mass above it; past two thirds the
/// quarter circle has become a slope from the crown.
///
/// Provenance: **tuned by render**.
const ROLL_MOST: f32 = 0.66;

/// How far below its middle a round mass's underside reaches, as a share of the
/// distance down to the grid's lowest point: `1` closes it at the lowest rim.
///
/// Provenance: **derived** from the fold it removes, **tuned by render**.
const ROUND_DOWN: f32 = 1.0;

/// Where a round mass's middle sits, as a share of the grid's height down from
/// its top.
///
/// Provenance: **tuned by render**.
const ROUND_MIDDLE: f32 = 0.45;

/// How much of a cornrow ridge stands at one point of a shell, `0` in a
/// parting and `1` on a braid's crest.
///
/// **A cosine with a PLATEAU exactly as wide as the columns sample it** (#348,
/// measured before it was built). A crest of `n` ridges only draws at full
/// height where a column lands on it, and at 5, 7, 8 and 10 ridges none does
/// for some of them: 36 columns draw the worst crest at 0.94, 0.87, 0.77 and
/// 0.77 of its height, which is uneven braids. Held flat for the half a column
/// either side of the crest - `cos(n pi / COLUMNS)` - every crest and every
/// parting of every count contains a column and draws whole.
///
/// **And melted into the crown rather than carried to it.** Every column meets
/// at one welded apex, so the pole cannot be thicker on one column than on the
/// next, and a ten-ridge period is 10 mm at a tenth of the way down a default
/// head - narrower than the ridge is tall. So the profile eases to its own mean
/// over [`RIDGE_FADE`] of the column: radial cornrows meeting a smooth crown.
fn ridged(shell: &Shell, azimuth: f32, share: f32) -> f32 {
    if shell.ridges == 0 {
        return 0.0;
    }
    let count = shell.ridges as f32;
    let top = (count * std::f32::consts::PI / shell.columns as f32).cos();
    let profile =
        crate::face::smooth((((count * azimuth).cos() + top) / (2.0 * top)).clamp(0.0, 1.0));
    let fade = crate::face::smooth((share / RIDGE_FADE).clamp(0.0, 1.0));
    0.5 + (profile - 0.5) * fade
}

/// How much of its colour a cornrow's parting keeps, against a braid's crest.
///
/// A parting is where the scalp shows between two braids and the braids shade
/// each other, so it is the darkest line on the head - but a parting drawn at
/// the roots' own under-shade reads as a painted stripe rather than as a gap.
///
/// Provenance: **tuned by render**.
const PARTING: f32 = 0.35;

/// Over what share of a column, down from the crown, cornrow ridges come up out
/// of the crown's smooth mean.
///
/// Provenance: **derived** from the measured ridge period near the pole (10 mm
/// at a tenth of the way down at ten ridges), **tuned by render**.
const RIDGE_FADE: f32 = 0.60;

/// Draws one shell into `into`, bound rigidly to `head`, and returns what it
/// cost in triangles.
///
/// **Rigid to the head joint**: a scalp shell moves with nothing else, which is
/// what every scalp card's tip already does. A facial solid is bound as the skin
/// it covers is (#349, see [`face`]).
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
    let (columns, rows_down) = (grid.columns, grid.rows);
    let apex = |points: &[Vec3]| {
        let sum: Vec3 = (0..columns).map(|column| points[grid.at(column, 0)]).sum();
        sum / columns as f32
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
        for column in 0..columns {
            let azimuth = TAU * column as f32 / columns as f32;
            for row in 1..rows_down {
                let at = grid.at(column, row);
                let down = row as f32 / (rows_down - 1) as f32;
                let colour = match shell.tone {
                    // The tips' colour at the crown falling to the roots' at
                    // the rim, which is the way round a mass of hair is lit:
                    // the rim is the part in its own shadow.
                    Tone::Crown => tips.lerp(roots, down),
                    // **One tone, and only the underside darker** (#348): a
                    // gradient over one round mass reads as a highlight
                    // painted on.
                    Tone::One => tips,
                };
                // **The partings between cornrows are darker, on the ridge's
                // own profile** (#348), so the stripe and the ridge agree by
                // construction rather than by two numbers kept in step.
                let colour = if shell.ridges > 0 {
                    let crest = ridged(shell, azimuth, grid.shares[row]);
                    colour * (PARTING + (1.0 - PARTING) * crest)
                } else {
                    colour
                };
                push(grid.outer[at], grid.outer_normals[at], colour);
            }
        }
        push(apex_in, Vec3::NEG_Y, under);
        for column in 0..columns {
            for row in 1..rows_down {
                let at = grid.at(column, row);
                push(grid.inner[at], -grid.normals[at], under);
            }
        }
        for column in 0..columns {
            let at = grid.at(column, rows_down - 1);
            let (outer, inner) = (grid.outer[at], grid.inner[at]);
            // **Down the head, not down the outer surface, once that surface
            // rolls** (#348, measured): a rolled rim's last outer step points
            // back INTO the head, so a bevel carried along it folded back over
            // the rim - 10 to 30 edges creased past 107 degrees at the rim's
            // own height on every afro past close-cropped. The inner surface
            // still runs down the head there.
            let heading = if shell.roll > 0.0 {
                (inner - grid.inner[grid.at(column, rows_down - 2)]).normalize_or(Vec3::NEG_Y)
            } else {
                (outer - grid.outer[grid.at(column, rows_down - 2)]).normalize_or(Vec3::NEG_Y)
            };
            let across = (outer - inner).normalize_or(grid.normals[at]);
            let thick = outer.distance(inner);
            push(
                (outer + inner) * 0.5 + heading * (thick * BEVEL),
                (across + heading).normalize_or(grid.normals[at]),
                roots,
            );
        }
    }
    let rows = (rows_down - 1) as u32;
    let outer_apex = first;
    let inner_apex = first + 1 + columns as u32 * rows;
    let outer =
        |column: usize, row: usize| first + 1 + (column % columns) as u32 * rows + (row as u32 - 1);
    let inner = |column: usize, row: usize| {
        inner_apex + 1 + (column % columns) as u32 * rows + (row as u32 - 1)
    };
    let bevel = |column: usize| inner_apex + 1 + columns as u32 * rows + (column % columns) as u32;
    for column in 0..columns {
        let next = column + 1;
        // The crown's fan, wound as the quads below it are.
        into.faces
            .push(vec![outer_apex, outer(column, 1), outer(next, 1)]);
        into.faces
            .push(vec![inner_apex, inner(next, 1), inner(column, 1)]);
        for row in 1..rows_down - 1 {
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
            outer(column, rows_down - 1),
            bevel(column),
            bevel(next),
            outer(next, rows_down - 1),
        ]);
        into.faces.push(vec![
            bevel(column),
            inner(column, rows_down - 1),
            inner(next, rows_down - 1),
            bevel(next),
        ]);
    }
    match shell.facets {
        Facets::Smooth | Facets::Relief => {}
        Facets::All => facet(into, before..into.faces.len()),
    }
    into.faces[before..].iter().map(|face| face.len() - 2).sum()
}

/// Gives each of those faces its own normal, by splitting its corners away from
/// the vertices it shares with its neighbours.
///
/// **A facet is a shading DISCONTINUITY, so it cannot be had without splitting**
/// (#346). Every vertex of the grid belongs to four quads, and a normal it holds
/// is one answer for all four however it is computed; the only way a face can
/// carry its own is to stop sharing. Nothing else about the mesh moves: the
/// positions are copies, the triangle count is what it was, and the SURFACE is
/// the surface it was - which is why the closed-solid guard welds by position
/// before it counts edges rather than trusting the index buffer (#346).
///
/// The face's own normal is Newell's, taken from the winding, so the outer
/// surface's faces face out and the inner surface's face the head exactly as the
/// smooth ones they replace did.
///
/// **And the vertices the split leaves behind are removed** (#351, owner call).
/// Until then every corner was copied and the original stayed in the buffer
/// referenced by no face: 830 of a faceted cap's, bun's or crest's 4,214 to
/// 4,312 vertices (#350's census), uploaded by every consumer for nothing. Only
/// a vertex the faceted faces used AND no face uses afterwards goes, and every
/// index past it moves down, so no other region's geometry is touched and the
/// triangles drawn are the same triangles.
fn facet(into: &mut PolyMesh, faces: std::ops::Range<usize>) {
    let split_from = into.positions.len();
    let mut used = vec![false; split_from];
    for face in faces.clone() {
        for at in &into.faces[face] {
            used[*at as usize] = true;
        }
    }
    for face in faces {
        let corners = into.faces[face].clone();
        let mut normal = Vec3::ZERO;
        for (index, at) in corners.iter().enumerate() {
            let here = into.positions[*at as usize];
            let there = into.positions[corners[(index + 1) % corners.len()] as usize];
            normal += (here - there).cross(here + there);
        }
        let normal = normal.normalize_or(into.normals[corners[0] as usize]);
        let first = into.positions.len() as u32;
        for at in &corners {
            let at = *at as usize;
            into.positions.push(into.positions[at]);
            into.normals.push(normal);
            into.uvs.push(into.uvs[at]);
            into.colours.push(into.colours[at]);
            into.skin.push(into.skin[at]);
        }
        into.faces[face] = (0..corners.len() as u32).map(|step| first + step).collect();
    }
    // What the faceted faces used and nothing references any more.
    for face in &into.faces {
        for at in face {
            if let Some(slot) = used.get_mut(*at as usize) {
                *slot = false;
            }
        }
    }
    let Some(lowest) = used.iter().position(|orphan| *orphan) else {
        return;
    };
    // Each vertex's index once the orphans before it are gone.
    let mut remap = Vec::with_capacity(into.positions.len());
    let mut kept = 0u32;
    for at in 0..into.positions.len() {
        remap.push(kept);
        if !used.get(at).copied().unwrap_or(false) {
            kept += 1;
        }
    }
    let keep = |at: &usize| !used.get(*at).copied().unwrap_or(false);
    fn retain<T>(channel: &mut Vec<T>, keep: impl Fn(&usize) -> bool) {
        let mut at = 0;
        channel.retain(|_| {
            let kept = keep(&at);
            at += 1;
            kept
        });
    }
    retain(&mut into.positions, keep);
    retain(&mut into.normals, keep);
    retain(&mut into.uvs, keep);
    retain(&mut into.colours, keep);
    retain(&mut into.skin, keep);
    for face in &mut into.faces[..] {
        for at in face.iter_mut() {
            *at = remap[*at as usize];
        }
    }
    debug_assert!(
        lowest < split_from,
        "an orphan is one of the faceted copies"
    );
}
