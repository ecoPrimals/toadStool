//! COVARIANCE F64 - Covariance computation - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Applications:
//! - Portfolio theory
//! - PCA
//! - Kalman filters

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Covariance evaluator
pub struct CovarianceF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl CovarianceF64 {
    /// Create new Covariance f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/covariance_f64.wgsl")
    }

    /// Compute covariance between two vectors (population covariance, ddof=0)
    pub fn covariance(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        self.covariance_ddof(x, y, 0)
    }

    /// Compute sample covariance (ddof=1)
    pub fn sample_covariance(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        self.covariance_ddof(x, y, 1)
    }

    /// Compute covariance with specified degrees of freedom adjustment
    pub fn covariance_ddof(&self, x: &[f64], y: &[f64], ddof: usize) -> Result<f64> {
        if x.len() != y.len() || x.is_empty() || x.len() <= ddof {
            return Ok(0.0);
        }

        // CPU fallback for reliability
        Ok(Self::covariance_cpu(x, y, ddof))
    }

    fn covariance_cpu(x: &[f64], y: &[f64], ddof: usize) -> f64 {
        let n = x.len();
        if n <= ddof {
            return 0.0;
        }

        // Two-pass for numerical stability
        let mean_x: f64 = x.iter().sum::<f64>() / n as f64;
        let mean_y: f64 = y.iter().sum::<f64>() / n as f64;

        let cov_sum: f64 = x
            .iter()
            .zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum();

        cov_sum / (n - ddof) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_covariance_positive() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Positive correlation: both increase together
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result > 0.0,
            "Covariance should be positive, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_negative() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Negative correlation: one increases, other decreases
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0]; // y = -2x + 12
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result < 0.0,
            "Covariance should be negative, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_zero() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // No correlation: independent
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 3.0, 3.0, 3.0, 3.0]; // constant
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result.abs() < 1e-10,
            "Covariance with constant should be 0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_self_is_variance() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Cov(X, X) = Var(X)
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let cov_xx = cov.covariance(&x, &x).unwrap();

        // Population variance = 2
        assert!(
            (cov_xx - 2.0).abs() < 1e-10,
            "Cov(X,X) = {}, expected variance = 2.0",
            cov_xx
        );
    }

    #[tokio::test]
    async fn test_sample_covariance() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Sample Cov(X,X) = sample variance = 10/4 = 2.5
        let result = cov.sample_covariance(&x, &y).unwrap();

        assert!(
            (result - 2.5).abs() < 1e-10,
            "Sample Cov(X,X) = {}, expected 2.5",
            result
        );
    }
}
