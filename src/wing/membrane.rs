//! A bat's wing and a glider's patagium (#421): spars, membranes and the
//! fold.
//!
//! **A bat's hand is five digits off the wrist** (research-10 3.2, 4.2, 5.2):
//! a short clawed thumb (I) and four long spars (II to V), Pteropus's
//! proportions against its forearm (S20: digit III's metacarpal 0.67 of it
//! and its first phalanx 0.50; the rest bg). Spread, the digits fan with III
//! leading the tip and V about a right angle behind it (4.2, bg); folded,
//! the wrist flexes so they swing back along the forearm, the metacarpals
//! beside the radius and the phalanges folded back on them (S46), each layer
//! a little further from the body so no two lie in one place. The membrane
//! is research-10 5.2's three, as one closed thin shell each:
//!
//! - **the dactylopatagium** between each pair of digits II to V, its
//!   trailing edge scalloped between their tips;
//! - **the plagiopatagium** from digit V and the arm to the flank and the
//!   hind leg, its trailing edge from V's tip to the ankle (S22, S25: the
//!   ankle in fruit bats);
//! - **the propatagium** from the shoulder to the thumb, in front of the
//!   arm.
//!
//! **A glider's patagium** runs from the outside of the forefoot to the
//! ankle along the flank (S30, S31), on a body with no digits of its own: it
//! roots in the flank and rides the fore and hind legs, its free edge
//! scalloped between the forefoot and the ankle.
//!
//! **Laid out spread, rested by its binding's inverse** (see [`super`]): a
//! membrane's shape is simple spread and its fold is what its weights make
//! of it - which is the fold research-10 describes, the membrane bunched
//! between spars laid together and carried under the forearm, against the
//! body (S46) - and a spread pose puts back exactly the surface that was
//! laid out. A membrane body's extension carries its hind legs out too,
//! behind the arm, since the trailing edge it tensions is anchored at the
//! ankle (S22).

use glam::{Quat, Vec2, Vec3};

use super::{
    Arm, Book, Hold, Kind, Plane, Spread, Trunk, along_bones, along_chain, frame, swing, turn,
};
use crate::det;
use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::{Limb, Zone};
use crate::rig::{Rig, SkinWeights};

/// One of a bat's spars, II to V: its bones' lengths as shares of the
/// forearm (metacarpal, then the two phalanges), where it points spread
/// against the forearm, how each phalanx curves on from the bone before it,
/// and where it lies folded against the forearm, all in radians toward the
/// leading edge (negative sweeps back).
struct Digit {
    bones: [f32; 3],
    spread: f32,
    curve: [f32; 2],
    folded: f32,
    /// How each phalanx folds on the bone before it, and how far its layer
    /// leans out from the body: [`ZIGZAG`].
    phalanges: ([f32; 2], [f32; 2]),
}

/// Digits II to V of a large flying fox. Digit III's metacarpal and first
/// phalanx are S20's (0.67 and 0.50 of the forearm); the rest are bg, digit
/// III's whole about 1.75 forearms and digit V's about 1.25 (research-10
/// 3.2). Spread, III leads the tip and V stands about a right angle behind
/// it (4.2, bg); folded, the four lie back along the forearm within a few
/// degrees of one another (S46), II nearest its leading side.
const DIGITS: [Digit; 4] = [
    Digit {
        bones: [0.58, 0.26, 0.14],
        spread: -0.22,
        curve: [-0.08, -0.08],
        folded: -2.98,
        phalanges: ZIGZAG,
    },
    Digit {
        bones: [0.67, 0.50, 0.58],
        spread: -0.48,
        curve: [-0.10, -0.14],
        folded: -3.01,
        phalanges: ZIGZAG,
    },
    Digit {
        bones: [0.62, 0.42, 0.38],
        spread: -1.05,
        curve: [-0.14, -0.18],
        folded: -3.04,
        phalanges: ZIGZAG,
    },
    Digit {
        bones: [0.62, 0.36, 0.30],
        spread: -1.75,
        curve: [-0.20, -0.22],
        folded: -3.07,
        phalanges: ZIGZAG,
    },
];

/// How each digit folds its phalanges on the bone before each, in radians
/// (S46: the phalanges fold back on the metacarpals), and how far each folded
/// layer leans away from the body, as a share of its length: so the second
/// phalanx lies outside the first and the first outside the metacarpal.
///
/// **A flatter Z** (#421, the owner's look): the first phalanx back along
/// the metacarpal at 149 degrees, the second forward along it at 143, each
/// layer leaning out a little, so the hand packs along the forearm. At 140
/// and 132 with twice the lean a phalanx stood out of the fold as a spike.
/// Flatter still, or folded the same way round both times, the turns of
/// neighbouring bones - at a knuckle shared across digits IV and V, or
/// between V and the blended spar beside it - came more than a half turn
/// apart partway through the extension, and dual-quaternion skinning made
/// the membrane between them jump: this is the flattest fold measured to
/// stay continuous.
const ZIGZAG: ([f32; 2], [f32; 2]) = ([2.6, -2.5], [0.05, 0.08]);

/// How far the cape a bat's plagiopatagium makes at rest - from its raised
/// arm down to its leg - is drawn in toward the flank under it, as a share
/// of how far each free point of it stands off the body (#421, the owner's
/// look; chosen by render).
const CAPE_CLING: f32 = 0.6;

/// The thumb, digit I: its metacarpal and its phalanx as shares of the
/// forearm (bg), and where it points spread, against the forearm, in
/// radians: forward, at the leading edge, clawed and free of the membrane.
const THUMB: [f32; 2] = [0.10, 0.16];
/// See [`THUMB`].
const THUMB_SPREAD: f32 = 0.85;

/// A spread bat's arm, in its wing's plane (research-10 4.2: the elbow at
/// 120 to 150 degrees, the wrist near straight): the humerus swept back, the
/// forearm forward, and the plane raised this far out of the trunk's, all in
/// radians. Chosen by render.
const BAT_HUMERUS: f32 = -0.45;
/// See [`BAT_HUMERUS`].
const BAT_FOREARM: f32 = 0.35;
/// See [`BAT_HUMERUS`].
const BAT_DIHEDRAL: f32 = 0.08;

