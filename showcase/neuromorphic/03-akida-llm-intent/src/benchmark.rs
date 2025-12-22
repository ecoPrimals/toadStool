//! Benchmark Akida vs CPU vs GPU intent classification

use crate::ClassificationResult;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkResults {
    pub platform: String,
    pub samples: usize,
    pub total_time_ms: f64,
    pub avg_latency_us: f64,
    pub min_latency_us: u64,
    pub max_latency_us: u64,
    pub throughput_per_sec: f64,
    pub avg_power_mw: Option<f64>,
    pub energy_per_inference_uj: Option<f64>,
}

impl BenchmarkResults {
    /// Create from individual classification results
    pub fn from_results(platform: String, results: &[ClassificationResult]) -> Self {
        let samples = results.len();
        let total_latency_us: u64 = results.iter().map(|r| r.latency_us).sum();
        let min_latency_us = results.iter().map(|r| r.latency_us).min().unwrap_or(0);
        let max_latency_us = results.iter().map(|r| r.latency_us).max().unwrap_or(0);
        
        let avg_latency_us = total_latency_us as f64 / samples as f64;
        let total_time_ms = total_latency_us as f64 / 1000.0;
        let throughput_per_sec = 1_000_000.0 / avg_latency_us;
        
        let avg_power_mw = if results.iter().any(|r| r.power_consumption_mw.is_some()) {
            let power_sum: f64 = results
                .iter()
                .filter_map(|r| r.power_consumption_mw)
                .sum();
            let power_count = results.iter().filter(|r| r.power_consumption_mw.is_some()).count();
            Some(power_sum / power_count as f64)
        } else {
            None
        };
        
        let energy_per_inference_uj = avg_power_mw.map(|power| {
            // Energy = Power × Time
            // 1mW × 1μs = 1 picojoule = 0.000001 μJ
            power * avg_latency_us / 1_000_000.0
        });
        
        Self {
            platform,
            samples,
            total_time_ms,
            avg_latency_us,
            min_latency_us,
            max_latency_us,
            throughput_per_sec,
            avg_power_mw,
            energy_per_inference_uj,
        }
    }
    
    /// Compare with another benchmark
    pub fn speedup_vs(&self, other: &Self) -> f64 {
        other.avg_latency_us / self.avg_latency_us
    }
    
    /// Energy efficiency vs another benchmark
    pub fn energy_efficiency_vs(&self, other: &Self) -> Option<f64> {
        match (self.energy_per_inference_uj, other.energy_per_inference_uj) {
            (Some(self_energy), Some(other_energy)) => Some(other_energy / self_energy),
            _ => None,
        }
    }
    
    /// Display comparison
    pub fn display_comparison(&self, baseline: &Self) {
        println!("\n═══ {} vs {} ═══", self.platform, baseline.platform);
        println!("  Latency:    {:.1}μs vs {:.1}μs ({:.1}x faster)", 
            self.avg_latency_us, baseline.avg_latency_us, self.speedup_vs(baseline));
        println!("  Throughput: {:.0}/s vs {:.0}/s", 
            self.throughput_per_sec, baseline.throughput_per_sec);
        
        if let (Some(self_power), Some(baseline_power)) = (self.avg_power_mw, baseline.avg_power_mw) {
            println!("  Power:      {:.1}mW vs {:.1}mW ({:.1}x lower)", 
                self_power, baseline_power, baseline_power / self_power);
        }
        
        if let Some(efficiency) = self.energy_efficiency_vs(baseline) {
            println!("  Energy Efficiency: {:.1}x better", efficiency);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_results() {
        let results = vec![
            ClassificationResult {
                category: IntentCategory::CodeGeneration,
                confidence: 0.9,
                latency_us: 100,
                power_consumption_mw: Some(1.5),
            },
            ClassificationResult {
                category: IntentCategory::Debugging,
                confidence: 0.85,
                latency_us: 120,
                power_consumption_mw: Some(1.6),
            },
        ];
        
        let benchmark = BenchmarkResults::from_results("Akida".to_string(), &results);
        assert_eq!(benchmark.samples, 2);
        assert_eq!(benchmark.avg_latency_us, 110.0);
        assert_eq!(benchmark.min_latency_us, 100);
        assert_eq!(benchmark.max_latency_us, 120);
    }
}

