//! Hands and feet.
//!
//! Both are attached parts rather than more of the capsule graph, for the
//! reasons set out in [`hand`] and [`foot`]. They are placed here, because
//! placing them is the part that has to know about the rig: which joint a limb
//! ends at, which way that limb points, and — the standing lesson of this crate
//! — how thick the body actually is where they join it, which is a question only
//! [`Surface`] can answer.
//!
//! Everything is built in the extremity joint's local space, as the eyes and
//! hair are built in the head's, so a renderer parents each part to its joint
//! and it follows the body for free.

pub mod foot;
pub mod hand;

use glam::{Mat4, Vec3};

use crate::mesh::{PolyMesh, VertexSkin};
use crate::plan::{Limb, Zone};
use crate::rig::skin::Influence;
use crate::rig::{Rig, Role, Surface};

pub use foot::Foot;
pub use hand::Hand;

/// How far a resting hand's fingers curl.
///
/// Not zero. A hand held flat reads as a surrender or as a mannequin; a relaxed
/// hand has curved fingers even when it is holding nothing.
const REST_CURL: f32 = 0.38;

/// A part built and placed in its joint's local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Attached {
    /// Which limb it finishes.
    pub limb: Limb,
    /// The joint it is parented to.
    pub joint: usize,
    /// Its geometry, in that joint's local space.
    pub mesh: PolyMesh,
    /// How far it extends from the joint, in metres.
    pub reach: f32,
}

/// Every hand and foot on a body.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extremities {
    /// Parts on limbs that reach.
    pub hands: Vec<Attached>,
    /// Parts on limbs that carry.
    pub feet: Vec<Attached>,
}

impl Extremities {
    /// Builds hands and feet for a body.
    ///
    /// A limb that carries the body gets a foot; one that does not gets a hand.
    ///
    /// Asked of the rig, not assumed from which end of the body the limb is on.
    /// Fore limbs are hands only on something that stands upright: a quadruped
    /// walks on all four, and giving its front legs fingers — which is what
    /// reading `is_fore` did — puts a pair of human hands on the ground.
    ///
    /// `ground` is the plane the body **stands on** — `0` for the humanoid
    /// plan, which builds bodies with the floor at the origin. Feet reach down
    /// to it, because how deep a foot is *is* the distance from the ankle to the
    /// floor, and no property of the ankle itself says that.
    ///
    /// Not the lowest point of the body's mesh. That is the bottom of the leg's
    /// last node, which floats above the floor by whatever the plan left it, and
    /// measuring from there gives a foot a couple of centimetres deep that the
    /// leg then pokes straight through.
    ///
    /// **The rig is grown, not just read.** Each hand hangs
    /// [`hand::BONES`] − 1 further joints off its wrist — a knuckle and two more
    /// down each of five digits, in [`Role::Digit`] — and the hand mesh comes
    /// back skinned to them rather than bound rigidly. That is the reference's
    /// layout exactly: `hand_l`, then `<digit>_01`, `_02`, `_03` and `_04_leaf`
    /// for thumb, index, middle, ring and pinky, twenty-one bones a hand.
    ///
    /// Taking `&mut Rig` rather than cloning one is the point: a hand skinned
    /// against a rig the caller does not hold is a hand bound to joints that do
    /// not exist.
    #[must_use]
    pub fn build(rig: &mut Rig, surface: &Surface, ground: f32) -> Self {
        let mut extremities = Self::default();
        let carries = rig.ground_contacts();
        // How tall the body actually stands, which is what a foot is in
        // proportion to. Taken from the MEASURED surface at the head rather than
        // from its node radius — subdivision pulls the mesh well inside the
        // radius the plan asked for, so a crown predicted from the plan overshoots
        // and every foot on the body grows with it.
        let stature = rig
            .in_zone(Zone::Head)
            .first()
            .map(|&head| rig.joints[head].position.y + surface.widest(head) - ground)
            .filter(|tall| *tall > f32::EPSILON);

        for limb in Limb::ALL {
            let extremity = rig.in_zone(Zone::Extremity(limb));
            let Some(&joint) = extremity.first() else {
                continue;
            };
            let Some(parent) = rig.joints[joint].parent else {
                continue;
            };
            let along = rig.joints[joint].position - rig.joints[parent].position;
            if along.length_squared() <= f32::EPSILON {
                continue;
            }

            // Measured at the *start* of the final bone — the wrist or the
            // ankle — which is where the part actually joins the body. The node
            // radius there overstates it by half again.
            let girth = surface.radius(joint, 0.0);
            if girth <= f32::EPSILON {
                continue;
            }

            if !carries.contains(&limb) {
                extremities
                    .hands
                    .push(grow_hand(rig, limb, joint, along, girth, stature));
            } else if extremity.len() < 2 {
                // **A foot is only a part on a plan that has not got one.** The
                // humanoid meshes its feet as nodes in the capsule graph — ankle,
                // ball, toe — so the leg runs continuously into the foot and there
                // is no seam to hide (#111), and hanging a swept slab there as
                // well would bury one solid inside another.
                //
                // Asked of the graph rather than of the plan: a limb whose
                // extremity zone holds more than one node has its foot already.
                // The quadruped plan still ends each leg in a single node and
                // still gets a built foot, and will go on doing so until it grows
                // the same chain.
                let drop = rig.joints[joint].position.y - ground;
                extremities
                    .feet
                    .push(grow_foot(limb, joint, along, girth, drop, stature));
            }
        }

        extremities
    }

