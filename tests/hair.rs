//! The hair of the head: where its cards are rooted, how they sit on the
//! crown, where they end, where they do not hang, and how they are lit.
//!
//! **Guards fitted AFTER the geometry was agreed by render** — milestone #10's
//! standing method (#316). The starting sheet showed six defects on the
//! default body: a star of bare scalp at the crown on every style, a sawtooth
//! fringe, long hair hanging straight through the face, a bald side on the
//! tied-back and curly styles, cards shading as flat ribbons, and the crop's
//! hairline as a hard cap edge. Each bound here was fitted to the tree that
//! fixed them and checked against the tree before it, which fails every one.
//!
//! **Everything is read off the BUILT hair** — the mesh `Avatar::build` hands
//! a renderer, split back into its cards — and never off a shape asked in
//! isolation, because the instrument that reads a shape has been wrong about
//! what the render showed before (#210, #313). A card here is the run of
//! quads the loft emits for one clump, two vertices a station, in order.
//!
//! **One guard here is a construction rather than a look**: which lane of the
//! strand mask a card is cut from (#340). That is arithmetic, never something
//! to agree by eye, and it is read off the built mesh all the same.
use std::ops::Range;

use symbios_avatar::face::{Canon, Skull};
use symbios_avatar::hair::mask::{LANES, StrandMask};
use symbios_avatar::hair::{
    BrowStyle, ChinStyle, FlankStyle, Follicles, MoustacheStyle, ScalpStyle,
};
use symbios_avatar::{Archetype, Avatar, AvatarRecord, MeshKind, Vec3};

/// The default body wearing one scalp style and nothing else on its head.
struct Head {
    /// The hair, head-local, as the renderer gets it.
    hair: symbios_avatar::hair::Growth,
    /// The body it grew on, for the surface under the cards.
    body: symbios_avatar::PolyMesh,
    /// Where the head's own space sits in the body's.
    origin: Vec3,
    /// The measured skull.
    skull: Skull,
    /// The space in front of the face no scalp hair may hang in.
    clearance: symbios_avatar::hair::follicle::scalp::Clearance,
}

impl Head {
    fn wearing(style: ScalpStyle) -> Self {
        let mut record = AvatarRecord::new("Hair", Archetype::default());
        record.hair.scalp.style = style;
        Self::of(record).expect("a scalp style grows hair")
    }

    /// A record's own body and scalp, every other region stripped, or `None`
    /// if its scalp grows nothing.
    fn of(mut record: AvatarRecord) -> Option<Self> {
        record.hair.brows.style = BrowStyle::None;
        record.hair.moustache.style = MoustacheStyle::None;
        record.hair.chin.style = ChinStyle::None;
        record.hair.flanks.style = FlankStyle::None;
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        let hair = avatar.parts.hair.clone()?;
        assert!(
            hair.grown
                .iter()
                .all(|grown| grown.follicle == symbios_avatar::hair::Follicle::Scalp),
            "only the scalp is dressed here"
        );
        Some(Self {
            hair,
            body: avatar.parts.body.clone(),
            origin: follicles.origin(),
            skull,
            clearance: follicles.clearance(),
        })
    }

    /// The crown and the throat, head-local.
    fn crown_and_throat(&self) -> (f32, f32) {
        let (throat, crown) = self.skull.throat_and_crown();
        (crown, throat)
    }

    /// Each card's run of vertices, in the order the loft emitted them.
    fn cards(&self) -> Vec<Range<usize>> {
        cards_of(&self.hair.mesh.faces)
    }

    /// Station `index` of a card: the midpoint of its two vertices.
    fn station(&self, card: &Range<usize>, index: usize) -> Vec3 {
        let at = card.start + index * 2;
        (self.hair.mesh.positions[at] + self.hair.mesh.positions[at + 1]) * 0.5
    }

    /// How many stations a card has.
    fn stations(card: &Range<usize>) -> usize {
        card.len() / 2
    }

