//! GraphConv - Basic graph convolution
//!
//! Performs message passing on graph structures.
//! Foundation for Graph Neural Networks (GNNs).

pub async fn graph_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],    // [num_nodes, in_features]
    edge_index: &[(usize, usize)],  // List of edges (src, dst)
    weights: &[f32],          // [in_features, out_features]
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    if node_features.len() != num_nodes * in_features {
        return Err("Node features dimension mismatch".into());
    }
    
    // Initialize output with self-loops
    let mut output = vec![0.0f32; num_nodes * out_features];
    
    // Aggregate messages from neighbors
    for &(src, dst) in edge_index {
        if src >= num_nodes || dst >= num_nodes {
            return Err("Edge index out of bounds".into());
        }
        
        // Message from src to dst
        for out_f in 0..out_features {
            let mut msg = 0.0;
            for in_f in 0..in_features {
                let feat_idx = src * in_features + in_f;
                let weight_idx = in_f * out_features + out_f;
                msg += node_features[feat_idx] * weights[weight_idx];
            }
            output[dst * out_features + out_f] += msg;
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
    async fn test_graph_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 3 * 4]; // 3 nodes, 4 features
        let edges = vec![(0, 1), (1, 2), (2, 0)]; // Simple cycle
        let weights = vec![0.1; 4 * 8]; // 4 in, 8 out
        let output = graph_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 4, 8).await.unwrap();
        assert_eq!(output.len(), 3 * 8);
    }
}
