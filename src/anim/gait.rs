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
//! the heel-strike and push-off attitudes a walk is judged by. Each reads the
//! pose the one before it left; running the roll before the plant would simply
//! be levelled away.
//!
//! The cycle itself is the caller's to drive, because how fast a body walks is a
//! question about the world — speed, terrain, intent — and this crate does not
//! know about any of that.

use glam::{Quat, Vec3};

use super::ground::{Ground, solve_contact};
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
}

/// Fraction of a walk cycle a two-legged body spends on both feet.
///
/// The overlap that makes a walk a walk rather than a slow run.
pub const DOUBLE_SUPPORT: f32 = 0.1;

/// How much further than strictly necessary a body sinks to take its stride.
///
/// The margin keeps a visible bend in the knee and the solver clear of the
/// singularity at full extension. It scales with the sinking rather than being
/// added to it, so a body that is standing still does not crouch at all.
pub const CROUCH_MARGIN: f32 = 1.15;

/// How far and how high a body steps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stride {
    /// Direction of travel in body space. `+Z` is forward.
    pub direction: Vec3,
    /// Ground covered per cycle, in metres.
    pub length: f32,
    /// How high a contact lifts at the top of its swing, in metres.
    pub lift: f32,
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
        }
    }

    /// Standing still.
    #[must_use]
    pub fn still() -> Self {
        Self {
            direction: Vec3::Z,
            length: 0.0,
            lift: 0.0,
        }
    }
}

