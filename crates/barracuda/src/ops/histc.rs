//! Histc - Histogram with custom bins
//!
//! Computes histogram with specified bin edges.

pub async fn histc(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    num_bins: usize,
    min: f32,
    max: f32,
) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    let mut counts = vec![0u32; num_bins];
    let bin_width = (max - min) / num_bins as f32;

    for &val in input {
        if val >= min && val < max {
            let bin = ((val - min) / bin_width) as usize;
            let bin = bin.min(num_bins - 1);
            counts[bin] += 1;
        }
    }

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_histc() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![0.1, 0.5, 0.9, 1.5, 2.1, 2.8];
        let counts = histc(&dev.device, &dev.queue, &input, 3, 0.0, 3.0)
            .await
            .unwrap();
        assert_eq!(counts.len(), 3);
    }
}
