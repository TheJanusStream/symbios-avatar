//! Where hair is allowed to grow, as five regions of the head's surface.
//!
//! Hair comes in two layers — painted into the skin, and low-poly geometry
//! standing off it — and the one thing both must agree about is WHERE. A beard
//! drawn to the jawline and a beard grown to the cheekbone is not a beard with
//! two layers, it is two beards. So the boundary is decided once, here, and both
//! layers ask the same question of the same object.
//!
//! **Five regions, because a head does not grow one kind of hair.** The scalp,
//! the brows, the upper lip, the chin and the flanks of the jaw each start and
//! stop somewhere different, take different styles, and are reached for
//! separately by anyone editing a face. They are [`Follicle`]'s variants and
//! each one owns its own file.
//!
//! # Everything here is measured, and nothing is a plan number
//!
//! The regions are cut from [`Skull`] and [`Canon`], which are themselves
//! measured from the built surface — so a boundary lands on the head that was
//! actually meshed rather than on the sphere the body plan described. This is
//! the lesson `hair::scalp` paid for: subdivision pulls a head well inside its
//! node radius, and anything placed against the nominal one floats.
//!
//! It is also why nothing here re-measures. [`Skull::measure`] is 61% of a
//! geometry build (#89), so this takes the one the pipeline already has.
//!
//! # A point is asked about in the head's own proportions
//!
//! A region cannot be written in metres, because a hairline eight centimetres
//! above the eye line is a hairline on one head and a crown on another. Every
//! boundary here is a fraction of a landmark span — [`Canon::frame`] for
//! anything up or down, the skull's own half-width and forward reach for
//! anything across or round — so the same numbers reach the same anatomy on
//! every head the crate builds. One internal `At` is that conversion, done once
//! per query and handed to whichever regions want it.
//!
//! # What a weight means
//!
//! [`Follicles::weight`] answers `0` where hair may not grow, `1` where it fully
//! may, and slides between over a soft edge. Neither layer wants a hard one: the
//! painted layer would show a cut-out and the geometry layer would end its
//! clumps in a straight line, and a hairline is neither. The falloff is written
//! into each region rather than applied here, because a jawline's edge is
//! sharper than a nape's and only the region knows.

pub mod brows;
pub mod chin;
pub mod flanks;
pub mod moustache;
pub mod scalp;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::face::skull::border_raise;
use crate::face::{Canon, Skull};

/// One kind of hair, and one region of the head to grow it on.
///
/// The unit everything about hair is organised by: a record carries one entry
/// per variant, a style catalogue belongs to exactly one of them, and both
/// layers are asked about a point one variant at a time.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Follicle {
    /// The hair of the head, above the hairline.
    Scalp,
    /// The eyebrows.
    Brows,
    /// The upper lip, between the mouth and the nose.
    Moustache,
    /// The chin and the plane under it.
    Chin,
    /// The flanks of the jaw, from the sideburn down over the jawline.
    Flanks,
}

impl Follicle {
    /// Every region, in the order a face is read down.
    ///
    /// Crown to chin, which is the order they are listed in the editor and
    /// reported in by any instrument that sweeps them — so two tools cannot
    /// disagree about which zone is which.
    pub const ALL: [Self; 5] = [
        Self::Scalp,
        Self::Brows,
        Self::Moustache,
        Self::Chin,
        Self::Flanks,
    ];

    /// What to call it in a panel, a report or a false-colour key.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Scalp => "scalp",
            Self::Brows => "brows",
            Self::Moustache => "moustache",
            Self::Chin => "chin",
            Self::Flanks => "flanks",
        }
    }

    /// A colour to draw it in, for any instrument that shows the five at once.
    ///
    /// Five hues far enough apart to be told apart where two regions meet,
    /// which is the only place a mask is hard to judge. Named here rather than
    /// in the instrument so that every instrument agrees: a zone that is green
    /// in the contact sheet and blue in the viewer is a zone nobody can discuss.
    ///
    /// Provenance: **picked**, for contrast against skin and against each other.
    #[must_use]
    pub const fn colour(self) -> Vec3 {
        match self {
            Self::Scalp => Vec3::new(0.25, 0.55, 1.00),
            Self::Brows => Vec3::new(1.00, 0.85, 0.20),
            Self::Moustache => Vec3::new(1.00, 0.35, 0.20),
            Self::Chin => Vec3::new(0.30, 0.90, 0.40),
            Self::Flanks => Vec3::new(0.85, 0.35, 0.95),
        }
    }
}

