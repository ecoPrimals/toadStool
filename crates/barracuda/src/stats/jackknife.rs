// SPDX-License-Identifier: AGPL-3.0-or-later
//! Leave-one-out jackknife estimator.
//!
//! Provenance: groundSpring `jackknife.rs` -> toadStool absorption (S70).

/// Result of a jackknife estimate.
#[derive(Debug, Clone, Copy)]
pub struct JackknifeResult {
    /// Jackknife estimate of the statistic.
    pub estimate: f64,
    /// Jackknife variance of the estimator.
    pub variance: f64,
    /// Standard error (sqrt of variance).
    pub std_error: f64,
}

/// Leave-one-out jackknife for the mean.
///
/// Returns `None` if fewer than 2 observations.
///
/// # Complexity
/// O(n) time, O(n) space for leave-one-out means.
#[must_use]
pub fn jackknife_mean_variance(data: &[f64]) -> Option<JackknifeResult> {
    let n = data.len();
    if n < 2 {
        return None;
    }

    let n_f = n as f64;
    let full_sum: f64 = data.iter().sum();
    let full_mean = full_sum / n_f;

    let mut jk_mean_sum = 0.0;
    let mut jk_means = Vec::with_capacity(n);

    for &d in data {
        let leave_mean = (full_sum - d) / (n_f - 1.0);
        jk_means.push(leave_mean);
        jk_mean_sum += leave_mean;
    }

    let jk_grand_mean = jk_mean_sum / n_f;
    let jk_var = (n_f - 1.0) / n_f
        * jk_means
            .iter()
            .map(|&m| (m - jk_grand_mean).powi(2))
            .sum::<f64>();

    Some(JackknifeResult {
        estimate: full_mean,
        variance: jk_var,
        std_error: jk_var.sqrt(),
    })
}

/// Generalized jackknife for an arbitrary statistic.
///
/// `statistic` is called n+1 times: once for the full dataset, then n times
/// with each observation removed.
///
/// Returns `None` if fewer than 2 observations.
#[must_use]
pub fn jackknife<F>(data: &[f64], statistic: F) -> Option<JackknifeResult>
where
    F: Fn(&[f64]) -> f64,
{
    let n = data.len();
    if n < 2 {
        return None;
    }

    let n_f = n as f64;
    let full_stat = statistic(data);

    let mut leave_out = Vec::with_capacity(n - 1);
    let mut pseudovalues = Vec::with_capacity(n);

    for i in 0..n {
        leave_out.clear();
        leave_out.extend_from_slice(&data[..i]);
        leave_out.extend_from_slice(&data[i + 1..]);
        let theta_i = statistic(&leave_out);
        pseudovalues.push(n_f * full_stat - (n_f - 1.0) * theta_i);
    }

    let mean_pseudo: f64 = pseudovalues.iter().sum::<f64>() / n_f;
    let var = pseudovalues
        .iter()
        .map(|&p| (p - mean_pseudo).powi(2))
        .sum::<f64>()
        / (n_f * (n_f - 1.0));

    Some(JackknifeResult {
        estimate: mean_pseudo,
        variance: var,
        std_error: var.sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jackknife_mean_basic() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let result = jackknife_mean_variance(&data).unwrap();
        assert!((result.estimate - 3.0).abs() < 1e-12);
        assert!(result.variance >= 0.0);
        assert!(result.std_error >= 0.0);
    }

    #[test]
    fn test_jackknife_mean_two_elements() {
        let data = [10.0, 20.0];
        let result = jackknife_mean_variance(&data).unwrap();
        assert!((result.estimate - 15.0).abs() < 1e-12);
    }

    #[test]
    fn test_jackknife_mean_too_few() {
        assert!(jackknife_mean_variance(&[]).is_none());
        assert!(jackknife_mean_variance(&[1.0]).is_none());
    }

    #[test]
    fn test_jackknife_constant() {
        let data = [5.0; 10];
        let result = jackknife_mean_variance(&data).unwrap();
        assert!((result.estimate - 5.0).abs() < 1e-12);
        assert!(result.variance < 1e-20);
    }

    #[test]
    fn test_jackknife_generalized() {
        let data = [2.0, 4.0, 6.0, 8.0];
        let result = jackknife(&data, |d| d.iter().sum::<f64>() / d.len() as f64).unwrap();
        assert!((result.estimate - 5.0).abs() < 1e-10);
    }
}
