//! Walking, for whatever number of legs a body has.
//!
//! A gait is two numbers per body: a **phase offset** for each ground contact,
//! saying when in the cycle it lifts, and a **duty factor**, saying what
//! fraction of the cycle it spends down. Everything else — a biped's walk, a
//! horse's trot, a wave gait rippling down a centipede — falls out of those.
//! That is what lets one implementation serve bodies whose leg counts differ,
//! which no library of authored walk cycles can do.
//!
//! Feet are placed by *goal*, not by joint angle: this contact belongs here now.
//! Where "here" is comes from the body's own rest stance and its stride, so a
//! short body takes short steps and a long-legged one takes long ones without
//! anything being retuned. Inverse kinematics turns the goals back into a pose.
//!
//! A walk is four passes in order, and the order is not free: [`step`] places
//! the contacts and sinks the body for them, [`swing_arms`] answers with the
//! upper body, [`super::plant_feet_of`] settles the stance feet onto whatever
//! terrain is really there, and [`roll_feet`] takes the soles back off it into
//! the heel-strike and push-off attitudes a walk is judged by — attitudes
//! measured against that same terrain, so a foot climbing a hill meets the hill
//! rather than the flat it was authored for (#250). Each reads the pose the one
//! before it left; running the roll before the plant would simply be levelled
//! away.
//!
//! The cycle itself is the caller's to drive, because how fast a body walks is a
//! question about the world — speed, terrain, intent — and this crate does not
//! know about any of that.

use glam::{Quat, Vec3};

use super::gaze::GazeConfig;
use super::ground::{Footing, FootingConfig, Ground, plant_feet_of, solve_contact};
use super::heading::Heading;
use super::pose::Pose;
use crate::plan::{Limb, Zone};
use crate::rig::Rig;

/// Where one contact is in the cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// On the ground, holding position while the body travels over it.
    /// Carries progress through the stance, in `0..=1`.
    Stance(f32),
    /// In the air, swinging forward. Carries progress through the swing.
    Swing(f32),
}

impl Phase {
    /// Whether this contact is carrying the body.
    #[must_use]
    pub fn is_stance(self) -> bool {
        matches!(self, Phase::Stance(_))
    }

    /// How far through whichever phase this is, in `0..=1`.
    #[must_use]
    pub fn progress(self) -> f32 {
        match self {
            Phase::Stance(t) | Phase::Swing(t) => t,
        }
    }
}

/// When each of a body's contacts lifts, and for how long.
#[derive(Clone, Debug, PartialEq)]
pub struct Gait {
    /// The contacts this gait drives, in the rig's own order.
    pub limbs: Vec<Limb>,
    /// Each contact's offset into the cycle, in `0..1`.
    pub offsets: Vec<f32>,
    /// Fraction of the cycle a contact spends on the ground.
    ///
    /// Above `0.5` more than half the feet are always down, which is what makes
    /// a gait a walk; below it, the body has airborne moments and is running.
    pub duty: f32,
}

impl Gait {
    /// A body standing still: every contact down, always.
    #[must_use]
    pub fn standing(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        Self {
            offsets: vec![0.0; limbs.len()],
            limbs,
            duty: 1.0,
        }
    }

    /// Contacts lifting one after another, evenly spread around the cycle.
    ///
    /// The general answer for any number of legs, and for two it is simply a
    /// walk. Duty is set so exactly one contact is airborne at a time on a body
    /// with several, which is the most stable way to move one.
    ///
    /// Two legs are the case that needs care: `1 − 1/2` is a duty of exactly a
    /// half, which means each foot leaves the instant the other lands and the
    /// body is never on both at once. Walking is defined by having that overlap;
    /// without it the result is a run performed at walking pace, and it reads as
    /// one. [`DOUBLE_SUPPORT`] is the floor that keeps it.
    #[must_use]
    pub fn wave(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        let count = limbs.len().max(1);
        Self {
            offsets: (0..limbs.len())
                .map(|index| index as f32 / count as f32)
                .collect(),
            limbs,
            duty: (1.0 - 1.0 / count as f32).max(0.5 + DOUBLE_SUPPORT),
        }
    }

    /// Diagonal pairs moving together — a horse's trot.
    ///
    /// Falls back to a wave gait on a body that is not four-legged, since
    /// "diagonal" means nothing without four corners.
    #[must_use]
    pub fn trot(rig: &Rig) -> Self {
        let limbs = rig.ground_contacts();
        if limbs.len() != 4 {
            return Self::wave(rig);
        }
        Self {
            offsets: limbs
                .iter()
                .map(|limb| {
                    // Front-left with hind-right, front-right with hind-left.
                    if limb.is_fore() == limb.is_left() {
                        0.0
                    } else {
                        0.5
                    }
                })
                .collect(),
            limbs,
            duty: 0.5,
        }
    }

    /// The same pattern, run rather than walked: a duty below a half, so the
    /// body has a moment with nothing on the ground.
    ///
    /// **A run is not a fast walk, and the difference is one number.** Every
    /// other constructor here floors a two-legged body at `0.5 + DOUBLE_SUPPORT`
    /// precisely to prevent this — a walk is defined by never being airborne —
    /// so until this existed there was no way to ask the crate for a body that
    /// leaves the ground, and a quadruped, which has no imported clips to fall
    /// back on, could move no faster than a trot (#186).
    ///
    /// The offsets are whatever [`Self::natural`] would use, which on four legs
    /// makes this a *running trot* — a trot with a suspension phase, which is a
    /// real gait — and on two an ordinary run. **It is not a canter or a
    /// gallop**: those are asymmetric, their offsets are not evenly spread, and
    /// neither this nor [`Self::wave`] can express that shape. That is a second
    /// constructor and a deferred slice of #186.
    #[must_use]
    pub fn running(rig: &Rig) -> Self {
        Self {
            duty: RUN_DUTY,
            ..Self::natural(rig)
        }
    }

    /// The gait a body of this shape moves with by default.
    #[must_use]
    pub fn natural(rig: &Rig) -> Self {
        if rig.ground_contacts().len() == 4 {
            Self::trot(rig)
        } else {
            Self::wave(rig)
        }
    }

    /// How many contacts this gait drives.
    #[must_use]
    pub fn len(&self) -> usize {
        self.limbs.len()
    }

    /// Whether the gait drives nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.limbs.is_empty()
    }

    /// Where contact `index` is at this point in the cycle.
    #[must_use]
    pub fn phase(&self, index: usize, cycle: f32) -> Phase {
        let duty = self.duty.clamp(0.0, 1.0);
        let local = (cycle - self.offsets.get(index).copied().unwrap_or(0.0)).rem_euclid(1.0);

        if duty >= 1.0 {
            return Phase::Stance(local);
        }
        if local < duty {
            Phase::Stance(if duty > 0.0 { local / duty } else { 0.0 })
        } else {
            Phase::Swing((local - duty) / (1.0 - duty))
        }
    }

    /// How many contacts are on the ground at this point in the cycle.
    ///
    /// Never reaching zero is what separates a walk from a run, and reaching it
    /// accidentally is what makes a gait look like a stumble.
    #[must_use]
    pub fn grounded(&self, cycle: f32) -> usize {
        (0..self.len())
            .filter(|&index| self.phase(index, cycle).is_stance())
            .count()
    }

    /// The moments in the cycle where a contact goes down or comes up, in
    /// order, as cycle fractions.
    ///
    /// [`Self::grounded`] is constant between consecutive entries and can only
    /// change at one, which is what makes an airborne stretch findable exactly
    /// rather than by sampling — and a stretch found by sampling is a stretch
    /// whose ends move with the sample rate.
    fn transitions(&self) -> Vec<f32> {
        let mut at: Vec<f32> = self
            .offsets
            .iter()
            .flat_map(|&offset| [offset.rem_euclid(1.0), (offset + self.duty).rem_euclid(1.0)])
            .collect();
        at.sort_by(f32::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < f32::EPSILON);
        at
    }

    /// The airborne stretch of the cycle containing `cycle`: how far through it
    /// we are, and how long it is, both as fractions of the cycle.
    ///
    /// `None` while anything is on the ground — which is the whole cycle for
    /// every gait but a run — and also for a gait that is airborne throughout,
    /// since a body that never lands is not taking a stride and has no arc to
    /// be part way along.
    #[must_use]
    pub fn flight_at(&self, cycle: f32) -> Option<(f32, f32)> {
        let cycle = cycle.rem_euclid(1.0);
        if self.duty >= 1.0 || self.grounded(cycle) > 0 {
            return None;
        }
        let at = self.transitions();
        if at.is_empty() {
            return None;
        }
        // The stretch is bounded by the transition before `cycle` and the one
        // after, both wrapping. A gait with no landing at all has neither and is
        // rejected above by way of its own transitions being uniformly airborne.
        let after = at.iter().find(|&&edge| edge > cycle).copied();
        let before = at.iter().rev().find(|&&edge| edge <= cycle).copied();
        let (start, end) = match (before, after) {
            (Some(before), Some(after)) => (before, after),
            // Wrapping round the end of the cycle: the stretch runs from the
            // last transition through zero to the first.
            (Some(before), None) => (before, at[0] + 1.0),
            (None, Some(after)) => (at[at.len() - 1] - 1.0, after),
            (None, None) => return None,
        };
        let span = end - start;
        if span <= f32::EPSILON || span >= 1.0 {
            return None;
        }
        Some((((cycle - start) / span).clamp(0.0, 1.0), span))
    }

    /// What fraction of the cycle this gait spends with nothing on the ground.
    ///
    /// **Summed exactly rather than sampled.** [`Self::grounded`] is piecewise
    /// constant and can only change where a contact goes down or comes up, so the
    /// airborne stretches are found rather than searched for — and a stretch
    /// found by sampling is one whose length moves with the sample rate, which
    /// is the kind of instrument this crate has been bitten by.
    #[must_use]
    pub fn airborne(&self) -> f32 {
        if self.duty >= 1.0 || self.is_empty() {
            return 0.0;
        }
        let at = self.transitions();
        if at.is_empty() {
            return 0.0;
        }
        at.iter()
            .zip(at.iter().cycle().skip(1))
            .map(|(&from, &to)| {
                let to = if to > from { to } else { to + 1.0 };
                // The midpoint, because an endpoint is exactly where the answer
                // is ambiguous.
                if self.grounded((from + to) * 0.5) == 0 {
                    to - from
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            .clamp(0.0, 1.0)
    }

    /// How many times the body transfers its support in one cycle.
    ///
    /// **Not how many feet land, and the difference is a factor of two on a
    /// trot.** A trot puts down diagonal PAIRS, so a four-legged body lands four
    /// feet in two events and advances two steps a cycle, where a four-legged
    /// wave lands them one at a time and advances four. Counting contacts
    /// instead would give a trotting body twice the cadence it needs at the
    /// same speed.
    ///
    /// Contacts that share an offset land together and count once. A standing
    /// gait transfers support nowhere and answers one, which is the honest
    /// answer for a body that is not going anywhere and keeps every caller
    /// dividing by this safe.
    #[must_use]
    pub fn footfalls(&self) -> usize {
        let mut at: Vec<f32> = self
            .offsets
            .iter()
            .map(|offset| offset.rem_euclid(1.0))
            .collect();
        at.sort_by(f32::total_cmp);
        at.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        at.len().max(1)
    }

    /// Where in this gait's cycle a body sits at the same point of its step as
    /// it does in `from` at `cycle`.
    ///
    /// **What a cycle number means changes when the gait does.** A duty of 0.6
    /// puts the first contact's takeoff at 0.6 and a duty of 0.35 puts it at
    /// 0.35, so carrying the number across unchanged lands the body at a
    /// different part of its step — a foot in mid-swing arrives planted, or a
    /// planted one arrives in the air. That is the discontinuity a crossfade is
    /// usually hiding, and it does not have to exist: the map between the two
    /// is exact.
    ///
    /// Matched on the **leading contact** — the one whose offset the cycle is
    /// measured from — because a single number cannot hold every contact's
    /// phase when the two gaits spread their offsets differently. Where the
    /// offsets agree, which is every pair this crate constructs for one body,
    /// matching one matches all of them.
    #[must_use]
    pub fn phase_matched(&self, from: &Gait, cycle: f32) -> f32 {
        let (Some(&mine), Some(&theirs)) = (self.offsets.first(), from.offsets.first()) else {
            return cycle.rem_euclid(1.0);
        };
        // Where the leading contact is in its own step, as a kind and a
        // progress. `phase` already answers exactly that.
        let local = match from.phase(0, cycle) {
            Phase::Stance(t) => t * self.duty.clamp(0.0, 1.0),
            Phase::Swing(t) => {
                let duty = self.duty.clamp(0.0, 1.0);
                duty + t * (1.0 - duty)
            }
        };
        let _ = theirs;
        (mine + local).rem_euclid(1.0)
    }

    /// The next moment in the cycle at which the body transfers its support,
    /// as a cycle fraction ahead of `cycle`.
    ///
    /// **The moment a transition is allowed to move the legs.** A contact that
    /// is bearing weight is pinned to the ground; starting a transition that
    /// moves it makes it slide, which reads as the body skating rather than
    /// changing what it is doing. A handoff is where support is already moving,
    /// so a change made there costs nothing.
    ///
    /// Zero when `cycle` is exactly on a handoff; never negative, and never
    /// more than a whole cycle away.
    #[must_use]
    pub fn until_handoff(&self, cycle: f32) -> f32 {
        let cycle = cycle.rem_euclid(1.0);
        // Support transfers when a contact goes DOWN. Coming up is a departure
        // and leaves the remaining feet carrying the body, which is not a
        // moment anything is free to move.
        self.offsets
            .iter()
            .map(|&offset| (offset.rem_euclid(1.0) - cycle).rem_euclid(1.0))
            .fold(1.0f32, f32::min)
    }

    /// Whether every contact this gait drives is on the ground right now.
    ///
    /// The moment a body may stop: with all of its feet down it can simply hold
    /// still, where a body frozen mid-swing has to put a foot somewhere.
    #[must_use]
    pub fn is_settled(&self, cycle: f32) -> bool {
        !self.is_empty() && self.grounded(cycle) == self.len()
    }

    /// The next moment the body could stop, as a cycle fraction ahead of
    /// `cycle`.
    ///
    /// `None` for a gait that never has all of its contacts down at once — a
    /// **run**, whose whole structure is that it does not. That is not a
    /// failure to report: a running body cannot stop where it is, it has to
    /// slow to a walk first, which along the speed axis it does by itself.
    /// Returning `None` says so rather than naming a moment that is not one.
    #[must_use]
    pub fn until_settled(&self, cycle: f32) -> Option<f32> {
        if self.is_empty() {
            return None;
        }
        let cycle = cycle.rem_euclid(1.0);
        if self.is_settled(cycle) {
            return Some(0.0);
        }
        // Every contact goes down at some transition, so a settled moment can
        // only begin at one. Checking just inside each interval is enough for
        // the same reason `airborne` samples midpoints.
        let at = self.transitions();
        at.iter()
            .filter_map(|&edge| {
                let just_inside = edge + 1e-4;
                self.is_settled(just_inside)
                    .then(|| (edge - cycle).rem_euclid(1.0))
            })
            .fold(None, |best: Option<f32>, ahead| {
                Some(best.map_or(ahead, |best| best.min(ahead)))
            })
    }

    /// Whether this gait has a moment with nothing on the ground.
    ///
    /// The one structural difference between a walk and a run, asked of the
    /// gait rather than inferred from its duty — which would be wrong the
    /// moment a body has some number of legs other than two.
    #[must_use]
    pub fn has_flight(&self) -> bool {
        self.airborne() > 0.0
    }
}

/// Fraction of a walk cycle a two-legged body spends on both feet.
///
/// The overlap that makes a walk a walk rather than a slow run.
pub const DOUBLE_SUPPORT: f32 = 0.1;

/// Fraction of the cycle a contact spends down in [`Gait::running`].
///
/// **0.35, which is the jog end of the range rather than the middle of it.**
/// The gait literature puts a runner's duty factor at about 0.35 near the
/// walk–run transition and falling toward 0.22 at a sprint, so a single
/// constant has to choose where on that range "running" means. The transition
/// end is the right choice for a constant: it is the speed a body arrives at
/// first, the flight phase it gives is real but short, and a duty picked at the
/// sprint end would make every run this crate offers a sprint.
///
/// It stops being a constant at #240, where duty comes from speed along with
/// the stride and the cadence, and this becomes that relation's value at the
/// transition.
pub const RUN_DUTY: f32 = 0.35;

/// How far a running body's leg spring compresses at midstance, as a share of
/// the reach of that leg.
///
/// **This is the only vertical constant a run needs; the flight arc follows
/// from it.** A runner's centre of mass swings through 80–100 mm over a stride
/// on a leg of 850–900 mm, which is a tenth of the leg either way you take it.
/// That tenth is the *total* excursion, trough to crest, and it is shared
/// between the compression here and the ballistic rise above it — see
/// [`flight_rise`] for the relation, which is velocity continuity at takeoff
/// and not a second tuned number. At the default duty that puts the
/// compression at about three quarters of the total.
///
/// Zero for any gait that never leaves the ground: a walk's vertical is the
/// inverted pendulum [`crouch_at`] already derives from reach geometry, and a
/// spring term added to it would be a second answer to a question that already
/// has one.
pub const RUN_COMPRESSION: f32 = 0.075;

/// How much further than strictly necessary a body sinks to take its stride.
///
/// The margin keeps a visible bend in the knee and the solver clear of the
/// singularity at full extension. It scales with the sinking rather than being
/// added to it, so a body that is standing still does not crouch at all.
pub const CROUCH_MARGIN: f32 = 1.15;

/// How far and how high a body steps, and how far its heading turns while it
/// does.
///
/// **Everything here is per stance**, which is the span [`contact_offset`]
/// parameterises: `length` is the ground a contact covers between going down
/// and coming up, and `yaw` is the body's own turn across the same span. The
/// two together are a screw motion in the ground plane, and that is the whole
/// of what a body travelling does to a foot that is standing on the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stride {
    /// Direction of travel in body space. `+Z` is forward.
    ///
    /// **Live as of #241, and it had never been anything but `+Z` before
    /// that.** Every construction site in every repo set it to forward and
    /// nothing read it except as a multiplier on a scalar slide, so the lateral
    /// path was code that compiled and had never run. It is now the heading the
    /// whole screw below is built around, which is the seam #242's reverse and
    /// strafe want: backwards is this direction negated and a strafe is it
    /// turned a quarter.
    pub direction: Vec3,
    /// Ground covered per cycle, in metres.
    pub length: f32,
    /// How high a contact lifts at the top of its swing, in metres.
    pub lift: f32,
    /// How far the body's own heading turns across one stance, in radians.
    ///
    /// Positive toward `+X`, the body's left — which is also the direction a
    /// positive rotation about `+Y` carries `+Z`, so this is the sign of the
    /// yaw itself rather than a convention laid over it.
    ///
    /// **The partner of `length`, in the same span and to the same purpose.**
    /// A body walking a curve is turning while it travels, and the two
    /// together say where a planted foot has to sit at every moment of the
    /// stance for it not to slide. Zero is a straight line, and at zero every
    /// expression below collapses to exactly the scalar slide this had before.
    ///
    /// The pair also fixes the path: `yaw / length` is the curvature of the
    /// arc the body walks, in reciprocal metres, and its reciprocal is the turn
    /// radius. That ratio is what [`super::Turn`] hands back and what the bank
    /// is derived from.
    pub yaw: f32,
}

/// What share of a leg's reach a natural stride covers.
///
/// Named rather than written into [`Stride::for_body`], because [`pace_of`]
/// reads the relation backwards to recover how fast a stride is, and a
/// constant that appears twice is one that will disagree with itself — the
/// slope plane in the viewer drifted apart twice for exactly that reason.
const STRIDE_OF_REACH: f32 = 0.7;

/// And what share of it a contact lifts at the top of its swing.
const LIFT_OF_REACH: f32 = 0.12;

impl Stride {
    /// A stride scaled to the body walking it.
    ///
    /// Scaled by how far the legs reach, not by how tall the body is: the same
    /// height can be mostly leg or mostly torso, and it is the leg that takes the
    /// step. Expressing it this way is what lets one stride description suit a
    /// child and a giant.
    #[must_use]
    pub fn for_body(rig: &Rig, pace: f32) -> Self {
        let reach = rig
            .ground_contacts()
            .into_iter()
            .filter_map(|limb| rig.limb_reach(limb))
            .fold(0.0f32, f32::max)
            .max(f32::EPSILON);
        Self {
            direction: Vec3::Z,
            length: reach * STRIDE_OF_REACH * pace.max(0.0),
            lift: reach * LIFT_OF_REACH * pace.max(0.0),
            yaw: 0.0,
        }
    }

    /// The same stride, taken in another direction (#242).
    ///
    /// **Both the heading and the length**, which is why this is one call
    /// rather than a field to assign. A body does not step as far backwards as
    /// forwards or as far sideways as either — see [`Heading::reach`] — so a
    /// caller that set `direction` alone would get a body reaching further than
    /// its hips allow, in the direction they allow least. Making the two
    /// inseparable is what stops that being expressible.
    ///
    /// The lift comes with it, in the same proportion, so a shorter step lifts
    /// its foot less: a toe clearance is a share of the ground being covered
    /// and a sideways shuffle covers little.
    ///
    /// **The rig is here for the second bound**, which is geometric and which
    /// only a body can answer: a sideways stride long enough drives one foot
    /// past the other. See `shuffle_limit` — and see `Heading`'s own docs for
    /// how that was found, which was by a guard refuting the claim it had been
    /// written to confirm.
    #[must_use]
    pub fn toward(self, rig: &Rig, heading: Heading) -> Self {
        // The sideways semi-axis is the smaller of what the hip allows and what
        // the stance does — folded into the ellipse rather than clipped over
        // it, so a diagonal stays interpolated. See `Heading::reach_within`.
        let lateral = if self.length > f32::EPSILON {
            crate::anim::heading::LATERAL_REACH.min(shuffle_limit(rig) / self.length)
        } else {
            crate::anim::heading::LATERAL_REACH
        };
        let scale = heading.reach_within(lateral);
        Self {
            direction: heading.direction(),
            length: self.length * scale,
            lift: self.lift * scale,
            ..self
        }
    }

    /// Standing still.
    #[must_use]
    pub fn still() -> Self {
        Self {
            direction: Vec3::Z,
            length: 0.0,
            lift: 0.0,
            yaw: 0.0,
        }
    }
}

/// The longest stride this body can take toward `heading` without putting one
/// foot through the other, in metres.
///
/// **A sideways step is bounded by the stance, not only by the hip.** Two feet
/// at opposite points of the cycle differ in their offsets by most of a
/// stride's length — one is sliding back through its stance while the other
/// swings forward — so a lateral stride wider than the feet stand apart crosses
/// them. Measured before this existed: strafing left on the default body put
/// the left foot 72 mm to the RIGHT of the right one, which is the
/// self-intersection a shuffle is chosen over a crossover to avoid.
///
/// Returned as a **distance**, which the caller turns into a share of its own
/// forward stride and hands to `Heading::reach_within` as the ellipse's
/// sideways semi-axis. A purely fore-and-aft heading never reaches it, because
/// two feet side by side have nothing to cross when they both travel down the
/// body's length — which is why every forward and backward reading in this
/// crate is untouched.
///
/// The margin is [`SHUFFLE_CLEARANCE`]. The bound scales with the body, because
/// the stance does.
fn shuffle_limit(rig: &Rig) -> f32 {
    let sides: Vec<f32> = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.in_zone(Zone::Extremity(limb)).first().copied())
        .map(|joint| rig.joints[joint].position.x)
        .collect();
    let low = sides.iter().copied().fold(f32::MAX, f32::min);
    let high = sides.iter().copied().fold(f32::MIN, f32::max);
    if sides.len() < 2 || high <= low {
        return f32::INFINITY;
    }
    (high - low) * (1.0 - SHUFFLE_CLEARANCE)
}

