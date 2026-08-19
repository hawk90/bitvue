//! Bjøntegaard-Delta rate/quality curve comparison (BD-rate, BD-quality).
//!
//! Standard method (VCEG-M33): fit a cubic polynomial through each curve's points, restrict to
//! the overlapping domain between the two curves, integrate each polynomial over that overlap,
//! and normalize by the overlap width to get an average delta.
//!
//! - **BD-rate** (bitrate savings at equal quality): fits `log10(bitrate) = f(quality)`,
//!   overlaps in the quality domain, and converts the average log-rate delta to a percentage via
//!   `100 * (10^delta - 1)`. Negative means the test curve needs less bitrate for the same
//!   quality.
//! - **BD-quality** (quality gain at equal rate): fits `quality = f(log10(bitrate))`, overlaps
//!   in the log-rate domain, and reports the average quality delta directly (same units as the
//!   curve's `quality` field -- dB for PSNR, unitless for SSIM).
//!
//! An earlier, pre-Electron-migration implementation (`src-tauri/src/commands/quality.rs`,
//! deleted `e7194cc`) used trapezoidal integration of the raw (unfit) points and directly
//! exponentiated the *quality* integral's difference as if it were a *log-rate* delta -- a unit
//! mismatch (comparing quality-domain area to a rate-domain quantity) with no `#[test]` coverage
//! to have caught it. This is a from-scratch, spec-correct replacement, not a port.

use bitvue_engine::{BitvueError, Result};
use serde::{Deserialize, Serialize};

/// A single point on a rate-distortion curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RdPoint {
    pub bitrate_kbps: f64,
    /// Quality metric value (PSNR in dB, SSIM, VMAF, ...) -- higher must mean better quality.
    pub quality: f64,
}

/// A named rate-distortion curve (e.g. one encoder/config across several QP/CRF values).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdCurve {
    pub name: String,
    pub points: Vec<RdPoint>,
}

/// Bjøntegaard-Delta comparison result between two RD curves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BdRateResult {
    pub anchor_name: String,
    pub test_name: String,
    /// Percentage bitrate delta at equal quality. Negative = test is more efficient.
    pub bd_rate_percent: f64,
    /// Average quality delta (test - anchor) at matched bitrates.
    pub bd_quality: f64,
}

/// VCEG-M33 requires at least 4 RD points per curve to fit a cubic.
const MIN_POINTS: usize = 4;

/// Compares two RD curves and returns their Bjøntegaard-Delta rate and quality.
pub fn calculate_bd_rate(anchor: &RdCurve, test: &RdCurve) -> Result<BdRateResult> {
    let a = finite_points(&anchor.points, &anchor.name)?;
    let t = finite_points(&test.points, &test.name)?;

    Ok(BdRateResult {
        anchor_name: anchor.name.clone(),
        test_name: test.name.clone(),
        bd_rate_percent: bd_rate_percent(&a, &t)?,
        bd_quality: bd_quality_delta(&a, &t)?,
    })
}

fn finite_points(points: &[RdPoint], curve_name: &str) -> Result<Vec<RdPoint>> {
    let filtered: Vec<RdPoint> = points
        .iter()
        .copied()
        .filter(|p| p.bitrate_kbps.is_finite() && p.bitrate_kbps > 0.0 && p.quality.is_finite())
        .collect();
    if filtered.len() < MIN_POINTS {
        return Err(BitvueError::InvalidData(format!(
            "curve '{curve_name}' has only {} valid point(s) after filtering NaN/Inf/non-positive \
             bitrates (need at least {MIN_POINTS})",
            filtered.len()
        )));
    }
    Ok(filtered)
}

fn bd_rate_percent(anchor: &[RdPoint], test: &[RdPoint]) -> Result<f64> {
    let (a_xs, a_ys) = sorted_xy(anchor, |p| p.quality, |p| p.bitrate_kbps.log10());
    let (t_xs, t_ys) = sorted_xy(test, |p| p.quality, |p| p.bitrate_kbps.log10());

    let a_coeffs = fit_cubic(&a_xs, &a_ys)?;
    let t_coeffs = fit_cubic(&t_xs, &t_ys)?;

    let (lo, hi) = overlap(&a_xs, &t_xs)?;
    let avg_log_rate_delta =
        (integrate_cubic(t_coeffs, lo, hi) - integrate_cubic(a_coeffs, lo, hi)) / (hi - lo);
    Ok(100.0 * (10f64.powf(avg_log_rate_delta) - 1.0))
}

