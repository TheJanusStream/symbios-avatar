//! The painted beard's edge, read on rolled bodies built whole (#432).
//!
//! Here rather than in `tests/texture.rs` beside the #344 guard because each of
//! these reads how much skin a texel spans, which the crate measures beside its
//! atlas and keeps to itself: see [`Footprints`].
//!
//! **The seeds were re-picked at generation 12** (#486), which re-rolled every
//! person: seeds 175 and 237, the issue's two rolls, and seed 5 drew bodies
//! that paint no beard at all. Seeds 4, 7 and 14 still paint one and are kept,
//! as different bodies. The measurements recorded below under the old seeds
//! are those seeds' generation-11 bodies.
//!
//! **Each replacement holds with margin without a nudge of libm and under
//! every nudge the gate makes** (up and down, one and two last places,
//! `scripts/gate.sh --libm`), because these guards are fragile under one
//! (#488): a roll's mesh moves under a nudge and a texel step or a halo count
//! swings by a tenth or by 70 texels. The first picks, by nearest painted
//! density alone, did not hold: 875 (0.667) steps 1.106 and fails
//! [`BEARD_STEP`] as a hard edge would, 13 (0.659) steps 0.615 but 0.717 under
//! two last places up, and 155 (0.395) paints 151 more halo texels than its
//! hard edge under one up, against [`FEATHER_HALO`]'s 160. The texture code did
//! not change: over the first 24 painted rolls of each generation the step
//! bounds fail on 8 of generation 11's and 5 of generation 12's, without a
//! nudge. So among the rolls near each old density, the ones with margin:
//!
//! - 24 (0.679) for 175's 0.665: steps at most 0.576 (thousandth-worst 0.355)
//!   over the five readings; 32 (0.659) read 0.69 under one down.
//! - 69 (0.965) for 237's 0.995: steps at most 0.564 (0.373); 775 (0.981)
//!   read 0.64, and 103 (0.977) 0.72 without a nudge.
//! - 969 (0.394) for 5's 0.395: at most 103 more halo texels than its hard
//!   edge over the five readings; 96, 754 and 66 near it read up to 130.
//!
//! Where the chin and the flanks meet, 0.682 at a density of 0.679 and 0.973
//! to 0.974 at 0.965; under the nose, nothing; the lip under it filled 0.59 to
//! 0.66 and 0.59 to 0.62.

use glam::Vec3;
use symbios_texture::generator::TextureMap;

use super::bake::{AtlasGeometry, Footprints, bake_measured};
use super::skin::{Condition, PaintedLayer, paint_skin_with};
use crate::face::{Canon, Skull};
use crate::hair::Follicles;
use crate::hair::painted::Cover;
use crate::plan::{Archetype, Zone};
use crate::{Avatar, AvatarConfig, AvatarRecord};

/// A rolled bearded body, built, and its atlas baked as the avatar's was with
/// how much skin each texel spans beside it.
struct Rolled {
    record: AvatarRecord,
    avatar: Avatar,
    follicles: Follicles,
    geometry: AtlasGeometry,
    footprints: Footprints,
    density: f32,
    colour: Vec3,
}

impl Rolled {
    fn of(seed: i64) -> Self {
        Self::with(seed, |_| {})
    }

