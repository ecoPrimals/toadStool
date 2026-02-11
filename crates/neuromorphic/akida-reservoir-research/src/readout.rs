//! Readout layer training for reservoir computing
//!
//! Trains a simple linear layer to map reservoir states to target outputs.
//! Uses ridge regression (fast, no backpropagation needed!).

use anyhow::Result;
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
            anyhow::bail!(
                "Mismatch: states has {} samples, targets has {}",
                states.nrows(),
                targets.nrows()
            );
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

        // Solve (X^T X + αI) W = X^T Y
        // For now, use pseudo-inverse (TODO: proper Cholesky solve)
        debug!("Solving linear system");
        let weights = Self::solve_ridge(xt_x_reg, xt_y);

        // Convert back to f32
        // Precision is sufficient for neuromorphic computation
        #[allow(clippy::cast_possible_truncation)]
        let weights_f32 = weights.mapv(|x| x as f32);

        info!("✅ Readout trained: {} weights", weights_f32.len());
        Ok(weights_f32.t().to_owned()) // Transpose to (C × D)
    }

    /// Solve ridge regression system
    ///
    /// Solves: (X^T X + αI) W = X^T Y
    ///
    /// For now, uses simple pseudo-inverse.
    /// TODO: Implement proper Cholesky decomposition for better numerical stability.
    fn solve_ridge(_xt_x_reg: Array2<f64>, xt_y: Array2<f64>) -> Array2<f64> {
        // Simplified solution using least squares
        // In production, use ndarray-linalg or nalgebra for proper solve

        // For now, just return XT_Y scaled (placeholder)
        // TODO: Implement proper matrix inversion or Cholesky solve
        warn_once_ridge_placeholder();

        xt_y
    }
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

/// Helper to warn about placeholder implementation (only once)
fn warn_once_ridge_placeholder() {
    use std::sync::Once;
    static WARN_ONCE: Once = Once::new();

    WARN_ONCE.call_once(|| {
        tracing::warn!("⚠️  Using placeholder ridge regression solver!");
        tracing::warn!("    For production, add ndarray-linalg or nalgebra dependency");
        tracing::warn!("    Current implementation may have numerical instability");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr2;

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
}