/// A spread glider (research-10 4.3: all four limbs out to the sides): the
/// humerus, the forearm and the forefoot, then the femur and the tibia, each
/// against straight out, in radians. Chosen by render.
const GLIDER_ARM: [f32; 3] = [0.15, 0.40, 0.55];
/// See [`GLIDER_ARM`].
const GLIDER_LEG: [f32; 2] = [-0.55, -0.75];

/// A spread bat's hind leg: out and back, the trailing edge's anchor.
const BAT_LEG: [f32; 2] = [-1.05, -1.20];

/// Over what share of the extension each part of a membrane wing moves:
/// all of it, the arm, the digits, the spars and the hind leg together. A
/// membrane blends them, and two joints on one schedule drift apart no
/// further than they stand spread (`Wings::pose`); on schedules of their own
/// a lagging leg and a leading arm met a half turn apart.
const ARM_SCHEDULE: (f32, f32) = (0.0, 1.0);
/// See [`ARM_SCHEDULE`].
const DIGIT_SCHEDULE: (f32, f32) = ARM_SCHEDULE;
/// See [`ARM_SCHEDULE`].
const LEG_SCHEDULE: (f32, f32) = ARM_SCHEDULE;

/// How thick a membrane is, either side of its middle, as a share of the
/// trunk: a millimetre or so on an avatar's bat. research-10 5.2's 130 to
/// 300 micrometres is far under anything a depth buffer at a game's
/// distance separates, so the shell is stylised thicker.
const HALF_THICKNESS: f32 = 0.004;

/// How far a free trailing edge sags in toward its membrane between the
/// spars it spans, as a share of the spars' length: the scallop between a
/// bat's fingertips (bg, chosen by render).
const SCALLOP: f32 = 0.10;

/// How many spars of its own a plagiopatagium or a glider's patagium is
/// stretched on between the arm and the leg (research-11 7: "an extra spar
/// per digit"): each a two-bone chain from the flank to the trailing edge,
/// folded at rest and straight spread, so the membrane between neighbours
/// turns by a share of the whole and its fold stays against the body.
const SPARS: usize = 3;

/// How many points each edge of a membrane is sampled at, and the grid it is
/// laid out on: rows from root to trailing edge, columns across each strip
/// between two spars.
const SAMPLES: usize = 33;

/// A digit's radius at its root, as a share of the forearm, and the thumb's.
const DIGIT_RADIUS: f32 = 0.011;
/// See [`DIGIT_RADIUS`].
const THUMB_RADIUS: f32 = 0.020;

/// Sides around a digit's tube.
const TUBE_SIDES: usize = 6;

/// The corner of a membrane's chart its digits' tubes are drawn in, as a
/// share of the chart each way: every tube in the same sliver, over the root
/// rows of the first piece, which lie inside the arm.
const DIGIT_CHART: f32 = 0.05;

/// How many pieces a bat's membrane is charted in, a column each: the
/// plagiopatagium, the dactylopatagium and the propatagium, in that order.
/// A glider's patagium is one.
const BAT_PIECES: f32 = 3.0;

/// What a texel of a membrane's own chart is (#423), as its painter asks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Charted {
    /// A bat's digit or thumb: the bone under its skin.
    Digit,
    /// The membrane itself, `edge` of the way from where it roots - in the
    /// body or along a limb - to its free edge, `0..=1`.
    Sheet {
        /// How far out toward the free edge.
        edge: f32,
    },
}

/// **What lies at `local` in a membrane's chart** (#423), `0..1` each way
/// across the region the build reserved for it: read off the layout `Wing::mesh`
/// gives it, so the painter knows a texel by where it is, as a mouth knows
/// its pocket. Each piece is a column, its rows running from root to free
/// edge (`sheet`'s `v`), but a propatagium's, whose free edge is its leading
/// one, across its column; a bat's tubes are the corner `DIGIT_CHART` wide.
pub(crate) fn charted(local: Vec2, bat: bool) -> Charted {
    let local = local.clamp(Vec2::ZERO, Vec2::ONE);
    if !bat {
        return Charted::Sheet { edge: local.y };
    }
    if local.x < DIGIT_CHART && local.y < DIGIT_CHART {
        return Charted::Digit;
    }
    let column = local.x * BAT_PIECES;
    if column >= BAT_PIECES - 1.0 {
        return Charted::Sheet {
            edge: (column - (BAT_PIECES - 1.0)).min(1.0),
        };
    }
    Charted::Sheet { edge: local.y }
}

/// One membrane wing, its joints attached and its drives recorded.
pub(crate) struct Wing {
    kind: Kind,
    plane: Plane,
    side: f32,
    /// Digits II to V, each root, knuckle, middle joint and tip.
    digits: Vec<[usize; 4]>,
    /// The thumb: root, knuckle, tip.
    thumb: Option<[usize; 3]>,
    /// The hind leg on the wing's side: hip, knee, ankle.
    hind: Option<[usize; 3]>,
    /// The membrane's own spars, flank to trailing edge, root first.
    spars: Vec<Vec<usize>>,
    /// The forearm's length, which the spars are sized by.
    forearm: f32,
    /// The trunk's length, which the shell's thickness is.
    trunk: f32,
    /// The arm's rest normal: its folded spars' plane.
    normal: Vec3,
}

/// The hind leg on a fore limb's side: hip, knee and ankle, as its plan
/// hangs them.
fn hind_of(rig: &Rig, limb: Limb) -> Option<[usize; 3]> {
    let hind = if limb.is_left() {
        Limb::HindLeft
    } else {
        Limb::HindRight
    };
    let upper = rig.in_zone(Zone::UpperLimb(hind));
    let lower = rig.in_zone(Zone::LowerLimb(hind));
    let [.., hip, knee] = upper[..] else {
        return None;
    };
    Some([hip, knee, *lower.first()?])
}

