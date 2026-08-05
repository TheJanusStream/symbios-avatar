//! Feet.
//!
//! A foot is the part of a body that touches the world, so it is the part whose
//! wrongness shows up in motion rather than in a still: a rounded stub has no
//! sole to plant, and a leg ending in one reads as stilts however good the gait
//! driving it is.
//!
//! Built, like a hand, as a part attached at the ankle rather than as more nodes
//! in the capsule graph. The shape is the three widths a foot actually has —
//! narrow at the heel, widest at the ball, tapering again to the toe — which
//! [`crate::prim::ribbon`] cannot express and [`crate::prim::sweep`] can. It is
//! also the reason a foot is not just a longer stub: the width has to come and
//! go, not run from one end to the other.
//!
//! The sole is flat and level. Everything that plants a foot on ground assumes
//! it, and a curved sole would rock.

use glam::{Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::prim;

/// Where along the foot the ankle sits, from the heel.
///
/// Well back. A foot balanced on its middle reads as a flipper, and every
/// picture of a walk depends on there being more foot in front of the joint
/// than behind it.
///
/// Provenance: **looked up**, from the Quaternius reference bodies — the ankle
/// joint sits 16% along the foot on the male and 19% on the female, measured
/// from the heel as a fraction of foot length (#110). It was 0.28 before, from a
/// first guess, which carried nearly twice the heel a foot has and left too
/// little in front of the joint for the toe to roll over.
const ANKLE_AT: f32 = 0.18;

/// The foot's cross-section, in half-width and half-depth, going the way
/// [`prim::sweep_outline`] winds: `x` across the foot, `y` **toward the sole**.
///
/// **A foot rests on a sole, and an ellipse has no sole.** The ring this used to
/// be put a vertex straight down, so the foot touched the ground along its centre
/// line and rose away from it to either side — measured on the built mesh by
/// ray-cast, 0.0 mm at the centre, 6 mm at the quarter width and up to 19 mm at
/// the edges. Both Quaternius bodies are flat across the whole sole to within a
/// few millimetres, and a foot that contacts along a line rocks under everything
/// that plants it (#110).
///
/// So the sole is three points at full depth — dead flat across the middle 90% of
/// the width — with the edges rolled up over two more, and the instep an ellipse
/// above. Twelve points, because ten cannot both hold the flat and round the top,
/// and because the foot is 5 stations long so this is 60 vertices either way.
///
/// Provenance: **derived** from the sole flatness measured on the reference — the
/// rolled corners sit 5% of the half-depth above the sole plane, which on a
/// default body is about a millimetre, inside the 0.3–8.9 mm the references
/// measure across their own soles.
const SECTION: [(f32, f32); 12] = [
    (1.00, 0.00),
    (0.98, 0.55),
    (0.80, 0.95),
    (0.45, 1.00),
    (0.00, 1.00),
    (-0.45, 1.00),
    (-0.80, 0.95),
    (-0.98, 0.55),
    (-1.00, 0.00),
    (-0.75, -0.72),
    (0.00, -1.00),
    (0.75, -0.72),
];

/// Half-widths along the foot, as fractions of its length.
///
/// Narrow at the heel, widest at the ball, narrowing again to the toe — which is
/// the whole reason a foot is a sweep rather than a tapered tube.
///
/// Provenance: **looked up**, read off the Quaternius male's own sole outline at
/// the five stations below. Its half-width runs 0.149 of foot length at the heel,
/// 0.161 a tenth along, 0.185–0.186 across the ball, 0.174 at four fifths and
/// 0.142 at nine tenths (#110). The previous values peaked at 0.21, which made
/// the foot 42% of its length wide against a measured 37–38%.
const WIDTHS: [f32; 5] = [0.150, 0.161, 0.185, 0.175, 0.130];

/// A built foot, in ankle-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Foot {
    /// The whole foot as one solid.
    pub mesh: PolyMesh,
    /// How far the foot reaches forward of the ankle, in metres.
    pub length: f32,
    /// How far the sole sits below the ankle joint, in metres.
    pub drop: f32,
}

