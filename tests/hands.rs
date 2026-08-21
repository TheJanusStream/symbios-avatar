//! The hands: one welded surface from forearm to fingertip, a domed knuckle
//! row, a thenar mound, reference proportions, and two hands that are
//! reflections of each other.
//!
//! **Guards fitted AFTER the geometry was agreed by render** — milestone
//! #10's standing method (#296). #297 welded the hand into the arm, #298
//! rebuilt it as one extruded solid with 21 bones kept, and #299 shaped it:
//! a knuckle row arched across the palm, fingers deeper than wide that taper
//! to a pad, a thenar mound under the thumb. Every bound here was fitted to
//! that tree and checked against the tree before it: the pre-hand body
//! (`669ed06`, the parent of #297's first commit) reads a 1.9 mm silhouette
//! step across the wrist crease where the palm's base sat over the arm, SIX
//! surface pieces per hand (the palm and five digits), a knuckle row arched
//! by nothing, a palm with no thenar mound (1.000 by symmetry), and a hand
//! 8.9 to 9.3% of stature long — and each of those is outside the bound that
//! guards it. The one guard that tree passes is the reflection: the appended
//! hands reflected exactly, and it is the WELD that reads 2 mm at the worst
//! vertex now (see `the_two_hands_are_reflections_to_the_float`).
//!
//! **Everything is read off the DRAWN skin** — [`Avatar::drawn`], which is
//! the charted body with the hand welded in — and never off `Hand::build` or
//! `parts.extremities`: the foot's audit read `cage + catmull_clark` for a
//! milestone and never saw a single carve (#306), and a hand instrument that
//! read the part before the weld would be blind to the one junction this
//! milestone is about. Ownership is by skin weight, which is what the
//! renderer bends, and the hand's frame is the rig's own: the wrist crease
//! to the knuckle row.

use std::collections::BTreeMap;

use symbios_avatar::{
    Archetype, Avatar, AvatarConfig, AvatarRecord, Limb, MeshKind, Role, Vec3, Zone,
};

/// The stations a hand is judged at: the default body and the frame axis's
/// ends, which move the hand through the stature and the arm's girth.
const STATIONS: [f32; 3] = [0.0, -1.5, 1.0];

/// One hand, read off the drawn skin in its own frame.
struct Hand {
    /// The largest radial step across the wrist crease, metres: down a ladder
    /// of stations along the arm's axis, per direction around it, the change
    /// of the silhouette's radius between neighbouring stations.
    wrist_step: f32,
    /// How many pieces the hand's surface is in, vertices welded by position.
    pieces: usize,
    /// Crease to fingertip along the hand's axis, metres.
    length: f32,
    /// The widest extent across the palm in the knuckle band, metres.
    width: f32,
    /// The middle knuckle's height over the little finger's, along the back
    /// of the hand, metres.
    arch: f32,
    /// How much deeper the palm is on the thumb's half than on the little
    /// finger's, through the band the thumb leaves from: a ratio.
    thenar: f32,
    /// The hand's vertices in body space, for the reflection test.
    points: Vec<Vec3>,
    /// How many faces within the wrist's band are wound against the skin's
    /// own normals — turned inside out.
    folded: usize,
    /// The same seam kink, read with the hand raised in the crate's own
    /// greeting — the wrist turned and lifted, which is where a binding
    /// that drops on one ring shows as a shelf.
    posed_step: f32,
}

