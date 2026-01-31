//! CircularPad2D - Padding with wrap-around
//!
//! Pads by wrapping to opposite edge (torus topology).

pub async fn circular_pad2d(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    pad_top: usize,
    pad_bottom: usize,
    pad_left: usize,
    pad_right: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let out_h = height + pad_top + pad_bottom;
    let out_w = width + pad_left + pad_right;
    let mut output = vec![0.0f32; batch_size * channels * out_h * out_w];
    
    for b in 0..batch_size {
        for c in 0..channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let ih_raw = oh as isize - pad_top as isize;
                    let iw_raw = ow as isize - pad_left as isize;
                    
                    // Wrap around
                    let ih = ((ih_raw % height as isize + height as isize) % height as isize) as usize;
                    let iw = ((iw_raw % width as isize + width as isize) % width as isize) as usize;
                    
                    let in_idx = b * channels * height * width + c * height * width + ih * width + iw;
                    let out_idx = b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
                    output[out_idx] = input[in_idx];
                }
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
    async fn test_circular_pad2d_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 1, 1, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_circular_pad2d_edge_cases() {
        let dev = get_test_device().await;
        
        // No padding (no-op)
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 0, 0, 0, 0).await.unwrap();
        assert_eq!(output, input);
        
        // Single pixel input
        let input = vec![5.0];
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 1, 1, 1, 1, 1, 1).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 3 * 3);
        // All should be 5.0 (wrapping around)
        assert!(output.iter().all(|&x| (x - 5.0).abs() < 1e-6));
    }

    #[tokio::test]
    async fn test_circular_pad2d_boundary() {
        let dev = get_test_device().await;
        
        // Pad only one side
        let input: Vec<f32> = (0..9).map(|i| i as f32).collect(); // 3×3
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 3, 3, 1, 0, 0, 0).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 3);
        
        // Asymmetric padding
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 3, 3, 1, 2, 1, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 1 * 6 * 6);
    }

    #[tokio::test]
    async fn test_circular_pad2d_large_batch() {
        let dev = get_test_device().await;
        
        // Multiple batches and channels
        let batch_size = 2;
        let channels = 3;
        let height = 4;
        let width = 4;
        
        let input: Vec<f32> = (0..batch_size * channels * height * width)
            .map(|i| (i % 10) as f32)
            .collect();
        
        let output = circular_pad2d(&dev.device, &dev.queue, &input, batch_size, channels, height, width, 1, 1, 1, 1).await.unwrap();
        
        assert_eq!(output.len(), batch_size * channels * 6 * 6);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_circular_pad2d_precision() {
        let dev = get_test_device().await;
        
        // Test wrapping behavior with distinct values
        let input = vec![
            1.0, 2.0,  // Row 0
            3.0, 4.0,  // Row 1
        ];
        
        let output = circular_pad2d(&dev.device, &dev.queue, &input, 1, 1, 2, 2, 1, 0, 1, 0).await.unwrap();
        
        // Output is 3×3: top row wraps from bottom
        assert_eq!(output.len(), 1 * 1 * 3 * 3);
        
        // Top-left should wrap from bottom-right
        assert!((output[0] - 4.0).abs() < 1e-6);
        // Top-middle should wrap from bottom-left
        assert!((output[1] - 3.0).abs() < 1e-6);
    }
}
