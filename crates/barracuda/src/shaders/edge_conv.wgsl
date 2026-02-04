// edge_conv.wgsl - Edge Convolution for Graph Neural Networks
//
// Learns edge features by aggregating neighbor information
// Reference: "Dynamic Graph CNN for Learning on Point Clouds" by Wang et al. (2019)
//
// For each node i, computes: h_i' = max_{j∈N(i)} MLP(h_i || h_j - h_i)

struct Params {
    num_nodes: u32,
    feature_dim: u32,
    output_dim: u32,
    k_neighbors: u32,  // Number of nearest neighbors
}

@group(0) @binding(0) var<storage, read> node_features: array<f32>;    // [num_nodes, feature_dim]
@group(0) @binding(1) var<storage, read> edge_index: array<u32>;       // [num_edges, 2] - adjacency list
@group(0) @binding(2) var<storage, read> mlp_weight: array<f32>;       // [output_dim, 2*feature_dim]
@group(0) @binding(3) var<storage, read> mlp_bias: array<f32>;         // [output_dim]
@group(0) @binding(4) var<storage, read_write> output: array<f32>;     // [num_nodes, output_dim]
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_idx = global_id.x;
    
    if (node_idx >= params.num_nodes) {
        return;
    }
    
    // Load center node features
    var center_features: array<f32, 256>; // Max 256 feature dim
    for (var f: u32 = 0u; f < params.feature_dim && f < 256u; f = f + 1u) {
        center_features[f] = node_features[node_idx * params.feature_dim + f];
    }
    
    // Aggregate over neighbors (simplified - assumes edges are stored sequentially)
    var max_features: array<f32, 256>; // Max 256 output dim
    for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
        max_features[o] = -1e10;
    }
    
    // For each neighbor (simplified - in practice, would need neighbor list)
    // This is a placeholder - full implementation needs proper edge index handling
    for (var k: u32 = 0u; k < params.k_neighbors; k = k + 1u) {
        // Get neighbor index (placeholder - would come from edge_index)
        let neighbor_idx = (node_idx + k + 1u) % params.num_nodes;
        
        // Compute edge feature: concat(h_i, h_j - h_i)
        var edge_feature: array<f32, 512>; // Max 512 (2 * 256)
        
        // First half: center features
        for (var f: u32 = 0u; f < params.feature_dim && f < 256u; f = f + 1u) {
            edge_feature[f] = center_features[f];
        }
        
        // Second half: relative features
        for (var f: u32 = 0u; f < params.feature_dim && f < 256u; f = f + 1u) {
            let neighbor_feat = node_features[neighbor_idx * params.feature_dim + f];
            edge_feature[params.feature_dim + f] = neighbor_feat - center_features[f];
        }
        
        // Apply MLP
        for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
            var sum: f32 = 0.0;
            
            // Matrix multiply
            for (var f: u32 = 0u; f < 2u * params.feature_dim; f = f + 1u) {
                let w_idx = o * 2u * params.feature_dim + f;
                sum = sum + mlp_weight[w_idx] * edge_feature[f];
            }
            
            // Add bias and ReLU
            sum = sum + mlp_bias[o];
            sum = max(0.0, sum);
            
            // Max pooling across neighbors
            max_features[o] = max(max_features[o], sum);
        }
    }
    
    // Write output
    for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
        output[node_idx * params.output_dim + o] = max_features[o];
    }
}
