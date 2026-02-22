//! Gamma and Incomplete Gamma Functions
//!
//! Implementations of gamma-related special functions commonly used in
//! statistics, physics, and scientific computing.
//!
//! # Functions
//!
//! - `gamma(x)` - Complete gamma function Γ(x)
//! - `ln_gamma(x)` - Natural log of gamma function, ln(Γ(x))
//! - `lower_incomplete_gamma(a, x)` - Lower incomplete gamma γ(a, x)
//! - `upper_incomplete_gamma(a, x)` - Upper incomplete gamma Γ(a, x)
//! - `regularized_gamma_p(a, x)` - P(a, x) = γ(a, x) / Γ(a)
//! - `regularized_gamma_q(a, x)` - Q(a, x) = Γ(a, x) / Γ(a) = 1 - P(a, x)
//!
//! # Applications
//!
//! - Chi-squared distribution CDF
//! - Poisson distribution
//! - Nuclear physics (shell model)
//! - Statistical hypothesis testing
//!
//! # References
//!
//! - Numerical Recipes, 3rd Edition, Chapter 6
//! - Abramowitz & Stegun, Chapter 6
//! - NIST Digital Library of Mathematical Functions, Chapter 8

use crate::error::{BarracudaError, Result};

/// Lanczos coefficients for gamma function (g=7)
///
/// These are high-precision constants from Numerical Recipes - intentionally exact.
#[allow(clippy::excessive_precision)]
const LANCZOS_G: f64 = 7.0;
#[allow(clippy::excessive_precision)]
const LANCZOS_COEFFS: [f64; 9] = [
    0.99999999999980993,
    676.5203681218851,
    -1259.1392167224028,
    771.32342877765313,
    -176.61502916214059,
    12.507343278686905,
    -0.13857109526572012,
    9.9843695780195716e-6,
    1.5056327351493116e-7,
];

/// Natural logarithm of the gamma function: ln(Γ(x))
///
/// Uses the Lanczos approximation for high accuracy.
///
/// # Arguments
///
/// * `x` - Input value (x > 0)
///
/// # Returns
///
/// ln(Γ(x))
///
/// # Example
///
/// ```
/// use barracuda::special::ln_gamma;
///
/// // ln(Γ(1)) = 0
/// assert!((ln_gamma(1.0).unwrap() - 0.0).abs() < 1e-10);
///
/// // ln(Γ(5)) = ln(4!) = ln(24)
/// assert!((ln_gamma(5.0).unwrap() - 24.0_f64.ln()).abs() < 1e-10);
/// ```
pub fn ln_gamma(x: f64) -> Result<f64> {
    if x <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("ln_gamma requires x > 0, got {}", x),
        });
    }

    if x < 0.5 {
        // Use reflection formula: Γ(z)Γ(1-z) = π/sin(πz)
        let pi = std::f64::consts::PI;
        Ok(pi.ln() - (pi * x).sin().ln() - ln_gamma(1.0 - x)?)
    } else {
        let x = x - 1.0;
        let mut ag = LANCZOS_COEFFS[0];
        for (i, &c) in LANCZOS_COEFFS.iter().enumerate().skip(1) {
            ag += c / (x + i as f64);
        }

        let tmp = x + LANCZOS_G + 0.5;
        Ok((2.0 * std::f64::consts::PI).sqrt().ln() + (x + 0.5) * tmp.ln() - tmp + ag.ln())
    }
}

/// Complete gamma function: Γ(x)
///
/// # Arguments
///
/// * `x` - Input value (x > 0 or negative non-integer)
///
/// # Example
///
/// ```
/// use barracuda::special::gamma;
///
/// // Γ(5) = 4! = 24
/// assert!((gamma(5.0).unwrap() - 24.0).abs() < 1e-10);
///
/// // Γ(0.5) = √π
/// assert!((gamma(0.5).unwrap() - std::f64::consts::PI.sqrt()).abs() < 1e-10);
/// ```
pub fn gamma(x: f64) -> Result<f64> {
    if x <= 0.0 && x.fract() == 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("gamma is undefined for non-positive integers, got {}", x),
        });
    }

    Ok(ln_gamma(x)?.exp())
}

#[allow(dead_code)]
const WGSL_INCOMPLETE_GAMMA_F64: &str = include_str!("../shaders/special/incomplete_gamma_f64.wgsl");

