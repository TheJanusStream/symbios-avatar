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
    BeastHead, Blends, Blob, Cone, EarSeat, EyeSeat, Muzzle, Nape, NoseSeat, OrnamentSeat,
    WHISKERS, WhiskerSeat,
};
use crate::plan::ornament::{Ornament, OrnamentKind};
use crate::plan::{Nose, QuadrupedParams};

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
    /// The neck joint the chain hangs from.
    pub neck: Vec3,
    /// Where the neck leaves the trunk.
    pub withers: Vec3,
    /// How far the back of the skull has gone from a cat's round one toward
    /// a horse's poll and jowl: 0 to 1 (`Frame::skull_back`).
    pub back: f32,
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
        poll: None,
        muzzle,
        pads,
        jaw,
        hinge,
        tongue,
        cheeks,
        nape: None,
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
        ornaments: [None; 6],
    };
    head.eye = eye_seat(&head, params, appeal);
    head.ear = ear_seat(&head, params, appeal);
    head.whiskers = whisker_seat(&head, params);
    head.ornaments = ornament_seats(&head, params, false);
    back_of_head(&mut head, chain.back, fullness);
    let on_the_face = ornament_seats(&head, params, true);
    for (slot, seat) in head.ornaments.iter_mut().zip(on_the_face) {
        *slot = slot.or(seat);
    }
    head.nape = Some(nape(&head, chain));
    head
}

/// The back of the head as its anatomy has it (#388), from `back` of the way
/// from a cat's round skull to a horse's.
///
/// A horse's skull is a long wedge: the forehead runs back to a poll, the
/// corner between and behind the ears; from there its nape falls almost
/// straight to the throat; and under it stands the jowl, the flat plate of the
/// jaw's ramus, as deep as the head is anywhere and much narrower than the
/// skull. A cat's is a round braincase with its cheeks on the front of it. So
/// the back ball draws in to the wedge's nape, a poll grows at its top, and the
/// cheeks flatten from balls into plates, set further back and lower.
///
/// **After the seats, because everything that rides on the head is seated on
/// the skull in front of all this**: an ear's base, a horn's, an eye and a
/// whisker all stay where they were, so a rig's ear line does not move and
/// Overlands' seats read what they read.
fn back_of_head(head: &mut BeastHead, back: f32, fullness: f32) {
    let r = head.radius;
    let back = back.clamp(0.0, 1.0);
    let wide = head.cranium.radii.x / (r * SURFACE);
    head.occiput = Blob {
        centre: head
            .occiput
            .centre
            .lerp(Vec3::new(0.0, 0.10 * r, -0.50 * r), back),
        radii: head
            .occiput
            .radii
            .lerp(Vec3::new(0.56, 0.62, 0.62) * r, back),
    };
    // Well inside the skull at a cat's end, where it adds nothing.
    head.poll = Some(Blob {
        centre: Vec3::new(0.0, 0.08 * r, -0.30 * r).lerp(Vec3::new(0.0, 0.58 * r, -0.72 * r), back),
        radii: Vec3::new(0.34, 0.24, 0.34) * r,
    });
    head.axis[0] = head.occiput.centre;
    let plate = Blob {
        centre: Vec3::new(0.40 * r * wide, -0.52 * r, -0.20 * r),
        radii: Vec3::new(0.30 * fullness.max(1.0), 0.72, 0.80) * r,
    };
    head.cheeks = Blob {
        centre: head.cheeks.centre.lerp(plate.centre, back),
        radii: head.cheeks.radii.lerp(plate.radii, back),
    };
}

