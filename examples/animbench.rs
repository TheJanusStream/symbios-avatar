//! Times one frame of the walking path, which is what #323's routing had to
//! pay for and had to prove it could afford.
//!
//! The anim path's transcendentals go through `det` (pure-Rust `libm`) rather
//! than the platform's, so every measurement is reproducible across toolchain
//! builds — and the cost of that is paid per frame per body, wasm included.
//! This instrument prices it: the whole engine entry point ([`Walk::drive`]
//! with the footing on, the way overlands runs it) on the default body, level
//! ground, at a walking pace, wall-clocked over enough frames for the figure
//! to settle. Run it `--release`, on the tree before and after a routing
//! change; the difference is the routing's bill and nothing else's.
//!
//! The figure to watch is nanoseconds per frame. A world of forty bodies at
//! 60 Hz has a whole-frame budget of 16.7 ms, so a walk costing `n` ns/frame
//! spends `40n` ns of it on gaits.

use std::hint::black_box;
use std::time::Instant;

use symbios_avatar::anim::Ground;
use symbios_avatar::{Archetype, Avatar, AvatarRecord, Gait, Pose, Stride, Walk};

use glam::Vec3;

/// Enough that the per-frame figure is stable to a few nanoseconds; a full
/// run is still well under a second.
const FRAMES: u32 = 20_000;

/// One warmup pass the clock never sees, for the caches.
const WARMUP: u32 = 2_000;

fn main() {
    let record = AvatarRecord::new("Bench", Archetype::default());
    let avatar = Avatar::build(&record).expect("the default record builds");
    let rig = &avatar.rig;
    let gait = Gait::natural(rig);
    let stride = Stride::for_body(rig, 1.0);

    let drive = |frames: u32| {
        let mut pose = Pose::rest(rig);
        for frame in 0..frames {
            let cycle = f64::from(frame) as f32 / 60.0;
            let walked = Walk::at(cycle).drive(rig, &mut pose, &gait, &stride, |foot| {
                Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)))
            });
            black_box(&pose);
            black_box(walked.lift);
        }
    };

    drive(WARMUP);
    let started = Instant::now();
    drive(FRAMES);
    let elapsed = started.elapsed();

    let per_frame = elapsed.as_nanos() / u128::from(FRAMES);
    println!("{FRAMES} frames in {elapsed:.2?}: {per_frame} ns/frame");
}