    /// A card's azimuth, read at its second station — the first step out of
    /// the pole, which every card takes down its own meridian.
    fn azimuth(&self, card: &Range<usize>) -> f32 {
        let at = self.station(card, 1);
        at.x.atan2(at.z)
    }

    /// The signed height of a head-local point over the body's surface, in
    /// metres: the distance to the nearest point of any face within reach,
    /// signed by that face's normal.
    fn over_skin(&self, point: Vec3) -> f32 {
        let mut best = (f32::MAX, 0.0f32);
        for face in 0..self.body.faces.len() {
            let corners = &self.body.faces[face];
            if corners.len() < 3 {
                continue;
            }
            let first = self.body.positions[corners[0] as usize] - self.origin;
            if first.distance_squared(point) > 0.06 * 0.06 {
                continue;
            }
            for fan in 1..corners.len() - 1 {
                let b = self.body.positions[corners[fan] as usize] - self.origin;
                let c = self.body.positions[corners[fan + 1] as usize] - self.origin;
                let (nearest, normal) = closest_on_triangle(point, first, b, c);
                let apart = nearest.distance_squared(point);
                if apart < best.0 {
                    best = (apart, (point - nearest).dot(normal).signum() * apart.sqrt());
                }
            }
        }
        best.1
    }

    /// The distance from a head-local point to the nearest card, in metres.
    fn to_hair(&self, point: Vec3) -> f32 {
        let mesh = &self.hair.mesh;
        let mut best = f32::MAX;
        for face in &mesh.faces {
            let a = mesh.positions[face[0] as usize];
            if a.distance_squared(point) > 0.08 * 0.08 {
                continue;
            }
            for fan in 1..face.len() - 1 {
                let b = mesh.positions[face[fan] as usize];
                let c = mesh.positions[face[fan + 1] as usize];
                let (nearest, _) = closest_on_triangle(point, a, b, c);
                best = best.min(nearest.distance(point));
            }
        }
        best
    }
}

/// Each card's run of vertices in a mesh of cards, in the order the loft emitted
/// them.
///
/// A card is quads `[s, s+1, s+3, s+2]` with `s` stepping by two; a new card
/// begins wherever the step does not.
fn cards_of(faces: &[Vec<u32>]) -> Vec<Range<usize>> {
    let mut cards = Vec::new();
    let mut start: Option<u32> = None;
    let mut last = 0u32;
    for face in faces {
        let first = face[0];
        match start {
            None => start = Some(first),
            Some(_) if first == last + 2 => {}
            Some(begun) => {
                cards.push(begun as usize..(last + 4) as usize);
                start = Some(first);
            }
        }
        last = first;
    }
    if let Some(begun) = start {
        cards.push(begun as usize..(last + 4) as usize);
    }
    cards
}

/// The closest point of a triangle to `p`, and the triangle's normal.
fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> (Vec3, Vec3) {
    let n = (b - a).cross(c - a).normalize_or(Vec3::Y);
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, n);
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, n);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (a + ab * (d1 / (d1 - d3)), n);
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, n);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (a + ac * (d2 / (d2 - d6)), n);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, n);
    }
    let denominator = 1.0 / (va + vb + vc);
    let v = vb * denominator;
    let w = vc * denominator;
    (a + ab * v + ac * w, n)
}

/// The gap, in radians, between the two closest-spaced of these azimuths' far
/// neighbours: the largest empty sector round the head.
fn widest_gap(mut azimuths: Vec<f32>) -> f32 {
    azimuths.sort_by(f32::total_cmp);
    let mut widest = 0.0f32;
    for pair in azimuths.windows(2) {
        widest = widest.max(pair[1] - pair[0]);
    }
    if let (Some(first), Some(last)) = (azimuths.first(), azimuths.last()) {
        widest = widest.max(first + std::f32::consts::TAU - last);
    }
    widest
}

