// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU substrate for homomorphic operations
//!
//! Pure Rust baseline implementation using concrete-core primitives.
//! This serves as the reference implementation for correctness validation.

use super::HomomorphicSubstrate;
use crate::{schemes::HomomorphicScheme, BenchmarkResult};
use anyhow::Result;
use std::time::Instant;

/// CPU-based homomorphic compute substrate
pub struct CpuHomomorphic {
    scheme: Box<dyn HomomorphicScheme + Send + Sync>,
}

impl CpuHomomorphic {
    /// Create new CPU substrate with BFV scheme
    pub fn new() -> Result<Self> {
        use crate::schemes::BfvScheme;
        Ok(Self {
            scheme: Box::new(BfvScheme::new()?),
        })
    }

    /// Create with custom scheme
    pub fn with_scheme(scheme: Box<dyn HomomorphicScheme + Send + Sync>) -> Self {
        Self { scheme }
    }
}

impl Default for CpuHomomorphic {
    fn default() -> Self {
        Self::new().expect("Failed to create CPU substrate")
    }
}

#[async_trait::async_trait]
impl HomomorphicSubstrate for CpuHomomorphic {
    fn name(&self) -> &str {
        "CPU (Pure Rust)"
    }

    async fn encrypted_add_batch(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>> {
        // Encrypt both inputs
        let enc_a = self.scheme.encrypt(a)?;
        let enc_b = self.scheme.encrypt(b)?;

        // Homomorphic addition (on encrypted data!)
        let enc_sum = self.scheme.add(&enc_a, &enc_b)?;

        // For benchmarking, we return encrypted result
        // In production, only authorized parties would decrypt
        Ok(enc_sum)
    }

    async fn encrypted_multiply_batch(&self, a: &[u64], b: &[u64]) -> Result<Vec<u64>> {
        let enc_a = self.scheme.encrypt(a)?;
        let enc_b = self.scheme.encrypt(b)?;

        // Homomorphic multiplication
        let enc_product = self.scheme.multiply(&enc_a, &enc_b)?;

        Ok(enc_product)
    }

    async fn benchmark(&self, dataset_size: usize, iterations: usize) -> Result<BenchmarkResult> {
        // Generate random dataset (before any awaits for Send)
        let (a, b) = {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let a: Vec<u64> = (0..dataset_size).map(|_| rng.gen_range(0..1000)).collect();
            let b: Vec<u64> = (0..dataset_size).map(|_| rng.gen_range(0..1000)).collect();
            (a, b)
        };

        // Warm-up
        let _ = self.encrypted_add_batch(&a[..10], &b[..10]).await?;

        // Benchmark
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = self.encrypted_add_batch(&a, &b).await?;
        }
        let duration = start.elapsed();

        let total_ops = dataset_size * iterations;
        let duration_secs = duration.as_secs_f64();
        let throughput = total_ops as f64 / duration_secs;
        let latency_ms = (duration_secs * 1000.0) / iterations as f64;

        // Real CPU power via RAPL (Linux), fallback to typical estimate
        let power_watts = Self::measure_cpu_power();
        let ops_per_joule = throughput / power_watts;

        Ok(BenchmarkResult {
            substrate_name: self.name().to_string(),
            throughput_ops_per_sec: throughput,
            latency_ms,
            power_watts,
            ops_per_joule,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn measure_power(&self) -> Option<f64> {
        Some(Self::measure_cpu_power())
    }
}

impl CpuHomomorphic {
    /// Query real-time CPU package power via RAPL (Linux)
    /// Falls back to typical single-core estimate if RAPL unavailable
    fn measure_cpu_power() -> f64 {
        let rapl_path = "/sys/class/powercap/intel-rapl:0/energy_uj";
        if let Ok(energy_before) = std::fs::read_to_string(rapl_path) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(energy_after) = std::fs::read_to_string(rapl_path) {
                if let (Ok(before), Ok(after)) = (
                    energy_before.trim().parse::<u64>(),
                    energy_after.trim().parse::<u64>(),
                ) {
                    let delta_uj = after.saturating_sub(before);
                    return delta_uj as f64 / 100_000.0; // 100ms sample → watts
                }
            }
        }
        tracing::warn!("CPU power: using typical estimate (RAPL unavailable)");
        25.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cpu_encrypted_add() {
        let cpu = CpuHomomorphic::new().unwrap();

        let a = vec![10, 20, 30];
        let b = vec![5, 10, 15];

        let result = cpu.encrypted_add_batch(&a, &b).await.unwrap();

        // Result should be encrypted (non-zero ciphertext)
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_cpu_benchmark() {
        let cpu = CpuHomomorphic::new().unwrap();

        let result = cpu.benchmark(100, 10).await.unwrap();

        assert_eq!(result.substrate_name, "CPU (Pure Rust)");
        assert!(result.throughput_ops_per_sec > 0.0);
        assert!(result.latency_ms > 0.0);
        assert!(result.power_watts > 0.0); // Real measurement or estimate
    }
}
