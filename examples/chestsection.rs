//! What the trunk is shaped like ACROSS, not how wide it is.
//!
//! `examples/bodyaudit` prints the trunk silhouette band by band as fractions
//! of stature **across**, and its HEIGHTS table carries reference figures that
//! are heights of landmarks. Both are the right instruments for the questions
//! they were built for and neither can see a chest: a pectoral and a breast are
//! **fore-and-aft**, and how far the front of the chest stands off the ribcage
//! it sits on, where that stands off peaks, and whether there are two of them
//! are three numbers nothing in this crate reports. Until they exist, the
//! defect epic #269 was raised for can only be seen, not stated.
//!
//! The precedent is `examples/facesection`, which bisects the **built surface**
//! rather than reading vertices, and which turned one described complaint about
//! the nose and mouth into three measured ones. This does the same on the
//! trunk.
//!
//! # The three readings, and why each is shaped the way it is
//!
//! **The section across.** At each band the front surface is walked from flank
//! to flank and the profile `reach(x)` is read by bisection. A tube's section is
//! an ellipse, so `reach` is concave and has exactly one maximum, on the
//! midline. A chest has **two**, off the midline, with a sternum between them.
//! So the reading is not a width but a count: how many prominent maxima, how far
//! apart, and how deep the dip between them. `reach` on a convex section is
//! concave whatever the faceting does to it, so a spurious second peak is not a
//! thing this can report by accident — and [`PROMINENCE`] guards the case where
//! it could.
//!
//! **The projection off the ribcage.** A projection measured against zero says
//! nothing; it has to be against the line the feature stands on, and that line
//! is the body's own BACK at the same height. Every trunk ring in this plan is
//! a centred ellipse, so a body with no chest is front-to-back symmetric by
//! construction and the asymmetry IS the feature. It reads exactly `+0.00` mm
//! at every band of today's body, which is a noise floor of nothing rather than
//! a number to subtract. See [`ribcage_residual`] for the fitted-ellipse
//! reference that was built first and refuted.
//!
//! **The two readings have very different sensitivities, and that is the point
//! rather than a redundancy.** Against the synthetic control at life spacing:
//!
//! | authored lobe | 10 mm | 20 mm | 40 mm | 90 mm |
//! |---|---|---|---|---|
//! | proud of the back | 8.3 | 16.7 | 33.4 | 75.0 |
//! | sides the section shows | 1 | 1 | 2 | 2 |
//!
//! So a projection is visible from a millimetre up and a SECTION only becomes
//! two-sided somewhere between 20 and 40 mm — because a feature at the
//! intermammary distance has to out-climb the ribcage's own fall of 30.6 mm
//! over the same span before it is a separate peak at all. A male pectoral
//! stands 10 to 20 mm in life, so on this trunk it is a projection that never
//! becomes a shape. Any mechanism #271 picks has to clear both bars, and they
//! are an order of magnitude apart.
//!
//! **The resolution actually available.** The nose dorsum had ONE cell between
//! the midline and the edge of the feature and that was the whole of #181. The
//! same number is printed here for the chest, because whichever mechanism #271
//! picks has to draw a feature on the cells that exist.
//!
//! # The control, which this cannot ship without
//!
//! Every section reading is exactly zero on today's body, and a number that
//! reads zero for every configuration **including the control** is what a dead
//! instrument looks like — this crate has caught twenty of them. There is no
//! feature to suppress here, so the control is synthesised: `--lobe` displaces
//! the built surface by a known analytic pair of lobes and the readings must
//! come back with the numbers that were put in. It is deliberately a
//! displacement of the FINISHED mesh and not a mechanism — it commits to
//! nothing #271 has to decide, and it doubles as the target the candidates
//! there are costed against.
//!
//! With `--lobe` the section readings are joined by a facesection-style
//! subtraction, `reach(lobed) − reach(plain)`, which is the relief the mesh
//! actually delivered against the relief that was authored. Their ratio is how
//! much of the feature survived onto the polygons, and it is a second,
//! independent check on the fitted residual: two readings taken different ways
//! agreeing is what says the instrument works.
//!
//! ```text
//! cargo run --release --example chestsection
//! cargo run --release --example chestsection -- --femininity 1
//! cargo run --release --example chestsection -- --volume 1 --projection -1
//! cargo run --release --example chestsection -- --lobe 20
//! cargo run --release --example chestsection -- 7 23 42
//! ```
//!
//! Life figures to column against, quoted from general anthropometric knowledge
//! rather than from a named table, in the same way `face::eye`'s globe is: a
//! male pectoral stands 10 to 20 mm off the ribcage at its thickest, a female
//! breast 40 to 90 mm from chest wall to nipple over the same ribcage, the two
//! sides separated by an intermammary distance of 180 to 230 mm at the nipples,
//! and both features peak between a fifth and a third of the way down the
//! sternum from the sternal notch.

