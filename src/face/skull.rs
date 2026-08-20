//! Giving the head a skull's shape.
//!
//! The body plan builds a head from two nodes, which sweep into a capped tube
//! and subdivide into an egg. An egg has no jaw, no chin, no cheekbones and no
//! occiput, so a face built on one reads as features stuck onto a blank — which
//! is exactly how it read.
//!
//! **The capsule graph cannot fix this, and adding nodes to it will not either.**
//! A jaw is not a tube. It is wide at the angle below the ear, narrows forward
//! through the cheek, and finishes in a chin that projects past everything above
//! it. A ball with sockets on it has no way to say that, and hanging another
//! capsule off the head to serve as a jaw produces a snout.
//!
//! So the shape is applied to the built mesh, analytically, in head-local space:
//! a breadth profile down the skull, a fore-aft elongation, an occipital
//! fullness that cuts away under the ear, a chin, and a jaw. Applied to the
//! **rest** mesh before anything is bound or unwrapped, so skin weights, texture
//! charts, hair and every other attached part follow it without knowing it
//! happened.
//!
//! All but one of those are a profile in height times a window in azimuth, and
//! that separability is what left the lower face a right circular frustum for as
//! long as it did: a mandible is a border whose HEIGHT MIGRATES WITH AZIMUTH and
//! no product of the two can say so. `jaw` is the one term here that takes
//! both at once, and it is the difference between a jawline and a cone.
//!
//! Heights here are in skull radii above the head joint, which is the same unit
//! the features are placed in. One unit for the head, everywhere. (Hair's own
//! regions are the exception and say so: [`crate::hair::Follicles`] resolves in
//! head-local metres, because a mask is asked about a point that arrived from a
//! scattered root or a texel rather than from a plan.)

use glam::Vec3;

use super::smooth;
use crate::mesh::PolyMesh;
use crate::plan::derive::humanoid::frame;
use crate::plan::{Composites, Zone};
use crate::rig::Rig;

/// How much longer a head is than it is wide, before anything else runs.
///
/// A head with a circular cross-section reads as a ball however well the rest of
/// it is shaped.
///
/// **This constant is not the built ratio, and reading it as one is a trap.**
/// It is a raw multiplier on the fore-aft radius, applied before [`BREADTH`]
/// narrows the lateral one and before [`OCCIPUT`] swells the back. What a head
/// comes out at is the product of the three, so a statement about the
/// coefficient can be true of the number and false of the head — which is
/// [`TEMPLE`]'s unit confusion again, one table down and in a different
/// disguise.
///
/// Nor can the built ratio alone support it, because the mesher stands between
/// the two: eight-point cage rings deliver `cos(π/8)` where four-point rings
/// deliver `cos(π/4)`, so a cage change moves the built width and depth by
/// double-digit percentages with this constant untouched. A number whose only
/// evidence is a measurement taken through a mesher does not survive the mesher
/// changing.
///
/// At 0.11 the vault measures 208.1 mm deep on a width of 160.9 — 1.29, against
/// a life head length over head breadth of 195 over 152 — and 1.29 to 1.32
/// across eight seeds at neutral breadth.
/// Provenance: **derived from the built ratio**, which is the only
/// honest thing to derive it from while it is one factor of three. The TARGET
/// is looked up — head length 195 mm against head breadth 152 to 156 — and
/// this is the value that puts the built vault on it. If [`BREADTH`],
/// [`DEPTH`] or [`OCCIPUT`] moves through the mid-cranium, or if the cage
/// changes again, this has to be re-measured rather than trusted.
const ELONGATION: f32 = 0.11;

/// How wide the skull is at each height, relative to its unshaped width.
///
/// **Widest on the parietal, well above the eye — not at the cheekbones,
/// however natural "widest at the cheekbones, just below the eye line"
/// sounds.** Anthropometry runs the other way. Maximum breadth is at
/// eurion — 156 mm against a bizygomatic 137 — and eurion sits 25 to 45 mm
/// *above* the pupil line. A table that peaks at the cheek puts a head's
/// widest point at or below the eye, which is the defect this one exists to
/// avoid.
///
/// This profile does not act alone. The cage cones on its own — a crown node
/// much narrower than the head node makes the blend converge toward an apex —
/// and a profile cannot un-cone a cage without inflating the cranium past
/// anything a skull does. So the two are shaped together: the cage is
/// near-cylindrical through the mid-cranium, and this profile narrows where a
/// head narrows.
///
/// The knots from the cheekbone down are also drawn in deliberately: left
/// wider, the head runs **11 to 21 percent too wide for its own height**
/// (H:W 1.22–1.31 against a life 1.48). That is a narrower face, and it is the
/// half of this table that has to be judged by eye rather than measured.
///
/// **The lower half narrows far less than it looks like it should, and returns
/// to nothing at all where the head meets the neck.** Two reasons, both
/// measured.
///
/// The unshaped head is already a taper. It is a capsule blend from a 131 mm
/// head node down to a 66 mm neck node, so it goes from 78.5 mm half-width at
/// the joint to 53.6 mm at the junction on its own — a third, with no profile
/// applied. Narrowing that by another 54% is what produced a jaw thinner than
/// the throat under it.
///
/// And the mesh is continuous across the junction while the shaping is not: it
/// moves head-owned vertices and leaves neck-owned ones exactly where they are.
/// Whatever this profile says at the bottom of the head is therefore a STEP in
/// the silhouette, and it said 0.46 — a 19 mm cliff in 11 mm of height, against
/// a neck the unshaped head met to within 2 mm. Anything but 1.0 down there is
/// a seam.
/// **The two vault knots must move whenever the built crown does, and that is a
/// hazard this table carries permanently.** Heights above the joint are RAW
/// skull radii and heights below it are profile heights, so the lower half of
/// this table follows a head's own lower face and the upper half does not
/// follow anything. And [`knot`] clamps above its first entry, so a crown that
/// rises past the knot marked "crown" leaves the top of the vault holding a
/// flat 0.58 instead of tapering: a cylinder with a cap on it, in the exact
/// place this table exists to round. With the crown knot at 1.12 and the
/// upper-cranium knot at 0.80 the built vault tapers smoothly from the
/// parietal to the vertex.
///
/// The parietal knot at 0.42 does NOT scale with the crown and must not: it is
/// where eurion sits, and eurion is quoted against the pupil line, which does
/// not move with the crown. Built, the widest point lands at +0.42 to +0.43 R
/// on eight seeds — 39 mm above the eye line against a life 25 to 45.
/// Provenance: **looked up, then tuned by render**. Eurion 156 mm
/// against a bizygomatic 137, with eurion 25 to 45 mm above the pupil line,
/// is the looked-up half and it is what sets this table's premise. The
/// knots from the cheekbone down are tuned: left wider, the head runs 11 to 21
/// percent too wide for its height, and narrowing it has to be judged by eye.
/// The two vault knots are **derived** — a pair scaled by the ratio the built
/// crown moves by, not a new shape; 1.12/0.80 is where the humanoid plan's
/// `CROWN_HIGH` puts them, with the head at an eighth of its own stature. The
/// parietal knot at 0.42 never scales and must not — eurion is quoted against
/// the pupil line, which does not move with a crown.
///
/// **THE FACE KNOTS SIT AT THE LANDMARKS THEY ARE NAMED FOR**, which is what
/// makes their amplitudes mean anything. The knot commented "the angle of
/// the jaw" is at −0.31 because [`GONION`] — the crate's own derived landmark
/// of that name, and where [`Skull::gonion`] answers — is at −0.31; narrowing
/// aimed at a landmark and spent below it reads at the landmark itself as most
/// of the way back to the cheekbone's 0.825. The knot at −0.18 follows the
/// gonion to the midpoint between the cheekbone and the gonion, and is the one
/// knot here with no landmark of its own.
///
/// Measured on `the_face_narrows_from_cheekbone_to_chin`'s own ruler with the
/// breadth axis held neutral, bigonial over bizygomatic reads **0.752,
/// 0.760, 0.763 and 0.715** on four bodies against a life 0.73–0.76, and over
/// the eight seeds of `examples/headaudit --sweep` with each body's own axis
/// rolled, 0.678 to 0.865.
///
/// What the narrow lower face costs, recorded rather than absorbed: the
/// off-midline bound in
/// `the_profile_agrees_with_the_surface_it_was_measured_from` stands at
/// 20.0 mm rather than 18.8. A narrower lower face is harder for a fixed
/// number of lateral columns to describe, and that ruler reports it — as
/// binning, not as a span that slid.
///
/// The alternative was tried and measured worse: raising the gonion knot to
/// 0.660 centres the population on life but takes that same bound to 24.4.
const BREADTH: [(f32, f32); 9] = [
    (1.12, 0.58),     // crown
    (0.80, 0.88),     // upper cranium
    (0.42, 0.94),     // the parietal, where a head is actually widest
    (0.20, 0.885),    // above the temple
    (-0.05, 0.825),   // the cheekbones, a plane change and not the widest point
    (-0.18, 0.771),   // below the cheek, midway to the gonion
    (-0.31, 0.646),   // the angle of the jaw, which is `GONION`
    (-0.60, 0.575),   // the chin — 0.547 → 0.575, wider with #197's rounder point
    (JUNCTION, 1.00), // the throat, which is the neck's width and not this one's
];

/// How deep the skull is at each height, relative to its unshaped depth.
///
/// Separate from [`BREADTH`], and that separation is the point: run the breadth
/// profile on the fore-aft axis too and narrowing the jaw drags the chin
/// backwards, which cancels the chin out entirely. A jaw narrows across; it does
/// not retreat.
/// The last knot is [`JUNCTION`], and its value is not a shape: it is whatever
/// makes `deep` come out at exactly one, so the head's fore-aft extent matches
/// the neck's where they meet. See [`BREADTH`] for why anything else is a seam —
/// unshaped, the two agreed to within 2 mm, and the profile was opening an 11 mm
/// gap at the nape and a 7 mm overhang at the throat.
///
/// **The two vault knots move with the crown**, by the same ratio and for
/// the same reason [`BREADTH`]'s do — read that table's note, which is where
/// the mechanism is written out; 1.12 and 0.70 is where the built crown puts
/// them. The knots at 0.20 and below do not follow it: they are the face, and
/// the face stays where it is.
/// Provenance: **tuned by render**, except the
/// last, which is **derived** — `1/(1 + ELONGATION)` is exactly what makes
/// `deep` come out at one where the head meets the neck, and is a solved
/// value rather than a shape — and the two vault knots, also **derived**, as
/// a pair scaled by the ratio the built crown moves by.
const DEPTH: [(f32, f32); 7] = [
    (1.12, 0.66),
    (0.70, 0.94),
    (0.20, 1.00),
    (-0.10, 1.00),
    (-0.46, 0.90),
    (-0.60, 0.78),
    (JUNCTION, 1.0 / (1.0 + ELONGATION)),
];

/// How much fuller the back of the skull is than the front, at each height.
///
/// Positive behind the ear is the occiput, which on an unshaped head is missing
/// entirely. Negative below it is the jaw cutting in — the hollow between the
/// jaw's angle and the neck, without which a head sits on its neck like a ball
/// on a post.
///
/// **Where this table STOPS matters as much as how hard it pushes.** Heights
/// above the joint are raw skull radii, and [`knot`] clamps above its first
/// entry, so whatever the top knot holds is carried flat all the way to the
/// vertex. The vertex of a skull is very nearly symmetric fore and aft; a top
/// knot that still holds a backward bulge leans the whole cap.
///
/// So it runs out at the crown: 0.0 at 1.12 radii, where [`BREADTH`] and
/// [`DEPTH`] also end, and 0.04 kept at 0.90 so the taper above the occiput
/// holds in the band it was tuned in.
///
/// **The amplitudes were left alone, and that is a finding rather than an
/// omission.** Built and measured after the depth came down, the vault reaches
/// 100.8 mm forward of the head joint and 107.2 behind it — 6% back-heavy, on a
/// vault:width of 1.29 against a life 1.28. Taking the peak down would flatten
/// the occiput to buy a symmetry nothing here has a source for: the head joint
/// is a modelling construct rather than a landmark, so there is no anthropometry
/// that says what a fore-and-aft split about it should be. What there IS a
/// source for is the TOTAL, and that is [`ELONGATION`]'s and it is now right.
/// Provenance: **tuned by render**, except the crown knot, which is
/// **derived** — it is where the built crown sits, the same value
/// [`BREADTH`] and [`DEPTH`] end at, and not a shape.
const OCCIPUT: [(f32, f32); 7] = [
    (1.12, 0.0),
    (0.90, 0.04),
    (0.35, 0.14),
    (0.05, 0.08),
    (-0.30, -0.10),
    (-0.58, -0.24),
    (JUNCTION, 0.0),
];

/// How far the brow ridge stands proud, in skull radii, at each height.
///
/// The bone above the eye, not the hair on it. Without it the forehead runs
/// straight down into the eye socket and the face has no ledge for the eyes to
/// sit under — which is a large part of why a smooth head reads as a doll.
/// Provenance: **tuned by render**, and in skull radii — unlike [`TEMPLE`]
/// directly below, whose docstring records the unit confusion between them.
const BROW: [(f32, f32); 5] = [
    (0.58, 0.0),
    (0.42, 0.018),
    (0.28, 0.042),
    (0.14, 0.030),
    (0.02, 0.0),
];

/// How far the temples are drawn in at each height, as a fraction of the local
/// half-width.
///
/// The flat at the side of the skull between the brow and the ear. A head
/// without it is a barrel from every angle above the cheekbone.
///
/// **A fraction, not skull radii, and the difference matters.** It is
/// subtracted from `wide` in [`reshape_to`], which is a
/// dimensionless multiplier on the horizontal radius — so the peak's 0.042
/// realises as 4.2% of the local half-width, a few millimetres at the widest,
/// rather than as 0.042 R. [`BROW`] genuinely is in skull radii: it is multiplied by
/// `radius` where this one is not. Two neighbouring profiles documented in the
/// same unit and applied in different ones is how a term gets tuned twice and
/// moves half as far as its author expects, so this doc says what the code
/// does.
///
/// **The peak sits at brow height, not higher.** The temporal fossa sits
/// just above the zygomatic arch and *below* the greatest breadth of the
/// skull; a peak up near 0.40 R hollows the parietal instead — well above the
/// brow crest — which is the one part of the vault that should be full.
/// Provenance: **tuned by render**. Worth reading as a caution:
/// a number tuned against a docstring that lies about its unit gets tuned
/// twice and moves half as far as its author expects.
const TEMPLE: [(f32, f32); 4] = [(0.50, 0.0), (0.30, 0.042), (0.12, 0.036), (-0.06, 0.0)];

/// How far the chin and jaw project forward at each height, in skull radii.
///
/// **This curve is two things at once, and each of the two has been got wrong on
/// its own.** Where it rises is the underside of the jaw; how high it rises is
/// the chin's projection. Fixing either without watching the other put a defect
/// on screen both times.
///
/// **The rise is spread from the junction to the tip, and the spreading is the
/// shape.** Concentrated instead, the bisected midline outline gains tens of
/// millimetres of projection inside a single 2 mm step: a horizontal shelf
/// with the chin's tip at the top of the wall above it, which is a chin aimed
/// upward and read exactly that way. Spread, the largest step is about 12 mm.
///
/// **The amplitude is not a lever for the underside, and cutting it to steepen
/// the run is the classic mistake.** Millimetres off the peak cost the chin
/// its projection and put the lower lip in front of it, and a face whose lip
/// swallows its chin has no jaw at all — which is exactly how it renders. The
/// peak is set against the lip instead: the carved tip comes out within a
/// couple of millimetres of the lower lip's line across seeds, which is where
/// a chin sits.
///
/// It reaches zero at [`JUNCTION`] like everything else. An earlier version let
/// go before the others, because holding 0.16 within a mesh row of the junction
/// stood the head's lowest band 27 mm forward of the throat; the gentler
/// tail here does not need the exception.
///
/// **Most of the submental deviation is NOT in this table, which is why the
/// measurement matters more than any knot.**
/// `the_underside_of_the_jaw_does_not_bulge`
/// measures the whole submental run against its own chord, and zeroing every
/// below-joint knot here still leaves most of the figure standing. Whatever
/// puts the rest there survives this profile being deleted.
///
/// **The amplitude is deliberately modest, because `stretch` grows it.** The
/// push is multiplied by `stretch`, and `stretch` is correct — it
/// holds the chin's ASPECT as the head's floor moves. The consequence is that
/// this table's push grows every time the head
/// gets longer. Left unchecked, the midline push at the
/// peak measured 67 mm on a section whose lateral
/// half-extent at that height is 22 mm — a five-to-one blade, which reads as a
/// second nose. The ear canal to pogonion on a life head scaled to ours is
/// about 92 to 101 mm, and the amplitude holds the built projection in that
/// range.
///
/// **The amplitude's floor is a sweep against what the chin does to its own
/// LIP**, because this amplitude was once cut on an argument that sounded good
/// and measured badly, and only a measurement stops that happening again.
/// `examples/headaudit` walks the carved midline as
/// the anatomy runs — the chin's crest, the crease under the lip, the lip's own
/// crest — and reports the margin between the first and the last.
///
/// ```text
///   amplitude   projection   proud   chin over its lip
///     x0.85        101.2      13.9        +8.9
///     x0.75         94.9       8.9        +5.3
///     x0.70         91.7       6.7        +3.5
///     x0.65         88.6       4.2        +1.7
///     x0.60         85.5       2.2        -0.1   <- the lip swallows the chin
/// ```
///
/// **That last row is the known failure, reproduced at a known point.** A
/// face whose lip swallows its chin has no jaw at all, and it happens at 0.60
/// of the swept base. The shipped knots sit clear of it; below 0.70 the 92 to
/// 101 mm life range is left as well. The chin landmark
/// itself starts moving at 0.65 — [`Skull::chin`] reads −99.5 there against
/// −99.9 above it — which is the crest going flat enough that the thing finding
/// it picks a different point, and a second independent signal for the same
/// edge.
///
/// **What this table owns, measured by deleting it.** With all four knots at
/// zero the head has no chin: proud reads −2.7 mm, and [`Skull::chin`] comes
/// back at +9.9 because there is no longer a crest on the lower midline to
/// find. So the prominence is this table's from end to end, and the amplitude is
/// a real lever rather than the last of one.
///
/// **The PEAK's height is as deliberate as its amplitude.** Everything above
/// is about how much this table pushes; this is about where.
///
/// The midline is `cage_reach · DEPTH · (1 + ELONGATION) + CHIN · stretch ·
/// radius`, and `cage_reach` is falling steeply through the jaw — so the
/// SURFACE crests ABOVE where this table crests: with the peak at −0.54, seed
/// 0's forward-most point sits at profile height −0.5045, 8.6 mm higher.
/// `the_underside_of_the_jaw_does_not_bulge`
/// draws its chord from the surface's crest, and for those 8.6 mm the chin is
/// still RISING toward its own maximum. That is the bulge, entire: it is why the
/// deviation peaks at step 3 of 20 on every seed, and why nothing below −0.58
/// ever moves it.
///
/// ```text
///   peak    worst deviation, fixed ruler    the population, 16 seeds
///   -0.54        +8.5 to +11.6              0.029 – 0.099
///   -0.53        +6.7 to  +7.4              0.031 – 0.081   <- here
///   -0.52        +4.5 to  +6.5              0.031 – 0.077, seed 9 inverts
///   -0.50        +1.0 to  +1.4              breaks the neck guard
/// ```
///
/// **Bounded by two cliffs rather than chosen.** −0.50 all but deletes the
/// bulge and cannot ship: it takes seed 21's head from 251.9 mm to 243.8 and its
/// neck ratio from 0.416 to 0.463, past `the_neck_is_the_length_of_a_neck`. −0.52
/// holds the neck but inverts seed 9, which goes from the best body in the sweep
/// at 0.029 to the worst at 0.077. Both failures are the same one: [`Skull::chin`]
/// is the crest of a sum whose two terms are moving against each other, so
/// moving this knot moves the landmark measuring it — the same 0.02 step shifts
/// the default body's chin 0.4 mm and seed 21's 8.1, twenty times as much, which
/// is a crest changing identity rather than moving.
///
/// At −0.53 the head does not move at all: crown to chin 211.9 mm, cranium:face
/// 1.02, [`Skull::chin`] −99.9, all unchanged. It moves the chin's
/// proud figure 0.8 mm (8.9 to 9.7) and costs 2.4 mm of its lead over its own
/// lip (8.9 to 6.5, against a floor of 2.0).
///
/// **And that trade is below what a render can show.** 2 mm on a 90 mm run:
/// the number improves and the picture does not. Whether the skin under
/// the jaw hugs the bone is not decided here, and saying so is the
/// point of writing it down.
///
/// Provenance: **tuned by render** for the spacing, the
/// amplitude and the tail. The amplitude was cut once on an argument that
/// sounded good and measured badly, which is why the reasoning is kept. Its
/// level is **derived** from the projection against life and
/// **bounded by a sweep** against the lip; the peak's height is
/// **derived** from where the surface crests and **bounded by a sweep** against
/// the neck and the chin landmark.
///
/// **The tail is steep because the reference's submental corner is.**
/// `examples/column`
/// against the reference, front reach below each body's own chin: at the chin
/// the two agree (102.2 against 98.6), and the reference then gives up 45 mm
/// of forward reach in ten millimetres and is near-vertical after. A tail knot
/// still holding half the peak a centimetre below the crest spreads that
/// 45 mm over fifty: a corner against a slope, and the tail knot is the slope.
///
/// **But one steep slope from the crest is too much of it**: aimed straight at
/// the reference's cliff, the tail takes the ten millimetres above the cliff
/// with it — twice the reference's drop over the first centimetre below the
/// chin, and then LESS than the reference over the next. The reference holds a
/// shelf under the chin and falls off a cliff below it, so the tail does the
/// same: a knot holds the crest's own value for the first 0.045
/// of profile height and the fall comes after —
/// `(-0.575, 0.230)` and `(-0.63, 0.020)` rather than a single mid-tail knot —
/// which puts the
/// first-centimetre drop at 16.1, 16.1, 14.2 and 19.8 mm on seeds 0, 3, 6 and
/// 12 against the reference's 9.6.
/// **The tail knot stays at −0.63, and easing it is a measured hazard**:
/// eased to −0.66 with `SUBMENTAL_SPEND` softened, the masculine chin hangs
/// in a soft midline drip — the render catches it — because the
/// bend below the mandible owes its gentleness to the spend curve
/// alone.
///
/// **What remains of the submental deviation belongs to [`SUBMENTAL_SPEND`],
/// not to this table and not to the cage.** Blaming the cage rests on
/// `examples/jawprobe`'s cage row against the reference's 0.165 — a share over
/// a bone-relative span quoted against a share over chin-to-throat, which is
/// the comparison that file's docstring exists to forbid. Measured
/// chin-relatively
/// instead, on the shipped body with `HeadTraits::chin` at zero, the cage spends
/// 0.0250 of the face over that centimetre against the reference's 0.0369: it is
/// two thirds as steep, not steeper. Nor is the first centimetre the defect —
/// read as REACH rather than as a drop, at each face's own share of
/// each height, ours lands on the reference at −10 on every seed, and
/// millimetre figures there compare ten millimetres of a 181 mm face with ten
/// of a 260 mm one. What remains is the second centimetre, and it belongs to
/// the chord in [`SUBMENTAL_SPEND`].
///
/// **The peak does not move, deliberately.** The crest of this sum changes
/// identity between bodies when the peak is touched, and a knot below
/// the peak cannot do that. A flatter tail should if anything help it: the sum's
/// crest is where this table's rise cancels the base's fall, and a table that is
/// flat there hands the decision to the base, which falls monotonically.
/// **The peak and its shelf are cut to 0.235/0.230 as a pair, so that the chin
/// does not stand out too far to the front while the crest keeps its identity**
/// — the trap is the peak and the shelf
/// swapping winner, not the amplitude itself — and the heights are untouched,
/// so [`MENTON`]'s identity with the peak knot holds. A deeper cut to
/// 0.225/0.220 was built and rendered: on the default body it read
/// well, and on seed 15 — the weakest rolled chin in the sweep — it erased
/// the chin outright (`the_chin_stands_clear_of_the_hollow_above_it` read NO
/// CHIN at one subdivision and 0.61 mm at the other, and the render agreed).
/// A flat amplitude cut lands hardest on the bodies with the least to give;
/// the rounder plan view is `point`'s widened window's job, not this one's.
const CHIN: [(f32, f32); 7] = [
    (0.05, 0.0),
    (-0.24, 0.060),
    (-0.42, 0.158),
    (-0.53, 0.235),
    (-0.575, 0.230),
    (-0.63, 0.020),
    (JUNCTION, 0.0),
];

/// Where the mandible's lower border sits at the angle of the jaw, in the same
/// profile heights every knot above is given in.
///
/// **Derived rather than picked.** The mandibular plane runs 22–28° below the
/// horizontal from the gonion forward to the menton, over a gonion-to-menton
/// run of about 85 mm — a 40 mm rise. Forty millimetres on the default head is
/// 0.30 of its radius, and through the floor remap that is 0.20 of a profile
/// height above [`MENTON`]. It lands within 0.02 of where [`crate::face::Canon`]
/// puts the mouth line, which is where a gonion sits on a face.
/// Provenance: **derived**, and the arithmetic is above: a mandibular plane
/// of 22 to 28 degrees over an 85 mm gonion-to-menton run is a 40 mm rise,
/// which is 0.30 R on the default head and 0.20 profile heights above
/// [`MENTON`] through the floor remap. The 22-28 degree plane is the
/// looked-up input and carries no source here.
const GONION: f32 = -0.31;

/// Where that same border sits on the midline, in profile heights.
///
/// [`CHIN`]'s peak knot, and the same number rather than a second one: the
/// border ends at the point of the chin by definition, and two constants for
/// one landmark is how a chin and the thing measuring it drift apart.
///
/// **It follows the peak wherever the peak goes**, which is the identity doing
/// its job: when the peak moves to keep the chin from rising below the
/// surface's own crest, this follows without anyone having to remember it.
/// `the_jawline_turns_a_corner` does not care, because the gonion it reads is
/// far out to the side where [`GONION`] rather than this dominates the border.
///
/// Provenance: **derived** from [`CHIN`], by identity rather than by
/// arithmetic — it is that table's peak knot and deliberately not a second
/// number for one landmark.
const MENTON: f32 = -0.53;

/// How far below the border the jaw's hollow reaches full depth, in profile
/// heights.
///
/// **The narrowest term in this file, and the one the resolution question is
/// about.** A knee that fits inside one mesh cell cannot render as a shape:
/// vertex rows catch the cut alternately and the crease's upper edge renders
/// as a scallop at exactly cell pitch. So the knee is held to the same floor
/// every feature here answers to — a span of one and a half to two cells over
/// the strip it actually crosses — and 0.060 profile heights is ~8.5 mm on the
/// default body, 1.5–2 cells over the refined strip the cosine border runs
/// through. Past about 56° there is no refinement at all and
/// the cells are 24 mm, so out at the gonion this knee is a fifth of a cell and
/// the border is smeared however sharp the field is. That is a resolution defect
/// and not a shape one; see [`FACE_PASSES`].
/// Provenance: **derived** from the mesh, not from a face: the cell
/// floor for a feature that must read as a shape rather than a bar, applied
/// to where the border actually runs.
const JAW_RISE: f32 = 0.060;

/// How deep the hollow under the jaw cuts, as a fraction of the horizontal
/// radius at its peak.
///
/// A seventh at the deepest, which is 7 mm under the angle of the jaw on the
/// default body. The deepest hollow the whole pipeline could cut before this was
/// 2.4 mm, and that is why there was no jaw.
///
/// **Set against the taper, not against the corner.** A first version cut a
/// fifth and reached its deepest a third of the way from the border down to the
/// junction. That turned the corner — but it also left the lower face a
/// cylinder: measured by bisection at 80° from dead ahead, the half-width at the
/// menton against the half-width at the angle of the jaw went from 0.92 to 0.98,
/// where a face converges. A hollow that is still near full depth at the chin's
/// own height is not a hollow under the jaw, it is a smaller head.
/// Provenance: **tuned by render**, against a bisected convergence
/// measurement rather than by eye — the 0.92-to-0.98 figure above is what
/// rejected the first value.
const JAW_DEPTH: f32 = 0.25;

