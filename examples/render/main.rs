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
//! ```

mod light;
mod scene;

use glam::{Mat4, Vec2, Vec3};
use light::Image;
use scene::{Frame, GBuffer, Item, Material, Paint, ShadowMap};
use symbios_avatar::{
    Archetype, AvatarRecord, Blink, CageConfig, Extremities, Eyes, Features, FootingConfig, Gait,
    Ground, Hair, HairParams, Outfit, PolyMesh, Pose, Posed, Rig, SkinConfig, Stride, Surface,
    UvConfig, UvUnwrap, Zone, anim::gait, anim::plant_feet_of, build_body, rig::skin, texture,
    unwrap,
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

    let Some(subject) = Subject::build(&record, &hair, linear, pass) else {
        eprintln!("the body could not be built");
        std::process::exit(1);
    };

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
    linear: bool,
    pass: Option<String>,
    mesh: PolyMesh,
    uv: UvUnwrap,
    albedo: Vec<u8>,
    atlas: u32,
    weights: symbios_avatar::SkinWeights,
    rig: Rig,
    /// Which part of the body each vertex belongs to, for framing a close-up.
    zones: Vec<Zone>,
    eyes: Option<Eyes>,
    features: Option<Features>,
    hair: Option<Hair>,
    extremities: Extremities,
    outfit: Outfit,
    gait: Gait,
    stride: Stride,
    height: f32,
}