/// Spreads a hind leg out along the wing's plane: each bone by the shortest
/// arc onto its spread direction, the foot riding the shin.
fn spread_leg(
    rig: &Rig,
    book: &mut Book,
    plane: &Plane,
    leg: [usize; 3],
    limb: Limb,
    at: [f32; 2],
) {
    let [hip, knee, ankle] = leg;
    let (h, k, a) = (Arm::at(rig, hip), Arm::at(rig, knee), Arm::at(rig, ankle));
    let down = -plane.dorsal * 0.12;
    let femur = (plane.dir(at[0]) + down).normalize();
    let tibia = (plane.dir(at[1]) + down).normalize();
    let thigh = det::from_rotation_arc((k - h).normalize_or(femur), femur);
    // The shin's own arc, taken from where the thigh carried it, so the knee
    // turns only by the difference.
    let carried = thigh * (a - k).normalize_or(tibia);
    let shin = det::from_rotation_arc(carried, tibia) * thigh;
    book.turn(hip, thigh, LEG_SCHEDULE, Some(limb));
    book.turn(knee, shin, LEG_SCHEDULE, Some(limb));
}

/// Turns the arm's bones from the fold onto the spread wing's plane, at
/// `angles` against straight out: the humerus, the forearm and the bone from
/// the wrist on - or, with no third angle, the wrist riding its forearm.
fn spread_arm(
    rig: &Rig,
    book: &mut Book,
    arm: &Arm,
    plane: &Plane,
    normal: Vec3,
    angles: [Option<f32>; 3],
) {
    let (s, e, w, p) = (
        Arm::at(rig, arm.shoulder),
        Arm::at(rig, arm.elbow),
        Arm::at(rig, arm.wrist),
        Arm::at(rig, arm.palm),
    );
    for (joint, rest, theta) in [
        (arm.shoulder, e - s, angles[0]),
        (arm.elbow, w - e, angles[1]),
        (arm.wrist, p - w, angles[2]),
    ] {
        let Some(theta) = theta else {
            continue;
        };
        let world = turn(frame(rest, normal), frame(plane.dir(theta), plane.dorsal));
        book.turn(joint, world, ARM_SCHEDULE, Some(arm.limb));
    }
}

/// A bat's wing on `arm`: the arm spread, five digits attached to its wrist
/// and their fold and spread recorded, and the hind leg on its side.
pub(crate) fn bat(rig: &mut Rig, book: &mut Book, trunk: &Trunk, arm: &Arm) -> Wing {
    let side = arm.side;
    let plane = trunk.plane(side, BAT_DIHEDRAL);
    let normal = arm.rest_normal(rig);
    let (e, w) = (Arm::at(rig, arm.elbow), Arm::at(rig, arm.wrist));
    let forearm = e.distance(w);
    spread_arm(
        rig,
        book,
        arm,
        &plane,
        normal,
        // **The wrist rides the forearm**: its own bone is the pad the crawl
        // stands on, below the arm's plane, and turned onto the plane it
        // carried every digit's turn out of it - some past a half turn, which
        // the shortest arc then took the other way round from their
        // neighbours'. Riding, each digit turns about the arm's own hinge.
        [Some(BAT_HUMERUS), Some(BAT_FOREARM), None],
    );

    // The digits, folded back along the forearm on its trailing side, each
    // phalanx folded back on the bone before it and leaning out from the
    // body a little more, so the layers do not meet.
    let toward_elbow = (e - w).normalize_or(Vec3::Z);
    let away = {
        let out = Vec3::new(side, 0.0, 0.0);
        let lean = normal * normal.dot(out).signum();
        lean.normalize_or(out)
    };
    let zone = Zone::Extremity(arm.limb);
    let mut digits = Vec::with_capacity(DIGITS.len());
    for digit in &DIGITS {
        let lengths = digit.bones.map(|share| share * forearm);
        // Back along the forearm: `folded` is measured from the forearm's own
        // line out of the elbow, so a turn near a half turn lies alongside it.
        let metacarpal = swing(-toward_elbow, normal, side, digit.folded);
        let (fold, lean) = digit.phalanges;
        let first = (swing(metacarpal, normal, side, fold[0]) + away * lean[0]).normalize();
        let second = (swing(first, normal, side, fold[1]) + away * lean[1]).normalize();
        let root_at = w + metacarpal * (0.02 * forearm);
        let knuckle_at = root_at + metacarpal * lengths[0];
        let middle_at = knuckle_at + first * lengths[1];
        let tip_at = middle_at + second * lengths[2];
        let root = Book::attach(rig, arm.wrist, root_at, zone);
        let knuckle = Book::attach(rig, root, knuckle_at, zone);
        let middle = Book::attach(rig, knuckle, middle_at, zone);
        let tip = Book::attach(rig, middle, tip_at, zone);
        let spread = [
            BAT_FOREARM + digit.spread,
            BAT_FOREARM + digit.spread + digit.curve[0],
            BAT_FOREARM + digit.spread + digit.curve[0] + digit.curve[1],
        ];
        for ((joint, rest), theta) in [(root, metacarpal), (knuckle, first), (middle, second)]
            .into_iter()
            .zip(spread)
        {
            let world = turn(frame(rest, normal), frame(plane.dir(theta), plane.dorsal));
            book.turn(joint, world, DIGIT_SCHEDULE, Some(arm.limb));
        }
        digits.push([root, knuckle, middle, tip]);
    }

    // The thumb, on the ground ahead of the wrist where the crawl plants it
    // (the hand's second node), and forward at the leading edge spread.
    let ahead = (Arm::at(rig, arm.tip) - w).normalize_or(-toward_elbow);
    let root_at = w + ahead * (0.02 * forearm);
    let knuckle_at = root_at + ahead * (THUMB[0] * forearm);
    let tip_at = knuckle_at + ahead * (THUMB[1] * forearm);
    let root = Book::attach(rig, arm.wrist, root_at, zone);
    let knuckle = Book::attach(rig, root, knuckle_at, zone);
    let tip = Book::attach(rig, knuckle, tip_at, zone);
    let world = turn(
        frame(ahead, normal),
        frame(plane.dir(BAT_FOREARM + THUMB_SPREAD), plane.dorsal),
    );
    book.turn(root, world, DIGIT_SCHEDULE, Some(arm.limb));

    let hind = hind_of(rig, arm.limb);
    if let Some(leg) = hind {
        spread_leg(rig, book, &plane, leg, arm.limb, BAT_LEG);
    }
    Wing {
        kind: Kind::Bat,
        plane,
        side,
        digits,
        thumb: Some([root, knuckle, tip]),
        hind,
        spars: Vec::new(),
        forearm,
        trunk: trunk.length,
        normal,
    }
}

