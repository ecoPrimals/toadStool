//! Anchor Generator - Generate anchor boxes
//!
//! Creates anchor boxes for object detection.

pub async fn anchor_generator(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    feature_h: usize,
    feature_w: usize,
    stride: usize,
    sizes: &[f32],
    aspect_ratios: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let num_anchors = sizes.len() * aspect_ratios.len();
    let total_anchors = feature_h * feature_w * num_anchors;
    let mut anchors = vec![0.0f32; total_anchors * 4];
    
    let mut anchor_idx = 0;
    
    for h in 0..feature_h {
        for w in 0..feature_w {
            let cx = (w * stride) as f32 + stride as f32 * 0.5;
            let cy = (h * stride) as f32 + stride as f32 * 0.5;
            
            for &size in sizes {
                for &ratio in aspect_ratios {
                    let anchor_w = size * ratio.sqrt();
                    let anchor_h = size / ratio.sqrt();
                    
                    anchors[anchor_idx * 4] = cx - anchor_w * 0.5;
                    anchors[anchor_idx * 4 + 1] = cy - anchor_h * 0.5;
                    anchors[anchor_idx * 4 + 2] = cx + anchor_w * 0.5;
                    anchors[anchor_idx * 4 + 3] = cy + anchor_h * 0.5;
                    
                    anchor_idx += 1;
                }
            }
        }
    }
    
    Ok(anchors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_anchor_generator() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let anchors = anchor_generator(&dev.device, &dev.queue, 4, 4, 16, &[32.0, 64.0], &[0.5, 1.0, 2.0]).await.unwrap();
        assert_eq!(anchors.len(), 4 * 4 * 2 * 3 * 4); // h*w*sizes*ratios*4
    }
}
