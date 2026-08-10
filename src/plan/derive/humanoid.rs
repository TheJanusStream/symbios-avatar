//! Every dimension the biped cage is built from.
//!
//! Eleven semantic axes in, one flat [`Dimensions`] out, and
//! [`crate::plan::humanoid`] does nothing afterwards but hang nodes on it. See
//! [`super`] for why this layer exists at all, what its clamps mean for an axis
//! added later, and what a refactor of it has to prove.
//!
//! **Four of the walls in this crate are here**: `girth`'s two ends, `hip_x`'s
//! floor and `clavicle_x`'s, plus `extremity`'s pair. They are listed together
//! on [`Dimensions`] and derived individually beside the expressions that meet
//! them — and two of those six ends turned out under audit not to be walls at
//! all, which the table on [`Dimensions`] now says (#163).

use glam::{Vec2, Vec3};

use crate::plan::{Composites, DEFAULT_BODY_FAT, HumanoidParams};

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
///
/// **0.55 → 0.75 and 0.80 → 1.10, on the owner's decision to accept wider
/// shoulders** (#106, option d). The narrowing to 0.55 was #98 buying shoulder
/// width by starving the trunk above the ribcage, and what it cost was visible:
/// the top band of the silhouette table pinched where the reference carries its
/// trapezius, because the taper out of the chest is aimed at this node. Putting
/// the width back re-widens the shoulders through `clavicle_x`'s floor, which
/// is what the decision agrees to stop fighting.
///
/// **The two axes are not the same trade and it is worth keeping them apart.**
///
/// ```text
///   .y  0.80 → 1.10   free: same clavicle floor, same saturation, same span,
///                     and it pays for most of the depth shortfall
///   .x  0.55 → 0.75   costs shoulder span at about 3.5 mm of span per mm of
///                     trunk half-width, because the floor under `clavicle_x`
///                     carries 0.31 of this constant
/// ```
///
/// Depth was the cheaper half and nobody had tried it: `.y` does not appear in
/// the clavicle floor at all, so it buys upper-trunk depth outright. Its own
/// wall is between 1.10 and 1.15, where the 400-roll envelope stops meshing.
///
/// **What the widening bought, mean over the classic-filtered roll** (2000
/// seeds, 114 bodies inside ±1 — `bodyaudit --classic`):
///
/// ```text
///   band 0.69–0.72   width  −10.6% → −6.6%   depth  −11.3% → −5.9%
///   coat hanger at the girdle   2.395 → 2.347   (reference 2.085)
///   shoulder joints             +9.3% → +15.6% over the male reference
/// ```
///
/// **The coat-hanger ratio answers differently depending on where it is
/// anchored, and #106 quotes both without saying so.** Taken at the girdle —
/// the trunk's own half-width where the shoulders actually are — it falls,
/// 2.468 to 2.250 on the default body, which is the trade working. Taken at a
/// fixed band down on the ribcage it rises, 2.564 to 2.695, and no setting of
/// this constant brings it down: the span outruns the ribcage at every width,
/// because the floor under the shoulders is `0.69 · chest_r` and the girdle is
/// only 0.31 of it. Anyone re-opening this should read that as "the ribcage
/// ratio is not reachable from here" rather than as a failed widening.
///
/// Provenance: **swept, then judged by render** (#106). The pair above was
/// chosen off a sweep of `.x` at 0.55, 0.65, 0.75, 0.85, 0.95 and 1.05, each
/// with `clavicle_x` re-floored against it; the ratio at the girdle is flat
/// from 0.65 up, so past 0.75 the extra span buys almost nothing the eye can
/// see. Renders of the default body and seeds 3 and 21 carried the choice.
const GIRDLE_SECTION: Vec2 = Vec2::new(0.75, 1.10);
/// How far the girdle's clavicle socket has slid toward the clavicle node by
/// the time its ring clears the joint, as a fraction of that bone.
///
/// A socket ring is not the size of its own joint: [`Socket::set_dist`] lerps it
/// from the joint's half-extents toward the *neighbour's* as it slides out, and
/// where it stops is a fixed point over all four of the girdle's sockets. This
/// is that fixed point, measured rather than solved — see the neck floor in
/// [`Dimensions::of`], which is the only thing that reads it.
///
/// Provenance: **measured against the mesher** (#174). The neck bone was
/// bisected on each of `tests/plan.rs`'s 400 rolled records to find the shortest
/// one that still meshes, and this blend is what makes
/// `(1 − b) · GIRDLE_SECTION.y · girdle_r + b · clavicle_r` an upper bound on 397
/// of those 400 demands. At 0.62 the tightest body has 5.3% of margin and the
/// median body 9.7%; at 0.70 the margin is 0.6% and the formula is no longer a
/// bound anyone should trust. Bodies whose neck floor lands on a *different*
/// socket are excluded and named at the site.
///
/// [`Socket::set_dist`]: crate::cage::Socket::set_dist
const CLAVICLE_RING_BLEND: f32 = 0.62;
/// [`CageConfig::max_socket_fraction`](crate::CageConfig::max_socket_fraction),
/// mirrored so the plan can floor a bone against what the mesher will let a
/// socket spend of it.
///
/// A socket may travel this fraction of its own bone and no further, so a
/// clearance of `d` needs `d / MAX_SOCKET_FRACTION` of bone to be reachable.
/// Mirroring a config default into the plan is the same bargain
/// [`CLAVICLE_RING_BLEND`] takes and carries the same risk: nothing checks the
/// two agree. The plan derives no cage config and is handed none, so the
/// alternative is a magic 0.82 with the dependency unrecorded, which is exactly
/// the defect #174 was raised for.
const MAX_SOCKET_FRACTION: f32 = 0.82;
/// See [`PELVIS_SECTION`]. A neck is very nearly round, but not quite — and it
/// is not centred on its own joint either, which is what the depth here is for.
///
/// **0.94 → 1.56, and it means nothing without [`NECK_LOBE`]** (#125). The pair
/// is one shape: the section is swept `NECK_LOBE` radii astern of the joint, so
/// it reaches `depth + lobe` behind and `depth − lobe` in front. That front
/// figure is 0.94 — exactly what stood here before #125 — and **every change to
/// this pair since has held it**: the throat stands where it always did and
/// only the rear reach moves.
///
/// **1.56 → 1.28, with [`NECK_LOBE`] 0.62 → 0.34, on the owner's side-view
/// verdict** (#148). The nape stood proud of the head's own rear line — the
/// neck, the back below it and the blend between them all bulging astern of
/// the occiput — so the rear reach came in by `0.56` neck radii while the
/// front held at 0.94. Judged on `--head --bare` renders across seeds 0, 3, 7
/// and 21: the rear contour now runs occiput to collar without the hump, and
/// the silhouette ruler in `the_neck_is_the_length_of_a_neck` reads the tuck
/// at four-decimal nothing on three of five seeds.
///
/// **1.28 → 0.94, with [`NECK_LOBE`] 0.34 → 0.00, and this is the FLOOR of
/// the mechanism** (#149). The owner judged #148's tuck an improvement and
/// asked for a lot more; at a lobe of zero the section is symmetric about its
/// own node and `depth` cannot go lower without pulling the throat back —
/// which is exactly what the reverted 1.32 attempt did and what
/// `the_chin_follows_the_jaw_and_not_the_skull` guards. The remaining astern
/// terms are [`NECK_BACK`]'s 0.35 (the cervical lean, which moves the throat
/// with it) and the girdle below. Judged across the same four seeds: the back
/// of the neck now sits inside the occiput's line with a true nape hollow.
/// Any further tuck is a different mechanism, not a smaller number here.
///
/// The paragraph below said this constant was SETTLED against the reference's
/// own column, and the measurement it stands on is still true — but it was
/// answering a different question. The reference carries its rear depth fused
/// into a trapezius it actually has; ours floats the same depth on a body with
/// no shoulder mass under it (#131, open), and there it reads as a dowager
/// hump rather than a back. Matching the reference's depth column is the right
/// TARGET once #131 lands the mass below it; until then the depth defers to
/// the render.
///
/// The two columns share exactly one landmark — each has a single narrowest
/// point — and the ratio there is stature-free and anchor-free. Depth over width
/// at each body's own waist:
///
/// ```text
///   seed        0      3      7     13     21    reference
///   D:W     1.648  1.645  1.733  1.712  1.707        1.663
/// ```
///
/// Within 4% across the space and within 1% on the two seeds nearest neutral. So
/// the neck's depth for its width is right and 1.32 would have made it wrong.
/// The waist also now sits +14 to +21 above the chin against the reference's
/// +10, where before #131 it was at −20 — eighty millimetres of anatomy out.
/// That was #143's own unmeasured candidate and the widening closed it without
/// being aimed at it.
///
/// **What the stale reading was.** `examples/neckaudit` cuts at heights below the
/// HEAD JOINT and its 167 mm reference depth was taken at a "mid-neck" that no
/// longer means the same anatomy, so it reported 195 against 167 and looked like
/// a 17% overshoot. This is #144's failure exactly — a comparison anchored on one
/// landmark is only valid where the two bodies share the anatomy between it and
/// the reading — arriving in a second place within one session.
///
/// The rest of this note is kept because the arithmetic in it is still correct
/// and it records what was built and thrown away.
///
/// **#131 widened `neck_r` by a third and this constant is expressed in neck
/// radii.** The
/// axis-free depth at mid-neck went 152 mm to 195 against the reference's 167,
/// so a figure that used to sit 9% under it now sits 17% over. Bringing this to
/// 1.32 lands it on 167.4 exactly and restores the old forward reach to within a
/// millimetre — 48.1 mm against 48.9 — which would also give back the millimetre
/// `tests/parts::the_underside_of_the_jaw_does_not_bulge` had to widen its bound
/// for. It was built, measured and REVERTED, for two reasons.
///
/// `rig::skin`'s `the_chin_follows_the_jaw_and_not_the_skull` fails at 1.32: the
/// throat comes back toward the neck bone, the neck wins weight under the chin,
/// and the jaw's hold on it goes 0.392 to 0.300 against a 0.30 bound with the
/// travel share at 33.5% against 35%.
///
/// And the two instruments disagree about which way this should go at all. The
/// axis-free depth says we are 17% too deep; `examples/column`, cutting planes
/// anchored on the chin, says the opposite — our back reached only −120.8 mm at
/// 60 mm under the chin against the reference's −146.7 before #131 and lands on
/// −149.6 after. Same body, same reference, opposite verdicts, because they read
/// at different heights off different anchors. Reconciling them is a pass of its
/// own, and picking a value that satisfies the skinning test instead would be
/// choosing a number to fit a bound.
///
/// The SURFACE follows that to within a couple of millimetres rather than
/// exactly, and the residual is not slack in the arithmetic: the limit surface
/// at the front of the ring is an average over neighbours which all moved back,
/// even though the front vertex did not. Bisected on the built body, the throat
/// comes back 2.1 mm at mid-neck and 5.3 mm at its worst, in the throat band on
/// the widest-span seed. It never comes forward, which is the direction that
/// would matter.
const NECK_SECTION: Vec2 = Vec2::new(1.0, 0.94);

