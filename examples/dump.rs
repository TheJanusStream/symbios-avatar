//! Dumps bodies to OBJ files and reports their topology.
//!
//! The engine-agnostic counterpart to a render contact sheet: run it, open the
//! `.obj` files in any DCC tool, and read the edge flow directly. The printed
//! audit is usually enough on its own to tell whether a change helped.
//!
//! Two kinds of body come through here and they are reported differently. A
//! *demo skeleton* is a test case for the mesher, so only its cage and its
//! subdivided surface are audited. A *record* is an avatar, so it goes through
//! [`symbios_avatar::Avatar`] — the same call a renderer makes — and what is
//! reported is what a consumer actually receives.
//!
//! That split is the point. This file used to carry its own copy of the
//! record-to-renderable recipe, which had already drifted from the render
//! example's copy: it dressed a quadruped in hair and two garments, because the
//! rule about which bodies wear clothes lived in the other file.
//!
//! ```text
//! cargo run --example dump                 # demo skeletons and default records
//! cargo run --example dump -- humanoid     # one demo body
//! cargo run --example dump -- --rolls 8    # eight rerolled avatar records
//! cargo run --example dump -- --walk 12    # twelve frames of a walk cycle
//! ```

use std::path::PathBuf;

use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Blink, CageConfig, FootingConfig, Gait, Ground, PolyMesh,
    Pose, QuadrupedParams, Skeleton, Stride, Vec3, anim::gait, anim::plant_feet_of, build_body,
    build_cage, demo,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rolls = args
        .iter()
        .position(|arg| arg == "--rolls")
        .and_then(|at| args.get(at + 1))
        .and_then(|count| count.parse::<i64>().ok());
    let wanted: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();

    let out = PathBuf::from(
        std::env::var("SYMBIOS_AVATAR_DUMP_DIR").unwrap_or_else(|_| "target/dump".into()),
    );
    if let Err(error) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {}: {error}", out.display());
        std::process::exit(1);
    }

    let frames = args
        .iter()
        .position(|arg| arg == "--walk")
        .and_then(|at| args.get(at + 1))
        .and_then(|count| count.parse::<usize>().ok());

    let mut failures = 0;

    if let Some(frames) = frames {
        failures += walk(&out, frames);
    } else if let Some(rolls) = rolls {
        // What a creator's randomise button actually produces.
        for seed in 0..rolls {
            for (label, archetype) in [
                ("roll_biped", Archetype::default()),
                (
                    "roll_beast",
                    Archetype::Quadruped(QuadrupedParams::default()),
                ),
            ] {
                let mut record = AvatarRecord::new(format!("{label} {seed}"), archetype);
                record.reroll(seed);
                let name = format!("{label}_{seed}");
                println!("{name:<16} {}", record.share_code());
                failures += mesh_report(&out, &name, &record.skeleton());
                failures += avatar_report(&out, &name, &record);
            }
        }
    } else {
        let bodies: Vec<(&str, Skeleton)> = vec![
            ("chain", demo::chain(3)),
            ("tripod", demo::tripod()),
            ("flat_tripod", demo::flat_tripod()),
            ("humanoid", demo::humanoid()),
            ("quadruped", demo::quadruped()),
        ];
        for (name, skeleton) in bodies {
            if !wanted.is_empty() && !wanted.iter().any(|w| *w == name) {
                continue;
            }
            failures += mesh_report(&out, name, &skeleton);
        }

        let records = [
            ("record_biped", AvatarRecord::default()),
            (
                "record_beast",
                AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default())),
            ),
        ];
        for (name, record) in records {
            if !wanted.is_empty() && !wanted.iter().any(|w| *w == name) {
                continue;
            }
            failures += mesh_report(&out, name, &record.skeleton());
            failures += avatar_report(&out, name, &record);
        }
    }

    println!("\nwrote OBJ files to {}", out.display());
    if failures > 0 {
        eprintln!("{failures} body/bodies failed to build");
        std::process::exit(1);
    }
}