    /// Every hand and foot, in a fixed order.
    ///
    /// As with [`crate::face::Features::meshes`], the order is the contract
    /// between whatever sizes these and whatever places them.
    pub fn all(&self) -> impl Iterator<Item = &Attached> {
        self.hands.iter().chain(&self.feet)
    }

    /// The same walk, for writing.
    pub fn all_mut(&mut self) -> impl Iterator<Item = &mut Attached> {
        self.hands.iter_mut().chain(&mut self.feet)
    }

    /// Every part on the body, as one mesh, given each joint's transform.
    ///
    /// For tools that want the whole body in one piece; a renderer is better off
    /// drawing each part under its own joint.
    #[must_use]
    pub fn assembled(&self, transform: impl Fn(usize) -> Mat4) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for part in self.hands.iter().chain(&self.feet) {
            mesh.append(&part.mesh.transformed(transform(part.joint)));
        }
        mesh
    }

    /// How many parts were built.
    #[must_use]
    pub fn len(&self) -> usize {
        self.hands.len() + self.feet.len()
    }

    /// Whether the body got no hands or feet at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Builds one hand and sets it back so the palm straddles the wrist.
///
/// **One hand is built and the other is its reflection, which is how both
/// reference bodies are made.** Measured off the GLBs, the Quaternius
/// male's mesh is 3,619 vertices either side of the midline and reflecting one
/// onto the other lands to 0.000 mm — mean and worst alike — and every paired
/// bone from `upperarm` out to `thumb_04_leaf` reflects to the same 0.000 mm.
/// The body is one side, mirrored.
///
/// [`Hand::build`] cannot be fed each arm in turn: it derives its whole frame
/// from the direction it is handed, so the second call *rotates* the hand
/// rather than reflecting it — a half turn about the body's axis, which
/// carries the thumb around with it. Built that way, the default body's two
/// hands reflect onto each other only to 5.5 mm mean and 33.4 mm worst, and
/// the thumbs point to opposite ends of the fore-aft axis: one hand has its
/// thumb in front of the palm and the other behind, which is a pair of right
/// hands.
///
/// So the hand is built once, in the half-space where `Hand::build`'s own
/// chirality is the wanted one, and reflected across the sagittal plane for the
/// arm on the other side. The reflection is a negative-determinant transform,
/// and [`PolyMesh::transformed`] reverses each face's winding to match, so the
/// reflected copy still faces outward.
///
/// The test is `the_two_hands_are_mirrors_of_each_other`, and it now measures
/// what its name says: every vertex of one hand against the nearest vertex of
/// the reflected other.
fn grow_hand(
    rig: &mut Rig,
    limb: Limb,
    joint: usize,
    along: Vec3,
    girth: f32,
    stature: Option<f32>,
) -> Attached {
    // **A hand's length is a share of stature, not of the wrist's girth** —
    // the foot's own lesson (#110), which the hand repeated: sizing off the
    // node radius meant slimming the wrist stub shrank the whole hand. Life
    // and the references put a hand at 10.5–11% of stature.
    let length = stature.map_or(girth * 5.2, |tall| tall * 0.105);
    // Which side is canonical is a fact about `Hand::build`, not about which
    // limb this is: with `up` on world Y it seats the thumb toward +Z — the
    // body's front, where both reference thumbs are — only when `out.x` is
    // negative. Asked of the geometry rather than of `Limb`, so a plan that
    // puts its limbs somewhere else still gets a matched pair.
    let reflected = along.x > 0.0;
    let canonical = if reflected {
        Vec3::new(-along.x, along.y, along.z)
    } else {
        along
    };

    let hand = Hand::build(girth, canonical.normalize(), Vec3::Y, REST_CURL, length);
    // The palm starts at the wrist crease, which is behind the joint the part
    // hangs from — otherwise the hand floats off the end of the arm with the
    // limb's own rounded tip showing through between them.
    // Set back by the whole bone, so the palm's round base sits exactly at the
    // wrist joint whose girth it was sized from. Anywhere short of that puts it
    // partway up a tapering forearm, where the arm is thicker than the base is,
    // and the mismatch shows as a step.
    //
    // The same transform carries the digits' joints, because they were read off
    // the sweep's own stations and have to keep landing on it. A reflection
    // applied to one and not the other is a hand whose bones are inside the
    // other hand.
    let place = Mat4::from_translation(-canonical);
    let place = if reflected {
        // In the joint's own space, whose origin is on the wrist and whose axes
        // are the world's, so this is the sagittal plane.
        Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)) * place
    } else {
        place
    };
    let mesh = hand.mesh.transformed(place);

    // The rig's own numbering for the hand's bones, in `Hand::influences`
    // order: the wrist first, then each digit from the knuckle out. Attached
    // parent-before-child down each digit, which is the order the whole crate
    // relies on and the order glTF requires.
    let wrist = rig.joints[joint].position;
    let mut bones = vec![joint];
    for joints in &hand.digits {
        let mut parent = joint;
        for &local in joints {
            let at = wrist + place.transform_point3(local);
            parent = rig
                .attach(parent, at, Role::Digit)
                .expect("the wrist is a joint of this rig");
            bones.push(parent);
        }
    }

    let mut mesh = mesh;
    mesh.set_skin(
        hand.influences()
            .into_iter()
            .map(|shares| {
                let mut skin = VertexSkin::default();
                for (slot, (bone, weight)) in shares.into_iter().enumerate() {
                    skin[slot] = Influence {
                        joint: bones[bone] as u16,
                        weight,
                    };
                }
                skin
            })
            .collect(),
    );

    Attached {
        limb,
        joint,
        reach: hand.length,
        mesh,
    }
}

