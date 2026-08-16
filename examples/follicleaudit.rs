//! What the five follicle regions hold on a built head.
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
//! **It measures the GROWN layer against the mask as well**. The epic's
//! own invariant is that the two layers agree about where hair is, and until this
//! nothing measured whether they did — the sheet showed a brow whose clumps might
//! or might not have been sitting under their own paint, and no amount of
//! squinting settled it. So each region now reports what it grew, the heights the
//! geometry actually occupies against the heights the mask allows, and the share
//! of the hair that is outside its own region altogether.
//!
//! The last of those is only a defect where hair is meant to LIE on the skin — a
//! brow, a stubbled jaw. Hair that hangs free leaves its region by design: a
//! scalp lock's tip is not on the scalp, and reading that column as an error
//! would be reading the instrument rather than the hair.
//!
//! **Every share here is an AREA share, and the reason is worth carrying**
//!. `refine_face` splits the front of the face ten times and leaves the
//! vault at the base subdivision, so a head carries thousands of vertices on a
//! chin and dozens on a crown. Counting vertices, the first cut of these numbers
//! read the scalp — the largest region on any head — as 2.9% of one, and the
//! chin as 25%. That is a measurement of the refinement schedule wearing a
//! mask's name.

use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use symbios_avatar::face::{Canon, Skull};
use symbios_avatar::hair::clump::scatter::scatter;
use symbios_avatar::hair::clump::{Bed, Sowing};
use symbios_avatar::hair::{BrowStyle, Follicle, FollicleParams, Follicles, Growth, ScalpStyle};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, PolyMesh, Rig, Vec3, Zone};

/// The seeds `--sweep` reports, which are `tests/budget.rs`'s own.
const SWEEP_SEEDS: [i64; 8] = [0, 3, 7, 13, 23, 42, 11, 15];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--sweep") {
        sweep();
        return;
    }
    if let Some(slot) = args.iter().position(|arg| arg == "--cards") {
        let region = args.get(slot + 1).map(String::as_str).unwrap_or("scalp");
        let style = args.get(slot + 2).map(String::as_str).unwrap_or("");
        let axis = args
            .get(slot + 3)
            .and_then(|arg| arg.parse::<f32>().ok())
            .unwrap_or(0.6);
        cards(region, style, axis);
        return;
    }
    let seed = args.first().and_then(|arg| arg.parse::<i64>().ok());
    report(seed);
}

