//! A beast's lower jaw and its tongue (#367): parts of their own, hinged so
//! the mouth can open.
//!
//! **A part, not a cut.** A person's mouth is a parting cut through one
//! closed surface (`face::mouth`) along a line across the front of the face.
//! A beast's runs back along both sides of the muzzle to the cheeks, and its
//! lower jaw is most of its lower face, so the jaw is built as a closed solid
//! of its own, standing up into the head from below, and the head's surface
//! stops at the upper lip. Shut, the line where the two surfaces cross is the
//! mouth's; open, the jaw turns down about its hinge and shows the palate -
//! painted as the inside of a mouth - and the tongue lying on its floor.
//!
//! **Its joints are not a marker chain, deliberately** (#367). The bridge's
//! talk and visemes find a person's jaw by its two markers
//! (`rig::skin::jaw_pivot`), and a beast's must not be opened by speech it
//! cannot make. Jaw and tongue ride [`Role::Facial`] joints, and
//! [`Jaw::pose`] is what opens them - what the pant, the yawn and the bark
//! of the motion work (#370) are to call.

use glam::{Mat4, Quat, Vec2, Vec3};

use crate::anim::Pose;
use crate::mesh::PolyMesh;
use crate::plan::BeastHead;
use crate::plan::beast_head::Cone;
use crate::prim;
use crate::rig::{Rig, Role};

/// How far the jaw opens at the top of [`Jaw::pose`]'s range, in radians: a
/// yawn's.
pub const MAX_OPEN: f32 = 0.65;

/// How far the tongue turns out over the lower lip at the top of its range,
/// in radians: a pant's.
pub const TONGUE_OUT: f32 = 0.7;

/// Where along the tongue it pivots, as a share of its length from the root:
/// near the lip, so turning it hangs its front out of the mouth and lifts
/// its root into the palate, where nothing shows it.
const TONGUE_PIVOT: f32 = 0.7;

/// The colour the inside of the mouth is tinted - the jaw's top and the
/// palate alike: a mouth is blood over darkness. Over flesh on the jaw, which
/// carries its inside as a mouth channel the coat painter reads (#368), and
/// over the coat on the palate, whose charts are the cheek's and the throat's.
pub const INSIDE: Vec3 = Vec3::new(0.32, 0.12, 0.12);

/// The tongue's colour, over the flesh the coat painter gives the whole
/// tongue (#368).
const TONGUE: Vec3 = Vec3::new(0.92, 0.48, 0.52);

/// Facets round the jaw and the tongue.
const SIDES: usize = 16;

/// How far inside the head a part of the jaw must lie before it is coloured
/// as the inside of a mouth, in head radii. A vertex's colour runs across
/// every facet it is a corner of, and the jaw's facets are a sixth of a
/// radius long: coloured any nearer the crossing, the inside of the mouth
/// shades the skin beside the closed lips as a dark seam. The band it keeps
/// skin is what an open mouth shows as a lip.
/// Provenance: **tuned by render** (#367), on a horse's long jaw.
const HIDDEN: f32 = 0.15;

/// How far out along its own normal a vertex of the head must still be
/// inside the shut jaw to count as the palate, in head radii: the same
/// argument as [`HIDDEN`], from the head's side of the crossing.
/// Provenance: **tuned by render** (#367).
const COVERED: f32 = 0.10;

/// How wide the jaw is against the muzzle over it: a little narrower, so
/// the upper lip hangs over its sides.
const JAW_WIDTH: f32 = 0.85;

/// A beast's lower jaw and tongue.
#[derive(Clone, Debug, PartialEq)]
pub struct Jaw {
    /// The jaw, in its joint's local space: about [`Jaw::hinge`].
    pub mesh: PolyMesh,
    /// The tongue, in its joint's local space: about [`Jaw::pivot`].
    pub tongue: PolyMesh,
    /// How much of each of the jaw's vertices is the inside of the mouth:
    /// `1` where the shut head hides it, `0` where it might show (#368).
    pub inside: Vec<f32>,
    /// Where the jaw hinges, in the head joint's local space.
    pub hinge: Vec3,
    /// Where the tongue turns, in the head joint's local space.
    pub pivot: Vec3,
    /// The joint the jaw rides, once [`Jaw::rig`] has run.
    pub joint: Option<usize>,
    /// The joint the tongue rides, a child of the jaw's.
    pub tongue_joint: Option<usize>,
    /// The head joint the jaw hangs from.
    pub head: usize,
}

