//! DIGAMMA F64 - Digamma function ψ(x) = Γ'(x)/Γ(x) - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Note: Uses CPU fallback as most GPUs don't support f64 log/sin/cos.
//!
//! Applications:
//! - Fisher information
//! - Bayesian statistics
//! - Neural network regularization

use crate::device::WgpuDevice;
use crate::error::Result;
use std::sync::Arc;

/// f64 Digamma function evaluator
///
/// Computes ψ(x) = d/dx ln(Γ(x)) using reflection + recurrence + asymptotic expansion.
pub struct DigammaF64 {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
}

impl DigammaF64 {
    /// Create new Digamma f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    #[allow(dead_code)]
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/digamma_f64.wgsl")
    }

    /// Compute ψ(x) for each element
    ///
    /// # Arguments
    /// * `x` - Input values
    ///
    /// # Returns
    /// Vector of ψ(x) values with f64 precision
    pub fn digamma(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        // CPU fallback - f64 log/sin/cos not supported on most GPUs
        Ok(self.digamma_cpu(x))
    }

    fn digamma_cpu(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| Self::digamma_scalar(xi)).collect()
    }

    fn digamma_scalar(x: f64) -> f64 {
        use std::f64::consts::PI;

        // Non-positive integer: pole
        if x <= 0.0 && x == x.floor() {
            return f64::NAN;
        }

        let mut y = x;
        let mut result = 0.0;

        // Reflection formula for x < 0
        if y < 0.0 {
            let cot_pi_y = (PI * y).cos() / (PI * y).sin();
            result -= PI * cot_pi_y;
            y = 1.0 - y;
        }

        // Recurrence to shift to larger argument
        while y < 6.0 {
            result -= 1.0 / y;
            y += 1.0;
        }

        // Asymptotic expansion for y >= 6
        result + Self::digamma_asymptotic(y)
    }

    fn digamma_asymptotic(x: f64) -> f64 {
        let inv_x = 1.0 / x;
        let inv_x2 = inv_x * inv_x;

        // Bernoulli number coefficients
        const B2: f64 = 1.0 / 12.0;
        const B4: f64 = -1.0 / 120.0;
        const B6: f64 = 1.0 / 252.0;
        const B8: f64 = -1.0 / 240.0;
        const B10: f64 = 1.0 / 132.0;
        const B12: f64 = -691.0 / 32760.0;

        let mut sum = x.ln() - 0.5 * inv_x;
        let mut term = inv_x2;

        sum -= B2 * term;
        term *= inv_x2;
        sum -= B4 * term;
        term *= inv_x2;
        sum -= B6 * term;
        term *= inv_x2;
        sum -= B8 * term;
        term *= inv_x2;
        sum -= B10 * term;
        term *= inv_x2;
        sum -= B12 * term;

        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_digamma_at_1() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(1) = -γ (Euler-Mascheroni constant)
        let euler_mascheroni = 0.5772156649015329;
        let result = digamma.digamma(&[1.0]).unwrap();

        assert!(
            (result[0] + euler_mascheroni).abs() < 1e-10,
            "ψ(1) = {}, expected -γ = {}",
            result[0],
            -euler_mascheroni
        );
    }

    #[tokio::test]
    async fn test_digamma_recurrence() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(x+1) = ψ(x) + 1/x
        for x in [1.0, 2.0, 3.0, 4.5, 7.3] {
            let result = digamma.digamma(&[x, x + 1.0]).unwrap();
            let psi_x = result[0];
            let psi_x1 = result[1];

            assert!(
                (psi_x1 - psi_x - 1.0 / x).abs() < 1e-10,
                "ψ({}) + 1/{} = {} should equal ψ({}) = {}",
                x,
                x,
                psi_x + 1.0 / x,
                x + 1.0,
                psi_x1
            );
        }
    }

    #[tokio::test]
    async fn test_digamma_known_values() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(2) = 1 - γ
        let euler_mascheroni = 0.5772156649015329;
        let result = digamma.digamma(&[2.0]).unwrap();
        let expected = 1.0 - euler_mascheroni;

        assert!(
            (result[0] - expected).abs() < 1e-10,
            "ψ(2) = {}, expected {}",
            result[0],
            expected
        );

        // ψ(1/2) = -γ - 2*ln(2)
        let result = digamma.digamma(&[0.5]).unwrap();
        let expected = -euler_mascheroni - 2.0 * 2.0_f64.ln();

        assert!(
            (result[0] - expected).abs() < 1e-10,
            "ψ(0.5) = {}, expected {}",
            result[0],
            expected
        );
    }

    #[tokio::test]
    async fn test_digamma_large_x() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // For large x, ψ(x) ≈ ln(x) - 1/(2x)
        let x = 100.0;
        let result = digamma.digamma(&[x]).unwrap();
        let approx = x.ln() - 0.5 / x;

        // The actual value is more accurate than the simple approximation
        // Asymptotic expansion includes higher order terms that improve accuracy
        assert!(
            (result[0] - approx).abs() < 1e-4,
            "ψ({}) = {}, asymptotic approx = {}",
            x,
            result[0],
            approx
        );
    }
}
