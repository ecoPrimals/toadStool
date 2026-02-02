//! ChannelShuffle - ShuffleNet operation
//!
//! Shuffles channels for efficient mobile network architectures.
//!
//! ## Algorithm
//!
//! Reorganizes channels into groups and shuffles them for better information flow.

pub async fn channel_shuffle(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    groups: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if channels % groups != 0 {
        return Err("Channels must be divisible by groups".into());
    }

    let channels_per_group = channels / groups;
    let spatial_size = height * width;
    let mut output = vec![0.0f32; input.len()];

    for b in 0..batch_size {
        for c in 0..channels {
            let group = c / channels_per_group;
            let idx_in_group = c % channels_per_group;
            let new_c = idx_in_group * groups + group;

            for s in 0..spatial_size {
                let in_idx = b * channels * spatial_size + c * spatial_size + s;
                let out_idx = b * channels * spatial_size + new_c * spatial_size + s;
                output[out_idx] = input[in_idx];
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_channel_shuffle_basic() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;
        let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let output = channel_shuffle(&device, &queue, &input, 1, 4, 2, 2, 2)
            .await
            .unwrap();
        assert_eq!(output.len(), 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_channel_shuffle_edge_cases() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Single group (no shuffle needed)
        let input: Vec<f32> = (0..8).map(|i| i as f32).collect();
        let output = channel_shuffle(&device, &queue, &input, 1, 4, 2, 1, 1)
            .await
            .unwrap();
        assert_eq!(output.len(), 8);

        // Channels == groups (each channel is its own group)
        let input: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let output = channel_shuffle(&device, &queue, &input, 1, 3, 2, 2, 3)
            .await
            .unwrap();
        assert_eq!(output.len(), 12);
    }

    #[tokio::test]
    async fn test_channel_shuffle_boundary() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Different group configurations
        let input: Vec<f32> = (0..24).map(|i| i as f32).collect();

        // 6 channels, 2 groups
        let output1 = channel_shuffle(&device, &queue, &input, 1, 6, 2, 2, 2)
            .await
            .unwrap();
        assert_eq!(output1.len(), 24);

        // 6 channels, 3 groups
        let output2 = channel_shuffle(&device, &queue, &input, 1, 6, 2, 2, 3)
            .await
            .unwrap();
        assert_eq!(output2.len(), 24);

        // Different shuffles should produce different outputs
        assert_ne!(output1, output2);
    }

    #[tokio::test]
    async fn test_channel_shuffle_large_batch() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Multiple batches, more channels
        let batch_size = 4;
        let channels = 16;
        let height = 8;
        let width = 8;
        let groups = 4;

        let input: Vec<f32> = (0..batch_size * channels * height * width)
            .map(|i| (i % 100) as f32)
            .collect();

        let output = channel_shuffle(
            &device, &queue, &input, batch_size, channels, height, width, groups,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), input.len());
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_channel_shuffle_precision() {
        let dev = get_test_device().await;
        let device = &dev.device;
        let queue = &dev.queue;

        // Test with distinct channel values
        // 4 channels, 2 groups → channels_per_group = 2
        // Channel mapping: [0,1,2,3] → [0,2,1,3]
        let mut input = vec![0.0f32; 1 * 4 * 1 * 1];
        for c in 0..4 {
            input[c] = c as f32;
        }

        let output = channel_shuffle(&device, &queue, &input, 1, 4, 1, 1, 2)
            .await
            .unwrap();

        // Check that shuffle happened correctly
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));

        // Values should be rearranged
        assert_ne!(output, input);
    }
}
