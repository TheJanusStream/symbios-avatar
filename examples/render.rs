//! Renders an avatar to a PNG contact sheet.
//!
//! Every other tool here reports numbers, and numbers cannot answer the question
//! that matters most: does it *look* right? A silhouette that reads as a person,
//! a face whose eyes sit where eyes sit, a walk that carries weight — none of
//! those are assertions. So this is a small orthographic rasteriser, enough to
//! put the body on screen from four angles and judge it.
//!
//! It is deliberately crude — one key light, one fill, a rim term, and the baked
//! albedo. Shading quality belongs in a real renderer; what this is for is
//! silhouette, proportion, placement, and deformation.
//!
//! ```text
//! cargo run --example render                  # the default body
//! cargo run --example render -- --seed 7      # a rerolled one
//! cargo run --example render -- --walk 8      # a walk cycle, one sheet per frame
//! ```

use glam::{Mat4, Vec2, Vec3};
use symbios_avatar::{
    Archetype, AvatarRecord, Blink, CageConfig, Eyes, FootingConfig, Gait, Ground, Hair,
    HairParams, PolyMesh, Pose, Rig, SkinConfig, Stride, UvConfig, UvUnwrap, anim::gait,
    anim::plant_feet_of, build_cage, catmull_clark, rig::skin, texture, unwrap,
};

