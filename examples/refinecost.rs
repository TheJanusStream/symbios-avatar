//! What each face-refinement pass costs, and which band it buys.
//!
//! A pass set cannot be costed by scaling a band, and a costing taken off an
//! older body goes stale as soon as anything moves the surface under it: the
//! cost is quantised by ring, so a band edge landing ON a row of faces rather
//! than between them is worth hundreds of triangles. Pass sets have to be
//! built and measured. This builds and measures them.
//!
//! One row per pass, walking [`FACE_PASSES`](symbios_avatar::face::refine_face)
//! from none to all of them, each row reporting what that pass added and what
//! the surface it left is like where the features are. **The cost and the
//! resolution together are the whole decision**: a pass that adds four thousand
//! triangles and halves the cell under the mouth is a different proposition
//! from one that adds four thousand and halves it under a cheek.
//!
//! ```text
//! cargo run --release --example refinecost           # the default body
//! cargo run --release --example refinecost -- 7 23   # named seeds
//! ```
//!
//! **The bands are measured once, off the fully refined body, and then held.**
//! A band defined per level would move as the head's own surface changed under
//! it, and the rows would not be comparable — which is the same reason the
//! refinement bands themselves are tied to the frame.
//!
//! **A cell is reported ACROSS and DOWN separately, because a single median is
//! wrong wherever it matters.** The
//! head's faces are not square: measured at the nose's dorsum on the shipped
//! body they run 3.42 mm across and 7.23 mm down, a 2:1 quad that
//! [`PolyMesh::refine_curved`] preserves, so every band holds two populations of
//! edge a factor of two apart. A median over both reports whichever one happens
//! to hold the middle, and a refinement that halves EVERY edge moves the count
//! balance rather than the median: a dorsum band that goes from 3.42/7.23 to
//! 1.71/3.64 reads `3.53 -> 3.58` as one median, and a nose base that goes
//! from 0.76/2.25 to 0.38/1.12 reads `0.83 -> 1.12`, WORSE.
//!
//! So each band reports the median of the edges that run across the face and the
//! median of the ones that run down it. Which one a question wants depends on
//! the question: `the_mouth_is_wider_than_the_mesh_under_it` counts cells ACROSS
//! the mouth, a ridge is held by the cell across the nose's dorsum, and a lip
//! term that is narrow in HEIGHT is held by the DOWN cell. Nothing here
//! recomputes a relief constant: the guard rails on how far a cut may go are
//! the tests that already hold them, and a report that restates the source is
//! not measuring the body.

use symbios_avatar::face::{
    Canon, HeadTraits, Skull, refine_face, refine_neck, shape_neck, shape_skull,
};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Vec3, Zone, build_cage,
    catmull_clark,
};

/// The largest number of passes the table walks to.
///
/// `FACE_PASSES` has nine entries and the crate asks for all nine, so a row
/// labelled 9 is the SHIPPED body and not one pass past it. Ten is one past:
/// `refine_face` repeats its tightest region rather than widening again, so that
/// row is what another pass over the mouth band would cost and buy — the
/// question a refinement proposal otherwise has to answer by hand.
const PASSES: usize = 11;

/// The bands each row reports a cell for.
///
/// Named for the feature rather than for a height, and measured off `Canon` so
/// they follow the feature stack rather than a raw skull radius. `lateral` is
/// whether the band is read out at the flank instead of down the feature's own
/// column: everything on the midline wants a narrow window, and the jaw flank
/// is the one region whose whole point is that it is not on the midline.
struct Band {
    name: &'static str,
    /// Millimetres above the head joint, low and high.
    span: (f32, f32),
    lateral: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seeds: Vec<i64> = args.iter().filter_map(|arg| arg.parse().ok()).collect();
    let seeds = if seeds.is_empty() { vec![-1] } else { seeds };

