//! A winged body's drawn toes (#422): a bird's four and a bat's five, each a
//! chain of digit joints with a tube drawn along it.
//!
//! **On `extremity::hand`'s pattern**: grown on the rig after the body is
//! bound, every joint [`Role::Digit`], so the binding never offers one skin,
//! and in the leg's extremity zone - the parent the toes hang from is the
//! foot's pad - so [`Rig::extremity_joints`], the foot every gait and ground
//! contact reads, holds them and `Rig::extremity_extent` spans them. The
//! foot's `LimbTip` landmark, where a prop at the foot sits, is the plan's
//! toe node and none of these (#430): the last of them is a claw's point.
//! Attached after every joint the plan hangs, the foot's contact is
//! still the zone's first joint, and `Rig::limb_chain` - three joints above
//! the extremity on every winged leg - never reaches them. A wing's joints
//! share the fore limb's zone and are left out of the foot by
//! `Wings::owns`; these are the foot.
//!
//! **Laid out by the foot's declaration** (`plan::winged_foot`): each toe's
//! joints in the frame its `FootShape` reads off the pad and the foot's second
//! node, so the toes stand where the ball or sole the body's foot was shaped
//! into says, and the coat painter, reading the same layout, paints the
//! talons horn and the toes bare.
//!
//! Each foot is one part, in its pad joint's space as every attached
//! extremity is, skinned to its toes' joints: a closed tube a toe, its rings
//! on its joints - and on each phalanx's middle on a bird, whose pads bulge
//! under its toes - and on its claw's curve, which ends in a point.

use glam::{Vec2, Vec3};

use super::Attached;
use crate::det;
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::beast_foot::FootShape;
use crate::plan::winged_foot::{CLAW_SAMPLES, DrawnToe, WingedFoot};
use crate::plan::{BeastFeet, Limb, Zone};
use crate::rig::skin::Influence;
use crate::rig::{Rig, Role};

/// How finely a toe is drawn: the sides of each ring, and whether each
/// phalanx gets a ring at its middle besides its ends. A bird's eight, with
/// the middles its pads bulge between; a bat's six on its joints alone, ten
/// toes on a body already near its budget.
/// Provenance: **tuned by render**, against the budget (#422).
fn resolution(foot: WingedFoot) -> (usize, bool) {
    match foot {
        WingedFoot::Bat => (6, false),
        _ => (8, true),
    }
}

/// Grows every drawn toe a winged body's feet declare on `rig` (#422), one
/// part a foot, and returns them; nothing on a body whose toes are not
/// drawn - a person's, a beast's, a glider's.
pub(crate) fn grow(rig: &mut Rig, feet: &BeastFeet) -> Vec<Attached> {
    let Some(winged) = feet.winged.filter(|winged| winged.drawn()) else {
        return Vec::new();
    };
    // The hind feet only: a bat's fore "foot" is its wrist's pad and its
    // thumb, which are the wing's (#421).
    super::beast::shapes(rig, feet)
        .into_iter()
        .filter(|(limb, _)| !limb.is_fore())
        .filter_map(|(limb, shape)| {
            let &pad = rig.in_zone(Zone::Extremity(limb)).first()?;
            Some(grow_foot(rig, limb, pad, &shape, winged))
        })
        .collect()
}

/// One foot's toes, on its pad joint.
fn grow_foot(
    rig: &mut Rig,
    limb: Limb,
    pad: usize,
    shape: &FootShape,
    winged: WingedFoot,
) -> Attached {
    let (sides, middles) = resolution(winged);
    let toes = shape.drawn_toes();
    let origin = rig.joints[pad].position;
    let mut mesh = PolyMesh::new();
    let mut reach = 0.0f32;
    for (row, toe) in toes.iter().enumerate() {
        // The toe's own joints, parent before child down it.
        let mut chain = Vec::with_capacity(toe.joints.len());
        let mut parent = pad;
        for &local in &toe.joints {
            parent = rig
                .attach(parent, shape.body(local), Role::Digit)
                .expect("the pad is a joint of this rig");
            chain.push(parent);
        }
        let tip = shape.body(*toe.joints.last().unwrap_or(&Vec3::ZERO));
        reach = reach.max(tip.distance(origin));
        let rows = (row as f32, toes.len() as f32);
        let mut tube = tube(toe, &chain, shape, sides, middles, rows);
        for point in &mut tube.positions {
            *point -= origin;
        }
        mesh.append(&tube);
    }
    Attached {
        limb,
        joint: pad,
        mesh,
        reach,
    }
}

