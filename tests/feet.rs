//! The feet: their proportions, their wedge, their bend toward the big toe,
//! their toe-out, and the stance they rest in.
//!
//! **Guards fitted AFTER the geometry was agreed by render** — milestone #10's
//! standing method (#304). #305 re-measured the foot's delivered width (every
//! foot had been 45% too wide and half again too thick) and stood the feet
//! apart; #306 carved the cage's level run into a wedge with a medial bend
//! and turned each foot out seven degrees. Every bound here was fitted to
//! that tree and checked against the tree before it: the pre-feet body reads
//! width/length 0.506, toe over heel depth 0.56, no bend and no toe-out, and
//! each of those is outside the bound that guards it.
//!
//! **Everything is read in the foot's OWN frame** — along the heel-to-toe
//! axis the rig places and across it — because a toed-out foot's width is
//! not its extent in `x`: seven degrees over 280 mm adds 34 mm to that, which
//! is a third of the foot. `examples/footaudit` prints the same figures.

use symbios_avatar::{Archetype, Avatar, AvatarRecord, Limb, Vec3, Zone};

/// The stations a foot is judged at: the default body and the frame axis's
/// ends, which move the foot's size through the girth.
const STATIONS: [f32; 3] = [0.0, -1.5, 1.0];

/// One foot, read in its own frame.
struct Foot {
    /// Heel-to-tip length along the foot's axis, metres.
    length: f32,
    /// Widest extent across the axis, metres.
    width: f32,
    /// The tallest point over the ground in the heel band and the toe band.
    heel_top: f32,
    toe_top: f32,
    /// How far the underside's centre at the toe band sits from the axis
    /// through the heel band's centre, across the foot: positive toward the
    /// body's midline.
    bend: f32,
    /// The axis's angle from the body's forward, positive away from the
    /// midline, degrees.
    toe_out: f32,
}

impl Foot {
    fn measure(avatar: &Avatar, limb: Limb) -> Option<Self> {
        let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
        let nodes: Vec<usize> = rig.in_zone(Zone::Extremity(limb));
        if nodes.len() < 3 {
            return None;
        }
        // The axis: from the rearmost node to the foremost, in the ground plane.
        let rear = nodes
            .iter()
            .map(|&joint| rig.joints[joint].position)
            .min_by(|a, b| a.z.total_cmp(&b.z))?;
        let fore = nodes
            .iter()
            .map(|&joint| rig.joints[joint].position)
            .max_by(|a, b| a.z.total_cmp(&b.z))?;
        let axis = Vec3::new(fore.x - rear.x, 0.0, fore.z - rear.z).try_normalize()?;
        let across = Vec3::new(-axis.z, 0.0, axis.x);
        // Medial is toward the body's midline: the side of the across axis
        // that points against the foot's own `x`. Outward, for the toe-out,
        // is the foot's own `x`.
        let medial = if across.x * rear.x < 0.0 { 1.0 } else { -1.0 };
        let toe_out = axis.x.atan2(axis.z).to_degrees() * rear.x.signum();

        let owned: Vec<Vec3> = mesh
            .positions
            .iter()
            .copied()
            .filter(|&at| rig.joints[rig.nearest_bone(at).joint].zone == Zone::Extremity(limb))
            .collect();
        if owned.len() < 32 {
            return None;
        }
        let ground = owned.iter().map(|at| at.y).fold(f32::MAX, f32::min);
        let along = |at: Vec3| (at - rear).dot(axis);
        let (back, front) = owned.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &at| {
            (lo.min(along(at)), hi.max(along(at)))
        });
        let length = front - back;
        let (left, right) = owned.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &at| {
            let side = (at - rear).dot(across);
            (lo.min(side), hi.max(side))
        });
        let band = |lo: f32, hi: f32| {
            owned
                .iter()
                .copied()
                .filter(|&at| {
                    let t = (along(at) - back) / length;
                    t >= lo && t < hi
                })
                .collect::<Vec<_>>()
        };
        let top = |at: &[Vec3]| at.iter().map(|p| p.y - ground).fold(0.0f32, f32::max);
        // The underside's centre across the foot, in a band.
        let centre = |at: &[Vec3]| {
            let sole: Vec<f32> = at
                .iter()
                .filter(|p| p.y - ground < 0.012)
                .map(|&p| (p - rear).dot(across))
                .collect();
            if sole.is_empty() {
                None
            } else {
                Some(sole.iter().sum::<f32>() / sole.len() as f32)
            }
        };
        let heel = band(0.0, 0.15);
        let toe = band(0.85, 1.0);
        let bend = (centre(&toe)? - centre(&heel)?) * medial;
        Some(Self {
            length,
            width: right - left,
            heel_top: top(&heel),
            toe_top: top(&toe),
            bend,
            toe_out,
        })
    }
}

