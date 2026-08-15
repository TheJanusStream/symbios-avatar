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
//! **Body space is the body's, not the world's, and that took a fix** (#49). A
//! goal stated as a displacement from the part's rest position is only a
//! displacement from the *body* while the body is at rest. Laid over a walk it
//! is not: the gait has sunk the pelvis, wound the spine and pitched the trunk,
//! and a goal that stayed at the rest height sat up to 191 mm high on the body
//! it was supposed to be a gesture of — driving the arm toward its reach limit
//! instead of toward droop. Each track now says which frame it means, and a
//! body-relative one is resolved against the joint its limb hangs from. See
//! [`Space`].
//!
//! What is lost is the ability to specify an exact joint angle. That is the
//! trade: this format can express what a movement *is* for, and cannot express
//! motion that only means something on one particular skeleton.

use glam::{Quat, Vec3};

use super::ground::solve_contact_toward;
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
    ///
    /// Addresses the limb whatever it is for: a march and a kick lift a limb the
    /// body stands on, so this deliberately does not ask whether it is free.
    /// [`Target::Grasper`] is the one that asks.
    Just(Limb),
    /// One named limb, but only where it is free to gesture.
    ///
    /// **The per-item refusal an expressive roster needs** (#248). A greeting
    /// wave is one-handed — [`Target::Graspers`] would raise both — and it is
    /// also a motion a body walking on all four of its limbs simply cannot
    /// make. Neither [`Target::Just`] nor [`Target::Graspers`] can say both of
    /// those at once, so a gesture written with either says the wrong thing on
    /// one body or the other: `Just` waves a quadruped's front leg at you, and
    /// `Graspers` turns every one-handed gesture into a two-handed one.
    ///
    /// Resolving to nothing is the refusal, and it needs no special path: a
    /// track that finds no limb does nothing, and a clip of nothing but such
    /// tracks leaves the body alone.
    Grasper(Limb),
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
            Target::Grasper(limb) => {
                if contacts.contains(&limb) || rig.in_zone(Zone::Extremity(limb)).is_empty() {
                    Vec::new()
                } else {
                    vec![limb]
                }
            }
        }
    }
}

/// Whether a track's goal travels with the body or stays where it is.
///
/// The distinction is invisible on a body at rest and is the whole of the
/// behaviour on one that is doing anything else, which is the normal case: a
/// clip here is meant to be laid *over* a gait rather than played on its own.
///
/// A **gesture** is body-relative. A wave is a wave because of where the hand
/// is relative to the shoulder, so when the walk underneath it sinks the body
/// and pitches the trunk, the wave sinks and pitches with it. A **contact** is
/// world-relative. A foot on the ground stays on the ground whatever the pelvis
/// above it does; that is the entire job of a plant, and carrying the goal along
/// with the body would undo it.
///
/// **Why this is a property of the track**, rather than of [`Target`] or of the
/// caller. It is not [`Target`]'s to answer, because the two questions cross: a
/// march or a kick lifts a limb the body stands on and is body-relative while it
/// does, and a body bracing against a wall holds a grasper still in the world.
/// The correlation between "contact" and "world" is real and is not an identity,
/// and folding it into `Target` would make the common case free at the price of
/// making the other one unsayable. It is not the caller's either, because one
/// clip needs both answers at once — a wave that also holds the feet still is a
/// body-relative track and a world-relative track in the same [`Clip`] — and a
/// caller playing a clip does not know what its author meant. The track is where
/// the author already says what this part is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Space {
    /// Carried by the body: measured in the frame of the joint the limb hangs
    /// from, so the goal follows every crouch, lean and turn above it.
    #[default]
    Body,
    /// Fixed in the space the body is posed in, which is where the ground is.
    World,
}