fn bd_quality_delta(anchor: &[RdPoint], test: &[RdPoint]) -> Result<f64> {
    let (a_xs, a_ys) = sorted_xy(anchor, |p| p.bitrate_kbps.log10(), |p| p.quality);
    let (t_xs, t_ys) = sorted_xy(test, |p| p.bitrate_kbps.log10(), |p| p.quality);

    let a_coeffs = fit_cubic(&a_xs, &a_ys)?;
    let t_coeffs = fit_cubic(&t_xs, &t_ys)?;

    let (lo, hi) = overlap(&a_xs, &t_xs)?;
    Ok((integrate_cubic(t_coeffs, lo, hi) - integrate_cubic(a_coeffs, lo, hi)) / (hi - lo))
}

/// Projects each point to (x, y) via the given selectors and sorts by `x`.
fn sorted_xy(
    points: &[RdPoint],
    x_of: impl Fn(&RdPoint) -> f64,
    y_of: impl Fn(&RdPoint) -> f64,
) -> (Vec<f64>, Vec<f64>) {
    let mut pairs: Vec<(f64, f64)> = points.iter().map(|p| (x_of(p), y_of(p))).collect();
    pairs.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .expect("NaN filtered out by finite_points")
    });
    pairs.into_iter().unzip()
}

/// The overlapping `[lo, hi]` domain between two sorted, non-empty x-coordinate lists.
fn overlap(a_xs: &[f64], t_xs: &[f64]) -> Result<(f64, f64)> {
    let lo = a_xs[0].max(t_xs[0]);
    let hi = a_xs[a_xs.len() - 1].min(t_xs[t_xs.len() - 1]);
    if lo >= hi {
        return Err(BitvueError::InvalidData(format!(
            "curves don't overlap: anchor domain [{:.4}, {:.4}], test domain [{:.4}, {:.4}]",
            a_xs[0],
            a_xs[a_xs.len() - 1],
            t_xs[0],
            t_xs[t_xs.len() - 1]
        )));
    }
    Ok((lo, hi))
}

/// Least-squares cubic fit `y = c0 + c1*x + c2*x^2 + c3*x^3` via the normal equations
/// (`(XᵀX)c = XᵀY`), solved with 4x4 Gaussian elimination + partial pivoting. Exact
/// interpolation when `xs.len() == 4` (matches VCEG-M33's minimum-points case).
fn fit_cubic(xs: &[f64], ys: &[f64]) -> Result<[f64; 4]> {
    let mut m = [[0.0f64; 4]; 4];
    let mut b = [0.0f64; 4];
    for (&x, &y) in xs.iter().zip(ys) {
        let mut xp = [1.0f64; 7]; // x^0 ..= x^6
        for p in 1..7 {
            xp[p] = xp[p - 1] * x;
        }
        for j in 0..4 {
            b[j] += xp[j] * y;
            for (k, row) in m[j].iter_mut().enumerate() {
                *row += xp[j + k];
            }
        }
    }
    solve_4x4(m, b)
}

#[allow(clippy::needless_range_loop)] // index arithmetic across two rows of `m` simultaneously
fn solve_4x4(mut m: [[f64; 4]; 4], mut b: [f64; 4]) -> Result<[f64; 4]> {
    for col in 0..4 {
        let pivot_row = (col..4)
            .max_by(|&r1, &r2| m[r1][col].abs().partial_cmp(&m[r2][col].abs()).unwrap())
            .unwrap();
        if m[pivot_row][col].abs() < 1e-12 {
            return Err(BitvueError::InvalidData(
                "RD curve points are degenerate for a cubic fit (need more spread in quality/bitrate values)"
                    .to_string(),
            ));
        }
        m.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let diag = m[col][col];
        for row in (col + 1)..4 {
            let factor = m[row][col] / diag;
            for k in col..4 {
                m[row][k] -= factor * m[col][k];
            }
            b[row] -= factor * b[col];
        }
    }

    let mut c = [0.0f64; 4];
    for row in (0..4).rev() {
        let mut sum = b[row];
        for k in (row + 1)..4 {
            sum -= m[row][k] * c[k];
        }
        c[row] = sum / m[row][row];
    }
    Ok(c)
}

