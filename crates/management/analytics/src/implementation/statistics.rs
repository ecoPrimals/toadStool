// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::cast_precision_loss)]

use std::time::{Duration, SystemTime};

use statrs::statistics::Statistics;

use crate::types::{PredictionPoint, TrendStatistics};
use crate::utils::{calculate_median, calculate_percentile};

pub fn perform_statistical_analysis(data: &[f64]) -> TrendStatistics {
    let mean = data.mean();
    let median = calculate_median(data);
    let std_deviation = data.std_dev();
    let min = data.min();
    let max = data.max();
    let percentile_95 = calculate_percentile(data, 0.95);

    let correlation_coefficient = if data.len() > 1 {
        let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
        let n = data.len() as f64;
        let x_mean: f64 = x.iter().sum::<f64>() / n;
        let y_mean: f64 = data.iter().sum::<f64>() / n;

        let numerator: f64 = x
            .iter()
            .zip(data.iter())
            .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
            .sum();

        let x_variance: f64 = x.iter().map(|xi| (xi - x_mean).powi(2)).sum();
        let y_variance: f64 = data.iter().map(|yi| (yi - y_mean).powi(2)).sum();

        if x_variance > 0.0 && y_variance > 0.0 {
            numerator / (x_variance * y_variance).sqrt()
        } else {
            0.0
        }
    } else {
        0.0
    };

    TrendStatistics {
        mean,
        median,
        std_deviation,
        min,
        max,
        percentile_95,
        correlation_coefficient,
    }
}

pub fn generate_predictions(data: &[f64], hours_ahead: u32) -> Vec<PredictionPoint> {
    if data.len() < 2 {
        return Vec::new();
    }

    let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
    let n = data.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = data.iter().sum();
    let sum_x_times_y: f64 = x.iter().zip(data.iter()).map(|(xi, yi)| xi * yi).sum();
    let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();

    let denominator = n.mul_add(sum_x2, -(sum_x * sum_x));
    if denominator.abs() < f64::EPSILON {
        return Vec::new();
    }

    let slope = (n.mul_add(sum_x_times_y, -(sum_x * sum_y))) / denominator;
    let intercept = slope.mul_add(-sum_x, sum_y) / n;

    let current_time = SystemTime::now();

    (1..=hours_ahead)
        .map(|i| {
            let future_x = data.len() as f64 + f64::from(i);
            let predicted_value = slope.mul_add(future_x, intercept);

            let std_error = (data
                .iter()
                .map(|yi| yi - predicted_value)
                .map(|diff| diff * diff)
                .sum::<f64>()
                / n)
                .sqrt();
            let confidence_interval = (
                1.96f64.mul_add(-std_error, predicted_value),
                1.96f64.mul_add(std_error, predicted_value),
            );

            PredictionPoint {
                timestamp: current_time + Duration::from_secs(u64::from(i) * 3600),
                predicted_value,
                confidence_interval,
                prediction_method: "linear_regression".to_string(),
            }
        })
        .collect()
}
