//! The beast's head (#367): the parts a [`BeastHead`] is a smooth union of,
//! placed about the head chain [`super::Dimensions`] hangs and shaped by the
//! head's own axes.
//!
//! **The chain is the head's skeleton and these are its flesh.** The cage
//! builds the head from the occiput, the cranium, the muzzle and the nose, and
//! subdivision rounds that into a tube; the parts here say what the tube is
//! shaped into. They are placed off the chain's nodes - the muzzle node only
//! carries the cage's tube between the two it lies between - so the projected
//! surface moves the head's vertices by a small share of its radius and a
//! skin weight bound after it still lands on the bone it would have.
//!
//! Every coefficient is in cranium radii and **tuned by render** (#367), on
//! `render --quadruped --gallery N --portrait` across the three families.

use glam::{Mat3, Quat, Vec2, Vec3};

use crate::plan::beast_head::{
    BeastHead, Blends, Blob, Cone, EarSeat, EyeSeat, HornSeat, Muzzle, NoseSeat, WHISKERS,
    WhiskerSeat,
};
use crate::plan::{Horn, Nose, QuadrupedParams};

/// One node of the head chain: where it sits, at rest in body space, and its
/// radius.
pub(super) type Ball = (Vec3, f32);

/// The head chain the parts are placed about.
pub(super) struct Chain {
    /// The nod that turns the head on its neck; the head's frame turns with
    /// it.
    pub nod: Quat,
    /// The back of the skull.
    pub occiput: Ball,
    /// The cranium, the frame's origin.
    pub cranium: Ball,
    /// The nose, which ends the chain.
    pub nose: Ball,
}

/// The share of a node's radius its subdivided surface keeps.
///
/// Subdivision pulls a surface inside the cage's rings, and a part sized at
/// the node's full radius would swell every head by the difference.
/// Provenance: **derived** - an eight-point ring's flats sit at `cos(pi/8)`,
/// 0.924, of its corners, and one Catmull-Clark pass settles the round tube
/// between the two.
const SURFACE: f32 = 0.93;

/// How far ahead of the muzzle's centre a nose this long reaches before a
/// muzzle counts as short, in cranium radii: a dog's, about. Below it the
/// lip's lobes grow toward a cat's whisker pads.
const SHORT_FACE: f32 = 1.6;

/// How far round the head from straight ahead an eye is seated, in radians,
/// at `eyeSet` -1, 0 and +1: on the front of a flat face, a little round
/// the side of a dog's, and on the side of a horse's skull.
/// Provenance: **tuned by render** (#367).
const EYE_SET: [f32; 3] = [0.42, 0.75, 1.25];

/// How far above the braincase's centre an eye is seated, as the rise of
/// the ray that seats it, in radians, at `eyeSet` -1, 0 and +1: over the
/// bridge of the nose, and higher on the side of a long skull.
/// Provenance: **tuned by render** (#367).
const EYE_RISE: [f32; 3] = [0.30, 0.28, 0.36];

/// How much of a globe's radius stands proud of the head: what shows
/// between the lids is the cap past the surface, and at four tenths that
/// cap is the iris and little else.
/// Provenance: **tuned by render** (#367).
const PROUD: f32 = 0.40;

/// How much of the surface's slope an eye's gaze keeps: a little. Seated on
/// the curve of the braincase, an eye looking along its normal stares at
/// the sky; held near level, it looks where the animal is going.
/// Provenance: **tuned by render** (#367).
const GAZE_TILT: f32 = 0.25;

