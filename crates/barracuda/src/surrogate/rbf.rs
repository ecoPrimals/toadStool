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
///
/// # Leave-One-Out Cross-Validation
///
/// LOO-CV provides a measure of surrogate quality without needing a separate
/// validation set:
///
/// ```ignore
/// let surrogate = RBFSurrogate::train(&x_data, &y_data, kernel, 1e-12)?;
/// let loo_rmse = surrogate.loo_cv_rmse()?;
/// println!("LOO-CV RMSE: {:.6}", loo_rmse);
/// ```
#[derive(Debug)]
pub struct RBFSurrogate {
    /// Training points (flattened: [n_train × n_dim])
    train_x: Vec<f64>,

    /// Training targets
    train_y: Vec<f64>,

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
    smoothing: f64,
}

impl RBFSurrogate {
    /// Construct from pre-computed parts (used by adaptive dispatch).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        train_x: Vec<f64>,
        train_y: Vec<f64>,
        weights: Vec<f64>,
        poly_coeffs: Vec<f64>,
        n_train: usize,
        n_dim: usize,
        kernel: RBFKernel,
        smoothing: f64,
    ) -> Self {
        Self {
            train_x,
            train_y,
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
            train_y: y_data.to_vec(),
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

    // === Leave-One-Out Cross-Validation ===

    /// Compute leave-one-out cross-validation RMSE.
    ///
    /// LOO-CV provides an unbiased estimate of prediction error without
    /// requiring a separate validation set. For RBF interpolation with
    /// smoothing λ > 0, the LOO residual is:
    ///
    /// LOO_i = (y_i - ŷ_i) / (1 - H_ii)
    ///
    /// where H is the hat matrix H = K(K + λI)⁻¹.
    ///
    /// # Returns
    ///
    /// Root mean square of LOO residuals
    ///
    /// # Example
    ///
    /// ```ignore
    /// let surrogate = RBFSurrogate::train(&x_data, &y_data, kernel, 1e-6)?;
    /// let rmse = surrogate.loo_cv_rmse()?;
    /// println!("LOO-CV RMSE: {:.6}", rmse);
    /// ```
    ///
    /// # Notes
    ///
    /// - For exact interpolation (smoothing ≈ 0), H_ii ≈ 1 and LOO residuals
    ///   are undefined. Use smoothing > 1e-10 for meaningful LOO-CV.
    /// - This is O(n³) due to hat matrix computation.
    pub fn loo_cv_rmse(&self) -> Result<f64> {
        let loo_residuals = self.loo_cv_errors()?;
        let mse = loo_residuals.iter().map(|r| r * r).sum::<f64>() / self.n_train as f64;
        Ok(mse.sqrt())
    }

    /// Compute per-point LOO-CV errors.
    ///
    /// Returns LOO_i = (y_i - ŷ_i) / (1 - H_ii) for each training point.
    /// Useful for identifying outliers or poorly-fit regions.
    ///
    /// # Returns
    ///
    /// Vector of LOO residuals (length n_train)
    pub fn loo_cv_errors(&self) -> Result<Vec<f64>> {
        if self.n_train == 0 {
            return Ok(Vec::new());
        }

        // Compute predictions at training points
        let train_points: Vec<Vec<f64>> = (0..self.n_train)
            .map(|i| {
                let start = i * self.n_dim;
                self.train_x[start..start + self.n_dim].to_vec()
            })
            .collect();
        let predictions = self.predict(&train_points)?;

        // Compute hat matrix diagonal
        let h_diag = self.compute_hat_diagonal()?;

        // Compute LOO residuals
        let mut loo_residuals = Vec::with_capacity(self.n_train);
        for i in 0..self.n_train {
            let residual = self.train_y[i] - predictions[i];
            let denom = 1.0 - h_diag[i];

            // Avoid division by zero (H_ii ≈ 1 means exact interpolation)
            let loo = if denom.abs() < 1e-10 {
                0.0 // Edge case: point has full influence
            } else {
                residual / denom
            };
            loo_residuals.push(loo);
        }

        Ok(loo_residuals)
    }

    /// Compute diagonal of the hat matrix H = K_raw · (K_smooth)⁻¹.
    ///
    /// For RBF interpolation with regularization, the hat matrix is:
    /// H = K_raw · (K_raw + λI)⁻¹
    ///
    /// where K_raw is the kernel matrix WITHOUT regularization and
    /// K_smooth = K_raw + λI is the regularized matrix.
    ///
    /// H_ii measures how much influence point i has on its own prediction.
    /// For exact interpolation (λ → 0), H_ii → 1.
    /// For smoothed interpolation, H_ii < 1.
    ///
    /// # Algorithm (hotSpring validated)
    ///
    /// For each point i:
    /// 1. Solve K_smooth · w = e_i (standard basis vector)
    /// 2. H_ii = K_raw[i,:] · w = dot product of row i with solution
    ///
    /// This correctly gives H_ii < 1 when smoothing > 0.
    fn compute_hat_diagonal(&self) -> Result<Vec<f64>> {
        let n = self.n_train;

        // Compute distance matrix
        let distances = compute_distances(
            &self.train_x,
            &self.train_x,
            n,
            n,
            self.n_dim,
        );

        // Build K_raw (kernel matrix WITHOUT smoothing)
        let mut k_raw = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                k_raw[i * n + j] = self.kernel.eval(distances[i * n + j]);
            }
        }

        // Build K_smooth = K_raw + λI (kernel matrix WITH smoothing)
        let mut k_smooth = k_raw.clone();
        for i in 0..n {
            k_smooth[i * n + i] += self.smoothing;
        }

        // Compute H_ii for each point
        // H = K_raw · (K_smooth)⁻¹
        // H_ii = e_i^T · K_raw · (K_smooth)⁻¹ · e_i
        //      = K_raw[i,:] · (K_smooth)⁻¹ · e_i
        //      = K_raw[i,:] · w_i  where K_smooth · w_i = e_i

        let mut h_diag = Vec::with_capacity(n);

        for i in 0..n {
            // Create e_i (standard basis vector)
            let mut e_i = vec![0.0; n];
            e_i[i] = 1.0;

            // Solve K_smooth · w = e_i
            let w = solve_f64(&k_smooth, &e_i, n)?;

            // H_ii = K_raw[i,:] · w (dot product)
            let h_ii: f64 = (0..n).map(|j| k_raw[i * n + j] * w[j]).sum();
            h_diag.push(h_ii);
        }

        Ok(h_diag)
    }

    /// Get the number of training points.
    pub fn n_train(&self) -> usize {
        self.n_train
    }

    /// Get the input dimension.
    pub fn n_dim(&self) -> usize {
        self.n_dim
    }
}