impl Hand {
    fn measure(avatar: &Avatar, limb: Limb) -> Option<Self> {
        let rig = &avatar.rig;
        let skin = avatar
            .drawn(0.0)
            .into_iter()
            .find(|drawn| drawn.kind == MeshKind::Skin)?
            .mesh;

        // The hand's bones: every digit joint whose chain climbs into this
        // limb, and the wrist they all hang from.
        let in_limb = |mut joint: usize| loop {
            if rig.joints[joint].zone == Zone::Extremity(limb) {
                return true;
            }
            match rig.joints[joint].parent {
                Some(parent) => joint = parent,
                None => return false,
            }
        };
        let digits: Vec<usize> = (0..rig.joints.len())
            .filter(|&joint| rig.joints[joint].role == Role::Digit && in_limb(joint))
            .collect();
        let knuckles: Vec<usize> = digits
            .iter()
            .copied()
            .filter(|&joint| {
                rig.joints[joint]
                    .parent
                    .is_some_and(|parent| rig.joints[parent].role != Role::Digit)
            })
            .collect();
        if knuckles.len() != 5 {
            return None;
        }
        let wrist = rig.joints[knuckles[0]].parent?;
        let forearm = rig.joints[wrist].parent?;
        let crease = rig.joints[forearm].position;
        let arm_axis = (rig.joints[wrist].position - crease).try_normalize()?;
        let mut bones: Vec<usize> = digits.clone();
        bones.push(wrist);

        // The frame. Knuckles are attached index, middle, ring, little, thumb;
        // `out` runs from the crease to the middle of the four finger
        // knuckles, `across` from the index's to the little finger's, and
        // `up` is the back of the hand — the side the fingers curl away
        // from, read off the middle fingertip rather than assumed.
        let at = |joint: usize| rig.joints[joint].position;
        let row = (0..4).map(|i| at(knuckles[i])).sum::<Vec3>() / 4.0;
        let out = (row - crease).try_normalize()?;
        let span = at(knuckles[3]) - at(knuckles[0]);
        let across = (span - out * span.dot(out)).try_normalize()?;
        let mut up = out.cross(across);
        let mut tip = knuckles[1];
        while let Some(&child) = digits.iter().find(|&&d| rig.joints[d].parent == Some(tip)) {
            tip = child;
        }
        if (at(tip) - row).dot(up) > 0.0 {
            up = -up;
        }

        // Ownership by skin weight: a vertex is the hand's if the bone that
        // holds it most is one of the hand's own.
        let owned: Vec<usize> = (0..skin.vertex_count())
            .filter(|&vertex| {
                skin.skin.get(vertex).is_some_and(|shares| {
                    shares
                        .iter()
                        .max_by(|a, b| a.weight.total_cmp(&b.weight))
                        .is_some_and(|top| bones.contains(&(top.joint as usize)))
                })
            })
            .collect();
        if owned.len() < 64 {
            return None;
        }
        let points: Vec<Vec3> = owned.iter().map(|&v| skin.positions[v]).collect();
        // The thumb's own bones, so the palm's width can be read without it:
        // the thumb points ACROSS, and a width that counts it is the thumb's
        // reach, not the palm's.
        let mut thumb_bones = vec![knuckles[4]];
        while let Some(&child) = digits
            .iter()
            .find(|&&d| rig.joints[d].parent == thumb_bones.last().copied())
        {
            thumb_bones.push(child);
        }
        let palm: Vec<Vec3> = owned
            .iter()
            .filter(|&&v| {
                let top = skin.skin[v]
                    .iter()
                    .max_by(|a, b| a.weight.total_cmp(&b.weight))
                    .map(|top| top.joint as usize);
                top.is_some_and(|top| !thumb_bones.contains(&top))
            })
            .map(|&v| skin.positions[v])
            .collect();

        // 1. The wrist step, on the WHOLE skin — the arm's side of the crease
        // is not the hand's to own. A ladder of stations along the arm's axis
        // across the crease; at each, twelve rays from the axis bisected
        // against the surface; the step is the largest change of radius
        // between neighbouring stations in one direction. Bisected, not
        // counted: a 4 mm slab of this mesh holds a vertex or two, and a
        // reading off it sat at 0.0 on five hands of six, which is a reading
        // that cannot fail.
        let wrist_step = seam_kink(&skin, crease, arm_axis, across, up);

        // 1a. Folded faces: every skin face whose centroid lies within the
        // wrist's band, its winding's area vector against the mean of its
        // corners' stored normals. A welded surface has none; a band turned
        // inside out has a ring of them (#318).
        let folded = skin
            .faces
            .iter()
            .filter(|face| {
                let centre = face
                    .iter()
                    .fold(Vec3::ZERO, |sum, &v| sum + skin.positions[v as usize])
                    / face.len() as f32;
                (centre - crease).dot(arm_axis).abs() < 0.03 && centre.distance(crease) < 0.06
            })
            .filter(|face| {
                let mut area = Vec3::ZERO;
                let mut stored = Vec3::ZERO;
                for (index, &v) in face.iter().enumerate() {
                    let here = skin.positions[v as usize];
                    let next = skin.positions[face[(index + 1) % face.len()] as usize];
                    area += here.cross(next);
                    stored += skin.normals[v as usize];
                }
                // A face that draws nothing has no side: once both rims lie
                // on one curve the weld's bridge is slivers of a few
                // hundredths of a square millimetre — a thirtieth of a pixel
                // at the close-up's 0.86 mm per pixel — and one lying
                // edge-on to the skin. A tenth of a square millimetre, turned
                // clearly against, is what a folded face is.
                let size = area.length() * 0.5;
                size > 1e-7
                    && area
                        .normalize_or(Vec3::ZERO)
                        .dot(stored.normalize_or(Vec3::ZERO))
                        < -0.2
            })
            .count();

        // 1b. And posed (#318): the crate's own greeting at its peak, the
        // hand raised and the wrist turned. A binding that drops from the
        // forearm to the hand on one ring meets at rest and shears under a
        // turn — the hand sat on the forearm with a shelf at the crease in
        // every raised frame of a wave, and nothing at rest could see it.
        let posed_step = {
            let mut pose = symbios_avatar::anim::Pose::rest(rig);
            symbios_avatar::anim::gesture::wave(limb).apply(rig, &mut pose, 0.4);
            let posed = pose.forward(rig);
            let skin = posed.deform_mesh(rig, &skin);
            let crease = posed.positions[forearm];
            let axis = (posed.positions[wrist] - crease).try_normalize()?;
            let across = axis.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
            let up = axis.cross(across);
            seam_kink(&skin, crease, axis, across, up)
        };

        // 2. Pieces: faces wholly owned, vertices welded by position, counted
        // by union-find. Charts split the skin by index, so an index walk
        // finds as many pieces as there are charts; positions do not lie.
        let mut keyed: BTreeMap<(i64, i64, i64), usize> = BTreeMap::new();
        let mut key_of = vec![usize::MAX; skin.vertex_count()];
        for &v in &owned {
            let p = skin.positions[v] * 1e5;
            let key = (p.x.round() as i64, p.y.round() as i64, p.z.round() as i64);
            let next = keyed.len();
            key_of[v] = *keyed.entry(key).or_insert(next);
        }
        let mut parent: Vec<usize> = (0..keyed.len()).collect();
        fn find(parent: &mut [usize], mut a: usize) -> usize {
            while parent[a] != a {
                parent[a] = parent[parent[a]];
                a = parent[a];
            }
            a
        }
        let mut any = false;
        for face in &skin.faces {
            if !face.iter().all(|&v| key_of[v as usize] != usize::MAX) {
                continue;
            }
            any = true;
            let first = key_of[face[0] as usize];
            for &v in &face[1..] {
                let (a, b) = (
                    find(&mut parent, first),
                    find(&mut parent, key_of[v as usize]),
                );
                parent[a] = b;
            }
        }
        if !any {
            return None;
        }
        let mut roots: Vec<usize> = (0..keyed.len()).map(|i| find(&mut parent, i)).collect();
        roots.sort_unstable();
        roots.dedup();
        let pieces = roots.len();

        // 3. Proportions, in the hand's frame from the crease.
        let along = |p: Vec3| (p - crease).dot(out);
        let length = points.iter().map(|&p| along(p)).fold(0.0f32, f32::max);
        let band = |lo: f32, hi: f32| {
            palm.iter()
                .copied()
                .filter(|&p| {
                    let t = along(p) / length;
                    t >= lo && t < hi
                })
                .collect::<Vec<_>>()
        };
        let knuckle_band = band(0.40, 0.55);
        let (left, right) = knuckle_band
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &p| {
                let s = (p - crease).dot(across);
                (lo.min(s), hi.max(s))
            });
        let width = right - left;

