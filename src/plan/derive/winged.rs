//! Every dimension the winged cage is built from (#420).
//!
//! Seven axes, the family blend and the composites in, one
//! [`Dimensions`] out - the beast's own, so the graph the plan hangs
//! (`plan::quadruped::hang`) is the beast's node for node - and how many of
//! each limb's nodes are markers inside the body.
//!
//! **Every length is a share of the trunk**, `T`, the shoulder joint to the
//! hip joint, which no posture changes: a bird perches with its trunk raised,
//! a bat crawls with it level, and a cross between them blends its bones and
//! rings in shares of a trunk both have, then stands as its leading family
//! does. research-10 normalises a wing against the humerus, which this turns
//! into shares of the trunk family by family (docs/winged-anatomy.md).
//!
//! **What blends and what does not.** A [`Frame`] - bone lengths, the six
//! rings, girths, the head's proportions - blends between a cross's two
//! families by its blend, as a quadruped's does; a [`Schema`] says what
//! `appeal` makes of it, blended the same way. The [`Posture`] is the leading
//! family's, and with it which nodes of a limb lie inside the body as markers:
//! a bird's whole wing arm, its femur and its knee; a bat's and a glider's
//! shoulder blade, shoulder and hip, as a quadruped's.
//!
//! New trigonometry here goes through [`crate::det`], so a body is the same
//! bits natively and in a browser.

use glam::{Quat, Vec2, Vec3};

use super::quadruped::head::{self, Chain};
use super::quadruped::{Dimensions, FORE_NODES, SKIN, TAIL_NODES, TRUNK_NODES};
use crate::det::DetMath;
use crate::plan::winged::{Posture, WingedFamily, WingedParams};
use crate::plan::{Blended, Composites, DEFAULT_BODY_FAT, Nose, QuadrupedParams};

/// What [`derive()`] hands the plan: the body, and how many of each leg's nodes
/// from the body down are markers inside it.
pub(crate) struct Built {
    /// The body's every node.
    pub dims: Dimensions,
    /// Markers at the top of each hind leg.
    pub hind_inside: usize,
    /// Markers at the top of each fore limb: all of them on a bird, whose
    /// folded wing is the wing part's to draw (#421).
    pub fore_inside: usize,
    /// Markers along the tail from its root: all of them on a bird, whose
    /// bony tail is a stub under its rump and whose visible tail is the fan
    /// the tail part draws (#421) - the owner, 2026-11-05: a short stub,
    /// never a tube to the ground.
    pub tail_inside: usize,
}

/// One trunk ring, in trunks: how far along the trunk it stands from the hip
/// joint (`0`) toward the shoulder joint (`1`), how far its centre sits off
/// that line, belly-ward negative, and its half-depth and half-width as the
/// skin should measure them.
#[derive(Clone, Copy, Debug)]
struct Ring {
    at: f32,
    lift: f32,
    depth: f32,
    width: f32,
}

impl Ring {
    const fn at(at: f32, lift: f32, depth: f32, width: f32) -> Self {
        Self {
            at,
            lift,
            depth,
            width,
        }
    }
}

/// The head a family wears (#422): the beast head's axes, which its
/// construction reads (`quadruped::head`), and the sizes it departs from a
/// beast's by (`winged_head`). A flat-faced owl's, its eyes on the front of
/// its face and no ear to see; a fox-faced fruit bat's, its eyes large and its
/// ears large and plain; a sugar glider's short snout, its eyes larger still,
/// set wide and standing out of its head, under large rounded ears (research-10
/// 6.1). A cross blends every one.
#[derive(Clone, Copy, Debug)]
struct Face {
    muzzle_length: f32,
    muzzle_breadth: f32,
    stop: f32,
    eye_set: f32,
    ear_size: f32,
    ear_point: f32,
    ear_carriage: f32,
    whiskers: f32,
    /// The eyes' globes against a beast's.
    eye_scale: f32,
    /// How much of a globe stands proud of the head (a beast's 0.40).
    proud: f32,
    /// The ears against a beast's of the same axes: none on a bird.
    ear_scale: f32,
}

/// The body a family is built in, in trunks.
#[derive(Clone, Copy, Debug)]
struct Frame {
    /// The head-and-body length a record's `size` names, in trunks, at the
    /// family's own proportions: measured off the built body at its anchor.
    hb: f32,
    /// The trunk's rings, pelvis first.
    rings: [Ring; TRUNK_NODES],
    /// The shoulder blade, its top to the shoulder joint.
    scapula: f32,
    /// The humerus.
    humerus: f32,
    /// The forearm, elbow to wrist: the ulna.
    forearm: f32,
    /// The wrist to the placeholder hand's first node: a bird's
    /// carpometacarpus, a bat's wrist pad, a glider's metacarpus.
    hand: f32,
    /// The hand's first node to its last: a bird's digit, a bat's thumb, a
    /// glider's toes.
    digit: f32,
    /// The arm's radius at the elbow.
    arm: f32,
    /// The femur.
    femur: f32,
    /// The tibia, or a bird's tibiotarsus.
    tibia: f32,
    /// The ankle to the pad: a bird's tarsometatarsus, a mammal's metatarsus.
    metatarsus: f32,
    /// The pad to the toes' tip.
    toe: f32,
    /// The leg's radius at the knee, or a bird's at the intertarsal joint.
    leg: f32,
    /// A pad's radius.
    pad: f32,
    /// How far the neck reaches from the top of the forechest.
    neck: f32,
    /// The neck's radius.
    neck_r: f32,
    /// The cranium's radius.
    head_r: f32,
    /// How far the back of the skull goes toward a poll (`Chain::back`).
    skull_back: f32,
    /// The tail's length.
    tail: f32,
    /// The tail's radius at its root.
    tail_r: f32,
    /// How much of its radius each tail node gives up toward the tip.
    taper: f32,
    /// The head's shape.
    face: Face,
}

