//! Motion that was authored on a body, retargeted onto ours and baked.
//!
//! **This is the second clip format, and it exists beside [`Clip`] rather than
//! instead of it.** The two answer different questions and neither can
//! answer the other's:
//!
//! * [`Clip`] describes motion by semantic query and normalised goal — raise
//!   both graspers, put every contact here — so one description serves a biped,
//!   a quadruped and a body nobody has built yet. Its own module doc states what
//!   it gives up for that: it cannot specify an exact joint angle.
//! * A [`PoseClip`] is exactly the thing that trade forbids. It carries a
//!   rotation per joint per frame, because it comes from a file where an
//!   animator turned those joints, and no query recovers what they meant.
//!
//! The reason the second format had to exist is measurable rather than a matter
//! of taste. The CC0 reference library this was written for is 162 clips of
//! human performance — *Bow*, *Confused*, *Meditate*, *Idle_Talking* — and what
//! makes any of them read is the spine, the neck and the head. `Clip` addresses
//! contacts and graspers, so baking that library into it would keep the hands
//! and feet and throw away the performance.
//!
//! # What a track is keyed on, and why not a joint index
//!
//! On a [`Slot`] — a zone and an ordinal within it — not on an index into
//! [`Rig::joints`]. A rig's joint indices depend on what the body turned out to
//! have: hair springs and attached parts are appended after the skeleton's own,
//! so an index baked against one body addresses something else on another. A
//! slot is resolved against whatever rig is actually present, and a track whose
//! slot finds nothing does nothing — the same way [`Clip`]'s queries behave, and
//! for the same reason.
//!
//! # What it costs, measured
//!
//! Rotations are quantised to four `i16`, so eight bytes per joint per frame,
//! and a track whose rotation never changes collapses to a single value. That
//! collapse is not a micro-optimisation: measured on the reference library's own
//! channels, **45 of 66 rotation tracks are constant through `Walk`, 50 through
//! `Idle_A` and 48 through `Dance_Simple`** — because forty of those joints are
//! fingers and a walking body does not use them.
//!
//! So including the fingers roughly triples the track *count* and adds almost
//! nothing to the bytes, which is the arithmetic behind baking them in. A
//! 1.4-second clip at 30 frames a second with twenty moving
//! tracks is about 6.7 KiB of rotation and half a KiB of root motion.
//!
//! [`Clip`]: super::clip::Clip

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::pose::Pose;
use crate::plan::Zone;
use crate::rig::Rig;

/// The scale a quaternion component is stored at.
///
/// A unit quaternion's components are in `-1..=1`, so a signed 16-bit fixed
/// point covers them at a resolution of about three ten-thousandths — roughly
/// two hundredths of a degree of rotation, which is far below what any of this
/// is measured to and a quarter of the memory of `f32`.
const SCALE: f32 = 32767.0;

/// Which joint of a body a track drives.
///
/// **A semantic address rather than an index**, so one baked clip plays on every
/// humanoid. [`Rig::in_zone`] returns a zone's joints in the rig's own order,
/// which is breadth-first and so parents before children, and the pair
/// `(zone, index)` names the same anatomy on any body built from the same plan:
/// `(Chest, 0)` is the first joint of the chest wherever it landed in the joint
/// list.
///
/// **`index` is an ordinal within the zone, not a distance along a chain**, and
/// on a branching zone those are different things. A hand's twenty-one joints
/// come out level by level rather than finger by finger, so
/// `(Extremity(ForeLeft), 3)` is the fourth hand joint the rig lists and not the
/// third bone out along one digit. That is a stable address either way — which
/// is all a slot has to be — but anything reading a slot as anatomy rather than
/// as a name will be wrong about a hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// Which region of the body.
    pub zone: Zone,
    /// Which joint of that zone, in hierarchy order.
    pub index: u8,
}

impl Slot {
    /// Names a joint by zone and ordinal.
    #[must_use]
    pub fn new(zone: Zone, index: u8) -> Self {
        Self { zone, index }
    }