/// The neck's upper run for a head (#388): every section's size measured off
/// the head as it now stands.
///
/// The run starts a bounded way down the neck, in head radii, and opens from
/// the [`THROAT`] share of the head's section at the neck joint to the
/// [`SEAT`] share inside the skull.
fn nape(head: &BeastHead, chain: &Chain) -> Nape {
    let r = head.radius;
    let local = |at: Vec3| head.local(at);
    let neck = local(chain.neck);
    let withers = local(chain.withers);
    let down = (withers - neck).length();
    let base = neck + (withers - neck) * (NAPE_RUN * r / down.max(f32::EPSILON)).min(NAPE_MOST);
    let end = head.occiput.centre.lerp(Vec3::ZERO, 0.5);
    let chord = end - base;
    let length = chord.length().max(f32::EPSILON);
    let along = chord / length;
    let crest = Vec3::new(0.0, along.z, -along.y).normalize_or(Vec3::Y);
    let section = rear_section(head);
    Nape {
        base,
        end,
        knee: (neck - base).dot(along) / length,
        bow: (neck - base).dot(crest),
        throat: section * THROAT,
        seat: section * SEAT,
        blend: NAPE_BLEND * r,
    }
}

/// The head's half-extents `(across, deep)` behind the eye, where the neck
/// comes into it: the widest it reaches to the side over its height and half
/// the height it stands there.
fn rear_section(head: &BeastHead) -> Vec2 {
    let r = head.radius;
    let z = -REAR_SLICE * r;
    let across = [0.35, 0.0, -0.35, -0.70]
        .into_iter()
        .filter_map(|y| head.leave(Vec3::new(0.0, y * r, z), Vec3::X))
        .map(|at| at.x)
        .fold(0.0, f32::max)
        .max(0.5 * r);
    let top = head
        .leave(Vec3::new(0.0, 0.0, z), Vec3::Y)
        .map_or(r, |at| at.y);
    let bottom = head
        .leave(Vec3::new(0.5 * across, 0.0, z), Vec3::NEG_Y)
        .map_or(-r, |at| at.y);
    Vec2::new(across, 0.5 * (top - bottom))
}

/// How far behind the braincase's centre the head's section is read, in head
/// radii: behind the eye, on the skull and the jowl, in front of the nape.
const REAR_SLICE: f32 = 0.45;

/// How much of the head's section the neck's is at the neck joint, across and
/// through: an animal's throat narrows behind the jaw, so it is not the skull's
/// width, and a neck under half of it reads as a post in a ball.
/// Provenance: **tuned by render** (#388).
const THROAT: Vec2 = Vec2::new(0.60, 0.64);

/// How much of the head's section the run has reached inside the skull.
/// Provenance: **tuned by render** (#388).
const SEAT: f32 = 0.85;

/// How far down the neck the run starts, in head radii, and at most what share
/// of the neck: over this much the neck goes from the cage's size to the
/// head's. Provenance: **tuned by render** (#388).
const NAPE_RUN: f32 = 2.6;
const NAPE_MOST: f32 = 0.55;

/// How softly the run joins the head, in head radii: the crease it would leave
/// is the join the owner saw. Provenance: **tuned by render** (#388).
const NAPE_BLEND: f32 = 0.30;

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

/// Where everything the record has it wear stands (#402), a slot a kind, read off the
/// head as declared so far: its surface, its ears, its lip and its jaw. **Nothing
/// here names a family**, so a cross and a head at either end of the envelope are
/// asked the same questions.
///
/// **In two goes**, as `on_the_face` says. What stands on the crown is seated with the
/// ears, on the skull in front of [`back_of_head`], as a horn always was, and what
/// stands on the lip and the jaw once that has moved the cheeks it has to clear.
fn ornament_seats(
    head: &BeastHead,
    params: &QuadrupedParams,
    on_the_face: bool,
) -> [Option<OrnamentSeat>; 6] {
    OrnamentKind::ALL.map(|kind| {
        let face = matches!(
            kind,
            OrnamentKind::UpperFangs | OrnamentKind::LowerFangs | OrnamentKind::Tusks
        );
        if face != on_the_face {
            return None;
        }
        let mut piece = params.ornaments.get(kind)?;
        piece.sanitize();
        let mut seat = match kind {
            OrnamentKind::Horn => horn_seat(head, &piece),
            OrnamentKind::PairedHorns => crown_seat(head, &piece, PAIRED_CROWN),
            OrnamentKind::Antlers => crown_seat(head, &piece, ANTLER_CROWN),
            OrnamentKind::UpperFangs => upper_fang_seat(head, &piece),
            OrnamentKind::LowerFangs => jaw_seat(head, &piece, LOWER_FANG_ON_JAW, FANG_SIDES),
            OrnamentKind::Tusks => jaw_seat(head, &piece, TUSK_ON_JAW, TUSK_SIDES),
        };
        if kind == OrnamentKind::Antlers {
            let mut antlers = params.ornaments.antlers?;
            antlers.sanitize();
            seat.tines = Some((
                (antlers.points - 1) as u8,
                antlers.fork,
                head.turn.inverse() * Vec3::Z,
            ));
        }
        Some(seat)
    })
}

