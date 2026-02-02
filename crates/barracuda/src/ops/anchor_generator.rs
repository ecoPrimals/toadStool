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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_anchor_generator_basic() {
        let dev = get_test_device().await;
        let anchors = anchor_generator(
            &dev.device,
            &dev.queue,
            4,
            4,
            16,
            &[32.0, 64.0],
            &[0.5, 1.0, 2.0],
        )
        .await
        .unwrap();
        assert_eq!(anchors.len(), 4 * 4 * 2 * 3 * 4); // h*w*sizes*ratios*4
        assert!(anchors.iter().all(|&x| x.is_finite()));
        // Anchors should be properly formatted (x1, y1, x2, y2)
        for i in 0..(anchors.len() / 4) {
            let x1 = anchors[i * 4];
            let x2 = anchors[i * 4 + 2];
            assert!(x2 > x1); // x2 should be greater than x1
        }
    }

    #[tokio::test]
    async fn test_anchor_generator_edge_cases() {
        let dev = get_test_device().await;

        // Single feature map location
        let anchors = anchor_generator(&dev.device, &dev.queue, 1, 1, 8, &[16.0], &[1.0])
            .await
            .unwrap();
        assert_eq!(anchors.len(), 1 * 1 * 1 * 1 * 4); // 4 coordinates
        assert!(anchors.iter().all(|&x| x.is_finite()));

        // Test with single aspect ratio
        let anchors = anchor_generator(&dev.device, &dev.queue, 2, 2, 16, &[32.0], &[1.0])
            .await
            .unwrap();
        assert_eq!(anchors.len(), 2 * 2 * 1 * 1 * 4);
    }

    #[tokio::test]
    async fn test_anchor_generator_boundary() {
        let dev = get_test_device().await;

        // Test with different strides
        let anchors1 = anchor_generator(&dev.device, &dev.queue, 3, 3, 8, &[16.0], &[1.0])
            .await
            .unwrap();
        let anchors2 = anchor_generator(&dev.device, &dev.queue, 3, 3, 16, &[16.0], &[1.0])
            .await
            .unwrap();

        assert!(anchors1.iter().all(|&x| x.is_finite()));
        assert!(anchors2.iter().all(|&x| x.is_finite()));
        // Different strides should produce different anchor positions
        assert_ne!(anchors1, anchors2);

        // Larger stride should produce larger coordinate values
        assert!(anchors2.iter().sum::<f32>() > anchors1.iter().sum::<f32>());
    }

    #[tokio::test]
    async fn test_anchor_generator_large_batch() {
        let dev = get_test_device().await;

        // Large feature map with multiple scales and ratios
        let feature_h = 16;
        let feature_w = 16;
        let sizes = vec![32.0, 64.0, 128.0];
        let ratios = vec![0.5, 1.0, 2.0];

        let anchors = anchor_generator(
            &dev.device,
            &dev.queue,
            feature_h,
            feature_w,
            16,
            &sizes,
            &ratios,
        )
        .await
        .unwrap();

        assert_eq!(
            anchors.len(),
            feature_h * feature_w * sizes.len() * ratios.len() * 4
        );
        assert!(anchors.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_anchor_generator_precision() {
        let dev = get_test_device().await;

        // Test with known values - single anchor at (0,0)
        let anchors = anchor_generator(&dev.device, &dev.queue, 1, 1, 16, &[32.0], &[1.0])
            .await
            .unwrap();

        // Center should be at (8, 8) - stride/2
        // Size=32, ratio=1.0 → w=h=32
        // Anchor box: [cx-w/2, cy-h/2, cx+w/2, cy+h/2]
        // = [8-16, 8-16, 8+16, 8+16] = [-8, -8, 24, 24]
        assert!((anchors[0] + 8.0).abs() < 1e-5); // x1
        assert!((anchors[1] + 8.0).abs() < 1e-5); // y1
        assert!((anchors[2] - 24.0).abs() < 1e-5); // x2
        assert!((anchors[3] - 24.0).abs() < 1e-5); // y2
    }
}
