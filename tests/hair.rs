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
use std::collections::HashMap;
use std::ops::Range;

use symbios_avatar::face::{Canon, Skull};
use symbios_avatar::hair::mask::{LANES, StrandMask};
use symbios_avatar::hair::strand_mask;
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
    /// Where each kind of hair grows on this head, and is painted.
    follicles: Follicles,
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
            follicles,
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
/// begins wherever the step does not. Any other face is not a card's - a
/// tail's knot lump (#342) - and is passed over.
fn cards_of(faces: &[Vec<u32>]) -> Vec<Range<usize>> {
    let mut cards = Vec::new();
    let mut start: Option<u32> = None;
    let mut last = 0u32;
    for face in faces {
        let first = face[0];
        if !is_card(face) {
            continue;
        }
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
    let (crown, _) = head.crown_and_throat();
    let mut front_tips_behind = 0usize;
    let mut front = 0usize;
    let mut back_tips_ahead = 0usize;
    let mut back = 0usize;
    let mut tail: Vec<Vec3> = Vec::new();
    for card in head.cards() {
        // **Which way a card faces is read below the crown's whorl** (#342):
        // a tied-back card turns round the pole as it leaves it, so its
        // second station is up to a whorl's turn from its own meridian, and
        // read there a front card counted as a back one. Five centimetres
        // down, the turn is done; a card rising from the nape starts below.
        let below = (0..Head::stations(&card))
            .map(|index| head.station(&card, index))
            .find(|at| at.y < crown - 0.05)
            .unwrap_or_else(|| head.station(&card, Head::stations(&card) - 1));
        let facing = below.x.atan2(below.z).cos();
        // And that far down the comb has begun turning a side card toward
        // the back whether or not it is gathered: a card from 82 degrees, pulled
        // a little under half-way, reads 111 degrees there and ends at the side
        // hairline. Back is therefore read further round than before.
        let tip = head.station(&card, Head::stations(&card) - 1);
        if facing > 0.5 {
            front += 1;
            front_tips_behind += usize::from(tip.z < 0.0);
        } else if facing < -0.6 {
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

/// How much of the scalp painted at full strength behind the temple plane no
/// card covers, in square metres: `(painted, bare)`.
///
/// **Covered means a card within 3 mm of the column straight out of the skin,
/// up to 24 mm out** (#342), which is what a viewer looking at that patch of
/// skin sees in front of it. The nearest card within 3 mm of the skin itself was
/// the first reading, and it failed its control: a crop, which the render shows
/// wholly covered, read 36% bare, because a card lies its lift and the envelope's
/// offset off the body and a card crossing others rides higher still.
///
/// Sampled over the body's own faces at about two millimetres, where the scalp
/// mask the painter uses is at 0.9 or more. With `cards` false the hair is
/// ignored, which is the instrument's liveness: it must read everything bare.
fn uncovered(head: &Head, cards: bool) -> (f32, f32) {
    const REACH: f32 = 0.003;
    const COLUMN: f32 = 0.024;
    const CELL: f32 = 0.008;
    const SPACING: f32 = 0.002;
    let key = |at: Vec3| {
        (
            (at.x / CELL).floor() as i32,
            (at.y / CELL).floor() as i32,
            (at.z / CELL).floor() as i32,
        )
    };
    let mesh = &head.hair.mesh;
    let mut grid: HashMap<(i32, i32, i32), Vec<[Vec3; 3]>> = HashMap::new();
    if cards {
        for face in &mesh.faces {
            for fan in 1..face.len() - 1 {
                let tri = [face[0], face[fan], face[fan + 1]].map(|at| mesh.positions[at as usize]);
                let low = key(tri[0].min(tri[1]).min(tri[2]) - Vec3::splat(REACH));
                let high = key(tri[0].max(tri[1]).max(tri[2]) + Vec3::splat(REACH));
                for x in low.0..=high.0 {
                    for y in low.1..=high.1 {
                        for z in low.2..=high.2 {
                            grid.entry((x, y, z)).or_default().push(tri);
                        }
                    }
                }
            }
        }
    }
    let near = |at: Vec3| {
        grid.get(&key(at)).is_some_and(|tris| {
            tris.iter()
                .any(|[a, b, c]| closest_on_triangle(at, *a, *b, *c).0.distance(at) <= REACH)
        })
    };
    let body = &head.body;
    let normals = body.shading_normals();
    let front = head.clearance.front;
    let (mut painted, mut bare) = (0.0f32, 0.0f32);
    for face in &body.faces {
        let local: Vec<Vec3> = face
            .iter()
            .map(|at| body.positions[*at as usize] - head.origin)
            .collect();
        if local.iter().all(|at| at.length() > 0.25 || at.z >= front) {
            continue;
        }
        for fan in 1..local.len() - 1 {
            let (a, b, c) = (local[0], local[fan], local[fan + 1]);
            let (na, nb, nc) = (
                normals[face[0] as usize],
                normals[face[fan] as usize],
                normals[face[fan + 1] as usize],
            );
            let longest = a.distance(b).max(b.distance(c)).max(c.distance(a));
            let steps = ((longest / SPACING).ceil() as usize).clamp(1, 60);
            let area = (b - a).cross(c - a).length() * 0.5 / (steps * steps) as f32;
            for i in 0..steps {
                for j in 0..steps - i {
                    for (u, v) in [(1.0 / 3.0, 1.0 / 3.0), (2.0 / 3.0, 2.0 / 3.0)] {
                        if u > 0.5 && i + j + 1 >= steps {
                            continue;
                        }
                        let (u, v) = ((i as f32 + u) / steps as f32, (j as f32 + v) / steps as f32);
                        let at = a + (b - a) * u + (c - a) * v;
                        if at.z >= front
                            || head
                                .follicles
                                .weight(symbios_avatar::hair::Follicle::Scalp, at)
                                < 0.9
                        {
                            continue;
                        }
                        painted += area;
                        let out = (na + (nb - na) * u + (nc - na) * v).normalize_or(Vec3::Y);
                        let reached =
                            (0..=8).any(|step| near(at + out * (COLUMN * step as f32 / 8.0)));
                        if !reached {
                            bare += area;
                        }
                    }
                }
            }
        }
    }
    (painted, bare)
}

/// Whether a face is a card's quad, `[s, s+1, s+3, s+2]`: see [`cards_of`].
fn is_card(face: &[u32]) -> bool {
    face.len() == 4 && face[1] == face[0] + 1 && face[2] == face[0] + 3 && face[3] == face[0] + 2
}

#[test]
fn a_tied_back_head_covers_its_painted_scalp_behind_the_temples() {
    // **Bare temples and a V of paint under the tail** (#342). A tied-back
    // head's gathered cards all start at the crown and leave the scalp for
    // the knot at the knot's height, so the scalp behind the ear, and all of
    // it between the knot and the nape's hairline, had nothing on it: the
    // sheets showed painted scalp there in both renderers. Measured before
    // #342 on the default head at the default density of 0.6, read as below:
    // 10.3%, 16.6% and 27.3% of the full-strength paint behind the temple
    // plane bare at tails of 0.3, 0.6 and 0.9. Wider cards and back cards that
    // rise to the knot from the nape bring it to the bound below.
    //
    // The instrument is checked both ways first: with no cards it reads every
    // sample bare, and on a crop, which the render shows covered, none.
    let crop = Head::wearing(ScalpStyle::Crop);
    let (painted, bare) = uncovered(&crop, false);
    assert!(
        painted > 0.03 && (bare - painted).abs() < 1e-9,
        "the coverage reading does not see a head with no hair on it as bare: {bare} of {painted}"
    );
    let (painted, bare) = uncovered(&crop, true);
    assert!(
        bare == 0.0,
        "a crop reads {:.1} cm2 of {:.1} cm2 bare, so the reading is not what the render shows",
        bare * 1e4,
        painted * 1e4
    );
    for tail in [0.3f32, 0.6, 0.9] {
        let head = Head::wearing(ScalpStyle::TiedBack { tail });
        let (painted, bare) = uncovered(&head, true);
        let share = bare / painted;
        println!(
            "tail {tail}: {:.1} cm2 of {:.1} cm2 bare ({:.1}%)",
            bare * 1e4,
            painted * 1e4,
            share * 100.0
        );
        assert!(
            share <= BARE_BEHIND_THE_TEMPLES,
            "a tied-back head at tail {tail} leaves {:.1}% of its painted scalp behind the \
             temples with no card over it",
            share * 100.0
        );
    }
}

/// The most of a tied-back head's full-strength painted scalp behind the temple
/// plane that may have no card over it; see
/// `a_tied_back_head_covers_its_painted_scalp_behind_the_temples`.
///
/// Measured at #342: 6.3%, 3.6% and 5.0% at tails of 0.3, 0.6 and 0.9. Not
/// zero, as the issue asked: what is left is a strip along the nape's hairline
/// and one behind the ear, and which cards cover them moves with the seating,
/// so a count change of a fifth moved the middle tail from 3.8% to 11.4%.
const BARE_BEHIND_THE_TEMPLES: f32 = 0.07;

/// A ribbon of cards read segment by segment, by area: all of it, where a
/// segment's two stations put its edges on opposite sides of the spine (a
/// bow-tie: the card turned over between two stations), and where either edge
/// runs backwards against the spine (a fold). A segment of no length - a seam
/// (#343) - has no area and is passed over.
fn creases(positions: &[Vec3], cards: &[Range<usize>]) -> (f32, f32, f32) {
    let (mut area, mut crossed, mut folded) = (0.0f32, 0.0f32, 0.0f32);
    for card in cards {
        for station in 0..(card.len() / 2).saturating_sub(1) {
            let at = card.start + station * 2;
            let (l0, r0) = (positions[at], positions[at + 1]);
            let (l1, r1) = (positions[at + 2], positions[at + 3]);
            let spine = (l1 + r1 - l0 - r0) * 0.5;
            if spine.length() < 1e-6 {
                continue;
            }
            let piece = ((r0 - l0).length() + (r1 - l1).length()) * 0.5 * spine.length();
            area += piece;
            if (r0 - l0).dot(r1 - l1) < 0.0 {
                crossed += piece;
            } else if (l1 - l0).dot(spine) <= 0.0 || (r1 - r0).dot(spine) <= 0.0 {
                folded += piece;
            }
        }
    }
    (area, crossed, folded)
}

/// One synthetic card round a 20 mm circle, 35 mm either side of its spine,
/// whose creases are known: its width in the circle's own plane (every edge
/// of it runs backwards on the inside), along the circle's axis (none), or
/// along the axis and turned over at every other station (all bow-ties).
fn hoop(width: Hoop) -> (Vec<Vec3>, Range<usize>) {
    const STATIONS: usize = 24;
    let mut positions = Vec::new();
    for station in 0..STATIONS {
        let turn = std::f32::consts::TAU * station as f32 / STATIONS as f32;
        let radial = Vec3::new(turn.cos(), 0.0, turn.sin());
        let side = match width {
            Hoop::InPlane => radial,
            Hoop::OnAxis => Vec3::Y,
            Hoop::TurnedOver => Vec3::Y * if station % 2 == 0 { 1.0 } else { -1.0 },
        } * 0.035;
        positions.push(radial * 0.020 - side);
        positions.push(radial * 0.020 + side);
    }
    (positions, 0..STATIONS * 2)
}

/// Which way a [`hoop`]'s width lies.
#[derive(Clone, Copy)]
enum Hoop {
    InPlane,
    OnAxis,
    TurnedOver,
}

#[test]
fn a_ringlet_neither_folds_nor_turns_over() {
    // **Torn paper close up and a black shard cloud at distance** (#343). A
    // ringlet is a flat card following a coil, and it was not edge-on that
    // drew the paper - measured on the ring of level cameras a curl's ribbon
    // was edge-on 27% of the time against a straight curtain's 23% - but the
    // card creasing itself: across the head's normal its width lay in the
    // coil's own plane for part of every turn, where the coil's 23 mm radius
    // is less than the card's 35 mm half-width and the inner edge ran
    // backwards; and it turned over between two stations wherever its face
    // passed the skin's side. Measured before #343 on the default head, the
    // hanging ribbon creased so: 10%, 19%, 26% and 27% at curls of 0.3, 0.6,
    // 0.8 and 0.9, 34% at a full curl cut full length, 22% on rolled seed 177.
    // A ringlet's width now lies along its coil's binormal, is seamed where it
    // turns over, and is held under the bend it goes round.
    //
    // The reading is checked both ways first, on cards whose creases are known.
    for (width, crossed, folded, what) in [
        (
            Hoop::InPlane,
            0.0,
            1.0,
            "a card wider than its bend in the bend's plane",
        ),
        (
            Hoop::OnAxis,
            0.0,
            0.0,
            "a card whose width is on its hoop's axis",
        ),
        (
            Hoop::TurnedOver,
            1.0,
            0.0,
            "a card turned over at every station",
        ),
    ] {
        let (positions, card) = hoop(width);
        let (area, bowtie, fold) = creases(&positions, &[card]);
        assert!(
            area > 0.0
                && (bowtie / area - crossed).abs() < 0.01
                && (fold / area - folded).abs() < 0.01,
            "the crease reading gets {what} wrong: {:.0}% turned over and {:.0}% folded",
            bowtie / area * 100.0,
            fold / area * 100.0
        );
    }
    let mut heads: Vec<(String, Head)> = [0.3f32, 0.6, 0.9]
        .into_iter()
        .map(|curl| {
            (
                format!("curl {curl}"),
                Head::wearing(ScalpStyle::Curly { curl }),
            )
        })
        .collect();
    for (label, cut) in [
        ("a full curl at full length", (1.0, 0.5, 0.6, 0.5)),
        ("a full curl at the greediest cut", (1.0, 1.0, 1.0, 1.0)),
    ] {
        let mut record = AvatarRecord::new("Hair", Archetype::default());
        record.hair.scalp.style = ScalpStyle::Curly { curl: 1.0 };
        let scalp = &mut record.hair.scalp.cut;
        (scalp.length, scalp.thickness, scalp.density, scalp.droop) = cut;
        heads.push((
            label.to_string(),
            Head::of(record).expect("a curl grows hair"),
        ));
    }
    let mut rolled = AvatarRecord::new("Rolled", Archetype::default());
    rolled.reroll(177);
    assert!(
        matches!(rolled.hair.scalp.style, ScalpStyle::Curly { .. }),
        "rolled seed 177 is no longer a curl, so it is not the control it is named as"
    );
    heads.push((
        "rolled seed 177".to_string(),
        Head::of(rolled).expect("a curl grows hair"),
    ));
    for (label, head) in &heads {
        let (area, bowtie, fold) = creases(&head.hair.mesh.positions, &head.cards());
        println!(
            "{label}: {:.0} cm2 of card, {:.2}% turned over, {:.2}% folded",
            area * 1e4,
            bowtie / area * 100.0,
            fold / area * 100.0
        );
        assert!(
            (bowtie + fold) <= CREASED * area,
            "{label}: {:.1}% of its ribbon is turned over or folded",
            (bowtie + fold) / area * 100.0
        );
    }
}

/// The most of a curl's card area that may be turned over or folded; see
/// `a_ringlet_neither_folds_nor_turns_over`.
///
/// Measured at #343: none, on all six heads, the greediest cut among them. A
/// ringlet whose width lies on its coil's binormal, seamed where it turns over
/// and held under its bend, has no crease by construction, so the bound is the
/// construction's.
const CREASED: f32 = 0.0;

/// The default body growing one chin style and one flank style and nothing else
/// on its head.
struct Beard {
    /// The hair, head-local, as the renderer gets it.
    hair: Option<symbios_avatar::hair::Growth>,
    /// The body's faces near the jaw, head-local, as triangles.
    skin: Vec<[Vec3; 3]>,
    /// The measured skull.
    skull: Skull,
    /// Where each kind of hair grows on this head.
    follicles: Follicles,
}

impl Beard {
    fn wearing(chin: ChinStyle, flanks: FlankStyle) -> Self {
        let mut record = AvatarRecord::new("Beard", Archetype::default());
        record.hair.scalp.style = ScalpStyle::None;
        record.hair.brows.style = BrowStyle::None;
        record.hair.moustache.style = MoustacheStyle::None;
        record.hair.chin.style = chin;
        record.hair.flanks.style = flanks;
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        let origin = follicles.origin();
        let body = &avatar.parts.body;
        let (low, high) = (skull.chin() - 0.06, canon.mouth_line() + 0.02);
        let skin = body
            .faces
            .iter()
            .flat_map(|face| {
                let local: Vec<Vec3> = face
                    .iter()
                    .map(|at| body.positions[*at as usize] - origin)
                    .collect();
                (1..local.len() - 1)
                    .map(move |fan| [local[0], local[fan], local[fan + 1]])
                    .collect::<Vec<_>>()
            })
            .filter(|[a, _, _]| a.y > low && a.y < high && a.length() < 0.25)
            .collect();
        Self {
            hair: avatar.parts.hair.clone(),
            skin,
            skull,
            follicles,
        }
    }

    /// The hair's triangles.
    fn triangles(&self) -> Vec<[Vec3; 3]> {
        let Some(hair) = &self.hair else {
            return Vec::new();
        };
        let mesh = &hair.mesh;
        mesh.faces
            .iter()
            .flat_map(|face| {
                (1..face.len() - 1).map(move |fan| {
                    [face[0], face[fan], face[fan + 1]].map(|at| mesh.positions[at as usize])
                })
            })
            .collect()
    }
}

/// How many of a beard's jawline points, from the angle of the jaw round to the
/// menton on both sides, are further than [`JAWLINE_REACH`] from any card; how
/// many were read; and the furthest any point is, with its signed facing (the
/// side's sign on the azimuth's cosine).
fn bare_jawline(beard: &Beard) -> (usize, usize, (f32, f32)) {
    const STEPS: usize = 40;
    let hair = beard.triangles();
    let (mut bare, mut read, mut worst) = (0usize, 0usize, (0.0f32, 0.0f32));
    for step in 0..=STEPS {
        let facing = step as f32 / STEPS as f32;
        let on = beard
            .skull
            .surface_at(beard.follicles.jawline(facing), facing.acos());
        for side in [1.0f32, -1.0] {
            if step == STEPS && side < 0.0 {
                continue;
            }
            // On the built skin, not on the skull's own profile of it.
            let wanted = Vec3::new(on.x * side, on.y, on.z);
            let point = beard
                .skin
                .iter()
                .map(|[a, b, c]| closest_on_triangle(wanted, *a, *b, *c).0)
                .min_by(|one, two| one.distance(wanted).total_cmp(&two.distance(wanted)))
                .unwrap_or(wanted);
            let apart = hair
                .iter()
                .map(|[a, b, c]| closest_on_triangle(point, *a, *b, *c).0.distance(point))
                .fold(f32::MAX, f32::min);
            read += 1;
            if apart.min(1.0) > worst.0 {
                worst = (apart.min(1.0), facing * side);
            }
            if apart > JAWLINE_REACH {
                bare += 1;
            }
        }
    }
    (bare, read, worst)
}

/// How far a jawline point may be from a card, in metres; see
/// `a_full_beard_draws_its_whole_jawline`.
const JAWLINE_REACH: f32 = 0.004;

#[test]
fn a_full_beard_draws_its_whole_jawline() {
    // **The flanks and the chin met without the jaw between them** (#344).
    // The issue's reading, as it asked for it: at full flanks and a full chin,
    // no point of the jawline from the angle of the jaw to the menton more than
    // 4 mm from a card. It PASSED before the fix all but for the menton (two of
    // 81 points, the worst 4.3 mm there): a flank clump combs down and stops ON
    // the jawline, so its tip touches every point of it. What the sheet showed
    // bare was under the jaw, and paint as coverage is what closed it. So this
    // holds the line at none, which the jaw row along the border does, and its
    // liveness is a chin with its flanks shaved.
    let shaved = bare_jawline(&Beard::wearing(ChinStyle::Full, FlankStyle::None));
    println!("flanks shaved: {} of {} bare", shaved.0, shaved.1);
    assert!(
        shaved.0 * 2 > shaved.1,
        "a full chin with its flanks shaved reads only {} of {} jawline points bare, so the \
         reading does not see a missing flank",
        shaved.0,
        shaved.1
    );
    for reach in [0.0f32, 0.7, 1.0] {
        let (bare, read, worst) = bare_jawline(&Beard::wearing(
            ChinStyle::Full,
            FlankStyle::FullConnect { reach },
        ));
        println!(
            "full flanks {reach}: {bare} of {read} bare, the furthest {:.2} mm at facing {:+.3}",
            worst.0 * 1000.0,
            worst.1
        );
        assert!(
            bare == 0,
            "at full flanks {reach} and a full chin {bare} of {read} jawline points are more than \
             {:.0} mm from a card, the furthest {:.2} mm at facing {:+.3}",
            JAWLINE_REACH * 1000.0,
            worst.0 * 1000.0,
            worst.1
        );
    }
}

#[test]
fn a_braided_rope_neither_folds_nor_turns_over() {
    // **A braid was a ribbon knotted on itself, and its first rope was crumpled
    // foil** (#344). Measured on the default head, a braid's cards were 4% to
    // 6% turned over before it was a rope, and the first rope 9% to 11%: its
    // strands' width turned a third of a circle across a segment the sampler
    // never split, the spine there being nearly straight. A strand winds by
    // the integral of how much of a rope it is, the loft follows the width's
    // turn where a card asks, and none of it creases. The reading is the
    // ringlet's own, checked on its hoops there.
    for twist in [0.0f32, 0.5, 1.0] {
        let beard = Beard::wearing(ChinStyle::Braided { twist }, FlankStyle::None);
        let hair = beard.hair.as_ref().expect("a braid grows hair");
        let cards = cards_of(&hair.mesh.faces);
        let (area, bowtie, fold) = creases(&hair.mesh.positions, &cards);
        println!(
            "braid {twist}: {} cards, {:.0} cm2, {:.2}% turned over, {:.2}% folded",
            cards.len(),
            area * 1e4,
            bowtie / area * 100.0,
            fold / area * 100.0
        );
        assert!(
            area > 0.0 && (bowtie + fold) <= CREASED * area,
            "a braid at twist {twist}: {:.1}% of its cards are turned over or folded",
            (bowtie + fold) / area.max(f32::EPSILON) * 100.0
        );
    }
}

#[test]
fn a_sideburn_is_a_strip_down_to_its_drop() {
    // **A sideburn was two dashes** (#344): its clumps combed down by the
    // flanks' own short reach and most were declined as too short, so a
    // sideburn at a full drop was six tabs by the ear. It is a strip now: every
    // clump runs from under the beard line to the drop's floor. Read off the
    // built mesh, per side: how many cards, and how much of the height from the
    // beard line to the jawline at its own azimuth each one spans.
    for drop in [0.5f32, 1.0] {
        let beard = Beard::wearing(ChinStyle::None, FlankStyle::Sideburns { drop });
        let hair = beard.hair.as_ref().expect("a sideburn grows hair");
        let line = beard.follicles.beard_line();
        for side in [1.0f32, -1.0] {
            let mut spans: Vec<f32> = Vec::new();
            for card in cards_of(&hair.mesh.faces) {
                let points = &hair.mesh.positions[card];
                let middle =
                    points.iter().fold(Vec3::ZERO, |sum, at| sum + *at) / points.len() as f32;
                if middle.x * side <= 0.0 {
                    continue;
                }
                let facing = middle.z / (middle.x * middle.x + middle.z * middle.z).sqrt();
                let (low, high) = points.iter().fold((f32::MAX, f32::MIN), |(low, high), at| {
                    (low.min(at.y), high.max(at.y))
                });
                spans.push((high - low) / (line.top(facing) - beard.follicles.jawline(facing)));
            }
            spans.sort_by(|one, two| two.total_cmp(one));
            println!("drop {drop}, side {side}: spans {spans:.2?}");
            let long = spans
                .iter()
                .filter(|span| **span >= STRIP_SPAN * drop)
                .count();
            assert!(
                long >= STRIP_CARDS,
                "a sideburn at drop {drop} has {long} cards on one side spanning {:.0}% of the \
                 height from its line to the jawline, where a strip has {STRIP_CARDS}: {spans:.2?}",
                STRIP_SPAN * drop * 100.0
            );
        }
    }
}

/// How many of a sideburn's cards a side runs the strip's height; see
/// `a_sideburn_is_a_strip_down_to_its_drop`.
const STRIP_CARDS: usize = 3;

/// What share of the height from the beard line to the jawline a strip card
/// spans at a drop of one, and that share of it at a shorter drop; see
/// `a_sideburn_is_a_strip_down_to_its_drop`.
///
/// Measured at #344 on the default head: four and five cards a side, spanning
/// 0.84 to 0.92 of it at a drop of one and 0.48 to 0.55 at a half. Before, a
/// sideburn's clumps combed down by the flanks' own reach of about 21 mm, and
/// the few not declined as too short were tabs.
const STRIP_SPAN: f32 = 0.8;

#[test]
fn a_tail_is_knotted_by_a_closed_lump_the_budget_pays_for() {
    // **A tail's knot is a lump, not the cards passing through it** (#342).
    // Every gathered card meets at one point behind the head, and from the
    // side that point was the edges of the cards: nothing to tie. A tied-back
    // head draws one small closed solid there, and it has to be what a
    // budget counts - it is geometry a renderer draws - so it is counted with
    // the region's triangles, which are counted from the mesh.
    //
    // Read off the built mesh: the faces that are not a card's quads.
    let crop = Head::wearing(ScalpStyle::Crop);
    assert!(
        crop.hair.mesh.faces.iter().all(|face| is_card(face)),
        "a crop has a face that is not a card's: only a tail is knotted"
    );
    for tail in [0.3f32, 0.6, 0.9] {
        let head = Head::wearing(ScalpStyle::TiedBack { tail });
        let mesh = &head.hair.mesh;
        let lump: Vec<&Vec<u32>> = mesh.faces.iter().filter(|face| !is_card(face)).collect();
        let tris: usize = lump.iter().map(|face| face.len() - 2).sum();
        assert!(
            (40..=60).contains(&tris),
            "tail {tail}: the knot is {tris} triangles, where a lump is 40 to 60"
        );
        // In the ledger: the region's count is every face the mesh draws.
        let drawn: usize = mesh.faces.iter().map(|face| face.len() - 2).sum();
        let counted: usize = head.hair.grown.iter().map(|grown| grown.tris).sum();
        assert_eq!(
            counted, drawn,
            "tail {tail}: the scalp's ledger says {counted} triangles and the mesh draws {drawn}"
        );
        // Closed: every edge is shared by exactly two of its faces.
        let mut edges: HashMap<(u32, u32), usize> = HashMap::new();
        for face in &lump {
            for (index, from) in face.iter().enumerate() {
                let to = face[(index + 1) % face.len()];
                *edges.entry((*from.min(&to), *from.max(&to))).or_default() += 1;
            }
        }
        assert!(
            edges.values().all(|count| *count == 2),
            "tail {tail}: the knot is not a closed solid"
        );
        // Wound outward, about its own middle.
        let corners: Vec<u32> = edges.keys().flat_map(|(a, b)| [*a, *b]).collect();
        let centre = corners
            .iter()
            .fold(Vec3::ZERO, |sum, at| sum + mesh.positions[*at as usize])
            / corners.len() as f32;
        for face in &lump {
            let [a, b, c] = [0, 1, 2].map(|at| mesh.positions[face[at] as usize]);
            let middle = face
                .iter()
                .fold(Vec3::ZERO, |sum, at| sum + mesh.positions[*at as usize])
                / face.len() as f32;
            assert!(
                (b - a).cross(c - a).dot(middle - centre) > 0.0,
                "tail {tail}: a face of the knot turns into it"
            );
        }
        // Solid under the strand mask a renderer cuts the hair out of.
        for at in &corners {
            let alpha = strand_mask().alpha(mesh.uvs[*at as usize]);
            assert!(
                alpha >= 0.5,
                "tail {tail}: the knot is cut away by the strand mask (alpha {alpha})"
            );
        }
        // Behind the head, where the tail hangs from.
        assert!(
            centre.z < -head.skull.depth_behind(centre.y).abs() * 0.8,
            "tail {tail}: the knot sits at {centre:?}, not behind the head"
        );
    }
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
            let corners = &mesh.positions[card.clone()];
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
            // **Either way round, but only turned round at a seam** (#343). A
            // ringlet that turns over is seamed there - the station drawn twice
            // at the same two points, once each way - and its lane follows
            // where an edge IS, so a strand runs on across the seam. So a
            // station may run its lane backwards, and the way round may change
            // only between two stations that are the same two points swapped.
            let mut last: Option<(bool, [Vec3; 2])> = None;
            for (index, station) in uvs.chunks_exact(2).enumerate() {
                let forward =
                    (station[0].x - from).abs() < 1e-5 && (station[1].x - to).abs() < 1e-5;
                let turned = (station[0].x - to).abs() < 1e-5 && (station[1].x - from).abs() < 1e-5;
                assert!(
                    forward || turned,
                    "{style:?}: a card cut from lane {lane} ({from}..{to}) has a station across \
                     u {}..{}",
                    station[0].x,
                    station[1].x
                );
                let at = [corners[index * 2], corners[index * 2 + 1]];
                if let Some((was, before)) = last
                    && was != turned
                {
                    assert!(
                        at[0].distance(before[1]) < 1e-6 && at[1].distance(before[0]) < 1e-6,
                        "{style:?}: a card turns its lane round at station {index} without a \
                         seam there"
                    );
                }
                last = Some((turned, at));
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
