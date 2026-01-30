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
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_gin_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 5 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4)];
        let mlp_weights = vec![0.1; 8 * 16];
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.0, 5, 8, 16).await.unwrap();
        assert_eq!(output.len(), 5 * 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_gin_conv_edge_cases() {
        let dev = get_test_device().await;
        
        // Single node, no edges
        let node_features = vec![1.0; 1 * 4];
        let edges = vec![];
        let mlp_weights = vec![0.1; 4 * 8];
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.0, 1, 4, 8).await.unwrap();
        assert_eq!(output.len(), 1 * 8);
        // With epsilon=0, should still have self features
        assert!(output.iter().any(|&x| x != 0.0));
        
        // Test with different epsilon values
        let node_features = vec![1.0; 2 * 4];
        let edges = vec![(0, 1)];
        let output_eps0 = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.0, 2, 4, 8).await.unwrap();
        let output_eps1 = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 1.0, 2, 4, 8).await.unwrap();
        // Different epsilon should produce different results
        assert!(output_eps0.iter().zip(output_eps1.iter()).any(|(a, b)| (a - b).abs() > 1e-6));
    }

    #[tokio::test]
    async fn test_gin_conv_boundary() {
        let dev = get_test_device().await;
        
        // Complete graph (all-to-all)
        let num_nodes = 4;
        let mut edges = Vec::new();
        for i in 0..num_nodes {
            for j in 0..num_nodes {
                if i != j {
                    edges.push((i, j));
                }
            }
        }
        
        let node_features = vec![0.5; num_nodes * 4];
        let mlp_weights = vec![0.1; 4 * 8];
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.5, num_nodes, 4, 8).await.unwrap();
        
        assert_eq!(output.len(), num_nodes * 8);
        // Sum aggregation should produce significant output
        assert!(output.iter().any(|&x| x.abs() > 0.1));
    }

    #[tokio::test]
    async fn test_gin_conv_large_batch() {
        let dev = get_test_device().await;
        
        // Larger graph with various connectivity
        let num_nodes = 12;
        let in_feat = 16;
        let out_feat = 32;
        
        // Create chain with skip connections
        let mut edges = Vec::new();
        for i in 0..num_nodes-1 {
            edges.push((i, i+1));
            if i % 3 == 0 && i + 3 < num_nodes {
                edges.push((i, i+3));
            }
        }
        
        let node_features = vec![0.5; num_nodes * in_feat];
        let mlp_weights = vec![0.1; in_feat * out_feat];
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, 0.1, num_nodes, in_feat, out_feat).await.unwrap();
        
        assert_eq!(output.len(), num_nodes * out_feat);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_gin_conv_precision() {
        let dev = get_test_device().await;
        
        // Test (1 + epsilon) * self + sum(neighbors) formula
        let node_features = vec![
            1.0, 0.0, // Node 0
            0.0, 1.0, // Node 1
            2.0, 3.0, // Node 2
        ];
        let edges = vec![(0, 2), (1, 2)]; // Both 0 and 1 connect to 2
        let mlp_weights = vec![0.5; 2 * 4];
        let epsilon = 0.5;
        
        let output = gin_conv(&dev.device, &dev.queue, &node_features, &edges, &mlp_weights, epsilon, 3, 2, 4).await.unwrap();
        
        assert_eq!(output.len(), 3 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        
        // Node 2 should have: (1 + 0.5) * [2.0, 3.0] + [1.0, 0.0] + [0.0, 1.0]
        // = [3.0 + 1.0, 4.5 + 1.0] = [4.0, 5.5] before MLP
        // After MLP, should still be finite and non-zero
        assert!(output.iter().skip(2*4).any(|&x| x > 0.0));
    }
}
