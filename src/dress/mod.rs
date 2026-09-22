//! Clothing.
//!
//! Currently the close-fitting kind: garments cut from the body's own surface,
//! which is what [`garment`] is about and why it is worth doing that way. Loose
//! clothing — anything that hangs rather than clings, a skirt or a coat — is a
//! different construction and is not here yet.
//!
//! An outfit is a top and a pair of trousers, each with an explicit colour, a
//! length and optionally a texture ([`params`]). A length is continuous: a
//! sleeve runs any share of the way from the shoulder to the wrist and a
//! trouser leg from high on the thigh to the ankle, the hem a ring square to
//! the limb wherever it lands. A texture is one of `symbios-texture`'s
//! tileable surfaces ([`surface`]), baked with the rest of the outfit into one
//! cloth atlas ([`cloth`]), so a textured outfit still costs one draw.

pub mod cloth;
pub mod garment;
pub mod params;
pub(crate) mod roll;
pub mod surface;

use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone, ZoneSet};
use crate::rig::{Rig, SkinWeights};

pub use garment::{Chain, Garment, GarmentCut, charted_faces, dye};
pub use params::{CALF, FOREARM, GarmentParams, OutfitParams, SHORTS, SLEEVE_RANGE, TROUSER_RANGE};
pub use surface::{GarmentTexture, SurfaceConfig};

/// Everything a body is wearing, outermost last.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outfit {
    /// The pieces, in the order they should be drawn.
    pub garments: Vec<Garment>,
}

impl Outfit {
    /// Cuts an outfit for a body.
    ///
    /// `zones` is the per-vertex zone map — the same one the unwrap uses.
    #[must_use]
    pub fn wear(
        mesh: &PolyMesh,
        rig: &Rig,
        weights: &SkinWeights,
        zones: &[Zone],
        params: &OutfitParams,
    ) -> Self {
        let mut garments = Vec::with_capacity(2);

        let trousers = trouser_cut(params.trousers.length);
        let top = top_cut(params.top.length);

        // Each garment's claim is smoothed with the other's held out of reach,
        // so a filled notch can never hand one face to both of them. The
        // trousers see the top's raw claim; the top sees the trousers' filled
        // one, which by then is final.
        let trousers_raw = garment::claimed(mesh, rig, zones, &trousers);
        let top_raw = garment::claimed(mesh, rig, zones, &top);
        let mut trousers_faces = trousers_raw.clone();
        garment::close(mesh, &mut trousers_faces, &top_raw);
        let mut top_faces = top_raw;
        garment::close(mesh, &mut top_faces, &trousers_faces);

        for (faces, cut, worn) in [
            (&trousers_faces, &trousers, &params.trousers),
            (&top_faces, &top, &params.top),
        ] {
            if let Some(mut garment) =
                Garment::sew(mesh, rig, weights, zones, faces, cut, worn.colour)
            {
                garment.texture.clone_from(&worn.texture);
                garments.push(garment);
            }
        }

        Self { garments }
    }

    /// Every garment as one mesh.
    ///
    /// For tools that want the whole outfit in one piece. Not a manifold: two
    /// garments are two solids.
    #[must_use]
    pub fn mesh(&self) -> PolyMesh {
        let mut mesh = PolyMesh::new();
        for garment in &self.garments {
            mesh.append(&garment.mesh);
        }
        mesh
    }

    /// Which body faces the outfit hides, one flag per face of the body.
    ///
    /// The union of every garment's [`hidden`](Garment::hidden), and the reason
    /// a dressed body draws less skin than a bare one: cloth stands over those
    /// faces in every pose, so emitting them is paying for geometry no camera
    /// can reach. `faces` is the body's face count, because an outfit knows
    /// what it claimed and not how big the body was.
    ///
    /// `hidden` rather than [`claim`](Garment::claim), and the difference is
    /// the row of faces the hem itself runs through: the hem is smoothed off
    /// the face boundaries it was cut along, so a face it crosses may end up
    /// half-seen and has to be drawn. About a sixth of the claim.
    #[must_use]
    pub fn covered(&self, faces: usize) -> Vec<bool> {
        let mut hidden = vec![false; faces];
        for garment in &self.garments {
            for &face in &garment.hidden {
                if let Some(flag) = hidden.get_mut(face as usize) {
                    *flag = true;
                }
            }
        }
        hidden
    }

