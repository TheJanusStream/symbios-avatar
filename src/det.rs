//! Bit-reproducible transcendentals for the motion path.
//!
//! The rig is tested by measuring it, and overlands #1183 measured the same
//! deterministic drive reading 25.4 / 35.2 / 50.1 mm on three builds of the
//! same source — the spread is the platform's libm. Rust documents
//! `f32::sin`/`cos`/`powf`/`exp` as platform-dependent, so any threshold
//! recorded through them on one machine carries a silent environment margin on
//! every other. This module routes the whole of [`crate::anim`] through the
//! pure-Rust [`libm`] crate instead, which computes the same bits on every
//! target, so a locomotion measurement asserted exactly here can be asserted
//! exactly on CI — and CI's run becomes the cross-platform check.
//!
//! What routes and what stays, established by reading glam 0.32.1 rather than
//! assuming (symbios-avatar #323/#324):
//!
//! * The nine scalar transcendentals the anim path uses ([`DetMath`]).
//! * `Quat::from_rotation_x/y/z` and `from_axis_angle` — their only
//!   transcendental is `sin_cos` ([`Rot`]).
//! * `Quat::slerp` ([`slerp`]) — glam implements it once per SIMD backend and
//!   the backends disagree: the sse2 path computes its three sines with an
//!   inline SIMD polynomial the `libm` feature never touches, so a native peer
//!   and a wasm peer slerp differently from the same record (#324).
//! * `Quat::to_axis_angle` ([`to_axis_angle`]) — hides a `math::atan2`.
//! * `Quat::from_rotation_arc` ([`from_rotation_arc`]) — deterministic except
//!   its 180° singularity branch, which calls `from_axis_angle`.
//! * `sqrt` STAYS on hardware: IEEE-754 requires it correctly rounded, so
//!   `length`/`normalize` already agree everywhere. symbios-ground reached the
//!   same conclusion for the terrain path.
//! * `Quat::angle_between` and the `acos` inside slerp STAY: glam's
//!   `acos_approx` is a 7-degree minimax polynomial used identically on all
//!   its paths. It is copied here (it is private) — copied, not reimplemented,
//!   so the two stay bit-equal.
//!
//! The quaternion helpers do their own horizontal reductions in a fixed
//! left-to-right association instead of calling glam's `dot`/`length`, because
//! a SIMD backend is free to associate a reduction differently and rounding
//! follows association. Element-wise mul/add/sub are IEEE-exact per lane and
//! stay on glam.

use glam::{Quat, Vec3};

/// The scalar transcendentals of the anim path, routed through [`libm`].
///
/// `det_`-prefixed because an inherent method always beats a trait method in
/// resolution — a method literally named `sin` would silently keep calling
/// `f32::sin`. `to_radians`/`to_degrees`/`sqrt`/`powi`/`rem_euclid` need no
/// routing: they are exact.
pub(crate) trait DetMath {
    fn det_sin(self) -> Self;
    fn det_cos(self) -> Self;
    // tan's anim call sites are all instrument-side today, but the trait stays
    // complete: a lib-side site added later must find the routed spelling
    // waiting, not fall back to `f32::tan` because the method was pruned.
    #[allow(dead_code)]
    fn det_tan(self) -> Self;
    fn det_asin(self) -> Self;
    fn det_acos(self) -> Self;
    fn det_atan(self) -> Self;
    fn det_atan2(self, other: Self) -> Self;
    fn det_exp(self) -> Self;
    fn det_powf(self, n: Self) -> Self;
}

impl DetMath for f32 {
    #[inline]
    fn det_sin(self) -> f32 {
        libm::sinf(self)
    }
    #[inline]
    fn det_cos(self) -> f32 {
        libm::cosf(self)
    }
    #[inline]
    fn det_tan(self) -> f32 {
        libm::tanf(self)
    }
    #[inline]
    fn det_asin(self) -> f32 {
        libm::asinf(self)
    }
    #[inline]
    fn det_acos(self) -> f32 {
        libm::acosf(self)
    }
    #[inline]
    fn det_atan(self) -> f32 {
        libm::atanf(self)
    }
    #[inline]
    fn det_atan2(self, other: f32) -> f32 {
        libm::atan2f(self, other)
    }
    #[inline]
    fn det_exp(self) -> f32 {
        libm::expf(self)
    }
    #[inline]
    fn det_powf(self, n: f32) -> f32 {
        libm::powf(self, n)
    }
}

