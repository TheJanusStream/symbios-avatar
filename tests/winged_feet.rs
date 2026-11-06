//! Winged bodies' feet (#422): a bird's four toes on the bird's formula,
//! anisodactyl or zygodactyl, a bat's five near-equal hooked ones, a
//! glider's paws with its hallux turned out; the toes are the foot, never
//! the leg's chain or its contact, and they rest on the ground.

use symbios_avatar::plan::{Blended, Lineage};
use symbios_avatar::{
    Archetype, Avatar, AvatarRecord, Limb, QuadrupedParams, Rig, Role, Toes, Vec3, WingedFamily,
    WingedParams, Zone,
};

/// Every family, and a cross of each pair at both sides of the halfway blend.
fn lineages() -> Vec<(String, Lineage<WingedFamily>)> {
    use WingedFamily::{Bat, Bird, Glider};
    let mut all: Vec<(String, Lineage<WingedFamily>)> = [Bird, Bat, Glider]
        .into_iter()
        .map(|family| (format!("{family:?}"), Lineage::pure(family)))
        .collect();
    for (a, b) in [(Bird, Bat), (Bat, Glider), (Glider, Bird)] {
        for blend in [0.3, 0.7] {
            all.push((
                format!("{a:?} x {b:?} {blend}"),
                Lineage::cross(a, b, blend),
            ));
        }
    }
    all
}

fn record(lineage: Lineage<WingedFamily>) -> AvatarRecord {
    let mut params = WingedParams::default();
    params.set_lineage(lineage);
    AvatarRecord::new("Winged", Archetype::Winged(params))
}

fn built(lineage: Lineage<WingedFamily>) -> Avatar {
    Avatar::build_for_shape(&record(lineage)).expect("a winged body builds")
}

/// The beak hook a bird stands on `toes` with (`WingedParams::toes`): the
/// owl's, `0`, grips zygodactyl and the crow's, `-1`, stands anisodactyl.
fn hook_for(toes: Toes) -> f32 {
    match toes {
        Toes::Anisodactyl => -1.0,
        Toes::Zygodactyl => 0.0,
    }
}

/// `lineage` built with its beak hooked `hook`.
fn built_hooked(lineage: Lineage<WingedFamily>, hook: f32) -> Avatar {
    let mut record = record(lineage);
    if let Archetype::Winged(params) = &mut record.archetype {
        params.beak_hook = hook;
    }
    Avatar::build_for_shape(&record).expect("a winged body builds")
}

/// One toe: its joints from the root to the claw's tip.
struct Toe {
    joints: Vec<usize>,
}

impl Toe {
    fn bones(&self) -> usize {
        self.joints.len() - 1
    }

    /// Its length along its bones.
    fn length(&self, rig: &Rig) -> f32 {
        self.joints
            .windows(2)
            .map(|pair| {
                rig.joints[pair[0]]
                    .position
                    .distance(rig.joints[pair[1]].position)
            })
            .sum()
    }

    /// Which way its first bone points across the ground.
    fn heading(&self, rig: &Rig) -> Vec3 {
        let run = rig.joints[self.joints[1]].position - rig.joints[self.joints[0]].position;
        Vec3::new(run.x, 0.0, run.z).normalize_or_zero()
    }
}

/// The toes grown on `limb`'s foot: every chain of digit joints in its
/// extremity zone that hangs from a joint of the body's own.
fn toes(rig: &Rig, limb: Limb) -> Vec<Toe> {
    let digit = |joint: usize| rig.joints[joint].role == Role::Digit;
    let owned = |joint: usize| rig.wings.as_ref().is_some_and(|wings| wings.owns(joint));
    let zone = rig.in_zone(Zone::Extremity(limb));
    zone.iter()
        .copied()
        .filter(|&joint| digit(joint) && !owned(joint))
        .filter(|&joint| {
            rig.joints[joint]
                .parent
                .is_some_and(|parent| !digit(parent))
        })
        .map(|root| {
            let mut joints = vec![root];
            while let Some(next) = (0..rig.joints.len())
                .find(|&child| rig.joints[child].parent == joints.last().copied())
            {
                joints.push(next);
            }
            Toe { joints }
        })
        .collect()
}

