//! Bakes the curated CC0 clip set into the artifact this repository carries.
//!
//! Run it after changing the retargeter, the rig, or the list below:
//!
//! ```text
//! cargo run --release --example bakeclips            # report and write the artifact
//! cargo run --release --example bakeclips -- --dry   # report only
//! ```
//!
//! # Why an offline bake
//!
//! Baked rather than imported at run time, so glTF
//! stays out of the shipping payload and every retarget defect is a bake-time
//! defect somebody can look at. The two source files are **not vendored** —
//! eleven megabytes of somebody else's CC0 — so this reads them from a sibling
//! checkout and says so plainly when they are absent.
//!
//! # The instrument this prints, and why it is the one that matters
//!
//! Not the bytes. **The collapse rate**: how many of our 65 tracks move against
//! how many of the reference's own 66 joints move through the same clip. Those
//! two numbers should agree closely, because a transfer that introduces motion
//! of its own is a transfer that is wrong, which is what a
//! first draft did, turning Walk's 21 moving tracks into 52 while every visible
//! check passed at 0.028 degrees. Nothing visible would have shown it. The size
//! did.
//!
//! So every clip is measured that way and not only Walk. The source's count is
//! derived rather than read off its channel list: each node's LOCAL rotation is
//! recovered per frame and counted as moving under the same tolerance
//! [`Curve::bake`] uses, which is the same question asked of both rigs.
//!
//! # And two the collapse rate cannot see
//!
//! The collapse rate says the transfer did not invent motion. It says nothing
//! about the motion the SOURCE brought with it — the imported clips do not
//! loop cleanly, and on some of them the body teleports between frames. Both are
//! printed per clip as ratios to that clip's own median step — see
//! [`Continuity`] — so a re-bake reports them and nobody has to remember the
//! caveat separately.
//!
//! [`Continuity`]: symbios_avatar::Continuity

use std::path::Path;

use glam::Quat;
use symbios_avatar::gltf::Gltf;
use symbios_avatar::plan::Zone;
use symbios_avatar::retarget::STILL;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, ClipLibrary, PoseClip, retarget};

/// Where the CC0 reference animations sit, relative to this checkout.
const LIBRARY: [&str; 2] = [
    "../mesh2motion-app/static/animations/human-base-animations.glb",
    "../mesh2motion-app/static/animations/human-addon-animations.glb",
];

/// Where the baked artifact is written.
const ARTIFACT: &str = "assets/clips.bin";

/// The frame rate every clip is resampled to.
///
/// The sources are two thirds `STEP` samplers at irregular times, so nothing is
/// lost by resampling; 30 is what `Walk` was measured at and what the retarget's
/// own audit reports.
const RATE: f32 = 30.0;

/// The curated set: what an avatar in a social space actually does.
///
/// **The principle, which is what this list is an argument for rather than a
/// list of favourites.** A clip earns a slot if it passes one of three tests and
/// none of the four disqualifications.
///
/// It earns a slot if it is ADDRESSED TO SOMEBODY — a social space is other
/// people, and a greeting, a nod, a refusal and a bow are all speech acts that
/// carry meaning to a viewer. Or if it is WHAT A BODY DOES BETWEEN THINGS: an
/// avatar is idle for most of the time it exists, and the idle is the state a
/// viewer sees longest. Or if it is HOW A BODY GETS SOMEWHERE, which is the only
/// motion the system drives rather than the user.
///
/// It is out if it NEEDS A PROP OR A WORLD WE DO NOT HAVE — every sword, pistol,
/// bow, axe, ladder and steering wheel in the library is a mime without the
/// object, and that is the largest single family in it. Or if it is a ONE-TIME
/// SPECTACLE: a backflip is fun to watch once and then it is the thing your
/// avatar does. Or if it NEEDS STATE WE DO NOT MODEL — there is no damage model,
/// so a death and a flinch are poses without a cause. Or if it is a TRANSITION
/// INTO OR OUT OF a state, because a transition is worth carrying only once
/// something owns the state machine, and nothing does yet.
///
/// `looping` is a fact about the clip, not a preference: a loop's wrap from last
/// frame to first is one frame like every other, so marking a one-shot as
/// looping puts a jump in it and marking a loop as a one-shot loses a frame.
const CURATED: &[(&str, bool)] = &[
    // The idle, which is what an avatar is doing whenever nobody is driving it.
    ("Idle_A", true),
    // Both halves of a conversation. A talking idle without a listening one
    // gives a room where everybody talks — which is the substitution this list
    // makes against the issue's first cut, where a Victory took this slot.
    ("Idle_Talking", true),
    ("Idle Listening", true),
    // Locomotion. Walk is in whatever else is (owner, 2026-08-07): the
    // procedural-versus-imported call is deferred to #141 and cannot be made
    // without it. Jog and Sprint come with it because a speed axis needs more
    // than one point on it.
    ("Walk", true),
    ("Jog", true),
    ("Sprint", true),
    // People sit down in rooms.
    ("Sitting_Idle", true),
    // The speech acts. A greeting, a yes, a no and a bow are the four gestures
    // that carry meaning without a shared language or a chat box.
    ("Greeting", false),
    ("Head Nod", false),
    ("Reject", false),
    ("Bow", false),
    // Away. An avatar left alone should read as away rather than as broken.
    ("Sleeping", true),
];

