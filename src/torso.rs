//! The chest, carved onto the trunk that the cage could not give one.
//!
//! **Every trunk ring in this crate is an ellipse**, and a pectoral or a breast
//! is a paired, localised forward lobe — a shape no ellipse expresses at any
//! setting of either of its two numbers. So until this module existed the chest
//! was a smooth tube on which `femininity`, `mass`, `body_fat` and `age` all
//! delivered exactly nothing: `examples/chestsection` reads the front of the
//! trunk standing `+0.00` mm proud of its own back at every band, one side to
//! the section at every band, and those three readings identical to the digit
//! between femininity `-1` and `+1`.
//!
//! # Why a carve, and not a node or a richer section
//!
//! Measured, all three against the same instrument (#271), and the other two
//! are refuted rather than merely unchosen — the numbers are in
//! `plan::derive::humanoid`'s module docs beside the trapezius table.
//! In short: a cage lobe cannot exist, because a socket pointing forward off a
//! trunk node has to clear its siblings' own forward reach and so must sit 110
//! mm in front of the chest's centre when the chest's surface is at 102.6; and
//! a richer section delivers half of what it is given and lands its lobes at
//! `±51` mm when life puts them at `±91`. A carve delivers 0.83 to 0.90 of what
//! it authors, lands within 3 mm of where it aims, spends no triangles, and —
//! being applied after the cage — cannot make a body fail to mesh.
//!
//! # What it can and cannot say
//!
//! The trunk's surface ring carries vertices at `x = 0, ±49.4, ±91.7, ±119.4`
//! mm on the reference body: ONE facet between the midline and the first vertex
//! out. A feature has to out-climb the ribcage's own 30.6 mm fall over the span
//! it covers before the section reads as two-sided at all, so a **projection**
//! shows from a millimetre up and a **shape** — two sides with a sternum
//! between them — needs 30 to 40. A bust clears that. A male pectoral, 10 to 20
//! mm in life, does not, and reads as a broad shelf with a shallow groove,
//! which is what a pectoral is.
//!
//! # The re-base
//!
//! **This changes the neutral body deliberately**, which in this crate is a
//! thing to declare rather than to discover later. `Composites::femininity`
//! says zero is the midpoint of the two measured references; the midpoint of a
//! male pectoral and a female breast is a small bust, not a flat plate. Holding
//! the neutral flat would have left the shipped default — every avatar built
//! from a record that never touched the axis — with the defect this module
//! exists to fix. Every bound the re-base moved is re-blessed in the same
//! commit, and the render goldens with it.

use glam::Vec3;

use crate::mesh::PolyMesh;
use crate::plan::derive::humanoid::between;
use crate::plan::{BODY_FAT_RANGE, Composites, DEFAULT_BODY_FAT, Zone};
use crate::rig::Rig;
use crate::texture::Condition;

/// How far a masculine chest stands off its ribcage, in chest-node radii.
///
/// A male pectoral runs 10 to 20 mm off the chest wall in life and the node is
/// 150.5 mm on the reference body, so the band is 0.066 to 0.133 radii. The
/// middle of it, because nothing here is a measurement of one person.
///
/// **It was 0.046 from #271 until #293, which is a third under the floor of the
/// band the paragraph above derives** — the value and its own stated derivation
/// never agreed, and `git log -S` says the constant was never edited in
/// between. What made that survivable for a while is written on this module's
/// note about relief: the surface was thought to deliver about twice the
/// authored push, and 6.9 mm doubled reads as 14, inside life's band. #284
/// refuted the doubling. It came from measuring relief at a fixed `x`, where a
/// radial carve widens the section and the reading climbs to the silhouette
/// edge — on the neutral body it reported 42 mm of "relief" at `x = 124` where
/// the lobe delivers 16.7 at `x = 92`. Measured along the push instead, the
/// masculine chest's own thickness was 5.8 to 9.7 mm across the WHOLE fat
/// range, with life's floor at 10.
///
/// At this value it reads 12.6 mm authored on a lean body, 15.0 at the shipped
/// fat fraction and 18.1 at 45% fat — inside the band at every point of the
/// axis rather than under it at all of them.
///
/// **[`FAT_GAIN`] was suspected with it and cleared by measurement.** A
/// pectoral is muscle, and a lean man's is more visible rather than smaller, so
/// a chest that thinned with leanness the way a breast does would be wrong. It
/// does not: the multiplier runs 0.843 at 5% fat to 1.350 at the top of the
/// range, a factor of 1.6 over the whole axis, and every point of it lands
/// inside life's band once this constant is right. What a lean body gets
/// instead is [`FIRMNESS`] and [`GAP_DEPTH`], which is where the visibility
/// belongs.
const PECTORAL: f32 = 0.0995;

/// How far a feminine chest stands off its ribcage, in chest-node radii.
///
/// A breast runs 40 to 90 mm from chest wall to nipple, so 0.27 to 0.60 radii;
/// this is the low end of that, at the default fat fraction, with [`FAT_GAIN`]
/// carrying the rest of the band.
///
/// **It was 0.225 until #293 — under the floor of its own band, for
/// [`PECTORAL`]'s reason and in the same change.** Both constants were sized
/// through a factor of two that #284 refuted: the surface was thought to
/// deliver about twice the authored push, so a value at half of life's figure
/// read as life's figure. Measured along the push instead, the feminine chest
/// stood 29.0 mm off its own ribcage at the shipped fat fraction against a life
/// floor of 40.
///
/// At 0.30 the axis maps onto life's band rather than under it: 45 mm authored
/// at the default fat fraction, 68 at 45% and 84 at the top of the range,
/// against 40 to 90. A very lean body comes in at 28, which is under the band
/// and is a very lean body.
const BUST: f32 = 0.30;

/// What mass does to the chest, as a share of the projection per unit of axis.
///
/// **Bulk, not softness**, which is the division `Composites::mass`'s own doc
/// draws: mass sets how much body there is and `body_fat` decides how it is
/// spent. A heavy lean chest is a big pectoral.
const MASS_GAIN: f32 = 0.35;

/// What a full fat range does to the projection, masculine and feminine.
///
/// **The first place two composites are read together on the trunk**, which
/// `BODY_FAT_RANGE`'s own doc says the chest is where it comes due: the same
/// fraction is a different body on the two frames, and a feminine frame stores
/// what it carries where this module draws. So fat buys more chest on a
/// feminine body than on a masculine one — more than twice as much — rather
/// than the same shelf getting thicker.
const FAT_GAIN: (f32, f32) = (0.35, 0.85);

/// How far round the section the pair sits, as a share of a quarter turn from
/// dead ahead.
///
/// **An AZIMUTH and not a lateral offset, which was tried first and is why this
/// says so.** Written as a share of the half-width, the lobe needed a cosine
/// taper to stop it running round the flank and widening the ribcage instead of
/// standing off it — and that taper fights the spacing, because it is weakest
/// exactly where the pair is meant to sit. The delivered separation then
/// depended on the PROJECTION: the same authored spacing put the peaks 104 mm
/// apart on a light chest and 192 mm apart on a heavy one. An angle needs no
/// taper, because a Gaussian centred at 51 degrees is already nothing by 90,
/// and it puts the pair where it was sent whatever the projection.
///
/// Life puts the intermammary distance at 180 to 230 mm; the reference body's
/// section is 260 mm across and 208 deep, so 0.57 of a quarter turn — 51
/// degrees — is that distance's low end. It is also where the mesh helps: the
/// trunk's surface ring carries a vertex at `±91.7` mm, which is 51 degrees
/// round, so the peak lands on a vertex rather than between two.
const SPACING: f32 = 0.57;

/// How broad each lobe is, as a share of a quarter turn, lean and soft.
///
/// **This is what makes a defined chest differ in GEOMETRY rather than only in
/// ink** (#272). At equal projection a lean chest is a tighter lobe with a
/// visible edge and a fat one is a broad soft rise, and the interpolation is
/// [`Condition::definition`] — the same derived read the skin painter uses, so
/// the shape and the striation over it cannot disagree about how lean this body
/// is.
const SPREAD: (f32, f32) = (0.18, 0.28);

/// Where the chest peaks, from the waist joint at `0` to the girdle at `1`.
///
/// Life puts a pectoral's and a breast's peak a fifth to a third of the way
/// down the sternum from the notch, which on this column is 0.67 to 0.80.
/// Below that band on purpose: the sternal notch sits above the girdle joint,
/// so the band this is measured in reaches higher than the sternum does.
const HEIGHT: f32 = 0.65;

/// How tall each lobe is, as a share of the waist-to-girdle span.
const TALL: f32 = 0.17;

/// How that height is shared above the peak and below it.
///
/// **The mean of the two is 1, so [`TALL`] still means what its own line says**
/// and the peak's amplitude is untouched either way: a Gaussian's crest is its
/// coefficient whatever its span is, so widening one side and narrowing the
/// other moves where the volume sits without moving how far the chest stands
/// off. That is the whole of what this axis is for.
///
/// Life prefers **45:55 upper to lower** — Mallucci's series and the ASPS
/// studies put the preference at 87 to 94% — and a symmetric Gaussian is 50:50
/// by construction. A half-Gaussian's area goes as its span, so 45:55 is a
/// ratio of 1.22 between the two; these are further apart than that, and the
/// reason is the whole of #285: the SURFACE does not deliver what the carve
/// authors. Measured with `examples/chestsection`, whose `authored` row probes
/// the carve itself and whose top row reads the built polygons, an authored
/// 50:50 arrived as 55:45 on the refined body — so the authored split has to
/// overshoot to land. These are tuned against the DELIVERED reading, which is
/// the one a render shows.
///
/// **#286's design note asked for the upper falloff to be the slower of the two
/// and that is the opposite of 45:55, so the measurable target won.** A slower
/// upper falloff is a taller upper pole and a taller pole has more area under
/// it; the prose and the ratio cannot both be had from one span pair. What the
/// anatomy means by a straight upper pole is a matter of the profile's
/// CURVATURE rather than its extent, which a Gaussian has no knob for — it is
/// recorded on `examples/chestsection`'s bow reading and left to a later slice
/// rather than faked here.
const POLES: (f32, f32) = (0.72, 1.28);

/// How square the lobe's own profile is, lean and soft.
///
/// **The exponent of the falloff, and it buys the plateau and the crisp border
/// with one number** (#287). A pectoral is a PLATE and a breast is a lobe: the
/// male-breast literature's defining feature is superior fullness turning into
/// a firm inferior border at the fold's level, which is a flat top and a steep
/// edge. `exp(-|t|^p)` at `p = 2` is the Gaussian this module has always drawn;
/// raising `p` flattens the crest and steepens the flank in the same move, so
/// the plateau and the border are not two terms that can disagree about where
/// the mass stops.
///
/// **Interpolated on [`Condition::definition`], which makes it exactly zero on
/// the neutral body** — `Condition::default().definition` is 0 — so #286's
/// re-base is untouched by this and only a body leaner than the default is
/// shaped by it. That is not a convenience: it is the same axis [`SPREAD`] and
/// [`FOLD_DEPTH`] run on and the same one the skin painter draws its striation
/// from, so the shape, the crease and the ink cannot disagree about how lean
/// this body is.
///
/// The pole split survives it: a half of `exp(-|t|^p)` has area `σ · Γ(1+1/p)`,
/// and with one exponent for both poles that factor cancels out of the ratio
/// [`POLES`] sets. Measured rather than trusted — `examples/chestsection`
/// prints the split at every femininity.
const FIRMNESS: (f32, f32) = (2.8, 2.0);

