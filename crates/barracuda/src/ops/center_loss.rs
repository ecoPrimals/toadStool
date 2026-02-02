//! CenterLoss - Center loss for face recognition (Wen et al.)
//!
//! Minimizes intra-class variance by learning class centers.
//! Improves feature discriminability.

pub async fn center_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    features: &[f32], // [batch_size, feature_dim]
    labels: &[usize], // [batch_size]
    centers: &[f32],  // [num_classes, feature_dim]
    batch_size: usize,
    feature_dim: usize,
    num_classes: usize,
) -> Result<f32, Box<dyn std::error::Error>> {
    if features.len() != batch_size * feature_dim {
        return Err("Features dimension mismatch".into());
    }

    if labels.len() != batch_size {
        return Err("Labels dimension mismatch".into());
    }

    if centers.len() != num_classes * feature_dim {
        return Err("Centers dimension mismatch".into());
    }

    let mut loss = 0.0;

    // Compute distance from each feature to its class center
    for i in 0..batch_size {
        let label = labels[i];
        if label >= num_classes {
            return Err("Label out of bounds".into());
        }

        // Distance to class center
        for f in 0..feature_dim {
            let feat = features[i * feature_dim + f];
            let center = centers[label * feature_dim + f];
            let diff = feat - center;
            loss += diff * diff;
        }
    }

    // Average over batch
    loss /= batch_size as f32;

    Ok(loss)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_center_loss_basic() {
        let dev = get_test_device().await;
        let features = vec![0.5; 32 * 128]; // 32 samples, 128 features
        let labels = vec![
            0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0,
            1, 0, 1,
        ];
        let centers = vec![0.3; 2 * 128]; // 2 classes
        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            32,
            128,
            2,
        )
        .await
        .unwrap();
        assert!(loss > 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_center_loss_edge_cases() {
        let dev = get_test_device().await;

        // Perfect match (features = centers)
        let features = vec![1.0; 4 * 10];
        let labels = vec![0, 0, 1, 1];
        let centers = vec![1.0; 2 * 10];
        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            4,
            10,
            2,
        )
        .await
        .unwrap();
        assert!(loss.abs() < 1e-6); // Should be near zero

        // Single sample
        let features = vec![0.5; 8];
        let labels = vec![0];
        let centers = vec![0.3; 1 * 8];
        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            1,
            8,
            1,
        )
        .await
        .unwrap();
        assert!(loss > 0.0);
    }

    #[tokio::test]
    async fn test_center_loss_boundary() {
        let dev = get_test_device().await;

        // Different class centers
        let features = vec![1.0; 8 * 5];
        let labels = vec![0, 0, 1, 1, 2, 2, 3, 3];
        let mut centers = vec![0.0; 4 * 5];
        // Set different centers for each class
        for c in 0..4 {
            for f in 0..5 {
                centers[c * 5 + f] = c as f32;
            }
        }

        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            8,
            5,
            4,
        )
        .await
        .unwrap();
        assert!(loss > 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_center_loss_large_batch() {
        let dev = get_test_device().await;

        // Large batch size
        let batch_size = 128;
        let feature_dim = 64;
        let num_classes = 10;

        let features = vec![0.5; batch_size * feature_dim];
        let labels: Vec<usize> = (0..batch_size).map(|i| i % num_classes).collect();
        let centers = vec![0.3; num_classes * feature_dim];

        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            batch_size,
            feature_dim,
            num_classes,
        )
        .await
        .unwrap();
        assert!(loss > 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_center_loss_precision() {
        let dev = get_test_device().await;

        // Test with known values
        let features = vec![
            1.0, 2.0, // Sample 0, class 0
            3.0, 4.0, // Sample 1, class 1
        ];
        let labels = vec![0, 1];
        let centers = vec![
            0.0, 0.0, // Center for class 0
            2.0, 2.0, // Center for class 1
        ];

        let loss = center_loss(
            &dev.device,
            &dev.queue,
            &features,
            &labels,
            &centers,
            2,
            2,
            2,
        )
        .await
        .unwrap();

        // Sample 0: distance to (0,0) = (1^2 + 2^2) = 5
        // Sample 1: distance to (2,2) = (1^2 + 2^2) = 5
        // Average = (5 + 5) / 2 = 5.0
        assert!((loss - 5.0).abs() < 0.01);
    }
}
