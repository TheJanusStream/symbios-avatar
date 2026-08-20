//! The column between the jaw and the shoulders, carved rather than swept.
//!
//! **Left to the cage alone, the head sits on a stump — the cage's stump.** A
//! neck arrives from [`crate::cage`] as a swept ring of its own radius, and that
//! radius is a fraction of STATURE times girth times the frame axis, while the
//! skull above it is a fraction of stature times `head_size`. The two share
//! stature and nothing else, so a body could carry either on the other:
//! measured over an ordinary grid with no carve, the built column ran from
//! 0.65 of the skull's own width to 1.21 of it — at `head_size` −1 the neck
//! WIDER than the head, which reads as a knob on a post.
//!
//! So the column's width stops being the cage's business. What the sweep
//! provides is surface; what this decides is how much of it there is, in HEAD
//! RADII, which is what makes a mismatch impossible rather than unlikely.
//!
//! **It spans the junction deliberately, and that is the whole trick.** The
//! visible neck is not one region: measured on the default body, 46 mm of it is
//! head-owned surface hanging below the chin, 13 mm is the gap down to the neck
//! joint and 47 mm is the neck's own bone. `face::skull` shapes the first and
//! lets go before the rest — see `SETTLE`, which leaves a band of head
//! deliberately unshaped so it matches whatever cylinder the cage put under it.
//! A carve that started at the head's floor would therefore start BELOW the
//! neck's own narrowest point and could never put a waist where a waist goes.
//! This one is bounded by two landmarks instead — the mandible's border above
//! and the girdle's crown below — and reaches identity at both, so it composes
//! with the skull's profiles rather than fighting them.
//!
//! **And then the whole band is faired** ([`fair`]). Three systems share
//! the mandible-to-throat skin — `CHIN`'s tail, `construct_submental`'s chords
//! and [`shape`]'s narrowing — and their raw seams render as a hanging tab, a
//! crevasse and shelf-breaks. The region is shaped by render, with the
//! instruments re-fit after, and the shape that survives that
//! judgement is: every seam smoothed out of the finished surface, one
//! femininity-scaled laryngeal prominence raised on the result.

use crate::mesh::PolyMesh;
use crate::rig::Rig;
use crate::{Vec3, Zone};

use super::skull::{HeadTraits, border, border_raise};
use super::smooth;

/// What a neck is worth against the skull it carries, as a fraction of the
/// built skull's own widest half-width.
///
/// **Sourced from one mannequin and corroborated from outside it, and the
/// caveat travels with the number.** The obvious derivation — measure
/// both CC0 references, as `HeadTraits`'s whole set is measured — does not
/// work here, and that was established by instrument, not assumed: the
/// male is 7,399 vertices so a fine band holds one ring or none; a ray from the
/// skull's axis is not a half-width once it leaves the skull, because a neck
/// stands behind that axis; a swept section at neck height contains the
/// trapezius as well as the neck. An instrument clear of all three reads 0.716
/// on the male, and the female's `neck_01` weights own so much shoulder that
/// her column measures WIDER below the jaw than at it. No threshold makes the
/// two files the same selection.
///
/// 0.716 is corroborated from anthropometry rather than from the other
/// mannequin: neck breadth against head breadth is about 110 mm against 152,
/// which is 0.72. Two independent routes to the same figure on a quantity where
/// the uncarved neutral body reads 0.90 is enough to author against.
///
/// Provenance: **measured on the male reference mannequin, corroborated by
/// anthropometry; the female mannequin cannot carry the measurement**.
const NECK_SKULL: f32 = 0.72;

/// What the built skull's widest half-width is worth against its own node.
///
/// [`NECK_SKULL`] is a fraction of the SURFACE's width and this converts it to
/// the rig's, so the target costs nothing to compute: measuring the built skull
/// would mean a second `Skull::measure`, which is 61% of a geometry build on
/// its own.
///
/// **Measured rather than assumed, and it is stable because the head's carve is
/// a set of factors on its own node.** Over a grid of `head_size`, `mass` and
/// `femininity`, the built skull's widest against `radius * scale.x` reads:
///
/// ```text
///   head_size    -1.0    0.0    +1.0
///   widest/node  0.782  0.780   0.781
/// ```
///
/// Eleven of the twelve cells land in 0.780–0.790. The twelfth is 0.823, at
/// `head_size` −1 with `mass` +1 — which is the one body in the grid whose neck
/// was WIDER than its head, so the instrument's "skull's widest" was reading
/// neck. That cell is the defect this module exists to remove.
///
/// Provenance: **measured over the parameter grid**.
const SKULL_OF_NODE: f32 = 0.781;

/// How much of the run the carve takes to arrive, below each point's own
/// border, as a fraction of the whole column.
///
/// **A share of the run and not a fixed depth**, so a long neck eases in over a
/// longer distance and a short one does not take the carve as a step. Measured
/// on the uncarved column, the narrowest section sits 80 mm
/// below the head joint against a border at 55 and a girdle crown at 210, so
/// the waist is about a sixth of the way down and the ramp has to be shorter
/// than that or the neck is at its narrowest nowhere.
///
/// Provenance: **measured on the built column**.
const RAMP: f32 = 0.16;

/// Where the shoulders take the column back, in the same fractions.
///
/// The neck holds its own width from the end of [`RAMP`] down to here and then
/// lets go, so the surface between them cannot swell — which is what an unheld
/// column does: `neckaudit` counts three turns down one, swell then pinch then
/// swell, where a neck narrows into its shoulder and stops. Held, it counts
/// one.
///
/// Provenance: **tuned by render**.
const RELEASE: f32 = 0.72;