/// `sin` and `cos` of the same angle in one call, as glam's constructors use.
#[inline]
pub(crate) fn sin_cos(a: f32) -> (f32, f32) {
    libm::sincosf(a)
}

/// The four `Quat` constructors the anim path uses, with their `sin_cos`
/// routed through [`libm`]. Mirrors glam 0.32.1's scalar path exactly —
/// `sin_cos(angle * 0.5)` then `from_xyzw` — so the only change is which
/// sine is linked.
pub(crate) struct Rot;

impl Rot {
    #[inline]
    pub(crate) fn x(angle: f32) -> Quat {
        let (s, c) = sin_cos(angle * 0.5);
        Quat::from_xyzw(s, 0.0, 0.0, c)
    }

    #[inline]
    pub(crate) fn y(angle: f32) -> Quat {
        let (s, c) = sin_cos(angle * 0.5);
        Quat::from_xyzw(0.0, s, 0.0, c)
    }

    #[inline]
    pub(crate) fn z(angle: f32) -> Quat {
        let (s, c) = sin_cos(angle * 0.5);
        Quat::from_xyzw(0.0, 0.0, s, c)
    }

    /// `axis` must be normalized, as with `Quat::from_axis_angle`.
    #[inline]
    pub(crate) fn axis_angle(axis: Vec3, angle: f32) -> Quat {
        let (s, c) = sin_cos(angle * 0.5);
        let v = axis * s;
        Quat::from_xyzw(v.x, v.y, v.z, c)
    }
}

/// Glam 0.32.1's `acos_approx_f32`, copied verbatim because it is private.
///
/// Based on DirectXMath's `XMScalarAcos`: a 7-degree minimax approximation of
/// `self.clamp(-1.0, 1.0).acos()`. It is pure add/mul/sqrt, so it is already
/// bit-reproducible on every target — it lives here only so [`slerp`] can call
/// it, and it must stay bit-equal to glam's copy.
#[inline]
fn acos_approx(v: f32) -> f32 {
    let nonnegative = v >= 0.0;
    let x = v.abs();
    let mut omx = 1.0 - x;
    if omx < 0.0 {
        omx = 0.0;
    }
    let root = omx.sqrt();

    #[allow(clippy::approx_constant)]
    let mut result =
        ((((((-0.001_262_491_1 * x + 0.006_670_09) * x - 0.017_088_126) * x + 0.030_891_88) * x
            - 0.050_174_303)
            * x
            + 0.088_978_99)
            * x
            - 0.214_598_8)
            * x
            + 1.570_796_3;
    result *= root;

    if nonnegative {
        result
    } else {
        core::f32::consts::PI - result
    }
}

/// Component-wise dot in fixed left-to-right association.
#[inline]
fn dot4(a: Quat, b: Quat) -> f32 {
    ((a.x * b.x + a.y * b.y) + a.z * b.z) + a.w * b.w
}

#[inline]
fn scale(q: Quat, s: f32) -> Quat {
    Quat::from_xyzw(q.x * s, q.y * s, q.z * s, q.w * s)
}

#[inline]
fn add(a: Quat, b: Quat) -> Quat {
    Quat::from_xyzw(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w)
}

/// Normalize as glam does — multiply by the reciprocal length — but with the
/// length reduction in fixed association.
#[inline]
fn normalize(q: Quat) -> Quat {
    scale(q, 1.0 / dot4(q, q).sqrt())
}