    /// Whether any garment wears a texture this build can draw — whether a
    /// build paints a cloth atlas at all.
    #[must_use]
    pub fn is_textured(&self) -> bool {
        self.garments.iter().any(|garment| {
            garment
                .texture
                .as_ref()
                .is_some_and(|texture| texture.surface.is_known())
        })
    }

    /// How many pieces are being worn.
    #[must_use]
    pub fn len(&self) -> usize {
        self.garments.len()
    }

    /// Whether the body is wearing nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.garments.is_empty()
    }
}

/// How a top is cut: the chest and the abdomen, and each arm `length` of the
/// way down — none of it at all at `0`, the whole of both arm zones at `1`.
fn top_cut(length: f32) -> GarmentCut {
    GarmentCut {
        zones: ZoneSet::default().with(Zone::Chest).with(Zone::Abdomen),
        limbs: limbs([Limb::ForeLeft, Limb::ForeRight], length),
        ..Default::default()
    }
}

/// How trousers are cut: the pelvis, and each leg `length` of the way down.
fn trouser_cut(length: f32) -> GarmentCut {
    GarmentCut {
        zones: ZoneSet::default().with(Zone::Pelvis),
        limbs: limbs([Limb::HindLeft, Limb::HindRight], length),
        // Up into the abdomen, so the waist seam belongs to exactly one
        // garment. The faces that straddle it are wholly inside neither the
        // top's zones nor the trousers', and without this neither takes them
        // — leaving a ring of bare skin at the waist.
        reach: ZoneSet::default().with(Zone::Abdomen),
        ..Default::default()
    }
}

