//! Measures the CC0 reference mannequins, so every reference column in this
//! crate can be re-derived instead of remembered.
//!
//! Every figure `examples/bodyaudit` compares the built body against — landmark
//! heights, segment lengths, spans, the trunk silhouette, the limb-thickness
//! ladder — was measured once by hand off these two GLBs and then written into
//! that file as a constant. Nothing could reproduce them, so nothing could
//! notice if one were wrong, and the female half of several tables was never
//! taken at all (#173).
//!
//! This reads both mannequins and prints the tables. The male column is the
//! proof: it has to come out as the numbers already in `bodyaudit`, because
//! those are what it is measuring a second way.
//!
//! **The reference's own bone names are used here and only here.** The crate's
//! standing rule is that rigs must not be matched by name — its bone called
//! `head` is the jaw band — and authoring a correspondence table off the file
//! is exactly the exception [`Skin::names`] documents.
//!
//! ```text
//! cargo run --release --example reference
//! ```
//!
//! [`Skin::names`]: symbios_avatar::gltf::Skin::names

use glam::Vec3;
use symbios_avatar::gltf::{Gltf, RestMesh, Skin};

/// The two mannequins, CC0, in the mesh2motion checkout beside this one.
const BODIES: [(&str, &str); 2] = [
    (
        "male",
        "../mesh2motion-app/static/models-variation/human-male.glb",
    ),
    (
        "female",
        "../mesh2motion-app/static/models-variation/human-female.glb",
    ),
];

/// The landmarks, as `(what this crate calls it, the reference's bone)`.
///
/// **`head` and `neck` are not like for like and are printed anyway.** The
/// reference's `head` sits at the base of the skull and its `neck_01` at the
/// top of the spine, where this plan's are the centres of a head node and a
/// neck node. The trend is worth seeing; the offset is not an error figure.
const LANDMARKS: [(&str, &str); 6] = [
    ("head ~", "head"),
    ("neck ~", "neck_01"),
    ("chest ~", "spine_03"),
    ("waist ~", "spine_02"),
    ("pelvis", "pelvis"),
    ("knee", "calf_l"),
];

/// The segments, as `(name, from, to)`.
const SEGMENTS: [(&str, &str, &str); 4] = [
    ("upper arm", "upperarm_l", "lowerarm_l"),
    ("forearm", "lowerarm_l", "hand_l"),
    ("thigh", "thigh_l", "calf_l"),
    ("shank", "calf_l", "foot_l"),
];

/// Where along each bone the limb thickness is sampled.
const STATIONS: [f32; 4] = [0.125, 0.375, 0.625, 0.875];

/// The trunk silhouette's lowest band, how tall each band is, and how many.
///
/// **The whole body, not the nine bands `bodyaudit` compares against.** Running
/// it to the crown is what showed that those nine stop below the reference's
/// own widest trunk band: the male keeps widening past 0.72 to a peak of 0.0965
/// at 0.75-0.78, which is where its shoulder mass actually is. A table that
/// ends at 0.72 reads that climb as a body narrowing.
const BAND: (f32, f32, usize) = (0.03, 0.03, 32);

/// One measured body: the file's skin, its mesh, and where every joint sits.
struct Body {
    skin: Skin,
    mesh: RestMesh,
    /// Each joint's world position, indexed as [`Skin::nodes`] is.
    joints: Vec<Vec3>,
    /// Rendered height, in metres.
    height: f32,
    /// The lowest point of the mesh, which every height is measured from.
    floor: f32,
}

