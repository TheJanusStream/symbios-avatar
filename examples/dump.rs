//! Dumps demo bodies to OBJ files and reports their topology.
//!
//! The engine-agnostic counterpart to a render contact sheet: run it, open the
//! `.obj` files in any DCC tool, and read the edge flow directly. It also prints
//! the manifold audit and face counts, which is usually enough to tell whether a
//! skeleton change helped without opening anything.
//!
//! ```text
//! cargo run --example dump              # every demo body, cage + 2 levels
//! cargo run --example dump -- humanoid  # just one
//! ```

use std::path::PathBuf;

use symbios_avatar::{CageConfig, PolyMesh, Skeleton, build_cage, catmull_clark, demo};

fn main() {
    let wanted: Vec<String> = std::env::args().skip(1).collect();
    let bodies: Vec<(&str, Skeleton)> = vec![
        ("chain", demo::chain(3)),
        ("tripod", demo::tripod()),
        ("flat_tripod", demo::flat_tripod()),
        ("humanoid", demo::humanoid()),
        ("quadruped", demo::quadruped()),
    ];

    let out = PathBuf::from(
        std::env::var("SYMBIOS_AVATAR_DUMP_DIR").unwrap_or_else(|_| "target/dump".into()),
    );
    if let Err(error) = std::fs::create_dir_all(&out) {
        eprintln!("cannot create {}: {error}", out.display());
        std::process::exit(1);
    }

    let config = CageConfig::default();
    let mut failures = 0;

    for (name, skeleton) in bodies {
        if !wanted.is_empty() && !wanted.iter().any(|w| w == name) {
            continue;
        }

        let cage = match build_cage(&skeleton, &config) {
            Ok(cage) => cage,
            Err(error) => {
                println!("{name:<12} FAILED  {error}");
                failures += 1;
                continue;
            }
        };
        let smooth = catmull_clark(&cage, 2);

        report(name, "cage", &cage);
        report(name, "smooth", &smooth);
        write(&out, name, "cage", &cage);
        write(&out, name, "smooth", &smooth);
    }

    println!("\nwrote OBJ files to {}", out.display());
    if failures > 0 {
        eprintln!("{failures} body/bodies failed to mesh");
        std::process::exit(1);
    }
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
        "{name:<12} {stage:<7} {v:>6} verts {f:>6} faces  {quads:>5.1}% quads  \
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