/// Lower incomplete gamma function: γ(a, x) = ∫₀ˣ t^(a-1) e^(-t) dt
///
/// Uses series expansion for x < a+1 and continued fraction for x >= a+1.
///
/// # Arguments
///
/// * `a` - Shape parameter (a > 0)
/// * `x` - Upper limit of integration (x >= 0)
///
/// # Example
///
/// ```
/// use barracuda::special::lower_incomplete_gamma;
///
/// let (gamma_val, _) = lower_incomplete_gamma(2.0, 1.0)?;
/// // γ(2, 1) ≈ 0.2642
/// assert!((gamma_val - 0.2642).abs() < 0.01);
/// # Ok::<(), barracuda::error::BarracudaError>(())
/// ```
pub fn lower_incomplete_gamma(a: f64, x: f64) -> Result<(f64, f64)> {
    if a <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("lower_incomplete_gamma requires a > 0, got {}", a),
        });
    }
    if x < 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("lower_incomplete_gamma requires x >= 0, got {}", x),
        });
    }

    if x == 0.0 {
        return Ok((0.0, gamma(a)?));
    }

    let gln = ln_gamma(a)?;
    let gamma_complete = gln.exp();

    if x < a + 1.0 {
        // Series expansion
        let p = gamma_series(a, x, gln)?;
        Ok((p * gamma_complete, gamma_complete))
    } else {
        // Continued fraction
        let q = gamma_cf(a, x, gln)?;
        Ok(((1.0 - q) * gamma_complete, gamma_complete))
    }
}

/// Upper incomplete gamma function: Γ(a, x) = ∫ₓ^∞ t^(a-1) e^(-t) dt
///
/// Γ(a, x) = Γ(a) - γ(a, x)
///
/// # Arguments
///
/// * `a` - Shape parameter (a > 0)
/// * `x` - Lower limit of integration (x >= 0)
pub fn upper_incomplete_gamma(a: f64, x: f64) -> Result<f64> {
    let (lower, complete) = lower_incomplete_gamma(a, x)?;
    Ok(complete - lower)
}

#[allow(dead_code)]
const WGSL_REGULARIZED_GAMMA_F64: &str = include_str!("../shaders/special/regularized_gamma_f64.wgsl");

/// Regularized lower incomplete gamma function: P(a, x) = γ(a, x) / Γ(a)
///
/// This is the CDF of the gamma distribution.
///
/// # Arguments
///
/// * `a` - Shape parameter (a > 0)
/// * `x` - Upper limit of integration (x >= 0)
///
/// # Returns
///
/// P(a, x) in [0, 1]
///
/// # Example
///
/// ```
/// use barracuda::special::regularized_gamma_p;
///
/// // P(1, 1) = 1 - e^(-1) ≈ 0.6321
/// let p = regularized_gamma_p(1.0, 1.0)?;
/// assert!((p - 0.6321).abs() < 0.01);
/// # Ok::<(), barracuda::error::BarracudaError>(())
/// ```
pub fn regularized_gamma_p(a: f64, x: f64) -> Result<f64> {
    if a <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("regularized_gamma_p requires a > 0, got {}", a),
        });
    }
    if x < 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("regularized_gamma_p requires x >= 0, got {}", x),
        });
    }

    if x == 0.0 {
        return Ok(0.0);
    }

    let gln = ln_gamma(a)?;

    if x < a + 1.0 {
        gamma_series(a, x, gln)
    } else {
        Ok(1.0 - gamma_cf(a, x, gln)?)
    }
}

/// Regularized upper incomplete gamma function: Q(a, x) = Γ(a, x) / Γ(a)
///
/// Q(a, x) = 1 - P(a, x)
///
/// # Arguments
///
/// * `a` - Shape parameter (a > 0)
/// * `x` - Lower limit of integration (x >= 0)
pub fn regularized_gamma_q(a: f64, x: f64) -> Result<f64> {
    Ok(1.0 - regularized_gamma_p(a, x)?)
}

/// Series expansion for regularized incomplete gamma P(a, x)
fn gamma_series(a: f64, x: f64, gln: f64) -> Result<f64> {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 1e-14;

    let mut sum = 1.0 / a;
    let mut term = sum;

    for n in 1..MAX_ITER {
        term *= x / (a + n as f64);
        sum += term;

        if term.abs() < sum.abs() * EPS {
            return Ok(sum * (-x + a * x.ln() - gln).exp());
        }
    }

    Err(BarracudaError::ExecutionError {
        message: "gamma_series: convergence failed".to_string(),
    })
}