/// Where one point sits on the head, in the terms the regions ask in.
///
/// **Computed once and shared, because the conversions are not free and every
/// region wants the same ones.** The two normalising divisions each cost a
/// table walk through [`Skull`], and a naive implementation that let five
/// regions each convert the same point did that ten times per query.
///
/// The fields are the head's own proportions rather than metres, for the reason
/// the module header gives: a fraction of a measured span reaches the same
/// anatomy on any head, and a millimetre does not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct At {
    /// Height above the head joint, in head-local metres.
    ///
    /// The one field left in metres, because every landmark it is compared
    /// against is a measured height in the same unit.
    pub height: f32,
    /// How far forward, as a share of the skull's own reach at this height and
    /// lateral offset: `1` at the front of the face, `0` above the axis,
    /// negative behind it.
    ///
    /// **Normalised per height AND per lateral offset**, through
    /// [`Skull::depth_across`], because a face curves away under anything wide:
    /// a mouth corner sits several millimetres behind the midline at its own
    /// height, and a share taken against the midline's reach reads it as having
    /// wrapped round the head.
    pub forward: f32,
    /// How far out to the side, as a share of the skull's half-width at this
    /// height: `0` on the midline and `1` at the flank.
    pub lateral: f32,
    /// The cosine of the azimuth from dead ahead, as [`crate::face::skull`]'s
    /// own shaping has it.
    ///
    /// Kept beside [`Self::forward`] rather than derived from it because the
    /// mandible's border is written in this and nothing else:
    /// `border_raise(facing)` is the line the jaw was carved to, and a beard
    /// that ends anywhere else ends somewhere the face has no crease.
    pub facing: f32,
    /// Signed lateral offset in metres: negative to the body's left.
    ///
    /// A region that has to tell one side from the other — a parting, a
    /// single-sided style — cannot do it from [`Self::lateral`], which is
    /// unsigned because a head is symmetric and most regions are.
    pub across: f32,
}

/// What each region is allowed to be asked about a point.
///
/// The dispatch seam: [`Follicles`] resolves a point once into an [`At`] and
/// hands it to whichever region was asked for, so adding a region is a new file
/// and one match arm rather than an edit to a shared formula.
pub(crate) trait Region {
    /// How much hair may grow at this point, `0` to `1`.
    fn weight(&self, at: &At) -> f32;
}

/// How the five regions are shaped on one head.
///
/// Every field is an axis a record will carry (#202) and a panel will show. They
/// are shape parameters rather than style ones: they move where hair MAY grow,
/// which both layers obey, and say nothing about what is grown there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FollicleParams {
    /// The scalp's hairline.
    pub scalp: scalp::Params,
    /// The brows' patch.
    pub brows: brows::Params,
    /// The upper lip's patch.
    pub moustache: moustache::Params,
    /// The chin's patch.
    pub chin: chin::Params,
    /// The jaw flanks' patch.
    pub flanks: flanks::Params,
}

impl FollicleParams {
    /// Clamps every axis into the range its own region documents.
    ///
    /// Called by the record's own sanitize, and by anything that builds these
    /// from outside a record, because a region asked about a hairline at `12.0`
    /// answers with a scalp that has swallowed the face.
    pub fn sanitize(&mut self) {
        self.scalp.sanitize();
        self.brows.sanitize();
        self.moustache.sanitize();
        self.chin.sanitize();
        self.flanks.sanitize();
    }
}

/// The five regions of one head, resolved against its own landmarks.
///
/// Built once per body and shared by both hair layers. Holds the [`Skull`] it
/// was cut from, because two of the fields a region is asked in are shares of
/// the skull's own reach, and so cannot be resolved until a point arrives.
#[derive(Clone, Debug, PartialEq)]
pub struct Follicles {
    /// The head joint every region is measured from, and the one hair parents to.
    pub head: usize,
    origin: Vec3,
    skull: Skull,
    scalp: scalp::Scalp,
    brows: brows::Brows,
    moustache: moustache::Moustache,
    chin: chin::Chin,
    flanks: flanks::Flanks,
}

