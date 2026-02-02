//! Quantize - Convert FP32 to INT8
//!
//! Quantizes floating point values to 8-bit integers.
//! Used for model compression and efficient inference.

pub async fn quantize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    scale: f32,
    zero_point: i32,
) -> Result<Vec<i8>, Box<dyn std::error::Error>> {
    let quantized: Vec<i8> = input
        .iter()
        .map(|&x| {
            let scaled = (x / scale + zero_point as f32).round();
            scaled.max(-128.0).min(127.0) as i8
        })
        .collect();

    Ok(quantized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_quantize_basic() {
        let dev = get_test_device().await;
        let input = vec![-1.0, 0.0, 1.0];
        let quantized = quantize(&dev.device, &dev.queue, &input, 0.01, 0)
            .await
            .unwrap();
        assert_eq!(quantized.len(), 3);
        // Values are i8 type (always in valid range)
    }

    #[tokio::test]
    async fn test_quantize_edge_cases() {
        let dev = get_test_device().await;

        // Test clamping at boundaries
        let input = vec![-1000.0, 1000.0, 0.0];
        let scale = 1.0;
        let zero_point = 0;
        let quantized = quantize(&dev.device, &dev.queue, &input, scale, zero_point)
            .await
            .unwrap();

        // Should clamp to INT8 range
        assert_eq!(quantized[0], -128); // Clamped to min
        assert_eq!(quantized[1], 127); // Clamped to max
        assert_eq!(quantized[2], 0); // Zero unchanged
    }

    #[tokio::test]
    async fn test_quantize_boundary() {
        let dev = get_test_device().await;

        // Test with different zero points
        let input = vec![0.0, 1.0, 2.0, 3.0];
        let scale = 0.1;

        let q1 = quantize(&dev.device, &dev.queue, &input, scale, 0)
            .await
            .unwrap();
        let q2 = quantize(&dev.device, &dev.queue, &input, scale, 50)
            .await
            .unwrap();

        // Different zero points should produce different quantizations
        assert_ne!(q1, q2);
        // i8 type guarantees all values are in valid range
    }

    #[tokio::test]
    async fn test_quantize_large_batch() {
        let dev = get_test_device().await;

        // Large tensor
        let size = 1000;
        let input: Vec<f32> = (0..size).map(|i| (i as f32 - 500.0) / 10.0).collect();
        let scale = 1.0;
        let zero_point = 0;

        let quantized = quantize(&dev.device, &dev.queue, &input, scale, zero_point)
            .await
            .unwrap();

        assert_eq!(quantized.len(), size);
        // i8 type guarantees valid range
    }

    #[tokio::test]
    async fn test_quantize_precision() {
        let dev = get_test_device().await;

        // Test round-trip with known values
        let input = vec![0.5, 1.0, 1.5, 2.0];
        let scale = 0.1;
        let zero_point = 0;

        let quantized = quantize(&dev.device, &dev.queue, &input, scale, zero_point)
            .await
            .unwrap();

        // 0.5 / 0.1 = 5, rounds to 5
        assert_eq!(quantized[0], 5);
        // 1.0 / 0.1 = 10, rounds to 10
        assert_eq!(quantized[1], 10);
        // 1.5 / 0.1 = 15, rounds to 15
        assert_eq!(quantized[2], 15);
        // 2.0 / 0.1 = 20, rounds to 20
        assert_eq!(quantized[3], 20);
    }
}
