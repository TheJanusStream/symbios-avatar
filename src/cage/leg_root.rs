//! Where a beast's legs join its trunk: wrapping each junction under the body
//! (#387).
//!
//! A bridged leg opens on its girdle's hull through a socket aimed at the
//! shoulder or the hip ([`super::limb`]), so its ring stands up the flank, as
//! it must to be a facet of a convex hull beside its fellow. Left there, the
//! leg reads as a tube stuck on the side of the body, a broad floor of chest
//! or belly between the pair. On an animal the top of the leg is part of the
//! body: the inner, lower edge of the junction runs down and in under the
//! trunk until left and right all but meet, a narrow strip of underside
//! between them, while the outer, upper edge still rises up the flank.
//!
//! **So the rings move after the hull, not before it.** A hull needs each
//! socket to be a flat facet its siblings stay behind, and two rings that
//! nearly meet under a keel would ask it for what crowded the sockets off the
//! girdles once already. Afterwards the hull's faces are only polygons on
//! vertices: the cage's topology stands as the hull made it, nothing can stop
//! building, no triangle is added, and no joint moves. Only skin does. A
//! person has no bridges, so a person's cage never comes through here.

use glam::Vec3;

use super::Socket;
use super::limb::{LimbBuild, RING};
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::skeleton::Skeleton;

/// How a pair of legs wraps under its girdle.
struct Wrap {
    /// How far off the midline the ring's lowest point comes to rest, in the
    /// girdle's radii: half the cage's strip of underside. Catmull-Clark draws
    /// the skin a little further out. Not the ring's own radius: a heavy
    /// thigh's ring is half as wide again, and the strip would grow with it.
    strip: f32,
    /// How far under the keel the ring is held, in the ring's radii, so the
    /// inner wall of the leg passes under the trunk's floor rather than along it.
    under: f32,
    /// What share of the way along the body toward the spine's ring the lowest
    /// points slide.
    along: f32,
    /// How much a ring's weight favours its end toward the spine, against
    /// straight down and in as one.
    ahead: f32,
    /// How far the inner side of the first ring the leg sweeps to - an elbow,
    /// a stifle - comes in, in that ring's half-widths, so the leg leans toward
    /// the strip instead of stepping out to it.
    lean: f32,
}

/// The fore legs. Their rings stand ahead of the elbows, on the rise of the
/// forechest, and a cat's spine ring stands right behind them: slid along the
/// body or held under the keel there, the ring folds over that ring's lower
/// edge and shows as a sliver on the chest. Less lean than this lets a heavy
/// kitten's skin through the ground as it lies down (3 mm at 0.13, measured).
const FORE: Wrap = Wrap {
    strip: 0.1,
    under: 0.0,
    along: 0.0,
    ahead: 0.0,
    lean: 0.3,
};

/// The hind legs. Their rings stand behind the belly with room ahead of them,
/// so the lowest points run forward into the groin and the thighs close on it.
/// Its back stays where it is: drawn in with the rest, the back of each thigh
/// ran to the midline and left a pit under the tail. Its lean is small because
/// the thigh's inner skin folds under a lying body: at 0.3 a heavy foal's chest
/// rode up to 196 mm, against the 191 `tests/rest.rs` allows it.
///
/// Measured and left: on the shortest legs (`legLength` -3) the wrapped thigh dips
/// through the ground for a moment as a body gets up from a sit, up to 1.4% of
/// its withers height, on 11 of 2,799 sampled corners against none of those
/// unwrapped. Held rests do not, no one of these settings changes it, and fading
/// the wrap there would give those bodies back the legs stuck on their sides.
const HIND: Wrap = Wrap {
    strip: 0.1,
    under: 0.1,
    along: 0.5,
    ahead: 0.4,
    lean: 0.1,
};