/// One ring of a toe's tube: where it stands and which way the toe runs
/// there, its radius across and above, below, how far along the toe it is,
/// and the joints that hold it.
struct Station {
    centre: Vec3,
    along: Vec3,
    across: f32,
    below: f32,
    share: f32,
    hold: VertexSkin,
}

/// A vertex held by one joint, or by two in shares.
fn held(parts: &[(usize, f32)]) -> VertexSkin {
    let mut skin = VertexSkin::default();
    for (slot, &(joint, weight)) in parts.iter().enumerate() {
        skin[slot] = Influence {
            joint: joint as u16,
            weight,
        };
    }
    skin
}

/// A toe's closed tube, in body space: rings along its phalanges, the pads
/// bulging under them, then along its claw's curve to a point, wound outward.
/// `rows` is which row of the part's chart it takes, of how many.
fn tube(
    toe: &DrawnToe,
    chain: &[usize],
    shape: &FootShape,
    sides: usize,
    middles: bool,
    rows: (f32, f32),
) -> PolyMesh {
    let body = |local: Vec3| shape.body(local);
    let points: Vec<Vec3> = toe.joints.iter().map(|&local| body(local)).collect();
    let claw: Vec<Vec3> = toe.claw.iter().map(|&local| body(local)).collect();
    let phalanges = toe.bones() - 1;
    let total: f32 = points
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum::<f32>()
        .max(f32::EPSILON);
    let mut stations = Vec::new();
    let mut run = 0.0f32;
    for bone in 0..phalanges {
        let (a, b) = (points[bone], points[bone + 1]);
        let along = (b - a).normalize_or(Vec3::Z);
        let incoming = if bone == 0 {
            along
        } else {
            ((a - points[bone - 1]).normalize_or(along) + along).normalize_or(along)
        };
        let radius = toe.radii[bone];
        // A pad under every joint, where the toe meets the ground.
        stations.push(Station {
            centre: a,
            along: incoming,
            across: radius,
            below: radius * (1.0 + toe.pads),
            share: run / total,
            hold: if bone == 0 {
                held(&[(chain[0], 1.0)])
            } else {
                held(&[(chain[bone - 1], 0.5), (chain[bone], 0.5)])
            },
        });
        let length = a.distance(b);
        if middles {
            let radius = 0.5 * (toe.radii[bone] + toe.radii[bone + 1]);
            stations.push(Station {
                centre: (a + b) * 0.5,
                along,
                across: radius,
                below: radius * (1.0 + 0.3 * toe.pads),
                share: (run + 0.5 * length) / total,
                hold: held(&[(chain[bone], 1.0)]),
            });
        }
        run += length;
    }
    // The claw's base, where the toe ends, then along its curve.
    let base = points[phalanges];
    let claw_bone = chain[phalanges];
    let into_claw = (claw[1] - claw[0]).normalize_or(Vec3::Z);
    let last_along = (base - points[phalanges.saturating_sub(1)]).normalize_or(into_claw);
    let radius = toe.radii[phalanges];
    stations.push(Station {
        centre: base,
        along: (last_along + into_claw).normalize_or(into_claw),
        across: radius,
        below: radius * (1.0 + toe.pads),
        share: run / total,
        hold: held(&[(chain[phalanges - 1], 0.5), (claw_bone, 0.5)]),
    });
    let claw_length: f32 = claw
        .windows(2)
        .map(|pair| pair[0].distance(pair[1]))
        .sum::<f32>();
    let mut along_claw = 0.0f32;
    for sample in 1..CLAW_SAMPLES {
        along_claw += claw[sample - 1].distance(claw[sample]);
        let along = (claw[sample + 1] - claw[sample - 1]).normalize_or(into_claw);
        let radius = toe.claw_radius_at(sample as f32 / CLAW_SAMPLES as f32);
        stations.push(Station {
            centre: claw[sample],
            along,
            across: radius,
            below: radius,
            share: (run + along_claw) / (run + claw_length).max(f32::EPSILON),
            hold: held(&[(claw_bone, 1.0)]),
        });
    }

    let mut mesh = PolyMesh::new();
    let mut skins = Vec::new();
    let mut uvs = Vec::new();
    let (row, of) = rows;
    let chart = |around: f32, along: f32| Vec2::new(along, (row + around) / of);
    for station in &stations {
        let up = (Vec3::Y - station.along * Vec3::Y.dot(station.along))
            .normalize_or(station.along.any_orthonormal_vector());
        let side = station.along.cross(up);
        for step in 0..sides {
            let angle = std::f32::consts::TAU * step as f32 / sides as f32;
            let (sin, cos) = det::sin_cos(angle);
            let rise = if sin < 0.0 {
                station.below
            } else {
                station.across
            };
            mesh.push_vertex(station.centre + side * (cos * station.across) + up * (sin * rise));
            skins.push(station.hold);
            // Round the tube from one side to the other and back, so the
            // chart has no seam to wrap across.
            let around = 1.0 - (angle - std::f32::consts::PI).abs() / std::f32::consts::PI;
            uvs.push(chart(around, station.share));
        }
    }
    let ring = |station: usize, step: usize| (station * sides + step % sides) as u32;
    for station in 0..stations.len() - 1 {
        for step in 0..sides {
            mesh.push_face(vec![
                ring(station, step),
                ring(station + 1, step),
                ring(station + 1, step + 1),
                ring(station, step + 1),
            ]);
        }
    }
    // A cap at the root, inside the ball or the sole; the claw's point.
    let first = &stations[0];
    let root = mesh.push_vertex(first.centre - first.along * (0.5 * first.across));
    skins.push(first.hold);
    uvs.push(chart(0.5, 0.0));
    let point = mesh.push_vertex(claw[CLAW_SAMPLES]);
    skins.push(held(&[(claw_bone, 1.0)]));
    uvs.push(chart(0.5, 1.0));
    let end = stations.len() - 1;
    for step in 0..sides {
        mesh.push_face(vec![ring(0, step), ring(0, step + 1), root]);
        mesh.push_face(vec![ring(end, step + 1), ring(end, step), point]);
    }
    mesh.set_uvs(uvs);
    mesh.set_skin(skins);
    // Wound outward by measurement: which way round a ring runs depends on
    // its frame, and a mirrored foot's runs the other way.
    if volume(&mesh) < 0.0 {
        for face in &mut mesh.faces {
            face.reverse();
        }
    }
    mesh
}

