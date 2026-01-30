//! LayerScale - Per-layer learnable scaling
//!
//! Used in vision transformers (CaiT, LeViT) to stabilize training.
//!
//! ## Algorithm
//!
//! ```text
//! LayerScale(x) = gamma ⊙ x
//! ```
//!
//! Where gamma is a learnable per-channel parameter.

pub async fn layer_scale(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    gamma: &[f32], // Per-channel scaling factors
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if gamma.len() != input.len() {
        return Err("Gamma must match input length".into());
    }
    
    let output: Vec<f32> = input.iter().zip(gamma.iter())
        .map(|(&x, &g)| x * g).collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_layer_scale() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 3.0];
        let gamma = vec![0.1, 0.2, 0.3];
        let output = layer_scale(&device, &queue, &input, &gamma).await.unwrap();
        assert_eq!(output, vec![0.1, 0.4, 0.9]);
    }
}
