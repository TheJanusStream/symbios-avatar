//! Where a leg leaves the trunk (#376).
//!
//! A beast's upper arm and thigh lie inside its body (#374), so the skin a
//! leg owns starts at the elbow and the stifle - the first joint outside the
//! trunk - and the skin above them is the trunk's, whichever bone the skin
//! binding gave it to. The zone map draws that line where the weights cross,
//! which is a hard line on the coat: a chart's edge, a pattern's frame
//! turning, a mark gated on one side and not the other. This is the line
//! drawn by the body's own shape instead, and drawn soft: how far a point is
//! from each leg's bones and from the spine, each in its own sections, gives
//! a share that is all leg down the leg, all trunk on the barrel and runs
//! smoothly between them through the crease where the two meet.
//!
//! **A section, not a radius.** A horse's thigh is half again as deep at the
//! stifle as it is across, and measured in its deepest radius all round the
//! stifle reached in past the midline into the groin: on every rolled horse
//! the skin there joined the hind leg's chart and stretched up to twenty to
//! one. Each joint is measured in its half-extents across, along the body's
//! `X`, and deep, square to the bone and to `X`, as the spine always was.
//! And the spine runs on past the croup, so the rump and the tail behind it
//! are the trunk's however far beyond its last joint they lie.
//!
//! A body whose limbs hang free of its trunk - a person's - has no junction:
//! its limbs carry no marker inside the trunk, and every share is zero.

use glam::{Vec2, Vec3};

use super::Rig;
use crate::face::step;
use crate::plan::{Limb, Zone};

/// The ratio of a point's distance from a leg to its distance from the
/// spine, each in its own sections, past which the point is all trunk, and
/// short of which it is all leg: the band the share runs through.
///
/// Provenance: **tuned by render** (#376), round the crease's own `1`: the
/// band is wide enough that a stripe turns over a stripe's width or two at
/// the elbow, narrow enough that the barrel beside the elbow keeps its bands.
const TRUNK_RATIO: f32 = 1.35;
const LEG_RATIO: f32 = 0.75;

/// One leg, from its junction down.
#[derive(Clone, Debug, PartialEq)]
struct Leg {
    limb: Limb,
    /// The trunk zone it leaves: the zone of the girdle it hangs from.
    girdle: Zone,
    /// Its joints from the first outside the trunk to the last of its foot,
    /// each with its section's half-extents, across and fore and aft.
    chain: Vec<(Vec3, Vec2)>,
    /// The box the chain's joints lie in, and its widest half-extent.
    low: Vec3,
    high: Vec3,
    widest: f32,
}

/// How much of a leg a point is, and which leg.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LegShare {
    /// The leg.
    pub limb: Limb,
    /// How much of it the point is, `0` on the trunk to `1` down the leg.
    pub share: f32,
    /// Where the leg leaves the trunk: its first joint outside it.
    pub junction: Vec3,
    /// How far down the leg's first bone the point lies below that joint,
    /// in the joint's widest radius: negative above it.
    pub depth: f32,
}

/// Every leg's junction with the trunk.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Junctions {
    /// The spine from the croup to the withers, each joint with its
    /// section's half-extents, across and deep.
    spine: Vec<(Vec3, Vec2)>,
    legs: Vec<Leg>,
}

