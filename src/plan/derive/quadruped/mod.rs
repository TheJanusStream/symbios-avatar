//! Every dimension the four-legged cage is built from.
//!
//! Nineteen semantic axes, the head's stored parts and three composites in,
//! one flat [`Dimensions`] out, and [`crate::plan::quadruped`] does nothing
//! afterwards but hang nodes on it and stamp the [`head`] it declares on the
//! skeleton. See [`super`] for why this layer exists at all and what a
//! refactor of it has to prove.
//!
//! **This plan's girth is the composites', spent at sites of its own** (#178):
//! `mass` says how much body there is, `bodyFat` how it is spent, age what the
//! legs lose, and legginess slims all of it. See [`Girth`]. On the trunk they
//! move the family's own body (#374): every ring is wider and the barrel
//! deeper for a heavier body under the same back, a fatter one's most at the
//! belly, whose underline drops; the girdles' depth is their bone's.

mod head;

use glam::{Quat, Vec2, Vec3};

use crate::det::DetMath;
use crate::plan::{BeastHead, Composites, DEFAULT_BODY_FAT, Family, QuadrupedParams};

/// A neck's cross-section, as `(lateral, vertical)` multiples of its radius:
/// rounder than the body it leaves.
///
/// The ring frame is transported from a world-up reference, so `x` is lateral
/// on any body and, on a quadruped's fore-and-aft spine, `y` points up. As on
/// the humanoid, every axis that moves moves *downward*: a socket's clearance
/// demand is its largest half-extent, so shrinking is free and growing costs
/// a joint its room.
const NECK_SECTION: Vec2 = Vec2::new(0.92, 1.0);

/// How far a dog's skull has gone from a cat's round one toward a horse's poll
/// and jowl (#388): between them, with a marked occiput and a modest jowl.
/// Provenance: **tuned by render**.
const DOG_SKULL_BACK: f32 = 0.60;

/// The withers height a record's back height builds, as a multiple of it.
///
/// Provenance: measured off the P5 body by `examples/quadprobe` (#374): its
/// trunk's top stood 1.18 to 1.26 back heights up, the girdle node's own
/// radius above the line the field names. Kept, so the rebuilt body stands
/// as tall in a world as the one the owner judged: a record's height is a
/// size, and this leaves every size where it was.
const WITHERS: f32 = 1.2;

/// The share of a trunk ring's half-extent the subdivided skin keeps, so the
/// cage is drawn this much wider than the skin it is meant to give. Measured
/// by `examples/quadprobe`.
pub(crate) const SKIN: f32 = 0.95;

/// How far under the points of the shoulders the underline of the chest's
/// front - the heart girth's ring and the forechest's - is held at least, in
/// withers heights. **A cage wall, not anatomy**: it puts the forechest's
/// hull at the shoulders' height, where [`PECTORAL`] can close the chest in
/// front of them, and the heart girth's under it, without which the skin
/// between the two rose off the shoulders anyway (`examples/quadprobe`).
/// Under every dog's and cat's own rings at their centre, so it deepens a
/// horse's forechest, a foal's and a slight horse's most.
const BRISKET: f32 = 0.04;

/// How far the forechest's hull is held to reach past the points of the
/// shoulders, at their height, in withers heights: the pectorals in front of
/// the joints, so a shoulder's socket and the leg's IK root are under the
/// skin (#374, #375). Measured against the built skin by `examples/quadprobe`
/// and held by `tests/plan.rs` over a sweep of rolls.
const PECTORAL: f32 = 0.07;

/// How many cross-sections the trunk is built from, from the pelvis forward:
/// the pelvis, the loin, the last rib, the middle of the ribcage, the heart
/// girth behind the elbows, and the forechest the forelegs hang from (#375).
pub(crate) const TRUNK_NODES: usize = 6;

/// Where each trunk ring stands between the forechest and the pelvis, as a
/// share of the run from the one to the other, pelvis first.
const TRUNK_SHARES: [f32; TRUNK_NODES] = [1.0, 0.73, 0.555, 0.38, 0.205, 0.0];

/// The shortest run from the forechest's ring to the pelvis's, in multiples
/// of the trunk's largest ring radius. **A cage wall, not anatomy**: the
/// ribs between the girdles need bones long enough to open their sockets on
/// the girdles' hulls. Below every family's own run at its centre, so it
/// moves only a short, deep body.
pub(crate) const SPAN_FLOOR: f32 = 2.85;

/// One trunk cross-section as the skin should measure it, in withers heights:
/// the top of the back over it, the underline under it, and its half-width.
#[derive(Clone, Copy, Debug)]
struct Ring {
    top: f32,
    bottom: f32,
    half_width: f32,
}

impl Ring {
    const fn at(top: f32, bottom: f32, half_width: f32) -> Self {
        Self {
            top,
            bottom,
            half_width,
        }
    }
}

/// The body a family stands in, in withers heights (#374, #375).
///
/// **Measured where the dossier measured it** - docs/research/research-07,
/// whose rows carry their confidence - and **chosen** where it is silent or
/// fights the cage; each family says which below. The standing angles are not
/// here: the stance is solved from these lengths and where the paws stand
/// ([`Dimensions::of`]), and the angles that come out are checks against the
/// dossier's, as its note 1 asks.
///
/// A cross blends its two families' frames by its blend ([`Frame::for_params`]),
/// in these normalised units, as the dossier's note 11 says to.
#[derive(Clone, Copy, Debug)]
struct Frame {
    /// Point of the shoulder to point of the buttock.
    length: f32,
    /// The trunk's rings, pelvis first: see [`TRUNK_NODES`].
    rings: [Ring; TRUNK_NODES],
    /// The shoulder blade, its top to the shoulder joint.
    scapula: f32,
    /// The shoulder blade's slope from horizontal, in radians.
    incline: f32,
    /// How high the top of the shoulder blade sits.
    scapula_top: f32,
    /// The upper arm, shoulder to elbow.
    humerus: f32,
    /// The forearm, elbow to carpus.
    radius: f32,
    /// The carpus to the pad: the metacarpus and the digits it stands on.
    metacarpus: f32,
    /// The metacarpus' slope from vertical, in radians: a dog's pastern.
    pastern: f32,
    /// How high the hip joint sits.
    hip: f32,
    /// How far the hip joint sits ahead of the point of the buttock.
    hip_ahead: f32,
    /// The thigh, hip to stifle.
    femur: f32,
    /// The second thigh, stifle to hock.
    tibia: f32,
    /// The hock to the pad: the metatarsus and the digits it stands on.
    metatarsus: f32,
    /// How far the hind pad stands behind the hip joint.
    paw_behind: f32,
    /// The legs' girth against a dog's.
    limb: f32,
    /// The thigh's girth at the stifle, against the leg's own: the hind
    /// leg's muscle, which the cage has no node for between the croup and
    /// the stifle, so the stifle's ring carries it down the thigh and on into
    /// the gaskin. **Chosen by render** (#374): the dossier measures masses
    /// (S8: a thigh 4.5% of a dog's body, its whole foreleg 4.5%), not
    /// outlines.
    thigh: f32,
    /// The stifle's cross-section, `(across, fore and aft)` multiples of its
    /// radius: deeper from the side than across, as a thigh is.
    thigh_section: Vec2,
    /// The neck's girth against P1's: research-07 note 4 puts a neck at
    /// 0.45 to 0.5 of the chest's girth (S9, the one dog measured), where
    /// P1's measured 0.27 to 0.43 of it.
    neck_girth: f32,
    /// The neck's cross-section: a horse's is deep and narrow, crested.
    neck_section: Vec2,
    /// How far the back of the skull has gone from round to a poll and a flat
    /// jowl (#388): nothing on a cat, whose braincase is the round end of the
    /// range, and all of it on a horse.
    skull_back: f32,
    /// How far the neck reaches against P1's. A horse's is 0.53 withers
    /// heights from the withers to the poll (S3, M); a dog's and a cat's are
    /// chosen by render.
    neck: f32,
}

