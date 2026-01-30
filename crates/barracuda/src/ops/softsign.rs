//! Softsign activation
//!
//! ## Algorithm
//!
//! ```text
//! Softsign(x) = x / (1 + |x|)
//! ```
//!
//! Smooth alternative to tanh, bounded between -1 and 1.

pub async fn softsign(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = input.iter().map(|&x| x / (1.0 + x.abs())).collect();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_softsign() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = softsign(&device, &queue, &input).await.unwrap();
        assert!((output[2] - 0.0).abs() < 1e-6); // 0 -> 0
    }
}