/// Every card of one region and style, one row each.
///
/// **The instrument a defect gets when four guesses from a contact sheet have
/// already been wrong.** Two blocky slabs stood off the back-top of the head in
/// every overhead view, and the four hypotheses a sheet can suggest — a walk
/// straying, a meridian that grows nothing, an outline solve diverging, the
/// crown pole — had each been ruled out by a measurement of its own. What a
/// sheet cannot show is which of seventy-five cards they ARE, so this prints
/// every card's own numbers and the outlier names the cause.
///
/// ```text
/// cargo run --release --example follicleaudit -- --cards scalp crop
/// cargo run --release --example follicleaudit -- --cards scalp bob 0.8
/// cargo run --release --example follicleaudit -- --cards moustache handlebar 0.9
/// ```
fn cards(region: &str, style: &str, axis: f32) {
    let Some(follicle) = Follicle::ALL
        .into_iter()
        .find(|other| other.name() == region)
    else {
        println!("unknown region {region}: expected one of scalp, brows, moustache, chin, flanks");
        return;
    };
    let mut record = AvatarRecord::new("Cards", Archetype::default());
    let named = match wear(&mut record, follicle, style, axis) {
        Some(named) => named,
        None => return,
    };
    let Some(avatar) = Avatar::build(&record) else {
        println!("this body has no head to grow hair on");
        return;
    };
    let Some(skull) = Skull::measure(&avatar.parts.body, &avatar.rig) else {
        println!("this body has no head to measure");
        return;
    };
    let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
    let follicles = Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default());
    let Some(sown) = record.hair.sowing(follicle, &follicles) else {
        println!("this record grows no {region} hair");
        return;
    };
    // The shipped roots: one stream from the record's own seed, drawn in
    // `Follicle::ALL` order, which is what `Avatar::build` does — so the regions
    // before this one have to be drawn even though nothing reads them, or these
    // are a second sample from the same distribution rather than the roots that
    // shipped (#89's lesson, in the form `grow` below takes it).
    let mut stream = Pcg64Mcg::seed_from_u64(record.seed as u64);
    let mut roots = Vec::new();
    for other in Follicle::ALL {
        let Some(sown) = record.hair.sowing(other, &follicles) else {
            continue;
        };
        let drawn = scatter(
            &avatar.parts.body,
            &avatar.rig,
            &avatar.parts.weights,
            &follicles,
            other,
            sown.clumps,
            &mut stream,
        );
        if other == follicle {
            roots = drawn;
        }
    }
    let (throat, crown) = skull.throat_and_crown();
    println!("{region} — {named}, {} roots asked for", roots.len());
    println!(
        "  crown {:+.1} mm, throat {:+.1} mm",
        crown * 1000.0,
        throat * 1000.0
    );
    println!();
    println!(
        "    #  azimuth  root mm  weight   len mm  on head   w@.02  w@.25  w@.60  w@1.0  \
         stands off   at mm"
    );
    let mut rows: Vec<Card> = roots
        .iter()
        .filter_map(|root| {
            card(
                sown.shape.as_ref(),
                root,
                &follicles,
                follicle,
                &skull,
                throat,
                crown,
            )
        })
        .collect();
    let trace = std::env::args().any(|arg| arg == "--trace");
    // Worst standoff first, because the question this answers is which cards are
    // the outliers and a row order of scatter order buries them.
    rows.sort_by(|one, two| two.stands.total_cmp(&one.stands));
    for (index, row) in rows.iter().enumerate() {
        println!(
            "  {:3}  {:+7.0}  {:+7.1}   {:5.3}  {:7.1}   {:5.1}%  {:6.1} {:6.1} {:6.1} {:6.1}  \
             {:8.1}  {:+7.1}",
            index,
            row.azimuth.to_degrees(),
            row.height * 1000.0,
            row.weight,
            row.length * 1000.0,
            row.on_head * 100.0,
            row.widths[0] * 1000.0,
            row.widths[1] * 1000.0,
            row.widths[2] * 1000.0,
            row.widths[3] * 1000.0,
            row.stands * 1000.0,
            row.stands_at * 1000.0,
        );
    }
    if trace {
        for row in rows.iter().take(2) {
            println!();
            println!(
                "  the walk of the card rooted at {:+.0} deg, {:+.1} mm:",
                row.azimuth.to_degrees(),
                row.height * 1000.0
            );
            println!("    along   height   azimuth   radius  profile   stands off   mask");
            for station in 0..=32 {
                let along = station as f32 / 32.0;
                let at = sown.shape.at(&row.root, along);
                let azimuth = at.x.atan2(at.z);
                let profile = skull.surface_at(at.y.clamp(throat, crown), azimuth);
                println!(
                    "    {:5.2}  {:+7.1}  {:+8.0}  {:7.1}  {:7.1}     {:+8.1}  {:5.3}",
                    along,
                    at.y * 1000.0,
                    azimuth.to_degrees(),
                    (at.x * at.x + at.z * at.z).sqrt() * 1000.0,
                    (profile.x * profile.x + profile.z * profile.z).sqrt() * 1000.0,
                    ((at.x * at.x + at.z * at.z).sqrt()
                        - (profile.x * profile.x + profile.z * profile.z).sqrt())
                        * 1000.0,
                    follicles.weight(follicle, at),
                );
            }
        }
    }
    println!();
    println!(
        "  on head    — share of the card's own stations standing where the scalp mask is over\n  \
         {:12}a third, which is what the walk calls its cap\n  \
         w@         — half-width a share of the way along it, in mm. A card fans out from the\n  \
         {:12}crown, so a low first column and a high last one is the healthy shape; a\n  \
         {:12}card at full width from its first station is the slab shape\n  \
         stands off — the furthest any station STILL ON THE MASK sits outside the measured\n  \
         {:12}profile at its own azimuth, and the height it does it at. Hair past the\n  \
         {:12}hairline is draping, and owes the profile nothing",
        "", "", "", "", "",
    );
}

