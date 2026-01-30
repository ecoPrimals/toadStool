//! SGDW - SGD with Decoupled Weight Decay
//!
//! Applies weight decay separately from gradient update.
//! More principled than L2 regularization for SGD.

pub async fn sgdw_step(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    params: &[f32],
    grads: &[f32],
    momentum_buffer: Option<&mut Vec<f32>>,
    lr: f32,
    momentum: f32,
    weight_decay: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let size = params.len();
    let mut new_params = params.to_vec();
    
    if let Some(buf) = momentum_buffer {
        // SGD with momentum + decoupled weight decay
        for i in 0..size {
            buf[i] = momentum * buf[i] + grads[i];
            new_params[i] = params[i] - lr * (buf[i] + weight_decay * params[i]);
        }
    } else {
        // Plain SGD with decoupled weight decay
        for i in 0..size {
            new_params[i] = params[i] - lr * (grads[i] + weight_decay * params[i]);
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
    async fn test_sgdw() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut momentum_buf = vec![0.0; 100];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, Some(&mut momentum_buf), 0.01, 0.9, 0.0001).await.unwrap();
        assert_eq!(new_params.len(), 100);
    }
}