impl Frame {
    /// One family's frame. See docs/winged-anatomy.md for each figure's row in
    /// research-10 and its confidence.
    fn of(family: WingedFamily) -> Self {
        match family {
            // The wing bones are a crow's against its humerus (S1, S2: ulna
            // 1.22, carpometacarpus 0.74, digits about 0.45 of it), the
            // humerus half a trunk (#421: a crow's 68 mm humerus on a trunk of
            // about 145 mm, bg; W1's 0.77 spread the bones alone past the
            // span a crow's feathers reach); the leg bones a corvid's (S2, S42, bg:
            // femur 0.65, tibiotarsus 1.15, tarsometatarsus 0.72). The body is
            // the feathered outline, an owl's egg: about 1.6 trunks from rump
            // to breast and about as wide as it is deep, centred under the
            // spine where the keel and the belly feathers are (bg, chosen by
            // render). A small owl's round head on no visible neck.
            WingedFamily::Bird => Self {
                hb: 2.29,
                rings: [
                    Ring::at(0.12, -0.22, 0.32, 0.30),
                    Ring::at(0.52, -0.32, 0.48, 0.44),
                    Ring::at(0.76, -0.34, 0.52, 0.48),
                    Ring::at(0.98, -0.30, 0.52, 0.48),
                    Ring::at(1.16, -0.22, 0.46, 0.43),
                    Ring::at(1.30, -0.10, 0.34, 0.34),
                ],
                scapula: 0.55,
                humerus: 0.50,
                forearm: 0.61,
                hand: 0.37,
                digit: 0.17,
                arm: 0.05,
                femur: 0.42,
                tibia: 0.62,
                metatarsus: 0.30,
                toe: 0.28,
                leg: 0.07,
                pad: 0.075,
                neck: 0.05,
                neck_r: 0.34,
                head_r: 0.36,
                skull_back: 0.0,
                tail: 0.12,
                tail_r: 0.07,
                taper: 0.10,
                face: Face {
                    muzzle_length: -1.5,
                    muzzle_breadth: 0.3,
                    stop: 1.0,
                    eye_set: super::winged_head::FRONTAL_EYES,
                    ear_size: -1.0,
                    ear_point: 1.0,
                    ear_carriage: 1.0,
                    whiskers: -1.5,
                    eye_scale: 1.45,
                    proud: 0.40,
                    ear_scale: 0.0,
                },
            },
            // A flying fox's (S20, S21): the forearm 1.5 humeri (bg), the
            // humerus about one trunk; the legs short and slight (bg), the
            // hind foot about 0.4 trunks (S21). A slim, furred barrel, the
            // head a fox's on a short neck, near tailless (S21, S22).
            WingedFamily::Bat => Self {
                hb: 2.50,
                rings: [
                    Ring::at(0.06, 0.0, 0.22, 0.20),
                    Ring::at(0.30, -0.02, 0.26, 0.24),
                    Ring::at(0.46, -0.05, 0.30, 0.27),
                    Ring::at(0.61, -0.06, 0.32, 0.29),
                    Ring::at(0.75, -0.05, 0.30, 0.27),
                    Ring::at(0.95, -0.02, 0.24, 0.22),
                ],
                scapula: 0.35,
                humerus: 1.0,
                forearm: 1.5,
                hand: 0.08,
                digit: 0.18,
                arm: 0.08,
                femur: 0.55,
                tibia: 0.65,
                metatarsus: 0.12,
                toe: 0.40,
                leg: 0.05,
                pad: 0.05,
                neck: 0.10,
                neck_r: 0.18,
                head_r: 0.21,
                skull_back: 0.3,
                tail: 0.04,
                tail_r: 0.03,
                taper: 0.14,
                face: Face {
                    muzzle_length: 0.55,
                    muzzle_breadth: -0.7,
                    stop: 0.6,
                    eye_set: -0.4,
                    ear_size: 1.4,
                    ear_point: 0.1,
                    ear_carriage: 1.0,
                    whiskers: -0.7,
                    eye_scale: 1.70,
                    proud: 0.50,
                    ear_scale: 1.25,
                },
            },
            // A sugar glider's (S30, S31, S36): an elongated small mammal on
            // long, gracile limbs (S36, chosen by render: no bone is measured
            // here), a tail as long as its head and body (S30), a short
            // snout, big eyes set wide and big ears (S31).
            WingedFamily::Glider => Self {
                hb: 2.41,
                rings: [
                    Ring::at(0.06, 0.0, 0.21, 0.22),
                    Ring::at(0.30, 0.04, 0.25, 0.28),
                    Ring::at(0.46, 0.05, 0.27, 0.30),
                    Ring::at(0.61, 0.04, 0.27, 0.30),
                    Ring::at(0.75, 0.02, 0.24, 0.26),
                    Ring::at(0.95, 0.01, 0.20, 0.21),
                ],
                scapula: 0.28,
                humerus: 0.32,
                forearm: 0.34,
                hand: 0.10,
                digit: 0.10,
                arm: 0.075,
                femur: 0.38,
                tibia: 0.42,
                metatarsus: 0.20,
                toe: 0.12,
                leg: 0.11,
                pad: 0.065,
                neck: 0.12,
                neck_r: 0.17,
                head_r: 0.28,
                skull_back: 0.0,
                tail: 2.3,
                tail_r: 0.14,
                taper: 0.0,
                face: Face {
                    muzzle_length: -1.0,
                    muzzle_breadth: 0.0,
                    stop: 0.7,
                    eye_set: 0.8,
                    ear_size: 1.5,
                    ear_point: -0.8,
                    ear_carriage: 0.9,
                    whiskers: 0.2,
                    eye_scale: 1.90,
                    proud: 0.62,
                    ear_scale: 1.0,
                },
            },
        }
    }

