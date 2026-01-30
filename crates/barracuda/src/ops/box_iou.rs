//! Box IoU - Intersection over Union for boxes
//!
//! Computes IoU matrix for all pairs of boxes.

pub async fn box_iou(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    boxes1: &[f32],  // [N, 4]
    boxes2: &[f32],  // [M, 4]
    n: usize,
    m: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if boxes1.len() != n * 4 || boxes2.len() != m * 4 {
        return Err("Dimension mismatch".into());
    }
    
    let mut ious = vec![0.0f32; n * m];
    
    for i in 0..n {
        for j in 0..m {
            let b1_idx = i * 4;
            let b2_idx = j * 4;
            
            let x1 = boxes1[b1_idx].max(boxes2[b2_idx]);
            let y1 = boxes1[b1_idx + 1].max(boxes2[b2_idx + 1]);
            let x2 = boxes1[b1_idx + 2].min(boxes2[b2_idx + 2]);
            let y2 = boxes1[b1_idx + 3].min(boxes2[b2_idx + 3]);
            
            let intersection = ((x2 - x1).max(0.0)) * ((y2 - y1).max(0.0));
            
            let area1 = (boxes1[b1_idx + 2] - boxes1[b1_idx]) * (boxes1[b1_idx + 3] - boxes1[b1_idx + 1]);
            let area2 = (boxes2[b2_idx + 2] - boxes2[b2_idx]) * (boxes2[b2_idx + 3] - boxes2[b2_idx + 1]);
            let union = area1 + area2 - intersection;
            
            ious[i * m + j] = if union > 0.0 { intersection / union } else { 0.0 };
        }
    }
    
    Ok(ious)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_box_iou() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let boxes1 = vec![0.0, 0.0, 10.0, 10.0];
        let boxes2 = vec![5.0, 5.0, 15.0, 15.0];
        let ious = box_iou(&dev.device, &dev.queue, &boxes1, &boxes2, 1, 1).await.unwrap();
        assert_eq!(ious.len(), 1);
        assert!(ious[0] > 0.0 && ious[0] < 1.0);
    }
}
