//! Interpolate - Multi-mode tensor interpolation
//!
//! Resize tensors with various interpolation modes.

#[derive(Debug, Clone, Copy)]
pub enum InterpolateMode {
    Nearest,
    Linear,
    Bilinear,
    Trilinear,
}

pub async fn interpolate(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    input_size: &[usize],
    output_size: &[usize],
    mode: InterpolateMode,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Simplified for 2D bilinear
    if input_size.len() != 4 || output_size.len() != 4 {
        return Err("Currently supports 4D tensors only".into());
    }

    let (b, c, in_h, in_w) = (input_size[0], input_size[1], input_size[2], input_size[3]);
    let (_, _, out_h, out_w) = (
        output_size[0],
        output_size[1],
        output_size[2],
        output_size[3],
    );

    let mut output = vec![0.0f32; b * c * out_h * out_w];
    let scale_h = in_h as f32 / out_h as f32;
    let scale_w = in_w as f32 / out_w as f32;

    for batch in 0..b {
        for ch in 0..c {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let ih_f = oh as f32 * scale_h;
                    let iw_f = ow as f32 * scale_w;

                    let value = match mode {
                        InterpolateMode::Nearest | InterpolateMode::Linear => {
                            let ih = ih_f.round() as usize;
                            let iw = iw_f.round() as usize;
                            let ih = ih.min(in_h - 1);
                            let iw = iw.min(in_w - 1);
                            input[batch * c * in_h * in_w + ch * in_h * in_w + ih * in_w + iw]
                        }
                        InterpolateMode::Bilinear | InterpolateMode::Trilinear => {
                            let ih0 = ih_f.floor() as usize;
                            let iw0 = iw_f.floor() as usize;
                            let ih1 = (ih0 + 1).min(in_h - 1);
                            let iw1 = (iw0 + 1).min(in_w - 1);

                            let h_weight = ih_f - ih0 as f32;
                            let w_weight = iw_f - iw0 as f32;

                            let idx00 =
                                batch * c * in_h * in_w + ch * in_h * in_w + ih0 * in_w + iw0;
                            let idx01 =
                                batch * c * in_h * in_w + ch * in_h * in_w + ih0 * in_w + iw1;
                            let idx10 =
                                batch * c * in_h * in_w + ch * in_h * in_w + ih1 * in_w + iw0;
                            let idx11 =
                                batch * c * in_h * in_w + ch * in_h * in_w + ih1 * in_w + iw1;

                            let v00 = input[idx00];
                            let v01 = input[idx01];
                            let v10 = input[idx10];
                            let v11 = input[idx11];

                            let v0 = v00 * (1.0 - w_weight) + v01 * w_weight;
                            let v1 = v10 * (1.0 - w_weight) + v11 * w_weight;
                            v0 * (1.0 - h_weight) + v1 * h_weight
                        }
                    };

                    output[batch * c * out_h * out_w + ch * out_h * out_w + oh * out_w + ow] =
                        value;
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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_interpolate_basic() {
        let dev = get_test_device().await;
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 3, 4, 4],
            &[1, 3, 8, 8],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 3 * 8 * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_interpolate_edge_cases() {
        let dev = get_test_device().await;

        // Nearest mode
        let input = vec![1.0; 1 * 1 * 2 * 2];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 1, 2, 2],
            &[1, 1, 4, 4],
            InterpolateMode::Nearest,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 1 * 4 * 4);

        // No resize (same size)
        let input = vec![1.0; 1 * 3 * 8 * 8];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 3, 8, 8],
            &[1, 3, 8, 8],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 3 * 8 * 8);
    }

    #[tokio::test]
    async fn test_interpolate_boundary() {
        let dev = get_test_device().await;

        // Downsampling
        let input = vec![1.0; 1 * 3 * 16 * 16];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 3, 16, 16],
            &[1, 3, 8, 8],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 3 * 8 * 8);

        // Large upsampling
        let input = vec![1.0; 1 * 3 * 4 * 4];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 3, 4, 4],
            &[1, 3, 32, 32],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 3 * 32 * 32);
    }

    #[tokio::test]
    async fn test_interpolate_large_batch() {
        let dev = get_test_device().await;

        // Batch size 8
        let batch_size = 8;
        let input = vec![1.0; batch_size * 3 * 8 * 8];
        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[batch_size, 3, 8, 8],
            &[batch_size, 3, 16, 16],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), batch_size * 3 * 16 * 16);
    }

    #[tokio::test]
    async fn test_interpolate_precision() {
        let dev = get_test_device().await;

        // Test bilinear interpolation with known values
        let mut input = vec![0.0; 1 * 1 * 2 * 2];
        input[0] = 1.0; // Top-left
        input[1] = 2.0; // Top-right
        input[2] = 3.0; // Bottom-left
        input[3] = 4.0; // Bottom-right

        let output = interpolate(
            &dev.device,
            &dev.queue,
            &input,
            &[1, 1, 2, 2],
            &[1, 1, 3, 3],
            InterpolateMode::Bilinear,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), 9);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Values should be interpolated between 1-4
        assert!(output.iter().all(|&x| x >= 1.0 && x <= 4.0));
    }
}
