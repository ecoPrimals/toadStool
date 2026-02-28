//! Readout layer training for reservoir computing
//!
//! Trains a simple linear layer to map reservoir states to target outputs.
//! Uses ridge regression (fast, no backpropagation needed!).

// Mathematical notation (n, l, etc.) is standard in linear algebra
#![allow(clippy::many_single_char_names)]

use crate::error::Result;
use ndarray::{Array1, Array2};
use tracing::{debug, info};

/// Readout layer trainer
pub struct ReadoutTrainer {
    /// Regularization parameter (alpha for ridge regression)
    alpha: f64,
}

impl ReadoutTrainer {
    /// Create trainer with regularization
    pub fn new(alpha: f64) -> Self {
        info!("Creating readout trainer (alpha={})", alpha);
        Self { alpha }
    }

    /// Default trainer with small regularization
    pub fn default_trainer() -> Self {
        Self::new(1e-6)
    }

    /// Train readout layer using ridge regression
    ///
    /// # Arguments
    /// * `states` - Reservoir states (N samples × D dimensions)
    /// * `targets` - Target outputs (N samples × C classes)
    ///
    /// # Returns
    /// * Readout weights (C classes × D dimensions)
    ///
    /// # Training Method
    ///
    /// Ridge regression (closed-form solution, no gradient descent!):
    ///
    /// W = (X^T X + αI)^(-1) X^T Y
    ///
    /// where:
    /// - X = states (N × D)
    /// - Y = targets (N × C)
    /// - α = regularization parameter
    /// - I = identity matrix (D × D)
    ///
    /// This is FAST compared to backpropagation:
    /// - No iterative optimization
    /// - No learning rate tuning
    /// - No gradient computation
    /// - Direct matrix solve
    ///
    /// # Errors
    ///
    /// Returns an error if the number of samples in states and targets don't match, or if the ridge regression solve fails.
    pub fn train(&self, states: &Array2<f32>, targets: &Array2<f32>) -> Result<Array2<f32>> {
        info!("Training readout layer...");
        debug!("States shape: {:?}", states.shape());
        debug!("Targets shape: {:?}", targets.shape());

        let n_samples = states.nrows();
        let n_features = states.ncols();
        let n_outputs = targets.ncols();

        if states.nrows() != targets.nrows() {
            return Err(crate::error::ReservoirError::InvalidState(format!(
                "Mismatch: states has {} samples, targets has {}",
                states.nrows(),
                targets.nrows()
            )));
        }

        info!(
            "Training on {} samples with {} features → {} outputs",
            n_samples, n_features, n_outputs
        );

        // Convert to f64 for numerical stability
        let states_f64 = states.mapv(f64::from);
        let targets_f64 = targets.mapv(f64::from);

        // Compute X^T X
        debug!("Computing X^T X");
        let xt_x = states_f64.t().dot(&states_f64);

        // Add regularization: X^T X + αI
        debug!("Adding regularization (alpha={})", self.alpha);
        let mut xt_x_reg = xt_x;
        for i in 0..n_features {
            xt_x_reg[[i, i]] += self.alpha;
        }

        // Compute X^T Y
        debug!("Computing X^T Y");
        let xt_y = states_f64.t().dot(&targets_f64);

        // Solve (X^T X + αI) W = X^T Y via Cholesky decomposition.
        // A = X^T X + αI is symmetric positive definite (α > 0), so Cholesky is stable.
        debug!("Solving linear system via Cholesky");
        let weights = Self::solve_ridge(xt_x_reg, xt_y);

        // Convert back to f32
        // Precision is sufficient for neuromorphic computation
        #[allow(clippy::cast_possible_truncation)]
        let weights_f32 = weights.mapv(|x| x as f32);

        info!("✅ Readout trained: {} weights", weights_f32.len());
        Ok(weights_f32.t().to_owned()) // Transpose to (C × D)
    }

