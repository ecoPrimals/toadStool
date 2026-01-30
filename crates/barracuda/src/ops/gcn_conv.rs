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
    let deg_inv_sqrt: Vec<f32> = degrees.iter()
        .map(|&d| 1.0 / (d as f32).sqrt())
        .collect();
    
    let mut output = vec![0.0f32; num_nodes * out_features];
    
    // Normalized aggregation
    for &(src, dst) in edge_index {
        let norm = deg_inv_sqrt[src] * deg_inv_sqrt[dst];
        
        for out_f in 0..out_features {
            let mut msg = 0.0;
            for in_f in 0..in_features {
                msg += node_features[src * in_features + in_f] 
                     * weights[in_f * out_features + out_f];
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
                msg += node_features[node * in_features + in_f]
                     * weights[in_f * out_features + out_f];
            }
            output[node * out_features + out_f] += norm * msg;
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
    async fn test_gcn_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 4 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3)];
        let weights = vec![0.1; 8 * 16];
        let output = gcn_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, 4, 8, 16).await.unwrap();
        assert_eq!(output.len(), 4 * 16);
    }
}
