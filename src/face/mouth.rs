//! A mouth that opens (#154).
//!
//! The mandible region (#152) gave the lower face a bone and the talk driver
//! (#153) gave that bone speech, but the skin over both was one continuous
//! sheet: opening the jaw stretched the face between the lips rather than
//! parting them. This cuts the parting.
//!
//! **The body stays one closed solid.** The surface is split along the lip
//! parting and sewn back together through a shallow pocket — inner lips, an
//! upper teeth ridge, a back wall — so the mouth is a concave fold of the same
//! watertight surface, not a hole. Every contract that stands on the closed
//! manifold (containment, garments, the meshability sweeps) survives by
//! construction. An attached mouth solid was rejected before this existed:
//! #59 measured what a solid laid on bending skin costs, and it is why the
//! lips are relief rather than parts.
//!
//! **The parting is the relief's own line.** The cut follows the same curved
//! groove `relief` carves — the mouth line dipping toward the corners by
//! `0.0292` of the frame per half-width squared, over the same
//! record-controlled width — so the seam lands inside the carved lips on every
//! body, by sharing the arithmetic rather than by hoping two copies agree.
//!
//! **Cut, not snapped.** The seam is made by splitting every mesh edge the
//! parting curve crosses, which keeps it smooth on any topology; snapping to
//! an existing vertex path was considered and rejected because the surface
//! under the mouth is hull-derived and a snapped path wanders the cell size.
//! The two outermost crossings become **welds** — single vertices where upper
//! and lower seam meet — so the mouth's corners stretch under a jaw open the
//! way a mouth's corners do, instead of tearing.

use glam::Vec3;
use std::collections::HashMap;

use super::canon::Canon;
use super::features::FaceParams;
use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::rig::Rig;
use crate::uv::{Rect, UvUnwrap};

/// How deep the pocket reaches, in head radii.
///
/// Shallow by the owner's word: enough that an open mouth shows an interior
/// rather than a void, and nothing a viewer never sees. On the default head
/// this is about 13 mm.
const DEPTH: f32 = 0.14;

/// Where the slit ends, as a share of the relief's mouth half-width.
///
/// Inside the vermilion's own end (the lobes vanish at 1.0, the groove runs to
/// 1.05), so the seam is always under carved lip rather than bare cheek.
const CORNERS: f32 = 0.92;

/// The vertices the surgery made, by what should hold them.
///
/// The seam chains run left to right and index `positions` of the mesh they
/// were cut into. `upper` and `lower` are coincident at rest — the closed
/// mouth — and part when the jaw pivot turns away from the skull.
#[derive(Clone, Debug, PartialEq)]
pub struct Mouth {
    /// The parting's upper edge, held by the skull.
    pub upper: Vec<u32>,
    /// Its lower edge, held by the jaw; `lower[i]` starts coincident with
    /// `upper[i]`.
    pub lower: Vec<u32>,
    /// The pocket's roof — the inner upper lip: the skull's.
    pub roof: Vec<u32>,
    /// The teeth ridge hanging from the roof: the skull's, painted apart.
    pub teeth: Vec<u32>,
    /// Pocket floor and the whole back wall: the jaw's.
    ///
    /// The back wall's TOP was the skull's first, and the owner caught what
    /// that means from outside (#156): open the jaw and the wall stands still
    /// while everything around it drops — a stationary sheet of mouth exactly
    /// where the inner lower lip should be, reading as the lip stuck on the
    /// skull. A mouth's floor and inner gum ride the mandible; the stretch a
    /// split territory needs happens in the one row hidden BEHIND the teeth
    /// ridge, where nobody has ever seen the inside of a mouth.
    pub floor: Vec<u32>,
    /// The outer skin of the lower lip, below the parting: the jaw's
    /// outright (#155, deepened by #157).
    ///
    /// The mandible field's blend has no business here: inside the slit there
    /// is no continuous skin across the parting any more, so the ramp that
    /// eases a cheek eased the lip tip instead — the seam's own edge moved
    /// with the jaw while the skin a millimetre below it hung on the skull,
    /// which the owner saw as the lower lip stuck mid-word. And the band is
    /// DEEP, because the field's top edge is a span-fraction constant while
    /// the cut follows each body's own canon: on seed 1 the two disagree by
    /// six millimetres, and the skin between them hung half-held across a
    /// wide-open mouth as a stretched sheet (#157). Where the field is
    /// already 1.0 the override changes nothing, so depth is cheap; the cut
    /// line is the one authority.
    pub lip: Vec<u32>,
    /// The outer skin just above the parting: the skull's outright (#157).
    ///
    /// The same seed-mismatch mirrored: a body whose parting sits below the
    /// field's top would have upper-lip skin claimed for the jaw. Skin above
    /// the cut is the skull's by definition, whatever the field says.
    pub overlip: Vec<u32>,
    /// The two corner vertices where the seams meet, left then right.
    ///
    /// Deliberately in neither camp: they keep the blended weights the field
    /// gave them, which is what lets a corner stretch instead of tear.
    pub welds: [u32; 2],
}

