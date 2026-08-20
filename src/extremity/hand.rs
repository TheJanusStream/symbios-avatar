//! Hands.
//!
//! A hand is built as its own part, attached at the wrist, rather than as more
//! nodes in the capsule graph. A palm carrying five digits would need a node
//! with fifteen sockets on it, and joint hulls are the one thing the B-Mesh
//! construction is worst at — the sockets have to clear each other on a sphere,
//! and fifteen of them will not. It would also give every creature without
//! fingers a rig full of joints it does not use.
//!
//! **One welded solid, not six** (#297/#298). The first hand was a palm sweep
//! with five closed digit sweeps appended to it, and every boundary between
//! those solids rendered: the palm's base collared the wrist, the finger roots
//! sank into the palm's end cap with a shadow line, and the thumb overlapped
//! the palm's side with a crease. This build is the box-modeller's instead: a
//! lofted palm cage whose distal end is four quads, each **extruded** into a
//! finger, with the thumb extruded from a side wall — so the fingers are
//! continuous with the palm by construction, there is no interior geometry,
//! and one Catmull–Clark pass rounds the whole cage into one smooth surface.
//!
//! The shape aims at what reads, not at anatomy. At the distance a game is
//! played from, a hand is a flat palm, four fingers of unequal length, and a
//! thumb set well down the side and pointing across the others. Getting those
//! four things right is most of the recognition; knuckle creases are not.
//!
//! Digits rest in a slight curl. A hand held perfectly flat reads as a
//! surrender, or as a mannequin — the relaxed hand has its fingers curved even
//! when nothing is being held.

use glam::{Vec2, Vec3};

use crate::catmull_clark;
use crate::mesh::PolyMesh;

/// How a digit's length divides between its phalanges, proximal first.
///
/// **Three, because a finger has three, and this is a rig as well as a
/// shape.** Four equal segments chosen for how smooth the curl looked would
/// not do: the reference divides every finger into three unequal ones, and
/// its bones sit on those divisions. Measured off the
/// Quaternius male, as fractions of each digit's own length:
///
/// ```text
///   index   0.419  0.322  0.260
///   middle  0.403  0.304  0.294
///   ring    0.394  0.306  0.300
///   pinky   0.390  0.311  0.299
///   thumb   0.362  0.367  0.271
/// ```
///
/// The four fingers agree closely enough to share one set; the thumb does not,
/// because its two long segments are nearly equal where a finger's taper.
///
/// Provenance: **looked up**, from the reference's own bone positions.
const PHALANGES: [f32; 3] = [0.402, 0.311, 0.287];
/// See [`PHALANGES`]. The thumb's own division, which is not a finger's.
const THUMB_PHALANGES: [f32; 3] = [0.362, 0.367, 0.271];

/// How many segments each digit is swept from, and so how many bones it has.
///
/// One per phalanx, so a ring of the extrusion and a joint of the rig are the
/// same thing and cannot drift apart.
pub const JOINTS: usize = PHALANGES.len();

/// Bones per hand: the wrist, then one per phalanx of each of five digits.
///
/// Twenty-one, which is what both reference hands carry — `hand_l`, then
/// `<digit>_01`, `_02`, `_03` and `_04_leaf` for thumb, index, middle, ring and
/// pinky. [`Hand::influences`] indexes into this order.
pub const BONES: usize = 1 + 5 * (JOINTS + 1);

/// Vertices around one palm ring: five columns across the back, five across
/// the palm.
///
/// Ten, because the distal ring's own column structure is what the fingers
/// grow from: consecutive column pairs tile the palm's end into exactly four
/// quads, one finger root each, and the shared columns between them are the
/// finger webs. A ring count that did not pair its columns top-to-bottom
/// would leave no quads to extrude.
const RING: usize = 10;

/// Where each finger's knuckle stands, and how long it is — all relative to
/// the middle finger.
///
/// **The knuckles are not in a line.** The
/// reference's four sit at 10.98, 10.91, 10.07 and 9.06 cm out from the wrist,
/// so the little finger's is nearly two centimetres nearer in than the index's,
/// and the row curves. Built on one straight line the four roots read as a
/// comb's spine however well the fingers themselves are proportioned.
///
/// Where each finger sits ACROSS the palm is no longer stored: the roots are
/// the distal ring's own quads now, so the spacing is the column spacing and
/// cannot disagree with the palm's width.
///
/// Reach from the wrist, which is what the eye actually reads, comes out at
/// 0.939, 1.000, 0.930 and 0.829 of the middle finger's on the reference.
///
/// Provenance: **looked up**; index first, pinky last.
const FINGERS: [Finger; 4] = [
    Finger {
        knuckle: 1.000,
        length: 0.869,
    },
    Finger {
        knuckle: 0.994,
        length: 1.000,
    },
    Finger {
        knuckle: 0.917,
        length: 0.936,
    },
    Finger {
        knuckle: 0.825,
        length: 0.829,
    },
];