    /// Which joint of `rig` this is, if the body has one.
    ///
    /// `None` for a body that does not — a quadruped asked for a grasper, a
    /// body whose hand carries fewer joints than the clip was baked against.
    /// That is a track doing nothing rather than an error, which is what lets
    /// one library of clips meet bodies it was not baked for.
    #[must_use]
    pub fn resolve(self, rig: &Rig) -> Option<usize> {
        rig.in_zone(self.zone).get(self.index as usize).copied()
    }
}

/// One joint's rotation over a clip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Curve {
    /// One rotation for the whole clip.
    ///
    /// Most tracks in most clips. See the module docs for the measurement.
    Held([i16; 4]),
    /// A rotation per frame, at the clip's own rate.
    Sampled(Vec<[i16; 4]>),
}

impl Curve {
    /// Bakes samples into a curve, collapsing one that never moves.
    ///
    /// `tolerance` is in quaternion component units; anything under it counts as
    /// not moving. Consecutive samples are put in the same hemisphere before
    /// they are compared or stored, because `q` and `-q` are the same rotation
    /// and a track that flips between them is a still joint that looks like it
    /// spins — and would defeat the collapse as well as the interpolation.
    #[must_use]
    pub fn bake(samples: &[Quat], tolerance: f32) -> Self {
        let Some(&first) = samples.first() else {
            return Self::Held(pack(Quat::IDENTITY));
        };
        let mut aligned = Vec::with_capacity(samples.len());
        let mut previous = first.normalize();
        aligned.push(previous);
        for &sample in &samples[1..] {
            let sample = sample.normalize();
            let sample = if sample.dot(previous) < 0.0 {
                -sample
            } else {
                sample
            };
            aligned.push(sample);
            previous = sample;
        }

        let held = aligned[0];
        if aligned
            .iter()
            .all(|sample| component_distance(*sample, held) <= tolerance)
        {
            return Self::Held(pack(held));
        }
        Self::Sampled(aligned.into_iter().map(pack).collect())
    }

    /// The rotation between two frames, blended.
    #[must_use]
    fn at(&self, before: usize, after: usize, blend: f32) -> Quat {
        match self {
            Self::Held(value) => unpack(*value),
            Self::Sampled(values) => {
                if values.is_empty() {
                    return Quat::IDENTITY;
                }
                let a = unpack(values[before.min(values.len() - 1)]);
                let b = unpack(values[after.min(values.len() - 1)]);
                a.slerp(b, blend)
            }
        }
    }

    /// How many bytes this curve occupies.
    #[must_use]
    pub fn bytes(&self) -> usize {
        match self {
            Self::Held(_) => 8,
            Self::Sampled(values) => values.len() * 8,
        }
    }
}

/// One joint's motion through a clip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JointTrack {
    /// Which joint it drives.
    pub slot: Slot,
    /// How that joint is turned, relative to its rest orientation.
    pub rotation: Curve,
}

/// Motion baked against a body plan, one rotation per joint per frame.
///
/// Sampled at a uniform rate rather than keyed at arbitrary times. The source
/// files this is baked from are two thirds `STEP` samplers at irregular times,
/// so nothing is lost by resampling and a great deal of indexing arithmetic is:
/// a frame is a multiplication, and a track is a flat array with no times beside
/// it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoseClip {
    /// What it is called, from whatever it was baked out of.
    pub name: String,
    /// Frames per second.
    pub rate: f32,
    /// How many frames each sampled track carries.
    pub frames: usize,
    /// Whether it runs round rather than stopping.
    ///
    /// A looping clip's last frame is **not** a copy of its first: the wrap from
    /// the last frame back to the first is one frame long like every other, and
    /// duplicating the ends puts a stutter in every cycle.
    pub looping: bool,
    /// One track per joint the clip moves.
    pub tracks: Vec<JointTrack>,
    /// Where the root sits each frame, relative to its rest position.
    ///
    /// Empty for a clip that stays put. The reference library's `_RM` clips
    /// carry root motion and the rest do not, and a dodge or a roll is nothing
    /// without it.
    pub root: Vec<Vec3>,
}

