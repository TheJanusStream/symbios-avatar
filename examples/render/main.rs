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
//! cargo run --release --example render -- --close throat # the collar and the trapezius line
//! cargo run --release --example render -- --close brows # or any follicle region
//! cargo run --release --example render -- --brow thick  # or natural, sculpted, none
//! cargo run --release --example render -- --scalp bob 0.8 # crop, bob, long, tied, curly,
//!                                                       # cap, slick, bell, bun, crest, afro, braids
//! cargo run --release --example render -- --moustache handlebar 0.9 # chevron, handlebar, pencil, sculpted
//! cargo run --release --example render -- --chin braided 0.8 # goatee, full, braided, sculpted
//! cargo run --release --example render -- --flanks full 0.7 # sideburns, full, sculpted
//! cargo run --release --example render -- --close hand --fist  # every finger curled
//! cargo run --release --example render -- --gaze 40  # look this many degrees to one side
//! cargo run --release --example render -- --bare      # no hair or clothes, to see the body
//! cargo run --release --example render -- --bare --chest 1,-1  # the chest axes; needs --bare
//! cargo run --release --example render -- --leg shorts --sleeve bare # the outfit's cuts
//! cargo run --release --example render -- --leg 0.6 --sleeve 0.3  # or any length: 0.5 is the knee, the elbow
//! cargo run --release --example render -- --top-colour 0.9,0.2,0.2 --trousers-colour 0.1,0.1,0.3
//! cargo run --release --example render -- --top-texture Fabric,6 --trousers-texture Truchet,3,45 # surface,tiles/m,degrees; none drops one
//! cargo run --release --example render -- --junction  # tint the skin by which bone deforms it
//! cargo run --release --example render -- --jawbind   # tint the skin by how the JAW bone holds it
//! cargo run --release --example render -- --follicles # tint the skin by where hair may grow
//! cargo run --release --example render -- --clumps    # grow the new clump engine on all five regions
//! cargo run --release --example render -- --strand-mask # the image every hair card is cut out of
//! cargo run --release --example render -- --jaw 20    # open the mouth this many degrees
//! cargo run --release --example render -- --brows 8   # raise the brows this many degrees
//! cargo run --release --example render -- --corners 12 # smile this many degrees (negative frowns)
//! cargo run --release --example render -- --expression happy # or sad, angry, surprised, neutral
//! cargo run --release --example render -- --viseme aa # an Oculus-15 mouth shape; try SS, oh, ou
//! cargo run --release --example render -- --quadruped --upper-legs # how the legs' outsides sit over the trunk; --at C mid-stride, --gesture NAME --at T a pose
//! cargo run --release --example render -- --jawsweep # tune the jaw's binding reach by measurement
//! cargo run --release --example render -- --clip Punch_Cross            # a CC0 clip, retargeted
//! cargo run --release --example render -- --clip Wave --clipframes 12   # more frames of it
//! cargo run --release --example render -- --linear     # matrix skinning, to compare
//! cargo run --release --example render -- --skin 0.9,-1,0.4,0  # melanin,undertone,blush,freckles
//! cargo run --release --example render -- --eyes 0.1,0.15,0.19,0.24,0.38,0.46,0.24,0.38,0.46  # ring,inner,outer rgb
//! cargo run --release --example render -- --hair 1 0.5 1 0.5   # scalp length,thickness,density,droop
//! cargo run --release --example render -- --hair 0 0 0 0 0     # a fifth zero shaves every region
//! cargo run --release --example render -- --face 1,0.5,1,1,0.5,0.5 # nose,noseWidth,brow,mouth,mouthWidth,ears
//! cargo run --release --example render -- --skull -1,1            # headBreadth,faceLength
//! cargo run --release --example render -- --femininity 1  # the frame axis, -1 .. +1
//! cargo run --release --example render -- --head-size -1  # the head's own size axis
//! cargo run --release --example render -- --mass 1 --fat 0.10  # heavy and lean: muscular
//! cargo run --release --example render -- --age 80  # the age axis, 18 .. 80 years
//! cargo run --release --example render -- --pass ao   # or normal, albedo, shadow, roughness
//! cargo run --release --example render -- --quadruped
//! cargo run --release --example render -- --quadruped --gallery 16   # 16 rolls side by side
//! cargo run --release --example render -- --quadruped --gallery 16 --pass silhouette
//! cargo run --release --example render -- --quadruped --gallery 16 --to-scale  # sizes compared
//! cargo run --release --example render -- --quadruped --gallery 16 --family cat  # or dog, horse, cross
//! cargo run --release --example render -- --quadruped --gallery 9 --pattern rosettes  # coats marked so
//! cargo run --release --example render -- --quadruped --gallery 9 --fantasy  # unnatural coats only
//! cargo run --release --example render -- --quadruped --seed 6 --pattern brindle --points  # any coat on any family
//! cargo run --release --example render -- --quadruped --speed 2.5 --strip 8  # a gait at a speed, one row
//! cargo run --release --example render -- --quadruped --gait canter --speed 3 --strip 8
//! cargo run --release --example render -- --quadruped --gesture "Play Bow" --strip 8
//! cargo run --release --example render -- --quadruped --seed 1 --gesture Sit --rise --strip 8  # getting up from it
//! cargo run --release --example render -- --quadruped --gesture Sit --interrupt 0.5 --strip 8  # told to get up part way down
//! cargo run --release --example render -- --quadruped --seed 1 --gesture "Lie Down" --at 1  # one moment, four views
//! cargo run --release --example render -- --quadruped --kin horse --drive 0.3 --speed 1.5 --strip 8  # driven up a slope
//! cargo run --release --example render -- --quadruped --kin horse --drive 0 --speed 1.7 --then 3.7 --from 156 --strip 14  # breaking into a trot, frame by frame
//! cargo run --release --example render -- --quadruped --kin horse --drive 0 --drop 0.6 --from 134 --strip 14  # off a ledge and landing
//! cargo run --release --example render -- --quadruped --kin horse --drive 0 --jump 2.5 --from 146 --strip 14  # a jump's landing
//! cargo run --release --example render -- --quadruped --swim 0.5 --strip 8
//! cargo run --release --example render -- --quadruped --leap 3 --strip 8
//! cargo run --release --example render -- --winged                 # a winged body (#420): an owl by default
//! cargo run --release --example render -- --winged --kin bat        # or glider, or a cross: --kin bird+bat (bird+bat@0.3 at a blend)
//! cargo run --release --example render -- --winged --gallery 9 --family glider --pass silhouette  # or bird, bat, cross
//! cargo run --release --example render -- --winged --kin bat --wings  # its wings folded, half and spread, and one close (#421)
//! cargo run --release --example render -- --winged --head --beak 1,-1,-1  # a bird's head (#422): beakLength,beakHook,facialDisc, a crow's
//! cargo run --release --example render -- --wings-crosses             # every cross, folded and spread
//! cargo run --release --example render -- --winged --close foot --beak 1,-1,-1  # a crow's beak: its toes anisodactyl (#422)
//! cargo run --release --example render -- --winged --beak 1,-1,-1 --plumage crow  # a crow's plumage (#423): or owl, parrot, songbird, beak (the head's nearest)
//! cargo run --release --example render -- --head --far 12  # at the pixels a peer 12 m off gets
//! cargo run --release --example render -- --tier far  # the FAR tier's hair in the hair's place
//! cargo run --release --example render -- --tier near # built with a far tier, drawn near (the control)
//! cargo run --release --example render -- --budget    # what one avatar costs
//! cargo run --release --example render -- --cost      # and what it costs to build
//! ```

mod light;
mod scene;

use glam::{Mat4, Quat, Vec3};
use light::Image;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;
use scene::{Cutout, Frame, GBuffer, Item, Material, Paint, ShadowMap};
use symbios_avatar::plan::{Blended, Kin, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarMesh, AvatarRecord, Blink, Canon, Category, Expression,
    EyeParams, FaceParams, FootingConfig, Gait, GazeConfig, Ground, Influence, Limb,
    MAX_INFLUENCES, MeshKind, PolyMesh, Pose, Rig, Role, Skeleton, SkinConfig, SkinParams,
    SkinWeights, Stride, Viseme, Walk, Zone,
    anim::contacts_in,
    anim::gaze,
    anim::plant_feet_of,
    face::Skull,
    gltf::Gltf,
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
/// How far the game's third-person camera looks down on an avatar it follows
/// from behind, about: what `--nape` judges a head from (#388).
const GAME_PITCH: f32 = 0.35;
/// Pixels one metre covers one metre from a camera, for `--far`: 1080 lines
/// over a 60-degree vertical field, `1080 / (2 tan 30)`.
///
/// **What a peer across a room is drawn at** (#350). A view framed for a
/// close-up covers its span in [`VIEW`] pixels; a head `d` metres off a
/// 1080-line camera covers the same span in `span * FAR_PIXELS / d`, which at
/// 12 m is about thirty.
const FAR_PIXELS: f32 = 935.3;

/// Which part of the body a close-up frames.
#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Head,
    Hand,
    Foot,
    /// The crotch, framed from under it.
    ///
    /// **The one view a contact sheet cannot give.** Front, side, back and
    /// three-quarter all look at the outside of a leg, and the notch between
    /// the thighs is behind that from every one of them. #279's defect — the
    /// cloth of a garment folding INSIDE the body at the crease — lived there
    /// undetected because nothing had ever looked at it, and a fix for it that
    /// is judged by a body sheet is not judged at all.
    Crotch,
    /// The chest, framed on the ribcage and the top of the abdomen.
    ///
    /// **A chest cannot be judged on the body sheet and milestone #9 needs it
    /// judged.** On the four-view sheet a bust is about forty pixels tall, and
    /// #286's whole subject — where the volume sits above and below its peak,
    /// and whether there is a crease under it — is a matter of the SIDE
    /// profile at a scale that sheet does not have. The instrument said the
    /// neutral body's pole split went from 55:45 to 45:55 and the sheet showed
    /// nothing whatsoever, which is the crotch's own argument (#279) arriving a
    /// second time: a fix judged by a sheet that cannot resolve it is not
    /// judged.
    ///
    /// The top of the abdomen comes along because the inframammary fold seats
    /// itself BELOW the lobe, and a chest framed on the ribcage alone crops at
    /// exactly the crease under scrutiny — which is the trap the crotch view
    /// documents, in the other direction.
    Chest,
    /// The throat, framed on the neck column, the chin above it and the top
    /// of the chest under it.
    ///
    /// **The head close-up catches the collar only marginally** (#301): it is
    /// framed on head plus neck, so the shoulder line where the column meets
    /// the girdle sits in the bottom margin of the frame and the trapezius is
    /// out of shot altogether. Milestone #10's throat work is the collar seam
    /// and the trapezius, both of which live at exactly that edge — the
    /// chest's #287 argument a third time. The lower part of the head comes
    /// along so the chin-to-throat profile is in shot, and the upper half of
    /// the chest so the shoulder slope is.
    Throat,
    /// One follicle region, framed on the mask itself.
    ///
    /// **A head shot is not close enough to judge a brow**. A brow is
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
    run();
}

