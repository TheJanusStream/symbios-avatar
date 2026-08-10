//! Measures the built body against the Quaternius reference mannequins, and
//! measures how a limb actually bends.
//!
//! `examples/measure` compares the body against the eight-head figure of
//! academic drawing, which is a *drawing* convention: it says where a knee sits
//! but not how thick a thigh is, and it has nothing at all to say about
//! skinning. The reference columns here come from a pair of production game
//! bodies instead — the Quaternius male and female mannequins shipped with
//! mesh2motion (CC0) — measured off the GLB rather than remembered, so every
//! figure below is a number somebody's animator signed off on.
//!
//! Two things get measured, because the body has two separate complaints
//! against it and they have nothing to do with each other.
//!
//! **Proportions.** Landmark heights, segment lengths and spans, all as
//! fractions of the *rendered* height, beside the same figure from each
//! reference. That is the table that says whether an arm is short.
//!
//! **The bend.** How much of a limb moves when exactly one joint of it turns.
//! A hinge moves the segment past the joint and nothing else; a body whose
//! weights are spread — or attached to the wrong end of a bone — drags the
//! segment *before* the joint along too, and the limb reads as a rope rather
//! than as two bones. The reference mannequin's weights cross over inside a
//! tenth of a bone length and never touch the grandparent bone at all, which is
//! the standard this prints against.
//!
//! The four [`SkinConfig`] fields are overridable from the command line, so the
//! binding can be swept against the reference figures without editing the
//! library and rebuilding between every guess.
//!
//! ```text
//! cargo run --example bodyaudit
//! cargo run --example bodyaudit -- --seed 7
//! cargo run --example bodyaudit -- --classic 2000   # averaged over plausible rolls
//! cargo run --example bodyaudit -- --reach 1.4 --smooth 1
//! ```

use glam::{Quat, Vec3};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Limb, Pose, Rig, SkinConfig,
    SkinWeights, Zone, build_cage, catmull_clark, mesh::PolyMesh, rig::skin,
};

/// A landmark's height as a fraction of stature, measured off each reference.
///
/// Male and female are listed separately rather than averaged because several
/// of these differ by more than the tolerance anyone would judge the body
/// against — the pelvis by 1.7% of stature, the upper arm by a quarter of its
/// own length — and an average would hide exactly the axis a body needs.
/// **Only `pelvis` and `knee` are strictly like for like.** Those two name the
/// same anatomy in both rigs. `head` and `neck` do not: the reference's head
/// bone sits at the base of the skull and its `neck_01` at the top of the
/// spine, where this plan's are the centres of a head node and a neck node.
/// Their rows are printed because the *trend* is still worth seeing, and
/// flagged because the offset is not an error figure. The head has its own
/// milestone and its own instruments.
const HEIGHTS: [(&str, Zone, f32, f32); 6] = [
    ("head ~", Zone::Head, 0.8575, 0.8673),
    ("neck ~", Zone::Neck, 0.7940, 0.8030),
    ("chest ~", Zone::Chest, 0.7007, 0.7187),
    ("waist ~", Zone::Abdomen, 0.6236, 0.6450),
    ("pelvis", Zone::Pelvis, 0.5013, 0.5179),
    ("knee", Zone::UpperLimb(Limb::HindLeft), 0.2876, 0.2900),
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed = args
        .iter()
        .position(|arg| arg == "--seed")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse::<i64>().ok());

    let number = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse::<f32>().ok())
    };
    let default = SkinConfig::default();
    let config = SkinConfig {
        reach: number("--reach").unwrap_or(default.reach),
        falloff: number("--falloff").unwrap_or(default.falloff),
        smoothing_iterations: number("--smooth")
            .map_or(default.smoothing_iterations, |value| value as usize),
        smoothing_strength: number("--strength").unwrap_or(default.smoothing_strength),
    };
    if let Some(count) = args
        .iter()
        .position(|arg| arg == "--classic")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse::<usize>().ok())
    {
        population(count, &config);
        return;
    }

    let mut record = AvatarRecord::new("Audited", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let Some((rig, mesh, weights, height, floor)) = build(&record, &config) else {
        eprintln!("the body would not build");
        std::process::exit(1);
    };

    println!("rendered height {height:.3} m");
    println!("reference: Quaternius male 1.830 m, female 1.806 m (CC0, via mesh2motion)");
    println!(
        "skin: reach {:.2}  falloff {:.2}  smoothing {}x{:.2}",
        config.reach, config.falloff, config.smoothing_iterations, config.smoothing_strength
    );
    // The single number that says how spread the binding is, and the one the
    // reference publishes: 1.88 bones per vertex, 85% of them on two or fewer.
    let spread: Vec<usize> = weights
        .vertices
        .iter()
        .map(|influences| influences.iter().filter(|i| i.weight > 0.001).count())
        .collect();
    let mean = spread.iter().sum::<usize>() as f32 / spread.len().max(1) as f32;
    let slim = spread.iter().filter(|&&n| n <= 2).count() as f32 / spread.len().max(1) as f32;
    println!(
        "influences/vertex {mean:.2} (reference 1.88), on 2 bones or fewer {:.0}% \
         (reference 85%)\n",
        slim * 100.0
    );

    proportions(&rig, height, floor);
    silhouette(&Trunk::measure(&rig, &mesh, &weights, height, floor));
    thickness(&rig, &mesh, &weights, height);
    prebend(&rig, height);
    locality(&rig, &mesh, &weights, height);
    hinge(&rig, &mesh, &weights);
}

