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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_dequantize() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![-100, 0, 100];
        let output = dequantize(&dev.device, &dev.queue, &input, 0.01, 0).await.unwrap();
        assert_eq!(output.len(), 3);
    }
}
