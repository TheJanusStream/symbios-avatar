//! The mane's solid, measured on built beasts (#407).

use std::collections::HashMap;

use glam::{Quat, Vec3};

use super::super::seam::{FAMILIES, Solid, rolled, triangles};
use super::super::{FurGrowth, FurZone};
use super::{CLEAR, FAR, Grid, LOCKS, NEAR, NECK_LOCKS, TIPPED, UNDER, capsules};
use crate::anim::Pose;
use crate::plan::{
    Agouti, Coat, Family, Fur, Ornament, OrnamentKind, Ornaments, Pigment, QuadrupedParams, White,
    Zone,
};
use crate::texture::coat::{CoatBody, MANE_SHADE};
use crate::{Archetype, Avatar, AvatarConfig, AvatarRecord};

/// The solid read back out of a fur mesh: both surfaces, column by column, row by
/// row, root first; the first [`super::LOCKS`] locks' worth of the mesh's vertices
/// and faces, laid out lock by lock, the outer surface then the inner.
struct Read {
    grid: Grid,
    outer: Vec<Vec3>,
    inner: Vec<Vec3>,
    /// How many vertices and faces of the mesh are the solid's two surfaces.
    vertices: usize,
    faces: usize,
}

impl Read {
    fn of(fur: &FurGrowth, grid: Grid) -> Self {
        let (per, rows) = (grid.per_lock, grid.rows);
        let columns = LOCKS * per + 1;
        let mut outer = vec![Vec3::ZERO; columns * rows];
        let mut inner = vec![Vec3::ZERO; columns * rows];
        let run = (per + 1) * rows;
        for lock in 0..LOCKS {
            for (surface, start) in [(&mut outer, 0), (&mut inner, run)] {
                for place in 0..=per {
                    for row in 0..rows {
                        let vertex = lock * 2 * run + start + place * rows + row;
                        surface[(lock * per + place) * rows + row] = fur.mesh.positions[vertex];
                    }
                }
            }
        }
        Self {
            grid,
            outer,
            inner,
            vertices: LOCKS * 2 * run,
            faces: LOCKS * 2 * per * (rows - 1),
        }
    }

    fn columns(&self) -> usize {
        LOCKS * self.grid.per_lock + 1
    }

    fn at(&self, column: usize, row: usize) -> usize {
        column * self.grid.rows + row
    }

    /// Whether a column grows on the neck.
    fn on_neck(&self, column: usize) -> bool {
        column <= NECK_LOCKS * self.grid.per_lock
    }
}

/// A beast of `family` (a cross of a dog and a horse for `None`) wearing a mane
/// `length` long at `stiffness`, its record changed by `wear`.
fn beast(seed: i64, length: f32, stiffness: f32, wear: impl Fn(&mut QuadrupedParams)) -> Avatar {
    built(seed, length, stiffness, wear).1
}

/// [`beast`], with its record.
fn built(
    seed: i64,
    length: f32,
    stiffness: f32,
    wear: impl Fn(&mut QuadrupedParams),
) -> (AvatarRecord, Avatar) {
    let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
    record.reroll(seed);
    let Archetype::Quadruped(params) = &mut record.archetype else {
        unreachable!("a beast")
    };
    params.fur.mane = length;
    params.fur.mane_stiffness = stiffness;
    wear(params);
    let avatar = Avatar::build_for_shape(&record).expect("the beast builds");
    (record, avatar)
}

/// Each family and the crosses, two rolls each, short, long and at the envelope's
/// end, limp, at the default and stiff.
fn maned() -> Vec<(String, Avatar)> {
    let mut beasts = Vec::new();
    for family in FAMILIES {
        for length in [0.3f32, 0.8, 3.0] {
            for stiffness in [0.0, Fur::DEFAULT_MANE_STIFFNESS, 1.0] {
                for (seed, avatar) in rolled(family, 2, move |fur: &mut Fur| {
                    fur.mane = length;
                    fur.mane_stiffness = stiffness;
                }) {
                    let name = family.map_or("cross", Family::name);
                    beasts.push((
                        format!("{name} seed {seed} mane {length} stiffness {stiffness}"),
                        avatar,
                    ));
                }
            }
        }
    }
    beasts
}

fn fur(avatar: &Avatar) -> &FurGrowth {
    avatar.parts.fur.as_ref().expect("fur")
}

fn height(avatar: &Avatar) -> f32 {
    CoatBody::of(&avatar.rig, None, None).height
}

