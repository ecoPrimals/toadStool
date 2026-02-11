//! Numerical gradient computation via finite differences

/// Compute numerical gradient using 3-point finite difference stencil
///
/// Matches numpy.gradient() behavior for 1D arrays with uniform spacing.
///
/// # Arguments
///
/// * `f` - Function values [f(x₀), f(x₁), ..., f(xₙ)]
/// * `dx` - Grid spacing (uniform)
///
/// # Returns
///
/// Gradient [df/dx(x₀), df/dx(x₁), ..., df/dx(xₙ)]
///
/// # Algorithm
///
/// - **Interior points**: Central difference (f[i+1] - f[i-1]) / (2·dx)
/// - **Boundary points**: Forward/backward 1st-order difference
///
/// # Examples
///
/// ```
/// use barracuda::numerical::gradient_1d;
///
/// // Gradient of y = x²
/// let y = vec![0.0, 1.0, 4.0, 9.0, 16.0];  // x = [0, 1, 2, 3, 4]
/// let dy_dx = gradient_1d(&y, 1.0);
///
/// // dy/dx should be approximately 2x
/// assert!((dy_dx[0] - 0.0).abs() < 0.1);   // at x=0, dy/dx ≈ 0
/// assert!((dy_dx[1] - 2.0).abs() < 0.1);   // at x=1, dy/dx ≈ 2
/// assert!((dy_dx[2] - 4.0).abs() < 0.1);   // at x=2, dy/dx ≈ 4
/// assert!((dy_dx[3] - 6.0).abs() < 0.1);   // at x=3, dy/dx ≈ 6
/// assert!((dy_dx[4] - 8.0).abs() < 0.1);   // at x=4, dy/dx ≈ 8
/// ```
///
/// # References
///
/// - numpy.gradient: <https://numpy.org/doc/stable/reference/generated/numpy.gradient.html>
pub fn gradient_1d(f: &[f64], dx: f64) -> Vec<f64> {
    let n = f.len();

    if n == 0 {
        return Vec::new();
    }

    if n == 1 {
        return vec![0.0];
    }

    let mut grad = vec![0.0; n];

    // Forward difference at start
    grad[0] = (f[1] - f[0]) / dx;

    // Central difference for interior points
    for i in 1..n - 1 {
        grad[i] = (f[i + 1] - f[i - 1]) / (2.0 * dx);
    }

    // Backward difference at end
    grad[n - 1] = (f[n - 1] - f[n - 2]) / dx;

    grad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_1d_linear() {
        // y = 2x (gradient should be 2.0 everywhere)
        let y = vec![0.0, 2.0, 4.0, 6.0, 8.0];
        let grad = gradient_1d(&y, 1.0);

        for g in &grad {
            assert!((g - 2.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_gradient_1d_quadratic() {
        // y = x² (gradient should be 2x)
        let x: Vec<f64> = (0..5).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|xi| xi * xi).collect();
        let grad = gradient_1d(&y, 1.0);

        // Interior points should match 2x closely
        for i in 1..4 {
            let expected = 2.0 * x[i];
            assert!(
                (grad[i] - expected).abs() < 0.1,
                "grad[{}] = {}, expected {}",
                i,
                grad[i],
                expected
            );
        }
    }

    #[test]
    fn test_gradient_1d_constant() {
        // y = 5.0 (gradient should be 0)
        let y = vec![5.0; 10];
        let grad = gradient_1d(&y, 1.0);

        for g in &grad {
            assert!(g.abs() < 1e-10);
        }
    }

    #[test]
    fn test_gradient_1d_empty() {
        let grad = gradient_1d(&[], 1.0);
        assert_eq!(grad.len(), 0);
    }

    #[test]
    fn test_gradient_1d_single() {
        let grad = gradient_1d(&[42.0], 1.0);
        assert_eq!(grad, vec![0.0]);
    }

    #[test]
    fn test_gradient_1d_nonuniform_spacing() {
        // Test with dx = 0.5
        let y = vec![0.0, 0.5, 2.0, 4.5, 8.0]; // y = 2x² with x = [0, 0.5, 1, 1.5, 2]
        let grad = gradient_1d(&y, 0.5);

        // At x=1.0 (i=2), dy/dx should be 4.0
        assert!((grad[2] - 4.0).abs() < 0.2);
    }
}