/// `Quat::slerp`, computed the same way on every target.
///
/// Glam implements slerp once per SIMD backend: on x86-64 the three sines are
/// an inline SIMD polynomial (`m128_sin`), on the scalar and wasm paths they
/// are `math::sin` — so the backends disagree with each other and no feature
/// flag reconciles them (#324). This is glam 0.32.1's scalar algorithm with
/// the sines from [`libm`]: same sign flip, same `1 - ε` nlerp threshold, same
/// `acos_approx`.
pub(crate) fn slerp(a: Quat, mut b: Quat, s: f32) -> Quat {
    let mut dot = dot4(a, b);
    if dot < 0.0 {
        b = -b;
        dot = -dot;
    }

    const DOT_THRESHOLD: f32 = 1.0 - f32::EPSILON;
    if dot > DOT_THRESHOLD {
        // Near-parallel: linear interpolation, to avoid dividing by sin(θ) ≈ 0.
        normalize(add(scale(a, 1.0 - s), scale(b, s)))
    } else {
        let theta = acos_approx(dot);
        let scale1 = libm::sinf(theta * (1.0 - s));
        let scale2 = libm::sinf(theta * s);
        let theta_sin = libm::sinf(theta);
        scale(add(scale(a, scale1), scale(b, scale2)), 1.0 / theta_sin)
    }
}

/// `Quat::to_axis_angle`, which hides a `math::atan2` in glam — the recovered
/// angle is `2·atan2(‖xyz‖, w)`, and atan2 implementations disagree on 15% of
/// arguments. Same `1e-8` degenerate-axis fallback as glam's.
pub(crate) fn to_axis_angle(q: Quat) -> (Vec3, f32) {
    const EPSILON: f32 = 1.0e-8;
    let length = ((q.x * q.x + q.y * q.y) + q.z * q.z).sqrt();
    if length >= EPSILON {
        let angle = 2.0 * libm::atan2f(length, q.w);
        let axis = Vec3::new(q.x / length, q.y / length, q.z / length);
        (axis, angle)
    } else {
        (Vec3::X, 0.0)
    }
}