/// How far one limb must sink for its chain to reach a goal `toward` its rest
/// contact, before any margin.
///
/// Measured to the joint the chain actually reaches, not to the contact hanging
/// off it — the same distinction the solve makes. Solved against the limb's
/// *actual* rest geometry rather than from stride length alone, because a
/// limb's contact is not generally beneath its hip: a quadruped's feet already
/// sit well forward of the joints that carry them, and assuming otherwise
/// under-crouches every four-legged body.
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
#[must_use]
pub fn crouch_for(rig: &Rig, gait: &Gait, stride: &Stride) -> f32 {
    let half = stride.length * 0.5;

    gait.limbs
        .iter()
        .filter_map(|&limb| {
            // Both ends of the stride, since a contact may start forward of its
            // hip and only the further extreme matters.
            [half, -half]
                .into_iter()
                .filter_map(|swing| sink_needed(rig, limb, stride.direction * swing))
                .fold(f32::NEG_INFINITY, f32::max)
                .into()
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN
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
    gait.limbs
        .iter()
        .enumerate()
        .filter_map(|(index, &limb)| {
            sink_needed(rig, limb, contact_offset(stride, gait.phase(index, cycle)))
        })
        .fold(0.0f32, f32::max)
        * CROUCH_MARGIN
}

/// Where a contact belongs, relative to where it rests.
///
/// During stance the foot holds still in the world while the body travels over
/// it, which in body space is a slide backwards. During swing it arcs forward
/// and up, and back down to meet the ground at the front of the step.
#[must_use]
pub fn contact_offset(stride: &Stride, phase: Phase) -> Vec3 {
    let half = stride.length * 0.5;
    match phase {
        Phase::Stance(t) => stride.direction * (half - stride.length * t),
        Phase::Swing(t) => {
            let along = stride.direction * (stride.length * t - half);
            along + Vec3::Y * (stride.lift * (t * std::f32::consts::PI).sin())
        }
    }
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
                seated_offset(contact_offset(stride, phase), home, &ground)
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
    pose.translation.y -= steps.crouch;

    for (limb, phase, _, target) in goals {
        if phase.is_stance() {
            steps.stance.push(limb);
        } else {
            steps.swing.push(limb);
        }
        if !solve_contact(rig, pose, limb, target) {
            steps.straining.push(limb);
        }
    }

    steps
}

/// Lift a contact's offset onto the ground actually beneath where it is going.
///
/// **The whole of #221.** [`contact_offset`] describes a stride against the
/// body's own rest ground plane: a stance foot slides backwards at that height
/// and a swing foot arcs above it, and both end the step exactly where they
/// began it vertically. On a slope that is wrong at both ends — uphill the arc
/// drove the sole 38.9 mm into the surface it was travelling over, downhill it
/// landed in the air and dropped the last centimetres at the plant.
///
/// The probe is taken **under the goal at this instant** rather than blended
/// between the step's two endpoints. The endpoint blend was the first design
/// and it is the weaker one: it holds only for ground that is flat between the
/// footfalls, so a foot still ploughs through anything it passes over on the
/// way. Sampling where the foot actually is clears whatever is under it, and
/// costs one probe per contact per frame.
///
/// **The correction is the ground's RISE between the two points, not its
/// height at one of them**, and getting that wrong is instructive enough to
/// record. Seating the goal at the probed surface directly — `offset.y +=
/// surface.y - home.y` — reads the difference between a SURFACE and a JOINT,
/// and `home` is the extremity joint, which sits inside the foot about 32 mm
/// above the sole. On level ground, where this should do nothing at all, it
/// drove every contact 32 mm under: the swing went from clearing by 2.3 mm to
/// scuffing by 30.1 mm and the pelvis sank half again as far. Only a difference
/// of two probes is dimensionally sound, because the stand-off cancels.
///
/// A probe that answers `None` at either end leaves the offset alone, which is
/// the level behaviour this had before — so a caller with no terrain to offer
/// loses nothing, and one whose terrain has holes in it gets the rest pose's
/// height rather than a hole.
///
/// The probe runs in whatever frame the caller poses in, because `home` and the
/// offset are both in that frame. A caller whose body is somewhere else in the
/// world must say so in the closure, exactly as [`super::plant_feet_of`]
/// already requires.
fn seated_offset<F>(offset: Vec3, home: Vec3, ground: &F) -> Vec3
where
    F: Fn(Vec3) -> Option<Ground>,
{
    match (ground(home), ground(home + offset)) {
        (Some(here), Some(there)) => offset + Vec3::Y * (there.position.y - here.position.y),
        _ => offset,
    }
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
/// **Two, measured, and the second is not optional.** Swept on the default walk
/// through `examples/walkaudit`: one pass delivers -34.6 to 9.5 degrees against
/// the -17.2 to 20.1 it is asked for, and scuffs the sole 37.1 mm under the
/// floor; two deliver the asked figures to a tenth of a degree and clear the
/// floor by 10.0 mm; three are identical to two on every column. So this is the
/// converged answer rather than a budget.
const ROLL_PASSES: usize = 2;

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
pub fn roll_feet(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32) -> Vec<Limb> {
    let mut straining = Vec::new();
    // A gait that never lifts a contact expresses no roll either — the same
    // rule [`step`] and [`crouch_at`] apply to their own outputs (#230). A
    // standing body puts its soles down flat and leaves them there.
    if !pose.fits(rig) || gait.duty >= 1.0 {
        return straining;
    }

    for (index, &limb) in gait.limbs.iter().enumerate() {
        let pitch = foot_pitch(gait.phase(index, cycle));
        if pitch == 0.0 {
            continue;
        }
        if roll_one(rig, pose, limb, pitch) == Some(false) {
            straining.push(limb);
        }
    }

    straining
}

/// Rolls one foot by `pitch` about whichever of its sole points bears the
/// weight.
///
/// `None` for a limb with no foot to roll — one the body has not got, or one
/// whose extremity is a single node and so has no length to pitch along.
/// Otherwise whether the leg reached the goal the roll asked for.
fn roll_one(rig: &Rig, pose: &mut Pose, limb: Limb, pitch: f32) -> Option<bool> {
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
    let settled = posed.rotations[ankle];
    let up = (settled * Vec3::Y).try_normalize()?;

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
pub fn swing_arms(rig: &Rig, pose: &mut Pose, gait: &Gait, cycle: f32) {
    if !pose.fits(rig) {
        return;
    }

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
            ((cycle - offset + ARM_LAG) * std::f32::consts::TAU).sin()
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
    let pitch = TRUNK_LEAN * pace_of(rig, stride);
    if pitch.abs() <= f32::EPSILON {
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
    // Positive about X carries `+Y` toward `+Z`, and `+Z` is forward — so a
    // forward lean is the positive rotation here, where a forward arm swing was
    // the negative one. The difference is not a sign error waiting to happen:
    // an arm hangs DOWN and a trunk stands UP, so the same rotation carries them
    // opposite ways.
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return;
    };
    let hinge = spine[0];
    let below = rig.joints[hinge].position - rig.joints[root].position;
    let above = rig.joints[girdle].position - rig.joints[hinge].position;
    let turn = trunk_angle_for(below, above, pitch);
    pose.rotations[hinge] *= Quat::from_rotation_x(turn);
    pose.rotations[neck] *= Quat::from_rotation_x(-turn);
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
/// pitched by `wanted`.
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
fn trunk_angle_for(below: Vec3, above: Vec3, wanted: f32) -> f32 {
    let pitch = |run: Vec3| run.z.atan2(run.y);
    let rest = pitch(below + above);
    let mut turn = wanted;
    for _ in 0..TRUNK_PASSES {
        let delivered = pitch(below + Quat::from_rotation_x(turn) * above) - rest;
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

    #[test]
    fn a_stance_foot_travels_backwards_and_a_swing_foot_lifts() {
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.8,
            lift: 0.1,
        };

        let early = contact_offset(&stride, Phase::Stance(0.0));
        let late = contact_offset(&stride, Phase::Stance(1.0));
        assert!(early.z > late.z, "the body travels over a planted foot");
        assert_eq!(early.y, 0.0, "a planted foot stays down");

        let peak = contact_offset(&stride, Phase::Swing(0.5));
        assert!(
            (peak.y - 0.1).abs() < 1e-5,
            "the swing should reach its lift"
        );
        assert!(contact_offset(&stride, Phase::Swing(0.0)).y.abs() < 1e-5);
        assert!(contact_offset(&stride, Phase::Swing(1.0)).y.abs() < 1e-5);
    }

    #[test]
    fn a_step_ends_where_the_next_one_starts() {
        // Stance must hand off to swing without a jump, or the foot teleports
        // once per cycle.
        let stride = Stride {
            direction: Vec3::Z,
            length: 0.8,
            lift: 0.1,
        };
        let handoff = contact_offset(&stride, Phase::Stance(1.0));
        let pickup = contact_offset(&stride, Phase::Swing(0.0));
        assert!(handoff.distance(pickup) < 1e-5, "{handoff:?} vs {pickup:?}");

        let landing = contact_offset(&stride, Phase::Swing(1.0));
        let plant = contact_offset(&stride, Phase::Stance(0.0));
        assert!(landing.distance(plant) < 1e-5, "{landing:?} vs {plant:?}");
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
        for frame in 0..12 {
            let cycle = frame as f32 / 12.0;
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, cycle);
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
        let bend_at = |limb: Limb, cycle: f32| {
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, cycle);
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
        let mut pose = Pose::rest(&rig);
        swing_arms(&rig, &mut pose, &gait, 0.0);
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
        let mut pose = Pose::rest(&rig);
        // A quarter cycle, where the lead is near its widest.
        swing_arms(&rig, &mut pose, &gait, 0.25);
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
        let posed_at = |cycle: f32| {
            let mut pose = Pose::rest(&rig);
            swing_arms(&rig, &mut pose, &gait, cycle);
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
        let mut pose = Pose::rest(rig);
        let steps = step(rig, &mut pose, gait, stride, cycle, |_| None);
        swing_arms(rig, &mut pose, gait, cycle);
        plant_feet_of(
            rig,
            &mut pose,
            &steps.stance,
            |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
            &FootingConfig::default(),
        );
        let straining = roll_feet(rig, &mut pose, gait, cycle);
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

        for sample in 0..240 {
            let cycle = sample as f32 / 240.0;
            let mut pose = Pose::rest(&rig);
            let steps = step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
            {
                use crate::anim::ground::{FootingConfig, Ground, plant_feet_of};
                plant_feet_of(
                    &rig,
                    &mut pose,
                    &steps.stance,
                    |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
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
            roll_feet(&rig, &mut pose, &gait, cycle);

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
                assert!(
                    held.1 < 2e-3,
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

        for sample in 0..120 {
            let cycle = sample as f32 / 120.0;
            let mut pose = Pose::rest(&rig);
            let steps = step(&rig, &mut pose, &gait, &stride, cycle, |_| None);
            swing_arms(&rig, &mut pose, &gait, cycle);
            {
                use crate::anim::ground::{FootingConfig, Ground, plant_feet_of};
                plant_feet_of(
                    &rig,
                    &mut pose,
                    &steps.stance,
                    |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
                    &FootingConfig::default(),
                );
            }

            let planted = pose.clone();
            let straining = roll_feet(&rig, &mut pose, &gait, cycle);
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
