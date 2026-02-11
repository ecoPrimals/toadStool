//! Radial basis function surrogate for expensive function approximation

use super::kernels::RBFKernel;
use crate::error::{BarracudaError, Result};
use crate::linalg::solve_f64;

/// RBF surrogate model with polynomial augmentation
///
/// Approximates expensive functions using radial basis functions:
/// s(x) = Σᵢ wᵢ φ(‖x - xᵢ‖) + p(x)
///
/// where:
/// - φ is the RBF kernel
/// - wᵢ are weights (learned from data)
/// - p(x) is a polynomial tail (linear: 1, x₁, ..., xₙ)
///
/// # Dual-Precision Architecture (Future)
///
/// Currently CPU f64 only. Future enhancement: GPU f32 cdist → promote → CPU f64 solve.
#[derive(Debug)]
pub struct RBFSurrogate {
    /// Training points (flattened: [n_train × n_dim])
    train_x: Vec<f64>,

    /// RBF weights (length n_train)
    weights: Vec<f64>,

    /// Polynomial coefficients (length n_dim + 1)
    poly_coeffs: Vec<f64>,

    /// Number of training points
    n_train: usize,

    /// Dimension of input space
    n_dim: usize,

    /// Kernel function
    kernel: RBFKernel,

    /// Smoothing parameter (regularization)
    #[allow(dead_code)]
    smoothing: f64,
}

impl RBFSurrogate {
    /// Construct from pre-computed parts (used by adaptive dispatch).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        train_x: Vec<f64>,
        weights: Vec<f64>,
        poly_coeffs: Vec<f64>,
        n_train: usize,
        n_dim: usize,
        kernel: RBFKernel,
        smoothing: f64,
    ) -> Self {
        Self {
            train_x,
            weights,
            poly_coeffs,
            n_train,
            n_dim,
            kernel,
            smoothing,
        }
    }

    /// Train RBF surrogate on data
    ///
    /// # Arguments
    ///
    /// * `x_data` - Training points [[x₁₁, x₁₂, ...], [x₂₁, x₂₂, ...], ...]
    /// * `y_data` - Training values [y₁, y₂, ...]
    /// * `kernel` - RBF kernel type
    /// * `smoothing` - Regularization parameter (1e-12 for exact interpolation)
    ///
    /// # Returns
    ///
    /// Trained surrogate model
    ///
    /// # Algorithm
    ///
    /// 1. Compute pairwise distances: D[i,j] = ‖xᵢ - xⱼ‖
    /// 2. Assemble kernel matrix: K[i,j] = φ(D[i,j]) + δᵢⱼ·smoothing
    /// 3. Augment with polynomial: [K P; Pᵀ 0] [w; c] = [y; 0]
    /// 4. Solve for weights w and polynomial coefficients c
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use barracuda::surrogate::{RBFSurrogate, RBFKernel};
    ///
    /// // Training data: y = x²
    /// let x_train = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
    /// let y_train = vec![0.0, 1.0, 4.0, 9.0];
    ///
    /// let surrogate = RBFSurrogate::train(
    ///     &x_train,
    ///     &y_train,
    ///     RBFKernel::ThinPlateSpline,
    ///     1e-12,
    /// )?;
    ///
    /// // Predict at new points
    /// let y_pred = surrogate.predict(&[vec![1.5], vec![2.5]])?;
    /// # Ok::<(), barracuda::error::BarracudaError>(())
    /// ```
    pub fn train(
        x_data: &[Vec<f64>],
        y_data: &[f64],
        kernel: RBFKernel,
        smoothing: f64,
    ) -> Result<Self> {
        let n_train = x_data.len();

        if n_train == 0 {
            return Err(BarracudaError::InvalidInput {
                message: "Training data cannot be empty".to_string(),
            });
        }

        if y_data.len() != n_train {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "x_data and y_data length mismatch: {} vs {}",
                    n_train,
                    y_data.len()
                ),
            });
        }

        let n_dim = x_data[0].len();

        // Flatten training data (can't use extend_from_slice due to nested structure)
        #[allow(clippy::manual_memcpy)]
        let train_x: Vec<f64> = x_data.iter().flat_map(|row| row.iter().copied()).collect();

        // Compute pairwise distances (CPU f64 for now)
        let distances = compute_distances(&train_x, &train_x, n_train, n_train, n_dim);

        // Assemble augmented system
        let n_poly = n_dim + 1; // 1 + x₁ + x₂ + ... + xₙ
        let n_total = n_train + n_poly;

        let mut a = vec![0.0; n_total * n_total];
        let mut b = vec![0.0; n_total];

        // Top-left: Kernel matrix K + smoothing·I
        for i in 0..n_train {
            for j in 0..n_train {
                let k_ij = kernel.eval(distances[i * n_train + j]);
                let smooth = if i == j { smoothing } else { 0.0 };
                a[i * n_total + j] = k_ij + smooth;
            }
        }

        // Top-right and bottom-left: Polynomial matrix P
        for i in 0..n_train {
            // Constant term
            a[i * n_total + n_train] = 1.0;
            a[n_train * n_total + i] = 1.0;

            // Linear terms
            for d in 0..n_dim {
                a[i * n_total + (n_train + 1 + d)] = train_x[i * n_dim + d];
                a[(n_train + 1 + d) * n_total + i] = train_x[i * n_dim + d];
            }
        }

        // Bottom-right: Zero block (already initialized to 0)

        // Right-hand side
        b[..n_train].copy_from_slice(y_data);
        // Polynomial constraints are zero (already initialized)

        // Solve linear system
        let solution = solve_f64(&a, &b, n_total)?;

        // Extract weights and polynomial coefficients
        let weights = solution[..n_train].to_vec();
        let poly_coeffs = solution[n_train..].to_vec();

        Ok(Self {
            train_x,
            weights,
            poly_coeffs,
            n_train,
            n_dim,
            kernel,
            smoothing,
        })
    }

    /// Predict at new points
    ///
    /// # Arguments
    ///
    /// * `x_eval` - Evaluation points [[x₁₁, x₁₂, ...], ...]
    ///
    /// # Returns
    ///
    /// Predicted values [ŷ₁, ŷ₂, ...]
    pub fn predict(&self, x_eval: &[Vec<f64>]) -> Result<Vec<f64>> {
        let n_eval = x_eval.len();

        if n_eval == 0 {
            return Ok(Vec::new());
        }

        if x_eval[0].len() != self.n_dim {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Dimension mismatch: expected {}, got {}",
                    self.n_dim,
                    x_eval[0].len()
                ),
            });
        }

        // Flatten evaluation points (can't use extend_from_slice due to nested structure)
        #[allow(clippy::manual_memcpy)]
        let eval_x: Vec<f64> = x_eval.iter().flat_map(|row| row.iter().copied()).collect();

        // Compute distances from evaluation points to training points
        let distances = compute_distances(&eval_x, &self.train_x, n_eval, self.n_train, self.n_dim);

        let mut predictions = Vec::with_capacity(n_eval);

        for i in 0..n_eval {
            let mut pred = 0.0;

            // RBF contribution
            for j in 0..self.n_train {
                let dist = distances[i * self.n_train + j];
                let phi = self.kernel.eval(dist);
                pred += self.weights[j] * phi;
            }

            // Polynomial contribution
            pred += self.poly_coeffs[0]; // Constant
            for d in 0..self.n_dim {
                pred += self.poly_coeffs[1 + d] * eval_x[i * self.n_dim + d];
            }

            predictions.push(pred);
        }

        Ok(predictions)
    }
}