/// Audits the mesher on one skeleton. Returns 1 on failure.
///
/// The cage as built, and the surface as a consumer actually gets it —
/// subdivided and with the skull shaped, which no capsule graph can express.
fn mesh_report(dir: &std::path::Path, name: &str, skeleton: &Skeleton) -> usize {
    let cage = match build_cage(skeleton, &CageConfig::default()) {
        Ok(cage) => cage,
        Err(error) => {
            println!("{name:<16} FAILED  {error}");
            return 1;
        }
    };
    let smooth = build_body(
        skeleton,
        &CageConfig::default(),
        symbios_avatar::BODY_SUBDIVISIONS,
        &Default::default(),
    )
    .unwrap_or_default();

    topology(name, "cage", &cage);
    topology(name, "smooth", &smooth);
    write(dir, name, "cage", &cage);
    write(dir, name, "smooth", &smooth);
    0
}

/// Builds a record the way a consumer does, and reports what comes back.
///
/// Returns 1 if the record could not be built at all.
fn avatar_report(dir: &std::path::Path, name: &str, record: &AvatarRecord) -> usize {
    let Some(avatar) = Avatar::build(record) else {
        println!("{name:<16} avatar  FAILED to build");
        return 1;
    };
    let budget = avatar.budget;
    println!(
        "{name:<16} {:<7} {:>6} tris  {:>2} meshes  {:>3} joints  {:>5} KiB texture",
        "budget",
        budget.tris,
        budget.meshes,
        budget.joints,
        budget.texture_bytes / 1024,
    );

    // Every mesh a renderer is handed, in one file each. These carry texture
    // coordinates and normals, so what opens in a DCC tool is what draws.
    let drawn = avatar.drawn(0.0);
    let mut whole = PolyMesh::new();
    for (index, part) in drawn.iter().enumerate() {
        println!(
            "{name:<16} {:<7} {:>6} tris  {:>6} verts",
            part.kind.name(),
            part.mesh.triangulated().len(),
            part.mesh.vertex_count(),
        );
        write(
            dir,
            name,
            &format!("{}_{index}", part.kind.name()),
            &part.mesh,
        );
        whole.append(&part.mesh);
    }
    write(dir, name, "whole", &whole);

    // The unwrap, and the atlas painted into it. A skull that measures wrong
    // puts every attached part somewhere wrong and nothing downstream says so,
    // so the measured figures are printed beside the planned ones.
    let parts = &avatar.parts;
    let used: f32 = parts.unwrap.charts.iter().map(|c| c.rect.area()).sum();
    println!(
        "{name:<16} {:<7} {:>6} charts  {:>6} split verts  {:.0}% atlas used",
        "unwrap",
        parts.unwrap.charts.len(),
        parts.unwrap.vertex_count() - parts.body.vertex_count(),
        used * 100.0,
    );
    let path = dir.join(format!("{name}_unwrapped.obj"));
    if let Err(error) = std::fs::write(&path, parts.unwrap.to_obj(&parts.body)) {
        eprintln!("cannot write {}: {error}", path.display());
    }
    for (suffix, pixels) in [
        ("albedo", &avatar.skin.albedo),
        ("normal", &avatar.skin.normal),
        ("orm", &avatar.skin.roughness),
    ] {
        let path = dir.join(format!("{name}_{suffix}.png"));
        let saved =
            image::RgbaImage::from_raw(avatar.skin.width, avatar.skin.height, pixels.clone())
                .ok_or("pixel buffer is the wrong size".to_string())
                .and_then(|image| image.save(&path).map_err(|e| e.to_string()));
        if let Err(error) = saved {
            eprintln!("cannot write {}: {error}", path.display());
        }
    }

    if let Some(eyes) = &parts.eyes {
        println!(
            "{name:<16} {:<7} radius {:.4}m  pivot {:?}",
            "eyes", eyes.left.radius, eyes.left.pivot,
        );
    }
    if let Some(hair) = &parts.hair {
        println!(
            "{name:<16} {:<7} {:>4} clumps in {} regions, {:>6} triangles  skull {:.3}m measured \
             vs {:.3}m planned",
            "hair",
            hair.clumps(),
            hair.grown.len(),
            hair.tris(),
            parts.surface.widest(hair.head),
            avatar.rig.joints[hair.head].radius,
        );
        for grown in &hair.grown {
            println!(
                "{:<16} {:<7} {:>4} clumps, {:>6} triangles",
                "",
                grown.follicle.name(),
                grown.clumps,
                grown.tris
            );
        }
    }
    // Cuts follow the body's zones, so a hem goes where the zone ends rather
    // than where its name suggests — worth printing, because that is how two of
    // the cut names turned out to be describing somewhere else entirely.
    for garment in &parts.outfit.garments {
        let (lo, hi) = garment.mesh.bounds();
        println!(
            "{name:<16} {:<7} {:>6} verts  spans y {:.3}..{:.3}  reaches x {:.3}",
            "worn",
            garment.vertex_count(),
            lo.y,
            hi.y,
            hi.x,
        );
    }
    // Sizing a part off the planned radius rather than the measured one is the
    // mistake this crate keeps making, so both are printed side by side.
    //
    // Labelled by which list the part came out of, not by which end of the body
    // its limb is on: a quadruped's front legs end in feet, and reading
    // `is_fore` reported four hands on something with none.
    let attached = parts
        .extremities
        .hands
        .iter()
        .map(|part| ("hand", part))
        .chain(parts.extremities.feet.iter().map(|part| ("foot", part)));
    for (label, part) in attached {
        println!(
            "{name:<16} {label:<7} {:>6} verts  {:?} reach {:.3}m  girth {:.4}m measured vs {:.4}m planned",
            part.mesh.vertex_count(),
            part.limb,
            part.reach,
            parts.surface.radius(part.joint, 0.0),
            avatar.rig.joints[part.joint].radius,
        );
    }
    0
}