/// One finger's placement, in the units [`FINGERS`] documents.
struct Finger {
    /// How far out the knuckle stands, of the furthest knuckle's set-back.
    knuckle: f32,
    /// Length, of the middle finger's.
    length: f32,
}

/// A built hand, in wrist-local space: one welded, subdivided solid.
#[derive(Clone, Debug, PartialEq)]
pub struct Hand {
    /// The whole hand as one closed surface.
    pub mesh: PolyMesh,
    /// Each digit's joint chain: knuckle, then one joint per phalanx boundary,
    /// ending at the tip. Index, middle, ring, pinky, then the thumb —
    /// [`JOINTS`] + 1 points each, and they are the **stations of the
    /// extrusion** rather than a second set of numbers that happens to agree
    /// with it, so a bone cannot drift away from the surface it bends. The
    /// last is a leaf, as the reference's `_04_leaf` is.
    pub digits: Vec<Vec<Vec3>>,
    /// How long the hand is from wrist to fingertip, in metres.
    pub length: f32,
    /// Each digit's root half-width, which is the envelope
    /// [`Hand::influences`] classifies vertices against.
    reaches: Vec<f32>,
}

/// One station of the palm loft.
struct Station {
    /// Distance along `out` from the wrist crease, per column — the knuckle
    /// row curves, so the distal station is not planar.
    along: [f32; 5],
    /// Half-width across the palm.
    wide: f32,
    /// Half-depth through it.
    deep: f32,
    /// How far the section has squared up from an ellipse toward a slab.
    flat: f32,
}