use symbios_avatar::face::HeadTraits;
use symbios_avatar::torso::{ChestTraits, carve_chest};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Skeleton, Vec3, Zone,
    build_body,
};

/// How far a bisection may travel forward from the trunk's own axis, in metres.
///
/// A trunk is nowhere near 400 mm deep from its spine; the bound only has to
/// lie outside every surface it will be asked about.
const FAR: f32 = 0.40;

/// Halvings per bisection. Thirty takes any trunk to well under a micron.
const HALVINGS: usize = 30;

/// Millimetres between samples across a section.
///
/// Far finer than any cell on the trunk — those run to tens of millimetres —
/// because the point is to read the polygon surface rather than to re-sample
/// the ring that drew it. A section walked at cell resolution cannot tell a
/// facet from a feature.
const STEP: f32 = 2.0;

/// Millimetres between samples down the midline.
const RISE: f32 = 5.0;

/// How far a local maximum must stand above the dip beside it to be counted, in
/// metres.
///
/// **A millimetre, and it is the guard against faceting rather than a
/// threshold on features.** `reach` across a convex section is concave and so
/// has one maximum however coarse the mesh is, but a section is only convex
/// while nothing else pushes on it — an arm socket, a clavicle — and a
/// vertex-level kink could otherwise be reported as a second lobe. Anything a
/// chest mechanism is worth building will clear this by an order of magnitude:
/// the shallowest feature this epic is about, a male pectoral, stands 10 mm.
const PROMINENCE: f32 = 1e-3;