/// How much of the carve still reaches the throat, dead ahead.
///
/// **Zero would be a neck that narrows sideways and nowhere else, and one tears
/// the jaw off.** Scaling the whole section about the
/// column's axis draws the throat back with the sides while the chin in
/// front of it stays exactly where the skull's own profiles put it — so
/// the submental surface has to bridge a gap grown by about twenty
/// millimetres over the same three rows of vertices, and it folds. In the
/// normal buffer that is a cliff with a torn edge; in the lit render it is the
/// jaw shattering.
///
/// A third of it, because a throat does come in a little under a narrower neck
/// and holding it rigid leaves a flat plate where the submental hollow should
/// keep curving.
///
/// Provenance: **tuned by render, against a fold**.
const THROAT_HOLD: f32 = 0.33;

/// How far the nape cuts in under the occiput, as a fraction of the column's
/// own radius at that height.
///
/// **The other half of what reads as a stump, and it is the BACK.** With no
/// undercut at all the occiput runs straight down into the column and the
/// silhouette from behind is one tube from crown to shoulders — the head has no
/// bottom. `OCCIPUT`'s own negative tail cuts this hollow where the head owns
/// the surface; below the head's floor nothing else does, and without this the
/// hollow stops dead at a zone boundary.
///
/// Provenance: **tuned by render**.
const NAPE_CUT: f32 = 0.14;

/// How far down the column the nape's cut has faded out, in the same fractions.
///
/// A nape is a hollow under the skull, not a groove down the whole neck: it has
/// to be gone well before the shoulders or the column reads pinched from behind
/// rather than undercut.
///
/// Provenance: **tuned by render**.
const NAPE_FADE: f32 = 0.55;

/// Where the column is measured, as a fraction of the way from the mandible's
/// border down to the girdle's crown.
///
/// The waist: far enough below the border that the jaw's own mass is out of the
/// section, and well above the shoulders. [`RAMP`] has the carve at full
/// strength by here, so this is the height whose width the carve actually sets
/// and the honest place to read what it has to work with.
///
/// Provenance: **measured on the built column** — the narrowest section
/// sits 80 mm below the head joint against a border at 55 and a girdle crown at
/// 210, which is about a sixth of the way down.
const WAIST: f32 = 0.18;

/// How far either side of the waist a vertex still counts, in the same
/// fractions.
///
/// Wide enough that a ring always lands inside it — the constraint any
/// height-window here must respect: the column's rings are about 8 mm apart before
/// refinement and a window narrower than that reports the tessellation. A tenth
/// of a run about 90 mm long is 9 mm either side.
const WINDOW: f32 = 0.10;

/// How many times the column's own faces are split before anything shapes it.
///
/// **A carve cannot draw a curve on a surface with no rows to hold it.**
/// `refine_face` rejects every face whose nearest
/// bone is not the head's, so without this pass the column arrives at the base
/// subdivision: the
/// midline throat measures as a POLYLINE with runs of exactly zero turn eleven
/// millimetres long, and the waist and the nape this module authors would be
/// drawn between rows that far apart — read on screen as the sides of the
/// neck and the throat simply not being smooth, which is what it is.
///
/// Runs BEFORE the carve and before `shape_skull`, for the reason `refine_face`
/// runs before shaping: splitting first samples the shape finely, and splitting
/// after subdivides the facets of a shape already drawn.
const REFINEMENT: usize = 1;

/// How far past the column's own span the refinement reaches, as a share of it.
///
/// A resolution boundary is a curvature spike wherever the surface is curved,
/// so the split has to finish somewhere the carve is not working.
/// Below, that is inside the shoulder, where the release has already returned
/// the surface to the cage's own; above, it is inside the head, whose faces are
/// refined eight times over by `refine_face` and cannot notice one more.
const MARGIN: f32 = 0.05;

/// Gives the column enough surface to carry the shape [`shape`] draws on it.
///
/// Does nothing to a body with no head or no neck, or to one that walks on four
/// legs — the same three cases the carve itself declines.
#[must_use]
pub fn refine(mesh: &PolyMesh, rig: &Rig, traits: &HeadTraits) -> PolyMesh {
    let Some(bounds) = span(rig, traits) else {
        return mesh.clone();
    };
    let (top, bottom, joint, radius) = bounds;
    let reach = (bottom - top) * MARGIN;
    let mut refined = mesh.clone();
    for _ in 0..REFINEMENT {
        let selected: Vec<bool> = (0..refined.face_count())
            .map(|face| {
                let at = refined.face_centroid(face);
                // **By DEPTH and not by zone, because the carve spans the
                // junction and a split that stopped at it would leave half the
                // curve unsupported** (#176). The first version took
                // `Zone::Neck` faces only, on the argument that the head above
                // is `refine_face`'s business — and the throat directly under
                // the jaw is head-owned, so it kept exactly the eleven
                // millimetre facets this exists to remove. It is not
                // double-paying either: that band falls below `FACE_PASSES`'
                // third region, whose floor is −0.714 profile heights, so the
                // throat gets two of the face's nine passes and nothing else
                // reaches it.
                //
                // The chest is excluded because the shoulder's own mass is the
                // girdle's business, and the head is excluded ABOVE the depth
                // its own passes reach — dropping that test entirely was
                // measured and it takes the default body from 27,182 triangles
                // to 59,916, because one more pass over a face already split
                // eight times is most of a body.
                let depth = (joint - at.y) / radius;
                match rig.joints[rig.nearest_bone(at).joint].zone {
                    Zone::Chest | Zone::Head => return false,
                    _ => {}
                }
                depth > top - reach && depth < bottom + reach
            })
            .collect();
        refined = refined.refine_curved(&selected);
    }

    // **The shoulder band, split LINEARLY, and the distinction is a streak
    // across the chest** (#302). The trapezius is drawn on chest-zone surface
    // the column's own pass excludes, and at the base subdivision the
    // shoulder's top is facets two to three centimetres wide — a slope drawn
    // across those is a polygon. But `refine_curved` lifts its new vertices
    // onto the tangent planes of the corners they come from, and along the
    // edge of a selection those lifted midpoints bulge out of the unrefined
    // neighbour's plane: the band's lower edge crossed the curved chest front
    // and rendered as a pair of diagonal creases under the throat. The fill
    // is the curvature this band is for, so the split only has to add
    // sampling, and a linear split's boundary is invisible by construction —
    // a midpoint on the chord is on the neighbour's edge already.
    if let Some(band) = shoulder_band(rig) {
        for _ in 0..REFINEMENT {
            let selected: Vec<bool> = (0..refined.face_count())
                .map(|face| {
                    let at = refined.face_centroid(face);
                    rig.joints[rig.nearest_bone(at).joint].zone == Zone::Chest && band(at)
                })
                .collect();
            refined = refined.refine(&selected);
        }
    }
    refined
}

