//! BETA F64 - Beta function B(a,b) = Γ(a)Γ(b)/Γ(a+b) - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Note: Uses CPU fallback as most GPUs don't support f64 log/exp.
//!
//! Applications:
//! - Beta distributions
//! - Bayesian statistics
//! - Binomial coefficients
//! - ML/statistics

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Beta function evaluator
///
/// Computes B(a,b) = Γ(a)Γ(b)/Γ(a+b) using log-gamma for stability.
pub struct BetaF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl BetaF64 {
    /// Create new Beta f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/beta_f64.wgsl")
    }

    /// Compute B(a,b) for each pair
    ///
    /// # Arguments
    /// * `pairs` - Input pairs as interleaved [a₀, b₀, a₁, b₁, ...]
    ///
    /// # Returns
    /// Vector of B(aᵢ, bᵢ) values with f64 precision
    pub fn beta(&self, pairs: &[f64]) -> Result<Vec<f64>> {
        if pairs.is_empty() || pairs.len() % 2 != 0 {
            return Ok(vec![]);
        }

        // CPU fallback - f64 log/exp not supported on most GPUs
        Ok(self.beta_cpu(pairs))
    }

    fn beta_cpu(&self, pairs: &[f64]) -> Vec<f64> {
        pairs
            .chunks(2)
            .map(|chunk| Self::beta_scalar(chunk[0], chunk[1]))
            .collect()
    }

    fn beta_scalar(a: f64, b: f64) -> f64 {
        if a <= 0.0 || b <= 0.0 {
            return f64::NAN;
        }
        // B(a,b) = exp(lgamma(a) + lgamma(b) - lgamma(a+b))
        use std::f64::consts::PI;

        fn lgamma(x: f64) -> f64 {
            if x <= 0.0 {
                return f64::NAN;
            }
            if x < 0.5 {
                return (PI / (PI * x).sin()).ln() - lgamma(1.0 - x);
            }
            // Lanczos approximation
            let g = 7.0;
            let x_shifted = x - 1.0;
            let mut sum = 0.99999999999980993;
            let coeffs = [
                676.5203681218851,
                -1259.1392167224028,
                771.32342877765313,
                -176.61502916214059,
                12.507343278686905,
                -0.13857109526572012,
                9.9843695780195716e-6,
                1.5056327351493116e-7,
            ];
            for (i, &c) in coeffs.iter().enumerate() {
                sum += c / (x_shifted + (i + 1) as f64);
            }
            let t = x_shifted + g + 0.5;
            let sqrt_2pi: f64 = 2.5066282746310005;
            sqrt_2pi.ln() + sum.ln() + (x_shifted + 0.5) * t.ln() - t
        }

        let log_beta = lgamma(a) + lgamma(b) - lgamma(a + b);
        log_beta.exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_beta_symmetric() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let beta = BetaF64::new(device).unwrap();

        // B(a,b) = B(b,a)
        let pairs = vec![2.0, 3.0, 3.0, 2.0];
        let result = beta.beta(&pairs).unwrap();

        assert!(
            (result[0] - result[1]).abs() < 1e-10,
            "B(2,3) = {} should equal B(3,2) = {}",
            result[0],
            result[1]
        );
    }

    #[tokio::test]
    async fn test_beta_known_values() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let beta = BetaF64::new(device).unwrap();

        // B(1,1) = 1
        let pairs = vec![1.0, 1.0];
        let result = beta.beta(&pairs).unwrap();
        assert!(
            (result[0] - 1.0).abs() < 1e-10,
            "B(1,1) = {}, expected 1.0",
            result[0]
        );

        // B(2,2) = 1/6
        let pairs = vec![2.0, 2.0];
        let result = beta.beta(&pairs).unwrap();
        let expected = 1.0 / 6.0;
        assert!(
            (result[0] - expected).abs() < 1e-10,
            "B(2,2) = {}, expected {}",
            result[0],
            expected
        );

        // B(3,3) = 1/30
        let pairs = vec![3.0, 3.0];
        let result = beta.beta(&pairs).unwrap();
        let expected = 1.0 / 30.0;
        assert!(
            (result[0] - expected).abs() < 1e-10,
            "B(3,3) = {}, expected {}",
            result[0],
            expected
        );
    }

    #[tokio::test]
    async fn test_beta_relation_to_gamma() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let beta = BetaF64::new(device).unwrap();

        // B(n, 1) = 1/n for positive integer n
        for n in 1..=5 {
            let pairs = vec![n as f64, 1.0];
            let result = beta.beta(&pairs).unwrap();
            let expected = 1.0 / n as f64;
            assert!(
                (result[0] - expected).abs() < 1e-10,
                "B({}, 1) = {}, expected {}",
                n,
                result[0],
                expected
            );
        }
    }
}