fn main() {
    let dry = std::env::args().any(|arg| arg == "--dry");

    let sources: Vec<(&str, Gltf)> = LIBRARY
        .iter()
        .filter_map(|path| match std::fs::read(path) {
            Ok(bytes) => match Gltf::read(&bytes) {
                Ok(library) => Some((*path, library)),
                Err(why) => {
                    eprintln!("{path}: {why}");
                    std::process::exit(1);
                }
            },
            Err(_) => None,
        })
        .collect();
    if sources.len() < LIBRARY.len() {
        eprintln!(
            "the CC0 reference animations are not checked out beside this repository.\n\
             This tool reads them from:\n  {}\n  {}\n\
             They are deliberately not vendored here — eleven megabytes of somebody\n\
             else's CC0 — so clone mesh2motion-app as a sibling of this checkout and\n\
             run this again. The artifact at {ARTIFACT} is already baked; nothing is\n\
             broken by not having them, only re-baking is.",
            LIBRARY[0], LIBRARY[1]
        );
        std::process::exit(1);
    }

    // Against a BUILT body, never a plan's skeleton. `Rig::from_skeleton` rigs
    // 33 joints and `Avatar::build` 73, and the difference is the hands and feet
    // where forty of the reference's sixty-six names land (#139). Baking against
    // the wrong one produces a clip with no fingers in it and no error, which is
    // why `Correspondence::human` refuses it — this is the side that must not
    // defeat that refusal.
    let record = AvatarRecord::new("Bake", Archetype::default());
    let avatar = Avatar::build(&record).expect("the default record builds a biped");
    let rig = &avatar.rig;

    println!(
        "{:<17}{:>8}{:>7}{:>9}{:>8}{:>7}{:>10}{:>10}{:>8}{:>8}",
        "clip",
        "seconds",
        "frames",
        "src move",
        "leaves",
        "ours",
        "bytes",
        "loop",
        "jump x",
        "seam x"
    );
    println!("{}", "-".repeat(92));

    let mut library = ClipLibrary::new();
    let mut provenance: Vec<(String, &str)> = Vec::new();
    let mut suspect = Vec::new();
    for &(name, looping) in CURATED {
        let Some((path, source, animation)) = find(&sources, name) else {
            eprintln!("no clip called {name} in either reference file");
            std::process::exit(1);
        };
        let matched = retarget::Correspondence::human(
            rig,
            source,
            &source.skin(0).expect("the reference has a skin"),
        )
        .expect("the reference matches a built body");
        let baked = retarget::clip(rig, source, &matched, animation, RATE, looping)
            .unwrap_or_else(|why| panic!("{name}: {why}"));

        let Moving { total, silent } = moving_in_source(source, animation, &baked);
        let ours = baked.moving();
        // **An identity, not a tolerance.** Measured over the whole set: our
        // sampled track count equals the source's moving joints minus the ones
        // the retargeter deliberately gives no motion to — the leaves, whose
        // rotation deforms nothing, and the reference's `root`, which arrives as
        // `Pose::translation` rather than as a joint. It holds exactly on all
        // twelve, so anything else is a finding rather than noise: ABOVE means
        // the transfer introduced motion, which is the shape the #139 bug had,
        // and BELOW means it dropped some.
        if ours + silent != total {
            suspect.push((name, total, silent, ours));
        }

        // **What the clip does to a body between its own frames, printed at
        // the moment it is made** (#249). The collapse rate says the transfer
        // did not invent motion; these say what motion the SOURCE brought with
        // it, which is not the same question and is the one that decides
        // whether a clip can be a gold standard. It cannot: see `Continuity`.
        let continuity = baked.continuity(rig);
        println!(
            "{:<17}{:>8.2}{:>7}{:>6} /66{:>8}{:>7}{:>10}{:>10}{:>8.1}{:>8}",
            name,
            baked.duration(),
            baked.frames,
            total,
            silent,
            ours,
            baked.bytes(),
            if looping { "yes" } else { "one-shot" },
            continuity.jump_ratio(),
            continuity
                .seam_ratio()
                .map_or_else(|| "-".to_string(), |ratio| format!("{ratio:.1}")),
        );
        provenance.push((baked.name.clone(), path));
        library.clips.push(baked);
    }

    let bytes = library.write();
    println!("{}", "-".repeat(92));
    println!(
        "{} clips, {} bytes ({:.1} KiB), {:.1} KiB per clip on average",
        library.len(),
        bytes.len(),
        bytes.len() as f32 / 1024.0,
        bytes.len() as f32 / 1024.0 / library.len() as f32
    );

    // The container decision, measured rather than argued. `PoseClip` derives
    // `Serialize`, so the alternative costs nothing to weigh.
    let json = serde_json::to_vec(&library.clips).expect("clips serialise");
    println!(
        "as JSON it would be {} bytes ({:.1} KiB), {:.1}x this",
        json.len(),
        json.len() as f32 / 1024.0,
        json.len() as f32 / bytes.len() as f32
    );

    // What the fingers cost, because the decision to keep them was the owner's
    // and a decision deserves its price in the report rather than in an
    // argument. Hands and feet share `Zone::Extremity`; a foot is four of the
    // twenty-five joints in one, so this is fingers to within a rounding.
    let digits: usize = library
        .clips
        .iter()
        .flat_map(|clip| &clip.tracks)
        .filter(|track| matches!(track.slot.zone, Zone::Extremity(_)))
        .map(|track| track.rotation.bytes() + 3)
        .sum();
    println!(
        "of which hands and feet are {} bytes ({:.1} KiB, {:.0}% of the artifact)",
        digits,
        digits as f32 / 1024.0,
        100.0 * digits as f32 / bytes.len() as f32
    );

    if suspect.is_empty() {
        println!("collapse: ours + the source's silent movers = the source's own, on every clip");
    } else {
        println!();
        println!("COLLAPSE RATES THAT DO NOT ADD UP, which is a finding and not noise:");
        for (name, total, silent, ours) in &suspect {
            let want = total - silent;
            println!(
                "  {name}: the source moves {total} joints, {silent} of which we \
                 deliberately hold, so {want} tracks should move and {ours} do"
            );
        }
        println!(
            "  Above means the transfer introduced motion of its own, which is what\n  \
             #139 did while every visible check passed at 0.028 degrees; below means\n  \
             it dropped some. Neither shows in a render."
        );
    }

    if dry {
        println!();
        println!("--dry: nothing written");
        return;
    }

    if let Some(parent) = Path::new(ARTIFACT).parent() {
        std::fs::create_dir_all(parent).expect("the artifact's directory can be made");
    }
    std::fs::write(ARTIFACT, &bytes).expect("the artifact can be written");
    println!();
    println!("wrote {ARTIFACT}");
    println!();
    println!("PROVENANCE — every clip is CC0, from mesh2motion (LICENSE-CC0.MD covers");
    println!("all 3d models, blend files, rigs and animations). Paste into docs/clips.md:");
    for (name, path) in &provenance {
        let file = Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(path);
        println!("| {name} | {file} | CC0 1.0 |");
    }
}

