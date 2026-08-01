//! Eyes, and the lids that close over them.
//!
//! Eyes carry more of a face than their size suggests, and the research on
//! stylised characters is unanimous about why: it is not the shading, it is that
//! they *move*. A body that blinks and looks at things reads as inhabited; the
//! same body with painted-on eyes reads as a mannequin. So this is geometry with
//! a rotation on it, not a texture.
//!
//! Lids are **spherical shells that rotate**, upper and lower, meeting at the
//! eye's equator when shut. A face rig would deform eyelid geometry that is part
//! of the head, but a head here is a smooth blob with no eyelid to deform —
//! and a shell that rotates is both honest about that and convincing, because
//! that is very nearly what a real lid does.
//!
//! Everything is built in **head-local space**. A renderer parents the parts to
//! the head joint, which follows the body for free; nothing here needs skinning.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::mesh::PolyMesh;
use crate::plan::Zone;
use crate::prim;
use crate::rig::{Rig, landmark};

/// How a body's eyes are shaped and set.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EyeParams {
    /// Eye size, as a fraction of the head's radius.
    #[serde(default, with = "crate::plan::scaled")]
    pub size: f32,
    /// How far apart the eyes are set, `-1` close and `+1` wide.
    #[serde(default, with = "crate::plan::scaled")]
    pub spacing: f32,
    /// How deeply the eyes are set into the head, `-1` protruding and `+1` sunken.
    #[serde(default, with = "crate::plan::scaled")]
    pub depth: f32,
    /// How far open the lids rest, `0` shut and `1` wide.
    #[serde(default, with = "crate::plan::scaled")]
    pub aperture: f32,
}

impl Default for EyeParams {
    fn default() -> Self {
        Self {
            size: 0.5,
            spacing: 0.0,
            depth: 0.0,
            aperture: 0.8,
        }
    }
}

impl EyeParams {
    /// Clamps every axis into range. Idempotent.
    pub fn sanitize(&mut self) {
        use crate::plan::scaled::quantize;
        self.size = quantize(finite(self.size, 0.5).clamp(0.0, 1.0));
        self.aperture = quantize(finite(self.aperture, 0.8).clamp(0.0, 1.0));
        self.spacing = quantize(finite(self.spacing, 0.0).clamp(-1.0, 1.0));
        self.depth = quantize(finite(self.depth, 0.0).clamp(-1.0, 1.0));
    }
}

/// Substitutes `fallback` for a non-finite value.
fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// One eye's parts, in head-local space.
#[derive(Clone, Debug, PartialEq)]
pub struct Eye {
    /// The eyeball, centred on [`Eye::pivot`].
    pub globe: PolyMesh,
    /// The upper lid, in its fully open position.
    pub upper_lid: PolyMesh,
    /// The lower lid, in its fully open position.
    pub lower_lid: PolyMesh,
    /// Where the eye turns about, in head-local space.
    pub pivot: Vec3,
    /// Radius of the globe.
    pub radius: f32,
    /// `-1` for the body's left eye, `+1` for its right.
    pub side: f32,
}

impl Eye {
    /// How far the lids swing between open and shut.
    ///
    /// The upper lid does most of the work, as a real one does; the lower barely
    /// moves. Splitting it evenly is the giveaway that a blink was animated by
    /// someone who did not look at one.
    const UPPER_SWING: f32 = 1.45;
    /// How far the lower lid swings.
    const LOWER_SWING: f32 = 0.45;

    /// The rotation to apply to a lid, about the eye's pivot.
    ///
    /// `closure` runs `0` for fully open to `1` for shut.
    #[must_use]
    pub fn lid_rotation(&self, closure: f32, upper: bool) -> Quat {
        let closure = closure.clamp(0.0, 1.0);
        // Positive about X carries the top of the eye forward over its front,
        // which is the direction an upper lid actually travels; the lower lid
        // starts underneath and so has to come the other way.
        let swing = if upper {
            Self::UPPER_SWING
        } else {
            -Self::LOWER_SWING
        };
        Quat::from_rotation_x(swing * closure)
    }