impl Frame {
    /// One family's frame.
    fn of(family: Family) -> Self {
        match family {
            // Bones: Lasota-Moskalewska and Harcourt (S5, M); the shoulder
            // blade's slope and the croup are background (L). Trunk: chest
            // depth 0.45 to 0.50 (S11, S12), the underline at the elbow, a
            // Labrador's length (S11). **The girth is chosen between two
            // rows that disagree**: the heart girth 1.07 to 1.15 (S9, S7) and
            // a chest that deep make a depth to width near 2:1, where the
            // dossier's Labrador is 1.3 to 1.5:1; this takes about 1.8:1,
            // and the loin is 0.8 of the ribcage's girth, between a
            // Labrador's and a sighthound's.
            Family::Dog => Self {
                length: 0.99,
                rings: [
                    Ring::at(1.0, 0.64, 0.125),
                    Ring::at(0.995, 0.62, 0.125),
                    Ring::at(1.0, 0.565, 0.135),
                    Ring::at(1.0, 0.515, 0.14),
                    Ring::at(0.995, 0.50, 0.125),
                    Ring::at(1.0, 0.58, 0.105),
                ],
                scapula: 0.25,
                incline: 60f32.to_radians(),
                scapula_top: 0.90,
                humerus: 0.30,
                radius: 0.31,
                metacarpus: 0.12,
                pastern: 20f32.to_radians(),
                hip: 0.80,
                hip_ahead: 0.12,
                femur: 0.33,
                tibia: 0.34,
                metatarsus: 0.17,
                paw_behind: 0.20,
                limb: 1.0,
                thigh: 1.3,
                thigh_section: Vec2::new(0.8, 1.25),
                neck_girth: 1.6,
                neck_section: NECK_SECTION,
                skull_back: DOG_SKULL_BACK,
                neck: 1.0,
            },
            // Bones: Poradowski's radiographs (S2, H), the shoulder blade and
            // the metapodials background (L). The hip at 0.88 (S21). The
            // trunk is chosen round: girth 1.3 to 1.5 at a depth to width
            // near 1.15:1 (S14, derived), and a loin rounder than the chest
            // (1.04 to 1.13, S14) with the pouch hanging under it.
            // **Chosen compact, against the dossier's length and stance**
            // (#374, by render): its 1.3 to 1.5 (S25, L, its definition
            // unconfirmed) read as a ferret, and the paw 0.22 behind the hip
            // trailed the hind leg out behind the croup. The paw stands near
            // under the hip instead, as a dog's does on the radiograph table
            // (S6); on the radiographs' long femur and tibia, trimmed a
            // little toward joint centres, that folds the stifle to about 96
            // degrees against the background's 110 to 125 (L) - a crouched
            // cat. Shorter thigh bones opened it, and folded the low corner
            // of the envelope's stifle so close under the croup that its
            // socket no longer fitted on the bone (`tests/budget.rs`).
            Family::Cat => Self {
                length: 1.0,
                rings: [
                    Ring::at(1.05, 0.62, 0.17),
                    Ring::at(1.03, 0.48, 0.235),
                    Ring::at(1.02, 0.50, 0.22),
                    Ring::at(1.005, 0.51, 0.21),
                    Ring::at(0.995, 0.52, 0.19),
                    Ring::at(1.0, 0.60, 0.14),
                ],
                scapula: 0.30,
                incline: 62f32.to_radians(),
                scapula_top: 0.97,
                humerus: 0.387,
                radius: 0.375,
                metacarpus: 0.13,
                pastern: 20f32.to_radians(),
                hip: 0.86,
                hip_ahead: 0.12,
                femur: 0.40,
                tibia: 0.42,
                metatarsus: 0.22,
                paw_behind: 0.08,
                limb: 1.0,
                thigh: 1.25,
                thigh_section: Vec2::new(0.8, 1.25),
                neck_girth: 1.9,
                neck_section: NECK_SECTION,
                skull_back: 0.0,
                neck: 0.85,
            },
            // Matsuura's landmarks (S4, H) throughout: segment lengths, the
            // shoulder blade at 60, chest depth 0.48, heart girth 1.15 to
            // 1.22 at 2:1, length 1.02. The barrel widens behind the girth to
            // its widest at the last ribs (bg, L) and the belly line rises
            // gently to the stifle, with no tuck-up. The cannon and the
            // pastern are one bone here: the pad node is the hoof.
            Family::Horse => Self {
                length: 1.06,
                rings: [
                    Ring::at(1.03, 0.64, 0.15),
                    Ring::at(0.99, 0.60, 0.16),
                    Ring::at(0.99, 0.555, 0.17),
                    Ring::at(0.995, 0.53, 0.155),
                    Ring::at(0.99, 0.52, 0.12),
                    Ring::at(1.0, 0.60, 0.10),
                ],
                scapula: 0.32,
                incline: 60f32.to_radians(),
                scapula_top: 0.90,
                humerus: 0.19,
                radius: 0.24,
                metacarpus: 0.21,
                pastern: 10f32.to_radians(),
                hip: 0.75,
                hip_ahead: 0.14,
                femur: 0.22,
                tibia: 0.33,
                metatarsus: 0.25,
                paw_behind: 0.14,
                limb: 0.85,
                thigh: 1.7,
                thigh_section: Vec2::new(0.75, 1.35),
                neck_girth: 1.3,
                neck_section: Vec2::new(0.72, 1.0),
                skull_back: 1.0,
                neck: 1.2,
            },
        }
    }