/// Below this share of a ring's reach toward the keel a vertex stays where the
/// hull put it: the upper edge keeps to the flank, and the back of a thigh does
/// not cut a cleft beside the tail.
const FLANK: f32 = 0.4;

/// How far under the spine ring's floor that same side is held, in the same
/// half-widths. Only a leg whose elbow stands above the chest's floor, as a
/// cat's does, is moved by it: its inner wall would otherwise run coincident
/// with the trunk's and fight it.
const CLEAR: f32 = 0.25;

/// Wraps every bridged leg's junction under its girdle. See the module.
pub(super) fn wrap_junctions(
    skeleton: &Skeleton,
    sockets: &[Socket],
    builds: &[LimbBuild],
    mesh: &mut PolyMesh,
) {
    for socket in sockets {
        if skeleton.bridge_aim(socket.joint, socket.toward).is_none() {
            continue;
        }
        let Some((spine, far)) = keel(skeleton, socket, sockets, mesh) else {
            continue;
        };
        let wrap = if skeleton.nodes[socket.joint as usize].zone == Zone::Pelvis {
            &HIND
        } else {
            &FORE
        };
        let side = socket.center().x.signum();
        let strip = wrap.strip * skeleton.nodes[socket.joint as usize].radius;
        wrap_ring(socket, wrap, side, strip, (spine, far), mesh);
        // The ring the leg sweeps to, next to the socket along its limb.
        let next = builds.iter().find_map(|build| {
            let last = build.rings.len() - 1;
            if build.rings[0] == socket.ring {
                Some(build.rings[1])
            } else if build.rings[last] == socket.ring {
                Some(build.rings[last - 1])
            } else {
                None
            }
        });
        if let Some(next) = next {
            lean_ring(&next, wrap.lean, side, strip, spine.y, mesh);
        }
    }
}

/// The lowest point of a girdle's spine ring, and of the ring on its far side -
/// the neck's or the tail's: the two ends of the keel a ring wraps under.
/// `None` for a joint that is not a girdle with a leg either side.
fn keel(
    skeleton: &Skeleton,
    socket: &Socket,
    sockets: &[Socket],
    mesh: &PolyMesh,
) -> Option<(Vec3, Vec3)> {
    let (mut spine, mut far, mut legs) = (None, None, 0);
    for other in sockets.iter().filter(|other| other.joint == socket.joint) {
        if skeleton.bridge_aim(other.joint, other.toward).is_some() {
            legs += 1;
            continue;
        }
        let low = other
            .ring
            .iter()
            .map(|&vertex| mesh.positions[vertex as usize])
            .fold(
                Vec3::splat(f32::MAX),
                |low, at| {
                    if at.y < low.y { at } else { low }
                },
            );
        match skeleton.nodes[other.toward as usize].zone {
            Zone::Pelvis | Zone::Abdomen | Zone::Chest => spine = Some(low),
            _ => far = Some(low),
        }
    }
    (legs == 2).then_some(())?;
    Some((spine?, far?))
}

/// Brings a socket ring's lower, inner vertices in under the keel.
fn wrap_ring(
    socket: &Socket,
    wrap: &Wrap,
    side: f32,
    strip: f32,
    (spine, far): (Vec3, Vec3),
    mesh: &mut PolyMesh,
) {
    let centre = socket.center();
    let radius = socket.half.max_element();
    // Down and in, as it lies in the ring's own plane.
    let normal = socket.u.cross(socket.v).normalize_or_zero();
    let inward = Vec3::new(-side, -1.0, wrap.ahead * (spine.z - centre.z).signum());
    let inward = (inward - normal * inward.dot(normal)).normalize_or_zero();
    let at = socket.ring.map(|vertex| mesh.positions[vertex as usize]);
    let reach = at
        .iter()
        .map(|&point| (point - centre).dot(inward))
        .fold(0.0f32, f32::max);
    if reach <= 0.0 {
        return;
    }
    let keel_at = |z: f32| {
        let share = ((z - spine.z) / (far.z - spine.z)).clamp(0.0, 1.0);
        spine.y + (far.y - spine.y) * share
    };
    for (&vertex, point) in socket.ring.iter().zip(at) {
        let share = weight(((point - centre).dot(inward) / reach - FLANK) / (1.0 - FLANK));
        let z = point.z + (spine.z - point.z) * wrap.along * share;
        // Never up: the rings already below the keel keep their depth.
        let y = point.y.min(keel_at(z) - wrap.under * radius);
        mesh.positions[vertex as usize] = Vec3::new(
            point.x + (side * strip - point.x) * share,
            point.y + (y - point.y) * share,
            z,
        );
    }
}

