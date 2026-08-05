//! A face carved into the head rather than stuck onto it.
//!
//! Features used to be separate closed solids — a nose, two brows, two lips —
//! appended to the body mesh without being welded to it. Measured against the
//! surface they sat on, every one of them was at or below one quad in its
//! smallest dimension: a brow ridge is 10 mm tall on a head whose faces were
//! 24 mm across, and a nose was exactly one quad wide (#59). There was no
//! surface to be a face, so the face was appliqué, and it read as appliqué —
//! sweeping all three prominence axes across their whole range barely changed
//! it, because the axes were attached to the wrong thing.
//!
//! [`crate::face::refine_face`] gave the front of the head 6.8 mm cells, which
//! is what makes this file possible: every feature here is a **displacement of
//! the head's own surface**, so the nose is made of the same skin as the cheek
//! beside it, there is no boundary for a bone or a morph to fail to cross, and
//! the texture charts and skin weights that follow are computed over a face
//! that already exists.
//!
//! **Moderate stylisation, which is a decision and not a default.** Real
//! anatomical structure — nostril wings, a philtrum, a lip line, a brow running
//! into the eye socket — at roughly human proportion, with edges softened rather
//! than exaggerated. Amplitudes stay near life. The gain comes from having
//! enough surface to carry detail at all, not from pushing the numbers, and the
//! shortcut this rules out is exaggerating a feature so it reads through a
//! coarse surface.
//!
//! Everything is anchored to the eyes, like [`super::features`], and for the
//! same reason: two features that each approximate where the face is will not
//! agree with each other.

use glam::Vec3;

use super::smooth;
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

use super::canon::Canon;
use super::features::FaceParams;

/// The mouth's lobes: how far each stands out, where it sits and how wide it is,
/// in units of the lip stack's half-height.
///
/// **Gathered into one place because their WIDTHS are the thing that decides
/// whether a mouth reads at all.** Every one of them has to be wider than the
/// mesh cell under it — a Gaussian one cell across renders as a single displaced
/// row of vertices, which is a horizontal bar, and four of those stacked is what
/// the owner reported as a terraced lower face (#85). At 3.6 mm cells they
/// measured 0.99, 1.29, 1.67 and 1.75 cells; the mouth band is refined a fourth
/// time so they are 2.0 to 3.5. `the_mouth_is_wider_than_the_mesh_under_it`
/// holds that, as a ratio rather than a millimetre figure, because the cell size
/// moves with the refinement level and with the size of the head.
/// **The fourth number is how each lobe ends at the corner of the mouth, and
/// giving them all the same one is what drew the bars.** Every lobe used to be
/// multiplied by a single `corner` factor that is 1.0 out to 0.90 of the mouth's
/// half-width and only then lets go: measured off the vertices, the lower lip
/// stood 5.34 mm proud DEAD FLAT from the midline to 20 mm out, with the whole
/// taper crammed into the last 6 mm of 26. That is an extruded bar with a
/// rounded end, and no amount of resolution can make it a lip (#82).
///
/// The two vermilion lobes and the sulcus below them are `Lens`: they thin
/// continuously from the middle and vanish where the lips meet. The line between
/// them is `Groove`, and it is the whole reason this is per-lobe — on a face the
/// commissure is the DEEPEST part of the mouth line, not the shallowest, and
/// fading the groove out along with the vermilion is what leaves a bar with a
/// rounded end instead of two lips meeting at a point.
/// Provenance: **tuned by render** (#82), against a bisected profile — the
/// 5.34 mm dead-flat measurement above is what condemned the previous table.
const LIPS: [(f32, f32, f32, Across); 4] = [
    (0.88, -0.60, 0.46, Across::Lens), // the lower lip
    (0.82, 0.58, 0.44, Across::Lens),  // the upper lip
    // The line between them, the narrowest thing on a face.
    (-0.44, 0.00, 0.26, Across::Groove),
    (-0.24, -1.32, 0.34, Across::Lens), // the crease under the lower lip
];

/// How a lobe of the mouth ends at the corner.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Across {
    /// Thins from the middle and reaches nothing where the lips meet.
    ///
    /// `1 - across²`, and deliberately not raised to a fractional power. The
    /// prettier `(1 - across²)^0.65` has an INFINITE derivative at the corner,
    /// which trades a bar for a crease — the same cusp this file already carries
    /// on the nose's flank and knows it aliases.
    Lens,
    /// Holds its depth almost to the corner, then ends without a cusp.
    ///
    /// Reaches past the vermilion on purpose: the lobes are gone by `1.0` and
    /// this is still going at `1.05`, which is what makes a corner read as two
    /// lips meeting rather than as the end of a slab.
    Groove,
}

impl Across {
    /// How much of a lobe survives, `across` being distance from the midline in
    /// mouth half-widths.
    fn taper(self, across: f32) -> f32 {
        match self {
            Self::Lens => (1.0 - across * across).max(0.0),
            Self::Groove => smooth(((1.05 - across) / 0.20).min(1.0)),
        }
    }
}

