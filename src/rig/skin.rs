//! Attaching a mesh to a rig.
//!
//! Because the same code generates both the skeleton and the surface, skin
//! weights can be derived analytically rather than solved for: every vertex is
//! near a known bone, and how near is exactly the influence that bone should
//! have. A general auto-rigger has to infer the skeleton from the mesh first,
//! which is the hard part — and it is a part we simply do not have.
//!
//! The falloff has bounded support, so a vertex is only influenced by bones that
//! can plausibly reach it. Raw distance weights are then **smoothed across the
//! surface**, which is what keeps a broad region like a torso from creasing
//! where two bones' influence meets — the classic failure of purely
//! distance-based weighting.
//!
//! # Which joint owns a bone
//!
//! A bone spans two joints, and only one of them deforms it. [`Pose::forward`]
//! places a joint at `positions[parent] + parent_rotation * offset`, so the
//! rotation stored at a joint turns that joint's *children* about it: rotating
//! a hip swings the thigh, and rotating a knee does not touch the thigh at all.
//! The joint that deforms the bone `parent → child` is therefore the **parent**
//! — the proximal end.
//!
//! [`Rig::bone`] names its segments by the distal end, because that is the
//! joint the segment leads into and there is exactly one of them. Binding has to
//! undo that: the weight a vertex earns from lying near the segment `hip → knee`
//! belongs to the **hip**, not to the knee.
//!
//! **Getting this wrong is what makes limbs bend like rope.** Binding the
//! thigh to the knee means flexing a knee rotates the thigh about the knee as
//! well, so the whole leg curves instead of hinging. Measured by
//! `examples/bodyaudit` on the default body: bound that way, turning only the
//! knee moves the mid-thigh 39.8 mm against the mid-shank's 73.1 mm, a 54%
//! leak into the segment that must not move; the elbow leaks 76%. Both are
//! zero once the bone is owned by the joint that actually turns it.
//!
//! A consequence worth stating, because it looks like a bug: **a leaf joint gets
//! no weight from the body's own surface.** A hand, a foot, a crown has no bone
//! leaving it, so nothing on the body is deformed by rotating one. That is
//! correct — those joints are position markers for the body, and they earn
//! their weight from the geometry *attached* at them, which is bound
//! separately.
//!
//! [`Pose::forward`]: crate::anim::Pose::forward

use glam::Vec3;
use std::collections::BTreeSet;

use super::Rig;
use crate::face::skull;
use crate::mesh::PolyMesh;
use crate::plan::Zone;

/// How many bones may influence one vertex.
///
/// Four is what glTF, and every engine that reads it, expects per vertex.
pub const MAX_INFLUENCES: usize = 4;

/// One bone's hold on one vertex.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Influence {
    /// Index of the influencing joint in [`Rig::joints`].
    pub joint: u16,
    /// How strongly it pulls, in `0..=1`.
    pub weight: f32,
}

impl Default for Influence {
    fn default() -> Self {
        Self {
            joint: 0,
            weight: 0.0,
        }
    }
}

/// Tuning for [`bind`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinConfig {
    /// How far a bone reaches, in multiples of its own radius.
    ///
    /// Larger values blend more bones together and soften joints; smaller ones
    /// keep influence local and crease more readily.
    pub reach: f32,
    /// Exponent on the falloff. Higher concentrates weight near the bone.
    pub falloff: f32,
    /// Surface smoothing passes applied to the raw distance weights.
    pub smoothing_iterations: usize,
    /// How far each pass moves a vertex toward its neighbours' average.
    pub smoothing_strength: f32,
}

impl Default for SkinConfig {
    fn default() -> Self {
        // Swept against the reference mannequin's own binding rather than
        // picked (#97). The two figures a bind is judged on are how many bones
        // hold a vertex — the reference averages 1.88, with 85% of its vertices
        // on two or fewer — and how much of a bone moves when the joint *after*
        // it turns, which the reference makes zero by giving the grandparent
        // bone no weight at all.
        //
        // Down from `reach: 2.6, smoothing_iterations: 3`, which averaged 3.12
        // bones per vertex with only 31% on two or fewer: three bones shared
        // the middle of every bone, and the ankle held 0.18 of the mid-thigh.
        // Measured across the sweep, at one smoothing pass:
        //
        // ```text
        //   reach   bones/vertex   <=2 bones   knee leak   area kept
        //    2.6        3.12          31%         13%        100%
        //    2.0        2.71          49%          6%        100%
        //    1.6        2.44          58%          3%        101%
        //    1.3        2.13          68%          2%         99%
        //    1.1        1.92          80%          1%         98%
        //    0.9        1.67          92%          0%         96%
        // ```
        //
        // 1.1 is where the locality lands on the reference; 0.9 overshoots it
        // and starts spending surface area on the inside of a fold. The area
        // column is why this is a floor rather than a preference — it is the
        // crease the smoothing exists to prevent, and it only becomes visible
        // below about 1.3. That it stays as high as 98% at all is dual
        // quaternion skinning doing its job; under matrices this column would
        // decide the value on its own.
        //
        // `falloff` and `smoothing_strength` were swept too and neither moved
        // anything worth having: 1.5 through 3.0 span 1.94 to 1.85 bones per
        // vertex, buying a percent of locality for a percent of area. They stay
        // where they were.
        Self {
            reach: 1.1,
            falloff: 2.0,
            smoothing_iterations: 1,
            smoothing_strength: 0.5,
        }
    }
}

/// Per-vertex bone influences.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinWeights {
    /// Influences per vertex, normalised and sorted strongest first.
    pub vertices: Vec<[Influence; MAX_INFLUENCES]>,
}

impl SkinWeights {
    /// The joint holding `vertex` most strongly.
    #[must_use]
    pub fn dominant(&self, vertex: usize) -> u16 {
        self.vertices[vertex][0].joint
    }

    /// Which body zone each vertex belongs to.
    ///
    /// This is what lets a garment suppress the skin beneath it: the body knows,
    /// per vertex, which zone it is in, a garment claims whole faces all of
    /// whose corners are in its zones, and the claimed faces are then never
    /// emitted — poke-through avoided by deleting the geometry rather than by
    /// hiding it. The claim is [`Garment::claim`](crate::Garment::claim) and
    /// the deletion is `Avatar::charted_body`; it is worth about 1,500
    /// triangles on a clothed body.
    ///
    /// A vertex takes the zone of the **nearer end** of the bone holding it, not
    /// the bone's own joint. A bone spans two nodes that may sit in different
    /// zones — the thigh bone runs from pelvis to knee — and reading one end
    /// alone would label the whole crotch as thigh, and leave the root's zone
    /// with no surface at all.
    ///
    /// The dominant joint is the bone's *proximal* end (see the module docs), so
    /// the bone in question is one of those **leaving** it. A joint may have
    /// several — a pelvis has three — which is why the nearest has to be
    /// searched for rather than looked up.
    #[must_use]
    pub fn zone_map(&self, mesh: &PolyMesh, rig: &Rig) -> Vec<Zone> {
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); rig.len()];
        for joint in 0..rig.len() {
            if let Some(parent) = rig.joints[joint].parent {
                children[parent].push(joint);
            }
        }

        (0..self.vertices.len())
            .map(|vertex| {
                let joint = self.dominant(vertex) as usize;
                let here = rig.joints[joint];
                let position = mesh.positions[vertex];
                let nearest = children[joint]
                    .iter()
                    .map(|&child| {
                        let (distance, along) = distance_to_segment(
                            position,
                            here.position,
                            rig.joints[child].position,
                        );
                        (distance, along, child)
                    })
                    .min_by(|a, b| a.0.total_cmp(&b.0));
                match nearest {
                    Some((_, along, child)) if along >= 0.5 => rig.joints[child].zone,
                    _ => here.zone,
                }
            })
            .collect()
    }

    /// Whether every vertex's weights sum to one, within `tolerance`.
    #[must_use]
    pub fn is_normalized(&self, tolerance: f32) -> bool {
        self.vertices.iter().all(|influences| {
            let total: f32 = influences.iter().map(|i| i.weight).sum();
            (total - 1.0).abs() <= tolerance
        })
    }
}