/// How far astern of its own joint the neck's section is swept, in neck radii.
///
/// **A neck is the front of a column, and ours was a pole between two balls**
/// (#125). Measured on the Quaternius reference, the surface reaches 2.0 to 2.4
/// times as far behind the neck's axis as the throat reaches in front, at every
/// height from the chin to the shoulders; ours read 1.00, exactly, at every
/// height, because a centred ellipse is symmetric fore and aft by construction.
/// Seen from above, ours was a closed oval floating in clear air where the
/// reference is a forward lobe fused into a wide back mass — the trapezius and
/// the upper back come right up behind the skull and there is no back of the
/// neck as a separate surface at all.
///
/// **A node cannot say that and this is not a node.** Three constructions were
/// tried and reverted before this one: a trapezius node off the girdle, which
/// wants a fifth socket on a node that has four and cannot separate them; the
/// same node off the neck, whose ring is too small to carry two sockets and
/// misses by two tenths of a millimetre; and the girdle deepened backward,
/// which meshes, half works, and stretches the whole lower face because the
/// head's floor moves with it. What lands instead is [`crate::skeleton::Node::offset`], which
/// moves the swept SECTION and leaves the joint — so nothing that measures the
/// head in radii about that joint sees this at all.
///
/// What it buys is the figure that needs no axis, because the axis is what an
/// instrument got wrong here twice: the section's DEPTH at mid-neck goes
/// 104.5 mm → 155.5 against the reference's 167, and the column carries about
/// fifty millimetres more behind it at every height from the head's floor to
/// the shoulders. `examples/neckaudit` prints the table.
///
/// **Two figures recorded on #125 before this landed do not survive their own
/// instrument, and the corrected ones are above.** The depth was reported as
/// 167.5 — measured with the joint hull still handing the neck's extra depth to
/// the girdle symmetrically, so a fifth of that number was the CHEST. And the
/// column was reported monotone for the first time; asked with a width probe
/// that takes the widest point of each slice instead of firing sideways from
/// the joint — a ray from the axis crosses an off-centre ellipse on a chord —
/// the column turns exactly once, at the neck's own waist, both before this
/// change and after. The lateral profile barely moves for this at all, which is
/// what a displacement in `z` should do.
///
/// Provenance: **derived from the reference** (#125), and paired with
/// [`NECK_SECTION`] rather than chosen against it. The mid-neck depth figures
/// above were measured at 0.62; **0.62 → 0.34 in #148, → 0.00 in #149**, the
/// two moving as one shape so the throat held at `depth − lobe = 0.94` while
/// the rear came in. Zero is this constant's floor, not its retirement: the
/// sweep-astern mechanism stays for the day #131's shoulder mass gives the
/// rear reach something to stand on — the full record, and why the reference's
/// own rear reach is deferred rather than matched, is on [`NECK_SECTION`].
const NECK_LOBE: f32 = 0.00;

/// How far behind the midline the neck node sits, in neck radii.
///
/// **A neck leans forward, and ours was a vertical pole** (#125). Measured on
/// the Quaternius reference, the neck node sits BEHIND BOTH ITS PARENT AND ITS
/// CHILD: `spine_03` at z −13.4 mm, `neck_01` at −51.4, `head` at +11.4. So the
/// column kinks backward at the neck and comes forward again into the skull,
/// which is the cervical curve. Every node in this file sat on `z = 0`, so ours
/// went straight up.
///
/// What it buys is the length complaint rather than the shape one. Moving the
/// neck back seats it further inside the shoulder mass, so the flare reaches
/// higher and the VISIBLE neck shortens: `the_neck_is_the_length_of_a_neck`
/// reads 0.327–0.428 across its five seeds against 0.423–0.472 before, and the
/// best of them is on the eight-head canon's 0.33.
///
/// **What it does NOT buy is the mass behind the neck, and a measurement said
/// otherwise before the instrument was corrected.** The reference's surface
/// reaches 2.0 to 2.4 times as far behind its neck axis as its throat reaches
/// in front. Ours read 1.00 before this and 2.50 after, which looked like the
/// whole defect fixed by one constant — and was the probe reading the OFFSET,
/// because it measured from `z = 0` while the axis had moved. Asked from the
/// column's own axis the answer is 0.91 against 0.95: the surface did not move.
/// Axis-free, the section at mid-neck was 103 mm deep against the reference's
/// 167. That is what a node's POSITION cannot say and its section can: see
/// [`NECK_LOBE`], which closed it without touching this.
///
/// 0.35 rather than the reference's own 0.73 of a neck radius, because ours is
/// bounded by what it does to the FLOOR rather than by the reference: see
/// `HEAD_BELOW_JOINT`, which had to be re-derived for it.
/// Provenance: **derived from the reference** (#125), then bounded by the
/// coupling to the head. `neck_01` sits 38.0 mm behind `spine_03` on a
/// 1.829 m body and its neck measures about 52 mm across, which is 0.73 of a
/// radius; ours is 0.35 because past that the lower face stretches faster
/// than `HEAD_BELOW_JOINT` can be brought back — three sentences of this
/// docstring said 0.50, which is a value that was measured and not kept.
const NECK_BACK: f32 = 0.35;

/// Where the crown node sits above the head joint, in head radii.
///
/// See [`CROWN_WIDE`]: the two are chosen together and neither number means
/// anything alone. It came down 0.72 → 0.68 when that went up, to hold the built
/// crown still while the vault was widened. It goes 0.68 → 0.86 here, to move
/// the built crown on purpose.
///
/// **Up from 0.68, and this is half of the head's missing height** (#79). The
/// built head measured 161 mm crown to chin on a breadth of 160, where the
/// eight-head canon on this body's own rendered stature asks 206 — a quarter
/// short. The other half is `HEAD_BELOW_JOINT`, and splitting it between the two
/// is what holds cranium:face at 1.00. Taking it all from either end buys the
/// height by spoiling the proportion #78 derived and #61 measured.
///
/// **Delivery is very nearly one for one, which the head overhaul did not think
/// it was.** #79 was raised recording the cap collapsing 69 mm under
/// subdivision, and concluded the crown node was a poor lever and the vault
/// needed shape rather than height. Swept on the eight-point cage that is no
/// longer true: 10.0 mm of built crown per 0.10 of this, against a 10.32 mm
/// nominal. The collapse belonged to the four-point cage.
///
/// At 0.86 the built crown sits at +106.0 mm on the default body — 1.03 head
/// radii, up from 0.85 — which is the figure three profile tables in
/// [`crate::face::skull`] had to be re-based onto, because their heights above
/// the joint are raw radii and do not follow a crown that moves.
/// Provenance: **derived** (#79), from cranium:face. `HEAD_BELOW_JOINT` sets
/// the chin and `Canon::EYE_LINE` the eye line; this is the value that puts
/// the crown as far above that line as the chin is below it. Measured 0.999
/// on the default body and 0.95 to 1.01 across eight seeds at neutral face
/// length.
const CROWN_HIGH: f32 = 0.917;

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
/// at the same time so the built crown did not move.
///
/// **Down to 0.825 when [`CROWN_HIGH`] went to 0.86, and this is the trap in
/// that pair** (#79). Raising the crown node WIDENS the head without touching a
/// breadth term anywhere: the node is most of a head radius across, so lifting
/// it makes the blend below it bulge. Measured, `CROWN_HIGH` 0.90 on its own
/// took maximum breadth from 159.7 mm to 165.7 — spending, silently, the one
/// absolute width on this head that was RIGHT, and the one #61 chose the breadth
/// axis's default against. At 0.825 the built breadth is 160.9 mm against a life
/// eu-eu of 156, which is where it was.
/// Provenance: **looked up, then tuned by render** (#79) — the only coefficient
/// in this file with a stated anthropometric premise underneath it. The premise
/// is that maximum head breadth is at eurion, high on the parietal roughly 25
/// to 45 mm above the pupil line, and exceeds bizygomatic breadth (156 mm
/// against 137). The value itself was swept against the built half-widths,
/// which is what the figures above are, and re-swept against the built breadth
/// when [`CROWN_HIGH`] moved.
const CROWN_WIDE: f32 = 0.825;

/// How broad the skull is for its own height, as a scale on both cross-section
/// axes of the head's nodes.
///
/// **The head was a fifth too wide for its height, and it is the last of the
/// three axes this issue names** (#79). Measured on the default body against a
/// life head scaled to our own crown-to-chin: the breadth read 160.6 mm where
/// life asks 135.1, while the FACE across the cheekbones read 114.2 against a
/// life 118.7 — so the face was right and the skull around it was not. A
/// correctly proportioned face inset in a vault a fifth too broad is what reads
/// as a pinched jaw, which is what the owner reported and what three width
/// measurements disagreed about before one of them was asked the right way.
///
/// **Both axes, not just the lateral one.** Narrowing the breadth alone would
/// take vault depth-to-width from 1.31 to 1.55 against a life 1.28 — that ratio
/// is one of the few on this head that is already right, and it is right because
/// the section is round. So this scales the section rather than squashing it.
///
/// It is a scale on the CAGE and costs no triangles, which matters: the dearest
/// body in the parameter space is close enough to the ceiling that a taller
/// crown has to be argued for and a narrower skull does not.
///
/// Provenance: **derived** (#79), from H:W. At 0.897 the breadth comes to about
/// 144 mm, which keeps the default inside the life population of roughly 140 to
/// 165 that [`HEAD_BREADTH_SPAN`] is quoted against — the reason it is not the
/// 0.845 that H:W 1.48 asks for at the height the head had before this pass.
const SKULL_SLENDER: f32 = 0.897;

/// Where the jaw's hinge sits, in head radii about the head joint: `(down,
/// forward)`.
///
/// **A MARKER, not meshed geometry — and that is a measured necessity, not a
/// shortcut** (#134). A mandible node as a third socket on the head was swept
/// against the cage and can never mesh: the plane rule needs ~0.12 m beside
/// rings as large as the head's, and 0.82 of the bone only reaches that once
/// the node hangs outside the head's own surface. The same wall #125 measured
/// at the girdle and the neck for a trapezius, now measured at the head.
///
/// So the jaw enters the RIG and not the cage: this pivot and [`JAW_TIP`] give
/// `anim` the condyle-to-chin bone a bone-driven jaw needs (#118), and the
/// skin binds to it by the ordinary falloff. The mandible's MASS stays the
/// skull stage's business — see `face::skull`'s submental construction, which
/// is the other half of #134.
///
/// **The hinge sits well BELOW the ear, and the anatomical TMJ was measured
/// and rejected for it** (#159). The temporomandibular joint is at the ear
/// canal — the first cut put the pivot there (down 0.06, #134, looked up) —
/// but a real jaw does not rotate about its condyle alone: the condyle
/// translates forward along the articular eminence as the mouth opens, and
/// the instantaneous centre of a wide opening migrates down the ramus toward
/// the mandibular foramen. A static pivot has to stand for that whole path.
/// At the ear it cannot: the chin sits 0.91 head radii below and 0.82 forward
/// of it, and rotation moves the chin backward by its depth below the pivot
/// per radian against downward by its forward offset — a back-to-down ratio
/// of 1.11, so a wide-open chin retreated faster than it fell and the owner
/// read it as the chin being pulled into the throat.
///
/// Lowering the pivot spends depth-below-the-pivot directly: swept at down
/// 0.06 / 0.25 / 0.45 / 0.60 (ratios 1.11 / 0.87 / 0.61 / 0.42) and judged on
/// the wide-open profile, 0.45 is where the chin reads as dropping rather
/// than folding while the two costs of going lower stay small: skin behind a
/// lowered pivot counter-rotates (the throat lifts on open, first visible at
/// 0.60), and the mouth-corner shear band grows with the arc. The rest pose
/// is untouched by construction — the pivot only matters posed.
/// Provenance: **tuned by render** (#159), against that four-point sweep;
/// the down component is a stand-in for condylar translation, not the TMJ.
const JAW_PIVOT: Vec2 = Vec2::new(0.45, 0.10);

/// Where the jaw bone ends: `(down as a share of the head's reach BELOW ITS
/// JOINT, forward in head radii)`.
///
/// The chin. The tip sits on that landmark so the bone `pivot → tip` IS the
/// mandible — rotating the pivot swings the chin through the arc a jaw actually
/// opens along.
///
/// **The two components are counted in two different rulers, and that is the
/// correction #135 measured** (#134 shipped both in head radii, as 0.97 and
/// 0.92). A head's reach below its own joint is what `face_length` stretches —
/// it is the `HEAD_BELOW_JOINT` span at the head's placement above, scaled by
/// [`FACE_LENGTH_SPAN`] — while the head's RADIUS does not move with it at all.
/// So a tip pinned to the radius stayed put while the chin walked away from it:
/// measured on the built body, the tip sat +16.5 mm BELOW the chin at
/// `face_length` −1, +0.7 mm at 0, and 15.2 mm ABOVE it at +1, and the axis
/// rolls over ±0.7. At +1 that put the bone's end up by the lower lip, and the
/// binding followed it there — the upper lip came out held 0.79 by the jaw at
/// every reach in a sweep, so the lips could not part.
///
/// In the span's own ruler the landmark is stationary: 0.603 of it at
/// `face_length` −1, 0 and +1, which lands the tip within 0.1 mm of the
/// measured chin at all three. `rig::skin::owner_of` independently measured the
/// same landmark at 0.599 of the same span, at two very different values of
/// `HEAD_BELOW_JOINT`, which is the second measurement this rests on.
///
/// The forward component has NO such ruler and stays in head radii, which is a
/// known residual rather than a solved problem: the chin's own projection
/// measures 0.852, 0.910 and 1.004 radii at `face_length` −1, 0 and +1 — it is
/// constant in neither ruler — so the tip sits 7.1 mm proud of a short face's
/// chin and 8.6 mm behind a long one's. Small against the ~28 mm falloff, and
/// the fix is to hang the marker off the MEASURED skull rather than off the
/// plan, which is a different issue's work.
/// Provenance: **measured** (#135) — the chin's height on built bodies at three
/// face lengths, agreeing with `owner_of`'s independent 0.599.
const JAW_TIP: Vec2 = Vec2::new(0.603, 0.92);

