//! WebGPU Systematic Experimentation Framework
//!
//! This module provides infrastructure for running systematic GPU experiments
//! to build an evidence-based understanding of WebGPU performance characteristics.
//!
//! Philosophy: "Measure everything. Assume nothing. Build knowledge systematically."

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of a single experimental run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// Unique experiment ID
    pub experiment_id: String,
    
    /// Hardware information
    pub hardware: HardwareInfo,
    
    /// Experiment parameters
    pub parameters: HashMap<String, serde_json::Value>,
    
    /// Measurements collected
    pub measurements: Measurements,
    
    /// Statistical summary
    pub statistics: Statistics,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Hardware information for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub gpu_name: String,
    pub gpu_vendor: String,
    pub backend: String, // "Vulkan", "Metal", "DX12", "OpenGL"
    pub driver_version: String,
    pub memory_size_mb: u64,
}

/// Measurements from experiment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Measurements {
    /// Execution times (one per run)
    pub execution_times_us: Vec<f64>,
    
    /// Memory bandwidth achieved (if measurable)
    pub bandwidth_gbs: Option<f64>,
    
    /// GPU occupancy (if measurable)
    pub occupancy_percent: Option<f64>,
    
    /// Custom measurements
    pub custom: HashMap<String, f64>,
}

/// Statistical summary of measurements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub mean_us: f64,
    pub median_us: f64,
    pub std_dev_us: f64,
    pub min_us: f64,
    pub max_us: f64,
    pub sample_size: usize,
    pub confidence_interval_95: (f64, f64),
}

/// Experiment configuration
#[derive(Debug, Clone)]
pub struct ExperimentConfig {
    /// Experiment ID
    pub id: String,
    
    /// Number of warmup runs (excluded from results)
    pub warmup_runs: usize,
    
    /// Number of measurement runs
    pub measurement_runs: usize,
    
    /// Parameters to sweep
    pub parameters: HashMap<String, Vec<serde_json::Value>>,
}

impl ExperimentConfig {
    /// Create new experiment config
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            warmup_runs: 3,
            measurement_runs: 10,
            parameters: HashMap::new(),
        }
    }
    
    /// Add parameter sweep
    pub fn add_parameter(mut self, name: impl Into<String>, values: Vec<serde_json::Value>) -> Self {
        self.parameters.insert(name.into(), values);
        self
    }
}

impl Statistics {
    /// Calculate statistics from measurements
    pub fn from_measurements(times_us: &[f64]) -> Self {
        let n = times_us.len();
        let mean = times_us.iter().sum::<f64>() / n as f64;
        
        let mut sorted = times_us.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };
        
        let variance = times_us.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / (n - 1) as f64;
        let std_dev = variance.sqrt();
        
        let min = sorted[0];
        let max = sorted[n - 1];
        
        // 95% confidence interval (t-distribution, df=n-1)
        let t_value = 2.262; // Approximate for n=10
        let margin = t_value * std_dev / (n as f64).sqrt();
        let confidence_interval_95 = (mean - margin, mean + margin);
        
        Self {
            mean_us: mean,
            median_us: median,
            std_dev_us: std_dev,
            min_us: min,
            max_us: max,
            sample_size: n,
            confidence_interval_95,
        }
    }
}

/// Experiment runner trait
pub trait Experiment {
    /// Run single iteration of experiment
    fn run(&self, parameters: &HashMap<String, serde_json::Value>) -> anyhow::Result<Measurements>;
    
    /// Get hardware info
    fn hardware_info(&self) -> HardwareInfo;
}

/// Run complete experiment with parameter sweep
pub fn run_experiment<E: Experiment>(
    experiment: &E,
    config: ExperimentConfig,
) -> anyhow::Result<Vec<ExperimentResult>> {
    let mut results = Vec::new();
    
    // Generate all parameter combinations
    let combinations = generate_combinations(&config.parameters);
    
    for params in combinations {
        println!("Running experiment {} with params: {:?}", config.id, params);
        
        // Warmup runs
        for _ in 0..config.warmup_runs {
            let _ = experiment.run(&params)?;
        }
        
        // Measurement runs
        let mut measurements_vec = Vec::new();
        for _ in 0..config.measurement_runs {
            let measurements = experiment.run(&params)?;
            measurements_vec.push(measurements);
        }
        
        // Aggregate measurements
        let all_times: Vec<f64> = measurements_vec.iter()
            .flat_map(|m| m.execution_times_us.iter().copied())
            .collect();
        
        let statistics = Statistics::from_measurements(&all_times);
        
        // Take first measurement's other fields
        let measurements = measurements_vec[0].clone();
        
        let result = ExperimentResult {
            experiment_id: config.id.clone(),
            hardware: experiment.hardware_info(),
            parameters: params.clone(),
            measurements: Measurements {
                execution_times_us: all_times,
                ..measurements
            },
            statistics,
            timestamp: chrono::Utc::now(),
        };
        
        results.push(result);
    }
    
    Ok(results)
}

/// Generate all parameter combinations (cartesian product)
fn generate_combinations(
    parameters: &HashMap<String, Vec<serde_json::Value>>,
) -> Vec<HashMap<String, serde_json::Value>> {
    if parameters.is_empty() {
        return vec![HashMap::new()];
    }
    
    let mut result = vec![HashMap::new()];
    
    for (key, values) in parameters {
        let mut new_result = Vec::new();
        for combo in &result {
            for value in values {
                let mut new_combo = combo.clone();
                new_combo.insert(key.clone(), value.clone());
                new_result.push(new_combo);
            }
        }
        result = new_result;
    }
    
    result
}

/// Save results to JSON file
pub fn save_results(
    results: &[ExperimentResult],
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load results from JSON file
pub fn load_results(
    path: impl AsRef<std::path::Path>,
) -> anyhow::Result<Vec<ExperimentResult>> {
    let json = std::fs::read_to_string(path)?;
    let results = serde_json::from_str(&json)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_statistics() {
        let times = vec![10.0, 12.0, 11.0, 13.0, 10.5, 11.5, 12.5, 10.0, 11.0, 12.0];
        let stats = Statistics::from_measurements(&times);
        
        assert!((stats.mean_us - 11.35).abs() < 0.1);
        assert_eq!(stats.sample_size, 10);
        assert!(stats.min_us <= stats.mean_us);
        assert!(stats.mean_us <= stats.max_us);
    }
    
    #[test]
    fn test_combinations() {
        let mut params = HashMap::new();
        params.insert("size".to_string(), vec![
            serde_json::json!(32),
            serde_json::json!(64),
        ]);
        params.insert("type".to_string(), vec![
            serde_json::json!("A"),
            serde_json::json!("B"),
        ]);
        
        let combos = generate_combinations(&params);
        assert_eq!(combos.len(), 4); // 2 × 2
    }
}
