//! What the two CC0 mannequins say about head DIMORPHISM (#166).
//!
//! The sibling of `examples/reference`, which measures the same two files for
//! the body. Kept for the reason that one is kept: every reference column in
//! this crate has to be re-derivable rather than remembered (#173), and
//! [`crate::face::HeadTraits`]'s anchors are reference columns.
//!
//! Measured by RAY CASTING rather than by binning vertices: the male head is 200
//! vertices against the female's 632, so any band statistic reports the
//! tessellation as much as the shape. A ray from the head's own axis finds the
//! surface wherever it is.
//!
//! Everything is normalised by the head's own span, so overall head size — which
//! is `head_size` on the cage and not this issue's business — divides out.

use glam::Vec3;
use symbios_avatar::gltf::Gltf;

struct Head {
    /// Head-owned triangles, in head-local units: origin at the menton's
    /// height on the lateral midline, scaled by the head's span.
    faces: Vec<[Vec3; 3]>,
    span: f32,
    /// Fore-aft position of the head axis, in head spans.
    axis_z: f32,
}

impl Head {
    fn read(path: &str) -> Self {
        let bytes = std::fs::read(path).expect("the mannequin is beside this checkout");
        let file = Gltf::read(&bytes).expect("it parses");
        let mesh = file
            .rest_meshes()
            .expect("meshes")
            .into_iter()
            .next()
            .expect("one");
        let skin = file.skin(mesh.skin.expect("skinned")).expect("a skin");
        let bone: Vec<bool> = skin
            .names
            .iter()
            .map(|name| name == "head" || name == "head_leaf")
            .collect();
        let mine: Vec<bool> = (0..mesh.positions.len())
            .map(|vertex| mesh.held_by(vertex, |joint| bone[joint]) > 0.5)
            .collect();

        let owned: Vec<Vec3> = (0..mesh.positions.len())
            .filter(|&v| mine[v])
            .map(|v| mesh.positions[v])
            .collect();
        let low = owned.iter().fold(f32::MAX, |a, p| a.min(p.y));
        let high = owned.iter().fold(f32::MIN, |a, p| a.max(p.y));
        let span = high - low;
        let mid_x = 0.5
            * (owned.iter().fold(f32::MAX, |a, p| a.min(p.x))
                + owned.iter().fold(f32::MIN, |a, p| a.max(p.x)));
        let mid_z = 0.5
            * (owned.iter().fold(f32::MAX, |a, p| a.min(p.z))
                + owned.iter().fold(f32::MIN, |a, p| a.max(p.z)));

        let to_local = |p: Vec3| Vec3::new((p.x - mid_x) / span, (p.y - low) / span, p.z / span);
        let faces = mesh
            .triangles
            .iter()
            .filter(|t| t.iter().all(|&v| mine[v as usize]))
            .map(|t| t.map(|v| to_local(mesh.positions[v as usize])))
            .collect();
        Self {
            faces,
            span,
            axis_z: mid_z / span,
        }
    }

    /// Farthest surface hit by a ray from the head axis at `height`, along a
    /// unit direction in the horizontal plane. `None` if the ray misses.
    fn reach(&self, height: f32, dir: Vec3) -> Option<f32> {
        let from = Vec3::new(0.0, height, self.axis_z);
        let mut best: Option<f32> = None;
        for face in &self.faces {
            if let Some(t) = hit(from, dir, face)
                && t > 0.0
                && best.is_none_or(|b| t > b)
            {
                best = Some(t);
            }
        }
        best
    }

    /// Half-width at a height, taken as the larger of the two sides.
    fn across(&self, height: f32) -> Option<f32> {
        match (self.reach(height, Vec3::X), self.reach(height, -Vec3::X)) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (found, None) | (None, found) => found,
        }
    }

    fn fore(&self, height: f32) -> Option<f32> {
        self.reach(height, Vec3::Z)
    }

    fn aft(&self, height: f32) -> Option<f32> {
        self.reach(height, -Vec3::Z)
    }
}

/// Möller–Trumbore, returning the ray parameter.
fn hit(from: Vec3, dir: Vec3, face: &[Vec3; 3]) -> Option<f32> {
    let (e1, e2) = (face[1] - face[0], face[2] - face[0]);
    let h = dir.cross(e2);
    let a = e1.dot(h);
    if a.abs() < 1e-9 {
        return None;
    }
    let f = 1.0 / a;
    let s = from - face[0];
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(f * e2.dot(q))
}

