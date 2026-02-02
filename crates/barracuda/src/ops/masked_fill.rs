//! MaskedFill - Conditional fill operation
//!
//! Fills elements where mask is true with a specified value.

pub async fn masked_fill(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    mask: &[bool],
    fill_value: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() != mask.len() {
        return Err("Input and mask must have same length".into());
    }

    let output: Vec<f32> = input
        .iter()
        .zip(mask.iter())
        .map(|(&x, &m)| if m { fill_value } else { x })
        .collect();

    Ok(output)
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
    async fn test_masked_fill_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mask = vec![false, true, false, true, false];
        let output = masked_fill(&device, &queue, &input, &mask, -999.0)
            .await
            .unwrap();
        assert_eq!(output, vec![1.0, -999.0, 3.0, -999.0, 5.0]);
    }

    #[tokio::test]
    async fn test_masked_fill_edge_cases() {
        let dev = get_test_device().await;

        // All masked
        let input = vec![1.0, 2.0, 3.0];
        let mask = vec![true, true, true];
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, 0.0)
            .await
            .unwrap();
        assert_eq!(output, vec![0.0, 0.0, 0.0]);

        // None masked
        let input = vec![1.0, 2.0, 3.0];
        let mask = vec![false, false, false];
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, 999.0)
            .await
            .unwrap();
        assert_eq!(output, vec![1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn test_masked_fill_boundary() {
        let dev = get_test_device().await;

        // Fill with infinity
        let input = vec![1.0, 2.0];
        let mask = vec![true, false];
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, f32::NEG_INFINITY)
            .await
            .unwrap();
        assert_eq!(output[0], f32::NEG_INFINITY);
        assert_eq!(output[1], 2.0);

        // Single element
        let input = vec![42.0];
        let mask = vec![true];
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, 0.0)
            .await
            .unwrap();
        assert_eq!(output, vec![0.0]);
    }

    #[tokio::test]
    async fn test_masked_fill_large_batch() {
        let dev = get_test_device().await;

        // 1000 elements
        let input: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let mask: Vec<bool> = (0..1000).map(|i| i % 2 == 0).collect();
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, -1.0)
            .await
            .unwrap();

        assert_eq!(output.len(), 1000);
        for i in 0..1000 {
            if i % 2 == 0 {
                assert_eq!(output[i], -1.0);
            } else {
                assert_eq!(output[i], i as f32);
            }
        }
    }

    #[tokio::test]
    async fn test_masked_fill_precision() {
        let dev = get_test_device().await;

        // Test negative fill value
        let input = vec![1.5, 2.5, 3.5];
        let mask = vec![false, true, false];
        let output = masked_fill(&dev.device, &dev.queue, &input, &mask, -7.5)
            .await
            .unwrap();

        assert_eq!(output.len(), 3);
        assert_eq!(output[0], 1.5);
        assert_eq!(output[1], -7.5);
        assert_eq!(output[2], 3.5);
    }
}
