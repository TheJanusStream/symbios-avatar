//! The throat-to-trunk junction: the collar, the nape and the trapezius.
//!
//! **Guards fitted AFTER the geometry was agreed by render, not before** —
//! milestone #10's standing method (#300). #301 removed a turtleneck collar
//! whose maker was none of the three candidates the research named, and
//! #302 built a trapezius across four passes, each one judged on the throat
//! close-up and two of them on defects the owner found that the instruments
//! of the time could not see. Every bound here was fitted to that agreed
//! tree and checked against the tree before it: the pre-throat body reads a
//! 3.6 mm collar rim at femininity −1.5, a 34° bend in the back line on the
//! heavy masculine body, and a 12.5° shoulder slope at −1 — and each of those
//! is outside the bound that guards it.
//!
//! Every figure is a silhouette, bisected against the surface, read down a
//! height ladder from the column into the shoulder: the same line the renders
//! were judged on, so a guard here can disagree with the render on the
//! render's own terms. `examples/neckaudit` prints the same readings.

use symbios_avatar::{Archetype, Avatar, AvatarRecord, Vec3, Zone};

/// The frame axis's four stations, and the body that caught the nape.
///
/// The first four are the stations every throat render was judged at; the
/// fifth is the heavy masculine body on which the owner found the nape's hump
/// after the first four read clean (#302), which is why a guard on this
/// junction reads more than the axis.
const JUNCTIONS: [(f32, f32); 5] = [
    (-1.5, 0.0),
    (-1.0, 0.0),
    (0.0, 0.0),
    (1.0, 0.0),
    (-1.5, 2.0),
];

/// Builds every station and reads its junction.
fn junctions() -> Vec<(f32, f32, Junction)> {
    JUNCTIONS
        .iter()
        .map(|&(femininity, mass)| {
            let mut record = AvatarRecord::new("Junction", Archetype::default());
            record.composites.femininity = femininity;
            record.composites.mass = mass;
            record.composites.sanitize();
            record.sanitize();
            let avatar = Avatar::build(&record).expect("a biped builds");
            let read = Junction::measure(&avatar).expect("the junction measures");
            (femininity, mass, read)
        })
        .collect()
}

#[test]
fn the_collar_is_not_a_rim() {
    // #301. The masculine collar was `HeadTraits::jaw_breadth` held at full
    // strength down to the head's floor: a 10 mm step in half-width at the
    // head/neck boundary, which the silhouette reads as the body being wider
    // just above the shoulder line than just below it. The pre-throat tree
    // reads 3.6 mm of that at −1.5; the agreed tree reads 1.2 at worst and
    // negative on most bodies, because a shoulder only widens going down.
    for (femininity, mass, read) in junctions() {
        assert!(
            read.collar < 0.0025,
            "femininity {femininity:+.1}, mass {mass:+.1}: the body is {:.1} mm wider above the \
             shoulder line than 10 mm below it — a collar rim. #301's maker was jaw_breadth \
             reaching the head's floor; #302's first fill folded along the vertex normals.",
            read.collar * 1000.0
        );
    }
}

#[test]
fn the_nape_has_no_ledge() {
    // #302. The back of the neck went through three shapes: two lumps flanking
    // the spine (an outward direction that flipped sign at the midline), a
    // slab with a ledge under it (the fill at full strength over the upper
    // back), and a hump above a step on the heavy masculine body (the fill's
    // backward push overshooting the upper back line). The agreed tree has no
    // fill on the nape at all — the hull's ring there is faired — and reads
    // at most 1.1 mm of overhang and a 13° bend; the pre-throat tree bends
    // 34° on the heavy body, at the ring.
    for (femininity, mass, read) in junctions() {
        assert!(
            read.nape < 0.003,
            "femininity {femininity:+.1}, mass {mass:+.1}: the back reaches {:.1} mm further \
             behind at some height than 10 mm below it — a ledge or a hump on the nape",
            read.nape * 1000.0
        );
        assert!(
            read.turn < 16.0,
            "femininity {femininity:+.1}, mass {mass:+.1}: the back line bends {:.1}° between \
             successive 10 mm segments — the hull's ring at the column's foot, unfaired",
            read.turn
        );
    }
}

#[test]
fn the_shoulders_slope_like_a_trapezius() {
    // #302. The shoulder line ran nearly horizontal and met the column at a
    // crease: 12.5° at femininity −1 on the pre-throat tree. The CC0 references
    // read 18.6° (male) and 20.1° (female) by `examples/reference`'s shoulder
    // table — measured to the top of the surface over the shoulder joint,
    // after three wrong acromions — and the agreed tree reads 20.7, 19.0,
    // 19.0, 14.9 and 21.3 across the five stations.
    //
    // **The feminine end is shallower than the female reference, and that is
    // recorded rather than guarded.** #302's design note wanted a longer
    // visible neck line at the feminine end and `HeadTraits::trapezius`
    // delivers it; the mannequins say the two sexes slope the same, the
    // female a degree steeper. Tightening the feminine end onto the reference
    // is a render decision for the owner, so the floor is 12° there and 16°
    // from neutral to masculine, where the state is 19° to 21° and the
    // pre-throat tree read 12.5° and 12.7°.
    //
    // **The pre-throat tree reads 21.8° at −1.5 and this guard passes it
    // there, on purpose.** That body's collar rim stands proud of the column
    // and the silhouette "leaves the column" at the rim's own height, so the
    // rim reads as a steep shoulder. It is `the_collar_is_not_a_rim`'s body,
    // and a slope guard that tried to own it would be a second ruler reading
    // the same defect under another name.
    for (femininity, mass, read) in junctions() {
        let floor = if femininity <= 0.0 { 16.0 } else { 12.0 };
        assert!(
            (floor..28.0).contains(&read.slope),
            "femininity {femininity:+.1}, mass {mass:+.1}: the shoulder line slopes {:.1}° from \
             where the silhouette leaves the column to the acromion, against a floor of \
             {floor}°; the references read 18.6° and 20.1°",
            read.slope
        );
    }
}

