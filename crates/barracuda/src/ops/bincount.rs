//! Bincount - Count occurrences of each value
//!
//! Computes histogram for integer-valued tensors.

pub async fn bincount(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[u32],
    num_bins: usize,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let mut counts = vec![0u32; num_bins];

    for &val in input {
        if (val as usize) < num_bins {
            counts[val as usize] += 1;
        }
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_bincount_basic() {
        let dev = get_test_device().await;
        let input = vec![0, 1, 1, 2, 2, 2, 3];
        let counts = bincount(&dev.device, &dev.queue, &input, 4).await.unwrap();
        assert_eq!(counts, vec![1, 2, 3, 1]);
    }

    #[tokio::test]
    async fn test_bincount_edge_cases() {
        let dev = get_test_device().await;

        // Empty input
        let input: Vec<u32> = vec![];
        let counts = bincount(&dev.device, &dev.queue, &input, 5).await.unwrap();
        assert_eq!(counts, vec![0, 0, 0, 0, 0]);

        // Single value
        let input = vec![2];
        let counts = bincount(&dev.device, &dev.queue, &input, 5).await.unwrap();
        assert_eq!(counts, vec![0, 0, 1, 0, 0]);
    }

    #[tokio::test]
    async fn test_bincount_boundary() {
        let dev = get_test_device().await;

        // Values at bin edges
        let input = vec![0, 4, 0, 4];
        let counts = bincount(&dev.device, &dev.queue, &input, 5).await.unwrap();
        assert_eq!(counts, vec![2, 0, 0, 0, 2]);

        // Values exceeding num_bins (should be ignored)
        let input = vec![0, 1, 10, 2, 20];
        let counts = bincount(&dev.device, &dev.queue, &input, 5).await.unwrap();
        assert_eq!(counts, vec![1, 1, 1, 0, 0]);
    }

    #[tokio::test]
    async fn test_bincount_large_batch() {
        let dev = get_test_device().await;

        // Large input with repeated values
        let mut input = Vec::new();
        for i in 0..100 {
            input.push(i % 10);
        }

        let counts = bincount(&dev.device, &dev.queue, &input, 10).await.unwrap();

        // Each bin should have 10 counts
        assert_eq!(counts.len(), 10);
        assert!(counts.iter().all(|&c| c == 10));
    }

    #[tokio::test]
    async fn test_bincount_precision() {
        let dev = get_test_device().await;

        // Test with known distribution
        let input = vec![0, 0, 1, 1, 1, 2, 2, 2, 2];
        let counts = bincount(&dev.device, &dev.queue, &input, 3).await.unwrap();

        assert_eq!(counts[0], 2); // Two 0s
        assert_eq!(counts[1], 3); // Three 1s
        assert_eq!(counts[2], 4); // Four 2s

        // Total should match input length
        let total: u32 = counts.iter().sum();
        assert_eq!(total as usize, input.len());
    }
}