/// Walks a body over a slope, writing each frame as a posed OBJ.
///
/// The only way to judge motion is to look at it. These frames open in any DCC
/// tool as a sequence, and the printed line per frame says what the gait and the
/// terrain each decided.
fn walk(dir: &std::path::Path, frames: usize) -> usize {
    let record = AvatarRecord::default();
    let Some(avatar) = Avatar::build(&record) else {
        eprintln!("the walking body would not build");
        return 1;
    };
    let rig = &avatar.rig;
    let gait = Gait::natural(rig);
    let stride = Stride::for_body(rig, 1.0);
    let grade = 0.12;
    let mut blink = Blink::seeded(1);

    for frame in 0..frames.max(1) {
        let cycle = frame as f32 / frames.max(1) as f32;
        let mut pose = Pose::rest(rig);
        let steps = gait::step(rig, &mut pose, &gait, &stride, cycle);

        let footing = plant_feet_of(
            rig,
            &mut pose,
            &steps.stance,
            |foot| {
                Some(Ground {
                    position: Vec3::new(foot.x, foot.z * grade, foot.z),
                    normal: Vec3::new(0.0, 1.0, -grade).normalize(),
                })
            },
            &FootingConfig::default(),
        );

        println!(
            "walk frame {frame:<3} cycle {cycle:.2}  stance {:?}  swing {:?}  \
             crouch {:.3}m  drop {:.3}m{}",
            steps.stance,
            steps.swing,
            steps.crouch,
            footing.pelvis_drop,
            if steps.is_clean() && footing.straining.is_empty() {
                ""
            } else {
                "  STRAINING"
            },
        );

        // The whole avatar, blinking as it goes: everything a renderer draws,
        // in one file per frame.
        let closure = blink.advance(1.0 / frames.max(1) as f32);
        let mut moved = PolyMesh::new();
        for part in avatar.posed(&pose, closure) {
            moved.append(&part.mesh);
        }
        let path = dir.join(format!("walk_{frame:02}.obj"));
        if let Err(error) = std::fs::write(&path, moved.to_obj()) {
            eprintln!("cannot write {}: {error}", path.display());
        }
    }
    0
}

/// Prints one line of topology stats.
fn topology(name: &str, stage: &str, mesh: &PolyMesh) {
    let audit = mesh.manifold_report();
    let (lo, hi) = mesh.bounds();
    let size = hi - lo;
    let status = if audit.is_clean() {
        "closed".to_string()
    } else {
        format!("OPEN {audit:?}")
    };
    println!(
        "{name:<16} {stage:<7} {v:>6} verts {f:>6} faces  {quads:>5.1}% quads  \
         {size_x:.2}x{size_y:.2}x{size_z:.2} m  {status}",
        v = mesh.vertex_count(),
        f = mesh.face_count(),
        quads = mesh.quad_fraction() * 100.0,
        size_x = size.x,
        size_y = size.y,
        size_z = size.z,
    );
}

/// Writes one OBJ file.
fn write(dir: &std::path::Path, name: &str, stage: &str, mesh: &PolyMesh) {
    let path = dir.join(format!("{name}_{stage}.obj"));
    if let Err(error) = std::fs::write(&path, mesh.to_obj()) {
        eprintln!("cannot write {}: {error}", path.display());
    }
}