impl Hand {
    /// Builds a hand for a wrist of the given measured radius.
    ///
    /// `out` points along the forearm, away from the body; `up` is the back of
    /// the hand.
    ///
    /// **This builds one chirality, and handing it the other arm's direction
    /// does not produce the other hand.** A hand is chiral — the thumb is
    /// on one particular edge of the palm and no rotation moves it to the other
    /// — and everything here is derived from a frame that turns with `out`:
    /// `across` is `out × up`, and the thumb is seated at `-across`. Feed it a
    /// reversed `out` and the whole construction rotates half a turn, thumb
    /// included, so what comes back is the *same* hand pointing the other way.
    /// See [`crate::extremity`] for how the other hand is actually made.
    ///
    /// Which chirality comes out follows from `across = out × up`: with `up`
    /// along world Y, a hand built with `out.x` negative seats its thumb toward
    /// `+Z`, which is the body's front, and that is where both of the
    /// reference's thumbs are.
    ///
    /// **Sized by `length`, matched to the arm by `wrist`, and the split is
    /// the foot's lesson** (#110) arriving at the hand: sizing a part off the
    /// girth of the node it hangs from couples its whole scale to a radius
    /// tuned for other reasons — slimming the wrist stub shrank the first
    /// welded hand by a third. The length is the body's to derive (a hand is a
    /// share of stature); the wrist girth decides only how wide the base ring
    /// must be to emerge from the arm without a step.
    #[must_use]
    pub fn build(wrist: f32, out: Vec3, up: Vec3, curl: f32, length: f32) -> Self {
        let out = out.normalize_or(Vec3::X);
        let up = (up - out * up.dot(out)).normalize_or(Vec3::Y);
        let across = out.cross(up);

        // Proportions within the hand. A palm is much flatter than it is
        // wide, and the fingers carry a bit under half the length — a short
        // palm gives a paw.
        let palm_length = length / 1.95;
        let palm_width = palm_length * 0.50;
        let palm_depth = palm_length * 0.17;
        let finger_length = palm_length * 0.95;
        // The base ring matches the ARM: snug against its measured surface at
        // the crease so the palm emerges from inside it at a tangent, capped
        // to the palm's own width for a body whose forearm outweighs its hand.
        let base = (wrist * 0.96).min(palm_width * 0.92);

        // The knuckle row's set-back per column. Edge columns take their own
        // finger's; a web column between two fingers takes their mean, which
        // is what makes the row a curve rather than a comb's spine.
        let knuckle = |edge: f32| -> f32 {
            let row = 0.94 * palm_length;
            match edge {
                e if e < -0.75 => row * FINGERS[0].knuckle,
                e if e < -0.25 => row * (FINGERS[0].knuckle + FINGERS[1].knuckle) * 0.5,
                e if e < 0.25 => row * (FINGERS[1].knuckle + FINGERS[2].knuckle) * 0.5,
                e if e < 0.75 => row * (FINGERS[2].knuckle + FINGERS[3].knuckle) * 0.5,
                _ => row * FINGERS[3].knuckle,
            }
        };
        // Column order matches [`columns`]: -1, -0.5, 0, +0.5, +1.
        let staggered = [
            knuckle(-1.0),
            knuckle(-0.5),
            knuckle(0.0),
            knuckle(0.5),
            knuckle(1.0),
        ];

        // Four stations: a round base tucked inside the forearm, an easing
        // ring, the palm proper, and the curved knuckle row. The base stays
        // ROUND and slightly under the wrist's own girth so the surface
        // emerges from inside the arm at a tangent instead of collaring it.
        // The first station is the WELD RING (#297): the hand is left open
        // here, and `extremity::weld` bridges this boundary to the hole cut in
        // the arm, so the two are one surface and there is no junction to
        // disguise. It sits a whisker behind the crease at the arm's own
        // measured girth — the bridge triangles then span millimetres and lie
        // almost in the surface. The buried-cap-and-proud-cuff arrangement
        // this replaces is kept in the history: it could only ever HIDE the
        // seam, and the owner asked for no seam at all.
        let stations = [
            Station {
                along: [-base * 0.10; 5],
                wide: base,
                deep: base,
                flat: 0.0,
            },
            Station {
                along: [base * 0.45; 5],
                wide: base * 1.02,
                deep: base * 0.98,
                flat: 0.10,
            },
            Station {
                along: [palm_length * 0.30; 5],
                wide: palm_width * 0.82,
                deep: base.max(palm_depth * 1.3) * 0.80,
                flat: 0.35,
            },
            Station {
                along: [palm_length * 0.62; 5],
                wide: palm_width * 0.95,
                deep: palm_depth * 1.12,
                flat: 0.70,
            },
            Station {
                along: staggered,
                wide: palm_width * 0.96,
                deep: palm_depth * 0.95,
                flat: 0.85,
            },
        ];

        let mut cage = PolyMesh::new();
        let rings: Vec<Vec<u32>> = stations
            .iter()
            .map(|station| {
                ring_points(station, out, up, across)
                    .into_iter()
                    .map(|point| cage.push_vertex(point))
                    .collect()
            })
            .collect();

        // The loft walls, in `prim::sweep`'s own winding, with one wall held
        // back: the quad on the thumb's side of the palm, between the palm's
        // two mid stations, is the thumb's root, and pushing it would seal
        // the thumb off from the hand it grows out of.
        const THUMB_WALL: (usize, usize) = (2, 4);
        for (band, pair) in rings.windows(2).enumerate() {
            for side in 0..RING {
                if band == THUMB_WALL.0 && side == THUMB_WALL.1 {
                    continue;
                }
                let next = (side + 1) % RING;
                cage.push_face([pair[0][side], pair[0][next], pair[1][next], pair[1][side]]);
            }
        }
        // No base cap: the proximal boundary is the weld ring, and capping it
        // would seal the hand back into a separate solid. Catmull–Clark's
        // boundary rules carry the open ring through subdivision.

        // The distal ring's columns tile its end into four quads — see
        // [`RING`] — and each is a finger's root. Corners run web-side-down,
        // outer-down, outer-up, web-side-up around each root, whichever side
        // of the palm the finger sits on, because the signs are read off the
        // roots themselves rather than assumed.
        let distal = &rings[rings.len() - 1];
        let mut digits = Vec::with_capacity(5);
        let mut reaches = Vec::with_capacity(5);
        for (finger, spec) in FINGERS.iter().enumerate() {
            let root = [
                distal[3 - finger],
                distal[4 - finger],
                distal[(5 + finger) % RING],
                distal[6 + finger],
            ];
            let joints = extrude_digit(
                &mut cage,
                root,
                out,
                up,
                across,
                finger_length * spec.length,
                &PHALANGES,
                curl,
            );
            reaches.push(root_reach(&cage, root));
            digits.push(joints);
        }

        // The thumb is the whole difference between a hand and a paddle. It
        // sits low on the palm's side, points across rather than along, and is
        // thicker and shorter than any finger — its root is the held-back side
        // wall, so it is as continuous with the palm as the fingers are.
        //
        // Angled well forward, not straight out to the side: a thumb held
        // square to the palm reaches further across than the hand is long,
        // which reads as a claw.
        let wall = [
            rings[THUMB_WALL.0][THUMB_WALL.1],
            rings[THUMB_WALL.0][(THUMB_WALL.1 + 1) % RING],
            rings[THUMB_WALL.0 + 1][(THUMB_WALL.1 + 1) % RING],
            rings[THUMB_WALL.0 + 1][THUMB_WALL.1],
        ];
        let thumb = extrude_digit(
            &mut cage,
            [wall[3], wall[0], wall[1], wall[2]],
            (out * 0.78 - across * 0.63).normalize(),
            up,
            across,
            finger_length * 0.66,
            &THUMB_PHALANGES,
            curl * 0.55,
        );
        reaches.push(root_reach(&cage, wall));
        digits.push(thumb);

        // One Catmull–Clark pass turns the box cage into the hand: fingers
        // round, the palm's slab softens, the webs fillet themselves — and
        // because the cage is one closed solid, so is the result. The joints
        // are NOT subdivided with it: a chain of points has no faces to
        // average, and the rings were placed on the joints deliberately.
        let mut mesh = catmull_clark(&cage, 1);
        planar_uvs(&mut mesh, out, across);

        Self {
            mesh,
            digits,
            length: palm_length + finger_length,
            reaches,
        }
    }