        // 4. The arch: the middle knuckle over the little finger's.
        let arch = (at(knuckles[1]) - at(knuckles[3])).dot(up);

        // 5. The thenar: palm-side depth on the thumb's half of the palm
        // against the little finger's, through the band the thumb leaves
        // from. The thumb itself is excluded by reading only within the
        // palm's own width.
        let half = width * 0.5;
        let middle = (left + right) * 0.5;
        let thumb_side = if (at(knuckles[4]) - crease).dot(across) < middle {
            -1.0
        } else {
            1.0
        };
        // Over ALL the hand's vertices, not `palm`: the thumb's bones hold
        // the palm beside their own wall, which is exactly the mound; the
        // thumb itself is kept out by the window across.
        let depth = |side: f32| {
            points
                .iter()
                .filter(|&&p| {
                    let t = along(p) / length;
                    (0.28..0.42).contains(&t)
                })
                .filter(|&&p| {
                    let s = (p - crease).dot(across) - middle;
                    s * side > half * 0.3 && s.abs() < half * 0.85
                })
                .map(|&p| -(p - crease).dot(up))
                .fold(0.0f32, f32::max)
        };
        let thenar = depth(thumb_side) / depth(-thumb_side).max(1e-6);

        Some(Self {
            wrist_step,
            folded,
            posed_step,
            pieces,
            length,
            width,
            arch,
            thenar,
            points,
        })
    }
}

