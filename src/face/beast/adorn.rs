//! A beast's whiskers and what it wears on its head (#367, #402): the head's parts
//! drawn in colours of their own.
//!
//! **In the eyes' draw, not the skin's.** A whisker is white and a horn
//! ivory on any coat, and the skin's draw can only darken its atlas - its
//! vertex colours multiply it - so neither could be drawn there on a dark
//! coat. The eyes' draw is vertex-coloured, glossy and already paid for:
//! a whisker catches the light as a hair does, and a horn as horn does.
//!
//! A whisker is a thin tapered tube rather than a card, so it shows from
//! every side with no alpha to cut it out.
//!
//! **One sweep for every ornament** (`swept`): a tapered tube along a line that
//! turns, with flutes that twist. A horn, a fang and a tusk are that, and an antler
//! is that for its beam and again for each tine. What stands where is
//! `plan::derive::quadruped`'s, read off the head, and what rides the jaw is kept
//! apart here so that the draw can bind it there.

use glam::{Mat4, Vec2, Vec3};

use crate::mesh::PolyMesh;
use crate::plan::beast_head::{OrnamentSeat, WhiskerSeat};
use crate::plan::{BeastHead, OrnamentKind};
use crate::prim;

/// A whisker's colour.
const WHISKER: Vec3 = Vec3::new(0.93, 0.92, 0.89);

/// A whisker's radius at its root, as a share of its length: drawn thicker
/// than life, which would be a line a pixel wide at any distance a face is
/// seen from.
/// Provenance: **tuned by render** (#367).
const WHISKER_GIRTH: f32 = 0.014;

/// Stations along a whisker.
const STATIONS: usize = 6;

/// What a unicorn's horn costs: `a_piece_costs_little_against_the_budget` measures it.
const UNICORN_TRIANGLES: usize = 140;

/// A beast's whiskers and everything it wears (#402), built in its head joint's
/// space and coloured, for the eyes' draw.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde-avatar", derive(serde::Serialize, serde::Deserialize))]
pub struct Adornments {
    /// Both sides' whiskers, as one mesh; empty when the head grows none.
    pub whiskers: PolyMesh,
    /// What rides the head joint: a horn, paired horns, upper fangs, antlers.
    ///
    /// **`horn` on the wire, as it was** (#402): a browser can hold an older
    /// `gen-worker.js` from its cache, whose bodies carry that key, and the worker's
    /// codec is self-describing, so under a new name its every whiskered beast would
    /// fail to decode.
    #[cfg_attr(feature = "serde-avatar", serde(default, rename = "horn"))]
    pub on_head: PolyMesh,
    /// What rides the jaw and opens with the mouth: lower fangs, tusks. Shut, in the
    /// same space as everything else here.
    #[cfg_attr(
        feature = "serde-avatar",
        serde(default, skip_serializing_if = "PolyMesh::is_empty")
    )]
    pub on_jaw: PolyMesh,
    /// The joint [`Self::on_jaw`] rides, once the jaw is rigged: it has to cross a
    /// worker boundary with the mesh, as the jaw's own part stays behind.
    #[cfg_attr(
        feature = "serde-avatar",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub jaw_joint: Option<usize>,
}

impl Adornments {
    /// Builds what a beast's head declares, for the head joint standing at
    /// `origin` in body space.
    #[must_use]
    pub fn build(beast: &BeastHead, origin: Vec3) -> Self {
        let into_head = Mat4::from_translation(beast.origin - origin) * Mat4::from_quat(beast.turn);
        let whiskers = beast.whiskers.as_ref().map_or_else(PolyMesh::new, |seat| {
            let mut mesh = whiskers(seat);
            mesh.paint(WHISKER);
            mesh.transformed(into_head)
        });
        let (mut on_head, mut on_jaw) = (PolyMesh::new(), PolyMesh::new());
        for (kind, seat) in OrnamentKind::ALL.into_iter().zip(&beast.ornaments) {
            if let Some(seat) = seat {
                let worn = if kind.rides_the_jaw() {
                    &mut on_jaw
                } else {
                    &mut on_head
                };
                worn.append(&piece(seat).transformed(into_head));
            }
        }
        Self {
            whiskers,
            on_head,
            on_jaw,
            jaw_joint: None,
        }
    }

    /// Whether the head grows and wears nothing here.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.whiskers.is_empty() && self.on_head.is_empty() && self.on_jaw.is_empty()
    }

    /// How many triangles what it wears costs past what a unicorn's horn does: what
    /// the fur's ceiling gives up, so that the most a body can wear on the longest
    /// fur stays under the budget as a unicorn's body always was (#402), and a body
    /// wearing no more than that grows the fur it always grew.
    #[must_use]
    pub fn triangles_past_a_unicorn(&self) -> usize {
        let triangles = |mesh: &PolyMesh| {
            mesh.faces
                .iter()
                .map(|face| face.len().saturating_sub(2))
                .sum::<usize>()
        };
        (triangles(&self.on_head) + triangles(&self.on_jaw)).saturating_sub(UNICORN_TRIANGLES)
    }

    /// The meshes that ride the head joint as one, empty if the head wears none.
    #[must_use]
    pub fn assembled(&self) -> PolyMesh {
        let mut mesh = self.whiskers.clone();
        mesh.append(&self.on_head);
        mesh
    }
}