/// The head of one quadruped: its parts, placed about its chain.
///
/// `appeal` arrives saturated as the rest of the derive reads it, and
/// `fullness` is the girth's factor at the cheek: a fat animal's face is a
/// round one.
pub(super) fn head(
    chain: &Chain,
    params: &QuadrupedParams,
    appeal: f32,
    fullness: f32,
) -> BeastHead {
    let (origin, r) = chain.cranium;
    let local = |at: Vec3| chain.nod.inverse() * (at - origin);
    let occiput = local(chain.occiput.0);
    let nose = local(chain.nose.0);
    // Saturated where a part would leave the head or swallow the next one.
    let breadth = params.muzzle_breadth.clamp(-1.5, 1.5);
    let stop = params.stop.clamp(-1.5, 1.5);
    let length = params.muzzle_length.clamp(-1.5, 1.5);
    let short = ((SHORT_FACE - nose.z / r) / SHORT_FACE).clamp(0.0, 1.0);
    // A long face is a long skull: narrower and flatter over the brain, and
    // deep through the jaw - a horse's wedge rather than a ball with a
    // spout in it.
    let long = length.max(0.0);

    // The braincase is rounder as the face is cuter and narrower as it is
    // longer.
    let wide = 1.0 + 0.05 * appeal - 0.08 * long;

    // The muzzle runs from inside the braincase to the nose, so the face
    // under the eyes is its back half: side on, the head tapers from the
    // cheek to the nose instead of putting a tube into a ball. Its top line
    // is the bridge of the nose, which `stop` sets against the forehead -
    // level with it and bowed for a Roman profile, under a high brow for a
    // deep stop - and its bottom line is the upper lip's: the lower jaw is a
    // part of its own, hinged under the ear (`face::beast::jaw`), standing up
    // into the head from below. `muzzleBreadth` widens it, more at the nose
    // than at the cheek, and squares its section as it widens: a boxer's
    // muzzle against a borzoi's wedge.
    let nose_r = chain.nose.1 * SURFACE;
    let front = nose.z + nose_r * 0.95;
    let width_front = nose_r * (1.0 + 0.30 * breadth);
    let height_front = nose_r * (1.0 + 0.10 * breadth) * (1.0 + 0.25 * long);
    let nose_line = nose.y + 0.04 * r;
    let lip = nose_line - height_front * 0.85;
    let muzzle = Muzzle {
        span: Vec2::new(-0.05 * r, front),
        top: Vec2::new((0.62 - 0.17 * stop) * r, nose_line + height_front),
        bottom: Vec2::new(-0.45 * r, lip),
        half_width: Vec2::new(0.82 * r * wide * (1.0 + 0.15 * breadth), width_front),
        square: 2.0 + 0.6 * (1.0 + breadth).clamp(0.0, 2.0),
        round: 0.5 * width_front.min(height_front),
        crown: 0.07 * r * (-stop).max(0.0),
    };

    // The upper lip's lobes, either side of the nose and under it: a short
    // face's whisker pads, round and forward, and a long face's flews or
    // lips, smaller against the muzzle they hang from.
    let pad = r * (0.20 + 0.04 * (1.0 + breadth).clamp(0.0, 2.0) + 0.09 * short);
    let pads = Blob {
        centre: Vec3::new(
            width_front * 0.62 + 0.10 * r * short,
            nose_line - height_front * 0.55,
            front - pad * 1.05,
        ),
        radii: Vec3::new(0.95, 1.0, 1.15) * pad,
    };

    // The lower jaw, from inside the cheek to a chin set back under the
    // upper lip. Its top stands above the lip's line all the way along, so
    // the closed jaw fills the head from below and the line where the two
    // surfaces cross is the mouth's; what shows under it is the jaw.
    let jaw = Cone {
        back: Vec3::new(0.0, -0.48 * r, 0.12 * r),
        back_radius: 0.26 * r * (1.0 + 0.10 * breadth),
        front: Vec3::new(0.0, lip - 0.05 * r, front - 0.28 * r - 0.4 * pad),
        front_radius: 0.14 * r * (1.0 + 0.20 * breadth),
    };
    // It hinges under the ear and behind the eye.
    let hinge = Vec3::new(0.0, -0.25 * r, -0.22 * r);
    // The tongue lies along the jaw's floor, just under its top, from a root
    // behind the cheek to a tip short of the chin: hidden in the head while
    // the mouth is shut.
    let top_of_jaw = |z: f32| {
        let along = (z - jaw.back.z) / (jaw.front.z - jaw.back.z);
        let along = along.clamp(0.0, 1.0);
        jaw.back.y
            + (jaw.front.y - jaw.back.y) * along
            + jaw.back_radius
            + (jaw.front_radius - jaw.back_radius) * along
    };
    let (root, tip) = (0.10 * r, jaw.front.z - 0.02 * r);
    let tongue = Cone {
        back: Vec3::new(0.0, top_of_jaw(root) - 0.02 * r, root),
        back_radius: 0.16 * r * (1.0 + 0.20 * breadth),
        front: Vec3::new(0.0, top_of_jaw(tip) - 0.02 * r, tip),
        front_radius: 0.12 * r * (1.0 + 0.20 * breadth),
    };

    // The nose. A leather pad caps the muzzle's tip, its top a little above
    // the bridge's end and its front well proud of the muzzle's,
    // as wide as the muzzle lets it be: a dog's ball, a cat's button. The
    // nostrils are cut into its front, low and either side of the middle;
    // without a pad they open on the muzzle's own front, wider apart and
    // larger - a horse's.
    let leather_or_soft = match params.nose {
        Nose::Leather => {
            let pad_r = width_front * 0.55;
            let leather = Blob {
                centre: Vec3::new(0.0, nose_line + height_front * 0.75, front - pad_r * 0.25),
                radii: Vec3::new(1.0, 0.72, 0.80) * pad_r,
            };
            NoseSeat {
                leather: Some(leather),
                nostril: Blob {
                    centre: leather.centre + Vec3::new(pad_r * 0.42, -pad_r * 0.15, pad_r * 0.78),
                    radii: Vec3::new(0.22, 0.16, 0.22) * pad_r,
                },
                blend: 0.06 * r,
            }
        }
        Nose::Soft => NoseSeat {
            leather: None,
            nostril: Blob {
                centre: Vec3::new(
                    width_front * 0.55,
                    nose_line + height_front * 0.15,
                    front - width_front * 0.12,
                ),
                radii: Vec3::new(0.14, 0.24, 0.16) * width_front,
            },
            blend: 0.05 * r,
        },
    };

    // The cheeks, behind the mouth and under the eye: broader as the face is
    // rounder and the body fatter, and deeper as the jaw is longer - a long
    // jaw is a big masseter, which is a horse's jowl.
    let cheek = (1.0 + 0.30 * long + 0.35 * (fullness - 1.0) + 0.04 * appeal).clamp(0.6, 1.6);
    // Held forward of the neck: a jowl reaching back to where the head's
    // shaping fades out folds the surface there, and the fold paints as a
    // seam.
    let cheeks = Blob {
        centre: Vec3::new(
            0.50 * r * wide,
            (-0.35 - 0.15 * long) * r,
            (0.25 - 0.25 * long) * r,
        ),
        radii: Vec3::new(0.45, 0.55, 0.55) * r * cheek,
    };

    let mut head = BeastHead {
        origin,
        turn: chain.nod,
        radius: r,
        axis: [occiput, nose],
        cranium: Blob {
            centre: Vec3::new(0.0, 0.03 * r, 0.0),
            radii: Vec3::new(wide, 0.95 - 0.18 * long, 1.0 + 0.10 * long) * r * SURFACE,
        },
        occiput: Blob {
            centre: occiput,
            radii: Vec3::new(1.0, 0.95, 1.0) * chain.occiput.1 * SURFACE,
        },
        muzzle,
        pads,
        jaw,
        hinge,
        tongue,
        cheeks,
        nose: leather_or_soft,
        blends: Blends {
            occiput: 0.30 * r,
            // The stop: a crease under a deep one, a slope under none.
            muzzle: (0.30 - 0.16 * stop).clamp(0.06, 0.55) * r,
            pads: 0.10 * r,
            cheeks: 0.25 * r,
        },
        // Seated on the finished head below: the seat is cast against the
        // surface every other part has already made.
        eye: EyeSeat {
            centre: Vec3::ZERO,
            facing: Quat::IDENTITY,
            radius: 0.0,
            pupil: params.pupil,
        },
        ear: EarSeat {
            base: Vec3::ZERO,
            frame: Quat::IDENTITY,
            length: 0.0,
            width: 0.0,
            point: 0.0,
            cup: 0.0,
            thickness: 0.0,
            fold_at: 0.0,
            fold_axis: Vec3::X,
            fold: 0.0,
        },
        whiskers: None,
        horn: None,
    };
    head.eye = eye_seat(&head, params, appeal);
    head.ear = ear_seat(&head, params, appeal);
    head.whiskers = whisker_seat(&head, params);
    head.horn = horn_seat(&head, params);
    head
}