/// A closed mesh's signed volume: positive wound outward.
fn volume(mesh: &PolyMesh) -> f32 {
    mesh.triangulated()
        .iter()
        .map(|tri| {
            let [a, b, c] = tri.map(|corner| mesh.positions[corner as usize]);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, Toes, WingedFamily, WingedParams};

    fn rig_of(family: WingedFamily) -> (Rig, BeastFeet) {
        let mut params = WingedParams::default();
        params.set_kind(family, None, 0.0);
        let skeleton = params.skeleton(&Composites::default());
        let rig = Rig::from_skeleton(&skeleton).expect("the body rigs");
        (
            rig,
            skeleton
                .beast_feet
                .expect("a winged body declares its feet"),
        )
    }

    #[test]
    fn every_toe_is_a_closed_tube_held_by_its_own_joints() {
        for family in [WingedFamily::Bird, WingedFamily::Bat] {
            let (mut rig, feet) = rig_of(family);
            let before = rig.len();
            let parts = grow(&mut rig, &feet);
            assert_eq!(parts.len(), 2, "{family:?}: a part on each hind foot");
            for part in &parts {
                assert!(
                    part.mesh.is_closed_manifold(),
                    "{family:?}: a toe has a hole"
                );
                assert!(
                    part.mesh.skin.iter().flatten().all(|hold| {
                        hold.weight == 0.0
                            || (usize::from(hold.joint) >= before
                                && rig.joints[usize::from(hold.joint)].role == Role::Digit)
                    }),
                    "{family:?}: a toe is held by a joint not its own"
                );
            }
        }
    }

    #[test]
    fn a_glider_s_toes_are_its_paws_and_draw_nothing() {
        let (mut rig, feet) = rig_of(WingedFamily::Glider);
        let before = rig.len();
        assert!(grow(&mut rig, &feet).is_empty());
        assert_eq!(rig.len(), before, "a glider grows no toe joints");
    }

    #[test]
    fn both_feet_are_mirror_images() {
        for toes in Toes::ALL {
            let (mut rig, mut feet) = rig_of(WingedFamily::Bird);
            feet.winged = Some(WingedFoot::Bird(toes));
            let parts = grow(&mut rig, &feet);
            let placed: Vec<Vec<Vec3>> = parts
                .iter()
                .map(|part| {
                    let at = rig.joints[part.joint].position;
                    part.mesh.positions.iter().map(|&p| p + at).collect()
                })
                .collect();
            for point in &placed[0] {
                let mirrored = Vec3::new(-point.x, point.y, point.z);
                let nearest = placed[1]
                    .iter()
                    .map(|other| other.distance(mirrored))
                    .fold(f32::MAX, f32::min);
                assert!(nearest < 1.0e-4, "{toes:?}: the feet differ by {nearest} m");
            }
        }
    }
}