/// The column's span, its head joint's height and the head's radius.
///
/// One definition, read by both [`refine`] and [`shape`], because a split that
/// covered a different run from the carve would put a resolution boundary in
/// the middle of the carve's own curvature — the one place it must not go.
fn span(rig: &Rig, traits: &HeadTraits) -> Option<(f32, f32, f32, f32)> {
    if rig.ground_contacts().len() > 2 {
        return None;
    }
    let &head = rig.in_zone(Zone::Head).first()?;
    let &neck = rig.in_zone(Zone::Neck).first()?;
    let parent = rig.joints[neck].parent?;
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return None;
    }
    let joint = rig.joints[head].position.y;
    let top = -border(rig, head, traits, 1.0) / radius;
    let bottom = (joint - rig.joints[parent].position.y - rig.joints[parent].radius) / radius;
    (bottom - top > f32::EPSILON).then_some((top, bottom, joint, radius))
}

/// Narrows the column between the jaw and the shoulders, in place.
///
/// Runs after [`super::shape_skull`], on the same mesh, and does nothing to a
/// body with no head or no neck — or to one that walks on four legs, for the
/// reason the skull's own shaping bails: this is a human column and a creature's
/// is its own shape.
pub fn shape(mesh: &mut PolyMesh, rig: &Rig, traits: &HeadTraits) {
    if rig.ground_contacts().len() > 2 {
        return;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
        return;
    };
    let Some(parent) = rig.joints[neck].parent else {
        return;
    };
    let radius = rig.joints[head].radius * rig.joints[head].scale.x;
    if radius <= f32::EPSILON {
        return;
    }

    // The column's own axis. A neck leans back, so the section it is carved
    // about is not the head's — measured off the neck joint, which is where the
    // plan put the lean.
    let axis = rig.joints[neck].position;
    let joint = rig.joints[head].position;

    // The frame every depth below is measured in: head radii under the head
    // joint, with no remap. It has to be azimuth-free, because the ceiling this
    // carve is bounded by is NOT — see [`super::skull::border`] — and a band
    // table indexed by a moving landmark is a table whose rows mean different
    // things at different azimuths.
    let depth = |point: Vec3| (joint.y - point.y) / rig.joints[head].radius;
    // Where the carve has finished: the girdle's crown, which is where the
    // shoulder mass starts and where this must already be identity.
    let bottom = depth(Vec3::new(
        0.0,
        rig.joints[parent].position.y + rig.joints[parent].radius,
        0.0,
    ));
    // Where it can begin: the HIGHEST the mandible's border ever gets, which is
    // out at the angle of the jaw. Nothing above this is ever touched at any
    // azimuth, so the band table can start here.
    let top = depth(Vec3::new(
        0.0,
        joint.y + border(rig, head, traits, 1.0),
        0.0,
    ));
    if bottom - top <= f32::EPSILON {
        return;
    }

    // How far round the column a point is, about the column's own axis.
    let round = |point: Vec3| {
        let across = Vec3::new(point.x - axis.x, 0.0, point.z - axis.z);
        let reach = across.length();
        if reach <= f32::EPSILON {
            return (0.0, 0.0, 0.0);
        }
        (
            across.z / reach,
            (across.x / reach).abs(),
            (-across.z / reach).max(0.0),
        )
    };

    // What the sweep produced, band by band: the widest the section gets, taken
    // as the largest lateral offset in the band. A maximum over a band is the
    // right statistic for an envelope and the wrong one for a profile — see
    // `Skull::measure`, which bins the same way and says so — and the carve
    // wants the envelope.
    let mine = owned(mesh, rig);
    let want = NECK_SKULL * SKULL_OF_NODE * radius;
    let run = bottom - top;

    // **How much this column has to come in, as ONE number** (#176). The first
    // version measured a band table — the widest lateral offset in each of
    // twenty-four slices — and scaled every height toward the target by its own
    // band. It read as noise, because a maximum over a band of a coarse quad
    // tube reports where the RINGS fall and not where the surface is: measured
    // on the default body, adjacent 3.7 mm bands came back 52, 60, 43, 53, 57,
    // 59 mm and two of the twenty-four held no vertex at all. `Skull::measure`
    // and `examples/headaudit` both carry the same warning in their own words,
    // and this divided by it, so every one of those swings became a step in the
    // surface.
    //
    // One window, one number, no table. The column's width varies slowly enough
    // over the waist that a single reading describes it, and a factor built
    // from one number is smooth in height by construction rather than by being
    // filtered afterwards.
    let waist = top + run * WAIST;
    let at_waist = mesh
        .positions
        .iter()
        .zip(&mine)
        .filter(|(point, mine)| **mine && (depth(**point) - waist).abs() < run * WINDOW)
        .fold(0.0f32, |wide, (point, _)| {
            wide.max((point.x - axis.x).abs())
        });
    // Only ever narrower. A column already inside the target is a small head on
    // a slender neck, and inflating it to meet a ratio would be this module
    // causing the defect it exists to remove.
    let narrow = 1.0 - (want / at_waist.max(f32::EPSILON)).min(1.0);
    if narrow <= 0.0 {
        return;
    }

    for (point, mine) in mesh.positions.iter_mut().zip(&mine) {
        if !*mine {
            continue;
        }
        let at = depth(*point);
        if at < top || at > bottom {
            continue;
        }
        let (facing, _, behind) = round(*point);
        // Below this point's OWN border, and nothing above it moves. The ramp
        // is a share of the whole run rather than a fixed depth, so a long neck
        // eases in over a longer distance and a short one does not have the
        // carve arrive as a step.
        let under = at
            - depth(Vec3::new(
                0.0,
                joint.y + border(rig, head, traits, border_raise(facing)),
                0.0,
            ));
        // In below its own border, held through the waist, and out again into
        // the shoulders. Both edges are smoothsteps, so the carve arrives and
        // leaves with zero slope and puts no curvature spike at either end.
        let hold = smooth(under / (run * RAMP)) * smooth((bottom - at) / (run * (1.0 - RELEASE)));
        if hold <= 0.0 {
            continue;
        }
        let factor = 1.0 - narrow * hold;

        // **The narrowing is LATERAL, and the throat is nearly held** (#175).
        // Scaling the whole section about the column's axis drew the throat
        // back with the sides while the chin in front of it did not move, and
        // the submental surface had to bridge a gap that had grown by twenty
        // millimetres over the same three rows: it folded, and rendered as a
        // cliff with a torn edge under the jaw. That is also the honest reading
        // of what `have` measured, which is a lateral half-width and not a
        // radius. A throat's fore-aft line is the larynx's and the jaw's
        // business; what is wrong with this column is how WIDE it is.
        let ahead = facing.max(0.0);
        let across = point.x - axis.x;
        let fore = point.z - axis.z;
        // The nape, which is the same carve asked of the back alone. Squared in
        // the aft cosine like `OCCIPUT`'s own tail, so it is nothing at the
        // throat and full behind — a hollow under the skull rather than a
        // groove down the neck.
        let cut = NAPE_CUT
            * behind
            * behind
            * smooth(under / (run * RAMP))
            * smooth((run * NAPE_FADE - under) / (run * NAPE_FADE));
        point.x = axis.x + across * factor * (1.0 - cut);
        // Fore and aft the carve is held back by how far forward the point is,
        // so the back of the column follows the sides in and the throat very
        // nearly stays. `THROAT_HOLD` is what is left of it dead ahead.
        let keep = 1.0 - (1.0 - THROAT_HOLD) * ahead * ahead;
        point.z = axis.z + fore * (1.0 + (factor * (1.0 - cut) - 1.0) * keep);
    }
}

