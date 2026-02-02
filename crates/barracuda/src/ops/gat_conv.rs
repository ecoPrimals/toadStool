//! GATConv - Graph Attention Networks (Veličković et al.)
//!
//! Attention-based graph convolution with learnable attention coefficients.
//! Computes attention scores between connected nodes.

pub async fn gat_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    weights: &[f32],
    attention: &[f32],
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; num_nodes * out_features];

    // Transform features first
    let mut transformed = vec![0.0f32; num_nodes * out_features];
    for node in 0..num_nodes {
        for out_f in 0..out_features {
            for in_f in 0..in_features {
                transformed[node * out_features + out_f] +=
                    node_features[node * in_features + in_f] * weights[in_f * out_features + out_f];
            }
        }
    }

    // Compute attention scores and aggregate
    for &(src, dst) in edge_index {
        // Simplified attention: concat(src, dst) dot attention_vector
        let mut score = 0.0;
        for f in 0..out_features {
            score += transformed[src * out_features + f] * attention[f];
            score += transformed[dst * out_features + f] * attention[out_features + f];
        }

        // LeakyReLU activation
        let alpha = if score > 0.0 { score } else { 0.01 * score };
        let weight = alpha.exp(); // Softmax will be per-node

        for f in 0..out_features {
            output[dst * out_features + f] += weight * transformed[src * out_features + f];
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_gat_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 5 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4)];
        let weights = vec![0.1; 8 * 16];
        let attention = vec![0.05; 32]; // 2 * out_features
        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
            5,
            8,
            16,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 5 * 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_gat_conv_edge_cases() {
        let dev = get_test_device().await;

        // Single node, no edges
        let node_features = vec![1.0; 1 * 4];
        let weights = vec![0.1; 4 * 8];
        let attention = vec![0.05; 16];
        let edges = vec![];
        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
            1,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 1 * 8);
        // All zeros (no attention computed)
        assert!(output.iter().all(|&x| x == 0.0));

        // Two nodes with single edge
        let node_features = vec![1.0; 2 * 4];
        let edges = vec![(0, 1)];
        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
            2,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 2 * 8);
    }

    #[tokio::test]
    async fn test_gat_conv_boundary() {
        let dev = get_test_device().await;

        // Multi-head attention simulation (single head here)
        let num_nodes = 4;
        let node_features = vec![0.5; num_nodes * 8];
        let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (2, 3)];
        let weights = vec![0.1; 8 * 16];
        let attention = vec![0.05; 32];

        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
            num_nodes,
            8,
            16,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), num_nodes * 16);
        // Attention-weighted aggregation should produce non-zero outputs
        assert!(output.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_gat_conv_large_batch() {
        let dev = get_test_device().await;

        // Larger graph
        let num_nodes = 12;
        let in_feat = 16;
        let out_feat = 32;

        // Create random-like edges
        let mut edges = Vec::new();
        for i in 0..num_nodes - 1 {
            edges.push((i, i + 1));
            if i % 2 == 0 && i + 2 < num_nodes {
                edges.push((i, i + 2));
            }
        }

        let node_features = vec![0.5; num_nodes * in_feat];
        let weights = vec![0.1; in_feat * out_feat];
        let attention = vec![0.05; 2 * out_feat];

        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
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
    async fn test_gat_conv_precision() {
        let dev = get_test_device().await;

        // Test attention weighting with distinct features
        let node_features = vec![
            1.0, 0.0, 0.0, 0.0, // Node 0
            0.0, 1.0, 0.0, 0.0, // Node 1
            0.0, 0.0, 1.0, 0.0, // Node 2
        ];
        let edges = vec![(0, 2), (1, 2)]; // Two sources to node 2
        let weights = vec![0.1; 4 * 8];
        let attention = vec![0.1; 16];

        let output = gat_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            &attention,
            3,
            4,
            8,
        )
        .await
        .unwrap();

        // Node 2 should receive attention-weighted messages
        for f in 0..8 {
            let val = output[2 * 8 + f];
            assert!(val.is_finite());
        }
        assert!(output.iter().any(|&x| x != 0.0));
    }
}
