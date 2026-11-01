//! A beast's gait against the literature (#378): one row per body and pattern,
//! read off the real [`Driver`] at a steady speed, against the ranges of
//! docs/research/research-08-quadruped-gait.md.
//!
//! ```text
//! cargo run --release --example gaitaudit                  # dog, cat, horse, a cross
//! cargo run --release --example gaitaudit -- --kin dog     # one body (dog, cat, horse, a+b)
//! cargo run --release --example gaitaudit -- --seed 6      # a rolled record
//! ```
//!
//! What each figure is, in the dossier's conventions:
//! - **Fr, lam/h**: the Froude number and the stride over the hip joint's
//!   height at rest, beside Alexander's `2.3 Fr^0.3` on that height (fact 10).
//! - **duty, Hz**: as the driver runs it, not as a table says it should.
//! - **ahead, behind** (LEG-T, fact 3): the line from the leg's proximal pivot
//!   to the paw, against the square to the trunk, at touch-down and lift-off.
//!   The pivot is the hip, or a third of the way down the shoulder blade.
//! - **panto**: for the fore the blade against the forearm, for the hind the
//!   femur against the metatarsus, each as its angle to the vertical in the
//!   side plane: the ROM of each and how far their changes differ at worst
//!   once their mean offset is gone (fact 2: parallel throughout).
//! - **carpus, MCP, hock, MTP**: the included angle (INC), least and most, in
//!   swing and in stance; above 180 is past straight (fact 5).
//! - **leg%**: the pivot-to-paw length, touch-down to lift-off and its least
//!   in swing against its most in stance (fact 6).
//! - **clear**: the lowest point of the swinging foot, pad or toe, at its
//!   highest moment, as a share of the withers height.
//! - **bob**: the rise and fall of the head, the withers and the pelvis as a
//!   share of the withers height, with where in the limb's stance each is
//!   lowest and highest (facts 7, 8). Sampled joints, not the skin.
//! - **spine**: the sagittal and the lateral bend between the trunk's two
//!   ends and its middle, in degrees (fact 9).
//! - **pop**: the most any joint jumps past its own motion in one sample (the
//!   second difference of its path), in millimetres at 240 samples a second:
//!   how a jolt reads, where a joint merely moving fast does not.
//! - **slide**: how far a planted paw moves on the ground between 10 and 60%
//!   of its stance, in millimetres.

use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
use symbios_avatar::anim::{Gait, Lead, Pattern, Phase, Speed};
use symbios_avatar::plan::Zone;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Family, Limb, QuadrupedParams, Vec3};

/// Samples a second: fine enough to find a touch-down within a degree.
const RATE: f32 = 240.0;

/// Whether `--verbose` asks where in the cycle a pop came.
fn args_verbose() -> bool {
    std::env::args().any(|arg| arg == "--verbose")
}

fn family(name: &str) -> Family {
    match name {
        "dog" => Family::Dog,
        "cat" => Family::Cat,
        "horse" => Family::Horse,
        other => panic!("--kin takes dog, cat, horse or a+b, not {other}"),
    }
}

/// A body's sensible speeds, in multiples of `sqrt(g h)`, hip height `h`.
const PACES: [(&str, f32); 4] = [
    ("walk", 0.25),
    ("trot", 1.2),
    ("canter", 3.2),
    ("gallop", 7.0),
];

/// The angle of `v` from straight down in the side plane, positive with its
/// end ahead: the dossier's SEG-V.
fn seg_v(v: Vec3) -> f32 {
    v.z.atan2(-v.y).to_degrees()
}

/// The included angle at a joint from the bone above to the bone below in the
/// side plane, with the joint's flexion rotating the lower bone `backward`
/// (a carpus's, a toe's) or forward (a hock's): 180 straight, above it past
/// straight.
fn included(upper: Vec3, lower: Vec3, flexes_back: bool) -> f32 {
    let turned = seg_v(lower) - seg_v(upper);
    let turned = (turned + 540.0).rem_euclid(360.0) - 180.0;
    if flexes_back {
        180.0 + turned
    } else {
        180.0 - turned
    }
}

#[derive(Clone, Copy)]
struct Span(f32, f32);