/// How deep the sternal gap cuts, as a share of the chest's own projection,
/// lean and soft.
///
/// **The depression BETWEEN the pair, so it is scaled by what the pair stands
/// off**: without two masses there is nothing for a sternum to sit between, and
/// scaling it this way means a body with no chest grows no groove without a
/// threshold to tune. Soft is zero — a sternal gap is a leanness reading, which
/// is #283's own note on what makes a pectoral read as one.
const GAP_DEPTH: (f32, f32) = (0.30, 0.0);

/// How far round the section the sternal gap reaches, as a share of a quarter
/// turn.
///
/// Narrow, because a sternum is: 0.15 of a quarter turn is 13.5°, which on the
/// reference trunk's 130 mm half-width puts the groove's own width at about 30
/// mm. The lobe itself is nothing at this azimuth — [`SPACING`] puts it at 0.57
/// and a Gaussian three spreads away is two parts in a thousand — so this cuts
/// into the sternum rather than out of the pair.
const GAP_WIDE: f32 = 0.15;

/// How deep the inframammary fold cuts, as a share of the chest's own
/// projection, lean and soft.
///
/// **A share of the projection, so it dies with the chest it belongs to.** A
/// flat chest has no fold and must not grow one; scaled this way the term goes
/// to zero exactly as the lobe does, with no threshold to tune and no branch to
/// get wrong.
///
/// Interpolated by [`Condition::definition`] — the same derived read the skin
/// painter and [`SPREAD`] use — because a fold is a crease and a crease is
/// sharper on a lean body.
///
/// **Lean first, which is this module's convention and was got backwards once**
/// (#287). `between` maps `-1` to the first element and the argument here is
/// `1 - 2·definition`, so element zero is the LEAN value — the same order
/// [`SPREAD`] and [`FIRMNESS`] are written in. Landed at #286 as `(0.10, 0.22)`
/// under a docstring reading "soft and defined", which handed the deep crease
/// to the soft body and the shallow one to the lean: caught by #287's border
/// test, which measured a soft chest's lower flank falling 22.2 against a lean
/// chest's 18.8 per unit of projection and could not be satisfied by any
/// firmness.
///
/// The other reading — that an inframammary fold deepens with VOLUME, a heavier
/// breast sitting further into its own crease — is not lost by pointing this at
/// leanness, because it is already carried: the depth is a share of the
/// projection, and the projection is what fat buys.
const FOLD_DEPTH: (f32, f32) = (0.22, 0.10);

/// Where the fold sits below the YOUNG peak, as a share of the young lower
/// pole's span.
///
/// **Seated on the lobe's own landmark and not on a constant, which is #182's
/// lesson verbatim: a constant seat dies quietly when a ruler moves.** The
/// lower pole's span carries the body's proportions and the lift axis, so a
/// fold written as a share of it sits where this chest's base is rather than
/// where the reference body's was.
///
/// **But not the age descent, which #286 wrote in and #288 took back out.**
/// This gives [`ChestTraits::fold`] its value once, off the un-aged geometry,
/// and ageing never touches it again — because ptosis IS the peak approaching
/// this line, and a fold that travels with the peak makes that impossible. See
/// that field for the measurement: derived off the aged peak, the fold fell 56
/// mm across the age range while the peak fell 52, so the two never met.
///
/// Life puts the fold about 70 mm below the peak on a reference-scale body,
/// which against this body's own lower span — `span · TALL · POLES.1`, or 50.2
/// mm on the reference trunk — is 1.4 of it. That is just outside the lobe's
/// visible edge, where a Gaussian has fallen to a twentieth of its crest, which
/// is where a breast's base is: the fold is the bottom of the footprint rather
/// than a groove cut into the lobe.
const FOLD_SEAT: f32 = 1.4;

/// How tall the crease is, as a share of the lower pole's span.
///
/// An inframammary crease runs 20 to 40 mm in life, which on the reference
/// body's 50.2 mm lower span is 0.4 to 0.8 of it; this is the low end, because
/// the term is a Gaussian and its visible width is about twice this.
const FOLD_TALL: f32 = 0.22;

/// How far round the section the crease reaches, as a multiple of the lobe's
/// own spread.
///
/// **Wider than the lobe, because the footprint is.** A breast's root spans the
/// sternal edge to the mid-axillary line with the axillary tail running
/// upper-outer, so the base it sits on is broader than the crest above it. It
/// is a crescent in azimuth and height — the jawline's `border_raise` machinery
/// (#195–197) rotated onto the trunk — and not a profile of radius against
/// height, which is the jaw's own lesson: separable height profiles could not
/// make a mandible and one non-separable border term could.
const FOLD_WIDE: f32 = 1.35;

/// How far age lowers the peak, as a share of the span, and how much it lets
/// the volume hang below itself.
///
/// **`Composites::ageing` and not a second curve**, which is that method's
/// whole argument: the trunk that settles, the lip that thins and the chest
/// that descends are one body getting older rather than three timetables that
/// can disagree.
///
/// **The first number went 0.10 → 0.18 at #288, re-tuned against a fold that
/// finally holds still.** Ptosis is graded by where the peak sits relative to
/// the inframammary fold (Regnault), and until [`ChestTraits::fold`] was
/// anchored there was nothing for a descent to be measured against: at
/// femininity `+1` the peak fell 52 mm across the age range and the fold fell
/// 56 mm with it. Anchored and re-tuned, the peak closes from 70 mm above its
/// fold at thirty to 20 mm at eighty, and the delivered pole split goes 42:58
/// to 29:71 — which is what "the volume stops being held up" looks like when
/// something is holding it.
///
/// **Not further, and the limit is the model rather than the anatomy.** At 0.24
/// the peak reaches the fold — Regnault grade 1 — and the crease stops being a
/// crease: the trough is cut at the crest instead of under it, because this
/// carve has no overhang and cannot put tissue in front of its own fold the way
/// a body does. Measured with `--profile`, the surface is smooth there rather
/// than notched, and `examples/chestsection`'s seat reading jumps from 20 mm to
/// 51 as it goes looking for a concavity that is no longer the fold's. So the
/// shipped envelope tops out just short of grade 1 by construction, which also
/// makes "never grade 3" free everywhere except at a negative lift axis, where
/// `ChestTraits::on` clamps it.
///
/// The second number is unchanged at 0.55 and was re-measured rather than
/// assumed: swept to 0.42 and 0.34 it moved the delivered split by one point
/// and two, because what shifts the poles on an old body is the PEAK descending
/// and not the tail slackening. It is kept where it is because it is what holds
/// the lower pole convex — at 0.34 the eighty-year-old's lower bow goes
/// negative, and a hollow lower pole is not what age does.
const DESCENT: (f32, f32) = (0.18, 0.55);

/// What a record's own chest axes ask for, over what the composites derived.
///
/// **Offsets, and the two-tier contract is why they are a separate type.** A
/// quantity here is `formula(composites)` and then this on top: the composites
/// carry the intent, so a stored avatar keeps meaning what it meant when the
/// formulas improve, and these carry the choice a creator made that no formula
/// predicted. Neutral is all zeros, which is the composites' own answer
/// unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChestAxes {
    /// How much chest there is, over what the composites predict.
    pub volume: f32,
    /// How far it stands off against how far it spreads, at a fixed volume.
    pub projection: f32,
    /// How high it sits, over what age and fat predict.
    pub lift: f32,
    /// How far apart the pair sits, over the azimuth this module's `SPACING`
    /// names.
    ///
    /// **MakeHuman's `breast-dist` and Second Life's Cleavage, and it is an
    /// AZIMUTH like the constant it offsets** (#289) — read that constant's own
    /// line for why, because the reason is a refutation rather than a
    /// preference: written as a lateral offset the delivered separation
    /// depended on the projection, and the same authored spacing put the peaks
    /// 104 mm apart on a light chest and 192 apart on a heavy one.
    pub spacing: f32,
    /// Where the volume sits between the upper pole and the lower one, over
    /// the split this module's `POLES` derives.
    ///
    /// **MakeHuman's `breast-volume-vert`, and a direct offset on the pole
    /// ratio rather than a second mechanism for it** (#289). Positive is a
    /// fuller lower pole and negative an evenly divided one; the mean of the
    /// two poles is held at 1 exactly as `POLES` holds it, so this moves
    /// where the volume sits without moving how far the chest stands off.
    ///
    /// **Not a second ptosis axis, and the distinction is [`ChestTraits::fold`]'s
    /// own.** Age descends the PEAK toward a fold that stays where it is;
    /// this moves the pole split and the fold under it together, because
    /// `FOLD_SEAT` is a share of the lower pole's span and a fold seated on a
    /// ruler has to travel when the ruler does.
    pub fullness: f32,
}

/// What one unit of [`ChestAxes::volume`] does to the projection.
///
/// Half again, so the axis is worth reaching for and cannot on its own out-swing
/// the composites that placed the chest: at `+1` a neutral body's 38.7 mm goes
/// to about 58, which is the distance femininity's whole positive half covers.
const VOLUME_GAIN: f32 = 0.5;

/// What one unit of [`ChestAxes::projection`] trades, standing off against
/// spreading across.
///
/// **Two coefficients rather than one, and they are not equal.** The axis is
/// meant to hold the volume roughly still while changing the shape, and a lobe's
/// volume goes as its height times the SQUARE of its breadth — so a given
/// fractional change in breadth moves twice as much volume as the same change in
/// height, and the height term has to be the larger of the two to compensate.
const PROJECTION_TRADE: (f32, f32) = (0.35, 0.16);

/// How the lobe's footprint grows with its projection, as an exponent on the
/// projection's ratio to the feminine reference [`BUST`].
///
/// **A bigger breast is a bigger breast, not a taller spike** (#309). Every
/// span the lobe is measured in — [`SPREAD`] across and [`TALL`] times
/// [`POLES`] up and down — was a constant, while the projection runs with
/// femininity, fat, mass and two record axes; at femininity +1.5 with volume
/// and projection both at +1 the crest stood 86 mm off the wall on an upper
/// pole 22 mm tall, a slope of four, and the render showed a faceted cone
/// with a crease down its lateral border where neighbouring vertices were
/// pushed radially by wholly different amounts. A record may ask for three
/// times that.
///
/// So the footprint swells with the height. At `1.0` the lobe would scale in
/// similarity and keep the reference's own slopes exactly; life's larger
/// breasts project a little more than they broaden, so this sits under it —
/// the flank slope grows slowly with size and never approaches vertical.
/// Only above the reference: a pectoral or a small chest keeps the spans its
/// constants name, so nothing the composites alone reach is re-shaped by this.
const SWELL: f32 = 0.85;

/// The most a chest stands off its ribcage, in chest-node radii.
///
/// Life's 90 mm is 0.6 radii; this is half again past it, which is a
/// stylisation and meant to be. Past it the lobes wrap the trunk and hang
/// below the band however the footprint swells: a record's axes may ask
/// (`volume` and `projection` run to ±3 in the exploration envelope) and the
/// carve delivers up to here.
const PROJECTION_CEILING: f32 = 0.9;

/// Where a limb's claim on the skin ends, in the limb's OWN radii from its
/// bone: full carve beyond the first, none inside the second.
///
/// **A point is the arm's if it lies on the arm, not if the arm's bone
/// happens to be the nearest one** (#310). The carve used to take
/// `Rig::nearest_bone`'s word for ownership, and at feminine shoulders that
/// answer puts the upper-arm bone nearer than the chest bone across the
/// upper-outer quadrant of the chest's FRONT — a vertex 104 mm from a 45 mm
/// arm bone, with the trunk's own bone 120 mm behind it, read as arm. Nothing
/// carved it, `refine_chest` did not refine it, and every lobe big enough to
/// reach it was cut off along that line: a concave wedge from the armpit
/// into the breast, worst with projection, which the owner found on the
/// dressed body. #309's fade softened the cut and left it. Measured in the
/// limb's radii instead, the arm's skin sits at one radius and the chest's
/// front at two and more, and the lobe runs out under the arm the way a
/// breast's axillary tail does.
const LIMB_FADE: (f32, f32) = (1.7, 1.15);