/// A glider's patagium on `arm`: the foreleg and the hind leg on its side
/// spread out along one plane.
pub(crate) fn glider(rig: &mut Rig, book: &mut Book, trunk: &Trunk, arm: &Arm) -> Wing {
    let plane = trunk.plane(arm.side, 0.0);
    let normal = arm.rest_normal(rig);
    let forearm = Arm::at(rig, arm.elbow).distance(Arm::at(rig, arm.wrist));
    spread_arm(rig, book, arm, &plane, normal, GLIDER_ARM.map(Some));
    let hind = hind_of(rig, arm.limb);
    if let Some(leg) = hind {
        spread_leg(rig, book, &plane, leg, arm.limb, GLIDER_LEG);
    }
    Wing {
        kind: Kind::Glider,
        plane,
        side: arm.side,
        digits: Vec::new(),
        thumb: None,
        hind,
        spars: Vec::new(),
        forearm,
        trunk: trunk.length,
        normal,
    }
}

/// Where a two-bone chain from `root` to `tip` bends, toward `pole`, its
/// bones `a` and `b` long: the cosine by the law of cosines, the sine by the
/// root, no trigonometry.
fn bend(root: Vec3, tip: Vec3, a: f32, b: f32, pole: Vec3) -> Vec3 {
    let span = tip - root;
    let length = span.length().max(1e-6);
    let along = span / length;
    let reach = length.clamp((a - b).abs() + 1e-4, (a + b) * 0.999);
    let across = (pole - along * pole.dot(along)).normalize_or(along.any_orthonormal_vector());
    let cos = ((a * a + reach * reach - b * b) / (2.0 * a * reach)).clamp(-1.0, 1.0);
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    root + (along * cos + across * sin) * a
}

/// One edge of a membrane, root to trailing edge: each point where it rests
/// and the binding it carries.
type Edge = Vec<(Vec3, Hold)>;

impl Wing {
    /// Attaches the membrane's own spars: a bat's blended from its arm and
    /// its leg ([`Self::blended_spars`]), a glider's run from its flank
    /// ([`Self::flank_spars`]).
    ///
    /// **A glider's flipped once** (#421, the critic): two-bone chains from
    /// the flank, each turned by the world's shortest arc onto its spread
    /// line - a turn that owed nothing to the foreleg's beside it - came a
    /// half turn apart on a glider-led cross with long wings at about 0.47
    /// of the extension, where the membrane between them flipped by three
    /// quarters of a metre. Blended as a bat's are, they came close to
    /// flipping across the whole extension instead. What holds them now is
    /// each turn's arc in the body's frame, one schedule for the arm, the leg
    /// and the spars, and the trim `super::fit` makes.
    pub(crate) fn spars(
        &mut self,
        rig: &mut Rig,
        book: &mut Book,
        spread: &[Vec3],
        arm: &Arm,
        body: &PolyMesh,
    ) {
        match self.kind {
            Kind::Bat => self.blended_spars(rig, book, arm, body),
            _ => self.flank_spars(rig, book, spread, arm, body),
        }
    }

