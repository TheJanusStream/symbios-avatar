//! Where the lower face TURNS, and how finely it is sampled — down the midline
//! and round the flank.
//!
//! **The instrument that tells a curve defect from a sampling one**, which
//! `face::skull` has got the wrong way round more than once — the ladder of dark
//! bars was refinement boundaries, and the jaw flank's smear is cell size.
//!
//! Bisects the built surface at a fine pitch and differences it twice. The reach
//! comes off `PolyMesh::contains`, so what it reports is the POLYGON mesh: a run
//! of exactly zero turn is one flat facet, and a spike is a vertex row. A feature
//! whose whole curvature falls between two consecutive rows cannot be drawn
//! however well its profile is authored, and no amount of re-tuning `CHIN` will
//! change that — the answer there is `FACE_PASSES`.
//!
//! `--ring` asks the same question the other way round, because the chin's
//! boxiness is one instance of a lower face built of flat planes meeting at a
//! hard edge from the zygomatic down to the jaw, and that edge runs DOWN the
//! face rather than across it. It cuts horizontal sections and
//! walks them by azimuth from dead ahead round past the ear. **The reference
//! turn is the step itself**: a circle sampled every 3° turns 3° per sample, so
//! a row reading 0.0 is a flat facet and one reading 9.0 is three samples' worth
//! of curvature arriving at once. It prints the median head-owned cell beside
//! it, since the two together are the whole diagnosis — a section can only turn
//! where the surface has a row to turn on.
//!
//! **Read the rings below the ear and be careful above it.** The bisection is
//! against the whole built body, so from about 60° out at the zygomatic's own
//! height it is reporting the EAR rather than the skull, and the cell column
//! reads 0.0 mm wherever the faces it lands on are not the head's. The heights
//! at and below the mandible have no ear in them and are the ones the flank's
//! own question is asked at.
//!
//! ```text
//! cargo run --release --example chinprofile -- 0 3 7 21
//! cargo run --release --example chinprofile -- --ring 0 7
//! cargo run --release --example chinprofile -- --ring --femininity 1 0
//! ```

use symbios_avatar::face::Skull;
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Vec3, Zone, build_body,
};

/// The heights `--ring` cuts at, in skull radii above the head joint.
///
/// The span the flank defect covers: the zygomatic arch stands at about +0.10,
/// the mouth line at −0.25, the mandible's border between −0.31 and −0.53, and
/// the jaw's hollow below that. Under −0.60 a section is mostly submental and
/// the neck is inside it.
const RINGS: [f32; 10] = [
    0.10, 0.00, -0.15, -0.30, -0.45, -0.60, -0.90, -1.20, -1.60, -2.00,
];

