//! The column between the jaw and the shoulders, carved rather than swept.
//!
//! **The head used to sit on a stump, and the stump was the cage's** (#175). A
//! neck arrives from [`crate::cage`] as a swept ring of its own radius, and that
//! radius is a fraction of STATURE times girth times the frame axis, while the
//! skull above it is a fraction of stature times `head_size`. The two share
//! stature and nothing else, so a body could carry either on the other:
//! measured over an ordinary grid before this existed, the built column ran from
//! 0.65 of the skull's own width to 1.21 of it — at `head_size` −1 the neck was
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

use crate::mesh::PolyMesh;
use crate::rig::Rig;
use crate::{Vec3, Zone};

use super::skull::{HeadTraits, border};
use super::smooth;

/// What a neck is worth against the skull it carries, as a fraction of the
/// built skull's own widest half-width.
///
/// **Sourced from one mannequin and corroborated from outside it, and the
/// caveat travels with the number** (#175). The obvious derivation — measure
/// both CC0 references, as `HeadTraits`'s whole set is measured — does not
/// work here, and four instruments were built and thrown away proving it: the
/// male is 7,399 vertices so a fine band holds one ring or none; a ray from the
/// skull's axis is not a half-width once it leaves the skull, because a neck
/// stands behind that axis; a swept section at neck height contains the
/// trapezius as well as the neck. The fourth worked on the male and read 0.716,
/// and the female's `neck_01` weights own so much shoulder that her column
/// measures WIDER below the jaw than at it. No threshold makes the two files
/// the same selection.
///
/// 0.716 is corroborated from anthropometry rather than from the other
/// mannequin: neck breadth against head breadth is about 110 mm against 152,
/// which is 0.72. Two independent routes to the same figure on a quantity where
/// our own neutral body read 0.90 is enough to author against.
///
/// Provenance: **measured on the male reference mannequin, corroborated by
/// anthropometry; the female mannequin cannot carry the measurement** (#175).
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
/// Provenance: **measured over the parameter grid** (#175).
const SKULL_OF_NODE: f32 = 0.781;

/// How much of the run the carve takes to arrive, below each point's own
/// border, as a fraction of the whole column.
///
/// **A share of the run and not a fixed depth**, so a long neck eases in over a
/// longer distance and a short one does not take the carve as a step. Measured
/// on the built column before this existed, the narrowest section sat 80 mm
/// below the head joint against a border at 55 and a girdle crown at 210, so
/// the waist is about a sixth of the way down and the ramp has to be shorter
/// than that or the neck is at its narrowest nowhere.
///
/// Provenance: **measured on the built column** (#175).
const RAMP: f32 = 0.16;

/// Where the shoulders take the column back, in the same fractions.
///
/// The neck holds its own width from the end of [`RAMP`] down to here and then
/// lets go, so the surface between them cannot swell — which is what it did:
/// `neckaudit` counted three turns down the column, swell then pinch then
/// swell, where a neck narrows into its shoulder and stops. It counts one now.
///
/// Provenance: **tuned by render** (#175).
const RELEASE: f32 = 0.72;

/// How much of the carve still reaches the throat, dead ahead.
///
/// **Zero would be a neck that narrows sideways and nowhere else, and one tears
/// the jaw off** (#175). The first version scaled the whole section about the
/// column's axis, which drew the throat back with the sides while the chin in
/// front of it stayed exactly where the skull's own profiles had put it — so
/// the submental surface had to bridge a gap that had grown by about twenty
/// millimetres over the same three rows of vertices, and it folded. In the
/// normal buffer it is a cliff with a torn edge; in the lit render it is the
/// jaw shattering.
///
/// A third of it, because a throat does come in a little under a narrower neck
/// and holding it rigid leaves a flat plate where the submental hollow should
/// keep curving.
///
/// Provenance: **tuned by render, against a fold** (#175).
const THROAT_HOLD: f32 = 0.33;

/// How far the nape cuts in under the occiput, as a fraction of the column's
/// own radius at that height.
///
/// **The other half of what reads as a stump, and it is the BACK.** With no
/// undercut at all the occiput runs straight down into the column and the
/// silhouette from behind is one tube from crown to shoulders — the head has no
/// bottom. `OCCIPUT`'s own negative tail cuts this hollow where the head owns
/// the surface; below the head's floor nothing did, so the hollow stopped dead
/// at a zone boundary.
///
/// Provenance: **tuned by render** (#175).
const NAPE_CUT: f32 = 0.14;

/// How far down the column the nape's cut has faded out, in the same fractions.
///
/// A nape is a hollow under the skull, not a groove down the whole neck: it has
/// to be gone well before the shoulders or the column reads pinched from behind
/// rather than undercut.
///
/// Provenance: **tuned by render** (#175).
const NAPE_FADE: f32 = 0.55;

/// Where the column is measured, as a fraction of the way from the mandible's
/// border down to the girdle's crown.
///
/// The waist: far enough below the border that the jaw's own mass is out of the
/// section, and well above the shoulders. [`RAMP`] has the carve at full
/// strength by here, so this is the height whose width the carve actually sets
/// and the honest place to read what it has to work with.
///
/// Provenance: **measured on the built column** (#175) — the narrowest section
/// sat 80 mm below the head joint against a border at 55 and a girdle crown at
/// 210, which is about a sixth of the way down.
const WAIST: f32 = 0.18;

/// How far either side of the waist a vertex still counts, in the same
/// fractions.
///
/// Wide enough that a ring always lands inside it, which is the whole lesson of
/// the band table this replaced: the column's rings are about 8 mm apart before
/// refinement and a window narrower than that reports the tessellation. A tenth
/// of a run about 90 mm long is 9 mm either side.
const WINDOW: f32 = 0.10;

/// How many times the column's own faces are split before anything shapes it.
///
/// **The neck had never been refined at all, and a carve cannot draw a curve
/// on a surface with no rows to hold it** (#176, and it is #158's lesson at the
/// other end of the same body). `refine_face` rejects every face whose nearest
/// bone is not the head's, so the column arrived at the base subdivision: the
/// midline throat measured as a POLYLINE with runs of exactly zero turn eleven
/// millimetres long, and the waist and the nape this module authors were being
/// drawn between rows that far apart. The owner read it as the sides of the
/// neck and the throat not being smooth, which is what it was.
///
/// Runs BEFORE the carve and before `shape_skull`, for the reason `refine_face`
/// runs before shaping: splitting first samples the shape finely, and splitting
/// after subdivides the facets of a shape already drawn.
const REFINEMENT: usize = 1;

/// How far past the column's own span the refinement reaches, as a share of it.
///
/// A resolution boundary is a curvature spike wherever the surface is curved
/// (#158), so the split has to finish somewhere the carve is not working.
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
    refined
}

/// The column's span, its head joint's height and the head's radius.
///
/// One definition, read by both [`refine`] and [`shape`], because a split that
/// covered a different run from the carve would put a resolution boundary in
/// the middle of the carve's own curvature — which is the one place #158 says
/// it must not go.
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
        let (facing, side, behind) = round(*point);
        // Below this point's OWN border, and nothing above it moves. The ramp
        // is a share of the whole run rather than a fixed depth, so a long neck
        // eases in over a longer distance and a short one does not have the
        // carve arrive as a step.
        let under = at
            - depth(Vec3::new(
                0.0,
                joint.y + border(rig, head, traits, side.max(behind)),
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
