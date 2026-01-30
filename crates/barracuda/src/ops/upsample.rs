//! Upsample - Nearest neighbor or bilinear interpolation
//!
//! Increases spatial resolution of feature maps.

#[derive(Debug, Clone, Copy)]
pub enum UpsampleMode {
    Nearest,
    Bilinear,
}

pub async fn upsample(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    batch_size: usize,
    channels: usize,
    in_h: usize,
    in_w: usize,
    out_h: usize,
    out_w: usize,
    mode: UpsampleMode,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; batch_size * channels * out_h * out_w];
    
    let scale_h = in_h as f32 / out_h as f32;
    let scale_w = in_w as f32 / out_w as f32;
    
    for b in 0..batch_size {
        for c in 0..channels {
            for oh in 0..out_h {
                for ow in 0..out_w {
                    let ih_f = oh as f32 * scale_h;
                    let iw_f = ow as f32 * scale_w;
                    
                    let value = match mode {
                        UpsampleMode::Nearest => {
                            let ih = ih_f.round() as usize;
                            let iw = iw_f.round() as usize;
                            let ih = ih.min(in_h - 1);
                            let iw = iw.min(in_w - 1);
                            let idx = b * channels * in_h * in_w + c * in_h * in_w + ih * in_w + iw;
                            input[idx]
                        },
                        UpsampleMode::Bilinear => {
                            let ih0 = ih_f.floor() as usize;
                            let iw0 = iw_f.floor() as usize;
                            let ih1 = (ih0 + 1).min(in_h - 1);
                            let iw1 = (iw0 + 1).min(in_w - 1);
                            
                            let h_weight = ih_f - ih0 as f32;
                            let w_weight = iw_f - iw0 as f32;
                            
                            let idx00 = b * channels * in_h * in_w + c * in_h * in_w + ih0 * in_w + iw0;
                            let idx01 = b * channels * in_h * in_w + c * in_h * in_w + ih0 * in_w + iw1;
                            let idx10 = b * channels * in_h * in_w + c * in_h * in_w + ih1 * in_w + iw0;
                            let idx11 = b * channels * in_h * in_w + c * in_h * in_w + ih1 * in_w + iw1;
                            
                            let v00 = input[idx00];
                            let v01 = input[idx01];
                            let v10 = input[idx10];
                            let v11 = input[idx11];
                            
                            let v0 = v00 * (1.0 - w_weight) + v01 * w_weight;
                            let v1 = v10 * (1.0 - w_weight) + v11 * w_weight;
                            v0 * (1.0 - h_weight) + v1 * h_weight
                        },
                    };
                    
                    let out_idx = b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow;
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
    async fn test_upsample_nearest() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 3.0, 4.0]; // 1x1x2x2
        let output = upsample(&device, &queue, &input, 1, 1, 2, 2, 4, 4, UpsampleMode::Nearest).await.unwrap();
        assert_eq!(output.len(), 16);
    }
}
