//! The head a beast's plan declares (#367): a frame and a smooth union of
//! analytic parts, which the build shapes the head's surface onto.
//!
//! A human skull is shaped by profiles measured off life and applied to the
//! built head (`face::skull`). A beast's cannot be: there is no measured
//! mannequin of a dog, a cat or a horse to take profiles from, and the three
//! are not one shape at three scales. What they share is a construction - the
//! one a head is drawn with: a braincase, a muzzle set into it at the stop,
//! the upper lip's lobes either side of the nose, a lower jaw under it and the
//! cheeks behind the mouth. So that is what this is, built from the plan's
//! axes by `plan::derive::quadruped`, and [`crate::face::beast`] projects the
//! head's own refined surface onto it.
//!
//! **Declared, not measured.** The later parts of the head - the eyes, the
//! ears, the nose - are seated against the same description (#367), so
//! nothing downstream has to measure a mesh to find out where the plan put the
//! brow.
//!
//! Distances are signed, negative inside, and a bound rather than exact
//! wherever a part is not a sphere: the one question asked of them is where a
//! ray leaves the head, and a bisection needs only the sign.

use glam::{Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::Pupil;

/// An ellipsoid in the head's frame: its centre and its three radii, in
/// metres.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Blob {
    /// Its centre.
    pub(crate) centre: Vec3,
    /// Its radii along the frame's axes.
    pub(crate) radii: Vec3,
}

/// A rounded cone in the head's frame: a ball at each end and the surface
/// between them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Cone {
    /// The back ball's centre.
    pub(crate) back: Vec3,
    /// The back ball's radius.
    pub(crate) back_radius: f32,
    /// The front ball's centre.
    pub(crate) front: Vec3,
    /// The front ball's radius.
    pub(crate) front_radius: f32,
}

/// The muzzle and the face under the eyes: superelliptic sections run from
/// inside the braincase to the nose, tapering as they go and rounded off at
/// the front.
///
/// **Described by its profile lines**, because that is how a head is read
/// side on: the top line is the bridge of the nose, which the stop sets
/// below the forehead, and the bottom line is the jaw's. Every pair is
/// `(back, front)`, and a section between the ends is the two interpolated
/// along the frame's `+Z`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Muzzle {
    /// Where the run starts and ends along the frame's `+Z`.
    pub(crate) span: Vec2,
    /// How high the top line runs: the bridge of the nose.
    pub(crate) top: Vec2,
    /// How high the bottom line runs: the jaw.
    pub(crate) bottom: Vec2,
    /// The section's half-width.
    pub(crate) half_width: Vec2,
    /// The sections' superellipse exponent: two is an ellipse, and higher is
    /// squarer - a boxer's muzzle against a borzoi's.
    pub(crate) square: f32,
    /// The radius the front is rounded off by.
    pub(crate) round: f32,
    /// How far the top line bows up between the ends: a Roman nose.
    pub(crate) crown: f32,
}

/// The nose (#367): a leather pad at the muzzle's tip, if the head has one,
/// and the nostrils cut into whichever of the pad and the muzzle ends it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct NoseSeat {
    /// The leather pad: a dog's ball, a cat's button. `None` on a soft
    /// muzzle.
    pub(crate) leather: Option<Blob>,
    /// The `+X` nostril, cut out of the surface; the other is its mirror.
    pub(crate) nostril: Blob,
    /// How softly the pad joins the muzzle and the nostrils open into it.
    pub(crate) blend: f32,
}

/// How many whiskers grow on each side of the muzzle.
pub(crate) const WHISKERS: usize = 4;

/// The whiskers (#367): where the `+X` side's sprout on the lip's lobe and
/// the way each sets off; the other side is their mirror.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct WhiskerSeat {
    /// Where each sprouts, sunk a little into the lobe.
    pub(crate) roots: [Vec3; WHISKERS],
    /// The way each sets off, as unit vectors.
    pub(crate) directions: [Vec3; WHISKERS],
    /// How long each is, in metres.
    pub(crate) length: f32,
    /// How far a whisker's tip sags below its line, as a share of its length.
    pub(crate) droop: f32,
}

/// One piece a head wears (#402), or the `+X` side's of a pair: a tapered tube along
/// a line that starts at [`Self::base`] and turns as it goes, in the head's frame,
/// in metres. `plan::derive::quadruped` says where it stands, reading the head and
/// nothing else, and `face::beast::adorn` sweeps it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct OrnamentSeat {
    /// The middle of its base, sunk into whatever it grows from.
    pub(crate) base: Vec3,
    /// The way it sets off, a unit vector.
    pub(crate) direction: Vec3,
    /// The way it turns as it goes, a unit vector square to [`Self::direction`].
    pub(crate) bend: Vec3,
    /// Square to both, the way a coil steps aside to clear itself.
    pub(crate) side: Vec3,
    /// What the sweep holds its section's first axis along.
    pub(crate) across: Vec3,
    /// Base to tip along its line.
    pub(crate) length: f32,
    /// Its radius at the base.
    pub(crate) radius: f32,
    /// How much of its radius it has lost at the tip.
    pub(crate) taper: f32,
    /// How far its line turns toward [`Self::bend`], in whole turns: never
    /// negative.
    pub(crate) curve: f32,
    /// How far its flutes turn about it, in whole turns.
    pub(crate) twist: f32,
    /// How deep its flutes are, as a share of its radius.
    pub(crate) flutes: f32,
    /// Its colour at the base and at the tip.
    pub(crate) colours: [Vec3; 2],
    /// How many facets its section has.
    pub(crate) sides: u8,
    /// Whether it has a `-X` twin.
    pub(crate) pair: bool,
    /// An antler's tines: how many, how wide they fork, `0..=1`, and the way that is
    /// forward as the body stands, the way they reach.
    pub(crate) tines: Option<(u8, f32, Vec3)>,
}

