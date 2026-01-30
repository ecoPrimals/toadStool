//! CenterLoss - Center loss for face recognition (Wen et al.)
//!
//! Minimizes intra-class variance by learning class centers.
//! Improves feature discriminability.

pub async fn center_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    features: &[f32],     // [batch_size, feature_dim]
    labels: &[usize],     // [batch_size]
    centers: &[f32],      // [num_classes, feature_dim]
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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_center_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let features = vec![0.5; 32 * 128]; // 32 samples, 128 features
        let labels = vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 
                         0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
        let centers = vec![0.3; 2 * 128]; // 2 classes
        let loss = center_loss(&dev.device, &dev.queue, &features, &labels, &centers, 32, 128, 2).await.unwrap();
        assert!(loss > 0.0);
    }
}