/// How far round the head a feature can reach before it is faded out.
///
/// A cosine of the angle from dead ahead. Every field below is bounded in `x`
/// and `y` already, so this is a backstop rather than a shape: without it a
/// nose whose lateral falloff has not quite reached zero by the ear puts a
/// ripple on the side of the head.
/// Provenance: **unsourced**, and a backstop rather than a shape, so it is
/// one of the few numbers here that no measurement could confirm — it exists
/// to be far enough round the head that nothing reaches it.
const FRONTAL: f32 = 0.15;

/// Carves a face into a built head, in place.
///
/// Runs on the **rest** mesh, after [`super::skull::shape`] and before anything
/// is bound or unwrapped, so skin weights, texture charts and every attached
/// part are fitted to a head that already has a face. Does nothing to a body
/// with no head, or one whose head carries too little surface to profile.
///
/// Displaces along vertex normals. The alternative — pushing everything along
/// `+Z` — is what a relief map does to a plane, and a face is not a plane: the
/// wing of a nose and the corner of a mouth both sit where the head has already
/// turned thirty degrees away from the front, and a push along one axis
/// flattens them into the cheek instead of standing them off it.
pub fn carve(mesh: &mut PolyMesh, rig: &Rig, canon: &Canon, params: &FaceParams) {
    let centre = rig.joints[canon.head].position;
    let face = Face::new(canon, params);

    // Taken before anything moves. Displacing along a normal that is itself
    // being displaced makes the result depend on vertex order, which is a
    // difference between two builds of the same body.
    let normals = mesh.vertex_normals();
    let owned: Vec<bool> = mesh
        .positions
        .iter()
        .map(|&point| rig.joints[rig.nearest_bone(point).joint].zone == Zone::Head)
        .collect();

    for (vertex, point) in mesh.positions.iter_mut().enumerate() {
        if !owned[vertex] {
            continue;
        }
        let local = *point - centre;
        let across = Vec3::new(local.x, 0.0, local.z);
        let reach = across.length();
        if reach <= f32::EPSILON {
            continue;
        }
        let frontal = ((across.z / reach - FRONTAL) / (1.0 - FRONTAL)).clamp(0.0, 1.0);
        if frontal <= 0.0 {
            continue;
        }
        *point += normals[vertex] * face.lift(local) * smooth(frontal);
    }
}

/// The landmarks every field here is placed from, in head-local metres.
///
/// **Two rulers, not one.** Every coefficient below used to be a multiple of the
/// eyeball's radius, which meant the eye-size slider silently resized the nose,
/// the mouth, the lips, the brow and both ears — and meant a coefficient fitted
/// on one body was up to 84% out on another, because that ruler's ratio to the
/// face it was ruling was itself a free variable (#77). Widths and reaches are
/// counted in [`Canon::unit`], heights in [`Canon::frame`], and the two are
/// measured off the built head rather than derived from each other.
struct Face {
    /// One eye-width: what anything across, or standing out, is counted in.
    unit: f32,
    /// The eye line to the chin: what anything up or down is counted in.
    frame: f32,
    /// The eye line.
    level: f32,
    /// Where the base of the nose sits.
    base: f32,
    /// Where the lips meet.
    mouth: f32,
    /// How tall the lip stack is, from the mouth line to a lobe's centre.
    plump: f32,
    /// How far each eye sits from the midline.
    apart: f32,
    /// How prominent each feature is.
    params: FaceParams,
}

impl Face {
    fn new(canon: &Canon, params: &FaceParams) -> Self {
        Self {
            unit: canon.unit,
            frame: canon.frame,
            level: canon.level,
            // The same fraction of the eye-line-to-chin span `super::features`
            // counts in, so a nose carved here and an ear placed there agree
            // about where the middle third of the face ends. Counted from the
            // chin's TIP, not from `throat_and_crown().0`: the span ends at the
            // throat, 28 mm below the chin on a default body, and a frame
            // stretched to the throat put the whole feature stack a third of a
            // storey too low (#72).
            base: canon.down(super::features::NOSE_BASE),
            mouth: canon.down(super::features::MOUTH_HEIGHT),
            // A height, so counted in the frame — and that is what retires the
            // clamp this line used to carry. Sized by the eyeball, the lip stack
            // could reach past the chin on a large-eyed short-faced head (seed
            // 99 put the sulcus lobe at −69.1 mm against a chin at −68.7), so it
            // was capped at 0.20 of the frame; and the cap then BOUND on every
            // mouth value on that seed and above 0.594 on the default, which
            // made most of the slider dead on half the bodies. A clamp that is
            // load-bearing is the tell that the quantity was measured in the
            // wrong ruler. In the frame, the bound is met by construction: the
            // lowest lobe sits 1.32 plumps below the mouth line, which is at
            // 0.596 of the frame, so the stack stays above the tip while
            // `plump` stays under 0.306 — and the axis tops out at 0.146.
            //
            // Rebased by 0.6717 with the rest of the heights when #78 lengthened
            // the frame, holding the lip stack at 13.7 mm where life is 11 to 14.
            // Left at its old fraction it would have grown by half.
            plump: canon.frame * (0.1142 + 0.0322 * params.mouth),
            apart: canon.apart,
            params: *params,
        }
    }