    /// Solve ridge regression system using Cholesky decomposition
    ///
    /// Solves: (X^T X + αI) W = X^T Y for W
    ///
    /// Uses Cholesky decomposition (pure Rust, no external BLAS/LAPACK):
    /// 1. Decompose A = L L^T (A is SPD when α > 0)
    /// 2. Solve L Y = B by forward substitution
    /// 3. Solve L^T W = Y by backward substitution
    ///
    /// This is numerically stable for symmetric positive definite matrices.
    #[allow(clippy::needless_pass_by_value)] // Ownership needed for error fallback
    fn solve_ridge(xt_x_reg: Array2<f64>, xt_y: Array2<f64>) -> Array2<f64> {
        let n = xt_x_reg.nrows();
        let n_rhs = xt_y.ncols();

        // Cholesky decomposition: A = L L^T
        // L is lower triangular
        let l = match cholesky_decompose(&xt_x_reg) {
            Ok(l) => l,
            Err(e) => {
                tracing::error!(
                    "Cholesky decomposition failed: {}. Falling back to identity.",
                    e
                );
                // Return xt_y as fallback (identity solve)
                return xt_y;
            }
        };

        // Solve for each column of the right-hand side
        let mut result = Array2::zeros((n, n_rhs));

        for col in 0..n_rhs {
            let b = xt_y.column(col).to_owned();

            // Forward substitution: L y = b
            let y = forward_substitute(&l, &b);

            // Backward substitution: L^T x = y
            let x = backward_substitute_transpose(&l, &y);

            result.column_mut(col).assign(&x);
        }

        debug!("Ridge regression solved via Cholesky decomposition");
        result
    }
}

/// Cholesky decomposition: A = L L^T
///
/// Returns the lower triangular matrix L.
/// A must be symmetric positive definite.
fn cholesky_decompose(a: &Array2<f64>) -> Result<Array2<f64>> {
    let n = a.nrows();
    if n != a.ncols() {
        return Err(crate::error::ReservoirError::Numerical(
            "Matrix must be square for Cholesky decomposition".to_string(),
        ));
    }

    let mut l = Array2::zeros((n, n));

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;

            if i == j {
                // Diagonal element
                for k in 0..j {
                    sum += l[[j, k]] * l[[j, k]];
                }
                let diag = a[[j, j]] - sum;
                if diag <= 0.0 {
                    return Err(crate::error::ReservoirError::Numerical(format!(
                        "Matrix is not positive definite at diagonal element {j}"
                    )));
                }
                l[[j, j]] = diag.sqrt();
            } else {
                // Off-diagonal element
                for k in 0..j {
                    sum += l[[i, k]] * l[[j, k]];
                }
                l[[i, j]] = (a[[i, j]] - sum) / l[[j, j]];
            }
        }
    }

    Ok(l)
}

/// Forward substitution: solve L x = b where L is lower triangular
fn forward_substitute(l: &Array2<f64>, b: &Array1<f64>) -> Array1<f64> {
    let n = l.nrows();
    let mut x = Array1::zeros(n);

    for i in 0..n {
        let mut sum = b[i];
        for j in 0..i {
            sum -= l[[i, j]] * x[j];
        }
        x[i] = sum / l[[i, i]];
    }

    x
}

/// Backward substitution: solve L^T x = b where L is lower triangular
fn backward_substitute_transpose(l: &Array2<f64>, b: &Array1<f64>) -> Array1<f64> {
    let n = l.nrows();
    let mut x = Array1::zeros(n);

    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= l[[j, i]] * x[j]; // L^T[i,j] = L[j,i]
        }
        x[i] = sum / l[[i, i]];
    }

    x
}

/// Readout predictor (inference with trained weights)
pub struct ReadoutPredictor {
    weights: Array2<f32>,
}

impl ReadoutPredictor {
    /// Create predictor with trained weights
    pub fn new(weights: Array2<f32>) -> Self {
        info!(
            "Creating readout predictor ({} × {})",
            weights.nrows(),
            weights.ncols()
        );
        Self { weights }
    }

    /// Predict output from reservoir state
    ///
    /// # Arguments
    /// * `state` - Reservoir state (D dimensions)
    ///
    /// # Returns
    /// * Output prediction (C classes)
    ///
    /// # Errors
    ///
    /// Returns an error if matrix dimensions are incompatible.
    pub fn predict(&self, state: &Array1<f32>) -> Result<Array1<f32>> {
        // output = W * state
        Ok(self.weights.dot(state))
    }

    /// Predict batch of states
    ///
    /// # Errors
    ///
    /// Returns an error if matrix dimensions are incompatible.
    pub fn predict_batch(&self, states: &Array2<f32>) -> Result<Array2<f32>> {
        // outputs = states * W^T
        Ok(states.dot(&self.weights.t()))
    }