    /// The frame `params` stands in: its family's, blended toward its
    /// hybrid's by its blend.
    fn for_params(params: &QuadrupedParams) -> Self {
        let own = Self::of(params.family);
        let Some(other) = params.hybrid.filter(|&other| other != params.family) else {
            return own;
        };
        let other = Self::of(other);
        let t = params.blend.clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            length: mix(own.length, other.length),
            rings: std::array::from_fn(|i| {
                let (a, b) = (own.rings[i], other.rings[i]);
                Ring::at(
                    mix(a.top, b.top),
                    mix(a.bottom, b.bottom),
                    mix(a.half_width, b.half_width),
                )
            }),
            scapula: mix(own.scapula, other.scapula),
            incline: mix(own.incline, other.incline),
            scapula_top: mix(own.scapula_top, other.scapula_top),
            humerus: mix(own.humerus, other.humerus),
            radius: mix(own.radius, other.radius),
            metacarpus: mix(own.metacarpus, other.metacarpus),
            pastern: mix(own.pastern, other.pastern),
            hip: mix(own.hip, other.hip),
            hip_ahead: mix(own.hip_ahead, other.hip_ahead),
            femur: mix(own.femur, other.femur),
            tibia: mix(own.tibia, other.tibia),
            metatarsus: mix(own.metatarsus, other.metatarsus),
            paw_behind: mix(own.paw_behind, other.paw_behind),
            limb: mix(own.limb, other.limb),
            thigh: mix(own.thigh, other.thigh),
            thigh_section: own.thigh_section.lerp(other.thigh_section, t),
            neck_girth: mix(own.neck_girth, other.neck_girth),
            neck_section: own.neck_section.lerp(other.neck_section, t),
            skull_back: mix(own.skull_back, other.skull_back),
            neck: mix(own.neck, other.neck),
        }
    }
}

/// What `appeal` makes of a family's body (#372, the owner's call of
/// 2026-10-28), as the share each measure gains at the top of the scale's
/// young end: `appeal` 1.5, where a body is all the way to the young animal
/// the family reads cute as. Below zero each runs the other way, by half as
/// far at most: `young` is `appeal / 1.5` held to -0.5, which `appeal` -0.75
/// reaches. **But not the ribcage's width**, which below zero narrows by a
/// tenth of `young` (at most 5%) rather than by [`Schema::round`]: a body
/// past the adult's is not a thinner one.
///
/// **A dog or a cat follows the puppy, rounder rather than longer.** A
/// 45-day puppy against its withers (research-07 summary fact 10, S7): head
/// 0.61 against an adult's 0.39, chest girth 1.58 against 1.15, body 1.44
/// against 1.04, legs at the adult's 0.61. The head takes the whole ratio and
/// the girth most of it; the length takes little of its own, because a
/// barrel grown as long as a puppy's under P1's long neck read as a
/// dachshund and a ferret (the probe, #374). The neck shortens and thickens.
///
/// **A horse follows a foal.** A foal stands on near-adult cannons at sixty
/// per cent of its adult height, its humerus and femur half an adult's
/// (research-07, foal growth, L-M): a big head on a short body and long legs.
/// The cannons lengthen, the upper arm and the thigh shorten, the barrel
/// shortens and does not round; how far is **chosen by render**, the dossier
/// giving no foal figure for the trunk.
///
/// A cross blends its two families' schemas by its blend, as the
/// [`Frame`] does.
#[derive(Clone, Copy, Debug)]
struct Schema {
    /// The head's radius.
    head: f32,
    /// The trunk's length.
    length: f32,
    /// The ribcage's width, toward round.
    round: f32,
    /// How far the barrel's underline sinks, in withers heights.
    sink: f32,
    /// How much shorter the neck reaches.
    neck: f32,
    /// How much thicker the neck is.
    neck_girth: f32,
    /// The metapodials, a horse's cannons.
    legs: f32,
    /// The upper arm and the thigh, inside the trunk: a foal's are half an
    /// adult's at sixty per cent of its height. Lengthened below and kept
    /// above, a foal's leg folded at the stifle to 96 degrees.
    upper: f32,
    /// The legs' girth.
    sturdy: f32,
    /// The pads' size.
    pads: f32,
}

impl Schema {
    /// One family's.
    fn of(family: Family) -> Self {
        match family {
            Family::Dog | Family::Cat => Self {
                head: 0.56,
                length: 0.10,
                round: 0.90,
                sink: 0.05,
                neck: 0.30,
                neck_girth: 0.30,
                legs: 0.0,
                upper: 0.0,
                sturdy: 0.18,
                pads: 0.375,
            },
            Family::Horse => Self {
                head: 0.40,
                length: -0.12,
                round: 0.0,
                sink: 0.0,
                neck: 0.20,
                neck_girth: 0.10,
                legs: 0.35,
                upper: -0.20,
                sturdy: 0.0,
                pads: 0.0,
            },
        }
    }

    /// The schema `params` is drawn by: its family's, blended toward its
    /// hybrid's by its blend.
    fn for_params(params: &QuadrupedParams) -> Self {
        let own = Self::of(params.family);
        let Some(other) = params.hybrid.filter(|&other| other != params.family) else {
            return own;
        };
        let other = Self::of(other);
        let t = params.blend.clamp(0.0, 1.0);
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            head: mix(own.head, other.head),
            length: mix(own.length, other.length),
            round: mix(own.round, other.round),
            sink: mix(own.sink, other.sink),
            neck: mix(own.neck, other.neck),
            neck_girth: mix(own.neck_girth, other.neck_girth),
            legs: mix(own.legs, other.legs),
            upper: mix(own.upper, other.upper),
            sturdy: mix(own.sturdy, other.sturdy),
            pads: mix(own.pads, other.pads),
        }
    }
}

/// Every number the four-legged cage is built from.
///
/// **Leg positions are half-bodies**, given for the `+X` side only, and the
/// assembler mirrors them — which is what keeps the animal symmetric by
/// construction rather than by two expressions agreeing.
///
/// **The trunk is six rings, each its own depth and width** (#375): the
/// pelvis, the loin, the last rib, the middle of the ribcage, the heart girth
/// and the forechest, the girdles at either end. The ribcage is the largest
/// mass and the forechest narrows between the elbows; how the loin compares
/// is the family's ([`Frame`]).
///
/// **The bones inside the trunk are markers** (#374). The shoulder blade and
/// the upper arm, the hip joint and the thigh lie inside an animal's
/// silhouette, and a socket for each on the girdle's hull would have to stand
/// outside it - which is what rooted the P1 legs beside the trunk, splayed
/// from the front, with the upper bones hanging under the body as tubes. So
/// the scapula's top, the shoulder and the hip are joints the rig keeps and
/// the cage skips: the cage sweeps each leg from its girdle straight to the
/// elbow or the stifle, where the leg leaves the body (`Skeleton`'s bridge).
///
/// **The stance is solved, not authored**: the lengths and the paw's place
/// go in and the joints come out, so a standing angle is a check against the
/// dossier and never an input (its note 1). See [`Dimensions::of`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dimensions {
    /// The trunk's nodes from the pelvis forward, each with its radius and
    /// its cross-section as `(lateral, vertical)` multiples of the radius:
    /// the first is the rear girdle, the pelvis, and the last the front
    /// one, the forechest under the withers.
    pub trunk: [(Vec3, f32, Vec2); TRUNK_NODES],
    /// Where the neck sits, risen from the withers.
    pub neck_at: Vec3,
    /// Radius of the neck node.
    pub neck_r: f32,
    /// Cross-section of the neck node.
    pub neck_section: Vec2,
    /// Where the back of the skull sits, between the neck and the cranium.
    pub occiput_at: Vec3,
    /// Radius of the back of the skull.
    pub occiput_r: f32,
    /// Where the cranium sits.
    pub head_at: Vec3,
    /// Radius of the cranium.
    pub head_r: f32,
    /// Where the muzzle sits, ahead of and below the cranium.
    pub muzzle_at: Vec3,
    /// Radius of the muzzle.
    pub muzzle_r: f32,
    /// Where the nose ends the muzzle.
    pub nose_at: Vec3,
    /// Radius of the nose.
    pub nose_r: f32,
    /// The head these nodes carry: the shape the build projects the head's
    /// surface onto, and seats the eyes against (#367). See [`head`].
    pub head: BeastHead,
    /// Each tail node from the croup outward, with its radius.
    pub tail: [(Vec3, f32); TAIL_NODES],
    /// The hind leg's nodes from the body down, on the `+X` side: the hip
    /// joint (a marker), the stifle, the hock, the pad and the toes.
    pub hind: [(Vec3, f32); HIND_NODES],
    /// The fore leg's nodes from the body down, on the `+X` side: the top of
    /// the shoulder blade and the shoulder joint (both markers), the elbow,
    /// the carpus, the pad and the toes.
    pub fore: [(Vec3, f32); FORE_NODES],
    /// The stifle's cross-section, as `(across, fore and aft)` multiples of
    /// its radius: the thigh's muscle, deeper from the side than across.
    pub stifle_section: Vec2,
}

