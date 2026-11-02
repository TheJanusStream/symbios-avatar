//! Where a card meets the skin, measured on the built beast (#386).
//!
//! A card that stands off the crest shows the line it enters the skin on: a
//! straight cut across a lock of hair, with skin right behind it. These read
//! that line off the drawn mesh, because the question is what a viewer sees:
//! each card's edges are walked from the root until they leave the body, and
//! each such point is asked whether any card lies between it and a camera.
//!
//! And what a mane's stiffness does above that line (#389): how far its outline
//! stands off the crest, side on, which is what a whole body's picture shows,
//! with the same cards at the same length all the way up the slider.

use glam::Vec3;

use super::{Bed, Crest, FurGrowth, FurZone};
use crate::mesh::PolyMesh;
use crate::plan::{Coat, Family, Fur};
use crate::texture::coat::CoatBody;
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
struct Solid {
    triangles: Vec<[Vec3; 3]>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
}

impl Solid {
    fn of(body: &PolyMesh) -> Self {
        Self {
            triangles: triangles(body),
            positions: body.positions.clone(),
            normals: body.vertex_normals(),
        }
    }

    /// Whether `at` is inside: an odd count of crossings along a ray no mesh
    /// is aligned with.
    fn holds(&self, at: Vec3) -> bool {
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

fn triangles(mesh: &PolyMesh) -> Vec<[Vec3; 3]> {
    mesh.triangulated()
        .iter()
        .map(|corners| corners.map(|corner| mesh.positions[corner as usize]))
        .collect()
}

/// How far along `ray` from `from` it crosses `triangle`, if it does.
fn hit(from: Vec3, ray: Vec3, [a, b, c]: &[Vec3; 3]) -> Option<f32> {
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
    /// The cosine of the sharpest turn a card's width takes between two stations
    /// past its root.
    sharpest: f32,
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
    let sharpest = cards
        .iter()
        .flat_map(|card| card.edges.windows(2).skip(1))
        .map(|pair| {
            let (from, to) = (pair[0].1 - pair[0].0, pair[1].1 - pair[1].0);
            from.normalize_or_zero().dot(to.normalize_or_zero())
        })
        .fold(1.0, f32::min);
    let mut reading = Reading {
        cards: cards.len(),
        sharpest,
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
fn rolled(
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
    // The mane's own sweep at each stiffness: 128 beasts a line of three (#389).
    for stiffness in STIFFNESSES {
        let mut seam = (0.0f32, 0.0f32, 0.0f32, 0usize, 0usize);
        for family in FAMILIES {
            let beasts: Vec<(f32, Avatar)> = [0.2f32, 0.8, 1.5, 3.0]
                .into_iter()
                .flat_map(|length| {
                    rolled(family, 8, move |fur: &mut Fur| {
                        fur.mane = length;
                        fur.mane_stiffness = stiffness;
                    })
                    .map(move |(_, avatar)| (length, avatar))
                })
                .collect();
            let readings = read_all(&beasts, FurZone::Mane);
            for ((_, _), reading) in beasts.iter().zip(&readings) {
                seam.0 += reading.seen;
                seam.1 = seam.1.max(reading.seen);
                seam.2 = seam.2.max(reading.standing);
                seam.3 += reading.open_roots;
                seam.4 += reading.cards;
            }
            let outlines: Vec<f32> = beasts
                .iter()
                .filter(|(length, _)| *length == 0.8)
                .map(|(_, avatar)| mean(&outline_of(avatar)))
                .collect();
            let tris = beasts
                .iter()
                .map(|(_, avatar)| avatar.parts.fur.as_ref().map_or(0, FurGrowth::tris))
                .max()
                .unwrap_or(0);
            println!(
                "stiffness {stiffness:4.2} {:6} outline at 0.8 {:.4} dearest fur {tris}",
                family.map_or("cross", Family::name),
                mean(&outlines),
            );
        }
        println!(
            "stiffness {stiffness:4.2} 128 beasts: seen mean {:.4} most {:.4}, steepest {:.1}, open roots {} of {} cards",
            seam.0 / 128.0,
            seam.1,
            seam.2,
            seam.3,
            seam.4,
        );
    }
    let rows: [Row; 5] = [
        ("forelock", FurZone::Forelock, |fur| fur.mane = 0.8),
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

/// The stiffnesses a mane's tests judge: none, the default and all (#389).
const STIFFNESSES: [f32; 3] = [0.0, Fur::DEFAULT_MANE_STIFFNESS, 1.0];

/// Every roll a mane's tests judge: each family and its crosses, short, long and
/// at the envelope's end, two rolls of each at each of [`STIFFNESSES`] and one at
/// each stiffness between them on a [`SWEEP`] of the slider.
fn maned() -> impl Iterator<Item = (String, f32, Avatar)> {
    FAMILIES.into_iter().flat_map(|family| {
        [0.2f32, 0.8, 3.0].into_iter().flat_map(move |length| {
            SWEEP.into_iter().flat_map(move |stiffness| {
                let rolls = if STIFFNESSES.contains(&stiffness) {
                    2
                } else {
                    1
                };
                rolled(family, rolls, move |fur: &mut Fur| {
                    fur.mane = length;
                    fur.mane_stiffness = stiffness;
                })
                .map(move |(seed, avatar)| {
                    let name = family.map_or("cross", Family::name);
                    (
                        format!("{name} seed {seed} mane {length} stiffness {stiffness}"),
                        stiffness,
                        avatar,
                    )
                })
            })
        })
    })
}

/// How far a mane's outline stands off the neck's crest, side on, along the
/// middle of the crest, as shares of the back's height: for each of a dozen
/// stretches of the crest, the highest any solid part of a card reaches above
/// the skin there, read square to the crest.
///
/// "Solid" is the mask's: a card's frayed tip is half air, but its fraying is
/// what an outline of hair is, so the last of the card counts at its mask's
/// edge rather than its own.
fn outline_of(avatar: &Avatar) -> Vec<f32> {
    with_crest(avatar, |crest, height| outline_along(avatar, crest, height))
}

/// Hands `read` the beast's crest and the back's height.
fn with_crest<T>(avatar: &Avatar, read: impl FnOnce(&Crest, f32) -> T) -> T {
    let parts = &avatar.parts;
    let anatomy = CoatBody::of(&avatar.rig, None, None);
    let coat = Coat::default();
    let bed = Bed {
        body: &parts.body,
        zones: &parts.zones,
        rig: &avatar.rig,
        weights: &parts.weights,
        anatomy: &anatomy,
        coat: &coat,
        ears: None,
        worn: 0,
    };
    read(&Crest::of(&bed).expect("a neck"), anatomy.height)
}

/// [`outline_of`], on `crest`, in shares of `height`.
fn outline_along(avatar: &Avatar, crest: &Crest, height: f32) -> Vec<f32> {
    const BINS: usize = 12;
    let first = crest.line[0].at;
    let last = crest.line[crest.line.len() - 1].at;
    let run = last - first;
    let mut bins = [0.0f32; BINS];
    for card in cards(avatar, FurZone::Mane) {
        for (station, (edges, &along)) in card.edges.iter().zip(&card.along).enumerate() {
            // A straight card has few stations: read it where its mask gives out,
            // between the two either side, and no further.
            let edges = match station.checked_sub(1) {
                Some(last) if along > OUTLINE && card.along[last] < OUTLINE => {
                    let share = (OUTLINE - card.along[last]) / (along - card.along[last]);
                    (
                        card.edges[last].0.lerp(edges.0, share),
                        card.edges[last].1.lerp(edges.1, share),
                    )
                }
                _ if along > OUTLINE => continue,
                _ => *edges,
            };
            for point in [edges.0, edges.1, (edges.0 + edges.1) * 0.5] {
                // Side on: the sideways part of a card is not in the picture.
                let flat = Vec3::new(0.0, point.y, point.z);
                let near = crest.near(flat);
                let share = (flat - first).dot(run) / run.length_squared();
                if !(0.2..0.7).contains(&share) {
                    continue;
                }
                let bin = (((share - 0.2) / 0.5 * BINS as f32) as usize).min(BINS - 1);
                let skin = Vec3::new(0.0, near.at.y, near.at.z);
                bins[bin] = bins[bin].max((flat - skin).dot(near.up));
            }
        }
    }
    bins.iter().map(|bin| bin / height).collect()
}

/// How far along a card it still counts for an outline: its mask holds together
/// to about there (see [`crate::hair::mask`]).
const OUTLINE: f32 = 0.85;

fn mean(values: &[f32]) -> f32 {
    values.iter().sum::<f32>() / values.len().max(1) as f32
}

/// Reads every one of `beasts` on threads of its own: a reading walks every
/// card's lines through every triangle of the body, and takes seconds a beast.
fn read_all<T: Send + Sync>(beasts: &[(T, Avatar)], zone: FurZone) -> Vec<Reading> {
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get().min(8));
    let chunk = beasts.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let handles: Vec<_> = beasts
            .chunks(chunk)
            .map(|some| {
                scope.spawn(move || {
                    some.iter()
                        .map(|(_, avatar)| read(avatar, zone))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("a reading"))
            .collect()
    })
}

/// The cosine of the sharpest turn a mane's card may take: seventy-five degrees.
const WHOLE: f32 = 0.26;

/// How much seam a camera may see, to the default stiffness and at the top of
/// the slider, rising evenly between the two.
const SEEN_TO_THE_DEFAULT: f32 = 0.06;
/// See [`SEEN_TO_THE_DEFAULT`].
const SEEN_AT_THE_TOP: f32 = 0.19;

#[test]
fn a_mane_hides_where_its_cards_enter_the_skin() {
    // One test rather than three, each reading the same dear sweep.
    let beasts: Vec<((String, f32), Avatar)> = maned()
        .map(|(name, stiffness, avatar)| ((name, stiffness), avatar))
        .collect();
    for (((name, stiffness), _), reading) in beasts.iter().zip(read_all(&beasts, FurZone::Mane)) {
        let stiffness = *stiffness;
        assert!(reading.cards > 60, "{name}: only {} cards", reading.cards);
        // The owner's complaint in one number: how much of the line where a
        // card meets the neck a camera sees, round the beast and from above it,
        // in shares of the back's height. A mane of standing cards shows about
        // a third of its height's worth from any angle (0.23 to 0.58 over a
        // wider sweep), which is the neck's whole length; the draped one under
        // 0.03. Up the slider each card's foot lies over the other side's seams
        // until there is too little foot left to, and past that a root stands in
        // plain view: pinched thinner to hide it, it left rungs of coat between
        // the cards, which read far worse (#389). There it hides by its colour:
        // the skin under a dark mane is painted as the mane
        // (`crate::texture::coat::ManeRoots`), and on any other coat a card is
        // the colour of the skin it leaves, as a ruff's are, which reads 0.7 of
        // this measure and shows nothing. Bounds set after reading `seam_table`
        // and `seam_by_stiffness`.
        let past = (stiffness - Fur::DEFAULT_MANE_STIFFNESS).max(0.0)
            / (1.0 - Fur::DEFAULT_MANE_STIFFNESS);
        let bound = SEEN_TO_THE_DEFAULT + (SEEN_AT_THE_TOP - SEEN_TO_THE_DEFAULT) * past;
        assert!(reading.seen < bound, "{name} shows its seam: {reading:?}");
        // Every ribbon whole, at every value. A card bends where its foot turns up
        // into its run, by about sixty degrees at the sharpest. One held still across
        // the neck goes on along the crest, and its width swings round to follow.
        assert!(reading.sharpest > WHOLE, "{name}: {reading:?}");
        // Each root's whole line, not just its middle: a straight line laid
        // across a curved crest stands off it at both ends. The cards let off
        // are the tiny ones where a mane thins out at either end of the crest.
        assert!(
            reading.open_roots * 5 < reading.cards,
            "{name}: {reading:?}"
        );
        // How steeply a card's lines leave the skin: a fin stands at 59 to 63
        // degrees over a wider sweep. Bound set after seeing the draped mane's
        // own 41 to 50, between the two. A stiff card rises past its foot, not
        // out of the skin (#389).
        assert!(reading.standing < 55.0, "{name}: {reading:?}");
    }
}

#[test]
fn a_mane_falls_both_ways_all_along_its_crest() {
    // A side taken from where a root happens to sit runs in long strings, and
    // a string on one side is a bald stretch on the other.
    for (name, _, avatar) in maned() {
        let sides: Vec<bool> = cards(&avatar, FurZone::Mane)
            .iter()
            .map(|card| {
                let (left, right) = card.edges[card.edges.len() - 1];
                left.x + right.x > 0.0
            })
            .collect();
        let longest = sides
            .chunk_by(|one, two| one == two)
            .map(<[bool]>::len)
            .max()
            .unwrap_or(0);
        assert!(
            longest <= 3,
            "{name}: {longest} cards in a row fall one way"
        );
    }
}

#[test]
fn a_mane_leaves_no_gap_down_either_side_of_the_neck() {
    // Cards combed along each bone's own run fan apart wherever the bones
    // turn, and leave a bare wedge of neck: measured along the mane's whole
    // run, halfway down each card, one side at a time, no gap may be much
    // wider than the usual one, whatever a root's own jitter does.
    for (name, _, avatar) in maned() {
        let cards = cards(&avatar, FurZone::Mane);
        let middle = |(left, right): (Vec3, Vec3)| (left + right) * 0.5;
        let (Some(first), Some(last)) = (cards.first(), cards.last()) else {
            panic!("{name}: no mane");
        };
        let run = (middle(last.edges[0]) - middle(first.edges[0])).normalize_or(Vec3::Y);
        for side in [false, true] {
            let mut along: Vec<f32> = cards
                .iter()
                .filter(|card| (card.edges[card.edges.len() - 1].0.x > 0.0) == side)
                .filter_map(|card| {
                    // Halfway exactly, between stations: a stiff card spends its
                    // on its bend, and the next along may be far past halfway.
                    let station = card.along.iter().position(|&along| along >= 0.5)?;
                    let at = middle(card.edges[station]);
                    let Some(before) = station.checked_sub(1) else {
                        return Some(at.dot(run));
                    };
                    let (from, to) = (card.along[before], card.along[station]);
                    let share = (0.5 - from) / (to - from).max(f32::EPSILON);
                    Some(middle(card.edges[before]).lerp(at, share).dot(run))
                })
                .collect();
            along.sort_by(f32::total_cmp);
            let gaps: Vec<f32> = along.windows(2).map(|pair| pair[1] - pair[0]).collect();
            // The mane thins out at either end, where its cards are its
            // shortest and a gap is its business.
            let ends = gaps.len() / 7;
            // Against the gaps around it, not the whole mane's: a long one
            // gathers at the withers, where gaps are small, and a mean of the
            // two called the ordinary gaps above them wedges (#389). A wedge is one
            // gap in a run of smaller ones.
            for at in ends..gaps.len() - ends {
                let around: Vec<f32> = (at.saturating_sub(4)..(at + 5).min(gaps.len()))
                    .filter(|&other| other != at)
                    .map(|other| gaps[other])
                    .collect();
                let usual = mean(&around);
                assert!(
                    gaps[at] <= usual * GAP,
                    "{name}: a gap of {}, usually {usual}",
                    gaps[at]
                );
            }
        }
    }
}

/// How much wider than those around it a gap may be. Read off the sweep: limp and
/// combed even, a mane's worst is 2.84; a stiff card has no comb, and its gaps are
/// its roots' own jitter, 4.04 at worst (a millimetre or two on a small neck).
const GAP: f32 = 4.5;

#[test]
fn a_mane_s_cards_never_turn_over() {
    // A card draped round a neck faces further and further from the way its
    // root does. Past a right angle the loft turns it over, and a quad between
    // the two ways round is a bow-tie.
    for (name, _, avatar) in maned() {
        for card in cards(&avatar, FurZone::Mane) {
            for pair in card.edges.windows(2) {
                let (one, two) = (pair[0].1 - pair[0].0, pair[1].1 - pair[1].0);
                assert!(one.dot(two) >= 0.0, "{name}: a bow-tie");
            }
        }
    }
}

/// How long a card is along itself, read off its stations' middles.
fn along_itself(card: &Card) -> f32 {
    card.edges
        .windows(2)
        .map(|pair| ((pair[1].0 + pair[1].1) - (pair[0].0 + pair[0].1)).length() * 0.5)
        .sum()
}

/// The longest cards of a mane, the ones its mask leaves whole: their middle
/// length, in metres.
fn card_length(avatar: &Avatar) -> f32 {
    let mut lengths: Vec<f32> = cards(avatar, FurZone::Mane)
        .iter()
        .map(along_itself)
        .collect();
    lengths.sort_by(f32::total_cmp);
    lengths[lengths.len() * 3 / 4]
}

/// Prints, for a person to read, what each stiffness does to a mane: `cargo test
/// --release stiffness_table -- --ignored --nocapture`.
#[test]
#[ignore = "a table to read, not a check"]
fn stiffness_table() {
    for family in FAMILIES {
        for mane in [0.2f32, 0.8, 1.5] {
            for stiffness in SWEEP {
                let beasts: Vec<_> = rolled(family, 3, move |fur: &mut Fur| {
                    fur.mane = mane;
                    fur.mane_stiffness = stiffness;
                })
                .map(|(_, avatar)| avatar)
                .collect();
                let outline: Vec<f32> = beasts.iter().map(|a| mean(&outline_of(a))).collect();
                let heights: Vec<f32> = beasts
                    .iter()
                    .map(|a| card_length(a) / with_crest(a, |_, height| height))
                    .collect();
                let count = beasts
                    .iter()
                    .map(|a| cards(a, FurZone::Mane).len())
                    .min()
                    .unwrap_or(0);
                let tris = beasts
                    .iter()
                    .map(|a| {
                        a.parts.fur.as_ref().map_or(0, |fur| {
                            fur.grown
                                .iter()
                                .filter(|grown| grown.zone == FurZone::Mane)
                                .map(|grown| grown.tris)
                                .sum::<usize>()
                        })
                    })
                    .max()
                    .unwrap_or(0);
                println!(
                    "{:6} mane {mane:3.1} stiffness {stiffness:4.2}: outline {:.4} length {:.4} (cut {:.4}) cards {count} mane tris {tris}",
                    family.map_or("cross", Family::name),
                    mean(&outline),
                    mean(&heights),
                    FurZone::Mane.cut().reach * mane,
                );
            }
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

/// Prints how much seam shows up the slider, on fewer beasts than [`seam_table`]'s
/// line of each stiffness it judges: `cargo test --release seam_by_stiffness --
/// --ignored --nocapture`.
#[test]
#[ignore = "a table to read, not a check"]
fn seam_by_stiffness() {
    for stiffness in SWEEP {
        let beasts: Vec<(f32, Avatar)> = FAMILIES
            .into_iter()
            .flat_map(|family| {
                [0.2f32, 0.8, 1.5, 3.0].into_iter().flat_map(move |length| {
                    rolled(family, 2, move |fur: &mut Fur| {
                        fur.mane = length;
                        fur.mane_stiffness = stiffness;
                    })
                    .map(move |(_, avatar)| (length, avatar))
                })
            })
            .collect();
        let readings = read_all(&beasts, FurZone::Mane);
        let seen: Vec<f32> = readings.iter().map(|reading| reading.seen).collect();
        for wanted in [0.2f32, 0.8, 1.5, 3.0] {
            let some: Vec<f32> = beasts
                .iter()
                .zip(&seen)
                .filter(|((length, _), _)| *length == wanted)
                .map(|(_, seen)| *seen)
                .collect();
            println!(
                "  mane {wanted:3.1}: seen mean {:.4} most {:.4}",
                mean(&some),
                some.iter().copied().fold(0.0, f32::max)
            );
        }
        let most = seen.iter().copied().fold(0.0, f32::max);
        let open: usize = readings.iter().map(|reading| reading.open_roots).sum();
        let steepest = readings
            .iter()
            .map(|reading| reading.standing)
            .fold(0.0, f32::max);
        println!(
            "stiffness {stiffness:4.2}: seen mean {:.4} most {most:.4}, steepest {steepest:.1}, open {open}",
            mean(&seen)
        );
    }
}

/// Every family and its crosses.
const FAMILIES: [Option<Family>; 4] = [
    Some(Family::Horse),
    Some(Family::Dog),
    Some(Family::Cat),
    None,
];

/// The stiffnesses a sweep of the slider reads.
const SWEEP: [f32; 7] = [0.0, 0.1, 0.2, Fur::DEFAULT_MANE_STIFFNESS, 0.6, 0.8, 1.0];

/// Each family's and its crosses' first `rolls` beasts with a mane `length` long,
/// at every stiffness of the [`SWEEP`], one line of them a roll.
fn along_the_slider(length: f32, rolls: usize) -> impl Iterator<Item = (String, Vec<Avatar>)> {
    FAMILIES.into_iter().flat_map(move |family| {
        (0..rolls).map(move |roll| {
            let name = format!(
                "{} roll {roll} mane {length}",
                family.map_or("cross", Family::name)
            );
            let line = SWEEP
                .into_iter()
                .map(|stiffness| {
                    rolled(family, roll + 1, move |fur: &mut Fur| {
                        fur.mane = length;
                        fur.mane_stiffness = stiffness;
                    })
                    .nth(roll)
                    .map(|(_, avatar)| avatar)
                    .expect("a roll")
                })
                .collect();
            (name, line)
        })
    })
}

#[test]
fn a_mane_is_the_same_cards_at_every_stiffness() {
    // No value of the slider where a set of cards appears or vanishes: a crest
    // of its own, grown over the falling cards from some stiffness on, is the
    // one thing the owner asked to be rid of (#389).
    for length in [0.2f32, 0.8, 1.5] {
        for (name, line) in along_the_slider(length, 2) {
            let counts: Vec<usize> = line
                .iter()
                .map(|avatar| cards(avatar, FurZone::Mane).len())
                .collect();
            assert!(
                counts.iter().all(|&count| count == counts[0]),
                "{name}: {counts:?}"
            );
        }
    }
}

#[test]
fn a_mane_is_as_long_at_every_stiffness() {
    // `mane` is how long a card is; stiffness only bends it.
    for length in [0.2f32, 0.8, 1.5] {
        for (name, line) in along_the_slider(length, 2) {
            let lengths: Vec<f32> = line.iter().map(card_length).collect();
            let longest = lengths.iter().copied().fold(0.0, f32::max);
            let shortest = lengths.iter().copied().fold(f32::MAX, f32::min);
            assert!(longest < shortest * LENGTH_BOUND, "{name}: {lengths:?}");
        }
    }
}

/// How much longer a mane's longest card on the slider is than its shortest, at
/// most: 1.283 at worst on the sweep, a horse's limp card against a stiff one, the
/// limp curve being the longer for how far its neck stands off its bone.
/// Provenance: **measured** (`stiffness_table`).
const LENGTH_BOUND: f32 = 1.33;

#[test]
fn a_mane_stands_higher_the_stiffer_it_is() {
    // The owner's picture in one number: how far the mane's outline stands above
    // the neck's skin, side on, along the middle of its crest, up the slider.
    for length in [0.3f32, 0.8, 1.5] {
        for (name, line) in along_the_slider(length, 2) {
            let bins: Vec<Vec<f32>> = line.iter().map(outline_of).collect();
            let means: Vec<f32> = bins.iter().map(|bins| mean(bins)).collect();
            // Stepping over 0.1, where a limp card's arch hands over and a couple
            // of millimetres either way are nobody's picture: six values.
            assert!(means[2] > means[0] * RISES, "{name}: {means:?}");
            for pair in means[2..6].windows(2) {
                assert!(pair[1] > pair[0] * RISES, "{name}: {means:?}");
            }
            // By 0.8 a long mane has all the height its length gives it: from there
            // to the top of the slider what changes is how far its tips bend over.
            assert!(means[6] > means[5] * HOLDS, "{name}: {means:?}");
            // At full stiffness, about as high as the card is long, as far as
            // the outline reads it: its last fraying tip left out.
            let reach = FurZone::Mane.cut().reach * length;
            assert!(
                means[6] > reach * STANDS_ALL_OF_IT,
                "{name}: {} against a cut of {reach}",
                means[6]
            );
            // And it is a crest along the neck, not a tuft here and there. The
            // lowest stretch is toward the withers, where its cards thin out: 0.45
            // of the mean at worst on the sweep.
            for (bins, at) in bins.iter().zip(&means).skip(3) {
                let lowest = bins.iter().copied().fold(f32::MAX, f32::min);
                assert!(lowest > at * 0.4, "{name}: {lowest} against {at}");
            }
        }
    }
}

/// How much higher each step up the slider stands, at least. Read off the sweep:
/// the thinnest step, off a limp mane's own arch on a long-maned cross, is 1.042.
const RISES: f32 = 1.01;
/// How much of its height at 0.8 a mane keeps at full stiffness. Read off the
/// sweep: 0.98.
const HOLDS: f32 = 0.95;
/// How much of its cut a mane stands at full stiffness, at least, where its outline
/// is read short of its tip and its cards thin out. Read off the sweep: 0.839.
const STANDS_ALL_OF_IT: f32 = 0.8;
