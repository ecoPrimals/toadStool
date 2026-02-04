// Tensor Expand (Broadcasting)
// Expands tensor dimensions by repeating values

struct Params {
    input_size: u32,
    output_size: u32,
    input_stride: u32,
    repeat_factor: u32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.output_size) {
        return;
    }
    
    // Map output index to input index via broadcasting
    // For simple expansion: output[i] = input[i % input_size]
    let input_idx = (idx / params.repeat_factor) % params.input_size;
    
    output[idx] = input[input_idx];
}