/// How thick each limb actually is, against the reference and against the node
/// radius that asked for it.
///
/// The distinction in the last clause is the whole point of this table. A node
/// radius is what the plan *requested*; subdivision delivers a good deal less,
/// and by different amounts for a hulled joint than for a tube of 4-point
/// rings. Every figure in `humanoid.rs`'s radius ladder is a request, and every
/// figure measured off a reference body is a surface, so the two have never
/// been in the same units. The `kept` column is the conversion, measured rather
/// than assumed — tune the ladder without it and you are sweeping against a
/// number you cannot see.
///
/// Measured as the mean perpendicular distance to the bone axis, over the
/// vertices the bone's own joint holds, which is exactly how the reference
/// columns were measured off the GLB.
fn thickness(rig: &Rig, mesh: &PolyMesh, weights: &SkinWeights, height: f32) {
    /// Where along each bone the thickness is sampled.
    const STATIONS: [f32; 4] = [0.125, 0.375, 0.625, 0.875];
    /// `(bone, male, female)` at each station, as fractions of stature.
    const REFERENCE: [(&str, [f32; 4], [f32; 4]); 4] = [
        (
            "upper arm",
            [0.0369, 0.0347, 0.0342, 0.0263],
            [0.0304, 0.0296, 0.0267, 0.0237],
        ),
        (
            "forearm",
            [0.0263, 0.0317, 0.0281, 0.0181],
            [0.0238, 0.0281, 0.0242, 0.0167],
        ),
        (
            "thigh",
            [0.0442, 0.0440, 0.0400, 0.0285],
            [0.0445, 0.0490, 0.0404, 0.0322],
        ),
        (
            "shank",
            [0.0250, 0.0366, 0.0298, 0.0183],
            [0.0286, 0.0342, 0.0270, 0.0184],
        ),
    ];

    println!("\nlimb thickness: mean radius at each station, fractions of H");
    println!(
        "{:<11} {:>6} {:>25} {:>25}   {:>10}",
        "bone", "", "ours / male / female", "", "nominal  kept"
    );

    let mut bones = Vec::new();
    for limb in [Limb::ForeLeft, Limb::HindLeft] {
        if let Some([root, mid, tip]) = rig.limb_chain(limb) {
            bones.push((root, mid));
            bones.push((mid, tip));
        }
    }

    for (index, (from, to)) in bones.into_iter().enumerate() {
        let Some((name, male, female)) = REFERENCE.get(index).copied() else {
            break;
        };
        let (start, end) = (rig.joints[from].position, rig.joints[to].position);
        let axis = end - start;
        let span = axis.length();
        if span <= f32::EPSILON {
            continue;
        }
        let along = axis / span;

        let mut measured = [f32::NAN; 4];
        for (slot, station) in STATIONS.iter().enumerate() {
            let mut total = 0.0;
            let mut count = 0usize;
            for (vertex, &at) in mesh.positions.iter().enumerate() {
                let held: f32 = weights.vertices[vertex]
                    .iter()
                    .filter(|influence| influence.joint as usize == from)
                    .map(|influence| influence.weight)
                    .sum();
                if held <= 0.4 {
                    continue;
                }
                let travel = (at - start).dot(along) / span;
                if (travel - station).abs() >= 0.125 {
                    continue;
                }
                total += (at - (start + axis * travel)).length();
                count += 1;
            }
            if count >= 5 {
                measured[slot] = total / count as f32 / height;
            }
        }

        // The conversion from request to surface, taken at the proximal station
        // against the node whose radius governs there.
        let nominal = rig.joints[from].radius / height;
        let kept = measured[0] / nominal;
        print!("{name:<11}");
        for slot in 0..4 {
            if measured[slot].is_nan() {
                print!("   --   /{:.4}/{:.4}", male[slot], female[slot]);
            } else {
                print!(
                    " {:.4}/{:.4}/{:.4}",
                    measured[slot], male[slot], female[slot]
                );
            }
        }
        if kept.is_finite() {
            println!("   {nominal:.4}  {:.0}%", kept * 100.0);
        } else {
            println!("   {nominal:.4}    --");
        }
    }
}

