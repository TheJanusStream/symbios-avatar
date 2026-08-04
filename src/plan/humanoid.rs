//! The upright biped body plan.
//!
//! Nine semantic axes. Each drives several skeleton quantities, and the
//! correlations between them are written out longhand below rather than fitted:
//! `build` thickens the torso *and* the limbs, `limb_length` raises the pelvis
//! *and* therefore shortens the torso at a fixed stature, and so on.
//!
//! Several derived lengths are floored at a multiple of the joint radius they
//! serve. Those floors are not styling — they are what keeps every point of the
//! parameter space meshable (see the module docs for [`super`]).
//!
//! # Where these numbers came from
//!
//! Every coefficient below carries a provenance tag; the four categories and
//! why they are worth the trouble are in the crate docs (#52). Where a value
//! here is **looked up**, the source is always the eight-head figure of
//! academic figure drawing, tabulated as `CANON` in `examples/measure` and
//! compared there against the *rendered* body.
//!
//! **Most of this file is unsourced, and that is the finding rather than an
//! apology.** `git blame` puts about thirty of the forty coefficients here in
//! the initial commits: the shape of the default body is largely a first guess
//! that has survived by never being contradicted. What holds them in place
//! today is not a source, it is `tests/plan.rs` — which proves they *mesh*, and
//! proving a body meshes says nothing at all about whether it is the right
//! body. An overhaul should treat the tagged minority as load-bearing and the
//! rest as open.
//!
//! The one thing to read before touching anything is the note on `clavicle_x`
//! in [`HumanoidParams::skeleton`], where a comment claiming the default body
//! sat *exactly* on canon was made false by a change in another file that never
//! touched this one.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::{
    BodyPlan, Category, PlanDecodeError, Rolls, put_length, put_signed, put_unit, take_length,
    take_signed, take_unit,
};
use super::{Limb, Zone};
use crate::skeleton::{Node, Skeleton};

/// Smallest and largest stature this plan accepts, in metres.
pub const HEIGHT_RANGE: (f32, f32) = (1.2, 2.2);

/// Cross-sections of the torso, as `(lateral, fore-and-aft)` multiples of the
/// node's radius.
///
/// Without these every node is a circle and the body is a **surface of
/// revolution** — the chest measured perfectly round, which is much of what
/// separates this silhouette from a character's. A real torso is a flattened
/// box: broad across the shoulders and shallow front to back, and that ratio is
/// what the eye reads as a ribcage.
///
/// The ring frame is parallel-transported from a world-up reference, so on an
/// upright body `x` is broadly lateral and `y` runs fore-and-aft; see
/// [`crate::skeleton::Node::scale`].
///
/// **Every one of these flattens rather than widens, and that is not a
/// compromise.** A joint refuses to mesh when two of its sockets overlap, and
/// the clearance a socket demands is its *largest* half-extent — so shrinking an
/// axis is free and growing one is not. Widening the chest to reach the same
/// ratio was tried first and cost the girdle its clavicle clearance at ordinary
/// parameters, which `tests/plan.rs` caught on the first run. Flattening also
/// happens to be the honest anatomy: a torso's width comes from the shoulder
/// girdle, which `shoulder_width` already drives, and its depth is what makes it
/// read as a ribcage rather than a drum. The front silhouette every radius here
/// was tuned against is untouched.
///
/// Provenance: **tuned by render** (commit `f54f38a`), all five of them, and
/// bounded from below by `tests/plan.rs` rather than by a source. The
/// three-quarters figure quoted for the chest is a remembered rule of thumb and
/// not a citation — if this set is ever revisited it should be measured, not
/// re-remembered.
const PELVIS_SECTION: Vec2 = Vec2::new(1.0, 0.80);
/// See [`PELVIS_SECTION`]. The waist is the shallowest part of the trunk.
const WAIST_SECTION: Vec2 = Vec2::new(1.0, 0.76);
/// See [`PELVIS_SECTION`]. A ribcage is about three-quarters as deep as it is
/// wide, which is the strongest single cue in the set.
const CHEST_SECTION: Vec2 = Vec2::new(1.0, 0.74);
/// See [`PELVIS_SECTION`].
const GIRDLE_SECTION: Vec2 = Vec2::new(1.0, 0.80);
/// See [`PELVIS_SECTION`]. A neck is very nearly round, but not quite.
const NECK_SECTION: Vec2 = Vec2::new(1.0, 0.94);