impl Subject {
    /// Builds a body from a record, all the way to something drawable.
    fn build(
        record: &AvatarRecord,
        hair: &HairParams,
        linear: bool,
        pass: Option<String>,
    ) -> Option<Self> {
        let skeleton = record.skeleton();
        let mesh = build_body(&skeleton, &CageConfig::default(), 2).ok()?;
        let rig = Rig::from_skeleton(&skeleton).ok()?;
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        let uv = unwrap(&mesh, &rig, &zones, &UvConfig::default());

        let atlas = 1024;
        let geometry = texture::bake_geometry(&mesh, &uv, atlas);
        let painted = texture::paint_skin(&geometry, &rig, &record.skin);
        let (lo, hi) = mesh.bounds();
        let surface = Surface::measure(&mesh, &rig);

        Some(Self {
            linear,
            pass,
            eyes: Eyes::build(&rig, &record.eyes),
            features: Eyes::build(&rig, &record.eyes)
                .map(|eyes| Features::build(&eyes, &record.face)),
            hair: Hair::build(&mesh, &rig, hair),
            // The plan stands its bodies on the origin.
            extremities: Extremities::build(&rig, &surface, 0.0),
            outfit: Outfit::wear(&mesh, &weights, &zones, &record.outfit),
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

    /// The body mid-walk, with its stance feet on the ground.
    fn walking(&self, cycle: f32) -> Pose {
        let mut pose = Pose::rest(&self.rig);
        let steps = gait::step(&self.rig, &mut pose, &self.gait, &self.stride, cycle);
        gait::swing_arms(&self.rig, &mut pose, &self.gait, cycle);
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
        let deformed = self.skinned(&posed);

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

    /// Deforms the body by whichever skinning method was asked for.
    fn skinned(&self, posed: &Posed) -> Vec<Vec3> {
        if self.linear {
            posed.deform_linear(&self.rig, &self.mesh.positions, &self.weights)
        } else {
            posed.deform(&self.rig, &self.mesh.positions, &self.weights)
        }
    }

    /// Draws one pose from four frames into a two-by-two sheet.
    fn render(&self, pose: &Pose, closure: f32, frames: &[Frame; 4]) -> Image {
        let posed = pose.forward(&self.rig);
        let built = self.assemble(&posed, closure);

        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        let side = VIEW * SUPERSAMPLE;
        for (index, frame) in frames.iter().enumerate() {
            let wall = backdrop(frame);
            let items = built.items(&wall);
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

    /// Everything to be drawn for one pose, in world space.
    fn assemble(&self, posed: &Posed, closure: f32) -> Built<'_> {
        let deformed = self.skinned(posed);
        // Normals over the body's own topology, then gathered through the
        // unwrap. Deriving them from the unwrapped copy splits every seam.
        let normals = self
            .uv
            .gather(&scene::smooth_normals(&deformed, &self.mesh.faces));
        let positions = self.uv.gather(&deformed);

        let rigid = |joint: usize| {
            Mat4::from_rotation_translation(posed.rotations[joint], posed.positions[joint])
        };

        // Clothing carries the skin weights of the vertices it was cut from.
        let worn: Vec<(PolyMesh, Vec3)> = self
            .outfit
            .garments
            .iter()
            .map(|garment| {
                let moved = if self.linear {
                    posed.deform_linear(&self.rig, &garment.mesh.positions, &garment.weights)
                } else {
                    posed.deform(&self.rig, &garment.mesh.positions, &garment.weights)
                };
                (
                    PolyMesh {
                        positions: moved,
                        faces: garment.mesh.faces.clone(),
                    },
                    Vec3::from_array(garment.colour),
                )
            })
            .collect();

        let limbs: Vec<PolyMesh> = self
            .extremities
            .hands
            .iter()
            .chain(&self.extremities.feet)
            .map(|part| part.mesh.transformed(rigid(part.joint)))
            .collect();

        // Hair is drawn one lock at a time, with a walk over brightness: as a
        // single solid in a single colour it reads as a helmet at close range.
        let locks: Vec<(PolyMesh, Vec3)> = self
            .hair
            .as_ref()
            .map(|hair| {
                let to_world = rigid(hair.head);
                let tone = Vec3::from_array(hair.colour);
                hair.groups
                    .iter()
                    .enumerate()
                    .map(|(at, group)| {
                        let step = (at as f32 * 0.618_034).fract();
                        (
                            group.mesh().transformed(to_world),
                            tone * (0.74 + 0.5 * step),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Globes and lids are kept apart so they can be shaded differently:
        // drawn in one colour a shut eye is invisible, which is how a working
        // blink first looked broken.
        let eyes = self.eyes.as_ref().map(|eyes| {
            let to_world = rigid(eyes.head);
            let mut globes = eyes.left.globe.clone();
            globes.append(&eyes.right.globe);
            let mut lids = PolyMesh::new();
            for eye in [&eyes.left, &eyes.right] {
                lids.append(&eye.upper_lid.transformed(eye.lid_transform(closure, true)));
                lids.append(&eye.lower_lid.transformed(eye.lid_transform(closure, false)));
            }
            (
                globes.transformed(to_world),
                lids.transformed(to_world),
                [
                    to_world.transform_point3(eyes.left.pivot),
                    to_world.transform_point3(eyes.right.pivot),
                ],
            )
        });

        let face = self
            .features
            .as_ref()
            .map(|features| features.assembled().transformed(rigid(features.head)));

        Built {
            bare: bare_skin(&self.albedo),
            face,
            positions,
            normals,
            faces: self.uv.faces.clone(),
            uvs: self.uv.uvs.clone(),
            albedo: &self.albedo,
            atlas: self.atlas,
            worn,
            limbs,
            locks,
            iris: match &eyes {
                Some((_, _, centres)) => {
                    let centres = *centres;
                    Box::new(move |point| iris(point, &centres))
                }
                None => Box::new(|_| Vec3::ONE),
            },
            eyes,
        }
    }
}

/// One pose's worth of world-space geometry.
struct Built<'a> {
    bare: Vec3,
    face: Option<PolyMesh>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    faces: Vec<Vec<u32>>,
    uvs: Vec<Vec2>,
    albedo: &'a [u8],
    atlas: u32,
    worn: Vec<(PolyMesh, Vec3)>,
    limbs: Vec<PolyMesh>,
    locks: Vec<(PolyMesh, Vec3)>,
    eyes: Option<(PolyMesh, PolyMesh, [Vec3; 2])>,
    /// Held here rather than made inline: an item borrows it, so it has to
    /// outlive the list of items.
    iris: Box<dyn Fn(Vec3) -> Vec3>,
}

/// Skin for the parts that carry no chart in the atlas.
///
/// Averaged from the baked skin rather than fixed, because a fixed one is a
/// pale mask on every avatar whose complexion is not pale — hands, feet, lids
/// and every facial feature came out lighter than the face they sat on.
fn bare_skin(albedo: &[u8]) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut taken = 0.0f32;
    for texel in albedo.chunks_exact(4) {
        if texel[3] == 0 {
            continue;
        }
        sum += Vec3::new(
            f32::from(texel[0]),
            f32::from(texel[1]),
            f32::from(texel[2]),
        ) / 255.0;
        taken += 1.0;
    }
    if taken == 0.0 {
        Vec3::new(0.86, 0.68, 0.60)
    } else {
        sum / taken
    }
}

impl Built<'_> {
    /// Everything as draw items, against the given backdrop.
    fn items<'a>(&'a self, wall: &'a PolyMesh) -> Vec<Item<'a>> {
        let mut items = vec![
            Item {
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
            },
            Item {
                positions: &self.positions,
                faces: &self.faces,
                normals: Some(&self.normals),
                paint: Paint::Atlas {
                    uvs: &self.uvs,
                    pixels: self.albedo,
                    side: self.atlas,
                },
                material: Material::skin(Vec3::ONE),
            },
        ];

        for part in &self.limbs {
            items.push(Item {
                positions: &part.positions,
                faces: &part.faces,
                normals: None,
                paint: Paint::Flat,
                material: Material::skin(self.bare),
            });
        }
        for (mesh, tone) in &self.worn {
            items.push(Item {
                positions: &mesh.positions,
                faces: &mesh.faces,
                normals: None,
                paint: Paint::Flat,
                material: Material::cloth(*tone),
            });
        }
        for (mesh, tone) in &self.locks {
            items.push(Item {
                positions: &mesh.positions,
                faces: &mesh.faces,
                normals: None,
                paint: Paint::Flat,
                material: Material::hair(*tone),
            });
        }
        if let Some(face) = &self.face {
            items.push(Item {
                positions: &face.positions,
                faces: &face.faces,
                normals: None,
                paint: Paint::Flat,
                material: Material::skin(self.bare),
            });
        }
        if let Some((globes, lids, _)) = &self.eyes {
            items.push(Item {
                positions: &globes.positions,
                faces: &globes.faces,
                normals: None,
                paint: Paint::Shaded(self.iris.as_ref()),
                material: Material::glossy(Vec3::ONE),
            });
            items.push(Item {
                positions: &lids.positions,
                faces: &lids.faces,
                normals: None,
                paint: Paint::Flat,
                material: Material::skin(self.bare),
            });
        }
        items
    }
}

/// A pale globe with a dark iris facing forward, so an eye reads as an eye.
fn iris(point: Vec3, centres: &[Vec3; 2]) -> Vec3 {
    let nearest = centres
        .iter()
        .min_by(|a, b| point.distance(**a).total_cmp(&point.distance(**b)))
        .copied()
        .unwrap_or(Vec3::ZERO);
    let forward = (point - nearest).normalize_or(Vec3::Z).z;
    if forward > 0.78 {
        Vec3::new(0.05, 0.06, 0.08)
    } else if forward > 0.50 {
        Vec3::new(0.24, 0.38, 0.46)
    } else {
        Vec3::new(0.93, 0.92, 0.90)
    }
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
