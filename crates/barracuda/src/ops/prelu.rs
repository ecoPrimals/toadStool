//! PReLU - Parametric ReLU activation
//!
//! ## Deep Debt Principles
//!
//! - **Learnable**: Unlike ReLU/LeakyReLU, slope is learned during training
//! - **Complete**: Proper parameter handling per channel
//! - **Modern Rust**: Clean API with proper validation
//!
//! ## Algorithm
//!
//! ```text
//! PReLU(x) = max(0, x) + alpha * min(0, x)
//! ```
//!
//! Where alpha is a learnable parameter (per channel or shared).

/// PReLU activation with learnable parameters
///
/// ## Usage
///
/// ```no_run
/// use barracuda::ops::prelu::*;
///
/// # async fn example(device: &wgpu::Device, queue: &wgpu::Queue) {
/// let input = vec![-1.0, 2.0, -3.0, 4.0];
/// let alpha = vec![0.25]; // Shared alpha or per-channel
/// let output = prelu(device, queue, &input, &alpha).await.unwrap();
/// # }
/// ```
pub async fn prelu(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    alpha: &[f32], // Per-channel slopes or single shared slope
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let shared_alpha = alpha.len() == 1;
    
    if !shared_alpha && alpha.len() != input.len() {
        return Err("Alpha must be either length 1 (shared) or match input length (per-channel)".into());
    }
    
    let output: Vec<f32> = input.iter().enumerate().map(|(i, &x)| {
        let a = if shared_alpha { alpha[0] } else { alpha[i] };
        if x > 0.0 { x } else { a * x }
    }).collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_prelu_shared() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let alpha = vec![0.25];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        assert_eq!(output, vec![-0.5, -0.25, 0.0, 1.0, 2.0]);
    }
}