/// Builds one foot, pointing the way the ankle bone leans.
fn grow_foot(
    limb: Limb,
    joint: usize,
    along: Vec3,
    girth: f32,
    drop: f32,
    stature: Option<f32>,
) -> Attached {
    // Only the horizontal part of the bone. The ankle bone drops as well as
    // reaching forward, and a foot built along it would point its toes into the
    // ground.
    let flat = Vec3::new(along.x, 0.0, along.z);
    let forward = flat.normalize_or(Vec3::Z);
    // **A foot's length is set by how high its ankle is, not by the bone.** The
    // ankle bone only says which way the toes point; how far forward the plan put
    // that node is a fact about the shin, and reading a foot off it gave one
    // 10.6% of stature long against a measured 15.7–16.4% — two thirds the size
    // it should be, which is the single most visible thing about our feet (#110).
    //
    // Measured on both Quaternius bodies, a foot is 16.4% of stature (male) and
    // 15.7% (female). **Against STATURE, not against `drop`**, and that was
    // decided by measuring the alternative rather than by argument: a foot is
    // also close to three times its own ankle height on the reference (3.06 and
    // 2.95), and that relation gives the wrong answer here — it produced a foot
    // 7.8% of stature, shorter than the 10.6% the previous guess managed.
    //
    // The reason is that `drop` is NOT the ankle's height. It is the height of
    // the plan's last leg node, the one this part hangs from, which sits at 2.57%
    // of stature where the ankle proper is at 6.86%. Reading the reference's
    // ankle against it compares two different landmarks — the reference ankle is
    // at 5.3–5.4% of stature, so our ankle is if anything slightly HIGH, and the
    // thing that is low is the node the foot is grown from.
    //
    // That is also why the foot comes out thin: its depth is `drop`, so it is as
    // thick as a node that sits barely above the floor, against a reference foot
    // about 4.8% of stature thick. Fixing it means meshing the foot from the
    // capsule graph instead of hanging a slab off one low node (#111), which is
    // what is happening next; this keeps the length honest in the meantime.
    //
    // Falls back to the `drop` relation on a body with no head to measure against.
    let length = stature
        .map_or(drop * 3.0, |tall| tall * 0.16)
        .max(girth * 2.4);
    let foot = Foot::build(girth, forward, Vec3::Y, length, drop.max(girth * 0.6));

    // Built about the ankle, which sits behind the joint the part hangs from.
    let ankle = Mat4::from_translation(-flat);
    Attached {
        limb,
        joint,
        reach: foot.length,
        mesh: foot.mesh.transformed(ankle),
    }
}