impl Follicles {
    /// Cuts the five regions from a measured head.
    ///
    /// Takes the [`Skull`] and [`Canon`] the pipeline has already measured
    /// rather than measuring its own: see the module header for what a second
    /// `Skull::measure` costs.
    #[must_use]
    pub fn of(rig: &crate::rig::Rig, skull: &Skull, canon: &Canon, params: &FollicleParams) -> Self {
        Self {
            head: skull.head,
            origin: rig.joints[skull.head].position,
            skull: skull.clone(),
            scalp: scalp::Scalp::of(skull, canon, &params.scalp),
            brows: brows::Brows::of(canon, &params.brows),
            moustache: moustache::Moustache::of(canon, &params.moustache),
            chin: chin::Chin::of(skull, canon, &params.chin),
            flanks: flanks::Flanks::of(skull, canon, &params.flanks),
        }
    }

    /// Where head-local space sits in the body, at rest.
    ///
    /// Anything holding body-space points — a texel, a bound vertex — comes
    /// back through here, or through [`Self::weight_in_body`], which does it.
    #[must_use]
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// The head this was cut from, as it was measured.
    ///
    /// **Handed out because a style has to be able to walk the surface it grows
    /// on** (#204). A scalp lock is supported by the skull for the first half of
    /// its travel and hangs only once the head has fallen away beneath it, and a
    /// lock that cannot ask where the skull is leaves the tangent plane at its
    /// root and hangs in the air — which is most of why a hundred and fifty of
    /// them read as strings over a bare scalp rather than as hair.
    ///
    /// The one the pipeline already measured, never a second: `Skull::measure` is
    /// 61% of a geometry build (#89).
    #[must_use]
    pub fn skull(&self) -> &Skull {
        &self.skull
    }

    /// The line one brow follows on this head.
    ///
    /// **Handed out rather than copied, because a style has to comb along the
    /// very curve the mask is centred on** (#205). Both brow styles take their
    /// direction, their rise and their fall from this, so a change to the ridge
    /// moves the paint and the geometry together and there is no second arc to
    /// keep in step. It is the same discipline the private `border` here follows:
    /// the beard's lower edge is not a copy of the jawline, it is the jawline.
    #[must_use]
    pub fn brow_ridge(&self) -> brows::Ridge {
        self.brows.ridge()
    }

    /// The patch of lip a moustache grows on, on this head.
    ///
    /// **The same discipline as [`Self::brow_ridge`]** (#206): the styles take
    /// their floor, their ceiling and their outer end from the object the mask
    /// was cut around, so a record moving the patch moves the paint and the
    /// geometry together. The moustache is the region where that matters most —
    /// its floor is the mouth, and a grown hair that disagrees with the mask by a
    /// millimetre disagrees with it INTO somebody's lip.
    #[must_use]
    pub fn lip(&self) -> moustache::Lip {
        self.moustache.lip()
    }

    /// The patch of chin a beard grows on, on this head.
    ///
    /// The third of the handed-out landmarks, and the one whose styles need it
    /// most: a chin beard is the only hair on a face that leaves the face, so a
    /// style has to know where the chin STOPS as well as where the patch is
    /// (#207). See [`chin::Pad::hangs_from`].
    #[must_use]
    pub fn pad(&self) -> chin::Pad {
        self.chin.pad()
    }

    /// The beard line one flank grows under, on this head.
    ///
    /// The fourth of the handed-out landmarks. Its lower edge is not in here
    /// because it is not a number: see [`Self::jawline`].
    #[must_use]
    pub fn beard_line(&self) -> flanks::Line {
        self.flanks.line()
    }

    /// The mandible's lower border at one azimuth, in head-local metres.
    ///
    /// **The same line the face was carved to, handed out rather than copied**
    /// (#196 is what a second copy cost, and #208 is where a style needed it).
    /// A flank beard's lower edge IS the approved crease, so a style asks for it
    /// here and there is no second arc to keep in step.
    #[must_use]
    pub fn jawline(&self, facing: f32) -> f32 {
        Self::border(&self.skull, facing)
    }

    /// How much of `follicle` may grow at a head-local point, `0` to `1`.
    #[must_use]
    pub fn weight(&self, follicle: Follicle, local: Vec3) -> f32 {
        let at = self.resolve(local);
        self.region(follicle).weight(&at)
    }

    /// The same, for a point in body space.
    ///
    /// **Named for its space, and so is its neighbour, because the two are one
    /// subtraction apart and a silent wrong-space query is a mask that is
    /// almost right.** The painted layer holds body-space texels and the
    /// geometry layer works head-local; both call this file.
    #[must_use]
    pub fn weight_in_body(&self, follicle: Follicle, point: Vec3) -> f32 {
        self.weight(follicle, point - self.origin)
    }

