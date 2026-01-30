//! GLU - Gated Linear Unit activation
//!
//! ## Algorithm
//!
//! ```text
//! GLU(x) = a ⊙ sigmoid(b)
//! ```
//!
//! Where x is split into two halves: a and b.
//! Used in language models and transformers.

/// GLU activation
pub async fn glu(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32], // Length must be even
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() % 2 != 0 {
        return Err("Input length must be even for GLU".into());
    }
    
    let half = input.len() / 2;
    let mut output = Vec::with_capacity(half);
    
    for i in 0..half {
        let a = input[i];
        let b = input[half + i];
        let sigmoid_b = 1.0 / (1.0 + (-b).exp());
        output.push(a * sigmoid_b);
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_glu() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let device = &dev.device;
        let queue = &dev.queue;
        let input = vec![1.0, 2.0, 0.0, 0.0]; // Split: [1,2] and [0,0]
        let output = glu(&device, &queue, &input).await.unwrap();
        assert_eq!(output.len(), 2);
    }
}
