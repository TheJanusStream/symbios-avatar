//! Renders an avatar to a PNG contact sheet.
//!
//! Every other tool here reports numbers, and numbers cannot answer the question
//! that matters most: does it *look* right? A silhouette that reads as a person,
//! a face whose eyes sit where eyes sit, a walk that carries weight — none of
//! those are assertions.
//!
//! This is the instrument every quality judgement in the project is made with,
//! which is the argument for it being good rather than merely sufficient. A
//! crude renderer does not just under-sell the work; it actively misleads. It
//! has already done so twice — a blink that looked broken because lids and
//! globes were drawn in one colour, and UV seam normals that read as a skinning
//! defect and sent a day's work after the wrong culprit.
//!
//! So: a small deferred renderer. Geometry lands in a G-buffer, occlusion is
//! measured, and everything is lit together in linear light, supersampled.
//!
//! It draws whatever [`symbios_avatar::Avatar`] hands it and knows nothing about
//! how a body is made. That is deliberate: this file used to carry its own copy
//! of the recipe, and a second copy of the recipe is a second implementation of
//! the crate — one that had already drifted from the other. Now it is a
//! conformance test. If the library returns something that cannot be drawn, this
//! is where that shows.
//!
//! **Run it in release.** Debug is roughly thirty times slower and there is
//! nothing to debug in a rasteriser that is working.
//!
//! ```text
//! cargo run --release --example render                 # the default body
//! cargo run --release --example render -- --seed 7     # a rerolled one
//! cargo run --release --example render -- --walk 8     # a walk, one sheet per frame
//! cargo run --release --example render -- --head       # close up on face and hair
//! cargo run --release --example render -- --close hand # or head, hand, foot
//! cargo run --release --example render -- --linear     # matrix skinning, to compare
//! cargo run --release --example render -- --hair 1,0,0,0.5,0.6,0.2
//! cargo run --release --example render -- --pass ao   # or normal, albedo, shadow
//! cargo run --release --example render -- --quadruped
//! cargo run --release --example render -- --budget    # what one avatar costs
//! ```

mod light;
mod scene;

use glam::{Mat4, Vec3};
use light::Image;
use scene::{Frame, GBuffer, Item, Material, Paint, ShadowMap};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarMesh, AvatarRecord, Blink, FootingConfig, Gait, Ground,
    HairParams, MeshKind, PolyMesh, Pose, Stride, Zone, anim::gait, anim::plant_feet_of,
};

/// Pixels per side of one view in the finished sheet.
const VIEW: usize = 512;
/// How many samples per pixel side are rendered before resolving.
///
/// A stair-stepped silhouette reads as unfinished whatever is inside it, and
/// silhouette is the first thing anyone judges.
const SUPERSAMPLE: usize = 2;
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
    // Skinning is a choice with a visible cost either way, so it is a flag:
    // dual quaternions pinch less and can bulge, matrices are the opposite.
    let linear = args.iter().any(|arg| arg == "--linear");
    // Which stage to show instead of the finished picture.
    let pass = value("--pass").cloned();
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

    // Both archetypes, because everything on a body is shared between them and
    // only one of them has ever been looked at.
    let archetype = if args.iter().any(|arg| arg == "--quadruped") {
        Archetype::Quadruped(symbios_avatar::QuadrupedParams::default())
    } else {
        Archetype::default()
    };
    let mut record = AvatarRecord::new("Rendered", archetype);
    if let Some(seed) = seed {
        record.reroll(seed);
    }

    // The record carries its own hair; the flag only replaces the axes it names.
    let axis = |at: usize, fallback: f32| overridden.get(at).copied().unwrap_or(fallback);
    let config = AvatarConfig {
        hair: (!overridden.is_empty()).then(|| HairParams {
            length: axis(0, record.hair.length),
            volume: axis(1, record.hair.volume),
            coverage: axis(2, record.hair.coverage),
            part: axis(3, record.hair.part),
            wave: axis(4, record.hair.wave),
            shade: axis(5, record.hair.shade),
            ..record.hair
        }),
        ..Default::default()
    };

    let Some(avatar) = Avatar::build_with(&record, &config) else {
        eprintln!("the body could not be built");
        std::process::exit(1);
    };
    if args.iter().any(|arg| arg == "--budget") {
        report(&avatar);
    }
    let subject = Subject::new(avatar, linear, pass);

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