/// Welds one hand into the body's skin mesh (#297).
///
/// **The owner's ask, taken literally: the wrist is welded, not disguised.**
/// The arm used to end in a capped stub with the hand's own solid buried over
/// it, and every arrangement of tucks and cuffs only decided where the two
/// surfaces' meeting line fell. This removes the meeting: the stub's surface
/// is cut out of the body, the hand arrives open at its weld ring
/// ([`Hand::build`] pushes no base cap), and the two boundaries are bridged
/// with one band of triangles — one surface, one silhouette, no interior
/// geometry.
///
/// `skin` is the charted body in body space; `hand` is the placed hand mesh,
/// already in body space and already skinned. Works on the charted copy
/// deliberately: the unwrap, the garments and the painted atlas all read the
/// uncut body, and cutting here changes only what is drawn.
///
/// The hole's ring is walked by POSITION, not by index: the charted body
/// splits vertices along UV seams, so the ring crosses vertices that exist
/// twice, and an index walk would stop at the first seam. The bridge then
/// zips the two rings by angle about the wrist's own axis, which needs no
/// agreement about counts — the arm ring is whatever the subdivision left,
/// the hand ring is the cage's twenty.
pub(crate) fn weld(skin: &mut PolyMesh, hand: &PolyMesh, rig: &Rig, limb: Limb, joint: usize) {
    use std::collections::BTreeMap;
    let Some(parent) = rig.joints[joint].parent else {
        return;
    };
    let crease = rig.joints[parent].position;
    let axis = (rig.joints[joint].position - crease).normalize_or(Vec3::Y);

    // 1. Cut the stub's surface out of the body: every face whose centroid
    // lies nearest the extremity bone of this limb. The zone runs from the
    // wrist crease to the stub's cap, which is exactly the surface the hand
    // replaces.
    let zone = Zone::Extremity(limb);
    let (kept, removed): (Vec<Vec<u32>>, Vec<Vec<u32>>) =
        skin.faces.iter().cloned().partition(|face| {
            let centre = face
                .iter()
                .fold(Vec3::ZERO, |sum, &v| sum + skin.positions[v as usize])
                / face.len() as f32;
            rig.joints[rig.nearest_bone(centre).joint].zone != zone
        });
    if removed.is_empty() {
        // Nothing to cut — a plan with no stub keeps its appended hand whole.
        skin.append(&hand.clone());
        return;
    }
    skin.faces = kept;

    // 2. The hole's rim: the vertices of kept boundary edges standing where a
    // REMOVED face used to stand — matched by POSITION, and that is
    // load-bearing twice over. By index it finds nothing: the charts are cut
    // by zone, so the cut's own edge is a chart boundary whose two sides
    // never shared vertices, and it was a "boundary edge" before the cut too.
    // And by proximity it finds too much: a dressed body's sleeve hem is a
    // boundary near the wrist, and a rim gathered that way would stitch the
    // hand to a shirt. A hem's positions never coincide with a removed
    // face's, so the position test excludes it for free.
    let key = |p: Vec3| {
        (
            (p.x * 1e5).round() as i64,
            (p.y * 1e5).round() as i64,
            (p.z * 1e5).round() as i64,
        )
    };
    let mut gone = std::collections::BTreeSet::new();
    for face in &removed {
        for &vertex in face {
            gone.insert(key(skin.positions[vertex as usize]));
        }
    }
    let after = edge_counts(skin);
    let mut arm_rim: Vec<(u32, Vec3)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (&edge, &uses) in &after {
        if uses != 1 {
            continue;
        }
        for vertex in [edge.0, edge.1] {
            let position = skin.positions[vertex as usize];
            if gone.contains(&key(position)) && seen.insert(vertex) {
                arm_rim.push((vertex, position));
            }
        }
    }

    // 3. The hand's weld ring: its only boundary, but bounded to the crease's
    // neighbourhood anyway so the invariant is stated where it is relied on.
    let hand_rim = rim_positions(hand, crease, rig.joints[parent].radius * 3.0);

    let side = axis.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = axis.cross(side);
    let angle = |p: Vec3| {
        let from = p - crease;
        from.dot(up).atan2(from.dot(side))
    };

    // 4. SNAP the hand's ring onto the arm's rim curve before stitching. The
    // two rings were only ever near each other: the arm's surface is not
    // round at the crease and the cut's rim jitters between subdivision rows,
    // so a bridge between free rings rendered as a cliff wherever the radii
    // disagreed — worst across the top, where the arm stands furthest off its
    // bone. Interpolating the rim's position at each hand vertex's own angle
    // puts both boundaries on ONE curve; the bridge that follows is a sliver
    // that only exists to keep the crack sealed when the wrist bends, because
    // the two rings answer to different bones.
    let mut hand = hand.clone();
    if !arm_rim.is_empty() {
        let mut curve: Vec<(f32, Vec3)> = arm_rim.iter().map(|&(_, p)| (angle(p), p)).collect();
        curve.sort_by(|a, b| a.0.total_cmp(&b.0));
        curve.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-5);
        let at = |theta: f32| -> Vec3 {
            let after = curve.iter().position(|&(a, _)| a >= theta).unwrap_or(0);
            let before = if after == 0 {
                curve.len() - 1
            } else {
                after - 1
            };
            let (a0, p0) = curve[before];
            let (a1, p1) = curve[after];
            let span = (a1 - a0).rem_euclid(std::f32::consts::TAU);
            let into = (theta - a0).rem_euclid(std::f32::consts::TAU);
            let t = if span <= 1e-6 {
                0.0
            } else {
                (into / span).clamp(0.0, 1.0)
            };
            p0 + (p1 - p0) * t
        };
        for &(vertex, position) in &hand_rim {
            hand.positions[vertex as usize] = at(angle(position));
        }
    }
    let hand = hand;

    let offset = skin.vertex_count() as u32;
    skin.append(&hand);

    if arm_rim.is_empty() || hand_rim.is_empty() {
        return;
    }
    let mut arm: Vec<(f32, u32)> = arm_rim.iter().map(|&(v, p)| (angle(p), v)).collect();
    let mut hand_ring: Vec<(f32, u32)> = hand_rim
        .iter()
        .map(|&(v, p)| (angle(p), v + offset))
        .collect();
    arm.sort_by(|a, b| a.0.total_cmp(&b.0));
    hand_ring.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut faces = Vec::new();
    let (mut a, mut h) = (0usize, 0usize);
    while a < arm.len() || h < hand_ring.len() {
        let arm_next = arm.get(a).map_or(f32::INFINITY, |&(angle, _)| angle);
        let hand_next = hand_ring.get(h).map_or(f32::INFINITY, |&(angle, _)| angle);
        let arm_at = arm[a.min(arm.len() - 1)].1;
        let hand_at = hand_ring[h.min(hand_ring.len() - 1)].1;
        if arm_next <= hand_next {
            let next = arm[(a + 1) % arm.len()].1;
            faces.push(vec![arm_at, next, hand_at]);
            a += 1;
        } else {
            let next = hand_ring[(h + 1) % hand_ring.len()].1;
            faces.push(vec![hand_at, next, arm_at]);
            h += 1;
        }
    }

    // Wound outward by measurement rather than by derivation: which way round
    // the two rims run depends on the cut, the chirality and the axis frame,
    // and a radial test settles it face by face.
    for face in &mut faces {
        let (p0, p1, p2) = (
            skin.positions[face[0] as usize],
            skin.positions[face[1] as usize],
            skin.positions[face[2] as usize],
        );
        let centre = (p0 + p1 + p2) / 3.0;
        let radial = centre - (crease + axis * (centre - crease).dot(axis));
        if (p1 - p0).cross(p2 - p0).dot(radial) < 0.0 {
            face.reverse();
        }
    }
    for face in faces {
        if face[0] != face[1] && face[1] != face[2] && face[0] != face[2] {
            skin.push_face(face);
        }
    }

    // 5. Fair the junction. The two surfaces now share one rim curve, but
    // they arrive at it at different slopes — the round forearm stands
    // further off the bone than the flat back of the hand — and the change
    // of slope concentrated on one ring reads as a step in the silhouette.
    // A few Laplacian passes over the crease's neighbourhood spread that
    // slope change across the band, which is `face::neck::fair`'s move: the
    // shape "smooth wrist" names is a fairing, not another station.
    //
    // **Chart-split copies move as one.** The charted body carries a vertex
    // once per chart; smoothing each copy by its own adjacency would walk
    // them apart and open every UV seam in the band, so positions are
    // grouped by location and re-welded after every pass.
    let reach = rig.joints[parent].radius * 2.0;
    let band: Vec<u32> = (0..skin.vertex_count() as u32)
        .filter(|&v| skin.positions[v as usize].distance(crease) <= reach)
        .collect();
    let in_band: std::collections::BTreeSet<u32> = band.iter().copied().collect();
    let mut around: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for face in &skin.faces {
        for (index, &v) in face.iter().enumerate() {
            if !in_band.contains(&v) {
                continue;
            }
            let prev = face[(index + face.len() - 1) % face.len()];
            let next = face[(index + 1) % face.len()];
            let entry = around.entry(v).or_default();
            entry.push(prev);
            entry.push(next);
        }
    }
    let mut groups: BTreeMap<(i64, i64, i64), Vec<u32>> = BTreeMap::new();
    for &v in &band {
        groups
            .entry(key(skin.positions[v as usize]))
            .or_default()
            .push(v);
    }
    for _ in 0..3 {
        let was = skin.positions.clone();
        for (&v, neighbours) in &around {
            if neighbours.is_empty() {
                continue;
            }
            let mean = neighbours
                .iter()
                .fold(Vec3::ZERO, |sum, &n| sum + was[n as usize])
                / neighbours.len() as f32;
            let old = was[v as usize];
            skin.positions[v as usize] = old + (mean - old) * 0.45;
        }
        for members in groups.values() {
            if members.len() < 2 {
                continue;
            }
            let mean = members
                .iter()
                .fold(Vec3::ZERO, |sum, &v| sum + skin.positions[v as usize])
                / members.len() as f32;
            for &v in members {
                skin.positions[v as usize] = mean;
            }
        }
    }

    // 6. Close the crack the fairing opens. Each rim smooths under its own
    // side's adjacency — the slivers joining them are too thin to couple the
    // two — so three passes walk the rims apart and the bridge stretches
    // into a visible groove. The hand's rim is set back onto the arm's
    // smoothed rim curve, which is the same snap as step 4 with the faired
    // positions as the target.
    {
        let mut curve: Vec<(f32, Vec3)> = arm
            .iter()
            .map(|&(_, v)| {
                let p = skin.positions[v as usize];
                (angle(p), p)
            })
            .collect();
        curve.sort_by(|a, b| a.0.total_cmp(&b.0));
        curve.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-5);
        if !curve.is_empty() {
            for &(_, v) in &hand_ring {
                let p = skin.positions[v as usize];
                let theta = angle(p);
                let after = curve.iter().position(|&(a, _)| a >= theta).unwrap_or(0);
                let before = if after == 0 {
                    curve.len() - 1
                } else {
                    after - 1
                };
                let (a0, p0) = curve[before];
                let (a1, p1) = curve[after];
                let span = (a1 - a0).rem_euclid(std::f32::consts::TAU);
                let into = (theta - a0).rem_euclid(std::f32::consts::TAU);
                let t = if span <= 1e-6 {
                    0.0
                } else {
                    (into / span).clamp(0.0, 1.0)
                };
                skin.positions[v as usize] = p0 + (p1 - p0) * t;
            }
        }
    }

    // 7. Re-derive the band's normals from the WELDED, faired surface. Each
    // side kept the normals its own source mesh computed, and the mismatch
    // rendered as a shading step ringing the wrist — the seam back, in light
    // instead of geometry. Only the band is touched: a global recompute
    // would crease every UV seam the body deliberately split.
    if skin.normals.is_empty() {
        return;
    }
    let touched = in_band;
    let mut sums: std::collections::BTreeMap<u32, Vec3> = BTreeMap::new();
    for face in &skin.faces {
        if !face.iter().any(|v| touched.contains(v)) {
            continue;
        }
        let mut normal = Vec3::ZERO;
        for (index, &v) in face.iter().enumerate() {
            let next = skin.positions[face[(index + 1) % face.len()] as usize];
            let here = skin.positions[v as usize];
            normal += here.cross(next);
        }
        for &v in face {
            if touched.contains(&v) {
                *sums.entry(v).or_insert(Vec3::ZERO) += normal;
            }
        }
    }
    for (vertex, sum) in sums {
        let normal = sum.normalize_or(skin.normals[vertex as usize]);
        skin.normals[vertex as usize] = normal;
    }

    // 8. The cut's faces are gone; their orphaned corners go with them, or
    // every buffer downstream carries 87 dead vertices per hand — the
    // `suppressing_the_covered_skin_leaves_no_vertex_behind` guard is what
    // holds this.
    skin.compact();
}

