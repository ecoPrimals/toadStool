//! Masked Select - Extract elements where mask is true
//!
//! Returns only elements where mask is true.

pub async fn masked_select(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    mask: &[bool],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() != mask.len() {
        return Err("Input and mask must have same length".into());
    }
    
    let output: Vec<f32> = input.iter().zip(mask.iter())
        .filter_map(|(&val, &m)| if m { Some(val) } else { None })
        .collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_masked_select() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mask = vec![true, false, true, false, true];
        let output = masked_select(&dev.device, &dev.queue, &input, &mask).await.unwrap();
        assert_eq!(output, vec![1.0, 3.0, 5.0]);
    }
}