/// Welds the solid's faces by where their corners are, and counts how many faces
/// each edge has: a closed solid has two on every one.
fn edges(fur: &FurGrowth, read: &Read) -> (HashMap<(usize, usize), usize>, Vec<usize>) {
    let mut welded: HashMap<[u32; 3], usize> = HashMap::new();
    let ids: Vec<usize> = fur.mesh.positions[..read.vertices]
        .iter()
        .map(|at| {
            let next = welded.len();
            *welded
                .entry(at.to_array().map(f32::to_bits))
                .or_insert(next)
        })
        .collect();
    let mut count = HashMap::new();
    for face in &fur.mesh.faces[..read.faces] {
        for (corner, &one) in face.iter().enumerate() {
            let two = face[(corner + 1) % face.len()];
            let (one, two) = (ids[one as usize], ids[two as usize]);
            if one != two {
                *count.entry((one.min(two), one.max(two))).or_insert(0) += 1;
            }
        }
    }
    (count, ids)
}

#[test]
fn a_mane_is_one_closed_solid_from_the_withers_to_the_forelock() {
    // The owner's picture: one mane, continuous from the back over the neck, between
    // the ears and onto the forehead. Two surfaces lofted on shared columns, closed
    // where the solid has no thickness: at its root, its hem and either end.
    for (name, avatar) in maned() {
        let fur = fur(&avatar);
        assert_eq!(fur.grown[0].zone, FurZone::Mane, "{name}");
        assert!(
            fur.grown
                .iter()
                .all(|grown| grown.zone != FurZone::Forelock),
            "{name}: a forelock of its own"
        );
        let read = Read::of(fur, NEAR);
        let (edges, ids) = edges(fur, &read);
        // Where its two surfaces lie flat together on the skin, a closing edge is
        // shared by four faces: still closed. An open edge has an odd count.
        let open = edges.values().filter(|&&faces| faces % 2 == 1).count();
        assert_eq!(
            open,
            0,
            "{name}: {open} edges of {} not closed",
            edges.len()
        );
        // One piece: every face reaches every other across shared edges.
        let mut parent: Vec<usize> = (0..ids.len()).collect();
        fn root(parent: &mut [usize], at: usize) -> usize {
            let mut at = at;
            while parent[at] != at {
                parent[at] = parent[parent[at]];
                at = parent[at];
            }
            at
        }
        for &(one, two) in edges.keys() {
            let (one, two) = (root(&mut parent, one), root(&mut parent, two));
            parent[one] = two;
        }
        let pieces: std::collections::HashSet<usize> = (0..ids.len())
            .map(|at| root(&mut parent, ids[at]))
            .collect();
        assert_eq!(pieces.len(), 1, "{name}: {} pieces", pieces.len());
        // And no gap along its root line, from the withers over the poll: no step
        // between two columns' roots longer than a few of the usual one.
        let steps: Vec<f32> = (1..read.columns())
            .map(|column| {
                read.outer[read.at(column, 0)].distance(read.outer[read.at(column - 1, 0)])
            })
            .collect();
        let usual = {
            let mut sorted = steps.clone();
            sorted.sort_by(f32::total_cmp);
            sorted[sorted.len() / 2]
        };
        let most = steps.iter().copied().fold(0.0, f32::max);
        assert!(
            most < usual * ROOT_STEP,
            "{name}: a step of {most} against {usual}"
        );
    }
}

/// How much longer than the middle step along the root line its longest may be.
/// Read off the sweep: a lock is 0.6 to 1.45 of the others' width and the head's
/// locks are wider than the neck's.
const ROOT_STEP: f32 = 6.0;

#[test]
fn a_mane_s_roots_are_in_the_skin_and_no_skin_shows_through_it() {
    // #389: where hair enters the skin, no cut edge stands out of it. The solid's
    // root row starts under the skin on both surfaces; and from a little way out,
    // its outer surface is outside the skin, so no coat shows through the mane.
    let mut worst = 0.0f32;
    for (name, avatar) in maned() {
        let fur = fur(&avatar);
        let read = Read::of(fur, NEAR);
        let solid = Solid::of(&avatar.parts.body);
        let open = (0..read.columns())
            .flat_map(|column| {
                [
                    read.outer[read.at(column, 0)],
                    read.inner[read.at(column, 0)],
                ]
            })
            .filter(|at| !solid.holds(*at))
            .count();
        assert_eq!(open, 0, "{name}: {open} roots out of the skin");
        // Each outer face's middle, past where the solid comes out of the skin.
        let mut through = 0usize;
        let mut faces = 0usize;
        for column in 0..read.columns() - 1 {
            for row in OUT_FROM..read.grid.rows - 2 {
                let middle = (read.outer[read.at(column, row)]
                    + read.outer[read.at(column + 1, row)]
                    + read.outer[read.at(column, row + 1)]
                    + read.outer[read.at(column + 1, row + 1)])
                    * 0.25;
                faces += 1;
                through += usize::from(solid.holds(middle));
            }
        }
        let share = through as f32 / faces as f32;
        worst = worst.max(share);
        assert!(
            share <= THROUGH,
            "{name}: {through} of {faces} faces in the skin"
        );
    }
    println!("the most skin through a mane: {worst:.4}");
}

