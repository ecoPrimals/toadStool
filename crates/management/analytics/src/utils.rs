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
