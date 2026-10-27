//! Every dimension the four-legged cage is built from.
//!
//! Twelve semantic axes and three composites in, one flat [`Dimensions`] out,
//! and [`crate::plan::quadruped`] does nothing afterwards but hang nodes on
//! it. See [`super`] for why this layer exists at all and what a refactor of
//! it has to prove.
//!
//! **This plan's girth is the composites', spent at sites of its own** (#178):
//! `mass` says how much body there is, `bodyFat` how it is spent, age what the
//! legs lose, and legginess slims all of it. See [`Girth`].

use glam::{Quat, Vec2, Vec3};

use crate::plan::{Composites, DEFAULT_BODY_FAT, QuadrupedParams};

/// Cross-sections of the trunk, as `(lateral, vertical)` multiples of the node's
/// radius.
///
/// The same idea as the humanoid's, and the axes come out the other way round
/// for a reason worth stating. The ring frame is transported from a world-up
/// reference, so `x` is lateral on any body; but a quadruped's spine runs
/// fore-and-aft rather than vertically, which leaves `y` pointing *up* instead
/// of forward. A galloping animal's ribcage is narrow and deep — the opposite
/// proportion to a person's, in the same two numbers.
///
/// As on the humanoid, every axis that moves moves *downward*: a socket's
/// clearance demand is its largest half-extent, so shrinking is free and growing
/// costs a joint its room. Both girdles here carry four sockets each.
const HIPS_SECTION: Vec2 = Vec2::new(0.86, 1.0);
/// See [`HIPS_SECTION`]. The loin, between the two girdles.
const SPINE_SECTION: Vec2 = Vec2::new(0.80, 1.0);
/// See [`HIPS_SECTION`].
const WITHERS_SECTION: Vec2 = Vec2::new(0.84, 1.0);
/// See [`HIPS_SECTION`]. A neck is rounder than the body it leaves.
const NECK_SECTION: Vec2 = Vec2::new(0.92, 1.0);

/// Every number the four-legged cage is built from.
///
/// **Leg positions are half-bodies**, given for the `+X` side only, and the
/// assembler mirrors them — which is what keeps the animal symmetric by
/// construction rather than by two expressions agreeing.
///
/// **Where this is bounded rather than anatomical:** `girth` saturates at
/// `0.50..1.80`, and each spine segment is floored at 1.6 times the largest
/// girdle it joins (1.2 at the top of `appeal`), so a short heavy animal stretches instead of folding its
/// two girdles into each other. **The floor is a look, not a mesher wall**
/// (#366): the cage closes with it at 0.9, because the mesher pushes its
/// sockets out as far as it needs. What 1.6 keeps is a loin between the two
/// girdles; the 2.5 it replaces held every body long, so `bodyLength` could
/// not shorten a heavy one and a horse could not be compact. The neck's floor
/// is the same kind of number, for the same reason (`neck_reach`).
///
/// **The graph is an animal's, not a table's** (#366). The neck rises from
/// the withers and carries a cranium, a muzzle and a nose; the loin rides
/// higher and narrower than the chest and the croup, which is the tuck-up;
/// each leg is laid out the way the humanoid's is — two nodes in the upper
/// limb, the ankle in the lower, the foot in the extremity — so the leg IK
/// solves the same three joints on both plans. On the hind leg that ankle is
/// the hock, held high off the ground, and the stifle points forward of the
/// thigh while the hock points back of it: the zig-zag every four-legged
/// silhouette is read by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dimensions {
    /// Where the rear girdle (the croup) sits.
    pub hips_at: Vec3,
    /// Radius of the rear girdle.
    pub hips_r: f32,
    /// Cross-section of the rear girdle.
    pub hips_section: Vec2,
    /// Where the loin sits, between the girdles.
    pub spine_at: Vec3,
    /// Radius of the loin.
    pub spine_r: f32,
    /// Cross-section of the loin.
    pub spine_section: Vec2,
    /// Where the front girdle (the chest, under the withers) sits.
    pub withers_at: Vec3,
    /// Radius of the front girdle.
    pub withers_r: f32,
    /// Cross-section of the front girdle.
    pub withers_section: Vec2,
    /// Where the neck sits, risen from the withers.
    pub neck_at: Vec3,
    /// Radius of the neck node.
    pub neck_r: f32,
    /// Cross-section of the neck node.
    pub neck_section: Vec2,
    /// Where the back of the skull sits, between the neck and the cranium.
    pub occiput_at: Vec3,
    /// Radius of the back of the skull.
    pub occiput_r: f32,
    /// Where the cranium sits.
    pub head_at: Vec3,
    /// Radius of the cranium.
    pub head_r: f32,
    /// Where the muzzle sits, ahead of and below the cranium.
    pub muzzle_at: Vec3,
    /// Radius of the muzzle.
    pub muzzle_r: f32,
    /// Where the nose ends the muzzle.
    pub nose_at: Vec3,
    /// Radius of the nose.
    pub nose_r: f32,
    /// Each tail node from the croup outward, with its radius.
    pub tail: [(Vec3, f32); TAIL_NODES],
    /// The hind leg's nodes from the body down, on the `+X` side: where it
    /// leaves the body, the stifle, the hock, the pad and the toes.
    pub hind: [(Vec3, f32); LEG_NODES],
    /// The fore leg's nodes from the body down, on the `+X` side: where it
    /// leaves the body, the elbow, the carpus, the pad and the toes.
    pub fore: [(Vec3, f32); LEG_NODES],
}

