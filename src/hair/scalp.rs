//! The surface hair grows on.
//!
//! Hair has to sit *on* a head, and the head it has to sit on is not the sphere
//! the body plan describes. Subdivision pulls a capped tube well inside its node
//! radius: measured across several bodies, the rendered skull reaches only about
//! `0.64` of that radius sideways and `0.86` of it upward. Placing hair against
//! the nominal radius would leave it floating a head-width off the scalp, which
//! is the same mistake that first made the eyes bulge like goggles.
//!
//! So rather than assume a shape, this measures one — a profile of horizontal
//! radius against height, sampled from the built mesh. The measured profile is
//! also fuller than any ellipsoid fitted to it, which is a second reason not to
//! guess. And because it is measured rather than derived, it works unchanged on
//! a creature's head, which is not a scaled human one.

use glam::Vec3;

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;

/// How many heights the profile is sampled at.
const BANDS: usize = 24;

/// How far below the head joint to sample, in head radii.
///
/// Far enough to catch the nape, since hair covers it, but not so far as to
/// start measuring the neck.
const FLOOR: f32 = -0.55;

/// A skull's surface, as a profile of radius against height.
///
/// Heights and widths are in *head radii* so that the same hair parameters suit
/// any size of head; points come back in metres, in head-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Scalp {
    /// The head joint hair is parented to.
    pub head: usize,
    centre: Vec3,
    radius: f32,
    profile: [f32; BANDS],
    lo: f32,
    hi: f32,
}

impl Scalp {
    /// Measures the head's surface from a built body.
    ///
    /// Returns `None` for a body with no head, or one whose head carries too
    /// little surface to profile.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, rig: &Rig) -> Option<Self> {
        let head = *rig.in_zone(Zone::Head).first()?;
        let centre = rig.joints[head].position;
        let radius = rig.joints[head].radius;
        if radius <= f32::EPSILON {
            return None;
        }

        // Only vertices the head itself carries. Asking the rig which bone is
        // nearest is what keeps the shoulders out of the profile — they sit
        // within a head radius of the skull on a compact body, and a plain
        // height cut-off lets them in.
        let mut samples: Vec<(f32, f32)> = Vec::new();
        for &point in &mesh.positions {
            let hit = rig.nearest_bone(point);
            if rig.joints[hit.joint].zone != Zone::Head {
                continue;
            }
            let height = (point.y - centre.y) / radius;
            if height < FLOOR {
                continue;
            }
            let across = Vec3::new(point.x - centre.x, 0.0, point.z - centre.z).length() / radius;
            samples.push((height, across));
        }
        if samples.len() < BANDS {
            return None;
        }

        let lo = FLOOR;
        let hi = samples.iter().fold(f32::MIN, |top, &(h, _)| top.max(h));
        if hi <= lo + f32::EPSILON {
            return None;
        }

        // Widest vertex per band. A band takes the widest rather than the mean
        // because hair must clear the surface everywhere, not on average.
        let step = (hi - lo) / (BANDS - 1) as f32;
        let mut profile = [f32::NAN; BANDS];
        for (band, width) in profile.iter_mut().enumerate() {
            let at = lo + step * band as f32;
            let widest = samples
                .iter()
                .filter(|(height, _)| (height - at).abs() <= step * 0.75)
                .fold(f32::MIN, |wide, &(_, across)| wide.max(across));
            if widest > f32::MIN {
                *width = widest;
            }
        }
        // The crown is a pole, so its band holds one vertex at zero width and
        // several from just below it. Taking the widest there would leave the
        // profile flat-topped and hair standing off the crown.
        profile[BANDS - 1] = 0.0;
        fill_gaps(&mut profile)?;

        Some(Self {
            head,
            centre,
            radius,
            profile,
            lo,
            hi,
        })
    }

    /// The head node's radius, which every other figure here is measured in.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Where head-local space sits in the body, at rest.
    ///
    /// Hair is built head-local, but the body it has to drape over is not, so
    /// anything that asks the rig a question has to come back through here.
    #[must_use]
    pub fn origin(&self) -> Vec3 {
        self.centre
    }

    /// The crown's height, in head radii above the head joint.
    #[must_use]
    pub fn top(&self) -> f32 {
        self.hi
    }

    /// The lowest height the profile covers, in head radii.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.lo
    }

    /// The skull's horizontal radius at `height`, both in head radii.
    #[must_use]
    pub fn width_at(&self, height: f32) -> f32 {
        if height >= self.hi {
            return 0.0;
        }
        if height <= self.lo {
            return self.profile[0];
        }
        let step = (self.hi - self.lo) / (BANDS - 1) as f32;
        let along = (height - self.lo) / step;
        let band = (along.floor() as usize).min(BANDS - 2);
        let blend = along - band as f32;
        self.profile[band] + (self.profile[band + 1] - self.profile[band]) * blend
    }

    /// A point on the surface, in head-local metres.
    ///
    /// `azimuth` is measured from the face — zero is straight ahead, `+Z` —
    /// turning toward the body's right.
    #[must_use]
    pub fn point(&self, azimuth: f32, height: f32) -> Vec3 {
        let across = self.width_at(height) * self.radius;
        Vec3::new(
            azimuth.sin() * across,
            height * self.radius,
            azimuth.cos() * across,
        )
    }

    /// The outward normal at a point on the surface.
    #[must_use]
    pub fn normal(&self, azimuth: f32, height: f32) -> Vec3 {
        // From the meridian's slope: a tangent of (dw, dh) has an outward normal
        // of (dh, -dw). At the crown the profile falls away steeply and this
        // turns to point straight up, which is what it should do.
        let step = (self.hi - self.lo) / (BANDS - 1) as f32;
        let slope = (self.width_at(height + step) - self.width_at(height - step)) / (2.0 * step);
        let radial = Vec3::new(azimuth.sin(), 0.0, azimuth.cos());
        (radial - Vec3::Y * slope).normalize_or(radial)
    }
}