/// Where a synthetic lobe sits and how wide it is, as shares of the section's
/// own half-width and of the chest band's height.
///
/// Sized on the life figures in the module note rather than on what the mesh
/// can carry: a control that was shrunk to fit the resolution would be
/// measuring the instrument against itself. The trunk's own half-width at the
/// chest is 130 mm on the reference body, so an intermammary distance of 180 to
/// 230 mm puts the pair at 0.69 to 0.88 of it and 0.70 is the low end of life
/// rather than a number that suited the mesh.
const LOBE: (f32, f32, f32, f32) = (0.70, 0.28, 0.62, 0.16);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |name: &str| -> Option<f32> {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse().ok())
    };
    let lobe = number("--lobe").map(|mm| mm / 1000.0);
    let seeds: Vec<i64> = args
        .iter()
        .enumerate()
        .filter(|(at, _)| *at == 0 || !args[at - 1].starts_with("--"))
        .filter_map(|(_, arg)| arg.parse().ok())
        .collect();
    let seeds = if seeds.is_empty() { vec![-1] } else { seeds };

    for seed in seeds {
        let mut record = AvatarRecord::new("Sectioned", Archetype::default());
        if seed >= 0 {
            record.reroll(seed);
        }
        // Compared at EQUAL STATURE, which is what these flags are for: rolling
        // a seed to find a feminine body moves five other axes at once, and
        // stature alone scales a body uniformly and proves nothing.
        if let Some(femininity) = number("--femininity") {
            record.composites.femininity = femininity;
        }
        if let Some(mass) = number("--mass") {
            record.composites.mass = mass;
        }
        if let Some(fat) = number("--fat") {
            record.composites.body_fat = fat;
        }
        if let Some(age) = number("--age") {
            record.composites.age = age as u32;
        }
        if let symbios_avatar::Archetype::Humanoid(params) = &mut record.archetype {
            if let Some(value) = number("--volume") {
                params.chest_volume = value;
            }
            if let Some(value) = number("--projection") {
                params.chest_projection = value;
            }
            if let Some(value) = number("--lift") {
                params.chest_lift = value;
            }
        }
        record.composites.sanitize();
        record.sanitize();

        // The shipped order, exactly: traits off the composites and one body
        // built from them. A probe that passes `Default::default()` measures a
        // body no shipped path produces.
        let skeleton = record.skeleton();
        let traits = HeadTraits::of(&record.composites);
        let Ok(plain) = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &traits,
        ) else {
            println!("seed {seed} does not mesh");
            continue;
        };
        let Ok(rig) = Rig::from_skeleton(&skeleton) else {
            continue;
        };
        let Some(mut trunk) = Trunk::of(&rig) else {
            println!("seed {seed} has no trunk column");
            continue;
        };
        // Measured once, on the body before any synthetic lobe touches it, so
        // the lobe is one shape on the trunk rather than a different one at
        // every height and so the control cannot move its own reference.
        trunk.half = half_width(&plain, &rig, trunk.chest);

        // The carve applied here rather than read back off `Avatar::build`,
        // and the reason is worth a line: the shipped avatar is GROUNDED —
        // `AvatarConfig::ground` translates the whole body so its feet stand on
        // zero — so its mesh and this rig are in different spaces, and a
        // section taken at the rig's chest height on the shipped mesh reads
        // empty air. Measuring both surfaces here keeps one space; that the
        // shipped path applies the same carve is a test's job, not an
        // instrument's (`the_shipped_body_carries_the_chest_it_was_built_with`).
        //
        // `plain` is kept as the uncarved control, and every relief number
        // below is the difference between the two — `facesection`'s own design,
        // for its reason: a projection measured against the body beside it is
        // measuring the ribcage's curvature as much as the feature's.
        let mut carved = plain.clone();
        if !args.iter().any(|arg| arg == "--bare") {
            let axes = match &record.archetype {
                symbios_avatar::Archetype::Humanoid(params) => symbios_avatar::torso::ChestAxes {
                    volume: params.chest_volume,
                    projection: params.chest_projection,
                    lift: params.chest_lift,
                },
                _ => symbios_avatar::torso::ChestAxes::default(),
            };
            carve_chest(
                &mut carved,
                &rig,
                &ChestTraits::of(&record.composites).on(axes),
            );
        }
        let lobed = lobe.map(|height| lobe_onto(&carved, &trunk, height));
        let surface = lobed.as_ref().unwrap_or(&carved);
        let (low, high) = plain.bounds();
        let stature = (high.y - low.y).max(1e-3);

        println!(
            "\n=== seed {seed} — femininity {:+.2}, mass {:+.2}, fat {:+.2}; stature {:.3} m{}",
            record.composites.femininity,
            record.composites.mass,
            record.composites.body_fat,
            stature,
            lobe.map_or(String::new(), |h| format!(
                ", SYNTHETIC LOBE {:.0} mm",
                h * 1000.0
            ))
        );
        column(&rig, &skeleton, &trunk, surface, stature);
        let cells = Cells::of(surface, &trunk);
        midline(surface, &trunk, stature);
        sections(surface, &plain, &rig, &trunk, &cells, stature, lobe);
        verdict(surface, &plain, &rig, &trunk);
    }
}