/// How many stations a piece with no curve to follow is swept through.
const PIECE_STATIONS: usize = 6;

/// How many more it takes per whole turn of curve.
/// Provenance: **tuned by render** (#402): a ram's curl keeps its round at 12, 30
/// degrees a station under smooth normals, against the triangles a pair costs.
const STATIONS_PER_TURN: f32 = 12.0;

/// How far a coil steps aside per turn, in the piece's radii at its base: two would
/// lay a turn against the one before it. Provenance: **chosen** (#402).
const COIL_STEP: f32 = 2.2;

/// How much a turn tightens toward the tip at most, as a share of it: a horn's curl
/// is a spiral rather than a circle. Provenance: **tuned by render** (#402).
const TIGHTENING: f32 = 0.5;

impl OrnamentSeat {
    /// How many stations its line is swept through.
    pub(crate) fn stations(&self) -> usize {
        PIECE_STATIONS + (self.curve * STATIONS_PER_TURN).ceil() as usize
    }

    /// Its line, base to tip. A piece with no curve is a straight run, and it steps
    /// aside only once its turn is well past a quarter: under that it cannot meet
    /// itself, and a sideways shift on a gentle curve would only take it out of
    /// the plane its seat chose.
    pub(crate) fn line(&self) -> Vec<Vec3> {
        let count = self.stations();
        let last = (count - 1) as f32;
        if self.curve == 0.0 {
            return (0..count)
                .map(|station| self.base + self.direction * (self.length * station as f32 / last))
                .collect();
        }
        let tight = TIGHTENING * smoothstep(0.0, 0.5, self.curve);
        let turned = |along: f32| {
            std::f32::consts::TAU * self.curve * (along * (1.0 - tight) + tight * along * along)
        };
        let step = self.length / last;
        let mut line = Vec::with_capacity(count);
        let mut on_plane = self.base;
        line.push(self.base);
        for station in 1..count {
            let middle = turned((station as f32 - 0.5) / last);
            on_plane += (self.direction * middle.cos() + self.bend * middle.sin()) * step;
            let angle = turned(station as f32 / last);
            let aside = COIL_STEP * self.radius * angle / std::f32::consts::TAU
                * smoothstep(0.3, 0.55, angle / std::f32::consts::TAU);
            line.push(on_plane + self.side * aside);
        }
        line
    }

    /// Its radius `along` of the way from base to tip, before any flute.
    pub(crate) fn radius_at(&self, along: f32) -> f32 {
        self.radius * (1.0 - self.taper * along)
    }

    /// Its line read at `count` evenly spaced shares of the way from base to tip,
    /// each with its share, whatever its stations are (#404): what a seat asks of
    /// the head, so that nothing it finds steps where a curve adds a station.
    pub(crate) fn read_along(&self, count: usize) -> Vec<(Vec3, f32)> {
        let line = self.line();
        let last = (line.len() - 1) as f32;
        (0..count)
            .map(|at| {
                let along = at as f32 / (count - 1) as f32;
                let station = along * last;
                let below = (station.floor() as usize).min(line.len() - 2);
                let point = line[below].lerp(line[below + 1], station - below as f32);
                (point, along)
            })
            .collect()
    }

    /// The `-X` twin of a pair: its mirror, its flutes turning the other way.
    pub(crate) fn mirrored(&self) -> Self {
        let mirror = Vec3::new(-1.0, 1.0, 1.0);
        Self {
            base: self.base * mirror,
            direction: self.direction * mirror,
            bend: self.bend * mirror,
            side: self.side * mirror,
            across: self.across * mirror,
            twist: -self.twist,
            ..*self
        }
    }
}

/// `0` under `low`, `1` over `high`, and an S between them.
fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let share = ((value - low) / (high - low)).clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// Where the `+X` eye sits (#367); the other is its mirror.
///
/// Seated with the head rather than found on it: the derive casts from the
/// braincase's centre to where `eyeSet` puts the eye, and sinks the globe
/// along the surface's own normal there, which is also where it looks - a
/// cat's on the front of its face, a horse's on the side of its skull.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct EyeSeat {
    /// The globe's centre, in the head's frame.
    pub(crate) centre: Vec3,
    /// The turn from the frame's `+Z` to where the eye looks at rest.
    pub(crate) facing: Quat,
    /// The globe's radius, before the record's eye size.
    pub(crate) radius: f32,
    /// The pupils' shape.
    pub(crate) pupil: Pupil,
}