    /// The narrowest Gaussian any of this face's fields uses, in metres.
    ///
    /// The number that decides whether a mouth can render at all: a term
    /// narrower than the mesh cell under it becomes one displaced row of
    /// vertices, which is a bar rather than a lip. See [`LIPS`].
    ///
    /// Test-only, and deliberately a method rather than arithmetic repeated in
    /// the test: a check that recomputes the widths from its own copy of the
    /// constants stops meaning anything the first time one of them moves.
    #[cfg(test)]
    fn finest(&self) -> f32 {
        self.plump
            * LIPS
                .iter()
                .map(|&(_, _, width, _)| width)
                .fold(f32::MAX, f32::min)
    }

    /// How far the surface stands out at a point, in metres.
    ///
    /// Summed, not maximised. Features that meet — the wing of a nose against
    /// the philtrum below it, the brow against the socket — have to add up
    /// across the join or there is a crease where one field wins.
    fn lift(&self, local: Vec3) -> f32 {
        self.nose(local) + self.brow(local) + self.mouth(local) + self.orbit(local)
    }

    /// The hollow on the nasal side of each eye, where the medial canthus sits.
    ///
    /// **A head here has no eye socket at all, and the consequence is not that
    /// the eye looks shallow — it is that the eye is not where it says it is.**
    /// The body is a closed surface with no opening cut for an eye, so the globe
    /// simply pokes through it and the shape of what emerges is set by whatever
    /// the skin happens to be doing rather than by the gaze. Measured on the
    /// globe's own surface about its own axis, the skin used to cover it from
    /// 16° medial inward and not reach it at all until 99° lateral: an opening
    /// 115° wide whose middle sat 42° off the direction the eye was looking.
    /// The iris is the one thing on an eye that IS centred, so it read as
    /// shoved to the nasal side (#88).
    ///
    /// **Medial only, and that is a measurement rather than a saving.** Laterally
    /// the lid's own margin closes the aperture at 59° while the skin does not
    /// reach the globe until 99, so the LID owns that edge already (#81) and
    /// anything added out there is a rim rather than an aperture. Medially the
    /// globe lies 3.6 mm under the skin at 40° off the gaze — which is where the
    /// aperture's centroid comes back inside 5° of the gaze — rising to 7.7 at
    /// 60°.
    ///
    /// **Only the DIFFERENCE between the cut at the column and the cut at the
    /// canthus counts, and that is the whole shape of it.**
    /// [`super::eye::Eyes::build`] seats the globe by bisecting the CARVED
    /// surface at the eye's own column, so whatever is cut there sinks the globe
    /// by the same amount and takes its medial surface down with it: a bowl
    /// centred on the eye achieves exactly nothing, however deep.
    ///
    /// This used to be read as "keep the hollow away from the column", and that
    /// is a stronger conclusion than the premise supports (#91). What the premise
    /// forbids is a hollow that is FLAT across the eye, not one that touches it.
    /// The hollow now reaches the column at about half its depth and the canthus
    /// at full, and it is that gradient which opens the aperture; the globe
    /// recedes 3 to 4 mm and the opening still gains 14°. Anatomically it is the
    /// right shape too — the medial orbital wall is the deepest part of the rim,
    /// but the rim does surround the eye.
    ///
    /// **What the aperture has to clear is the IRIS, not the pupil.** Sclera
    /// shows medially only past 28.9° off the gaze, which is `eye::LIMBUS`; the
    /// pupil's 8.2° is not the bar and a medial edge that clears it looks like a
    /// success while the eye still reads as having no white on the nasal side.
    /// Before this pass the edge sat at −15.5° to −24° on seven seeds — inside
    /// the iris on six of them — so the iris met skin directly and the eye read
    /// as shoved toward the nose.
    ///
    /// **Carried by no axis on purpose.** Every other field here is weighted by
    /// a [`FaceParams`] slider; this one is not, because a brow slider that also
    /// controlled the socket would put the eye back off its own axis at one end
    /// of its range, and an eye has to be seated everywhere in the parameter
    /// space rather than at the default. If an orbit deserves an axis, that is
    /// the face's parameter space to widen, not this term's to borrow.
    fn orbit(&self, local: Vec3) -> f32 {
        /// How deep the hollow cuts, in eye-widths: 16.0 mm on a default body.
        ///
        /// **Up from 0.240, and it is pinned by a ceiling rather than chosen.**
        /// A carve displaces along the vertex NORMAL, and the normal on the
        /// nasal flank runs about 44° off the head's radial, so only about 0.72
        /// of what is cut is depth the globe recovers — and the globe then
        /// follows the socket back, which takes another share. The authored
        /// number is therefore much larger than anything that appears on the
        /// face: 0.620 recedes the eye 5 mm and opens the aperture by rather
        /// less than it removes.
        ///
        /// **What stops it going further is `an_eye_is_seated_in_the_face`, not
        /// taste.** Deepening the socket exposes more of the globe, and that
        /// test holds exposure under 25% because an eye past it reads as popped
        /// out (#73). The two requirements pull against each other and this sits
        /// at the boundary, so a future change that wants a deeper orbit has to
        /// buy exposure back somewhere else rather than raise that ceiling.
        ///
        /// **Down from 0.620 because #93 shortened the neck**, which drops the
        /// head about 18 mm and re-scales the skull's below-joint domain, and
        /// that was enough to put seed 13's right eye at 25% exposure. Sitting
        /// on a ceiling means any change to the head spends the margin, and this
        /// is the second time in two passes that has happened here; 0.540 is too
        /// shallow and loses the nasal white again, so the usable band is narrow
        /// in both directions.
        const DEPTH: f32 = 0.580;
        /// How far medial of the pupil it is deepest, in eye-widths.
        ///
        /// **Down from 0.28, and inward is the direction nobody had tried.**
        /// 0.28 put the hollow's centre 8.7 mm medial of the pupil while the
        /// aperture's edge sits about 3.3 mm out, so the Gaussian delivered less
        /// than half its depth where the edge actually was. #88 tested this axis
        /// OUTWARD only — 0.36 took the medial edge from −42° to −19° on seed 7
        /// — and recorded that as "do not move `MEDIAL`", when what it shows is
        /// "do not move it outward". Inward is worth more than depth is, and
        /// costs no material.
        const MEDIAL: f32 = 0.14;
        /// How far it reaches across and up, as Gaussian widths in eye-widths.
        ///
        /// Both are six to eleven cells where the face is refined, well clear of
        /// the one-cell floor that renders a field as a bar (#85). Widening `UP`
        /// past 0.40 buys nothing — 0.48 moves the aperture's centre by under a
        /// degree.
        ///
        /// `ACROSS` narrowed with `MEDIAL`, and for the same reason: what opens
        /// the aperture is the DIFFERENCE between the cut at the canthus and the
        /// cut at the eye's own column, so a hollow that is tight around the
        /// canthus has a steeper gradient than a broad one of the same depth.
        /// Broadening it was measured and is worse — 0.28 across drops the
        /// nasal white on seed 15 from 11% to nothing.
        const ACROSS: f32 = 0.16;
        const UP: f32 = 0.40;

        let medial = (self.apart - local.x.abs()) / self.unit;
        let up = (local.y - self.level) / self.unit;
        -self.unit * DEPTH * bump(medial, MEDIAL, ACROSS) * bump(up, 0.0, UP)
    }