/// How many nodes a tail carries: enough to bend, and to swing as a chain once
/// the springs reach it (#370).
pub(crate) const TAIL_NODES: usize = 5;

/// How many nodes a leg carries: two in the upper limb, one in the lower, two
/// in the extremity — the humanoid's split, so `Rig::limb_chain` takes the same
/// three joints on both plans.
pub(crate) const LEG_NODES: usize = 5;

/// A tail's bends at `tailCarriage` 0, in radians above level from each node
/// to the next, the first unused: out and a little up from the croup, then
/// falling away in a sabre curve.
const SABRE: [f32; TAIL_NODES] = [0.15, 0.05, -0.25, -0.55, -0.80];
/// See [`SABRE`]: at `tailCarriage` -1, out of the croup and straight down.
const HANGING: [f32; TAIL_NODES] = [0.0, -0.55, -1.10, -1.35, -1.45];
/// See [`SABRE`]: at `tailCarriage` 1, raised and curled over the back.
const RAISED: [f32; TAIL_NODES] = [0.0, 0.85, 1.30, 1.85, 2.40];

/// How much more body there is than neutral at each kind of site (#178).
///
/// **The humanoid's model with this plan's own exponents.** `mass` spans a
/// ratio of volume per height ([`MASS_SPAN`], the humanoid's); `bodyFat`
/// splits it into a lean budget and a fat one ([`SOFT`]); and age takes a
/// share of the legs' lean budget ([`WASTING_SPAN`]) - the humanoid's
/// appendicular wasting, which is a mammal's before it is a person's. What is
/// NOT the humanoid's is where the budget lands: a human's exponents are
/// measured human allometry, and a quadruped's are a species question before
/// they are a body one. So each [`Site`] here names its own, by the pattern
/// the humanoid's measurements show - the fleshy sites gain fastest, the bony
/// ones slowest, the head not at all - and they are **chosen, not measured**.
///
/// **And legginess slims all of it**, the correlation this plan's girth has
/// always carried: at a given back height a leggy animal is a slender one and
/// a squat animal is a barrel, so the legs' length reduces girth rather than
/// changing the height. Letting the two move independently produces bodies
/// that read as mistakes.
///
/// Every factor is one on the neutral body - mass 0, `DEFAULT_BODY_FAT`, under
/// `AGE_PIVOT`, legs neutral - so the composites at their defaults build the
/// plan's own body unchanged.
#[derive(Clone, Copy, Debug)]
struct Girth {
    /// Lean volume per unit height, relative to neutral.
    lean: f32,
    /// Fat volume per unit height, relative to neutral.
    fat: f32,
    /// The legs' lean budget, after age has taken its share.
    wasting: f32,
    /// What legginess leaves of every girth.
    slim: f32,
}

