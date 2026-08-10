//! Motion described so it survives being replayed on a body that did not exist
//! when it was written.
//!
//! A conventional clip stores joint rotations, which bakes in the skeleton it
//! was authored on. A clip here stores two things instead, both borrowed from
//! Spore's retargeting (Hecker et al., SIGGRAPH 2008):
//!
//! * a **semantic query** naming which parts it addresses — every ground
//!   contact, every grasper — rather than which bones;
//! * **goals in normalised body space**, measured as fractions of the limb's own
//!   reach rather than in metres.
//!
//! Both matter. The query lets a clip meet whatever body it lands on: "raise
//! both graspers" waves a biped's arms and does nothing to a quadruped, which
//! has none free, and neither case needs a special path. The normalised goal
//! lets a child and a giant perform the same gesture at their own scale.
//!
//! What is lost is the ability to specify an exact joint angle. That is the
//! trade: this format can express what a movement *is* for, and cannot express
//! motion that only means something on one particular skeleton.

use glam::Vec3;

use super::ground::solve_contact;
use super::pose::Pose;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// Which parts of a body a track addresses.
///
/// Resolved against the body at play time, so a track that finds nothing simply
/// does nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Every limb the body stands on.
    Contacts,
    /// Every limb free to hold something — a biped's hands, and nothing on a
    /// body that walks on all of its limbs.
    Graspers,
    /// One named limb, for motion that is genuinely one-sided.
    Just(Limb),
}

impl Target {
    /// The limbs this addresses on `rig`, in the rig's own order.
    #[must_use]
    pub fn resolve(self, rig: &Rig) -> Vec<Limb> {
        let contacts = rig.ground_contacts();
        match self {
            Target::Contacts => contacts,
            Target::Graspers => Limb::ALL
                .into_iter()
                .filter(|limb| {
                    !contacts.contains(limb) && !rig.in_zone(Zone::Extremity(*limb)).is_empty()
                })
                .collect(),
            Target::Just(limb) => {
                if rig.in_zone(Zone::Extremity(limb)).is_empty() {
                    Vec::new()
                } else {
                    vec![limb]
                }
            }
        }
    }
}

/// Where a part should be, at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    /// When, as a fraction of the clip's length.
    pub time: f32,
    /// Where, as a displacement from the part's rest position in multiples of
    /// its limb's reach.
    ///
    /// Measuring in reaches rather than metres is what makes the same gesture
    /// suit any body: `(0.0, 0.6, 0.4)` is "half a limb-length up and a little
    /// forward" whether the limb is a toddler's arm or a giant's.
    pub offset: Vec3,
}

impl Key {
    /// A key at `time` displacing the part by `offset` limb-reaches.
    #[must_use]
    pub fn new(time: f32, offset: Vec3) -> Self {
        Self { time, offset }
    }
}

/// One part's motion through a clip.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    /// Which parts this drives.
    pub target: Target,
    /// Keys in time order.
    pub keys: Vec<Key>,
}

impl Track {
    /// A track over `target` with the given keys, sorted into time order.
    #[must_use]
    pub fn new(target: Target, keys: impl Into<Vec<Key>>) -> Self {
        let mut keys = keys.into();
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self { target, keys }
    }

    /// Where this track wants its parts at `time`, in limb-reaches.
    ///
    /// Interpolation happens in goal space, not joint space. Two poses half a
    /// second apart interpolate to a sensible in-between *place*, where
    /// interpolated joint angles would swing a limb through wherever the
    /// arithmetic happened to lead.
    #[must_use]
    pub fn offset_at(&self, time: f32, looping: bool) -> Option<Vec3> {
        let first = self.keys.first()?;
        let last = self.keys.last()?;
        if self.keys.len() == 1 {
            return Some(first.offset);
        }

        if time <= first.time {
            return Some(if looping {
                // Wrapping round: blend from the last key back to the first.
                let span = 1.0 - last.time + first.time;
                let t = if span > f32::EPSILON {
                    (time + 1.0 - last.time) / span
                } else {
                    0.0
                };
                last.offset.lerp(first.offset, t.clamp(0.0, 1.0))
            } else {
                first.offset
            });
        }
        if time >= last.time {
            return Some(if looping {
                let span = 1.0 - last.time + first.time;
                let t = if span > f32::EPSILON {
                    (time - last.time) / span
                } else {
                    0.0
                };
                last.offset.lerp(first.offset, t.clamp(0.0, 1.0))
            } else {
                last.offset
            });
        }

        let index = self
            .keys
            .partition_point(|key| key.time <= time)
            .saturating_sub(1);
        let (before, after) = (self.keys[index], self.keys[index + 1]);
        let span = after.time - before.time;
        let t = if span > f32::EPSILON {
            (time - before.time) / span
        } else {
            0.0
        };
        Some(before.offset.lerp(after.offset, t))
    }
}

