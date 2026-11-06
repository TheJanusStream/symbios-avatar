//! A bird's flight feathers and its tail fan (#421).
//!
//! **What grows where** (research-10 5.1): ten primaries off the hand,
//! numbered from the wrist out - six on the carpometacarpus, four on the
//! digit; ten secondaries off the ulna, numbered from the wrist in; three
//! tertials, the innermost, by the elbow; three humerals on the upper arm,
//! covering the gap to the body a spread wing opens there; the alula's three
//! small feathers on digit I at the wrist; and over them the coverts - the
//! greater, median and lesser rows over the secondaries and a row of primary
//! coverts over the primaries. Twelve rectrices on the pygostyle, with a row
//! of upper tail coverts over their bases.
//!
//! **How they overlap**: seen from above each flight feather lies over the
//! next one toward the wingtip, the innermost tertial on top and the
//! outermost primary underneath (S45; 5.1), and every covert row over the
//! feathers it covers - so each card stands a little further up its wing's
//! normal than the one it overlies, folded and spread alike.
//!
//! **How they move** (research-11 7, the feather-spread rig): every flight
//! feather rides a joint at its root, on the bone it grows from, turned by
//! its own share of the fan - the primaries most, from lying along the hand
//! when folded to a fan from the hand's line back to square behind the
//! wrist when spread - so folding stacks them along the flank and spreading
//! opens them evenly. A covert is held between the joints of the two
//! feathers it lies over, graded, so it opens with them. The tail fans on
//! two joints at its edges, every rectrix held between its edge and the
//! tail's last joint by how far out it lies, so the fan opens evenly about
//! the pygostyle.
//!
//! **Cards, in the hair's draw**: each a strip of quads lofted along its
//! rachis, shaped as a vane - narrow at the quill, broadest past the middle,
//! rounded at the tip - and curved a little toward the body when folded,
//! which spread is a wing's camber. Cut from the strand mask's solid run, so
//! a card is drawn whole where a lock of hair would fray; coloured per
//! vertex from the bird's plumage (#423) by the one sampler its skin is
//! painted with (`texture::coat::PlumageSampler::card`), each card by the
//! kind of feather it is and where it roots.

use glam::{Vec2, Vec3};

use super::{Arm, Book, Hold, Trunk, frame, swing, turn};
use crate::det;
use crate::hair::mask::{StrandMask, lane_of};
use crate::mesh::PolyMesh;
use crate::plan::{WingedParams, Zone};
use crate::rig::Rig;
use crate::texture::coat::{Feather, FeatherTone, PlumageSampler};

/// A spread bird wing (research-10 4.1: the elbow at 150 to 160 degrees, the
/// wrist at 160 to 170): the humerus swept back, the forearm forward, the
/// hand and the digit out, and the plane raised out of the trunk's, all in
/// radians from straight out. Chosen by render.
const HUMERUS: f32 = -0.26;
/// See [`HUMERUS`].
const FOREARM: f32 = 0.21;
/// See [`HUMERUS`].
const HAND: f32 = 0.0;
/// See [`HUMERUS`].
const DIGIT: f32 = -0.09;
/// See [`HUMERUS`].
const DIHEDRAL: f32 = 0.10;

/// Over what share of the extension the arm and the feathers move: the arm
/// across all of it, elbow and wrist on one schedule as the linkage couples
/// them; the feathers' fan a little behind.
const ARM_SCHEDULE: (f32, f32) = (0.0, 1.0);
/// See [`ARM_SCHEDULE`].
const FAN_SCHEDULE: (f32, f32) = (0.1, 1.0);