    /// A brow ridge arching over each eye, with the socket beneath it.
    ///
    /// The ridge was a separate solid floating 21.5 mm clear of the nearest head
    /// vertex, which is a bar above an eye rather than a brow. What makes a brow
    /// read is not the bar: it is the ledge and the hollow UNDER it that the eye
    /// sits in, and a hollow is not something a solid added to a surface can do
    /// at all.
    fn brow(&self, local: Vec3) -> f32 {
        let unit = self.unit;
        let weight = self.params.brow;
        // Heights in the frame, widths and reaches in the unit (#77) — and the
        // height fractions were then multiplied by 0.6717 when #78 lengthened
        // the frame by half, because every one of these was fitted by eye
        // against a render and each already lands near life: the crest ends up
        // 18.2 mm above the eye line, where a brow ridge is about 20. A fraction
        // fitted against a frame that was 39% short does not survive that frame
        // being corrected; the millimetres do.
        let rise = self.frame * (0.1394 + 0.0674 * weight);
        let span = unit * 0.8536;
        let thick = self.frame * (0.0764 + 0.0360 * weight);
        let reach = unit * (0.1039 + 0.1336 * weight);

        // Zero over the pupil, negative toward the midline, positive outward.
        let side = (local.x.abs() - self.apart) / span;
        if !(-1.15..=1.15).contains(&side) {
            return 0.0;
        }
        // The arch, which is what stops two brows reading as one bar: highest
        // just outside the pupil and falling at both ends, the outer end lower.
        let arch = ramp(
            &[
                (-1.15, 0.62),
                (-0.40, 0.94),
                (0.15, 1.00),
                (0.70, 0.84),
                (1.15, 0.52),
            ],
            side,
        );

        // Up the face through the ridge: the socket below, the crest, and back
        // to the forehead above.
        let up = (local.y - (self.level + rise * arch)) / thick;
        if !(-2.60..=1.60).contains(&up) {
            return 0.0;
        }
        // The crest, and the socket under it that the eye sits in.
        let ledge = bump(up, 0.0, 0.70) - 0.46 * bump(up, -1.45, 0.55);

        let ends =
            smooth(((side + 1.15) / 0.35).min(1.0)) * smooth(((1.15 - side) / 0.35).min(1.0));
        reach * ledge * ends * smooth((2.60 - up.abs()) / 0.7)
    }

