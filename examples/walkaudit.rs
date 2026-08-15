//! Measures a walk cycle against the body walking it, and against the gait
//! literature.
//!
//! `examples/dump -- --walk` writes frames to look at and prints what the gait
//! and the terrain each decided. This asks the questions a walk is judged on
//! that no parameter can answer, because each of them is a property of the
//! *posed body* rather than of a constant:
//!
//! Two readings exist for the ground a slope never asked about. The **halves**
//! split the sole fore and aft and score each against the ground beneath
//! itself, because a sole spans 289 mm of foot and a staircase can put a riser
//! under the middle of it; the **jump** is the furthest a contact moved between
//! two samples, because a seat that teleports is above the surface at every
//! sample and no penetration reading can see it. A jump that halves when
//! `--samples` doubles was fast; one that does not was a cliff.
//!
//! 1. **Does a swinging foot clear the ground?** Not the ankle — the foot. The
//!    leg IK places a joint, and everything below it rides along in whatever
//!    orientation the rest pose left it, so a foot can sit at a perfectly good
//!    ankle height with its toe through the floor. Measured on the real surface,
//!    selected by [`Patch`], which is the only thing that can find a foot on a
//!    body whose foot is part of its leg.
//! 2. **Is the elbow bent?** Asked as the angle at the joint between the upper
//!    arm and the forearm, so it cannot be satisfied by a rotation that turns
//!    out to be about the arm's own axis and bends nothing.
//! 3. **Does the torso turn with the arms?** Asked as the angle between the
//!    shoulder line and the hip line, measured off the posed skeleton. A body
//!    that swings its arms from rigid shoulders reads as a puppet.
//! 4. **Does the pelvis bob?** A real walk vaults over its stance leg twice a
//!    cycle; a pelvis that rides at one height reads as a dolly shot.
//! 5. **Does the foot articulate?** Heel-strike lands toe-up, push-off leaves
//!    toe-down. A sole held flat through the whole cycle is a shuffle.
//! 6. **Does a planted foot hold still in the world?** Which is the question a
//!    turn is judged on, and the one nothing here could ask until `--turn`
//!    existed, because everything above is a property of the body's *own*
//!    frame and a skid is only visible from outside it. See the world clock
//!    below.
//!
//! # The world clock, and why a turn needs one
//!
//! Every reading above is taken on a body posed at the origin: the gait
//! expresses travel as a foot sliding backwards *in body space*, and where the
//! body actually is never comes into it. That is exactly why a turn could not
//! be measured. A planted foot is pinned to the **ground**, not to the body,
//! and whether it stays pinned is a question about the world frame the body is
//! moving through.
//!
//! So `--turn` gives the instrument one: the body travels a circular arc of
//! whatever curvature its stride and its yaw rate imply, and every foot is
//! carried out into that frame before it is measured. Three readings follow,
//! and the first of them checks the clock itself — **on a straight walk a
//! planted foot must not move at all**, and it only comes out that way if the
//! body's travel per cycle and the stride's excursion per stance agree. They
//! are derived from each other, so the reading is a real check on the crate
//! rather than on the arithmetic here.
//!
//! A yaw *rate* is per second where everything else here is per cycle, and the
//! two are joined by the cadence. That is taken from the stride the body is
//! actually walking, through [`Speed::of`], rather than named beside it — the
//! same argument `pace_of` makes for the trunk lean, and the reason a `--turn`
//! reading cannot be a turn the legs are not taking.
//!
//! **Clearances are measured against the body's own standing depth, not against
//! `y = 0`.** The build delivers a sole that bulges below its own ground plane
//! (#220), and an instrument that measures from the floor reads that geometry as
//! a gait defect. The baseline is printed so the build defect stays visible; it
//! is just not billed to the walk.
//!
//! The table samples at display resolution (`--frames`); every summary figure is
//! computed from its own fine sweep of the cycle, because a coarse table hides
//! whatever falls between its rows — at 16 frames the swing's lowest pass never
//! appeared at all.
//!
//! Reference bands are approximate normal-adult figures from the gait-analysis
//! literature (Murray's walking-pattern studies, Winter's and Perry's gait
//! texts): minimum toe clearance ~10–30 mm; elbow never straight, ~10–25° at
//! its straightest and ~25–45° at its peak; shoulder-against-hip counter-rotation
//! peaking ~5–15°; pelvis vertical excursion ~25–50 mm peak-to-peak at a natural
//! pace; duty factor ~0.6 with ~0.2 of the cycle on both feet; heel-strike
//! ~15–25° toe-up and push-off ~15–20° toe-down. They are bands to stand beside
//! a number, not targets to fit — this body is stylised, and where it leaves a
//! band on purpose the summary should say so, out loud, every run.
//!
//! ```text
//! cargo run --example walkaudit
//! cargo run --example walkaudit -- --frames 24
//! cargo run --example walkaudit -- --pace 1.4 --grade 0.12
//! cargo run --example walkaudit -- --camber 0.20
//! cargo run --example walkaudit -- --run          # a run, not a walk (#186)
//! cargo run --example walkaudit -- --turn 40      # a left turn, 40 deg/s (#241)
//! cargo run --example walkaudit -- --turn -40 --grade 0.15
//! cargo run --example walkaudit -- --turn 40 --bare   # the placement alone
//! cargo run --example walkaudit -- --heading 180      # walking backwards (#242)
//! cargo run --example walkaudit -- --heading 90       # strafing left
//! cargo run --example walkaudit -- --headings         # the sweep, and the pop check
//! cargo run --example walkaudit -- --step 0.15        # a staircase, not a slope (#245)
//! cargo run --example walkaudit -- --step 0.15 --step-phase 0.0   # risers under the feet
//! cargo run --example walkaudit -- --step 0.15 --samples 960      # fast, or discontinuous?
//! ```
//!
//! [`Speed::of`]: symbios_avatar::anim::Speed::of

use glam::{Quat, Vec3};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Gait, Ground, Heading, Limb, Patch, Pose, Rig, Stride, Walk,
    anim::{Speed, gait},
};

/// How many points of the cycle the summary statistics are taken from.
///
/// Deliberately not `--frames`: the table is for reading and the statistics are
/// for judging, and a statistic taken at display resolution inherits the
/// display's blind spots.
const SWEEP: usize = 240;