    /// `own` drawn `t` of the way to `other`, measure by measure.
    fn mixed(own: Self, other: Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        let (a, b) = (own.face, other.face);
        Self {
            hb: mix(own.hb, other.hb),
            rings: std::array::from_fn(|i| {
                let (a, b) = (own.rings[i], other.rings[i]);
                Ring::at(
                    mix(a.at, b.at),
                    mix(a.lift, b.lift),
                    mix(a.depth, b.depth),
                    mix(a.width, b.width),
                )
            }),
            scapula: mix(own.scapula, other.scapula),
            humerus: mix(own.humerus, other.humerus),
            forearm: mix(own.forearm, other.forearm),
            hand: mix(own.hand, other.hand),
            digit: mix(own.digit, other.digit),
            arm: mix(own.arm, other.arm),
            femur: mix(own.femur, other.femur),
            tibia: mix(own.tibia, other.tibia),
            metatarsus: mix(own.metatarsus, other.metatarsus),
            toe: mix(own.toe, other.toe),
            leg: mix(own.leg, other.leg),
            pad: mix(own.pad, other.pad),
            neck: mix(own.neck, other.neck),
            neck_r: mix(own.neck_r, other.neck_r),
            head_r: mix(own.head_r, other.head_r),
            skull_back: mix(own.skull_back, other.skull_back),
            tail: mix(own.tail, other.tail),
            tail_r: mix(own.tail_r, other.tail_r),
            taper: mix(own.taper, other.taper),
            face: Face {
                muzzle_length: mix(a.muzzle_length, b.muzzle_length),
                muzzle_breadth: mix(a.muzzle_breadth, b.muzzle_breadth),
                stop: mix(a.stop, b.stop),
                eye_set: mix(a.eye_set, b.eye_set),
                ear_size: mix(a.ear_size, b.ear_size),
                ear_point: mix(a.ear_point, b.ear_point),
                ear_carriage: mix(a.ear_carriage, b.ear_carriage),
                whiskers: mix(a.whiskers, b.whiskers),
                eye_scale: mix(a.eye_scale, b.eye_scale),
                proud: mix(a.proud, b.proud),
                ear_scale: mix(a.ear_scale, b.ear_scale),
            },
        }
    }
}

/// What `appeal` makes of a family's body, as the share each measure gains at
/// `appeal` 1.5, all the way to its young (the owner's call, 2026-11-05): an
/// owlet's or a fledgling's, a pup's, a joey's. research-10's fact 10: the
/// baby schema of research-07 - a large head, a short face, a round outline -
/// and a pup is big-headed and short-winged (S26). Below zero each runs the
/// other way by half as far at most, as on a beast.
#[derive(Clone, Copy, Debug)]
struct Schema {
    /// The head's radius.
    head: f32,
    /// The trunk's rings, rounder.
    round: f32,
    /// The neck's reach, shorter.
    neck: f32,
    /// The tail's length, shorter.
    tail: f32,
    /// The arm, shorter: a pup's (S26), and no other family's - research-10
    /// says a cute default should not shrink a wing.
    arm: f32,
    /// The legs' girth, sturdier.
    sturdy: f32,
}

impl Schema {
    fn of(family: WingedFamily) -> Self {
        match family {
            WingedFamily::Bird => Self {
                head: 0.40,
                round: 0.12,
                neck: 0.30,
                tail: 0.35,
                arm: 0.0,
                sturdy: 0.20,
            },
            WingedFamily::Bat => Self {
                head: 0.45,
                round: 0.10,
                neck: 0.25,
                tail: 0.0,
                arm: 0.15,
                sturdy: 0.15,
            },
            WingedFamily::Glider => Self {
                head: 0.50,
                round: 0.12,
                neck: 0.25,
                tail: 0.30,
                arm: 0.0,
                sturdy: 0.15,
            },
        }
    }

    fn mixed(own: Self, other: Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            head: mix(own.head, other.head),
            round: mix(own.round, other.round),
            neck: mix(own.neck, other.neck),
            tail: mix(own.tail, other.tail),
            arm: mix(own.arm, other.arm),
            sturdy: mix(own.sturdy, other.sturdy),
        }
    }
}

/// The shortest bone from a girdle's ring to the ring beside it, in the
/// girdle's own cage radii: **a cage wall**, so its socket opens on the hull.
const GIRDLE_BONE: f32 = 0.75;

/// Where a four-legged body's rings stand along its trunk, the hip at `0`
/// and the shoulder at `1`: the bat's and the glider's, and every cross that
/// stands as they do.
const FOUR_LEGGED_AT: [f32; TRUNK_NODES] = [0.06, 0.30, 0.46, 0.61, 0.75, 0.95];

/// The shortest bridge from a girdle's node to the first joint a leg's cage
/// sweeps to, in the girdle's cage radii: **a cage wall**, so the bridge's
/// socket opens on the hull.
const BRIDGE: f32 = 1.7;

/// The pelvis's own: four sockets crowd its hull, the loin's among them.
const PELVIS_BONE: f32 = 1.1;

/// The shortest neck bone off the forechest, in the forechest's cage radii:
/// **a cage wall**, the beast's floor on its neck, so the neck's socket clears
/// the fore limbs' on the forechest's hull.
const NECK_BONE: f32 = 1.2;

/// Volume per unit length at `mass` +1, relative to neutral: the beast's and
/// the person's, so one `mass` is one amount of body on every plan.
const MASS_SPAN: f32 = 1.49;

/// The tissue a lean body keeps, as a body-fat fraction: the beast's.
const SOFT: f32 = 0.08;

/// How a perching bird's leg is set, in radians: the tarsometatarsus leaning
/// back from the foot as it rises, the femur forward and down from the hip,
/// and the tibiotarsus between them (bg, research-10 note 3.1's femur
/// "near horizontal inside the body wall"; chosen by render).
const PERCH_TARSUS: f32 = 0.30;
/// See [`PERCH_TARSUS`]: the tibiotarsus, forward of vertical as it rises.
const PERCH_TIBIA: f32 = 1.1;
/// See [`PERCH_TARSUS`]: the femur, below level as it runs forward.
const PERCH_FEMUR: f32 = 0.25;

/// How far a perching bird's trunk is raised from level, in radians: an
/// owl's upright carriage (bg; chosen by render).
const PERCH_PITCH: f32 = 0.95;

