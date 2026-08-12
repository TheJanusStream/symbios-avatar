//! What the nose and the mouth are shaped like ACROSS, not how wide they are.
//!
//! `examples/headaudit` reports a nose's delivered WIDTH, off the displacement
//! rather than off the constants that drew it, and that was the right thing to
//! measure when the question was whether the axes reached the surface at all.
//! It is the wrong instrument for #180, which is a judgement about
//! cross-section: **a ridge and a nose have the same width.** What separates
//! them is how far the thing stands off the face, how the section is shaped
//! between the midline and the flank, and whether the small negative terms — an
//! alar crease, a lip line — survive onto the polygons at all.
//!
//! So this bisects the built surface rather than reading vertices, and every
//! number it prints is a difference between two surfaces:
//!
//! ```text
//! delivered(x, y) = reach(carved, x, y) − reach(plain, x, y)
//! ```
//!
//! **That subtraction is the whole design.** The head under a face is a curved
//! blob, so a projection measured against the cheek beside it is measuring the
//! head's own curvature as much as the feature's; measured against the same
//! head with the carve turned off, it is the relief and nothing else. It also
//! costs nothing to be honest about resolution: the carve moves VERTICES, and
//! `delivered` reads the polygon surface between them, so the ratio of the two
//! is exactly how much of the authored feature survives onto the mesh. A
//! feature whose peak lands between two rows reads lower here than the vertices
//! it was written onto, and that gap is what #59's mean-edge argument predicts.
//!
//! Both bodies are built the way the shipped path builds them — `HeadTraits::of`
//! the record's composites, and `FaceParams::on` those traits — because a probe
//! that passes `Default::default()` measures a body no shipped path produces
//! (#179).
//!
//! ```text
//! cargo run --release --example facesection            # the default body
//! cargo run --release --example facesection -- 7 23 42 # named seeds
//! cargo run --release --example facesection -- --nose 0.9  # one axis end
//! ```
//!
//! Life figures to column against, quoted from general anthropometric
//! knowledge rather than from a named table, in the same way `face::eye`'s
//! globe is: nasion-to-subnasale 45 to 55 mm, alar breadth 31 to 42 mm, nasal
//! tip projection 17 to 21 mm from the subnasale, the alar-facial groove 1.5 to
//! 3 mm deep, and a vermilion standing 4 to 6 mm off the face around it.

use symbios_avatar::face::{Canon, FaceParams, HeadTraits, Skull, carve_face};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Vec3, Zone, build_body,
};

/// How far a bisection may travel forward from the head's own axis, in metres.
///
/// A head is nowhere near 300 mm deep from its joint; the bound only has to be
/// outside every surface it will be asked about.
const FAR: f32 = 0.30;

/// Halvings per bisection. Thirty takes any head to well under a micron.
const HALVINGS: usize = 30;

/// Where the nose is sectioned, along its own span from root to under.
///
/// Not evenly spaced: the top half of a nose is a bridge and says little, and
/// everything the issue is about — the tip, the wings, the crease under them —
/// happens in the last third. `NOSE_RISE` peaks at 0.80 and `NOSE_SPREAD` is
/// widest at 0.92, so those two are both sampled directly rather than
/// straddled.
const ALONG: [f32; 7] = [0.15, 0.35, 0.55, 0.70, 0.80, 0.92, 0.98];

/// How far out a section is walked, as a multiple of the nose's own half-width.
///
/// Past the wings on purpose: the crease that makes a nostril read as a nostril
/// sits at the flank, and a section that stops at the flank cannot show it.
const OUT: f32 = 2.0;

/// Millimetres between samples down the mouth's own column.
///
/// Finer than [`STEP`] because the mouth is the finest thing on the face: the
/// cell at the lip line is 0.82 mm, so a 1.5 mm step reads one sample per two
/// cells and cannot resolve an edge from a slope.
const MOUTH_STEP: f32 = 0.5;

