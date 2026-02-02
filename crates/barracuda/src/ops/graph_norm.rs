//! GraphNorm - Graph normalization (Cai et al.)
//!
//! Normalizes node features per graph in a batch.
//! Addresses over-smoothing in deep GNNs.

pub async fn graph_norm(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    batch_assignment: &[usize],
    num_nodes: usize,
    num_features: usize,
    num_graphs: usize,
    epsilon: f32,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    // Compute mean and variance per graph
    let mut graph_mean = vec![0.0f32; num_graphs * num_features];
    let mut graph_var = vec![0.0f32; num_graphs * num_features];
    let mut counts = vec![0; num_graphs];

    // Mean
    for node in 0..num_nodes {
        let graph_id = batch_assignment[node];
        counts[graph_id] += 1;
        for f in 0..num_features {
            graph_mean[graph_id * num_features + f] += node_features[node * num_features + f];
        }
    }

    for graph in 0..num_graphs {
        if counts[graph] > 0 {
            let count = counts[graph] as f32;
            for f in 0..num_features {
                graph_mean[graph * num_features + f] /= count;
            }
        }
    }

    // Variance
    for node in 0..num_nodes {
        let graph_id = batch_assignment[node];
        for f in 0..num_features {
            let diff =
                node_features[node * num_features + f] - graph_mean[graph_id * num_features + f];
            graph_var[graph_id * num_features + f] += diff * diff;
        }
    }

    for graph in 0..num_graphs {
        if counts[graph] > 0 {
            let count = counts[graph] as f32;
            for f in 0..num_features {
                graph_var[graph * num_features + f] /= count;
            }
        }
    }

    // Normalize
    let mut output = vec![0.0f32; num_nodes * num_features];
    for node in 0..num_nodes {
        let graph_id = batch_assignment[node];
        for f in 0..num_features {
            let mean = graph_mean[graph_id * num_features + f];
            let std = (graph_var[graph_id * num_features + f] + epsilon).sqrt();
            output[node * num_features + f] = (node_features[node * num_features + f] - mean) / std;
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
    async fn test_graph_norm() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 8 * 8];
        let batch = vec![0, 0, 0, 0, 1, 1, 1, 1];
        let output = graph_norm(
            &dev.device,
            &dev.queue,
            &node_features,
            &batch,
            8,
            8,
            2,
            1e-5,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 8 * 8);
    }
}