impl PoseClip {
    /// How long it runs, in seconds.
    ///
    /// A looping clip runs for `frames / rate`, because the wrap back to the
    /// first frame is itself a frame; a one-shot runs to its last frame, which
    /// is one frame less.
    #[must_use]
    pub fn duration(&self) -> f32 {
        if self.rate <= 0.0 || self.frames == 0 {
            return 0.0;
        }
        let frames = if self.looping {
            self.frames as f32
        } else {
            (self.frames - 1) as f32
        };
        frames / self.rate
    }

    /// Which two frames `time` falls between, and how far between them.
    #[must_use]
    fn span(&self, time: f32) -> (usize, usize, f32) {
        if self.frames == 0 || self.rate <= 0.0 {
            return (0, 0, 0.0);
        }
        let last = self.frames - 1;
        let at = time * self.rate;
        if self.looping {
            let wrapped = at.rem_euclid(self.frames as f32);
            let before = wrapped.floor() as usize % self.frames;
            return (before, (before + 1) % self.frames, wrapped.fract());
        }
        if at <= 0.0 {
            return (0, 0, 0.0);
        }
        if at >= last as f32 {
            return (last, last, 0.0);
        }
        let before = at.floor() as usize;
        (before, (before + 1).min(last), at.fract())
    }

    /// Writes this clip's joints into `pose`, leaving every other joint alone.
    ///
    /// **Leaving the rest alone is the point.** It is what lets an imported
    /// gesture play over a procedural walk: bake the upper body, apply it after
    /// [`gait::step`], and the legs keep whatever the gait gave them. A clip that
    /// overwrote the whole pose could only ever be the only thing playing.
    ///
    /// Root translation is written only if the clip carries any, so a gesture
    /// baked without root motion does not drag a walking body back to the
    /// origin.
    ///
    /// [`gait::step`]: crate::anim::gait::step
    pub fn apply(&self, rig: &Rig, pose: &mut Pose, time: f32) {
        let (before, after, blend) = self.span(time);
        for track in &self.tracks {
            let Some(joint) = track.slot.resolve(rig) else {
                continue;
            };
            if joint < pose.rotations.len() {
                pose.rotations[joint] = track.rotation.at(before, after, blend);
            }
        }
        if !self.root.is_empty() {
            let a = self.root[before.min(self.root.len() - 1)];
            let b = self.root[after.min(self.root.len() - 1)];
            pose.translation = a.lerp(b, blend);
        }
    }

    /// This clip alone, on a rig, at `time`.
    #[must_use]
    pub fn pose(&self, rig: &Rig, time: f32) -> Pose {
        let mut pose = Pose::rest(rig);
        self.apply(rig, &mut pose, time);
        pose
    }

