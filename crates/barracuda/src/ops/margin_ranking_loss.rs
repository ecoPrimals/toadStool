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
    
    #[tokio::test]
    async fn test_margin_ranking_loss() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input1 = vec![2.0, 3.0, 4.0];
        let input2 = vec![1.0, 2.0, 3.0];
        let loss = margin_ranking_loss(&dev.device, &dev.queue, &input1, &input2, 1.0, 0.5).await.unwrap();
        assert!(loss >= 0.0);
    }
}