/// The marker radii of the jaw's two nodes, in head radii: `(pivot, tip)`.
///
/// **Retired as a binding mechanism by #152 and kept as the record of why.**
/// These were BINDING reaches — the falloff that decided which skin follows
/// the mandible scaled with them — and the sweep below is the measurement that
/// eventually killed that mechanism: its own conclusion is that no reach can
/// hold the chin's flank while releasing the upper lip. The mandible's skin is
/// now the REGION `face::skull::mandible_hold` describes (the owner's
/// lip-to-larynx contract), `rig::skin::bind` skips marker bones in its
/// falloff entirely, and these radii mesh nothing and bind nothing.
/// **Sourced by sweep against a posed jaw** (#135), which is the only thing
/// that can source it: dual quaternion blending deforms a bad reach and a good
/// one identically at rest AND under a head turn, so every suite in the crate
/// stayed green over the unsourced first cut of `(0.24, 0.30)`. Rotating the
/// pivot is what tells them apart, and `render --jaw 20 --jawsweep` is what
/// rotates it.
///
/// The two opposed requirements: **the chin must follow the jaw and the upper
/// lip must stay with the head.** At the first cut the upper lip was held 0.652
/// by the mandible and travelled 19.2 mm of the lower lip's 28.2 at a 20-degree
/// open — the lips could not part, which is the one thing a jaw is for. Swept
/// at three face lengths, in the upper lip's travel and its worst single
/// vertex's hold, against the separation the lips actually open by:
///
/// ```text
///   pivot  tip    upper lip travel, worst hold      lips part by
///                 short face   default   long face
///   0.06  0.20     0.2 mm .12  0.0 .00   0.0 .00    9.9 to 13.4 mm
///   0.10  0.20     0.9 mm .33  0.0 .01   0.0 .00   13.5 to 14.9 mm
///   0.14  0.20     2.2 mm .51  0.2 .19   0.0 .01   15.6 to 18.5 mm
///   0.10  0.23     3.2 mm .60  0.7 .32   0.0 .00   15.9 to 20.7 mm
///   0.10  0.26     6.6 mm .73  3.2 .62   0.3 .12   14.3 to 26.1 mm
///   0.24  0.30    (the first cut) 19.2 mm .90 on the default body
/// ```
///
/// A SHORT face is what binds, and it is not obvious: it packs the mouth line
/// closer to the chin, so the same reach that clears the upper lip on a long
/// face swallows it on a short one. 0.20 at the tip is the last row where the
/// worst upper-lip vertex stays under a third at every face length, and the
/// pivot buys separation up to about 0.10 before it starts costing the upper
/// lip more than the lower one gains.
///
/// What it does NOT buy, measured and left standing: the chin follows at 37 to
/// 44% of a rigid mandible's arc rather than at 100, and the skin under the jaw
/// at 8 to 20%. Both are the same limit and no value of this constant moves
/// them — the bone is a single MIDLINE segment with a spherical falloff, so the
/// chin's flanks sit 27.4 mm from it while the upper lip sits 28.5, and nothing
/// keyed to distance can hold one and release the other. A jaw that carries its
/// own corners wants a pair of rami, which markers can afford (they mesh
/// nothing) and which is #118's next slice, not this constant's business.
/// Provenance: **swept** (#135) against a 20-degree open at `face_length` −1, 0
/// and +1; the table above is the sweep.
const JAW_REACH: Vec2 = Vec2::new(0.10, 0.20);

/// How far the `head_breadth` axis narrows or broadens the skull, as a share of
/// its own lateral half-extent at each end.
///
/// **The head is the only major node in this file with no section, and giving it
/// one is what makes a broad skull and a narrow one two different people rather
/// than two different sizes** (#61). Every other trunk node carries a
/// [`PELVIS_SECTION`]-style pair because a body part is not a surface of
/// revolution; the head carried none, so the record could say how BIG a head was
/// and never how it was shaped.
///
/// Done at the cage rather than in [`crate::face::skull`]'s `BREADTH` profile,
/// and the reason is the seam that file spends four docstrings on. A profile
/// there moves head-owned vertices and leaves neck-owned ones, so anything it
/// says at the bottom of the head is a step in the silhouette and every profile
/// has to be authored back to identity at the junction — which means the jaw,
/// the part of a face a breadth axis most needs to move, gets the least of it.
/// The cage has no such boundary: the blend from the head node to the neck node
/// interpolates the section along with everything else, so a narrowed skull
/// meets its throat by construction.
///
/// **A fifth either way**, which on the default body is a maximum breadth of
/// 128 to 192 mm against a life population of roughly 140 to 165. Deliberately
/// past life at both ends: a record axis that only reaches the population's own
/// bounds cannot express anyone at them, since a face is the sum of several
/// axes and each one has to have somewhere to go.
///
/// Provenance: **derived from a sweep, then bounded by meshability** (#61).
/// The span is what `tests/plan.rs`'s 1500-body sweep carries — see
/// `head_breadth` in [`HumanoidParams`] for why the broad end is the one that
/// binds — and the life figures above are what the default is chosen against,
/// not what the span is.
const HEAD_BREADTH_SPAN: f32 = 0.20;

/// How far the `face_length` axis stretches the head below its own joint, as a
/// share of [`Dimensions::of`]'s `HEAD_BELOW_JOINT`.
///
/// **The other half of #61's owner call, and the constant it names.** Face
/// length was a derived quantity: `head_size` moved the crown and the chin
/// together, so a record could ask for a bigger head and never for a longer
/// face. This is the one coefficient that separates them, and the whole of its
/// derivation is written beside it at its use site.
///
/// **A sixth either way, and what bounds it is the triangle budget rather than
/// anything about a face.** Measured end to end on eight seeds: the eye-to-chin
/// frame runs 51.5 to 70.9 mm on seed 7 and 78.5 to 107.6 on seed 23 — a factor
/// of 1.38 either — and cranium:face runs 1.24–1.30 at the short end through
/// 0.90–0.93 at the long one, so life's 1.00 sits inside the range at about
/// `+0.45`. Nothing else moves with it: the bizygomatic holds to 0.5 mm and the
/// eye's aperture to one degree across the whole sweep.
///
/// The refinement bands were the bound when this axis was designed and are not
/// any more — they are fractions of each head's own lower face now, so the
/// feature stack cannot walk out of them at either end. See `FACE_PASSES`. What
/// binds instead is that a longer face is more surface to refine: the dearest
/// body anywhere in the space is 29,886 triangles against a 30,000 target, and
/// 826 of the distance to it is this axis's top end.
///
/// **Which is also why the neutral did not move**, though the measurement says
/// it should: cranium:face is 1.05–1.08 today against the 1.00 that #78 derived
/// `HEAD_BELOW_JOINT` to give, and restoring it costs 534 triangles at that same
/// dearest corner and puts it over the target. The head is 32% short of life
/// overall (#79) and 6% of that is this coefficient's; spending the budget on
/// the small half while the large half is open is the wrong order.
///
/// **That deferral is discharged and the fear behind it was wrong** (#79). Both
/// halves were spent together: `HEAD_BELOW_JOINT` 1.19 → 1.55 with
/// [`CROWN_HIGH`] 0.68 → 0.86, cranium:face 0.999 on the default body. And the
/// budget went the other way — a taller vault sits ABOVE `FACE_PASSES`'s
/// above-joint ceilings, so the dearest body anywhere in the space fell from
/// 29,886 to 29,092. The 534-triangle figure quoted above is real and it is also
/// not monotone in the stretch: 1.19 → 1.55 costs 188 at that corner where
/// 1.19 → 1.27 costs 534, because a band edge lands on a ring of faces rather
/// than between them. Cost here has to be measured at the value, never
/// interpolated to it.
///
/// The neutral of THIS axis is unchanged at 0.0 — what moved is the constant it
/// multiplies, so a record asking for a long face still gets one a sixth longer
/// than the default, and the default is now the right length to be a sixth of.
///
/// Provenance: **derived** (#61), from the budget headroom measured across the
/// space rather than from anthropometry — the anthropometry is what says where
/// the DEFAULT should be, and it is recorded above rather than applied.
const FACE_LENGTH_SPAN: f32 = 0.17;

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

/// Every number the biped cage is built from.
///
/// One flat record rather than a tree, and named for the body rather than for
/// the arithmetic: a reader who wants to know how wide a knee is looks up
/// `knee_r`, and a reader who wants to know *why* reads the derivation beside
/// it in [`Dimensions::of`].
///
/// **Positions are half-bodies.** Everything lateral — `hip_x`, `clavicle_x`,
/// the arm's chain — is given for the `+X` side only, and the assembler mirrors
/// it. That is not a saving; it is what keeps a body symmetric by construction
/// rather than by two expressions agreeing.
///
/// **Where this is bounded by the mesher rather than by anatomy**, because a
/// composite axis fanning in here will meet these before it meets any range on
/// its own slider:
///
/// ```text
///   girth        0.50 .. 1.72   radii scale with it and lengths do not, so
///                               the top end leaves what a hull can close
///                               (wall 1.82); the bottom is editorial, no
///                               wall exists above the reachable 0.16
///   hip_x       >= 0.49  · pelvis_r   below it the two hip sockets overlap
///                                     (wall 0.468); does not bind inside ±1
///   clavicle_x  >= 1.41  · girdle_r   below it the girdle's neck socket runs
///                                     out of bone (wall 1.36)
///   extremity    0.55 .. 1.36   the foot is a chain of small nodes on a
///                               stature-fixed length (walls 0.41 and 1.42);
///                               the bottom end is editorial too
/// ```
///
/// **Every figure in that table was re-measured in #163's audit rather than
/// carried forward**, each against the gate it actually stands on: a
/// coefficient against the 1500 random bodies and the corners, a saturation
/// against the 400 rerolled records. Two moved, two were confirmed, and two of
/// the six ends turned out not to be mesher walls at all. Each is derived
/// beside its own expression.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dimensions {
    /// Height of the pelvis node above the ground plane.
    pub pelvis_y: f32,
    /// Radius of the pelvis node.
    pub pelvis_r: f32,
    /// Cross-section of the pelvis node.
    pub pelvis_section: Vec2,
    /// Height of the waist connector.
    pub waist_y: f32,
    /// Radius of the waist connector.
    pub waist_r: f32,
    /// Cross-section of the waist connector.
    pub waist_section: Vec2,
    /// Height of the chest connector.
    pub chest_y: f32,
    /// Radius of the chest connector.
    pub chest_r: f32,
    /// Cross-section of the chest connector.
    pub chest_section: Vec2,
    /// Height of the shoulder girdle.
    pub girdle_y: f32,
    /// Radius of the shoulder girdle.
    pub girdle_r: f32,
    /// Cross-section of the shoulder girdle.
    pub girdle_section: Vec2,
    /// Where the neck's joint sits, behind the midline by [`NECK_BACK`].
    pub neck_at: Vec3,
    /// Radius of the neck node.
    pub neck_r: f32,
    /// Cross-section of the neck node.
    pub neck_section: Vec2,
    /// How far astern of its own joint the neck's section is swept. See
    /// [`NECK_LOBE`]: the joint stays on the axis and the mass goes behind it.
    pub neck_offset: Vec2,
    /// Height of the head's joint.
    pub head_y: f32,
    /// Radius of the head node.
    pub head_r: f32,
    /// Cross-section shared by the head and crown nodes.
    pub skull_section: Vec2,
    /// Height of the crown node.
    pub crown_y: f32,
    /// Radius of the crown node.
    pub crown_r: f32,
    /// Where the jaw's hinge marker sits.
    pub jaw_pivot_at: Vec3,
    /// Marker radius of the jaw's hinge.
    pub jaw_pivot_r: f32,
    /// Where the jaw's chin marker sits.
    pub jaw_tip_at: Vec3,
    /// Marker radius of the jaw's tip.
    pub jaw_tip_r: f32,
    /// How far out the clavicle sits, on the `+X` side.
    pub clavicle_x: f32,
    /// Height of the clavicle.
    pub clavicle_y: f32,
    /// Radius of the clavicle node.
    pub clavicle_r: f32,
    /// Where the arm's chain begins, on the `+X` side.
    pub shoulder_at: Vec3,
    /// Radius of the shoulder node.
    pub shoulder_r: f32,
    /// Where the elbow sits, on the `+X` side.
    pub elbow_at: Vec3,
    /// Radius of the elbow node.
    pub elbow_r: f32,
    /// Where the wrist sits, on the `+X` side.
    pub wrist_at: Vec3,
    /// Radius of the wrist node.
    pub wrist_r: f32,
    /// Where the hand's stub sits, on the `+X` side.
    pub hand_at: Vec3,
    /// Radius of the hand's stub.
    pub hand_r: f32,
    /// How far out the leg's chain runs, on the `+X` side.
    pub hip_x: f32,
    /// Height of the hip socket.
    pub hip_y: f32,
    /// Radius of the hip node.
    pub hip_r: f32,
    /// Height of the knee.
    pub knee_y: f32,
    /// How far forward of the hip-to-ankle line the knee stands.
    pub knee_z: f32,
    /// Radius of the knee node.
    pub knee_r: f32,
    /// Height of the ankle.
    pub ankle_y: f32,
    /// Radius of the ankle node.
    pub ankle_r: f32,
    /// Height of the foot's level run.
    pub foot_y: f32,
    /// Cross-section shared by every node of the foot.
    pub sole_section: Vec2,
    /// How far behind the ankle the heel sits.
    pub heel_z: f32,
    /// Radius of the heel node.
    pub heel_r: f32,
    /// Height of the stub that closes the heel.
    pub cap_y: f32,
    /// How far behind the ankle that stub sits.
    pub cap_z: f32,
    /// Radius of that stub.
    pub cap_r: f32,
    /// How far ahead of the ankle the ball sits.
    pub ball_z: f32,
    /// Radius of the ball node.
    pub ball_r: f32,
    /// How far ahead of the ankle the toe sits.
    pub toe_z: f32,
    /// Radius of the toe node.
    pub toe_r: f32,
}

