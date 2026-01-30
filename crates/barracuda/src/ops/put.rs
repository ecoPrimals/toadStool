//! Put - Scatter operation with indexing
//!
//! Places values into output tensor at specified indices.

pub async fn put(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    size: usize,
    indices: &[usize],
    values: &[f32],
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if indices.len() != values.len() {
        return Err("Indices and values must have same length".into());
    }
    
    let mut output = vec![0.0f32; size];
    
    for (idx, value) in indices.iter().zip(values.iter()) {
        if *idx < size {
            output[*idx] = *value;
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_put() {
        let (device, queue) = crate::test_utils::create_device().await.unwrap();
        let indices = vec![0, 2, 4];
        let values = vec![10.0, 30.0, 50.0];
        let output = put(&device, &queue, 5, &indices, &values).await.unwrap();
        assert_eq!(output, vec![10.0, 0.0, 30.0, 0.0, 50.0]);
    }
}