/// Continued fraction for regularized incomplete gamma Q(a, x)
fn gamma_cf(a: f64, x: f64, gln: f64) -> Result<f64> {
    const MAX_ITER: usize = 200;
    const EPS: f64 = 1e-14;
    const FPMIN: f64 = 1e-30;

    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;

    for n in 1..MAX_ITER {
        let an = -(n as f64) * (n as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;

        if (delta - 1.0).abs() < EPS {
            return Ok((-x + a * x.ln() - gln).exp() * h);
        }
    }

    Err(BarracudaError::ExecutionError {
        message: "gamma_cf: convergence failed".to_string(),
    })
}

/// Digamma function: ψ(x) = d/dx ln(Γ(x)) = Γ'(x) / Γ(x)
///
/// The logarithmic derivative of the gamma function, commonly used in
/// statistics (e.g., expectation of log-gamma random variables) and
/// physics (e.g., harmonic oscillator energy levels).
///
/// # Arguments
///
/// * `x` - Input value (x > 0)
///
/// # Returns
///
/// ψ(x)
///
/// # Algorithm
///
/// Uses recurrence relation ψ(x+1) = ψ(x) + 1/x to reduce to x ≥ 7,
/// then applies asymptotic expansion:
///
/// ψ(x) ≈ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + ...
///
/// # Example
///
/// ```
/// use barracuda::special::digamma;
///
/// // ψ(1) = -γ (Euler-Mascheroni constant)
/// let psi_1 = digamma(1.0).unwrap();
/// assert!((psi_1 - (-0.5772156649015329)).abs() < 1e-9);
///
/// // Recurrence: ψ(2) = ψ(1) + 1
/// let psi_2 = digamma(2.0).unwrap();
/// assert!((psi_2 - (psi_1 + 1.0)).abs() < 1e-9);
/// ```
///
/// # Reference
///
/// - Abramowitz & Stegun, 6.3.18
/// - hotSpring Phase 5: inline implementation in `validate_special_functions.rs`
pub fn digamma(x: f64) -> Result<f64> {
    if x <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("digamma requires x > 0, got {}", x),
        });
    }

    // Recurrence to shift x >= 7
    let mut val = 0.0;
    let mut xx = x;
    while xx < 7.0 {
        val -= 1.0 / xx;
        xx += 1.0;
    }

    // Asymptotic expansion for large x
    let inv_x = 1.0 / xx;
    let inv_x2 = inv_x * inv_x;

    // ψ(x) ≈ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶)
    val += xx.ln() - 0.5 * inv_x - inv_x2 * (1.0 / 12.0 - inv_x2 * (1.0 / 120.0 - inv_x2 / 252.0));

    Ok(val)
}

/// Beta function: B(a, b) = Γ(a)Γ(b) / Γ(a+b)
///
/// The beta function is fundamental in probability (beta distribution) and
/// statistical inference (posterior distributions, Bayesian analysis).
///
/// # Arguments
///
/// * `a` - First parameter (a > 0)
/// * `b` - Second parameter (b > 0)
///
/// # Returns
///
/// B(a, b)
///
/// # Properties
///
/// - Symmetric: B(a, b) = B(b, a)
/// - B(1, 1) = 1
/// - B(a, 1) = 1/a
/// - B(1/2, 1/2) = π
///
/// # Example
///
/// ```
/// use barracuda::special::beta;
/// use std::f64::consts::PI;
///
/// // B(1, 1) = 1
/// let b_11 = beta(1.0, 1.0).unwrap();
/// assert!((b_11 - 1.0).abs() < 1e-10);
///
/// // B(1/2, 1/2) = π
/// let b_half = beta(0.5, 0.5).unwrap();
/// assert!((b_half - PI).abs() < 1e-10);
///
/// // B(a, 1) = 1/a
/// let b_a1 = beta(3.0, 1.0).unwrap();
/// assert!((b_a1 - 1.0/3.0).abs() < 1e-10);
/// ```
///
/// # Reference
///
/// - Abramowitz & Stegun, 6.2
/// - hotSpring Phase 5: inline implementation in `validate_special_functions.rs`
pub fn beta(a: f64, b: f64) -> Result<f64> {
    if a <= 0.0 || b <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("beta requires a > 0 and b > 0, got a={}, b={}", a, b),
        });
    }

    // B(a, b) = exp(ln(Γ(a)) + ln(Γ(b)) - ln(Γ(a+b)))
    let ln_beta = ln_gamma(a)? + ln_gamma(b)? - ln_gamma(a + b)?;
    Ok(ln_beta.exp())
}

#[allow(dead_code)]
const WGSL_LN_BETA_F64: &str = include_str!("../shaders/special/ln_beta_f64.wgsl");

