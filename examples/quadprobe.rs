//! Measures a quadruped's built body against the anatomy dossier (#374, #375).
//!
//! Every figure is a fraction of the withers height, read off the BUILT
//! surface (cage, subdivision and the beast's shaping), never off the nodes:
//! a cage ring is not where the skin is. The bones and their angles are read
//! off the skeleton, because a bone is a joint-to-joint length.
//!
//! The targets it is read against, per family, are docs/quadruped-anatomy.md.
//!
//! ```text
//! cargo run --release --example quadprobe                 # dog, cat, horse at their anchors
//! cargo run --release --example quadprobe -- --appeal 1   # the same at the roll's default appeal
//! cargo run --release --example quadprobe -- --mass 1 --fat 0.45  # the same, heavier and fatter
//! cargo run --release --example quadprobe -- --seed 1     # one rolled record
//! cargo run --release --example quadprobe -- --seed 1 --girdle  # and its girdles' nodes, for a socket failure
//! cargo run --release --example quadprobe -- --sweep 3000 # how many of 3,000 rolls mesh, why not, and in how many pieces
//! ```
//!
//! What it measures, and where:
//! - **WH**: the top of the trunk's section over the fore elbow.
//! - **Sections**: the trunk's loop where a plane across the body cuts the
//!   mesh, chosen as the loop round the spine's own line, so a leg or a tail
//!   cut by the same plane is never counted. Girth is its perimeter.
//! - **Heart girth**: the section just behind the elbow; **ribcage**: the
//!   largest girth ahead of the trunk's middle; **loin**: the section over the
//!   trunk's last node before the pelvis.
//! - **Trunk length**: front at the height of the point of the shoulder, back
//!   at the hindmost the trunk reaches between 0.55 and 0.75 WH, under the
//!   tail's root (the tail is the same surface, so a line through it would
//!   run on along it).
//! - **Neck**: its length from the top of the withers to the occiput joint,
//!   and its girth where the plane square to its run cuts it at the neck
//!   node, against the heart girth; the head's length is its nodes' extent
//!   fore and aft, a comparison between bodies rather than a skull's length.
//! - **Thigh and gaskin**: the hind leg's extent fore and aft and across
//!   where a level plane cuts it three quarters of the way down the femur
//!   (under the belly) and halfway down the tibia - how thick they read from
//!   the side.