    /// Every region's weight at one head-local point, in [`Follicle::ALL`] order.
    ///
    /// One resolve for five answers, which is what any sweep over the whole
    /// head wants — a false-colour instrument, a root scatterer walking the
    /// surface once, or a painter with five layers to composite.
    #[must_use]
    pub fn weights(&self, local: Vec3) -> [f32; 5] {
        let at = self.resolve(local);
        Follicle::ALL.map(|follicle| self.region(follicle).weight(&at))
    }

    /// The region with the most claim on a head-local point, if any has one.
    ///
    /// For anything that has to pick one — a false colour, a single-material
    /// draw — rather than composite all five.
    #[must_use]
    pub fn strongest(&self, local: Vec3) -> Option<(Follicle, f32)> {
        let weights = self.weights(local);
        Follicle::ALL
            .into_iter()
            .zip(weights)
            .filter(|(_, weight)| *weight > 0.0)
            .max_by(|(_, one), (_, two)| one.total_cmp(two))
    }

    /// Resolves a head-local point into the terms the regions ask in.
    fn resolve(&self, local: Vec3) -> At {
        let across = local.x;
        let reach = (local.x * local.x + local.z * local.z).sqrt();
        // A point on the head's own axis has no azimuth. It is called dead
        // ahead, because the only points that reach it are on the midline under
        // the chin and at the crown, and of the two only the chin's region cares
        // — where the border it wants is the menton's, which is `facing` at 1.
        let facing = if reach > f32::EPSILON {
            local.z / reach
        } else {
            1.0
        };
        // Against the skull's reach at this height, floored so that a head with
        // no measurable width at some band — a crown, a creature's muzzle —
        // answers a large share rather than an infinity.
        let half_width = self.skull.half_width(local.y).max(MINIMUM_SPAN);
        let depth = self
            .skull
            .depth_across(local.y, across.abs())
            .max(MINIMUM_SPAN);
        At {
            height: local.y,
            forward: local.z / depth,
            lateral: across.abs() / half_width,
            facing,
            across,
        }
    }

    /// The region behind one variant.
    fn region(&self, follicle: Follicle) -> &dyn Region {
        match follicle {
            Follicle::Scalp => &self.scalp,
            Follicle::Brows => &self.brows,
            Follicle::Moustache => &self.moustache,
            Follicle::Chin => &self.chin,
            Follicle::Flanks => &self.flanks,
        }
    }

    /// The mandible's lower border at one azimuth, in head-local metres.
    ///
    /// **The jawline the face was actually carved to, not a second copy of it**
    /// (#196 is what happens when there are two). [`Skull::chin`] and
    /// [`Skull::gonion`] are the two ends of the very profile heights
    /// `face::skull::jaw` runs its border between, and `border_raise` is the
    /// same run in the same cosine — so the beard's lower edge and the crease
    /// under it are one line, and a change to either moves both.
    pub(crate) fn border(skull: &Skull, facing: f32) -> f32 {
        let chin = skull.chin();
        chin + (skull.gonion() - chin) * border_raise(facing)
    }
}

/// The smallest span a normalising division will divide by, in metres.
///
/// A millimetre: small enough that no real band of a head is clamped by it, and
/// large enough that a band which measured nothing cannot turn a share into an
/// infinity and a region into the whole head.
///
/// Provenance: **derived** from the failure it prevents rather than from a face.
const MINIMUM_SPAN: f32 = 0.001;

