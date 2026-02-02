//! Tanhshrink activation
//!
//! ## Algorithm
//!
//! ```text
//! Tanhshrink(x) = x - tanh(x)
//! ```
//!
//! Residual form of tanh activation.

pub async fn tanhshrink(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let output: Vec<f32> = input.iter().map(|&x| x - x.tanh()).collect();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn tanhshrink_cpu(x: f32) -> f32 {
        x - x.tanh()
    }

    #[tokio::test]
    async fn test_tanhshrink_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        let input = vec![0.0, 1.0, 2.0];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        let expected: Vec<f32> = input.iter().map(|&x| tanhshrink_cpu(x)).collect();

        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_tanhshrink_edge_cases() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // At zero, tanhshrink(0) = 0 - tanh(0) = 0
        let input = vec![0.0];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        assert!(output[0].abs() < 1e-6);

        // Negative values
        let input = vec![-5.0, -2.0, -0.5];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        let expected: Vec<f32> = input.iter().map(|&x| tanhshrink_cpu(x)).collect();
        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_tanhshrink_boundary() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Large positive (approaches x - 1)
        let input = vec![10.0, 20.0];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        for (i, &val) in output.iter().enumerate() {
            assert!((val - (input[i] - 1.0)).abs() < 0.01);
        }

        // Large negative (approaches x + 1)
        let input = vec![-10.0, -20.0];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        for (i, &val) in output.iter().enumerate() {
            assert!((val - (input[i] + 1.0)).abs() < 0.01);
        }
    }

    #[tokio::test]
    async fn test_tanhshrink_large_tensor() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) * 0.01).collect();
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        let expected: Vec<f32> = input.iter().map(|&x| tanhshrink_cpu(x)).collect();

        for (out, exp) in output.iter().zip(expected.iter()) {
            assert!((out - exp).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_tanhshrink_precision() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Test FP32 precision
        let input = vec![-2.345, -1.234, 0.0, 1.234, 2.345];
        let output = tanhshrink(&device, &queue, &input).await.unwrap();
        let expected: Vec<f32> = input.iter().map(|&x| tanhshrink_cpu(x)).collect();

        // Verify FP32 precision
        let max_error = output
            .iter()
            .zip(expected.iter())
            .map(|(out, exp)| (out - exp).abs())
            .fold(0.0f32, f32::max);

        assert!(
            max_error < 1e-6,
            "Max error: {} exceeds threshold",
            max_error
        );
    }
}