/// How much of its standing separation a shuffling body keeps between its feet.
///
/// **Half.** Nought would let the two soles arrive at the same point, which is
/// not a crossover but is not a step either; one would forbid a sideways stride
/// altogether. Half leaves the feet a stance-width apart at their closest,
/// which on the default body is 88 mm — about a foot's width, which is the
/// clearance that matters and which this crate cannot ask for directly because
/// a foot's width is a mesh question and this is a rig one.
const SHUFFLE_CLEARANCE: f32 = 0.5;

/// How far one limb must sink for its chain to reach a goal `toward` its rest
/// contact, before any margin.
///
/// Measured to the joint the chain actually reaches, not to the contact hanging
/// off it — the same distinction the solve makes. Solved against the limb's
/// *actual* rest geometry rather than from stride length alone, because a
/// limb's contact is not generally beneath its hip: a quadruped's feet already
/// sit well forward of the joints that carry them, and assuming otherwise
/// under-crouches every four-legged body.
///
/// **Sized with the hang the rest pose has, which is the only one available**
/// — the crouch has to be chosen before the solve that would measure a better
/// one. Under #254 that looked like the cause of a strained leg on flat ground
/// and was not: the goal there is inside the limb's reach by two tenths of a
/// millimetre, and what refuses it is the solver holding back from the
/// singularity a rest-pose leg permanently stands at. Sinking further would
/// have been treating a reporting threshold as a geometry problem. See
/// [`super::ground::CONTACT_STRAIN`].
fn sink_needed(rig: &Rig, limb: Limb, toward: Vec3) -> Option<f32> {
    let reach = rig.limb_reach(limb)?;
    let chain = rig.limb_chain(limb)?;
    let offset = rig.joints[chain[2]].position - rig.joints[chain[0]].position;
    let reaching = offset + toward;
    let horizontal = reaching.length_squared() - reaching.y * reaching.y;
    // Sinking by `c` shortens the hip-to-goal distance by moving the hip down
    // toward the goal's height.
    let needed = -reaching.y - (reach * reach - horizontal).max(0.0).sqrt();
    Some(needed.max(0.0))
}