/// Builds each station and reads both feet.
fn feet() -> Vec<(f32, Limb, Foot, f32)> {
    let mut out = Vec::new();
    for femininity in STATIONS {
        let mut record = AvatarRecord::new("Footed", Archetype::default());
        record.composites.femininity = femininity;
        record.composites.sanitize();
        record.sanitize();
        let avatar = Avatar::build(&record).expect("a biped builds");
        let (low, high) = avatar.parts.body.bounds();
        let stature = high.y - low.y;
        for limb in [Limb::HindLeft, Limb::HindRight] {
            let foot = Foot::measure(&avatar, limb).expect("a foot measures");
            out.push((femininity, limb, foot, stature));
        }
    }
    out
}

#[test]
fn a_foot_is_the_size_of_a_foot() {
    // #305. `FOOT_KEPT` claimed the ball delivered 0.61 of its asked radius
    // as surface and it delivered 0.884 on every seed, so every foot was asked
    // 45% too wide: the pre-feet tree reads width/length 0.506 against the
    // references' 0.37 and 0.38, and length/stature 0.169 against 0.164 and
    // 0.157. The agreed tree reads 0.36 and 0.164 on the default body.
    for (femininity, limb, foot, stature) in feet() {
        let wide = foot.width / foot.length;
        assert!(
            (0.30..0.43).contains(&wide),
            "femininity {femininity:+.1} {limb:?}: the foot is {:.3} as wide as it is long; the \
             references read 0.37 and 0.38, and the boot read 0.506",
            wide
        );
        let long = foot.length / stature;
        assert!(
            (0.145..0.180).contains(&long),
            "femininity {femininity:+.1} {limb:?}: the foot is {:.3} of stature long; the \
             references read 0.164 and 0.157",
            long
        );
    }
}

#[test]
fn a_foot_is_a_wedge_and_not_a_slab() {
    // #306. The level run solved every node's depth to land on the ground, so
    // every node's top sat twice its height above it and the foot was the same
    // thickness from heel to toe: by this ruler the pre-feet tree's toe stands
    // 0.56 of the heel band's height (`footaudit`'s coarser bands read 0.44). The references' toe stands 0.29 to 0.32 of the
    // heel's depth; `foot::shape` takes the agreed tree to 0.30.
    for (femininity, limb, foot, _) in feet() {
        let ratio = foot.toe_top / foot.heel_top;
        assert!(
            (0.22..0.38).contains(&ratio),
            "femininity {femininity:+.1} {limb:?}: the toe stands {:.3} of the heel band's \
             height ({:.1} over {:.1} mm); the references read 0.29 to 0.32 and the slab 0.56",
            ratio,
            foot.toe_top * 1000.0,
            foot.heel_top * 1000.0
        );
    }
}

#[test]
fn a_foot_bends_toward_its_big_toe_and_the_two_feet_are_chiral() {
    // #306. A capsule section is symmetric and a forefoot is not: the big-toe
    // side runs nearly straight and the little-toe side curves in, so the
    // forefoot's axis bends toward the midline. `foot::shape` shifts the toe
    // band 0.14 of the half-width medially, which is 5 to 9 mm across the
    // bodies here; the pre-feet tree reads zero on both feet. Asserted on BOTH
    // feet with the medial sign resolved per side, which is the chirality:
    // two feet cut from one plan and bent the same way in `x` would be a
    // left foot and another left foot.
    for (femininity, limb, foot, _) in feet() {
        assert!(
            foot.bend > 0.003,
            "femininity {femininity:+.1} {limb:?}: the toe band's underside sits {:.1} mm toward \
             the midline of the heel's — a symmetric forefoot, or one bent the wrong way",
            foot.bend * 1000.0
        );
    }
}

#[test]
fn the_feet_toe_out_a_little_and_mirror_each_other() {
    // #306. Both feet ran dead parallel and read as stood on rails; the plan
    // turns each level run seven degrees about the ankle, toe away from the
    // midline. Read off the rig's own heel-to-toe axis, signed outward per
    // side, so a foot turned the wrong way fails as loudly as one not turned.
    for (femininity, limb, foot, _) in feet() {
        assert!(
            (4.0..10.0).contains(&foot.toe_out),
            "femininity {femininity:+.1} {limb:?}: the foot toes out {:.1}°, against a plan of 7°",
            foot.toe_out
        );
    }
}
