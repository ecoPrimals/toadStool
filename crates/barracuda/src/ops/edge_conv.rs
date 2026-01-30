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
    use crate::device::WgpuDevice;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_edge_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let weights = vec![0.1; 16 * 16]; // 2*in_features x out_features
        let output = edge_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 4, 8, 16).await.unwrap();
        assert_eq!(output.len(), 4 * 16);
    }
}
