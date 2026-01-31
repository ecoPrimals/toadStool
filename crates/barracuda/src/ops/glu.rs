//! GLU - Gated Linear Unit activation
//!
//! ## Algorithm
//!
//! ```text
//! GLU(x) = a ⊙ sigmoid(b)
//! ```
//!
//! Where x is split into two halves: a and b.
//! Used in language models and transformers.

/// GLU activation
pub async fn glu(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32], // Length must be even
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() % 2 != 0 {
        return Err("Input length must be even for GLU".into());
    }
    
    let half = input.len() / 2;
    let mut output = Vec::with_capacity(half);
    
    for i in 0..half {
        let a = input[i];
        let b = input[half + i];
        let sigmoid_b = 1.0 / (1.0 + (-b).exp());
        output.push(a * sigmoid_b);
    }
    
    Ok(output)
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
    async fn test_glu_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 0.0, 0.0]; // Split: [1,2] and [0,0]
        let output = glu(&device, &queue, &input).await.unwrap();
        assert_eq!(output.len(), 2);
        // GLU: a * sigmoid(b) where sigmoid(0) = 0.5
        // output[0] = 1.0 * sigmoid(0.0) = 1.0 * 0.5 = 0.5
        // output[1] = 2.0 * sigmoid(0.0) = 2.0 * 0.5 = 1.0
        assert!((output[0] - 0.5).abs() < 0.01);
        assert!((output[1] - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_glu_edge_cases() {
        let dev = get_test_device().await;

        // Minimum size (2 elements)
        let input = vec![5.0, 0.0];
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 1);
        assert!((output[0] - 2.5).abs() < 0.01); // 5.0 * sigmoid(0) = 2.5

        // Large positive gate values (sigmoid → 1)
        let input = vec![3.0, 4.0, 10.0, 10.0]; // Split: [3,4] and [10,10]
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 2);
        // sigmoid(10) ≈ 1.0, so output ≈ [3, 4]
        assert!(output[0] > 2.5 && output[0] < 3.5);
        assert!(output[1] > 3.5 && output[1] < 4.5);
    }

    #[tokio::test]
    async fn test_glu_boundary() {
        let dev = get_test_device().await;

        // Negative gate values (sigmoid → 0)
        let input = vec![5.0, 6.0, -10.0, -10.0];
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 2);
        // sigmoid(-10) ≈ 0, so output ≈ [0, 0]
        assert!(output[0].abs() < 0.5);
        assert!(output[1].abs() < 0.5);

        // Mixed signs
        let input = vec![2.0, 3.0, -5.0, 5.0];
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_glu_large_tensor() {
        let dev = get_test_device().await;

        // 1000 elements (500 output)
        let input = vec![1.0; 1000];
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        assert_eq!(output.len(), 500);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_glu_precision() {
        let dev = get_test_device().await;

        // Test sigmoid gating with known values
        let input = vec![2.0, 4.0, 0.0, 0.0]; // a=[2,4], b=[0,0]
        let output = glu(&dev.device, &dev.queue, &input).await.unwrap();
        
        // sigmoid(0) = 0.5
        // output = [2*0.5, 4*0.5] = [1.0, 2.0]
        assert_eq!(output.len(), 2);
        assert!((output[0] - 1.0).abs() < 0.01);
        assert!((output[1] - 2.0).abs() < 0.01);
    }
}