/// How many fairing sweeps the column gets, and the pair each sweep is.
///
/// Taubin's smooth-then-unshrink pair, the same one `dress::garment` fairs a
/// hem with: the positive pass pulls each vertex toward its neighbours' mean
/// and the larger negative one restores the low frequencies, so what dies is
/// the crease and not the column's own girth.
const FAIR_PASSES: usize = 48;
const FAIR_SMOOTH: f32 = 0.5;
const FAIR_UNSHRINK: f32 = -0.53;

/// How much of the run the fairing takes to arrive below the border, and to
/// leave above the girdle, as shares of the whole column.
///
/// Tighter above than the carve's own [`RAMP`], because the fairing must not
/// reach the chin: the border is the boundary of the FACE's identity, and a
/// fairing weight that is still nonzero there planes the chin button flat.
const FAIR_IN: f32 = 0.14;

/// How far ABOVE its own border a point may still be faired, as a share of the
/// run — see the weight's comment: the border is a seam between two shaping
/// systems, and the seam is the thing being removed.
///
/// **A fiftieth, and the difference between a fiftieth and a twentieth is a
/// POINTED CHIN.** At 0.05 the fairing's unshrinking pass reaches the
/// chin's own crest and pushes it forward: measured on `examples/column`, the
/// midline crest stands 105.3 mm against the untouched body's 102.3, and the
/// render reads as a chin gone too pointy. At 0.02 the crest is
/// 102.3 again — bit for bit the untouched shape — and the jawline
/// streaks this constant exists to remove stay gone, which is the whole reason
/// it survives at all rather than going to zero.
const FAIR_OVER: f32 = 0.02;

/// [`FAIR_IN`], dead ahead of the column — see the weight's comment: under the
/// chin there is no edge for a gentle ramp to protect, and the gentleness is
/// what lets a wattle's drip survive.
const FAIR_IN_AHEAD: f32 = 0.03;

/// How far BELOW the girdle's crown the fairing still holds, dead ahead, as a
/// share of the run — see the weight's comment: the crown is a height, the
/// shoulder mass it protects is at the sides, and the drip hangs under it.
const FAIR_BELOW: f32 = 0.30;

/// The wattle pocket's shrinking passes, and how far below the border the
/// pocket reaches — see the melt's comment: a smooth prow sits inside Taubin's
/// pass-band, and only a pass that shrinks can take it down.
const MELT_PASSES: usize = 10;
const MELT_REACH: f32 = 0.30;

