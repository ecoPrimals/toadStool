// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark harness for running `NeuroBench` workloads
//!
//! Coordinates model loading, dataset loading, inference, and metric collection.
//!
//! ## Usage
//!
//! ```ignore
//! use neurobench_runner::{Harness, HarnessConfig, Benchmark, BenchmarkConfig};
//!
//! let config = HarnessConfig {
//!     device_id: "0000:a1:00.0".to_string(),
//!     data_dir: "data/neurobench".to_string(),
//!     ..Default::default()
//! };
//! let mut harness = Harness::with_config(config)?;
//! let result = harness.run(Benchmark::DvsGesture, &BenchmarkConfig::default())?;
//! result.print_summary();
//! ```

use crate::data::{Dataset, Sample};
use crate::{Benchmark, BenchmarkConfig, BenchmarkResult, Error, Result};
#[cfg(target_os = "linux")]
use akida_driver::select_backend;
use akida_driver::{BackendSelection, NpuBackend};
use std::time::Instant;
use tracing::{debug, info, warn};

/// Harness configuration
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    /// `PCIe` address of the Akida device
    pub device_id: String,
    /// Backend selection (Auto, Kernel, Userspace, Vfio)
    pub backend: BackendSelection,
    /// Path to model files
    pub models_dir: String,
    /// Path to dataset files
    pub data_dir: String,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            device_id: "0000:a1:00.0".to_string(),
            backend: BackendSelection::Auto,
            models_dir: "models/akida".to_string(),
            data_dir: "data/neurobench".to_string(),
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
    ///
    /// # Errors
    ///
    /// Returns `Err` if the device cannot be opened or the backend fails to initialize.
    pub fn new(device_id: &str) -> Result<Self> {
        Self::with_config(HarnessConfig {
            device_id: device_id.to_string(),
            ..Default::default()
        })
    }

    /// Create with specific config
    ///
    /// # Errors
    ///
    /// Returns `Err` if the device cannot be opened or the backend fails to initialize.
    pub fn with_config(config: HarnessConfig) -> Result<Self> {
        info!(
            "Initializing NeuroBench harness for device {}",
            config.device_id
        );

        #[cfg(not(target_os = "linux"))]
        {
            let _ = &config;
            return Err(Error::HardwareInit(
                "NeuroBench harness requires Unix (NPU hardware access)".into(),
            ));
        }

        #[cfg(target_os = "linux")]
        {
            let device = select_backend(config.backend, &config.device_id)
                .map_err(|e| Error::HardwareInit(e.to_string()))?;

            info!("Using {:?} backend", device.backend_type());

            Ok(Self { config, device })
        }
    }

    /// Run a benchmark with real dataset
    ///
    /// # Errors
    ///
    /// Returns `Err` if model loading, dataset loading, or inference fails.
    pub fn run(
        &mut self,
        benchmark: Benchmark,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResult> {
        info!("Running benchmark: {:?}", benchmark);

        // Load appropriate model
        self.load_model(benchmark)?;

        // Load dataset
        let dataset_path =
            std::path::Path::new(&self.config.data_dir).join(benchmark_data_dir(benchmark));
        let dataset = Dataset::load(benchmark, &dataset_path)?;

        info!(
            "Loaded {} samples from {}",
            dataset.len(),
            dataset_path.display()
        );

        let mut result = BenchmarkResult::new(benchmark);
        let mut latencies = Vec::with_capacity(config.num_iterations);
        let mut power_samples: Vec<f64> = Vec::new();

        // Warmup with first sample
        if let Some(warmup_sample) = dataset.samples.first() {
            info!("Warming up ({} iterations)...", config.warmup_iterations);
            let warmup_input = sample_to_f32(warmup_sample, benchmark);
            for _ in 0..config.warmup_iterations {
                let _ = self.device.infer(&warmup_input);
            }
        }

        // Main benchmark loop - iterate over dataset
        let samples_to_use = config.num_iterations.min(dataset.len());
        info!(
            "Running {} iterations ({} samples)...",
            samples_to_use,
            dataset.len()
        );

        for (i, sample) in dataset.samples.iter().take(samples_to_use).enumerate() {
            let input = sample_to_f32(sample, benchmark);
            let start = Instant::now();

            match self.device.infer(&input) {
                Ok(output) => {
                    let elapsed = start.elapsed();
                    latencies.push(elapsed);

                    result.num_samples += 1;

                    // Check prediction against ground truth
                    if let Some(predicted_class) = get_predicted_class(&output) {
                        if predicted_class == sample.label {
                            result.num_correct += 1;
                        }
                        debug!(
                            "Sample {}: predicted={}, actual={}, correct={}",
                            i,
                            predicted_class,
                            sample.label,
                            predicted_class == sample.label
                        );
                    }
                }
                Err(e) => {
                    warn!("Inference {} failed: {}", i, e);
                }
            }

            // Power measurement (every 10 samples)
            if config.measure_power
                && i % 10 == 0
                && let Ok(power) = self.device.measure_power()
            {
                power_samples.push(f64::from(power));
            }
        }

        // Calculate power
        if !power_samples.is_empty() {
            let len = u32::try_from(power_samples.len()).unwrap_or(1);
            let avg_power: f64 = power_samples.iter().sum::<f64>() / f64::from(len);
            result.mean_power_mw = Some(avg_power * 1000.0); // W to mW
        }

        // Finalize metrics
        result.finalize(&latencies);

        Ok(result)
    }

    /// Run benchmark with synthetic data (for hardware validation without datasets)
    ///
    /// # Errors
    ///
    /// Returns `Err` if model loading or inference fails.
    pub fn run_synthetic(
        &mut self,
        benchmark: Benchmark,
        config: &BenchmarkConfig,
    ) -> Result<BenchmarkResult> {
        info!("Running benchmark with synthetic data: {:?}", benchmark);

        // Load model
        self.load_model(benchmark)?;

        let mut result = BenchmarkResult::new(benchmark);
        let mut latencies = Vec::with_capacity(config.num_iterations);
        let mut power_samples: Vec<f64> = Vec::new();

        // Generate synthetic input
        let input_shape = benchmark.input_shape();
        let input_size: usize = input_shape.iter().product();
        let test_input: Vec<f32> = (0..input_size)
            .map(|i| f32::from(u8::try_from(i % 256).unwrap_or(0)) / 255.0)
            .collect();

        // Warmup
        info!("Warming up ({} iterations)...", config.warmup_iterations);
        for _ in 0..config.warmup_iterations {
            let _ = self.device.infer(&test_input);
        }

        // Main loop
        info!(
            "Running {} iterations (synthetic)...",
            config.num_iterations
        );
        for i in 0..config.num_iterations {
            let start = Instant::now();

            match self.device.infer(&test_input) {
                Ok(output) => {
                    let elapsed = start.elapsed();
                    latencies.push(elapsed);

                    if !output.is_empty() {
                        result.num_samples += 1;
                        // Synthetic data - accuracy is not meaningful
                        result.num_correct += 1;
                    }
                }
                Err(e) => {
                    warn!("Inference {} failed: {}", i, e);
                }
            }

            if config.measure_power
                && i % 10 == 0
                && let Ok(power) = self.device.measure_power()
            {
                power_samples.push(f64::from(power));
            }
        }

        if !power_samples.is_empty() {
            let len = u32::try_from(power_samples.len()).unwrap_or(1);
            let avg_power: f64 = power_samples.iter().sum::<f64>() / f64::from(len);
            result.mean_power_mw = Some(avg_power * 1000.0);
        }

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
                self.device
                    .load_model(&bytes)
                    .map_err(|e| Error::ModelLoad(e.to_string()))?;
            }
            Err(e) => {
                // Deep Debt: Don't use mock model - fail clearly when model file is missing
                return Err(Error::ModelLoad(format!(
                    "Model file not found: {model_path} ({e}). NeuroBench requires a valid model file."
                )));
            }
        }

        Ok(())
    }

    /// Get device info
    #[must_use]
    pub fn device_info(&self) -> String {
        format!(
            "{} ({:?})",
            self.config.device_id,
            self.device.backend_type()
        )
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get data directory name for benchmark
const fn benchmark_data_dir(benchmark: Benchmark) -> &'static str {
    match benchmark {
        Benchmark::DvsGesture => "dvs_gesture",
        Benchmark::KeywordFscil => "keyword_fscil",
        Benchmark::ChaoticFunction => "chaotic",
        Benchmark::NhpMotor => "nhp_motor",
        Benchmark::EventCamera => "event_camera",
    }
}