/// How many nodes a tail carries: enough to bend, and to swing as a chain once
/// the springs reach it (#370).
pub(crate) const TAIL_NODES: usize = 5;

/// How many nodes a hind leg carries: two in the upper limb (the hip joint, a
/// marker, and the stifle), the hock in the lower, two in the extremity -
/// the humanoid's split, so `Rig::limb_chain` takes the same three joints on
/// both plans.
pub(crate) const HIND_NODES: usize = 5;

/// How many nodes a fore leg carries: [`HIND_NODES`]' and the shoulder
/// blade's top before them, a third upper-limb node (#374). `Rig::limb_chain`
/// takes the LAST three of the upper and lower limb, so it still hands the leg
/// IK the shoulder, the elbow and the carpus; a stride turns the blade with the
/// forearm (`anim::quadruped::stand`, #378), and at rest it rides the trunk.
pub(crate) const FORE_NODES: usize = 6;

/// How many of a leg's nodes, from the body down, are markers inside it:
/// the hip joint on the hind leg; the blade's top and the shoulder on the
/// fore.
pub(crate) const HIND_INSIDE: usize = 1;
/// See [`HIND_INSIDE`].
pub(crate) const FORE_INSIDE: usize = 2;

/// A tail's bends at `tailCarriage` 0, in radians above level from each node
/// to the next, the first unused: out and a little up from the croup, then
/// falling away in a sabre curve.
const SABRE: [f32; TAIL_NODES] = [0.15, 0.05, -0.25, -0.55, -0.80];
/// See [`SABRE`]: at `tailCarriage` -1, out of the croup and straight down.
const HANGING: [f32; TAIL_NODES] = [0.0, -0.55, -1.10, -1.35, -1.45];
/// See [`SABRE`]: at `tailCarriage` 1, raised and curled over the back.
const RAISED: [f32; TAIL_NODES] = [0.0, 0.85, 1.30, 1.85, 2.40];

/// How much more body there is than neutral at each kind of site (#178).
///
/// **The humanoid's model with this plan's own exponents.** `mass` spans a
/// ratio of volume per height ([`MASS_SPAN`], the humanoid's); `bodyFat`
/// splits it into a lean budget and a fat one ([`SOFT`]); and age takes a
/// share of the legs' lean budget ([`WASTING_SPAN`]) - the humanoid's
/// appendicular wasting, which is a mammal's before it is a person's. What is
/// NOT the humanoid's is where the budget lands: a human's exponents are
/// measured human allometry, and a quadruped's are a species question before
/// they are a body one. So each [`Site`] here names its own, by the pattern
/// the humanoid's measurements show - the fleshy sites gain fastest, the bony
/// ones slowest, the head not at all - and they are **chosen, not measured**.
///
/// **And legginess slims all of it**, the correlation this plan's girth has
/// always carried: at a given back height a leggy animal is a slender one and
/// a squat animal is a barrel, so the legs' length reduces girth rather than
/// changing the height. Letting the two move independently produces bodies
/// that read as mistakes.
///
/// Every factor is one on the neutral body - mass 0, `DEFAULT_BODY_FAT`, under
/// `AGE_PIVOT`, legs neutral - so the composites at their defaults build the
/// plan's own body unchanged.
#[derive(Clone, Copy, Debug)]
struct Girth {
    /// Lean volume per unit height, relative to neutral.
    lean: f32,
    /// Fat volume per unit height, relative to neutral.
    fat: f32,
    /// The legs' lean budget, after age has taken its share.
    wasting: f32,
    /// What legginess leaves of every girth.
    slim: f32,
}

/// Volume per unit height at `mass` +1, relative to neutral: the humanoid's
/// span, so one `mass` means one amount of body on both plans.
const MASS_SPAN: f32 = 1.49;

/// The tissue at a site that is not going anywhere, as a body-fat fraction:
/// the humanoid's, and for its reason - without it a lean body's belly is not
/// lean but absent.
const SOFT: f32 = 0.08;

/// What age takes out of the legs' lean volume by the top of `AGE_RANGE`: the
/// humanoid's sarcopenia figure, which is a mammal's.
const WASTING_SPAN: f32 = 0.22;

/// One site the girth budget is spent at: its exponent on volume per height,
/// and how much of its growth is fat rather than lean.
type Site = (f32, f32);

/// The belly, under the loin: the depot, where a heavy animal is round.
const BELLY: Site = (0.62, 0.60);
/// The croup: muscle with fat over it.
const CROUP: Site = (0.50, 0.35);
/// The chest: a ribcage, which grows slowest of the trunk.
const CHEST: Site = (0.45, 0.20);
/// The neck, where a fat cat's disappears.
const NECK: Site = (0.55, 0.30);
/// The thigh and the arm: the legs' muscle.
const THIGH: Site = (0.50, 0.30);
/// The second thigh and the forearm.
const SHANK: Site = (0.40, 0.15);
/// The hock and the carpus: bone and tendon.
const CANNON: Site = (0.20, 0.0);
/// The pad and the toes.
const PAD: Site = (0.15, 0.0);
/// The tail.
const TAIL: Site = (0.30, 0.15);
/// The cheeks (#367): soft tissue over a bony face, where a fat cat's face
/// goes round.
const CHEEK: Site = (0.30, 0.70);

impl Girth {
    /// Reads the composites and legginess into the budgets.
    fn of(params: &QuadrupedParams, composites: &Composites) -> Self {
        let bulk = MASS_SPAN.powf(composites.mass);
        let fat = composites.body_fat.clamp(0.0, 0.95);
        Self {
            lean: bulk,
            fat: bulk * (fat + SOFT) / (DEFAULT_BODY_FAT + SOFT),
            wasting: bulk * (1.0 - WASTING_SPAN * composites.ageing()),
            slim: 1.0 - 0.18 * params.leg_length,
        }
    }