/// The primaries, P1 at the wrist to P10 at the tip: each one's length in
/// humeri, and its direction spread against straight out, in radians. An
/// owl's broad, rounded wing (research-10 3.1, bg), the longest about 2.2
/// humeri; spread from P10 a little behind the hand's line to P1 square
/// behind the wrist. Chosen by render against a crow's chord (S3) and an
/// owl's span.
const PRIMARIES: [(f32, f32); 10] = [
    (1.70, -1.62),
    (1.80, -1.50),
    (1.90, -1.38),
    (2.00, -1.25),
    (2.08, -1.11),
    (2.16, -0.97),
    (2.22, -0.83),
    (2.22, -0.68),
    (2.12, -0.53),
    (1.90, -0.38),
];

/// The secondaries, S1 at the wrist to S10 by the elbow, then the three
/// tertials: lengths in humeri and directions spread, as [`PRIMARIES`]. Each
/// square behind the ulna, turning in toward the body as they near it.
const SECONDARIES: [(f32, f32); 13] = [
    (1.75, -1.62),
    (1.74, -1.64),
    (1.72, -1.66),
    (1.71, -1.68),
    (1.70, -1.70),
    (1.68, -1.72),
    (1.67, -1.74),
    (1.65, -1.76),
    (1.64, -1.78),
    (1.62, -1.80),
    (1.55, -1.90),
    (1.42, -1.98),
    (1.28, -2.06),
];

/// The humerals on the upper arm, H1 by the elbow in: lengths in humeri and
/// directions spread, back and in toward the body.
const HUMERALS: [(f32, f32); 3] = [(1.25, -2.12), (1.05, -2.20), (0.85, -2.28)];

/// The alula's three feathers, on digit I at the wrist: lengths in humeri
/// and directions spread, along the leading edge (bg: 2 to 4 of them).
const ALULA: [(f32, f32); 3] = [(0.55, 0.12), (0.45, 0.05), (0.36, -0.02)];

/// How wide each kind of feather is, as a share of its length: a primary's
/// vane narrow, a secondary's broader, a covert's broadest.
const PRIMARY_WIDTH: f32 = 0.17;
/// See [`PRIMARY_WIDTH`].
const SECONDARY_WIDTH: f32 = 0.24;
/// See [`PRIMARY_WIDTH`].
const INNER_WIDTH: f32 = 0.30;
/// See [`PRIMARY_WIDTH`].
const COVERT_WIDTH: f32 = 0.40;

/// How long each covert row is against the feathers it covers: the greater,
/// the median, the lesser and the marginal over the secondaries, the greater
/// and lesser primary coverts, and a row over the tertials (bg, chosen by
/// render).
const COVERT_ROWS: [f32; 4] = [0.44, 0.30, 0.19, 0.11];
/// See [`COVERT_ROWS`].
const PRIMARY_COVERTS: [f32; 2] = [0.36, 0.20];
/// See [`COVERT_ROWS`].
const INNER_COVERTS: [f32; 1] = [0.40];

/// How far the humerals rise out of the spread wing's plane, as a share of
/// it: the plane through the shoulder runs below the back, and in it they
/// passed through the body; the tertials rise half as far (#421, the
/// critic's guard; chosen by render).
const INNER_RISE: f32 = 0.6;

/// How far the folded flight feathers turn in over the back from straight
/// back along the body, in radians: the primaries the most, so they cross
/// over the rump toward the tail (#421; chosen by render).
const PRIMARY_TUCK: f32 = 0.78;
/// See [`PRIMARY_TUCK`].
const SECONDARY_TUCK: f32 = 0.30;

/// How far each card stands above the one it overlies, along its wing's
/// normal, as a share of the trunk: about half a millimetre on an avatar's
/// owl, so a fold's thirty layers are a centimetre and a half of wing.
const LIFT: f32 = 0.002;

/// How far a card curves toward the body over its length, as a share of
/// it: a folded feather lying round the flank, a spread one's camber.
const CAMBER: f32 = 0.06;

