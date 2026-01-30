//! GraphBatchNorm - Batch normalization for graphs
//!
//! Standard batch normalization adapted for graph data.
//! Normalizes across all nodes in the batch.

pub async fn graph_batch_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    gamma: &[f32],  // Scale parameter
    beta: &[f32],   // Shift parameter
    num_nodes: usize,
    num_features: usize,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if gamma.len() != num_features || beta.len() != num_features {
        return Err("Gamma/beta dimension mismatch".into());
    }
    
    // Compute mean per feature across all nodes
    let mut mean = vec![0.0f32; num_features];
    for node in 0..num_nodes {
        for f in 0..num_features {
            mean[f] += node_features[node * num_features + f];
        }
    }
    for f in 0..num_features {
        mean[f] /= num_nodes as f32;
    }
    
    // Compute variance
    let mut variance = vec![0.0f32; num_features];
    for node in 0..num_nodes {
        for f in 0..num_features {
            let diff = node_features[node * num_features + f] - mean[f];
            variance[f] += diff * diff;
        }
    }
    for f in 0..num_features {
        variance[f] /= num_nodes as f32;
    }
    
    // Normalize and apply affine transform
    let mut output = vec![0.0f32; num_nodes * num_features];
    for node in 0..num_nodes {
        for f in 0..num_features {
            let normalized = (node_features[node * num_features + f] - mean[f])
                           / (variance[f] + epsilon).sqrt();
            output[node * num_features + f] = gamma[f] * normalized + beta[f];
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_graph_batch_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 10 * 16];
        let gamma = vec![1.0; 16];
        let beta = vec![0.0; 16];
        let output = graph_batch_norm(&dev.device, &dev.queue, &node_features, &gamma, &beta, 10, 16, 1e-5).await.unwrap();
        assert_eq!(output.len(), 10 * 16);
    }
}