use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, Limb, QuadrupedParams, Skeleton, Vec2,
    Vec3, Zone, build_body,
    face::HeadTraits,
    mesh::PolyMesh,
    plan::{Composites, DEFAULT_BODY_FAT, Family},
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
            .cloned()
    };
    let appeal = value("--appeal").and_then(|v| v.parse::<f32>().ok());
    // `--mass` and `--fat`: the anchors' composites replaced, to read what
    // the composites do to a family's body.
    let mass = value("--mass").and_then(|v| v.parse::<f32>().ok());
    let fat = value("--fat").and_then(|v| v.parse::<f32>().ok());
    // `--sweep N`: how many of N rolled beasts the cage meshes, and why the
    // rest do not; and how many mesh into more than one piece.
    if let Some(count) = value("--sweep").and_then(|v| v.parse::<i64>().ok()) {
        let (mut failed, mut split) = (0, 0);
        for seed in 0..count {
            let record = AvatarRecord::rolled(
                "Probe",
                Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            );
            let cage = symbios_avatar::build_cage(&record.skeleton(), &CageConfig::default());
            if let Ok(cage) = &cage {
                let pieces = pieces(cage);
                if pieces > 1 {
                    split += 1;
                    println!("seed {seed}: {pieces} separate pieces");
                }
            }
            if let Err(error) = cage {
                failed += 1;
                if let Archetype::Quadruped(params) = &record.archetype {
                    println!(
                        "seed {seed} {:?}/{:?} appeal {:.2}, mass {:.2}, fat {:.2}: {error}",
                        params.family,
                        params.hybrid,
                        params.appeal,
                        record.composites.mass,
                        record.composites.body_fat
                    );
                }
            }
        }
        println!("{failed} of {count} rolls do not mesh");
        println!("{split} of {count} rolls mesh into more than one piece");
        return;
    }
    if let Some(seed) = value("--seed").and_then(|v| v.parse::<i64>().ok()) {
        let record = AvatarRecord::rolled(
            "Probe",
            Archetype::Quadruped(QuadrupedParams::default()),
            seed,
        );
        if let Archetype::Quadruped(params) = &record.archetype {
            println!(
                "seed {seed}: {:?} {:?} blend {:.2}, appeal {:.2}, mass {:.2}",
                params.family, params.hybrid, params.blend, params.appeal, record.composites.mass
            );
        }
        if args.iter().any(|arg| arg == "--girdle") {
            girdle(&record);
        }
        measure(&record);
        return;
    }
    for family in [Family::Dog, Family::Cat, Family::Horse] {
        let mut params = QuadrupedParams::default();
        params.set_kind(family, None, 0.0);
        if let Some(appeal) = appeal {
            params.appeal = appeal;
        }
        let mut record = AvatarRecord::new("Probe", Archetype::Quadruped(params));
        // The family's own composites, as a roll centres them.
        let (centre_mass, centre_fat) = match family {
            Family::Dog => (0.0, DEFAULT_BODY_FAT),
            Family::Cat => (-0.4, DEFAULT_BODY_FAT + 0.02),
            Family::Horse => (0.6, DEFAULT_BODY_FAT - 0.06),
        };
        record.composites = Composites {
            mass: mass.unwrap_or(centre_mass),
            body_fat: fat.unwrap_or(centre_fat),
            ..record.composites
        };
        println!(
            "\n=== {family:?} (appeal {:.2}, mass {:.2}, fat {:.2}) ===",
            params.appeal, record.composites.mass, record.composites.body_fat
        );
        measure(&record);
    }
}

/// One trunk section: where it was cut and what the loop measured.
#[derive(Clone, Copy, Debug)]
struct Section {
    z: f32,
    girth: f32,
    top: f32,
    bottom: f32,
    width: f32,
}

impl Section {
    fn depth(&self) -> f32 {
        self.top - self.bottom
    }
}