/// What one unit of [`ChestAxes::lift`] does: how far up the band it moves the
/// peak, and how much of the age descent it refuses.
const LIFT_GAIN: (f32, f32) = (0.06, 0.5);

/// What one unit of [`ChestAxes::spacing`] moves the pair, as a share of a
/// quarter turn.
///
/// **Derived on the same section and by the same arithmetic as [`SPACING`]
/// itself**, which is the point of writing it here rather than picking a round
/// number: life puts the intermammary distance at 180 to 230 mm, the reference
/// trunk's ring is 260 mm across and 208 deep, and where [`SPACING`]'s 0.57 of
/// a quarter turn is that band's LOW end at 183 mm, 0.72 is 224 mm — the wide
/// end of it. Symmetric, so `-1` lands at 0.42, about 137 mm: closer than life
/// goes, which is the creator's choice this axis exists to carry and is what
/// the negative half of every axis on [`ChestAxes`] does.
///
/// **0.15 rather than the 0.175 that reaches life's 230 mm exactly, and the
/// rail is why.** `ChestTraits::on` keeps the crest a spread clear of the
/// flank, and a soft body's spread is [`SPREAD`]'s own 0.28 — so its ceiling is
/// 0.72 and a gain that authored 0.745 would be pinned there on every ordinary
/// body, which is an axis whose last sixth does nothing. At 0.15 the gain and
/// the rail agree at the positive end instead of fighting, and what a body with
/// room to spare loses is 6 mm of separation it had no way to show.
///
/// **What a moved pair costs, and it is not amplitude**: `SPACING`'s own line
/// records that 0.57 puts the crest on a surface vertex, and any other value
/// puts it between two. On `render --bare --close chest` that shows as a flat
/// facet with a hard highlight at the crest, at every value of this axis but
/// zero. The lobe is the right size and in the right place — the cell under it
/// is 19 to 23 mm — so it is the second refinement pass #292 costs and nothing
/// this axis can fix.
///
/// Measured with `examples/chestsection --profile`, whose crest azimuth is
/// read RADIALLY: on the neutral body the delivered crest sits at 0.44, 0.58
/// and 0.71 of a quarter turn for `-1`, `0` and `+1`, against authored 0.42,
/// 0.57 and 0.72 — where each was sent, to the hundredth. At femininity `+1`
/// the positive end reads 0.68 and stops there however far the axis is pushed,
/// which is the rail on a body whose spread is 0.32 and is the design working
/// rather than the axis failing. **The section table's `separation between
/// them` reads this axis BACKWARDS** and must not be used for it — see that
/// instrument's own note.
const SPACING_GAIN: f32 = 0.15;

/// What one unit of [`ChestAxes::fullness`] moves the pole split, as a share of
/// [`TALL`] shifted from the upper pole to the lower one.
///
/// **Stated on the DELIVERED split and measured, not on the authored one**,
/// which is [`POLES`]'s own lesson: the surface does not deliver what the carve
/// authors, and a gain written against the authored ratio would land somewhere
/// nobody chose. The shipped body reads 45:55 at femininity 0 off an authored
/// 39:61, and an authored 50:50 arrives as 55:45 — so ten delivered points cost
/// 0.28 of split and five cost 0.14.
///
/// `±1` is therefore about six delivered points either side of life's preferred
/// 45:55, which is the width of the preference band rather than the width of
/// what a chest can look like. Measured on the shipped body at femininity 0:
/// **51:49 at `-1`, 45:55 at `0` and 39:61 at `+1`**, off authored splits of
/// 45:55, 39:61 and 33:67 — and the crest reads 29.5, 29.5 and 29.6 mm across
/// the three, which is the mean-of-1 invariant holding rather than being
/// asserted. **Deliberately far short of what age reaches** —
/// [`DESCENT`] takes the delivered split to 29:71 at eighty — because an offset
/// that out-swings the composite it corrects is the incoherence the two-tier
/// contract exists to prevent, and because a pole split that far over IS
/// ptosis, which the record already has an axis for.
const FULLNESS_GAIN: f32 = 0.14;

/// How many of the lobe's own spreads the pair stays off the midline.
///
/// **Where the mirror stops showing**, which is what makes this rail a
/// different multiple from the one at the flank. `carve_chest` places the pair
/// by `from.x.abs()`, so the two halves meet at the sternum and the surface
/// there carries the lobe's own slope: at one spread the midline holds
/// `exp(-1)` — a third — of the crest and the meeting is a crease, at 1.5 it
/// holds a tenth and the two are a pair. Verified on `render --bare --close
/// chest` at femininity `+1`, which is the sheet a merged pair shows up on and
/// the section table does not.
///
/// It also fixes where the axis's own end lands: on the neutral body, whose
/// spread is 0.28, `-1` authors 0.42 and this rail sits at 0.42 — so the near
/// end of [`SPACING_GAIN`] and the near rail agree exactly, as the far end and
/// the far rail already did.
const SPACING_FLOOR: f32 = 1.5;

/// The narrowest either pole may be left, as a share of [`TALL`].
///
/// A pole is the span of a falloff and a span of zero is a step in the
/// surface. **It does not bind inside the record's own envelope** — the axes
/// clamp at `±3` and [`FULLNESS_GAIN`] can take the upper pole no lower than
/// 0.30 there — so this is a rail for a caller assembling [`ChestAxes`] by
/// hand, which is a thing a public type allows.
const POLE_FLOOR: f32 = 0.15;

/// What the chest is shaped like, derived from the composites.
///
/// **Derived rather than stored**, the way `face::HeadTraits` and
/// `texture::Condition` are, and for the same reason: a consumer needs "how far
/// does this chest stand off and how broad is it", not four axis values it
/// would have to re-derive at every site. Record axes land on top of these as
/// offsets rather than replacing them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChestTraits {
    /// How far the chest stands off the ribcage at its peak, in chest-node
    /// radii.
    pub projection: f32,
    /// How far off the midline the pair sits, as a share of the trunk's
    /// half-width. See this module's `SPACING`.
    pub spacing: f32,
    /// How broad each lobe is across, as a share of the half-width.
    pub spread: f32,
    /// Where the peak sits between the waist joint and the girdle.
    pub height: f32,
    /// How far the volume hangs below its own peak, `0` young and `1` at the
    /// top of the age range.
    pub descent: f32,
    /// Where the inframammary fold sits, in the same waist-to-girdle span as
    /// [`ChestTraits::height`].
    ///
    /// **Carried rather than derived from the peak, because ageing must not
    /// move it** (#288). An inframammary fold is a ligamentous structure
    /// attached to the chest wall: in ptosis the BREAST descends toward the
    /// fold and the fold stays where it is, which is the whole of what the
    /// Regnault grades measure. Derived off the peak it did the opposite —
    /// measured before this landed, at femininity `+1` the peak fell 52 mm
    /// across the age range and the fold fell 56 mm with it, so the two never
    /// approached each other and no body was ever ptotic by any definition.
    ///
    /// Lift moves it, because a creator asking for a higher chest means the
    /// whole structure; age does not.
    pub fold: f32,
    /// How the lobe's height is shared above the peak and below it, as
    /// multiples of this module's `TALL` whose mean is 1.
    ///
    /// **Carried rather than read off `POLES` at the carve**, because
    /// [`ChestAxes::fullness`] offsets it and a constant the carve reads
    /// directly is a constant no record axis can reach. The mean is held at 1
    /// wherever this is written, so `TALL` goes on meaning what its own line
    /// says and the crest's amplitude is untouched by the split.
    pub poles: (f32, f32),
    /// How lean this body reads, `0` soft and `1` defined.
    ///
    /// **Carried rather than re-derived**, which is what this whole type is
    /// for: `SPREAD` and `FOLD_DEPTH` both interpolate on it, the skin
    /// painter draws its striation and crease ink from the same number, and a
    /// second derivation is a second place for the shape and the ink over it to
    /// disagree about how lean a body is.
    pub definition: f32,
}

impl ChestTraits {
    /// Reads the composites into what the carve needs.
    #[must_use]
    pub fn of(composites: &Composites) -> Self {
        // Clamped for `Condition::of`'s reason: the frame axis carries the
        // exploration envelope, and a body three times past the feminine
        // reference is a stylisation rather than a body with three busts.
        let frame = composites.femininity.clamp(-1.5, 1.5);
        let ageing = composites.ageing();
        // Fat above the default adds; below it takes away. Measured from the
        // default rather than from the range's floor so that the projection at
        // the shipped fat fraction is the one the constants above name.
        let spare = (composites.body_fat - DEFAULT_BODY_FAT)
            / (BODY_FAT_RANGE.1 - DEFAULT_BODY_FAT).max(f32::EPSILON);
        let fat_gain = between(frame, FAT_GAIN.0, FAT_GAIN.1);
        let definition = Condition::of(composites).definition;
        Self {
            projection: (between(frame, PECTORAL, BUST)
                * (1.0 + MASS_GAIN * composites.mass.clamp(-1.5, 1.5))
                * (1.0 + fat_gain * spare))
                .max(0.0),
            spacing: SPACING,
            spread: between(1.0 - 2.0 * definition, SPREAD.0, SPREAD.1),
            height: HEIGHT - DESCENT.0 * ageing,
            descent: ageing,
            poles: POLES,
            // The young geometry's own fold, which is the anchor: `HEIGHT`
            // before any descent, less the seat [`FOLD_SEAT`] names, taken
            // against the lower pole before [`DESCENT`] slackens it.
            fold: fold_seat(POLES),
            definition,
        }
    }
}

/// Where the fold sits, on the poles it is a share of.
///
/// **One derivation read from two places**, which is [`FOLD_SEAT`]'s own
/// argument taken one step further: that constant is a share of the lower
/// pole's span so the seat travels when the body's proportions do, and
/// [`ChestAxes::fullness`] moves that span too. Written out at both sites it
/// would be two places for the seat to drift from its ruler.
fn fold_seat(poles: (f32, f32)) -> f32 {
    HEIGHT - FOLD_SEAT * TALL * poles.1
}

/// How far the lobe's spans swell for a projection, per [`SWELL`]: `1` at and
/// below the feminine reference, more above it.
fn swell(projection: f32) -> f32 {
    (projection.min(PROJECTION_CEILING) / BUST)
        .max(1.0)
        .powf(SWELL)
}

