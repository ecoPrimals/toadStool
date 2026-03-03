// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmarking utilities for comparing CPU vs Akida

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{FilterConfig, FilterStats};

/// Comparison results between CPU and Akida
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResults {
    /// CPU baseline stats
    pub cpu: FilterStats,
    
    /// Akida accelerated stats
    pub akida: FilterStats,
    
    /// Speedup factor
    pub speedup: f64,
    
    /// Power reduction factor
    pub power_reduction: f64,
    
    /// Efficiency gain factor
    pub efficiency_gain: f64,
}

impl ComparisonResults {
    /// Create comparison from individual stats
    pub fn from_stats(cpu: FilterStats, akida: FilterStats) -> Self {
        let speedup = akida.speedup(&cpu);
        let power_reduction = akida.power_reduction(&cpu);
        let efficiency_gain = akida.efficiency_gain(&cpu);
        
        Self {
            cpu,
            akida,
            speedup,
            power_reduction,
            efficiency_gain,
        }
    }
    
    /// Save results to JSON file
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    /// Load results from JSON file
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let results = serde_json::from_str(&json)?;
        Ok(results)
    }
    
    /// Print comparison summary
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║             CPU vs Akida Comparison Results               ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");
        
        println!("CPU Baseline:");
        println!("  Total k-mers: {:,}", self.cpu.total_kmers);
        println!("  Kept k-mers: {:,}", self.cpu.kept_kmers);
        println!("  Processing time: {:.2}s", self.cpu.processing_time_secs);
        println!("  Throughput: {:.0} k-mers/sec", self.cpu.throughput);
        println!("  Power: {:.1}W", self.cpu.power_watts);
        println!("  Efficiency: {:.0} k-mers/joule\n", self.cpu.efficiency);
        
        println!("Akida Accelerated:");
        println!("  Total k-mers: {:,}", self.akida.total_kmers);
        println!("  Kept k-mers: {:,}", self.akida.kept_kmers);
        println!("  Processing time: {:.2}s", self.akida.processing_time_secs);
        println!("  Throughput: {:.0} k-mers/sec", self.akida.throughput);
        println!("  Power: {:.1}W", self.akida.power_watts);
        println!("  Efficiency: {:.0} k-mers/joule\n", self.akida.efficiency);
        
        println!("Improvements:");
        println!("  Speedup: {:.1}x faster", self.speedup);
        println!("  Power reduction: {:.1}x less power", self.power_reduction);
        println!("  Efficiency gain: {:.0}x more efficient\n", self.efficiency_gain);
    }
}

/// Generate sample DNA sequences for benchmarking
pub fn generate_sample_sequences(count: usize, length: usize) -> Vec<Vec<u8>> {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    let bases = [b'A', b'C', b'G', b'T'];
    
    (0..count)
        .map(|_| {
            (0..length)
                .map(|_| bases[rng.gen_range(0..4)])
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_comparison_results() {
        let cpu = FilterStats::new(1_000_000, 900_000, 10.0, 25.0);
        let akida = FilterStats::new(1_000_000, 900_000, 4.0, 1.0);
        
        let comparison = ComparisonResults::from_stats(cpu, akida);
        
        assert!((comparison.speedup - 2.5).abs() < 0.1);
        assert!((comparison.power_reduction - 25.0).abs() < 0.1);
        assert!(comparison.efficiency_gain > 60.0);
    }
    
    #[test]
    fn test_generate_sequences() {
        let sequences = generate_sample_sequences(10, 100);
        
        assert_eq!(sequences.len(), 10);
        assert!(sequences.iter().all(|s| s.len() == 100));
        
        // Check that sequences contain only valid bases
        for seq in &sequences {
            assert!(seq.iter().all(|&b| matches!(b, b'A' | b'C' | b'G' | b'T')));
        }
    }
}

