//! Lookahead - Lookahead Optimizer (Zhang et al.)
//!
//! Maintains two sets of weights: fast and slow.
//! Interpolates between them for better convergence.

pub struct LookaheadState {
    pub slow_weights: Vec<f32>,
    pub k_counter: usize,
}

pub async fn lookahead_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    fast_weights: &[f32],
    state: &mut LookaheadState,
    k: usize,  // Sync frequency
    alpha: f32, // Slow weights step size
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = fast_weights.len();
    if state.slow_weights.len() != size {
        return Err("State dimension mismatch".into());
    }
    
    state.k_counter += 1;
    
    // Update slow weights every k steps
    if state.k_counter % k == 0 {
        for i in 0..size {
            state.slow_weights[i] += alpha * (fast_weights[i] - state.slow_weights[i]);
        }
        Ok(state.slow_weights.clone())
    } else {
        Ok(fast_weights.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_lookahead() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let fast_weights = vec![1.0; 100];
        let mut state = LookaheadState {
            slow_weights: vec![0.9; 100],
            k_counter: 0,
        };
        let result = lookahead_step(&dev.device, &dev.queue, &fast_weights, &mut state, 5, 0.5).await.unwrap();
        assert_eq!(result.len(), 100);
    }
}