impl Jaw {
    /// Builds the jaw and tongue a beast's head declares, for the head joint
    /// `head`, against `body` - the built surface, which decides what of the
    /// jaw the shut head hides.
    #[must_use]
    pub fn build(rig: &Rig, head: usize, beast: &BeastHead, body: &PolyMesh) -> Self {
        let origin = rig.joints[head].position;
        let into_joint = |mesh: PolyMesh, about: Vec3| {
            mesh.transformed(Mat4::from_quat(beast.turn) * Mat4::from_translation(-about))
        };
        // As wide as the muzzle over it, nearly, and as deep as the jaw's own
        // cone: a trough, not a rod.
        let mut jaw = swept(&beast.jaw, |at, radius, round| {
            let wide = (beast.muzzle.half_width_at(at.z) * JAW_WIDTH).max(radius);
            Vec2::new(wide * round, radius * 0.9 * round)
        });
        // The jaw's inside is what the head hides while the mouth is shut:
        // coloured as a mouth's well inside the head and inside the surface
        // actually built, skin wherever it might show.
        let inside: Vec<f32> = jaw
            .positions
            .iter()
            .map(|&at| {
                let hidden =
                    beast.distance(at) < -HIDDEN * beast.radius && body.contains(beast.body(at));
                if hidden { 1.0 } else { 0.0 }
            })
            .collect();
        jaw.set_colours(
            inside
                .iter()
                .map(|&hidden| if hidden > 0.5 { INSIDE } else { Vec3::ONE })
                .collect(),
        );
        let mut tongue = swept(&beast.tongue, |_, radius, round| {
            Vec2::new(radius * round, radius * 0.35 * round)
        });
        tongue.paint(TONGUE);
        let pivot = beast.tongue.back + (beast.tongue.front - beast.tongue.back) * TONGUE_PIVOT;
        Self {
            mesh: into_joint(jaw, beast.hinge),
            tongue: into_joint(tongue, pivot),
            inside,
            hinge: beast.body(beast.hinge) - origin,
            pivot: beast.body(pivot) - origin,
            joint: None,
            tongue_joint: None,
            head,
        }
    }

    /// Hangs the jaw on a joint at its hinge and the tongue on one of the
    /// jaw's: [`Role::Facial`], so neither binds the body's surface nor is
    /// taken for a person's jaw. Rigging twice does nothing.
    pub fn rig(&mut self, rig: &mut Rig) {
        if self.joint.is_some() {
            return;
        }
        let Some(origin) = rig.joints.get(self.head).map(|joint| joint.position) else {
            return;
        };
        self.joint = rig.attach(self.head, origin + self.hinge, Role::Facial);
        self.tongue_joint = self
            .joint
            .and_then(|jaw| rig.attach(jaw, origin + self.pivot, Role::Facial));
    }

    /// Opens the jaw `open` of the way to [`MAX_OPEN`] and turns the tongue
    /// `tongue` of the way out over the lip, both `0..=1`, on a pose of this
    /// rig: a pant is a little of each, a yawn the jaw alone.
    ///
    /// Each turns about the head's own across, which the head's nod leaves
    /// where it was, so the jaw drops toward the throat however the head is
    /// carried. Does nothing before [`Jaw::rig`].
    pub fn pose(&self, pose: &mut Pose, open: f32, tongue: f32) {
        for (joint, turn) in [
            (self.joint, MAX_OPEN * open.clamp(0.0, 1.0)),
            (self.tongue_joint, TONGUE_OUT * tongue.clamp(0.0, 1.0)),
        ] {
            if let Some(rotation) = joint.and_then(|joint| pose.rotations.get_mut(joint)) {
                *rotation = Quat::from_rotation_x(turn);
            }
        }
    }

    /// Which of `body`'s vertices the shut jaw covers: the palate, and
    /// whatever of the cheek the jaw stands inside, which an open mouth
    /// shows and is painted as the inside of one. Asked of the jaw as built,
    /// at rest, so what is painted is what it hides.
    ///
    /// **Covered with room to spare**: a vertex counts only if it is inside
    /// the jaw and still inside it `COVERED` head radii out along its own
    /// normal. A texel's colour runs across the whole triangle it is in, so
    /// a vertex painted dark just inside the crossing bleeds onto the skin
    /// just outside it, and the shut mouth wears a dark seam. Both, because
    /// the palate faces down at the jaw: pushed out along its normal, skin
    /// over the crevice above the jaw's top lands inside the jaw while
    /// itself in plain view.
    #[must_use]
    pub fn covers(&self, rig: &Rig, body: &PolyMesh, radius: f32) -> Vec<bool> {
        let Some(origin) = rig.joints.get(self.head).map(|joint| joint.position) else {
            return vec![false; body.vertex_count()];
        };
        let hinge = origin + self.hinge;
        let (lo, hi) = self.mesh.bounds();
        let normals = body.vertex_normals();
        body.positions
            .iter()
            .zip(&normals)
            .map(|(&point, &normal)| {
                let within =
                    |at: Vec3| at.cmpge(lo).all() && at.cmple(hi).all() && self.mesh.contains(at);
                let at = point - hinge;
                within(at) && within(at + normal * (COVERED * radius))
            })
            .collect()
    }

    /// The jaw and the tongue with the joints they ride; before
    /// [`Jaw::rig`], the head's.
    pub fn parts(&self) -> impl Iterator<Item = (&PolyMesh, usize)> {
        [
            (&self.mesh, self.joint.unwrap_or(self.head)),
            (&self.tongue, self.tongue_joint.unwrap_or(self.head)),
        ]
        .into_iter()
    }

    /// Both meshes, the jaw first, for writing.
    pub fn meshes_mut(&mut self) -> impl Iterator<Item = &mut PolyMesh> {
        [&mut self.mesh, &mut self.tongue].into_iter()
    }

