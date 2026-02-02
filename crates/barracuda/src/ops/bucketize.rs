//! Bucketize - Assign elements to bins
//!
//! Maps each input value to its bin index.

pub async fn bucketize(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    boundaries: &[f32], // Must be sorted
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let output: Vec<usize> = input
        .iter()
        .map(|&val| {
            // Binary search for bin
            boundaries.iter().take_while(|&&b| val >= b).count()
        })
        .collect();

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_bucketize_basic() {
        let dev = get_test_device().await;
        let input = vec![0.5, 1.5, 2.5, 3.5];
        let boundaries = vec![1.0, 2.0, 3.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();
        assert_eq!(bins, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_bucketize_edge_cases() {
        let dev = get_test_device().await;

        // Values exactly at boundaries
        let input = vec![1.0, 2.0, 3.0];
        let boundaries = vec![1.0, 2.0, 3.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();
        assert_eq!(bins, vec![1, 2, 3]); // At boundary means in that bin

        // Empty input
        let input: Vec<f32> = vec![];
        let boundaries = vec![1.0, 2.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();
        assert_eq!(bins.len(), 0);
    }

    #[tokio::test]
    async fn test_bucketize_boundary() {
        let dev = get_test_device().await;

        // Values below/above all boundaries
        let input = vec![-1.0, 0.0, 10.0, 20.0];
        let boundaries = vec![1.0, 5.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();
        assert_eq!(bins, vec![0, 0, 2, 2]); // Below all, in bin 0; above all, in last bin

        // Single boundary
        let input = vec![0.5, 1.5, 2.5];
        let boundaries = vec![2.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();
        assert_eq!(bins, vec![0, 0, 1]);
    }

    #[tokio::test]
    async fn test_bucketize_large_batch() {
        let dev = get_test_device().await;

        // Many values
        let input: Vec<f32> = (0..100).map(|i| i as f32 * 0.1).collect();
        let boundaries = vec![2.0, 4.0, 6.0, 8.0];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();

        assert_eq!(bins.len(), 100);
        // Values 0-1.9 in bin 0, 2.0-3.9 in bin 1, etc.
        assert_eq!(bins[0], 0); // 0.0 < 2.0
        assert_eq!(bins[20], 1); // 2.0 >= 2.0
        assert_eq!(bins[40], 2); // 4.0 >= 4.0
    }

    #[tokio::test]
    async fn test_bucketize_precision() {
        let dev = get_test_device().await;

        // Test with fractional boundaries
        let input = vec![0.1, 0.25, 0.5, 0.75, 0.9];
        let boundaries = vec![0.25, 0.5, 0.75];
        let bins = bucketize(&dev.device, &dev.queue, &input, &boundaries)
            .await
            .unwrap();

        assert_eq!(bins[0], 0); // 0.1 < 0.25
        assert_eq!(bins[1], 1); // 0.25 >= 0.25
        assert_eq!(bins[2], 2); // 0.5 >= 0.5
        assert_eq!(bins[3], 3); // 0.75 >= 0.75
        assert_eq!(bins[4], 3); // 0.9 >= 0.75
    }
}
