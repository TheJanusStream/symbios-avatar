//! What a garment's cut costs, and how smooth the hem it leaves is (#117).
//!
//! Three questions, because the tracker filed them as three defects and they
//! turned out to be one piece of arithmetic:
//!
//! 1. **What suppression would save.** A claimed body face is enclosed by the
//!    garment solid on every side, the rim included, so it can never be seen.
//!    The saving is that face's triangles, counted against the body's own — not
//!    against the cloth's, which is a different mesh and a different question.
//! 2. **How coarse the hem is.** A cut takes whole faces, so a hem cannot
//!    deviate from the curve it wants to be by less than about half a face. The
//!    quantum is therefore the hem's own step length, reported in millimetres
//!    because that is the unit a neckline is judged in, and the staircase's
//!    fingerprint is the TURN distribution: a cut on a quad grid turns by right
//!    angles, a smooth hem does not turn at all. `>45 deg` is the share of hem
//!    corners that take a step of the staircase.
//! 3. **Where the rim shows.** A rim is perpendicular to the cloth by
//!    construction, so a rim is not a defect; a rim on BOTH sides of a single
//!    row of faces is. Those are the spurs and isthmuses below — a claimed face
//!    with three cut edges, or with two opposite ones — and they are what reads
//!    as a sliver seen edge-on.
//!
//! **The residual column is the one to argue a smooth hem against.** It fits
//! each hem loop with its first four harmonics — a neckline, an armhole and a
//! waistband are all smooth rings, so four is generous — and reports how far the
//! delivered hem sits from that fit, in millimetres. It is the amplitude a
//! smooth cut would remove, and it is bounded below by the quantisation: a
//! face-granular hem cannot beat about a quarter of its step.
//!
//! Read against `docs/instruments.md` rule 2: every quantity here is asked of
//! the claim and the surface the crate built, never of a vertex window, and the
//! hem is walked with [`hem_loops`] rather than with a private copy of the rim
//! traversal.
//!
//! ```text
//! cargo run --release --example garmentaudit
//! cargo run --release --example garmentaudit -- 7 42      # named seeds
//! cargo run --release --example garmentaudit -- --cuts    # every sleeve x leg
//! ```

use std::collections::HashMap;

use symbios_avatar::dress::garment::hem_loops;
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Garment, Leg, MeshKind, PolyMesh, Sleeve, Vec3,
};

/// Seeds swept when none are named, matching `every_garment_is_a_closed_solid`.
const SWEPT: [i64; 6] = [1, 2, 4, 7, 9, 12];

/// How many harmonics a hem loop is allowed before the rest is called roughness.
const SMOOTH_HARMONICS: usize = 4;

/// How many samples a loop is resampled to before fitting.
const SAMPLES: usize = 64;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let every_cut = args.iter().any(|arg| arg == "--cuts");
    let seeds: Vec<i64> = args
        .iter()
        .filter_map(|arg| arg.parse::<i64>().ok())
        .collect();
    let seeds = if seeds.is_empty() {
        SWEPT.to_vec()
    } else {
        seeds
    };
    let cuts: Vec<(Sleeve, Leg)> = if every_cut {
        [Sleeve::Bare, Sleeve::Forearm, Sleeve::Wrist]
            .into_iter()
            .flat_map(|sleeve| {
                [Leg::Shorts, Leg::Calf, Leg::Ankle]
                    .into_iter()
                    .map(move |leg| (sleeve.clone(), leg))
            })
            .collect()
    } else {
        vec![(Sleeve::default(), Leg::default())]
    };

    let mut suppressed = Vec::new();
    for seed in seeds {
        for (sleeve, leg) in &cuts {
            let mut record = AvatarRecord::new("Dressed", Archetype::default());
            record.reroll(seed);
            record.outfit.sleeve = sleeve.clone();
            record.outfit.leg = leg.clone();
            let avatar = Avatar::build(&record).expect("a biped builds");
            suppressed.push(report(&avatar, seed, sleeve, leg));
        }
    }

    let worst = suppressed.iter().copied().fold(f32::MAX, f32::min);
    let best = suppressed.iter().copied().fold(0.0f32, f32::max);
    println!(
        "\nsuppressible skin over the whole sweep: {:.1}% of the body at least, {:.1}% at most",
        worst * 100.0,
        best * 100.0
    );
}