    /// How many bytes its curves occupy.
    ///
    /// The figure the baked artifact is budgeted against. Counts the motion
    /// rather than the container: names and structure are the same handful of
    /// bytes whatever they are serialised into, and the curves are not.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.tracks
            .iter()
            .map(|track| track.rotation.bytes())
            .sum::<usize>()
            + self.root.len() * 12
    }

    /// This clip alone, at one of its own frames, with no interpolation.
    ///
    /// [`Self::pose`] takes a time and lands between two frames; this lands ON
    /// one, which is what a continuity reading needs — asked through a time,
    /// `frame / rate * rate` comes back a hair under the frame it names and the
    /// answer is smeared with its neighbour.
    #[must_use]
    fn at_frame(&self, rig: &Rig, frame: usize) -> Pose {
        let mut pose = Pose::rest(rig);
        for track in &self.tracks {
            let Some(joint) = track.slot.resolve(rig) else {
                continue;
            };
            if joint < pose.rotations.len() {
                pose.rotations[joint] = track.rotation.at(frame, frame, 0.0);
            }
        }
        if !self.root.is_empty() {
            pose.translation = self.root[frame.min(self.root.len() - 1)];
        }
        pose
    }

    /// What this clip does to a body between its own frames.
    ///
    /// **The measurement that keeps a baked clip a reference rather than a gold
    /// standard.** In the imported set the clips do not loop cleanly, and on
    /// some of them the body teleports between frames, as if a reference frame
    /// had changed under it. Both are real and both are a number every
    /// comparison inherits, rather than a caveat somebody has to remember.
    ///
    /// See [`Continuity`] for what the two readings are and why neither is
    /// asked as "does it close".
    #[must_use]
    pub fn continuity(&self, rig: &Rig) -> Continuity {
        let empty = Continuity {
            step: 0.0,
            jump: 0.0,
            jump_at: 0,
            seam: None,
        };
        if self.frames < 2 {
            return empty;
        }
        let places: Vec<Vec<Vec3>> = (0..self.frames)
            .map(|frame| self.at_frame(rig, frame).forward(rig).positions)
            .collect();
        // **The furthest any ONE joint moves**, not the mean over joints: a
        // teleport is a body arriving somewhere else, and a mean over
        // seventy-odd joints of which two moved would hide one.
        let travel = |a: &[Vec3], b: &[Vec3]| {
            a.iter()
                .zip(b)
                .fold(0.0f32, |most, (from, to)| most.max(from.distance(*to)))
        };
        let steps: Vec<f32> = places
            .windows(2)
            .map(|pair| travel(&pair[0], &pair[1]))
            .collect();
        let (jump_at, jump) =
            steps
                .iter()
                .enumerate()
                .fold((0usize, 0.0f32), |worst, (at, step)| {
                    if *step > worst.1 {
                        (at + 1, *step)
                    } else {
                        worst
                    }
                });
        // The median, so one teleport does not raise the family it is being
        // compared against — which is exactly what a mean would let it do.
        let mut sorted = steps.clone();
        sorted.sort_by(f32::total_cmp);
        Continuity {
            step: sorted[sorted.len() / 2],
            jump,
            jump_at,
            // **Excluded from the family it is judged against.** The wrap is the
            // thing being asked about, so a clip whose wrap is enormous must not
            // get to raise its own median with it.
            seam: self
                .looping
                .then(|| travel(&places[self.frames - 1], &places[0])),
        }
    }

    /// How many of its tracks actually move.
    ///
    /// Beside [`Self::bytes`] because the ratio is the thing worth watching: a
    /// clip whose every track is sampled either has a body doing a great deal or
    /// a bake whose tolerance is too tight.
    #[must_use]
    pub fn moving(&self) -> usize {
        self.tracks
            .iter()
            .filter(|track| matches!(track.rotation, Curve::Sampled(_)))
            .count()
    }
}

/// What a baked clip does to a body between its own frames.
///
/// **A wrapping motion cannot be asked whether it closes, only whether the step
/// across the wrap is in family.** Every frame of a clip moves the body some
/// distance; a loop that closes is one whose wrap moves it about as far as the
/// frames either side of it do. The absolute distance says nothing on its own —
/// a sprint's every frame moves further than an idle's — so the reading that
/// means something is the RATIO to the clip's own family, and the family is the
/// MEDIAN step rather than the mean, because a mean lets one teleport raise the
/// bar it is being measured against.
///
/// **A ratio far from one is the defect, and it is a different defect on each
/// side.** Much above one is a jerk: the body is somewhere else at the top of
/// the cycle. Much BELOW one is a stutter, and it is the more common of the two
/// here — a wrap that moves the body a third of a frame's distance is a body
/// pausing for a frame every cycle, which is what a last frame duplicating the
/// first produces (see [`PoseClip::looping`]). Measured on the shipped
/// artifact, none of the twelve jerks at the wrap and half of them pause there.
///
/// The same reading answers the second defect. A teleport is a step much larger
/// than the family wherever it happens; a seam is one that happens at the wrap.
/// One pass finds both, and [`Self::jump_at`] says which frame to look at.
///
/// Distances are metres on the rig the reading was taken on, which is why the
/// ratios are the figures that travel and the metres are the ones that make
/// them concrete.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Continuity {
    /// The median distance the furthest-moving joint travels between two
    /// adjacent frames.
    pub step: f32,
    /// The largest such distance anywhere inside the clip.
    pub jump: f32,
    /// The frame [`Self::jump`] arrives at.
    pub jump_at: usize,
    /// The same distance across a loop's wrap, last frame back to first.
    ///
    /// `None` for a clip that does not loop, where the question does not arise:
    /// a one-shot has no wrap and asking about one would invent a defect.
    pub seam: Option<f32>,
}