/// Which way a foot points: from its pad to the plan's second node.
fn forward(rig: &Rig, limb: Limb) -> Vec3 {
    let zone = rig.in_zone(Zone::Extremity(limb));
    let run = rig.joints[zone[1]].position - rig.joints[zone[0]].position;
    Vec3::new(run.x, 0.0, run.z).normalize_or_zero()
}

/// **A bird stands on four toes, phalanges 2, 3, 4 and 5 counting the claw**
/// (research-10 3.1, S17), the hallux back on every bird and the fourth with
/// it on a zygodactyl one.
#[test]
fn a_bird_stands_on_four_toes_on_the_bird_formula_either_way_round() {
    for (toes_asked, back) in [(Toes::Anisodactyl, 1), (Toes::Zygodactyl, 2)] {
        let avatar = built_hooked(Lineage::pure(WingedFamily::Bird), hook_for(toes_asked));
        let rig = &avatar.rig;
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let foot = toes(rig, limb);
            let mut formula: Vec<usize> = foot.iter().map(Toe::bones).collect();
            formula.sort_unstable();
            assert_eq!(formula, vec![2, 3, 4, 5], "{toes_asked:?} {limb:?}");
            let ahead = forward(rig, limb);
            let behind: Vec<usize> = foot
                .iter()
                .filter(|toe| toe.heading(rig).dot(ahead) < -0.5)
                .map(Toe::bones)
                .collect();
            assert_eq!(
                behind.len(),
                back,
                "{toes_asked:?} {limb:?}: {behind:?} back"
            );
            // The hallux - two bones - is back either way, and opposes the
            // middle toe: four bones, straight ahead.
            assert!(
                behind.contains(&2),
                "{toes_asked:?}: the hallux is not back"
            );
            if toes_asked == Toes::Zygodactyl {
                assert!(behind.contains(&5), "the fourth toe is not back");
            }
            let middle = foot
                .iter()
                .find(|toe| toe.bones() == 4)
                .expect("a third toe");
            assert!(
                middle.heading(rig).dot(ahead) > 0.9,
                "{toes_asked:?}: III is not ahead"
            );
        }
    }
    // The default bird is an owl, whose hooked beak grips with two toes back;
    // a crow's straighter one stands on one, as about nine birds in ten do.
    for (hook, back, what) in [
        (None, 2, "the default owl"),
        (Some(-1.0), 1, "a crow's beak"),
    ] {
        let avatar = match hook {
            Some(hook) => built_hooked(Lineage::pure(WingedFamily::Bird), hook),
            None => built(Lineage::pure(WingedFamily::Bird)),
        };
        let rig = &avatar.rig;
        let ahead = forward(rig, Limb::HindLeft);
        let behind = toes(rig, Limb::HindLeft)
            .iter()
            .filter(|toe| toe.heading(rig).dot(ahead) < -0.5)
            .count();
        assert_eq!(behind, back, "{what}: {behind} toes back");
    }
}

/// **A bat hangs from five near-equal toes with hooked claws**, the hallux
/// with two bones and the others three; and its fore "feet", its wrists' pads
/// and thumbs, grow none.
#[test]
fn a_bat_stands_on_five_near_equal_hooked_toes() {
    let avatar = built(Lineage::pure(WingedFamily::Bat));
    let rig = &avatar.rig;
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let foot = toes(rig, limb);
        let mut formula: Vec<usize> = foot.iter().map(Toe::bones).collect();
        formula.sort_unstable();
        assert_eq!(formula, vec![2, 3, 3, 3, 3], "{limb:?}");
        let lengths: Vec<f32> = foot.iter().map(|toe| toe.length(rig)).collect();
        let (short, long) = lengths
            .iter()
            .fold((f32::MAX, 0.0f32), |(lo, hi), &l| (lo.min(l), hi.max(l)));
        assert!(
            short > 0.85 * long,
            "{limb:?}: toes from {short} to {long} m"
        );
        for toe in &foot {
            // The claw, the last bone, comes down off the toe to the ground:
            // its tip below its base, the claw lifted off the toe's line just
            // enough that it rests there (how far it curls is the layout's
            // own test, `plan::winged_foot`).
            let [.., base, tip] = toe.joints[..] else {
                panic!("a toe of {} joints", toe.joints.len());
            };
            let claw_way = (rig.joints[tip].position - rig.joints[base].position).normalize();
            assert!(
                claw_way.y < -0.15,
                "{limb:?}: a claw that does not come down ({claw_way})"
            );
        }
    }
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        assert!(toes(rig, limb).is_empty(), "{limb:?}: a wrist grew toes");
    }
}