/// From which row a mane's outer surface is out of the skin it was sunk in.
const OUT_FROM: usize = 2;

/// What share of a mane's outer faces may have their middle under the skin: a face
/// whose middle is a hair under the skin it lies on is coat a pixel wide. Read off
/// the sweep: 0.0 to 0.016.
const THROUGH: f32 = 0.16;

#[test]
fn a_lying_mane_s_inner_surface_is_on_the_skin() {
    // Laid on the skin, the inner surface leaves no shadowed notch under a lock
    // (#407's first round): read how far each of its vertices is from the skin, on
    // manes that lie, down the part of each column that lies on the neck or the head.
    let mut most = 0.0f32;
    for (name, avatar) in maned()
        .into_iter()
        .filter(|(name, _)| !name.ends_with("stiffness 1") && !name.contains("mane 3"))
    {
        let fur = fur(&avatar);
        let read = Read::of(fur, NEAR);
        let skin = triangles(&avatar.parts.body);
        let height = height(&avatar);
        let mut off: Vec<f32> = Vec::new();
        for column in 1..read.columns() - 1 {
            for row in 2..read.grid.rows * 7 / 10 {
                off.push(distance(&skin, read.inner[read.at(column, row)]) / height);
            }
        }
        off.sort_by(f32::total_cmp);
        let ninety = off[off.len() * 9 / 10];
        most = most.max(ninety);
        assert!(
            ninety < ON_SKIN,
            "{name}: nine in ten within {ninety} of the skin"
        );
    }
    println!("the furthest nine in ten of an inner surface stand off the skin: {most:.5}");
}

/// How far off the skin nine in ten of a lying mane's inner vertices may be, as a
/// share of the back's height. Read off the sweep: at most 0.0047 (the inner surface
/// lies 0.0015 off the skin where it is laid, and further where a lock falls free
/// past the neck's side).
const ON_SKIN: f32 = 0.004;

/// How far `at` is from the nearest of `triangles`.
fn distance(triangles: &[[Vec3; 3]], at: Vec3) -> f32 {
    triangles
        .iter()
        .map(|&[a, b, c]| closest(at, a, b, c).distance(at))
        .fold(f32::MAX, f32::min)
}

/// The point of the triangle `a b c` nearest `p` (Ericson's).
fn closest(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

/// A nod or a turn at the poll: the occiput turned, and the neck under it with
/// `neck` of the turn at each of its joints.
fn bent(avatar: &Avatar, turn: Quat, neck: f32) -> Pose {
    let rig = &avatar.rig;
    let mut pose = Pose::rest(rig);
    for &joint in &rig.in_zone(Zone::Neck) {
        pose.rotations[joint] = Quat::IDENTITY.slerp(turn, neck);
    }
    if let Some(&occiput) = rig.in_zone(Zone::Head).first() {
        pose.rotations[occiput] = turn;
    }
    pose
}

/// The most any edge of `faces` (of at least `shortest` at rest) stretches from
/// `rest` to `posed`, as a ratio.
fn stretch<'a>(
    faces: impl Iterator<Item = &'a Vec<u32>>,
    rest: &[Vec3],
    posed: &[Vec3],
    shortest: f32,
) -> f32 {
    let mut most = 1.0f32;
    for face in faces {
        for (corner, &one) in face.iter().enumerate() {
            let (one, two) = (one as usize, face[(corner + 1) % face.len()] as usize);
            let length = rest[one].distance(rest[two]);
            if length >= shortest {
                most = most.max(posed[one].distance(posed[two]) / length);
            }
        }
    }
    most
}