/// A soft-edged band: `1` between `lo` and `hi`, `0` outside, sliding over
/// `fade`.
///
/// The workhorse every region is written in, so that the shape of an edge is
/// decided in one place and each region only says where and how wide. Both
/// edges are smoothsteps, so a band arrives and leaves with zero slope and puts
/// no crease in either layer.
pub(crate) fn band(value: f32, lo: f32, hi: f32, fade: f32) -> f32 {
    let fade = fade.max(f32::EPSILON);
    crate::face::smooth((value - lo) / fade) * crate::face::smooth((hi - value) / fade)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Rig;
    use crate::{Archetype, Avatar, AvatarRecord, Composites, PolyMesh, Zone};

    /// A built head and everything cut from it.
    struct Head {
        follicles: Follicles,
        skull: Skull,
        /// The head's own surface, as a centroid and an area per face.
        ///
        /// **Area rather than vertices, and the difference is not a detail**
        /// (#199). `refine_face` splits the front of the face ten times and
        /// leaves the vault at the base subdivision, so a head carries thousands
        /// of vertices on a chin and dozens on a crown. Counting vertices, the
        /// first cut of these tests read the scalp — the largest region on the
        /// head — as 2.9% of it and the chin as 25%, which is a measurement of
        /// the refinement schedule and not of the mask.
        surface: Vec<(Vec3, f32)>,
        origin: Vec3,
    }

    /// Builds one body and cuts its regions, the way the pipeline will.
    fn head_of(record: &AvatarRecord) -> Head {
        let avatar = Avatar::build(record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default());
        let origin = avatar.rig.joints[skull.head].position;
        Head {
            surface: surface_of(&avatar.parts.body, &avatar.rig, origin),
            skull,
            follicles,
            origin,
        }
    }

    /// The same, for a rolled seed.
    fn seeded(seed: i64) -> Head {
        let mut record = AvatarRecord::new("Follicles", Archetype::default());
        record.reroll(seed);
        head_of(&record)
    }

    /// The seeds every sweep here runs.
    const SEEDS: [i64; 6] = [0, 3, 7, 13, 23, 42];

    /// Every head-owned face, as a head-local centroid and an area.
    fn surface_of(body: &PolyMesh, rig: &Rig, origin: Vec3) -> Vec<(Vec3, f32)> {
        (0..body.face_count())
            .filter_map(|face| {
                let centre = body.face_centroid(face);
                if rig.joints[rig.nearest_bone(centre).joint].zone != Zone::Head {
                    return None;
                }
                let corners = &body.faces[face];
                // Fan from the first corner: every face here is a quad or a
                // triangle, both of which a fan measures exactly.
                let area = (1..corners.len().saturating_sub(1))
                    .map(|step| {
                        let one = body.positions[corners[step] as usize]
                            - body.positions[corners[0] as usize];
                        let two = body.positions[corners[step + 1] as usize]
                            - body.positions[corners[0] as usize];
                        one.cross(two).length() * 0.5
                    })
                    .sum();
                Some((centre - origin, area))
            })
            .collect()
    }

    /// What share of the head's own surface AREA one region holds.
    fn share(head: &Head, follicle: Follicle) -> f32 {
        let whole: f32 = head.surface.iter().map(|(_, area)| area).sum();
        let held: f32 = head
            .surface
            .iter()
            .filter(|(local, _)| head.follicles.weight(follicle, *local) > 0.5)
            .map(|(_, area)| area)
            .sum();
        held / whole.max(f32::EPSILON)
    }

    #[test]
    fn every_region_lands_on_some_of_the_head_it_was_cut_from() {
        // A region that selects nothing passes every assertion about its
        // boundaries, which is what this file is most exposed to: five regions
        // written in fractions of a measured span, any one of which could be off
        // its landmark and still be a perfectly smooth mask of nothing. Every
        // sweep below is worthless without this one.
        //
        // Asserted on FULL weight rather than on any weight at all, because a
        // region can be present and still useless: the moustache's first cut
        // reached 0.227 at its best and nowhere ever came on, which is a patch
        // no style can grow a hair in.
        for seed in SEEDS {
            let head = seeded(seed);
            for follicle in Follicle::ALL {
                let best = head
                    .surface
                    .iter()
                    .map(|(local, _)| head.follicles.weight(follicle, *local))
                    .fold(0.0f32, f32::max);
                assert!(
                    best > 0.95,
                    "seed {seed}: the {} region never comes fully on anywhere on the head — \
                     its best weight is {best:.3}",
                    follicle.name()
                );
            }
        }
    }

    #[test]
    fn no_region_swallows_the_head() {
        // The opposite failure, and the one a fraction-of-a-span boundary makes
        // easy: a sign slip or a missing gate hands a region the whole skull,
        // which still renders as *a* mask and still passes the test above.
        //
        // The ceilings are measured rather than picked, and the population they
        // came from is below. The scalp is allowed the most, being the one
        // region that covers a whole surface rather than a patch of one.
        for seed in SEEDS {
            let head = seeded(seed);
            for follicle in Follicle::ALL {
                let held = share(&head, follicle);
                // The bounds are the measured population with room either
                // side, and the population is quoted so that the next person to
                // move one of these re-measures rather than trusts a number
                // that was true once. `follicleaudit --sweep` prints it.
                let (floor, ceiling) = match follicle {
                    // 32.7% to 48.1%: a skull cap, on a surface whose other
                    // half is a face and a throat.
                    Follicle::Scalp => (0.20, 0.60),
                    // 0.41% to 0.91%. The smallest region on the head by an
                    // order of magnitude, and the one a wrong landmark empties
                    // first.
                    Follicle::Brows => (0.002, 0.02),
                    // 0.47% to 0.65%.
                    Follicle::Moustache => (0.002, 0.02),
                    // 3.1% to 5.1%.
                    Follicle::Chin => (0.015, 0.10),
                    // 6.6% to 7.9%.
                    Follicle::Flanks => (0.03, 0.14),
                };
                assert!(
                    held >= floor && held <= ceiling,
                    "seed {seed}: the {} region holds {:.1}% of the head's area, outside the \
                     {:.1}-{:.1}% a region of its kind may",
                    follicle.name(),
                    held * 100.0,
                    floor * 100.0,
                    ceiling * 100.0,
                );
            }
        }
    }

    #[test]
    fn no_boundary_is_a_cliff() {
        // Both layers need this and neither can enforce it: a painted mask with
        // a hard edge is a cut-out, and a scattered one ends its clumps in a
        // straight line.
        //
        // **Measured as weight per millimetre travelled, not per sample.** The
        // first cut of this reported a step between neighbouring samples, which
        // says as much about the sampling as about the mask — and a sweep round
        // a head crosses a near-vertical boundary like the temple's much faster
        // in degrees than it does in millimetres. A gradient in a real unit is
        // the same number however finely it is walked, and it is the number both
        // layers actually care about: its reciprocal is how wide the edge is.
        //
        // Six tenths of a unit per millimetre is an edge about 1.5 mm wide,
        // which is a floor rather than a target: the finest cells on a refined
        // face measure 0.8 mm, so an edge under two of them is one the surface
        // cannot express however soft the field says it is. The measured
        // population is 1.8 mm at its tightest (seed 42's flank, per
        // `follicleaudit --sweep`), and the walk here is horizontal — so a
        // boundary tilted off the horizontal, like the flank's beard line,
        // is crossed at an angle and reads narrower than it is. Both facts are
        // why the bound sits here rather than at the 2 mm the sentence above
        // would suggest.
        let head = seeded(0);
        let (throat, crown) = head.skull.throat_and_crown();
        let mut worst = [(0.0f32, 0.0f32); 5];
        let point = |angle: f32, height: f32| {
            let half = head.skull.half_width(height).max(MINIMUM_SPAN);
            let across = half * angle.sin();
            let depth = head.skull.depth_across(height, across.abs());
            Vec3::new(across, height, depth * angle.cos())
        };
        for step in 0..300 {
            let height = throat + (crown - throat) * step as f32 / 299.0;
            for turn in 0..360 {
                let here = std::f32::consts::TAU * turn as f32 / 360.0;
                let next = here + std::f32::consts::TAU / 360.0;
                let (one, two) = (point(here, height), point(next, height));
                let span = (two - one).length() * 1000.0;
                if span < 1e-3 {
                    continue;
                }
                let (left, right) = (head.follicles.weights(one), head.follicles.weights(two));
                for (slot, worst) in worst.iter_mut().enumerate() {
                    let slope = (left[slot] - right[slot]).abs() / span;
                    if slope > worst.0 {
                        *worst = (slope, height);
                    }
                }
            }
        }
        for (follicle, (slope, height)) in Follicle::ALL.into_iter().zip(worst) {
            assert!(
                slope < 0.60,
                "the {} region's edge climbs {slope:.2} of its weight per millimetre at height \
                 {:.0} mm, which is an edge {:.1} mm wide — neither layer can soften that",
                follicle.name(),
                height * 1000.0,
                // The same 0.1-to-0.9 width `follicleaudit` prints, by the same
                // arithmetic: a smoothstep spends that crossing over 0.58 of its
                // run and is steepest at 1.5 over it. Two instruments reporting
                // one quantity in two conventions is how a population gets
                // quoted against a bound it was never measured with.
                0.58 * 1.5 / slope,
            );
        }
    }

    #[test]
    fn the_beard_ends_on_the_jawline_the_face_was_carved_to() {
        // The one boundary shared with another file. `face::skull::jaw` cuts a
        // crease along `border_raise`'s cosine (#195), and a beard whose lower
        // edge is a second copy of that line drifts away from it the first time
        // either moves — which is exactly what #196 was. Asserted as the
        // identity it is, and then for the shape the owner asked for: the border
        // is the chin's own height dead ahead and has climbed to the gonion at
        // the side.
        let head = seeded(0);
        let skull = &head.skull;
        for turn in 0..12 {
            let facing = (std::f32::consts::PI * turn as f32 / 11.0).cos();
            let here = Follicles::border(skull, facing);
            let want = skull.chin() + (skull.gonion() - skull.chin()) * border_raise(facing);
            assert!(
                (here - want).abs() < 1e-6,
                "the beard's border and the jaw's disagree by {:.2} mm at facing {facing:+.2}",
                (here - want) * 1000.0
            );
        }
        assert!(
            (Follicles::border(skull, 1.0) - skull.chin()).abs() < 1e-6,
            "the border does not meet the menton dead ahead"
        );
        assert!(
            (Follicles::border(skull, 0.0) - skull.gonion()).abs() < 1e-6,
            "the border does not reach the gonion at the side"
        );
    }

    #[test]
    fn a_head_local_query_and_a_body_space_one_agree() {
        // The two-space trap named on `weight_in_body`: the two are one
        // subtraction apart, and a silent mistake is a mask that is almost
        // right — which is the hardest kind to see in a render.
        let head = seeded(7);
        for (local, _) in head.surface.iter().take(300) {
            for follicle in Follicle::ALL {
                let here = head.follicles.weight(follicle, *local);
                let there = head
                    .follicles
                    .weight_in_body(follicle, *local + head.origin);
                assert!(
                    (here - there).abs() < 1e-6,
                    "the {} region answers {here:.3} head-local and {there:.3} in body space",
                    follicle.name()
                );
            }
        }
    }

    #[test]
    fn the_regions_hold_their_landmarks_across_the_frame_axes() {
        // A mask cut in fractions of a measured span should reach the same
        // anatomy on a long narrow head and a short broad one. Swept over the
        // frame's own extremes, each region's share is allowed to move — a
        // receding hairline is a real difference between faces — but not to
        // vanish, which is what a boundary reading a metre rather than a share
        // does on the first head that is not the default one.
        for femininity in [-1.0f32, 0.0, 1.0] {
            for age in [25u32, 80] {
                let mut record = AvatarRecord::new("Axis", Archetype::default());
                record.reroll(11);
                record.composites = Composites {
                    femininity,
                    age,
                    ..Composites::default()
                };
                let head = head_of(&record);
                for follicle in Follicle::ALL {
                    let held = share(&head, follicle);
                    assert!(
                        held > 0.001,
                        "at femininity {femininity:+.0} age {age} the {} region has shrunk to \
                         {:.2}% of the head",
                        follicle.name(),
                        held * 100.0
                    );
                }
            }
        }
    }

    #[test]
    fn the_two_beard_regions_close_their_seam() {
        // #208 will blend these two into one beard, and it can only do that if
        // the masks already meet: a bald stripe down the jaw between the chin's
        // patch and the flank's is not something a style can paper over. Swept
        // along the jawline itself, the two are asked together at every azimuth
        // and their sum has to hold up across the join.
        for seed in SEEDS {
            let head = seeded(seed);
            let mut thinnest = (f32::MAX, 0.0f32);
            for turn in 1..40 {
                let angle = std::f32::consts::FRAC_PI_2 * turn as f32 / 40.0;
                let facing = angle.cos();
                // A little above the border, where a beard is at its fullest and
                // the two regions overlap if they overlap anywhere.
                let height =
                    Follicles::border(&head.skull, facing) + head.skull.chin().abs() * 0.10;
                let half = head.skull.half_width(height).max(MINIMUM_SPAN);
                let across = half * angle.sin();
                let depth = head.skull.depth_across(height, across.abs());
                let local = Vec3::new(across, height, depth * facing);
                let sum = head.follicles.weight(Follicle::Chin, local)
                    + head.follicles.weight(Follicle::Flanks, local);
                if sum < thinnest.0 {
                    thinnest = (sum, angle.to_degrees());
                }
            }
            assert!(
                thinnest.0 > 0.30,
                "seed {seed}: the chin and flank regions leave a gap along the jawline — their \
                 weights sum to only {:.2} at {:.0}° off the midline",
                thinnest.0,
                thinnest.1
            );
        }
    }
}
