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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_nms_basic() {
        let dev = get_test_device().await;
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 1.0,
                y1: 1.0,
                x2: 11.0,
                y2: 11.0,
                score: 0.8,
            }, // Overlaps
        ];
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();
        assert_eq!(keep.len(), 1); // Second box suppressed
        assert_eq!(keep[0], 0); // Highest score kept
    }

    #[tokio::test]
    async fn test_nms_edge_cases() {
        let dev = get_test_device().await;

        // No overlapping boxes (all kept)
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 20.0,
                y1: 20.0,
                x2: 30.0,
                y2: 30.0,
                score: 0.8,
            },
            BoundingBox {
                x1: 40.0,
                y1: 40.0,
                x2: 50.0,
                y2: 50.0,
                score: 0.7,
            },
        ];
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();
        assert_eq!(keep.len(), 3); // All boxes kept

        // Single box
        let boxes = vec![BoundingBox {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            score: 0.9,
        }];
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();
        assert_eq!(keep.len(), 1);
    }

    #[tokio::test]
    async fn test_nms_boundary() {
        let dev = get_test_device().await;

        // Test with overlapping boxes
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 5.0,
                y1: 0.0,
                x2: 15.0,
                y2: 10.0,
                score: 0.8,
            },
        ];

        // Very strict threshold (keep everything)
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.99).await.unwrap();
        assert_eq!(keep.len(), 2); // Both kept

        // Very loose threshold (suppress aggressively)
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.01).await.unwrap();
        assert_eq!(keep.len(), 1); // Only highest score

        // Empty boxes list
        let keep = nms(&dev.device, &dev.queue, &[], 0.5).await.unwrap();
        assert_eq!(keep.len(), 0);
    }

    #[tokio::test]
    async fn test_nms_large_batch() {
        let dev = get_test_device().await;

        // Many boxes in grid pattern
        let mut boxes = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                let x = (i * 15) as f32;
                let y = (j * 15) as f32;
                let score = 1.0 - (i + j) as f32 * 0.01;
                boxes.push(BoundingBox {
                    x1: x,
                    y1: y,
                    x2: x + 10.0,
                    y2: y + 10.0,
                    score,
                });
            }
        }

        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();
        assert!(keep.len() > 0);
        assert!(keep.len() <= boxes.len());
    }

    #[tokio::test]
    async fn test_nms_precision() {
        let dev = get_test_device().await;

        // Test score-based sorting
        let boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.5,
            },
            BoundingBox {
                x1: 1.0,
                y1: 1.0,
                x2: 11.0,
                y2: 11.0,
                score: 0.9,
            }, // High overlap
            BoundingBox {
                x1: 0.5,
                y1: 0.5,
                x2: 10.5,
                y2: 10.5,
                score: 0.7,
            }, // Medium overlap
        ];
        let keep = nms(&dev.device, &dev.queue, &boxes, 0.5).await.unwrap();

        // Highest score should be kept first
        assert!(keep.contains(&1)); // score 0.9
        assert_eq!(keep.len(), 1); // Others suppressed due to overlap
    }
}
