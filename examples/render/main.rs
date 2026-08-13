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
//! cargo run --release --example render -- --close brows # or any follicle region
//! cargo run --release --example render -- --brow thick  # or natural, none
//! cargo run --release --example render -- --scalp bob 0.8 # crop, bob, long, tied, curly
//! cargo run --release --example render -- --moustache handlebar 0.9 # chevron, handlebar, pencil
//! cargo run --release --example render -- --chin braided 0.8 # goatee, full, braided
//! cargo run --release --example render -- --flanks full 0.7 # sideburns, full
//! cargo run --release --example render -- --close hand --fist  # every finger curled
//! cargo run --release --example render -- --gaze 40  # look this many degrees to one side
//! cargo run --release --example render -- --bare      # no hair or clothes, to see the body
//! cargo run --release --example render -- --junction  # tint the skin by which bone deforms it
//! cargo run --release --example render -- --jawbind   # tint the skin by how the JAW bone holds it
//! cargo run --release --example render -- --follicles # tint the skin by where hair may grow
//! cargo run --release --example render -- --clumps    # grow the new clump engine on all five regions
//! cargo run --release --example render -- --jaw 20    # open the mouth this many degrees
//! cargo run --release --example render -- --jawsweep # tune the jaw's binding reach by measurement
//! cargo run --release --example render -- --clip Punch_Cross            # a CC0 clip, retargeted
//! cargo run --release --example render -- --clip Wave --clipframes 12   # more frames of it
//! cargo run --release --example render -- --linear     # matrix skinning, to compare
//! cargo run --release --example render -- --hair 1,0,0,0.5,0.6,0.2,9,0.45  # length,volume,coverage,part,wave,shade,locks,curl
//! cargo run --release --example render -- --skin 0.9,-1,0.4,0,0  # melanin,undertone,blush,freckles,stubble
//! cargo run --release --example render -- --hair 1 0.5 1 0.5   # scalp length,thickness,density,droop
//! cargo run --release --example render -- --hair 0 0 0 0 0     # a fifth zero shaves every region
//! cargo run --release --example render -- --face 1,0.5,1,1,0.5,0.5 # nose,noseWidth,brow,mouth,mouthWidth,ears
//! cargo run --release --example render -- --skull -1,1            # headBreadth,faceLength
//! cargo run --release --example render -- --femininity 1  # the frame axis, -1 .. +1
//! cargo run --release --example render -- --mass 1 --fat 0.10  # heavy and lean: muscular
//! cargo run --release --example render -- --age 80  # the age axis, 18 .. 80 years
//! cargo run --release --example render -- --pass ao   # or normal, albedo, shadow
//! cargo run --release --example render -- --quadruped
//! cargo run --release --example render -- --budget    # what one avatar costs
//! cargo run --release --example render -- --cost      # and what it costs to build
//! ```

mod light;
mod scene;

use glam::{Mat4, Quat, Vec3};
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use light::Image;
use scene::{Frame, GBuffer, Item, Material, Paint, ShadowMap};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarMesh, AvatarRecord, Blink, Canon, EyeParams, FaceParams,
    FootingConfig, Gait, GazeConfig, Ground, Influence, Limb, MAX_INFLUENCES, MeshKind,
    PolyMesh, Pose, Rig, Role, Skeleton, SkinConfig, SkinParams, SkinWeights, Stride, Zone,
    anim::contacts_in, anim::gait, anim::gaze, anim::plant_feet_of, face::Skull, gltf::Gltf,
    hair::{
        Follicle, FollicleParams, Follicles, Growth,
        clump::{Bed, Fall, Sowing},
    },
    retarget,
};