/// Landmark heights, segment lengths and spans against both references.
fn proportions(rig: &Rig, height: f32, floor: f32) {
    println!(
        "{:<14} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "landmark", "ours", "male", "female", "off-M", "off-F"
    );
    for (name, zone, male, female) in HEIGHTS {
        let joints = rig.in_zone(zone);
        let Some(&joint) = (if zone.is_core() {
            joints.first()
        } else {
            joints.last()
        }) else {
            continue;
        };
        let at = (rig.joints[joint].position.y - floor) / height;
        println!(
            "{name:<14} {at:>7.4} {male:>7.4} {female:>7.4} {:>+8.4} {:>+8.4}",
            at - male,
            at - female
        );
    }

    // Segment lengths, which is where the arm's shortfall lives. Heights alone
    // cannot show it: an arm can hang from the right shoulder and still be a
    // fifth too short.
    let length = |a: usize, b: usize| rig.joints[a].position.distance(rig.joints[b].position);
    let arm = rig.limb_chain(Limb::ForeLeft);
    let leg = rig.limb_chain(Limb::HindLeft);
    println!(
        "\n{:<14} {:>7} {:>7} {:>7} {:>8} {:>8}",
        "segment", "ours", "male", "female", "off-M", "off-F"
    );
    let row = |name: &str, ours: f32, male: f32, female: f32| {
        let ours = ours / height;
        println!(
            "{name:<14} {ours:>7.4} {male:>7.4} {female:>7.4} {:>+8.4} {:>+8.4}",
            ours - male,
            ours - female
        );
    };
    if let Some([shoulder, elbow, wrist]) = arm {
        row("upper arm", length(shoulder, elbow), 0.1621, 0.1293);
        row("forearm", length(elbow, wrist), 0.1529, 0.1549);
    }
    if let Some([hip, knee, ankle]) = leg {
        row("thigh", length(hip, knee), 0.2223, 0.2366);
        row("shank", length(knee, ankle), 0.2368, 0.2398);
    }

    // Spans are compared by *where the arm leaves the body*, not by bone name.
    // Both rigs have something called a clavicle and they are not the same
    // anatomy: the reference's runs from beside the sternum out to the
    // shoulder and sits 0.021 H apart, while this plan's is already out at the
    // shoulder. Pairing the two names put our 0.238 beside their 0.190 and made
    // a 78% error look like 25%.
    // Measured at the *root* of each limb chain — the shoulder and the hip —
    // and not by taking the widest joint of the zone, which is the elbow. That
    // read 0.53 against the reference's 0.19 and looked like a threefold error
    // in the shoulders; nearly all of it was an A-posed arm being measured
    // against a T-posed one.
    let root_span = |limb: Limb| {
        rig.limb_chain(limb)
            .map_or(0.0, |chain| rig.joints[chain[0]].position.x.abs() * 2.0)
    };
    println!();
    row("shoulder joints", root_span(Limb::ForeLeft), 0.1899, 0.1560);
    row("hip joints", root_span(Limb::HindLeft), 0.0973, 0.0986);
}

/// `(band start, reference half-width, reference half-depth)`, all of H.
///
/// The Quaternius male, measured off the GLB with its own skin weights used to
/// drop the T-posed arms out of the shoulder bands — which is the step that
/// makes the figure a torso rather than a wingspan.
///
/// **Re-derived by `examples/reference` and one figure was wrong** (#173).
/// These were measured once by hand and written down, and nothing could
/// reproduce them until that example existed. Reading them off the GLB a second
/// way agrees on all nine widths and on eight of the nine depths; the last
/// band's depth was 0.0663 and measures 0.0715, which is 7.8% and is the
/// difference between our top band reading 9% shallow and 15%. The same pass
/// corrected the thigh, the shank and the female upper arm by smaller amounts.
///
/// **The table stops below the reference's own widest trunk band, and that is
/// worth knowing before quoting the top row.** Run out to the crown, the male
/// keeps widening past 0.72 to 0.0965 at 0.75–0.78 — its shoulder shelf, which
/// sits where ours does — so this last row is a climb read as a peak. Widening
/// the table is #100's, since it is the female column that makes it worth
/// having.
const TRUNK: [(f32, f32, f32); 9] = [
    (0.45, 0.0717, 0.0504),
    (0.48, 0.0910, 0.0551),
    (0.51, 0.0878, 0.0614),
    (0.54, 0.0779, 0.0616),
    (0.57, 0.0734, 0.0592),
    (0.60, 0.0706, 0.0558),
    (0.63, 0.0678, 0.0571),
    (0.66, 0.0799, 0.0643),
    (0.69, 0.0911, 0.0715),
];