/// The trunk's own column, and the band a chest lives in.
struct Trunk {
    /// Height of the waist joint, the bottom of the band.
    waist: f32,
    /// Height of the chest joint.
    chest: f32,
    /// Height of the shoulder girdle, the top of the band.
    girdle: f32,
    /// The column's joints, low to high, with their zone names.
    column: Vec<(&'static str, usize)>,
    /// How far the trunk reaches sideways at the chest, measured once.
    half: f32,
}

impl Trunk {
    /// Reads the column off the rig rather than off the plan that built it.
    fn of(rig: &Rig) -> Option<Self> {
        let one = |zone: Zone| rig.in_zone(zone).first().copied();
        let pelvis = one(Zone::Pelvis)?;
        let waist = one(Zone::Abdomen)?;
        let chests = rig.in_zone(Zone::Chest);
        let (&chest, &girdle) = (chests.first()?, chests.get(1)?);
        let neck = one(Zone::Neck)?;
        Some(Self {
            waist: rig.joints[waist].position.y,
            chest: rig.joints[chest].position.y,
            girdle: rig.joints[girdle].position.y,
            column: vec![
                ("pelvis", pelvis),
                ("waist", waist),
                ("chest", chest),
                ("girdle", girdle),
                ("neck", neck),
            ],
            half: f32::NAN,
        })
    }