/// Millimetres between samples across a section.
///
/// Finer than any cell on the face, because the point is to read the polygon
/// surface rather than to re-sample the field that drew it.
const STEP: f32 = 0.5;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let axis = |name: &str| -> Option<f32> {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse().ok())
    };
    let nose = axis("--nose");
    let mouth = axis("--mouth");
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
        if let Some(nose) = nose {
            record.face.nose = nose;
        }
        if let Some(mouth) = mouth {
            record.face.mouth = mouth;
        }
        record.sanitize();

        // The shipped order, exactly: traits off the composites, face axes as
        // offsets on those traits, one body built from them and a second one
        // carved. See the module note on #179.
        let skeleton = record.skeleton();
        let traits = HeadTraits::of(&record.composites);
        let params = record.face.on(&traits);
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
        let Some(skull) = Skull::measure(&plain, &rig) else {
            continue;
        };
        let canon = Canon::measure(&rig, &skull, &record.eyes);
        let mut carved = plain.clone();
        carve_face(&mut carved, &rig, &canon, &params);

        let Some(head) = rig.in_zone(Zone::Head).first().copied() else {
            continue;
        };
        let centre = rig.joints[head].position;
        let section = Section {
            plain: &plain,
            carved: &carved,
            centre,
        };

        println!(
            "\n=== seed {seed} — nose {:.2}, width {:.2}, mouth {:.2}, width {:.2}; \
             unit {:.1} mm, frame {:.1} mm",
            params.nose,
            params.nose_width,
            params.mouth,
            params.mouth_width,
            canon.unit * 1000.0,
            canon.frame * 1000.0
        );
        let cells = cell_sizes(&carved, &rig, centre);
        nose_sections(&section, &canon, &cells);
        mouth_profile(&section, &canon, &params, &cells);
    }
}

/// The pair of surfaces every number here is a difference between.
struct Section<'a> {
    /// The head as it is shaped, with no face carved into it.
    plain: &'a PolyMesh,
    /// The same head after [`carve_face`].
    carved: &'a PolyMesh,
    /// The head joint, which every coordinate below is relative to.
    centre: Vec3,
}

impl Section<'_> {
    /// How far a mesh reaches forward at a point, or `None` outside it.
    fn reach(mesh: &PolyMesh, from: Vec3) -> Option<f32> {
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

    /// The relief the carve delivered onto the SURFACE at one point, in metres.
    ///
    /// Positive is proud. `None` where either bisection starts outside its own
    /// mesh, which off the face is common and is not a defect.
    fn delivered(&self, across: f32, up: f32) -> Option<f32> {
        let from = Vec3::new(self.centre.x + across, self.centre.y + up, self.centre.z);
        Some(Self::reach(self.carved, from)? - Self::reach(self.plain, from)?)
    }
}

