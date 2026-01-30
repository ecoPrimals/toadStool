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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_sage_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 6 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)];
        let weights = vec![0.1; 16 * 16]; // 2*in_features x out_features
        let output = sage_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 6, 8, 16).await.unwrap();
        assert_eq!(output.len(), 6 * 16);
    }
}