/// Volume per unit height at `mass` +1, relative to neutral: the humanoid's
/// span, so one `mass` means one amount of body on both plans.
const MASS_SPAN: f32 = 1.49;

/// The tissue at a site that is not going anywhere, as a body-fat fraction:
/// the humanoid's, and for its reason - without it a lean body's belly is not
/// lean but absent.
const SOFT: f32 = 0.08;

/// What age takes out of the legs' lean volume by the top of `AGE_RANGE`: the
/// humanoid's sarcopenia figure, which is a mammal's.
const WASTING_SPAN: f32 = 0.22;

/// One site the girth budget is spent at: its exponent on volume per height,
/// and how much of its growth is fat rather than lean.
type Site = (f32, f32);

/// The belly, under the loin: the depot, where a heavy animal is round.
const BELLY: Site = (0.62, 0.60);
/// The croup: muscle with fat over it.
const CROUP: Site = (0.50, 0.35);
/// The chest: a ribcage, which grows slowest of the trunk.
const CHEST: Site = (0.45, 0.20);
/// The neck, where a fat cat's disappears.
const NECK: Site = (0.55, 0.30);
/// The thigh and the arm: the legs' muscle.
const THIGH: Site = (0.50, 0.30);
/// The second thigh and the forearm.
const SHANK: Site = (0.40, 0.15);
/// The hock and the carpus: bone and tendon.
const CANNON: Site = (0.20, 0.0);
/// The pad and the toes.
const PAD: Site = (0.15, 0.0);
/// The tail.
const TAIL: Site = (0.30, 0.15);

impl Girth {
    /// Reads the composites and legginess into the budgets.
    fn of(params: &QuadrupedParams, composites: &Composites) -> Self {
        let bulk = MASS_SPAN.powf(composites.mass);
        let fat = composites.body_fat.clamp(0.0, 0.95);
        Self {
            lean: bulk,
            fat: bulk * (fat + SOFT) / (DEFAULT_BODY_FAT + SOFT),
            wasting: bulk * (1.0 - WASTING_SPAN * composites.ageing()),
            slim: 1.0 - 0.18 * params.leg_length,
        }
    }

    /// The factor at a site of the trunk, the neck or the tail.
    fn at(&self, site: Site) -> f32 {
        self.spent(self.lean, site)
    }

    /// The factor at a site of a leg: the budget age wastes.
    fn limb(&self, site: Site) -> f32 {
        self.spent(self.wasting, site)
    }

    /// One site's factor, given which lean budget it spends.
    ///
    /// Saturated for the exploration range (#160), as the girth this replaces
    /// was: radii scale with it while the plan's lengths do not, and the old
    /// pair sweep failed exactly where a slight body compounded long legs.
    fn spent(&self, lean: f32, (power, fat_share): Site) -> f32 {
        (lean.powf(power * (1.0 - fat_share)) * self.fat.powf(power * fat_share) * self.slim)
            .clamp(0.50, 1.80)
    }
}