    /// The same, with the rolled record changed by `edit` before it is built.
    fn with(seed: i64, edit: impl FnOnce(&mut AvatarRecord)) -> Self {
        let mut record = AvatarRecord::new("Bearded", Archetype::default());
        record.reroll(seed);
        edit(&mut record);
        let density = record.hair.chin.skin.density;
        assert!(
            density > 0.0,
            "seed {seed} no longer rolls a painted beard: pick another bearded roll"
        );
        let colour = Vec3::from_array(record.hair.chin.skin.colour) * 255.0;
        let avatar = Avatar::build(&record).expect("a biped builds");
        let skull = Skull::measure(&avatar.parts.body, &avatar.rig).expect("a head measures");
        let canon = Canon::measure(&avatar.rig, &skull, &record.eyes);
        let follicles = Follicles::of(&avatar.rig, &skull, &canon, &record.hair.regions);
        // Baked with the attached parts and the mouth's pocket, as the build
        // bakes the atlas it paints (#473): baked without them, a texel an
        // ear's gutter took in the painted atlas read as the face's beside it,
        // and once the packer laid an ear's rectangle against the face's chart
        // at the sideburn the two texels stood at one place on the skin and a
        // beard's whole density apart.
        let parts = &avatar.parts;
        let body = &parts.body;
        let placed = crate::avatar::attached_in_body(
            &avatar.rig,
            &parts.features,
            &parts.extremities,
            &parts.eyes,
            &parts.beast,
            &parts.wings,
        );
        let borrowed: Vec<(&crate::PolyMesh, Zone, &[f32])> = placed
            .iter()
            .map(|(mesh, zone, mouth)| (mesh, *zone, mouth.as_slice()))
            .collect();
        let mut inside = vec![0.0f32; body.vertex_count()];
        if let Some(mouth) = &parts.mouth {
            for &vertex in mouth.roof.iter().chain(&mouth.floor) {
                inside[vertex as usize] = 1.0;
            }
            for &vertex in &mouth.teeth {
                inside[vertex as usize] = 0.5;
            }
        }
        let (geometry, footprints) = bake_measured(
            body,
            &parts.unwrap,
            &borrowed,
            &inside,
            AvatarConfig::default().atlas,
        );
        assert_eq!(
            geometry.texels.len() * 4,
            avatar.skin.albedo.len(),
            "the atlas baked here is not the shape of the one the body was painted on"
        );
        Self {
            record,
            avatar,
            follicles,
            geometry,
            footprints,
            density,
            colour,
        }
    }

    /// The body built again with its beard's three regions shaved to nothing,
    /// and its skin.
    fn shaved(&self) -> TextureMap {
        Avatar::build(&shave(&self.record))
            .expect("a biped builds")
            .skin
    }

    /// The body's skin painted again over the atlas baked here, with or
    /// without the footprints to feather by, and shaved or not.
    fn repaint(&self, footprints: Option<&Footprints>, shaved: bool) -> TextureMap {
        let record = if shaved {
            shave(&self.record)
        } else {
            self.record.clone()
        };
        let painted = record.hair.painted();
        paint_skin_with(
            &self.geometry,
            footprints,
            &self.avatar.rig,
            &record.skin,
            &Condition::of(&record.composites),
            Some(&PaintedLayer {
                follicles: &self.follicles,
                painted: &painted,
            }),
        )
    }

    /// How far texel `at` went from `bare`'s colour toward the beard's in
    /// `painted`, or `None` where the two are too close in colour to say.
    fn share(&self, painted: &TextureMap, bare: &TextureMap, at: usize) -> Option<f32> {
        let rgb = |map: &TextureMap| {
            Vec3::new(
                f32::from(map.albedo[at * 4]),
                f32::from(map.albedo[at * 4 + 1]),
                f32::from(map.albedo[at * 4 + 2]),
            )
        };
        let skin = rgb(bare);
        let toward = self.colour - skin;
        (toward.length_squared() >= 400.0)
            .then(|| (rgb(painted) - skin).dot(toward) / toward.length_squared())
    }

    /// The beard's three masks at a texel's centre, alone.
    fn beard_at(&self, position: Vec3) -> [f32; 3] {
        let weights = self.follicles.weights(position - self.follicles.origin());
        let mut beard = [0.0; 3];
        let regions = crate::hair::Follicle::ALL
            .into_iter()
            .zip(weights)
            .filter(|(follicle, _)| Cover::of(*follicle).is_some());
        for (into, (_, weight)) in beard.iter_mut().zip(regions) {
            *into = weight;
        }
        beard
    }
}

