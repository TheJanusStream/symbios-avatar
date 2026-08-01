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
//! cargo run --example render -- --head        # close up on face, neck and hair
//! cargo run --example render -- --close hand  # or head, hand, foot
//! cargo run --example render -- --hair 1,0,0,0.5,0.6,0.2   # override hair axes
//! ```

use glam::{Mat4, Vec2, Vec3};
use symbios_avatar::{
    Archetype, AvatarRecord, Blink, CageConfig, Extremities, Eyes, FootingConfig, Gait, Ground,
    Hair, HairParams, PolyMesh, Pose, Rig, SkinConfig, Stride, Surface, UvConfig, UvUnwrap, Zone,
    anim::gait, anim::plant_feet_of, build_cage, catmull_clark, rig::skin, texture, unwrap,
};

/// Pixels per side of one view in the sheet.
const VIEW: usize = 420;
/// How much of the body's height the frame covers, as a multiple of it.
const MARGIN: f32 = 1.12;
/// The same, for a close-up, over the focused part's largest side.
const CLOSE_MARGIN: f32 = 1.5;
/// How far the overhead view tilts over, in radians.
const OVERHEAD_PITCH: f32 = 1.0;

/// Which part of the body a close-up frames.
#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Head,
    Hand,
    Foot,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|at| args.get(at + 1))
    };
    let seed = value("--seed").and_then(|s| s.parse::<i64>().ok());
    let frames = value("--walk").and_then(|s| s.parse::<usize>().ok());
    // --head is the common case and stays as its own flag; --close names any of
    // them.
    let focus = match value("--close").map(String::as_str) {
        Some("head") => Some(Focus::Head),
        Some("hand") => Some(Focus::Hand),
        Some("foot") => Some(Focus::Foot),
        Some(other) => {
            eprintln!("unknown --close target {other}: expected head, hand or foot");
            std::process::exit(1);
        }
        None => args
            .iter()
            .any(|arg| arg == "--head")
            .then_some(Focus::Head),
    };
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

    // One shot, framed either on the whole body or on the head.
    let shoot = |pose: &Pose, closure: f32| -> Option<Image> {
        match focus {
            Some(focus) => subject.close_up(pose, closure, focus),
            None => Some(subject.sheet(pose, closure)),
        }
    };
    let stem = match focus {
        Some(Focus::Head) => "render_head",
        Some(Focus::Hand) => "render_hand",
        Some(Focus::Foot) => "render_foot",
        None => "render",
    };

    match frames {
        Some(frames) => {
            let mut blink = Blink::seeded(1);
            for frame in 0..frames.max(1) {
                let cycle = frame as f32 / frames.max(1) as f32;
                let closure = blink.advance(1.0 / frames.max(1) as f32);
                let Some(sheet) = shoot(&subject.walking(cycle), closure) else {
                    eprintln!("this body has no such part to frame");
                    std::process::exit(1);
                };
                write(&out, &format!("{stem}_walk_{frame:02}"), &sheet);
            }
            println!("rendered {} walk frames", frames.max(1));
        }
        None => {
            let (Some(sheet), Some(blinking)) = (
                shoot(&subject.standing(), 0.0),
                shoot(&subject.standing(), 1.0),
            ) else {
                eprintln!("this body has no such part to frame");
                std::process::exit(1);
            };
            write(&out, stem, &sheet);
            write(&out, &format!("{stem}_blink"), &blinking);
            match focus {
                Some(_) => println!("rendered {stem} close up, eyes open and shut"),
                None => println!("rendered a standing body, eyes open and shut"),
            }
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
    /// Which part of the body each vertex belongs to, for framing a close-up.
    zones: Vec<Zone>,
    eyes: Option<Eyes>,
    hair: Option<Hair>,
    extremities: Extremities,
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

        let surface = Surface::measure(&mesh, &rig);

        Some(Self {
            eyes: Eyes::build(&rig, &record.eyes),
            hair: Hair::build(&mesh, &rig, hair),
            // The plan stands its bodies on the origin.
            extremities: Extremities::build(&rig, &surface, 0.0),
            gait: Gait::natural(&rig),
            stride: Stride::for_body(&rig, 1.0),
            height: (hi.y - lo.y).max(0.1),
            albedo: painted.albedo,
            atlas,
            mesh,
            uv,
            weights,
            rig,
            zones,
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

    /// Four views of the whole body in one image.
    fn sheet(&self, pose: &Pose, closure: f32) -> Image {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let of = |turn: f32| Frame {
            turn,
            pitch: 0.0,
            centre: Vec3::new(0.0, self.height * 0.5, 0.0),
            span: self.height * MARGIN,
        };
        // Front, side, back, three-quarter.
        let frames = [of(0.0), of(FRAC_PI_2), of(PI), of(FRAC_PI_4)];
        self.render(pose, closure, &frames)
    }

    /// Four close-ups of one part of the body in one image.
    ///
    /// Different angles from the body sheet, because different things are being
    /// judged. A three-quarter view is where a face either reads or does not; a
    /// profile is where the brow line and chin live; and the view from above is
    /// the only one that shows a crown, a parting, or the back of a hand.
    fn close_up(&self, pose: &Pose, closure: f32, focus: Focus) -> Option<Image> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
        let posed = pose.forward(&self.rig);
        let deformed = posed.deform(&self.rig, &self.mesh.positions, &self.weights);

        // Framed on the part's own vertices. Anything hanging off it — long
        // hair, most obviously — is deliberately left to fall out of frame:
        // this is a close-up, and zooming out far enough to hold the whole
        // drape would put us back where we started.
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        let mut hold = |point: Vec3| {
            lo = lo.min(point);
            hi = hi.max(point);
        };

        match focus {
            Focus::Head => {
                for (vertex, zone) in self.zones.iter().enumerate() {
                    if matches!(zone, Zone::Head | Zone::Neck) {
                        hold(deformed[vertex]);
                    }
                }
            }
            Focus::Hand | Focus::Foot => {
                // One of them, not both: a pair framed together sits a body's
                // width apart and zooms straight back out to the body sheet.
                let parts = if matches!(focus, Focus::Hand) {
                    &self.extremities.hands
                } else {
                    &self.extremities.feet
                };
                let part = parts.first()?;
                let to_world = Mat4::from_rotation_translation(
                    posed.rotations[part.joint],
                    posed.positions[part.joint],
                );
                for &point in &part.mesh.positions {
                    hold(to_world.transform_point3(point));
                }
                // A little of the limb above it, for the join.
                let wrist = posed.positions[part.joint];
                hold(wrist + Vec3::splat(part.reach * 0.25));
                hold(wrist - Vec3::splat(part.reach * 0.25));
            }
        }
        if lo.x > hi.x {
            return None;
        }

        let of = |turn: f32, pitch: f32| Frame {
            turn,
            pitch,
            centre: (lo + hi) * 0.5,
            // Generous, so hair standing off the crown or a thumb held wide
            // both stay in frame.
            span: (hi.x - lo.x).max(hi.y - lo.y).max(hi.z - lo.z) * CLOSE_MARGIN,
        };
        let frames = [
            of(0.0, 0.0),
            of(FRAC_PI_4, 0.0),
            of(FRAC_PI_2, 0.0),
            of(0.0, OVERHEAD_PITCH),
        ];
        Some(self.render(pose, closure, &frames))
    }

    /// Draws one pose from four frames into a two-by-two sheet.
    fn render(&self, pose: &Pose, closure: f32, frames: &[Frame; 4]) -> Image {
        let posed = pose.forward(&self.rig);
        let deformed = posed.deform(&self.rig, &self.mesh.positions, &self.weights);

        let head_of = |head: usize| {
            Mat4::from_rotation_translation(posed.rotations[head], posed.positions[head])
        };

        // Eyes ride the head rigidly rather than being skinned. Globes and lids
        // are kept apart so they can be shaded differently — drawn in one colour
        // a shut eye is invisible, which is how a working blink first looked
        // broken.
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
            )
        });

        // Hair rides the head rigidly too, and is drawn one lock at a time.
        // Drawn as a single solid in a single colour it reads as a helmet at
        // close range, because nothing separates one group from the next.
        let hair = self.hair.as_ref().map(|hair| {
            let to_world = head_of(hair.head);
            let locks: Vec<PolyMesh> = hair
                .groups
                .iter()
                .map(|group| group.mesh().transformed(to_world))
                .collect();
            (locks, Vec3::from_array(hair.colour))
        });

        // Hands and feet ride their own joints, the way the eyes ride the head.
        let limbs: Vec<PolyMesh> = self
            .extremities
            .hands
            .iter()
            .chain(&self.extremities.feet)
            .map(|part| {
                let joint = part.joint;
                part.mesh.transformed(Mat4::from_rotation_translation(
                    posed.rotations[joint],
                    posed.positions[joint],
                ))
            })
            .collect();

        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        for (index, frame) in frames.iter().enumerate() {
            let mut view = Image::new(VIEW, VIEW);
            self.draw(&mut view, &deformed, frame);
            // Skin-coloured, and untextured: the atlas has no chart for a part
            // that is not in the body mesh.
            for part in &limbs {
                draw_solid(&mut view, part, frame, &|_| Vec3::new(0.86, 0.68, 0.60));
            }
            if let Some((globes, lids, centres)) = &parts {
                // A pale globe with a dark iris facing forward, so the eye reads
                // as an eye rather than a bead.
                draw_solid(&mut view, globes, frame, &|point| {
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
                // Lids are skin, so shutting them plainly covers the eye.
                draw_solid(&mut view, lids, frame, &|_| Vec3::new(0.78, 0.58, 0.50));
            }
            if let Some((locks, tone)) = &hair {
                for (lock, mesh) in locks.iter().enumerate() {
                    // A golden-ratio walk over brightness: neighbouring locks
                    // never land on the same tone, and no generator is needed to
                    // keep it reproducible.
                    let step = (lock as f32 * 0.618_034).fract();
                    let shade = *tone * (0.78 + 0.44 * step);
                    draw_solid(&mut view, mesh, frame, &|_| shade);
                }
            }
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        sheet
    }

    /// Draws the textured body into one view.
    fn draw(&self, image: &mut Image, deformed: &[Vec3], frame: &Frame) {
        let positions: Vec<Vec3> = self.uv.gather(deformed);
        let normals = normals_of(&positions, &self.uv.faces);
        let camera = frame.camera();

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
                    tri.map(|i| frame.eye() * normals[i as usize]),
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
fn draw_solid(image: &mut Image, mesh: &PolyMesh, frame: &Frame, colour: &dyn Fn(Vec3) -> Vec3) {
    let normals = normals_of(&mesh.positions, &mesh.faces);
    let camera = frame.camera();
    for face in &mesh.faces {
        for corner in 1..face.len().saturating_sub(1) {
            let tri = [face[0], face[corner], face[corner + 1]];
            raster(
                image,
                tri.map(|i| camera.transform_point3(mesh.positions[i as usize])),
                tri.map(|i| frame.eye() * normals[i as usize]),
                tri.map(|i| colour(mesh.positions[i as usize])),
            );
        }
    }
}

/// One orthographic view: where it looks from, at what, and how close.
///
/// Framing is a parameter rather than a constant because the body and the head
/// need wildly different ones. A skull is about a twelfth of a body's height, so
/// in a full-body frame it lands in a few dozen pixels — which is most of why
/// the eye bulge, the floating hair and the bald crown all survived so long.
#[derive(Clone, Copy)]
struct Frame {
    /// Rotation about the body's own axis; zero looks at the face.
    turn: f32,
    /// Tilt, in radians. Positive looks down from above.
    pitch: f32,
    /// The point the view is centred on, in world space.
    centre: Vec3,
    /// How much of the world the frame covers, in metres.
    span: f32,
}

impl Frame {
    /// World space to normalised device space: x and y in -1..1, z away.
    fn camera(&self) -> Mat4 {
        Mat4::from_scale(Vec3::new(
            2.0 / self.span,
            2.0 / self.span,
            -1.0 / self.span,
        )) * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_y(self.turn)
            * Mat4::from_translation(-self.centre)
    }

    /// Carries a world normal into this view's frame.
    ///
    /// The key light and the rim term are both written in the camera's space, so
    /// without this every view but the front is lit from behind — which left the
    /// back view a flat silhouette and hid whatever was wrong with it. It must
    /// be the same rotation `camera` applies: guessing the sign once got the
    /// back view right and the side wrong, which is what a rotation by pi being
    /// its own inverse will do to you.
    fn eye(&self) -> glam::Quat {
        glam::Quat::from_rotation_x(self.pitch) * glam::Quat::from_rotation_y(self.turn)
    }
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
