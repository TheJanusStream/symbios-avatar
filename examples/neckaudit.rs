//! The column between the shoulders and the jaw, measured axis-free.
//!
//! Every figure here is a bisection against `PolyMesh::contains`, the same
//! primitive `tests/parts.rs` judges a buried feature with.
//!
//! **The depth is the reading to trust and the reaches are the reading that
//! lied.** Back-over-forward measured from `z = 0` reported 2.50 against the
//! reference's 2.43 after the neck was leaned back, which looked like the whole
//! defect closed by one constant and was the probe reading the OFFSET rather
//! than the surface. So the reaches below are printed against the neck node's
//! OWN z, and the depth — front minus back, which no axis enters — is printed
//! beside them.
//!
//! What it is for, beyond looking: the neck's section is swept off its own joint
//! and the arithmetic says the THROAT should not move for it. This is what
//! checks that, and it earned its keep the first time it was run — the throat
//! had come 9 mm forward and the chest 20, because the socket ring a joint hull
//! opens for a limb blended that limb's depth without its offset.
//!
//! **And it says how long the neck reads, broken into who owns that length.**
//! The table at the end runs
//! `tests/plan::the_neck_is_the_length_of_a_neck`'s own arithmetic over that
//! test's own seeds — so its ratio column can be checked against a shipped
//! assertion — and then splits the chin-to-shoulder span four ways. That split
//! is the answer to why tuning the neck moves it so little: the
//! neck bone owns two to five millimetres of it.
//!
//! ```text
//! cargo run --release --example neckaudit
//! cargo run --release --example neckaudit -- 7
//! ```

use symbios_avatar::face::Skull;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3, Zone};

/// The seeds `tests/plan::the_neck_is_the_length_of_a_neck` asserts over.
///
/// The table below reproduces that test's own arithmetic, so these have to be
/// its seeds and not a nicer set: an instrument that cannot be checked against
/// a shipped assertion is the one this project keeps being lied to by.
const GUARDED: [i64; 5] = [0, 3, 7, 13, 21];