/// Where the `+X` ear stands and how it is made (#367); the other is its
/// mirror.
///
/// An ear is a part, not more of the head's surface: a card of leather with
/// a cup in it, standing off the braincase - see `face::beast::ears` for the
/// card. Declared here because where it stands is the skull's business, as
/// the eyes' seat is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EarSeat {
    /// The middle of the ear's base, sunk a little into the braincase, in
    /// the head's frame.
    pub(crate) base: Vec3,
    /// The ear's own frame at its base, turned into the head's: `+Y` runs up
    /// the ear, `+Z` is the way its cup opens and `+X` is across it.
    pub(crate) frame: Quat,
    /// Base to tip, in metres.
    pub(crate) length: f32,
    /// Across the base, in metres.
    pub(crate) width: f32,
    /// How pointed the tip is: zero a round-tipped leaf, one a triangle.
    pub(crate) point: f32,
    /// How deep the cup is, as a share of the half-width.
    pub(crate) cup: f32,
    /// The leather's thickness at the base, in metres.
    pub(crate) thickness: f32,
    /// Where along the ear it folds, as a share of its length.
    pub(crate) fold_at: f32,
    /// The axis it folds about, in the ear's own frame.
    pub(crate) fold_axis: Vec3,
    /// How far it folds, in radians: none for a pricked ear.
    pub(crate) fold: f32,
}

/// The neck's upper run as the head it carries sizes it (#388): a tube of
/// elliptical sections, in the head's frame, from a little way down the neck
/// to inside the skull.
///
/// **Sized off the head, not off the family.** The cage's neck has one ring,
/// whose radius the chest's depth and the family set, and subdivided it leaves
/// a neck half the depth of the head it runs into: the crease the head's
/// surface then wraps round. So the sections are measured off the finished
/// head (`plan::derive::quadruped`), and the surface of both the neck and the
/// head is shaped onto them by `face::beast`, so the two cannot disagree where
/// they meet. **Not part of [`BeastHead::distance`]**: that is the head, which
/// a coat's marks and the jaw ask about, and a neck is not one.
///
/// A point is read as `(u, v, x)`: how far along the chord from [`Self::base`]
/// to [`Self::end`] as a share of it, how far toward the crest from the chord,
/// and how far across.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Nape {
    /// Where the run starts, on the neck's line. Below it the neck is the
    /// cage's.
    pub(crate) base: Vec3,
    /// Where the run ends, inside the skull.
    pub(crate) end: Vec3,
    /// How far along the chord the neck joint stands, where the sections take
    /// their full size.
    pub(crate) knee: f32,
    /// How far the neck joint stands off the chord toward the crest: the
    /// neck's bend, which the centre line follows as a parabola.
    pub(crate) bow: f32,
    /// A section's half-extents `(across, deep)` at the neck joint: the throat.
    pub(crate) throat: Vec2,
    /// And where the run ends, inside the skull.
    pub(crate) seat: Vec2,
    /// How softly the run joins the head: a smooth union's radius, in metres.
    pub(crate) blend: f32,
}

/// How softly each part joins the head: a smooth union's radius, in metres.
/// Small is a crease and large a slope - the stop, above all.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Blends {
    /// The back of the skull into the braincase.
    pub(crate) occiput: f32,
    /// The muzzle into the braincase: how sharp the stop is.
    pub(crate) muzzle: f32,
    /// The upper lip's lobes into the muzzle.
    pub(crate) pads: f32,
    /// The cheeks into the head.
    pub(crate) cheeks: f32,
}

/// A beast's head, as its plan declares it.
///
/// **The frame** is the cranium joint at rest, turned so that its `+Z` runs
/// forward along the head, `+Y` up and `+X` toward the body's left; every part
/// is in it, in metres. A part named in the singular that a head has two of -
/// the lip's lobes, the cheeks - is given for the `+X` side and mirrored.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeastHead {
    /// The frame's origin: the cranium joint at rest, in body space.
    pub origin: Vec3,
    /// The frame's turn out of body space.
    pub turn: Quat,
    /// The cranium's radius, which every part is authored in.
    pub radius: f32,
    /// The straight line the surface is projected from, back to front: from
    /// the back of the skull to the nose.
    pub(crate) axis: [Vec3; 2],
    /// The braincase.
    pub(crate) cranium: Blob,
    /// The back of the skull, where the neck comes in.
    pub(crate) occiput: Blob,
    /// The poll: the top of the back of the skull, the corner a long head's
    /// forehead turns down its nape at, between and behind the ears (#388).
    /// Grown after every seat is cast, so a seat never reads it.
    pub(crate) poll: Option<Blob>,
    /// The muzzle.
    pub(crate) muzzle: Muzzle,
    /// The upper lip's lobes either side of the nose: a cat's whisker pads, a
    /// dog's flews, a horse's lips.
    pub(crate) pads: Blob,
    /// The lower jaw, from its angle to the chin.
    ///
    /// **Not part of the head's own surface** (#367): it is a part of its
    /// own, hinged at [`Self::hinge`], so the mouth can open. The muzzle's
    /// bottom line is the upper lip's, and the jaw stands up into the head
    /// above it; where the two surfaces cross is the line of the closed
    /// mouth. See `face::beast::jaw`.
    pub(crate) jaw: Cone,
    /// Where the jaw hinges, under the ear and behind the eye.
    pub(crate) hinge: Vec3,
    /// The tongue, lying along the jaw's floor from its root to its tip.
    pub(crate) tongue: Cone,
    /// The cheeks: a cat's broad face, a horse's jowl.
    pub(crate) cheeks: Blob,
    /// The neck's upper run, if the head is carried on one.
    pub(crate) nape: Option<Nape>,
    /// The nose.
    pub(crate) nose: NoseSeat,
    /// How softly the parts join.
    pub(crate) blends: Blends,
    /// Where the eyes sit.
    pub(crate) eye: EyeSeat,
    /// Where the ears stand.
    pub(crate) ear: EarSeat,
    /// The whiskers, if the head has any.
    pub(crate) whiskers: Option<WhiskerSeat>,
    /// What it wears, a slot a kind in `OrnamentKind::ALL`'s order (#402).
    pub(crate) ornaments: [Option<OrnamentSeat>; 6],
}

