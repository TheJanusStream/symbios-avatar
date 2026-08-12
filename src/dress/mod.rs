//! Clothing.
//!
//! Currently the close-fitting kind: garments cut from the body's own surface,
//! which is what [`garment`] is about and why it is worth doing that way. Loose
//! clothing — anything that hangs rather than clings, a skirt or a coat — is a
//! different construction and is not here yet.
//!
//! An outfit is a small ordered set of garments, each with its own cut. Cuts are
//! **named** rather than continuous. A sleeve is short or long, not 0.62 of the
//! way down the arm: real clothing comes in cuts, and a slider between them
//! would spend most of its range on hems that land mid-forearm and look like a
//! mistake.

pub mod garment;

use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::plan::{Limb, Zone, ZoneSet};
use crate::rig::SkinWeights;

pub use garment::{Garment, GarmentCut, dye};

/// How far down the arm a top's sleeves run.
///
/// Named for where the hem actually lands, which was worth measuring rather
/// than assuming. A cut follows the body's zones, and an arm carries only two of
/// them, so there is no cut that stops at the elbow: the shorter of these ends
/// about 70% of the way down the arm and the longer at 93%. Calling the first
/// one "short" would have been a lie in the record and in the lexicon.
///
/// An open union, like every other token in these records: a cut this build has
/// never heard of is kept as [`Sleeve::Other`] and worn as the default, rather
/// than failing the whole avatar. See [`Sleeve::cut`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sleeve {
    /// No sleeve at all.
    Bare,
    /// To the middle of the forearm.
    #[default]
    Forearm,
    /// To the wrist.
    Wrist,
    /// A cut added after this build, kept verbatim.
    #[serde(untagged)]
    Other(String),
}

impl Sleeve {
    /// The cut to actually wear: this one, or the default if it is unknown.
    #[must_use]
    pub fn cut(&self) -> Sleeve {
        match self {
            Sleeve::Other(_) => Sleeve::default(),
            known => known.clone(),
        }
    }
}

/// How far down the leg a pair of trousers runs.
///
/// As with [`Sleeve`], named for where the hem lands. The middle cut finishes
/// below the knee rather than at it, and an unknown cut is worn as the default.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Leg {
    /// To just below the hip.
    Shorts,
    /// To the calf.
    Calf,
    /// To the ankle.
    #[default]
    Ankle,
    /// A cut added after this build, kept verbatim.
    #[serde(untagged)]
    Other(String),
}

impl Leg {
    /// The cut to actually wear: this one, or the default if it is unknown.
    #[must_use]
    pub fn cut(&self) -> Leg {
        match self {
            Leg::Other(_) => Leg::default(),
            known => known.clone(),
        }
    }
}

/// What a body is wearing.
///
/// Not `Copy`: a cut may be an unrecognised token this build is preserving, and
/// preserving it means owning the string it came in as.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OutfitParams {
    /// How far the top's sleeves run.
    pub sleeve: Sleeve,
    /// How far the trousers run.
    pub leg: Leg,
    /// The top's colour around the wheel.
    #[serde(with = "crate::plan::scaled")]
    pub top_hue: f32,
    /// How light the top is.
    #[serde(with = "crate::plan::scaled")]
    pub top_shade: f32,
    /// The trousers' colour around the wheel.
    #[serde(with = "crate::plan::scaled")]
    pub leg_hue: f32,
    /// How light the trousers are.
    #[serde(with = "crate::plan::scaled")]
    pub leg_shade: f32,
}

impl Default for OutfitParams {
    fn default() -> Self {
        Self {
            sleeve: Sleeve::default(),
            leg: Leg::default(),
            // Far enough apart on the wheel to read as two garments. Neighbouring
            // hues at similar lightness come out as one bodysuit.
            top_hue: 0.04,
            top_shade: 0.62,
            leg_hue: 0.61,
            leg_shade: 0.20,
        }
    }
}