/// **A glider's hind paw carries its hallux turned out on the inside** (S31):
/// its paws are a beast's analytic foot, so the toes grow no joints, and the
/// paw reaches further to the inside than to the outside.
#[test]
fn a_glider_s_hind_paw_has_its_hallux_on_the_inside() {
    let avatar = built(Lineage::pure(WingedFamily::Glider));
    let rig = &avatar.rig;
    let parts = &avatar.parts;
    for limb in Limb::ALL {
        assert!(
            toes(rig, limb).is_empty(),
            "{limb:?}: a glider grew drawn toes"
        );
    }
    for limb in [Limb::HindLeft, Limb::HindRight] {
        let pad = rig.in_zone(Zone::Extremity(limb))[0];
        let centre = rig.joints[pad].position;
        let radius = rig.joints[pad].radius;
        let outward = Vec3::X * centre.x.signum();
        let (mut inside, mut outside) = (0.0f32, 0.0f32);
        for (vertex, &zone) in parts.zones.iter().enumerate() {
            let at = parts.body.positions[vertex];
            if zone != Zone::Extremity(limb) || at.y > 1.2 * radius {
                continue;
            }
            let across = (at - centre).dot(outward);
            inside = inside.max(-across);
            outside = outside.max(across);
        }
        assert!(
            inside > outside + 0.25 * radius,
            "{limb:?}: the paw reaches {inside:.4} m in and {outside:.4} m out"
        );
    }
}

/// **The toes are the foot, and the leg is the plan's**: every limb's contact
/// is its plan's own first extremity joint and its chain the three joints
/// above it, as on the rig the skeleton roots before anything is grown;
/// every drawn toe's joint is in the foot `Rig::extremity_joints` hands the
/// gait, no wing's is; and the foot's extent along its toes reaches past what
/// it was without them.
#[test]
fn the_toes_are_the_foot_and_never_the_leg_or_its_contact() {
    let mut toed = 0;
    for (name, lineage) in lineages() {
        let record = record(lineage);
        let avatar = Avatar::build_for_shape(&record).expect("a winged body builds");
        let rig = &avatar.rig;
        let bare = Rig::from_skeleton(&record.skeleton()).expect("the skeleton rigs");
        for limb in Limb::ALL {
            let Some(&contact) = rig.in_zone(Zone::Extremity(limb)).first() else {
                continue;
            };
            assert_eq!(
                Some(&contact),
                bare.in_zone(Zone::Extremity(limb)).first(),
                "{name}: {limb:?}'s contact moved"
            );
            assert_eq!(rig.joints[contact].role, Role::Deform, "{name}: {limb:?}");
            assert_eq!(
                rig.limb_chain(limb),
                bare.limb_chain(limb),
                "{name}: {limb:?}'s chain moved"
            );
            let foot = rig.extremity_joints(limb);
            let grown = toes(rig, limb);
            if grown.is_empty() {
                continue;
            }
            toed += 1;
            // Grown after the wings resolved, so no toe is listed among the
            // wings' own joints - which `extremity_joints` leaves out of a foot.
            if let Some(wings) = &rig.wings {
                let last = wings.joints.iter().copied().max().unwrap_or(0);
                for &joint in grown.iter().flat_map(|toe| &toe.joints) {
                    assert!(
                        !wings.owns(joint) && joint > last,
                        "{name}: {limb:?}'s toe joint {joint} is among the wings' joints or before them"
                    );
                }
            }
            for toe in &grown {
                assert!(
                    toe.joints.iter().all(|joint| foot.contains(joint)),
                    "{name}: {limb:?}'s foot leaves out a toe"
                );
                assert!(
                    rig.limb_chain(limb)
                        .is_some_and(|chain| chain.iter().all(|joint| !toe.joints.contains(joint))),
                    "{name}: {limb:?}'s chain reaches a toe"
                );
            }
            let ahead = forward(rig, limb);
            let reach = |rig: &Rig| {
                let (behind, ahead) = rig.extremity_extent(limb, ahead).expect("a foot");
                behind + ahead
            };
            assert!(
                reach(rig) > reach(&bare) * 1.3,
                "{name}: {limb:?}'s foot spans {:.3} m with its toes, {:.3} without",
                reach(rig),
                reach(&bare)
            );
        }
    }
    assert!(toed >= 10, "only {toed} feet grew toes");
}

