// SPDX-License-Identifier: AGPL-3.0-only

//! Agreement metrics and descriptive statistics.
//!
//! Absorbed from airSpring `testutil/stats.rs` and groundSpring `stats/metrics.rs`
//! (Feb 2026, Session 64). These are domain-agnostic statistical measures used
//! across all springs for model validation.
//!
//! # Agreement metrics
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`rmse`] | Root Mean Square Error |
//! | [`mbe`] | Mean Bias Error |
//! | [`nash_sutcliffe`] | Nash-Sutcliffe Efficiency (NSE) |
//! | [`r_squared`] | Coefficient of determination (R²) via SS method |
//! | [`index_of_agreement`] | Willmott Index of Agreement (IA) |
//! | [`hit_rate`] | Threshold-based occurrence agreement |
//!
//! # Descriptive statistics
//!
//! | Function | Description |
//! |----------|-------------|
//! | [`mean`] | Arithmetic mean |
//! | [`percentile`] | Interpolated percentile (0–100 scale) |

// ── Agreement metrics ───────────────────────────────────────────────────

/// Root Mean Square Error.
///
/// RMSE = sqrt(Σ(obs - sim)² / n)
///
/// Returns `0.0` for empty slices.
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn rmse(observed: &[f64], simulated: &[f64]) -> f64 {
    assert_eq!(observed.len(), simulated.len(), "length mismatch");
    let n = observed.len();
    if n == 0 {
        return 0.0;
    }
    let sum_sq: f64 = observed
        .iter()
        .zip(simulated)
        .map(|(o, s)| (o - s).powi(2))
        .sum();
    (sum_sq / n as f64).sqrt()
}

/// Mean Bias Error (simulated − observed).
///
/// Positive MBE indicates the model overestimates.
///
/// Returns `0.0` for empty slices.
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn mbe(observed: &[f64], simulated: &[f64]) -> f64 {
    assert_eq!(observed.len(), simulated.len(), "length mismatch");
    let n = observed.len();
    if n == 0 {
        return 0.0;
    }
    let sum_bias: f64 = observed
        .iter()
        .zip(simulated)
        .map(|(o, s)| s - o)
        .sum();
    sum_bias / n as f64
}

/// Nash-Sutcliffe Efficiency (Nash & Sutcliffe, 1970).
///
/// NSE = 1 − Σ(obs − sim)² / Σ(obs − obs̄)²
///
/// NSE = 1.0 is perfect; NSE < 0 means the model is worse than the mean.
///
/// Returns `0.0` when `ss_tot` is zero (constant observations) or for empty slices.
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn nash_sutcliffe(observed: &[f64], simulated: &[f64]) -> f64 {
    assert_eq!(observed.len(), simulated.len(), "length mismatch");
    let n = observed.len();
    if n == 0 {
        return 0.0;
    }
    let mean_obs: f64 = observed.iter().sum::<f64>() / n as f64;
    let ss_res: f64 = observed
        .iter()
        .zip(simulated)
        .map(|(o, s)| (o - s).powi(2))
        .sum();
    let ss_tot: f64 = observed.iter().map(|o| (o - mean_obs).powi(2)).sum();
    if ss_tot == 0.0 {
        return 0.0;
    }
    1.0 - ss_res / ss_tot
}

/// Coefficient of determination (R²) via sum-of-squares method.
///
/// R² = 1 − SS_res / SS_tot
///
/// Can be negative for poor models. Equivalent to [`nash_sutcliffe`].
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn r_squared(observed: &[f64], simulated: &[f64]) -> f64 {
    nash_sutcliffe(observed, simulated)
}

/// Index of Agreement (Willmott, 1981).
///
/// IA = 1 − Σ(obs − sim)² / Σ(|sim − obs̄| + |obs − obs̄|)²
///
/// Values range from 0.0 (no agreement) to 1.0 (perfect).
///
/// Returns `0.0` when denominator is zero or for empty slices.
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn index_of_agreement(observed: &[f64], simulated: &[f64]) -> f64 {
    assert_eq!(observed.len(), simulated.len(), "length mismatch");
    let n = observed.len();
    if n == 0 {
        return 0.0;
    }
    let mean_obs: f64 = observed.iter().sum::<f64>() / n as f64;
    let numerator: f64 = observed
        .iter()
        .zip(simulated)
        .map(|(o, s)| (o - s).powi(2))
        .sum();
    let denominator: f64 = observed
        .iter()
        .zip(simulated)
        .map(|(o, s)| ((s - mean_obs).abs() + (o - mean_obs).abs()).powi(2))
        .sum();
    if denominator == 0.0 {
        return 0.0;
    }
    1.0 - numerator / denominator
}

