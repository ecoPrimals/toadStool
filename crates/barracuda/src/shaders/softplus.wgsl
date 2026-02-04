// Softplus - Smooth approximation of ReLU
//
// Deep Debt Principles:
// - Pure WGSL implementation (universal compute)
// - Zero unsafe code (memory safe)
// - Hardware-agnostic (works on any GPU/CPU via WebGPU)
// - Self-contained logic (no external dependencies)

struct Params {
    size: u32,
    beta: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.size) {
        return;
    }
    
    let x = input[idx];
    // Softplus: (1/beta) * log(1 + exp(beta * x))
    // For numerical stability, use: max(0, x) + log(1 + exp(-abs(x)))
    let beta_x = params.beta * x;
    output[idx] = (1.0 / params.beta) * (max(0.0, beta_x) + log(1.0 + exp(-abs(beta_x))));
}