/// Fills bands no vertex landed in, so the profile is continuous.
fn fill_gaps(profile: &mut [f32; BANDS]) -> Option<()> {
    let first = profile.iter().position(|width| width.is_finite())?;
    let last = profile.iter().rposition(|width| width.is_finite())?;
    for band in 0..first {
        profile[band] = profile[first];
    }
    for band in last + 1..BANDS {
        profile[band] = profile[last];
    }
    let mut band = first;
    while band <= last {
        if profile[band].is_finite() {
            band += 1;
            continue;
        }
        let gap_start = band;
        while !profile[band].is_finite() {
            band += 1;
        }
        let (before, after) = (profile[gap_start - 1], profile[band]);
        let span = (band - gap_start + 1) as f32;
        for (step, hole) in (gap_start..band).enumerate() {
            profile[hole] = before + (after - before) * (step + 1) as f32 / span;
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn body(seed: i64) -> (PolyMesh, Rig) {
        let mut record = AvatarRecord::new("Scalped", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        (
            catmull_clark(&cage, 2),
            Rig::from_skeleton(&skeleton).expect("the body should rig"),
        )
    }

    #[test]
    fn a_head_can_be_profiled() {
        let (mesh, rig) = body(1);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        assert!(scalp.top() > 0.5, "crown at {}", scalp.top());
        assert!(scalp.radius() > 0.0);
    }

    #[test]
    fn the_profile_closes_at_the_crown_and_is_widest_low() {
        let (mesh, rig) = body(7);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        assert_eq!(scalp.width_at(scalp.top()), 0.0);
        assert!(scalp.width_at(scalp.top() - 0.05) < scalp.width_at(0.0));
        assert!(scalp.width_at(0.0) > 0.4, "a skull should have some width");
    }

    #[test]
    fn the_measured_surface_lies_inside_the_node_radius() {
        // The whole reason this module exists: were the rendered skull the size
        // its node claims, hair could be placed by arithmetic alone.
        let (mesh, rig) = body(23);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        assert!(
            scalp.width_at(0.0) < 0.8,
            "the skull measured {} of its node radius across",
            scalp.width_at(0.0)
        );
    }

    #[test]
    fn no_measured_point_lies_outside_the_profile() {
        let (mesh, rig) = body(3);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        let head = rig.joints[scalp.head];
        for &point in &mesh.positions {
            if rig.joints[rig.nearest_bone(point).joint].zone != Zone::Head {
                continue;
            }
            let height = (point.y - head.position.y) / head.radius;
            if height < FLOOR || height > scalp.top() - 0.1 {
                continue;
            }
            let across = Vec3::new(point.x - head.position.x, 0.0, point.z - head.position.z)
                .length()
                / head.radius;
            assert!(
                across <= scalp.width_at(height) + 0.06,
                "a vertex at height {height} reached {across}, past the profile's {}",
                scalp.width_at(height)
            );
        }
    }

    #[test]
    fn the_crown_normal_points_up_and_the_side_normal_out() {
        let (mesh, rig) = body(11);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        assert!(scalp.normal(0.0, scalp.top() - 0.02).y > 0.8);
        let side = scalp.normal(std::f32::consts::FRAC_PI_2, 0.0);
        assert!(side.x > 0.7, "a side normal pointed {side:?}");
    }

    #[test]
    fn the_front_of_the_head_faces_positive_z() {
        let (mesh, rig) = body(5);
        let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
        assert!(scalp.point(0.0, 0.0).z > 0.0);
    }

    #[test]
    fn gaps_in_a_profile_are_interpolated() {
        let mut profile = [f32::NAN; BANDS];
        profile[2] = 1.0;
        profile[6] = 0.0;
        fill_gaps(&mut profile).expect("two samples are enough");
        assert!(profile.iter().all(|width| width.is_finite()));
        assert_eq!(profile[0], 1.0);
        assert_eq!(profile[BANDS - 1], 0.0);
        assert!(
            (profile[4] - 0.5).abs() < 1e-5,
            "midpoint was {}",
            profile[4]
        );
    }
}