/// The folded bird wing's angles (#421), in radians: the elbow's interior
/// angle and the wrist's (research-10 4.1: crown birds' median 31 and 52
/// degrees, S5; the hand folds to 20 to 50 degrees in life), and how far the
/// folded humerus points up from straight back along the flank (chosen by
/// render).
const FOLD_ELBOW: f32 = 0.54;
/// See [`FOLD_ELBOW`].
const FOLD_WRIST: f32 = 0.79;
/// See [`FOLD_ELBOW`].
const FOLD_HUMERUS: f32 = 0.20;

/// How far outside the trunk the folded wing's plane stands, in trunks: the
/// flight feathers rooted on its forearm and hand lie on the body rather
/// than in it (#421).
const FOLD_MARGIN: f32 = 0.02;

/// How far the folded wing's plane leans in from the flank over the back, in
/// radians: a perched bird's wing lies on its upper flank and its back, the
/// primaries crossing over the rump (#421; chosen by render).
const FOLD_LEAN: f32 = 0.55;

/// How far a crawling bat's trunk is raised, and how close its belly comes to
/// the ground, in trunks (research-10 6.2: quadrupedal on thumbs and feet,
/// wings folded; chosen by render).
const CRAWL_PITCH: f32 = 0.10;
/// See [`CRAWL_PITCH`].
const CRAWL_BELLY: f32 = 0.10;

/// Where a crawling bat plants each wrist against its shoulder, and each hind
/// foot against its hip, in trunks `(out, ahead)`: wide and forward, the
/// elbows up; the feet out and back, the knees up and out (S22: the knee faces
/// dorsolaterally).
const CRAWL_WRIST: Vec2 = Vec2::new(0.55, 0.70);
/// See [`CRAWL_WRIST`].
const CRAWL_FOOT: Vec2 = Vec2::new(0.40, -0.25);
/// Which way a crawling bat's hind foot runs from its pad, `(out, up, ahead)`:
/// out and back, its toes trailing (#424), as a bat's rotated hind limb sets
/// it on the ground (S22, research-11 2.3).
const CRAWL_TOES: Vec3 = Vec3::new(0.45, 0.0, -0.9);

/// How far a standing glider's legs are bent: the hip's height as a share of
/// the femur, tibia and metatarsus laid end to end; the humerus's slope down
/// from the shoulder to the elbow behind it. A small mammal stands crouched
/// (chosen by render).
const STAND_HIP: f32 = 0.48;
/// See [`STAND_HIP`]: how far behind the shoulder the wrist is planted, in
/// humeri, so the elbow folds back above it.
const STAND_WRIST_BEHIND: f32 = 0.3;