/// A card's vane across its length, root to tip: where each row stands as a
/// share of the length, and the vane's width there as a share of its
/// broadest. Narrow at the quill, broadest past the middle, rounded at the
/// tip.
const VANE: [(f32, f32); 6] = [
    (0.0, 0.30),
    (0.2, 0.85),
    (0.5, 1.0),
    (0.8, 0.90),
    (0.93, 0.62),
    (1.0, 0.14),
];

/// The share of the strand mask's length a card spans, from its root: the
/// mask's lock is solid over its first three fifths (`hair::mask`), so a
/// card cut from inside it is drawn whole.
const MASK_SPAN: (f32, f32) = (0.02, 0.56);

/// The tail: twelve rectrices (research-10 5.1: 12 in most birds), the
/// central pair's length in trunks at the plan's own tail and how far
/// `tailLength` moves it, closed a few degrees apart and fanned to this
/// half-angle, in radians (bg, chosen by render). An owl's short tail; a
/// long one toward a parrot's.
const RECTRICES: usize = 12;
/// See [`RECTRICES`].
const TAIL_LENGTH: (f32, f32) = (0.36, 0.22);
/// See [`RECTRICES`].
const TAIL_CLOSED: f32 = 0.20;
/// See [`RECTRICES`].
const TAIL_FANNED: f32 = 0.85;
/// How wide a rectrix is, as a share of its length.
const RECTRIX_WIDTH: f32 = 0.34;
/// How far a closed tail tilts back from the trunk's line toward level, as
/// a share of the trunk's up added to its back: behind the rump.
const TAIL_TILT: f32 = 0.8;

/// How far the outermost rectrices turn down about their own shafts, in
/// radians: the roof of a closed tail (bg, chosen by render).
const TAIL_ROOF: f32 = 0.65;