/// Builds each station and reads both hands.
fn hands() -> Vec<(f32, Limb, Hand, f32)> {
    let mut out = Vec::new();
    for femininity in STATIONS {
        let mut record = AvatarRecord::new("Handed", Archetype::default());
        record.composites.femininity = femininity;
        record.composites.sanitize();
        record.sanitize();
        // Undressed at build time: a dressed body does not emit the skin its
        // clothes cover, and the open hems that leaves break the parity ray
        // `PolyMesh::contains` casts along +x — from the right wrist that ray
        // crosses the whole torso, and the right hand's wrist ladder read
        // 39 mm of step on a junction the left hand read at 0.4 mm.
        let config = AvatarConfig {
            dressed: false,
            ..Default::default()
        };
        let avatar = Avatar::build_with(&record, &config).expect("a biped builds");
        let (low, high) = avatar.parts.body.bounds();
        let stature = high.y - low.y;
        for limb in [Limb::ForeLeft, Limb::ForeRight] {
            let hand = Hand::measure(&avatar, limb).expect("a hand measures");
            out.push((femininity, limb, hand, stature));
        }
    }
    out
}

/// Reflects one hand onto the other: for every vertex of the left, how far
/// the nearest vertex of the right is, mirrored across the sagittal plane.
/// The seam kink: a ladder of stations along `axis` across `crease`, twelve
/// rays from the axis at each, the largest second difference of the
/// silhouette's radius on the rungs straddling the seam.
///
/// The radius is the OUTERMOST hit of a ray from the axis against the skin's
/// triangles — the silhouette, which is what an eye reads of a collar and what
/// a nested surface cannot hide under. Not `PolyMesh::contains`: that bisects
/// on a parity ray cast along +x, and from the medial side of a wrist that ray
/// crosses the wrist itself, weld slivers included; one graze miscounted, and
/// the instrument read a 4 mm bump that no vertex stood anywhere near.
///
/// **The ladder's second difference, not its first** (#318). A step is a
/// KINK: the radius changes slope on one rung. A first difference cannot tell
/// that from a slope — the heel of the hand broadens toward the thenar at
/// 2.3 mm a rung on the thumb's side, which is the hand's own shape and read
/// as 2.26 mm of "step" the moment the weld stopped replacing the heel with
/// 23 mm of forearm stub. **And read ON THE SEAM**: the hand's weld ring sits
/// a tenth of its base behind the crease (about 2.5 mm), so the triples whose
/// middle rung lies between 5 mm behind the crease and the crease itself
/// straddle it. The heel, five millimetres on, curves at about a 20 mm radius
/// — 0.3 mm of second difference on these rungs — and is not a seam.
fn seam_kink(
    skin: &symbios_avatar::PolyMesh,
    crease: Vec3,
    axis: Vec3,
    across: Vec3,
    up: Vec3,
) -> f32 {
    let radius = |from: Vec3, dir: Vec3| -> Option<f32> {
        let mut hit: Option<f32> = None;
        for face in &skin.faces {
            let a = skin.positions[face[0] as usize];
            for corner in 1..face.len() - 1 {
                let b = skin.positions[face[corner] as usize];
                let c = skin.positions[face[corner + 1] as usize];
                if let Some(t) = ray_triangle(from, dir, a, b, c).filter(|&t| t < 0.12) {
                    hit = Some(hit.map_or(t, |h| h.max(t)));
                }
            }
        }
        hit
    };
    const RUNGS: [f32; 7] = [-0.0075, -0.005, -0.0025, 0.0, 0.0025, 0.005, 0.0075];
    let mut kink = 0.0f32;
    for bin in 0..12 {
        let angle = std::f32::consts::TAU * bin as f32 / 12.0;
        let dir = across * angle.cos() + up * angle.sin();
        // Each rung is the median of three rays a millimetre apart: the
        // weld's bridge triangles lie almost in the surface, and one ray
        // grazing them read a 3.5 mm spike on one rung of one bin with its
        // neighbours smooth either side.
        let ladder: Vec<Option<f32>> = RUNGS
            .iter()
            .map(|&rung| {
                let mut reads: Vec<f32> = [-0.001, 0.0, 0.001]
                    .iter()
                    .filter_map(|&jitter| radius(crease + axis * (rung + jitter), dir))
                    .collect();
                reads.sort_by(f32::total_cmp);
                (reads.len() == 3).then(|| reads[1])
            })
            .collect();
        for (middle, triple) in ladder.windows(3).enumerate() {
            let at = RUNGS[middle + 1];
            if !(-0.005..=0.0).contains(&at) {
                continue;
            }
            if let (Some(a), Some(b), Some(c)) = (triple[0], triple[1], triple[2]) {
                kink = kink.max(((c - b) - (b - a)).abs());
            }
        }
    }
    kink
}