/// How much more body there is than neutral, and how much of it is fat.
///
/// **The single most important thing here is that this is two numbers and not
/// one.** The axis it replaces was `1 + 0.28 · build + 0.15 · muscle`, one
/// factor on every radius from the wrist to the waist, which is a body that
/// inflates like a balloon: the only heavy body it could draw was a neutral one
/// scaled up and the only slight one was the same body scaled down. Nothing
/// about it could say that a heavy person's waist grows four times as much as
/// their wrist, or that a muscular one's shoulders grow while their waist does
/// not. Both of those are measured facts, and they are the two axes here:
/// [`Composites::mass`] sets how much body there is and
/// [`Composites::body_fat`] decides how it is spent.
///
/// ## The exponents are the measurement
///
/// A circumference tracks body size neither linearly nor uniformly. Heymsfield
/// et al. (2008) fitted circumference against volume-over-height across five
/// sites and found the powers spread over nearly a factor of two — and,
/// usefully here, found them to differ by sex in exactly the pattern the
/// android/gynoid story predicts:
///
/// ```text
///            male   female
///   arm      0.61    0.62
///   waist    0.62    0.61
///   hip      0.42    0.49
///   thigh    0.50    0.54
///   calf     0.38    0.33
/// ```
///
/// "Males enlarging at a greater rate around the waist and calf and females
/// around the hips, thighs" is that paper's own summary, and it is the whole of
/// the flagship cross-composite formula this was raised for (#164): the
/// android/gynoid blend is not authored, it is [`between`] applied to a
/// measured pair. See [`Girth::at`].
///
/// At a fixed height a body that grows only sideways has radius ∝ volume^0.5,
/// so **0.5 is the neutral exponent here and not 0.333**: the waist and the arm
/// sit above it, the hip and the calf below. Nevill et al. (2004) reach the
/// same conclusion from body mass across 478 subjects — fleshy sites gain
/// faster than geometric similarity and bony ones slower, with head girth
/// "almost constant, irrespective of subjects' body size/mass". That is where
/// the wrist and ankle figures come from, and why the head carries no factor at
/// all.
///
/// Provenance: **looked up** for the five measured sites and for the
/// bony-and-invariant pattern; **interpolated** for the chest, girdle, forearm
/// and neck, which neither paper measures and which are placed between the
/// sites that bracket them. Every pair is written at its own use site rather
/// than in a table here, so the two figures a reader wants — which exponents,
/// and how much of the site is fat — sit beside the radius they set.
///
/// Sources, added to `docs/research/research-02-parametric-bodies.md`:
/// Heymsfield et al., *Body circumferences: clinical implications emerging from
/// a new geometric model*, Nutr Metab 2008 (PMC2569934); Nevill et al., *Are
/// adult physiques geometrically similar? The dangers of allometric scaling
/// using body mass power laws*, Am J Phys Anthropol 2004 (PMID 15160370).
#[derive(Clone, Copy, Debug)]
struct Girth {
    /// Lean volume per unit height, relative to the neutral body.
    lean: f32,
    /// Fat volume per unit height, relative to the neutral body.
    fat: f32,
    /// Where this body sits on the frame axis, which splits several exponents.
    femininity: f32,
}

impl Girth {
    /// Reads the two composites into a lean budget and a fat one.
    ///
    /// `mass` is a slider and not a weight, so it spans a *ratio* of volume per
    /// height rather than an interval of it: a body twice as heavy is twice as
    /// heavy whichever end you start from, and a linear span would make the
    /// slight end of the axis do more than the heavy end. `MASS_SPAN` is chosen
    /// so the waist at the top of the axis lands where the old `build` put
    /// EVERY radius at its top — `1.49^0.615` is 1.28 and the retired
    /// coefficient was 0.28. That is the one calibration in here, and it is
    /// deliberately against the thing being replaced rather than a fresh guess.
    fn of(composites: &Composites, femininity: f32) -> Self {
        /// Volume per unit height at `mass` +1, relative to neutral.
        const MASS_SPAN: f32 = 1.49;
        let bulk = MASS_SPAN.powf(composites.mass);
        // Fat and lean are shares of ONE budget, so raising the fraction at a
        // fixed mass takes volume out of the lean sites and puts it in the fat
        // ones. That is what makes `body_fat` a recomposition axis rather than
        // a second size axis: low fat with high mass is muscular, high fat with
        // high mass is heavy, and the waist and the deltoid are what disagree
        // about which is which.
        // **Fat is added to the frame rather than taken out of it**, which is
        // the difference between an axis that describes weight gain and one
        // that describes recomposition at a fixed volume. A body that puts on
        // fat gets bigger; it does not trade its shoulders for its belly. So
        // `mass` sets how much frame there is and this rides on top of it.
        //
        // `SOFT` is the tissue at a site that is not going anywhere. Without it
        // the fat budget spans a factor of twenty across the axis — 3% to 60%
        // of body fat really is that ratio — and any power of twenty takes a
        // lean body's waist to a third of its neutral one. What a very lean
        // person has is not an absent waist, it is a waist with the variable
        // layer gone, so the ratio is taken over `fat + SOFT` and the lean end
        // lands at about −24% instead of −42%.
        const SOFT: f32 = 0.08;
        let fat = composites.body_fat.clamp(0.0, 0.95);
        Self {
            lean: bulk,
            fat: bulk * (fat + SOFT) / (DEFAULT_BODY_FAT + SOFT),
            femininity,
        }
    }

    /// The factor at one site.
    ///
    /// `male` and `female` are that site's measured exponents and `fat_share`
    /// is how much of its growth is fat rather than lean — `0` for bone, `1`
    /// for a pure depot. At the neutral body every input is one and this
    /// returns exactly one, which is the identity the epic requires of a
    /// neutral composite.
    ///
    /// At a fixed body-fat fraction the two budgets are equal and this
    /// collapses to `bulk^p`, the published relation unmodified. The split only
    /// does something when the fraction moves, which is the point of it.
    fn at(&self, male: f32, female: f32, fat_share: f32) -> f32 {
        let power = between(self.femininity, male, female);
        self.lean.powf(power * (1.0 - fat_share)) * self.fat.powf(power * fat_share)
    }
}

/// The factor that carries one quantity between the two measured references.
///
/// **Every use of this names the two figures it interpolates**, rather than a
/// coefficient derived from them, so the anchors are readable at the site and a
/// re-measurement is a one-line edit. `examples/reference` prints them all off
/// the CC0 mannequins; `male` and `female` here are always fractions of that
/// body's own stature, which is the only form two bodies of different heights
/// can be compared in.
///
/// **It multiplies rather than replaces, and at neutral it is exactly one.**
/// The plan's own value stays the anchor and this moves it by the *ratio* the
/// references differ by:
///
/// ```text
///   frame(f) = 1 + f · (female − male) / (female + male)
/// ```
///
/// so `frame(+1) / frame(−1) = female / male` exactly, whatever the plan builds
/// at zero. That matters here more than it looks: this body's shoulder span is
/// 10.8% over the male reference and stays there (#106), and snapping the axis
/// onto the reference midpoint would have quietly re-litigated that decision on
/// the way past.
pub(crate) fn frame(femininity: f32, male: f32, female: f32) -> f32 {
    between(femininity, male, female) / ((male + female) * 0.5)
}

/// The value itself, rather than the factor: a plain interpolation whose ends
/// are the two references and whose middle is their midpoint.
///
/// [`frame`] is this divided by that midpoint. Both exist because a quantity
/// the plan already has an opinion about wants the factor — it keeps the plan's
/// own value as the anchor — and a quantity that IS the measurement, like an
/// allometric exponent, wants the value.
pub(crate) fn between(femininity: f32, male: f32, female: f32) -> f32 {
    (male + female) * 0.5 + femininity * (female - male) * 0.5
}

