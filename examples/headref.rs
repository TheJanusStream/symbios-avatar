//! What the two CC0 mannequins say about head DIMORPHISM (#166).
//!
//! The sibling of `examples/reference`, which measures the same two files for
//! the body. Kept for the reason that one is kept: every reference column in
//! this crate has to be re-derivable rather than remembered (#173), and
//! [`crate::face::Dimorphism`]'s anchors are reference columns.
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

/// The neck's half-width at mid-neck, as a fraction of stature.
///
/// The one body quantity #166 asks for that `examples/reference` does not
/// already print. Taken as a radius rather than a circumference because that is
/// what `neck_r` is, and at the middle of the bone so neither the jaw above nor
/// the trapezius below is in the reading.
fn neck(path: &str) -> (f32, f32) {
    let bytes = std::fs::read(path).expect("the mannequin");
    let file = Gltf::read(&bytes).expect("it parses");
    let mesh = file
        .rest_meshes()
        .expect("meshes")
        .into_iter()
        .next()
        .expect("one");
    let skin = file.skin(mesh.skin.expect("skinned")).expect("a skin");
    let neck: Vec<bool> = skin.names.iter().map(|n| n == "neck_01").collect();
    let mine: Vec<usize> = (0..mesh.positions.len())
        .filter(|&v| mesh.held_by(v, |j| neck[j]) > 0.5)
        .collect();
    let (low, high) = mine.iter().fold((f32::MAX, f32::MIN), |(l, h), &v| {
        (l.min(mesh.positions[v].y), h.max(mesh.positions[v].y))
    });
    let (whole_low, whole_high) = mesh.bounds();
    let stature = whole_high.y - whole_low.y;
    let mid = 0.5 * (low + high);
    let band = 0.15 * (high - low);
    let mid_x = 0.5
        * mine
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), &v| {
                (l.min(mesh.positions[v].x), h.max(mesh.positions[v].x))
            })
            .0
        + 0.5
            * mine
                .iter()
                .fold((f32::MAX, f32::MIN), |(l, h), &v| {
                    (l.min(mesh.positions[v].x), h.max(mesh.positions[v].x))
                })
                .1;
    let mut wide = 0.0f32;
    for &v in &mine {
        let at = mesh.positions[v];
        if (at.y - mid).abs() <= band {
            wide = wide.max((at.x - mid_x).abs());
        }
    }
    // The two selections have to cover the same fraction of stature or the
    // reading below is comparing two different pieces of anatomy. They do —
    // 0.789..0.868 against 0.793..0.877 — which is what makes the surprise in
    // the result a fact about the mannequins rather than about this probe.
    (wide / stature, stature)
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
        let (r, stature) = neck(path);
        println!("{label:>7}: neck half-width {r:.5} of a {stature:.4} m stature");
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
    println!("  height   ---- half-width ----      ---- forward reach ----");
    println!("           male  female  female/male   male  female  female/male");
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
            "  {height:.2}     {}    {}",
            row(male.across(height), mw, female.across(height), fw),
            row(male.fore(height), md, female.fore(height), fd)
        );
    }
}