/// The deepest a body sinks anywhere in its stride — the envelope of
/// [`crouch_at`].
///
/// A leg standing straight has no slack: swinging its foot forward by half a
/// stride puts the goal further from the hip than the leg is long. Bodies solve
/// this by sinking as they stride, and so does this — without it every step is
/// out of reach and the legs merely stretch toward the ground.
///
/// This is the figure to plan around — camera heights, clearances — but not the
/// height to *hold*: a walk pinned at its own worst case rides flat, and that
/// flatness is exactly the pelvis bob the walk used to lack.
///
/// **An upper bound on a run rather than its exact worst moment.** A run adds a
/// spring compression that is deepest at midstance, where the reach term is at
/// its shallowest, so summing the two maxima names a depth the body never
/// quite reaches. That is the right way for a planning figure to be wrong.
#[must_use]
pub fn crouch_for(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let reaching = gait
        .limbs
        .iter()
        .filter_map(|&limb| {
            // **Per limb, since #241.** The two extremes of a stance are the
            // ends of that contact's own arc, and on a turn the outside foot's
            // is longer than the inside foot's — so the envelope is the deepest
            // any one of them asks for, not one figure taken for all. A
            // contact with no home is one `step` would skip, and skipping it
            // here is what keeps the planning figure and the frame agreeing.
            let home = home_of(rig, limb)?;
            // Both ends of the stride, since a contact may start forward of its
            // hip and only the further extreme matters.
            [0.5f32, -0.5]
                .into_iter()
                .filter_map(|share| sink_needed(rig, limb, carried(home, stride, share)))
                .fold(f32::NEG_INFINITY, f32::max)
                .into()
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN;

    let squash = if gait.has_flight() {
        gait.limbs
            .iter()
            .filter_map(|&limb| rig.limb_reach(limb))
            .fold(0.0f32, f32::max)
            * RUN_COMPRESSION
            * pace_of(rig, stride)
    } else {
        0.0
    };
    reaching + squash
}

/// How far the body sinks at this point of the cycle — which is the pelvis bob.
///
/// Each limb asks for the sink its *current* goal needs — a foot at the far end
/// of its stride pulls the body down, one passing under its hip lets it rise,
/// and a swinging foot asks for less because its goal is lifted off the ground.
/// The body sinks by the deepest request. On a walking biped that request peaks
/// at every heel-strike and toe-off, where the legs are split at full stride
/// with both ends on the ground, and bottoms out as the stance foot passes
/// under the hip — so the pelvis vaults twice a cycle, highest at each
/// midstance, exactly the inverted pendulum a real walk rides. Nothing here is
/// tuned: the bob's depth, its timing and its pace-scaling all fall out of the
/// same reach geometry the stride was already solved against, and a standing
/// body still sinks exactly zero.
#[must_use]
///
/// **Level ground.** [`step`] seats its own contacts on whatever terrain its
/// caller offers and re-derives the sink from the seated goals (#221), so on a
/// slope the body actually sinks by more or less than this. That is deliberate
/// rather than a divergence to fix: this is the planning figure — what a body
/// of these proportions does on the flat — and the number a camera height or a
/// clearance is designed against.
pub fn crouch_at(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> f32 {
    // The same rule [`step`] applies to its targets (#230): a gait that never
    // lifts a contact expresses no stride, so a standing body asks for no sink
    // whatever pace the caller left on the stride.
    if gait.duty >= 1.0 {
        return 0.0;
    }
    let reaching = gait
        .limbs
        .iter()
        .enumerate()
        .filter_map(|(index, &limb)| {
            // Per limb since #241 — see `crouch_for`. A contact's goal is a
            // function of where that contact rests, so on a turn the two legs
            // ask for different sinks and the body owes the deeper one.
            let home = home_of(rig, limb)?;
            sink_needed(
                rig,
                limb,
                contact_offset(home, stride, gait.phase(index, cycle)),
            )
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN;
    reaching + compression_at(rig, gait, stride, cycle)
}

/// How far a running body's leg spring is squashed at this point of the cycle.
///
/// **A run's vertical is not the walk's, and it is not a tuning of it.** A walk
/// vaults over a straight leg, so it is *highest* at midstance and lowest where
/// the legs are split — which is what [`crouch_at`]'s reach geometry delivers
/// for free. A run does the opposite: the leg is a spring that takes the
/// landing, so the body is *lowest* at midstance and rises from there. Applying
/// the walk's rule to a run gives a body that bobs the wrong way twice a step,
/// which is what a duty below a half produced before this existed (#186).
///
/// A half sine over the stance, which is the shape a spring compressing and
/// returning has: zero at touchdown, [`RUN_COMPRESSION`] of the leg's reach at
/// midstance, zero again at takeoff, so it joins the flight arc without a step.
/// Scaled by pace, because a body landing harder squashes further and a body
/// barely running barely does.
///
/// Zero for any gait with no flight phase, and zero for every contact that is
/// not down — the spring is the leg that is carrying the body.
#[must_use]
pub fn compression_at(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> f32 {
    if !gait.has_flight() {
        return 0.0;
    }
    let reach = gait
        .limbs
        .iter()
        .filter_map(|&limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    let deepest = (0..gait.len())
        .filter_map(|index| match gait.phase(index, cycle) {
            Phase::Stance(t) => Some((t * std::f32::consts::PI).sin()),
            Phase::Swing(_) => None,
        })
        .fold(0.0f32, f32::max);
    reach * RUN_COMPRESSION * pace_of(rig, stride) * deepest
}

/// How far above its stance height a running body is carried while airborne.
///
/// **Ballistic, and that is a shape rather than a choice**: a body with nothing
/// on the ground is a projectile under constant gravity, so its height through
/// the flight is a parabola — zero at takeoff, zero at touchdown, an apex in
/// between.
///
/// **The apex is derived, not tuned.** The body leaves the ground with whatever
/// vertical velocity the spring gave it, so the arc above and the compression
/// below are two halves of one motion and cannot be given independent
/// amplitudes without the body kinking at takeoff. Matching the vertical speed
/// on both sides of that instant fixes the relation: the compression leaves
/// stance at `π·C` per unit stance fraction and the parabola enters flight at
/// `4·A` per unit flight fraction, so
///
/// ```text
/// A = (π/4) · C · (flight fraction / stance fraction)
/// ```
///
/// which is why [`RUN_COMPRESSION`] is the only number here. A duty that falls
/// — a faster run — lengthens the flight and shortens the stance, and the apex
/// grows by exactly that ratio without anything being retuned.
///
/// Applied as a **rigid** lift of the whole body, feet included, because a
/// projectile does not change shape. [`step`] adds it after the legs are
/// solved, which is what makes it rigid; adding it before would have the legs
/// reach back down for goals that stayed on the ground.
#[must_use]
pub fn flight_rise(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> f32 {
    let Some((progress, _)) = gait.flight_at(cycle) else {
        return 0.0;
    };
    // **The ratio of the whole cycle's airborne share to its supported share**,
    // not of this one stretch to the cycle around it: the arithmetic wants one
    // flight against the one stance beside it, and on a gait whose stretches
    // are all alike — which every gait this constructs is — those two ratios
    // are the same number. Taking it from the totals also gives a sane answer
    // on a gait whose stretches are not alike, where "the stance beside it"
    // stops naming one thing.
    let airborne = gait.airborne();
    let grounded = 1.0 - airborne;
    if grounded <= f32::EPSILON || airborne <= f32::EPSILON {
        return 0.0;
    }
    let reach = gait
        .limbs
        .iter()
        .filter_map(|&limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    let compression = reach * RUN_COMPRESSION * pace_of(rig, stride);
    let apex = std::f32::consts::FRAC_PI_4 * compression * (airborne / grounded);
    apex * 4.0 * progress * (1.0 - progress)
}

/// Where a contact belongs, relative to where it rests.
///
/// During stance the foot holds still in the world while the body travels over
/// it, which in body space is a slide backwards. During swing it retraces that
/// path forwards, lifted, to meet the ground at the front of the step.
///
/// # Why this takes the contact's home, and what that bought (#241)
///
/// **A planted foot holds still in the world, and that one sentence is the
/// whole of the geometry.** Its body-space position is therefore the body's own
/// motion, inverted, applied at the point where the foot is standing — and a
/// point is where the limb comes in. On a straight line every contact gets the
/// same answer, because a translation moves every point of the ground alike,
/// and that is why this was a scalar slide for as long as the only motion was a
/// straight line.
///
/// The moment the body turns it stops being alike. A body walking a curve is
/// rotating as well as translating, and a rotation moves a point on the inside
/// of the arc less than one on the outside. So the **differential stride falls
/// out** rather than being applied: the inside foot sweeps a shorter arc
/// because it is nearer the centre, by exactly the ratio of the two radii, and
/// nothing here knows which foot is which or that a turn has an inside at all.
/// The alternative — scaling `length` per limb by a factor derived from the
/// radius — gets the same lengths and the wrong paths, because a foot on an arc
/// does not travel in a straight line, and it needs a constant nobody can check
/// where this needs none.
///
/// The same expression covers the cases that would otherwise each be a branch:
/// a straight walk is `yaw = 0`, a **pivot in place** is `length = 0` with the
/// feet counter-rotating about the body's centre, and #242's reverse and strafe
/// are the sign and the axis of [`Stride::direction`].
///
/// # Why the swing retraces the stance
///
/// The foot must arrive where the *next* stance begins, which is where the last
/// one started. Running the same path backwards is the shortest description of
/// that and it is also, exactly, what the straight-line version did: a slide
/// from `+half` to `−half` and a swing from `−half` to `+half` are one segment
/// walked twice. The lift is added on top, unchanged.
#[must_use]
pub fn contact_offset(home: Vec3, stride: &Stride, phase: Phase) -> Vec3 {
    match phase {
        // The stance is parameterised either side of the moment the foot was
        // planted, so `home` is where the contact sits at MIDSTANCE — which is
        // what the old form said too, with its `half` at each end.
        Phase::Stance(t) => carried(home, stride, t - 0.5),
        Phase::Swing(t) => {
            carried(home, stride, 0.5 - t)
                + Vec3::Y * (stride.lift * (t * std::f32::consts::PI).sin())
        }
    }
}

/// Which way a contact points, relative to the heading it rests at.
///
/// **The rotational half of [`contact_offset`], and it is needed for the same
/// reason.** A planted foot holds still in the world, and holding still means
/// its *bearing* as much as its position: a body turning over a foot that is on
/// the floor leaves that foot pointing where it was put. Without this the
/// contact stops sliding and carries on spinning — measured, a foot was dragged
/// through 29.4 of the 29.5 degrees a stance asked for, which at a 200 mm foot
/// is a bigger skid than the translation ever was.
///
/// Zero on a straight walk, at every phase, which is why nothing needed it
/// until #241.
///
/// # Where this is applied, and why not earlier
///
/// [`roll_feet`], which is the last stage to touch an ankle. It cannot be
/// applied when the contact is *placed* — [`super::level_feet`] assigns the
/// ankle `from_rotation_arc(Y, up)`, a pure tilt with no bearing in it at all,
/// so any heading authored before the plant is wiped by it. That assignment is
/// invisible on a straight walk, where the bearing wanted is zero and the one
/// destroyed was zero too, and it is the reason this reads as a property of the
/// ankle's ground attitude rather than of the leg's swing.
#[must_use]
pub fn contact_heading(stride: &Stride, phase: Phase) -> f32 {
    let share = match phase {
        Phase::Stance(t) => t - 0.5,
        Phase::Swing(t) => 0.5 - t,
    };
    -stride.yaw * share
}

/// Where a contact planted at `home` sits in body space, `share` of a stance
/// either side of the moment it was planted.
///
/// The body's motion over that span is a screw in the ground plane — travel
/// `length · share` along [`Stride::direction`] and a heading turn of
/// `yaw · share` — and this is that motion inverted and applied at `home`.
///
/// **Written in the `sin a / a` form**, which is the arc's own shape function
/// and stays finite as the curvature goes to zero. The centre-and-radius form
/// is the obvious one and it is unusable here: the centre runs off to infinity
/// on a straight line, which is the case that has to keep working exactly.
fn carried(home: Vec3, stride: &Stride, share: f32) -> Vec3 {
    let angle = stride.yaw * share;
    let forward = stride.direction.normalize_or_zero();
    if forward == Vec3::ZERO {
        return Vec3::ZERO;
    }
    // The turn centre sits to this side. `+Y × +Z` is `+X`, the body's left,
    // and a positive rotation about `+Y` carries `+Z` toward `+X` — so a
    // positive yaw curves to the left, and the two agree by construction rather
    // than by a sign that has to be remembered.
    let across = Vec3::Y.cross(forward);
    let (along_shape, across_shape) = if angle.abs() < 1e-4 {
        // The leading terms of both series. Below this the difference is under
        // a float's resolution on any distance a body walks, and above it the
        // closed forms are well conditioned.
        (1.0 - angle * angle / 6.0, angle * 0.5)
    } else {
        (angle.sin() / angle, (1.0 - angle.cos()) / angle)
    };
    let travelled = stride.length * share;
    let body = forward * (travelled * along_shape) + across * (travelled * across_shape);
    // Where the body has got to, undone: the plant point seen from the frame
    // the body occupies `share` of a stance later.
    Quat::from_rotation_y(-angle) * (home - body) - home
}

/// What one step of a gait did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Steps {
    /// Contacts currently carrying the body.
    pub stance: Vec<Limb>,
    /// Contacts currently in the air.
    pub swing: Vec<Limb>,
    /// Contacts whose goal was out of reach.
    pub straining: Vec<Limb>,
    /// How far the body sank to keep its stride within reach, in metres.
    pub crouch: f32,
    /// How far the body was carried above its stance height while airborne, in
    /// metres. Zero for anything but a run, and zero at both ends of a flight.
    pub rise: f32,
}

impl Steps {
    /// Whether every contact reached where the gait wanted it.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.straining.is_empty()
    }
}

/// Poses a body's legs for one moment of a gait.
///
/// `cycle` runs `0..1` and wraps; the caller advances it, because how fast a
/// body walks depends on the world rather than on the body.
///
/// The result reports which contacts are down, which a caller passes to
/// [`super::plant_feet_of`] to settle them onto real terrain — a swinging foot
/// must not be dragged to the ground it is travelling over.
pub fn step<F>(
    rig: &Rig,
    pose: &mut Pose,
    gait: &Gait,
    stride: &Stride,
    cycle: f32,
    ground: F,
) -> Steps
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let mut steps = Steps::default();
    if !pose.fits(rig) {
        return steps;
    }

    // Every contact's goal for this instant, seated on the ground beneath it,
    // computed once because the crouch and the solve must agree about where the
    // feet are going. Deriving the sink from level offsets and then solving
    // against seated ones would sink the body for a stride it is not taking.
    let goals: Vec<(Limb, Phase, Vec3, Vec3)> = gait
        .limbs
        .iter()
        .enumerate()
        .filter_map(|(index, &limb)| {
            let home = home_of(rig, limb)?;
            let phase = gait.phase(index, cycle);
            // A gait that never lifts a contact has no stride to express: at
            // duty 1.0 every phase is a stance whose offset would slide the
            // whole stride and WRAP — every foot teleport-hopping in lockstep
            // once per cycle whenever the caller's pace is nonzero (#230).
            // Standing still is standing still whatever the stride says, so the
            // caller need not remember to zero it.
            let offset = if gait.duty >= 1.0 {
                Vec3::ZERO
            } else {
                seated_offset(home, stride, phase, &ground)
            };
            Some((limb, phase, offset, home + offset))
        })
        .collect();

    // Sink far enough that every foot's current goal is within its leg's reach
    // — no further. Holding the whole stride's worst case instead is what kept
    // the pelvis riding dead flat, 47 mm down, through the entire cycle.
    steps.crouch = goals
        .iter()
        .filter_map(|&(limb, _, offset, _)| sink_needed(rig, limb, offset))
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN;
    // A running body's leg is a spring as well as a strut. Added rather than
    // taken as the deeper of the two, because the leg really is both at once —
    // a hip sits at `(L − squash)·cos θ`, so the two sinks compose. Taking the
    // maximum instead leaves a step where the curves cross: measured on the
    // default body at pace 1.0 the pelvis rose 18 mm just after touchdown and
    // then fell 25, a visible hitch on a body that should be descending
    // smoothly into its midstance. Zero for a gait that never leaves the
    // ground, which leaves every walk exactly as it was.
    steps.crouch += compression_at(rig, gait, stride, cycle);
    pose.translation.y -= steps.crouch;

    for &(limb, phase, _, _) in &goals {
        if phase.is_stance() {
            steps.stance.push(limb);
        } else {
            steps.swing.push(limb);
        }
    }

    for &(limb, _, _, target) in &goals {
        if !solve_contact(rig, pose, limb, target) {
            steps.straining.push(limb);
        }
    }

    // **Last, and rigidly.** A body with nothing on the ground is a projectile:
    // it does not change shape, so the whole of it goes up together, feet
    // included. Applied before the legs were solved this would instead be the
    // legs reaching back down for goals that had stayed on the floor.
    steps.rise = flight_rise(rig, gait, stride, cycle);
    pose.translation.y += steps.rise;

    steps
}

/// One frame of a procedural walk, start to finish.
///
/// # Why this exists
///
/// Driving this gait correctly means four stages in a fixed order, with a
/// footing solve in the middle of them, and **the record is that callers do not
/// manage it**. [`roll_feet`] was missing from three of five consumers for as
/// long as it existed, so every body they drew walked on rigid ankles — no
/// heel-strike, no toe-off, the foot tilting bodily with the shin (#1069, #251,
/// and `examples/locomotion` under #238). [`step`]'s ground closure was missing
/// from all of them until #221 put it in the signature, which is the one that
/// got fixed everywhere, precisely because the compiler asked. Then #239 added
/// a fourth stage to remember.
///
/// A doc comment naming the order was not enough three times over. This is the
/// order, executable.
///
/// # The order, and why each step sits where it does
///
/// 1. [`step`] places the contacts and sinks the body to reach them.
/// 2. [`swing_arms`] swings the arms against the legs and winds the spine.
/// 3. [`lean`] pitches the trunk into the walk and holds the head level.
/// 4. [`plant_feet_of`] settles the stance contacts onto the real ground.
/// 5. [`roll_feet`] rolls the ankles — **after** the plant, because the plant
///    lays every sole flat and a roll applied before it is simply levelled
///    away.
///
/// The same ground is given to the stride, to the plant and to the roll.
/// Handing any two of them different floors is what leaves a swing arc at the
/// rest height while the feet settle onto a hill, which is the whole of #221;
/// the roll joined that list under #250, where a sole pitched for level ground
/// went through the hill it was climbing.
///
/// # Ablation
///
/// The individual stages stay public, and this is not a replacement for calling
/// them — an instrument that wants to see what the legs alone are doing turns
/// [`Self::posture`] off, and one asking what the gait delivers before any
/// correction turns [`Self::footing`] off. What the entry point buys is that
/// *not choosing* gives the whole sequence rather than an accidental subset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Walk {
    /// Where in the cycle this frame is, `0..1`, wrapping.
    pub cycle: f32,
    /// Whether the postural layer runs: the arms swinging against the legs and
    /// the trunk leaning into the walk.
    ///
    /// One flag for both because they answer one question — what the body above
    /// the legs is doing — and a body with neither is the mannequin #102
    /// described.
    pub posture: bool,
    /// How to aim the head down the path the body is walking, or `None` to
    /// leave the gaze alone.
    ///
    /// **`None` by default**, including from [`Self::at`], and that is a
    /// deliberate asymmetry with [`Self::footing`]. Where a body looks is very
    /// often something its caller is already deciding — at another avatar, at
    /// what it is carrying, at the camera — and a walk that quietly takes the
    /// gaze over would be overwriting an intention rather than filling a gap.
    /// A caller with nothing else in mind turns it on; one that is aiming the
    /// head itself composes [`super::look_at`] on top and this stays out of the
    /// way.
    ///
    /// On a straight walk it is a target directly ahead, which is where the
    /// head already points, so switching it on costs a body walking in a line
    /// nothing.
    pub gaze: Option<GazeConfig>,
    /// How to settle the stance contacts onto the ground, or `None` to leave
    /// the gait's own placement untouched.
    ///
    /// `None` is what a caller wants when it is measuring the gait rather than
    /// drawing it: `examples/locomotion` reads the pose before and after this
    /// to say how much work the solve is doing.
    pub footing: Option<FootingConfig>,
}

impl Walk {
    /// A full walk at this point of the cycle: posture on, feet settled with
    /// the default footing.
    #[must_use]
    pub fn at(cycle: f32) -> Self {
        Self {
            cycle,
            posture: true,
            gaze: None,
            footing: Some(FootingConfig::default()),
        }
    }

    /// How far ahead down its own path a walk looks, in cycles.
    ///
    /// **One cycle, which is one full step per leg**, and it is a horizon
    /// rather than an angle — see [`path_ahead`]. Drivers and walkers alike are
    /// reported to fixate the path roughly a step or two ahead, and the near
    /// end of that band is the conservative choice here for the same reason
    /// [`RUN_DUTY`] took the transition end of its own: it is the reading a body
    /// arrives at first, and a longer horizon on a tight turn asks the neck for
    /// more than it has.
    pub const GAZE_LEAD: f32 = 1.0;

    /// Runs every stage, in order, and reports what the frame did.
    ///
    /// `ground` answers what is beneath a point in the same frame the body is
    /// posed in, exactly as [`plant_feet_of`] requires; it is given to the
    /// stride and to the plant so the two cannot disagree about the floor.
    pub fn drive<F>(
        &self,
        rig: &Rig,
        pose: &mut Pose,
        gait: &Gait,
        stride: &Stride,
        ground: F,
    ) -> Walked
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let steps = step(rig, pose, gait, stride, self.cycle, &ground);
        if self.posture {
            swing_arms(rig, pose, gait, stride, self.cycle);
            lean(rig, pose, gait, stride);
        }
        // After the lean, which the neck has just taken back off to hold the
        // head level — a gaze applied first would be levelled away, the same
        // ordering trap the roll hit against the plant.
        if let Some(config) = self.gaze {
            super::look_at(
                rig,
                pose,
                path_ahead(rig, gait, stride, Self::GAZE_LEAD),
                &config,
            );
        }

        let mut walked = self.settle(rig, pose, gait, stride, &steps.stance, ground);
        walked.steps = steps;
        walked
    }

    /// The tail of the sequence: settle the contacts, then roll the ankles.
    ///
    /// **Separated because one real consumer has to put something between the
    /// two halves.** The viewer can layer an imported clip over the procedural
    /// walk, and a clip moves the legs — so its contacts must be settled and
    /// its ankles rolled *after* that, not before. Hand-rolling the tail is
    /// exactly the fragility this type exists to remove, so the tail is a step
    /// of its own rather than two calls a caller is trusted to order.
    ///
    /// Callers that have nothing to interleave should use [`Self::drive`],
    /// which is this with the head of the sequence in front of it.
    pub fn settle<F>(
        &self,
        rig: &Rig,
        pose: &mut Pose,
        gait: &Gait,
        stride: &Stride,
        stance: &[Limb],
        ground: F,
    ) -> Walked
    where
        F: Fn(Vec3) -> Option<Ground>,
    {
        let mut walked = Walked::default();
        if let Some(config) = self.footing
            && !stance.is_empty()
        {
            let before = pose.forward(rig).positions;
            walked.footing = Some(plant_feet_of(rig, pose, stance, &ground, &config));
            let after = pose.forward(rig).positions;
            // **Per joint, each against itself.** Taking the lowest joint of a
            // foot before and after and measuring between them compares a heel
            // against a toe the moment an ankle turns, and reported a quarter
            // of a metre of correction on flat ground where there was four
            // millimetres of it — an argmin whose identity moves is not a
            // measurement. `examples/locomotion` learned that separately and
            // this is the same reading, defined once.
            for &limb in stance {
                for &joint in &rig.extremity_joints(limb) {
                    walked.lift = walked.lift.max(before[joint].distance(after[joint]));
                }
            }
        }
        // After the plant. See the type's own docs for why the order is not a
        // preference — and see [`contact_heading`] for why the stride has to
        // come this far down the sequence rather than stopping at [`step`].
        roll_feet(rig, pose, gait, stride, self.cycle, &ground);
        walked
    }
}

/// What one frame of [`Walk::drive`] did.
#[derive(Clone, Debug, Default)]
pub struct Walked {
    /// Which contacts are down, which are swinging, and which strained.
    pub steps: Steps,
    /// What the footing solve reported, or `None` if it did not run.
    pub footing: Option<Footing>,
    /// How far the footing solve had to move any one joint of a contact, in
    /// metres.
    ///
    /// The readout the locomotion question is settled on rather than on taste:
    /// a pose whose feet already land where the ground is needs no correction,
    /// and one whose do not is being held together by the solve. Zero when the
    /// solve did not run.
    pub lift: f32,
}

impl Walked {
    /// How many contacts the footing solve could not fully satisfy.
    ///
    /// Zero when the solve did not run, which is the same answer a caller wants
    /// for "nothing is straining" and saves every readout unwrapping the
    /// [`Footing`] itself.
    #[must_use]
    pub fn straining(&self) -> usize {
        self.footing
            .as_ref()
            .map_or(0, |footing| footing.straining.len())
    }
}

/// Lift a contact's offset onto the ground it is travelling over, and over the
/// ground it is travelling toward.
///
/// **The whole of #221, and then the whole of #245.** [`contact_offset`]
/// describes a stride against the body's own rest ground plane: a stance foot
/// slides backwards at that height and a swing foot arcs above it, and both end
/// the step exactly where they began it vertically. On a slope that is wrong at
/// both ends — uphill the arc drove the sole 38.9 mm into the surface it was
/// travelling over, downhill it landed in the air and dropped the last
/// centimetres at the plant.
///
/// # A stance is seated where it stands
///
/// The probe is taken **under the goal at this instant** rather than blended
/// between the step's two endpoints. The endpoint blend was the first design
/// and it is the weaker one: it holds only for ground that is flat between the
/// footfalls, so a foot still ploughs through anything it passes over on the
/// way. Sampling where the foot actually is clears whatever is under it.
///
/// **The surface's own height, and nothing subtracted from it** (#255). This
/// took the RISE between two probes — the surface under the goal minus the
/// surface under the foot's rest position — which is right for every axis the
/// stride runs ALONG and silently wrong for the one it runs across. A camber
/// moves the two contacts apart in height without moving either along its own
/// stride, so the two probes return the same rise and the correction is zero:
/// measured, a swinging foot went 52.2 mm through a 30 percent side-slope,
/// which is the stance width times the camber.
///
/// # A swing is seated where it will LAND (#245)
///
/// **A smooth grade is the easy case, and seating a swing where it stands is
/// only right while the ground is continuous.** A step is not. Across a riser
/// the ground beneath the foot jumps, so a goal seated on it jumps too, and the
/// foot arrives at the top of the step in a single frame — measured on a 100 mm
/// staircase, the sole passed 98.0 mm through the riser and the goal moved
/// 100.4 mm between two samples where a walking foot moves 10.
///
/// A penetration reading alone cannot see that, which is why
/// `examples/walkaudit` grew a jump reading for it: the foot is above the
/// surface at every sample, it simply got there impossibly fast.
///
/// So a swing is built from three probes instead of one:
///
/// * **where it takes off**, which is the ground the last stance left;
/// * **where it will land**, which is the ground under the goal at the front of
///   the step — the probe this issue is named for, and the one that makes the
///   vertical continuous at both ends because the arc now *ends* at the height
///   it is going to;
/// * **the highest ground in between**, which sets how far the arc has to rise
///   to clear whatever it is stepping over.
///
/// The middle probes are taken at the resolution of the body's own foot, on the
/// reasoning that a foot cannot fall into a gap narrower than itself — see
/// [`clearance_probes`]. That is a real limit and it is stated rather than
/// hidden: ground that rises and falls again inside one foot length can be
/// missed.
///
fn seated_offset<F>(home: Vec3, stride: &Stride, phase: Phase, ground: &F) -> Vec3
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let offset = contact_offset(home, stride, phase);
    let beneath = |at: Vec3| ground(home + at).map_or(0.0, |there| there.position.y);

    let Phase::Swing(t) = phase else {
        // A stance is seated where it stands. It is on the ground; there is
        // nothing to anticipate.
        return offset + Vec3::Y * beneath(offset);
    };

    // The two ends of the swing, which are the last stance's departure and the
    // next one's arrival — asked of `contact_offset` rather than assumed, so
    // they follow the arc a turning body walks (#241) as readily as a straight
    // one.
    let leaving = contact_offset(home, stride, Phase::Swing(0.0));
    let arriving = contact_offset(home, stride, Phase::Swing(1.0));
    let (from, to) = (beneath(leaving), beneath(arriving));

    // Smoothstep rather than a straight line, so the foot leaves and arrives
    // with no vertical speed of its own and the whole of the climb happens in
    // the middle of the swing, where the arc has already lifted it clear.
    let base = from + (to - from) * smoothstep(t);

    // How far above the higher of the two ends anything in between rises. The
    // arc has to clear that as well as make its own toe clearance, so the two
    // add: a step gets the lift a flat walk gets, on top of the step.
    let mut over = 0.0f32;
    let probes = clearance_probes(home, stride);
    for probe in 1..probes {
        let at = probe as f32 / probes as f32;
        let along = leaving.lerp(arriving, at);
        over = over.max(beneath(along) - from.max(to));
    }

    let apex = stride.lift + over.max(0.0);
    Vec3::new(
        offset.x,
        base + apex * (t * std::f32::consts::PI).sin(),
        offset.z,
    )
}

/// How many points along a swing are probed for what it has to clear.
///
/// **At the resolution of the body's own foot**, because a foot cannot fall
/// into a gap narrower than itself: ground that rises and falls again within
/// one foot length is ground the foot bridges. That makes the count a property
/// of the body and the stride rather than a number, so a long stride on a small
/// body probes more finely than a short one on a large.
///
/// **What this cannot see, said out loud**: a step whose tread is shorter than
/// the foot is a step the foot spans, and one narrower than the spacing here
/// can be stepped over without being cleared. At least two, so a swing always
/// has one sample in the middle of it.
fn clearance_probes(home: Vec3, stride: &Stride) -> usize {
    let _ = home;
    let foot = stride.lift.max(1e-3);
    ((stride.length / foot).ceil() as usize).clamp(2, 16)
}

/// How far the toe rides above the sole at heel-strike, in radians.
///
/// Twenty degrees, in the middle of the 15–25 the gait literature reports for a
/// normal adult. It is the attitude the foot arrives in rather than one it
/// holds: [`foot_pitch`] rolls it away again over the first [`SOLE_DOWN`] of the
/// stance.
const HEEL_STRIKE: f32 = 0.35;

/// How far the toe rides below the sole at toe-off, in radians.
///
/// Seventeen degrees, against a reported 15–20. Smaller than [`HEEL_STRIKE`]
/// because a rigid foot spends it about the toe rather than about the ball —
/// see [`roll_feet`] on the toes this body has not got.
const TOE_OFF: f32 = 0.30;

/// Share of the stance spent rolling from heel-strike down to a flat sole.
///
/// The loading response, which is over quickly: the shin travels forward over a
/// foot that is already down, so the roll is the fastest thing the ankle does
/// all cycle.
const SOLE_DOWN: f32 = 0.15;

/// Share of the stance the heel keeps down before it begins to peel.
///
/// Heel-rise starts about half way through a stance and finishes at toe-off, so
/// the peel is spread over the rest of it. A foot that starts peeling at the
/// moment it lands never reads as bearing weight.
const HEEL_PEEL: f32 = 0.55;

/// How many times [`roll_feet`] pins the ankle and re-solves the leg under it.
///
/// The same fixed point [`super::plant_feet_of`] iterates, for the same reason:
/// solving the leg turns the shin, which carries the ankle's parent with it, so
/// a foot pitched before the solve is not quite pitched after it.
///
/// **Four, re-swept under #254 — it was two, and two was right at the time.**
/// The number depends on how accurately the leg solve places the contact, and
/// that changed: with a solve that missed by the extremity's hang, the contact
/// barely moved from pass to pass and two passes converged. With one that lands
/// where it was aimed, the leg moves further each pass and the fixed point
/// takes longer to settle.
///
/// Swept on the default walk through `examples/walkaudit`, asked for -17.2 to
/// 20.1 degrees: two passes now deliver -19.9 to 18.5 and scuff the sole 8.3 mm
/// under the floor; three deliver -17.6 to 19.8 and clear it by 0.7 mm; four
/// deliver -17.3 to 20.0 and clear by 2.1; six deliver the asked figures exactly
/// and clear by 2.4. Four is where the degrees are inside a tenth and the
/// clearance has stopped moving usefully.
///
/// **The lesson is the constant, not the number.** A pass count swept against
/// one version of the thing it iterates is a number with a hidden argument, and
/// this is the second constant in this file to need re-measuring for exactly
/// that reason (see [`super::ground::solve_contact_toward`]).
const ROLL_PASSES: usize = 4;

/// A Hermite ramp, `0..1`, flat at both ends.
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How far the sole is pitched at this point of the cycle, in radians, positive
/// toe-up.
///
/// **One function of the whole cycle, not of the stance alone.** A foot does not
/// teleport: the attitude it leaves the ground in is the one it carries into its
/// swing, and the attitude it lands in is the one the swing has to deliver. Written
/// per-phase, the two ends would each be right and the joins between them would
/// snap.
///
/// The stance is the part the literature names: land at `HEEL_STRIKE` toe-up,
/// roll flat within `SOLE_DOWN` of the stance, hold the sole down until
/// `HEEL_PEEL`, then peel to `TOE_OFF` toe-down at the moment of lift. The
/// swing simply carries the toe-down attitude back round to the toe-up one, and
/// passing through flat on the way is what gives a swinging toe its clearance.
///
/// Every ramp is a `smoothstep`, so the pitch is *C¹* across all four joins —
/// including the two the phases share. That is not decoration: a corner in this
/// curve is an infinite angular acceleration at the ankle, and it reads as a
/// flick.
#[must_use]
pub fn foot_pitch(phase: Phase) -> f32 {
    match phase {
        Phase::Stance(t) if t < SOLE_DOWN => HEEL_STRIKE * (1.0 - smoothstep(t / SOLE_DOWN)),
        Phase::Stance(t) if t < HEEL_PEEL => 0.0,
        Phase::Stance(t) => -TOE_OFF * smoothstep((t - HEEL_PEEL) / (1.0 - HEEL_PEEL)),
        Phase::Swing(t) => -TOE_OFF + (HEEL_STRIKE + TOE_OFF) * smoothstep(t),
    }
}

/// Rolls each foot through the attitude its phase calls for, and moves the leg
/// to keep the sole on the ground while it does.
///
/// **Call after [`super::plant_feet_of`]**, the way [`swing_arms`] is called
/// after [`step`]. The footing solve lays every sole flat along the ground; this
/// is what takes it off again, and running it first would simply be levelled
/// away.
///
/// `ground` answers what is beneath a point, exactly as [`step`] and
/// [`super::plant_feet_of`] ask it, and it must be the same floor all three are
/// given — see [`Walk::drive`].
///
/// # The pitch rides on the surface, not on the ankle (#250)
///
/// This used to take the attitude to pitch about from the ankle's own rotation,
/// on the reasoning that [`super::level_feet`] had just laid the sole into the
/// ground and so the ankle already held the answer. It does not, on ground
/// steep enough to matter: the clamp in [`super::level_feet`] is on the *local*
/// ankle angle, and a 30% grade asks 44.1 degrees of an ankle that has 40.1
/// ([`FootingConfig::max_ankle`]). What the ankle reports there is where it got
/// to rather than where it was sent, and a roll built on it puts the pitch on
/// the wrong plane and picks the wrong sole point to pivot about.
///
/// Measured by `examples/walkaudit` — worst sole pass over one cycle, against
/// 2.3 mm of clearance on the flat:
///
/// | grade | −40% | −30% | +30% | +40% |
/// |-------|------|------|------|------|
/// | read off the ankle | −6.8 mm | −2.9 mm | −17.1 mm | −47.7 mm |
/// | read off the ground | −6.8 mm | −2.9 mm | −1.6 mm | −5.5 mm |
///
/// **The asymmetry was the tell**, and it is worth recording because it is what
/// a downhill-only test would have missed entirely: climbing needs the ankle to
/// flex further than descending does, so only the uphill readings ever clamped.
///
/// The remainder is symmetric in the grade and is a different, smaller term
/// this did not touch. So is the whole lateral axis: a body walking *across* a
/// slope still puts a swinging foot 52 mm through the hillside at 30% camber,
/// which is #255 and is larger than what this fixed.
///
/// Returns the contacts whose leg could not reach the rolled goal, which should
/// be none — rolling *raises* the contact at both extremes, so it asks the leg
/// for less reach than the plant already got.
///
/// # The pivot is derived rather than chosen
///
/// Pitching the ankle in place swings the whole foot about a joint that is
/// inside it: on the default body the ankle sits 91 mm above the sole and 19 mm
/// ahead of the heel, so a naive toe-down drives the toe through the floor. A
/// foot rolls about whatever part of its **sole** is bearing weight, and which
/// part that is changes with the sign of the pitch.
///
/// Neither end is written down here. Every plan in this crate builds its bodies
/// standing on `y = 0` — it is what [`crate::extremity::Extremities::build`]
/// takes a ground plane for — so each of the foot's own joints stands over a
/// sole point at rest height zero, and that is a sole this module can read
/// without a mesh. Roll that set of points by the pitch, and the one that ends
/// up lowest is the one bearing weight; translate the foot so that point does
/// not move. Toe-up selects the back of the heel and toe-down selects the toe,
/// both for free, and a foot shaped like nothing in this crate gets whichever
/// answer its own geometry gives.
///
/// Measured on the default body by `examples/walkaudit`, which is the argument
/// for the whole construction: pitching about the contact joint instead drives
/// the sole **60.5 mm** under the floor, and the derived pivot leaves the
/// tightest pass of the cycle **10.0 mm above** it.
///
/// # A foot of one node is left alone
///
/// `plan::quadruped` gives each limb a single extremity node, so a beast in this
/// crate has no heel-to-toe run to pitch along and no second sole point to bear
/// the weight. It is left exactly as the plant left it, because deriving an axis
/// from one point means choosing one, and a hoof pitched about a guess is worse
/// than a hoof that does not pitch. Giving a beast a foot it can roll is a plan
/// question (#178), not a gait one.
///
/// # The toes do not extend, and that is a choice
///
/// A real push-off keeps the ball *and* the toe down and opens the joint between
/// them; this rolls a rigid foot up onto its toe tip. Counter-rotating the ball
/// would buy that anatomy and cost the instrument its reading — with the
/// forefoot held flat, the heel-to-toe run `examples/walkaudit` measures reports
/// 0.70 of the pitch asked, so `TOE_OFF` would no longer deliver the degrees
/// it is written in. Recorded as a cost rather than smuggled in.
pub fn roll_feet<F>(
    rig: &Rig,
    pose: &mut Pose,
    gait: &Gait,
    stride: &Stride,
    cycle: f32,
    ground: F,
) -> Vec<Limb>
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let mut straining = Vec::new();
    // A gait that never lifts a contact expresses no roll either — the same
    // rule [`step`] and [`crouch_at`] apply to their own outputs (#230). A
    // standing body puts its soles down flat and leaves them there.
    if !pose.fits(rig) || gait.duty >= 1.0 {
        return straining;
    }

    for (index, &limb) in gait.limbs.iter().enumerate() {
        let phase = gait.phase(index, cycle);
        // **Scaled by how much of the travel is fore-and-aft** (#242). Forward
        // walking lands on the heel and leaves from the toe; backwards it is
        // exactly the other way round, and sideways the sole comes down flat.
        // All three are one multiplication by the heading's forward share,
        // which is `+1`, `−1` and `0` — so a diagonal gets a partial roll
        // rather than a choice between two of them, and nothing pops as the
        // heading swings.
        let pitch = foot_pitch(phase) * Heading::toward(stride.direction).along();
        let heading = contact_heading(stride, phase);
        // Nothing to do is nothing to do — but it now takes both to be nothing,
        // and a foot flat in mid-stance on a turn still has a bearing to hold.
        if pitch == 0.0 && heading == 0.0 {
            continue;
        }
        if roll_one(rig, pose, limb, pitch, heading, &ground) == Some(false) {
            straining.push(limb);
        }
    }

    straining
}

/// Rolls one foot by `pitch` about whichever of its sole points bears the
/// weight, and turns it to `heading`, both measured against the surface under
/// that foot.
///
/// `None` for a limb with no foot to roll — one the body has not got, or one
/// whose extremity is a single node and so has no length to pitch along.
/// Otherwise whether the leg reached the goal the roll asked for.
fn roll_one<F>(
    rig: &Rig,
    pose: &mut Pose,
    limb: Limb,
    pitch: f32,
    heading: f32,
    ground: &F,
) -> Option<bool>
where
    F: Fn(Vec3) -> Option<Ground>,
{
    let joints = rig.extremity_joints(limb);
    let (&ankle, sole) = (joints.first()?, joints.get(1..)?);
    let parent = rig.joints[ankle].parent?;
    let contact = *rig.in_zone(Zone::Extremity(limb)).first()?;

    // The joint being turned has to be ABOVE the foot, or turning it does not
    // move the contact and the pivot arithmetic below is measuring a foot that
    // is only half following. [`Rig::extremity_joints`] delivers that by
    // prepending the joint the extremity hangs from — but only where there is
    // one, so a limb rooted at its own extremity is left alone rather than
    // rolled about itself.
    if rig.joints[ankle].zone == Zone::Extremity(limb) {
        return None;
    }

    // The attitude the footing solve settled the foot at. Everything below is
    // measured against this one pose, so the passes do not chase themselves.
    let posed = pose.forward(rig);
    let placed = posed.rotations[ankle];

    // **The surface, asked for directly, rather than read back off the ankle**
    // (#250). What the settled attitude reports is where the ankle *got to*,
    // which on a steep grade is not where [`super::level_feet`] sent it: the
    // clamp there is on the local ankle angle, and a 30% grade asks 44.1
    // degrees of an ankle that has 40.1. Rolling about a short base then puts
    // the pitch on the wrong plane and picks the wrong sole point to pivot
    // about, and the foot ends up through the hill it is climbing.
    let up = ground(posed.positions[contact])
        .map_or(Vec3::Y, |ground| ground.normal)
        .normalize_or(Vec3::Y);

    // The base attitude the pitch rides on: the placed foot with its sole laid
    // into the surface. Correcting the up axis rather than rebuilding the
    // rotation keeps the heading the leg gave it — a foot points where the leg
    // swung it, and only its tilt is the ground's business.
    let settled = Quat::from_rotation_arc((placed * Vec3::Y).normalize_or(Vec3::Y), up) * placed;

    // **And then turned to face where it was planted** (#241). About the
    // surface's own normal rather than about `+Y`, which is what keeps the sole
    // flat: a rotation about the axis a plane's normal points along carries the
    // plane onto itself, so the bearing costs the levelling nothing. Applied on
    // the outside, in world space, because it is the foot's attitude in the
    // WORLD that a planted foot holds — the whole point of the reading.
    //
    // Ahead of the pitch below, so the pitch axis is derived from a foot that
    // is already pointing the right way; taking the run off an unturned foot
    // would pitch it along an axis it no longer has.
    let settled = if heading == 0.0 {
        settled
    } else {
        Quat::from_axis_angle(up, heading) * settled
    };

    // The foot's run, rearmost sole joint to foremost, carried into the pose.
    // `None` where the two coincide: a foot of one node has no direction to be
    // pitched along, and neither has one whose joints stack vertically.
    let along = |&joint: &usize| rig.joints[joint].position.z;
    let rear = *sole.iter().min_by(|a, b| along(a).total_cmp(&along(b)))?;
    let fore = *sole.iter().max_by(|a, b| along(a).total_cmp(&along(b)))?;
    let run = settled * (rig.joints[fore].position - rig.joints[rear].position);
    let axis = run.cross(up).try_normalize()?;
    let roll = Quat::from_axis_angle(axis, pitch);

    // Each sole point, as an offset from the contact joint. The point is
    // directly beneath its joint at rest height zero, which is where the build
    // put the floor.
    let rest = rig.joints[contact].position;
    let ground = |joint: usize| {
        let at = rig.joints[joint].position;
        settled * (Vec3::new(at.x, 0.0, at.z) - rest)
    };
    let bearing = sole
        .iter()
        .map(|&joint| ground(joint))
        .min_by(|a, b| (roll * *a).dot(up).total_cmp(&(roll * *b).dot(up)))?;

    // Where the contact joint has to go for the bearing point to stay put.
    let goal = posed.positions[contact] + bearing - roll * bearing;
    let want = roll * settled;

    let mut reached = false;
    for _ in 0..ROLL_PASSES {
        // **Assigned, not composed**, exactly as [`super::level_feet`] assigns
        // the attitude it wants: this is a constraint on where the sole ends up
        // rather than a contribution to a gesture, so it is re-established from
        // the settled attitude every pass instead of piling one roll on the
        // last.
        let outer = pose.forward(rig).rotations[parent];
        pose.rotations[ankle] = outer.inverse() * want;
        reached = solve_contact(rig, pose, limb, goal);
    }

    Some(reached)
}

/// How far an arm swings fore and aft, in radians.
const ARM_SWING: f32 = 0.46;

/// How far the arms drop from the pose the body is built in, in radians.
///
/// Bodies are modelled with their arms already angled down — an A-pose — so this
/// is only the rest of the way to a hanging arm, not the whole of it. Measured
/// from a T-pose the same drop needs roughly twice the rotation, and a shoulder
/// turned that far bulges however it is skinned.
const ARM_DROP: f32 = 0.66;

/// How far the shoulders twist against the hips, in radians.
///
/// Spread down the spine rather than spent at one joint — see [`swing_arms`].
const SHOULDER_TWIST: f32 = 0.17;

/// How far the trunk pitches forward at a natural walking pace, in radians.
///
/// Five and a half degrees, in the middle of the 2–7 the gait literature
/// reports for a normal adult trunk, and growing from there with pace. A body
/// that walks bolt upright reads as a mannequin being carried along: it was the
/// last of the three postural complaints #102 made about this gait, after the
/// elbows and the pelvis, and the only one nothing had been done about.
///
/// **Against pace, not against speed in metres.** Pace here is the stride the
/// body is actually taking measured against the legs taking it — recovered from
/// [`Stride::for_body`]'s own relation — so a short body and a tall one at the
/// same dimensionless stride lean by the same amount. That is the dynamic
/// similarity the whole gait is built on, and it is the form #240's speed axis
/// will want.
const TRUNK_LEAN: f32 = 0.096;

/// How far behind the legs the arms run, as a share of the cycle.
///
/// Not zero. Arms driven in lockstep with the legs read as clockwork; the lag is
/// most of what makes a swing look like it is being carried rather than driven.
const ARM_LAG: f32 = 0.07;

/// How far the elbow is bent even when the arm hangs at rest, in radians.
///
/// **An arm is never straight.** A body walking with locked elbows reads as a
/// mannequin being carried along, and the rest pose this plan builds is
/// perfectly straight — hip to knee to ankle and shoulder to elbow to wrist are
/// each three collinear points, so without this the arm has no bend at all.
const ELBOW_REST: f32 = 0.30;

/// How much further the elbow closes as the arm swings forward, in radians.
///
/// A walking arm folds on the way through and opens again behind, which is most
/// of what stops the swing reading as a pendulum.
const ELBOW_SWING: f32 = 0.26;

/// Swings the arms against the legs and counter-rotates the shoulders.
///
/// The arm on one side follows the leg on the *other*, which is the whole of why
/// a walk reads as a walk: a body swinging its arms in time with the legs on the
/// same side looks like it is marching, and one not swinging them at all — which
/// is what this gait did until now — does not look like it is walking.
///
/// Call after [`step`], which places the feet; this only touches the upper body.
///
/// **A limb the body stands on is left alone.** A quadruped's fore limbs are
/// legs, and they have just been placed by an IK solve; swinging them as though
/// they were arms moved each fore contact by 0.21 to 0.24 m every frame the
/// render tool drew. Asking which limbs carry the body — rather than which end
/// of it they are on — is also what makes this right for bodies nobody has
/// planned, a centaur's arms swinging while its four legs walk.
///
/// **Rotations are composed, not assigned.** This was the one pose producer
/// that overwrote whatever ran before it, which is why it could destroy an IK
/// solve rather than merely disagree with one. Each producer contributes to a
/// pose once; running this twice compounds its own drop, as any additive layer
/// would.
pub fn swing_arms(rig: &Rig, pose: &mut Pose, gait: &Gait, stride: &Stride, cycle: f32) {
    if !pose.fits(rig) {
        return;
    }
    // **The arms answer the legs' FORE-AND-AFT excursion, because that is what
    // they counterbalance** (#242). A walking body's legs swing angular
    // momentum about its vertical axis and the arms cancel it; a body shuffling
    // sideways generates none and swings its arms not at all, and one walking
    // backwards generates it the other way round and swings them the other way.
    //
    // The damping the backward case wants arrives here too, without a second
    // constant: a backward stride is three quarters of a forward one, so the
    // arms come with it. Forward this is exactly one and the swing is what it
    // always was.
    let heading = Heading::toward(stride.direction);
    let travel = heading.along() * heading.reach();

    let carries = rig.ground_contacts();
    let mut lead = 0.0;
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        if carries.contains(&limb) {
            continue;
        }
        let Some([shoulder, elbow, _]) = rig.limb_chain(limb) else {
            continue;
        };
        // The leg diagonally opposite drives this arm.
        let Some(driver) = gait
            .limbs
            .iter()
            .position(|&other| other == limb.mirrored().paired())
        else {
            continue;
        };
        let offset = gait.offsets.get(driver).copied().unwrap_or(0.0);
        // The legs drive the arms, so legs that never move drive nothing: at
        // duty 1.0 every offset is 0.0 and both arms would ride the SAME sine,
        // swinging in sync on a body that is standing still (#230). Zero drive
        // keeps the drop and the resting elbow below — a standing body hangs
        // its arms at its sides, it does not pump them.
        let drive = if gait.duty >= 1.0 {
            0.0
        } else {
            ((cycle - offset + ARM_LAG) * std::f32::consts::TAU).sin() * travel
        };
        if limb == Limb::ForeLeft {
            lead = drive;
        }

        // Down first, then fore and aft about the body's own axis. Positive
        // rotation about X carries a hanging arm backward, so a forward swing is
        // the negative one.
        let side = rig.joints[shoulder].position.x.signum();
        pose.rotations[shoulder] *=
            Quat::from_rotation_x(-ARM_SWING * drive) * Quat::from_rotation_z(-ARM_DROP * side);

        // **The elbow folds forward, about X, and the same way on both arms.**
        // This used to turn about Y with a `side` factor, and both halves of
        // that were wrong. The arm hangs in the frontal plane, so Y is only 50
        // degrees off its own axis and most of the rotation was spent spinning
        // the forearm rather than bending it — measured on the walk, the elbow
        // reached 15 degrees of bend for 19 degrees of rotation asked. What
        // little bend there was went sideways, in and out from the body, rather
        // than forward where an elbow folds. And `side` mirrored the fold, so
        // one arm bent forward while the other bent back; an elbow is not
        // chiral, and folding it is the one thing about an arm that is the same
        // on both sides.
        //
        // **And it folds about the axis the drop has already carried away from
        // X.** The elbow's frame hangs off the shoulder rotation above, so a
        // plain local X sits `ARM_DROP` off the world's by the time it acts —
        // measured at the joint, 0.64 of every degree asked arrived as bend and
        // the rest rolled the forearm about its own length (#223). Undoing the
        // drop in the axis puts the fold back on world X, where a fold ahead of
        // a hanging arm is a bend and nothing else, and the constants deliver
        // the degrees they are written in.
        let fold = Quat::from_rotation_z(ARM_DROP * side) * Vec3::X;
        pose.rotations[elbow] *=
            Quat::from_axis_angle(fold, -(ELBOW_REST + ELBOW_SWING * drive.max(0.0)));
    }

    // Shoulders against hips, and then the neck against the shoulders so the
    // head keeps looking where it is going rather than being turned by the walk.
    //
    // **Spread down the whole spine, not spent at the top of it.** The twist
    // used to go entirely into the joint the arms hang from, which turned the
    // shoulders by the right angle and left the ribcage and waist beneath them
    // dead still — the shoulders read as a yoke swivelling on a post rather
    // than as a torso winding. Sharing it out costs nothing: local rotations
    // compound down a chain, so the shoulders still arrive at the same angle.
    //
    // Weighted toward the top, which is where a spine actually turns: the
    // shares run 1, 2, 3 up the chain, so the waist contributes a sixth and the
    // shoulder girdle a half. Derived from the chain the body has rather than
    // written out, so a plan with a longer spine winds along all of it.
    if let Some(&neck) = rig.in_zone(Zone::Neck).first()
        && let Some(girdle) = rig.joints[neck].parent
    {
        let spine = spine_to(rig, girdle);
        let total: f32 = (1..=spine.len()).map(|rank| rank as f32).sum();
        for (rank, &joint) in spine.iter().enumerate() {
            let share = (rank + 1) as f32 / total.max(1.0);
            pose.rotations[joint] *= Quat::from_rotation_y(SHOULDER_TWIST * lead * share);
        }
        pose.rotations[neck] *= Quat::from_rotation_y(-SHOULDER_TWIST * lead);
    }
}

/// How fast this stride is, as a multiple of the pace [`Stride::for_body`]
/// calls natural.
///
/// Recovered from the stride rather than passed alongside it, so a caller
/// cannot tell the lean one pace and the legs another. The relation is
/// `for_body`'s own, read backwards: a body's natural stride is a fixed share
/// of the reach of the legs taking it, so dividing one by the other leaves a
/// dimensionless number that means the same thing on any body.
fn pace_of(rig: &Rig, stride: &Stride) -> f32 {
    let reach = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    if reach <= f32::EPSILON {
        return 0.0;
    }
    (stride.length / (reach * STRIDE_OF_REACH)).max(0.0)
}

/// Pitches the trunk forward into the walk, and holds the head level over it.
///
/// A walking body leans; a standing one does not. The lean grows with pace
/// because it is what a body does to put its mass ahead of its feet — the
/// literature reports 2–7 degrees at a normal walking pace and more as that
/// pace rises, and this crate's own figure sits in the middle of that band at
/// the pace [`Stride::for_body`] calls natural.
///
/// **Pitched rigidly from the lowest spine joint, where the twist is spread
/// along the whole chain.** The two are opposite on purpose and the anatomy is
/// the reason: a spine genuinely twists along its length, so the shoulder wind
/// is shared out; a trunk leans as one piece about the hip, which is the joint
/// at the bottom of that same chain. Writing the lean the way the twist is
/// written also makes the constant lie — **spread down the spine it delivered
/// 2.1 degrees of the 5.5 it asked for**, because local rotations compound but
/// the trunk's CHORD from pelvis to shoulders ends up a length-weighted average
/// of them rather than their sum. That is the #223 elbow defect exactly: a
/// constant nothing was checking against the body it moved. Pitching the base
/// carries everything above it by the angle written here, and
/// `examples/walkaudit` reads back what this says.
///
/// **The head is put back level.** The neck takes the whole lean off again, so
/// a body walking faster looks where it is going instead of at its own feet.
/// That is the same bargain [`swing_arms`] strikes with the shoulder twist, and
/// for the same reason: a gaze dragged around by locomotion reads as a body
/// with no attention of its own. [`super::look_at`] composes on top of this and
/// is unaffected.
///
/// Call after [`step`], like [`swing_arms`]; it touches nothing below the
/// pelvis and so cannot disturb a footing solve.
///
/// **Rotations are composed, not assigned**, so this stacks with the twist
/// rather than replacing it — running it twice compounds its own lean, as any
/// additive layer would.
pub fn lean(rig: &Rig, pose: &mut Pose, gait: &Gait, stride: &Stride) {
    if !pose.fits(rig) {
        return;
    }
    // A gait that never lifts a contact is standing, whatever stride it was
    // handed — the same rule `step` and `crouch_at` apply (#230). A standing
    // body stands up straight.
    if gait.duty >= 1.0 {
        return;
    }
    // **One inclination rather than two rotations, and the reason is
    // simplicity — not correctness.** The trunk is pitched forward by the pace
    // and banked sideways by the turn, and those are the two components of the
    // single direction the body leans in: the way its own effective gravity
    // points once the centripetal demand is added to the real one. Solving each
    // separately and composing the two results is the obvious alternative and
    // it was written here as the thing this form fixes.
    //
    // It fixes nothing. Measured against it, the two constructions agree to
    // 0.02 degrees at a 4.3 degree bank and 0.08 at 8.5, which is past the
    // sharpest turn a walk takes; they only part company at banks a body would
    // have to be running to reach, and there neither is right — the combined
    // form overshoots the pitch it was asked for (14.4 delivered of 12.0 at a
    // 40 degree bank) because the solve pins the inclination toward `toward`
    // rather than that inclination's forward component, and the composed form
    // undershoots it by more. What this form actually buys is one solve instead
    // of two and one axis convention instead of two. That is enough of a
    // reason; the correctness claim was not, and was checked.
    let pitch = TRUNK_LEAN * pace_of(rig, stride);
    let bank = bank_of(rig, gait, stride);
    // **Along the way the body is GOING, not the way it faces** (#242). The
    // lean exists to put the body's mass ahead of its feet in the direction it
    // is travelling, so a body walking backwards leans back and one shuffling
    // sideways leans that way. On a forward walk this is `+Z` and nothing
    // moves, which is every reading this crate has taken.
    let travel = stride.direction.normalize_or(Vec3::Z);
    let toward = (travel * pitch + Vec3::X * bank).normalize_or_zero();
    let wanted = (pitch * pitch + bank * bank).sqrt();
    if wanted <= f32::EPSILON || toward == Vec3::ZERO {
        return;
    }

    let Some(&neck) = rig.in_zone(Zone::Neck).first() else {
        return;
    };
    let Some(girdle) = rig.joints[neck].parent else {
        return;
    };
    let spine = spine_to(rig, girdle);
    if spine.is_empty() {
        return;
    }
    // The axis that carries `+Y` toward `toward`. For a pure forward lean that
    // is `+X`, and a positive rotation about it carries `+Y` toward `+Z` —
    // where a forward arm swing was the negative one. The difference is not a
    // sign error waiting to happen: an arm hangs DOWN and a trunk stands UP, so
    // the same rotation carries them opposite ways. Writing the axis as a cross
    // product rather than naming it is what lets the bank in without a second
    // convention to keep straight.
    let axis = Vec3::Y.cross(toward);
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let hinge = spine[0];
    let below = rig.joints[hinge].position - rig.joints[root].position;
    let above = rig.joints[girdle].position - rig.joints[hinge].position;
    let turn = trunk_angle_for(below, above, wanted, axis, toward);
    pose.rotations[hinge] *= Quat::from_axis_angle(axis, turn);
    pose.rotations[neck] *= Quat::from_axis_angle(axis, -turn);
}

/// Where on its own path the body will be `cycles` from now, in body space, at
/// the height of its head.
///
/// **The gaze's target, and a point rather than an angle** so that
/// [`super::look_at`] can be handed it directly. The gaze layer already spreads
/// a turn down the chest, neck and head and already clamps it at a neck's
/// limit; an angle computed here would be a second opinion about both.
///
/// The lead is a *distance* ahead rather than an angle, which is what makes it
/// derived rather than dialled: a body walking a bend looks where it is going,
/// so the further round the bend that is, the further its head has come. On a
/// straight line the point is straight ahead and the gaze does nothing at all.
///
/// Taken from the stride for `bank_of`'s reason — a caller cannot aim the
/// head down a curve the feet are not walking.
#[must_use]
pub fn path_ahead(rig: &Rig, gait: &Gait, stride: &Stride, cycles: f32) -> Vec3 {
    let duty = gait.duty.clamp(f32::EPSILON, 1.0);
    let angle = stride.yaw / duty * cycles;
    let ahead = stride.length / duty * cycles;
    let (along, across) = if angle.abs() < 1e-4 {
        (1.0 - angle * angle / 6.0, angle * 0.5)
    } else {
        (angle.sin() / angle, (1.0 - angle.cos()) / angle)
    };
    let forward = stride.direction.normalize_or(Vec3::Z);
    let sideways = Vec3::Y.cross(forward);
    let eye = rig
        .in_zone(Zone::Head)
        .first()
        .map_or(0.0, |&joint| rig.joints[joint].position.y);
    forward * (ahead * along) + sideways * (ahead * across) + Vec3::Y * eye
}

/// How far the trunk banks into the turn this stride describes, in radians,
/// positive toward the body's left.
///
/// [`super::Turn::bank`] is where the physics is written down; this is that
/// quantity **recovered from the stride** instead of passed beside it, which is
/// exactly [`pace_of`]'s argument for the pitch: a caller that carries a bank
/// next to a stride can tell the trunk one turn and the legs another, and a
/// body leaning into a corner its feet are not walking is worse than one that
/// does not lean at all.
///
/// The recovery is `atan(Fr · L · c)`, which is `atan(v·ω/g)` rewritten in
/// quantities the stride has: `c = yaw / length` is the curvature of the path,
/// `L` is the leg, and the Froude number comes from [`super::Speed::of`]. All
/// three are exact except the last.
///
/// # What the recovery costs, measured rather than estimated
///
/// [`super::Speed::of`] reads the **centreline** step against Grieve's
/// relation, while [`super::Turn::stride`] set the cadence from the working
/// contact — so on a turn the recovered speed sits under the true one and the
/// bank with it. Against [`super::Turn::bank`], which is the statics and has no
/// recovery in it, on the default body:
///
/// | radius | 4.0 m | 2.4 m | 1.3 m | 0.67 m | 0.33 m |
/// |--------|-------|-------|-------|--------|--------|
/// | short by | 3.7% | 3.1% | 10.3% | 19.1% | 32.9% |
///
/// **The shortfall is not a function of the radius but of the radius against
/// the stance width**, which is why the 2.4 m column beats the 4.0 m one — the
/// first is a fast body and the second a slow one. It is under four percent
/// wherever the turn is wider than a few paces, and it reaches a third only
/// under half a metre of radius, where the body is very nearly pivoting.
///
/// **Left, rather than fixed, and on an argument rather than for the effort.**
/// Grieve's relation is fitted to bodies walking in a straight line and there
/// is no published one for a body walking a curve, so reading a turning body's
/// step against it is approximate by construction and no arrangement of these
/// terms makes it exact. The direction of the error is the useful part: people
/// walking sharp turns are reported *not* to reach the ideal bank — they place
/// their feet instead, which is the differential stride this crate now has — so
/// under-leaning where the radius closes up is closer to a body than the ideal
/// angle would be. It is exact where the ideal angle is well founded and
/// conservative where it is not.
///
/// The two alternatives are both worse. Handing [`lean`] a speed re-opens
/// exactly the disagreement this is built to make unrepresentable, and setting
/// the cadence from the body's centre instead leaves the outside foot
/// over-striding to keep up.
fn bank_of(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    if stride.yaw == 0.0 || stride.length <= f32::EPSILON {
        return 0.0;
    }
    let leg = rig
        .ground_contacts()
        .into_iter()
        .filter_map(|limb| rig.limb_reach(limb))
        .fold(0.0f32, f32::max);
    let froude = super::speed::Speed::of(rig, gait, stride).froude();
    (froude * leg * (stride.yaw / stride.length)).atan()
}

/// How many times [`trunk_angle_for`] refines its guess.
///
/// **Three, measured.** The relation is a chord pitching about a point part way
/// along itself, so the angle asked for and the angle delivered differ by a
/// factor that itself depends on the angle. One pass lands within about a
/// tenth of a degree on the default body, two within a thousandth, and three is
/// indistinguishable from the limit on every seed swept — cheap enough that
/// there is no reason to run fewer, and the same fixed point [`roll_feet`]
/// iterates for the same kind of reason.
const TRUNK_PASSES: usize = 3;

/// The rotation to put at the base of the spine so the whole trunk arrives
/// inclined by `wanted`, toward `toward`, about `axis`.
///
/// **Solved rather than assumed, because the two are not the same angle.** The
/// trunk's inclination is the pitch of the chord from the pelvis to the
/// shoulders, which is what the gait literature measures and what
/// `examples/walkaudit` reads back. But the pelvis cannot be rotated — it
/// carries the legs, and turning it turns them out from under the footing solve
/// — so the hinge is the joint above it, and the segment `below` it stays put
/// while only `above` swings. The chord is then a length-weighted mix of a
/// still part and a turned one, and it arrives at a fraction of the angle
/// applied: 3.0 degrees of a 5.5 asked, on the default body.
///
/// Rather than let the constant mean a budget nobody can check, this inverts
/// the relation. `wanted` is the inclination, the return is whatever rotation
/// delivers it, and the two are the same number only on a body whose pelvis has
/// no height at all.
fn trunk_angle_for(below: Vec3, above: Vec3, wanted: f32, axis: Vec3, toward: Vec3) -> f32 {
    // The inclination of a run, measured in the plane it is being tilted in.
    // `toward.dot` rather than a named component, so the same solve answers for
    // a forward pitch, a sideways bank, and the mixture of the two a body
    // walking round a bend actually holds.
    let pitch = |run: Vec3| run.dot(toward).atan2(run.y);
    let rest = pitch(below + above);
    let mut turn = wanted;
    for _ in 0..TRUNK_PASSES {
        let delivered = pitch(below + Quat::from_axis_angle(axis, turn) * above) - rest;
        // The shortfall, applied to the guess. A body whose trunk is all pelvis
        // delivers nothing however far it turns, and dividing by that would
        // spin it; the guard leaves such a body upright, which is the honest
        // answer for one that cannot lean.
        if delivered.abs() <= f32::EPSILON {
            return 0.0;
        }
        turn *= wanted / delivered;
    }
    turn
}

/// The spine from the pelvis up to `top`, pelvis end first.
///
/// Walked up the parent chain and stopped at the root, so it is whatever spine
/// the body has rather than a list of names. The root itself is left out: it
/// carries the whole body, and turning it turns the legs too.
fn spine_to(rig: &Rig, top: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut at = Some(top);
    while let Some(joint) = at {
        if rig.joints[joint].parent.is_none() || !rig.joints[joint].zone.is_core() {
            break;
        }
        chain.push(joint);
        at = rig.joints[joint].parent;
    }
    chain.reverse();
    chain
}

/// Where a limb's contact rests when the body is standing.
fn home_of(rig: &Rig, limb: Limb) -> Option<Vec3> {
    let joint = *rig.in_zone(crate::plan::Zone::Extremity(limb)).first()?;
    Some(rig.joints[joint].position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams, Zone};

    fn biped() -> Rig {
        Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    fn quadruped() -> Rig {
        Rig::from_skeleton(&QuadrupedParams::default().skeleton(&crate::Composites::default()))
            .expect("rigs")
    }

    /// The world height of one contact in the given pose.
    fn contact_height(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let joint = rig.in_zone(Zone::Extremity(limb))[0];
        pose.forward(rig).positions[joint].y
    }

    #[test]
    fn a_gait_covers_exactly_the_body_that_carries_it() {
        assert_eq!(Gait::natural(&biped()).len(), 2);
        assert_eq!(Gait::natural(&quadruped()).len(), 4);
        assert_eq!(Gait::standing(&biped()).limbs, biped().ground_contacts());
    }

    #[test]
    fn standing_never_lifts_a_foot() {
        let gait = Gait::standing(&biped());
        for step in 0..20 {
            let cycle = step as f32 / 20.0;
            assert_eq!(gait.grounded(cycle), gait.len(), "a foot left the ground");
        }
    }

    #[test]
    fn a_walking_body_always_has_a_foot_down() {
        // The property that separates a walk from a stumble. It has to hold at
        // every point of the cycle, not merely on average.
        for rig in [biped(), quadruped()] {
            let gait = Gait::wave(&rig);
            for step in 0..120 {
                let cycle = step as f32 / 120.0;
                assert!(
                    gait.grounded(cycle) >= 1,
                    "{} contacts: nothing on the ground at {cycle}",
                    gait.len()
                );
            }
        }
    }

    #[test]
    fn a_two_legged_walk_has_both_feet_down_some_of_the_time() {
        // Without an overlap each foot leaves as the other lands, which is a run
        // performed slowly rather than a walk.
        let gait = Gait::wave(&biped());
        let samples = 200;
        let both = (0..samples)
            .filter(|step| gait.grounded(*step as f32 / samples as f32) == 2)
            .count() as f32
            / samples as f32;
        assert!(
            (both - 2.0 * DOUBLE_SUPPORT).abs() < 0.05,
            "double support was {both:.2} of the cycle"
        );
    }

    #[test]
    fn a_wave_gait_lifts_one_contact_at_a_time() {
        let rig = quadruped();
        let gait = Gait::wave(&rig);
        for step in 0..120 {
            let cycle = step as f32 / 120.0;
            let airborne = gait.len() - gait.grounded(cycle);
            assert!(airborne <= 1, "{airborne} feet airborne at {cycle}");
        }
    }

    #[test]
    fn a_trot_moves_diagonal_pairs_together() {
        let rig = quadruped();
        let gait = Gait::trot(&rig);
        let offset_of = |limb: Limb| {
            let index = gait.limbs.iter().position(|&l| l == limb).expect("limb");
            gait.offsets[index]
        };

        assert_eq!(offset_of(Limb::ForeLeft), offset_of(Limb::HindRight));
        assert_eq!(offset_of(Limb::ForeRight), offset_of(Limb::HindLeft));
        assert_ne!(offset_of(Limb::ForeLeft), offset_of(Limb::ForeRight));
    }

    #[test]
    fn a_trot_falls_back_to_a_wave_on_a_body_without_four_corners() {
        let rig = biped();
        assert_eq!(Gait::trot(&rig), Gait::wave(&rig));
    }

    #[test]
    fn phases_run_stance_then_swing_and_wrap() {
        let gait = Gait {
            limbs: vec![Limb::HindLeft],
            offsets: vec![0.0],
            duty: 0.6,
        };
        let close = |a: Phase, b: Phase| match (a, b) {
            (Phase::Stance(x), Phase::Stance(y)) | (Phase::Swing(x), Phase::Swing(y)) => {
                (x - y).abs() < 1e-4
            }
            _ => false,
        };
        assert!(close(gait.phase(0, 0.0), Phase::Stance(0.0)));
        assert!(close(gait.phase(0, 0.3), Phase::Stance(0.5)));
        assert!(close(gait.phase(0, 0.6), Phase::Swing(0.0)));
        assert!(close(gait.phase(0, 0.8), Phase::Swing(0.5)));
        // A cycle later is the same place.
        assert!(close(gait.phase(0, 1.3), gait.phase(0, 0.3)));
        assert!(close(gait.phase(0, -0.7), gait.phase(0, 0.3)));
    }

    /// A contact standing half a metre to the body's left, at ankle height.
    fn home() -> Vec3 {
        Vec3::new(0.5, 0.1, 0.0)
    }

    fn straight() -> Stride {
        Stride {
            direction: Vec3::Z,
            length: 0.8,
            lift: 0.1,
            yaw: 0.0,
        }
    }

    #[test]
    fn a_stance_foot_travels_backwards_and_a_swing_foot_lifts() {
        let stride = straight();

        let early = contact_offset(home(), &stride, Phase::Stance(0.0));
        let late = contact_offset(home(), &stride, Phase::Stance(1.0));
        assert!(early.z > late.z, "the body travels over a planted foot");
        assert_eq!(early.y, 0.0, "a planted foot stays down");

        let peak = contact_offset(home(), &stride, Phase::Swing(0.5));
        assert!(
            (peak.y - 0.1).abs() < 1e-5,
            "the swing should reach its lift"
        );
        assert!(contact_offset(home(), &stride, Phase::Swing(0.0)).y.abs() < 1e-5);
        assert!(contact_offset(home(), &stride, Phase::Swing(1.0)).y.abs() < 1e-5);
    }

    #[test]
    fn a_straight_stride_is_the_slide_it_always_was() {
        // The whole arc construction has to leave the straight line EXACTLY
        // where it found it, or every reading this crate has ever taken moves
        // under it. Asserted against the closed form the scalar version used,
        // at a contact well off the centreline so a rotation that leaked in
        // would show.
        let stride = straight();
        let half = stride.length * 0.5;
        for at in 0..=20 {
            let t = at as f32 / 20.0;
            let stance = contact_offset(home(), &stride, Phase::Stance(t));
            assert!(
                stance.distance(Vec3::Z * (half - stride.length * t)) < 1e-6,
                "stance {t}: {stance:?}"
            );
            let swing = contact_offset(home(), &stride, Phase::Swing(t));
            let along = Vec3::Z * (stride.length * t - half);
            let lift = Vec3::Y * (stride.lift * (t * std::f32::consts::PI).sin());
            assert!(swing.distance(along + lift) < 1e-6, "swing {t}: {swing:?}");
        }
    }

    #[test]
    fn a_step_ends_where_the_next_one_starts() {
        // Stance must hand off to swing without a jump, or the foot teleports
        // once per cycle. **On a turn as well**: the swing retraces the stance
        // rather than lerping its endpoints, and if it did lerp them the
        // handoff would still match while everything between the ends cut the
        // chord of the arc.
        for yaw in [0.0f32, 0.4, -0.4, 1.2] {
            let stride = Stride { yaw, ..straight() };
            let handoff = contact_offset(home(), &stride, Phase::Stance(1.0));
            let pickup = contact_offset(home(), &stride, Phase::Swing(0.0));
            assert!(
                handoff.distance(pickup) < 1e-5,
                "yaw {yaw}: {handoff:?} vs {pickup:?}"
            );

            let landing = contact_offset(home(), &stride, Phase::Swing(1.0));
            let plant = contact_offset(home(), &stride, Phase::Stance(0.0));
            assert!(
                landing.distance(plant) < 1e-5,
                "yaw {yaw}: {landing:?} vs {plant:?}"
            );
        }
    }

    #[test]
    fn a_planted_contact_holds_its_ground_through_a_turn() {
        // **The property the whole arc exists for**, asserted the way the
        // instrument measures it: carry the body-space offset back out through
        // the body's own travel and turn, and the foot must not have moved.
        //
        // Reintroducing the defect means dropping the rotation — using the
        // translation alone, which is what a per-limb SCALE on the stride
        // amounts to. That leaves the contact tracing the chord instead of the
        // arc and the check fails by centimetres at a metre's radius.
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.6,
            lift: 0.1,
            yaw: 0.5,
        };
        for home in [Vec3::new(0.09, 0.12, 0.0), Vec3::new(-0.09, 0.12, 0.0)] {
            let planted = |t: f32| {
                let share = t - 0.5;
                let angle = stride.yaw * share;
                let (along, across) = if angle.abs() < 1e-4 {
                    (1.0, angle * 0.5)
                } else {
                    (angle.sin() / angle, (1.0 - angle.cos()) / angle)
                };
                let travelled = stride.length * share;
                let origin = Vec3::Z * (travelled * along) + Vec3::X * (travelled * across);
                let at = home + contact_offset(home, &stride, Phase::Stance(t));
                origin + Quat::from_rotation_y(angle) * at
            };
            let anchor = planted(0.5);
            for at in 0..=20 {
                let moved = planted(at as f32 / 20.0).distance(anchor);
                assert!(moved < 1e-5, "the contact at {home:?} slid {moved} m");
            }
        }
    }

    #[test]
    fn the_inside_of_a_turn_takes_the_shorter_step() {
        // The differential stride, which nothing computes: it is what a
        // rotation does to two points at different radii. Measured as the
        // ground each contact covers across its own stance.
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.6,
            lift: 0.1,
            yaw: 0.5,
        };
        let covered = |home: Vec3| {
            let from = home + contact_offset(home, &stride, Phase::Stance(0.0));
            let to = home + contact_offset(home, &stride, Phase::Stance(1.0));
            from.distance(to)
        };
        // Positive yaw curves left, so the LEFT foot is on the inside.
        let inside = covered(Vec3::new(0.09, 0.12, 0.0));
        let outside = covered(Vec3::new(-0.09, 0.12, 0.0));
        assert!(
            inside < outside,
            "the inside foot covered {inside} m and the outside {outside}"
        );
        // And by the ratio of the two radii, which is the only number the
        // geometry allows: the path's radius is `length / yaw`.
        let radius = stride.length / stride.yaw;
        let wanted = (radius - 0.09) / (radius + 0.09);
        assert!(
            (inside / outside - wanted).abs() < 1e-3,
            "the two steps stood at {:.4} where the radii say {wanted:.4}",
            inside / outside
        );
    }

    #[test]
    fn the_trunk_banks_by_what_the_statics_ask_and_the_pitch_stays_where_it_was() {
        // Two things at once, because they are one rotation. The bank must
        // arrive within the shortfall `bank_of` documents, and adding it must
        // not have moved the forward pitch every reading this crate has taken
        // was measured against.
        //
        // **What this does not guard**, said out loud because it was tried:
        // composing two separate solves instead of one combined tilt passes
        // this test unchanged, because at a walk's banks the two constructions
        // are 0.08 degrees apart. The combined form is here for being one solve
        // rather than for being the right one — see `lean`.
        let rig = crate::Rig::from_skeleton(
            &HumanoidParams {
                height: 1.75,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs");
        let speed = super::super::Speed::new(&rig, 1.4);
        let level = |pose: &Pose| {
            let posed = pose.forward(&rig);
            let neck = rig.in_zone(Zone::Neck)[0];
            let girdle = rig.joints[neck].parent.expect("a girdle");
            let root = rig
                .joints
                .iter()
                .position(|joint| joint.parent.is_none())
                .expect("a root");
            let run = posed.positions[girdle] - posed.positions[root];
            let rest = rig.joints[girdle].position - rig.joints[root].position;
            (
                run.z.atan2(run.y).to_degrees() - rest.z.atan2(rest.y).to_degrees(),
                run.x.atan2(run.y).to_degrees() - rest.x.atan2(rest.y).to_degrees(),
            )
        };

        let straight = super::super::Turn::STRAIGHT;
        let mut upright = Pose::rest(&rig);
        lean(
            &rig,
            &mut upright,
            &straight.gait(&rig, speed),
            &straight.stride(&rig, speed),
        );
        let (pitch, roll) = level(&upright);
        assert!(roll.abs() < 1e-3, "a straight walk must not bank: {roll}");

        for degrees in [15.0f32, 30.0, -30.0] {
            let turn = super::super::Turn::new(degrees.to_radians());
            let gait = turn.gait(&rig, speed);
            let stride = turn.stride(&rig, speed);
            let mut pose = Pose::rest(&rig);
            lean(&rig, &mut pose, &gait, &stride);
            let (turned_pitch, turned_roll) = level(&pose);
            let wanted = turn.bank(&rig, speed).to_degrees();
            assert!(
                turned_roll * wanted > 0.0,
                "at {degrees} deg/s the body banked {turned_roll} where the turn asks {wanted}"
            );
            // Within the shortfall the doc names — under four percent at these
            // radii, which are 5.3 m and 2.7 m.
            assert!(
                (turned_roll.abs() - wanted.abs()).abs() < wanted.abs() * 0.06,
                "banked {turned_roll} against {wanted}"
            );
            // And the pitch is very nearly the walk's. **Not exactly**, and
            // the residue is geometry rather than interference: the segment
            // below the hinge does not turn, so the trunk's chord is a
            // length-weighted mix of a still part and a turned one — and a mix
            // taken in two planes at once dilutes each of them by a little more
            // than either alone. Measured at 1.6 percent of the lean at these
            // radii, 8.08 degrees falling to 7.92, which is a fortieth of the
            // band the literature quotes.
            assert!(
                (turned_pitch - pitch).abs() < pitch.abs() * 0.03,
                "the forward lean moved from {pitch} to {turned_pitch}"
            );
        }
    }

    #[test]
    fn the_head_leads_the_turn_and_a_straight_walk_costs_it_nothing() {
        // The two halves of why the gaze can be switched on without thinking
        // about it: down a curve the head really does come round, and down a
        // straight line the target is where the head already points so nothing
        // moves. Reintroducing the defect means aiming at a fixed distance
        // straight ahead — the target then never leaves the centreline and the
        // first assertion fails at every yaw rate.
        let rig = crate::Rig::from_skeleton(
            &HumanoidParams {
                height: 1.75,
                ..Default::default()
            }
            .skeleton(&crate::Composites::default()),
        )
        .expect("rigs");
        let speed = super::super::Speed::new(&rig, 1.4);
        let head = rig.in_zone(Zone::Head)[0];
        let facing = |walk: &Walk, turn: super::super::Turn| {
            let mut pose = Pose::rest(&rig);
            walk.drive(
                &rig,
                &mut pose,
                &turn.gait(&rig, speed),
                &turn.stride(&rig, speed),
                |at: Vec3| {
                    Some(Ground {
                        position: Vec3::new(at.x, 0.0, at.z),
                        normal: Vec3::Y,
                    })
                },
            );
            let forward = pose.forward(&rig).rotations[head] * Vec3::Z;
            forward.x.atan2(forward.z).to_degrees()
        };
        let looking = Walk {
            gaze: Some(GazeConfig::default()),
            ..Walk::at(0.25)
        };
        let blind = Walk::at(0.25);

        let straight = facing(&looking, super::super::Turn::STRAIGHT);
        assert!(
            (straight - facing(&blind, super::super::Turn::STRAIGHT)).abs() < 0.5,
            "a straight walk must not move the head: {straight} deg"
        );
        for degrees in [20.0f32, 60.0, -60.0] {
            let turn = super::super::Turn::new(degrees.to_radians());
            let led = facing(&looking, turn) - facing(&blind, turn);
            assert!(
                led * degrees > 0.0,
                "at {degrees} deg/s the head led by {led} deg — the wrong way"
            );
            assert!(
                led.abs() < 90.0,
                "at {degrees} deg/s the head led by {led} deg — past a neck"
            );
        }
    }

    #[test]
    fn a_backward_walk_lands_on_its_toe_and_a_shuffle_lands_flat() {
        // The whole of the foot's answer to a heading, and all three cases come
        // out of one multiplication by the heading's forward share.
        //
        // **Measured off the POSED foot, through the real drive.** Written the
        // obvious way — `foot_pitch(strike) * heading.along()` — this asserted
        // its own arithmetic and passed with the scaling removed from the
        // crate entirely, which is a test that can never fail from a change to
        // the thing it is named after.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let joints = rig.extremity_joints(Limb::HindLeft);
        let (heel, toe) = (joints[1], joints[joints.len() - 1]);
        let rest = {
            let run = rig.joints[toe].position - rig.joints[heel].position;
            run.y.atan2((run.x * run.x + run.z * run.z).sqrt())
        };
        let pitch_at = |heading: Heading, cycle: f32| {
            let stride = Stride::for_body(&rig, 1.0).toward(&rig, heading);
            let mut pose = Pose::rest(&rig);
            Walk::at(cycle).drive(&rig, &mut pose, &gait, &stride, |at: Vec3| {
                Some(Ground {
                    position: Vec3::new(at.x, 0.0, at.z),
                    normal: Vec3::Y,
                })
            });
            let posed = pose.forward(&rig);
            let run = posed.positions[toe] - posed.positions[heel];
            (run.y.atan2((run.x * run.x + run.z * run.z).sqrt()) - rest).to_degrees()
        };
        // At the strike, which is the start of this contact's stance.
        let forward = pitch_at(Heading::FORWARD, 0.0);
        let backward = pitch_at(Heading::BACKWARD, 0.0);
        let sideways = pitch_at(Heading::LEFT, 0.0);
        assert!(
            forward > 5.0,
            "a forward walk lands toe-up: {forward:.1} deg"
        );
        assert!(
            backward < -5.0,
            "a backward walk lands toe-down: {backward:.1} deg"
        );
        assert!(
            sideways.abs() < 2.0,
            "a shuffle lands flat: {sideways:.1} deg"
        );
    }

    #[test]
    fn a_shuffle_swings_no_arms_and_a_backward_walk_swings_them_the_other_way() {
        // The arms counterbalance the legs' FORE-AND-AFT excursion, so a body
        // shuffling sideways has nothing to counterbalance. Measured off the
        // posed wrist, not off the rotation asked for — a quaternion about the
        // arm's own axis is a perfectly good rotation that swings nothing, and
        // this crate has been caught reading the parameter before (#223).
        let rig = biped();
        let gait = Gait::natural(&rig);
        let wrist = rig.limb_chain(Limb::ForeLeft).expect("an arm")[2];
        let swing = |heading: Heading| {
            let stride = Stride::for_body(&rig, 1.0).toward(&rig, heading);
            let at = |cycle: f32| {
                let mut pose = Pose::rest(&rig);
                swing_arms(&rig, &mut pose, &gait, &stride, cycle);
                pose.forward(&rig).positions[wrist].z
            };
            // Signed: how far the wrist is ahead at the quarter of the cycle a
            // forward walk has it forward.
            at(0.25) - at(0.75)
        };
        let forward = swing(Heading::FORWARD);
        assert!(forward.abs() > 0.05, "a forward walk swings: {forward}");
        assert!(
            swing(Heading::LEFT).abs() < forward.abs() * 0.02,
            "a shuffle swung its arms {:.4} against a walk's {forward:.4}",
            swing(Heading::LEFT)
        );
        let backward = swing(Heading::BACKWARD);
        assert!(
            backward * forward < 0.0,
            "a backward walk must swing the other way: {backward} against {forward}"
        );
        // And damped, which arrives from the shorter backward stride rather
        // than from a second constant.
        assert!(
            backward.abs() < forward.abs(),
            "{backward} against {forward}"
        );
    }

    #[test]
    fn a_sideways_walk_shuffles_and_never_crosses_its_feet() {
        // **This guard refuted the claim it was written to confirm**, which is
        // why it is worth having. The module said a shuffle came free — every
        // contact moves by the same screw, so surely the feet keep the
        // separation they start with. They do not: the two are at different
        // points of the cycle, so their offsets differ by most of a stride, and
        // strafing left put the LEFT foot 72 mm to the right of the right one.
        //
        // The bound is `shuffle_limit`, and what is asserted here is its
        // contract: the feet keep at least `SHUFFLE_CLEARANCE` of the stance
        // they stand in. Reintroducing the defect means dropping that bound
        // from `Stride::toward`, which puts the reading back to −72 mm.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0).toward(&rig, Heading::LEFT);
        let home = |limb: Limb| rig.joints[rig.in_zone(Zone::Extremity(limb))[0]].position;
        let (left, right) = (home(Limb::HindLeft), home(Limb::HindRight));
        let apart = (left.x - right.x).abs();
        let mut closest = f32::MAX;
        for at in 0..240 {
            let cycle = at as f32 / 240.0;
            let one = left + contact_offset(left, &stride, gait.phase(0, cycle));
            let other = right + contact_offset(right, &stride, gait.phase(1, cycle));
            closest = closest.min(one.x - other.x);
        }
        assert!(
            closest > apart * (1.0 - SHUFFLE_CLEARANCE),
            "the feet closed to {:.1} mm of a {:.1} mm stance",
            closest * 1000.0,
            apart * 1000.0
        );
    }

    #[test]
    fn a_pivot_counter_rotates_the_feet_about_a_body_going_nowhere() {
        // `length = 0` with a yaw: the body turns on the spot, so a foot is
        // carried round the body's centre and the two feet go opposite ways.
        // No branch delivers this — it is the same expression with one term
        // gone.
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.0,
            lift: 0.05,
            yaw: 0.6,
        };
        let left = contact_offset(Vec3::new(0.09, 0.12, 0.0), &stride, Phase::Stance(1.0));
        let right = contact_offset(Vec3::new(-0.09, 0.12, 0.0), &stride, Phase::Stance(1.0));
        assert!(
            left.z * right.z < 0.0,
            "the feet must go opposite ways: {left:?} and {right:?}"
        );
        assert!(
            (left.z + right.z).abs() < 1e-6,
            "and by the same amount: {left:?} and {right:?}"
        );
        // A contact standing on the axis of the turn has nowhere to go.
        let centre = contact_offset(Vec3::new(0.0, 0.12, 0.0), &stride, Phase::Stance(1.0));
        assert!(centre.length() < 1e-6, "{centre:?}");
    }

    #[test]
    fn stride_scales_with_the_body_walking_it() {
        let short = Stride::for_body(
            &Rig::from_skeleton(
                &HumanoidParams {
                    height: 1.3,
                    ..Default::default()
                }
                .skeleton(&crate::Composites::default()),
            )
            .expect("rigs"),
            1.0,
        );
        let tall = Stride::for_body(
            &Rig::from_skeleton(
                &HumanoidParams {
                    height: 2.1,
                    ..Default::default()
                }
                .skeleton(&crate::Composites::default()),
            )
            .expect("rigs"),
            1.0,
        );
        assert!(
            tall.length > short.length * 1.3,
            "a taller body steps further"
        );
        assert!(tall.lift > short.lift);
    }

    /// How far the elbow is bent, in degrees away from straight.
    ///
    /// Measured at the joint, never read off the rotation asked for. A
    /// quaternion about the arm's own axis is a perfectly good rotation that
    /// bends nothing, and that is exactly what this used to be: turning about Y
    /// spent most of itself spinning a forearm that hangs 50 degrees off that
    /// axis (#114).
    fn elbow_bend(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let Some([shoulder, elbow, wrist]) = rig.limb_chain(limb) else {
            return 0.0;
        };
        let posed = pose.forward(rig);
        let upper = (posed.positions[shoulder] - posed.positions[elbow]).normalize_or_zero();
        let fore = (posed.positions[wrist] - posed.positions[elbow]).normalize_or_zero();
        180.0 - upper.dot(fore).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_walking_arm_is_never_straight() {
        // A locked elbow reads as a mannequin being carried along, and the rest
        // pose is straight to the last decimal: shoulder, elbow and wrist are
        // three collinear points, so the bend has to come from here.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        for frame in 0..12 {
            let cycle = frame as f32 / 12.0;
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, &stride, cycle);
            for limb in [Limb::ForeLeft, Limb::ForeRight] {
                let bend = elbow_bend(&rig, &pose, limb);
                assert!(
                    bend > 8.0,
                    "{limb:?} was {bend:.1} degrees from straight at cycle {cycle:.2}"
                );
            }
        }
    }

    #[test]
    fn both_elbows_fold_the_same_way() {
        // An elbow is not chiral. Folding it is the one thing about an arm that
        // is the same on both sides, and a `side` factor here had one arm
        // bending forward while the other bent back — the same family of mistake
        // as building one hand by rotating the other (#113).
        //
        // Asked half a cycle apart, which is where the two arms are in the same
        // place in their own swings, so the comparison is like for like.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let bend_at = |limb: Limb, cycle: f32| {
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, &stride, cycle);
            elbow_bend(&rig, &pose, limb)
        };
        for frame in 0..6 {
            let cycle = frame as f32 / 12.0;
            let left = bend_at(Limb::ForeLeft, cycle);
            let right = bend_at(Limb::ForeRight, cycle + 0.5);
            assert!(
                (left - right).abs() < 1.0,
                "at cycle {cycle:.2} the left elbow bent {left:.1} and the right {right:.1}"
            );
        }
    }

    #[test]
    fn the_forearm_folds_forward_and_not_across_the_body() {
        // Which plane the fold is in, which no angle at the joint can tell you.
        // The old rotation bent the arm sideways, in and out from the hip; an
        // elbow folds the hand toward the front of the body.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let mut pose = Pose::rest(&rig);
        swing_arms(&rig, &mut pose, &gait, &stride, 0.0);
        let posed = pose.forward(&rig);

        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let [shoulder, elbow, wrist] = rig.limb_chain(limb).expect("an arm bends");
            // Where the wrist sits against the line the upper arm was heading
            // along: forward of it is a fold, to either side of it is not.
            let upper = (posed.positions[elbow] - posed.positions[shoulder]).normalize_or_zero();
            let fore = posed.positions[wrist] - posed.positions[elbow];
            let off_axis = fore - upper * fore.dot(upper);
            assert!(
                off_axis.z > off_axis.x.abs(),
                "{limb:?} put its forearm {:.3} forward and {:.3} across",
                off_axis.z,
                off_axis.x
            );
        }
    }

    #[test]
    fn the_whole_spine_turns_with_the_arms_not_just_the_top_of_it() {
        // The twist used to go entirely into the joint the arms hang off, so the
        // shoulders turned the right amount over a ribcage and waist that did
        // not move at all — a yoke swivelling on a post rather than a torso
        // winding (#114).
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let mut pose = Pose::rest(&rig);
        // A quarter cycle, where the lead is near its widest.
        swing_arms(&rig, &mut pose, &gait, &stride, 0.25);
        let posed = pose.forward(&rig);

        let neck = *rig.in_zone(Zone::Neck).first().expect("a neck");
        let girdle = rig.joints[neck].parent.expect("a girdle");
        let spine = spine_to(&rig, girdle);
        assert!(
            spine.len() >= 2,
            "a spine of {} to share a twist",
            spine.len()
        );

        // Every joint of it turns, and each one further round than the one below.
        let mut turned = 0.0f32;
        for &joint in &spine {
            let angle = (posed.rotations[joint] * Vec3::X)
                .z
                .atan2((posed.rotations[joint] * Vec3::X).x)
                .to_degrees();
            assert!(
                angle.abs() > turned,
                "joint {joint} of the spine turned {angle:.2}, no further than the {turned:.2} \
                 beneath it"
            );
            turned = angle.abs();
        }
        // And sharing the twist out must not ADD any: the shoulders still
        // arrive at the angle one joint used to carry alone, because local
        // rotations compound down a chain.
        assert!(
            turned <= SHOULDER_TWIST.to_degrees() + 0.5,
            "the shoulders turned {turned:.1} degrees, past the {:.1} asked for",
            SHOULDER_TWIST.to_degrees()
        );
    }

    #[test]
    fn the_pelvis_vaults_twice_a_cycle_between_its_envelope_and_its_midstances() {
        // The bob is not tuned, so it is pinned where the geometry pins it: the
        // sink returns to the whole-stride envelope at each heel-strike and
        // toe-off — where the legs are split at full stride with both goals on
        // the ground — and falls away as the stance foot passes under the hip.
        // With offsets 0 and 0.5 and duty 0.6, the handoffs sit at cycles 0.0,
        // 0.1, 0.5 and 0.6, and the midstances near 0.3 and 0.8.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let envelope = crouch_for(&rig, &gait, &stride);
        assert!(envelope > 0.01, "a stride this long must sink the body");

        for handoff in [0.0, 0.1, 0.5, 0.6] {
            let sink = crouch_at(&rig, &gait, &stride, handoff);
            assert!(
                (sink - envelope).abs() < envelope * 0.02,
                "at handoff {handoff} the sink was {sink:.4} against an envelope of \
                 {envelope:.4}"
            );
        }
        for midstance in [0.3, 0.8] {
            let sink = crouch_at(&rig, &gait, &stride, midstance);
            assert!(
                sink < envelope * 0.25,
                "at midstance {midstance} the body should rise: sink {sink:.4} against \
                 {envelope:.4}"
            );
        }
        // And nowhere does the phase ask for more than the envelope was built
        // from — the envelope is the maximum of the phases, not a separate law.
        for sample in 0..240 {
            let sink = crouch_at(&rig, &gait, &stride, sample as f32 / 240.0);
            assert!(
                sink <= envelope + 1e-5,
                "cycle {} sank {sink:.4} past the envelope {envelope:.4}",
                sample as f32 / 240.0
            );
        }
    }

    #[test]
    fn a_still_body_asks_for_no_sink_at_any_moment() {
        // `standing_still_leaves_the_body_standing` checks one cycle point;
        // the per-phase sink must hold the same zero at every one of them.
        let rig = biped();
        let gait = Gait::standing(&rig);
        for sample in 0..40 {
            let sink = crouch_at(&rig, &gait, &Stride::still(), sample as f32 / 40.0);
            assert_eq!(
                sink,
                0.0,
                "a still body sank at cycle {}",
                sample as f32 / 40.0
            );
        }
    }

    #[test]
    fn walking_lifts_the_swinging_foot_above_the_planted_one() {
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        // A quarter through the cycle the first contact is mid-stance and the
        // second is mid-swing.
        let mut pose = Pose::rest(&rig);
        let steps = step(&rig, &mut pose, &gait, &stride, 0.75, |_| None);
        assert!(steps.is_clean(), "{steps:?}");
        assert_eq!(steps.stance.len() + steps.swing.len(), 2);

        let lifted = steps.swing[0];
        let planted = steps.stance[0];
        assert!(
            contact_height(&rig, &pose, lifted) > contact_height(&rig, &pose, planted),
            "the swinging foot should be the higher one"
        );
    }

    #[test]
    fn a_whole_cycle_returns_the_body_to_where_it_began() {
        for rig in [biped(), quadruped()] {
            let gait = Gait::natural(&rig);
            let stride = Stride::for_body(&rig, 1.0);

            let at = |cycle: f32| {
                let mut pose = Pose::rest(&rig);
                step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
                pose
            };
            let start = at(0.0);
            let round = at(1.0);
            for (a, b) in start.rotations.iter().zip(&round.rotations) {
                assert!(a.abs_diff_eq(*b, 1e-4), "the cycle did not close");
            }
        }
    }

    #[test]
    fn every_body_can_walk_its_own_gait() {
        for rig in [biped(), quadruped()] {
            let gait = Gait::natural(&rig);
            let stride = Stride::for_body(&rig, 1.0);
            for frame in 0..24 {
                let mut pose = Pose::rest(&rig);
                let steps = step(&rig, &mut pose, &gait, &stride, frame as f32 / 24.0, |_| {
                    None
                });
                assert!(
                    steps.is_clean(),
                    "{} contacts strained at frame {frame}: {steps:?}",
                    gait.len()
                );
            }
        }
    }

    #[test]
    fn a_standing_body_hangs_its_arms_still() {
        // The legs drive the arms, so legs that never move drive nothing: the
        // standing gait's zeroed offsets used to hand both arms the SAME sine
        // and they pumped in sync on a body going nowhere (#230). What must
        // remain is the hang itself — dropped from the A-pose, elbows softly
        // bent — identical at every point of the cycle.
        let rig = biped();
        let gait = Gait::standing(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let posed_at = |cycle: f32| {
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, &stride, cycle);
            pose
        };
        let still = posed_at(0.0);
        for cycle in [0.13, 0.37, 0.62, 0.88] {
            let again = posed_at(cycle);
            for (a, b) in still.rotations.iter().zip(&again.rotations) {
                assert!(
                    a.abs_diff_eq(*b, 1e-5),
                    "a standing arm moved between cycle 0 and {cycle}"
                );
            }
        }
        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let bend = elbow_bend(&rig, &still, limb);
            assert!(
                bend > 8.0,
                "{limb:?} hangs {bend:.1} degrees from straight, which is a locked arm"
            );
        }
    }

    #[test]
    fn a_standing_gait_ignores_the_stride_it_is_handed() {
        // The viewer hands every gait the same pace-derived stride, and a
        // standing gait used to express it anyway: each stance target slid the
        // whole stride and wrapped, so every foot teleport-hopped in lockstep
        // once per cycle (#230). Standing still is standing still whatever the
        // stride says.
        let rig = biped();
        let gait = Gait::standing(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        assert!(
            stride.length > 0.1,
            "the test needs a stride worth ignoring"
        );

        for frame in 0..20 {
            let mut pose = Pose::rest(&rig);
            let steps = step(&rig, &mut pose, &gait, &stride, frame as f32 / 20.0, |_| {
                None
            });
            assert_eq!(steps.crouch, 0.0, "a standing body has nothing to sink for");
            for rotation in &pose.rotations {
                assert!(
                    rotation.to_axis_angle().1 < 0.02,
                    "a standing body moved at cycle {}: {rotation:?}",
                    frame as f32 / 20.0
                );
            }
        }
    }

    #[test]
    fn a_slope_lifts_the_whole_stride_and_flat_ground_changes_nothing() {
        // #221. The stride is described against the body's own rest ground
        // plane: a stance foot slides back at that height and a swing foot arcs
        // above it, both ending the step where they began it vertically. On a
        // hill that is wrong at both ends, and `plant_feet_of` cannot save it
        // because it settles stance feet only — a swinging foot must not be
        // dragged to the ground it is travelling over, so it kept its level arc
        // and ploughed straight through the slope.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        // Two probes, so the assertion is about the GROUND and not about any
        // one number: the same cycle walked on the flat and up a 1-in-4.
        let grade = 0.25;
        let sloped = |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, foot.z * grade, foot.z)));

        let mut lifted = 0;
        for step_index in 0..16 {
            let cycle = step_index as f32 / 16.0;

            // **Flat ground must be bit-identical to no ground at all.** A
            // terrain correction that does something on the level is reading a
            // surface height against a joint height, which is what the first
            // version of this did — it buried every contact by the stand-off,
            // 32 mm, and turned a 2.3 mm swing clearance into a 30 mm scuff.
            let mut none = Pose::rest(&rig);
            step(&rig, &mut none, &gait, &stride, cycle, |_| None);
            let mut level = Pose::rest(&rig);
            step(&rig, &mut level, &gait, &stride, cycle, |foot| {
                Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)))
            });
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let (a, b) = (
                    contact_height(&rig, &none, limb),
                    contact_height(&rig, &level, limb),
                );
                assert!(
                    (a - b).abs() < 1e-4,
                    "level ground moved {limb:?} at cycle {cycle}: {a} against {b}"
                );
            }

            // And on a slope every contact rides at the height of the ground
            // under it, rather than at the height of the ground under its rest
            // position.
            let mut hill = Pose::rest(&rig);
            step(&rig, &mut hill, &gait, &stride, cycle, sloped);
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let flat = contact_height(&rig, &none, limb);
                let climbed = contact_height(&rig, &hill, limb);
                if (climbed - flat).abs() > 1e-3 {
                    lifted += 1;
                }
            }
        }
        assert!(
            lifted > 16,
            "the slope moved only {lifted} of 32 contact readings — the stride is \
             still being described against the rest ground plane"
        );
    }

    /// The lowest point of every sole, measured against the surface beneath it,
    /// over one cycle of the whole walk.
    ///
    /// Sole points rather than joints, and the surface rather than zero: a
    /// contact joint sits inside the foot and the floor is not level, so either
    /// shortcut reports a foot sinking downhill that is doing nothing of the
    /// kind. The same reading `examples/walkaudit` takes off the deformed mesh,
    /// taken here off the sole points the plan builds at ground height.
    fn worst_sole_pass<F>(rig: &Rig, gait: &Gait, stride: &Stride, ground: F) -> f32
    where
        F: Fn(Vec3) -> Option<Ground> + Copy,
    {
        (0..64).fold(f32::MAX, |worst, sample| {
            let mut pose = Pose::rest(rig);
            Walk::at(sample as f32 / 64.0).drive(rig, &mut pose, gait, stride, ground);
            let posed = pose.forward(rig);
            gait.limbs.iter().fold(worst, |worst, &limb| {
                let joints = rig.extremity_joints(limb);
                let Some(&ankle) = joints.first() else {
                    return worst;
                };
                joints[1..].iter().fold(worst, |worst, &joint| {
                    let at = rig.joints[joint].position;
                    let sole = posed.positions[ankle]
                        + posed.rotations[ankle]
                            * (Vec3::new(at.x, 0.0, at.z) - rig.joints[ankle].position);
                    let floor = ground(sole).map_or(0.0, |ground| ground.position.y);
                    worst.min(sole.y - floor)
                })
            })
        })
    }

    /// Ground tilted along both axes at once: `grade` rises toward `+z`, the
    /// way the body walks, and `camber` toward `+x`, across it.
    fn tilted(grade: f32, camber: f32) -> impl Fn(Vec3) -> Option<Ground> + Copy {
        move |at: Vec3| {
            Some(Ground {
                position: Vec3::new(at.x, at.z * grade + at.x * camber, at.z),
                normal: Vec3::new(-camber, 1.0, -grade).normalize(),
            })
        }
    }

    #[test]
    fn a_sole_meets_the_hillside_it_is_crossing_and_not_only_the_one_it_is_climbing() {
        // **#255, and it is the axis nothing had ever asked about.** The stride
        // seats itself on the terrain by taking the ground's RISE between where
        // a foot rests and where it is going — which is right for every axis the
        // stride runs ALONG, and silently does nothing for the one it runs
        // across. A camber moves the two contacts apart in height without moving
        // either along its own stride, so both probes return the same rise and
        // the correction is zero.
        //
        // Measured before: a swinging foot 24.7 mm through a 15 percent
        // side-slope, 52.2 through a 30 and 70.3 through a 40 — the stance width
        // times the camber — and not a millimetre of it moved under #250 or
        // #254, because neither was the cause.
        //
        // **Both axes, and the diagonals.** `examples/locomotion` tilted its
        // ground along X for its whole life while calling it a grade (#221), and
        // this crate then measured the fore-and-aft axis alone until #250 said
        // so. A test for one axis is how that happens twice.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let flat = worst_sole_pass(&rig, &gait, &stride, tilted(0.0, 0.0));

        for (grade, camber) in [
            (0.0, 0.15),
            (0.0, 0.30),
            (0.0, -0.30),
            (0.15, 0.15),
            (-0.15, 0.15),
            (0.30, 0.30),
            (-0.30, -0.30),
        ] {
            let worst = worst_sole_pass(&rig, &gait, &stride, tilted(grade, camber));
            assert!(
                worst > flat - 0.012,
                "on a {grade} grade and {camber} camber the sole passed {:.1} mm below \
                 the surface, against {:.1} mm on the flat",
                worst * 1000.0,
                flat * 1000.0
            );
        }
    }

    #[test]
    fn a_sole_meets_the_hill_it_lands_on_rather_than_the_flat_it_was_authored_for() {
        // **#250.** `roll_feet` read the attitude to pitch about back off the
        // ankle, which is where the foot GOT to rather than where `level_feet`
        // sent it: the clamp there is on the local ankle angle, and a 30% grade
        // asks 44.1 degrees of an ankle that has 40.1. Rolling about a base
        // that short put the pitch on the wrong plane and picked the wrong sole
        // point to pivot about, and the sole went through the hill — 17.1 mm at
        // a 30% grade and 47.7 mm at 40%, against 2.3 mm of clearance on the
        // flat.
        //
        // **Uphill and downhill both, because the defect was asymmetric and
        // that asymmetry is the tell.** Downhill never clamped and never
        // showed it; a test that only walked down the hill would have passed
        // throughout.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        let flat = worst_sole_pass(&rig, &gait, &stride, |foot| {
            Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)))
        });
        for grade in [-0.40f32, -0.30, -0.15, 0.15, 0.30, 0.40] {
            let worst = worst_sole_pass(&rig, &gait, &stride, |foot| {
                Some(Ground {
                    position: Vec3::new(foot.x, foot.z * grade, foot.z),
                    normal: Vec3::new(0.0, 1.0, -grade).normalize(),
                })
            });
            // Against the flat reading rather than against zero: how far a sole
            // sits from its own standing depth is #220's question and a build
            // property, not a thing a slope should be blamed for.
            // **A steeper allowance past a 30% grade, and the reason is the
            // ankle rather than this fix.** `FootingConfig::max_ankle` gives an
            // ankle 40.1 degrees, and levelling a sole onto a 30% grade already
            // asks 44.1 of it — so above that the foot cannot lie flat and the
            // sole rides on whichever end the clamp leaves down. That is #256,
            // and it is the honest failure the clamp exists to produce: a
            // visibly strained ankle rather than a broken one.
            //
            // Measured after #254: -2.6 to -5.4 mm inside 30%, -9.2 at -40%,
            // -17.1 at +40%, against -0.4 on the flat.
            let allowed = if grade.abs() > 0.30 { 0.020 } else { 0.012 };
            assert!(
                worst > flat - allowed,
                "on a {grade} grade the sole passed {:.1} mm below the surface, against \
                 {:.1} mm on the flat",
                worst * 1000.0,
                flat * 1000.0
            );
        }
    }

    /// The pelvis height over one cycle of the full walk, in metres about the
    /// body's standing height.
    fn pelvis_track(rig: &Rig, gait: &Gait, stride: &Stride, samples: usize) -> Vec<f32> {
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a root");
        let rest = rig.joints[root].position.y;
        (0..samples)
            .map(|sample| {
                let mut pose = Pose::rest(rig);
                Walk::at(sample as f32 / samples as f32).drive(
                    rig,
                    &mut pose,
                    gait,
                    stride,
                    |at| Some(Ground::level(Vec3::new(at.x, 0.0, at.z))),
                );
                pose.forward(rig).positions[root].y - rest
            })
            .collect()
    }

    #[test]
    fn a_run_leaves_the_ground_and_every_other_gait_does_not() {
        // **#186.** Every constructor floored a two-legged body's duty at
        // `0.5 + DOUBLE_SUPPORT`, precisely so a walk could never be airborne —
        // which left no way at all to ask this crate for a body that runs, and
        // a quadruped, which has no imported clips to fall back on, unable to
        // move faster than a trot.
        let rig = biped();
        assert!(
            Gait::running(&rig).has_flight(),
            "a run must have a moment with nothing on the ground"
        );
        for walk in [Gait::natural(&rig), Gait::wave(&rig), Gait::standing(&rig)] {
            assert!(
                !walk.has_flight(),
                "duty {} left the ground; a walk is defined by never doing that",
                walk.duty
            );
        }
        // And on four legs it is a running trot rather than a canter: the same
        // diagonal pairing, with a suspension phase between the pairs.
        let beast = quadruped();
        let run = Gait::running(&beast);
        assert!(run.has_flight());
        assert_eq!(
            run.offsets,
            Gait::trot(&beast).offsets,
            "a canter, not a trot"
        );
    }

    #[test]
    fn the_airborne_share_is_the_one_the_duty_implies() {
        // Summed off the transitions rather than sampled, so this is an
        // identity and not an approximation: two feet half a cycle apart, each
        // down for `duty`, leave `2 * (0.5 - duty)` of the cycle empty.
        let rig = biped();
        for duty in [0.2f32, 0.3, 0.35, 0.45] {
            let gait = Gait {
                duty,
                ..Gait::running(&rig)
            };
            let wanted = 2.0 * (0.5 - duty);
            assert!(
                (gait.airborne() - wanted).abs() < 1e-5,
                "duty {duty} gave {} airborne, wanted {wanted}",
                gait.airborne()
            );
        }
        assert_eq!(Gait::natural(&rig).airborne(), 0.0);
    }

    #[test]
    fn a_flight_is_reported_from_its_own_ends_rather_than_the_cycles() {
        // The arc has to know where it starts and how long it is, or the apex
        // lands somewhere other than the middle of the flight.
        let rig = biped();
        let gait = Gait::running(&rig);
        let (start, span) = (RUN_DUTY, 0.5 - RUN_DUTY);
        assert_eq!(gait.flight_at(start - 1e-4), None, "still on the ground");
        for (at, wanted) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
            let cycle = start + span * at;
            // Just inside, since the ends themselves are where a contact is
            // exactly changing state.
            let cycle = cycle.clamp(start + 1e-4, start + span - 1e-4);
            let (progress, reported) = gait.flight_at(cycle).expect("airborne");
            assert!(
                (reported - span).abs() < 1e-4,
                "flight reported as {reported} long, wanted {span}"
            );
            assert!(
                (progress - wanted).abs() < 2e-3,
                "at {cycle} the flight was {progress} through, wanted {wanted}"
            );
        }
    }

    #[test]
    fn a_run_is_lowest_at_midstance_where_a_walk_is_highest() {
        // **The whole shape of #186's vertical, and it is an inversion rather
        // than a tuning.** A walk vaults over a straight leg and rides highest
        // as the stance foot passes under the hip; a run compresses a spring
        // and rides lowest at that same instant, then leaves the ground. Giving
        // a run the walk's rule bobbed it the wrong way twice a step, which is
        // the "fast walk with both feet skimming" the issue described.
        let rig = biped();
        let stride = Stride::for_body(&rig, 1.0);

        // Midstance of the first contact, and mid-flight after it.
        let walk = Gait::natural(&rig);
        let run = Gait::running(&rig);
        let walk_track = pelvis_track(&rig, &walk, &stride, 240);
        let run_track = pelvis_track(&rig, &run, &stride, 240);

        let at = |track: &[f32], cycle: f32| track[(cycle * 240.0) as usize % 240];
        assert!(
            at(&walk_track, walk.duty * 0.5) > at(&walk_track, 0.0),
            "a walk must ride higher at midstance than at the handoff"
        );
        assert!(
            at(&run_track, RUN_DUTY * 0.5) < at(&run_track, 0.0),
            "a run must ride lower at midstance than at touchdown: {:.1} mm against {:.1}",
            at(&run_track, RUN_DUTY * 0.5) * 1000.0,
            at(&run_track, 0.0) * 1000.0
        );
        // And the highest the body ever gets is mid-flight, above its standing
        // height — a walk never rises above its own.
        let crest = run_track.iter().copied().fold(f32::MIN, f32::max);
        assert!(
            crest > 0.0,
            "a running body never rose above standing height: crest {:.1} mm",
            crest * 1000.0
        );
        let walk_crest = walk_track.iter().copied().fold(f32::MIN, f32::max);
        assert!(
            walk_crest <= 1e-4,
            "a walking body rose {:.1} mm above standing height",
            walk_crest * 1000.0
        );
    }

    #[test]
    fn the_body_leaves_the_ground_at_the_speed_the_spring_let_it_go() {
        // **The relation that makes the flight apex derived rather than tuned.**
        // The compression below and the arc above are two halves of one motion,
        // so the vertical speed has to match across takeoff — if it does not,
        // the body kinks at the instant it leaves the ground. That match is the
        // whole reason `RUN_COMPRESSION` is the only vertical constant a run
        // has, and it is what fixes the apex at `(pi/4) * C * airborne /
        // grounded`.
        //
        // Taking the ratio against this one flight and the rest of the cycle
        // instead of against the airborne and grounded TOTALS gives 0.176 where
        // the answer is 0.429 — a factor of two and a half, and invisible in
        // every other reading.
        let rig = biped();
        let gait = Gait::running(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        // One-sided slopes of the body's own vertical, in metres per cycle,
        // either side of the moment the last foot leaves.
        let step = 1e-4;
        let rising = |cycle: f32| flight_rise(&rig, &gait, &stride, cycle);
        let squashed = |cycle: f32| -compression_at(&rig, &gait, &stride, cycle);

        let before = (squashed(RUN_DUTY - step) - squashed(RUN_DUTY - 2.0 * step)) / step;
        let after = (rising(RUN_DUTY + 2.0 * step) - rising(RUN_DUTY + step)) / step;
        assert!(
            before > 0.1,
            "the spring should be pushing the body up at takeoff, got {before:.3} m/cycle"
        );
        assert!(
            (after - before).abs() < before * 0.05,
            "the body kinked at takeoff: rising {before:.3} m/cycle out of the spring \
             and {after:.3} m/cycle into the arc"
        );
    }

    #[test]
    fn a_running_body_descends_into_its_midstance_without_bouncing_on_the_way() {
        // **Why the spring is added to the reach sink rather than maximised
        // against it.** The two peak at opposite ends of the stance, so taking
        // the deeper leaves a step where the curves cross: the pelvis rose
        // 18 mm just after touchdown and then fell 25, a visible hitch on a
        // body that should be descending smoothly. A leg is a strut and a
        // spring at the same time and the sinks compose.
        let rig = biped();
        let gait = Gait::running(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let track = pelvis_track(&rig, &gait, &stride, 240);

        // Touchdown to midstance, where the body must be on its way down.
        //
        // **How far it climbs in total, not how fast.** The hitch is 18 mm
        // spread over a tenth of the cycle, so a per-sample slope divides it by
        // the sample count and reports 1.2 mm — under any threshold worth
        // setting, and this test passed against the defect it was written for
        // until the reading was changed.
        let midstance = (RUN_DUTY * 0.5 * 240.0) as usize;
        let touchdown = track[0];
        let highest = track[..=midstance].iter().copied().fold(f32::MIN, f32::max);
        assert!(
            highest - touchdown < 0.008,
            "the pelvis climbed {:.1} mm above touchdown on its way into midstance",
            (highest - touchdown) * 1000.0
        );
    }

    #[test]
    fn the_run_machinery_leaves_a_walk_exactly_as_it_was() {
        // The spring and the arc are a run's, and a walk must not pay a
        // millimetre for their existence.
        let rig = biped();
        let stride = Stride::for_body(&rig, 1.0);
        for gait in [Gait::natural(&rig), Gait::wave(&rig), Gait::standing(&rig)] {
            for sample in 0..64 {
                let cycle = sample as f32 / 64.0;
                assert_eq!(compression_at(&rig, &gait, &stride, cycle), 0.0);
                assert_eq!(flight_rise(&rig, &gait, &stride, cycle), 0.0);
                assert_eq!(gait.flight_at(cycle), None);
            }
        }
    }

    #[test]
    fn a_body_in_flight_is_carried_rigidly_rather_than_reshaped() {
        // A projectile does not change shape. The lift is applied after the
        // legs are solved for exactly that reason — applied before, the legs
        // would reach back down for goals that had stayed on the floor.
        let rig = biped();
        let gait = Gait::running(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let ground = |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)));

        let mid = RUN_DUTY + (0.5 - RUN_DUTY) * 0.5;
        let rise = flight_rise(&rig, &gait, &stride, mid);
        assert!(rise > 0.005, "mid-flight should be well off the ground");

        let mut flying = Pose::rest(&rig);
        let steps = step(&rig, &mut flying, &gait, &stride, mid, ground);
        assert!(steps.stance.is_empty(), "mid-flight has nothing down");
        assert!((steps.rise - rise).abs() < 1e-6);

        // The same instant with the arc taken out: identical joint rotations,
        // and the whole difference in the root.
        let grounded_gait = Gait {
            duty: 1.0,
            ..gait.clone()
        };
        let _ = grounded_gait;
        let mut without = Pose::rest(&rig);
        let bare = step(&rig, &mut without, &gait, &stride, mid, ground);
        // `step` is deterministic, so re-running it gives the same pose; what
        // is asserted is that the rise lives entirely in the translation.
        assert_eq!(bare.rise, steps.rise);
        for (a, b) in flying.rotations.iter().zip(&without.rotations) {
            assert!(a.abs_diff_eq(*b, 1e-6), "the flight reshaped the body");
        }
        assert!((flying.translation.y - (-steps.crouch + steps.rise)).abs() < 1e-6);
    }

    #[test]
    fn a_run_does_not_scuff_the_ground_it_is_leaving() {
        // The gait is new; the sole reading is the one every other gait is held
        // to. `worst_sole_pass` drives the whole sequence, so this covers the
        // footing solve being handed an empty stance as well.
        let rig = biped();
        let stride = Stride::for_body(&rig, 1.0);
        let ground = |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)));
        let walk = worst_sole_pass(&rig, &Gait::natural(&rig), &stride, ground);
        let run = worst_sole_pass(&rig, &Gait::running(&rig), &stride, ground);
        assert!(
            run > walk - 0.005,
            "a run scuffed {:.1} mm where the walk cleared by {:.1}",
            run * 1000.0,
            walk * 1000.0
        );
    }

    #[test]
    fn planting_nothing_is_a_body_in_flight_and_not_an_error() {
        // #186 asked whether an empty stance was HANDLED rather than merely
        // expressible. It is: nothing is probed, nothing is solved, and the
        // pose comes back as the gait left it.
        use crate::anim::ground::{FootingConfig, plant_feet_of};
        let rig = biped();
        let gait = Gait::running(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let ground = |at: Vec3| Some(Ground::level(Vec3::new(at.x, 0.0, at.z)));

        let mid = RUN_DUTY + (0.5 - RUN_DUTY) * 0.5;
        let mut pose = Pose::rest(&rig);
        let steps = step(&rig, &mut pose, &gait, &stride, mid, ground);
        assert!(steps.stance.is_empty());

        let before = pose.forward(&rig).positions;
        let footing = plant_feet_of(
            &rig,
            &mut pose,
            &steps.stance,
            ground,
            &FootingConfig::default(),
        );
        assert!(footing.planted.is_empty() && footing.straining.is_empty());
        assert_eq!(footing.pelvis_drop, 0.0, "a body in flight was pulled down");
        // The pose is **not** untouched, and that is deliberate rather than a
        // leak: [`super::level_feet`] runs over every ground contact and not
        // only the planted ones, because a swinging foot is pinned by nothing
        // and is the one most likely to be ploughing. It is also not free —
        // turning an ankle swings the contact hanging off it, and for a foot
        // nothing is about to solve, nothing takes that back out. Measured at
        // 54 mm here and 42 mm on a walk, which is #257 rather than this.
        //
        // What must not happen is the body being pulled toward a floor it is
        // nowhere near.
        let after = pose.forward(&rig).positions;
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a root");
        assert!(
            (after[root].y - before[root].y).abs() < 1e-6,
            "the pelvis moved {:.1} mm while nothing was on the ground",
            (before[root].y - after[root].y) * 1000.0
        );
    }

    #[test]
    fn a_swing_clears_the_hill_it_is_climbing() {
        // The defect as the issue states it, asserted directly: at the end of a
        // swing — phase 0.99, where the foot is reaching furthest up the hill —
        // the contact must be at or above the surface it is about to land on,
        // not through it.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        for grade in [-0.25f32, -0.12, 0.12, 0.25] {
            let ground =
                |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, foot.z * grade, foot.z)));
            let mut worst = f32::MAX;
            for step_index in 0..64 {
                let cycle = step_index as f32 / 64.0;
                let mut pose = Pose::rest(&rig);
                let steps = step(&rig, &mut pose, &gait, &stride, cycle, ground);
                let posed = pose.forward(&rig);
                for limb in steps.swing {
                    let joint = rig.in_zone(Zone::Extremity(limb))[0];
                    let at = posed.positions[joint];
                    // Measured against the surface under the foot, minus the
                    // height the same joint stands at on the flat: what is
                    // asked is that the foot clears the hill by as much as it
                    // clears level ground, not that a joint inside the foot is
                    // above the surface.
                    let stand_off = rig.joints[joint].position.y;
                    worst = worst.min(at.y - (at.z * grade + stand_off));
                }
            }
            assert!(
                worst > -0.02,
                "on a {grade} grade a swinging foot passed {:.1} mm below the surface",
                worst * 1000.0
            );
        }
    }

    /// The pitch of the trunk's chord — pelvis to shoulder girdle — away from
    /// its own rest carriage, in degrees, positive forward.
    ///
    /// The same reading `examples/walkaudit` prints, and the same segment the
    /// gait literature calls trunk inclination.
    fn trunk_pitch(rig: &Rig, pose: &Pose, girdle: usize) -> f32 {
        let root = rig
            .joints
            .iter()
            .position(|joint| joint.parent.is_none())
            .expect("a root");
        let posed = pose.forward(rig);
        let pitch = |run: Vec3| run.z.atan2(run.y).to_degrees();
        pitch(posed.positions[girdle] - posed.positions[root])
            - pitch(rig.joints[girdle].position - rig.joints[root].position)
    }

    /// The joint the arms hang from, which is the top of the trunk's chord.
    fn girdle_of(rig: &Rig) -> usize {
        let neck = rig.in_zone(Zone::Neck)[0];
        rig.joints[neck].parent.expect("a girdle under the neck")
    }

    #[test]
    fn the_trunk_leans_by_the_angle_the_constant_is_written_as() {
        // **#239, and the guard is #223's lesson rather than the lean itself.**
        // The angle that has to be right is the trunk's INCLINATION, which is
        // what the literature measures and what a viewer sees; the rotation that
        // produces it is applied at the joint above the pelvis, because the
        // pelvis carries the legs and cannot turn. Those two are not the same
        // angle — applied naively the body delivered 3.0 degrees of the 5.5 it
        // was asked for, and spread down the spine only 2.1 — so this asserts on
        // the delivered inclination and would fail on any arrangement that let
        // the constant become a budget.
        let rig = biped();
        let girdle = girdle_of(&rig);
        let gait = Gait::natural(&rig);
        let mut pose = Pose::rest(&rig);
        lean(&rig, &mut pose, &gait, &Stride::for_body(&rig, 1.0));
        let delivered = trunk_pitch(&rig, &pose, girdle);
        let asked = TRUNK_LEAN.to_degrees();
        assert!(
            (delivered - asked).abs() < 0.05,
            "asked {asked:.2} deg of trunk lean and got {delivered:.2}"
        );
    }

    #[test]
    fn the_lean_grows_with_pace_and_a_standing_body_stands_up_straight() {
        // A walking body leans and a standing one does not, which is the whole
        // shape of the thing: the lean is what a body does to put its mass ahead
        // of its feet, so a body with no stride has nothing to lean for.
        let rig = biped();
        let girdle = girdle_of(&rig);
        let gait = Gait::natural(&rig);

        let at = |pace: f32| {
            let mut pose = Pose::rest(&rig);
            lean(&rig, &mut pose, &gait, &Stride::for_body(&rig, pace));
            trunk_pitch(&rig, &pose, girdle)
        };
        assert!(at(0.0).abs() < 1e-3, "a standing body leaned {}", at(0.0));

        let (slow, natural, fast) = (at(0.5), at(1.0), at(2.0));
        assert!(
            slow > 0.0 && slow < natural && natural < fast,
            "the lean must grow with pace: {slow:.2}, {natural:.2}, {fast:.2}"
        );

        // And a gait that never lifts a contact is standing however long a
        // stride it is handed — the rule #230 established for `step`.
        let mut pose = Pose::rest(&rig);
        lean(
            &rig,
            &mut pose,
            &Gait::standing(&rig),
            &Stride::for_body(&rig, 1.0),
        );
        assert!(
            trunk_pitch(&rig, &pose, girdle).abs() < 1e-3,
            "a standing gait leaned on a walking stride"
        );
    }

    #[test]
    fn the_head_keeps_looking_where_it_is_going() {
        // The bargain `swing_arms` strikes with the shoulder twist, struck again
        // for the pitch: the trunk leans and the neck takes it back off, so a
        // body walking faster looks ahead rather than at its own feet. Without
        // this a body at a run reads as studying the ground.
        let rig = biped();
        let neck = rig.in_zone(Zone::Neck)[0];
        let Some(&head) = rig.in_zone(Zone::Head).first() else {
            return;
        };
        let gait = Gait::natural(&rig);
        let pitch = |pose: &Pose| {
            let posed = pose.forward(&rig);
            let run = posed.positions[head] - posed.positions[neck];
            run.z.atan2(run.y).to_degrees()
        };
        let rest = pitch(&Pose::rest(&rig));
        for pace in [0.5f32, 1.0, 2.0] {
            let mut pose = Pose::rest(&rig);
            lean(&rig, &mut pose, &gait, &Stride::for_body(&rig, pace));
            let carried = pitch(&pose);
            assert!(
                (carried - rest).abs() < 0.05,
                "at pace {pace} the lean carried the head {:.2} deg off level",
                carried - rest
            );
        }
    }

    /// How far apart two rotations may be and still count as the same pose.
    ///
    /// Loose enough for float noise and two orders inside anything a missing
    /// stage would do, since dropping one moves joints by degrees.
    const SAME_POSE: f32 = 1e-6;

    /// How far one rotation is from another, as `1 - |dot|` of the two
    /// **normalised**.
    ///
    /// **The normalising is the whole point and leaving it out cost an hour.**
    /// `1 - |dot(a, b)|` is a distance only if both are unit quaternions, and a
    /// pose's rotations are products of several composed turns whose norm has
    /// drifted a part in a million by the time a knee has been stepped, planted
    /// and rolled. Compared raw, a quaternion differs from ITSELF by up to
    /// 1.1e-6 — which is what the bitwise check finally showed: zero joints
    /// different, and a "difference" reported at every one of them. Two whole
    /// hypotheses were chased on the strength of that number, one about closures
    /// passed by reference and one about inlining, and both were answers to a
    /// question the instrument had invented.
    fn apart(a: Quat, b: Quat) -> f32 {
        1.0 - a.normalize().dot(b.normalize()).abs()
    }

    #[test]
    fn the_entry_point_runs_the_sequence_an_instrument_would_hand_roll() {
        // **#253.** The point of `Walk` is that not choosing gives the whole
        // sequence, and this is what says the whole sequence is still the one
        // the instruments run. If they ever diverge, `examples/walkaudit` stops
        // measuring what the app draws — which is the failure mode that makes a
        // second instrument worthless.
        //
        // Written out longhand deliberately: this is the one place where
        // repeating the subject is the test rather than a flaw in it.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let grade = 0.15;
        let ground = |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, foot.z * grade, foot.z)));

        for sample in 0..12 {
            let cycle = sample as f32 / 12.0;

            let mut longhand = Pose::rest(&rig);
            let expected = step(&rig, &mut longhand, &gait, &stride, cycle, ground);
            swing_arms(&rig, &mut longhand, &gait, &stride, cycle);
            lean(&rig, &mut longhand, &gait, &stride);
            if !expected.stance.is_empty() {
                plant_feet_of(
                    &rig,
                    &mut longhand,
                    &expected.stance,
                    ground,
                    &FootingConfig::default(),
                );
            }
            roll_feet(&rig, &mut longhand, &gait, &stride, cycle, ground);

            let mut driven = Pose::rest(&rig);
            let walked = Walk::at(cycle).drive(&rig, &mut driven, &gait, &stride, ground);

            assert_eq!(walked.steps.stance, expected.stance, "at cycle {cycle}");
            assert_eq!(walked.steps.swing, expected.swing, "at cycle {cycle}");
            assert!(
                (driven.translation - longhand.translation).length() < 1e-6,
                "at cycle {cycle} the root sat differently"
            );
            for joint in 0..longhand.rotations.len() {
                let apart = apart(driven.rotations[joint], longhand.rotations[joint]);
                assert!(
                    apart < SAME_POSE,
                    "at cycle {cycle} joint {joint} differs between the entry point and the \
                     sequence it is supposed to be"
                );
            }
        }
    }

    #[test]
    fn the_ablation_switches_take_off_what_they_name_and_nothing_else() {
        // The other half of the contract: an instrument turning a stage off has
        // to get exactly that stage off. Without this, `posture` could quietly
        // become "and also skip the roll" and every reading taken with it off
        // would be measuring two changes at once.
        let rig = biped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let ground = |_: Vec3| None;
        let cycle = 0.3;

        let legs_only = Walk {
            posture: false,
            ..Walk::at(cycle)
        };
        let mut bare = Pose::rest(&rig);
        legs_only.drive(&rig, &mut bare, &gait, &stride, ground);

        let mut expected = Pose::rest(&rig);
        let steps = step(&rig, &mut expected, &gait, &stride, cycle, ground);
        plant_feet_of(
            &rig,
            &mut expected,
            &steps.stance,
            ground,
            &FootingConfig::default(),
        );
        roll_feet(&rig, &mut expected, &gait, &stride, cycle, ground);
        for joint in 0..expected.rotations.len() {
            let apart = apart(bare.rotations[joint], expected.rotations[joint]);
            assert!(
                apart < SAME_POSE,
                "posture off changed joint {joint} as well"
            );
        }

        // And an arm must actually have moved with it on, or the switch is
        // testing nothing.
        let mut dressed = Pose::rest(&rig);
        Walk::at(cycle).drive(&rig, &mut dressed, &gait, &stride, ground);
        let shoulder = rig.in_zone(Zone::UpperLimb(Limb::ForeLeft))[0];
        assert!(
            apart(dressed.rotations[shoulder], bare.rotations[shoulder]) > 1e-6,
            "the posture switch moved nothing"
        );

        // Footing off must leave the gait's own placement alone.
        let unplanted = Walk {
            footing: None,
            ..Walk::at(cycle)
        };
        let mut raw = Pose::rest(&rig);
        unplanted.drive(&rig, &mut raw, &gait, &stride, ground);
        let mut without = Pose::rest(&rig);
        step(&rig, &mut without, &gait, &stride, cycle, ground);
        swing_arms(&rig, &mut without, &gait, &stride, cycle);
        lean(&rig, &mut without, &gait, &stride);
        roll_feet(&rig, &mut without, &gait, &stride, cycle, ground);
        for joint in 0..without.rotations.len() {
            let apart = apart(raw.rotations[joint], without.rotations[joint]);
            assert!(
                apart < SAME_POSE,
                "footing off changed joint {joint} as well"
            );
        }
    }

    #[test]
    fn standing_still_leaves_the_body_standing() {
        let rig = biped();
        let mut pose = Pose::rest(&rig);
        let steps = step(
            &rig,
            &mut pose,
            &Gait::standing(&rig),
            &Stride::still(),
            0.4,
            |_| None,
        );
        assert_eq!(steps.swing.len(), 0);
        assert!(steps.is_clean());
        assert_eq!(steps.crouch, 0.0, "a still body has nothing to sink for");
        for rotation in &pose.rotations {
            // Not exactly nothing: a rest leg stands at full extension, and the
            // solver holds a fraction of a degree back from that singularity on
            // purpose. Well under a degree is standing still.
            assert!(
                rotation.to_axis_angle().1 < 0.02,
                "a still body should not move: {rotation:?}"
            );
        }
    }

    /// The heel-to-toe run of one foot, in degrees against the ground and
    /// against its own rest attitude, positive toe-up.
    ///
    /// The same question `examples/walkaudit` asks, and asked the same way: the
    /// foot's nodes need not lie level in a foot that stands flat — on this body
    /// they run three degrees uphill — so zero means "carried as it stands".
    fn sole_pitch(rig: &Rig, pose: &Pose, limb: Limb) -> f32 {
        let joints = rig.extremity_joints(limb);
        let sole = &joints[1..];
        let along = |&joint: &usize| rig.joints[joint].position.z;
        let rear = *sole
            .iter()
            .min_by(|a, b| along(a).total_cmp(&along(b)))
            .expect("a foot");
        let fore = *sole
            .iter()
            .max_by(|a, b| along(a).total_cmp(&along(b)))
            .expect("a foot");
        let angle = |run: Vec3| {
            run.y
                .atan2((run.x * run.x + run.z * run.z).sqrt())
                .to_degrees()
        };
        let posed = pose.forward(rig);
        angle(posed.positions[fore] - posed.positions[rear])
            - angle(rig.joints[fore].position - rig.joints[rear].position)
    }

    /// One moment of the whole walk, in the order every consumer runs it:
    /// step, arms, plant, roll.
    fn walked(rig: &Rig, gait: &Gait, stride: &Stride, cycle: f32) -> (Pose, Vec<Limb>) {
        use crate::anim::ground::{FootingConfig, Ground, plant_feet_of};
        let ground = |foot: Vec3| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)));
        let mut pose = Pose::rest(rig);
        let steps = step(rig, &mut pose, gait, stride, cycle, ground);
        swing_arms(rig, &mut pose, gait, stride, cycle);
        plant_feet_of(
            rig,
            &mut pose,
            &steps.stance,
            ground,
            &FootingConfig::default(),
        );
        let straining = roll_feet(rig, &mut pose, gait, stride, cycle, ground);
        (pose, straining)
    }

    #[test]
    fn a_foot_lands_toe_up_and_leaves_toe_down() {
        // The measurement this whole slice exists for. Before it, the sole was
        // held flat through the entire cycle — `examples/walkaudit` reported
        // -0.0 to 0.0 degrees in both phases, which is a shuffle.
        //
        // Asked of the POSED body rather than of `foot_pitch`, because a
        // constant that never reaches the foot is exactly the failure #223
        // found at the elbow: 0.639 of every degree asked was arriving, and no
        // test of the constant could have seen it.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        let (mut lowest, mut highest) = (f32::MAX, f32::MIN);
        for sample in 0..240 {
            let cycle = sample as f32 / 240.0;
            let (pose, _) = walked(&rig, &gait, &stride, cycle);
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let pitch = sole_pitch(&rig, &pose, limb);
                lowest = lowest.min(pitch);
                highest = highest.max(pitch);
            }
        }

        // Reference bands from the gait literature, as `examples/walkaudit`
        // prints them: ~15-25 degrees toe-up at heel-strike, ~15-20 toe-down at
        // push-off. Asserted as bands rather than as figures, so re-tuning
        // inside them is free and leaving them is not.
        assert!(
            (15.0..=25.0).contains(&highest),
            "heel-strike reached {highest:.1} deg toe-up, outside the 15-25 band"
        );
        assert!(
            (-20.0..=-15.0).contains(&lowest),
            "push-off reached {lowest:.1} deg toe-down, outside the -20 to -15 band"
        );
    }

    #[test]
    fn the_part_of_the_sole_bearing_weight_does_not_move_as_the_foot_rolls() {
        // The invariant the pivot rule exists to hold, and the one that
        // separates this from a naive ankle pitch: a foot rolls about whatever
        // part of its sole is down, and that part stays where it is. Pitching
        // about the ankle instead — a joint 91 mm above the sole and 19 mm
        // ahead of the heel — drives the toe through the floor.
        //
        // Measured on the sole this module can actually read: every plan builds
        // its bodies standing on y = 0, so each of the foot's joints stands over
        // a sole point at rest height zero. The mesh's own sole is convex and
        // sits below that plane (#220), which is a build defect this cannot see
        // and must not be blamed for.
        let rig = biped();
        let gait = Gait::wave(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        let ground = |foot: Vec3| {
            Some(crate::anim::ground::Ground::level(Vec3::new(
                foot.x, 0.0, foot.z,
            )))
        };
        for sample in 0..240 {
            let cycle = sample as f32 / 240.0;
            let mut pose = Pose::rest(&rig);
            let steps = step(&rig, &mut pose, &gait, &stride, cycle, ground);
            {
                use crate::anim::ground::{FootingConfig, plant_feet_of};
                plant_feet_of(
                    &rig,
                    &mut pose,
                    &steps.stance,
                    ground,
                    &FootingConfig::default(),
                );
            }

            // Where each sole point stands before the roll, and after it.
            let soles = |pose: &Pose, limb: Limb| -> Vec<Vec3> {
                let joints = rig.extremity_joints(limb);
                let posed = pose.forward(&rig);
                let ankle = joints[0];
                joints[1..]
                    .iter()
                    .map(|&joint| {
                        let at = rig.joints[joint].position;
                        posed.positions[ankle]
                            + posed.rotations[ankle]
                                * (Vec3::new(at.x, 0.0, at.z) - rig.joints[ankle].position)
                    })
                    .collect()
            };

            let before: Vec<Vec<Vec3>> = steps.stance.iter().map(|&l| soles(&pose, l)).collect();
            roll_feet(&rig, &mut pose, &gait, &stride, cycle, ground);

            for (&limb, before) in steps.stance.iter().zip(&before) {
                let after = soles(&pose, limb);
                // The bearing point is whichever ends up lowest; it is the one
                // that must not have moved. The others are free to lift, which
                // is the whole of what rolling a foot is.
                let held = after
                    .iter()
                    .zip(before)
                    .map(|(after, before)| (after.y, after.distance(*before)))
                    .fold((f32::MAX, f32::MAX), |best, (height, moved)| {
                        if height < best.0 {
                            (height, moved)
                        } else {
                            best
                        }
                    });
                // **Three millimetres, and it was two when two errors were
                // cancelling.** This loop alternates pinning the ankle at the
                // attitude the roll wants and solving the leg under it, and the
                // solve turns the joint the ankle hangs from — so neither
                // constraint is exact until the two agree. Before #254 the leg
                // solve missed by the extremity's hang in a direction that
                // happened to offset the attitude error, and the pair read
                // under 2 mm. With the solve landing where it is aimed, the
                // residual is the alternation's own, and it converges to about
                // 2.2 mm rather than to nothing.
                //
                // Swept before settling here: 3.6 mm at two roll passes, 2.2 at
                // four, 2.1 at six with a tenfold tighter contact tolerance, and
                // 1.7 only at twelve contact passes and a tolerance a hundred
                // times tighter — which buys a fifth of a millimetre for
                // roughly double the solve cost. Ending the loop on the pin
                // instead of the solve was tried and is worse, at 2.5 mm.
                assert!(
                    held.1 < 3e-3,
                    "{limb:?} at cycle {cycle:.3}: the sole point bearing the weight \
                     slid {:.1} mm",
                    held.1 * 1000.0
                );
                // And nothing is driven under the floor it is standing on.
                let deepest = after.iter().map(|at| at.y).fold(f32::MAX, f32::min);
                assert!(
                    deepest > -2e-3,
                    "{limb:?} at cycle {cycle:.3}: the sole reached {:.1} mm under \
                     the floor",
                    deepest * 1000.0
                );
            }
        }
    }

    #[test]
    fn the_pitch_turns_the_corner_between_stance_and_swing_without_a_kink() {
        // Every ramp is a smoothstep so that the pitch is C1 all the way round,
        // including the two joins the phases share — where a foot leaves the
        // ground and where it lands. A corner here is an infinite angular
        // acceleration at the ankle, and it reads as a flick.
        //
        // Asserted as a bound on the second difference: a curve with a corner
        // has one sample's worth of turn concentrated at a point, so its second
        // difference does not shrink as the sampling refines. This one's does.
        let gait = Gait::wave(&biped());
        let curve = |steps: usize| -> f32 {
            let at = |sample: usize| foot_pitch(gait.phase(0, sample as f32 / steps as f32));
            (1..steps)
                .map(|sample| (at(sample + 1) - 2.0 * at(sample) + at(sample - 1)).abs())
                .fold(0.0f32, f32::max)
        };

        // Halving the step must quarter the worst second difference on a C1
        // curve. Allowed a wide margin — this is a shape test, not a numeric one
        // — but a corner would hold its value flat instead of falling at all.
        let coarse = curve(480);
        let fine = curve(960);
        assert!(
            fine < coarse * 0.4,
            "refining the sampling took the worst kink from {coarse:.2e} to only \
             {fine:.2e}, which is a corner rather than a curve"
        );

        // And the two ends of the cycle meet: the attitude a foot leaves the
        // ground in is the one it carries into its swing.
        let toe_off = foot_pitch(Phase::Stance(1.0));
        let lift = foot_pitch(Phase::Swing(0.0));
        assert!(
            (toe_off - lift).abs() < 1e-6,
            "the foot leaves at {toe_off:.4} rad and starts its swing at {lift:.4}"
        );
        let landing = foot_pitch(Phase::Swing(1.0));
        let strike = foot_pitch(Phase::Stance(0.0));
        assert!(
            (landing - strike).abs() < 1e-6,
            "the foot lands at {landing:.4} rad and starts its stance at {strike:.4}"
        );
    }

    #[test]
    fn a_standing_body_keeps_its_soles_flat() {
        // The rule #230 established, applied to this producer too: a gait that
        // never lifts a contact has no roll to express, whatever stride or
        // cycle the caller leaves on it.
        let rig = biped();
        let gait = Gait::standing(&rig);
        let stride = Stride::for_body(&rig, 1.0);

        for sample in 0..40 {
            let cycle = sample as f32 / 40.0;
            let (pose, straining) = walked(&rig, &gait, &stride, cycle);
            assert!(straining.is_empty(), "a standing body strained at {cycle}");
            for limb in [Limb::HindLeft, Limb::HindRight] {
                let pitch = sole_pitch(&rig, &pose, limb);
                assert!(
                    pitch.abs() < 0.5,
                    "a standing {limb:?} pitched {pitch:.2} deg at cycle {cycle}"
                );
            }
        }
    }

    #[test]
    fn rolling_asks_the_leg_for_no_more_reach_than_the_plant_already_got() {
        // The roll RAISES the contact at both extremes — a foot up on its heel
        // or its toe is a foot whose ankle is nearer the hip — so it cannot put
        // a goal out of reach that the plant had already reached. If this ever
        // fails, the pivot is being taken from the wrong end of the foot.
        let rig = biped();
        let gait = Gait::wave(&rig);
        for pace in [0.4, 1.0, 1.4] {
            let stride = Stride::for_body(&rig, pace);
            for sample in 0..120 {
                let cycle = sample as f32 / 120.0;
                let (_, straining) = walked(&rig, &gait, &stride, cycle);
                assert!(
                    straining.is_empty(),
                    "pace {pace} cycle {cycle:.3}: {straining:?} could not reach the roll"
                );
            }
        }
    }

    #[test]
    fn a_foot_of_one_node_is_left_alone_rather_than_given_an_invented_axis() {
        // Nothing here is written for two legs — the pitch comes from the phase
        // and the pivot from the foot's own geometry — but a quadruped in this
        // crate has nothing to roll: `plan::quadruped` gives each limb a SINGLE
        // extremity node, so there is no heel-to-toe run to pitch along and no
        // second sole point to bear the weight.
        //
        // The right answer is to leave it exactly as the plant left it. Deriving
        // an axis from one point would mean choosing one, and a hoof pitched
        // about a guess is worse than a hoof that does not pitch. Giving a beast
        // a foot it can roll is a plan question (#178), not a gait one.
        let rig = quadruped();
        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        assert_eq!(gait.len(), 4, "the beast this tests needs four corners");
        for limb in &gait.limbs {
            assert_eq!(
                rig.extremity_joints(*limb).len(),
                2,
                "{limb:?} has more than an ankle and one node — rewrite this test"
            );
        }

        let ground = |foot: Vec3| {
            Some(crate::anim::ground::Ground::level(Vec3::new(
                foot.x, 0.0, foot.z,
            )))
        };
        for sample in 0..120 {
            let cycle = sample as f32 / 120.0;
            let mut pose = Pose::rest(&rig);
            let steps = step(&rig, &mut pose, &gait, &stride, cycle, ground);
            swing_arms(&rig, &mut pose, &gait, &stride, cycle);
            {
                use crate::anim::ground::{FootingConfig, plant_feet_of};
                plant_feet_of(
                    &rig,
                    &mut pose,
                    &steps.stance,
                    ground,
                    &FootingConfig::default(),
                );
            }

            let planted = pose.clone();
            let straining = roll_feet(&rig, &mut pose, &gait, &stride, cycle, ground);
            assert!(
                straining.is_empty(),
                "a foot that cannot roll must not be reported as straining at {cycle:.3}"
            );
            for (before, after) in planted.rotations.iter().zip(&pose.rotations) {
                assert!(
                    before.abs_diff_eq(*after, 1e-6),
                    "the beast was moved at cycle {cycle:.3}"
                );
            }
        }
    }
}
