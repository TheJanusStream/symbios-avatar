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

/// The body's measured thickness, one radius per joint.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    radii: Vec<f32>,
}

impl Surface {
    /// Measures a built body.
    ///
    /// Joints that carry no surface of their own keep their node radius, which
    /// is the best guess available for them.
    #[must_use]
    pub fn measure(mesh: &PolyMesh, rig: &Rig) -> Self {
        let mut spreads: Vec<Vec<f32>> = vec![Vec::new(); rig.len()];
        for &point in &mesh.positions {
            let hit = rig.nearest_bone(point);
            spreads[hit.joint].push(hit.distance);
        }

        let radii = spreads
            .iter_mut()
            .enumerate()
            .map(|(joint, spread)| {
                if spread.is_empty() {
                    return rig.joints[joint].radius;
                }
                spread.sort_by(f32::total_cmp);
                let at = ((spread.len() - 1) as f32 * PERCENTILE).round() as usize;
                spread[at]
            })
            .collect();

        Self { radii }
    }

    /// How far the body's surface stands from a joint's bone, in metres.
    #[must_use]
    pub fn radius(&self, joint: usize) -> f32 {
        self.radii.get(joint).copied().unwrap_or(0.0)
    }

    /// Pushes a point out of the body, if it is inside it.
    ///
    /// The push is horizontal. Things that drape — hair, cloth — go *around* a
    /// chest, and pushing along the shortest line would instead slide them up
    /// the body toward the shoulders.
    #[must_use]
    pub fn clear(&self, rig: &Rig, point: Vec3, margin: f32) -> Vec3 {
        let hit = rig.nearest_bone(point);
        let needed = self.radius(hit.joint) + margin;
        if hit.distance >= needed {
            return point;
        }
        let away = point - hit.closest;
        let flat = Vec3::new(away.x, 0.0, away.z);
        let out = flat.normalize_or(Vec3::Z);
        point + out * (needed - flat.length()).max(0.0)
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
            catmull_clark(&cage, 2),
            Rig::from_skeleton(&skeleton).expect("the body should rig"),
        )
    }

    #[test]
    fn a_body_measures_thinner_than_its_plan_asked_for() {
        // The whole point. If this ever stops holding, everything positioned
        // against a measured surface should be revisited.
        let (mesh, rig) = body(1);
        let surface = Surface::measure(&mesh, &rig);
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        assert!(
            surface.radius(head) < rig.joints[head].radius,
            "the skull measured {} against a node radius of {}",
            surface.radius(head),
            rig.joints[head].radius
        );
    }

    #[test]
    fn every_joint_gets_a_positive_radius() {
        let (mesh, rig) = body(7);
        let surface = Surface::measure(&mesh, &rig);
        for joint in 0..rig.len() {
            assert!(
                surface.radius(joint) > 0.0,
                "joint {joint} measured {}",
                surface.radius(joint)
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
        assert!(
            hit.distance >= surface.radius(hit.joint) + 0.009,
            "a pushed point still sat {} inside a surface of {}",
            hit.distance,
            surface.radius(hit.joint)
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
