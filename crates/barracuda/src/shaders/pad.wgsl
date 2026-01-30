// Pad operation - add padding with constant value
// Simplified 1D version

struct PadParams {
    input_size: u32,
    pad_left: u32,
    pad_right: u32,
    pad_value: f32,
}

@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform> params: PadParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let output_size = params.pad_left + params.input_size + params.pad_right;
    
    if (idx >= output_size) {
        return;
    }
    
    if (idx < params.pad_left) {
        // Left padding
        output[idx] = params.pad_value;
    } else if (idx < params.pad_left + params.input_size) {
        // Input data
        output[idx] = input[idx - params.pad_left];
    } else {
        // Right padding
        output[idx] = params.pad_value;
    }
}
