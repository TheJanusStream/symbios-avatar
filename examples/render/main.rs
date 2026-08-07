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
//! cargo run --release --example render -- --close hand --fist  # every finger curled
//! cargo run --release --example render -- --gaze 40  # look this many degrees to one side
//! cargo run --release --example render -- --bare      # no hair, to see the face
//! cargo run --release --example render -- --junction  # tint the skin by which bone deforms it
//! cargo run --release --example render -- --linear     # matrix skinning, to compare
//! cargo run --release --example render -- --hair 1,0,0,0.5,0.6,0.2,9,0.45  # length,volume,coverage,part,wave,shade,locks,curl
//! cargo run --release --example render -- --skin 0.9,-1,0.4,0,0  # melanin,undertone,blush,freckles,stubble
//! cargo run --release --example render -- --face 1,0.5,1,1,0.5,0.5 # nose,noseWidth,brow,mouth,mouthWidth,ears
//! cargo run --release --example render -- --skull -1,1            # headBreadth,faceLength
//! cargo run --release --example render -- --pass ao   # or normal, albedo, shadow
//! cargo run --release --example render -- --quadruped
//! cargo run --release --example render -- --budget    # what one avatar costs
//! cargo run --release --example render -- --cost      # and what it costs to build
//! ```

mod light;
mod scene;

