//! NMS - Non-Maximum Suppression
//!
//! Filters overlapping bounding boxes in object detection.
//! Used in YOLO, Faster R-CNN, etc.

pub struct BoundingBox {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub score: f32,
}

pub fn iou(box1: &BoundingBox, box2: &BoundingBox) -> f32 {
    let x1 = box1.x1.max(box2.x1);
    let y1 = box1.y1.max(box2.y1);
    let x2 = box1.x2.min(box2.x2);
    let y2 = box1.y2.min(box2.y2);
    
    let intersection = ((x2 - x1).max(0.0)) * ((y2 - y1).max(0.0));
    
    let area1 = (box1.x2 - box1.x1) * (box1.y2 - box1.y1);
    let area2 = (box2.x2 - box2.x1) * (box2.y2 - box2.y1);
    let union = area1 + area2 - intersection;
    
    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

pub async fn nms(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    boxes: &[BoundingBox],
    iou_threshold: f32,
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    // Sort by score descending
    let mut indices: Vec<usize> = (0..boxes.len()).collect();
    indices.sort_by(|&a, &b| boxes[b].score.partial_cmp(&boxes[a].score).unwrap());
    
    let mut keep = Vec::new();
    let mut suppressed = vec![false; boxes.len()];
    
    for &idx in &indices {
        if suppressed[idx] {
            continue;
        }
        
        keep.push(idx);
        
        // Suppress overlapping boxes
        for &other_idx in &indices {
            if !suppressed[other_idx] && idx != other_idx {
                if iou(&boxes[idx], &boxes[other_idx]) > iou_threshold {
                    suppressed[other_idx] = true;
                }
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
    async fn test_nms() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let boxes = vec![
            BoundingBox { x1: 0.0, y1: 0.0, x2: 10.0, y2: 10.0, score: 0.9 },
            BoundingBox { x1: 1.0, y1: 1.0, x2: 11.0, y2: 11.0, score: 0.8 }, // Overlaps
        ];
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();
        assert_eq!(keep.len(), 1); // Second box suppressed
    }
}