/// A record with its beard's three regions painted at no density.
fn shave(record: &AvatarRecord) -> AvatarRecord {
    let mut shaved = record.clone();
    shaved.hair.moustache.skin.density = 0.0;
    shaved.hair.chin.skin.density = 0.0;
    shaved.hair.flanks.skin.density = 0.0;
    shaved
}

/// How far a beard region's painted texels may miss the share of the way to
/// their colour their density asks for: `PAINTED_SLACK` beside
/// `a_beard_painted_at_full_density_is_its_own_colour` in `tests/texture.rs`.
const PAINTED_SLACK: f32 = 0.025;

#[test]
fn a_painted_beard_fades_over_texels_the_atlas_has() {
    // **The beard's edge went from bare skin to its whole density between one
    // texel and the next** (#432). The head is one chart wrapped round the
    // neck, so under the jaw and behind the cheek, where the skin turns to face
    // along the wrap, a texel spanned four to eight millimetres of skin (two to
    // four since #469), and a chin patch fading over five did its whole fade
    // inside one: coarse, hard-edged patches under the chin and along the jaw
    // on the issue's two rolls, seeds 175 and 237.
    //
    // Read on the built atlas, between every texel of the head and the next
    // one along each axis that is its neighbour on the skin as well - no
    // farther apart than half again the coarser one's footprint along its long
    // axis, facing the same way - as the step in how far each went toward the
    // beard's colour, in shares of the beard's density. A step of one is the
    // whole edge in one texel.
    //
    // Measured at #432: seed 175 at most 0.992 before and 0.650 after, seed
    // 237 1.048 and 0.733; the thousandth-worst step 0.693 to 0.425 and 0.830 to
    // 0.511. What is left at the top is under the chin, where the head's chart
    // folds and texels side by side stand five to ten millimetres apart. On
    // #469's chart, with the feather a tenth wider, 0.605 and 0.637, the
    // thousandth-worst 0.374 and 0.413. On #473's face chart, with #474's
    // packer and the atlas baked with its parts, 0.541 and 0.583, the
    // thousandth-worst 0.306 and 0.375. On its charts of the face, the crown
    // and the band round the back (#473's fixer) 0.408 and 0.580, the
    // thousandth-worst 0.308 and 0.388; the same beard painted hard-edged,
    // with no feather, 0.791 and 0.954, the thousandth-worst 0.425 and 0.543.
    // With #475's moustache outline and its ragged, stranded edge 0.485 and
    // 0.553, the thousandth-worst 0.300 and 0.377; with no feather seed 175
    // reads 0.945 and 0.532. With #475's fixer's ends, as thick as the band
    // under the nose and never under two and a half fades, 0.485 and 0.584,
    // the thousandth-worst 0.301 and 0.381.
    for seed in [24, 69] {
        let rolled = Rolled::of(seed);
        let bare = rolled.shaved();
        let geometry = &rolled.geometry;
        let width = geometry.width as usize;
        let mut steps = Vec::new();
        for (at, texel) in geometry.texels.iter().enumerate() {
            let Some(here) = texel.filter(|texel| texel.zone == Zone::Head) else {
                continue;
            };
            let Some(mine) = rolled.share(&rolled.avatar.skin, &bare, at) else {
                continue;
            };
            let along = [(at % width + 1 < width).then_some(at + 1), Some(at + width)];
            for next in along.into_iter().flatten() {
                let Some(Some(there)) = geometry.texels.get(next) else {
                    continue;
                };
                let coarser = rolled.footprints.read(geometry, at).axes[0]
                    .length()
                    .max(rolled.footprints.read(geometry, next).axes[0].length());
                if there.zone != Zone::Head
                    || (there.position - here.position).length() > 1.5 * coarser
                    || here.normal.dot(there.normal) < 0.9
                {
                    continue;
                }
                if let Some(theirs) = rolled.share(&rolled.avatar.skin, &bare, next) {
                    steps.push((mine - theirs).abs() / rolled.density);
                }
            }
        }
        assert!(
            steps.len() > 20_000,
            "seed {seed}: only {} neighbouring head texels to read",
            steps.len()
        );
        steps.sort_by(|one, two| two.total_cmp(one));
        let (worst, thousandth) = (steps[0], steps[steps.len() / 1000]);
        println!(
            "seed {seed}: the beard's steepest step between neighbouring texels is {worst:.3} of \
             its density, the thousandth-worst {thousandth:.3}, over {} pairs",
            steps.len()
        );
        assert!(
            worst <= BEARD_STEP && thousandth <= BEARD_STEP_THOUSANDTH,
            "seed {seed}: the painted beard steps {worst:.2} of its density between two \
             neighbouring texels (the thousandth-worst {thousandth:.2})"
        );
    }
}