/// Degrees between samples round a section, and the turn a circle reads.
const SWEEP: f32 = 3.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rings = args.iter().any(|arg| arg == "--ring");
    let femininity = args
        .iter()
        .position(|arg| arg == "--femininity")
        .and_then(|at| args.get(at + 1))
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0);
    // The value after `--femininity` is not a seed, and it is often negative,
    // which is exactly what a seed looks like to `parse`.
    let seeds: Vec<i64> = args
        .iter()
        .enumerate()
        .filter(|(at, _)| *at == 0 || args[at - 1] != "--femininity")
        .filter_map(|(_, arg)| arg.parse().ok())
        .collect();
    let seeds = if seeds.is_empty() { vec![-1] } else { seeds };
    for seed in seeds {
        let mut record = AvatarRecord::new("Chin", Archetype::default());
        if seed >= 0 {
            record.reroll(seed);
        }
        record.composites.femininity = femininity;
        let skeleton = record.skeleton();
        let traits = symbios_avatar::face::HeadTraits::of(&record.composites);
        let body = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &traits,
        )
        .expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let joint = *rig.in_zone(Zone::Head).first().expect("a head");
        let (centre, radius) = (rig.joints[joint].position, rig.joints[joint].radius);
        let Some(skull) = Skull::measure(&body, &rig) else {
            continue;
        };
        let (throat, crown) = skull.throat_and_crown();
        println!(
            "=== seed {seed}, femininity {femininity:+.1}: radius {:.4} m, throat {:.3} crown {:.3} radii, chin {:.4}",
            radius,
            throat / radius,
            crown / radius,
            skull.chin() / radius
        );

        if rings {
            ring(&body, &rig, centre, radius);
            continue;
        }

        // Forward reach on the midline, in metres from the head joint.
        let reach = |y: f32| -> Option<f32> {
            let from = centre + glam::Vec3::Y * y;
            if !body.contains(from) {
                return None;
            }
            let (mut near, mut far) = (0.0f32, radius * 2.5);
            for _ in 0..34 {
                let middle = 0.5 * (near + far);
                if body.contains(from + glam::Vec3::Z * middle) {
                    near = middle;
                } else {
                    far = middle;
                }
            }
            Some(near)
        };

        // The floor remap `reshape_to` and `FACE_PASSES` both work in, so the
        // crest can be quoted in the unit the refinement bands are authored in.
        let floor = skull.throat_and_crown().0 / radius;
        let stretch = floor * 0.92 / -0.70;
        let crest = (0..=200)
            .filter_map(|s| {
                let at = -0.60 - 0.004 * s as f32;
                reach(at * radius).map(|z| (z, at))
            })
            .fold(
                (0.0f32, 0.0f32),
                |best, (z, at)| if z > best.0 { (z, at) } else { best },
            );
        println!(
            "  floor {floor:.3} radii, stretch {stretch:.4}; crest at {:.3} radii = {:.3} profile heights",
            crest.1,
            crest.1 / stretch
        );

        // A fine ladder over the lower face, in radii above the joint.
        let step = 0.01f32;
        let mut rows: Vec<(f32, f32)> = Vec::new();
        let mut at = 0.30f32;
        while at > -2.60 {
            if let Some(z) = reach(at * radius) {
                rows.push((at, z / radius));
            }
            at -= step;
        }

        // The turn between successive segments, in degrees: the corner is
        // wherever this spikes, and its height names the mechanism.
        println!("  height    reach    turn");
        for window in rows.windows(3) {
            let (a, b, c) = (window[0], window[1], window[2]);
            let first = (b.1 - a.1).atan2(b.0 - a.0);
            let second = (c.1 - b.1).atan2(c.0 - b.0);
            let turn = wrapped(second - first);
            let flag = if turn.abs() > 6.0 { "  <<<" } else { "" };
            println!("  {:+.3}   {:.4}   {turn:+6.1}{flag}", b.0, b.1);
        }
    }
}

/// A difference of two `atan2` angles, in degrees, brought into (−180, 180].
///
/// **Without this a smooth minimum reads as a catastrophic corner.** The two
/// branch directions straddle `atan2`'s cut wherever a profile turns around,
/// which is exactly where a chin crests and where a throat stops coming in — so
/// the most interesting rows on the page are exactly the ones a raw difference
/// reports worst. Unwrapped, a −304.2 at the chin's crest stands for +55.8 and
/// a +352.0 in the throat for −8.0, while every reading between −180 and 180 is
/// already right — and a curve is judged by its sign.
fn wrapped(radians: f32) -> f32 {
    let mut turn = radians.to_degrees() % 360.0;
    if turn > 180.0 {
        turn -= 360.0;
    }
    if turn <= -180.0 {
        turn += 360.0;
    }
    turn
}