/// The first reference file holding a clip of that name, and its index in it.
fn find<'a>(sources: &'a [(&'a str, Gltf)], name: &str) -> Option<(&'a str, &'a Gltf, usize)> {
    sources
        .iter()
        .find_map(|(path, library)| library.clip(name).map(|at| (*path, library, at)))
}

/// How many of the reference's own joints move, and how many of those we hold.
struct Moving {
    /// Joints whose local rotation varies through the clip.
    total: usize,
    /// Of those, the ones the retargeter deliberately gives no motion to: the
    /// leaves, whose rotation deforms no surface, and the reference's own
    /// `root`, which arrives as [`PoseClip`]'s translation rather than as a
    /// joint. `total` minus this is exactly how many of our tracks should move.
    silent: usize,
}

/// How many of the reference's own joints actually move through a clip.
///
/// **Derived rather than read off the file's channel list**, so that it is the
/// same question our own [`PoseClip::moving`] answers: a joint moves if its
/// LOCAL rotation varies by more than [`STILL`], the tolerance a curve is baked
/// at. Read off the channels instead it would count a sampler that exists and
/// holds still, which is a different and less useful number.
///
/// **Local, not world**, and that distinction is the whole point. A finger
/// carried rigidly by a moving hand has a world delta as large as the hand's and
/// a local rotation that never changes; counting the world one would say the
/// source moves everything and make the comparison useless — which is the same
/// confusion of frames that produces a silent transfer bug.
///
/// Sampled at the same rate and over the same span as the bake, so the two
/// counts are comparable frame for frame.
fn moving_in_source(library: &Gltf, animation: usize, baked: &PoseClip) -> Moving {
    let Ok(skin) = library.skin(0) else {
        return Moving {
            total: 0,
            silent: 0,
        };
    };
    // Where each joint's parent sits, as a document node — the hierarchy the
    // local transform is taken against. A joint whose parent is outside the skin
    // reads as a root, and a root's local transform is its world one.
    let parent: Vec<Option<usize>> = skin
        .parents
        .iter()
        .map(|at| at.map(|index| skin.nodes[index]))
        .collect();
    // A joint we hold still whatever the source does with it: a leaf, because
    // its rotation deforms no surface, or the skin's own root, whose motion is
    // translation on our side.
    let held: Vec<bool> = (0..skin.nodes.len())
        .map(|joint| {
            skin.parents.get(joint).copied().flatten().is_none()
                || !skin.parents.contains(&Some(joint))
        })
        .collect();

    let mut first: Vec<Option<Quat>> = vec![None; skin.nodes.len()];
    let mut worst = vec![0.0f32; skin.nodes.len()];
    for frame in 0..baked.frames {
        let time = baked.duration() * frame as f32 / baked.frames.max(1) as f32;
        let Ok(posed) = library.sample(animation, time) else {
            return Moving {
                total: 0,
                silent: 0,
            };
        };
        for (joint, &node) in skin.nodes.iter().enumerate() {
            let local = match parent[joint] {
                Some(above) => posed[above].inverse() * posed[node],
                None => posed[node],
            };
            let (_, turn, _) = local.to_scale_rotation_translation();
            let turn = turn.normalize();
            match first[joint] {
                None => first[joint] = Some(turn),
                Some(start) => {
                    // `q` and `-q` are the same rotation, so a track that flips
                    // sign is a still joint that looks like it spins — the same
                    // alignment `Curve::bake` does before it compares.
                    let turn = if turn.dot(start) < 0.0 { -turn } else { turn };
                    let apart = (turn.x - start.x)
                        .abs()
                        .max((turn.y - start.y).abs())
                        .max((turn.z - start.z).abs())
                        .max((turn.w - start.w).abs());
                    worst[joint] = worst[joint].max(apart);
                }
            }
        }
    }
    let movers: Vec<usize> = (0..skin.nodes.len())
        .filter(|&joint| worst[joint] > STILL)
        .collect();
    Moving {
        silent: movers.iter().filter(|&&joint| held[joint]).count(),
        total: movers.len(),
    }
}
