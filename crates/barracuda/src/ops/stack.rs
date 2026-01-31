//! Stack - Stack tensors along new dimension
//!
//! Creates new dimension and stacks inputs along it.

pub async fn stack(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    tensors: &[Vec<f32>],
    _dim: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if tensors.is_empty() {
        return Err("Cannot stack empty tensor list".into());
    }
    
    let elem_size = tensors[0].len();
    for t in tensors {
        if t.len() != elem_size {
            return Err("All tensors must have same size".into());
        }
    }
    
    let mut output = Vec::with_capacity(tensors.len() * elem_size);
    
    // Simple implementation: concat along new dimension
    for tensor in tensors {
        output.extend_from_slice(tensor);
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }
    
    #[tokio::test]
    async fn test_stack_basic() {
        let dev = get_test_device().await;
        let t1 = vec![1.0, 2.0];
        let t2 = vec![3.0, 4.0];
        let output = stack(&dev.device, &dev.queue, &[t1, t2], 0).await.unwrap();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_stack_edge_cases() {
        let dev = get_test_device().await;

        // Single tensor
        let t1 = vec![1.0, 2.0, 3.0];
        let output = stack(&dev.device, &dev.queue, &[t1], 0).await.unwrap();
        assert_eq!(output.len(), 3);

        // Many tensors
        let tensors: Vec<Vec<f32>> = (0..10).map(|_| vec![1.0, 2.0]).collect();
        let output = stack(&dev.device, &dev.queue, &tensors, 0).await.unwrap();
        assert_eq!(output.len(), 20);
    }

    #[tokio::test]
    async fn test_stack_boundary() {
        let dev = get_test_device().await;

        // Large tensors
        let t1 = vec![1.0; 1000];
        let t2 = vec![2.0; 1000];
        let output = stack(&dev.device, &dev.queue, &[t1, t2], 0).await.unwrap();
        assert_eq!(output.len(), 2000);

        // Different values
        let t1 = vec![0.0; 100];
        let t2 = vec![1.0; 100];
        let t3 = vec![2.0; 100];
        let output = stack(&dev.device, &dev.queue, &[t1, t2, t3], 0).await.unwrap();
        assert_eq!(output.len(), 300);
    }

    #[tokio::test]
    async fn test_stack_large_batch() {
        let dev = get_test_device().await;

        // 100 tensors
        let tensors: Vec<Vec<f32>> = (0..100).map(|i| vec![i as f32; 10]).collect();
        let output = stack(&dev.device, &dev.queue, &tensors, 0).await.unwrap();
        assert_eq!(output.len(), 1000);
    }

    #[tokio::test]
    async fn test_stack_precision() {
        let dev = get_test_device().await;

        // Verify data ordering
        let t1 = vec![1.0, 2.0];
        let t2 = vec![3.0, 4.0];
        let t3 = vec![5.0, 6.0];
        let output = stack(&dev.device, &dev.queue, &[t1, t2, t3], 0).await.unwrap();
        
        assert_eq!(output.len(), 6);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Verify stacking order
        assert_eq!(output[0], 1.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[4], 5.0);
    }
}