/// How far ABOVE the menton the melt still reaches, dead ahead, as a share of
/// the run. The chin's underside comes to a midline point — visible as a drip
/// once the wattle around it is faired away, and present under the wattle all
/// along — and the point's tip is the few millimetres of surface just above
/// the border. The chin's forward crest sits two to three centimetres higher
/// and the melt's weight is long dead there, which is what keeps this from
/// being a chin re-shape in disguise; the render across the sweep and the
/// chin's own test battery both hold it to that.
const MELT_OVER: f32 = 0.03;
const FAIR_OUT: f32 = 0.20;

/// Where the laryngeal prominence sits, as a fraction of the border-to-girdle
/// run, and how far it spreads in the same fractions.
///
/// Provenance: **tuned by render** — the thyroid notch has no landmark
/// on this rig to derive from.
const LARYNX_AT: f32 = 0.36;
const LARYNX_SPREAD: f32 = 0.12;

/// How far across the throat the prominence reaches, in head radii.
const LARYNX_WIDTH: f32 = 0.09;

/// Fairs the column into one smooth surface, then raises the larynx on it.
///
/// **This is where the mandible-to-throat skin gets its shape, and it is a
/// fairing rather than another term.** Three
/// systems share this band — `CHIN`'s tail, `construct_submental`'s six
/// column-chords and [`shape`]'s lateral narrowing — and each is individually
/// defensible while their seams are not: rendered unfaired, the
/// region carries a hanging tab under the chin, a crevasse up one side of the
/// throat and shelf-breaks in profile, every one of them a boundary between two
/// of those systems. Tuning three systems into agreement knot by
/// knot fails by moving each seam rather
/// than closing it — so the seams are faired out of the finished surface, which
/// is the operation "smooth, nicely curved" actually names.
///
/// The larynx is raised AFTER the fairing, on the faired surface, so the
/// smoothing cannot eat it — the same ordering reason the hem smooths before
/// the rim is built. Its amplitude is [`HeadTraits::larynx`], the frame axis's:
/// prominent at the masculine end, absent at the feminine.
///
/// Runs after [`shape`], does nothing to the bodies [`shape`] declines, and
/// moves vertices without adding any — the budget cannot see it.
pub fn fair(mesh: &mut PolyMesh, rig: &Rig, traits: &HeadTraits) {
    if rig.ground_contacts().len() > 2 {
        return;
    }
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
        return;
    };
    let Some(parent) = rig.joints[neck].parent else {
        return;
    };
    let radius = rig.joints[head].radius;
    if radius <= f32::EPSILON {
        return;
    }
    let joint = rig.joints[head].position;
    let axis = rig.joints[neck].position;
    let depth = |point: Vec3| (joint.y - point.y) / radius;
    let bottom = depth(Vec3::new(
        0.0,
        rig.joints[parent].position.y + rig.joints[parent].radius,
        0.0,
    ));
    let top = depth(Vec3::new(
        0.0,
        joint.y + border(rig, head, traits, 1.0),
        0.0,
    ));
    let run = bottom - top;
    if run <= f32::EPSILON {
        return;
    }

    // Each vertex's weight: zero at its own border and at the girdle, one
    // through the column. Computed once — the weights are a property of where
    // a vertex STARTED, and re-deriving them from positions the fairing is
    // itself moving would let the band creep.
    let mine = owned(mesh, rig);
    let weight: Vec<f32> = mesh
        .positions
        .iter()
        .zip(&mine)
        .map(|(&point, &mine)| {
            if !mine {
                return 0.0;
            }
            let at = depth(point);
            // The clip matches the widened window below, not the raw span — a
            // clip at `bottom` silently zeroed everything the front's lowered
            // release was written to reach, which is exactly where the drip
            // hung (the probe read the smoothsteps at 1.000 and the stored
            // weight at 0.000, and this line was the difference).
            if at < top - run * FAIR_OVER || at > bottom + run * FAIR_BELOW {
                return 0.0;
            }
            let across = Vec3::new(point.x - axis.x, 0.0, point.z - axis.z);
            let reach = across.length();
            // Dead-on-axis counts as dead ahead: a raise of 0 keeps the ramp
            // anchored at the menton there, exactly as `side.max(behind)` did.
            let facing = if reach <= f32::EPSILON {
                1.0
            } else {
                across.z / reach
            };
            let under = at
                - depth(Vec3::new(
                    0.0,
                    joint.y + border(rig, head, traits, border_raise(facing)),
                    0.0,
                ));
            // The ramp starts a little ABOVE each point's own border, because
            // the border itself is a seam: `shape_skull` owns the surface up
            // to it and this band below, and a weight that is exactly zero
            // there preserves every artefact the two systems disagree by,
            // which rendered as streaks along the jawline. The head above the
            // ramp's start is untouched, and the weight where the chin's own
            // curvature lives is a few percent — shading noise dies, the
            // jawline's large-scale shape cannot.
            //
            // **And it arrives fastest dead ahead.** The gentle ramp exists to
            // protect an edge — the mandible line the profile view is judged
            // by — and dead ahead under the chin there is no edge to protect:
            // the chin's crest is above the border, and what lives just below
            // it on the midline was the hanging wattle's last remnant, a
            // pointed drip that survived every pass at ramp-zone weight. The
            // ahead cosine collapses the ramp there and leaves it whole at the
            // sides.
            let ahead = if reach <= f32::EPSILON {
                0.0
            } else {
                (across.z / reach).max(0.0)
            };
            let arrive = FAIR_IN - (FAIR_IN - FAIR_IN_AHEAD) * ahead * ahead;
            // And the release lets go LOWER dead ahead, for the mirrored
            // reason: the release protects the girdle's own mass, which is the
            // shoulders — at the sides. Dead ahead at the crown's height there
            // is only the pit of the throat, and the wattle's drip hung BELOW
            // the crown on a short masculine neck, which put it in this fade's
            // dead zone: probed at weight 0.000 while every knob above it read
            // 0.3 to 0.8, which is why it survived forty-eight passes intact.
            let release = bottom + run * FAIR_BELOW * ahead * ahead;
            smooth((under + run * FAIR_OVER) / (run * (arrive + FAIR_OVER)))
                * smooth((release - at) / (run * FAIR_OUT))
        })
        .collect();
    if weight.iter().all(|&w| w <= 0.0) {
        return;
    }

    let around = neighbours(mesh);
    taubin(mesh, &around, &weight, FAIR_PASSES);

    // **The wattle's pocket, melted rather than faired** — and the distinction
    // is the whole finding. The drip under the chin survived forty-eight
    // Taubin passes AT FULL WEIGHT (probed: weight 0.998–1.000 on its own
    // vertices), because it is not a crease: it is a smooth rounded prow, and
    // smooth-then-unshrink preserves smooth shapes by construction — that is
    // its pass-band, and the reason it is safe to run over the whole column.
    // Removing a smooth protrusion needs the shrinking Laplacian that Taubin
    // exists to avoid, so the pocket dead ahead and just below the border gets
    // a few passes of it, confined by the same azimuth logic as everything
    // else here: `ahead²` keeps it off the jawline, the ramp keeps it off the
    // chin above and the throat below.
    let pocket: Vec<f32> = mesh
        .positions
        .iter()
        .zip(&mine)
        .map(|(&point, &mine)| {
            if !mine {
                return 0.0;
            }
            let at = depth(point);
            let across = Vec3::new(point.x - axis.x, 0.0, point.z - axis.z);
            let reach = across.length();
            if reach <= f32::EPSILON {
                return 0.0;
            }
            let ahead = (across.z / reach).max(0.0);
            let under = at
                - depth(Vec3::new(
                    0.0,
                    joint.y + border(rig, head, traits, border_raise(ahead)),
                    0.0,
                ));
            ahead
                * ahead
                * smooth((under + run * MELT_OVER) / (run * (0.04 + MELT_OVER)))
                * smooth((run * MELT_REACH - under) / (run * 0.12))
        })
        .collect();
    if pocket.iter().any(|&w| w > 0.0) {
        for _ in 0..MELT_PASSES {
            relax(mesh, &around, &pocket, FAIR_SMOOTH);
        }
    }

    // The larynx, on the faired surface. A bump in height and in breadth,
    // pushed dead forward: the throat's own normal leans with the column and
    // pushing along it would smear the prominence up the underside of the jaw.
    let stand = traits.larynx * radius;
    if stand <= 0.0 {
        return;
    }
    for (vertex, point) in mesh.positions.iter_mut().enumerate() {
        if weight[vertex] <= 0.0 || point.z <= axis.z {
            continue;
        }
        let at = (depth(*point) - top) / run;
        let tall = ((at - LARYNX_AT) / LARYNX_SPREAD).powi(2);
        let wide = ((point.x - axis.x) / (LARYNX_WIDTH * radius)).powi(2);
        point.z += stand * (-(tall + wide)).exp();
    }
}