/// `Quat::from_rotation_arc`. The main branch is cross + normalize — already
/// exact — but the 180° singularity branch calls `from_axis_angle`, so it is
/// mirrored here with [`Rot::axis_angle`] and the reductions fixed.
///
/// `from` and `to` must be normalized, as with glam's.
pub(crate) fn from_rotation_arc(from: Vec3, to: Vec3) -> Quat {
    const ONE_MINUS_EPS: f32 = 1.0 - 2.0 * f32::EPSILON;
    let dot = (from.x * to.x + from.y * to.y) + from.z * to.z;
    if dot > ONE_MINUS_EPS {
        Quat::IDENTITY
    } else if dot < -ONE_MINUS_EPS {
        Rot::axis_angle(from.any_orthonormal_vector(), core::f32::consts::PI)
    } else {
        let c = from.cross(to);
        normalize(Quat::from_xyzw(c.x, c.y, c.z, 1.0 + dot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints golden constants for `scalar_goldens_are_bit_exact`, choosing
    /// for each routed function the first argument in a gait-plausible range
    /// where this box's platform libm and the `libm` crate disagree — a golden
    /// at a round value hashes the same under either implementation and proves
    /// nothing (symbios-ground's `DIVERGENT_ROUGHNESS = 0.768` is that oddly
    /// specific for the same reason). Run it with
    /// `cargo test --release divergence_probe -- --ignored --nocapture`
    /// on a glibc box if the routing is ever deliberately changed.
    #[test]
    #[ignore = "generator for the goldens below, run by hand"]
    fn divergence_probe() {
        fn first_divergent(
            name: &str,
            lo: f32,
            hi: f32,
            std_f: impl Fn(f32) -> f32,
            det_f: impl Fn(f32) -> f32,
        ) {
            let steps = 2_000_000;
            for i in 0..=steps {
                let a = lo + (hi - lo) * (i as f32 / steps as f32);
                let (s, d) = (std_f(a), det_f(a));
                if s.to_bits() != d.to_bits() {
                    println!(
                        "{name}: arg {a:?} (0x{:08x}) libm 0x{:08x} platform 0x{:08x}",
                        a.to_bits(),
                        d.to_bits(),
                        s.to_bits()
                    );
                    return;
                }
            }
            println!("{name}: NO divergence in [{lo}, {hi}] — widen the range");
        }

        first_divergent("sin", 0.0, 7.0, f32::sin, DetMath::det_sin);
        first_divergent("cos", 0.0, 7.0, f32::cos, DetMath::det_cos);
        first_divergent("tan", 0.0, 1.5, f32::tan, DetMath::det_tan);
        first_divergent("asin", -1.0, 1.0, f32::asin, DetMath::det_asin);
        first_divergent("acos", -1.0, 1.0, f32::acos, DetMath::det_acos);
        first_divergent("atan", -4.0, 4.0, f32::atan, DetMath::det_atan);
        first_divergent(
            "atan2(a, 0.75)",
            -2.0,
            2.0,
            |a| a.atan2(0.75),
            |a| a.det_atan2(0.75),
        );
        first_divergent("exp", -4.0, 4.0, f32::exp, DetMath::det_exp);
        first_divergent(
            "powf(a, 1.7)",
            0.0,
            4.0,
            |a| a.powf(1.7),
            |a| a.det_powf(1.7),
        );
        first_divergent("sin_cos.0", 0.0, 7.0, |a| a.sin_cos().0, |a| sin_cos(a).0);

        let a = Rot::axis_angle(Vec3::new(0.6, 0.48, 0.64), 0.9337);
        let b = slerp(Rot::x(1.234_567), Rot::z(-2.345_678), 0.371);
        let (axis, angle) = to_axis_angle(b);
        let arc = from_rotation_arc(Vec3::new(0.6, 0.48, 0.64), Vec3::Y);
        for (name, got) in [
            ("axis_angle.x", a.x),
            ("slerp.x", b.x),
            ("slerp.w", b.w),
            ("to_axis_angle.axis.x", axis.x),
            ("to_axis_angle.angle", angle),
            ("arc.x", arc.x),
            ("arc.w", arc.w),
        ] {
            println!("composite {name}: {got:?} 0x{:08x}", got.to_bits());
        }
    }

    /// Exact bit patterns, at arguments the probe above found divergent on the
    /// recording box (Gentoo glibc, 2026-09-01) — each differs from that
    /// platform's answer by the last bit, so a routed function slipping back
    /// to the platform libm fails here on any glibc box. The same assertion
    /// passing on CI — a different build environment, the one that read
    /// 50.1 mm where this box read 25.4 (#323) — is the cross-platform proof.
    /// Arguments are `from_bits` so no decimal round-trip can move them.
    #[test]
    fn scalar_goldens_are_bit_exact() {
        let f = f32::from_bits;
        for (name, got, want) in [
            ("sin", f(0x3c49_9f1b).det_sin(), 0x3c49_9dcd),
            ("cos", f(0x3f49_228e).det_cos(), 0x3f34_f7ba),
            ("tan", f(0x3b84_a948).det_tan(), 0x3b84_a977),
            ("asin", f(0xbf7f_b939).det_asin(), 0xbfc3_1cd1),
            ("acos", f(0xbf80_0000).det_acos(), 0x4049_0fda),
            ("atan", f(0xc07f_ffef).det_atan(), 0xbfa9_b462),
            ("atan2", f(0xbfff_ffde).det_atan2(0.75), 0xbf9b_23a2),
            ("exp", f(0xc07f_ffde).det_exp(), 0x3c96_0afe),
            ("powf", f(0x37fb_a882).det_powf(1.7), 0x32af_d4c4),
            ("sin_cos.0", sin_cos(f(0x3c49_9f1b)).0, 0x3c49_9dcd),
        ] {
            assert_eq!(
                got.to_bits(),
                want,
                "{name}: got {got:?} (0x{:08x}), want 0x{want:08x}",
                got.to_bits()
            );
        }
    }

    /// The composite helpers, pinned bit-exactly at non-trivial inputs. These
    /// do not need a divergent argument — the perturbation control in #323's
    /// record covers liveness — they pin that every target computes the same
    /// quaternion from the same inputs, which is what #324 is about.
    #[test]
    fn composite_goldens_are_bit_exact() {
        let a = Rot::axis_angle(Vec3::new(0.6, 0.48, 0.64), 0.9337);
        let b = slerp(Rot::x(1.234_567), Rot::z(-2.345_678), 0.371);
        let (axis, angle) = to_axis_angle(b);
        let arc = from_rotation_arc(Vec3::new(0.6, 0.48, 0.64), Vec3::Y);
        for (name, got, want) in [
            ("axis_angle.x", a.x, 0x3e8a_4364u32),
            ("slerp.x", b.x, 0x3edc_f620),
            ("slerp.w", b.w, 0x3f4a_6537),
            ("to_axis_angle.axis.x", axis.x, 0x3f34_6db3),
            ("to_axis_angle.angle", angle, 0x3fa8_b403),
            ("arc.x", arc.x, 0xbebe_75cb),
            ("arc.w", arc.w, 0x3f5c_3834),
        ] {
            assert_eq!(
                got.to_bits(),
                want,
                "{name}: got {got:?} (0x{:08x}), want 0x{want:08x}",
                got.to_bits()
            );
        }
    }

    /// FNV-1a over the bits, so one wrong ULP anywhere in a sequence of
    /// floats changes the digest.
    fn fold(hash: &mut u64, value: f32) {
        *hash ^= u64::from(value.to_bits());
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    /// Two seconds of the whole walking path, bit-exact.
    ///
    /// The scalar goldens prove each routed function computes libm's bits; this
    /// proves the *assembly* of them — 120 driven frames with the footing on —
    /// lands on the same pose everywhere, which is the property overlands'
    /// instruments need (#1183: 25.4 / 35.2 / 50.1 mm from three builds of one
    /// source). It asserts in two stages so a failure names its culprit:
    ///
    /// * `RIG_GOLDEN` covers only the skeleton the walk runs on. If CI fails
    ///   HERE, the divergence is upstream of locomotion — the plan/skeleton
    ///   derivation, which #323 deliberately did not route — and the walk was
    ///   never tested.
    /// * `WALK_GOLDEN` covers every joint of every frame. Failing here with
    ///   the rig stage green is a locomotion call site that slipped off the
    ///   routing.
    ///
    /// A deliberate gait retune moves `WALK_GOLDEN` — rerun with
    /// `--nocapture` and take the printed values, on any box: after #323 they
    /// are the same everywhere, which is the point.
    #[test]
    fn locomotion_is_bit_reproducible() {
        use crate::anim::{Gait, Ground, Pose, Stride, Walk};
        use crate::plan::{BodyPlan, HumanoidParams};
        use crate::rig::Rig;

        let rig =
            Rig::from_skeleton(&HumanoidParams::default().skeleton(&crate::Composites::default()))
                .expect("the default humanoid rigs");
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for joint in &rig.joints {
            fold(&mut hash, joint.position.x);
            fold(&mut hash, joint.position.y);
            fold(&mut hash, joint.position.z);
        }
        println!("rig digest:  0x{hash:016x}");
        assert_eq!(
            hash, RIG_GOLDEN,
            "the rig itself differs, upstream of locomotion — see this test's doc"
        );

        let gait = Gait::natural(&rig);
        let stride = Stride::for_body(&rig, 1.0);
        let mut pose = Pose::rest(&rig);
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for frame in 0..120u32 {
            let cycle = frame as f32 / 60.0;
            Walk::at(cycle).drive(&rig, &mut pose, &gait, &stride, |foot| {
                Some(Ground::level(Vec3::new(foot.x, 0.0, foot.z)))
            });
            for rotation in &pose.rotations {
                fold(&mut hash, rotation.x);
                fold(&mut hash, rotation.y);
                fold(&mut hash, rotation.z);
                fold(&mut hash, rotation.w);
            }
            fold(&mut hash, pose.translation.x);
            fold(&mut hash, pose.translation.y);
            fold(&mut hash, pose.translation.z);
        }
        println!("walk digest: 0x{hash:016x}");
        assert_eq!(
            hash, WALK_GOLDEN,
            "a locomotion call site is off the routing — see this test's doc"
        );
    }

    /// Digests recorded 2026-09-01 on the box that recorded the scalar
    /// goldens; CI agreeing is the cross-platform proof.
    const RIG_GOLDEN: u64 = 0x51dd_0120_cfe8_425b;
    const WALK_GOLDEN: u64 = 0xe79a_5358_7569_a921;
}
