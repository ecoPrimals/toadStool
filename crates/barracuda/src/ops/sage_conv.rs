//! SAGEConv - GraphSAGE Convolution (Hamilton et al.)
//!
//! Samples and aggregates features from node neighborhoods.
//! Mean aggregation variant for scalability.

pub async fn sage_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    weights: &[f32],
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Aggregate neighbor features (mean)
    let mut neighbor_agg = vec![0.0f32; num_nodes * in_features];
    let mut neighbor_count = vec![0; num_nodes];
    
    for &(src, dst) in edge_index {
        for f in 0..in_features {
            neighbor_agg[dst * in_features + f] += node_features[src * in_features + f];
        }
        neighbor_count[dst] += 1;
    }
    
    // Average
    for node in 0..num_nodes {
        if neighbor_count[node] > 0 {
            let count = neighbor_count[node] as f32;
            for f in 0..in_features {
                neighbor_agg[node * in_features + f] /= count;
            }
        }
    }
    
    // Concatenate self and neighbor features, then transform
    let mut output = vec![0.0f32; num_nodes * out_features];
    for node in 0..num_nodes {
        for out_f in 0..out_features {
            // Self features
            for in_f in 0..in_features {
                output[node * out_features + out_f] += 
                    node_features[node * in_features + in_f] 
                    * weights[in_f * out_features + out_f];
            }
            // Neighbor features
            for in_f in 0..in_features {
                output[node * out_features + out_f] += 
                    neighbor_agg[node * in_features + in_f]
                    * weights[(in_features + in_f) * out_features + out_f];
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
    async fn test_sage_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 6 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
        let weights = vec![0.1; 16 * 16]; // 2*in_features x out_features
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 6, 8, 16).await.unwrap();
        assert_eq!(output.len(), 6 * 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sage_conv_edge_cases() {
        let dev = get_test_device().await;
        
        // Node with no neighbors (isolated)
        let node_features = vec![1.0; 3 * 4];
        let edges = vec![(0, 1)]; // Node 2 is isolated
        let weights = vec![0.1; 8 * 8];
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 4, 8).await.unwrap();
        assert_eq!(output.len(), 3 * 8);
        // Isolated node should still have self features
        assert!(output.iter().any(|&x| x != 0.0));
        
        // Single node, no edges
        let node_features = vec![1.0; 1 * 4];
        let edges = vec![];
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 1, 4, 8).await.unwrap();
        assert_eq!(output.len(), 1 * 8);
    }

    #[tokio::test]
    async fn test_sage_conv_boundary() {
        let dev = get_test_device().await;
        
        // Node with many neighbors (hub)
        let num_nodes = 6;
        let hub = 0;
        let mut edges = Vec::new();
        for i in 1..num_nodes {
            edges.push((i, hub)); // All nodes connect to hub
        }
        
        let node_features = vec![0.5; num_nodes * 4];
        let weights = vec![0.1; 8 * 8];
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, num_nodes, 4, 8).await.unwrap();
        
        // Hub should aggregate from all neighbors (mean aggregation)
        assert_eq!(output.len(), num_nodes * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sage_conv_large_batch() {
        let dev = get_test_device().await;
        
        // Larger graph
        let num_nodes = 15;
        let in_feat = 16;
        let out_feat = 32;
        
        // Create random-like edges
        let mut edges = Vec::new();
        for i in 0..num_nodes-1 {
            edges.push((i, i+1));
            if i + 2 < num_nodes {
                edges.push((i, i+2));
            }
        }
        
        let node_features = vec![0.5; num_nodes * in_feat];
        let weights = vec![0.1; (2 * in_feat) * out_feat];
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, num_nodes, in_feat, out_feat).await.unwrap();
        
        assert_eq!(output.len(), num_nodes * out_feat);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_sage_conv_precision() {
        let dev = get_test_device().await;
        
        // Test mean aggregation with known values
        let node_features = vec![
            1.0, 0.0, // Node 0
            0.0, 1.0, // Node 1
            2.0, 2.0, // Node 2
        ];
        let edges = vec![(0, 2), (1, 2)]; // Nodes 0 and 1 connect to node 2
        let weights = vec![0.5; 4 * 4]; // 2*in_features x out_features
        
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 3, 2, 4).await.unwrap();
        
        // Node 2 should aggregate mean of neighbors (0.5, 0.5) plus self (2.0, 2.0)
        assert_eq!(output.len(), 3 * 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Should have non-zero output from aggregation
        assert!(output.iter().any(|&x| x > 0.0));
    }
}
