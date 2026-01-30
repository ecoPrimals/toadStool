//! GINConv - Graph Isomorphism Network (Xu et al.)
//!
//! Maximally expressive GNN using sum aggregation and MLP.
//! (1 + epsilon) * h_v + sum(h_u for u in N(v))

pub async fn gin_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    mlp_weights: &[f32],
    epsilon: f32,
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Sum aggregation
    let mut aggregated = vec![0.0f32; num_nodes * in_features];
    
    for node in 0..num_nodes {
        // (1 + epsilon) * self
        for f in 0..in_features {
            aggregated[node * in_features + f] = 
                (1.0 + epsilon) * node_features[node * in_features + f];
        }
    }
    
    // Add neighbors
    for &(src, dst) in edge_index {
        for f in 0..in_features {
            aggregated[dst * in_features + f] += node_features[src * in_features + f];
        }
    }
    
    // Apply MLP (simplified: single linear layer)
    let mut output = vec![0.0f32; num_nodes * out_features];
    for node in 0..num_nodes {
        for out_f in 0..out_features {
            for in_f in 0..in_features {
                output[node * out_features + out_f] += 
                    aggregated[node * in_features + in_f]
                    * mlp_weights[in_f * out_features + out_f];
            }
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
    async fn test_gin_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 5 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4)];
        let mlp_weights = vec![0.1; 8 * 16];
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.0, 5, 8, 16).await.unwrap();
        assert_eq!(output.len(), 5 * 16);
    }
}