impl Span {
    const EMPTY: Self = Self(f32::INFINITY, f32::NEG_INFINITY);
    fn take(&mut self, value: f32) {
        self.0 = self.0.min(value);
        self.1 = self.1.max(value);
    }
    fn rom(self) -> f32 {
        self.1 - self.0
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(out, "{:.0}-{:.0}", self.0, self.1)
    }
}

/// One sample of the body: its cycle, its legs, the head's, withers' and
/// pelvis's heights, and the spine's bends.
type Sample = (f32, Vec<Leg>, [f32; 3], (f32, f32));

/// One sample of one leg.
#[derive(Clone, Copy)]
struct Leg {
    phase: Phase,
    /// The pivot to the paw.
    effective: Vec3,
    /// The blade or the femur; the forearm or the metatarsus.
    pair: (f32, f32),
    /// Carpus or hock; MCP or MTP.
    joints: (f32, f32),
    /// The lowest of the pad and the toe.
    low: f32,
    /// The paw in the world.
    paw: Vec3,
    /// The elbow's or stifle's included angle; the shoulder's or the hip's.
    upper: (f32, f32),
}

fn audit(name: &str, record: &AvatarRecord) {
    let Some(avatar) = Avatar::build(record) else {
        println!("{name}: would not build");
        return;
    };
    let rig = &avatar.rig;
    let first = |zone: Zone| *rig.in_zone(zone).first().expect("a beast's trunk");
    let neck = first(Zone::Neck);
    let withers = rig.joints[neck].parent.expect("a girdle");
    let hips = first(Zone::Pelvis);
    let head = first(Zone::Head);
    let spine: Vec<usize> = {
        let mut joints = vec![withers];
        while let Some(parent) = rig.joints[*joints.last().expect("a joint")].parent {
            joints.push(parent);
        }
        joints.reverse();
        joints
    };
    let middle = spine[spine.len() / 2];
    let upper = |limb: Limb| rig.in_zone(Zone::UpperLimb(limb));
    let hip_h = rig.joints[upper(Limb::HindLeft)[0]].position.y;
    let mesh = &avatar.parts.body;
    let elbow = rig.joints[upper(Limb::ForeLeft)[2]].position;
    let wh = mesh
        .positions
        .iter()
        .filter(|at| (at.z - elbow.z).abs() < 0.02 * rig.extent().max(0.5))
        .map(|at| at.y)
        .fold(0.0f32, f32::max);
    println!(
        "\n{name}: WH {:.3} m, hip {:.3} m ({:.2} WH), speed's measure {:.3} m",
        wh,
        hip_h,
        hip_h / wh,
        Speed::from_froude(1.0).metres_per_second(rig).powi(2) / 9.81
    );
    println!(
        "{:<7}{:>5}{:>6}{:>5}{:>5} {:>10} {:>9} {:>16} {:>5}",
        "", "m/s", "lam/h", "ref", "Hz", "duty", "Fr(hip)", "bob %WH h/w/p", "slide"
    );
    for (label, relative) in PACES {
        let pace = relative.sqrt() * (9.81 * hip_h).sqrt();
        let pattern = match label {
            "walk" => Pattern::Walk,
            "trot" => Pattern::Trot,
            "canter" => Pattern::Canter(Lead::Right),
            _ if rig.anatomy.feet == symbios_avatar::plan::FootKind::Hooves => {
                Pattern::Gallop(Lead::Right)
            }
            _ => Pattern::RotaryGallop(Lead::Right),
        };
        row(
            label,
            rig,
            pace,
            pattern,
            (hip_h, wh),
            (withers, hips, head, middle),
        );
    }
}