impl Body {
    /// Reads one GLB.
    fn read(path: &str) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let file = Gltf::read(&bytes).ok()?;
        let mesh = file.rest_meshes().ok()?.into_iter().next()?;
        let skin = file.skin(mesh.skin?).ok()?;
        let world = file.rest().ok()?;
        let joints = skin
            .nodes
            .iter()
            .map(|&node| world[node].w_axis.truncate())
            .collect();
        let (low, high) = mesh.bounds();
        Some(Self {
            skin,
            mesh,
            joints,
            height: (high.y - low.y).max(1e-3),
            floor: low.y,
        })
    }

    /// Where a named bone sits, or the origin if the file has no such bone.
    fn at(&self, bone: &str) -> Vec3 {
        self.skin
            .names
            .iter()
            .position(|name| name == bone)
            .map_or(Vec3::ZERO, |joint| self.joints[joint])
    }

    /// A named bone's height as a fraction of stature.
    fn up(&self, bone: &str) -> f32 {
        (self.at(bone).y - self.floor) / self.height
    }

    /// Whether each joint is part of an arm, by walking down from the shoulder.
    ///
    /// The arms come off at `upperarm`, not at `clavicle`: the clavicle carries
    /// the trapezius shelf, which is trunk, and dropping it takes the shoulders
    /// off the body along with the arms.
    fn arms(&self) -> Vec<bool> {
        let mut arm = vec![false; self.skin.len()];
        for (joint, name) in self.skin.names.iter().enumerate() {
            if name == "upperarm_l" || name == "upperarm_r" {
                arm[joint] = true;
            }
        }
        // Parents come before children in this file, but the specification does
        // not promise it, so this repeats until nothing new is claimed.
        loop {
            let mut grew = false;
            for (joint, parent) in self.skin.parents.iter().enumerate() {
                if let Some(parent) = parent
                    && arm[*parent]
                    && !arm[joint]
                {
                    arm[joint] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        arm
    }
}

fn main() {
    let bodies: Vec<(&str, Body)> = BODIES
        .iter()
        .filter_map(|(who, path)| Some((*who, Body::read(path)?)))
        .collect();
    if bodies.len() < BODIES.len() {
        println!(
            "The CC0 mannequins are not beside this checkout, so this cannot run. They live in\n\
             the mesh2motion repo at static/models-variation; clone it as a sibling of this one."
        );
        return;
    }

    for (who, body) in &bodies {
        println!(
            "{who:>7}: renders {:.3} m, {} vertices on {} joints",
            body.height,
            body.mesh.positions.len(),
            body.skin.len()
        );
    }

    heights(&bodies);
    segments(&bodies);
    spans(&bodies);
    silhouette(&bodies);
    thickness(&bodies);
}

/// Prints a row of one figure per body.
fn row(name: &str, values: impl Iterator<Item = f32>) {
    print!("{name:<14}");
    for value in values {
        print!(" {value:>9.4}");
    }
    println!();
}

/// Landmark heights, as fractions of each body's own stature.
fn heights(bodies: &[(&str, Body)]) {
    println!("\nlandmark heights, fractions of stature");
    row("landmark", std::iter::empty());
    for (name, bone) in LANDMARKS {
        row(name, bodies.iter().map(|(_, body)| body.up(bone)));
    }
}

/// Segment lengths, as fractions of stature.
fn segments(bodies: &[(&str, Body)]) {
    println!("\nsegment lengths, fractions of stature");
    for (name, from, to) in SEGMENTS {
        row(
            name,
            bodies
                .iter()
                .map(|(_, body)| body.at(from).distance(body.at(to)) / body.height),
        );
    }
}

/// Joint-to-joint spans, as fractions of stature.
///
/// Measured at the root of each limb chain — where the arm and the leg leave
/// the body — because that is the only shoulder figure two rigs can agree on.
/// Pairing bones called `clavicle` put our 0.238 beside their 0.190 and made a
/// 78% error look like 25%.
fn spans(bodies: &[(&str, Body)]) {
    println!("\nspans, fractions of stature");
    for (name, left, right) in [
        ("shoulder", "upperarm_l", "upperarm_r"),
        ("hip", "thigh_l", "thigh_r"),
    ] {
        row(
            name,
            bodies
                .iter()
                .map(|(_, body)| (body.at(left).x - body.at(right).x).abs() / body.height),
        );
    }
}

/// The trunk's silhouette, band by band, with the arms taken off.
///
/// Every vertex the arms do not hold, bucketed by height, reporting the
/// half-width and half-depth of what is left. This is the one comparison that
/// survives two rigs disagreeing about what a bone is called.
fn silhouette(bodies: &[(&str, Body)]) {
    println!("\ntrunk silhouette with the arms removed, half-width and half-depth of stature");
    print!("{:<14}", "band");
    for (who, _) in bodies {
        print!(" {who:>9} w {who:>7} d     n");
    }
    println!();
    println!(
        "{:<14} the vertex count is the honesty column: the male mannequin is 7399 vertices, so\n\
         {:<14} a band holding a dozen of them is one ring of a coarse mesh and not a figure to\n\
         {:<14} quote on its own.",
        "", "", ""
    );

    let measured: Vec<(Vec<bool>, &Body)> =
        bodies.iter().map(|(_, body)| (body.arms(), body)).collect();
    for band in 0..BAND.2 {
        let low = BAND.0 + band as f32 * BAND.1;
        print!("{:>7.2}-{:.2} ", low, low + BAND.1);
        for (arm, body) in &measured {
            let mut span = (f32::MAX, f32::MIN);
            let mut deep = (f32::MAX, f32::MIN);
            let mut count = 0usize;
            for (vertex, &at) in body.mesh.positions.iter().enumerate() {
                let up = (at.y - body.floor) / body.height;
                if up < low || up >= low + BAND.1 {
                    continue;
                }
                if body.mesh.held_by(vertex, |joint| arm[joint]) > 0.25 {
                    continue;
                }
                span = (span.0.min(at.x), span.1.max(at.x));
                deep = (deep.0.min(at.z), deep.1.max(at.z));
                count += 1;
            }
            if count < 8 {
                print!(" {:>9} {:>8} {count:>5}", "--", "--");
                continue;
            }
            print!(
                " {:>9.4} {:>8.4} {count:>5}",
                (span.1 - span.0) * 0.5 / body.height,
                (deep.1 - deep.0) * 0.5 / body.height
            );
        }
        println!();
    }
}

/// Limb thickness at four stations along each bone.
///
/// The mean perpendicular distance to the bone axis, over the vertices that
/// bone's own joint holds — which is exactly how `bodyaudit` measures ours, so
/// the two columns are in the same units.
fn thickness(bodies: &[(&str, Body)]) {
    println!("\nlimb thickness, mean radius at each station, fractions of stature");
    for (name, from, to) in SEGMENTS {
        print!("{name:<14}");
        for (_, body) in bodies {
            let held = body
                .skin
                .names
                .iter()
                .position(|bone| bone == from)
                .unwrap_or(usize::MAX);
            let (start, end) = (body.at(from), body.at(to));
            let axis = end - start;
            let span = axis.length().max(1e-6);
            let along = axis / span;
            for station in STATIONS {
                let mut total = 0.0;
                let mut count = 0usize;
                for (vertex, &at) in body.mesh.positions.iter().enumerate() {
                    if body.mesh.held_by(vertex, |joint| joint == held) <= 0.4 {
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
                    print!(" {:>7.4}", total / count as f32 / body.height);
                } else {
                    print!(" {:>7}", "--");
                }
            }
            print!("  |");
        }
        println!();
    }
    println!(
        "{:<14} {:>7} {:>7} {:>7} {:>7}",
        "", "0.125", "0.375", "0.625", "0.875"
    );
}