#[test]
fn a_nod_and_a_turn_bend_a_mane_at_the_poll_without_tearing_it() {
    // Every point of the solid is bound as the skin under it, so a nod or a turn
    // bends it as it bends the neck and the head: nowhere does the solid open where
    // two of its locks meet or its two surfaces close, and no edge of it stretches
    // much past what the skin under it does. The renderer's own nod and turn, every
    // neck joint and the occiput turned: far past a beast's reach, the skin's own
    // edges stretch to 1.6 to 2.4 times their length.
    let mut worst = 0.0f32;
    for (name, avatar) in maned() {
        let lies = !name.ends_with("stiffness 1");
        let fur = fur(&avatar);
        let read = Read::of(fur, NEAR);
        let height = height(&avatar);
        let (_, ids) = edges(fur, &read);
        let body = &avatar.parts.body;
        for (pose, turn) in [
            ("nod", Quat::from_rotation_x(0.45)),
            ("turn", Quat::from_rotation_y(0.55)),
        ] {
            let posed = bent(&avatar, turn, NECK_SHARE).forward(&avatar.rig);
            let moved = posed.deform_mesh(&avatar.rig, &fur.mesh);
            // Coincident at rest, coincident posed: no crack opens.
            let mut first: HashMap<usize, Vec3> = HashMap::new();
            for (vertex, id) in ids.iter().enumerate() {
                let at = moved.positions[vertex];
                let was = *first.entry(*id).or_insert(at);
                assert!(
                    was.distance(at) < height * 1.0e-5,
                    "{name} {pose}: the solid opens at {at}"
                );
            }
            let skin_moved = posed.deform(&avatar.rig, &body.positions, &avatar.parts.weights);
            let skin = stretch(
                body.faces.iter().filter(|face| {
                    face.iter().all(|&corner| {
                        matches!(avatar.parts.zones[corner as usize], Zone::Neck | Zone::Head)
                    })
                }),
                &body.positions,
                &skin_moved,
                height * SHORTEST,
            );
            let mane = stretch(
                fur.mesh.faces[..read.faces].iter(),
                &fur.mesh.positions,
                &moved.positions,
                height * SHORTEST,
            );
            // Standing, a mane fans open over a bending neck as a stiff brush does:
            // its tips are further from the bend than the skin.
            if lies {
                worst = worst.max(mane / skin);
                assert!(
                    mane < skin * STRETCH,
                    "{name} {pose}: the mane stretched {mane}, the skin {skin}"
                );
            }
        }
    }
    println!("the most a nod or a turn stretches a mane against its skin: {worst:.3}");
}

/// How much of a nod or a turn the neck's joints take, each: the rest is the poll's.
const NECK_SHARE: f32 = 0.25;

/// The shortest edge a stretch is read on, as a share of the back's height: a
/// shorter one's stretch is its ends' rounding.
const SHORTEST: f32 = 0.005;

/// How much further than the skin's own edges a mane's may stretch in a nod or a
/// turn. Read off the sweep: PENDING.
const STRETCH: f32 = 3.5;

#[test]
fn a_mane_keeps_clear_of_the_ears_and_what_the_head_wears() {
    // The forelock parts round a unicorn's horn and sits between antlers' bases:
    // no vertex of the solid and no middle of its faces lies inside the capsule
    // kept round an ear or a worn piece, and every vertex of a worn piece lies in
    // its capsules, so the capsules are the pieces'.
    type Wear = fn(&mut Ornaments);
    let wears: [(&str, Wear); 3] = [
        ("unicorn", |worn| worn.horn = Some(Ornament::UNICORN)),
        ("antlers", |worn| worn.wear(OrnamentKind::Antlers, true)),
        ("horns", |worn| worn.wear(OrnamentKind::PairedHorns, true)),
    ];
    for seed in [6i64, 28, 73] {
        for stiffness in [0.0, Fur::DEFAULT_MANE_STIFFNESS, 1.0] {
            for (what, wear) in wears {
                let (record, avatar) =
                    built(seed, 0.9, stiffness, |params| wear(&mut params.ornaments));
                let name = format!("seed {seed} stiffness {stiffness} wearing {what}");
                let fur = fur(&avatar);
                let read = Read::of(fur, NEAR);
                let beast = avatar.parts.beast.as_ref().expect("a head");
                let skeleton = record.skeleton();
                let anatomy = CoatBody::of(&avatar.rig, skeleton.beast_head.as_ref(), None);
                let head = anatomy.head.as_ref().expect("a declared head");
                let coat = Coat::default();
                let bed = super::super::Bed {
                    body: &avatar.parts.body,
                    zones: &avatar.parts.zones,
                    rig: &avatar.rig,
                    weights: &avatar.parts.weights,
                    anatomy: &anatomy,
                    coat: &coat,
                    ears: Some(&beast.ears),
                    worn: 0,
                };
                let capsules = capsules(&bed, head);
                assert!(capsules.len() > 2, "{name}: nothing worn");
                let worn = avatar.parts.adornments.as_ref().expect("worn");
                let joint = avatar.parts.eyes.as_ref().expect("eyes").head;
                let at = avatar.rig.joints[joint].position;
                let piece = worn.on_head.transformed(glam::Mat4::from_translation(at));
                let inside = Solid::of(&piece);
                let held = read
                    .outer
                    .iter()
                    .chain(&read.inner)
                    .filter(|point| inside.holds(**point))
                    .count();
                assert_eq!(
                    held, 0,
                    "{name}: {held} of the mane's vertices inside the piece"
                );
                let clear = CLEAR * anatomy.height * 0.5;
                let mut points: Vec<Vec3> = read.outer.clone();
                points.extend(&read.inner);
                let into = points
                    .iter()
                    .flat_map(|point| capsules.iter().map(|capsule| capsule.inside(*point, clear)))
                    .fold(f32::MIN, f32::max);
                assert!(into <= 0.0, "{name}: {into} into a capsule");
            }
        }
    }
}