/// The nose, sectioned across at each of [`ALONG`].
fn nose_sections(section: &Section, canon: &Canon, cells: &Cells) {
    // The nose's own span, read the way `relief::nose` reads it so that `along`
    // means the same thing in both. Any other anchor is a landmark the carve
    // moves (#133).
    let root = canon.level + canon.frame * 0.1237;
    let under = canon.nose_foot();
    // **Two lengths, and only one of them is comparable to life.** The field's
    // span runs from the root to seven millimetres UNDER the base, because a
    // term has to reach zero somewhere and `relief::nose` gates on that span;
    // the nose a tape measure would find runs nasion to subnasale and stops at
    // the base. Printing the span alone invites the wrong comparison — it reads
    // 61 mm on the default face against a life figure of 45 to 55, and the nose
    // is not 12 mm too long.
    println!(
        "\n## The nose across — nasion to subnasale {:.1} mm (life 45-55), the field's own \
         span {:.1} mm\n",
        (root - canon.nose_base()) * 1000.0,
        (root - under) * 1000.0
    );
    println!(
        "| along | mm up | authored | delivered | at x | half at half | shoulder | flat | \
         wing dip | cell | shoulder/cell |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|");

    for &along in &ALONG {
        let up = root - along * (root - under);
        // Walked outward until the relief has plainly finished rather than to a
        // width computed here: recomputing the ramp would restate the source.
        let span = canon.unit * 0.60 * OUT;
        let mut profile: Vec<(f32, f32)> = Vec::new();
        let mut across = 0.0f32;
        while across <= span {
            if let Some(relief) = section.delivered(across, up) {
                profile.push((across, relief));
            }
            across += STEP / 1000.0;
        }
        let Some(&(_, first)) = profile.first() else {
            println!("| {along:.2} | — | off the head |");
            continue;
        };
        let peak = profile
            .iter()
            .fold(first, |most, &(_, relief)| most.max(relief));
        let at = profile
            .iter()
            .find(|&&(_, relief)| relief >= peak - f32::EPSILON)
            .map_or(0.0, |&(across, _)| across);
        // Where the section has fallen to a half and to a fifth of its peak,
        // walking OUT from the peak rather than searching the whole profile:
        // the wing crease is negative and would otherwise satisfy any
        // downward threshold on the wrong side of it.
        let fell = |share: f32| -> f32 {
            profile
                .iter()
                .skip_while(|&&(across, _)| across < at)
                .find(|&&(_, relief)| relief < peak * share)
                .map_or(f32::NAN, |&(across, _)| across)
        };
        let (half, shoulder) = (fell(0.5), fell(0.2));
        // How flat the top is: a blade falls off at once and reads near zero, a
        // tip with volume on it holds its height out to a real fraction of its
        // own shoulder. This is the number a ridge and a nose differ in.
        let flat = fell(0.8) / shoulder;
        // The deepest the surface goes BELOW the uncarved head at the flank,
        // which is the alar crease where there is one. Windowed to the shoulder
        // and half again rather than to everything beyond the half-width: the
        // orbit's own carve is deeply negative and sits just outside the nose
        // at the heights the bridge runs through, so an unwindowed minimum
        // reports the eye socket and calls it a nostril.
        let dip = profile
            .iter()
            .filter(|&&(across, _)| across > shoulder && across < shoulder * 1.5)
            .fold(0.0f32, |low, &(_, relief)| low.min(relief));
        let cell = cells.at(up);
        println!(
            "| {along:.2} | {:+6.1} | {:8.1} | {:9.1} | {:4.1} | {:5.1} | {:5.1} | {:.2} | \
             {:5.2} | {:4.1} | {:5.1} |",
            up * 1000.0,
            authored(section, up, cell) * 1000.0,
            peak * 1000.0,
            at * 1000.0,
            half * 1000.0,
            shoulder * 1000.0,
            flat,
            dip * 1000.0,
            cell * 1000.0,
            shoulder / cell.max(f32::EPSILON)
        );
    }
    println!(
        "\n`authored` is the furthest the carve moved any VERTEX in this band and `delivered` is \
         what the polygon SURFACE shows for it — the gap between them is what falls between two \
         rows. `at x` is where across the face the delivered peak sits. `half at half` and \
         `shoulder` are where the section has fallen to a half and a fifth of it, walking out \
         from the peak. `flat` is the 80% width over the 20% width — a blade tends to zero and a \
         tip with volume on it holds. `wing dip` is the deepest the surface goes UNDER the \
         uncarved head just outside the shoulder, which is the alar crease where one survives. \
         **`shoulder/cell` is the column this instrument was written for**: it is how many mesh \
         cells there are between the midline and the edge of the feature, and a section drawn on \
         one of them can only be a tent."
    );
}

