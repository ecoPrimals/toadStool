//! LAMB - Layer-wise Adaptive Moments optimizer for Batch training
//!
//! Enables large batch training (BERT with 64K batch size).
//! Combines layer-wise adaptation with Adam.

pub struct LambState {
    pub m: Vec<f32>,
    pub v: Vec<f32>,
    pub step: usize,
}

pub async fn lamb_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut LambState,
    lr: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
    weight_decay: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    state.step += 1;
    
    // Update moments
    for i in 0..size {
        state.m[i] = beta1 * state.m[i] + (1.0 - beta1) * grads[i];
        state.v[i] = beta2 * state.v[i] + (1.0 - beta2) * grads[i] * grads[i];
    }
    
    // Compute bias-corrected moments and update direction
    let mut update = vec![0.0f32; size];
    for i in 0..size {
        let m_hat = state.m[i] / (1.0 - beta1.powi(state.step as i32));
        let v_hat = state.v[i] / (1.0 - beta2.powi(state.step as i32));
        update[i] = m_hat / (v_hat.sqrt() + epsilon) + weight_decay * params[i];
    }
    
    // Compute norms for layer-wise adaptation
    let param_norm: f32 = params.iter().map(|&x| x * x).sum::<f32>().sqrt();
    let update_norm: f32 = update.iter().map(|&x| x * x).sum::<f32>().sqrt();
    
    // Compute trust ratio
    let trust_ratio = if param_norm > 0.0 && update_norm > 0.0 {
        param_norm / update_norm
    } else {
        1.0
    };
    
    // Apply update with trust ratio
    let mut new_params = vec![0.0f32; size];
    for i in 0..size {
        new_params[i] = params[i] - lr * trust_ratio * update[i];
    }
    
    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_lamb() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = LambState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = lamb_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-6, 0.01).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
