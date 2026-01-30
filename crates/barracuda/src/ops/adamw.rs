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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_adamw() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = AdamWState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = adamw_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8, 0.01).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