/// How far out, in head radii, a ray starts its march inward. Past every part
/// of every head the plan can declare: the longest muzzle ends a little over
/// three radii from the axis's back end, and the march starts from the axis
/// point nearest the vertex rather than from that end.
const REACH: f32 = 3.0;

/// How many steps a ray takes per head radius on its march inward. Fine
/// enough that no part this thin can be stepped over: the lip's lobes, the
/// thinnest, are a fifth of a radius across at their smallest.
const STEPS_PER_RADIUS: f32 = 24.0;

/// How many times the crossing a march brackets is halved: to a 2^-10 share
/// of a step, which on a 75 mm cranium is under four micrometres.
const BISECTIONS: usize = 10;

/// How many steps a point takes down the distance's gradient onto the drawn
/// surface: each takes off most of what the bound leaves.
const SETTLE_STEPS: usize = 2;

/// How much nearer the head a point may lie than the neck's upper run, in head
/// radii, and still be charted with the run: the back of the skull beside the
/// crease lies round the head chart's pole, and is as much a neck to a chart.
/// Provenance: **tuned against the chart census and by render** (#395): none left
/// a hard-edged sliver of the head's chart on the back of a dog's skull, and 0.3
/// took the cheeks of a short face.
const ON_RUN: f32 = 0.15;

/// How far either side of the crease between the head and the neck's upper run
/// a face still counts as in the crook, in the run's blends.
/// Provenance: **tuned against the inward-face census** (#395).
const CROOK: f32 = 6.0;

/// How far from the nose a face has to be to count as in the crook, as a share
/// of the head's length: a head hung low brings its muzzle against the run, and
/// the rim of a nostril or a lip is a cell turned on purpose.
/// Provenance: **tuned by render** (#395), at the top of the envelope, long faces
/// and short.
const CROOK_NOSE: f32 = 0.35;

/// How far along the run a face has to be to count as in the crook, as a share
/// of the run: the run's start is a square cap, whose gradient says nothing of
/// which way the skin there faces.
/// Provenance: **chosen** (#395), under the 0.4 the lowest fold starts at.
const CROOK_FROM: f32 = 0.25;

impl BeastHead {
    /// A body-space point, in the head's frame.
    #[must_use]
    pub fn local(&self, body: Vec3) -> Vec3 {
        self.turn.inverse() * (body - self.origin)
    }

    /// A point in the head's frame, in body space.
    #[must_use]
    pub fn body(&self, local: Vec3) -> Vec3 {
        self.origin + self.turn * local
    }

    /// How far a point in the head's frame is from the head's surface:
    /// negative inside.
    #[must_use]
    pub fn distance(&self, at: Vec3) -> f32 {
        let mirrored = Vec3::new(at.x.abs(), at.y, at.z);
        let blends = &self.blends;
        let mut distance = self.cranium.distance(at);
        distance = smooth_union(distance, self.occiput.distance(at), blends.occiput);
        if let Some(poll) = &self.poll {
            distance = smooth_union(distance, poll.distance(at), blends.occiput);
        }
        distance = smooth_union(distance, self.muzzle.distance(at), blends.muzzle);
        distance = smooth_union(distance, self.pads.distance(mirrored), blends.pads);
        distance = smooth_union(distance, self.cheeks.distance(mirrored), blends.cheeks);
        let nose = &self.nose;
        if let Some(leather) = &nose.leather {
            distance = smooth_union(distance, leather.distance(at), nose.blend);
        }
        // A nostril is a hollow: the head minus the nostril, eased at its
        // rim as a union is at its crease.
        -smooth_union(-distance, nose.nostril.distance(mirrored), nose.blend)
    }

    /// [`Self::distance`] with the neck's upper run joined on: the surface the
    /// head's vertices are shaped onto, which the head proper is not all of.
    #[must_use]
    pub fn drawn_distance(&self, at: Vec3) -> f32 {
        let head = self.distance(at);
        self.nape.as_ref().map_or(head, |nape| {
            smooth_union(head, nape.distance(at), nape.blend)
        })
    }

    /// How far the jaw reaches to the side at `z`, as `face::beast::jaw` builds it.
    pub(crate) fn jaw_wide_at(&self, z: f32) -> f32 {
        (self.muzzle.half_width_at(z) * JAW_WIDTH).max(self.jaw.at(z).1)
    }

    /// The middle of the nose, in the head's frame: the leather pad's
    /// centre, or the midpoint between the nostrils of a soft muzzle.
    #[must_use]
    pub fn nose_centre(&self) -> Vec3 {
        self.nose.leather.map_or_else(
            || self.nose.nostril.centre * Vec3::new(0.0, 1.0, 1.0),
            |leather| leather.centre,
        )
    }