/// What a track's offsets are measured in.
///
/// **Two normalisations, because a gesture means one of two different things**
/// and stating both in one unit is what makes a roster read wrong on half the
/// bodies it lands on (#248).
///
/// A **push, a stretch, a reach** is about how far the limb extends, and it
/// scales with the limb: a long-armed body pushing something away puts its hand
/// further out, and that is right. That is [`Scale::Reach`], and it is the
/// default because it is what the format was built for.
///
/// A **wave, a hand on the chest, a salute** is about where the hand IS on the
/// body, and it does not scale with the limb at all: a long-armed body waving
/// still puts its hand beside its own head, with a more folded elbow. Measured
/// in reaches, that gesture drifts — the audit puts it at 0.115 of a body
/// height between the short-limbed and long-limbed ends of the sweep, which is
/// a hand at the ear on one body and above the crown on the other.
///
/// The unit is a property of the track rather than of the clip, because one
/// gesture wants both: a refusal holds its hands at chest height, which is the
/// body's business, and pushes them out, which is the arm's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scale {
    /// Multiples of the limb's own reach, shoulder to extremity.
    #[default]
    Reach,
    /// Multiples of the body's own vertical extent — see [`Rig::extent`].
    Body,
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
    /// Whether the goal travels with the body or stays put.
    pub space: Space,
    /// What the offsets are measured in.
    pub scale: Scale,
}

impl Track {
    /// A track over `target` with the given keys, sorted into time order.
    ///
    /// Body-relative, because that is what a gesture is and gestures are what
    /// this format is for. A track that means to hold something still in the
    /// world says so with [`Self::in_world`].
    #[must_use]
    pub fn new(target: Target, keys: impl Into<Vec<Key>>) -> Self {
        let mut keys = keys.into();
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        Self {
            target,
            keys,
            space: Space::Body,
            scale: Scale::Reach,
        }
    }

    /// Measures this track's offsets in the body's own extent rather than in
    /// the limb's reach. See [`Scale`].
    #[must_use]
    pub fn on_body(mut self) -> Self {
        self.scale = Scale::Body;
        self
    }