/// Where the crown node sits above the head joint, in head radii.
///
/// See [`CROWN_WIDE`]: this came down from 0.72 when that went up, so the built
/// crown lands in the same place. The pair is chosen together and neither
/// number means anything alone.
///
/// Provenance: **derived** from [`CROWN_WIDE`] (#79). The arithmetic is that
/// the built crown height must not move while the node widens, so the two were
/// swept together and the pair that held the measured crown fixed was kept.
const CROWN_HIGH: f32 = 0.68;

/// How wide the crown node is, in head radii.
///
/// **Up from 0.66, and it is the cage's half of #79.** A cranium is very nearly
/// a cylinder in front view — its greatest breadth is on the parietal, 25 to
/// 45 mm above the pupil line, not at the cheekbone — and at 0.66 this node was
/// so much smaller than the head node below it that the blend between the two
/// converged toward an apex. Measured on the built cage, with no profile
/// applied at all, the half-width fell from 0.584 head radii at the joint to
/// 0.407 at +0.75 R: a cone before the skull's breadth profile ever ran.
///
/// At 0.87 the same measurement reads 0.620 / 0.632 / 0.630 / 0.599 / 0.513 at
/// +0.05 / +0.20 / +0.35 / +0.55 / +0.75 R — flat through the mid-cranium and
/// falling only where a skull does. [`CROWN_HIGH`] came down from 0.72 to 0.68
/// at the same time so the built crown does not move: this issue is the vault's
/// SHAPE, and cranium:face measures 1.00 after #78, so there is no proportion
/// left to spend on height.
///
/// Provenance: **looked up, then tuned by render** (#79) — the only coefficient
/// in this file with a stated anthropometric premise underneath it. The premise
/// is that maximum head breadth is at eurion, high on the parietal roughly 25
/// to 45 mm above the pupil line, and exceeds bizygomatic breadth (156 mm
/// against 137). The value itself was swept against the built half-widths,
/// which is what the figures above are.
const CROWN_WIDE: f32 = 0.87;

/// How far below horizontal a resting arm lies, in radians.
///
/// The A in A-pose. Forty degrees: far enough that neither hanging the arms nor
/// raising them to horizontal asks for a rotation big enough to tear the
/// shoulder, and shallow enough that the armpit still opens up for the mesher.
///
/// Provenance: **derived** from the worst-case skinning rotation. A T-pose
/// arm posed down to walk turns the shoulder about 75°, which bulges under both
/// dual quaternions and matrices; halving that in each direction is what picks
/// 40°, and it is why production models are built this way. The floor under it
/// is the mesher's, not taste: the armpit has to stay open.
const A_POSE: f32 = 0.70;

/// Parameters describing one biped.
///
/// Axes run `-1..=1` unless noted, with `0` the neutral middle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HumanoidParams {
    /// Standing height in metres, within [`super::humanoid_height_range`].
    #[serde(with = "super::scaled")]
    pub height: f32,
    /// Overall mass, from slight to heavy.
    #[serde(with = "super::scaled")]
    pub build: f32,
    /// Musculature, `0..=1`.
    #[serde(with = "super::scaled")]
    pub muscle: f32,
    /// Width of the shoulder girdle.
    #[serde(with = "super::scaled")]
    pub shoulder_width: f32,
    /// Width of the pelvis.
    #[serde(with = "super::scaled")]
    pub hip_width: f32,
    /// Limb length relative to the torso; longer limbs raise the pelvis and so
    /// shorten the torso at a fixed stature.
    #[serde(with = "super::scaled")]
    pub limb_length: f32,
    /// Neck length.
    #[serde(with = "super::scaled")]
    pub neck_length: f32,
    /// Head size.
    #[serde(with = "super::scaled")]
    pub head_size: f32,
    /// Hand and foot size.
    #[serde(with = "super::scaled")]
    pub extremity_size: f32,
}

/// Neutral stature, used when a record omits the field.
fn default_height() -> f32 {
    1.75
}

impl Default for HumanoidParams {
    fn default() -> Self {
        Self {
            height: default_height(),
            build: 0.0,
            muscle: 0.0,
            shoulder_width: 0.0,
            hip_width: 0.0,
            limb_length: 0.0,
            neck_length: 0.0,
            head_size: 0.0,
            extremity_size: 0.0,
        }
    }
}

