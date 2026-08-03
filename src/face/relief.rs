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

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

use super::eye::Eyes;
use super::features::FaceParams;
use super::skull::Skull;

/// How far round the head a feature can reach before it is faded out.
///
/// A cosine of the angle from dead ahead. Every field below is bounded in `x`
/// and `y` already, so this is a backstop rather than a shape: without it a
/// nose whose lateral falloff has not quite reached zero by the ear puts a
/// ripple on the side of the head.
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
pub fn carve(mesh: &mut PolyMesh, rig: &Rig, eyes: &Eyes, params: &FaceParams) {
    let Some(skull) = Skull::measure(mesh, rig) else {
        return;
    };
    let centre = rig.joints[skull.head].position;
    let face = Face::new(eyes, &skull, params);

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
struct Face {
    /// An eye's radius, which the proportion canons are expressed in.
    unit: f32,
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
    fn new(eyes: &Eyes, skull: &Skull, params: &FaceParams) -> Self {
        let level = eyes.left.pivot.y;
        // The chin's tip, NOT `span().0`. The span ends at the throat, 28 mm
        // below the chin on a default body, and a frame stretched to the throat
        // put the whole feature stack a third of a storey too low: the lower
        // lip landed on the chin's own tip and the crease under it was carved
        // into the underside of the jaw, which read as the jaw rotated up into
        // the throat (#72).
        let chin = skull.chin();
        let unit = eyes.left.radius;
        Self {
            unit,
            level,
            // The same fraction of the eye-line-to-chin span `super::features`
            // counts in, so a nose carved here and an ear placed there agree
            // about where the middle third of the face ends.
            base: level + (chin - level) * super::features::NOSE_BASE,
            mouth: level + (chin - level) * super::features::MOUTH_HEIGHT,
            // Sized by the eye, like everything else — but never deeper than
            // the face has room for. The eye and the face length are separate
            // axes, and on a large-eyed short-faced head an uncapped lip stack
            // reached past the chin: seed 99 put the sulcus lobe at −69.1 mm
            // against a chin at −68.7, carving the crease into the tip itself.
            // The lowest lobe sits at 1.32 plumps below the mouth line and the
            // chin 0.31 frames below it, so 0.20 keeps the whole stack above
            // the tip with about 0.05 frames to spare, on every head.
            plump: (unit * (0.46 + 0.24 * params.mouth)).min(0.20 * (level - chin)),
            apart: eyes.right.pivot.x.abs(),
            params: *params,
        }
    }

    /// How far the surface stands out at a point, in metres.
    ///
    /// Summed, not maximised. Features that meet — the wing of a nose against
    /// the philtrum below it, the brow against the socket — have to add up
    /// across the join or there is a crease where one field wins.
    fn lift(&self, local: Vec3) -> f32 {
        self.nose(local) + self.brow(local) + self.mouth(local)
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
        let rise = unit * (0.62 + 0.30 * weight);
        let span = unit * 1.15;
        let thick = unit * (0.34 + 0.16 * weight);
        let reach = unit * (0.14 + 0.18 * weight);

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
        let half = unit * (0.92 + 0.16 * full);
        let plump = self.plump;
        // Lips stand about five millimetres off the face around them, and this
        // is that. It was nearly ten, and at ten the profile below has to swing
        // through its whole range inside a single cell — which does not draw a
        // lip line, it draws a terrace. The mouth came out as a stack of
        // horizontal bars, and no amount of re-authoring the knots fixed it
        // while the amplitude was the thing at fault.
        let reach = unit * (0.19 + 0.15 * full);

        let across = local.x.abs() / half;
        // The mouth line is not level: the corners sit lower than the middle,
        // and a mouth drawn straight across reads as a slot.
        let line = self.mouth - unit * 0.13 * across * across;
        let up = (local.y - line) / plump;

        let lips = if across > 1.20 || !(-2.40..=2.20).contains(&up) {
            0.0
        } else {
            // Lower lip, upper lip, the line between them, and the crease under
            // the lower lip that separates it from the chin. The line is a
            // groove in one surface rather than a seam between two pieces,
            // which is what it is on a person.
            let profile = 0.88 * bump(up, -0.60, 0.46) + 0.82 * bump(up, 0.58, 0.44)
                - 0.44 * bump(up, 0.00, 0.26)
                - 0.24 * bump(up, -1.32, 0.34);
            let corner = smooth(((1.20 - across) / 0.30).min(1.0));
            let ends = smooth((2.40 - up.abs()) / 0.6);
            profile * corner * ends
        };

        // The philtrum: the groove from the base of the nose to the bow of the
        // upper lip. Small, and one of the two or three things that most says a
        // face was modelled rather than assembled.
        let top = line + plump * 0.62;
        let groove = if local.y > top && local.y < self.base {
            let down = ((self.base - local.y) / (self.base - top)).clamp(0.0, 1.0);
            let wide = unit * 0.26;
            let sides = 1.0 - (local.x.abs() / wide).min(1.0);
            -0.34 * sides * sides * smooth(down)
        } else {
            0.0
        };

        reach * (lips + groove)
    }

    /// A nose: a bridge from between the brows, a tip, and two wings.
    fn nose(&self, local: Vec3) -> f32 {
        let unit = self.unit;
        // How far a nose stands off the face it is on. About 20 mm on a person,
        // and this lands near it: the axis moves it by half again either way
        // rather than by the factor that would be needed to make a coarse
        // surface show it.
        let reach = unit * (0.45 + 0.50 * self.params.nose);

        let root = self.level + unit * 0.55;
        let under = self.base - unit * 0.30;
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
                &[(0.00, 0.30), (0.35, 0.26), (0.75, 0.40), (0.92, 0.52)],
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
        let wing = if along > 0.68 {
            let outside = (across - 1.0).abs();
            -0.16 * (1.0 - (outside / 0.45).min(1.0)).powi(2) * ((along - 0.68) / 0.32).min(1.0)
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

/// Smoothstep, for fading a field out without leaving a crease where it ends.
fn smooth(at: f32) -> f32 {
    let at = at.clamp(0.0, 1.0);
    at * at * (3.0 - 2.0 * at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::EyeParams;
    use crate::{CageConfig, HumanoidParams, plan::BodyPlan};

    /// A default head, before and after carving.
    fn head() -> (PolyMesh, PolyMesh, Rig, Vec3) {
        let skeleton = HumanoidParams::default().skeleton();
        let plain = crate::build_body(&skeleton, &CageConfig::default(), 2).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let eyes = Eyes::build(&rig, &EyeParams::default()).expect("a humanoid has eyes");
        let mut carved = plain.clone();
        carve(&mut carved, &rig, &eyes, &FaceParams::default());
        let centre = rig.joints[eyes.head].position;
        (plain, carved, rig, centre)
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
        let (plain, carved, rig, centre) = head();
        let eyes = Eyes::build(&rig, &EyeParams::default()).expect("eyes");
        let unit = eyes.left.radius;

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

        let level = eyes.left.pivot.y;
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
            let skeleton = record.skeleton();
            let plain = crate::build_body(&skeleton, &CageConfig::default(), 2).expect("meshes");
            let rig = Rig::from_skeleton(&skeleton).expect("rigs");
            let eyes = Eyes::build(&rig, &record.eyes).expect("eyes");
            let skull = Skull::measure(&plain, &rig).expect("a skull");
            let mut carved = plain.clone();
            carve(&mut carved, &rig, &eyes, &record.face);

            let unit = eyes.left.radius;
            let chin = skull.chin();
            let centre = rig.joints[eyes.head].position;
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
        let skeleton = HumanoidParams::default().skeleton();
        let plain = crate::build_body(&skeleton, &CageConfig::default(), 2).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let eyes = Eyes::build(&rig, &EyeParams::default()).expect("eyes");

        // In the nose's own band, not over the whole mesh. This asserted against
        // `bounds().1.z` and started failing the moment the chin was pulled back
        // to where a chin belongs (#71) — because the furthest-forward point of
        // a head is its BROW, and a bounding box asked about a nose answers
        // about a brow. It had been passing for the same reason it then failed:
        // by accident.
        let centre = rig.joints[eyes.head].position;
        let level = eyes.left.pivot.y;
        let unit = eyes.left.radius;
        let reach = |nose: f32| {
            let mut mesh = plain.clone();
            carve(
                &mut mesh,
                &rig,
                &eyes,
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