/// **The toes rest on the ground without passing through it**: at rest the
/// lowest point of every drawn foot is within a millimetre of the ground,
/// and every claw's tip off it by less than a toe's thickness.
#[test]
fn the_toes_rest_on_the_ground_and_not_in_it() {
    for (name, lineage, toes_asked) in lineages()
        .into_iter()
        .map(|(name, lineage)| (name, lineage, None))
        .chain([(
            "anisodactyl Bird".to_string(),
            Lineage::pure(WingedFamily::Bird),
            Some(Toes::Anisodactyl),
        )])
    {
        let avatar = match toes_asked {
            Some(toes) => built_hooked(lineage, hook_for(toes)),
            None => built(lineage),
        };
        let rig = &avatar.rig;
        for part in &avatar.parts.extremities.feet {
            let at = rig.joints[part.joint].position;
            let lowest = part
                .mesh
                .positions
                .iter()
                .map(|point| point.y + at.y)
                .fold(f32::MAX, f32::min);
            assert!(
                (-0.001..0.001).contains(&lowest),
                "{name}: {:?}'s toes reach down to {lowest:.4} m",
                part.limb
            );
            for toe in toes(rig, part.limb) {
                let tip = rig.joints[*toe.joints.last().expect("a toe")].position.y;
                assert!(
                    (0.0..0.01).contains(&tip),
                    "{name}: a claw's tip at {tip:.4} m"
                );
            }
        }
    }
}

/// **Every family and cross builds its feet clean**: a drawn foot is closed
/// tubes, every vertex held by at most four of its own toes' joints summing
/// to one, mirrored left to right; and no person or beast grows a toe.
#[test]
fn every_family_and_cross_builds_its_feet_clean() {
    for (name, lineage) in lineages() {
        let avatar = built(lineage);
        let rig = &avatar.rig;
        let drawn = lineage.leading() != WingedFamily::Glider;
        assert_eq!(
            avatar.parts.extremities.feet.len(),
            if drawn { 2 } else { 0 },
            "{name}: drawn feet"
        );
        for part in &avatar.parts.extremities.feet {
            assert!(part.mesh.is_closed_manifold(), "{name}: a toe has a hole");
            assert!(
                part.mesh.positions.iter().all(|point| point.is_finite()),
                "{name}: a toe's vertex is not finite"
            );
            for skin in &part.mesh.skin {
                let held: Vec<_> = skin.iter().filter(|hold| hold.weight > 0.0).collect();
                let sum: f32 = held.iter().map(|hold| hold.weight).sum();
                assert!((sum - 1.0).abs() < 1.0e-4, "{name}: weights sum to {sum}");
                assert!(
                    held.iter().all(|hold| {
                        let joint = &rig.joints[usize::from(hold.joint)];
                        joint.role == Role::Digit && joint.zone == Zone::Extremity(part.limb)
                    }),
                    "{name}: a toe held by a joint not its foot's"
                );
            }
        }
        if let [left, right] = &avatar.parts.extremities.feet[..] {
            let place = |part: &symbios_avatar::extremity::Attached| -> Vec<Vec3> {
                let at = rig.joints[part.joint].position;
                part.mesh
                    .positions
                    .iter()
                    .map(|&point| point + at)
                    .collect()
            };
            let (left, right) = (place(left), place(right));
            let worst = left
                .iter()
                .map(|point| {
                    let mirrored = Vec3::new(-point.x, point.y, point.z);
                    right
                        .iter()
                        .map(|other| other.distance(mirrored))
                        .fold(f32::MAX, f32::min)
                })
                .fold(0.0f32, f32::max);
            assert!(worst < 1.0e-4, "{name}: the feet differ by {worst} m");
        }
    }
    let mut beast = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
    beast.reroll(3);
    for record in [AvatarRecord::default(), beast] {
        let avatar = Avatar::build_for_shape(&record).expect("it builds");
        for limb in [Limb::HindLeft, Limb::HindRight] {
            assert!(
                toes(&avatar.rig, limb).is_empty(),
                "a {limb:?} toe on a non-winged body"
            );
        }
    }
}