fn measure(record: &AvatarRecord) {
    let skeleton = record.skeleton();
    let traits = HeadTraits::of(&record.composites);
    let mesh = match build_body(
        &skeleton,
        &CageConfig::default(),
        BODY_SUBDIVISIONS,
        &traits,
    ) {
        Ok(mesh) => mesh,
        Err(error) => {
            println!("DOES NOT MESH: {error}");
            return;
        }
    };
    let report = mesh.manifold_report();
    println!(
        "mesh: {} vertices, {} faces, clean manifold {}",
        mesh.positions.len(),
        mesh.faces.len(),
        report.is_clean()
    );

    let fore = leg(&skeleton, Limb::ForeLeft);
    let hind = leg(&skeleton, Limb::HindLeft);
    // The elbow: the last upper-limb node; the carpus the first lower-limb one.
    let elbow = fore
        .iter()
        .rposition(|&(zone, _)| zone == Zone::UpperLimb(Limb::ForeLeft));
    let stifle = hind
        .iter()
        .rposition(|&(zone, _)| zone == Zone::UpperLimb(Limb::HindLeft));
    let (Some(elbow), Some(_stifle)) = (elbow, stifle) else {
        println!("no elbow or stifle");
        return;
    };
    let elbow_at = fore[elbow].1;
    let spine = trunk_line(&skeleton);
    // The withers: the top of the section over the front girdle, the trunk's
    // front node.
    let withers = section(&mesh, &spine, spine[0].z)
        .or_else(|| section(&mesh, &spine, 0.5 * (spine[0].z + spine[1].z)));
    let Some(withers) = withers else {
        println!("no trunk loop over the withers");
        return;
    };
    let heart = withers;
    let wh = withers.top;
    let unit = |value: f32| value / wh;

    // A section a little behind both the withers and the elbow is the heart
    // girth.
    let heart_z = elbow_at.z.min(spine[0].z) - 0.06 * wh;
    let heart = section(&mesh, &spine, heart_z).unwrap_or(heart);
    let front_z = spine.first().map_or(0.0, |p| p.z);
    let back_z = spine.last().map_or(0.0, |p| p.z);
    let middle = 0.5 * (front_z + back_z);
    let cut = |from: f32, to: f32, steps: usize| -> Vec<Section> {
        (0..=steps)
            .filter_map(|i| section(&mesh, &spine, from + (to - from) * i as f32 / steps as f32))
            .collect()
    };
    let ribs = cut(heart_z, middle, 24)
        .into_iter()
        .max_by(|a, b| a.girth.total_cmp(&b.girth))
        .unwrap_or(heart);
    // The loin: the section over the trunk's last node before the pelvis.
    let loin = section(&mesh, &spine, spine[spine.len() - 2].z).unwrap_or(ribs);

    println!("WH {wh:.3} m");
    println!(
        "{:<12} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "section", "z/WH", "girth", "depth", "width", "d:w", "bottom"
    );
    for (name, s) in [("heart", heart), ("ribcage", ribs), ("loin", loin)] {
        println!(
            "{name:<12} {:>7.3} {:>7.3} {:>7.3} {:>7.3} {:>7.2} {:>7.3}",
            unit(s.z),
            unit(s.girth),
            unit(s.depth()),
            unit(s.width),
            s.depth() / s.width.max(1e-6),
            unit(s.bottom)
        );
    }
    println!(
        "chest depth {:.3}  brisket {:.3}  elbow joint {:.3}  loin/ribcage girth {:.2}",
        unit(wh - heart.bottom),
        unit(heart.bottom),
        unit(elbow_at.y),
        loin.girth / ribs.girth
    );
    let shoulder_y = if elbow >= 2 {
        fore[elbow - 1].1.y
    } else {
        0.65 * wh
    };
    // Front at the point of the shoulder's height; back at the hindmost the
    // trunk reaches at any height of the buttock's.
    let front = run(&mesh, shoulder_y, middle).map(|(_, front)| front);
    let back = (11..=15)
        .filter_map(|twentieth| run(&mesh, twentieth as f32 * 0.05 * wh, middle))
        .map(|(back, _)| back)
        .reduce(f32::min);
    if let (Some(front), Some(back)) = (front, back) {
        println!(
            "trunk length {:.3} (front at y {:.3}: {:.3} ahead of the shoulder; back {:.3} behind the hip)",
            unit(front - back),
            unit(shoulder_y),
            unit(front - fore[elbow.saturating_sub(1)].1.z),
            unit(hind[0].1.z - back)
        );
    }
    // The trunk's profile, every tenth of its length.
    let profile: Vec<String> = cut(front_z, back_z, 10)
        .iter()
        .map(|s| {
            format!(
                "{:.2}/{:.2}/{:.2}",
                unit(s.top),
                unit(s.bottom),
                unit(s.width)
            )
        })
        .collect();
    println!(
        "profile top/bottom/width, front to back: {}",
        profile.join(" ")
    );

    // The neck: its length from the top of the withers to the occiput, and
    // its girth where the plane square to its run cuts it at the neck node.
    let neck_node = skeleton.nodes.iter().find(|node| node.zone == Zone::Neck);
    let head_nodes: Vec<Vec3> = skeleton
        .nodes
        .iter()
        .filter(|node| node.zone == Zone::Head && !node.marker)
        .map(|node| node.position)
        .collect();
    if let (Some(neck), Some(&occiput)) = (neck_node, head_nodes.first()) {
        let withers_top = Vec3::new(0.0, withers.top, spine[0].z);
        let run = occiput - spine[0];
        let girth = cut_plane(&mesh, neck.position, run, Vec3::X)
            .into_iter()
            .find(|ring| contains(ring, Vec2::ZERO))
            .map(|ring| perimeter(&ring));
        let head_length = skeleton
            .nodes
            .iter()
            .filter(|node| node.zone == Zone::Head && !node.marker)
            .map(|node| node.position.z + node.radius)
            .fold(f32::MIN, f32::max)
            - skeleton
                .nodes
                .iter()
                .filter(|node| node.zone == Zone::Head && !node.marker)
                .map(|node| node.position.z - node.radius)
                .fold(f32::MAX, f32::min);
        println!(
            "neck length {:.3} (withers top to occiput), mid-neck girth {:.3} = {:.2} of the heart's; head length {:.3} (its nodes, fore and aft)",
            unit(withers_top.distance(occiput)),
            unit(girth.unwrap_or(0.0)),
            girth.unwrap_or(0.0) / heart.girth,
            unit(head_length)
        );
    }
    // The thigh and the gaskin from the side: the hind leg's run fore and aft
    // where a level plane cuts it halfway down the femur and the tibia.
    if hind.len() >= 3 {
        // The thigh is cut three quarters of the way down, under the belly.
        for (name, from, to, share) in [
            ("thigh", hind[0].1, hind[1].1, 0.75),
            ("gaskin", hind[1].1, hind[2].1, 0.5),
        ] {
            let middle = from + (to - from) * share;
            let loops = cut_plane(&mesh, middle, Vec3::Y, Vec3::X);
            // The plane's own origin is the bone's middle.
            if let Some(ring) = loops.into_iter().find(|ring| contains(ring, Vec2::ZERO)) {
                let (low, high) = ring
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.y), h.max(p.y)));
                let (left, right) = ring
                    .iter()
                    .fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.x), h.max(p.x)));
                println!(
                    "{name}: fore-aft {:.3}, across {:.3} at height {:.3}",
                    unit(high - low),
                    unit(right - left),
                    unit(middle.y)
                );
            } else {
                println!(
                    "{name}: no loop round the bone at height {:.3}",
                    unit(middle.y)
                );
            }
        }
    }

    for (name, chain) in [("fore", &fore), ("hind", &hind)] {
        let lengths: Vec<String> = chain
            .windows(2)
            .map(|pair| format!("{:.3}", unit(pair[0].1.distance(pair[1].1))))
            .collect();
        let heights: Vec<String> = chain
            .iter()
            .map(|&(_, p)| format!("{:.3}", unit(p.y)))
            .collect();
        let lateral: Vec<String> = chain
            .iter()
            .map(|&(_, p)| format!("{:.3}", unit(p.x)))
            .collect();
        let angles: Vec<String> = chain
            .windows(3)
            .map(|three| {
                let a = three[0].1 - three[1].1;
                let b = three[2].1 - three[1].1;
                format!("{:.0}", a.angle_between(b).to_degrees())
            })
            .collect();
        println!("{name}: bones {}", lengths.join(" "));
        println!("{name}: heights {}", heights.join(" "));
        println!("{name}: lateral {}", lateral.join(" "));
        println!("{name}: interior angles {}", angles.join(" "));
    }
    if elbow >= 2 {
        let scapula = fore[elbow - 1].1 - fore[elbow - 2].1;
        let incline = (-scapula.y).atan2(scapula.z.abs()).to_degrees();
        println!("scapula inclination {incline:.0} deg from horizontal");
    }
}