/// Pixels per side of one view in the sheet.
const VIEW: usize = 420;
/// How much of the body's height the frame covers, as a multiple of it.
const MARGIN: f32 = 1.12;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
    };
    let seed = value("--seed").and_then(|s| s.parse::<i64>().ok());
    let frames = value("--walk").and_then(|s| s.parse::<usize>().ok());
    // Six numbers, in the order the axes are declared: length, volume,
    // coverage, part, wave, shade. For walking the parameter space by eye,
    // which is the only way any of it got tuned.
    let overridden: Vec<f32> = value("--hair")
        .map(|spec| {
            spec.split(',')
                .filter_map(|a| a.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let out = std::path::PathBuf::from(
        std::env::var("SYMBIOS_AVATAR_DUMP_DIR").unwrap_or_else(|_| "target/dump".into()),
    );
    if let Err(error) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {}: {error}", out.display());
        std::process::exit(1);
    }

    let mut record = AvatarRecord::new("Rendered", Archetype::default());
    if let Some(seed) = seed {
        record.reroll(seed);
    }

    // The record carries its own hair; the flag only replaces the axes it names.
    let axis = |at: usize, fallback: f32| overridden.get(at).copied().unwrap_or(fallback);
    let hair = HairParams {
        length: axis(0, record.hair.length),
        volume: axis(1, record.hair.volume),
        coverage: axis(2, record.hair.coverage),
        part: axis(3, record.hair.part),
        wave: axis(4, record.hair.wave),
        shade: axis(5, record.hair.shade),
        ..record.hair
    };

    let Some(subject) = Subject::build(&record, &hair) else {
        eprintln!("the body could not be built");
        std::process::exit(1);
    };

    match frames {
        Some(frames) => {
            let mut blink = Blink::seeded(1);
            for frame in 0..frames.max(1) {
                let cycle = frame as f32 / frames.max(1) as f32;
                let closure = blink.advance(1.0 / frames.max(1) as f32);
                let sheet = subject.sheet(&subject.walking(cycle), closure);
                write(&out, &format!("render_walk_{frame:02}"), &sheet);
            }
            println!("rendered {} walk frames", frames.max(1));
        }
        None => {
            let sheet = subject.sheet(&subject.standing(), 0.0);
            write(&out, "render", &sheet);
            let blinking = subject.sheet(&subject.standing(), 1.0);
            write(&out, "render_blink", &blinking);
            println!("rendered a standing body, eyes open and shut");
        }
    }
    println!("wrote PNGs to {}", out.display());
}

/// Everything needed to draw one avatar.
struct Subject {
    mesh: PolyMesh,
    uv: UvUnwrap,
    albedo: Vec<u8>,
    atlas: u32,
    weights: symbios_avatar::SkinWeights,
    rig: Rig,
    eyes: Option<Eyes>,
    hair: Option<Hair>,
    gait: Gait,
    stride: Stride,
    height: f32,
}

impl Subject {
    /// Builds a body from a record, all the way to something drawable.
    fn build(record: &AvatarRecord, hair: &HairParams) -> Option<Self> {
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).ok()?;
        let mesh = catmull_clark(&cage, 2);
        let rig = Rig::from_skeleton(&skeleton).ok()?;
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());

        let atlas = 512;
        let geometry = texture::bake_geometry(&mesh, &uv, atlas);
        let painted = texture::paint_skin(&geometry, &rig, &record.skin);
        let (lo, hi) = mesh.bounds();

        Some(Self {
            eyes: Eyes::build(&rig, &record.eyes),
            hair: Hair::build(&mesh, &rig, hair),
            gait: Gait::natural(&rig),
            stride: Stride::for_body(&rig, 1.0),
            height: (hi.y - lo.y).max(0.1),
            albedo: painted.albedo,
            atlas,
            mesh,
            uv,
            weights,
            rig,
        })
    }

    /// The body standing still.
    fn standing(&self) -> Pose {
        Pose::rest(&self.rig)
    }

    /// The body mid-walk, with its stance feet on a gentle slope.
    fn walking(&self, cycle: f32) -> Pose {
        let mut pose = Pose::rest(&self.rig);
        let steps = gait::step(&self.rig, &mut pose, &self.gait, &self.stride, cycle);
        plant_feet_of(
            &self.rig,
            &mut pose,
            &steps.stance,
            |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
            &FootingConfig::default(),
        );
        pose
    }

    /// Four views of the body in one image.
    fn sheet(&self, pose: &Pose, closure: f32) -> Image {
        let posed = pose.forward(&self.rig);
        let deformed = posed.deform(&self.rig, &self.mesh.positions, &self.weights);

        // Eyes ride the head rigidly rather than being skinned. Globes and lids
        // are kept apart so they can be shaded differently — drawn in one colour
        // a shut eye is invisible, which is how a working blink first looked
        // broken.
        let head_of = |head: usize| {
            Mat4::from_rotation_translation(posed.rotations[head], posed.positions[head])
        };

        let parts = self.eyes.as_ref().map(|eyes| {
            let to_world = head_of(eyes.head);

            let mut globes = eyes.left.globe.clone();
            globes.append(&eyes.right.globe);

            let mut lids = PolyMesh::new();
            for eye in [&eyes.left, &eyes.right] {
                lids.append(&eye.upper_lid.transformed(eye.lid_transform(closure, true)));
                lids.append(&eye.lower_lid.transformed(eye.lid_transform(closure, false)));
            }
            let centres = [
                to_world.transform_point3(eyes.left.pivot),
                to_world.transform_point3(eyes.right.pivot),
            ];
            (
                globes.transformed(to_world),
                lids.transformed(to_world),
                centres,
                eyes.left.radius,
            )
        });

        // Hair rides the head rigidly too. It is drawn as one solid in one
        // colour: strand groups overlap by design, so there is nothing to shade
        // apart the way the lids had to be.
        let hair = self
            .hair
            .as_ref()
            .map(|hair| (hair.mesh().transformed(head_of(hair.head)), hair.colour));

        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        // Front, side, back, three-quarter.
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        for (index, turn) in [0.0, FRAC_PI_2, PI, FRAC_PI_4].into_iter().enumerate() {
            let mut view = Image::new(VIEW, VIEW);
            self.draw(&mut view, &deformed, turn);
            if let Some((globes, lids, centres, radius)) = &parts {
                // A pale globe with a dark iris facing forward, so the eye reads
                // as an eye rather than a bead.
                draw_solid(&mut view, globes, turn, self.height, &|point| {
                    let nearest = centres
                        .iter()
                        .min_by(|a, b| point.distance(**a).total_cmp(&point.distance(**b)))
                        .copied()
                        .unwrap_or(Vec3::ZERO);
                    let forward = (point - nearest).normalize_or(Vec3::Z).z;
                    if forward > 0.72 {
                        Vec3::new(0.10, 0.13, 0.16)
                    } else if forward > 0.45 {
                        Vec3::new(0.28, 0.42, 0.52)
                    } else {
                        Vec3::new(0.95, 0.94, 0.93)
                    }
                });
                let _ = radius;
                // Lids are skin, so shutting them plainly covers the eye.
                draw_solid(&mut view, lids, turn, self.height, &|_| {
                    Vec3::new(0.78, 0.58, 0.50)
                });
            }
            if let Some((mesh, colour)) = &hair {
                let tone = Vec3::from_array(*colour);
                draw_solid(&mut view, mesh, turn, self.height, &|_| tone);
            }
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        sheet
    }

    /// Draws the textured body into one view.
    fn draw(&self, image: &mut Image, deformed: &[Vec3], turn: f32) {
        let positions: Vec<Vec3> = self.uv.gather(deformed);
        let normals = normals_of(&positions, &self.uv.faces);
        let camera = camera(turn, self.height);

        for face in &self.uv.faces {
            for corner in 1..face.len().saturating_sub(1) {
                let tri = [face[0], face[corner], face[corner + 1]];
                let shade = |index: u32| {
                    let uv = self.uv.uvs[index as usize];
                    self.sample(uv)
                };
                raster(
                    image,
                    tri.map(|i| camera.transform_point3(positions[i as usize])),
                    tri.map(|i| eye_turn(turn) * normals[i as usize]),
                    tri.map(shade),
                );
            }
        }
    }

    /// The albedo at one texture coordinate.
    fn sample(&self, uv: Vec2) -> Vec3 {
        let x = ((uv.x * self.atlas as f32) as u32).min(self.atlas - 1);
        let y = ((uv.y * self.atlas as f32) as u32).min(self.atlas - 1);
        let at = ((y * self.atlas + x) * 4) as usize;
        if at + 2 >= self.albedo.len() {
            return Vec3::splat(0.5);
        }
        Vec3::new(
            f32::from(self.albedo[at]),
            f32::from(self.albedo[at + 1]),
            f32::from(self.albedo[at + 2]),
        ) / 255.0
    }
}

