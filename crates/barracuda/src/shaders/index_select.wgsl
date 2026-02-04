// Index Select - Select elements by indices
//
// Deep Debt Principles:
// - Pure WGSL implementation (universal compute)
// - Zero unsafe code (memory safe)
// - Hardware-agnostic (works on any GPU/CPU via WebGPU)
// - Self-contained logic (no external dependencies)

struct Params {
    input_size: u32,
    output_size: u32,
    num_indices: u32,
    _pad: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.output_size) {
        return;
    }
    
    // For 1D case: output[idx] = input[indices[idx]]
    // For multi-dimensional, we need to handle strides
    // Simplified: assume 1D for now
    let index = indices[idx];
    
    // Bounds check
    if (index < params.input_size) {
        output[idx] = input[index];
    } else {
        output[idx] = 0.0;
    }
}
