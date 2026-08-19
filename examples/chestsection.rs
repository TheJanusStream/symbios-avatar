//! What the trunk is shaped like ACROSS, not how wide it is.
//!
//! `examples/bodyaudit` prints the trunk silhouette band by band as fractions
//! of stature **across**, and its HEIGHTS table carries reference figures that
//! are heights of landmarks. Both are the right instruments for the questions
//! they were built for and neither can see a chest: a pectoral and a breast are
//! **fore-and-aft**, and how far the front of the chest stands off the ribcage
//! it sits on, where that stands off peaks, and whether there are two of them
//! are three numbers nothing in this crate reports. Until they exist, the
//! defect epic #269 was raised for can only be seen, not stated.
//!
//! The precedent is `examples/facesection`, which bisects the **built surface**
//! rather than reading vertices, and which turned one described complaint about
//! the nose and mouth into three measured ones. This does the same on the
//! trunk.
//!
//! # The three readings, and why each is shaped the way it is
//!
//! **The section across.** At each band the front surface is walked from flank
//! to flank and the profile `reach(x)` is read by bisection. A tube's section is
//! an ellipse, so `reach` is concave and has exactly one maximum, on the
//! midline. A chest has **two**, off the midline, with a sternum between them.
//! So the reading is not a width but a count: how many prominent maxima, how far
//! apart, and how deep the dip between them. `reach` on a convex section is
//! concave whatever the faceting does to it, so a spurious second peak is not a
//! thing this can report by accident — and [`PROMINENCE`] guards the case where
//! it could.
//!
//! **The projection off the ribcage.** A projection measured against zero says
//! nothing; it has to be against the line the feature stands on, and that line
//! is the body's own BACK at the same height. Every trunk ring in this plan is
//! a centred ellipse, so a body with no chest is front-to-back symmetric by
//! construction and the asymmetry IS the feature. It reads exactly `+0.00` mm
//! at every band of today's body, which is a noise floor of nothing rather than
//! a number to subtract. See [`ribcage_residual`] for the fitted-ellipse
//! reference that was built first and refuted.
//!
//! **The two readings have very different sensitivities, and that is the point
//! rather than a redundancy.** Against the synthetic control at life spacing:
//!
//! | authored lobe | 10 mm | 20 mm | 40 mm | 90 mm |
//! |---|---|---|---|---|
//! | proud of the back | 8.3 | 16.7 | 33.4 | 75.0 |
//! | sides the section shows | 1 | 1 | 2 | 2 |
//!
//! So a projection is visible from a millimetre up and a SECTION only becomes
//! two-sided somewhere between 20 and 40 mm — because a feature at the
//! intermammary distance has to out-climb the ribcage's own fall of 30.6 mm
//! over the same span before it is a separate peak at all. A male pectoral
//! stands 10 to 20 mm in life, so on this trunk it is a projection that never
//! becomes a shape. Any mechanism #271 picks has to clear both bars, and they
//! are an order of magnitude apart.
//!
//! **The resolution actually available.** The nose dorsum had ONE cell between
//! the midline and the edge of the feature and that was the whole of #181. The
//! same number is printed here for the chest, because whichever mechanism #271
//! picks has to draw a feature on the cells that exist.
//!
//! # The control, which this cannot ship without
//!
//! Every section reading is exactly zero on today's body, and a number that
//! reads zero for every configuration **including the control** is what a dead
//! instrument looks like — this crate has caught twenty of them. There is no
//! feature to suppress here, so the control is synthesised: `--lobe` displaces
//! the built surface by a known analytic pair of lobes and the readings must
//! come back with the numbers that were put in. It is deliberately a
//! displacement of the FINISHED mesh and not a mechanism — it commits to
//! nothing #271 has to decide, and it doubles as the target the candidates
//! there are costed against.
//!
//! With `--lobe` the section readings are joined by a facesection-style
//! subtraction, `reach(lobed) − reach(plain)`, which is the relief the mesh
//! actually delivered against the relief that was authored. Their ratio is how
//! much of the feature survived onto the polygons, and it is a second,
//! independent check on the fitted residual: two readings taken different ways
//! agreeing is what says the instrument works.
//!
//! # The shape questions, and why each needs its own ruler (#284)
//!
//! The readings above answer "is there a chest". These answer "is it shaped
//! like one", which is milestone #9's whole subject, and **every one of them is
//! printed at femininity `-1`, `0` and `+1` in the same table whatever the
//! flags say**. A reading only ever taken at one setting has no control, and
//! this crate has caught thirty-nine instruments measuring something other than
//! their name — each one of them by a control.
//!
//! Two profiles are read down the lobe and they are deliberately different
//! rulers. The **crest** is the largest relief over the bare body at each
//! height: the lobe's own vertical profile with the ribcage divided out, which
//! is what a share of volume has to be measured on. The **cut** is the absolute
//! forward reach along one vertical line through the crest's peak: the
//! silhouette a render shows and a caliper reads, which is what every
//! anatomical claim about pole shape is stated on. Reading a pole's curvature
//! off the relief would call the ribcage's own fall part of the breast; reading
//! its volume off the silhouette would call the ribcage part of the volume.
//!
//! **Pole ratio**, on the crest. Life prefers 45:55 upper:lower — the ratio
//! Mallucci's series and the ASPS studies put at 87–94% preference. Each pole
//! runs from the peak to where the relief falls to [`POLE_END`] of its peak,
//! and the share is the area under the relief over that span; truncating both
//! poles at the same relief HEIGHT rather than at a fixed distance is what
//! makes a symmetric lobe read 50:50 exactly rather than approximately.
//!
//! **The carve's vertical term used to be a symmetric Gaussian and the surface
//! it delivers was not, which is the first thing this reading found.** Measured
//! on 0.2.0 before #285 and #286: an authored 50:50 arrived as 58:42 at
//! femininity 0, 63:37 at `-1` and 50:50 at `+1` — upper-heavy by eight points
//! at the neutral, because a radial push lands a vertex where the ribcage
//! behind it was shallower and how much shallower is not the same above the
//! peak as below it. Refining the trunk (#285) took the neutral to 55:45 and
//! the rest is the carve's own asymmetry (#286): today the shipped body reads
//! **45:55 at femininity 0**, 49:51 at `-1` and 43:57 at `+1`, off an authored
//! 39:61. That gap between the `authored` row and the top one is the whole
//! reason `POLES` overshoots, and the reason #286's target was a MEASURED one
//! rather than `45/55` written into a constant.
//!
//! **Pole bow**, on the cut: the mean signed distance from the silhouette to
//! the chord across its own pole, positive out. Convex is positive, straight is
//! zero, hollow is negative. Life wants a lower pole convex and an upper pole
//! straight to slightly concave; a Gaussian is convex on both sides of its
//! peak, by an equal amount, which is again its own control.
//!
//! **Fold depth and seat**, on the cut. An inframammary fold is a crease on a
//! DESCENDING ramp — the ribcage is already falling away toward the waist under
//! it — so it is not a local minimum of the profile and looking for one finds
//! nothing on any body, folded or not. What it is, is a local concavity: how
//! far the surface sits below the chord spanning [`NOTCH`] either side of it.
//! The deepest such notch below the peak is the fold, and where it sits below
//! the peak is the seat life puts at about 70 mm on a reference-scale body.
//!
//! **Its noise floor is not small and it is a resolution finding, which is why
//! the bare row exists.** The `bare` row is the same chord on the UNCARVED body
//! at the same cut, so it says how much of a notch the ribcage's own faceting
//! is worth before any chest is put on it: 4.2, 7.4 and 13.1 mm at the three
//! femininities on today's refined trunk, and 3.7, 8.4 and 17.4 before #285
//! refined it.
//!
//! **So the fold this crate now carves is under its own instrument's floor, and
//! that is the honest reading of it.** The carved rows are 2.0, 1.3 and 5.5 mm
//! seated 15, 35 and 70 mm below the peak, and on a LEAN body — where
//! `torso::FOLD_DEPTH` is deepest — 1.0, 2.2 and 2.7 mm seated 71, 63 and 50.
//! The SEAT is the reading that moved:
//! before #286 the deepest concavity under the lobe sat 3 to 15 mm under the
//! peak, which is the peak's own shoulder, and now it sits where life puts the
//! inframammary fold. The depth does not clear the ribcage's faceting, and
//! ablation says why rather than leaving it open: at three times the shipped
//! depth the same body reads 20.2 mm against a bare 13.1 and the crease renders
//! plainly. The term is right and the cell under it is 15 to 23 mm against a
//! crease that is 20 to 40 mm wide in life — which is the second refinement
//! pass #285 costed and the budget refused.
//!
//! **The border's steepness**, on the cut: the largest `|d reach / dy|` inside
//! the LOWER POLE, over a fixed [`SLOPE_BASE`] baseline. A pectoral's defining
//! feature is a crisp INFERIOR border and a Gaussian's is its smoothness, so
//! this is the masculine reading and a smooth lobe's number is its own control.
//!
//! **Windowed to the pole rather than to everything below the peak, and the
//! control is why**: the ribcage falls toward the waist more steeply than a
//! pectoral stands off it, so the unwindowed reading was the TRUNK's slope on
//! every body — at femininity `-1` and 5% fat it gave 0.481 for the carved
//! chest and 0.481 for the bare one under it, identical to the digit.
//!
//! **The sternal gap**, twice. The section's existing "sternum below the crest"
//! is kept because it is a ledger, and a **groove** reading is added beside it
//! because that one cannot see a masculine gap at all: see [`GROOVE`], where a
//! 3.75 mm cut into the midline read 0.00 by construction.
//!
//! # What a refinement band would cost, before one is written (#285)
//!
//! `--band near,far,low,high` selects trunk faces the way the carve itself
//! selects vertices — the same quarter-turn azimuth, the same
//! `Zone::Chest | Zone::Abdomen` gate through `nearest_bone`, heights in the
//! waist-to-girdle band `torso::Column` is written in — refines them, and
//! reports what that cost. Repeat the flag for a pass set; they apply in order,
//! exactly as `face::refine_face` walks its own table.
//!
//! **Two predictions are printed beside the measurement and the gap between
//! them is the point.** A quad that splits becomes four quads, so the napkin
//! arithmetic is `+6 triangles per selected quad`. But an unselected face
//! ABSORBS every midpoint a selected neighbour put on a shared edge — that is
//! how [`PolyMesh::refine_curved`] stays conforming — so each face of the
//! band's own boundary pays one more triangle. That is the face passes' "a band
//! edge landing ON a ring of faces costs a whole row" (#61) as a number instead
//! of a warning, and it is why a band is measured at the sweep corners rather
//! than argued at the default.
//!
//! ```text
//! cargo run --release --example chestsection
//! cargo run --release --example chestsection -- --femininity 1
//! cargo run --release --example chestsection -- --volume 1 --projection -1
//! cargo run --release --example chestsection -- --lobe 20
//! cargo run --release --example chestsection -- 7 23 42
//! cargo run --release --example chestsection -- --band 0,0.95,0.45,0.92
//! cargo run --release --example chestsection -- --band 0,0.95,0.45,0.92 --band 0,0.2,0.5,0.9
//! cargo run --release --example chestsection -- --profile
//! ```
//!
//! **A band edge inside the lobe reads as a fold, and that is the trap this
//! probe exists to catch before #286 does.** With the band's floor at `0.50` of
//! the waist-to-girdle span — 34 mm under the peak, inside the lower pole — the
//! neutral body reported 5.40 mm of notch seated 44 mm below its peak, against
//! 2.90 for the same body with the boundary clear of the lobe. **The `bare` row
//! did not move**, which is what identifies it: the coarse side of a resolution
//! boundary under-delivers the carve's push and the step between the two sides
//! is a crease that only a CARVED body has. A refinement band for the chest has
//! to clear the whole lobe, not just the part of it a section is taken
//! through — see `torso::refine_chest`, whose floor sits at 0.05 for exactly
//! this reason and could not afford the −0.10 that was wanted.
//!
//! Life figures to column against, quoted from general anthropometric knowledge
//! rather than from a named table, in the same way `face::eye`'s globe is: a
//! male pectoral stands 10 to 20 mm off the ribcage at its thickest, a female
//! breast 40 to 90 mm from chest wall to nipple over the same ribcage, the two
//! sides separated by an intermammary distance of 180 to 230 mm at the nipples,
//! and both features peak between a fifth and a third of the way down the
//! sternum from the sternal notch.