/// The two [`TRUNK`] bands the coat-hanger ratio used to be quoted at.
///
/// Kept because this issue's history is written in them — `0.66..0.69` is where
/// the reference reads 2.38 — and printed under the girdle-anchored figure
/// rather than instead of it. See [`Trunk::hanger_at_girdle`] for why a fixed
/// band is the weaker instrument of the two.
const HANGER: [usize; 2] = [7, 8];

/// The reference's coat-hanger ratio: its shoulder span over its own trunk
/// half-width where its shoulder mass sits, `0.1899 / 0.0911`.
const HANGER_REFERENCE: f32 = 0.1899 / 0.0911;

/// One body's trunk, in the form an average over a population needs.
///
/// Everything here is a fraction of the body's own **rendered** height, which
/// is the whole reason this is a struct rather than a print: rendered height
/// moves whenever the head or the neck does, so a band figure from one build
/// cannot be compared with a band figure from another unless both say what they
/// are a fraction of. #106 carried a stale table for four days on exactly that.
#[derive(Clone, Copy)]
struct Trunk {
    /// Rendered height, in metres.
    height: f32,
    /// Half-width of each [`TRUNK`] band, arms removed.
    width: [f32; 9],
    /// Half-depth of each [`TRUNK`] band, arms removed.
    depth: [f32; 9],
    /// Distance between the two shoulder joints — where the arm chain begins.
    shoulder_span: f32,
    /// Distance between the two hip joints.
    hip_span: f32,
    /// Height of the shoulder girdle's own node.
    girdle_at: f32,
    /// Half-width of the trunk in a band centred on the girdle.
    girdle_width: f32,
}

impl Trunk {
    /// Measures one built body.
    ///
    /// The one comparison that survives two rigs disagreeing about what a bone
    /// is called. Both columns are measured the same way — every vertex the arms
    /// do not hold, bucketed by height, reporting the half-width and half-depth
    /// of what is left — so a torso can be judged without either skeleton being
    /// consulted. A band the mesh does not populate comes back `NaN` rather than
    /// zero, so an average can drop it instead of averaging it in.
    fn measure(rig: &Rig, mesh: &PolyMesh, weights: &SkinWeights, height: f32, floor: f32) -> Self {
        let arm: Vec<bool> = (0..rig.len())
            .map(|joint| match rig.joints[joint].zone {
                Zone::UpperLimb(limb) | Zone::LowerLimb(limb) | Zone::Extremity(limb) => {
                    limb.is_fore()
                }
                _ => false,
            })
            .collect();

        let band = |low: f32| {
            let mut span = (f32::MAX, f32::MIN);
            let mut deep = (f32::MAX, f32::MIN);
            let mut count = 0usize;
            for (vertex, &at) in mesh.positions.iter().enumerate() {
                let up = (at.y - floor) / height;
                if up < low || up >= low + 0.03 {
                    continue;
                }
                let held: f32 = weights.vertices[vertex]
                    .iter()
                    .filter(|influence| arm[influence.joint as usize])
                    .map(|influence| influence.weight)
                    .sum();
                if held > 0.25 {
                    continue;
                }
                span = (span.0.min(at.x), span.1.max(at.x));
                deep = (deep.0.min(at.z), deep.1.max(at.z));
                count += 1;
            }
            if count >= 8 {
                (
                    (span.1 - span.0) * 0.5 / height,
                    (deep.1 - deep.0) * 0.5 / height,
                )
            } else {
                (f32::NAN, f32::NAN)
            }
        };

        let mut width = [f32::NAN; 9];
        let mut depth = [f32::NAN; 9];
        for (slot, &(low, _, _)) in TRUNK.iter().enumerate() {
            (width[slot], depth[slot]) = band(low);
        }

        // The girdle stands above every band in the table — 0.773 of stature on
        // the default body, against a table that stops at 0.72 — so the ratio
        // #106 is judged by has to go and find it rather than assume a band. It
        // is the highest joint of the chest zone standing on the midline: the
        // node both clavicles hang off.
        let girdle_at = rig
            .in_zone(Zone::Chest)
            .iter()
            .filter(|&&joint| rig.joints[joint].position.x.abs() < 1e-4)
            .map(|&joint| (rig.joints[joint].position.y - floor) / height)
            .fold(f32::NAN, f32::max);
        let (girdle_width, _) = band(girdle_at - 0.015);

        let root_span = |limb: Limb| {
            rig.limb_chain(limb).map_or(f32::NAN, |chain| {
                rig.joints[chain[0]].position.x.abs() * 2.0
            }) / height
        };
        Self {
            height,
            width,
            depth,
            shoulder_span: root_span(Limb::ForeLeft),
            hip_span: root_span(Limb::HindLeft),
            girdle_at,
            girdle_width,
        }
    }