/// Dresses one record's region in one named style, and says what it wore.
///
/// The empty name leaves whatever the record already asks for, which for the
/// regions whose catalogue is one style is the only thing to say.
fn wear(record: &mut AvatarRecord, follicle: Follicle, style: &str, axis: f32) -> Option<String> {
    use symbios_avatar::hair::{ChinStyle, FlankStyle, MoustacheStyle};
    match follicle {
        Follicle::Scalp => {
            record.hair.scalp.style = match style {
                "" | "crop" => ScalpStyle::Crop,
                "bob" => ScalpStyle::Bob { fringe: axis },
                "long" => ScalpStyle::Long { weight: axis },
                "tied" => ScalpStyle::TiedBack { tail: axis },
                "curly" => ScalpStyle::Curly { curl: axis },
                other => return unknown(other, "crop, bob, long, tied or curly"),
            };
            Some(format!("{:?}", record.hair.scalp.style))
        }
        Follicle::Brows => {
            record.hair.brows.style = match style {
                "" | "natural" => BrowStyle::Natural,
                "thick" => BrowStyle::Thick,
                other => return unknown(other, "natural or thick"),
            };
            Some(format!("{:?}", record.hair.brows.style))
        }
        Follicle::Moustache => {
            record.hair.moustache.style = match style {
                "" | "chevron" => MoustacheStyle::Chevron,
                "handlebar" => MoustacheStyle::Handlebar { sweep: axis },
                "pencil" => MoustacheStyle::Pencil { ride: axis },
                other => return unknown(other, "chevron, handlebar or pencil"),
            };
            Some(format!("{:?}", record.hair.moustache.style))
        }
        Follicle::Chin => {
            record.hair.chin.style = match style {
                "" | "full" => ChinStyle::Full,
                "goatee" => ChinStyle::Goatee { point: axis },
                "braided" => ChinStyle::Braided { twist: axis },
                other => return unknown(other, "goatee, full or braided"),
            };
            Some(format!("{:?}", record.hair.chin.style))
        }
        Follicle::Flanks => {
            record.hair.flanks.style = match style {
                "" | "full" => FlankStyle::FullConnect { reach: axis },
                "sideburns" => FlankStyle::Sideburns { drop: axis },
                other => return unknown(other, "sideburns or full"),
            };
            Some(format!("{:?}", record.hair.flanks.style))
        }
    }
}

/// Says what was expected, and grows nothing.
fn unknown(style: &str, expected: &str) -> Option<String> {
    println!("unknown style {style}: expected {expected}");
    None
}

/// What one card is, measured off the shape the record asked for.
struct Card {
    /// The root it grew from, so a trace can re-walk it.
    root: symbios_avatar::hair::Root,
    azimuth: f32,
    height: f32,
    weight: f32,
    length: f32,
    /// Share of its stations still on the scalp mask.
    on_head: f32,
    /// Half-width at four shares of the way along it, in metres.
    widths: [f32; 4],
    /// The furthest any station stands outside the profile, in metres.
    stands: f32,
    /// The height it does that at.
    stands_at: f32,
}