impl ChestTraits {
    /// Applies a record's own axes on top of what the composites derived.
    ///
    /// **Multiplicative on the projection and additive on the placement**, and
    /// that is the difference between an offset that scales with the body and
    /// one that does not: a creator asking for more chest means more of the
    /// chest this body has, and a creator asking for it higher means a fixed
    /// distance up a band that is already measured in the body's own span.
    #[must_use]
    pub fn on(self, axes: ChestAxes) -> Self {
        let (stand, trade) = PROJECTION_TRADE;
        // **Lift refuses the descent once and every consequence of it follows**
        // (#288), which is what makes this one axis rather than three. The
        // descent it refuses is the same number that lowers the peak and
        // slackens the lower pole, so giving back `DESCENT.0` times the
        // difference raises the peak by exactly the ptosis lift declined — and
        // the pole balance comes back with it, because that balance IS the
        // slackening.
        let descent = (self.descent * (1.0 - LIFT_GAIN.1 * axes.lift)).clamp(0.0, 1.0);
        let held = self.height + DESCENT.0 * (self.descent - descent) + LIFT_GAIN.0 * axes.lift;
        // **Shifted from one pole to the other rather than added to one of
        // them**, so their mean stays 1 and [`TALL`] goes on meaning what its
        // line says: a Gaussian's crest is its coefficient whatever its span
        // is, so this moves where the volume sits and not how far the chest
        // stands off. [`POLE_FLOOR`] is a rail for a hand-built [`ChestAxes`]
        // and does not bind inside the record's own envelope.
        let shift = FULLNESS_GAIN * axes.fullness;
        let poles = (
            (self.poles.0 - shift).max(POLE_FLOOR),
            (self.poles.1 + shift).max(POLE_FLOOR),
        );
        // The fold travels with the pole it is a share of — [`FOLD_SEAT`]'s own
        // argument, which is why `fullness` moving the lower pole moves the
        // crease under it while AGE still does not (#288). Written as the move
        // rather than as a fresh seat, so a fold that already carries a lift is
        // not silently re-derived without it.
        let fold = self.fold + fold_seat(poles) - fold_seat(self.poles) + LIFT_GAIN.0 * axes.lift;
        let spread = (self.spread * (1.0 - trade * axes.projection)).max(0.05);
        // **Both rails are measured in the lobe's own spread rather than
        // written as constants** (#182's lesson: a constant rail dies quietly
        // when the ruler moves), and they are DIFFERENT multiples of it because
        // the two ends fail differently. At the flank nothing meets and the
        // lobe simply runs out of body, so one spread inside it is enough. At
        // the midline the carve MIRRORS — it reads `from.x.abs()` — so the two
        // lobes meet there, and a crest one spread off centre delivers a third
        // of itself onto the midline with a slope: rendered at `-3` on a
        // feminine body that is one narrow mound with a crease down it, not a
        // close-set pair. At [`SPACING_FLOOR`] spreads it is a pair again.
        //
        // Not `clamp`, which panics when the two rails cross — nothing in the
        // envelope takes a spread past 0.4, and a hand-built axis set that did
        // should be pinned rather than a panic.
        let spacing = (self.spacing + SPACING_GAIN * axes.spacing)
            .max(SPACING_FLOOR * spread)
            .min(1.0 - spread);
        Self {
            projection: (self.projection
                * (1.0 + VOLUME_GAIN * axes.volume)
                * (1.0 + stand * axes.projection))
                .max(0.0),
            spacing,
            spread,
            poles,
            // **Never below its own fold, which is the one Regnault grade a
            // stylised body cannot wear.** Grades 1 and 2 are the peak reaching
            // and passing the fold; grade 3 is the peak at the breast's lowest
            // contour, and on a body this abstract that does not read as age,
            // it reads as broken. A negative lift axis is what reaches for it —
            // the roll envelope goes to `-3` — rather than any age.
            height: held.max(fold),
            descent,
            fold,
            ..self
        }
    }
}

impl Default for ChestTraits {
    /// The neutral body's chest — which is a chest, not a plate. See the
    /// module's note on the re-base.
    fn default() -> Self {
        Self::of(&Composites::default())
    }
}

/// The column a chest is carved on: where the band is and how wide the trunk
/// gets.
struct Column {
    /// Height of the waist joint.
    waist: f32,
    /// Height of the shoulder girdle.
    girdle: f32,
    /// The chest node's radius, which is the lateral half-extent of its cage
    /// ring — `CHEST_SECTION`'s lateral multiple is 1.
    half: f32,
    /// Where the trunk's own axis runs, taken at the chest.
    axis: Vec3,
}

impl Column {
    /// Reads the column off the rig, or `None` on a body that has no trunk.
    fn of(rig: &Rig) -> Option<Self> {
        let waist = *rig.in_zone(Zone::Abdomen).first()?;
        let chests = rig.in_zone(Zone::Chest);
        let (&chest, &girdle) = (chests.first()?, chests.get(1)?);
        Some(Self {
            waist: rig.joints[waist].position.y,
            girdle: rig.joints[girdle].position.y,
            half: rig.joints[chest].radius,
            axis: rig.joints[chest].position,
        })
    }
}

/// The region each chest-refinement pass covers: how far round the section it
/// reaches as a share of a quarter turn from dead ahead, then its lowest and
/// highest point in the waist-to-girdle band [`Column`] is written in.
///
/// **Written in the CARVE's own two units and not in `FACE_PASSES`'s**, which
/// is the one design decision here. The face's bands are a cosine of the
/// azimuth off an unsectioned head with heights in two different
/// normalisations either side of a joint; this module places its lobe by
/// [`SPACING`] — a share of a quarter turn — at a [`HEIGHT`] between the waist
/// joint and the girdle. A band that has to hold resolution UNDER a shape is
/// written in the shape's own numbers or the two disagree about where the lobe
/// is on every body whose proportions differ from the one the band was written
/// on, which is exactly what `face::band_at` exists to prevent on the face.
///
/// # The one pass, and the measurement that says it is needed
///
/// It is the whole front of the chest band, and what it buys is the bound #271
/// stated and could not lift: the trunk's surface ring carries ONE facet
/// between the midline and the first vertex out, so a paired shape needs 30 to
/// 40 mm of relief before a section reads as two-sided at all — and a male
/// pectoral stands 10 to 20. Measured with `examples/chestsection --lobe` on
/// the default body, before and after:
///
/// ```text
///   synthetic lobe      25 mm            30 mm
///   before        1 side, dip 0.00   3 sides, dip 0.78 mm
///   after         3 sides, dip 1.76  3 sides, dip 5.64 mm
/// ```
///
/// The threshold drops below 25 mm and the groove the surface can express at 30
/// mm goes up sevenfold. That is the difference between a shelf and a chest.
///
/// It also moves what the carve DELIVERS toward what it authors: the neutral
/// body's authored 50:50 pole split arrives at 58:42 on the shipped cell and at
/// 55:45 after this pass. See [`refine_chest`] for why that is a resolution
/// finding and not a carve one.
///
/// # What the budget refused, and the arithmetic that refused it
///
/// **A second, tighter pass over the lower half of the lobe** — where the
/// inframammary fold (#286) and the pectoral's inferior border (#287) both
/// live, and where this one leaves 15 to 23 mm cells against creases that are
/// 20 to 40 mm wide in life. It was costed at `(0.35, 1.0, 0.22, 0.48)`: 252
/// triangles of skin, 660 on a dressed body, and it halves the cell where it
/// runs. It is not here because there is nothing to buy it with, and the chain
/// is worth writing down because #283's research got it wrong by 1,454:
///
/// * The binding corner is not the dearest body. It is the PRODUCT of the
///   dearest head and the greediest legal hair, which `tests/budget.rs` reaches
///   at 29,078 against a 30,000 target — 922 free, where the research had 2,350.
/// * `hair::clump::MAX_TRIANGLES` is defined as what the body leaves, so a
///   dearer body takes it down. That is its documented contract and not a new
///   decision.
/// * But #209 put a FLOOR under it that is not negotiable: nothing a re-roll
///   can produce is ever trimmed, and the dearest re-rollable head costs about
///   2,750. Bisected 2026-08-19 — the tier test passes at 2,750 and fails at
///   2,278, reporting a crop at density 1 that costs 2,732.
///
/// With the tight pass in, the ceiling has to be at most 2,278 to satisfy the
/// leftover and at least 2,750 to satisfy the floor, and no number is both. One
/// pass fits with 188 triangles to spare; two need the 30,000 target to move,
/// which is the owner's call and was declined on 2026-08-19.
///
/// **Retaken the same day and declined again**, and this time on the pass's own
/// render rather than on the budget — see the note under the band floor below
/// for the before-and-after, which is a fold 30 to 80% deeper on lean bodies and
/// nothing at all for the pectoral border or for #289's `chestSpacing`. The
/// arithmetic above is also softer than it reads: measured again with the pass
/// actually in, the dearest bald body is 27,628 and the leftover 2,372, so the
/// target needed 30,378 rather than the 32,278 the first paragraph implies. It
/// is still a raise, and it still buys one term.
///
/// **The floor of this band is −0.10, and it was 0.05 for one day** (#292,
/// 2026-08-19). #285 measured −0.10, wanted it, and could not afford the 90
/// triangles; the risk that took is a trap #284 measured: a band edge inside
/// the lobe reads as a fold, because the coarse side of a resolution boundary
/// under-delivers the push and the step between them is a crease — with the
/// floor at 0.50 the instrument reported 5.40 mm of notch 44 mm under the peak
/// where the same body clear of the boundary reports 2.90, and the bare control
/// did not move. At 0.05 the boundary sat under the lobe on every ordinary body
/// and inside its lower tail wherever age and lift together dragged the peak
/// down, which is a corner of the roll envelope rather than the middle of it —
/// but it is a corner a record can ask for, so it was bought back the moment
/// there was a decision open to buy it in.
///
/// It cost 108 rather than 90, measured on the dearest bald body (27,032 →
/// 27,140), and it came out of `hair::clump::MAX_TRIANGLES` — 2,938 → 2,830,
/// which leaves 80 triangles between that ceiling and #209's floor under it.
///
/// **The second pass was retaken at #292 with the fold's render in hand, and
/// declined again.** What it bought, measured: on a LEAN body the fold went
/// 1.87/2.26/3.63 mm to 2.59/3.97/4.78 against bare rows of 2.30/2.17/0.80, so
/// it cleared its own noise floor at three femininities instead of two; on a
/// soft body 1.63/2.47/7.62 to 2.34/4.46/7.75 against 6.31/11.58/10.50, which
/// clears at none of them either way. What it did NOT buy, and both were
/// expected to: the pectoral's inferior border did not move at all
/// (0.697/0.927/1.006 → 0.680/0.857/1.006), and #289's `chestSpacing` had
/// nothing to gain, because the flat bright patch a moved lobe shows on a
/// render is a SPECULAR HIGHLIGHT and not a facet — the normal buffer over it
/// is smooth, and a third pass over the crest band changed the picture by
/// nothing for 1,408 triangles. 596 triangles and 600 of `TRIANGLE_TARGET` for
/// one term's depth on lean bodies was not the trade it was filed as.
///
/// **A tight pass on the sternum column** was in #285's design and is refused
/// on its own merits rather than on the budget's: it costs 182 triangles and
/// buys nothing measurable, because this pass already starts at dead ahead so
/// the sternum is refined once before any narrow pass could name it. With it,
/// the synthetic 25 mm lobe's groove reads 1.61 mm against 1.76 without, and
/// the 30 mm lobe's 5.49 against 5.64 — inside the noise and on the wrong side
/// of it. Worth revisiting at #287, when there is a sternal-gap term for it to
/// hold.
const CHEST_PASSES: [(f32, f32, f32, f32); 1] = [(0.0, 1.0, -0.10, 0.99)];