/// `(mean, worst)` in metres.
fn reflection(left: &Hand, right: &Hand) -> (f32, f32) {
    let mut total = 0.0f32;
    let mut worst = 0.0f32;
    for &point in &left.points {
        let mirrored = Vec3::new(-point.x, point.y, point.z);
        let nearest = right
            .points
            .iter()
            .map(|&other| other.distance(mirrored))
            .fold(f32::MAX, f32::min);
        total += nearest;
        worst = worst.max(nearest);
    }
    (total / left.points.len() as f32, worst)
}

#[test]
fn the_wrist_has_no_step() {
    // #297. The first hand's palm base collared the arm: its round base ring
    // sat OVER the forearm and the wrist crease was two nested surfaces. Read
    // as the silhouette — the outermost hit, so the nesting cannot hide it —
    // the pre-hand tree steps 1.9 mm in five millimetres across the crease
    // on every body; the welded tree reads 0.3 to 0.5 mm, which is the
    // forearm's own taper over the same five.
    for (femininity, limb, hand, _) in hands() {
        assert!(
            hand.wrist_step < 0.0015,
            "femininity {femininity:+.1} {limb:?}: the skin steps {:.2} mm across the wrist \
             crease; the welded wrist reads 0.5 mm and the collar read 1.9 mm",
            hand.wrist_step * 1000.0
        );
    }
}