impl Foot {
    /// Builds a foot of the given `length`, whose sole sits `drop` below the
    /// joint it hangs from.
    ///
    /// Both come from the body rather than from the ankle's girth. A foot's
    /// depth is the distance from the ankle to the ground, which is a fact about
    /// where the body plan put the ankle; guessing it from the ankle's thickness
    /// gave a wafer that the leg's own rounded tip poked straight through.
    ///
    /// `forward` is the way the toes point and `up` is away from the ground.
    /// Both come from the rig, so a foot that is toed out slightly stays that
    /// way and nothing here has to know which leg it is on.
    #[must_use]
    pub fn build(ankle: f32, forward: Vec3, up: Vec3, length: f32, drop: f32) -> Self {
        let up = up.normalize_or(Vec3::Y);
        let forward = (forward - up * forward.dot(up)).normalize_or(Vec3::Z);
        let across = forward.cross(up);
        let heel = length * ANKLE_AT;

        // Narrow at the heel, widest at the ball, tapering again to the toe.
        // Depth is constant, and the path runs dead straight and level: a sweep
        // turns its rings to follow the path, so a path that rose and fell to
        // shape the instep would tilt every ring and take the sole with it.
        let toe = length - heel;
        let reach = [-heel, -heel * 0.35, toe * 0.45, toe * 0.8, toe];

        let half = drop * 0.5;

        let centre = -up * half;
        let path: Vec<Vec3> = reach.iter().map(|&at| centre + forward * at).collect();
        // Widths scale with the foot's length, not the ankle's girth: a foot is
        // wider than the leg above it, and sizing it off the ankle gave a sole
        // narrower than the shin that stood on it. The girth still sets a floor,
        // for a body whose ankle is thicker than its foot is long.
        let outlines: Vec<Vec<Vec2>> = WIDTHS
            .iter()
            .map(|&wide| {
                let wide = (length * wide).max(ankle * 0.5);
                SECTION
                    .iter()
                    .map(|&(x, y)| Vec2::new(x * wide, y * half))
                    .collect()
            })
            .collect();

        let mut mesh = prim::sweep_outline(&path, &outlines, across);

        // The instep is shaped afterwards, by pressing the top of the slab down
        // toward the sole. Measuring the squash from the sole means the sole
        // cannot move: a vertex already on it has nothing to scale.
        let floor = -drop;
        for point in &mut mesh.positions {
            let at = (point.dot(forward) + heel) / length;
            let above = point.dot(up) - floor;
            let squashed = above * instep(at);
            *point += up * (floor + squashed - point.dot(up));
        }

        Self { mesh, length, drop }
    }
}

