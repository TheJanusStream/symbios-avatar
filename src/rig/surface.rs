//! How thick the body actually is, joint by joint.
//!
//! A joint's radius is what the body plan *asked* for, not what the mesh came
//! out as. Subdivision shrinks a capped tube substantially — the skull ends up
//! near two thirds of its node radius — so anything positioned against the node
//! radius sits well outside the body it is supposed to touch. That mistake has
//! now been made three times in this crate: eyes that bulged like goggles, hair
//! that floated off the crown, and a fringe shoved out in front of the face.
//!
//! This measures the body once, so the fourth time is answered by a lookup.
//! Garments will want exactly the same question.

use glam::Vec3;

use crate::mesh::PolyMesh;
use crate::rig::Rig;

/// Where the measured radius sits in each joint's spread of surface distances.
///
/// Not the maximum: one vertex stretched by a neighbouring joint's socket would
/// then set the radius for the whole bone. High enough that hair laid on the
/// body rests on it rather than in it.
const PERCENTILE: f32 = 0.9;

/// How many places along each bone the thickness is measured.
///
/// One number per bone is not enough. A clavicle runs from the base of the neck
/// out to the shoulder: at its inner end the nearest surface is the whole upper
/// chest, at its outer end it is a thin cap. Measured as a single radius it came
/// out at 0.098 m against a 0.070 m node — and anything draped against that
/// figure gets flung out into a horizontal shelf at the shoulder, which is
/// exactly what hair did. The same holds for a thigh, thick at the hip and
/// slender at the knee.
const SAMPLES: usize = 5;

/// The body's measured thickness, sampled along each bone.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Surface {
    profiles: Vec<[f32; SAMPLES]>,
}

impl Surface {
    /// Measures a built body.
    ///
    /// Joints that carry no surface of their own keep their node radius, which
    /// is the best guess available for them.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, rig: &Rig) -> Self {
        let mut spreads: Vec<Vec<Vec<f32>>> = vec![vec![Vec::new(); SAMPLES]; rig.len()];
        for &point in &mesh.positions {
            let hit = rig.nearest_bone(point);
            let bin = (along_bone(rig, hit.joint, hit.closest) * (SAMPLES - 1) as f32).round();
            spreads[hit.joint][(bin as usize).min(SAMPLES - 1)].push(hit.distance);
        }

        let profiles = spreads
            .iter_mut()
            .enumerate()
            .map(|(joint, bins)| {
                let fallback = rig.joints[joint].radius;
                let mut profile = [f32::NAN; SAMPLES];
                for (sample, spread) in bins.iter_mut().enumerate() {
                    if spread.is_empty() {
                        continue;
                    }
                    spread.sort_by(f32::total_cmp);
                    let at = ((spread.len() - 1) as f32 * PERCENTILE).round() as usize;
                    profile[sample] = spread[at];
                }
                fill_gaps(&mut profile, fallback);
                profile
            })
            .collect();

        Self { profiles }
    }

    /// How far the body's surface stands from a bone, `along` its length in
    /// `0..=1`, in metres.
    #[must_use]
    pub fn radius(&self, joint: usize, along: f32) -> f32 {
        let Some(profile) = self.profiles.get(joint) else {
            return 0.0;
        };
        let at = along.clamp(0.0, 1.0) * (SAMPLES - 1) as f32;
        let sample = (at.floor() as usize).min(SAMPLES - 2);
        let blend = at - sample as f32;
        profile[sample] + (profile[sample + 1] - profile[sample]) * blend
    }

    /// The thickest the body gets along a bone, in metres.
    #[must_use]
    pub fn widest(&self, joint: usize) -> f32 {
        self.profiles
            .get(joint)
            .map_or(0.0, |profile| profile.iter().fold(0.0f32, |a, b| a.max(*b)))
    }

    /// Pushes a point out of the body, if it is inside it.
    ///
    /// The push is horizontal. Things that drape — hair, cloth — go *around* a
    /// chest, and pushing along the shortest line would instead slide them up
    /// the body toward the shoulders.
    #[must_use]
    pub fn clear(&self, rig: &Rig, point: Vec3, margin: f32) -> Vec3 {
        point + self.clearance(rig, point, margin)
    }

    /// How far, and which way, a point would have to move to clear the body.
    ///
    /// Separate from [`Self::clear`] so a caller that drapes something along a
    /// path can smooth the correction rather than take it whole at one step —
    /// an abrupt sideways jog turns a falling ribbon into a horizontal shelf.
    #[must_use]
    pub fn clearance(&self, rig: &Rig, point: Vec3, margin: f32) -> Vec3 {
        let hit = rig.nearest_bone(point);
        let along = along_bone(rig, hit.joint, hit.closest);
        let needed = self.radius(hit.joint, along) + margin;
        if hit.distance >= needed {
            return Vec3::ZERO;
        }
        let away = point - hit.closest;
        let flat = Vec3::new(away.x, 0.0, away.z);
        let out = flat.normalize_or(Vec3::Z);
        out * (needed - flat.length()).max(0.0)
    }
}

