//! Gamma function and related functions
//!
//! Implements Γ(x), ln Γ(x), ψ(x), and B(a,b) using Lanczos approximation.
//!
//! # Functions
//!
//! - `gamma(x)` - Gamma function Γ(x)
//! - `lgamma(x)` - Log-gamma function ln Γ(x)
//! - `digamma(x)` - Digamma function ψ(x) = d/dx ln Γ(x)
//! - `beta(a, b)` - Beta function B(a,b) = Γ(a)Γ(b)/Γ(a+b)
//!
//! # References
//!
//! - Lanczos, C. (1964). "A Precision Approximation of the Gamma Function"
//! - Abramowitz & Stegun, §6.3 (digamma), §6.2 (beta)
//! - DLMF 5: <https://dlmf.nist.gov/5>

use std::f64::consts::PI;

/// Compute Γ(x) via Lanczos approximation
///
/// The gamma function is defined as Γ(n) = (n-1)! for positive integers,
/// and extends to all complex numbers except non-positive integers.
///
/// # Arguments
///
/// * `x` - Real number (must be > 0)
///
/// # Returns
///
/// Γ(x)
///
/// # Algorithm
///
/// Uses 9-term Lanczos approximation with g=7, accurate to ~15 digits.
/// Special handling for positive half-integers (n + 1/2) for exact results.
///
/// # Examples
///
/// ```
/// use barracuda::special::gamma;
/// use std::f64::consts::PI;
///
/// // Γ(1) = 1
/// assert!((gamma(1.0) - 1.0).abs() < 1e-14);
///
/// // Γ(5) = 4! = 24
/// assert!((gamma(5.0) - 24.0).abs() < 1e-12);
///
/// // Γ(1/2) = √π
/// assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-12);
///
/// // Γ(3/2) = √π / 2
/// assert!((gamma(1.5) - PI.sqrt() / 2.0).abs() < 1e-12);
/// ```
///
/// # References
///
/// - Lanczos, C. (1964). "A Precision Approximation of the Gamma Function"
/// - Numerical Recipes, 3rd Edition, Section 6.1
pub fn gamma(x: f64) -> f64 {
    if x <= 0.0 {
        // Reflection formula for negative values
        // Γ(x)Γ(1-x) = π / sin(πx)
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }

    // Special case: positive half-integers (n + 1/2)
    // Γ(n + 1/2) = √π · (2n-1)!! / 2ⁿ
    // Check if x = n + 0.5 where n is integer
    let n_plus_half = x - 0.5;
    if n_plus_half >= 0.0 && (n_plus_half - n_plus_half.round()).abs() < 1e-10 {
        return gamma_half_integer(x);
    }

    // Lanczos approximation with g = 7
    lanczos_gamma(x)
}

/// Exact computation for positive half-integers
///
/// Γ(n + 1/2) = √π · (2n-1)!! / 2ⁿ
fn gamma_half_integer(x: f64) -> f64 {
    let n = (x - 0.5).round() as i32;

    if n < 0 {
        return lanczos_gamma(x);
    }

    let mut result = PI.sqrt();

    // Compute (2n-1)!! = 1 · 3 · 5 · ... · (2n-1)
    for k in 1..=n {
        result *= (2 * k - 1) as f64;
    }

    // Divide by 2ⁿ
    result / (1u64 << n) as f64
}

/// Lanczos approximation (9-term, g=7)
///
/// Γ(x+1) ≈ √(2π) · (x + g + 0.5)^(x + 0.5) · exp(-(x + g + 0.5)) · Aₓ
///
/// where Aₓ = c₀ + Σᵢ cᵢ/(x + i)
fn lanczos_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;

    // Lanczos coefficients for g=7, n=9 (precision required for gamma function accuracy)
    #[allow(clippy::excessive_precision)]
    const LANCZOS_COEFF: [f64; 9] = [
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

    if x < 0.5 {
        // Use reflection formula for better accuracy
        PI / ((PI * x).sin() * lanczos_gamma(1.0 - x))
    } else {
        // Standard Lanczos formula
        let x = x - 1.0;
        let mut a = LANCZOS_COEFF[0];

        for i in 1..9 {
            a += LANCZOS_COEFF[i] / (x + i as f64);
        }

        let t = x + G + 0.5;
        let sqrt_2pi = (2.0 * PI).sqrt();

        sqrt_2pi * t.powf(x + 0.5) * (-t).exp() * a
    }
}

