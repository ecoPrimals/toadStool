// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU-based k-mer filtering (baseline)

use anyhow::Result;
use rayon::prelude::*;

use crate::kmer::{extract_kmers, gc_content, is_adapter, is_low_complexity};
use crate::{FilterConfig, FilterStats};

/// Filter k-mers using CPU
pub fn filter_kmers_cpu(
    sequences: &[Vec<u8>],
    config: &FilterConfig,
) -> Result<FilterStats> {
    let start = std::time::Instant::now();
    
    // Extract all k-mers from all sequences
    let all_kmers: Vec<Vec<u8>> = sequences
        .par_iter()
        .flat_map(|seq| extract_kmers(seq, config.kmer_size))
        .collect();
    
    let total_kmers = all_kmers.len() as u64;
    
    // Filter k-mers in parallel
    let kept_kmers = all_kmers
        .par_iter()
        .filter(|kmer| should_keep_kmer(kmer, config))
        .count() as u64;
    
    let elapsed = start.elapsed().as_secs_f64();
    
    // Estimate CPU power consumption (typical for 8-core workload)
    let power_watts = estimate_cpu_power(sequences.len(), elapsed);
    
    Ok(FilterStats::new(
        total_kmers,
        kept_kmers,
        elapsed,
        power_watts,
    ))
}

/// Check if k-mer should be kept based on filtering criteria
fn should_keep_kmer(kmer: &[u8], config: &FilterConfig) -> bool {
    // Check GC content
    let gc = gc_content(kmer);
    if gc < config.min_gc_content || gc > config.max_gc_content {
        return false;
    }
    
    // Check low complexity
    if config.filter_low_complexity && is_low_complexity(kmer) {
        return false;
    }
    
    // Check adapters
    if config.filter_adapters && is_adapter(kmer) {
        return false;
    }
    
    true
}

/// Estimate CPU power consumption based on workload
fn estimate_cpu_power(sequence_count: usize, duration_secs: f64) -> f64 {
    // Base idle power per core: ~5W
    // Active power per core: ~15W
    // Typical 8-core filtering workload
    
    let num_cores = rayon::current_num_threads();
    let base_power = 5.0 * num_cores as f64;
    let active_power = 15.0 * num_cores as f64;
    
    // Assume 70% active during filtering
    let utilization = 0.7;
    
    base_power + (active_power * utilization)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cpu_filtering() {
        let sequences = vec![
            b"ACGTACGTACGTACGTACGTACGTACGTACGTACGT".to_vec(),
            b"GCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTAGCTA".to_vec(),
        ];
        
        let config = FilterConfig::default();
        let stats = filter_kmers_cpu(&sequences, &config).unwrap();
        
        assert!(stats.total_kmers > 0);
        assert!(stats.kept_kmers <= stats.total_kmers);
        assert!(stats.throughput > 0.0);
        assert!(stats.power_watts > 0.0);
    }
    
    #[test]
    fn test_kmer_filtering_logic() {
        let config = FilterConfig::default();
        
        // Good k-mer: balanced GC content
        let good_kmer = b"ACGTACGTACGTACGTACGTACGTACGTACGT";
        assert!(should_keep_kmer(good_kmer, &config));
        
        // Bad k-mer: too low GC
        let low_gc = b"AAAAAAAAAA AAAAAAAAAAAAAAAAAAAAAA";
        assert!(!should_keep_kmer(low_gc, &config));
        
        // Bad k-mer: low complexity
        let low_complex = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert!(!should_keep_kmer(low_complex, &config));
    }
}