/// One leg's nodes from the body down, with their zones.
fn leg(skeleton: &Skeleton, limb: Limb) -> Vec<(Zone, Vec3)> {
    skeleton
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.zone, Zone::UpperLimb(l) | Zone::LowerLimb(l) | Zone::Extremity(l) if l == limb)
        })
        .map(|node| (node.zone, node.position))
        .collect()
}

/// The trunk's nodes, front to back.
fn trunk_line(skeleton: &Skeleton) -> Vec<Vec3> {
    let mut line: Vec<Vec3> = skeleton
        .nodes
        .iter()
        .filter(|node| matches!(node.zone, Zone::Pelvis | Zone::Abdomen | Zone::Chest))
        .filter(|node| !node.marker)
        .map(|node| node.position)
        .collect();
    line.sort_by(|a, b| b.z.total_cmp(&a.z));
    line
}

/// The spine's height at `z`, interpolated along the trunk's nodes.
fn spine_y(spine: &[Vec3], z: f32) -> f32 {
    for pair in spine.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if z <= a.z && z >= b.z {
            let t = (a.z - z) / (a.z - b.z).max(1e-6);
            return a.y + (b.y - a.y) * t;
        }
    }
    if z > spine[0].z {
        spine[0].y
    } else {
        spine[spine.len() - 1].y
    }
}

