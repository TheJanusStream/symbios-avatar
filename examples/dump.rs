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
//! ```

use std::path::PathBuf;

use symbios_avatar::{
    Archetype, AvatarRecord, CageConfig, PolyMesh, QuadrupedParams, Skeleton, build_cage,
    catmull_clark, demo,
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

    let mut failures = 0;

    if let Some(rolls) = rolls {
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
                failures += emit(&out, &name, &record.skeleton());
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
            failures += emit(&out, name, &skeleton);
        }
    }

    println!("\nwrote OBJ files to {}", out.display());
    if failures > 0 {
        eprintln!("{failures} body/bodies failed to mesh");
        std::process::exit(1);
    }
}

/// Meshes one skeleton, reports it, and writes both stages. Returns 1 on failure.
fn emit(dir: &std::path::Path, name: &str, skeleton: &Skeleton) -> usize {
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