    /// The coat-hanger ratio at one [`TRUNK`] band.
    fn hanger(&self, band: usize) -> f32 {
        self.shoulder_span / self.width[band]
    }

    /// The coat-hanger ratio at the girdle, which is the one to judge by.
    ///
    /// **The number that separates the two failures this axis sits between.** A
    /// span that grows because the body under it grew is a trunk; a span that
    /// grows on its own is a coat hanger, and both read as "wider shoulders" in
    /// a span figure alone. #106's decision is bought only if this comes DOWN
    /// while [`Self::shoulder_span`] goes up.
    ///
    /// Taken at each body's own girdle height rather than at a fixed band,
    /// because the two bodies do not carry their shoulders at the same fraction
    /// of stature — ours at 0.773, against a reference whose shoulder mass fills
    /// its 0.69–0.72 band — and a fixed band compares a ribcage against a
    /// trapezius. That mismatch is most of why the top band of the table reads
    /// as a pinch. The figure it prints against is [`HANGER_REFERENCE`].
    fn hanger_at_girdle(&self) -> f32 {
        self.shoulder_span / self.girdle_width
    }
}

/// The trunk's silhouette, band by band, with the arms taken off.
fn silhouette(trunk: &Trunk) {
    println!("\ntrunk silhouette with the arms removed, fractions of H");
    println!(
        "{:>11} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "band", "width", "ref", "off", "depth", "ref", "off"
    );
    for (slot, (low, want_wide, want_deep)) in TRUNK.into_iter().enumerate() {
        if trunk.width[slot].is_nan() {
            continue;
        }
        println!(
            "{low:>7.2}-{:.2} {:>8.4} {want_wide:>8.4} {:>+7.1}% {:>8.4} {want_deep:>8.4} {:>+7.1}%",
            low + 0.03,
            trunk.width[slot],
            (trunk.width[slot] / want_wide - 1.0) * 100.0,
            trunk.depth[slot],
            (trunk.depth[slot] / want_deep - 1.0) * 100.0,
        );
    }

    println!("\ncoat hanger: shoulder span over the trunk's own half-width there");
    println!(
        "{:>11} {:>8} {:>8} {:>8} {:>8}",
        "band", "span", "half-w", "ratio", "ref"
    );
    println!(
        "{:>5.3}girdle {:>7.4} {:>8.4} {:>8.3} {HANGER_REFERENCE:>8.3}",
        trunk.girdle_at,
        trunk.shoulder_span,
        trunk.girdle_width,
        trunk.hanger_at_girdle(),
    );
    for slot in HANGER {
        let (low, want_wide, _) = TRUNK[slot];
        println!(
            "{low:>7.2}-{:.2} {:>8.4} {:>8.4} {:>8.3} {:>8.3}",
            low + 0.03,
            trunk.shoulder_span,
            trunk.width[slot],
            trunk.hanger(slot),
            0.1899 / want_wide,
        );
    }
}

/// Builds one record into everything the tables are measured off.
///
/// Returns the rig, the subdivided mesh, its binding, the rendered height and
/// the floor the bands are counted from, or `None` if the body does not build.
fn build(
    record: &AvatarRecord,
    config: &SkinConfig,
) -> Option<(Rig, PolyMesh, SkinWeights, f32, f32)> {
    let skeleton = record.skeleton();
    let cage = build_cage(&skeleton, &CageConfig::default()).ok()?;
    let mesh = catmull_clark(&cage, BODY_SUBDIVISIONS);
    let rig = Rig::from_skeleton(&skeleton).ok()?;
    let weights = skin::bind(&mesh, &rig, config);
    let (low, high) = mesh.bounds();
    Some((rig, mesh, weights, (high.y - low.y).max(1e-3), low.y))
}