#[test]
fn no_face_at_the_wrist_is_turned_inside_out() {
    // **The fold** (#318). The cut's rim is the forearm's last ring, 23 mm
    // past the crease; the hand's weld ring sits 2 mm behind it and its next
    // ring 4 mm in front. Snapping the hand's ring out to the rim dragged it
    // past its own neighbour and turned the hand's base band inside out —
    // twenty faces wound against the rest of the hand, a notch in the
    // silhouette and a shading field that broke on the seam however the
    // normals were summed. The arm's rim comes to the hand now. Read as the
    // faces in the wrist's band whose winding turns against their own
    // normals: twenty of the hand's own faces before the weld's compaction,
    // six that still draw on the skin as shipped, and none on a weld.
    for (femininity, limb, hand, _) in hands() {
        assert_eq!(
            hand.folded, 0,
            "femininity {femininity:+.1} {limb:?}: {} faces at the wrist are turned inside out",
            hand.folded
        );
    }
}

#[test]
fn the_wrist_has_no_step_when_the_hand_waves() {
    // **A junction is a binding as much as a surface** (#318). The hand is
    // bound to its own twenty-one bones and to no other, so the arm's rim
    // held 0.86 of the forearm and the hand's ring, on the same curve, none
    // of it: positions met and the weights dropped on one ring. At rest that
    // is invisible; raised in a wave the wrist sheared and the hand sat on
    // the forearm with a shelf at the crease. The weld blends the forearm's
    // hold across the wrist now. Read as the seam kink with the crate's own
    // greeting applied at its peak.
    for (femininity, limb, hand, _) in hands() {
        // Before: 20.9 mm on the default left hand — a shelf; now 1.5 to
        // 2.2 mm across the six hands, the blended band bending.
        assert!(
            hand.posed_step < 0.003,
            "femininity {femininity:+.1} {limb:?}: with the hand raised in a wave the skin              kinks {:.2} mm at the wrist",
            hand.posed_step * 1000.0
        );
    }
}

#[test]
fn a_hand_is_one_surface_from_forearm_to_fingertip() {
    // #298. Six closed solids appended to the arm's stub pass every manifold
    // check — each is closed — and render every boundary between them: the
    // pre-hand tree's hand is SIX pieces (the palm and five digits; the arm's
    // stub under them is the arm's own) by position-welded edge walk. The
    // extruded, welded hand is one, the arm included.
    for (femininity, limb, hand, _) in hands() {
        assert_eq!(
            hand.pieces, 1,
            "femininity {femininity:+.1} {limb:?}: the hand's surface is in {} pieces; the \
             appended hand was six",
            hand.pieces
        );
    }
}