/// How far the trapezius fill stands off the crease between the column and
/// the shoulder, in GIRDLE radii, on the neutral body.
///
/// Girdle radii rather than head radii, because the girdle already carries
/// the body's mass and frame through the allometric girth — a heavy body's
/// fill is bigger in millimetres for the same constant — and
/// [`HeadTraits::trapezius`] puts the frame axis on top of that.
///
/// Provenance: **tuned by render**, on `--close throat` across the frame axis.
const TRAPEZIUS_RISE: f32 = 0.18;

/// How far UP the column the fill still reaches above the crease, as a share
/// of the column's own run from the mandible's border to the girdle's crown,
/// and how far BELOW the shoulder's own top line it reaches, in girdle radii.
///
/// Up: the fill flares the base of the column into the shoulder, and the
/// angle of that flare is the fill's height over this reach — a short reach
/// is a lip, which is the collar #301 removed coming back. **A share of the
/// run and not of the column's radius**, for the reason [`RAMP`] is: measured
/// in radii it reached 1.2 of them, which on a small head is past the
/// column's waist, and `the_neck_is_the_width_of_a_neck_on_every_head_it_carries`
/// read the waist 0.872 of the skull against a ceiling of 0.83 — the fill was
/// widening the one section this module exists to narrow. The waist sits at
/// [`WAIST`] of the run below the border, so anything under `1 − WAIST` of
/// the run above the crown leaves it alone; this stops well short of that.
/// Down: the shoulder's top surface is what the fill raises, and the surface
/// a girdle radius under it is the chest and the back, which it must not.
const TRAPEZIUS_UP: f32 = 0.6;
const TRAPEZIUS_DOWN: f32 = 0.6;

/// How far OUT the fill carries for every unit it carries up.
///
/// The fill moves every vertex along one diagonal — up and away from the
/// column's axis — rather than along its own normal. **Along the normals it
/// folds**: at a concave crease the wall's normal is outward and the top's is
/// up, so a wall vertex just above the crease moves further out than the
/// crease vertex, which moves diagonally, and the surface crosses itself.
/// That rendered as a lip with a dark notch under it, the collar back again.
/// One direction for the whole field cannot fold while the field is smooth.
///
/// **0.7, not 1.0**: at a full unit the masculine column's base widened by
/// the whole stand and read as a collar again from the front; most of a
/// trapezius's slope is rise, not spread.
const TRAPEZIUS_FLARE: f32 = 0.7;