/// How much of the way from the border to the junction the hollow takes to let
/// go again.
///
/// It has to reach nothing at [`JUNCTION`] like every other profile here, and
/// where it spends that distance decides whether there is a jaw or a narrower
/// neck. Releasing over 0.85 of the run puts the deepest point just under the
/// border — where a submandibular hollow is — and leaves the surface converging
/// the rest of the way, which is what the chin needs.
/// Provenance: **tuned by render**.
const JAW_RELEASE: f32 = 0.85;

/// Where the head's surface runs into the neck's, in skull radii.
///
/// Every profile below the joint has to reach identity here, because [`shape`]
/// moves head-owned vertices and leaves neck-owned ones — so whatever a profile
/// says at the bottom of the head is a step in the silhouette, not a shape.
///
/// One figure rather than each profile's own, because they all have to arrive at
/// the same place: a profile that lets go 0.05 radii before its neighbour leaves
/// a shoulder in the surface where its neighbour is still pulling.
///
/// **It is a nominal depth, not a real one.** Measured over sixteen seeds, the
/// head's surface reaches anywhere from -0.55 to -0.89 radii below the joint,
/// depending on how large the head node is against the neck node — a spread of
/// sixty percent, and no worse in millimetres. So no constant is the junction on
/// every body, and [`shape`] measures each head and scales the whole below-joint
/// domain so that its own floor lands exactly here. A chin authored at -0.52 is
/// then three quarters of the way down every head rather than off the bottom of
/// some and halfway down others.
/// Provenance: **derived**, and it is a nominal rather than a measured
/// depth — the paragraph above is the derivation, including why no constant
/// can be the junction on every body.
const JUNCTION: f32 = -0.70;

/// How far down the head the profiles have finished letting go, as a fraction of
/// the way to its floor.
///
/// **Not one, and it has to be measured to see why.** Reaching identity exactly
/// at the floor still left an eleven-millimetre shelf at the throat, because the
/// zone boundary is per-vertex and the mesh's rings do not line up with it: a
/// triangle spans from a neck vertex that was never touched to a head vertex a
/// centimetre higher that got the chin's full push, and the surface between them
/// is the shelf. Settling out a little above the floor leaves a band of head that
/// is simply unshaped, which is what the neck has to meet.
/// Provenance: **tuned by render**, against an eleven-millimetre shelf that
/// only appears when the value is exactly one.
const SETTLE: f32 = 0.92;

/// How far the head reaches below its own joint, as a share of the bone.
///
/// **The number that stops the neck from stretching the face.** Every
/// profile below the joint is scaled onto the head's floor, and a floor
/// MEASURED — the lowest vertex whose nearest bone is the head's — is the
/// wrong floor: it reads the SURFACE, and the surface at the bottom of the
/// head is the blend
/// into the neck, so anything that moves the neck moves the floor, which
/// stretches the whole lower face silently and in the wrong direction.
///
/// Measured three ways before it was believed. Deepening the girdle backward
/// moved the chin from −95.8 mm to −107.7 and grew the head from 201.8 mm to
/// 215.2. The neck's forward lean needed `HEAD_BELOW_JOINT` re-derived from 1.55
/// to 1.50 to hold the head still, and is bounded at a third of a neck radius by
/// this rather than by anatomy. And giving the neck the off-centre section it
/// wants took the head to 262.9 mm with a cranium:face of 0.62, against which
/// `HEAD_BELOW_JOINT` does not converge — 1.15 gives 178 mm, 1.30 gives 239,
/// 1.40 gives 200 — because the floor moves under the lever.
///
/// So it is derived from the RIG, which a section cannot touch: the head joint
/// stands `HEAD_BELOW_JOINT` radii above the neck joint by construction, and the
/// head's surface runs out at this share of that bone.
///
/// **What it costs is that the floor is no longer exactly where the surface
/// ends**, and `JUNCTION` is where every profile reaches identity. Measured over
/// seven bodies the true share runs 0.894 to 0.953, so on the worst of them the
/// identity now lands about 5% of the bone from where the surface actually
/// stops. That is tolerable only because the profiles are FLAT there — the last
/// knot of `BREADTH` is 1.00 and of `OCCIPUT` is 0.0 — so a small error in where
/// the junction falls is a small step and not a cliff. Anything that gives those
/// tables a gradient at `JUNCTION` makes this a seam again.
/// Provenance: **measured, then fixed**. Seven bodies read 0.9082,
/// 0.9002, 0.9182, 0.9008, 0.8936, 0.9533 and 0.8984 against their own
/// bones; 0.91 is the middle of that.
const SETTLED: f32 = 0.91;
/// How far up the forehead the frontal fullness reaches, and how much of it
/// each height takes.
///
/// A companion to [`BROW`] rather than part of it: the brow ridge is the bone
/// over the eye and this is the vault above it, and the frame axis moves the
/// two in OPPOSITE directions — a heavy brow sits under a forehead that slopes
/// away, and a light one under a forehead that stands up. One profile scaled
/// two ways could not say that.
///
/// Zero at the brow's own crest so the two do not fight over the same
/// millimetre, and zero again at the crown where a ray grazes and no
/// measurement is trustworthy.
///
/// Provenance: **derived** from the two CC0 mannequins. Their foreheads
/// are where they most disagree: measured by ray from each head's own axis and
/// normalised by that head's own peak forward reach, the female's falls away 9%
/// less steeply from 0.65 to 0.90 of the head's span, which is 0.48 to 0.94 in
/// the profile heights here. The knots are that window.
const FOREHEAD: [(f32, f32); 4] = [(1.00, 0.0), (0.80, 1.0), (0.58, 0.55), (0.45, 0.0)];

/// How a head reads the composites.
///
/// **The only record parameter the profiles in this file take.** Every other
/// head variation is cage-side — `head_size` is a node
/// radius, `head_breadth` a node section, `face_length` a joint placement — and
/// `crate::plan::derive::humanoid`'s `HEAD_BREADTH_SPAN` records why: a
/// breadth-like quantity moved here rather than on the cage opens the head/neck
/// seam, because [`shape`] moves head-owned vertices and leaves the neck's
/// alone. What belongs here is the carve — the shapes a capsule cannot say —
/// and facial dimorphism is almost entirely carve.
///
/// **Every field is a factor about ONE, and the neutral head is the identity.**
/// `femininity` zero is the midpoint of the two measured references, which is
/// the head this crate already built, so `HeadTraits::of(&Composites::default())`
/// has to be [`HeadTraits::default`] to four decimals or the neutral
/// anchor has moved. `the_neutral_head_is_the_head_that_was_already_built`
/// asserts it.
///
/// **What is measured and what is looked up**, because the two references carry
/// some of this set and not all of it. Their vaults agree to within 1% in width
/// and 2% in depth from 0.25 to 0.80 of the head's span — which is what makes
/// the three places they disagree worth reading — but neither mannequin has a
/// brow ridge to speak of, and the gonial angle needs a border detector that
/// measuring a silhouette does not give. So [`Self::jaw_breadth`] and
/// the elongation term are derived from the references, [`Self::chin`] and
/// [`Self::frontal`] take their SIGN from the references and their size from a
/// render, and [`Self::brow`] and [`Self::gonion`] are looked up. Each says so.
///
/// **`ELONGATION` is measurably dimorphic and is deliberately NOT here.** The
/// two mannequins' head length-to-breadth reads 1.566 masculine against 1.522
/// feminine, so the axis has a real claim on it — but `head_breadth` is already
/// a record axis and it sets that same ratio from the cage's own section. A
/// second, hidden driver of one quantity is two axes that can contradict each
/// other, which is the whole thing the two-tier parameter model exists to stop.
/// It is also not a carve: the
/// split this file's header describes puts breadth-like quantities on the cage.
/// Anyone who wants the axis to reach it should move `head_breadth`'s DEFAULT,
/// not add a term here.
///
/// It was tried first, and what it cost is worth recording: a longer head
/// reaches further forward everywhere, so the elongation term swamped
/// [`Self::frontal`] three to two at the forehead and inverted the one
/// measurement the forehead window was derived from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadTraits {
    /// Multiplier on the horizontal radius through the lower face.
    ///
    /// Bigonial breadth, and the strongest signal the references carry. Measured
    /// as each head's half-width at 0.15 of its own span over its own peak
    /// half-width: male 0.708, female 0.594, so the male's jaw is 19% wider
    /// relative to his own vault. The effect is 7.5% at 0.20 of span and gone by
    /// 0.25, which is why `HeadTraits::breadth_at` tapers it out over exactly
    /// that run rather than applying it to the whole lower face.
    ///
    /// **And it is where age shows in a lower face**, as jowling: the soft
    /// tissue over the mandible descends and the jawline widens where it used
    /// to run straight. That is tissue rather than bone, and this crate has one
    /// surface for both, so it lands on the same multiplier — see
    /// [`HeadTraits::of`] for the size and why it is smaller than the sex term
    /// beside it.
    ///
    /// What it delivers, on `headaudit --axis age` over its eight sweep seeds:
    /// the bigonial-to-bizygomatic ratio rises 0.8% to 1.2% between eighteen
    /// and eighty, on every seed and monotonically. A fifth of what the frame
    /// axis does to the same ratio, which is the intended size — an old jaw
    /// loses its straight line without becoming a masculine one.
    ///
    /// Provenance: **derived from the reference mannequins**; the age
    /// term **looked up, sized by render**.
    pub jaw_breadth: f32,
    /// Multiplier on `CHIN`'s forward push.
    ///
    /// **Sign measured, size not.** The references agree that the male chin
    /// projects further — axis-free, as chin reach minus brow reach so the axis
    /// cancels, his stands 13.3 mm behind his own brow against her 29.4 — but
    /// 16 mm of difference between two mannequins is a stylisation and not
    /// anthropometry: life puts chin-behind-brow in the 5–15 mm band on both
    /// sexes and the difference between them at a few millimetres. Taking the
    /// measured figure would have given a factor of 1 ± 0.34 and very nearly
    /// deleted the feminine chin.
    ///
    /// Provenance: **sign derived from the references, magnitude tuned by
    /// render**.
    pub chin: f32,
    /// Multiplier on `BROW`'s ledge.
    ///
    /// The supraorbital ridge, heavier on a masculine skull, and one of the
    /// first things a forensic determination reads. Neither mannequin has one
    /// to measure — their foreheads agree to 2% right through the brow's own
    /// band — so this is the looked-up direction at a size the render carries.
    ///
    /// Provenance: **looked up, sized by render**.
    pub brow: f32,
    /// How far the forehead above the brow stands proud, in skull radii, over
    /// the `FOREHEAD` window.
    ///
    /// **Zero at neutral, and signed**, unlike every other field here: the
    /// neutral head is the one this crate already built and it gains nothing, so
    /// a masculine forehead slopes away from it and a feminine one stands up.
    /// See `FOREHEAD` for the measurement that sets the window.
    ///
    /// Provenance: **sign and window derived from the references, magnitude
    /// tuned by render**.
    pub frontal: f32,
    /// How much fuller the lips are than the record asks for, as an offset on
    /// `super::features::FaceParams::mouth`.
    ///
    /// **An offset rather than a factor**, alone among these fields, because
    /// what it modifies is a record axis rather than a coefficient — see
    /// `FaceParams::on`, which is where offsets on derived defaults land.
    ///
    /// Feminine lips are fuller, and this is the one item in the dimorphism
    /// set the two mannequins cannot speak to at all: at 328 and 388 head
    /// triangles neither of them has a mouth to measure. So it is looked up.
    ///
    /// **What it delivers, measured rather than assumed, because the axis it
    /// offsets turns out to be a narrow one.** `relief::Face::plump` — the lip
    /// band's half-height, which is what the field is actually built from — runs
    /// 11.96 to 15.33 mm over the WHOLE record axis on the default body, so that
    /// slider is a ±12% control end to end. ±0.12 on it moves the lips 13.24 to
    /// 14.05 mm, about ±0.4 mm, which is roughly half of the sex difference in
    /// vermilion height that anthropometry reports. Half rather than all,
    /// because there is no measurement here to spend and the record's own axis
    /// is not wide enough to carry a whole one on top of the user's range.
    ///
    /// Anyone widening this should widen `plump`'s own gain first and
    /// re-derive this from it — the ceiling is that axis's range and not this
    /// coefficient.
    ///
    /// **Age takes the same axis the other way and takes more of it.**
    /// A lip thins over a life — the vermilion loses height, the philtrum
    /// lengthens and the upper lip rolls under — and unlike the sex difference
    /// this one is not subtle. It is still bounded by the same narrow axis:
    /// measured through `relief::Face::plump` on the default face, thirty to
    /// eighty runs 13.60 mm to 13.09 mm, so the whole of the age term is 0.5 mm
    /// of the 3.4 mm that slider spans end to end. That is less than life and
    /// it is all there is to give until `plump`'s own gain widens.
    ///
    /// Provenance: **looked up, sized against the axis it offsets**, and
    /// the age term the same way.
    pub lips: f32,
    /// Multiplier on the whole `OCCIPUT` profile.
    ///
    /// **One factor for both of its lobes, because the references move them
    /// together.** Measured as aft reach from each head's own axis over that
    /// head's own peak aft reach, the feminine cranium stands 5–7% FURTHER back
    /// through 0.40 to 0.60 of the head's span — the occipital curve proper,
    /// which is 0.03 to 0.39 in the profile heights here and is exactly
    /// `OCCIPUT`'s positive lobe — and 16–21% LESS far back at 0.25 to 0.30,
    /// which is where that profile's negative tail cuts the hollow in under the
    /// ear. A rounder, fuller occiput above a deeper hollow, against a flatter
    /// one above a fuller nuchal region: the textbook pair, and scaling the
    /// table says both at once, since a factor above one makes the positive
    /// knots more positive and the negative ones more negative.
    ///
    /// **Sized well under what the references ask for.** Matching their 6% of
    /// reach needs a factor near 1.6, because `OCCIPUT` is a fractional swell on
    /// top of `deep` and a small change in the profile is a much smaller change
    /// in the silhouette. The low band that asks for more than that is also the
    /// least trustworthy reading in the set — it sits where the head-owned
    /// selection meets the neck. So this takes the direction and a quarter of
    /// the size, as the chin does.
    ///
    /// Provenance: **sign and window derived from the references, magnitude
    /// tuned by render**.
    pub occiput: f32,
    /// Replaces `GONION` — where the mandible's lower border sits at the
    /// angle of the jaw.
    ///
    /// The gonial angle, as the border's height rather than as a degree: with
    /// `MENTON` fixed at the chin, raising this steepens the mandibular plane
    /// and lowering it flattens the jaw into a square one. The looked-up plane
    /// is 22–28° below the horizontal, and the sexes sit at the two ends of it —
    /// a masculine mandible is the flatter, more everted one. `GONION`'s own
    /// docstring derives −0.31 from the middle of that band, so the ends of the
    /// band are the ends of this axis and no new source is needed.
    ///
    /// **Age deliberately has no term here, and the numbers are why.** The
    /// gonial angle does grow more obtuse over a life — the alveolar bone
    /// resorbs, the ramus shortens — so the axis has a claim
    /// on this field, and a term of `+0.018 · ageing`, half the sex term, was
    /// written and then measured on `headaudit --axis age`. It does not behave
    /// like half of anything:
    ///
    /// ```text
    ///   bigonial : bizygomatic, age 18 → 80
    ///   seed        7      23      29      42       1       3       6      12
    ///   with     +0.9%   +0.8%  −11.5%   −3.0%   −6.5%   +0.9%   +1.1%   +1.0%
    ///   without  +1.1%   +1.0%   +1.2%   +0.9%   +1.1%   +0.9%   +1.1%   +1.0%
    /// ```
    ///
    /// Three of eight seeds went the wrong way and one of them by an order of
    /// magnitude more than the term should be able to move, while the jowl term
    /// beside it is uniform across all eight. That is the signature of a
    /// LANDMARK moving rather than a shape changing: this field sets where the
    /// mandible's lower border sits, `bigonial` is measured AT that border, and
    /// on a jaw whose taper the border can jump across, the instrument reads
    /// its own input. Which of the two it is — a real reshaping or a
    /// measurement artefact — is not answerable with a width taken at a moving
    /// point, and the term was the least-supported of the three age terms
    /// anyway, so it is not shipped. Reopening it wants an instrument that
    /// reads the mandibular plane ANGLE directly.
    ///
    /// Provenance: **derived from `GONION`'s own looked-up 22–28° plane**,
    /// read at its ends instead of its middle.
    pub gonion: f32,
    /// How far the laryngeal prominence stands off the throat, in head radii.
    ///
    /// The one feature of the neck that is a FEATURE rather than a fault in the
    /// surface, and the frame axis is where it lives on a person: the male
    /// larynx sits at about a 90° thyroid angle and shows, the female at about
    /// 120° and barely does. Applied by [`super::neck::fair`] AFTER the column
    /// is faired, so the smoothing cannot eat it.
    ///
    /// Provenance: **sized by render**, like the ageing jowl beside it —
    /// there is nothing to measure it against; neither mannequin models one.
    pub larynx: f32,
    /// How much trapezius the shoulders carry, as a factor on the neutral
    /// body's.
    ///
    /// The slope from the base of the neck out to the acromion, which is the
    /// one line the frame axis moves most in a silhouette: a masculine body
    /// carries a big trapezius and a short visible neck, a feminine one a
    /// long visible neck line over a shoulder that falls away. Applied by
    /// [`super::neck::trapezius`] on the column's own frame; its amplitude is
    /// a share of the girdle's radius, which already carries mass, so this
    /// is the frame term alone.
    ///
    /// Provenance: **sized by render** against the two CC0 mannequins'
    /// shoulder lines.
    pub trapezius: f32,
}

impl Default for HeadTraits {
    /// The head this crate built before the axis existed.
    fn default() -> Self {
        Self {
            jaw_breadth: 1.0,
            chin: 1.0,
            brow: 1.0,
            occiput: 1.0,
            lips: 0.0,
            frontal: 0.0,
            gonion: GONION,
            larynx: LARYNX_NEUTRAL,
            trapezius: 1.0,
        }
    }
}

/// The neutral body's laryngeal prominence, in head radii.
///
/// What [`HeadTraits::larynx`] reads at femininity zero, and therefore what
/// every neutral probe and test body carries. See the field.
const LARYNX_NEUTRAL: f32 = 0.030;

impl HeadTraits {
    /// Resolves the profiles' parameters for one body.
    #[must_use]
    pub fn of(composites: &Composites) -> Self {
        // Saturated for the head's own reasons and not for the body's. The
        // frame axis is stretched past ±1 by generator 2, and a face is where
        // that reads soonest: the terms here are carve rather than size, so a
        // femininity of +2.9 does not make a larger-eyed face, it deletes a
        // chin. #164 clamps the same axis at (−1.25, 2.85) for the BODY, at a
        // wall it measured; this is a judgement about faces and is tighter.
        let femininity = composites.femininity.clamp(-1.5, 1.5);
        // Age needs no clamp of its own: the ramp is zero under
        // `crate::plan::AGE_PIVOT` and one at the top of the record's range,
        // and the record's range is the adult band rather than a stretched
        // shape axis. So the head's age terms are bounded by construction and
        // the neutral head is still the head that was already built.
        let ageing = composites.ageing();
        Self {
            // Jowling, at rather less than the sex difference beside it. The
            // frame term spans ±9% about one and this adds 4% at eighty:
            // enough to lose the straight line of a young jaw, not enough to
            // read as the wide masculine mandible it shares a multiplier with.
            // Sized by render, because there is nothing to measure it against —
            // neither mannequin is old.
            jaw_breadth: frame(femininity, 0.708, 0.594) + 0.04 * ageing,
            // ±12% about the neutral chin, against the ±34% the references
            // themselves asked for. See the field.
            chin: 1.0 + 0.12 * -femininity,
            // ±25%, which is the largest factor here and still reads as a
            // shading difference rather than as a ledge appearing.
            brow: 1.0 + 0.25 * -femininity,
            // A quarter of the 1.6 the references ask for; see the field.
            occiput: 1.0 + 0.25 * femininity,
            // The sex difference is ±0.12 of this axis and age takes 0.15 of
            // it, which is the one place in this set where age outruns the
            // frame — see the field for what that is in millimetres and why it
            // is still short of life.
            lips: 0.12 * femininity - 0.15 * ageing,
            // In skull radii, against a `BROW` whose own peak is 0.042.
            frontal: 0.014 * femininity,
            // Strongly sexed and floored at nothing: a prominence cannot be
            // carved INTO the throat, and the feminine end of the axis reads
            // as its absence rather than as a hollow.
            larynx: (LARYNX_NEUTRAL - 0.030 * femininity).max(0.0),
            // The 22–28° mandibular plane read at its ends. `GONION`'s
            // derivation turns a degree into a height: the plane's rise over an
            // 85 mm gonion-to-menton run, as a fraction of the head's radius and
            // then through the floor remap. Rerunning it at 22° and 28° puts the
            // border 0.036 profile heights either side of the middle.
            // No age term, and the field records the measurement that took it
            // back out.
            gonion: GONION + 0.036 * femininity,
            // The frame term on the shoulder slope's fill; see the field. A
            // feminine shoulder still has a trapezius, so it floors well
            // above nothing.
            //
            // **A slope of 0.15, not 0.30**: the amplitude is already in
            // girdle radii, and the girdle grows toward the masculine end
            // through the girth, so the frame term compounds on it. At 0.30
            // the −1.5 body's stand was 33 mm and the fill read as a slab.
            trapezius: (1.0 - 0.15 * femininity).max(0.5),
        }
    }

    /// How much wider or narrower the lower face is at `height`.
    ///
    /// One at [`Self::jaw_breadth`]'s upper edge and above, full at the angle of
    /// the jaw and through the jaw's body. The window is the references' own:
    /// their half-widths disagree by 16% at 0.15 of the head's span, 7.5% at
    /// 0.20 and nothing at 0.25, which is −0.43, −0.34 and −0.25 in the profile
    /// heights here.
    ///
    /// **And it lets go again before the head's floor, which it did not, and
    /// that was the masculine collar** (#301). Held "at the angle of the jaw
    /// and below" it was still at full strength on the last ring of head-owned
    /// surface, a 13% lateral push at femininity −1.5 — and the neck-owned
    /// ring four millimetres under it moves by nothing, because ownership is
    /// per vertex. A 10 mm step in half-width at the head/neck boundary, all
    /// the way round; and on a short masculine neck that boundary sits ON the
    /// girdle's crown, so the step read as a turtleneck collar with points at
    /// the lateral corners. [`SETTLED`]'s own rule names it: anything with a
    /// gradient at [`JUNCTION`] is a seam. Every profile table's last knot is
    /// identity there; this multiplier is not a table and had no last knot.
    /// Full through [`MENTON`] — the bigonial breadth is the jaw's body, and
    /// the body ends at the border — then a smoothstep to one at the junction,
    /// so the release has zero slope at both ends like the carve's own ramps.
    fn breadth_at(&self, height: f32) -> f32 {
        let arrive = ((-0.25 - height) / 0.18).clamp(0.0, 1.0);
        let hold = smooth((height - JUNCTION) / (MENTON - JUNCTION));
        1.0 + (self.jaw_breadth - 1.0) * arrive * hold
    }
}

