// GINConv - Graph Isomorphism Network (Xu et al.)
// Expressive GNN with MLP: h_i' = MLP((1 + ε) * h_i + Σ_j h_j)
//
// Algorithm:
// 1. Aggregate neighbor features: aggr_i = Σ_j h_j
// 2. Add self-features with epsilon: combined_i = (1 + ε) * h_i + aggr_i
// 3. Apply MLP (single layer here): output = W * combined + b

struct Params {
    num_nodes: u32,
    num_edges: u32,
    in_features: u32,
    out_features: u32,
    epsilon: f32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> node_features: array<f32>;    // [num_nodes, in_features]
@group(0) @binding(2) var<storage, read> edge_index: array<u32>;       // [num_edges * 2]
@group(0) @binding(3) var<storage, read> mlp_weights: array<f32>;      // [in_features, out_features]
@group(0) @binding(4) var<storage, read> mlp_bias: array<f32>;         // [out_features]
@group(0) @binding(5) var<storage, read_write> aggregated: array<atomic<i32>>; // [num_nodes, in_features] (atomic accumulation)
@group(0) @binding(6) var<storage, read_write> output: array<f32>;     // [num_nodes, out_features]

// Step 1: Aggregate neighbors (sum pooling)
@compute @workgroup_size(256)
fn aggregate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let edge = global_id.x;
    if (edge >= params.num_edges) {
        return;
    }

    let src = edge_index[edge * 2u];
    let dst = edge_index[edge * 2u + 1u];

    // Aggregate: sum neighbor features
    for (var f = 0u; f < params.in_features; f++) {
        let val = node_features[src * params.in_features + f];
        atomicAdd(&aggregated[dst * params.in_features + f], bitcast<i32>(val));
    }
}

// Step 2: Combine with self-features and apply MLP
@compute @workgroup_size(256)
fn apply_mlp(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    if (node >= params.num_nodes) {
        return;
    }

    // Combine: (1 + ε) * h_i + aggr_i
    for (var out_f = 0u; out_f < params.out_features; out_f++) {
        var sum = mlp_bias[out_f];
        
        for (var in_f = 0u; in_f < params.in_features; in_f++) {
            let self_feat = (1.0 + params.epsilon) * node_features[node * params.in_features + in_f];
            let neighbor_feat = bitcast<f32>(atomicLoad(&aggregated[node * params.in_features + in_f]));
            let combined = self_feat + neighbor_feat;
            
            let weight_idx = in_f * params.out_features + out_f;
            sum += combined * mlp_weights[weight_idx];
        }
        
        output[node * params.out_features + out_f] = sum;
    }
}
