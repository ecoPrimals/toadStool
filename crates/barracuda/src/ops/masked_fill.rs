//! MaskedFill - Conditional fill operation
//!
//! Fills elements where mask is true with a specified value.

pub async fn masked_fill(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    input: &[f32],
    mask: &[bool],
    fill_value: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if input.len() != mask.len() {
        return Err("Input and mask must have same length".into());
    }
    
    let output: Vec<f32> = input.iter().zip(mask.iter())
        .map(|(&x, &m)| if m { fill_value } else { x })
        .collect();
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_masked_fill() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mask = vec![false, true, false, true, false];
        let output = masked_fill(&device, &queue, &input, &mask, -999.0).await.unwrap();
        assert_eq!(output, vec![1.0, -999.0, 3.0, -999.0, 5.0]);
    }
}