/// Draws an untextured mesh, colouring each vertex by where it sits.
fn draw_solid(
    image: &mut Image,
    mesh: &PolyMesh,
    turn: f32,
    height: f32,
    colour: &dyn Fn(Vec3) -> Vec3,
) {
    let normals = normals_of(&mesh.positions, &mesh.faces);
    let camera = camera(turn, height);
    for face in &mesh.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let tri = [face[0], face[corner], face[corner + 1]];
            raster(
                image,
                tri.map(|i| camera.transform_point3(mesh.positions[i as usize])),
                tri.map(|i| eye_turn(turn) * normals[i as usize]),
                tri.map(|i| colour(mesh.positions[i as usize])),
            );
        }
    }
}

/// Carries a world normal into the turned camera's frame.
///
/// The key light and the rim term are both written in the camera's space, so
/// without this every view but the front is lit from behind — which left the
/// back view a flat silhouette and hid whatever was wrong with it.
fn eye_turn(turn: f32) -> glam::Quat {
    // The same rotation the camera applies to positions. Guessing its sign got
    // the back view right and the side view wrong, which is what a rotation by
    // pi being its own inverse will do to you.
    glam::Quat::from_rotation_y(turn)
}

/// An orthographic camera turned `turn` radians about the body.
fn camera(turn: f32, height: f32) -> Mat4 {
    let span = height * MARGIN;
    // Normalised device space: x and y in -1..1, z increasing away from the eye.
    Mat4::from_scale(Vec3::new(2.0 / span, 2.0 / span, -1.0 / span))
        * Mat4::from_translation(Vec3::new(0.0, -height * 0.5, 0.0))
        * Mat4::from_rotation_y(turn)
}

/// Smooth vertex normals for a mesh given as faces over positions.
fn normals_of(positions: &[Vec3], faces: &[Vec<u32>]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for face in faces {
        if face.len() < 3 {
            continue;
        }
        let anchor = positions[face[0] as usize];
        let weighted: Vec3 = (1..face.len() - 1)
            .map(|corner| {
                let a = positions[face[corner] as usize] - anchor;
                let b = positions[face[corner + 1] as usize] - anchor;
                a.cross(b)
            })
            .sum();
        for &index in face {
            normals[index as usize] += weighted;
        }
    }
    for normal in &mut normals {
        *normal = normal.normalize_or(Vec3::Y);
    }
    normals
}