fn run() {
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
        Some("crotch") => Some(Focus::Crotch),
        Some("chest") => Some(Focus::Chest),
        Some("throat") => Some(Focus::Throat),
        Some(other) => match Follicle::ALL.into_iter().find(|it| it.name() == other) {
            Some(follicle) => Some(Focus::Region(follicle)),
            None => {
                eprintln!(
                    "unknown --close target {other}: expected head, hand, foot, crotch, chest, throat or \
                     one of {}",
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
    // The other hand or foot of the pair for a close-up: a one-handed clip
    // lands on one side, and the framing has to follow it.
    let other = args.iter().any(|arg| arg == "--other");
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
    // Which hair tier to draw (#350): `near` builds the far tier too and draws
    // the near one - the control that asking for a far tier moves no byte of
    // the near - and `far` draws the far tier in the hair's place.
    let tier = match value("--tier").map(String::as_str) {
        None => None,
        Some("near") => Some(false),
        Some("far") => Some(true),
        Some(other) => {
            eprintln!("unknown --tier {other}: expected near or far");
            std::process::exit(1);
        }
    };
    // The framing a peer twelve metres off gets (#350). See [`FAR_PIXELS`].
    let far = value("--far")
        .and_then(|metres| metres.parse::<f32>().ok())
        .filter(|metres| *metres > 0.0);
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
        // The sculpted facial family (#349): a solid wedge along the ridge.
        "sculpted" => symbios_avatar::hair::BrowStyle::Sculpted,
        other => {
            eprintln!("unknown --brow style {other}: expected none, natural, thick or sculpted");
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
            // The helmet family (#346): shells rather than cards, each with one
            // axis of its own like every other entry in the catalogue.
            "cap" => symbios_avatar::hair::ScalpStyle::Cap { fringe: axis },
            "slick" => symbios_avatar::hair::ScalpStyle::SlickBack { volume: axis },
            "bell" => symbios_avatar::hair::ScalpStyle::Bell { length: axis },
            // A shell PLUS an appendage (#347): a closed ball at the back, and
            // a shaved head with a fin along its midline.
            "bun" => symbios_avatar::hair::ScalpStyle::Bun { height: axis },
            "crest" => symbios_avatar::hair::ScalpStyle::Crest { height: axis },
            // A volume shell and a ridged one (#348): one round mass with its
            // rim rolled in, and cornrows.
            "afro" => symbios_avatar::hair::ScalpStyle::Afro { size: axis },
            "braids" => symbios_avatar::hair::ScalpStyle::Braids { rows: axis },
            other => {
                eprintln!(
                    "unknown --scalp style {other}: expected none, crop, bob, long, tied, curly, \
                     cap, slick, bell, bun, crest, afro or braids"
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
            // A solid chevron flaring to a handlebar (#349).
            "sculpted" => symbios_avatar::hair::MoustacheStyle::Sculpted { flare: axis },
            other => {
                eprintln!(
                    "unknown --moustache style {other}: expected none, chevron, handlebar, \
                     pencil or sculpted"
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
            // A closed mass hanging from the menton (#349).
            "sculpted" => symbios_avatar::hair::ChinStyle::Sculpted { length: axis },
            other => {
                eprintln!(
                    "unknown --chin style {other}: expected none, goatee, full, braided or sculpted"
                );
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
            // A closed band from the beard line to the crease (#349).
            "sculpted" => symbios_avatar::hair::FlankStyle::Sculpted,
            other => {
                eprintln!(
                    "unknown --flanks style {other}: expected none, sideburns, full or sculpted"
                );
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
    let roots: Vec<f32> = value("--roots")
        .map(|spec| {
            spec.split(',')
                .filter_map(|a| a.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();

    // Four numbers, in the order the axes are declared: melanin, undertone,
    // blush, freckles. It was five and the fifth was stubble, which stopped
    // being a complexion axis at #212 — the painted hair layer draws it now,
    // per region, and `--moustache`/`--chin`/`--flanks` are how it is asked
    // for. A complexion cannot be judged from its numbers,
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

    // The strand mask itself (#340): the one image the library ships for every
    // head of hair, written out to be looked at rather than inferred from the
    // heads it cuts. Its alpha as grey on black, each texel a 3x3 block.
    if args.iter().any(|arg| arg == "--strand-mask") {
        let mask = symbios_avatar::strand_mask();
        let side = mask.side * 3;
        let picture = image::RgbaImage::from_fn(side, side, |x, y| {
            let alpha = mask.rgba[((y / 3 * mask.side + x / 3) * 4 + 3) as usize];
            image::Rgba([alpha, alpha, alpha, 255])
        });
        let path = out.join("strand_mask.png");
        match picture.save(&path) {
            Ok(()) => println!("wrote the strand mask to {}", path.display()),
            Err(error) => eprintln!("cannot write {}: {error}", path.display()),
        }
        return;
    }

    // Both archetypes, because everything on a body is shared between them and
    // only one of them has ever been looked at.
    let archetype = if args.iter().any(|arg| arg == "--quadruped") {
        Archetype::Quadruped(symbios_avatar::QuadrupedParams::default())
    } else if args.iter().any(|arg| arg == "--winged") {
        Archetype::Winged(symbios_avatar::WingedParams::default())
    } else {
        Archetype::default()
    };
    // `--walls N`: where the backdrop stands across N rolls, and whether it
    // still stands behind anything a frame draws (#379).
    if let Some(count) = value("--walls").and_then(|v| v.parse::<i64>().ok()) {
        walls(&archetype, count);
        return;
    }
    let mut record = AvatarRecord::new("Rendered", archetype);
    if let Some(seed) = seed {
        record.reroll(seed);
    }
    // Which beast a head is judged on (#388): `--kin horse` turns the roll
    // into a family (`--kin dog+cat` a cross of the two), `--head-axis 1.5` and
    // `--appeal 2` are the big-headed and small-headed ends of the roll.
    // The kin is the plan's own families, through the machinery every plan
    // with families shares (#419): a new plan is one more arm here.
    if let Some(kin) = value("--kin") {
        match &mut record.archetype {
            Archetype::Quadruped(params) => set_kin(params, kin),
            Archetype::Winged(params) => {
                set_kin(params, kin);
                // An unrolled winged body's eyes are its family's (#423), which
                // a record is given when it is made: made a bird, then turned
                // into a bat here, it would look out of an owl's.
                if seed.is_none() {
                    record.eyes = AvatarRecord::new("Kin", Archetype::Winged(*params)).eyes;
                }
            }
            _ => {
                eprintln!(
                    "--kin picks a family, and this plan has none: add --quadruped or --winged"
                );
                std::process::exit(2);
            }
        }
    }
    if let Archetype::Quadruped(params) = &mut record.archetype {
        if let Some(size) = value("--head-axis").and_then(|v| v.parse::<f32>().ok()) {
            params.head_size = size;
        }
        if let Some(appeal) = value("--appeal").and_then(|v| v.parse::<f32>().ok()) {
            params.appeal = appeal;
        }
    }
    if let Archetype::Winged(params) = &mut record.archetype {
        if let Some(size) = value("--head-axis").and_then(|v| v.parse::<f32>().ok()) {
            params.head_size = size;
        }
        if let Some(appeal) = value("--appeal").and_then(|v| v.parse::<f32>().ok()) {
            params.appeal = appeal;
        }
        // A bird's head (#422): `--beak 1,-1,-1` is beakLength, beakHook and
        // facialDisc - a crow's.
        if let Some(spec) = value("--beak") {
            let axes: Vec<f32> = spec
                .split(',')
                .filter_map(|a| a.trim().parse().ok())
                .collect();
            let [length, hook, disc] = axes[..] else {
                eprintln!("--beak takes beakLength,beakHook,facialDisc, not `{spec}`");
                std::process::exit(2);
            };
            (params.beak_length, params.beak_hook, params.facial_disc) = (length, hook, disc);
        }
        // A bird's plumage (#423): `--plumage crow` dresses it in a kind's
        // preset (owl, crow, parrot, songbird), `--plumage beak` in the one its
        // head's axes stand nearest.
        if let Some(kind) = value("--plumage") {
            use symbios_avatar::Plumage;
            params.plumage = match kind.as_str() {
                "owl" => Plumage::OWL,
                "crow" => Plumage::CROW,
                "parrot" => Plumage::PARROT,
                "songbird" => Plumage::SONGBIRD,
                "beak" => params.plumage_for_head(),
                _ => {
                    eprintln!("--plumage takes owl, crow, parrot, songbird or beak, not `{kind}`");
                    std::process::exit(2);
                }
            };
        }
    }
    // An unrolled bird's eyes follow the head `--beak` gave it (#423), as a
    // record made with that head would have them: a crow's dark, a parrot's
    // pale, where it was made an owl and would look out of an owl's gold.
    if seed.is_none()
        && value("--beak").is_some()
        && let Archetype::Winged(params) = &record.archetype
    {
        record.eyes = AvatarRecord::new("Beak", Archetype::Winged(*params)).eyes;
    }
    // One exact record, as its JSON is saved (#395): `--record cat.json`, for the
    // body someone is wearing rather than a roll that looks like it.
    if let Some(path) = value("--record") {
        let text = std::fs::read_to_string(path).expect("the record's file reads");
        record = serde_json::from_str(&text).expect("the file holds a record");
    }
    // A mane's length, whatever the roll grew (#386): `--fur-mane 1.5`.
    if let Some(length) = value("--fur-mane").and_then(|v| v.parse::<f32>().ok()) {
        match &mut record.archetype {
            Archetype::Quadruped(params) => params.fur.mane = length,
            _ => {
                eprintln!("--fur-mane grows a quadruped's mane: add --quadruped");
                std::process::exit(2);
            }
        }
    }
    // A mane's stiffness, which no roll draws (#389): `--fur-stiffness 0.8`.
    let stiffness = value("--fur-stiffness").and_then(|v| v.parse::<f32>().ok());
    if let Some(stiffness) = stiffness {
        match &mut record.archetype {
            Archetype::Quadruped(params) => params.fur.mane_stiffness = stiffness,
            _ => {
                eprintln!("--fur-stiffness stiffens a quadruped's mane: add --quadruped");
                std::process::exit(2);
            }
        }
    }
    // A horn, which no roll gives (#367): `--unicorn` puts one on a beast.
    if args.iter().any(|arg| arg == "--unicorn") {
        match &mut record.archetype {
            Archetype::Quadruped(params) => {
                params.ornaments.horn = Some(symbios_avatar::Ornament::UNICORN);
            }
            _ => {
                eprintln!("--unicorn horns a quadruped: add --quadruped");
                std::process::exit(2);
            }
        }
    }

    // Antlers, which few rolls give (#407 looks at a mane against them).
    if args.iter().any(|arg| arg == "--antlers") {
        match &mut record.archetype {
            Archetype::Quadruped(params) => {
                params
                    .ornaments
                    .wear(symbios_avatar::OrnamentKind::Antlers, true);
            }
            _ => {
                eprintln!("--antlers crowns a quadruped: add --quadruped");
                std::process::exit(2);
            }
        }
    }

    // A coat pattern, or a colourpoint, that a family never rolls (#376):
    // `--pattern NAME` marks one beast's coat so - in a gallery it filters
    // instead - and `--points` gives it a colourpoint, so every pattern is
    // judged on every family.
    if args.iter().any(|arg| arg == "--points") || value("--pattern").is_some() {
        let Archetype::Quadruped(params) = &mut record.archetype else {
            eprintln!("--pattern and --points mark a quadruped's coat: add --quadruped");
            std::process::exit(2);
        };
        if let Some(name) = value("--pattern") {
            params.coat.pattern = symbios_avatar::plan::Pattern::ALL
                .into_iter()
                .find(|pattern| pattern.name() == name.as_str())
                .unwrap_or_else(|| {
                    eprintln!("--pattern takes a coat pattern's name, not `{name}`");
                    std::process::exit(2);
                });
        }
        if args.iter().any(|arg| arg == "--points") {
            params.coat.points = true;
        }
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
    // The head's own size axis, which is the one the neck-to-skull ratio is
    // worst at: `face::neck`'s whole subject is a small head on a heavy body,
    // and a throat judged without that corner in reach is not judged (#302).
    if let Some(spec) = value("--head-size")
        && let Ok(head_size) = spec.parse::<f32>()
        && let Archetype::Humanoid(params) = &mut record.archetype
    {
        params.head_size = head_size;
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
    // **Hair is re-rolled against the composites the flags just set** (#203).
    // `--age` and `--femininity` are applied after the seed has already been
    // rolled, which was harmless while hair took nothing from the composites
    // and is not now: greying is age-coupled and facial hair is gated on the
    // frame axis, so `--age 70` would have aged the skin and left the hair the
    // colour a thirty-year-old rolled — losing the single most visible age
    // marker on a head from the one flag that exists to sweep age.
    //
    // Everything but hair is locked, so this re-draws the haircut and moves no
    // other axis: the body on the sheet is the same body.
    if seed.is_some() && (value("--age").is_some() || value("--femininity").is_some()) {
        let held = Category::ALL
            .into_iter()
            .filter(|category| *category != Category::Hair)
            .fold(symbios_avatar::LockSet::NONE, |locks, category| {
                locks.with(category)
            });
        let was = record.locks;
        record.locks = held;
        record.reroll(record.seed);
        record.locks = was;
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

    // The chest's five axes, in the order they read: how much there is, how
    // far it stands off against how far it spreads, how high it sits (#273),
    // how far apart the pair sits and where its volume sits between the poles
    // (#289). Here for `--skull`'s reason — an axis whose default has to be
    // chosen by looking at both ends needs an instrument that can ask for an
    // end — and because a chest is invisible on a DRESSED body: the skin under
    // the clothes is not emitted, so `--bare` is not optional for this one.
    // The outfit's two lengths (#314, #356): the cut is what the hem is
    // judged on and a hem cannot be read off a default. A length is a share
    // of the limb, `0.5` the elbow or the knee; the old cut names still read,
    // as the lengths a record written with them reads as.
    let length = |flag: &str, names: &[(&str, f32)]| -> Option<f32> {
        let given = value(flag)?;
        let named = names
            .iter()
            .find(|(name, _)| name == given)
            .map(|(_, l)| *l);
        let length = named.or_else(|| given.parse::<f32>().ok());
        if length.is_none() {
            let known: Vec<&str> = names.iter().map(|(name, _)| *name).collect();
            eprintln!("unknown {flag} {given}: expected a length or one of {known:?}");
            std::process::exit(1);
        }
        length
    };
    use symbios_avatar::dress::{CALF, FOREARM, SHORTS};
    if let Some(leg) = length(
        "--leg",
        &[("shorts", SHORTS), ("calf", CALF), ("ankle", 1.0)],
    ) {
        record.outfit.trousers.length = leg;
    }
    if let Some(sleeve) = length(
        "--sleeve",
        &[("bare", 0.0), ("forearm", FOREARM), ("wrist", 1.0)],
    ) {
        record.outfit.top.length = sleeve;
    }
    // Each garment's colour, sRGB, and its texture (#355, #357): a surface by
    // its wire name, then optionally its tiles per metre and its turn in
    // degrees, comma-separated — `--top-texture Fabric,6,45`. `none` takes a
    // rolled texture off, for judging what the texture adds to a seed.
    let colour = |flag: &str| -> Option<[f32; 3]> {
        let given: Vec<f32> = value(flag)?
            .split(',')
            .filter_map(|channel| channel.trim().parse().ok())
            .collect();
        match given[..] {
            [r, g, b] => Some([r, g, b]),
            _ => {
                eprintln!("{flag} takes three sRGB channels, r,g,b");
                std::process::exit(1);
            }
        }
    };
    let texture = |flag: &str| -> Option<Option<symbios_avatar::GarmentTexture>> {
        let given = value(flag)?;
        let mut parts = given.split(',');
        let name = parts.next().unwrap_or_default().trim();
        if name == "none" {
            return Some(None);
        }
        let Some(surface) = symbios_avatar::SurfaceConfig::named(name) else {
            eprintln!(
                "unknown {flag} {name}: expected one of {:?}",
                symbios_avatar::SurfaceConfig::NAMES
            );
            std::process::exit(1);
        };
        let mut texture = symbios_avatar::GarmentTexture::new(surface);
        if let Some(scale) = parts.next().and_then(|s| s.trim().parse().ok()) {
            texture.scale = scale;
        }
        if let Some(rotation) = parts.next().and_then(|s| s.trim().parse().ok()) {
            texture.rotation = rotation;
        }
        Some(Some(texture))
    };
    if let Some(colour) = colour("--top-colour") {
        record.outfit.top.colour = colour;
    }
    if let Some(colour) = colour("--trousers-colour") {
        record.outfit.trousers.colour = colour;
    }
    if let Some(texture) = texture("--top-texture") {
        record.outfit.top.texture = texture;
    }
    if let Some(texture) = texture("--trousers-texture") {
        record.outfit.trousers.texture = texture;
    }
    record.sanitize();
    if let Some(spec) = value("--chest") {
        let given: Vec<f32> = spec
            .split(',')
            .filter_map(|axis| axis.trim().parse().ok())
            .collect();
        if let Archetype::Humanoid(ref mut params) = record.archetype {
            if let Some(&volume) = given.first() {
                params.chest_volume = volume;
            }
            if let Some(&projection) = given.get(1) {
                params.chest_projection = projection;
            }
            if let Some(&lift) = given.get(2) {
                params.chest_lift = lift;
            }
            if let Some(&spacing) = given.get(3) {
                params.chest_spacing = spacing;
            }
            if let Some(&fullness) = given.get(4) {
                params.chest_fullness = fullness;
            }
        }
        record.sanitize();
    }

    // Nine numbers: the limbal ring, the colour at the pupil's edge, and the
    // colour at the periphery, each r,g,b in 0..1 — the same free channels the
    // record stores (#229).
    if let Some(spec) = value("--eyes") {
        let given: Vec<f32> = spec
            .split(',')
            .filter_map(|axis| axis.trim().parse().ok())
            .collect();
        let triple = |at: usize, fallback: [f32; 3]| -> [f32; 3] {
            match (given.get(at), given.get(at + 1), given.get(at + 2)) {
                (Some(&r), Some(&g), Some(&b)) => [r, g, b],
                _ => fallback,
            }
        };
        record.eyes.ring = triple(0, record.eyes.ring);
        record.eyes.inner = triple(3, record.eyes.inner);
        record.eyes.outer = triple(6, record.eyes.outer);
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
            let mut hair = record.hair.clone();
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
            // The hair's own colour, root and tip alike, before the facial flags
            // below copy it: `--roots 0.42,0.30,0.17` sheets a light head, and a
            // defect that only a light head shows is one a dark sheet hides.
            if roots.len() == 3 {
                hair.scalp.roots = [roots[0], roots[1], roots[2]];
                hair.scalp.tips = hair.scalp.roots;
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
            }
        }),
        // **Undressed at build time, not by dropping the cloth draw** (#117).
        // A dressed body does not emit the skin its clothes cover, so filtering
        // the cloth mesh out of the merge leaves a torso with its middle
        // missing — which is what this flag did until the body it was pointed
        // at stopped drawing that skin.
        dressed: !bare,
        far_hair: tier.is_some(),
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
    // How far a beast's tongue hangs out, `0..=1`; with `--jaw`, a pant (#367).
    let tongue = value("--tongue")
        .and_then(|share| share.parse::<f32>().ok())
        .unwrap_or(0.0);
    // How far the brows are raised, in degrees; negative lowers them. Same
    // instrument-first argument as `--jaw` one slice later (#215).
    let brows = value("--brows").and_then(|degrees| degrees.parse::<f32>().ok());
    // How far the mouth corners are raised (a smile) or, negative, dropped (a
    // frown), in degrees. The third pose this file exists to take before any
    // system does (#216).
    let corners = value("--corners").and_then(|degrees| degrees.parse::<f32>().ok());
    // A named preset from the expression layer (#217), applied before the
    // per-bone flags so those stay override instruments: `--expression happy
    // --brows 0` is happy with the brows pinned back to rest.
    // A mouth shape from the lipsync vocabulary (#218), applied AFTER the
    // expression so speech owns the mouth — the precedence the viseme module
    // states once.
    let viseme = match value("--viseme") {
        Some(name) => match Viseme::named(name.as_str()) {
            some @ Some(_) => some,
            None => {
                let names: Vec<&str> = Viseme::NAMES.iter().map(|(name, _)| *name).collect();
                eprintln!(
                    "unknown --viseme {name}: expected one of {}",
                    names.join(", ")
                );
                std::process::exit(1);
            }
        },
        None => None,
    };
    let expression = match value("--expression") {
        Some(name) => match Expression::named(name.as_str()) {
            some @ Some(_) => some,
            None => {
                let names: Vec<&str> = Expression::PRESETS.iter().map(|(name, _)| *name).collect();
                eprintln!(
                    "unknown --expression {name}: expected one of {}",
                    names.join(", ")
                );
                std::process::exit(1);
            }
        },
        None => None,
    };
    let show = Show {
        linear,
        bare,
        fist,
        other,
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
        tongue,
        brows,
        corners,
        expression,
        viseme,
        far,
        far_tier: tier == Some(true),
    };
    if clumps {
        grow_clumps(&mut avatar, &record);
    }
    let mandible = jaw_bone(&avatar.rig, &record.skeleton());
    let beast_jaw = avatar.parts.beast.is_some();
    if (jawbind && mandible.is_none()) || (jaw.is_some() && mandible.is_none() && !beast_jaw) {
        eprintln!(
            "this body has no jaw bone: --jawbind needs a humanoid, --jaw a humanoid or a beast"
        );
        std::process::exit(1);
    }
    if args.iter().any(|arg| arg == "--jawsweep") {
        match mandible {
            Some(mandible) => sweep_jaw(&avatar, mandible, jaw.unwrap_or(20.0)),
            None => eprintln!("this body has no jaw bone to sweep"),
        }
    }
    let arches = face_leaves(&avatar.rig, &record.skeleton(), true);
    if brows.is_some() && arches.is_empty() {
        eprintln!("this body has no brow joints: --brows needs a humanoid");
        std::process::exit(1);
    }
    let commissures = face_leaves(&avatar.rig, &record.skeleton(), false);
    if corners.is_some() && commissures.is_empty() {
        eprintln!("this body has no mouth-corner joints: --corners needs a humanoid");
        std::process::exit(1);
    }
    // **A gallery of rolls** (#365): many seeds of one body plan side by side,
    // the instrument the creature work is judged by. Every other flag is
    // ignored, the build's own overrides included - hair, complexion, dress,
    // tier - because each tile is its own record built as it rolled; only the
    // pass, `--seed` (the first roll), `--to-scale`, `--family`,
    // `--portrait`, `--fantasy` and `--pattern` carry.
    // A winged body's wings (#421): `--wings` draws one body folded, half
    // and spread, from the side, the three quarters and above, and one wing
    // close; `--wings-crosses` every pair of families crossed, folded and
    // spread. `--extend E` poses any other sheet's wings at E instead.
    if args.iter().any(|arg| arg == "--wings-crosses") {
        wing_crosses(value("--pass").cloned(), &out);
        return;
    }
    if args.iter().any(|arg| arg == "--wings") {
        let name = value("--kin").cloned().unwrap_or_else(|| "winged".into());
        wing_sheets(
            &record,
            value("--pass").cloned(),
            &out,
            &name.replace('+', "-"),
        );
        return;
    }
    if let Some(given) = value("--gallery") {
        let Some(count) = given.parse::<usize>().ok().filter(|&count| count > 0) else {
            eprintln!("--gallery takes how many rolls to draw, at least one, not `{given}`");
            std::process::exit(2);
        };
        // `--family dog`, `cat` or `horse` keeps the rolls of that family that
        // are no cross, and `--family cross` keeps only the crosses (#366): any
        // of the plan's own families (#419).
        let family = value("--family").cloned();
        if let Some(name) = &family {
            let Some(families) = families_of(&record.archetype) else {
                eprintln!(
                    "--family picks a plan's family, and this plan has none: add --quadruped or --winged"
                );
                std::process::exit(2);
            };
            if name != "cross" && !families.contains(&name.as_str()) {
                eprintln!(
                    "--family takes {} or cross, not `{name}`",
                    families.join(", ")
                );
                std::process::exit(2);
            }
        }
        // `--fantasy` keeps only the coats drawn in an unnatural palette, and
        // `--pattern NAME` only the coats marked with it (#368).
        let fantasy = args.iter().any(|arg| arg == "--fantasy");
        let pattern = value("--pattern").map(|name| {
            symbios_avatar::plan::Pattern::ALL
                .into_iter()
                .find(|pattern| pattern.name() == name.as_str())
                .unwrap_or_else(|| {
                    eprintln!("--pattern takes a coat pattern's name, not `{name}`");
                    std::process::exit(2);
                })
        });
        if (fantasy || pattern.is_some()) && !matches!(record.archetype, Archetype::Quadruped(_)) {
            eprintln!("--fantasy and --pattern pick a quadruped's coat: add --quadruped");
            std::process::exit(2);
        }
        let keep = |record: &AvatarRecord| {
            let coat = match &record.archetype {
                Archetype::Quadruped(params) => Some(params.coat),
                _ => None,
            };
            let coat = (!fantasy
                || coat.is_some_and(|coat| coat.fantasy != symbios_avatar::plan::Fantasy::None))
                && pattern.is_none_or(|pattern| coat.is_some_and(|coat| coat.pattern == pattern));
            coat && match (&record.archetype, family.as_deref()) {
                (_, None) => true,
                (Archetype::Quadruped(params), Some(filter)) => keeps(params, filter),
                (Archetype::Winged(params), Some(filter)) => keeps(params, filter),
                _ => false,
            }
        };
        let plan = match &record.archetype {
            Archetype::Quadruped(_) => {
                Archetype::Quadruped(symbios_avatar::QuadrupedParams::default())
            }
            Archetype::Winged(_) => Archetype::Winged(symbios_avatar::WingedParams::default()),
            _ => Archetype::default(),
        };
        let to_scale = args.iter().any(|arg| arg == "--to-scale");
        // `--portrait`: each tile the head alone, from the front three
        // quarters, which is where a face is judged (#367).
        let portrait = args.iter().any(|arg| arg == "--portrait");
        let mane = value("--fur-mane").and_then(|v| v.parse::<f32>().ok());
        gallery(
            &plan,
            seed.unwrap_or(1),
            count,
            show.pass,
            Tiles {
                to_scale,
                portrait,
                mane,
                stiffness,
            },
            &keep,
            &out,
        );
        return;
    }
    let mut subject = Subject::new(avatar, show, mandible, arches, commissures);

    // **A gait at a speed** (#370): `--speed` in metres per second builds the
    // stride and the gait from the speed axis, and `--gait` holds a
    // four-legged body in one pattern whatever the speed - walk, trot, pace,
    // canter, gallop or rotary.
    let pattern = value("--gait").map(|name| {
        symbios_avatar::anim::Pattern::ALL
            .into_iter()
            .find(|pattern| pattern.name().replace(' ', "-") == name.as_str())
            .unwrap_or_else(|| {
                eprintln!(
                    "--gait takes walk, trot, pace, canter, gallop or rotary-gallop, not `{name}`"
                );
                std::process::exit(2);
            })
    });
    let speed = value("--speed").and_then(|metres| metres.parse::<f32>().ok());
    if speed.is_some() || pattern.is_some() {
        let rig = &subject.avatar.rig;
        let mut paced = symbios_avatar::anim::Speed::new(rig, speed.unwrap_or(1.0));
        if let Some(pattern) = pattern {
            paced = paced.in_pattern(pattern);
        }
        subject.gait = paced.gait(rig);
        subject.stride = paced.stride(rig);
        println!(
            "Froude {:.2}, {}, duty {:.2}, cycle {:.2} m at {:.2} Hz",
            paced.froude(),
            paced
                .pattern()
                .map_or("a person's gait", |pattern| pattern.name()),
            subject.gait.duty,
            paced.cycle_length(rig),
            paced.cadence(rig)
        );
    }
    // **The crest** (#386): a sheet of the neck alone, close enough to see
    // where a mane enters the skin - side, front three quarters,
    // behind three quarters, and from above behind.
    if args.iter().any(|arg| arg == "--throat") {
        let Some(frames) = subject.throat() else {
            eprintln!("--throat frames a neck: this body has none");
            std::process::exit(2);
        };
        write(
            &out,
            "throat",
            &subject.render(&subject.standing(), 0.0, &frames),
        );
        return;
    }
    if args.iter().any(|arg| arg == "--crest") {
        let Some(frames) = subject.crest() else {
            eprintln!("--crest frames a neck: this body has none");
            std::process::exit(2);
        };
        write(
            &out,
            "crest",
            &subject.render(&subject.standing(), 0.0, &frames),
        );
        return;
    }
    // **The nape** (#388): the back of the head and where the neck meets it -
    // side on, from behind and above as the game's camera sees it, three
    // quarters behind, and behind level. `--nape-pose` draws it nodded, turned,
    // or at the end of a rest (`nod`, `turn`, or a gesture's name).
    if args.iter().any(|arg| arg == "--nape") || args.iter().any(|arg| arg == "--mane") {
        let mut pose = subject.standing();
        let rig = &subject.avatar.rig;
        match value("--nape-pose").map(String::as_str) {
            None => {}
            Some(name @ ("nod" | "turn")) => {
                let turn = if name == "nod" {
                    Quat::from_rotation_x(0.45)
                } else {
                    Quat::from_rotation_y(0.55)
                };
                for &joint in &rig.in_zone(Zone::Neck) {
                    pose.rotations[joint] = turn;
                }
                if let Some(&occiput) = rig.in_zone(Zone::Head).first() {
                    pose.rotations[occiput] = turn;
                }
            }
            Some(name) => {
                let Some(gesture) = symbios_avatar::anim::gesture::for_body(rig, name) else {
                    eprintln!("this body has no gesture {name}");
                    std::process::exit(1);
                };
                gesture.apply(rig, &mut pose, 1.0);
            }
        }
        // **The mane, whole** (#407): six views of one mane from the withers to the
        // forelock - front, three quarters front, side close on the head and neck,
        // from above, behind and above, and the whole body side on - in a 3 by 2
        // sheet.
        if args.iter().any(|arg| arg == "--mane") {
            let Some(frames) = subject.mane(&pose) else {
                eprintln!("--mane frames a head and neck: this body has none");
                std::process::exit(2);
            };
            let built = subject.deformed(&pose, 0.0);
            let (tints, _) = subject.tints(&built);
            let mut sheet = Image::new(VIEW * 3, VIEW * 2);
            for (index, frame) in frames.iter().enumerate() {
                let view = subject.shoot(&built, &tints, frame);
                sheet.blit(&view, (index % 3) * VIEW, (index / 3) * VIEW);
            }
            write(&out, "mane", &sheet);
            // And the whole body from behind and above as the game's camera
            // sees it, side on, and three quarters ahead: to be read at play size.
            let whole = subject.sheet_frames()[0];
            let play = [
                (std::f32::consts::PI, GAME_PITCH),
                (std::f32::consts::FRAC_PI_2, 0.0),
                (std::f32::consts::FRAC_PI_4, 0.15),
            ];
            let mut sheet = Image::new(VIEW * 3, VIEW);
            for (index, (turn, pitch)) in play.into_iter().enumerate() {
                let frame = Frame {
                    turn,
                    pitch,
                    ..whole
                };
                sheet.blit(&subject.shoot(&built, &tints, &frame), index * VIEW, 0);
            }
            write(&out, "play", &sheet);
            return;
        }
        let Some(frames) = subject.nape(&pose) else {
            eprintln!("--nape frames a head: this body has none");
            std::process::exit(2);
        };
        write(&out, "nape", &subject.render(&pose, 0.0, &frames));
        return;
    }
    // **A strip** (#370): one row of side views across a whole cycle, each
    // frame's feet on the ground printed under it - the unit a gait is
    // judged in.
    if let Some(frames) = value("--strip").and_then(|count| count.parse::<usize>().ok()) {
        let frames = frames.max(1);
        let span = subject.reach * MARGIN;
        // A slope falls away under one end of the body: framed lower by it.
        let sloped = value("--drive")
            .and_then(|grade| grade.parse::<f32>().ok())
            .map_or(0.0, f32::abs);
        let frame = Frame {
            turn: std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            centre: Vec3::new(
                subject.middle.x,
                span * (0.45 - 0.5 * sloped),
                subject.middle.z,
            ),
            span,
        };
        let mut strip = Image::new(frames * VIEW, VIEW);
        // The trunk's line, read off the posed pelvis, loin and withers: its
        // pitch nose up and how far the loin stands above the chord - how a
        // beast rocks and folds (#370).
        let rig = &subject.avatar.rig;
        // The withers are the girdle the neck hangs from: the chest's first
        // joint is the middle of the ribcage on the six-ring trunk (#375).
        let girdle = rig
            .in_zone(Zone::Neck)
            .first()
            .and_then(|&neck| rig.joints[neck].parent);
        let trunk = [
            rig.in_zone(Zone::Pelvis).first().copied(),
            rig.in_zone(Zone::Abdomen).first().copied(),
            girdle,
        ];
        // Both read against the standing line: the girdles' and the loin's
        // ring centres stand at their own heights.
        let line_of = |at: &[Vec3], [hips, loin, withers]: [usize; 3]| {
            let run = at[withers] - at[hips];
            let along = (at[loin] - at[hips]).dot(run) / run.length_squared();
            (
                run.y.atan2(run.z).to_degrees(),
                at[loin].y - (at[hips].y + run.y * along),
            )
        };
        // `--gesture NAME` plays one of the rosters across the strip instead
        // of a gait, from the body standing, its last frame its end (#370);
        // with `--rise`, a beast getting up from the state it names (#377).
        let rising = args.iter().any(|arg| arg == "--rise");
        let playing = value("--gesture").map(|name| {
            let clip = if rising {
                symbios_avatar::anim::emote::getting_up(rig, name).map(|(clip, _)| clip)
            } else {
                symbios_avatar::anim::gesture::for_body(rig, name)
            };
            clip.unwrap_or_else(|| {
                eprintln!("this body has no gesture called {name}");
                std::process::exit(1);
            })
        });
        // With `--interrupt SECONDS`, the state is asked of the driver and
        // the beast told to get up that many seconds in - part of the way
        // down, or held - and the strip runs from the asking to standing
        // again (#377).
        let interrupted = value("--interrupt")
            .and_then(|seconds| seconds.parse::<f32>().ok())
            .zip(value("--gesture"))
            .map(|(seconds, name)| {
                use symbios_avatar::anim::driver::{Driver, Inputs, level_ground};
                let step = 1.0 / 60.0;
                let mut driver = Driver::seeded(7);
                let mut poses = Vec::new();
                let asked = 10usize;
                let told = asked + (seconds / step).round() as usize;
                for at in 0..asked + 6000 {
                    let gesture = if at == asked {
                        Some(name.as_str())
                    } else if at == told {
                        Some(symbios_avatar::anim::emote::GET_UP)
                    } else {
                        None
                    };
                    let inputs = Inputs {
                        delta: step,
                        gesture,
                        ..Inputs::default()
                    };
                    let Some(driven) = driver.drive(rig, &inputs, level_ground) else {
                        continue;
                    };
                    if at >= asked {
                        poses.push(driven.pose);
                    }
                    if at > told && driver.gesture().is_none() {
                        break;
                    }
                }
                poses
            });
        // `--drive GRADE` drives the body through the driver on a plane rising
        // GRADE per metre ahead, at `--speed` metres a second (standing where
        // there is none), its speed ramped from standing; the strip is its last
        // second, three seconds in, drawn over the slope (#399). Two seconds
        // in, `--then M/S` ramps it on to another speed, and `--drop METRES`
        // takes the ground that far out from under it, a ledge it falls off,
        // and `--jump M/S` sends it up off the ground at that speed;
        // `--from FRAME` draws the strip's frames one after another from that
        // frame rather than across the last second (#412, #414).
        let grade = value("--drive").and_then(|grade| grade.parse::<f32>().ok());
        let then = value("--then").and_then(|pace| pace.parse::<f32>().ok());
        let ledge = value("--drop").and_then(|metres| metres.parse::<f32>().ok());
        let jump = value("--jump").and_then(|launch| launch.parse::<f32>().ok());
        let from = value("--from").and_then(|frame| frame.parse::<usize>().ok());
        let driven: Option<Vec<Pose>> = grade.map(|grade| {
            use symbios_avatar::anim::driver::{Driver, Inputs};
            let step = 1.0 / 60.0;
            let normal = Vec3::new(0.0, 1.0, -grade).normalize();
            let pace = speed.unwrap_or(0.0);
            let mut driver = Driver::seeded(7);
            let (mut at, mut going) = (Vec3::ZERO, 0.0f32);
            let (mut height, mut falling) = (0.0f32, 0.0f32);
            let mut poses = Vec::new();
            for index in 0..240 {
                let pace = then.filter(|_| index >= 120).unwrap_or(pace);
                going += (pace - going).clamp(-2.0 * step, 2.0 * step);
                if index == 120
                    && let Some(launch) = jump
                {
                    falling = launch;
                    height = 1.0e-4;
                }
                if ledge.is_some() || jump.is_some() {
                    if index == 120
                        && let Some(metres) = ledge
                    {
                        height = metres;
                    }
                    if index >= 120 && height > 0.0 {
                        falling -= 9.81 * step;
                        height = (height + falling * step).max(0.0);
                    }
                    if height <= 0.0 {
                        falling = 0.0;
                    }
                }
                let velocity = Vec3::new(0.0, falling, going);
                at += Vec3::Z * going * step;
                let below = height;
                let ground = move |at: Vec3| {
                    Some(Ground {
                        position: Vec3::new(at.x, at.z * grade - below, at.z),
                        normal,
                    })
                };
                let inputs = Inputs {
                    delta: step,
                    velocity,
                    at,
                    ..Inputs::default()
                };
                let shown = match from {
                    Some(from) => (from..from + frames).contains(&index),
                    None => index >= 180,
                };
                if let Some(mut driven) = driver.drive(rig, &inputs, ground)
                    && shown
                {
                    driven.pose.translation.y += below;
                    poses.push(driven.pose);
                }
            }
            poses
        });
        // `--swim M/S` swims the strip instead, through one stroke, and
        // `--leap M/S` leaps it, from the wind-up to standing again (#370).
        let swimming = value("--swim").and_then(|pace| pace.parse::<f32>().ok());
        let leaping = value("--leap")
            .and_then(|launch| launch.parse::<f32>().ok())
            .map(symbios_avatar::anim::Leap::new);
        for index in 0..frames {
            let cycle = index as f32 / frames as f32;
            if let Some(leap) = leaping {
                let mut pose = Pose::rest(rig);
                let last = (frames - 1).max(1) as f32;
                leap.drive(
                    rig,
                    &mut pose,
                    leap.duration(rig) * index as f32 / last,
                    |foot| Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z))),
                );
                let mut view = subject.view(&pose, 0.0, &frame);
                ground_line(&mut view, &frame);
                strip.blit(&view, index * VIEW, 0);
                continue;
            }
            let pose = match (&playing, swimming) {
                _ if driven.is_some() => {
                    let poses = driven.as_deref().unwrap_or_default();
                    let at = index * poses.len() / frames;
                    poses.get(at).cloned().unwrap_or_else(|| Pose::rest(rig))
                }
                (Some(_), _) if interrupted.is_some() => {
                    let poses = interrupted.as_deref().unwrap_or_default();
                    let last = (frames - 1).max(1) as f32;
                    let at = (index as f32 / last * (poses.len() - 1) as f32).round() as usize;
                    poses.get(at).cloned().unwrap_or_else(|| Pose::rest(rig))
                }
                (Some(clip), _) => {
                    let mut pose = subject.standing();
                    let last = (frames - 1).max(1) as f32;
                    clip.apply(rig, &mut pose, index as f32 / last);
                    pose
                }
                (None, Some(pace)) => {
                    let mut pose = Pose::rest(rig);
                    symbios_avatar::anim::Swim::at(cycle)
                        .toward(pace)
                        .drive(rig, &mut pose);
                    pose
                }
                (None, None) => subject.walking(cycle),
            };
            let mut view = subject.view(&pose, 0.0, &frame);
            match grade {
                Some(grade) => slope_line(&mut view, &frame, grade),
                None => ground_line(&mut view, &frame),
            }
            strip.blit(&view, index * VIEW, 0);
            let line = if let [Some(hips), Some(loin), Some(withers)] = trunk {
                let rest: Vec<Vec3> = rig.joints.iter().map(|joint| joint.position).collect();
                let (pitch, arch) = line_of(&pose.forward(rig).positions, [hips, loin, withers]);
                let (pitch_0, arch_0) = line_of(&rest, [hips, loin, withers]);
                format!(
                    ", trunk {:+5.1} deg, arch {:+4.0} mm",
                    pitch - pitch_0,
                    (arch - arch_0) * 1000.0
                )
            } else {
                String::new()
            };
            let down: Vec<&str> = subject
                .gait
                .limbs
                .iter()
                .enumerate()
                .filter(|(contact, _)| subject.gait.phase(*contact, cycle).is_stance())
                .map(|(_, limb)| match limb {
                    Limb::ForeLeft => "LF",
                    Limb::ForeRight => "RF",
                    Limb::HindLeft => "LH",
                    Limb::HindRight => "RH",
                })
                .collect();
            println!("frame {index:2} at {cycle:.2}: {}{line}", down.join(" "));
        }
        write(&out, "strip", &strip);
        return;
    }

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
        Some(Focus::Crotch) => "render_crotch",
        Some(Focus::Chest) => "render_chest",
        Some(Focus::Throat) => "render_throat",
        Some(Focus::Region(follicle)) => &format!("render_{}", follicle.name()),
        None => "render",
    };

    if let Some(mode) = value("--golden") {
        golden(&subject, mode);
        return;
    }

    // `--upper-legs` standing, or `--at C` that far through a stride: how the
    // outsides of a beast's upper legs sit over its trunk (#394). With
    // `--gesture NAME --at T` it draws that moment instead, below.
    let upper_legs = args.iter().any(|arg| arg == "--upper-legs");
    if upper_legs && value("--gesture").is_none() {
        let pose = match value("--at").and_then(|cycle| cycle.parse::<f32>().ok()) {
            Some(cycle) => subject.walking(cycle),
            None => subject.standing(),
        };
        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        for (index, frame) in subject.upper_leg_frames().iter().enumerate() {
            let view = subject.view(&pose, 0.0, frame);
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        write(&out, &format!("{stem}_upper_legs"), &sheet);
        println!("rendered the upper legs");
        return;
    }

    // `--underside` standing: how a beast's legs join its trunk (#387). With
    // `--gesture NAME --at T` it draws that moment instead, below.
    if args.iter().any(|arg| arg == "--underside") && value("--gesture").is_none() {
        let pose = subject.standing();
        let mut sheet = Image::new(VIEW * 2, VIEW * 2);
        for (index, frame) in subject.underside_frames().iter().enumerate() {
            let view = subject.view(&pose, 0.0, frame);
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        write(&out, &format!("{stem}_underside"), &sheet);
        println!("rendered the underside");
        return;
    }

    // One of the crate's own goal-space gestures, by roster name — what the
    // viewer plays for a greeting (#318). Sampled over its own length from
    // the standing pose, the way `gesturing` in the viewer applies it.
    if let Some(name) = value("--gesture") {
        let Some(gesture) = symbios_avatar::anim::gesture::for_body(&subject.avatar.rig, name)
        else {
            eprintln!(
                "this body has no gesture {name}: a person's are {}, and a beast's {}",
                symbios_avatar::anim::gesture::ROSTER.join(", "),
                symbios_avatar::anim::emote::ROSTER.join(", ")
            );
            std::process::exit(1);
        };
        // `--at T`: one moment of it, its four views on the ground line
        // (#377) - `--at 1` the pose a state ends in.
        if let Some(time) = value("--at").and_then(|time| time.parse::<f32>().ok()) {
            let mut pose = subject.standing();
            gesture.apply(&subject.avatar.rig, &mut pose, time);
            // `--underside`: the same moment from below, in front and from
            // the low three-quarters, where a leg's junction reads (#387).
            let underside = args.iter().any(|arg| arg == "--underside");
            let frames = if upper_legs {
                subject.upper_leg_frames()
            } else if underside {
                subject.underside_frames()
            } else {
                subject.sheet_frames()
            };
            let mut sheet = Image::new(VIEW * 2, VIEW * 2);
            for (index, frame) in frames.iter().enumerate() {
                let mut view = subject.view(&pose, 0.0, frame);
                if !underside && !upper_legs {
                    ground_line(&mut view, frame);
                }
                sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
            }
            write(&out, &format!("{stem}_gesture_at"), &sheet);
            println!("rendered {name} at {time}");
            return;
        }
        let count = clip_frames.max(1);
        for frame in 0..count {
            // A gesture's clock runs from 0 to 1 over its own length.
            let time = frame as f32 / count as f32;
            let mut pose = subject.standing();
            gesture.apply(&subject.avatar.rig, &mut pose, time);
            let Some(sheet) = shoot(&pose, 0.0) else {
                eprintln!("this body has no such part to frame");
                std::process::exit(1);
            };
            write(&out, &format!("{stem}_gesture_{frame:02}"), &sheet);
        }
        println!("rendered {count} frames of {name}");
        return;
    }

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
            // The open-eye sheet rests at the expression's own closure —
            // negative widens, through the bound `Eye::lid_rotation` owns —
            // and the blink sheet stays at a full 1.0, because a blink
            // closes all the way over ANY expression; that it does is the
            // lids' single-writer contract, photographed (#217).
            let resting = expression.map_or(0.0, |expression| expression.closure());
            let (Some(sheet), Some(blinking)) = (
                shoot(&subject.standing(), resting),
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
/// comment**. The figures wander by a percent between runs and are useful
/// to about that: what they are for is attributing a regression to a change,
/// which needs a before as well as an after.
///
/// Two separate levers, and this print is arranged to show that they are
/// separate. Time scales with the atlas because `paint_skin` calls
/// `nearest_bone` per texel; geometry does not move with the atlas at all. At
/// the shipping 1024 the atlas is fifteen times the drawn geometry, so atlas
/// size is the whole of the memory lever and `nearest_bone` is the whole of the
/// time lever. The remedies for either are deliberately not in this
/// instrument.
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
    // A beast's fur, zone by zone (#407): what a mane costs against the rest.
    if let Some(fur) = &avatar.parts.fur {
        for grown in &fur.grown {
            println!(
                "{:<8} {:>7} tris  {:>6} clumps",
                grown.zone.name(),
                grown.tris,
                grown.clumps
            );
        }
    }
    // The far tier's own line (#350): drawn in the hair's place, never beside it.
    if let Some(far) = &avatar.far_hair {
        println!(
            "{:<8} {:>7} tris  {:>6} verts",
            "far hair",
            far.mesh.triangulated().len(),
            far.mesh.vertex_count(),
        );
    }
}

/// What a sheet is asked to show, beyond the body itself.
///
/// Gathered into one struct rather than passed one by one: every one of these
/// is an instrument switch, they arrive together and they travel together, and
/// the constructor was already at the argument count where the next one would
/// have been a lint.
#[derive(Default)]
struct Show {
    linear: bool,
    /// Whether to draw the hair at all.
    ///
    /// A shell of hair covers the ears and most of the brow on almost every
    /// seed, and the face is the thing under active work — judging a feature
    /// through a fringe is judging the fringe.
    bare: bool,
    /// Whether every finger is curled, to show the hand rig working.
    fist: bool,
    /// Whether `--close hand` / `--close foot` frames the second of the pair
    /// rather than the first (#318): the Greeting clip waves the other hand.
    other: bool,
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
    /// How far the mouth is opened, in degrees, if at all. A beast's jaw is
    /// its own part on its own joint (#367), and opens through its own pose.
    jaw: Option<f32>,
    /// How far a beast's tongue is turned out over its lip, `0..=1` (#367).
    tongue: f32,
    /// How far the brows are raised (or, negative, lowered), in degrees, if at
    /// all. The same instrument argument as `jaw`: each brow has a joint
    /// and a territory, and only rotating the joints can show whether the
    /// binding articulates — a green suite cannot.
    brows: Option<f32>,
    /// How far the mouth corners are raised (a smile) or dropped (a frown),
    /// in degrees, if at all.
    corners: Option<f32>,
    /// A named resting face from the expression layer, if any.
    expression: Option<Expression>,
    /// A mouth shape from the lipsync vocabulary, if any.
    viseme: Option<Viseme>,
    /// How many metres off to draw each view from, if not close (#350): the
    /// same frame at the pixels a 1080-line camera spends on it at that
    /// distance, scaled back up square-by-square so a sheet stitches.
    far: Option<f32>,
    /// Whether the FAR tier's hair is drawn in the near hair's place (#350).
    far_tier: bool,
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
/// a second definition of one here is a second implementation of it. The jaw
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

/// The brow joints (`above` true) or the mouth corners (`above` false), as
/// the rig carries them: lone marker leaves off a non-marker parent, off the
/// midline, split by which side of their parent joint they sit — the same
/// structural identification `skin::bind` uses, so the instrument poses
/// exactly the joints the binding credits.
fn face_leaves(rig: &Rig, skeleton: &Skeleton, above: bool) -> Vec<usize> {
    let marked = |joint: usize| {
        rig.joints[joint]
            .node
            .is_some_and(|node| skeleton.nodes[node as usize].marker)
    };
    (0..rig.len())
        .filter(|&joint| {
            marked(joint)
                && rig.joints[joint]
                    .parent
                    .is_some_and(|parent| !marked(parent))
                && !(0..rig.len())
                    .any(|child| marked(child) && rig.joints[child].parent == Some(joint))
                && rig.joints[joint].position.x != 0.0
                && rig.joints[joint].parent.is_some_and(|parent| {
                    (rig.joints[joint].position.y > rig.joints[parent].position.y) == above
                })
        })
        .collect()
}

/// Sheets of a body playing one CC0 clip, evenly across the clip's own length.
///
/// **The instrument for the two defects a still cannot show.** A mirrored
/// correspondence poses a rest body identically to a correct one, so only a
/// one-handed clip says which side ours landed on; and a twist bug moves
/// a bone's axes without moving its position, so only a roll-heavy clip shows it
///. Both go through `retarget::clip` — the same path a bake takes — so
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
    /// The brow joints, for `--brows`.
    brows: Vec<usize>,
    /// The mouth-corner joints, for `--corners`.
    corners: Vec<usize>,
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
        show: Show,
        jaw: Option<JawBone>,
        brows: Vec<usize>,
        corners: Vec<usize>,
    ) -> Self {
        let (lo, hi) = avatar.parts.body.bounds();
        Self {
            gait: Gait::natural(&avatar.rig),
            stride: Stride::for_body(&avatar.rig, 1.0),
            middle: (lo + hi) * 0.5,
            reach: (hi - lo).max_element().max(0.1),
            avatar,
            show,
            jaw,
            brows,
            corners,
        }
    }

    /// The body standing still, or with its hands closed.
    ///
    /// Closing them is what a hand rig is *for*, and a rest pose cannot show
    /// whether one works. Where a whole hand rides the wrist, every
    /// finger joint in the world moves nothing and the rest pose
    /// looks exactly the same either way.
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
        // The expression layer first, so the per-bone flags below remain
        // override instruments over a named resting face (#217).
        if let Some(expression) = self.show.expression {
            expression.apply(rig, &mut pose);
        }
        // After the expression: speech owns the mouth (#218).
        if let Some(viseme) = self.show.viseme {
            viseme.apply(rig, &mut pose);
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
        if let Some(beast) = &self.avatar.parts.beast {
            let open = self.show.jaw.map_or(0.0, |degrees| {
                degrees.to_radians() / symbios_avatar::face::beast::jaw::MAX_OPEN
            });
            beast.jaw.pose(&mut pose, open, self.show.tongue);
        }
        // About the lateral axis at each brow's pivot on the skull's own axis,
        // NEGATIVE because positive x-rotation drops what is forward of the
        // axis (the jaw's comment above) and a raise lifts it: the forehead
        // skin slides tangentially up the skull's curve, which is what the
        // tissue does (#215).
        if let Some(degrees) = self.show.brows {
            for &brow in &self.brows {
                pose.rotations[brow] = Quat::from_rotation_x(-degrees.to_radians());
            }
        }
        // About the FORE axis at each corner's pivot near the midline: a
        // vertex's lift under z-rotation is proportional to its x distance
        // from the pivot, so the philtrum holds still and the commissure
        // moves most, which is the shape of a smile (#216). The sign flips
        // per side so both corners rise together; negative degrees is a
        // frown.
        if let Some(degrees) = self.show.corners {
            for &corner in &self.corners {
                let sign = rig.joints[corner].position.x.signum();
                pose.rotations[corner] = Quat::from_rotation_z(sign * degrees.to_radians());
            }
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
        // The whole sequence through the engine's entry point (#253). The floor
        // is level and given to the stride as well as the plant, so the two
        // agree about it — on level ground that is the same answer either way,
        // and saying it once is how it stays that way.
        Walk::at(cycle).drive(rig, &mut pose, &self.gait, &self.stride, |foot| {
            Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)))
        });
        pose
    }

    /// Four views of the whole body in one image.
    fn sheet(&self, pose: &Pose, closure: f32) -> Image {
        self.render(pose, closure, &self.sheet_frames())
    }

    /// The four frames of [`Self::sheet`].
    fn sheet_frames(&self) -> [Frame; 4] {
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
        [of(0.0), of(FRAC_PI_2), of(PI), of(FRAC_PI_4)]
    }

    /// The four views a leg's junction with the trunk is judged from (#387):
    /// straight up from under the body, nose at the top of the frame; square
    /// to the chest; and low from the three-quarters, in front and behind.
    fn underside_frames(&self) -> [Frame; 4] {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        // On the trunk's underside, from pelvis to withers, so the frame is
        // spent on the junctions and not on the head and the tail.
        let rig = &self.avatar.rig;
        let along: Vec<Vec3> = [Zone::Pelvis, Zone::Abdomen, Zone::Chest]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
            .map(|joint| rig.joints[joint].position)
            .collect();
        let (near, far) = along.iter().fold((f32::MAX, f32::MIN), |(near, far), at| {
            (near.min(at.z), far.max(at.z))
        });
        let low = along.iter().map(|at| at.y).sum::<f32>() / along.len().max(1) as f32;
        let of = |turn: f32, pitch: f32| Frame {
            turn,
            pitch,
            centre: Vec3::new(0.0, low * 0.8, (near + far) * 0.5),
            span: (far - near) * 1.7,
        };
        [
            of(PI, -FRAC_PI_2 + 0.02),
            of(0.0, 0.0),
            of(FRAC_PI_4, -0.4),
            of(PI - FRAC_PI_4, -0.4),
        ]
    }

    /// The four views the outside of a beast's upper legs is judged from
    /// (#394): side on; the three-quarters, in front and behind; and from
    /// behind and above, as the game's camera sees it.
    fn upper_leg_frames(&self) -> [Frame; 4] {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let rig = &self.avatar.rig;
        let along: Vec<Vec3> = [Zone::Pelvis, Zone::Abdomen, Zone::Chest]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
            .map(|joint| rig.joints[joint].position)
            .collect();
        let (near, far) = along.iter().fold((f32::MAX, f32::MIN), |(near, far), at| {
            (near.min(at.z), far.max(at.z))
        });
        let low = along.iter().map(|at| at.y).sum::<f32>() / along.len().max(1) as f32;
        let of = |turn: f32, pitch: f32| Frame {
            turn,
            pitch,
            centre: Vec3::new(0.0, low * 0.8, (near + far) * 0.5),
            span: (far - near) * 2.0,
        };
        [
            of(FRAC_PI_2, 0.0),
            of(FRAC_PI_4, 0.1),
            of(PI - FRAC_PI_4, 0.1),
            of(PI, 0.5),
        ]
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
        let frames = self.close_frames(pose, focus)?;
        Some(self.render(pose, closure, &frames))
    }

    /// Every frame the standing sheet and the close-ups draw this body in,
    /// by view: the close-ups it has no part for left out.
    fn every_frame(&self, pose: &Pose) -> Vec<(&'static str, [Frame; 4])> {
        let mut frames = vec![("sheet", self.sheet_frames())];
        for (name, focus) in [
            ("head", Focus::Head),
            ("hand", Focus::Hand),
            ("foot", Focus::Foot),
            ("crotch", Focus::Crotch),
            ("chest", Focus::Chest),
            ("throat", Focus::Throat),
        ] {
            if let Some(four) = self.close_frames(pose, focus) {
                frames.push((name, four));
            }
        }
        frames
    }

    /// The four frames of [`Self::close_up`], or `None` where the body has no
    /// such part: apart from the drawing, so `--walls` can ask where every
    /// close-up's backdrop stands without rendering one.
    fn close_frames(&self, pose: &Pose, focus: Focus) -> Option<[Frame; 4]> {
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
                // A person's neck comes along, because a head with no neck
                // in shot says nothing about the junction. A beast's does
                // not: it is as long as the head and rises from the withers,
                // and framing it shrinks the face to a third of the view
                // (#367).
                let neck = self.avatar.rig.anatomy.human_head();
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    if *zone == Zone::Head || (neck && *zone == Zone::Neck) {
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
                // A beast's paw is its extremity's lowest three pad radii
                // (#422): its pad's bone runs up the pastern, and framed
                // whole the leg fills the shot and the toes are specks.
                let rig = &self.avatar.rig;
                let paw = (rig.anatomy.feet != symbios_avatar::FootKind::Human)
                    .then(|| rig.in_zone(Zone::Extremity(limb)).first().copied())
                    .flatten()
                    .map(|pad| posed.positions[pad].y + 2.0 * rig.joints[pad].radius);
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    let ours = match paw {
                        Some(top) => *zone == Zone::Extremity(limb) && deformed[vertex].y < top,
                        None => {
                            matches!(zone, Zone::Extremity(at) | Zone::LowerLimb(at) if *at == limb)
                        }
                    };
                    if ours {
                        hold(deformed[vertex]);
                    }
                }
            }
            // The pelvis, and the top of each thigh with it: a crotch framed on
            // the pelvis alone cuts off at exactly the crease under scrutiny.
            // Bounded to the upper half of the thighs by height rather than by
            // zone, because `UpperLimb` runs all the way to the knee and framing
            // on that zooms back out to a pair of legs.
            Focus::Crotch => {
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                let mut floor = f32::MAX;
                let mut ceiling = f32::MIN;
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    if matches!(zone, Zone::Pelvis) {
                        floor = floor.min(deformed[vertex].y);
                        ceiling = ceiling.max(deformed[vertex].y);
                    }
                }
                let reach = floor - (ceiling - floor) * 0.5;
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    // HIND limbs, not `UpperLimb(_)`: that zone is an upper ARM
                    // as readily as a thigh, and on a T-pose the arms are a
                    // body's width apart — enough to zoom the frame straight
                    // back out to the sheet this view exists to improve on.
                    let thigh = matches!(zone, Zone::UpperLimb(Limb::HindLeft | Limb::HindRight))
                        && deformed[vertex].y > reach;
                    if matches!(zone, Zone::Pelvis) || thigh {
                        hold(deformed[vertex]);
                    }
                }
            }
            // The ribcage, and the top of the abdomen under it so the fold's
            // own seat is in shot. Bounded by height rather than by taking the
            // whole abdomen, which reaches the waist and zooms the frame back
            // out past the thing it is for.
            Focus::Chest => {
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                let (mut floor, mut ceiling) = (f32::MAX, f32::MIN);
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    if matches!(zone, Zone::Abdomen) {
                        floor = floor.min(deformed[vertex].y);
                        ceiling = ceiling.max(deformed[vertex].y);
                    }
                }
                let reach = ceiling - (ceiling - floor) * 0.5;
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    let belly = matches!(zone, Zone::Abdomen) && deformed[vertex].y > reach;
                    if matches!(zone, Zone::Chest) || belly {
                        hold(deformed[vertex]);
                    }
                }
            }
            // The column, the bottom third of the head above it and the top
            // of the chest under it — both bounded by height, because the
            // whole head zooms the frame out to the head shot and the whole
            // chest zooms it out to the bust.
            Focus::Throat => {
                let deformed =
                    posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                let bounds = |wanted: Zone| {
                    let (mut floor, mut ceiling) = (f32::MAX, f32::MIN);
                    for (vertex, zone) in parts.zones.iter().enumerate() {
                        if *zone == wanted {
                            floor = floor.min(deformed[vertex].y);
                            ceiling = ceiling.max(deformed[vertex].y);
                        }
                    }
                    (floor, ceiling)
                };
                let (head_floor, head_ceiling) = bounds(Zone::Head);
                let (chest_floor, chest_ceiling) = bounds(Zone::Chest);
                let chin = head_floor + (head_ceiling - head_floor) / 3.0;
                let girdle = chest_ceiling - (chest_ceiling - chest_floor) * 0.3;
                // And bounded sideways to the inner two thirds of the chest's
                // own width, which is about the acromion: the chest zone
                // carries the arm roots, and on a T-pose those put the frame
                // a body's width across — the bust shot again, not the throat.
                let wide = parts
                    .zones
                    .iter()
                    .enumerate()
                    .filter(|(_, zone)| **zone == Zone::Chest)
                    .fold(0.0f32, |wide, (vertex, _)| {
                        wide.max(deformed[vertex].x.abs())
                    });
                for (vertex, zone) in parts.zones.iter().enumerate() {
                    let held = match zone {
                        Zone::Neck => true,
                        Zone::Head => deformed[vertex].y < chin,
                        Zone::Chest => {
                            deformed[vertex].y > girdle && deformed[vertex].x.abs() < wide * 0.55
                        }
                        _ => false,
                    };
                    if held {
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
                let part = if self.show.other {
                    attached.get(1).or(attached.first())?
                } else {
                    attached.first()?
                };
                let to_world = Mat4::from_rotation_translation(
                    posed.rotations[part.joint],
                    posed.positions[part.joint],
                );
                for &point in &part.mesh.positions {
                    hold(to_world.transform_point3(point));
                }
                // A winged body's toes grow from a foot the body itself
                // ends in (#422): the ball or the sole comes into shot.
                if matches!(focus, Focus::Foot) {
                    let deformed =
                        posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
                    for (vertex, zone) in parts.zones.iter().enumerate() {
                        if *zone == Zone::Extremity(part.limb) {
                            hold(deformed[vertex]);
                        }
                    }
                }
                let wrist = posed.positions[part.joint];
                hold(wrist + Vec3::splat(part.reach * 0.25));
                hold(wrist - Vec3::splat(part.reach * 0.25));
            }
        }
        if lo.x > hi.x {
            return None;
        }

        // The throat's frame is already bounded to the thing under judgement
        // on every side, so it takes less margin than a part framed on its
        // own bounding box: at the standard margin the collar seam is thirty
        // pixels tall, which is the brow's argument again.
        let margin = if matches!(focus, Focus::Throat) {
            CLOSE_MARGIN * 0.8
        } else {
            CLOSE_MARGIN
        };
        let of = |turn: f32, pitch: f32| Frame {
            turn,
            pitch,
            centre: (lo + hi) * 0.5,
            span: (hi.x - lo.x).max(hi.y - lo.y).max(hi.z - lo.z) * margin,
        };
        // The fourth view looks down at every close-up but the crotch, which is
        // the one part of a body that can only be seen from beneath it.
        let steep = if matches!(focus, Focus::Crotch) {
            -OVERHEAD_PITCH
        } else {
            OVERHEAD_PITCH
        };
        // The throat's fourth view is the rear three-quarter, not the
        // overhead: from above, the head fills the frame and the nape — where
        // the trapezius lives and where #302's first fill grew two lumps
        // either side of the spine — is the one thing out of shot.
        let fourth = if matches!(focus, Focus::Throat) {
            of(3.0 * FRAC_PI_4, 0.0)
        } else {
            of(0.0, steep)
        };
        Some([of(0.0, 0.0), of(FRAC_PI_4, 0.0), of(FRAC_PI_2, 0.0), fourth])
    }

    /// One three-quarter view of the head alone, for a gallery of faces
    /// (#367): framed on the head zone at rest, so a long neck does not shrink
    /// the face the tile is for, as `close_up`'s neck does.
    fn portrait(&self) -> Option<Frame> {
        let parts = &self.avatar.parts;
        let posed = self.standing().forward(&self.avatar.rig);
        let deformed = posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
        let (lo, hi) = parts
            .zones
            .iter()
            .zip(&deformed)
            .filter(|(zone, _)| **zone == Zone::Head)
            .fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(lo, hi), (_, &point)| (lo.min(point), hi.max(point)),
            );
        (lo.x <= hi.x).then(|| Frame {
            turn: PORTRAIT_TURN,
            pitch: PORTRAIT_PITCH,
            centre: (lo + hi) * 0.5,
            span: (hi - lo).max_element() * CLOSE_MARGIN,
        })
    }

    /// Four views of the neck alone at rest (#386), framed on its zone: a
    /// mane's roots are a few pixels at any framing that holds the whole body.
    fn crest(&self) -> Option<[Frame; 4]> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
        let parts = &self.avatar.parts;
        let posed = self.standing().forward(&self.avatar.rig);
        let deformed = posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
        let (lo, hi) = parts
            .zones
            .iter()
            .zip(&deformed)
            .filter(|(zone, _)| **zone == Zone::Neck)
            .fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(lo, hi), (_, &point)| (lo.min(point), hi.max(point)),
            );
        (lo.x <= hi.x).then(|| {
            let of = |turn: f32, pitch: f32| Frame {
                turn,
                pitch,
                centre: (lo + hi) * 0.5,
                span: (hi - lo).max_element() * CLOSE_MARGIN,
            };
            [
                of(FRAC_PI_2, 0.0),
                of(FRAC_PI_4, PORTRAIT_PITCH),
                of(3.0 * FRAC_PI_4, PORTRAIT_PITCH),
                of(3.0 * FRAC_PI_4, OVERHEAD_PITCH),
            ]
        })
    }

    /// Four views of the throat at rest (#395), framed on the neck and what is
    /// either side of it: side on, three quarters ahead, and from ahead and from
    /// three quarters ahead looking up at it.
    fn throat(&self) -> Option<[Frame; 4]> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
        let [side, ..] = self.crest()?;
        let wide = Frame {
            span: side.span * 1.7,
            ..side
        };
        Some([
            Frame {
                turn: FRAC_PI_2,
                pitch: 0.0,
                ..wide
            },
            Frame {
                turn: FRAC_PI_4,
                pitch: 0.0,
                ..wide
            },
            Frame {
                turn: 0.0,
                pitch: -0.45,
                ..wide
            },
            Frame {
                turn: FRAC_PI_4,
                pitch: -0.55,
                ..wide
            },
        ])
    }

    /// Four views of the back of the head in `pose` (#388), framed on the head
    /// zone and the neck's upper half: the join is the subject, and a framing
    /// that holds the whole neck leaves it a few pixels.
    fn nape(&self, pose: &Pose) -> Option<[Frame; 4]> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let parts = &self.avatar.parts;
        let posed = pose.forward(&self.avatar.rig);
        let deformed = posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
        let bounds = |wanted: Zone| {
            parts
                .zones
                .iter()
                .zip(&deformed)
                .filter(|(zone, _)| **zone == wanted)
                .fold(
                    (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                    |(lo, hi), (_, &point)| (lo.min(point), hi.max(point)),
                )
        };
        let (lo, hi) = bounds(Zone::Head);
        (lo.x <= hi.x).then(|| {
            let of = |turn: f32, pitch: f32| Frame {
                turn,
                pitch,
                centre: (lo + hi) * 0.5,
                span: (hi - lo).max_element() * CLOSE_MARGIN * 1.2,
            };
            [
                of(FRAC_PI_2, 0.0),
                of(PI, GAME_PITCH),
                of(3.0 * FRAC_PI_4, GAME_PITCH * 0.5),
                of(PI, 0.0),
            ]
        })
    }

    /// Six views of a mane in `pose` (#407), framed on the head and the neck
    /// together: front, three quarters front, side, from above (nose up the
    /// frame), behind and above as the game's camera sees it, and the whole body
    /// side on.
    fn mane(&self, pose: &Pose) -> Option<[Frame; 6]> {
        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let parts = &self.avatar.parts;
        let posed = pose.forward(&self.avatar.rig);
        let deformed = posed.deform(&self.avatar.rig, &parts.body.positions, &parts.weights);
        let (lo, hi) = parts
            .zones
            .iter()
            .zip(&deformed)
            .filter(|(zone, _)| matches!(**zone, Zone::Head | Zone::Neck))
            .fold(
                (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
                |(lo, hi), (_, &point)| (lo.min(point), hi.max(point)),
            );
        (lo.x <= hi.x).then(|| {
            let of = |turn: f32, pitch: f32| Frame {
                turn,
                pitch,
                centre: (lo + hi) * 0.5,
                span: (hi - lo).max_element() * 1.2,
            };
            let whole = self.sheet_frames()[1];
            [
                of(0.0, 0.1),
                of(FRAC_PI_4, 0.15),
                of(FRAC_PI_2, 0.0),
                of(PI, 1.45),
                of(PI, GAME_PITCH),
                whole,
            ]
        })
    }

    /// Draws one pose from four frames into a two-by-two sheet.
    fn render(&self, pose: &Pose, closure: f32, frames: &[Frame; 4]) -> Image {
        let built = self.deformed(pose, closure);
        let (tints, span) = self.tints(&built);
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
        for (index, frame) in frames.iter().enumerate() {
            let view = self.shoot(&built, &tints, frame);
            sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
        }
        sheet
    }

    /// Draws one pose from one frame, the size of one view of a sheet (#365):
    /// what a gallery tiles.
    fn view(&self, pose: &Pose, closure: f32, frame: &Frame) -> Image {
        let built = self.deformed(pose, closure);
        let (tints, _) = self.tints(&built);
        self.shoot(&built, &tints, frame)
    }

    /// The tint each drawn mesh is overlaid with, if any, and the junction's
    /// reach per joint that the tint measured on the way.
    fn tints(&self, built: &[AvatarMesh]) -> (Vec<Option<Vec<Vec3>>>, Vec<Reach>) {
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
        (tints, span)
    }

    /// One frame of the deformed meshes, at the size of one view.
    fn shoot(&self, built: &[AvatarMesh], tints: &[Option<Vec<Vec3>>], frame: &Frame) -> Image {
        let side = VIEW * SUPERSAMPLE;
        // A silhouette is the body alone, black on white: no wall behind
        // it to cover the pixels around it (#365).
        let wall = if self.show.pass.as_deref() == Some("silhouette") {
            PolyMesh::new()
        } else {
            backdrop(frame, built)
        };
        let items = items(
            built,
            &wall,
            self.avatar.skin.albedo.as_slice(),
            self.avatar.skin.roughness.as_slice(),
            self.avatar.skin.normal.as_slice(),
            self.atlas(),
            self.avatar.cloth.as_ref(),
            tints,
        );
        // The key light is written in the camera's frame, so its shadow has
        // to be cast anew for each view. Cheap enough at this size, and it
        // keeps every view internally consistent.
        let toward = frame.to_world(light::KEY);
        let shadow = ShadowMap::cast(&items, toward, frame.centre, frame.span, 1024);

        // Far off, the same frame at the pixels that distance spends on it
        // (#350); close, exactly what it always drew.
        let pixels = self
            .show
            .far
            .map(|metres| ((frame.span * FAR_PIXELS / metres).round() as usize).clamp(1, VIEW));
        let side = pixels.map_or(side, |pixels| pixels * SUPERSAMPLE);
        let mut buffer = GBuffer::new(side, side);
        if pixels.is_some() {
            let share = side as f32 / (VIEW * SUPERSAMPLE) as f32;
            buffer.floor = share * share;
        }
        buffer.draw(&items, frame);
        let ao = light::occlusion(&buffer, frame);
        let shaded = match &self.show.pass {
            Some(pass) => light::inspect(&buffer, &ao, frame, &shadow, pass),
            None => light::shade(&buffer, &ao, frame, &shadow),
        };
        let view = light::resolve(&shaded, SUPERSAMPLE);
        match pixels {
            Some(pixels) => light::enlarge(&view, pixels, VIEW),
            None => view,
        }
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
        if self.show.far_tier {
            // The far tier in the hair's place, posed as `Avatar::posed`
            // poses everything: through the blink, then forward.
            let mut blinking = pose.clone();
            if let Some(eyes) = &self.avatar.parts.eyes {
                eyes.blink(&mut blinking, closure);
            }
            let posed = blinking.forward(&self.avatar.rig);
            let far = self.avatar.far_hair.as_ref().map(|far| AvatarMesh {
                kind: far.kind,
                mesh: posed.deform_mesh(&self.avatar.rig, &far.mesh),
            });
            return bare(
                self.avatar
                    .posed(pose, closure)
                    .into_iter()
                    .filter(|drawn| drawn.kind != MeshKind::Hair)
                    .chain(far)
                    .collect(),
            );
        }
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
/// concerned. That is the reason `owner_of` and its `COVERED` constant exist;
/// here it is as a picture rather than as a constant.
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
/// **The question the jaw's constants cannot answer.** The bone runs
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
/// **A judgement image that goes through the real hair mesh rather than
/// beside it** — same material, same light, same merge — because a system
/// shown in a viewer of its own is a system nobody can compare to the one it
/// replaces.
///
/// The shapes and colours here are a REFERENCE rather than a catalogue: one
/// `Fall` per region at a length that suits it, and two colours off the
/// record.
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
        "all",
        growth.clumps(),
        growth.tris()
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
/// **The visual half of the follicle audit**: `follicleaudit` says how much of the
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
/// the first cut of this had it the other way round**. Showing only the
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
/// It answers `None` where the pipeline grows no hair: on a body with no head,
/// and on a head its plan does not call human (#365) — a quadruped's, which is
/// measured, and which is not an error.
fn regions_of(avatar: &Avatar, eyes: &EyeParams) -> Option<Follicles> {
    if !avatar.rig.anatomy.human_head() {
        return None;
    }
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

/// [`ground_line`] for a plane rising `grade` per metre ahead, seen from the
/// side (#399).
fn slope_line(view: &mut Image, frame: &Frame, grade: f32) {
    if frame.pitch != 0.0 {
        return;
    }
    let camera = frame.camera();
    let steps = view.width * 2;
    for at in 0..steps {
        if at % 12 >= 8 {
            continue;
        }
        let z = frame.centre.z + frame.span * (at as f32 / steps as f32 * 2.0 - 1.0);
        let point = camera.project_point3(Vec3::new(frame.centre.x, z * grade, z));
        let column = ((point.x + 1.0) * 0.5 * view.width as f32).round();
        let row = ((1.0 - point.y) * 0.5 * view.height as f32).round();
        if (0.0..view.width as f32).contains(&column) && (0.0..view.height as f32).contains(&row) {
            view.colour[row as usize * view.width + column as usize] = Vec3::new(0.9, 0.25, 0.05);
        }
    }
}

/// Draws the ground across a level view (#377): a thin line at the height
/// the body stands on, over everything, so whatever reaches below it shows
/// below the line. The renderer has no ground plane - its backdrop stands
/// behind the body - so a sunk paw and a planted one looked alike.
fn ground_line(view: &mut Image, frame: &Frame) {
    if frame.pitch != 0.0 {
        return;
    }
    let camera = frame.camera();
    let height = camera
        .project_point3(Vec3::new(frame.centre.x, 0.0, frame.centre.z))
        .y;
    let row = ((1.0 - height) * 0.5 * view.height as f32).round();
    if !(0.0..view.height as f32).contains(&row) {
        return;
    }
    let row = row as usize;
    for column in 0..view.width {
        // Dashed, so the body behind the line still reads through it.
        if column % 6 < 4 {
            view.colour[row * view.width + column] = Vec3::new(0.9, 0.25, 0.05);
        }
    }
}

/// Everything to draw, as items, against the given backdrop.
///
/// One item per merged mesh, which is the whole argument for merging: what used
/// to be a draw per lock of hair and a draw per garment is now a draw per
/// material, and the colours that distinguished them ride on the vertices.
#[allow(
    clippy::too_many_arguments,
    reason = "two atlases and the draw list: splitting them apart is the only other way"
)]
fn items<'a>(
    built: &'a [AvatarMesh],
    wall: &'a PolyMesh,
    albedo: &'a [u8],
    orm: &'a [u8],
    relief: &'a [u8],
    atlas: u32,
    cloth: Option<&'a symbios_avatar::TextureMap>,
    tints: &'a [Option<Vec<Vec3>>],
) -> Vec<Item<'a>> {
    let mut items = vec![Item {
        positions: &wall.positions,
        faces: &wall.faces,
        normals: None,
        paint: Paint::Flat,
        tint: None,
        cutout: None,
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
        // The skin takes its atlas, and so does the cloth when the outfit wears
        // a texture (#357) — the cloth atlas is laid out like the skin's, at
        // its own side. Everything else carries the colour it wants on its
        // vertices, as does a cloth in plain colours. The cloth's metal, if a
        // garment is made of it, is lost here as the skin's would be: this
        // renderer reads roughness from an ORM map and nothing else.
        let paint = match (drawn.kind, cloth) {
            (MeshKind::Skin, _) => Paint::Atlas {
                uvs: &drawn.mesh.uvs,
                pixels: albedo,
                orm: Some(orm),
                normals: Some(relief),
                side: atlas,
            },
            (MeshKind::Cloth, Some(cloth)) => Paint::Atlas {
                uvs: &drawn.mesh.uvs,
                pixels: &cloth.albedo,
                orm: Some(&cloth.roughness),
                normals: Some(&cloth.normal),
                side: cloth.width,
            },
            _ => Paint::Vertex(&drawn.mesh.colours),
        };
        // Hair is cut out of the strand mask the library ships (#340), as the
        // adapter's alpha-masked hair material cuts it: this instrument has to
        // draw the locks a consumer draws, not the rectangles under them.
        let cutout = matches!(drawn.kind, MeshKind::Hair).then(|| Cutout {
            uvs: &drawn.mesh.uvs,
            mask: symbios_avatar::strand_mask(),
        });
        // A skin that carries colours of its own over its atlas - a beast's
        // mouth, its tongue (#367) - is drawn with them, as a consumer that
        // multiplies vertex colour into the atlas draws it. An overlay tint
        // asked for by an instrument takes their place, and a skin all white
        // is drawn from its atlas alone, exactly as it always was.
        let own = (drawn.kind == MeshKind::Skin
            && drawn.mesh.colours.iter().any(|&colour| colour != Vec3::ONE))
        .then_some(drawn.mesh.colours.as_slice());
        items.push(Item {
            positions: &drawn.mesh.positions,
            faces: &drawn.mesh.faces,
            normals: Some(&drawn.mesh.normals),
            paint,
            tint: tint.as_deref().or(own),
            cutout,
            material,
        });
    }

    items
}

/// Where a beast's fur grows and how long, for a gallery's list of tiles
/// (#369).
fn fur_words(fur: &symbios_avatar::Fur) -> String {
    let zones: Vec<String> = [
        ("mane", fur.mane),
        ("ruff", fur.ruff),
        ("tail", fur.tail),
        ("feathering", fur.feathering),
        ("tufts", fur.tufts),
    ]
    .into_iter()
    .filter(|(_, length)| *length > 0.0)
    .map(|(zone, length)| format!("{zone} {length:.2}"))
    .collect();
    if zones.is_empty() {
        "no fur".to_string()
    } else {
        format!("fur {}", zones.join(", "))
    }
}

/// A coat's loci in a breeder's words, for a gallery's list of tiles (#368).
fn coat_words(coat: &symbios_avatar::Coat) -> String {
    use symbios_avatar::plan::{Fantasy, Pattern, White};
    let mut words = vec![
        if coat.dilute {
            format!("dilute {}", coat.pigment.name())
        } else {
            coat.pigment.name().to_string()
        },
        coat.agouti.name().to_string(),
    ];
    if coat.pattern != Pattern::None {
        words.push(coat.pattern.name().to_string());
    }
    if coat.white != White::None {
        words.push(format!("white {}", coat.white.name()));
    }
    for (carried, locus) in [
        (coat.points, "points"),
        (coat.merle, "merle"),
        (coat.roan, "roan"),
        (coat.dun, "dun"),
        (coat.tortie, "tortie"),
    ] {
        if carried {
            words.push(locus.to_string());
        }
    }
    if coat.fantasy != Fantasy::None {
        words.push(format!("in {}", coat.fantasy.name()));
    }
    if !coat.colours.is_empty() {
        words.push("free colours".to_string());
    }
    words.push(format!(
        "intensity {:.2}, marks {}",
        coat.intensity, coat.marks
    ));
    words.join(", ")
}

/// The names of a plan's families, or `None` for a plan that has none (#419):
/// the one place a plan with families is named, beside [`set_kin`]'s.
fn families_of(archetype: &Archetype) -> Option<Vec<&'static str>> {
    match archetype {
        Archetype::Quadruped(_) => Some(names::<symbios_avatar::Family>()),
        Archetype::Winged(_) => Some(names::<symbios_avatar::WingedFamily>()),
        _ => None,
    }
}

/// Every family of a plan, by name, in its own order.
fn names<K: Kin>() -> Vec<&'static str> {
    K::ALL.iter().map(|kin| kin.name()).collect()
}

/// One of a plan's families by name, or the tool's refusal of it.
fn family_named<K: Kin>(name: &str) -> K {
    K::parse(name).unwrap_or_else(|| {
        eprintln!(
            "--kin takes {} or two joined by +, not `{name}`",
            names::<K>().join(", ")
        );
        std::process::exit(2);
    })
}

/// `--kin horse` makes the body that family, `--kin dog+cat` a cross of the
/// two at half: through the plan's own change of family, so its character is
/// carried across (#388, #419).
fn set_kin<P: Blended>(params: &mut P, kin: &str) {
    // `bird+bat@0.3` a cross at a blend of its own (#422), `bird+bat` at half.
    let (kin, blend) = match kin.split_once('@') {
        Some((kin, blend)) => (kin, blend.parse::<f32>().unwrap_or(0.5)),
        None => (kin, 0.5),
    };
    let lineage = match kin.split_once('+') {
        Some((own, other)) => {
            Lineage::cross(family_named::<P::Kin>(own), family_named(other), blend)
        }
        None => Lineage::pure(family_named::<P::Kin>(kin)),
    };
    params.set_lineage(lineage);
}

/// Whether a roll is one `--family` keeps: `cross` keeps the crosses, a
/// family's name the rolls of that family that are no cross.
fn keeps<P: Blended>(params: &P, filter: &str) -> bool {
    let lineage = params.lineage();
    match filter {
        "cross" => lineage.hybrid.is_some(),
        name => lineage.hybrid.is_none() && lineage.family.name() == name,
    }
}

/// A body's family as a caption says it: `cat`, or `dog x horse at 0.40`.
fn lineage_words<P: Blended>(params: &P) -> String {
    let lineage = params.lineage();
    match lineage.hybrid {
        Some(other) => format!(
            "{} x {} at {:.2}",
            lineage.family.name(),
            other.name(),
            lineage.blend
        ),
        None => lineage.family.name().to_string(),
    }
}

/// Draws `count` rolls of `plan` that `keep` accepts, seeds `first` onward,
/// one standing side view each, into a grid (#365).
///
/// Each roll fills its own tile, standing on the tile's ground line, because
/// a shape is what a silhouette is judged on - and the stature axis spans more
/// than a factor of seven, which at one scale leaves the small rolls a few
/// pixels long. `tiles.to_scale` draws them all at the largest body's scale
/// instead, for the other half of telling families apart: their size, and
/// `tiles.portrait` draws each head alone, from the front three quarters
/// (#367). Seeds run left to
/// right, top to bottom, and the tile is the seed's: a roll that fails to
/// build leaves its tile blank rather than shifting every later one. Each is
/// built with the default config, as it rolled.
///
/// `keep` skips the seeds it declines (#366: one family at a time), and every
/// tile's seed is printed in tile order so a tile can be rolled again on its
/// own. A filter that keeps too few gives up after a thousand seeds a tile
/// and draws what it found.
fn gallery(
    plan: &Archetype,
    first: i64,
    count: usize,
    pass: Option<String>,
    tiles: Tiles,
    keep: &dyn Fn(&AvatarRecord) -> bool,
    out: &std::path::Path,
) {
    let config = AvatarConfig::default();
    use std::f32::consts::FRAC_PI_2;
    let mut records = Vec::with_capacity(count);
    let mut seed = first;
    while records.len() < count && seed < first + 1000 * count as i64 {
        let mut record = AvatarRecord::new("Gallery", plan.clone());
        record.reroll(seed);
        if let Archetype::Quadruped(params) = &mut record.archetype {
            params.fur.mane = tiles.mane.unwrap_or(params.fur.mane);
            params.fur.mane_stiffness = tiles.stiffness.unwrap_or(params.fur.mane_stiffness);
        }
        if keep(&record) {
            records.push((seed, record));
        }
        seed += 1;
    }
    for (tile, (seed, record)) in records.iter().enumerate() {
        let kind = match &record.archetype {
            Archetype::Quadruped(params) => {
                let family = lineage_words(params);
                format!(
                    "{family}: {}; {}",
                    coat_words(&params.coat),
                    fur_words(&params.fur)
                )
            }
            Archetype::Winged(params) => format!(
                "{}: size {:.2}, wing {:+.2}, neck {:+.2}, legs {:+.2}, tail {:+.2}, head {:+.2}, appeal {:+.2}, beak {:+.2},{:+.2},{:+.2}; {}",
                lineage_words(params),
                params.size,
                params.wing_length,
                params.neck_length,
                params.leg_length,
                params.tail_length,
                params.head_size,
                params.appeal,
                params.beak_length,
                params.beak_hook,
                params.facial_disc,
                surface_words(params)
            ),
            other => other.name().to_string(),
        };
        println!("tile {tile}: seed {seed}, {kind}");
    }
    let count = records.len().max(1);
    let subjects: Vec<Option<Subject>> = records
        .iter()
        .map(|(_, record)| {
            let avatar = Avatar::build_with(record, &config)?;
            let show = Show {
                eyes: record.eyes,
                pass: pass.clone(),
                ..Show::default()
            };
            Some(Subject::new(avatar, show, None, Vec::new(), Vec::new()))
        })
        .collect();
    let largest = subjects
        .iter()
        .flatten()
        .map(|subject| subject.reach)
        .fold(0.1f32, f32::max);
    let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
    let rows = count.div_ceil(columns);
    let mut sheet = Image::new(columns * VIEW, rows * VIEW);
    let mut backed = false;
    for (index, subject) in subjects.iter().enumerate() {
        let Some(subject) = subject else {
            eprintln!("seed {} did not build", records[index].0);
            continue;
        };
        let frame = if tiles.portrait {
            let Some(frame) = subject.portrait() else {
                eprintln!("seed {} has no head to frame", records[index].0);
                continue;
            };
            frame
        } else {
            // The ground a twentieth of a tile above the bottom edge.
            let span = if tiles.to_scale {
                largest
            } else {
                subject.reach
            } * MARGIN;
            Frame {
                turn: FRAC_PI_2,
                pitch: 0.0,
                centre: Vec3::new(subject.middle.x, span * 0.45, subject.middle.z),
                span,
            }
        };
        let view = subject.view(&subject.standing(), 0.0, &frame);
        // A tile no roll fills takes the backdrop's colour rather than black:
        // on a silhouette sheet a black square reads as a body (#366, on a
        // gallery of eight in a grid of nine). Filled from the first view
        // drawn, before it is copied in.
        if !backed {
            sheet.colour.fill(view.colour[0]);
            backed = true;
        }
        sheet.blit(&view, (index % columns) * VIEW, (index / columns) * VIEW);
    }
    write(out, "gallery", &sheet);
    println!(
        "drew {} rolls, from seed {first} to {}, into {columns} by {rows}",
        records.len(),
        seed - 1
    );
}

/// A winged body's surfaces in words (#423): its plumage's upper and under
/// colours, pattern and gloss, and the coat it stores, if any.
fn surface_words(params: &symbios_avatar::WingedParams) -> String {
    let rgb = |colour: [f32; 3]| {
        format!(
            "#{:02x}{:02x}{:02x}",
            (colour[0] * 255.0) as u8,
            (colour[1] * 255.0) as u8,
            (colour[2] * 255.0) as u8
        )
    };
    let plumage = &params.plumage;
    let coat = params
        .coat
        .as_ref()
        .map_or_else(|| "family coat".to_string(), coat_words);
    format!(
        "plumage head {} upper {} under {} flight {} tail {}, {} gloss {:.2}; {coat}",
        rgb(plumage.head),
        rgb(plumage.upper),
        rgb(plumage.under),
        rgb(plumage.flight),
        rgb(plumage.tail),
        plumage.pattern.name(),
        plumage.gloss
    )
}

/// A winged body posed at one extension of both wings and its tail (#421).
fn wing_pose(rig: &Rig, extension: f32) -> Pose {
    let mut pose = Pose::rest(rig);
    rig.spread_wings(&mut pose, extension, extension);
    rig.spread_tail(&mut pose, extension);
    pose
}

/// The centre and largest side of what a pose draws.
fn posed_bounds(subject: &Subject, pose: &Pose) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for drawn in subject.avatar.posed(pose, 0.0) {
        for &at in &drawn.mesh.positions {
            lo = lo.min(at);
            hi = hi.max(at);
        }
    }
    (lo, hi)
}

/// The three views a wing is judged from at each extension, framed on what
/// the pose draws: side on, the front three quarters from above, and
/// straight down from above.
fn wing_frames(subject: &Subject, pose: &Pose) -> [Frame; 3] {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
    let (lo, hi) = posed_bounds(subject, pose);
    let centre = (lo + hi) * 0.5;
    let span = (hi - lo).max_element().max(0.1) * 1.08;
    [
        Frame {
            turn: FRAC_PI_2,
            pitch: 0.05,
            centre,
            span,
        },
        Frame {
            turn: FRAC_PI_4,
            pitch: 0.45,
            centre,
            span,
        },
        Frame {
            turn: 0.0,
            pitch: FRAC_PI_2 - 0.02,
            centre,
            span,
        },
    ]
}

/// One winged body's wing sheets (#421): `wings-NAME.png`, three rows -
/// folded, half and spread - of three views each; and `wing-close-NAME.png`,
/// the left wing close, folded from the side and spread from above and
/// below.
fn wing_sheets(record: &AvatarRecord, pass: Option<String>, out: &std::path::Path, name: &str) {
    let Some(avatar) = Avatar::build(record) else {
        eprintln!("this body does not build");
        return;
    };
    print_wing_budget(name, &avatar);
    let show = Show {
        eyes: record.eyes,
        pass,
        ..Show::default()
    };
    let subject = Subject::new(avatar, show, None, Vec::new(), Vec::new());
    let mut sheet = Image::new(3 * VIEW, 3 * VIEW);
    for (row, extension) in [0.0f32, 0.5, 1.0].into_iter().enumerate() {
        let pose = wing_pose(&subject.avatar.rig, extension);
        for (column, frame) in wing_frames(&subject, &pose).iter().enumerate() {
            let view = subject.view(&pose, 0.0, frame);
            sheet.blit(&view, column * VIEW, row * VIEW);
        }
    }
    write(out, &format!("wings-{name}"), &sheet);

    // The left wing close: framed on its own joints at each pose.
    use std::f32::consts::{FRAC_PI_2, PI};
    let rig = &subject.avatar.rig;
    let wing_joints: Vec<usize> = (0..rig.len())
        .filter(|&joint| {
            matches!(
                rig.joints[joint].zone,
                Zone::UpperLimb(Limb::ForeLeft)
                    | Zone::LowerLimb(Limb::ForeLeft)
                    | Zone::Extremity(Limb::ForeLeft)
            )
        })
        .collect();
    // Framed on every drawn vertex the left wing's joints hold most.
    let close = |pose: &Pose, turn: f32, pitch: f32| {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for drawn in subject.avatar.posed(pose, 0.0) {
            for (at, skin) in drawn.mesh.positions.iter().zip(&drawn.mesh.skin) {
                if wing_joints.contains(&(skin[0].joint as usize)) {
                    lo = lo.min(*at);
                    hi = hi.max(*at);
                }
            }
        }
        Frame {
            turn,
            pitch,
            centre: (lo + hi) * 0.5,
            span: (hi - lo).max_element().max(0.05) * 1.15,
        }
    };
    let folded = wing_pose(rig, 0.0);
    let half = wing_pose(rig, 0.5);
    let spread = wing_pose(rig, 1.0);
    let mut sheet = Image::new(2 * VIEW, 2 * VIEW);
    for (index, (pose, frame)) in [
        (&folded, close(&folded, -FRAC_PI_2, 0.05)),
        (&half, close(&half, -FRAC_PI_2 * 0.6, 0.6)),
        (&spread, close(&spread, 0.0, FRAC_PI_2 - 0.02)),
        (&spread, close(&spread, PI, -FRAC_PI_2 + 0.02)),
    ]
    .into_iter()
    .enumerate()
    {
        let view = subject.view(pose, 0.0, &frame);
        sheet.blit(&view, (index % 2) * VIEW, (index / 2) * VIEW);
    }
    write(out, &format!("wing-close-{name}"), &sheet);
    println!("wrote wings-{name}.png and wing-close-{name}.png");
}

/// What a winged body costs, draw by draw (#421).
fn print_wing_budget(name: &str, avatar: &Avatar) {
    let draws: Vec<String> = avatar
        .meshes
        .iter()
        .map(|drawn| format!("{} {}", drawn.kind.name(), drawn.mesh.triangulated().len()))
        .collect();
    let wings = avatar.parts.wings.as_ref();
    println!(
        "{name}: {} triangles in {} draws ({}), {} joints; membrane {} and feathers {} triangles",
        avatar.budget.tris,
        avatar.budget.meshes,
        draws.join(", "),
        avatar.budget.joints,
        wings.map_or(0, |w| w.membrane.triangulated().len()),
        wings.map_or(0, |w| w.feathers.triangulated().len()),
    );
}

/// Every pair of winged families crossed at both sides of the halfway blend,
/// each folded side on and spread from above (#421): `wings-crosses.png`.
fn wing_crosses(pass: Option<String>, out: &std::path::Path) {
    use WingedFamily::{Bat, Bird, Glider};
    use symbios_avatar::{WingedFamily, WingedParams};
    let crosses = [
        (Bird, Bat, 0.3),
        (Bird, Bat, 0.7),
        (Bat, Glider, 0.3),
        (Bat, Glider, 0.7),
        (Glider, Bird, 0.3),
        (Glider, Bird, 0.7),
    ];
    let mut sheet = Image::new(4 * VIEW, 3 * VIEW);
    for (index, &(family, other, blend)) in crosses.iter().enumerate() {
        let mut params = WingedParams::default();
        params.set_kind(family, Some(other), blend);
        let record = AvatarRecord::new("Cross", Archetype::Winged(params));
        let Some(avatar) = Avatar::build(&record) else {
            eprintln!("{family:?} x {other:?} at {blend} does not build");
            continue;
        };
        print_wing_budget(&format!("{family:?} x {other:?} {blend}"), &avatar);
        let show = Show {
            eyes: record.eyes,
            pass: pass.clone(),
            ..Show::default()
        };
        let subject = Subject::new(avatar, show, None, Vec::new(), Vec::new());
        for (column, extension) in [0.0f32, 1.0].into_iter().enumerate() {
            let pose = wing_pose(&subject.avatar.rig, extension);
            let frames = wing_frames(&subject, &pose);
            let frame = if extension == 0.0 {
                frames[0]
            } else {
                frames[2]
            };
            let view = subject.view(&pose, 0.0, &frame);
            sheet.blit(&view, ((index % 2) * 2 + column) * VIEW, (index / 2) * VIEW);
        }
    }
    write(out, "wings-crosses", &sheet);
    println!("wrote wings-crosses.png");
}

/// How a gallery frames its tiles.
#[derive(Clone, Copy)]
struct Tiles {
    /// Every roll at one scale, the largest's, rather than each filling its
    /// tile.
    to_scale: bool,
    /// The head alone, from the front three quarters, rather than the whole
    /// body side on (#367).
    portrait: bool,
    /// A mane's length, whatever each roll grew (#386).
    mane: Option<f32>,
    /// A mane's stiffness, whatever a roll starts at (#389).
    stiffness: Option<f32>,
}

/// How far round from the face a portrait looks: the front three quarters.
const PORTRAIT_TURN: f32 = 0.55;

/// How far down a portrait looks, in radians: a little, as at a beast
/// standing below the eye.
const PORTRAIT_PITCH: f32 = 0.25;

/// A wall behind the subject, square to whichever way the camera is looking.
///
/// Not decoration either. A figure against a void has nothing to be lit against
/// and nothing to throw a shadow on, and a cast shadow is the strongest single
/// cue that a body occupies space. Camera-facing rather than world-fixed,
/// because the camera walks all the way round and a fixed wall would end up in
/// front of the subject for half the sheet.
///
/// Stood [`wall_behind`] the frame's centre, which reads `built`, the meshes
/// the frame draws, posed, so that it stands behind them all.
fn backdrop(frame: &Frame, built: &[AvatarMesh]) -> PolyMesh {
    let toward = frame.to_world(Vec3::Z);
    let right = frame.to_world(Vec3::X);
    let up = frame.to_world(Vec3::Y);
    let middle = frame.centre - toward * (frame.span * wall_behind(frame, built));
    let reach = frame.span * 2.2;

    let mut mesh = PolyMesh::new();
    for (across, rise) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        mesh.push_vertex(middle + right * (across * reach) + up * (rise * reach));
    }
    mesh.push_face([0, 1, 2, 3]);
    mesh
}

