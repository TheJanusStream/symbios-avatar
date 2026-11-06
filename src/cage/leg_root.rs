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
//!
//! **The leg's outside goes over the trunk (#394).** The hull's widest points
//! are its sockets' own, and a ring's widest vertex need not be the one the
//! leg's outer wall runs up to: a ring is matched to the next by index, and on
//! a cat the thigh's outer wall climbs to the ring's top and back, inside the
//! hip, while the ring's widest vertex runs off down the front of the thigh.
//! So the trunk stood past the wall at the leg's top, and swinging the leg
//! brought it through. Each ring vertex is carried out as far as the vertex it
//! runs down to is the swept ring's outside, past whichever stands widest
//! there, the ring itself or the trunk, and the swept ring follows part of the
//! way. Only the outer side moves, so the wrap under the trunk stands as it was.

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
    /// How much less a socket ring's vertex comes out with the leg's outer
    /// wall for lying past its girdle toward the ring on the far side.
    far: f32,
}

/// The fore legs. Their rings stand ahead of the elbows, on the rise of the
/// forechest, and a cat's spine ring stands right behind them: slid along the
/// body or held under the keel there, the ring folds over that ring's lower
/// edge and shows as a sliver on the chest. Less lean than this lets a heavy
/// kitten's skin through the ground as it lies down (3 mm at 0.13, measured).
/// Their wall holds back at the forechest: carried out there it stood the chest
/// out from a small-headed cat's neck, a step of 0.61 head radii against the 0.45
/// `the_neck_grows_into_the_head_without_a_step` allows.
const FORE: Wrap = Wrap {
    strip: 0.1,
    under: 0.0,
    along: 0.0,
    ahead: 0.0,
    lean: 0.3,
    far: 1.0,
};