/// The furthest the carve moved any vertex in a band, in metres.
///
/// Read off the vertices rather than off the surface, which is the whole point
/// of having it beside `delivered`: the carve writes vertices and a renderer
/// draws the polygons between them, so the two agree only where the mesh is
/// fine enough to hold the feature. The band is one cell tall, so it holds the
/// row the peak landed on and not the rows either side of it.
fn authored(section: &Section, up: f32, cell: f32) -> f32 {
    let window = cell.max(0.001);
    section
        .plain
        .positions
        .iter()
        .zip(&section.carved.positions)
        .filter(|(was, _)| {
            (was.y - section.centre.y - up).abs() < window
                && was.z - section.centre.z > 0.0
                && (was.x - section.centre.x).abs() < window * 3.0
        })
        .fold(0.0f32, |most, (was, now)| {
            most.max((*now - *was).dot((*was - section.centre).normalize_or_zero()))
        })
}

/// The mouth, sectioned down the midline through the lip band.
fn mouth_profile(section: &Section, canon: &Canon, params: &FaceParams, cells: &Cells) {
    let line = canon.mouth_line();
    // The lip stack's own half-height, which is the ruler `relief::mouth`
    // authors every lobe in — so the band below is stated in it rather than in
    // a fraction of the frame.
    let plump = canon.frame * (0.1142 + 0.0322 * params.mouth);
    // A band that reaches the base of the nose above and well under the lower
    // lip below, so the philtrum and the sub-lip crease are both inside it.
    //
    // **It did not, and said it did** (#182). The bottom was `frame * 0.12`,
    // which is 12.5 mm below the line on the default head, and the crease under
    // the lower lip is authored at 1.32 plumps — 18.0 mm. The instrument stopped
    // 5.5 mm short of the feature its own comment named, so every reading of the
    // lower lip's outer flank ever taken here was of the flank alone with
    // nothing beyond it. 1.9 plumps clears the crease's centre and its far
    // shoulder both, on every seed, because it is measured in the same unit the
    // crease is placed in (`docs/instruments.md` rules 3 and 4).
    let (top, bottom) = (canon.nose_base(), line - plump * 1.9);
    println!(
        "\n## The mouth down the midline, lip line at {:+.1} mm\n",
        line * 1000.0
    );
    println!("| mm up | from line | midline | slope | 8 mm out | 16 mm out |");
    println!("|---|---|---|---|---|---|");
    let mut samples: Vec<(f32, f32)> = Vec::new();
    // **A second column, because the upper border cannot be judged on the
    // midline** (#182). The philtrum is a groove centred a plump above the line,
    // so above the upper vermilion the midline surface keeps falling whatever
    // the border does — there is no roll there to find, and on a face there is
    // none either: the roll is the philtral column, which is beside the dimple
    // and not in it. Eight millimetres out clears the philtrum, whose own half
    // width is 5.2 mm on the default head.
    let mut beside: Vec<(f32, f32)> = Vec::new();
    let mut up = top;
    let mut previous: Option<(f32, f32)> = None;
    while up >= bottom {
        let mid = section.delivered(0.0, up);
        if let Some(mid) = mid {
            samples.push((up, mid));
        }
        if let Some(out) = section.delivered(0.008, up) {
            beside.push((up, out));
        }
        // **The column a border shows in, and relief alone cannot show it.** A
        // vermilion is an edge: on a person the lip rises out of the skin at a
        // definite line rather than fading into it, and an edge is a SLOPE and
        // not a height. A lobe that is a plain Gaussian has its steepest point
        // in the middle of its own flank and nothing anywhere that reads as a
        // boundary, so a mouth drawn out of two of them is two swellings with a
        // line between — which is what #180 reported as an incision. Millimetres
        // of relief per millimetre of face, so it is dimensionless and can be
        // compared between bodies of different sizes.
        let slope = match (previous, mid) {
            (Some((was_up, was)), Some(now)) => {
                Some((now - was) / (up - was_up).abs().max(f32::EPSILON))
            }
            _ => None,
        };
        if let Some(mid) = mid {
            previous = Some((up, mid));
        }
        println!(
            "| {:+6.1} | {:+6.1} | {} | {} | {} | {} |",
            up * 1000.0,
            (up - line) * 1000.0,
            millimetres(mid),
            slope.map_or_else(|| "     —".into(), |slope| format!("{slope:+6.2}")),
            millimetres(section.delivered(0.008, up)),
            millimetres(section.delivered(0.016, up)),
        );
        up -= MOUTH_STEP / 1000.0;
    }

    // The three numbers the issue turns on, off the midline column: how far the
    // vermilion stands proud above and below the line, and how deep the line
    // between them is cut. A mouth is two lips with a line; an incision is the
    // line on its own.
    let peak_between = |lo: f32, hi: f32| -> (f32, f32, f32) {
        samples
            .iter()
            .filter(|&&(up, _)| up >= lo && up <= hi)
            .fold((0.0f32, 0.0f32, line), |(high, low, at), &(up, relief)| {
                if relief > high {
                    (relief, low, up)
                } else {
                    (high, low.min(relief), at)
                }
            })
    };
    // Capped at one plump above the line rather than the 1.30 the lobe's own
    // tail reaches: the nose base sits 1.5 plumps up on the default face, and a
    // window that reaches it reports the NOSE's relief as a vermilion.
    let (upper, _, upper_at) = peak_between(line + plump * 0.15, line + plump * 1.00);
    let (lower, _, lower_at) = peak_between(line - plump * 1.30, line - plump * 0.15);
    let (_, groove, _) = peak_between(line - plump * 0.15, line + plump * 0.15);
    let cell = cells.at(line);
    println!(
        "\nUpper vermilion {:+.2} mm proud at {:+.1} mm off the line, lower {:+.2} at {:+.1}, \
         and the line between them {:+.2} deep — against a lip stack of {:.1} mm and a cell of \
         {:.1} mm here. Life stands a vermilion 4 to 6 mm off the face around it. A mouth that \
         reads as an incision is one whose groove is the only term to survive; a mouth whose \
         groove is a tenth of its vermilion has the opposite problem, and neither is a \
         resolution defect if the cell is under the feature.",
        upper * 1000.0,
        (upper_at - line) * 1000.0,
        lower * 1000.0,
        (lower_at - line) * 1000.0,
        groove * 1000.0,
        plump * 1000.0,
        cell * 1000.0
    );

    // **Where each lip STOPS, which is a different question from how proud it
    // stands** (#182). A vermilion border is a crease: walking outward from the
    // vermilion's peak the surface falls, turns back up at the junction, and
    // rises to the roll outside it. A lobe that is a plain Gaussian never turns
    // — it falls to nothing and stays there — so a mouth built out of two of
    // them has no line at which a lip stops being a lip, whatever it stands off
    // the face. This reports the turn, or says there is none.
    //
    // **Windowed in plumps, and the window is most of what makes it honest.**
    // Unbounded, the walk out of the upper vermilion runs all the way up the
    // philtrum and calls the NOSE a roll — it reported a 13 mm border on the
    // first run — and the walk down out of the lower vermilion finds the sulcus
    // at 1.32 plumps, which is a real crease and not this one. A vermilion
    // border sits 8 to 12 mm off the line on a life-sized mouth, so the window
    // is 0.5 to 1.2 plumps and everything outside it is another feature.
    //
    // The window alone is not enough, because the nose's own flank reaches
    // INSIDE it — bounded to 1.2 plumps the walk still called the nose base a
    // roll, 4.55 mm proud at +16.1. So a turn is also rejected when the roll
    // stands higher than half the vermilion it walked out of: a border's roll
    // is a ridge on the cutaneous lip, and anything half a lip tall is a
    // different feature no matter where it sits.
    // Each lip's flank in one number: where its fall breaks, and what the fall
    // has RECOVERED to there. Negative is still falling — a shoulder — and
    // positive is a turn, which is a crease with a roll outside it. Reported
    // this way because the obvious reading, walk out until the surface rises
    // and call the top of the rise a roll, cannot tell a border's roll from the
    // NOSE: on the default head it called the nose base a 13 mm border, and
    // bounding it to a window and to half a vermilion's height still let seed
    // 23's nose flank in at 1.90 mm. A slope recovery needs no roll and so has
    // nothing to confuse.
    for (name, from, way, column) in [
        ("upper", upper_at, 1.0f32, &beside),
        ("lower", lower_at, -1.0f32, &samples),
    ] {
        // Up to the NOSE'S FOOT above, which is where the cutaneous lip ends
        // and the nose's own relief begins. A window picked as a fraction of the
        // lip stack reaches past it: at 1.2 plumps this reported a turn of +1.90
        // on the default head with the border lobes set to ZERO, all of it the
        // nose (#182). `Canon::nose_foot` is the same landmark `relief::nose`
        // ends its ramp at.
        let window = if way > 0.0 {
            (line + plump * 0.5, canon.nose_foot())
        } else {
            (line - plump * 1.2, line - plump * 0.5)
        };
        let at2 = if way > 0.0 { ", 8 mm out" } else { "" };
        match flank_break(column, from, way, window) {
            Some((at, fall, recovery, stopped)) => println!(
                "  the {name} lip's flank falls at {fall:.2} and bottoms out {:+.1} mm off the \
                 line{at2}, rising at {recovery:+.2} past it — {}.",
                (at - line) * 1000.0,
                if stopped {
                    "a crease, so the lip stops there"
                } else {
                    "the end of the span, so it never stops"
                }
            ),
            None => println!("  the {name} lip's flank could not be read{at2}."),
        }
    }
    // The one crease this face already carries, measured the same way so the
    // two readings above have something of their own kind to be judged against.
    // It is the labiomental sulcus and NOT a vermilion border — authored at
    // 1.32 plumps, outside the window above.
    match flank_break(
        &samples,
        lower_at,
        -1.0,
        (line - plump * 1.9, line - plump * 1.15),
    ) {
        Some((at, fall, recovery, _)) => println!(
            "  for scale, the sulcus under the lower lip — the only crease this face had — \
             falls at {fall:.2}, bottoms out at {:+.1} and rises at {recovery:+.2} past it.",
            (at - line) * 1000.0
        ),
        None => println!("  and the sulcus under the lower lip could not be read."),
    }
}

