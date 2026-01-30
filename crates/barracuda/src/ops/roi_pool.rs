//! RoI Pooling - Region of Interest pooling
//!
//! Pools features from regions of interest.
//! Used in Faster R-CNN, object detection.

pub struct RoI {
    pub batch_idx: usize,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

pub async fn roi_pool(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    features: &[f32],
    rois: &[RoI],
    _batch_size: usize,
    channels: usize,
    height: usize,
    width: usize,
    output_h: usize,
    output_w: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; rois.len() * channels * output_h * output_w];
    
    for (roi_idx, roi) in rois.iter().enumerate() {
        let roi_h = (roi.y2 - roi.y1) * height as f32;
        let roi_w = (roi.x2 - roi.x1) * width as f32;
        
        let bin_h = roi_h / output_h as f32;
        let bin_w = roi_w / output_w as f32;
        
        for c in 0..channels {
            for oh in 0..output_h {
                for ow in 0..output_w {
                    let h_start = (roi.y1 * height as f32 + oh as f32 * bin_h) as usize;
                    let h_end = ((roi.y1 * height as f32 + (oh + 1) as f32 * bin_h) as usize).min(height);
                    let w_start = (roi.x1 * width as f32 + ow as f32 * bin_w) as usize;
                    let w_end = ((roi.x1 * width as f32 + (ow + 1) as f32 * bin_w) as usize).min(width);
                    
                    let mut max_val = f32::NEG_INFINITY;
                    
                    for h in h_start..h_end {
                        for w in w_start..w_end {
                            let feat_idx = roi.batch_idx * channels * height * width + c * height * width + h * width + w;
                            if feat_idx < features.len() {
                                max_val = max_val.max(features[feat_idx]);
                            }
                        }
                    }
                    
                    let out_idx = roi_idx * channels * output_h * output_w + c * output_h * output_w + oh * output_w + ow;
                    output[out_idx] = if max_val.is_finite() { max_val } else { 0.0 };
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
    async fn test_roi_pool() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let features = vec![1.0; 1 * 3 * 8 * 8];
        let rois = vec![RoI { batch_idx: 0, x1: 0.0, y1: 0.0, x2: 0.5, y2: 0.5 }];
        let output = roi_pool(&dev.device, &dev.queue, &features, &rois, 1, 3, 8, 8, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 2 * 2);
    }
}
