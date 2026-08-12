//! What the five follicle regions hold on a built head (#199).
//!
//! The numeric half of the pair: this reports where each region lands and how
//! wide its edges are, and `render --follicles` shows the same five regions on
//! the head so they can be judged rather than read. Both exist because a mask
//! has two ways of being wrong that look identical in a table — it can be in the
//! wrong place, which only a render shows, and it can be the right shape at the
//! wrong size, which only a measurement shows.
//!
//! ```text
//! cargo run --release --example follicleaudit            # the default body
//! cargo run --release --example follicleaudit -- 42      # one seed
//! cargo run --release --example follicleaudit -- --sweep # the population
//! ```
//!
//! **Every share here is an AREA share, and the reason is worth carrying**
//! (#199). `refine_face` splits the front of the face ten times and leaves the
//! vault at the base subdivision, so a head carries thousands of vertices on a
//! chin and dozens on a crown. Counting vertices, the first cut of these numbers
//! read the scalp — the largest region on any head — as 2.9% of one, and the
//! chin as 25%. That is a measurement of the refinement schedule wearing a
//! mask's name.

use symbios_avatar::face::{Canon, Skull};
use symbios_avatar::hair::{Follicle, FollicleParams, Follicles};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, PolyMesh, Vec3, Zone};

/// The seeds `--sweep` reports, which are `tests/budget.rs`'s own.
const SWEEP_SEEDS: [i64; 8] = [0, 3, 7, 13, 23, 42, 11, 15];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--sweep") {
        sweep();
        return;
    }
    let seed = args.first().and_then(|arg| arg.parse::<i64>().ok());
    report(seed);
}

/// One head, in full.
fn report(seed: Option<i64>) {
    let Some(head) = measure(seed) else {
        println!("this body has no head to grow hair on");
        return;
    };
    match seed {
        Some(seed) => println!("seed {seed}"),
        None => println!("the default body"),
    }
    println!(
        "  frame {:.1} mm, eye line {:+.1}, chin {:+.1}, mouth {:+.1}, nose {:+.1}, ear {:+.1}, \
         crown {:+.1}",
        head.frame * 1000.0,
        head.level * 1000.0,
        head.chin * 1000.0,
        head.mouth * 1000.0,
        head.nose * 1000.0,
        head.ear * 1000.0,
        head.crown * 1000.0,
    );
    println!("  {:.0} cm² of head-owned surface", head.area * 10_000.0);
    println!();
    println!("  region       area    core   band, head-local mm   edge");
    for follicle in Follicle::ALL {
        let held = held(&head, follicle);
        println!(
            "  {:10} {:5.1}%  {:5.3}   {:+7.1} to {:+7.1}    {:4.1} mm",
            follicle.name(),
            held.share * 100.0,
            held.best,
            held.lo * 1000.0,
            held.hi * 1000.0,
            held.edge * 1000.0,
        );
    }
    println!();
    println!(
        "  area   — share of the head's own surface the region fully holds (weight over a half)\n  \
         core   — the most any one point on the head is inside it; under 1.00 the region has no\n  \
         {:9}full-weight middle for a clump to root in\n  \
         band   — the heights it spans, which is what to check a landmark against\n  \
         edge   — the narrowest edge anywhere on it: how far the weight takes to cross from 0.1\n  \
         {:9}to 0.9. Under about 2 mm the surface cannot express it, cells being 0.8 to 3",
        "", "",
    );
}

/// The population, one row per seed.
fn sweep() {
    println!("area share of the head's surface, and the narrowest edge, by seed");
    println!(
        "  seed   {:>7} {:>7} {:>7} {:>7} {:>7}   narrowest edge",
        "scalp", "brows", "mouth", "chin", "flanks"
    );
    let mut worst_edge = (f32::MAX, 0i64, Follicle::Scalp);
    let mut extremes = [(f32::MAX, 0.0f32); 5];
    for seed in SWEEP_SEEDS {
        let Some(head) = measure(Some(seed)) else {
            continue;
        };
        let mut row = String::new();
        let mut narrowest = (f32::MAX, Follicle::Scalp);
        for (slot, follicle) in Follicle::ALL.into_iter().enumerate() {
            let held = held(&head, follicle);
            row.push_str(&format!(" {:6.2}%", held.share * 100.0));
            extremes[slot].0 = extremes[slot].0.min(held.share);
            extremes[slot].1 = extremes[slot].1.max(held.share);
            if held.edge < narrowest.0 {
                narrowest = (held.edge, follicle);
            }
        }
        if narrowest.0 < worst_edge.0 {
            worst_edge = (narrowest.0, seed, narrowest.1);
        }
        println!(
            "  {seed:4} {row}   {:4.1} mm on the {}",
            narrowest.0 * 1000.0,
            narrowest.1.name()
        );
    }
    println!();
    for (follicle, (low, high)) in Follicle::ALL.into_iter().zip(extremes) {
        println!(
            "  {:10} {:5.2}% to {:5.2}% of the head",
            follicle.name(),
            low * 100.0,
            high * 100.0
        );
    }
    println!(
        "  the narrowest edge anywhere in the population is {:.1} mm, on seed {}'s {}",
        worst_edge.0 * 1000.0,
        worst_edge.1,
        worst_edge.2.name()
    );
}