/// Compute pairwise Euclidean distances (CPU f64)
///
/// Returns flattened distance matrix [n1 × n2]
fn compute_distances(x1: &[f64], x2: &[f64], n1: usize, n2: usize, n_dim: usize) -> Vec<f64> {
    let mut distances = vec![0.0; n1 * n2];

    for i in 0..n1 {
        for j in 0..n2 {
            let mut dist_sq = 0.0;
            for d in 0..n_dim {
                let diff = x1[i * n_dim + d] - x2[j * n_dim + d];
                dist_sq += diff * diff;
            }
            distances[i * n2 + j] = dist_sq.sqrt();
        }
    }

    distances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbf_linear_1d() {
        // Interpolate y = 2x
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y_train = vec![0.0, 2.0, 4.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        // Should interpolate training points exactly
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..3 {
            assert!(
                (y_pred[i] - y_train[i]).abs() < 1e-10,
                "Failed to interpolate training point {}: pred = {}, true = {}",
                i,
                y_pred[i],
                y_train[i]
            );
        }

        // Test interpolation
        let y_mid = surrogate.predict(&[vec![1.5]]).unwrap();
        assert!(
            (y_mid[0] - 3.0).abs() < 0.1,
            "Poor interpolation at x=1.5: {}",
            y_mid[0]
        );
    }

    #[test]
    fn test_rbf_quadratic_1d() {
        // Approximate y = x²
        let x_train: Vec<Vec<f64>> = (0..5).map(|i| vec![i as f64]).collect();
        let y_train: Vec<f64> = (0..5).map(|i| (i * i) as f64).collect();

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        // Should interpolate training points exactly
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..5 {
            assert!(
                (y_pred[i] - y_train[i]).abs() < 1e-8,
                "Failed at training point x={}: pred = {}, true = {}",
                i,
                y_pred[i],
                y_train[i]
            );
        }
    }

    #[test]
    fn test_rbf_2d() {
        // Simple 2D function: f(x,y) = x + y
        let x_train = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
        ];
        let y_train = vec![0.0, 1.0, 1.0, 2.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        // Test center point
        let y_center = surrogate.predict(&[vec![0.5, 0.5]]).unwrap();
        assert!(
            (y_center[0] - 1.0).abs() < 0.1,
            "Poor interpolation at center: {}",
            y_center[0]
        );
    }

    #[test]
    fn test_rbf_gaussian_kernel() {
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y_train = vec![0.0, 1.0, 0.0]; // Peak at x=1

        let surrogate = RBFSurrogate::train(
            &x_train,
            &y_train,
            RBFKernel::Gaussian { epsilon: 1.0 },
            1e-12,
        )
        .unwrap();

        // Should interpolate training points
        let y_pred = surrogate.predict(&x_train).unwrap();
        for i in 0..3 {
            assert!((y_pred[i] - y_train[i]).abs() < 1e-8);
        }
    }

    #[test]
    fn test_rbf_empty_training_data() {
        let result = RBFSurrogate::train(&[], &[], RBFKernel::ThinPlateSpline, 1e-12);
        assert!(result.is_err());
    }

    #[test]
    fn test_rbf_mismatched_lengths() {
        let x_train = vec![vec![0.0], vec![1.0]];
        let y_train = vec![0.0, 1.0, 2.0]; // Wrong length

        let result = RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12);
        assert!(result.is_err());
    }
}
