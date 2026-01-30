//! EdgeConv - Edge Convolution for Dynamic Graphs (Wang et al.)
//!
//! Computes edge features from node pairs: h(x_i, x_j - x_i)
//! Used in point cloud processing (DGCNN).

pub async fn edge_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    weights: &[f32],
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; num_nodes * out_features];
    
    // For each edge, compute edge features
    for &(src, dst) in edge_index {
        // Edge feature: concatenate [x_dst, x_src - x_dst]
        let mut edge_feat = vec![0.0f32; 2 * in_features];
        
        for f in 0..in_features {
            // x_dst
            edge_feat[f] = node_features[dst * in_features + f];
            // x_src - x_dst
            edge_feat[in_features + f] = 
                node_features[src * in_features + f] 
                - node_features[dst * in_features + f];
        }
        
        // Transform edge features with MLP
        for out_f in 0..out_features {
            let mut val = 0.0;
            for in_f in 0..(2 * in_features) {
                val += edge_feat[in_f] * weights[in_f * out_features + out_f];
            }
            output[dst * out_features + out_f] += val.max(0.0); // ReLU
        }
    }
    
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    
    #[tokio::test]
    async fn test_edge_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let weights = vec![0.1; 16 * 16]; // 2*in_features x out_features
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 4, 8, 16).await.unwrap();
        assert_eq!(output.len(), 4 * 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_edge_conv_edge_cases() {
        let dev = get_test_device().await;
        
        // No edges (all nodes isolated)
        let node_features = vec![1.0; 3 * 4];
        let edges = vec![];
        let weights = vec![0.1; 8 * 8];
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 4, 8).await.unwrap();
        assert_eq!(output.len(), 3 * 8);
        // All zeros (no edge features computed)
        assert!(output.iter().all(|&x| x == 0.0));
        
        // Single edge
        let edges = vec![(0, 1)];
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 4, 8).await.unwrap();
        assert_eq!(output.len(), 3 * 8);
        // Node 1 should have edge features, others zero
        assert!(output[8..16].iter().any(|&x| x != 0.0)); // Node 1
    }

    #[tokio::test]
    async fn test_edge_conv_boundary() {
        let dev = get_test_device().await;
        
        // Test with distinct node features (point cloud simulation)
        let node_features = vec![
            1.0, 0.0, 0.0, 0.0, // Node 0
            0.0, 1.0, 0.0, 0.0, // Node 1
            0.0, 0.0, 1.0, 0.0, // Node 2
            0.0, 0.0, 0.0, 1.0, // Node 3
        ];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let weights = vec![0.1; 8 * 8];
        
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 4, 4, 8).await.unwrap();
        
        // Edge features capture spatial relationships
        assert_eq!(output.len(), 4 * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
        assert!(output.iter().all(|&x| x >= 0.0)); // ReLU applied
    }

    #[tokio::test]
    async fn test_edge_conv_large_batch() {
        let dev = get_test_device().await;
        
        // Larger point cloud (e.g., DGCNN)
        let num_nodes = 20;
        let in_feat = 3; // 3D points
        let out_feat = 64;
        
        // K-nearest neighbors simulation (simplified)
        let mut edges = Vec::new();
        for i in 0..num_nodes {
            // Connect each node to next 3 nodes (cyclic)
            for k in 1..=3 {
                let j = (i + k) % num_nodes;
                edges.push((j, i));
            }
        }
        
        let node_features = vec![0.5; num_nodes * in_feat];
        let weights = vec![0.1; (2 * in_feat) * out_feat];
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, num_nodes, in_feat, out_feat).await.unwrap();
        
        assert_eq!(output.len(), num_nodes * out_feat);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_edge_conv_precision() {
        let dev = get_test_device().await;
        
        // Test edge feature computation: [x_dst, x_src - x_dst]
        let node_features = vec![
            1.0, 2.0, // Node 0
            3.0, 4.0, // Node 1
            5.0, 6.0, // Node 2
        ];
        let edges = vec![(0, 1), (1, 2)];
        let weights = vec![0.5; 4 * 4]; // 2*in_features x out_features
        
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 2, 4).await.unwrap();
        
        assert_eq!(output.len(), 3 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        
        // Edge from 0 to 1: [3.0, 4.0, 1.0-3.0, 2.0-4.0] = [3.0, 4.0, -2.0, -2.0]
        // After ReLU, negative values should be zero
        // Node 1 should have non-zero output
        assert!(output[4..8].iter().any(|&x| x > 0.0));
    }
}