    /// Whether a point in the head's frame is on the nose's leather: inside
    /// the pad's own blob, `margin` of the way past its surface included.
    #[must_use]
    pub fn on_leather(&self, at: Vec3, margin: f32) -> bool {
        self.nose
            .leather
            .is_some_and(|leather| leather.distance(at) < margin)
    }

    /// Where the drawn surface is - the head and the neck's upper run joined -
    /// seen through `at`, in the head's frame: on the ray through it from the line
    /// it is built round.
    ///
    /// **That line is the head's axis near the head and the neck's upper run near
    /// the run, because the run is a tube and a ray from elsewhere can lie down it
    /// (#395).** A neck carried high under a nodded head hangs below the head in
    /// the head's own frame, so a ray through the throat from the head's axis runs
    /// inside the run for most of its length and leaves it low on the neck, inside
    /// the chest: the outermost crossing of the two together drew the skin from
    /// the jaw to that point and folded it back on the way up. Cast from the run's
    /// own centre line, a ray leaves by the wall beside the vertex. Which surface
    /// a vertex is nearer decides which line it is cast from, blended across the
    /// crease between them so that nothing jumps.
    ///
    /// **The head's outermost crossing comes first, because that is the one that
    /// shows**: the march runs inward from outside every part and stops at the
    /// first sample inside any, so a ray that clips a lip's lobe on its way to the
    /// jaw lands on the lobe. From there the landing is the first way out of the
    /// head and the run together. `None` when `at` gives the ray no direction.
    #[must_use]
    pub fn surface_through(&self, at: Vec3) -> Option<Vec3> {
        let [back, front] = self.axis;
        let along = front - back;
        let share =
            ((at - back).dot(along) / along.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
        let on_axis = back + along * share;
        let anchor = self.nape.as_ref().map_or(on_axis, |nape| {
            let lead = (self.distance(at) - nape.distance(at)) / nape.blend;
            let lean = (0.5 * (lead + 1.0)).clamp(0.0, 1.0);
            on_axis.lerp(nape.line_beside(at), lean * lean * (3.0 - 2.0 * lean))
        });
        let direction = (at - anchor).try_normalize()?;
        let start = march(|at| self.distance(at), anchor, direction, self.radius).unwrap_or(anchor);
        let step = self.radius / STEPS_PER_RADIUS;
        if self.drawn_distance(start) > 0.0 {
            return Some(start);
        }
        // A march's end lies on the head, and on no other surface, unless the run
        // stands over it.
        let (mut inside, mut outside, mut gone) = (start, start, 0.0);
        while self.drawn_distance(outside) <= 0.0 {
            inside = outside;
            gone += step;
            outside = start + direction * gone;
            if gone > REACH * self.radius {
                return Some(start);
            }
        }
        for _ in 0..BISECTIONS {
            let middle = inside.lerp(outside, 0.5);
            if self.drawn_distance(middle) <= 0.0 {
                inside = middle;
            } else {
                outside = middle;
            }
        }
        Some(inside.lerp(outside, 0.5))
    }

    /// `at`, a point near the drawn surface, moved onto it down the distance's
    /// gradient. A few steps, because the distance is a bound rather than exact
    /// and the first falls short.
    #[must_use]
    pub(crate) fn settled(&self, at: Vec3) -> Vec3 {
        let mut point = at;
        for _ in 0..SETTLE_STEPS {
            let off = self.drawn_distance(point);
            point -= self.drawn_normal(point) * off;
        }
        point
    }

    /// The drawn surface's outward normal at `at`, in the head's frame.
    #[must_use]
    pub(crate) fn drawn_normal(&self, at: Vec3) -> Vec3 {
        gradient(|at| self.drawn_distance(at), at, self.radius)
    }

    /// Whether a point in the head's frame lies on the neck's upper run for
    /// charting: nearer the run's surface than the head's, or as good as.
    #[must_use]
    pub(crate) fn on_run(&self, at: Vec3) -> bool {
        self.nape
            .as_ref()
            .is_some_and(|nape| nape.distance(at) < self.distance(at) + ON_RUN * self.radius)
    }

    /// Whether a point in the head's frame lies in the crease between the head
    /// and the neck's upper run.
    #[must_use]
    pub(crate) fn in_crook(&self, at: Vec3) -> bool {
        self.nape.as_ref().is_some_and(|nape| {
            nape.read(at).0 > CROOK_FROM
                && at.distance(self.nose_centre())
                    > CROOK_NOSE * self.axis[0].distance(self.axis[1])
                && self.distance(at).abs().max(nape.distance(at).abs()) < CROOK * nape.blend
        })
    }

    /// Where a point of the neck's upper run goes, in the head's frame: onto
    /// the run's section, as much as it is along the run, or where it is when
    /// the head declares no run.
    #[must_use]
    pub fn carried(&self, at: Vec3) -> Vec3 {
        self.nape.as_ref().map_or(at, |nape| nape.carried(at))
    }

    /// Where the neck's upper run's skin is on the line out of its centre through
    /// `at`, a point in the head's frame, and how much of the drawn neck there is
    /// the run's: none below the run's start, all of it from the neck joint on as
    /// [`Self::carried`] shapes it, and less again where the head takes over (#407:
    /// a mane lies on it). `None` off the run, or with no run declared.
    #[must_use]
    pub(crate) fn nape_skin(&self, at: Vec3) -> Option<(Vec3, f32)> {
        let nape = self.nape.as_ref()?;
        let (u, v, x) = nape.read(at);
        if !(0.0..1.0).contains(&u) {
            return None;
        }
        let half = nape.half(u).max(Vec2::splat(1.0e-5));
        let off = Vec2::new(x, v - nape.centre(u));
        let scale = (off / half).length();
        if scale <= f32::EPSILON {
            return None;
        }
        let drawn = off / scale - off;
        let (along, _) = nape.chord();
        let on = at + Vec3::X * drawn.x + Nape::crestward(along) * drawn.y;
        let fades_in = smoothstep(0.0, nape.knee.max(f32::EPSILON), u);
        let lead = (self.distance(at) - nape.distance(at)) / nape.blend;
        let run = smoothstep(0.0, 1.0, 0.5 * (lead + 1.0));
        Some((on, fades_in * run))
    }

    /// How far along the neck's upper run a point is, for choosing the faces
    /// worth refining: negative below it. `None` when the head declares no run.
    #[must_use]
    pub(crate) fn along_nape(&self, at: Vec3) -> Option<f32> {
        self.nape.as_ref().map(|nape| nape.read(at).0)
    }

    /// The surface's outward normal at `at`, in the head's frame: the
    /// distance's gradient, by central differences a thousandth of a radius
    /// apart.
    #[must_use]
    pub fn normal(&self, at: Vec3) -> Vec3 {
        gradient(|at| self.distance(at), at, self.radius)
    }

    /// Where a ray from `from` along the unit `direction` leaves the head, in
    /// the head's frame: the outermost crossing, as [`Self::surface_through`]
    /// takes it. `None` when the ray never enters the head, or when the head
    /// still holds the point the march starts from.
    #[must_use]
    pub fn leave(&self, from: Vec3, direction: Vec3) -> Option<Vec3> {
        march(|at| self.distance(at), from, direction, self.radius)
    }
}

/// Where a ray from `from` along the unit `direction` leaves the shape
/// `distance` describes: marched inward from [`REACH`] of `unit` out, the
/// first sample inside bracketing the outermost crossing, and bisected.
/// `unit` is the shape's own size - a head's radius, a pad's - which the
/// march's reach and step are measured in.
pub(crate) fn march(
    distance: impl Fn(Vec3) -> f32,
    from: Vec3,
    direction: Vec3,
    unit: f32,
) -> Option<Vec3> {
    let point = |reach: f32| from + direction * reach;
    let step = unit / STEPS_PER_RADIUS;
    let mut outside = unit * REACH;
    if distance(point(outside)) <= 0.0 {
        return None;
    }
    let mut inside = outside - step;
    while distance(point(inside)) > 0.0 {
        outside = inside;
        inside -= step;
        if inside < 0.0 {
            return None;
        }
    }
    for _ in 0..BISECTIONS {
        let middle = 0.5 * (inside + outside);
        if distance(point(middle)) <= 0.0 {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    Some(point(0.5 * (inside + outside)))
}

/// The outward normal of the shape `distance` describes at `at`: its
/// gradient, by central differences a thousandth of `unit` apart.
pub(crate) fn gradient(distance: impl Fn(Vec3) -> f32, at: Vec3, unit: f32) -> Vec3 {
    let step = unit * 1.0e-3;
    let along = |axis: Vec3| distance(at + axis * step) - distance(at - axis * step);
    Vec3::new(along(Vec3::X), along(Vec3::Y), along(Vec3::Z)).normalize_or(Vec3::Z)
}

impl Blob {
    /// A bound on the distance from `at`, negative inside: exact on a sphere,
    /// and never further than the true distance on an ellipsoid.
    pub(crate) fn distance(&self, at: Vec3) -> f32 {
        let radii = self.radii.max(Vec3::splat(1.0e-5));
        let offset = at - self.centre;
        let scaled = (offset / radii).length();
        let gradient = (offset / (radii * radii)).length();
        if gradient <= f32::EPSILON {
            return -radii.min_element();
        }
        scaled * (scaled - 1.0) / gradient
    }
}

impl Nape {
    /// The chord's direction and length.
    fn chord(&self) -> (Vec3, f32) {
        let chord = self.end - self.base;
        let length = chord.length().max(f32::EPSILON);
        (chord / length, length)
    }

    /// The way toward the crest, square to the chord and across the body.
    fn crestward(along: Vec3) -> Vec3 {
        Vec3::new(0.0, along.z, -along.y).normalize_or(Vec3::Y)
    }

    /// A point as `(u, v, x)`: see the struct.
    fn read(&self, at: Vec3) -> (f32, f32, f32) {
        let (along, length) = self.chord();
        let from = at - self.base;
        (
            from.dot(along) / length,
            from.dot(Self::crestward(along)),
            at.x,
        )
    }

    /// The point of the run's centre line beside `at`, in the head's frame.
    fn line_beside(&self, at: Vec3) -> Vec3 {
        let (along, length) = self.chord();
        let u = self.read(at).0.clamp(0.0, 1.0);
        self.base + along * (u * length) + Self::crestward(along) * self.centre(u)
    }

    /// How far toward the crest the centre line stands at `u`: a parabola
    /// through the chord's ends and the neck joint.
    fn centre(&self, u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        let knee = self.knee.clamp(0.05, 0.95);
        self.bow * u * (1.0 - u) / (knee * (1.0 - knee))
    }

    /// A section's half-extents at `u`: the throat's up to the neck joint,
    /// opening smoothly to the seat's inside the skull.
    fn half(&self, u: f32) -> Vec2 {
        let share = ((u - self.knee) / (1.0 - self.knee).max(f32::EPSILON)).clamp(0.0, 1.0);
        let share = share * share * (3.0 - 2.0 * share);
        self.throat.lerp(self.seat, share)
    }

    /// A bound on the distance from `at`, negative inside, capped square at
    /// the run's two ends.
    fn distance(&self, at: Vec3) -> f32 {
        let (u, v, x) = self.read(at);
        let half = self.half(u).max(Vec2::splat(1.0e-5));
        let round = Vec2::new(x, v - self.centre(u)) / half;
        let side = (round.length() - 1.0) * half.min_element();
        let (_, length) = self.chord();
        side.max(-u * length).max((u - 1.0) * length)
    }

    /// Where a point of the run goes: onto its section, along the line out of
    /// the centre through it, faded in from the run's start to the neck joint
    /// so the neck leaves the cage's surface without a step.
    fn carried(&self, at: Vec3) -> Vec3 {
        let (u, v, x) = self.read(at);
        if u <= 0.0 {
            return at;
        }
        let share = (u / self.knee.max(f32::EPSILON)).clamp(0.0, 1.0);
        let share = share * share * (3.0 - 2.0 * share);
        let half = self.half(u).max(Vec2::splat(1.0e-5));
        let off = Vec2::new(x, v - self.centre(u));
        let scale = (off / half).length();
        if scale <= f32::EPSILON {
            return at;
        }
        let drawn = off / scale - off;
        let (along, _) = self.chord();
        at + (Vec3::X * drawn.x + Self::crestward(along) * drawn.y) * share
    }
}

/// How wide the jaw is against the muzzle over it: a little narrower, so
/// the upper lip hangs over its sides.
pub(crate) const JAW_WIDTH: f32 = 0.85;

impl Cone {
    /// Its axis's height and its radius at `z` along the frame, held at the ends
    /// past them.
    pub(crate) fn at(&self, z: f32) -> (f32, f32) {
        let share = ((z - self.back.z) / (self.front.z - self.back.z)).clamp(0.0, 1.0);
        (
            self.back.y + (self.front.y - self.back.y) * share,
            self.back_radius + (self.front_radius - self.back_radius) * share,
        )
    }
}

impl Muzzle {
    /// The section's half-width at `z` along the frame, held at the ends of
    /// the run past them.
    pub(crate) fn half_width_at(&self, z: f32) -> f32 {
        let length = (self.span.y - self.span.x).max(f32::EPSILON);
        let share = ((z - self.span.x) / length).clamp(0.0, 1.0);
        self.half_width.x + (self.half_width.y - self.half_width.x) * share
    }

    /// The distance from `at` to the run of sections, the front rounded off:
    /// a bound, exact across the flats.
    fn distance(&self, at: Vec3) -> f32 {
        let length = (self.span.y - self.span.x).max(f32::EPSILON);
        let share = ((at.z - self.span.x) / length).clamp(0.0, 1.0);
        let between = |pair: Vec2| pair.x + (pair.y - pair.x) * share;
        // The Roman nose bows the top line and leaves the bottom where it is.
        let top = between(self.top) + self.crown * (std::f32::consts::PI * share).sin();
        let bottom = between(self.bottom);
        let width = between(self.half_width).max(1.0e-5);
        let height = (0.5 * (top - bottom)).max(1.0e-5);
        let centre = 0.5 * (top + bottom);
        let power = self.square.max(1.0);
        let across =
            (at.x.abs() / width).powf(power) + ((at.y - centre).abs() / height).powf(power);
        let side = (across.powf(power.recip()) - 1.0) * width.min(height);
        let front = at.z - self.span.y;
        let back = self.span.x - at.z;
        round_intersection(side, front, self.round).max(back)
    }
}

/// A smooth union of two distances: the polynomial smooth minimum, whose
/// `blend` is how far either side of the crease the two surfaces are eased
/// into each other. Zero is a plain union.
pub(crate) fn smooth_union(a: f32, b: f32, blend: f32) -> f32 {
    if blend <= 0.0 {
        return a.min(b);
    }
    let share = (0.5 + 0.5 * (b - a) / blend).clamp(0.0, 1.0);
    b + (a - b) * share - blend * share * (1.0 - share)
}

/// An intersection of two distances with the edge between them rounded off
/// at `radius`.
pub(crate) fn round_intersection(a: f32, b: f32, radius: f32) -> f32 {
    let outside = Vec2::new(a + radius, b + radius).max(Vec2::ZERO);
    (-radius).min(a.max(b)) + outside.length()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A head of one ball, every other part shrunk into it: the simplest
    /// shape whose surface is known exactly.
    fn ball(radius: f32) -> BeastHead {
        let inside = Blob {
            centre: Vec3::ZERO,
            radii: Vec3::splat(radius * 0.1),
        };
        BeastHead {
            origin: Vec3::new(0.0, 1.0, 0.5),
            turn: Quat::from_rotation_x(0.3),
            radius,
            axis: [
                Vec3::new(0.0, 0.0, -0.2 * radius),
                Vec3::new(0.0, 0.0, 0.2 * radius),
            ],
            cranium: Blob {
                centre: Vec3::ZERO,
                radii: Vec3::splat(radius),
            },
            occiput: inside,
            poll: None,
            muzzle: Muzzle {
                span: Vec2::new(0.0, 0.05 * radius),
                top: Vec2::splat(0.05 * radius),
                bottom: Vec2::splat(-0.05 * radius),
                half_width: Vec2::splat(0.05 * radius),
                square: 2.0,
                round: 0.01 * radius,
                crown: 0.0,
            },
            pads: inside,
            jaw: Cone {
                back: Vec3::ZERO,
                back_radius: 0.05 * radius,
                front: Vec3::Z * 0.01,
                front_radius: 0.05 * radius,
            },
            hinge: Vec3::ZERO,
            tongue: Cone {
                back: Vec3::ZERO,
                back_radius: 0.02 * radius,
                front: Vec3::Z * 0.01,
                front_radius: 0.02 * radius,
            },
            cheeks: inside,
            nape: None,
            nose: NoseSeat {
                leather: None,
                nostril: inside,
                blend: 0.0,
            },
            blends: Blends {
                occiput: 0.0,
                muzzle: 0.0,
                pads: 0.0,
                cheeks: 0.0,
            },
            eye: EyeSeat {
                centre: Vec3::new(0.3, 0.2, 0.9) * radius,
                facing: Quat::IDENTITY,
                radius: 0.2 * radius,
                pupil: Pupil::Round,
            },
            ear: EarSeat {
                base: Vec3::new(0.4, 0.8, 0.0) * radius,
                frame: Quat::IDENTITY,
                length: radius,
                width: 0.6 * radius,
                point: 0.5,
                cup: 0.4,
                thickness: 0.03 * radius,
                fold_at: 0.5,
                fold_axis: Vec3::X,
                fold: 0.0,
            },
            whiskers: None,
            ornaments: [None; 6],
        }
    }

    #[test]
    fn a_ray_lands_on_the_surface_it_was_cast_at() {
        let head = ball(0.1);
        for at in [
            Vec3::new(0.03, 0.02, 0.0),
            Vec3::new(-0.01, -0.05, 0.04),
            Vec3::new(0.0, 0.2, 0.3),
        ] {
            let landed = head.surface_through(at).expect("a ray off the axis lands");
            assert!(
                (landed.length() - 0.1).abs() < 1.0e-4,
                "{at} landed at {landed}, {:.5} from the centre",
                landed.length()
            );
            // On the ray from the axis point nearest `at`, so it kept its side.
            assert!(landed.x * at.x >= 0.0 && landed.y * at.y >= 0.0);
        }
    }

    #[test]
    fn a_ball_is_normal_along_its_radius() {
        let head = ball(0.1);
        let at = Vec3::new(0.06, 0.0, 0.08);
        assert!(head.normal(at).abs_diff_eq(at.normalize(), 1.0e-3));
    }

    #[test]
    fn a_point_on_the_axis_gives_no_ray() {
        let head = ball(0.1);
        assert_eq!(head.surface_through(Vec3::new(0.0, 0.0, 0.01)), None);
    }

    #[test]
    fn the_frame_round_trips() {
        let head = ball(0.1);
        let at = Vec3::new(0.2, -0.4, 0.9);
        assert!(head.body(head.local(at)).abs_diff_eq(at, 1.0e-6));
        // And +Z is forward along the turn: the frame is the turned body.
        assert!(
            head.body(Vec3::Z)
                .abs_diff_eq(head.origin + head.turn * Vec3::Z, 1.0e-6)
        );
    }

    #[test]
    fn a_smooth_union_reaches_past_either_part_and_a_hard_one_does_not() {
        assert_eq!(smooth_union(0.3, 0.5, 0.0), 0.3);
        assert!(
            smooth_union(0.3, 0.3, 0.2) < 0.3,
            "eased where the two meet"
        );
        assert_eq!(
            smooth_union(0.0, 1.0, 0.2),
            0.0,
            "and untouched far from it"
        );
    }

    #[test]
    fn a_squarer_muzzle_fills_its_corners() {
        let muzzle = |square: f32| Muzzle {
            span: Vec2::new(-1.0, 1.0),
            top: Vec2::ONE,
            bottom: -Vec2::ONE,
            half_width: Vec2::ONE,
            square,
            round: 0.1,
            crown: 0.0,
        };
        let corner = Vec3::new(0.8, 0.8, 0.0);
        assert!(
            muzzle(2.0).distance(corner) > 0.0,
            "an ellipse misses the corner"
        );
        assert!(muzzle(4.0).distance(corner) < 0.0, "a box holds it");
    }

    #[test]
    fn a_roman_crown_raises_the_top_line_and_leaves_the_bottom() {
        let muzzle = |crown: f32| Muzzle {
            span: Vec2::new(0.0, 2.0),
            top: Vec2::ONE,
            bottom: -Vec2::ONE,
            half_width: Vec2::ONE,
            square: 2.0,
            round: 0.1,
            crown,
        };
        let top = Vec3::new(0.0, 1.1, 1.0);
        let bottom = Vec3::new(0.0, -0.99, 1.0);
        assert!(muzzle(0.0).distance(top) > 0.0);
        assert!(
            muzzle(0.3).distance(top) < 0.0,
            "the bow takes the top line up"
        );
        assert!(
            muzzle(0.3).distance(bottom) < 0.0,
            "and the bottom stays inside"
        );
    }
}