#[test]
fn scalp_cards_are_spaced_round_the_head() {
    // **A scalp root is a meridian, and a meridian scattered by area lands
    // wherever the faces do** (#316). A tied-back head of twenty-eight cards
    // had gaps of 46° and 39° on one side, which the sheet showed as a bald
    // side; a curly head of twenty-five had the same. Cards are seated one
    // to a sector now, with a jitter inside it, so the widest empty sector
    // on any style is bounded by the sector's own width.
    //
    // Read at each card's second station, which is the first step out of
    // the pole and is still on the card's own meridian. The bound is three
    // sectors: the jitter either side of the mean spacing, and a sector
    // widened once where no face centre fell inside it, which a crop's
    // five-degree sectors do often. Before: 3.6 sectors on the tied-back.
    for style in [
        ScalpStyle::TiedBack { tail: 0.8 },
        ScalpStyle::Curly { curl: 0.8 },
        ScalpStyle::Crop,
    ] {
        let head = Head::wearing(style);
        let cards = head.cards();
        let azimuths: Vec<f32> = cards.iter().map(|card| head.azimuth(card)).collect();
        let sector = std::f32::consts::TAU / cards.len() as f32;
        let gap = widest_gap(azimuths);
        assert!(
            gap <= sector * 3.0,
            "{style:?}: the widest bare sector round the head is {:.0}° against a mean card \
             spacing of {:.0}°, {} cards",
            gap.to_degrees(),
            sector.to_degrees(),
            cards.len()
        );
    }
}

#[test]
fn the_crown_is_covered() {
    // **The star of bare scalp at the whorl** (#316), which #65 closed once
    // and which came back: about seven random azimuth gaps over eleven
    // degrees, which an eased fan ramp could not close until thirty
    // millimetres out. Read as coverage: points on the skull's own envelope
    // fifteen to thirty millimetres from the pole, seventy-two azimuths a
    // ring, each within a card's thickness of some card. Before: 10–20% of
    // them bare on every style.
    for style in [
        ScalpStyle::Crop,
        ScalpStyle::Bob { fringe: 0.8 },
        ScalpStyle::TiedBack { tail: 0.8 },
    ] {
        let head = Head::wearing(style);
        let (crown, _) = head.crown_and_throat();
        let mut bare = 0usize;
        let mut probed = 0usize;
        for ring in [0.015f32, 0.020, 0.025, 0.030] {
            for turn in 0..72 {
                let azimuth = std::f32::consts::TAU * turn as f32 / 72.0;
                // Down the envelope to the height whose radius is `ring`.
                let height = (0..=40)
                    .map(|step| crown - 0.030 * step as f32 / 40.0)
                    .find(|height| {
                        let at = head.skull.surface_at(*height, azimuth);
                        (at.x * at.x + at.z * at.z).sqrt() >= ring
                    })
                    .unwrap_or(crown - 0.030);
                let at = head.skull.surface_at(height, azimuth);
                probed += 1;
                if head.to_hair(at) > 0.003 {
                    bare += 1;
                }
            }
        }
        let share = bare as f32 / probed as f32;
        assert!(
            share <= 0.02,
            "{style:?}: {:.0}% of the crown's envelope has no card within 3 mm of it",
            share * 100.0
        );
    }
}

#[test]
fn no_card_dips_under_the_crown() {
    // **A card is lifted off the surface it is lying on, not off its root**
    // (#316). A card rooted at the nape carried its clearance sideways across
    // the whorl, and its chords cut a millimetre into the back of the dome:
    // slivers of scalp through the cards behind the crown on the sheet.
    // Measured before: 15% of the stations in the first third of a crop's
    // cards under the body, worst 1.1 mm. The bound is a quarter of a
    // millimetre, which is under what the render resolves.
    let head = Head::wearing(ScalpStyle::Crop);
    let (crown, _) = head.crown_and_throat();
    let mut worst = 0.0f32;
    let mut under = 0usize;
    let mut probed = 0usize;
    for card in head.cards() {
        for index in 0..Head::stations(&card) {
            let at = head.station(&card, index);
            if at.y < crown - 0.040 {
                break;
            }
            let over = head.over_skin(at);
            probed += 1;
            if over < -0.00025 {
                under += 1;
            }
            worst = worst.min(over);
        }
    }
    assert!(probed > 100, "the crown was not probed");
    assert!(
        under == 0,
        "{under} of {probed} card stations over the crown sit under the skin, worst {:.2} mm",
        worst * 1000.0
    );
}