    /// **A glider's spars run from the flank**: each a two-bone chain rooted on
    /// the trunk just under the flank, between the shoulder and the hip,
    /// reaching the free edge - straight from the forefoot to the ankle when
    /// spread, folded at its middle at rest, bent out and down from the body,
    /// its tip where the edge rests. Blended from the foreleg and the leg as
    /// a bat's are, every bone of them turned near a half turn and the
    /// blends between them came close to flipping across the whole
    /// extension. `spread` is where each joint stands with the arm and the
    /// leg spread.
    fn flank_spars(
        &mut self,
        rig: &mut Rig,
        book: &mut Book,
        spread: &[Vec3],
        arm: &Arm,
        body: &PolyMesh,
    ) {
        let Some([hip, _, ankle]) = self.hind else {
            return;
        };
        let outer = arm.palm;
        let trunk: Vec<usize> = (0..rig.len())
            .filter(|&joint| {
                matches!(
                    rig.joints[joint].zone,
                    Zone::Pelvis | Zone::Abdomen | Zone::Chest
                ) && rig.joints[joint].role == crate::rig::Role::Deform
            })
            .collect();
        let (shoulder, hip_at) = (Arm::at(rig, arm.shoulder), Arm::at(rig, hip));
        let (outer_rest, ankle_rest) = (Arm::at(rig, outer), Arm::at(rig, ankle));
        let (outer_spread, ankle_spread) = (spread[outer], spread[ankle]);
        let pole = Vec3::new(self.side, 0.0, 0.0) - self.plane.dorsal * 0.5;
        let zone = Zone::Extremity(arm.limb);
        for index in 1..=SPARS {
            let share = index as f32 / (SPARS + 1) as f32;
            let root_at = under_flank(body, shoulder + (hip_at - shoulder) * share, self.side);
            let end_spread = outer_spread + (ankle_spread - outer_spread) * share;
            let length = root_at.distance(end_spread);
            let end_rest = outer_rest + (ankle_rest - outer_rest) * share;
            let reach = root_at.distance(end_rest).min(length * 0.98);
            let end_rest = root_at + (end_rest - root_at).normalize_or(pole) * reach;
            let middle_rest = bend(root_at, end_rest, length * 0.5, length * 0.5, pole);
            let middle_spread = (root_at + end_spread) * 0.5;
            let parent = trunk
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    Arm::at(rig, a)
                        .distance_squared(root_at)
                        .total_cmp(&Arm::at(rig, b).distance_squared(root_at))
                })
                .unwrap_or(arm.shoulder);
            let root = Book::attach(rig, parent, root_at, zone);
            let middle = Book::attach(rig, root, middle_rest, zone);
            let tip = Book::attach(rig, middle, end_rest, zone);
            let first = det::from_rotation_arc(
                (middle_rest - root_at).normalize_or(Vec3::Z),
                (middle_spread - root_at).normalize_or(Vec3::Z),
            );
            let carried = first * (end_rest - middle_rest).normalize_or(Vec3::Z);
            let second =
                det::from_rotation_arc(carried, (end_spread - middle_spread).normalize_or(Vec3::Z))
                    * first;
            book.turn(root, first, DIGIT_SCHEDULE, Some(arm.limb));
            book.turn(middle, second, DIGIT_SCHEDULE, Some(arm.limb));
            self.spars.push(vec![root, middle, tip]);
        }
    }

    /// The joints of the membrane's front edge: a bat's arm and digit V, a
    /// glider's foreleg to its forefoot.
    fn front(&self, arm: &Arm) -> Vec<usize> {
        let mut front = vec![arm.shoulder, arm.elbow, arm.wrist];
        match self.digits.last() {
            Some(fifth) => front.extend_from_slice(fifth),
            None => front.push(arm.palm),
        }
        front
    }

    /// **A membrane's spars are its arm and its leg blended** (#421): each a
    /// chain with as many bones as the front edge (a bat's arm and digit V, a
    /// glider's foreleg), rooted on the flank between
    /// the shoulder and the hip, resting where the arm and the leg rest
    /// blended by its place between them, and turning, bone by bone, by
    /// their two turns blended alike. Folded, each lies between the arm and
    /// the leg as they lie, so the membrane on them drapes from the folded
    /// arm to the flank and the foot and bunches where the arm folds back on
    /// itself; spread, each fans between them, so no strip between two of
    /// them turns by more than a share of the whole. A flank-rooted spar of
    /// its own two bones had to fold sharply to fit a crawl and stood out of
    /// the fold as a spike.
    fn blended_spars(&mut self, rig: &mut Rig, book: &mut Book, arm: &Arm, body: &PolyMesh) {
        let Some([hip, knee, ankle]) = self.hind else {
            return;
        };
        let front = self.front(arm);
        let bones = front.len() - 1;
        // The leg as as many points as the front has joints, evenly by its
        // length, and the bone each segment of it lies on.
        let leg = [hip, knee, ankle];
        let lengths = [
            Arm::at(rig, hip).distance(Arm::at(rig, knee)),
            Arm::at(rig, knee).distance(Arm::at(rig, ankle)),
        ];
        let total = (lengths[0] + lengths[1]).max(1e-6);
        let leg_at = |share: f32| -> (Vec3, usize) {
            let run = share * total;
            if run <= lengths[0] {
                let t = run / lengths[0].max(1e-6);
                (Arm::at(rig, hip).lerp(Arm::at(rig, knee), t), 0)
            } else {
                let t = ((run - lengths[0]) / lengths[1].max(1e-6)).min(1.0);
                (Arm::at(rig, knee).lerp(Arm::at(rig, ankle), t), 1)
            }
        };
        let front_rest: Vec<Vec3> = front.iter().map(|&joint| Arm::at(rig, joint)).collect();
        let front_turn: Vec<Quat> = front[..bones]
            .iter()
            .map(|&joint| book.world_of(rig, joint))
            .collect();
        let leg_points: Vec<(Vec3, usize)> = (0..=bones)
            .map(|index| leg_at(index as f32 / bones as f32))
            .collect();
        // Each segment turns as the bone under its middle does.
        let leg_turn: Vec<Quat> = (0..bones)
            .map(|index| {
                let (_, bone) = leg_at((index as f32 + 0.5) / bones as f32);
                book.world_of(rig, leg[bone])
            })
            .collect();
        let trunk: Vec<usize> = (0..rig.len())
            .filter(|&joint| {
                matches!(
                    rig.joints[joint].zone,
                    Zone::Pelvis | Zone::Abdomen | Zone::Chest
                ) && rig.joints[joint].role == crate::rig::Role::Deform
            })
            .collect();
        let zone = Zone::Extremity(arm.limb);
        for index in 1..=SPARS {
            let share = index as f32 / (SPARS + 1) as f32;
            let mut points: Vec<Vec3> = (0..=bones)
                .map(|at| front_rest[at].lerp(leg_points[at].0, share))
                .collect();
            points[0] = under_flank(body, points[0], self.side);
            let parent = trunk
                .iter()
                .copied()
                .min_by(|&a, &b| {
                    Arm::at(rig, a)
                        .distance_squared(points[0])
                        .total_cmp(&Arm::at(rig, b).distance_squared(points[0]))
                })
                .unwrap_or(arm.shoulder);
            let mut chain = Vec::with_capacity(points.len());
            let mut at_parent = parent;
            for (at, &point) in points.iter().enumerate() {
                let joint = Book::attach(rig, at_parent, point, zone);
                if at < bones {
                    let turn = det::slerp(front_turn[at], leg_turn[at], share);
                    book.turn(joint, turn, DIGIT_SCHEDULE, Some(arm.limb));
                }
                chain.push(joint);
                at_parent = joint;
            }
            self.spars.push(chain);
        }
    }

    /// Every closed shell of this wing, and a bat's digits, at the fold.
    pub(crate) fn mesh(
        &self,
        rig: &Rig,
        spread: &Spread,
        body: &PolyMesh,
        weights: &SkinWeights,
        arm: &Arm,
    ) -> (PolyMesh, Vec<f32>) {
        let mut mesh = PolyMesh::new();
        let mut reach = Vec::new();
        let Some([hip, knee, ankle]) = self.hind else {
            return (mesh, reach);
        };
        // A chain of joints as an edge, bone by bone; the leg by its length,
        // as a bat's spars blend it.
        let chain = |joints: &[usize]| -> Edge { along_bones(rig, joints, SAMPLES) };
        let leg_edge = match self.kind {
            Kind::Bat => along_chain(rig, &[hip, knee, ankle], SAMPLES),
            _ => chain(&[hip, knee, ankle]),
        };
        let half = HALF_THICKNESS * self.trunk;
        let mut pieces: Vec<(PolyMesh, Vec<f32>)> = Vec::new();
        // The plagiopatagium, or a glider's patagium: from the arm in front,
        // over the spars, to the leg behind, its trailing edge scalloped
        // between their tips.
        let front = self.front(arm);
        let cape = if self.kind == Kind::Bat {
            CAPE_CLING
        } else {
            0.0
        };
        let mut edges = vec![chain(&front)];
        edges.extend(self.spars.iter().map(|spar| chain(spar)));
        edges.push(leg_edge);
        pieces.push(sheet(
            &edges,
            (18, 4),
            (true, true),
            true,
            (spread, body, cape),
            &self.plane,
            half,
        ));
        if self.kind == Kind::Bat {
            // The dactylopatagium, digit V forward to digit II.
            let hand: Vec<Edge> = self.digits.iter().rev().map(|digit| chain(digit)).collect();
            if hand.len() > 1 {
                pieces.push(sheet(
                    &hand,
                    (14, 4),
                    (false, false),
                    false,
                    (spread, body, 0.0),
                    &self.plane,
                    half,
                ));
            }
            // The propatagium: a leading edge from the neck to the thumb's
            // knuckle in front of the arm, each point of it held as the arm
            // is beside it, so it folds with the arm.
            if let Some([thumb, knuckle, _]) = self.thumb {
                let behind = chain(&[arm.shoulder, arm.elbow, arm.wrist, thumb, knuckle]);
                let s = Arm::at(rig, arm.shoulder);
                let neck_at =
                    Vec3::new(s.x * 0.5, s.y, s.z) + self.plane.forward * (0.15 * self.trunk);
                let neck_hold = nearest_hold(body, weights, neck_at);
                // At rest the leading edge lies on the arm's leading side, in
                // its plane and clear of its girth, from the neck to the thumb.
                let clear = 3.0 * rig.joints[arm.elbow].radius.max(0.02 * self.trunk);
                let lead: Edge = behind
                    .iter()
                    .enumerate()
                    .map(|(index, (at, hold))| {
                        let t = index as f32 / (behind.len() - 1) as f32;
                        let before = behind[index.saturating_sub(1)].0;
                        let after = behind[(index + 1).min(behind.len() - 1)].0;
                        let leading = swing(
                            (after - before).normalize_or(self.plane.forward),
                            self.normal,
                            self.side,
                            std::f32::consts::FRAC_PI_2,
                        );
                        let body_share = (1.0 - t / 0.2).max(0.0);
                        (
                            *at + (neck_at - *at) * body_share + leading * clear,
                            Hold::mix(&[(&neck_hold, body_share), (hold, 1.0 - body_share)]),
                        )
                    })
                    .collect();
                pieces.push(sheet(
                    &[behind, lead],
                    (14, 3),
                    (true, false),
                    false,
                    (spread, body, 0.0),
                    &self.plane,
                    half,
                ));
            }
        }
        // Each piece its own column of the part's chart.
        let columns = pieces.len().max(1) as f32;
        for (index, (mut piece, out)) in pieces.into_iter().enumerate() {
            piece.uvs = piece
                .uvs
                .iter()
                .map(|uv| Vec2::new((index as f32 + uv.x) / columns, uv.y))
                .collect();
            mesh.append(&piece);
            reach.extend(out);
        }
        // A bat's digits and thumb: thin closed tubes along their bones, in
        // their own sliver of the chart.
        let tubes = self
            .digits
            .iter()
            .map(|chain| (chain.to_vec(), DIGIT_RADIUS))
            .chain(self.thumb.map(|chain| (chain.to_vec(), THUMB_RADIUS)));
        for (chain, radius) in tubes {
            let mut tube = tube(rig, &chain, radius * self.forearm, self.normal);
            tube.uvs = tube.uvs.iter().map(|uv| *uv * DIGIT_CHART).collect();
            tube.set_normals(tube.vertex_normals());
            // A digit roots at the wrist and runs out to the membrane's edge.
            reach.extend(tube.uvs.iter().map(|uv| uv.y / DIGIT_CHART));
            mesh.append(&tube);
        }
        (mesh, reach)
    }
}

