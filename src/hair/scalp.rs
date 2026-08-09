//! The surface hair grows on.
//!
//! Hair has to sit *on* a head, and the head it has to sit on is not the sphere
//! the body plan describes. Subdivision pulls a capped tube well inside its node
//! radius: measured across several bodies, the rendered skull reaches only about
//! `0.64` of that radius sideways and `0.86` of it upward. Placing hair against
//! the nominal radius would leave it floating a head-width off the scalp, which
//! is the same mistake that first made the eyes bulge like goggles.
//!
//! The `0.64` above was measured on a four-point cage. Eight-point rings (#107)
//! take it to `0.84`–`0.90` across sixteen seeds, so the gap is a fifth of what
//! it was and this module is no less necessary for it: a tenth of a head radius
//! is 9 mm on the default body, against a hair stand-off of about 3 mm.
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

/// How many directions round the head each band is sampled at.
///
/// A head is not a surface of revolution and the parts that depart from one are
/// exactly the parts hair has to clear: `shape_skull` gives it a brow ridge
/// standing proud, an occiput at the back and temples drawn in. Measured on the
/// default body, the skull stands up to 12.7 mm outside a radius-against-height
/// profile — worst straight ahead at the brow and straight back at the occiput —
/// against a hair stand-off of about 3 mm, so hair lofted on one profile sat
/// nine millimetres inside the head and the brow and occiput came through it
/// (#69). Sixteen is enough to catch a brow ridge without turning the profile
/// into a copy of the mesh.
const SECTORS: usize = 16;

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
    around: [[f32; SECTORS]; BANDS],
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
        //
        // **The bone credited to the head runs down to the NECK joint, so this
        // question is coarser than it looks — and here it does not matter**
        // (#125). `rig::skin::owner_of` bounds the head's claim on that bone at
        // `COVERED`, three quarters of it below the joint, because the throat
        // answers `Zone::Head` otherwise. That line lands 1.125 head radii under
        // the joint on the plan this crate ships and [`FLOOR`] stops sampling at
        // 0.55, so this window never reaches it: every vertex in it is the
        // head's under either question. Which means the profile that widened
        // when the neck was given mass astern widened because the head's own
        // surface moved, and not because a nape was counted as one.
        let mut samples: Vec<(f32, f32, usize)> = Vec::new();
        for &point in &mesh.positions {
            let hit = rig.nearest_bone(point);
            if rig.joints[hit.joint].zone != Zone::Head {
                continue;
            }
            let height = (point.y - centre.y) / radius;
            if height < FLOOR {
                continue;
            }
            let offset = Vec3::new(point.x - centre.x, 0.0, point.z - centre.z);
            let across = offset.length() / radius;
            // Same convention as `point`: zero straight ahead at +Z, turning
            // toward the body's right.
            let bearing = offset.x.atan2(offset.z).rem_euclid(std::f32::consts::TAU);
            let sector = ((bearing / std::f32::consts::TAU) * SECTORS as f32) as usize % SECTORS;
            samples.push((height, across, sector));
        }
        if samples.len() < BANDS {
            return None;
        }

        let lo = FLOOR;
        let hi = samples.iter().fold(f32::MIN, |top, &(h, _, _)| top.max(h));
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
                .filter(|(height, _, _)| (height - at).abs() <= step * 0.75)
                .fold(f32::MIN, |wide, &(_, across, _)| wide.max(across));
            if widest > f32::MIN {
                *width = widest;
            }
        }
        // The crown is a pole, so its band holds one vertex at zero width and
        // several from just below it. Taking the widest there would leave the
        // profile flat-topped and hair standing off the crown.
        profile[BANDS - 1] = 0.0;
        fill_gaps(&mut profile)?;

        // The same again, per direction round the head.
        //
        // **A sector with no sample takes its OWN meridian's, from the bands
        // above and below it — not the band's widest** (#125). Falling back to
        // the band was safe while a head was nearly a surface of revolution,
        // because every sector's answer was nearly every other's. It is not safe
        // once the head has mass on one side: measured on seed 7 with the neck's
        // section swept astern, band 7 sampled only its rear sectors, the nape
        // read 0.98 radii there, and all six sectors across the FACE — which
        // sampled nothing — were handed the back of the head to stand on. The
        // profile then stepped 0.83 → 0.98 between two adjacent bands straight
        // ahead, which is what lifted a strand uphill and buried another.
        //
        // A head is far smoother up a meridian than round a band, so this is
        // also the better interpolation on a symmetric head; it simply could not
        // be told apart from the old one there.
        let mut around = [[f32::NAN; SECTORS]; BANDS];
        for (band, row) in around.iter_mut().enumerate() {
            let at = lo + step * band as f32;
            for (sector, width) in row.iter_mut().enumerate() {
                let widest = samples
                    .iter()
                    .filter(|(height, _, bearing)| {
                        (height - at).abs() <= step * 0.75 && *bearing == sector
                    })
                    .fold(f32::MIN, |wide, &(_, across, _)| wide.max(across));
                if widest > f32::MIN {
                    *width = widest;
                }
            }
        }
        for sector in 0..SECTORS {
            let mut meridian: [f32; BANDS] = std::array::from_fn(|band| around[band][sector]);
            // A sector that sampled nothing anywhere has no meridian to fill
            // from, and only then does the band's own widest stand in.
            if fill_gaps(&mut meridian).is_none() {
                meridian = profile;
            }
            for (band, row) in around.iter_mut().enumerate() {
                row[sector] = meridian[band];
            }
        }
        // The crown is a pole in every direction.
        around[BANDS - 1] = [0.0; SECTORS];

        Some(Self {
            head,
            centre,
            radius,
            profile,
            around,
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

    /// How far the surface reaches sideways at `height`, looking along `azimuth`.
    ///
    /// The measure hair should use. [`Self::width_at`] answers for the head as a
    /// whole, which is a surface of revolution the head is not: a brow ridge and
    /// an occiput both stand well outside it, and hair placed against it sits
    /// inside them (#69).
    #[must_use]
    pub fn width_around(&self, azimuth: f32, height: f32) -> f32 {
        let at = ((height - self.lo) / (self.hi - self.lo) * (BANDS - 1) as f32)
            .clamp(0.0, (BANDS - 1) as f32);
        let band = (at.floor() as usize).min(BANDS - 2);
        let blend = at - band as f32;

        let turn = azimuth.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
        let column = turn * SECTORS as f32;
        let near = (column.floor() as usize) % SECTORS;
        let far = (near + 1) % SECTORS;
        let along = column - column.floor();

        let row = |band: usize| {
            self.around[band][near] + (self.around[band][far] - self.around[band][near]) * along
        };
        row(band) + (row(band + 1) - row(band)) * blend
    }

    /// A point on the surface, in head-local metres.
    ///
    /// `azimuth` is measured from the face — zero is straight ahead, `+Z` —
    /// turning toward the body's right.
    #[must_use]
    pub fn point(&self, azimuth: f32, height: f32) -> Vec3 {
        let across = self.width_around(azimuth, height) * self.radius;
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
        let slope = (self.width_around(azimuth, height + step)
            - self.width_around(azimuth, height - step))
            / (2.0 * step);
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
        let (mesh, rig, _) = sectioned_body(seed);
        (mesh, rig)
    }

    /// The same, plus how far the head's own node reaches, against its radius.
    ///
    /// **The head has an elliptical section now** (#61), so "a fraction of the
    /// node radius" and "a fraction of what the cage swept" are two different
    /// numbers where they used to be one.
    ///
    /// **The node's LATERAL half-extent, and the caller asks the surface a
    /// lateral question to match** (#79). This went briefly to the node's
    /// largest reach instead, on the argument that [`Scalp::width_at`] is a
    /// radius from the joint's axis and takes the widest sample in the band
    /// whatever direction it lies in. That is true of `width_at` and it is the
    /// wrong half of the fix: on thirteen of sixteen bodies that widest sample
    /// IS the back one — the profile's rear reading and its maximum agree to
    /// three decimals — so the bound had stopped measuring the head's width at
    /// all and started measuring the nape the neck's section puts behind it.
    /// A number that tracks the neck cannot guard the head, and it had been
    /// re-based twice in two days for exactly that reason.
    ///
    /// So the caller reads [`Scalp::width_around`] at the two side bearings
    /// instead, which is the reading this module's own opening paragraph
    /// describes — *the rendered skull reaches only about 0.64 of that radius
    /// SIDEWAYS*. It became askable when the per-sector fill was fixed (#125):
    /// a side sector used to fall back to the band's widest, which is to say to
    /// the same rear sample.
    fn sectioned_body(seed: i64) -> (PolyMesh, Rig, f32) {
        let mut record = AvatarRecord::new("Scalped", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let head = *rig
            .in_zone(Zone::Head)
            .first()
            .expect("a humanoid has a head");
        let section = rig.joints[head]
            .node
            .and_then(|node| skeleton.nodes.get(node as usize))
            .map_or(1.0, |node| node.scale.x);
        (
            // [`crate::BODY_SUBDIVISIONS`], not a literal. This helper measures
            // how much of its node radius the head's SURFACE delivers, and a
            // level of its own measures a head nobody renders — the same defect
            // #107 found in two of the skull tests, in the same shape.
            catmull_clark(&cage, crate::BODY_SUBDIVISIONS),
            rig,
            section,
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
        //
        // **The gap is a fifth of what it was, and the module still earns its
        // keep** (#107). Eight-point cage rings sit far closer to the surface
        // they approximate than a four-point diamond does, so the head that used
        // to deliver 0.65 of its node radius across now delivers 0.84 to 0.90 —
        // swept over sixteen seeds, which is where the bound below comes from.
        // A tenth of a head radius is still 9 mm of scalp on the default body,
        // several times the hair's own stand-off, so hair placed by arithmetic
        // would still sit inside the skull.
        //
        // **Swept rather than asked of one seed.** The figure varies by 0.06
        // across seeds — more than the margin this bound has — so a single body
        // cannot say where the worst case is.
        //
        // **The head has an elliptical section now, and the bound had to be told
        // which radius it means** (#61). A broad skull sweeps a ring 1.14 node
        // radii across and its surface duly measured 0.956 of a bare radius —
        // outside a bound written when the two were the same number and right on
        // the geometry. So the reading is against the cage's own reach, which is
        // what the gap this module exists for is a gap from — see
        // [`sectioned_body`] for why that reach is the node's LARGEST and not
        // its lateral one, which is a correction to this paragraph rather than
        // an addition to it (#125).
        //
        // **And the ceiling had to come up to 1.02, which is not a relaxation.**
        // The neck below the head is not sectioned with it, so on a NARROW skull
        // the surface at the head joint is held out by the throat: seed 15's
        // section is 0.869 and its surface delivers 1.008 of it. That is the
        // cage doing what a blend does, not the module losing its argument — a
        // point on the head's own ring is still inside it, and what stands proud
        // is a point the neck owns.
        //
        // What the module exists for is unchanged and is now the FIRST reading
        // rather than an inference from the second: across sixteen bodies the
        // surface delivers 0.838 to 0.956 of the node radius, a spread of 14%,
        // and no single constant can stand in for a measurement that varies that
        // far. Before the axis it varied 6% and the argument was thinner.
        let mut worst: (i64, f32) = (0, 0.0);
        let mut raw = (f32::MAX, f32::MIN);
        for seed in 1i64..=16 {
            let (mesh, rig, section) = sectioned_body(seed);
            let scalp = Scalp::measure(&mesh, &rig).expect("a humanoid has a head");
            // Sideways, at the head joint's own height — see [`sectioned_body`]
            // for why this is not `width_at`, which answers with the nape.
            let across = scalp
                .width_around(std::f32::consts::FRAC_PI_2, 0.0)
                .max(scalp.width_around(-std::f32::consts::FRAC_PI_2, 0.0));
            raw = (raw.0.min(across), raw.1.max(across));
            if across / section > worst.1 {
                worst = (seed, across / section);
            }
        }
        assert!(
            raw.1 / raw.0 > 1.10,
            "the head delivers {:.3} to {:.3} of its node radius across sixteen \
             bodies — close enough to a constant that hair could be placed by \
             arithmetic, which is what this module exists instead of",
            raw.0,
            raw.1
        );
        // **1.02, then 1.04, then 1.08, and then the reading was wrong** (#79).
        // Two of those three re-bases were the neck: the surface at the head
        // joint is held out by the throat on a narrow skull, and #125's
        // off-centre section put a nape behind it. Both were real and neither
        // was the head standing outside its own node, which is what this bound
        // is for. See [`sectioned_body`]: on thirteen of sixteen bodies the
        // radial maximum this used to read IS the rear reading, so the number
        // was tracking the neck and had to move whenever the neck did.
        //
        // Sideways, it does not. Measured over the same sixteen bodies with the
        // skull's section at the humanoid plan's `SKULL_SLENDER`, the surface
        // delivers 0.764 to
        // 0.877 of the node's lateral half-extent — a spread of 15%, which is
        // the argument this module exists for, and a ceiling with room in it.
        //
        // The reading to watch is no longer a seed count, because no body is
        // near it: it is whether the top of the range climbs. At 1.0 the head
        // would be standing exactly on its own ring, and subdivision does not
        // do that.
        //
        // **1.10, up from 0.95, and it is the exploration range** (#160):
        // generator 2 rolls `head_breadth` past ±1, and the breadth profile in
        // `face::skull` multiplies the surface laterally AFTER the ring was
        // swept, so an extreme-broad skull honestly stands past its own ring
        // (seed 9 reads 1.045 under generator 2). Hair does not care — roots
        // seat against the MEASURED surface, which is this module's whole
        // argument — so the ceiling is a sanity bound on the instrument, not
        // on the hair, and it moves with the range the bodies draw from.
        assert!(
            worst.1 < 1.10,
            "seed {}: the skull measured {} of what its own ring swept across",
            worst.0,
            worst.1
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