use std::collections::HashMap;

use symbios_avatar::face::HeadTraits;
use symbios_avatar::torso::{ChestTraits, carve_chest};
use symbios_avatar::{
    Archetype, AvatarRecord, BODY_SUBDIVISIONS, CageConfig, PolyMesh, Rig, Skeleton, Vec3, Zone,
    build_body,
};

/// How far a bisection may travel forward from the trunk's own axis, in metres.
///
/// A trunk is nowhere near 400 mm deep from its spine; the bound only has to
/// lie outside every surface it will be asked about.
const FAR: f32 = 0.40;

/// Halvings per bisection. Thirty takes any trunk to well under a micron.
const HALVINGS: usize = 30;

/// Millimetres between samples across a section.
///
/// Far finer than any cell on the trunk — those run to tens of millimetres —
/// because the point is to read the polygon surface rather than to re-sample
/// the ring that drew it. A section walked at cell resolution cannot tell a
/// facet from a feature.
const STEP: f32 = 2.0;

/// Millimetres between samples down the midline.
const RISE: f32 = 5.0;

/// How far a local maximum must stand above the dip beside it to be counted, in
/// metres.
///
/// **A millimetre, and it is the guard against faceting rather than a
/// threshold on features.** `reach` across a convex section is concave and so
/// has one maximum however coarse the mesh is, but a section is only convex
/// while nothing else pushes on it — an arm socket, a clavicle — and a
/// vertex-level kink could otherwise be reported as a second lobe. Anything a
/// chest mechanism is worth building will clear this by an order of magnitude:
/// the shallowest feature this epic is about, a male pectoral, stands 10 mm.
const PROMINENCE: f32 = 1e-3;

/// Where a synthetic lobe sits and how wide it is, as shares of the section's
/// own half-width and of the chest band's height.
///
/// Sized on the life figures in the module note rather than on what the mesh
/// can carry: a control that was shrunk to fit the resolution would be
/// measuring the instrument against itself. The trunk's own half-width at the
/// chest is 130 mm on the reference body, so an intermammary distance of 180 to
/// 230 mm puts the pair at 0.69 to 0.88 of it and 0.70 is the low end of life
/// rather than a number that suited the mesh.
const LOBE: (f32, f32, f32, f32) = (0.70, 0.28, 0.62, 0.16);

/// Millimetres between samples across a section when the question is only
/// WHERE the crest is.
///
/// Coarser than [`STEP`] on purpose. A section's shape is read at 2 mm because
/// a facet has to be distinguishable from a feature; the crest line only needs
/// the argmax at each height, and the lobe is a hundred millimetres across, so
/// this halves the bisection count of the whole shape table for a 4 mm
/// uncertainty in a position nothing downstream divides by.
const CREST_STEP: f32 = 4.0;

/// Millimetres between samples down the cut through the crest's peak.
///
/// Finer than [`RISE`] because the readings taken on it — a notch and a maximum
/// slope — are the two that a coarse sampling flatters. Well under the trunk's
/// own 30 to 50 mm cell, which means this reads the polygon surface rather than
/// the ring that drew it, exactly as [`STEP`] does across.
const CUT: f32 = 2.5;

/// Half the chord the sternal groove's depth is measured against, in
/// millimetres.
///
/// **A groove is not a crest, and "sternum below the crest" cannot see one.**
/// That reading is `crest − midline` across the section, and on a chest whose
/// lobes have not out-climbed the ribcage's own fall the furthest-forward point
/// IS the midline — so it reads 0 by construction however deep a sternal gap is
/// cut. Measured: at femininity `-1` with the gap carving 3.75 mm into the
/// midline it still read 0.00, because the ribcage's midline stands about 30 mm
/// proud of the lobe's azimuth and a 12.5 mm pectoral cannot make that up. That
/// is #271's two-sided bound wearing a different name, and it is why the
/// masculine reading has to be a local concavity instead.
///
/// The pair sits about 91 mm off the midline on the reference trunk and
/// `torso::GAP_WIDE` puts the groove's own width at about 30 mm, so a chord
/// spanning 50 mm sits on the sternum's shoulders either side of it and inside
/// the lobes.
const GROOVE: f32 = 25.0;

/// Half the chord a fold's depth is measured against, in millimetres.
///
/// An inframammary crease runs 20 to 40 mm wide in life, so a chord spanning 40
/// mm sits on the surface either side of one. Larger and the ribcage's own
/// curvature enters the reading; smaller and the chord lies inside the crease
/// and measures nothing.
const NOTCH: f32 = 20.0;

/// The baseline a slope is taken over, in millimetres.
///
/// A central difference over ±5 mm. Adjacent [`CUT`] samples would report the
/// slope of whichever facet they land inside, which on a 30 to 50 mm cell is a
/// reading of the tessellation; 10 mm is still an order under the lobe and
/// averages nothing a border cares about.
const SLOPE_BASE: f32 = 5.0;

/// How far round the section the lobe's own crest is looked for, as a share of
/// a quarter turn from dead ahead.
///
/// `torso::SPACING` puts the pair at 0.57 and `SPREAD` gives it 0.18 to 0.28
/// either side, so a window this wide holds every lobe the carve can place and
/// then some. It is generous because it no longer has to be careful: see
/// [`radial`] for the ruler that made it so.
const PAIR: (f32, f32) = (0.0, 1.0);

/// Where a pole stops, as a share of the crest's peak relief.
///
/// **A relief HEIGHT and not a distance, which is what makes the control
/// exact.** Cut at a fixed distance either side of the peak, a symmetric
/// Gaussian truncated inside an asymmetric band reports an asymmetric area;
/// cut where the relief itself falls to a tenth, it reports 50:50 to the digit
/// whatever the band around it is doing. A tenth of peak is also about where a
/// lobe stops being visible against the ribcage it sits on.
const POLE_END: f32 = 0.10;

/// The femininities the shape table is always taken at.
///
/// Not a flag. See the module note: these readings exist to say what the chest
/// is SHAPED like, and every one of them starts life reading its own control —
/// 50:50 poles, no fold, a smooth border — so a table that could be asked for
/// one femininity at a time would be a table nobody ever saw a control in.
const CONTROLS: [f32; 3] = [-1.0, 0.0, 1.0];

/// What the flags asked for, over what the seed rolled.
///
/// One struct rather than seven `Option`s threaded through, because the shape
/// table builds its own bodies and the ONE thing it may not do is build them a
/// different way from the tables above it: a control that measures a different
/// body from the reading it controls is worse than no control.
#[derive(Clone, Copy, Default)]
struct Overrides {
    femininity: Option<f32>,
    mass: Option<f32>,
    fat: Option<f32>,
    age: Option<f32>,
    volume: Option<f32>,
    projection: Option<f32>,
    lift: Option<f32>,
    lobe: Option<f32>,
    bare: bool,
}

/// One built body, and everything every reading here is taken on.
struct Body {
    record: AvatarRecord,
    skeleton: Skeleton,
    rig: Rig,
    trunk: Trunk,
    /// The body as the cage and the skull left it, before the carve.
    plain: PolyMesh,
    /// What the readings are taken on: the carve, plus `--lobe` if it was
    /// asked for.
    surface: PolyMesh,
    stature: f32,
    /// What each `--band` pass cost on the way in, in order.
    cost: Vec<PassCost>,
    /// What the carve was run with, or `None` under `--bare`.
    traits: Option<ChestTraits>,
}

/// What one candidate refinement pass over the trunk cost, and what it bought.
struct PassCost {
    band: Band,
    /// Faces the band selected, and how many of them were quads.
    selected: (usize, usize),
    /// Faces of the selection's own perimeter — see [`boundary`].
    perimeter: usize,
    /// What the napkin says (`+6` per selected quad) and what the perimeter
    /// makes of it.
    predicted: (usize, usize),
    /// What it actually cost, and what the body stands at after it.
    spent: (usize, usize),
    /// The median cell in the chest band afterwards: over every edge, then
    /// across the trunk and down it.
    cell: (f32, f32, f32),
}

