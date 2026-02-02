//! BBox Transform - Transform bounding boxes
//!
//! Applies deltas to anchor boxes (object detection).

pub async fn bbox_transform(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    anchors: &[f32], // [N, 4] (x1, y1, x2, y2)
    deltas: &[f32],  // [N, 4] (dx, dy, dw, dh)
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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_bbox_transform_basic() {
        let dev = get_test_device().await;
        let anchors = vec![0.0, 0.0, 10.0, 10.0];
        let deltas = vec![0.0, 0.0, 0.0, 0.0]; // Identity transform
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Zero deltas should preserve anchor center
        assert!((output[0] - 0.0).abs() < 0.01);
        assert!((output[2] - 10.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_bbox_transform_edge_cases() {
        let dev = get_test_device().await;

        // Test with single anchor at origin
        let anchors = vec![0.0, 0.0, 1.0, 1.0];
        let deltas = vec![0.0, 0.0, 0.0, 0.0];
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Test with translation only (no scaling)
        let anchors = vec![10.0, 10.0, 20.0, 20.0];
        let deltas = vec![0.5, 0.5, 0.0, 0.0]; // Move center
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bbox_transform_boundary() {
        let dev = get_test_device().await;

        // Test with scaling (exponential deltas)
        let anchors = vec![0.0, 0.0, 10.0, 10.0];
        let deltas = vec![0.0, 0.0, 0.693, 0.693]; // exp(0.693) ≈ 2.0
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));

        // Width and height should approximately double
        let out_w = output[2] - output[0];
        let out_h = output[3] - output[1];
        assert!(out_w > 15.0); // Should be ~20
        assert!(out_h > 15.0);

        // Test with negative scaling
        let deltas = vec![0.0, 0.0, -0.693, -0.693]; // exp(-0.693) ≈ 0.5
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();
        let out_w = output[2] - output[0];
        assert!(out_w < 7.0); // Should be ~5
    }

    #[tokio::test]
    async fn test_bbox_transform_large_batch() {
        let dev = get_test_device().await;

        // Multiple anchors
        let num_boxes = 100;
        let mut anchors = Vec::new();
        let mut deltas = Vec::new();

        for i in 0..num_boxes {
            let base = (i * 10) as f32;
            anchors.extend_from_slice(&[base, base, base + 10.0, base + 10.0]);
            deltas.extend_from_slice(&[0.1, 0.1, 0.0, 0.0]);
        }

        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, num_boxes)
            .await
            .unwrap();

        assert_eq!(output.len(), num_boxes * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_bbox_transform_precision() {
        let dev = get_test_device().await;

        // Test with known values
        // Anchor: [0, 0, 10, 10] → center (5, 5), size (10, 10)
        // Deltas: [0.1, 0.2, 0, 0] → shift center by (1, 2)
        let anchors = vec![0.0, 0.0, 10.0, 10.0];
        let deltas = vec![0.1, 0.2, 0.0, 0.0];
        let output = bbox_transform(&dev.device, &dev.queue, &anchors, &deltas, 1)
            .await
            .unwrap();

        // New center: (5 + 1, 5 + 2) = (6, 7)
        // New box: [1, 2, 11, 12]
        assert!((output[0] - 1.0).abs() < 0.01);
        assert!((output[1] - 2.0).abs() < 0.01);
        assert!((output[2] - 11.0).abs() < 0.01);
        assert!((output[3] - 12.0).abs() < 0.01);
    }
}
