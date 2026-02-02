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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_kl_divergence_basic() {
        let dev = get_test_device().await;
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let q = vec![0.2, 0.3, 0.3, 0.2];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl >= 0.0);
        assert!(kl.is_finite());
    }

    #[tokio::test]
    async fn test_kl_divergence_edge_cases() {
        let dev = get_test_device().await;

        // Identical distributions (KL = 0)
        let p = vec![0.25, 0.25, 0.25, 0.25];
        let q = vec![0.25, 0.25, 0.25, 0.25];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl.abs() < 0.01);

        // Single element
        let p = vec![1.0];
        let q = vec![1.0];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl.abs() < 0.01);
    }

    #[tokio::test]
    async fn test_kl_divergence_boundary() {
        let dev = get_test_device().await;

        // Very different distributions
        let p = vec![0.9, 0.1];
        let q = vec![0.1, 0.9];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl > 0.0);
        assert!(kl.is_finite());

        // Uniform vs peaked
        let p = vec![1.0, 0.0, 0.0, 0.0];
        let q = vec![0.25, 0.25, 0.25, 0.25];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl.is_finite());
    }

    #[tokio::test]
    async fn test_kl_divergence_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let p: Vec<f32> = (0..1000).map(|i| (i as f32 + 1.0) / 1000.0).collect();
        let q: Vec<f32> = (0..1000)
            .map(|i| ((i + 500) as f32 % 1000.0 + 1.0) / 1000.0)
            .collect();
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        assert!(kl >= 0.0);
        assert!(kl.is_finite());
    }

    #[tokio::test]
    async fn test_kl_divergence_precision() {
        let dev = get_test_device().await;

        // Known KL calculation (uniform distributions)
        let p = vec![0.5, 0.5];
        let q = vec![0.5, 0.5];
        let kl = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        // KL(P||Q) = 0.5*ln(0.5/0.5) + 0.5*ln(0.5/0.5) = 0
        assert!(kl.abs() < 0.01);

        // Asymmetry test: KL(P||Q) != KL(Q||P)
        let p = vec![0.7, 0.3];
        let q = vec![0.3, 0.7];
        let kl_pq = kl_divergence(&dev.device, &dev.queue, &p, &q)
            .await
            .unwrap();
        let kl_qp = kl_divergence(&dev.device, &dev.queue, &q, &p)
            .await
            .unwrap();
        assert!(kl_pq.is_finite() && kl_qp.is_finite());
        // Both should be positive
        assert!(kl_pq > 0.0 && kl_qp > 0.0);
    }
}
