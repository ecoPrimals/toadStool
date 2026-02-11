//! Linear system solvers

use crate::error::{BarracudaError, Result};

/// Solve Ax = b using Gauss-Jordan elimination with partial pivoting
///
/// This is a direct method that works for general square matrices.
/// For symmetric positive definite matrices, Cholesky decomposition
/// is more efficient (future implementation).
///
/// # Arguments
///
/// * `a` - Coefficient matrix (row-major, n×n)
/// * `b` - Right-hand side vector (length n)
/// * `n` - Matrix dimension
///
/// # Returns
///
/// Solution vector x, or error if matrix is singular
///
/// # Precision
///
/// f64 on CPU. For f32 GPU version, use the `linsolve.wgsl` shader.
///
/// # Examples
///
/// ```
/// use barracuda::linalg::solve_f64;
///
/// // Solve:
/// //   2x + y = 5
/// //   x + 3y = 8
/// //
/// // Matrix form: [2 1] [x]   [5]
/// //              [1 3] [y] = [8]
///
/// let a = vec![
///     2.0, 1.0,
///     1.0, 3.0,
/// ];
/// let b = vec![5.0, 8.0];
///
/// let x = solve_f64(&a, &b, 2)?;
///
/// // Solution: x=1, y=3
/// assert!((x[0] - 1.0).abs() < 1e-10);
/// assert!((x[1] - 3.0).abs() < 1e-10);
/// # Ok::<(), barracuda::error::BarracudaError>(())
/// ```
///
/// # Algorithm
///
/// 1. **Forward elimination** with partial pivoting
///     - For each column k, find row with largest |a[i,k]|
///     - Swap rows to bring pivot to diagonal
///     - Eliminate column k below diagonal
/// 2. **Backward substitution**
///     - Solve for x from bottom to top
///
/// # References
///
/// - Golub & Van Loan, "Matrix Computations", 4th ed., Algorithm 3.4.1
/// - numpy.linalg.solve
pub fn solve_f64(a: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>> {
    if a.len() != n * n {
        return Err(BarracudaError::InvalidInput {
            message: format!(
                "Matrix size mismatch: expected {}×{} = {}, got {}",
                n,
                n,
                n * n,
                a.len()
            ),
        });
    }

    if b.len() != n {
        return Err(BarracudaError::InvalidInput {
            message: format!("Vector size mismatch: expected {}, got {}", n, b.len()),
        });
    }

    if n == 0 {
        return Ok(Vec::new());
    }

    // Create augmented matrix [A | b]
    let mut aug = vec![0.0; n * (n + 1)];

    for i in 0..n {
        for j in 0..n {
            aug[i * (n + 1) + j] = a[i * n + j];
        }
        aug[i * (n + 1) + n] = b[i];
    }

    // Forward elimination with partial pivoting
    for k in 0..n {
        // Find pivot row (max |a[i,k]| for i >= k)
        let mut max_row = k;
        let mut max_val = aug[k * (n + 1) + k].abs();

        for i in (k + 1)..n {
            let val = aug[i * (n + 1) + k].abs();
            if val > max_val {
                max_val = val;
                max_row = i;
            }
        }

        // Check for singularity
        if max_val < 1e-14 {
            return Err(BarracudaError::ExecutionError {
                message: format!(
                    "Singular matrix: pivot at column {} is near-zero ({:e})",
                    k, max_val
                ),
            });
        }

        // Swap rows k and max_row
        if max_row != k {
            for j in k..=n {
                aug.swap(k * (n + 1) + j, max_row * (n + 1) + j);
            }
        }

        // Eliminate column k below diagonal
        for i in (k + 1)..n {
            let factor = aug[i * (n + 1) + k] / aug[k * (n + 1) + k];

            for j in k..=n {
                aug[i * (n + 1) + j] -= factor * aug[k * (n + 1) + j];
            }
        }
    }

    // Backward substitution
    let mut x = vec![0.0; n];

    for i in (0..n).rev() {
        let mut sum = aug[i * (n + 1) + n];

        for j in (i + 1)..n {
            sum -= aug[i * (n + 1) + j] * x[j];
        }

        x[i] = sum / aug[i * (n + 1) + i];
    }

    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_f64_2x2() {
        // 2x + y = 5
        // x + 3y = 8
        // Solution: x = 1.4, y = 2.2
        let a = vec![2.0, 1.0, 1.0, 3.0];
        let b = vec![5.0, 8.0];

        let x = solve_f64(&a, &b, 2).unwrap();

        assert!((x[0] - 1.4).abs() < 1e-10);
        assert!((x[1] - 2.2).abs() < 1e-10);
    }

    #[test]
    fn test_solve_f64_3x3() {
        // 3x + 2y - z = 1
        // 2x - 2y + 4z = -2
        // -x + 0.5y - z = 0
        let a = vec![3.0, 2.0, -1.0, 2.0, -2.0, 4.0, -1.0, 0.5, -1.0];
        let b = vec![1.0, -2.0, 0.0];

        let x = solve_f64(&a, &b, 3).unwrap();

        // Solution: x=1, y=-2, z=-2
        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - (-2.0)).abs() < 1e-10);
        assert!((x[2] - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_solve_f64_identity() {
        // Ix = b should give x = b
        let a = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let b = vec![5.0, 7.0, 9.0];

        let x = solve_f64(&a, &b, 3).unwrap();

        assert!((x[0] - 5.0).abs() < 1e-14);
        assert!((x[1] - 7.0).abs() < 1e-14);
        assert!((x[2] - 9.0).abs() < 1e-14);
    }

    #[test]
    fn test_solve_f64_diagonal() {
        // Diagonal matrix
        let a = vec![2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let b = vec![6.0, 9.0, 12.0];

        let x = solve_f64(&a, &b, 3).unwrap();

        assert!((x[0] - 3.0).abs() < 1e-14);
        assert!((x[1] - 3.0).abs() < 1e-14);
        assert!((x[2] - 3.0).abs() < 1e-14);
    }

    #[test]
    fn test_solve_f64_singular_matrix() {
        // Singular matrix (row 2 = row 1)
        let a = vec![1.0, 2.0, 1.0, 2.0];
        let b = vec![1.0, 1.0];

        let result = solve_f64(&a, &b, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_solve_f64_size_mismatch() {
        let a = vec![1.0, 2.0, 3.0]; // Wrong size
        let b = vec![1.0, 2.0];

        let result = solve_f64(&a, &b, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_solve_f64_empty() {
        let x = solve_f64(&[], &[], 0).unwrap();
        assert_eq!(x.len(), 0);
    }

    #[test]
    fn test_solve_f64_large_well_conditioned() {
        // 5×5 well-conditioned system
        let n = 5;
        let mut a = vec![0.0; n * n];
        let mut b = vec![0.0; n];

        // Create a diagonally dominant matrix (well-conditioned)
        for i in 0..n {
            a[i * n + i] = 10.0; // Diagonal
            b[i] = (i + 1) as f64;

            for j in 0..n {
                if i != j {
                    a[i * n + j] = 0.1; // Off-diagonal
                }
            }
        }

        let x = solve_f64(&a, &b, n).unwrap();

        // Verify Ax = b
        for i in 0..n {
            let mut ax_i = 0.0;
            for j in 0..n {
                ax_i += a[i * n + j] * x[j];
            }
            assert!(
                (ax_i - b[i]).abs() < 1e-10,
                "Row {}: Ax = {}, b = {}",
                i,
                ax_i,
                b[i]
            );
        }
    }
}