/// A piece's shape, its line's vectors left for the seat to set.
fn shaped(head: &BeastHead, piece: &Ornament, sides: u8, pair: bool) -> OrnamentSeat {
    OrnamentSeat {
        base: Vec3::ZERO,
        direction: Vec3::Y,
        bend: Vec3::Z,
        side: Vec3::X,
        across: Vec3::X,
        length: piece.length * head.radius,
        radius: piece.thickness * head.radius,
        taper: piece.taper,
        curve: piece.curve.abs(),
        twist: piece.twist,
        flutes: piece.flutes,
        colours: [Vec3::from(piece.base), Vec3::from(piece.tip)],
        sides,
        pair,
        tines: None,
    }
}

/// `toward` with whatever of it runs along the unit `along` taken off, as a unit
/// vector: the way a line going `along` turns to head toward it.
fn square_to(toward: Vec3, along: Vec3) -> Vec3 {
    (toward - along * toward.dot(along)).normalize_or(along.any_orthonormal_vector())
}

/// How far a crown piece's base is sunk into the skull, so that a thick one's rim is
/// under the surface as well: a unicorn's, up to the thickness at which it starts
/// to show. Provenance: **tuned by render** (#402).
fn sunk(head: &BeastHead, radius: f32) -> f32 {
    (0.06 * head.radius).max(0.6 * radius - 0.04 * head.radius)
}

/// One horn (#367): on the forehead above and between the eyes, rising forward of
/// upright as the body stands however the head is carried. Its curve turns it
/// forward over the nose, or back over the skull when negative.
fn horn_seat(head: &BeastHead, piece: &Ornament) -> OrnamentSeat {
    let r = head.radius;
    let centre = head.cranium.centre;
    let up = Vec3::new(0.0, 0.62f32.cos(), 0.62f32.sin());
    let forehead = head.leave(centre, up).unwrap_or(centre + up * r);
    // Up and forward as the body stands, not as the head is carried: a
    // horse's head hangs nose-down, and a horn turned with it points at
    // whatever is in front of it.
    let direction = head.turn.inverse() * Vec3::new(0.0, 0.75, 0.66).normalize();
    let mut seat = shaped(head, piece, HORN_SIDES, false);
    seat.base = forehead - direction * sunk(head, seat.radius);
    seat.direction = direction;
    let forward = Vec3::X.cross(direction);
    seat.bend = if piece.curve < 0.0 { -forward } else { forward };
    seat
}

/// Facets round a horn's section, and round a smaller piece's: a fang's is a few
/// pixels across wherever a face is seen from.
const HORN_SIDES: u8 = 12;
const PAIRED_SIDES: u8 = 10;
const FANG_SIDES: u8 = 8;
const TUSK_SIDES: u8 = 10;
const BEAM_SIDES: u8 = 8;

/// How a pair stands on the crown: where on the skull before any ear is in the way,
/// and the ways it sets off and turns as the body stands (`+X` out, `+Y` up, `+Z`
/// forward).
struct Crown {
    /// How far round from the top of the skull toward its side, in radians.
    spread: f32,
    /// How far forward of the skull's top, in radians.
    forward: f32,
    /// Facets round its section.
    sides: u8,
    /// The way it sets off with no curve, or a positive one.
    rising: Vec3,
    /// The way a positive curve turns it.
    turning: Vec3,
    /// The way it sets off under a full negative curve: back over the ear.
    falling: Vec3,
    /// The way a negative curve turns it: down and round under the ear.
    curling: Vec3,
}