    /// Two lips with a line between them, and the philtrum above.
    ///
    /// A mouth modelled as one bar has no line across it and the line is the
    /// whole feature; modelled as two solids it has a line and also two hard
    /// boundaries where each solid meets the face. As a displacement the line is
    /// a groove in the same surface, which is what it is on a person.
    fn mouth(&self, local: Vec3) -> f32 {
        let unit = self.unit;
        let full = self.params.mouth;
        let half = unit * (0.6829 + 0.1188 * full);
        let plump = self.plump;
        // Lips stand about five millimetres off the face around them, and this
        // is that. It was nearly ten, and at ten the profile below has to swing
        // through its whole range inside a single cell — which does not draw a
        // lip line, it draws a terrace. The mouth came out as a stack of
        // horizontal bars, and no amount of re-authoring the knots fixed it
        // while the amplitude was the thing at fault.
        let reach = unit * (0.1410 + 0.1113 * full);

        let across = local.x.abs() / half;
        // The mouth line is not level: the corners sit lower than the middle,
        // and a mouth drawn straight across reads as a slot.
        let line = self.mouth - self.frame * 0.0292 * across * across;
        let up = (local.y - line) / plump;

        let lips = if across > 1.05 || !(-2.40..=2.20).contains(&up) {
            0.0
        } else {
            // Lower lip, upper lip, the line between them, and the crease under
            // the lower lip that separates it from the chin. The line is a
            // groove in one surface rather than a seam between two pieces,
            // which is what it is on a person.
            //
            // **Each lobe carries its own taper across the face**, which is what
            // stops the pair reading as two stacked slabs — see [`LIPS`]. The
            // gate above is where the widest of them, the groove, has already
            // reached zero, so it cuts nothing.
            let profile: f32 = LIPS
                .iter()
                .map(|&(weight, centre, width, across_the_face)| {
                    weight * bump(up, centre, width) * across_the_face.taper(across)
                })
                .sum();
            let ends = smooth((2.40 - up.abs()) / 0.6);
            profile * ends
        };

        // The philtrum: the groove from the base of the nose to the bow of the
        // upper lip. Small, and one of the two or three things that most says a
        // face was modelled rather than assembled.
        //
        // **No gate, a bump in its own coordinate.** This used to be bracketed
        // by `if local.y > top && local.y < self.base`, and at `top` the groove
        // was at full depth — `down` is 1 there — so it fell to nothing across
        // one row of vertices: a 2.1 mm cliff on the default and 3.1 on seed
        // 99, in a window about one cell tall (#84). A term with a boundary can
        // grow a step back the moment a neighbouring constant moves it; a term
        // with no boundary cannot. The philtrum is deepest in the middle and
        // fades into the nose base above and the bow of the lip below, which is
        // what it does on a face anyway.
        let top = line + plump * 0.62;
        let middle = 0.5 * (top + self.base);
        let half = (self.base - top).abs().max(f32::EPSILON) * 0.5;
        let wide = unit * 0.1930;
        let sides = 1.0 - (local.x.abs() / wide).min(1.0);
        let groove = -0.34 * sides * sides * bump((local.y - middle) / half, 0.0, 0.62);

        reach * (lips + groove)
    }

    /// A nose: a bridge from between the brows, a tip, and two wings.
    fn nose(&self, local: Vec3) -> f32 {
        let unit = self.unit;
        // How far a nose stands off the face it is on. About 20 mm on a person,
        // and this lands near it: the axis moves it by half again either way
        // rather than by the factor that would be needed to make a coarse
        // surface show it.
        let reach = unit * (0.3340 + 0.3712 * self.params.nose);

        // Rebased by 0.6717 with every other height when #78 lengthened the
        // frame: the nose holds its 49 mm from root to under, which is life's
        // nasion-to-subnasale. At the old fractions it would have grown to 74.
        let root = self.level + self.frame * 0.1237;
        let under = self.base - self.frame * 0.0674;
        let along = (root - local.y) / (root - under);
        // **Outside its own span, not clamped to the end of it.** A ramp read
        // with a clamped parameter holds its first value forever, so a nose
        // whose bridge starts at 0.12 of its reach put a 0.12 ridge up the
        // forehead and over the crown — plainly visible in a render and
        // invisible in every number, since the field was doing exactly what it
        // was asked at every point inside the nose.
        if !(0.0..=1.0).contains(&along) {
            return 0.0;
        }

        // Down the midline: nothing at the brow, deepening through the bridge,
        // fullest just above the base, and gone under it. Both ends are zero so
        // the nose begins and ends rather than stepping.
        let height = ramp(
            &[
                (0.00, 0.00),
                (0.22, 0.34),
                (0.55, 0.62),
                (0.80, 1.00),
                (0.92, 0.86),
                (1.00, 0.00),
            ],
            along,
        );
        // Across: a narrow bridge opening into the wings, about one eye-width
        // at the nostrils, per the canon of fifths.
        let half = unit
            * ramp(
                &[
                    (0.00, 0.2227),
                    (0.35, 0.1930),
                    (0.75, 0.2969),
                    (0.92, 0.3860),
                ],
                along,
            );

        let across = local.x.abs() / half;
        // A rounded ridge rather than a blade. Squared and subtracted gives a
        // parabola whose sides fall away too fast to read as a nose from the
        // front; the exponent puts the shoulder back on it.
        let section = (1.0 - across * across).max(0.0).powf(0.65);

        // The crease where a wing meets the cheek. Narrow, negative, and only
        // down at the wings, which is the whole of what makes a nostril read as
        // a nostril rather than as the end of a bump.
        //
        // **It fades out at the bottom as well as the top**, and it did not.
        // The gate above returns zero outside the nose's span, and this term
        // used to arrive there at FULL amplitude — so the field fell 0.16 of
        // the nose's reach across one row of vertices, 2.5 mm on the default
        // and 3.9 on seed 99, drawing the topmost of the sub-nasal ledges
        // (#84). A term that ends inside its own window cannot step out of it.
        let wing = if along > 0.68 {
            let outside = (across - 1.0).abs();
            let held =
                smooth(((along - 0.68) / 0.20).min(1.0)) * smooth(((1.0 - along) / 0.14).min(1.0));
            -0.16 * (1.0 - (outside / 0.45).min(1.0)).powi(2) * held
        } else {
            0.0
        };

        reach * (height * section + wing)
    }
}