/// Prints what one avatar costs, against the targets it is judged by.
///
/// The numbers, not a verdict: the WebGL2 tier wants one to three skinned
/// meshes and fifteen to thirty thousand triangles, and knowing how far over
/// that a body is, is the only way to know which half of it to cut.
fn report(avatar: &Avatar) {
    let budget = avatar.budget;
    println!(
        "{:<8} {:>7} tris  {:>2} meshes  {:>3} joints  {:>5} KiB texture",
        "budget",
        budget.tris,
        budget.meshes,
        budget.joints,
        budget.texture_bytes / 1024,
    );
    for drawn in avatar.drawn(0.0) {
        println!(
            "{:<8} {:>7} tris  {:>6} verts",
            drawn.kind.name(),
            drawn.mesh.triangulated().len(),
            drawn.mesh.vertex_count(),
        );
    }
}

/// An avatar, and the gait it walks with.
struct Subject {
    avatar: Avatar,
    linear: bool,
    pass: Option<String>,
    gait: Gait,
    stride: Stride,
    /// The centre of the body's rest extent.
    middle: Vec3,
    /// Its largest side, so a long body is framed by its length.
    reach: f32,
}

impl Subject {
    /// Wraps a built avatar in what a contact sheet additionally needs.
    fn new(avatar: Avatar, linear: bool, pass: Option<String>) -> Self {
        let (lo, hi) = avatar.parts.body.bounds();
        Self {
            gait: Gait::natural(&avatar.rig),
            stride: Stride::for_body(&avatar.rig, 1.0),
            middle: (lo + hi) * 0.5,
            reach: (hi - lo).max_element().max(0.1),
            avatar,
            linear,
            pass,
        }
    }

    /// The body standing still.
    fn standing(&self) -> Pose {
        Pose::rest(&self.avatar.rig)
    }

