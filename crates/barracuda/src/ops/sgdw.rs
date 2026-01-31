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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_sgdw_basic() {
        let dev = get_test_device().await;
        let params = vec![1.0; 100];
        let grads = vec![0.01; 100];
        let mut momentum_buf = vec![0.0; 100];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, Some(&mut momentum_buf), 0.01, 0.9, 0.0001).await.unwrap();
        assert_eq!(new_params.len(), 100);
        assert!(new_params.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sgdw_edge_cases() {
        let dev = get_test_device().await;

        // Without momentum
        let params = vec![1.0; 10];
        let grads = vec![0.1; 10];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, None, 0.01, 0.9, 0.0001).await.unwrap();
        assert_eq!(new_params.len(), 10);

        // Zero gradients
        let params = vec![2.0; 10];
        let grads = vec![0.0; 10];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, None, 0.01, 0.0, 0.0001).await.unwrap();
        assert!(new_params.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sgdw_boundary() {
        let dev = get_test_device().await;

        // High weight decay
        let params = vec![1.0; 10];
        let grads = vec![0.01; 10];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, None, 0.1, 0.0, 0.1).await.unwrap();
        // With high weight decay, params should shrink more
        assert!(new_params.iter().all(|&x| x < 1.0));

        // High momentum
        let params = vec![1.0; 10];
        let grads = vec![0.1; 10];
        let mut momentum_buf = vec![0.5; 10];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, Some(&mut momentum_buf), 0.01, 0.99, 0.001).await.unwrap();
        assert!(new_params.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sgdw_large_batch() {
        let dev = get_test_device().await;

        // 10000 parameters
        let params = vec![1.0; 10000];
        let grads = vec![0.001; 10000];
        let mut momentum_buf = vec![0.0; 10000];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, Some(&mut momentum_buf), 0.01, 0.9, 0.0001).await.unwrap();
        assert_eq!(new_params.len(), 10000);
    }

    #[tokio::test]
    async fn test_sgdw_precision() {
        let dev = get_test_device().await;

        // Decoupled weight decay should apply separately
        let params = vec![10.0, 10.0];
        let grads = vec![1.0, 1.0];
        let new_params = sgdw_step(&dev.device, &dev.queue, &params, &grads, None, 0.1, 0.0, 0.01).await.unwrap();
        
        assert_eq!(new_params.len(), 2);
        assert!(new_params.iter().all(|&x| x.is_finite()));
        // Should decrease due to both gradient and weight decay
        assert!(new_params.iter().all(|&x| x < 10.0));
    }
}
