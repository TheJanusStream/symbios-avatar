//! The one monotone-cubic reader the face's profiles are read through.
//!
//! [`super::skull`] gives its profiles from the crown down and [`super::relief`]
//! gives its ramps from each feature's own top down, so the two run in opposite
//! directions. That is a tempting reason to keep two readers — two readers
//! that disagree about which end they start from is exactly the kind of thing
//! that looks correct in both files. But a lerp is cheap to keep twice; **a
//! Fritsch–Carlson limiter is not**, and both callers need the same C1
//! treatment, so keeping them apart would mean two copies of the one piece of
//! arithmetic in this crate that is genuinely subtle. So there is one reader,
//! and the direction is read off the knots rather than assumed: ascending or
//! descending, it clamps before the first knot and past the last and
//! interpolates between.

/// The slope of the straight line between two neighbouring knots.
///
/// Signed by the knots' own order, which is what lets one reader serve both
/// directions: a descending profile gives a negative run and a negative rise for
/// the same shape an ascending one gives two positives for, and the quotient is
/// the same slope in the curve's own parameter either way.
fn secant(curve: &[(f32, f32)], segment: usize) -> f32 {
    let (before, under) = curve[segment];
    let (after, over) = curve[segment + 1];
    (over - under) / (after - before)
}

/// The tangent to take at a knot: the average of the slopes either side, held
/// inside the range that keeps the curve monotone.
///
/// **Limited here, at the knot, and not inside [`monotone`] per segment.** The
/// textbook Fritsch–Carlson presentation rescales a segment's two tangents
/// together, which means a tangent shared by two segments can be pulled back by
/// one of them and left alone by the other — and then the curve arrives at that
/// knot with two different slopes. That is a corner, reintroduced by the very
/// step meant to tame the curve. `skull::DEPTH`'s knot at 0.55 had exactly it:
/// the segment above finished at −0.537 and the one below started at −0.514,
/// leaving a 0.023 break that survived halving the sample step.
///
/// Limiting each tangent once, against BOTH its neighbours, gives one slope per
/// knot — so the result is C1 by construction — and holding it to three times
/// the shorter adjacent secant is the classical sufficient condition for the
/// cubic to stay monotone across both segments.
///
/// **Zero wherever the curve turns around.** At a knot where it stops falling
/// and starts rising, any non-zero tangent carries the curve past the knot's own
/// value before it comes back. `skull::BREADTH` turns at its chin knot — the
/// secants either side are +1.0 and −3.4 — and averaging them dipped the profile
/// 0.0007 below the 0.66 it is authored to reach.
fn tangent(curve: &[(f32, f32)], at: usize) -> f32 {
    if curve.len() < 2 {
        return 0.0;
    }
    match at {
        0 => secant(curve, 0),
        at if at + 1 == curve.len() => secant(curve, at - 1),
        at => {
            let (before, after) = (secant(curve, at - 1), secant(curve, at));
            if before * after <= 0.0 {
                return 0.0;
            }
            let average = 0.5 * (before + after);
            let room = 3.0 * before.abs().min(after.abs());
            average.signum() * average.abs().min(room)
        }
    }
}