impl Dimensions {
    /// Derives every dimension of one quadruped from its record's axes and
    /// its composites.
    ///
    /// **The frame axis is a humanoid one** and is not read: it is anchored on
    /// a measured male and female mannequin, and there is no such pair for a
    /// beast - a quadruped's dimorphism is a species question before it is a
    /// body question (#178). `mass`, `bodyFat` and age are read, through
    /// [`Girth`], and age also settles the back and lowers the head.
    pub(crate) fn of(params: &QuadrupedParams, composites: &Composites) -> Self {
        let h = params.height;
        let girth = Girth::of(params, composites);
        let ageing = composites.ageing();
        // `appeal` (#366): the baby schema's proportions over the anatomy - a
        // bigger head on a shorter neck, a shorter muzzle and body, rounder,
        // and thicker legs on bigger pads - read by every family in its own
        // terms, so a cute horse is still a horse. Saturated where the head
        // would swallow the neck and the legs their own joints.
        let appeal = params.appeal.clamp(-1.5, 2.0);
        let round = 1.0 + 0.06 * appeal;
        let sturdy = 1.0 + 0.25 * appeal;

        let girdle_r = h * 0.181 * girth.at(CROUP) * round;
        // A deep chest is a larger front girdle about the same centre: the
        // withers stand over the back line and the brisket drops toward the
        // elbow, which is a horse's front and a greyhound's (#366).
        let withers_r = h * 0.190 * girth.at(CHEST) * round * (1.0 + 0.12 * params.chest_depth);
        // The loin is narrower and rides higher than the girdles either side
        // of it: the tuck-up, which is most of what tells a trunk from a tube.
        let spine_r = h * 0.150 * girth.at(BELLY) * round;
        // The neck that leaves the top of the chest thickens with its depth:
        // a deep chest carries a heavy forehand, a draught horse's rather than
        // a whippet's.
        let neck_r = h * 0.078 * girth.at(NECK) * (1.0 + 0.2 * params.chest_depth).clamp(0.6, 1.6);

        // Spine segments are floored at a multiple of the girdles they join, so
        // a short-bodied heavy animal stretches rather than folding its girdles
        // into each other.
        let stretch = (1.0 + 0.25 * params.body_length) * (1.0 - 0.12 * appeal);
        // A cute body carries a shorter loin, down to 1.2 radii at the top of
        // the axis: on a body the floor already holds, a rounder girdle would
        // otherwise make it LONGER, against everything the axis means.
        let segment_floor = (1.6 - 0.2 * appeal.max(0.0)) * girdle_r.max(withers_r);

        let withers_z = h * 0.345 * stretch;
        let spine_z = withers_z - (h * 0.483 * stretch).max(segment_floor);
        let hips_z = spine_z - (h * 0.448 * stretch).max(segment_floor);

        // The neck rises from the top of the chest rather than its front, so
        // the brisket stands forward under the throat, and the head is
        // carried above the back line rather than level with it. How steeply
        // is `neckCarriage`: a cat's carried low and forward, a horse's high
        // (#366). Saturated well short of level and of upright. Floored at
        // 1.4 chest radii, a look rather than a wall like the spine's: at the
        // 1.9 it replaces, the floor and not `neckLength` set every cat's
        // neck, and the head sat a hound's height over its shoulders.
        let neck_reach = (h * 0.32 * (1.0 + 0.3 * params.neck_length) * (1.0 - 0.15 * appeal))
            .max(withers_r * 1.4);
        let level_rise = 0.70f32.atan2(0.62);
        // Age carries it lower, as an old dog carries its head.
        let rise = (level_rise + 0.26 * params.neck_carriage - 0.12 * ageing).clamp(0.30, 1.35);
        let neck_at = Vec3::new(
            0.0,
            h * 1.06 + neck_reach * 0.935 * rise.sin(),
            withers_z + neck_reach * 0.935 * rise.cos(),
        );
        let head_r = h * 0.150 * (1.0 + 0.25 * params.head_size) * (1.0 + 0.30 * appeal);
        // The skull is two rings, the back of it and the brow, because a lone
        // large ring between two small ones is what subdivision smooths away:
        // two of a size make a plateau it keeps, and the head reads as a mass
        // rather than as the neck's swollen end. The back of it carries on up
        // the neck's own line.
        let occiput_r = head_r * 0.88;
        let occiput_at = neck_at
            + Quat::from_rotation_x(level_rise - rise)
                * Vec3::new(0.0, 0.70, 0.71)
                * (neck_r + occiput_r)
                * 1.05;
        // And the rest of the head nods about it as the neck is carried
        // higher: a horse's long head hangs from an arched neck, a cat's is
        // held level. Positive is nose-down.
        let nod = Quat::from_rotation_x((0.75 * params.neck_carriage).clamp(-0.21, 0.9));
        let head_at = occiput_at + nod * Vec3::new(0.0, 0.10, 1.0).normalize() * head_r * 0.85;
        // The muzzle leaves the cranium forward and a little down; the nose
        // closes it. Both scale with the head they hang off, and between them
        // they stand the nose `snout` cranium radii past the cranium's front:
        // a third of one on a cat, a dog's 1.3, two on a horse
        // (`muzzleLength`, #366). Floored so the shortest face keeps a nose.
        let snout = ((1.30 + 1.12 * params.muzzle_length) * (1.0 - 0.30 * appeal)).max(0.15);
        let ahead = head_r * (0.60 + snout) / 1.90;
        let muzzle_r = head_r * 0.58;
        let muzzle_at = head_at + nod * Vec3::new(0.0, -head_r * 0.30, ahead * 1.15);
        let nose_r = head_r * 0.40;
        let nose_at = muzzle_at + nod * Vec3::new(0.0, -head_r * 0.10, ahead * 0.75);

        // The tail is set high on the croup and carried as `tailCarriage`
        // says: hanging, out and down in a sabre curve, or raised and curled
        // over the back (#366), tapering to its tip. Saturated at the ends,
        // because a curl past `RAISED` folds the tail into the back.
        let tail_step = h * 0.13 * (1.0 + 0.6 * params.tail_length);
        let tail_r = h * 0.050 * girth.at(TAIL);
        let set = Vec3::new(0.0, h * 1.08, hips_z - girdle_r * 1.20);
        let carriage = params.tail_carriage.clamp(-1.0, 1.0);
        let toward = if carriage < 0.0 { HANGING } else { RAISED };
        let bends: [f32; TAIL_NODES] =
            std::array::from_fn(|i| SABRE[i] + (toward[i] - SABRE[i]) * carriage.abs());
        let lay = |step: f32| {
            let mut tail = [(set, tail_r); TAIL_NODES];
            let mut at = set;
            for (index, bend) in bends.into_iter().enumerate() {
                if index > 0 {
                    at +=
                        Vec3::new(0.0, bend.sin(), -bend.cos()) * step.max(tail[index - 1].1 * 2.2);
                }
                let taper = 1.0 - 0.14 * index as f32;
                tail[index] = (at, (tail_r * taper).max(h * 0.012));
            }
            tail
        };
        // A long tail hanging straight down would run into the ground (#366:
        // a tail three units long at -1, and one roll in three thousand), so
        // its step is shortened until every node clears it by its own radius
        // and a margin. Lengths only: the carriage keeps its shape.
        let clear = h * 0.04;
        let mut tail = lay(tail_step);
        let fit = tail
            .iter()
            .filter(|&&(at, _)| at.y < set.y)
            .map(|&(at, radius)| (set.y - radius - clear) / (set.y - at.y))
            .fold(1.0f32, f32::min);
        if fit < 1.0 {
            tail = lay(tail_step * fit.max(0.0));
        }

        // The pad's centre: its radius above the ground, less what
        // subdivision takes off a small node, so the surface meets the floor.
        let pad_r = h * 0.050 * girth.limb(PAD) * sturdy;
        let toe_r = h * 0.042 * girth.limb(PAD) * sturdy;
        let pad_y = pad_r * 0.82;
        let toe_y = toe_r * 0.82;

        // Each girdle spreads its legs far enough that the two sockets
        // separate, and drops far enough that the bone can carry them there -
        // but never so far that the leg left below it is shorter than four of
        // its pads. A barrel on short legs raises its legs' roots into its
        // girth instead, because a root dropped past that floor leaves the
        // joints below it no span to be placed in, and the leg cannot fold
        // (#366: a deep chest on a wildcard of muscle, seed 1).
        let leg_floor = pad_y + pad_r * 4.0;
        let rear_x = girdle_r * 1.55;
        let front_x = withers_r * 1.55;
        let rear_root_y = (h * 0.966 - girdle_r * 1.95).max(leg_floor);
        let front_root_y = (h - withers_r * 1.95).max(leg_floor);

        // **Every joint of a leg is placed as a share of that leg's own drop**
        // from where it leaves the body to the pad, never as a share of the
        // body's height. The two disagree as soon as legginess or girth moves
        // the root: placed by height, a short-legged roll put its elbow at the
        // height of its own carpus, the forearm lay flat, and the leg could not
        // fold far enough to plant (#366, found by `every_rerolled_body_can_walk`
        // on seed 1).
        let hind_drop = rear_root_y - pad_y;
        let fore_drop = front_root_y - pad_y;

        // `stance` is what the legs stand on (#366): up on the toes at 0, a
        // dog's and a cat's, and up on the tips of them at 1, a horse's - the
        // long bones of the foot lengthen, so the hock and the carpus ride
        // higher, the stifle tucks up toward the belly and the foot shortens
        // to a hoof's length. Below 0 the foot lies flatter. Every share is
        // saturated so the three joints the leg IK solves stay a bent leg.
        let stance = params.stance;
        let stifle_share = (0.30 - 0.06 * stance).clamp(0.18, 0.40);
        let hock_share = (0.42 + 0.10 * stance).clamp(0.22, 0.62);
        let carpus_share = (0.25 + 0.08 * stance).clamp(0.12, 0.40);
        let toe_ahead = pad_r * (1.4 - 0.5 * stance).clamp(0.7, 2.2);

        // Hind: the thigh leaves the croup, the stifle sits forward of it at
        // the flank, the hock well back of the stifle and - on a dog - four
        // tenths of the way up, and the long bones of the foot drop from the
        // hock to the pad.
        let hind = [
            (
                Vec3::new(rear_x, rear_root_y, hips_z + hind_drop * 0.05),
                h * 0.092 * girth.limb(THIGH) * sturdy,
            ),
            (
                Vec3::new(
                    rear_x,
                    rear_root_y - hind_drop * stifle_share,
                    hips_z + hind_drop * 0.20,
                ),
                h * 0.060 * girth.limb(SHANK) * sturdy,
            ),
            (
                Vec3::new(
                    rear_x,
                    pad_y + hind_drop * hock_share,
                    hips_z - hind_drop * 0.05,
                ),
                h * 0.042 * girth.limb(CANNON) * sturdy,
            ),
            (Vec3::new(rear_x, pad_y, hips_z + hind_drop * 0.04), pad_r),
            (
                Vec3::new(rear_x, toe_y, hips_z + hind_drop * 0.04 + toe_ahead),
                toe_r,
            ),
        ];
        // Fore: the arm leaves the chest at the point of the shoulder, the
        // elbow sits at the brisket well behind it - the bend a stride takes
        // its slack from, which a straight foreleg would not have - the forearm
        // drops to the carpus, and a short pastern carries the pad.
        let fore = [
            (
                Vec3::new(front_x, front_root_y, withers_z + fore_drop * 0.09),
                h * 0.080 * girth.limb(THIGH) * sturdy,
            ),
            (
                Vec3::new(
                    front_x,
                    front_root_y - fore_drop * 0.28,
                    withers_z - fore_drop * 0.13,
                ),
                h * 0.055 * girth.limb(SHANK) * sturdy,
            ),
            (
                Vec3::new(
                    front_x,
                    pad_y + fore_drop * carpus_share,
                    withers_z - fore_drop * 0.06,
                ),
                h * 0.040 * girth.limb(CANNON) * sturdy,
            ),
            (
                Vec3::new(front_x, pad_y, withers_z - fore_drop * 0.02),
                pad_r,
            ),
            (
                Vec3::new(front_x, toe_y, withers_z - fore_drop * 0.02 + toe_ahead),
                toe_r,
            ),
        ];

        Self {
            hips_at: Vec3::new(0.0, h * 0.966, hips_z),
            hips_r: girdle_r,
            hips_section: HIPS_SECTION,
            // The loin rides above the girdles, and settles toward them with
            // age: the dipped back of an old horse.
            spine_at: Vec3::new(0.0, h * (1.03 - 0.05 * ageing), spine_z),
            spine_r,
            spine_section: SPINE_SECTION,
            withers_at: Vec3::new(0.0, h, withers_z),
            withers_r,
            withers_section: WITHERS_SECTION,
            neck_at,
            neck_r,
            neck_section: NECK_SECTION,
            occiput_at,
            occiput_r,
            head_at,
            head_r,
            muzzle_at,
            muzzle_r,
            nose_at,
            nose_r,
            tail,
            hind,
            fore,
        }
    }
}