    /// Fixes this track's goal in the space the body is posed in, rather than
    /// carrying it along with the body. See [`Space`].
    #[must_use]
    pub fn in_world(mut self) -> Self {
        self.space = Space::World;
        self
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
        if !pose.fits(rig) {
            return Vec::new();
        }
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
                let (Some(reach), Some(home), Some(pole)) = (
                    rig.limb_reach(limb),
                    rest_contact(rig, limb),
                    rig.bend_pole(limb),
                ) else {
                    continue;
                };
                // Both the goal and the fold direction go through the same
                // frame. Moving one without the other would put the limb on a
                // bend plane that drifts with the crouch.
                let frame = match track.space {
                    Space::World => Frame::REST,
                    Space::Body => Frame::carrying(rig, pose, limb),
                };
                let unit = match track.scale {
                    Scale::Reach => reach,
                    Scale::Body => rig.extent(),
                };
                let goal = frame.at(home + offset * unit);
                if !solve_contact_toward(rig, pose, limb, goal, frame.at(pole)) {
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

/// The rigid transform taking a point in the rig's rest space to where a pose
/// has carried it.
///
/// Everything a clip is written in — [`rest_contact`], [`Rig::bend_pole`], the
/// reach an offset is measured in — is stated about the rig at rest, because
/// that is the only body the author has. This is what turns those statements
/// into places on a body that is mid-stride.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    /// Orientation of the anchor in the posed body.
    rotation: Quat,
    /// Where the anchor has ended up.
    origin: Vec3,
    /// Where the anchor sits on the rig at rest.
    rest: Vec3,
}

impl Frame {
    /// The rig's own rest space, unmoved — what [`Space::World`] measures in.
    const REST: Self = Self {
        rotation: Quat::IDENTITY,
        origin: Vec3::ZERO,
        rest: Vec3::ZERO,
    };

    /// The frame `limb` is carried by: the joint its chain hangs from.
    ///
    /// **The chain's parent, not the chain's root and not the pelvis.** Not the
    /// chain root, because that is the joint the IK solve is about to rotate, so
    /// a goal stated in its frame would depend on the arm's previous pose and
    /// stop being the same gesture twice. Not the pelvis, because measured over
    /// a walk the shoulder girdle leaves the pelvis behind: the spine wind and
    /// the trunk lean move it 47 to 92 mm at moments when the root translation
    /// is exactly zero, and a goal hung off the root would carry all of that as
    /// error.
    ///
    /// A limb whose chain hangs straight off the root has no joint above it to
    /// borrow a frame from, and the root translation is the whole of what
    /// carries it.
    fn carrying(rig: &Rig, pose: &Pose, limb: Limb) -> Self {
        let anchor = rig
            .limb_chain(limb)
            .and_then(|chain| rig.joints[chain[0]].parent);
        let Some(anchor) = anchor else {
            return Self {
                origin: pose.translation,
                ..Self::REST
            };
        };
        let posed = pose.forward(rig);
        Self {
            rotation: posed.rotations[anchor],
            origin: posed.positions[anchor],
            rest: rig.joints[anchor].position,
        }
    }

    /// Where `point`, given in the rig's rest space, has been carried to.
    fn at(self, point: Vec3) -> Vec3 {
        self.origin + self.rotation * (point - self.rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim::gait::{self, Gait};
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

    /// A body sunk, pitched and turned, which is what a gait leaves behind.
    ///
    /// Deliberately not [`Walk`]: the arm swing changes the arm's own local
    /// rotations, and how far that moves an extremity through one solve is
    /// [`solve_contact`]'s business (#254), not this frame's. Everything here
    /// moves the body *above* the shoulder, which is exactly what #49 was
    /// missing, and it moves it three different ways — a translation, a
    /// rotation below the anchor, and a rotation of the root itself — because
    /// adding `pose.translation` answers only the first.
    fn carried(rig: &Rig, sink: f32, yaw: f32) -> Pose {
        let mut pose = Pose::rest(rig);
        pose.translation.y -= sink;
        pose.rotations[0] = Quat::from_rotation_y(yaw);
        gait::lean(
            rig,
            &mut pose,
            &Gait::wave(rig),
            &gait::Stride::for_body(rig, 1.0),
        );
        pose
    }

    /// The hand and the elbow, as the shoulder girdle sees them.
    fn arm_from_the_girdle(rig: &Rig, pose: &Pose) -> (Vec3, Vec3) {
        let chain = rig.limb_chain(Limb::ForeLeft).expect("an arm");
        let anchor = rig.joints[chain[0]].parent.expect("a girdle");
        let hand = rig.in_zone(Zone::Extremity(Limb::ForeLeft))[0];
        let posed = pose.forward(rig);
        // Undone by the anchor's own orientation, so this is the gesture rather
        // than the gesture plus wherever the body happens to be pointing.
        let seen = |joint: usize| {
            posed.rotations[anchor].inverse() * (posed.positions[joint] - posed.positions[anchor])
        };
        (seen(hand), seen(chain[1]))
    }

    #[test]
    fn a_gesture_is_the_same_gesture_on_a_body_that_has_moved_under_it() {
        // **#49.** A goal was measured from the part's REST position, which is
        // only a body-relative statement while the body is at rest. Over a walk
        // it is not: the gait sinks the pelvis by up to 117 mm at pace 1.5 and
        // the trunk lean carries the shoulder girdle 150 mm from where it
        // started, and a goal that stayed behind sat that far high on the body
        // it was supposed to be a gesture of — measured at 191 mm, which drove
        // the arm toward its reach limit instead of toward droop.
        let rig = biped();
        let (rest_hand, rest_elbow) = {
            let mut pose = Pose::rest(&rig);
            wave().apply(&rig, &mut pose, 0.5);
            arm_from_the_girdle(&rig, &pose)
        };

        for (sink, yaw) in [
            (0.0, 0.0),
            (0.047, 0.0),
            (0.117, 0.0),
            (0.117, 0.9),
            (0.0, -1.4),
        ] {
            let mut pose = carried(&rig, sink, yaw);
            wave().apply(&rig, &mut pose, 0.5);
            let (hand, elbow) = arm_from_the_girdle(&rig, &pose);
            assert!(
                hand.distance(rest_hand) < 1e-4,
                "sunk {sink} and turned {yaw}, the hand moved {:.1} mm on the body",
                hand.distance(rest_hand) * 1000.0
            );
            // The elbow too, which is the pole's half of it: the fold direction
            // is read off the rest pose and has to travel with the goal, or the
            // arm bends on a plane that drifts with the crouch.
            assert!(
                elbow.distance(rest_elbow) < 1e-4,
                "sunk {sink} and turned {yaw}, the elbow moved {:.1} mm on the body",
                elbow.distance(rest_elbow) * 1000.0
            );
        }
    }

    #[test]
    fn a_world_goal_stays_where_it_is_while_the_body_sinks_past_it() {
        // The other half of the decision, and the reason it is a property of
        // the track rather than a fix applied everywhere: a foot on the ground
        // does not follow the pelvis down. Same clip, same body, one word
        // different, opposite behaviour.
        let rig = biped();
        let hand = rig.in_zone(Zone::Extremity(Limb::ForeLeft))[0];
        let held =
            Clip::new([Track::new(Target::Graspers, [Key::new(0.0, Vec3::ZERO)]).in_world()]);

        let mut rest = Pose::rest(&rig);
        held.apply(&rig, &mut rest, 0.0);
        let anchored = rest.forward(&rig).positions[hand];

        // The same clip, one word different, on the same sunk body.
        let follows = Clip::new([Track::new(Target::Graspers, [Key::new(0.0, Vec3::ZERO)])]);

        let mut sunk = carried(&rig, 0.117, 0.0);
        // Where the hand would have gone had nothing held it at all.
        let adrift = sunk.forward(&rig).positions[hand].distance(anchored);
        let mut body = sunk.clone();
        follows.apply(&rig, &mut body, 0.0);
        let carried_down = body.forward(&rig).positions[hand].distance(anchored);
        held.apply(&rig, &mut sunk, 0.0);
        let stayed = sunk.forward(&rig).positions[hand].distance(anchored);

        assert!(
            adrift > 0.1,
            "the body did not move far enough for this to be a test: {adrift:.4}"
        );
        // The body-space track goes down with the body, as it should.
        assert!(
            carried_down > adrift * 0.9,
            "the body-space track held the hand back: {:.1} mm of {:.1}",
            carried_down * 1000.0,
            adrift * 1000.0
        );
        // The world-space one does not. It is not exactly still, and the
        // remainder is not this frame's: the solve corrects for the way the
        // hand hangs off the wrist using an offset it read before turning the
        // wrist, so it lands short by that much (#254). Measured at 38.1 mm
        // against a body that sank 117.
        assert!(
            stayed < adrift * 0.35,
            "a world-space goal followed the body down by {:.1} mm of {:.1}",
            stayed * 1000.0,
            adrift * 1000.0
        );
    }

    #[test]
    fn the_frame_a_gesture_is_measured_in_does_not_move_when_the_limb_does() {
        // **Why the anchor is the chain's parent and not the chain's root.**
        // The root is the joint the solve is about to rotate — and something
        // has usually rotated it already, since `swing_arms` swings the arms
        // before a clip is layered over the walk. Reading the frame off it
        // would make a gesture mean one thing at the top of the arm swing and
        // another at the bottom, which is the same class of defect as #49 and
        // harder to see.
        let rig = biped();
        let chain = rig.limb_chain(Limb::ForeLeft).expect("an arm");
        let rest = Frame::carrying(&rig, &Pose::rest(&rig), Limb::ForeLeft);

        let mut swung = Pose::rest(&rig);
        swung.rotations[chain[0]] = Quat::from_rotation_x(-0.6);
        swung.rotations[chain[1]] = Quat::from_rotation_x(0.4);
        assert_eq!(
            Frame::carrying(&rig, &swung, Limb::ForeLeft),
            rest,
            "the arm's own pose moved the frame its goal is measured in"
        );

        // And it does move when the body above it does, or it would be measuring
        // nothing at all.
        assert_ne!(
            Frame::carrying(&rig, &carried(&rig, 0.117, 0.4), Limb::ForeLeft),
            rest
        );
    }

    #[test]
    fn body_space_is_what_a_track_means_unless_it_says_otherwise() {
        // Gestures are what this format is for, and a gesture is body-relative.
        let track = Track::new(Target::Graspers, [Key::new(0.0, Vec3::ZERO)]);
        assert_eq!(track.space, Space::Body);
        assert_eq!(track.in_world().space, Space::World);
    }
}
