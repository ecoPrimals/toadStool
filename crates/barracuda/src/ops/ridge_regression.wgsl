// Ridge regression for ESN readout training
// Solves: W_out = (X^T·X + λI)^(-1)·X^T·Y
// Simplified GPU implementation using normal equations

struct Params {
    n: u32,
    t: u32,
    m: u32,
    regularization: f32,
}

@group(0) @binding(0) var<storage, read> states: array<f32>;   // T×N
@group(0) @binding(1) var<storage, read> targets: array<f32>;  // T×M
@group(0) @binding(2) var<storage, read_write> output: array<f32>; // N×M
@group(0) @binding(3) var<uniform> params: Params;

// Simplified ridge regression using pseudo-inverse approximation
// For production, use proper matrix inversion or iterative solvers
@compute @workgroup_size(16, 16)
fn ridge_regression(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x; // neuron index (N)
    let j = gid.y; // output index (M)
    
    if (i >= params.n || j >= params.m) {
        return;
    }
    
    // Compute X^T·Y element (i,j)
    // This is simplified - proper implementation would compute (X^T·X + λI)^(-1)·X^T·Y
    var sum = 0.0;
    var norm_sq = 0.0;
    
    for (var k = 0u; k < params.t; k = k + 1u) {
        let state_val = states[k * params.n + i];
        let target_val = targets[k * params.m + j];
        sum = sum + state_val * target_val;
        norm_sq = norm_sq + state_val * state_val;
    }
    
    // Apply regularization: w = X^T·y / (||x||^2 + λ)
    let regularized_norm = norm_sq + params.regularization;
    output[i * params.m + j] = sum / regularized_norm;
}
