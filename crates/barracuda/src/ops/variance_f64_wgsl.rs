//! VARIANCE F64 - Variance and standard deviation - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Applications:
//! - Statistics
//! - Normalization
//! - Feature scaling

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// Simple variance reduction variant (scalar path).
pub const WGSL_VARIANCE_SIMPLE: &str = include_str!("../shaders/misc/variance_simple.wgsl");

/// f64 Variance/StdDev evaluator
pub struct VarianceF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl VarianceF64 {
    /// Create new Variance f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/variance_f64.wgsl")
    }

    /// Compute variance of a vector (population variance, ddof=0)
    pub fn variance(&self, data: &[f64]) -> Result<f64> {
        self.variance_ddof(data, 0)
    }

    /// Compute sample variance (ddof=1)
    pub fn sample_variance(&self, data: &[f64]) -> Result<f64> {
        self.variance_ddof(data, 1)
    }

    /// Compute variance with specified degrees of freedom adjustment
    pub fn variance_ddof(&self, data: &[f64], ddof: usize) -> Result<f64> {
        if data.is_empty() || data.len() <= ddof {
            return Ok(0.0);
        }

        // CPU fallback for reliability
        Ok(Self::variance_cpu(data, ddof))
    }

    /// Compute standard deviation (population, ddof=0)
    pub fn std_dev(&self, data: &[f64]) -> Result<f64> {
        Ok(self.variance(data)?.sqrt())
    }

    /// Compute sample standard deviation (ddof=1)
    pub fn sample_std_dev(&self, data: &[f64]) -> Result<f64> {
        Ok(self.sample_variance(data)?.sqrt())
    }

    fn variance_cpu(data: &[f64], ddof: usize) -> f64 {
        let n = data.len();
        if n <= ddof {
            return 0.0;
        }

        // Two-pass for numerical stability
        let mean: f64 = data.iter().sum::<f64>() / n as f64;
        let var_sum: f64 = data.iter().map(|x| (x - mean).powi(2)).sum();

        var_sum / (n - ddof) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_variance_simple() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let var = VarianceF64::new(device).unwrap();

        // Variance of [1, 2, 3, 4, 5] with mean=3
        // Σ(x-μ)² = 4+1+0+1+4 = 10
        // Population variance = 10/5 = 2
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = var.variance(&data).unwrap();

        assert!(
            (result - 2.0).abs() < 1e-10,
            "Variance = {}, expected 2.0",
            result
        );
    }

    #[tokio::test]
    async fn test_sample_variance() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let var = VarianceF64::new(device).unwrap();

        // Sample variance = 10/4 = 2.5
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = var.sample_variance(&data).unwrap();

        assert!(
            (result - 2.5).abs() < 1e-10,
            "Sample variance = {}, expected 2.5",
            result
        );
    }

    #[tokio::test]
    async fn test_std_dev() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let var = VarianceF64::new(device).unwrap();

        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = var.std_dev(&data).unwrap();
        let expected = 2.0_f64.sqrt();

        assert!(
            (result - expected).abs() < 1e-10,
            "Std dev = {}, expected {}",
            result,
            expected
        );
    }

    #[tokio::test]
    async fn test_variance_constant() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let var = VarianceF64::new(device).unwrap();

        // Variance of constant array is 0
        let data = vec![5.0; 100];
        let result = var.variance(&data).unwrap();

        assert!(
            result.abs() < 1e-10,
            "Variance of constant = {}, expected 0.0",
            result
        );
    }
}