    /// Which of the hand's own bones holds each vertex of [`Hand::mesh`].
    ///
    /// Bones are numbered as the reference names them: `0` is the wrist, and
    /// digit `d`'s phalanx `j` is `1 + d * (JOINTS + 1) + j`, for [`BONES`] in
    /// all. A caller that has attached those bones to a rig maps the numbers
    /// through its own list; nothing here needs to know what a rig is.
    ///
    /// **Classified geometrically, because the welded mesh has no per-digit
    /// vertex ranges any more.** A vertex belongs to the digit whose joint
    /// chain it sits nearest, if that chain is within the digit's own root
    /// reach; everything else is palm and belongs to the wrist. The weights
    /// along a digit are shared between neighbouring phalanges exactly as
    /// before: each ring sits ON a joint, so binding a ring to one bone would
    /// hinge the surface at the only places it has geometry, and a curling
    /// finger would come out as three rigid tubes. The rule for which bone
    /// owns which phalanx is the crate's own: the joint at a bone's
    /// **proximal** end turns it (see [`crate::rig::skin`]), so the knuckle
    /// drives the first phalanx and the tip drives nothing.
    #[must_use]
    pub fn influences(&self) -> Vec<[(usize, f32); 2]> {
        const WRIST: usize = 0;
        self.mesh
            .positions
            .iter()
            .map(|&point| {
                let mut nearest = (f32::MAX, 0usize, 0.0f32);
                for (digit, joints) in self.digits.iter().enumerate() {
                    let (off, along) = station(joints, point);
                    if off < nearest.0 {
                        nearest = (off, digit, along);
                    }
                }
                let (off, digit, along) = nearest;
                // Everything beyond a digit's own reach of its chain is palm.
                // The margin is generous because the classification only has
                // to be right ABOUT THE SPLIT — a palm vertex misread as a
                // knuckle at station zero earns half a share of the first
                // phalanx, which is what knuckle skin should do anyway.
                if off > self.reaches[digit] * 2.2 {
                    return [(WRIST, 1.0), (WRIST, 0.0)];
                }
                let base = 1 + digit * (JOINTS + 1);
                let bone = (along.floor() as usize).min(JOINTS - 1);
                let within = along - bone as f32;
                // Half of a ring's hold goes to the phalanx it starts and half
                // to the one it ends, so the two share the fold between them.
                // The first ring shares with the wrist, which is what lets a
                // knuckle bend at all.
                if within < 0.5 {
                    let before = if bone == 0 { WRIST } else { base + bone - 1 };
                    [(before, 0.5 - within), (base + bone, 0.5 + within)]
                } else {
                    [(base + bone, 1.5 - within), (base + bone + 1, within - 0.5)]
                }
            })
            .collect()
    }
}

/// Where along a digit's chain a point sits, in phalanges from the knuckle,
/// and how far off the chain it stands.
///
/// Runs `0.0` at the knuckle to [`JOINTS`] at the tip. Found by projecting
/// onto each segment in turn and keeping the nearest, rather than by
/// distance to the joints themselves: a fat ring around a short phalanx is
/// nearer the *next* joint than its own, and reading it that way would bind
/// the base of a finger to its fingertip.
fn station(joints: &[Vec3], point: Vec3) -> (f32, f32) {
    let mut best = (f32::MAX, 0.0f32);
    for (index, pair) in joints.windows(2).enumerate() {
        let axis = pair[1] - pair[0];
        let span = axis.length_squared();
        let along = if span <= f32::EPSILON {
            0.0
        } else {
            ((point - pair[0]).dot(axis) / span).clamp(0.0, 1.0)
        };
        let off = point.distance(pair[0] + axis * along);
        if off < best.0 {
            best = (off, index as f32 + along);
        }
    }
    best
}

