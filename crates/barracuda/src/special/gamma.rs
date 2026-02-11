//! Gamma function via Lanczos approximation

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
}
