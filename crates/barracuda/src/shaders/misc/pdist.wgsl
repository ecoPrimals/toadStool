// pdist.wgsl - Pairwise Distances (all pairs)
//
// Computes distances between all pairs of row vectors
// Output is condensed distance matrix (upper triangle)

struct Params {
    num_vectors: u32,
    dim: u32,
    p: f32,         // p-norm
    epsilon: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;        // [num_vectors, dim]
@group(0) @binding(1) var<storage, read_write> output: array<f32>; // [num_pairs] where num_pairs = n*(n-1)/2
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = global_id.y;
    
    if (i >= params.num_vectors || j >= params.num_vectors || i >= j) {
        return;
    }
    
    var dist: f32 = 0.0;
    
    for (var d: u32 = 0u; d < params.dim; d = d + 1u) {
        let val_i = input[i * params.dim + d];
        let val_j = input[j * params.dim + d];
        let diff = abs(val_i - val_j);
        
        if (params.p == 1.0) {
            dist = dist + diff;
        } else if (params.p == 2.0) {
            dist = dist + diff * diff;
        } else {
            dist = dist + pow(diff + params.epsilon, params.p);
        }
    }
    
    if (params.p == 2.0) {
        dist = sqrt(dist);
    } else if (params.p > 1.0 && params.p != 2.0) {
        dist = pow(dist + params.epsilon, 1.0 / params.p);
    }
    
    // Condensed index: row i, col j (i < j)
    // Index = i * n - i*(i+1)/2 + (j - i - 1)
    let out_idx = i * params.num_vectors - (i * (i + 1u)) / 2u + (j - i - 1u);
    output[out_idx] = dist;
}