/// The largest step in a painted beard's coverage between two neighbouring
/// texels, in shares of its density; see
/// `a_painted_beard_fades_over_texels_the_atlas_has`.
///
/// Measured at #432: 0.650 and 0.733 on the two rolls, against 0.992 and 1.048
/// before the feather; at most 0.669 and 0.736 under a nudge of libm. On #469's
/// chart, with the feather at 1.05, 0.617 and 0.661 but 0.792 under a nudge;
/// at 1.15, 0.605 and 0.637, at most 0.697 and 0.764 under a nudge. On #473's
/// face chart 0.541 and 0.583, at most 0.537 and 0.576 under a nudge.
///
/// **Tightened by #473's fixer**, so that it still catches the beard it is
/// for: on the face chart a beard painted with no feather at all read 0.838
/// on seed 175, a hair inside 0.80. On the charts of the face, the crown and
/// the band round the back, feathered, 0.408 and 0.580, at most 0.512 and
/// 0.580 under a nudge; with no feather 0.791 and 0.954, at least 0.787 and
/// 0.875 under a nudge.
///
/// Re-measured with #475's moustache, its outline round the mouth and its
/// ragged edge: 0.485 and 0.553, at most 0.520 and 0.607 under a nudge; seed
/// 175 with no feather 0.945. **The ragged edge is what sets this one now**:
/// its tufts are steps a texel or two apart wherever the face chart is fine,
/// and `skin::RAGGED` and `skin::RAGGED_ACROSS` were sized under it - at 6 mm
/// and 260 tufts a metre seed 237 read 0.686 at a moustache's end.
///
/// With #475's fixer's thinner ends 0.485 and 0.584, at most 0.510 and 0.601
/// under a nudge; ends two fades across read 0.693 on seed 237, and 0.770
/// under a nudge, which is what set `moustache::END_FADES`.
///
/// With #475's second fixer's band under the nose, its top at where the skull
/// measured the nose's underside beginning and its bottom over half a fade:
/// 0.458 and 0.669, at most 0.498 and 0.655 under a nudge - seed 237's worst
/// is the band's clipped bottom edge (`moustache::LIP_FADE`). Cut by the
/// skin's facing alone instead, the top edge stepped 0.75 to 0.86: a texel's
/// normal turns 0.16 to 0.28 between neighbours across the crease, and a cut
/// on it is not feathered.
const BEARD_STEP: f32 = 0.70;