impl Junctions {
    /// Reads the junctions off a rig: one per leg whose first bones are
    /// markers inside the trunk.
    pub(crate) fn of(rig: &Rig) -> Self {
        let spine: Vec<(Vec3, Vec2)> = [Zone::Pelvis, Zone::Abdomen, Zone::Chest]
            .into_iter()
            .flat_map(|zone| rig.in_zone(zone))
            .map(|joint| {
                let joint = &rig.joints[joint];
                (joint.position, joint.scale * joint.radius)
            })
            .collect();
        let legs = Limb::ALL
            .into_iter()
            .filter_map(|limb| {
                let upper = rig.in_zone(Zone::UpperLimb(limb));
                let &root = upper.first()?;
                if !rig.joints[root].marker {
                    return None;
                }
                let girdle = rig.joints[root]
                    .parent
                    .map(|joint| rig.joints[joint].zone)?;
                let below = [Zone::LowerLimb(limb), Zone::Extremity(limb)]
                    .into_iter()
                    .flat_map(|zone| rig.in_zone(zone));
                // Each joint in its own section, not its widest: measured in
                // the thigh's deep radius all round, the stifle reached in
                // past the midline into the groin, and onto the tail.
                let chain: Vec<(Vec3, Vec2)> = upper
                    .into_iter()
                    .skip_while(|&joint| rig.joints[joint].marker)
                    .chain(below)
                    .map(|joint| {
                        let joint = &rig.joints[joint];
                        (joint.position, joint.scale * joint.radius)
                    })
                    .collect();
                let (low, high, widest) = chain.iter().fold(
                    (Vec3::INFINITY, Vec3::NEG_INFINITY, 0.0f32),
                    |(low, high, widest), &(at, half)| {
                        (low.min(at), high.max(at), widest.max(half.max_element()))
                    },
                );
                (chain.len() >= 2).then_some(Leg {
                    limb,
                    girdle,
                    chain,
                    low,
                    high,
                    widest,
                })
            })
            .collect();
        Self { spine, legs }
    }

    /// Whether the body has any leg that leaves a trunk.
    pub(crate) fn is_empty(&self) -> bool {
        self.legs.is_empty() || self.spine.len() < 2
    }

    /// The leg `at` is most a part of, and how much, or `None` where it is
    /// no part of any - over the barrel, and everywhere on a body with no
    /// junction.
    pub(crate) fn leg(&self, at: Vec3) -> Option<LegShare> {
        if self.is_empty() {
            return None;
        }
        let trunk = self.spine_distance(at).max(f32::EPSILON);
        // The leg is the nearest, in its own sections: by the share instead,
        // a paw far below the trunk is as much the other leg of its pair's,
        // whose bones are a few of its radii away where the spine is many of
        // its own.
        let mut nearest: Option<(&Leg, f32)> = None;
        for leg in &self.legs {
            // Every texel asks, and most are nowhere near a leg: a point's
            // distance from the chain in its own sections is at least its
            // distance from the chain's box in the widest, so a point that
            // far out is past the band's trunk edge without walking it - and
            // so is every leg farther than one that is.
            let outside = (leg.low - at).max(at - leg.high).max(Vec3::ZERO).length();
            if outside >= TRUNK_RATIO * trunk * leg.widest {
                continue;
            }
            let distance = section_distance(&leg.chain, at, false);
            if nearest.is_none_or(|(_, least)| distance < least) {
                nearest = Some((leg, distance));
            }
        }
        let (leg, distance) = nearest?;
        let share = step(TRUNK_RATIO, LEG_RATIO, distance / trunk);
        let ((junction, half), (next, _)) = (leg.chain[0], leg.chain[1]);
        (share > 0.0).then(|| LegShare {
            limb: leg.limb,
            share,
            junction,
            depth: (at - junction).dot((next - junction).normalize_or(Vec3::NEG_Y))
                / half.max_element().max(f32::EPSILON),
        })
    }

    /// The trunk zone the upper leg zoned `zone` leaves, if it is one with a
    /// junction.
    pub(crate) fn girdle(&self, zone: Zone) -> Option<Zone> {
        let Zone::UpperLimb(limb) = zone else {
            return None;
        };
        self.legs
            .iter()
            .find(|leg| leg.limb == limb)
            .map(|leg| leg.girdle)
    }

    /// How much of any leg `at` is, `0..=1`.
    pub(crate) fn share(&self, at: Vec3) -> f32 {
        self.leg(at).map_or(0.0, |leg| leg.share)
    }

    /// The run of `limb`'s bone nearest `at`, down the leg: the way its
    /// coat's grain lies.
    pub(crate) fn leg_axis(&self, limb: Limb, at: Vec3) -> Option<Vec3> {
        let leg = self.legs.iter().find(|leg| leg.limb == limb)?;
        leg.chain
            .windows(2)
            .map(|pair| (pair[0], pair[1]))
            .min_by(|a, b| {
                section_distance(&[a.0, a.1], at, false).total_cmp(&section_distance(
                    &[b.0, b.1],
                    at,
                    false,
                ))
            })
            .map(|((a, _), (b, _))| (b - a).normalize_or(Vec3::NEG_Y))
    }