/// How far behind a frame's centre its backdrop stands, in spans: the wall's
/// depth on the frame's own [`Frame::camera`].
///
/// **Close behind the subject**, at [`WALL_BEHIND`]. Far back, a key light
/// this far off-axis throws its shadow sideways by more than the wall is wide
/// and the shadow simply leaves the picture - which is what happened the
/// moment the lighting was made good enough to have form.
///
/// **And behind everything the frame draws** (#379). A fixed distance was
/// sized on a person, whose head or chest is about as deep as the frame round
/// it is wide, and lands inside a beast: a close-up's span is the part's own,
/// and a horse's chest framed from the front three quarters has the far hind
/// leg the better part of a span behind its centre. The wall stood in front
/// of that leg and hid all of it but the stifle, which comes forward under
/// the belly, so the stifle drew as a round lump hanging off the belly, and
/// on a dog as a fragment floating beside the near hind leg - and was taken
/// for the body's, as the shadow acne was before it ([`ShadowMap::lit`]).
/// So where anything in frame reaches past [`WALL_BEHIND`] the wall stands
/// [`WALL_CLEARANCE`] behind the deepest of it; where nothing does it stands
/// exactly where it always has, and every frame it never cut draws as before.
///
/// **Every body's frames** (#380). A person was cut too, more mildly: the
/// head close-up's view from above had the wall through the shoulders, and
/// the sheet's side view the far hand past it. Both are frames the goldens
/// pin, so the goldens were blessed again on a tree where nothing else had
/// moved.
fn wall_behind(frame: &Frame, built: &[AvatarMesh]) -> f32 {
    let deepest = deepest_shown(frame, built);
    if deepest < WALL_BEHIND {
        WALL_BEHIND
    } else {
        deepest + WALL_CLEARANCE
    }
}