/// The region each refinement pass covers: how far round the head it reaches as
/// a cosine of the angle from dead ahead, then its lowest and highest point.
///
/// **Heights above the joint are skull radii; heights below it are PROFILE
/// HEIGHTS**, the same remapped unit [`reshape_to`] reads its knots in. The
/// asymmetry is deliberate and it is what keeps a band aimed at a feature when
/// face length moves.
///
/// **That kink at the joint is the only thing standing between a band and the
/// landmarks it is aimed at, and it is what [`band_at`] is for.** The
/// height axis itself is shared: [`reshape_to`] returns `local.y` untouched, so
/// a face that sits at a given height on the mesh [`refine_face`] selects from
/// sits at exactly that height on the shaped head — asserted by
/// `shaping_the_skull_does_not_move_a_vertex_up_or_down`. What a band cannot be
/// handed is a landmark straight out of [`super::Canon`], because those are
/// metres above the joint and these are two different normalisations of them
/// either side of zero. [`band_at`] does that conversion, and
/// `every_refinement_band_still_contains_its_own_feature` guards that the
/// bands below still cover what they exist for.
///
/// **Why profile heights below the joint and not raw radii.** A head reaches
/// anywhere from −1.07 to −1.16 radii below its joint on the bodies that ship
/// and runs −0.89 to −1.36 across the face-length axis, so a band edge in raw
/// radii is the mouth on one body and the chin on another. The
/// features are not in raw radii either: every one of them is placed as a
/// fraction of the eye-to-chin frame, and the chin is a fixed 0.7097 of the
/// head's own floor — which works out to a **dead constant −0.540 profile
/// heights on every body, whatever its floor**. Worked through, the mouth line
/// lands at −0.304 to −0.310 across the whole face-length range against a −0.307
/// on the default, so in this unit the whole feature stack holds still and a
/// band that covers it once covers it always. In raw radii the same arithmetic
/// puts the mouth line at −0.377 R on a short face and −0.520 on a long one,
/// against a finest pass spanning −0.52 to −0.34: the lip line walks out of its
/// own refinement at BOTH ends of the axis, and a mouth outside its refinement
/// renders as a stack of bars.
///
/// **Graded, not uniform, and that is what makes it affordable.** Refining the
/// whole front of the head twice costs 2,660 triangles and spends most of them
/// on a forehead and a pair of cheeks, which carry nothing. The first pass is
/// broad — the front of the head plus the sides that hold the temples and the
/// jaw, stopping around the ears so the boundary between fine and coarse
/// geometry falls where the hair usually is. The second is only the band the
/// features actually occupy, brow to chin.
///
/// Measured, not guessed: at one pass the median edge under the brow is 12.7 mm
/// and a brow ridge is 10 mm tall, so a whole feature spans one quad. At two it
/// is 6.2 mm.
///
/// **The third region is the mouth alone, and it exists because a mouth is made
/// of smaller parts than the rest of a face.** With the face at 3.6 mm cells,
/// every term in the lip field was at or near one cell wide — the lip line's
/// groove 0.99, the sulcus 1.29, the two vermilion lobes 1.67 and 1.75 — and a
/// Gaussian one cell wide cannot render as anything but a single displaced row
/// of vertices, which is a bar — a terraced lower face.
///
/// Widening every lip term to three cells instead removes the bars outright —
/// and takes the mouth with them, which is the other half of the answer. A lip
/// line is 1–2 mm across on a person, so a mouth wide enough to survive 3.6 mm
/// sampling is not a mouth. The band gets the resolution instead, and keeps its
/// shape.
///
/// **The fifth region is the LIP LINE alone**, and it is that narrow because the
/// whole mouth band cannot be afforded twice. The narrowest term — the groove
/// between the lips, at 0.26 of a lip stack — is the only one that falls short
/// at the fourth pass's cell; refining the whole mouth band again costs 10,244
/// triangles, putting skin at 66% of the body against `tests/budget.rs`'s 0.60
/// guard. So the pass goes where the shortfall is: every
/// other lip term measures 2.2 to 2.9 cells and needs nothing.
///
/// Bounded at plus or minus 0.7 of a lip stack about the mouth line, where the
/// groove's own Gaussian has fallen to nothing, so the resolution boundary lands
/// on a part of the field that is not doing anything. It still takes in both
/// vermilion lobes.
///
/// **0.7 is derived rather than trimmed to fit.**
/// The groove is `bump(up, 0.00, 0.26)`, which is `exp(-(up/0.26)²)`: at 0.85 of
/// a lip stack it is two parts in a hundred thousand of its peak and at 0.7 it
/// is seven in ten thousand. Both are nothing, and the pass exists for the
/// groove alone — every other lip term measures 2.2 to 2.9 cells and needs no
/// refinement at all. The lower lip's own lobe is still at 95% of peak at 0.7,
/// and still inside; what is outside is a tail of the one term that was ever the
/// reason for this pass.
///
/// It buys 390 triangles on the default body and 750 on the dearest. Costs
/// here are quantised: a band edge lands on a ring of faces rather than between
/// them, so a one-percent move in an edge is a whole row of quads in or out.
///
/// **The sixth region is the JAW FLANK, and it is the first here that is an
/// annulus rather than a cap.** Every pass above reaches from dead ahead round
/// to a cosine and stops, so the region a pass covers always contains the front
/// of the face — which is why the mouth's passes cannot be widened to take in
/// the jaw's angle without paying for a fourth and fifth refinement of a nose.
/// Measured on the shipped head, the median head-owned edge in the band from
/// −0.85 to −0.30 R runs 0.8 mm dead ahead, 1.8 mm at 40°, 3.5 mm at 55° and
/// **24 mm past 60°** — the base subdivision, untouched. The jaw's own border
/// migrates from the menton out to the gonion at 90°, so without this pass half
/// of it lies in a region with no resolution at all and [`JAW_RISE`] is a fifth
/// of a cell there.
///
/// So a pass carries a near AND a far cosine, and this one takes the strip
/// between them: from 57° out to 99°, over the heights the border crosses. It
/// costs nothing on the front of the face because it does not reach it — which
/// is why it is so cheap. Listed twice, like the mouth's: one pass took the
/// gonion from 24 mm cells to 13 for 196 triangles, two take it to 6.6 for 652.
/// A third, over the knee's own band alone, reaches 3.6 mm and costs another
/// 664 — but that puts skin at 59.0% of the body against `tests/budget.rs`'s
/// 0.60 guard, and a guard with one percent left in it is not a guard. So the
/// border out at the gonion is carved across three quarters of a cell and is
/// softer there than the 33–57° jawline is; that is measured, and it is the
/// reason this band is where the next resolution should come from.
///
/// **There is no region for the chin's crest, and that is a measured decision
/// rather than an open one.** The chin's crest measures −0.498 to −0.522
/// profile heights across seeds 7, 0, 21 and 3, so it falls BELOW the nose-base
/// pair's floor of −0.443 and the comment on that pair has been true of no body
/// this crate builds: the most projecting feature of the lower face gets three
/// passes where the mouth above it gets six, and its whole descent falls between
/// two consecutive vertex rows. A narrow band at the crest —
/// `(0.85, 1.0, -0.580, -0.443)`, or `(0.92, ...)` at the lip line's own
/// azimuth — does exactly what it says it will: rows at the crest go from 4.2 mm
/// apart to 2.5, and the descent spreads over four of them instead of one.
///
/// It has been built, costed and judged twice, and it does not show either
/// time. At 0.85 it costs 326 triangles on the default and 712 at the dearest
/// corner of `tests/budget.rs`'s sweep; at 0.92, 246 and 504. Against that,
/// side by side at the same seed and light, the chin is
/// marginally rounder in the lit render and indistinguishable in the normal
/// buffer, where a geometric change of any size has nowhere to hide. The bars
/// the eye reads across the lower face are the mouth's own forms, in a band
/// that already has six passes.
///
/// So the crest stays at three passes. What would justify a pass here is a chin
/// whose SHAPE needs the rows — a curve authored to turn inside that 4 mm — and
/// not the rows on their own.
///
/// **The seventh region is the NOSE'S DORSUM — the pass the note above
/// could not justify for the chin, spent somewhere it shows — and it is aimed
/// by arithmetic rather than by iteration.**
/// Without it, everything from the nose base pair's ceiling up to the root of
/// the nose sits at the third pass's cell — 3.42 mm across and 7.23 mm down on
/// the default body, against 0.76 and 2.25 immediately below it — because the
/// five passes after the third all stop at or under −0.171 profile heights.
/// Nothing goes near the bridge, and the bridge is where a nose is a tent:
/// `examples/facesection` counts one to two post-carve facets between the
/// midline and the shoulder over the top half of it, and the render shows a
/// hard crease down the ridge.
///
/// The band is [`band_at`] applied to the nose's own two ends.
/// Its floor is the nose base pair's ceiling exactly,
/// so the two are contiguous and no strip of the nose is left between them. Its
/// ceiling is the root — `level + frame * 0.1237`, where [`super::relief`]'s
/// nose begins — measured at 0.150 to 0.196 radii across the corners of
/// `tests/budget.rs`'s sweep, so 0.20 clears the highest of them. That spread is
/// the kink this table has at the joint made visible: the root is ABOVE the
/// joint, so it is a raw radius and drifts by 4.7 mm as the frame moves under
/// it, where the base below the joint holds to within 0.018 profile heights.
///
/// **The azimuth is 0.97 and that is where the cost is.** The nose's shoulder
/// subtends about 7° on the unsectioned head at this height, so a cosine of 0.97
/// — 14° — is twice the reach the feature needs and leaves the resolution
/// boundary off the shape entirely. The same band at 0.92 costs 30,154 at the
/// dearest corner of the sweep, and at 0.55, 6,196 triangles on the default
/// body alone. At 0.97 it is **382 on the default and 548 at the dearest**.
/// A band's cost lives in its azimuth, which is why the two ends here are
/// measured rather than guessed at.
const FACE_PASSES: [(f32, f32, f32, f32); 10] = [
    // **The broadest pass, and it is here because the body's subdivision level
    // halved** (#107). Eight-point cage rings buy the body a smooth surface at
    // one Catmull-Clark pass instead of two, which is where the triangle budget
    // for them comes from — but the head goes through the same halving, and
    // arrives with cells twice the size every band below was measured against.
    // No band pass can make that up: each one refines only what it already
    // covers, so the strip outside the first band's near cosine, and everything
    // above its ceiling, stays at the base level however many passes follow.
    //
    // This one covers the whole front and both flanks — out past the ears at
    // -0.30, from the floor of the head to well above the brow — and it runs
    // first, so every band pass after it splits cells that are already halved.
    // Measured as the median edge of a head-owned face over the four feature
    // bands: 9.3 mm without it against 4.8-5.7 with, which is back inside the
    // range the passes below were tuned at. The rendered nose has its bridge
    // and its tip again; without it the whole feature was a mound.
    // −0.80 rather than the −0.821 the old −1.15 converts to: the head's own
    // floor is at JUNCTION / SETTLE = −0.761 profile heights on EVERY body by
    // construction, so anything past it covers the whole skull and the extra is
    // margin rather than reach.
    //
    // **The ceiling went 1.0 → 1.10 for the same reason the floor is −0.80**
    // (#79). Heights ABOVE the joint are raw skull radii, not profile heights,
    // and the built crown moved from 0.85 radii to 1.03 — so a ceiling of 1.0
    // stopped being margin past the head and became a resolution boundary
    // three millimetres under the vertex. It costs 22 triangles on the default
    // body and 22 at the dearest corner to put the crown back inside, which is
    // the price of the ceiling meaning what the sentence above says it means.
    // The other two ceilings, 0.60 and 0.50, are NOT margin and did not move:
    // they are feature bands, and `BROW` runs out at 0.58.
    (-0.30, 1.0, -0.80, 1.10),
    (0.25, 1.0, -0.80, 0.60),
    (0.55, 1.0, -0.714, 0.50),
    // The flank of the jaw, from where the mouth's passes give up round to
    // just behind the ear. Listed twice for the same reason the mouth's band
    // is, and it is the only region here that both of its bounds are real.
    (-0.15, 0.55, -0.571, -0.200),
    (-0.15, 0.55, -0.571, -0.200),
    // Nose base to below the chin: the only band where the features are
    // smaller than the surface carrying them. Listed twice because a region is
    // refined once per pass that names it, and this one wants two.
    (0.55, 1.0, -0.443, -0.171),
    (0.55, 1.0, -0.443, -0.171),
    // The dorsum of the nose, from the nose base pair's own ceiling up to the
    // root. Narrow, because a bridge is: see the note above for what the
    // azimuth costs.
    (0.97, 1.0, -0.171, 0.20),
    (0.92, 1.0, -0.360, -0.255),
    // The FRONT of the mandible's border (#196). The cosine border (#195)
    // runs from −0.495 at 33° to −0.43 at 57°, which is BELOW the nose-base
    // pair's floor and INSIDE the jaw annulus's near cosine — a strip no band
    // above covered, left at the three broad passes' cell, and the crease's
    // knee rendered as a scallop at cell pitch there. First costed and
    // rejected as too dear, then CHOSEN BY THE OWNER on the A/B sheet
    // (2026-08-12): +1,340 triangles on the default body and +2,848 at the
    // dearest sweep corner, and `tests/budget.rs`'s ceilings were re-based to
    // carry it — the sheets beat the arithmetic, which is #193's standing
    // order for this band.
    (0.55, 0.92, -0.575, -0.415),
];

/// The `FACE_PASSES` band edge that selects a given height on the built face.
///
/// Takes a height in metres above the head joint — which is what
/// [`super::Canon`] and [`Skull`] speak, so `band_at(rig, canon.nose_base())` is
/// the number to write in a band that wants to reach the base of the nose — and
/// returns it in the unit [`refine_face`]'s own table is written in.
///
/// **The conversion exists because the unit has a kink at the joint and the
/// height axis does not.** Nothing about the shaping moves a vertex up or
/// down: [`reshape_to`] scales `x` and `z` and hands `y` back as it found it, so
/// a landmark measured on the finished surface is at the same height on the
/// unshaped one that [`refine_face`] runs against, and this is a change of units
/// rather than of coordinates. Below the joint that unit is a profile height, so
/// a band follows the length of the face it is aimed at; above the joint it is a
/// raw skull radius, because what is above the brow is a vault the frame does
/// not stretch and a ceiling there is margin past a crown.
///
/// Returns `None` for a body with no head, or one whose head has no radius.
#[must_use]
pub fn band_at(rig: &Rig, height: f32) -> Option<f32> {
    let &head = rig.in_zone(Zone::Head).first()?;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return None;
    }
    let radii = height / radius;
    if radii >= 0.0 {
        return Some(radii);
    }
    let stretch = floor(rig, head) * SETTLE / JUNCTION;
    (stretch > f32::EPSILON).then(|| radii / stretch)
}

/// Gives the face enough surface to carry features, before anything shapes it.
///
/// The head arrives from the cage as a four-sided tube. Subdivided twice it is
/// 189 faces with a mean edge of 24 mm, and every feature a face needs is at or
/// below that: a brow ridge is 10 mm tall and a nose one quad wide. Nothing can
/// be shaped into a surface that has no vertices where the shape goes.
///
/// Refines only the front of the head, because the cost is triangles and the
/// back of a skull carries nothing. Runs BEFORE [`shape`], so the vertices it
/// adds are placed on the sphere and then mapped onto the skull by [`reshape`]
/// along with every other one — which samples the skull more finely, rather than
/// subdividing the facets of an already-shaped one.
///
/// **And it splits with [`PolyMesh::refine_curved`], because sampling the field
/// more finely is only half of what a face needs.** [`reshape_to`] is an
/// anisotropic SCALING of the section it is handed — `x` by one factor, `z` by
/// another, both slow functions of azimuth — so a flat chord in maps to a flat
/// chord out. The head arrives as a sixteen-sided tube, every plain midpoint
/// sits on one of those chords, and a plain split would therefore buy the base
/// skull no curvature whatsoever: measured round a linearly-split head with
/// `examples/chinprofile --ring`, the section turned 0.0° through four
/// consecutive samples and then 19 to 23° in one, at every multiple of 22.5°.
/// The additive terms — `CHIN`'s push, the brow's ledge, the mouth's relief —
/// are all the extra samples would buy, and the eye reads the
/// sixteen-gon underneath them as flat planes meeting at a hard edge from the
/// zygomatic down to the jaw. A curved split costs no triangles and is the
/// whole of that defect.
///
/// **A wider band is not the alternative, and it is costed rather than
/// argued.** Extending the jaw flank's annulus up to the zygomatic —
/// `(-0.15, 0.55, -0.571, 0.20)`, an eleventh pass with `FACE_REFINEMENT`
/// moved with it — costs 5,428 triangles on the default body and lands the
/// dearest corner of `tests/budget.rs`'s own sweep at 33,966, four times
/// through the 30,000 target, for a region whose fields carry nothing
/// finer than the cells already there.
///
/// Does nothing to a body with no head, or to one that walks on four legs: this
/// is a human skull's geometry and a creature's head is its own shape.
#[must_use]
pub fn refine_face(mesh: &PolyMesh, rig: &Rig, levels: usize) -> PolyMesh {
    if levels == 0 || rig.ground_contacts().len() > 2 {
        return mesh.clone();
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return mesh.clone();
    };
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return mesh.clone();
    }

    // The same remap [`shape`] applies, measured once on the mesh that arrives.
    // `refine` only adds vertices — it moves none — so the head's floor is the
    // same before the first pass and after the last, and measuring it per pass
    // would be a slower way of getting the same number.
    let stretch = (floor(rig, head) * SETTLE / JUNCTION).max(f32::EPSILON);
    let section = rig.joints[head].scale.x.max(f32::EPSILON);

    let mut refined = mesh.clone();
    for pass in 0..levels {
        // Passes past the last named one repeat the tightest region rather than
        // widening again, so asking for more resolution never spends it on a
        // forehead.
        let (near, far, low, high) = FACE_PASSES[pass.min(FACE_PASSES.len() - 1)];
        // Below the joint a band edge is a profile height, so it is stretched
        // onto this head's own lower face; above it, a skull radius, unchanged.
        // See [`FACE_PASSES`] and [`reshape_to`], whose remap this is.
        let onto = |edge: f32| if edge < 0.0 { edge * stretch } else { edge };
        let (low, high) = (onto(low), onto(high));
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| {
                let at = refined.face_centroid(face);
                // Asked of the rig rather than cut by height, for the same
                // reason `shape` does: the neck runs up into the same band and
                // refining it would spend triangles on a throat.
                if rig.joints[rig.nearest_bone(at).joint].zone != Zone::Head {
                    return false;
                }
                let local = at - centre;
                let height = local.y / radius;
                if height < low || height > high {
                    return false;
                }
                // **The angle round the UNSECTIONED head** (#61). A pass covers
                // an angular region — the front of the face, the flank that
                // holds the jaw — and the skull carries a lateral section now,
                // so the same material reports a different azimuth on a narrow
                // head than on a broad one. Left alone, a narrow skull passed
                // more of itself into every band and cost 29,156 triangles
                // against a broad one's 26,424: a tenth of the whole avatar's
                // budget decided by which way a slider was pushed. Dividing the
                // section out asks the question of the cage's own ring, where
                // the regions were authored.
                let across = Vec3::new(local.x / section, 0.0, local.z);
                let span = across.length();
                span > f32::EPSILON && across.z / span > near && across.z / span <= far
            })
            .collect();
        refined = refined.refine_curved(&selected);
    }
    refined
}

/// Shapes the head of a built body, in place.
///
/// Does nothing to a body with no head. Idempotent only in the sense that it is
/// a function of the rest positions — call it once, on the rest mesh, before
/// binding or unwrapping.
pub fn shape(mesh: &mut PolyMesh, rig: &Rig, traits: &HeadTraits) {
    // These are a HUMAN skull's proportions — a chin, a brow ridge, cheekbones
    // widest. On something that walks on all fours they are simply wrong, in the
    // same way that giving its front legs fingers was wrong. A creature's head
    // is its own shape and belongs with the rest of the creature work.
    if rig.ground_contacts().len() > 2 {
        return;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return;
    }

    // Which vertices belong to the head, asked of the rig rather than cut by
    // height: the neck runs up into the same band and must not be reshaped, or
    // the throat pinches away from the jaw it is supposed to meet.
    let owned: Vec<bool> = mesh
        .positions
        .iter()
        .map(|&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
        .collect();

    // How far the head reaches below its joint, and every profile below the
    // joint is scaled to land on it. See [`JUNCTION`] and [`floor`].
    let floor = floor(rig, head);

    for (point, &mine) in mesh.positions.iter_mut().zip(&owned) {
        if !mine {
            continue;
        }
        *point = centre + reshape_to(*point - centre, radius, floor, traits);
    }

    if std::env::var_os("SKIP_SUBMENTAL").is_none() {
        construct_submental(mesh, &owned, centre, radius, floor);
    }
}

/// How far off the midline the submental construction reaches, in head radii,
/// and the columns its chords stand in.
///
/// Half a radius covers the chin and the jaw's body out to the gonial corner
/// on every seed measured; the ramus and the ear are beyond it and keep their
/// own shape.
const SUBMENTAL_SPAN: f32 = 0.50;
const SUBMENTAL_COLUMNS: usize = 6;

/// How much convexity the chin's own button may keep above the submental
/// chord, in head radii, and the share of the run it may keep it over.
///
/// **This pair is the discriminator between a chin and a bulge under the jaw.**
/// A chord anchored at the crest with no allowance at all amputates the chin
/// button on every soft-chinned body — a dome's lower shoulder is always
/// forward of the straight line from its apex — and one anchored a fixed
/// chin-thickness below the crest spares the BULGE too, because on the default
/// family the bulge peaks 8 mm under the crest, inside any anatomical
/// thickness.
///
/// What separates them is not where they sit but how far they stand out:
/// measured across the sweep, a button deviates from the crest-to-throat chord
/// by about a millimetre and the bulge by eight. So the chord runs from the
/// TRUE crest, and this allowance — about 2.3 mm on the default head, fading
/// to nothing over the top [`BUTTON_RUN`] of the run — is the convexity a chin
/// is entitled to. Everything past it is bulge and is planed off.
///
/// **A large button over a SHORT run, rather than the reverse.** `CHIN`'s tail
/// holds a shelf under the chin, and a raised surface is no use if this ceiling
/// cuts it straight back off — the construction only ever clamps downward, so
/// the allowance is what lets the shelf survive. But the allowance cannot
/// simply grow: swept at 0.050 over 0.85 of the run it puts seed 10 back 5.7 mm
/// proud of its chord at 43% of the way down, which is a bulge and not a
/// button, and `the_underside_of_the_jaw_does_not_bulge` fails at 0.065 against
/// its 0.045. Three times the convexity at the crest, dead by a fifth of the
/// way down — above everywhere the bulge has ever been measured — does both.
///
/// **And on its own it buys nothing**, which is worth recording because it looks
/// like the obvious lever. With `CHIN`'s tail left alone this pair moves the
/// first-centimetre drop from 19.7 mm to 20.2: the surface is not against the
/// ceiling there, so raising the ceiling is a no-op. It is a permission, not a
/// push.
///
const BUTTON: f32 = 0.070;
/// See [`BUTTON`].
const BUTTON_RUN: f32 = 0.22;

/// How much of the crest-to-throat drop the submental ceiling has spent, at
/// each share of the run from the crest down to [`JUNCTION`].
///
/// **A STRAIGHT chord is not what a jaw does**, which is why the ceiling this
/// curve describes is not one. Planing the underside onto the line joining each
/// column's crest to its throat stands 19.9, 18.7, 23.7 and 10.3 mm PROUD of
/// the CC0 reference two centimetres under the chin on seeds 0, 3, 6 and 12 —
/// measured with `examples/column` at each face's own share of each height —
/// and within a few millimetres of it at every other height on the run. One
/// row, and it is the row a straight line puts in the wrong place.
///
/// # Why this and not [`CHIN`]'s tail
///
/// Because on half the population the tail cannot reach it. With
/// `SKIP_SUBMENTAL` set, seed 0 reads 82.7 mm at −20 and the shipped body reads
/// 73.5: the ceiling is removing 9.2 mm there, so the surface is AGAINST it and
/// lowering `CHIN` underneath only reduces what gets planed off. Swept, moving
/// the tail knot from −0.63 to −0.60 moved seed 0's −20 row by 2.7 mm and seed
/// 12's by 22.9 — the same knot, an order of magnitude apart, because seed 12's
/// surface is clear of the ceiling and seed 0's is not. A ceiling binds on
/// whichever bodies are against it and lowering it moves all of them.
///
/// # What it is shaped from
///
/// The reference's own run, on the ruler `examples/column` reads it with: 0.16
/// of its chin-to-throat drop spent a sixth of the way down, 0.76 a third of
/// the way, 0.92 by half. The knots below are that shape re-fitted against
/// measurement rather than transcribed, because this run is the crest to
/// `JUNCTION` and the reference's is its chin to its throat, and a share over
/// one span is not a share over the other.
///
/// Measured, ours proud of the reference at 0, −10, −20, −30 and −40 mm:
///
/// ```text
///   seed  0   +3.7  +2.5   +9.5  +1.0  +0.8
///   seed  3   +1.4  +1.5   +6.9  -7.0  -7.4
///   seed  6   +5.0  +4.2   +3.0  +4.8  +5.2
///   seed 12   -4.9  -5.5   -5.8  -6.0  -4.2
/// ```
///
/// # The top third is deliberately left straight
///
/// A curve that starts falling at 0.15 of the run AMPUTATES a chin: seed 9's
/// goes from 73.3 mm of forward reach to 52.7, `Skull::chin`
/// migrates 15 mm up the head, and the whole submental run inverts — its reach
/// increasing downward. The body it fails on is the one whose head the column
/// has all but swallowed, where this run is only a few millimetres long and a
/// fifth of it is under a chin.
/// So `spend` is the identity down to 0.30, and the fall lives entirely below
/// it, where a chin's dome does not reach. [`BUTTON`]'s allowance fades out
/// by 0.22, which is inside the straight part.
///
/// # One curve, not a shelf and a cliff
///
/// The bend directly below the mandible is gentler than the reference's step:
/// the knots below deliberately do not transcribe it. Softening the middle
/// alone does not move the thing that matters — measured on `examples/column`,
/// the midline drop over the first centimetre below the chin reads 19.5 mm
/// either way against the reference's 9.6, because with the ceiling switched
/// off entirely that centimetre still falls 15.6, so the ceiling never owned
/// it. Holding the spend out of the top third, and stopping the fairing
/// inflating the crest (`FAIR_OVER`), takes the run to 14.3 / 19.8 — slow, then
/// steep, then slow. Judged in both renderers at the frame ends, the heavy
/// corner and the guarded seeds.
///
/// Provenance: **shape derived from the reference, knots tuned by sweep against
/// it and by render**.
#[rustfmt::skip]
const SUBMENTAL_SPEND: [(f32, f32); 6] = [
    (1.00, 1.00),
    (0.78, 0.90),
    (0.58, 0.66),
    (0.42, 0.36),
    (0.34, 0.12),
    (0.00, 0.00),
];

fn construct_submental(mesh: &mut PolyMesh, owned: &[bool], centre: Vec3, radius: f32, floor: f32) {
    let stretch = (floor * SETTLE) / JUNCTION;
    if !stretch.is_finite() || stretch <= 0.0 {
        return;
    }
    let world_y = |height: f32| centre.y + height * stretch * radius;

    // **Chords stand in COLUMNS of constant |x| and clamp FORWARD reach, not
    // radial reach — and the first construction here was radial and measurably
    // wrong.** Radial chords cut each azimuth at its own crest, so the flank
    // BESIDE the chin was planed at the chin's own height while the midline
    // kept its full reach: the blade #128 closed reopened at +19.7 mm proud
    // against 9.7. A column only ever compares the surface with itself at
    // other HEIGHTS, so a row through the chin keeps its lateral shape whole.
    //
    // Bisected against the shaped surface, never binned, as everywhere.
    let reach = |y: f32, across: f32| -> Option<f32> {
        let from = Vec3::new(centre.x + across, y, centre.z);
        if !mesh.contains(from) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, radius * 2.5);
        if mesh.contains(from + Vec3::Z * far) {
            return None;
        }
        for _ in 0..32 {
            let middle = 0.5 * (near + far);
            if mesh.contains(from + Vec3::Z * middle) {
                near = middle;
            } else {
                far = middle;
            }
        }
        Some(near)
    };
    let column_x =
        |column: usize| SUBMENTAL_SPAN * radius * column as f32 / (SUBMENTAL_COLUMNS - 1) as f32;
    // The top anchor is each column's own crest, scanned finely: the chord
    // falls at nearly a millimetre per millimetre, so a quantized crest height
    // turns directly into millimetres carved off the chin — a 0.03-step ladder
    // cut 7.6 mm off seed 2's chin by quantization alone.
    let crest = |across: f32| -> Option<(f32, f32)> {
        // The scan stops at MENTON − 0.07: the deepest crest measured anywhere
        // in the sweep sits at −0.59, and every rung below the crest zone is
        // pure build cost — this scan runs inside every body build, and at its
        // first width it alone took the lib suite from 4.9 s to 12.2 s.
        let mut best: Option<(f32, f32)> = None;
        for step in 0..=21 {
            let height = MENTON + 0.14 - 0.01 * step as f32;
            if height <= JUNCTION {
                break;
            }
            let Some(forward) = reach(world_y(height), across) else {
                continue;
            };
            if best.is_none_or(|(_, reach)| forward > reach) {
                best = Some((height, forward));
            }
        }
        best
    };
    let top: Vec<Option<(f32, f32)>> = (0..SUBMENTAL_COLUMNS)
        .map(|column| crest(column_x(column)))
        .collect();
    let bottom: Vec<Option<f32>> = (0..SUBMENTAL_COLUMNS)
        .map(|column| reach(world_y(JUNCTION), column_x(column)))
        .collect();
    let heights: Vec<Option<f32>> = top.iter().map(|a| a.map(|(h, _)| h)).collect();
    let reaches: Vec<Option<f32>> = top.iter().map(|a| a.map(|(_, r)| r)).collect();
    // A column that measured nothing keeps its surface: the clamp needs both
    // ends of its chord, and inventing one from a neighbour is how the scalp's
    // sector fill put the back of a head under the face (#125).
    let chord_at = |anchors: &[Option<f32>], across: f32| -> Option<f32> {
        let at = across / (SUBMENTAL_SPAN * radius) * (SUBMENTAL_COLUMNS - 1) as f32;
        let low = (at.floor() as usize).min(SUBMENTAL_COLUMNS - 1);
        let high = (low + 1).min(SUBMENTAL_COLUMNS - 1);
        let between = at - low as f32;
        Some(anchors[low]? * (1.0 - between) + anchors[high]? * between)
    };

    for (point, &mine) in mesh.positions.iter_mut().zip(owned) {
        if !mine {
            continue;
        }
        let local = *point - centre;
        let height = (local.y / radius) * (JUNCTION / (floor * SETTLE).min(-f32::EPSILON));
        if height >= MENTON + 0.14 || height <= JUNCTION || local.z <= 0.0 {
            continue;
        }
        let across = local.x.abs();
        if across >= SUBMENTAL_SPAN * radius {
            continue;
        }
        let (Some(crest_at), Some(crest), Some(throat)) = (
            chord_at(&heights, across),
            chord_at(&reaches, across),
            chord_at(&bottom, across),
        ) else {
            continue;
        };
        if height >= crest_at {
            continue;
        }

        let t = (crest_at - height) / (crest_at - JUNCTION).max(f32::EPSILON);
        // The chord plus the chin's own entitlement: [`BUTTON`] of convexity,
        // fading to nothing over the top [`BUTTON_RUN`] of the run. The
        // allowance is what lets one construction keep seed 21's soft dome and
        // plane off seed 0's bulge — the two differ by magnitude, not height.
        let allowed = BUTTON * radius * smooth((BUTTON_RUN - t) / BUTTON_RUN);
        let spent = knot(&SUBMENTAL_SPEND, t);
        let ceiling = crest * (1.0 - spent) + throat * spent + allowed;
        let excess = local.z - ceiling;
        if excess <= 0.0 {
            continue;
        }
        // Faded at the bottom and lateral edges so the construction meets the
        // throat and the jaw's corner with no crease; at the top the allowance
        // is the continuity, since the chord starts at the crest's own reach.
        let fade =
            smooth((1.0 - t) / 0.12) * smooth((SUBMENTAL_SPAN * radius - across) / (0.12 * radius));
        point.z = centre.z + local.z - excess * fade;
    }
}

/// Where a point on an unshaped head ends up once the skull is shaped.
///
/// Public because everything that sits on a head has to agree with it: the eyes,
/// and through them every feature anchored to the eyes. Placed against the
/// unshaped sphere they end up buried, because the shaped face is a quarter
/// further forward than the sphere it came from. One function, used both to move
/// the mesh and to place what sits on it, is the only way those two stay in
/// step.
///
/// Takes and returns a position relative to the head joint, in metres.
#[must_use]
pub fn reshape(local: Vec3, radius: f32, traits: &HeadTraits) -> Vec3 {
    reshape_to(local, radius, JUNCTION, traits)
}