#[test]
fn long_hair_does_not_hang_over_the_face() {
    // **Straps fell flat and straight through the eyes and the mouth to the
    // chest** (#316). Long hair is parted now: the front locks are combed to
    // the temple as they descend and hang beside the face. Read as the hair
    // in a box in front of the face — from the brow to the chin, within
    // three centimetres of the midline, forward of the head's centre —
    // sampled along every card. The curly style had the same fault for the opposite
    // reason, a fringe share that let its ringlets curtain the eyes.
    // Before: 0.8% of the long hair's length and 2.1% of the curly's in the
    // box; now none and 0.4%.
    //
    // **And then the render disagreed with it, and the render was right**
    // (#341). That box was three centimetres either side of the midline and a
    // share of the hair was allowed in it, so it passed at Long 0.9 while a
    // bob cut long hung its fringe to the nose - rolled seed 42 - and a bob at
    // fringe 0 was a curtain over the eyes by design. Measured on the tree
    // before #341 against the box below: 109 of 300 rolled records put hair in
    // front of the face, every one a bob or a curl; seed 42, 22 stations; a
    // bob at fringe 0 and length 0.35, 22; a curl at 0.8 and 0.35, 5.
    //
    // Now the box is the engine's own landmark, [`Follicles::clearance`] - brow
    // to chin, forward of the temple plane and between the temples - and the
    // walk stops a lock before it enters, so the bound is ZERO stations. The
    // corners are the ones #341 names, seed 42 is the control, and a sweep of
    // rolled records covers what nobody named.
    let mut heads: Vec<(String, Head)> = Vec::new();
    for (style, length) in [
        (ScalpStyle::Long { weight: 1.0 }, 1.0),
        (ScalpStyle::Long { weight: 0.9 }, 0.35),
        (ScalpStyle::Bob { fringe: 0.0 }, 1.0),
        (ScalpStyle::Bob { fringe: 0.0 }, 0.35),
        (ScalpStyle::Bob { fringe: 0.8 }, 1.0),
        (ScalpStyle::Curly { curl: 1.0 }, 1.0),
        (ScalpStyle::Curly { curl: 0.8 }, 0.35),
    ] {
        let mut record = AvatarRecord::new("Hair", Archetype::default());
        record.hair.scalp.style = style;
        record.hair.scalp.cut.length = length;
        let head = Head::of(record).expect("a scalp style grows hair");
        heads.push((format!("{style:?} at length {length}"), head));
    }
    for seed in std::iter::once(42).chain(0..40) {
        let mut record = AvatarRecord::new("Rolled", Archetype::default());
        record.reroll(seed);
        if let Some(head) = Head::of(record) {
            heads.push((format!("rolled seed {seed}"), head));
        }
    }
    let mut inside = Vec::new();
    // **And that the box is where the hair goes** (#341): a box sitting
    // somewhere no lock reaches passes this at zero for nothing. The lowest a
    // station comes above the brow while in front of the face, over them all.
    let mut closest = f32::MAX;
    for (label, head) in &heads {
        let face = head.clearance;
        assert!(
            face.brow > face.chin && face.front > 0.0 && face.side > 0.0,
            "{label}: the clearance is not a box in front of a face: {face:?}"
        );
        let mut count = 0usize;
        let mut stations = 0usize;
        for card in head.cards() {
            for index in 0..Head::stations(&card) {
                let at = head.station(&card, index);
                stations += 1;
                if face.contains(at) {
                    count += 1;
                } else if at.y >= face.brow && at.z > face.front && at.x.abs() < face.side {
                    closest = closest.min(at.y - face.brow);
                }
            }
        }
        if count > 0 {
            inside.push(format!("{label}: {count} of {stations} stations"));
        }
    }
    println!(
        "closest station above the brow in front of the face: {:.1} mm",
        closest * 1000.0
    );
    assert!(
        inside.is_empty(),
        "scalp hair hangs in front of the face:\n{}",
        inside.join("\n")
    );
    // Measured at #341: 0.5 mm, the check's own margin - somewhere in this
    // set the walk's floor is what stopped a lock, which is the construction
    // being exercised rather than never reached. Two millimetres is the bound.
    assert!(
        closest <= 0.002,
        "no station comes within {:.1} mm of the brow in front of the face, so the clearance is \
         not being tested against any hair",
        closest * 1000.0
    );
}

