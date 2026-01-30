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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_adabound() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = AdaBoundState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = adabound_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.1, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