/// The neck's SECTION at mid-neck, as fractions of stature.
///
/// **A mean radius about the neck's own axis, not a half-width across it**
/// (#166). The first version of this measured `max |x|` in a band and reported
/// the feminine mannequin's neck as 14% the thicker of the two, which is the
/// opposite of what neck circumference does in life. A neck is not round, and
/// one axis of an ellipse is not its size: the lateral reading alone cannot
/// tell a wider neck from a flatter one. Rays at every azimuth can, and they
/// answer the question `neck_r` actually asks, which is a radius.
///
/// Returns `(mean radius, lateral half-width, fore-aft half-depth)`, each over
/// stature.
fn neck(path: &str) -> (f32, f32, f32) {
    let bytes = std::fs::read(path).expect("the mannequin");
    let file = Gltf::read(&bytes).expect("it parses");
    let mesh = file
        .rest_meshes()
        .expect("meshes")
        .into_iter()
        .next()
        .expect("one");
    let skin = file.skin(mesh.skin.expect("skinned")).expect("a skin");
    let bone: Vec<bool> = skin.names.iter().map(|n| n == "neck_01").collect();
    let mine: Vec<bool> = (0..mesh.positions.len())
        .map(|v| mesh.held_by(v, |j| bone[j]) > 0.5)
        .collect();
    let owned: Vec<Vec3> = (0..mesh.positions.len())
        .filter(|&v| mine[v])
        .map(|v| mesh.positions[v])
        .collect();

    let (low, high) = owned
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(p.y), h.max(p.y)));
    let bounds = |f: fn(&Vec3) -> f32| {
        owned
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), p| (l.min(f(p)), h.max(f(p))))
    };
    let (x0, x1) = bounds(|p| p.x);
    let (z0, z1) = bounds(|p| p.z);
    let axis = Vec3::new(0.5 * (x0 + x1), 0.5 * (low + high), 0.5 * (z0 + z1));

    // The whole mesh's triangles, not the neck's own: a section cut at mid-neck
    // has to hit whatever surface is there, and a triangle straddling the
    // weight boundary belongs to the neck for half its area. Restricting the
    // set to fully-neck-owned triangles opens gaps for a ray to escape through.
    let faces: Vec<[Vec3; 3]> = mesh
        .triangles
        .iter()
        .filter(|t| t.iter().any(|&v| mine[v as usize]))
        .map(|t| t.map(|v| mesh.positions[v as usize]))
        .collect();

    let (whole_low, whole_high) = mesh.bounds();
    let stature = whole_high.y - whole_low.y;
    let spokes = 72;
    let mut sum = 0.0f32;
    let mut seen = 0;
    let (mut across, mut through) = (0.0f32, 0.0f32);
    for spoke in 0..spokes {
        let angle = std::f32::consts::TAU * spoke as f32 / spokes as f32;
        let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
        // **The NEAREST crossing, where the head above takes the farthest.**
        // A head is measured against a closed-ish patch and the outer surface
        // is the last thing a ray meets; a neck's triangle set includes every
        // triangle that straddles the weight boundary, so it reaches down into
        // the shoulders, and a ray taking the farthest hit sails out of the
        // neck and lands on a trapezius. Taking the farthest read the male's
        // silhouette 32% wider than a vertex count of the same band did, which
        // is what exposed it.
        let mut best = f32::MAX;
        for face in &faces {
            if let Some(t) = hit(axis, dir, face)
                && t > 1e-4
                && t < best
            {
                best = t;
            }
        }
        if best < f32::MAX {
            sum += best;
            seen += 1;
            across = across.max(best * dir.x.abs());
            through = through.max(best * dir.z.abs());
        }
    }
    let mean = if seen > 0 { sum / seen as f32 } else { 0.0 };
    (mean / stature, across / stature, through / stature)
}

fn main() {
    for (label, path) in [
        (
            "male",
            "../mesh2motion-app/static/models-variation/human-male.glb",
        ),
        (
            "female",
            "../mesh2motion-app/static/models-variation/human-female.glb",
        ),
    ] {
        let (mean, across, through) = neck(path);
        println!(
            "{label:>7}: neck mean radius {mean:.5} of stature, {across:.5} across, \
             {through:.5} through  (flatness {:.3})",
            through / across
        );
    }

    let male = Head::read("../mesh2motion-app/static/models-variation/human-male.glb");
    let female = Head::read("../mesh2motion-app/static/models-variation/human-female.glb");

    // Peak breadth and peak forward reach are each mesh's OWN, so what follows
    // is the shape of a head and not its size or its aspect ratio. Normalising
    // horizontals by the vertical span instead reports the length-to-breadth
    // difference — one number — uniformly at every height, and buries the shape
    // under it.
    let peak = |head: &Head, f: fn(&Head, f32) -> Option<f32>| {
        (0..=200)
            .filter_map(|s| {
                let h = s as f32 / 200.0;
                f(head, h).map(|v| (v, h))
            })
            .fold(
                (0.0f32, 0.0f32),
                |best, (v, h)| {
                    if v > best.0 { (v, h) } else { best }
                },
            )
    };
    for (label, head) in [("male", &male), ("female", &female)] {
        let (wide, at_wide) = peak(head, Head::across);
        let (fore, at_fore) = peak(head, Head::fore);
        println!(
            "{label:>7}: span {:.4} m   widest {:.4} at {at_wide:.2} of span   \
             furthest forward {:.4} at {at_fore:.2}   length:breadth {:.3}",
            head.span,
            wide,
            fore,
            1.0 / (2.0 * wide)
        );
    }

    let (mw, _) = peak(&male, Head::across);
    let (fw, _) = peak(&female, Head::across);
    let (md, _) = peak(&male, Head::fore);
    let (fd, _) = peak(&female, Head::fore);
    println!("\n  each column is that head's own peak = 1.000, so this is shape alone");
    let (ma, _) = peak(&male, Head::aft);
    let (fa, _) = peak(&female, Head::aft);
    println!(
        "  height   ---- half-width ----      ---- forward reach ----     ---- aft reach ----"
    );
    println!(
        "           male  female  female/male   male  female  female/male   male  female  f/m"
    );
    for step in 0..=20 {
        let height = step as f32 / 20.0;
        let row = |a: Option<f32>, pa: f32, b: Option<f32>, pb: f32| match (a, b) {
            (Some(a), Some(b)) => {
                format!(
                    "{:.3} {:.3}  {:+6.1}%",
                    a / pa,
                    b / pb,
                    100.0 * ((b / pb) / (a / pa) - 1.0)
                )
            }
            _ => "  -     -        -   ".to_string(),
        };
        println!(
            "  {height:.2}     {}    {}    {}",
            row(male.across(height), mw, female.across(height), fw),
            row(male.fore(height), md, female.fore(height), fd),
            row(male.aft(height), ma, female.aft(height), fa)
        );
    }
}
