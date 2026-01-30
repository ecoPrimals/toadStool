//! Grid Sample - Spatial transformer sampling
//!
//! Samples input at arbitrary grid locations.
//! Used in spatial transformer networks, warping.

pub async fn grid_sample(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],      // [batch, channels, height, width]
    grid: &[f32],       // [batch, out_height, out_width, 2] (normalized coords)
    batch_size: usize,
    channels: usize,
    in_height: usize,
    in_width: usize,
    out_height: usize,
    out_width: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * channels * out_height * out_width];
    
    for b in 0..batch_size {
        for oh in 0..out_height {
            for ow in 0..out_width {
                let grid_idx = b * out_height * out_width * 2 + oh * out_width * 2 + ow * 2;
                let x = grid[grid_idx];     // Normalized x in [-1, 1]
                let y = grid[grid_idx + 1]; // Normalized y in [-1, 1]
                
                // Convert to pixel coordinates
                let px = ((x + 1.0) / 2.0) * (in_width - 1) as f32;
                let py = ((y + 1.0) / 2.0) * (in_height - 1) as f32;
                
                // Bilinear interpolation
                let px0 = px.floor() as usize;
                let py0 = py.floor() as usize;
                let px1 = (px0 + 1).min(in_width - 1);
                let py1 = (py0 + 1).min(in_height - 1);
                
                let wx = px - px0 as f32;
                let wy = py - py0 as f32;
                
                for c in 0..channels {
                    let idx00 = b * channels * in_height * in_width + c * in_height * in_width + py0 * in_width + px0;
                    let idx01 = b * channels * in_height * in_width + c * in_height * in_width + py0 * in_width + px1;
                    let idx10 = b * channels * in_height * in_width + c * in_height * in_width + py1 * in_width + px0;
                    let idx11 = b * channels * in_height * in_width + c * in_height * in_width + py1 * in_width + px1;
                    
                    let v00 = if px0 < in_width && py0 < in_height { input[idx00] } else { 0.0 };
                    let v01 = if px1 < in_width && py0 < in_height { input[idx01] } else { 0.0 };
                    let v10 = if px0 < in_width && py1 < in_height { input[idx10] } else { 0.0 };
                    let v11 = if px1 < in_width && py1 < in_height { input[idx11] } else { 0.0 };
                    
                    let v0 = v00 * (1.0 - wx) + v01 * wx;
                    let v1 = v10 * (1.0 - wx) + v11 * wx;
                    let value = v0 * (1.0 - wy) + v1 * wy;
                    
                    let out_idx = b * channels * out_height * out_width + c * out_height * out_width + oh * out_width + ow;
                    output[out_idx] = value;
                }
            }
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_grid_sample() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let grid = vec![0.0; 1 * 2 * 2 * 2]; // Identity grid
        let output = grid_sample(&dev.device, &dev.queue, &input, &grid, 1, 3, 4, 4, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 2 * 2);
    }
}