/// How far BEHIND the column's axis the fill still reaches, in column depths,
/// fading from full at the column's back to nothing here.
///
/// Without it the fill was full over the whole upper back below the crown,
/// and at the masculine end pushed it back as one slab with a ledge under
/// the nape. The trapezius's mass is on the shoulders and the column; down
/// the back it is a sheet, not a step.
const TRAPEZIUS_AFT: f32 = 2.5;

/// The band of shoulder the trapezius fill reaches, as a test on a point,
/// read by [`refine`] so the fill lands on surface fine enough to hold it.
///
/// `None` on a body with no girdle or no acromion, which is a body the fill
/// itself declines.
fn shoulder_band(rig: &Rig) -> Option<impl Fn(Vec3) -> bool> {
    let (column, girdle, reach, _) = shoulder(rig)?;
    let crown = girdle.position.y + girdle.radius;
    let floor = crown - girdle.radius * TRAPEZIUS_DOWN;
    let aft = girdle.position.z - girdle.radius * girdle.scale.y;
    let fore = column.position.z + column.radius * column.scale.y;
    Some(move |at: Vec3| at.y > floor && at.x.abs() < reach && at.z > aft && at.z < fore)
}

/// The column, the girdle it stands on, and the acromion's lateral reach and
/// top height — the landmarks the trapezius fill is drawn between.
///
/// The acromion is the girdle's lateral child, which is where the shoulder's
/// top surface ends and the arm begins. A body with no such child has no
/// shoulder to slope, and gets `None`.
fn shoulder(rig: &Rig) -> Option<(&crate::rig::Joint, &crate::rig::Joint, f32, f32)> {
    if rig.ground_contacts().len() > 2 {
        return None;
    }
    let &neck = rig.in_zone(Zone::Neck).first()?;
    let parent = rig.joints[neck].parent?;
    let girdle = &rig.joints[parent];
    let (reach, acromion) = rig
        .joints
        .iter()
        .enumerate()
        .filter(|(index, joint)| *index != neck && joint.parent == Some(parent))
        .map(|(_, joint)| (joint.position.x.abs(), joint.position.y + joint.radius))
        .filter(|(reach, _)| *reach > girdle.radius * 0.5)
        .max_by(|a, b| a.0.total_cmp(&b.0))?;
    Some((&rig.joints[neck], girdle, reach, acromion))
}

/// Fills the crease between the column and the shoulder with a trapezius.
///
/// **The shoulder line ran nearly horizontal and met the column at a hard
/// crease on every body** (#302). A trapezius is the slope from the base of
/// the neck out to the acromion, and nothing in the cage can say it: the
/// module docs on `plan::derive::humanoid` record six refuted constructions
/// — the girdle cannot carry a fifth socket and the neck cannot carry two —
/// so the mass has to be a carve on the surface, like the chest's and the
/// jaw's.
///
/// **It lives here and not in `torso`, and that is the ownership decision.**
/// The trapezius band is chest-zone surface, but `owned` already claims the
/// chest for the column's release, the frame every junction measurement is
/// taken in — the column's axis, the girdle's crown — is this module's, and
/// #301 has just closed a seam between two systems that shared one band of
/// skin. A `carve_shoulders` beside `carve_chest` would have been the next
/// one.
///
/// The fill is a crescent: full at the crease, fading up the column over
/// `TRAPEZIUS_UP`, fading out along the shoulder's top to nothing at the
/// acromion, and fading to nothing toward the front of the column so the
/// pit of the throat and the clavicles keep their hollow. Every vertex moves
/// up and outward along one diagonal (`TRAPEZIUS_FLARE`), so the column's
/// base flares into the shoulder as the shoulder's top rises toward it and
/// the right angle between them becomes two gentle turns. The raise falls
/// off as the square of the distance to the acromion rather than as a
/// smoothstep: a zero slope at the crease is a shelf, and a shelf's edge is
/// a lip.
///
/// Runs last of the column's passes, after [`shape`]: the fill is smooth by
/// construction and the fairing before it would only relax it. Moves
/// vertices without adding any.
pub fn trapezius(mesh: &mut PolyMesh, rig: &Rig, traits: &HeadTraits) {
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some((column, girdle, reach_x, acromion)) = shoulder(rig) else {
        return;
    };
    let axis = column.position;
    let crown = girdle.position.y + girdle.radius;
    // The column's run, border to crown, in metres: the ruler the flare's
    // reach is a share of.
    let run = rig.joints[head].position.y + border(rig, head, traits, 1.0) - crown;
    if run <= f32::EPSILON {
        return;
    }
    let base = column.radius * column.scale.x;
    if reach_x - base <= f32::EPSILON {
        return;
    }
    let stand = girdle.radius * TRAPEZIUS_RISE * traits.trapezius;
    if stand <= 0.0 {
        return;
    }
    let up = run * TRAPEZIUS_UP;
    let down = girdle.radius * TRAPEZIUS_DOWN;
    let front = column.radius * column.scale.y;

    let mine = owned(mesh, rig);
    for (vertex, point) in mesh.positions.iter_mut().enumerate() {
        if !mine[vertex] {
            continue;
        }
        // Out along the shoulder: full over the column, nothing at the
        // acromion, leaving with zero slope and arriving with one.
        let across = (point.x - axis.x).abs();
        let t = ((across - base) / (reach_x - base)).clamp(0.0, 1.0);
        let out = (1.0 - t) * (1.0 - t);
        // Up and down about the shoulder's own top line, which runs from the
        // crease at the crown to the acromion's top.
        let top = crown + (acromion - crown) * t;
        let rise = point.y - top;
        let tall = if rise >= 0.0 {
            smooth((up - rise) / up)
        } else {
            smooth((down + rise) / down)
        };
        // Fore and aft: full a column's depth behind the axis, gone at the
        // column's front, so the pit of the throat and the clavicles keep
        // their hollow. Over two depths rather than one: the fill's front
        // edge is a line across the base of the throat, and over one depth
        // the line was a crease.
        let ahead = point.z - axis.z;
        let fore = smooth((front - ahead) / (2.0 * front));
        let aft = smooth((TRAPEZIUS_AFT * front + ahead) / ((TRAPEZIUS_AFT - 1.0) * front));
        let w = out * tall * fore * aft;
        if w <= 0.0 {
            continue;
        }
        // **Outward is RADIAL about the column's axis, not lateral.** A plain
        // sign of `x` flips at the spine, so two vertices either side of the
        // nape's midline were pushed APART — a pair of lumps flanking a groove
        // down the back of the neck; and a lateral push that faded to nothing
        // on the midline folded the flank where it changed direction. The
        // radial direction is continuous all the way round, so the nape moves
        // back and up — which is the slope a trapezius gives the rear line of
        // a neck — and the flanks move out and up.
        let across = Vec3::new(point.x - axis.x, 0.0, point.z - axis.z);
        let outward = across.try_normalize().unwrap_or(Vec3::ZERO);
        *point += (outward * TRAPEZIUS_FLARE + Vec3::Y) * (stand * w);
    }

    // **And then the junction is faired, because the fill is mass and the
    // stump is a seam** (#302, the owner's second read). The column's back is
    // a vertical tube and the girdle hull's back facet leaves it at a hard
    // ring — a ledge on every body, and a step the size of the gap between
    // the column's back and the upper back on a heavy masculine one. A fill
    // that raises the nape above that ring and stops at it sharpens the
    // ledge; a term that targets the ledge is the knot-by-knot negotiation
    // the fairing's own docstring records failing. So the band from the
    // flare's top down into the upper back is faired the way the column is,
    // all the way round but the throat, and the ledge settles into the slope
    // the fill put there. Taubin keeps the fill's mass — it is low frequency
    // — and takes the ring, which is not.
    let settle: Vec<f32> = mesh
        .positions
        .iter()
        .zip(&mine)
        .map(|(&point, &mine)| {
            if !mine {
                return 0.0;
            }
            let across = Vec3::new(point.x - axis.x, 0.0, point.z - axis.z);
            let reach = across.length();
            let ahead = if reach <= f32::EPSILON {
                0.0
            } else {
                (across.z / reach).max(0.0)
            };
            let t = ((across.x.abs() - base) / (reach_x - base)).clamp(0.0, 1.0);
            let rise = point.y - (crown + (acromion - crown) * t);
            let band = if rise >= 0.0 {
                smooth((up - rise) / (up * 0.5))
            } else {
                smooth((down + rise) / (down * 0.5))
            };
            band * (1.0 - ahead * ahead) * (1.0 - t)
        })
        .collect();
    if settle.iter().any(|&w| w > 0.0) {
        let around = neighbours(mesh);
        taubin(mesh, &around, &settle, SETTLE_PASSES);
    }
}