    /// The transform placing a lid for a given closure.
    #[must_use]
    pub fn lid_transform(&self, closure: f32, upper: bool) -> Mat4 {
        Mat4::from_translation(self.pivot)
            * Mat4::from_quat(self.lid_rotation(closure, upper))
            * Mat4::from_translation(-self.pivot)
    }

    /// The rotation turning this eye toward a point in head-local space.
    ///
    /// Clamped, because an eye that can point anywhere looks deranged rather
    /// than attentive.
    #[must_use]
    pub fn gaze_rotation(&self, target: Vec3, limit: f32) -> Quat {
        let toward = (target - self.pivot).normalize_or_zero();
        if toward == Vec3::ZERO {
            return Quat::IDENTITY;
        }
        let (axis, angle) = Quat::from_rotation_arc(landmark::FORWARD, toward).to_axis_angle();
        Quat::from_axis_angle(axis, angle.min(limit.max(0.0)))
    }

    /// Every part of this eye as one mesh, posed at the given closure.
    ///
    /// Convenient for inspection and export; a renderer keeps the parts separate
    /// so the lids can move without rebuilding anything.
    #[must_use]
    pub fn assembled(&self, closure: f32) -> PolyMesh {
        let mut mesh = self.globe.clone();
        mesh.append(
            &self
                .upper_lid
                .transformed(self.lid_transform(closure, true)),
        );
        mesh.append(
            &self
                .lower_lid
                .transformed(self.lid_transform(closure, false)),
        );
        mesh
    }
}

/// A body's pair of eyes, and where they belong.
#[derive(Clone, Debug, PartialEq)]
pub struct Eyes {
    /// The body's left eye.
    pub left: Eye,
    /// The body's right eye.
    pub right: Eye,
    /// The joint the pair is parented to.
    pub head: usize,
}

impl Eyes {
    /// Builds a pair of eyes for a body.
    ///
    /// Returns `None` for a body with no head to put them in.
    #[must_use]
    pub fn build(rig: &Rig, params: &EyeParams) -> Option<Self> {
        let head = *rig.in_zone(Zone::Head).first()?;
        let skull = rig.joints[head].radius;

        // Set into the face: forward, a little above centre, and apart. All
        // proportional to the skull, so a large head gets large eyes without
        // anything being retuned.
        // Set *into* the face, not onto it. The skull's rendered surface sits
        // well inside its node radius — subdivision pulls a capped tube in — so
        // placing eyes against the nominal radius leaves them bulging like
        // goggles, which is exactly how it first looked.
        let radius = skull * (0.14 + 0.08 * params.size);
        let apart = skull * (0.34 + 0.10 * params.spacing);
        let forward = skull * (0.60 - 0.10 * params.depth) - radius * 0.35;
        let rise = skull * 0.05;

        Some(Self {
            left: eye(-1.0, Vec3::new(-apart, rise, forward), radius, params),
            right: eye(1.0, Vec3::new(apart, rise, forward), radius, params),
            head,
        })
    }

    /// Both eyes as one mesh, posed at the given closure.
    #[must_use]
    pub fn assembled(&self, closure: f32) -> PolyMesh {
        let mut mesh = self.left.assembled(closure);
        mesh.append(&self.right.assembled(closure));
        mesh
    }
}

