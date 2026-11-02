//! Where a card meets the skin, measured on the built beast (#386).
//!
//! A card that stands off the skin shows the line it enters the skin on: a
//! straight cut across a lock of hair, with skin right behind it. These read
//! that line off the drawn mesh, because the question is what a viewer sees:
//! each card's edges are walked from the root until they leave the body, and
//! each such point is asked whether any card lies between it and a camera.
//!
//! The zones of cards are read here: a ruff, a tail's brush, feathering. A mane is
//! a solid since #407, and its own tests (`super::mane`) read where it meets the
//! skin; the body and the rolls here serve them too.

use glam::Vec3;

use super::{FurGrowth, FurZone};
use crate::mesh::PolyMesh;
use crate::plan::{Family, Fur};
use crate::{Archetype, Avatar, AvatarRecord};

/// How far along a card its mask keeps the lane whole, so that the card is
/// solid and hides what lies behind it (see [`crate::hair::mask`]).
const SOLID: f32 = 0.6;

/// How far across a card the seam is sampled, edge to edge.
const ACROSS: [f32; 5] = [0.05, 0.275, 0.5, 0.725, 0.95];

/// Which way a seam must face a camera to count, as a cosine: a seam on the
/// neck's far side is the neck's to hide.
const FACING: f32 = 0.2;

/// One card: its stations, each its two edges, and the share of the way along
/// the card each stands.
struct Card {
    edges: Vec<(Vec3, Vec3)>,
    along: Vec<f32>,
}

/// The cards of `zone`, read back out of the mesh's run of quads: a card is a
/// run of faces whose first corners step by two.
fn cards(avatar: &Avatar, zone: FurZone) -> Vec<Card> {
    let Some(fur) = avatar.parts.fur.as_ref() else {
        return Vec::new();
    };
    let mesh = &fur.mesh;
    let mut face = 0;
    let mut cards: Vec<Card> = Vec::new();
    for grown in &fur.grown {
        let faces = face..face + grown.tris / 2;
        face = faces.end;
        if grown.zone != zone {
            continue;
        }
        let mut last = None;
        for corners in &mesh.faces[faces] {
            let station = |one: u32, two: u32| {
                (
                    mesh.positions[one as usize],
                    mesh.positions[two as usize],
                    mesh.uvs[one as usize].y,
                )
            };
            if last != Some(corners[0]) {
                let (left, right, along) = station(corners[0], corners[1]);
                cards.push(Card {
                    edges: vec![(left, right)],
                    along: vec![along],
                });
            }
            let (left, right, along) = station(corners[3], corners[2]);
            if let Some(card) = cards.last_mut() {
                card.edges.push((left, right));
                card.along.push(along);
            }
            last = Some(corners[0] + 2);
        }
    }
    cards
}

/// A closed body's triangles, to count crossings against.
pub(super) struct Solid {
    triangles: Vec<[Vec3; 3]>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
}

impl Solid {
    pub(super) fn of(body: &PolyMesh) -> Self {
        Self {
            triangles: triangles(body),
            positions: body.positions.clone(),
            normals: body.vertex_normals(),
        }
    }

    /// Whether `at` is inside: an odd count of crossings along a ray no mesh
    /// is aligned with.
    pub(super) fn holds(&self, at: Vec3) -> bool {
        let ray = Vec3::new(0.2113, 0.8741, 0.4372).normalize();
        self.triangles
            .iter()
            .filter(|triangle| hit(at, ray, triangle).is_some())
            .count()
            % 2
            == 1
    }

    /// Which way the skin nearest `at` faces.
    fn facing(&self, at: Vec3) -> Vec3 {
        let nearest = (0..self.positions.len())
            .min_by(|&a, &b| {
                self.positions[a]
                    .distance_squared(at)
                    .total_cmp(&self.positions[b].distance_squared(at))
            })
            .unwrap_or(0);
        self.normals[nearest]
    }
}

pub(super) fn triangles(mesh: &PolyMesh) -> Vec<[Vec3; 3]> {
    mesh.triangulated()
        .iter()
        .map(|corners| corners.map(|corner| mesh.positions[corner as usize]))
        .collect()
}