/// How far behind a frame's centre the deepest thing it draws stands, in
/// spans, or `f32::MIN` if it draws nothing.
///
/// A face is in frame when any corner of it lands inside the frame's square,
/// and then all its corners count: a face across the frame's edge is drawn
/// in part, and its part in frame is no nearer than its deepest corner.
fn deepest_shown(frame: &Frame, built: &[AvatarMesh]) -> f32 {
    let camera = frame.camera();
    let mut deepest = f32::MIN;
    for drawn in built {
        let clip: Vec<Vec3> = drawn
            .mesh
            .positions
            .iter()
            .map(|&point| camera.transform_point3(point))
            .collect();
        for face in &drawn.mesh.faces {
            let shown = face.iter().any(|&corner| {
                let at = clip[corner as usize];
                at.x.abs() <= 1.0 && at.y.abs() <= 1.0
            });
            if shown {
                for &corner in face {
                    deepest = deepest.max(clip[corner as usize].z);
                }
            }
        }
    }
    deepest
}

/// How far behind a frame's centre its backdrop stands when nothing in frame
/// reaches it, in spans: see [`wall_behind`].
const WALL_BEHIND: f32 = 0.32;

/// How far behind the deepest thing in frame the backdrop stands when that
/// is past [`WALL_BEHIND`], in spans: near enough that its shadow still lands
/// in the picture, clear enough that nothing touches the wall.
const WALL_CLEARANCE: f32 = 0.05;

