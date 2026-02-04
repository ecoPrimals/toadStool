// SAGEConv - GraphSAGE (Hamilton et al.)
// Scalable sampling and aggregation: h_i' = W * [h_i || aggr_i]
//
// Algorithm:
// 1. Aggregate neighbors: aggr_i = MEAN(h_j for j in N(i))
// 2. Concatenate self and aggregated: [h_i || aggr_i]
// 3. Apply linear transformation: output = W * concat

struct Params {
    num_nodes: u32,
    num_edges: u32,
    in_features: u32,
    out_features: u32,
    normalize: u32,  // 0 or 1 (L2 normalize output)
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> node_features: array<f32>;    // [num_nodes, in_features]
@group(0) @binding(2) var<storage, read> edge_index: array<u32>;       // [num_edges * 2]
@group(0) @binding(3) var<storage, read> weights: array<f32>;          // [2 * in_features, out_features]
@group(0) @binding(4) var<storage, read> degrees: array<u32>;          // [num_nodes] (neighbor count)
@group(0) @binding(5) var<storage, read_write> aggregated: array<f32>; // [num_nodes, in_features]
@group(0) @binding(6) var<storage, read_write> output: array<f32>;     // [num_nodes, out_features]

// Step 1: Aggregate neighbors (mean pooling)
@compute @workgroup_size(256)
fn aggregate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let edge = global_id.x;
    if (edge >= params.num_edges) {
        return;
    }

    let src = edge_index[edge * 2u];
    let dst = edge_index[edge * 2u + 1u];

    // Accumulate neighbor features
    for (var f = 0u; f < params.in_features; f++) {
        let val = node_features[src * params.in_features + f];
        atomicAdd(&aggregated[dst * params.in_features + f], bitcast<i32>(val));
    }
}

// Step 2: Divide by degree (mean) and apply transformation
@compute @workgroup_size(256)
fn apply_transform(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    if (node >= params.num_nodes) {
        return;
    }

    let deg = f32(degrees[node]);
    
    // Compute output: W * [h_i || aggr_i / deg]
    for (var out_f = 0u; out_f < params.out_features; out_f++) {
        var sum = 0.0;
        
        // Self features: first half of weight matrix
        for (var in_f = 0u; in_f < params.in_features; in_f++) {
            let feat = node_features[node * params.in_features + in_f];
            let weight_idx = in_f * params.out_features + out_f;
            sum += feat * weights[weight_idx];
        }
        
        // Aggregated features: second half of weight matrix
        for (var in_f = 0u; in_f < params.in_features; in_f++) {
            var aggr = aggregated[node * params.in_features + in_f];
            if (deg > 0.0) {
                aggr /= deg; // Mean aggregation
            }
            let weight_idx = (params.in_features + in_f) * params.out_features + out_f;
            sum += aggr * weights[weight_idx];
        }
        
        output[node * params.out_features + out_f] = sum;
    }
}

// Step 3: Optional L2 normalization
@compute @workgroup_size(256)
fn normalize_output(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    if (node >= params.num_nodes || params.normalize == 0u) {
        return;
    }

    // Compute L2 norm
    var norm_sq = 0.0;
    for (var f = 0u; f < params.out_features; f++) {
        let val = output[node * params.out_features + f];
        norm_sq += val * val;
    }
    let norm = sqrt(norm_sq) + 1e-8;

    // Normalize
    for (var f = 0u; f < params.out_features; f++) {
        let idx = node * params.out_features + f;
        output[idx] /= norm;
    }
}