/// Where the whiskers sprout and how they run (#367), or `None` below
/// [`NO_WHISKERS`] of `whiskers`.
///
/// Four a side, on the outer face of the lip's lobe, fanning from a little
/// above level to a little below, each setting off out and a little forward
/// and sagging along its length: the fan a cat's face is drawn with.
fn whisker_seat(head: &BeastHead, params: &QuadrupedParams) -> Option<WhiskerSeat> {
    let reach = params.whiskers.clamp(-1.5, 1.5);
    if reach <= NO_WHISKERS {
        return None;
    }
    let r = head.radius;
    let pad = head.pads;
    let rises: [f32; WHISKERS] = [0.32, 0.12, -0.08, -0.28];
    let roots = rises.map(|rise| {
        let toward = Vec3::new(
            0.9f32.sin() * rise.cos(),
            rise.sin(),
            0.9f32.cos() * rise.cos(),
        );
        pad.centre + toward * pad.radii * 0.96
    });
    let directions = rises.map(|rise| Vec3::new(1.0, 0.9 * rise, 0.35).normalize());
    Some(WhiskerSeat {
        roots,
        directions,
        length: r * (0.55 + 0.45 * reach),
        droop: 0.18,
    })
}

/// Below this much `whiskers`, a muzzle grows none.
const NO_WHISKERS: f32 = -0.6;