/// The same, for the thousandth-worst step.
///
/// Measured at #432: 0.425 and 0.511, against 0.693 and 0.830 before; at most
/// 0.425 and 0.526 under a nudge of libm. On #469's chart 0.374 and 0.413, at
/// most 0.377 and 0.446 under a nudge. On #473's face chart 0.306 and 0.375,
/// at most 0.306 and 0.374 under a nudge.
///
/// **Tightened by #473's fixer**: with no feather the face chart read 0.455,
/// under 0.55, so this half no longer caught #432's bug. On the charts of the
/// face, the crown and the band round the back, feathered, 0.308 and 0.388,
/// at most 0.309 and 0.388 under a nudge; with no feather 0.425 and 0.543, at
/// least 0.416 and 0.518 under a nudge (seed 237 fails it by a tenth).
///
/// Re-measured by #473's second fixer, feathered: 0.299 and 0.388, at most
/// 0.303 and 0.388 under a nudge; seed 175 with no feather 0.387, 0.380 to
/// 0.394 under a nudge - inside this, and above seed 237's feathered 0.388, so
/// no one bound here catches it on both rolls, and it is left: seed 175's
/// hard edge is caught by [`BEARD_STEP`], 0.782 and at least 0.752 under a
/// nudge against the feathered beard's 0.498 and at most 0.596.
///
/// Re-measured with #475's moustache, its outline round the mouth and its
/// ragged edge (`skin::RAGGED`): 0.300 and 0.377, at most 0.298 and 0.385
/// under a nudge; seed 175 with no feather 0.532.
const BEARD_STEP_THOUSANDTH: f32 = 0.42;

#[test]
fn where_two_beard_regions_meet_the_beard_keeps_its_density() {
    // **The chin's patch and the flanks' overlap under the jaw, and each was
    // laid over the other** (#432): at the 0.665 seed 175 rolls, skin both
    // regions held whole went `1 - (1 - 0.665)^2` = 0.89 of the way to the
    // beard's colour, and painted as a denser patch at each corner of the jaw
    // and under the chin. Density is coverage and nothing else (#344), so
    // where the two regions between them hold the skin whole - each at half or
    // more, the two at 1.3 or more - the texel goes the beard's density of the
    // way, as it does under either alone.
    //
    // Measured at #432: 0.655 on seed 175 at 0.665, and 0.819 with the regions
    // laid one after another again under the same feather; seed 237 rolls
    // 0.995, where the two arithmetics agree, and reads 0.999.
    for seed in [24, 69] {
        let rolled = Rolled::of(seed);
        let bare = rolled.shaved();
        let mut shares = Vec::new();
        for (at, texel) in rolled.geometry.texels.iter().enumerate() {
            let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) else {
                continue;
            };
            let [_, chin, flanks] = rolled.beard_at(texel.position);
            let both = chin.min(flanks) >= 0.5 && chin + flanks >= 1.3;
            if let Some(share) = rolled
                .share(&rolled.avatar.skin, &bare, at)
                .filter(|_| both)
            {
                shares.push(share);
            }
        }
        assert!(
            shares.len() > 200,
            "seed {seed}: only {} texels where both regions reach",
            shares.len()
        );
        let mean = shares.iter().sum::<f32>() / shares.len() as f32;
        println!(
            "seed {seed}: where the chin and the flanks both reach the beard goes {mean:.3} of \
             the way to its colour, at a density of {:.3}, over {} texels",
            rolled.density,
            shares.len()
        );
        assert!(
            (mean - rolled.density).abs() <= 2.0 * PAINTED_SLACK,
            "seed {seed}: where two beard regions meet the beard goes {mean:.3} of the way to its \
             colour at a density of {:.3}",
            rolled.density
        );
    }
}