impl Dimensions {
    /// Derives every dimension of one biped from its record's axes.
    ///
    /// Read top to bottom: the girth composite, then the trunk it thickens,
    /// then the heights that hang off the trunk, then the limbs. Each
    /// coefficient carries its own provenance, and the ones that clamp say what
    /// wall they are standing against.
    pub(crate) fn of(params: &HumanoidParams, composites: &Composites) -> Self {
        let h = params.height;
        // The frame axis: where this body sits between the two measured
        // references (#100). Every quantity below that carries it names its own
        // pair of anchors at the site — see [`frame`] — because the axis is not
        // one gain, it is a dozen measurements that happen to move together.
        //
        // **The shoulder complex moves as ONE factor, taken from the chest.**
        // The clavicle's floor is `(0.31 · GIRDLE_SECTION.x · girdle_r + 0.69 ·
        // chest_r) / 0.805`, so the three are inseparable: scale them by
        // different factors and the ratio between `clavicle_x` and its own
        // floor moves, and at the masculine end it lands on it — measured at
        // 1.468 against a floor of 1.47, inside ±1, which is the exact defect
        // #100 was raised to fix. Scaling all three together leaves that ratio
        // invariant at every value of the axis, including out in the envelope.
        //
        // The chest is the one of the three to take the factor FROM, because it
        // is the one this body already sits on: the chest node measures 0.0912
        // of stature across against the male reference's 0.0911, where the
        // shoulder span is 10.8% over its own. Scaling by the shoulder's own
        // ratio would move a figure that is already wrong; scaling by the
        // chest's moves one that is right, and the shoulders ride it.
        //
        // Saturated for the exploration range, on the masculine side only. The
        // wall is the pelvis's spine socket: a masculine frame widens the waist
        // and narrows the pelvis at the same time, and the socket that has to
        // clear both runs out of hull at `femininity` −1.30 against the 1500
        // random bodies, with rolled ones failing from −1.409 when build and
        // limb length are co-extreme. −1.25 is that wall plus ~4%, the same
        // margin the other saturations in this file carry.
        //
        // **Clamped here rather than on the axis**, because this is the axis's
        // BODY reading and the skull (#166) and the fat distribution (#164)
        // read the same stored value without meeting this wall. A composite
        // saturated at its source would take a limit that belongs to one
        // consumer and impose it on the others.
        //
        // Swept the other way too, and the feminine side does not wall: the
        // gate holds to +2.95, so +2.85 is a margin rather than a limit. It is
        // written down so that a later term that does bite up there has
        // somewhere obvious to say so.
        //
        // The waist alone was tried first and is not enough. Clamping its own
        // factor to 1.06 moved the wall from −1.30 to only −1.75 and bound
        // inside ±1 while doing it — the pelvis narrowing is as much of the
        // squeeze as the waist widening, and a clamp on either one alone pays
        // for half of it at full price.
        let femininity = composites.femininity.clamp(-1.25, 2.85);
        let girth = Girth::of(composites, femininity);
        let shoulder_frame = frame(femininity, 0.0911, 0.0702);
        // **Two things fall out of that factor that nobody asked for, and both
        // are worth having written down before they get discovered as bugs.**
        //
        // `chest_gap` is 1.3 girdle radii, so scaling `girdle_r` moves the
        // chest NODE as well as widening it: the chest landmark runs 0.6759,
        // 0.6916, 0.7022 of stature across the axis, against references at
        // 0.7007 male and 0.7187 female. The direction is right and the two
        // ends sit about equally under their own anchors — but the axis moves
        // the landmark by 0.026 of stature where the references differ by
        // 0.018, so it OVERSHOOTS by about half again. Correcting it means
        // taking the frame out of `chest_gap` alone, which changes the girdle's
        // socket geometry rather than its size, and #106's second comment is a
        // long record of why that is not a small edit.
        //
        // The masculine end also renders about 0.75% taller (1.737 m against
        // 1.724), because a wider girdle lengthens `torso_min` and `girdle_y`
        // takes its floor from it. `height` is supposed to own stature, so that
        // is a wart rather than a feature; it is under a centimetre and it is
        // in the noise of what `limb_length` already does to the same floor.

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
        // The pelvis flares and the waist comes in, which between them are most
        // of what the axis reads as. Both anchors are taken in a band centred on
        // that body's OWN bone rather than at a fixed fraction of stature: the
        // female reference's pelvis sits at 0.5179 and the male's at 0.5013, so
        // a fixed band holds her pelvis against his abdomen and reports the
        // difference between two heights as a difference between two bodies.
        let pelvis_r = h * 0.079 * girth.at(0.42, 0.49, 0.60) * frame(femininity, 0.0910, 0.1019);
        // **The waist has almost no lateral headroom and a great deal of
        // fore-and-aft, so the belly goes where a belly goes.** #106 swept this
        // node's ceiling at 0.088 of stature against a base of 0.074 — 17% —
        // because the pelvis's spine socket blends toward it and the two hip
        // sockets have to clear the result. A body-fat axis that put a full
        // waist into that 17% would spend its whole range before reaching a
        // fat person.
        //
        // The clearance a socket demands is its LARGEST half-extent, which on
        // this node is the lateral one: [`WAIST_SECTION`] is 0.76 deep against
        // 1.0 across, so depth is free until it catches up. That is also the
        // anatomy — an abdomen projects forward far more than it spreads
        // sideways — so the lateral term takes a reduced share of the fat and
        // the section below takes the rest.
        let waist_r = (h * 0.074 * girth.at(0.62, 0.61, 0.45) * frame(femininity, 0.0706, 0.0618))
            .min(pelvis_r * 1.05);
        // The belly, as depth rather than width. Capped at 1.0 — a round waist
        // — because past there depth becomes the largest half-extent and starts
        // costing the same clearance the width was avoiding.
        let waist_section = Vec2::new(
            WAIST_SECTION.x,
            (WAIST_SECTION.y * girth.at(0.62, 0.61, 0.75)).min(1.0),
        );
        // **The chest and the girdle take ONE factor and the same clamp**, for
        // the reason #100 records: `clavicle_x`'s floor is a weighted sum of
        // the two, so anything that moves them apart moves the shoulder
        // relative to its own floor and eventually onto it. Giving the ribcage
        // a fat share and the shoulder shelf a lean one was tried first and
        // failed at `body_fat` 0.40 on the DEFAULT body, at exactly that
        // socket.
        //
        // The ceiling is the girdle's other constraint: its neck socket has to
        // clear its siblings, which `neck_y`'s floor pays for in bone at 1.02
        // girdle radii. Let the complex grow without limit and a heavy body
        // either fails to mesh — measured at `mass` +2.29 — or, if that floor
        // is raised to cover it, grows a longer NECK, because 1.12 radii is
        // more bone than the neck's own length term asks for at neutral. A
        // heavy person has a thicker neck, not a longer one, so the cap goes
        // here rather than there.
        let ribcage = girth.at(0.55, 0.55, 0.25);
        // The girdle is bone where the ribcage is not, so it grows far less —
        // and it has to, because `neck_y`'s floor buys this node's socket
        // clearance in NECK BONE at 1.02 girdle radii. A girdle that inflated
        // with the ribcage would lengthen the neck by a quarter on an ordinary
        // heavy body, which is measurably what happened when it did: the
        // shoulder-line ratio in `the_neck_is_the_length_of_a_neck` went from
        // 0.48 to 0.64. A heavy person has a thicker neck, not a longer one.
        let chest_r = h * 0.086 * ribcage * (1.0 + 0.08 * params.shoulder_width) * shoulder_frame;
        let girdle_r = h * 0.062 * ribcage * (1.0 + 0.06 * params.shoulder_width) * shoulder_frame;
        // A neck is a good deal narrower than the skull above it. At the old
        // figure it measured WIDER than the head — 0.098 m against 0.093 — which
        // reads as a tree trunk and, worse, swallows the jaw: the chin is shaped
        // and narrows properly, but a neck two and a half times its width leaves
        // nothing of it to see.
        //
        // **0.030 × girth → 0.040 + 0.020 × (girth − 1), and the base and the
        // gain moved for different reasons** (#131). Both halves are measured
        // against the CC0 reference with `examples/column`, whose plane cuts are
        // anchored on each body's own chin.
        //
        // THE BASE, because the neck was a quarter narrow and nothing had ever
        // measured it against a surface. `NECK_SECTION.x` is 1.0, so this
        // coefficient *is* the neck's lateral half-extent, and the built surface
        // hands back 0.87–0.95 of it. Half-width at the chin, every seed scaled
        // to the reference's own 1.829 m stature:
        //
        // ```text
        //             seed 0  seed 3  seed 7  seed 13  seed 21   (reference 71.2)
        //   0.030       54.6    53.6    63.5     57.3     65.3
        //   0.040/.020  65.5    64.5    70.0     67.8     71.5
        // ```
        //
        // THE GAIN, because the spread that made this look already-solved on the
        // heavy end is stature, not width. Seed 21 reads 70.4 mm at 0.030 where
        // the reference reads 66.7, which says the widening would overshoot —
        // and seed 21 is a 2.03 m body. Divided by its own stature it reads
        // 63.4, narrow like every other seed. What the old multiplicative form
        // then does is spend that headroom twice: at a flat 0.042 the light
        // seeds land on the reference and the heavy ones sit 15% past it. The
        // softened gain collapses the stature-normalised spread from 65–78 to
        // 64–70 about a reference 66.7, and it is the one place in this file
        // where a radius does not simply multiply `girth` — deliberately, and
        // only because there is a measurement here and none on the torso.
        //
        // **What it buys on the graph, which is the half #131 was actually
        // raised for.** #125 could not hang a trapezius node anywhere: off the
        // girdle it missed by 12.7 mm, off the neck by two tenths. That was
        // read as a coincidence waiting to break. It is not — the binding rule
        // is [`CageError::SocketsOverlap`] and not the plane test, and the two
        // scale differently: two sockets sit at `socket_distance × hub.radius`
        // along their own directions, so their SEPARATION grows exactly with
        // this coefficient, while the clearance they need is a ring radius
        // blended toward neighbours that do not move. Measured on the default
        // body, a probe node behind and above the girdle's crown:
        //
        // ```text
        //   0.030   separation 56.8 mm   needed 75.7   —  and at every other
        //                                                 placement tried
        //   0.040   separation 85.9 mm   needed 88.6   —  and five placements
        //                                                 mesh outright
        // ```
        //
        // So the ring is open for the first time. The node itself is NOT
        // shippable yet and stays on #131: every placement that meshes on the
        // default body still fails the 1500-body sweep, and it fails on tall
        // heavy bodies because a probe measured in GIRDLE radii outgrows a neck
        // whose girth gain has just been softened. It wants placing in neck
        // radii.
        //
        // **Bounded above, twice, and the tighter bound is not meshability.**
        // This meshes to 0.068 across `tests/plan.rs` — the failure at 0.070 is
        // a girdle hull with unmatched edges, not the head's neck socket — so
        // there is more than twice the headroom anyone assumed. What actually
        // stops it is `the_neck_is_the_length_of_a_neck`, whose 0.44 is a
        // ratchet on the state: its shoulder line is found at 1.5× the
        // narrowest half-width OF THE NECK, so widening the neck raises the
        // ruler as well as the thing measured and the span grows for free.
        // Small — seed 7 goes 0.437 to 0.443 — but the bound has 0.003 of slack
        // and 0.042 spends it. 0.042 with a 0.018 gain is the better surface
        // fit (a median 69.3 against 71.2) and is refused on that ground alone;
        // raising the ratchet to buy neck width would undo three passes of
        // bringing it down.
        //
        // Provenance: **derived from the reference surface** (#131), swept for
        // meshability and bounded by the neck-length ratchet. Was **tuned by
        // render** (commit `0d7684f`) against the argument above — a measured
        // 0.098 m neck against a 0.093 m head — which was the right shape of
        // argument for a tuned number and was still a quarter short of life.
        //
        // **And it takes the frame axis, which is the last of #166's set to land
        // and the one that needed the instrument fixed first** (#166). A neck is
        // one of the more dimorphic parts of a body and this coefficient could
        // not say so. Measured on the two mannequins by `examples/headref`, as
        // the mean radius of the section at mid-neck over stature: masculine
        // 0.05387, feminine 0.04411.
        //
        // **The first measurement said the opposite and the instrument was the
        // reason**, which is worth the space because this file has a dozen
        // coefficients that could be read the same wrong way. Taking `max |x|`
        // over the neck's VERTICES in a band read the feminine neck as 14% the
        // thicker of the two. Two faults at once: a neck is not round, so one
        // axis of an ellipse is not its size — the two mannequins' necks are
        // within 3% ACROSS and 23% apart THROUGH, and the feminine one is
        // simply flatter, 0.642 deep for its width against 0.762 — and the male
        // mesh carries 89 neck vertices against the female's 237, so a band
        // statistic on him missed his widest ring outright and under-read him
        // by a third. Rays at every azimuth from the neck's own axis have
        // neither fault. Checked before it was believed: taking the nearest
        // crossing rather than the farthest changes nothing, so no ray was
        // escaping into the shoulder.
        //
        // **The pair is used at its measured ratio and that is a judgement**,
        // not an oversight. It is 22% end to end where life's neck
        // circumferences put the same dimorphism nearer 8%, so these two
        // mannequins exaggerate it about as much as they exaggerate the chin —
        // which #166 refused to spend in full. The chin is taken at a third and
        // this at all of it, because they are different kinds of measurement: a
        // chin projection is one sculpting decision about one landmark, and a
        // section's mean radius is bulk. This coefficient is also already
        // calibrated against these same surfaces (#131 above), so the ratio and
        // the base are in the same units; tempering one and not the other would
        // put the axis's ends somewhere neither source describes.
        //
        // Provenance: **derived from the reference mannequins** (#166), on top
        // of the derivation above.
        let neck_r = h
            * (0.040 + 0.020 * (girth.at(0.30, 0.30, 0.25) - 1.0))
            * frame(femininity, 0.05387, 0.04411);
        // Provenance: **unsourced**, both the 0.075 and the 0.25 gain. 0.075 of
        // stature is close to the eight-head figure's head, but the eight-head
        // figure specifies head HEIGHT and this is a node RADIUS, so the
        // resemblance is not a derivation and must not be written up as one —
        // the built head is 214 mm tall on a 160 mm breadth (#79), which is not
        // a number this coefficient predicts.
        let head_r = h * 0.059 * (1.0 + 0.25 * params.head_size);

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

        let pelvis_y = (h * (0.5 + 0.03 * params.limb_length))
            .min(nominal_girdle - torso_min)
            .max(ankle_y + hip_drop + h * 0.10);
        let girdle_y = nominal_girdle.max(pelvis_y + torso_min);
        let chest_y = girdle_y - chest_gap;
        // Provenance: **derived** — the waist is the midpoint of pelvis and
        // chest, which is a definition rather than a measurement. The 0.02
        // clearance that keeps it off the chest is **unsourced**.
        let waist_y = (pelvis_y + (chest_y - pelvis_y) * 0.5)
            .clamp(pelvis_y + pelvis_gap, chest_y - h * 0.02);

        // Hoisted out of the limb ladder below, because the neck's floor is a
        // clearance against the clavicle's socket and cannot be computed without
        // it. See [`Dimensions::clavicle_r`] for what the pair mean.
        let arm_upper = frame(femininity, 0.0369, 0.0304);
        let clavicle_r = h * 0.040 * girth.at(0.61, 0.62, 0.35) * arm_upper;

        // The girdle is a joint, so its neck socket has to clear the clavicles'.
        // The neck above it is a plain connector and constrains nothing — an
        // earlier floor here was invented rather than required, and it alone
        // added half a head-height of giraffe.
        //
        // **The floor no longer binds on a neutral body, and this note said it
        // did for two coefficient changes after it stopped being true** (#129).
        // It was written when the pair were 1.32 and 0.072, where the floor gave
        // 143.2 mm against a nominal of 126.0; #107 took them to 1.02 and 0.064
        // and the sentence stayed. On the default body the nominal is 112.0 mm
        // and the floor is 110.7 — the NOMINAL binds, by 1.3 mm. The claim was
        // still being quoted as the reason the neck could not be shortened when
        // #129 was opened, so it is corrected here rather than deleted.
        //
        // **What is true is that the floor binds on most bodies but not on the
        // middle of the space.** Over the distribution `tests/plan.rs` samples
        // it binds on 59.7% of them, because the floor carries `girth` and
        // `shoulder_width` through `girdle_r` and the nominal carries neither.
        // So `neck_length` is ONE-SIDED: at neutral girth the two cross at
        // −0.04, and everything below that moves nothing at all. Multiplying the
        // nominal by `girth` as well does not fix it — both sides then scale
        // together and the crossover does not move. Only the floor coming down
        // frees the short half, and the floor is a meshing constraint rather
        // than a proportion.
        //
        // Down from 1.32 (#93): at 1.32 the chin sat 103.0 mm above the shoulder
        // line on a 214.6 mm head, a ratio of 0.480, where the eight-head figure
        // puts the shoulder about a third of a head below the chin. 1.15 gives
        // 88.1 mm and 0.411.
        //
        // **And this line is a much weaker lever on the visible neck than the
        // three passes that tuned it assumed** (#129). `examples/neckaudit`
        // breaks the chin-to-shoulder span into its four owners, and the neck
        // BONE's share of it is 2.1 to 5.3 mm across the guarded seeds, against
        // 42.6 to 57.8 mm of head-owned surface hanging below the chin. The
        // reason is `girdle_r` on both sides of this expression: the girdle's
        // crown is `girdle_y + girdle_r` and the neck joint is `girdle_y` plus a
        // floor of `1.02 · girdle_r`, so the two sit within a few millimetres of
        // each other by construction and shortening the neck moves the head down
        // onto a shoulder that came up to meet it. Measured: taking the pair to
        // 0.060 and 0.96 drops the neck joint 7.0 mm and buys 1.5 mm of visible
        // neck on seed 0.
        //
        // **Today's meshing cliff, re-measured on the eight-point cage** (#129):
        // 0.92 passes `tests/plan.rs`'s 1500 bodies and 0.90 fails, on the
        // girdle's own neck socket — `at joint 3, the socket toward node 4 must
        // sit 0.0540 from the joint centre to clear its siblings, but its bone
        // only allows 0.0507`. The sweep recorded below found 1.00 passing and
        // 0.85 failing, which was the four-point ring; the cliff moved with it.
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
        // 1.16 this floor stopped binding on the default body — at 0.064 that
        // crossover is 1.032, so it is behind us and the note is kept only
        // because the shape of the argument still holds: lowering the floor
        // alone changes nothing in the middle of the space while still failing
        // extreme bodies. Reaching the canon's 0.33 needs the 0.072 down too,
        // and that steepens the shoulder ramp into a coat-hanger: the same flare
        // over less height. And it must not come from ABOVE — lowering the head
        // by cutting `HEAD_BELOW_JOINT` takes it out of the lower face, which
        // #78 raised to fix a face measuring 39% short, and cranium:face is
        // exactly 1.00 now.
        //
        // **Both figures came down for the eight-point cage — 0.072 to 0.064 and
        // 1.15 to 1.02 — and the paragraph above is why it took both** (#107).
        // The cage moved a great deal more of the head BELOW its own joint: the
        // head's surface used to run out around 0.75 radii under the joint and
        // now runs to 1.07, measured over sixteen seeds. The exposed neck did not
        // change; the head above the chin, which is what
        // `the_neck_is_the_length_of_a_neck` divides by, shrank about 15%, and
        // the ratio went from 0.404–0.476 across its five seeds to 0.492–0.567
        // against a 0.52 bound.
        //
        // **Neither radius is the lever, and both were swept before this was
        // touched.** `head_r` from 0.059 to 0.071 moves the worst seed only from
        // 0.567 to 0.523, because a larger head also sits higher — `head_y`
        // follows `head_r` — and exposes the neck it was meant to cover, so the
        // two effects very nearly cancel. `neck_r` from 0.030 to 0.025 is worth
        // 0.02. The neck's own LENGTH is the only term that moves it, which is
        // what the note above already said from the other direction.
        //
        // That `neck_r` sweep was run at a base of 0.030 and #131 has since taken
        // it to 0.040, in the direction the sweep called cheap. It measured the
        // same: the ratio moves 0.006 on the worst seed, and it moves for the
        // instrument rather than for the body — the shoulder line is found at
        // 1.5× the narrowest half-width OF THE NECK, so a wider neck raises the
        // ruler along with the thing being measured. The 0.44 bound below has
        // 0.003 of slack left because of it.
        //
        // So the warning above — that taking 0.072 down steepens the shoulder
        // ramp into a coat hanger — was written against a trunk measuring 13–29%
        // narrow. It is not that trunk any more: the same silhouette now runs
        // within about 9% of the reference across the middle bands with no
        // coefficient changed, so there is flare to spend the height against.
        // At 0.064 and 1.02 the five seeds read 0.435–0.493.
        //
        // Provenance: **derived from a sweep against the canon** (#93, re-swept
        // for the cage in #107), bounded below by socket clearance — 1.32, 1.15,
        // 1.02 and 1.00 all mesh across `tests/plan.rs`'s 1500 random bodies and
        // its corners; 0.85 does not.
        // **1.02 → 1.12, and it is a debt rather than a tuning** (#164, #174).
        // This floor buys the girdle's neck-socket clearance in neck BONE, and
        // the allometric girth grows the girdle enough that 1.02 stopped
        // clearing: five of four hundred rolled records failed to mesh at this
        // socket, which breaks the crate's one invariant. 1.12 is where the
        // gate goes green.
        //
        // What it costs is measured and is not small. 1.12 girdle radii exceeds
        // the neck's own length term at neutral, so this binds on EVERY body
        // rather than on heavy ones: the default body's rendered height went
        // 1.724 m to 1.733, the shoulder-line ratio in
        // `the_neck_is_the_length_of_a_neck` moved its classic population from
        // 0.46–0.56 to 0.46–0.64, and two face rulers had to be re-based
        // because the skull's `throat..crown` span is what every head
        // measurement in this crate is taken over.
        //
        // Everything cheaper was tried and measured. Capping the ribcage makes
        // it WORSE — 1.30 takes the failures from 5 to 20, because the trunk
        // stops growing while the waist and pelvis do not. Clamping `mass`
        // barely moves it, 5 to 3, because the failing bodies are femininity ×
        // body-fat interactions rather than heavy ones. Shallowing the girdle
        // back toward its pre-#100 depth takes 5 to 1 and gives up that issue's
        // upper-trunk depth to do it. #174 owns the real fix, which is that
        // every bone in this trunk is a multiple of `girdle_r` and so the trunk
        // cannot thicken without lengthening.
        //
        // **1.12 girdle radii → the clearance itself, and the bone #164 borrowed
        // comes back from the bodies it was taken from** (#174). Everything
        // above is kept because the mechanism it describes is real; what was
        // wrong was the UNIT.
        //
        // The socket this floor buys room for is the girdle's, pointing at the
        // neck, and what it has to clear is the pair of CLAVICLE sockets beside
        // it. Their reach along the neck's own axis is one number — the vertical
        // half-extent of a clavicle socket's ring — and that ring is neither the
        // girdle's section nor the clavicle's but a blend of the two, because
        // [`Socket::set_dist`] lerps a ring toward its neighbour as it slides
        // out. So the demand is `(1 − b) · girdle depth + b · clavicle_r`, over
        // [`MAX_SOCKET_FRACTION`] because that is all of the bone a socket may
        // spend.
        //
        // **Two things in that were being missed by a bare multiple of
        // `girdle_r`, and they are different mistakes.**
        //
        // The girdle's own term is its DEPTH and not its radius. A lateral bone's
        // ring frame is transported from world up, so on the clavicle sockets
        // [`GIRDLE_SECTION`]`.y` — the fore-and-aft figure everywhere else in
        // this file — lands VERTICALLY, which is the one direction the neck
        // socket cannot afford. #100 took that constant 0.80 → 1.10 and recorded
        // that `.y` "does not appear in the clavicle floor at all, so it buys
        // upper-trunk depth outright". True of the clavicle's LATERAL floor and
        // false here, and the 37% it grew by is most of why 1.02 stopped
        // clearing. Naming the constant in the expression is what stops the two
        // drifting apart again.
        //
        // The clavicle's own term is not a multiple of the girdle at all — it is
        // `h · 0.040 · girth.at(0.61, 0.62, 0.35) · arm_upper` against the
        // girdle's `h · 0.062 · girth.at(0.55, 0.55, 0.25) · …`, a different
        // girth exponent and a different frame factor. Over the 400 rolled
        // records `clavicle_r / girdle_r` runs 0.53 to 0.88, and the clavicle is
        // 62% of the ring. That spread is the whole defect: a floor priced in
        // girdle radii has to cover the body with the fattest shoulder for its
        // girdle, and then every other body pays it.
        //
        // **What the population actually demands, bisected per body** — the neck
        // bone driven directly, with the length term below overridden so it
        // cannot hide the wall. That probe was a throwaway; what is kept is
        // `examples/envelope`, which walks the same 400 seeds this gates on and
        // names every one that fails to build:
        //
        // ```text
        //   demanded bone, in girdle radii   0.79  …  1.08  (397 of 400)
        //   this expression delivers         0.91  …  1.18
        //   the default body demanded        0.909, and was given 1.12
        // ```
        //
        // So the constant was not merely too high, it was the wrong shape: it
        // over-served the median body by a fifth and still had to be raised to
        // cover the tail. Priced properly the default body's floor lands at
        // 0.998 girdle radii — under the 1.032 crossover this file's earlier note
        // computes — so it does not bind on an ordinary body at all, and the
        // neck goes back to being the length its own term asks for.
        //
        // **Three of the 400 are excluded from that fit and are not this
        // socket.** Seeds 0, 46 and 125 refuse a short neck at the girdle's
        // CLAVICLE socket rather than its neck socket — shortening the bone
        // pulls the neck socket in toward the girdle, where its ring is wider,
        // and the clavicle's plane has to clear that. They demand 1.31–1.76
        // girdle radii and are covered anyway, because their own length term is
        // longer than either figure. A body that ever fails there wants
        // `clavicle_x`, not this.
        //
        // Provenance: **derived from the mesher, then bounded by bisection**
        // (#174). See [`CLAVICLE_RING_BLEND`] for the blend and its margin.
        //
        // [`Socket::set_dist`]: crate::cage::Socket::set_dist
        let neck_clearance = (GIRDLE_SECTION.y * girdle_r * (1.0 - CLAVICLE_RING_BLEND)
            + clavicle_r * CLAVICLE_RING_BLEND)
            / MAX_SOCKET_FRACTION;
        let neck_y = girdle_y + (h * 0.064 * (1.0 + 0.3 * params.neck_length)).max(neck_clearance);
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
        // **1.19 -> 1.55, and it is the face's half of #79's missing quarter.**
        // Everything above is the derivation of 1.19 and it is still correct
        // arithmetic; what changed underneath it is the head it was solved for.
        // #107 re-derived `head_r` from 0.075 of stature to 0.059 and flipped
        // the cage to eight-point rings, and the head came out 161 mm crown to
        // chin against a breadth of 160 — a face frame of 78.3 mm, 68% of the
        // 114.7 above, where #78 had left it at 92. The chain above solves for a
        // cranium:face of 1.0 at whatever height the crown is; it cannot notice
        // that the whole head is a quarter short, because the ratio is right at
        // any size. So this moves WITH [`CROWN_HIGH`], not instead of it: the
        // pair takes the head to 201.8 mm at a cranium:face of 0.999, and the
        // frame to 100.9 mm — 88% of that 114.7.
        //
        // Not to 100%, and deliberately. 114.7 is the frame a head would want
        // if it were sized for its own breadth, which is a 236 mm head; the
        // owner's call is the eight-head canon on the body's own rendered
        // stature, which asks about 206 and lands this at 8.4 heads against
        // 10.2 before. The remaining 12% is that target's, not this one's.
        //
        // **What it costs, measured rather than feared:** +590 triangles on the
        // default body and +188 at the dearest corner of the parameter space,
        // both of which `CROWN_HIGH` more than pays back — a taller vault sits
        // above `FACE_PASSES`'s above-joint ceilings, and the vault carries no
        // features. The dearest body anywhere in the space fell from 29,886 to
        // 29,092 across this whole change.
        //
        // RAISED rather than dropping `neck_y` to hold stature, which is what #78
        // asked for and is barely available: `neck_y` sat exactly on its
        // girdle-socket floor when this was written — 143.2 mm against a nominal
        // of 126.0 — and since #107 it does not. The floor is 110.7 mm against a
        // nominal 112.0, so there is 1.3 mm of travel before the socket rule
        // takes over and only about 9 more once the floor comes down to where
        // `tests/plan.rs` still meshes (#129). Raising is sound because a built
        // body already stands about 6% under its nominal stature — the crown
        // collapses under subdivision — so this spends height the body was
        // already missing.
        // Provenance: **derived** (#78), and the whole chain is written out
        // above rather than summarised — which is what this tag is supposed to
        // mean. It is the one coefficient in this file whose first derivation
        // was recorded alongside its correction.
        //
        // **It is an axis now, and this is the constant it multiplies** (#61).
        // The derivation above stands unchanged at the neutral value: what the
        // axis says is that a cranium:face of exactly 1.0 is the middle of a
        // range rather than the only answer, since people whose faces are long
        // for their heads and people whose faces are short for theirs are the
        // most legible single difference between two skulls. See
        // [`FACE_LENGTH_SPAN`].
        // **1.55 to 1.50, and it moved because the NECK moved** (#125). This
        // coefficient is not independent of the shoulders and nothing had said
        // so. `face::skull::shape` normalises its whole below-joint domain by
        // the head's measured FLOOR, and the floor is where the head's surface
        // runs into the neck — so giving the neck a backward offset moved the
        // floor, which stretched the lower face for free and in the wrong
        // direction. Left at 1.55 the head came out 210.5 mm crown to chin with
        // a cranium:face of 0.92, against #79's 201.8 and 1.00.
        //
        // At 1.50 with `NECK_BACK` at 0.35 it reads 200.1 mm and 1.02, and the
        // vault's depth-to-width is untouched at 1.29. So the head #79 tuned is
        // preserved rather than re-tuned; what changed is the constant that
        // buys it.
        //
        // THE HEAD, THE NECK AND THE SHOULDERS ARE ONE MEASUREMENT CHAIN. Any
        // change to the girdle or the neck re-tunes the face silently, and the
        // only thing that catches it is running `headaudit` afterwards.
        const HEAD_BELOW_JOINT: f32 = 1.599;
        let head_y =
            neck_y + head_r * HEAD_BELOW_JOINT * (1.0 + FACE_LENGTH_SPAN * params.face_length);

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
        // Saturated at the floor its own derivation states (#160): past
        // `hip_x = hip_r / 1.64` the two hip sockets interpenetrate, so an
        // exploration-range `hip_width` below about −1.1 narrows the stance no
        // further — the slider is unlimited, the geometry stops at what can be
        // built. 0.532 is the 0.516 floor plus the same ~3% margin the axis's
        // old end carried; inside the old ±1 range the clamp never binds.
        //
        // **0.532 → 0.49, measured rather than derived** (#163). The closed form
        // above puts the wall at 0.516 and the sweep puts it at 0.468 — the 400
        // rerolled records mesh down to there and fail below it — so the shipped
        // figure was carrying 13.7% of margin while claiming 3%. The formula
        // over-predicts, which #106 already found once and recorded as "a rule
        // of thumb, not a formula"; this is the same finding with a number on
        // it. 0.49 is the measured wall plus 4.7%.
        //
        // **Inside ±1 this floor does not bind at all**, and #163 calls it the
        // thing blocking #100 from narrowing the hips, so the distinction is
        // worth having: the coefficient bottoms out at 0.54 at `hip_width` −1,
        // which is already above the old floor, so nothing inside the
        // conservative range ever met it. What the floor really bounds is how
        // far the 0.64 above may come down before a narrower body stops
        // building — and that headroom just went from nothing to 8%.
        //
        // **Divided back out of `pelvis_r`, and the division is the anatomy.**
        // The pelvis above flares 5.7% on a feminine frame while the hip JOINTS
        // move 1.3% — measured, both — so a hip position that simply rode the
        // pelvis radius would separate the femoral heads four times as much as
        // either reference does. What widens is the iliac crest, not the
        // distance between the trochanters, and the two are different bones
        // doing different jobs. So this takes its own pair of anchors and
        // cancels the pelvis's.
        let hip_x = pelvis_r
            * (0.64 + 0.10 * params.hip_width).max(0.49)
            * frame(femininity, 0.0973, 0.0986)
            / frame(femininity, 0.0910, 0.1019);
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
        // Where the knee sits is dimorphic too, and only just: the male
        // reference splits its leg 0.2223 to 0.2368 of stature and the female
        // 0.2366 to 0.2398, which is a thigh fraction of 0.4842 against 0.4967.
        // Small, kept because it is measured and free, and because a knee that
        // did not move at all while the pelvis rose would drift the shank.
        let thigh_fraction = THIGH_FRACTION * frame(femininity, 0.4842, 0.4967);
        let knee_y = ankle_y + (hip_y - ankle_y) * (1.0 - thigh_fraction);
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
        // with a swept slab hung off it. The slab is gone: these place the heel,
        // the ball and the toe as real nodes in the leg chain.
        //
        // Provenance for the three figures below: **looked up**, from the
        // Quaternius male and female (which share one skeleton). Foot length is
        // 16.4% of stature on the male and 15.7% on the female; the ball is
        // 53.7% of the foot's length forward of the ankle and the toe tip 82.3%.
        // `FOOT_LONG` is 0.156 rather than the 0.160 those two average to
        // because it is the length *requested* of the mesher and the built foot
        // comes out a little longer than the nodes that ask for it — measured
        // 0.1658 of stature at 0.156, against the reference pair's 0.157-0.164.
        const FOOT_LONG: f32 = 0.156;
        const FOOT_BALL_ALONG: f32 = 0.537;
        const FOOT_TOE_ALONG: f32 = 0.823;
        let foot_long = h * FOOT_LONG;
        let ball_z = foot_long * FOOT_BALL_ALONG;
        let toe_z = foot_long * FOOT_TOE_ALONG;

        // **The foot is a level run, and the heel is the one part of it that is
        // not** (#111).
        //
        // A foot doubles back on itself — heel behind the ankle, toe well in
        // front, leg meeting both from above — and how that is said in capsules
        // decides whether the sole is flat, which is the property everything
        // that plants a foot depends on.
        //
        // **A sole is flat only where the bone under it is level.** A cage ring
        // stands perpendicular to its own bone, and where a chain bends it
        // stands on the bisector, so a turn tilts the section and lifts the
        // surface beneath it. Three arrangements were built and measured before
        // this one, and the measurements are the argument:
        //
        // ```text
        //                                 behind ankle   sole contact
        //                                  / length      behind ankle
        //   no heel at all                   -0.106           0.0%
        //   ankle -> heel -> ball -> toe      0.151           0.0%
        //   heel hulled, cap straight back    0.313          25.2%
        //   heel hulled, cap back and down    0.164          14.5%
        //   reference (male / female)      0.156 / 0.189  11.2% / 13.7%
        // ```
        //
        // The second row is the whole lesson. Running the leg's own chain into
        // a heel puts the foot's outline exactly where the reference has it and
        // the foot bears **none** of its weight there: the turn at the heel is
        // about 100 degrees, its ring tilts 39 degrees, and the sole comes out a
        // rocker. Outline is not a heel, which is why `examples/footaudit`
        // measures contact area rather than the rearmost point — a thick ankle
        // has a rearmost point behind the ankle too.
        //
        // So `heel -> ball -> toe` is level at `foot_y`, its rings are upright,
        // and the roll that stands a section on a flat edge does what it was
        // added for. The heel is made a *joint* rather than a bend by hanging a
        // short stub off the back of it: a joint is hulled from its sockets and
        // has no bisector ring, so nothing tilts.
        //
        // **The stub goes back and DOWN, and that is not cosmetic.** A socket
        // becomes an opening in the hull only where its own plane clears every
        // sibling corner, so a stub pointing straight back has to clear the
        // ankle's ring — which reaches backward — and needs 0.038 of bone where
        // the whole heel is 0.014 deep. It then has to be pushed out to 0.31 of
        // foot length, twice the reference, which is the third row above.
        // Angled down, the ankle's ring and the ball's both project *negatively*
        // onto it and it needs almost nothing, so the heel can sit where a heel
        // sits.
        //
        // **Nothing about the rig moves**, which is what decided this over
        // rooting the leg's chain at a heel — the other way to avoid a hull.
        // [`Rig::limb_chain`] still answers hip, knee, ankle, because it only
        // reaches into the extremity zone when a limb has fewer than three
        // joints above it; so the IK solve, `Footing` and every segment length
        // measured against the reference go on naming the same three joints. The
        // ankle still owns the first bone of the foot, so rotating it carries
        // heel, ball and toe together, which is what an ankle does. Rooting the
        // foot at a heel would have moved the ankle landmark onto the heel and
        // silently re-pointed all of that at a different bone — the mistake #110
        // spent a session finding.
        //
        // Provenance: **placed by measurement** (#111), against the reference
        // figures in the table above, and checked on twelve seeds.
        //
        // [`Rig::limb_chain`]: crate::rig::Rig::limb_chain
        const FOOT_HEEL_BACK: f32 = 0.07;
        const FOOT_CAP_BACK: f32 = 0.05;
        const FOOT_CAP_DROP: f32 = 0.05;
        let heel_z = -foot_long * FOOT_HEEL_BACK;
        let cap_z = heel_z - foot_long * FOOT_CAP_BACK;

        // How high the level run sits, and so where the body's sole ends up.
        //
        // **Not the reference's ball joint height.** That is 0.83% of stature
        // and it is a joint buried in flesh; this is the centre of a capsule
        // whose own section reaches most of the way to the ground. Read across
        // as though the two were the same landmark, it put the built sole 55 mm
        // *below* the floor this plan stands its bodies on — so the body was
        // 55 mm taller than `h`, the ankle measured 9.6% of stature above the
        // sole against a reference 5.4%, and every figure taken as a fraction of
        // rendered height was quietly 3% out.
        //
        // Provenance: **measured** (#111) — set so the built sole lands on the
        // plan's own ground plane, which it now does to 0.2 mm.
        const FOOT_SOLE_UP: f32 = 0.0163;
        let foot_y = h * FOOT_SOLE_UP;
        let cap_y = foot_y - foot_long * FOOT_CAP_DROP;

        // The radius that makes the foot the right WIDTH, and width is what a
        // sole is measured by: 37-38% of foot length on both references, so half
        // of it is the 0.185 below.
        //
        // **[`FOOT_KEPT`] is measured, not derived, and the derivation it
        // replaced was wrong in an instructive way.** It used to read
        // `0.64 * 0.707` — the fraction of its own half-extent a four-point ring
        // is said to deliver, times the fraction a section rolled half a segment
        // reaches along each axis — which is two assumptions multiplied
        // together with nothing measuring either. Built and measured, the foot
        // delivered 0.33, not 0.45, and the foot came out 30% narrow.
        //
        // The reason is worth keeping: **a fat ring between thin neighbours does
        // not survive subdivision.** With the foot only ball and toe, the ball
        // was a bulge with the thin ankle on one side and the tapering toe on
        // the other, and Catmull-Clark averaged it away. The heel and its cap
        // give it neighbours its own size and the delivered fraction rose to
        // 0.61-0.69 on its own, with no radius changed. Adding nodes bought back
        // what inflating a radius could not.
        const FOOT_KEPT: f32 = 0.61;
        let foot_r = foot_long * 0.185 / FOOT_KEPT;
        // How thick the foot is against how wide, from the same outlines: the ball
        // is about 20% of foot length deep against 37% wide.
        const FOOT_FLAT: f32 = 0.55;
        const FOOT_BALL_WIDE: f32 = 1.0;
        // The heel is nearly as wide as the ball and never narrower than the toe:
        // measured across the reference sole, 89.7 mm at the back against a widest
        // 111.7 mm (male) and 85.1 against 103.3 (female), so 0.82 of the ball on
        // both. A foot that tapers to a point at the back reads as a hoof.
        const FOOT_HEEL_WIDE: f32 = 0.82;
        // The stub that closes the back of the heel. Full-bodied rather than a
        // spike, because the back of the foot is where the sole has to reach the
        // ground: at 0.45 of the ball it rode 20 mm high there and bore 3% of
        // the contact, and at 0.75 it lies down and bears 14%.
        const FOOT_CAP_WIDE: f32 = 0.75;
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
        // **1.50 down to 1.42 on the eight-point cage, and this is the payment
        // #107 was taken out for** (#106). The floor above is not a body
        // proportion, it is cage geometry: the 0.82 in it is how far a socket
        // ring's flat reaches against its corners, which is `cos` of half a ring
        // segment — 0.707 at four points and 0.924 at eight. So the floor moved
        // when the ring did, and 1.50 was pinned to the old one.
        //
        // Re-derived by sweep rather than by re-running the algebra, because the
        // binding term is whichever of two clearances is worse and that is not
        // stable across the parameter space. Against `tests/plan.rs`'s 1500
        // random bodies and its corners:
        //
        // ```text
        //   1.50   meshes    shoulder span 0.2175 of stature
        //   1.46   meshes                  0.2122
        //   1.42   meshes                  0.2070   <- here
        //   1.38   FAILS                   0.2017
        //   1.35   FAILS                   0.1978
        // ```
        //
        // Reference is 0.1899, so this takes the shoulders from 14.5% over it to
        // 9%. 1.38 is where meshing goes, and 1.42 sits about 3% above that —
        // the same margin the `shoulder_width = −1` corner was given above.
        //
        // **The other half of the fix is not a coefficient and has already
        // happened.** #106's complaint was that the socket clears the chest's
        // CAGE half-width while the surface renders at 0.656 of it, so a
        // shoulder sat 45% outside a ribcage nobody could see. The cage now
        // delivers 0.79–1.00, so the same clearance puts it about 15% outside —
        // and the ribcage it clears filled out to the reference on its own. The
        // coat-hanger ratio, shoulder span over the trunk's own half-width at
        // the girdle, went 3.47 to 2.78 against a reference 2.38.
        //
        // Provenance: **derived, corrected against a failure, then bounded by a
        // sweep** (#98, re-swept for the eight-point cage in #107). The first
        // derivation is left above because it is the mistake this socket
        // invites, and because `hip_x` gets away with the same reasoning only by
        // accident — a hip socket's siblings blend toward the *waist*, which is
        // no wider than the pelvis.
        // Saturated for the exploration range (#160): the envelope sweep put
        // the single-axis meshing wall at `shoulder_width` −2.08 (coefficient
        // 1.253 — the girdle's third socket, toward the neck, runs out of
        // bone), and rolled bodies hit it earlier when build and limb length
        // are co-extreme. 1.30 is that wall plus ~4% for the interactions the
        // seed sweep found; inside the old ±1 range the clamp never binds.
        let clavicle_x = girdle_r * (1.51 + 0.08 * params.shoulder_width).max(1.41);
        // How high the shoulder mass arrives — and **raising it does not
        // shorten the visible neck, which was measured and is worth not
        // repeating** (#129).
        //
        // The argument for trying it was that the shoulder line an eye reads is
        // a line between two crowns, the girdle's on the midline and the
        // clavicle's out at `clavicle_x`, and that only this end of it is free
        // of the neck's arithmetic. Both halves of that are true. What is false
        // is that the crossing responds to it: the guard finds the shoulder line
        // where the body is half again as wide as the narrowest point of the
        // neck, which on the default body is 74 mm from the midline, and that
        // close in the surface is the girdle's, not the clavicle's.
        //
        // Measured, 0.004 to 0.014, which is +17.5 mm on the default body:
        //
        // ```text
        //   seed 0 visible neck   94.5 mm -> 92.5    ratio 0.385 -> 0.377
        //   the flare's ramp      50.9, 59.1, 76.1, 94.9, 114.8 at 5 mm steps
        //                    ->   50.5, 71.9, 172.8, 180.6, 188.8
        // ```
        //
        // So 17.5 mm of raise bought 2.0 mm of neck, and what it did instead was
        // bring the ARM up: the second row is a cliff where the first is a ramp,
        // and rendered bare it reads as a shoulder pad with a notch beside the
        // neck rather than as a shorter neck. The shape got worse for the length
        // it bought.
        //
        // **And it is bounded well below where that would matter anyway.**
        // 0.014 meshes, 0.016 loses two of the guarded seeds and 0.018 loses the
        // default body, on the girdle's own NECK socket — raising the clavicle
        // swings its socket up toward the neck's and the neck's has to slide out
        // past it, which its short bone cannot do. That is the same constraint
        // that floors `neck_y`, met from the other side, and it means a raised
        // clavicle and a shortened neck cannot be spent together.
        //
        // Not taken off the clavicle node's RADIUS either. It raises the same
        // crown, but its sphere at `clavicle_x` does not reach in to 74 mm from
        // the midline at any radius the shoulder could carry, and it would spend
        // this issue's evidence on #66 and #98's number.
        //
        // Provenance: **unsourced**, and left there deliberately — #129 swept it
        // and the sweep argues for the value it already had.
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
        // **The upper arm is where the two references disagree most in the whole
        // body**, 0.1621 against 0.1293 of stature — a fifth — while their
        // forearms agree to 1%. The note above predicted that this one
        // coefficient would carry most of the axis's travel, and it does.
        let upper_arm =
            h * (0.1404 + 0.025 * params.limb_length) * frame(femininity, 0.1621, 0.1293);
        let forearm = h * (0.1484 + 0.025 * params.limb_length) * frame(femininity, 0.1529, 0.1549);
        // Provenance: **unsourced**, and note it feeds arm span through
        // `hand_at` below — so the extremity axis moves a figure that `#66`
        // tuned, and nothing connects the two.
        let hand_len = h * 0.040 * (1.0 + 0.3 * params.extremity_size);
        let shoulder_at = Vec3::new(shoulder_x, clavicle_y, 0.0);
        let elbow_at = shoulder_at + arm * upper_arm;
        let wrist_at = elbow_at + arm * forearm;
        let hand_at = wrist_at + arm * hand_len;

        // Saturated for the exploration range (#160): the foot is a chain of
        // small nodes on a stature-fixed length, and the envelope sweep put
        // its meshing wall at `extremity_size` +1.41 (multiplier 1.42 — the
        // heel's sockets overlap) with rolled bodies failing from about +1.5
        // when the limbs are short; the small end fails only in combination,
        // where a heavy build meets near-zero foot radii. The clamp binds
        // outside the old ±1 range only (0.7..1.3 untouched); past it, hands
        // and feet stop growing rather than stop building.
        //
        // **Audited and both ends kept** (#163). Re-walked against the same
        // 400-roll envelope gate at the wider girdle, which reaches nothing
        // down here and did not move either end: the top fails at 1.42, which
        // is the figure recorded above measured a second way, so 1.36 carries
        // 4.2% of margin and is a real wall. The bottom fails at 0.41, so 0.55
        // carries 34% — far more than a wall needs, and like `girth`'s low end
        // it is holding a judgement about how small a foot may get rather than
        // a meshability limit. Both stay: nothing wants the extra room, and the
        // pair is documented here so the next axis that fans in through this
        // clamp knows which end will actually stop it.
        let extremity = (1.0 + 0.3 * params.extremity_size).clamp(0.55, 1.36);

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
        // The limb ladder is dimorphic in both directions at once, which is why
        // it is four pairs and not one gain: the arm thins on a feminine frame
        // and the calf thickens. Each radius takes the reference station
        // nearest it — proximal for a proximal node — from the limb-thickness
        // table `examples/reference` prints.
        //
        // `arm_upper` and `clavicle_r` are derived up with the trunk instead of
        // here, because the neck's floor clears the clavicle's socket and so has
        // to know how fat that node is before it can place the neck.
        let arm_lower = frame(femininity, 0.0263, 0.0238);
        let shoulder_r = h * 0.041 * girth.at(0.61, 0.62, 0.35) * arm_upper;
        let elbow_r = h * 0.026 * girth.at(0.45, 0.45, 0.20) * arm_lower;
        let wrist_r = h * 0.018 * girth.at(0.12, 0.12, 0.00) * frame(femininity, 0.0181, 0.0167);
        let hip_r = h * 0.054 * girth.at(0.50, 0.54, 0.55);
        // The knee sits between the thigh's distal station and the shank's
        // proximal one, which disagree — the thigh barely moves, the calf
        // thickens 6.7% — so it takes the calf's, which is the mass the eye
        // reads there. The ankle takes the shank's distal station, where the
        // two references agree to within half a percent and this is very nearly
        // a no-op kept for completeness.
        let knee_r = h * 0.028 * girth.at(0.38, 0.33, 0.25) * frame(femininity, 0.0250, 0.0286);
        let ankle_r = h * 0.019 * girth.at(0.12, 0.12, 0.00) * frame(femininity, 0.0183, 0.0184);
        // Slim: this is the base of the hand, not a stand-in for one. While the
        // limbs ended in these nodes they were fattened to read as a fist and a
        // boot, and now that real hands and feet hang off them a blob only pokes
        // through the part it is meant to be inside.
        let hand_r = h * 0.020 * extremity;
        let heel_r = foot_r * FOOT_HEEL_WIDE * extremity;
        let cap_r = foot_r * FOOT_CAP_WIDE * extremity;
        let ball_r = foot_r * FOOT_BALL_WIDE * extremity;
        let toe_r = foot_r * FOOT_TOE_WIDE * extremity;

        // Both skull nodes carry the same section, so the breadth axis narrows
        // the vault and the face by one factor rather than tapering one into the
        // other. See [`HEAD_BREADTH_SPAN`].
        //
        // [`SKULL_SLENDER`] scales BOTH axes, so it changes how broad the head is
        // for its own height without touching how deep it is for its breadth.
        let skull_section = Vec2::new(
            SKULL_SLENDER * (1.0 + HEAD_BREADTH_SPAN * params.head_breadth),
            SKULL_SLENDER,
        );

        Self {
            pelvis_y,
            pelvis_r,
            pelvis_section: PELVIS_SECTION,
            waist_y,
            waist_r,
            waist_section,
            chest_y,
            chest_r,
            chest_section: CHEST_SECTION,
            girdle_y,
            girdle_r,
            girdle_section: GIRDLE_SECTION,
            neck_at: Vec3::new(0.0, neck_y, -NECK_BACK * neck_r),
            neck_r,
            neck_section: NECK_SECTION,
            neck_offset: Vec2::new(0.0, -NECK_LOBE * neck_r),
            head_y,
            head_r,
            skull_section,
            crown_y: head_y + head_r * CROWN_HIGH,
            crown_r: head_r * CROWN_WIDE,
            jaw_pivot_at: Vec3::new(0.0, head_y - head_r * JAW_PIVOT.x, head_r * JAW_PIVOT.y),
            jaw_pivot_r: head_r * JAW_REACH.x,
            // The tip's height is a share of the head's reach BELOW ITS JOINT,
            // not of its radius: that span is what `face_length` stretches, and
            // the chin rides it. See [`JAW_TIP`] for the three measurements.
            jaw_tip_at: Vec3::new(
                0.0,
                head_y - (head_y - neck_y) * JAW_TIP.x,
                head_r * JAW_TIP.y,
            ),
            jaw_tip_r: head_r * JAW_REACH.y,
            clavicle_x,
            clavicle_y,
            clavicle_r,
            shoulder_at,
            shoulder_r,
            elbow_at,
            elbow_r,
            wrist_at,
            wrist_r,
            hand_at,
            hand_r,
            hip_x,
            hip_y,
            hip_r,
            knee_y,
            knee_z,
            knee_r,
            ankle_y,
            ankle_r,
            foot_y,
            // Squashed to [`FOOT_FLAT`] of its width. The half-segment roll that
            // stands it on a flat edge instead of on a vertex is the
            // assembler's, because it is about the ground rather than about the
            // body.
            sole_section: Vec2::new(1.0, FOOT_FLAT),
            heel_z,
            heel_r,
            cap_y,
            cap_z,
            cap_r,
            ball_z,
            ball_r,
            toe_z,
            toe_r,
        }
    }
}