/// The trunk's loop where the plane at `z` cuts the mesh.
fn section(mesh: &PolyMesh, spine: &[Vec3], z: f32) -> Option<Section> {
    let loops = cut_loops(mesh, z);
    let inside = Vec2::new(0.0, spine_y(spine, z));
    let ring = loops.into_iter().find(|ring| contains(ring, inside))?;
    let girth = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| a.distance(*b))
        .sum();
    let (mut top, mut bottom, mut low_x, mut high_x) = (f32::MIN, f32::MAX, f32::MAX, f32::MIN);
    for p in &ring {
        top = top.max(p.y);
        bottom = bottom.min(p.y);
        low_x = low_x.min(p.x);
        high_x = high_x.max(p.x);
    }
    Some(Section {
        z,
        girth,
        top,
        bottom,
        width: high_x - low_x,
    })
}

/// Every closed loop the plane `z = at` cuts through the mesh, as `(x, y)`.
fn cut_loops(mesh: &PolyMesh, at: f32) -> Vec<Vec<Vec2>> {
    cut_plane(mesh, Vec3::new(0.0, 0.0, at), Vec3::Z, Vec3::X)
}

/// Every closed loop the plane through `origin` facing `normal` cuts through
/// the mesh, in the plane's own `(across, up)` coordinates: `across` along
/// `side` (made square to the normal), `up` square to both.
fn cut_plane(mesh: &PolyMesh, origin: Vec3, normal: Vec3, side: Vec3) -> Vec<Vec<Vec2>> {
    use std::collections::HashMap;
    let normal = normal.normalize();
    let across = (side - normal * side.dot(normal)).normalize();
    let up = normal.cross(across);
    let flat = |p: Vec3| Vec2::new((p - origin).dot(across), (p - origin).dot(up));
    let key = |a: u32, b: u32| (a.min(b), a.max(b));
    let mut point: HashMap<(u32, u32), Vec2> = HashMap::new();
    let mut links: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for face in &mesh.faces {
        let mut crossings = Vec::new();
        for (i, &a) in face.iter().enumerate() {
            let b = face[(i + 1) % face.len()];
            let (pa, pb) = (mesh.positions[a as usize], mesh.positions[b as usize]);
            let (da, db) = ((pa - origin).dot(normal), (pb - origin).dot(normal));
            if (da < 0.0) != (db < 0.0) {
                let t = da / (da - db);
                let p = pa + (pb - pa) * t;
                point.insert(key(a, b), flat(p));
                crossings.push(key(a, b));
            }
        }
        if crossings.len() == 2 {
            links.entry(crossings[0]).or_default().push(crossings[1]);
            links.entry(crossings[1]).or_default().push(crossings[0]);
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut loops = Vec::new();
    let mut starts: Vec<(u32, u32)> = links.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if !seen.insert(start) {
            continue;
        }
        let mut ring = vec![point[&start]];
        let mut previous = start;
        let mut here = links[&start][0];
        while here != start && seen.insert(here) {
            ring.push(point[&here]);
            let next = links[&here]
                .iter()
                .copied()
                .find(|&n| n != previous)
                .unwrap_or(start);
            previous = here;
            here = next;
        }
        if ring.len() > 2 {
            loops.push(ring);
        }
    }
    loops
}

/// A closed loop's length.
fn perimeter(ring: &[Vec2]) -> f32 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| a.distance(*b))
        .sum()
}