/// Where a horn stands, if the record gives one (#367): on the forehead
/// above and between the eyes, rising forward of upright as the body stands
/// however the head is carried, a little longer than the braincase is tall.
fn horn_seat(head: &BeastHead, params: &QuadrupedParams) -> Option<HornSeat> {
    if params.horn == Horn::None {
        return None;
    }
    let r = head.radius;
    let centre = head.cranium.centre;
    let up = Vec3::new(0.0, 0.62f32.cos(), 0.62f32.sin());
    let forehead = head.leave(centre, up).unwrap_or(centre + up * r);
    // Up and forward as the body stands, not as the head is carried: a
    // horse's head hangs nose-down, and a horn turned with it points at
    // whatever is in front of it.
    let direction = head.turn.inverse() * Vec3::new(0.0, 0.75, 0.66).normalize();
    Some(HornSeat {
        base: forehead - direction * (0.06 * r),
        direction,
        length: 1.3 * r,
        radius: 0.15 * r,
    })
}

/// Where the ears stand on a head and how they are made (#367).
///
/// `earCarriage` decides most of it. A pricked ear stands high on the crown,
/// nearly upright, cupped deep and facing forward; a rose ear is turned out
/// and back and folded back along its middle; a dropped ear leaves lower on
/// the side of the skull, along the surface, and folds outward and down a
/// fifth of the way up, so the flap hangs beside the face. `earSize` and
/// `earPoint` are the leather: how long, and how the tip ends, and a pointed
/// ear is a narrower one.
fn ear_seat(head: &BeastHead, params: &QuadrupedParams, appeal: f32) -> EarSeat {
    let r = head.radius;
    let size = params.ear_size.clamp(-1.5, 1.5);
    let point = ((params.ear_point.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    let carriage = params.ear_carriage.clamp(-1.0, 1.0);
    let up = (carriage + 1.0) * 0.5;
    // Where on the braincase: the polar angle from the crown, and how far
    // behind the eyes. A dropped ear leaves from the side of the skull,
    // where the surface faces out, so the flap hangs clear of the cheek.
    let spread = 1.30 - 0.75 * up;
    let setback: f32 = 0.30;
    let toward = Vec3::new(
        spread.sin() * setback.cos(),
        spread.cos() * setback.cos(),
        -setback.sin(),
    );
    let centre = head.cranium.centre;
    let surface = head.leave(centre, toward).unwrap_or(centre + toward * r);
    let normal = head.normal(surface);
    // How it rises: between the head's up and the surface, a pricked ear
    // nearly upright and a dropped one along the skull.
    let rise = Vec3::Y
        .lerp(normal, 0.35 + 0.65 * (1.0 - up))
        .normalize_or(Vec3::Y);
    // Where its cup opens: forward, turned out about the ear's own length -
    // furthest on a rose.
    let turn = if carriage >= 0.0 {
        1.0 + (0.25 - 1.0) * carriage
    } else {
        1.0 + 0.7 * carriage
    };
    let forward = (Vec3::Z - rise * rise.dot(Vec3::Z)).normalize_or(Vec3::Z);
    let opening = Quat::from_axis_angle(rise, turn) * forward;
    let frame = Quat::from_mat3(&Mat3::from_cols(rise.cross(opening), rise, opening));
    // The fold: about the ear's own forward axis for a drop, so the flap
    // goes out and down, turning to its across for a rose, so it goes back;
    // none at all for a pricked ear.
    let toward_rose = (1.0 + carriage).clamp(0.0, 1.0);
    let angle = toward_rose * std::f32::consts::FRAC_PI_2;
    // A drop folds a little less than a right angle, so the flap leans out
    // from the skull it hangs beside rather than into it.
    let fold = if carriage < 0.0 {
        1.15 + (1.3 - 1.15) * toward_rose
    } else {
        1.3 * (1.0 - carriage)
    };
    let length = r * (0.85 + 0.30 * size) * (1.0 + 0.10 * appeal) * (1.0 + 0.35 * (1.0 - up));
    let width = length * (0.78 - 0.16 * point);
    EarSeat {
        base: surface - rise * (0.12 * length),
        frame,
        length,
        width,
        point,
        cup: 0.12 + 0.18 * up,
        thickness: 0.07 * width,
        // Folded no nearer the skull than a quarter of a radius out, so a
        // short ear's flap clears the cheek it hangs beside.
        fold_at: (0.30 + 0.15 * toward_rose)
            .max(0.12 + 0.25 * r / length.max(f32::EPSILON))
            .min(0.7),
        fold_axis: -(Vec3::Z * angle.cos() + Vec3::X * angle.sin()),
        fold,
    }
}

/// Where the eyes sit on a head (#367): cast from the braincase's centre to
/// the surface where `eyeSet` puts them, looking along the surface's own
/// normal there held near level ([`GAZE_TILT`]), and sunk along the gaze
/// until [`PROUD`] of the globe stands out. A cute face is drawn with larger
/// eyes.
fn eye_seat(head: &BeastHead, params: &QuadrupedParams, appeal: f32) -> EyeSeat {
    let r = head.radius;
    let set = params.eye_set.clamp(-1.5, 1.5);
    let along = |[frontal, middle, lateral]: [f32; 3]| {
        if set < 0.0 {
            middle + (middle - frontal) * set
        } else {
            middle + (lateral - middle) * set
        }
    };
    let rise = along(EYE_RISE).clamp(0.0, 0.6);
    let radius = r * (0.25 + 0.05 * appeal);
    let centre = head.cranium.centre;
    // The braincase's front: a seat past it is on the muzzle.
    let brow = centre.z + head.cranium.radii.z;
    let seat_at = |round: f32| {
        let toward = Vec3::new(
            round.sin() * rise.cos(),
            rise.sin(),
            round.cos() * rise.cos(),
        );
        let surface = head.leave(centre, toward).unwrap_or(centre + toward * r);
        let normal = head.normal(surface);
        let gaze = Vec3::new(normal.x, normal.y * GAZE_TILT, normal.z).normalize_or(Vec3::Z);
        (surface - gaze * radius * (1.0 - PROUD), gaze)
    };
    // **Round the head until the seat is one** (#367's critic): at the
    // envelope's frontal end a long Roman muzzle's bridge catches the seating
    // ray first and puts the eye on the nose, and the pair can come close
    // enough to the midline to run into each other. A seat on the braincase,
    // with room for its globe either side of the middle, is reached by
    // turning further out; a body inside the envelope's middle is seated at
    // its first try.
    let mut round = along(EYE_SET).clamp(0.15, 1.45);
    let (mut seat, mut gaze) = seat_at(round);
    while (seat.z > brow || seat.x < radius * SEAT_ROOM) && round < MOST_ROUND {
        round = (round + SEAT_STEP).min(MOST_ROUND);
        (seat, gaze) = seat_at(round);
    }
    EyeSeat {
        centre: seat,
        facing: Quat::from_rotation_arc(Vec3::Z, gaze),
        radius,
        pupil: params.pupil,
    }
}

/// How far an eye's centre must stand from the midline, in its globe's
/// radii: past one, so the pair never meet, with a little air between.
const SEAT_ROOM: f32 = 1.1;

/// How far round the head each retry of a seat turns, in radians.
const SEAT_STEP: f32 = 0.05;

/// The furthest round the head an eye is ever seated, in radians: on the
/// side of the skull.
const MOST_ROUND: f32 = 1.45;
