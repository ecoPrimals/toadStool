//! RandomPerspective - Random perspective transformation
//!
//! Applies random perspective distortion.
//! Simulates different camera viewpoints.

pub async fn random_perspective(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    image: &[f32],
    channels: usize,
    height: usize,
    width: usize,
    distortion_scale: f32,
    seed: u64,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Generate random corner displacements
    let mut rng = seed;
    let mut rand = || {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        ((rng % 2000) as f32 / 1000.0 - 1.0) * distortion_scale
    };

    // Source corners
    let src_corners = [
        (0.0, 0.0),
        (width as f32, 0.0),
        (width as f32, height as f32),
        (0.0, height as f32),
    ];

    // Destination corners with random displacement
    let dst_corners = [
        (
            src_corners[0].0 + rand() * width as f32,
            src_corners[0].1 + rand() * height as f32,
        ),
        (
            src_corners[1].0 + rand() * width as f32,
            src_corners[1].1 + rand() * height as f32,
        ),
        (
            src_corners[2].0 + rand() * width as f32,
            src_corners[2].1 + rand() * height as f32,
        ),
        (
            src_corners[3].0 + rand() * width as f32,
            src_corners[3].1 + rand() * height as f32,
        ),
    ];

    let mut output = vec![0.0f32; channels * height * width];

    // Simplified perspective transform using bilinear interpolation
    for c in 0..channels {
        for i in 0..height {
            for j in 0..width {
                let u = j as f32 / width as f32;
                let v = i as f32 / height as f32;

                // Bilinear interpolation of perspective coordinates
                let top = (
                    dst_corners[0].0 * (1.0 - u) + dst_corners[1].0 * u,
                    dst_corners[0].1 * (1.0 - u) + dst_corners[1].1 * u,
                );
                let bottom = (
                    dst_corners[3].0 * (1.0 - u) + dst_corners[2].0 * u,
                    dst_corners[3].1 * (1.0 - u) + dst_corners[2].1 * u,
                );

                let src_x = top.0 * (1.0 - v) + bottom.0 * v;
                let src_y = top.1 * (1.0 - v) + bottom.1 * v;

                // Sample from source with bilinear interpolation
                if src_x >= 0.0
                    && src_x < (width - 1) as f32
                    && src_y >= 0.0
                    && src_y < (height - 1) as f32
                {
                    let x0 = src_x as usize;
                    let y0 = src_y as usize;
                    let dx = src_x - x0 as f32;
                    let dy = src_y - y0 as f32;

                    let v00 = image[c * height * width + y0 * width + x0];
                    let v01 = image[c * height * width + y0 * width + x0 + 1];
                    let v10 = image[c * height * width + (y0 + 1) * width + x0];
                    let v11 = image[c * height * width + (y0 + 1) * width + x0 + 1];

                    output[c * height * width + i * width + j] = v00 * (1.0 - dx) * (1.0 - dy)
                        + v01 * dx * (1.0 - dy)
                        + v10 * (1.0 - dx) * dy
                        + v11 * dx * dy;
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
    async fn test_random_perspective() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let image = vec![1.0; 3 * 100 * 100];
        let transformed =
            random_perspective(&dev.device, &dev.queue, &image, 3, 100, 100, 0.2, 33333)
                .await
                .unwrap();
        assert_eq!(transformed.len(), image.len());
    }
}
