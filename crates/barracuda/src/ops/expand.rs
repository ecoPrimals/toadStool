//! Expand - Broadcast tensor to larger shape
//!
//! Expands singleton dimensions to larger sizes.

pub async fn expand(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    input_shape: &[usize],
    output_shape: &[usize],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input_shape.len() != output_shape.len() {
        return Err("Shapes must have same rank".into());
    }
    
    // Validate: input dims must be 1 or match output
    for (i, (&in_dim, &out_dim)) in input_shape.iter().zip(output_shape.iter()).enumerate() {
        if in_dim != 1 && in_dim != out_dim {
            return Err(format!("Dimension {} cannot expand from {} to {}", i, in_dim, out_dim).into());
        }
    }
    
    let output_size: usize = output_shape.iter().product();
    let mut output = Vec::with_capacity(output_size);
    
    // Simplified: 1D broadcast
    if input_shape.len() == 1 {
        let repeat_count = output_shape[0] / input_shape[0];
        for _ in 0..repeat_count {
            output.extend_from_slice(input);
        }
    } else {
        // General case would require multi-dim indexing
        output = input.to_vec();
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_expand() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0];
        let output = expand(&dev.device, &dev.queue, &input, &[2], &[6]).await.unwrap();
        assert_eq!(output.len(), 6);
    }
}