/// Compute the log-gamma function ln Γ(x).
///
/// More numerically stable than computing log(gamma(x)) for large x.
///
/// # Arguments
///
/// * `x` - Positive real number
///
/// # Returns
///
/// ln Γ(x)
///
/// # Examples
///
/// ```
/// use barracuda::special::lgamma;
/// use std::f64::consts::PI;
///
/// // ln Γ(1) = 0
/// assert!(lgamma(1.0).abs() < 1e-14);
///
/// // ln Γ(1/2) = ln √π
/// assert!((lgamma(0.5) - 0.5 * PI.ln()).abs() < 1e-12);
/// ```
pub fn lgamma(x: f64) -> f64 {
    if x <= 0.0 {
        return f64::NAN;
    }

    const G: f64 = 7.0;

    #[allow(clippy::excessive_precision)]
    const LANCZOS_COEFF: [f64; 9] = [
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

    let x = x - 1.0;
    let mut a = LANCZOS_COEFF[0];

    for i in 1..9 {
        a += LANCZOS_COEFF[i] / (x + i as f64);
    }

    let t = x + G + 0.5;

    // ln Γ(x+1) = 0.5*ln(2π) + (x+0.5)*ln(t) - t + ln(a)
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Compute the digamma function ψ(x) = d/dx ln Γ(x).
///
/// The digamma function is the logarithmic derivative of the gamma function.
/// It appears in Bayesian statistics, maximum likelihood estimation, and
/// various physics applications.
///
/// # Arguments
///
/// * `x` - Positive real number
///
/// # Returns
///
/// ψ(x) = Γ'(x)/Γ(x)
///
/// # Algorithm
///
/// Uses the asymptotic expansion for large x and recurrence for small x.
///
/// # Examples
///
/// ```
/// use barracuda::special::digamma;
///
/// // ψ(1) = -γ (Euler-Mascheroni constant)
/// let euler_gamma = 0.5772156649015329;
/// assert!((digamma(1.0) + euler_gamma).abs() < 1e-10);
///
/// // ψ(2) = 1 - γ
/// assert!((digamma(2.0) - (1.0 - euler_gamma)).abs() < 1e-10);
/// ```
///
/// # References
///
/// - Abramowitz & Stegun, §6.3.18 (asymptotic expansion)
/// - DLMF 5.7.6: <https://dlmf.nist.gov/5.7.6>
pub fn digamma(x: f64) -> f64 {
    if x <= 0.0 {
        // Reflection formula: ψ(1-x) - ψ(x) = π·cot(πx)
        return digamma(1.0 - x) + PI / (PI * x).tan();
    }

    // Use recurrence to shift x to larger values where asymptotic expansion is accurate
    // ψ(x+1) = ψ(x) + 1/x
    let mut result = 0.0;
    let mut x = x;

    while x < 7.0 {
        result -= 1.0 / x;
        x += 1.0;
    }

    // Asymptotic expansion (A&S 6.3.18)
    // ψ(x) ≈ ln(x) - 1/(2x) - 1/(12x²) + 1/(120x⁴) - 1/(252x⁶) + ...
    let x2 = x * x;
    let x4 = x2 * x2;
    let x6 = x4 * x2;

    result + x.ln() - 0.5 / x - 1.0 / (12.0 * x2) + 1.0 / (120.0 * x4) - 1.0 / (252.0 * x6)
}

/// Compute the beta function B(a, b) = Γ(a)Γ(b)/Γ(a+b).
///
/// The beta function is fundamental in Bayesian statistics (beta distribution)
/// and appears in many combinatorial and integral formulas.
///
/// # Arguments
///
/// * `a` - First parameter (positive)
/// * `b` - Second parameter (positive)
///
/// # Returns
///
/// B(a, b) = ∫₀¹ t^(a-1) (1-t)^(b-1) dt
///
/// # Properties
///
/// - B(a, b) = B(b, a) (symmetric)
/// - B(a, 1) = 1/a
/// - B(1, 1) = 1
/// - B(n, m) = (n-1)!(m-1)!/(n+m-1)! for positive integers
///
/// # Examples
///
/// ```
/// use barracuda::special::beta;
///
/// // B(1, 1) = 1
/// assert!((beta(1.0, 1.0) - 1.0).abs() < 1e-14);
///
/// // B(2, 3) = Γ(2)Γ(3)/Γ(5) = 1·2/24 = 1/12
/// assert!((beta(2.0, 3.0) - 1.0/12.0).abs() < 1e-14);
///
/// // Symmetric
/// assert!((beta(3.0, 5.0) - beta(5.0, 3.0)).abs() < 1e-14);
/// ```
///
/// # References
///
/// - Abramowitz & Stegun, §6.2
/// - DLMF 5.12: <https://dlmf.nist.gov/5.12>
pub fn beta(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return f64::NAN;
    }

    // Use log-gamma for numerical stability
    // B(a,b) = exp(lgamma(a) + lgamma(b) - lgamma(a+b))
    (lgamma(a) + lgamma(b) - lgamma(a + b)).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamma_integers() {
        // Γ(n) = (n-1)!
        assert!((gamma(1.0) - 1.0).abs() < 1e-14); // Γ(1) = 0! = 1
        assert!((gamma(2.0) - 1.0).abs() < 1e-14); // Γ(2) = 1! = 1
        assert!((gamma(3.0) - 2.0).abs() < 1e-14); // Γ(3) = 2! = 2
        assert!((gamma(4.0) - 6.0).abs() < 1e-13); // Γ(4) = 3! = 6
        assert!((gamma(5.0) - 24.0).abs() < 1e-12); // Γ(5) = 4! = 24
        assert!((gamma(6.0) - 120.0).abs() < 1e-11); // Γ(6) = 5! = 120
        assert!((gamma(10.0) - 362880.0).abs() < 1e-8); // Γ(10) = 9!
    }

    #[test]
    fn test_gamma_half_integers() {
        // Γ(1/2) = √π
        assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-14);

        // Γ(3/2) = √π / 2
        assert!((gamma(1.5) - PI.sqrt() / 2.0).abs() < 1e-14);

        // Γ(5/2) = 3√π / 4
        assert!((gamma(2.5) - 3.0 * PI.sqrt() / 4.0).abs() < 1e-13);

        // Γ(7/2) = 15√π / 8
        assert!((gamma(3.5) - 15.0 * PI.sqrt() / 8.0).abs() < 1e-13);
    }

    #[test]
    fn test_gamma_fractional() {
        // Test some known values
        // Γ(0.1) ≈ 9.513507698668732
        assert!((gamma(0.1) - 9.513507698668732).abs() < 1e-12);

        // Γ(2.5) = 3√π/4 ≈ 1.329340388179137
        let expected = 3.0 * PI.sqrt() / 4.0;
        assert!((gamma(2.5) - expected).abs() < 1e-13);
    }

    #[test]
    fn test_gamma_large() {
        // Γ(15) = 14! = 87178291200
        let expected = 87178291200.0;
        let result = gamma(15.0);
        println!(
            "gamma(15) = {}, expected = {}, error = {}",
            result,
            expected,
            (result - expected).abs()
        );
        // Lanczos approximation has relative error, not absolute
        assert!((result - expected).abs() / expected < 1e-10);
    }

    #[test]
    fn test_gamma_recurrence() {
        // Γ(x+1) = x·Γ(x)
        for x in [1.0, 2.0, 3.5, 5.7, 10.3] {
            let gamma_x = gamma(x);
            let gamma_x_plus_1 = gamma(x + 1.0);
            assert!(
                (gamma_x_plus_1 - x * gamma_x).abs() / gamma_x_plus_1 < 1e-12,
                "Recurrence failed for x={}: Γ({}) * {} = {} vs Γ({}) = {}",
                x,
                x,
                x,
                x * gamma_x,
                x + 1.0,
                gamma_x_plus_1
            );
        }
    }

    #[test]
    fn test_gamma_reflection() {
        // Γ(x)Γ(1-x) = π / sin(πx)
        for x in [0.1, 0.3, 0.7, 0.9] {
            let gamma_x = gamma(x);
            let gamma_1_minus_x = gamma(1.0 - x);
            let expected = PI / (PI * x).sin();

            let product = gamma_x * gamma_1_minus_x;
            assert!(
                (product - expected).abs() / expected < 1e-11,
                "Reflection failed for x={}: Γ({})·Γ({}) = {} vs π/sin(πx) = {}",
                x,
                x,
                1.0 - x,
                product,
                expected
            );
        }
    }

    // lgamma tests
    #[test]
    fn test_lgamma_one() {
        // ln Γ(1) = ln(1) = 0
        assert!(lgamma(1.0).abs() < 1e-14);
    }

    #[test]
    fn test_lgamma_half() {
        // ln Γ(1/2) = ln √π = 0.5 ln π
        assert!((lgamma(0.5) - 0.5 * PI.ln()).abs() < 1e-12);
    }

    #[test]
    fn test_lgamma_integers() {
        // ln Γ(n) = ln((n-1)!)
        assert!((lgamma(2.0) - 0.0).abs() < 1e-14); // ln(1!) = 0
        assert!((lgamma(3.0) - 2.0_f64.ln()).abs() < 1e-14); // ln(2!) = ln(2)
        assert!((lgamma(5.0) - 24.0_f64.ln()).abs() < 1e-12); // ln(4!) = ln(24)
    }

    #[test]
    fn test_lgamma_vs_log_gamma() {
        // lgamma(x) should equal log(gamma(x)) for moderate x
        for x in [1.5, 2.5, 5.0, 10.0] {
            let lg = lgamma(x);
            let log_g = gamma(x).ln();
            assert!(
                (lg - log_g).abs() < 1e-10,
                "lgamma({}) = {} vs log(gamma) = {}",
                x,
                lg,
                log_g
            );
        }
    }

    // digamma tests
    #[test]
    fn test_digamma_one() {
        // ψ(1) = -γ (Euler-Mascheroni constant)
        // Asymptotic expansion precision ~1e-9
        let euler_gamma = 0.5772156649015329;
        assert!((digamma(1.0) + euler_gamma).abs() < 1e-9);
    }

    #[test]
    fn test_digamma_two() {
        // ψ(2) = ψ(1) + 1 = 1 - γ
        let euler_gamma = 0.5772156649015329;
        assert!((digamma(2.0) - (1.0 - euler_gamma)).abs() < 1e-9);
    }

    #[test]
    fn test_digamma_recurrence() {
        // ψ(x+1) = ψ(x) + 1/x
        for x in [1.0, 2.5, 5.0, 10.0] {
            let psi_x = digamma(x);
            let psi_x_plus_1 = digamma(x + 1.0);
            assert!(
                (psi_x_plus_1 - psi_x - 1.0 / x).abs() < 1e-10,
                "Recurrence failed for x={}: ψ({}) - ψ({}) = {} vs 1/x = {}",
                x,
                x + 1.0,
                x,
                psi_x_plus_1 - psi_x,
                1.0 / x
            );
        }
    }

    #[test]
    fn test_digamma_scipy_values() {
        // Compare with scipy.special.digamma
        // scipy.special.digamma(0.5) = -1.9635100260214235
        assert!((digamma(0.5) - (-1.9635100260214235)).abs() < 1e-9);
        // scipy.special.digamma(5.0) = 1.5061176684318
        assert!((digamma(5.0) - 1.5061176684318).abs() < 1e-9);
    }

    // beta tests
    #[test]
    fn test_beta_one_one() {
        // B(1, 1) = 1
        assert!((beta(1.0, 1.0) - 1.0).abs() < 1e-14);
    }

    #[test]
    fn test_beta_integers() {
        // B(n, m) = (n-1)!(m-1)!/(n+m-1)!
        // B(2, 3) = 1·2/24 = 1/12
        assert!((beta(2.0, 3.0) - 1.0 / 12.0).abs() < 1e-14);
        // B(3, 4) = 2·6/720 = 1/60
        assert!((beta(3.0, 4.0) - 1.0 / 60.0).abs() < 1e-14);
    }

    #[test]
    fn test_beta_symmetry() {
        // B(a, b) = B(b, a)
        for (a, b) in [(2.0, 5.0), (1.5, 3.5), (0.5, 2.0)] {
            assert!((beta(a, b) - beta(b, a)).abs() < 1e-14);
        }
    }

    #[test]
    fn test_beta_computed_values() {
        // B(0.5, 0.5) = Γ(0.5)²/Γ(1) = π
        assert!((beta(0.5, 0.5) - PI).abs() < 1e-12);
        // B(2.5, 3.5) = Γ(2.5)Γ(3.5)/Γ(6) ≈ 0.0368
        // Γ(2.5) = 1.329..., Γ(3.5) = 3.323..., Γ(6) = 120
        let b_25_35 = beta(2.5, 3.5);
        let expected = gamma(2.5) * gamma(3.5) / gamma(6.0);
        assert!(
            (b_25_35 - expected).abs() < 1e-12,
            "beta(2.5,3.5) = {} but expected {}",
            b_25_35,
            expected
        );
    }

    #[test]
    fn test_beta_via_gamma() {
        // B(a,b) = Γ(a)Γ(b)/Γ(a+b)
        for (a, b) in [(2.0, 3.0), (0.5, 1.5), (3.5, 2.5)] {
            let b_ab = beta(a, b);
            let via_gamma = gamma(a) * gamma(b) / gamma(a + b);
            assert!(
                (b_ab - via_gamma).abs() / b_ab < 1e-12,
                "B({},{}) = {} vs via gamma = {}",
                a,
                b,
                b_ab,
                via_gamma
            );
        }
    }
}