impl HumanoidParams {
    /// How much thicker than neutral this body is.
    ///
    /// Mass and musculature both add girth, and girth feeds every torso and
    /// limb radius — that single correlation is most of what makes `build` read
    /// as one coherent slider rather than a dozen independent ones.
    ///
    /// Provenance: **unsourced**, both gains, from the initial body plan. What
    /// is defensible here is the *structure* — one girth factor feeding every
    /// radius, so the axes cannot drift apart — rather than the ±28% and +15%
    /// it spans. Nothing has ever measured whether a heavy body is 28% thicker
    /// than a neutral one.
    fn girth(&self) -> f32 {
        1.0 + 0.28 * self.build + 0.15 * self.muscle
    }
}

impl BodyPlan for HumanoidParams {
    fn sanitize(&mut self) {
        // Fallbacks come from `Default` rather than being written out again, so
        // they cannot drift from the documented defaults. See
        // [`super::sanitize_axis`] for why the guard has to precede the clamp.
        let default = Self::default();
        self.height = super::sanitize_axis(self.height, default.height, HEIGHT_RANGE);
        self.muscle = super::sanitize_axis(self.muscle, default.muscle, (0.0, 1.0));
        for (axis, fallback) in [
            (&mut self.build, default.build),
            (&mut self.shoulder_width, default.shoulder_width),
            (&mut self.hip_width, default.hip_width),
            (&mut self.limb_length, default.limb_length),
            (&mut self.neck_length, default.neck_length),
            (&mut self.head_size, default.head_size),
            (&mut self.extremity_size, default.extremity_size),
        ] {
            *axis = super::sanitize_axis(*axis, fallback, (-1.0, 1.0));
        }
    }