    /// How far `at` is from the spine, in the trunk's own section: about `1`
    /// on its skin wherever round the barrel.
    ///
    /// Past the croup the spine runs on. The rump and the tail lie behind its
    /// joint there, and measured from that joint a point's distance grew with
    /// how far beyond it the point lay, so a tail many of the spine's radii
    /// from it and only a few of a hind leg's came out part leg (#376), and
    /// wore a leg's marks and its grain: behind the croup only a point's
    /// distance square to the spine counts.
    fn spine_distance(&self, at: Vec3) -> f32 {
        section_distance(&self.spine, at, true)
    }
}

/// How far `at` is from a run of joints, in the run's own sections: each
/// joint's half-extents across, along the body's `X`, and deep, square to the
/// bone and to `X`, blended between joints. Past the run's last joint the
/// distance grows with how far beyond it `at` lies, and past its first too
/// unless `open`, where the run carries on straight.
fn section_distance(run: &[(Vec3, Vec2)], at: Vec3, open: bool) -> f32 {
    run.windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let ((a, a_half), (b, b_half)) = (pair[0], pair[1]);
            let axis = b - a;
            let first = if open && index == 0 {
                f32::NEG_INFINITY
            } else {
                0.0
            };
            let along = if axis.length_squared() > f32::EPSILON {
                ((at - a).dot(axis) / axis.length_squared()).clamp(first, 1.0)
            } else {
                0.0
            };
            let offset = at - (a + axis * along);
            let half = a_half
                .lerp(b_half, along.max(0.0))
                .max(Vec2::splat(f32::EPSILON));
            // Across is the body's own `X`; deep is the rest of the offset,
            // which past either closed end of the run includes how far beyond
            // it `at` lies.
            let across = offset.x;
            let deep = (offset - Vec3::X * across).length();
            Vec2::new(across / half.x, deep / half.y).length()
        })
        .fold(f32::INFINITY, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, Avatar, AvatarRecord, QuadrupedParams};

    /// A rolled beast of every family, built: its rig and its body's
    /// vertices with their zones.
    fn beasts() -> Vec<Avatar> {
        // 17 and 1 roll dogs, 2 and 14 cats, 6 and 13 horses.
        [17i64, 1, 2, 14, 6, 13]
            .into_iter()
            .map(|seed| {
                let mut record =
                    AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
                record.reroll(seed);
                Avatar::build(&record).expect("the beast builds")
            })
            .collect()
    }

    /// The spine's nearest point to `at`.
    fn nearest_on_spine(junctions: &Junctions, at: Vec3) -> Vec3 {
        junctions
            .spine
            .windows(2)
            .map(|pair| {
                let (a, b) = (pair[0].0, pair[1].0);
                let along = ((at - a).dot(b - a) / (b - a).length_squared()).clamp(0.0, 1.0);
                a + (b - a) * along
            })
            .min_by(|p, q| p.distance(at).total_cmp(&q.distance(at)))
            .expect("a spine")
    }

    #[test]
    fn a_person_has_no_junction() {
        // A person's limbs hang free of the trunk, and its coat is its skin:
        // nothing may ask a junction of it and get one.
        let rig = Rig::from_skeleton(&AvatarRecord::default().skeleton()).expect("rigs");
        let junctions = Junctions::of(&rig);
        assert!(junctions.is_empty());
        assert!(junctions.leg(rig.joints[0].position).is_none());
    }

    #[test]
    fn a_leg_is_all_its_own_below_the_junction_and_none_of_the_barrel() {
        // By share rather than by distance, a paw far below the trunk was as
        // much the other leg of its pair's - a few of its radii off, where
        // the spine is many of its own - and wore that leg's grain.
        for avatar in beasts() {
            let junctions = Junctions::of(&avatar.rig);
            assert_eq!(junctions.legs.len(), 4, "a beast has four junctions");
            let body = &avatar.parts.body;
            let normals = body.vertex_normals();
            for ((&at, &zone), normal) in
                body.positions.iter().zip(&avatar.parts.zones).zip(normals)
            {
                match zone {
                    Zone::LowerLimb(limb) | Zone::Extremity(limb) => {
                        let leg = junctions.leg(at).expect("a lower leg is a leg's");
                        assert_eq!(leg.limb, limb, "{zone:?} at {at} was another leg's");
                        assert!(leg.share > 0.99, "{zone:?} at {at} is {:.2} leg", leg.share);
                    }
                    // The back, which faces the sky. Not a fold facing up below
                    // the spine: the junctions meeting under the trunk (#387)
                    // gave a horse's groin one, the crease the share runs through.
                    Zone::Chest | Zone::Abdomen | Zone::Pelvis
                        if normal.y > 0.5 && at.y > nearest_on_spine(&junctions, at).y =>
                    {
                        assert_eq!(junctions.share(at), 0.0, "the back at {at} is a leg's");
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn neither_the_tail_nor_the_back_is_any_leg_s() {
        // Past the croup the spine's distance grew with how far beyond its
        // end a point lay, so a tail many of the spine's radii from it and a
        // few of a hind leg's came out part leg (#376): a horse's tail root
        // at 0.6, a dog's at 0.37, and with them a piebald's lean, a ticked
        // coat's bars and a leg's grain. Over two dozen rolls and the
        // shortest legs of a dog and a horse, where the tail hangs nearest
        // the hocks, no tail is a leg's, and no back: the barrel's skin above
        // the spine that faces the sky. Not the folds facing up below it, at
        // the elbow and in a cat's groin, which are the crease the share runs
        // through.
        use crate::plan::Family;
        let mut bodies: Vec<(String, AvatarRecord)> = (0i64..24)
            .map(|seed| {
                let mut record =
                    AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
                record.reroll(seed);
                (format!("seed {seed}"), record)
            })
            .collect();
        for family in [Family::Dog, Family::Horse] {
            let mut params = QuadrupedParams::default();
            params.set_kind(family, None, 0.0);
            params.leg_length = -1.0;
            let record = AvatarRecord::new("Beast", Archetype::Quadruped(params));
            bodies.push((format!("{family:?} short-legged"), record));
        }
        // Only the mesh and its zones are read.
        let config = crate::AvatarConfig {
            atlas: 64,
            ..crate::AvatarConfig::default()
        };
        for (name, record) in bodies {
            let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
            let junctions = Junctions::of(&avatar.rig);
            let (mut tails, mut backs) = (0, 0);
            let body = &avatar.parts.body;
            for ((&at, &zone), normal) in body
                .positions
                .iter()
                .zip(&avatar.parts.zones)
                .zip(body.vertex_normals())
            {
                let back = matches!(zone, Zone::Chest | Zone::Abdomen | Zone::Pelvis)
                    && normal.y > 0.2
                    && at.y > nearest_on_spine(&junctions, at).y;
                if zone == Zone::Tail || back {
                    let share = junctions.share(at);
                    assert_eq!(share, 0.0, "{name}: {zone:?} at {at} is {share:.3} leg");
                    tails += usize::from(zone == Zone::Tail);
                    backs += usize::from(back);
                }
            }
            assert!(
                tails > 20 && backs > 100,
                "{name}: {tails} tail, {backs} back"
            );
        }
    }

    #[test]
    fn the_box_a_leg_lies_in_never_changes_an_answer() {
        // The box only spares the walk down a leg's chain: asked of every
        // vertex, it must answer exactly what the walk does.
        for avatar in beasts() {
            let junctions = Junctions::of(&avatar.rig);
            for &at in &avatar.parts.body.positions {
                let trunk = junctions.spine_distance(at).max(f32::EPSILON);
                let walked = junctions
                    .legs
                    .iter()
                    .map(|leg| (leg.limb, section_distance(&leg.chain, at, false)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(limb, distance)| (limb, step(TRUNK_RATIO, LEG_RATIO, distance / trunk)))
                    .filter(|&(_, share)| share > 0.0);
                assert_eq!(
                    junctions.leg(at).map(|leg| (leg.limb, leg.share)),
                    walked,
                    "at {at}"
                );
            }
        }
    }
}