/// Every whisker, both sides, in the head's frame.
fn whiskers(seat: &WhiskerSeat) -> PolyMesh {
    let mut mesh = PolyMesh::new();
    for side in [1.0, -1.0] {
        let mirror = Vec3::new(side, 1.0, 1.0);
        for (&root, &direction) in seat.roots.iter().zip(&seat.directions) {
            let path: Vec<Vec3> = (0..STATIONS)
                .map(|station| {
                    let along = station as f32 / (STATIONS - 1) as f32;
                    let sag = Vec3::NEG_Y * (seat.droop * seat.length * along * along);
                    (root + direction * (seat.length * along) + sag) * mirror
                })
                .collect();
            let girth = WHISKER_GIRTH * seat.length;
            mesh.append(&prim::tube(
                &path,
                Vec2::splat(girth),
                Vec2::splat(girth * 0.25),
                3,
            ));
        }
    }
    mesh
}

/// One piece and its twin if it is a pair, in the head's frame.
fn piece(seat: &OrnamentSeat) -> PolyMesh {
    let mut mesh = one_side(seat);
    if seat.pair {
        mesh.append(&one_side(&seat.mirrored()));
    }
    mesh
}

/// One side of a piece: its beam, and a beam's tines.
fn one_side(seat: &OrnamentSeat) -> PolyMesh {
    let line = seat.line();
    let mut mesh = swept(seat, &line, [0.0, 1.0]);
    if let Some((tines, fork, forward)) = seat.tines {
        for tine in 0..tines {
            mesh.append(&tine_of(seat, &line, (tine, tines), fork, forward));
        }
    }
    mesh
}

/// A tapered tube along `line` with two flutes twisting round it as it goes, coloured
/// from its seat's base to its tip over the stretch `shade` of the way along. The one
/// generator (#402): a horn, a fang, a tusk, an antler's beam and each of its tines.
///
/// A unicorn's horn (#367) is this with a straight line, and stayed byte for byte:
/// see `a_unicorn_is_the_horn_it_always_was`.
fn swept(seat: &OrnamentSeat, line: &[Vec3], shade: [f32; 2]) -> PolyMesh {
    let sides = usize::from(seat.sides);
    let last = (line.len() - 1) as f32;
    let outlines: Vec<Vec<Vec2>> = (0..line.len())
        .map(|station| {
            let along = station as f32 / last;
            let radius = seat.radius * (1.0 - seat.taper * along);
            let twist = std::f32::consts::TAU * seat.twist * along;
            (0..sides)
                .map(|side| {
                    let angle = std::f32::consts::TAU * side as f32 / sides as f32;
                    let flute = 1.0 + seat.flutes * (2.0 * angle + twist).sin();
                    Vec2::new(angle.cos(), angle.sin()) * (radius * flute)
                })
                .collect()
        })
        .collect();
    let mut mesh = prim::sweep_outline(line, &outlines, seat.across);
    let [root, end] = seat.colours;
    let colour = |along: f32| root + (end - root) * (shade[0] + (shade[1] - shade[0]) * along);
    mesh.set_colours(
        (0..line.len())
            .flat_map(|station| vec![colour(station as f32 / last); sides])
            .collect(),
    );
    mesh
}

/// Facets round a tine, and stations along one.
const TINE_SIDES: u8 = 6;
const TINE_STATIONS: usize = 5;

/// How long the brow tine is against the beam, the highest tine's share of that, and
/// the angles a tine leaves the beam at, in radians, at no fork and at a full one.
/// Provenance: **tuned by render** (#402), on a stag's five points.
const TINE_LENGTH: f32 = 0.42;
const TINE_LAST: f32 = 0.55;
const TINE_FORK: [f32; 2] = [0.20, 1.40];

/// Where up the beam the first and the last tine leave it.
const TINE_SPAN: [f32; 2] = [0.14, 0.86];

/// How far each tine turns round the beam from the one below it, in radians: a stag's
/// stand in a rough row over the face rather than in a fan.
const TINE_ROUND: f32 = 0.30;

/// One tine: leaving the beam `tine` of `tines` up its length, forward and fanning, and
/// curving back toward the way the beam was going, as a tine does.
fn tine_of(
    seat: &OrnamentSeat,
    line: &[Vec3],
    (tine, tines): (u8, u8),
    fork: f32,
    forward: Vec3,
) -> PolyMesh {
    let up = f32::from(tine) / f32::from((tines - 1).max(1));
    let at = TINE_SPAN[0] + (TINE_SPAN[1] - TINE_SPAN[0]) * up;
    let last = (line.len() - 1) as f32;
    let index = ((at * last) as usize).min(line.len() - 2);
    let local = at * last - index as f32;
    let root = line[index].lerp(line[index + 1], local);
    let ahead = (line[index + 1] - line[index]).normalize_or(seat.direction);
    // Toward the face, whichever way the beam is headed, and round the beam from
    // there step by step. On the far side of a pair, the mirror of the near one.
    let forward = (forward - ahead * ahead.dot(forward)).normalize_or(seat.side);
    let round = TINE_ROUND * (f32::from(tine) - 0.5 * f32::from(tines - 1));
    let handed = if seat.side.x < 0.0 { -round } else { round };
    let out = glam::Quat::from_axis_angle(ahead, handed) * forward;
    let angle = TINE_FORK[0] + (TINE_FORK[1] - TINE_FORK[0]) * fork;
    let direction = ahead * angle.cos() + out * angle.sin();
    let length = seat.length * TINE_LENGTH * (1.0 + (TINE_LAST - 1.0) * up);
    let bend = (ahead - direction * ahead.dot(direction)).normalize_or(seat.side);
    let span = (TINE_STATIONS - 1) as f32;
    let mut along_tine = root;
    let path: Vec<Vec3> = (0..TINE_STATIONS)
        .map(|station| {
            if station > 0 {
                let turned = TINE_TURN * (station as f32 - 0.5) / span;
                along_tine += (direction * turned.cos() + bend * turned.sin()) * (length / span);
            }
            along_tine
        })
        .collect();
    let radius = seat.radius_at(at) * TINE_GIRTH;
    let tine_seat = OrnamentSeat {
        radius,
        taper: 0.85,
        sides: TINE_SIDES,
        across: seat.side,
        ..*seat
    };
    swept(&tine_seat, &path, [at, 1.0])
}