/// Derives the body of `params` under `composites`.
pub(crate) fn derive(params: &WingedParams, composites: &Composites) -> Built {
    let lineage = params.lineage();
    let frame = lineage.blended(Frame::of, Frame::mixed);
    let schema = lineage.blended(Schema::of, Schema::mixed);
    let posture = params.posture();
    // Metres per trunk: the record's size is a head-and-body length.
    let t = params.size / frame.hb;

    let appeal = params.appeal.clamp(-1.5, 2.0);
    let young = (appeal / 1.5).clamp(-0.5, 1.33);

    // The composites, as on a beast: `mass` how much body there is, `bodyFat`
    // how it is spent. Exponents chosen, the beast's pattern: the trunk
    // fastest, the legs slower.
    let lean = MASS_SPAN.det_powf(composites.mass);
    let fat = lean * (composites.body_fat.clamp(0.0, 0.95) + SOFT) / (DEFAULT_BODY_FAT + SOFT);
    let trunk_girth = (lean.det_powf(0.30) * fat.det_powf(0.15)).clamp(0.6, 1.7);
    let limb_girth = lean.det_powf(0.25).clamp(0.7, 1.4);

    // The axes, each a step from the family's own frame.
    let arm = ((1.0 + 0.15 * params.wing_length) * (1.0 - schema.arm * young)).clamp(0.55, 1.6);
    let leg = (1.0 + 0.15 * params.leg_length).clamp(0.8, 1.4);
    let neck_reach = frame.neck
        * ((1.0 + 0.3 * params.neck_length) * (1.0 - schema.neck * young)).clamp(0.3, 2.2);
    let tail_len = ((frame.tail * (1.0 + 0.4 * params.tail_length) + 0.12 * params.tail_length)
        * (1.0 - schema.tail * young.max(0.0)))
    .max(0.04);
    let head_r = frame.head_r
        * (1.0 + 0.25 * params.head_size).clamp(0.4, 2.0)
        * (1.0 + schema.head * young);
    let round = 1.0 + schema.round * young;
    let sturdy = (1.0 + schema.sturdy * young) * limb_girth;

    let humerus = frame.humerus * arm;
    let forearm = frame.forearm * arm;
    let hand = frame.hand * arm;
    let digit = frame.digit * arm;
    let femur = frame.femur * leg;
    let tibia = frame.tibia * leg;
    let metatarsus = frame.metatarsus * leg;
    // How far ahead of the pad the foot's second node stands (#422): a
    // bird's ball and a bat's sole are only part of the foot, their drawn
    // toes the rest; a glider's paw is the whole of it.
    let toe = frame.toe * params.winged_foot().sole_share();
    let pad_r = frame.pad * sturdy;
    let pad_y = pad_r * 0.82;
    let toe_r = pad_r * 0.8;

    // The trunk's rings about its own line, in trunks, before it is stood.
    // Where each ring stands along the trunk is the posture's: a bird's egg
    // reaches past its shoulders, where nothing hangs from its breast, and a
    // body on four legs hangs its fore limbs from a forechest behind them,
    // as a beast does, so their sockets open forward and clear the ribs'.
    let rings: [Ring; TRUNK_NODES] = std::array::from_fn(|i| {
        let ring = frame.rings[i];
        Ring {
            at: match posture {
                Posture::Perch => ring.at,
                Posture::Crawl | Posture::Stand => FOUR_LEGGED_AT[i],
            },
            depth: ring.depth * round * trunk_girth.sqrt(),
            width: ring.width * round * trunk_girth,
            ..ring
        }
    });
    // **Cage walls, not anatomy**: the trunk is stretched along its line,
    // hip to shoulder and every ring with them, until each girdle's bone to
    // the ring beside it is long enough to open its socket on the girdle's
    // hull, and a run between two girdles (a body on four legs) is the
    // beast's `SPAN_FLOOR` of its largest ring. A round body on a short
    // trunk grows a little longer instead of refusing.
    let cage_radius = |ring: Ring| ring.depth.max(ring.width) / SKIN;
    let largest = rings
        .iter()
        .map(|&ring| cage_radius(ring))
        .fold(0.0, f32::max);
    let last = TRUNK_NODES - 1;
    let mut stretch = (PELVIS_BONE * cage_radius(rings[0]) / (rings[1].at - rings[0].at)).max(1.0);
    if posture != Posture::Perch {
        stretch = stretch
            .max(GIRDLE_BONE * cage_radius(rings[last]) / (rings[last].at - rings[last - 1].at))
            .max(super::quadruped::SPAN_FLOOR * largest / (rings[last].at - rings[0].at));
    }
    let rings = rings.map(|ring| Ring {
        at: ring.at * stretch,
        ..ring
    });

    let pitch = match posture {
        Posture::Perch => PERCH_PITCH,
        Posture::Crawl => CRAWL_PITCH,
        Posture::Stand => 0.0,
    };
    // The trunk's line and the way out of its back, in the side plane.
    let along = Vec3::new(0.0, pitch.det_sin(), pitch.det_cos());
    let up = Vec3::new(0.0, pitch.det_cos(), -pitch.det_sin());
    let centre = |hip: Vec3, ring: Ring| hip + along * ring.at + up * ring.lift;

    // Lateral seats of the joints inside the body, each a share of its
    // ring's half-width.
    let pelvis_w = rings[0].width;
    let forechest_w = rings[TRUNK_NODES - 1].width;
    let hip_x = pelvis_w * 0.75;
    let shoulder_x = forechest_w
        * match posture {
            Posture::Perch => 0.6,
            Posture::Crawl | Posture::Stand => 1.2,
        };

    // The centre of the trunk's mass along the body: the rings weighed by
    // their sections. A perching bird stands its feet under it.
    let mass_z = {
        let (sum, weight) = rings.iter().fold((0.0, 0.0), |(s, w), &ring| {
            let weight = ring.depth * ring.width;
            (s + centre(Vec3::ZERO, ring).z * weight, w + weight)
        });
        sum / weight
    };
    // A perching bird's leg from its hip: the knee forward and down inside the
    // body, out against its wall (bg) - its socket's aim, which has to leave
    // the loin's - the foot under the trunk's mass and the intertarsal joint
    // bent back between them, the bird's "knee" being its ankle.
    let perch_leg = |hip: Vec3| {
        let knee = hip
            + Vec3::new(
                hip_x + 0.45 * femur,
                -femur * PERCH_FEMUR.det_sin(),
                femur * PERCH_FEMUR.det_cos(),
            );
        let foot = Vec3::new(hip_x * 0.9, pad_y, mass_z + 0.03);
        let ankle = bend(knee, foot, tibia, metatarsus, Vec3::NEG_Z);
        (knee, foot, ankle)
    };

    // **Where the hip stands**, from the posture: a bird's on its legs, set
    // from the foot up; a bat's where its belly clears the ground; a glider's
    // a crouch's share of its leg.
    let hip_y = match posture {
        Posture::Perch => {
            let mut hip_y = pad_y
                + metatarsus * PERCH_TARSUS.det_cos()
                + tibia * PERCH_TIBIA.det_cos()
                + femur * PERCH_FEMUR.det_sin();
            // **A cage wall, not anatomy**: the body stands as much higher
            // as the leg's bridge needs to open its socket on the pelvis's
            // hull. Short legs under a heavy rump stand a little taller.
            let pelvis = centre(Vec3::ZERO, rings[0]);
            let least = BRIDGE * cage_radius(rings[0]);
            for _ in 0..4 {
                let hip = Vec3::new(0.0, hip_y, 0.0);
                let gap = perch_leg(hip).2.distance(hip + pelvis);
                if gap >= least {
                    break;
                }
                hip_y += least - gap;
            }
            hip_y
        }
        Posture::Crawl => {
            let lowest = rings
                .iter()
                .map(|&ring| centre(Vec3::ZERO, ring).y - ring.depth * pitch.det_cos())
                .fold(f32::MAX, f32::min);
            CRAWL_BELLY - lowest
        }
        Posture::Stand => pad_y + STAND_HIP * (femur + tibia + metatarsus),
    };
    let hip = Vec3::new(0.0, hip_y, 0.0);
    let shoulder_on_line = hip + along * stretch;
    let ring_at: [Vec3; TRUNK_NODES] = std::array::from_fn(|i| centre(hip, rings[i]));

    let hind_radius = |share: f32| frame.leg * sturdy * share;
    let arm_radius = |share: f32| frame.arm * sturdy * share;

    // The hind leg, `+X` side: hip, knee, ankle, pad, toes.
    let (hind, hind_inside) = match posture {
        Posture::Perch => {
            let (knee, foot, ankle) = perch_leg(hip);
            let hip_at = Vec3::new(hip_x, hip.y, hip.z);
            (
                [
                    (hip_at, hind_radius(0.9)),
                    (splay(knee, ring_at[0]), hind_radius(0.9)),
                    (ankle, hind_radius(1.0)),
                    (foot, pad_r),
                    (foot + Vec3::Z * toe, toe_r),
                ],
                2,
            )
        }
        Posture::Crawl => {
            let hip_at = splay(
                Vec3::new(hip_x, hip.y - 0.25 * rings[0].depth, hip.z),
                ring_at[0],
            );
            let foot = Vec3::new(hip_x + CRAWL_FOOT.x, pad_y, hip.z + CRAWL_FOOT.y);
            let ankle = foot + Vec3::new(0.0, metatarsus, -0.02);
            let knee = bend(
                hip_at,
                ankle,
                femur,
                tibia,
                Vec3::new(0.6, 0.8, -0.6).normalize(),
            );
            let knee = beyond(knee, hip_at, ring_at[0], BRIDGE * cage_radius(rings[0]));
            (
                [
                    (hip_at, hind_radius(1.4)),
                    (knee, hind_radius(1.2)),
                    (ankle, hind_radius(0.8)),
                    (foot, pad_r),
                    // The foot trails out and back from its pad, its toes
                    // behind it, with the leg it ends (S22's rotated limb):
                    // the gait lays them on the ground through a stride
                    // (#424; W1 and W3 pointed it forward and out, which
                    // the toe-off rolled 49 to 58 mm into the ground).
                    (foot + CRAWL_TOES.normalize() * toe, toe_r),
                ],
                1,
            )
        }
        Posture::Stand => {
            let hip_at = splay(
                Vec3::new(hip_x, hip.y - 0.25 * rings[0].depth, hip.z),
                ring_at[0],
            );
            let foot = Vec3::new(pelvis_w * 0.8, pad_y, hip.z - 0.05);
            let ankle = foot + Vec3::new(0.0, metatarsus, -0.04);
            let knee = bend(hip_at, ankle, femur, tibia, Vec3::Z);
            let knee = beyond(knee, hip_at, ring_at[0], BRIDGE * cage_radius(rings[0]));
            (
                [
                    (hip_at, hind_radius(1.4)),
                    (Vec3::new(foot.x, knee.y, knee.z), hind_radius(1.3)),
                    (Vec3::new(foot.x, ankle.y, ankle.z), hind_radius(0.75)),
                    (foot, pad_r),
                    (foot + Vec3::Z * toe, toe_r),
                ],
                1,
            )
        }
    };

    // The fore limb, `+X` side: the blade's top, the shoulder, the elbow, the
    // wrist, the hand's two nodes.
    let shoulder = Vec3::new(shoulder_x, shoulder_on_line.y, shoulder_on_line.z)
        + up * (rings[TRUNK_NODES - 1].lift
            + match posture {
                Posture::Perch => 0.3,
                Posture::Crawl | Posture::Stand => -0.3,
            } * rings[TRUNK_NODES - 1].depth);
    let blade = shoulder - along * frame.scapula * 0.85 + up * frame.scapula * 0.35;
    let blade = Vec3::new(shoulder_x * 0.6, blade.y, blade.z);
    let (fore, fore_inside) = match posture {
        // **Folded along the flank, as a Z** (research-10 fact 3, 4.1): the
        // humerus back along the flank, the forearm forward under it, the
        // hand back again over the tail, at the fold's measured angles - the
        // elbow's interior 31 degrees and the wrist's 45 (S5's crown birds,
        // 31 and 52; 20 to 50 in life). The shoulder is inside the breast;
        // the forearm and the hand lie in one plane just outside the widest
        // ring, where the folded flight feathers the wing part roots on them
        // (#421) lie on the flank. Every node is a marker: the feathers are
        // what shows of the wing.
        Posture::Perch => {
            // The fold's plane leans in over the back by `FOLD_LEAN`, and stands
            // at the trunk's support distance in that direction - the furthest
            // any ring's section reaches along its normal - so the folded wing
            // lies on the upper flank and the back rather than standing off
            // the side as a plate (#421, the owner's look).
            let (lean_sin, lean_cos) = (FOLD_LEAN.det_sin(), FOLD_LEAN.det_cos());
            let normal = Vec3::X * lean_cos + up * lean_sin;
            let reach_of = |ring: &Ring| {
                ring.lift * lean_sin
                    + (ring.width * ring.width * lean_cos * lean_cos
                        + ring.depth * ring.depth * lean_sin * lean_sin)
                        .sqrt()
            };
            let plane = rings[1..last].iter().map(reach_of).fold(0.0f32, f32::max) + FOLD_MARGIN;
            let shoulder = Vec3::new(forechest_w * 0.6, shoulder.y, shoulder.z);
            // A direction in the fold's plane, `phi` from straight back toward
            // the back's own up, leaning in with the plane.
            let dorsal = up * lean_cos - Vec3::X * lean_sin;
            let fold = |phi: f32| -along * phi.det_cos() + dorsal * phi.det_sin();
            let out = plane - (shoulder - hip).dot(normal);
            let elbow = shoulder
                + normal * out
                + fold(FOLD_HUMERUS) * (humerus * humerus - out * out).max(0.0).sqrt();
            let forearm_at = FOLD_HUMERUS - (std::f32::consts::PI - FOLD_ELBOW);
            let wrist = elbow + fold(forearm_at) * forearm;
            let hand_at = forearm_at + (std::f32::consts::PI - FOLD_WRIST);
            let palm = wrist + fold(hand_at) * hand;
            let tip = palm + fold(hand_at - 0.08) * digit;
            (
                [
                    (blade, arm_radius(0.6)),
                    (shoulder, arm_radius(0.6)),
                    (elbow, arm_radius(0.5)),
                    (wrist, arm_radius(0.45)),
                    (palm, arm_radius(0.4)),
                    (tip, arm_radius(0.3)),
                ],
                FORE_NODES,
            )
        }
        // **A crawling bat's wing is its foreleg**: the elbow folded up and
        // out over the back, the forearm down to the wrist planted ahead,
        // the thumb forward of it. The digits folded along the forearm are
        // the wing part's (#421).
        Posture::Crawl => {
            let shoulder = splay(shoulder, ring_at[TRUNK_NODES - 1]);
            let wrist = Vec3::new(
                shoulder.x + CRAWL_WRIST.x,
                pad_y + hand,
                shoulder.z + CRAWL_WRIST.y,
            );
            let elbow = bend(
                shoulder,
                wrist,
                humerus,
                forearm,
                Vec3::new(0.35, 1.0, -0.7).normalize(),
            );
            let pad = Vec3::new(wrist.x, pad_y, wrist.z + 0.02);
            (
                [
                    (blade, arm_radius(1.0)),
                    (shoulder, arm_radius(1.3)),
                    (elbow, arm_radius(1.0)),
                    (wrist, arm_radius(0.75)),
                    (pad, pad_r),
                    (pad + Vec3::new(0.15, 0.0, 1.0).normalize() * digit, toe_r),
                ],
                2,
            )
        }
        // An ordinary foreleg: the wrist planted under the shoulder, the
        // elbow bent back between them, the paw ahead of the wrist.
        Posture::Stand => {
            let shoulder = splay(shoulder, ring_at[TRUNK_NODES - 1]);
            let x = forechest_w * 0.8;
            let wrist = Vec3::new(x, pad_y + hand, shoulder.z - STAND_WRIST_BEHIND * humerus);
            let pad = Vec3::new(x, pad_y, wrist.z + 0.03);
            let elbow = bend(shoulder, wrist, humerus, forearm, Vec3::NEG_Z);
            let elbow = Vec3::new(x.max(elbow.x), elbow.y, elbow.z);
            let elbow = beyond(
                elbow,
                shoulder,
                ring_at[TRUNK_NODES - 1],
                BRIDGE * cage_radius(rings[TRUNK_NODES - 1]),
            );
            (
                [
                    (blade, arm_radius(1.0)),
                    (shoulder, arm_radius(1.3)),
                    (elbow, arm_radius(1.0)),
                    (wrist, arm_radius(0.6)),
                    (pad, pad_r),
                    (pad + Vec3::Z * digit, toe_r),
                ],
                2,
            )
        }
    };

    // The trunk's nodes, with their sections as the cage reads them: the
    // ring frame's first axis across the body, its second out of the back.
    let trunk: [(Vec3, f32, Vec2); TRUNK_NODES] = std::array::from_fn(|i| {
        let (half_width, half_depth) = (rings[i].width / SKIN, rings[i].depth / SKIN);
        let radius = half_depth.max(half_width);
        (
            ring_at[i],
            radius,
            Vec2::new(half_width, half_depth) / radius,
        )
    });
    let withers = ring_at[TRUNK_NODES - 1];

    // **The neck and the head.** The neck leaves the top of the forechest:
    // nearly straight up on a perching bird, whose head sits on its body,
    // forward and up on a bat and a glider. The head chain hangs off it as a
    // beast's does, level-faced.
    let rise: f32 = match posture {
        Posture::Perch => 1.5,
        Posture::Crawl => 0.45,
        Posture::Stand => 0.55,
    };
    let neck_r = frame.neck_r * trunk_girth.sqrt() * (1.0 + 0.2 * young.max(0.0));
    // A perching bird's head sits on its body (research-10, the owl: no
    // visible neck), so its neck leaves from inside the top of the breast
    // and the head chain's balls overlap.
    let (neck_from, spacing) = match posture {
        Posture::Perch => (withers + up * rings[TRUNK_NODES - 1].depth * 0.1, 0.6),
        Posture::Crawl => (withers + up * rings[TRUNK_NODES - 1].depth * 0.3, 0.9),
        Posture::Stand => (withers + up * rings[TRUNK_NODES - 1].depth * 0.45, 1.05),
    };
    // Floored so the neck's bone is long enough for its socket on the
    // forechest's hull: **a cage wall, not anatomy**, the beast's floor in
    // the forechest's own radii.
    let rising = Vec3::new(0.0, rise.det_sin(), rise.det_cos());
    let neck_reach = if posture == Posture::Perch {
        neck_reach
    } else {
        // The reach that puts the neck joint `NECK_BONE` forechest radii
        // from the forechest's node, given where along its top it leaves.
        let off = neck_from - withers;
        let bone = NECK_BONE * trunk[TRUNK_NODES - 1].1;
        let along = off.dot(rising);
        let least = -along
            + (along * along - off.length_squared() + bone * bone)
                .max(0.0)
                .sqrt();
        neck_reach.max(least)
    };
    let neck_at = neck_from + rising * neck_reach;
    let level_rise = 0.70f32.det_atan2(0.62);
    let occiput_r = head_r * 0.88;
    let occiput_at = neck_at
        + crate::det::Rot::x(level_rise - rise)
            * Vec3::new(0.0, 0.70, 0.71)
            * (neck_r + occiput_r)
            * spacing;
    let head_at = occiput_at
        + Vec3::new(0.0, 0.10, 1.0).normalize()
            * head_r
            * if posture == Posture::Perch {
                0.55
            } else {
                0.85
            };
    let face = frame.face;
    let asked = super::winged_head::Asked::of(params, young);
    let snout = ((1.30 + 1.12 * face.muzzle_length) * (1.0 - 0.30 * appeal)).max(0.15);
    let ahead = head_r * (0.60 + snout) / 1.90;
    let muzzle_r = head_r * 0.58;
    let muzzle_at = head_at + Vec3::new(0.0, -head_r * 0.30, ahead * 1.15);
    let nose_r = head_r * 0.40;
    let nose_at = muzzle_at + Vec3::new(0.0, -head_r * 0.10, ahead * 0.75);
    let proxy = QuadrupedParams {
        muzzle_length: face.muzzle_length,
        muzzle_breadth: face.muzzle_breadth,
        stop: face.stop,
        eye_set: face.eye_set + asked.eye_set_shift(),
        ear_size: face.ear_size,
        ear_point: face.ear_point,
        ear_carriage: face.ear_carriage,
        // A beak grows none (#422).
        whiskers: if asked.beaked() { -1.5 } else { face.whiskers },
        nose: match lineage.leading() {
            WingedFamily::Bird => Nose::Soft,
            WingedFamily::Bat | WingedFamily::Glider => Nose::Leather,
        },
        ..QuadrupedParams::default()
    };

    // The tail, from the rump along the trunk's line backward, each node
    // turned by its bend from the one before: a bird's held along its body,
    // a bat's nub, a glider's long tail falling away behind it.
    let tail_r = frame.tail_r * trunk_girth.sqrt();
    // Set off the pelvis far enough for its socket to clear the legs': the
    // beast's rule, a bird's lower, under its rump.
    let pelvis_r = trunk[0].1;
    let set = ring_at[0]
        + match posture {
            // A bird's is markers (`Built::tail_inside`): its stub stays near
            // the rump, where the fan it will carry roots.
            Posture::Perch => up * 0.3 * pelvis_r - along * 0.8 * pelvis_r,
            Posture::Crawl | Posture::Stand => up * 0.9 * pelvis_r - along * 1.4 * pelvis_r,
        };
    let bends: [f32; TAIL_NODES] = match posture {
        Posture::Perch => [0.0, 0.25, 0.2, 0.1, 0.05],
        Posture::Crawl => [0.0, -0.1, -0.1, -0.1, -0.1],
        Posture::Stand => [0.35, -0.1, -0.2, -0.15, -0.1],
    };
    let lay = |length: f32| {
        let mut tail = [(set, tail_r); TAIL_NODES];
        let mut at = set;
        let mut heading = -pitch;
        for (index, bend) in bends.into_iter().enumerate() {
            heading += bend;
            if index > 0 {
                let step = (length / (TAIL_NODES - 1) as f32).max(tail[index - 1].1 * 2.2);
                at += Vec3::new(0.0, heading.det_sin(), -heading.det_cos()) * step;
            }
            let taper = (1.0 - frame.taper * index as f32).max(0.3);
            tail[index] = (at, (tail_r * taper).max(0.012));
        }
        tail
    };
    let clear = 0.04;
    let mut tail = lay(tail_len);
    let fit = tail
        .iter()
        .filter(|&&(at, radius)| at.y - radius < clear)
        .map(|&(at, radius)| ((set.y - radius - clear) / (set.y - at.y).max(1e-4)).max(0.0))
        .fold(1.0f32, f32::min);
    if fit < 1.0 {
        tail = lay(tail_len * fit);
    }

    // Every position and radius so far is in trunks: to metres.
    let scale = |(at, radius): (Vec3, f32)| (at * t, radius * t);
    let chain = Chain {
        nod: Quat::IDENTITY,
        occiput: (occiput_at * t, occiput_r * t),
        cranium: (head_at * t, head_r * t),
        nose: (nose_at * t, nose_r * t),
        neck: neck_at * t,
        withers: withers * t,
        back: frame.skull_back * (1.0 - 0.5 * young.max(0.0)),
    };
    let mut head = head::head(&chain, &proxy, appeal, trunk_girth.sqrt());
    // A perching bird's head sits on its body, so the neck's run under it is
    // as wide as it is where it meets the skull, not a beast's throat:
    // without it an owl's head stood on a collar.
    if posture == Posture::Perch
        && let Some(nape) = &mut head.nape
    {
        nape.throat = nape.seat;
    }
    // And what the family makes of the beast's head (#422): an owl's disc, the
    // eyes and the ears sized, and a bird's beak.
    super::winged_head::dress(
        &mut head,
        &proxy,
        appeal,
        super::winged_head::Sizes {
            eyes: face.eye_scale,
            proud: face.proud,
            ears: face.ear_scale,
        },
        &asked,
    );
    let dims = Dimensions {
        trunk: trunk.map(|(at, radius, section)| (at * t, radius * t, section)),
        neck_at: neck_at * t,
        neck_r: neck_r * t,
        neck_section: Vec2::new(0.92, 1.0),
        occiput_at: occiput_at * t,
        occiput_r: occiput_r * t,
        head_at: head_at * t,
        head_r: head_r * t,
        muzzle_at: muzzle_at * t,
        muzzle_r: muzzle_r * t,
        nose_at: nose_at * t,
        nose_r: nose_r * t,
        head,
        tail: tail.map(scale),
        hind: hind.map(scale),
        fore: fore.map(scale),
        stifle_section: Vec2::ONE,
    };
    Built {
        dims,
        hind_inside,
        fore_inside,
        tail_inside: match posture {
            Posture::Perch => TAIL_NODES,
            Posture::Crawl | Posture::Stand => 0,
        },
    }
}

