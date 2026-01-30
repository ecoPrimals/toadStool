//! CyclicalLR - Cyclical Learning Rate Schedule (Smith)
//!
//! Varies learning rate between min and max boundaries.
//! Triangular, triangular2, or exp_range policies.

pub enum CyclicalPolicy {
    Triangular,
    Triangular2,
    ExpRange(f32),  // gamma parameter
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
    async fn test_cyclical_lr() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let lr = cyclical_lr(&dev.device, &dev.queue, 0.001, 0.006, 2000, 1000, CyclicalPolicy::Triangular).await.unwrap();
        assert!(lr >= 0.001 && lr <= 0.006);
    }
}
