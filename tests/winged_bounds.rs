//! The per-frame bounds a winged body's motion is held to (#424), shared by
//! `winged_flight`, `winged_rests`, `winged_motion` and `winged_life`: how far its joints move
//! into a frame, where a frame pops against its neighbours, and how low its
//! whole drawn body and its drawn toes reach.

use symbios_avatar::{Avatar, MeshKind, Pose, Vec3};

/// How far each joint moved from `before` to `now`, the most of them.
pub fn furthest(before: &[Vec3], now: &[Vec3]) -> f32 {
    now.iter()
        .zip(before)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f32::max)
}

/// Every frame at which some joint moved more than twice the most it moved
/// in the four frames either side, and `slack`: a pop, there or back,
/// wherever in a run of even frames it falls. Each is the frame, how far it
/// moved and the most around it.
pub fn pops(joints: &[Vec<Vec3>], slack: f32) -> Vec<(usize, f32, f32)> {
    pops_over(joints, &vec![1.0; joints.len()], 1.0, slack)
}

/// [`pops`] over frames of uneven length, `deltas` each frame's: every
/// frame's travel read as a `reference` frame's at the speed it moved, so a
/// long frame among short ones is held to what its neighbours move in as
/// long, and `slack` is a reference frame's. Over even frames of the
/// reference's length it is [`pops`] to the bit.
pub fn pops_over(
    joints: &[Vec<Vec3>],
    deltas: &[f32],
    reference: f32,
    slack: f32,
) -> Vec<(usize, f32, f32)> {
    let steps: Vec<f32> = (1..joints.len())
        .map(|at| {
            let moved = furthest(&joints[at - 1], &joints[at]);
            if deltas[at] == reference {
                moved
            } else {
                moved * reference / deltas[at]
            }
        })
        .collect();
    (4..steps.len().saturating_sub(4))
        .filter_map(|at| {
            let around = (at - 4..at)
                .chain(at + 1..=at + 4)
                .map(|frame| steps[frame])
                .fold(0.0f32, f32::max);
            (steps[at] > around * 2.0 + slack).then_some((at + 1, steps[at], around))
        })
        .collect()
}

/// The lowest point of everything drawn of the body that can meet the
/// ground - its skin with its toes and membrane, and its flight feathers -
/// in `pose`, in metres above the ground.
pub fn lowest_drawn(avatar: &Avatar, pose: &Pose) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    let mut low = f32::MAX;
    for mesh in avatar
        .meshes
        .iter()
        .filter(|mesh| mesh.kind == MeshKind::Skin)
        .map(|mesh| &mesh.mesh)
        .chain(avatar.parts.wings.as_ref().map(|wings| &wings.feathers))
    {
        let moved = posed.deform_mesh(rig, mesh);
        low = moved.positions.iter().map(|at| at.y).fold(low, f32::min);
    }
    low
}

/// How far below the ground, past whatever stood below it before the motion
/// began, any drawn point may go: a few millimetres.
pub const UNDER_GROUND: f32 = 0.006;

/// The lowest point of every drawn toe, posed, over the level ground.
pub fn lowest_toe(avatar: &Avatar, pose: &Pose) -> f32 {
    let rig = &avatar.rig;
    let posed = pose.forward(rig);
    avatar
        .parts
        .extremities
        .feet
        .iter()
        .flat_map(|part| {
            let at = rig.joints[part.joint].position;
            let mut mesh = part.mesh.clone();
            for point in &mut mesh.positions {
                *point += at;
            }
            posed.deform_mesh(rig, &mesh).positions
        })
        .map(|point| point.y)
        .fold(f32::MAX, f32::min)
}