/// Where a lip's outer flank bottoms out, walking away from the vermilion peak.
///
/// Returns `(at, fall, recovery, stopped)`: the lowest point of the flank
/// inside `window`, how fast it was falling at its steepest on the way there,
/// the slope over the single step just past it, and whether that lowest point
/// lies strictly inside the window rather than at its far end. A flank that
/// bottoms out strictly inside has a crease on it — the surface stops falling
/// and comes back up, which is what a vermilion border is. One that bottoms out
/// at the far end never stopped.
///
/// **The lowest point, and the recovery read one step past it — not a roll**
/// (#182). Reading the border as a minimum followed by a maximum needs a
/// maximum, and above the upper lip the next maximum is the NOSE: unbounded it
/// called the nose base a 13 mm border on the default head, and with a window
/// and a height ceiling on top of it, it still read seed 99's nose flank as a
/// recovery of 5.67 millimetres of relief per millimetre of face. A single step
/// is 0.5 mm and the nose's rise is two millimetres further out.
fn flank_break(
    samples: &[(f32, f32)],
    from: f32,
    way: f32,
    window: (f32, f32),
) -> Option<(f32, f32, f32, bool)> {
    let mut walk: Vec<(f32, f32)> = samples
        .iter()
        .copied()
        .filter(|&(up, _)| (up - from) * way > 0.0 && up >= window.0 && up <= window.1)
        .collect();
    walk.sort_by(|a, b| ((a.0 - from) * way).total_cmp(&((b.0 - from) * way)));
    if walk.len() < 3 {
        return None;
    }

    let low = walk
        .iter()
        .enumerate()
        .fold((0usize, f32::MAX), |(at, low), (index, &(_, relief))| {
            if relief < low {
                (index, relief)
            } else {
                (at, low)
            }
        })
        .0;
    // Signed along the walk, so a falling flank is negative on either lip.
    let slope = |a: usize, b: usize| (walk[b].1 - walk[a].1) / ((walk[b].0 - walk[a].0) * way);
    let fall = (0..low)
        .map(|step| slope(step, step + 1))
        .fold(0.0f32, f32::min);
    let recovery = if low + 1 < walk.len() {
        slope(low, low + 1)
    } else {
        0.0
    };
    Some((
        walk[low].0,
        -fall,
        recovery,
        low > 0 && low + 1 < walk.len(),
    ))
}