    /// The factor at a site of the trunk, the neck or the tail.
    fn at(&self, site: Site) -> f32 {
        self.spent(self.lean, site)
    }

    /// The factor at a site of a leg: the budget age wastes.
    fn limb(&self, site: Site) -> f32 {
        self.spent(self.wasting, site)
    }

    /// One site's factor, given which lean budget it spends.
    ///
    /// Saturated for the exploration range (#160), as the girth this replaces
    /// was: radii scale with it while the plan's lengths do not, and the old
    /// pair sweep failed exactly where a slight body compounded long legs.
    fn spent(&self, lean: f32, (power, fat_share): Site) -> f32 {
        (lean.powf(power * (1.0 - fat_share)) * self.fat.powf(power * fat_share) * self.slim)
            .clamp(0.50, 1.80)
    }
}

impl Dimensions {
    /// Derives every dimension of one quadruped from its record's axes and
    /// its composites.
    ///
    /// **The frame axis is a humanoid one** and is not read: it is anchored on
    /// a measured male and female mannequin, and there is no such pair for a
    /// beast - a quadruped's dimorphism is a species question before it is a
    /// body question (#178). `mass`, `bodyFat` and age are read, through
    /// [`Girth`], and age also settles the back and lowers the head.
    ///
    /// **The body reads its family** (#374): the frame it stands in is the
    /// family's, a cross's blended ([`Frame`]), and the body's axes move it
    /// by how far each sits from the family's own centre, so a roll at its
    /// family's centre IS the family's frame. The head still reads the axes
    /// alone, as P2 built it.
    pub(crate) fn of(params: &QuadrupedParams, composites: &Composites) -> Self {
        Self::eased(params, composites, &AS_ASKED)
    }