#[test]
fn a_mane_costs_its_grid_at_every_length_and_stiffness() {
    // A fixed cost, about the cards' it replaced (2,004 to 2,488 triangles of mane
    // and forelock at the default stiffness), whatever the record says; and the far
    // tier's on its coarser grid, which with every other zone's cards unchanged is
    // never dearer than the near tier.
    assert!(NEAR.triangles() <= 2_100, "{}", NEAR.triangles());
    assert!(
        FAR.triangles() * 3 <= NEAR.triangles(),
        "{}",
        FAR.triangles()
    );
    for (name, avatar) in maned() {
        let fur = fur(&avatar);
        let mane = fur.grown[0];
        assert_eq!(mane.clumps, LOCKS, "{name}");
        assert!(
            (NEAR.triangles()..=NEAR.triangles() + FRINGE_MOST).contains(&mane.tris),
            "{name}: {} triangles",
            mane.tris
        );
        assert_eq!(
            fur.mesh.faces[..Read::of(fur, NEAR).faces].len() * 2,
            NEAR.triangles(),
            "{name}"
        );
    }
    for seed in [6i64, 73] {
        let mut record =
            AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
        record.reroll(seed);
        let config = AvatarConfig {
            far_hair: true,
            ..AvatarConfig::default()
        };
        let avatar = Avatar::build_with(&record, &config).expect("the beast builds");
        let near = avatar
            .meshes
            .iter()
            .find(|drawn| drawn.kind == crate::MeshKind::Hair)
            .expect("fur");
        let far = avatar.far_hair.as_ref().expect("a far tier");
        // The near tier's cards, and the mane's solid on the far grid with no
        // fringe in place of the near one.
        let near_fur = fur(&avatar);
        assert_eq!(
            far.mesh.triangulated().len(),
            near_fur.tris() - near_fur.grown[0].tris + FAR.triangles(),
            "seed {seed}: the far tier's cards are not the near's"
        );
        assert!(far.mesh.triangulated().len() < near.mesh.triangulated().len());
        assert!(far.mesh.vertex_count() < near.mesh.vertex_count());
        // A beast with no mane has none: its cards are what it draws at any distance.
        let mut bare = record.clone();
        if let Archetype::Quadruped(params) = &mut bare.archetype {
            params.fur.mane = 0.0;
        }
        let maneless = Avatar::build_with(&bare, &config).expect("the beast builds");
        assert!(
            maneless.far_hair.is_none(),
            "seed {seed}: a far tier with no mane"
        );
        // Built without one, there is none, and the near tier is the same bytes.
        let plain = Avatar::build(&record).expect("the beast builds");
        assert!(plain.far_hair.is_none());
        assert_eq!(plain.parts.fur, avatar.parts.fur, "seed {seed}");
    }
}

/// What the fringe a standing mane grows may add. Read off the sweep: 252 at most.
const FRINGE_MOST: usize = 320;

/// The colours of the solid's outer and inner surfaces.
fn colours(avatar: &Avatar) -> (Vec<Vec3>, Vec<Vec3>) {
    let fur = fur(avatar);
    let run = (NEAR.per_lock + 1) * NEAR.rows;
    let (mut outer, mut inner) = (Vec::new(), Vec::new());
    for lock in 0..LOCKS {
        outer.extend_from_slice(&fur.mesh.colours[lock * 2 * run..lock * 2 * run + run]);
        inner.extend_from_slice(&fur.mesh.colours[lock * 2 * run + run..(lock + 1) * 2 * run]);
    }
    (outer, inner)
}

/// Whether `colour` is on the way from `roots` a shade toward white, darkened by
/// as much as `under` at most.
fn on_the_way(colour: Vec3, roots: Vec3, under: f32) -> bool {
    let tips = roots.lerp(Vec3::ONE, TIPPED);
    (0..=100).any(|step| {
        let along = roots.lerp(tips, step as f32 / 100.0);
        (0..=20).any(|dark| {
            let shade = 1.0 + (under - 1.0) * dark as f32 / 20.0;
            colour.distance(along * shade) < 2.0e-3
        })
    })
}

