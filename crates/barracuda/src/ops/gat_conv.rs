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
                    node_features[node * in_features + in_f] 
                    * weights[in_f * out_features + out_f];
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
            output[dst * out_features + f] += 
                weight * transformed[src * out_features + f];
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
    async fn test_gat_conv() {
        let dev = Arc::new(WgpuDevice::new().await.unwrap());
        let node_features = vec![1.0; 5 * 8];
        let edges = vec![(0, 1), (1, 2), (2, 3), (3, 4)];
        let weights = vec![0.1; 8 * 16];
        let attention = vec![0.05; 32]; // 2 * out_features
        let output = gat_conv(&dev.device, &dev.queue, &node_features, &edges, &weights, &attention, 5, 8, 16).await.unwrap();
        assert_eq!(output.len(), 5 * 16);
    }
}