    /// [`Self::of`] with each of the axes its girdles are built from only
    /// part of the way from its family's centre to what the record asks
    /// (#391): see [`Ease`]. At [`AS_ASKED`] it is the body as asked, bit for
    /// bit; at all zeros, its family's own frame under the record's height,
    /// head and tail.
    ///
    /// **The head reads the record as it is**: none of its nodes is a joint
    /// the cage has to hull, so nothing about it refuses a body, and a
    /// creator's head stays theirs - its size, its muzzle, its attitude and
    /// what `appeal` makes of it. See `QuadrupedParams::skeleton` for when a
    /// body is eased, and by how much.
    pub(crate) fn eased(asked: &QuadrupedParams, as_asked: &Composites, ease: &Ease) -> Self {
        let centre = crate::plan::quadruped::axis_centres(asked);
        let (params, composites) = &eased_axes(asked, as_asked, &centre, ease);
        let h = params.height;
        let wh = h * WITHERS;
        let girth = Girth::of(params, composites);
        let ageing = composites.ageing();
        let frame = Frame::for_params(params);
        // `appeal` (#366): the baby schema's proportions over the anatomy - a
        // bigger head and a shorter muzzle - read by every family in its own
        // terms, so a cute horse is still a horse. Saturated where the head
        // would swallow the neck. The head's is the record's own.
        let appeal = params.appeal.clamp(-1.5, 2.0);
        let head_appeal = asked.appeal.clamp(-1.5, 2.0);
        // And the body follows the family's young (#372, the owner's call):
        // a puppy's, a kitten's or a foal's ([`Schema`]). At 1.5 a body is all
        // the way there; below zero it leans the other way.
        let young = (appeal / 1.5).clamp(-0.5, 1.33);
        let head_young = (head_appeal / 1.5).clamp(-0.5, 1.33);
        let schema = Schema::for_params(params);
        let sturdy = 1.0 + schema.sturdy * young;
        let pads = 1.0 + schema.pads * young;

        // Each body axis as a step from its family's centre.
        let leggy = params.leg_length - centre.leg_length;
        let long = params.body_length - centre.body_length;
        let deep = params.chest_depth - centre.chest_depth;
        let toes = params.stance - centre.stance;

        // Legginess lengthens the leg below the body - the forearm, the second
        // thigh and the long bones of the foot - at a fixed withers height, so
        // the trunk over it gets shallower: a leggy animal is a slender one.
        // The bones inside the trunk keep their length, because they cannot
        // grow without passing each other: a blade grown with the leg hung
        // its shoulder below its own elbow.
        let reach = (1.0 + 0.10 * leggy).clamp(0.75, 1.3);
        // A foal's long legs are its cannons (`Schema::legs`): near their
        // adult length at birth, where every bone above them is short.
        let metapodial =
            (reach * (1.0 + 0.20 * toes).clamp(0.6, 1.5) * (1.0 + schema.legs * young))
                .clamp(0.45, 1.95);
        let pad_r = wh * 0.042 * girth.limb(PAD) * pads;
        let toe_r = wh * 0.035 * girth.limb(PAD) * pads;
        // The pad's centre: its radius above the ground, less what subdivision
        // takes off a small node, so the surface meets the floor.
        let pad_y = pad_r * 0.82;
        let toe_y = toe_r * 0.82;

        // **The foreleg, solved from the top down.** The blade hangs from its
        // top at its slope; the forearm stands vertical under the withers
        // (S11) on its pastern; the upper arm closes the gap between the
        // shoulder and the elbow, which says how far behind the shoulder the
        // elbow stands. A blade that would leave the arm too short to reach
        // slides down the ribs instead, as a sling lets it.
        let (incline_sin, incline_cos) = (frame.incline.det_sin(), frame.incline.det_cos());
        let (pastern_sin, pastern_cos) = (frame.pastern.det_sin(), frame.pastern.det_cos());
        let upper = (1.0 + schema.upper * young).clamp(0.75, 1.25);
        let humerus = frame.humerus * upper * wh;
        let carpus_y = pad_y + frame.metacarpus * metapodial * wh * pastern_cos;
        let elbow_y = carpus_y + frame.radius * reach * wh;
        let blade = frame.scapula * wh;
        // Held between an arm hanging nearly straight and one lying no
        // flatter than 17 degrees: past either the arm cannot close the gap.
        let shoulder_y = (frame.scapula_top * wh - blade * incline_sin)
            .min(elbow_y + humerus * 0.97)
            .max(elbow_y + humerus * 0.3);
        let behind = (humerus * humerus - (shoulder_y - elbow_y).powi(2)).sqrt();

        // **The trunk**, its rings where the family's stand, every height but
        // the back's moved by how far the legs moved the elbow: longer legs
        // under the same withers leave a shallower body.
        // Floored where the girdles' hulls would crowd the ribs between
        // them out of their sockets (#374's sweep: a short horse at the
        // bottom of `appeal`).
        let stretch = ((1.0 + 0.12 * long) * (1.0 + schema.length * young)).clamp(0.82, 1.8);
        let length = frame.length * wh * stretch;
        // Point of the shoulder at the front, point of the buttock at the back.
        let shoulder_z = 0.5 * length;
        let buttock_z = -0.5 * length;
        let elbow_z = shoulder_z - behind;
        let natural_elbow = {
            let carpus = frame.metacarpus * frame.pastern.det_cos();
            frame.radius + carpus
        };
        let lift = (elbow_y - pad_y) / wh - natural_elbow;
        let hip_z = buttock_z + frame.hip_ahead * wh * stretch;
        // The pelvis's node stands a little ahead of the hip joints, so the
        // tail leaving its back and the thighs leaving its sides each have a
        // bone long enough to open a socket on.
        let pelvis_z = hip_z + 0.05 * wh;
        // Toward round on the young side; on the other, a tenth of `young`
        // narrower whatever the family's `Schema::round`.
        let rounder = 1.0 + schema.round * young.max(0.0) - 0.1 * young.min(0.0).abs();
        let sites = [CROUP, BELLY, BELLY, CHEST, CHEST, CHEST];
        // **The composites move the trunk from the family's own body**: its
        // frame is that body's, measured at the mass and fat a roll of the
        // family centres on, so each ring's width, and the barrel's depth,
        // take how far this body's girth at the ring's site stands from that
        // one's. A heavier body is a bigger trunk under the same back; a
        // fatter one carries it most at the belly, whose underline drops. At
        // the family's own composites every ring is its frame's. Legginess is
        // in both girths, so it slims the width alone: it has made the trunk
        // shallower already, through `lift`.
        let centred = Girth::of(
            params,
            &Composites {
                mass: centre.mass,
                body_fat: centre.body_fat,
                ..*composites
            },
        );
        let grown = |site: Site| girth.at(site) / centred.at(site);
        let pelvis_width = frame.rings[0].half_width * wh * centred.at(CROUP);
        let rings: [(f32, f32, Vec2); TRUNK_NODES] = std::array::from_fn(|i| {
            let ring = frame.rings[i];
            // The barrel is the four rings between the girdles; the ribcage
            // the three of them ahead of the loin.
            let barrel = (1..=4).contains(&i);
            let ribs = (2..=4).contains(&i);
            // A deep chest drops the underline of the ribcage; the loin's
            // follows by half, the pelvis not at all. A puppy's barrel hangs
            // lower too.
            let drop = match i {
                0 => 0.0,
                1 => 0.02 * deep,
                _ => 0.04 * deep,
            } + if barrel {
                schema.sink * young.max(0.0)
            } else {
                0.0
            };
            let top = ring.top * wh * (1.0 - if i == 1 { 0.04 * ageing } else { 0.0 });
            let bottom = ((ring.bottom + lift - drop) * wh).min(top - 0.2 * wh);
            let mut half_depth = 0.5 * (top - bottom);
            // The family's own width here, which the walls below are drawn
            // on before this body's composites move it, so that a wall holds
            // the shape and never the amount of body.
            let mut half_width = ring.half_width * wh * centred.at(sites[i]);
            // A puppy's ribcage rounds toward round, not past it.
            if ribs {
                half_width = (half_width * rounder).min(half_depth * 1.05);
            }
            // **A cage wall, not anatomy**: the loin is held to a little
            // over the pelvis's width, because the spine's socket on the
            // pelvis's hull carries the loin's width and the thighs' sockets
            // beside it have to clear it within their bones (#374's sweep).
            if i == 1 {
                half_width = half_width.min(pelvis_width * 1.15);
            }
            // The width takes all of the growth and the ribcage's depth the
            // square root of it, under the same back: all of it hung a heavy
            // dog's chest under its elbows, and on the girdles, whose depth
            // is the bone's, crowded the legs' sockets off their hulls
            // (`examples/quadprobe --sweep`). The loin takes half the
            // ribcage's power: its socket on the pelvis's hull is sized by
            // its depth, and a fat horse's at the top of the envelope crowded
            // the thighs' off it (`tests/budget.rs`).
            let grow = grown(sites[i]);
            half_width *= grow;
            if ribs {
                half_depth *= grow.sqrt();
                // And still round, not past it: a heavy kitten's ribs past
                // their depth crowded the forechest's sockets off its hull.
                half_width = half_width.min(half_depth * 1.05);
            } else if i == 1 {
                half_depth *= grow.det_powf(0.25);
            }
            // And held there as the two girths part: a fat loin's belly
            // goes down, where the pelvis's hull is not in its way.
            if i == 1 {
                half_width = half_width.min(pelvis_width * grown(CROUP) * 1.15);
            }
            half_depth = half_depth.max(0.1 * wh);
            // **A cage wall, not anatomy** ([`BRISKET`]): the front of the
            // chest reaches down past the points of the shoulders, so its
            // hull is there to close the chest in front of them. A foal's
            // long cannons raise its underline (`lift`) under shoulders that
            // stay where the blades put them, and left them hanging under
            // the front of its chest.
            if i >= TRUNK_NODES - 2 {
                half_depth = half_depth.max(0.5 * (top - shoulder_y + BRISKET * wh));
            }
            let y = top - half_depth;
            let (half_width, half_depth) = (half_width / SKIN, half_depth / SKIN);
            let radius = half_depth.max(half_width);
            (y, radius, Vec2::new(half_width, half_depth) / radius)
        });
        // The withers are the tops of the blades: the front girdle stands
        // under them, ahead of the elbows by as much as the arm lies back -
        // and a third of the way on toward the points of the shoulders, so
        // the chest its hull closes reaches past them, as the pectorals do.
        // Under the blades' tops, a horse's steep upper arm left the front
        // of its chest behind its own shoulders. **And further on where that
        // falls short** (a cage wall): the forechest's hull, an ellipse in
        // the side view, is held to reach [`PECTORAL`] past the shoulder at
        // the shoulder's height, which moves a horse's on by about 0.06
        // withers heights.
        let withers_z = {
            let (y, radius, section) = rings[TRUNK_NODES - 1];
            let below = (shoulder_y - y) / (radius * section.y);
            let reach = radius * (1.0 - below * below).max(0.0).sqrt();
            (shoulder_z - 0.67 * blade * incline_cos).max(shoulder_z + PECTORAL * wh - reach)
        };
        // The run from the forechest to the pelvis is floored at a multiple
        // of the largest ring, P1's spine floor in one number: a short deep
        // body stretches rather than crowd its ribs out of their girdles'
        // sockets (#374: every axis at -3 did). The hip moves back with it.
        let largest = rings.iter().map(|ring| ring.1).fold(0.0, f32::max);
        let span = (withers_z - pelvis_z).max(SPAN_FLOOR * largest);
        let pelvis_z = withers_z - span;
        let hip_z = pelvis_z - 0.05 * wh;
        let trunk: [(Vec3, f32, Vec2); TRUNK_NODES] = std::array::from_fn(|i| {
            let (y, radius, section) = rings[i];
            let z = withers_z - span * TRUNK_SHARES[i];
            (Vec3::new(0.0, y, z), radius, section)
        });
        let (hips_at, withers_at) = (trunk[0].0, trunk[TRUNK_NODES - 1].0);
        let forechest = trunk[5].1 * trunk[5].2.x * SKIN;
        let pelvis = trunk[0].1 * trunk[0].2.x * SKIN;

        // The neck rises from the top of the chest rather than its front, so
        // the brisket stands forward under the throat, and the head is
        // carried above the back line rather than level with it. How steeply
        // is `neckCarriage`: a cat's carried low and forward, a horse's high
        // (#366). Saturated well short of level and of upright. Floored at
        // the P1 chest's radius, a look rather than a wall: P1's 1.4 of it
        // held every cat's neck there, longer than its family's own (#374).
        let neck_r = h
            * 0.078
            * frame.neck_girth
            * girth.at(NECK)
            * (1.0 + 0.2 * params.chest_depth).clamp(0.6, 1.6)
            * (1.0 + schema.neck_girth * young);
        let neck_reach = (h
            * 0.32
            * frame.neck
            * (1.0 + 0.3 * params.neck_length)
            * (1.0 - schema.neck * young))
            .max(h * 0.19);
        let level_rise = 0.70f32.det_atan2(0.62);
        // Age carries it lower, as an old dog carries its head.
        let rise = (level_rise + 0.26 * params.neck_carriage - 0.12 * ageing).clamp(0.30, 1.35);
        let neck_at = Vec3::new(
            0.0,
            h * 1.06 + neck_reach * 0.935 * rise.det_sin(),
            withers_z + 0.06 * wh + neck_reach * 0.935 * rise.det_cos(),
        );
        // A puppy's head is 0.61 withers heights long against an adult's 0.39
        // (S7): appeal grows it by that ratio, a foal's by less ([`Schema`]).
        let head_r = h * 0.150 * (1.0 + 0.25 * params.head_size) * (1.0 + schema.head * head_young);
        // The skull is two rings, the back of it and the brow, because a lone
        // large ring between two small ones is what subdivision smooths away:
        // two of a size make a plateau it keeps, and the head reads as a mass
        // rather than as the neck's swollen end. The back of it carries on up
        // the neck's own line.
        let occiput_r = head_r * 0.88;
        let occiput_at = neck_at
            + Quat::from_rotation_x(level_rise - rise)
                * Vec3::new(0.0, 0.70, 0.71)
                * (neck_r + occiput_r)
                * 1.05;
        // And the rest of the head nods about it as the neck is carried
        // higher: a horse's long head hangs from an arched neck, a cat's is
        // held level. Positive is nose-down.
        let nod = Quat::from_rotation_x((0.75 * asked.neck_carriage).clamp(-0.21, 0.9));
        let head_at = occiput_at + nod * Vec3::new(0.0, 0.10, 1.0).normalize() * head_r * 0.85;
        // The muzzle leaves the cranium forward and a little down; the nose
        // closes it. Both scale with the head they hang off, and between them
        // they stand the nose `snout` cranium radii past the cranium's front:
        // a third of one on a cat, a dog's 1.3, two on a horse
        // (`muzzleLength`, #366). Floored so the shortest face keeps a nose.
        let snout = ((1.30 + 1.12 * params.muzzle_length) * (1.0 - 0.30 * head_appeal)).max(0.15);
        let ahead = head_r * (0.60 + snout) / 1.90;
        let muzzle_r = head_r * 0.58;
        let muzzle_at = head_at + nod * Vec3::new(0.0, -head_r * 0.30, ahead * 1.15);
        let nose_r = head_r * 0.40;
        let nose_at = muzzle_at + nod * Vec3::new(0.0, -head_r * 0.10, ahead * 0.75);
        let head = head::head(
            &head::Chain {
                nod,
                occiput: (occiput_at, occiput_r),
                cranium: (head_at, head_r),
                nose: (nose_at, nose_r),
                neck: neck_at,
                withers: trunk[TRUNK_NODES - 1].0,
                // A young skull is a round one whatever it grows into.
                back: frame.skull_back * (1.0 - 0.5 * head_young.max(0.0)),
            },
            asked,
            head_appeal,
            Girth::of(asked, as_asked).at(CHEEK),
        );

        // The tail is set high on the croup and carried as `tailCarriage`
        // says: hanging, out and down in a sabre curve, or raised and curled
        // over the back (#366), tapering to its tip. Saturated at the ends,
        // because a curl past `RAISED` folds the tail into the back.
        let tail_step = h * 0.13 * (1.0 + 0.6 * params.tail_length);
        let tail_r = h * 0.050 * girth.at(TAIL);
        // Set at the top of the croup's back, far enough off the pelvis's
        // node for its socket to clear the thighs' on the hull.
        let set = hips_at + Vec3::new(0.0, 0.75, -1.15) * trunk[0].1;
        let carriage = params.tail_carriage.clamp(-1.0, 1.0);
        let toward = if carriage < 0.0 { HANGING } else { RAISED };
        let bends: [f32; TAIL_NODES] =
            std::array::from_fn(|i| SABRE[i] + (toward[i] - SABRE[i]) * carriage.abs());
        let lay = |step: f32| {
            let mut tail = [(set, tail_r); TAIL_NODES];
            let mut at = set;
            for (index, bend) in bends.into_iter().enumerate() {
                if index > 0 {
                    at += Vec3::new(0.0, bend.det_sin(), -bend.det_cos())
                        * step.max(tail[index - 1].1 * 2.2);
                }
                let taper = 1.0 - 0.14 * index as f32;
                tail[index] = (at, (tail_r * taper).max(h * 0.012));
            }
            tail
        };
        // A long tail hanging straight down would run into the ground (#366:
        // a tail three units long at -1, and one roll in three thousand), so
        // its step is shortened until every node clears it by its own radius
        // and a margin. Lengths only: the carriage keeps its shape.
        let clear = h * 0.04;
        let mut tail = lay(tail_step);
        let fit = tail
            .iter()
            .filter(|&&(at, _)| at.y < set.y)
            .map(|&(at, radius)| (set.y - radius - clear) / (set.y - at.y))
            .fold(1.0f32, f32::min);
        if fit < 1.0 {
            tail = lay(tail_step * fit.max(0.0));
        }

        // The legs stand under the body, not beside it (#374): each drops
        // straight from under the side of its girdle's ring, as an animal's
        // do seen from the front.
        // The elbow tucks against the ribs behind it rather than inside them:
        // a puppy's barrel rounds out past its forechest, and a leg set by the
        // forechest alone ran its arm up through the side of the chest.
        let heart = trunk[4].1 * trunk[4].2.x * SKIN;
        let fore_x = forechest.max(heart * 0.85) * 0.80;
        let hind_x = pelvis * 0.80;
        let limb = frame.limb * sturdy;

        // Fore: the blade's top beside the withers, the point of the
        // shoulder ahead and below it, the elbow behind the shoulder at the
        // brisket, the forearm straight down to the carpus, and the pastern
        // forward to the pad.
        let carpus_z = elbow_z;
        let pad_z = carpus_z + frame.metacarpus * metapodial * wh * pastern_sin;
        let fore = [
            (
                Vec3::new(
                    forechest * 0.35,
                    shoulder_y + blade * incline_sin,
                    shoulder_z - blade * incline_cos,
                ),
                wh * 0.06 * limb,
            ),
            (
                // Outside the keel of the ribs: the shoulder joint sits on
                // the blade's outer face, and its socket on the hull opens
                // the further from its fellow for it.
                Vec3::new(
                    splay(forechest * 1.35, withers_at, shoulder_y, shoulder_z),
                    shoulder_y,
                    shoulder_z,
                ),
                wh * 0.065 * limb * girth.limb(THIGH),
            ),
            (
                Vec3::new(fore_x, elbow_y, elbow_z),
                wh * 0.058 * limb * girth.limb(SHANK),
            ),
            (
                Vec3::new(fore_x, carpus_y, carpus_z),
                wh * 0.032 * limb * girth.limb(CANNON),
            ),
            (Vec3::new(fore_x, pad_y, pad_z), pad_r),
            (Vec3::new(fore_x, toe_y, pad_z + toe_r * 1.6), toe_r),
        ];

        // Hind: the paw stands its family's distance behind the hip, the
        // metatarsus straight up from it to the hock, and the stifle closes
        // the thigh and the second thigh between the hip and the hock, bent
        // forward: two bones and two ends are one bend.
        let hip = Vec2::new(hip_z, frame.hip * wh);
        let paw_z = hip_z - frame.paw_behind * wh;
        let hock = Vec2::new(paw_z, pad_y + frame.metatarsus * metapodial * wh);
        let stifle = knee(
            hip,
            hock,
            frame.femur * upper * wh,
            frame.tibia * reach * wh,
        );
        let hind = [
            (
                Vec3::new(splay(pelvis * 0.75, hips_at, hip.y, hip.x), hip.y, hip.x),
                // The thigh's socket on the croup is sized by this joint
                // (`cage::limb`), so it spends the thigh's girth.
                wh * 0.08 * limb * girth.limb(THIGH),
            ),
            (
                Vec3::new(hind_x, stifle.y, stifle.x),
                wh * 0.088 * limb * frame.thigh * girth.limb(THIGH),
            ),
            (
                Vec3::new(hind_x, hock.y, hock.x),
                wh * 0.034 * limb * girth.limb(CANNON),
            ),
            (Vec3::new(hind_x, pad_y, paw_z), pad_r),
            (Vec3::new(hind_x, toe_y, paw_z + toe_r * 1.6), toe_r),
        ];

        Self {
            trunk,
            neck_at,
            neck_r,
            neck_section: frame.neck_section,
            occiput_at,
            occiput_r,
            head_at,
            head_r,
            muzzle_at,
            muzzle_r,
            nose_at,
            nose_r,
            head,
            tail,
            hind,
            fore,
            stifle_section: frame.thigh_section,
        }
    }
}