/// The mouth-corner joints a rig carries, each `(corner, side sign, head)` —
/// empty when it carries none.
///
/// A corner is a skeleton-backed lone marker leaf off a non-marker parent,
/// off the midline and BELOW its parent joint — above it is a brow, and the
/// jaw is the marker chain. Shared by [`bind`] and the mouth cut's seam
/// assignment (`Avatar::build`), so the two cannot disagree about what a
/// corner is: the seam assignment overwrites the slit's own edge vertices
/// after the bind, and it has to hand back exactly the share this field
/// would have given them or a smile tears at the lip's free edge.
pub(crate) fn mouth_corners(rig: &Rig) -> Vec<(usize, f32, usize)> {
    (0..rig.len())
        .filter(|&joint| face_leaf(rig, joint) && !leaf_above_parent(rig, joint))
        .filter_map(|joint| {
            Some((
                joint,
                rig.joints[joint].position.x.signum(),
                rig.joints[joint].parent?,
            ))
        })
        .collect()
}

/// The brow joints a rig carries, each `(brow, side sign)` — the lone leaves
/// ABOVE the head joint, where [`mouth_corners`] finds the ones below. Shared
/// by [`bind`] and the expression layer, for the reason on
/// [`mouth_corners`].
pub(crate) fn brow_joints(rig: &Rig) -> Vec<(usize, f32)> {
    (0..rig.len())
        .filter(|&joint| face_leaf(rig, joint) && leaf_above_parent(rig, joint))
        .map(|joint| (joint, rig.joints[joint].position.x.signum()))
        .collect()
}

/// The jaw's pivot, if this rig carries the marker chain: the joint the
/// mandible region swings on. The same structural lookup [`bind`] and the
/// mouth cut use — a marker whose child is a marker is the chain, and the
/// pivot is the tip's parent.
pub(crate) fn jaw_pivot(rig: &Rig) -> Option<usize> {
    (0..rig.len()).find_map(|tip| {
        let pivot = rig.joints[tip].parent?;
        (rig.joints[tip].marker && rig.joints[pivot].marker).then_some(pivot)
    })
}

/// Whether `joint` is a skeleton-backed lone marker leaf off a non-marker
/// parent, off the midline — the shape every per-side face joint (a brow, a
/// mouth corner) has. `node.is_some()` excludes the lid joints, which are
/// attached after the first bind with no skeleton node behind them; the
/// jaw's chain is excluded by both ends (the pivot has a marker child, the
/// tip a marker parent).
fn face_leaf(rig: &Rig, joint: usize) -> bool {
    rig.joints[joint].marker
        && rig.joints[joint].node.is_some()
        && rig.joints[joint]
            .parent
            .is_some_and(|parent| !rig.joints[parent].marker)
        && !(0..rig.len())
            .any(|child| rig.joints[child].marker && rig.joints[child].parent == Some(joint))
        && rig.joints[joint].position.x != 0.0
}

/// Which side of its parent joint a face leaf sits: above is a brow, below a
/// mouth corner. Height against the PARENT and not against a constant,
/// because the head joint is the one landmark both families are placed from.
fn leaf_above_parent(rig: &Rig, joint: usize) -> bool {
    rig.joints[joint]
        .parent
        .is_some_and(|parent| rig.joints[joint].position.y > rig.joints[parent].position.y)
}

/// [`face::skull::corner_hold`](crate::face::skull) at `position`, for one
/// entry of [`mouth_corners`].
///
/// The ruler arithmetic lives here once — the below-joint drop off the
/// neck→head pair, the azimuth cosines, the side sign — because two call
/// sites computing it independently is how a seam and a field drift apart.
pub(crate) fn corner_hold_at(rig: &Rig, corner: (usize, f32, usize), position: Vec3) -> f32 {
    let (_, sign, head) = corner;
    let Some(neck) = rig.joints[head].parent else {
        return 0.0;
    };
    let (crest, foot) = (rig.joints[head].position, rig.joints[neck].position);
    let drop = (crest.y - position.y) / (crest.y - foot.y).max(f32::EPSILON);
    let across = Vec3::new(position.x - crest.x, 0.0, position.z - crest.z);
    let reach = across.length();
    if reach <= f32::EPSILON {
        return 0.0;
    }
    skull::corner_hold(drop, across.z / reach, across.x / reach * sign)
}