/// Definite integral of `c0 + c1*x + c2*x^2 + c3*x^3` over `[a, b]`.
fn integrate_cubic(c: [f64; 4], a: f64, b: f64) -> f64 {
    c.iter()
        .enumerate()
        .map(|(p, &coeff)| {
            let exp = (p + 1) as i32;
            coeff * (b.powi(exp) - a.powi(exp)) / exp as f64
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(name: &str, points: &[(f64, f64)]) -> RdCurve {
        RdCurve {
            name: name.to_string(),
            points: points
                .iter()
                .map(|&(bitrate_kbps, quality)| RdPoint {
                    bitrate_kbps,
                    quality,
                })
                .collect(),
        }
    }

    #[test]
    fn identical_curves_have_zero_delta() {
        let c = curve(
            "x",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
            ],
        );
        let result = calculate_bd_rate(&c, &c).unwrap();
        assert!(
            result.bd_rate_percent.abs() < 1e-6,
            "{}",
            result.bd_rate_percent
        );
        assert!(result.bd_quality.abs() < 1e-6, "{}", result.bd_quality);
    }

    #[test]
    fn uniformly_scaled_bitrate_gives_exact_bd_rate() {
        // Every test point is anchor's bitrate * 0.8 at the same quality. Since scaling by a
        // constant factor is a constant shift in log10(bitrate), the cubic fit reproduces it
        // exactly (it's within the fit's own basis), so BD-rate must equal exactly
        // 100*(0.8-1) = -20% -- a known-answer synthetic case, not just a sanity check.
        let anchor = curve(
            "anchor",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
            ],
        );
        let test = curve(
            "test",
            &[(400.0, 30.0), (800.0, 33.0), (1600.0, 36.0), (3200.0, 38.0)],
        );
        let result = calculate_bd_rate(&anchor, &test).unwrap();
        assert!(
            (result.bd_rate_percent - (-20.0)).abs() < 1e-6,
            "expected exactly -20%, got {}",
            result.bd_rate_percent
        );
    }

    #[test]
    fn uniformly_higher_quality_gives_exact_bd_quality() {
        // Test is always +2.0 quality units better at the same log-rate -- a constant shift in
        // the fit's own basis, so BD-quality must equal exactly +2.0.
        let anchor = curve(
            "anchor",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
            ],
        );
        let test = curve(
            "test",
            &[
                (500.0, 32.0),
                (1000.0, 35.0),
                (2000.0, 38.0),
                (4000.0, 40.0),
            ],
        );
        let result = calculate_bd_rate(&anchor, &test).unwrap();
        assert!(
            (result.bd_quality - 2.0).abs() < 1e-6,
            "expected exactly +2.0, got {}",
            result.bd_quality
        );
    }

    #[test]
    fn better_test_curve_has_negative_bd_rate() {
        let anchor = curve(
            "anchor",
            &[
                (500.0, 28.0),
                (1000.0, 31.0),
                (2000.0, 34.0),
                (4000.0, 37.0),
            ],
        );
        let test = curve(
            "test",
            &[(400.0, 29.0), (800.0, 32.0), (1600.0, 35.0), (3200.0, 38.0)],
        );
        let result = calculate_bd_rate(&anchor, &test).unwrap();
        assert!(result.bd_rate_percent < 0.0, "{}", result.bd_rate_percent);
        assert!(result.bd_quality > 0.0, "{}", result.bd_quality);
    }

    #[test]
    fn fewer_than_four_points_is_an_error() {
        let anchor = curve(
            "anchor",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
            ],
        );
        let test = curve("test", &[(500.0, 30.0), (1000.0, 33.0), (2000.0, 36.0)]);
        let err = calculate_bd_rate(&anchor, &test).unwrap_err();
        assert!(err.to_string().contains("at least 4"), "{err}");
    }

    #[test]
    fn nan_and_non_positive_bitrate_points_are_filtered() {
        let anchor = curve(
            "anchor",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
                (f64::NAN, 40.0),
                (-1.0, 41.0),
                (5000.0, f64::INFINITY),
            ],
        );
        let test = curve(
            "test",
            &[
                (500.0, 30.0),
                (1000.0, 33.0),
                (2000.0, 36.0),
                (4000.0, 38.0),
            ],
        );
        // Should succeed (4 valid anchor points survive filtering) and match the identical case.
        let result = calculate_bd_rate(&anchor, &test).unwrap();
        assert!(
            result.bd_rate_percent.abs() < 1e-6,
            "{}",
            result.bd_rate_percent
        );
    }

    #[test]
    fn non_overlapping_domains_is_an_error() {
        let anchor = curve(
            "anchor",
            &[
                (500.0, 10.0),
                (1000.0, 12.0),
                (2000.0, 14.0),
                (4000.0, 16.0),
            ],
        );
        let test = curve(
            "test",
            &[
                (500.0, 50.0),
                (1000.0, 52.0),
                (2000.0, 54.0),
                (4000.0, 56.0),
            ],
        );
        let err = calculate_bd_rate(&anchor, &test).unwrap_err();
        assert!(err.to_string().contains("overlap"), "{err}");
    }
}
