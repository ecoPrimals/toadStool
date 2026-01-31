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
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_mixup_basic() {
        let dev = get_test_device().await;
        let data1 = vec![1.0; 100];
        let data2 = vec![0.0; 100];
        let label1 = vec![1.0, 0.0];
        let label2 = vec![0.0, 1.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.7).await.unwrap();
        assert_eq!(mixed_data.len(), 100);
        assert_eq!(mixed_labels.len(), 2);
        assert!((mixed_data[0] - 0.7).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_mixup_edge_cases() {
        let dev = get_test_device().await;

        // Lambda = 1.0 (all data1)
        let data1 = vec![2.0; 10];
        let data2 = vec![1.0; 10];
        let label1 = vec![1.0];
        let label2 = vec![0.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 1.0).await.unwrap();
        assert!((mixed_data[0] - 2.0).abs() < 1e-5);
        assert!((mixed_labels[0] - 1.0).abs() < 1e-5);

        // Lambda = 0.0 (all data2)
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.0).await.unwrap();
        assert!((mixed_data[0] - 1.0).abs() < 1e-5);
        assert!((mixed_labels[0] - 0.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_mixup_boundary() {
        let dev = get_test_device().await;

        // Lambda = 0.5 (equal mix)
        let data1 = vec![4.0; 10];
        let data2 = vec![2.0; 10];
        let label1 = vec![1.0, 0.0];
        let label2 = vec![0.0, 1.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.5).await.unwrap();
        assert!((mixed_data[0] - 3.0).abs() < 1e-5);
        assert!((mixed_labels[0] - 0.5).abs() < 1e-5);
        assert!((mixed_labels[1] - 0.5).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_mixup_large_batch() {
        let dev = get_test_device().await;

        // Large image data
        let data1 = vec![1.0; 3 * 224 * 224];
        let data2 = vec![0.0; 3 * 224 * 224];
        let label1 = vec![1.0, 0.0, 0.0];
        let label2 = vec![0.0, 1.0, 0.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.3).await.unwrap();
        assert_eq!(mixed_data.len(), 3 * 224 * 224);
        assert_eq!(mixed_labels.len(), 3);
    }

    #[tokio::test]
    async fn test_mixup_precision() {
        let dev = get_test_device().await;

        // Test interpolation accuracy
        let data1 = vec![10.0; 5];
        let data2 = vec![5.0; 5];
        let label1 = vec![1.0];
        let label2 = vec![0.0];
        let (mixed_data, mixed_labels) = mixup(&dev.device, &dev.queue, &data1, &data2, &label1, &label2, 0.6).await.unwrap();
        
        // 0.6 * 10.0 + 0.4 * 5.0 = 6.0 + 2.0 = 8.0
        assert!((mixed_data[0] - 8.0).abs() < 1e-5);
        // 0.6 * 1.0 + 0.4 * 0.0 = 0.6
        assert!((mixed_labels[0] - 0.6).abs() < 1e-5);
    }
}
