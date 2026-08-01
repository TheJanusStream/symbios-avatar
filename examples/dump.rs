//! Dumps bodies to OBJ files and reports their topology.
//!
//! The engine-agnostic counterpart to a render contact sheet: run it, open the
//! `.obj` files in any DCC tool, and read the edge flow directly. The printed
//! audit is usually enough on its own to tell whether a change helped.
//!
//! ```text
//! cargo run --example dump                 # demo skeletons, cage + 2 levels
//! cargo run --example dump -- humanoid     # one demo body
//! cargo run --example dump -- --rolls 8    # eight rerolled avatar records
//! cargo run --example dump -- --walk 12    # twelve frames of a walk cycle
//! ```

use std::path::PathBuf;

use glam::Mat4;
use symbios_avatar::{
    Archetype, AvatarRecord, Blink, CageConfig, Extremities, EyeParams, Eyes, FootingConfig, Gait,
    Ground, Hair, HairParams, Outfit, OutfitParams, PolyMesh, Pose, QuadrupedParams, Rig, Scalp,
    Skeleton, SkinConfig, SkinParams, Stride, Surface, UvConfig, Vec3, anim::gait,
    anim::plant_feet_of, build_cage, catmull_clark, demo, rig::skin, texture, unwrap,
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
                failures += emit(&out, &name, &record.skeleton(), &record.skin);
            }
        }
    } else {
        let bodies: Vec<(&str, Skeleton)> = vec![
            ("chain", demo::chain(3)),
            ("tripod", demo::tripod()),
            ("flat_tripod", demo::flat_tripod()),
            ("humanoid", demo::humanoid()),
            ("quadruped", demo::quadruped()),
            ("record_biped", AvatarRecord::default().skeleton()),
            (
                "record_beast",
                AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()))
                    .skeleton(),
            ),
        ];
        for (name, skeleton) in bodies {
            if !wanted.is_empty() && !wanted.iter().any(|w| *w == name) {
                continue;
            }
            failures += emit(&out, name, &skeleton, &SkinParams::default());
        }
    }

    println!("\nwrote OBJ files to {}", out.display());
    if failures > 0 {
        eprintln!("{failures} body/bodies failed to mesh");
        std::process::exit(1);
    }
}

