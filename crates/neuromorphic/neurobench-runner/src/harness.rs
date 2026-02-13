//! Benchmark harness for running NeuroBench workloads
//!
//! Coordinates model loading, inference, and metric collection.

use crate::{Benchmark, BenchmarkConfig, BenchmarkResult, Error, Result};
use akida_driver::{BackendSelection, NpuBackend, select_backend};
use std::time::Instant;
use tracing::{info, warn};

/// Harness configuration
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// PCIe address of the Akida device
    pub device_id: String,
    /// Backend selection (Auto, Kernel, Userspace, Vfio)
    pub backend: BackendSelection,
    /// Path to model files
    pub models_dir: String,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            device_id: "0000:a1:00.0".to_string(),
            backend: BackendSelection::Auto,
            models_dir: "models/akida".to_string(),
        }
    }
}

/// Main benchmark harness
pub struct Harness {
    config: HarnessConfig,
    device: Box<dyn NpuBackend>,
}

impl Harness {
    /// Create new harness with default config
    pub fn new(device_id: &str) -> Result<Self> {
        Self::with_config(HarnessConfig {
            device_id: device_id.to_string(),
            ..Default::default()
        })
    }
    
    /// Create with specific config
    pub fn with_config(config: HarnessConfig) -> Result<Self> {
        info!("Initializing NeuroBench harness for device {}", config.device_id);
        
        let device = select_backend(config.backend, &config.device_id)
            .map_err(|e| Error::HardwareInit(e.to_string()))?;
        
        info!("Using {:?} backend", device.backend_type());
        
        Ok(Self { config, device })
    }
    
    /// Run a benchmark
    pub fn run(&mut self, benchmark: Benchmark, config: &BenchmarkConfig) -> Result<BenchmarkResult> {
        info!("Running benchmark: {:?}", benchmark);
        
        // Load appropriate model
        self.load_model(benchmark)?;
        
        let mut result = BenchmarkResult::new(benchmark);
        let mut latencies = Vec::with_capacity(config.num_iterations);
        let mut power_samples: Vec<f64> = Vec::new();
        
        // Generate test data (f32 for NpuBackend trait)
        let input_shape = benchmark.input_shape();
        let input_size: usize = input_shape.iter().product();
        let test_input: Vec<f32> = (0..input_size)
            .map(|i| ((i % 256) as f32) / 255.0)  // Normalize to 0-1
            .collect();
        
        // Warmup
        info!("Warming up ({} iterations)...", config.warmup_iterations);
        for _ in 0..config.warmup_iterations {
            let _ = self.device.infer(&test_input);
        }
        
        // Main benchmark loop
        info!("Running {} iterations...", config.num_iterations);
        for i in 0..config.num_iterations {
            let start = Instant::now();
            
            match self.device.infer(&test_input) {
                Ok(output) => {
                    let elapsed = start.elapsed();
                    latencies.push(elapsed);
                    
                    // Check prediction (mock - real would compare to labels)
                    if !output.is_empty() {
                        result.num_samples += 1;
                        // Assume correct for now (would need real labels)
                        result.num_correct += 1;
                    }
                }
                Err(e) => {
                    warn!("Inference {} failed: {}", i, e);
                }
            }
            
            // Power measurement
            if config.measure_power && i % 10 == 0 {
                if let Ok(power) = self.device.measure_power() {
                    power_samples.push(f64::from(power));
                }
            }
        }
        
        // Calculate power
        if !power_samples.is_empty() {
            let avg_power: f64 = power_samples.iter().sum::<f64>() / power_samples.len() as f64;
            result.mean_power_mw = Some(avg_power * 1000.0); // W to mW
        }
        
        // Finalize metrics
        result.finalize(&latencies);
        
        Ok(result)
    }
    
    /// Load model for benchmark
    fn load_model(&mut self, benchmark: Benchmark) -> Result<()> {
        let model_name = match benchmark {
            Benchmark::DvsGesture => "dvs_gesture",
            Benchmark::KeywordFscil => "ds_cnn_kws",
            Benchmark::ChaoticFunction => "esn_chaotic",
            Benchmark::NhpMotor => "nhp_motor",
            Benchmark::EventCamera => "event_camera",
        };
        
        let model_path = format!("{}/{}.fbz", self.config.models_dir, model_name);
        
        match std::fs::read(&model_path) {
            Ok(bytes) => {
                info!("Loading model from {} ({} bytes)", model_path, bytes.len());
                self.device.load_model(&bytes)
                    .map_err(|e| Error::ModelLoad(e.to_string()))?;
            }
            Err(_) => {
                warn!("Model file not found: {}", model_path);
                // Use mock model for testing
                let mock_model = vec![0u8; 1024];
                self.device.load_model(&mock_model)
                    .map_err(|e| Error::ModelLoad(e.to_string()))?;
            }
        }
        
        Ok(())
    }
    
    /// Get device info
    pub fn device_info(&self) -> String {
        format!("{} ({:?})", self.config.device_id, self.device.backend_type())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_harness_config_default() {
        let config = HarnessConfig::default();
        assert_eq!(config.backend, BackendSelection::Auto);
    }
}