    fn skeleton(&self) -> Skeleton {
        let h = self.height;
        let girth = self.girth();

        // The torso carries a separate shoulder girdle above the ribcage. That
        // is not decoration: a single node carrying spine, neck, and both arms
        // needs every one of those sockets to clear the others, and the room
        // that takes scales with its own girth — so a chest wide enough to read
        // as a chest forces a neck long enough to read as a giraffe. Splitting
        // the two lets the ribcage be broad while the girdle above it stays
        // slim, which is what shortens the neck.
        // Provenance: **unsourced**, every figure on these four lines, from the
        // initial body plan. They are fractions of stature that nobody has
        // compared to a stature-fraction table; `examples/measure` prints the
        // built radii as `radius/H` beside the canon column precisely so that
        // comparison can be made, and it never has been. The two
        // `shoulder_width` gains are unsourced too, and note they are gains on a
        // RADIUS while the shoulder breadth anyone would check is set by
        // `clavicle_x` below — so this axis moves the torso and the girdle by
        // different amounts and no measurement ties them together.
        let pelvis_r = h * 0.079 * girth;
        let waist_r = h * 0.078 * girth;
        let chest_r = h * 0.088 * girth * (1.0 + 0.08 * self.shoulder_width);
        let girdle_r = h * 0.062 * girth * (1.0 + 0.06 * self.shoulder_width);
        // A neck is a good deal narrower than the skull above it. At the old
        // figure it measured WIDER than the head — 0.098 m against 0.093 — which
        // reads as a tree trunk and, worse, swallows the jaw: the chin is shaped
        // and narrows properly, but a neck two and a half times its width leaves
        // nothing of it to see.
        // Provenance: **tuned by render** (commit `0d7684f`). The evidence is
        // the paragraph above — a measured 0.098 m neck against a 0.093 m head —
        // which is the right shape of argument for a tuned number: a comparison
        // that came out the wrong way round.
        let neck_r = h * 0.038 * girth;
        // Provenance: **unsourced**, both the 0.075 and the 0.25 gain. 0.075 of
        // stature is close to the eight-head figure's head, but the eight-head
        // figure specifies head HEIGHT and this is a node RADIUS, so the
        // resemblance is not a derivation and must not be written up as one —
        // the built head is 214 mm tall on a 160 mm breadth (#79), which is not
        // a number this coefficient predicts.
        let head_r = h * 0.075 * (1.0 + 0.25 * self.head_size);

        // Provenance: **unsourced**, from the very first commit. Compare
        // `examples/measure`, whose canon column has no ankle row, so nothing
        // checks this even indirectly.
        let ankle_y = h * 0.0686;
        // Provenance: **unsourced**. This one is load-bearing in a way the
        // others are not — it sets how far the hip sockets sit below the pelvis
        // node, so it trades against `hip_x` for the room the pelvis needs to
        // separate three sockets, and `hip_x` is a meshability floor (below).
        let hip_drop = pelvis_r * 1.85;

        // Only the pelvis and the girdle are joints; the waist and chest between
        // them are connectors and constrain nothing.
        // Provenance: **unsourced** multipliers serving a **derived** purpose.
        // The purpose is exact — a joint's sockets must clear each other, so the
        // spine has to leave the pelvis and reach the girdle with room to spare
        // — but the 1.5, the 1.3 and the 0.06 that express it were picked rather
        // than solved for. Whether each is the tightest value that meshes is
        // unknown; `hip_x` and `clavicle_x` below are the two that were actually
        // swept, and both turned out to be sitting well above their floors.
        let pelvis_gap = pelvis_r * 1.5;
        let chest_gap = girdle_r * 1.3;
        let torso_min = pelvis_gap + h * 0.06 + chest_gap;

        // Provenance: **unsourced**. Nearest canon landmark is the chest at
        // 0.720 of height (`examples/measure`), which this is not: the girdle
        // built from it measures 0.675, and the two are different points on the
        // body, so the 0.045 gap printed there is not evidence of an error.
        let nominal_girdle = h * 0.755;
        // Provenance: **unsourced** (the 0.5, the 0.03 gain and the 0.10 floor).
        // The canon puts the pelvis at 0.545 of height and the built one lands
        // at 0.492, but do not read that as this coefficient being 0.045 low:
        // the 0.5 is nominal and both `min` and `max` here can override it, so
        // what is printed is the outcome of a three-way clamp rather than of
        // this number.
        //
        // **On the default body the nominal 0.5 does not bind — the `min` does.**
        // Hand-evaluated: 0.875 nominal against a `min` of 0.8678 and a `max` of
        // 0.5508, so the pelvis is pulled 7.2 mm DOWN to leave the torso its
        // minimum length, landing at 0.4959 h. And because that term binds
        // exactly, `girdle_y` falls out at `pelvis_y + torso_min` = the nominal
        // girdle to the last decimal, so its `max` is a tie rather than a
        // clearance. Raising `nominal_girdle` or shrinking `torso_min` therefore
        // moves the pelvis, which is not what either name suggests. And raising
        // the 0.5 does nothing whatever — the `min` caps it at any value — while
        // lowering it does nothing until it passes below 0.4959. An overhaul
        // that edits this number and watches the render will conclude the
        // coefficient is inert, and be half right for the wrong reason.

        let pelvis_y = (h * (0.5 + 0.03 * self.limb_length))
            .min(nominal_girdle - torso_min)
            .max(ankle_y + hip_drop + h * 0.10);
        let girdle_y = nominal_girdle.max(pelvis_y + torso_min);
        let chest_y = girdle_y - chest_gap;
        // Provenance: **derived** — the waist is the midpoint of pelvis and
        // chest, which is a definition rather than a measurement. The 0.02
        // clearance that keeps it off the chest is **unsourced**.
        let waist_y = (pelvis_y + (chest_y - pelvis_y) * 0.5)
            .clamp(pelvis_y + pelvis_gap, chest_y - h * 0.02);

        // The girdle is a joint, so its neck socket has to clear the clavicles'.
        // The neck above it is a plain connector and constrains nothing — an
        // earlier floor here was invented rather than required, and it alone
        // added half a head-height of giraffe.
        // The neck has to clear the girdle's socket, but the floor was doing all
        // the work: a neck sitting exactly as high as the sockets allow leaves
        // barely any column between the collar and the jaw.
        // Provenance: **tuned by render** (commit `0d7684f`), and the note above
        // is what tuning it looked like — an earlier floor here was invented
        // rather than required and cost the body half a head-height of giraffe.
        // The 1.32 socket floor BINDS on the default body (143.2 mm against a
        // nominal 126.0, measured in #78), so what ships is the floor and not
        // the 0.072: same trap as `pelvis_y` above, one line apart.
        let neck_y = girdle_y + (h * 0.072 * (1.0 + 0.3 * self.neck_length)).max(girdle_r * 1.32);
        // How far the head reaches below its own joint, and it is a HEAD measure
        // rather than a stature one. This was `(h * 0.052).max(head_r * 0.45)`,
        // where the second term can never bind — 0.45 of a head radius is at most
        // 0.042 h against the first term's 0.052 — so the whole lower face was a
        // stature constant. Two things followed. The head_size axis moved the
        // crown 55 mm and left the chin at −64.6 mm both times; and the entire
        // jaw, chin and throat transition had 91 mm to happen in, of which 27 was
        // throat. The lower face measured 39% short of the canon and read, in the
        // owner's words, as mangled (#78).
        //
        // `face::skull::shape` already scales its whole below-joint domain so the
        // head's measured floor lands at JUNCTION, so this coefficient stretches
        // every profile below the joint and moves nothing above it. The chin is
        // 0.7097 of the floor, measured to 0.1 mm on four seeds, so it lands at
        // 0.78 R below the joint here.
        //
        // Derived rather than picked, and derived TWICE — the first attempt is
        // recorded because it is the kind of mistake this coefficient invites.
        // The chain is: the head's own floor is a fixed share of this extent,
        // the chin is 0.7097 of that floor, and the crown sits 0.806 R above the
        // eye line. Measured over four seeds the floor is 0.895 of the extent
        // (0.890 to 0.904) and the crown ratio is 0.806 (0.8058 to 0.8067), both
        // stable to under 2%. So the eye-to-chin frame is R x (0.05 + 0.6352 K),
        // and setting it equal to the crown's 0.806 R — a cranium:face of 1.0,
        // which is life's — gives K = 1.19.
        //
        // Predicted 1.11 first, by assuming the floor was the whole extent. It is
        // not: ownership stops about a ninth short of the neck node, and that one
        // wrong factor put the frame 12 mm under target.
        //
        // The frame still lands at 92% of the canon's absolute 114.7 mm rather
        // than 100%, and the missing 8% is not here: it is that the crown itself
        // is 8% short of life once subdivision has collapsed the cap (#79).
        // Chasing the absolute figure from this end would buy it by making the
        // face too long for its own cranium.
        //
        // RAISED rather than dropping `neck_y` to hold stature, which is what #78
        // asked for and cannot be done: `neck_y` sits exactly on its girdle-socket
        // floor above (143.2 mm against a nominal 126.0), so lowering it breaks
        // `tests/plan.rs`'s meshability sweep. Raising is sound because a built
        // body already stands about 6% under its nominal stature — the crown
        // collapses under subdivision — so this spends height the body was
        // already missing.
        // Provenance: **derived** (#78), and the whole chain is written out
        // above rather than summarised — which is what this tag is supposed to
        // mean. It is the one coefficient in this file whose first derivation
        // was recorded alongside its correction.
        const HEAD_BELOW_JOINT: f32 = 1.19;
        let head_y = neck_y + head_r * HEAD_BELOW_JOINT;

        // The pelvis carries the spine and both legs; the drop to the hip is
        // what gives that joint the room to separate three sockets.
        //
        // The coefficient is a MESHABILITY FLOOR, not a style choice, and the
        // difference matters because the canon is on the far side of it. A
        // default body measured 0.270 of height across the hips against a canon
        // 0.190, and #66 asked for canon. Swept: 1.60 and 1.45 mesh every seed,
        // 1.30 loses one, and 1.13 — the figure that actually lands on 0.190 —
        // loses five of ten. The two leg sockets and the spine's have to clear
        // each other on one node, and bringing the legs together is what takes
        // that room away. 1.35 is the tightest value with margin over the floor
        // and gives 0.228, which is most of the way and as far as this joint
        // goes. Reaching canon needs a narrower pelvis or sockets placed
        // differently, and both are changes to the body rather than to a number.
        //
        // Provenance: **looked up, then bounded by a sweep** (#66). The looked-up
        // half is the eight-head figure's 0.190 of height across the hips; the
        // sweep is 1.60 / 1.45 / 1.35 / 1.30 / 1.13 against ten seeds, and it is
        // why the canon figure is not what ships. The 0.35 gain is
        // **unsourced**. See the note on `clavicle_x` below before trusting the
        // 0.228 quoted here: it was measured against a body that has since grown.
        let hip_x = pelvis_r * (1.35 + 0.35 * self.hip_width);
        let hip_y = pelvis_y - hip_drop;

        // Provenance: **unsourced**. The canon has a knee row — 0.285 of height
        // — and the built knee measures 0.227, the largest single deviation
        // `examples/measure` prints. Whether the 0.60 or the pelvis above it is
        // responsible is undetermined, because the knee is placed as a fraction
        // of the hip-to-ankle span and the hip is itself clamped.
        let knee_y = ankle_y + (hip_y - ankle_y) * 0.60;
        // Provenance: **unsourced**, all three figures.
        let foot_y = h * 0.0257;
        let foot_z = h * 0.057 * (1.0 + 0.3 * self.extremity_size);

        // The clavicle has to reach past the chest socket's corners before an
        // arm can attach — the single tightest constraint on the whole body.
        //
        // 1.85 put the default body exactly on the canonical 0.245 of height
        // across the shoulders, down from 0.285 (#66). Unlike the hips this one
        // had room: 1.85 meshes every seed and only 1.70 starts to lose them.
        //
        // Provenance: **looked up, then tuned by render** (#66) — the eight-head
        // figure's 0.245, reached by sweep and confirmed by `examples/measure`.
        //
        // **AND IT NO LONGER HOLDS, which is the whole argument of #52 happening
        // to this file.** That sentence is written in the past tense above
        // because the default body now measures 0.235, and NOTHING HERE MOVED.
        // The head overhaul (#78, #79) grew the rendered body from 1.639 m to
        // 1.705 m, and every one of these figures is a fraction of rendered
        // height, so all three fell by that same 4.03% at once:
        //
        // ```text
        //   shoulders  0.245 -> 0.235   0.245 x 1.639/1.705 = 0.2355
        //   hips       0.228 -> 0.219   0.228 x 1.639/1.705 = 0.2192
        //   arm span   0.930 -> 0.894   0.930 x 1.639/1.705 = 0.8940
        // ```
        //
        // Three independent quantities predicted to the printed precision by one
        // ratio, so this is the denominator moving and not a regression in any
        // coefficient. It is exactly the shape of the `FIFTH`/`PUPIL` defect:
        // a number calibrated against another number, with nothing recording the
        // dependency, so a change somewhere else falsifies a comment here
        // silently. `tests/plan.rs` still passes because its tolerance is 0.015
        // and the error is 0.010 — two thirds of the margin spent, no warning.
        //
        // Whether to re-tune for the taller body is the body overhaul's call and
        // an owner one: it is a change to the silhouette, and arm span at 0.894
        // against a canon 1.000 is the bigger deviation of the two.
        let clavicle_x = girdle_r * (1.85 + 0.25 * self.shoulder_width);
        // Provenance: **unsourced**, both.
        let clavicle_y = girdle_y + h * 0.004;
        let shoulder_x = clavicle_x + h * 0.048;
        // Arms hang at an angle, not straight out. Built in a T-pose, posing
        // them down to walk rotates each shoulder about 75 degrees and the
        // shoulder bulges — measured the same under dual quaternions and under
        // matrices, so it is the size of the rotation and not the skinning. An
        // A-pose roughly halves the worst case in both directions, down to a
        // hanging arm and up to a raised one, and is why production models are
        // built this way.
        let arm = Vec3::new(A_POSE.cos(), -A_POSE.sin(), 0.0);
        // Lengthened with the shoulders, not independently of them. Arm span is
        // measured fingertip to fingertip, so it carries the shoulder span
        // inside it: narrowing the clavicles for #66 took the body from 0.929 of
        // height to 0.890, moving one canon figure by breaking another. These
        // put it back to 0.930. The remaining 7% against a canon 1.000 was there
        // before and is left alone — closing it means arms about a fifth longer,
        // which is a change to the silhouette and wants deciding on its own.
        //
        // Provenance: **derived** (#66) — solved backwards from arm span, which
        // is the measurement, rather than picked as segment lengths. Same caveat
        // as `clavicle_x`: the 0.930 they were solved for now reads 0.894, and
        // for the same reason. The 0.025 gains are **unsourced**.
        let upper_arm = h * (0.123 + 0.025 * self.limb_length);
        let forearm = h * (0.110 + 0.025 * self.limb_length);
        // Provenance: **unsourced**, and note it feeds arm span through
        // `hand_at` below — so the extremity axis moves a figure that `#66`
        // tuned, and nothing connects the two.
        let hand_len = h * 0.040 * (1.0 + 0.3 * self.extremity_size);
        let shoulder_at = Vec3::new(shoulder_x, clavicle_y, 0.0);
        let elbow_at = shoulder_at + arm * upper_arm;
        let wrist_at = elbow_at + arm * forearm;
        let hand_at = wrist_at + arm * hand_len;

        let extremity = 1.0 + 0.3 * self.extremity_size;

        // Provenance for every node radius below — clavicle 0.040, shoulder
        // 0.038, elbow 0.032, wrist 0.025, hand 0.020, hip 0.052, knee 0.042,
        // ankle 0.030, foot 0.019, all times stature: **unsourced**, from the
        // initial body plan, nine numbers in one ladder. They are also the set
        // most likely to look right and be wrong, because a limb tapering
        // monotonically from hip to foot reads as a limb whatever the actual
        // figures are — there is no silhouette cue that a wrong taper violates.
        // `examples/measure` prints `radius/H` for the zones it knows, which is
        // the instrument to point at them if anyone ever does.
        let mut skeleton = Skeleton::new();
        let pelvis = skeleton.add_node(
            Node::new(Vec3::new(0.0, pelvis_y, 0.0), pelvis_r)
                .with_scale(PELVIS_SECTION)
                .in_zone(Zone::Pelvis),
        );
        let waist = skeleton.extend_from(
            pelvis,
            Node::new(Vec3::new(0.0, waist_y, 0.0), waist_r)
                .with_scale(WAIST_SECTION)
                .in_zone(Zone::Abdomen),
        );
        let chest = skeleton.extend_from(
            waist,
            Node::new(Vec3::new(0.0, chest_y, 0.0), chest_r)
                .with_scale(CHEST_SECTION)
                .in_zone(Zone::Chest),
        );
        let girdle = skeleton.extend_from(
            chest,
            Node::new(Vec3::new(0.0, girdle_y, 0.0), girdle_r)
                .with_scale(GIRDLE_SECTION)
                .in_zone(Zone::Chest),
        );
        let neck = skeleton.extend_from(
            girdle,
            Node::new(Vec3::new(0.0, neck_y, 0.0), neck_r)
                .with_scale(NECK_SECTION)
                .in_zone(Zone::Neck),
        );
        // A skull takes two nodes. One leaf gives a capped tube whose dome
        // collapses under subdivision, leaving a flat-topped stub with the head
        // joint sitting at the very top of the body — which is exactly what a
        // measured rendering showed. A crown above it fills the cranium out.
        let head = skeleton.extend_from(
            neck,
            Node::new(Vec3::new(0.0, head_y, 0.0), head_r).in_zone(Zone::Head),
        );
        skeleton.extend_from(
            head,
            Node::new(
                Vec3::new(0.0, head_y + head_r * CROWN_HIGH, 0.0),
                head_r * CROWN_WIDE,
            )
            .in_zone(Zone::Head),
        );

        for (side, fore, hind) in [
            (-1.0f32, Limb::ForeLeft, Limb::HindLeft),
            (1.0, Limb::ForeRight, Limb::HindRight),
        ] {
            // Arms rest in a T-pose: VRM 1.0 requires it of exported humanoids.
            let clavicle = skeleton.extend_from(
                girdle,
                Node::new(
                    Vec3::new(side * clavicle_x, clavicle_y, 0.0),
                    h * 0.040 * girth,
                )
                .in_zone(Zone::Chest),
            );
            let shoulder = skeleton.extend_from(
                clavicle,
                Node::new(
                    Vec3::new(side * shoulder_at.x, shoulder_at.y, shoulder_at.z),
                    h * 0.038 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let elbow = skeleton.extend_from(
                shoulder,
                Node::new(
                    Vec3::new(side * elbow_at.x, elbow_at.y, elbow_at.z),
                    h * 0.032 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let wrist = skeleton.extend_from(
                elbow,
                Node::new(
                    Vec3::new(side * wrist_at.x, wrist_at.y, wrist_at.z),
                    h * 0.025 * girth,
                )
                .in_zone(Zone::LowerLimb(fore)),
            );
            skeleton.extend_from(
                wrist,
                // Slim: this is the base of the hand, not a stand-in for one.
                // While the limbs ended in these nodes they were fattened to
                // read as a fist and a boot, and now that real hands and feet
                // hang off them a blob only pokes through the part it is meant
                // to be inside.
                Node::new(
                    Vec3::new(side * hand_at.x, hand_at.y, hand_at.z),
                    h * 0.020 * extremity,
                )
                .in_zone(Zone::Extremity(fore)),
            );

            let hip = skeleton.extend_from(
                pelvis,
                Node::new(Vec3::new(side * hip_x, hip_y, 0.0), h * 0.052 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let knee = skeleton.extend_from(
                hip,
                Node::new(Vec3::new(side * hip_x, knee_y, 0.0), h * 0.042 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let ankle = skeleton.extend_from(
                knee,
                Node::new(Vec3::new(side * hip_x, ankle_y, 0.0), h * 0.030 * girth)
                    .in_zone(Zone::LowerLimb(hind)),
            );
            skeleton.extend_from(
                ankle,
                Node::new(
                    Vec3::new(side * hip_x, foot_y, foot_z),
                    h * 0.019 * extremity,
                )
                .in_zone(Zone::Extremity(hind)),
            );
        }

        skeleton
    }

    fn reroll(&mut self, category: Category, rolls: &Rolls) {
        match category {
            Category::Stature => {
                self.height = rolls.range("humanoid.height", HEIGHT_RANGE.0, HEIGHT_RANGE.1);
            }
            Category::Build => {
                self.build = rolls.range("humanoid.build", -1.0, 1.0);
                self.muscle = rolls.range("humanoid.muscle", 0.0, 1.0);
            }
            Category::Frame => {
                self.shoulder_width = rolls.range("humanoid.shoulderWidth", -1.0, 1.0);
                self.hip_width = rolls.range("humanoid.hipWidth", -1.0, 1.0);
            }
            Category::Proportions => {
                self.limb_length = rolls.range("humanoid.limbLength", -1.0, 1.0);
                self.neck_length = rolls.range("humanoid.neckLength", -1.0, 1.0);
            }
            Category::Features => {
                self.head_size = rolls.range("humanoid.headSize", -1.0, 1.0);
                self.extremity_size = rolls.range("humanoid.extremitySize", -1.0, 1.0);
            }
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        put_length(out, self.height);
        put_signed(out, self.build);
        put_unit(out, self.muscle);
        put_signed(out, self.shoulder_width);
        put_signed(out, self.hip_width);
        put_signed(out, self.limb_length);
        put_signed(out, self.neck_length);
        put_signed(out, self.head_size);
        put_signed(out, self.extremity_size);
    }

    fn decode(bytes: &mut &[u8]) -> Result<Self, PlanDecodeError> {
        let mut params = Self {
            height: take_length(bytes)?,
            build: take_signed(bytes)?,
            muscle: take_unit(bytes)?,
            shoulder_width: take_signed(bytes)?,
            hip_width: take_signed(bytes)?,
            limb_length: take_signed(bytes)?,
            neck_length: take_signed(bytes)?,
            head_size: take_signed(bytes)?,
            extremity_size: take_signed(bytes)?,
        };
        params.sanitize();
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::NodeKind;

    #[test]
    fn the_neutral_body_has_the_expected_topology() {
        let skeleton = HumanoidParams::default().skeleton();
        skeleton.validate().expect("valid skeleton");

        // The pelvis carries the spine and two legs; the shoulder girdle carries
        // the spine, the neck, and both arms. The chest between them is a plain
        // connector, which is the whole reason the girdle exists.
        assert_eq!(skeleton.kind(0), NodeKind::Joint);
        assert_eq!(skeleton.degree(0), 3, "pelvis");
        assert_eq!(skeleton.kind(2), NodeKind::Connector, "chest");
        assert_eq!(skeleton.degree(3), 4, "shoulder girdle");

        // Two hands, two feet, one head: five leaves.
        let leaves = (0..skeleton.nodes.len() as u32)
            .filter(|&node| skeleton.kind(node) == NodeKind::Leaf)
            .count();
        assert_eq!(leaves, 5);
    }

    #[test]
    fn stature_scales_the_whole_body() {
        let short = HumanoidParams {
            height: 1.3,
            ..Default::default()
        }
        .skeleton();
        let tall = HumanoidParams {
            height: 2.1,
            ..Default::default()
        }
        .skeleton();
        let head_of = |s: &Skeleton| s.nodes[4].position.y;
        assert!(head_of(&tall) > head_of(&short) * 1.5);
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = HumanoidParams {
            height: 99.0,
            build: 5.0,
            muscle: -3.0,
            head_size: f32::NAN,
            ..Default::default()
        };
        params.sanitize();
        assert_eq!(params.height, HEIGHT_RANGE.1);
        assert_eq!(params.build, 1.0);
        assert_eq!(params.muscle, 0.0);
        assert_eq!(params.head_size, 0.0);

        let once = params;
        params.sanitize();
        assert_eq!(once, params, "sanitize must reach a fixpoint");
    }

    #[test]
    fn build_thickens_torso_and_limbs_together() {
        let slight = HumanoidParams {
            build: -1.0,
            ..Default::default()
        }
        .skeleton();
        let heavy = HumanoidParams {
            build: 1.0,
            ..Default::default()
        }
        .skeleton();
        // The torso and a limb both answer to the one axis. Found by zone
        // rather than by index, so adding a node does not silently retarget the
        // assertion at some other part of the body.
        let radius_in = |skeleton: &Skeleton, zone: Zone| {
            skeleton
                .nodes
                .iter()
                .find(|node| node.zone == zone)
                .expect("zone exists")
                .radius
        };
        for zone in [Zone::Chest, Zone::UpperLimb(Limb::ForeLeft)] {
            assert!(
                radius_in(&heavy, zone) > radius_in(&slight, zone) * 1.4,
                "{zone:?} should thicken with build"
            );
        }
    }
}