fn main() {
    let seed: Option<i64> = std::env::args().nth(1).and_then(|arg| arg.parse().ok());
    let mut record = AvatarRecord::new("Necked", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let avatar = Avatar::build(&record).expect("a biped builds");
    let mesh = &avatar.parts.body;
    let rig = &avatar.rig;

    let head = *rig.in_zone(Zone::Head).first().expect("a head");
    let neck = rig.joints[head].parent.expect("a head sits on a neck");
    let joint = rig.joints[head].position;
    let axis = rig.joints[neck].position;

    // Outward from a point known to be inside, in metres.
    let reach = |from: Vec3, along: Vec3| -> Option<f32> {
        if !mesh.contains(from) {
            return None;
        }
        let (mut near, mut far) = (0.0f32, 0.4f32);
        if mesh.contains(from + along * far) {
            return None;
        }
        for _ in 0..40 {
            let middle = (near + far) * 0.5;
            if mesh.contains(from + along * middle) {
                near = middle;
            } else {
                far = middle;
            }
        }
        Some(near)
    };

    println!(
        "seed {}   head joint y {:.1} mm   neck joint y {:.1} mm, z {:.1} mm",
        seed.map_or("default".to_string(), |seed| seed.to_string()),
        joint.y * 1000.0,
        axis.y * 1000.0,
        axis.z * 1000.0,
    );
    println!("  y below the head joint, and everything in millimetres");
    println!("      y    half-width     ahead    behind     DEPTH");

    let mut widths: Vec<f32> = Vec::new();
    let mut step = 0;
    while step <= 44 {
        let y = joint.y - 0.005 * step as f32;
        let from = Vec3::new(0.0, y, axis.z);
        let (Some(ahead), Some(behind)) = (reach(from, Vec3::Z), reach(from, -Vec3::Z)) else {
            step += 1;
            continue;
        };
        // **The width is a maximum over the slice, not a ray from the axis, and
        // the difference is the trap this issue has already paid for once.** A
        // section swept off-centre is still an ellipse; a ray fired sideways
        // from the joint crosses it on a CHORD, so it reads narrower the further
        // the mass moves, and reports a column pinching that is not.
        let across = (0..=20)
            .filter_map(|slice| {
                let z = axis.z - behind + (ahead + behind) * slice as f32 / 20.0;
                reach(Vec3::new(0.0, y, z), Vec3::X)
            })
            .fold(0.0f32, f32::max);
        widths.push(across);
        println!(
            "  {:6.1}   {:9.1}   {:7.1}   {:7.1}   {:7.1}",
            (y - joint.y) * 1000.0,
            across * 1000.0,
            ahead * 1000.0,
            behind * 1000.0,
            (ahead + behind) * 1000.0,
        );
        step += 1;
    }

    // Swell, pinch, swell is what the reference does not do: its half-width
    // grows all the way out into the shoulder, monotonically. A turn is a
    // reversal in the width, and it is counted with a DEADBAND because the
    // surface ripples a few tenths of a millimetre between rows and a turn under
    // half a millimetre over five of height is not a swell — it is #47.
    const DEADBAND: f32 = 0.0005;
    let mut turns = 0;
    let mut rising: Option<bool> = None;
    let mut mark = widths[0];
    for &width in &widths[1..] {
        if (width - mark).abs() < DEADBAND {
            continue;
        }
        let now = width > mark;
        if rising.is_some_and(|was| was != now) {
            turns += 1;
        }
        rising = Some(now);
        mark = width;
    }
    println!(
        "  turns down the column, past a {:.1} mm deadband: {turns}",
        DEADBAND * 1000.0
    );

    visible_neck();
    ratio();
    junction();
}

/// The frame axis's four stations, and the body that caught the nape.
///
/// The first four are the stations every throat render of milestone #10 was
/// judged at; the fifth is the heavy masculine body on which the owner found
/// the nape's hump after the first four read clean (#302), which is the
/// argument for a guard reading more than the axis.
const JUNCTIONS: [(f32, f32); 5] = [
    (-1.5, 0.0),
    (-1.0, 0.0),
    (0.0, 0.0),
    (1.0, 0.0),
    (-1.5, 2.0),
];

/// The collar and the trapezius, which the column readings above cannot see.
///
/// **Every figure here is a silhouette, bisected, read down a height ladder
/// from the column into the shoulder** — the collar (#301) and the trapezius
/// (#302) were both judged on exactly that silhouette in the throat close-up,
/// and a guard that reads the same line is a guard that can disagree with the
/// render on the render's own terms. `tests/throat.rs` asserts these; this
/// prints them, with the reasoning the bounds were fitted against.
///
/// - OVERHANG, coronal: how much wider the body is at some height than it is
///   10 mm below that height, at worst, over the band from the column's foot
///   into the shoulder. A turtleneck collar is a rim — wider above than below
///   — and reads positive; a shoulder that only widens going down reads at or
///   under zero. The masculine collar of #301 was about 10 mm of it.
/// - OVERHANG, sagittal: the same question asked of the back line, which is
///   where the nape's hump and ledge lived (#302): the back reaching further
///   behind at some height than 10 mm below it.
/// - TURN: the sharpest bend in the back line, as the angle between
///   successive 10 mm segments of it. A ledge is a pair of sharp turns; a
///   trapezius is a gentle one.
/// - SLOPE: the angle of the shoulder line against horizontal, from where the
///   silhouette leaves the column to the acromion. `examples/reference`'s
///   shoulder table reads the same angle off the CC0 mannequins.
/// - WAIST: the column's narrowest half-width in the UPPER half of its run,
///   against the skull's widest. The ratio `ratio()` above prints takes the
///   narrowest anywhere down to the girdle's crown, and since #302 that
///   minimum lives in the trapezius's blend at the column's foot — a ruler
///   that reads the foot and calls it the neck. This one reads the neck.
fn junction() {
    println!();
    println!("the junction: collar, nape and trapezius, by silhouette (#300)");
    println!("  fem   mass   overhang coronal  overhang sagittal   turn    slope   waist/skull");
    println!("                 (mm, + is a rim)   (mm, + is a ledge)  (deg)   (deg)");
    for (femininity, mass) in JUNCTIONS {
        let mut record = AvatarRecord::new("Junction", Archetype::default());
        record.composites.femininity = femininity;
        record.composites.mass = mass;
        record.composites.sanitize();
        record.sanitize();
        let Some(avatar) = Avatar::build(&record) else {
            continue;
        };
        let Some(read) = Junction::measure(&avatar) else {
            println!("  {femininity:+.1}  {mass:+.1}   no junction measured");
            continue;
        };
        println!(
            "  {femininity:+.1}  {mass:+.1}   {:15.1}  {:17.1}  {:6.1}  {:6.1}   {:.3}",
            read.collar * 1000.0,
            read.nape * 1000.0,
            read.turn,
            read.slope,
            read.waist,
        );
    }
}

/// The junction's five readings. See [`junction`].
///
/// **Duplicated in `tests/throat.rs` on purpose**, as `visible_neck` is
/// duplicated from `tests/plan.rs`: the test is the contract and this is the
/// instrument, and the day they disagree the instrument is the one that is
/// wrong — which is only checkable if the numbers can be held side by side.
struct Junction {
    collar: f32,
    nape: f32,
    turn: f32,
    slope: f32,
    waist: f32,
}

impl Junction {
    fn measure(avatar: &Avatar) -> Option<Self> {
        let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
        let head = *rig.in_zone(Zone::Head).first()?;
        let neck = rig.joints[head].parent?;
        let girdle = rig.joints[neck].parent?;
        let axis = rig.joints[neck].position;
        let crown = rig.joints[girdle].position.y + rig.joints[girdle].radius;
        let skull = Skull::measure(mesh, rig)?;
        let at = rig.joints[head].position;
        let chin = at.y + skull.chin();
        // The acromion: the girdle's lateral child.
        let (reach, acromion) = rig
            .joints
            .iter()
            .enumerate()
            .filter(|(index, joint)| *index != neck && joint.parent == Some(girdle))
            .map(|(_, joint)| (joint.position.x.abs(), joint.position.y + joint.radius))
            .filter(|(reach, _)| *reach > rig.joints[girdle].radius * 0.5)
            .max_by(|a, b| a.0.total_cmp(&b.0))?;

        // Outward from a point on the column's axis, bisected against the
        // surface. `None` when the axis point is already outside.
        let out = |from: Vec3, along: Vec3| -> Option<f32> {
            if !mesh.contains(from) {
                return None;
            }
            let (mut near, mut far) = (0.0f32, 0.45f32);
            if mesh.contains(from + along * far) {
                return None;
            }
            for _ in 0..30 {
                let middle = (near + far) * 0.5;
                if mesh.contains(from + along * middle) {
                    near = middle;
                } else {
                    far = middle;
                }
            }
            Some(near)
        };
        // The widest chord of the section at `y`, swept over its own depth —
        // the silhouette an eye reads, not a ray from an axis the section
        // does not sit on.
        let wide = |y: f32| -> Option<f32> {
            let from = Vec3::new(axis.x, y, axis.z);
            let (ahead, behind) = (out(from, Vec3::Z)?, out(from, -Vec3::Z)?);
            (0..=16)
                .filter_map(|slice| {
                    let z = axis.z - behind + (ahead + behind) * slice as f32 / 16.0;
                    out(Vec3::new(axis.x, y, z), Vec3::X)
                })
                .reduce(f32::max)
        };
        // How far the back reaches behind the column's axis at `y`.
        let back = |y: f32| -> Option<f32> { out(Vec3::new(axis.x, y, axis.z), -Vec3::Z) };

        // The ladder: 2 mm steps from 60 mm above the crown to the acromion.
        const STEP: f32 = 0.002;
        const LOOK: usize = 5; // 10 mm, in steps
        let top = crown + 0.06;
        let mut widths = Vec::new();
        let mut backs = Vec::new();
        let mut y = top;
        while y > acromion - 0.02 {
            widths.push(wide(y));
            backs.push(back(y));
            y -= STEP;
        }
        let overhang = |ladder: &[Option<f32>]| -> f32 {
            ladder
                .iter()
                .enumerate()
                .filter_map(|(i, above)| {
                    let above = (*above)?;
                    let below = ladder.get(i + LOOK).copied().flatten()?;
                    Some(above - below)
                })
                .fold(f32::MIN, f32::max)
        };
        let collar = overhang(&widths);
        let nape = overhang(&backs);

        // The back line's sharpest bend: successive 10 mm segments as vectors
        // in the sagittal plane, the angle between them.
        let mut turn = 0.0f32;
        for i in 0..backs.len().saturating_sub(2 * LOOK) {
            let (Some(a), Some(b), Some(c)) = (backs[i], backs[i + LOOK], backs[i + 2 * LOOK])
            else {
                continue;
            };
            let rise = STEP * LOOK as f32;
            let first = glam::Vec2::new(b - a, -rise);
            let second = glam::Vec2::new(c - b, -rise);
            turn = turn.max(first.angle_to(second).abs().to_degrees());
        }

        // The slope: from where the silhouette first stands 10 mm wider than
        // the column's foot, down to the acromion's top at its reach.
        let foot = wide(crown + 0.03)?;
        let leave = widths
            .iter()
            .enumerate()
            .find_map(|(i, w)| (w.is_some_and(|w| w > foot + 0.01)).then_some(i))?;
        let (leave_y, leave_x) = (top - STEP * leave as f32, widths[leave]?);
        let slope = ((leave_y - acromion) / (reach - leave_x).max(1e-3))
            .atan()
            .to_degrees();

        // The waist, read in the upper half of the run from the chin to the
        // crown, against the skull's widest above the chin.
        let mut skull_wide = 0.0f32;
        let mut y = chin;
        while y < at.y + skull.throat_and_crown().1 {
            skull_wide = skull_wide.max(wide(y).unwrap_or(0.0));
            y += 0.005;
        }
        let mut narrowest = f32::MAX;
        let mut y = chin;
        while y > chin - (chin - crown) * 0.5 {
            if let Some(w) = wide(y) {
                narrowest = narrowest.min(w);
            }
            y -= 0.002;
        }
        Some(Self {
            collar,
            nape,
            turn,
            slope,
            waist: narrowest / skull_wide.max(f32::EPSILON),
        })
    }
}

/// What a neck is worth against the skull it carries, across the axes that move
/// the two independently.
///
/// **The one number the stump is a symptom of.** `neck_r` is a fraction of
/// stature times girth times the frame axis; `head_r` is a fraction of stature
/// times `head_size`. They share stature and nothing else, so the column and the
/// skull can disagree by a whole axis and no constant anywhere says what they
/// ought to be worth to each other. This walks the grid and prints the spread.
///
/// Both figures are bisected against the built surface and both are the widest
/// chord of their own section, so they are the same measurement asked at two
/// heights: the skull's widest anywhere above the jaw, and the column's
/// narrowest below it.
///
/// **The reference pair cannot source a target for this.** The anchor is the
/// neutral body's own reading.
fn ratio() {
    println!();
    println!("neck against skull, over the axes that move them apart (#175)");
    println!("  head_size  mass  fem    skull    neck   ratio     node  skull/node");

    for &head_size in &[-1.0f32, 0.0, 1.0] {
        for &(mass, femininity) in &[(0.0f32, 0.0f32), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0)] {
            let mut record = AvatarRecord::new("Ratio", Archetype::default());
            if let Archetype::Humanoid(ref mut params) = record.archetype {
                params.head_size = head_size;
            }
            record.composites.mass = mass;
            record.composites.femininity = femininity;
            record.composites.sanitize();
            record.sanitize();
            let Some(avatar) = Avatar::build(&record) else {
                continue;
            };
            let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
            let Some(skull) = Skull::measure(mesh, rig) else {
                continue;
            };
            let head = *rig.in_zone(Zone::Head).first().expect("a head");
            let at = rig.joints[head].position;
            let (throat, crown) = skull.throat_and_crown();

            // The widest chord of the section at `y`, swept over its own depth:
            // a ray fired sideways from the axis crosses an off-centre section
            // on a chord, which is the trap the ladder above already records.
            let widest = |y: f32| -> f32 {
                let deep = |dir: Vec3| -> f32 {
                    let (mut inside, mut outside) = (0.0f32, 0.40f32);
                    for _ in 0..32 {
                        let middle = 0.5 * (inside + outside);
                        if mesh.contains(Vec3::new(at.x, y, at.z) + dir * middle) {
                            inside = middle;
                        } else {
                            outside = middle;
                        }
                    }
                    inside
                };
                let (ahead, behind) = (deep(Vec3::Z), deep(-Vec3::Z));
                (0..=20)
                    .map(|slice| {
                        let z = at.z - behind + (ahead + behind) * slice as f32 / 20.0;
                        let (mut inside, mut outside) = (0.0f32, 0.40f32);
                        for _ in 0..32 {
                            let middle = 0.5 * (inside + outside);
                            if mesh.contains(Vec3::new(at.x + middle, y, z)) {
                                inside = middle;
                            } else {
                                outside = middle;
                            }
                        }
                        inside
                    })
                    .fold(0.0f32, f32::max)
            };

            // **The narrowest is hunted from the CHIN, not from the head's
            // floor** (#175). The floor is where the head's SURFACE stops, and
            // the column's waist is well above it: measured on the default
            // body the waist sits 80 mm under the head joint and the floor at
            // 135, so a search that starts at the floor starts below the neck
            // and reports the shoulders. That is not a bad ruler for the region
            // it names — it is a ruler pointed at the wrong region, and it read
            // a carve that had moved the waist by 20 mm as having done nothing.
            //
            // From the chin down to the girdle's crown is what an eye calls the
            // neck, and the jaw between them is wider than both, so a minimum
            // over the whole run finds the waist without needing a landmark to
            // separate them.
            let chin = at.y + skull.chin();
            let neck = *rig.in_zone(Zone::Neck).first().expect("a neck");
            let girdle = rig.joints[neck].parent.expect("a neck sits on a girdle");
            let crown_of_girdle = rig.joints[girdle].position.y + rig.joints[girdle].radius;
            let mut widest_skull = 0.0f32;
            let mut y = chin;
            while y < at.y + crown {
                widest_skull = widest_skull.max(widest(y));
                y += 0.002;
            }
            let mut narrowest = f32::MAX;
            let mut y = chin;
            while y > crown_of_girdle {
                narrowest = narrowest.min(widest(y));
                y -= 0.002;
            }
            let _ = throat;
            let node = rig.joints[head].radius * rig.joints[head].scale.x;
            println!(
                "  {head_size:+.1}      {mass:+.1}  {femininity:+.1}   {:6.1}  {:6.1}   {:.3}   {:6.1}  {:.3}",
                widest_skull * 1000.0,
                narrowest * 1000.0,
                narrowest / widest_skull.max(f32::EPSILON),
                node * 1000.0,
                widest_skull / node.max(f32::EPSILON)
            );
        }
    }
}