#[test]
fn a_picked_mane_is_its_colour_and_a_rolled_one_is_its_coat_s_long_hair_whole() {
    // The owner's two rules (#407). A colour a creator picked stays exact: every root
    // of the outer surface is it. A mane that follows its coat is that coat's own
    // long hair whole - a bay's black, a dun's dark, or the coat's own colour a shade
    // darker so it keeps an outline - with no patch of a patched coat's white or
    // marks. Every vertex of either is on the way from its root to its tip, and the
    // underside no darker than its shade.
    let patched = |params: &mut QuadrupedParams| {
        params.coat.white = White::Piebald;
    };
    let picked = [0.31, 0.62, 0.18];
    for seed in [6i64, 11, 28, 73, 69] {
        // Picked: exact.
        let avatar = beast(seed, 0.8, Fur::DEFAULT_MANE_STIFFNESS, |params| {
            patched(params);
            params.coat.colours.mane = Some(picked);
        });
        let chosen = Vec3::from_array(picked);
        let (outer, inner) = colours(&avatar);
        assert!(outer.contains(&chosen), "seed {seed}: never exact");
        for colour in &outer {
            assert!(on_the_way(*colour, chosen, 1.0), "seed {seed}: {colour}");
        }
        for colour in &inner {
            assert!(
                on_the_way(*colour, chosen, UNDER),
                "seed {seed}: under {colour}"
            );
        }
        // Rolled, on a coat patched white: one colour, the coat's long hair.
        for (what, coat) in [
            (
                "bay",
                Coat {
                    pigment: Pigment::Black,
                    agouti: Agouti::Bay,
                    ..Coat::default()
                },
            ),
            ("chestnut", Coat::default()),
            (
                "red bay",
                Coat {
                    pigment: Pigment::Red,
                    agouti: Agouti::Bay,
                    ..Coat::default()
                },
            ),
            (
                "liver bay",
                Coat {
                    pigment: Pigment::Liver,
                    agouti: Agouti::Bay,
                    ..Coat::default()
                },
            ),
            (
                "dilute bay",
                Coat {
                    pigment: Pigment::Black,
                    agouti: Agouti::Bay,
                    dilute: true,
                    ..Coat::default()
                },
            ),
            (
                "dun",
                Coat {
                    pigment: Pigment::Black,
                    dun: true,
                    ..Coat::default()
                },
            ),
        ] {
            let avatar = beast(seed, 0.8, Fur::DEFAULT_MANE_STIFFNESS, |params| {
                params.coat = coat;
                patched(params);
            });
            let long_hair = Vec3::from_array(crate::texture::coat::coat_colour(
                &avatar_coat(seed, coat, patched),
                crate::plan::CoatChannel::Mane,
            ));
            // What a creator's picker opens on is what is drawn.
            let roots = long_hair;
            let (outer, inner) = colours(&avatar);
            for colour in &outer {
                assert!(
                    on_the_way(*colour, roots, 1.0),
                    "seed {seed} {what}: {colour}"
                );
            }
            for colour in &inner {
                assert!(
                    on_the_way(*colour, roots, UNDER),
                    "seed {seed} {what}: {colour}"
                );
            }
            let ground = Vec3::from_array(crate::texture::coat::coat_colour(
                &avatar_coat(seed, coat, patched),
                crate::plan::CoatChannel::Light,
            ));
            if what == "chestnut" {
                // Darker than the coat it follows, by the shade.
                assert!(
                    roots.distance(ground * MANE_SHADE) < 1.0e-5,
                    "seed {seed}: {roots} against a coat of {ground}"
                );
            }
            // And never lighter than that, a coat's dark included: a red bay's is
            // a pale brown, which lying on its neck all but vanished (horse 11).
            let light = |colour: Vec3| colour.dot(Vec3::new(0.2126, 0.7152, 0.0722));
            assert!(
                light(roots) <= light(ground) * MANE_SHADE + 1.0e-5,
                "seed {seed} {what}: {roots} against a coat of {ground}"
            );
            // A coat that draws its long hair in its dark draws its mane in it,
            // exactly, as its tail and its legs are: measured against the ground,
            // which on any pigment but red is the dark itself, a black, a liver or a
            // dilute bay's mane, or a dun's, was its dark shaded down to 0.6 of it.
            // Only a dark too pale against the body it lies on is shaded, a red bay's,
            // and down to the shade and no further.
            let dark = Vec3::from_array(crate::texture::coat::coat_colour(
                &avatar_coat(seed, coat, patched),
                crate::plan::CoatChannel::Dark,
            ));
            match what {
                "bay" | "liver bay" | "dilute bay" | "dun" => {
                    assert_eq!(roots, dark, "seed {seed} {what}: its dark");
                }
                "red bay" => {
                    let most = light(ground) * MANE_SHADE;
                    assert!(light(dark) > most, "the premise: a red bay's dark is pale");
                    assert!(
                        (light(roots) - most).abs() < 1.0e-5
                            && roots.distance(dark * (light(roots) / light(dark))) < 1.0e-5,
                        "seed {seed} {what}: {roots}, its dark {dark} shaded to {most}"
                    );
                }
                _ => {}
            }
        }
    }
}

