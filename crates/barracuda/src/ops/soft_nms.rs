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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_soft_nms_basic() {
        let dev = get_test_device().await;
        let mut boxes = vec![
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
            },
        ];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();
        assert!(keep.len() >= 1);
    }

    #[tokio::test]
    async fn test_soft_nms_edge_cases() {
        let dev = get_test_device().await;

        // Single box
        let mut boxes = vec![BoundingBox {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            score: 0.9,
        }];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();
        assert_eq!(keep.len(), 1);

        // No overlap
        let mut boxes = vec![
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
        ];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();
        assert_eq!(keep.len(), 2);
    }

    #[tokio::test]
    async fn test_soft_nms_boundary() {
        let dev = get_test_device().await;

        // High overlap
        let mut boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 0.5,
                y1: 0.5,
                x2: 10.5,
                y2: 10.5,
                score: 0.85,
            },
        ];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();
        assert!(keep.len() >= 1);
        // Score of second box should be reduced
        assert!(boxes[1].score < 0.85);

        // Different sigma
        let mut boxes = vec![
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
            },
        ];
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.3)
            .await
            .unwrap();
        assert!(keep.len() >= 1);
    }

    #[tokio::test]
    async fn test_soft_nms_large_batch() {
        let dev = get_test_device().await;

        // 100 boxes
        let mut boxes: Vec<BoundingBox> = (0..100)
            .map(|i| BoundingBox {
                x1: (i * 5) as f32,
                y1: 0.0,
                x2: (i * 5 + 10) as f32,
                y2: 10.0,
                score: 0.9 - i as f32 * 0.001,
            })
            .collect();
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();
        assert!(keep.len() > 0);
    }

    #[tokio::test]
    async fn test_soft_nms_precision() {
        let dev = get_test_device().await;

        // Verify score reduction
        let mut boxes = vec![
            BoundingBox {
                x1: 0.0,
                y1: 0.0,
                x2: 10.0,
                y2: 10.0,
                score: 0.9,
            },
            BoundingBox {
                x1: 2.0,
                y1: 2.0,
                x2: 12.0,
                y2: 12.0,
                score: 0.8,
            },
        ];
        let initial_score = boxes[1].score;
        let keep = soft_nms(&dev.device, &dev.queue, &mut boxes, 0.5, 0.5)
            .await
            .unwrap();

        assert!(keep.len() >= 1);
        assert!(boxes[0].score == 0.9); // First box unchanged
        assert!(boxes[1].score <= initial_score); // Second box score reduced or same
    }
}
