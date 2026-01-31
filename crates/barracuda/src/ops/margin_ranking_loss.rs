//! Margin Ranking Loss - Ranking pairs of inputs
//!
//! Ensures input1 ranks higher than input2 by a margin.

pub async fn margin_ranking_loss(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input1: &[f32],
    input2: &[f32],
    target: f32, // +1 or -1
    margin: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    if input1.len() != input2.len() {
        return Err("Inputs must have same length".into());
    }
    
    let mut total_loss = 0.0;
    
    for i in 0..input1.len() {
        let loss = (- target * (input1[i] - input2[i]) + margin).max(0.0);
        total_loss += loss;
    }
    
    Ok(total_loss / input1.len() as f32)
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
    async fn test_margin_ranking_loss_basic() {
        let dev = get_test_device().await;
        let input1 = vec![2.0, 3.0, 4.0];
        let input2 = vec![1.0, 2.0, 3.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_margin_ranking_loss_edge_cases() {
        let dev = get_test_device().await;

        // Perfect ranking (loss = 0)
        let input1 = vec![5.0, 6.0, 7.0];
        let input2 = vec![1.0, 2.0, 3.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss.abs() < 0.1); // Near zero

        // Single element
        let input1 = vec![3.0];
        let input2 = vec![1.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_margin_ranking_loss_boundary() {
        let dev = get_test_device().await;

        // Negative target (input2 should rank higher)
        let input1 = vec![1.0, 2.0];
        let input2 = vec![2.0, 3.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, -1.0, 0.5).await.unwrap();
        assert!(loss >= 0.0);

        // Zero margin
        let input1 = vec![2.0, 3.0];
        let input2 = vec![1.0, 2.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.0).await.unwrap();
        assert!(loss >= 0.0);
    }

    #[tokio::test]
    async fn test_margin_ranking_loss_large_batch() {
        let dev = get_test_device().await;

        // 1000 pairs
        let input1: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let input2: Vec<f32> = (0..1000).map(|i| (i - 1) as f32).collect();
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss >= 0.0);
        assert!(loss.is_finite());
    }

    #[tokio::test]
    async fn test_margin_ranking_loss_precision() {
        let dev = get_test_device().await;

        // Known loss calculation
        // input1=2, input2=1, target=1, margin=0.5
        // loss = max(0, -(1)*(2-1) + 0.5) = max(0, -1 + 0.5) = max(0, -0.5) = 0
        let input1 = vec![2.0];
        let input2 = vec![1.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss.abs() < 0.01);

        // input1=1, input2=2, target=1, margin=0.5
        // loss = max(0, -(1)*(1-2) + 0.5) = max(0, 1 + 0.5) = 1.5
        let input1 = vec![1.0];
        let input2 = vec![2.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!((loss - 1.5).abs() < 0.01);
    }
}