/// Horizontal sections through the lower face, by azimuth from dead ahead.
///
/// Reach is bisected from the head's own axis, so it is the section's own
/// polar radius and a flat facet reads as a run of zero turn exactly as the
/// midline ladder's does. The cell column is the median mean-edge of the
/// head-owned faces whose centroid falls in the same azimuth bucket and within
/// half a ring spacing of the height, which is the surface the section is cut
/// from rather than a separate measurement of a different thing.
fn ring(body: &PolyMesh, rig: &Rig, centre: Vec3, radius: f32) {
    let steps = (150.0 / SWEEP) as usize;
    for height in RINGS {
        let from = centre + Vec3::Y * (height * radius);
        if !body.contains(from) {
            println!("  height {height:+.2} radii: the axis is outside the surface");
            continue;
        }
        let cells = cells(body, rig, centre, radius, height);
        let reach = |azimuth: f32| -> f32 {
            let along = Vec3::new(azimuth.to_radians().sin(), 0.0, azimuth.to_radians().cos());
            let (mut near, mut far) = (0.0f32, radius * 2.5);
            for _ in 0..34 {
                let middle = 0.5 * (near + far);
                if body.contains(from + along * middle) {
                    near = middle;
                } else {
                    far = middle;
                }
            }
            near / radius
        };
        let section: Vec<(f32, f32)> = (0..=steps)
            .map(|step| {
                let azimuth = step as f32 * SWEEP;
                (azimuth, reach(azimuth))
            })
            .collect();
        println!(
            "  height {height:+.2} radii — a circle turns {SWEEP:.1} deg per step\n    \
             azimuth   reach    turn   cell"
        );
        for window in section.windows(3) {
            let point = |(azimuth, reach): (f32, f32)| {
                let radians = azimuth.to_radians();
                (reach * radians.sin(), reach * radians.cos())
            };
            let (a, b, c) = (point(window[0]), point(window[1]), point(window[2]));
            let first = (b.1 - a.1).atan2(b.0 - a.0);
            let second = (c.1 - b.1).atan2(c.0 - b.0);
            let turn = -wrapped(second - first);
            let flag = if turn > SWEEP * 2.5 {
                "  <<< corner"
            } else if turn < SWEEP * 0.34 {
                "  <<< flat"
            } else {
                ""
            };
            let cell = cells(window[1].0);
            println!(
                "    {:5.0}    {:.4}  {turn:+6.1}   {cell:4.1}mm{flag}",
                window[1].0, window[1].1
            );
        }
    }
}

/// The median head-owned cell size at one height, as a function of azimuth.
///
/// Returned as a closure over a bucketed table rather than measured per row:
/// the mesh has to be walked once whatever the section's pitch is, and a bucket
/// narrower than the cells in it has nothing to take a median of.
fn cells(
    body: &PolyMesh,
    rig: &Rig,
    centre: Vec3,
    radius: f32,
    height: f32,
) -> impl Fn(f32) -> f32 {
    /// Degrees of azimuth per bucket.
    const BUCKET: f32 = 10.0;
    /// How far either side of the section a face may sit and still count, in
    /// radii — half the spacing of `RINGS`.
    const REACH: f32 = 0.075;

    let mut buckets: Vec<Vec<f32>> = vec![Vec::new(); (180.0 / BUCKET) as usize + 1];
    for face in 0..body.face_count() {
        let at = body.face_centroid(face);
        if rig.joints[rig.nearest_bone(at).joint].zone != Zone::Head {
            continue;
        }
        let local = at - centre;
        if (local.y / radius - height).abs() > REACH {
            continue;
        }
        let across = Vec3::new(local.x, 0.0, local.z);
        if across.length() <= f32::EPSILON {
            continue;
        }
        let azimuth = (across.z / across.length())
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        let corners = &body.faces[face];
        let mean = (0..corners.len())
            .map(|corner| {
                body.positions[corners[corner] as usize]
                    .distance(body.positions[corners[(corner + 1) % corners.len()] as usize])
            })
            .sum::<f32>()
            / corners.len() as f32;
        buckets[(azimuth / BUCKET) as usize].push(mean * 1000.0);
    }
    for bucket in &mut buckets {
        bucket.sort_by(f32::total_cmp);
    }
    move |azimuth: f32| {
        let bucket = &buckets[((azimuth / BUCKET) as usize).min(buckets.len() - 1)];
        bucket.get(bucket.len() / 2).copied().unwrap_or(0.0)
    }
}
