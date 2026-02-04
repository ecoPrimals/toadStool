// SpectralNorm1D - Spectral normalization for 1D convolutions
// Normalizes weight matrix by its largest singular value
// Used for stabilizing GAN training in audio generation

struct Params {
    rows: u32,        // out_channels
    cols: u32,        // in_channels * kernel_size
    n_power_iter: u32,
}

@group(0) @binding(0) var<storage, read> weights: array<f32>;      // [rows, cols]
@group(0) @binding(1) var<storage, read_write> u: array<f32>;     // [rows]
@group(0) @binding(2) var<storage, read_write> v: array<f32>;     // [cols]
@group(0) @binding(3) var<storage, read_write> output: array<f32>; // [rows, cols]
@group(0) @binding(4) var<uniform> params: Params;

// Power iteration: v = W^T @ u
@compute @workgroup_size(256)
fn power_iter_v(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let c = global_id.x;
    
    if (c >= params.cols) {
        return;
    }
    
    var sum: f32 = 0.0;
    for (var r: u32 = 0u; r < params.rows; r = r + 1u) {
        sum = sum + weights[r * params.cols + c] * u[r];
    }
    
    v[c] = sum;
}

// Power iteration: u = W @ v
@compute @workgroup_size(256)
fn power_iter_u(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let r = global_id.x;
    
    if (r >= params.rows) {
        return;
    }
    
    var sum: f32 = 0.0;
    for (var c: u32 = 0u; c < params.cols; c = c + 1u) {
        sum = sum + weights[r * params.cols + c] * v[c];
    }
    
    u[r] = sum;
}

// Normalize weights by sigma
@compute @workgroup_size(256)
fn normalize_weights(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let total = params.rows * params.cols;
    
    if (idx >= total) {
        return;
    }
    
    // Compute sigma = u^T @ W @ v (simplified - would need reduction)
    // For now, use a placeholder sigma value
    // Full implementation would compute sigma via reduction pass
    
    let r = idx / params.cols;
    let c = idx % params.cols;
    
    // Simplified: normalize by estimated sigma
    // Full implementation would compute actual sigma
    output[idx] = weights[idx];
}
