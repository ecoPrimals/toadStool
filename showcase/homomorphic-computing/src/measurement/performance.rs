//! Performance Profiling Infrastructure
//!
//! **Deep Debt**: Profile actual performance, don't hardcode estimates!
//!
//! This module implements real performance profiling for homomorphic operations.

use anyhow::Result;
use std::time::Instant;

/// Performance metrics for homomorphic operations
#[derive(Clone, Debug)]
pub struct PerformanceMetrics {
    /// Operations per second (throughput)
    pub ops_per_sec: f64,
    
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    
    /// Minimum latency in milliseconds
    pub min_latency_ms: f64,
    
    /// Maximum latency in milliseconds
    pub max_latency_ms: f64,
    
    /// Standard deviation of latency
    pub stddev_latency_ms: f64,
    
    /// Whether this is measured (true) or estimated (false)
    pub is_measured: bool,
}

/// Performance profiler for substrates
///
/// **Deep Debt**: Measure actual performance at runtime
pub struct PerformanceProfiler {
    warmup_iterations: usize,
    benchmark_iterations: usize,
}

impl PerformanceProfiler {
    /// Create new performance profiler
    pub fn new() -> Self {
        Self {
            warmup_iterations: 10,
            benchmark_iterations: 100,
        }
    }
    
    /// Profile a substrate's performance
    ///
    /// **Real Measurement**: Runs actual benchmarks with warmup
    pub async fn profile<F, Fut>(&self, operation: F) -> Result<PerformanceMetrics>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            operation().await?;
        }
        
        // Benchmark phase
        let mut latencies = Vec::with_capacity(self.benchmark_iterations);
        
        for _ in 0..self.benchmark_iterations {
            let start = Instant::now();
            operation().await?;
            let elapsed = start.elapsed();
            latencies.push(elapsed);
        }
        
        // Calculate statistics
        let latencies_ms: Vec<f64> = latencies.iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();
        
        let avg_latency_ms = latencies_ms.iter().sum::<f64>() / latencies_ms.len() as f64;
        let min_latency_ms = latencies_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_latency_ms = latencies_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // Calculate standard deviation
        let variance = latencies_ms.iter()
            .map(|&x| (x - avg_latency_ms).powi(2))
            .sum::<f64>() / latencies_ms.len() as f64;
        let stddev_latency_ms = variance.sqrt();
        
        // Calculate throughput (ops/sec)
        let ops_per_sec = 1000.0 / avg_latency_ms;
        
        Ok(PerformanceMetrics {
            ops_per_sec,
            avg_latency_ms,
            min_latency_ms,
            max_latency_ms,
            stddev_latency_ms,
            is_measured: true,
        })
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_performance_profiler() {
        let profiler = PerformanceProfiler::new();
        
        // Profile a simple operation
        let metrics = profiler.profile(|| async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(())
        }).await.unwrap();
        
        println!("Throughput: {:.2} ops/sec", metrics.ops_per_sec);
        println!("Latency: {:.2} ± {:.2} ms", metrics.avg_latency_ms, metrics.stddev_latency_ms);
        
        // Should be roughly 100 ops/sec (10ms per op)
        assert!(metrics.ops_per_sec > 80.0 && metrics.ops_per_sec < 120.0);
        assert!(metrics.is_measured);
    }
}