/// Builds one body at a seed and a set of overrides, the shipped way.
///
/// **The shipped order, exactly**: traits off the composites, one body built
/// from them, the carve applied here so that both surfaces are in one space.
/// See the note at the call site of `carve_chest` below for why the carve is
/// re-applied rather than read back off `Avatar::build`.
fn build(seed: i64, over: &Overrides, bands: &[Band]) -> Option<Body> {
    let mut record = AvatarRecord::new("Sectioned", Archetype::default());
    if seed >= 0 {
        record.reroll(seed);
    }
    // Compared at EQUAL STATURE, which is what these flags are for: rolling
    // a seed to find a feminine body moves five other axes at once, and
    // stature alone scales a body uniformly and proves nothing.
    if let Some(femininity) = over.femininity {
        record.composites.femininity = femininity;
    }
    if let Some(mass) = over.mass {
        record.composites.mass = mass;
    }
    if let Some(fat) = over.fat {
        record.composites.body_fat = fat;
    }
    if let Some(age) = over.age {
        record.composites.age = age as u32;
    }
    if let symbios_avatar::Archetype::Humanoid(params) = &mut record.archetype {
        if let Some(value) = over.volume {
            params.chest_volume = value;
        }
        if let Some(value) = over.projection {
            params.chest_projection = value;
        }
        if let Some(value) = over.lift {
            params.chest_lift = value;
        }
    }
    record.composites.sanitize();
    record.sanitize();

    let skeleton = record.skeleton();
    let traits = HeadTraits::of(&record.composites);
    let Ok(mut plain) = build_body(
        &skeleton,
        &CageConfig::default(),
        BODY_SUBDIVISIONS,
        &traits,
    ) else {
        println!("seed {seed} does not mesh");
        return None;
    };
    let rig = Rig::from_skeleton(&skeleton).ok()?;
    let mut trunk = Trunk::of(&rig)?;

    // **The candidate refinement goes in HERE, before the carve, because that
    // is where `refine_chest` will go** — the resolution-first block in
    // `build_body`, whose own comment is that resolution has to come before
    // shape or the shape is sampled at the coarser surface's cells. A band
    // costed on the side and never measured under a chest would answer the
    // cheaper half of the question.
    //
    // The half-width is read before the passes as well as after: `Cells` needs
    // one to know where the front of the trunk stops, and the passes move it.
    trunk.half = half_width(&plain, &rig, trunk.chest);
    let cost: Vec<PassCost> = bands
        .iter()
        .map(|&band| band.apply(&mut plain, &rig, &trunk))
        .collect();

    // Measured once, on the body before any synthetic lobe touches it, so
    // the lobe is one shape on the trunk rather than a different one at
    // every height and so the control cannot move its own reference.
    trunk.half = half_width(&plain, &rig, trunk.chest);

    // The carve applied here rather than read back off `Avatar::build`,
    // and the reason is worth a line: the shipped avatar is GROUNDED —
    // `AvatarConfig::ground` translates the whole body so its feet stand on
    // zero — so its mesh and this rig are in different spaces, and a
    // section taken at the rig's chest height on the shipped mesh reads
    // empty air. Measuring both surfaces here keeps one space; that the
    // shipped path applies the same carve is a test's job, not an
    // instrument's (`the_shipped_body_carries_the_chest_it_was_built_with`).
    //
    // `plain` is kept as the uncarved control, and every relief number
    // below is the difference between the two — `facesection`'s own design,
    // for its reason: a projection measured against the body beside it is
    // measuring the ribcage's curvature as much as the feature's.
    let mut carved = plain.clone();
    let mut carved_with = None;
    if !over.bare {
        let axes = match &record.archetype {
            symbios_avatar::Archetype::Humanoid(params) => symbios_avatar::torso::ChestAxes {
                volume: params.chest_volume,
                projection: params.chest_projection,
                lift: params.chest_lift,
            },
            _ => symbios_avatar::torso::ChestAxes::default(),
        };
        let traits = ChestTraits::of(&record.composites).on(axes);
        carve_chest(&mut carved, &rig, &traits);
        carved_with = Some(traits);
    }
    let surface = match over.lobe {
        Some(height) => lobe_onto(&carved, &trunk, height),
        None => carved,
    };
    let (low, high) = plain.bounds();
    let stature = (high.y - low.y).max(1e-3);
    Some(Body {
        record,
        skeleton,
        rig,
        trunk,
        plain,
        surface,
        stature,
        cost,
        traits: carved_with,
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let number = |name: &str| -> Option<f32> {
        args.iter()
            .position(|arg| arg == name)
            .and_then(|at| args.get(at + 1))
            .and_then(|value| value.parse().ok())
    };
    let over = Overrides {
        femininity: number("--femininity"),
        mass: number("--mass"),
        fat: number("--fat"),
        age: number("--age"),
        volume: number("--volume"),
        projection: number("--projection"),
        lift: number("--lift"),
        lobe: number("--lobe").map(|mm| mm / 1000.0),
        bare: args.iter().any(|arg| arg == "--bare"),
    };
    let bands = Band::all(&args);
    let seeds: Vec<i64> = args
        .iter()
        .enumerate()
        .filter(|(at, _)| *at == 0 || !args[at - 1].starts_with("--"))
        .filter_map(|(_, arg)| arg.parse().ok())
        .collect();
    let seeds = if seeds.is_empty() { vec![-1] } else { seeds };

    for seed in seeds {
        let Some(body) = build(seed, &over, &bands) else {
            continue;
        };
        let Body {
            record,
            rig,
            trunk,
            plain,
            surface,
            stature,
            skeleton,
            ..
        } = &body;

        println!(
            "\n=== seed {seed} — femininity {:+.2}, mass {:+.2}, fat {:+.2}; stature {:.3} m{}",
            record.composites.femininity,
            record.composites.mass,
            record.composites.body_fat,
            stature,
            over.lobe.map_or(String::new(), |h| format!(
                ", SYNTHETIC LOBE {:.0} mm",
                h * 1000.0
            ))
        );
        column(rig, skeleton, trunk, surface, *stature);
        let cells = Cells::of(surface, trunk);
        midline(surface, trunk, *stature);
        sections(surface, plain, rig, trunk, &cells, *stature, over.lobe);
        verdict(surface, plain, rig, trunk);
        if args.iter().any(|arg| arg == "--profile") {
            profiles(&body);
        }
        shape(seed, &over, &bands);
        costing(&body);
    }
}

/// The trunk's own column, and the band a chest lives in.
struct Trunk {
    /// Height of the waist joint, the bottom of the band.
    waist: f32,
    /// Height of the chest joint.
    chest: f32,
    /// Height of the shoulder girdle, the top of the band.
    girdle: f32,
    /// The column's joints, low to high, with their zone names.
    column: Vec<(&'static str, usize)>,
    /// How far the trunk reaches sideways at the chest, measured once.
    half: f32,
    /// Where the trunk's own axis runs, taken at the chest.
    ///
    /// `torso::Column`'s own field, for the refinement probe: an azimuth is
    /// measured from the axis the carve measures its own from, or the band and
    /// the shape under it are not talking about the same angle.
    axis: Vec3,
}

impl Trunk {
    /// Reads the column off the rig rather than off the plan that built it.
    fn of(rig: &Rig) -> Option<Self> {
        let one = |zone: Zone| rig.in_zone(zone).first().copied();
        let pelvis = one(Zone::Pelvis)?;
        let waist = one(Zone::Abdomen)?;
        let chests = rig.in_zone(Zone::Chest);
        let (&chest, &girdle) = (chests.first()?, chests.get(1)?);
        let neck = one(Zone::Neck)?;
        Some(Self {
            waist: rig.joints[waist].position.y,
            chest: rig.joints[chest].position.y,
            girdle: rig.joints[girdle].position.y,
            column: vec![
                ("pelvis", pelvis),
                ("waist", waist),
                ("chest", chest),
                ("girdle", girdle),
                ("neck", neck),
            ],
            half: f32::NAN,
            axis: rig.joints[chest].position,
        })
    }

    /// The heights a section is taken at, low to high.
    fn bands(&self) -> Vec<f32> {
        (0..=6)
            .map(|step| self.waist + (self.girdle - self.waist) * step as f32 / 6.0)
            .collect()
    }
}

/// How far the surface reaches forward from the trunk's axis at a point, or
/// `None` where the axis point itself is outside the body.
fn reach(mesh: &PolyMesh, across: f32, up: f32) -> Option<f32> {
    let from = Vec3::new(across, up, 0.0);
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, FAR);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from + Vec3::Z * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// How far out the section reaches before it stops being the TRUNK, in metres.
///
/// **Bounded by which bone the surface belongs to, not by where the body ends**
/// (#270). Walked to the body's edge instead, a section taken at girdle height
/// runs straight out along the shoulders and into the arms: it read a half of
/// 318 mm against the chest's 132, found three "peaks" 472 mm apart, and would
/// have reported a pair of deltoids as a chest. The selector is `nearest_bone`
/// on the surface point, which is the one [`symbios_avatar::rig::Surface`]
/// already uses to decide what a vertex belongs to.
fn half_width(mesh: &PolyMesh, rig: &Rig, up: f32) -> f32 {
    let mut across = 0.0f32;
    while across < 0.5 {
        let axis = Vec3::new(across, up, 0.0);
        if !mesh.contains(axis) {
            break;
        }
        let Some(forward) = reach(mesh, across, up) else {
            break;
        };
        if !is_trunk(rig, Vec3::new(across, up, forward)) {
            break;
        }
        across += STEP / 1000.0;
    }
    (across - STEP / 1000.0).max(0.0)
}

/// Whether a point on the surface belongs to the trunk rather than to a limb.
fn is_trunk(rig: &Rig, point: Vec3) -> bool {
    matches!(
        rig.joints[rig.nearest_bone(point).joint].zone,
        Zone::Pelvis | Zone::Abdomen | Zone::Chest
    )
}

/// The column's nodes beside what the surface actually delivered on the
/// midline.
///
/// **The column that matters is `on cage`, not `reach/r`.** A node's radius is
/// a circle and the trunk's rings are ellipses, so the depth the plan actually
/// asked for is `radius · scale.y` and the ratio worth knowing is how much of
/// THAT the limit surface delivers. Divided by the radius alone the number is a
/// mixture of the cage-to-surface loss and whatever the section's own
/// fore-and-aft multiple happens to be, and reads as if a chest were a third
/// shallower than its plan when the loss is under a tenth.
fn column(rig: &Rig, skeleton: &Skeleton, trunk: &Trunk, mesh: &PolyMesh, stature: f32) {
    println!("\n## The column — what the plan asked for, and what the surface came out at\n");
    println!(
        "| joint | y mm | node r mm | section | cage depth mm | midline reach mm | on cage | \
         of stature |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for &(name, joint) in &trunk.column {
        let at = rig.joints[joint].position.y;
        let radius = rig.joints[joint].radius;
        // **Through `Joint::node`, because a rig is not a skeleton in the same
        // order.** `Rig::from_skeleton` walks breadth-first from the pelvis, so
        // joint index and node index diverge from the second joint on; indexed
        // straight, this column reported the chest's section as 0.94 against
        // the plan's 0.74 and the girdle's as 1.00 against 1.10 — a ruler
        // reading the node next door, which is the trap this crate has now
        // caught twenty-one times.
        let scale = rig.joints[joint]
            .node
            .and_then(|node| skeleton.nodes.get(node as usize))
            .map_or(1.0, |node| node.scale.y);
        let asked = radius * scale;
        let forward = reach(mesh, 0.0, at);
        println!(
            "| {name} | {:.0} | {:.1} | {:.2} | {:.1} | {} | {} | {} |",
            at * 1000.0,
            radius * 1000.0,
            scale,
            asked * 1000.0,
            forward.map_or("—".into(), |f| format!("{:.1}", f * 1000.0)),
            forward.map_or("—".into(), |f| format!("{:.3}", f / asked)),
            forward.map_or("—".into(), |f| format!("{:.4}", f / stature)),
        );
    }
}

/// The forward reach down the midline, and whether it peaks anywhere in the
/// chest band.
///
/// **The reading epic #269 predicted and the one that says "tube" fastest.** A
/// pectoral or a bust is a local maximum of forward projection partway down the
/// chest; a lofted tube between a waist and a girdle has none, because a loft
/// between two ellipses is monotone in whatever the two disagree about.
fn midline(mesh: &PolyMesh, trunk: &Trunk, stature: f32) {
    println!("\n## Forward reach down the midline, waist to girdle\n");
    println!("| y mm | reach mm | of stature | slope mm/mm |");
    println!("|---|---|---|---|");
    let mut profile: Vec<(f32, f32)> = Vec::new();
    let mut up = trunk.waist;
    while up <= trunk.girdle + 1e-6 {
        if let Some(forward) = reach(mesh, 0.0, up) {
            profile.push((up, forward));
        }
        up += RISE / 1000.0;
    }
    for window in 0..profile.len() {
        let (up, forward) = profile[window];
        let slope = if window == 0 {
            f32::NAN
        } else {
            let (was_up, was) = profile[window - 1];
            (forward - was) / (up - was_up)
        };
        println!(
            "| {:.0} | {:.1} | {:.4} | {:+.3} |",
            up * 1000.0,
            forward * 1000.0,
            forward / stature,
            slope
        );
    }
    let peaks = prominent(&profile);
    match peaks.len() {
        0 | 1
            if peaks
                .first()
                .is_none_or(|&(at, _)| at <= trunk.waist + 1e-4 || at >= trunk.girdle - 1e-4) =>
        {
            println!(
                "\n  NO local maximum between the waist and the girdle: the front of the trunk \
                 is monotone through the whole chest. A chest has one."
            );
        }
        _ => {
            for &(at, forward) in &peaks {
                println!(
                    "\n  local maximum at y {:.0} mm ({:.2} of the way up the band), reach \
                     {:.1} mm",
                    at * 1000.0,
                    (at - trunk.waist) / (trunk.girdle - trunk.waist),
                    forward * 1000.0
                );
            }
        }
    }
    println!(
        "\n  the chest joint sits at {:.2} of the way up the band; life puts a pectoral's and \
         a bust's peak between 0.2 and 0.33 of the sternum below the notch",
        (trunk.chest - trunk.waist) / (trunk.girdle - trunk.waist)
    );
}

/// The section across, band by band.
fn sections(
    mesh: &PolyMesh,
    plain: &PolyMesh,
    rig: &Rig,
    trunk: &Trunk,
    cells: &Cells,
    stature: f32,
    lobe: Option<f32>,
) {
    println!("\n## The section across, flank to flank\n");
    println!(
        "| y mm | half mm | peaks | apart mm | off midline mm | dip mm | proud mm | of stature \
         | at x mm | cell mm | cells to flank |{}",
        if lobe.is_some() {
            " authored mm | delivered mm | survived |"
        } else {
            ""
        }
    );
    println!(
        "|---|---|---|---|---|---|---|---|---|---|---|{}",
        if lobe.is_some() { "---|---|---|" } else { "" }
    );
    for up in trunk.bands() {
        // Walked from flank to flank rather than out from the midline: two
        // lobes are a fact about the whole section, and a half-section cannot
        // report a separation.
        let section = Section::at(mesh, rig, up);
        let half = section.half;
        if half <= 0.0 {
            println!("| {:.0} | off the body |", up * 1000.0);
            continue;
        }
        let peaks = &section.peaks;
        let apart = match (peaks.first(), peaks.last()) {
            (Some(&(low, _)), Some(&(high, _))) if peaks.len() > 1 => high - low,
            _ => 0.0,
        };
        let off = peaks
            .iter()
            .fold(0.0f32, |most, &(at, _)| most.max(at.abs()));
        let dip = (section.crest - section.midline).max(0.0);
        let (proud, at) = ribcage_residual(mesh, &section.profile, up);
        let cell = cells.at(up);
        let mut row = format!(
            "| {:.0} | {:.0} | {} | {:.0} | {:.0} | {:.1} | {:+.2} | {:+.4} | {:.0} | {:.1} | \
             {:.1} |",
            up * 1000.0,
            half * 1000.0,
            peaks.len(),
            apart * 1000.0,
            off * 1000.0,
            dip * 1000.0,
            proud * 1000.0,
            proud / stature,
            at * 1000.0,
            cell * 1000.0,
            half / cell,
        );
        if let Some(height) = lobe {
            // The relief the mesh delivered against the relief that was asked
            // for, facesection's own subtraction: an independent second reading
            // of the same feature.
            let authored = lobe_field(trunk, height, off, up);
            let delivered = match (reach(mesh, off, up), reach(plain, off, up)) {
                (Some(with), Some(without)) => with - without,
                _ => f32::NAN,
            };
            // The ratio is only a ratio where there is something to divide by:
            // a band the lobe barely reaches divides two roundings and prints a
            // number that looks like a resolution finding and is arithmetic.
            row.push_str(&format!(
                " {:.1} | {:.1} | {} |",
                authored * 1000.0,
                delivered * 1000.0,
                if authored > 5e-4 {
                    format!("{:.2}", delivered / authored)
                } else {
                    "—".into()
                }
            ));
        }
        println!("{row}");
    }
}

/// How far the front of the section stands proud of the ribcage behind it, and
/// where.
///
/// **The reference is the body's OWN BACK at the same height, and the two
/// obvious alternatives are both refuted.** A projection measured against zero
/// says nothing. A projection measured against an ellipse fitted to the
/// section's own flanks was built first and is worse than nothing: a bust at
/// the intermammary distance life reports sits at 0.7 of the half-width, which
/// is inside any flank window wide enough to fit through, so the fit is dragged
/// forward by the very feature it is the reference for — measured, it turned a
/// 90 mm lobe into a residual of MINUS 35 mm, a deficit where the feature is.
///
/// The back is immune to that and needs no fitting at all. Every trunk ring in
/// this plan is a centred ellipse — only the neck uses `Node::offset` — so a
/// body with no chest is front-to-back symmetric by construction, and the
/// asymmetry IS the feature. What this reads on today's body is therefore the
/// instrument's own noise floor, and that number is printed rather than assumed.
fn ribcage_residual(mesh: &PolyMesh, profile: &[(f32, f32)], up: f32) -> (f32, f32) {
    profile
        .iter()
        .fold((f32::MIN, f32::NAN), |best, &(across, forward)| {
            let Some(behind) = depth_back(mesh, across, up) else {
                return best;
            };
            let residual = forward - behind;
            if residual > best.0 {
                (residual, across)
            } else {
                best
            }
        })
}

/// How far the surface reaches from the trunk's own axis at one azimuth, in
/// metres.
///
/// **The ruler a radially-carved body has to be read with, and `reach` at a
/// fixed `x` is not it.** `carve_chest` moves a vertex along the trunk's own
/// outward direction, which at the 51° the pair sits at is three quarters
/// sideways and one quarter forward — so the carve makes the section WIDER as
/// well as deeper, and a forward reach taken at a fixed `x` counts all of that
/// widening as relief. Measured on the neutral body at its peak's own height:
///
/// ```text
///   x mm     share of half    bare reach    carved reach    "relief"
///     92         0.70            76.2           92.9         +16.7
///    112         0.85            50.3           84.4         +34.1
///    124         0.94            21.4           56.3         +35.0
/// ```
///
/// There is no local maximum at the lobe at all: the number climbs all the way
/// to the flank, because out there the bare surface is running away from the
/// ray faster than the carve is pushing it. A crest found that way is the
/// silhouette edge, and it took the peak's height, both pole spans and the cut
/// with it.
///
/// Read along the push's own direction the arithmetic is what the mechanism
/// says it is: the relief at an azimuth is the displacement at that azimuth,
/// and it peaks where the lobe was placed.
fn radial(mesh: &PolyMesh, trunk: &Trunk, azimuth: f32, up: f32) -> Option<f32> {
    let from = Vec3::new(trunk.axis.x, up, trunk.axis.z);
    if !mesh.contains(from) {
        return None;
    }
    let angle = azimuth * std::f32::consts::FRAC_PI_2;
    let out = Vec3::new(angle.sin(), 0.0, angle.cos());
    let (mut inside, mut outside) = (0.0f32, FAR);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from + out * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// How far the surface reaches BACKWARD from the trunk's axis at a point.
fn depth_back(mesh: &PolyMesh, across: f32, up: f32) -> Option<f32> {
    let from = Vec3::new(across, up, 0.0);
    if !mesh.contains(from) {
        return None;
    }
    let (mut inside, mut outside) = (0.0f32, FAR);
    for _ in 0..HALVINGS {
        let mid = 0.5 * (inside + outside);
        if mesh.contains(from - Vec3::Z * mid) {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    Some(inside)
}

/// Local maxima of a profile that stand [`PROMINENCE`] above the dips beside
/// them, in order.
///
/// A plateau — a facet whose whole top is the maximum — is reported once, at its
/// middle, rather than as one peak per sample across it.
fn prominent(profile: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut peaks = Vec::new();
    let mut at = 0usize;
    while at < profile.len() {
        let height = profile[at].1;
        let mut end = at;
        while end + 1 < profile.len() && (profile[end + 1].1 - height).abs() <= f32::EPSILON {
            end += 1;
        }
        let rises_into = at == 0 || profile[at - 1].1 < height;
        let falls_out_of = end + 1 >= profile.len() || profile[end + 1].1 < height;
        if rises_into && falls_out_of {
            // The deeper of the two dips flanking it, which is what prominence
            // means: a shoulder on the side of a bigger peak is not a peak.
            let dip = |range: &mut dyn Iterator<Item = &(f32, f32)>| {
                range
                    .take_while(|&&(_, other)| other <= height)
                    .fold(height, |low, &(_, other)| low.min(other))
            };
            let before = dip(&mut profile[..at].iter().rev());
            let after = dip(&mut profile[end + 1..].iter());
            if height - before.max(after) >= PROMINENCE {
                peaks.push((0.5 * (profile[at].0 + profile[end].0), height));
            }
        }
        at = end + 1;
    }
    peaks
}

/// The synthetic lobe's own field: a paired forward bump, in metres.
///
/// Gaussian in both axes and mirrored about the midline, so the pair is one
/// expression and the sternum between them is what two Gaussians leave rather
/// than a third term. See [`LOBE`] for the geometry and the module note for why
/// this is a displacement of the finished mesh and not a mechanism.
fn lobe_field(trunk: &Trunk, height: f32, across: f32, up: f32) -> f32 {
    let span = trunk.girdle - trunk.waist;
    let (centre, wide, along, tall) = LOBE;
    let peak_up = trunk.waist + span * along;
    let half = trunk.half;
    let sideways = (across.abs() - centre * half) / (wide * half);
    let vertical = (up - peak_up) / (tall * span);
    height * (-(sideways * sideways) - vertical * vertical).exp()
}

/// A copy of the body with [`lobe_field`] displaced onto its front.
///
/// Only vertices already on the front of the trunk move, and they move along
/// `+Z`, which is the axis every reading here is taken along.
fn lobe_onto(mesh: &PolyMesh, trunk: &Trunk, height: f32) -> PolyMesh {
    let mut lobed = mesh.clone();
    for point in &mut lobed.positions {
        if point.z <= 0.0 || point.y < trunk.waist || point.y > trunk.girdle {
            continue;
        }
        point.z += lobe_field(trunk, height, point.x, point.y);
    }
    lobed
}

/// Median cell size on the front of the trunk, band by band.
struct Cells {
    /// Band height, its median edge, and the medians of the edges that run
    /// across the trunk and down it taken apart, low to high.
    ///
    /// **Three numbers rather than one, and `examples/refinecost`'s own
    /// docstring is why**: a body's faces are not square, a refinement that
    /// halves EVERY edge moves the balance between two populations rather than
    /// the median over both, and a band whose cell went from 3.4 across and 7.2
    /// down to 1.7 and 3.6 reads `3.53 -> 3.58` as one median. The combined
    /// figure stays because the section table has always printed it and it is a
    /// ledger; the refinement probe reads the two apart, because halving is the
    /// whole thing it is being asked about.
    bands: Vec<(f32, f32, f32, f32)>,
}

impl Cells {
    /// Measures the front of the trunk between the waist and the girdle.
    ///
    /// Front-facing edges only, and inside the chest's own column rather than
    /// across the whole trunk: a median taken round the back reports the ring
    /// spacing of a surface no chest feature is drawn on, which is the trap
    /// `facesection` documents for the cheek.
    fn of(mesh: &PolyMesh, trunk: &Trunk) -> Self {
        let mut bands: Vec<(f32, Vec<f32>, Vec<f32>)> = trunk
            .bands()
            .into_iter()
            .map(|band| (band, Vec::new(), Vec::new()))
            .collect();
        let gap = (trunk.girdle - trunk.waist) / 12.0;
        for face in &mesh.faces {
            for pair in 0..face.len() {
                let a = mesh.positions[face[pair] as usize];
                let b = mesh.positions[face[(pair + 1) % face.len()] as usize];
                let middle = 0.5 * (a + b);
                if middle.z <= 0.0 || middle.x.abs() > trunk.half {
                    continue;
                }
                if let Some(slot) = bands
                    .iter_mut()
                    .find(|(height, _, _)| (middle.y - *height).abs() < gap)
                {
                    // An edge runs ACROSS the trunk or DOWN it by which way it
                    // travels furthest, which on a swept ring surface is the
                    // ring's own direction against the loft's.
                    let edge = b - a;
                    if edge.y.abs() >= edge.x.abs().max(edge.z.abs()) {
                        slot.2.push(a.distance(b));
                    } else {
                        slot.1.push(a.distance(b));
                    }
                }
            }
        }
        let median = |edges: &mut Vec<f32>| {
            edges.sort_by(f32::total_cmp);
            edges.get(edges.len() / 2).copied().unwrap_or(f32::NAN)
        };
        Self {
            bands: bands
                .into_iter()
                .map(|(height, mut across, mut down)| {
                    let mut both: Vec<f32> = across.iter().chain(&down).copied().collect();
                    (
                        height,
                        median(&mut both),
                        median(&mut across),
                        median(&mut down),
                    )
                })
                .collect(),
        }
    }

    /// The band nearest a height.
    fn band(&self, up: f32) -> Option<&(f32, f32, f32, f32)> {
        self.bands
            .iter()
            .min_by(|a, b| (a.0 - up).abs().total_cmp(&(b.0 - up).abs()))
    }

    /// The median cell nearest a height, over every edge.
    fn at(&self, up: f32) -> f32 {
        self.band(up).map_or(f32::NAN, |&(_, cell, _, _)| cell)
    }

    /// The median cell nearest a height, across the trunk and down it.
    fn apart(&self, up: f32) -> (f32, f32) {
        self.band(up)
            .map_or((f32::NAN, f32::NAN), |&(_, _, across, down)| (across, down))
    }
}

/// One section, read flank to flank: how far out the trunk goes, the profile
/// itself, its prominent maxima, its crest and what the midline reads under
/// them.
struct Section {
    half: f32,
    profile: Vec<(f32, f32)>,
    peaks: Vec<(f32, f32)>,
    crest: f32,
    midline: f32,
}

impl Section {
    /// The section at one height.
    ///
    /// One method because three readers want the same walk and a second copy of
    /// it would be a second definition of "the crest" — which is exactly how a
    /// ledger and the instrument that prints it drift apart.
    fn at(mesh: &PolyMesh, rig: &Rig, up: f32) -> Section {
        let half = half_width(mesh, rig, up);
        let mut profile: Vec<(f32, f32)> = Vec::new();
        let mut across = -half;
        while across <= half {
            if let Some(forward) = reach(mesh, across, up) {
                profile.push((across, forward));
            }
            across += STEP / 1000.0;
        }
        let peaks = prominent(&profile);
        let crest = profile.iter().fold(0.0f32, |most, &(_, f)| most.max(f));
        let midline = profile
            .iter()
            .find(|&&(at, _)| at.abs() < STEP / 2000.0)
            .map_or(0.0, |&(_, f)| f);
        Section {
            half,
            profile,
            peaks,
            crest,
            midline,
        }
    }
}

/// The three numbers epic #269 was raised to get, said once.
///
/// Printed after the tables rather than instead of them: a reader who wants to
/// know whether the trunk has a chest should not have to read a grid to find
/// out, and a reader who doubts the answer needs the grid it came from.
fn verdict(mesh: &PolyMesh, plain: &PolyMesh, rig: &Rig, trunk: &Trunk) {
    let up = trunk.waist + (trunk.girdle - trunk.waist) * LOBE.2;
    let section = Section::at(mesh, rig, up);
    let (proud, _) = ribcage_residual(mesh, &section.profile, up);
    // The relief the carve actually delivered: the same surface with and
    // without it, which is what life's "chest wall to nipple" is comparable to.
    // `proud` is the mechanism-independent shape reading and this is the
    // feature's own size; the first runs about twice the second, because a
    // radial displacement lands a vertex where the ribcage behind it was
    // shallower.
    let relief = section
        .profile
        .iter()
        .fold(0.0f32, |most, &(across, forward)| {
            most.max(forward - reach(plain, across, up).unwrap_or(forward))
        });
    println!("\n## At the height a chest peaks, {:.0} mm\n", up * 1000.0);
    println!(
        "  relief over the bare body {:+.2} mm  (life: pectoral 10-20, bust 40-90)",
        relief * 1000.0
    );
    println!("  proud of its own back   {:+.2} mm", proud * 1000.0);
    println!("  sides the section shows {}", section.peaks.len());
    println!(
        "  separation between them {:.0} mm",
        match (section.peaks.first(), section.peaks.last()) {
            (Some(&(low, _)), Some(&(high, _))) if section.peaks.len() > 1 => (high - low) * 1000.0,
            _ => 0.0,
        }
    );
    println!(
        "  sternum below the crest {:.2} mm",
        (section.crest - section.midline).max(0.0) * 1000.0
    );
}

/// The lobe's own vertical profile: the largest relief over the bare body at
/// each height, and where round the section that largest relief sits.
///
/// **Measured radially** — see [`radial`], whose note is the whole reason this
/// is not a walk across `x`.
///
/// **One side only.** The carve is mirrored about the midline by construction —
/// it reads `from.x.abs()` — and so is the synthetic control, so walking the
/// far flank as well would spend bisections re-measuring a reflection.
///
/// Sampled at [`CREST_STEP`] rather than [`STEP`] because what this wants from
/// each height is an argmax and not a shape.
fn crest(surface: &PolyMesh, plain: &PolyMesh, trunk: &Trunk) -> Vec<(f32, f32, f32)> {
    // The azimuth step that puts a sample every `CREST_STEP` round the section
    // at the chest, so the resolution of the search is the one the constant
    // names whatever the trunk's own size is doing.
    let step = (CREST_STEP / 1000.0) / (trunk.half.max(f32::EPSILON) * std::f32::consts::FRAC_PI_2);
    let mut profile = Vec::new();
    let mut up = trunk.waist;
    while up <= trunk.girdle + 1e-6 {
        let mut best = (0.0f32, PAIR.0);
        let mut azimuth = PAIR.0;
        while azimuth <= PAIR.1 {
            if let (Some(with), Some(without)) = (
                radial(surface, trunk, azimuth, up),
                radial(plain, trunk, azimuth, up),
            ) && with - without > best.0
            {
                best = (with - without, azimuth);
            }
            azimuth += step;
        }
        profile.push((up, best.0, best.1));
        up += RISE / 1000.0;
    }
    profile
}

/// The lobe's vertical profile as the carve AUTHORS it, sampled as finely as
/// the reading is rather than as coarsely as the mesh is.
///
/// **Probed through `carve_chest` itself rather than by writing its formula out
/// again, which would make the control a copy of the thing it controls.** The
/// carve moves positions and reads nothing else, so a `PolyMesh` carrying
/// nothing but a grid of probe points is a legal argument to it, and what comes
/// back is the displacement it would have given a vertex there. The points sit
/// on the bare surface so that `nearest_bone`'s zone gate — the carve's own
/// first act — answers for them the way it answers for the body.
///
/// **This is the control the synthetic `--lobe` cannot be, and the reason is
/// the one [`radial`] gives.** `--lobe` displaces the finished mesh along `+Z`;
/// the carve displaces it outward. Read radially, a `+Z` lobe's relief is not
/// its own field but its field times whatever the local surface is doing, and
/// the control came back at 58:42, 52:48 and 43:57 on a shape that is symmetric
/// by construction. It is still the control for the SECTION readings above,
/// which are taken along `+Z` and always were.
fn authored(
    plain: &PolyMesh,
    rig: &Rig,
    trunk: &Trunk,
    traits: &ChestTraits,
) -> Vec<(f32, f32, f32)> {
    let step = (CREST_STEP / 1000.0) / (trunk.half.max(f32::EPSILON) * std::f32::consts::FRAC_PI_2);
    let mut probe = PolyMesh::new();
    let mut at: Vec<(f32, f32)> = Vec::new();
    let mut up = trunk.waist;
    while up <= trunk.girdle + 1e-6 {
        let mut azimuth = PAIR.0;
        while azimuth <= PAIR.1 {
            if let Some(out) = radial(plain, trunk, azimuth, up) {
                let angle = azimuth * std::f32::consts::FRAC_PI_2;
                probe.push_vertex(Vec3::new(
                    trunk.axis.x + out * angle.sin(),
                    up,
                    trunk.axis.z + out * angle.cos(),
                ));
                at.push((up, azimuth));
            }
            azimuth += step;
        }
        up += RISE / 1000.0;
    }
    let was = probe.positions.clone();
    carve_chest(&mut probe, rig, traits);

    let mut profile: Vec<(f32, f32, f32)> = Vec::new();
    for (index, &(up, azimuth)) in at.iter().enumerate() {
        let moved = probe.positions[index].distance(was[index]);
        match profile.last_mut() {
            Some(last) if (last.0 - up).abs() < f32::EPSILON => {
                if moved > last.1 {
                    *last = (up, moved, azimuth);
                }
            }
            _ => profile.push((up, moved, azimuth)),
        }
    }
    profile
}

/// Where across the section an azimuth lands on the surface, in metres off the
/// midline.
///
/// The cut every silhouette reading is taken down is a SAGITTAL plane through
/// the nipple, which is what the plastic-surgery literature's pole profiles are
/// drawn on, so the crest's azimuth has to be turned back into an `x` before it
/// can be one.
fn abscissa(mesh: &PolyMesh, trunk: &Trunk, azimuth: f32, up: f32) -> f32 {
    let out = radial(mesh, trunk, azimuth, up).unwrap_or(0.0);
    trunk.axis.x + out * (azimuth * std::f32::consts::FRAC_PI_2).sin()
}

/// The silhouette down one vertical line through the lobe: absolute forward
/// reach against height, at a fixed distance off the midline.
///
/// The absolute surface and not the relief, because every anatomical claim
/// about a pole's shape, a fold's depth or a border's crispness is stated on
/// the outline a side view shows — which is the ribcage AND the lobe on it.
///
/// **Bounded by which bone the surface belongs to, exactly as [`half_width`]
/// is, and the bare control is what caught it needing to be** (#270's trap, in
/// a second place). The lobe's own cut runs at about 92 mm off the midline on
/// the reference body, and 92 mm out at girdle height is a DELTOID: undamped,
/// the uncarved trunk read a 19 mm notch under its own peak and an upper-pole
/// bow of +11 mm, neither of which is a rib. Gated, the same body reads what a
/// ribcage reads.
fn cut(surface: &PolyMesh, rig: &Rig, trunk: &Trunk, across: f32) -> Vec<(f32, f32)> {
    let mut profile = Vec::new();
    let mut up = trunk.waist;
    while up <= trunk.girdle + 1e-6 {
        if let Some(forward) = reach(surface, across, up)
            && is_trunk(rig, Vec3::new(across, up, forward))
        {
            profile.push((up, forward));
        }
        up += CUT / 1000.0;
    }
    profile
}

/// A profile read between its own samples, or `None` outside it.
fn at(profile: &[(f32, f32)], up: f32) -> Option<f32> {
    let window = profile.windows(2).find(|pair| {
        let (low, high) = (pair[0].0, pair[1].0);
        (low..=high).contains(&up)
    })?;
    let (low, high) = (window[0], window[1]);
    let span = high.0 - low.0;
    if span <= f32::EPSILON {
        return Some(low.1);
    }
    Some(low.1 + (high.1 - low.1) * (up - low.0) / span)
}

/// Everything the shape questions read off one vertical cut.
///
/// **Taken twice on every body — once on the carved surface and once on the
/// bare one under it at the same `x` and over the same spans — because these
/// four are read on the SILHOUETTE and a silhouette is the ribcage plus the
/// lobe.** The control run says why: a synthetic lobe that is symmetric by
/// construction bowed `+2.28` mm above its peak and `+0.04` below, and every
/// bit of that difference is the trunk, which is convex where it climbs to the
/// girdle and flat where it falls to the waist. Differencing the two would
/// answer a different question — the anatomy's straight upper pole is a claim
/// about the outline a side view shows, not about the carve's contribution to
/// it — so both are printed and the reader subtracts whichever way the question
/// runs.
struct Cut {
    /// Mean signed distance from the silhouette to its own pole's chord, upper
    /// then lower, in metres. Out is positive.
    bow: (f32, f32),
    /// How deep the deepest notch below the peak is and how far below the peak
    /// it sits, in metres.
    fold: (f32, f32),
    /// The largest `|d reach / dy|` below the peak, and where.
    border: (f32, f32),
    /// How far the crest stands above the midline at the peak's height.
    dip: f32,
    /// How far the midline sits inside the chord across it, at the peak's
    /// height — the sternal groove, which [`GROOVE`] explains the need for.
    groove: f32,
    /// How many sides the section shows there.
    sides: usize,
}

impl Cut {
    /// Reads one surface down a line through `across`, between two pole ends.
    fn of(
        mesh: &PolyMesh,
        rig: &Rig,
        trunk: &Trunk,
        across: f32,
        peak: f32,
        span: (f32, f32),
    ) -> Self {
        let profile = cut(mesh, rig, trunk, across);
        let (high, low) = span;

        // The chord across each pole. A profile that bulges past its own chord
        // is convex and one that falls inside it is hollow, which is what the
        // anatomy's "straight to slightly concave upper pole" is a statement
        // about.
        let bow = |from: f32, to: f32| -> f32 {
            let (low, high) = (from.min(to), from.max(to));
            let (Some(start), Some(finish)) = (at(&profile, low), at(&profile, high)) else {
                return f32::NAN;
            };
            let inside: Vec<&(f32, f32)> = profile
                .iter()
                .filter(|&&(up, _)| (low..=high).contains(&up))
                .collect();
            if high - low <= f32::EPSILON || inside.is_empty() {
                return f32::NAN;
            }
            inside
                .iter()
                .map(|&&(up, forward)| {
                    forward - (start + (finish - start) * (up - low) / (high - low))
                })
                .sum::<f32>()
                / inside.len() as f32
        };

        // The fold. A crease sits on a ramp that is already descending, so it
        // is not a local minimum and no body has one; what it is, is how far
        // the surface falls inside a chord laid across it.
        let notch = NOTCH / 1000.0;
        let base = SLOPE_BASE / 1000.0;
        let mut fold = (0.0f32, 0.0f32);
        let mut border = (0.0f32, 0.0f32);
        for &(up, forward) in &profile {
            if up >= peak || up - notch < trunk.waist {
                continue;
            }
            if let (Some(under), Some(over)) = (at(&profile, up - notch), at(&profile, up + notch))
            {
                let depth = 0.5 * (under + over) - forward;
                if depth > fold.0 {
                    fold = (depth, peak - up);
                }
            }
        }
        // **Inside the lower pole and not merely below the peak, because the
        // ribcage is steeper than a pectoral is.** Read over the whole cut, the
        // maximum slope is the trunk's own fall toward the waist: at femininity
        // -1 and 5% fat the carved body reported 0.481 against a bare 0.481, to
        // the digit, on a lobe standing 5.7 mm off its own ribcage. A border is
        // the edge of a mass and it lives at that mass's own lower edge, so the
        // window is the lower pole — which is where #286's fold sits too, at
        // 1.4 of the span against a pole ending at 1.5.
        for &(up, _) in &profile {
            if !(low..peak).contains(&up) {
                continue;
            }
            if let (Some(under), Some(over)) = (at(&profile, up - base), at(&profile, up + base)) {
                let slope = ((over - under) / (2.0 * base)).abs();
                if slope > border.0 {
                    border = (slope, up);
                }
            }
        }

        let section = Section::at(mesh, rig, peak);
        // The groove, read across the section the way the fold is read down the
        // cut: how far the surface falls inside a chord laid over it.
        let across = |at: f32| {
            section
                .profile
                .windows(2)
                .find(|pair| (pair[0].0..=pair[1].0).contains(&at))
                .map(|pair| {
                    let span = pair[1].0 - pair[0].0;
                    if span <= f32::EPSILON {
                        pair[0].1
                    } else {
                        pair[0].1 + (pair[1].1 - pair[0].1) * (at - pair[0].0) / span
                    }
                })
        };
        let chord = GROOVE / 1000.0;
        let groove = match (across(-chord), across(0.0), across(chord)) {
            (Some(left), Some(middle), Some(right)) => 0.5 * (left + right) - middle,
            _ => f32::NAN,
        };
        Self {
            bow: (bow(peak, high), bow(low, peak)),
            fold,
            border,
            dip: (section.crest - section.midline).max(0.0),
            groove,
            sides: section.peaks.len(),
        }
    }

    /// The row's own columns, so the carved reading and its control are printed
    /// by one piece of code and cannot come out in different units.
    fn row(&self) -> String {
        format!(
            "{:+.2} | {:+.2} | {:.2} | {:.0} | {:.3} | {:.2} | {:.2} | {}",
            self.bow.0 * 1000.0,
            self.bow.1 * 1000.0,
            self.fold.0 * 1000.0,
            self.fold.1 * 1000.0,
            self.border.0,
            self.dip * 1000.0,
            self.groove * 1000.0,
            self.sides,
        )
    }
}

/// The pole arithmetic on one vertical profile of a lobe.
///
/// One type because it is run twice on every body — on what the surface
/// DELIVERS and on what the carve AUTHORS — and two copies of it would be two
/// definitions of a pole, which is how a reading and its own control come to
/// disagree for reasons that are nobody's geometry.
struct Poles {
    /// Where the lobe peaks, and how much relief it has there.
    peak: (f32, f32),
    /// How far round the section it peaks.
    azimuth: f32,
    /// Where each pole ends, upper then lower, in metres.
    ends: (f32, f32),
    /// The share of the lobe's own area below the peak.
    lower_share: f32,
}

impl Poles {
    /// Splits a crest profile at its peak and shares the area either side.
    ///
    /// **The peak and both pole ends are interpolated between samples rather
    /// than snapped to them, and the control is why.** Read at the sample grid,
    /// a lobe symmetric by construction came back at 48:52, 46:54 and 52:48 on
    /// three bodies — four points of scatter on a reading whose whole job is
    /// telling 45:55 from 50:50 — and 51:49, 50:50, 51:49 interpolated. The
    /// peak is a parabola through the largest sample and its two neighbours and
    /// each pole ends where the relief crosses its own floor between two
    /// samples, which costs no bisections at all: the error was never in the
    /// surface, only in where the ruler's marks happened to fall on it.
    ///
    /// `None` where there is no lobe to read — the floor is [`PROMINENCE`]'s
    /// own millimetre, below which there is nothing to call a peak and every
    /// reading here is anchored to one.
    fn of(crest: &[(f32, f32, f32)]) -> Option<Self> {
        let relief: Vec<(f32, f32)> = crest.iter().map(|&(up, high, _)| (up, high)).collect();
        let (top, &(_, _, azimuth)) = crest
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.1.total_cmp(&b.1.1))?;
        if crest[top].1 < PROMINENCE {
            return None;
        }
        let (peak, height) = vertex(&relief, top);

        // Each pole ends where the relief falls to a share of its own peak, so
        // the two are truncated at the same HEIGHT and a symmetric lobe reads
        // symmetric whatever the band around it is shaped like.
        let floor = height * POLE_END;
        let mut above = top;
        while above + 1 < crest.len() && crest[above + 1].1 >= floor {
            above += 1;
        }
        let mut below = top;
        while below > 0 && crest[below - 1].1 >= floor {
            below -= 1;
        }
        let cross = |inside: usize, outside: Option<usize>| -> f32 {
            let Some(outside) = outside.map(|at| relief[at]) else {
                return relief[inside].0;
            };
            let inside = relief[inside];
            let fall = inside.1 - outside.1;
            if fall.abs() <= f32::EPSILON {
                return inside.0;
            }
            inside.0 + (outside.0 - inside.0) * (inside.1 - floor) / fall
        };
        let high = cross(above, (above + 1 < relief.len()).then_some(above + 1));
        let low = cross(below, below.checked_sub(1));
        let (upper, lower) = (
            integrate(&relief, peak, high),
            integrate(&relief, low, peak),
        );
        Some(Self {
            peak: (peak, height),
            azimuth,
            ends: (high, low),
            lower_share: lower / (upper + lower).max(f32::EPSILON),
        })
    }

    /// The row's own crest columns.
    fn row(&self) -> String {
        format!(
            "{:.1} | {:.0} | {:.0} | {:.0} | {:.0}:{:.0}",
            self.peak.1 * 1000.0,
            self.peak.0 * 1000.0,
            (self.ends.0 - self.peak.0) * 1000.0,
            (self.peak.0 - self.ends.1) * 1000.0,
            (1.0 - self.lower_share) * 100.0,
            self.lower_share * 100.0,
        )
    }
}

/// Everything the shape questions read off one body.
struct Shape {
    /// The poles the SURFACE delivered.
    delivered: Poles,
    /// The poles the carve AUTHORED, where there is a carve — see
    /// [`authored`].
    authored: Option<Poles>,
    /// The silhouette readings on the carved body, and the same four on the
    /// bare one under it.
    carved: Cut,
    bare: Cut,
}

impl Shape {
    /// Reads all of it off one built body, or `None` where there is no lobe to
    /// read.
    fn of(body: &Body) -> Option<Self> {
        let Body {
            surface,
            plain,
            trunk,
            rig,
            traits,
            ..
        } = body;
        let delivered = Poles::of(&crest(surface, plain, trunk))?;
        // The sagittal plane through the crest, which is where the silhouette
        // readings are taken — see [`abscissa`].
        let across = abscissa(surface, trunk, delivered.azimuth, delivered.peak.0);
        let span = delivered.ends;
        let peak = delivered.peak.0;
        Some(Self {
            authored: traits
                .as_ref()
                .and_then(|traits| Poles::of(&authored(plain, rig, trunk, traits))),
            delivered,
            carved: Cut::of(surface, rig, trunk, across, peak, span),
            bare: Cut::of(plain, rig, trunk, across, peak, span),
        })
    }
}

/// Where a sampled profile really peaks, from the largest sample and the two
/// beside it.
///
/// The parabola through three evenly spaced samples, which is the standard
/// sub-sample peak and is exact for the Gaussian's own top to second order.
/// Falls back to the sample itself at either end of the profile or where the
/// three are collinear.
fn vertex(profile: &[(f32, f32)], top: usize) -> (f32, f32) {
    let here = profile[top];
    let (Some(&before), Some(&after)) = (
        top.checked_sub(1).and_then(|at| profile.get(at)),
        profile.get(top + 1),
    ) else {
        return here;
    };
    let bend = before.1 - 2.0 * here.1 + after.1;
    if bend >= -f32::EPSILON {
        return here;
    }
    let shift = 0.5 * (before.1 - after.1) / bend;
    (
        here.0 + shift * (after.0 - before.0) * 0.5,
        here.1 - 0.25 * (before.1 - after.1) * shift,
    )
}

/// The area under a sampled profile between two heights, ends included.
///
/// Trapezoidal over the samples, with the two partial cells at the ends read
/// off [`at`] rather than dropped: a pole's own end is where the relief crosses
/// its floor, which is almost never on a sample.
fn integrate(profile: &[(f32, f32)], from: f32, to: f32) -> f32 {
    let (low, high) = (from.min(to), from.max(to));
    let (Some(start), Some(finish)) = (at(profile, low), at(profile, high)) else {
        return 0.0;
    };
    let mut edges = vec![(low, start)];
    edges.extend(
        profile
            .iter()
            .filter(|&&(up, _)| (low..=high).contains(&up))
            .copied(),
    );
    edges.push((high, finish));
    edges
        .windows(2)
        .map(|pair| 0.5 * (pair[0].1 + pair[1].1) * (pair[1].0 - pair[0].0))
        .sum()
}

/// The two profiles every shape question is derived from, printed.
///
/// **Behind a flag but not optional to have**, which is the same rule the
/// section tables are held to: a summary nobody can get the profile back out of
/// is a summary nobody can check, and three ledgers in this crate went stale
/// because the number was computed and thrown away. `--profile` prints it.
fn profiles(body: &Body) {
    let crest = crest(&body.surface, &body.plain, &body.trunk);
    let Some(&(peak, _, azimuth)) = crest.iter().max_by(|a, b| a.1.total_cmp(&b.1)) else {
        return;
    };
    let across = abscissa(&body.surface, &body.trunk, azimuth, peak);
    let cut = cut(&body.surface, &body.rig, &body.trunk, across);
    let bare = self::cut(&body.plain, &body.rig, &body.trunk, across);
    println!(
        "\n## The crest and the cut, at femininity {:+.2}, cut at x {:.0} mm\n",
        body.record.composites.femininity,
        across * 1000.0
    );
    println!(
        "| y mm | from peak mm | crest relief mm | crest azimuth | cut reach mm | bare reach mm |"
    );
    println!("|---|---|---|---|---|---|");
    println!(
        "\n  and across the section at the peak, where [`PAIR`] bounds the search to \
         {:.0}–{:.0} mm:\n",
        PAIR.0 * body.trunk.half * 1000.0,
        PAIR.1 * body.trunk.half * 1000.0
    );
    println!("| x mm | share of half | bare reach mm | carved reach mm | relief mm |");
    println!("|---|---|---|---|---|");
    let mut across = 0.0f32;
    while across <= body.trunk.half {
        if let (Some(with), Some(without)) = (
            reach(&body.surface, across, peak),
            reach(&body.plain, across, peak),
        ) {
            println!(
                "| {:.0} | {:.2} | {:.1} | {:.1} | {:+.1} |",
                across * 1000.0,
                across / body.trunk.half,
                without * 1000.0,
                with * 1000.0,
                (with - without) * 1000.0
            );
        }
        across += CREST_STEP / 1000.0;
    }
    println!(
        "\n| y mm | from peak mm | crest relief mm | crest azimuth | cut reach mm | bare reach mm |"
    );
    println!("|---|---|---|---|---|---|");
    for &(up, relief, at) in &crest {
        let read = |profile: &[(f32, f32)]| {
            self::at(profile, up).map_or("—".into(), |forward| format!("{:.1}", forward * 1000.0))
        };
        println!(
            "| {:.0} | {:+.0} | {:.2} | {:.2} | {} | {} |",
            up * 1000.0,
            (up - peak) * 1000.0,
            relief * 1000.0,
            at,
            read(&cut),
            read(&bare),
        );
    }
}

/// The four shape questions, each at its own control.
///
/// **The femininity sweep is not a flag** — see [`CONTROLS`]. Everything else
/// the invocation asked for is held, so `--fat 0.45 --band ...` compares three
/// chests on one soft body rather than three bodies.
fn shape(seed: i64, over: &Overrides, bands: &[Band]) {
    println!(
        "\n## The shape questions, each at femininity {:+.0} / {:+.0} / {:+.0}\n",
        CONTROLS[0], CONTROLS[1], CONTROLS[2]
    );
    println!(
        "| femininity | crest mm | peak y mm | upper mm | lower mm | poles up:down | upper bow \
         mm | lower bow mm | fold mm | seat below peak mm | border slope | sternum dip mm | \
         groove mm | sides |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for femininity in CONTROLS {
        let over = Overrides {
            femininity: Some(femininity),
            ..*over
        };
        // Built rather than borrowed even where the primary body already sits
        // at this femininity: a control that is sometimes the reading it
        // controls is a control nobody can check.
        let Some(built) = build(seed, &over, bands) else {
            continue;
        };
        let Some(shape) = Shape::of(&built) else {
            println!("| {femininity:+.2} | no lobe to read |");
            continue;
        };
        let femininity = built.record.composites.femininity;
        println!(
            "| {femininity:+.2} | {} | {} |",
            shape.delivered.row(),
            shape.carved.row()
        );
        println!(
            "| {femininity:+.2} bare | — | — | — | — | — | {} |",
            shape.bare.row()
        );
        if let Some(authored) = &shape.authored {
            println!(
                "| {femininity:+.2} authored | {} | — | — | — | — | — | — | — | — |",
                authored.row()
            );
        }
    }
    println!(
        "\n  the `bare` row is the same four silhouette readings on the UNCARVED body under \
         that chest, over the same cut and the same spans: whatever it says, the ribcage said, \
         not the carve. the `authored` row is what the carve asked for, probed through \
         `carve_chest` itself at the reading's own resolution: what it and the top row \
         disagree about, the mesh ate\n\n  life: poles 45:55, an upper pole straight to slightly concave and \
         a lower one convex, a fold about 70 mm below the peak, and a masculine chest defined \
         by a crisp inferior border rather than by how far it stands off"
    );
}

/// One candidate refinement pass over the trunk.
///
/// **Written in the carve's own language and not in `refine_face`'s.** The face
/// bands are a cosine of the azimuth off an unsectioned head with heights in
/// two units either side of a joint; `torso::carve_chest` places its lobe by a
/// share of a quarter turn from dead ahead, at a height between the waist joint
/// and the girdle. A band that has to hold resolution UNDER a shape had better
/// be measured in the same numbers as the shape, or the two disagree about
/// where the lobe is on every body whose proportions differ from the one the
/// band was written on.
#[derive(Clone, Copy)]
struct Band {
    /// Nearest the midline, as a share of a quarter turn: `0` is dead ahead.
    near: f32,
    /// Furthest round toward the flank, in the same share.
    far: f32,
    /// Lowest, from the waist joint at `0` to the girdle at `1`.
    low: f32,
    /// Highest, in the same span.
    high: f32,
}

impl Band {
    /// Every `--band near,far,low,high` on the command line, in order.
    fn all(args: &[String]) -> Vec<Self> {
        args.iter()
            .enumerate()
            .filter(|(_, arg)| *arg == "--band")
            .filter_map(|(at, _)| args.get(at + 1))
            .filter_map(|spec| Self::parse(spec))
            .collect()
    }

    /// One `near,far,low,high`, or `None` if it is not four numbers.
    fn parse(spec: &str) -> Option<Self> {
        let parts: Vec<f32> = spec
            .split(',')
            .filter_map(|part| part.parse().ok())
            .collect();
        match parts[..] {
            [near, far, low, high] => Some(Self {
                near,
                far,
                low,
                high,
            }),
            _ => {
                println!("  --band wants four numbers, near,far,low,high — ignoring `{spec}`");
                None
            }
        }
    }

    /// Which faces this band selects.
    ///
    /// The gate is the carve's own: `Zone::Chest | Zone::Abdomen` through
    /// `nearest_bone`, so an arm crossing the band is never refined. The carve
    /// solved that exact problem for its vertices and the answer does not
    /// change for a face centroid.
    fn selects(&self, mesh: &PolyMesh, rig: &Rig, trunk: &Trunk) -> Vec<bool> {
        (0..mesh.face_count())
            .map(|face| {
                let at = mesh.face_centroid(face);
                if !matches!(
                    rig.joints[rig.nearest_bone(at).joint].zone,
                    Zone::Chest | Zone::Abdomen
                ) {
                    return false;
                }
                let from = Vec3::new(at.x - trunk.axis.x, 0.0, at.z - trunk.axis.z);
                if from.length_squared() <= f32::EPSILON {
                    return false;
                }
                let azimuth = from.x.abs().atan2(from.z) / std::f32::consts::FRAC_PI_2;
                let up = (at.y - trunk.waist) / (trunk.girdle - trunk.waist);
                (self.near..=self.far).contains(&azimuth) && (self.low..=self.high).contains(&up)
            })
            .collect()
    }

    /// Refines what this band selects, in place, and reports what that cost.
    ///
    /// The two predictions are printed beside the measurement rather than
    /// instead of it, because the gap between them IS the reading: see
    /// [`boundary`] for what the perimeter costs and why a napkin cannot know
    /// it.
    fn apply(self, mesh: &mut PolyMesh, rig: &Rig, trunk: &Trunk) -> PassCost {
        let selected = self.selects(mesh, rig, trunk);
        let faces = selected.iter().filter(|&&on| on).count();
        let quads = mesh
            .faces
            .iter()
            .zip(&selected)
            .filter(|&(face, &on)| on && face.len() == 4)
            .count();
        // Σ(n + 2) over the selected faces is what they cost between them: an
        // n-gon that splits becomes n quads, which is 2n triangles where it was
        // n − 2.
        let corners: usize = mesh
            .faces
            .iter()
            .zip(&selected)
            .filter(|&(_, &on)| on)
            .map(|(face, _)| face.len() + 2)
            .sum();
        let perimeter = boundary(mesh, &selected);
        let was = triangles(mesh);
        *mesh = mesh.refine_curved(&selected);
        let now = triangles(mesh);
        // **At the band's OWN mid-height, not at the lobe's.** Read at a fixed
        // height a tight pass low on the trunk reports the cell of a region it
        // never touched: the fold strip at 0.13–0.51 moved this column by 1.3
        // mm when read at the peak and by the factor of two it actually buys
        // when read at its own middle. The median is still taken across the
        // whole front of the trunk, so a pass narrow in AZIMUTH moves it only
        // by the share of that front it covers.
        let up = trunk.waist + (trunk.girdle - trunk.waist) * 0.5 * (self.low + self.high);
        PassCost {
            band: self,
            selected: (faces, quads),
            perimeter,
            predicted: (6 * quads, corners + perimeter),
            spent: (now - was, now),
            cell: {
                let cells = Cells::of(mesh, trunk);
                let (across, down) = cells.apart(up);
                (cells.at(up), across, down)
            },
        }
    }
}

/// Triangles a mesh draws.
fn triangles(mesh: &PolyMesh) -> usize {
    mesh.faces
        .iter()
        .map(|face| face.len().saturating_sub(2))
        .sum()
}

/// How many edges of a selection have an unselected face on the other side.
///
/// **This is the whole gap between the napkin arithmetic and the bill.** A
/// selected quad becomes four quads and pays six triangles; an UNSELECTED face
/// absorbs every midpoint a selected neighbour put on a shared edge — which is
/// how `PolyMesh::refine_curved` stays conforming without cracking — and pays
/// one triangle for each. So the band's own perimeter is a cost, and a band
/// edge that lands along a ring of faces rather than across it buys a whole
/// row of them.
fn boundary(mesh: &PolyMesh, selected: &[bool]) -> usize {
    let mut sides: HashMap<(u32, u32), (bool, bool)> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        let chosen = selected.get(index).copied().unwrap_or(false);
        for corner in 0..face.len() {
            let (a, b) = (face[corner], face[(corner + 1) % face.len()]);
            let key = (a.min(b), a.max(b));
            let slot = sides.entry(key).or_insert((false, false));
            if chosen {
                slot.0 = true;
            } else {
                slot.1 = true;
            }
        }
    }
    sides
        .values()
        .filter(|&&(with, without)| with && without)
        .count()
}

/// What the candidate pass set cost and what resolution it bought.
///
/// Nothing here is scaled from another body: the passes were applied in order
/// to the body every reading above was then taken on, because the cost is
/// quantised by ring and a costing taken off a different surface goes stale the
/// moment anything moves the one it will run on.
fn costing(body: &Body) {
    if body.cost.is_empty() {
        return;
    }
    println!("\n## What the refinement passes cost, measured\n");
    println!(
        "| pass | azimuth | band | faces | quads | perimeter | +6/quad | + the perimeter | \
         measured | total tris | cell mm | across mm | down mm |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (index, pass) in body.cost.iter().enumerate() {
        println!(
            "| {} | {:.2}–{:.2} | {:.2}–{:.2} | {} | {} | {} | {} | {} | {} | {} | {:.1} | {:.1} \
             | {:.1} |",
            index + 1,
            pass.band.near,
            pass.band.far,
            pass.band.low,
            pass.band.high,
            pass.selected.0,
            pass.selected.1,
            pass.perimeter,
            pass.predicted.0,
            pass.predicted.1,
            pass.spent.0,
            pass.spent.1,
            pass.cell.0 * 1000.0,
            pass.cell.1 * 1000.0,
            pass.cell.2 * 1000.0,
        );
    }
}