/// Measures one card, or `None` if the style declined this root.
fn card(
    shape: &dyn symbios_avatar::hair::Shape,
    root: &symbios_avatar::hair::Root,
    follicles: &Follicles,
    follicle: Follicle,
    skull: &Skull,
    throat: f32,
    crown: f32,
) -> Option<Card> {
    let length = shape.length(root);
    if length <= f32::EPSILON {
        return None;
    }
    // Evenly, and finely: this is not the loft's adaptive sampling and is not
    // meant to be. A card that stands off the head does it somewhere along its
    // own curve, and an even walk cannot miss the place the way a sampler that
    // spends its stations on curvature can.
    const STATIONS: usize = 128;
    let mut on_head = 0usize;
    let mut stands = f32::MIN;
    let mut stands_at = 0.0;
    for station in 0..=STATIONS {
        let along = station as f32 / STATIONS as f32;
        let at = shape.at(root, along);
        let on = follicles.weight(follicle, at) >= 0.35;
        if on {
            on_head += 1;
        }
        // **Only over the part of the card that is still meant to be LYING on
        // the scalp**, which is the difference between the defect and the
        // design. A card keeps the widest radius it has passed once the head
        // starts coming back in — that is hair draping, and it stands 20 mm off
        // a nape by intention. Measuring the whole card would report every
        // healthy crop as the worst offender on the head.
        if !on || at.y < throat {
            continue;
        }
        let azimuth = at.x.atan2(at.z);
        let profile = skull.surface_at(at.y.clamp(throat, crown), azimuth);
        let out = (at.x * at.x + at.z * at.z).sqrt()
            - (profile.x * profile.x + profile.z * profile.z).sqrt();
        if out > stands {
            stands = out;
            stands_at = at.y;
        }
    }
    let widths = [0.02, 0.25, 0.60, 1.0].map(|along| shape.width_at(root, along));
    Some(Card {
        root: *root,
        azimuth: root.at.x.atan2(root.at.z),
        height: root.at.y,
        weight: root.weight,
        length,
        on_head: on_head as f32 / (STATIONS + 1) as f32,
        widths,
        stands: if stands == f32::MIN { 0.0 } else { stands },
        stands_at,
    })
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
    let ridge = head.follicles.brow_ridge();
    println!(
        "  the brow ridge runs {:.1} mm to {:.1} mm out from the midline — a span of {:.1} — at \
         {:+.1} mm, arching {:.1}",
        ridge.inner * 1000.0,
        ridge.outer * 1000.0,
        ridge.span() * 1000.0,
        ridge.level * 1000.0,
        ridge.arch * 1000.0,
    );
    println!();
    println!("  region      clumps    tris  per   grown band, mm     off-mask");
    for follicle in Follicle::ALL {
        let Some(grown) = &head.grown[slot_of(follicle)] else {
            println!("  {:10}      — nothing grown", follicle.name());
            continue;
        };
        println!(
            "  {:10} {:5}  {:6}  {:3}   {:+6.1} to {:+6.1}     {:5.1}%",
            follicle.name(),
            grown.clumps,
            grown.tris,
            grown.tris / grown.clumps.max(1),
            grown.lo * 1000.0,
            grown.hi * 1000.0,
            grown.outside * 100.0,
        );
    }
    println!();
    // **The brow's own line, because that is the invariant #205 could not read
    // off a render.** The general column beside it cannot separate hair standing
    // off its line from hair reaching past the tail, and only one of those is a
    // defect: the mask's lateral fade is exactly where a brow's last hairs
    // belong. The ridge is a public line, so for this region the two can be
    // asked separately, and the answer is a distance rather than a share.
    if let Some(off) = off_the_ridge(&head) {
        println!(
            "  brow hair stands {:.1} mm off the ridge on average and {:.1} mm at worst, \
             against a band {:.1} mm deep — {:.1}% of it outside that",
            off.mean * 1000.0,
            off.worst * 1000.0,
            ridge.thick * 1000.0,
            off.beyond * 100.0,
        );
        println!();
    }
    println!(
        "  per        — triangles a clump, whose floor is 14: a straight one, capped both ends\n  \
         grown band — the heights the GEOMETRY occupies. NOT to be read against the mask band\n  \
         {:15}above it: that one is the >0.5 core measured over the surface's own faces, and\n  \
         {:15}comparing the two says a brow has left its region when it has not\n  \
         off-mask   — share of the hair standing where its region's weight is under 0.1. Hair\n  \
         {:15}that hangs free is expected to leave — a scalp lock's tip is not on the\n  \
         {:15}scalp — and so is the last hair past a brow's tail",
        "", "", "", "",
    );
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
    /// What each region grew, in [`Follicle::ALL`] order, or `None` for a region
    /// this record asks for no geometry in.
    grown: [Option<Grown>; 5],
    /// The brow geometry's own vertices, head-local, for the ridge measurement.
    brows: Option<Vec<Vec3>>,
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

/// What one region actually grew.
struct Grown {
    clumps: usize,
    tris: usize,
    /// The heights its geometry occupies, in head-local metres.
    lo: f32,
    hi: f32,
    /// Share of its vertices standing where the region's own weight is under 0.1.
    outside: f32,
}

/// How far the grown brows stand off the ridge their paint is centred on.
struct OffRidge {
    /// Mean distance from the line, in metres.
    mean: f32,
    /// The worst any vertex is.
    worst: f32,
    /// Share of vertices further off it than the band is deep.
    beyond: f32,
}

/// Measures the grown brows against the ridge, or `None` if none are grown.
fn off_the_ridge(head: &Head) -> Option<OffRidge> {
    let ridge = head.follicles.brow_ridge();
    let hair = head.brows.as_ref()?;
    if hair.is_empty() {
        return None;
    }
    let mut total = 0.0f32;
    let mut worst = 0.0f32;
    let mut beyond = 0usize;
    for at in hair {
        let off = (at.y - ridge.height(ridge.along(at.x))).abs();
        total += off;
        worst = worst.max(off);
        if off > ridge.thick {
            beyond += 1;
        }
    }
    Some(OffRidge {
        mean: total / hair.len() as f32,
        worst,
        beyond: beyond as f32 / hair.len() as f32,
    })
}

/// Where one region sits in [`Follicle::ALL`].
fn slot_of(follicle: Follicle) -> usize {
    Follicle::ALL
        .iter()
        .position(|other| *other == follicle)
        .unwrap_or(0)
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
    let (grown, brows) = grow(
        &record,
        &avatar.parts.body,
        &avatar.rig,
        &avatar.parts.weights,
        &follicles,
    );
    Some(Head {
        grown,
        brows,
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

/// Grows the record's own hair, one region at a time.
///
/// **Region by region, and from one stream in [`Follicle::ALL`]'s order**, which
/// is exactly what `Avatar::build` does — so the roots measured here are the roots
/// that shipped, not a second sample from the same distribution. A fresh stream
/// per region would draw different hair and this would be measuring an
/// instrument rather than the artifact.
fn grow(
    record: &AvatarRecord,
    body: &PolyMesh,
    rig: &Rig,
    weights: &symbios_avatar::SkinWeights,
    follicles: &Follicles,
) -> ([Option<Grown>; 5], Option<Vec<Vec3>>) {
    let bed = Bed {
        body,
        rig,
        weights,
        follicles,
    };
    let mut stream = Pcg64Mcg::seed_from_u64(record.seed as u64);
    let mut grown = [None, None, None, None, None];
    let mut brows = None;
    for follicle in Follicle::ALL {
        let Some(sown) = record.hair.sowing(follicle, follicles) else {
            continue;
        };
        let mut growth = Growth::on(follicles.head);
        growth.grow(
            &bed,
            &Sowing {
                follicle,
                count: sown.clumps,
                shape: sown.shape.as_ref(),
                roots: Vec3::from_array(sown.roots),
                tips: Vec3::from_array(sown.tips),
            },
            &mut stream,
        );
        let Some(ledger) = growth.grown.first() else {
            continue;
        };
        let (lo, hi) = growth
            .mesh
            .positions
            .iter()
            .fold((f32::MAX, f32::MIN), |span, at| {
                (span.0.min(at.y), span.1.max(at.y))
            });
        let outside = growth
            .mesh
            .positions
            .iter()
            .filter(|at| follicles.weight(follicle, **at) < 0.1)
            .count() as f32
            / growth.mesh.positions.len().max(1) as f32;
        grown[slot_of(follicle)] = Some(Grown {
            clumps: ledger.clumps,
            tris: ledger.tris,
            lo,
            hi,
            outside,
        });
        if follicle == Follicle::Brows {
            brows = Some(growth.mesh.positions.clone());
        }
    }
    (grown, brows)
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
                    let one = body.positions[corners[step] as usize]
                        - body.positions[corners[0] as usize];
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
    // **The library's own surface, not a second copy of it** (#204). This was
    // four lines assembling the two profile tables by hand — the same four lines
    // `Skull::surface_at` is, except that it used the FORWARD reach for the back of
    // the head, which reads the occiput as inside itself. Two copies of one
    // formula is how one of them stays wrong.
    let point = |angle: f32, height: f32| head.skull.surface_at(height, angle);
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
    if steepest > 0.0 {
        0.58 * 1.5 / steepest
    } else {
        0.0
    }
}