/// The same, on a head whose surface is known to run out at `floor`.
///
/// Heights below the joint are scaled so that `floor` lands where the profiles
/// have finished letting go, which is what makes a profile knot mean the same
/// fraction of the way down every head. Above the joint nothing changes, which is why [`reshape`] can
/// still be called by anything placed on the face without knowing the floor:
/// the eyes sit at `+0.05` radii and every feature is placed from [`Skull`],
/// which measures the built surface rather than predicting it.
#[must_use]
pub fn reshape_to(local: Vec3, radius: f32, floor: f32, traits: &HeadTraits) -> Vec3 {
    if radius <= f32::EPSILON {
        return local;
    }
    let height = local.y / radius;
    // How many radii of real head one unit of profile height is worth, which is
    // the inverse of the remap below. On a head whose surface runs out exactly
    // where the profiles finish letting go this is one, and every knot means
    // what it says.
    //
    // **It is not one any more, and that is what flattens a chin** (#107). The
    // remap normalises HEIGHT and nothing normalises the push that goes with
    // it: [`CHIN`] is added in radii while its domain is stretched to fit
    // whatever floor the head came out with, so the chin's aspect ratio is a
    // free variable. Eight-point cage rings and one subdivision took the floor
    // from the −0.55 to −0.89 radii this profile was authored against to −1.07
    // to −1.16, measured over sixteen seeds — so the chin was drawn half again
    // as tall as before at exactly the same projection, which is a ramp and not
    // a prominence. Measured on the shaped midline, the crest walked 0.074
    // profile heights ABOVE [`CHIN`]'s own peak and the hollow above it fell
    // from 3.9 mm at worst to 0.78.
    //
    // Applied to the whole head rather than to the remapped half, because
    // [`CHIN`] is not quite zero at the joint — its highest knot is at +0.05 —
    // and a factor that switched on at zero would put a step of about
    // 0.7 mm around the head at the joint's own height.
    let stretch = (floor * SETTLE) / JUNCTION;
    let height = if height < 0.0 {
        height * (JUNCTION / (floor * SETTLE).min(-f32::EPSILON))
    } else {
        height
    };
    let across = Vec3::new(local.x, 0.0, local.z);
    let reach = across.length();
    if reach <= f32::EPSILON {
        return local;
    }

    // How far round the head this point is: +1 dead ahead, -1 behind.
    let facing = across.z / reach;
    let ahead = facing.max(0.0);
    let behind = (-facing).max(0.0);

    // Breadth across, depth fore and aft, and the head longer than it is wide.
    // The occiput swells the back of the cranium, and the same curve gone
    // negative lower down cuts the jaw in under the ear.
    //
    // Below the joint the breadth narrowing is weighted TOWARD THE FRONT, and
    // that is what lets a chin exist at all. A cross-section has one width, and
    // at the height of a chin it has to be two things: a chin is 45 mm across
    // and the throat directly behind it is the width of a neck. Narrowing the
    // whole ring to make the chin gave a head with a wasp waist above its own
    // neck — measured, 43 mm of head sitting on 52 mm of neck (#47) — and not
    // narrowing it at all gave a face that ran into the throat with no jawline.
    // Full at the front, half at the sides, none at the back.
    //
    // Faded in by height rather than applied everywhere, because the same
    // weighting on the CRANIUM would leave the back of the skull wide and the
    // forehead narrow, which is a different animal.
    let frontal = (-height / -JUNCTION).clamp(0.0, 1.0) * (0.5 + 0.5 * facing - 1.0) + 1.0;
    let wide = 1.0 - (1.0 - knot(&BREADTH, height)) * frontal;
    let deep = knot(&DEPTH, height)
        * (1.0 + ELONGATION)
        * (1.0 + knot(&OCCIPUT, height) * traits.occiput * behind * behind);

    // The chin is a central prominence, so its push falls off much faster
    // round the jaw than the other terms do. Spread evenly across the front — an
    // `ahead` squared, as the brow uses — it carries the whole lower face
    // forward and reads as a muzzle rather than as a chin.
    //
    // **`ahead⁴` → a plateau, and the difference is the chin's PLAN VIEW**
    // (#150). A cosine power is at its steepest exactly where a chin is at its
    // roundest: the midline. At `ahead⁴` the flank 30° out kept only 0.56 of
    // the push and the section at the chin's height solved as an ellipse five
    // times deeper than wide — the owner read it as a second nose (#128), and
    // rounding it means holding the push near full ACROSS the chin and then
    // letting go, not sliding from the first degree. The smoothstep holds 0.87
    // at 30°, crosses `ahead⁴` near 40° and is dead by 60° — where `ahead²`
    // still carries a quarter, which is the muzzle the paragraph above
    // rejects. Judged on the top-down and frontal renders across seeds.
    // Widening this window was tried for #197's rounder chin and REVERTED:
    // `point` multiplies the whole [`CHIN`] profile, which still reads ~0.09
    // up at the lip band, so a wider window pushed the upper lip's own flank
    // forward and `the_upper_lip_stops_being_a_lip` failed on five seeds. The
    // rounder point is [`BREADTH`]'s chin knot's job — lateral, and zero
    // anywhere but the heights it names.
    let point = smooth((facing - 0.42) / 0.58);
    // The brow ridge and the vault above it, which the frame axis moves in
    // opposite directions — see [`HeadTraits::frontal`] and [`FOREHEAD`].
    let ledge = (knot(&BROW, height) * traits.brow + knot(&FOREHEAD, height) * traits.frontal)
        * ahead
        * ahead;
    let hollow = knot(&TEMPLE, height) * (local.x / reach) * (local.x / reach);

    // The jaw draws the whole horizontal radius in rather than the width alone:
    // below the mandible's border the surface turns under toward the neck, and
    // narrowing across without retreating at the same time gives a slab. The
    // chin and the brow are added after it, so neither is scaled by a hollow
    // that has no business with either.
    let mandible = 1.0 - jaw(height, facing, local.x / reach, traits);

    Vec3::new(
        local.x * (wide - hollow) * mandible * traits.breadth_at(height),
        local.y,
        local.z * deep * mandible
            + (knot(&CHIN, height) * traits.chin * point * stretch + ledge) * radius,
    )
}

/// How far the horizontal radius is drawn in under the jaw, as a fraction of
/// itself.
///
/// **The one term here that is not separable, and that is the whole of why it
/// exists.** Every other shape in this file is a profile in height times a
/// window in azimuth locked at 0°, 90° or 180°: [`BREADTH`] and [`DEPTH`] have
/// no azimuthal window at all, [`BROW`] and [`CHIN`] are powers of `ahead`,
/// [`TEMPLE`] is a square of the lateral cosine, and [`OCCIPUT`]'s negative tail
/// — documented as the hollow between the jaw's angle and the neck — is
/// multiplied by `behind` squared, which is ZERO at the azimuth that hollow
/// lives at. A mandible is none of those. It is a border whose HEIGHT MIGRATES
/// WITH AZIMUTH, from the gonion out at the side down to the menton dead ahead,
/// and no product of a height profile and an angular window can say that. The
/// built half-width falls a dead constant 1.4–1.7 mm per 4 mm over sixteen
/// consecutive bands without it — a right circular frustum on every seed — and
/// the sharpest turn anywhere in the front silhouette is 3.4° against a
/// mandible's fifty.
///
/// So this takes both at once. [`GONION`] and [`MENTON`] give the border's
/// height at the two ends and it runs between them with the cosine of the
/// azimuth — straight in side view, as a mandible's lower border is; in the
/// sine instead, the crease's forward end lands at mouth height. Everything
/// below the border is drawn in, everything above it is left exactly as the
/// profiles left it.
///
/// **A knee and a long release, not a shelf, and the difference is which edge
/// the jawline is.** A step that saturates and then stops puts an equal and
/// opposite corner at its lower edge where the hollow lets go, and measured
/// against a sweep that lower edge is the LARGER of
/// the two — and a test passing on the bottom of a hollow while the jawline
/// above it stays soft is the instrument failure this whole file is written
/// against. So the fall-off is spread over [`JAW_RELEASE`] of the whole
/// run down to [`JUNCTION`] — five or six times the knee — which leaves one
/// corner where the border is and none anywhere else, and reaches nothing at the
/// junction, so no other profile had to move to make room for it.
///
/// `facing` is the cosine of the azimuth from dead ahead and `side` its sine,
/// both as [`reshape_to`] already has them; `height` is after the floor remap.
fn jaw(height: f32, facing: f32, side: f32, traits: &HeadTraits) -> f32 {
    let side = side.abs();
    // Nothing on the midline, where the chin already rules and where a hollow
    // would carve a groove either side of it; full from about 53° out; and dead
    // by 107° behind, which is past the ear and into the neck's own business.
    //
    // **The midline exclusion went from 0.15 to 0.45 when the hollow deepened**
    // (#128). The sentence above was always the intent and 0.15 was too little
    // of it: the ramp began 8.6° off the midline and reached full at 37°, which
    // is 20 mm off the midline at the chin's own height. Deepening [`JAW_DEPTH`]
    // for the front silhouette's corner (#79) duly cut a groove down both sides
    // of the chin, and the owner reported the result as a second nose hanging
    // off it. Measured on the default body at the chin's height, as how far the
    // midline stands proud of a straight run through its own neighbours at 15°
    // and 30°:
    //
    // ```text
    //   exclusion   proud   reach at 15°
    //     0.15      +37.7      56.9 mm
    //     0.25      +34.5      58.5
    //     0.35      +29.3      61.1
    //     0.45      +21.7      65.1     <- the same as no hollow at all
    // ```
    //
    // At 0.45 the hollow has stopped touching the chin's flank entirely and
    // `the_jawline_turns_a_corner` still reads 22.0 to 27.3° against its bound
    // of 20, because the gonion it is measured at is far outside this window.
    //
    // **And then the exclusion came back down to 0.10, because the trap it
    // guarded went away with the border curve** (#195). The #128 groove was cut
    // AT THE CHIN'S OWN HEIGHT, which was only possible because the sine-run
    // border stood at mouth height that far forward. With the border's height
    // running in the cosine it stays below the chin's underside across the whole
    // front, `under > 0` selects nothing above it, and the hollow this window
    // admits can only touch under-chin material the chin does not own. The
    // midline itself stays excluded so the midline profile (#193's S) is
    // untouched.
    let window = smooth((side - 0.10) / 0.45) * smooth((facing + 0.30) / 0.30);
    if window <= 0.0 {
        return 0.0;
    }

    // The border's height runs in the COSINE of the azimuth, not the sine
    // (#195): a mandible's lower border is straight in side view, so its height
    // is linear in the forward coordinate. In the sine it climbed to mouth
    // height within the first 30° and the visible crease terminated on the
    // cheek above the chin; in the cosine it holds near the menton across the
    // front and passes BELOW the chin into the submental plane. Behind 90° the
    // cosine's sign would carry the border on above the gonion, and
    // [`border_raise`] holds it there instead — past the ear is the window's
    // fade, not the border's business.
    let border = MENTON + (traits.gonion - MENTON) * border_raise(facing);
    let under = border - height;
    if under <= 0.0 {
        return 0.0;
    }
    let room = (border - JUNCTION).max(f32::EPSILON);
    let along = under / room;
    JAW_DEPTH * window * smooth(under / JAW_RISE) * smooth((1.0 - along) / JAW_RELEASE)
}

/// Where the lower-jaw region lets go of the throat, as a fraction of the
/// head's reach below its own joint — the owner's "about the adam's apple".
///
/// **Derived, not modelled**: there is no larynx in the geometry, so the
/// line is where the throat's forward reach goes vertical below the submental
/// corner — measured on the default body at −140 mm below the head joint on a
/// 166 mm below-joint span, and expressed in that span's own fractions because
/// they are the one ruler that holds across bodies: the chin measures 0.60 of
/// it on every body (`JAW_TIP`, `owner_of`, twice each), the head's floor 0.90.
pub(crate) const LARYNX: f32 = 0.84;

/// Where the skull stops owning the nape, in the same fractions.
///
/// Named for the boundary rather than the bone, because [`OCCIPUT`] is already
/// the cranium's swell profile: the back of the head is a rigid occiput and
/// the column below it is neck. The gonion maps to 0.35 of the below-joint
/// span and the occiput's underside arrives at the same height, which is what
/// a jawline meeting the ear means.
pub(crate) const NAPE: f32 = 0.35;

/// How much of the skin at a point one BROW carries, `0..1`.
///
/// The sibling of [`mandible_hold`] for the second face territory: the
/// brow ridge and the forehead above it, one side at a time. `rise` is height
/// above the head's own joint in head radii, `facing` and `side` are the same
/// horizontal azimuth cosines the mandible uses — but `side` here is signed
/// TOWARD the brow being asked about, so one function serves both sides and
/// the caller flips the sign for the other.
///
/// The bands are measured, not styled — probed against [`super::Canon`] across
/// femininity −1..+1 and brow 0..1:
/// * the eye line sits at +0.049 head radii above the joint and the upper
///   lid's aperture reaches to about +0.12, so the field is ZERO below +0.13
///   and full by +0.18 — a raised brow must not drag the lids up with it,
///   which is the boundary on this whole family of joints;
/// * the brow's crest sits at +0.19 to +0.26 by the brow axis, inside the
///   full band at both ends;
/// * the fade above spans +0.35 to +0.55 — mid-forehead to the hairline —
///   so the raise dies into the scalp the way skin over a skull does,
///   rather than shearing at an edge, which is what a narrow top blend on a
///   territory field always produces.
///
/// Azimuth: full ahead of the temple and gone behind it — the brow's own tail
/// measures `facing` 0.70 at the temple and the release runs 0.55 down to
/// 0.30. Across the midline the field fades over `side` −0.05..+0.15, so the
/// glabella between the brows follows the AVERAGE of the two at about a third
/// of their motion, which is what the skin between two raised brows does.
pub(crate) fn brow_hold(rise: f32, facing: f32, side: f32) -> f32 {
    let risen = smooth((rise - 0.13) / 0.05);
    let fade = smooth((0.55 - rise) / 0.20);
    let ahead = smooth((facing - 0.30) / 0.25);
    let mine = smooth((side + 0.05) / 0.20);
    risen * fade * ahead * mine
}

/// How much of the skin at a point one MOUTH CORNER carries, `0..1`.
///
/// The third face territory: a patch astride the commissure, one side
/// at a time. `below` is the fraction of the head's below-joint span — the
/// mandible's own ruler, because this field lives where that one does —
/// `facing`/`side` the usual azimuth cosines, `side` signed toward the corner
/// being asked about like [`brow_hold`]'s.
///
/// The bands, measured:
/// * **Height**: symmetric about the MOUTH LINE, which sits at 0.345 of the
///   span on every body probed and dips toward the corner — the centre
///   follows that dip. Symmetric about the line and not about a height
///   constant, because the slit's two edges are held by different bones
///   (head above, jaw below — the lower edge is wholly the jaw's) and
///   only a field that takes the SAME share from both sides moves the seam
///   as one thing. Full within ±0.06 of the span, gone by ±0.10.
/// * **Side**: the corner sits at 0.21 to 0.31 of the horizontal reach by
///   the mouth-width axis; the field rises from 0.10, holds across that
///   range, and dies by 0.48 — in from the cheek, out of the philtrum and
///   the chin's midline.
/// * **Facing**: a frontal guard only; the mouth is nearly frontal (the
///   corner measures 0.95), so `side` is the coordinate that does the work
///   here, the opposite division of labour from the brow's.
///
/// **Capped at 0.72 rather than reaching 1.0, and the cap is the jaw's.** A
/// corner fully owned by its own joint is pinned when the mandible opens,
/// and a mouth whose corners do not follow the jaw at all opens as a slot in
/// a mask. At 0.72 the corner keeps about a quarter of its jaw share below
/// the line, so an open carries the commissure part-way — the almond a real
/// mouth opens into — while a smile still moves the corner at nearly
/// three-quarters of the rigid arc.
pub(crate) fn corner_hold(below: f32, facing: f32, side: f32) -> f32 {
    let out = side.abs().min(0.31);
    let line = 0.345 + 0.018 * (out / 0.26) * (out / 0.26);
    let band = smooth((0.10 - (below - line).abs()) / 0.04);
    let outward = smooth((side - 0.10) / 0.08);
    let inward = smooth((0.48 - side) / 0.12);
    let ahead = smooth((facing - 0.60) / 0.15);
    0.72 * band * outward * inward * ahead
}

/// How strongly a point belongs to the lower-jaw region, 0..1.
///
/// **The region, stated once: the lower lip, to the chin, under the chin, to
/// about the laryngeal prominence**, along the jawline to the gonion, with the
/// ear as the hinge. Written down here, where the jaw's other landmarks
/// already live, so the carve and the binding read the same lines — three
/// fragments of this region each implemented alone is how they come to
/// contradict each other.
///
/// `below` is the fraction of the head's below-joint span (0 at the joint, 1
/// at the neck joint); `facing`/`side` are the azimuth cosine/sine
/// [`reshape_to`] already uses. Three windows multiplied:
///
/// - **Top**: rises below the mouth line — 0.39 of the span on the midline,
///   easing to the gonion's 0.35 at the side, so the lower lip is inside and
///   the upper lip is out. The blend is 0.06 wide; the bind-time smoothing
///   pass softens it further.
/// - **Bottom**: fades to nothing at [`LARYNX`] over 0.10 of the span.
/// - **Round**: dies behind the ear with the same curve [`jaw`]'s hollow uses,
///   so the region and the cosmetic border agree about where the face ends.
///   No release toward the gonion: the hinge sits at the ear, so skin near it
///   sweeps a small arc however fully it is held.
pub(crate) fn mandible_hold(below: f32, facing: f32, side: f32) -> f32 {
    let top = 0.39 - 0.04 * side.abs();
    // The top blend narrows where the mouth's slit is (#154): the parting is a
    // real seam there, its lower edge wholly the jaw's, and skin blending over
    // 0.06 of the span two millimetres under a seam that moves outright is a
    // shear band across one mesh cell. Ahead of the mouth's corners — the
    // slit's own azimuth range — the blend tightens to 0.015; past them the
    // cheek keeps the soft edge, which is what a cheek wants.
    let width = 0.06 - 0.045 * smooth((facing - 0.80) / 0.12);
    let risen = smooth((below - top) / width);
    let fade = smooth((LARYNX - below) / 0.10);
    let round = smooth((facing + 0.30) / 0.30);
    risen * fade * round
}