/// How much neck an eye actually sees: the chin down to the shoulder line.
///
/// This is `tests/plan::the_neck_is_the_length_of_a_neck`'s arithmetic, run for
/// reading rather than for asserting — the same bisected half-width, the same
/// "half again as wide as the narrowest point" rule, the same five seeds. It is
/// duplicated rather than shared because the test is the contract and this is
/// an instrument: the day they disagree, the instrument is the one that is
/// wrong, and that is only checkable if the numbers can be held side by side.
///
/// The reference figures it is read against: the
/// Quaternius pair land on 0.13 and 0.14, and the eight-head canon this crate
/// quotes elsewhere puts the shoulder line about a third of a head under the
/// chin, so 0.33.
///
/// **The span is broken into its four owners, and that is the reading that
/// matters.** Tuning the neck itself moves this span very little, and the
/// columns below say why: the neck BONE's own contribution is the few
/// millimetres by which its
/// length exceeds the girdle's radius, because the girdle's crown sits directly
/// under the neck joint by construction. Most of what an eye reads as neck is
/// head-owned surface hanging below the chin, and the rest is the girdle.
fn visible_neck() {
    println!();
    println!("visible neck, chin to the shoulder line, by the guard's own rule");
    println!("  seed      head       neck     ratio   (canon 0.33, reference 0.13)");
    println!("           and who owns the span: chin to the head's own floor,");
    println!("           that floor to the neck joint, the neck joint to the");
    println!("           girdle's crown, and the crown down to the shoulder line");

    // **The default body leads the table, and its absence was a hole** (#6).
    // Every other instrument in this crate reads the shipped body first; this
    // one read only the guard's five rolled seeds, so the body the project
    // judges most often had no row in the one table that says how long a neck
    // reads. It was found by the #6 re-judgement asking the question and having
    // to answer it with a rolled seed.
    for seed in std::iter::once(None).chain(GUARDED.map(Some)) {
        let mut record = AvatarRecord::new("Necked", Archetype::default());
        let seed = match seed {
            Some(seed) => {
                record.reroll(seed);
                seed.to_string()
            }
            None => "def".to_string(),
        };
        let avatar = Avatar::build(&record).expect("a biped builds");
        let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
        let Some(skull) = Skull::measure(mesh, rig) else {
            println!("  {seed:5}   no skull measured");
            continue;
        };
        let at = rig.joints[*rig.in_zone(Zone::Head).first().expect("a head")].position;
        let (throat, crown) = skull.throat_and_crown();
        let (chin, crown, throat) = (at.y + skull.chin(), at.y + crown, at.y + throat);

        // Bisected against the surface, never binned: binning vertices into
        // height bands reports ripple that is not in the mesh.
        let half_width = |y: f32| {
            let (mut inside, mut outside) = (0.0f32, 0.40f32);
            for _ in 0..32 {
                let middle = 0.5 * (inside + outside);
                if mesh.contains(Vec3::new(at.x + middle, y, at.z)) {
                    inside = middle;
                } else {
                    outside = middle;
                }
            }
            inside
        };

        let (mut narrowest, mut y) = (f32::MAX, throat);
        while y > throat - 0.30 {
            narrowest = narrowest.min(half_width(y));
            if half_width(y) > narrowest * 1.5 {
                break;
            }
            y -= 0.001;
        }

        // The four owners of the span, top to bottom. `throat` is where the
        // head's own SURFACE stops, so the first is head-owned and the rest is
        // the body's; the girdle's crown is its joint plus its radius, since a
        // node's section scales its width and its depth and never its height.
        let neck_joint = rig.joints[rig.joints[*rig.in_zone(Zone::Head).first().expect("a head")]
            .parent
            .expect("a head sits on a neck")];
        let girdle = rig.joints[neck_joint.parent.expect("a neck sits on a girdle")];
        let crown_of_girdle = girdle.position.y + girdle.radius;

        println!(
            "  {seed:5}   {:6.1} mm  {:6.1} mm   {:.3}    {:5.1} + {:5.1} + {:5.1} + {:5.1}",
            (crown - chin) * 1000.0,
            (chin - y) * 1000.0,
            (chin - y) / (crown - chin),
            (chin - throat) * 1000.0,
            (throat - neck_joint.position.y) * 1000.0,
            (neck_joint.position.y - crown_of_girdle) * 1000.0,
            (crown_of_girdle - y) * 1000.0,
        );
    }
}