/// Computes skin weights binding `mesh` to `rig`.
///
/// Every vertex ends up with at most [`MAX_INFLUENCES`] joints whose weights sum
/// to one, sorted strongest first.
///
/// Only [`crate::rig::Role::Deform`] joints are bound against. A rig may carry
/// joints the body is not made of — a spring chain down a lock of hair, a face
/// rig, a socket a prop hangs from — and the body's surface has to ignore every
/// one of them. Binding against a bone with no capsule behind it does not fail
/// loudly; it quietly attaches a patch of skin to something that was never
/// meant to move it.
#[must_use]
pub fn bind(mesh: &PolyMesh, rig: &Rig, config: &SkinConfig) -> SkinWeights {
    let vertices = mesh.positions.len();
    let joints = rig.len();
    let Some(first_deforming) = rig.deforming().next() else {
        return SkinWeights::default();
    };
    if vertices == 0 || joints == 0 {
        return SkinWeights::default();
    }

    // The jaw chain, if this rig carries one: `(pivot, head, neck)`. The two
    // markers are excluded from the falloff loop below; their skin is the
    // REGION `face::skull::mandible_hold` describes — the owner's contract,
    // lower lip to larynx, gonion to ear — written into the weights directly.
    // A distance falloff was measured unable to say that (#135: the chin's
    // flank and the upper lip sit 27.4 and 28.5 mm from the bone, and nothing
    // keyed to distance can hold one and release the other).
    let jaw = (0..joints)
        .find(|&tip| {
            rig.joints[tip].marker
                && rig.joints[tip]
                    .parent
                    .is_some_and(|pivot| rig.joints[pivot].marker)
        })
        .and_then(|tip| {
            let pivot = rig.joints[tip].parent?;
            let head = rig.joints[pivot].parent?;
            let neck = rig.joints[head].parent?;
            Some((pivot, head, neck))
        });

    // The brows, if this rig carries them (#215): the lone marker leaves
    // ABOVE the head joint — see [`face_leaf`] for the identification and why
    // `node.is_some()` is load-bearing in it. Their skin is the region
    // `face::skull::brow_hold` describes, taken from the head's hold the way
    // the mandible takes its own — and from the crown's, whose bone begins to
    // win across the field's upper fade; a share taken from the head alone
    // would leave a crown-held stripe standing still inside a moving patch.
    // Each entry is `(brow, side sign, head, crown)`.
    let brows: Vec<(usize, f32, usize, Option<usize>)> = brow_joints(rig)
        .into_iter()
        .filter_map(|(joint, sign)| {
            let head = rig.joints[joint].parent?;
            let crown = (0..joints).find(|&child| {
                rig.joints[child].parent == Some(head)
                    && !rig.joints[child].marker
                    && rig.joints[child].role.deforms()
            });
            Some((joint, sign, head, crown))
        })
        .collect();

    // The mouth corners (#216): the lone leaves BELOW the head joint. Their
    // region is `face::skull::corner_hold`, a patch astride the slit — taken
    // from the head AND the jaw, because the seam's two edges are held by
    // different bones (the lower is wholly the jaw's, #154) and a corner that
    // took from only one of them would tear the commissure the first time it
    // smiled.
    let corners = mouth_corners(rig);

    let mut dense = vec![0.0f32; vertices * joints];
    for (vertex, &position) in mesh.positions.iter().enumerate() {
        let row = &mut dense[vertex * joints..(vertex + 1) * joints];
        let mut nearest = (f32::INFINITY, first_deforming);
        // Which body part this vertex is ON, asked exactly the way
        // [`crate::face::skull::shape`] asks it. See [`owner_of`]: for one bone
        // in the rig the answer decides which end of it turns this vertex.
        let mine = rig.joints[rig.nearest_bone(position).joint].zone;

        // Iterated by *segment*, credited to the joint that deforms it. See the
        // module docs: `rig.bone(segment)` runs `parent → segment`, and it is
        // the parent's rotation that turns it.
        for segment in 0..joints {
            if !rig.joints[segment].role.deforms() || rig.joints[segment].marker {
                continue;
            }
            let (start, end) = rig.bone(segment);
            let (start_radius, end_radius) = rig.bone_radii(segment);
            let (distance, along) = distance_to_segment(position, start, end);
            let radius = start_radius + (end_radius - start_radius) * along;
            let owner = owner_of(rig, segment, mine, along, start.distance(end), position);
            if !rig.joints[owner].role.deforms() {
                continue;
            }

            if distance < nearest.0 {
                nearest = (distance, owner);
            }
            let span = radius * config.reach;
            if span > 0.0 {
                // Several bones may leave one joint — a pelvis has three — and
                // a joint's hold on a vertex is the strongest of them rather
                // than their sum, which would give a crotch vertex twice the
                // pull of a thigh one for no reason but the topology above it.
                let pull = (1.0 - distance / span).max(0.0).powf(config.falloff);
                row[owner] = row[owner].max(pull);
            }
        }

        // A vertex beyond every bone's reach still has to belong somewhere.
        if row.iter().all(|w| *w <= 0.0) {
            row[nearest.1] = 1.0;
        }

        // The lower-jaw region (#152): the mandible takes its share of the
        // head's and the neck's hold, and only theirs — the field is defined
        // against the neck→head bone's own span, the ruler `owner_of` already
        // measures the chin and the throat with.
        // The brow regions (#215). The ruler is height above the head joint in
        // HEAD RADII: the brow bands sit above the joint where the below-joint
        // span has no meaning, and the radius is the one measure of the skull
        // that `face_length` does not stretch (#135's ruler lesson, applied in
        // the direction it was learned).
        for &(brow, sign, head, crown) in &brows {
            let crest = rig.joints[head].position;
            let radius = rig.joints[head].radius.max(f32::EPSILON);
            let rise = (position.y - crest.y) / radius;
            let across = Vec3::new(position.x - crest.x, 0.0, position.z - crest.z);
            let reach = across.length();
            if reach <= f32::EPSILON {
                continue;
            }
            let hold = skull::brow_hold(rise, across.z / reach, across.x / reach * sign);
            if hold > 0.0 {
                let mut taken = hold * row[head];
                row[head] *= 1.0 - hold;
                if let Some(crown) = crown {
                    taken += hold * row[crown];
                    row[crown] *= 1.0 - hold;
                }
                row[brow] += taken;
            }
        }

        if let Some((pivot, head, neck)) = jaw {
            let (foot, crest) = (rig.joints[neck].position, rig.joints[head].position);
            // Pure HEIGHT, not a projection onto the bone: the neck→head bone
            // leans — its foot sits astern — so projecting onto it folds a
            // vertex's forward reach into its height, and the one place that
            // matters is exactly the place this field is for. Measured before
            // this was fixed: the lip and chin front at z +95 mm read 0.09
            // below-joint units high, landing above the mouth line, and the
            // lower lip came out held 0.997 by the skull.
            let drop = (crest.y - position.y) / (crest.y - foot.y).max(f32::EPSILON);
            let across = Vec3::new(position.x - crest.x, 0.0, position.z - crest.z);
            let reach = across.length();
            if reach > f32::EPSILON {
                let hold = skull::mandible_hold(drop, across.z / reach, across.x / reach);
                if hold > 0.0 {
                    let taken = hold * (row[head] + row[neck]);
                    row[head] *= 1.0 - hold;
                    row[neck] *= 1.0 - hold;
                    row[pivot] += taken;
                }
            }
        }

        // The mouth corners (#216), AFTER the mandible on purpose: the corner
        // takes its share from the head and from the jaw, and the jaw's share
        // at the commissure only exists once the mandible region has taken
        // it. Taking from both is the seam contract — the slit's edges are
        // head-held above and jaw-held below (#154), and a corner that moved
        // one edge without the other would tear the mouth at its own corner.
        for &corner in &corners {
            let hold = corner_hold_at(rig, corner, position);
            if hold > 0.0 {
                let (joint, _, head) = corner;
                let mut taken = hold * row[head];
                row[head] *= 1.0 - hold;
                if let Some((pivot, ..)) = jaw {
                    taken += hold * row[pivot];
                    row[pivot] *= 1.0 - hold;
                }
                row[joint] += taken;
            }
        }
        normalize(row);
    }

    smooth(&mut dense, mesh, joints, config);

    SkinWeights {
        vertices: (0..vertices)
            .map(|vertex| strongest(&dense[vertex * joints..(vertex + 1) * joints]))
            .collect(),
    }
}