/// Leans a leg's first swept ring in toward the strip and, where it stands
/// above the trunk's floor, under it.
fn lean_ring(
    ring: &[u32; RING],
    lean: f32,
    side: f32,
    strip: f32,
    floor: f32,
    mesh: &mut PolyMesh,
) {
    let at = ring.map(|vertex| mesh.positions[vertex as usize]);
    let centre = at.iter().copied().sum::<Vec3>() / RING as f32;
    let half = at
        .iter()
        .map(|point| (*point - centre).x * -side)
        .fold(0.0f32, f32::max);
    if half <= 0.0 {
        return;
    }
    for (&vertex, point) in ring.iter().zip(at) {
        let share = weight((point - centre).x * -side / half);
        // Measured off the midline on the leg's own side, so a fat stifle
        // whose cage crossed the midline is drawn back to its own side.
        let out = point.x * side;
        let held = (out - lean * half).max(strip);
        let x = out + (held - out) * share;
        let y = point.y.min(floor - CLEAR * half);
        mesh.positions[vertex as usize] =
            Vec3::new(side * x, point.y + (y - point.y) * share, point.z);
    }
}

/// A smooth step over a clamped share.
fn weight(share: f32) -> f32 {
    let share = share.clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_with_sockets};
    use crate::plan::{Family, Limb};
    use crate::{Archetype, AvatarRecord, HumanoidParams, QuadrupedParams, catmull_clark};

    /// A family's table row: how far apart its left and right junctions come.
    #[derive(Default)]
    struct Readings {
        rolls: usize,
        least: f32,
        most: f32,
    }

    impl Readings {
        fn read(&mut self, gap: f32) {
            self.least = if self.rolls == 0 {
                gap
            } else {
                self.least.min(gap)
            };
            self.most = self.most.max(gap);
            self.rolls += 1;
        }
    }

    /// How far off the midline a ring's skin comes, and that point's height;
    /// and the ring's top on the skin. Read off the subdivided surface, whose
    /// first vertices are the cage's own, each where the smoothing drew it.
    fn ring_on_skin(ring: &[u32; RING], skin: &[Vec3]) -> (f32, f32, Vec3) {
        let at = ring.map(|vertex| skin[vertex as usize]);
        let inner = at
            .iter()
            .copied()
            .fold(Vec3::splat(f32::MAX), |best, point| {
                if point.x.abs() < best.x.abs() {
                    point
                } else {
                    best
                }
            });
        let top = at
            .iter()
            .copied()
            .fold(Vec3::splat(f32::MIN), |best, point| {
                if point.y > best.y { point } else { best }
            });
        (inner.x.abs(), inner.y, top)
    }

    /// #387: fore and hind apart, per family over 60 rolls each, as R3's critic
    /// taught: a pooled test hid a defect once.
    #[test]
    fn left_and_right_junctions_almost_meet_under_the_trunk() {
        // Shares of the withers height, skin to skin, chosen after reading the
        // table this test prints: at most WIDEST apart, at least NARROWEST so
        // a strip of underside stays. On the code before the wrap the same
        // table reaches 0.16 to 0.35 in every family's fore and hind, whose
        // most is what this holds down (a roll's rear vertex can stand close
        // already, so its least says little).
        const WIDEST: f32 = 0.15;
        const NARROWEST: f32 = 0.015;
        const ROLLS: usize = 60;

        let mut fore: [Readings; 4] = Default::default();
        let mut hind: [Readings; 4] = Default::default();
        let mut seed = 0;
        while fore.iter().any(|row| row.rolls < ROLLS) {
            let record = AvatarRecord::rolled(
                "Probe",
                Archetype::Quadruped(QuadrupedParams::default()),
                seed,
            );
            seed += 1;
            let Archetype::Quadruped(params) = &record.archetype else {
                panic!("a quadruped rolled something else");
            };
            let row = match (params.hybrid, params.family) {
                (Some(_), _) => 3,
                (None, Family::Dog) => 0,
                (None, Family::Cat) => 1,
                (None, _) => 2,
            };
            if fore[row].rolls >= ROLLS {
                continue;
            }
            let wh = params.height * 1.2;
            let skeleton = record.skeleton();
            let (cage, sockets) = build_with_sockets(&skeleton, &CageConfig::default())
                .unwrap_or_else(|error| panic!("seed {seed}: {error}"));
            let skin = catmull_clark(&cage, 1).positions;
            for (limbs, table, name) in [
                ([Limb::ForeLeft, Limb::ForeRight], &mut fore, "fore"),
                ([Limb::HindLeft, Limb::HindRight], &mut hind, "hind"),
            ] {
                let ends = limbs.map(|limb| {
                    let socket = sockets
                        .iter()
                        .find(|socket| {
                            skeleton.bridge_aim(socket.joint, socket.toward).is_some()
                                && skeleton.nodes[socket.toward as usize].zone
                                    == Zone::UpperLimb(limb)
                        })
                        .expect("a bridged leg opens a socket");
                    (socket, ring_on_skin(&socket.ring, &skin))
                });
                let gap = (ends[0].1.0 + ends[1].1.0) / wh;
                table[row].read(gap);
                assert!(
                    (NARROWEST..=WIDEST).contains(&gap),
                    "seed {seed} {name}: left and right junctions {gap:.3} WH apart"
                );
                let (socket, (inner, low, top)) = ends[0];
                let hub = skeleton.nodes[socket.joint as usize];
                // Under the trunk: below its middle by a quarter of its depth.
                assert!(
                    low < hub.position.y - 0.25 * hub.half_extents().y,
                    "seed {seed} {name}: the junction comes in at {low:.3}, beside a \
                     girdle at {:.3}",
                    hub.position.y
                );
                // And its outer edge still on the flank: risen by a share of
                // the girdle's depth, well out from where its inner edge came
                // to rest.
                assert!(
                    top.y - low > 0.3 * hub.half_extents().y && top.x.abs() > 2.0 * inner,
                    "seed {seed} {name}: the junction's top is {top:?}, its inner edge \
                     {inner:.3} off the midline"
                );
            }
        }
        for (name, row) in ["dog", "cat", "horse", "cross"].iter().zip(0..) {
            println!(
                "{name}: fore {:.3}..{:.3}, hind {:.3}..{:.3} WH, {} rolls",
                fore[row].least, fore[row].most, hind[row].least, hind[row].most, fore[row].rolls
            );
        }
    }

    /// A person has no bridge, and so nothing here can reach one. Pinned so a
    /// body plan adopting bridges has to ask.
    #[test]
    fn a_person_has_no_bridged_socket() {
        let record =
            AvatarRecord::rolled("Probe", Archetype::Humanoid(HumanoidParams::default()), 7);
        let skeleton = record.skeleton();
        let (_, sockets) =
            build_with_sockets(&skeleton, &CageConfig::default()).expect("a person meshes");
        assert!(
            sockets
                .iter()
                .all(|socket| skeleton.bridge_aim(socket.joint, socket.toward).is_none())
        );
    }
}