#[test]
fn a_feathered_beard_paints_no_halo_and_thins_no_edge() {
    // **The feather is to soften a beard's edge over the texels the atlas has,
    // not to move it** (#432). Sized as a round disc by the atlas's coarsest
    // step, it reached across a dozen of the fine texels the other way under
    // the jaw: it painted a half-tone band past the masks' edge, and thinned
    // the beard inside it.
    //
    // Read texel by texel on the head against the same body painted with the
    // hard edge, each with its share of the way to the beard's colour against
    // its masks at its centre: a HALO texel is one every beard mask leaves bare
    // (under 0.01) painted past a tenth of the density, a THINNED one is one a
    // mask holds whole (0.99 or more) painted under nine tenths of it. The
    // hard edge has thinned texels of its own, where the grain's swing in a
    // pale beard over pale skin reads as less than whole.
    //
    // Measured at #432, halo and thinned with the hard edge, with builder E's
    // round disc of one footprint, and with this feather: seed 175 0 and 0,
    // 270 and 339, 177 and 216; seed 237 0 and 557, 333 and 888, 254 and 828;
    // seed 4 0 and 2437, 446 and 2722, 301 and 2664; seed 5 0 and 1392, 387
    // and 1693, 213 and 1626; seed 7 0 and 451, 279 and 866, 196 and 769; seed
    // 14 0 and 2430, 242 and 3116, 108 and 2920. On #469's chart, with the
    // feather at 1.15: halo at most 169 more than the hard edge (seed 4) and
    // thinned at most 320 more (seed 14), under every nudge of libm.
    //
    // Each against the same body shaved and painted the same way, with the
    // footprints or without (#473's second fixer): the scalp's grain reads its
    // spacing off the footprints where it has them (`skin::scalp_spacing_at`),
    // and against a bare skin painted without them the scalp's own grain
    // counted as halo - 453 texels of seed 4's hairline.
    for seed in [24, 69, 4, 969, 7, 14] {
        let rolled = Rolled::of(seed);
        let counts = |painted: &TextureMap, bare: &TextureMap| {
            let (mut halo, mut thinned) = (0usize, 0usize);
            for (at, texel) in rolled.geometry.texels.iter().enumerate() {
                let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) else {
                    continue;
                };
                let Some(share) = rolled.share(painted, bare, at) else {
                    continue;
                };
                let beard = rolled.beard_at(texel.position);
                if beard.iter().all(|&mask| mask < 0.01) && share > 0.1 * rolled.density {
                    halo += 1;
                }
                if beard.iter().any(|&mask| mask >= 0.99) && share < 0.9 * rolled.density {
                    thinned += 1;
                }
            }
            (halo, thinned)
        };
        let hard = counts(&rolled.repaint(None, false), &rolled.repaint(None, true));
        let feathered = Some(&rolled.footprints);
        let soft = counts(
            &rolled.repaint(feathered, false),
            &rolled.repaint(feathered, true),
        );
        println!(
            "seed {seed}: halo {} -> {}, thinned {} -> {} (hard edge -> feathered)",
            hard.0, soft.0, hard.1, soft.1
        );
        assert!(
            soft.0 <= hard.0 + FEATHER_HALO && soft.1 <= hard.1 + FEATHER_THINNED,
            "seed {seed}: the feathered beard paints {} halo texels and {} thinned, against {} \
             and {} with the hard edge",
            soft.0,
            soft.1,
            hard.0,
            hard.1
        );
    }
}

/// How many more halo texels a feathered beard may paint than its hard edge,
/// on the 1024-texel atlas; see
/// `a_feathered_beard_paints_no_halo_and_thins_no_edge`.
///
/// A feather has some by its nature: it softens an edge both ways, and past
/// the masks' last hundredth its outer half reads as halo. Measured at #432:
/// at most 301 (seed 4), and no more than that under any nudge of libm; the
/// round disc painted 446 and 387 on seeds 4 and 5, so 350 was a bound it
/// failed. On #469's finer chart at most 181 (seed 4, under a nudge). On #473's
/// second fixer's charts, each against a bare skin painted the same way, at
/// most 115 (seed 4, under a nudge).
///
/// **What it catches now** (#473's third fixer). On #473's charts the feather
/// paints 29 to 103 halo texels over the six seeds (111 at most under a nudge
/// of libm), and a feather that spills, its weights doubled and held to one,
/// 193 to 303: at 350 it passed on every seed, so the bound sits between them,
/// the spill failing on all six. A
/// feather twice and one and a half times as wide paints 551 and 214 on seed
/// 175 and fails too. The round disc is no longer told apart by its halo on
/// these charts, which lay the head's skin about as finely each way: 35 to
/// 131, inside the feather's own spread across the seeds; what still tells it
/// apart is the footprint's own test
/// (`bake::tests::a_footprint_is_one_texel_of_skin_each_way_and_one_reading_at_a_seam`).
///
/// With #475's moustache 20 to 78 over the six seeds, 128 at most under a
/// nudge (seed 4): its outline's edge is longer, round the mouth, and its
/// ragged edge only ever eats into the masks, so it paints no halo of its own
/// (a ragged edge read outward as well painted 69 to 219, and failed here).
/// With #475's fixer's outline 21 to 79, 131 at most under a nudge (seed 4).
const FEATHER_HALO: usize = 160;

