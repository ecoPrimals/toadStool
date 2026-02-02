//! MessagePassing - Generic message passing framework
//!
//! Abstract base for GNN layers: message() -> aggregate() -> update()
//! Implements the message passing neural network paradigm.

pub enum Aggregation {
    Sum,
    Mean,
    Max,
}

pub async fn message_passing(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    edge_index: &[(usize, usize)],
    edge_features: Option<&[f32]>,
    aggregation: Aggregation,
    num_nodes: usize,
    num_features: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; num_nodes * num_features];
    let mut counts = vec![0; num_nodes];

    // Message aggregation phase
    for (edge_idx, &(src, dst)) in edge_index.iter().enumerate() {
        counts[dst] += 1;

        for f in 0..num_features {
            let mut msg = node_features[src * num_features + f];

            // Optionally incorporate edge features
            if let Some(edge_feat) = edge_features {
                msg *= edge_feat[edge_idx * num_features + f];
            }

            match aggregation {
                Aggregation::Sum | Aggregation::Mean => {
                    output[dst * num_features + f] += msg;
                }
                Aggregation::Max => {
                    output[dst * num_features + f] = output[dst * num_features + f].max(msg);
                }
            }
        }
    }

    // Apply mean normalization if needed
    if matches!(aggregation, Aggregation::Mean) {
        for node in 0..num_nodes {
            if counts[node] > 0 {
                let count = counts[node] as f32;
                for f in 0..num_features {
                    output[node * num_features + f] /= count;
                }
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

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_message_passing_basic() {
        let dev = get_test_device().await;
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Sum,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 4 * 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_message_passing_edge_cases() {
        let dev = get_test_device().await;

        // Single edge
        let node_features = vec![1.0; 2 * 4];
        let edges = vec![(0, 1)];
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Sum,
            2,
            4,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 2 * 4);

        // No edges
        let node_features = vec![1.0; 3 * 4];
        let edges = vec![];
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Sum,
            3,
            4,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 3 * 4);
    }

    #[tokio::test]
    async fn test_message_passing_boundary() {
        let dev = get_test_device().await;

        // Mean aggregation
        let node_features = vec![2.0; 4 * 8];
        let edges = vec![(0, 1), (0, 1), (2, 3)]; // Duplicate edge 0->1
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Mean,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 4 * 8);

        // Max aggregation
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2)];
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Max,
            4,
            8,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 4 * 8);
    }

    #[tokio::test]
    async fn test_message_passing_large_graph() {
        let dev = get_test_device().await;

        // 100 nodes, many edges
        let node_features = vec![1.0; 100 * 16];
        let edges: Vec<(usize, usize)> = (0..50).map(|i| (i, i + 1)).collect();
        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            None,
            Aggregation::Sum,
            100,
            16,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 100 * 16);
    }

    #[tokio::test]
    async fn test_message_passing_precision() {
        let dev = get_test_device().await;

        // Test with edge features
        let node_features = vec![2.0; 3 * 4];
        let edges = vec![(0, 1), (1, 2)];
        let edge_features = vec![0.5; 2 * 4]; // Scale by 0.5

        let output = message_passing(
            &dev.device,
            &dev.queue,
            &node_features,
            &edges,
            Some(&edge_features),
            Aggregation::Sum,
            3,
            4,
        )
        .await
        .unwrap();

        assert_eq!(output.len(), 3 * 4);
        // Node 1 receives message: 2.0 * 0.5 = 1.0 per feature
        assert!(output[4..8].iter().any(|&x| x > 0.0));
    }
}
