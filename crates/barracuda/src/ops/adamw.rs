//! AdamW - Adam with Decoupled Weight Decay (Loshchilov & Hutter)
//!
//! Fixes weight decay in Adam by decoupling it from gradient-based update.
//! Standard optimizer for modern transformers.

pub struct AdamWState {
    pub m: Vec<f32>,  // First moment
    pub v: Vec<f32>,  // Second moment
    pub step: usize,
}

pub async fn adamw_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut AdamWState,
    lr: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    if grads.len() != size || state.m.len() != size || state.v.len() != size {
        return Err("Dimension mismatch".into());
    }
    
    state.step += 1;
    let mut new_params = params.to_vec();
    
    for i in 0..size {
        // Update biased first moment estimate
        state.m[i] = beta1 * state.m[i] + (1.0 - beta1) * grads[i];
        
        // Update biased second moment estimate
        state.v[i] = beta2 * state.v[i] + (1.0 - beta2) * grads[i] * grads[i];
        
        // Bias correction
        let m_hat = state.m[i] / (1.0 - beta1.powi(state.step as i32));
        let v_hat = state.v[i] / (1.0 - beta2.powi(state.step as i32));
        
        // Update with decoupled weight decay
        new_params[i] = params[i] - lr * (m_hat / (v_hat.sqrt() + epsilon) + weight_decay * params[i]);
    }
    
    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_adamw_basic() {
        let dev = get_test_device().await;
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = AdamWState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8, 0.01).await.unwrap();
        assert_eq!(new_params.len(), 100);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // Params should decrease (gradient + weight decay)
        assert!(new_params.iter().zip(params.iter()).all(|(a, b)| a < b));
    }

    #[tokio::test]
    async fn test_adamw_edge_cases() {
        let dev = get_test_device().await;
        
        // Test with zero weight decay (should behave like Adam)
        let params = vec![1.0; 10];
        let grads = vec![0.1; 10];
        let mut state = AdamWState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        let new_params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.9, 0.999, 1e-8, 0.0).await.unwrap();
        assert!(new_params.iter().all(|&x| x.is_finite()));
        
        // Test with zero gradients (only weight decay)
        let params = vec![10.0; 10];
        let grads = vec![0.0; 10];
        let mut state = AdamWState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        let new_params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.9, 0.999, 1e-8, 0.1).await.unwrap();
        // Should still decrease due to weight decay
        assert!(new_params[0] < 10.0);
    }

    #[tokio::test]
    async fn test_adamw_boundary() {
        let dev = get_test_device().await;
        
        // Test with different weight decay values
        let params1 = vec![1.0; 50];
        let params2 = vec![1.0; 50];
        let grads = vec![0.05; 50];
        
        let mut state1 = AdamWState {
            m: vec![0.0; 50],
            v: vec![0.0; 50],
            step: 0,
        };
        
        let mut state2 = AdamWState {
            m: vec![0.0; 50],
            v: vec![0.0; 50],
            step: 0,
        };
        
        // Small weight decay
        let new_params1 = adamw_step(&dev.device, &dev.queue, &params1, &grads, &mut state1, 0.001, 0.9, 0.999, 1e-8, 0.001).await.unwrap();
        
        // Large weight decay
        let new_params2 = adamw_step(&dev.device, &dev.queue, &params2, &grads, &mut state2, 0.001, 0.9, 0.999, 1e-8, 0.1).await.unwrap();
        
        // Both should decrease, but larger decay should decrease more
        assert!(new_params1.iter().all(|&x| x.is_finite()));
        assert!(new_params2.iter().all(|&x| x.is_finite()));
        assert!(new_params2[0] < new_params1[0]);
    }

    #[tokio::test]
    async fn test_adamw_large_batch() {
        let dev = get_test_device().await;
        
        // Larger parameter set (transformer-style)
        let size = 512;
        let params: Vec<f32> = (0..size).map(|i| (i as f32) / 100.0).collect();
        let grads = vec![0.01; size];
        let mut state = AdamWState {
            m: vec![0.0; size],
            v: vec![0.0; size],
            step: 0,
        };
        
        let new_params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8, 0.01).await.unwrap();
        
        assert_eq!(new_params.len(), size);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        assert_eq!(state.step, 1);
        assert!(state.m.iter().any(|&x| x != 0.0));
        assert!(state.v.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_adamw_precision() {
        let dev = get_test_device().await;
        
        // Test decoupled weight decay (key feature of AdamW)
        let mut params = vec![10.0, 20.0, 30.0];
        let grads = vec![1.0, 2.0, 3.0];
        let mut state = AdamWState {
            m: vec![0.0; 3],
            v: vec![0.0; 3],
            step: 0,
        };
        
        // Step 1
        params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.1, 0.9, 0.999, 1e-8, 0.01).await.unwrap();
        assert!(params.iter().all(|&x| x.is_finite()));
        assert!(params[0] < 10.0);
        assert!(params[1] < 20.0);
        assert!(params[2] < 30.0);
        
        // Step 2 (momentum + weight decay accumulated)
        let params_step2 = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.1, 0.9, 0.999, 1e-8, 0.01).await.unwrap();
        assert!(params_step2.iter().all(|&x| x.is_finite()));
        // Should continue decreasing
        assert!(params_step2[0] < params[0]);
    }
}