/// The trunk averaged over the rolled bodies that are plausible bodies.
///
/// **A rolled seed is no longer a sample of a person** (#171). Generator 2's
/// wildcard tail reaches the whole exploration envelope by design, so seed 13
/// renders at 0.49 m and seed 7 at 2.35 m, and averaging a proportion across raw
/// seeds averages in caricatures. This filters first, the way
/// `the_neck_is_the_length_of_a_neck` already does: **every axis this ruler
/// reads has to be inside ±1**, and for the trunk that is not only the axes that
/// set its width. `neck_length` and `head_size` are in the list because every
/// figure here is a fraction of RENDERED height and those two move it — the trap
/// that made #106 carry a stale table for four days. Stature is in it too, by
/// its own rule: it is a length rather than a sigma, so what it has to be inside
/// is `humanoid_height_range`.
///
/// The kept sample's own height spread is printed with it, because an average
/// over bodies of very different statures is still an average of ratios and the
/// reader is entitled to see how wide that was.
fn population(count: usize, config: &SkinConfig) {
    let (short, tall) = symbios_avatar::plan::humanoid_height_range();
    let mut kept = Vec::new();
    let mut wild = 0usize;
    let mut failed = 0usize;
    for seed in 0..count as i64 {
        let mut record = AvatarRecord::new("Audited", Archetype::default());
        record.reroll(seed);
        let Archetype::Humanoid(params) = &record.archetype else {
            continue;
        };
        let classic = (short..=tall).contains(&params.height)
            && [
                params.build,
                params.muscle,
                params.shoulder_width,
                params.hip_width,
                params.limb_length,
                params.neck_length,
                params.head_size,
            ]
            .iter()
            .all(|axis| axis.abs() <= 1.0);
        if !classic {
            wild += 1;
            continue;
        }
        match build(&record, config) {
            Some((rig, mesh, weights, height, floor)) => {
                kept.push(Trunk::measure(&rig, &mesh, &weights, height, floor));
            }
            None => failed += 1,
        }
    }

    println!(
        "classic sweep: {count} seeds rolled, {wild} wild, {failed} would not build, {} kept",
        kept.len()
    );
    if kept.is_empty() {
        return;
    }
    let mean = |of: &dyn Fn(&Trunk) -> f32| {
        let taken: Vec<f32> = kept
            .iter()
            .map(of)
            .filter(|value| value.is_finite())
            .collect();
        (
            taken.iter().sum::<f32>() / taken.len().max(1) as f32,
            taken.len(),
        )
    };
    let (height, _) = mean(&|trunk: &Trunk| trunk.height);
    let low = kept
        .iter()
        .map(|trunk| trunk.height)
        .fold(f32::MAX, f32::min);
    let high = kept
        .iter()
        .map(|trunk| trunk.height)
        .fold(f32::MIN, f32::max);
    println!("rendered height mean {height:.3} m, spread {low:.3}..{high:.3} m");

    println!("\ntrunk silhouette, mean over the kept bodies, fractions of each body's own H");
    println!(
        "{:>11} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>5}",
        "band", "width", "ref", "off", "depth", "ref", "off", "n"
    );
    for (slot, (start, want_wide, want_deep)) in TRUNK.into_iter().enumerate() {
        let (wide, seen) = mean(&|trunk: &Trunk| trunk.width[slot]);
        let (deep, _) = mean(&|trunk: &Trunk| trunk.depth[slot]);
        if seen == 0 {
            continue;
        }
        println!(
            "{start:>7.2}-{:.2} {wide:>8.4} {want_wide:>8.4} {:>+7.1}% {deep:>8.4} {want_deep:>8.4} {:>+7.1}% {seen:>5}",
            start + 0.03,
            (wide / want_wide - 1.0) * 100.0,
            (deep / want_deep - 1.0) * 100.0,
        );
    }

    let (shoulder, _) = mean(&|trunk: &Trunk| trunk.shoulder_span);
    let (hip, _) = mean(&|trunk: &Trunk| trunk.hip_span);
    println!(
        "\nshoulder joints {shoulder:.4} against male 0.1899 ({:+.1}%)",
        (shoulder / 0.1899 - 1.0) * 100.0
    );
    println!(
        "hip joints      {hip:.4} against male 0.0973 ({:+.1}%)",
        (hip / 0.0973 - 1.0) * 100.0
    );
    println!("\ncoat hanger: shoulder span over the trunk's own half-width there");
    println!("{:>11} {:>8} {:>8}", "band", "ratio", "ref");
    let (girdle, _) = mean(&Trunk::hanger_at_girdle);
    let (at, _) = mean(&|trunk: &Trunk| trunk.girdle_at);
    println!("{at:>5.3}girdle {girdle:>17.3} {HANGER_REFERENCE:>8.3}");
    for slot in HANGER {
        let (start, want_wide, _) = TRUNK[slot];
        let (ratio, _) = mean(&|trunk: &Trunk| trunk.hanger(slot));
        println!(
            "{start:>7.2}-{:.2} {ratio:>17.3} {:>8.3}",
            start + 0.03,
            0.1899 / want_wide
        );
    }
}