    for seed in seeds {
        let mut record = AvatarRecord::new("Refined", Archetype::default());
        if seed >= 0 {
            record.reroll(seed);
        }
        let skeleton = record.skeleton();
        let traits = HeadTraits::of(&record.composites);
        let Ok(cage) = build_cage(&skeleton, &CageConfig::default()) else {
            println!("seed {seed} does not mesh");
            continue;
        };
        let Ok(rig) = Rig::from_skeleton(&skeleton) else {
            continue;
        };
        let base = catmull_clark(&cage, BODY_SUBDIVISIONS);

        // The shipped body first, so the bands can be measured off it and then
        // held for every row. See the module note.
        let shipped = built(&base, &rig, &traits, PASSES - 2);
        let Some(skull) = Skull::measure(&shipped, &rig) else {
            continue;
        };
        let canon = Canon::measure(&rig, &skull, &record.eyes);
        let Some(head) = rig.in_zone(Zone::Head).first().copied() else {
            continue;
        };
        let centre = rig.joints[head].position;
        let radius = rig.joints[head].radius;

        let frame = canon.frame;
        let bands = [
            Band {
                name: "brow",
                span: (canon.level, canon.level + frame * 0.22),
                lateral: false,
            },
            Band {
                name: "nose dorsum",
                span: (canon.level - frame * 0.20, canon.level + frame * 0.12),
                lateral: false,
            },
            Band {
                name: "nose base",
                span: (canon.nose_base() - frame * 0.07, canon.nose_base()),
                lateral: false,
            },
            // **Split above and below the line, because the finest pass may
            // not cover both.** `FACE_PASSES`'s last band is 0.360 to 0.255 of
            // a profile height, and a lip stack is taller than the band is: a
            // single median over the whole mouth mixes cells the tightest pass
            // reached with cells it never touched, and reports the average of
            // two different surfaces as though it were one.
            Band {
                name: "upper lip",
                span: (canon.mouth_line(), canon.mouth_line() + frame * 0.13),
                lateral: false,
            },
            Band {
                name: "lower lip",
                span: (canon.mouth_line() - frame * 0.13, canon.mouth_line()),
                lateral: false,
            },
            Band {
                name: "jaw flank",
                span: (canon.chin(), canon.mouth_line()),
                lateral: true,
            },
        ];

        println!(
            "\n=== seed {seed} — head radius {:.1} mm, frame {:.1} mm, {} passes\n",
            radius * 1000.0,
            frame * 1000.0,
            PASSES - 2
        );
        print!("| pass | tris | added | ");
        for band in &bands {
            print!("{} | ", band.name);
        }
        println!();
        println!("|---|---|---|{}", "---|".repeat(bands.len()));

        let mut previous = 0usize;
        for level in 0..PASSES {
            let mesh = built(&base, &rig, &traits, level);
            let tris = mesh.triangulated().len();
            let added = tris.saturating_sub(previous);
            print!(
                "| {level} | {tris} | {} | ",
                if level == 0 {
                    "—".into()
                } else {
                    format!("{added:+}")
                }
            );
            for band in &bands {
                let (across, down) = cell(&mesh, centre, radius, band);
                let show = |edge: Option<f32>| match edge {
                    Some(edge) => format!("{:.2}", edge * 1000.0),
                    None => "—".into(),
                };
                print!("{} / {} | ", show(across), show(down));
            }
            println!();
            previous = tris;
        }
        println!(
            "\n`tris` is the whole BODY's triangles, which is what the budget counts; `added` is \
             this pass's own cost. Cells are millimetres, `across / down`: the median head-owned \
             edge in the band running across the face and running down it. **Row 8 is the body \
             the crate ships**; row 9 is one more pass over the tightest band, which is what \
             `refine_face` does when it is asked for more than `FACE_PASSES` names."
        );
    }
}

/// The head as [`crate::build_body`] builds it, with a given number of face
/// refinement passes.
///
/// The shipped order, and all of it: refine the face, refine the neck, shape the
/// skull, shape the neck. Leaving the shaping off would be quicker and would
/// measure a sphere — the cells this reports are the cells the relief carve
/// actually lands on, and `shape_skull` is an anisotropic scaling that moves
/// them.
fn built(base: &PolyMesh, rig: &Rig, traits: &HeadTraits, levels: usize) -> PolyMesh {
    let mut mesh = refine_face(base, rig, levels);
    mesh = refine_neck(&mesh, rig, traits);
    shape_skull(&mut mesh, rig, traits);
    shape_neck(&mut mesh, rig, traits);
    mesh
}

/// The median edge of the head's own faces in a band, across and then down, in
/// metres.
///
/// Front-facing only, and windowed across: the narrow passes of `FACE_PASSES`
/// reach in to a cosine of 0.92 — about 23 degrees off dead ahead — so a window
/// half a head wide takes its median from the CHEEK and reports a nose as four
/// times coarser than the mesh it is drawn on.
///
/// **Split by direction because the faces are 2:1 and one median over both is a
/// reading of which population is larger** — see the module note.
fn cell(mesh: &PolyMesh, centre: Vec3, radius: f32, band: &Band) -> (Option<f32>, Option<f32>) {
    let mut across_face: Vec<f32> = Vec::new();
    let mut down_face: Vec<f32> = Vec::new();
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let a = mesh.positions[face[corner] as usize];
            let b = mesh.positions[face[(corner + 1) % face.len()] as usize];
            let middle = 0.5 * (a + b) - centre;
            if middle.z <= radius * 0.20 {
                continue;
            }
            let across = middle.x.abs();
            let wanted = if band.lateral {
                across > radius * 0.35
            } else {
                across < radius * 0.20
            };
            if !wanted || middle.y < band.span.0 || middle.y > band.span.1 {
                continue;
            }
            // An edge belongs to the direction it mostly runs in. The face is a
            // quad off a subdivided tube, so every edge is close to one axis or
            // the other and the split is not a close call.
            let step = b - a;
            let length = a.distance(b);
            if step.y.abs() > (step.x * step.x + step.z * step.z).sqrt() {
                down_face.push(length);
            } else {
                across_face.push(length);
            }
        }
    }
    across_face.sort_by(f32::total_cmp);
    down_face.sort_by(f32::total_cmp);
    let median = |edges: &[f32]| edges.get(edges.len() / 2).copied();
    (median(&across_face), median(&down_face))
}
