//! GlobalPooling - Graph-level pooling operations
//!
//! Aggregates node features to graph-level representation.
//! Supports sum, mean, and max pooling.

pub enum PoolingType {
    Sum,
    Mean,
    Max,
}

pub async fn global_pooling(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    node_features: &[f32],
    batch_assignment: &[usize], // Which graph each node belongs to
    pooling_type: PoolingType,
    num_nodes: usize,
    num_features: usize,
    num_graphs: usize,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let mut output = vec![0.0f32; num_graphs * num_features];
    let mut counts = vec![0; num_graphs];

    // Initialize for max pooling
    if matches!(pooling_type, PoolingType::Max) {
        for val in output.iter_mut() {
            *val = f32::NEG_INFINITY;
        }
    }

    // Aggregate per graph
    for node in 0..num_nodes {
        let graph_id = batch_assignment[node];
        if graph_id >= num_graphs {
            return Err("Batch assignment out of bounds".into());
        }

        counts[graph_id] += 1;

        for f in 0..num_features {
            let feat = node_features[node * num_features + f];
            let out_idx = graph_id * num_features + f;

            match pooling_type {
                PoolingType::Sum | PoolingType::Mean => {
                    output[out_idx] += feat;
                }
                PoolingType::Max => {
                    output[out_idx] = output[out_idx].max(feat);
                }
            }
        }
    }

    // Normalize for mean
    if matches!(pooling_type, PoolingType::Mean) {
        for graph in 0..num_graphs {
            if counts[graph] > 0 {
                let count = counts[graph] as f32;
                for f in 0..num_features {
                    output[graph * num_features + f] /= count;
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

    #[tokio::test]
    async fn test_global_pooling_mean() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 10 * 8]; // 10 nodes, 8 features
        let batch = vec![0, 0, 0, 1, 1, 1, 1, 2, 2, 2]; // 3 graphs
        let output = global_pooling(
            &dev.device,
            &dev.queue,
            &node_features,
            &batch,
            PoolingType::Mean,
            10,
            8,
            3,
        )
        .await
        .unwrap();
        assert_eq!(output.len(), 3 * 8);
    }
}