/// The coat `beast` built for `seed` wears with `coat` and `wear`.
fn avatar_coat(seed: i64, coat: Coat, wear: impl Fn(&mut QuadrupedParams)) -> Coat {
    let mut record = AvatarRecord::new("Beast", Archetype::Quadruped(QuadrupedParams::default()));
    record.reroll(seed);
    let Archetype::Quadruped(params) = &mut record.archetype else {
        unreachable!("a beast")
    };
    params.coat = coat;
    wear(params);
    params.coat
}

/// How far a mane's outer surface stands off its root, along the way its root faces,
/// for each column: what a side-on picture reads as its height.
fn standing(read: &Read) -> Vec<f32> {
    (0..read.columns())
        .map(|column| {
            let root = read.outer[read.at(column, 0)];
            (0..read.grid.rows)
                .map(|row| read.outer[read.at(column, row)].y - root.y)
                .fold(0.0, f32::max)
        })
        .collect()
}

#[test]
fn a_standing_mane_falls_away_over_the_poll_with_its_neck() {
    let mut worst = 0.0f32;
    // Horse 11 at full stiffness stood highest at the poll, a crest rising off a
    // neck that rises to it. Standing, a mane's crest is its tallest along the neck's
    // middle, and over the poll and the forelock it is lower.
    for (name, avatar) in maned()
        .into_iter()
        .filter(|(name, _)| name.ends_with("stiffness 1") && !name.contains("mane 0.3"))
    {
        let read = Read::of(fur(&avatar), NEAR);
        let heights = standing(&read);
        let per = read.grid.per_lock;
        let neck: Vec<f32> = heights[per * 2..per * 7].to_vec();
        // The poll: the last of the neck's locks and the first of the head's.
        let poll = heights[per * (NECK_LOCKS - 2)..per * (NECK_LOCKS + 2)].to_vec();
        let middle = neck.iter().copied().fold(0.0, f32::max);
        let over = poll.iter().copied().fold(0.0, f32::max);
        worst = worst.max(over / middle);
        assert!(
            over < middle * POLL_BOUND,
            "{name}: {over} over the poll, {middle} on the neck"
        );
    }
    println!("the highest a standing mane stands over the poll against its neck: {worst:.3}");
}

#[test]
fn a_standing_mane_is_one_brush() {
    // Cross 73 at full stiffness stood as a row of separate spikes: each lock's
    // point tall, the notch between two locks cut most of the way down. A stiff mane
    // is one brush, roached: where two locks meet down the neck, the column they
    // share stands nearly as high as the lower of the middles either side of it,
    // which a taper toward the withers or the poll leaves it above.
    let mut worst = (1.0f32, String::new());
    for (name, avatar) in maned()
        .into_iter()
        .filter(|(name, _)| name.ends_with("stiffness 1") && !name.contains("mane 0.3"))
    {
        let read = Read::of(fur(&avatar), NEAR);
        let heights = standing(&read);
        let per = read.grid.per_lock;
        for lock in 1..NECK_LOCKS {
            let edge = lock * per;
            let sides = heights[edge - per / 2].min(heights[edge + per / 2]);
            if heights[edge] / sides < worst.0 {
                worst = (heights[edge] / sides, format!("{name}, after lock {lock}"));
            }
        }
    }
    println!(
        "the deepest notch in a standing mane, against the locks either side: {:.3}, {}",
        worst.0, worst.1
    );
    assert!(worst.0 > BRUSH_BOUND, "{}: {}", worst.1, worst.0);
}

/// How high the column two standing locks share stands, at the least, against the
/// lower of their middles. Read off the sweep: 0.985 at the least; with the
/// points and notches a lying mane keeps, 0.43.
const BRUSH_BOUND: f32 = 0.75;

#[test]
fn a_mane_is_shaded_as_one_strip_streaked_along_its_hair() {
    // The port's locks were each shaded round, their normals fanned from one edge to
    // the other, and read from above as a braid of bundles. A column two locks share
    // is shaded alike in both, so the strip has no crease at a lock's edge; and its
    // highlight breaks into streaks along the hair, the normals of two columns side
    // by side turned apart.
    let mut apart = Vec::new();
    for (name, avatar) in maned() {
        let fur = fur(&avatar);
        let read = Read::of(fur, NEAR);
        let (per, rows) = (read.grid.per_lock, read.grid.rows);
        let run = (per + 1) * rows;
        assert_eq!(fur.mesh.normals.len(), fur.mesh.positions.len(), "{name}");
        let normal = |lock: usize, surface: usize, place: usize, row: usize| {
            fur.mesh.normals[lock * 2 * run + surface * run + place * rows + row]
        };
        for lock in 0..LOCKS - 1 {
            for surface in 0..2 {
                for row in 0..rows {
                    let (one, two) = (
                        normal(lock, surface, per, row),
                        normal(lock + 1, surface, 0, row),
                    );
                    assert!(
                        one.distance(two) < 1.0e-5,
                        "{name}: a crease between locks {lock} and {}: {one} against {two}",
                        lock + 1
                    );
                }
            }
        }
        for lock in 1..NECK_LOCKS - 1 {
            for place in 0..per {
                let row = rows / 2;
                apart.push(normal(lock, 0, place, row).angle_between(normal(
                    lock,
                    0,
                    place + 1,
                    row,
                )));
            }
        }
    }
    apart.sort_by(f32::total_cmp);
    let middle = apart[apart.len() / 2];
    println!("two columns' normals side by side, the middle of the sweep: {middle:.3}");
    assert!(middle > STREAKS, "no streaks: {middle}");
}