    /// The heights a section is taken at, low to high.
    fn bands(&self) -> Vec<f32> {
        (0..=6)
            .map(|step| self.waist + (self.girdle - self.waist) * step as f32 / 6.0)
            .collect()
    }
}

/// How far the surface reaches forward from the trunk's axis at a point, or
/// `None` where the axis point itself is outside the body.
fn reach(mesh: &PolyMesh, across: f32, up: f32) -> Option<f32> {
    let from = Vec3::new(across, up, 0.0);
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, FAR);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from + Vec3::Z * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// How far out the section reaches before it stops being the TRUNK, in metres.
///
/// **Bounded by which bone the surface belongs to, not by where the body ends**
/// (#270). Walked to the body's edge instead, a section taken at girdle height
/// runs straight out along the shoulders and into the arms: it read a half of
/// 318 mm against the chest's 132, found three "peaks" 472 mm apart, and would
/// have reported a pair of deltoids as a chest. The selector is `nearest_bone`
/// on the surface point, which is the one [`symbios_avatar::rig::Surface`]
/// already uses to decide what a vertex belongs to.
fn half_width(mesh: &PolyMesh, rig: &Rig, up: f32) -> f32 {
    let mut across = 0.0f32;
    while across < 0.5 {
        let axis = Vec3::new(across, up, 0.0);
        if !mesh.contains(axis) {
            break;
        }
        let Some(forward) = reach(mesh, across, up) else {
            break;
        };
        if !is_trunk(rig, Vec3::new(across, up, forward)) {
            break;
        }
        across += STEP / 1000.0;
    }
    (across - STEP / 1000.0).max(0.0)
}

/// Whether a point on the surface belongs to the trunk rather than to a limb.
fn is_trunk(rig: &Rig, point: Vec3) -> bool {
    matches!(
        rig.joints[rig.nearest_bone(point).joint].zone,
        Zone::Pelvis | Zone::Abdomen | Zone::Chest
    )
}

/// The column's nodes beside what the surface actually delivered on the
/// midline.
///
/// **The column that matters is `on cage`, not `reach/r`.** A node's radius is
/// a circle and the trunk's rings are ellipses, so the depth the plan actually
/// asked for is `radius · scale.y` and the ratio worth knowing is how much of
/// THAT the limit surface delivers. Divided by the radius alone the number is a
/// mixture of the cage-to-surface loss and whatever the section's own
/// fore-and-aft multiple happens to be, and reads as if a chest were a third
/// shallower than its plan when the loss is under a tenth.
fn column(rig: &Rig, skeleton: &Skeleton, trunk: &Trunk, mesh: &PolyMesh, stature: f32) {
    println!("\n## The column — what the plan asked for, and what the surface came out at\n");
    println!(
        "| joint | y mm | node r mm | section | cage depth mm | midline reach mm | on cage | \
         of stature |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for &(name, joint) in &trunk.column {
        let at = rig.joints[joint].position.y;
        let radius = rig.joints[joint].radius;
        // **Through `Joint::node`, because a rig is not a skeleton in the same
        // order.** `Rig::from_skeleton` walks breadth-first from the pelvis, so
        // joint index and node index diverge from the second joint on; indexed
        // straight, this column reported the chest's section as 0.94 against
        // the plan's 0.74 and the girdle's as 1.00 against 1.10 — a ruler
        // reading the node next door, which is the trap this crate has now
        // caught twenty-one times.
        let scale = rig.joints[joint]
            .node
            .and_then(|node| skeleton.nodes.get(node as usize))
            .map_or(1.0, |node| node.scale.y);
        let asked = radius * scale;
        let forward = reach(mesh, 0.0, at);
        println!(
            "| {name} | {:.0} | {:.1} | {:.2} | {:.1} | {} | {} | {} |",
            at * 1000.0,
            radius * 1000.0,
            scale,
            asked * 1000.0,
            forward.map_or("—".into(), |f| format!("{:.1}", f * 1000.0)),
            forward.map_or("—".into(), |f| format!("{:.3}", f / asked)),
            forward.map_or("—".into(), |f| format!("{:.4}", f / stature)),
        );
    }
}

/// The forward reach down the midline, and whether it peaks anywhere in the
/// chest band.
///
/// **The reading epic #269 predicted and the one that says "tube" fastest.** A
/// pectoral or a bust is a local maximum of forward projection partway down the
/// chest; a lofted tube between a waist and a girdle has none, because a loft
/// between two ellipses is monotone in whatever the two disagree about.
fn midline(mesh: &PolyMesh, trunk: &Trunk, stature: f32) {
    println!("\n## Forward reach down the midline, waist to girdle\n");
    println!("| y mm | reach mm | of stature | slope mm/mm |");
    println!("|---|---|---|---|");
    let mut profile: Vec<(f32, f32)> = Vec::new();
    let mut up = trunk.waist;
    while up <= trunk.girdle + 1e-6 {
        if let Some(forward) = reach(mesh, 0.0, up) {
            profile.push((up, forward));
        }
        up += RISE / 1000.0;
    }
    for window in 0..profile.len() {
        let (up, forward) = profile[window];
        let slope = if window == 0 {
            f32::NAN
        } else {
            let (was_up, was) = profile[window - 1];
            (forward - was) / (up - was_up)
        };
        println!(
            "| {:.0} | {:.1} | {:.4} | {:+.3} |",
            up * 1000.0,
            forward * 1000.0,
            forward / stature,
            slope
        );
    }
    let peaks = prominent(&profile);
    match peaks.len() {
        0 | 1
            if peaks
                .first()
                .is_none_or(|&(at, _)| at <= trunk.waist + 1e-4 || at >= trunk.girdle - 1e-4) =>
        {
            println!(
                "\n  NO local maximum between the waist and the girdle: the front of the trunk \
                 is monotone through the whole chest. A chest has one."
            );
        }
        _ => {
            for &(at, forward) in &peaks {
                println!(
                    "\n  local maximum at y {:.0} mm ({:.2} of the way up the band), reach \
                     {:.1} mm",
                    at * 1000.0,
                    (at - trunk.waist) / (trunk.girdle - trunk.waist),
                    forward * 1000.0
                );
            }
        }
    }
    println!(
        "\n  the chest joint sits at {:.2} of the way up the band; life puts a pectoral's and \
         a bust's peak between 0.2 and 0.33 of the sternum below the notch",
        (trunk.chest - trunk.waist) / (trunk.girdle - trunk.waist)
    );
}

/// The section across, band by band.
fn sections(
    mesh: &PolyMesh,
    plain: &PolyMesh,
    rig: &Rig,
    trunk: &Trunk,
    cells: &Cells,
    stature: f32,
    lobe: Option<f32>,
) {
    println!("\n## The section across, flank to flank\n");
    println!(
        "| y mm | half mm | peaks | apart mm | off midline mm | dip mm | proud mm | of stature \
         | at x mm | cell mm | cells to flank |{}",
        if lobe.is_some() {
            " authored mm | delivered mm | survived |"
        } else {
            ""
        }
    );
    println!(
        "|---|---|---|---|---|---|---|---|---|---|---|{}",
        if lobe.is_some() { "---|---|---|" } else { "" }
    );
    for up in trunk.bands() {
        let half = half_width(mesh, rig, up);
        if half <= 0.0 {
            println!("| {:.0} | off the body |", up * 1000.0);
            continue;
        }
        // Walked from flank to flank rather than out from the midline: two
        // lobes are a fact about the whole section, and a half-section cannot
        // report a separation.
        let mut profile: Vec<(f32, f32)> = Vec::new();
        let mut across = -half;
        while across <= half {
            if let Some(forward) = reach(mesh, across, up) {
                profile.push((across, forward));
            }
            across += STEP / 1000.0;
        }
        let peaks = prominent(&profile);
        let apart = match (peaks.first(), peaks.last()) {
            (Some(&(low, _)), Some(&(high, _))) if peaks.len() > 1 => high - low,
            _ => 0.0,
        };
        let off = peaks
            .iter()
            .fold(0.0f32, |most, &(at, _)| most.max(at.abs()));
        let crest = profile.iter().fold(0.0f32, |most, &(_, f)| most.max(f));
        let midline = profile
            .iter()
            .find(|&&(at, _)| at.abs() < STEP / 2000.0)
            .map_or(0.0, |&(_, f)| f);
        let dip = (crest - midline).max(0.0);
        let (proud, at) = ribcage_residual(mesh, &profile, up);
        let cell = cells.at(up);
        let mut row = format!(
            "| {:.0} | {:.0} | {} | {:.0} | {:.0} | {:.1} | {:+.2} | {:+.4} | {:.0} | {:.1} | \
             {:.1} |",
            up * 1000.0,
            half * 1000.0,
            peaks.len(),
            apart * 1000.0,
            off * 1000.0,
            dip * 1000.0,
            proud * 1000.0,
            proud / stature,
            at * 1000.0,
            cell * 1000.0,
            half / cell,
        );
        if let Some(height) = lobe {
            // The relief the mesh delivered against the relief that was asked
            // for, facesection's own subtraction: an independent second reading
            // of the same feature.
            let authored = lobe_field(trunk, height, off, up);
            let delivered = match (reach(mesh, off, up), reach(plain, off, up)) {
                (Some(with), Some(without)) => with - without,
                _ => f32::NAN,
            };
            // The ratio is only a ratio where there is something to divide by:
            // a band the lobe barely reaches divides two roundings and prints a
            // number that looks like a resolution finding and is arithmetic.
            row.push_str(&format!(
                " {:.1} | {:.1} | {} |",
                authored * 1000.0,
                delivered * 1000.0,
                if authored > 5e-4 {
                    format!("{:.2}", delivered / authored)
                } else {
                    "—".into()
                }
            ));
        }
        println!("{row}");
    }
}

/// How far the front of the section stands proud of the ribcage behind it, and
/// where.
///
/// **The reference is the body's OWN BACK at the same height, and the two
/// obvious alternatives are both refuted.** A projection measured against zero
/// says nothing. A projection measured against an ellipse fitted to the
/// section's own flanks was built first and is worse than nothing: a bust at
/// the intermammary distance life reports sits at 0.7 of the half-width, which
/// is inside any flank window wide enough to fit through, so the fit is dragged
/// forward by the very feature it is the reference for — measured, it turned a
/// 90 mm lobe into a residual of MINUS 35 mm, a deficit where the feature is.
///
/// The back is immune to that and needs no fitting at all. Every trunk ring in
/// this plan is a centred ellipse — only the neck uses `Node::offset` — so a
/// body with no chest is front-to-back symmetric by construction, and the
/// asymmetry IS the feature. What this reads on today's body is therefore the
/// instrument's own noise floor, and that number is printed rather than assumed.
fn ribcage_residual(mesh: &PolyMesh, profile: &[(f32, f32)], up: f32) -> (f32, f32) {
    profile
        .iter()
        .fold((f32::MIN, f32::NAN), |best, &(across, forward)| {
            let Some(behind) = depth_back(mesh, across, up) else {
                return best;
            };
            let residual = forward - behind;
            if residual > best.0 {
                (residual, across)
            } else {
                best
            }
        })
}

/// How far the surface reaches BACKWARD from the trunk's axis at a point.
fn depth_back(mesh: &PolyMesh, across: f32, up: f32) -> Option<f32> {
    let from = Vec3::new(across, up, 0.0);
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, FAR);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from - Vec3::Z * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// Local maxima of a profile that stand [`PROMINENCE`] above the dips beside
/// them, in order.
///
/// A plateau — a facet whose whole top is the maximum — is reported once, at its
/// middle, rather than as one peak per sample across it.
fn prominent(profile: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut peaks = Vec::new();
    let mut at = 0usize;
    while at < profile.len() {
        let height = profile[at].1;
        let mut end = at;
        while end + 1 < profile.len() && (profile[end + 1].1 - height).abs() <= f32::EPSILON {
            end += 1;
        }
        let rises_into = at == 0 || profile[at - 1].1 < height;
        let falls_out_of = end + 1 >= profile.len() || profile[end + 1].1 < height;
        if rises_into && falls_out_of {
            // The deeper of the two dips flanking it, which is what prominence
            // means: a shoulder on the side of a bigger peak is not a peak.
            let dip = |range: &mut dyn Iterator<Item = &(f32, f32)>| {
                range
                    .take_while(|&&(_, other)| other <= height)
                    .fold(height, |low, &(_, other)| low.min(other))
            };
            let before = dip(&mut profile[..at].iter().rev());
            let after = dip(&mut profile[end + 1..].iter());
            if height - before.max(after) >= PROMINENCE {
                peaks.push((0.5 * (profile[at].0 + profile[end].0), height));
            }
        }
        at = end + 1;
    }
    peaks
}

/// The synthetic lobe's own field: a paired forward bump, in metres.
///
/// Gaussian in both axes and mirrored about the midline, so the pair is one
/// expression and the sternum between them is what two Gaussians leave rather
/// than a third term. See [`LOBE`] for the geometry and the module note for why
/// this is a displacement of the finished mesh and not a mechanism.
fn lobe_field(trunk: &Trunk, height: f32, across: f32, up: f32) -> f32 {
    let span = trunk.girdle - trunk.waist;
    let (centre, wide, along, tall) = LOBE;
    let peak_up = trunk.waist + span * along;
    let half = trunk.half;
    let sideways = (across.abs() - centre * half) / (wide * half);
    let vertical = (up - peak_up) / (tall * span);
    height * (-(sideways * sideways) - vertical * vertical).exp()
}

/// A copy of the body with [`lobe_field`] displaced onto its front.
///
/// Only vertices already on the front of the trunk move, and they move along
/// `+Z`, which is the axis every reading here is taken along.
fn lobe_onto(mesh: &PolyMesh, trunk: &Trunk, height: f32) -> PolyMesh {
    let mut lobed = mesh.clone();
    for point in &mut lobed.positions {
        if point.z <= 0.0 || point.y < trunk.waist || point.y > trunk.girdle {
            continue;
        }
        point.z += lobe_field(trunk, height, point.x, point.y);
    }
    lobed
}

/// Median cell size on the front of the trunk, band by band.
struct Cells {
    /// Band height and its median edge, low to high.
    bands: Vec<(f32, f32)>,
}

impl Cells {
    /// Measures the front of the trunk between the waist and the girdle.
    ///
    /// Front-facing edges only, and inside the chest's own column rather than
    /// across the whole trunk: a median taken round the back reports the ring
    /// spacing of a surface no chest feature is drawn on, which is the trap
    /// `facesection` documents for the cheek.
    fn of(mesh: &PolyMesh, trunk: &Trunk) -> Self {
        let mut bands: Vec<(f32, Vec<f32>)> = trunk
            .bands()
            .into_iter()
            .map(|band| (band, Vec::new()))
            .collect();
        let gap = (trunk.girdle - trunk.waist) / 12.0;
        for face in &mesh.faces {
            for pair in 0..face.len() {
                let a = mesh.positions[face[pair] as usize];
                let b = mesh.positions[face[(pair + 1) % face.len()] as usize];
                let middle = 0.5 * (a + b);
                if middle.z <= 0.0 || middle.x.abs() > trunk.half {
                    continue;
                }
                if let Some(slot) = bands
                    .iter_mut()
                    .find(|(height, _)| (middle.y - *height).abs() < gap)
                {
                    slot.1.push(a.distance(b));
                }
            }
        }
        Self {
            bands: bands
                .into_iter()
                .map(|(height, mut edges)| {
                    edges.sort_by(f32::total_cmp);
                    let median = edges.get(edges.len() / 2).copied().unwrap_or(f32::NAN);
                    (height, median)
                })
                .collect(),
        }
    }

