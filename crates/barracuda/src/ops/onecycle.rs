//! OneCycle - 1cycle Learning Rate Policy (Smith)
//!
//! Single cycle: warmup to max_lr, then anneal to min_lr.
//! Enables super-convergence with high learning rates.

pub async fn onecycle_lr(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    max_lr: f32,
    total_steps: usize,
    current_step: usize,
    pct_start: f32,  // Percentage of cycle spent warming up (default 0.3)
    div_factor: f32, // Initial lr = max_lr / div_factor
    final_div_factor: f32, // Final lr = max_lr / final_div_factor
) -> Result<f32, Box<dyn std::error::Error>> {
    if current_step >= total_steps {
        return Err("Current step exceeds total steps".into());
    }
    
    let step = current_step as f32;
    let total = total_steps as f32;
    let warmup_steps = (pct_start * total).floor();
    
    let lr = if step < warmup_steps {
        // Warmup phase: increase from initial_lr to max_lr
        let initial_lr = max_lr / div_factor;
        let pct = step / warmup_steps;
        initial_lr + (max_lr - initial_lr) * pct
    } else {
        // Annealing phase: decrease from max_lr to final_lr
        let final_lr = max_lr / final_div_factor;
        let pct = (step - warmup_steps) / (total - warmup_steps);
        max_lr - (max_lr - final_lr) * pct
    };
    
    Ok(lr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_onecycle() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let lr_start = onecycle_lr(&dev.device, &dev.queue, 0.01, 10000, 0, 0.3, 25.0, 10000.0).await.unwrap();
        let lr_peak = onecycle_lr(&dev.device, &dev.queue, 0.01, 10000, 3000, 0.3, 25.0, 10000.0).await.unwrap();
        let lr_end = onecycle_lr(&dev.device, &dev.queue, 0.01, 10000, 9999, 0.3, 25.0, 10000.0).await.unwrap();
        assert!(lr_peak > lr_start);
        assert!(lr_end < lr_peak);
    }
}