impl OutfitParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        for axis in [
            &mut self.top_hue,
            &mut self.top_shade,
            &mut self.leg_hue,
            &mut self.leg_shade,
        ] {
            // Infinities clamp like any other out-of-range value; only a NaN
            // has no position on the axis at all and needs replacing.
            *axis = quantize(if axis.is_nan() {
                0.5
            } else {
                axis.clamp(0.0, 1.0)
            });
        }
    }
}

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
        weights: &SkinWeights,
        zones: &[Zone],
        params: &OutfitParams,
    ) -> Self {
        let mut garments = Vec::with_capacity(2);

        let trousers = GarmentCut {
            zones: leg_zones(&params.leg),
            // Up into the abdomen, so the waist seam belongs to exactly one
            // garment. The faces that straddle it are wholly inside neither the
            // top's zones nor the trousers', and without this neither takes them
            // — leaving a ring of bare skin at the waist.
            reach: ZoneSet::default().with(Zone::Abdomen),
            ..Default::default()
        };
        let top = GarmentCut {
            zones: top_zones(&params.sleeve),
            ..Default::default()
        };

        // Each garment's claim is smoothed with the other's held out of reach,
        // so a filled notch can never hand one face to both of them. The
        // trousers see the top's raw claim; the top sees the trousers' filled
        // one, which by then is final.
        let trousers_raw = garment::claimed(mesh, zones, &trousers);
        let top_raw = garment::claimed(mesh, zones, &top);
        let mut trousers_faces = trousers_raw.clone();
        garment::close(mesh, &mut trousers_faces, &top_raw);
        let mut top_faces = top_raw;
        garment::close(mesh, &mut top_faces, &trousers_faces);

        if let Some(worn) = Garment::sew(
            mesh,
            weights,
            &trousers_faces,
            &trousers,
            dye(params.leg_hue, params.leg_shade),
        ) {
            garments.push(worn);
        }
        if let Some(worn) = Garment::sew(
            mesh,
            weights,
            &top_faces,
            &top,
            dye(params.top_hue, params.top_shade),
        ) {
            garments.push(worn);
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
    /// half-seen and has to be drawn. About a sixth of the claim (#117).
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

/// Which zones a top covers.
///
/// An unrecognised cut is worn as the default one — see [`Sleeve::cut`] — so a
/// record written by a newer build dresses rather than going bare.
fn top_zones(sleeve: &Sleeve) -> ZoneSet {
    let mut zones = ZoneSet::default().with(Zone::Chest).with(Zone::Abdomen);
    for limb in [Limb::ForeLeft, Limb::ForeRight] {
        match sleeve.cut() {
            Sleeve::Forearm => zones = zones.with(Zone::UpperLimb(limb)),
            Sleeve::Wrist => {
                zones = zones
                    .with(Zone::UpperLimb(limb))
                    .with(Zone::LowerLimb(limb));
            }
            Sleeve::Bare | Sleeve::Other(_) => {}
        }
    }
    zones
}

/// Which zones trousers cover.
///
/// As with [`top_zones`], an unrecognised cut falls back to the default.
fn leg_zones(leg: &Leg) -> ZoneSet {
    let mut zones = ZoneSet::default().with(Zone::Pelvis);
    for limb in [Limb::HindLeft, Limb::HindRight] {
        match leg.cut() {
            Leg::Calf => zones = zones.with(Zone::UpperLimb(limb)),
            Leg::Ankle => {
                zones = zones
                    .with(Zone::UpperLimb(limb))
                    .with(Zone::LowerLimb(limb));
            }
            Leg::Shorts | Leg::Other(_) => {}
        }
    }
    zones
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::{Rig, SkinConfig, skin};
    use crate::{Archetype, AvatarRecord, CageConfig, build_cage, catmull_clark};
    use glam::Vec3;

    fn body(seed: i64) -> (PolyMesh, SkinWeights, Vec<Zone>) {
        let mut record = AvatarRecord::new("Worn", Archetype::default());
        record.reroll(seed);
        let skeleton = record.skeleton();
        let cage = build_cage(&skeleton, &CageConfig::default()).expect("meshes");
        let mesh = catmull_clark(&cage, crate::BODY_SUBDIVISIONS);
        let rig = Rig::from_skeleton(&skeleton).expect("rigs");
        let weights = skin::bind(&mesh, &rig, &SkinConfig::default());
        let zones = weights.zone_map(&mesh, &rig);
        (mesh, weights, zones)
    }

    #[test]
    fn a_body_can_be_dressed() {
        let (mesh, weights, zones) = body(1);
        let outfit = Outfit::wear(&mesh, &weights, &zones, &OutfitParams::default());
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
        for seed in [1i64, 9] {
            let (mesh, weights, zones) = body(seed);
            let normals = mesh.vertex_normals();
            for (sleeve, leg) in [(Sleeve::Bare, Leg::Shorts), (Sleeve::Forearm, Leg::Ankle)] {
                let params = OutfitParams {
                    sleeve,
                    leg,
                    ..Default::default()
                };
                let outfit = Outfit::wear(&mesh, &weights, &zones, &params);
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
                            "seed {seed}: face {face} is not drawn and nothing covers it"
                        );
                    }
                }
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
            let (mesh, weights, zones) = body(seed);
            let outfit = Outfit::wear(&mesh, &weights, &zones, &OutfitParams::default());
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
        for seed in 1i64..=12 {
            let (mesh, weights, zones) = body(seed);
            for sleeve in [Sleeve::Bare, Sleeve::Forearm, Sleeve::Wrist] {
                for leg in [Leg::Shorts, Leg::Calf, Leg::Ankle] {
                    let params = OutfitParams {
                        sleeve: sleeve.clone(),
                        leg: leg.clone(),
                        ..Default::default()
                    };
                    let outfit = Outfit::wear(&mesh, &weights, &zones, &params);
                    for garment in &outfit.garments {
                        assert!(
                            garment.mesh.is_closed_manifold(),
                            "seed {seed} {sleeve:?}/{leg:?}: {:?}",
                            garment.mesh.manifold_report()
                        );
                    }
                }
            }
        }
    }

    /// A seed whose shorts pinch, which is what the guard below needs.
    ///
    /// Named rather than inlined because it has had to move twice — see that
    /// test. Any body whose waist ring touches itself will do; twenty of the
    /// first forty seeds did when this was last swept.
    const PINCHING: i64 = 1;

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
        let (mesh, weights, zones) = body(PINCHING);
        let params = OutfitParams {
            sleeve: Sleeve::Bare,
            leg: Leg::Shorts,
            ..Default::default()
        };
        let outfit = Outfit::wear(&mesh, &weights, &zones, &params);
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
        let (mesh, weights, zones) = body(23);
        let worn = |sleeve, leg| {
            let params = OutfitParams {
                sleeve,
                leg,
                ..Default::default()
            };
            Outfit::wear(&mesh, &weights, &zones, &params)
                .garments
                .iter()
                .map(Garment::vertex_count)
                .sum::<usize>()
        };
        assert!(worn(Sleeve::Forearm, Leg::Ankle) > worn(Sleeve::Bare, Leg::Ankle));
        assert!(worn(Sleeve::Wrist, Leg::Ankle) > worn(Sleeve::Forearm, Leg::Ankle));
        assert!(worn(Sleeve::Wrist, Leg::Calf) > worn(Sleeve::Wrist, Leg::Shorts));
        assert!(worn(Sleeve::Wrist, Leg::Ankle) > worn(Sleeve::Wrist, Leg::Calf));
    }

    #[test]
    fn the_waist_seam_belongs_to_exactly_one_garment() {
        // Two garments meeting have to agree on the ring of faces between them.
        // Left to themselves neither takes it, and a band of bare skin shows.
        let (mesh, weights, zones) = body(3);
        let outfit = Outfit::wear(&mesh, &weights, &zones, &OutfitParams::default());

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
        let (mesh, _weights, zones) = body(11);
        let params = OutfitParams::default();
        let trousers = GarmentCut {
            zones: leg_zones(&params.leg),
            reach: ZoneSet::default().with(Zone::Abdomen),
            ..Default::default()
        };
        let top = GarmentCut {
            zones: top_zones(&params.sleeve),
            ..Default::default()
        };
        let claimed = |cut: &GarmentCut| -> Vec<usize> {
            mesh.faces
                .iter()
                .enumerate()
                .filter(|(_, face)| {
                    face.iter().all(|&c| {
                        cut.zones.contains(zones[c as usize])
                            || cut.reach.contains(zones[c as usize])
                    }) && face.iter().any(|&c| cut.zones.contains(zones[c as usize]))
                })
                .map(|(index, _)| index)
                .collect()
        };
        let below = claimed(&trousers);
        let above = claimed(&top);
        assert!(!below.is_empty() && !above.is_empty());
        assert!(
            below.iter().all(|face| !above.contains(face)),
            "a face was claimed by both garments"
        );
    }

    #[test]
    fn clothing_is_reproducible() {
        let (mesh, weights, zones) = body(13);
        let params = OutfitParams::default();
        assert_eq!(
            Outfit::wear(&mesh, &weights, &zones, &params),
            Outfit::wear(&mesh, &weights, &zones, &params)
        );
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = OutfitParams {
            top_hue: 9.0,
            top_shade: -3.0,
            leg_hue: f32::NAN,
            leg_shade: f32::INFINITY,
            ..Default::default()
        };
        params.sanitize();
        assert_eq!(params.top_hue, 1.0);
        assert_eq!(params.top_shade, 0.0);
        assert_eq!(params.leg_hue, 0.5);
        assert_eq!(params.leg_shade, 1.0);

        let once = params.clone();
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn an_outfit_survives_a_round_trip_through_json() {
        let params = OutfitParams::default();
        let text = serde_json::to_string(&params).expect("serialises");
        assert_eq!(
            params,
            serde_json::from_str::<OutfitParams>(&text).expect("deserialises")
        );
        // Cuts are named, so they travel as names rather than as magic numbers.
        assert!(text.contains("forearm"), "{text}");
        assert!(text.contains("ankle"), "{text}");
    }

    #[test]
    fn every_body_can_be_dressed() {
        for seed in [1, 5, 17, 42, 99] {
            let (mesh, weights, zones) = body(seed);
            let outfit = Outfit::wear(&mesh, &weights, &zones, &OutfitParams::default());
            assert_eq!(outfit.len(), 2, "seed {seed} could not be dressed");
        }
    }
}