/// Fills one triangle, depth-tested and shaded.
fn raster(image: &mut Image, clip: [Vec3; 3], normals: [Vec3; 3], colours: [Vec3; 3]) {
    let size = image.width as f32;
    let screen =
        clip.map(|point| Vec2::new((point.x * 0.5 + 0.5) * size, (0.5 - point.y * 0.5) * size));

    let area = edge(screen[0], screen[1], screen[2]);
    if area.abs() < 1e-6 {
        return;
    }

    let lo = screen[0]
        .min(screen[1])
        .min(screen[2])
        .floor()
        .max(Vec2::ZERO);
    let hi = screen[0]
        .max(screen[1])
        .max(screen[2])
        .ceil()
        .min(Vec2::splat(size));

    for y in (lo.y as usize)..(hi.y as usize).min(image.height) {
        for x in (lo.x as usize)..(hi.x as usize).min(image.width) {
            let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let mut bary = Vec3::new(
                edge(screen[1], screen[2], point),
                edge(screen[2], screen[0], point),
                edge(screen[0], screen[1], point),
            ) / area;
            if bary.min_element() < 0.0 {
                continue;
            }
            bary /= bary.element_sum();

            let depth = clip[0].z * bary.x + clip[1].z * bary.y + clip[2].z * bary.z;
            let at = y * image.width + x;
            if depth >= image.depth[at] {
                continue;
            }

            let normal = (normals[0] * bary.x + normals[1] * bary.y + normals[2] * bary.z)
                .normalize_or(Vec3::Y);
            let albedo = colours[0] * bary.x + colours[1] * bary.y + colours[2] * bary.z;

            image.depth[at] = depth;
            image.colour[at] = light(normal, albedo);
        }
    }
}

/// A key light, a cool fill, and a rim — enough to read a silhouette by.
fn light(normal: Vec3, albedo: Vec3) -> Vec3 {
    let key = Vec3::new(-0.4, 0.7, 0.6).normalize();
    let fill = Vec3::new(0.6, 0.1, 0.3).normalize();

    let direct = normal.dot(key).max(0.0);
    let bounce = normal.dot(fill).max(0.0) * 0.25;
    // The rim is what separates a body from its background without an outline.
    let rim = (1.0 - normal.z.abs()).powf(3.0) * 0.35;

    albedo * (0.25 + direct * 0.85) + Vec3::new(0.55, 0.62, 0.78) * bounce + Vec3::splat(rim)
}

/// Twice the signed area of a triangle.
fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// A colour buffer with a depth buffer beside it.
struct Image {
    width: usize,
    height: usize,
    colour: Vec<Vec3>,
    depth: Vec<f32>,
}

impl Image {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            colour: vec![Vec3::new(0.10, 0.11, 0.13); width * height],
            depth: vec![f32::INFINITY; width * height],
        }
    }

    /// Copies another image in at an offset.
    fn blit(&mut self, other: &Image, x: usize, y: usize) {
        for row in 0..other.height {
            for column in 0..other.width {
                let (tx, ty) = (x + column, y + row);
                if tx < self.width && ty < self.height {
                    self.colour[ty * self.width + tx] = other.colour[row * other.width + column];
                }
            }
        }
    }

    /// The buffer as sRGB bytes.
    fn bytes(&self) -> Vec<u8> {
        self.colour
            .iter()
            .flat_map(|colour| {
                let c = colour.clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
                [c.x as u8, c.y as u8, c.z as u8, 255]
            })
            .collect()
    }
}

/// Writes an image as a PNG.
fn write(dir: &std::path::Path, name: &str, image: &Image) {
    let path = dir.join(format!("{name}.png"));
    let saved = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes())
        .ok_or_else(|| "buffer is the wrong size".to_string())
        .and_then(|png| png.save(&path).map_err(|error| error.to_string()));
    if let Err(error) = saved {
        eprintln!("cannot write {}: {error}", path.display());
    }
}
