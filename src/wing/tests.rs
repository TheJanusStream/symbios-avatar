//! The wing parts' own checks (#421); the whole-body ones are in
//! `tests/wings.rs`.

use super::*;

#[test]
fn a_drive_is_the_fold_at_its_start_and_the_spread_at_its_end() {
    let spread = Rot::y(1.0);
    let drive = Drive {
        joint: 0,
        anchor: None,
        spread,
        from: 0.2,
        to: 0.8,
    };
    assert!(drive.at(0.0).angle_between(Quat::IDENTITY) < 1e-5);
    assert!(drive.at(0.2).angle_between(Quat::IDENTITY) < 1e-5);
    assert!(drive.at(0.8).angle_between(spread) < 1e-4);
    assert!(drive.at(1.0).angle_between(spread) < 1e-4);
}

#[test]
fn a_hold_is_cut_to_four_shares_summing_to_one() {
    let hold = Hold(vec![
        (1, 0.1),
        (2, 0.4),
        (3, -0.2),
        (4, 0.3),
        (5, 0.25),
        (6, 0.05),
    ]);
    let skin = hold.cut();
    let total: f32 = skin.iter().map(|influence| influence.weight).sum();
    assert!((total - 1.0).abs() < 1e-6);
    assert!(skin.iter().all(|influence| influence.weight >= 0.0));
    let joints: Vec<u16> = skin.iter().map(|influence| influence.joint).collect();
    assert_eq!(joints, [2, 4, 5, 1], "the four largest, largest first");
}

#[test]
fn a_frame_turn_carries_one_frame_onto_another() {
    let rest = frame(Vec3::X, Vec3::Y);
    let spread = frame(Vec3::Z, Vec3::X);
    let q = turn(rest, spread);
    assert!((q * Vec3::X - Vec3::Z).length() < 1e-5);
    assert!((q * Vec3::Y - Vec3::X).length() < 1e-5);
}