/// The same, as `--samples` overrides it.
///
/// **A jump reading cannot tell fast from discontinuous on its own.** A goal
/// that teleports moves the same millimetres however finely the cycle is
/// sampled; one that is merely travelling quickly moves half as far when the
/// sampling doubles. Doubling this is the test, and it is the only way to read
/// a jump figure that is large but not obviously a cliff (#245).
fn sweep_of(args: &[String]) -> usize {
    args.iter()
        .position(|arg| arg == "--samples")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(SWEEP)
        .max(2)
}

/// What one foot's sole is doing at one moment, in the body's own frame.
struct Sole {
    /// Whether the foot is swinging.
    airborne: bool,
    /// The lowest point of the whole sole above the ground **beneath that
    /// point**, measured from the body's own standing depth.
    lowest: f32,
    /// The same reading taken over the rear half of the sole patch, and over
    /// the fore half.
    ///
    /// **A sole spans about 200 mm of foot, and on a staircase that is enough
    /// to span a riser** — so a single minimum over the patch cannot say
    /// whether a foot is buried in the ground it stands on or standing on a
    /// tread with its toe out over the next step's column. The two halves can
    /// (#245), and which of them the worst reading came from is the difference
    /// between a placement defect and a foot that is simply longer than the
    /// stair it is being asked to fit on.
    heel: f32,
    toe: f32,
    /// Progress through the current phase, `0..1`.
    progress: f32,
    /// Pitch against the ground plane in degrees, positive toe-up.
    pitch: f32,
}

/// What one moment of the walk measures.
struct Moment {
    /// Where in the cycle this is, `0..1`.
    cycle: f32,
    /// Per foot, in the body's own frame.
    feet: Vec<Sole>,
    /// Elbow bend per arm, degrees away from straight.
    elbows: Vec<f32>,
    /// Shoulder line against hip line, degrees.
    twist: f32,
    /// Pelvis height about its standing height, in metres.
    pelvis: f32,
    /// Trunk pitch away from its own rest carriage, in degrees, positive
    /// forward.
    lean: f32,
    /// Trunk bank away from its own rest carriage, in degrees, positive toward
    /// the body's left — which on a left turn is INTO it.
    bank: f32,
    /// Per foot, in the **world** frame: whether it is bearing weight, where
    /// its ground contact is, and which way its heel-to-toe line points in
    /// degrees.
    ///
    /// Separate from `feet` because it is a different frame and not a different
    /// column: everything in `feet` is a property of the body's own space, and
    /// merging the two is how an instrument ends up reporting a world skid as a
    /// body-space clearance.
    planted: Vec<(bool, Vec3, f32)>,
    /// Per foot, in the **world** frame: the fore-and-aft span the sole covers
    /// while it is bearing weight, and `None` while it is not.
    ///
    /// The ground the foot is actually standing on, as opposed to the point the
    /// gait pins — which is what a staircase has to be laid out against.
    footprint: Vec<Option<(f32, f32)>>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let frames = number("--frames").map_or(16, |value| value as usize).max(2);
    let samples = sweep_of(&args);
    let pace = number("--pace").unwrap_or(1.0);
    let grade = number("--grade").unwrap_or(0.0);
    // The lateral axis, which is a different question from the grade and was
    // for a long time the only one being asked by accident: `examples/locomotion`
    // tilted its ground along X and called the result a grade for its whole
    // life (fixed under #221). A foot on a side-slope has to ROLL to meet it,
    // where a foot on a grade has to PITCH, and nothing here measured the first
    // one until #250.
    let camber = number("--camber").unwrap_or(0.0);
    // Degrees per second, positive toward the body's LEFT — which is `+X`, and
    // which is also the direction a positive rotation about `+Y` carries `+Z`,
    // so the sign here is the sign of the yaw and not a convention layered over
    // one. Checked against the rig rather than assumed: `Limb::HindLeft` sits
    // at `x = +0.088` on the default body.
    let turn = number("--turn").unwrap_or(0.0);
    // Which way the body TRAVELS, against the way it faces: 0 is forward, 180
    // is backwards, +90 is a strafe to the body's left. A separate input from
    // the speed and from the turn, so a diagonal is a heading rather than a
    // mode — see `Heading` (#242).
    let heading = number("--heading").unwrap_or(0.0);
    // A staircase instead of a plane: the rise of each step, in metres, with
    // the tread taken from the stride so the body meets one riser per step it
    // takes. **A smooth grade is the easy case** — the terrain-aware swing #221
    // added seats each goal on the ground beneath it, which is exactly right
    // while that ground is continuous and is a cliff the moment it is not
    // (#245).
    let step = number("--step").unwrap_or(0.0);
    // Ablation: the gait's own placement, with neither the plant nor the roll
    // over it. **Which is the only way to tell whose skid a skid is** — the
    // roll deliberately moves the contact joint, sliding the foot so that
    // whichever sole point is bearing weight stays put while the foot pitches
    // about it, and that shows in the planted reading as a slide the gait did
    // not ask for. On the flat it is 31.9 mm of the 32.1 measured; the gait's
    // own placement is 2.8. Without this flag the two are inseparable and the
    // turn gets billed for the ankle's work. Every column but the planted one
    // reads as a body with no posture and unsettled feet under this flag; it is
    // for the one reading, not for the table.
    let bare = args.iter().any(|arg| arg == "--bare");