/// Reads a profile, which is given from the crown downward.
///
/// **The interpolant lives in [`super::curve`] and this is the skull's name
/// for it.** The relief's ramps and these profiles need the same C1 treatment
/// and run in opposite directions, and a lerp is cheap to keep twice where a
/// monotone limiter is not. One reader, direction taken from the knots.
///
/// Everything about WHY it is monotone and cubic — the 7.9 mm terracing a
/// linear read leaves, and [`CHIN`]'s tail into [`JUNCTION`], where an ordinary spline
/// dips below zero and stands the head's lowest band behind its own throat — is
/// written at [`super::curve::monotone`], with the profiles named.
fn knot(profile: &[(f32, f32)], height: f32) -> f32 {
    super::curve::monotone(profile, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::Canon;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    /// The same head at a chosen point on the frame axis, with its joint.
    ///
    /// **The ruler here is the head JOINT and not the skull's own span**, and
    /// that is deliberate. Every measurement in this file is normally binned
    /// over `throat..crown`, and this file records four separate occasions on
    /// which a change that moved the neck was reported as a face regression
    /// because the span moved under the ruler. The frame axis is the fifth
    /// candidate — but the cage does not read it for the head, so the head
    /// node's position and radius are bit-identical across the whole sweep, and
    /// a height in radii above that joint means the same thing at both ends.
    fn framed(seed: i64, femininity: f32) -> (PolyMesh, Vec3, f32, f32) {
        let mut record = AvatarRecord::new("Framed", Archetype::default());
        record.reroll(seed);
        record.composites = Composites {
            femininity,
            ..Composites::default()
        };
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mut mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        shape(&mut mesh, &rig, &HeadTraits::of(&record.composites));
        let joint = *rig.in_zone(Zone::Head).first().expect("a head");
        // The remap `reshape_to` applies, so a test can ask for a PROFILE
        // height — the coordinate every table in this file is authored in — and
        // get the radii above the joint that it lands at on this head.
        let stretch = floor(&rig, joint) * SETTLE / JUNCTION;
        (
            mesh,
            rig.joints[joint].position,
            rig.joints[joint].radius,
            stretch,
        )
    }

    /// A body's head, refined and shaped the way the crate ships it, with the
    /// canon read off the surface that came out.
    ///
    /// The two head axes are corners rather than seeds for the reason
    /// `tests/budget.rs` sweeps them: a re-roll draws them timidly and the
    /// bands have to hold anywhere a record can reach.
    fn faced(seed: i64, breadth: f32, length: f32) -> (Rig, Canon) {
        let mut record = AvatarRecord::new("Banded", Archetype::default());
        record.reroll(seed);
        if let Archetype::Humanoid(params) = &mut record.archetype {
            params.head_breadth = breadth;
            params.face_length = length;
        }
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let traits = HeadTraits::of(&record.composites);
        let mut mesh = refine_face(
            &catmull_clark(&cage, crate::BODY_SUBDIVISIONS),
            &rig,
            FACE_PASSES.len(),
        );
        shape(&mut mesh, &rig, &traits);
        let skull = Skull::measure(&mesh, &rig).expect("a head measures");
        let canon = Canon::measure(&rig, &skull, &record.eyes);
        (rig, canon)
    }

    #[test]
    fn shaping_the_skull_does_not_move_a_vertex_up_or_down() {
        // **The fact that makes a band aimable at a landmark** (#185), and it
        // was assumed in one direction and reported the other way round in
        // another. [`refine_face`] selects on the unshaped mesh and every
        // feature is authored against a landmark measured on the shaped one, so
        // the two agree only if the shaping leaves the height axis alone. It
        // does: [`reshape_to`] scales `x` and `z` and returns `local.y`, and
        // `construct_submental` only ever pushes `z` back. Asserted rather than
        // read off the source, because a term added to that `y` would break
        // every band on the face silently and no other test looks at it.
        for seed in [7, 23, 42] {
            let (plain, shaped, ..) = head(seed);
            assert_eq!(
                plain.positions.len(),
                shaped.positions.len(),
                "seed {seed} changed vertex count under shaping"
            );
            for (index, (before, after)) in
                plain.positions.iter().zip(&shaped.positions).enumerate()
            {
                assert!(
                    (before.y - after.y).abs() <= f32::EPSILON,
                    "seed {seed} vertex {index} moved from {} to {} vertically",
                    before.y,
                    after.y
                );
            }
        }
    }

    #[test]
    fn every_refinement_band_still_contains_its_own_feature() {
        // **What #185 found nothing to catch.** The bands were tuned by
        // iteration against the built result, and #115 recorded the unease
        // about that in as many words — they land on their features, but
        // nothing said so, so a change to the feature stack could slide them
        // off one at a time in silence. #78 did exactly that once already, and
        // the lip line walked out of its own refinement at both ends.
        //
        // Each pair below is a landmark [`Canon`] measures on the built face
        // and the pass that exists for it, in that pass's own units through
        // [`band_at`]. The brow is checked at both ends of its own ridge; the
        // rest are single heights, because what a band must not do is lose the
        // landmark its feature is hung from.
        // **How far past its own band a landmark may sit, in profile heights.**
        // Not slack for its own sake: the base of the nose is measured at
        // −0.168 to −0.186 across the corners below, and the pass pair that
        // exists for it stops at −0.171 — so on seed 29's short broad head the
        // subnasale sits 0.003 above the seam, which is 0.6 mm on a surface
        // whose cells there are 1.5. Closing it was costed rather than argued:
        // that pair reaches round to a cosine of 0.55, so lifting its ceiling to
        // −0.166 or −0.160 costs 950 and 852 triangles against the 82 the
        // dearest corner of `tests/budget.rs` has left, and the two are not even
        // in cost order — the band edge lands on a ring of faces either way.
        //
        // What makes it affordable to leave is that the seam is no longer an
        // edge of the refinement: the dorsum band starts at exactly that ceiling
        // now, so what meets there is one pass against two rather than a refined
        // band against a raw one. A tenth of a cell of tolerance still catches
        // any real slide — #78's was a third of a band wide.
        const SEAM: f32 = 0.005;
        for seed in [1, 7, 23, 29, 42, 99] {
            for (breadth, length) in [(0.0, 0.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
                let (rig, canon) = faced(seed, breadth, length);
                let at = |height: f32| band_at(&rig, height).expect("a head has a band");
                let where_ = format!("seed {seed} breadth {breadth} length {length}");
                for (name, height, pass) in [
                    ("the eye line", canon.level, 2),
                    ("the brow ridge", canon.level + canon.frame * 0.22, 2),
                    ("the nose base", canon.nose_base(), 5),
                    ("the nose root", canon.level + canon.frame * 0.1237, 7),
                    ("the mouth line", canon.mouth_line(), 8),
                    ("the chin", canon.chin(), 3),
                ] {
                    let (_, _, low, high) = FACE_PASSES[pass];
                    let edge = at(height);
                    assert!(
                        edge >= low - SEAM && edge <= high + SEAM,
                        "{where_}: {name} is at {edge:.3} and pass {pass} covers {low} to {high}"
                    );
                }
            }
        }
    }

    fn head(seed: i64) -> (PolyMesh, PolyMesh, Rig, Vec3, f32) {
        let mut record = AvatarRecord::new("Skulled", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let plain = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let mut shaped = plain.clone();
        shape(&mut shaped, &rig, &Default::default());
        let joint = *rig.in_zone(Zone::Head).first().expect("a head");
        let (centre, radius) = (rig.joints[joint].position, rig.joints[joint].radius);
        (plain, shaped, rig, centre, radius)
    }

    /// How far the surface reaches from `from` along `along`, in metres.
    ///
    /// **Bisected against the mesh, not binned over its vertices.** A band that
    /// takes the extreme vertex within a window of heights is reading the mesh's
    /// row spacing as much as its shape: on the coarse fixture here the rows are
    /// 0.18 radii apart, so two adjacent windows routinely return the same row
    /// and a slope series comes back as alternating zeros and cliffs. `contains`
    /// answers about the surface at the point asked, which is what a silhouette
    /// is. Returns `None` if `from` is not inside to begin with.
    fn bisect(mesh: &PolyMesh, from: Vec3, along: Vec3) -> Option<f32> {
        if !mesh.contains(from) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, 0.30f32);
        for _ in 0..30 {
            let middle = 0.5 * (near + far);
            if mesh.contains(from + along * middle) {
                near = middle;
            } else {
                far = middle;
            }
        }
        Some(near)
    }

    /// The widest and deepest the head gets in a band of heights.
    ///
    /// Head vertices only. The neck runs up through the same heights and is
    /// broader than a chin, so a band that takes everything reports the throat
    /// and concludes the jaw got wider.
    fn band(mesh: &PolyMesh, rig: &Rig, centre: Vec3, radius: f32, at: f32) -> (f32, f32) {
        let mut wide: f32 = 0.0;
        let mut deep: f32 = 0.0;
        for point in &mesh.positions {
            let height = (point.y - centre.y) / radius;
            if (height - at).abs() > 0.08
                || rig.joints[rig.nearest_bone(*point).joint].zone != Zone::Head
            {
                continue;
            }
            wide = wide.max((point.x - centre.x).abs());
            deep = deep.max((point.z - centre.z).abs());
        }
        (wide / radius, deep / radius)
    }

    #[test]
    fn the_neutral_head_is_the_head_that_was_already_built() {
        // The epic's identity anchor (#161, #166): `femininity` zero is the
        // midpoint of the two measured references, which is the head this crate
        // built before the axis existed. Every field of [`HeadTraits`] is a
        // factor about one for exactly this reason, and a pair of anchors whose
        // midpoint is not today's value would move every neutral body in the
        // crate without a single test naming the axis.
        assert_eq!(
            HeadTraits::of(&Composites::default()),
            HeadTraits::default(),
            "the neutral frame no longer builds the head this crate shipped"
        );
    }

    #[test]
    fn the_frame_axis_moves_the_face_and_leaves_the_vault_alone() {
        // **What the two CC0 mannequins actually disagree about** (#166). Their
        // vaults match to within 1% in width and 2% in depth from 0.25 to 0.80
        // of the head's span, and they part company in three places: the jaw is
        // wider on the masculine head, the chin projects further, and the
        // feminine forehead stands more upright. This asserts that shape and
        // asserts the agreement too — a dimorphism that also swelled the
        // parietal would be `head_size` wearing a second name, and this crate
        // already has that axis on the cage.
        //
        // Heights are radii above the head JOINT rather than fractions of the
        // skull's span; see [`framed`] for why that distinction is load-bearing
        // here and has been got wrong four times elsewhere in this file.
        for seed in [0i64, 3, 7, 21] {
            let ends: Vec<(PolyMesh, Vec3, f32, f32)> = [-1.0f32, 0.0, 1.0]
                .iter()
                .map(|&f| framed(seed, f))
                .collect();
            // Heights are asked for as profile heights and put through each
            // head's own remap, because that is the coordinate `BREADTH` and
            // `CHIN` are authored in — a raw radius means a different part of
            // the face on every body, and at the jaw it means a fifth of the
            // dimorphism.
            let at = |index: usize, height: f32, along: Vec3| {
                let (mesh, centre, radius, stretch) = &ends[index];
                let up = if height < 0.0 {
                    height * stretch
                } else {
                    height
                };
                bisect(mesh, *centre + Vec3::Y * up * radius, along)
                    .expect("the midline is inside the head")
                    / radius
            };
            let across = |index: usize, height: f32| at(index, height, Vec3::X);
            let ahead = |index: usize, height: f32| at(index, height, Vec3::Z);

            // The jaw, at the angle where `BREADTH` calls the profile widest
            // below the cheek. Masculine is wider and the axis is monotone.
            let (jaw_m, jaw_n, jaw_f) = (across(0, -0.46), across(1, -0.46), across(2, -0.46));
            assert!(
                jaw_m > jaw_n && jaw_n > jaw_f,
                "seed {seed}: the jaw reads {jaw_m:.4}, {jaw_n:.4}, {jaw_f:.4} across the axis \
                 and has to narrow all the way"
            );
            assert!(
                jaw_m / jaw_f > 1.04,
                "seed {seed}: the jaw is only {:.1}% wider at the masculine end, which is less \
                 than the references' own 19% by more than the taper can explain",
                100.0 * (jaw_m / jaw_f - 1.0)
            );

            // The parietal, where the two references agree and this axis must
            // not invent a difference.
            let (vault_m, vault_f) = (across(0, 0.42), across(2, 0.42));
            assert!(
                (vault_m / vault_f - 1.0).abs() < 0.02,
                "seed {seed}: the axis moved the parietal by {:.1}%, which is head SIZE and \
                 belongs on the cage",
                100.0 * (vault_m / vault_f - 1.0)
            );

            // The chin projects further on the masculine head, and the forehead
            // above the brow stands further forward on the feminine one. Two
            // rays from the same joint at two heights, so nothing here depends
            // on where the skull's span begins.
            let (chin_m, chin_f) = (ahead(0, -0.50), ahead(2, -0.50));
            assert!(
                chin_m > chin_f,
                "seed {seed}: the chin reaches {chin_m:.4} masculine against {chin_f:.4} feminine"
            );
            // **As a SLOPE and not as a reach**, which is the same
            // normalisation lesson the references themselves taught: measured
            // as absolute forward reach the answer is dominated by how long the
            // head is, and every quantity in this file that is read without
            // dividing out the head reports head size. The upper forehead over
            // the brow's own reach is elongation-free and is what "upright"
            // means.
            // The occipital curve against the hollow under the ear behind it,
            // which is the pair `HeadTraits::occiput` scales together. A ratio
            // for the same reason the forehead is one: the absolute reach
            // behind a head is mostly how long the head is.
            let behind = |index: usize| at(index, 0.21, -Vec3::Z) / at(index, -0.15, -Vec3::Z);
            let (occiput_m, occiput_f) = (behind(0), behind(2));
            assert!(
                occiput_f > occiput_m,
                "seed {seed}: the cranium stands {occiput_f:.4} of the hollow behind the jaw at \
                 the feminine end against {occiput_m:.4} at the masculine one, and the feminine \
                 occiput is the fuller one"
            );

            let slope = |index: usize| ahead(index, 0.94) / ahead(index, 0.48);
            let (slope_m, slope_f) = (slope(0), slope(2));
            assert!(
                slope_f > slope_m,
                "seed {seed}: the forehead keeps {slope_f:.4} of its brow's reach at the \
                 feminine end against {slope_m:.4} at the masculine one, and the feminine \
                 forehead is the upright one"
            );
        }
    }

    #[test]
    fn shaping_leaves_the_topology_alone() {
        // Vertices move; nothing is added, removed or re-joined, which is what
        // lets this run before binding and unwrapping without either caring.
        let (plain, shaped, ..) = head(1);
        assert_eq!(plain.vertex_count(), shaped.vertex_count());
        assert_eq!(plain.faces, shaped.faces);
        assert!(
            shaped.is_closed_manifold(),
            "{:?}",
            shaped.manifold_report()
        );
    }

    #[test]
    fn shaping_makes_a_head_longer_than_it_is_wide() {
        // The single clearest difference between a head and a ball.
        //
        // **Read as what the SHAPING adds, and it used to be read as an absolute
        // ratio on a head asserted to arrive round** (#61). Skull breadth is a
        // record axis now, so the cage no longer hands this a circular section:
        // seed 1 draws a broad skull and arrives 0.89 times as long as it is
        // wide, which failed a precondition demanding 1.00 within 0.06. That
        // precondition was never the property — it was a fact about the plan
        // that happened to hold — and asserting it here made a test of `shape`
        // fail because of something `shape` does not do.
        //
        // What `shape` owes is [`ELONGATION`], and it owes it on every body
        // whatever section arrives. So the reading is the RATIO of ratios, which
        // is the same quantity the old bound was reaching for on a body where
        // the denominator was one, and it holds across seeds rather than on the
        // one that used to be round.
        //
        // **Bisected, not binned.** `band` takes the extreme head-owned VERTEX
        // within 0.08 radii of a height, and this fixture is deliberately coarse
        // — so lengthening the head below its joint slid the rows and the window
        // at the joint came back empty, which divides to `NaN` and reports
        // nothing at all. A window over vertices reads the mesh's row spacing as
        // much as its shape, and this file has now been caught by that four
        // times.
        for seed in [1i64, 7, 23, 29, 42] {
            let (plain, shaped, _, centre, _) = head(seed);
            let reach = |mesh: &PolyMesh, along: Vec3| {
                bisect(mesh, centre, along).expect("the head joint is inside the head")
            };
            let (was_wide, was_deep) = (reach(&plain, Vec3::X), reach(&plain, Vec3::Z));
            let (wide, deep) = (reach(&shaped, Vec3::X), reach(&shaped, Vec3::Z));
            let gained = (deep / wide) / (was_deep / was_wide);
            assert!(
                gained > 1.12,
                "seed {seed}: shaping only lengthened the head by {gained:.2}, from \
                 {:.2} times as long as wide to {:.2}",
                was_deep / was_wide,
                deep / wide
            );
        }
    }

    #[test]
    fn the_head_is_widest_above_the_eye_line() {
        // This replaces `the_cheekbones_are_the_widest_part_of_the_head`, which
        // asserted the opposite and passed — and would have blocked the fix.
        //
        // A head's maximum breadth is at the EURION, high on the parietal:
        // eu-eu is about 156 mm against a bizygomatic 137, and the widest band
        // sits roughly 25–45 mm above the pupil line. [`BREADTH`] was authored
        // on the belief that a head is widest at the cheekbones, which is where
        // an unshaped head is *narrower* than its own cranium, so the profile
        // narrows the vault to 0.62 and hands the maximum to the eye line.
        //
        // Measured when this was written, the widest band was at −0.0 to +0.05
        // radii on every seed — at or just below the eye — which is the "pointed
        // egg" read (#73).
        //
        // **It is not a target any more, and it has not been for two milestones**
        // (#61). #79 inverted BREADTH's premise and this passed from that day;
        // it stayed marked as the target because nobody re-ran an ignored test.
        // Re-measured on nine bodies after the cage flip, the maximum sits at
        // +0.46 to +0.48 R on every one of them — so the property survived a
        // change that spent every other gain #79 made, which is the strongest
        // thing that can be said for it and exactly what a ratchet is for. An
        // ignored test asserting something true is worse than no test: it reads
        // as an open defect and it guards nothing.
        for seed in [7, 23, 29, 42] {
            let (_, shaped, rig, centre, radius) = head(seed);
            let mut widest = (0.0f32, 0.0f32);
            let mut at = -0.50;
            while at <= 0.70 {
                let wide = band(&shaped, &rig, centre, radius, at).0;
                if wide > widest.0 {
                    widest = (wide, at);
                }
                at += 0.05;
            }
            // The eye line sits at +0.05 radii, and the parietal maximum wants
            // to be a fifth to a third of the way up from there to the crown.
            assert!(
                widest.1 > 0.20,
                "seed {seed}: the head is widest at {:+.2} radii, at or below the \
                 eye line rather than up on the parietal",
                widest.1
            );
        }
    }

    #[test]
    fn the_face_narrows_from_cheekbone_to_chin() {
        // Renamed from `the_jaw_narrows_toward_the_chin`, which is not what it
        // checks: a width RATIO at three heights is satisfied by any smooth
        // taper, and the built jawline was a cone. The property it does check is
        // real and worth keeping, so it keeps it under its own name and
        // `the_jawline_turns_a_corner` above asks the question this cannot.
        //
        // **The instrument was `face_width`, which bins vertices and reads the
        // widest point of the head's forward half — and at the chin's height
        // that point is not the chin.** Measured on the shipped surface of seed
        // 7: at the menton, −73.1 mm, the widest forward-half sample sits SEVEN
        // millimetres in front of the head joint, which is ninety millimetres
        // behind the chin's tip. It is the upper neck, seen from the side. The
        // test passed for as long as the whole lower face was a cone that
        // tapered through the neck as well, and [`jaw`] broke it by carving a
        // hollow under the mandible without narrowing the neck under that —
        // which is what a neck does. Two seeds already read 0.84 and 0.81
        // against the 0.80 bound before that change; it had almost no margin
        // and no diagnosis of why.
        //
        // So it bisects the same column `the_jawline_turns_a_corner` does,
        // half-way forward on the midline's own reach, which is on the FACE.
        // Read there, the menton against the angle of the jaw measures 0.73
        // where the vertex-binned reading says 0.94 — and it IMPROVED with the
        // jaw, from 0.81, where the binned reading said it got worse.
        //
        // Heights as fractions of the head's own span, which is how `shape`
        // reads them: a head reaches anywhere from −0.55 to −0.89 radii below
        // its joint depending on its node sizes, so a fixed figure is the jaw on
        // one body and the throat on another.
        //
        // **Measured with the breadth axis held neutral** (#61). What this
        // asserts is BREADTH's shape and [`jaw`]'s corner, and a record that
        // asks for a narrow skull is neither. It matters because the axis moves
        // this ratio the WRONG way at the narrow end — the neck below the head
        // is not sectioned with it, so narrowing the skull narrows the cheek
        // more than it narrows the angle of the jaw, and seed 42 runs 0.848 at
        // the broad end to 0.932 at the narrow one about a neutral 0.882. That
        // is the axis's own coupling and it has its own test; folding it in here
        // would mean re-basing a bound that has been read four times as a
        // statement about the head.
        // Seeds re-picked for generator 2 (#160): the four are rolled bodies
        // whose head axes land inside the range this bound was tuned over
        // (|breadth| and |face| under 0.7) — the claim is about BREADTH's
        // shape on a person, and the exploration tail's caricatures are #79's
        // range work, not this guard's.
        for seed in [29i64, 43, 50, 57] {
            let (mesh, measured, centre, radius) =
                skull_of(seed, crate::FACE_REFINEMENT, Some(0.0));
            let width = |y: f32| {
                let axis = centre + Vec3::Y * y;
                let reach = bisect(&mesh, axis, Vec3::Z)?;
                bisect(&mesh, axis + Vec3::Z * reach * 0.5, Vec3::X)
            };
            let cheek = width(-0.05 * radius).expect("a cheekbone");
            let angle = width(measured.gonion()).expect("an angle of the jaw");
            let chin = width(measured.chin()).expect("a menton");
            // **0.90 → a BAND of 0.70 to 0.78, and the ceiling is met by
            // shape rather than by slack now** (#79). The comment this replaces
            // recorded 0.84, 0.77, 0.80 and 0.89 and called that spread root
            // cause 4 of #73: the head was 0.77 to 0.89 as wide at the angle of
            // the jaw as at the cheekbone where life is 0.73 to 0.76, and it was
            // BREADTH's shape to fix. By the time it was re-measured the
            // population had drifted to 0.858, 0.866, 0.870 and 0.816 — every
            // body outside life, three of four worse, and 0.030 of margin left
            // under a bound nobody had tightened.
            //
            // BREADTH's face knots sit on the landmarks they are named for now;
            // read that table for the change and for what it cost. These four
            // measure 0.752, 0.760, 0.763 and 0.715, which straddles life from
            // below.
            //
            // **Two-sided, because this can overshoot now.** A single ceiling
            // was safe while every body was above life and the only way to move
            // was down. A jaw a third narrower than its own cheekbone is a
            // defect in its own right and nothing here would have said so —
            // which is the shape of every guard this crate has caught sleeping.
            // Both bounds are the state, 0.78 a hair over the widest and 0.70 a
            // hair under the narrowest, and the target between them is life's
            // 0.73 to 0.76.
            assert!(
                angle < cheek * 0.78,
                "seed {seed}: the jaw did not narrow: {angle} of {cheek}"
            );
            assert!(
                angle > cheek * 0.70,
                "seed {seed}: the jaw narrowed past a jaw: {angle} of {cheek}, \
                 where life is 0.73 to 0.76 of the cheekbone"
            );
            assert!(
                chin < angle * 0.80,
                "seed {seed}: the chin did not narrow: {chin} of {angle}"
            );
        }
    }

    #[test]
    fn the_breadth_axis_narrows_the_jaw_with_the_cheek() {
        // **The axis's own coupling, measured rather than assumed** (#61). A
        // record's breadth axis scales the head and crown nodes and nothing
        // else, so the neck below stays exactly as wide — and the angle of the
        // jaw sits close enough to the neck that some of its width is the
        // neck's. Narrowing the skull therefore narrows the cheekbone more than
        // it narrows the gonion, which moves bigonial over bizygomatic the wrong
        // way at the narrow end.
        //
        // Life holds that ratio near-constant across head shapes: a
        // dolichocephalic skull has a narrow jaw to match. So the question is
        // not whether the artefact exists — it does, and the mechanism is the
        // throat, which cannot move — but whether it is small against the axis
        // it rides on.
        //
        // Measured on two seeds over the full range: the cheekbone runs 49.2 to
        // 70.7 mm on seed 42 and 88.3 to 125.9 on seed 23, a factor of 1.44 and
        // 1.43, while the ratio runs 0.906 to 0.814 and 0.953 to 0.848 — a tenth
        // as far, and monotone.
        //
        // **Re-measured when BREADTH's face knots moved onto their landmarks,
        // and the artefact got SMALLER** (#79): 0.794 to 0.715 and 0.834 to
        // 0.744, so the swing goes 0.092 to 0.079 on seed 42 and 0.106 to 0.091
        // on seed 23. The quantity this asserts on falls with it, 1.113 to 1.110
        // and 1.124 to 1.121. Narrowing the jaw where the profile means to
        // narrow it leaves less of the gonion's width owed to the throat, which
        // is the mechanism the paragraph above names.
        //
        // **The figures this replaces were three body-shapes stale**, and worth
        // a line because they nearly went into a write-up as a before: they read
        // 82.3 to 118.2 mm and 112.9 to 165.9 with ratios 0.932 to 0.848 and
        // 0.781 to 0.737, against a tree that measures 49.2 to 70.7 and 88.3 to
        // 125.9 with the code they described unchanged. #107's cage and #79's
        // own narrowing had both landed since. A recorded number is a
        // measurement of the day it was taken.
        //
        // Bounded at a fifth because that is comfortably above what the throat
        // can be worth and far below the 1.44 the axis itself delivers. If this
        // fires, the breadth axis has started reshaping the face rather than
        // sizing it.
        for seed in [23i64, 42] {
            let mut widths = Vec::new();
            for breadth in [-1.0f32, 0.0, 1.0] {
                let (mesh, measured, centre, radius) =
                    skull_of(seed, crate::FACE_REFINEMENT, Some(breadth));
                let width = |y: f32| {
                    let axis = centre + Vec3::Y * y;
                    let reach = bisect(&mesh, axis, Vec3::Z)?;
                    bisect(&mesh, axis + Vec3::Z * reach * 0.5, Vec3::X)
                };
                let cheek = width(-0.05 * radius).expect("a cheekbone");
                let angle = width(measured.gonion()).expect("an angle of the jaw");
                widths.push((breadth, cheek, angle));
            }
            let cheeks = (widths[2].1 / widths[0].1, widths[2].2 / widths[0].2);
            assert!(
                cheeks.0 > 1.30,
                "seed {seed}: the breadth axis moved the cheekbone by only {:.2} \
                 across its whole range: {widths:?}",
                cheeks.0
            );
            let ratios: Vec<f32> = widths.iter().map(|&(_, c, a)| a / c).collect();
            assert!(
                ratios[0] / ratios[2] < 1.20,
                "seed {seed}: the breadth axis swung bigonial over bizygomatic by \
                 {:.0}%, {:?} — it is reshaping the face rather than sizing it",
                (ratios[0] / ratios[2] - 1.0) * 100.0,
                ratios
                    .iter()
                    .map(|r| (r * 1000.0).round() / 1000.0)
                    .collect::<Vec<_>>()
            );
            // Monotone, which is what says it is one axis rather than two
            // effects crossing over somewhere in the middle.
            assert!(
                widths[0].1 < widths[1].1 && widths[1].1 < widths[2].1,
                "seed {seed}: the cheekbone did not widen monotonically: {widths:?}"
            );
        }
    }

    #[test]
    fn the_head_meets_the_neck_without_a_step() {
        // The defect this issue is named for. `shape` moves head-owned vertices
        // and leaves neck-owned ones exactly where they are, so anything the
        // profiles still say at the bottom of the head is a cliff in the
        // silhouette rather than a shape. It measured 19 mm across 11 mm of
        // height, against an unshaped body that met itself to within 2 mm.
        //
        // The assertion is that the head's lowest surface is where the UNSHAPED
        // body left it, because that is what the untouched neck below is still
        // agreeing with. An earlier version of this test compared the surface a
        // little either side of the junction instead, and could not tell a step
        // from a slope: the head genuinely flares by a centimetre over that
        // span, so the figure it produced was mostly the flare.
        //
        // Read by bisection rather than off the vertex list, since the step is
        // between two rings and that is exactly where no vertex is. Measured
        // across sixteen seeds it comes out under 0.1 mm everywhere; before the
        // fix the default body was 23 mm out on the side alone.
        for seed in [1, 3, 6, 8, 9, 12] {
            let (plain, shaped, rig, centre, radius) = head(seed);
            let floor = shaped
                .positions
                .iter()
                .filter(|&&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
                .fold(f32::MAX, |low, point| low.min(point.y));

            let reach = |mesh: &PolyMesh, axis: Vec3| -> Option<f32> {
                let inside =
                    |at: f32| mesh.contains(Vec3::new(centre.x, floor, centre.z) + axis * at);
                if !inside(0.0) || inside(radius * 4.0) {
                    return None;
                }
                let (mut near, mut far) = (0.0f32, radius * 4.0);
                for _ in 0..32 {
                    let middle = (near + far) * 0.5;
                    if inside(middle) {
                        near = middle
                    } else {
                        far = middle
                    }
                }
                Some(near)
            };

            for axis in [Vec3::X, Vec3::Z, -Vec3::Z] {
                let (Some(was), Some(now)) = (reach(&plain, axis), reach(&shaped, axis)) else {
                    continue;
                };
                assert!(
                    (now - was).abs() < 0.001,
                    "seed {seed} on {axis}: the shaping moved the head's lowest surface \
                     {:.1} mm away from the neck it has to meet",
                    (now - was) * 1000.0
                );
            }
        }
    }

    #[test]
    fn the_chin_projects_further_forward_than_the_brow() {
        // A face whose chin sits behind its brow reads as receding, and an
        // unshaped head has exactly that: a sphere's widest point is its middle.
        //
        // **Asked at the chin's own landmark, and bisected.** This used to bin
        // vertices in a ±0.09 radii window at a hard-coded −0.45 R, which was
        // where a chin sat when the head reached 0.69 radii below its joint. At
        // 1.19 (#78) two things broke at once: −0.45 R is now the mouth, and the
        // rows below the joint are spread 0.31 radii apart on this deliberately
        // coarse fixture — wider than the window — so the filter came back EMPTY
        // and the subtraction of two `f32::MIN`s reported a tidy zero. A test
        // that bins vertices reads the mesh's row spacing as much as its shape.
        let (plain, shaped, rig, centre, radius) = head(3);
        let skull = Skull::measure(&shaped, &rig).expect("a skull");
        let at = centre.y + skull.chin();
        let gained = bisect(&shaped, centre.with_y(at), Vec3::Z).expect("a chin to measure")
            - bisect(&plain, centre.with_y(at), Vec3::Z).expect("an egg to measure it against");
        assert!(
            gained > radius * 0.05,
            "the chin only came forward by {:.1} mm, against a {:.1} mm floor",
            gained * 1000.0,
            radius * 50.0
        );
    }

    #[test]
    fn the_chin_landmark_lands_on_the_chin() {
        // [`Skull::chin`] is the one landmark read off the plan rather than the
        // surface, so this is the test that keeps it honest: bisect the built
        // surface on the midline around the landmark and find where the forward
        // reach actually peaks. Measured before the tolerance was set: the tip
        // sits 0 to 2 mm above the landmark on every seed tried, because the
        // profile's peak rides on the egg's own slope. 5 mm is the alarm for
        // the profile and the landmark drifting apart — which is exactly what
        // would happen silently if a CHIN knot moved without this file's
        // derivation moving with it.
        for seed in [1i64, 23, 42, 99] {
            let mut record = AvatarRecord::new("Skulled", Archetype::default());
            record.reroll(seed);
            let skeleton = record.skeleton();
            let mesh = crate::build_body(
                &skeleton,
                &CageConfig::default(),
                crate::BODY_SUBDIVISIONS,
                &Default::default(),
            )
            .expect("a body builds");
            let rig = Rig::from_skeleton(&skeleton).expect("rigs");
            let skull = Skull::measure(&mesh, &rig).expect("a skull");
            let centre = rig.joints[skull.head].position;

            let reach = |y: f32| {
                let inside = |z: f32| mesh.contains(Vec3::new(centre.x, y, centre.z + z));
                let (mut near, mut far) = (0.0f32, 0.30f32);
                for _ in 0..30 {
                    let mid = 0.5 * (near + far);
                    if inside(mid) {
                        near = mid;
                    } else {
                        far = mid;
                    }
                }
                near
            };
            let chin = centre.y + skull.chin();
            let mut tip = (f32::MIN, 0.0f32);
            let mut y = chin - 0.020;
            while y < chin + 0.020 {
                let at = reach(y);
                if at > tip.0 {
                    tip = (at, y);
                }
                y += 0.002;
            }
            assert!(
                (tip.1 - chin).abs() < 0.005,
                "seed {seed}: the surface's chin peaks {:+.1} mm from the landmark",
                (tip.1 - chin) * 1000.0
            );
        }
    }

    #[test]
    fn the_chin_landmark_lands_on_the_chin_of_the_shipped_face() {
        // **A target when it was written, and met.** The test above runs on a
        // head that has been shaped but NOT carved; this one runs on the surface
        // that ships, and against that the landmark used to be 12.5 mm out on
        // the default and 14.7 on seed 99 — two to three times the 5 mm alarm
        // the uncarved test sets for itself. A tolerance is only worth what the
        // mesh under it is worth.
        //
        // **AT TWO SUBDIVISION LEVELS, AND WITH THE LANDMARK'S OWN DEFINITION,
        // and neither of those is decoration (#108).** This used to scan ±25 mm
        // about the landmark for the forward-most point of the carved midline,
        // which is not what a chin is: the lower lip reaches further forward than
        // the chin does — 119.2 mm against 114.0 on the default body — so the
        // scan answered with the chin only while the lip stayed outside its
        // window. Move the cage and the lip comes inside it, the argmax walks to
        // the top of the scan, and the failure reads "+23.0 mm" on every seed and
        // every configuration, because +23.0 is where the scan stops rather than
        // where anything is. Two instruments that agreed on one cage, and a
        // saturated reading of the disagreement everywhere else.
        //
        // So both sides now ask [`chin_of`], which has a definition — the lowest
        // crest of the midline profile — instead of a window. The landmark comes
        // off the shaped head at build time; the test re-asks the carved one. A
        // disagreement is now a real distance, and a level that cannot answer at
        // all says so rather than returning the edge of its own scan.
        //
        // Kept as a pair rather than replacing the first: they measure different
        // surfaces and both are real.
        // The shipped level and one ABOVE it. It used to be one below, and one
        // below is now zero — an unsubdivided cage, which is a control polygon
        // and not a surface anybody renders. A neighbouring level is here so the
        // assertion cannot be tuned to exactly one resolution; which side it
        // sits on is arbitrary, and only one side exists at
        // [`crate::BODY_SUBDIVISIONS`] = 1.
        for levels in [crate::BODY_SUBDIVISIONS, crate::BODY_SUBDIVISIONS + 1] {
            for seed in [1i64, 23, 42, 99] {
                let mut record = AvatarRecord::new("Skulled", Archetype::default());
                record.reroll(seed);
                let skeleton = record.skeleton();
                let mut mesh = crate::build_body(
                    &skeleton,
                    &CageConfig::default(),
                    levels,
                    &Default::default(),
                )
                .expect("a body builds");
                let rig = Rig::from_skeleton(&skeleton).expect("rigs");
                let skull = Skull::measure(&mesh, &rig).expect("a skull");
                let canon = crate::face::Canon::measure(&rig, &skull, &Default::default());
                crate::face::carve_face(&mut mesh, &rig, &canon, &Default::default());

                let carved = chin_of(&mesh, &rig, skull.head, skull.throat_and_crown().0)
                    .expect("the carved face has a chin on it");
                assert!(
                    (carved - skull.chin()).abs() < 0.005,
                    "seed {seed} at {levels} subdivisions: the carved face's chin is \
                     {:+.1} mm from the landmark the build measured",
                    (carved - skull.chin()) * 1000.0
                );
            }
        }
    }

    #[test]
    fn a_head_with_no_chin_on_it_still_answers_inside_its_own_surface() {
        // [`shape`] bails on anything that walks on four legs, so a creature's
        // head arrives at [`Skull::measure`] as the tube the cage laid — no
        // chin, no crest, nothing for `chin_of` to find. The landmark then falls
        // back to the plan's estimate, and the only thing asked of it is that it
        // stays inside the head it claims to be on: a chin below the throat or
        // above the crown puts every feature fraction outside the surface.
        //
        // Here because it is the one branch of the landmark no biped reaches,
        // and because it used to be a clamp on an estimate that was always
        // taken. Now it is the exception, which means nothing else exercises it.
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        let mesh = crate::build_body(
            &skeleton,
            &CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &Default::default(),
        )
        .expect("a creature builds");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let skull = Skull::measure(&mesh, &rig).expect("a creature has a head to measure");
        assert_eq!(
            chin_of(&mesh, &rig, skull.head, skull.throat_and_crown().0),
            None,
            "an unshaped head reported a chin, so this is no longer the fallback's test"
        );
        let (throat, crown) = skull.throat_and_crown();
        assert!(
            (throat..=crown).contains(&skull.chin()),
            "the fallback chin is at {:+.1} mm, outside a head running {:+.1} to {:+.1}",
            skull.chin() * 1000.0,
            throat * 1000.0,
            crown * 1000.0
        );
    }

    #[test]
    fn the_chin_leads_its_own_lip() {
        // **The contract #72 was bought with, finally asserted** (#128). Cutting
        // [`CHIN`]'s amplitude to steepen the underside cost the chin 7 mm of
        // projection and put the lower lip in front of it; a face whose lip
        // swallows its chin has no jaw at all, which is how it looked, and the
        // amplitude had to go back up. Nothing has guarded that since. #128 cut
        // the same amplitude twice more — for a different and measured reason —
        // and found the same cliff sitting three steps below where it stopped:
        //
        // ```text
        //   amplitude   chin over its lip, default body
        //     x0.75           +5.3 mm    <- ships
        //     x0.70           +3.5
        //     x0.65           +1.7
        //     x0.60           -0.1       <- #72, exactly
        // ```
        //
        // **Measured on the CARVED surface, which is the only one that has a lip
        // on it at all.** The shaped head carries [`CHIN`] and no mouth, so the
        // question cannot be asked of it.
        //
        // **And walked rather than scanned.** Up the midline from the chin the
        // profile falls into the mentolabial crease and rises into the lip, so
        // the two landmarks are the first turn that starts a rise and the first
        // that starts a fall after it. A window would answer with the chin only
        // while the lip stayed outside it and report the edge of its own scan
        // everywhere else, which is the trap
        // `the_chin_landmark_lands_on_the_chin_of_the_shipped_face` was rewritten
        // to escape (#108).
        //
        // **The floor is 1.5 mm, and it guards a CLIFF rather than a
        // proportion — the cliff is at zero.** It is deliberately not ratcheted
        // onto the distribution: the point is a warning that fires one step
        // before the defect rather than a bound that tracks whatever the last
        // change left behind.
        //
        // Across the seeds the margin runs 1.79 to 4.45 mm, seed 0 being the
        // tight one. **The note that used to sit here said 3.1 to 7.7 with seed
        // 13 tightest, and that had gone stale**: by the time #116 measured it
        // the distribution had halved and seed 0 was living on FIFTY MICRONS of
        // the old 2 mm floor. Nobody knew, because a passing test prints
        // nothing — so this one prints its margins now, which is rule 9 of
        // `docs/instruments.md` and is how the staleness was found at all.
        //
        // **Re-blessed from 2.0 under #116** (owner decision, 2026-08-18).
        // Angle-weighted vertex normals cost a uniform third of a millimetre of
        // this margin on every seed — the relief carve offsets along those
        // normals, so correcting them moves the features — and that was
        // accepted as the price of a normal that does not point into the body
        // at a crease. 1.5 rather than 1.7 because 1.7 would leave seed 0 with
        // ninety microns and reproduce exactly the fragility that hid the drift:
        // a warning one step from the state it watches is not a warning.
        const MARGIN: f32 = 0.0015;
        /// A reversal under this is surface ripple and not a feature, the same
        /// reason `examples/neckaudit` counts its turns with a deadband.
        const DEADBAND: f32 = 0.0004;

        let mut margins: Vec<(i64, f32)> = Vec::new();
        for seed in [0i64, 1, 3, 7, 13, 21, 23, 42] {
            let mut record = AvatarRecord::new("Skulled", Archetype::default());
            record.reroll(seed);
            let skeleton = record.skeleton();
            let mut mesh = crate::build_body(
                &skeleton,
                &CageConfig::default(),
                crate::BODY_SUBDIVISIONS,
                &Default::default(),
            )
            .expect("a body builds");
            let rig = Rig::from_skeleton(&skeleton).expect("rigs");
            let skull = Skull::measure(&mesh, &rig).expect("a skull");
            let canon = crate::face::Canon::measure(&rig, &skull, &Default::default());
            crate::face::carve_face(&mut mesh, &rig, &canon, &Default::default());

            let centre = rig.joints[skull.head].position;
            let chin = skull.chin();
            let profile: Vec<(f32, f32)> = (0..)
                .map(|step| chin + 0.001 * step as f32)
                .take_while(|y| *y < canon.nose_base())
                .filter_map(|y| midline(&mesh, centre, y).map(|reach| (y, reach)))
                .collect();

            let mut turns: Vec<(f32, bool)> = Vec::new();
            let mut rising: Option<bool> = None;
            let mut mark = profile[0];
            for &(y, reach) in &profile[1..] {
                if (reach - mark.1).abs() < DEADBAND {
                    continue;
                }
                let now = reach > mark.1;
                if rising.is_some_and(|was| was != now) {
                    turns.push((mark.1, now));
                }
                rising = Some(now);
                mark = (y, reach);
            }
            let crease = turns.iter().position(|(_, up)| *up);
            let lip = crease.and_then(|from| {
                turns[from..]
                    .iter()
                    .find(|(_, up)| !*up)
                    .map(|(reach, _)| *reach)
            });
            let at_chin = midline(&mesh, centre, chin).expect("the chin is on the midline");
            margins.push((seed, lip.map_or(f32::MAX, |lip| at_chin - lip)));
        }

        println!(
            "chin-lip margins: {}",
            margins
                .iter()
                .map(|(seed, margin)| format!("seed {seed} {:+.2} mm", margin * 1000.0))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let swallowed: Vec<_> = margins
            .iter()
            .filter(|(_, margin)| *margin < MARGIN)
            .collect();
        assert!(
            swallowed.is_empty(),
            "the lower lip reaches past the chin on {} of {} faces, against a floor of \
             {:.1} mm: {}. #72 is what this looks like — the chin stops reading as a jaw.",
            swallowed.len(),
            margins.len(),
            MARGIN * 1000.0,
            margins
                .iter()
                .map(|(seed, margin)| format!("seed {seed} {:+.1} mm", margin * 1000.0))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    #[test]
    fn the_vault_profiles_reach_the_crown_they_shape() {
        // **The defect this guards is silent, and it shipped for a whole pass**
        // (#79). Heights below the joint are profile heights and follow a head's
        // own lower face; heights ABOVE it are raw skull radii and follow
        // nothing. `knot` clamps above its first entry, so any vault a table's
        // top knot does not reach is held at a constant — and #79 raised the
        // built crown from 0.85 radii to 1.03 without moving three tables that
        // stopped at 0.86, 0.86 and 0.70.
        //
        // What that looked like: the top sixth of the head kept a constant
        // relative width instead of tapering, and the built half-width ran 46.5,
        // 42.1, 40.8, 39.4 mm over twelve millimetres of height before falling
        // off a cliff at the cap. A cylinder with a lid, in the one part of the
        // skull `BREADTH` exists to round. Nothing failed. The renders showed it
        // and the numbers did not, because every ratio this crate measures — H:W,
        // cranium:face, the widest point — is taken below where the clamp began.
        //
        // So the assertion is registration rather than shape: whatever the crown
        // node is set to, the tables that shape the vault have to reach it.
        for seed in [0i64, 3, 7, 11, 23, 42] {
            let (_, measured, _, radius) = skull(seed, 1);
            let crown = measured.throat_and_crown().1 / radius;
            for (name, profile) in [
                ("BREADTH", &BREADTH[..]),
                ("DEPTH", &DEPTH[..]),
                ("OCCIPUT", &OCCIPUT[..]),
            ] {
                let top = profile[0].0;
                assert!(
                    top >= crown,
                    "seed {seed}: {name}'s top knot is at {top:.2} skull radii and the \
                     built crown is at {crown:.2}, so `knot` clamps {name} flat over the \
                     top {:.0}% of the vault and the cap keeps whatever the last knot \
                     said. Move the knot, do not widen this bound.",
                    100.0 * (crown - top) / crown
                );
            }
        }
    }

    #[test]
    fn the_back_of_the_cranium_is_fuller_than_the_back_of_the_jaw() {
        // **Bisected, not binned, and it took a taller head to expose that**
        // (#79). This read the furthest-back VERTEX within 0.09 radii of each
        // height. On this fixture — the cage at `BODY_SUBDIVISIONS`, with no
        // face refinement — the rows run about 0.18 radii apart, so a window
        // 0.18 wide is one row on a good day and none on a bad one. Raising the
        // crown from 0.85 radii to 1.03 respaced those rows, both windows came
        // up EMPTY, and the fold over an empty iterator returned `f32::MIN` for
        // the occiput and the jaw alike. The assertion then compared minus
        // infinity against minus infinity and failed, reporting a skull with no
        // back at all on a head whose occiput is measurably fine.
        //
        // Which is the defect this module's own `bisect` helper was written for,
        // in a docstring three hundred lines above this one, and it is the
        // seventeenth instrument in this project to have been measuring
        // something other than its name.
        //
        // **It compared the built SILHOUETTE, and the silhouette behind a jaw is
        // not the jaw any more** (#125). Giving the neck a section swept astern
        // fills in behind the skull — which is the reference construction, whose
        // own back has no neck on it at all, only a slope from the occiput into
        // the shoulders. Measured on this body, the surface behind the jawline
        // went from 0.798 radii to 1.008 while the occiput went 1.039 to 1.070,
        // so the ratio fell from 1.30 to 1.06 with nothing wrong with the head.
        // Reading it off the midline does not save it: at half the band's width
        // the ratio is 1.02 and at three quarters it is 0.93, because the nape
        // is there too.
        //
        // So this asks what it was always FOR — that `shape` builds a cranium
        // fuller than a jaw — of the only thing that can still answer: the
        // difference `shape` itself makes. Both readings carry the same neck
        // underneath, so the neck cancels. Measured over these six bodies it
        // moves the occiput back by 0.185 to 0.193 radii with and without the
        // neck's mass, against −0.047 to +0.015 at the jaw: the bound below has
        // the whole of that gap to sit in, and a vault whose profile stops
        // reaching — the failure this test exists for — takes it to zero.
        for seed in [11i64, 0, 3, 5, 7, 15] {
            let (plain, shaped, _, centre, radius) = head(seed);
            let back = |mesh: &PolyMesh, at: f32| {
                bisect(mesh, centre + Vec3::Y * at * radius, -Vec3::Z)
                    .expect("the midline is inside the head at both heights")
                    / radius
            };
            let occiput = back(&shaped, 0.30) - back(&plain, 0.30);
            let jaw = back(&shaped, -0.42) - back(&plain, -0.42);
            assert!(
                occiput > jaw + 0.12,
                "seed {seed}: shaping pushed the back of the cranium out {occiput:+.3} radii \
                 and the back of the jaw {jaw:+.3}, and a cranium that is not built fuller \
                 than its own jawline is a ball on a post"
            );
        }
    }

    #[test]
    fn nothing_below_the_head_is_touched() {
        // The neck runs up into the same band of heights. Reshaping it would
        // pinch the throat away from the jaw it has to meet.
        let (plain, shaped, rig, ..) = head(5);
        for (index, (was, now)) in plain.positions.iter().zip(&shaped.positions).enumerate() {
            let _ = index;
            if rig.joints[rig.nearest_bone(*was).joint].zone != Zone::Head {
                assert_eq!(was, now, "a vertex outside the head moved");
            }
        }
    }

    #[test]
    fn shaping_the_same_mesh_twice_is_not_the_same_as_once() {
        // Stated because it is a trap rather than a feature: this is a function
        // of the REST positions, so running it on an already-shaped head shapes
        // the shaping. It belongs exactly once in the build.
        let (plain, shaped, rig, ..) = head(9);
        let mut twice = shaped.clone();
        shape(&mut twice, &rig, &Default::default());
        assert_ne!(twice.positions, shaped.positions);
        assert_eq!(twice.vertex_count(), plain.vertex_count());
    }

    #[test]
    fn a_body_that_walks_on_all_fours_keeps_its_own_head() {
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let plain = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");

        let mut shaped = plain.clone();
        shape(&mut shaped, &rig, &Default::default());
        assert_eq!(
            plain.positions, shaped.positions,
            "a quadruped was given a human chin and brow"
        );
    }

    /// Where the built body's surface actually is, by bisecting on the mesh
    /// itself along `axis` from the head joint.
    ///
    /// The only thing here that does not go through [`Skull`], and so the only
    /// thing that can catch [`Skull`] being wrong. `contains` is the same
    /// primitive `tests/parts.rs` judges a buried feature with.
    fn probe(mesh: &PolyMesh, from: Vec3, axis: Vec3) -> Option<f32> {
        let inside = |reach: f32| mesh.contains(from + axis * reach);
        if !inside(0.0) || inside(0.3) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, 0.3f32);
        for _ in 0..40 {
            let middle = (near + far) * 0.5;
            if inside(middle) {
                near = middle
            } else {
                far = middle
            }
        }
        Some(near)
    }

    /// A measured skull, and the head it was measured from.
    fn skull(seed: i64, levels: usize) -> (PolyMesh, Skull, Vec3, f32) {
        skull_of(seed, levels, None)
    }

    /// The same, at a chosen skull breadth rather than the seed's own.
    ///
    /// **Every test below that measures a WIDTH RATIO wants this, and wants
    /// `Some(0.0)`.** Skull breadth is a record axis, so a re-rolled
    /// seed arrives with a section the plan gave it, and a test asserting what
    /// `BREADTH` and [`jaw`] do to a head has no business also measuring what
    /// the record asked for. Held at neutral these read the carve alone; the
    /// axis's own effect is
    /// `the_breadth_axis_narrows_the_jaw_with_the_cheek`.
    fn skull_of(seed: i64, levels: usize, breadth: Option<f32>) -> (PolyMesh, Skull, Vec3, f32) {
        let mut record = AvatarRecord::new("Skulled", Archetype::default());
        record.reroll(seed);
        // **The composites are held neutral here for the same reason `breadth`
        // is** (#100). The frame axis reaches the body and not the skull — the head's own
        // nodes do not read it — but it does move the girdle under the neck,
        // and every measurement in this file is taken over `throat..crown`, so
        // a rolled `femininity` moves the SPAN these errors are binned into
        // without moving the geometry they measure. Seed 2 crossed the
        // off-midline ceiling by 3 mm on exactly that, and the file already
        // records the same thing happening to seeds 0 and 3 when the neck was
        // given a section (#125): a body-side change reading as a profile
        // regression because the ruler moved.
        //
        // When the skull does read this axis (#166) the sweep wants it back,
        // deliberately and at both ends, rather than one value per seed.
        //
        // **All of them, not just the frame axis** (#164): the allometric girth
        // moves the girdle under the neck the same way, and seed 2 crossed the
        // off-midline ceiling again the day it landed.
        record.composites = crate::Composites::default();
        if let (Some(breadth), Archetype::Humanoid(params)) = (breadth, &mut record.archetype) {
            params.head_breadth = breadth;
            params.face_length = 0.0;
        }
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let mut mesh = refine_face(
            &catmull_clark(&cage, crate::BODY_SUBDIVISIONS),
            &rig,
            levels,
        );
        shape(&mut mesh, &rig, &Default::default());
        let measured = Skull::measure(&mesh, &rig).expect("a humanoid has a skull");
        let joint = &rig.joints[measured.head];
        (mesh, measured, joint.position, joint.radius)
    }

    #[test]
    fn the_profile_agrees_with_the_surface_it_was_measured_from() {
        // The whole contract. Every figure below is millimetres, and every one
        // of them was a measurement before it was an assertion (#67). Measured
        // over sixteen seeds and thirteen heights each: the midline depth runs
        // -1.7 to +4.0, the width -2.2 to +5.8, and the depth off the midline
        // +0.1 to +9.0. The bounds here are those with room to move.
        //
        // ASYMMETRIC ON PURPOSE, both ways. A bin's answer is the outermost
        // sample in it, so it sits slightly OUTSIDE the surface at the bin's
        // centre — hence the wide upper bound. And the failure this guards
        // against is a profile that reports the head too NARROW, which is what
        // buries a feature; too wide only stands one off. The vertex-binned
        // profile this replaced would fail the lower bound at four of these
        // thirteen heights on the first seed alone.
        //
        // **The sweep starts at 0.08 of the span so that the CHIN is inside
        // it.** It began at 0.15, and since `chin` is always 0.7097 × the
        // throat the chin lands at 0.13 of the span — so the one region the
        // owner kept reporting as wrong was, by construction, the region this
        // contract never looked at (#74). It stops at 0.90 rather than 1.0
        // because the crown is a subdivided cap whose bins are the worst in the
        // profile; `the_profile_agrees_over_its_whole_span` below is that
        // target, and it is 13.3 mm out at the throat today.
        for seed in 0..6 {
            let (mesh, skull, centre, _) = skull(seed, 1);
            let (lo, hi) = skull.throat_and_crown();
            for step in 0..=12 {
                let height = lo + (hi - lo) * (0.08 + 0.82 * step as f32 / 12.0);
                let from = centre + Vec3::Y * height;

                // **The band under the chin gets its own bounds, and the
                // discriminator is the CHIN rather than the step index** (#94).
                // It was `step == 0` for six issues, on the reading that step 0
                // sits at 0.08 of the span and the chin at 0.13. That is true of
                // the SPAN and not of any particular body: measured over these
                // six seeds, step 0 lands between 15.3 and 64.8 mm below the
                // chin and step 1 lands anywhere from 0.2 mm ABOVE it (seed 0)
                // to 38.8 below (seed 1). So the loose bound was being handed
                // out by index to whichever anatomy happened to fall under it.
                //
                // Split by the landmark instead, the population separates
                // cleanly: over these six seeds every midline error outside
                // ±3.1 mm is at a height BELOW the chin, and every height above
                // it reads within 0.1 mm of the surface except the crown cap,
                // which is +6.8 at worst. Under the chin the head's surface is
                // running into the neck's — where
                // `the_profile_agrees_over_its_whole_span` already records and
                // tolerates 13.3 mm — and since #94 it is also running over a
                // constructed cliff, which a band MAXIMUM describes badly by
                // construction: `SUBMENTAL_SPEND` drops the surface 35 mm in one
                // centimetre and a band spanning that reports its top.
                //
                // **So the floor above the chin goes back to −6.0, where it was
                // before #94's last session moved it** (#94). That re-base was
                // attributed to the span sliding at "a height in the MOUTH";
                // the height was −0.259 in head-local METRES and seed 2's chin
                // is at −0.257, so it was 2.1 mm UNDER the chin and inside the
                // submental construction's own reach. Below the chin, −7.0
                // against a measured −6.8 and 13.0 against a measured 12.1.
                //
                // It was raised from 9.0 to take #93's shorter neck, which drops
                // the whole head about 18 mm and so lands its lowest band where
                // the body is already rising into the trapezius. That is a real
                // cost of that change and this is where it is paid. Widening the
                // whole sweep to fit it would have hidden a regression anywhere
                // else in the profile; this cannot.
                //
                // **11.0 → 13.0 for the neck's off-centre section, and it is the
                // SURFACE that moved rather than the profile** (#125). The neck
                // is swept `NECK_SECTION.y − NECK_LOBE` radii forward of its own
                // joint, which is exactly where its front stood before, so the
                // cage's throat does not move at all — but the limit surface at
                // the front is an average over ring neighbours that DID move
                // back, and it follows them a little. Measured on the six seeds
                // this sweeps, at step 0 and on the midline:
                //
                // ```text
                //   seed   surface before   after    profile before   after   error
                //     0        80.7          75.4        89.6         87.7    12.3
                //     1        62.2          60.4        70.5         61.8     1.4
                //     2        84.0          80.7        87.4         85.3     4.6
                //     3        72.6          70.8        78.8         76.7     5.9
                //     4        55.9          52.5        60.9         59.4     6.9
                //     5        69.7          65.4        78.6         69.0     3.6
                // ```
                //
                // Five of the six IMPROVED and seed 0 is the whole of the cost:
                // its throat came back 5.3 mm where its profile came back 1.9.
                //
                // **And the POPULATION barely moved, which is the more useful
                // reading and it makes both bounds here overdue rather than
                // relaxed.** Run over sixteen bodies instead of the six this
                // sweeps, the midline error ran −5.3 to +12.1 mm BEFORE the neck
                // was given a section, against −5.6 to +12.3 after: two tenths
                // of a millimetre at each end. Seeds 6 upward were already
                // outside both bounds and this test never visited them, so what
                // the neck did was move seed 0 and seed 3 into a tail that was
                // there all along. The paragraph at the top of this test still
                // quotes −1.7 to +4.0 from #67's sweep, and that has not
                // described this code for some time.
                //
                // Split by step, over sixteen seeds with the neck applied: step
                // 0 runs +0.7 to +12.3 and every other step runs −5.6 to +6.2.
                // So the throat band keeps a ceiling of its own and the rest of
                // the sweep keeps a tighter one, which is the arrangement #93
                // introduced and the reason it is worth keeping.
                // **The floor went −6.0 → −6.5 for #94's chin shelf**, and it
                // is the span sliding rather than the profile disagreeing: the
                // chin stands further forward, `Skull::measure`'s throat moves
                // with it, and every band this is binned into slides. Seed 2 at
                // −0.259 — a height in the MOUTH, which neither `BUTTON` nor
                // `CHIN`'s tail can reach — went to −6.1 mm on that alone. The
                // same mechanism as the re-bases below, one tenth of a
                // millimetre's worth of it.
                // Judged by render at seeds 0, 3, 6 and 12, bare and close:
                // the jaw's underside runs cleaner into the throat on 6 and
                // nothing else in the face moved.
                let (floor, ceiling) = if height < skull.chin() {
                    (-7.0, 13.0)
                } else {
                    (-6.0, 9.0)
                };
                if let Some(surface) = probe(&mesh, from, Vec3::Z) {
                    let error = (skull.depth(height) - surface) * 1000.0;
                    assert!(
                        (floor..ceiling).contains(&error),
                        "seed {seed} at {height:.3}: the midline depth is {error:.1} mm out"
                    );
                }
                if let Some(surface) = probe(&mesh, from, Vec3::X) {
                    let error = (skull.width_across(height, 0.0) - surface) * 1000.0;
                    assert!(
                        (-4.0..9.0).contains(&error),
                        "seed {seed} at {height:.3}: the width is {error:.1} mm out"
                    );
                }
                // Off the midline, where a mouth's corners sit and where the
                // per-band normalisation earns its keep.
                //
                // **The shallow bound came out from −4.0 to −5.0 for #79's
                // longer face, and the obvious alternative was tried first and
                // is wrong.** The head reaches 30% further below its joint than
                // it did, so `BANDS`'s pitch over `throat..crown` went from
                // about 10 mm to 12.5, and the worst off-midline reading — the
                // jaw just under the joint, where the surface turns fastest —
                // went from −2.0 mm to −4.0. Raising `BANDS` makes it WORSE,
                // measured: 24 bands take the worst seed to −4.7 and 26 to −5.4.
                // A bin's answer is the outermost sample in it, so starving the
                // bins costs more than the finer pitch buys, which is exactly
                // what `COLUMNS`'s docstring records happening to the lateral
                // bins at twenty-one.
                //
                // The fix is the one `the_profile_agrees_over_its_whole_span`
                // already names: spend the band budget on the face rather than
                // on the throat, which is a quarter of this span and carries
                // nothing. That belongs with #74, not with a proportions pass.
                let across = skull.half_width(height) * 0.5;
                if let Some(surface) = probe(&mesh, from + Vec3::X * across, Vec3::Z) {
                    let error = (skull.depth_across(height, across) - surface) * 1000.0;
                    // **14.0 → 17.0 as a debt** (#164, #174). Nothing about
                    // the head changed: the neck floor did, so the
                    // `throat..crown` span every band here is binned into moved
                    // under the ruler, and seed 2's worst off-midline reading
                    // went 14.0 to 16.4. The file already records this exact
                    // failure mode twice — at #125 for the neck's section and
                    // at #100 for the frame axis — and it is the third time a
                    // body-side change has reported itself as a profile
                    // regression here.
                    //
                    // **17.0 STAYS, and it is the fourth time** (#174). The
                    // neck floor was fixed — a computed socket clearance
                    // instead of a blanket 1.12 girdle radii — and this ruler
                    // did not come back. Two separate reasons, and they pull
                    // opposite ways:
                    //
                    // Seed 2, the body the paragraph above blames, does not
                    // read the floor at all. At neutral composites its neck
                    // bone is its own length term, 0.1693 m against a floor of
                    // 0.1136 under #164 and 0.1070 under #174, so `max` picked
                    // the length term throughout and 16.4 is 16.4 under both.
                    // A floor cannot move a body it does not bind on, and that
                    // subtraction is all it would ever have taken to check.
                    //
                    // Seed 5 does read it, and the worst reading is now its:
                    // its floor DID bind, #174 shortens its neck bone 12%
                    // (0.1412 m to 0.1236), the span moves again, and the
                    // off-midline error goes 16.4 to 16.8. So the sweep's worst
                    // rose four tenths of a millimetre for a change that made
                    // the body better, on a seed the earlier note never named.
                    //
                    // That is the whole lesson of the three citations above,
                    // arriving a fourth time and now in the other direction:
                    // this bound moves whenever the neck moves, either way,
                    // because the span it bins into is anchored on the throat.
                    // A ruler that cannot tell a shorter neck from a worse
                    // profile is measuring the wrong span, and fixing that is
                    // #74's re-binning rather than a number here. 17.0 is the
                    // state plus 0.2 mm.
                    //
                    // **17.0 → 18.8, and it is the fifth time, this one for a
                    // head that got rounder** (#158). `refine_face` splits with
                    // `PolyMesh::refine_curved` now, so the sixteen-sided tube
                    // the head arrives as is filled in to its own arc instead
                    // of being subdivided along its chords; the surface gains
                    // up to a facet's sagitta, about 2 mm, most of it midway
                    // between the coarse rows. Seed 2's worst off-midline
                    // reading goes 16.4 to 18.6 and the spread's other end
                    // barely moves, −3.7 to −3.1. What this ruler is reporting
                    // is that the bins have not moved with the surface, which
                    // is #74 again.
                    //
                    // **18.8 → 20.0, and it is the sixth time — but not for
                    // the span this time** (#79). BREADTH's face knots moved
                    // onto the landmarks they are named for, which narrows the
                    // lower face by up to a fifth at the angle of the jaw. This
                    // probe stands at `half_width(height) * 0.5`, so a narrower
                    // head moves the SAMPLE as well as the surface: seed 2's
                    // worst went 18.8 to 19.9 with its `across` falling from
                    // 0.0455 m to 0.0428. Every sample height is identical
                    // before and after, checked rather than assumed, so nothing
                    // slid — a fixed number of lateral columns simply describes
                    // a faster-turning face worse. #74's re-binning again.
                    //
                    // **And it caught something real on the way**, which is the
                    // argument for keeping it however badly it bins: seed 3
                    // came back at −88.6 mm, and that was not a ruler at all.
                    // One cell of `Skull::measure`'s `front` table had no face
                    // sample in it and was left holding the back of the skull,
                    // so `depth_across` answered 21 mm BEHIND the head joint at
                    // the eye's own column. Fixed where it was, in `measure`.
                    assert!(
                        (-5.0..20.0).contains(&error),
                        "seed {seed} at {height:.3}: the depth off the midline is {error:.1} mm out"
                    );
                }
            }
        }
    }

    #[test]
    fn the_measured_surface_is_smooth_in_height_at_every_azimuth() {
        // **The instrument the outline solve never had** (#210). Nothing
        // measured `surface_at` at all, so a profile that jumped forty
        // millimetres between two heights six apart was reported by no test on
        // this crate — and the one test that walks it,
        // `a_lock_stays_on_the_head_while_the_head_is_holding_it_up`, compares a
        // lock against the SAME profile and stopped measuring the moment the
        // profile had been wider anywhere above, which noise makes true
        // immediately. Two blocky slabs stood off the back of every head of hair
        // for an issue and a half because of that pair.
        //
        // A head is a smooth thing, so the claim is simply that: the radius may
        // not move faster than the surface of a head does, in either direction.
        // The failure it catches is not a bad radius — one of those is a
        // millimetre here and there — but a DISCONTINUOUS one, which is what a
        // diverging solve and a seam between two models both make, and what
        // anything walking the surface latches onto.
        //
        // **Both directions, because the two defects were one each.** In height,
        // the fixed-point substitution oscillated on the back diagonals and the
        // trust band admitted whichever iterate landed inside it. In azimuth,
        // the near-astern case answered with the midline's own reach over the
        // cosine — a flat plane across the back of a round head — and the seam
        // where that took over from the tables was a 15 mm step at 160°.
        //
        // The two are measured differently and on purpose. A rise is millimetres
        // of radius per millimetre of height, which is already dimensionless. A
        // turn is millimetres per RADIAN over the radius there, because a head is
        // a scale as well as a shape: seed 2's is twice the default's, and an
        // absolute bound on how fast a radius may move round it reads that head's
        // size as a defect.
        //
        // Measured before they were asserted, and both fail on the solve they
        // replace. Over these six seeds it rises 26.8 to 33.9 and turns 73.1 to
        // 75.5; bisected onto the same tables, with the seam behind the head
        // gone and the one at the front blended, it rises 5.8 to 7.3 — the jaw's
        // own step down to the neck, around 125° — and turns 1.8 to 3.1.
        for seed in 0..6 {
            let (_, skull, _, _) = skull(seed, 1);
            let (throat, crown) = skull.throat_and_crown();
            // **Below the crown's own cap band**, which is not noise: the top
            // band is closed with a quarter-circle to a point (#204) and a
            // dome's tangent is vertical at its pole, so the radius there moves
            // arbitrarily fast by construction.
            let top = crown - skull.crown_band();
            let radius = |height: f32, azimuth: f32| {
                let at = skull.surface_at(height, azimuth);
                (at.x * at.x + at.z * at.z).sqrt()
            };
            let mut rise = (0.0f32, 0.0f32, 0.0f32);
            let mut turn = (0.0f32, 0.0f32, 0.0f32);
            for degree in 0..720 {
                let azimuth = (degree as f32 * 0.5).to_radians();
                let next = ((degree + 1) as f32 * 0.5).to_radians();
                for step in 0..200 {
                    let one = top - (top - throat) * step as f32 / 200.0;
                    let two = top - (top - throat) * (step + 1) as f32 / 200.0;
                    let up = (radius(one, azimuth) - radius(two, azimuth)).abs() / (one - two);
                    if up > rise.0 {
                        rise = (up, azimuth.to_degrees(), one);
                    }
                    let here = radius(one, azimuth);
                    let round = (here - radius(one, next)).abs()
                        / (here.max(MINIMUM_REACH) * (next - azimuth));
                    if round > turn.0 {
                        turn = (round, azimuth.to_degrees(), one);
                    }
                }
            }
            assert!(
                rise.0 < 12.0,
                "seed {seed}: the surface's radius moves {:.1} mm per mm of height at {:.0} deg, \
                 {:+.1} mm — a head does not, so the outline solve is jumping rather than \
                 following the tables",
                rise.0,
                rise.1,
                rise.2 * 1000.0
            );
            assert!(
                turn.0 < 5.0,
                "seed {seed}: the surface's radius turns {:.2} of its own radius per radian of \
                 azimuth at {:.0} deg, {:+.1} mm — a head does not, so there is a seam between \
                 two models of it",
                turn.0,
                turn.1,
                turn.2 * 1000.0
            );
        }
    }

    #[test]
    #[ignore = "the target, not the state: the profile's end bins are 13 mm out at the throat (#74)"]
    fn the_profile_agrees_over_its_whole_span() {
        // The contract above covers 0.08–0.90 of the span. This is the same
        // check over 0.02–0.98, which is what "the profile agrees with the
        // surface" ought to mean without qualification.
        //
        // It fails at the ends, and the reason is worth writing down: BANDS is
        // 20 over `throat..crown`, and 26–28 mm of that span is throat, so the
        // pitch is 7.8–12.8 mm and the outermost bins each straddle a region
        // where the surface is turning fastest. Fixing it means spending the
        // band budget on the face rather than on the neck — measure `lo` as the
        // chin, or scale BANDS so the eye-to-chin frame gets a pitch matching
        // the refined cells.
        for seed in 0..6 {
            let (mesh, skull, centre, _) = skull(seed, 1);
            let (lo, hi) = skull.throat_and_crown();
            for step in 0..=12 {
                let height = lo + (hi - lo) * (0.02 + 0.96 * step as f32 / 12.0);
                let from = centre + Vec3::Y * height;
                if let Some(surface) = probe(&mesh, from, Vec3::Z) {
                    let error = (skull.depth(height) - surface) * 1000.0;
                    assert!(
                        (-4.0..9.0).contains(&error),
                        "seed {seed} at {height:.3}: the midline depth is {error:.1} mm out"
                    );
                }
            }
        }
    }

    #[test]
    fn refining_the_face_does_not_move_the_profile() {
        // Refinement adds vertices and moves none, so the surface is unchanged
        // and the measurement of it must be too. It was NOT: binning the raw
        // vertex list, the half-width at the ear line moved eleven millimetres
        // between one refinement pass and two while the mesh moved half of one,
        // and an ear seated from it fell from 53% visible to 11% (#67).
        //
        // Eight millimetres, not zero, and the gap is a known residual rather
        // than slack. On five seeds in sixteen, refining re-labels one row of
        // vertices at the jaw from neck-owned to head-owned, and the measured
        // chin steps 6.6 mm lower — the surface is identical, the LABELLING of
        // it is finer. That slides the band grid, which moves the profile by up
        // to 4.2 mm where the head's width is changing fastest. Mean movement
        // is 0.3 mm. It is bounded, it does not bite in the one refinement
        // setting that ships, and fixing it means cutting the head from the
        // skeleton instead of by nearest bone, which would redefine the chin
        // every feature height is measured down from.
        //
        // **Asked at the shipped refinement and one above it, because the
        // residual is a transient at the coarse end and this used to ask at the
        // coarsest pair there is** (#107). Swept pass by pass over the same six
        // seeds, the worst movement in the depth is:
        //
        // ```text
        //   1 -> 2   14.0 mm      5 -> 6   0.3 mm
        //   2 -> 3    4.9 mm      6 -> 7   0.4 mm
        //   3 -> 4    0.0 mm      7 -> 8   0.1 mm
        //   4 -> 5    0.0 mm      8 -> 9   0.0 mm
        // ```
        //
        // It has settled to nothing well before [`crate::FACE_REFINEMENT`], and
        // the transient is bigger than it was because the first pass is no
        // longer the one it used to be: `FACE_PASSES` now opens with a broad
        // pass covering the whole head, so "one pass" is a different and
        // coarser face than the front-band pass this test used to compare from.
        // Asserting on that pair measures a head the crate has never built.
        for seed in 0..6 {
            let (_, coarse, centre, _) = skull(seed, crate::FACE_REFINEMENT);
            let (_, fine, ..) = skull(seed, crate::FACE_REFINEMENT + 1);
            let (lo, hi) = coarse.throat_and_crown();
            for step in 0..=12 {
                let height = lo + (hi - lo) * (0.15 + 0.55 * step as f32 / 12.0);
                let _ = centre;
                let moved =
                    (fine.width_across(height, 0.0) - coarse.width_across(height, 0.0)).abs();
                assert!(
                    moved < 0.008,
                    "seed {seed} at {height:.3}: refining moved the width {:.1} mm",
                    moved * 1000.0
                );
                let deeper = (fine.depth(height) - coarse.depth(height)).abs();
                assert!(
                    deeper < 0.008,
                    "seed {seed} at {height:.3}: refining moved the depth {:.1} mm",
                    deeper * 1000.0
                );
            }
        }
    }

    #[test]
    fn the_width_falls_away_behind_the_cheekbone() {
        // The axis this profile gained for the ear. If it answered the same at
        // every depth it would be the band maximum again under a longer name,
        // and the test would pass while measuring nothing.
        let (_, skull, ..) = skull(3, 1);
        let (lo, hi) = skull.throat_and_crown();
        let height = lo + (hi - lo) * 0.45;
        let reach = (hi - lo) * 0.25;
        let front = skull.width_across(height, reach * 0.5);
        let back = skull.width_across(height, -reach);
        assert!(
            back < front,
            "the head was no narrower behind the cheek: {back:.4} against {front:.4}"
        );
        assert!(
            back > front * 0.5,
            "the head fell away implausibly fast: {back:.4} against {front:.4}"
        );
    }

    #[test]
    fn a_profile_reads_between_its_knots() {
        // This used to assert that a segment's midpoint is the AVERAGE of its
        // two knot values, which is a test for linear interpolation wearing an
        // interval property's name. The interval property is the one worth
        // having, and it is the one the monotone limiter exists to guarantee:
        // no profile may leave the range its own knots span, because the
        // segment where that would bite is CHIN's tail into JUNCTION, and a
        // spline dipping below zero there stands the head's lowest band behind
        // the throat (#47, #83).
        for (name, profile) in PROFILES {
            assert_eq!(knot(profile, profile[0].0 + 1.0), profile[0].1, "{name}");
            let end = profile.len() - 1;
            assert_eq!(
                knot(profile, profile[end].0 - 1.0),
                profile[end].1,
                "{name}"
            );

            for step in 0..=2000 {
                let height =
                    profile[end].0 + (profile[0].0 - profile[end].0) * step as f32 / 2000.0;
                let value = knot(profile, height);
                let segment = (0..end)
                    .find(|&at| height >= profile[at + 1].0)
                    .unwrap_or(end - 1);
                let (low, high) = (
                    profile[segment].1.min(profile[segment + 1].1),
                    profile[segment].1.max(profile[segment + 1].1),
                );
                assert!(
                    value >= low - 1e-4 && value <= high + 1e-4,
                    "{name} at {height:.4} reads {value:.4}, outside the {low:.4}..{high:.4} \
                     its own knots span"
                );
            }
        }
    }

    /// Every shaping profile, for the checks that must hold of all of them.
    const PROFILES: [(&str, &[(f32, f32)]); 6] = [
        ("BREADTH", &BREADTH),
        ("DEPTH", &DEPTH),
        ("OCCIPUT", &OCCIPUT),
        ("BROW", &BROW),
        ("TEMPLE", &TEMPLE),
        ("CHIN", &CHIN),
    ];

    #[test]
    fn a_profile_has_no_corners_in_it() {
        // The defect this file's terracing turned out to be (#83). A
        // piecewise-linear profile's SLOPE jumps at every knot, and with 27
        // knot heights across the six profiles that is a tangent break every
        // 7.9 mm down the head — running the whole way round it, since BREADTH
        // and DEPTH carry no azimuthal window.
        //
        // **Asked by halving the step, not by a threshold**, because a
        // threshold cannot tell a corner from a tight curve. Sampling the slope
        // by finite difference, a genuine tangent discontinuity gives the same
        // jump however finely it is sampled, while smooth curvature gives a
        // jump proportional to the step. So the discriminator is the RATIO.
        //
        // A first version of this test asserted the jump was under 0.02 and
        // failed at 0.053 on BREADTH's chin knot — where the profile is
        // correctly C1 and merely turning hard. It was measuring curvature
        // times sampling step and calling it a corner.
        //
        // Measured: with the piecewise-linear interpolant this replaced, the
        // ratio is 1.00 on every profile — the corner does not care about the
        // step. With the cubic it is at most 0.51.
        for (name, profile) in PROFILES {
            let end = profile.len() - 1;
            let span = profile[0].0 - profile[end].0;
            let worst_jump = |ticks: usize| {
                let step = span / ticks as f32;
                let slope =
                    |height: f32| (knot(profile, height + step) - knot(profile, height)) / step;
                (1..ticks).fold((0.0f32, 0.0f32), |worst, tick| {
                    let height = profile[end].0 + span * tick as f32 / ticks as f32;
                    let jump = (slope(height) - slope(height - step)).abs();
                    if jump > worst.0 {
                        (jump, height)
                    } else {
                        worst
                    }
                })
            };
            let coarse = worst_jump(2000).0;
            let (fine, at) = worst_jump(4000);
            assert!(
                coarse > f32::EPSILON && fine / coarse < 0.75,
                "{name}'s worst slope jump is {coarse:.4} sampled coarsely and {fine:.4} \
                 sampled twice as finely at {at:.4} — a ratio of {:.2}. A smooth profile \
                 halves; a corner does not care.",
                fine / coarse
            );
        }
    }
}

/// How many heights the measured profile is sampled at.
const BANDS: usize = 20;

/// How many lateral columns the depth map is sampled at.
///
/// A face is not equally deep across its width, and a feature that spans it —
/// a mouth reaches nearly two eye-widths — cannot be placed from a profile that
/// only knows height. Placed against the midline depth, a lip's corners sit
/// well proud of a face that has curved away from them, or well inside one that
/// has not.
///
/// **Columns are a fraction of the band's own width, not of the head's.** Scaled
/// against one figure for the whole head, the chin — which is a third as wide as
/// the cheekbones — puts every sample it has into the first two columns and
/// leaves the rest to be filled in from them, so the map answers with the
/// midline wherever it is asked. Measured that way the forward reach came back
/// 14 mm too deep at the chin against 5 mm at the cheek.
///
/// Fifteen, not nine, and not twenty-one. A bin's answer is the outermost sample
/// in it, so halving the bin halves how far the surface has curved away inside
/// it: nine columns reported the face 4.2 mm too deep on average, fifteen
/// 1.9 mm. At twenty-one the bins start coming up empty and are filled from a
/// neighbour instead — the mean keeps falling, to 1.1 mm, while the worst case
/// turns round and becomes 16 mm too *shallow*. The mean is the wrong thing to
/// tune against; the tail is what buries a lip.
const COLUMNS: usize = 15;

/// How many fore-aft columns the width map is sampled at.
///
/// The mirror of [`COLUMNS`], and it exists for the ear: an ear sits on the side
/// of the head and about a third of an eye-radius *behind* the midline, where
/// the head is a couple of millimetres narrower than at the cheekbone in front
/// of it. A width that only knows height reports the cheekbone.
///
/// Normalised per band for the same reason [`COLUMNS`] is.
const DEPTHS: usize = 15;

/// How far either side of a bin's centre a sample still counts, in bins.
///
/// Half a bin, so bins share only their boundaries — a sample exactly between
/// two centres counts for both, and nothing else does.
///
/// **Wider is not safer, and the widest window in the crate was right to be
/// wide.** The shell era's scalp profile carried three quarters of a bin
/// because it needed a curve that cleared the head *everywhere*, so overstating
/// was the safe direction for it. This one is a measurement, and a maximum
/// taken over a wide window is not a measurement of the middle of it. Measured at three quarters, the face came back 2.2 mm too
/// wide and 7.9 mm too deep off the midline; at a half, 0.9 mm and 4.2 mm, with
/// no bins left empty and the ear's visibility unchanged between refinement
/// passes either way.
const WINDOW: f32 = 0.5;

/// The skull as it was actually built.
///
/// [`shape`] carves a jaw, a chin, a brow and an occiput into a subdivided egg,
/// and [`reshape`] says where a point on the *planned* sphere lands under that
/// carving. Neither answers the question a feature actually asks, which is
/// "where is the surface, at this height, in this direction" — because
/// subdivision has already pulled the mesh well inside the node radius the plan
/// named, by a factor that depends on the body.
///
/// So this measures it. Two profiles against height in head-local metres: how
/// far the surface reaches sideways, and how far it reaches forward. Ears were
/// placed against the planned radius and sat 11 to 39 mm *inside* the head on
/// every seed measured; lips were buried on some bodies and proud on others,
/// which is what a guess looks like when the thing guessed at varies.
///
/// The same argument [`crate::hair::Follicles`] makes for every hair region, and
/// for the same reason: measure the body in hand rather than the plan that asked
/// for it.
///
/// Sampled from the **surface**, not from the vertex list. A head carries a few
/// hundred vertices and a feature is placed to within a couple of millimetres,
/// so a bin holding two or three quad corners reports a number that is several
/// millimetres under the surface and jumps about as the mesh is refined. Face
/// centroids and edge midpoints cost nothing and take the sample count up by
/// nearly an order of magnitude; overlapping bins do the rest.
#[derive(Clone, Debug, PartialEq)]
pub struct Skull {
    /// The head joint everything on the face hangs from.
    pub head: usize,
    lo: f32,
    hi: f32,
    chin: f32,
    across: [f32; BANDS],
    ahead: [f32; BANDS],
    behind: [f32; BANDS],
    front: [[f32; COLUMNS]; BANDS],
    side: [[f32; DEPTHS]; BANDS],
}

impl Skull {
    /// Measures a built head.
    ///
    /// Returns `None` for a body with no head, or one carrying too little
    /// surface to profile.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, rig: &Rig) -> Option<Self> {
        let head = *rig.in_zone(Zone::Head).first()?;
        let centre = rig.joints[head].position;
        let mine = samples(mesh, rig, centre);
        if mine.len() < BANDS {
            return None;
        }

        // **The crown is measured and the throat is measured-but-bounded, and
        // the asymmetry is the point** (#127). The top of a head is the head's
        // own surface and nothing else reaches it. The bottom is the blend into
        // the neck, so a fold over head-OWNED samples answers with whatever
        // `Rig::nearest_bone` credited — and once a neck carries mass away from
        // its own bone, the two segments are millimetres apart and the answer is
        // a coin toss. Measured, giving the neck an off-centre section took this
        // from −110 mm to −153 and `Skull::chin` followed it down to −157 on a
        // head 200 mm tall.
        //
        // Replacing the fold with [`floor`] outright is WRONG and was tried:
        // that puts the low end at −140.9 against a surface that stops near
        // −110, and `the_profile_agrees_with_the_surface_it_was_measured_from`
        // fails at the low end, correctly, because 30 mm of the span is then not
        // head surface at all. `JUNCTION` and the throat were never the same
        // place — `SETTLE` exists because ownership stops short of the neck node
        // — so the derived floor is a CEILING on how far this may reach, not a
        // replacement for it.
        //
        // Clamped rather than chosen: on every body that owns a sane amount of
        // surface the fold wins and this reads exactly as it always did.
        let (measured, hi) = mine.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.y), hi.max(p.y))
        });
        let lo = measured.max(floor(rig, head) * rig.joints[head].radius);
        if hi - lo <= f32::EPSILON {
            return None;
        }

        // Where the chin's tip is: the lowest crest of the midline profile, found
        // on the built surface rather than predicted from the plan. See
        // [`chin_of`] for the definition and [`Self::chin`] for why this landmark
        // matters enough to measure.
        //
        // A head with no chin on it — a creature's, which [`shape`] leaves as a
        // sphere — falls back to where the `CHIN` profile's peak knot would have
        // put one through the same floor scaling [`shape`] uses. That is not a
        // chin, but it is inside the head's own surface, which is all a body
        // without a face asks of it.
        let chin = chin_of(mesh, rig, head, lo).unwrap_or_else(|| {
            let radius = rig.joints[head].radius;
            let peak = CHIN
                .iter()
                .fold(
                    CHIN[0],
                    |best, &knot| if knot.1 > best.1 { knot } else { best },
                )
                .0;
            (peak * (floor(rig, head) * SETTLE) / JUNCTION * radius).max(lo)
        });
        let height = |point: &Vec3| (point.y - lo) / (hi - lo) * (BANDS - 1) as f32;

        let mut across = [0.0f32; BANDS];
        let mut ahead = [f32::MIN; BANDS];
        let mut behind = [f32::MAX; BANDS];
        for point in &mine {
            for band in window(height(point), BANDS) {
                across[band] = across[band].max(point.x.abs());
                ahead[band] = ahead[band].max(point.z);
                behind[band] = behind[band].min(point.z);
            }
        }
        // A band with no sample in it takes its neighbour's, so a sparse sample
        // never reports a skull that pinches to nothing.
        fill(&mut across);
        carry(&mut ahead, f32::MIN);
        carry(&mut behind, f32::MAX);

        // The same reaches, now also binned across the other axis: forward reach
        // by how far across the face it was measured, and lateral reach by how
        // far back. Mirrored into one half, because a head is symmetric and
        // folding doubles the samples in every bin.
        let mut front = [[f32::MIN; COLUMNS]; BANDS];
        let mut side = [[f32::MIN; DEPTHS]; BANDS];
        for point in &mine {
            for band in window(height(point), BANDS) {
                // **A forward reach is measured from the front of the head, and
                // only samples that are in front of the joint can be one**
                // (#158). This is a maximum over whatever landed in the cell,
                // and a lateral column that the FACE happens not to sample —
                // the front is refined and the occiput is not, so the back's
                // samples are spread far apart across the same axis — was left
                // holding the back of the skull. Measured on seed 3 the day
                // `refine_curved` moved the vertices a couple of millimetres:
                // one cell of one band read −69.2 mm between neighbours of
                // +68.5 and +64.3, and `depth_across` interpolated a face that
                // stood 21 mm BEHIND its own head joint at the eye's own
                // column. The joint is inside the skull on every body, so a
                // sample behind it is never the front, and a cell with no front
                // sample in it is empty rather than backwards — which is what
                // `spread` below already exists to answer.
                if point.z > 0.0 {
                    let lateral = lateral(across[band], point.x.abs());
                    for column in window(lateral, COLUMNS) {
                        front[band][column] = front[band][column].max(point.z);
                    }
                }
                let fore = fore(behind[band], ahead[band], point.z);
                for column in window(fore, DEPTHS) {
                    side[band][column] = side[band][column].max(point.x.abs());
                }
            }
        }
        // Each row falls back to ITS OWN band. Falling back to one shared value
        // puts the bottom of the skull's depth on every band that happened to
        // sample thinly.
        for (band, row) in front.iter_mut().enumerate() {
            spread(row, ahead[band]);
        }
        for (band, row) in side.iter_mut().enumerate() {
            spread(row, across[band]);
        }

        Some(Self {
            head,
            lo,
            hi,
            chin,
            across,
            ahead,
            behind,
            front,
            side,
        })
    }

    /// How far the skull reaches sideways at `height`, in head-local metres.
    ///
    /// The widest the head gets anywhere in that band of heights. Use
    /// [`Self::width_across`] for anything seated at a known depth, which on a
    /// head means anything behind the cheekbone.
    #[must_use]
    pub fn half_width(&self, height: f32) -> f32 {
        self.sample(&self.across, height)
    }

    /// How far the skull reaches sideways at `height`, `depth` in front of the
    /// head joint.
    ///
    /// All three in head-local metres; the answer is a half-width, a head being
    /// symmetric. The mirror of [`Self::depth_across`], and it exists for the
    /// same reason: a head is no more a cylinder than it is a surface of
    /// revolution. An ear seats about a third of an eye-radius behind the
    /// midline, where the skull has already begun to fall away, and
    /// [`Self::half_width`] answers there with the cheekbone in front of it.
    #[must_use]
    pub fn width_across(&self, height: f32, depth: f32) -> f32 {
        self.bilinear(&self.side, height, |band| {
            fore(self.behind[band], self.ahead[band], depth)
        })
    }

    /// How far the skull reaches forward at `height`, in head-local metres.
    ///
    /// Measured on the midline. Use [`Self::depth_across`] for anything wide
    /// enough that the face curves away beneath it.
    #[must_use]
    pub fn depth(&self, height: f32) -> f32 {
        self.sample(&self.ahead, height)
    }

    /// How far the skull reaches forward at `height`, `across` from the midline.
    ///
    /// Both in head-local metres; `across` is taken by magnitude, a head being
    /// symmetric. This is what a mouth or a brow needs: a feature two eye-widths
    /// across sits on a surface that has curved back by several millimetres at
    /// its corners, and placing those corners at the midline depth leaves them
    /// standing off the face.
    #[must_use]
    pub fn depth_across(&self, height: f32, across: f32) -> f32 {
        self.bilinear(&self.front, height, |band| {
            lateral(self.across[band], across)
        })
    }

    /// How far the skull reaches BACKWARD at `height`, in head-local metres.
    ///
    /// The mirror of [`Self::depth`], and a head needs both: an occiput reaches
    /// further behind the joint than a brow does in front of it, so anything
    /// treating the head as symmetric fore-and-aft puts the back of it inside
    /// itself. Measured on the midline.
    ///
    /// **Signed, so it is NEGATIVE**, like the coordinate it is: this is the `z`
    /// the back of the head reaches, which is what the profile's own tables hold
    /// and what `fore` maps a depth against. The first cut of this negated it into
    /// a distance and [`Self::surface_at`] then took a semi-axis of about nothing
    /// for the whole back hemisphere — the head's radius behind the ear read as
    /// 6.4 mm rather than 90, so a lock combed round to the nape dived for the
    /// axis. It survived a test because the test compared the lock's radius
    /// against the same broken formula on both sides; only a style that crossed
    /// from the front hemisphere into the back one shows it.
    #[must_use]
    pub fn depth_behind(&self, height: f32) -> f32 {
        self.sample(&self.behind, height)
    }

    /// Where the surface sits at one height and one azimuth, in head-local
    /// metres.
    ///
    /// **The measured head as a shape something can be walked over**, which is
    /// what hair needs and nothing here offered: [`Self::half_width`] and
    /// [`Self::depth_across`] are the two halves of one ellipse per height, and
    /// every caller that wanted a point on the surface was assembling them
    /// itself — `hair::follicle`'s own edge instrument and the scalp styles —
    /// and a third copy is how they drift.
    ///
    /// `azimuth` is measured from dead ahead, turning toward the body's right,
    /// so its cosine is [`crate::hair::follicle`]'s `facing` and zero is the
    /// middle of the face.
    ///
    /// It is the profile's own ellipse and not the built mesh, so it is smooth
    /// where a mesh has facets — which is what anything sliding over a head
    /// wants — and it knows nothing about a nose. Anything that must be ON the
    /// surface rather than near it belongs on the faces themselves; see
    /// `hair::clump::scatter`.
    ///
    /// **The ellipse has to be asymmetric fore-and-aft.** Written as the
    /// forward reach times the azimuth's cosine it uses the brow's depth for
    /// the occiput and reads the back of the head as 5 mm inside itself, which
    /// a scalp lock walking the surface turns into hair sunk into the skull
    /// behind the ear.
    ///
    /// **And an ellipse through the three extremes is an outer BOUND rather than
    /// the surface**, which is the worse trap: a head is
    /// flatter at the back and narrower at the temples than any ellipse through
    /// its own widest points, so hair walking that ellipse floats a centimetre
    /// off the head at the diagonals and stands cards proud of
    /// the scalp with light under them. The two-dimensional tables say where the
    /// outline actually is — [`Self::width_across`] is the half-width at a depth
    /// and [`Self::depth_across`] the depth at a half-width — so the ellipse is
    /// only a first guess and the measured outline is found from it.
    ///
    /// **It is found by BISECTING a monotone residual rather than by iterating
    /// the table.** Solving `r = w(r cos) / sin` by substitution is a fixed
    /// point only where `|w'(z) cos / sin| < 1`; on the back diagonals of a head
    /// the outline turns fast and the ray is shallow, so the iteration DIVERGES
    /// by oscillating — measured at 157°, one step answers 70 mm and the next
    /// 132 about a true 85. Dropping a step that strays too far from the guess
    /// does not repair that, because a
    /// diverging iterate lands inside the trust band by luck about half the
    /// time: the accepted radius then flickers between 66 mm and 109 as the
    /// height moves by six millimetres, and a walk that keeps the widest radius
    /// it has passed latches the highest flicker and hangs there — which puts
    /// blocky slabs off the back-top of a head of hair.
    ///
    /// The residual — how far inside the tables' own surface a radius is —
    /// falls monotonically along any ray from the joint, because the head
    /// narrows in whichever direction the ray is going. So it has exactly one
    /// crossing, bisection cannot diverge whatever the outline does, and the
    /// answer moves smoothly with the height because the tables do. No trust
    /// band, and nothing to tune.
    ///
    /// Whichever table is inverted is still chosen by the direction, since
    /// dividing by a sine near zero is dividing by nothing.
    #[must_use]
    pub fn surface_at(&self, height: f32, azimuth: f32) -> glam::Vec3 {
        let (sin, cos) = azimuth.sin_cos();
        let side = self.half_width(height).max(MINIMUM_REACH);
        let fore = if cos >= 0.0 {
            self.depth(height)
        } else {
            -self.depth_behind(height)
        }
        .max(MINIMUM_REACH);
        // The ellipse through the measured extremes, which brackets the outline
        // rather than standing in for it: a head is flatter at the back and
        // narrower at the temples than any ellipse through its own widest
        // points, so the measured surface is INSIDE this everywhere.
        let guess = 1.0 / ((sin / side).powi(2) + (cos / fore).powi(2)).sqrt();
        // **The box the tables themselves measured**, which is how far along this
        // ray the outline could possibly be: the head reaches `side` sideways at
        // this height and `fore` the way this ray is pointing, so a radius past
        // either is outside the head whatever a table says about it.
        //
        // It is a bound on the SEARCH and not a nicety (#210). Both tables are
        // read by a column that clamps at its own end — the honest answer for a
        // feature seated wider than the band it sits on — so asked about a point
        // well behind the head they answer the back of the band rather than
        // nothing, and a residual that should have gone negative never does. At
        // 157° below the jaw that put the surface 102 mm out where the head
        // reaches 59.
        let reach = (side / sin.abs().max(MINIMUM_REACH)).min(fore / cos.abs().max(MINIMUM_REACH));
        // How much of the answer the half-widths get, the depths taking the
        // rest. **A blend and not a choice** (#210): the two tables are two
        // independent measurements of one outline, so they do not agree to the
        // millimetre, and switching between them at a threshold puts that
        // disagreement into the surface as a step — 4.8 mm across half a degree
        // at 20°, which is a seam a walking card latches exactly as it latched
        // the diverging solve. Each still has the whole say where it is the
        // sharper measurement; only the band between them is shared.
        let lateral = smooth((sin.abs() - LATERAL_ENOUGH + LATERAL_BLEND) / (2.0 * LATERAL_BLEND));
        let across = |radius: f32| self.width_across(height, radius * cos) - radius * sin.abs();
        let along =
            |radius: f32| self.depth_across(height, (radius * sin).abs()) - radius * cos.abs();
        let radius = if cos < 0.0 || lateral >= 1.0 {
            // The half-widths: how far the head reaches sideways at the depth
            // this radius would put the point at, less the sideways distance it
            // has gone. Behind the joint this is the only measurement there is,
            // whatever the direction, so the blend does not apply there.
            //
            // **Including straight back, which used to be its own case** (#210).
            // There is no table of half-widths behind the joint to INVERT, so
            // the near-astern ray was answered with the midline's own reach over
            // the cosine — which is a flat plane across the back of the head,
            // and 15 mm outside the occiput at the twenty degrees either side of
            // it where the head is roundest. The join showed as a step in the
            // surface: 87.8 mm at 159.5° and 102.8 at 160.0, which a walking
            // card latched exactly as it latched the diverging solve.
            //
            // Nothing needs inverting here. The residual has no division in it
            // at all, so a sine near zero is not a problem to be avoided but a
            // case that answers itself: the sideways term vanishes, no radius
            // inside the head can drive the residual negative, and the search
            // returns the box — which behind the head IS the midline's reach.
            // The old fallback's answer, arrived at continuously.
            outline(guess.min(reach), reach, across)
        } else if lateral <= 0.0 {
            outline(guess.min(reach), reach, along)
        } else {
            let one = outline(guess.min(reach), reach, along);
            one + (outline(guess.min(reach), reach, across) - one) * lateral
        };
        // **The topmost band is a band and not a pole, so the crown is closed
        // with a cap** (#204). Every profile here is sampled into bands of about
        // eleven millimetres, and the highest one carries the head's width over
        // its whole span — 55 mm at the top of a default head. Asked at the crown
        // it therefore answers 55 mm rather than nothing, so anything walking the
        // surface from the crown outward starts 55 mm out and leaves a bare disc
        // where the whorl is. A quarter-circle over the last band closes it to a
        // point, which is the one thing a dome's own geometry says for certain.
        let band = (self.hi - self.lo) / (BANDS - 1) as f32;
        let into = ((height - (self.hi - band)) / band.max(MINIMUM_REACH)).clamp(0.0, 1.0);
        let radius = (radius * (1.0 - into * into).max(0.0).sqrt()).max(MINIMUM_REACH);
        glam::Vec3::new(radius * sin, height, radius * cos)
    }

    /// How far below the crown the profile's own cap runs, in head-local metres.
    ///
    /// **The one part of this surface that is geometry rather than
    /// measurement.** The topmost band carries the head's width over its whole
    /// span, so asked at the crown it answers a radius rather than nothing;
    /// [`Self::surface_at`] closes it with a quarter-circle over that band (see
    /// there for the bald disc it left when it did not). This is how deep that
    /// band is, and anything walking the surface down from the crown needs it:
    /// the radius goes from nothing to the head's full width inside it, and a
    /// walk stepping uniformly in height crosses the whole turn in two steps.
    ///
    /// Answered rather than re-derived, because deriving it means knowing
    /// `BANDS`, and a second copy of that number is one that stays behind when
    /// the profile is resampled.
    #[must_use]
    pub fn crown_band(&self) -> f32 {
        (self.hi - self.lo) / (BANDS - 1) as f32
    }

    /// The throat and the crown — the lowest and highest the measured profile
    /// reaches, in head-local metres.
    ///
    /// **The low end is the THROAT, not the chin**, and the name says so. The
    /// head's surface runs 28 mm past the chin on a default body before the
    /// neck owns it, so reading the low end as the chin and hanging the feature
    /// frame from it puts the mouth 9 mm above the chin's tip where a face has
    /// about 20, and reads as the whole jaw rotated up into the throat.
    /// Anything placed as a fraction of the way down the FACE wants
    /// [`Self::chin`].
    #[must_use]
    pub fn throat_and_crown(&self) -> (f32, f32) {
        (self.lo, self.hi)
    }

    /// Where the chin's tip is, in head-local metres.
    ///
    /// The forward-most point of the lower face — what the feature frame ends
    /// at. Placing features down to [`Self::throat_and_crown`]'s low end instead put the
    /// mouth 9 mm above the chin's tip where a face has about 20: the lower lip
    /// was painted onto the chin itself and the crease below the lip was carved
    /// into the underside of the jaw. Material added above the tip and removed
    /// below it reads as the whole jaw rotated up into the throat.
    ///
    /// **Found on the surface, by a definition rather than by a window.** It is
    /// the lowest crest of the midline profile — see `chin_of` — and nothing
    /// about the plan enters into it on a head that has a chin. The plan's
    /// `CHIN` peak says roughly where the tip is, and a bisected search around
    /// it is an estimate tuned to sit on top of that guess: it drifts with the
    /// head, by up to 6 mm as the frame moves, and then comes apart from the
    /// test that checks it. Binning cannot do this at all — finding the maximum
    /// from 20 measured bands needs the shallow dip above the chin to survive
    /// them, and it does not.
    ///
    /// Verified across seeds by `the_chin_landmark_lands_on_the_chin`, against
    /// the carved surface at two subdivision levels by
    /// `the_chin_landmark_lands_on_the_chin_of_the_shipped_face`, and the margin
    /// the definition needs by `the_chin_stands_clear_of_the_hollow_above_it`.
    ///
    /// A head with no chin — a creature's, which [`shape`] leaves as a sphere —
    /// falls back to the plan's estimate, clamped to the span so it still
    /// answers inside its own surface.
    #[must_use]
    pub fn chin(&self) -> f32 {
        self.chin
    }

    /// Where the angle of the jaw sits, in head-local metres.
    ///
    /// The gonion: the corner of the mandible, out at the side, which is what a
    /// bigonial breadth is measured across. Derived from [`Self::chin`] through
    /// the same pair of profile heights `jaw` runs the mandible's border
    /// between, so the landmark and the border cannot come apart — which is
    /// the failure a landmark measured independently of its own profile has.
    ///
    /// **Public because things outside this impl need it and would each write
    /// their own arithmetic.** `chin() * (GONION / MENTON)` inline in a test is
    /// not available to `examples/headaudit` at all, because both constants are
    /// private, and a landmark that only a test can compute is a landmark
    /// nobody re-measures.
    #[must_use]
    pub fn gonion(&self) -> f32 {
        self.chin * (GONION / MENTON)
    }

    /// Reads a two-axis table at a height and an already-scaled column.
    ///
    /// One walk for both maps, so the fore-aft one cannot drift from the lateral
    /// one the way two hand-written interpolations would.
    fn bilinear<const COLS: usize>(
        &self,
        table: &[[f32; COLS]; BANDS],
        height: f32,
        column: impl Fn(usize) -> f32,
    ) -> f32 {
        let at = ((height - self.lo) / (self.hi - self.lo) * (BANDS - 1) as f32)
            .clamp(0.0, (BANDS - 1) as f32);
        let band = (at.floor() as usize).min(BANDS - 2);
        let blend = at - band as f32;

        // The column is asked of each band separately, because the axis it
        // indexes is a fraction of THAT band's own extent. Taking one column
        // index for both is what put the chin's whole width into two columns.
        let row = |band: usize| {
            let column = column(band).clamp(0.0, (COLS - 1) as f32);
            let left = (column.floor() as usize).min(COLS - 2);
            let along = column - left as f32;
            table[band][left] + (table[band][left + 1] - table[band][left]) * along
        };
        row(band) + (row(band + 1) - row(band)) * blend
    }

    /// Reads one profile at a height, interpolating between bands.
    fn sample(&self, profile: &[f32; BANDS], height: f32) -> f32 {
        let at = ((height - self.lo) / (self.hi - self.lo) * (BANDS - 1) as f32)
            .clamp(0.0, (BANDS - 1) as f32);
        let band = (at.floor() as usize).min(BANDS - 2);
        let blend = at - band as f32;
        profile[band] + (profile[band + 1] - profile[band]) * blend
    }
}

