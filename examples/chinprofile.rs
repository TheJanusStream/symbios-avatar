//! Where the lower face's midline profile TURNS, and how finely it is sampled.
//!
//! **The instrument that tells a curve defect from a sampling one** (#158), which
//! `face::skull` has got the wrong way round more than once — the ladder of dark
//! bars was refinement boundaries, the jaw flank's smear is cell size, and the
//! chin reading boxy is where this was written.
//!
//! Bisects the built surface on the midline at a fine pitch and differences it
//! twice. The reach comes off `PolyMesh::contains`, so what it reports is the
//! POLYGON mesh: a run of exactly zero turn is one flat facet, and a spike is a
//! vertex row. A feature whose whole curvature falls between two consecutive
//! rows cannot be drawn however well its profile is authored, and no amount of
//! re-tuning `CHIN` will change that — the answer there is `FACE_PASSES`.
//!
//! ```text
//! cargo run --release --example chinprofile -- 0 3 7 21
//! ```

use symbios_avatar::face::Skull;
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Rig, Zone, build_body,
};

fn main() {
    let seeds: Vec<i64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let seeds = if seeds.is_empty() { vec![-1] } else { seeds };
    for seed in seeds {
        let mut record = AvatarRecord::new("Chin", Archetype::default());
        if seed >= 0 {
            record.reroll(seed);
        }
        let skeleton = record.skeleton();
        let dimorphism = symbios_avatar::face::Dimorphism::of(&record.composites);
        let body = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &dimorphism,
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
            "=== seed {seed}: radius {:.4} m, throat {:.3} crown {:.3} radii, chin {:.4}",
            radius,
            throat / radius,
            crown / radius,
            skull.chin() / radius
        );

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
        while at > -1.30 {
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
            let turn = (second - first).to_degrees();
            let flag = if turn.abs() > 6.0 { "  <<<" } else { "" };
            println!("  {:+.3}   {:.4}   {turn:+6.1}{flag}", b.0, b.1);
        }
    }
}