/// Audits one built avatar. Returns the share of body triangles it could drop.
fn report(avatar: &Avatar, seed: i64, sleeve: &Sleeve, leg: &Leg) -> f32 {
    let body = &avatar.parts.body;
    let body_tris = body.triangulated().len();
    let cloth_tris: usize = avatar
        .meshes
        .iter()
        .filter(|mesh| mesh.kind == MeshKind::Cloth)
        .map(|mesh| mesh.mesh.triangulated().len())
        .sum();

    println!("\nseed {seed}  {sleeve:?}/{leg:?}   body {body_tris} tris   cloth {cloth_tris} tris");
    println!(
        "  {:<9} {:>6} {:>7} {:>5} {:>6} {:>9} {:>8} {:>7} {:>10} {:>6} {:>5}",
        "garment",
        "claim",
        "hidden",
        "loops",
        "edges",
        "step mm",
        "turn deg",
        ">45 deg",
        "resid mm",
        "spurs",
        "isth"
    );

    let mut hidden_total = 0;
    let mut given_back = 0;
    for (index, garment) in avatar.parts.outfit.garments.iter().enumerate() {
        let name = if index == 0 { "trousers" } else { "top" };
        // The crate's own answer, not a re-derivation of it: a garment knows
        // what it hides, and the difference from its claim is what the smoothed
        // hem gives back.
        let hidden = triangles(body, &garment.hidden);
        hidden_total += hidden;
        given_back += triangles(body, &garment.claim) - hidden;
        let mine = mask(garment, body.faces.len());
        let loops = hem_loops(body, &mine);
        let cut = Hem::measure(&positions(body, &loops));
        let worn = Hem::measure(&positions(&garment.mesh, &garment.hem));
        let (spurs, isthmuses) = rim_faults(body, &mine);
        for (stage, hem) in [("cut", &cut), ("worn", &worn)] {
            println!(
                "  {:<9} {:>6} {:>7} {:>5} {:>6} {:>4.1}/{:<4.1} {:>8.1} {:>6.0}% {:>4.2}/{:<5.2} {:>6} {:>5}",
                format!("{name} {stage}"),
                garment.claim.len(),
                hidden,
                loops.len(),
                hem.edges,
                hem.step_mean,
                hem.step_max,
                hem.turn_mean,
                hem.stepped * 100.0,
                hem.residual_rms,
                hem.residual_max,
                spurs,
                isthmuses
            );
        }
    }
    println!(
        "  {:<9} {:>6} {:>7} of {} body tris = {:.1}%, after giving back {} along the hem (claimed {})",
        "hidden",
        "",
        hidden_total,
        body_tris,
        hidden_total as f32 / body_tris as f32 * 100.0,
        given_back,
        hidden_total + given_back
    );
    hidden_total as f32 / body_tris as f32
}

/// How each hem loop departs from the smooth ring it is trying to be.
struct Hem {
    /// Hem edges over every loop.
    edges: usize,
    /// Mean and longest step along the hem, in millimetres.
    step_mean: f32,
    step_max: f32,
    /// Mean turn at a hem corner, in degrees.
    turn_mean: f32,
    /// Share of corners that turn by more than 45 degrees.
    stepped: f32,
    /// Distance from a four-harmonic fit of the loop, in millimetres.
    residual_rms: f32,
    residual_max: f32,
}

/// The loops of a hem as positions, whichever mesh names them.
fn positions(mesh: &PolyMesh, loops: &[Vec<u32>]) -> Vec<Vec<Vec3>> {
    loops
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&vertex| mesh.positions[vertex as usize])
                .collect()
        })
        .collect()
}

impl Hem {
    /// Measures every loop together, weighting each corner equally.
    fn measure(loops: &[Vec<Vec3>]) -> Self {
        let mut edges = 0;
        let mut steps: Vec<f32> = Vec::new();
        let mut turns: Vec<f32> = Vec::new();
        let mut residuals: Vec<f32> = Vec::new();
        for points in loops {
            if points.len() < 3 {
                continue;
            }
            edges += points.len();
            for at in 0..points.len() {
                let here = points[at];
                let next = points[(at + 1) % points.len()];
                let last = points[(at + points.len() - 1) % points.len()];
                steps.push(here.distance(next) * 1000.0);
                let (into, out) = (here - last, next - here);
                if into.length() > 1e-9 && out.length() > 1e-9 {
                    turns.push(
                        into.normalize()
                            .dot(out.normalize())
                            .clamp(-1.0, 1.0)
                            .acos()
                            .to_degrees(),
                    );
                }
            }
            // Too short a loop has no high harmonics to lose: fitting four of
            // them to eleven points fits the noise as well as the ring.
            if points.len() >= 3 * SMOOTH_HARMONICS {
                residuals.extend(roughness(points));
            }
        }
        let mean = |values: &[f32]| {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f32>() / values.len() as f32
            }
        };
        let max = |values: &[f32]| values.iter().copied().fold(0.0f32, f32::max);
        let rms = |values: &[f32]| {
            if values.is_empty() {
                0.0
            } else {
                (values.iter().map(|v| v * v).sum::<f32>() / values.len() as f32).sqrt()
            }
        };
        Self {
            edges,
            step_mean: mean(&steps),
            step_max: max(&steps),
            turn_mean: mean(&turns),
            stepped: if turns.is_empty() {
                0.0
            } else {
                turns.iter().filter(|&&turn| turn > 45.0).count() as f32 / turns.len() as f32
            },
            residual_rms: rms(&residuals),
            residual_max: max(&residuals),
        }
    }
}