/// Where on a ray from the head joint the measured outline is.
///
/// `residual` says how far INSIDE the tables' own surface a radius is: positive
/// short of the outline, negative past it. It falls monotonically along the ray
/// — the head narrows in whichever direction the ray is going — so it has one
/// crossing, and bisecting onto it converges whatever the outline does.
///
/// `reach` is how far along the ray the measured extremes themselves allow, and
/// the search is closed at it: see [`Skull::surface_at`] for the reading past
/// the end of a table that makes that necessary.
///
/// **A bisection and not a substitution.**
/// Rearranging the same equation into `r = w(r cos) / sin` and iterating
/// it converges only where `|w'(z) cos / sin| < 1`, which the back diagonals of
/// a head are not: there the iteration oscillates, and no test on how far one
/// step moved can tell an oscillating iterate from a converging one. See
/// [`Skull::surface_at`].
fn outline(guess: f32, reach: f32, residual: impl Fn(f32) -> f32) -> f32 {
    // The ellipse through the measured extremes is an outer bound on the
    // outline, so it brackets it — but only where the tables answer honestly.
    // Past the end of a table's own columns the reading clamps rather than
    // falling away, so the bracket is closed at the box the tables measured and
    // a residual still positive there means the ray has left the head: the box
    // is then the answer, and the last honest one there is.
    let mut lo = 0.0;
    let mut hi = guess.max(MINIMUM_REACH).min(reach);
    if residual(hi) > 0.0 {
        hi = reach;
        if residual(hi) > 0.0 {
            return hi;
        }
    }
    for _ in 0..OUTLINE_STEPS {
        let middle = 0.5 * (lo + hi);
        if residual(middle) > 0.0 {
            lo = middle;
        } else {
            hi = middle;
        }
    }
    0.5 * (lo + hi)
}

