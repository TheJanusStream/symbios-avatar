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
use crate::cage::limb::HALF_SEGMENT;
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
///
/// **The pelvis was narrowed to 0.55 in #98 and put back in #104, and the round
/// trip is worth keeping.** The narrowing was real at the time: the hip sockets
/// could not come in past the reach of the spine socket's ring, that reach is
/// `x · pelvis_r`, and moving it was what let `hip_x` fall from 1.35 to 0.60.
///
/// Then #104 fattened the hip node from 0.047 to 0.067 of stature, and **that
/// changed which constraint binds.** The failure below 0.64 is now
/// `SocketsOverlap` — the two hip sockets running into each other, which is
/// governed by `hip_x ≥ hip_r / 1.64` and has nothing to do with this constant.
/// Swept to confirm rather than reasoned about: at `hip_x` 0.50 the sweep fails
/// identically at 0.55 and at 1.00, and at the shipped 0.64 it passes at both.
/// The narrowing had stopped paying for itself.
///
/// Putting it back is not merely tidying. A pelvis at 0.55 is *deeper than it
/// is wide*, which dragged the abdomen's width-to-depth ratio to 1.15 and lost
/// `a_trunk_is_not_a_surface_of_revolution` — a test measuring the waist caught
/// a constant describing the pelvis, because the zone spans the bone between
/// them.
///
/// The moral for the rest of this file: **a meshability floor is a property of
/// the whole joint, not of the coefficient that happens to be nearest it.**
/// Change a radius two nodes away and the binding constraint can move without
/// any of the numbers here being touched.
const PELVIS_SECTION: Vec2 = Vec2::new(1.0, 0.80);
/// See [`PELVIS_SECTION`]. The waist is the shallowest part of the trunk.
const WAIST_SECTION: Vec2 = Vec2::new(1.0, 0.76);
/// See [`PELVIS_SECTION`]. A ribcage is about three-quarters as deep as it is
/// wide, which is the strongest single cue in the set.
const CHEST_SECTION: Vec2 = Vec2::new(1.0, 0.74);
/// See [`PELVIS_SECTION`], whose exception this shares and for the same reason.
///
/// The girdle is the shoulders' equivalent of the pelvis: its lateral
/// half-extent is the floor under how close the clavicle sockets may sit, so
/// the width had to leave this node before it could leave `clavicle_x`. The
/// shoulder silhouette is carried by the clavicle and shoulder nodes outboard
/// of it, and the ribcage below by [`CHEST_SECTION`], which is untouched.
const GIRDLE_SECTION: Vec2 = Vec2::new(0.55, 0.80);
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
        // Provenance: **unsourced**, the pelvis and the girdle, from the initial
        // body plan. The two `shoulder_width` gains are unsourced too, and note
        // they are gains on a RADIUS while the shoulder breadth anyone would
        // check is set by `clavicle_x` below — so this axis moves the torso and
        // the girdle by different amounts and no measurement ties them together.
        //
        // **The waist is measured and the chest is not, and the difference
        // between them is a constraint rather than an oversight** (#106). The
        // trunk renders 13–29% narrower than the reference all the way up, and
        // dividing through by the delivered fraction the way #104 did for the
        // limbs asks for 0.097 here and 0.118 on the chest.
        //
        // The waist reached 0.086 of that 0.097 and stopped, because the
        // pelvis's spine socket blends toward this node as it slides: a wider
        // waist inflates the ring the hip sockets have to clear, and above about
        // 0.087 the pelvis stops meshing (`SocketNotOnHull` at joint 0, swept —
        // 0.083 passes, 0.088 does not). That closes a little over half the
        // gap: the bands from 0.54 to 0.66 of stature went from 13–20% narrow
        // to 9–14%.
        //
        // The chest did not move at all. Its own floor is the same rule one
        // joint up — the girdle's chest socket blends toward this radius, so
        // widening the ribcage widens the shoulders in near-lockstep:
        //
        // ```text
        //   chest_r   chest surface   shoulder span   (reference 0.0799 / 0.190)
        //    0.088       0.0598           0.214
        //    0.103       0.0700           0.238
        //    0.118       0.0803           0.272
        // ```
        //
        // There is no good point on that line, and no coefficient escapes it:
        // the socket must clear the chest's *cage* half-width, which is 1.5×
        // what the limit surface renders, so a shoulder ends up pushed half
        // again as far out as the ribcage it clears. Escaping it means changing
        // where the arm attaches, which is a change to the body. Left at 0.088
        // pending that decision.
        let pelvis_r = h * 0.079 * girth;
        let waist_r = h * 0.086 * girth;
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
        // How far the hip sockets sit below the pelvis node, and it is the
        // single coefficient that decides how long a leg is: every millimetre
        // here comes straight out of the thigh.
        //
        // **Down from 1.85, which was never swept.** The comment it replaces
        // said this "trades against `hip_x` for the room the pelvis needs to
        // separate three sockets" and named `hip_x` and `clavicle_x` as the two
        // that were actually measured — so the drop was picked, and then every
        // later argument treated it as a constraint. It was not one. Against
        // the Quaternius reference the leg measured 0.292 of stature where the
        // reference is 0.456, and 1.85 was where most of that went.
        //
        // What the pelvis really demands is written out at `hip_x`: a socket
        // surfaces only when its own plane clears every sibling ring. Worked
        // through for this socket there are two candidate binders and they pull
        // opposite ways. A steep drop makes the two hip sockets point *toward*
        // each other — at 1.85 their axes dot at +0.30, so each has to clear the
        // other and the sibling binds. A shallow one turns them lateral, the
        // sibling term goes negative, and what binds instead is the spine
        // socket's ring, whose corners reach `pelvis_r` sideways. The room
        // available is `max_socket_fraction` of the bone, which shrinks as the
        // drop does. Tabulated at the shipped `hip_x`:
        //
        // ```text
        //   drop   spine reach   room     leg/H
        //   1.85      -0.023    0.260     0.292
        //   0.90      +0.044    0.184     0.369
        //   0.40      +0.094    0.160     0.410
        //   0.20      +0.115    0.155     0.425
        //   0.00      +0.135    0.153     0.433
        // ```
        //
        // Every row has room to spare, and the margin only closes as the drop
        // reaches zero. 0.30 is taken rather than 0.20 because the arithmetic
        // above is for the *default* body: `girth` swells `pelvis_r` by 28% at
        // the heavy end, which grows the spine ring the socket must clear while
        // growing the room by the same factor, and `tests/plan.rs` sweeps 1500
        // bodies across that range.
        //
        // This does **not** narrow the hips, which are the other half of #98 and
        // want the pelvis split in two. `hip_x` cannot come down while one node
        // carries the spine and both legs — see the note there.
        //
        // Provenance: **derived, then bounded by a sweep** (#98) — the plane
        // condition worked through above, confirmed against `tests/plan.rs`.
        const HIP_DROP: f32 = 0.30;
        let hip_drop = pelvis_r * HIP_DROP;

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
        //
        // **The floor BINDS, so this line sets the length of the neck and the
        // 0.072 has nothing to do with it** — the same trap as `pelvis_y` above,
        // one screen apart. Down from 1.32 (#93): at 1.32 the chin sat 103.0 mm
        // above the shoulder line on a 214.6 mm head, a ratio of 0.480, where
        // the eight-head figure puts the shoulder about a third of a head below
        // the chin. 1.15 gives 88.1 mm and 0.411.
        //
        // **Measure that span against the shoulder SURFACE, never against the
        // clavicle joint.** The joint sits about 95 mm lower, under the
        // trapezius, so reading the neck off the rig predicts 190 mm where the
        // surface gives 103 — an 85% error, and the reason a body that had been
        // measured against canon repeatedly still had this in it.
        //
        // **Length was the smaller half of the defect.** At 1.32 the neck was
        // not merely long, it was LUMPY: half-width ran 42.0 under the chin,
        // swelled to 54.0, pinched back to 51.5, and only then flared — a bulge
        // above a waist, the #47 defect in a new place, the head capsule's swell
        // and the neck node's failing to merge over a 143 mm span. At 1.15 the
        // profile is monotone from the narrowest point into the shoulders (41.5,
        // 42.9, 45.0, 47.1, 48.8, 49.6, 50.5, 50.6, 52.0, 57.6, 72.4).
        // Shortening is what fixed the shape.
        //
        // **This drops the whole head, and that is what it costs.** `head_y`
        // follows `neck_y`, so the head sits about 18 mm lower and its lowest
        // band lands where the body's own surface is already rising into the
        // trapezius — which is why `face::skull`'s profile-against-surface
        // contract had to have its throat bound restated to take this. See the
        // note there; it is the one place this change is not free.
        //
        // **Not shortened further, and not from the other end.** Below about
        // 1.16 this floor stops binding at all, so lower values change nothing
        // on a default body while still failing extreme ones — 0.85 loses
        // `tests/plan.rs`. Reaching the canon's 0.33 needs the 0.072 down too,
        // and that steepens the shoulder ramp into a coat-hanger: the same flare
        // over less height. And it must not come from ABOVE — lowering the head
        // by cutting `HEAD_BELOW_JOINT` takes it out of the lower face, which
        // #78 raised to fix a face measuring 39% short, and cranium:face is
        // exactly 1.00 now.
        //
        // Provenance: **derived from a sweep against the canon** (#93), bounded
        // below by socket clearance — 1.32, 1.15 and 1.00 all mesh across
        // `tests/plan.rs`'s 1500 random bodies and its corners; 0.85 does not.
        let neck_y = girdle_y + (h * 0.072 * (1.0 + 0.3 * self.neck_length)).max(girdle_r * 1.15);
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

        // How far out the hip sockets sit, and until #98 the single number this
        // file was most sure it could not move.
        //
        // **Down from 1.35, and the way through was the pelvis rather than this
        // coefficient.** The note that stood here recorded a sweep — 1.60 and
        // 1.45 mesh, 1.30 loses one seed, 1.13 loses five — and concluded that
        // 0.190 of height across the hips was unreachable. It also wrote down
        // the answer, in its last sentence: *reaching canon needs a narrower
        // pelvis or sockets placed differently*. Both were true, and the first
        // one is cheap.
        //
        // The condition is exact and worth stating once, because it governs the
        // shoulders too (see `clavicle_x`). A socket surfaces as a hull facet
        // only when its own plane clears every sibling ring point, and it may
        // slide out at most `max_socket_fraction` of its own bone. So a socket
        // leaving a joint SIDEWAYS has to clear whatever the socket above it
        // reaches sideways — and that reach is the joint's own lateral
        // half-extent, because `Socket::joint_half` is the hub's `half_extents`
        // and the spine socket's ring frame runs `u = ±X` on an upright body.
        // Roughly:
        //
        // ```text
        //   hip_x  >  (spine ring's lateral reach) / 0.82
        //          =  PELVIS_SECTION.x · pelvis_r / 0.82
        // ```
        //
        // At `PELVIS_SECTION.x` of 1.0 that floor is 1.22 pelvis radii, and this
        // coefficient sat at 1.35 — just above it. Every earlier sweep was
        // measuring the floor rather than the hips. Narrowing the section to
        // 0.55 moved the floor to 0.67 and this followed it down, which is how
        // #98 got the hips to the reference.
        //
        // **The section has since gone back to 1.0 and this has stayed down,
        // which looks like a contradiction and is not.** The floor above is not
        // the only one, and it stopped being the binding one when #104 fattened
        // the hip node — see the closed form below. Swept to check rather than
        // argued: at 0.50 the sweep fails identically whether the section is
        // 0.55 or 1.00, and at 0.64 it passes at both.
        //
        // The drop also helps, and it is why this clears rather than merely
        // grazes. The spine socket sits *above* the pelvis centre while a hip
        // socket points down and out, so the two terms subtract: at the shipped
        // `HIP_DROP` the spine ring's projection onto the hip axis is 12.6 mm
        // against 76 mm of travel available.
        //
        // **The gain came down with the base**, 0.35 to 0.10, and had to. It is
        // a multiple of `pelvis_r`, so at `hip_width = −1` the old pair gave
        // 1.00 radii and the new base alone would give 0.25 — a bone too short
        // to carry a socket at all.
        //
        // **The narrow end of the axis is what bounds this pair, not the middle,
        // and there is a closed form for it** (#104). The two hip sockets both
        // sit at `max_socket_fraction` of their own bone, because the distance
        // `socket_distance` asks for is further than the bone is long — so their
        // centres are `2 · 0.82 · bone · (hip_x / bone)` apart, and the bone
        // cancels:
        //
        // ```text
        //   separation  =  1.64 · hip_x        (whatever the drop)
        // ```
        //
        // Which has to clear `socket_clearance · (r + r)`, and by the time a
        // socket has slid 82% of the way to the hip its ring is essentially the
        // hip's own radius. So `hip_x ≥ hip_r / 1.64`, or 0.516 pelvis radii at
        // the shipped ladder — and `girth` cancels there too, since it scales
        // both. 0.64 − 0.10 leaves 4.5% over that floor.
        //
        // This is the constraint that #104's fatter hip ran into: at the old
        // 0.60 − 0.16 the axis reached 0.44 and `tests/plan.rs` found bodies
        // whose two hip sockets interpenetrated, by 0.16% on the first one it
        // hit. The drop is no help — it is not in the formula. **And it is now
        // the binding one**, at 0.516 against the spine socket's 0.67-with-the-
        // drop-subtracted, which is why [`PELVIS_SECTION`] could go back to 1.0.
        //
        // Provenance: **derived, then bounded by a sweep** (#98, #104) — two
        // conditions, both written out above, both confirmed against
        // `tests/plan.rs`. Which of them binds depends on the hip radius, so
        // neither can be dropped from the record.
        let hip_x = pelvis_r * (0.64 + 0.10 * self.hip_width);
        let hip_y = pelvis_y - hip_drop;

        // Where the knee sits between hip and ankle, and so how the leg's length
        // is split between thigh and shank.
        //
        // **Down from 0.60**, which put 40% of the leg above the knee and 60%
        // below — a shank half again as long as its thigh, which is a bird. The
        // reference splits 0.2220 to 0.2339 of stature, a thigh fraction of
        // 0.487, so the knee belongs at `1 − 0.487` of the way up. The female
        // reference gives 0.4993, within a body's own variation of the same
        // number.
        //
        // The old comment blamed this coefficient for the knee measuring 0.227
        // against a canon 0.285 and could not tell whether the fault was here or
        // in the pelvis above. It was the pelvis: at the shipped `HIP_DROP` the
        // knee lands at 0.269 with the split corrected, and the rest of the gap
        // is the hip still sitting low, which is #98's other half.
        //
        // Provenance: **derived** from the reference pair (#99) — a ratio of two
        // measured segment lengths, which is the one form of derivation this
        // file can do without a sweep, because it moves no socket.
        const THIGH_FRACTION: f32 = 0.487;
        let knee_y = ankle_y + (hip_y - ankle_y) * (1.0 - THIGH_FRACTION);
        // How far forward of the hip-to-ankle line the knee stands.
        //
        // A leg built dead straight has no opinion about which way it folds, and
        // [`crate::rig::Rig::bend_pole`] says so in as many words: every limb of
        // this plan measured exactly zero, so the pole came from a hardcoded
        // rule about limb names rather than from the body. That rule happens to
        // be right for a knee and is one fewer thing that has to stay right.
        //
        // It also keeps the solver off the singularity at full extension, which
        // is what [`crate::anim::gait::CROUCH_MARGIN`] exists to paper over from
        // the other end.
        //
        // Measured on the reference: its knee sits 42 mm forward of the line
        // through hip and ankle on a 1.830 m body, a tenth of the thigh. The
        // elbow's equivalent is 6 mm and is **deliberately not copied** — it
        // falls below the half-percent-of-stature floor `bend_pole` treats as
        // arithmetic noise, so it would change nothing, and the fore-limb
        // fallback already folds an arm the right way.
        //
        // Provenance: **looked up** (#99) from the Quaternius male, as a
        // fraction of stature.
        const KNEE_FORWARD: f32 = 0.0222;
        let knee_z = h * KNEE_FORWARD;
        // **The foot's own geometry, all of it measured** (#111). It used to be
        // one node at `h * 0.0257` high and `h * 0.057` forward, tagged unsourced,
        // with a swept slab hung off it. The slab is gone: these place the ball
        // and the toe as real nodes in the leg chain.
        //
        // Provenance for every figure below: **looked up**, from the Quaternius
        // male and female (which share one skeleton), as fractions of stature or
        // of foot length. Foot length is 16.4% of stature on the male and 15.7% on
        // the female; the ball is 53.7% of the foot's length forward of the ankle and
        // the toe tip 82.3%; the ball's own height is 0.83% of stature and the
        // toe's 0.63%.
        const FOOT_LONG: f32 = 0.160;
        const FOOT_BALL_ALONG: f32 = 0.537;
        const FOOT_TOE_ALONG: f32 = 0.823;
        let foot_long = h * FOOT_LONG;
        let ball_z = foot_long * FOOT_BALL_ALONG;
        let toe_z = foot_long * FOOT_TOE_ALONG;
        let ball_y = h * 0.0083;
        let toe_y = h * 0.0063;
        // The radius that makes the foot the right WIDTH, and the width is what a
        // sole is measured by: 37–38% of foot length on both references. A node
        // radius is a request the mesher delivers about 0.64 of at four ring
        // points, and a section rolled half a segment reaches 0.707 of its
        // half-extent along each axis — so the request is the wanted half-width
        // divided by both.
        let foot_r = foot_long * 0.185 / (0.64 * 0.707);
        // How thick the foot is against how wide, from the same outlines: the ball
        // is about 20% of foot length deep against 37% wide.
        const FOOT_FLAT: f32 = 0.55;
        const FOOT_BALL_WIDE: f32 = 1.0;
        // The toe is narrower than the ball — the sole outline runs 0.185 of foot
        // length at the ball and 0.142 at nine tenths along.
        const FOOT_TOE_WIDE: f32 = 0.77;

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
        // **Re-tuned, and against the reference rather than the drawing canon**
        // (#98). The paragraph above asked whether to, and left it as an owner's
        // call; the owner's call was to measure. The canonical 0.245 quoted
        // throughout is *bideltoid breadth* — across the shoulder muscle — and
        // what this coefficient actually sets is where the arm's chain begins,
        // which the reference puts at 0.190 of stature on the male and 0.156 on
        // the female. This now targets their midpoint, 0.173.
        //
        // **The constraint is the hips' constraint with one term more, and
        // getting that term wrong is worth recording.** The first attempt here
        // reasoned exactly as `hip_x` does: a clavicle socket leaves the girdle
        // sideways, so it must clear the neck socket above and the chest socket
        // below, both of which reach `GIRDLE_SECTION.x · girdle_r` laterally,
        // and their vertical offsets project to nothing on a horizontal axis —
        // giving a floor of `1.22 · GIRDLE_SECTION.x · girdle_r`, or 73 mm.
        // `tests/plan.rs` rejected it immediately with `needed: 0.121`, two
        // thirds larger.
        //
        // The missing term is that **a socket ring is not the size of its own
        // joint.** `Socket::set_dist` lerps the ring from `joint_half` toward
        // the *neighbour's* half-extents as it slides out, so the chest socket
        // — sitting 0.9 girdle radii down a bone only 1.3 girdle radii long —
        // arrives 69% of the way to the CHEST's width and reaches 125 mm
        // sideways, not 60. The floor under the shoulders is set by the ribcage,
        // not by the girdle:
        //
        // ```text
        //   clavicle_x  >  (0.31 · GIRDLE_SECTION.x · girdle_r + 0.69 · chest_r) / 0.82
        // ```
        //
        // That second term is fixed anatomy — the chest node measures 0.0912 of
        // stature across against the reference's 0.0911, so it is right and must
        // not be narrowed to buy shoulder width. So the shoulders bottom out at
        // about 0.20 of stature rather than the 0.173 aimed for. Both halves of
        // the fix still earn their place: `GIRDLE_SECTION` at 0.55 is worth
        // 0.021 of stature, and the rest is this coefficient coming off its old
        // 1.85 to sit just above the true floor.
        //
        // **The gain is bounded by the `shoulder_width = −1` corner, not by the
        // neutral body.** `girth` cancels — `clavicle_x` and both floor terms
        // are multiples of it — but `shoulder_width` does not, because it swells
        // the chest by 0.08 and the girdle by only 0.06, so a narrow-shouldered
        // body has proportionally *more* ribcage to clear. At −1 the floor is
        // 1.379 girdle radii; 1.50 − 0.08 leaves 3% over it, and that corner is
        // what the pair below is chosen for rather than the middle.
        //
        // Provenance: **derived, corrected against a failure, then bounded by a
        // sweep** (#98). The first derivation is left above because it is the
        // mistake this socket invites, and because `hip_x` gets away with the
        // same reasoning only by accident — a hip socket's siblings blend toward
        // the *waist*, which is no wider than the pelvis.
        let clavicle_x = girdle_r * (1.50 + 0.08 * self.shoulder_width);
        // Provenance: **unsourced**.
        let clavicle_y = girdle_y + h * 0.004;
        // How far outboard of the clavicle the arm's chain starts.
        //
        // Down from 0.048. This and `clavicle_x` share the shoulder's width
        // between them and neither means anything alone: the reference splits
        // the same span the other way round, with its clavicle joints almost on
        // the midline (0.021 of stature apart, against this plan's 0.096) and a
        // long clavicle bone carrying the rest. What has to match is the sum,
        // and the sum is now 0.214 of stature against the reference's 0.190 and
        // 0.156.
        //
        // Nearly all of that sum is `clavicle_x` now, because `clavicle_x` is
        // pinned just over a floor it cannot cross. This is what is left, and it
        // is kept non-zero only so the two nodes do not coincide and leave the
        // cage a zero-length bone.
        //
        // Provenance: **derived** (#98) — whatever is left of the target span
        // once `clavicle_x` has taken its floor.
        let shoulder_x = clavicle_x + h * 0.010;
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
        //
        // **Both bases raised against the reference pair** (#99). They measured
        // 0.128 and 0.114 of rendered stature where the Quaternius male gives
        // 0.162 and 0.153 and the female 0.129 and 0.155 — so the forearm was
        // short against *both* references by a quarter of its own length, and
        // the upper arm sat on the female figure while the pair straddle it.
        // The bases here are the midpoint of the two, converted out of rendered
        // height into nominal `h` by the 0.965 a built body loses to
        // subdivision.
        //
        // The upper arm is where the two references disagree most in the whole
        // body — 0.162 against 0.129, a fifth — while their forearms agree to
        // 1%. A midpoint is the honest neutral until the frame axis (#100)
        // carries that difference, and it is worth knowing that this one
        // coefficient is where most of that axis's travel will be.
        //
        // The `limb_length` gains are left where they were. They are unsourced
        // either way, and scaling them with the bases would be a second change
        // wearing the first one's evidence.
        let upper_arm = h * (0.1404 + 0.025 * self.limb_length);
        let forearm = h * (0.1484 + 0.025 * self.limb_length);
        // Provenance: **unsourced**, and note it feeds arm span through
        // `hand_at` below — so the extremity axis moves a figure that `#66`
        // tuned, and nothing connects the two.
        let hand_len = h * 0.040 * (1.0 + 0.3 * self.extremity_size);
        let shoulder_at = Vec3::new(shoulder_x, clavicle_y, 0.0);
        let elbow_at = shoulder_at + arm * upper_arm;
        let wrist_at = elbow_at + arm * forearm;
        let hand_at = wrist_at + arm * hand_len;

        let extremity = 1.0 + 0.3 * self.extremity_size;

        // The limb radius ladder, **measured at last** (#104).
        //
        // The note that stood here called these nine figures unsourced, from the
        // initial body plan, and warned they were "the set most likely to look
        // right and be wrong, because a limb tapering monotonically from hip to
        // foot reads as a limb whatever the actual figures are — there is no
        // silhouette cue that a wrong taper violates". That was exactly right,
        // and it undersold the problem: **the thigh was not tapering at all.**
        // Measured, its surface radius read 0.0300, 0.0297, 0.0299, 0.0299 of
        // stature along its length, flat to four decimals, where the reference
        // sheds 36% from hip to knee.
        //
        // **A node radius is a request, not a surface, and that is why nobody
        // could check these.** Subdivision delivers about 0.64 of what is asked
        // for — measured across all four limb bones at 65%, 66%, 61%, 65%, so
        // the factor is a property of the mesher rather than of any one limb.
        // Every figure below is therefore a reference surface radius divided by
        // 0.64, and the division is the reason a decade of eyeballing the ladder
        // against published tables could never have converged: the two columns
        // were in different units. `examples/bodyaudit` prints both, with the
        // conversion in the `kept` column.
        //
        // **The defect was at the proximal ends only**, which is why the whole
        // ladder is not simply scaled up. Ours against the male reference at the
        // far end of each bone: wrist 0.0177 against 0.0181, ankle 0.0210
        // against 0.0183 — already right, the ankle slightly fat. It is the hip
        // and the shoulder that had collapsed toward their neighbours:
        //
        // ```text
        //   shoulder 0.038 -> 0.055    hip   0.047 -> 0.067
        //   elbow    0.032 -> 0.037    knee  0.042 -> 0.037
        //   wrist    0.025 -> 0.025    ankle 0.030 -> 0.025
        // ```
        //
        // **The shoulder is the one figure here that is not its measured value,
        // and it is held down by the wardrobe rather than by the body** (#105).
        // It wants 0.059. Above about 0.057 a `Sleeve::Bare` hem — which cuts
        // exactly through the saddle where the arm meets the torso — closes into
        // a figure-eight instead of a loop and the garment stops being a closed
        // solid. Bisected to this coefficient alone: every other figure in this
        // ladder is innocent, and reverting this one makes the test pass. 0.055
        // is the largest value that cuts, and it leaves the deltoid about 7%
        // thinner than the reference. Raise it when the hem can be cut.
        //
        // **Two passes, because 0.64 is an average and the ends do not share
        // it.** The first pass divided through by it uniformly and left the
        // shoulder 14% thin and the ankle 10% fat; the second corrected each
        // node against its own measured error. The delivered fraction runs 57%
        // at the shoulder to 70% at the ankle — it is smaller where the node is
        // large relative to its bone, which is what a limit surface does. Anyone
        // re-deriving this should expect to go round twice.
        //
        // The hip is the largest move in the file and it does a second job: it
        // is most of the pelvic silhouette, so it is also what puts the hips
        // back near the reference's 0.091 of stature across.
        //
        // **What is left is not a coefficient problem.** The ends now match — the
        // wrist reads 0.0182 against 0.0181, the ankle 0.0184 against 0.0183 —
        // and the MIDDLES are thin: shank 0.0234 against 0.0366, thigh 0.0366
        // against 0.0440. A real limb has a muscle belly and this one has two
        // nodes interpolating linearly between its ends, so a calf or a forearm
        // cannot bulge no matter what these figures say. Fixing it needs a
        // mid-limb node or a radius profile, not a bigger number here.
        //
        // Provenance: **derived, then corrected against a second measurement**
        // (#104) — reference surface radii over the delivered fraction, bounded
        // by `tests/plan.rs`. Not covered: the clavicle at 0.040 and the hand
        // and foot at 0.020 and 0.019, which are stubs inside attached geometry
        // and have no surface of their own to measure.
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
                    h * 0.055 * girth,
                )
                .in_zone(Zone::UpperLimb(fore)),
            );
            let elbow = skeleton.extend_from(
                shoulder,
                Node::new(
                    Vec3::new(side * elbow_at.x, elbow_at.y, elbow_at.z),
                    h * 0.037 * girth,
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

            // The hip carries the pelvic silhouette now, which is new: with
            // [`PELVIS_SECTION`] narrowed, the width at hip level is these two
            // nodes and not the pelvis between them. Measured on the reference,
            // the thigh's mean radius at the hip is 0.0455 of stature, which is
            // 0.0439 of nominal `h` once the 0.965 a built body loses to
            // subdivision is taken out; 0.047 is that with a little back,
            // because these are the radii *before* subdivision shrinks them.
            //
            // Down from 0.052, and it had to come down or the body would have
            // *gained* width from #98: the hips moved in but the blobs on them
            // did not, and the silhouette is the sum. 0.047 puts the outer edge
            // at 0.0943 of stature against the reference's 0.0910.
            let hip = skeleton.extend_from(
                pelvis,
                Node::new(Vec3::new(side * hip_x, hip_y, 0.0), h * 0.067 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let knee = skeleton.extend_from(
                hip,
                Node::new(Vec3::new(side * hip_x, knee_y, knee_z), h * 0.037 * girth)
                    .in_zone(Zone::UpperLimb(hind)),
            );
            let ankle = skeleton.extend_from(
                knee,
                Node::new(Vec3::new(side * hip_x, ankle_y, 0.0), h * 0.025 * girth)
                    .in_zone(Zone::LowerLimb(hind)),
            );
            // **The foot is part of the leg, not a slab hung off its end** (#111).
            // Two more nodes carry it — the ball of the foot and the toe — so the
            // cage rings run through it and the ankle is continuous surface. Both
            // reference bodies are built that way: the male's foot is inside the
            // leg shell and the female's inside one shell running crown to sole,
            // which is why neither has a seam where ours had one.
            //
            // Rolled by half a ring segment, which is what stands the section on
            // a flat edge instead of on a vertex. Without it a foot meshed from
            // the graph rests on a keel for exactly the reason the swept foot did.
            let ball = skeleton.extend_from(
                ankle,
                Node::new(
                    Vec3::new(side * hip_x, ball_y, ball_z),
                    foot_r * FOOT_BALL_WIDE * extremity,
                )
                .with_scale(Vec2::new(1.0, FOOT_FLAT))
                .with_roll(HALF_SEGMENT)
                .in_zone(Zone::Extremity(hind)),
            );
            skeleton.extend_from(
                ball,
                Node::new(
                    Vec3::new(side * hip_x, toe_y, toe_z),
                    foot_r * FOOT_TOE_WIDE * extremity,
                )
                .with_scale(Vec2::new(1.0, FOOT_FLAT))
                .with_roll(HALF_SEGMENT)
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