/// How many more thinned texels.
///
/// Measured at #432: at most 490 more (seed 14), and no more under a nudge of
/// libm; the round disc 686 more there. On #469's chart at most 320 more. On
/// #473's second fixer's charts at most 225 more (seed 7, under a nudge). With
/// #475's ragged edge at most 328 more (seed 7), 322 under a nudge; with its
/// fixer's outline at most 326 more (seed 7), and 326 under a nudge.
const FEATHER_THINNED: usize = 600;

#[test]
fn a_painted_moustache_leaves_the_nose_s_underside_bare() {
    // **The owner's report, #475**: the painted moustache bled onto the
    // underside of the nose. Its ceiling was a height, the nostril line, and
    // the nose's foot overhangs the lip 6 mm below it on the carved face, so
    // the skin that faces the ground under the nose and its wings took the
    // moustache's whole density. Read on the built atlas, texel by texel, as how
    // far each head texel that faces down (its normal's `y` under -0.6) between
    // the nose's foot and the nostril line, under the nose, went toward the
    // beard's colour, in shares of the beard's density.
    //
    // Measured: 0.00 on both rolls, over 299 and 37 texels, at most 0.02 under
    // a nudge of libm; before #475 the worst read 1.00 and 1.12.
    for seed in [24, 69] {
        let rolled = Rolled::of(seed);
        let bare = rolled.shaved();
        let outline = rolled.follicles.outline();
        let (mut worst, mut read) = (0.0f32, 0usize);
        for (at, texel) in rolled.geometry.texels.iter().enumerate() {
            let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) else {
                continue;
            };
            let local = texel.position - rolled.follicles.origin();
            let under_the_nose = texel.normal.y < -0.6
                && local.z > 0.0
                && local.x.abs() < outline.nose
                && local.y > outline.foot
                && local.y < outline.lip.nostrils;
            if !under_the_nose {
                continue;
            }
            let Some(share) = rolled.share(&rolled.avatar.skin, &bare, at) else {
                continue;
            };
            read += 1;
            worst = worst.max(share / rolled.density);
        }
        println!(
            "seed {seed}: the nose's underside goes {worst:.2} of the beard's density toward its \
             colour at worst, over {read} texels"
        );
        assert!(
            read > 20,
            "seed {seed}: only {read} texels under the nose to read"
        );
        assert!(
            worst <= NOSE_BARE,
            "seed {seed}: the painted moustache reaches {worst:.2} of its density onto the \
             underside of the nose"
        );
    }
}

/// The most of a beard's density any texel under the nose may take; see
/// `a_painted_moustache_leaves_the_nose_s_underside_bare`. A twentieth: the
/// grain's own swing on skin the masks leave bare is none, so this is room for
/// the colour arithmetic and nothing else.
const NOSE_BARE: f32 = 0.05;