/// Meshes one skeleton, reports it, and writes both stages. Returns 1 on failure.
fn emit(dir: &std::path::Path, name: &str, skeleton: &Skeleton, skin_params: &SkinParams) -> usize {
    let cage = match build_cage(skeleton, &CageConfig::default()) {
        Ok(cage) => cage,
        Err(error) => {
            println!("{name:<16} FAILED  {error}");
            return 1;
        }
    };
    let smooth = catmull_clark(&cage, 2);

    report(name, "cage", &cage);
    report(name, "smooth", &smooth);
    write(dir, name, "cage", &cage);
    write(dir, name, "smooth", &smooth);

    // A body is only finished when it can also be posed and painted.
    match Rig::from_skeleton(skeleton) {
        Ok(rig) => {
            let weights = skin::bind(&smooth, &rig, &SkinConfig::default());
            let zones = weights.zone_map(&smooth, &rig);
            let uv = unwrap(&smooth, &rig, &zones, &UvConfig::default());
            let used: f32 = uv.charts.iter().map(|c| c.rect.area()).sum();
            println!(
                "{name:<16} {:<7} {:>6} joints {:>6} charts  {:>6} split verts  {:.0}% atlas used",
                "rig",
                rig.len(),
                uv.charts.len(),
                uv.vertex_count() - smooth.vertex_count(),
                used * 100.0,
            );
            let path = dir.join(format!("{name}_unwrapped.obj"));
            if let Err(error) = std::fs::write(&path, uv.to_obj(&smooth)) {
                eprintln!("cannot write {}: {error}", path.display());
            }

            // Paint the body so the atlas can actually be looked at.
            let atlas = texture::bake_geometry(&smooth, &uv, 1024);
            let map = texture::paint_skin(&atlas, &rig, skin_params);
            println!(
                "{name:<16} {:<7} {:>6} texels {:.0}% covered",
                "skin",
                atlas.covered(),
                atlas.coverage() * 100.0,
            );
            for (suffix, pixels) in [
                ("albedo", &map.albedo),
                ("normal", &map.normal),
                ("orm", &map.roughness),
            ] {
                let path = dir.join(format!("{name}_{suffix}.png"));
                let saved = image::RgbaImage::from_raw(map.width, map.height, pixels.clone())
                    .ok_or("pixel buffer is the wrong size".to_string())
                    .and_then(|image| image.save(&path).map_err(|e| e.to_string()));
                if let Err(error) = saved {
                    eprintln!("cannot write {}: {error}", path.display());
                }
            }
        }
        Err(error) => println!("{name:<16} rig     FAILED  {error}"),
    }

    // Eyes, open and shut, so a blink can be checked before it is animated.
    if let Ok(rig) = Rig::from_skeleton(skeleton)
        && let Some(eyes) = Eyes::build(&rig, &EyeParams::default())
    {
        let to_head = Mat4::from_translation(rig.joints[eyes.head].position);
        for (state, closure) in [("open", 0.0f32), ("shut", 1.0)] {
            let mesh = eyes.assembled(closure).transformed(to_head);
            let path = dir.join(format!("{name}_eyes_{state}.obj"));
            if let Err(error) = std::fs::write(&path, mesh.to_obj()) {
                eprintln!("cannot write {}: {error}", path.display());
            }
        }
        println!(
            "{name:<16} {:<7} {:>6} verts  radius {:.4}m  pivot {:?}",
            "eyes",
            eyes.assembled(0.0).vertex_count(),
            eyes.left.radius,
            eyes.left.pivot,
        );
    }

    // Hair, and the surface it was grown against. Both are measured from the
    // built mesh, so both are worth reporting: a skull that measures wrong puts
    // the hair somewhere wrong, and nothing downstream would say so.
    if let Ok(rig) = Rig::from_skeleton(skeleton)
        && let Ok(cage) = build_cage(skeleton, &CageConfig::default())
    {
        let mesh = catmull_clark(&cage, 2);
        if let Some(hair) = Hair::build(&mesh, &rig, &HairParams::default())
            && let Some(scalp) = Scalp::measure(&mesh, &rig)
        {
            let grown = hair
                .mesh()
                .transformed(Mat4::from_translation(scalp.origin()));
            let path = dir.join(format!("{name}_hair.obj"));
            if let Err(error) = std::fs::write(&path, grown.to_obj()) {
                eprintln!("cannot write {}: {error}", path.display());
            }
            let surface = Surface::measure(&mesh, &rig);
            println!(
                "{name:<16} {:<7} {:>6} verts  {} groups  skull {:.3}m measured vs {:.3}m planned  drop {:.3}m",
                "hair",
                grown.vertex_count(),
                hair.groups.len(),
                surface.widest(scalp.head),
                rig.joints[scalp.head].radius,
                hair.drop(),
            );
        }

        // Clothing, and where each hem lands. Cuts follow the body's zones, so
        // a hem goes where the zone ends rather than where its name suggests —
        // worth printing, because that is how two of the cut names turned out
        // to be describing somewhere else entirely.
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        let outfit = Outfit::wear(&mesh, &weights, &zones, &OutfitParams::default());
        if !outfit.is_empty() {
            let path = dir.join(format!("{name}_outfit.obj"));
            if let Err(error) = std::fs::write(&path, outfit.mesh().to_obj()) {
                eprintln!("cannot write {}: {error}", path.display());
            }
            for garment in &outfit.garments {
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
        }

        // Hands and feet, and the measured girths they were sized from. The
        // plan's own numbers are printed beside them because sizing parts off
        // the planned radius rather than the measured one is the mistake this
        // crate keeps making.
        let surface = Surface::measure(&mesh, &rig);
        let extremities = Extremities::build(&rig, &surface, 0.0);
        if !extremities.is_empty() {
            let whole =
                extremities.assembled(|joint| Mat4::from_translation(rig.joints[joint].position));
            let path = dir.join(format!("{name}_extremities.obj"));
            if let Err(error) = std::fs::write(&path, whole.to_obj()) {
                eprintln!("cannot write {}: {error}", path.display());
            }
            for part in extremities.hands.iter().chain(&extremities.feet) {
                println!(
                    "{name:<16} {:<7} {:>6} verts  {:?} reach {:.3}m  girth {:.4}m measured vs {:.4}m planned",
                    if part.limb.is_fore() { "hand" } else { "foot" },
                    part.mesh.vertex_count(),
                    part.limb,
                    part.reach,
                    surface.radius(part.joint, 0.0),
                    rig.joints[part.joint].radius,
                );
            }
        }
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
    let skeleton = record.skeleton();
    let Ok(cage) = build_cage(&skeleton, &CageConfig::default()) else {
        eprintln!("the walking body would not mesh");
        return 1;
    };
    let mesh = catmull_clark(&cage, 2);
    let Ok(rig) = Rig::from_skeleton(&skeleton) else {
        eprintln!("the walking body would not rig");
        return 1;
    };
    let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
    let gait = Gait::natural(&rig);
    let stride = Stride::for_body(&rig, 1.0);
    let grade = 0.12;
    let eyes = Eyes::build(&rig, &EyeParams::default());
    let mut blink = Blink::seeded(1);

    for frame in 0..frames.max(1) {
        let cycle = frame as f32 / frames.max(1) as f32;
        let mut pose = Pose::rest(&rig);
        let steps = gait::step(&rig, &mut pose, &gait, &stride, cycle);

        let footing = plant_feet_of(
            &rig,
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

        let posed = pose.forward(&rig);
        let mut moved = PolyMesh {
            positions: posed.deform(&rig, &mesh.positions, &weights),
            faces: mesh.faces.clone(),
        };

        // The eyes ride the head rather than being skinned, so they follow it
        // through one rigid transform.
        if let Some(eyes) = &eyes {
            let closure = blink.advance(1.0 / frames.max(1) as f32);
            let head = Mat4::from_rotation_translation(
                posed.rotations[eyes.head],
                posed.positions[eyes.head],
            ) * Mat4::from_translation(-rig.joints[eyes.head].position);
            moved.append(
                &eyes
                    .assembled(closure)
                    .transformed(head * Mat4::from_translation(rig.joints[eyes.head].position)),
            );
        }
        let path = dir.join(format!("walk_{frame:02}.obj"));
        if let Err(error) = std::fs::write(&path, moved.to_obj()) {
            eprintln!("cannot write {}: {error}", path.display());
        }
    }
    0
}

/// Prints one line of topology stats.
fn report(name: &str, stage: &str, mesh: &PolyMesh) {
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
