//! RoI Align - Improved RoI pooling with bilinear interpolation
//!
//! More accurate than RoI pooling for object detection.
//! Used in Mask R-CNN.

use super::roi_pool::RoI;

pub async fn roi_align(
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
    sampling_ratio: usize,
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
                    let mut sum = 0.0;
                    let mut count = 0;
                    
                    // Sample points in bin
                    for sh in 0..sampling_ratio {
                        for sw in 0..sampling_ratio {
                            let h = roi.y1 * height as f32 + (oh as f32 + (sh as f32 + 0.5) / sampling_ratio as f32) * bin_h;
                            let w = roi.x1 * width as f32 + (ow as f32 + (sw as f32 + 0.5) / sampling_ratio as f32) * bin_w;
                            
                            if h >= 0.0 && h < height as f32 && w >= 0.0 && w < width as f32 {
                                // Bilinear interpolation
                                let h0 = h.floor() as usize;
                                let w0 = w.floor() as usize;
                                let h1 = (h0 + 1).min(height - 1);
                                let w1 = (w0 + 1).min(width - 1);
                                
                                let hweight = h - h0 as f32;
                                let wweight = w - w0 as f32;
                                
                                let idx00 = roi.batch_idx * channels * height * width + c * height * width + h0 * width + w0;
                                let idx01 = roi.batch_idx * channels * height * width + c * height * width + h0 * width + w1;
                                let idx10 = roi.batch_idx * channels * height * width + c * height * width + h1 * width + w0;
                                let idx11 = roi.batch_idx * channels * height * width + c * height * width + h1 * width + w1;
                                
                                let v00 = if idx00 < features.len() { features[idx00] } else { 0.0 };
                                let v01 = if idx01 < features.len() { features[idx01] } else { 0.0 };
                                let v10 = if idx10 < features.len() { features[idx10] } else { 0.0 };
                                let v11 = if idx11 < features.len() { features[idx11] } else { 0.0 };
                                
                                let v0 = v00 * (1.0 - wweight) + v01 * wweight;
                                let v1 = v10 * (1.0 - wweight) + v11 * wweight;
                                let val = v0 * (1.0 - hweight) + v1 * hweight;
                                
                                sum += val;
                                count += 1;
                            }
                        }
                    }
                    
                    let out_idx = roi_idx * channels * output_h * output_w + c * output_h * output_w + oh * output_w + ow;
                    output[out_idx] = if count > 0 { sum / count as f32 } else { 0.0 };
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
    async fn test_roi_align() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let features = vec![1.0; 1 * 3 * 8 * 8];
        let rois = vec![RoI { batch_idx: 0, x1: 0.0, y1: 0.0, x2: 0.5, y2: 0.5 }];
        let output = roi_align(&dev.device, &dev.queue, &features, &rois, 1, 3, 8, 8, 2, 2, 2).await.unwrap();
        assert_eq!(output.len(), 1 * 3 * 2 * 2);
    }
}
