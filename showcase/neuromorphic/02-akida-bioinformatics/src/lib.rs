//! K-mer filtering for bioinformatics with Akida acceleration

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub mod akida_filter;
pub mod benchmark;
pub mod cpu_filter;
pub mod kmer;

/// K-mer filtering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// K-mer size (typically 31 for Kraken2)
    pub kmer_size: usize,
    
    /// Minimum GC content (0.0-1.0)
    pub min_gc_content: f64,
    
    /// Maximum GC content (0.0-1.0)
    pub max_gc_content: f64,
    
    /// Discard low-complexity sequences
    pub filter_low_complexity: bool,
    
    /// Discard adapter sequences
    pub filter_adapters: bool,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            kmer_size: 31,
            min_gc_content: 0.4,
            max_gc_content: 0.6,
            filter_low_complexity: true,
            filter_adapters: true,
        }
    }
}

/// Filtering statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStats {
    /// Total k-mers processed
    pub total_kmers: u64,
    
    /// K-mers kept
    pub kept_kmers: u64,
    
    /// K-mers discarded
    pub discarded_kmers: u64,
    
    /// Processing time in seconds
    pub processing_time_secs: f64,
    
    /// Throughput (k-mers/sec)
    pub throughput: f64,
    
    /// Power consumption in watts
    pub power_watts: f64,
    
    /// Energy efficiency (k-mers/joule)
    pub efficiency: f64,
}

impl FilterStats {
    /// Create new stats from measurements
    pub fn new(
        total_kmers: u64,
        kept_kmers: u64,
        processing_time_secs: f64,
        power_watts: f64,
    ) -> Self {
        let discarded_kmers = total_kmers.saturating_sub(kept_kmers);
        let throughput = total_kmers as f64 / processing_time_secs;
        let energy_joules = power_watts * processing_time_secs;
        let efficiency = if energy_joules > 0.0 {
            total_kmers as f64 / energy_joules
        } else {
            0.0
        };
        
        Self {
            total_kmers,
            kept_kmers,
            discarded_kmers,
            processing_time_secs,
            throughput,
            power_watts,
            efficiency,
        }
    }
    
    /// Calculate speedup compared to another run
    pub fn speedup(&self, other: &Self) -> f64 {
        other.processing_time_secs / self.processing_time_secs
    }
    
    /// Calculate power reduction compared to another run
    pub fn power_reduction(&self, other: &Self) -> f64 {
        other.power_watts / self.power_watts
    }
    
    /// Calculate efficiency gain compared to another run
    pub fn efficiency_gain(&self, other: &Self) -> f64 {
        self.efficiency / other.efficiency
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_filter_stats() {
        let stats = FilterStats::new(
            1_000_000,  // total
            900_000,    // kept
            5.0,        // time
            25.0,       // power
        );
        
        assert_eq!(stats.total_kmers, 1_000_000);
        assert_eq!(stats.kept_kmers, 900_000);
        assert_eq!(stats.discarded_kmers, 100_000);
        assert_eq!(stats.throughput, 200_000.0);
        
        // Energy = 25W * 5s = 125J
        // Efficiency = 1M / 125 = 8000 k-mers/J
        assert!((stats.efficiency - 8000.0).abs() < 0.1);
    }
    
    #[test]
    fn test_comparisons() {
        let cpu = FilterStats::new(1_000_000, 900_000, 10.0, 25.0);
        let akida = FilterStats::new(1_000_000, 900_000, 4.0, 1.0);
        
        assert!((akida.speedup(&cpu) - 2.5).abs() < 0.1);
        assert!((akida.power_reduction(&cpu) - 25.0).abs() < 0.1);
        assert!(akida.efficiency_gain(&cpu) > 60.0);
    }
}