/// Result of LOO-CV grid search for optimal smoothing.
pub struct LooSmoothing {
    /// Optimal smoothing parameter
    pub smoothing: f64,
    /// LOO-CV RMSE at optimal smoothing
    pub rmse: f64,
    /// All grid search results as (smoothing, rmse) pairs
    pub grid_results: Vec<(f64, f64)>,
}

/// Find optimal smoothing parameter via LOO-CV grid search.
///
/// Performs grid search over logarithmically-spaced smoothing values,
/// returning the smoothing that minimizes LOO-CV RMSE.
///
/// # Arguments
///
/// * `x_data` - Training points (each inner vec is one point)
/// * `y_data` - Training targets
/// * `kernel` - RBF kernel to use
/// * `smoothing_grid` - Grid of smoothing values to test (or None for default)
///
/// # Returns
///
/// [`LooSmoothing`] with optimal_smoothing, optimal_rmse, and all grid results.
///
/// # Example
///
/// ```ignore
/// let result = loo_cv_optimal_smoothing(
///     &x_data, &y_data,
///     RBFKernel::ThinPlateSpline,
///     None,  // Use default grid
/// )?;
/// println!("Optimal smoothing: {:.2e}, RMSE: {:.6}", result.smoothing, result.rmse);
/// ```
///
/// # Reference
///
/// hotSpring validation: `surrogate.rs::loo_cv_optimal_smoothing()`
pub fn loo_cv_optimal_smoothing(
    x_data: &[Vec<f64>],
    y_data: &[f64],
    kernel: RBFKernel,
    smoothing_grid: Option<&[f64]>,
) -> Result<LooSmoothing> {
    // Default grid: logarithmically spaced from 1e-10 to 1.0
    let default_grid: Vec<f64> = (-10..=0)
        .map(|i| 10.0_f64.powi(i))
        .collect();
    let grid = smoothing_grid.unwrap_or(&default_grid);

    if grid.is_empty() {
        return Err(BarracudaError::InvalidInput {
            message: "smoothing_grid cannot be empty".into(),
        });
    }

    let mut results = Vec::with_capacity(grid.len());
    let mut best_smoothing = grid[0];
    let mut best_rmse = f64::INFINITY;

    for &s in grid {
        // Train surrogate with this smoothing
        let surrogate = match RBFSurrogate::train(x_data, y_data, kernel, s) {
            Ok(surr) => surr,
            Err(_) => continue, // Skip invalid configurations
        };

        // Compute LOO-CV RMSE
        let rmse = match surrogate.loo_cv_rmse() {
            Ok(r) if r.is_finite() => r,
            _ => continue, // Skip non-finite RMSE
        };

        results.push((s, rmse));

        if rmse < best_rmse {
            best_rmse = rmse;
            best_smoothing = s;
        }
    }

    if results.is_empty() {
        return Err(BarracudaError::ExecutionError {
            message: "No valid smoothing values found during grid search".into(),
        });
    }

    Ok(LooSmoothing {
        smoothing: best_smoothing,
        rmse: best_rmse,
        grid_results: results,
    })
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

    #[test]
    fn test_loo_cv_rmse() {
        // With smoothing, LOO-CV should give meaningful results
        let x_train = vec![
            vec![0.0],
            vec![0.5],
            vec![1.0],
            vec![1.5],
            vec![2.0],
        ];
        // Noisy linear function: y ≈ 2x
        let y_train = vec![0.1, 1.1, 1.9, 3.1, 3.9];

        // Use moderate smoothing so LOO-CV is defined
        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-6).unwrap();

        let loo_rmse = surrogate.loo_cv_rmse().unwrap();

        // Should be non-negative
        assert!(loo_rmse >= 0.0);

        // Should be small since data is nearly linear
        assert!(
            loo_rmse < 1.0,
            "LOO-CV RMSE too large: {}",
            loo_rmse
        );
    }

    #[test]
    fn test_loo_cv_errors() {
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y_train = vec![0.0, 1.0, 4.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-4).unwrap();

        let errors = surrogate.loo_cv_errors().unwrap();

        // Should have one error per training point
        assert_eq!(errors.len(), 3);

        // Errors should be finite
        for e in &errors {
            assert!(e.is_finite(), "Non-finite LOO error: {}", e);
        }
    }

    #[test]
    fn test_loo_cv_with_exact_interpolation() {
        // With very small smoothing (exact interpolation), LOO residuals
        // may be near zero or undefined (H_ii ≈ 1)
        let x_train = vec![vec![0.0], vec![1.0], vec![2.0]];
        let y_train = vec![0.0, 1.0, 4.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        // Should not panic
        let _ = surrogate.loo_cv_rmse();
    }

    #[test]
    fn test_rbf_accessors() {
        // Need at least n_dim + 1 points for polynomial augmentation
        let x_train = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
        ];
        let y_train = vec![0.0, 1.0, 1.0, 2.0];

        let surrogate =
            RBFSurrogate::train(&x_train, &y_train, RBFKernel::ThinPlateSpline, 1e-12).unwrap();

        assert_eq!(surrogate.n_train(), 4);
        assert_eq!(surrogate.n_dim(), 2);
    }

    #[test]
    fn test_loo_cv_hat_diagonal_correct() {
        // Test that H_ii < 1 when smoothing > 0 (the bug was H_ii = 1 always)
        // Use non-linear noisy data that won't be perfectly fit
        let x_train = vec![
            vec![0.0],
            vec![0.5],
            vec![1.0],
            vec![1.5],
            vec![2.0],
            vec![2.5],
            vec![3.0],
        ];
        // Noisy quadratic: y ≈ x² + noise
        let y_train = vec![0.1, 0.35, 0.9, 2.3, 4.1, 6.2, 9.1];

        // Use significant smoothing with Gaussian kernel
        let surrogate = RBFSurrogate::train(
            &x_train, &y_train,
            RBFKernel::Gaussian { epsilon: 1.0 },
            0.5  // High smoothing to force underfitting
        ).unwrap();

        let errors = surrogate.loo_cv_errors().unwrap();

        // With the CORRECT hat matrix and high smoothing, predictions != targets
        // So residuals (y - ŷ) should be non-zero
        // And with H_ii < 1, the LOO residuals should also be non-zero

        // First verify we have some residuals (smoothed predictions differ from targets)
        let train_points: Vec<Vec<f64>> = x_train.clone();
        let predictions = surrogate.predict(&train_points).unwrap();
        let max_residual: f64 = predictions.iter()
            .zip(y_train.iter())
            .map(|(p, y)| (p - y).abs())
            .fold(0.0, f64::max);

        // With high smoothing, the fit should be imperfect
        assert!(
            max_residual > 0.01,
            "Expected imperfect fit with high smoothing. Max residual: {}",
            max_residual
        );

        // LOO errors should be non-zero (the main test)
        let max_abs_error = errors.iter().map(|e| e.abs()).fold(0.0, f64::max);
        assert!(
            max_abs_error > 0.001,
            "LOO errors should be non-zero with smoothing. Max error: {}",
            max_abs_error
        );
    }

    #[test]
    fn test_loo_cv_smoothing_effect() {
        // More smoothing should generally increase LOO-CV RMSE (underfitting)
        // Less smoothing should decrease LOO-CV RMSE (better fit, but risk overfitting)
        let x_train: Vec<Vec<f64>> = (0..10)
            .map(|i| vec![i as f64 * 0.1])
            .collect();
        let y_train: Vec<f64> = x_train
            .iter()
            .map(|x| 2.0 * x[0] + 0.1 * (x[0] * 10.0).sin())
            .collect();

        let kernel = RBFKernel::Gaussian { epsilon: 1.0 };

        let surrogate_low = RBFSurrogate::train(
            &x_train, &y_train, kernel, 1e-4
        ).unwrap();
        let surrogate_high = RBFSurrogate::train(
            &x_train, &y_train, kernel, 0.5
        ).unwrap();

        let rmse_low = surrogate_low.loo_cv_rmse().unwrap();
        let rmse_high = surrogate_high.loo_cv_rmse().unwrap();

        // Both should be finite and positive
        assert!(rmse_low.is_finite() && rmse_low > 0.0);
        assert!(rmse_high.is_finite() && rmse_high > 0.0);

        // High smoothing should generally give higher RMSE (underfitting)
        // But this isn't strictly monotonic, so we just verify they're different
        assert!(
            (rmse_low - rmse_high).abs() > 1e-6,
            "LOO-CV should be sensitive to smoothing. low={}, high={}",
            rmse_low, rmse_high
        );
    }
}
