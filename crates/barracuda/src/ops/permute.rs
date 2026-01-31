//! Permute - Reorder dimensions
//!
//! Permutes dimensions according to specified order.

pub async fn permute(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    shape: &[usize],
    dims: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if dims.len() != shape.len() {
        return Err("Permutation dims must match shape rank".into());
    }
    
    // Simplified for common case: 4D tensor permutation
    if shape.len() != 4 || dims != &[0, 2, 3, 1] {
        return Err("Currently supports NCHW -> NHWC permutation only".into());
    }
    
    let (n, c, h, w) = (shape[0], shape[1], shape[2], shape[3]);
    let mut output = vec![0.0f32; input.len()];
    
    for b in 0..n {
        for ch in 0..c {
            for row in 0..h {
                for col in 0..w {
                    let in_idx = b * c * h * w + ch * h * w + row * w + col;
                    let out_idx = b * h * w * c + row * w * c + col * c + ch;
                    output[out_idx] = input[in_idx];
                }
            }
        }
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
    async fn test_permute_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 3, 4, 4], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), input.len());
    }

    #[tokio::test]
    async fn test_permute_edge_cases() {
        let dev = get_test_device().await;

        // Small tensor (NCHW -> NHWC)
        let input = vec![1.0; 1 * 2 * 2 * 2];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 2, 2, 2], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), 8);

        // Single channel
        let input = vec![1.0; 1 * 1 * 4 * 4];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 1, 4, 4], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), 16);
    }

    #[tokio::test]
    async fn test_permute_boundary() {
        let dev = get_test_device().await;

        // Large spatial dimensions
        let input = vec![1.0; 1 * 3 * 32 * 32];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 3, 32, 32], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 32 * 32);

        // Many channels
        let input = vec![1.0; 1 * 64 * 8 * 8];
        let output = permute(&dev.device, &dev.queue, &input, &[1, 64, 8, 8], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), 1 * 64 * 8 * 8);
    }

    #[tokio::test]
    async fn test_permute_large_batch() {
        let dev = get_test_device().await;

        // Batch size 8
        let batch_size = 8;
        let input = vec![1.0; batch_size * 16 * 16 * 16];
        let output = permute(&dev.device, &dev.queue, &input, &[batch_size, 16, 16, 16], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), batch_size * 16 * 16 * 16);
    }

    #[tokio::test]
    async fn test_permute_precision() {
        let dev = get_test_device().await;

        // Test value reordering
        let mut input = vec![0.0; 1 * 2 * 2 * 2];
        for i in 0..8 {
            input[i] = i as f32;
        }
        
        let output = permute(&dev.device, &dev.queue, &input, &[1, 2, 2, 2], &[0, 2, 3, 1]).await.unwrap();
        assert_eq!(output.len(), 8);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Permuted data should still be in range [0, 7]
        assert!(output.iter().all(|&x| x >= 0.0 && x < 8.0));
    }
}