/// How far apart, at the least, the normals of two columns side by side are in the
/// middle of the sweep, in radians: a smooth sheet's are a few hundredths apart.
/// Read off the sweep: 0.62.
const STREAKS: f32 = 0.4;

/// How high a standing mane may stand over the poll against the neck's middle, for
/// its own height off the skin. Read off the sweep: 0.31 at most; at the
/// prototype's taper, which stood highest at the poll on horse 11, 0.64.
const POLL_BOUND: f32 = 0.45;

/// Signed distance to the body: positive outside.
fn signed(solid: &Solid, skin: &[[Vec3; 3]], at: Vec3) -> f32 {
    let off = distance(skin, at);
    if solid.holds(at) { -off } else { off }
}

/// Prints what the guards read, beast by beast: `cargo test --release mane_table --
/// --ignored --nocapture`.
#[test]
#[ignore = "a table to read, not a check"]
fn mane_table() {
    let beasts = maned();
    let lines: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = beasts
            .chunks(beasts.len().div_ceil(8))
            .map(|some| {
                scope.spawn(move || {
                    some.iter()
                        .map(|(name, avatar)| line(name, avatar))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a line"))
            .collect()
    });
    for line in lines {
        println!("{line}");
    }
}

fn line(name: &str, avatar: &Avatar) -> String {
    let fur = fur(avatar);
    let read = Read::of(fur, NEAR);
    let solid = Solid::of(&avatar.parts.body);
    let skin = triangles(&avatar.parts.body);
    let height = height(avatar);
    let rows = read.grid.rows;
    // Inner on the skin, limp part.
    let mut inner: Vec<f32> = Vec::new();
    for column in 1..read.columns() - 1 {
        for row in 2..rows * 7 / 10 {
            inner.push(signed(&solid, &skin, read.inner[read.at(column, row)]) / height);
        }
    }
    inner.sort_by(f32::total_cmp);
    // Outer faces' middles under the skin, and how deep, neck and head apart.
    let (mut through, mut deepest, mut where_) = ([0usize; 2], 0.0f32, (0, 0));
    for column in 0..read.columns() - 1 {
        for row in OUT_FROM..rows - 2 {
            let middle = (read.outer[read.at(column, row)]
                + read.outer[read.at(column + 1, row)]
                + read.outer[read.at(column, row + 1)]
                + read.outer[read.at(column + 1, row + 1)])
                * 0.25;
            if solid.holds(middle) {
                through[usize::from(!read.on_neck(column))] += 1;
                let depth = distance(&skin, middle) / height;
                if depth > deepest {
                    deepest = depth;
                    where_ = (column, row);
                }
            }
        }
    }
    let roots = (0..read.columns())
        .filter(|&column| !solid.holds(read.outer[read.at(column, 0)]))
        .count();
    // Stretch under a nod.
    let posed = bent(avatar, Quat::from_rotation_x(0.45), NECK_SHARE).forward(&avatar.rig);
    let moved = posed.deform_mesh(&avatar.rig, &fur.mesh);
    let (mut most, mut at) = (0.0f32, 0usize);
    for face in &fur.mesh.faces[..read.faces] {
        for (corner, &one) in face.iter().enumerate() {
            let two = face[(corner + 1) % face.len()] as usize;
            let one = one as usize;
            let rest = fur.mesh.positions[one].distance(fur.mesh.positions[two]);
            let opened = (moved.positions[one].distance(moved.positions[two]) - rest) / height;
            if opened > most {
                most = opened;
                at = one;
            }
        }
    }
    let run = (NEAR.per_lock + 1) * NEAR.rows;
    let lock = at / (2 * run);
    let in_lock = at % run;
    format!(
        "{name:44} inner p10 {:+.4} p50 {:+.4} p90 {:+.4} | through neck {} head {} deepest {:.4} at {:?} | roots out {roots} | nod {most:.4} lock {lock} col {} row {} | tris {}",
        inner[inner.len() / 10],
        inner[inner.len() / 2],
        inner[inner.len() * 9 / 10],
        through[0],
        through[1],
        deepest,
        where_,
        in_lock / NEAR.rows,
        in_lock % NEAR.rows,
        fur.grown[0].tris,
    )
}