#[test]
fn the_two_hands_are_reflections_to_the_float() {
    // #296's ratchet. `Hand::build` once read 5.5 mm mean / 33.4 mm worst
    // between the two hands because the second was the first turned half
    // round (#113); the reference pair reflects to 0.000. On the drawn skin,
    // weld and chart splits included, the agreed tree reads 0.03 to 0.04 mm
    // mean and 1.6 to 2.0 mm worst, and the worst vertex sits ON the wrist
    // ring — the weld's position-based rim matching and fairing do not land
    // identically on the two sides. **This is the one guard the pre-hand
    // tree passes**: its appended hands reflect to 0.000 / 0.000, so the
    // failing tree for this bound is #113's, not #297's parent. The bound is
    // the ratchet against 33.4 mm with the weld's 2 mm inside it; closing
    // that 2 mm is the weld's business, noted on #307.
    let all = hands();
    for femininity in STATIONS {
        let side = |limb: Limb| {
            all.iter()
                .find(|(f, l, _, _)| *f == femininity && *l == limb)
                .map(|(_, _, hand, _)| hand)
                .expect("both hands measured")
        };
        let (mean, worst) = reflection(side(Limb::ForeLeft), side(Limb::ForeRight));
        assert!(
            mean < 0.0001 && worst < 0.003,
            "femininity {femininity:+.1}: reflecting the left hand onto the right leaves \
             {:.3} mm mean and {:.3} mm worst; the welded pair read 0.04 / 2.0 and the \
             turned pair 5.5 / 33.4",
            mean * 1000.0,
            worst * 1000.0
        );
    }
}

#[test]
fn a_hand_has_the_proportions_of_a_hand() {
    // #297/#298. Hand length is a share of STATURE (10.5% authored, the
    // reference share), not of the wrist's girth: sized off the girth, the
    // pre-hand tree's hand reaches 8.9 to 9.3% of stature from the crease
    // depending on the arm it hangs from. Read along the hand's axis to the
    // furthest vertex, the curled fingers of the agreed tree reach 9.86 to
    // 9.92% on every station. The palm's width across the knuckle band is
    // 0.514 of that reach (authored 0.96 of a palm length over 1.95 of them,
    // 0.49, plus the thenar); the pre-hand tree reads 0.534, INSIDE this
    // bound — the width guards the shape against drift, and it is the length
    // that tells the two trees apart.
    for (femininity, limb, hand, stature) in hands() {
        let long = hand.length / stature;
        assert!(
            (0.095..0.105).contains(&long),
            "femininity {femininity:+.1} {limb:?}: the hand reaches {:.4} of stature; the \
             agreed tree reads 0.099 and the girth-sized hand 0.089 to 0.093",
            long
        );
        let wide = hand.width / hand.length;
        assert!(
            (0.46..0.56).contains(&wide),
            "femininity {femininity:+.1} {limb:?}: the palm is {:.3} as wide as the hand is \
             long; the agreed tree reads 0.514",
            wide
        );
    }
}

#[test]
fn the_knuckle_row_is_arched_and_the_palm_has_a_thenar_mound() {
    // #299. The metacarpal heads sit on a dome whose crest is the middle
    // finger's: `KNUCKLE_ARCH` lifts the middle knuckle 0.36 of the distal
    // station's half-depth over the little finger's, which the rig's own
    // knuckle joints read as 3.1 mm on this stature; the pre-hand tree's
    // knuckles lie in one plane and read 0.0. And the palm under the thumb
    // carries the thenar mound: through the band the thumb leaves from, the
    // thumb's half of the palm is 1.46 to 1.49 times as deep palmward as the
    // little finger's; a palm with no mound reads 1.000 by symmetry, which is
    // exactly what the pre-hand tree reads.
    for (femininity, limb, hand, _) in hands() {
        assert!(
            hand.arch > 0.0015,
            "femininity {femininity:+.1} {limb:?}: the middle knuckle stands {:.2} mm over the \
             little finger's; the arched row reads 3.1 mm and a flat row 0.0",
            hand.arch * 1000.0
        );
        assert!(
            hand.thenar > 1.25,
            "femininity {femininity:+.1} {limb:?}: the thumb's half of the palm is {:.3} as \
             deep as the little finger's; the mound reads 1.46 and a flat palm 1.00",
            hand.thenar
        );
    }
}

/// Möller–Trumbore: distance along `dir` from `from` to triangle `abc`, if hit.
fn ray_triangle(from: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (ab, ac) = (b - a, c - a);
    let p = dir.cross(ac);
    let det = ab.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = from - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(ab);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = ac.dot(q) * inv;
    (t > 0.0).then_some(t)
}
