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

/// **Held short, a share stops at the floor** (#427): a tenth at a time from
/// a whole turn, it comes to [`SHARE_FLOOR`] and no lower. Multiplied by
/// 0.9 while above it, it stopped at about 0.185.
#[test]
fn a_share_held_short_stops_at_the_floor() {
    let mut share = 1.0f32;
    let mut steps = 0;
    while share > SHARE_FLOOR {
        share = held_short(share);
        steps += 1;
        assert!(steps < 100, "never reached the floor");
    }
    assert_eq!(share, SHARE_FLOOR, "a share of {share}, below the floor");
}

/// **A pair that can be trimmed no further is passed over for good**
/// (#427): thirty such pairs ahead of one that takes forty trims, and it is
/// mended within the loop's sixty rounds. Retried after every trim, each of
/// them spent a round again, and the pair behind them was trimmed once.
#[test]
fn exhausted_pairs_cost_the_loop_no_rounds() {
    // Each pair and how many trims it still takes; the first thirty take
    // none they can be given.
    let mut left: std::collections::BTreeMap<u32, u32> = (0..30).map(|pair| (pair, 1)).collect();
    left.insert(100, 40);
    let torn = |left: &std::collections::BTreeMap<u32, u32>| -> Vec<u32> {
        left.iter()
            .filter(|&(_, &trims)| trims > 0)
            .map(|(&pair, _)| pair)
            .collect()
    };
    let mended = mend(&mut left, torn, |left, pair| {
        if pair < 30 {
            return false;
        }
        *left.get_mut(&pair).expect("a pair") -= 1;
        true
    });
    assert!(mended);
    assert_eq!(
        left[&100],
        0,
        "the mendable pair was trimmed {} times",
        40 - left[&100]
    );
}

/// **An extension, a spread or a stroke that is not a number poses the
/// fold** (#427): `f32::clamp` passes a NaN through, and every driven joint
/// came out NaN.
#[test]
fn a_non_finite_extension_reads_as_the_fold() {
    let drive = Drive {
        joint: 0,
        anchor: None,
        spread: Rot::y(1.0),
        from: 0.0,
        to: 1.0,
    };
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let at = drive.at(bad);
        assert!(at.is_finite(), "{bad}: {at:?}");
        assert!(at.angle_between(Quat::IDENTITY) < 1e-6, "{bad}: {at:?}");
        assert_eq!(unit(bad), 0.0);
        assert_eq!(finite(bad), 0.0);
    }
    assert_eq!(unit(0.4), 0.4);
    assert_eq!(unit(1.7), 1.0);
}