/// The hind legs. Their rings stand behind the belly with room ahead of them,
/// so the lowest points run forward into the groin and the thighs close on it.
/// Its back stays where it is: drawn in with the rest, the back of each thigh
/// ran to the midline and left a pit under the tail. Its lean is small because
/// the thigh's inner skin folds under a lying body: at 0.3 a heavy foal's chest
/// rode up to 196 mm, against the 191 `tests/rest.rs` allows it.
///
/// On the shortest legs (`legLength` -3) the wrapped thigh hangs lowest as a body
/// gets up from a sit, and hips rising over a leg folded that tight only sink it
/// further: `settle` in `anim/rest.rs` lifts the feet for it instead (#390), as
/// fading the wrap there would give those bodies back the legs stuck on their
/// sides.
///
/// Their wall holds back some beside the tail: carried out there in full, a lying
/// dog's rump folded in dark spikes, seen from behind and above.
const HIND: Wrap = Wrap {
    strip: 0.1,
    under: 0.1,
    along: 0.5,
    ahead: 0.4,
    lean: 0.1,
    far: 0.6,
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

/// How far past the trunk a leg's outer wall comes at its top, in half-widths
/// of the ring the leg sweeps to, so a heavy thigh stands proud by more than a
/// slight one.
///
/// Provenance: **tuned by render** (#394), read with the swing of a galloping
/// cat's thigh: at 0.5 a horse's hips read wider than its barrel from behind.
const PROUD: f32 = 0.2;

/// What share of the way out to the top of its wall the swept ring comes - an
/// elbow, a stifle - so the wall falls from the top, not tucks in under it.
///
/// Provenance: **tuned by render** (#394): at 0.8 a cat's elbow stepped hard
/// into its forearm, and at 0 the wall leaned in and the trunk stood past it.
const FOLLOW: f32 = 0.5;

/// The furthest the swept ring comes out, in its half-widths: little, for it
/// carries the leg's own section. Widened further, a thigh stops being deeper
/// than it is wide (`a_thigh_is_deeper_from_the_side_than_across`, seed 27 at
/// 0.15), and a cat's fore leg stretches its coat.
///
/// Provenance: **measured** (#394): at 0.1 the worst tenth of a fore leg's texels
/// stretches 4.33 to one against the 4.5 `tests/uv.rs` allows; at 0.15, 4.39.
const FOLLOW_MOST: f32 = 0.1;

/// How much less of that a ring vertex comes out for standing a girdle's depth
/// above the girdle. Skin binds to the bone it is nearest, so a high vertex
/// carried out binds to the thigh and swings with it.
///
/// Provenance: **tuned by render** (#394): at none, a galloping or lying cat's
/// hip stood up as a corner over the line of its back.
const CROWN: f32 = 0.6;

/// How far along the body from a leg's own run a ring of the spine still counts
/// as the trunk beside it, in lengths of that run. It fades to nothing there, so
/// no ring's counting can jump as a slider slides it along the body.
///
/// Provenance: **measured** (#394), a first choice left standing: it counts a
/// cat's ribs behind its upper arm, the widest of the trunk there, and took the
/// median of the trunk's widest past the leg's on its fore legs from 0.0065 of the
/// withers height to -0.0044, against the hub's spine ring alone. No other value
/// has been tried.
const BESIDE: f32 = 0.5;

/// Wraps every bridged leg's junction under its girdle and brings its outer
/// wall out over the trunk. See the module.
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
        // Only a leg that hangs from its girdle wraps under it: a crawling
        // bat's arm rises from its chest to an elbow over its back (#420),
        // and wrapped, its junction drew a web from the keel up the arm.
        // Every beast's elbow and stifle stand below their girdles.
        if skeleton.nodes[socket.toward as usize].position.y
            > skeleton.nodes[socket.joint as usize].position.y
        {
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
            let trunk = trunk_beside(skeleton, socket, sockets, builds, &next, mesh);
            let hub = skeleton.nodes[socket.joint as usize];
            let girdle = (hub.position.y, hub.half_extents().y, hub.position.z, far.z);
            broaden_wall(wrap, &socket.ring, &next, side, trunk, girdle, mesh);
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

/// How far off the midline the spine stands beside a leg's top: the widest of
/// its rings along the run from the socket to the swept ring, each counted for
/// less the further along the body it lies past that run ([`BESIDE`]).
fn trunk_beside(
    skeleton: &Skeleton,
    socket: &Socket,
    sockets: &[Socket],
    builds: &[LimbBuild],
    next: &[u32; RING],
    mesh: &PolyMesh,
) -> f32 {
    let along = |ring: &[u32; RING]| {
        ring.iter()
            .map(|&vertex| mesh.positions[vertex as usize].z)
            .sum::<f32>()
            / RING as f32
    };
    let (rim, foot) = (along(&socket.ring), along(next));
    let (near, far) = (rim.min(foot), rim.max(foot));
    let run = (far - near).max(f32::EPSILON);
    let spines = sockets.iter().filter(|other| {
        other.joint == socket.joint
            && skeleton.bridge_aim(other.joint, other.toward).is_none()
            && matches!(
                skeleton.nodes[other.toward as usize].zone,
                Zone::Pelvis | Zone::Abdomen | Zone::Chest
            )
    });
    let rings = spines
        .flat_map(|spine| {
            builds.iter().filter(|build| {
                build.rings.first() == Some(&spine.ring) || build.rings.last() == Some(&spine.ring)
            })
        })
        .flat_map(|build| build.rings.iter());
    rings.fold(0.0f32, |widest, ring| {
        let at = along(ring);
        let past = (near - at).max(at - far).max(0.0) / run;
        let wide = ring
            .iter()
            .map(|&vertex| mesh.positions[vertex as usize].x.abs())
            .fold(0.0, f32::max);
        widest.max(wide * (1.0 - weight(past / BESIDE)))
    })
}

/// Brings a leg's outer wall out over the trunk at its top, each ring vertex by
/// how far the vertex it runs down to is the swept ring's outside. `girdle` is
/// its hub's height and half its depth, and how far along the body the hub and
/// the ring on its far side stand.
fn broaden_wall(
    wrap: &Wrap,
    rim: &[u32; RING],
    next: &[u32; RING],
    side: f32,
    trunk: f32,
    (height, depth, hub_z, far_z): (f32, f32, f32, f32),
    mesh: &mut PolyMesh,
) {
    let off_midline = |ring: &[u32; RING], mesh: &PolyMesh| {
        ring.map(|vertex| mesh.positions[vertex as usize].x * side)
    };
    let outermost = |ring: &[f32; RING]| ring.iter().copied().fold(f32::MIN, f32::max);
    let (rim_at, next_at) = (off_midline(rim, mesh), off_midline(next, mesh));
    let (rim_out, next_out) = (outermost(&rim_at), outermost(&next_at));
    let centre = next_at.iter().sum::<f32>() / RING as f32;
    let half = next_out - centre;
    if half <= 0.0 {
        return;
    }
    let top = rim_out.max(trunk) + PROUD * half;
    let follow = ((top - next_out).max(0.0) * FOLLOW).min(FOLLOW_MOST * half);
    for index in 0..RING {
        let share = weight((next_at[index] - centre) / half);
        mesh.positions[next[index] as usize].x += side * follow * share;
        let vertex = &mut mesh.positions[rim[index] as usize];
        let crown = 1.0 - CROWN * weight((vertex.y - height) / depth);
        let run = far_z - hub_z;
        let past = (vertex.z - hub_z) * run.signum() / run.abs().max(f32::EPSILON);
        let ahead = 1.0 - wrap.far * weight(past);
        vertex.x += side * (top - rim_at[index]).max(0.0) * share * crown * ahead;
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
    use std::collections::{HashMap, HashSet};

    use crate::cage::{CageConfig, build_with_limbs, build_with_sockets};
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

    /// How far the trunk stands past a leg's outer wall, as a share of the
    /// withers height: side on, over the leg's own run along the body, height
    /// by height, the trunk's widest skin less the leg's. Read off the
    /// subdivided surface, whose faces follow the cage's in order, one per
    /// corner, so each face still says whether it is the leg's or the trunk's.
    /// Tail and neck are not the trunk. A leg's very top, where its skin thins
    /// to the point of the ring, is not yet a wall to stand past.
    fn trunk_past_wall(
        skeleton: &Skeleton,
        cage: &PolyMesh,
        sockets: &[Socket],
        builds: &[LimbBuild],
        limb: Limb,
        wh: f32,
    ) -> f32 {
        /// How far below its top a leg's skin is first a wall, a share of WH.
        const POINT: f32 = 0.06;
        let rings_of = |socket: &Socket| {
            builds
                .iter()
                .find(|build| {
                    build.rings.first() == Some(&socket.ring)
                        || build.rings.last() == Some(&socket.ring)
                })
                .expect("every socket closes a limb")
                .rings
                .iter()
                .flatten()
                .copied()
        };
        let bridged = |socket: &&Socket| skeleton.bridge_aim(socket.joint, socket.toward).is_some();
        let socket = sockets
            .iter()
            .filter(bridged)
            .find(|socket| skeleton.nodes[socket.toward as usize].zone == Zone::UpperLimb(limb))
            .expect("a bridged leg opens a socket");
        let leg: HashSet<u32> = rings_of(socket).collect();
        let legs: HashSet<u32> = sockets.iter().filter(bridged).flat_map(rings_of).collect();
        let beyond: HashSet<u32> = sockets
            .iter()
            .filter(|socket| {
                !bridged(socket)
                    && !matches!(
                        skeleton.nodes[socket.toward as usize].zone,
                        Zone::Pelvis | Zone::Abdomen | Zone::Chest
                    )
            })
            .flat_map(rings_of)
            .collect();
        let skin = catmull_clark(cage, 1);
        let side = socket.center().x.signum();
        let cell = 0.004 * wh;
        // By (along, height): the leg's widest skin and the trunk's.
        let mut cells: HashMap<(i32, i32), (f32, f32)> = HashMap::new();
        let owners = cage
            .faces
            .iter()
            .flat_map(|face| std::iter::repeat_n(face, face.len()));
        for (face, parent) in skin.faces.iter().zip(owners) {
            let is_leg = parent.iter().all(|vertex| leg.contains(vertex));
            let is_trunk = !parent.iter().all(|vertex| legs.contains(vertex))
                && !parent.iter().all(|vertex| beyond.contains(vertex));
            if !is_leg && !is_trunk {
                continue;
            }
            let corners: Vec<Vec3> = face
                .iter()
                .map(|&vertex| skin.positions[vertex as usize] * Vec3::new(side, 1.0, 1.0))
                .collect();
            for [a, b, c] in [
                [corners[0], corners[1], corners[2]],
                [corners[0], corners[2], corners[3]],
            ] {
                let (ab, ac) = (b - a, c - a);
                let area = ab.z * ac.y - ac.z * ab.y;
                if area == 0.0 {
                    continue;
                }
                let cells_over =
                    |low: f32, high: f32| (low / cell).floor() as i32..=(high / cell).ceil() as i32;
                for along in cells_over(a.z.min(b.z).min(c.z), a.z.max(b.z).max(c.z)) {
                    for height in cells_over(a.y.min(b.y).min(c.y), a.y.max(b.y).max(c.y)) {
                        let (dz, dy) = (along as f32 * cell - a.z, height as f32 * cell - a.y);
                        let u = (dz * ac.y - ac.z * dy) / area;
                        let v = (ab.z * dy - dz * ab.y) / area;
                        if u < 0.0 || v < 0.0 || u + v > 1.0 {
                            continue;
                        }
                        let x = a.x + ab.x * u + ac.x * v;
                        let widest = cells.entry((along, height)).or_insert((f32::MIN, f32::MIN));
                        if is_leg {
                            widest.0 = widest.0.max(x);
                        } else {
                            widest.1 = widest.1.max(x);
                        }
                    }
                }
            }
        }
        let legged: Vec<_> = cells
            .iter()
            .filter(|(_, widest)| widest.0 > f32::MIN)
            .collect();
        let run = legged.iter().map(|(key, _)| key.0);
        let (near, far) = (run.clone().min().unwrap_or(0), run.max().unwrap_or(0));
        let top = legged.iter().map(|(key, _)| key.1).max().unwrap_or(0);
        // The leg's run only, and down from its point.
        let mut rows: HashMap<i32, (f32, f32)> = HashMap::new();
        for (&(along, height), &(on_leg, on_trunk)) in &cells {
            if (near..=far).contains(&along)
                && height as f32 * cell <= top as f32 * cell - POINT * wh
            {
                let row = rows.entry(height).or_insert((f32::MIN, f32::MIN));
                row.0 = row.0.max(on_leg);
                row.1 = row.1.max(on_trunk);
            }
        }
        rows.values()
            .filter(|row| row.0 > f32::MIN && row.1 > f32::MIN)
            .map(|row| row.1 - row.0)
            .fold(f32::MIN, f32::max)
            / wh
    }

    /// #394: fore and hind apart, per family over 60 rolls each, as #387's
    /// table is. A body plan's trunk and legs differ, so each row has its own
    /// ceiling, from the table this test prints, with margin for CI's libm.
    ///
    /// | worst, WH | dog fore | dog hind | cat fore | cat hind | cross fore | cross hind |
    /// |-----------|----------|----------|----------|----------|------------|------------|
    /// | unbroadened | 0.0062 | 0.0046 | 0.0418 | 0.0201 | 0.0338 | 0.0199 |
    /// | broadened | 0.0035 | 0.0036 | 0.0145 | 0.0096 | 0.0058 | 0.0088 |
    ///
    /// A horse's trunk stood past neither wall at rest before or after, so its
    /// row holds that line and no more.
    #[test]
    fn the_trunk_does_not_stand_past_a_leg_s_outer_wall() {
        const ROLLS: usize = 60;
        /// Ceilings by row, dog, cat, horse, cross, fore then hind.
        const CEILING: [(f32, f32); 4] = [
            (0.0042, 0.0042),
            (0.018, 0.0115),
            (0.001, 0.001),
            (0.0075, 0.0105),
        ];
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
            let (cage, sockets, builds) = build_with_limbs(&skeleton, &CageConfig::default())
                .unwrap_or_else(|error| panic!("seed {seed}: {error}"));
            for (limb, table) in [(Limb::ForeLeft, &mut fore), (Limb::HindLeft, &mut hind)] {
                table[row].read(trunk_past_wall(
                    &skeleton, &cage, &sockets, &builds, limb, wh,
                ));
            }
        }
        for (name, row) in ["dog", "cat", "horse", "cross"].iter().zip(0..) {
            println!(
                "{name}: fore {:.4}..{:.4}, hind {:.4}..{:.4} WH, {} rolls",
                fore[row].least, fore[row].most, hind[row].least, hind[row].most, fore[row].rolls
            );
        }
        for (name, row) in ["dog", "cat", "horse", "cross"].iter().zip(0..) {
            assert!(
                fore[row].most <= CEILING[row].0 && hind[row].most <= CEILING[row].1,
                "{name}: the trunk stands {:.4} past the fore wall, {:.4} past the hind, in WH",
                fore[row].most,
                hind[row].most
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