/// Natural logarithm of the beta function: ln(B(a, b))
///
/// Useful for avoiding overflow when a, b are large.
///
/// # Example
///
/// ```
/// use barracuda::special::ln_beta;
///
/// // ln(B(1, 1)) = ln(1) = 0
/// let lb_11 = ln_beta(1.0, 1.0).unwrap();
/// assert!(lb_11.abs() < 1e-10);
///
/// // For large values, avoids overflow
/// let lb_large = ln_beta(100.0, 100.0).unwrap();
/// assert!(lb_large.is_finite());
/// ```
pub fn ln_beta(a: f64, b: f64) -> Result<f64> {
    if a <= 0.0 || b <= 0.0 {
        return Err(BarracudaError::InvalidInput {
            message: format!("ln_beta requires a > 0 and b > 0, got a={}, b={}", a, b),
        });
    }

    Ok(ln_gamma(a)? + ln_gamma(b)? - ln_gamma(a + b)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_ln_gamma_integers() {
        // ln(Γ(n)) = ln((n-1)!)
        assert!((ln_gamma(1.0).unwrap() - 0.0).abs() < 1e-10); // ln(0!) = 0
        assert!((ln_gamma(2.0).unwrap() - 0.0).abs() < 1e-10); // ln(1!) = 0
        assert!((ln_gamma(3.0).unwrap() - 2.0_f64.ln()).abs() < 1e-10); // ln(2!) = ln(2)
        assert!((ln_gamma(4.0).unwrap() - 6.0_f64.ln()).abs() < 1e-10); // ln(3!) = ln(6)
        assert!((ln_gamma(5.0).unwrap() - 24.0_f64.ln()).abs() < 1e-10); // ln(4!) = ln(24)
    }

    #[test]
    fn test_gamma_integers() {
        // Γ(n) = (n-1)!
        assert!((gamma(1.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((gamma(2.0).unwrap() - 1.0).abs() < 1e-10);
        assert!((gamma(3.0).unwrap() - 2.0).abs() < 1e-10);
        assert!((gamma(4.0).unwrap() - 6.0).abs() < 1e-10);
        assert!((gamma(5.0).unwrap() - 24.0).abs() < 1e-10);
    }

    #[test]
    fn test_gamma_half_integer() {
        // Γ(1/2) = √π
        assert!((gamma(0.5).unwrap() - PI.sqrt()).abs() < 1e-10);

        // Γ(3/2) = √π / 2
        assert!((gamma(1.5).unwrap() - PI.sqrt() / 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_gamma_invalid_input() {
        assert!(gamma(0.0).is_err());
        assert!(gamma(-1.0).is_err());
        assert!(gamma(-2.0).is_err());
    }

    #[test]
    fn test_regularized_gamma_p_exponential() {
        // For a=1, P(1, x) = 1 - e^(-x) (exponential CDF)
        let p = regularized_gamma_p(1.0, 1.0).unwrap();
        assert!((p - (1.0 - (-1.0_f64).exp())).abs() < 1e-10);

        let p = regularized_gamma_p(1.0, 2.0).unwrap();
        assert!((p - (1.0 - (-2.0_f64).exp())).abs() < 1e-10);
    }

    #[test]
    fn test_regularized_gamma_p_bounds() {
        // P(a, 0) = 0
        assert!((regularized_gamma_p(2.0, 0.0).unwrap() - 0.0).abs() < 1e-10);

        // P(a, x) approaches 1 as x -> infinity
        let p_large = regularized_gamma_p(2.0, 50.0).unwrap();
        assert!(p_large > 0.9999999);
    }

    #[test]
    fn test_regularized_gamma_q_complement() {
        // Q(a, x) = 1 - P(a, x)
        let a = 2.5;
        let x = 3.0;
        let p = regularized_gamma_p(a, x).unwrap();
        let q = regularized_gamma_q(a, x).unwrap();

        assert!((p + q - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_incomplete_gamma_relation() {
        // γ(a, x) + Γ(a, x) = Γ(a)
        let a = 2.0;
        let x = 1.5;

        let (lower, complete) = lower_incomplete_gamma(a, x).unwrap();
        let upper = upper_incomplete_gamma(a, x).unwrap();

        assert!((lower + upper - complete).abs() < 1e-10);
    }

    #[test]
    fn test_gamma_series_small_x() {
        // Test that series expansion works for x < a+1
        let p = regularized_gamma_p(3.0, 1.0).unwrap();
        assert!(p > 0.0 && p < 1.0);
    }

    #[test]
    fn test_gamma_cf_large_x() {
        // Test that continued fraction works for x >= a+1
        let p = regularized_gamma_p(2.0, 5.0).unwrap();
        assert!(p > 0.9 && p < 1.0);
    }

    // Digamma tests

    #[test]
    fn test_digamma_euler_mascheroni() {
        // ψ(1) = -γ (Euler-Mascheroni constant)
        const EULER_MASCHERONI: f64 = 0.5772156649015329;
        let psi_1 = digamma(1.0).unwrap();
        assert!(
            (psi_1 - (-EULER_MASCHERONI)).abs() < 1e-9,
            "ψ(1) = {}, expected {}",
            psi_1,
            -EULER_MASCHERONI
        );
    }

    #[test]
    fn test_digamma_recurrence() {
        // Recurrence relation: ψ(x+1) = ψ(x) + 1/x
        let psi_1 = digamma(1.0).unwrap();
        let psi_2 = digamma(2.0).unwrap();
        assert!(
            (psi_2 - (psi_1 + 1.0)).abs() < 1e-10,
            "ψ(2) = {}, expected ψ(1) + 1 = {}",
            psi_2,
            psi_1 + 1.0
        );

        let psi_3 = digamma(3.0).unwrap();
        assert!(
            (psi_3 - (psi_2 + 0.5)).abs() < 1e-10,
            "ψ(3) = {}, expected ψ(2) + 1/2 = {}",
            psi_3,
            psi_2 + 0.5
        );
    }

    #[test]
    fn test_digamma_half() {
        // ψ(1/2) = -γ - 2*ln(2)
        const EULER_MASCHERONI: f64 = 0.5772156649015329;
        let expected = -EULER_MASCHERONI - 2.0 * 2.0_f64.ln();
        let psi_half = digamma(0.5).unwrap();
        assert!(
            (psi_half - expected).abs() < 1e-9,
            "ψ(1/2) = {}, expected {}",
            psi_half,
            expected
        );
    }

    #[test]
    fn test_digamma_invalid_input() {
        assert!(digamma(0.0).is_err());
        assert!(digamma(-1.0).is_err());
    }

    // Beta function tests

    #[test]
    fn test_beta_simple_values() {
        // B(1, 1) = 1
        let b_11 = beta(1.0, 1.0).unwrap();
        assert!((b_11 - 1.0).abs() < 1e-10);

        // B(a, 1) = 1/a
        let b_31 = beta(3.0, 1.0).unwrap();
        assert!((b_31 - 1.0 / 3.0).abs() < 1e-10);

        // B(1, b) = 1/b
        let b_14 = beta(1.0, 4.0).unwrap();
        assert!((b_14 - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_beta_symmetry() {
        // B(a, b) = B(b, a)
        let b_ab = beta(2.5, 3.7).unwrap();
        let b_ba = beta(3.7, 2.5).unwrap();
        assert!((b_ab - b_ba).abs() < 1e-12);
    }

    #[test]
    fn test_beta_half_half() {
        // B(1/2, 1/2) = π
        let b_half = beta(0.5, 0.5).unwrap();
        assert!(
            (b_half - PI).abs() < 1e-10,
            "B(1/2, 1/2) = {}, expected π = {}",
            b_half,
            PI
        );
    }

    #[test]
    fn test_beta_integers() {
        // B(m, n) = (m-1)!(n-1)! / (m+n-1)! for positive integers
        // B(2, 3) = 1!*2! / 4! = 2/24 = 1/12
        let b_23 = beta(2.0, 3.0).unwrap();
        assert!((b_23 - 1.0 / 12.0).abs() < 1e-10);

        // B(3, 4) = 2!*3! / 6! = 12/720 = 1/60
        let b_34 = beta(3.0, 4.0).unwrap();
        assert!((b_34 - 1.0 / 60.0).abs() < 1e-10);
    }

    #[test]
    fn test_beta_invalid_input() {
        assert!(beta(0.0, 1.0).is_err());
        assert!(beta(1.0, 0.0).is_err());
        assert!(beta(-1.0, 2.0).is_err());
    }

    #[test]
    fn test_ln_beta() {
        // ln(B(1, 1)) = 0
        let lb_11 = ln_beta(1.0, 1.0).unwrap();
        assert!(lb_11.abs() < 1e-10);

        // ln(B(a, b)) = ln(beta(a, b))
        let a = 2.5;
        let b = 3.7;
        let lb = ln_beta(a, b).unwrap();
        let b_val = beta(a, b).unwrap();
        assert!((lb - b_val.ln()).abs() < 1e-12);
    }
}
