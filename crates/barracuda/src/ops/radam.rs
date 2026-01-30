//! RAdam - Rectified Adam (Liu et al.)
//!
//! Addresses variance warmup issue in Adam.
//! Automatically adjusts learning rate based on variance.

pub struct RAdamState {
    pub m: Vec<f32>,
    pub v: Vec<f32>,
    pub step: usize,
}

pub async fn radam_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut RAdamState,
    lr: f32,
    beta1: f32,
    beta2: f32,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    state.step += 1;
    let mut new_params = params.to_vec();
    
    // Compute maximum length of approximated SMA (Simple Moving Average)
    let rho_inf = 2.0 / (1.0 - beta2) - 1.0;
    
    // Compute current approximated SMA length
    let rho_t = rho_inf - 2.0 * (state.step as f32) 
                * beta2.powi(state.step as i32) 
                / (1.0 - beta2.powi(state.step as i32));
    
    for i in 0..size {
        state.m[i] = beta1 * state.m[i] + (1.0 - beta1) * grads[i];
        state.v[i] = beta2 * state.v[i] + (1.0 - beta2) * grads[i] * grads[i];
        
        let m_hat = state.m[i] / (1.0 - beta1.powi(state.step as i32));
        
        if rho_t > 5.0 {
            // Variance is tractable, use adaptive learning rate
            let v_hat = state.v[i] / (1.0 - beta2.powi(state.step as i32));
            let r = ((rho_t - 4.0) * (rho_t - 2.0) * rho_inf 
                   / ((rho_inf - 4.0) * (rho_inf - 2.0) * rho_t)).sqrt();
            new_params[i] = params[i] - lr * r * m_hat / (v_hat.sqrt() + epsilon);
        } else {
            // Variance not tractable, use unadapted step
            new_params[i] = params[i] - lr * m_hat;
        }
    }
    
    Ok(new_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_radam() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut state = RAdamState {
            m: vec![0.0; 100],
            v: vec![0.0; 100],
            step: 0,
        };
        let new_params = radam_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.9, 0.999, 1e-8).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
