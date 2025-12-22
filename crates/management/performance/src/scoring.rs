//! Performance scoring algorithms
//!
//! Algorithms for calculating performance and efficiency scores based on
//! runtime metrics and execution duration.

use std::time::Duration;

use toadstool::resources::RuntimeMetrics;

/// Calculate performance score based on metrics and duration
///
/// Scoring algorithm:
/// - Execution time: 40% weight (faster is better)
/// - Memory usage: 30% weight (lower is better)
/// - CPU usage: 30% weight (lower is better)
///
/// Returns a score from 0-100 (higher is better)
pub fn calculate_performance_score(metrics: &RuntimeMetrics, duration: Duration) -> f64 {
    let execution_score = calculate_execution_score(duration);
    let memory_score = calculate_memory_score(metrics);
    let cpu_score = calculate_cpu_score(metrics);

    // Weighted average
    (execution_score * 0.4 + memory_score * 0.3 + cpu_score * 0.3).min(100.0)
}

/// Calculate resource efficiency score
///
/// Measures how efficiently resources were used relative to work done.
/// Lower resource usage per unit time = higher efficiency.
///
/// Returns a score from 0-100 (higher is better)
pub fn calculate_efficiency_score(metrics: &RuntimeMetrics, duration: Duration) -> f64 {
    let memory_efficiency = calculate_memory_efficiency(metrics);
    let cpu_efficiency = calculate_cpu_efficiency(metrics);
    let time_efficiency = calculate_time_efficiency(duration);

    // Weighted average
    (memory_efficiency * 0.4 + cpu_efficiency * 0.3 + time_efficiency * 0.3).min(100.0)
}

/// Calculate execution time score (faster = higher score)
fn calculate_execution_score(duration: Duration) -> f64 {
    if duration.as_secs() > 0 {
        // Normalize against 5 minute baseline
        100.0 - (duration.as_secs_f64() / 300.0 * 100.0).min(100.0)
    } else {
        100.0
    }
}

/// Calculate memory usage score (lower usage = higher score)
fn calculate_memory_score(metrics: &RuntimeMetrics) -> f64 {
    let memory_usage_mb = metrics.memory.used_bytes as f64 / 1024.0 / 1024.0;

    // Normalize against 1GB baseline
    100.0 - (memory_usage_mb / 1024.0 * 100.0).min(100.0)
}

/// Calculate CPU usage score (lower usage = higher score)
fn calculate_cpu_score(metrics: &RuntimeMetrics) -> f64 {
    100.0 - metrics.cpu.usage_percent.min(100.0)
}

/// Calculate memory efficiency (work per MB)
fn calculate_memory_efficiency(metrics: &RuntimeMetrics) -> f64 {
    let memory_usage_mb = metrics.memory.used_bytes as f64 / 1024.0 / 1024.0;

    if memory_usage_mb > 0.0 {
        // Higher efficiency when using less memory
        100.0 / (memory_usage_mb / 1024.0).max(1.0)
    } else {
        100.0
    }
}

/// Calculate CPU efficiency (work per CPU percentage)
fn calculate_cpu_efficiency(metrics: &RuntimeMetrics) -> f64 {
    if metrics.cpu.usage_percent > 0.0 {
        // Higher efficiency when using less CPU
        100.0 / metrics.cpu.usage_percent.max(1.0)
    } else {
        100.0
    }
}

/// Calculate time efficiency (work per second)
fn calculate_time_efficiency(duration: Duration) -> f64 {
    if duration.as_secs() > 0 {
        // Higher efficiency when taking less time
        100.0 / duration.as_secs_f64().max(1.0)
    } else {
        100.0
    }
}

/// Calculate runtime selection score with custom weights
///
/// Combines multiple factors into a single weighted score for runtime selection.
pub fn calculate_weighted_score(
    avg_execution_time: Duration,
    avg_memory_usage: f64,
    avg_cpu_usage: f64,
    current_load: f64,
    success_rate: f64,
    weights: &super::types::SelectionWeights,
) -> f64 {
    let execution_score = 100.0 - (avg_execution_time.as_secs_f64() / 300.0 * 100.0).min(100.0);
    let memory_score = 100.0 - (avg_memory_usage / 1024.0 * 100.0).min(100.0);
    let cpu_score = 100.0 - avg_cpu_usage.min(100.0);
    let availability_score = 100.0 - current_load;
    let success_score = success_rate;

    weights.execution_time * execution_score
        + weights.memory_usage * memory_score
        + weights.cpu_usage * cpu_score
        + weights.resource_availability * availability_score
        + weights.historical_success_rate * success_score
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::resources::{CpuMetrics, MemoryMetrics};

    fn create_test_metrics(memory_mb: u64, cpu_percent: f64) -> RuntimeMetrics {
        RuntimeMetrics {
            memory: MemoryMetrics {
                usage_percent: (memory_mb as f64 / 8192.0) * 100.0,
                used_bytes: memory_mb * 1024 * 1024,
                peak_bytes: memory_mb * 1024 * 1024,
            },
            cpu: CpuMetrics {
                usage_percent: cpu_percent,
                cores_used: cpu_percent / 100.0 * 4.0,
                cpu_time_seconds: 0.0,
            },
            storage: toadstool::resources::StorageMetrics::default(),
            network: toadstool::resources::NetworkMetrics::default(),
            gpu: None,
            timing: toadstool::resources::TimingMetrics {
                start_time: chrono::Utc::now(),
                end_time: None,
                duration: chrono::Duration::zero(),
            },
        }
    }

    #[test]
    fn test_performance_score_excellent() {
        // Fast execution, low resource usage
        let metrics = create_test_metrics(100, 10.0);
        let duration = Duration::from_secs(1);

        let score = calculate_performance_score(&metrics, duration);
        assert!(score > 90.0, "Expected high score, got {}", score);
    }

    #[test]
    fn test_performance_score_poor() {
        // Slow execution, high resource usage
        let metrics = create_test_metrics(2048, 90.0);
        let duration = Duration::from_secs(300);

        let score = calculate_performance_score(&metrics, duration);
        assert!(score < 30.0, "Expected low score, got {}", score);
    }

    #[test]
    fn test_efficiency_score_calculation() {
        let metrics = create_test_metrics(512, 50.0);
        let duration = Duration::from_secs(10);

        let score = calculate_efficiency_score(&metrics, duration);
        assert!(score > 0.0 && score <= 100.0);
    }

    #[test]
    fn test_execution_score_fast() {
        let duration = Duration::from_secs(1);
        let score = calculate_execution_score(duration);
        assert!(score > 90.0);
    }

    #[test]
    fn test_execution_score_slow() {
        let duration = Duration::from_secs(300); // 5 minutes
        let score = calculate_execution_score(duration);
        assert!(score < 10.0);
    }

    #[test]
    fn test_memory_score_low_usage() {
        let metrics = create_test_metrics(100, 0.0); // 100MB
        let score = calculate_memory_score(&metrics);
        assert!(score > 80.0);
    }

    #[test]
    fn test_memory_score_high_usage() {
        let metrics = create_test_metrics(2048, 0.0); // 2GB
        let score = calculate_memory_score(&metrics);
        assert!(score < 50.0);
    }

    #[test]
    fn test_cpu_score_low_usage() {
        let metrics = create_test_metrics(0, 10.0);
        let score = calculate_cpu_score(&metrics);
        assert!(score > 85.0);
    }

    #[test]
    fn test_cpu_score_high_usage() {
        let metrics = create_test_metrics(0, 95.0);
        let score = calculate_cpu_score(&metrics);
        assert!(score < 10.0);
    }
}