/// How far each axis a body's girdles are built from goes from its family's
/// centre toward what the record asks (#391), in this order: `legLength`,
/// `bodyLength`, `chestDepth`, `stance`, `neckLength`, `neckCarriage`, what
/// `appeal` makes of the body, and the composites `mass`, `bodyFat` and age.
/// `1` is all the way, the record's own value; `0` none of it, the family's.
///
/// **These are the axes a girdle's sockets are crowded by**: they set how
/// deep the chest hangs against the elbows, how far the girdles stand from
/// the ribs beside them, how thick the legs and the neck leave them and at
/// what angle. Easing one moves the body along the line its own slider runs
/// on, so an eased body is one a creator could have asked for.
pub(crate) type Ease = [f32; EASED_AXES];

/// How many axes an [`Ease`] carries.
pub(crate) const EASED_AXES: usize = 10;

/// The [`Ease`] of a body as its record asks for it.
pub(crate) const AS_ASKED: Ease = [1.0; EASED_AXES];

/// The axes and composites [`Dimensions::eased`] builds a body from: each
/// eased axis its [`Ease`]'s share of the way from its family's centre to
/// what was asked, the rest as asked. An axis at a share of 1 is the
/// record's own value, untouched rather than recomputed, so a body that is
/// not eased is bit-identical to one built before easing existed.
fn eased_axes(
    asked: &QuadrupedParams,
    composites: &Composites,
    centre: &crate::plan::quadruped::AxisCentres,
    ease: &Ease,
) -> (QuadrupedParams, Composites) {
    let toward = |axis: usize, value: f32, centre: f32| {
        if ease[axis] >= 1.0 {
            value
        } else {
            centre + (value - centre) * ease[axis]
        }
    };
    let params = QuadrupedParams {
        leg_length: toward(0, asked.leg_length, centre.leg_length),
        body_length: toward(1, asked.body_length, centre.body_length),
        chest_depth: toward(2, asked.chest_depth, centre.chest_depth),
        stance: toward(3, asked.stance, centre.stance),
        neck_length: toward(4, asked.neck_length, centre.neck_length),
        neck_carriage: toward(5, asked.neck_carriage, centre.neck_carriage),
        appeal: toward(6, asked.appeal, centre.appeal),
        ..*asked
    };
    let composites = Composites {
        mass: toward(7, composites.mass, centre.mass),
        body_fat: toward(8, composites.body_fat, centre.body_fat),
        age: if ease[9] >= 1.0 {
            composites.age
        } else {
            toward(9, composites.age as f32, centre.age).round() as u32
        },
        ..*composites
    };
    (params, composites)
}