/// Reads a piecewise-linear curve given from low to high.
///
/// The mirror of [`super::skull`]'s profile reader, which runs the other way
/// because a skull is described from its crown down and a feature from its own
/// top down. Kept separate rather than shared and flipped: two readers that
/// disagree about which end they start from is exactly the kind of thing that
/// looks correct in both files.
fn ramp(curve: &[(f32, f32)], at: f32) -> f32 {
    let Some(&(first, low)) = curve.first() else {
        return 0.0;
    };
    if at <= first {
        return low;
    }
    for pair in curve.windows(2) {
        let ((before, under), (after, over)) = (pair[0], pair[1]);
        if at <= after {
            let along = (at - before) / (after - before).max(f32::EPSILON);
            return under + (over - under) * along;
        }
    }
    curve.last().map_or(0.0, |&(_, high)| high)
}

/// A smooth bump, one at `centre` and falling away over `width`.
///
/// **Why the profiles here are not all piecewise-linear like the skull's.**
/// [`ramp`] has a slope that jumps at every knot. That is invisible where a span
/// holds several cells and unmissable where it holds one, and a mouth is the
/// second case: both lips, the line between them and the crease below span about
/// 25 mm on a surface with 3.4 mm cells, so the knots land roughly a cell apart
/// and each slope change lands on its own row of quads. The mouth came out as a
/// stack of horizontal bars. Re-authoring the knots did not help and could not
/// have — a Gaussian has no knots to alias.
fn bump(at: f32, centre: f32, width: f32) -> f32 {
    let along = (at - centre) / width.max(f32::EPSILON);
    (-along * along).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CageConfig;
    use crate::face::skull::Skull;

    /// A body's uncarved head, its canon, and the rig that built it.
    fn measured(record: &crate::AvatarRecord) -> (PolyMesh, Rig, Canon) {
        let skeleton = record.skeleton();
        let plain = crate::build_body(&skeleton, &CageConfig::default(), crate::BODY_SUBDIVISIONS)
            .expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let skull = Skull::measure(&plain, &rig).expect("a skull");
        let canon = Canon::measure(&rig, &skull, &record.eyes);
        (plain, rig, canon)
    }

    /// A default head, before and after carving.
    fn head() -> (PolyMesh, PolyMesh, Rig, Vec3) {
        let record = crate::AvatarRecord::new("Carved", crate::Archetype::default());
        let (plain, rig, canon) = measured(&record);
        let mut carved = plain.clone();
        carve(&mut carved, &rig, &canon, &FaceParams::default());
        let centre = rig.joints[canon.head].position;
        (plain, carved, rig, centre)
    }

    #[test]
    fn the_relief_field_has_no_cliffs_in_it() {
        // One assertion for every gated term in the file, present and future,
        // which a per-feature test cannot be. A field with a hard `if` in it
        // steps by the term's full amplitude across one row of vertices, and
        // that is a ledge on the face — the owner reported the lower third as
        // a stack of horizontal bars (#84).
        //
        // **This measures the FIELD, not the surface**, and the distinction is
        // the difference between the two defects tangled together here. A
        // discontinuity is in the field and shows up at any sampling; ALIASING
        // is in the relationship between a smooth field and the mesh under it —
        // the lip line's Gaussian is 0.99 cells wide — and this test is blind
        // to it by construction. That half is still open (#82).
        //
        // Sampled at 0.1 mm down the face, an order finer than the 3.6 mm cells,
        // so the steepest honest gradient in the field (the nose's tail, about
        // 1.33) contributes 0.13 mm per step while a gate contributes its whole
        // amplitude. Measured before the gates were fixed: 2.63 mm.
        let record = crate::AvatarRecord::new("Cliffs", crate::Archetype::default());
        let (_, _, canon) = measured(&record);
        let face = Face::new(&canon, &FaceParams::default());

        // Asked by HALVING THE STEP, like the profile test in `super::skull`,
        // because a threshold cannot tell a cliff from a steep slope. A true
        // discontinuity gives the same jump however finely it is sampled; a
        // steep but continuous feature halves with the step.
        //
        // The measured ratios say exactly what the field is made of. A gate
        // gives 1.00 — both gates did, before this issue. A smooth feature
        // gives 0.50. What is left here gives **0.65 at the nose's flank**, and
        // that figure is not arbitrary: it is 2^-0.65, the signature of the
        // `powf(0.65)` shoulder that rounds the nose's section. So the worst
        // remaining thing in the field is a cusp with an infinite derivative
        // rather than a step — continuous, and it is where a nose wing meets a
        // cheek, which on a face is a crease. It will still alias against a
        // 3.6 mm cell, and that half of the problem is #82's, not this one's.
        let unit = canon.unit;
        let (top, bottom) = (face.level + unit * 1.2, canon.chin() - unit * 0.4);
        let worst_jump = |step: f32| {
            let mut worst = (0.0f32, Vec3::ZERO);
            let mut across = -unit * 2.0;
            while across <= unit * 2.0 {
                let mut height = bottom;
                while height <= top {
                    let here = Vec3::new(across, height, 0.0);
                    let jump = (face.lift(here + Vec3::Y * step) - face.lift(here)).abs();
                    if jump > worst.0 {
                        worst = (jump, here);
                    }
                    height += step;
                }
                across += 0.0005;
            }
            worst
        };
        let coarse = worst_jump(0.0002).0;
        let (fine, at) = worst_jump(0.0001);
        assert!(
            coarse > f32::EPSILON && fine / coarse < 0.75,
            "the relief field's worst step is {:.3} mm sampled coarsely and {:.3} mm \
             sampled twice as finely, at ({:.1}, {:.1}) mm — a ratio of {:.2}. A steep \
             slope halves; a cliff does not care.",
            coarse * 1000.0,
            fine * 1000.0,
            at.x * 1000.0,
            at.y * 1000.0,
            fine / coarse
        );
    }

    #[test]
    fn the_mouth_is_wider_than_the_mesh_under_it() {
        // A RATIO, not a millimetre figure. The cell size moves with
        // FACE_REFINEMENT and with the size of the head, so a constant here
        // would be the next thing in this codebase named for one quantity and
        // holding another.
        //
        // Measured: at three refinement passes the lip line's groove was 0.99
        // cells and the mouth rendered as a stack of horizontal bars. The
        // fourth pass covers the mouth band alone and takes it to about 2.0
        // (#85). Below about 1.5 a Gaussian cannot survive being sampled.
        let mut worst: Vec<(Option<i64>, f32, f32, f32)> = Vec::new();
        for seed in [None, Some(7), Some(29), Some(99)] {
            let mut record = crate::AvatarRecord::new("Mouth", crate::Archetype::default());
            if let Some(seed) = seed {
                record.reroll(seed);
            }
            let (mesh, rig, canon) = measured(&record);
            let face = Face::new(&canon, &record.face);
            let centre = rig.joints[canon.head].position;

            // The median edge of the faces the mouth actually sits on.
            let mut edges: Vec<f32> = Vec::new();
            for index in 0..mesh.face_count() {
                let at = mesh.face_centroid(index) - centre;
                if at.z <= 0.0 || (at.y - face.mouth).abs() > face.plump * 1.4 {
                    continue;
                }
                let ring = &mesh.faces[index];
                for corner in 0..ring.len() {
                    let next = (corner + 1) % ring.len();
                    edges.push(
                        mesh.positions[ring[corner] as usize]
                            .distance(mesh.positions[ring[next] as usize]),
                    );
                }
            }
            assert!(!edges.is_empty(), "seed {seed:?}: no faces under the mouth");
            edges.sort_by(f32::total_cmp);
            let cell = edges[edges.len() / 2];
            worst.push((seed, face.finest(), cell, face.finest() / cell));
        }

        // Every seed reported, not the first to fail. The ratio moves with the
        // head's size, the mouth axis and the refinement band all at once, so
        // one seed's number says almost nothing about where the margin is.
        let table: Vec<String> = worst
            .iter()
            .map(|(seed, finest, cell, ratio)| {
                format!(
                    "{seed:?}: {:.2} mm term / {:.2} mm cell = {ratio:.2}",
                    finest * 1000.0,
                    cell * 1000.0
                )
            })
            .collect();
        let tightest = worst.iter().fold(f32::MAX, |low, entry| low.min(entry.3));
        assert!(
            tightest > 1.5,
            "the narrowest term in the mouth is {tightest:.2} cells wide, and a Gaussian \
             that narrow renders as one displaced row of vertices, which is a bar — {}",
            table.join("; ")
        );
    }

    #[test]
    fn carving_leaves_the_topology_alone() {
        // Vertices move; nothing is added, removed or re-joined. That is the
        // whole point of doing this as a displacement — the face is the body's
        // own surface, so it is bound, charted and painted as one thing.
        let (plain, carved, ..) = head();
        assert_eq!(plain.vertex_count(), carved.vertex_count());
        assert_eq!(plain.faces, carved.faces);
        assert!(
            carved.is_closed_manifold(),
            "{:?}",
            carved.manifold_report()
        );
    }

    #[test]
    fn nothing_behind_the_face_moves() {
        // The fields are bounded in x and y, but a field that had not quite
        // reached zero would put a ripple round the back of the head, and a
        // ripple on an occiput is very hard to see and very easy to ship.
        let (plain, carved, _, centre) = head();
        for (was, now) in plain.positions.iter().zip(&carved.positions) {
            if was.z - centre.z > 0.0 {
                continue;
            }
            assert!(
                was.distance(*now) < 1e-6,
                "a vertex behind the face moved {:.2} mm",
                was.distance(*now) * 1000.0
            );
        }
    }

    #[test]
    fn a_nose_stands_off_the_face_it_is_carved_into() {
        // The measurement that says a nose exists: how far the surface on the
        // midline moved, against how far it moved a nose-width to the side.
        let (plain, carved, ..) = head();
        let record = crate::AvatarRecord::new("Nosed", crate::Archetype::default());
        let (_, rig, canon) = measured(&record);
        let centre = rig.joints[canon.head].position;
        let unit = canon.unit;

        let moved = |low: f32, high: f32, near: f32, far: f32| {
            plain
                .positions
                .iter()
                .zip(&carved.positions)
                .filter(|(was, _)| {
                    let local = **was - centre;
                    local.y > low && local.y < high && local.x.abs() >= near && local.x.abs() < far
                })
                .map(|(was, now)| was.distance(*now))
                .fold(0.0f32, f32::max)
        };

        let level = canon.level;
        let bridge = moved(level - unit * 1.4, level, 0.0, unit * 0.4);
        let cheek = moved(level - unit * 1.4, level, unit * 1.6, unit * 3.0);
        assert!(
            bridge > unit * 0.30,
            "the nose only stands {:.1} mm off the face",
            bridge * 1000.0
        );
        assert!(
            cheek < bridge * 0.25,
            "the nose spread onto the cheek: {:.1} mm against {:.1}",
            cheek * 1000.0,
            bridge * 1000.0
        );
    }

    #[test]
    fn the_carve_leaves_the_jaw_to_the_skull() {
        // Written from the defect that reached the owner twice (#71, #72). The
        // feature frame used to end at `span().0` — the THROAT — so the whole
        // stack sat a third of a storey too low: the lower lip was painted onto
        // the chin's own tip (+6.8 mm at −63 on the default) and the crease
        // under the lip was carved into the underside of the jaw (−2.7 mm at
        // −75). Material added above the tip and removed below it reads as the
        // jaw rotated up into the throat, which is exactly how it was reported.
        //
        // So the assertion is about territory rather than about a margin: below
        // the chin the face belongs to the skull profile, and the carve keeps
        // its hands off it. At the tip itself the sulcus tail may graze — about
        // a millimetre, measured — but nothing like a lip's worth.
        //
        // Every seed, which the old frame could not survive: this replaced a
        // default-only test whose "lip" band on seed 99 held the side of a
        // nose.
        for seed in [
            None,
            Some(1),
            Some(7),
            Some(23),
            Some(29),
            Some(42),
            Some(99),
        ] {
            let mut record = crate::AvatarRecord::new("Jaw", crate::Archetype::default());
            if let Some(seed) = seed {
                record.reroll(seed);
            }
            let (plain, rig, canon) = measured(&record);
            let mut carved = plain.clone();
            carve(&mut carved, &rig, &canon, &record.face);

            let unit = canon.unit;
            let chin = canon.chin();
            let centre = rig.joints[canon.head].position;
            for (was, now) in plain.positions.iter().zip(&carved.positions) {
                let height = was.y - centre.y;
                let moved = was.distance(*now) * 1000.0;
                if height < chin - unit * 0.35 {
                    assert!(
                        moved < 1.0,
                        "seed {seed:?}: the carve moved the underside of the jaw \
                         {moved:.1} mm at {:.1} mm below the chin",
                        (chin - height) * 1000.0
                    );
                } else if height < chin + unit * 0.15 {
                    assert!(
                        moved < 2.5,
                        "seed {seed:?}: the carve moved the chin's tip {moved:.1} mm"
                    );
                }
            }
        }
    }

    #[test]
    fn a_more_prominent_nose_stands_further_out() {
        let record = crate::AvatarRecord::new("Nosier", crate::Archetype::default());
        let (plain, rig, canon) = measured(&record);

        // In the nose's own band, not over the whole mesh. This asserted against
        // `bounds().1.z` and started failing the moment the chin was pulled back
        // to where a chin belongs (#71) — because the furthest-forward point of
        // a head is its BROW, and a bounding box asked about a nose answers
        // about a brow. It had been passing for the same reason it then failed:
        // by accident.
        let centre = rig.joints[canon.head].position;
        let level = canon.level;
        let unit = canon.unit;
        let reach = |nose: f32| {
            let mut mesh = plain.clone();
            carve(
                &mut mesh,
                &rig,
                &canon,
                &FaceParams {
                    nose,
                    ..Default::default()
                },
            );
            mesh.positions
                .iter()
                .map(|point| *point - centre)
                .filter(|local| {
                    local.x.abs() < unit * 0.6
                        && local.y < level - unit * 0.4
                        && local.y > level - unit * 2.4
                })
                .fold(f32::MIN, |far, local| far.max(local.z))
        };
        assert!(
            reach(1.0) > reach(0.0) + 0.004,
            "the whole nose axis moved the nose by under 4 mm: {:.1} against {:.1}",
            reach(1.0) * 1000.0,
            reach(0.0) * 1000.0
        );
    }
}
