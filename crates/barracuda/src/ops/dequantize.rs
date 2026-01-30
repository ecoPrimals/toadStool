//! Dequantize - Convert INT8 to FP32
//!
//! Dequantizes 8-bit integers back to floating point.

pub async fn dequantize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[i8],
    scale: f32,
    zero_point: i32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let dequantized: Vec<f32> = input.iter()
        .map(|&x| (x as i32 - zero_point) as f32 * scale)
        .collect();
    
    Ok(dequantized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_dequantize_basic() {
        let dev = get_test_device().await;
        let input = vec![-100, 0, 100];
        let output = dequantize(&dev.device, &dev.queue, &input, 0.01, 0).await.unwrap();
        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_dequantize_edge_cases() {
        let dev = get_test_device().await;
        
        // Test with INT8 boundaries
        let input = vec![-128, 127, 0];
        let scale = 1.0;
        let zero_point = 0;
        let output = dequantize(&dev.device, &dev.queue, &input, scale, zero_point).await.unwrap();
        
        assert_eq!(output[0], -128.0);
        assert_eq!(output[1], 127.0);
        assert_eq!(output[2], 0.0);
    }

    #[tokio::test]
    async fn test_dequantize_boundary() {
        let dev = get_test_device().await;
        
        // Test with different zero points
        let input = vec![0, 10, 20, 30];
        let scale = 0.5;
        
        let d1 = dequantize(&dev.device, &dev.queue, &input, scale, 0).await.unwrap();
        let d2 = dequantize(&dev.device, &dev.queue, &input, scale, 10).await.unwrap();
        
        // Different zero points should produce different dequantizations
        assert_ne!(d1, d2);
        // With zero_point=0: output[0] = (0 - 0) * 0.5 = 0.0
        assert_eq!(d1[0], 0.0);
        // With zero_point=10: output[0] = (0 - 10) * 0.5 = -5.0
        assert_eq!(d2[0], -5.0);
    }

    #[tokio::test]
    async fn test_dequantize_large_batch() {
        let dev = get_test_device().await;
        
        // Large tensor
        let size = 1000;
        let input: Vec<i8> = (0..size).map(|i| (i % 256) as i8).collect();
        let scale = 0.1;
        let zero_point = 0;
        
        let output = dequantize(&dev.device, &dev.queue, &input, scale, zero_point).await.unwrap();
        
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_dequantize_precision() {
        let dev = get_test_device().await;
        
        // Test inverse of quantization
        let input = vec![5, 10, 15, 20];
        let scale = 0.1;
        let zero_point = 0;
        
        let output = dequantize(&dev.device, &dev.queue, &input, scale, zero_point).await.unwrap();
        
        // (5 - 0) * 0.1 = 0.5
        assert!((output[0] - 0.5).abs() < 1e-6);
        // (10 - 0) * 0.1 = 1.0
        assert!((output[1] - 1.0).abs() < 1e-6);
        // (15 - 0) * 0.1 = 1.5
        assert!((output[2] - 1.5).abs() < 1e-6);
        // (20 - 0) * 0.1 = 2.0
        assert!((output[3] - 2.0).abs() < 1e-6);
    }
}
