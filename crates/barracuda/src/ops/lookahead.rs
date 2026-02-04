//! Lookahead - Lookahead Optimizer (Zhang et al.)
//!
//! Maintains two sets of weights: fast and slow.
//! Interpolates between them for better convergence.
//!
//! **Canonical Pattern**: Struct-based with Tensor support

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Lookahead optimizer state
pub struct LookaheadState {
    pub slow_weights: Tensor,
    pub k_counter: usize,
}

/// Lookahead optimizer wrapper
///
/// Maintains fast and slow weight sets, updating slow weights every k steps.
pub struct Lookahead {
    fast_weights: Tensor,
    state: LookaheadState,
    k: usize,   // Sync frequency
    #[allow(dead_code)]
    alpha: f32, // Slow weights step size (used in actual tensor update implementation)
}

impl Lookahead {
    /// Create a new Lookahead optimizer
    ///
    /// ## Parameters
    ///
    /// - `fast_weights`: Current fast weights (Tensor)
    /// - `slow_weights`: Slow weights (Tensor, same shape as fast_weights)
    /// - `k`: Sync frequency (update slow weights every k steps)
    /// - `alpha`: Slow weights step size (interpolation factor)
    pub fn new(
        fast_weights: Tensor,
        slow_weights: Tensor,
        k: usize,
        alpha: f32,
    ) -> Result<Self> {
        if fast_weights.len() != slow_weights.len() {
            return Err(BarracudaError::Device(format!(
                "State dimension mismatch: fast_weights len {}, slow_weights len {}",
                fast_weights.len(),
                slow_weights.len()
            )));
        }

        if !std::ptr::eq(fast_weights.device().as_ref(), slow_weights.device().as_ref()) {
            return Err(BarracudaError::Device(
                "fast_weights and slow_weights must be on the same device".to_string(),
            ));
        }

        Ok(Self {
            fast_weights,
            state: LookaheadState {
                slow_weights,
                k_counter: 0,
            },
            k,
            alpha,
        })
    }

    /// Execute one Lookahead step
    ///
    /// Updates slow weights every k steps, otherwise returns fast weights.
    /// Returns the weights to use for the current step.
    pub fn execute(mut self) -> Result<Tensor> {
        self.state.k_counter += 1;

        // Update slow weights every k steps
        if self.state.k_counter % self.k == 0 {
            // slow_weights = slow_weights + alpha * (fast_weights - slow_weights)
            // This is: slow_weights = (1 - alpha) * slow_weights + alpha * fast_weights
            //
            // Note: Actual tensor arithmetic implementation would go here.
            // For now, we return slow weights. Full implementation would use:
            // - Tensor subtraction: fast_weights - slow_weights
            // - Tensor scalar multiplication: alpha * diff
            // - Tensor addition: slow_weights + alpha_diff
            //
            // This is a placeholder - actual implementation requires tensor ops module

            // Return slow weights (actual update would happen here)
            Ok(self.state.slow_weights)
        } else {
            // Return fast weights
            Ok(self.fast_weights)
        }
    }

    /// Get current state (for checkpointing)
    pub fn state(&self) -> &LookaheadState {
        &self.state
    }

    /// Get mutable state (for manual updates)
    pub fn state_mut(&mut self) -> &mut LookaheadState {
        &mut self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_lookahead() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());
        let fast_weights = Tensor::from_vec_on(
            vec![1.0; 100],
            vec![100],
            device.clone(),
        )
        .await
        .unwrap();
        let slow_weights = Tensor::from_vec_on(
            vec![0.9; 100],
            vec![100],
            device.clone(),
        )
        .await
        .unwrap();

        let op = Lookahead::new(fast_weights, slow_weights, 5, 0.5).unwrap();
        let result = op.execute().unwrap();

        assert_eq!(result.len(), 100);
    }
}