/// Cuts the parting into `body` and sews the pocket behind it.
///
/// Returns `None` — leaving the body exactly as it was — when the rig carries
/// no jaw markers (a quadruped), when the parting curve crosses no clean chain
/// of edges, or when the cut would be degenerate. A body without an openable
/// mouth is the state every body shipped in until #154, not an error.
#[must_use]
pub fn open(body: &mut PolyMesh, rig: &Rig, canon: &Canon, params: &FaceParams) -> Option<Mouth> {
    // No jaw, no mouth to open: the seam's two sides would have nothing to
    // part them.
    (0..rig.len()).find(|&tip| {
        rig.joints[tip].marker
            && rig.joints[tip]
                .parent
                .is_some_and(|pivot| rig.joints[pivot].marker)
    })?;
    let head = *rig.in_zone(Zone::Head).first()?;
    let centre = rig.joints[head].position;
    let radius = rig.joints[head].radius;

    // The relief's own arithmetic, shared rather than copied by value: the
    // mouth line dips toward the corners, and the slit has to follow the
    // carved groove or it surfaces on bare skin.
    let half = canon.unit * (0.6829 + 0.2376 * params.mouth_width);
    let mouth_y = centre.y + canon.mouth_line();
    let reach = half * CORNERS;
    let above = |p: Vec3| -> f32 {
        let across = (p.x - centre.x) / half;
        p.y - (mouth_y - canon.frame * 0.0292 * across * across)
    };

    // ---- Find the crossings --------------------------------------------
    //
    // Every edge the parting curve crosses, both endpoints inside the slit's
    // width and forward of the head's axis. The outermost crossings terminate
    // the chain, so the bound on the ENDPOINTS is what shapes the corner.
    let eligible = |v: u32| -> bool {
        let p = body.positions[v as usize];
        (p.x - centre.x).abs() <= reach && p.z > centre.z && (p.y - mouth_y).abs() < radius * 0.35
    };
    let mut crossings: HashMap<(u32, u32), f32> = HashMap::new();
    for face in &body.faces {
        for at in 0..face.len() {
            let (a, b) = (face[at], face[(at + 1) % face.len()]);
            let key = if a < b { (a, b) } else { (b, a) };
            if crossings.contains_key(&key) || !eligible(a) || !eligible(b) {
                continue;
            }
            let (sa, sb) = (
                above(body.positions[a as usize]),
                above(body.positions[b as usize]),
            );
            if sa * sb < 0.0 {
                crossings.insert(key, sa / (sa - sb));
            }
        }
    }
    if crossings.len() < 6 {
        return None;
    }

    // ---- Order them into one chain --------------------------------------
    //
    // Two crossings on one face are neighbours. A clean transversal cut gives
    // every face at most two, the chain a simple path, and anything else —
    // a wiggle the curve enters twice, an island — is a body this pass
    // declines to cut rather than cuts badly.
    let mut on_face: Vec<(usize, Vec<(u32, u32)>)> = Vec::new();
    for (index, face) in body.faces.iter().enumerate() {
        let mut mine: Vec<(u32, u32)> = Vec::new();
        for at in 0..face.len() {
            let (a, b) = (face[at], face[(at + 1) % face.len()]);
            let key = if a < b { (a, b) } else { (b, a) };
            if crossings.contains_key(&key) {
                mine.push(key);
            }
        }
        match mine.len() {
            0 => {}
            1 | 2 => on_face.push((index, mine)),
            _ => return None,
        }
    }
    let mut neighbours: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    let mut degree_one = 0usize;
    for (_, mine) in &on_face {
        if let [a, b] = mine.as_slice() {
            neighbours.entry(*a).or_default().push(*b);
            neighbours.entry(*b).or_default().push(*a);
        } else {
            degree_one += 1;
        }
    }
    // A simple open chain has exactly the two pentagon faces beyond its welds.
    if degree_one != 2 {
        return None;
    }
    let start = *crossings
        .keys()
        .find(|key| neighbours.get(key).map_or(0, Vec::len) == 1)?;
    let mut chain: Vec<(u32, u32)> = vec![start];
    let mut previous: Option<(u32, u32)> = None;
    loop {
        let here = *chain.last()?;
        let next = neighbours
            .get(&here)?
            .iter()
            .find(|&&key| Some(key) != previous)
            .copied();
        match next {
            Some(key) => {
                previous = Some(here);
                chain.push(key);
            }
            None => break,
        }
    }
    if chain.len() != crossings.len() || chain.len() < 6 {
        return None;
    }
    // Left to right, so the classes mean the same thing on every body.
    let position_of = |key: &(u32, u32)| -> Vec3 {
        let t = crossings[key];
        body.positions[key.0 as usize].lerp(body.positions[key.1 as usize], t)
    };
    if position_of(chain.first()?).x > position_of(chain.last()?).x {
        chain.reverse();
    }

    // ---- Split the crossed edges -----------------------------------------
    //
    // One new vertex per crossing; every face is rebuilt with the crossings
    // inserted into its cycle, and the faces the chain passes through are
    // divided at theirs.
    let placed: Vec<Vec3> = chain.iter().map(position_of).collect();
    let mut split_at: HashMap<(u32, u32), u32> = HashMap::new();
    for (key, at) in chain.iter().zip(placed) {
        split_at.insert(*key, body.push_vertex(at));
    }
    let cut = |face: &[u32]| -> Vec<u32> {
        let mut cycle = Vec::with_capacity(face.len() + 2);
        for at in 0..face.len() {
            let (a, b) = (face[at], face[(at + 1) % face.len()]);
            cycle.push(a);
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&inserted) = split_at.get(&key) {
                cycle.push(inserted);
            }
        }
        cycle
    };
    let mut divided: Vec<Vec<u32>> = Vec::new();
    // The faces that lie below the parting, recorded AT the division — where
    // each half's side is a fact of the walk — rather than re-derived from
    // positions afterwards. A positional test was built first and
    // misclassified at the corners, where the curve dips and a lower half's
    // far vertices sit above it: the half kept the upper seam, the hole never
    // opened there, and the skin hung off the moving lip as a stretched
    // spike.
    let split_base = body.positions.len() - chain.len();
    let mut below_faces: Vec<usize> = Vec::new();
    for (index, mine) in on_face.iter().rev() {
        let cycle = cut(&body.faces[*index]);
        if mine.len() == 1 {
            // The face beyond a weld: it gains the weld vertex and stays whole.
            body.faces[*index] = cycle;
            continue;
        }
        // Divided: walk the cycle, switching sides at each inserted vertex.
        let (first, second) = (split_at[&mine[0]], split_at[&mine[1]]);
        let mut sides: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
        let mut side = 0usize;
        for &vertex in &cycle {
            sides[side].push(vertex);
            if vertex == first || vertex == second {
                side = 1 - side;
                sides[side].push(vertex);
            }
        }
        // The cycle always starts on an original vertex — `cut` pushes the
        // original before any insertion — so the walk closes both arcs
        // complete: each crossing vertex was pushed onto both sides at the
        // switch, and the wrap-around edge belongs to the side the walk began
        // and ended on. Traced for a start above the parting and a start
        // below; both land whole.
        //
        // Which arc is the lower one is read off the arc's own original
        // vertices: an arc's originals are all on one side of the parting by
        // construction of the walk.
        let lower_side = |arc: &[u32]| -> bool {
            arc.iter()
                .find(|&&vertex| (vertex as usize) < split_base)
                .is_some_and(|&vertex| above(body.positions[vertex as usize]) < 0.0)
        };
        if lower_side(&sides[0]) {
            below_faces.push(*index);
        }
        if lower_side(&sides[1]) {
            below_faces.push(body.faces.len() + divided.len());
        }
        body.faces[*index] = sides[0].clone();
        divided.push(sides[1].clone());
    }
    body.faces.extend(divided);

    // ---- Part the seam ---------------------------------------------------
    //
    // The interior chain vertices are doubled; faces below the parting take
    // the copies. The welds stay single, which is the whole of how a corner
    // stretches rather than tears.
    let welds = [split_at[&chain[0]], split_at[chain.last()?]];
    let upper: Vec<u32> = chain[1..chain.len() - 1]
        .iter()
        .map(|key| split_at[key])
        .collect();
    // The lower edge sits a fraction of a millimetre behind the upper, the
    // way a lower lip sits behind the one over it. Exact coincidence was
    // built first and z-fought: the seam's own pixels flickered between lip
    // and pocket, a dark speckled line across a mouth that is supposed to
    // read shut. The recess is the owner's "closed, need not be identical"
    // clause spent where it buys something.
    let lower: Vec<u32> = upper
        .iter()
        .map(|&u| {
            let at = body.positions[u as usize];
            let inward = Vec3::new(centre.x - at.x, 0.0, centre.z - at.z).normalize_or_zero();
            body.push_vertex(at + inward * (0.005 * radius) - Vec3::Y * (0.002 * radius))
        })
        .collect();
    let twin: HashMap<u32, u32> = upper.iter().copied().zip(lower.iter().copied()).collect();
    for &index in &below_faces {
        for vertex in &mut body.faces[index] {
            if let Some(&copy) = twin.get(vertex) {
                *vertex = copy;
            }
        }
    }

    // ---- The lip tip band ------------------------------------------------
    //
    // The outer skin of the lower lip within a few millimetres of the
    // parting. **Walked from the lower seam, not selected by position**: the
    // parting curve dips toward the corners and the upper lip's carved bulge
    // dips with it, so a positional band caught upper-lip vertices sitting
    // locally below the curve — overridden to the jaw, they hung off the
    // opening as stretched spikes when it moved. The seam is a topological
    // fact: skin reachable from the lower edge without passing a weld is
    // lower lip, whatever its millimetres say. Collected here, after the seam
    // is doubled and before the pocket exists, so the walk sees exactly the
    // original skin.
    let flood = |seeds: &[u32], blocked: &[u32], banded: &dyn Fn(u32) -> bool| -> Vec<u32> {
        let mut adjacent: HashMap<u32, Vec<u32>> = HashMap::new();
        for face in &body.faces {
            for at in 0..face.len() {
                let (a, b) = (face[at], face[(at + 1) % face.len()]);
                if banded(a) || banded(b) {
                    adjacent.entry(a).or_default().push(b);
                    adjacent.entry(b).or_default().push(a);
                }
            }
        }
        let barrier: std::collections::HashSet<u32> = blocked.iter().copied().collect();
        let mut seen: std::collections::HashSet<u32> = seeds.iter().copied().collect();
        let mut queue: Vec<u32> = seeds.to_vec();
        let mut band = Vec::new();
        while let Some(here) = queue.pop() {
            let Some(next) = adjacent.get(&here) else {
                continue;
            };
            for &vertex in next {
                if seen.contains(&vertex) || barrier.contains(&vertex) || !banded(vertex) {
                    continue;
                }
                seen.insert(vertex);
                band.push(vertex);
                queue.push(vertex);
            }
        }
        band
    };
    // Deep on the lower side, because the mandible field's top edge is a
    // span-fraction constant while this cut follows the body's own canon:
    // on seed 1 the two disagree by six millimetres, and the skin between
    // them hung half-held across a wide-open mouth as a stretched sheet
    // (#157). Where the field already answers 1.0 the override is a no-op,
    // so depth costs nothing; the cut line is the one authority. The upper
    // side is the same mismatch mirrored: skin above the cut is the skull's
    // by definition, whatever the field says.
    let above_seam: Vec<u32> = upper.iter().chain(&welds).copied().collect();
    let below_seam: Vec<u32> = lower.iter().chain(&welds).copied().collect();
    let lip = flood(&lower, &above_seam, &|vertex: u32| {
        let p = body.positions[vertex as usize];
        let under = -above(p);
        (p.x - centre.x).abs() <= reach + 0.02 * radius
            && p.z > centre.z
            && under > -0.002 * radius
            && under < 0.10 * radius
    });
    let overlip = flood(&above_seam, &below_seam, &|vertex: u32| {
        let p = body.positions[vertex as usize];
        let over = above(p);
        (p.x - centre.x).abs() <= reach + 0.02 * radius
            && p.z > centre.z
            && over > -0.002 * radius
            && over < 0.05 * radius
    });

    // ---- Sew the pocket --------------------------------------------------
    //
    // The seam is cut at mesh resolution — every crossing — but the pocket
    // behind it is not built at it: an interior in shadow does not need a
    // section per crossing, and sewing one there put the dearest-hair budget
    // 146 triangles over its WebGL2 target. Sections stand at every third
    // crossing; the fine seam is stitched to the coarse sections by one
    // polygon per span, which the consumers fan-triangulate like any other
    // face. Eight points per section, tapering to nothing at the welds: inner
    // lip roll, the teeth ridge hanging from the roof, the back wall, the
    // floor.
    let up = Vec3::Y;
    let count = upper.len();
    let mut picks: Vec<usize> = (0..count).step_by(3).collect();
    if picks.last() != Some(&(count - 1)) {
        picks.push(count - 1);
    }
    let mut ring_of: Vec<Vec<u32>> = Vec::with_capacity(picks.len());
    for &at in &picks {
        let arc = (at as f32 + 1.0) / (count as f32 + 1.0);
        let taper = (1.0 - (2.0 * arc - 1.0).powi(2)).max(0.0).sqrt();
        let mouth_of = body.positions[upper[at] as usize];
        let inward = {
            let toward = Vec3::new(centre.x - mouth_of.x, 0.0, centre.z - mouth_of.z);
            toward.normalize_or_zero()
        };
        let d = DEPTH * radius * taper;
        let r = radius * taper;
        // The ridge stops well short of the floor: at rest the cavity is a
        // sliver, and a ridge reaching the floor's own depth z-fought it as
        // white specks through the closed seam. An open jaw drops the floor
        // by centimetres, so a short ridge still reads as teeth.
        ring_of.push(vec![
            body.push_vertex(mouth_of + inward * (0.35 * d) + up * (0.015 * r)),
            body.push_vertex(mouth_of + inward * (0.55 * d) + up * (0.012 * r)),
            body.push_vertex(mouth_of + inward * (0.55 * d) - up * (0.010 * r)),
            body.push_vertex(mouth_of + inward * d + up * (0.008 * r)),
            body.push_vertex(mouth_of + inward * d - up * (0.020 * r)),
            body.push_vertex(mouth_of + inward * (0.35 * d) - up * (0.032 * r)),
        ]);
    }

    // The strip's winding comes from the mesh rather than from an assumption:
    // the upper polys use the seam edge in one direction, and a closed solid
    // needs the pocket to use it in the other.
    let forward = {
        let (a, b) = (upper[0], upper[1]);
        body.faces
            .iter()
            .any(|face| (0..face.len()).any(|at| face[at] == a && face[(at + 1) % face.len()] == b))
    };
    let mut sew = |cycle: Vec<u32>| {
        let mut cycle = cycle;
        cycle.dedup();
        while cycle.len() > 1 && cycle.last() == cycle.first() {
            cycle.pop();
        }
        if cycle.len() < 3 {
            return;
        }
        if !forward {
            cycle.reverse();
        }
        body.faces.push(cycle);
    };

    // The roof-side and floor-side spans: the fine seam between two coarse
    // sections, closed by the sections' first (or last) interior points. The
    // welds stand in for a section of their own at each end.
    let span_of = |from: usize, to: usize| -> (Vec<u32>, Vec<u32>) {
        (
            upper[picks[from]..=picks[to]].to_vec(),
            lower[picks[from]..=picks[to]].to_vec(),
        )
    };
    for pair in 0..=picks.len() {
        // Ring endpoints for this span; a weld plays a collapsed ring.
        let (near_ring, far_ring, seam_u, seam_l): (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) =
            if pair == 0 {
                let (mut su, mut sl) = span_of(0, 0);
                su.insert(0, welds[0]);
                sl.insert(0, welds[0]);
                (vec![welds[0]; 6], ring_of[0].clone(), su, sl)
            } else if pair == picks.len() {
                let (mut su, mut sl) = span_of(picks.len() - 1, picks.len() - 1);
                su.push(welds[1]);
                sl.push(welds[1]);
                (ring_of[picks.len() - 1].clone(), vec![welds[1]; 6], su, sl)
            } else {
                let (su, sl) = span_of(pair - 1, pair);
                (ring_of[pair - 1].clone(), ring_of[pair].clone(), su, sl)
            };
        // Roof span: seam upper run, reversed, closed by the two rings' first
        // points; floor span mirrored.
        let mut roof_span: Vec<u32> = seam_u.iter().rev().copied().collect();
        roof_span.push(near_ring[0]);
        roof_span.push(far_ring[0]);
        sew(roof_span);
        let mut floor_span: Vec<u32> = seam_l.clone();
        floor_span.push(far_ring[5]);
        floor_span.push(near_ring[5]);
        sew(floor_span);
        // The interior rows, coarse quads between the rings.
        for row in 0..5 {
            sew(vec![
                far_ring[row],
                near_ring[row],
                near_ring[row + 1],
                far_ring[row + 1],
            ]);
        }
    }

    let mut roof = Vec::new();
    let mut teeth = Vec::new();
    let mut floor = Vec::new();
    for ring in &ring_of {
        roof.push(ring[0]);
        teeth.push(ring[1]);
        teeth.push(ring[2]);
        floor.push(ring[3]);
        floor.push(ring[4]);
        floor.push(ring[5]);
    }

    Some(Mouth {
        upper,
        lower,
        roof,
        teeth,
        floor,
        lip,
        overlip,
        welds,
    })
}
/// Moves the pocket's charts into `rect`, out from under the face's.
///
/// The unwrap charts by zone and connectivity, and the pocket is Head-zone
/// skin connected to the face through its own seams — so it landed in the
/// face's chart, where a projection that flattens a face cannot also flatten
/// a fold hidden behind it: the cavity's texels rasterised over the lip's and
/// the paint ran down the chin as dark drips (#155). The interior gets the
/// same treatment as an attached part instead: one reserved rectangle,
/// asked from the packer like an ear's, with the faces re-charted into it on
/// their own duplicated vertices so the seam's texels stay the lip's.
///
/// The layout inside the rectangle is by construction rather than by
/// projection: `u` is the position across the mouth, `v` the ring station —
/// seam edge to roof to teeth to back wall to floor — which cannot fold
/// because the stations are ordered by the sewing itself.
pub(crate) fn chart_interior(charts: &mut UvUnwrap, mouth: &Mouth, mesh: &PolyMesh, rect: Rect) {
    let mut station: HashMap<u32, f32> = HashMap::new();
    for &vertex in mouth.upper.iter().chain(&mouth.welds) {
        station.insert(vertex, 0.0);
    }
    // The classes hold their rings' vertices in pushed order: the roof is the
    // inner-lip roll alone, teeth the ridge's outer and hanging edge in
    // pairs, the floor the back wall's top and bottom and the floor's front
    // in threes. Position within the class tells the stations apart.
    for &vertex in &mouth.roof {
        station.insert(vertex, 0.22);
    }
    for (at, &vertex) in mouth.teeth.iter().enumerate() {
        station.insert(vertex, if at % 2 == 0 { 0.34 } else { 0.45 });
    }
    for (at, &vertex) in mouth.floor.iter().enumerate() {
        station.insert(
            vertex,
            match at % 3 {
                0 => 0.58,
                1 => 0.70,
                _ => 0.85,
            },
        );
    }
    for &vertex in &mouth.lower {
        station.insert(vertex, 1.0);
    }

    let interior: std::collections::HashSet<u32> = mouth
        .roof
        .iter()
        .chain(&mouth.teeth)
        .chain(&mouth.floor)
        .copied()
        .collect();
    let (mut left, mut right) = (f32::MAX, f32::MIN);
    for &vertex in station.keys() {
        let x = mesh.positions[vertex as usize].x;
        left = left.min(x);
        right = right.max(x);
    }
    let span = (right - left).max(f32::EPSILON);

    for index in 0..charts.faces.len() {
        let pocket = charts.faces[index]
            .iter()
            .any(|&corner| interior.contains(&charts.source[corner as usize]));
        if !pocket {
            continue;
        }
        let face = charts.faces[index].clone();
        let mut recharted = Vec::with_capacity(face.len());
        for corner in face {
            let source = charts.source[corner as usize];
            let p = mesh.positions[source as usize];
            let across = (p.x - left) / span;
            let ring = station.get(&source).copied().unwrap_or(0.5);
            let uv = rect.min + (rect.max - rect.min) * glam::Vec2::new(across, ring);
            charts.source.push(source);
            charts.uvs.push(uv);
            recharted.push(charts.source.len() as u32 - 1);
        }
        charts.faces[index] = recharted;
    }
}