/// `at`, inside the body, carried out across it on the side `side` to just
/// under its surface: where a spar roots in the flank, so the membrane on it
/// leaves the body at once rather than from deep inside it.
fn under_flank(body: &PolyMesh, at: Vec3, side: f32) -> Vec3 {
    carry_out(body, at, side, -0.006)
}

/// `at`, inside the body, carried across it along `x` on the side `side`
/// until it stands `margin` outside its surface (inside, for a negative
/// margin); a point outside the body, or one the body does not let go of
/// within thirty centimetres, stays where it is.
///
/// **Asked on the left, always**: the inside test casts a ray a little off
/// the axes, so it is not quite the same on the two sides of a symmetric
/// body, and a fold pushed by it on each side came out unlike its mirror.
/// The right side's point is asked as its mirror and carried back.
fn carry_out(body: &PolyMesh, at: Vec3, side: f32, margin: f32) -> Vec3 {
    if side < 0.0 {
        let mirrored = carry_out(body, Vec3::new(-at.x, at.y, at.z), 1.0, margin);
        return Vec3::new(-mirrored.x, mirrored.y, mirrored.z);
    }
    if body.positions.is_empty() || !body.contains(at) {
        return at;
    }
    let (mut inside, mut outside) = (0.0f32, 0.3f32);
    let out = |by: f32| at + Vec3::new(side * by, 0.0, 0.0);
    if body.contains(out(outside)) {
        return at;
    }
    for _ in 0..12 {
        let middle = 0.5 * (inside + outside);
        if body.contains(out(middle)) {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    out((outside + margin).max(0.0))
}

/// The skin nearest `at` on the body, as a hold: what a membrane rooted
/// inside the body there moves with.
fn nearest_hold(body: &PolyMesh, weights: &SkinWeights, at: Vec3) -> Hold {
    let nearest = body
        .positions
        .iter()
        .enumerate()
        .min_by(|a, b| {
            a.1.distance_squared(at)
                .total_cmp(&b.1.distance_squared(at))
        })
        .map_or(0, |(index, _)| index);
    weights
        .vertices
        .get(nearest)
        .map_or_else(Hold::default, Hold::of)
}

/// An edge at `u`, `0` its root to `1` its end, between its samples.
fn sample(edge: &Edge, u: f32) -> (Vec3, Hold) {
    let at = u.clamp(0.0, 1.0) * (edge.len() - 1) as f32;
    let index = (at.floor() as usize).min(edge.len() - 2);
    let t = at - index as f32;
    let (a, b) = (&edge[index], &edge[index + 1]);
    (
        a.0 + (b.0 - a.0) * t,
        Hold::mix(&[(&a.1, 1.0 - t), (&b.1, t)]),
    )
}

/// A closed thin shell over the strips between consecutive `edges`, each
/// strip ruled between its two edges where they rest, `nu` rows from the
/// root to the trailing edge and `nv` columns across: each vertex held as
/// the two edges beside it are, blended across the strip - two influences an
/// edge, four in all. `attached` says whether the first edge and the last
/// root in the body - an arm or a leg, inside which their vertices ride - so
/// the reach handed back beside the mesh is nought along them. `whole` scallops the trailing edge once across all the
/// strips, as a plagiopatagium's runs from digit V's tip to the ankle in one
/// curve; otherwise once between each pair of tips, as a hand's does.
/// **The fold is built and the spread is bound**: ruled at
/// rest, a strip lies between the spars it is stretched on wherever they are
/// folded, and a spread pose carries it out between them, each vertex by the
/// blend of their two motions; the spars are close enough, and turn little
/// enough from one to the next, that what that blend leaves out of a taut
/// sheet is a slight billow. The trailing edge is scalloped between the
/// edges' tips; the two sheets stand `half` either side of the middle, along
/// the normal the spread sheet has, carried back to the fold.
fn sheet(
    edges: &[Edge],
    (nu, nv): (usize, usize),
    attached: (bool, bool),
    whole: bool,
    (spread, body, cling): (&Spread, &PolyMesh, f32),
    plane: &Plane,
    half: f32,
) -> (PolyMesh, Vec<f32>) {
    let normals = if cling > 0.0 {
        body.vertex_normals()
    } else {
        Vec::new()
    };
    let strips = edges.len() - 1;
    let columns = strips * nv;
    let scallop = if whole { SCALLOP * 2.0 } else { SCALLOP };
    let mut grid: Vec<(Vec3, VertexSkin, Vec3)> = Vec::with_capacity((nu + 1) * (columns + 1));
    for i in 0..=nu {
        let u = i as f32 / nu as f32;
        for j in 0..=columns {
            let strip = (j / nv).min(strips - 1);
            let v = (j - strip * nv) as f32 / nv as f32;
            // One sweep of the scallop across the whole edge, or one between
            // each pair of spar tips.
            let across = if whole { j as f32 / columns as f32 } else { v };
            let (sin, _) = det::sin_cos(std::f32::consts::PI * across);
            let reach = u * (1.0 - scallop * sin * u * u);
            let (a, hold_a) = sample(&edges[strip], reach);
            let (b, hold_b) = sample(&edges[strip + 1], reach);
            let hold = Hold::mix(&[(&hold_a, 1.0 - v), (&hold_b, v)]).cut();
            // **Never through the body**: a free point of the fold that
            // would rest inside it is carried out across it to clear its
            // surface. What roots in the flank or rides a limb stays.
            let free = u >= 0.2 && !(attached.0 && j <= 1) && !(attached.1 && j + 1 >= columns);
            let mut rest = a + (b - a) * v;
            // A cape drawn in toward the flank under it.
            // Asked on the left and mirrored, as `carry_out` is: the nearest
            // vertex ties differently on the two sides of a symmetric body.
            if free && cling > 0.0 {
                let flip = Vec3::new(plane.out.x.signum(), 1.0, 1.0);
                let asked = rest * flip;
                if let Some((near, out)) = body.positions.iter().zip(&normals).min_by(|x, y| {
                    x.0.distance_squared(asked)
                        .total_cmp(&y.0.distance_squared(asked))
                }) {
                    let stands = (asked - *near).dot(*out);
                    let wanted = 8.0 * half;
                    if stands > wanted {
                        rest -= *out * flip * ((stands - wanted) * cling);
                    }
                }
            }
            let rest = if free {
                carry_out(body, rest, plane.out.x.signum(), 4.0 * half)
            } else {
                rest
            };
            grid.push((rest, hold, spread.forward(&hold, rest)));
        }
    }
    let at = |i: usize, j: usize| i * (columns + 1) + j;
    let point = |i: usize, j: usize| grid[at(i, j)].2;
    let tangents = |i: usize, j: usize| {
        let du = point((i + 1).min(nu), j) - point(i.saturating_sub(1), j);
        let dv = point(i, (j + 1).min(columns)) - point(i, j.saturating_sub(1));
        du.cross(dv)
    };
    // Which way the grid's own normal faces: the sheets are wound so the top
    // one faces the wing's upper side.
    let facing: f32 = (0..=nu)
        .flat_map(|i| (0..=columns).map(move |j| (i, j)))
        .map(|(i, j)| tangents(i, j).dot(plane.dorsal))
        .sum();
    let sign = if facing >= 0.0 { 1.0 } else { -1.0 };

    let mut mesh = PolyMesh::new();
    let mut skins = Vec::new();
    let mut uvs = Vec::new();
    let mut normals = Vec::new();
    let mut reach = Vec::new();
    for offset in [half, -half] {
        for i in 0..=nu {
            for j in 0..=columns {
                // An edge rooted in a limb rides inside it, and so does the
                // column of membrane beside it.
                let rooted = (attached.0 && j <= 1) || (attached.1 && j + 1 >= columns);
                reach.push(if rooted { 0.0 } else { i as f32 / nu as f32 });
                let (rest, hold, spread_at) = grid[at(i, j)];
                let normal = (tangents(i, j) * sign).normalize_or(plane.dorsal);
                let (_, facing) = spread.inverse(&hold, spread_at, normal);
                mesh.push_vertex(rest + facing * offset);
                skins.push(hold);
                uvs.push(Vec2::new(j as f32 / columns as f32, i as f32 / nu as f32));
                normals.push(facing * offset.signum());
            }
        }
    }
    let count = (nu + 1) * (columns + 1);
    let top = |i: usize, j: usize| at(i, j) as u32;
    let bottom = |i: usize, j: usize| (count + at(i, j)) as u32;
    let mut faces: Vec<Vec<u32>> = Vec::new();
    // **Each quad split on the diagonal that keeps both its halves facing the
    // sheet's way spread**: a strip carried out by the blend of two spars is
    // a little twisted, and a twisted quad split on its other diagonal folds
    // one half over - which a wing wound one way and its mirror wound the
    // other drew as a checkerboard of holes on one side only.
    for i in 0..nu {
        for j in 0..columns {
            let corners = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
            let p = corners.map(|(a, b)| point(a, b));
            let facing = corners.iter().map(|&(a, b)| tangents(a, b)).sum::<Vec3>() * sign;
            let worst = |tri: [usize; 3]| {
                ((p[tri[1]] - p[tri[0]]).cross(p[tri[2]] - p[tri[0]]) * sign)
                    .normalize_or(Vec3::ZERO)
                    .dot(facing.normalize_or(plane.dorsal))
            };
            let split = if worst([0, 1, 2]).min(worst([0, 2, 3]))
                >= worst([0, 1, 3]).min(worst([1, 2, 3]))
            {
                [[0, 1, 2], [0, 2, 3]]
            } else {
                [[0, 1, 3], [1, 2, 3]]
            };
            for tri in split {
                let at = tri.map(|corner| corners[corner]);
                faces.push(at.iter().map(|&(a, b)| top(a, b)).collect());
                faces.push(at.iter().rev().map(|&(a, b)| bottom(a, b)).collect());
            }
        }
    }
    // The rim, round the grid's edge counter-clockwise in `(i, j)`.
    let mut ring: Vec<(usize, usize)> = Vec::new();
    ring.extend((0..nu).map(|i| (i, 0)));
    ring.extend((0..columns).map(|j| (nu, j)));
    ring.extend((1..=nu).rev().map(|i| (i, columns)));
    ring.extend((1..=columns).rev().map(|j| (0, j)));
    for index in 0..ring.len() {
        let (a, b) = (ring[index], ring[(index + 1) % ring.len()]);
        faces.push(vec![
            top(b.0, b.1),
            top(a.0, a.1),
            bottom(a.0, a.1),
            bottom(b.0, b.1),
        ]);
    }
    for mut face in faces {
        if sign < 0.0 {
            face.reverse();
        }
        mesh.push_face(face);
    }
    mesh.set_uvs(uvs);
    mesh.set_skin(skins);
    mesh.set_normals(normals);
    (mesh, reach)
}

/// A closed tube of [`TUBE_SIDES`] along a chain of joints at rest, `radius`
/// at its root and half that at its tip, capped at both ends: a digit's bone
/// under its skin. Each ring at a joint is shared between the bones either
/// side; each mid-bone ring is its bone's alone.
fn tube(rig: &Rig, chain: &[usize], radius: f32, reference: Vec3) -> PolyMesh {
    let points: Vec<Vec3> = chain.iter().map(|&joint| Arm::at(rig, joint)).collect();
    let bones = points.len() - 1;
    // Stations: each joint and each bone's middle, with the hold at each.
    let mut stations: Vec<(Vec3, Vec3, f32, VertexSkin)> = Vec::new();
    let total: f32 = points
        .windows(2)
        .map(|p| p[0].distance(p[1]))
        .sum::<f32>()
        .max(1e-6);
    let mut run = 0.0f32;
    for bone in 0..bones {
        let (a, b) = (points[bone], points[bone + 1]);
        let along = (b - a).normalize_or(Vec3::Z);
        let joint_hold = if bone == 0 {
            Hold::one(chain[0])
        } else {
            Hold(vec![(chain[bone - 1], 0.5), (chain[bone], 0.5)])
        };
        let incoming = if bone == 0 {
            along
        } else {
            ((points[bone] - points[bone - 1]).normalize_or(along) + along).normalize_or(along)
        };
        stations.push((a, incoming, run / total, joint_hold.cut()));
        let length = a.distance(b);
        stations.push((
            (a + b) * 0.5,
            along,
            (run + length * 0.5) / total,
            Hold::one(chain[bone]).cut(),
        ));
        run += length;
    }
    let last = points[bones];
    let last_along = (last - points[bones - 1]).normalize_or(Vec3::Z);
    stations.push((last, last_along, 1.0, Hold::one(chain[bones - 1]).cut()));

    let mut mesh = PolyMesh::new();
    let mut skins = Vec::new();
    let mut uvs = Vec::new();
    for &(centre, along, share, hold) in &stations {
        let e1 =
            (reference - along * reference.dot(along)).normalize_or(along.any_orthonormal_vector());
        let e2 = along.cross(e1);
        let r = radius * (1.0 - 0.5 * share);
        for side in 0..TUBE_SIDES {
            let angle = std::f32::consts::TAU * side as f32 / TUBE_SIDES as f32;
            let (sin, cos) = det::sin_cos(angle);
            mesh.push_vertex(centre + (e1 * cos + e2 * sin) * r);
            skins.push(hold);
            uvs.push(Vec2::new(side as f32 / TUBE_SIDES as f32, share));
        }
    }
    let ring = |station: usize, side: usize| (station * TUBE_SIDES + side % TUBE_SIDES) as u32;
    for station in 0..stations.len() - 1 {
        for side in 0..TUBE_SIDES {
            mesh.push_face(vec![
                ring(station, side),
                ring(station + 1, side),
                ring(station + 1, side + 1),
                ring(station, side + 1),
            ]);
        }
    }
    // A cap at the root, a point at the tip (a claw on the thumb, a
    // fingertip on a spar).
    let root = mesh.push_vertex(stations[0].0 - stations[0].1 * (radius * 0.5));
    skins.push(stations[0].3);
    uvs.push(Vec2::ZERO);
    let point = mesh.push_vertex(last + last_along * (radius * 1.5));
    skins.push(stations[stations.len() - 1].3);
    uvs.push(Vec2::new(0.5, 1.0));
    let end = stations.len() - 1;
    for side in 0..TUBE_SIDES {
        mesh.push_face(vec![ring(0, side), ring(0, side + 1), root]);
        mesh.push_face(vec![ring(end, side + 1), ring(end, side), point]);
    }
    mesh.set_uvs(uvs);
    mesh.set_skin(skins);
    // Wound outward by measurement: the ring's order round the bone depends
    // on the frame, and the volume says which way it came out.
    if signed_volume(&mesh) < 0.0 {
        for face in &mut mesh.faces {
            face.reverse();
        }
    }
    mesh
}

/// A closed mesh's signed volume: positive wound outward.
pub(crate) fn signed_volume(mesh: &PolyMesh) -> f32 {
    mesh.triangulated()
        .iter()
        .map(|tri| {
            let (a, b, c) = (
                mesh.positions[tri[0] as usize],
                mesh.positions[tri[1] as usize],
                mesh.positions[tri[2] as usize],
            );
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}