/// Whether `point` lies inside the closed polygon `ring`.
fn contains(ring: &[Vec2], point: Vec2) -> bool {
    let mut inside = false;
    for (i, a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        if (a.y > point.y) != (b.y > point.y) {
            let x = a.x + (point.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if point.x < x {
                inside = !inside;
            }
        }
    }
    inside
}

/// The run of the trunk along the midline at height `y`, back to front: the
/// stretch of the line along `z` inside the mesh that contains `middle`.
fn run(mesh: &PolyMesh, y: f32, middle: f32) -> Option<(f32, f32)> {
    let mut crossings = Vec::new();
    for face in &mesh.faces {
        for i in 1..face.len() - 1 {
            let tri = [face[0], face[i], face[i + 1]].map(|v| mesh.positions[v as usize]);
            if let Some(z) = cross_line(tri, y, 1e-4) {
                crossings.push(z);
            }
        }
    }
    crossings.sort_by(f32::total_cmp);
    // Pairs of crossings bound the runs inside a closed surface.
    crossings
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .find(|pair| pair[0] <= middle && middle <= pair[1])
        .map(|pair| (pair[0], pair[1]))
}

/// Where the line at `x` (a hair off the midline, which the mesh has edges
/// along), height `y`, running along `z`, crosses a triangle.
fn cross_line(tri: [Vec3; 3], y: f32, x: f32) -> Option<f32> {
    // Barycentric solve in the (x, y) plane.
    let [a, b, c] = tri;
    let p = Vec2::new(x, y);
    let (a2, b2, c2) = (
        Vec2::new(a.x, a.y),
        Vec2::new(b.x, b.y),
        Vec2::new(c.x, c.y),
    );
    let denominator = (b2 - a2).perp_dot(c2 - a2);
    if denominator.abs() < 1e-12 {
        return None;
    }
    let u = (p - a2).perp_dot(c2 - a2) / denominator;
    let v = (b2 - a2).perp_dot(p - a2) / denominator;
    if u < 0.0 || v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(a.z + (b.z - a.z) * u + (c.z - a.z) * v)
}

/// How many separate pieces a mesh is: its faces' connected components.
///
/// The cage's count is the built body's, because subdivision and the shaping
/// after it add and move vertices but join and part nothing. A stray piece of
/// the body would be a second one (#379), which none has turned out to be.
fn pieces(mesh: &PolyMesh) -> usize {
    let mut parent: Vec<usize> = (0..mesh.positions.len()).collect();
    fn root(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            parent[at] = parent[parent[at]];
            at = parent[at];
        }
        at
    }
    for face in &mesh.faces {
        let first = root(&mut parent, face[0] as usize);
        for &corner in &face[1..] {
            let other = root(&mut parent, corner as usize);
            parent[other] = first;
        }
    }
    let mut roots: Vec<usize> = mesh
        .faces
        .iter()
        .map(|face| root(&mut parent, face[0] as usize))
        .collect();
    roots.sort_unstable();
    roots.dedup();
    roots.len()
}

/// Prints the front girdle's neighbourhood: the debug view of a socket
/// failure.
#[allow(dead_code)]
pub fn girdle(record: &AvatarRecord) {
    let skeleton = record.skeleton();
    for index in [0usize, 1, 16, 17, 27, 28, 5, 21, 22, 23, 32, 33, 34] {
        let node = skeleton.nodes[index];
        println!(
            "node {index}: {:?} r {:.3} scale {:?} marker {}",
            node.position, node.radius, node.scale, node.marker
        );
    }
}