/// The junction's four readings, as `examples/neckaudit` prints them.
///
/// **Duplicated from the instrument on purpose**, as `the_neck_is_the_length_of_a_neck`
/// is duplicated by `neckaudit`'s `visible_neck`: the test is the contract and
/// the example is the instrument, and the day they disagree the instrument is
/// the one that is wrong — which is only checkable if the numbers can be held
/// side by side. The instrument also prints a waist-over-skull ratio this
/// file does not assert: on a short masculine neck the chin-to-crown run is
/// fifty millimetres and the column's narrowest genuinely sits low, so a
/// ruler confined to the upper half reads the jaw and nothing more.
struct Junction {
    collar: f32,
    nape: f32,
    turn: f32,
    slope: f32,
}

impl Junction {
    fn measure(avatar: &Avatar) -> Option<Self> {
        let (mesh, rig) = (&avatar.parts.body, &avatar.rig);
        let head = *rig.in_zone(Zone::Head).first()?;
        let neck = rig.joints[head].parent?;
        let girdle = rig.joints[neck].parent?;
        let axis = rig.joints[neck].position;
        let crown = rig.joints[girdle].position.y + rig.joints[girdle].radius;
        // The acromion: the girdle's lateral child.
        let (reach, acromion) = rig
            .joints
            .iter()
            .enumerate()
            .filter(|(index, joint)| *index != neck && joint.parent == Some(girdle))
            .map(|(_, joint)| (joint.position.x.abs(), joint.position.y + joint.radius))
            .filter(|(reach, _)| *reach > rig.joints[girdle].radius * 0.5)
            .max_by(|a, b| a.0.total_cmp(&b.0))?;

        // Outward from a point on the column's axis, bisected against the
        // surface. `None` when the axis point is already outside.
        let out = |from: Vec3, along: Vec3| -> Option<f32> {
            if !mesh.contains(from) {
                return None;
            }
            let (mut near, mut far) = (0.0f32, 0.45f32);
            if mesh.contains(from + along * far) {
                return None;
            }
            for _ in 0..30 {
                let middle = (near + far) * 0.5;
                if mesh.contains(from + along * middle) {
                    near = middle;
                } else {
                    far = middle;
                }
            }
            Some(near)
        };
        // The widest chord of the section at `y`, swept over its own depth —
        // the silhouette an eye reads, not a ray from an axis the section
        // does not sit on.
        let wide = |y: f32| -> Option<f32> {
            let from = Vec3::new(axis.x, y, axis.z);
            let (ahead, behind) = (out(from, Vec3::Z)?, out(from, -Vec3::Z)?);
            (0..=16)
                .filter_map(|slice| {
                    let z = axis.z - behind + (ahead + behind) * slice as f32 / 16.0;
                    out(Vec3::new(axis.x, y, z), Vec3::X)
                })
                .reduce(f32::max)
        };
        // How far the back reaches behind the column's axis at `y`.
        let back = |y: f32| -> Option<f32> { out(Vec3::new(axis.x, y, axis.z), -Vec3::Z) };

        // The ladder: 2 mm steps from 60 mm above the crown to the acromion.
        const STEP: f32 = 0.002;
        const LOOK: usize = 5; // 10 mm, in steps
        let top = crown + 0.06;
        let mut widths = Vec::new();
        let mut backs = Vec::new();
        let mut y = top;
        while y > acromion - 0.02 {
            widths.push(wide(y));
            backs.push(back(y));
            y -= STEP;
        }
        let overhang = |ladder: &[Option<f32>]| -> f32 {
            ladder
                .iter()
                .enumerate()
                .filter_map(|(i, above)| {
                    let above = (*above)?;
                    let below = ladder.get(i + LOOK).copied().flatten()?;
                    Some(above - below)
                })
                .fold(f32::MIN, f32::max)
        };
        let collar = overhang(&widths);
        let nape = overhang(&backs);

        // The back line's sharpest bend: successive 10 mm segments as vectors
        // in the sagittal plane, the angle between them.
        let mut turn = 0.0f32;
        for i in 0..backs.len().saturating_sub(2 * LOOK) {
            let (Some(a), Some(b), Some(c)) = (backs[i], backs[i + LOOK], backs[i + 2 * LOOK])
            else {
                continue;
            };
            let rise = STEP * LOOK as f32;
            let first = glam::Vec2::new(b - a, -rise);
            let second = glam::Vec2::new(c - b, -rise);
            turn = turn.max(first.angle_to(second).abs().to_degrees());
        }

        // The slope: from where the silhouette first stands 10 mm wider than
        // the column's foot, down to the acromion's top at its reach.
        let foot = wide(crown + 0.03)?;
        let leave = widths
            .iter()
            .enumerate()
            .find_map(|(i, w)| (w.is_some_and(|w| w > foot + 0.01)).then_some(i))?;
        let (leave_y, leave_x) = (top - STEP * leave as f32, widths[leave]?);
        let slope = ((leave_y - acromion) / (reach - leave_x).max(1e-3))
            .atan()
            .to_degrees();

        Some(Self {
            collar,
            nape,
            turn,
            slope,
        })
    }
}