/// One ring of the palm loft.
///
/// Ten points, ordered the way `prim::sweep` orders a ring so every winding
/// downstream matches: the angle runs from the pinky's side of the palm down
/// under and back over the top. The across positions ease from the ellipse's
/// own cosines toward five EVEN columns as `flat` rises, because the distal
/// ring's columns are the finger roots and cosine spacing would give the
/// index and pinky roots two thirds the width of the middle two.
fn ring_points(station: &Station, out: Vec3, up: Vec3, across: Vec3) -> Vec<Vec3> {
    (0..RING)
        .map(|side| {
            let angle = (18.0 + 36.0 * side as f32).to_radians();
            let (sin, cos) = angle.sin_cos();
            let even = if cos.abs() > 0.8 {
                cos.signum()
            } else if cos.abs() > 0.3 {
                cos.signum() * 0.5
            } else {
                0.0
            };
            let sideways = cos + (even - cos) * station.flat;
            let vertical = -sin.signum() * (sin.abs() + (1.0 - sin.abs()) * station.flat);
            let column = ((even + 1.0) * 2.0).round() as usize;
            out * station.along[column.min(4)]
                + across * (sideways * station.wide)
                + up * (vertical * station.deep)
        })
        .collect()
}

/// Extrudes one digit from a root quad of the cage and returns its joints.
///
/// The root's four vertices stay exactly what they are — faces of the palm —
/// which is the whole point: the digit is continuous with the hand because it
/// shares those vertices, not because anything was sealed afterwards. Each
/// following ring is a tapered quad in the frame the curl carries along, and
/// the corner signs are read off the root itself so the tube cannot twist
/// whichever way round the root was handed over.
#[allow(clippy::too_many_arguments)]
fn extrude_digit(
    cage: &mut PolyMesh,
    root: [u32; 4],
    along: Vec3,
    up: Vec3,
    across: Vec3,
    length: f32,
    phalanges: &[f32; JOINTS],
    curl: f32,
) -> Vec<Vec3> {
    let centre = root
        .iter()
        .fold(Vec3::ZERO, |sum, &v| sum + cage.positions[v as usize])
        / 4.0;

    // Curl bends the digit toward the palm, a little more at each joint, the
    // way a finger closes from the tip inward.
    let bend = curl * 0.55;
    let mut path = Vec::with_capacity(JOINTS + 1);
    let mut at = centre;
    let mut direction = along;
    path.push(at);
    for (joint, share) in phalanges.iter().enumerate() {
        let turn = bend * (joint as f32 + 1.0) / JOINTS as f32;
        direction = (direction * turn.cos() - up * turn.sin()).normalize_or(direction);
        at += direction * (length * share);
        path.push(at);
    }

    // The root's own half-extents and corner signs, in the digit's lateral
    // frame. `side` is whichever of `across` is left once the digit's own
    // direction is removed, so a thumb leaving at an angle still measures its
    // root in its own plane.
    let first = (path[1] - path[0]).normalize_or(along);
    let side = (across - first * across.dot(first)).normalize_or(across);
    let rise = first.cross(side);
    let mut wide = 0.0f32;
    let mut deep = 0.0f32;
    let signs: Vec<(f32, f32)> = root
        .iter()
        .map(|&v| {
            let offset = cage.positions[v as usize] - centre;
            let s = offset.dot(side);
            let r = offset.dot(rise);
            wide = wide.max(s.abs());
            deep = deep.max(r.abs());
            (s.signum(), r.signum())
        })
        .collect();

    // Tapering by how far along the digit each ring actually is, not by which
    // number it is: the phalanges are of unequal length, so counting rings
    // would step the taper unevenly and pinch the shortest one. Not to a
    // point — a fingertip is rounded, and a cone reads as a claw.
    let mut travelled = 0.0;
    let mut rings: Vec<[u32; 4]> = vec![root];
    for (joint, share) in phalanges.iter().enumerate() {
        travelled += share;
        let taper = 1.0 - 0.26 * travelled;
        let segment = (path[joint + 1] - path[joint]).normalize_or(first);
        let side = (across - segment * across.dot(segment)).normalize_or(side);
        let rise = segment.cross(side);
        let ring: Vec<u32> = signs
            .iter()
            .map(|&(s, r)| {
                cage.push_vertex(
                    path[joint + 1] + side * (s * wide * taper) + rise * (r * deep * taper),
                )
            })
            .collect();
        rings.push([ring[0], ring[1], ring[2], ring[3]]);
    }

    for pair in rings.windows(2) {
        for corner in 0..4 {
            let next = (corner + 1) % 4;
            cage.push_face([
                pair[0][corner],
                pair[0][next],
                pair[1][next],
                pair[1][corner],
            ]);
        }
    }
    let tip = rings[rings.len() - 1];
    cage.push_face(tip.to_vec());

    path
}