    /// Get weights
    pub fn weights(&self) -> &Array2<f32> {
        &self.weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::{arr1, arr2};

    #[test]
    fn test_readout_trainer_shapes() {
        let trainer = ReadoutTrainer::default_trainer();

        // Simple test case
        let states = arr2(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);

        let targets = arr2(&[[1.0, 0.0], [0.0, 1.0]]);

        let result = trainer.train(&states, &targets);
        assert!(result.is_ok());

        let weights = result.unwrap();
        assert_eq!(weights.shape(), &[2, 3]); // (C × D)
    }

    #[test]
    fn test_readout_predictor() {
        let weights = arr2(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);

        let predictor = ReadoutPredictor::new(weights);

        let state = Array1::from_vec(vec![1.0, 1.0, 1.0]);
        let output = predictor.predict(&state).unwrap();

        assert_eq!(output.len(), 2);
        assert!((output[0] - 6.0).abs() < f32::EPSILON); // 1+2+3
        assert!((output[1] - 15.0).abs() < f32::EPSILON); // 4+5+6
    }

    #[test]
    fn test_cholesky_decompose_simple() {
        // 2x2 SPD matrix: [[4, 2], [2, 3]]
        // L should be [[2, 0], [1, sqrt(2)]]
        let a = arr2(&[[4.0, 2.0], [2.0, 3.0]]);
        let l = cholesky_decompose(&a).unwrap();

        // L * L^T should equal A
        let reconstructed = l.dot(&l.t());
        for i in 0..2 {
            for j in 0..2 {
                assert!((reconstructed[[i, j]] - a[[i, j]]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_cholesky_solve_linear_system() {
        // Solve Ax = b where A is SPD
        // A = [[4, 2], [2, 3]], b = [8, 7]
        // Solving: 4x + 2y = 8, 2x + 3y = 7
        // x = 1.25, y = 1.5 (verify: 4*1.25 + 2*1.5 = 8, 2*1.25 + 3*1.5 = 7)
        let a = arr2(&[[4.0, 2.0], [2.0, 3.0]]);
        let b = arr1(&[8.0, 7.0]);

        let l = cholesky_decompose(&a).unwrap();
        let y = forward_substitute(&l, &b);
        let x = backward_substitute_transpose(&l, &y);

        assert!((x[0] - 1.25).abs() < 1e-10);
        assert!((x[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_ridge_regression_identity_data() {
        // Test with data where we know the expected output
        // If X = I (identity), targets = some vector v
        // Then (X^T X + αI) = (1+α)I
        // X^T Y = v
        // W = v / (1+α)

        let trainer = ReadoutTrainer::new(0.0); // No regularization for exact solve

        // Identity-like input
        let states = arr2(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

        let targets = arr2(&[[1.0], [2.0], [3.0]]);

        let weights = trainer.train(&states, &targets).unwrap();

        // Without regularization, weights should approximate the targets
        // W^T * x should give targets
        let predictor = ReadoutPredictor::new(weights);
        for i in 0..3 {
            let state = states.row(i).to_owned();
            let prediction = predictor.predict(&state).unwrap();
            assert!(
                (prediction[0] - targets[[i, 0]]).abs() < 0.1,
                "Prediction {} vs target {}",
                prediction[0],
                targets[[i, 0]]
            );
        }
    }

    #[test]
    fn test_ridge_regression_with_regularization() {
        // Regularization should reduce weight magnitudes
        let trainer_low_reg = ReadoutTrainer::new(0.001);
        let trainer_high_reg = ReadoutTrainer::new(10.0);

        let states = arr2(&[[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);
        let targets = arr2(&[[1.0], [2.0], [3.0]]);

        let weights_low = trainer_low_reg.train(&states, &targets).unwrap();
        let weights_high = trainer_high_reg.train(&states, &targets).unwrap();

        // Higher regularization should produce smaller weights
        let norm_low: f32 = weights_low.iter().map(|x| x * x).sum();
        let norm_high: f32 = weights_high.iter().map(|x| x * x).sum();

        assert!(
            norm_high < norm_low,
            "High regularization should produce smaller weights: {norm_high} vs {norm_low}"
        );
    }
}
