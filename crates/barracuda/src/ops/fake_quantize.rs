//! Fake Quantize - Simulate quantization in training
//!
//! Quantizes then dequantizes (stays in FP32).
//! Used for quantization-aware training.

pub async fn fake_quantize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    scale: f32,
    zero_point: i32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simplified inline implementation
    let quantized: Vec<i8> = input.iter()
        .map(|&x| {
            let scaled = (x / scale + zero_point as f32).round();
            scaled.max(-128.0).min(127.0) as i8
        })
        .collect();
    
    let dequantized: Vec<f32> = quantized.iter()
        .map(|&x| (x as i32 - zero_point) as f32 * scale)
        .collect();
    
    Ok(dequantized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_fake_quantize_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0];
        let output = fake_quantize(&dev.device, &dev.queue, &input, 0.1, 0).await.unwrap();
        assert_eq!(output.len(), 3);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_fake_quantize_edge_cases() {
        let dev = get_test_device().await;
        
        // Zero values
        let input = vec![0.0, 0.0, 0.0];
        let output = fake_quantize(&dev.device, &dev.queue, &input, 0.1, 0).await.unwrap();
        assert!(output.iter().all(|&x| x.abs() < 0.2));
        
        // Small values (should quantize to near zero)
        let input = vec![0.01, 0.02, 0.03];
        let output = fake_quantize(&dev.device, &dev.queue, &input, 0.1, 0).await.unwrap();
        assert!(output.iter().all(|&x| x.abs() < 0.2));
    }

    #[tokio::test]
    async fn test_fake_quantize_boundary() {
        let dev = get_test_device().await;
        
        // Values at quantization boundaries
        let scale = 0.1;
        let input = vec![-12.8, -12.7, 0.0, 12.7, 12.8]; // Near int8 limits
        let output = fake_quantize(&dev.device, &dev.queue, &input, scale, 0).await.unwrap();
        
        // Should clamp to [-128, 127] * scale = [-12.8, 12.7]
        assert!(output.iter().all(|&x| x >= -12.8 && x <= 12.7));
        
        // Test with non-zero zero_point
        let output = fake_quantize(&dev.device, &dev.queue, &input, scale, 50).await.unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_fake_quantize_large_batch() {
        let dev = get_test_device().await;
        
        // Large tensor (typical model weights)
        let size = 1000;
        let input: Vec<f32> = (0..size).map(|i| (i as f32) * 0.01 - 5.0).collect();
        let scale = 0.05;
        
        let output = fake_quantize(&dev.device, &dev.queue, &input, scale, 0).await.unwrap();
        assert_eq!(output.len(), size);
        assert!(output.iter().all(|&x| x.is_finite()));
        
        // Check quantization introduces rounding
        for i in 0..10 {
            let diff = (input[i] - output[i]).abs();
            assert!(diff < scale); // Error within one quantization step
        }
    }

    #[tokio::test]
    async fn test_fake_quantize_precision() {
        let dev = get_test_device().await;
        
        // Test round-trip quantization is deterministic
        let scale = 0.1;
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let output1 = fake_quantize(&dev.device, &dev.queue, &input, scale, 0).await.unwrap();
        let output2 = fake_quantize(&dev.device, &dev.queue, &input, scale, 0).await.unwrap();
        
        // Should be deterministic
        for (i, (&a, &b)) in output1.iter().zip(output2.iter()).enumerate() {
            assert_eq!(a, b, "Quantization not deterministic at index {}", i);
        }
        
        // Test that all outputs are finite
        assert!(output1.iter().all(|&x| x.is_finite()));
        
        // Test output is within reasonable range of input
        for (i, &out_val) in output1.iter().enumerate() {
            let diff = (input[i] - out_val).abs();
            assert!(diff < 1.0, "Quantized value {} differs too much from input {}", out_val, input[i]);
        }
        
        // Test that quantization works with different scales
        let output_large_scale = fake_quantize(&dev.device, &dev.queue, &input, 1.0, 0).await.unwrap();
        assert!(output_large_scale.iter().all(|&x| x.is_finite()));
    }
}