/// `--walls N`: where the backdrop stands across N rolls of a plan, for every
/// frame of the standing sheet and of every close-up the body has (#379).
///
/// Prints, per view, how many frames have something drawn past
/// [`WALL_BEHIND`] - the frames the wall stood in front of the subject in
/// before it learned to stand back - and how many still have anything behind
/// the wall [`backdrop`] builds, which is the defect: zero on a beast, and on
/// a person what [`wall_behind`] leaves standing.
fn walls(plan: &Archetype, count: i64) {
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    // Per view and frame of it: frames taken, frames past the old wall and
    // frames still cut; and the rolls with any frame past the old wall.
    let tally: Mutex<BTreeMap<(&str, usize), [usize; 3]>> = Mutex::new(BTreeMap::new());
    let rolls: Mutex<Vec<i64>> = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get().min(8));
    std::thread::scope(|scope| {
        for thread in 0..threads {
            let (tally, rolls) = (&tally, &rolls);
            scope.spawn(move || {
                let mut seed = thread as i64;
                while seed < count {
                    let mut record = AvatarRecord::new("Rendered", plan.clone());
                    record.reroll(seed);
                    if let Some(avatar) = Avatar::build(&record) {
                        let show = Show {
                            eyes: record.eyes,
                            ..Show::default()
                        };
                        let subject = Subject::new(avatar, show, None, Vec::new(), Vec::new());
                        let pose = subject.standing();
                        let built = subject.deformed(&pose, 0.0);
                        let mut past = false;
                        for (name, frames) in subject.every_frame(&pose) {
                            for (index, frame) in frames.iter().enumerate() {
                                let deep = deepest_shown(frame, &built) >= WALL_BEHIND;
                                let cut = behind_wall(frame, &built) > 0;
                                past |= deep;
                                let mut tally = tally.lock().unwrap_or_else(|e| e.into_inner());
                                let row = tally.entry((name, index)).or_insert([0; 3]);
                                row[0] += 1;
                                row[1] += usize::from(deep);
                                row[2] += usize::from(cut);
                            }
                        }
                        if past {
                            rolls.lock().unwrap_or_else(|e| e.into_inner()).push(seed);
                        }
                    }
                    seed += threads as i64;
                }
            });
        }
    });
    let tally = tally.into_inner().unwrap_or_else(|e| e.into_inner());
    let mut rolls = rolls.into_inner().unwrap_or_else(|e| e.into_inner());
    rolls.sort_unstable();
    // A sheet's frames are front, side, back and three-quarter; a
    // close-up's front, three-quarter, side and the fourth view.
    println!(
        "{:<8} {:>5} {:>7} {:>16} {:>14}",
        "view", "frame", "frames", "past old wall", "behind wall"
    );
    for ((name, index), [frames, deep, cut]) in &tally {
        println!("{name:<8} {index:>5} {frames:>7} {deep:>16} {cut:>14}");
    }
    let shown: Vec<String> = rolls.iter().take(12).map(i64::to_string).collect();
    println!(
        "{} of {count} rolls have a frame past the old wall (first: {})",
        rolls.len(),
        shown.join(", ")
    );
}