    /// Each mesh's mouth channel, in [`Self::parts`]' order: the jaw's
    /// inside, and all of the tongue (#368).
    #[must_use]
    pub fn mouths(&self) -> [Vec<f32>; 2] {
        [self.inside.clone(), vec![1.0; self.tongue.vertex_count()]]
    }
}

/// A closed tube along a cone's axis, its ends rounded off as half-balls, in
/// the head's frame. `section` gives the half-extents at a station - across
/// and up - from where the station is, the cone's radius there and how much
/// of it the rounding of an end leaves.
fn swept(cone: &Cone, section: impl Fn(Vec3, f32, f32) -> Vec2) -> PolyMesh {
    let axis = cone.front - cone.back;
    let ahead = axis.normalize_or(Vec3::Z);
    // A quarter-circle of stations rounds each end: the angle from the
    // axis, out past the end ball's centre and back to its full girth.
    let rounding = [0.25, 0.5, 0.75, 1.0].map(|share| share * std::f32::consts::FRAC_PI_2);
    let mut path = Vec::new();
    let mut sections = Vec::new();
    let mut station = |at: Vec3, radius: f32, round: f32| {
        path.push(at);
        sections.push(section(at, radius, round));
    };
    for angle in rounding {
        let at = cone.back - ahead * (cone.back_radius * angle.cos());
        station(at, cone.back_radius, angle.sin());
    }
    for step in 1..8 {
        let share = step as f32 / 8.0;
        let radius = cone.back_radius + (cone.front_radius - cone.back_radius) * share;
        station(cone.back + axis * share, radius, 1.0);
    }
    for angle in rounding.into_iter().rev() {
        let at = cone.front + ahead * (cone.front_radius * angle.cos());
        station(at, cone.front_radius, angle.sin());
    }
    prim::sweep(&path, &sections, SIDES, Vec3::X)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, QuadrupedParams, Zone};

    fn built() -> (Rig, BeastHead, Jaw) {
        let skeleton = QuadrupedParams::default().skeleton(&Composites::default());
        let beast = skeleton.beast_head.expect("a quadruped declares its head");
        let mut rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        let head = rig
            .in_zone(Zone::Head)
            .into_iter()
            .min_by(|&a, &b| {
                let off = |joint: usize| rig.joints[joint].position.distance(beast.origin);
                off(a).total_cmp(&off(b))
            })
            .expect("a head");
        let body = crate::build_body(
            &skeleton,
            &crate::CageConfig::default(),
            crate::BODY_SUBDIVISIONS,
            &crate::face::HeadTraits::default(),
        )
        .expect("the body meshes");
        let mut jaw = Jaw::build(&rig, head, &beast, &body);
        jaw.rig(&mut rig);
        (rig, beast, jaw)
    }

    #[test]
    fn the_jaw_and_tongue_are_closed_solids() {
        let (_, _, jaw) = built();
        assert!(jaw.mesh.is_closed_manifold(), "the jaw has a hole");
        assert!(jaw.tongue.is_closed_manifold(), "the tongue has a hole");
    }

    #[test]
    fn the_shut_mouth_hides_its_inside_and_shows_its_chin() {
        let (rig, beast, jaw) = built();
        let at = |offset: Vec3| beast.local(rig.joints[jaw.head].position + jaw.hinge + offset);
        // Every tongue vertex is inside the head while the mouth is shut.
        let tongue_root = rig.joints[jaw.head].position + jaw.pivot;
        for &point in &jaw.tongue.positions {
            let local = beast.local(tongue_root + point);
            assert!(
                beast.distance(local) < 0.0,
                "the shut tongue shows at {local}"
            );
        }
        // And some of the jaw is outside it: the chin.
        let shown = jaw
            .mesh
            .positions
            .iter()
            .filter(|&&point| beast.distance(at(point)) > 0.0)
            .count();
        assert!(
            shown > jaw.mesh.positions.len() / 5,
            "only {shown} jaw vertices show"
        );
    }

    #[test]
    fn opening_drops_the_chin_and_rides_the_jaw_s_own_joints() {
        let (rig, _, jaw) = built();
        let joint = jaw.joint.expect("rigged");
        let tongue = jaw.tongue_joint.expect("rigged");
        assert_eq!(rig.joints[joint].role, Role::Facial);
        assert_eq!(rig.joints[tongue].parent, Some(joint));
        assert_eq!(
            crate::rig::skin::jaw_pivot(&rig),
            None,
            "a beast's jaw is not a person's marker chain"
        );
        let mut pose = Pose::rest(&rig);
        jaw.pose(&mut pose, 1.0, 0.0);
        let posed = pose.forward(&rig);
        // The chin - the jaw's front - drops when the jaw turns.
        let chin = jaw
            .mesh
            .positions
            .iter()
            .copied()
            .max_by(|a, b| a.z.total_cmp(&b.z))
            .expect("a jaw");
        let dropped = posed.rotations[joint] * chin;
        assert!(
            dropped.y < chin.y - 0.1 * chin.length(),
            "the chin went from {chin} to {dropped}"
        );
    }
}
