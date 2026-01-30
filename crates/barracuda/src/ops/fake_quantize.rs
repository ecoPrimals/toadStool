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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_fake_quantize() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0];
        let output = fake_quantize(&dev.device, &dev.queue, &input, 0.1, 0).await.unwrap();
        assert_eq!(output.len(), 3);
    }
}