    /// The body mid-walk, with its stance feet on the ground.
    fn walking(&self, cycle: f32) -> Pose {
        let rig = &self.avatar.rig;
        let mut pose = Pose::rest(rig);
        let steps = gait::step(rig, &mut pose, &self.gait, &self.stride, cycle);
        gait::swing_arms(rig, &mut pose, &self.gait, cycle);
        plant_feet_of(
            rig,
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
        // Framed on the body's whole extent, not on its height. A quadruped is
        // longer than it is tall, and a frame sized by height alone crops the
        // ends off it — which is not a small thing when the frame is the only
        // way anyone judges the body.
        let of = |turn: f32| Frame {
            turn,
            pitch: 0.0,
            centre: self.middle,
            span: self.reach * MARGIN,
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
    ///
    /// Framed on the *parts*, not on the merged meshes: a merged mesh cannot say
    /// which of its vertices are a head, which is exactly what a close-up needs
    /// to know and exactly why [`symbios_avatar::Parts`] is kept.
    fn close_up(&self, pose: &Pose, closure: f32, focus: Focus) -> Option<Image> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
        let parts = &self.avatar.parts;
        let posed = pose.forward(&self.avatar.rig);

        // Anything hanging off the part — long hair, most obviously — is
        // deliberately left to fall out of frame: this is a close-up, and
        // zooming out far enough to hold the whole drape would put us back where
        // we started.
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        let mut hold = |point: Vec3| {
            lo = lo.min(point);
            hi = hi.max(point);
        };

        match focus {
            Focus::Head => {
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    if matches!(zone, Zone::Head | Zone::Neck) {
                        hold(deformed[vertex]);
                    }
                }
            }
            Focus::Hand | Focus::Foot => {
                // One of them, not both: a pair framed together sits a body's
                // width apart and zooms straight back out to the body sheet.
                let attached = if matches!(focus, Focus::Hand) {
                    &parts.extremities.hands
                } else {
                    &parts.extremities.feet
                };
                let part = attached.first()?;
                let to_world = Mat4::from_rotation_translation(
                    posed.rotations[part.joint],
                    posed.positions[part.joint],
                );
                for &point in &part.mesh.positions {
                    hold(to_world.transform_point3(point));
                }
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
        let built = self.deformed(pose, closure);

        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        let side = VIEW * SUPERSAMPLE;
        for (index, frame) in frames.iter().enumerate() {
            let wall = backdrop(frame);
            let items = items(
                &built,
                &wall,
                self.avatar.skin.albedo.as_slice(),
                self.atlas(),
            );
            // The key light is written in the camera's frame, so its shadow has
            // to be cast anew for each view. Cheap enough at this size, and it
            // keeps every view internally consistent.
            let toward = frame.to_world(light::KEY);
            let shadow = ShadowMap::cast(&items, toward, frame.centre, frame.span, 1024);

            let mut buffer = GBuffer::new(side, side);
            buffer.draw(&items, frame);
            let ao = light::occlusion(&buffer, frame);
            let shaded = match &self.pass {
                Some(pass) => light::inspect(&buffer, &ao, frame, &shadow, pass),
                None => light::shade(&buffer, &ao, frame, &shadow),
            };
            let view = light::resolve(&shaded, SUPERSAMPLE);
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        sheet
    }

    /// Side of the skin atlas.
    fn atlas(&self) -> u32 {
        self.avatar.skin.width
    }

    /// Everything the avatar draws, deformed into the pose.
    ///
    /// The `--linear` flag is the one thing that cannot go through
    /// [`Avatar::posed`], because comparing the two skinning methods is the
    /// point of it.
    fn deformed(&self, pose: &Pose, closure: f32) -> Vec<AvatarMesh> {
        if !self.linear {
            return self.avatar.posed(pose, closure);
        }
        let posed = pose.forward(&self.avatar.rig);
        self.avatar
            .drawn(closure)
            .into_iter()
            .map(|drawn| {
                let mut mesh = drawn.mesh.clone();
                mesh.positions = posed.deform_linear(
                    &self.avatar.rig,
                    &drawn.mesh.positions,
                    &symbios_avatar::SkinWeights {
                        vertices: drawn.mesh.skin.clone(),
                    },
                );
                AvatarMesh {
                    kind: drawn.kind,
                    mesh,
                }
            })
            .collect()
    }
}

/// Everything to draw, as items, against the given backdrop.
///
/// One item per merged mesh, which is the whole argument for merging: what used
/// to be a draw per lock of hair and a draw per garment is now a draw per
/// material, and the colours that distinguished them ride on the vertices.
fn items<'a>(
    built: &'a [AvatarMesh],
    wall: &'a PolyMesh,
    albedo: &'a [u8],
    atlas: u32,
) -> Vec<Item<'a>> {
    let mut items = vec![Item {
        positions: &wall.positions,
        faces: &wall.faces,
        normals: None,
        paint: Paint::Flat,
        material: Material {
            albedo: Vec3::new(0.30, 0.31, 0.35),
            roughness: 1.0,
            specular: 0.02,
            wrap: 0.0,
        },
    }];

    for drawn in built {
        let material = match drawn.kind {
            MeshKind::Skin => Material::skin(Vec3::ONE),
            MeshKind::Hair => Material::hair(Vec3::ONE),
            MeshKind::Cloth => Material::cloth(Vec3::ONE),
            MeshKind::Eye => Material::glossy(Vec3::ONE),
        };
        // Skin is the only thing the atlas covers; everything else carries the
        // colour it wants on its vertices. See the note in `symbios_avatar::avatar`
        // about attached parts, which are mapped to one texel of it for now.
        let paint = match drawn.kind {
            MeshKind::Skin => Paint::Atlas {
                uvs: &drawn.mesh.uvs,
                pixels: albedo,
                side: atlas,
            },
            _ => Paint::Vertex(&drawn.mesh.colours),
        };
        items.push(Item {
            positions: &drawn.mesh.positions,
            faces: &drawn.mesh.faces,
            normals: Some(&drawn.mesh.normals),
            paint,
            material,
        });
    }

    items
}

/// A wall behind the subject, square to whichever way the camera is looking.
///
/// Not decoration either. A figure against a void has nothing to be lit against
/// and nothing to throw a shadow on, and a cast shadow is the strongest single
/// cue that a body occupies space. Camera-facing rather than world-fixed,
/// because the camera walks all the way round and a fixed wall would end up in
/// front of the subject for half the sheet.
fn backdrop(frame: &Frame) -> PolyMesh {
    let toward = frame.to_world(Vec3::Z);
    let right = frame.to_world(Vec3::X);
    let up = frame.to_world(Vec3::Y);
    // Close behind the subject. Far back, a key light this far off-axis throws
    // its shadow sideways by more than the wall is wide and the shadow simply
    // leaves the picture — which is what happened the moment the lighting was
    // made good enough to have form.
    let middle = frame.centre - toward * (frame.span * 0.32);
    let reach = frame.span * 2.2;

    let mut mesh = PolyMesh::new();
    for (across, rise) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        mesh.push_vertex(middle + right * (across * reach) + up * (rise * reach));
    }
    mesh.push_face([0, 1, 2, 3]);
    mesh
}

/// Saves a sheet.
fn write(dir: &std::path::Path, name: &str, image: &Image) {
    let path = dir.join(format!("{name}.png"));
    let saved = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes())
        .ok_or_else(|| "buffer is the wrong size".to_string())
        .and_then(|png| png.save(&path).map_err(|error| error.to_string()));
    if let Err(error) = saved {
        eprintln!("cannot write {}: {error}", path.display());
    }
}
