//! BBox Transform - Transform bounding boxes
//!
//! Applies deltas to anchor boxes (object detection).

pub async fn bbox_transform(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    anchors: &[f32],  // [N, 4] (x1, y1, x2, y2)
    deltas: &[f32],   // [N, 4] (dx, dy, dw, dh)
    num_boxes: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if anchors.len() != num_boxes * 4 || deltas.len() != num_boxes * 4 {
        return Err("Dimension mismatch".into());
    }
    
    let mut transformed = vec![0.0f32; num_boxes * 4];
    
    for i in 0..num_boxes {
        let idx = i * 4;
        
        let anchor_w = anchors[idx + 2] - anchors[idx];
        let anchor_h = anchors[idx + 3] - anchors[idx + 1];
        let anchor_cx = anchors[idx] + anchor_w * 0.5;
        let anchor_cy = anchors[idx + 1] + anchor_h * 0.5;
        
        let pred_cx = deltas[idx] * anchor_w + anchor_cx;
        let pred_cy = deltas[idx + 1] * anchor_h + anchor_cy;
        let pred_w = deltas[idx + 2].exp() * anchor_w;
        let pred_h = deltas[idx + 3].exp() * anchor_h;
        
        transformed[idx] = pred_cx - pred_w * 0.5;
        transformed[idx + 1] = pred_cy - pred_h * 0.5;
        transformed[idx + 2] = pred_cx + pred_w * 0.5;
        transformed[idx + 3] = pred_cy + pred_h * 0.5;
    }
    
    Ok(transformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_bbox_transform() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let anchors = vec![0.0, 0.0, 10.0, 10.0];
        let deltas = vec![0.0, 0.0, 0.0, 0.0]; // Identity transform
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1).await.unwrap();
        assert_eq!(output.len(), 4);
    }
}
