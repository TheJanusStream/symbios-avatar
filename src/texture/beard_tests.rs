//! The painted beard's edge, read on rolled bodies built whole (#432).
//!
//! Here rather than in `tests/texture.rs` beside the #344 guard because each of
//! these reads how much skin a texel spans, which the crate measures beside its
//! atlas and keeps to itself: see [`Footprints`].

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
        let mut record = AvatarRecord::new("Bearded", Archetype::default());
        record.reroll(seed);
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
        let body = &avatar.parts.body;
        let (geometry, footprints) = bake_measured(
            body,
            &avatar.parts.unwrap,
            &[],
            &vec![0.0; body.vertex_count()],
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
    // along the wrap, a texel spans four to eight millimetres of skin, and a
    // chin patch fading over five did its whole fade inside one: coarse,
    // hard-edged patches under the chin and along the jaw on the issue's two
    // rolls, seeds 175 and 237.
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
    // folds and texels side by side stand five to ten millimetres apart.
    for seed in [175, 237] {
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
                let coarser = rolled.footprints[at].axes[0]
                    .length()
                    .max(rolled.footprints[next].axes[0].length());
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
/// before the feather; at most 0.669 and 0.736 under a nudge of libm.
const BEARD_STEP: f32 = 0.80;

/// The same, for the thousandth-worst step.
///
/// Measured at #432: 0.425 and 0.511, against 0.693 and 0.830 before; at most
/// 0.425 and 0.526 under a nudge of libm.
const BEARD_STEP_THOUSANDTH: f32 = 0.55;

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
    for seed in [175, 237] {
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
    // 14 0 and 2430, 242 and 3116, 108 and 2920.
    for seed in [175, 237, 4, 5, 7, 14] {
        let rolled = Rolled::of(seed);
        let bare = rolled.repaint(None, true);
        let counts = |painted: &TextureMap| {
            let (mut halo, mut thinned) = (0usize, 0usize);
            for (at, texel) in rolled.geometry.texels.iter().enumerate() {
                let Some(texel) = texel.filter(|texel| texel.zone == Zone::Head) else {
                    continue;
                };
                let Some(share) = rolled.share(painted, &bare, at) else {
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
        let hard = counts(&rolled.repaint(None, false));
        let soft = counts(&rolled.repaint(Some(&rolled.footprints), false));
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
/// round disc painted 446 and 387 on seeds 4 and 5, so this is a bound it
/// fails.
const FEATHER_HALO: usize = 350;

/// How many more thinned texels.
///
/// Measured at #432: at most 490 more (seed 14), and no more under a nudge of
/// libm; the round disc 686 more there.
const FEATHER_THINNED: usize = 600;