/// A motion, described by what it does rather than by how one body does it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clip {
    /// The parts this moves.
    pub tracks: Vec<Track>,
    /// Whether the end joins back onto the beginning.
    pub looping: bool,
}

impl Clip {
    /// A clip made of `tracks`.
    #[must_use]
    pub fn new(tracks: impl Into<Vec<Track>>) -> Self {
        Self {
            tracks: tracks.into(),
            looping: false,
        }
    }

    /// Marks the clip as joining its end back onto its beginning.
    #[must_use]
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    /// Poses `rig` at `time`, starting from its rest pose.
    #[must_use]
    pub fn pose(&self, rig: &Rig, time: f32) -> Pose {
        let mut pose = Pose::rest(rig);
        self.apply(rig, &mut pose, time);
        pose
    }

    /// Applies the clip to an existing pose, leaving untouched anything it does
    /// not address.
    ///
    /// Returns the limbs whose goal was out of reach, which is information a
    /// caller may want — a gesture that a stubby-armed body cannot complete is
    /// worth knowing about — rather than a failure.
    pub fn apply(&self, rig: &Rig, pose: &mut Pose, time: f32) -> Vec<Limb> {
        let time = if self.looping {
            time.rem_euclid(1.0)
        } else {
            time.clamp(0.0, 1.0)
        };
        let mut straining = Vec::new();

        for track in &self.tracks {
            let Some(offset) = track.offset_at(time, self.looping) else {
                continue;
            };
            for limb in track.target.resolve(rig) {
                let (Some(reach), Some(home)) = (rig.limb_reach(limb), rest_contact(rig, limb))
                else {
                    continue;
                };
                if !solve_contact(rig, pose, limb, home + offset * reach) {
                    straining.push(limb);
                }
            }
        }

        straining
    }
}

