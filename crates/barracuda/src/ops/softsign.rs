//! Softsign activation
//!
//! ## Algorithm
//!
//! ```text
//! Softsign(x) = x / (1 + |x|)
//! ```
//!
//! Smooth alternative to tanh, bounded between -1 and 1.

pub async fn softsign(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = input.iter().map(|&x| x / (1.0 + x.abs())).collect();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_softsign() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = softsign(&device, &queue, &input).await.unwrap();
        assert!((output[2] - 0.0).abs() < 1e-6); // 0 -> 0
    }
}