/// Where a ray from the pelvis out along `toward` leaves the body, a little
/// inside it: the rump a tail fan roots in.
fn rump(body: &PolyMesh, from: Vec3, toward: Vec3, trunk: f32) -> Vec3 {
    if body.positions.is_empty() || !body.contains(from) {
        return from;
    }
    let (mut inside, mut outside) = (0.0f32, trunk);
    for _ in 0..14 {
        let middle = 0.5 * (inside + outside);
        if body.contains(from + toward * middle) {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    from + toward * (inside - 0.015 * trunk).max(0.0)
}

/// One feather card, at the fold.
#[derive(Clone, Debug)]
struct Card {
    /// What holds it.
    hold: Hold,
    /// Where its quill roots, at rest.
    root: Vec3,
    /// Which way it runs, at rest.
    dir: Vec3,
    /// Its wing's normal at rest: its upper side.
    normal: Vec3,
    length: f32,
    width: f32,
    /// How far up `normal` it stands, to overlie the cards it covers.
    lift: f32,
    /// Its colour along its length, from the plumage (#423).
    tone: FeatherTone,
}

/// One bird wing, its feathers' joints attached and their drives recorded.
pub(crate) struct Wing {
    cards: Vec<Card>,
}

/// A point `t` along the bone leaving `joint` toward `next`, at rest.
fn on_bone(rig: &Rig, joint: usize, next: usize, t: f32) -> Vec3 {
    let (a, b) = (Arm::at(rig, joint), Arm::at(rig, next));
    a + (b - a) * t
}

/// A bird's wing on `arm`: its arm spread and its feathers rooted on it,
/// folded on the flank of `body` (its surface and its vertex normals), each
/// coloured from `plumage` (#423).
pub(crate) fn wing(
    rig: &mut Rig,
    book: &mut Book,
    trunk: &Trunk,
    arm: &Arm,
    body: (&PolyMesh, &[Vec3]),
    plumage: &PlumageSampler,
) -> Wing {
    let side = arm.side;
    let plane = trunk.plane(side, DIHEDRAL);
    let (s, e, w, p, t) = (
        Arm::at(rig, arm.shoulder),
        Arm::at(rig, arm.elbow),
        Arm::at(rig, arm.wrist),
        Arm::at(rig, arm.palm),
        Arm::at(rig, arm.tip),
    );
    // The folded wing's plane, as the plan laid the forearm and the hand in
    // it - on the upper flank, leaning over the back - its dorsal side out.
    let outward = Vec3::new(side, 0.0, 0.0) + trunk.up;
    let normal = {
        let n = (w - e).cross(p - w).normalize_or(Vec3::new(side, 0.0, 0.0));
        if n.dot(outward) < 0.0 { -n } else { n }
    };
    let humerus = s.distance(e);
    // **Feathers are sized by the arm they grow on** (#421): each kind by
    // the bone it roots on, against a crow's ratios (S1), so a cross whose
    // forearm a bat's lengthens grows secondaries to fill it and its fan
    // stays whole, where sizing every feather by the humerus left gaps.
    let forearm_unit = e.distance(w) / 1.22;
    let hand_unit = ((w.distance(p) + p.distance(t)) / 1.08).max(forearm_unit * 0.8);
    let world = |book: &mut Book, joint: usize, rest: Vec3, theta: f32, schedule| {
        let target = turn(frame(rest, normal), frame(plane.dir(theta), plane.dorsal));
        book.turn(joint, target, schedule, Some(arm.limb));
        target
    };
    // Elbow and wrist on one schedule: the linkage (S6).
    world(book, arm.shoulder, e - s, HUMERUS, ARM_SCHEDULE);
    world(book, arm.elbow, w - e, FOREARM, ARM_SCHEDULE);
    world(book, arm.wrist, p - w, HAND, ARM_SCHEDULE);
    world(book, arm.palm, t - p, DIGIT, ARM_SCHEDULE);

    // "Straight back" along the body, and a turn of it in the fold's plane
    // toward the back.
    let back = -trunk.along;
    let dorsal = {
        let d = normal.cross(back).normalize_or(trunk.up);
        if d.dot(trunk.up) < 0.0 { -d } else { d }
    };
    let fold = |phi: f32| {
        let (sin, cos) = det::sin_cos(phi);
        (back * cos + dorsal * sin).normalize()
    };
    let zone = Zone::Extremity(arm.limb);
    let lift = LIFT * trunk.length;
    let mut cards = Vec::new();
    let mut rank = 0.0f32;
    // A flight feather on its own joint: rooted `along` the bone from
    // `joint` to `next`, lying `rest` at the fold and `theta` spread.
    let feather = |rig: &mut Rig,
                   book: &mut Book,
                   (joint, next, along): (usize, usize, f32),
                   rest: Vec3,
                   (length, theta): (f32, f32),
                   (width, unit): (f32, f32),
                   kind: Feather,
                   (rank, rise): (f32, f32)| {
        let root = on_bone(rig, joint, next, along);
        let quill = Book::attach(rig, joint, root, zone);
        let tone = plumage.card(root, kind);
        let mut card = Card {
            hold: Hold::one(quill),
            root,
            dir: rest,
            normal,
            length: length * unit,
            width: width * length * unit,
            lift: rank * lift,
            tone,
        };
        // Laid on the body first: its joint carries it from where it lies.
        card.hug(body, lift);
        // Spread, it rises `rise` of the way out of the wing's plane: an
        // inner feather over the back, which the plane through the shoulder
        // runs under.
        let spread = (plane.dir(theta) + plane.dorsal * rise).normalize();
        let target = turn(frame(card.dir, card.normal), frame(spread, plane.dorsal));
        book.turn(quill, target, FAN_SCHEDULE, Some(arm.limb));
        card
    };

    // The primaries, P10 lowest, so their stack is laid down tip first.
    let hand = (p - w).normalize_or(back);
    let mut primaries = Vec::with_capacity(PRIMARIES.len());
    for (index, &spec) in PRIMARIES.iter().enumerate().rev() {
        let (bone, along) = if index < 6 {
            ((arm.wrist, arm.palm), 0.08 + 0.15 * index as f32)
        } else {
            ((arm.palm, arm.tip), 0.10 + 0.28 * (index - 6) as f32)
        };
        // Folded, each lies back along the body and turned in over the back,
        // so the primaries cross over the rump toward the tail rather than
        // hang down the flank, a few degrees apart, the inner ones the most.
        let rest = fold(PRIMARY_TUCK - 0.012 * (9 - index) as f32);
        let card = feather(
            rig,
            book,
            (bone.0, bone.1, along),
            rest,
            spec,
            (PRIMARY_WIDTH, hand_unit),
            Feather::Remex,
            (rank, 0.0),
        );
        rank += 1.0;
        primaries.push(card);
    }
    primaries.reverse();
    // The secondaries, S1 by the wrist inward, then the tertials by the
    // elbow, each over the last.
    let mut secondaries = Vec::with_capacity(SECONDARIES.len());
    for (index, &spec) in SECONDARIES.iter().enumerate() {
        let along = if index < 10 {
            0.94 - 0.088 * index as f32
        } else {
            0.10 - 0.04 * (index - 10) as f32
        };
        let rest = fold(SECONDARY_TUCK + 0.012 * index as f32);
        let width = if index < 10 {
            (SECONDARY_WIDTH, forearm_unit)
        } else {
            (INNER_WIDTH, forearm_unit)
        };
        let card = feather(
            rig,
            book,
            (arm.elbow, arm.wrist, along),
            rest,
            spec,
            width,
            Feather::Remex,
            (rank, if index < 10 { 0.0 } else { INNER_RISE * 0.5 }),
        );
        rank += 1.0;
        secondaries.push(card);
    }
    // The humerals, over the tertials.
    for (index, &spec) in HUMERALS.iter().enumerate() {
        let rest = fold(SECONDARY_TUCK + 0.18 + 0.04 * index as f32);
        let card = feather(
            rig,
            book,
            (arm.shoulder, arm.elbow, 0.85 - 0.2 * index as f32),
            rest,
            spec,
            (INNER_WIDTH, humerus),
            Feather::Covert,
            (rank, INNER_RISE),
        );
        rank += 1.0;
        cards.push(card);
    }
    // The alula, three cards on one joint at the wrist.
    {
        let root = on_bone(rig, arm.wrist, arm.palm, 0.04);
        let joint = Book::attach(rig, arm.wrist, root, zone);
        let rest = swing(fold(PRIMARY_TUCK), normal, side, 0.04);
        let target = turn(
            frame(rest, normal),
            frame(plane.dir(ALULA[1].1), plane.dorsal),
        );
        book.turn(joint, target, FAN_SCHEDULE, Some(arm.limb));
        for (index, &(length, theta)) in ALULA.iter().enumerate() {
            let off = theta - ALULA[1].1;
            let root = root + hand * (0.02 * humerus * index as f32);
            cards.push(Card {
                hold: Hold::one(joint),
                root,
                dir: swing(rest, normal, side, off),
                normal,
                length: length * hand_unit,
                width: 0.25 * length * hand_unit,
                lift: (rank + index as f32 * 0.3) * lift,
                tone: plumage.card(root, Feather::Covert),
            });
        }
        rank += 1.0;
    }

    // The coverts, row over row, each graded between the two feathers it
    // lies over at its place between them, and each row over the one before:
    // two over the primaries, the greater, median, lesser and marginal over
    // the secondaries, and one over the tertials and humerals - two to a
    // gap, so they read as layered feathers rather than a plate.
    let between = |a: &Card, b: &Card, at: f32, share: f32, row_lift: f32| Card {
        hold: Hold::mix(&[(&a.hold, 1.0 - at), (&b.hold, at)]),
        root: a.root.lerp(b.root, at),
        dir: a.dir.lerp(b.dir, at).normalize_or(a.dir),
        normal,
        length: (a.length + (b.length - a.length) * at) * share,
        width: (a.length + (b.length - a.length) * at) * share * COVERT_WIDTH,
        lift: row_lift,
        tone: plumage.card(a.root.lerp(b.root, at), Feather::Covert),
    };
    let rows = |cards: &mut Vec<Card>, under: &[Card], shares: &[f32], rank: &mut f32| {
        for &share in shares {
            for pair in under.windows(2) {
                for at in [0.25, 0.75] {
                    cards.push(between(&pair[0], &pair[1], at, share, *rank * lift));
                }
            }
            *rank += 1.0;
        }
    };
    let coverts = cards.len();
    rows(&mut cards, &primaries, &PRIMARY_COVERTS, &mut rank);
    rows(&mut cards, &secondaries[..10], &COVERT_ROWS, &mut rank);
    rows(&mut cards, &secondaries[9..], &INNER_COVERTS, &mut rank);
    // The coverts laid on the body as their feathers are.
    for card in &mut cards[coverts..] {
        card.hug(body, lift);
    }
    cards.extend(primaries);
    cards.extend(secondaries);
    Wing { cards }
}

impl Wing {
    /// Every card of this wing, at the fold, and how far along its card
    /// each vertex stands.
    pub(crate) fn mesh(&self) -> (PolyMesh, Vec<f32>) {
        lofted(&self.cards, 0)
    }
}

/// A bird's tail fan: its rectrices and coverts on two edge joints at the
/// pygostyle.
pub(crate) struct Tail {
    cards: Vec<Card>,
}

/// The tail fan on a bird's tail, if it has one, rooted in `body`'s rump and
/// coloured from `plumage` (#423).
pub(crate) fn tail(
    rig: &mut Rig,
    book: &mut Book,
    trunk: &Trunk,
    params: &WingedParams,
    body: &PolyMesh,
    plumage: &PlumageSampler,
) -> Option<Tail> {
    // **Rooted just inside the rump**, where the fan's quills are buried,
    // and held by the tail's first joint: a bird's bony tail is a stub of
    // markers that ends about five centimetres behind its body (#420), and a
    // fan hung there stood clear of it.
    let tail = rig.in_zone(Zone::Tail);
    let &pygostyle = tail.first()?;
    let pelvis = *rig.in_zone(Zone::Pelvis).first()?;
    let closed = (-trunk.along + trunk.up * TAIL_TILT).normalize();
    let root = rump(body, Arm::at(rig, pelvis), closed, trunk.length);
    let lateral = Vec3::X;
    let mut normal = lateral.cross(closed).normalize_or(trunk.up);
    if normal.dot(trunk.up) < 0.0 {
        normal = -normal;
    }
    let length =
        trunk.length * (TAIL_LENGTH.0 + TAIL_LENGTH.1 * params.tail_length).clamp(0.15, 1.4);
    let edges = [-1.0f32, 1.0].map(|sense| {
        let joint = Book::attach(rig, pygostyle, root, Zone::Tail);
        let opened = det::Rot::axis_angle(normal, sense * (TAIL_FANNED - TAIL_CLOSED));
        book.turn(joint, opened, (0.0, 1.0), None);
        joint
    });
    let lift = LIFT * trunk.length;
    let middle = (RECTRICES - 1) as f32 * 0.5;
    let mut cards = Vec::with_capacity(RECTRICES + 5);
    for index in 0..RECTRICES {
        let out = (index as f32 - middle) / middle;
        let edge = if out < 0.0 { edges[0] } else { edges[1] };
        let hold = Hold(vec![(edge, out.abs()), (pygostyle, 1.0 - out.abs())]);
        let dir = det::Rot::axis_angle(normal, out * TAIL_CLOSED) * closed;
        // **A closed tail is a roof, not a board**: each half's rectrices
        // turned down about their own shafts, the outer ones the most, so
        // from the side the fan shows as a wedge behind the rump - lying in
        // one plane through the body's width it drew as a single stick.
        let normal = det::Rot::axis_angle(dir, -out * TAIL_ROOF) * normal;
        // The central pair on top, the outer ones under and a little short:
        // a rounded tail.
        let rank = middle - (index as f32 - middle).abs();
        let reach = length * (1.0 - 0.08 * out.abs());
        let root = root + lateral * (out * 0.06 * length);
        cards.push(Card {
            hold,
            root,
            dir,
            normal,
            length: reach,
            width: reach * RECTRIX_WIDTH,
            lift: rank * lift,
            tone: plumage.card(root, Feather::Rectrix),
        });
    }
    // The upper tail coverts, over the rectrices' bases.
    for index in 0..5 {
        let out = (index as f32 - 2.0) / 2.0;
        let edge = if out < 0.0 { edges[0] } else { edges[1] };
        let root = root + lateral * (out * 0.05 * length);
        cards.push(Card {
            hold: Hold(vec![
                (edge, 0.6 * out.abs()),
                (pygostyle, 1.0 - 0.6 * out.abs()),
            ]),
            root,
            dir: det::Rot::axis_angle(normal, out * TAIL_CLOSED) * closed,
            normal,
            length: length * 0.45,
            width: length * 0.45 * COVERT_WIDTH,
            lift: (middle + 1.0) * lift,
            tone: plumage.card(root, Feather::TailCovert),
        });
    }
    Some(Tail { cards })
}

impl Tail {
    /// Every card of the fan, closed, and how far along its card each
    /// vertex stands.
    pub(crate) fn mesh(&self) -> (PolyMesh, Vec<f32>) {
        lofted(&self.cards, 1000)
    }
}

/// Cards lofted one after another, salted from `salt`, with each vertex's
/// share of its card's length.
fn lofted(cards: &[Card], salt: usize) -> (PolyMesh, Vec<f32>) {
    let mut mesh = PolyMesh::new();
    let mut reach = Vec::new();
    for (index, card) in cards.iter().enumerate() {
        card.loft(&mut mesh, salt + index);
        reach.extend(VANE.iter().flat_map(|&(t, _)| [t, t]));
    }
    (mesh, reach)
}

/// How close a folded card's tip lies to the body it folds on, as a share
/// of the trunk, before its own lift: a few millimetres of air.
const HUG_GAP: f32 = 0.008;

/// How far a folded card's tip may be carried to lie on its body, at most,
/// as a share of its length.
const HUG_REACH: f32 = 0.6;

impl Card {
    /// **Lays this folded card on `body`, straight**: its tip carried along
    /// the body's normal there until it stands its gap and its lift off the
    /// surface, the card turned about its quill to reach it, and its plane
    /// turned halfway toward the body's own at its middle. Every flight
    /// feather has a joint of its own, which takes any fold onto its spread
    /// place, so a card can lie on an egg and still spread flat; bent along
    /// its length to lie there instead, it spread as a banana.
    ///
    /// **Laid on the left, always, and reflected**: the inside test casts a
    /// ray a little off the axes and the nearest vertex ties differently on
    /// the two sides, so a card laid on each side came out unlike its mirror.
    fn hug(&mut self, body: (&PolyMesh, &[Vec3]), lift: f32) {
        if self.root.x >= 0.0 {
            self.lay(body, lift);
            return;
        }
        let flip = Vec3::new(-1.0, 1.0, 1.0);
        let root = self.root;
        self.root *= flip;
        self.dir *= flip;
        self.normal *= flip;
        self.lay(body, lift);
        self.root = root;
        self.dir *= flip;
        self.normal *= flip;
    }

    /// [`Self::hug`] on the left.
    fn lay(&mut self, (body, normals): (&PolyMesh, &[Vec3]), lift: f32) {
        if body.positions.is_empty() || normals.len() != body.positions.len() {
            return;
        }
        let nearest = |at: Vec3| {
            body.positions
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    a.1.distance_squared(at)
                        .total_cmp(&b.1.distance_squared(at))
                })
                .map_or(0, |(index, _)| index)
        };
        let wanted = lift / LIFT * HUG_GAP + self.lift;
        for _ in 0..3 {
            let tip = self.root + self.dir * self.length;
            let at = nearest(tip);
            let stands = (tip - body.positions[at]).dot(normals[at]);
            let most = HUG_REACH * self.length;
            let tip = tip + normals[at] * (wanted - stands).clamp(-most, most);
            self.dir = (tip - self.root).normalize_or(self.dir);
        }
        // And backed off the body, turned out about its quill, until none
        // of its rows lies inside: a chord laid on an egg's curve cuts into
        // it wherever the egg bulges between the quill and the tip.
        let across = self.normal.cross(self.dir).normalize_or(Vec3::X);
        for _ in 0..24 {
            // Each row as it will be lofted: its lift, and the camber that
            // curves it back toward the body (#421, the critic's guard: left
            // out, a smallest bird's cards bowed into its trunk).
            let inside = VANE.iter().skip(1).any(|&(t, wide)| {
                let centre = self.root
                    + self.dir * (self.length * t)
                    + self.normal * (self.lift - CAMBER * self.length * t * t);
                let half = across * (0.5 * self.width * wide);
                body.contains(centre + half)
                    || body.contains(centre - half)
                    || body.contains(centre)
            });
            if !inside {
                break;
            }
            self.dir = (self.dir + self.normal * 0.06).normalize_or(self.dir);
        }
        let middle = normals[nearest(self.root + self.dir * (self.length * 0.5))];
        let blended = (self.normal + middle).normalize_or(self.normal);
        let normal = blended - self.dir * blended.dot(self.dir);
        if normal.dot(self.normal) > 0.0 {
            self.normal = normal.normalize_or(self.normal);
        }
    }

    /// Lofts this card into `mesh`: [`VANE`]'s rows across its rachis, in
    /// its wing's plane, curved toward the body by [`CAMBER`].
    fn loft(&self, mesh: &mut PolyMesh, salt: usize) {
        let skin = self.hold.cut();
        let across = self.normal.cross(self.dir).normalize_or(Vec3::X);
        // The middle of a lane, where the lock is whole to its edges.
        let (lo, hi) = StrandMask::lane_span(lane_of(self.root + Vec3::splat(salt as f32 * 1e-3)));
        let (lo, hi) = (lo + (hi - lo) * 0.12, hi - (hi - lo) * 0.12);
        // A little variety from feather to feather, from its place in the
        // wing: never a roll, so a body draws the same every time.
        let shade = 0.92 + 0.16 * ((salt * 37 % 11) as f32 / 10.0);
        let mut card = PolyMesh::new();
        let mut uvs = Vec::with_capacity(VANE.len() * 2);
        let mut colours = Vec::with_capacity(VANE.len() * 2);
        for &(t, wide) in &VANE {
            let centre = self.root
                + self.dir * (self.length * t)
                + self.normal * (self.lift - CAMBER * self.length * t * t);
            let half = across * (0.5 * self.width * wide);
            let tone = (self.tone.at(t) * shade).min(Vec3::ONE);
            let v = MASK_SPAN.0 + (MASK_SPAN.1 - MASK_SPAN.0) * t;
            for (offset, u) in [(-half, lo), (half, hi)] {
                card.push_vertex(centre + offset);
                uvs.push(Vec2::new(u, v));
                colours.push(tone);
            }
        }
        // Wound to face the wing's upper side.
        for row in 0..VANE.len() as u32 - 1 {
            let a = row * 2;
            card.push_face(vec![a, a + 2, a + 3, a + 1]);
        }
        card.set_uvs(uvs);
        card.set_colours(colours);
        card.set_skin(vec![skin; VANE.len() * 2]);
        mesh.append(&card);
    }
}
