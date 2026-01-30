//! MixUp - MixUp augmentation (Zhang et al.)
//!
//! Linearly interpolates between pairs of examples and labels.
//! Encourages linear behavior between classes.

pub async fn mixup(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    data1: &[f32],
    data2: &[f32],
    label1: &[f32],
    label2: &[f32],
    lambda: f32,
) -> Result<(Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    if data1.len() != data2.len() || label1.len() != label2.len() {
        return Err("Input dimensions must match".into());
    }
    
    // Mix data
    let mut mixed_data = vec![0.0f32; data1.len()];
    for i in 0..data1.len() {
        mixed_data[i] = lambda * data1[i] + (1.0 - lambda) * data2[i];
    }
    
    // Mix labels
    let mut mixed_labels = vec![0.0f32; label1.len()];
    for i in 0..label1.len() {
        mixed_labels[i] = lambda * label1[i] + (1.0 - lambda) * label2[i];
    }
    
    Ok((mixed_data, mixed_labels))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_mixup() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let data1 = vec![1.0; 100];
        let data2 = vec![0.0; 100];
        let label1 = vec![1.0, 0.0];
        let label2 = vec![0.0, 1.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.7).await.unwrap();
        assert_eq!(mixed_data.len(), 100);
        assert_eq!(mixed_labels.len(), 2);
        assert!((mixed_data[0] - 0.7).abs() < 1e-5);
    }
}