/// A bull's: from the corners of the forehead, out, and curving up and forward; a
/// ram's, under a negative curve: back over the ear, round it and forward under it.
/// Provenance: **tuned by render** (#402).
const PAIRED_CROWN: Crown = Crown {
    spread: 0.62,
    forward: 0.10,
    sides: PAIRED_SIDES,
    rising: Vec3::new(0.82, 0.42, 0.38),
    turning: Vec3::new(-0.2, 1.0, 0.25),
    falling: Vec3::new(0.45, 0.80, -0.40),
    curling: Vec3::new(0.05, -0.55, -1.0),
};

/// A stag's: a little nearer the midline than a horn's, up and back, the beam then
/// curving forward so the tines stand over the face.
/// Provenance: **tuned by render** (#402).
const ANTLER_CROWN: Crown = Crown {
    spread: 0.50,
    forward: 0.06,
    sides: BEAM_SIDES,
    rising: Vec3::new(0.56, 0.72, -0.40),
    turning: Vec3::new(0.05, 0.25, 1.0),
    falling: Vec3::new(0.70, 0.62, -0.35),
    curling: Vec3::new(0.2, -0.3, -1.0),
};

/// A pair on the crown, in front of the ears, whatever the ears are.
///
/// The seat is cast on the skull where [`Crown`] asks and then walked forward along
/// it until its section is clear of the ear's blade, which is read in the ear's own
/// frame: a bull's horn stands in front of a pricked ear rather than in it, however
/// big the ear. A step at a time by what is still missing, so nothing jumps as an
/// ear grows.
fn crown_seat(head: &BeastHead, piece: &Ornament, crown: Crown) -> OrnamentSeat {
    let r = head.radius;
    let centre = head.cranium.centre;
    let mut seat = shaped(head, piece, crown.sides, true);
    let cast = |forward: f32| {
        let forward = forward.min(CROWN_MOST_FORWARD);
        let toward = Vec3::new(
            crown.spread.sin() * forward.cos(),
            crown.spread.cos() * forward.cos(),
            forward.sin(),
        );
        head.leave(centre, toward).unwrap_or(centre + toward * r)
    };
    let needed = seat.radius * EAR_AIR + head.ear.cup * 0.5 * head.ear.width;
    let mut forward = crown.forward;
    for _ in 0..CROWN_WALK {
        let local = head.ear.frame.inverse() * (cast(forward) - head.ear.base);
        let beside = (local.x.abs() - 0.5 * head.ear.width).max(0.0);
        let off = Vec2::new(beside, local.z).length();
        forward += (needed - off).max(0.0) / r;
    }
    let surface = cast(forward);
    let stands = head.turn.inverse();
    let falling = smooth_share(-piece.curve / CURL_SETTLED);
    let direction = (stands * crown.rising.normalize())
        .lerp(stands * crown.falling.normalize(), falling)
        .normalize();
    let turning = if piece.curve < 0.0 {
        crown.curling
    } else {
        crown.turning
    };
    seat.direction = direction;
    seat.bend = square_to(stands * turning, direction);
    let side = direction.cross(seat.bend);
    seat.side = if side.x < 0.0 { -side } else { side };
    seat.across = seat.side;
    seat.base = surface - direction * sunk(head, seat.radius);
    // However thick, the two never meet across the middle.
    seat.base.x = seat.base.x.max(seat.radius * EAR_AIR);
    seat
}

/// How much further than an ear's blade a crown piece's section stands off it, in the
/// piece's radii. Provenance: **chosen** (#402), 15 % of air.
const EAR_AIR: f32 = 1.15;

/// How many steps a crown seat walks forward off an ear: each takes off most of what
/// the last left, as the skull's surface is nearly a ball there.
const CROWN_WALK: usize = 4;

/// The furthest forward of the skull's top a crown seat goes, in radians: past this
/// it would be on the brow over the eye.
const CROWN_MOST_FORWARD: f32 = 0.85;

