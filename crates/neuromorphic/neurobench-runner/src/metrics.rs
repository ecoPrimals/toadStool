//! Metrics collection for NeuroBench
//!
//! Provides detailed metric tracking beyond the summary in BenchmarkResult.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Comprehensive metrics container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub latency: LatencyMetrics,
    pub power: PowerMetrics,
    pub throughput: ThroughputMetrics,
    pub accuracy: AccuracyMetrics,
}

/// Latency distribution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyMetrics {
    /// All latency samples
    pub samples: Vec<Duration>,
    /// Minimum latency
    pub min: Duration,
    /// Maximum latency
    pub max: Duration,
    /// Mean latency
    pub mean: Duration,
    /// Median latency
    pub median: Duration,
    /// Standard deviation (as Duration)
    pub std_dev_nanos: u64,
    /// Percentiles
    pub p50: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p999: Duration,
}

impl LatencyMetrics {
    /// Calculate metrics from raw samples
    pub fn from_samples(samples: Vec<Duration>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        
        let mut sorted: Vec<_> = samples.iter().copied().collect();
        sorted.sort();
        
        let n = sorted.len();
        let min = sorted[0];
        let max = sorted[n - 1];
        let median = sorted[n / 2];
        
        // Mean
        let total_nanos: u128 = sorted.iter().map(|d| d.as_nanos()).sum();
        let mean_nanos = total_nanos / n as u128;
        let mean = Duration::from_nanos(mean_nanos as u64);
        
        // Standard deviation
        let variance: f64 = sorted.iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos as f64;
                diff * diff
            })
            .sum::<f64>() / n as f64;
        let std_dev_nanos = variance.sqrt() as u64;
        
        // Percentiles
        let percentile = |p: f64| -> Duration {
            let idx = ((n as f64 * p) as usize).min(n - 1);
            sorted[idx]
        };
        
        Self {
            samples,
            min,
            max,
            mean,
            median,
            std_dev_nanos,
            p50: percentile(0.50),
            p90: percentile(0.90),
            p95: percentile(0.95),
            p99: percentile(0.99),
            p999: percentile(0.999),
        }
    }
}

/// Power consumption metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PowerMetrics {
    /// Power samples (Watts)
    pub samples: Vec<f64>,
    /// Minimum power (W)
    pub min_w: f64,
    /// Maximum power (W)
    pub max_w: f64,
    /// Mean power (W)
    pub mean_w: f64,
    /// Idle power (W)
    pub idle_w: Option<f64>,
    /// Energy per inference (uJ)
    pub energy_per_inference_uj: Option<f64>,
}

impl PowerMetrics {
    /// Calculate metrics from raw samples
    pub fn from_samples(samples: Vec<f64>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        
        let min_w = samples.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_w = samples.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mean_w = samples.iter().sum::<f64>() / samples.len() as f64;
        
        Self {
            samples,
            min_w,
            max_w,
            mean_w,
            idle_w: None,
            energy_per_inference_uj: None,
        }
    }
    
    /// Calculate energy per inference given latency
    pub fn with_latency(&mut self, mean_latency: Duration) {
        let latency_us = mean_latency.as_micros() as f64;
        // Power (W) * time (us) = energy (uJ) / 1e6, so W * us / 1e6 * 1e6 = uJ
        self.energy_per_inference_uj = Some(self.mean_w * latency_us);
    }
}

/// Throughput metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThroughputMetrics {
    /// Inferences per second
    pub inferences_per_second: f64,
    /// Total inferences
    pub total_inferences: usize,
    /// Total time
    pub total_time: Duration,
    /// Effective batch size
    pub batch_size: usize,
}

/// Accuracy metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccuracyMetrics {
    /// Top-1 accuracy
    pub top1: f64,
    /// Top-5 accuracy
    pub top5: f64,
    /// Confusion matrix (flattened row-major)
    pub confusion_matrix: Vec<usize>,
    /// Number of classes
    pub num_classes: usize,
    /// Per-class accuracy
    pub per_class_accuracy: Vec<f64>,
}

impl AccuracyMetrics {
    /// Create new accuracy metrics
    pub fn new(num_classes: usize) -> Self {
        Self {
            num_classes,
            confusion_matrix: vec![0; num_classes * num_classes],
            per_class_accuracy: vec![0.0; num_classes],
            ..Default::default()
        }
    }
    
    /// Record a prediction
    pub fn record(&mut self, predicted: usize, actual: usize) {
        if predicted < self.num_classes && actual < self.num_classes {
            self.confusion_matrix[actual * self.num_classes + predicted] += 1;
        }
    }
    
    /// Finalize accuracy calculations
    pub fn finalize(&mut self) {
        let mut total_correct = 0;
        let mut total_samples = 0;
        
        for class in 0..self.num_classes {
            let class_total: usize = self.confusion_matrix
                [class * self.num_classes..(class + 1) * self.num_classes]
                .iter()
                .sum();
            
            if class_total > 0 {
                let class_correct = self.confusion_matrix[class * self.num_classes + class];
                self.per_class_accuracy[class] = class_correct as f64 / class_total as f64;
                total_correct += class_correct;
                total_samples += class_total;
            }
        }
        
        if total_samples > 0 {
            self.top1 = total_correct as f64 / total_samples as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_latency_metrics() {
        let samples: Vec<Duration> = (1..=100)
            .map(|i| Duration::from_micros(i * 10))
            .collect();
        
        let metrics = LatencyMetrics::from_samples(samples);
        
        assert_eq!(metrics.min, Duration::from_micros(10));
        assert_eq!(metrics.max, Duration::from_micros(1000));
        println!("Mean latency: {:?}", metrics.mean);
        println!("P99 latency: {:?}", metrics.p99);
    }
    
    #[test]
    fn test_accuracy_metrics() {
        let mut metrics = AccuracyMetrics::new(3);
        
        // Record predictions
        metrics.record(0, 0); // correct
        metrics.record(0, 0); // correct
        metrics.record(1, 0); // wrong
        metrics.record(1, 1); // correct
        metrics.record(2, 2); // correct
        
        metrics.finalize();
        
        assert_eq!(metrics.top1, 4.0 / 5.0);
        println!("Top-1 accuracy: {:.2}%", metrics.top1 * 100.0);
    }
}