/// A root quad's larger half-extent, which is the digit's claim radius for
/// [`Hand::influences`].
fn root_reach(cage: &PolyMesh, root: [u32; 4]) -> f32 {
    let centre = root
        .iter()
        .fold(Vec3::ZERO, |sum, &v| sum + cage.positions[v as usize])
        / 4.0;
    root.iter()
        .map(|&v| cage.positions[v as usize].distance(centre))
        .fold(0.0f32, f32::max)
}

/// Charts the hand by projection onto the back-of-hand plane.
///
/// Subdivision carries positions and nothing else, so the cage's texture
/// coordinates do not survive it and the finished surface is charted here
/// instead. Palm and back share texels under this projection, which is what
/// the old tube charts did around every ring; skin at this distance is a
/// complexion, not a print.
fn planar_uvs(mesh: &mut PolyMesh, out: Vec3, across: Vec3) {
    let (lo, hi) = mesh.bounds();
    let span = (hi - lo).max(Vec3::splat(1e-6));
    let scale = |value: f32, low: f32, range: f32| ((value - low) / range).clamp(0.0, 1.0);
    mesh.uvs = mesh
        .positions
        .iter()
        .map(|&point| {
            Vec2::new(
                scale(point.dot(across), lo.dot(across).min(hi.dot(across)), {
                    let a = span.dot(across.abs()).abs();
                    if a <= 1e-6 { 1.0 } else { a }
                }),
                scale(point.dot(out), lo.dot(out).min(hi.dot(out)), {
                    let a = span.dot(out.abs()).abs();
                    if a <= 1e-6 { 1.0 } else { a }
                }),
            )
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand(curl: f32) -> Hand {
        Hand::build(0.025, Vec3::X, Vec3::Y, curl, 0.185)
    }

    /// Signed volume by the divergence theorem: positive for a closed surface
    /// wound outward.
    fn signed_volume(mesh: &PolyMesh) -> f32 {
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

    #[test]
    fn a_hand_is_one_solid_open_only_at_the_weld_ring() {
        // **The whole point of the rebuild** (#298): the first hand was six
        // closed solids and every boundary between them rendered. The one
        // boundary left is deliberate — the proximal ring `extremity::weld`
        // bridges into the arm (#297) — and everything else is watertight.
        let hand = hand(0.3);
        let report = hand.mesh.manifold_report();
        assert_eq!(report.nonmanifold_edges, 0, "{report:?}");
        assert_eq!(report.inconsistent_edges, 0, "{report:?}");
        assert_eq!(report.degenerate_faces, 0, "{report:?}");
        assert_eq!(report.out_of_range, 0, "{report:?}");
        let ring = boundary_ring(&hand.mesh);
        assert_eq!(
            report.boundary_edges,
            ring.len(),
            "boundary beyond the weld ring: {report:?}"
        );
        // Capped virtually at the ring's own centre, the solid is wound
        // outward — the same check a closed mesh gets, paid one fan.
        let mut capped = hand.mesh.clone();
        let centre = ring
            .iter()
            .fold(Vec3::ZERO, |sum, &v| sum + capped.positions[v as usize])
            / ring.len() as f32;
        let hub = capped.push_vertex(centre);
        // The ring was walked in the CAP's direction — `boundary_ring`
        // follows each boundary edge reversed — so the fan uses it as is.
        for pair in ring.windows(2) {
            capped.push_face([pair[0], pair[1], hub]);
        }
        capped.push_face([ring[ring.len() - 1], ring[0], hub]);
        assert!(
            capped.is_closed_manifold(),
            "the weld ring is not one closed loop: {:?}",
            capped.manifold_report()
        );
        assert!(signed_volume(&capped) > 0.0, "the hand is wound inside out");
    }

    /// The open boundary as one ordered loop of vertex indices.
    fn boundary_ring(mesh: &PolyMesh) -> Vec<u32> {
        use std::collections::BTreeMap;
        let mut forward: BTreeMap<u32, u32> = BTreeMap::new();
        let mut count = std::collections::BTreeMap::new();
        for face in &mesh.faces {
            for (index, &a) in face.iter().enumerate() {
                let b = face[(index + 1) % face.len()];
                let key = (a.min(b), a.max(b));
                *count.entry(key).or_insert(0usize) += 1;
            }
        }
        for face in &mesh.faces {
            for (index, &a) in face.iter().enumerate() {
                let b = face[(index + 1) % face.len()];
                if count[&(a.min(b), a.max(b))] == 1 {
                    // A boundary edge a→b in an outward face: the hole walks
                    // b→a.
                    forward.insert(b, a);
                }
            }
        }
        let Some((&start, _)) = forward.iter().next() else {
            return Vec::new();
        };
        let mut ring = vec![start];
        let mut at = forward[&start];
        while at != start {
            ring.push(at);
            at = forward[&at];
        }
        ring
    }

    #[test]
    fn a_hand_is_one_connected_surface() {
        // Welded means welded: every vertex reachable from every other along
        // edges. Six appended solids pass a manifold check — each is closed —
        // and fail this one.
        let hand = hand(0.3);
        let count = hand.mesh.vertex_count();
        let mut adjacency = vec![Vec::new(); count];
        for face in &hand.mesh.faces {
            for (index, &vertex) in face.iter().enumerate() {
                let next = face[(index + 1) % face.len()];
                adjacency[vertex as usize].push(next as usize);
                adjacency[next as usize].push(vertex as usize);
            }
        }
        let mut seen = vec![false; count];
        let mut stack = vec![0usize];
        seen[0] = true;
        while let Some(vertex) = stack.pop() {
            for &other in &adjacency[vertex] {
                if !seen[other] {
                    seen[other] = true;
                    stack.push(other);
                }
            }
        }
        let reached = seen.iter().filter(|&&s| s).count();
        assert_eq!(
            reached, count,
            "{} of {count} vertices reachable from the first — the hand is \
             still more than one piece",
            reached
        );
    }

    #[test]
    fn a_hand_has_five_digit_chains_of_the_rig_layout() {
        let hand = hand(0.3);
        assert_eq!(hand.digits.len(), 5);
        for joints in &hand.digits {
            assert_eq!(joints.len(), JOINTS + 1);
        }
        assert_eq!(BONES, 21);
    }

    #[test]
    fn a_hand_reaches_out_along_the_arm() {
        let hand = hand(0.0);
        let (lo, hi) = hand.mesh.bounds();
        assert!(hi.x > 0.0, "the hand did not extend along +X");
        assert!(lo.x > -0.03, "the hand reached back up the arm");
        // Longer than it is wide, as a hand is.
        assert!(hi.x - lo.x > hi.z - lo.z);
    }

    #[test]
    fn a_palm_is_flatter_than_it_is_wide() {
        // Measured across the mid-palm, not over the whole solid: the palm's
        // base is deliberately round, to match the forearm it emerges from,
        // and that round end would otherwise set the depth.
        let hand = hand(0.0);
        let knuckles = hand.digits[0][0].x.min(hand.digits[3][0].x);
        let mid: Vec<&Vec3> = hand
            .mesh
            .positions
            .iter()
            .filter(|p| p.x > knuckles * 0.55 && p.x < knuckles * 0.9)
            .collect();
        let across = mid.iter().map(|p| p.z.abs()).fold(0.0f32, f32::max);
        let through = mid.iter().map(|p| p.y.abs()).fold(0.0f32, f32::max);
        assert!(
            across > through * 1.4,
            "palm measured {across} across and {through} through"
        );
    }

    #[test]
    fn fingers_are_of_unequal_length() {
        // A comb reads as a comb. The stagger across the knuckles is much of
        // what makes a hand recognisable.
        let hand = hand(0.0);
        let reach: Vec<f32> = hand.digits[..4]
            .iter()
            .map(|joints| joints[JOINTS].x)
            .collect();
        let longest = reach.iter().fold(0.0f32, |a, b| a.max(*b));
        let shortest = reach.iter().fold(f32::MAX, |a, b| a.min(*b));
        assert!(
            longest > shortest * 1.08,
            "fingers reached between {shortest} and {longest}"
        );
    }

    #[test]
    fn the_knuckle_row_curves() {
        // The index knuckle stands further out than the pinky's by nearly two
        // centimetres on the reference; a straight row reads as a comb.
        let hand = hand(0.0);
        assert!(
            hand.digits[0][0].x > hand.digits[3][0].x * 1.1,
            "the knuckle row is flat: index at {}, pinky at {}",
            hand.digits[0][0].x,
            hand.digits[3][0].x
        );
    }

    #[test]
    fn the_thumb_sits_across_the_palm_not_along_it() {
        let hand = hand(0.0);
        let thumb = &hand.digits[4];
        let middle = &hand.digits[1];
        // It reaches further to one side and not as far forward.
        assert!(
            thumb[JOINTS].z < middle[JOINTS].z,
            "the thumb did not sit to the side"
        );
        assert!(
            thumb[JOINTS].x < middle[JOINTS].x,
            "the thumb reached as far as a finger"
        );
    }

    #[test]
    fn curling_closes_the_fingers_toward_the_palm() {
        let flat = hand(0.0).mesh.bounds();
        let closed = hand(1.0).mesh.bounds();
        assert!(
            closed.1.x < flat.1.x,
            "a curled hand should not reach as far: {} against {}",
            closed.1.x,
            flat.1.x
        );
        assert!(closed.0.y < flat.0.y, "curling should dip below the palm");
    }

    #[test]
    fn reversing_the_arm_rotates_the_hand_rather_than_reflecting_it() {
        // The trap this test used to fall into (#113). It asserted that the two
        // builds agreed on their *x* bounds and called that mirroring — which a
        // half-turn about Y satisfies exactly as well as a reflection does, so
        // the check passed on a body wearing two right hands.
        //
        // What separates the two is the THUMB, because that is the only part of
        // a hand that knows which one it is. Under a reflection across the
        // sagittal plane the thumb keeps its `z`; under the half-turn this
        // actually performs, `z` flips with everything else.
        let one = Hand::build(0.025, Vec3::X, Vec3::Y, 0.3, 0.185);
        let other = Hand::build(0.025, -Vec3::X, Vec3::Y, 0.3, 0.185);

        let (rlo, rhi) = one.mesh.bounds();
        let (llo, lhi) = other.mesh.bounds();
        assert!((rhi.x + llo.x).abs() < 1e-5, "reach did not turn about Y");
        assert!((rlo.x + lhi.x).abs() < 1e-5, "the wrist end did not turn");

        // And here is the defect, stated so it cannot come back: the thumbs
        // end up on opposite sides of the body's fore-aft axis, which is two
        // of the same hand.
        assert!(
            one.digits[4][JOINTS].z * other.digits[4][JOINTS].z < 0.0,
            "the thumbs sat on the same side of z, so this is a reflection \
             after all and the caller no longer needs to make one"
        );
    }

    #[test]
    fn a_hand_built_along_negative_x_puts_its_thumb_forward() {
        // Which chirality `build` makes, pinned to the one fact that decides
        // it: `across` is `out × up`, the thumb sits at `-across`, so with
        // `up` on world Y an `out` with negative x seats the thumb toward +Z.
        // That is the body's front, and it is where both of the reference's
        // thumbs are.
        let hand = Hand::build(0.025, -Vec3::X, Vec3::Y, 0.0, 0.185);
        assert!(
            hand.digits[4][JOINTS].z > hand.digits[1][JOINTS].z,
            "the thumb reached to z {} against the middle finger's {}",
            hand.digits[4][JOINTS].z,
            hand.digits[1][JOINTS].z
        );
    }

    #[test]
    fn a_hand_scales_with_the_length_it_is_asked_for_not_the_wrist() {
        // The foot's lesson (#110): the node a part hangs from is tuned for
        // other reasons, and a part sized off it inherits every one of them.
        let small = Hand::build(0.025, Vec3::X, Vec3::Y, 0.3, 0.15);
        let large = Hand::build(0.025, Vec3::X, Vec3::Y, 0.3, 0.30);
        assert!((large.length / small.length - 2.0).abs() < 1e-4);
        let thick = Hand::build(0.04, Vec3::X, Vec3::Y, 0.3, 0.15);
        assert!((thick.length - small.length).abs() < 1e-6);
    }

    #[test]
    fn every_vertex_is_held_and_no_weight_leaves_the_hand() {
        let hand = hand(0.3);
        let influences = hand.influences();
        assert_eq!(influences.len(), hand.mesh.vertex_count());
        for shares in &influences {
            let total: f32 = shares.iter().map(|(_, w)| w).sum();
            assert!((total - 1.0).abs() < 1e-4, "weights sum to {total}");
            for &(bone, _) in shares {
                assert!(bone < BONES, "bone {bone} is not one of the hand's");
            }
        }
        // The fingertips belong to their own digits, not the palm: a hand
        // whose classification collapsed to the wrist would pass everything
        // above and never curl.
        for (digit, joints) in hand.digits.iter().enumerate() {
            let tip = joints[JOINTS];
            let nearest = hand
                .mesh
                .positions
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.distance(tip).total_cmp(&b.1.distance(tip)))
                .map(|(index, _)| index)
                .expect("the mesh has vertices");
            let base = 1 + digit * (JOINTS + 1);
            assert!(
                influences[nearest]
                    .iter()
                    .any(|&(bone, weight)| bone >= base
                        && bone < base + JOINTS + 1
                        && weight > 0.4),
                "digit {digit}'s tip vertex is not held by its own bones: {:?}",
                influences[nearest]
            );
        }
    }
}