/// Hit rate: fraction of days where observed and modeled agree on
/// threshold exceedance.
///
/// A day "occurs" if its value exceeds `threshold`. Returns the fraction
/// of entries where both agree (both above or both at-or-below).
///
/// Returns `0.0` for empty slices.
///
/// # Panics
///
/// Panics if `observed` and `simulated` have different lengths.
#[must_use]
pub fn hit_rate(observed: &[f64], simulated: &[f64], threshold: f64) -> f64 {
    assert_eq!(observed.len(), simulated.len(), "length mismatch");
    let n = observed.len();
    if n == 0 {
        return 0.0;
    }
    let agree = observed
        .iter()
        .zip(simulated)
        .filter(|(&o, &s)| (o > threshold) == (s > threshold))
        .count();
    agree as f64 / n as f64
}

// ── Descriptive statistics ──────────────────────────────────────────────

/// Arithmetic mean of a slice. Returns `0.0` for empty slices.
#[must_use]
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Interpolated percentile (0–100 scale).
///
/// Uses linear interpolation between adjacent ranks.
///
/// Returns `0.0` for empty slices.
///
/// # Panics
///
/// Panics if `p` is not in 0.0..=100.0.
#[must_use]
pub fn percentile(values: &[f64], p: f64) -> f64 {
    assert!((0.0..=100.0).contains(&p), "percentile must be 0–100");
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = rank - lo as f64;
        sorted[lo].mul_add(1.0 - frac, sorted[hi] * frac)
    }
}

/// CPU dot product: Σ aᵢ·bᵢ.
///
/// For GPU dot product, use [`crate::ops::fused_map_reduce_f64::FusedMapReduceF64::dot`]
/// or [`crate::ops::weighted_dot_f64::WeightedDotF64`].
///
/// # Panics
///
/// Panics if `a` and `b` have different lengths.
#[must_use]
pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "length mismatch");
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// CPU L2 norm: sqrt(Σ xᵢ²).
///
/// For GPU L2 norm, use [`crate::ops::norm_reduce_f64::NormReduceF64::l2`].
#[must_use]
pub fn l2_norm(xs: &[f64]) -> f64 {
    xs.iter().map(|x| x * x).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rmse_identical_is_zero() {
        let x = [1.0, 2.0, 3.0];
        assert!(rmse(&x, &x) < 1e-12);
    }

    #[test]
    fn rmse_known_value() {
        let obs = [1.0, 2.0, 3.0];
        let sim = [1.0, 3.0, 3.0];
        let expected = (1.0_f64 / 3.0).sqrt();
        assert!((rmse(&obs, &sim) - expected).abs() < 1e-10);
    }

    #[test]
    fn rmse_empty() {
        assert!(rmse(&[], &[]) < 1e-12);
    }

    #[test]
    fn mbe_positive_bias() {
        let obs = [1.0, 2.0, 3.0];
        let sim = [2.0, 3.0, 4.0];
        assert!((mbe(&obs, &sim) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn mbe_negative_bias() {
        let obs = [5.0, 6.0, 7.0];
        let sim = [4.0, 5.0, 6.0];
        assert!((mbe(&obs, &sim) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn nash_sutcliffe_perfect() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((nash_sutcliffe(&x, &x) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn nash_sutcliffe_worse_than_mean_is_negative() {
        let obs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let sim = [5.0, 4.0, 3.0, 2.0, 1.0];
        assert!(nash_sutcliffe(&obs, &sim) < 0.0);
    }

    #[test]
    fn nash_sutcliffe_mean_predictor_is_zero() {
        let obs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let pred = vec![3.0; 5];
        assert!(nash_sutcliffe(&obs, &pred).abs() < 1e-10);
    }

    #[test]
    fn index_of_agreement_perfect() {
        let x = [1.0, 2.0, 3.0, 4.0];
        assert!((index_of_agreement(&x, &x) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn hit_rate_perfect() {
        let obs = [0.0, 5.0, 0.0, 3.0];
        assert!((hit_rate(&obs, &obs, 0.1) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn hit_rate_known() {
        let obs = [0.0, 5.0, 0.0, 3.0];
        let sim = [0.0, 4.0, 0.0, 0.0];
        assert!((hit_rate(&obs, &sim, 0.1) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn percentile_median() {
        let vals = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((percentile(&vals, 50.0) - 3.0).abs() < 1e-12);
    }

    #[test]
    fn percentile_interpolation() {
        let vals = [1.0, 2.0, 3.0, 4.0];
        assert!((percentile(&vals, 25.0) - 1.75).abs() < 1e-12);
    }

    #[test]
    fn mean_known() {
        assert!((mean(&[2.0, 4.0, 6.0]) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn mean_empty() {
        assert!(mean(&[]).abs() < 1e-12);
    }

    #[test]
    fn dot_known() {
        assert!((dot(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]) - 32.0).abs() < 1e-12);
    }

    #[test]
    fn l2_norm_known() {
        assert!((l2_norm(&[3.0, 4.0]) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn l2_norm_empty() {
        assert!(l2_norm(&[]).abs() < 1e-12);
    }
}
