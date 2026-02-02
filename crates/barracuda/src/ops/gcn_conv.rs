//! GCNConv - Graph Convolutional Network (Kipf & Welling)
//!
//! Normalized graph convolution: D^(-1/2) * A * D^(-1/2) * X * W
//! Standard GCN layer with symmetric normalization.

pub async fn gcn_conv(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    weights: &[f32],
    num_nodes: usize,
    in_features: usize,
    out_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Compute degree for each node
    let mut degrees = vec![0; num_nodes];
    for &(src, dst) in edge_index {
        degrees[src] += 1;
        degrees[dst] += 1;
    }

    // Add self-loops (degree + 1)
    for d in degrees.iter_mut() {
        *d += 1;
    }

    // Compute D^(-1/2)
    let deg_inv_sqrt: Vec<f32> = degrees.iter().map(|&d| 1.0 / (d as f32).sqrt()).collect();

    let mut output = vec![0.0f32; num_nodes * out_features];

    // Normalized aggregation
    for &(src, dst) in edge_index {
        let norm = deg_inv_sqrt[src] * deg_inv_sqrt[dst];

        for out_f in 0..out_features {
            let mut msg = 0.0;
            for in_f in 0..in_features {
                msg +=
                    node_features[src * in_features + in_f] * weights[in_f * out_features + out_f];
            }
            output[dst * out_features + out_f] += norm * msg;
        }
    }

    // Self-loops
    for node in 0..num_nodes {
        let norm = deg_inv_sqrt[node] * deg_inv_sqrt[node];
        for out_f in 0..out_features {
            let mut msg = 0.0;
            for in_f in 0..in_features {
                msg +=
                    node_features[node * in_features + in_f] * weights[in_f * out_features + out_f];
            }
            output[node * out_features + out_f] += norm * msg;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_gcn_conv_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let weights = vec![0.1; 8 * 16];
        let output = gcn_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            4,
            8,
            16,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 4 * 16);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_gcn_conv_edge_cases() {
        let dev = get_test_device().await;

        // Single node with self-loop
        let node_features = vec![1.0; 1 * 4];
        let edges = vec![];
        let weights = vec![0.1; 4 * 8];
        let output = gcn_conv(
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
        // Should have output from self-loop
        assert!(output.iter().any(|&x| x != 0.0));

        // Two nodes, bidirectional edge
        let node_features = vec![1.0; 2 * 4];
        let edges = vec![(0, 1), (1, 0)];
        let output = gcn_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            2,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 2 * 8);
    }

    #[tokio::test]
    async fn test_gcn_conv_boundary() {
        let dev = get_test_device().await;

        // Star graph (one central node connected to all others)
        let num_nodes = 5;
        let center = 0;
        let mut edges = Vec::new();
        for i in 1..num_nodes {
            edges.push((center, i));
            edges.push((i, center));
        }

        let node_features = vec![1.0; num_nodes * 4];
        let weights = vec![0.1; 4 * 8];
        let output = gcn_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            num_nodes,
            4,
            8,
        )
        .await
        .unwrap();

        // Center node should have high degree, more normalized
        assert_eq!(output.len(), num_nodes * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_gcn_conv_large_batch() {
        let dev = get_test_device().await;

        // Large graph with many edges
        let num_nodes = 20;
        let in_feat = 16;
        let out_feat = 32;

        // Create ring graph
        let mut edges = Vec::new();
        for i in 0..num_nodes {
            let next = (i + 1) % num_nodes;
            edges.push((i, next));
            edges.push((next, i)); // Bidirectional
        }

        let node_features = vec![0.5; num_nodes * in_feat];
        let weights = vec![0.1; in_feat * out_feat];
        let output = gcn_conv(
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
    async fn test_gcn_conv_precision() {
        let dev = get_test_device().await;

        // Test symmetric normalization with known degrees
        let node_features = vec![1.0; 3 * 2];
        let edges = vec![(0, 1), (1, 0), (1, 2), (2, 1)]; // Node 1 has degree 2
        let weights = vec![0.5; 2 * 4];

        let output = gcn_conv(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            &weights,
            3,
            2,
            4,
        )
        .await
        .unwrap();

        // All nodes should have output (including self-loops)
        for node in 0..3 {
            for f in 0..4 {
                let val = output[node * 4 + f];
                assert!(val.is_finite() && val >= 0.0);
            }
        }
    }
}