/// How deep the foot is at a point `at` along its length, as a share of the
/// heel's depth.
///
/// A foot is deepest where the ankle sits on it and shallowest at the toe. The
/// curve is gentle through the middle so the instep does not read as a step.
/// Provenance: **derived** from the reference toe. A Quaternius foot is 9.1%
/// (male) and 10.3% (female) of its own length thick at the toe, and a foot is
/// about three times its ankle height long — so the toe wants to come out near
/// 0.30 of the heel's depth. The tail was 0.42 before, leaving the toe at 0.58
/// and the foot a slab of nearly even thickness: a 1.6:1 wedge against the
/// references' 2.9:1 and 3.2:1 (#110).
fn instep(at: f32) -> f32 {
    let along = at.clamp(0.0, 1.0);
    // Gentle, and gentlest through the middle. Tapering hard from the ankle
    // forward thins the foot exactly where the leg's last node sits, and the
    // node then shows through the top of the foot it is supposed to be inside.
    1.0 - 0.70 * along * along * along
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A foot the crate would actually build.
    ///
    /// The length is three times the drop because that is the relation
    /// `grow_foot` uses, measured off both reference bodies (#110). It was 0.11
    /// against a 0.045 drop — a ratio of 2.4 — and a fixture with proportions the
    /// builder never produces is a fixture that answers about nothing. It also
    /// hid the sole: a foot that short comes out narrower than it is deep, which
    /// is the shape `a_foot_is_longer_than_it_is_wide_and_wider_than_it_is_deep`
    /// exists to rule out.
    const DROP: f32 = 0.045;

    fn foot() -> Foot {
        Foot::build(0.03, Vec3::Z, Vec3::Y, DROP * 3.0, DROP)
    }

    #[test]
    fn a_foot_is_a_closed_solid() {
        let foot = foot();
        assert!(
            foot.mesh.is_closed_manifold(),
            "{:?}",
            foot.mesh.manifold_report()
        );
    }

    #[test]
    fn the_sole_is_flat_across_its_width_and_not_a_keel() {
        // **The defect this file shipped with, and the reason `SECTION` exists.**
        // An elliptical ring puts a vertex straight down, so the foot rested on
        // its centre line and rose away to both sides: measured on the built body
        // by ray-cast, 0.0 mm at the centre, 6 mm at the quarter width and up to
        // 19 mm at the edges. A foot that touches along a line rocks under
        // everything that plants it, and nothing in this file said so (#110).
        //
        // **Asked as contact AREA, not as height spread**, because a height
        // spread measured over a band is a measurement of the band: filter the
        // vertices within a tenth of the drop and the worst of them is a tenth of
        // the drop, whatever the shape. So this takes a tight band — the height a
        // sole may vary by and still be one surface — and asks how far ACROSS the
        // foot it reaches. A keel answers with almost nothing, because its
        // neighbours climb away from the contact line immediately; a sole answers
        // with most of the width.
        let foot = foot();
        let (lo, hi) = foot.mesh.bounds();
        let band = DROP * 0.05;
        let contact: Vec<_> = foot
            .mesh
            .positions
            .iter()
            .filter(|p| p.y - lo.y < band)
            .collect();
        assert!(
            contact.len() >= 10,
            "only {} vertices lie on the sole",
            contact.len()
        );

        let across = contact.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        });
        let width = hi.x - lo.x;
        assert!(
            (across.1 - across.0) > width * 0.7,
            "the foot contacts the ground over {:.0}% of its width — a keel, not a sole",
            (across.1 - across.0) / width * 100.0
        );

        // And it is level along its length as well as across: a sole that tilts
        // plants on an edge.
        let ends = contact.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.z), hi.max(p.z))
        });
        assert!(
            (ends.1 - ends.0) > (hi.z - lo.z) * 0.7,
            "the sole reaches {:.0}% of the way from heel to toe",
            (ends.1 - ends.0) / (hi.z - lo.z) * 100.0
        );
    }

    #[test]
    fn a_foot_is_longer_than_it_is_wide_and_wider_than_it_is_deep() {
        let (lo, hi) = foot().mesh.bounds();
        let (long, wide, deep) = (hi.z - lo.z, hi.x - lo.x, hi.y - lo.y);
        assert!(long > wide * 1.5, "{long} long against {wide} wide");
        assert!(wide > deep, "{wide} wide against {deep} deep");
    }

    #[test]
    fn a_foot_reaches_forward_and_hangs_below_the_ankle() {
        let (lo, hi) = foot().mesh.bounds();
        assert!(hi.z > 0.0, "the toes did not point forward");
        assert!(lo.y < 0.0, "the sole did not sit below the joint");
        // Much more of it is in front of the ankle than behind.
        assert!(
            hi.z > -lo.z * 2.0,
            "{} forward against {} back",
            hi.z,
            -lo.z
        );
    }

    /// The widest half-width of each ring, from heel to toe.
    ///
    /// Rings are sparse, so sampling by a band of z misses most of them; group
    /// by the distinct heights the rings actually sit at instead.
    fn ring_widths(foot: &Foot) -> Vec<f32> {
        let mut rings: Vec<(f32, f32)> = Vec::new();
        for point in &foot.mesh.positions {
            match rings
                .iter_mut()
                .find(|(z, _)| (z - point.z).abs() < foot.length * 1e-3)
            {
                Some((_, wide)) => *wide = wide.max(point.x.abs()),
                None => rings.push((point.z, point.x.abs())),
            }
        }
        rings.sort_by(|a, b| a.0.total_cmp(&b.0));
        rings.into_iter().map(|(_, wide)| wide).collect()
    }

    #[test]
    fn the_ball_is_the_widest_part() {
        // Not the heel and not the toe. This is the whole reason a foot needs a
        // sweep with a section at every point rather than one taper: no
        // interpolation between two ends can say "widest in the middle".
        let foot = foot();
        let widths = ring_widths(&foot);
        assert!(widths.len() >= 5, "only {} rings", widths.len());
        let widest = widths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .expect("a foot has rings")
            .0;
        assert!(
            widest > 0 && widest < widths.len() - 1,
            "the widest ring was at the {widest} of {} — an end, not the ball",
            widths.len()
        );
    }

    #[test]
    fn the_sole_is_level_from_heel_to_toe() {
        // Everything that plants a foot on ground assumes this, and a sole that
        // dipped further at one end than the other would rock.
        let foot = foot();
        let mut lowest: Vec<(f32, f32)> = Vec::new();
        for point in &foot.mesh.positions {
            match lowest
                .iter_mut()
                .find(|(z, _)| (z - point.z).abs() < foot.length * 1e-3)
            {
                Some((_, low)) => *low = low.min(point.y),
                None => lowest.push((point.z, point.y)),
            }
        }
        assert!(lowest.len() >= 5, "only {} rings", lowest.len());
        let floor = lowest.iter().fold(f32::MAX, |a, b| a.min(b.1));
        let ceiling = lowest.iter().fold(f32::MIN, |a, b| a.max(b.1));
        assert!(
            ceiling - floor < foot.drop * 1e-3,
            "the sole varied by {} along the foot",
            ceiling - floor
        );
    }

    #[test]
    fn a_foot_follows_the_direction_it_is_given() {
        let toed_out = Foot::build(
            0.03,
            Vec3::new(0.4, 0.0, 1.0).normalize(),
            Vec3::Y,
            0.11,
            0.045,
        );
        let (lo, hi) = toed_out.mesh.bounds();
        assert!(
            hi.x > -lo.x,
            "a toed-out foot should reach further to one side"
        );
    }

    #[test]
    fn a_foot_takes_its_length_and_depth_from_the_body() {
        // Not from the ankle's girth. Sized off girth alone the foot came out a
        // wafer that the leg's own rounded tip poked through.
        let shallow = Foot::build(0.03, Vec3::Z, Vec3::Y, 0.11, 0.02);
        let deep = Foot::build(0.03, Vec3::Z, Vec3::Y, 0.11, 0.08);
        assert!((deep.drop / shallow.drop - 4.0).abs() < 1e-4);
        assert!(deep.mesh.bounds().0.y < shallow.mesh.bounds().0.y);

        // Twice the fixture's length, expressed against the fixture rather than
        // as a literal, so this keeps saying "the length passes through" when the
        // fixture's proportions move.
        let long = Foot::build(0.03, Vec3::Z, Vec3::Y, DROP * 6.0, DROP);
        assert!((long.length / foot().length - 2.0).abs() < 1e-4);
    }

    #[test]
    fn more_of_a_foot_lies_in_front_of_the_ankle_than_behind_it() {
        let (lo, hi) = foot().mesh.bounds();
        assert!(
            hi.z > -lo.z * 2.0,
            "{} forward against {} back",
            hi.z,
            -lo.z
        );
    }
}
