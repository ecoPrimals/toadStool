//! Soft NMS - Soft Non-Maximum Suppression
//!
//! Reduces scores of overlapping boxes instead of removing them.
//! Better performance than hard NMS.

use super::nms::BoundingBox;

pub async fn soft_nms(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    boxes: &mut [BoundingBox],
    iou_threshold: f32,
    sigma: f32,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    let mut indices: Vec<usize> = (0..boxes.len()).collect();
    indices.sort_by(|&a, &b| boxes[b].score.partial_cmp(&boxes[a].score).unwrap());
    
    let mut keep = Vec::new();
    
    for i in 0..indices.len() {
        let idx = indices[i];
        
        if boxes[idx].score < 0.001 {
            continue;
        }
        
        keep.push(idx);
        
        // Soft suppress overlapping boxes
        for j in (i + 1)..indices.len() {
            let other_idx = indices[j];
            
            let overlap = super::nms::iou(&boxes[idx], &boxes[other_idx]);
            
            if overlap > iou_threshold {
                // Gaussian decay
                boxes[other_idx].score *= (-(overlap * overlap) / sigma).exp();
            }
        }
    }
    
    Ok(keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_soft_nms() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let mut boxes = vec![
            BoundingBox { x1: 0.0, y1: 0.0, x2: 10.0, y2: 10.0, score: 0.9 },
            BoundingBox { x1: 1.0, y1: 1.0, x2: 11.0, y2: 11.0, score: 0.8 },
        ];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5).await.unwrap();
        assert!(keep.len() >= 1);
    }
}