/// How far each limb's middle joint stands off the line through its ends.
///
/// This is the quantity [`Rig::bend_pole`] reads to decide which way a knee
/// folds, and on the humanoid plan it is exactly zero — so the pole falls back
/// to a rule about limb names rather than being measured from the body. The
/// reference mannequin carries a real one: its knee sits 42 mm forward of the
/// hip-to-ankle line, which is a tenth of the thigh.
fn prebend(rig: &Rig, height: f32) {
    println!(
        "\n{:<14} {:>9} {:>9} {:>10}",
        "limb", "standoff", "of H", "reference"
    );
    for (limb, reference) in [(Limb::ForeLeft, 0.0034f32), (Limb::HindLeft, 0.0230)] {
        let Some(chain) = rig.limb_chain(limb) else {
            continue;
        };
        let [root, mid, tip] = chain.map(|joint| rig.joints[joint].position);
        let line = (tip - root).normalize_or(Vec3::Y);
        let offset = mid - root;
        let standoff = (offset - line * offset.dot(line)).length();
        println!(
            "{:<14} {:>8.1}mm {:>9.4} {:>10.4}",
            format!("{limb:?}"),
            standoff * 1000.0,
            standoff / height,
            reference
        );
    }
}

/// How each bone's influence is spread along its own length.
///
/// Vertices are bucketed by where they project onto the bone, and each bucket
/// reports the weight it gives the bone's own joint, its parent, and its child.
/// The reference reads 1.000 through the middle of a bone, crosses over within
/// a tenth of the joint, and gives the grandparent nothing whatever.
fn locality(rig: &Rig, mesh: &PolyMesh, weights: &SkinWeights, height: f32) {
    for (limb, name) in [(Limb::HindLeft, "thigh"), (Limb::ForeLeft, "upper arm")] {
        let Some([root, mid, tip]) = rig.limb_chain(limb) else {
            continue;
        };
        let (start, end) = (rig.joints[root].position, rig.joints[mid].position);
        let axis = end - start;
        let span = axis.length();
        if span <= f32::EPSILON {
            continue;
        }

        println!(
            "\n{name}: weight along the bone from {:?} (t=0) to {:?} (t=1)",
            rig.joints[root].zone, rig.joints[mid].zone
        );
        println!(
            "{:>12} {:>8} {:>8} {:>8} {:>7}",
            "t-band", "w[root]", "w[mid]", "w[tip]", "n"
        );

        // Only vertices actually on this limb: near the bone, and not so far
        // out that a hip band scoops up the other leg.
        let radius = rig.joints[root].radius.max(rig.joints[mid].radius);
        for step in -1..12 {
            let low = step as f32 / 10.0;
            let (mut sum, mut count) = ([0.0f32; 3], 0usize);
            for (vertex, &position) in mesh.positions.iter().enumerate() {
                let along = (position - start).dot(axis) / span / span;
                if along < low || along >= low + 0.1 {
                    continue;
                }
                let off = (position - (start + axis * along)).length();
                if off > radius * 2.0 {
                    continue;
                }
                for influence in &weights.vertices[vertex] {
                    let joint = influence.joint as usize;
                    if joint == root {
                        sum[0] += influence.weight;
                    } else if joint == mid {
                        sum[1] += influence.weight;
                    } else if joint == tip {
                        sum[2] += influence.weight;
                    }
                }
                count += 1;
            }
            if count < 4 {
                continue;
            }
            let n = count as f32;
            println!(
                "{:>5.1}..{:>4.1} {:>8.3} {:>8.3} {:>8.3} {:>7}",
                low,
                low + 0.1,
                sum[0] / n,
                sum[1] / n,
                sum[2] / n,
                count
            );
        }
        let _ = height;
    }
}

