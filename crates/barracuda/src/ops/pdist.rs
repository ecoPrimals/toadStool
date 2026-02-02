//! Pdist - Pairwise distances within single set
//!
//! Computes all pairwise distances for samples.

pub async fn pdist(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32], // [n, dim]
    n: usize,
    dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() != n * dim {
        return Err("Dimension mismatch".into());
    }

    let num_pairs = n * (n - 1) / 2;
    let mut distances = Vec::with_capacity(num_pairs);

    for i in 0..n {
        for j in (i + 1)..n {
            let mut dist_sq = 0.0;

            for d in 0..dim {
                let diff = input[i * dim + d] - input[j * dim + d];
                dist_sq += diff * diff;
            }

            distances.push(dist_sq.sqrt());
        }
    }

    Ok(distances)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pdist() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0]; // 3 points in 2D
        let distances = pdist(&dev.device, &dev.queue, &input, 3, 2).await.unwrap();
        assert_eq!(distances.len(), 3); // 3 choose 2 = 3 pairs
    }
}
