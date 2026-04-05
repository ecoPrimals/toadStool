// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analytics utility functions

/// Helper function to calculate median
pub fn calculate_median(data: &[f64]) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }

    if len.is_multiple_of(2) {
        f64::midpoint(sorted[len / 2 - 1], sorted[len / 2])
    } else {
        sorted[len / 2]
    }
}

/// Helper function to calculate percentile
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
pub fn calculate_percentile(data: &[f64], p: f64) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }

    let index = (p * (len - 1) as f64).round() as usize;
    sorted.get(index).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_median_empty() {
        let data: Vec<f64> = vec![];
        assert!((calculate_median(&data) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_median_single() {
        let data = vec![42.0];
        assert!((calculate_median(&data) - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_median_odd() {
        let data = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        assert!((calculate_median(&data) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_median_even() {
        let data = vec![1.0, 3.0, 5.0, 7.0];
        let m = calculate_median(&data);
        assert!((m - 4.0).abs() < 1e-10, "expected 4.0, got {m}");
    }

    #[test]
    fn test_calculate_median_unsorted() {
        let data = vec![9.0, 1.0, 5.0, 3.0, 7.0];
        assert!((calculate_median(&data) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_percentile_empty() {
        let data: Vec<f64> = vec![];
        assert!((calculate_percentile(&data, 0.5) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_percentile_single() {
        let data = vec![10.0];
        assert!((calculate_percentile(&data, 0.5) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_percentile_p0() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((calculate_percentile(&data, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_percentile_p100() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((calculate_percentile(&data, 1.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_calculate_percentile_p50() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!((calculate_percentile(&data, 0.5) - 3.0).abs() < 1e-10);
    }
}
