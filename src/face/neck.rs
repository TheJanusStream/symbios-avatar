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

use super::skull::{Dimorphism, border};
use super::smooth;

/// What a neck is worth against the skull it carries, as a fraction of the
/// built skull's own widest half-width.
///
/// **Sourced from one mannequin and corroborated from outside it, and the
/// caveat travels with the number** (#175). The obvious derivation — measure
/// both CC0 references, as `Dimorphism`'s whole set is measured — does not
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

/// How many bands the built column is measured in before it is carved.
///
/// The carve sets a WIDTH, so it needs the width the sweep actually produced,
/// and that is cheaper to measure than to predict: one pass over the vertices
/// against a Catmull-Clark surface whose shrink nobody has an expression for.
/// Twenty-four bands over a column about 150 mm long is a band every 6 mm,
/// which is finer than the rings the cage puts there.
const BANDS: usize = 24;

/// Narrows the column between the jaw and the shoulders, in place.
///
/// Runs after [`super::shape_skull`], on the same mesh, and does nothing to a
/// body with no head or no neck — or to one that walks on four legs, for the
/// reason the skull's own shaping bails: this is a human column and a creature's
/// is its own shape.
pub fn shape(mesh: &mut PolyMesh, rig: &Rig, dimorphism: &Dimorphism) {
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
        joint.y + border(rig, head, dimorphism, 1.0),
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
    let band_of = |at: f32| {
        let along = ((at - top) / (bottom - top)).clamp(0.0, 1.0);
        ((along * (BANDS - 1) as f32).round() as usize).min(BANDS - 1)
    };
    let mine = owned(mesh, rig);
    let mut have = [0.0f32; BANDS];
    for (point, mine) in mesh.positions.iter().zip(&mine) {
        if !*mine {
            continue;
        }
        let at = depth(*point);
        if at < top || at > bottom {
            continue;
        }
        have[band_of(at)] = have[band_of(at)].max((point.x - axis.x).abs());
    }
    // A band no vertex reached takes its neighbour's, so a sparse ring never
    // reports a column pinching to nothing and the carve never divides by it.
    carry(&mut have);

    let want = NECK_SKULL * SKULL_OF_NODE * radius;
    let run = bottom - top;
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
                joint.y + border(rig, head, dimorphism, side.max(behind)),
                0.0,
            ));
        let hold = smooth(under / (run * RAMP)) * smooth((bottom - at) / (run * (1.0 - RELEASE)));
        if hold <= 0.0 {
            continue;
        }
        let here = have[band_of(at)].max(f32::EPSILON);
        // Only ever narrower. A column already inside the target is a small
        // head on a slender neck, and inflating it to meet a ratio would be
        // this module causing the defect it exists to remove.
        let factor = 1.0 + hold * ((want / here).min(1.0) - 1.0);

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
            * hold
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

/// Replaces bands no vertex reached with the nearest one that was.
fn carry(bands: &mut [f32; BANDS]) {
    let Some(first) = bands.iter().position(|&width| width > 0.0) else {
        return;
    };
    for band in 0..first {
        bands[band] = bands[first];
    }
    for band in first + 1..BANDS {
        if bands[band] <= 0.0 {
            bands[band] = bands[band - 1];
        }
    }
}