/// How far along `ray` from `from` it crosses `triangle`, if it does.
pub(super) fn hit(from: Vec3, ray: Vec3, [a, b, c]: &[Vec3; 3]) -> Option<f32> {
    let (one, two) = (*b - *a, *c - *a);
    let turned = ray.cross(two);
    let det = one.dot(turned);
    if det.abs() < 1e-12 {
        return None;
    }
    let off = from - *a;
    let u = off.dot(turned) / det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let back = off.cross(one);
    let v = ray.dot(back) / det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = two.dot(back) / det;
    (t > 0.0).then_some(t)
}

/// What one beast's cards of one zone do where they meet its skin.
#[derive(Clone, Copy, Debug, Default)]
struct Reading {
    /// How many cards.
    cards: usize,
    /// How many of them have a root corner out of the skin, or its middle.
    open_roots: usize,
    /// How much seam a camera sees, averaged over the views, as a share of
    /// the back's height.
    seen: f32,
    /// How much seam there is, whatever sees it, as the same share.
    seam: f32,
    /// How steeply a card's lines leave the skin, in degrees above it, as the
    /// mean over the cards' lines.
    standing: f32,
    /// What share of the stations past a card's seam have gone back into the
    /// skin.
    back_in: f32,
}

/// Reads `zone` on `avatar`.
fn read(avatar: &Avatar, zone: FurZone) -> Reading {
    let solid = Solid::of(&avatar.parts.body);
    let cards = cards(avatar, zone);
    let height = avatar
        .parts
        .body
        .positions
        .iter()
        .map(|at| at.y)
        .fold(f32::MIN, f32::max);
    let fur = avatar.parts.fur.as_ref().expect("fur");
    let blinds: Vec<[Vec3; 3]> = fur
        .mesh
        .faces
        .iter()
        .filter(|corners| {
            corners
                .iter()
                .all(|&corner| fur.mesh.uvs[corner as usize].y <= SOLID)
        })
        .flat_map(|corners| {
            let at = |index: usize| fur.mesh.positions[corners[index] as usize];
            [[at(0), at(1), at(2)], [at(0), at(2), at(3)]]
        })
        .collect();
    let mut views = Vec::new();
    for turn in 0..12 {
        for pitch in [0.0f32, 25.0, 50.0] {
            let (turn, pitch) = ((turn as f32 * 30.0 + 15.0).to_radians(), pitch.to_radians());
            views.push(Vec3::new(
                turn.sin() * pitch.cos(),
                pitch.sin(),
                turn.cos() * pitch.cos(),
            ));
        }
    }
    let mut reading = Reading {
        cards: cards.len(),
        ..Reading::default()
    };
    let (mut past, mut sunk) = (0usize, 0usize);
    let mut seam_points = Vec::new();
    for card in &cards {
        let (left, right) = card.edges[0];
        if !solid.holds(left) || !solid.holds(right) || !solid.holds((left + right) * 0.5) {
            reading.open_roots += 1;
        }
        // Each sampled line down the card, to where it leaves the skin.
        let mut exits: Vec<Vec3> = Vec::new();
        let mut steep = 0.0f32;
        for across in ACROSS {
            let line: Vec<Vec3> = card
                .edges
                .iter()
                .map(|(left, right)| left.lerp(*right, across))
                .collect();
            let exit = exit_of(&solid, &line);
            exits.push(exit.0);
            if let Some(after) = line.get(exit.1 + 1) {
                let run = (*after - line[exit.1]).normalize_or_zero();
                let skin = solid.facing(exit.0);
                steep += run.dot(skin).clamp(-1.0, 1.0).asin().to_degrees();
            }
            for point in line.iter().skip(exit.1 + 1) {
                past += 1;
                sunk += usize::from(solid.holds(*point));
            }
        }
        reading.standing += steep / ACROSS.len() as f32;
        for (index, exit) in exits.iter().enumerate() {
            let before = exits[index.saturating_sub(1)].distance(*exit);
            let after = exits[(index + 1).min(exits.len() - 1)].distance(*exit);
            seam_points.push((*exit, (before + after) * 0.5));
        }
    }
    reading.standing /= cards.len().max(1) as f32;
    reading.back_in = sunk as f32 / past.max(1) as f32;
    for (point, length) in seam_points {
        reading.seam += length;
        let skin = solid.facing(point);
        for &view in &views {
            if skin.dot(view) < FACING {
                continue;
            }
            let from = point + view * 0.002;
            if !blinds.iter().any(|blind| hit(from, view, blind).is_some()) {
                reading.seen += length;
            }
        }
    }
    reading.seen /= views.len() as f32 * height;
    reading.seam /= height;
    reading
}