/// Reads a curve given as knots in either direction.
///
/// **Monotone cubic Hermite (Fritsch–Carlson), and both halves of that name are
/// load-bearing.**
///
/// *Cubic*, because a piecewise-linear curve has a slope that jumps at every
/// knot. On the skull the union of six profiles' knot heights is 27 values over
/// a 212 mm span — a tangent discontinuity every 7.9 mm — and `skull::BREADTH`
/// and `skull::DEPTH` carry no azimuthal window, so each of theirs runs the
/// whole way round the head. That is the signature of a terraced lower face,
/// visible as full-width horizontal bands in the renderer's normal pass.
///
/// **Finer sampling makes a C0 break worse, not better**, so refinement cannot
/// hide it: a slope jump spread across a 24 mm quad is hidden by Gouraud
/// interpolation, and the same jump resolved at 3.6 mm is a ledge. Refining
/// onto the limit surface was measured: it moves the head 0.059 mm and changes
/// the banding not at all. The interpolant is the cause.
///
/// *Monotone*, because an ordinary interpolating spline overshoots, and there is
/// one segment on the skull where overshoot is a shipped defect rather than a
/// wobble: `skull::CHIN`'s tail into its junction, where a natural or
/// Catmull-Rom spline dips **below zero**, which stands the head's lowest band
/// behind the throat it has to meet — an open seam at the neck. Fritsch–
/// Carlson's limiter forbids that by construction: where a segment is monotone
/// the interpolant is monotone, so no curve can leave the interval its own
/// knots span.
pub(super) fn monotone(curve: &[(f32, f32)], at: f32) -> f32 {
    let Some(&(first, low)) = curve.first() else {
        return 0.0;
    };
    if curve.len() < 2 {
        return low;
    }
    let Some(&(last, high)) = curve.last() else {
        return low;
    };
    // Which way round the knots run, taken from the knots themselves rather
    // than from a flag a caller could get wrong. `up` is the ascending case,
    // which is how the relief's ramps are authored; the skull's profiles
    // descend.
    let up = last > first;
    if (up && at <= first) || (!up && at >= first) {
        return low;
    }
    if (up && at >= last) || (!up && at <= last) {
        return high;
    }

    let segment = (0..curve.len() - 1)
        .find(|&index| {
            let edge = curve[index + 1].0;
            if up { at <= edge } else { at >= edge }
        })
        .unwrap_or(curve.len() - 2);
    let (before, under) = curve[segment];
    let (after, over) = curve[segment + 1];

    let run = after - before;
    if run.abs() <= f32::EPSILON {
        return under;
    }
    let slope = (over - under) / run;
    let (mut start, mut end) = (tangent(curve, segment), tangent(curve, segment + 1));

    // A flat segment must stay flat: a cubic through two equal values bulges
    // between them unless both its tangents are zero. Everything else is
    // already held in range by [`tangent`], which limits each knot ONCE so that
    // both segments meeting there agree — see its documentation for why doing
    // it per segment leaves a corner behind.
    if slope.abs() <= f32::EPSILON {
        start = 0.0;
        end = 0.0;
    }

    let along = (at - before) / run;
    let (square, cube) = (along * along, along * along * along);
    (2.0 * cube - 3.0 * square + 1.0) * under
        + (cube - 2.0 * square + along) * run * start
        + (-2.0 * cube + 3.0 * square) * over
        + (cube - square) * run * end
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same curve authored both ways round reads the same shape.
    ///
    /// **The test the old arrangement could not have**, and the reason one
    /// reader is worth the move: with a reader per direction, nothing compared
    /// them, and the docstring that kept them apart was reasoning about a risk
    /// rather than measuring it.
    #[test]
    fn a_curve_reads_the_same_from_either_end() {
        let up = [
            (0.0, 0.0),
            (0.22, 0.34),
            (0.55, 0.62),
            (0.80, 1.00),
            (1.0, 0.0),
        ];
        let down: Vec<(f32, f32)> = up.iter().rev().copied().collect();
        for step in 0..=200 {
            let at = step as f32 / 200.0;
            let (a, b) = (monotone(&up, at), monotone(&down, at));
            assert!(
                (a - b).abs() < 1e-6,
                "at {at:.3} the ascending curve reads {a} and the descending one {b}"
            );
        }
    }

    /// Outside its own knots a curve holds its end values rather than running on.
    #[test]
    fn a_curve_holds_its_ends() {
        let up = [(0.2f32, 0.5), (0.8, 0.9)];
        let down = [(0.8f32, 0.9), (0.2, 0.5)];
        for curve in [&up, &down] {
            assert_eq!(monotone(curve, -1.0), 0.5, "before the first knot");
            assert_eq!(monotone(curve, 2.0), 0.9, "past the last knot");
        }
    }
}
