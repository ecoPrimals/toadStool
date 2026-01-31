// Spectral radius computation via power iteration
// Computes largest absolute eigenvalue of a matrix

struct Params {
    size: u32,
}

@group(0) @binding(0) var<storage, read> matrix: array<f32>;
@group(0) @binding(1) var<storage, read> input_vector: array<f32>;
@group(0) @binding(2) var<storage, read_write> output_vector: array<f32>;
@group(0) @binding(3) var<storage, read_write> norm: f32;
@group(0) @binding(4) var<uniform> params: Params;

// Matrix-vector multiply: output = A · input
@compute @workgroup_size(256)
fn matrix_vector_multiply(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.size) {
        return;
    }
    
    var sum = 0.0;
    for (var j = 0u; j < params.size; j = j + 1u) {
        let idx = i * params.size + j;
        sum = sum + matrix[idx] * input_vector[j];
    }
    
    output_vector[i] = sum;
    
    // First thread computes norm
    if (i == 0u) {
        var norm_sq = 0.0;
        for (var j = 0u; j < params.size; j = j + 1u) {
            norm_sq = norm_sq + output_vector[j] * output_vector[j];
        }
        norm = sqrt(norm_sq);
    }
}

// Normalize vector: output = output / ||output||
@compute @workgroup_size(256)
fn normalize_vector(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.size) {
        return;
    }
    
    // Wait for norm to be computed (implicit via buffer dependencies)
    let n = norm;
    if (n > 1e-10) {
        output_vector[i] = output_vector[i] / n;
    }
}
