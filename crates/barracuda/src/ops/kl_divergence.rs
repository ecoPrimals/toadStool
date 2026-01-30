//! KL Divergence - Kullback-Leibler divergence loss
//!
//! Measures difference between two probability distributions.
//! Used in VAE, distribution matching, knowledge distillation.
//!
//! ## Algorithm
//!
//! ```text
//! KL(P || Q) = Σ P(i) * log(P(i) / Q(i))
//! ```

pub async fn kl_divergence(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    predicted: &[f32], // P (predicted distribution)
    target: &[f32],    // Q (target distribution)
) -> Result<f32, Box<dyn std::error::Error>> {
    if predicted.len() != target.len() {
        return Err("Predicted and target must have same length".into());
    }
    
    let mut kl = 0.0;
    const EPSILON: f32 = 1e-10;
    
    for i in 0..predicted.len() {
        let p = predicted[i].max(EPSILON);
        let q = target[i].max(EPSILON);
        kl += p * (p / q).ln();
    }
    
    Ok(kl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_kl_divergence() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let q = vec![0.2, 0.3, 0.3, 0.2];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q).await.unwrap();
        assert!(kl >= 0.0);
    }
}