/// How far out a leg's inner joint stands - a shoulder, a hip - given where
/// it would stand and the girdle it hangs from.
///
/// **A cage wall, not anatomy.** The cage aims each leg's socket at this
/// joint (`Skeleton::bridge_aim`), and the two legs' sockets open apart on
/// the girdle's hull only if they leave it at least a third of the way
/// sideways (#374's sweep: a horse's shoulder dropped far below its
/// forechest aimed both sockets down the same line). Held there, the joint
/// moves out a little on the few bodies that need it, and nowhere else.
fn splay(lateral: f32, girdle: Vec3, y: f32, z: f32) -> f32 {
    let off = Vec2::new(y - girdle.y, z - girdle.z).length();
    lateral.max(0.65 * off)
}

/// Where a two-bone leg from `hip` to `hock` bends, in the body's side plane
/// as `(z, y)`: the bend that puts the joint FORWARD of the line between
/// them, a stifle's. A reach longer than both bones straightens the leg; a
/// shorter one than their difference folds it shut.
fn knee(hip: Vec2, hock: Vec2, femur: f32, tibia: f32) -> Vec2 {
    let span = hock - hip;
    let reach = span
        .length()
        .clamp((femur - tibia).abs() + 1e-4, (femur + tibia) * 0.999);
    let along = span / span.length().max(1e-6);
    let cos =
        ((femur * femur + reach * reach - tibia * tibia) / (2.0 * femur * reach)).clamp(-1.0, 1.0);
    let turn = cos.det_acos();
    let (sin, cos) = (turn.det_sin(), turn.det_cos());
    // Turned toward +z: the leg runs down, so forward is a clockwise turn
    // in (z, y) when the reach points down and back.
    let one = Vec2::new(along.x * cos - along.y * sin, along.x * sin + along.y * cos);
    let other = Vec2::new(
        along.x * cos + along.y * sin,
        -along.x * sin + along.y * cos,
    );
    let pick = if one.x > other.x { one } else { other };
    hip + pick * femur
}