/// Undirected edge → how many faces use it.
fn edge_counts(mesh: &PolyMesh) -> std::collections::BTreeMap<(u32, u32), usize> {
    let mut count = std::collections::BTreeMap::new();
    for face in &mesh.faces {
        for (index, &a) in face.iter().enumerate() {
            let b = face[(index + 1) % face.len()];
            *count.entry((a.min(b), a.max(b))).or_insert(0usize) += 1;
        }
    }
    count
}

/// A mesh's boundary vertices near `at`, with their positions.
///
/// Every directed edge whose reverse is missing is a boundary edge; the rim is
/// their endpoints, filtered to `reach` of the crease so no other boundary on
/// the mesh — a garment hem, another limb's weld — can leak in. Deduplicated
/// by index, not by position: a UV-seam split leaves two vertices in one
/// place, and BOTH must be stitched or the seam opens under animation.
fn rim_positions(mesh: &PolyMesh, at: Vec3, reach: f32) -> Vec<(u32, Vec3)> {
    use std::collections::BTreeMap;
    let mut count: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for face in &mesh.faces {
        for (index, &a) in face.iter().enumerate() {
            let b = face[(index + 1) % face.len()];
            *count.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let mut rim: Vec<(u32, Vec3)> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (&(a, b), &uses) in &count {
        if uses != 1 {
            continue;
        }
        for vertex in [a, b] {
            let position = mesh.positions[vertex as usize];
            if position.distance(at) <= reach && seen.insert(vertex) {
                rim.push((vertex, position));
            }
        }
    }
    rim
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};

    fn body(seed: i64) -> (Rig, Surface, f32) {
        let mut record = AvatarRecord::new("Handed", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("the body should mesh");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("the body should rig");
        let surface = Surface::measure(&mesh, &rig);
        // The plan stands its bodies on the origin.
        (rig, surface, 0.0)
    }

    #[test]
    fn a_biped_gets_two_hands_and_leaves_its_feet_to_the_cage() {
        // **A biped's feet are not parts any more** (#111). They are ankle, ball
        // and toe nodes in the capsule graph, so the leg is continuous into the
        // foot and nothing is hung off the end of it. Two hands is the whole of
        // what this builds for an upright body, and a foot appearing here again
        // would mean a swept slab buried inside the meshed one.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        assert_eq!(built.hands.len(), 2);
        assert_eq!(built.feet.len(), 0, "a humanoid foot comes from the cage");
        assert_eq!(built.len(), 2);
        assert!(!built.is_empty());
    }

    #[test]
    fn feet_go_on_the_limbs_that_carry_the_body() {
        let (mut rig, surface, ground) = body(7);
        let built = Extremities::build(&mut rig, &surface, ground);
        let carries = rig.ground_contacts();
        assert!(built.feet.iter().all(|part| carries.contains(&part.limb)));
        assert!(built.hands.iter().all(|part| !carries.contains(&part.limb)));
    }

    #[test]
    fn a_quadruped_walks_on_four_feet_and_has_no_hands() {
        // Read from which end of the body a limb is on, a quadruped's front
        // legs come out as a pair of human hands, on the ground.
        use crate::plan::{BodyPlan, QuadrupedParams};
        let skeleton = QuadrupedParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let mut rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let surface = Surface::measure(&mesh, &rig);

        let built = Extremities::build(&mut rig, &surface, 0.0);
        assert_eq!(built.feet.len(), 4, "a quadruped stands on four");
        assert!(built.hands.is_empty(), "a quadruped has no hands");
    }

    #[test]
    fn every_part_hangs_from_an_extremity_joint() {
        let (mut rig, surface, ground) = body(23);
        let built = Extremities::build(&mut rig, &surface, ground);
        for part in built.hands.iter().chain(&built.feet) {
            assert_eq!(rig.joints[part.joint].zone, Zone::Extremity(part.limb));
        }
    }

    #[test]
    fn a_hand_is_a_share_of_stature_not_of_the_wrist() {
        // The foot's lesson (#110) arriving at the hand (#297): a part sized
        // off the girth of the node it hangs from inherits every reason that
        // node's radius was tuned, and slimming the wrist stub for the welded
        // hand shrank the whole hand by a third before this moved to stature.
        // 10.5% of stature is the reference figure `grow_hand` quotes.
        let (mut rig, surface, ground) = body(29);
        // The same derivation `Extremities::build` uses: measured crown, not
        // the node radius the plan asked for.
        let stature = rig
            .in_zone(Zone::Head)
            .first()
            .map(|&head| rig.joints[head].position.y + surface.widest(head) - ground)
            .expect("a biped has a head");
        let built = Extremities::build(&mut rig, &surface, ground);
        let hand = &built.hands[0];
        assert!(
            (hand.reach - stature * 0.105).abs() < 1e-4,
            "reach {} against a stature of {stature}",
            hand.reach
        );
        // The girth still matters for what it SHOULD decide — the base ring
        // that has to emerge from the arm — and it stays a MEASURED quantity.
        // No relation to the planned node radius is asserted any more: the
        // wrist stub was slimmed for the welded hand (#297) and the surface
        // there now blends fatter than the stub's own node, which is exactly
        // the kind of coupling the stature sizing above exists to escape.
        assert!(surface.radius(hand.joint, 0.0) > 0.0);
    }

    #[test]
    fn feet_reach_forward_and_hands_reach_outward() {
        let (mut rig, surface, ground) = body(11);
        let built = Extremities::build(&mut rig, &surface, ground);

        for foot in &built.feet {
            let (lo, hi) = foot.mesh.bounds();
            assert!(lo.y < 0.0, "a foot did not sit below its joint");
            // Measured from the ankle, which sits behind the joint the part
            // hangs from — the last node of the leg is out near the ball of the
            // foot, so most of the foot is behind it and that says nothing.
            let parent = rig.joints[foot.joint].parent.expect("a foot has an ankle");
            let ankle = rig.joints[parent].position - rig.joints[foot.joint].position;
            assert!(
                hi.z - ankle.z > (ankle.z - lo.z) * 1.5,
                "a foot reached {} in front of its ankle and {} behind",
                hi.z - ankle.z,
                ankle.z - lo.z
            );
        }
        for hand in &built.hands {
            // Along the limb's own axis, not along world X: the part is set back
            // by the whole wrist bone, so measuring from the joint it hangs from
            // understates how far it reaches.
            let parent = rig.joints[hand.joint].parent.expect("a hand has a wrist");
            let out = (rig.joints[hand.joint].position - rig.joints[parent].position).normalize();
            let across = out.cross(Vec3::Y).normalize();
            let span = |axis: Vec3| {
                let reach = hand.mesh.positions.iter().map(|p| p.dot(axis));
                reach.clone().fold(f32::MIN, f32::max) - reach.fold(f32::MAX, f32::min)
            };
            assert!(
                span(out) > span(across),
                "a hand reached {} along the arm and {} across it",
                span(out),
                span(across)
            );
        }
    }

    #[test]
    fn the_two_hands_are_mirrors_of_each_other() {
        // **Measured as a reflection, not as a pair of bounding boxes** (#113).
        // This used to compare the two hands' x spans, which a hand rotated half
        // a turn satisfies exactly as well as a reflected one — and that is what
        // the body had: two right hands, agreeing on every box measurement
        // anyone thought to take.
        //
        // The check is the one the reference passes. Reflect one hand across the
        // sagittal plane and ask, for every vertex of it, how far the nearest
        // vertex of the other hand is. On the Quaternius male that answer is
        // 0.000 mm at both the mean and the worst, over 3,619 vertices a side;
        // its body is one half mirrored, and so is ours now.
        let (mut rig, surface, ground) = body(5);
        let built = Extremities::build(&mut rig, &surface, ground);
        assert_eq!(built.hands.len(), 2);

        // In world space, because the two parts live in different joints' spaces
        // and reflecting inside one of them would compare a hand with itself.
        let world: Vec<PolyMesh> = built
            .hands
            .iter()
            .map(|hand| {
                hand.mesh
                    .transformed(Mat4::from_translation(rig.joints[hand.joint].position))
            })
            .collect();
        let reflected = world[0].transformed(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));

        let mut worst = 0.0f32;
        for &point in &reflected.positions {
            let nearest = world[1]
                .positions
                .iter()
                .map(|&other| other.distance(point))
                .fold(f32::MAX, f32::min);
            worst = worst.max(nearest);
        }
        // A tenth of a millimetre. The construction is exact up to float error —
        // the reflected hand is built from the same numbers with one sign
        // changed — so this is a check for a wrong *shape*, not a tolerance on a
        // fit. Before the fix it read 33.4 mm.
        assert!(
            worst < 1e-4,
            "reflecting one hand onto the other left {:.3} mm; the pair are not \
             reflections, which means one of them is the wrong hand",
            worst * 1000.0
        );

        // And the thumbs agree about which way the body faces, which is the
        // property a reflection preserves and a half-turn destroys.
        for hand in &built.hands {
            let (lo, hi) = hand.mesh.bounds();
            let _ = lo;
            assert!(
                hi.z > 0.0,
                "a hand reached only to z {}, so its thumb is behind the palm",
                hi.z
            );
        }
    }

    #[test]
    fn assembling_places_every_part_somewhere_different() {
        let (mut rig, surface, ground) = body(9);
        let built = Extremities::build(&mut rig, &surface, ground);
        let whole = built.assembled(|joint| Mat4::from_translation(rig.joints[joint].position));
        let (lo, hi) = whole.bounds();
        // Two hands, one at the end of each arm. The span is lateral now that
        // feet are meshed with the body rather than assembled here (#111), so
        // the vertical reach this used to check went with them — what is left is
        // the property it was really testing, that each part is placed at its own
        // joint instead of all of them landing on the origin.
        assert!(hi.x - lo.x > 0.5, "parts spanned only {}", hi.x - lo.x);
        assert!(
            lo.x < 0.0 && hi.x > 0.0,
            "both hands ended up on the same side"
        );
    }

    #[test]
    fn a_foot_reaches_the_ground_the_body_stands_on() {
        // The point of taking a ground plane at all. Sized from anything about
        // the ankle itself, a foot comes out a wafer the leg pokes through.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        for foot in &built.feet {
            let sole = foot.mesh.bounds().0.y + rig.joints[foot.joint].position.y;
            assert!(
                (sole - ground).abs() < 1e-3,
                "a sole sat at {sole} against a ground of {ground}"
            );
        }
    }

    #[test]
    fn extremities_are_reproducible() {
        // Two bodies, not one built twice. Building grows the rig, so a second
        // pass over the same rig hangs a second set of finger bones off it and
        // is a different question — one whose honest answer is "no".
        let (mut first, surface, ground) = body(13);
        let (mut again, _, _) = body(13);
        assert_eq!(
            Extremities::build(&mut first, &surface, ground),
            Extremities::build(&mut again, &surface, ground)
        );
    }

    #[test]
    fn each_hand_carries_the_references_twenty_one_bones() {
        // `hand_l`, then four joints down each of five digits. The reference
        // rigs its hands exactly this way and the whole point of matching it is
        // that a hand pose or a grip retargets between the two without a table.
        let (mut rig, surface, ground) = body(1);
        let before = rig.len();
        let built = Extremities::build(&mut rig, &surface, ground);

        let digits: Vec<usize> = (0..rig.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit)
            .collect();
        assert_eq!(digits.len(), (hand::BONES - 1) * built.hands.len());
        assert_eq!(rig.len(), before + digits.len());
        assert_eq!(hand::BONES, 21, "the reference carries twenty-one a hand");

        // Parent-before-child down every digit, which glTF requires and the rest
        // of the crate assumes.
        for &joint in &digits {
            let parent = rig.joints[joint]
                .parent
                .expect("a digit hangs off something");
            assert!(parent < joint, "joint {joint} precedes its parent {parent}");
            assert_eq!(
                rig.joints[joint].zone, rig.joints[parent].zone,
                "a finger left its own hand's zone"
            );
        }
    }

    #[test]
    fn the_two_hands_bones_are_mirrors_of_each_other() {
        // The same claim as for the meshes, about the rig. Every paired bone of
        // the reference — `upperarm` out to `thumb_04_leaf` — reflects onto its
        // opposite to 0.000 mm, and a hand rig that does not is one where a
        // shared animation puts a left thumb where a right one should be.
        let (mut rig, surface, ground) = body(1);
        // Only the rig matters here; the parts are checked elsewhere.
        let _ = Extremities::build(&mut rig, &surface, ground);

        let digits: Vec<Vec3> = (0..rig.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit)
            .map(|joint| rig.joints[joint].position)
            .collect();
        assert!(!digits.is_empty());

        let mut worst = 0.0f32;
        for &at in &digits {
            let mirror = Vec3::new(-at.x, at.y, at.z);
            worst = worst.max(
                digits
                    .iter()
                    .map(|&other| other.distance(mirror))
                    .fold(f32::MAX, f32::min),
            );
        }
        assert!(
            worst < 1e-6,
            "a digit joint reflected onto nothing nearer than {:.4} mm",
            worst * 1000.0
        );
    }

    #[test]
    fn a_finger_is_bound_to_its_own_bones_and_to_no_others() {
        // What the rig is for. Every vertex of a digit has to be held by that
        // digit's chain or by the wrist it hangs from — one stray weight and a
        // closing fist drags a knuckle from the other side of the hand with it.
        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        let hand = &built.hands[0];
        assert_eq!(
            hand.mesh.skin.len(),
            hand.mesh.vertex_count(),
            "the hand came back unskinned, so it is glued shut"
        );

        // The bones this hand may legally use: its wrist, and the digit joints
        // descended from it.
        let mut allowed = vec![hand.joint];
        for joint in 0..rig.len() {
            if rig.joints[joint].role == Role::Digit {
                let mut walk = rig.joints[joint].parent;
                while let Some(parent) = walk {
                    if parent == hand.joint {
                        allowed.push(joint);
                        break;
                    }
                    walk = rig.joints[parent].parent;
                }
            }
        }
        assert_eq!(allowed.len(), hand::BONES);

        for influences in &hand.mesh.skin {
            let total: f32 = influences.iter().map(|hold| hold.weight).sum();
            assert!(
                (total - 1.0).abs() < 1e-5,
                "a hand vertex was held {total} times over"
            );
            for hold in influences.iter().filter(|hold| hold.weight > 0.0) {
                assert!(
                    allowed.contains(&(hold.joint as usize)),
                    "a hand vertex was bound to joint {}, which is not one of its own",
                    hold.joint
                );
            }
        }
    }

    #[test]
    fn bending_a_knuckle_closes_one_finger_and_leaves_the_rest_alone() {
        // The measurement that says the rig deforms rather than decorates, and
        // the one a rigid bind fails outright: before #113 the whole hand rode
        // the wrist, so turning any finger joint moved nothing at all.
        use crate::anim::Pose;
        use glam::Quat;

        let (mut rig, surface, ground) = body(1);
        let built = Extremities::build(&mut rig, &surface, ground);
        let hand = &built.hands[0];
        let weights = crate::rig::SkinWeights {
            vertices: hand.mesh.skin.clone(),
        };

        // The index finger's knuckle: digit 0, phalanx 0, so the first digit
        // joint attached for this hand.
        let knuckle = (0..rig.len())
            .find(|&joint| {
                rig.joints[joint].role == Role::Digit
                    && rig.joints[joint].parent == Some(hand.joint)
            })
            .expect("a hand has knuckles");

        let mut pose = Pose::rest(&rig);
        pose.rotations[knuckle] = Quat::from_rotation_x(1.0);
        let moved = pose
            .forward(&rig)
            .deform(&rig, &hand.mesh.positions, &weights);

        let stirred = (0..hand.mesh.vertex_count())
            .filter(|&vertex| hand.mesh.positions[vertex].distance(moved[vertex]) > 1e-4)
            .count();
        assert!(stirred > 8, "only {stirred} vertices followed the knuckle");
        assert!(
            stirred < hand.mesh.vertex_count() / 4,
            "{stirred} of {} vertices moved: one knuckle is carrying the whole hand",
            hand.mesh.vertex_count()
        );
    }
}