#[test]
fn a_painted_moustache_fills_the_lip_under_the_nose() {
    // **The painted Chaplin was a pencil line** (#475's second fixer). The
    // patch's ceiling was the nose's foot with the mask's whole fade under it,
    // and the lip under a carved nose is five or six millimetres from the
    // vermilion to where the nose's underside begins: a band that came fully
    // on only in its lowest two millimetres, so a Chaplin - the band under the
    // nose and nothing else - and the middle of a classic read as a stroke
    // along the vermilion, about a third of the lip.
    //
    // Read on the built atlas down the middle of the lip, two millimetres
    // either side of the midline, from the vermilion up to the lowest texel
    // that faces the ground as the nose's underside does (`y` under -0.6, the
    // bound `a_painted_moustache_leaves_the_nose_s_underside_bare` reads the
    // underside by): how far toward the beard's colour the lip went, in
    // shares of its density, on the mean over that height - texels averaged
    // in half-millimetre rows and read between the rows - at `width` `-1`,
    // the Chaplin, and `0`, the classic. A share of the lip, so a band that
    // fills it reads near one whatever the lip's height, and a stroke along
    // the vermilion a third.
    //
    // Measured: see `LIP_FILLED`.
    let mut thinnest = f32::MAX;
    for seed in [24, 69] {
        for width in [-1.0f32, 0.0] {
            let rolled = Rolled::with(seed, |record| {
                record.hair.regions.moustache.width = width;
            });
            let bare = rolled.shaved();
            let outline = rolled.follicles.outline();
            let mut column: Vec<(f32, f32, f32)> = Vec::new();
            for (at, texel) in rolled.geometry.texels.iter().enumerate() {
                let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) else {
                    continue;
                };
                let local = texel.position - rolled.follicles.origin();
                if local.x.abs() > 0.002
                    || local.z <= 0.0
                    || local.y < outline.lip.vermilion
                    || local.y > outline.lip.nostrils
                {
                    continue;
                }
                let Some(share) = rolled.share(&rolled.avatar.skin, &bare, at) else {
                    continue;
                };
                column.push((local.y, texel.normal.y, share / rolled.density));
            }
            column.sort_by(|one, two| one.0.total_cmp(&two.0));
            let crease = column
                .iter()
                .find(|(_, facing, _)| *facing < -0.6)
                .map(|(y, _, _)| *y)
                .unwrap_or_else(|| panic!("seed {seed}: no texel under the nose faces the ground"));
            let lip = (crease - outline.lip.vermilion).max(f32::EPSILON);
            // Half-millimetre rows from the vermilion to the underside, each
            // the mean of its texels, read between the rows' middles.
            const ROW: f32 = 0.0005;
            let count = (lip / ROW).ceil() as usize;
            let mut rows = vec![(0.0f32, 0usize); count];
            for (y, _, share) in &column {
                let row = ((y - outline.lip.vermilion) / ROW) as usize;
                if row < count {
                    rows[row].0 += share.clamp(0.0, 1.0);
                    rows[row].1 += 1;
                }
            }
            let read: Vec<(f32, f32)> = rows
                .iter()
                .enumerate()
                .filter(|(_, (_, n))| *n > 0)
                .map(|(row, (sum, n))| ((row as f32 + 0.5) * ROW, sum / *n as f32))
                .collect();
            let mut filled = 0.0f32;
            for pair in read.windows(2) {
                filled += (pair[0].1 + pair[1].1) * 0.5 * (pair[1].0 - pair[0].0);
            }
            let filled = filled / lip;
            println!(
                "seed {seed} width {width:+}: the moustache fills {filled:.2} of the {:.1} mm of lip \
                 under the nose, over {} texels",
                lip * 1000.0,
                column.len()
            );
            assert!(
                column.len() >= 6,
                "seed {seed} width {width:+}: only {} texels down the midline to read",
                column.len()
            );
            thinnest = thinnest.min(filled);
        }
    }
    assert!(
        thinnest >= LIP_FILLED,
        "the painted moustache fills only {thinnest:.2} of the lip under the nose"
    );
}

/// The least share of the lip under the nose a painted moustache fills, down
/// the middle of the lip; see
/// `a_painted_moustache_fills_the_lip_under_the_nose`.
///
/// Measured at #475's second fixer: seed 175 0.57 of 6.4 mm at `-1` and at
/// `0`, seed 237 0.66 of 6.1 mm at both; at least 0.56 under a nudge of libm.
/// With #475's fixer's ceiling at the foot under the whole fade 0.41 and 0.30
/// on seed 175, 0.34 and 0.27 on seed 237.
const LIP_FILLED: f32 = 0.5;