/// Where a line leaves the skin, and the last station before it: its start,
/// if it starts outside.
fn exit_of(solid: &Solid, line: &[Vec3]) -> (Vec3, usize) {
    if !solid.holds(line[0]) {
        return (line[0], 0);
    }
    for (station, pair) in line.windows(2).enumerate() {
        if solid.holds(pair[1]) {
            continue;
        }
        let (mut inside, mut outside) = (pair[0], pair[1]);
        for _ in 0..8 {
            let middle = inside.lerp(outside, 0.5);
            if solid.holds(middle) {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        return (outside, station);
    }
    (line[line.len() - 1], line.len() - 1)
}

/// `count` rolled beasts of a family, or of its crosses, each with `fur`
/// changed by `wear`.
pub(super) fn rolled(
    family: Option<Family>,
    count: usize,
    wear: impl Fn(&mut Fur),
) -> impl Iterator<Item = (i64, Avatar)> {
    (0..4000)
        .filter_map(move |seed| {
            let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(Default::default()));
            record.reroll(seed);
            let Archetype::Quadruped(params) = &mut record.archetype else {
                return None;
            };
            let keep = match family {
                Some(family) => params.hybrid.is_none() && params.family == family,
                None => params.hybrid.is_some(),
            };
            if !keep {
                return None;
            }
            wear(&mut params.fur);
            Some((seed, record))
        })
        .take(count)
        .filter_map(|(seed, record)| Some((seed, Avatar::build_for_shape(&record)?)))
}

/// One row of [`seam_table`]: its name, the zone it reads, and the fur it wears.
type Row = (&'static str, FurZone, fn(&mut Fur));

/// Prints what the sweep reads, for a person to compare: `cargo test --release
/// seam_table -- --ignored --nocapture`.
#[test]
#[ignore = "a table to read, not a check"]
fn seam_table() {
    let rows: [Row; 4] = [
        ("ruff 0.3", FurZone::Ruff, |fur| fur.ruff = 0.3),
        ("ruff 1.0", FurZone::Ruff, |fur| fur.ruff = 1.0),
        ("tail 0.8", FurZone::Tail, |fur| fur.tail = 0.8),
        ("feather", FurZone::Feathering, |fur| fur.feathering = 0.8),
    ];
    for (name, zone, wear) in rows {
        for family in FAMILIES {
            let mut total = Reading::default();
            let mut most: f32 = 0.0;
            let mut beasts = 0;
            for (_, avatar) in rolled(family, 8, wear) {
                let reading = read(&avatar, zone);
                total.cards += reading.cards;
                total.open_roots += reading.open_roots;
                total.seen += reading.seen;
                total.seam += reading.seam;
                total.standing += reading.standing;
                total.back_in += reading.back_in;
                most = most.max(reading.seen);
                beasts += 1;
            }
            let per = beasts.max(1) as f32;
            println!(
                "{name:9} {:6} cards {:4} open {:4} seam {:.3} seen {:.4} (most {:.4}) standing {:4.1} back in {:.3}",
                family.map_or("cross", Family::name),
                total.cards,
                total.open_roots,
                total.seam / per,
                total.seen / per,
                most,
                total.standing / per,
                total.back_in / per,
            );
        }
    }
}

/// Prints the dearest fur at each step of the slider, on long manes over every
/// roll a sweep builds: `cargo test --release fur_cost_table -- --ignored --nocapture`.
#[test]
#[ignore = "a table to read, not a check"]
fn fur_cost_table() {
    for step in 0..=10 {
        let stiffness = step as f32 / 10.0;
        let dearest = FAMILIES
            .into_iter()
            .flat_map(|family| {
                [0.8f32, 1.0, 1.5, 3.0].into_iter().flat_map(move |length| {
                    rolled(family, 8, move |fur: &mut Fur| {
                        fur.mane = length;
                        fur.mane_stiffness = stiffness;
                    })
                })
            })
            .map(|(_, avatar)| avatar.parts.fur.as_ref().map_or(0, FurGrowth::tris))
            .max()
            .unwrap_or(0);
        println!("stiffness {stiffness:3.1}: dearest fur {dearest}");
    }
}

/// Every family and its crosses.
pub(super) const FAMILIES: [Option<Family>; 4] = [
    Some(Family::Horse),
    Some(Family::Dog),
    Some(Family::Cat),
    None,
];
