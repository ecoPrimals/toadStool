//! Tanhshrink activation
//!
//! ## Algorithm
//!
//! ```text
//! Tanhshrink(x) = x - tanh(x)
//! ```
//!
//! Residual form of tanh activation.

pub async fn tanhshrink(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = input.iter().map(|&x| x - x.tanh()).collect();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_tanhshrink() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![0.0, 1.0, 2.0];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        assert!((output[0] - 0.0).abs() < 1e-6);
    }
}