#[test]
fn a_fringe_ends_on_no_one_line() {
    // **The sawtooth** (#316): cards that all hang the same distance past the
    // hairline end on one contour, and the contour of tapered cards is a row
    // of teeth. Each card's hang is staggered by its own salt now, and each
    // leaves the scalp at its own point across the hairline's fade. Read at
    // the tips of the front cards of a bob, which is where a fringe is
    // judged: the largest cluster of tips within a millimetre of one
    // another, as a share of them all. Before, five of nine front tips sat
    // at exactly 31 mm below the crown's height of the others' spread — a
    // line with a few thinned stragglers; now the largest cluster is four
    // of fifteen.
    let head = Head::wearing(ScalpStyle::Bob { fringe: 0.8 });
    let tips: Vec<f32> = head
        .cards()
        .iter()
        .filter(|card| head.azimuth(card).cos() > 0.75)
        .map(|card| head.station(card, Head::stations(card) - 1).y)
        .collect();
    assert!(tips.len() >= 6, "only {} cards over the brow", tips.len());
    let cluster = tips
        .iter()
        .map(|tip| {
            tips.iter()
                .filter(|other| (*other - tip).abs() <= 0.001)
                .count()
        })
        .max()
        .unwrap_or(0);
    let share = cluster as f32 / tips.len() as f32;
    assert!(
        share <= 0.34,
        "{cluster} of the bob's {} front cards end within a millimetre of one line",
        tips.len()
    );
}

#[test]
fn a_tail_gathers_the_back_and_leaves_the_front() {
    // **A tied-back head is drawn back, not drawn up** (#316). Every lock
    // used to be combed toward the knot from the crown and lerped to it over
    // its last half — a bare band above the brow where the front locks had
    // turned away, and a chord through the occiput for a high tail. Now the
    // back half feeds the tail and the front lies to the hairline. Read at
    // the tips: a front card's tip is in front of the head's centre, on the
    // forehead; a back card's tip is behind it, under the knot.
    let head = Head::wearing(ScalpStyle::TiedBack { tail: 0.8 });
    let mut front_tips_behind = 0usize;
    let mut front = 0usize;
    let mut back_tips_ahead = 0usize;
    let mut back = 0usize;
    let mut tail: Vec<Vec3> = Vec::new();
    for card in head.cards() {
        let facing = head.azimuth(&card).cos();
        let tip = head.station(&card, Head::stations(&card) - 1);
        if facing > 0.5 {
            front += 1;
            front_tips_behind += usize::from(tip.z < 0.0);
        } else if facing < -0.3 {
            back += 1;
            back_tips_ahead += usize::from(tip.z > 0.0);
            tail.push(tip);
        }
    }
    assert!(
        front >= 4 && back >= 6,
        "{front} front cards, {back} back cards"
    );
    assert!(
        front_tips_behind == 0,
        "{front_tips_behind} of {front} front cards end behind the head: they were gathered"
    );
    assert!(
        back_tips_ahead == 0,
        "{back_tips_ahead} of {back} back cards end in front of the head"
    );
    // And the gathered tips meet: within a few centimetres of one another.
    let middle = tail.iter().fold(Vec3::ZERO, |sum, at| sum + *at) / tail.len() as f32;
    let spread = tail
        .iter()
        .map(|at| at.distance(middle))
        .fold(0.0f32, f32::max);
    assert!(
        spread < 0.030,
        "the tail's {} tips spread {:.0} mm about their middle",
        tail.len(),
        spread * 1000.0
    );
}