/// Builds one eye at `pivot`.
fn eye(side: f32, pivot: Vec3, radius: f32, params: &EyeParams) -> Eye {
    let globe = prim::sphere(radius, 10, 14).transformed(Mat4::from_translation(pivot));

    // A lid is a shell just clear of the globe, so it never intersects it as it
    // swings. Its rest position is set by the aperture: a wide-open eye starts
    // with the lids further back.
    let shell = radius * 1.06;
    let thickness = radius * 0.10;
    let open = params.aperture.clamp(0.0, 1.0);

    let lid = |upper: bool| {
        let swing = if upper {
            Eye::UPPER_SWING
        } else {
            -Eye::LOWER_SWING
        };
        // Built around +Y then turned so the pair meet across the eye when shut.
        //
        // The sign matters and it was wrong: written as `swing * (1 - open)`
        // this CLOSES the lids as the aperture rises, and the default aperture
        // left a fifteen-degree slit — an eye that read as a letterbox with a
        // stripe of iris in it. Opening the upper lid is a negative rotation
        // about X and opening the lower one is positive, which is exactly what
        // multiplying by each lid's own swing gives.
        let rest = Quat::from_rotation_x(swing * (0.42 - 0.72 * open))
            * if upper {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_x(std::f32::consts::PI)
            };
        prim::cap_shell(shell, thickness, 1.25, 3, 14)
            .transformed(Mat4::from_translation(pivot) * Mat4::from_quat(rest))
    };

    Eye {
        globe,
        upper_lid: lid(true),
        lower_lid: lid(false),
        pivot,
        radius,
        side,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BodyPlan, HumanoidParams, QuadrupedParams};

    fn eyes(params: &EyeParams) -> Eyes {
        let rig = Rig::from_skeleton(&HumanoidParams::default().skeleton()).expect("rigs");
        Eyes::build(&rig, params).expect("a head to put eyes in")
    }

    #[test]
    fn a_body_gets_two_eyes_set_in_its_face() {
        let pair = eyes(&EyeParams::default());
        assert!(pair.left.pivot.x < 0.0, "the left eye is on the left");
        assert!(pair.right.pivot.x > 0.0);
        assert!(
            pair.left.pivot.z > 0.0,
            "eyes belong on the front of a head"
        );
        assert_eq!(pair.left.pivot.x, -pair.right.pivot.x, "and are symmetric");
    }

    #[test]
    fn every_part_of_an_eye_is_a_solid() {
        let pair = eyes(&EyeParams::default());
        for (name, mesh) in [
            ("globe", &pair.left.globe),
            ("upper lid", &pair.left.upper_lid),
            ("lower lid", &pair.left.lower_lid),
        ] {
            assert!(
                mesh.is_closed_manifold(),
                "{name} is not closed: {:?}",
                mesh.manifold_report()
            );
        }
    }

    #[test]
    fn eyes_scale_with_the_head_they_sit_in() {
        let of_height = |height: f32| {
            let rig = Rig::from_skeleton(
                &HumanoidParams {
                    height,
                    ..Default::default()
                }
                .skeleton(),
            )
            .expect("rigs");
            let pair = Eyes::build(&rig, &EyeParams::default()).expect("eyes");
            let skull = rig.joints[pair.head].radius;
            (pair.left.radius, skull)
        };

        let (small, small_skull) = of_height(1.3);
        let (large, large_skull) = of_height(2.1);
        assert!(large > small, "a bigger head has bigger eyes");
        let ratio = (large / large_skull) / (small / small_skull);
        assert!(
            (ratio - 1.0).abs() < 0.05,
            "and the same size relative to itself, got {ratio:.3}"
        );
    }

    #[test]
    fn the_sliders_move_the_eyes_the_way_they_say() {
        let wide = eyes(&EyeParams {
            spacing: 1.0,
            ..Default::default()
        });
        let close = eyes(&EyeParams {
            spacing: -1.0,
            ..Default::default()
        });
        assert!(wide.left.pivot.x < close.left.pivot.x, "wider is wider");

        let big = eyes(&EyeParams {
            size: 1.0,
            ..Default::default()
        });
        let small = eyes(&EyeParams {
            size: 0.0,
            ..Default::default()
        });
        assert!(big.left.radius > small.left.radius * 1.4);

        let sunken = eyes(&EyeParams {
            depth: 1.0,
            ..Default::default()
        });
        let bulging = eyes(&EyeParams {
            depth: -1.0,
            ..Default::default()
        });
        assert!(sunken.left.pivot.z < bulging.left.pivot.z);
    }

    #[test]
    fn a_blink_covers_the_eye_and_opening_uncovers_it() {
        // The property that makes a blink read: when shut, no part of the globe
        // is left showing at the front.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;

        let exposed = |closure: f32| {
            let upper = eye.upper_lid.transformed(eye.lid_transform(closure, true));
            let lower = eye.lower_lid.transformed(eye.lid_transform(closure, false));
            // How far down the front of the globe each lid reaches, measured as
            // the frontmost point each covers.
            let front_of = |mesh: &PolyMesh| {
                mesh.positions
                    .iter()
                    .map(|point| (*point - eye.pivot).z)
                    .fold(f32::MIN, f32::max)
            };
            let covered = front_of(&upper).max(front_of(&lower));
            eye.radius - covered
        };

        assert!(
            exposed(1.0) < exposed(0.0),
            "shutting the lids should cover more of the eye"
        );
        assert!(
            exposed(1.0) < eye.radius * 0.25,
            "a shut eye should be almost entirely covered"
        );
    }

    #[test]
    fn lids_swing_further_above_the_eye_than_below_it() {
        // Real blinks are mostly the upper lid. Splitting the motion evenly is
        // the giveaway that nobody looked at one.
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;
        let upper = eye.lid_rotation(1.0, true).to_axis_angle().1;
        let lower = eye.lid_rotation(1.0, false).to_axis_angle().1;
        assert!(upper > lower * 2.0, "upper {upper:.2} vs lower {lower:.2}");
    }

    #[test]
    fn a_resting_aperture_sets_how_open_the_eyes_start() {
        let narrow = eyes(&EyeParams {
            aperture: 0.1,
            ..Default::default()
        });
        let wide = eyes(&EyeParams {
            aperture: 1.0,
            ..Default::default()
        });

        let gap = |pair: &Eyes| {
            let (lo, _) = pair.left.upper_lid.bounds();
            lo.y
        };
        assert!(
            gap(&narrow) < gap(&wide),
            "narrowed lids should hang lower over the eye"
        );
    }

    #[test]
    fn an_eye_looks_where_it_is_told_but_only_so_far() {
        let pair = eyes(&EyeParams::default());
        let eye = &pair.left;

        let ahead = eye.gaze_rotation(eye.pivot + Vec3::Z, 0.6);
        assert!(
            ahead.is_near_identity(),
            "looking straight ahead is no turn"
        );

        let aside = eye.gaze_rotation(eye.pivot + Vec3::new(1.0, 0.0, 1.0), 0.6);
        assert!((aside.to_axis_angle().1 - 0.6).abs() < 1e-4, "clamped");

        let slight = eye.gaze_rotation(eye.pivot + Vec3::new(0.1, 0.0, 1.0), 0.6);
        assert!(
            slight.to_axis_angle().1 < 0.6,
            "a small look is not clamped"
        );
    }

    #[test]
    fn a_body_without_a_head_gets_no_eyes() {
        use crate::skeleton::{Node, Skeleton};
        let mut bare = Skeleton::new();
        let a = bare.add_node(Node::new(Vec3::ZERO, 0.2));
        bare.extend_from(a, Node::new(Vec3::Y, 0.2));
        let rig = Rig::from_skeleton(&bare).expect("rigs");
        assert_eq!(Eyes::build(&rig, &EyeParams::default()), None);
    }

    #[test]
    fn a_creature_gets_eyes_too() {
        let rig = Rig::from_skeleton(&QuadrupedParams::default().skeleton()).expect("rigs");
        let pair = Eyes::build(&rig, &EyeParams::default()).expect("eyes");
        assert!(pair.left.radius > 0.0);
        assert!(pair.assembled(0.0).is_closed_manifold());
    }

    #[test]
    fn sanitize_clamps_and_is_idempotent() {
        let mut params = EyeParams {
            size: 9.0,
            spacing: f32::NAN,
            depth: -7.0,
            aperture: f32::INFINITY,
        };
        params.sanitize();
        assert_eq!(params.size, 1.0);
        assert_eq!(params.spacing, 0.0);
        assert_eq!(params.depth, -1.0);
        assert_eq!(params.aperture, 0.8);

        let once = params;
        params.sanitize();
        assert_eq!(once, params);
    }
}