/// Which joint's rotation turns the bone leading into `segment`, for a vertex
/// lying on the body part `on`.
///
/// **Almost always the parent, and the exception is the head.** The
/// module docs above give the rule and why it is the parent: rotating a hip
/// swings the thigh, so the bone `hip → knee` is the hip's. That is right for
/// every hinge in the body, and a head is not a hinge.
///
/// A head is a **rigid body that extends below its own joint**, and it is the
/// only part of this rig that does. The head node sits `HEAD_BELOW_JOINT`
/// radii above the neck node, so the whole lower face — the jaw, the chin, the
/// mouth, the base of the nose — hangs off the `neck → head` bone, below the
/// joint that ought to turn it. Credited to the parent, every one of those
/// vertices belongs to the NECK. Measured on the default body under that
/// rule: the mouth line and the chin are held 1.00 by the neck and move
/// **zero millimetres** when the head turns thirty degrees, against the 57.4
/// and 60.6 mm a rigid head moves them; the brow at 0.75/0.25 loses 14.7 mm
/// of 58.7. The face stays behind while the skull turns out from under it —
/// the nose and chin smear under look-at.
///
/// And the deeper the head reaches below its joint — the built floor sits
/// 1.07 to 1.16 radii below it — the more of the face the parent rule hands
/// to the neck. Nothing else here measures a head turn, so nothing else would
/// say so.
///
/// The test is the vertex's own body part rather than a distance, because a
/// distance cannot answer it: the chin's tip sits **1.34 head radii** from the
/// head joint — `shape` pushes it there — so no reach keyed to the node radius
/// covers the chin without also swallowing the throat. `on` comes from
/// [`Rig::nearest_bone`], which is the same question `skull::shape` and
/// `relief::carve` ask to decide what the head is. Two functions disagreeing
/// about which vertices are the head is the defect; agreeing is the fix.
///
/// The neck keeps the bone for vertices whose part is the neck, so the throat
/// still follows the neck and rotating the neck still deforms something —
/// giving the whole bone to the head would have left the neck joint owning
/// nothing at all.
fn owner_of(rig: &Rig, segment: usize, on: Zone, along: f32, length: f32, position: Vec3) -> usize {
    let parent = rig.joints[segment].parent.unwrap_or(segment);
    if on != Zone::Head
        || rig.joints[segment].zone != Zone::Head
        || rig.joints[parent].zone == Zone::Head
    {
        return parent;
    }
    // **The split has to land between the chin and the throat, and both of
    // those are fixed fractions of this bone on every body.** The zone test
    // alone hands the whole bone over, because the head's SURFACE runs on past
    // its jaw to meet the neck — `shape` says so, and `SETTLE` exists because it
    // does — so a vertex at the head's floor answers `Zone::Head` while plainly
    // belonging to the throat. Measured on the default body with the zone test
    // alone, the throat came out held 0.79 by the head and 0.03 by the neck,
    // which turns a windpipe with a glance.
    //
    // The bone is `neck → head` and its length is `head_r * HEAD_BELOW_JOINT`,
    // so everything below the joint is a share of it that does not depend on
    // either. `shape` puts the head's floor at 0.896 of that extent and the chin
    // at 0.599 — measured to three figures at `HEAD_BELOW_JOINT` 1.19 and again
    // at 1.55, where they read 0.8958/0.5953 and 0.8956/0.5988. So the chin's
    // projection is 0.40 along the bone and the throat's floor 0.10, on every
    // body, and the boundary belongs halfway between them.
    //
    // **This was `head_radius / length`, which is `1 / HEAD_BELOW_JOINT` written
    // in a way that hides it** (#79). At 1.19 that is 0.84 and the boundary
    // lands at 0.16 — inside the gap, which is why it worked, but only 19% of
    // the way up from the throat rather than halfway. The expression runs the
    // wrong way: the further the head reaches below its joint, the LESS of the
    // bone it claims. At 1.55 it gives 0.645, putting the boundary at 0.355
    // against a chin at 0.401 — seven millimetres apart, close enough that
    // `smooth` blended neck weight onto the chin and
    // `the_whole_face_turns_with_the_head` left a chin vertex 8 mm behind.
    //
    // A constant was right here while the boundary had one landmark pair to
    // separate, and #152 gave it two. `along` is a projection onto the bone,
    // which is vertical — that the chin also stands 1.34 radii FORWARD does
    // not enter into it, and is why a plain distance to the node cannot make
    // this call. But how far DOWN the head's claim should run is not one
    // number round the column: on the FRONT the lower-jaw region needs a
    // head-family base under it all the way to the larynx line, and on the
    // BACK the skull is rigid only to the occiput — the flat 0.75 (which is
    // [`skull::LARYNX`] minus a smoothing margin, halfway between the chin at
    // 0.40-along and the throat floor at 0.10) gave the head the nape too, so
    // a head turn sheared the whole column as one piece (#151's second
    // finding). The boundary now swings with azimuth between the two measured
    // landmarks: [`skull::LARYNX`] dead ahead, [`skull::NAPE`] behind, the
    // same constants the mandible field and the carve read.
    if length <= f32::EPSILON {
        return segment;
    }
    // The swing uses the mandible field's own azimuth release, not a bare
    // cosine: the head's claim has to stay at full depth for the whole face
    // AND its sides — the under-ear jawline is the region's base, and a
    // boundary that fell toward the nape at 90° was measured handing it to
    // the neck, which the field then split 0.7/0.3 and a head turn left the
    // neck's share 5.9 mm behind. Only past the ear does the column begin.
    let crest = rig.joints[segment].position;
    let across = Vec3::new(position.x - crest.x, 0.0, position.z - crest.z);
    let facing = across.z / across.length().max(f32::EPSILON);
    let round = crate::face::smooth((facing + 0.30) / 0.30);
    let covered = skull::NAPE + (skull::LARYNX - skull::NAPE) * round;
    if along >= 1.0 - covered {
        segment
    } else {
        parent
    }
}

/// Distance from `point` to the segment `start..end`, and how far along it fell.
fn distance_to_segment(point: Vec3, start: Vec3, end: Vec3) -> (f32, f32) {
    let axis = end - start;
    let length_squared = axis.length_squared();
    if length_squared <= f32::EPSILON {
        return (point.distance(start), 0.0);
    }
    let along = ((point - start).dot(axis) / length_squared).clamp(0.0, 1.0);
    (point.distance(start + axis * along), along)
}

/// Scales a row so its weights sum to one.
fn normalize(row: &mut [f32]) {
    let total: f32 = row.iter().sum();
    if total > 0.0 {
        for weight in row.iter_mut() {
            *weight /= total;
        }
    }
}

/// Blends each vertex's weights toward its neighbours'.
///
/// Distance alone gives a sharp boundary wherever two bones' influence meets,
/// which shows up as a crease across a broad region under animation. Averaging
/// over the surface turns that boundary into a gradient.
fn smooth(dense: &mut [f32], mesh: &PolyMesh, joints: usize, config: &SkinConfig) {
    if config.smoothing_iterations == 0 || config.smoothing_strength <= 0.0 {
        return;
    }
    let neighbors = adjacency(mesh);

    for _ in 0..config.smoothing_iterations {
        let previous = dense.to_vec();
        for (vertex, near) in neighbors.iter().enumerate() {
            if near.is_empty() {
                continue;
            }
            let row = &mut dense[vertex * joints..(vertex + 1) * joints];
            for (joint, weight) in row.iter_mut().enumerate() {
                let average: f32 = near
                    .iter()
                    .map(|&other| previous[other * joints + joint])
                    .sum::<f32>()
                    / near.len() as f32;
                *weight += (average - *weight) * config.smoothing_strength;
            }
            normalize(row);
        }
    }
}

/// Vertices sharing an edge with each vertex.
fn adjacency(mesh: &PolyMesh) -> Vec<Vec<usize>> {
    let mut sets: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); mesh.positions.len()];
    for face in &mesh.faces {
        for corner in 0..face.len() {
            let a = face[corner] as usize;
            let b = face[(corner + 1) % face.len()] as usize;
            if a < sets.len() && b < sets.len() {
                sets[a].insert(b);
                sets[b].insert(a);
            }
        }
    }
    sets.into_iter()
        .map(|set| set.into_iter().collect())
        .collect()
}

