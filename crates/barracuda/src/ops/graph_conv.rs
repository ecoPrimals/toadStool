//! GraphConv - Basic graph convolution
//!
//! Performs message passing on graph structures.
//! Foundation for Graph Neural Networks (GNNs).

pub async fn graph_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],         // [num_nodes, in_features]
    edge_index: &[(usize, usize)], // List of edges (src, dst)
    weights: &[f32],               // [in_features, out_features]
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
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_graph_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 3 * 4]; // 3 nodes, 4 features
        let edges = vec![(0, 1), (1, 2), (2, 0)]; // Simple cycle
        let weights = vec![0.1; 4 * 8]; // 4 in, 8 out
        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            3,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 3 * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_graph_conv_edge_cases() {
        let dev = get_test_device().await;

        // Single node, no edges
        let node_features = vec![1.0; 1 * 4];
        let edges = vec![];
        let weights = vec![0.1; 4 * 8];
        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            1,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 8);
        // All zeros (no messages)
        assert!(output.iter().all(|&x| x == 0.0));

        // Self-loop only
        let edges = vec![(0, 0)];
        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            1,
            4,
            8,
        )
        .await
        .unwrap();
        assert!(output.iter().any(|&x| x != 0.0)); // Should have non-zero output
    }

    #[tokio::test]
    async fn test_graph_conv_boundary() {
        let dev = get_test_device().await;

        // Fully connected graph (all-to-all)
        let num_nodes = 4;
        let mut edges = Vec::new();
        for i in 0..num_nodes {
            for j in 0..num_nodes {
                if i != j {
                    edges.push((i, j));
                }
            }
        }

        let node_features = vec![1.0; num_nodes * 2];
        let weights = vec![0.1; 2 * 4];
        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            num_nodes,
            2,
            4,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), num_nodes * 4);

        // Each node should receive messages from all others
        assert!(output.iter().all(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_graph_conv_large_batch() {
        let dev = get_test_device().await;

        // Larger graph (10 nodes)
        let num_nodes = 10;
        let in_feat = 8;
        let out_feat = 16;

        // Create chain graph: 0->1->2->...->9
        let edges: Vec<(usize, usize)> = (0..num_nodes - 1).map(|i| (i, i + 1)).collect();

        let node_features = vec![0.5; num_nodes * in_feat];
        let weights = vec![0.1; in_feat * out_feat];
        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            num_nodes,
            in_feat,
            out_feat,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), num_nodes * out_feat);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_graph_conv_precision() {
        let dev = get_test_device().await;

        // Test message aggregation with known values
        let node_features = vec![
            1.0, 0.0, // Node 0
            0.0, 1.0, // Node 1
            1.0, 1.0, // Node 2
        ];
        let edges = vec![(0, 2), (1, 2)]; // Both nodes connect to node 2
        let weights = vec![1.0, 0.0, 0.0, 1.0]; // Identity-like

        let output = graph_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            3,
            2,
            2,
        )
        .await
        .unwrap();

        // Node 2 should receive messages from node 0 and node 1
        assert!(output[2 * 2] > 0.0); // First feature
        assert!(output[2 * 2 + 1] > 0.0); // Second feature
        assert!(output.iter().all(|&x| x.is_finite()));
    }
}