/// Turns exactly one joint and reports what moved.
///
/// The question a proportion table cannot answer. Bending a knee should carry
/// the shank and leave the thigh where it was; whatever fraction of the thigh's
/// displacement survives is the limb behaving like a rope. Printed for both the
/// joint's own segment and the one before it, so the ratio between them is the
/// finding rather than either number alone.
fn hinge(rig: &Rig, mesh: &PolyMesh, weights: &SkinWeights) {
    println!("\nbending one joint at a time (mean vertex travel, mm)");
    println!(
        "{:<22} {:>12} {:>12} {:>9}",
        "turned", "before joint", "after joint", "leak"
    );

    for (limb, name) in [(Limb::HindLeft, "knee"), (Limb::ForeLeft, "elbow")] {
        let Some([root, mid, tip]) = rig.limb_chain(limb) else {
            continue;
        };
        let mut pose = Pose::rest(rig);
        // A half-radian bend about the body's lateral axis: a knee lifting, an
        // elbow closing. The exact axis does not matter to the question being
        // asked, only that one joint and no other has turned.
        pose.rotations[mid] = Quat::from_rotation_x(0.5);
        let moved = pose.forward(rig).deform(rig, &mesh.positions, weights);

        let mut travel = [0.0f32; 2];
        let mut counts = [0usize; 2];
        for (index, segment) in [(0usize, (root, mid)), (1, (mid, tip))] {
            let (start, end) = (
                rig.joints[segment.0].position,
                rig.joints[segment.1].position,
            );
            let axis = end - start;
            let span = axis.length_squared();
            if span <= f32::EPSILON {
                continue;
            }
            let radius = rig.joints[segment.0]
                .radius
                .max(rig.joints[segment.1].radius);
            for (vertex, &position) in mesh.positions.iter().enumerate() {
                let along = (position - start).dot(axis) / span;
                // The middle of the segment only: the bands next to the joint
                // are meant to move, and including them answers a different
                // question.
                if !(0.25..0.75).contains(&along) {
                    continue;
                }
                if (position - (start + axis * along)).length() > radius * 2.0 {
                    continue;
                }
                travel[index] += position.distance(moved[vertex]);
                counts[index] += 1;
            }
        }

        let before = travel[0] / counts[0].max(1) as f32 * 1000.0;
        let after = travel[1] / counts[1].max(1) as f32 * 1000.0;
        println!(
            "{:<22} {before:>11.1} {after:>11.1} {:>8.0}%",
            format!("{name} ({limb:?})"),
            if after > 0.0 {
                before / after * 100.0
            } else {
                0.0
            }
        );
    }

    // The other half of the trade. Concentrating weight stops the leak and
    // starts a crease, so what a hard fold does to the surface is measured
    // beside it: tightening the falloff on the leak figure alone would buy a
    // hinge and pay for it with an elbow that pinches shut.
    //
    // **Measured as surface area, because the obvious measure is inert.** The
    // first attempt averaged each vertex's distance to the joint and printed a
    // flat 100.0% for every configuration from reach 0.9 to 2.6. That was not a
    // result: rotating one joint leaves the joint itself where it was, so a
    // vertex bound to the bone before it does not move and one bound to the
    // bone after it turns rigidly about that very point. Distance to the hub is
    // conserved by construction, and the experiment could not have come out any
    // other way. Area is what a crease actually destroys.
    println!("\nfolding a joint 90 degrees (surface area kept across the joint)");
    println!(
        "{:<22} {:>11} {:>11} {:>8}",
        "folded", "rest mm2", "posed mm2", "kept"
    );
    for (limb, name) in [(Limb::HindLeft, "knee"), (Limb::ForeLeft, "elbow")] {
        let Some([_, mid, _]) = rig.limb_chain(limb) else {
            continue;
        };
        let mut pose = Pose::rest(rig);
        pose.rotations[mid] = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let moved = pose.forward(rig).deform(rig, &mesh.positions, weights);

        let centre = rig.joints[mid].position;
        let radius = rig.joints[mid].radius;
        let mut rest = 0.0f32;
        let mut held = 0.0f32;
        for face in &mesh.faces {
            let corners: Vec<usize> = face.iter().map(|&index| index as usize).collect();
            if corners.len() < 3 {
                continue;
            }
            let middle = corners
                .iter()
                .map(|&vertex| mesh.positions[vertex])
                .fold(Vec3::ZERO, |sum, at| sum + at)
                / corners.len() as f32;
            if middle.distance(centre) > radius * 1.5 {
                continue;
            }
            let fan = |at: &dyn Fn(usize) -> Vec3| {
                (1..corners.len() - 1)
                    .map(|corner| {
                        (at(corners[corner]) - at(corners[0]))
                            .cross(at(corners[corner + 1]) - at(corners[0]))
                            .length()
                            * 0.5
                    })
                    .sum::<f32>()
            };
            rest += fan(&|vertex| mesh.positions[vertex]);
            held += fan(&|vertex| moved[vertex]);
        }
        if rest <= 0.0 {
            continue;
        }
        println!(
            "{:<22} {:>10.0} {:>11.0} {:>7.0}%",
            format!("{name} ({limb:?})"),
            rest * 1e6,
            held * 1e6,
            held / rest * 100.0
        );
    }
}
