//! NAdam - Nesterov-accelerated Adam (Dozat)
//!
//! Combines Nesterov momentum with Adam's adaptive learning rates.
//! Often faster convergence than vanilla Adam.

pub struct NAdamState {
    pub m: Vec<f32>,
    pub v: Vec<f32>,
    pub step: usize,
}

pub async fn nadam_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut NAdamState,
    lr: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    state.step += 1;
    let mut new_params = params.to_vec();
    
    let bias_correction1 = 1.0 - beta1.powi(state.step as i32);
    let bias_correction2 = 1.0 - beta2.powi(state.step as i32);
    
    for i in 0..size {
        // Update biased first moment estimate
        state.m[i] = beta1 * state.m[i] + (1.0 - beta1) * grads[i];
        
        // Update biased second moment estimate
        state.v[i] = beta2 * state.v[i] + (1.0 - beta2) * grads[i] * grads[i];
        
        // Compute bias-corrected first moment with Nesterov momentum
        let m_hat = (beta1 * state.m[i] + (1.0 - beta1) * grads[i]) / bias_correction1;
        
        // Compute bias-corrected second moment
        let v_hat = state.v[i] / bias_correction2;
        
        // Update parameters
        new_params[i] = params[i] - lr * m_hat / (v_hat.sqrt() + epsilon);
    }
    
    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_nadam_basic() {
        let dev = get_test_device().await;
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = NAdamState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 100);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // Parameters should decrease with positive gradients
        assert!(new_params[0] < params[0]);
    }

    #[tokio::test]
    async fn test_nadam_edge_cases() {
        let dev = get_test_device().await;

        // Zero gradients (no update)
        let params = vec![1.0; 10];
        let grads = vec![0.0; 10];
        let mut state = NAdamState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        let new_params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8).await.unwrap();
        // With zero grads, params should remain close to original
        assert!((new_params[0] - params[0]).abs() < 0.01);

        // Single parameter
        let params = vec![5.0];
        let grads = vec![0.1];
        let mut state = NAdamState {
            m: vec![0.0],
            v: vec![0.0],
            step: 0,
        };
        let new_params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 1);
    }

    #[tokio::test]
    async fn test_nadam_boundary() {
        let dev = get_test_device().await;

        // High learning rate
        let params = vec![1.0; 10];
        let grads = vec![0.01; 10];
        let mut state = NAdamState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        let new_params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        assert!(new_params[0] < params[0]);

        // Multiple steps
        let mut params = vec![1.0; 10];
        let grads = vec![0.01; 10];
        let mut state = NAdamState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        for _ in 0..5 {
            params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8).await.unwrap();
        }
        assert_eq!(state.step, 5);
    }

    #[tokio::test]
    async fn test_nadam_large_batch() {
        let dev = get_test_device().await;

        // Large parameter vector
        let params = vec![1.0; 10000];
        let grads = vec![0.01; 10000];
        let mut state = NAdamState {
            m: vec![0.0; 10000],
            v: vec![0.0; 10000],
            step: 0,
        };
        let new_params = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 10000);
    }

    #[tokio::test]
    async fn test_nadam_precision() {
        let dev = get_test_device().await;

        // Test momentum accumulation
        let mut params = vec![10.0; 10];
        let grads = vec![1.0; 10];
        let mut state = NAdamState {
            m: vec![0.0; 10],
            v: vec![0.0; 10],
            step: 0,
        };
        
        // First step
        let new_params1 = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.9, 0.999, 1e-8).await.unwrap();
        
        // Second step
        params = new_params1;
        let new_params2 = nadam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.01, 0.9, 0.999, 1e-8).await.unwrap();
        
        // Parameters should continue decreasing
        assert!(new_params2[0] < params[0]);
    }
}