/// Convert sample to f32 vector for inference
fn sample_to_f32(sample: &Sample, benchmark: Benchmark) -> Vec<f32> {
    // If sample is already f32 encoded (4 bytes per value)
    if sample.input.len().is_multiple_of(4) {
        return sample.as_f32();
    }

    // Otherwise, treat as u8 and normalize to 0-1
    let input_shape = benchmark.input_shape();
    let expected_size: usize = input_shape.iter().product();

    sample
        .input
        .iter()
        .take(expected_size)
        .map(|&b| f32::from(b) / 255.0)
        .collect()
}

/// Get predicted class from model output
fn get_predicted_class(output: &[f32]) -> Option<usize> {
    if output.is_empty() {
        return None;
    }

    // Find argmax (index of maximum value)
    output
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_config_default() {
        let config = HarnessConfig::default();
        assert_eq!(config.backend, BackendSelection::Auto);
        assert!(!config.data_dir.is_empty());
        assert_eq!(config.device_id, "0000:a1:00.0");
        assert!(!config.models_dir.is_empty());
    }

    #[test]
    fn test_harness_config_clone() {
        let config = HarnessConfig {
            device_id: "0000:01:00.0".to_string(),
            backend: BackendSelection::Kernel,
            models_dir: "/models".to_string(),
            data_dir: "/data".to_string(),
        };
        let cloned = config.clone();
        assert_eq!(cloned.device_id, config.device_id);
        assert_eq!(cloned.backend, BackendSelection::Kernel);
    }

    #[test]
    fn test_get_predicted_class() {
        let output = vec![0.1, 0.2, 0.9, 0.3];
        assert_eq!(get_predicted_class(&output), Some(2));

        let empty: Vec<f32> = vec![];
        assert_eq!(get_predicted_class(&empty), None);

        let single = vec![1.0];
        assert_eq!(get_predicted_class(&single), Some(0));

        let ties = vec![0.5, 0.5, 0.5];
        let tied_result = get_predicted_class(&ties);
        assert!(tied_result.is_some());
        assert!(tied_result.unwrap() < 3);
    }

    #[test]
    fn test_benchmark_data_dir() {
        assert_eq!(benchmark_data_dir(Benchmark::DvsGesture), "dvs_gesture");
        assert_eq!(benchmark_data_dir(Benchmark::KeywordFscil), "keyword_fscil");
        assert_eq!(benchmark_data_dir(Benchmark::ChaoticFunction), "chaotic");
        assert_eq!(benchmark_data_dir(Benchmark::NhpMotor), "nhp_motor");
        assert_eq!(benchmark_data_dir(Benchmark::EventCamera), "event_camera");
    }

    #[test]
    fn test_sample_to_f32_u8_path() {
        let sample = Sample {
            input: vec![0, 128, 255],
            label: 0,
            id: None,
        };
        let result = sample_to_f32(&sample, Benchmark::DvsGesture);
        assert!(!result.is_empty());
        assert!((result[0] - 0.0).abs() < 0.01);
        assert!((result[1] - 0.5).abs() < 0.01);
        assert!((result[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_sample_to_f32_f32_path() {
        let data = [0.25_f32, 0.5, 0.75];
        let sample = Sample::from_f32(&data, 0, None);
        let result = sample_to_f32(&sample, Benchmark::DvsGesture);
        assert_eq!(result.len(), 3);
        assert!((result[0] - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn test_benchmark_result_new_and_finalize() {
        let mut result = BenchmarkResult::new(Benchmark::DvsGesture);
        assert_eq!(result.benchmark, Benchmark::DvsGesture);
        assert_eq!(result.num_samples, 0);
        result.num_samples = 10;
        result.num_correct = 8;
        result.finalize(&[
            std::time::Duration::from_micros(100),
            std::time::Duration::from_micros(110),
            std::time::Duration::from_micros(120),
        ]);
        assert!(result.accuracy > 0.7);
        assert!(result.throughput > 0.0);
    }

    #[test]
    fn test_benchmark_result_finalize_empty() {
        let mut result = BenchmarkResult::new(Benchmark::KeywordFscil);
        result.finalize(&[]);
        assert_eq!(result.mean_latency, std::time::Duration::ZERO);
    }
}