/// A metres figure as millimetres, or a dash where there was no reading.
fn millimetres(value: Option<f32>) -> String {
    value.map_or_else(
        || "     —".into(),
        |value| format!("{:+6.2}", value * 1000.0),
    )
}

/// The median edge length of the head's own faces, by height.
///
/// Banded rather than averaged over the whole head: the face is refined and the
/// vault is not, so one figure for a head is a figure for neither. Every number
/// in the tables above is worth reading against the cell beside it — a feature
/// narrower than the surface under it cannot be drawn however well it is
/// authored, which is #59's argument and the one #180 expects to be the answer.
struct Cells {
    /// Median edge per band, low to high, and the band's own height.
    bands: Vec<(f32, f32)>,
}

impl Cells {
    /// The median cell at a height above the head joint.
    fn at(&self, up: f32) -> f32 {
        self.bands
            .iter()
            .min_by(|a, b| {
                (a.0 - up)
                    .abs()
                    .partial_cmp(&(b.0 - up).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map_or(f32::NAN, |&(_, cell)| cell)
    }
}

/// Measures [`Cells`] over the front of the head.
///
/// Front-facing faces only. The back of a head is not refined and would drag
/// every median up, and a feature is only ever drawn on the surface a camera
/// can see.
fn cell_sizes(mesh: &PolyMesh, rig: &Rig, centre: Vec3) -> Cells {
    let radius = rig
        .in_zone(Zone::Head)
        .first()
        .map_or(0.1, |&head| rig.joints[head].radius);
    let mut bands: Vec<(f32, Vec<f32>)> = Vec::new();
    let mut band = 0.06f32;
    while band > -0.14 {
        bands.push((band, Vec::new()));
        band -= 0.01;
    }
    for face in &mesh.faces {
        for pair in 0..face.len() {
            let a = mesh.positions[face[pair] as usize];
            let b = mesh.positions[face[(pair + 1) % face.len()] as usize];
            let middle = 0.5 * (a + b);
            // The front of the head, and above the throat: `Zone` is per-vertex
            // and this is per-edge, so the test is geometric.
            // The feature's OWN column, not the front of the head. The narrow
            // passes of `FACE_PASSES` reach in to a cosine of 0.92, about 23
            // degrees off dead ahead, so a window half a head wide takes its
            // median from the CHEEK — which is unrefined by those passes and
            // reports a nose as four times coarser than the mesh it is drawn
            // on. A fifth of a radius is a nose's own width and a mouth's.
            if middle.z - centre.z < radius * 0.35 || (middle.x - centre.x).abs() > radius * 0.20 {
                continue;
            }
            let up = middle.y - centre.y;
            if let Some(slot) = bands
                .iter_mut()
                .find(|(height, _)| (up - *height).abs() < 0.005)
            {
                slot.1.push(a.distance(b));
            }
        }
    }
    Cells {
        bands: bands
            .into_iter()
            .filter(|(_, edges)| !edges.is_empty())
            .map(|(height, mut edges)| {
                edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                (height, edges[edges.len() / 2])
            })
            .collect(),
    }
}