/// Gives the chest enough surface to carry a shape, before anything carves one.
///
/// **[`carve_chest`]'s sibling, and it runs a long way before it**: this is
/// resolution and that is shape, and `build_body`'s own comment on the block it
/// sits in says why the order cannot be the other way round. A carve applied to
/// a surface with 30 to 50 mm cells delivers what those cells can hold, which
/// is not what it authored — measured with `examples/chestsection`, the
/// neutral body's authored 50:50 pole split arrives at 58:42 on the shipped
/// cell and at 55:45 after this. Refining afterwards would only subdivide the
/// facets of a shape that had already been flattened.
///
/// Selects the way the carve selects — `trunk_claim`, the trunk's own
/// bones nearest and no limb's skin — because an arm passes through this
/// band and a chest drawn onto one is a defect nobody would have to look hard
/// for; the carve solved that exact problem for its vertices and the answer
/// does not change for a face centroid. One rule for both, or the carve
/// reaches skin the refinement left coarse (#310).
///
/// Splits with [`PolyMesh::refine_curved`] for [`refine_face`]'s reason: the
/// trunk arrives as a lofted ring surface, a plain midpoint sits on one of its
/// chords, and a linear split would buy the section more vertices and no
/// curvature at all.
///
/// Does nothing to a body with no trunk column, which is the same refusal
/// `Column::of` gives the carve and costs no special path.
///
/// [`refine_face`]: crate::face::refine_face
#[must_use]
pub fn refine_chest(mesh: &PolyMesh, rig: &Rig, levels: usize) -> PolyMesh {
    let Some(column) = Column::of(rig) else {
        return mesh.clone();
    };
    let span = column.girdle - column.waist;
    if levels == 0 || span <= f32::EPSILON {
        return mesh.clone();
    }

    let mut refined = mesh.clone();
    for pass in 0..levels {
        // Passes past the last named one repeat the tightest region rather than
        // widening again, which is `refine_face`'s rule and for its reason:
        // asking for more resolution should never spend it on a flank.
        let (near, far, low, high) = CHEST_PASSES[pass.min(CHEST_PASSES.len() - 1)];
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| {
                let at = refined.face_centroid(face);
                if trunk_claim(rig, at) <= 0.0 {
                    return false;
                }
                let up = (at.y - column.waist) / span;
                if up < low || up > high {
                    return false;
                }
                let from = Vec3::new(at.x - column.axis.x, 0.0, at.z - column.axis.z);
                if from.length_squared() <= f32::EPSILON {
                    return false;
                }
                let azimuth = from.x.abs().atan2(from.z) / std::f32::consts::FRAC_PI_2;
                (near..=far).contains(&azimuth)
            })
            .collect();
        refined = refined.refine_curved(&selected);
    }
    refined
}

/// Carves a chest onto a built body.
///
/// **After the cage and before anything measures, binds or charts the
/// surface**, which is `face::carve_face`'s own rule and for its reason: skin
/// weights, texture charts, the garment cut and every attached part are fitted
/// to the mesh in hand, and a chest that appears afterwards is a chest none of
/// them knows about.
///
/// Moves vertices along the trunk's own outward direction rather than straight
/// forward, because a breast projects off the chest WALL and the wall is
/// already turning away by the spacing the pair sits at. Straight forward would
/// slide the lobe round toward the midline as it grew.
///
/// A body with no trunk column is left alone, which is the refusal a quadruped
/// or a partial rig needs and costs no special path.
pub fn carve_chest(mesh: &mut PolyMesh, rig: &Rig, traits: &ChestTraits) {
    let Some(column) = Column::of(rig) else {
        return;
    };
    let span = column.girdle - column.waist;
    if span <= f32::EPSILON || column.half <= f32::EPSILON || traits.projection <= 0.0 {
        return;
    }
    let peak = column.waist + span * traits.height;
    let reach = traits.projection.min(PROJECTION_CEILING) * column.half;
    // The footprint grows with the height — see [`SWELL`]. Every span below
    // is the trait's own times this, so the lobe keeps its shape as it grows
    // rather than sharpening into a cone.
    let swell = swell(traits.projection);
    let spread = traits.spread * swell;
    let tall_span = span * TALL * swell;
    // The fold's own height, and the span its crease is measured in. Both are
    // taken before [`DESCENT`] slackens anything: an anchor that the ageing it
    // is the reference for can move is not an anchor. The seat travels down
    // with the swelled lower pole, which is [`FOLD_SEAT`]'s own rule — it is
    // a share of that pole — written as the move from the trait's seat so a
    // fold carrying a lift keeps it.
    let anchor =
        column.waist + span * (traits.fold - FOLD_SEAT * TALL * traits.poles.1 * (swell - 1.0));
    let held = tall_span * traits.poles.1;

    for point in &mut mesh.positions {
        // The trunk and nothing else. An arm passes through this band and a
        // chest drawn onto one is a defect nobody would have to look hard for
        // — and the push fades toward the arm rather than stopping at its
        // boundary, see [`LIMB_FADE`].
        let claim = trunk_claim(rig, *point);
        if claim <= 0.0 {
            continue;
        }
        let from = Vec3::new(point.x - column.axis.x, 0.0, point.z - column.axis.z);
        let out = from.normalize_or_zero();
        if out == Vec3::ZERO {
            continue;
        }
        // How far round the section this vertex sits, from dead ahead to the
        // flank. See [`SPACING`] for why the lobe is placed in this and not in
        // a lateral offset.
        let azimuth = from.x.abs().atan2(from.z) / std::f32::consts::FRAC_PI_2;
        let across = (azimuth - traits.spacing) / spread;
        // Below the peak the fall is slackened by age, which is what a chest
        // descending is: the volume does not move down so much as stop being
        // held up — and it is the FULLER of the two poles before age touches
        // it, which is what [`POLES`] is.
        let up = point.y - peak;
        let lower = tall_span * traits.poles.1 * (1.0 + DESCENT.1 * traits.descent);
        let tall = if up < 0.0 {
            lower
        } else {
            tall_span * traits.poles.0
        };
        let along = (up / tall.max(f32::EPSILON)).abs();
        // **C1 at the peak comes free and it is worth saying why**, because a
        // crest with a kink in it renders as a crease down the middle of the
        // chest. The slope of `exp(-|t|^p)` at its own centre is zero for every
        // `p` above 1, whatever the span, so the two halves meet flat however
        // far apart [`POLES`] pulls them and however square [`FIRMNESS`] makes
        // them, and no blend is needed between them.
        let firm = between(1.0 - 2.0 * traits.definition, FIRMNESS.0, FIRMNESS.1);
        let lobe = reach * (-across.abs().powf(firm) - along.powf(firm)).exp();

        // The inframammary fold: a crescent in azimuth and height, seated on
        // [`ChestTraits::fold`] — the body's own anchor, which ageing does not
        // move. See [`FOLD_SEAT`] for where that anchor comes from and
        // `ChestTraits::fold` for why the peak has to approach it rather than
        // carry it.
        let seat = (point.y - anchor) / (FOLD_TALL * held).max(f32::EPSILON);
        let wide = across / FOLD_WIDE;
        let depth = reach * between(1.0 - 2.0 * traits.definition, FOLD_DEPTH.0, FOLD_DEPTH.1);
        let fold = depth * (-wide * wide - seat * seat).exp();

        // The sternal gap: a narrow trough on the midline, running the height
        // of the pair it sits between. See [`GAP_WIDE`] for why this cuts the
        // sternum rather than the lobes.
        let midline = azimuth / GAP_WIDE;
        let gap = reach
            * between(1.0 - 2.0 * traits.definition, GAP_DEPTH.0, GAP_DEPTH.1)
            * (-midline * midline - along.powf(firm)).exp();

        *point += out * ((lobe - fold - gap) * claim);
    }
}