    let record = AvatarRecord::new("Walker", Archetype::default());
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the walking body would not build");
        std::process::exit(1);
    };
    let rig = &avatar.rig;
    let body = &avatar.parts.body;
    let weights = &avatar.parts.weights;
    let gait = if args.iter().any(|arg| arg == "--run") {
        Gait::running(rig)
    } else {
        Gait::natural(rig)
    };
    let mut stride = Stride::for_body(rig, pace).toward(rig, Heading::degrees(heading));

    // The world clock. See the module docs: a yaw RATE is per second and
    // everything else here is per cycle, so the cadence is what joins them —
    // recovered from the stride the legs are actually taking rather than named
    // beside it, so a `--turn` reading cannot describe a turn at a speed the
    // body is not walking.
    let cadence = Speed::of(rig, &gait, &stride).cadence(rig);
    let per_cycle_yaw = if cadence > f32::EPSILON {
        turn.to_radians() / cadence
    } else {
        0.0
    };
    // How far the body travels in one cycle, from the stride rather than
    // alongside it. A contact's excursion is the body's travel over the share
    // of the cycle that contact is DOWN, so the body's travel is the excursion
    // divided by the duty — and it is that identity the straight-line skate
    // reading checks.
    let per_cycle_travel = if gait.duty > f32::EPSILON && gait.duty < 1.0 {
        stride.length / gait.duty
    } else {
        0.0
    };
    // And what the legs are asked for: `Stride::yaw` is per STANCE, the same
    // span its `length` is, so it is the cycle's yaw times the duty.
    stride.yaw = per_cycle_yaw * gait.duty;
    let radius = if per_cycle_yaw.abs() > 1e-6 {
        per_cycle_travel / per_cycle_yaw
    } else {
        f32::INFINITY
    };

    // Where the body is and which way it faces after `u` cycles.
    //
    // A circular arc of the curvature the travel and the yaw imply, written in
    // the form that stays finite as the curvature goes to zero: `sin a / a` and
    // `(1 − cos a)/a` are its two shape functions and both have limits, where
    // the centre-and-radius form has neither. A straight walk is then the same
    // expression rather than a branch beside it.
    //
    // **Along the HEADING, not along `+Z`** (#242). This clock was written when
    // the only travel was forward, and it advanced the body down its own facing
    // — so a body strafing left was drawn walking forward while its feet walked
    // sideways, and the sweep's skid reading grew with the heading all the way
    // to 832 mm at backwards. That is the instrument disagreeing with the
    // crate, not the crate being wrong: `carried` had the heading right from
    // #241. The two build their arcs from the same two vectors now.
    let facing = Heading::degrees(heading).direction();
    let sideways = Vec3::Y.cross(facing);
    let frame = |u: f32| -> (Vec3, f32) {
        let a = per_cycle_yaw * u;
        let (along, across) = if a.abs() < 1e-4 {
            (1.0 - a * a / 6.0, a * 0.5)
        } else {
            (a.sin() / a, (1.0 - a.cos()) / a)
        };
        let travelled = per_cycle_travel * u;
        (
            facing * (travelled * along) + sideways * (travelled * across),
            a,
        )
    };
    // **One stair per FOOTFALL**, which is how a person takes stairs and the
    // only spacing that puts each landing on a tread rather than part way up a
    // riser. Taken from the body's travel per cycle divided by how many times
    // it transfers support, which is `Gait::footfalls` — the same count
    // `Speed::cycle_length` divides by, and taking the contact count instead
    // would put a trotting body on half a stair.
    let tread =
        (Stride::for_body(rig, pace).length / gait.duty / gait.footfalls().max(1) as f32).max(1e-3);
    // **Where the risers fall, which is a choice and was being made by accident.**
    // Fixing the tread to the footfall spacing decides how FAR apart the risers
    // are and says nothing about where they sit between the feet, and the grid
    // this had before was anchored at world `z = 0` — which is to say at
    // wherever the body happened to start. Measured, that put every riser 50 to
    // 85 mm under the leading edge of a 289 mm sole, on a 414 mm tread with 125
    // mm to spare, and the whole -98 mm reading of #245 was that overhang. A
    // staircase whose risers land under the feet measures its own alignment.
    //
    // So the phase is calibrated below against the body's own footprint rather
    // than chosen, and `--step-phase` overrides it for the deliberately
    // awkward case. `None` here is flat ground, which is also how the
    // calibration pass gets an unstepped body to measure.
    let riser_at: std::cell::Cell<Option<f32>> = std::cell::Cell::new(None);
    let world_height = |at: Vec3| {
        let plane = at.z * grade + at.x * camber;
        match riser_at.get() {
            Some(phase) if step != 0.0 => plane + ((at.z - phase) / tread).floor() * step,
            _ => plane,
        }
    };
    // Each tread is flat, so the surface normal is the underlying plane's — a
    // staircase is level underfoot and only its height jumps.
    let world_normal = Vec3::new(-camber, 1.0, -grade).normalize();
    // **The world has a height, and this had none.** A body-space reading is
    // taken against the ground under the body's own origin, so carrying one out
    // into the world means adding that ground back — and while it was left out,
    // every world reading was short by however far the body had climbed. On a
    // plane that is a smooth error and nothing here could see it. On a
    // staircase the reference under the origin is a step function, so the whole
    // body frame dropped 100 mm the moment the origin crossed a riser and a
    // contact that had not moved at all read as teleporting exactly one riser
    // (#245). The gait was innocent: the ruler had no vertical.
    let into_world = |u: f32, at: Vec3| {
        let (origin, yaw) = frame(u);
        origin + Vec3::Y * world_height(origin) + Quat::from_rotation_y(yaw) * at
    };

    // **The joint the gait actually pins**, which is not the ankle and the
    // difference is the whole reading. `home_of` and `solve_contact` both take
    // the contact from `in_zone(Extremity)`, whose first entry is the joint
    // *past* the ankle; `extremity_joints` carries the ankle at its head, which
    // is why the spans below slice it from 1. Measuring the ankle instead reads
    // the roll — it is a child of nothing that is planted, and it swings 88 mm
    // fore-and-aft across a stance on a perfectly straight walk while the
    // contact under it holds to 1.6 mm. That is this crate's recurring defect
    // exactly: an instrument measuring something other than its own name.
    let contacts: Vec<Option<usize>> = [Limb::HindLeft, Limb::HindRight]
        .into_iter()
        .map(|limb| {
            rig.in_zone(symbios_avatar::Zone::Extremity(limb))
                .first()
                .copied()
        })
        .collect();

    // The feet, as surface rather than as joints. Selected once from the rest
    // body and then carried through every frame, because a patch is a set of
    // vertex indices and deforming a mesh does not renumber it.
    let feet: Vec<(Limb, Patch)> = [Limb::HindLeft, Limb::HindRight]
        .into_iter()
        .map(|limb| {
            (
                limb,
                Patch::held_by(body, weights, &rig.extremity_joints(limb)),
            )
        })
        .collect();

    // The body's own standing depth, per foot: where the sole rests before
    // anything is posed. Every clearance is measured from here — see the module
    // docs and #220 for why the floor itself is the wrong ruler.
    let standing: Vec<f32> = feet
        .iter()
        .map(|(_, patch)| {
            patch
                .vertices()
                .map(|vertex| body.positions[vertex].y)
                .fold(f32::MAX, f32::min)
        })
        .collect();

    // Which half of the sole each vertex is in, taken once from the rest body
    // and split at the patch's own mid-length. Rear half is the heel's, fore
    // half is the toe's; see `Sole::heel` for why a whole-patch minimum cannot
    // answer the question a staircase asks.
    let halves: Vec<(Vec<usize>, Vec<usize>)> = feet
        .iter()
        .map(|(_, patch)| {
            let span = patch
                .vertices()
                .map(|vertex| body.positions[vertex].z)
                .fold((f32::MAX, f32::MIN), |span, z| {
                    (span.0.min(z), span.1.max(z))
                });
            let mid = (span.0 + span.1) / 2.0;
            patch
                .vertices()
                .partition(|&vertex| body.positions[vertex].z < mid)
        })
        .collect();

    // Heel and toe, for pitch: of the joints past the ankle, the rearmost and
    // foremost at rest. A foot with fewer than two has no pitch to measure.
    // Pitch is reported about the line's own REST attitude — the nodes need not
    // lie level in a foot that stands flat, and on this body they run 3 degrees
    // uphill — so 0 means "carried as it stands", exactly like the soles.
    let spans: Vec<Option<(usize, usize, f32)>> = feet
        .iter()
        .map(|(limb, _)| {
            let joints = rig.extremity_joints(*limb);
            let past_ankle = joints.get(1..).unwrap_or(&[]);
            let heel = past_ankle.iter().copied().min_by(|&a, &b| {
                rig.joints[a]
                    .position
                    .z
                    .total_cmp(&rig.joints[b].position.z)
            });
            let toe = past_ankle.iter().copied().max_by(|&a, &b| {
                rig.joints[a]
                    .position
                    .z
                    .total_cmp(&rig.joints[b].position.z)
            });
            match (heel, toe) {
                (Some(heel), Some(toe)) if heel != toe => {
                    let run = rig.joints[toe].position - rig.joints[heel].position;
                    let rest = run
                        .y
                        .atan2((run.x * run.x + run.z * run.z).sqrt())
                        .to_degrees();
                    Some((heel, toe, rest))
                }
                _ => None,
            }
        })
        .collect();

    let pelvis_rest = rig
        .joints
        .iter()
        .position(|joint| joint.parent.is_none())
        .map(|root| (root, rig.joints[root].position.y));

    // The instrument's model of terrain, and now the gait's too: `step` seats
    // its contacts on the same surface the plant settles them onto (#221),
    // which is what stops a swing arc built at the rest ground height from
    // ploughing through a slope it is climbing.
    //
    // **The plane is the world's; the closure is the body's.** `step` and
    // `plant_feet_of` ask what is beneath a point *in the frame the body is
    // posed in*, so a body that has walked and turned must be given the same
    // hillside seen from where it now stands — carried out into the world,
    // sampled, and brought back. On a plane that is a rotation of the gradient
    // and a subtraction of the body's own height, and with no turn and no
    // travel it collapses to exactly the expression this had before, which is
    // why every reading above is unmoved.
    let height = |u: f32, at: Vec3| world_height(into_world(u, at)) - world_height(frame(u).0);
    let measure = |u: f32, stride: &Stride| -> Moment {
        let cycle = u.rem_euclid(1.0);
        let normal = Quat::from_rotation_y(-frame(u).1) * world_normal;
        let floor = |foot: Vec3| {
            Some(Ground {
                position: Vec3::new(foot.x, height(u, foot), foot.z),
                normal,
            })
        };
        let mut pose = Pose::rest(rig);
        // The whole sequence, through the engine's own entry point (#253) —
        // step, arms, lean, plant, roll, in that order. Hand-rolled here until
        // the fourth stage arrived and the order stopped being something an
        // instrument should be trusted to remember.
        if bare {
            // **Not `Walk` with its flags turned down**, which is what this
            // tried first and is wrong: `Walk::settle` rolls the ankles
            // unconditionally — the roll is outside the `footing` option, on
            // purpose, because forgetting it is what #251 and #1069 were — so
            // a `Walk` with everything switched off still rolls, and the
            // ablation read 31.7 mm where the placement alone leaves 2.8. The
            // stage has to be stepped past rather than configured away.
            gait::step(rig, &mut pose, &gait, stride, cycle, floor);
        } else {
            Walk::at(cycle).drive(rig, &mut pose, &gait, stride, floor);
        }

        let posed = pose.forward(rig);
        let moved = posed.deform(rig, &body.positions, weights);

        // The footprint: what fore-and-aft band of world a bearing sole covers,
        // which is the band a riser must not be inside. Taken from the sole
        // patch rather than from the contact joint, because the joint is a
        // point and the question is about the 289 mm of foot around it.
        let footprint: Vec<Option<(f32, f32)>> = feet
            .iter()
            .map(|(limb, patch)| {
                let index = gait.limbs.iter().position(|other| other == limb);
                if index.is_some_and(|at| !gait.phase(at, cycle).is_stance()) {
                    return None;
                }
                Some(
                    patch
                        .vertices()
                        .map(|vertex| into_world(u, moved[vertex]).z)
                        .fold((f32::MAX, f32::MIN), |span, z| {
                            (span.0.min(z), span.1.max(z))
                        }),
                )
            })
            .collect();

        let feet = feet
            .iter()
            .zip(&standing)
            .zip(&spans)
            .zip(&halves)
            .map(|((((limb, patch), base), span), (rear, fore))| {
                // Height above the ground beneath that vertex, not above zero:
                // on a grade the floor is not level and a foot that tracks it
                // would otherwise read as sinking downhill.
                let above = |vertex: &usize| {
                    let at = moved[*vertex];
                    at.y - height(u, at)
                };
                let sole = patch
                    .vertices()
                    .map(|vertex| above(&vertex))
                    .fold(f32::MAX, f32::min);
                let heel = rear.iter().map(above).fold(f32::MAX, f32::min);
                let toe = fore.iter().map(above).fold(f32::MAX, f32::min);
                let index = gait.limbs.iter().position(|other| other == limb);
                let phase = index.map_or(gait::Phase::Stance(0.0), |at| gait.phase(at, cycle));
                let pitch = span.map_or(0.0, |(heel, toe, rest)| {
                    let run = posed.positions[toe] - posed.positions[heel];
                    let flat = (run.x * run.x + run.z * run.z).sqrt();
                    // Against the slope the foot walks on, not against level:
                    // a sole lying along a ramp is flat for this question. Read
                    // off the ground's normal *in the body's frame* rather than
                    // from `--grade` directly, because a body that has turned
                    // is climbing some mixture of the grade and the camber and
                    // neither flag alone names it any more.
                    run.y.atan2(flat).to_degrees()
                        - (-normal.z / normal.y).atan().to_degrees()
                        - rest
                });
                Sole {
                    airborne: !phase.is_stance(),
                    lowest: sole - base,
                    heel: heel - base,
                    toe: toe - base,
                    progress: phase.progress(),
                    pitch,
                }
            })
            .collect();

        let elbows = [Limb::ForeLeft, Limb::ForeRight]
            .into_iter()
            .map(|limb| elbow_bend(rig, &posed, limb))
            .collect();

        // The same feet again, in the world. A stance foot's contact should
        // hold one point of ground for the whole of its stance and its
        // heel-to-toe line should hold one bearing; both are free to do whatever they like
        // in body space while the body travels and turns over them, which is
        // why neither can be asked of the columns above.
        let planted = [Limb::HindLeft, Limb::HindRight]
            .into_iter()
            .zip(&contacts)
            .zip(&spans)
            .map(|((limb, contact), span)| {
                let index = gait.limbs.iter().position(|other| *other == limb);
                let stance = index.is_none_or(|at| gait.phase(at, cycle).is_stance());
                let at = contact.map_or(Vec3::ZERO, |joint| into_world(u, posed.positions[joint]));
                let bearing = span.map_or(0.0, |(heel, toe, _)| {
                    let run =
                        into_world(u, posed.positions[toe]) - into_world(u, posed.positions[heel]);
                    run.x.atan2(run.z).to_degrees()
                });
                (stance, at, bearing)
            })
            .collect();

        Moment {
            cycle,
            feet,
            elbows,
            twist: torso_twist(rig, &posed),
            pelvis: pelvis_rest.map_or(0.0, |(root, rest)| posed.positions[root].y - rest),
            lean: trunk_lean(rig, &posed),
            bank: trunk_bank(rig, &posed),
            planted,
            footprint,
        }
    };

    println!(
        "walking at pace {pace:.2} on a {:.0}% grade and {:.0}% camber, {frames} frames of \
         one cycle",
        grade * 100.0,
        camber * 100.0
    );
    if heading != 0.0 {
        println!(
            "travelling {heading:.0} deg off forward ({}), which takes {:.0}% of a forward \
             stride: {:.3} m against {:.3}",
            Heading::degrees(heading).describe(),
            Heading::degrees(heading).reach() * 100.0,
            stride.length,
            Stride::for_body(rig, pace).length,
        );
    }
    if turn != 0.0 {
        println!(
            "turning {turn:.1} deg/s {} at {:.2} m/s and {cadence:.2} cycles/s: {:.1} deg and \
             {:.3} m per cycle, on a {:.2} m radius",
            if turn > 0.0 { "left" } else { "right" },
            Speed::of(rig, &gait, &stride).metres_per_second(rig),
            per_cycle_yaw.to_degrees(),
            per_cycle_travel,
            radius.abs(),
        );
    }
    // **Laying the staircase out against the feet, not against the origin.**
    // The tread is already the footfall spacing, so every landing is congruent
    // modulo one tread and a single phase serves the whole flight. Which phase
    // is measured, not chosen: sweep the body over flat ground, fold every
    // bearing footprint onto one tread, and put the riser in the middle of the
    // widest band no sole ever covers. A riser anywhere else is one the foot is
    // standing on, and the -98 mm this instrument reported for a whole session
    // was exactly that (#245).
    //
    // If there is no free band the foot is longer than the stair, which is a
    // real and different finding — it is reported rather than papered over, and
    // the phase falls back to the emptiest point.
    let gap;
    if step != 0.0 {
        const BINS: usize = 2048;
        let mut covered = [false; BINS];
        for at in 0..samples * 2 {
            let moment = measure(at as f32 / samples as f32, &stride);
            for span in moment.footprint.iter().flatten() {
                let from = span.0.rem_euclid(tread);
                let across = ((span.1 - span.0) / tread * BINS as f32).ceil() as usize;
                let first = (from / tread * BINS as f32) as usize;
                for bin in 0..=across.min(BINS) {
                    covered[(first + bin) % BINS] = true;
                }
            }
        }
        // The widest free run, on a circle: doubled so a run that wraps the
        // seam is found as one run rather than as two halves.
        let (mut best, mut run) = ((0usize, 0usize), 0usize);
        for bin in 0..BINS * 2 {
            run = if covered[bin % BINS] { 0 } else { run + 1 };
            if run > best.1 {
                best = (bin + 1 - run, run);
            }
        }
        let middle = (best.0 as f32 + best.1 as f32 / 2.0) / BINS as f32 * tread;
        gap = (
            best.1 as f32 / BINS as f32 * tread,
            middle.rem_euclid(tread),
        );
        riser_at.set(Some(number("--step-phase").unwrap_or(gap.1)));
        println!(
            "a staircase of {:.0} mm risers on {:.0} mm treads — one stair per footfall — with \
             the risers at {:.0} mm into each tread, the middle of the {:.0} mm band no sole \
             ever covers{}",
            step * 1000.0,
            tread * 1000.0,
            riser_at.get().unwrap_or(0.0) * 1000.0,
            gap.0 * 1000.0,
            if gap.0 <= 0.0 {
                " — WHICH IS NOTHING: this foot is longer than this tread and cannot stand \
                 clear of a riser at any phase, so every reading below carries an overhang \
                 the gait has no way to avoid"
            } else {
                ""
            },
        );
    }

    println!(
        "stride {:.3} m long, lifting {:.3} m; sole clearances measured above the body's \
         own standing depth ({:.1} mm under the build's floor — the mesh's, not the walk's, #220)",
        stride.length,
        stride.lift,
        standing
            .iter()
            .map(|base| base * -1000.0)
            .fold(0.0f32, f32::max),
    );
    println!(
        "\n{:>5} {:>6}  {:>25} {:>25}  {:>13}  {:>7} {:>7}",
        "frame", "cycle", "HindLeft sole", "HindRight sole", "elbow L / R", "torso", "pelvis"
    );
    println!(
        "{:>5} {:>6}  {:>25} {:>25}  {:>13}  {:>7} {:>7}",
        "",
        "",
        "phase     mm  pitch deg",
        "phase     mm  pitch deg",
        "degrees bent",
        "degrees",
        "mm"
    );

    for frame in 0..frames {
        let moment = measure(frame as f32 / frames as f32, &stride);
        let mut cells = String::new();
        for foot in &moment.feet {
            cells.push_str(&format!(
                " {:>6} {:>10.1} {:>7.1}",
                if foot.airborne { "swing" } else { "stance" },
                foot.lowest * 1000.0,
                foot.pitch,
            ));
        }
        println!(
            "{frame:>5} {:>6.3} {cells}  {:>6.1} {:>6.1}  {:>7.1} {:>7.1}",
            moment.cycle,
            moment.elbows[0],
            moment.elbows[1],
            moment.twist,
            moment.pelvis * 1000.0,
        );
    }

    // The judging pass, at its own resolution — and over **two** cycles rather
    // than one, because a stance wraps. On the default gait `HindRight` goes
    // down at 0.5 and comes up at 1.1, so a single cycle's worth of samples
    // holds two broken halves of that stance and no whole one, and a skid
    // measured across a broken stance is measured across a re-plant. Every
    // body-space reading below is periodic in the cycle and so is unmoved by
    // the extra lap.
    let sweep: Vec<Moment> = (0..samples * 2)
        .map(|at| measure(at as f32 / samples as f32, &stride))
        .collect();

    let mut lowest_swing = f32::MAX;
    let mut lowest_at = 0.0f32;
    let mut mid_swing = f32::MAX;
    let mut stance_err = 0.0f32;
    let mut straightest = f32::MAX;
    let mut deepest_bend = 0.0f32;
    let mut widest_twist = 0.0f32;
    let (mut lean_low, mut lean_high) = (f32::MAX, f32::MIN);
    let (mut bank_low, mut bank_high) = (f32::MAX, f32::MIN);
    let (mut pelvis_low, mut pelvis_high) = (f32::MAX, f32::MIN);
    let (mut stance_pitch, mut swing_pitch) = ((f32::MAX, f32::MIN), (f32::MAX, f32::MIN));
    // The same two readings again, split along the foot — see `Sole::heel`.
    let (mut swing_heel, mut swing_toe) = (f32::MAX, f32::MAX);
    let (mut stance_heel, mut stance_toe) = (f32::MAX, f32::MAX);
    for moment in &sweep {
        for foot in &moment.feet {
            if foot.airborne {
                if foot.lowest < lowest_swing {
                    lowest_swing = foot.lowest;
                    lowest_at = foot.progress;
                }
                if (0.4..=0.6).contains(&foot.progress) {
                    mid_swing = mid_swing.min(foot.lowest);
                }
                swing_heel = swing_heel.min(foot.heel);
                swing_toe = swing_toe.min(foot.toe);
                swing_pitch = (swing_pitch.0.min(foot.pitch), swing_pitch.1.max(foot.pitch));
            } else {
                stance_err = stance_err.max(foot.lowest.abs());
                stance_heel = stance_heel.min(foot.heel);
                stance_toe = stance_toe.min(foot.toe);
                stance_pitch = (
                    stance_pitch.0.min(foot.pitch),
                    stance_pitch.1.max(foot.pitch),
                );
            }
        }
        let bend = moment.elbows.iter().copied().fold(f32::MAX, f32::min);
        straightest = straightest.min(bend);
        deepest_bend = deepest_bend.max(moment.elbows.iter().copied().fold(0.0, f32::max));
        widest_twist = widest_twist.max(moment.twist.abs());
        lean_low = lean_low.min(moment.lean);
        lean_high = lean_high.max(moment.lean);
        bank_low = bank_low.min(moment.bank);
        bank_high = bank_high.max(moment.bank);
        pelvis_low = pelvis_low.min(moment.pelvis);
        pelvis_high = pelvis_high.max(moment.pelvis);
    }
    let both_down = (0..samples)
        .filter(|&at| gait.grounded(at as f32 / samples as f32) >= 2)
        .count() as f32
        / samples as f32;

    // What a PLANTED contact did in the world, over each whole stance the sweep
    // contains.
    //
    // A foot bearing weight is pinned to the ground, so anything it does out
    // here is a slide. Split along the heading the body held at the middle of
    // that stance, because the two components are different defects: a
    // fore-and-aft slide is the stride and the travel disagreeing, and a
    // lateral one is the turn — which is the skate a differential stride exists
    // to prevent, and the axis #250 found this crate could not see at all.
    //
    // Partial runs at either end of the sweep are dropped rather than measured:
    // a stance clipped by the end of the sampling is a stance whose spread is
    // an artefact of where the sampling stopped.
    let mut skid_along = 0.0f32;
    let mut skid_lateral = 0.0f32;
    let mut skid_total = 0.0f32;
    let mut spin = 0.0f32;
    let mut stances = 0usize;
    // **The reading a step needs and a slope never did.** Seating a goal on the
    // ground beneath it is continuous while the ground is; across a riser it is
    // a cliff, and the foot arrives at the top of it in one frame. A
    // penetration reading cannot see that — the foot is above the surface at
    // every sample, it just got there impossibly fast — so the jump is measured
    // directly, as the furthest any contact moved between two samples of the
    // sweep.
    let mut jump = 0.0f32;
    let mut jump_at = 0.0f32;
    // Which foot, and whether it was bearing weight — because a goal that
    // teleports in stance and one that teleports in flight are different
    // defects, and a bare millimetre count cannot be acted on.
    let mut jump_by = (0usize, false);
    for pair in sweep.windows(2) {
        for (foot, (before, after)) in pair[0].planted.iter().zip(&pair[1].planted).enumerate() {
            if before.1.distance(after.1) > jump {
                jump = before.1.distance(after.1);
                jump_at = pair[1].cycle;
                jump_by = (foot, after.0);
            }
        }
    }
    for foot in 0..2 {
        let mut run: Vec<usize> = Vec::new();
        for at in 0..=sweep.len() {
            let down = sweep.get(at).is_some_and(|moment| moment.planted[foot].0);
            if down {
                run.push(at);
                continue;
            }
            // A run touching either end of the sweep was cut by the sampling
            // rather than by a footfall.
            let whole = !run.is_empty() && run[0] > 0 && at < sweep.len();
            if whole {
                stances += 1;
                let mid = run[run.len() / 2];
                let yaw = per_cycle_yaw * (mid as f32 / samples as f32);
                let heading = Vec3::new(yaw.sin(), 0.0, yaw.cos());
                let across = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
                let axis = |direction: Vec3| {
                    let spread = run
                        .iter()
                        .map(|&at| sweep[at].planted[foot].1.dot(direction));
                    let low = spread.clone().fold(f32::MAX, f32::min);
                    let high = spread.fold(f32::MIN, f32::max);
                    high - low
                };
                skid_along = skid_along.max(axis(heading));
                skid_lateral = skid_lateral.max(axis(across));
                // **Horizontally, and that is not a shortcut.** A planted
                // contact rises and falls by design — it is the joint the roll
                // pitches the sole about, so a heel-strike and a toe-off carry
                // it up — and folding that into the skid reported 75 mm of
                // slide on a straight walk where there are 32. A skid is a
                // distance over the GROUND.
                for &a in &run {
                    for &b in &run {
                        let (from, to) = (sweep[a].planted[foot].1, sweep[b].planted[foot].1);
                        let flat = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
                        skid_total = skid_total.max(flat.length());
                    }
                }
                // Unwrapped against the first sample of the stance, since the
                // body's own bearing runs right round on a long enough turn and
                // a raw min-and-max would fold at the seam.
                let first = sweep[run[0]].planted[foot].2;
                let turned = run.iter().map(|&at| {
                    let delta = sweep[at].planted[foot].2 - first;
                    (delta + 180.0).rem_euclid(360.0) - 180.0
                });
                let low = turned.clone().fold(f32::MAX, f32::min);
                let high = turned.fold(f32::MIN, f32::max);
                spin = spin.max(high - low);
            }
            run.clear();
        }
    }

    println!("\nagainst the reference, over {samples} samples of the cycle:");
    println!(
        "  swing:  lowest pass {:.1} mm above standing depth, at swing phase {:.2}; \
         mid-swing bottoms at {:.1} mm",
        lowest_swing * 1000.0,
        lowest_at,
        mid_swing * 1000.0,
    );
    println!(
        "          (this lift's arc starts and ends AT the ground, so only a negative reading \
         is a scuff; what clears it at a phase end is the roll — a foot pitched onto its heel \
         or its toe carries the rest of its sole up. A real swing's tightest pass is 10-30 mm \
         up, mid-swing, because the toe hangs)"
    );
    println!(
        "  stance: soles held within {:.1} mm of standing depth (a plant holds the sole flat \
         at 0; the roll lifts whatever is not bearing weight, so a foot pitched onto its heel \
         or its toe rests on its rim and reads here as a positive. How flat the sole is to \
         begin with is `examples/footaudit`'s measurement, not this one's — quoting a figure \
         for it here is what left this line citing an 11.7 mm convexity that #220 had already \
         fixed)",
        stance_err * 1000.0
    );
    println!(
        "  halves: the worst swing pass was {:.1} mm at the heel and {:.1} mm at the toe; \
         in stance, {:.1} mm and {:.1} mm (each half against the ground beneath ITSELF. On \
         continuous ground the two track each other and this line says nothing; on a \
         staircase they part, and which one is under says whether a foot is buried in the \
         step it stands on or standing on a tread with its toe out over the next riser's \
         column, #245)",
        swing_heel * 1000.0,
        swing_toe * 1000.0,
        stance_heel * 1000.0,
        stance_toe * 1000.0,
    );
    println!(
        "  elbows: never under {straightest:.1} deg bent, peaking {deepest_bend:.1} \
         (reference: ~10-25 at straightest, ~25-45 at peak)"
    );
    println!(
        "  torso:  shoulders led the hips by up to {widest_twist:.1} deg \
         (reference peak: ~5-15)"
    );
    println!(
        "  lean:   trunk pitched {lean_low:.1} to {lean_high:.1} deg forward of its rest \
         carriage (reference: ~2-7 at a normal pace, more as it rises; a body that walks \
         bolt upright reads as a mannequin being carried, #239)"
    );
    // **A run is held to a run's references, not a walk's.** Quoting a walk's
    // band beside a run's reading is how a correct number gets read as a
    // defect, and the two differ on every line that mentions the ground: a
    // runner's centre of mass travels roughly twice as far vertically, spends
    // no time at all on both feet, and rises ABOVE its standing height, which
    // a walking body never does.
    println!(
        "  pelvis: rode {:.1} to {:.1} mm about standing height, {:.1} mm peak-to-peak \
         (reference: {})",
        pelvis_low * 1000.0,
        pelvis_high * 1000.0,
        (pelvis_high - pelvis_low) * 1000.0,
        if gait.has_flight() {
            "~70-100 mm running, and the crest is above standing height"
        } else {
            "~25-50 mm at a natural walking pace"
        }
    );
    println!(
        "  feet:   pitched {:.1} to {:.1} deg in stance, {:.1} to {:.1} in swing \
         (reference: ~15-25 toe-up at heel-strike, ~15-20 toe-down at push-off)",
        stance_pitch.0, stance_pitch.1, swing_pitch.0, swing_pitch.1,
    );
    if turn != 0.0 {
        println!(
            "  bank:   trunk inclined {bank_low:.2} to {bank_high:.2} deg toward its left, \
             against the {:.2} deg the statics ask for (atan of v*w/g — no constant in it; \
             the shortfall is `Speed::of` reading the centreline step, see `bank_of`)",
            (Speed::of(rig, &gait, &stride).metres_per_second(rig) * turn.to_radians() / 9.81)
                .atan()
                .to_degrees(),
        );
    }
    println!(
        "  planted: over {stances} whole stances a bearing foot slid {:.1} mm along its own \
         heading and {:.1} mm across it, {:.1} mm over the ground in all, and was dragged \
         {spin:.1} deg round \
         (asked of the turn: {:.1} deg per stance)",
        skid_along * 1000.0,
        skid_lateral * 1000.0,
        skid_total * 1000.0,
        (per_cycle_yaw * gait.duty).to_degrees().abs(),
    );
    println!(
        "          (a foot bearing weight is pinned to the GROUND, so every millimetre here is \
         a skid. On a straight walk all four must be ~0, and that is a check on the crate \
         rather than on the instrument: the travel comes from the stride's own excursion \
         divided by the duty, so a reading here says the two disagree)"
    );
    // **The heading sweep** (#242): the acceptance test asks for a
    // walkaudit-style read of sole penetration and clearance in every
    // direction, and for no pop crossing from forward to diagonal. The pop is a
    // property of `Heading::reach` and is asserted in the engine's own tests
    // where it can be swept a degree at a time; what this adds is the reading
    // no unit test can take — what the posed body's SOLES do once every layer
    // has run over them.
    if args.iter().any(|arg| arg == "--headings") {
        println!(
            "\n{:>8} {:>18} {:>8} {:>10} {:>10} {:>9}",
            "heading", "", "reach", "stride m", "worst mm", "skid mm"
        );
        for degrees in (0..360).step_by(30) {
            let heading = Heading::degrees(degrees as f32);
            let swept = Stride::for_body(rig, pace).toward(rig, heading);
            // Its own clock, because each heading covers a different distance
            // per cycle, and the outer one was built for `--heading`'s.
            let per_cycle = swept.length / gait.duty;
            let ahead = heading.direction();
            let mut worst = f32::MAX;
            let mut planted = 0.0f32;
            let mut anchor: Option<(usize, Vec3)> = None;
            for at in 0..samples {
                let moment = measure(at as f32 / samples as f32, &swept);
                for foot in &moment.feet {
                    if foot.airborne {
                        worst = worst.min(foot.lowest);
                    }
                }
                // The skid, on the first contact, across whatever stance the
                // sweep happens to catch it in — the same question the turn
                // reading asks and the same answer a straight walk gives.
                // **Back out of the outer clock, then into this heading's.**
                // `Moment::planted` is already carried into the world by the
                // clock `--heading` built, so adding this sweep's travel on top
                // counts it twice — measured, every heading reported about 500
                // mm of skid including the forward one the main reading puts at
                // 31.9. One clock at a time.
                let u = at as f32 / samples as f32;
                let (origin, yaw) = frame(u);
                let local = Quat::from_rotation_y(-yaw) * (moment.planted[0].1 - origin);
                let at_world = local + ahead * (per_cycle * u);
                let down = moment.planted[0].0;
                match (down, anchor) {
                    (true, Some((_, from))) => planted = planted.max(at_world.distance(from)),
                    (true, None) => anchor = Some((at, at_world)),
                    (false, _) => anchor = None,
                }
            }
            println!(
                "{:>7}\u{00b0} {:>18} {:>8.2} {:>10.3} {:>10.1} {:>9.1}",
                degrees,
                heading.describe(),
                heading.reach(),
                swept.length,
                worst * 1000.0,
                planted * 1000.0,
            );
        }
        println!(
            "          (worst mm is the lowest a SWINGING sole passed above the body's own \
             standing depth — negative is a scuff. skid is how far a planted contact moved in \
             the world across one stance, which must not care which way the body is going)"
        );
    }

    println!(
        "  jump:   the furthest a contact moved between two of the {} samples was {:.1} mm, \
         at cycle {jump_at:.2}, by foot {} in {} — {:.2} m/s at this body's cadence",
        samples * 2,
        jump * 1000.0,
        jump_by.0,
        if jump_by.1 { "stance" } else { "flight" },
        jump * samples as f32 * cadence,
    );
    println!(
        "          (a foot travels about a stride per stance, so a smooth walk lands near \
         {:.1} mm a sample here. Much more is the goal being TELEPORTED — which is what \
         seating it on the ground directly beneath it does the moment that ground has a step \
         in it)",
        stride.length / (samples as f32 * gait.duty) * 1000.0,
    );
    println!(
        "  gait:   duty {:.2}, both feet down {both_down:.2} of the cycle, airborne \
         {:.2} (reference: {})",
        gait.duty,
        gait.airborne(),
        if gait.has_flight() {
            "duty ~0.35 at the transition, ~0.22 at a sprint, no double support"
        } else {
            "duty ~0.60, both feet down ~0.20, airborne 0"
        }
    );
}

