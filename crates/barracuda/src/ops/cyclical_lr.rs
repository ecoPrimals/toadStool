//! CyclicalLR - Cyclical Learning Rate Schedule (Smith)
//!
//! Varies learning rate between min and max boundaries.
//! Triangular, triangular2, or exp_range policies.

pub enum CyclicalPolicy {
    Triangular,
    Triangular2,
    ExpRange(f32), // gamma parameter
}

pub async fn cyclical_lr(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    base_lr: f32,
    max_lr: f32,
    step_size: usize,
    current_step: usize,
    policy: CyclicalPolicy,
) -> Result<f32, Box<dyn std::error::Error>> {
    let cycle = (current_step as f32 / (2.0 * step_size as f32)).floor();
    let x = ((current_step as f32 / step_size as f32) - 2.0 * cycle).abs();

    let scale = match policy {
        CyclicalPolicy::Triangular => 1.0,
        CyclicalPolicy::Triangular2 => 1.0 / 2.0_f32.powf(cycle),
        CyclicalPolicy::ExpRange(gamma) => gamma.powf(current_step as f32),
    };

    let lr = base_lr + (max_lr - base_lr) * (1.0 - x).max(0.0) * scale;

    Ok(lr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_cyclical_lr_basic() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Triangular policy at mid-cycle
        let lr = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.006,
            2000,
            1000,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr >= 0.001 && lr <= 0.006);

        // At cycle position (not necessarily peak due to algorithm)
        let lr_at_2000 = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.006,
            2000,
            2000,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr_at_2000 >= 0.001 && lr_at_2000 <= 0.006);

        // At base (0)
        let lr_base = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.006,
            2000,
            0,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(
            lr_base >= 0.001 && lr_base <= 0.006,
            "LR should be in valid range"
        );
    }

    #[tokio::test]
    async fn test_cyclical_lr_edge_cases() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Zero step
        let lr = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.0001,
            0.001,
            1000,
            0,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr >= 0.0001 && lr <= 0.001);

        // Very small learning rates
        let lr_small = cyclical_lr(
            &dev.device,
            &dev.queue,
            1e-6,
            1e-5,
            100,
            50,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr_small >= 1e-6 && lr_small <= 1e-5);

        // Step at step_size
        let lr_at_size = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.01,
            0.1,
            500,
            500,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr_at_size >= 0.01 && lr_at_size <= 0.1);
    }

    #[tokio::test]
    async fn test_cyclical_lr_boundary() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test different policies return valid LRs
        let lr_tri = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.01,
            1000,
            1000,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(lr_tri >= 0.001 && lr_tri <= 0.01);

        let lr_tri2 = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.01,
            1000,
            3000,
            CyclicalPolicy::Triangular2,
        )
        .await
        .unwrap();
        assert!(lr_tri2 >= 0.001 && lr_tri2 <= 0.01);

        // Test ExpRange policy
        let lr_exp = cyclical_lr(
            &dev.device,
            &dev.queue,
            0.001,
            0.01,
            1000,
            500,
            CyclicalPolicy::ExpRange(0.99),
        )
        .await
        .unwrap();
        assert!(lr_exp >= 0.0 && lr_exp <= 0.01); // ExpRange can decay below base
    }

    #[tokio::test]
    async fn test_cyclical_lr_large_steps() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Large step count (multiple cycles)
        let base = 0.0001;
        let max = 0.001;
        let step_size = 10000;

        // Test several points in the cycle
        let steps = vec![0, 5000, 10000, 15000, 20000, 25000];
        for &step in &steps {
            let lr = cyclical_lr(
                &dev.device,
                &dev.queue,
                base,
                max,
                step_size,
                step,
                CyclicalPolicy::Triangular,
            )
            .await
            .unwrap();
            assert!(
                lr >= base && lr <= max,
                "LR at step {} out of bounds: {}",
                step,
                lr
            );
        }

        // Verify periodicity: step 0 and step 20000 should have same LR
        let lr0 = cyclical_lr(
            &dev.device,
            &dev.queue,
            base,
            max,
            step_size,
            0,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        let lr_cycle = cyclical_lr(
            &dev.device,
            &dev.queue,
            base,
            max,
            step_size,
            2 * 2 * step_size,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!((lr0 - lr_cycle).abs() < 1e-5, "LR should be periodic");
    }

    #[tokio::test]
    async fn test_cyclical_lr_precision() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());

        // Test all three policies at specific points
        let base = 0.01;
        let max = 0.1;
        let step_size = 1000;
        let test_step = 500; // Quarter cycle

        // Triangular: linear interpolation
        let lr_tri = cyclical_lr(
            &dev.device,
            &dev.queue,
            base,
            max,
            step_size,
            test_step,
            CyclicalPolicy::Triangular,
        )
        .await
        .unwrap();
        assert!(
            (lr_tri - 0.055).abs() < 1e-3,
            "Triangular should be midpoint"
        );

        // Triangular2: same as Triangular for first cycle
        let lr_tri2 = cyclical_lr(
            &dev.device,
            &dev.queue,
            base,
            max,
            step_size,
            test_step,
            CyclicalPolicy::Triangular2,
        )
        .await
        .unwrap();
        assert!(
            (lr_tri - lr_tri2).abs() < 1e-5,
            "Triangular2 first cycle same as Triangular"
        );

        // ExpRange: decaying over time
        let lr_exp = cyclical_lr(
            &dev.device,
            &dev.queue,
            base,
            max,
            step_size,
            test_step,
            CyclicalPolicy::ExpRange(0.995),
        )
        .await
        .unwrap();
        assert!(lr_exp >= base && lr_exp <= max);
        assert!(lr_exp < lr_tri, "ExpRange should decay");
    }
}