#[test]
fn a_card_is_lit_as_a_round_lock() {
    // **A flat card lit flat is a ribbon** (#316). Classified by the normal
    // pass: every card was one colour edge to edge, and a head of them read
    // as a bundle of dark straps. Each edge's normal is bevelled outward
    // about the spine now, so the strip shades as the half-cylinder a lock
    // is. Read as the angle between a station's two normals, which was zero.
    let head = Head::wearing(ScalpStyle::Bob { fringe: 0.8 });
    let normals = &head.hair.mesh.normals;
    let mut flat = 0usize;
    let mut stations = 0usize;
    for pair in normals.as_chunks::<2>().0 {
        stations += 1;
        let apart = pair[0].dot(pair[1]).clamp(-1.0, 1.0).acos();
        if apart < 40f32.to_radians() {
            flat += 1;
        }
    }
    assert!(
        flat == 0,
        "{flat} of {stations} card stations are lit flat across their width"
    );
}

#[test]
fn every_card_is_cut_from_one_lane_of_the_strand_mask() {
    // #340. The strand mask carries its locks side by side, and a card's
    // texture coordinates have to cover exactly one of them, edge to edge, at
    // every station: a card straddling two would draw half of each lock with a
    // gutter down its middle, one off every lane would draw nothing, and one
    // on a strip of a lane would draw a lock with no edges. Down the card, the
    // root has to be the mask's first row and the tip its last, clear one, or
    // the lock frays somewhere other than at its end.
    //
    // Read off the mesh a renderer is handed, with all five regions grown, on
    // every scalp style. And every lane has to be in use: a hash that sent
    // every card to one lane would pass the rest of this and cut one lock
    // everywhere.
    let spans: Vec<(f32, f32)> = (0..LANES).map(StrandMask::lane_span).collect();
    for style in [
        ScalpStyle::Crop,
        ScalpStyle::Bob { fringe: 0.8 },
        ScalpStyle::Long { weight: 0.8 },
        ScalpStyle::TiedBack { tail: 0.8 },
        ScalpStyle::Curly { curl: 0.8 },
    ] {
        let mut record = AvatarRecord::new("Laned", Archetype::default());
        record.hair.scalp.style = style;
        record.hair.brows.style = BrowStyle::Thick;
        record.hair.moustache.style = MoustacheStyle::Handlebar { sweep: 0.9 };
        record.hair.chin.style = ChinStyle::Full;
        record.hair.flanks.style = FlankStyle::FullConnect { reach: 0.7 };
        let avatar = Avatar::build(&record).expect("a biped builds");
        let hair = avatar
            .drawn(0.0)
            .into_iter()
            .find(|mesh| mesh.kind == MeshKind::Hair)
            .expect("a head of hair is drawn");
        let mesh = &hair.mesh;
        assert_eq!(
            mesh.uvs.len(),
            mesh.positions.len(),
            "{style:?}: the hair is not mapped"
        );
        let mut used = [false; LANES as usize];
        for card in cards_of(&mesh.faces) {
            let uvs = &mesh.uvs[card];
            let Some(lane) = spans
                .iter()
                .position(|(from, _)| (uvs[0].x - from).abs() < 1e-5)
            else {
                panic!(
                    "{style:?}: a card's edge is at u {}, which is no lane's edge",
                    uvs[0].x
                );
            };
            let (from, to) = spans[lane];
            used[lane] = true;
            for station in uvs.chunks_exact(2) {
                assert!(
                    (station[0].x - from).abs() < 1e-5 && (station[1].x - to).abs() < 1e-5,
                    "{style:?}: a card cut from lane {lane} ({from}..{to}) has a station across \
                     u {}..{}",
                    station[0].x,
                    station[1].x
                );
            }
            let (root, tip) = (uvs[0].y, uvs[uvs.len() - 1].y);
            assert!(
                root.abs() < 1e-6 && (tip - 1.0).abs() < 1e-6,
                "{style:?}: a card runs down the mask from v {root} to v {tip}, not root to tip"
            );
        }
        assert!(
            used.iter().all(|in_use| *in_use),
            "{style:?}: the lanes in use are {used:?}"
        );
    }
}