/// How much of the chest carve a point takes: `0` unless the nearest of the
/// body's own bones (limbs aside) is the trunk's, then `1` clear of every
/// limb and falling to `0` onto a limb's skin — see [`LIMB_FADE`].
///
/// Limbs are kept out of the nearest-bone contest on purpose: that contest
/// is the right way to tell the chest from the neck or the pelvis, which
/// share the column, and the wrong way to tell it from an arm, which hangs
/// beside it at whatever distance the shoulders put it.
pub(crate) fn trunk_claim(rig: &Rig, point: Vec3) -> f32 {
    let mut nearest = (f32::INFINITY, false);
    let mut limb = 1.0f32;
    for joint in rig.surfaced() {
        let (start, end) = rig.bone(joint);
        let axis = end - start;
        let along = if axis.length_squared() <= f32::EPSILON {
            0.0
        } else {
            ((point - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
        };
        let distance = point.distance(start + axis * along);
        match rig.joints[joint].zone {
            Zone::UpperLimb(_) | Zone::LowerLimb(_) | Zone::Extremity(_) => {
                let (near, far) = rig.bone_radii(joint);
                let radius = (near + (far - near) * along).max(f32::EPSILON);
                let t = ((distance / radius - LIMB_FADE.1) / (LIMB_FADE.0 - LIMB_FADE.1))
                    .clamp(0.0, 1.0);
                limb = limb.min(t * t * (3.0 - 2.0 * t));
            }
            zone => {
                if distance < nearest.0 {
                    nearest = (distance, matches!(zone, Zone::Chest | Zone::Abdomen));
                }
            }
        }
    }
    if nearest.1 { limb } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avatar::{Avatar, MeshKind};
    use crate::plan::{Archetype, BodyPlan, Composites, HumanoidParams};
    use crate::record::AvatarRecord;
    use crate::{BODY_SUBDIVISIONS, CageConfig, build_body};

    /// A record at named composites, sanitized the way a shipped one is.
    fn body_of(femininity: f32, mass: f32, fat: f32, age: u32) -> AvatarRecord {
        let mut record = AvatarRecord::new("Chested", Archetype::default());
        record.composites.femininity = femininity;
        record.composites.mass = mass;
        record.composites.body_fat = fat;
        record.composites.age = age;
        record.sanitize();
        record
    }

    /// Vertices of the chest band, as a predicate on a point.
    fn in_band(column: &Column, point: Vec3) -> bool {
        let span = column.girdle - column.waist;
        (column.waist + span * 0.45..column.waist + span * 0.90).contains(&point.y)
            && point.x.abs() < column.half * 1.2
            && point.z > 0.0
    }

    /// The furthest the carve pushed any vertex of the chest band.
    ///
    /// **Paired by index, which is exact**: the carve only moves positions, so
    /// the two meshes are the same mesh and the difference is the displacement
    /// itself rather than a bisection's estimate of it. What it reports is what
    /// was AUTHORED; the relief a caliper would read off the surface is about
    /// twice it, because a radial push lands a vertex where the ribcage behind
    /// it was shallower — see `examples/chestsection`, which is where the
    /// comparison against life is made.
    fn pushed(plain: &PolyMesh, carved: &PolyMesh, rig: &Rig) -> f32 {
        let Some(column) = Column::of(rig) else {
            return 0.0;
        };
        plain
            .positions
            .iter()
            .zip(&carved.positions)
            .filter(|(before, _)| in_band(&column, **before))
            .fold(0.0f32, |most, (before, after)| {
                most.max(before.distance(*after))
            })
    }

    /// How far the chest's own quarter of the section stands forward against
    /// the trunk's midline at the same heights.
    ///
    /// **Dimensionless, and it needs no second mesh to compare against**, which
    /// is what the shipped body requires: `Avatar::build` refines and cuts the
    /// surface, so its topology is not the plain body's and nothing can be
    /// paired by index. A lofted ellipse hands back the ratio of its own two
    /// half-extents at the lobe's azimuth and a chest raises it.
    ///
    /// **Not "how far forward does the chest band reach", which was tried and
    /// is why this says so.** The furthest-forward vertex of a tube is on the
    /// MIDLINE, and the midline is the one place a paired lobe does not touch —
    /// so that reading moved by 0.2 mm for a carve the instrument put at 38,
    /// and reported a masculine chest out-projecting a feminine one.
    fn flank_over_midline(mesh: &PolyMesh, rig: &Rig) -> f32 {
        let Some(column) = Column::of(rig) else {
            return 0.0;
        };
        let (mut flank, mut midline) = (0.0f32, 0.0f32);
        for &point in &mesh.positions {
            if !in_band(&column, point) {
                continue;
            }
            let share = point.x.abs() / column.half;
            if share < 0.15 {
                midline = midline.max(point.z);
            } else if (0.45..0.85).contains(&share) {
                flank = flank.max(point.z);
            }
        }
        if midline <= f32::EPSILON {
            return 0.0;
        }
        flank / midline
    }

    /// The same body with and without the carve, in one space.
    fn pair(record: &AvatarRecord) -> (PolyMesh, PolyMesh) {
        let skeleton = record.skeleton();
        let traits = crate::face::HeadTraits::of(&record.composites);
        let plain = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &traits,
        )
        .expect("the default plan meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("the default plan rigs");
        let mut carved = plain.clone();
        carve_chest(&mut carved, &rig, &ChestTraits::of(&record.composites));
        (plain, carved)
    }

    #[test]
    fn the_shipped_body_carries_the_chest_it_was_built_with() {
        // **The wiring, which is the one thing the instrument cannot check.**
        // `examples/chestsection` carves the body itself so that both surfaces
        // it measures are in one space; that leaves nobody asking whether
        // `Avatar::build` — the path every consumer takes — applies the same
        // carve. It did not, once: the module existed, the instrument read 38
        // mm, and the shipped render was flat.
        let record = body_of(1.0, 0.0, 0.22, 30);
        // **Undressed, and it has to be asked for at build time.** A dressed
        // body does not emit the skin its clothes cover, so the shipped Skin
        // mesh of a clothed avatar has NOTHING at chest height — measured, 14
        // vertices in the whole band and none of them near the midline. The
        // garment over a chest that is no longer a tube is #274's.
        let avatar = Avatar::build_with(
            &record,
            &crate::avatar::AvatarConfig {
                dressed: false,
                ..Default::default()
            },
        )
        .expect("the default record builds");
        let shipped = avatar
            .meshes
            .iter()
            .find(|part| part.kind == MeshKind::Skin)
            .expect("an avatar has skin");
        let (plain, _) = pair(&record);
        let skeleton = record.skeleton();
        let bare_rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let with = flank_over_midline(&shipped.mesh, &avatar.rig);
        let without = flank_over_midline(&plain, &bare_rig);
        assert!(
            with > without + 0.10,
            "the shipped body's chest stood {with:.3} of its own midline against an uncarved \
             {without:.3} — the carve is not reaching the path every consumer takes",
        );
    }

    #[test]
    fn a_feminine_chest_projects_where_a_masculine_one_barely_does() {
        // The axis's largest single piece of unfinished business (#272): before
        // this module `femininity` reached the chest only as `shoulder_frame`,
        // which NARROWED the ribcage — measured, `-1` delivered a chest 25 mm
        // deeper than `+1`, the opposite of the anatomy.
        //
        // Compared at equal stature, because rolling a seed to find a feminine
        // body moves five other axes at once.
        // Each end against its OWN uncarved body: the two ribcages are not the
        // same width, so a shared reference would report the frame axis's
        // narrowing as a chest.
        let (lean_bare, masculine) = pair(&body_of(-1.0, 0.0, 0.22, 30));
        let (full_bare, feminine) = pair(&body_of(1.0, 0.0, 0.22, 30));
        let lean_rig = Rig::from_skeleton(&body_of(-1.0, 0.0, 0.22, 30).skeleton()).expect("rigs");
        let full_rig = Rig::from_skeleton(&body_of(1.0, 0.0, 0.22, 30).skeleton()).expect("rigs");
        let lean = pushed(&lean_bare, &masculine, &lean_rig);
        let full = pushed(&full_bare, &feminine, &full_rig);
        assert!(
            lean > 0.005,
            "a masculine chest projected {:.1} mm, which is no pectoral at all",
            lean * 1000.0
        );
        assert!(
            full > lean * 2.0,
            "a feminine chest projected {:.1} mm against a masculine {:.1}, which is not the \
             difference between a bust and a pectoral",
            full * 1000.0,
            lean * 1000.0
        );
    }

    #[test]
    fn a_lean_chest_is_a_different_shape_and_not_only_a_different_ink() {
        // **Geometry, not ink** (#272). `texture::skin` spent
        // `Condition::definition` on striation grain and crease ink because
        // there was no geometry to spend it on; a defined chest has to differ
        // in SHAPE or the painter is still drawing a pectoral that is not
        // there. What differs is the lobe's breadth at the same projection —
        // see [`SPREAD`] — so the reading is where the furthest-forward vertex
        // sits, not how far forward it is.
        let lean = ChestTraits::of(&Composites {
            body_fat: 0.05,
            ..Composites::default()
        });
        let soft = ChestTraits::of(&Composites {
            body_fat: 0.45,
            ..Composites::default()
        });
        assert!(
            soft.spread > lean.spread * 1.2,
            "a chest at 45% fat spread {:.3} of a quarter turn against a lean {:.3}",
            soft.spread,
            lean.spread
        );
        // And the same axis is the painter's, so the two cannot disagree about
        // how lean this body is.
        assert!(
            Condition::of(&Composites {
                body_fat: 0.05,
                ..Composites::default()
            })
            .definition
                > Condition::of(&Composites {
                    body_fat: 0.45,
                    ..Composites::default()
                })
                .definition,
        );
    }

    #[test]
    fn the_three_axes_say_three_different_things() {
        // **None of them reproduces another**, which is #273's acceptance and
        // the reason `chest_projection` exists at all: two bodies can carry the
        // same amount of chest and differ entirely in whether it is a high
        // tight one or a broad soft one, so an axis that moved projection and
        // breadth together could never say so. The readings are the traits
        // rather than the surface, because what separates volume from
        // projection is the BREADTH and a section reports one number for it.
        let base = ChestTraits::of(&Composites::default());
        let volume = base.on(ChestAxes {
            volume: 1.0,
            ..Default::default()
        });
        let shape = base.on(ChestAxes {
            projection: 1.0,
            ..Default::default()
        });
        let lift = base.on(ChestAxes {
            lift: 1.0,
            ..Default::default()
        });
        // Volume is size and only size.
        assert!(volume.projection > base.projection * 1.3);
        assert!((volume.spread - base.spread).abs() < 1e-6);
        assert!((volume.height - base.height).abs() < 1e-6);
        // Projection trades one against the other, in opposite directions.
        assert!(shape.projection > base.projection);
        assert!(shape.spread < base.spread);
        // And it is a TRADE rather than a second volume knob: a lobe's volume
        // goes as its height times the square of its breadth, so the two
        // coefficients are unequal on purpose and what is left over is small
        // beside what `volume` itself does.
        let volume_of = |traits: &ChestTraits| traits.projection * traits.spread * traits.spread;
        let drift = (volume_of(&shape) / volume_of(&base) - 1.0).abs();
        assert!(
            drift < 0.10,
            "the shape axis moved the volume by {:.0}%, which makes it a second size axis",
            drift * 100.0
        );
        // Lift moves where it sits and nothing about how much there is.
        assert!(lift.height > base.height);
        assert!((lift.projection - base.projection).abs() < 1e-6);
        assert!((lift.spread - base.spread).abs() < 1e-6);
    }

    #[test]
    fn a_record_written_before_the_chest_axes_still_means_what_it_meant() {
        // **The compatibility direction, and it is the one a schema test
        // usually forgets** (#211, #212). A share code minted at version 7 has
        // no chest bytes on it; what it MEANT is the neutral offsets — the
        // chest its composites came to — rather than a gap to be guessed at,
        // exactly as a pre-composites code means the neutral composites.
        //
        // Built through the real version-7 encoder shape rather than by hand,
        // so the fixture cannot drift out of step with the layout it is
        // testing.
        use crate::plan::{Archetype, HumanoidParams};
        let params = HumanoidParams {
            height: 1.83,
            shoulder_width: 0.4,
            head_breadth: -0.6,
            extremity_size: 0.25,
            chest_volume: 0.9,
            chest_projection: -0.7,
            chest_lift: 0.5,
            ..Default::default()
        };
        let mut wire = Vec::new();
        Archetype::Humanoid(params).encode(&mut wire);
        // A version-7 payload is today's with the whole chest cut off the end
        // of it: three axes appended at version 8 and two more at version 9.
        // A literal rather than something derived from the layout, because it
        // is precisely what the older arm must NOT read — a count taken from
        // the encoder would agree with whatever the encoder did next.
        let pre_chest = &wire[..wire.len() - 5];
        let mut reading = pre_chest;
        let Ok(Archetype::Humanoid(back)) = Archetype::decode_pre_chest(&mut reading) else {
            panic!("a version-7 payload did not decode");
        };
        assert!(reading.is_empty(), "the version-7 arm left bytes behind");
        assert!((back.height - params.height).abs() < 0.002);
        assert!((back.shoulder_width - params.shoulder_width).abs() < 0.03);
        assert!((back.head_breadth - params.head_breadth).abs() < 0.03);
        assert!((back.extremity_size - params.extremity_size).abs() < 0.03);
        assert_eq!(
            (
                back.chest_volume,
                back.chest_projection,
                back.chest_lift,
                back.chest_spacing,
                back.chest_fullness,
            ),
            (0.0, 0.0, 0.0, 0.0, 0.0),
            "a code written before the chest axes decoded to something other than their neutral",
        );

        // And the other direction: today's arm reads today's payload back,
        // chest and all. Without this the test above passes on a decoder that
        // has simply stopped reading the axes.
        let mut whole = wire.as_slice();
        let Ok(Archetype::Humanoid(full)) = Archetype::decode(&mut whole) else {
            panic!("a version-9 payload did not decode");
        };
        assert!(whole.is_empty(), "the current arm left bytes behind");
        assert!((full.chest_volume - params.chest_volume).abs() < 0.03);
        assert!((full.chest_projection - params.chest_projection).abs() < 0.03);
        assert!((full.chest_lift - params.chest_lift).abs() < 0.03);
    }

    #[test]
    fn a_record_written_before_the_distribution_axes_still_means_what_it_meant() {
        // The same three directions one version on (#289, and #211/#212 for
        // why three): a version-8 code carries the chest's size and height and
        // says nothing about where its volume sits, and what it MEANT is the
        // pair where `SPACING` puts it, split the way `POLES` derives — which
        // is the neutral offset, not a gap to guess at.
        use crate::plan::{Archetype, HumanoidParams};
        let params = HumanoidParams {
            height: 1.61,
            hip_width: -0.35,
            face_length: 0.8,
            chest_volume: 0.9,
            chest_projection: -0.7,
            chest_lift: 0.5,
            chest_spacing: -1.2,
            chest_fullness: 0.6,
            ..Default::default()
        };
        let mut wire = Vec::new();
        Archetype::Humanoid(params).encode(&mut wire);

        // OLD DECODES NEW: version 8 is today's payload without the two axes
        // version 9 appended.
        let mut reading = &wire[..wire.len() - 2];
        let Ok(Archetype::Humanoid(back)) = Archetype::decode_pre_distribution(&mut reading) else {
            panic!("a version-8 payload did not decode");
        };
        assert!(reading.is_empty(), "the version-8 arm left bytes behind");
        assert!((back.height - params.height).abs() < 0.002);
        assert!((back.hip_width - params.hip_width).abs() < 0.03);
        assert!((back.chest_volume - params.chest_volume).abs() < 0.03);
        assert!((back.chest_lift - params.chest_lift).abs() < 0.03);
        assert_eq!(
            (back.chest_spacing, back.chest_fullness),
            (0.0, 0.0),
            "a code written before the distribution axes decoded to something other than their \
             neutral",
        );

        // NEW DECODES NEW, without which the direction above passes on a
        // decoder that has simply stopped reading the two axes.
        let mut whole = wire.as_slice();
        let Ok(Archetype::Humanoid(full)) = Archetype::decode(&mut whole) else {
            panic!("a version-9 payload did not decode");
        };
        assert!(whole.is_empty(), "the current arm left bytes behind");
        assert!((full.chest_spacing - params.chest_spacing).abs() < 0.03);
        assert!((full.chest_fullness - params.chest_fullness).abs() < 0.03);

        // NEW DECODES OLD, which is the direction that catches an arm reading
        // at the wrong offset: a version-8 payload put through today's arm has
        // two spans too few and must fail rather than return a body.
        let mut short = &wire[..wire.len() - 2];
        assert!(
            Archetype::decode(&mut short).is_err(),
            "today's arm read a version-8 payload to the end, which means the two layouts are \
             not distinguishable by anything but the version byte",
        );
    }

    /// The lobe's own vertical profile as the carve authors it, probed through
    /// [`carve_chest`] itself.
    ///
    /// **Through the real function and not by writing its formula out again**,
    /// which would make the assertion a copy of the thing it asserts. The carve
    /// reads positions and the rig and nothing else, so a `PolyMesh` carrying
    /// only a column of probe points is a legal argument to it. The points sit
    /// on the lobe's own azimuth at the chest node's radius, which is inside
    /// the trunk and so answers `nearest_bone`'s zone gate the way the body
    /// does. `examples/chestsection` prints the same profile off the built
    /// SURFACE; this is what was asked for, and the two differ by what the mesh
    /// can hold.
    fn authored(rig: &Rig, traits: &ChestTraits) -> Vec<(f32, f32)> {
        authored_at(rig, traits, traits.spacing)
    }

    /// The same probe at any azimuth, for the terms that do not live under the
    /// lobe: [`GAP_DEPTH`] cuts the midline, which the lobe's own column never
    /// visits.
    fn authored_at(rig: &Rig, traits: &ChestTraits, spacing: f32) -> Vec<(f32, f32)> {
        let Some(column) = Column::of(rig) else {
            return Vec::new();
        };
        let angle = spacing * std::f32::consts::FRAC_PI_2;
        let out = Vec3::new(angle.sin(), 0.0, angle.cos());
        let span = column.girdle - column.waist;
        let mut probe = PolyMesh::new();
        let steps = 400;
        for step in 0..=steps {
            let up = column.waist + span * step as f32 / steps as f32;
            probe.push_vertex(
                Vec3::new(column.axis.x, up, column.axis.z) + out * (column.half * 0.5),
            );
        }
        let was = probe.positions.clone();
        carve_chest(&mut probe, rig, traits);
        // **Signed along the push, not a distance.** The fold is the one place
        // this carve pulls a vertex IN, and a magnitude cannot tell that from a
        // lobe.
        probe
            .positions
            .iter()
            .zip(&was)
            .map(|(now, before)| (before.y, (*now - *before).dot(out)))
            .collect()
    }

    #[test]
    fn the_lobe_carries_more_of_itself_below_its_peak_than_above() {
        // **45:55, which is what milestone #9 is for.** Mallucci's series and
        // the ASPS studies put the preference for that split at 87 to 94%, and
        // the symmetric Gaussian this module shipped with was 50:50 by
        // construction — see [`POLES`], which is the whole change.
        //
        // Asserted on what the carve AUTHORS rather than on what a mesh
        // delivers, because those are two findings and only one of them is this
        // module's: `examples/chestsection` reads the delivered split and
        // #285's refinement is what moves it. The authored figure overshoots
        // 45:55 on purpose and that constant's docstring says by how much.
        let record = body_of(0.0, 0.0, crate::plan::DEFAULT_BODY_FAT, 30);
        let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
        let traits = ChestTraits::of(&record.composites);
        let profile = authored(&rig, &traits);
        let (top, &(peak, most)) = profile
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
            .expect("a chest has a peak");
        assert!(most > 0.005, "no lobe to split: {:.1} mm", most * 1000.0);
        let area = |slice: &[(f32, f32)]| -> f32 {
            slice
                .windows(2)
                .map(|pair| 0.5 * (pair[0].1 + pair[1].1) * (pair[1].0 - pair[0].0))
                .sum()
        };
        // The lobe's own area, so the fold's cut below it is not counted as
        // negative volume: these are two terms and this test is about one.
        let lobe: Vec<(f32, f32)> = profile.iter().map(|&(up, at)| (up, at.max(0.0))).collect();
        let (upper, lower) = (area(&lobe[top..]), area(&lobe[..=top]));
        let share = lower / (upper + lower);
        assert!(
            (0.55..0.66).contains(&share),
            "the lobe carries {:.0}% of itself below its peak at {:.0} mm, where life prefers \
             55% and this carve overshoots it to land there on the built surface",
            share * 100.0,
            peak * 1000.0,
        );
    }

    #[test]
    fn the_two_distribution_axes_move_a_chest_without_resizing_it() {
        // **#289's two axes, and the claim is a NEGATIVE one as much as a
        // positive**: `spacing` says where the pair sits and `fullness` says
        // where its volume sits between the poles, and neither may turn into a
        // second size axis — which is the failure `the_three_axes_say_three
        // _different_things` exists to catch on the first three.
        //
        // Probed through `carve_chest` itself at every azimuth (see
        // [`authored_at`]), because the reading that matters is WHERE the lobe
        // came out and no field on the traits can answer that: the same lesson
        // that cost #285 a session, one axis on. The instrument's own
        // section-across table reads a moved pair BACKWARDS — its columns are a
        // walk across `x` and a radial push at a large azimuth widens the
        // section instead of standing it off — so this shares the carve's
        // direction rather than trusting a ruler that does not.
        let record = body_of(0.0, 0.0, crate::plan::DEFAULT_BODY_FAT, 30);
        let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
        let base = ChestTraits::of(&record.composites);
        let with = |axes: ChestAxes| base.on(axes);
        let apart = with(ChestAxes {
            spacing: 1.0,
            ..Default::default()
        });
        let close = with(ChestAxes {
            spacing: -1.0,
            ..Default::default()
        });
        let fuller = with(ChestAxes {
            fullness: 1.0,
            ..Default::default()
        });
        let evener = with(ChestAxes {
            fullness: -1.0,
            ..Default::default()
        });

        // The largest push the carve gives a column at one azimuth.
        let crest = |traits: &ChestTraits, azimuth: f32| {
            authored_at(&rig, traits, azimuth)
                .iter()
                .map(|&(_, at)| at)
                .fold(f32::MIN, f32::max)
        };
        // And where round the section that push is largest, which is the
        // delivered spacing rather than the authored one.
        let sits_at = |traits: &ChestTraits| {
            let mut best = (f32::MIN, 0.0f32);
            let mut azimuth = 0.0f32;
            while azimuth <= 1.0 {
                let at = crest(traits, azimuth);
                if at > best.0 {
                    best = (at, azimuth);
                }
                azimuth += 0.01;
            }
            best.1
        };

        // SPACING IS A PLACEMENT. It moves the pair and touches nothing else
        // the chest is made of — not its size, not its breadth, not its height,
        // and not the fold, which belongs to the height and not to the azimuth.
        assert!(apart.spacing > base.spacing && close.spacing < base.spacing);
        for moved in [apart, close] {
            assert!((moved.projection - base.projection).abs() < 1e-6);
            assert!((moved.spread - base.spread).abs() < 1e-6);
            assert!((moved.height - base.height).abs() < 1e-6);
            assert!((moved.fold - base.fold).abs() < 1e-6);
            assert!((moved.poles.0 - base.poles.0).abs() < 1e-6);
            assert!((moved.poles.1 - base.poles.1).abs() < 1e-6);
        }
        for traits in [base, apart, close] {
            let landed = sits_at(&traits);
            assert!(
                (landed - traits.spacing).abs() <= 0.02,
                "a chest sent to {:.2} of a quarter turn came out at {:.2}",
                traits.spacing,
                landed,
            );
            assert!(
                (crest(&traits, traits.spacing) / crest(&base, base.spacing) - 1.0).abs() < 0.01,
                "moving the pair changed how far it stands off, which makes this a size axis",
            );
        }

        // FULLNESS IS A SPLIT. The two poles' mean stays 1 — see [`POLES`] —
        // so `TALL` goes on meaning what its line says and the crest's own
        // amplitude cannot move: a Gaussian's crest is its coefficient whatever
        // its span is.
        assert!(fuller.poles.1 > base.poles.1 && fuller.poles.0 < base.poles.0);
        assert!(evener.poles.1 < base.poles.1 && evener.poles.0 > base.poles.0);
        for moved in [fuller, evener] {
            assert!(
                (moved.poles.0 + moved.poles.1 - 2.0).abs() < 1e-6,
                "the poles' mean moved, so the axis is standing the chest off as well as \
                 splitting it",
            );
            assert!((moved.projection - base.projection).abs() < 1e-6);
            assert!((moved.spread - base.spread).abs() < 1e-6);
            assert!((moved.spacing - base.spacing).abs() < 1e-6);
            assert!(
                (crest(&moved, moved.spacing) / crest(&base, base.spacing) - 1.0).abs() < 0.01,
                "the crest moved with the split, which is the one thing the mean-of-1 is for",
            );
        }

        // And the split it authors moves the way it was asked to. The share
        // below the peak is the same reading `the_lobe_carries_more_of_itself
        // _below_its_peak_than_above` asserts against life's 45:55.
        let below = |traits: &ChestTraits| {
            let profile = authored(&rig, traits);
            let top = profile
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.1.total_cmp(&b.1.1))
                .expect("a chest has a peak")
                .0;
            let lobe: Vec<(f32, f32)> = profile.iter().map(|&(up, at)| (up, at.max(0.0))).collect();
            let area = |slice: &[(f32, f32)]| -> f32 {
                slice
                    .windows(2)
                    .map(|pair| 0.5 * (pair[0].1 + pair[1].1) * (pair[1].0 - pair[0].0))
                    .sum()
            };
            let (upper, lower) = (area(&lobe[top..]), area(&lobe[..=top]));
            lower / (upper + lower)
        };
        let (even, mid, full) = (below(&evener), below(&base), below(&fuller));
        assert!(
            even < mid && mid < full,
            "the fullness axis authored {:.0}% / {:.0}% / {:.0}% below the peak at -1, 0 and +1, \
             which is not an ordered axis",
            even * 100.0,
            mid * 100.0,
            full * 100.0,
        );

        // **And the fold travelled with the pole it is a share of**, which is
        // the one thing `fullness` does that age must never do (#288): ptosis
        // is the peak descending toward a fold that holds still, and a creator
        // asking for a fuller lower pole is asking for the whole structure.
        assert!(
            fuller.fold < base.fold && evener.fold > base.fold,
            "the fold sat at {:.3} of the band whatever the lower pole under it was doing, \
             which is `FOLD_SEAT` written as a constant seat after all",
            base.fold,
        );
    }

    #[test]
    fn the_fold_sits_under_the_lobe_and_dies_with_it() {
        // **The seat is the reading, not the depth** (#286). Life anchors the
        // inframammary fold about 70 mm below the peak on a reference-scale
        // body, and [`FOLD_SEAT`] is written as a share of the lower pole's own
        // span so that lift and the age descent carry it with the lobe instead
        // of leaving it where the reference body's chest used to be — #182's
        // lesson, which is that a constant seat dies quietly when a ruler
        // moves.
        // **On a LEAN body, because that is where a crease is a crease.**
        // [`FOLD_DEPTH`] follows leanness — see its own note — so on a soft
        // chest the lobe's own tail still out-reaches the trough and the net
        // displacement never turns inward, which is a true statement about a
        // soft body rather than a missing fold. The sign test wants the body
        // the term is strongest on.
        let record = body_of(1.0, 0.0, 0.05, 30);
        let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
        let traits = ChestTraits::of(&record.composites);
        let profile = authored(&rig, &traits);
        let peak = profile
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("a chest has a peak")
            .0;
        // Found by SIGN and not by a threshold: the fold is the one place this
        // carve pulls a vertex in rather than pushing it out, so the deepest
        // negative displacement under the peak is the crease and nothing else
        // can be mistaken for it.
        let (seat, deepest) = profile
            .iter()
            .filter(|&&(up, _)| up < peak)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|&(up, at)| (peak - up, at))
            .expect("there is a body below the peak");
        assert!(
            deepest < 0.0,
            "nothing under the lobe pulls in; the deepest reading is {:.2} mm",
            deepest * 1000.0,
        );
        assert!(
            (0.050..0.095).contains(&seat),
            "the fold sits {:.0} mm below the peak, where life puts it at about 70; it cuts \
             {:.2} mm",
            seat * 1000.0,
            deepest * 1000.0,
        );

        // And it dies with the chest it belongs to: a flat chest has no fold,
        // and it needs no threshold to say so because the depth is a share of
        // the projection.
        let flat = ChestTraits {
            projection: 0.0,
            ..traits
        };
        let none = authored(&rig, &flat);
        assert!(
            none.iter().all(|&(_, moved)| moved.abs() <= f32::EPSILON),
            "a chest with no projection still moved {:.3} mm of trunk",
            none.iter()
                .fold(0.0f32, |most, &(_, at)| most.max(at.abs()))
                * 1000.0,
        );
    }

    #[test]
    fn a_lean_chest_is_firmer_and_carries_a_sternal_gap() {
        // **#287's two terms, and both are zero on the neutral body by
        // construction** — `Condition::default().definition` is 0 — so this is
        // the only place they can be asserted at all. That is deliberate: a
        // pectoral's defining feature is leanness showing through it, and a
        // term that acted on the default body would be re-basing every avatar
        // to say so.
        let lean = body_of(-1.0, 0.0, 0.05, 30);
        let soft = body_of(-1.0, 0.0, 0.40, 30);
        let rig = Rig::from_skeleton(&lean.skeleton()).expect("rigs");
        let of = |record: &AvatarRecord| ChestTraits::of(&record.composites);

        // THE BORDER. A pectoral's edge is where its mass stops, so the reading
        // is the steepest fall on the lobe's own lower flank — measured on what
        // the carve authors, at equal projection, because a bigger chest has a
        // steeper flank for reasons that are not firmness.
        let flank = |traits: &ChestTraits| -> f32 {
            let profile = authored(&rig, traits);
            let peak = profile
                .iter()
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .expect("a chest has a peak");
            let most = peak.1.max(f32::EPSILON);
            profile
                .windows(2)
                .filter(|pair| pair[1].0 <= peak.0)
                .map(|pair| ((pair[1].1 - pair[0].1) / (pair[1].0 - pair[0].0)).abs() / most)
                .fold(0.0f32, f32::max)
        };
        let (crisp, rise) = (flank(&of(&lean)), flank(&of(&soft)));
        assert!(
            crisp > rise * 1.15,
            "a lean chest's lower border fell at {crisp:.2} against a soft {rise:.2}, per unit of \
             projection — which is not the difference between an edge and a rise",
        );

        // THE GAP. A narrow trough on the midline, which is an azimuth the lobe
        // itself never reaches: [`SPACING`] puts the pair at 0.57 and a
        // Gaussian three spreads away is two parts in a thousand. So anything
        // that moves here is this term and nothing else.
        let midline = |traits: &ChestTraits| -> f32 {
            authored_at(&rig, traits, 0.0)
                .iter()
                .fold(0.0f32, |most, &(_, at)| most.min(at).min(most))
        };
        assert!(
            midline(&of(&lean)) < -1e-4,
            "a lean chest cut {:.3} mm of sternal gap, which is none",
            midline(&of(&lean)) * 1000.0,
        );
        // A tenth rather than nothing, because `Condition::definition` is a
        // ramp and not a switch: at 40% fat on a masculine frame it has not
        // quite reached zero, and asserting an exact zero would be asserting
        // where that ramp's foot is rather than what this term does.
        assert!(
            midline(&of(&soft)) > midline(&of(&lean)) * 0.1,
            "a soft chest cut {:.3} mm of sternal gap against a lean {:.3}, which is not the \
             difference between a leanness reading and a body shape",
            midline(&of(&soft)) * 1000.0,
            midline(&of(&lean)) * 1000.0,
        );

        // And the neutral body barely moves here, which is what keeps #286's
        // re-base the last one. **Barely and not "not at all", and the
        // difference is a finding**: `Condition::of` reads exactly 0 definition
        // at the default composites, so [`GAP_DEPTH`] is exactly zero there —
        // but this probe reads the whole carve at the midline, and what moves
        // it is [`FOLD_DEPTH`]'s own crescent, which reaches the sternum at a
        // tenth of its strength because a fold runs inward across the base of
        // the chest. That is the fold doing its job, so the reading is a ratio
        // rather than a zero — measured at 13% of the lean body's, and the bar
        // is a quarter.
        let neutral =
            ChestTraits::of(&body_of(0.0, 0.0, crate::plan::DEFAULT_BODY_FAT, 30).composites);
        assert!(
            midline(&neutral) > midline(&of(&lean)) * 0.25,
            "the neutral body's midline moved {:.4} mm against a lean body's {:.4}",
            midline(&neutral) * 1000.0,
            midline(&of(&lean)) * 1000.0,
        );
    }

    #[test]
    fn ptosis_is_the_peak_descending_toward_a_fold_that_does_not_move() {
        // **The Regnault definition, which is what #288 is for.** Ptosis is
        // graded by where the nipple sits relative to the INFRAMAMMARY FOLD,
        // and the fold is a ligamentous structure attached to the chest wall:
        // it does not descend, the breast descends toward it. What this module
        // shipped before was pseudoptosis only — the volume bottoming out —
        // and worse, the fold was derived off the peak, so measured at
        // femininity `+1` the peak fell 52 mm across the age range and the
        // fold fell 56 mm with it. Nothing ever approached anything.
        let at = |age: u32| ChestTraits::of(&body_of(1.0, 0.0, DEFAULT_BODY_FAT, age).composites);
        let (young, old) = (at(30), at(80));

        assert!(
            (young.fold - old.fold).abs() < 1e-6,
            "the fold moved from {:.4} to {:.4} of the band as the body aged, and a fold is \
             attached to a ribcage",
            young.fold,
            old.fold,
        );
        assert!(
            old.height < young.height - 0.05,
            "the peak sat at {:.3} of the band young and {:.3} old, which is not a descent",
            young.height,
            old.height,
        );
        // And it CLOSES on the fold rather than merely moving: the two are the
        // same distance apart on a body that is not ageing at all if the peak
        // and the fold are both carried by the same term, which is the bug
        // above wearing a passing test.
        let (before, after) = (young.height - young.fold, old.height - old.fold);
        assert!(
            after < before * 0.6,
            "the peak stood {:.3} of the band above its fold young and {after:.3} old, which is \
             not a breast approaching one",
            before,
        );

        // **Never grade 3**, which on a stylised body does not read as age but
        // as broken. The age axis alone does not reach it; a negative lift axis
        // is what tries, and the roll envelope goes to -3.
        for age in [30u32, 55, 80] {
            for lift in [-3.0f32, -1.0, 0.0, 1.0, 3.0] {
                let traits = at(age).on(ChestAxes {
                    lift,
                    ..Default::default()
                });
                assert!(
                    traits.height >= traits.fold,
                    "at age {age} and lift {lift:+} the peak sat {:.3} of the band against a fold \
                     at {:.3}, which is Regnault grade 3",
                    traits.height,
                    traits.fold,
                );
            }
        }

        // LIFT RESISTS ALL OF IT THROUGH ONE AXIS, which is what keeps this a
        // body ageing rather than three timetables that can disagree: the
        // descent it refuses is the same number that lowers the peak and
        // slackens the lower pole.
        let lifted = old.on(ChestAxes {
            lift: 1.0,
            ..Default::default()
        });
        assert!(
            lifted.descent < old.descent,
            "lift did not refuse the descent: {:.3} against {:.3}",
            lifted.descent,
            old.descent,
        );
        assert!(
            lifted.height > old.height + 0.05,
            "lift raised an eighty-year-old peak from {:.3} to {:.3}, which is the offset alone \
             and not the ptosis it declined",
            old.height,
            lifted.height,
        );
    }

    #[test]
    fn the_neutral_body_has_a_chest() {
        // **The re-base, as a ratchet.** This module deliberately changes the
        // body a record built at neutral composites produces — see the module
        // note — and a deliberate change to the neutral body is worth a number
        // that cannot drift back. Life's bands are a pectoral at 10 to 20 mm
        // and a bust at 40 to 90; the midpoint of the two references sits
        // between them.
        //
        // **Re-based 14–28 → 24–36 at #293**, with the two constants it is the
        // midpoint of. `femininity` is 0 at the neutral, so this reading is
        // exactly `(PECTORAL + BUST) / 2` in millimetres: the mid of a 15 mm
        // pectoral and a 45 mm bust is 30, and it reads 29.8. The old band was
        // built on values that sat under both of life's, and on the belief that
        // a caliper reads about twice the authored push — which #284 refuted by
        // finding that reading was the flank of a widening section rather than
        // the lobe. See `PECTORAL` for the whole of it.
        let record = body_of(0.0, 0.0, crate::plan::DEFAULT_BODY_FAT, 30);
        let (plain, carved) = pair(&record);
        let rig = Rig::from_skeleton(&record.skeleton()).expect("rigs");
        let authored = pushed(&plain, &carved, &rig) * 1000.0;
        assert!(
            (24.0..36.0).contains(&authored),
            "the neutral body's chest was pushed {authored:.1} mm, which is not the midpoint of \
             a pectoral and a bust — `examples/chestsection` reads the thickness a caliper \
             would take along the push itself",
        );
    }

    #[test]
    fn a_chest_survives_every_body_the_roll_envelope_reaches() {
        // A carve cannot make a body fail to mesh — it is applied after the
        // cage — so what this asks is the other thing: that no body in the
        // envelope comes out with a chest that is not a chest. The bound is
        // generous on purpose; it is a guard against a formula going unbounded
        // on a corner, not a second calibration.
        for seed in 0..120i64 {
            let mut record = AvatarRecord::new("Rolled", Archetype::default());
            record.reroll(seed);
            record.sanitize();
            // **With the record's own axes on**, which is the whole point of
            // sweeping rolled seeds rather than composites: the randomiser
            // reaches chestVolume from -2.99 to +1.54 over 400 seeds, and a
            // volume that negative asks for a projection below zero.
            let axes = match &record.archetype {
                crate::plan::Archetype::Humanoid(params) => ChestAxes {
                    volume: params.chest_volume,
                    projection: params.chest_projection,
                    lift: params.chest_lift,
                    spacing: params.chest_spacing,
                    fullness: params.chest_fullness,
                },
                _ => ChestAxes::default(),
            };
            let traits = ChestTraits::of(&record.composites).on(axes);
            assert!(
                traits.projection.is_finite() && (0.0..1.0).contains(&traits.projection),
                "seed {seed} asked for a chest {:.3} radii off its ribcage",
                traits.projection
            );
            assert!(
                traits.spread > 0.05 && traits.spacing > 0.2 && traits.spacing < 0.9,
                "seed {seed} placed its chest at {:.2} of a quarter turn, spread {:.2}",
                traits.spacing,
                traits.spread
            );
            // And the pair the split is written as: a pole is the span of a
            // falloff, so a span at or below zero is a step in the surface
            // rather than a shape (#289).
            assert!(
                traits.poles.0 >= POLE_FLOOR && traits.poles.1 >= POLE_FLOOR,
                "seed {seed} split its lobe {:.2}:{:.2}, which is not a pair of spans",
                traits.poles.0,
                traits.poles.1,
            );
        }
        // And the plan the carve rides is untouched by it, which is the whole
        // reason this mechanism was chosen: `HumanoidParams` still meshes.
        let params = HumanoidParams::default();
        assert!(
            build_body(
                &params.skeleton(&Composites::default()),
                &CageConfig::default(),
                BODY_SUBDIVISIONS,
                &crate::face::HeadTraits::default(),
            )
            .is_ok()
        );
    }
}
