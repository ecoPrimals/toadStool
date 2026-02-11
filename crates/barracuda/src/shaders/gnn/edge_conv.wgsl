// edge_conv.wgsl - Edge Convolution for Graph Neural Networks
//
// Learns edge features by aggregating neighbor information.
// Reference: "Dynamic Graph CNN for Learning on Point Clouds" by Wang et al. (2019)
//
// For each node i, computes: h_i' = max_{j∈N(i)} ReLU(W · [h_i ‖ (h_j - h_i)] + b)
//
// Edge storage: CSR-like format using edge_offsets and edge_targets.
//   edge_offsets[i]   = start index in edge_targets for node i's neighbors
//   edge_offsets[i+1] = end index (exclusive)
//   edge_targets[k]   = neighbor node index
//
// Cross-domain: point cloud learning, molecular graphs, social networks,
// physics simulations on unstructured meshes.

struct Params {
    num_nodes: u32,
    feature_dim: u32,
    output_dim: u32,
    num_edges: u32,    // Total edges in the graph
}

@group(0) @binding(0) var<storage, read> node_features: array<f32>;    // [num_nodes, feature_dim]
@group(0) @binding(1) var<storage, read> edge_offsets: array<u32>;     // [num_nodes + 1] CSR row offsets
@group(0) @binding(2) var<storage, read> edge_targets: array<u32>;     // [num_edges] neighbor indices
@group(0) @binding(3) var<storage, read> mlp_weight: array<f32>;       // [output_dim, 2*feature_dim]
@group(0) @binding(4) var<storage, read> mlp_bias: array<f32>;         // [output_dim]
@group(0) @binding(5) var<storage, read_write> output: array<f32>;     // [num_nodes, output_dim]
@group(0) @binding(6) var<uniform> params: Params;

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

    // Initialize max-pooled output features to -inf
    var max_features: array<f32, 256>; // Max 256 output dim
    for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
        max_features[o] = -1e10;
    }

    // Get neighbor range from CSR offsets
    let neighbor_start = edge_offsets[node_idx];
    let neighbor_end = edge_offsets[node_idx + 1u];

    // Aggregate over actual neighbors from edge index
    for (var k: u32 = neighbor_start; k < neighbor_end; k = k + 1u) {
        let neighbor_idx = edge_targets[k];

        // Bounds check on neighbor index
        if (neighbor_idx >= params.num_nodes) {
            continue;
        }

        // Compute edge feature: concat(h_i, h_j - h_i)
        var edge_feature: array<f32, 512>; // Max 512 (2 * 256)

        // First half: center features h_i
        for (var f: u32 = 0u; f < params.feature_dim && f < 256u; f = f + 1u) {
            edge_feature[f] = center_features[f];
        }

        // Second half: relative features (h_j - h_i)
        for (var f: u32 = 0u; f < params.feature_dim && f < 256u; f = f + 1u) {
            let neighbor_feat = node_features[neighbor_idx * params.feature_dim + f];
            edge_feature[params.feature_dim + f] = neighbor_feat - center_features[f];
        }

        // Apply single-layer MLP with ReLU: ReLU(W · edge_feature + b)
        for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
            var sum: f32 = 0.0;

            // Matrix multiply: W[o, :] · edge_feature
            let double_feat = 2u * params.feature_dim;
            for (var f: u32 = 0u; f < double_feat && f < 512u; f = f + 1u) {
                let w_idx = o * double_feat + f;
                sum = sum + mlp_weight[w_idx] * edge_feature[f];
            }

            // Add bias and ReLU activation
            sum = max(0.0, sum + mlp_bias[o]);

            // Max pooling across neighbors
            max_features[o] = max(max_features[o], sum);
        }
    }

    // Handle isolated nodes (no neighbors): output zero instead of -inf
    let has_neighbors = neighbor_end > neighbor_start;

    // Write output
    for (var o: u32 = 0u; o < params.output_dim && o < 256u; o = o + 1u) {
        let val = select(0.0, max_features[o], has_neighbors);
        output[node_idx * params.output_dim + o] = val;
    }
}