/// Where a limb's contact sits when the body is at rest.
fn rest_contact(rig: &Rig, limb: Limb) -> Option<Vec3> {
    let joint = *rig.in_zone(Zone::Extremity(limb)).first()?;
    Some(rig.joints[joint].position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    /// A hand raised and lowered.
    fn wave() -> Clip {
        Clip::new([Track::new(
            Target::Graspers,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(0.5, Vec3::new(0.0, 0.5, 0.35)),
                Key::new(1.0, Vec3::ZERO),
            ],
        )])
    }

    #[test]
    fn a_query_finds_the_parts_a_body_actually_has() {
        // The point of naming parts semantically: a biped has two free hands, a
        // quadruped has none, and neither needs a special case.
        assert_eq!(Target::Graspers.resolve(&biped()).len(), 2);
        assert_eq!(Target::Graspers.resolve(&quadruped()).len(), 0);
        assert_eq!(Target::Contacts.resolve(&biped()).len(), 2);
        assert_eq!(Target::Contacts.resolve(&quadruped()).len(), 4);
        assert_eq!(
            Target::Just(Limb::ForeLeft).resolve(&biped()),
            vec![Limb::ForeLeft]
        );
    }

    #[test]
    fn a_gesture_a_body_cannot_make_simply_does_not_happen() {
        let rig = quadruped();
        let before = Pose::rest(&rig);
        let after = wave().pose(&rig, 0.5);
        assert_eq!(before, after, "a quadruped has no free hand to wave");
    }

    #[test]
    fn the_same_gesture_scales_to_the_body_making_it() {
        // What normalised goals buy: one description, every body, at its own
        // size — and the same shape of movement in each.
        let reach_of = |height: f32| {
            let rig = Rig::from_skeleton(
                &HumanoidParams {
                    height,
                    ..Default::default()
                }
                .skeleton(&crate::Composites::default()),
            )
            .expect("rigs");
            let joint = rig.in_zone(Zone::Extremity(Limb::ForeLeft))[0];
            let rest = Pose::rest(&rig).forward(&rig).positions[joint];
            let raised = wave().pose(&rig, 0.5).forward(&rig).positions[joint];
            (
                raised.y - rest.y,
                rig.limb_reach(Limb::ForeLeft).expect("reach"),
            )
        };

        let (small_lift, small_reach) = reach_of(1.3);
        let (large_lift, large_reach) = reach_of(2.1);

        assert!(large_lift > small_lift, "a bigger body lifts further");
        // And by the same fraction of itself.
        let ratio = (large_lift / large_reach) / (small_lift / small_reach);
        assert!(
            (ratio - 1.0).abs() < 0.15,
            "the gesture should be the same size relative to each body, got {ratio:.3}"
        );
    }

    #[test]
    fn keys_interpolate_in_goal_space() {
        let track = Track::new(
            Target::Contacts,
            [
                Key::new(0.0, Vec3::ZERO),
                Key::new(1.0, Vec3::new(0.0, 1.0, 0.0)),
            ],
        );
        assert_eq!(track.offset_at(0.0, false), Some(Vec3::ZERO));
        assert_eq!(track.offset_at(0.5, false), Some(Vec3::new(0.0, 0.5, 0.0)));
        assert_eq!(track.offset_at(1.0, false), Some(Vec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn keys_are_held_outside_a_clip_that_does_not_loop() {
        let track = Track::new(
            Target::Contacts,
            [Key::new(0.25, Vec3::X), Key::new(0.75, Vec3::Y)],
        );
        assert_eq!(track.offset_at(0.0, false), Some(Vec3::X));
        assert_eq!(track.offset_at(1.0, false), Some(Vec3::Y));
    }

    #[test]
    fn a_looping_clip_joins_its_end_to_its_beginning() {
        let track = Track::new(
            Target::Contacts,
            [Key::new(0.0, Vec3::ZERO), Key::new(0.5, Vec3::Y)],
        );
        // Between the last key and the first, the wrap blends back.
        let three_quarters = track.offset_at(0.75, true).expect("interpolates");
        assert!(
            (three_quarters.y - 0.5).abs() < 1e-5,
            "halfway back round, got {three_quarters:?}"
        );
        assert_eq!(track.offset_at(1.0, true), track.offset_at(0.0, true));
    }

    #[test]
    fn keys_out_of_order_are_sorted_rather_than_trusted() {
        let track = Track::new(
            Target::Contacts,
            [Key::new(1.0, Vec3::Y), Key::new(0.0, Vec3::ZERO)],
        );
        assert_eq!(track.offset_at(0.5, false), Some(Vec3::new(0.0, 0.5, 0.0)));
    }

    #[test]
    fn a_clip_leaves_alone_what_it_does_not_address() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        wave().apply(&rig, &mut pose, 0.5);

        // Legs are untouched by a clip that only addresses graspers.
        let legs = rig.limb_chain(Limb::HindLeft).expect("a leg");
        for joint in legs {
            assert!(
                pose.rotations[joint].is_near_identity(),
                "the legs should not have moved"
            );
        }
    }

    #[test]
    fn an_unreachable_goal_is_reported_rather_than_hidden() {
        let rig = biped();
        let far = Clip::new([Track::new(
            Target::Graspers,
            [Key::new(0.0, Vec3::new(0.0, 8.0, 0.0))],
        )]);
        let mut pose = Pose::rest(&rig);
        let straining = far.apply(&rig, &mut pose, 0.0);
        assert_eq!(straining.len(), 2, "both arms should report the shortfall");
    }

    #[test]
    fn an_empty_clip_leaves_a_body_at_rest() {
        let rig = biped();
        assert_eq!(Clip::default().pose(&rig, 0.5), Pose::rest(&rig));
    }

    #[test]
    fn playing_a_clip_is_deterministic() {
        let rig = biped();
        assert_eq!(wave().pose(&rig, 0.3), wave().pose(&rig, 0.3));
    }
}
