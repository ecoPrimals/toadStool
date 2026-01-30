//! Adafactor - Memory-efficient adaptive learning rate method
//!
//! Reduces memory by factorizing second moment matrix.
//! Used in T5 and large-scale training.

pub struct AdafactorState {
    pub row_mean: Vec<f32>,
    pub col_mean: Vec<f32>,
    pub step: usize,
}

pub async fn adafactor_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    state: &mut AdafactorState,
    lr: f32,
    beta2: f32,
    epsilon: f32,
    rows: usize,
    cols: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if params.len() != rows * cols || grads.len() != rows * cols {
        return Err("Dimension mismatch".into());
    }
    
    state.step += 1;
    
    // Update factorized second moment
    for r in 0..rows {
        let mut row_sum = 0.0;
        for c in 0..cols {
            let g = grads[r * cols + c];
            row_sum += g * g;
        }
        state.row_mean[r] = beta2 * state.row_mean[r] 
                          + (1.0 - beta2) * row_sum / cols as f32;
    }
    
    for c in 0..cols {
        let mut col_sum = 0.0;
        for r in 0..rows {
            let g = grads[r * cols + c];
            col_sum += g * g;
        }
        state.col_mean[c] = beta2 * state.col_mean[c]
                          + (1.0 - beta2) * col_sum / rows as f32;
    }
    
    // Update parameters using factorized approximation
    let mut new_params = params.to_vec();
    for r in 0..rows {
        for c in 0..cols {
            let idx = r * cols + c;
            // Approximate v[i] as outer product of row and col means
            let v_approx = state.row_mean[r] * state.col_mean[c];
            let rms = (v_approx + epsilon).sqrt();
            new_params[idx] = params[idx] - lr * grads[idx] / rms;
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
    async fn test_adafactor() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 10 * 10];
        let grads = vec![0.01; 10 * 10];
        let mut state = AdafactorState {
            row_mean: vec![0.0; 10],
            col_mean: vec![0.0; 10],
            step: 0,
        };
        let new_params = adafactor_step(&dev.device, &dev.queue, &params, &grads, &mut state, 0.001, 0.999, 1e-8, 10, 10).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
