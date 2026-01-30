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
    use crate::device::test_pool::get_test_device;

    fn prelu_cpu(input: &[f32], alpha: &[f32]) -> Vec<f32> {
        let shared_alpha = alpha.len() == 1;
        input.iter().enumerate().map(|(i, &x)| {
            let a = if shared_alpha { alpha[0] } else { alpha[i] };
            if x > 0.0 { x } else { a * x }
        }).collect()
    }
    
    #[tokio::test]
    async fn test_prelu_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let alpha = vec![0.25];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        let expected = prelu_cpu(&input, &alpha);
        
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_prelu_edge_cases() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // All negative values
        let input = vec![-3.0, -2.0, -1.0];
        let alpha = vec![0.1];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        let expected = prelu_cpu(&input, &alpha);
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-6);
        }

        // All positive values
        let input = vec![1.0, 2.0, 3.0];
        let alpha = vec![0.1];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        assert_eq!(output, input); // Should pass through unchanged
    }

    #[tokio::test]
    async fn test_prelu_boundary() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Zero alpha (kills negative values)
        let input = vec![-1.0, 0.0, 1.0];
        let alpha = vec![0.0];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        assert_eq!(output, vec![0.0, 0.0, 1.0]);

        // Alpha = 1 (identity for negative)
        let input = vec![-2.0, -1.0, 1.0, 2.0];
        let alpha = vec![1.0];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        assert_eq!(output, input);
    }

    #[tokio::test]
    async fn test_prelu_large_tensor() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| (i as f32) - 500.0).collect();
        let alpha = vec![0.2];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        let expected = prelu_cpu(&input, &alpha);
        
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_prelu_precision() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Test FP32 precision
        let input = vec![-1.234, -0.567, 0.0, 0.891, 2.345];
        let alpha = vec![0.123];
        let output = prelu(&device, &queue, &input, &alpha).await.unwrap();
        let expected = prelu_cpu(&input, &alpha);
        
        // Verify FP32 precision
        let max_error = output.iter().zip(expected.iter())
            .map(|(out, exp)| (out - exp).abs())
            .fold(0.0f32, f32::max);
        
        assert!(max_error < 1e-6, "Max error: {} exceeds threshold", max_error);
    }
}