/// How far the elbow is bent, in degrees away from straight.
///
/// Asked at the joint rather than of the rotation applied to it. A quaternion
/// about the arm's own axis is a perfectly good rotation that bends nothing, and
/// reading the parameter instead of the geometry cannot tell the two apart.
fn elbow_bend(rig: &Rig, posed: &symbios_avatar::anim::Posed, limb: Limb) -> f32 {
    let Some([shoulder, elbow, wrist]) = rig.limb_chain(limb) else {
        return 0.0;
    };
    let upper = posed.positions[shoulder] - posed.positions[elbow];
    let fore = posed.positions[wrist] - posed.positions[elbow];
    let (upper, fore) = (upper.normalize_or_zero(), fore.normalize_or_zero());
    180.0 - upper.dot(fore).clamp(-1.0, 1.0).acos().to_degrees()
}

/// How far the shoulders are turned against the hips, in degrees.
///
/// Positive is the left shoulder forward. Measured between the two lines rather
/// than from either alone, so a body that turns as one — which is a turn and not
/// a swing — reads as zero.
/// How far the trunk is pitched forward of its own rest carriage, in degrees.
///
/// Measured pelvis-to-shoulders on the POSED body and referenced to the same
/// line at rest, for the reason every other column here is: this body is not
/// built standing perfectly upright, so an angle from vertical would report a
/// lean that was never applied. Zero means "carried as it stands".
///
/// Taken from the joint the arms hang off rather than from the neck, because
/// the neck deliberately takes the lean back off again to hold the head level —
/// measuring there would read the head's correction and call it the trunk's
/// posture.
fn trunk_lean(rig: &Rig, posed: &symbios_avatar::anim::Posed) -> f32 {
    let Some(&neck) = rig.in_zone(symbios_avatar::Zone::Neck).first() else {
        return 0.0;
    };
    let Some(girdle) = rig.joints[neck].parent else {
        return 0.0;
    };
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return 0.0;
    };
    let pitch = |run: Vec3| run.z.atan2(run.y).to_degrees();
    pitch(posed.positions[girdle] - posed.positions[root])
        - pitch(rig.joints[girdle].position - rig.joints[root].position)
}

