// graph_conv.wgsl - Graph Convolution (GCN)
//
// Graph Convolutional Network layer
// Reference: "Semi-Supervised Classification with Graph Convolutional Networks" by Kipf & Welling (2017)
//
// H' = σ(D^{-1/2} A D^{-1/2} H W)

struct Params {
    num_nodes: u32,
    in_features: u32,
    out_features: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> node_features: array<f32>;    // [num_nodes, in_features]
@group(0) @binding(1) var<storage, read> adj_matrix: array<f32>;       // [num_nodes, num_nodes] - normalized adjacency
@group(0) @binding(2) var<storage, read> weight: array<f32>;           // [in_features, out_features]
@group(0) @binding(3) var<storage, read> bias: array<f32>;             // [out_features]
@group(0) @binding(4) var<storage, read_write> output: array<f32>;     // [num_nodes, out_features]
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let node_idx = global_id.x;
    
    if (node_idx >= params.num_nodes) {
        return;
    }
    
    // For each output feature
    for (var out_f: u32 = 0u; out_f < params.out_features; out_f = out_f + 1u) {
        var sum: f32 = 0.0;
        
        // Aggregate from neighbors
        for (var neighbor: u32 = 0u; neighbor < params.num_nodes; neighbor = neighbor + 1u) {
            let adj_weight = adj_matrix[node_idx * params.num_nodes + neighbor];
            
            if (adj_weight > 0.0) {
                // Multiply by feature transformation
                for (var in_f: u32 = 0u; in_f < params.in_features; in_f = in_f + 1u) {
                    let feat = node_features[neighbor * params.in_features + in_f];
                    let w = weight[in_f * params.out_features + out_f];
                    sum = sum + adj_weight * feat * w;
                }
            }
        }
        
        // Add bias and apply ReLU
        sum = sum + bias[out_f];
        sum = max(0.0, sum);
        
        output[node_idx * params.out_features + out_f] = sum;
    }
}
