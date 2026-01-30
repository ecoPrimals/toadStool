//! Quantize - Convert FP32 to INT8
//!
//! Quantizes floating point values to 8-bit integers.
//! Used for model compression and efficient inference.

pub async fn quantize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    scale: f32,
    zero_point: i32,
) -> Result<Vec<i8>, Box<dyn std::error::Error>> {
    let quantized: Vec<i8> = input.iter()
        .map(|&x| {
            let scaled = (x / scale + zero_point as f32).round();
            scaled.max(-128.0).min(127.0) as i8
        })
        .collect();
    
    Ok(quantized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_quantize() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![-1.0, 0.0, 1.0];
        let quantized = quantize(&dev.device, &dev.queue, &input, 0.01, 0).await.unwrap();
        assert_eq!(quantized.len(), 3);
    }
}