impl Continuity {
    /// How far out of family the worst step inside the clip is.
    ///
    /// One means the clip's own median; a smooth motion sits near it. Returns
    /// `0.0` for a clip that never moves, where a ratio has no meaning.
    #[must_use]
    pub fn jump_ratio(&self) -> f32 {
        if self.step <= f32::EPSILON {
            0.0
        } else {
            self.jump / self.step
        }
    }

    /// How far out of family the step across the wrap is, for a loop.
    #[must_use]
    pub fn seam_ratio(&self) -> Option<f32> {
        let seam = self.seam?;
        (self.step > f32::EPSILON).then(|| seam / self.step)
    }
}

/// A clip being played: where it has got to, and how fast it runs.
///
/// Deliberately small and deliberately not a scheduler. Transitions between
/// clips belong to [`Inertializer`], which already carries momentum through a
/// change rather than crossfading it to a stall, and choosing *which* clip plays
/// belongs to whatever is driving the body. This is the cursor.
///
/// [`Inertializer`]: crate::anim::Inertializer
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Play {
    /// Seconds into the clip.
    pub time: f32,
    /// How fast it runs, as a multiple. Negative plays it backwards.
    pub speed: f32,
    /// Whether a one-shot has run out. Never true while looping.
    done: bool,
}

impl Default for Play {
    fn default() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            done: false,
        }
    }
}

impl Play {
    /// A cursor at the start of a clip.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the cursor on by `dt` seconds.
    ///
    /// A looping clip wraps and never finishes. A one-shot stops at its last
    /// frame and reports [`Self::finished`], which is the signal to blend back
    /// to whatever was underneath — held there rather than snapped to rest,
    /// because a one-shot that ends by teleporting to the rest pose is worse
    /// than one that ends late.
    pub fn advance(&mut self, clip: &PoseClip, dt: f32) {
        let duration = clip.duration();
        if duration <= 0.0 {
            self.done = !clip.looping;
            return;
        }
        self.time += dt * self.speed;
        if clip.looping {
            self.time = self.time.rem_euclid(duration);
            self.done = false;
        } else if self.time >= duration {
            self.time = duration;
            self.done = true;
        } else if self.time <= 0.0 {
            self.time = 0.0;
            self.done = self.speed < 0.0;
        }
    }

    /// Whether a one-shot has run out.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.done
    }

    /// Restarts it.
    pub fn rewind(&mut self) {
        self.time = 0.0;
        self.done = false;
    }

    /// Writes the clip's joints into `pose` at the cursor.
    pub fn apply(&self, clip: &PoseClip, rig: &Rig, pose: &mut Pose) {
        clip.apply(rig, pose, self.time);
    }

    /// The clip alone, at the cursor.
    #[must_use]
    pub fn pose(&self, clip: &PoseClip, rig: &Rig) -> Pose {
        clip.pose(rig, self.time)
    }
}

/// A unit quaternion as four fixed-point components.
fn pack(rotation: Quat) -> [i16; 4] {
    let rotation = rotation.normalize();
    [rotation.x, rotation.y, rotation.z, rotation.w]
        .map(|component| (component.clamp(-1.0, 1.0) * SCALE).round() as i16)
}

/// Four fixed-point components back to a unit quaternion.
fn unpack(value: [i16; 4]) -> Quat {
    let [x, y, z, w] = value.map(|component| f32::from(component) / SCALE);
    let rotation = Quat::from_xyzw(x, y, z, w);
    // Rounding four components independently leaves the result a hair off unit,
    // and a hair off unit scales whatever it deforms.
    if rotation.length_squared() > f32::EPSILON {
        rotation.normalize()
    } else {
        Quat::IDENTITY
    }
}