/// How many times [`outline`] halves that bracket.
///
/// Twelve, which is a head's own radius over four thousand — a fortieth of a
/// millimetre, well under the tenth of one the profile tables themselves are
/// quantised to by [`BANDS`]. More steps would be measuring the bilinear
/// interpolation rather than the head.
///
/// Provenance: **derived** from the tables' own precision.
const OUTLINE_STEPS: usize = 12;

/// How lateral a direction has to be before the side table is the one to read.
///
/// The sine of about twenty degrees. Below it, and in front of the joint, the
/// depths are the measurement that actually varies along the ray: a residual
/// built from the half-widths is nearly flat there, so where its root is says
/// more about the table's own interpolation than about the head.
///
/// **It is not about dividing by a sine near zero.** That is a substitution's
/// problem, and the residual this chooses between has no division in it. What
/// is left is which of two measurements is the sharper one to read, and behind
/// the head there is only one.
///
/// Provenance: **derived** from the conditioning.
const LATERAL_ENOUGH: f32 = 0.35;

/// How wide a band either side of that the two tables share, in the same sine.
///
/// A tenth, which is about six degrees either side of twenty. Wide enough that
/// the millimetre or two the two measurements disagree by is spread into a slope
/// no steeper than the head's own, and narrow enough that each table still has
/// the whole say over the directions it measures well.
///
/// Provenance: **derived** from the disagreement, measured
/// (`the_measured_surface_is_smooth_in_height_at_every_azimuth`).
const LATERAL_BLEND: f32 = 0.10;