/// How far a hem loop sits from its own first few harmonics, in millimetres.
///
/// The loop is resampled by arc length first, so a run of short steps does not
/// out-vote a run of long ones in the fit — the smooth ring being fitted is a
/// curve, not a vertex list.
fn roughness(points: &[Vec3]) -> Vec<f32> {
    let sampled = resample(points, SAMPLES);
    let mut fitted = vec![Vec3::ZERO; SAMPLES];
    for harmonic in 0..=SMOOTH_HARMONICS {
        let (mut cosine, mut sine) = (Vec3::ZERO, Vec3::ZERO);
        for (at, point) in sampled.iter().enumerate() {
            let angle = std::f32::consts::TAU * harmonic as f32 * at as f32 / SAMPLES as f32;
            cosine += *point * angle.cos();
            sine += *point * angle.sin();
        }
        let scale = if harmonic == 0 { 1.0 } else { 2.0 } / SAMPLES as f32;
        for (at, point) in fitted.iter_mut().enumerate() {
            let angle = std::f32::consts::TAU * harmonic as f32 * at as f32 / SAMPLES as f32;
            *point += (cosine * angle.cos() + sine * angle.sin()) * scale;
        }
    }
    sampled
        .iter()
        .zip(&fitted)
        .map(|(had, fit)| had.distance(*fit) * 1000.0)
        .collect()
}

/// A closed polyline resampled to `count` points at equal arc length.
fn resample(points: &[Vec3], count: usize) -> Vec<Vec3> {
    let mut along = Vec::with_capacity(points.len() + 1);
    let mut total = 0.0f32;
    along.push(0.0f32);
    for at in 0..points.len() {
        total += points[at].distance(points[(at + 1) % points.len()]);
        along.push(total);
    }
    if total <= 0.0 {
        return vec![points[0]; count];
    }
    (0..count)
        .map(|step| {
            let want = total * step as f32 / count as f32;
            let at = along.partition_point(|&reached| reached <= want).max(1) - 1;
            let span = along[at + 1] - along[at];
            let part = if span > 0.0 {
                (want - along[at]) / span
            } else {
                0.0
            };
            points[at].lerp(points[(at + 1) % points.len()], part)
        })
        .collect()
}

/// Claimed faces that carry three cut edges, and those that carry two opposite
/// ones — the two shapes that put a rim on both sides of one row of faces.
fn rim_faults(body: &PolyMesh, mine: &[bool]) -> (usize, usize) {
    let mut users: HashMap<(u32, u32), usize> = HashMap::new();
    for (index, face) in body.faces.iter().enumerate() {
        if !mine[index] {
            continue;
        }
        for at in 0..face.len() {
            let (a, b) = (face[at], face[(at + 1) % face.len()]);
            *users
                .entry(if a < b { (a, b) } else { (b, a) })
                .or_default() += 1;
        }
    }
    let (mut spurs, mut isthmuses) = (0, 0);
    for (index, face) in body.faces.iter().enumerate() {
        if !mine[index] {
            continue;
        }
        let cut: Vec<bool> = (0..face.len())
            .map(|at| {
                let (a, b) = (face[at], face[(at + 1) % face.len()]);
                users[&if a < b { (a, b) } else { (b, a) }] == 1
            })
            .collect();
        let count = cut.iter().filter(|&&edge| edge).count();
        if count >= 3 {
            spurs += 1;
        } else if count == 2
            && face.len() == 4
            && (0..face.len()).any(|at| cut[at] && cut[(at + 2) % face.len()])
        {
            isthmuses += 1;
        }
    }
    (spurs, isthmuses)
}

/// A garment's claim as one flag per body face.
fn mask(garment: &Garment, faces: usize) -> Vec<bool> {
    let mut mine = vec![false; faces];
    for &face in &garment.claim {
        mine[face as usize] = true;
    }
    mine
}

/// How many triangles a set of body faces costs.
fn triangles(body: &PolyMesh, faces: &[u32]) -> usize {
    faces
        .iter()
        .map(|&face| body.faces[face as usize].len().saturating_sub(2))
        .sum()
}