use glam::{Mat4, Quat, Vec3};
use light::Image;
use scene::{Frame, GBuffer, Item, Material, Paint, ShadowMap};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarMesh, AvatarRecord, Blink, FaceParams, FootingConfig,
    Gait, GazeConfig, Ground, HairParams, Limb, MeshKind, PolyMesh, Pose, Rig, Role, SkinParams,
    Stride, Zone, anim::gait, anim::gaze, anim::plant_feet_of,
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
    let bare = args.iter().any(|arg| arg == "--bare");
    let fist = args.iter().any(|arg| arg == "--fist");
    // Which bone holds which patch of skin through the head-to-body junction.
    let junction = args.iter().any(|arg| arg == "--junction");
    // Which stage to show instead of the finished picture.
    let pass = value("--pass").cloned();
    // Six numbers, in the order the axes are declared: length, volume,
    // coverage, part, wave, shade, and optionally the group count after them.
    // For walking the parameter space by eye, which is the only way any of it
    // got tuned — including the group count, which is where the helmet the
    // module docs warn about would show.
    let overridden: Vec<f32> = value("--hair")
        .map(|spec| {
            spec.split(',')
                .filter_map(|a| a.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    // Five numbers, in the order the axes are declared: melanin, undertone,
    // blush, freckles, stubble. A complexion cannot be judged from its numbers,
    // and the melanin ramp is the one place in the crate where "it looks right
    // to me" is the least trustworthy test there is.
    let complexion: Vec<f32> = value("--skin")
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

    // Six numbers, in the order the axes are declared: nose, its width, brow,
    // mouth, its width, ears. Set on the record rather than through a config
    // override, because that is where a face lives. Same reason `--skin` exists:
    // a face cannot be judged from its numbers, and these are the numbers that
    // decide whether two records describe two people (#61).
    if let Some(spec) = value("--face") {
        let given: Vec<f32> = spec
            .split(',')
            .filter_map(|axis| axis.trim().parse().ok())
            .collect();
        let axis = |at: usize, fallback: f32| given.get(at).copied().unwrap_or(fallback);
        record.face = FaceParams {
            nose: axis(0, record.face.nose),
            nose_width: axis(1, record.face.nose_width),
            brow: axis(2, record.face.brow),
            mouth: axis(3, record.face.mouth),
            mouth_width: axis(4, record.face.mouth_width),
            ears: axis(5, record.face.ears),
        };
    }

    // The skull's own two axes, which live on the body plan rather than on the
    // face: how broad the head is across and how long the face is below the
    // eyes. They are the pair the owner called for in #61 and the pair whose
    // default has to be chosen by looking at both ends of the range, so the
    // instrument that looks needs a way to ask for an end.
    if let Some(spec) = value("--skull") {
        let given: Vec<f32> = spec
            .split(',')
            .filter_map(|axis| axis.trim().parse().ok())
            .collect();
        if let Archetype::Humanoid(ref mut params) = record.archetype {
            if let Some(&breadth) = given.first() {
                params.head_breadth = breadth;
            }
            if let Some(&length) = given.get(1) {
                params.face_length = length;
            }
        }
        record.sanitize();
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
            locks: axis(6, record.hair.locks as f32) as u32,
            curl: axis(7, record.hair.curl),
        }),
        complexion: (!complexion.is_empty()).then(|| {
            let axis = |at: usize, fallback: f32| complexion.get(at).copied().unwrap_or(fallback);
            SkinParams {
                melanin: axis(0, record.skin.melanin),
                undertone: axis(1, record.skin.undertone),
                blush: axis(2, record.skin.blush),
                freckles: axis(3, record.skin.freckles),
                stubble: axis(4, record.skin.stubble),
            }
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
    if args.iter().any(|arg| arg == "--cost") {
        cost(&record, &config);
    }
    // How far to one side the body is looking, in degrees. **The instrument
    // gap #123 fell through.** Every sheet this tool has ever drawn was a rest
    // pose from the neck up, so the entire lower face could be bound to the
    // neck -- and was -- with the chin and the mouth moving zero millimetres
    // under a head turn, and nothing here able to show it. The only instrument
    // that could was the Bevy viewer, and it took the owner looking at one.
    let gaze = value("--gaze").and_then(|degrees| degrees.parse::<f32>().ok());
    let subject = Subject::new(avatar, linear, bare, fist, junction, pass, gaze);

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

/// Prints what one avatar costs to *build*, at three atlas sizes.
///
/// **A baseline exists to be re-taken, so it lives in the tool rather than in a
/// comment** (#56). The figures wander by a percent between runs and are useful
/// to about that: what they are for is attributing a regression to a change,
/// which needs a before as well as an after.
///
/// Two separate levers, and this print is arranged to show that they are
/// separate. Time scales with the atlas because `paint_skin` calls
/// `nearest_bone` per texel; geometry does not move with the atlas at all. At
/// the shipping 1024 the atlas is fifteen times the drawn geometry, so atlas
/// size is the whole of the memory lever and `nearest_bone` is the whole of the
/// time lever — see #56 for the remedies, which are deliberately not here.
///
/// The geometry figure is counted rather than estimated: position, normal, uv,
/// colour, four joint indices and four weights per drawn vertex.
fn cost(record: &AvatarRecord, config: &AvatarConfig) {
    /// Bytes one drawn vertex occupies, counted channel by channel.
    const PER_VERTEX: usize = 3 * 4 + 3 * 4 + 2 * 4 + 3 * 4 + 4 * 2 + 4 * 4;

    for atlas in [1024u32, 512, 256] {
        let at = AvatarConfig {
            atlas,
            ..config.clone()
        };
        // One warm build first: the first build of a process pays for
        // allocations the steady-state case does not, and the steady-state case
        // is the one twenty avatars in a room will see.
        let Some(_) = Avatar::build_with(record, &at) else {
            eprintln!("the body could not be built at atlas {atlas}");
            return;
        };
        let mut best = f64::MAX;
        let mut built = None;
        for _ in 0..3 {
            let start = std::time::Instant::now();
            let avatar = Avatar::build_with(record, &at);
            best = best.min(start.elapsed().as_secs_f64() * 1000.0);
            built = avatar;
        }
        let Some(avatar) = built else {
            return;
        };
        let vertices: usize = avatar
            .meshes
            .iter()
            .map(|drawn| drawn.mesh.positions.len())
            .sum();
        println!(
            "atlas {atlas:>5}  build {best:>7.1} ms  texture {:>6} KiB  geometry {:>4} KiB",
            avatar.budget.texture_bytes / 1024,
            vertices * PER_VERTEX / 1024,
        );
    }
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
    /// Whether to draw the hair at all.
    ///
    /// A shell of hair covers the ears and most of the brow on almost every
    /// seed, and the face is the thing under active work — judging a feature
    /// through a fringe is judging the fringe (#67, #59).
    bare: bool,
    /// Whether every finger is curled, to show the hand rig working.
    fist: bool,
    /// Whether to tint the skin by which bone deforms it, through the junction
    /// between the head and the body. See [`junction_tint`].
    junction: bool,
    /// How far to one side the body is looking, in degrees, if at all.
    gaze: Option<f32>,
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
    fn new(
        avatar: Avatar,
        linear: bool,
        bare: bool,
        fist: bool,
        junction: bool,
        pass: Option<String>,
        gaze: Option<f32>,
    ) -> Self {
        let (lo, hi) = avatar.parts.body.bounds();
        Self {
            gait: Gait::natural(&avatar.rig),
            stride: Stride::for_body(&avatar.rig, 1.0),
            middle: (lo + hi) * 0.5,
            reach: (hi - lo).max_element().max(0.1),
            avatar,
            linear,
            bare,
            fist,
            junction,
            gaze,
            pass,
        }
    }

    /// The body standing still, or with its hands closed.
    ///
    /// Closing them is what a hand rig is *for*, and a rest pose cannot show
    /// whether one works: before #113 the whole hand rode the wrist, so every
    /// finger joint in the world would have moved nothing and the rest pose
    /// looked exactly the same either way.
    fn standing(&self) -> Pose {
        let rig = &self.avatar.rig;
        let mut pose = Pose::rest(rig);
        if self.fist {
            for joint in 0..rig.len() {
                if rig.joints[joint].role == Role::Digit {
                    pose.rotations[joint] = Quat::from_rotation_x(0.75);
                }
            }
        }
        // Through the real gaze system rather than a rotation dropped on the
        // head joint, because the defect this exists to show is about which
        // joints a look-at actually turns: it distributes down the neck and the
        // chest, and a face bound to the wrong one of those lags by exactly the
        // share that joint did not get.
        if let Some(degrees) = self.gaze {
            let head = rig.in_zone(Zone::Head).first().copied().unwrap_or_default();
            let from = rig.joints[head].position;
            let angle = degrees.to_radians();
            let target = from + Vec3::new(angle.sin(), 0.0, angle.cos()) * 2.0;
            gaze::look_at(rig, &mut pose, target, &GazeConfig::default());
        }
        pose
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
            // A biped's foot is not a part any more — it is meshed into the leg
            // (#111) — so it is framed by zone, the way the head is. One foot,
            // not both: a pair framed together sits a body's width apart and
            // zooms straight back out to the body sheet. The leg above it comes
            // along, because a foot with no ankle in shot says nothing about
            // whether the ankle is right.
            Focus::Foot if parts.extremities.feet.is_empty() => {
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                let limb = Limb::HindLeft;
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    if matches!(zone, Zone::Extremity(at) | Zone::LowerLimb(at) if *at == limb) {
                        hold(deformed[vertex]);
                    }
                }
            }
            Focus::Hand | Focus::Foot => {
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
        // Only the skin is bound to the rig, so only the skin can be tinted by
        // it. Computed once for the sheet rather than once per view: the
        // classification is of the surface, and the surface does not turn.
        let mut span: Vec<Reach> = vec![None; self.avatar.rig.len()];
        let tints: Vec<Option<Vec<Vec3>>> = built
            .iter()
            .map(|drawn| {
                (self.junction && drawn.kind == MeshKind::Skin)
                    .then(|| junction_tint(&drawn.mesh, &self.avatar.rig, &mut span))
            })
            .collect();
        if self.junction {
            // Once per run, not once per sheet. A head sheet is drawn twice —
            // eyes open and eyes shut — and the same table printed twice reads
            // as two different bodies.
            static SAID: std::sync::Once = std::sync::Once::new();
            SAID.call_once(|| report_junction(&self.avatar.rig, &span));
        }

        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        let side = VIEW * SUPERSAMPLE;
        for (index, frame) in frames.iter().enumerate() {
            let wall = backdrop(frame);
            let items = items(
                &built,
                &wall,
                self.avatar.skin.albedo.as_slice(),
                self.atlas(),
                &tints,
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
        let bare = |built: Vec<AvatarMesh>| {
            built
                .into_iter()
                .filter(|drawn| !self.bare || drawn.kind != MeshKind::Hair)
                .collect()
        };
        if !self.linear {
            return bare(self.avatar.posed(pose, closure));
        }
        let posed = pose.forward(&self.avatar.rig);
        bare(
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
                .collect(),
        )
    }
}

/// Which bone deforms each patch of skin, through the head-to-body junction.
///
/// **There is no boundary between a head and a body on this surface** — every
/// audit in the crate says so, and `headaudit` prints it in as many words: head
/// and neck are one continuous surface with no boundary anywhere. So "the
/// connecting element" is not a thing that can be pointed at geometrically. It
/// has to be a *definition*, and the one drawn here is the rig's own: which bone
/// the skin binding gives each vertex to.
///
/// **A bone is held by its PROXIMAL joint**, which is the thing this picture is
/// most worth looking at for. [`symbios_avatar::rig::skin`] explains why — the
/// rotation stored at a joint turns that joint's children about it — so the
/// bone from the neck up to the head is owned by the NECK, and the jaw and chin
/// hanging off its far end are the neck's surface as far as deformation is
/// concerned. That is #123's whole subject and the reason `owner_of` and its
/// `COVERED` constant exist. Here it is as a picture rather than as a constant.
///
/// Three bones are named and everything else is left alone:
///
/// ```text
///   girdle -> neck     blue      the shoulder's hold on the column
///   neck   -> head     green     the connecting element itself
///   head   -> crown    red       the head's own
/// ```
///
/// **Tinted by strength, not just by label.** The weight is smooth across the
/// surface and a hard classification would draw a crisp boundary that does not
/// exist in the deformation, which is exactly the kind of picture that gets
/// believed. So the colour is mixed toward white by how strongly that joint
/// actually holds the vertex: a firmly-held patch reads saturated and a
/// contested one fades out. Where the colours run into each other is where the
/// blend is.
///
/// A joint may own several bones — the girdle has three leaving it — so the
/// bone is found by taking the vertex's dominant joint and then the nearest of
/// the segments leaving it, the same search `SkinWeights::zone_map` makes for
/// the same reason.
fn junction_tint(mesh: &PolyMesh, rig: &Rig, span: &mut [Reach]) -> Vec<Vec3> {
    let head = rig.in_zone(Zone::Head).first().copied();
    let Some(head) = head else {
        return vec![Vec3::ONE; mesh.positions.len()];
    };
    let neck = rig.joints[head].parent;
    let girdle = neck.and_then(|neck| rig.joints[neck].parent);

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); rig.len()];
    for joint in 0..rig.len() {
        if let Some(parent) = rig.joints[joint].parent {
            children[parent].push(joint);
        }
    }

    let named = |joint: usize| -> Option<Vec3> {
        if Some(joint) == girdle {
            Some(Vec3::new(0.35, 0.55, 1.00))
        } else if Some(joint) == neck {
            Some(Vec3::new(0.30, 1.00, 0.45))
        } else if joint == head {
            Some(Vec3::new(1.00, 0.40, 0.35))
        } else {
            None
        }
    };

    let mut record = |joint: usize, y: f32| {
        let seen = span[joint].get_or_insert((f32::MAX, f32::MIN, 0));
        *seen = (seen.0.min(y), seen.1.max(y), seen.2 + 1);
    };

    mesh.skin
        .iter()
        .zip(&mesh.positions)
        .map(|(influences, &point)| {
            let joint = influences[0].joint as usize;
            let Some(colour) = named(joint) else {
                return Vec3::ONE;
            };
            // A named joint still has to be carrying this vertex on the bone we
            // mean. The girdle's other two bones are the clavicles, and a
            // shoulder tinted as the neck's would be this picture telling the
            // exact lie it is drawn to expose.
            let toward = children[joint]
                .iter()
                .min_by(|&&a, &&b| {
                    let of = |child: usize| {
                        segment_distance(
                            point,
                            rig.joints[joint].position,
                            rig.joints[child].position,
                        )
                    };
                    of(a).total_cmp(&of(b))
                })
                .copied();
            let wanted = if joint == head {
                toward.is_some()
            } else {
                toward
                    == Some(if Some(joint) == neck {
                        head
                    } else {
                        neck.unwrap_or(head)
                    })
            };
            if !wanted {
                return Vec3::ONE;
            }
            record(joint, point.y);
            // Mixed toward white by how strongly this joint holds the vertex,
            // so the blend shows as a gradient instead of a hard edge.
            let hold = influences
                .iter()
                .filter(|influence| influence.joint as usize == joint)
                .map(|influence| influence.weight)
                .sum::<f32>()
                .clamp(0.0, 1.0);
            Vec3::ONE.lerp(colour, hold)
        })
        .collect()
}

/// How far up the body one bone's skin reaches, and how much of it there is.
///
/// Accumulated across every skin mesh before anything is printed: eyelids are
/// skin too, and reported per mesh the lids come out as a head with no neck
/// under it, which reads as the classification having lost the neck.
type Reach = Option<(f32, f32, usize)>;

/// What the junction overlay claims, as numbers beside it.
///
/// **A false-colour overlay is an instrument.** One that cannot be checked
/// against a figure is how this project has been misled twenty-one times, so the
/// height each bone's territory spans is printed next to the picture that draws
/// it. If a band appears somewhere these numbers do not put it, believe the
/// numbers and fix the picture.
fn report_junction(rig: &Rig, span: &[Reach]) {
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let neck = rig.joints[head].parent;
    let girdle = neck.and_then(|neck| rig.joints[neck].parent);

    // **Reported as a share of the neck-to-head bone as well as in millimetres,
    // because that is the unit its own design is written in and this overlay
    // caused a wrong conclusion for want of it.** `rig::skin::owner_of` places
    // the split between the head's bone and the neck's at a fixed 0.25 along
    // that bone, deliberately halfway between the throat's floor at 0.10 and
    // the chin's projection at 0.40 — both of which are constants on every
    // body. Read in millimetres alone this boundary looks like it disagrees
    // with `Skull`'s measured throat by 20 mm; read in the bone's own unit it
    // is plainly the designed gap between two landmarks.
    let bone = neck.map(|neck| (rig.joints[neck].position.y, rig.joints[head].position.y));
    let along = |y: f32| match bone {
        Some((from, to)) if (to - from).abs() > f32::EPSILON => (y - from) / (to - from),
        _ => f32::NAN,
    };

    println!("the head-to-body junction, by which bone deforms the skin");
    println!("  a bone is held by its PROXIMAL joint, so these are named from below");
    for (joint, colour, what) in [
        (girdle, "blue ", "girdle -> neck"),
        (neck, "green", "neck   -> head"),
        (Some(head), "red  ", "head   -> crown"),
    ] {
        match joint.and_then(|joint| span[joint]) {
            Some((lo, hi, count)) => println!(
                "  {colour}  {what}: {count:5} vertices, {:7.1} to {:7.1} mm up the body, \
                 {:.2} to {:.2} along the neck-to-head bone",
                lo * 1000.0,
                hi * 1000.0,
                along(lo),
                along(hi),
            ),
            None => println!("  {colour}  {what}: no skin bound to it"),
        }
    }
    for (joint, name) in [(neck, "neck"), (girdle, "girdle")] {
        if let Some(joint) = joint {
            println!(
                "  the {name} joint itself sits at {:7.1} mm",
                rig.joints[joint].position.y * 1000.0
            );
        }
    }
}

/// Distance from a point to a segment, for naming which bone a vertex lies on.
fn segment_distance(point: Vec3, from: Vec3, to: Vec3) -> f32 {
    let along = to - from;
    let length = along.length_squared();
    if length <= f32::EPSILON {
        return point.distance(from);
    }
    let t = ((point - from).dot(along) / length).clamp(0.0, 1.0);
    point.distance(from + along * t)
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
    tints: &'a [Option<Vec<Vec3>>],
) -> Vec<Item<'a>> {
    let mut items = vec![Item {
        positions: &wall.positions,
        faces: &wall.faces,
        normals: None,
        paint: Paint::Flat,
        tint: None,
        material: Material {
            albedo: Vec3::new(0.30, 0.31, 0.35),
            roughness: 1.0,
            specular: 0.02,
            wrap: 0.0,
        },
    }];

    for (drawn, tint) in built.iter().zip(tints) {
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
            tint: tint.as_deref(),
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