    /// The median cell nearest a height.
    fn at(&self, up: f32) -> f32 {
        self.bands
            .iter()
            .min_by(|a, b| (a.0 - up).abs().total_cmp(&(b.0 - up).abs()))
            .map_or(f32::NAN, |&(_, cell)| cell)
    }
}

/// The three numbers epic #269 was raised to get, said once.
///
/// Printed after the tables rather than instead of them: a reader who wants to
/// know whether the trunk has a chest should not have to read a grid to find
/// out, and a reader who doubts the answer needs the grid it came from.
fn verdict(mesh: &PolyMesh, plain: &PolyMesh, rig: &Rig, trunk: &Trunk) {
    let up = trunk.waist + (trunk.girdle - trunk.waist) * LOBE.2;
    let half = half_width(mesh, rig, up);
    let mut profile: Vec<(f32, f32)> = Vec::new();
    let mut across = -half;
    while across <= half {
        if let Some(forward) = reach(mesh, across, up) {
            profile.push((across, forward));
        }
        across += STEP / 1000.0;
    }
    let peaks = prominent(&profile);
    let crest = profile.iter().fold(0.0f32, |most, &(_, f)| most.max(f));
    let midline = profile
        .iter()
        .find(|&&(at, _)| at.abs() < STEP / 2000.0)
        .map_or(0.0, |&(_, f)| f);
    let (proud, _) = ribcage_residual(mesh, &profile, up);
    // The relief the carve actually delivered: the same surface with and
    // without it, which is what life's "chest wall to nipple" is comparable to.
    // `proud` is the mechanism-independent shape reading and this is the
    // feature's own size; the first runs about twice the second, because a
    // radial displacement lands a vertex where the ribcage behind it was
    // shallower.
    let relief = profile.iter().fold(0.0f32, |most, &(across, forward)| {
        most.max(forward - reach(plain, across, up).unwrap_or(forward))
    });
    println!("\n## At the height a chest peaks, {:.0} mm\n", up * 1000.0);
    println!(
        "  relief over the bare body {:+.2} mm  (life: pectoral 10-20, bust 40-90)",
        relief * 1000.0
    );
    println!("  proud of its own back   {:+.2} mm", proud * 1000.0);
    println!("  sides the section shows {}", peaks.len());
    println!(
        "  separation between them {:.0} mm",
        match (peaks.first(), peaks.last()) {
            (Some(&(low, _)), Some(&(high, _))) if peaks.len() > 1 => (high - low) * 1000.0,
            _ => 0.0,
        }
    );
    println!(
        "  sternum below the crest {:.2} mm",
        (crest - midline).max(0.0) * 1000.0
    );
}