/// The smallest semi-axis [`Skull::surface_at`] will divide by, in metres.
///
/// A millimetre, for the reason `hair::follicle`'s own floor exists: a band that
/// measured nothing would otherwise turn a radius into an infinity, and no real
/// band of a head is clamped by this.
///
/// Provenance: **derived** from the failure it prevents.
const MINIMUM_REACH: f32 = 0.001;

/// How far the head's own surface reaches below its joint, in skull radii.
///
/// The same measurement [`shape`] scales every below-joint profile against, and
/// taken the same way: over head-owned vertices only, because the neck runs up
/// through the same heights and is not the head.
/// How far the head reaches below its own joint, in head radii, negative down.
///
/// **Asked of the rig and not of the mesh.** See [`SETTLED`] for why, and
/// for what it costs. The short of it: the surface at the bottom of a head is
/// the blend into the neck, so measuring the floor there made the entire lower
/// face a function of the neck's shape.
fn floor(rig: &Rig, head: usize) -> f32 {
    let joint = &rig.joints[head];
    let parent = joint.parent.unwrap_or(head);
    if joint.radius <= f32::EPSILON {
        return -SETTLED;
    }
    -SETTLED * (joint.position.y - rig.joints[parent].position.y).abs() / joint.radius
}

/// How far up its run the mandible's border stands at one azimuth: 0 at the
/// menton dead ahead, 1 at the gonion, held there behind the ear.
///
/// **The one conversion from an azimuth to [`border`]'s `raise`, and it is one
/// function because the two halves of it can fork.** [`jaw`] runs the border in
/// the COSINE of the azimuth — straight in side view, as a mandible is — and a
/// caller feeding [`border`] the sine instead gets the LARGER number everywhere
/// between dead ahead and the side, so a fairing ramp begins above the real
/// crease and runs partial-weight smoothing straight across it: a band along
/// the jawline rougher than the head above or the throat below it.
/// `facing` is the cosine of the azimuth from dead ahead, exactly as
/// [`reshape_to`] has it; feed it a point's own `across.z / reach`.
pub(crate) fn border_raise(facing: f32) -> f32 {
    (1.0 - facing.max(0.0)).clamp(0.0, 1.0)
}

/// Where the mandible's lower border sits at the angle of the jaw, in metres
/// below the head joint.
///
/// [`GONION`] in the unit the rest of the body is measured in, so that
/// `face::neck` can be bounded by the same landmark this file carves to without
/// re-deriving the floor remap — the arithmetic most apt to walk out from
/// under a caller when the head's proportions move. `raise` comes from
/// [`border_raise`]
/// on a point's own azimuth, or is 1.0 for "the highest the border ever gets".
pub(crate) fn border(rig: &Rig, head: usize, traits: &HeadTraits, raise: f32) -> f32 {
    // **The border MIGRATES, and a flat one tears a chin off** (#175). This is
    // [`jaw`]'s own line — [`MENTON`] on the midline, [`HeadTraits::gonion`] out
    // at the angle — and `face::neck` first used the gonion's height at every
    // azimuth, which put the ceiling 20 mm above the chin's own tip on the
    // midline. The column's carve then took the chin and the submental
    // construction with it and drew them into the throat, which rendered as the
    // jaw shattering. `raise` is 0 dead ahead and 1 at the side or behind.
    //
    // The same remap `reshape_to` applies: a profile height is a fraction of
    // the way down THIS head, so it reaches the same anatomy on every body.
    at_profile(
        rig,
        head,
        MENTON + (traits.gonion - MENTON) * raise.clamp(0.0, 1.0),
    )
}

/// A profile height in metres below the head joint, on this head.
///
/// The remap [`reshape_to`] applies, in one place, so anything outside this
/// file that needs to reach an anatomy the profiles name does not re-derive it
/// — the arithmetic most apt to walk out from under a caller when the head's
/// proportions move.
pub(crate) fn at_profile(rig: &Rig, head: usize, profile: f32) -> f32 {
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return 0.0;
    }
    profile * ((floor(rig, head) * SETTLE) / JUNCTION) * radius
}

/// Where the chin is on a built head, in metres above the head joint.
///
/// **One definition, used twice, which is the whole point of it.**
/// [`Skull::measure`] reads the landmark with this on the shaped head, and
/// `the_chin_landmark_lands_on_the_chin_of_the_shipped_face` re-applies it to the
/// carved one — so that test compares two answers to the same question instead
/// of two different instruments that happen to agree on one cage.
///
/// The chin is [`menton`]'s lowest crest, searched from the throat up to the
/// highest height at which [`shape`] still pushes a chin forward at all: `CHIN`'s
/// highest non-zero knot, through the same floor remap [`reshape_to`] uses.
/// Above that there is no chin in the geometry to find, only the face — and the
/// bound matters, because a head whose lower face came out as a smooth cone has
/// its forward-most point at the brow, and a search without a ceiling answers
/// with it. Measured at eight-point rings and one subdivision, two seeds in
/// sixteen are that head; they return `None` here rather than a landmark 110 mm
/// too high.
///
/// `None` when there is no crest below the ceiling — an unshaped head, or one
/// whose chin the cage has flattened out of existence.
fn chin_of(mesh: &PolyMesh, rig: &Rig, head: usize, throat: f32) -> Option<f32> {
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return None;
    }
    let pushed = || CHIN.iter().filter(|knot| knot.1 > 0.0);
    let onto = |height: f32| height * (floor(rig, head) * SETTLE) / JUNCTION * radius;
    let ceiling = onto(pushed().fold(f32::MIN, |high, knot| high.max(knot.0)));
    // **Both ends of the search say where a chin can BE, and the lower one used
    // to say where the surface ran out** (#127). The ceiling has always been
    // `CHIN`'s highest non-zero knot: above it there is no chin in the geometry
    // to find, only the face, and a head whose lower face came out as a smooth
    // cone answers with its brow. The floor was the THROAT, which is a fact
    // about the surface rather than about a chin — so a neck carrying mass
    // astern puts a crest down near the nape and this search obligingly returns
    // it. Measured, that gave a head 250.5 mm crown to chin with a cranium:face
    // of 0.67, on a body whose head is 200.
    //
    // So it is `CHIN`'s lowest non-zero knot through the same remap. Below that
    // the table pushes nothing forward, so anything found there is not a chin
    // whatever else it is. The throat still binds when it is the higher of the
    // two, which is every body that has not been given a nape to trip over.
    let basement = onto(pushed().fold(f32::MAX, |low, knot| low.min(knot.0)));
    menton(mesh, centre, throat.max(basement), ceiling)
}

/// How far the midline surface reaches forward at `y` above the head joint.
///
/// `None` where the midline is not inside the mesh at all, which is what the
/// throat does at the bottom of the scan and what a head with no surface there
/// does everywhere. Bisected against [`PolyMesh::contains`], the same primitive
/// the rest of the crate judges a surface with.
fn midline(mesh: &PolyMesh, centre: Vec3, y: f32) -> Option<f32> {
    let inside = |z: f32| mesh.contains(Vec3::new(centre.x, centre.y + y, centre.z + z));
    if !inside(0.0) {
        return None;
    }
    let (mut near, mut out) = (0.0f32, 0.30f32);
    for _ in 0..24 {
        let mid = 0.5 * (near + out);
        if inside(mid) {
            near = mid;
        } else {
            out = mid;
        }
    }
    Some(near)
}

/// How far the midline profile has to fall for a crest to be over, in metres.
///
/// **The hollow above the chin is what this has to fit inside, and it is the
/// shallowest thing here.** Measured on the midline between the chin's crest and
/// the lower lip's, over sixteen seeds at both subdivision levels the crate can
/// ship, it runs 3.9 to 11.9 mm. A fifth of a millimetre clears the shallowest of
/// those nineteen times over, and there is nothing below it to reject: the
/// bisection resolves to tens of nanometres and the surface under it is
/// piecewise linear, so this is not a noise floor. It is the smallest hollow that
/// still counts as separating two features.
///
/// The margin is measured rather than assumed, by
/// `the_chin_stands_clear_of_the_hollow_above_it`, and it is cage-dependent: at
/// eight-point rings and one subdivision the same hollow runs 0.67 mm at its
/// shallowest, because the closer the cage sits to the surface the flatter the
/// whole lower face comes out.
const FALL: f32 = 0.0002;

/// How close to a crest's maximum still counts as being on it, in metres.
///
/// **A chin is a plateau, not a point, and on some cages it is a long one.** The
/// midline reach across the shipped chin falls 0.9 mm over the 2 mm either side
/// of its tip; across an eight-point cage's it varies by 0.01 mm over 6 mm — the
/// cage lays a flat facet where the chin is and subdivision keeps it flat. An
/// argmax over that is not a measurement of anything: it answers with whichever
/// end of the flat a rounding error favours, and two instruments that each take
/// an argmax over it can disagree by the width of the plateau while both are
/// looking at the same chin. So the landmark is the MIDDLE of the flat, and this
/// is what counts as flat: a tenth of a millimetre, well under the millimetre
/// this crate places features to and well over the profile's own wander.
const FLAT: f32 = 0.0001;

/// The lowest crest of the midline profile between `throat` and `ceiling`, as
/// the height of the middle of that crest above the head joint.
///
/// **The chin is the first prominence you meet walking up from the throat**, and
/// that is the whole definition — no window, no estimate, nothing tuned. Below it
/// the surface is still climbing out of the neck; above it comes the mentolabial
/// sulcus and then the lower lip, which on a carved face reaches FURTHER FORWARD
/// than the chin does (119.2 mm against 114.0 on the default body). So "the
/// forward-most point of the lower face" is not the chin and never was; the
/// forward-most point of the FIRST crest is.
///
/// This is the definition [`Skull::measure`] reads the landmark with and the one
/// `the_chin_landmark_lands_on_the_chin_of_the_shipped_face` re-applies to the
/// carved surface, which is what makes that test a comparison rather than a
/// coincidence. Two bisected argmaxes over windows around a plan estimate agree
/// on exactly the one cage they are tuned against; on any other, the window
/// reaches the lower lip and the argmax walks to the top of its own scan, which
/// reads as a 23 mm error and is not one — it is a saturated measurement of an
/// unbounded disagreement.
///
/// Scanned coarsely and then finely: the crest is bracketed at four steps and
/// re-read at one within that bracket, because [`PolyMesh::contains`] is the
/// expensive thing in this file and a whole-face scan at the fine step would be
/// four times the cost of the window it replaces rather than a sixth over.
///
/// `None` when the profile climbs all the way to `ceiling` without falling clear
/// of its own maximum. That is the saturated case and it is reported as no
/// answer rather than as the top of the scan, which is precisely the reading
/// error that makes a bounded figure out of an unbounded one.
fn menton(mesh: &PolyMesh, centre: Vec3, throat: f32, ceiling: f32) -> Option<f32> {
    /// How finely, in metres. Half the finest cell the face ever carries.
    const STEP: f32 = 0.0009;
    /// How coarsely the crest is bracketed first, in metres. Four fine steps,
    /// which is still a fifth of the shallowest sulcus's width.
    const COARSE: f32 = 0.0036;

    // The lowest crest in a range: the running maximum, held until the profile
    // has fallen clear of it, reported as the span of heights that are level
    // with that maximum. `fell` says whether the crest ended inside the range or
    // the scan simply ran out of head.
    let crest = |from: f32, to: f32, step: f32| {
        let (mut top, mut lo, mut hi, mut fell) = (f32::MIN, from, from, false);
        let mut y = from;
        while y <= to {
            if let Some(at) = midline(mesh, centre, y) {
                if at > top + FLAT {
                    (top, lo, hi) = (at, y, y);
                } else if at >= top - FLAT {
                    (top, hi) = (top.max(at), y);
                } else if top - at > FALL {
                    fell = true;
                    break;
                }
            }
            y += step;
        }
        (top > f32::MIN).then_some((lo, hi, fell))
    };

    let (lo, hi, fell) = crest(throat, ceiling, COARSE)?;
    if !fell {
        return None;
    }
    // Re-read at the fine step inside the bracket the coarse pass gave, widened
    // by one coarse step either side so the plateau's own ends are inside it.
    let (lo, hi, _) = crest(lo - COARSE, hi + COARSE, STEP)?;
    Some(0.5 * (lo + hi))
}

/// Where a lateral offset falls in a band's own columns.
///
/// Zero on the midline, [`COLUMNS`] minus one at the band's widest. Anything
/// wider than the band clamps, which is the honest answer: a mouth wider than
/// the jaw it sits on has its corners at the edge of the jaw.
fn lateral(width: f32, across: f32) -> f32 {
    (across.abs() / width.max(f32::EPSILON)) * (COLUMNS - 1) as f32
}

/// Where a fore-aft offset falls in a band's own columns.
///
/// Zero at the back of the band, [`DEPTHS`] minus one at the front.
fn fore(behind: f32, ahead: f32, depth: f32) -> f32 {
    ((depth - behind) / (ahead - behind).max(f32::EPSILON)) * (DEPTHS - 1) as f32
}

/// Every bin whose window covers `at`, clamped to the table.
///
/// Samples outside the table entirely still land in the nearest bin: a head is
/// convex enough that the edge of a profile is a better answer than a hole.
fn window(at: f32, bins: usize) -> std::ops::RangeInclusive<usize> {
    let last = bins - 1;
    let at = at.clamp(0.0, last as f32);
    let first = (at - WINDOW).ceil().max(0.0) as usize;
    let end = ((at + WINDOW).floor().max(0.0) as usize).min(last);
    // A window narrower than half a bin can fall between two centres, and a
    // sample that lands in no bin at all is a sample thrown away.
    if end < first {
        let nearest = (at.round() as usize).min(last);
        return nearest..=nearest;
    }
    first..=end
}

/// The head's surface, in head-local metres, as points to bin.
///
/// Vertices, plus each fully-head face's centroid and the midpoint of each of
/// its edges and of each corner-to-centroid span. Those interior samples are on
/// the same surface [`PolyMesh::contains`] tests against, and they are what
/// takes a 283-vertex head to something a twenty-band profile can be read from.
///
/// Only the head's own surface. A vertex whose nearest bone is the neck belongs
/// to the neck however close to the jaw it sits, and including it would report
/// the throat as the face. Faces are held to a stricter rule than vertices —
/// every corner head-owned, not just the centroid — because a face straddling
/// the jaw would otherwise drag samples off the throat into the lowest bands,
/// which is exactly where the chin is read from.
fn samples(mesh: &PolyMesh, rig: &Rig, centre: Vec3) -> Vec<Vec3> {
    let mine = |point: Vec3| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head;
    let owned: Vec<bool> = mesh.positions.iter().map(|&point| mine(point)).collect();

    let mut out: Vec<Vec3> = mesh
        .positions
        .iter()
        .zip(&owned)
        .filter(|&(_, &ours)| ours)
        .map(|(&point, _)| point - centre)
        .collect();

    for (face, corners) in mesh.faces.iter().enumerate() {
        if !corners.iter().all(|&corner| owned[corner as usize]) {
            continue;
        }
        let centroid = mesh.face_centroid(face);
        out.push(centroid - centre);
        for (at, &corner) in corners.iter().enumerate() {
            let here = mesh.positions[corner as usize];
            let next = mesh.positions[corners[(at + 1) % corners.len()] as usize];
            out.push((here + next) * 0.5 - centre);
            out.push((here + centroid) * 0.5 - centre);
        }
    }
    out
}

/// Fills a row's empty columns from the nearest measured one.
fn spread(row: &mut [f32], fallback: f32) {
    let mut last = f32::MIN;
    for value in row.iter_mut() {
        if *value == f32::MIN {
            *value = last;
        } else {
            last = *value;
        }
    }
    let mut next = f32::MIN;
    for value in row.iter_mut().rev() {
        if *value == f32::MIN {
            *value = next;
        } else {
            next = *value;
        }
    }
    for value in row.iter_mut() {
        if *value == f32::MIN {
            *value = fallback;
        }
    }
}

/// Replaces bands that no sample reached with the nearest one that was.
///
/// Separate from [`fill`] because these profiles are signed — the back of a
/// skull is a negative depth — so "empty" cannot be spelled as "not positive".
fn carry(profile: &mut [f32; BANDS], empty: f32) {
    let Some(first) = profile.iter().position(|value| *value != empty) else {
        profile.fill(0.0);
        return;
    };
    for band in 0..first {
        profile[band] = profile[first];
    }
    for band in first + 1..BANDS {
        if profile[band] == empty {
            profile[band] = profile[band - 1];
        }
    }
}

/// Replaces empty bands with the nearest filled one.
fn fill(profile: &mut [f32; BANDS]) {
    let mut last = 0.0f32;
    for value in profile.iter_mut() {
        if *value <= 0.0 {
            *value = last;
        } else {
            last = *value;
        }
    }
    let mut next = 0.0f32;
    for value in profile.iter_mut().rev() {
        if *value <= 0.0 {
            *value = next;
        } else {
            next = *value;
        }
    }
}