/// The largest component-wise difference between two rotations.
///
/// Compared component-wise rather than by angle because that is the quantity the
/// storage is in, so a tolerance expressed here means the same thing as the
/// error the packing introduces.
fn component_distance(a: Quat, b: Quat) -> f32 {
    (a.x - b.x)
        .abs()
        .max((a.y - b.y).abs())
        .max((a.z - b.z).abs())
        .max((a.w - b.w).abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Limb;

    /// A rig to resolve slots against.
    fn rig() -> Rig {
        Rig::from_skeleton(&crate::AvatarRecord::default().skeleton()).expect("a humanoid rigs")
    }

    /// A clip turning one joint from rest to a quarter turn over four frames.
    fn turning(zone: Zone, looping: bool) -> PoseClip {
        let samples: Vec<Quat> = (0..4)
            .map(|frame| Quat::from_rotation_z(frame as f32 / 3.0 * std::f32::consts::FRAC_PI_2))
            .collect();
        PoseClip {
            name: "Turn".into(),
            rate: 3.0,
            frames: 4,
            looping,
            tracks: vec![JointTrack {
                slot: Slot::new(zone, 0),
                rotation: Curve::bake(&samples, 1e-4),
            }],
            root: Vec::new(),
        }
    }

    /// A clip spinning one joint at a constant rate through a whole turn, so
    /// that its wrap back to the first frame is one step like every other.
    ///
    /// **A whole turn and not part of one**, because that is what makes the
    /// loop honest: the last frame sits one step short of the first, so wrapping
    /// costs exactly what any other frame does.
    fn spinning(frames: usize) -> PoseClip {
        let samples: Vec<Quat> = (0..frames)
            .map(|frame| {
                Quat::from_rotation_z(frame as f32 / frames as f32 * std::f32::consts::TAU)
            })
            .collect();
        PoseClip {
            name: "Spin".into(),
            rate: frames as f32,
            frames,
            looping: true,
            tracks: vec![JointTrack {
                slot: Slot::new(Zone::Chest, 0),
                rotation: Curve::bake(&samples, 1e-4),
            }],
            root: Vec::new(),
        }
    }

    #[test]
    fn a_loop_whose_wrap_costs_what_a_frame_costs_reads_as_closed() {
        // **A wrapping motion cannot be asked whether it CLOSES**, only whether
        // the step across the wrap is in family, and this is the family: a
        // constant-rate spin, where every step including the wrap is the same
        // size. One is what that has to read.
        let read = spinning(24).continuity(&rig());
        let ratio = read.seam_ratio().expect("a loop has a seam");
        assert!(
            (ratio - 1.0).abs() < 0.05,
            "a constant-rate loop's wrap read {ratio:.2} of its own median step",
        );
    }

    #[test]
    fn a_loop_that_repeats_its_first_frame_reads_as_a_pause() {
        // **The defect the shipped set actually has**, and it is the one below
        // one rather than above it. A last frame that duplicates the first
        // makes the wrap cost nothing, so the body holds still for a frame
        // every cycle — which is what [`PoseClip::looping`] warns about and
        // what six of the twelve imported clips read as, down to 0.1 on
        // `Sleeping` and 0.4 on `Jog`.
        //
        // Built by taking a whole turn's worth of frames and asking for one
        // more, so the extra frame lands back on the start.
        let mut clip = spinning(24);
        let samples: Vec<Quat> = (0..=24)
            .map(|frame| Quat::from_rotation_z(frame as f32 / 24.0 * std::f32::consts::TAU))
            .collect();
        clip.frames = 25;
        clip.tracks[0].rotation = Curve::bake(&samples, 1e-4);
        let ratio = clip
            .continuity(&rig())
            .seam_ratio()
            .expect("a loop has a seam");
        assert!(
            ratio < 0.1,
            "a loop repeating its first frame read {ratio:.2}, which is not a pause",
        );
    }

    #[test]
    fn a_teleport_is_found_and_named() {
        // The second half of the owner's report: on some clips the body
        // arrives somewhere else between two frames. A ratio against the
        // clip's own MEDIAN step is what finds it — against the mean, a jump
        // large enough to matter raises the bar it is measured against — and
        // the frame is reported because a defect that cannot be pointed at has
        // to be hunted for by eye.
        //
        // Measured on the shipped artifact, `Bow` reads 24.5 at frame 71 of 113
        // and `Reject` 6.0 at frame 79 of 114.
        let mut clip = spinning(24);
        let mut samples: Vec<Quat> = (0..24)
            .map(|frame| Quat::from_rotation_z(frame as f32 / 24.0 * std::f32::consts::TAU))
            .collect();
        // One frame thrown a quarter turn off the path it was on.
        samples[10] = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * samples[10];
        clip.tracks[0].rotation = Curve::bake(&samples, 1e-4);
        let read = clip.continuity(&rig());
        assert!(
            read.jump_ratio() > 4.0,
            "a thrown frame read {:.1} of the clip's own median step",
            read.jump_ratio(),
        );
        assert_eq!(
            read.jump_at, 10,
            "the jump was reported at frame {} rather than at the one that moved",
            read.jump_at,
        );
    }

    #[test]
    fn a_one_shot_is_not_asked_about_a_wrap_it_does_not_have() {
        // Asking a one-shot whether it loops cleanly invents a defect: its last
        // frame is meant to be somewhere else entirely. `None` is the answer,
        // and the four expressive clips in the shipped set are all of this
        // kind.
        let read = turning(Zone::Chest, false).continuity(&rig());
        assert_eq!(read.seam, None);
        assert_eq!(read.seam_ratio(), None);
        assert!(read.jump > 0.0, "a one-shot is still asked about teleports");
    }

    #[test]
    fn a_slot_names_the_same_anatomy_on_any_body() {
        let rig = rig();
        let chest = Slot::new(Zone::Chest, 0).resolve(&rig).expect("a chest");
        assert_eq!(rig.joints[chest].zone, Zone::Chest);
        assert_eq!(
            Slot::new(Zone::Chest, 0).resolve(&rig),
            Some(rig.in_zone(Zone::Chest)[0]),
            "a slot is the zone's own hierarchy order and nothing else"
        );
        // A body that does not have what the slot asks for does nothing rather
        // than failing, which is what lets one library meet many bodies.
        assert_eq!(Slot::new(Zone::Chest, 200).resolve(&rig), None);
        assert_eq!(Slot::new(Zone::Tail, 0).resolve(&rig), None);
    }

    #[test]
    fn a_still_track_collapses_and_a_moving_one_does_not() {
        // The measurement this format is built around: 45 of the reference
        // library's 66 rotation tracks are constant through a WALK, because
        // forty of those joints are fingers. Collapsing them is most of the
        // format's compactness, and it must not collapse anything that moves.
        let still = Curve::bake(&[Quat::IDENTITY; 30], 1e-4);
        assert!(matches!(still, Curve::Held(_)));
        assert_eq!(still.bytes(), 8);

        let moving = Curve::bake(
            &(0..30)
                .map(|frame| Quat::from_rotation_x(frame as f32 * 0.05))
                .collect::<Vec<_>>(),
            1e-4,
        );
        assert!(matches!(moving, Curve::Sampled(_)));
        assert_eq!(moving.bytes(), 240);

        // q and -q are the same rotation. A track that flips between them is a
        // STILL joint, and a bake that read the sign as motion would sample all
        // forty finger tracks of every clip in the library.
        let flipping: Vec<Quat> = (0..8)
            .map(|frame| {
                if frame % 2 == 0 {
                    Quat::IDENTITY
                } else {
                    -Quat::IDENTITY
                }
            })
            .collect();
        assert!(
            matches!(Curve::bake(&flipping, 1e-4), Curve::Held(_)),
            "a sign flip is not motion"
        );
    }

    #[test]
    fn quantisation_costs_less_than_a_hundredth_of_a_degree() {
        // The claim the format's size rests on, checked rather than asserted in
        // prose. Swept over rotations about every axis at every angle.
        let mut worst = 0.0f32;
        for step in 0..360 {
            let angle = (step as f32).to_radians();
            for axis in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE.normalize()] {
                let original = Quat::from_axis_angle(axis, angle);
                let restored = unpack(pack(original));
                // Compared as a rotation rather than component-wise: the
                // question is how far a point moves, not how far a number did.
                let moved = (original * Vec3::X).distance(restored * Vec3::X);
                worst = worst.max(moved);
            }
        }
        assert!(
            worst < 1e-4,
            "quantisation moves a unit vector by {worst}, which is {} degrees",
            worst.to_degrees()
        );
    }

    #[test]
    fn a_clip_writes_only_the_joints_it_has_tracks_for() {
        // **The property that lets an imported gesture play over a procedural
        // walk.** A clip that overwrote the whole pose could only ever be the
        // only thing playing.
        let rig = rig();
        let clip = turning(Zone::Chest, false);
        let chest = Slot::new(Zone::Chest, 0).resolve(&rig).expect("a chest");

        let mut pose = Pose::rest(&rig);
        let elsewhere = Slot::new(Zone::UpperLimb(Limb::HindLeft), 0)
            .resolve(&rig)
            .expect("a thigh");
        let marked = Quat::from_rotation_x(0.3);
        pose.rotations[elsewhere] = marked;
        pose.translation = Vec3::new(0.0, 1.0, 0.0);

        clip.apply(&rig, &mut pose, clip.duration());
        assert!(
            pose.rotations[chest].angle_between(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2))
                < 1e-3,
            "the clip's own joint did not land on its last frame"
        );
        assert_eq!(
            pose.rotations[elsewhere], marked,
            "a joint the clip has no track for was overwritten"
        );
        assert_eq!(
            pose.translation,
            Vec3::new(0.0, 1.0, 0.0),
            "a clip with no root motion moved the root"
        );
    }

    #[test]
    fn a_one_shot_stops_at_its_last_frame_and_a_loop_wraps() {
        let rig = rig();
        let chest = Slot::new(Zone::Chest, 0).resolve(&rig).expect("a chest");
        let quarter = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);

        // A one-shot: four frames at 3 fps is one second, and it holds there.
        let shot = turning(Zone::Chest, false);
        assert!((shot.duration() - 1.0).abs() < 1e-6);
        let mut play = Play::new();
        play.advance(&shot, 0.5);
        assert!(!play.finished());
        play.advance(&shot, 5.0);
        assert!(play.finished());
        assert!(
            (play.time - 1.0).abs() < 1e-6,
            "a one-shot ran past its end"
        );
        assert!(play.pose(&shot, &rig).rotations[chest].angle_between(quarter) < 1e-3);

        // A loop: the wrap back to the first frame is itself a frame, so four
        // frames at 3 fps run for four thirds of a second, not one.
        let round = turning(Zone::Chest, true);
        assert!((round.duration() - 4.0 / 3.0).abs() < 1e-6);
        let mut play = Play::new();
        play.advance(&round, 4.0 / 3.0 + 0.25);
        assert!(!play.finished(), "a looping clip never finishes");
        assert!(
            (play.time - 0.25).abs() < 1e-6,
            "a loop did not wrap cleanly: {}",
            play.time
        );

        // And it wraps in pose as well as in time: half way through the wrap
        // frame the joint is between its last rotation and its first, which a
        // clip that duplicated its ends would render as a stutter.
        let wrapping = round.pose(&rig, 3.5 / 3.0).rotations[chest];
        assert!(
            wrapping.angle_between(quarter) > 1e-3 && wrapping.angle_between(Quat::IDENTITY) > 1e-3,
            "the wrap frame is not interpolating"
        );
    }

    #[test]
    fn root_motion_is_carried_only_by_clips_that_have_it() {
        let rig = rig();
        let mut clip = turning(Zone::Chest, false);
        clip.root = vec![Vec3::ZERO, Vec3::Z, Vec3::Z * 2.0, Vec3::Z * 3.0];

        let half = clip.pose(&rig, 0.5);
        assert!(
            half.translation.abs_diff_eq(Vec3::Z * 1.5, 1e-5),
            "root motion did not interpolate: {:?}",
            half.translation
        );
        assert!(clip.bytes() > 0);
        assert_eq!(clip.moving(), 1);
    }

    #[test]
    fn a_clip_with_no_frames_poses_a_rest_pose() {
        // Empty is a real case: #140 will bake whatever the library holds, and a
        // pose-only entry has one frame or none. Neither may panic.
        let rig = rig();
        let empty = PoseClip::default();
        assert_eq!(empty.duration(), 0.0);
        assert_eq!(empty.pose(&rig, 3.0), Pose::rest(&rig));

        let mut play = Play::new();
        play.advance(&empty, 1.0);
        assert!(play.finished(), "an empty one-shot is over immediately");
    }
}