/// Where a two-bone limb from `root` to `tip` bends, toward `pole`: the bones
/// `a` and `b` long, a reach past both straightened and one shorter than their
/// difference folded shut. No trigonometry: the bend's cosine by the law of
/// cosines, its sine by the square root.
fn bend(root: Vec3, tip: Vec3, a: f32, b: f32, pole: Vec3) -> Vec3 {
    let span = tip - root;
    let length = span.length().max(1e-6);
    let along = span / length;
    let reach = length.clamp((a - b).abs() + 1e-4, (a + b) * 0.999);
    let across = (pole - along * pole.dot(along)).normalize_or(Vec3::Y);
    let cos = ((a * a + reach * reach - b * b) / (2.0 * a * reach)).clamp(-1.0, 1.0);
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    root + (along * cos + across * sin) * a
}

/// `point`, carried on along the line from `from` until it stands at least
/// `least` from `centre`: **a cage wall**, the first joint a bridged leg
/// sweeps to held far enough from its girdle for the bridge's socket.
fn beyond(point: Vec3, from: Vec3, centre: Vec3, least: f32) -> Vec3 {
    let off = point - centre;
    if off.length() >= least {
        return point;
    }
    let dir = (point - from).normalize_or(Vec3::NEG_Y);
    // |off + dir t| = least, the root past `point`.
    let along = off.dot(dir);
    let t = -along
        + (along * along - off.length_squared() + least * least)
            .max(0.0)
            .sqrt();
    point + dir * t.max(0.0)
}

/// A joint inside the body a leg's socket is aimed at, held out from its
/// girdle at least 0.65 of its offset from it: **a cage wall, not anatomy**,
/// the beast's (`derive::quadruped`'s `splay`), so two sockets open apart.
fn splay(joint: Vec3, girdle: Vec3) -> Vec3 {
    let off = Vec2::new(joint.y - girdle.y, joint.z - girdle.z).length();
    Vec3::new(joint.x.max(0.65 * off), joint.y, joint.z)
}