/// The strongest influences in a row, normalised and sorted.
fn strongest(row: &[f32]) -> [Influence; MAX_INFLUENCES] {
    let mut ranked: Vec<Influence> = row
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight > 0.0)
        .map(|(joint, &weight)| Influence {
            joint: joint as u16,
            weight,
        })
        .collect();
    ranked.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    ranked.truncate(MAX_INFLUENCES);

    let total: f32 = ranked.iter().map(|i| i.weight).sum();
    let mut out = [Influence::default(); MAX_INFLUENCES];
    if total <= 0.0 {
        out[0].weight = 1.0;
        return out;
    }
    for (slot, influence) in out.iter_mut().zip(ranked) {
        *slot = Influence {
            joint: influence.joint,
            weight: influence.weight / total,
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cage::{CageConfig, build_cage};
    use crate::plan::{BodyPlan, HumanoidParams};
    use crate::subdiv::catmull_clark;

    fn humanoid() -> (PolyMesh, Rig) {
        let skeleton = HumanoidParams::default().skeleton(&crate::Composites::default());
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        (catmull_clark(&cage, crate::BODY_SUBDIVISIONS), rig)
    }

    #[test]
    fn every_vertex_is_bound_and_normalized() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());

        assert_eq!(skin.vertices.len(), mesh.vertex_count());
        assert!(skin.is_normalized(1e-4), "weights must sum to one");
        for (vertex, influences) in skin.vertices.iter().enumerate() {
            assert!(
                influences[0].weight > 0.0,
                "vertex {vertex} has no dominant joint"
            );
            assert!(
                influences.iter().all(|i| (i.joint as usize) < rig.len()),
                "vertex {vertex} references a joint that does not exist"
            );
        }
    }

    #[test]
    fn influences_are_sorted_strongest_first() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());
        for influences in &skin.vertices {
            for pair in influences.windows(2) {
                assert!(pair[0].weight >= pair[1].weight);
            }
        }
    }

    #[test]
    fn vertices_bind_to_the_body_part_they_sit_on() {
        let (mesh, rig) = humanoid();
        let skin = bind(&mesh, &rig, &SkinConfig::default());
        let zones = skin.zone_map(&mesh, &rig);

        // The highest vertex is on the head; the lowest is on a foot.
        let highest = argmax(&mesh, |p| p.y);
        assert_eq!(zones[highest], Zone::Head);

        let lowest = argmax(&mesh, |p| -p.y);
        assert!(
            matches!(zones[lowest], Zone::Extremity(limb) if !limb.is_fore()),
            "lowest vertex should be a foot, got {:?}",
            zones[lowest]
        );

        // The most lateral vertices are hands, and they are on opposite sides.
        let left = argmax(&mesh, |p| -p.x);
        let right = argmax(&mesh, |p| p.x);
        let (Zone::Extremity(left_limb), Zone::Extremity(right_limb)) = (zones[left], zones[right])
        else {
            panic!(
                "outermost vertices should be hands: {:?}",
                (zones[left], zones[right])
            );
        };
        assert!(left_limb.is_fore() && right_limb.is_fore());
        assert_eq!(left_limb.mirrored(), right_limb);
    }

    #[test]
    fn smoothing_softens_the_boundary_between_bones() {
        let (mesh, rig) = humanoid();
        let sharp = bind(
            &mesh,
            &rig,
            &SkinConfig {
                smoothing_iterations: 0,
                ..Default::default()
            },
        );
        let soft = bind(&mesh, &rig, &SkinConfig::default());

        // A smoothed bind spreads each vertex over more bones.
        let blended =
            |skin: &SkinWeights| skin.vertices.iter().filter(|i| i[1].weight > 0.01).count();
        assert!(
            blended(&soft) > blended(&sharp),
            "smoothing should blend more vertices across bones"
        );
        assert!(soft.is_normalized(1e-4));
    }

    #[test]
    fn binding_is_deterministic() {
        let (mesh, rig) = humanoid();
        let config = SkinConfig::default();
        assert_eq!(bind(&mesh, &rig, &config), bind(&mesh, &rig, &config));
    }

    #[test]
    fn an_empty_mesh_binds_to_nothing() {
        let (_, rig) = humanoid();
        let skin = bind(&PolyMesh::new(), &rig, &SkinConfig::default());
        assert!(skin.vertices.is_empty());
    }

    #[test]
    fn the_whole_face_turns_with_the_head() {
        // **The defect this test exists for is that a head turn left the face
        // behind** (#123). The head node sits a head-and-a-fifth of its own
        // radius above the neck node, so the jaw, the chin, the mouth and the
        // base of the nose all hang off the `neck → head` bone — below the joint
        // that has to turn them. Credited to the bone's parent, as every other
        // bone correctly is, all of it belonged to the NECK: measured on the
        // default body, the mouth line and the chin were held 1.00 by the neck
        // and moved ZERO millimetres when the head turned thirty degrees,
        // against the 57.4 and 60.6 mm a rigid head moves them. Under look-at
        // the skull turned out from under its own face.
        //
        // Nothing said so because nothing had ever posed a head. The gait tests
        // turn hips and knees, the reference comparison is a rest pose, and the
        // software renderer had no way to ask for a gaze until this issue — so
        // the only instrument that could see it was the Bevy viewer, and it took
        // the owner looking at one.
        //
        // A head is rigid, so the assertion is exact rather than a tolerance on
        // a blend: every head-owned vertex must land where rotating it about the
        // head joint would put it. Half a millimetre is the arithmetic's noise,
        // not a budget for lag.
        use crate::anim::Pose;
        use glam::Quat;

        let record = crate::AvatarRecord::new("Turned", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let head = *rig
            .in_zone(Zone::Head)
            .first()
            .expect("a humanoid has a head");
        let centre = rig.joints[head].position;
        // The FACE: forward of the head joint and at or above the chin's tip.
        // Both bounds are the blend's, and both were found by running without
        // them. Below the chin the surface is the underside of the jaw running
        // back into the throat, and behind the joint it is the nape — each is a
        // place the head's hold must hand over to the neck's, so each is a blend
        // by design and `the_throat_stays_with_the_neck` is what checks that
        // side. The first version bounded the face at one head radius below the
        // joint and failed on a vertex 98 mm down at the angle of the jaw; the
        // second bounded it at the chin alone and failed on one 52 mm behind the
        // joint at the nape. Both were the blend doing its job, and neither is
        // the face.
        let chin = crate::face::Skull::measure(&avatar.parts.body, rig)
            .expect("a humanoid has a skull")
            .chin();

        let turn = Quat::from_rotation_y(30f32.to_radians());
        let mut pose = Pose::rest(rig);
        pose.rotations[head] = turn;
        let matrices = pose.forward(rig).skinning_matrices(rig);

        let body = &avatar.parts.body;
        let mut worst = (0usize, 0.0f32, Vec3::ZERO);
        let mut checked = 0usize;
        for (vertex, &rest) in body.positions.iter().enumerate() {
            if rig.joints[rig.nearest_bone(rest).joint].zone != Zone::Head {
                continue;
            }
            if rest.y < centre.y + chin || rest.z <= centre.z {
                continue;
            }
            checked += 1;
            let mut skinned = Vec3::ZERO;
            for influence in avatar.parts.weights.vertices[vertex] {
                if influence.weight > 0.0 {
                    skinned += influence.weight
                        * matrices[influence.joint as usize].transform_point3(rest);
                }
            }
            let rigid = centre + turn * (rest - centre);
            let lag = (rigid - skinned).length();
            if lag > worst.1 {
                worst = (vertex, lag, rest - centre);
            }
        }
        // A filter that selects nothing passes every assertion after it, which
        // is the shape of half the instrument failures this crate has found.
        assert!(
            checked > 500,
            "only {checked} vertices were read as the face; the filter, not the \
             binding, is what this would be measuring"
        );
        assert!(
            worst.1 < 0.0005,
            "a head turn left a face vertex {:.1} mm behind, at {:+.0},{:+.0},{:+.0} mm \
             from the head joint, of {checked} read — the face is bound to something \
             that is not the head",
            worst.1 * 1000.0,
            worst.2.x * 1000.0,
            worst.2.y * 1000.0,
            worst.2.z * 1000.0,
        );
    }

    #[test]
    fn the_jaw_opens_and_the_skull_does_not() {
        // **The mandible bone existed for a whole issue before anything rotated
        // it** (#134 built it, #135 posed it). Nothing caught that its binding
        // reach was wrong, and the reason is worth stating because it defeats
        // every other test here: dual quaternion blending deforms weights split
        // between the head bone and the jaw bone IDENTICALLY at rest and under
        // a head turn. `the_whole_face_turns_with_the_head` passes at any reach
        // whatsoever — the jaw hangs off the head, so a head turn carries it
        // rigidly however the skin is shared. Only turning the PIVOT tells a
        // good binding from a bad one.
        //
        // What was measured at the unsourced first cut of `(0.24, 0.30)`: at a
        // 20-degree open the upper lip travelled 19.2 mm against the lower
        // lip's 28.2, held 0.652 by a bone that must not hold it at all. The
        // lips could not part, which is the one thing a jaw is for.
        //
        // The bands are the crate's own landmarks and nothing else — the mouth
        // line, the base of the nose, the chin, the eye line — and where one
        // needs half of a span it is the half between two of them, the same
        // construction `owner_of` puts its own boundary on. `examples/render`'s
        // `--jawbind` prints the same bands beside the picture; if the two ever
        // disagree, one of them is lying and neither is the binding.
        use crate::anim::Pose;
        use glam::Quat;

        let check = |face_length: f32, what: &str| {
            let mut record = crate::AvatarRecord::new("Jawed", crate::Archetype::default());
            if let crate::Archetype::Humanoid(ref mut params) = record.archetype {
                params.face_length = face_length;
            }
            record.sanitize();
            let avatar = crate::Avatar::build(&record).expect("a biped builds");
            let rig = &avatar.rig;
            let body = &avatar.parts.body;
            let head = *rig.in_zone(Zone::Head).first().expect("a head");
            let centre = rig.joints[head].position;

            // The mandible, by the skeleton's own marker flag: the chain runs
            // head -> pivot -> tip, and only the tip has a marked parent.
            let skeleton = record.skeleton();
            let marked = |joint: usize| {
                rig.joints[joint]
                    .node
                    .is_some_and(|node| skeleton.nodes[node as usize].marker)
            };
            let tip = (0..rig.len())
                .find(|&joint| marked(joint) && rig.joints[joint].parent.is_some_and(marked))
                .expect("a humanoid has a jaw");
            let pivot = rig.joints[tip].parent.expect("the tip hangs off the pivot");

            let skull = crate::face::Skull::measure(body, rig).expect("a skull");
            let canon = crate::face::Canon::measure(rig, &skull, &Default::default());
            let (chin, mouth, nose) = (skull.chin(), canon.mouth_line(), canon.nose_base());

            // The bone is owned by its PROXIMAL joint, so the PIVOT is what
            // opens the mouth. Rotating the tip — a leaf — would move nothing,
            // and a test that did it would pass by measuring nothing.
            let mut pose = Pose::rest(rig);
            pose.rotations[pivot] = Quat::from_rotation_x(20f32.to_radians());
            let moved = pose
                .forward(rig)
                .deform(rig, &body.positions, &avatar.parts.weights);

            // Held, travelled and counted over one band: `(count, jaw's mean
            // hold, head's mean hold, mean travel)`.
            let band = |lo: f32, hi: f32, front: bool| {
                let mut count = 0usize;
                let (mut jaw, mut skull_hold, mut travel) = (0.0f32, 0.0f32, 0.0f32);
                for (vertex, &rest) in body.positions.iter().enumerate() {
                    let local = rest - centre;
                    if local.y < lo || local.y >= hi {
                        continue;
                    }
                    if front && (local.z <= 0.0 || local.x.abs() > canon.unit) {
                        continue;
                    }
                    let hold = |joint: usize| {
                        avatar.parts.weights.vertices[vertex]
                            .iter()
                            .filter(|i| i.joint as usize == joint)
                            .map(|i| i.weight)
                            .sum::<f32>()
                    };
                    count += 1;
                    jaw += hold(pivot);
                    skull_hold += hold(head);
                    travel += (moved[vertex] - rest).length();
                }
                let n = count.max(1) as f32;
                (count, jaw / n, skull_hold / n, travel / n * 1000.0)
            };

            let upper = band(mouth, mouth + (nose - mouth) * 0.5, true);
            let lower = band(chin + (mouth - chin) * 0.5, mouth, true);
            let cranium = band(canon.level, f32::MAX, false);
            let neck = band(f32::MIN, skull.throat_and_crown().0, false);

            // A band that selects nothing passes every assertion after it,
            // which is the shape of half the instrument failures this crate has
            // found. Each of these carries hundreds of vertices when it is
            // reading the thing it is named for.
            for (count, named) in [
                (upper.0, "the upper lip"),
                (lower.0, "the lower lip"),
                (cranium.0, "the cranium"),
                (neck.0, "the neck"),
            ] {
                assert!(
                    count > 100,
                    "{what}: {named} read {count} vertices — the band, not the binding"
                );
            }

            // THE CONTRACT, in the two directions that oppose each other.
            // Measured on the default body at `(0.10, 0.20)`: the upper lip is
            // held 1.000 by the head and travels 0.00 mm, the lower lip travels
            // 13.5 mm. Swept over `face_length` −1 to +1 the upper lip's travel
            // stays under 0.9 mm, which is what the margins here are for.
            //
            // Asserted as "not the JAW's" rather than "the head's", because
            // #216 made the second form false on purpose: near the commissure
            // the upper lip is corner-held, and a corner joint hangs off the
            // head — static under a jaw open, which is all this contract ever
            // required. The head's own share on the band measured 0.711 the
            // day the corners landed; the jaw's, 0.049.
            assert!(
                upper.1 < 0.10,
                "{what}: the upper lip is held {:.3} by the jaw (and {:.3} by the head) — a mouth \
                 whose top lip drops with its bottom one cannot open",
                upper.1,
                upper.2,
            );
            assert!(
                upper.3 < 2.0,
                "{what}: the upper lip travelled {:.2} mm on a jaw that opened 20 degrees, against \
                 the lower lip's {:.2} — the lips are not parting",
                upper.3,
                lower.3,
            );
            // **8.0 → 7.5** (#149). The default face read 8.22 mm when the
            // bound was 8.0 and reads 7.95 after the nape tuck — a 3% shift on
            // a reading with 2.7% of slack, from a change whose only skin-side
            // effect is the neck's rear blend. The mouth was judged open on a
            // `--jaw 20` render before the bound moved; if this figure falls
            // again, suspect the jaw's own binding, not the margin.
            //
            // **7.5 → 10.0, a ratchet UP for the first time** (#152). The
            // mandible became a region — `skull::mandible_hold`, the owner's
            // lip-to-larynx contract — and the lower lip now travels 11.2 mm
            // on the default face and 10.4 on the short one, against the 7.95
            // the old throat-wedge binding delivered. The floor rises onto the
            // new state so the region cannot silently degrade back into the
            // wedge; the upper lip's own bounds held at 1.000/0.00 unchanged.
            assert!(
                lower.3 > 10.0,
                "{what}: the lower lip travelled only {:.2} mm — the jaw is not carrying the mouth",
                lower.3,
            );

            // And the two that must not move at all. The cranium is a rigid
            // skull above a hinge and the neck is below the head's own bone;
            // either of them moving means the reach has grown past the face.
            assert!(
                cranium.3 < 0.05 && cranium.1 <= 0.0,
                "{what}: the cranium travelled {:.3} mm and is held {:.3} by the jaw — a mandible \
                 does not reach the vault",
                cranium.3,
                cranium.1,
            );
            // **Exactly zero to a hundred-thousandth** (#302). The trapezius
            // fill flares the base of the column, and on the short face four
            // head-owned flank vertices at the column's foot rose seven
            // millimetres into the outermost fringe of the jaw's reach: a
            // mean hold of 1e-7 over the band and 0.000013 mm of travel. A
            // real leak — the throat-wedge binding this guards against — is
            // a few hundredths on a hundred throat vertices, which is 5e-4
            // over this band and fifty times the tolerance.
            assert!(
                neck.3 < 0.05 && neck.1 < 1e-5,
                "{what}: the neck travelled {:.3} mm and is held {:.6} by the jaw — read \
                 `the_throat_stays_with_the_neck` before touching COVERED",
                neck.3,
                neck.1,
            );
        };

        // The default, and the SHORT face — which is the binding case and not
        // the obvious one: a short face packs the mouth line closer to the
        // chin, so the reach that clears the upper lip on a long face swallows
        // it on a short one.
        check(0.0, "the default face");
        check(-1.0, "a short face");
    }

    #[test]
    fn the_chin_follows_the_jaw_and_not_the_skull() {
        // The other half of the contract. Under the marker-falloff binding the
        // chin followed at 0.392 and travelled 44% of a rigid mandible's arc,
        // and the reason was structural: a single midline bone's spherical
        // falloff put the chin's flanks 27.4 mm from it and the upper lip
        // 28.5, so nothing keyed to distance could hold one and release the
        // other. #135 named a pair of rami as the fix.
        //
        // **The rami were never built, because #152 dissolved the problem
        // instead**: the mandible's skin became the REGION
        // `face::skull::mandible_hold` describes, lip to larynx and gonion to
        // ear hinge, and `bind` moves the head's and neck's hold onto the jaw
        // by that field. Measured on the region binding (#214, 2026-08-13) the
        // chin band is held 1.000 and travels 97.6% of the rigid arc — the
        // corners are carried, and the floors below are that state's ratchet.
        // The old floors (0.30 and 0.35) were the falloff's residual and would
        // have passed a regression to half the delivered hold.
        use crate::anim::Pose;
        use glam::Quat;

        let record = crate::AvatarRecord::new("Chinned", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let body = &avatar.parts.body;
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let centre = rig.joints[head].position;

        let skeleton = record.skeleton();
        let marked = |joint: usize| {
            rig.joints[joint]
                .node
                .is_some_and(|node| skeleton.nodes[node as usize].marker)
        };
        let tip = (0..rig.len())
            .find(|&joint| marked(joint) && rig.joints[joint].parent.is_some_and(marked))
            .expect("a humanoid has a jaw");
        let pivot = rig.joints[tip].parent.expect("the tip hangs off the pivot");

        let skull = crate::face::Skull::measure(body, rig).expect("a skull");
        let canon = crate::face::Canon::measure(rig, &skull, &Default::default());
        let (chin, mouth) = (skull.chin(), canon.mouth_line());

        let mut pose = Pose::rest(rig);
        pose.rotations[pivot] = Quat::from_rotation_x(20f32.to_radians());
        let moved = pose
            .forward(rig)
            .deform(rig, &body.positions, &avatar.parts.weights);

        let hinge = rig.joints[pivot].position;
        let swing = 2.0 * (20f32.to_radians() * 0.5).sin();
        let (mut count, mut held, mut travel, mut rigid) = (0usize, 0.0f32, 0.0f32, 0.0f32);
        for (vertex, &rest) in body.positions.iter().enumerate() {
            let local = rest - centre;
            if local.y < chin || local.y >= chin + (mouth - chin) * 0.5 {
                continue;
            }
            if local.z <= 0.0 || local.x.abs() > canon.unit {
                continue;
            }
            count += 1;
            held += avatar.parts.weights.vertices[vertex]
                .iter()
                .filter(|i| i.joint as usize == pivot)
                .map(|i| i.weight)
                .sum::<f32>();
            travel += (moved[vertex] - rest).length();
            rigid += rest.distance(hinge) * swing;
        }
        assert!(
            count > 100,
            "the chin read {count} vertices — the band, not the binding"
        );
        let (held, share) = (held / count as f32, travel / rigid);
        // Printed because the margin is what the next binding change needs to
        // see (`docs/instruments.md` rule 9).
        println!(
            "the chin: held {held:.3} by the jaw, {:.1}% of a rigid mandible's arc, \
             {count} vertices",
            share * 100.0
        );
        assert!(
            held > 0.95 && share > 0.90,
            "the chin is held {held:.3} by the jaw and travels {:.1}% of a rigid mandible's arc, \
             of {count} vertices read — the region binding delivered 1.000 and 97.6% (#214), and \
             this floor is that state's ratchet",
            share * 100.0,
        );
    }

    #[test]
    fn the_brows_rise_and_the_lids_do_not() {
        // #215's contract, asserted the way the jaw's is: only POSING the
        // joints says anything about this binding — dual quaternion blending
        // deforms any territory identically at rest and under a head turn, so
        // every other test in the crate is green whatever `brow_hold` says
        // (the #135 lesson, third time now).
        //
        // Three bands in the head-radius ruler the field itself is authored
        // in, at a 10-degree raise:
        // * the BROW band (+0.18..+0.28 radii, frontal) must travel — the
        //   crest sits at +0.19..+0.26 across the brow axis and the pivot's
        //   geometry makes 10 degrees about 13 mm of lift at the crest;
        // * the LID band (+0.02..+0.12) must not: the field is zero below
        //   +0.13 exactly so a raise does not peel the lids open, and what
        //   leaks below is the bind-time smoothing pass, which this bounds;
        // * the CROWN band (above +0.70) must not: the fade dies at +0.55 and
        //   a scalp that rides a brow raise is the shear #152's top-blend
        //   lesson warns about, one territory later.
        use crate::anim::Pose;
        use glam::Quat;

        let record = crate::AvatarRecord::new("Browed", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let body = &avatar.parts.body;
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let crest = rig.joints[head].position;
        let radius = rig.joints[head].radius;

        // The same structural identification `bind` uses: skeleton-backed
        // marker leaves off the midline, ABOVE the head joint. The lid joints
        // are marker leaves too but carry no node, and the mouth corners
        // (#216) are the same shape of leaf below the joint — if this find
        // ever returns more than two, one of those distinctions has broken
        // and the forehead belongs to nobody knowable.
        let brows: Vec<usize> = (0..rig.len())
            .filter(|&joint| {
                rig.joints[joint].marker
                    && rig.joints[joint].node.is_some()
                    && rig.joints[joint]
                        .parent
                        .is_some_and(|parent| !rig.joints[parent].marker)
                    && !(0..rig.len()).any(|child| {
                        rig.joints[child].marker && rig.joints[child].parent == Some(joint)
                    })
                    && rig.joints[joint].position.x != 0.0
                    && rig.joints[joint].parent.is_some_and(|parent| {
                        rig.joints[joint].position.y > rig.joints[parent].position.y
                    })
            })
            .collect();
        assert_eq!(brows.len(), 2, "a humanoid carries one brow joint per side");

        let mut pose = Pose::rest(rig);
        for &brow in &brows {
            pose.rotations[brow] = Quat::from_rotation_x(-10f32.to_radians());
        }
        let moved = pose
            .forward(rig)
            .deform(rig, &body.positions, &avatar.parts.weights);

        let band = |low: f32, high: f32| -> (usize, f32) {
            let (mut count, mut travel) = (0usize, 0.0f32);
            for (vertex, &rest) in body.positions.iter().enumerate() {
                let local = rest - crest;
                let rise = local.y / radius;
                if rise < low || rise >= high || local.z <= 0.0 {
                    continue;
                }
                count += 1;
                travel += (moved[vertex] - rest).length();
            }
            (count, travel / count.max(1) as f32 * 1000.0)
        };
        let (brow_count, brow_travel) = band(0.18, 0.28);
        let (lid_count, lid_travel) = band(0.02, 0.12);
        let (crown_count, crown_travel) = band(0.70, 2.00);
        // Printed because the margins are what the next face-bone territory
        // needs to see (`docs/instruments.md` rule 9).
        println!(
            "a 10-degree raise: the brow band travels {brow_travel:.2} mm ({brow_count} \
             vertices), the lid band {lid_travel:.2} ({lid_count}), the crown \
             {crown_travel:.2} ({crown_count})"
        );
        assert!(
            brow_count > 50 && lid_count > 50 && crown_count > 50,
            "a band is starved of vertices — the ruler moved, not the binding"
        );
        assert!(
            brow_travel > 4.0,
            "the brow band travels {brow_travel:.2} mm at a 10-degree raise — the territory \
             does not articulate"
        );
        assert!(
            lid_travel < 1.0,
            "the lid band travels {lid_travel:.2} mm under a brow raise — the field is \
             reaching below its own floor and a raise will peel the eyes open"
        );
        assert!(
            crown_travel < 0.5,
            "the crown travels {crown_travel:.2} mm under a brow raise — the fade is not \
             dying by the hairline"
        );
    }

    #[test]
    fn the_corners_smile_and_the_face_does_not() {
        // #216's contract, the third of its family (#135, #215): only posing
        // the joints says anything about the binding. A 15-degree smile, four
        // bands in the below-joint ruler `corner_hold` is authored in:
        // * each CORNER patch must travel — the lever from the pivot to the
        //   commissure is ~16 mm on the default, capped at 0.72;
        // * the PHILTRUM must not: the pose's lift is proportional to x off
        //   the pivot, and the field dies by side 0.10, so the midline is
        //   protected twice over — this asserts both protections survived;
        // * the CHIN's midline must not — the field's height band ends well
        //   above it;
        // * the BROW band must not: a smile that raises the brows has crossed
        //   two territories, which no field here may do.
        use crate::anim::Pose;
        use glam::Quat;

        let record = crate::AvatarRecord::new("Smiled", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let body = &avatar.parts.body;
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let crest = rig.joints[head].position;
        let neck = rig.joints[head].parent.expect("a head sits on a neck");
        let span = crest.y - rig.joints[neck].position.y;
        let radius = rig.joints[head].radius;

        let corners = mouth_corners(rig);
        assert_eq!(
            corners.len(),
            2,
            "a humanoid carries one mouth-corner joint per side"
        );

        let mut pose = Pose::rest(rig);
        for &(corner, sign, _) in &corners {
            pose.rotations[corner] = Quat::from_rotation_z(sign * 15f32.to_radians());
        }
        let moved = pose
            .forward(rig)
            .deform(rig, &body.positions, &avatar.parts.weights);

        // `below` in span fractions, `out` in span fractions of |x|; frontal
        // only. The corner band brackets the line's 0.345 and the corner's
        // side range; the numbers are #216's probe.
        let band = |low: f32, high: f32, out_low: f32, out_high: f32| -> (usize, f32) {
            let (mut count, mut travel) = (0usize, 0.0f32);
            for (vertex, &rest) in body.positions.iter().enumerate() {
                let local = rest - crest;
                let below = -local.y / span;
                let out = local.x.abs() / span;
                if below < low || below >= high || out < out_low || out >= out_high {
                    continue;
                }
                if local.z <= 0.0 {
                    continue;
                }
                count += 1;
                travel += (moved[vertex] - rest).length();
            }
            (count, travel / count.max(1) as f32 * 1000.0)
        };
        let (corner_count, corner_travel) = band(0.28, 0.42, 0.10, 0.30);
        let (philtrum_count, philtrum_travel) = band(0.24, 0.34, 0.00, 0.04);
        let (chin_count, chin_travel) = band(0.50, 0.70, 0.00, 0.10);
        // The brow band, in the head-radius ruler `brow_hold` uses.
        let (mut brow_count, mut brow_travel) = (0usize, 0.0f32);
        for (vertex, &rest) in body.positions.iter().enumerate() {
            let local = rest - crest;
            let rise = local.y / radius;
            if !(0.18..0.28).contains(&rise) || local.z <= 0.0 {
                continue;
            }
            brow_count += 1;
            brow_travel += (moved[vertex] - rest).length();
        }
        let brow_travel = brow_travel / brow_count.max(1) as f32 * 1000.0;

        println!(
            "a 15-degree smile: the corner bands travel {corner_travel:.2} mm ({corner_count} \
             vertices), the philtrum {philtrum_travel:.2} ({philtrum_count}), the chin \
             {chin_travel:.2} ({chin_count}), the brows {brow_travel:.2} ({brow_count})"
        );
        assert!(
            corner_count > 50 && philtrum_count > 20 && chin_count > 50 && brow_count > 50,
            "a band is starved of vertices — the ruler moved, not the binding"
        );
        assert!(
            corner_travel > 1.0,
            "the corner bands travel {corner_travel:.2} mm at a 15-degree smile — the territory \
             does not articulate"
        );
        assert!(
            philtrum_travel < 0.6,
            "the philtrum travels {philtrum_travel:.2} mm under a smile — the field is crossing \
             the midline"
        );
        assert!(
            chin_travel < 0.6,
            "the chin travels {chin_travel:.2} mm under a smile — the field is reaching into the \
             mandible's own territory"
        );
        assert!(
            brow_travel < 0.3,
            "the brows travel {brow_travel:.2} mm under a smile — two territories have crossed"
        );
    }

    #[test]
    fn the_throat_stays_with_the_neck() {
        // The other side of the same boundary, and the reason `owner_of` is
        // bounded by the head node's own extent rather than by the zone alone.
        // The head's SURFACE runs past its jaw down to meet the neck — `shape`
        // says so and `SETTLE` exists because it does — so a vertex at the
        // head's floor answers `Zone::Head` while plainly being a throat. With
        // the zone test alone it came out held 0.79 by the head and 0.03 by the
        // neck, which turns a windpipe with a glance.
        let record = crate::AvatarRecord::new("Throated", crate::Archetype::default());
        let avatar = crate::Avatar::build(&record).expect("a biped builds");
        let rig = &avatar.rig;
        let head = *rig.in_zone(Zone::Head).first().expect("a head");
        let neck = rig.joints[head].parent.expect("a head sits on a neck");
        let centre = rig.joints[head].position;

        // The forward-most vertex at the head's own floor: the throat.
        //
        // **Along the midline first and down second, which is not the order
        // this was written in** (#106). Taking the zone's own lowest vertex
        // and then asking for one near the midline finds nothing: the lowest
        // head-zone vertices are a symmetric pair 45% of a head radius out to
        // each side, the throat sits 8 mm above them, and a 3%-of-a-radius
        // window is 3 mm. It passed for as long as it did because a midline
        // vertex happened to be the lowest, and it stopped the day the girdle
        // widened — a body-plan change three joints away, with nothing between
        // it and this test but the mesh. Same shape of failure the file's other
        // docstrings record; the fix is to measure the quantity the comment
        // names rather than one that usually coincides with it.
        let midline = |at: &Vec3| (at.x - centre.x).abs() < rig.joints[head].radius * 0.05;
        let floor = avatar
            .parts
            .body
            .positions
            .iter()
            .filter(|at| midline(at) && rig.joints[rig.nearest_bone(**at).joint].zone == Zone::Head)
            .fold(f32::MAX, |low, at| low.min(at.y));
        let throat = avatar
            .parts
            .body
            .positions
            .iter()
            .enumerate()
            .filter(|(_, at)| midline(at) && (at.y - floor).abs() < rig.joints[head].radius * 0.03)
            .max_by(|a, b| a.1.z.total_cmp(&b.1.z))
            .map(|(vertex, _)| vertex)
            .expect("a body has a throat");

        let held = |joint: usize| -> f32 {
            avatar.parts.weights.vertices[throat]
                .iter()
                .find(|influence| influence.joint as usize == joint)
                .map_or(0.0, |influence| influence.weight)
        };
        assert!(
            held(neck) > held(head),
            "the throat is held {:.2} by the head and {:.2} by the neck",
            held(head),
            held(neck)
        );
    }

    /// Index of the vertex scoring highest under `score`.
    fn argmax(mesh: &PolyMesh, score: impl Fn(Vec3) -> f32) -> usize {
        mesh.positions
            .iter()
            .enumerate()
            .max_by(|a, b| score(*a.1).total_cmp(&score(*b.1)))
            .expect("mesh has vertices")
            .0
    }
}