/// How far along a bone a point sits, in `0..=1`.
fn along_bone(rig: &Rig, joint: usize, point: Vec3) -> f32 {
    let (start, end) = rig.bone(joint);
    let axis = end - start;
    if axis.length_squared() <= f32::EPSILON {
        return 0.0;
    }
    ((point - start).dot(axis) / axis.length_squared()).clamp(0.0, 1.0)
}

/// Fills samples no vertex landed in, so a profile is continuous.
fn fill_gaps(profile: &mut [f32; SAMPLES], fallback: f32) {
    let Some(first) = profile.iter().position(|width| width.is_finite()) else {
        profile.fill(fallback);
        return;
    };
    let last = profile
        .iter()
        .rposition(|width| width.is_finite())
        .unwrap_or(first);
    for sample in 0..first {
        profile[sample] = profile[first];
    }
    for sample in last + 1..SAMPLES {
        profile[sample] = profile[last];
    }
    let mut sample = first;
    while sample <= last {
        if profile[sample].is_finite() {
            sample += 1;
            continue;
        }
        let gap = sample;
        while !profile[sample].is_finite() {
            sample += 1;
        }
        let (before, after) = (profile[gap - 1], profile[sample]);
        let span = (sample - gap + 1) as f32;
        for (step, hole) in (gap..sample).enumerate() {
            profile[hole] = before + (after - before) * (step + 1) as f32 / span;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Zone;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn body(seed: i64) -> (PolyMesh, Rig) {
        let mut record = AvatarRecord::new("Measured", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        (
            catmull_clark(&cage, crate::BODY_SUBDIVISIONS),
            Rig::from_skeleton(&skeleton).expect("the body should rig"),
        )
    }

    #[test]
    fn a_body_measures_thinner_than_its_plan_asked_for() {
        // The whole point. If this ever stops holding, everything positioned
        // against a measured surface should be revisited.
        //
        // **Asked of the head's whole CHAIN, and over six seeds rather than
        // one** (#131). This used to hold `widest(head)` against
        // `joints[head].radius`, and both halves of that were wrong. A skull is
        // two nodes — the head and a crown above it at `CROWN_HIGH` head radii —
        // so the head BONE runs up to the crown and its surface is under no
        // obligation to stay inside the lower node's own radius. And on one
        // seed it did not have to: measured over these six, seeds 7 and 21 read
        // 1.161 and 1.037 of the head node's radius while the test passed,
        // because seed 1 was the only body it ever asked.
        //
        // Against what the chain actually reaches — the further of the head
        // node's own half-extents and the crown's, measured from the head joint
        // — the reading is the two thirds this module's own docs claim, and it
        // is far tighter than it was:
        //
        // ```text
        //   seed        0      1      3      7     13     21
        //   before  0.556  0.522  0.591  0.701  0.573  0.627
        //   after   0.608  0.622  0.656  0.684  0.623  0.689
        // ```
        //
        // The move is #131's wider neck filling the blend under the jaw, so more
        // of the head's own sample bins have surface in them to average. The
        // 0.75 is a ratchet on the second row, not a target.
        for seed in [0i64, 1, 3, 7, 13, 21] {
            let (mesh, rig) = body(seed);
            let surface = Surface::measure(&mesh, &rig);
            let head = *rig.in_zone(Zone::Head).first().expect("a head");
            let reach = head_reach(&rig, head);
            let measured = surface.widest(head);
            assert!(
                measured < reach * 0.75,
                "seed {seed}: the skull measured {measured} against a chain that \
                 reaches {reach}, a fraction of {:.3}. It has to stay under 1.0 for \
                 this module to mean anything; 0.75 is the ratchet.",
                measured / reach
            );
        }
    }

    /// The furthest this joint's own node chain asks its surface to reach.
    ///
    /// The joint's half-extents, or a child in the same zone plus that child's,
    /// whichever is further. A single node radius is not the answer for anything
    /// built from more than one node, which the skull is.
    fn head_reach(rig: &Rig, head: usize) -> f32 {
        let at = rig.joints[head].position;
        let zone = rig.in_zone(Zone::Head);
        let own = rig.joints[head].radius * rig.joints[head].scale.max_element();
        rig.joints
            .iter()
            .enumerate()
            .filter(|(index, joint)| joint.parent == Some(head) && zone.contains(index))
            .fold(own, |far, (_, joint)| {
                far.max((joint.position - at).length() + joint.radius * joint.scale.max_element())
            })
    }

    #[test]
    fn every_joint_gets_a_positive_radius() {
        let (mesh, rig) = body(7);
        let surface = Surface::measure(&mesh, &rig);
        for joint in 0..rig.len() {
            assert!(
                surface.widest(joint) > 0.0,
                "joint {joint} measured {}",
                surface.widest(joint)
            );
        }
    }

    #[test]
    fn a_point_inside_the_body_is_pushed_out_of_it() {
        let (mesh, rig) = body(23);
        let surface = Surface::measure(&mesh, &rig);
        let chest = *rig.in_zone(Zone::Chest).first().expect("a chest");
        let inside = rig.joints[chest].position + Vec3::new(0.001, 0.0, 0.0);

        let pushed = surface.clear(&rig, inside, 0.01);
        let hit = rig.nearest_bone(pushed);
        let along = super::along_bone(&rig, hit.joint, hit.closest);
        assert!(
            hit.distance >= surface.radius(hit.joint, along) + 0.009,
            "a pushed point still sat {} inside a surface of {}",
            hit.distance,
            surface.radius(hit.joint, along)
        );
    }

    #[test]
    fn a_point_already_clear_is_left_alone() {
        let (mesh, rig) = body(3);
        let surface = Surface::measure(&mesh, &rig);
        let outside = rig.joints[0].position + Vec3::new(10.0, 0.0, 0.0);
        assert_eq!(surface.clear(&rig, outside, 0.01), outside);
    }

    #[test]
    fn a_bone_is_measured_along_its_length_not_as_one_number() {
        // A clavicle is thick where it meets the neck and thin at the shoulder.
        // Read as a single radius it came out half again too wide, and anything
        // draped against that figure juts out into a shelf.
        let (mesh, rig) = body(1);
        let surface = Surface::measure(&mesh, &rig);
        let clavicles = rig.in_zone(Zone::Chest);
        let varying = clavicles.iter().any(|&joint| {
            let ends: Vec<f32> = (0..5)
                .map(|s| surface.radius(joint, s as f32 / 4.0))
                .collect();
            let wide = ends.iter().fold(0.0f32, |a, b| a.max(*b));
            let narrow = ends.iter().fold(f32::MAX, |a, b| a.min(*b));
            wide > narrow * 1.25
        });
        assert!(varying, "no bone measured meaningfully thicker at one end");
    }

    #[test]
    fn the_push_does_not_slide_a_point_up_the_body() {
        // Pushing along the shortest line would carry hair over the shoulder
        // instead of around the chest.
        let (mesh, rig) = body(11);
        let surface = Surface::measure(&mesh, &rig);
        let chest = *rig.in_zone(Zone::Chest).first().expect("a chest");
        let inside = rig.joints[chest].position;
        assert!((surface.clear(&rig, inside, 0.02).y - inside.y).abs() < 1e-6);
    }
}
