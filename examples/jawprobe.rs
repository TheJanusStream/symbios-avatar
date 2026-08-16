//! Where the submental bulge comes from, on a ruler that cannot move.
//!
//! The shipped guard — `tests/parts::the_underside_of_the_jaw_does_not_bulge` —
//! measures the underside against the chord joining two MEASURED landmarks, the
//! chin's crest and the throat. That is the right contract and the wrong
//! instrument for attribution: any change to the profiles moves the ends of the
//! thing measuring them, so a term can be zeroed and the number improve because
//! the ruler moved rather than because the shape did.
//!
//! **A moving ruler contaminates every attribution taken off it.** On the
//! moving chord, deleting `CHIN`'s whole below-joint tail still leaves +3.4 mm
//! of bulge — a reading that sends the search after `DEPTH`. On a fixed ruler
//! the same experiment reads −0.4 mm: with no chin there is no crest for
//! `chin_of` to find, so the chord starts somewhere else entirely.
//!
//! So this measures between two FIXED heights instead — 0.40 and 0.09 along the
//! neck-to-head bone, which `rig::skin::owner_of` records as where the chin and
//! the throat floor sit on every body. Terms can be zeroed and read honestly.
//!
//! It reports three stages, so each column differs from the last by exactly one
//! thing: the subdivided and refined cage, the same with `shape_skull`, and the
//! surface that ships.
//!
//! ```text
//! cargo run --release --example jawprobe
//! ```

use symbios_avatar::face::Skull;
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Vec3, Zone, build_body,
    build_cage, catmull_clark, face,
};

/// The chin's projection and the throat's floor, as shares of the neck-to-head
/// bone.
///
/// Not measured here on purpose: these are the constants `rig::skin::owner_of`
/// derives its own boundary from, and using them makes this ruler the same one
/// the binding is written against. They are stable across the parameter space —
/// `shape` puts the head's floor at 0.896 of the below-joint extent and the chin
/// at 0.599, measured to three figures at two very different values of
/// `HEAD_BELOW_JOINT`.
const CHIN_ALONG: f32 = 0.40;
const THROAT_ALONG: f32 = 0.09;

/// How many refinement passes the unshaped stage gets, matching `build_body`.
const REFINEMENT: usize = 2;

fn main() {
    println!("the underside of the jaw, against a chord between two FIXED heights");
    println!("  positive is a bulge; a jawline should be straight to slightly hollow\n");
    for seed in [0i64, 3, 6, 12] {
        let mut record = AvatarRecord::new("Jaw", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let Ok(rig) = Rig::from_skeleton(&skeleton) else {
            continue;
        };
        let Some(&head) = rig.in_zone(Zone::Head).first() else {
            continue;
        };
        let Some(neck) = rig.joints[head].parent else {
            continue;
        };
        let (head_y, neck_y) = (rig.joints[head].position.y, rig.joints[neck].position.y);
        let bone = head_y - neck_y;
        let (top, bottom) = (neck_y + CHIN_ALONG * bone, neck_y + THROAT_ALONG * bone);

        let Ok(cage) = build_cage(&skeleton, &CageConfig::default()) else {
            continue;
        };
        let plain = face::refine_face(&catmull_clark(&cage, BODY_SUBDIVISIONS), &rig, REFINEMENT);
        let Ok(shaped) = build_body(
            &skeleton,
            &CageConfig::default(),
            BODY_SUBDIVISIONS,
            &Default::default(),
        ) else {
            continue;
        };
        let mut carved = shaped.clone();
        if let Some(skull) = Skull::measure(&shaped, &rig) {
            let canon = face::Canon::measure(&rig, &skull, &Default::default());
            face::carve_face(&mut carved, &rig, &canon, &Default::default());
        }

        println!("seed {seed}: the bone is {:.1} mm long", bone * 1000.0);
        for (name, mesh) in [("cage ", &plain), ("shape", &shaped), ("carve", &carved)] {
            match run(mesh, rig.joints[head].position, top, bottom) {
                Some((worst, hollow, shelf, shape)) => println!(
                    "  {name}: bulge {worst:+6.1} hollow {hollow:+7.1} shelf {shelf:.3} | {shape}"
                ),
                None => println!("  {name}: the midline is not inside this surface"),
            }
        }
    }
}

/// What share of the submental drop has happened a fifth of the way down.
///
/// The figure that discriminates, where the chord's worst does not.
///
/// **The hollow is not the defect, however loudly the chord says it is.** A
/// chord from the chin's tip to the throat cuts straight across a jaw, so ANY
/// real jaw is deeply hollow against it — measured off `examples/column`, the
/// CC0 reference reads −21.7 mm at −20 below its own chin on exactly this
/// ruler, and our body reads −23.2 there. Within a millimetre and a half of the
/// reference at every height below −20, and the same at −30 and −40. There was
/// never anything wrong with the hollow.
///
/// What differs is the first ten millimetres. The reference holds its forward
/// reach under the chin and then cuts away hard — 98.6 to 89.0 to 53.6 — a
/// shelf and then a cliff. Ours gives up twice as much immediately and then
/// less: 102.5 to 82.8 to 54.7, and the same on every seed measured. So the
/// figure worth watching is how much of the total drop has happened by a fifth
/// of the way down: **0.165 on the reference, about 0.32 on ours**.
///
/// **The reference target for it lives in `examples/column` and not here**, and
/// the two are not interchangeable: this file's chord runs between two FIXED
/// heights, deliberately, so that a change to the profiles cannot move the
/// ruler under it, while `column` measures each body from its own chin so two
/// statures compare at the same anatomy. A share taken over one span is not the
/// same number as a share over the other, and quoting the reference's 0.165
/// beside this column would be comparing them. What this one is for is
/// ATTRIBUTION — the cage row against the shape row against the carve row — and
/// for that it only has to be the same ruler three times.
///
/// Reports the forward deviation from the chord joining the two heights, sampled
/// down it, as `(worst bulge, worst hollow, shelf share, the profile)`.
///
/// Bisected against `PolyMesh::contains`, never binned — binning the
/// furthest-forward vertex per band reports millimetres of ripple that are not
/// in the mesh, which is the note `examples/headaudit` opens with.
fn run(mesh: &PolyMesh, at: Vec3, top: f32, bottom: f32) -> Option<(f32, f32, f32, String)> {
    let reach = |y: f32| -> Option<f32> {
        if !mesh.contains(Vec3::new(at.x, y, at.z)) {
            return None;
        }
        let (mut inside, mut outside) = (at.z, at.z + 0.40);
        for _ in 0..32 {
            let middle = 0.5 * (inside + outside);
            if mesh.contains(Vec3::new(at.x, y, middle)) {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        Some(inside)
    };
    let (top_z, bottom_z) = (reach(top)?, reach(bottom)?);
    let mut worst = f32::MIN;
    let mut hollow = f32::MAX;
    let mut shelf = None;
    let mut shape = String::new();
    for step in 1..20 {
        let t = step as f32 / 20.0;
        let Some(z) = reach(top + (bottom - top) * t) else {
            continue;
        };
        let out = (z - (top_z + (bottom_z - top_z) * t)) * 1000.0;
        shape.push_str(&format!("{out:+.1} "));
        worst = worst.max(out);
        hollow = hollow.min(out);
        // The share of the whole submental drop that has already happened a
        // fifth of the way down. See [`SHELF`].
        if step == 4 {
            shelf = Some((top_z - z) / (top_z - bottom_z).max(f32::EPSILON));
        }
    }
    (worst > f32::MIN).then_some((worst, hollow, shelf.unwrap_or(0.0), shape))
}
