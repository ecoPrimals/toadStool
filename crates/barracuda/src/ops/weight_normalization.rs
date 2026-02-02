//! Weight Normalization - Reparameterizes weights
//!
//! Decouples magnitude and direction of weight vectors.
//! Speeds up training convergence.

pub async fn weight_normalization(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    weights: &[f32],
    num_filters: usize,
    filter_size: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; weights.len()];

    for f in 0..num_filters {
        // Compute norm of this filter
        let mut norm = 0.0;
        for i in 0..filter_size {
            let w = weights[f * filter_size + i];
            norm += w * w;
        }
        norm = norm.sqrt();

        // Normalize
        for i in 0..filter_size {
            output[f * filter_size + i] = weights[f * filter_size + i] / (norm + 1e-8);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_weight_normalization() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let weights = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2 filters, 3 weights each
        let output = weight_normalization(&dev.device, &dev.queue, &weights, 2, 3)
            .await
            .unwrap();
        assert_eq!(output.len(), 6);
    }
}
