//! AdaBound - Adaptive learning rate with dynamic bounds (Luo et al.)
//!
//! Transforms from Adam-like to SGD-like learning rate.
//! Achieves Adam's fast early training + SGD's good generalization.

pub struct AdaBoundState {
    pub m: Vec<f32>,
    pub v: Vec<f32>,
    pub step: usize,
}

pub async fn adabound_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut AdaBoundState,
    lr: f32,
    final_lr: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    state.step += 1;
    let mut new_params = params.to_vec();
    
    // Compute dynamic bounds
    let gamma = 0.1;  // Convergence speed
    let lower_bound = final_lr * (1.0 - 1.0 / (gamma * state.step as f32 + 1.0));
    let upper_bound = final_lr * (1.0 + 1.0 / (gamma * state.step as f32));
    
    for i in 0..size {
        // Update moments
        state.m[i] = beta1 * state.m[i] + (1.0 - beta1) * grads[i];
        state.v[i] = beta2 * state.v[i] + (1.0 - beta2) * grads[i] * grads[i];
        
        // Bias correction
        let m_hat = state.m[i] / (1.0 - beta1.powi(state.step as i32));
        let v_hat = state.v[i] / (1.0 - beta2.powi(state.step as i32));
        
        // Compute step size with bounds
        let step_size = lr / (v_hat.sqrt() + epsilon);
        let clipped_lr = step_size.max(lower_bound).min(upper_bound);
        
        new_params[i] = params[i] - clipped_lr * m_hat;
    }
    
    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_adabound_basic() {
        let dev = get_test_device().await;
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = AdaBoundState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 100);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // Params should decrease with positive gradients
        assert!(new_params.iter().zip(params.iter()).all(|(a, b)| a < b));
    }

    #[tokio::test]
    async fn test_adabound_edge_cases() {
        let dev = get_test_device().await;
        
        // Test with zero gradients
        let params = vec![1.0; 10];
        let grads = vec![0.0; 10];
        let mut state = AdaBoundState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        let new_params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        assert!(new_params.iter().all(|&x| x.is_finite()));
        
        // Test with single parameter
        let params = vec![5.0];
        let grads = vec![0.1];
        let mut state = AdaBoundState {
            m: vec![0.0],
            v: vec![0.0],
            step: 0,
        };
        let new_params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 1);
        assert!(new_params[0] < 5.0);
    }

    #[tokio::test]
    async fn test_adabound_boundary() {
        let dev = get_test_device().await;
        
        // Test dynamic bounds convergence (early vs late training)
        let params = vec![1.0; 50];
        let grads = vec![0.1; 50];
        
        // Early training (step 1) - should behave like Adam
        let mut state_early = AdaBoundState {
            m: vec![0.0; 50],
            v: vec![0.0; 50],
            step: 0,
        };
        let new_params_early = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state_early, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        
        // Late training (step 1000) - bounds should be tighter
        let mut state_late = AdaBoundState {
            m: vec![0.05; 50],
            v: vec![0.005; 50],
            step: 999, // Will become 1000
        };
        let new_params_late = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state_late, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        
        // Both should produce valid updates
        assert!(new_params_early.iter().all(|&x| x.is_finite()));
        assert!(new_params_late.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_adabound_large_batch() {
        let dev = get_test_device().await;
        
        // Larger parameter set (neural network layer)
        let size = 512;
        let params: Vec<f32> = (0..size).map(|i| (i as f32) / 100.0).collect();
        let grads = vec![0.01; size];
        let mut state = AdaBoundState {
            m: vec![0.0; size],
            v: vec![0.0; size],
            step: 0,
        };
        
        let new_params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        
        assert_eq!(new_params.len(), size);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // State should be updated
        assert_eq!(state.step, 1);
        assert!(state.m.iter().any(|&x| x != 0.0));
        assert!(state.v.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_adabound_precision() {
        let dev = get_test_device().await;
        
        // Test multiple optimization steps
        let mut params = vec![10.0, 20.0, 30.0];
        let grads = vec![1.0, 2.0, 3.0];
        let mut state = AdaBoundState {
            m: vec![0.0; 3],
            v: vec![0.0; 3],
            step: 0,
        };
        
        // Step 1
        params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.1, 1.0, 0.9, 0.999, 1e-8).await.unwrap();
        assert!(params.iter().all(|&x| x.is_finite()));
        assert!(params[0] < 10.0);
        assert!(params[1] < 20.0);
        assert!(params[2] < 30.0);
        
        // Step 2 (momentum accumulated)
        let params_step2 = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.1, 1.0, 0.9, 0.999, 1e-8).await.unwrap();
        assert!(params_step2.iter().all(|&x| x.is_finite()));
        // Should continue decreasing
        assert!(params_step2[0] < params[0]);
    }
}