/// How many vertices `frame` shows behind the wall [`backdrop`] builds for it.
fn behind_wall(frame: &Frame, built: &[AvatarMesh]) -> usize {
    let wall = backdrop(frame, built);
    let toward = frame.to_world(Vec3::Z);
    let camera = frame.camera();
    built
        .iter()
        .flat_map(|drawn| &drawn.mesh.positions)
        .filter(|&&point| {
            let at = camera.transform_point3(point);
            at.x.abs() <= 1.0 && at.y.abs() <= 1.0 && (point - wall.positions[0]).dot(toward) < 0.0
        })
        .count()
}

/// A golden pixel may drift this far per channel before it counts as changed.
///
/// Two steps of sRGB, which is below anything an eye can be asked to judge and
/// above the float wobble two machines can disagree by. Zero would make the
/// golden a hash and fail on the first FMA contraction; more would let a real
/// shading change hide inside it.
const GOLDEN_CHANNEL_TOLERANCE: i16 = 2;

/// Share of pixels allowed past the channel tolerance before a check fails.
const GOLDEN_PIXEL_SHARE: f32 = 0.005;

/// Blesses or checks the golden images the instrument is held to.
///
/// The instrument every judgement in this crate rests on had zero tests of its
/// own, having twice produced a false diagnosis. A golden is the cheap
/// insurance: two fixed questions — the DEFAULT record's standing sheet and its
/// head close-up — rendered through the very code path every other run uses,
/// resolved down one factor.
///
/// Run it bare: `--golden` composes with no other flag, because a golden that
/// moves with the flag set is not a fixture. `bless` writes `tests/golden/`;
/// `check` compares against it and exits nonzero on drift, naming the worst
/// channel and the share of pixels past tolerance. A missing golden fails
/// loudly — a check that silently skips is not a check.
///
/// **The fixtures are session-local by owner decision (2026-08-18) and are not
/// committed**, so `tests/golden/` is empty on a fresh clone and no test runs
/// this. Bless on the tree as you found it, change, then check — a golden
/// blessed after a change is a photograph of the change rather than a
/// baseline. `docs/instruments.md` carries the workflow and what it costs.
fn golden(subject: &Subject, mode: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let standing = subject.standing();
    let Some(head) = subject.close_up(&standing, 0.0, Focus::Head) else {
        eprintln!("the golden head close-up would not frame");
        std::process::exit(1);
    };
    let shots = [
        ("body", light::resolve(&subject.sheet(&standing, 0.0), 2)),
        ("head", light::resolve(&head, 2)),
    ];

    match mode {
        "bless" => {
            if let Err(error) = std::fs::create_dir_all(&dir) {
                eprintln!("cannot create {}: {error}", dir.display());
                std::process::exit(1);
            }
            for (name, image) in &shots {
                write(&dir, name, image);
            }
            println!(
                "blessed {} golden images into {}",
                shots.len(),
                dir.display()
            );
        }
        "check" => {
            let mut failed = false;
            for (name, image) in &shots {
                let path = dir.join(format!("{name}.png"));
                let golden = match image::open(&path) {
                    Ok(golden) => golden.to_rgba8(),
                    Err(error) => {
                        eprintln!(
                            "no golden at {}: {error}\nthe fixtures are session-local and \
                             not committed — run `--golden bless` on the tree as you found it, \
                             before the change you mean to judge",
                            path.display()
                        );
                        std::process::exit(1);
                    }
                };
                let ours = image.bytes();
                if (golden.width() as usize, golden.height() as usize)
                    != (image.width, image.height)
                {
                    eprintln!(
                        "{name}: golden is {}x{}, render is {}x{}",
                        golden.width(),
                        golden.height(),
                        image.width,
                        image.height
                    );
                    failed = true;
                    continue;
                }

                let mut worst = 0i16;
                let mut past = 0usize;
                for (theirs, ours) in golden
                    .as_raw()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(ours.as_chunks::<4>().0.iter())
                {
                    let delta = theirs
                        .iter()
                        .zip(ours)
                        .map(|(&a, &b)| (i16::from(a) - i16::from(b)).abs())
                        .max()
                        .unwrap_or(0);
                    worst = worst.max(delta);
                    if delta > GOLDEN_CHANNEL_TOLERANCE {
                        past += 1;
                    }
                }
                let share = past as f32 / (image.width * image.height) as f32;
                let verdict = if share > GOLDEN_PIXEL_SHARE {
                    failed = true;
                    "DRIFTED"
                } else {
                    "matches"
                };
                println!(
                    "{name}: {verdict} — worst channel delta {worst}, {past} pixels \
                     ({:.2}%) past tolerance {GOLDEN_CHANNEL_TOLERANCE}",
                    share * 100.0
                );
            }
            if failed {
                eprintln!(
                    "the render has drifted from its goldens; if the change is deliberate, \
                     eyeball the sheets and re-bless with `--golden bless`"
                );
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("--golden takes `bless` or `check`, not `{other}`");
            std::process::exit(2);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_a_frame_draws_stands_behind_its_backdrop() {
        // #379: a horse's, a cat's and a dog's chest framed from the front
        // three quarters had the far hind leg behind the wall, which hid all
        // of it but the stifle. #380: a person's head from above had it
        // through the shoulders, and the side view the far hand past it.
        // Every frame of the sheet and of each close-up the body has, on the
        // three beasts the defect was seen on and on the persons the goldens
        // draw and two rolls of them.
        let beasts = [13, 2, 17].map(|seed| {
            let mut record = AvatarRecord::new(
                "Rendered",
                Archetype::Quadruped(symbios_avatar::QuadrupedParams::default()),
            );
            record.reroll(seed);
            (format!("beast {seed}"), record)
        });
        let persons = [None, Some(1), Some(2)].map(|seed| {
            let mut record = AvatarRecord::default();
            if let Some(seed) = seed {
                record.reroll(seed);
            }
            (format!("person {seed:?}"), record)
        });
        for (body, record) in beasts.into_iter().chain(persons) {
            let avatar = Avatar::build(&record).expect("a rolled body builds");
            let subject = Subject::new(avatar, Show::default(), None, Vec::new(), Vec::new());
            let pose = subject.standing();
            let built = subject.deformed(&pose, 0.0);
            let frames = subject.every_frame(&pose);
            assert!(
                frames.iter().any(|(name, _)| *name == "chest"),
                "{body}: no chest close-up to frame"
            );
            for (name, four) in &frames {
                for (index, frame) in four.iter().enumerate() {
                    assert_eq!(
                        behind_wall(frame, &built),
                        0,
                        "{body}, {name} frame {index}: vertices drawn behind the backdrop"
                    );
                }
            }
        }
    }
}