/// A pair of limbs run `length` of the way down, or neither at a length of
/// nothing.
fn limbs(pair: [Limb; 2], length: f32) -> [Option<(Limb, f32)>; 2] {
    pair.map(|limb| (length > 0.0).then_some((limb, length)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};
    use glam::Vec3;

    fn body(seed: i64) -> (PolyMesh, Rig, SkinWeights, Vec<Zone>) {
        let mut record = AvatarRecord::new("Worn", Archetype::default());
        record.reroll(seed);
        body_of(&record)
    }

    fn body_of(record: &AvatarRecord) -> (PolyMesh, Rig, SkinWeights, Vec<Zone>) {
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        (mesh, rig, weights, zones)
    }

    /// An outfit at the given lengths, in the default colours.
    fn cut(sleeve: f32, leg: f32) -> OutfitParams {
        let mut params = OutfitParams::default();
        params.top.length = sleeve;
        params.trousers.length = leg;
        params
    }

    /// The lengths the old named cuts read as, sleeve then leg: the corners
    /// every sweep here covers, and the middles between them.
    const SLEEVES: [f32; 5] = [0.0, 0.25, 0.5, FOREARM, 1.0];
    const LEGS: [f32; 5] = [SHORTS, 0.35, 0.5, CALF, 1.0];

    #[test]
    fn a_body_can_be_dressed() {
        let (mesh, rig, weights, zones) = body(1);
        let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &OutfitParams::default());
        assert_eq!(outfit.len(), 2, "a top and trousers");
        assert!(!outfit.is_empty());
        assert!(outfit.mesh().face_count() > 100);
    }

    /// Whether any triangle of `cloth` stands over `from` within `reach`.
    ///
    /// Möller–Trumbore, once per triangle, with the triangle list built by the
    /// caller — `PolyMesh::triangulated` allocates, and a ray cast that
    /// re-triangulates the garment for every face it asks about measures the
    /// allocator.
    fn under_cloth(cloth: &[[Vec3; 3]], from: Vec3, along: Vec3, reach: f32) -> bool {
        cloth.iter().any(|&[a, b, c]| {
            let (e1, e2) = (b - a, c - a);
            let across = along.cross(e2);
            let det = e1.dot(across);
            if det.abs() < 1e-12 {
                return false;
            }
            let inv = 1.0 / det;
            let to = from - a;
            let u = to.dot(across) * inv;
            if !(-1e-6..=1.000_001).contains(&u) {
                return false;
            }
            let up = to.cross(e1);
            let v = along.dot(up) * inv;
            if v < -1e-6 || u + v > 1.000_001 {
                return false;
            }
            let at = e2.dot(up) * inv;
            at > 1e-5 && at <= reach
        })
    }

    #[test]
    fn every_scrap_of_suppressed_skin_has_cloth_standing_over_it() {
        // The safety argument for not drawing the skin under a garment, asked
        // rather than reasoned about: a ray leaving each suppressed face along
        // its own normal — the direction the skin faces, and so the direction
        // anything seeing it would have to come from — must hit the garment.
        //
        // **Not `contains`, and that is a measured correction rather than a
        // preference.** The obvious test is that every corner of every hidden
        // face lies inside the garment solid, and it fails on 24 to 40 corners
        // per body, all of them in the crotch: an inward offset of 1.5 mm in a
        // concavity that tight self-intersects, so the solid is tangled there
        // and `contains` reports points that are 1.5 mm from cloth as outside
        // it. The skin is not visible — it is under both the cloth and the far
        // thigh — and a test that says otherwise is measuring the offset's
        // degeneracy, not the garment's coverage (`docs/instruments.md` rule 1).
        //
        // Swept over lengths as well as the two old extremes (#356): a hem
        // mid-limb is the case the old named cuts never produced.
        for seed in [1i64, 9] {
            let (mesh, rig, weights, zones) = body(seed);
            let normals = mesh.vertex_normals();
            for (sleeve, leg) in [(0.0, SHORTS), (FOREARM, 1.0), (0.35, 0.6)] {
                let params = cut(sleeve, leg);
                let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &params);
                assert!(
                    outfit
                        .covered(mesh.face_count())
                        .iter()
                        .any(|&hidden| hidden),
                    "seed {seed}: a dressed body hid nothing at all"
                );
                for garment in &outfit.garments {
                    // A garment always has a hem, so it always gives its row
                    // back — and a garment can give back ALL of it: a pair of
                    // shorts is two rings of faces wide, every one of them is
                    // on a hem, so it hides nothing and its hem cannot move.
                    // That is the clamp working, not a failure.
                    assert!(garment.hidden.len() < garment.claim.len());
                    let cloth: Vec<[Vec3; 3]> = garment
                        .mesh
                        .triangulated()
                        .iter()
                        .map(|corners| corners.map(|c| garment.mesh.positions[c as usize]))
                        .collect();
                    for &face in &garment.hidden {
                        let out = mesh.faces[face as usize]
                            .iter()
                            .map(|&corner| normals[corner as usize])
                            .fold(Vec3::ZERO, |sum, normal| sum + normal)
                            .normalize();
                        let from = mesh.face_centroid(face as usize) + out * 1e-4;
                        assert!(
                            under_cloth(&cloth, from, out, 0.05),
                            "seed {seed} {sleeve}/{leg}: face {face} is not drawn and nothing covers it"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn no_garment_stands_inside_the_body_it_was_cut_from() {
        // The premise of the whole module, asked instead of assumed: a garment
        // cannot intersect the body because every point of it is a body point
        // pushed outward. At the crotch it did (#279) — 4 to 14 outer columns
        // per body, 1.2 to 8.0 mm inside the skin, on every seed of
        // `garmentaudit`'s sweep and on the default body. Eight millimetres is
        // the whole thickness, so the worst of them were offset backwards.
        //
        // **The OUTER shell only, and the reading is worthless without that.**
        // The inner shell is inside the body on purpose, so measured over the
        // whole cloth mesh this reads about half the vertices "inside" on a
        // body with nothing wrong with it. The inner twin of a column is that
        // column plus half the vertex count.
        //
        // **Against an undressed build.** `Outfit::wear` is given the body's
        // own mesh here, which still carries the faces the clothes cover; a
        // dressed body does not emit the skin under its clothes, so a ray cast
        // at one goes through the hole and comes back with a confident wrong
        // parity.
        //
        // The seeds are the three worst of the sweep, plus both ends of the two
        // composite axes that reshape a torso — the extreme chest is where the
        // top's own two vertices showed up, and it is not reachable by rerolling.
        let mut extremes = Vec::new();
        for (femininity, mass, fat) in [(1.0, 1.0, 0.60f32), (-1.0, 1.0, 0.60), (1.0, -1.0, 0.03)] {
            let mut record = AvatarRecord::new("Extreme", Archetype::default());
            record.composites.femininity = femininity;
            record.composites.mass = mass;
            record.composites.body_fat = fat;
            record.sanitize();
            extremes.push(record);
        }
        let mut rolled = Vec::new();
        for seed in [1i64, 9, 12] {
            let mut record = AvatarRecord::new("Worn", Archetype::default());
            record.reroll(seed);
            rolled.push(record);
        }

        for record in rolled.iter().chain(&extremes) {
            let (mesh, rig, weights, zones) = body_of(record);
            let params = cut(0.0, 1.0);
            let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &params);
            for (index, garment) in outfit.garments.iter().enumerate() {
                let half = garment.mesh.positions.len() / 2;
                let inside: Vec<usize> = (0..half)
                    .filter(|&column| mesh.contains(garment.mesh.positions[column]))
                    .collect();
                assert!(
                    inside.is_empty(),
                    "{}: garment {index} has {} outer columns inside the body, first at {:?}",
                    record.name,
                    inside.len(),
                    garment.mesh.positions[inside[0]]
                );
            }
        }
    }

    #[test]
    fn the_waist_seam_stays_shut_when_the_hems_are_smoothed() {
        // The one thing sliding a hem could break that nothing else guards. The
        // top's lower hem and the trousers' upper one are the SAME ring of body
        // edges, and each garment smooths its own copy of it without consulting
        // the other; if the two copies land anywhere different, a sliver of skin
        // opens between two garments that used to meet exactly.
        //
        // They agree because the operator is a function of the ring alone — the
        // same vertices in the same order, walked the other way, which a
        // symmetric filter and a symmetric clamp cannot tell apart. That is an
        // argument, and this is the measurement.
        for seed in [1i64, 5, 9] {
            let (mesh, rig, weights, zones) = body(seed);
            let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &OutfitParams::default());
            let hems: Vec<std::collections::HashMap<u32, Vec<Vec3>>> = outfit
                .garments
                .iter()
                .map(|garment| {
                    let mut at: std::collections::HashMap<u32, Vec<Vec3>> =
                        std::collections::HashMap::new();
                    for ring in &garment.hem {
                        for &vertex in ring {
                            at.entry(garment.source[vertex as usize])
                                .or_default()
                                .push(garment.mesh.positions[vertex as usize]);
                        }
                    }
                    at
                })
                .collect();

            let mut shared = 0;
            let mut worst = 0.0f32;
            for (from, here) in &hems[0] {
                let Some(there) = hems[1].get(from) else {
                    continue;
                };
                shared += 1;
                for point in here {
                    let apart = there
                        .iter()
                        .map(|other| other.distance(*point))
                        .fold(f32::MAX, f32::min);
                    worst = worst.max(apart);
                }
            }
            assert!(
                shared > 8,
                "seed {seed}: the two garments shared {shared} hem vertices, so this proved nothing"
            );
            assert!(
                worst < 1e-5,
                "seed {seed}: {shared} shared hem vertices, worst {} mm apart",
                worst * 1000.0
            );
        }
    }

    #[test]
    fn every_garment_is_a_closed_solid() {
        // **Swept over seeds, because one body cannot see this defect** (#105).
        // It used to ask seed 7 alone, on the reasoning that the failure was a
        // bare sleeve's hem cutting through the arm-to-torso saddle. It was not:
        // measured under the eight-point cage of #107 the same failure appears
        // on ten of twelve seeds, identically at all three sleeve lengths, and
        // in the TROUSERS rather than the top — the boundary running round the
        // top of a pair of shorts touches itself, and every vertex where it does
        // put four rim quads on one edge. A sleeve-shaped hypothesis tested on a
        // single body is how it stayed filed as a sleeve bug.
        //
        // Swept over lengths since #356: the old three cuts per garment, and a
        // hem in the middle of each segment, which is new ground for the fan
        // split — a ring cut mid-limb is a boundary no zone ever drew.
        for seed in 1i64..=12 {
            let (mesh, rig, weights, zones) = body(seed);
            for sleeve in SLEEVES {
                for leg in LEGS {
                    let params = cut(sleeve, leg);
                    let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &params);
                    for garment in &outfit.garments {
                        assert!(
                            garment.mesh.is_closed_manifold(),
                            "seed {seed} {sleeve}/{leg}: {:?}",
                            garment.mesh.manifold_report()
                        );
                    }
                }
            }
        }
    }

    /// A seed whose hem pinches, which is what the guard below needs.
    ///
    /// Named rather than inlined because it has had to move three times — see
    /// that test. Any body whose waist ring touches itself will do. Twenty of
    /// the first forty seeds pinched when this was last swept for #164, and
    /// sixteen of those were the SHORTS' hem running through the crotch; #314
    /// moved that hem down the thigh and the crotch pinch with it, which left
    /// the waist-ring pinch on seeds 17, 24, 36 and 39 — two vertices each, on
    /// every outfit alike.
    const PINCHING: i64 = 17;

    #[test]
    fn a_pinched_hem_is_cut_into_separate_columns() {
        // The mechanism behind the test above, asserted directly so that one
        // cannot go on passing for a reason other than the fix. `Garment::cut`
        // gives each run of covered faces at a vertex its own garment vertex, so
        // where the boundary touches itself the same body vertex appears in
        // `source` more than once. If that stops happening the split has been
        // undone and `every_garment_is_a_closed_solid` is passing on a body that
        // happens not to pinch.
        //
        // Seed 7's shorts were the case #105 was filed on: six pinch vertices in
        // the abdomen, all on one cage ring at the waist. **Seed 7 stopped
        // pinching when the frame axis reached the trunk** (#100) — it rolls a
        // femininity that narrows its waist, and the pinch is a waist ring
        // touching itself. Nothing about the split changed.
        //
        // **And moved again for #164**, which retired `build` and rebuilt every
        // girth from the allometry, so the population moved under it a second
        // time. That is twice in two issues, which is the argument for the
        // constant below: the seed is named once, at the top, so the next body
        // change is a one-line edit rather than a hunt.
        //
        // Moved to seed 0 rather than re-tuned, and the population is the
        // reason that is safe: twenty of the first forty seeds pinch here, so
        // this guard is easy to satisfy and hard to lose by accident. What it
        // is NOT is a guard tied to one body — which is what it was, and what
        // made a change three subsystems away read as a regression here.
        let (mesh, rig, weights, zones) = body(PINCHING);
        let params = cut(0.0, SHORTS);
        let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &params);
        let split: usize = outfit
            .garments
            .iter()
            .map(|garment| {
                // `source` lists the body vertex behind each garment vertex,
                // outer shell then inner, so a body vertex carrying one column
                // appears exactly twice.
                let mut seen = std::collections::HashMap::<u32, usize>::new();
                for &from in &garment.source {
                    *seen.entry(from).or_default() += 1;
                }
                seen.values().filter(|&&times| times > 2).count()
            })
            .sum();
        assert!(
            split > 0,
            "no body vertex was cut into more than one garment column, so the \
             pinch this guards against is no longer being reached"
        );
    }

    #[test]
    fn longer_cuts_cover_more() {
        let (mesh, rig, weights, zones) = body(23);
        let worn = |sleeve, leg| {
            Outfit::wear(&mesh, &rig, &weights, &zones, &cut(sleeve, leg))
                .garments
                .iter()
                .map(Garment::vertex_count)
                .sum::<usize>()
        };
        assert!(worn(FOREARM, 1.0) > worn(0.0, 1.0));
        assert!(worn(1.0, 1.0) > worn(FOREARM, 1.0));
        assert!(worn(1.0, CALF) > worn(1.0, SHORTS));
        assert!(worn(1.0, 1.0) > worn(1.0, CALF));
    }

    /// Where along `chain` a garment's hem lies nearest `want`: the mean
    /// position of the hem ring whose columns sit nearest that point of the
    /// limb, and how far round the ring the position strays either way.
    fn hem_at(garment: &Garment, chain: &Chain, want: f32) -> (f32, f32) {
        let goal = chain.point(want);
        garment
            .hem
            .iter()
            .map(|ring| {
                let points: Vec<Vec3> = ring
                    .iter()
                    .map(|&column| garment.mesh.positions[column as usize])
                    .collect();
                let middle = points.iter().copied().sum::<Vec3>() / points.len() as f32;
                let along: Vec<f32> = points.iter().map(|&p| chain.position(p)).collect();
                let mean = along.iter().sum::<f32>() / along.len() as f32;
                let spread = along
                    .iter()
                    .map(|at| (at - mean).abs())
                    .fold(0.0f32, f32::max);
                (middle.distance(goal), mean, spread)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, mean, spread)| (mean, spread))
            .expect("a garment has a hem")
    }

    #[test]
    fn a_hem_lands_where_its_length_says() {
        // What "any length" has to mean (#356): the DELIVERED hem — after the
        // claim, the smoothing and the placement — sits at the length along
        // the limb, on every length, and moves as the length does. The claim
        // alone cannot say it: a limb carries about four rows of faces per
        // segment, so a hem left on the rings the claim cut along sat at one
        // of a handful of places and a length moved it in jumps of a sixth of
        // the limb (measured over 400 lengths, before the placement).
        for seed in [1i64, 9] {
            let (mesh, rig, weights, zones) = body(seed);
            let arm = Chain::of(&rig, Limb::ForeLeft).expect("an arm");
            let leg = Chain::of(&rig, Limb::HindLeft).expect("a leg");
            let mut last = (0.0f32, 0.0f32);
            for step in 1..20 {
                let length = step as f32 / 20.0;
                let (sleeve, trouser) = (length, length.max(SHORTS));
                let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &cut(sleeve, trouser));
                let [trousers, top] = &outfit.garments[..] else {
                    panic!("seed {seed} at {length}: a top and trousers");
                };
                let (on_arm, arm_spread) = hem_at(top, &arm, sleeve);
                let (on_leg, leg_spread) = hem_at(trousers, &leg, trouser);
                for (what, want, got, spread) in [
                    ("sleeve", sleeve, on_arm, arm_spread),
                    ("trouser", trouser, on_leg, leg_spread),
                ] {
                    assert!(
                        (got - want).abs() < 0.03,
                        "seed {seed}: a {what} of {want:.3} delivered its hem at {got:.3} (spread {spread:.3})"
                    );
                }
                assert!(
                    on_arm >= last.0 - 1e-3 && on_leg >= last.1 - 1e-3,
                    "seed {seed} at {length}: a hem moved back up the limb"
                );
                last = (on_arm, on_leg);
            }
        }
    }

    #[test]
    fn the_whole_length_takes_both_limb_zones_as_the_old_long_cuts_did() {
        // A share of one is the old wrist and ankle cut exactly: both of the
        // limb's zones, whole. What a record written as "wrist" reads as.
        let (mesh, rig, _, zones) = body(3);
        let whole = GarmentCut {
            zones: ZoneSet::default()
                .with(Zone::Chest)
                .with(Zone::Abdomen)
                .with(Zone::UpperLimb(Limb::ForeLeft))
                .with(Zone::LowerLimb(Limb::ForeLeft))
                .with(Zone::UpperLimb(Limb::ForeRight))
                .with(Zone::LowerLimb(Limb::ForeRight)),
            ..Default::default()
        };
        assert_eq!(
            garment::claimed(&mesh, &rig, &zones, &top_cut(1.0)),
            garment::claimed(&mesh, &rig, &zones, &whole)
        );
        // And nothing of the arm at all is the old bare cut.
        let bare = GarmentCut {
            zones: ZoneSet::default().with(Zone::Chest).with(Zone::Abdomen),
            ..Default::default()
        };
        assert_eq!(
            garment::claimed(&mesh, &rig, &zones, &top_cut(0.0)),
            garment::claimed(&mesh, &rig, &zones, &bare)
        );
    }

    #[test]
    fn the_waist_seam_belongs_to_exactly_one_garment() {
        // Two garments meeting have to agree on the ring of faces between them.
        // Left to themselves neither takes it, and a band of bare skin shows.
        let (mesh, rig, weights, zones) = body(3);
        let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &OutfitParams::default());

        let waist: Vec<usize> = (0..mesh.positions.len())
            .filter(|&v| matches!(zones[v], Zone::Abdomen | Zone::Pelvis))
            .collect();
        assert!(!waist.is_empty());

        // Every face straddling the abdomen and the pelvis must be covered.
        let straddling: Vec<&Vec<u32>> = mesh
            .faces
            .iter()
            .filter(|face| {
                face.iter().any(|&c| zones[c as usize] == Zone::Abdomen)
                    && face.iter().any(|&c| zones[c as usize] == Zone::Pelvis)
            })
            .collect();
        assert!(!straddling.is_empty(), "no face straddles the waist");

        // A covered face's centroid is within a garment's thickness of it.
        let dressed = outfit.mesh();
        for face in straddling {
            let middle = face
                .iter()
                .map(|&c| mesh.positions[c as usize])
                .fold(Vec3::ZERO, |sum, p| sum + p)
                / face.len() as f32;
            let nearest = dressed
                .positions
                .iter()
                .map(|p| p.distance(middle))
                .fold(f32::MAX, f32::min);
            assert!(
                nearest < 0.05,
                "a waist face at {middle:?} had no garment within {nearest}"
            );
        }
    }

    #[test]
    fn the_top_and_the_trousers_do_not_claim_the_same_face() {
        // Overlapping garments flicker against each other. The reach that closes
        // the waist must hand the seam to one of them, not to both.
        let (mesh, rig, _weights, zones) = body(11);
        let params = OutfitParams::default();
        // The real claim, not a re-statement of it: #314's first cut at
        // rescuing hand-zoned hip skin let the top claim down the hips and
        // the trousers up the belly by bone radius alone, interleaved at the
        // waist, and a copy of the old zone rule here would not have seen it.
        let claimed = |cut: &GarmentCut| -> Vec<usize> {
            garment::claimed(&mesh, &rig, &zones, cut)
                .iter()
                .enumerate()
                .filter_map(|(index, &mine)| mine.then_some(index))
                .collect()
        };
        let below = claimed(&trouser_cut(params.trousers.length));
        let above = claimed(&top_cut(params.top.length));
        assert!(!below.is_empty() && !above.is_empty());
        assert!(
            below.iter().all(|face| !above.contains(face)),
            "a face was claimed by both garments"
        );
    }

    #[test]
    fn clothing_is_reproducible() {
        let (mesh, rig, weights, zones) = body(13);
        let params = OutfitParams::default();
        assert_eq!(
            Outfit::wear(&mesh, &rig, &weights, &zones, &params),
            Outfit::wear(&mesh, &rig, &weights, &zones, &params)
        );
    }

    #[test]
    fn each_garment_wears_its_own_colour_and_texture() {
        let (mesh, rig, weights, zones) = body(2);
        let mut params = OutfitParams::default();
        params.top.colour = [0.9, 0.1, 0.1];
        params.trousers.colour = [0.1, 0.1, 0.9];
        params.trousers.texture = Some(GarmentTexture::new(
            SurfaceConfig::named("Fabric").expect("a surface"),
        ));
        let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &params);
        let [trousers, top] = &outfit.garments[..] else {
            panic!("a top and trousers");
        };
        assert_eq!(top.colour, [0.9, 0.1, 0.1]);
        assert_eq!(trousers.colour, [0.1, 0.1, 0.9]);
        assert!(top.texture.is_none());
        assert!(trousers.texture.is_some());
        assert!(outfit.is_textured());
        assert_eq!(trousers.limbs, vec![Limb::HindLeft, Limb::HindRight]);
    }

    #[test]
    fn every_body_can_be_dressed() {
        for seed in [1, 5, 17, 42, 99] {
            let (mesh, rig, weights, zones) = body(seed);
            let outfit = Outfit::wear(&mesh, &rig, &weights, &zones, &OutfitParams::default());
            assert_eq!(outfit.len(), 2, "seed {seed} could not be dressed");
        }
    }
}
