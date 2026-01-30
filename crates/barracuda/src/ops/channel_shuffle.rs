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
    
    #[tokio::test]
    async fn test_channel_shuffle() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let input: Vec<f32> = (0..16).map(|i| i as f32).collect();
        let output = channel_shuffle(&device, &queue, &input, 1, 4, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 16);
    }
}