/// How much negative curve it takes for a crown piece to have set off the falling way
/// altogether, in whole turns. Provenance: **tuned by render** (#402).
const CURL_SETTLED: f32 = 0.35;

/// `0` at or under zero, `1` at or over one, and an S between them.
fn smooth_share(share: f32) -> f32 {
    let share = share.clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// Where along the jaw an upper fang hangs, a lower fang and a tusk rise, as a share
/// of the way from its back to its chin: the upper behind the lower, as a canine's.
/// Provenance: **tuned by render** (#402).
const UPPER_FANG_ON_JAW: f32 = 0.68;
const LOWER_FANG_ON_JAW: f32 = 0.82;
const TUSK_ON_JAW: f32 = 0.66;

/// How much further than the jaw's side a fang's section hangs, in its radii: air
/// between them on either side of a shut mouth. Provenance: **chosen** (#402), and
/// the guard in `face::beast::adorn` asks it of the jaw as built.
const JAW_AIR: f32 = 1.15;

/// An upper fang: rooted in the upper lip and hanging from it past the shut jaw, on
/// the outside of the jaw, which the lip hangs over: the jaw is narrower than the
/// muzzle precisely so that a lip can. Where a deep jaw stands as wide as the lip, the
/// fang roots in the lip all the same and leans out by as little as clears the jaw,
/// whose side is read along the fang's whole run, however it curves. A mouth opening
/// turns the jaw in the plane the fang stands outside of.
fn upper_fang_seat(head: &BeastHead, piece: &Ornament) -> OrnamentSeat {
    let r = head.radius;
    let jaw = &head.jaw;
    let z = jaw.back.z + (jaw.front.z - jaw.back.z) * UPPER_FANG_ON_JAW;
    let mut seat = shaped(head, piece, FANG_SIDES, true);
    let (axis, _) = jaw.at(z);
    let hang = |seat: &mut OrnamentSeat, tilt: f32| {
        let direction = Vec3::new(tilt.sin(), -tilt.cos(), 0.12).normalize();
        let back = square_to(Vec3::NEG_Z, direction);
        seat.direction = direction;
        seat.bend = if piece.curve < 0.0 { -back } else { back };
        let side = direction.cross(seat.bend);
        seat.side = if side.x < 0.0 { -side } else { side };
        seat.across = seat.side;
    };
    // Off the jaw straight down, along the line in its own plane.
    hang(&mut seat, 0.0);
    seat.base = Vec3::new(0.0, axis, z);
    let line = seat.line();
    let last = (line.len() - 1) as f32;
    let straight = line
        .iter()
        .enumerate()
        .map(|(station, at)| {
            head.jaw_wide_at(at.z) + seat.radius_at(station as f32 / last) * JAW_AIR
        })
        .fold(0.0, f32::max);
    // Rooted up that line, at the first height its end is buried under the skin, so
    // that it hangs straight and no flat end shows. Where the head does not reach
    // that far out, drawn in until it does, to lean out past the jaw from there. Only
    // a very thick fang never buries its end, and bulges through the deepest it got.
    let buried = (0.8 * seat.radius).max(ROOT_DEPTH * r);
    let rise = (ROOT_RISE * r).max(2.0 * seat.radius);
    let mut x = straight;
    let mut deepest = Vec3::new(x, axis, z);
    for _ in 0..ROOT_TRIES {
        let lip = head
            .leave(Vec3::new(x, axis + r, z), Vec3::NEG_Y)
            .map_or(axis, |at| at.y);
        let heights = (0..=ROOT_SAMPLES).map(|at| lip + rise * at as f32 / ROOT_SAMPLES as f32);
        let mut found = None;
        for y in heights {
            let at = Vec3::new(x, y, z);
            let depth = head.distance(at);
            if depth < head.distance(deepest) {
                deepest = at;
            }
            if depth < -buried {
                found = Some(at);
                break;
            }
        }
        if let Some(at) = found {
            deepest = at;
            break;
        }
        x -= ROOT_STEP * r;
    }
    // Over the jaw, if drawn in far enough to be standing in it. The ring is what is
    // asked of it, so far enough over for the ring.
    let (over, girth) = jaw.at(z);
    let round = seat.radius * JAW_AIR;
    let across = deepest.x / (head.jaw_wide_at(z) + round);
    if across < 1.0 {
        let top = over + (0.9 * girth + round) * (1.0 - across * across).sqrt() * ROOT_CLEAR;
        deepest.y = deepest.y.max(top);
    }
    seat.base = deepest;
    let tilt = least_lean(
        &mut seat,
        FANG_LEAN_MOST,
        |seat, tilt| hang(seat, tilt),
        |seat| clears_the_jaw(head, seat),
    );
    hang(&mut seat, tilt);
    seat
}

/// How many heights a fang's root is tried at and over how much of a head radius, how
/// many places across, how far apart in head radii, and how far under the skin one
/// has to be to do at the least, in head radii.
const ROOT_SAMPLES: usize = 16;
const ROOT_RISE: f32 = 0.6;
const ROOT_TRIES: usize = 20;
const ROOT_STEP: f32 = 0.03;
const ROOT_DEPTH: f32 = 0.02;

/// How much further over the jaw a root drawn in over it stands, as a share of the
/// height it reads there: the ring's round is not the outer ellipse the height is
/// read off. Provenance: **chosen** (#402).
const ROOT_CLEAR: f32 = 1.2;

/// Whether a piece's line, round its section, stays out of the shut jaw: the jaw's
/// elliptical section along its run, as `face::beast::jaw` sweeps it.
fn clears_the_jaw(head: &BeastHead, seat: &OrnamentSeat) -> bool {
    let line = seat.line();
    let last = (line.len() - 1) as f32;
    let round = seat.direction.cross(seat.side);
    line.iter().enumerate().all(|(station, &at)| {
        let reach = seat.radius_at(station as f32 / last) * JAW_AIR;
        (0..JAW_RING).all(|point| {
            let angle = std::f32::consts::TAU * point as f32 / JAW_RING as f32;
            let on_ring = at + (seat.side * angle.cos() + round * angle.sin()) * reach;
            let (axis, girth) = head.jaw.at(on_ring.z);
            let across = on_ring.x / head.jaw_wide_at(on_ring.z);
            let up = (on_ring.y - axis) / (0.9 * girth);
            across * across + up * up > 1.0
        })
    })
}

/// How many points round a fang's section are asked of the jaw.
const JAW_RING: usize = 8;

/// The least lean in `0..=most` that `clear` passes: the first of a coarse walk up,
/// then halved, so that it follows what it depends on without a jump. `most`, if
/// nothing does.
fn least_lean(
    seat: &mut OrnamentSeat,
    most: f32,
    lean: impl Fn(&mut OrnamentSeat, f32),
    clear: impl Fn(&OrnamentSeat) -> bool,
) -> f32 {
    let (mut low, mut high) = (0.0, most);
    for step in 0..=LEAN_STEPS {
        let tilt = most * step as f32 / LEAN_STEPS as f32;
        lean(seat, tilt);
        if clear(seat) {
            high = tilt;
            break;
        }
        low = tilt;
    }
    for _ in 0..LEAN_HALVINGS {
        let tilt = 0.5 * (low + high);
        lean(seat, tilt);
        if clear(seat) {
            high = tilt;
        } else {
            low = tilt;
        }
    }
    high
}

/// The most an upper fang leans out from hanging straight, in radians.
const FANG_LEAN_MOST: f32 = 1.0;

/// A lower fang or a tusk: rooted in the jaw's side under the lip and leaning out by
/// as little as clears the head, the upper lip above all, as a boar's does. The lean
/// is found by asking the piece's whole line, at its section, of the surface: it
/// depends on how long it is and how it curves, so a short fang stands nearly
/// upright under the lip and a long tusk goes out round it.
fn jaw_seat(head: &BeastHead, piece: &Ornament, on_jaw: f32, sides: u8) -> OrnamentSeat {
    let jaw = &head.jaw;
    let z = jaw.back.z + (jaw.front.z - jaw.back.z) * on_jaw;
    let (axis, girth) = jaw.at(z);
    // Round the jaw's section from its side toward its underside to the first of it
    // the shut head does not cover, and rooted inside it there: the jaw is mostly
    // hidden by a flew hanging low over it, and a piece has to come out of what shows.
    let (wide, deep) = (head.jaw_wide_at(z), 0.9 * girth);
    let shows = (0..=JAW_SHOW_STEPS)
        .map(|step| {
            let round = JAW_SHOW_MOST * step as f32 / JAW_SHOW_STEPS as f32;
            Vec3::new(wide * round.cos(), axis - deep * round.sin(), z)
        })
        .find(|&on_jaw| head.distance(on_jaw) > 0.0);
    let mut seat = shaped(head, piece, sides, true);
    // On a snub head none of it may show, and then it comes out of the cheek by the
    // jaw, which it still rides.
    seat.base = shows.map_or_else(
        || {
            let low = Vec3::new(0.0, axis - deep, z);
            let side = head.leave(low, Vec3::X).unwrap_or(low);
            side - Vec3::X * seat.radius
        },
        |shows| Vec3::new(0.0, axis, z).lerp(shows, JAW_ROOT),
    );
    let lean = |seat: &mut OrnamentSeat, tilt: f32| {
        let direction = Vec3::new(tilt.sin(), tilt.cos(), 0.10).normalize();
        let back = square_to(Vec3::new(0.0, 0.6, -0.8), direction);
        let forward = square_to(Vec3::new(0.0, 0.6, 0.8), direction);
        seat.direction = direction;
        seat.bend = if piece.curve < 0.0 { forward } else { back };
        let side = direction.cross(seat.bend);
        seat.side = if side.x < 0.0 { -side } else { side };
        seat.across = seat.side;
    };
    // Out of the head by half way, and once out it stays out: its root may grow
    // through a flew, and nothing further along may go back in.
    let clear = |seat: &OrnamentSeat| {
        let line = seat.line();
        let last = (line.len() - 1) as f32;
        let out = |station: usize| {
            let round = seat.radius_at(station as f32 / last);
            [seat.side, -seat.side, Vec3::Y, Vec3::NEG_Y, Vec3::ZERO]
                .into_iter()
                .all(|off| head.distance(line[station] + off * round) > 0.0)
        };
        (0..line.len())
            .position(out)
            .is_some_and(|first| first * 2 <= line.len() && (first..line.len()).all(out))
    };
    let tilt = least_lean(&mut seat, LEAN_MOST, |seat, tilt| lean(seat, tilt), clear);
    lean(&mut seat, tilt);
    // Past where a snub head's cheek swallows the whole of a short one, slid out along
    // itself until it shows, which a body with any face to speak of never needs.
    for _ in 0..SLIDE_STEPS {
        if clear(&seat) {
            break;
        }
        seat.base += seat.direction * (SLIDE_STEP * head.radius);
    }
    seat
}

/// How many steps a piece a cheek would swallow slides out, and how far each is in
/// head radii.
const SLIDE_STEPS: usize = 16;
const SLIDE_STEP: f32 = 0.05;

/// How far from the jaw's axis toward its surface a jaw piece roots: inside it, near
/// its wall. Provenance: **tuned by render** (#402).
const JAW_ROOT: f32 = 0.75;

/// How far round the jaw's section a piece's root is looked for, in radians from its
/// side, and in how many steps.
const JAW_SHOW_MOST: f32 = 1.4;
const JAW_SHOW_STEPS: usize = 14;

/// The most a jaw piece leans out from upright, in radians: nearly level, from the
/// side of the jaw.
const LEAN_MOST: f32 = 1.45;

/// How many steps the walk up to a clearing lean takes, and how many times what it
/// finds is halved: to under a tenth of a degree.
const LEAN_STEPS: usize = 12;
const LEAN_HALVINGS: usize = 7;

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
