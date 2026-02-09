// GCNConv - Graph Convolutional Network (Kipf & Welling)
// Standard GCN layer: H' = σ(D^{-1/2} A D^{-1/2} H W)
//
// Algorithm:
// 1. Transform features: H' = HW
// 2. Normalize by degree: D^{-1/2}
// 3. Aggregate neighbors with normalization
// 4. Apply activation (done externally)

struct Params {
    num_nodes: u32,
    num_edges: u32,
    in_features: u32,
    out_features: u32,
    add_self_loops: u32,  // 0 or 1
    normalize: u32,       // 0 or 1
    _pad1: u32,
    _pad2: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> node_features: array<f32>;  // [num_nodes, in_features]
@group(0) @binding(2) var<storage, read> edge_index: array<u32>;     // [num_edges * 2]
@group(0) @binding(3) var<storage, read> weights: array<f32>;         // [in_features, out_features]
@group(0) @binding(4) var<storage, read> degrees: array<f32>;         // [num_nodes] (sqrt(degree))
@group(0) @binding(5) var<storage, read_write> transformed: array<f32>; // [num_nodes, out_features]
@group(0) @binding(6) var<storage, read_write> output: array<atomic<i32>>;    // [num_nodes, out_features] (atomic accumulation)

// Step 1: Transform features (H' = HW)
@compute @workgroup_size(256)
fn transform_features(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    if (node >= params.num_nodes) {
        return;
    }

    for (var out_f = 0u; out_f < params.out_features; out_f++) {
        var sum = 0.0;
        for (var in_f = 0u; in_f < params.in_features; in_f++) {
            let feat_idx = node * params.in_features + in_f;
            let weight_idx = in_f * params.out_features + out_f;
            sum += node_features[feat_idx] * weights[weight_idx];
        }
        transformed[node * params.out_features + out_f] = sum;
    }
}

// Step 2: Aggregate with symmetric normalization
@compute @workgroup_size(256)
fn aggregate(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let edge = global_id.x;
    if (edge >= params.num_edges) {
        return;
    }

    let src = edge_index[edge * 2u];
    let dst = edge_index[edge * 2u + 1u];

    // Normalization factor: 1 / sqrt(deg(i) * deg(j))
    var norm = 1.0;
    if (params.normalize != 0u) {
        norm = 1.0 / (degrees[src] * degrees[dst]);
    }

    // Aggregate: accumulate normalized messages
    for (var f = 0u; f < params.out_features; f++) {
        let val = norm * transformed[src * params.out_features + f];
        atomicAdd(&output[dst * params.out_features + f], bitcast<i32>(val));
    }
}

// Step 3: Add self-loops
@compute @workgroup_size(256)
fn add_self_loops(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node = global_id.x;
    if (node >= params.num_nodes) {
        return;
    }

    if (params.add_self_loops != 0u) {
        for (var f = 0u; f < params.out_features; f++) {
            let idx = node * params.out_features + f;
            atomicAdd(&output[idx], bitcast<i32>(transformed[idx]));
        }
    }
}