/// A built head, its regions, and the surface they are measured over.
struct Head {
    follicles: Follicles,
    skull: Skull,
    /// Head-local centroid and area, per head-owned face.
    surface: Vec<(Vec3, f32)>,
    area: f32,
    frame: f32,
    level: f32,
    chin: f32,
    mouth: f32,
    nose: f32,
    ear: f32,
    crown: f32,
}

/// What one region holds on one head.
struct Held {
    /// Share of the head's area the region fully holds.
    share: f32,
    /// The most any point is inside it.
    best: f32,
    /// The heights it reaches, in head-local metres.
    lo: f32,
    hi: f32,
    /// Its narrowest edge, in metres: how far the weight takes to go 0.1 to 0.9.
    edge: f32,
}

/// Builds a body and cuts its regions.
fn measure(seed: Option<i64>) -> Option<Head> {
    let mut record = AvatarRecord::new("Follicles", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    let avatar = Avatar::build(&record)?;
    let skull = Skull::measure(&avatar.parts.body, &avatar.rig)?;
    let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
    let follicles = Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default());
    let origin = avatar.rig.joints[skull.head].position;
    let surface = surface_of(&avatar.parts.body, &avatar.rig, origin);
    let area = surface.iter().map(|(_, area)| area).sum();
    let (_, crown) = skull.throat_and_crown();
    Some(Head {
        frame: canon.frame,
        level: canon.level,
        chin: skull.chin(),
        mouth: canon.mouth_line(),
        nose: canon.nose_base(),
        ear: canon.ear_centre(),
        crown,
        follicles,
        skull,
        surface,
        area,
    })
}

/// Every head-owned face, as a head-local centroid and an area.
fn surface_of(body: &PolyMesh, rig: &symbios_avatar::Rig, origin: Vec3) -> Vec<(Vec3, f32)> {
    (0..body.face_count())
        .filter_map(|face| {
            let centre = body.face_centroid(face);
            if rig.joints[rig.nearest_bone(centre).joint].zone != Zone::Head {
                return None;
            }
            let corners = &body.faces[face];
            let area = (1..corners.len().saturating_sub(1))
                .map(|step| {
                    let one =
                        body.positions[corners[step] as usize] - body.positions[corners[0] as usize];
                    let two = body.positions[corners[step + 1] as usize]
                        - body.positions[corners[0] as usize];
                    one.cross(two).length() * 0.5
                })
                .sum();
            Some((centre - origin, area))
        })
        .collect()
}

/// Measures one region against one head.
fn held(head: &Head, follicle: Follicle) -> Held {
    let mut share = 0.0;
    let mut best: f32 = 0.0;
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for (local, area) in &head.surface {
        let weight = head.follicles.weight(follicle, *local);
        best = best.max(weight);
        if weight > 0.5 {
            share += area;
            lo = lo.min(local.y);
            hi = hi.max(local.y);
        }
    }
    Held {
        share: share / head.area.max(f32::EPSILON),
        best,
        lo: if lo > hi { 0.0 } else { lo },
        hi: if lo > hi { 0.0 } else { hi },
        edge: edge_of(head, follicle),
    }
}

/// The narrowest edge the region has anywhere, in metres.
///
/// Walked round the head at every height, as the distance travelled over the
/// surface per unit of weight gained — inverted into the width of the 0.1-to-0.9
/// crossing, which is what a person means by how soft an edge is.
///
/// **Measured per millimetre travelled and not per sample**, because a sweep
/// round a head crosses a near-vertical boundary like the temple's far faster
/// in degrees than in millimetres, and a per-sample figure would report the
/// sweep rather than the mask. See the module header for why a diagonal
/// boundary reads tighter here than it is.
fn edge_of(head: &Head, follicle: Follicle) -> f32 {
    let (throat, crown) = head.skull.throat_and_crown();
    let point = |angle: f32, height: f32| {
        let half = head.skull.half_width(height).max(0.001);
        let across = half * angle.sin();
        let depth = head.skull.depth_across(height, across.abs());
        Vec3::new(across, height, depth * angle.cos())
    };
    let mut steepest: f32 = 0.0;
    for step in 0..300 {
        let height = throat + (crown - throat) * step as f32 / 299.0;
        for turn in 0..360 {
            let here = std::f32::consts::TAU * turn as f32 / 360.0;
            let next = here + std::f32::consts::TAU / 360.0;
            let (one, two) = (point(here, height), point(next, height));
            let span = (two - one).length();
            if span < 1e-6 {
                continue;
            }
            let left = head.follicles.weight(follicle, one);
            let right = head.follicles.weight(follicle, two);
            steepest = steepest.max((left - right).abs() / span);
        }
    }
    // A smoothstep spends 0.1 to 0.9 over about 0.58 of its run, and its
    // steepest slope is 1.5 over that run — so the width the eye reads is this
    // rather than the reciprocal of the slope.
    if steepest > 0.0 { 0.58 * 1.5 / steepest } else { 0.0 }
}