/// How far the trunk is banked toward the body's left, in degrees.
///
/// The lateral partner of [`trunk_lean`], measured the same way and for the
/// same reason: against this body's own rest carriage rather than against
/// vertical, so zero means "carried as it stands".
fn trunk_bank(rig: &Rig, posed: &symbios_avatar::anim::Posed) -> f32 {
    let Some(&neck) = rig.in_zone(symbios_avatar::Zone::Neck).first() else {
        return 0.0;
    };
    let Some(girdle) = rig.joints[neck].parent else {
        return 0.0;
    };
    let Some(root) = rig.joints.iter().position(|joint| joint.parent.is_none()) else {
        return 0.0;
    };
    let roll = |run: Vec3| run.x.atan2(run.y).to_degrees();
    roll(posed.positions[girdle] - posed.positions[root])
        - roll(rig.joints[girdle].position - rig.joints[root].position)
}

fn torso_twist(rig: &Rig, posed: &symbios_avatar::anim::Posed) -> f32 {
    let line = |a: Limb, b: Limb| -> Option<Vec3> {
        let at = |limb: Limb| -> Option<Vec3> {
            let chain = rig.limb_chain(limb)?;
            Some(posed.positions[chain[0]])
        };
        let across = at(b)? - at(a)?;
        Some(Vec3::new(across.x, 0.0, across.z).normalize_or_zero())
    };
    let (Some(shoulders), Some(hips)) = (
        line(Limb::ForeLeft, Limb::ForeRight),
        line(Limb::HindLeft, Limb::HindRight),
    ) else {
        return 0.0;
    };
    // Signed about the body's up axis, so a lead reads the same way through the
    // whole cycle instead of folding at the halfway point.
    let angle = shoulders.cross(hips).dot(Vec3::Y);
    angle.atan2(shoulders.dot(hips)).to_degrees()
}