/// How many Taubin pairs the junction band gets after the fill.
///
/// Fewer than the column's [`FAIR_PASSES`]: the ring at the column's foot is
/// two or three rows of mesh, and a pass count that would plane a chin is
/// more than a ring needs.
const SETTLE_PASSES: usize = 24;

/// The neighbour graph, from the faces.
///
/// Every vertex takes part, banded or not: vertices outside a fairing band
/// serve as anchors, and a mean over neighbours that ignored the fixed
/// surface beside a band would let the band's edge drift away from the
/// surface it must meet.
fn neighbours(mesh: &PolyMesh) -> Vec<Vec<u32>> {
    let mut around: Vec<Vec<u32>> = vec![Vec::new(); mesh.vertex_count()];
    for face in &mesh.faces {
        for at in 0..face.len() {
            let (a, b) = (face[at], face[(at + 1) % face.len()]);
            around[a as usize].push(b);
            around[b as usize].push(a);
        }
    }
    around
}

/// One Laplacian step: each weighted vertex moves toward its neighbours'
/// mean by `step` times its weight. Positive shrinks, negative unshrinks.
fn relax(mesh: &mut PolyMesh, around: &[Vec<u32>], weight: &[f32], step: f32) {
    let was = mesh.positions.clone();
    for (vertex, point) in mesh.positions.iter_mut().enumerate() {
        let w = weight[vertex];
        if w <= 0.0 || around[vertex].is_empty() {
            continue;
        }
        let mean = around[vertex]
            .iter()
            .fold(Vec3::ZERO, |sum, &other| sum + was[other as usize])
            / around[vertex].len() as f32;
        *point += (mean - *point) * (step * w);
    }
}

/// Taubin's smooth-then-unshrink pairs, `passes` of them, over a weighted band.
fn taubin(mesh: &mut PolyMesh, around: &[Vec<u32>], weight: &[f32], passes: usize) {
    for _ in 0..passes {
        relax(mesh, around, weight, FAIR_SMOOTH);
        relax(mesh, around, weight, FAIR_UNSHRINK);
    }
}

/// Which vertices the column may move.
///
/// Zone rather than height, and then height on top of it: an arm crosses these
/// heights on a T-posed body and a shoulder is not a neck. The chest is in the
/// set because the girdle's own crown is chest-owned and the release has to
/// reach it, and it is safe because [`RELEASE`] has already returned the carve
/// to identity by the time the surface gets there.
fn owned(mesh: &PolyMesh, rig: &Rig) -> Vec<bool> {
    mesh.positions
        .iter()
        .map(|&point| {
            matches!(
                rig.joints[rig.nearest_bone(point).joint].zone,
                Zone::Head | Zone::Neck | Zone::Chest
            )
        })
        .collect()
}