/// How far a tine turns back toward the beam's way along its length, in radians.
const TINE_TURN: f32 = 0.7;

/// A tine's radius at its root, against the beam's there.
const TINE_GIRTH: f32 = 0.72;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, Composites, Ornament, Ornaments, QuadrupedParams};

    fn unicorn() -> Ornaments {
        Ornaments {
            horn: Some(Ornament::UNICORN),
            ..Ornaments::default()
        }
    }

    fn adorned(whiskers: f32, ornaments: Ornaments) -> (BeastHead, Adornments) {
        let params = QuadrupedParams {
            whiskers,
            ornaments,
            ..QuadrupedParams::default()
        };
        let head = params
            .skeleton(&Composites::default())
            .beast_head
            .expect("a quadruped declares its head");
        (head, Adornments::build(&head, head.origin))
    }

    #[test]
    fn a_long_fan_of_whiskers_grows_and_none_is_none() {
        let (_, whiskered) = adorned(0.6, Ornaments::default());
        assert!(whiskered.whiskers.face_count() > 0);
        assert!(
            whiskered.on_head.positions.is_empty(),
            "no horn was asked for"
        );
        let (_, bare) = adorned(-1.0, Ornaments::default());
        assert!(bare.whiskers.positions.is_empty() && bare.assembled().positions.is_empty());
    }

    #[test]
    fn whiskers_reach_out_past_the_face_on_both_sides() {
        let (head, adorned) = adorned(0.6, Ornaments::default());
        let across = adorned
            .whiskers
            .positions
            .iter()
            .map(|point| point.x)
            .fold((f32::MAX, f32::MIN), |(lo, hi), x| (lo.min(x), hi.max(x)));
        let muzzle = head.muzzle.half_width_at(head.pads.centre.z);
        assert!(
            across.1 > muzzle * 1.5 && across.0 < -muzzle * 1.5,
            "{across:?}"
        );
    }

    #[test]
    fn a_unicorn_s_horn_rises_from_its_forehead_as_a_closed_solid() {
        let (head, adorned) = adorned(-1.0, unicorn());
        assert!(adorned.on_head.is_closed_manifold(), "the horn has a hole");
        let tip = adorned
            .on_head
            .positions
            .iter()
            .copied()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("a horn");
        assert!(
            tip.y > head.radius,
            "the tip stands {} m up on a {} m head",
            tip.y,
            head.radius
        );
    }

    #[test]
    fn a_horn_rises_the_same_way_however_the_head_is_carried() {
        // Up and forward as the body stands: a nodding head turns its base,
        // never where it points.
        let pointing = |neck_carriage: f32| {
            let params = QuadrupedParams {
                neck_carriage,
                ornaments: unicorn(),
                ..QuadrupedParams::default()
            };
            let head = params
                .skeleton(&Composites::default())
                .beast_head
                .expect("a quadruped declares its head");
            head.turn * head.ornaments[0].expect("a unicorn's horn").direction
        };
        let (low, high) = (pointing(-1.0), pointing(1.0));
        assert!(low.abs_diff_eq(high, 1.0e-4), "{low} against {high}");
        assert!(low.y > 0.7, "it rises: {low}");
    }

    /// The bodies a piece has to fit: every family, crosses, and heads at the ends of
    /// the axes that move what it stands on.
    fn heads() -> Vec<(String, QuadrupedParams)> {
        use crate::plan::Family;
        let mut all = Vec::new();
        for (name, family, other) in [
            ("dog", Family::Dog, None),
            ("cat", Family::Cat, None),
            ("horse", Family::Horse, None),
            ("dog x horse", Family::Dog, Some(Family::Horse)),
            ("horse x cat", Family::Horse, Some(Family::Cat)),
        ] {
            let mut neutral = QuadrupedParams::default();
            neutral.set_kind(family, other, 0.5);
            all.push((name.to_owned(), neutral));
            for (axis, end) in [
                ("muzzle", 3.0),
                ("muzzle", -3.0),
                ("head", 3.0),
                ("head", -3.0),
                ("ear", 3.0),
                ("ear", -3.0),
            ] {
                let mut params = neutral;
                match axis {
                    "muzzle" => params.muzzle_length = end,
                    "head" => params.head_size = end,
                    _ => {
                        params.ear_size = end;
                        params.ear_carriage = 1.0;
                    }
                }
                all.push((format!("{name} {axis} {end}"), params));
            }
            // Ears at every carriage, which is what moves them on the skull.
            for ear_carriage in [-1.0, 0.0] {
                let mut params = neutral;
                params.ear_size = 1.5;
                params.ear_carriage = ear_carriage;
                all.push((format!("{name} big ears at {ear_carriage}"), params));
            }
        }
        all
    }

    /// Every kind at its preset.
    fn wearing(kind: OrnamentKind) -> Ornaments {
        let mut worn = Ornaments::default();
        worn.wear(kind, true);
        worn
    }

    /// `kind` shaped by `shape`.
    fn shaped(kind: OrnamentKind, shape: impl Fn(&mut Ornament)) -> Ornaments {
        let mut worn = wearing(kind);
        if let Some(piece) = worn.get_mut(kind) {
            shape(piece);
        }
        worn
    }

    fn seat_of(params: &QuadrupedParams, kind: OrnamentKind) -> (BeastHead, OrnamentSeat) {
        let head = params
            .skeleton(&Composites::default())
            .beast_head
            .expect("a head");
        let index = OrnamentKind::ALL
            .iter()
            .position(|&other| other == kind)
            .expect("a kind");
        (head, head.ornaments[index].expect("worn"))
    }

    /// The horn as #367 drew it, copied here when the general sweep replaced it.
    fn the_old_horn(head: &BeastHead) -> PolyMesh {
        let r = head.radius;
        let centre = head.cranium.centre;
        let up = Vec3::new(0.0, 0.62f32.cos(), 0.62f32.sin());
        let forehead = head.leave(centre, up).unwrap_or(centre + up * r);
        let direction = head.turn.inverse() * Vec3::new(0.0, 0.75, 0.66).normalize();
        let base = forehead - direction * (0.06 * r);
        let (length, radius) = (1.3 * r, 0.15 * r);
        const STATIONS: usize = 6;
        const SIDES: usize = 12;
        let path: Vec<Vec3> = (0..STATIONS)
            .map(|station| base + direction * (length * station as f32 / (STATIONS - 1) as f32))
            .collect();
        let outlines: Vec<Vec<Vec2>> = (0..STATIONS)
            .map(|station| {
                let along = station as f32 / (STATIONS - 1) as f32;
                let radius = radius * (1.0 - 0.94 * along);
                let twist = std::f32::consts::TAU * 1.5 * along;
                (0..SIDES)
                    .map(|side| {
                        let angle = std::f32::consts::TAU * side as f32 / SIDES as f32;
                        let flute = 1.0 + 0.12 * (2.0 * angle + twist).sin();
                        Vec2::new(angle.cos(), angle.sin()) * (radius * flute)
                    })
                    .collect()
            })
            .collect();
        let mut mesh = prim::sweep_outline(&path, &outlines, Vec3::X);
        mesh.paint(Vec3::new(0.96, 0.91, 0.80));
        mesh
    }

    #[test]
    fn a_unicorn_is_the_horn_it_always_was() {
        for (name, mut params) in heads() {
            params.ornaments = unicorn();
            let head = params
                .skeleton(&Composites::default())
                .beast_head
                .expect("a head");
            let worn = Adornments::build(&head, head.origin);
            let into_head = Mat4::from_quat(head.turn);
            let mut old = PolyMesh::new();
            old.append(&the_old_horn(&head).transformed(into_head));
            // The faces and colours exactly, and the positions to a hair: the old
            // horn's angles here are constants the compiler folds, and the new
            // generator's come from libm at run time, so a libm that differs by its
            // last place reads as different bytes. Byte for byte on this one, by
            // the dump.
            assert_eq!(worn.on_head.faces, old.faces, "{name}: its faces");
            assert_eq!(worn.on_head.colours, old.colours, "{name}: its colours");
            assert_eq!(worn.on_head.positions.len(), old.positions.len(), "{name}");
            for (new, was) in worn.on_head.positions.iter().zip(&old.positions) {
                assert!(new.distance(*was) < 1.0e-5, "{name}: {new} against {was}");
            }
        }
    }

    #[cfg(feature = "serde-avatar")]
    #[test]
    fn a_body_from_an_older_worker_still_decodes() {
        // A cached `gen-worker.js` sends `{whiskers, horn}`. Under a new key the
        // decode failed, and the worker's codec `.expect`s it in the client (#402).
        let (_, worn) = adorned(0.6, wearing(OrnamentKind::Horn));
        let mut old = serde_json::to_value(&worn).expect("writes");
        let map = old.as_object_mut().expect("a map");
        assert!(map.contains_key("horn"), "the wire key: {:?}", map.keys());
        map.remove("on_jaw");
        map.retain(|key, _| key == "whiskers" || key == "horn");
        let read: Adornments = serde_json::from_value(old).expect("decodes");
        assert_eq!(read.on_head, worn.on_head);
    }

    #[test]
    fn every_piece_is_closed_coloured_and_rides_what_it_should_on_every_head() {
        for (name, base) in heads() {
            for kind in OrnamentKind::ALL {
                let params = QuadrupedParams {
                    ornaments: wearing(kind),
                    whiskers: -1.0,
                    ..base
                };
                let head = params
                    .skeleton(&Composites::default())
                    .beast_head
                    .expect("a head");
                let worn = Adornments::build(&head, head.origin);
                let (mesh, other) = if kind.rides_the_jaw() {
                    (&worn.on_jaw, &worn.on_head)
                } else {
                    (&worn.on_head, &worn.on_jaw)
                };
                assert!(other.is_empty(), "{name} {kind:?} on the wrong joint");
                assert!(mesh.face_count() > 0, "{name} {kind:?} drew nothing");
                assert!(mesh.is_closed_manifold(), "{name} {kind:?} has a hole");
                assert!(
                    mesh.positions.iter().all(|at| at.is_finite()),
                    "{name} {kind:?} is not finite"
                );
                // Its base colour at its base and its tip's further on, in sRGB.
                let piece = params.ornaments.get(kind).expect("worn");
                let has = |colour: [f32; 3]| {
                    let colour = Vec3::from(colour);
                    mesh.colours.iter().any(|c| c.abs_diff_eq(colour, 1.0e-4))
                };
                assert_eq!(mesh.colours.len(), mesh.positions.len());
                assert!(has(piece.base) && has(piece.tip), "{name} {kind:?}");
            }
        }
    }

    #[test]
    fn a_pair_is_a_mirror() {
        let params = QuadrupedParams {
            ornaments: wearing(OrnamentKind::PairedHorns),
            whiskers: -1.0,
            ..QuadrupedParams::default()
        };
        let (_, seat) = seat_of(&params, OrnamentKind::PairedHorns);
        let (one, other) = (seat.line(), seat.mirrored().line());
        for (one, other) in one.iter().zip(&other) {
            assert!(one.abs_diff_eq(*other * Vec3::new(-1.0, 1.0, 1.0), 1.0e-5));
            assert!(one.x > 0.0, "the near side stays on its own side: {one}");
        }
    }

    #[test]
    fn a_curve_turns_a_piece_and_a_coil_clears_itself() {
        let base = QuadrupedParams::default();
        let (head, straight) = seat_of(
            &QuadrupedParams {
                ornaments: shaped(OrnamentKind::PairedHorns, |piece| piece.curve = 0.0),
                ..base
            },
            OrnamentKind::PairedHorns,
        );
        let r = head.radius;
        let tip = |seat: &OrnamentSeat| *seat.line().last().expect("a line");
        let (_, gentle) = seat_of(
            &QuadrupedParams {
                ornaments: shaped(OrnamentKind::PairedHorns, |piece| piece.curve = 0.2),
                ..base
            },
            OrnamentKind::PairedHorns,
        );
        assert!(tip(&straight).distance(tip(&gentle)) > 0.2 * r);
        // A ram's: more than a turn, and no station of it within its own section of
        // a station far enough along to be the next time round.
        let (_, ram) = seat_of(
            &QuadrupedParams {
                ornaments: shaped(OrnamentKind::PairedHorns, |piece| {
                    piece.curve = -1.5;
                    piece.length = 4.0;
                    piece.taper = 0.5;
                }),
                ..base
            },
            OrnamentKind::PairedHorns,
        );
        let line = ram.line();
        let last = (line.len() - 1) as f32;
        let stride = line.len() / 3;
        for (index, at) in line.iter().enumerate() {
            for (further, other) in line.iter().enumerate().skip(index + stride) {
                let sections =
                    ram.radius_at(index as f32 / last) + ram.radius_at(further as f32 / last);
                assert!(
                    at.distance(*other) > 0.9 * sections,
                    "station {index} runs into {further}"
                );
            }
        }
    }

    /// A piece's tip across every slider that moves its seat - its curve, its length,
    /// its thickness, and the muzzle it stands on - on a dog, a cat, a cross and a snub
    /// cat:
    /// swept coarsely, and the stretches that moved it most followed down to a single
    /// step of the wire's grid, where no step may move it by more than [`STEP_MOST`].
    /// A seat's search eases from one clear place to the next (#404), quickly where a
    /// clear stretch closes, but over several steps of a slider rather than one: the
    /// first clear tilt stepped by up to .53 of a head radius in one.
    ///
    /// **Short of one**: an upper fang's thickness on the snub cat still steps by up to
    /// .33 where its root, drawn in under the lip, moves from one column of places to
    /// the next right at the jaw's shoulder, where the least lean that clears the jaw
    /// turns on a hair of where the root is. #410 holds it.
    #[test]
    fn no_slider_makes_a_piece_jump() {
        const WIRE: f32 = 0.001;
        const COARSE: i32 = 48;
        const FOLLOWED: usize = 3;
        let axes = [
            ("curve", -1.5, 1.5),
            ("length", 0.05, 4.0),
            ("thickness", 0.01, 0.5),
            ("muzzle", -3.0, 3.0),
        ];
        for name in ["dog", "cat", "horse x cat", "cat muzzle -3"] {
            let (_, base) = heads()
                .into_iter()
                .find(|(head, _)| head == name)
                .expect("a sampled head");
            for kind in OrnamentKind::ALL {
                for (axis, low, high) in axes {
                    if name == "cat muzzle -3"
                        && kind == OrnamentKind::UpperFangs
                        && axis == "thickness"
                    {
                        continue;
                    }
                    // The tip, in head radii, at a slider's value in steps of the grid.
                    let tip = |grid: i32| {
                        let value = grid as f32 * WIRE;
                        let mut params = QuadrupedParams {
                            ornaments: shaped(kind, |piece| match axis {
                                "curve" => piece.curve = value,
                                "length" => piece.length = value,
                                "thickness" => piece.thickness = value,
                                _ => {}
                            }),
                            ..base
                        };
                        if axis == "muzzle" {
                            params.muzzle_length = value;
                        }
                        let (head, seat) = seat_of(&params, kind);
                        *seat.line().last().expect("a line") / head.radius
                    };
                    let (from, to) = ((low / WIRE).round() as i32, (high / WIRE).round() as i32);
                    let stride = (to - from) / COARSE;
                    let coarse: Vec<Vec3> =
                        (0..=COARSE).map(|at| tip(from + at * stride)).collect();
                    let mut stretches: Vec<(f32, i32)> = coarse
                        .windows(2)
                        .zip(0..)
                        .map(|(pair, at)| (pair[0].distance(pair[1]), from + at * stride))
                        .collect();
                    stretches.sort_by(|one, other| other.0.total_cmp(&one.0));
                    // And where a curve's bend flips, and a muzzle passes its middle.
                    if low < 0.0 && high > 0.0 {
                        stretches.insert(0, (f32::INFINITY, -stride / 2));
                    }
                    for &(_, start) in stretches.iter().take(FOLLOWED + 1) {
                        // Halved toward whichever half moved it further, to one step.
                        let (mut low, mut high) =
                            ((start, tip(start)), (start + stride, tip(start + stride)));
                        while high.0 - low.0 > 1 {
                            let middle = (low.0 + high.0) / 2;
                            let at = (middle, tip(middle));
                            if low.1.distance(at.1) >= at.1.distance(high.1) {
                                high = at;
                            } else {
                                low = at;
                            }
                        }
                        let moved = low.1.distance(high.1);
                        assert!(
                            moved < STEP_MOST,
                            "{name} {kind:?}: a step of {axis} at {} moved its tip {moved} head \
                             radii",
                            high.0 as f32 * WIRE
                        );
                    }
                }
            }
        }
    }

    /// The most one step of a slider may move a piece's tip by, in head radii: a long
    /// piece's curve moves it by about a fortieth, and a lean eased over a few steps
    /// out of a closing clearance by up to a tenth.
    const STEP_MOST: f32 = 0.12;

    #[test]
    fn nothing_on_the_crown_stands_in_an_ear() {
        for (name, base) in heads() {
            for (kind, curve) in [
                (OrnamentKind::PairedHorns, 0.14),
                (OrnamentKind::PairedHorns, -1.1),
                (OrnamentKind::Antlers, 0.14),
            ] {
                let params = QuadrupedParams {
                    ornaments: shaped(kind, |piece| piece.curve = curve),
                    ..base
                };
                let (head, seat) = seat_of(&params, kind);
                // Where it leaves the skull, read in the ear's own frame, against the
                // blade's width and its section.
                let line = seat.line();
                let last = (line.len() - 1) as f32;
                let leaves = line
                    .iter()
                    .enumerate()
                    .find(|(_, at)| head.distance(**at) > 0.0)
                    .map_or(0, |(station, _)| station);
                let local = head.ear.frame.inverse() * (line[leaves] - head.ear.base);
                let beside = (local.x.abs() - 0.5 * head.ear.width).max(0.0);
                let off = Vec2::new(beside, local.z).length();
                let section = seat.radius_at(leaves as f32 / last);
                assert!(
                    off > section * 0.97,
                    "{name} {kind:?} at {curve}: {off} off an ear with a {section} section"
                );
                // The pair never meets across the middle.
                assert!(seat.base.x >= seat.radius, "{name} {kind:?}");
                // Rooted in the skull.
                assert!(head.distance(seat.base) < 0.0, "{name} {kind:?} floats");
            }
        }
    }

    #[test]
    fn an_antler_ends_in_as_many_points_as_it_says_and_they_fork_as_wide() {
        let params = |points: u32, fork: f32| {
            let mut ornaments = wearing(OrnamentKind::Antlers);
            let antlers = ornaments.antlers.as_mut().expect("worn");
            antlers.points = points;
            antlers.fork = fork;
            QuadrupedParams {
                ornaments,
                whiskers: -1.0,
                ..QuadrupedParams::default()
            }
        };
        let faces = |points: u32| {
            let (_, worn) = adorned(-1.0, params(points, 0.5).ornaments);
            worn.on_head.face_count()
        };
        // A shell a side for each tine, the beam's own the same however many.
        let tine = faces(2) - faces(1);
        assert!(tine > 0, "a second point drew nothing");
        for points in 1..=8 {
            assert_eq!(
                faces(points),
                faces(1) + tine * (points as usize - 1),
                "{points} points"
            );
        }
        // How far its points reach off the beam's line, at either end of the fork.
        let spread = |fork: f32| {
            let params = params(8, fork);
            let (head, seat) = seat_of(&params, OrnamentKind::Antlers);
            let worn = Adornments::build(&head, head.origin);
            let line: Vec<Vec3> = seat.line().iter().map(|&at| head.turn * at).collect();
            worn.on_head
                .positions
                .iter()
                .filter(|at| at.x > 0.0)
                .map(|&at| {
                    line.iter()
                        .map(|on_line| on_line.distance(at))
                        .fold(f32::MAX, f32::min)
                })
                .fold(0.0, f32::max)
                / head.radius
        };
        assert!(
            spread(1.0) > 1.5 * spread(0.0),
            "{} against {}",
            spread(1.0),
            spread(0.0)
        );
    }

    #[test]
    fn a_preset_pair_keeps_its_whole_length_out_of_the_ears_as_built() {
        for (name, base) in heads() {
            for kind in [OrnamentKind::PairedHorns, OrnamentKind::Antlers] {
                let params = QuadrupedParams {
                    ornaments: wearing(kind),
                    ..base
                };
                let avatar = built(&params);
                let face = avatar.parts.beast.as_ref().expect("a beast");
                let head = avatar.rig.joints[face.ears.head].position;
                let (beast, seat) = seat_of(&params, kind);
                let line = seat.line();
                let last = (line.len() - 1) as f32;
                let ear = &face.ears.left;
                for (station, &at) in line.iter().enumerate() {
                    let at = beast.turn * at + beast.origin - head;
                    let round = seat.radius_at(station as f32 / last);
                    let nearest = ear
                        .mesh
                        .positions
                        .iter()
                        .map(|&on_ear| (on_ear + ear.base).distance(at))
                        .fold(f32::MAX, f32::min);
                    assert!(
                        nearest > round * 0.97,
                        "{name} {kind:?}: station {station} is {nearest} off the ear with a \
                         {round} section"
                    );
                }
            }
        }
    }

    /// The jaw of `params`, built on the real surface, and what the head wears.
    fn built(params: &QuadrupedParams) -> crate::Avatar {
        let record = crate::AvatarRecord::new("Ornate", crate::Archetype::Quadruped(*params));
        crate::Avatar::build_with(
            &record,
            &crate::AvatarConfig {
                atlas: 64,
                ..crate::AvatarConfig::default()
            },
        )
        .expect("builds")
    }

    #[test]
    fn an_upper_fang_clears_the_shut_jaw_it_hangs_outside() {
        let shapes: [&dyn Fn(&mut Ornament); 5] = [
            &|_| {},
            &|piece| piece.length = 2.5,
            &|piece| {
                piece.length = 2.5;
                piece.curve = 0.3;
            },
            &|piece| {
                piece.length = 1.5;
                piece.curve = -0.2;
                piece.thickness = 0.14;
            },
            &|piece| {
                piece.length = 1.5;
                piece.thickness = 0.4;
            },
        ];
        for (name, base) in heads() {
            for (index, shape) in shapes.iter().enumerate() {
                let params = QuadrupedParams {
                    ornaments: shaped(OrnamentKind::UpperFangs, shape),
                    whiskers: -1.0,
                    ..base
                };
                let avatar = built(&params);
                let face = avatar.parts.beast.as_ref().expect("a beast");
                let fangs = &face.adornments.on_head;
                assert!(fangs.face_count() > 0);
                let (low, high) = face.jaw.mesh.bounds();
                let inside = fangs
                    .positions
                    .iter()
                    .map(|&at| at - face.jaw.hinge)
                    .filter(|&at| {
                        at.cmpge(low).all() && at.cmple(high).all() && face.jaw.mesh.contains(at)
                    })
                    .count();
                assert_eq!(
                    inside, 0,
                    "{name} shape {index}: {inside} vertices in the jaw"
                );
                // And the root is in the lip rather than beside it.
                let (head, seat) = seat_of(&params, OrnamentKind::UpperFangs);
                // A fang hangs. Its lean is what is left over from clearing the jaw, and
                // one rooted inside the jaw leans as far as it is let and clears nothing.
                assert!(
                    seat.direction.y < -0.6,
                    "{name} shape {index}: a fang splayed to {}",
                    seat.direction
                );
                assert!(
                    head.distance(seat.base) < 0.0,
                    "{name} shape {index}: a fang hangs from nothing"
                );
            }
        }
    }

    #[test]
    fn a_lower_piece_clears_the_upper_lip() {
        for (name, base) in heads() {
            for kind in [OrnamentKind::LowerFangs, OrnamentKind::Tusks] {
                let params = QuadrupedParams {
                    ornaments: wearing(kind),
                    ..base
                };
                let (head, seat) = seat_of(&params, kind);
                let line = seat.line();
                let last = (line.len() - 1) as f32;
                let out = |station: usize| {
                    let round = seat.radius_at(station as f32 / last);
                    [seat.side, -seat.side, Vec3::Y, Vec3::NEG_Y, Vec3::ZERO]
                        .into_iter()
                        .all(|off| head.distance(line[station] + off * round) > 0.0)
                };
                let first = (0..line.len())
                    .position(out)
                    .unwrap_or_else(|| panic!("{name} {kind:?} is inside the head throughout"));
                assert!(
                    first * 2 <= line.len(),
                    "{name} {kind:?} takes {first} stations of {} to leave the lip",
                    line.len()
                );
                for station in first..line.len() {
                    assert!(
                        out(station),
                        "{name} {kind:?}: station {station} went back in"
                    );
                }
                // And it does rise, past the lip's line, when it is long enough to.
                if kind == OrnamentKind::Tusks {
                    let tip = line.last().expect("a line");
                    assert!(tip.y > seat.base.y, "{name}: a tusk that does not rise");
                }
            }
        }
    }

    /// The moved vertices of a posed draw: how many, against the body at rest.
    fn moved(avatar: &crate::Avatar, pose: &crate::anim::Pose) -> usize {
        let eye = |meshes: Vec<crate::AvatarMesh>| {
            meshes
                .into_iter()
                .find(|mesh| mesh.kind == crate::MeshKind::Eye)
                .expect("an eyes' draw")
                .mesh
        };
        let rest = eye(avatar.posed(&crate::anim::Pose::rest(&avatar.rig), 0.0));
        let posed = eye(avatar.posed(pose, 0.0));
        rest.positions
            .iter()
            .zip(&posed.positions)
            .filter(|(one, other)| one.distance(**other) > 1.0e-4)
            .count()
    }

    #[test]
    fn what_rides_the_jaw_opens_with_it_and_what_rides_the_head_does_not() {
        use crate::plan::Family;
        for family in Family::ALL {
            let mut base = QuadrupedParams::default();
            base.set_kind(family, None, 0.0);
            base.whiskers = -1.0;
            for kind in OrnamentKind::ALL {
                let avatar = built(&QuadrupedParams {
                    ornaments: wearing(kind),
                    ..base
                });
                let face = avatar.parts.beast.as_ref().expect("a beast");
                let mut open = crate::anim::Pose::rest(&avatar.rig);
                face.jaw.pose(&mut open, 1.0, 0.0);
                let opened = moved(&avatar, &open);
                let worn = face.adornments.on_jaw.vertex_count();
                if kind.rides_the_jaw() {
                    assert!(worn > 0, "{family:?} {kind:?}");
                    assert!(
                        opened >= worn,
                        "{family:?} {kind:?}: {opened} of {worn} moved"
                    );
                } else {
                    assert_eq!(opened, 0, "{family:?} {kind:?} moved with the jaw");
                }
                // And everything goes with the head through a turn of it, the piece
                // as rigidly as the eyes: a tip the same way from an eye as at rest.
                let mut turned = crate::anim::Pose::rest(&avatar.rig);
                let occiput = avatar.rig.in_zone(crate::plan::Zone::Head)[0];
                turned.rotations[occiput] = glam::Quat::from_rotation_y(0.5);
                let eye_vertices = avatar
                    .parts
                    .eyes
                    .as_ref()
                    .map_or(0, |eyes| eyes.left.globe.vertex_count());
                assert!(
                    moved(&avatar, &turned) > eye_vertices + worn,
                    "{family:?} {kind:?} stayed behind a turning head"
                );
            }
        }
    }

    #[test]
    fn every_envelope_corner_builds_finite_and_closed() {
        for (name, base) in heads().into_iter().step_by(3) {
            for (length, thickness, taper, curve, twist, flutes) in [
                (4.0, 0.5, 0.98, 1.5, 4.0, 0.5),
                (4.0, 0.5, 0.0, -1.5, -4.0, 0.5),
                (0.05, 0.01, 0.98, -1.5, 4.0, 0.0),
                (0.05, 0.5, 0.0, 1.5, 0.0, 0.0),
            ] {
                let shape = Ornament {
                    length,
                    thickness,
                    taper,
                    curve,
                    twist,
                    flutes,
                    base: [0.0; 3],
                    tip: [1.0; 3],
                };
                for points in [1, 8] {
                    for fork in [0.0, 1.0] {
                        let ornaments = Ornaments {
                            horn: Some(shape),
                            paired_horns: Some(shape),
                            upper_fangs: Some(shape),
                            lower_fangs: Some(shape),
                            tusks: Some(shape),
                            antlers: Some(crate::plan::Antlers {
                                beam: shape,
                                points,
                                fork,
                            }),
                        };
                        let params = QuadrupedParams { ornaments, ..base };
                        let head = params
                            .skeleton(&Composites::default())
                            .beast_head
                            .expect("a head");
                        let worn = Adornments::build(&head, head.origin);
                        for mesh in [&worn.on_head, &worn.on_jaw] {
                            assert!(mesh.is_closed_manifold(), "{name}");
                            assert!(mesh.positions.iter().all(|at| at.is_finite()), "{name}");
                            assert!(
                                mesh.vertex_normals().iter().all(|n| n.is_finite()),
                                "{name}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// What each piece costs, and a ceiling on it that `docs/budget.md` reckons with.
    #[test]
    fn a_piece_costs_little_against_the_budget() {
        let triangles = |mesh: &PolyMesh| {
            mesh.faces
                .iter()
                .map(|face| face.len().saturating_sub(2))
                .sum::<usize>()
        };
        for (kind, shape) in [
            (OrnamentKind::Horn, (0.0, 0)),
            (OrnamentKind::Horn, (1.5, 0)),
            (OrnamentKind::PairedHorns, (0.14, 0)),
            (OrnamentKind::PairedHorns, (-1.5, 0)),
            (OrnamentKind::UpperFangs, (0.05, 0)),
            (OrnamentKind::UpperFangs, (1.5, 0)),
            (OrnamentKind::LowerFangs, (0.1, 0)),
            (OrnamentKind::LowerFangs, (1.5, 0)),
            (OrnamentKind::Tusks, (0.2, 0)),
            (OrnamentKind::Tusks, (1.5, 0)),
            (OrnamentKind::Antlers, (0.14, 5)),
            (OrnamentKind::Antlers, (1.5, 8)),
        ] {
            let mut ornaments = shaped(kind, |piece| piece.curve = shape.0);
            if let (true, Some(antlers)) = (shape.1 > 0, &mut ornaments.antlers) {
                antlers.points = shape.1;
            }
            let (_, worn) = adorned(-1.0, ornaments);
            let cost = triangles(&worn.on_head) + triangles(&worn.on_jaw);
            if kind == OrnamentKind::Horn && shape.0 == 0.0 {
                assert_eq!(cost, UNICORN_TRIANGLES);
                assert_eq!(worn.triangles_past_a_unicorn(), 0);
            }
            println!("{kind:?} at curve {}: {cost} triangles", shape.0);
            assert!(cost <= PIECE_MOST, "{kind:?} at {}: {cost}", shape.0);
        }
    }

    /// The dearest any one kind can be: a stag's most tines on its tightest curl.
    const PIECE_MOST: usize = 1_600;
}