fn row(
    label: &str,
    rig: &symbios_avatar::Rig,
    pace: f32,
    pattern: Pattern,
    (hip_h, wh): (f32, f32),
    (withers, hips, head, middle): (usize, usize, usize, usize),
) {
    let step = 1.0 / RATE;
    let mut driver = Driver::seeded(7);
    let mut at = Vec3::ZERO;
    let mut samples: Vec<Sample> = Vec::new();
    let mut strained = 0usize;
    let mut before: (Option<Vec<Vec3>>, Option<Vec<Vec3>>) = (None, None);
    let (mut pop, mut pop_joint) = (0.0f32, 0usize);
    let mut pop_at: Vec<Phase> = Vec::new();
    let upper = |limb: Limb| rig.in_zone(Zone::UpperLimb(limb));
    let foot = |limb: Limb| rig.in_zone(Zone::Extremity(limb));
    let lower = |limb: Limb| rig.in_zone(Zone::LowerLimb(limb))[0];
    let mut gait: Option<Gait> = None;
    let mut speed = None;
    let mut last = 0.0f32;
    let mut cycles = 0;
    for frame in 0..(RATE as usize * 30) {
        let velocity = Vec3::Z * pace;
        at += velocity * step;
        let driven = driver
            .drive(
                rig,
                &Inputs {
                    delta: step,
                    velocity,
                    at,
                    pattern: Some(pattern),
                    ..Inputs::default()
                },
                level_ground,
            )
            .expect("an unheld body is posed");
        if frame < RATE as usize * 5 {
            continue;
        }
        let cycle = driver.cycle();
        if cycle < last {
            cycles += 1;
        }
        last = cycle;
        if cycles == 0 {
            continue;
        }
        if cycles > 2 {
            break;
        }
        let now = driver.speed().expect("a travelling body has a speed");
        let g = now.gait(rig);
        let posed = driven.pose.forward(rig).positions;
        if let (Some(one), Some(two)) = (&before.0, &before.1) {
            for joint in 0..posed.len() {
                let jolt: Vec3 = posed[joint] - one[joint] * 2.0 + two[joint];
                if jolt.length() > pop {
                    pop = jolt.length();
                    pop_joint = joint;
                    pop_at = (0..g.len()).map(|index| g.phase(index, cycle)).collect();
                }
            }
        }
        before = (Some(posed.clone()), before.0.take());
        let legs: Vec<Leg> = g
            .limbs
            .iter()
            .enumerate()
            .map(|(index, &limb)| {
                let up = upper(limb);
                let (pad, toe) = (posed[foot(limb)[0]], posed[foot(limb)[1]]);
                let low = lower(limb);
                let (pivot, pair, joints, upper_angles) = if limb.is_fore() {
                    let (top, shoulder, elbow) = (posed[up[0]], posed[up[1]], posed[up[2]]);
                    let carpus = posed[low];
                    (
                        top + (shoulder - top) / 3.0,
                        (seg_v(shoulder - top), seg_v(carpus - elbow)),
                        (
                            included(carpus - elbow, pad - carpus, true),
                            included(pad - carpus, toe - pad, true),
                        ),
                        (
                            (shoulder - elbow)
                                .angle_between(carpus - elbow)
                                .to_degrees(),
                            (top - shoulder)
                                .angle_between(elbow - shoulder)
                                .to_degrees(),
                        ),
                    )
                } else {
                    let (hip, stifle) = (posed[up[0]], posed[up[1]]);
                    let hock = posed[low];
                    (
                        hip,
                        (seg_v(stifle - hip), seg_v(pad - hock)),
                        (
                            (stifle - hock).angle_between(pad - hock).to_degrees(),
                            included(pad - hock, toe - pad, true),
                        ),
                        (
                            (hip - stifle).angle_between(hock - stifle).to_degrees(),
                            (posed[hips] - hip).angle_between(stifle - hip).to_degrees(),
                        ),
                    )
                };
                Leg {
                    phase: g.phase(index, cycle),
                    effective: pad - pivot,
                    pair,
                    joints,
                    low: pad.y.min(toe.y),
                    paw: at + pad,
                    upper: upper_angles,
                }
            })
            .collect();
        let bends = {
            let (a, b, c) = (posed[hips], posed[middle], posed[withers]);
            let (rear, front) = (b - a, c - b);
            (
                (front.y.atan2(front.z) - rear.y.atan2(rear.z)).to_degrees(),
                (front.x.atan2(front.z) - rear.x.atan2(rear.z)).to_degrees(),
            )
        };
        strained += usize::from(driven.straining > 0);
        samples.push((
            cycle,
            legs,
            [posed[head].y, posed[withers].y, posed[hips].y],
            bends,
        ));
        gait = Some(g);
        speed = Some(now);
    }
    let (Some(gait), Some(speed)) = (gait, speed) else {
        println!("{label}: no samples");
        return;
    };
    let cycle_m = speed.cycle_length(rig);
    let lam = cycle_m / hip_h;
    let fr = pace * pace / (9.81 * hip_h);
    let mut bob = Vec::new();
    for channel in 0..3 {
        let mut span = Span::EMPTY;
        for sample in &samples {
            span.take(sample.2[channel]);
        }
        bob.push(span.rom() / wh * 100.0);
    }
    // Slide.
    let mut slide = 0.0f32;
    for index in 0..gait.len() {
        let mut planted: Option<Vec3> = None;
        for sample in &samples {
            let leg = sample.1[index];
            match leg.phase {
                Phase::Stance(t) if (0.1..0.6).contains(&t) => {
                    let was = *planted.get_or_insert(leg.paw);
                    slide =
                        slide.max(Vec3::new(leg.paw.x - was.x, 0.0, leg.paw.z - was.z).length());
                }
                _ => planted = None,
            }
        }
    }
    let tail = format!(
        " pop {:.1} mm at {:?} {}{}",
        pop * 1000.0,
        rig.joints[pop_joint].zone,
        if args_verbose() {
            format!("{:?} {pop_at:?}", gait.limbs)
        } else {
            String::new()
        },
        if strained > 0 {
            format!("  STRAINED {strained}")
        } else {
            String::new()
        }
    );
    println!(
        "{:<7}{:>5.2}{:>6.2}{:>5.2}{:>5.2} {:>10.3} {:>9.2} {:>5.1}/{:>4.1}/{:>4.1} {:>6.1}{tail}",
        label,
        pace,
        lam,
        2.3 * fr.powf(0.3),
        speed.cadence(rig),
        gait.duty,
        fr,
        bob[0],
        bob[1],
        bob[2],
        slide * 1000.0,
    );
    // Bob phase: where in a fore's and a hind's stance each channel is lowest
    // and highest.
    let stance_of = |fore: bool| {
        gait.limbs
            .iter()
            .position(|limb| limb.is_fore() == fore && limb.is_left())
            .expect("a limb")
    };
    let phase_of = |channel: usize, of: usize| {
        let (mut low, mut high) = ((f32::INFINITY, 0.0), (f32::NEG_INFINITY, 0.0));
        for sample in &samples {
            if let Phase::Stance(t) = sample.1[of].phase {
                let y = sample.2[channel];
                if y < low.0 {
                    low = (y, t);
                }
                if y > high.0 {
                    high = (y, t);
                }
            }
        }
        (low.1 * 100.0, high.1 * 100.0)
    };
    let harmonic = |channel: usize, n: f32| {
        let (mut re, mut im, mut count) = (0.0f32, 0.0f32, 0.0f32);
        let mean = samples.iter().map(|s| s.2[channel]).sum::<f32>() / samples.len() as f32;
        for sample in &samples {
            let angle = std::f32::consts::TAU * n * sample.0;
            re += (sample.2[channel] - mean) * angle.cos();
            im += (sample.2[channel] - mean) * angle.sin();
            count += 1.0;
        }
        (2.0 * re / count, 2.0 * im / count)
    };
    let describe = |channel: usize| {
        let (one, two) = (harmonic(channel, 1.0), harmonic(channel, 2.0));
        (
            one.0.hypot(one.1) / wh * 100.0,
            two.0.hypot(two.1) / wh * 100.0,
            two.1.atan2(two.0) / std::f32::consts::TAU,
        )
    };
    let (hd, wi, pe) = (describe(0), describe(1), describe(2));
    let (wlo, whi) = phase_of(1, stance_of(true));
    let (hlo, hhi) = phase_of(0, stance_of(true));
    let (plo, phi) = phase_of(2, stance_of(false));
    println!(
        "  bob@stance% low-high  head {hlo:.0}-{hhi:.0} (fore)  withers {wlo:.0}-{whi:.0} (fore)  pelvis {plo:.0}-{phi:.0} (hind); h1/h2 %WH head {:.1}/{:.1} withers {:.1}/{:.1} pelvis {:.1}/{:.1}; withers-pelvis h2 phase {:.2} stride",
        hd.0,
        hd.1,
        wi.0,
        wi.1,
        pe.0,
        pe.1,
        ((wi.2 - pe.2) * 0.5).rem_euclid(0.5),
    );
    let mut spine = (Span::EMPTY, Span::EMPTY);
    for sample in &samples {
        spine.0.take(sample.3.0);
        spine.1.take(sample.3.1);
    }
    println!(
        "  spine ROM sagittal {:.1} lateral {:.1} deg",
        spine.0.rom(),
        spine.1.rom()
    );
    for fore in [true, false] {
        let index = stance_of(fore);
        let (mut td, mut to) = (None, None);
        let (mut first_t, mut last_t) = (f32::INFINITY, f32::NEG_INFINITY);
        let mut joints = [Span::EMPTY; 4];
        let mut uppers = [Span::EMPTY; 2];
        let mut pair = (Span::EMPTY, Span::EMPTY);
        let (mut offsets, mut stance_len, mut swing_len) = (Vec::new(), Span::EMPTY, Span::EMPTY);
        let mut clear = 0.0f32;
        for sample in &samples {
            let leg = sample.1[index];
            pair.0.take(leg.pair.0);
            pair.1.take(leg.pair.1);
            offsets.push(leg.pair.0 - leg.pair.1);
            uppers[0].take(leg.upper.0);
            uppers[1].take(leg.upper.1);
            let length = leg.effective.length();
            match leg.phase {
                Phase::Stance(t) => {
                    joints[2].take(leg.joints.0);
                    joints[3].take(leg.joints.1);
                    stance_len.take(length);
                    if t < first_t {
                        first_t = t;
                        td = Some(leg.effective);
                    }
                    if t > last_t {
                        last_t = t;
                        to = Some(leg.effective);
                    }
                }
                Phase::Swing(_) => {
                    joints[0].take(leg.joints.0);
                    joints[1].take(leg.joints.1);
                    swing_len.take(length);
                    clear = clear.max(leg.low);
                }
            }
        }
        let ahead = |v: Vec3| v.z.atan2(-v.y).to_degrees();
        let mean = offsets.iter().sum::<f32>() / offsets.len() as f32;
        let worst = offsets
            .iter()
            .map(|o| (o - mean).abs())
            .fold(0.0f32, f32::max);
        let (td_v, to_v) = (td.unwrap_or(Vec3::NEG_Y), to.unwrap_or(Vec3::NEG_Y));
        println!(
            "  {} ahead {:>3.0} behind {:>3.0} | panto {:>2.0}/{:>2.0} worst {:>2.0} | {} sw {:>7} st {:>7} | {} sw {:>7} st {:>7} | elbow/stifle {} {} {} | leg {:+.1}% sw/st {:.0}% | clear {:.1}%WH",
            if fore { "fore" } else { "hind" },
            ahead(td_v),
            -ahead(to_v),
            pair.0.rom(),
            pair.1.rom(),
            worst,
            if fore { "carpus" } else { "hock" },
            joints[0].to_string(),
            joints[2].to_string(),
            if fore { "MCP" } else { "MTP" },
            joints[1].to_string(),
            joints[3].to_string(),
            uppers[0],
            if fore { "shldr" } else { "hip" },
            uppers[1],
            (to_v.length() / td_v.length() - 1.0) * 100.0,
            swing_len.0 / stance_len.1 * 100.0,
            clear / wh * 100.0,
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let beast = |own: Family, other: Option<Family>| {
        let mut params = QuadrupedParams::default();
        params.set_kind(own, other, if other.is_some() { 0.5 } else { 0.0 });
        AvatarRecord::new("Gait", Archetype::Quadruped(params))
    };
    if let Some(seed) = value("--seed").and_then(|seed| seed.parse::<i64>().ok()) {
        let mut record =
            AvatarRecord::new("Gait", Archetype::Quadruped(QuadrupedParams::default()));
        record.reroll(seed);
        audit(&format!("seed {seed}"), &record);
        return;
    }
    if let Some(kin) = value("--kin") {
        let record = match kin.split_once('+') {
            Some((own, other)) => beast(family(own), Some(family(other))),
            None => beast(family(&kin), None),
        };
        audit(&kin, &record);
        return;
    }
    for (name, own, other) in [
        ("dog", Family::Dog, None),
        ("cat", Family::Cat, None),
        ("horse", Family::Horse, None),
        ("cat+horse", Family::Cat, Some(Family::Horse)),
    ] {
        audit(name, &beast(own, other));
    }
}