/// Where the CC0 reference animations sit, relative to this checkout.
///
/// Not vendored — eleven megabytes of somebody else's CC0 — so `--clip` fails
/// with this path in the message rather than with a file-not-found.
const LIBRARY: [&str; 2] = [
    "../mesh2motion-app/static/animations/human-base-animations.glb",
    "../mesh2motion-app/static/animations/human-addon-animations.glb",
];

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
    /// One follicle region, framed on the mask itself.
    ///
    /// **A head shot is not close enough to judge a brow** (#205). A brow is
    /// eight millimetres of the head's own height, so at the head close-up's
    /// framing the whole region is thirty pixels and every defect in it is one:
    /// the shipped brow was combed the wrong way for two issues and the sheet
    /// showed it as a smudge.
    ///
    /// Framed on whichever vertices the region actually claims, so it follows
    /// the mask rather than a hand-picked box, and every region of the milestone
    /// gets its own shot for free.
    Region(Follicle),
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
    // A CC0 clip, retargeted and sampled across its own length. The one thing
    // no still of a rest pose can show is which SIDE a one-handed clip lands
    // on, because a mirrored body at rest looks exactly like a correct one
    // (#142); and a twist bug is invisible in a walk but not in a clip that
    // rolls (#139). Both want eyes on the real motion.
    let clip = value("--clip").cloned();
    let clip_frames = value("--clipframes")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(8);
    let focus = match value("--close").map(String::as_str) {
        Some("head") => Some(Focus::Head),
        Some("hand") => Some(Focus::Hand),
        Some("foot") => Some(Focus::Foot),
        Some(other) => match Follicle::ALL.into_iter().find(|it| it.name() == other) {
            Some(follicle) => Some(Focus::Region(follicle)),
            None => {
                eprintln!(
                    "unknown --close target {other}: expected head, hand, foot or one of {}",
                    Follicle::ALL.map(Follicle::name).join(", ")
                );
                std::process::exit(1);
            }
        },
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
    // The same question asked of the mandible instead: which skin does the
    // condyle-to-chin bone hold, and how strongly. See [`jaw_tint`].
    let jawbind = args.iter().any(|arg| arg == "--jawbind");
    // Where each of the five kinds of hair is allowed to grow. See
    // [`follicle_tint`], and `follicleaudit` for the same regions as numbers.
    let follicles = args.iter().any(|arg| arg == "--follicles");
    // Replace the shipped hair with the clump engine's own, grown on the five
    // regions. See [`grow_clumps`]: this is #201's judgement image, and it
    // draws through the real hair material rather than beside it.
    let clumps = args.iter().any(|arg| arg == "--clumps");
    // Which stage to show instead of the finished picture.
    let pass = value("--pass").cloned();
    // Six numbers, in the order the axes are declared: length, volume,
    // coverage, part, wave, shade, and optionally the group count after them.
    // For walking the parameter space by eye, which is the only way any of it
    // got tuned — including the group count, which is where the helmet the
    // module docs warn about would show.
    // Which brow style to wear, by name, since a style is not a number and the
    // two of them are what #205 is judged on. Named rather than indexed because
    // a sheet is compared against another sheet and `--brow thick` says which
    // one this is where `--brow 1` does not.
    let brow = value("--brow").map(|name| match name.as_str() {
        "none" => symbios_avatar::hair::BrowStyle::None,
        "natural" => symbios_avatar::hair::BrowStyle::Natural,
        "thick" => symbios_avatar::hair::BrowStyle::Thick,
        other => {
            eprintln!("unknown --brow style {other}: expected none, natural or thick");
            std::process::exit(1);
        }
    });
    // Which scalp style to wear, and the axis its own variant carries. `--scalp
    // bob` takes the variant's default; `--scalp bob 0.9` sets its axis, which is
    // the only way to walk an axis that exists on one style and not the others.
    let scalp = value("--scalp").map(|name| {
        let axis = args
            .iter()
            .position(|arg| arg == "--scalp")
            .and_then(|at| args.get(at + 2))
            .and_then(|it| it.parse::<f32>().ok())
            .unwrap_or(0.6);
        match name.as_str() {
            "none" => symbios_avatar::hair::ScalpStyle::None,
            "crop" => symbios_avatar::hair::ScalpStyle::Crop,
            "bob" => symbios_avatar::hair::ScalpStyle::Bob { fringe: axis },
            "long" => symbios_avatar::hair::ScalpStyle::Long { weight: axis },
            "tied" => symbios_avatar::hair::ScalpStyle::TiedBack { tail: axis },
            "curly" => symbios_avatar::hair::ScalpStyle::Curly { curl: axis },
            other => {
                eprintln!(
                    "unknown --scalp style {other}: expected none, crop, bob, long, tied or curly"
                );
                std::process::exit(1);
            }
        }
    });
    // Which moustache style to wear, and the axis its own variant carries — the
    // same shape as `--scalp`, since two of the three carry one (#206). The
    // painted layer comes on with it: a grown moustache over a bare lip is a
    // shot of the geometry rather than of the moustache, and the two layers are
    // judged together.
    let moustache = value("--moustache").map(|name| {
        let axis = args
            .iter()
            .position(|arg| arg == "--moustache")
            .and_then(|at| args.get(at + 2))
            .and_then(|it| it.parse::<f32>().ok())
            .unwrap_or(0.6);
        match name.as_str() {
            "none" => symbios_avatar::hair::MoustacheStyle::None,
            "chevron" => symbios_avatar::hair::MoustacheStyle::Chevron,
            "handlebar" => symbios_avatar::hair::MoustacheStyle::Handlebar { sweep: axis },
            "pencil" => symbios_avatar::hair::MoustacheStyle::Pencil { ride: axis },
            other => {
                eprintln!(
                    "unknown --moustache style {other}: expected none, chevron, handlebar or \
                     pencil"
                );
                std::process::exit(1);
            }
        }
    });
    // Which chin beard to wear, and the axis its own variant carries.
    let chin = value("--chin").map(|name| {
        let axis = args
            .iter()
            .position(|arg| arg == "--chin")
            .and_then(|at| args.get(at + 2))
            .and_then(|it| it.parse::<f32>().ok())
            .unwrap_or(0.6);
        match name.as_str() {
            "none" => symbios_avatar::hair::ChinStyle::None,
            "goatee" => symbios_avatar::hair::ChinStyle::Goatee { point: axis },
            "full" => symbios_avatar::hair::ChinStyle::Full,
            "braided" => symbios_avatar::hair::ChinStyle::Braided { twist: axis },
            other => {
                eprintln!("unknown --chin style {other}: expected none, goatee, full or braided");
                std::process::exit(1);
            }
        }
    });
    // Which flank beard to wear, and the axis its own variant carries.
    let flanks = value("--flanks").map(|name| {
        let axis = args
            .iter()
            .position(|arg| arg == "--flanks")
            .and_then(|at| args.get(at + 2))
            .and_then(|it| it.parse::<f32>().ok())
            .unwrap_or(0.6);
        match name.as_str() {
            "none" => symbios_avatar::hair::FlankStyle::None,
            "sideburns" => symbios_avatar::hair::FlankStyle::Sideburns { drop: axis },
            "full" => symbios_avatar::hair::FlankStyle::FullConnect { reach: axis },
            other => {
                eprintln!("unknown --flanks style {other}: expected none, sideburns or full");
                std::process::exit(1);
            }
        }
    });
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
    // The frame axis (#100), which is the only way to look at both ends of it
    // on the same body: rolling seeds to find a feminine one moves five other
    // axes at once, and the question is what THIS axis does.
    if let Some(spec) = value("--femininity")
        && let Ok(femininity) = spec.parse::<f32>()
    {
        record.composites.femininity = femininity;
        record.composites.sanitize();
    }
    // The two axes of #164, so a body can be looked at across the grid the
    // acceptance asks for: mass sets how much body there is, bodyFat how it is
    // spent. Low fat with high mass is muscular; high with high is heavy.
    if let Some(spec) = value("--mass")
        && let Ok(mass) = spec.parse::<f32>()
    {
        record.composites.mass = mass;
        record.composites.sanitize();
    }
    if let Some(spec) = value("--fat")
        && let Ok(fat) = spec.parse::<f32>()
    {
        record.composites.body_fat = fat;
        record.composites.sanitize();
    }
    // The age axis (#167), in whole years. Everything it does is sub-centimetre
    // on the body sheet — a settle of about 4 cm of stature end to end, a
    // deltoid 9% thinner — so it is meant to be looked at as a sweep of the
    // same seed rather than as one body, and the face terms want `--head`.
    if let Some(spec) = value("--age")
        && let Ok(age) = spec.parse::<u32>()
    {
        record.composites.age = age;
        record.composites.sanitize();
    }
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
        hair: (!overridden.is_empty()
            || brow.is_some()
            || scalp.is_some()
            || moustache.is_some()
            || chin.is_some()
            || flanks.is_some())
        .then(|| {
            // The scalp's own four axes, in the order `Cut` declares them, plus
            // a fifth that silences every region — which is what `--mane 0` on
            // the viewer does and what a bald judgement shot wants.
            let mut hair = record.hair;
            hair.scalp.cut.length = axis(0, hair.scalp.cut.length);
            hair.scalp.cut.thickness = axis(1, hair.scalp.cut.thickness);
            hair.scalp.cut.density = axis(2, hair.scalp.cut.density);
            hair.scalp.cut.droop = axis(3, hair.scalp.cut.droop);
            if axis(4, 1.0) <= 0.0 {
                hair.scalp.style = symbios_avatar::hair::ScalpStyle::None;
                hair.brows.style = symbios_avatar::hair::BrowStyle::None;
                hair.moustache.style = symbios_avatar::hair::MoustacheStyle::None;
                hair.chin.style = symbios_avatar::hair::ChinStyle::None;
                hair.flanks.style = symbios_avatar::hair::FlankStyle::None;
                for paint in [
                    &mut hair.scalp.skin,
                    &mut hair.brows.skin,
                    &mut hair.moustache.skin,
                    &mut hair.chin.skin,
                    &mut hair.flanks.skin,
                ] {
                    paint.density = 0.0;
                }
            }
            if let Some(style) = brow {
                hair.brows.style = style;
            }
            if let Some(style) = scalp {
                hair.scalp.style = style;
            }
            if let Some(style) = flanks {
                hair.flanks.style = style;
                if !matches!(style, symbios_avatar::hair::FlankStyle::None) {
                    hair.flanks.skin = symbios_avatar::hair::Paint {
                        density: 0.85,
                        colour: hair.scalp.roots,
                    };
                    hair.flanks.roots = hair.scalp.roots;
                    hair.flanks.tips = hair.scalp.roots;
                }
            }
            if let Some(style) = chin {
                hair.chin.style = style;
                if !matches!(style, symbios_avatar::hair::ChinStyle::None) {
                    hair.chin.skin = symbios_avatar::hair::Paint {
                        density: 0.85,
                        colour: hair.scalp.roots,
                    };
                    hair.chin.roots = hair.scalp.roots;
                    hair.chin.tips = hair.scalp.roots;
                }
            }
            if let Some(style) = moustache {
                hair.moustache.style = style;
                // The record's default is a shaved lip — no geometry and no
                // paint — so a flag asking for a moustache has to turn the skin
                // layer on under it as well.
                if !matches!(style, symbios_avatar::hair::MoustacheStyle::None) {
                    hair.moustache.skin = symbios_avatar::hair::Paint {
                        density: 0.85,
                        colour: hair.scalp.roots,
                    };
                    hair.moustache.roots = hair.scalp.roots;
                    hair.moustache.tips = hair.scalp.roots;
                }
            }
            hair
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
        // **Undressed at build time, not by dropping the cloth draw** (#117).
        // A dressed body does not emit the skin its clothes cover, so filtering
        // the cloth mesh out of the merge leaves a torso with its middle
        // missing — which is what this flag did until the body it was pointed
        // at stopped drawing that skin.
        dressed: !bare,
        ..Default::default()
    };

    let Some(mut avatar) = Avatar::build_with(&record, &config) else {
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
    // How far the mouth is opened, in degrees. The same instrument gap one step
    // further in: #134 built a mandible bone and NOTHING had ever rotated it, so
    // its binding reach shipped declared unsourced. A weight map cannot show
    // whether a jaw articulates — dual quaternion blending makes a bad reach
    // deform identically to a good one at rest and under a head turn — and only
    // turning the PIVOT can (#135).
    let jaw = value("--jaw").and_then(|degrees| degrees.parse::<f32>().ok());
    let show = Show {
        linear,
        bare,
        fist,
        junction,
        jawbind,
        follicles,
        // Carried rather than defaulted, because the brow region's two ends are
        // placed in `Canon::apart` and that is the one landmark eye spacing
        // moves. Two other call sites in this file read a default `EyeParams`
        // and are right to — neither of them measures anything the eye touches.
        eyes: record.eyes,
        pass,
        gaze,
        jaw,
    };
    if clumps {
        grow_clumps(&mut avatar, &record);
    }
    let mandible = jaw_bone(&avatar.rig, &record.skeleton());
    if (jaw.is_some() || jawbind) && mandible.is_none() {
        eprintln!("this body has no jaw bone: --jaw and --jawbind need a humanoid");
        std::process::exit(1);
    }
    if args.iter().any(|arg| arg == "--jawsweep") {
        match mandible {
            Some(mandible) => sweep_jaw(&avatar, mandible, jaw.unwrap_or(20.0)),
            None => eprintln!("this body has no jaw bone to sweep"),
        }
    }
    let subject = Subject::new(avatar, show, mandible);

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
        Some(Focus::Region(follicle)) => &format!("render_{}", follicle.name()),
        None => "render",
    };

    if let Some(wanted) = &clip {
        match clip_sheets(&subject, wanted, clip_frames, &shoot) {
            Ok(sheets) => {
                for (frame, sheet) in sheets.iter().enumerate() {
                    write(&out, &format!("{stem}_clip_{frame:02}"), sheet);
                }
                println!("rendered {} frames of {wanted}", sheets.len());
            }
            Err(why) => {
                eprintln!("{why}");
                std::process::exit(1);
            }
        }
        return;
    }

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

/// What a sheet is asked to show, beyond the body itself.
///
/// Gathered into one struct rather than passed one by one: every one of these
/// is an instrument switch, they arrive together and they travel together, and
/// the constructor was already at the argument count where the next one would
/// have been a lint.
struct Show {
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
    /// Whether to tint the skin by which hair region owns it. See
    /// [`follicle_tint`].
    follicles: bool,
    /// The record's own eye axes, which the follicle overlay needs to place a
    /// brow. See where this is set.
    eyes: EyeParams,
    /// Whether to tint the skin by the mandible's hold on it. See [`jaw_tint`].
    jawbind: bool,
    pass: Option<String>,
    /// How far to one side the body is looking, in degrees, if at all.
    gaze: Option<f32>,
    /// How far the mouth is opened, in degrees, if at all.
    jaw: Option<f32>,
}

/// The mandible, as the rig carries it.
///
/// **The bone is owned by its PROXIMAL joint**, which is the whole reason this
/// pair is named rather than just the tip: rotating the PIVOT is what opens the
/// mouth, and rotating the tip does nothing at all — it is a leaf, and
/// `rig::skin` gives a leaf no weight from the body's own surface.
#[derive(Clone, Copy)]
struct JawBone {
    /// The hinge at the ear, which the bone is owned by.
    pivot: usize,
    /// The chin, which the bone leads into.
    tip: usize,
}

/// The mandible in `rig`, if this body has one.
///
/// Found through the SKELETON's own marker flag rather than by position or by
/// index, because a marker is exactly what the crate calls a rig-only node and
/// a second definition of one here is a second implementation of #134. The jaw
/// is the marker whose parent is also a marker: the chain runs
/// `head → pivot → tip` and only the tip has a marked parent.
fn jaw_bone(rig: &Rig, skeleton: &Skeleton) -> Option<JawBone> {
    let marked = |joint: usize| {
        rig.joints[joint]
            .node
            .is_some_and(|node| skeleton.nodes[node as usize].marker)
    };
    let tip = (0..rig.len())
        .find(|&joint| marked(joint) && rig.joints[joint].parent.is_some_and(marked))?;
    Some(JawBone {
        pivot: rig.joints[tip].parent?,
        tip,
    })
}

/// Sheets of a body playing one CC0 clip, evenly across the clip's own length.
///
/// **The instrument for the two defects a still cannot show.** A mirrored
/// correspondence poses a rest body identically to a correct one, so only a
/// one-handed clip says which side ours landed on (#142); and a twist bug moves
/// a bone's axes without moving its position, so only a roll-heavy clip shows it
/// (#139). Both go through `retarget::clip` — the same path a bake takes — so
/// what is on screen is what would be baked.
///
/// The feet are re-planted at playback rather than trusted from the clip,
/// because the source was authored on a 1.830 m reference and this body is
/// whatever the record says. That is the retargeter's own recorded position: a
/// baked contact would freeze one ground plane into a clip that has to play on
/// every other.
fn clip_sheets(
    subject: &Subject,
    wanted: &str,
    frames: usize,
    shoot: &impl Fn(&Pose, f32) -> Option<Image>,
) -> Result<Vec<Image>, String> {
    let mut found = None;
    for path in LIBRARY {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let library = Gltf::read(&bytes).map_err(|why| format!("{path}: {why}"))?;
        if let Some(animation) = library.clip(wanted) {
            found = Some((library, animation));
            break;
        }
    }
    let Some((library, animation)) = found else {
        return Err(format!(
            "no clip called {wanted} in {} or {} — and if neither file is \
             checked out beside this repository, that is why",
            LIBRARY[0], LIBRARY[1]
        ));
    };

    let rig = &subject.avatar.rig;
    let skin = library.skin(0).map_err(|why| why.to_string())?;
    let matched =
        retarget::Correspondence::human(rig, &library, &skin).map_err(|why| why.to_string())?;
    let baked = retarget::clip(rig, &library, &matched, animation, 30.0, false)
        .map_err(|why| why.to_string())?;
    println!(
        "{wanted}: {:.2} s, {} of {} tracks move, {} bytes baked",
        baked.duration(),
        baked.moving(),
        matched.len(),
        baked.bytes()
    );

    let mut blink = Blink::seeded(1);
    let count = frames.max(1);
    let mut sheets = Vec::with_capacity(count);
    for frame in 0..count {
        let time = baked.duration() * frame as f32 / count as f32;
        let closure = blink.advance(baked.duration() / count as f32);
        let mut pose = baked.pose(rig, time);
        // **Only the feet this frame already has on the ground.** Handing
        // `ground_contacts` straight to the solve plants both feet whatever the
        // body is doing, which drags a foot that is in the air down onto the
        // floor — obvious in a run, hidden in a walk. `contacts_in` asks the
        // pose rather than the rig.
        //
        // It does NOT detect a lying or sitting body, and the first version of
        // this comment claimed it did. Measured on the shipped clips, Sleeping
        // and Sitting_Idle both report two contacts, correctly: a body on its
        // back has its heels on the floor beside its back.
        let stance = contacts_in(rig, &pose);
        plant_feet_of(
            rig,
            &mut pose,
            &stance,
            |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
            &FootingConfig::default(),
        );
        sheets.push(shoot(&pose, closure).ok_or("this body has no such part to frame")?);
    }
    Ok(sheets)
}

/// An avatar, and the gait it walks with.
struct Subject {
    avatar: Avatar,
    show: Show,
    /// The mandible, for `--jaw` and `--jawbind`.
    jaw: Option<JawBone>,
    gait: Gait,
    stride: Stride,
    /// The centre of the body's rest extent.
    middle: Vec3,
    /// Its largest side, so a long body is framed by its length.
    reach: f32,
}

impl Subject {
    /// Wraps a built avatar in what a contact sheet additionally needs.
    fn new(avatar: Avatar, show: Show, jaw: Option<JawBone>) -> Self {
        let (lo, hi) = avatar.parts.body.bounds();
        Self {
            gait: Gait::natural(&avatar.rig),
            stride: Stride::for_body(&avatar.rig, 1.0),
            middle: (lo + hi) * 0.5,
            reach: (hi - lo).max_element().max(0.1),
            avatar,
            show,
            jaw,
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
        if self.show.fist {
            for joint in 0..rig.len() {
                if rig.joints[joint].role == Role::Digit {
                    pose.rotations[joint] = Quat::from_rotation_x(0.75);
                }
            }
        }
        // About the lateral axis, at the PIVOT. Positive drops the chin and
        // draws it back, which is the arc a mandible opens along: the condyle
        // stays put at the ear and everything forward of it swings down. Set
        // directly rather than through a system because there is no jaw system
        // yet — that is what #118 is for, and this is the instrument that has to
        // exist before one can be judged.
        if let (Some(degrees), Some(jaw)) = (self.show.jaw, self.jaw) {
            pose.rotations[jaw.pivot] = Quat::from_rotation_x(degrees.to_radians());
        }
        // Through the real gaze system rather than a rotation dropped on the
        // head joint, because the defect this exists to show is about which
        // joints a look-at actually turns: it distributes down the neck and the
        // chest, and a face bound to the wrong one of those lags by exactly the
        // share that joint did not get.
        if let Some(degrees) = self.show.gaze {
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
            // Every vertex the region has a real claim on, which is the mask
            // deciding the frame. Held above a quarter rather than above zero: a
            // fade's outer tail reaches a long way for very little weight, and
            // framing on that zooms back out to the head.
            Focus::Region(follicle) => {
                let follicles = regions_of(&self.avatar, &self.show.eyes)?;
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                for (vertex, point) in parts.body.positions.iter().enumerate() {
                    if follicles.weight_in_body(follicle, *point) > 0.25 {
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
                if drawn.kind != MeshKind::Skin {
                    return None;
                }
                match (self.show.junction, self.show.jawbind, self.jaw) {
                    (true, _, _) => Some(junction_tint(&drawn.mesh, &self.avatar.rig, &mut span)),
                    (_, true, Some(jaw)) => Some(jaw_tint(&drawn.mesh, jaw)),
                    _ if self.show.follicles => {
                        Some(follicle_tint(&drawn.mesh, &self.avatar, &self.show.eyes))
                    }
                    _ => None,
                }
            })
            .collect();
        // Once per run, not once per sheet. A head sheet is drawn twice — eyes
        // open and eyes shut — and the same table printed twice reads as two
        // different bodies.
        static SAID: std::sync::Once = std::sync::Once::new();
        if self.show.junction {
            SAID.call_once(|| report_junction(&self.avatar.rig, &span));
        } else if let Some(jaw) = self
            .jaw
            .filter(|_| self.show.jawbind || self.show.jaw.is_some())
        {
            SAID.call_once(|| report_jaw(&self.avatar, jaw, pose, self.show.jaw));
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
            let shaded = match &self.show.pass {
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
        // Hair only: the clothes are gone before this, because a body built
        // dressed has no skin under them to show (see `dressed` above).
        let bare = |built: Vec<AvatarMesh>| {
            built
                .into_iter()
                .filter(|drawn| !self.show.bare || drawn.kind != MeshKind::Hair)
                .collect()
        };
        if !self.show.linear {
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

/// How strongly the mandible holds each patch of skin.
///
/// **The question nobody had answered about #134's jaw.** The bone runs
/// diagonally through the face's interior, from the hinge at the ear to the
/// chin, so which vertices its bounded falloff actually catches is not
/// something anyone can read off the constants: it might be the lower lip, the
/// corners of the mouth, a midline stripe, or nothing at all. Guessing is what
/// [`JAW_REACH`]'s *unsourced* declaration is an admission of.
///
/// White is no hold at all, amber a half share, red the whole vertex. Two stops
/// rather than one, because the interesting range is the WEAK end: a chin held
/// 0.14 and a chin held 0.9 are different bodies, and a single white-to-red
/// lerp draws them as two shades of pink.
///
/// Read with the numbers [`report_jaw`] prints, never alone. A false-colour
/// overlay is an instrument, twenty-two of this crate's instruments have lied,
/// and this one has no way to show what it does not reach.
///
/// [`JAW_REACH`]: symbios_avatar::plan
fn jaw_tint(mesh: &PolyMesh, jaw: JawBone) -> Vec<Vec3> {
    const AMBER: Vec3 = Vec3::new(1.00, 0.62, 0.10);
    const RED: Vec3 = Vec3::new(0.85, 0.05, 0.10);
    mesh.skin
        .iter()
        .map(|influences| {
            let hold = held(influences, jaw.pivot);
            if hold <= 0.5 {
                Vec3::ONE.lerp(AMBER, hold * 2.0)
            } else {
                AMBER.lerp(RED, (hold - 0.5) * 2.0)
            }
        })
        .collect()
}

/// Grows the clump engine's hair on all five regions, replacing the shipped one.
///
/// **#201's judgement image, and it goes through the real hair mesh rather than
/// beside it** — same material, same light, same merge — because a new system
/// shown in a viewer of its own is a system nobody can compare to the one it
/// replaces.
///
/// The shapes and colours here are a REFERENCE rather than a catalogue: one
/// `Fall` per region at a length that suits it, and two colours a record will
/// carry once #202 lands. The styles of #204-#208 are what this becomes.
fn grow_clumps(avatar: &mut Avatar, record: &AvatarRecord) {
    let Some(skull) = Skull::measure(&avatar.parts.body, &avatar.rig) else {
        eprintln!("this body has no head to grow hair on");
        return;
    };
    let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
    let follicles = Follicles::of(&avatar.rig, &skull, &canon, &FollicleParams::default());
    let mut growth = Growth::default();
    let mut stream = Pcg64Mcg::seed_from_u64(record.seed as u64);
    // Roots, tips, clumps and length per region. Dark at the root and lighter
    // at the tip on every one of them, which is what hair does and what the
    // owner's two-colour model is for.
    let plan: [(Follicle, usize, f32, Vec3, Vec3); 5] = [
        (
            Follicle::Scalp,
            900,
            0.055,
            Vec3::new(0.09, 0.05, 0.03),
            Vec3::new(0.42, 0.26, 0.12),
        ),
        (
            Follicle::Brows,
            90,
            0.010,
            Vec3::new(0.07, 0.04, 0.02),
            Vec3::new(0.24, 0.15, 0.08),
        ),
        (
            Follicle::Moustache,
            110,
            0.012,
            Vec3::new(0.08, 0.05, 0.03),
            Vec3::new(0.34, 0.21, 0.10),
        ),
        (
            Follicle::Chin,
            220,
            0.024,
            Vec3::new(0.08, 0.05, 0.03),
            Vec3::new(0.34, 0.21, 0.10),
        ),
        (
            Follicle::Flanks,
            320,
            0.018,
            Vec3::new(0.08, 0.05, 0.03),
            Vec3::new(0.30, 0.19, 0.09),
        ),
    ];
    let bed = Bed {
        body: &avatar.parts.body,
        rig: &avatar.rig,
        weights: &avatar.parts.weights,
        follicles: &follicles,
    };
    for (follicle, count, length, roots, tips) in plan {
        growth.grow(
            &bed,
            &Sowing {
                follicle,
                count,
                shape: &Fall {
                    length,
                    ..Fall::default()
                },
                roots,
                tips,
            },
            &mut stream,
        );
    }
    println!("the clump engine grew:");
    for grown in &growth.grown {
        println!(
            "  {:10} {:4} clumps, {:6} triangles",
            grown.follicle.name(),
            grown.clumps,
            grown.tris
        );
    }
    println!(
        "  {:10} {:4} clumps, {:6} triangles",
        "all", growth.clumps(), growth.tris()
    );

    let head = skull.head;
    let mut placed = growth
        .mesh
        .transformed(Mat4::from_translation(avatar.rig.joints[head].position));
    placed.set_normals(placed.vertex_normals());
    placed.bind_rigidly(head as u16);
    match avatar
        .meshes
        .iter_mut()
        .find(|mesh| mesh.kind == MeshKind::Hair)
    {
        Some(hair) => hair.mesh = placed,
        None => avatar.meshes.push(AvatarMesh {
            kind: MeshKind::Hair,
            mesh: placed,
        }),
    }
}

/// Which hair region owns each vertex of the skin, as a colour to tint it by.
///
/// **The visual half of #199's pair**: `follicleaudit` says how much of the
/// head each region holds and how wide its edges are, and this says WHERE — the
/// one thing a table cannot, and the thing a mask is most often wrong about.
///
/// Tinted rather than substituted, for the reason [`scene::Item::tint`] gives:
/// a flat false colour throws away the shading that says what shape the surface
/// is, which on a head is most of how a boundary is judged. Each region's own
/// [`Follicle::colour`] is used, so this sheet and any other instrument showing
/// the five agree about which is which.
///
/// **The hue is the regions' weighted mean and the strength is their SUM, and
/// the first cut of this had it the other way round** (#199). Showing only the
/// strongest region drew the chin-to-flank seam as a pale wedge — each of the
/// two holds about 0.4 there while their sum is 0.8, so the one place both
/// layers will composite read as a bald patch in the very picture meant to rule
/// one out. Coverage is the question this instrument answers, so coverage is
/// what its strength has to mean; the seam shows as a blend of the two hues,
/// which is what a seam is.
fn follicle_tint(mesh: &PolyMesh, avatar: &Avatar, eyes: &EyeParams) -> Vec<Vec3> {
    let Some(follicles) = regions_of(avatar, eyes) else {
        return vec![Vec3::ONE; mesh.positions.len()];
    };
    mesh.positions
        .iter()
        .map(|point| {
            let weights = follicles.weights(*point - follicles.origin());
            let total: f32 = weights.iter().sum();
            if total <= f32::EPSILON {
                return Vec3::ONE;
            }
            let hue = Follicle::ALL
                .into_iter()
                .zip(weights)
                .fold(Vec3::ZERO, |sum, (follicle, weight)| {
                    sum + follicle.colour() * weight
                })
                / total;
            // Held back from the full colour so the shading underneath still
            // reads: a region at full weight is unmistakably its own hue, and
            // its fade is visibly a fade.
            Vec3::ONE.lerp(hue, total.min(1.0) * 0.85)
        })
        .collect()
}

/// The five follicle regions of a built body, measured the way the pipeline does.
///
/// **A renderer measuring its own head is a second implementation of the
/// recipe**, which this file's header is on record about — so this is the one
/// place it happens, shared by the false-colour overlay and the region close-up.
/// It answers `None` for a body with no head, which is a quadruped and not an
/// error.
fn regions_of(avatar: &Avatar, eyes: &EyeParams) -> Option<Follicles> {
    let skull = Skull::measure(&avatar.parts.body, &avatar.rig)?;
    let canon = Canon::measure(&avatar.rig, &skull, eyes);
    Some(Follicles::of(
        &avatar.rig,
        &skull,
        &canon,
        &FollicleParams::default(),
    ))
}

/// How strongly one joint holds one vertex.
///
/// Summed over the influence slots rather than searched for, because nothing
/// forbids a joint appearing twice and a search would silently report the
/// first.
///
/// The `+ 0.0` is not superstition. **Rust sums floats from an identity of
/// `-0.0`**, because `-0.0 + x == x` for every `x` including `+0.0` while
/// `+0.0 + -0.0` is `+0.0` — so a joint that appears in no slot at all comes
/// back as negative zero, `clamp` keeps it (`-0.0 < 0.0` is false), and the
/// table prints `-0.000` for "not held". That sign cost a real minute of
/// suspecting a negative weight, which is exactly the tax an instrument is
/// supposed to not charge.
fn held(influences: &[Influence; MAX_INFLUENCES], joint: usize) -> f32 {
    influences
        .iter()
        .filter(|influence| influence.joint as usize == joint)
        .map(|influence| influence.weight)
        .sum::<f32>()
        .clamp(0.0, 1.0)
        + 0.0
}

/// A band of the face, named by the two landmarks that bound it.
struct Band {
    what: &'static str,
    /// Its floor and ceiling in head-local metres.
    lo: f32,
    hi: f32,
    /// Whether it is confined to the front of the face and to the mouth's own
    /// width, rather than running right round the head.
    front: bool,
}

/// What the jaw overlay claims, as numbers beside it — and what a posed jaw
/// actually moved.
///
/// **Measured on `parts.body` against `parts.weights`**, which is the same data
/// the shipped binding tests read, so a disagreement between this and
/// `the_whole_face_turns_with_the_head` is a defect in one of them rather than
/// two tools measuring two things. The overlay is tinted from the merged skin
/// mesh, which additionally carries the lids; the vertex counts here are
/// therefore of the body alone.
///
/// The bands are bounded by the crate's own landmarks and nothing else — the
/// mouth line, the base of the nose, the chin, the throat and the eye line —
/// and where a band needs to be half of a span, it is the half between two of
/// them, which is the same construction `rig::skin::owner_of` puts its own
/// boundary on. No band is a bin of a continuum: each is the region between two
/// measured heights, and each prints its population so a filter that selects
/// nothing cannot pass for a result.
fn report_jaw(avatar: &Avatar, jaw: JawBone, pose: &Pose, opened: Option<f32>) {
    let rig = &avatar.rig;
    let body = &avatar.parts.body;
    let weights = &avatar.parts.weights;
    let Some(&head) = rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some(skull) = Skull::measure(body, rig) else {
        println!("this head has no skull to measure the jaw against");
        return;
    };
    // The eye axis is the only thing `Canon` reads its parameter for, and only
    // for the pupils' spacing, which nothing below asks about.
    let canon = Canon::measure(rig, &skull, &EyeParams::default());
    let centre = rig.joints[head].position;
    let pivot = rig.joints[jaw.pivot].position;
    let tip = rig.joints[jaw.tip].position;
    let reach = SkinConfig::default().reach;

    println!("\nthe mandible, by how strongly it holds the skin");
    println!("  the bone runs pivot -> tip and is owned by the PIVOT: rotating the pivot opens it");
    for (what, at, radius) in [
        ("pivot", pivot, rig.joints[jaw.pivot].radius),
        ("tip  ", tip, rig.joints[jaw.tip].radius),
    ] {
        let local = at - centre;
        println!(
            "  {what} at {:+6.1},{:+6.1},{:+6.1} mm from the head joint, marker radius {:5.1} mm, \
             falloff span {:5.1} mm",
            local.x * 1000.0,
            local.y * 1000.0,
            local.z * 1000.0,
            radius * 1000.0,
            radius * reach * 1000.0,
        );
    }
    println!(
        "  the bone is {:.1} mm long; the head's own radius is {:.1} mm",
        pivot.distance(tip) * 1000.0,
        rig.joints[head].radius * 1000.0,
    );
    // **The tip claims to sit ON the measured chin, and whether it does is a
    // parameter of the record rather than a fact.** Printed every run because
    // the constant is written in head RADII while the chin's height is a share
    // of the head's reach BELOW ITS JOINT, and `face_length` moves the second
    // without moving the first.
    println!(
        "  the tip sits {:+.1} mm from the measured chin, which is at y {:+.1} mm (positive is BELOW it)",
        (skull.chin() - (tip.y - centre.y)) * 1000.0,
        skull.chin() * 1000.0,
    );
    println!(
        "  and {:+.1} mm behind the chin's own projection, which reaches z {:+.1} mm",
        (skull.depth(skull.chin()) - (tip.z - centre.z)) * 1000.0,
        skull.depth(skull.chin()) * 1000.0,
    );

    // The territory, before any band: every vertex the bone reaches at all.
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    let (mut reached, mut dominant, mut strongest) = (0usize, 0usize, 0.0f32);
    for (vertex, &rest) in body.positions.iter().enumerate() {
        let hold = held(&weights.vertices[vertex], jaw.pivot);
        if hold <= 0.01 {
            continue;
        }
        reached += 1;
        strongest = strongest.max(hold);
        if weights.dominant(vertex) as usize == jaw.pivot {
            dominant += 1;
        }
        lo = lo.min(rest - centre);
        hi = hi.max(rest - centre);
    }
    println!(
        "  held at all (over 0.01): {reached:5} of {} body vertices, {dominant} of them dominantly, \
         strongest {strongest:.3}",
        body.positions.len(),
    );
    if reached > 0 {
        println!(
            "  its territory spans x {:+6.1} to {:+6.1}, y {:+6.1} to {:+6.1}, z {:+6.1} to {:+6.1} mm \
             about the head joint",
            lo.x * 1000.0,
            hi.x * 1000.0,
            lo.y * 1000.0,
            hi.y * 1000.0,
            lo.z * 1000.0,
            hi.z * 1000.0,
        );
    }

    // **A marker is rig-only to the CAGE, not to everything.** `nearest_bone`
    // asks "what part of the body is under this point" and answers it over the
    // deforming joints — which a marker has to be, or nothing could bind to it
    // — so the mandible competes for queries its own docstring says a face rig
    // must never win. Every skull, relief and scalp call site reads only the
    // hit's ZONE, which is `Head` either way and so cannot move; `paint_skin`
    // and `rig::Surface` read its RADIUS, which the jaw's reach does move.
    let (mut stolen, mut lowest, mut highest) = (0usize, f32::MAX, f32::MIN);
    for &point in &body.positions {
        let joint = rig.nearest_bone(point).joint;
        if joint == jaw.pivot || joint == jaw.tip {
            stolen += 1;
            lowest = lowest.min(point.y - centre.y);
            highest = highest.max(point.y - centre.y);
        }
    }
    println!(
        "  the marker bones win nearest_bone for {stolen} of {} body vertices, spanning y {:+.1} to \
         {:+.1} mm — zone-only readers cannot see it, paint_skin and rig::Surface can",
        body.positions.len(),
        lowest * 1000.0,
        highest * 1000.0,
    );

    let bands = face_bands(&skull, &canon);
    println!(
        "  bands, in head-local mm: upper lip {:.1} to {:.1}, lower lip {:.1} to {:.1}, chin {:.1} to {:.1}, \
         throat at {:.1}",
        bands[0].lo * 1000.0,
        bands[0].hi * 1000.0,
        bands[1].lo * 1000.0,
        bands[1].hi * 1000.0,
        bands[2].lo * 1000.0,
        bands[2].hi * 1000.0,
        skull.throat_and_crown().0 * 1000.0,
    );
    println!("               verts   jaw: least   mean    most    head: mean");

    // A posed jaw is the only thing that can show whether the binding works:
    // dual quaternion blending deforms a bad reach and a good one identically
    // at rest and under a head turn, which is why nothing has caught this.
    let moved = opened.map(|_| pose.forward(rig).deform(rig, &body.positions, weights));
    let field = Field {
        body,
        centre,
        unit: canon.unit,
        jaw: jaw.pivot,
        head,
        pivot,
        arc: opened.map_or(0.0, |degrees| 2.0 * (degrees.to_radians() * 0.5).sin()),
    };

    for band in &bands {
        let Some(read) = field.tally(band, weights, moved.as_deref()) else {
            println!("  {} : NO VERTICES — the band, not the binding", band.what);
            continue;
        };
        print!(
            "  {} {:5}      {:.3}  {:.3}  {:.3}        {:.3}",
            band.what, read.count, read.least, read.mean, read.most, read.head,
        );
        if opened.is_some() {
            print!(
                "   moved {:6.2} mm mean, {:6.2} worst, {:5.1}% of a rigid mandible's arc",
                read.travel * 1000.0,
                read.worst * 1000.0,
                read.share * 100.0,
            );
        }
        println!();
    }

    cross_check_jaw(avatar, jaw, head);
    if let Some(degrees) = opened {
        println!("  opened {degrees:.0} degrees about the lateral axis at the pivot");
    } else {
        println!("  nothing is posed: run with --jaw 20 to see whether any of this articulates");
    }
}

/// The bands of a face, bounded by the landmarks the crate itself measures.
///
/// Where a band needs half of a span it is the half between two landmarks,
/// which is the construction `rig::skin::owner_of` puts its own boundary on.
fn face_bands(skull: &Skull, canon: &Canon) -> [Band; 6] {
    let chin = skull.chin();
    let mouth = canon.mouth_line();
    let nose = canon.nose_base();
    let (throat, _) = skull.throat_and_crown();
    [
        Band {
            what: "the upper lip",
            lo: mouth,
            hi: mouth + (nose - mouth) * 0.5,
            front: true,
        },
        Band {
            what: "the lower lip",
            lo: chin + (mouth - chin) * 0.5,
            hi: mouth,
            front: true,
        },
        Band {
            what: "the chin     ",
            lo: chin,
            hi: chin + (mouth - chin) * 0.5,
            front: true,
        },
        Band {
            what: "under the jaw",
            lo: throat,
            hi: chin,
            front: true,
        },
        Band {
            what: "the cranium  ",
            lo: canon.level,
            hi: f32::MAX,
            front: false,
        },
        Band {
            what: "the neck     ",
            lo: f32::MIN,
            hi: throat,
            front: false,
        },
    ]
}

/// Everything a band tally needs that does not change between bands.
struct Field<'a> {
    body: &'a PolyMesh,
    /// The head joint: every band bound is head-local.
    centre: Vec3,
    /// One eye width, which is the ruler a face's widths are counted in.
    unit: f32,
    /// The joint that owns the mandible — the PIVOT, not the tip.
    jaw: usize,
    head: usize,
    /// The hinge's position, for the arc a rigid mandible would carry a vertex
    /// through.
    pivot: Vec3,
    /// Chord over radius at the opened angle, `2 sin(θ/2)`, or zero at rest.
    arc: f32,
}

/// What one band reads, at one binding.
struct Tally {
    count: usize,
    /// The jaw's hold over the band: least, mean, most.
    least: f32,
    mean: f32,
    most: f32,
    /// The head's mean hold, which is the jaw's rival everywhere that matters.
    head: f32,
    /// Mean and worst travel under the pose, in METRES — both, so a "worst"
    /// smaller than its own mean cannot pass for a result. It already did once
    /// here, and printing one of them scaled and the other raw is how.
    travel: f32,
    worst: f32,
    /// Travel as a share of the arc a rigid mandible would have carried the
    /// band through. 1.0 is a jaw; 0.0 is a skull.
    share: f32,
}

impl Field<'_> {
    /// Measures one band against one binding, or `None` if the band is empty.
    ///
    /// A band that selects nothing passes every assertion after it, which is
    /// the shape of half the instrument failures this crate has found — so an
    /// empty band is not a zero, it is an absence, and it says so.
    fn tally(&self, band: &Band, weights: &SkinWeights, moved: Option<&[Vec3]>) -> Option<Tally> {
        let mut count = 0usize;
        let (mut least, mut most, mut sum, mut head_sum) = (f32::MAX, 0.0f32, 0.0f32, 0.0f32);
        let (mut travel, mut worst, mut rigid) = (0.0f32, 0.0f32, 0.0f32);
        for (vertex, &rest) in self.body.positions.iter().enumerate() {
            let local = rest - self.centre;
            if local.y < band.lo || local.y >= band.hi {
                continue;
            }
            if band.front && (local.z <= 0.0 || local.x.abs() > self.unit) {
                continue;
            }
            count += 1;
            let hold = held(&weights.vertices[vertex], self.jaw);
            least = least.min(hold);
            most = most.max(hold);
            sum += hold;
            head_sum += held(&weights.vertices[vertex], self.head);
            if let Some(moved) = moved {
                let went = (moved[vertex] - rest).length();
                travel += went;
                worst = worst.max(went);
                rigid += rest.distance(self.pivot) * self.arc;
            }
        }
        if count == 0 {
            return None;
        }
        let n = count as f32;
        Some(Tally {
            count,
            least,
            mean: sum / n,
            most,
            head: head_sum / n,
            travel: travel / n,
            worst,
            share: if rigid > 0.0 { travel / rigid } else { 0.0 },
        })
    }
}

/// Sweeps the mandible's binding reach against the face it has to move.
///
/// **`JAW_REACH` shipped declared unsourced and this is what sources it.** The
/// two marker radii are the only thing that decides which skin the jaw bone
/// catches, and the plan feeds them nowhere else — `Rig::from_skeleton` copies
/// them straight onto the joints and the cage never sees a marker at all — so
/// setting them on a cloned rig and re-running the crate's own `skin::bind` is
/// exactly what editing the constant would do, to the last bit, without
/// rebuilding a body per candidate. The mesh cannot change: markers mesh
/// nothing.
///
/// What the sweep is looking for, and it is a pair of opposed requirements:
/// the chin has to FOLLOW the jaw and the upper lip has to STAY with the head,
/// and both are read at a real 20-degree open rather than off the weights,
/// because travel is the thing anybody looks at.
fn sweep_jaw(avatar: &Avatar, jaw: JawBone, degrees: f32) {
    let body = &avatar.parts.body;
    let Some(&head) = avatar.rig.in_zone(Zone::Head).first() else {
        return;
    };
    let Some(skull) = Skull::measure(body, &avatar.rig) else {
        return;
    };
    let canon = Canon::measure(&avatar.rig, &skull, &EyeParams::default());
    let bands = face_bands(&skull, &canon);
    let radius = avatar.rig.joints[head].radius;
    let config = SkinConfig::default();

    println!(
        "\nsweeping the mandible's binding reach, in head radii, at a {degrees:.0} degree open"
    );
    println!("  the chin must follow the jaw; the upper lip must stay with the head");
    println!(
        "  pivot   tip |  upper lip: hold  most  moved |  chin: hold  moved   arc |  lower lip moved | \
         under jaw hold | separation"
    );
    for pivot_reach in [0.06f32, 0.10, 0.14, 0.18] {
        for tip_reach in [0.20f32, 0.23, 0.26, 0.29] {
            let mut rig = avatar.rig.clone();
            rig.joints[jaw.pivot].radius = radius * pivot_reach;
            rig.joints[jaw.tip].radius = radius * tip_reach;
            let weights = symbios_avatar::rig::skin::bind(body, &rig, &config);

            let mut pose = Pose::rest(&rig);
            pose.rotations[jaw.pivot] = Quat::from_rotation_x(degrees.to_radians());
            let moved = pose.forward(&rig).deform(&rig, &body.positions, &weights);
            let field = Field {
                body,
                centre: rig.joints[head].position,
                unit: canon.unit,
                jaw: jaw.pivot,
                head,
                pivot: rig.joints[jaw.pivot].position,
                arc: 2.0 * (degrees.to_radians() * 0.5).sin(),
            };
            let read = |band: &Band| field.tally(band, &weights, Some(&moved));
            let (Some(upper), Some(lower), Some(chin), Some(under)) = (
                read(&bands[0]),
                read(&bands[1]),
                read(&bands[2]),
                read(&bands[3]),
            ) else {
                continue;
            };
            println!(
                "  {pivot_reach:5.2} {tip_reach:5.2} |        {:.3} {:.3} {:6.2} mm |     {:.3} {:6.2} mm \
                 {:4.0}% | {:9.2} mm |          {:.3} | {:6.2} mm",
                upper.mean,
                upper.most,
                upper.travel * 1000.0,
                chin.mean,
                chin.travel * 1000.0,
                chin.share * 100.0,
                lower.travel * 1000.0,
                under.mean,
                (lower.travel - upper.travel) * 1000.0,
            );
        }
    }
}

/// The one number in [`report_jaw`] that a shipped assertion also computes.
///
/// **A new instrument is checked against an existing one before it is
/// believed.** `rig::skin::the_throat_stays_with_the_neck` finds the throat by
/// exactly this construction and asserts the neck outholds the head there; if
/// the numbers printed here do not agree with that verdict, this tool is the
/// thing that is wrong. The jaw's own hold is printed beside them because the
/// throat is where a raised reach would first tear.
fn cross_check_jaw(avatar: &Avatar, jaw: JawBone, head: usize) {
    let rig = &avatar.rig;
    let body = &avatar.parts.body;
    let Some(neck) = rig.joints[head].parent else {
        return;
    };
    let centre = rig.joints[head].position;
    let floor = body
        .positions
        .iter()
        .filter(|p| rig.joints[rig.nearest_bone(**p).joint].zone == Zone::Head)
        .fold(f32::MAX, |low, p| low.min(p.y));
    let throat = body
        .positions
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            (p.y - floor).abs() < rig.joints[head].radius * 0.03
                && (p.x - centre.x).abs() < rig.joints[head].radius * 0.05
        })
        .max_by(|a, b| a.1.z.total_cmp(&b.1.z));
    let Some((throat, _)) = throat else {
        return;
    };
    let influences = &avatar.parts.weights.vertices[throat];
    println!(
        "  cross-check, the_throat_stays_with_the_neck's own throat vertex: head {:.2}, neck {:.2}, \
         jaw {:.2} — the test passes iff neck outholds head",
        held(influences, head),
        held(influences, neck),
        held(influences, jaw.pivot),
    );
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
